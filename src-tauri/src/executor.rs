//! Workflow execution engine
//!
//! Each workflow has its own execution strategy:
//! - Today: per-operation pipeline (ADR-0042) — Rust-native prepare + delivery + AI enrichment
//! - Week: per-operation pipeline (I94) — Rust-native prepare + delivery + AI enrichment
//! - Archive: pure Rust reconciliation + file moves
//! - InboxBatch: direct processor calls

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::error::ExecutionError;
use crate::notification::send_notification;
use crate::pty::{ModelTier, PtyManager};
use crate::scheduler::SchedulerMessage;
use crate::state::{create_execution_record, AppState};
use crate::types::{
    AiModelConfig, EmailSyncStage, EmailSyncState, EmailSyncStatus, ExecutionTrigger, WorkflowId,
    WorkflowPhase, WorkflowStatus,
};

/// Executor manages workflow execution
pub struct Executor {
    state: Arc<AppState>,
    app_handle: AppHandle,
}

impl Executor {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        Self { state, app_handle }
    }

    /// Read AI model config from current config, falling back to defaults.
    fn ai_model_config(&self) -> AiModelConfig {
        self.state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.ai_models.clone()))
            .unwrap_or_default()
    }

    fn emit_email_sync_status(&self, status: &EmailSyncStatus) {
        let _ = self.app_handle.emit("email-sync-status", status);
    }

    fn build_email_sync_status(
        &self,
        state: EmailSyncState,
        stage: EmailSyncStage,
        code: Option<String>,
        message: Option<String>,
        using_last_known_good: Option<bool>,
    ) -> EmailSyncStatus {
        EmailSyncStatus {
            state,
            stage,
            code,
            message,
            using_last_known_good,
            can_retry: Some(true),
            last_attempt_at: Some(Utc::now().to_rfc3339()),
            last_success_at: None,
        }
    }

    fn is_model_unavailable_error(err: &str) -> bool {
        err.to_lowercase().contains("model_unavailable")
    }

    fn enrich_emails_with_fallback(
        &self,
        data_dir: &Path,
        workspace: &Path,
        user_ctx: &crate::types::UserContext,
        extraction_pty: &PtyManager,
        synthesis_pty: &PtyManager,
    ) -> Result<(), String> {
        match crate::workflow::deliver::enrich_emails(data_dir, extraction_pty, workspace, user_ctx)
        {
            Ok(()) => Ok(()),
            Err(err) if Self::is_model_unavailable_error(&err) => {
                log::warn!(
                    "Email enrichment extraction model unavailable, retrying with synthesis tier: {}",
                    err
                );
                match crate::workflow::deliver::enrich_emails(
                    data_dir,
                    synthesis_pty,
                    workspace,
                    user_ctx,
                ) {
                    Ok(()) => {
                        let _ = self.app_handle.emit(
                            "email-enrichment-warning",
                            "Email enrichment used synthesis fallback model",
                        );
                        Ok(())
                    }
                    Err(fallback_err) => Err(format!(
                        "Email enrichment fallback failed after extraction model error: {}",
                        fallback_err
                    )),
                }
            }
            Err(err) => Err(err),
        }
    }

    /// Start the executor loop
    ///
    /// Listens for workflow execution requests from the scheduler or manual triggers.
    pub async fn run(&self, mut receiver: mpsc::Receiver<SchedulerMessage>) {
        while let Some(msg) = receiver.recv().await {
            log::info!(
                "Executing workflow {:?} (trigger: {:?})",
                msg.workflow,
                msg.trigger
            );

            if let Err(e) = self.execute_workflow(msg.workflow, msg.trigger).await {
                log::error!("Workflow {:?} failed: {}", msg.workflow, e);
            }
        }
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: WorkflowId,
        trigger: ExecutionTrigger,
    ) -> Result<(), ExecutionError> {
        // Get workspace path from config
        let workspace = self.get_workspace_path()?;

        // Create execution record
        let record = create_execution_record(workflow_id, trigger);
        let execution_id = record.id.clone();
        self.state.add_execution_record(record.clone());

        let result = if workflow_id == WorkflowId::Archive {
            // Archive workflow: pure Rust, no three-phase, no notification
            self.execute_archive(&workspace, &execution_id, trigger).await
        } else if workflow_id == WorkflowId::InboxBatch {
            // Inbox batch: direct processor calls, no three-phase
            self.execute_inbox_batch(&workspace, &execution_id, trigger)
                .await
        } else if workflow_id == WorkflowId::Week {
            // Week workflow: per-operation pipeline (I94)
            self.execute_week(&workspace, &execution_id, trigger, &record)
                .await
        } else {
            // Today workflow: per-operation pipeline (ADR-0042)
            // Phase 1 (Python) then Rust-native mechanical delivery — no Phase 2/3
            self.execute_today_pipeline(&workspace, &execution_id, trigger, &record)
                .await
        };

        if let Err(ref err) = result {
            let finished_at = Utc::now();
            let duration_secs = (finished_at - record.started_at)
                .num_seconds()
                .max(0) as u64;
            let error_phase = match self.state.get_workflow_status(workflow_id) {
                WorkflowStatus::Running { phase, .. } => Some(phase),
                _ => None,
            };
            let can_retry = err.is_retryable();

            self.state.update_execution_record(&execution_id, |r| {
                r.finished_at = Some(finished_at);
                r.duration_secs = Some(duration_secs);
                r.success = false;
                r.error_message = Some(err.to_string());
                r.error_phase = error_phase;
                r.can_retry = Some(can_retry);
            });

            self.emit_status_event(
                workflow_id,
                WorkflowStatus::Failed {
                    error: err.into(),
                    execution_id,
                },
            );
        }

        result
    }

    /// Execute archive workflow (pure Rust, silent operation)
    ///
    /// Sequence (ADR-0040 / permanence hardening):
    /// 1. Reconcile: read schedule.json, check transcript status, compute stats
    /// 2. Persist/freeze meetings while prep JSON still exists
    /// 3. Archive: move files, clean data/
    /// 4. Write day-summary + next-morning flags
    async fn execute_archive(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
    ) -> Result<(), ExecutionError> {
        use crate::workflow::archive::run_archive;
        use crate::workflow::reconcile;

        log::info!("Running archive workflow with reconciliation");

        // Step 1: Reconcile BEFORE archive (schedule.json gets cleaned)
        // Own DB connection to avoid starving foreground IPC commands
        let recon = {
            let own_db = crate::db::ActionDb::open().ok();
            let db_ref = own_db.as_ref();
            reconcile::run_reconciliation(workspace, db_ref)
        };

        log::info!(
            "Reconciliation: {} meetings completed, {} actions completed today, {} flags",
            recon.meetings.completed,
            recon.actions.completed_today,
            recon.flags.len(),
        );

        // Step 1.5: Daily impact rollup (feature-gated, I36/I39)
        {
            let impact_enabled = self
                .state
                .config
                .read()
                .ok()
                .and_then(|g| {
                    g.as_ref()
                        .map(|c| crate::types::is_feature_enabled(c, "impactRollup"))
                })
                .unwrap_or(false);

            if impact_enabled {
                if let Ok(db) = crate::db::ActionDb::open() {
                    match crate::workflow::impact_rollup::rollup_daily_impact(
                        workspace,
                        &db,
                        &recon.date,
                    ) {
                        Ok(r) if !r.skipped && (r.wins_rolled_up > 0 || r.risks_rolled_up > 0) => {
                            log::info!(
                                "Impact rollup: {} wins, {} risks → {}",
                                r.wins_rolled_up,
                                r.risks_rolled_up,
                                r.file_path,
                            );
                        }
                        Ok(r) if r.skipped => {
                            log::info!("Impact rollup: skipped (already rolled up today)");
                        }
                        Ok(_) => {
                            log::info!("Impact rollup: no captures to roll up");
                        }
                        Err(e) => {
                            log::warn!("Impact rollup failed (non-fatal): {}", e);
                        }
                    }
                }
            }
        }

        // Step 2: Persist meetings + freeze prep snapshots BEFORE archive cleanup.
        if let Ok(db_guard) = self.state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                reconcile::persist_meetings(db, &recon, workspace);
            }
        }

        // Step 3: Archive (move files, clean data/)
        let result = run_archive(workspace)
            .await
            .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;

        log::info!(
            "Archive complete: {} files moved{}",
            result.files_archived,
            if result.archive_path.is_empty() {
                String::new()
            } else {
                format!(" to {}", result.archive_path)
            }
        );

        // Step 4: Persist reconciliation summary artifacts
        // Write day-summary.json to archive directory (if files were archived)
        if !result.archive_path.is_empty() {
            let archive_path = std::path::Path::new(&result.archive_path);
            if let Err(e) =
                reconcile::write_day_summary(archive_path, &recon, result.files_archived)
            {
                log::warn!("Failed to write day summary: {}", e);
            }
        }

        // Write next-morning-flags.json to _today/data/ (survives until next archive)
        let today_dir = workspace.join("_today");
        if let Err(e) = reconcile::write_morning_flags(&today_dir, &recon) {
            log::warn!("Failed to write morning flags: {}", e);
        }

        // Update execution record
        let finished_at = Utc::now();
        self.state.update_execution_record(execution_id, |r| {
            r.finished_at = Some(finished_at);
            r.success = true;
        });

        // Update last scheduled run time
        if matches!(
            trigger,
            ExecutionTrigger::Scheduled | ExecutionTrigger::Missed
        ) {
            self.state
                .set_last_scheduled_run(WorkflowId::Archive, Utc::now());
        }

        // NO notification (silent operation)
        // NO workflow-completed event (no dashboard refresh needed)
        // Archive runs in the background, users don't need to know

        Ok(())
    }

    /// Execute inbox batch workflow (classify + enrich, no three-phase)
    ///
    /// 1. Quick-classify all inbox files
    /// 2. For each NeedsEnrichment result, run AI enrichment (cap at 5 per batch)
    /// 3. Emit `inbox-updated` so the frontend refreshes
    async fn execute_inbox_batch(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
    ) -> Result<(), ExecutionError> {
        // Feature gate (I39): skip if inbox processing is disabled
        let inbox_enabled = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .map(|c| crate::types::is_feature_enabled(c, "inboxProcessing"))
            })
            .unwrap_or(true);
        if !inbox_enabled {
            log::info!("Inbox batch skipped (feature disabled)");
            let finished_at = Utc::now();
            self.state.update_execution_record(execution_id, |r| {
                r.finished_at = Some(finished_at);
                r.success = true;
            });
            return Ok(());
        }

        log::info!("Running inbox batch workflow");

        // Get profile from config
        let profile = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.profile.clone()))
            .unwrap_or_else(|| "general".to_string());

        // Step 1: Quick-classify all inbox files
        let results = {
            let own_db = crate::db::ActionDb::open().ok();
            let db_ref = own_db.as_ref();
            crate::processor::process_all(workspace, db_ref, &profile)
        };

        let routed_count = results
            .iter()
            .filter(|(_, r)| matches!(r, crate::processor::ProcessingResult::Routed { .. }))
            .count();
        let needs_enrichment: Vec<String> = results
            .iter()
            .filter(|(_, r)| matches!(r, crate::processor::ProcessingResult::NeedsEnrichment))
            .map(|(name, _)| name.clone())
            .collect();

        log::info!(
            "Inbox batch: {} files routed, {} need enrichment",
            routed_count,
            needs_enrichment.len()
        );

        // Step 2: Enrich up to 5 files per batch (2 min per file × 5 = 10 min max)
        const MAX_ENRICHMENTS_PER_BATCH: usize = 5;
        let to_enrich = &needs_enrichment[..needs_enrichment.len().min(MAX_ENRICHMENTS_PER_BATCH)];
        let mut enriched_count = 0;

        // Build user context and AI model config for AI prompts
        let user_ctx = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or_default();
        let ai_config = self.ai_model_config();

        for filename in to_enrich {
            log::info!("AI enriching '{}'", filename);
            let result = crate::processor::enrich::enrich_file(
                workspace,
                filename,
                Some(&self.state),
                &profile,
                Some(&user_ctx),
                Some(&ai_config),
                None,
            );
            match &result {
                crate::processor::enrich::EnrichResult::Routed { classification, .. } => {
                    log::info!("Enriched '{}' → routed as {}", filename, classification);
                    enriched_count += 1;
                }
                crate::processor::enrich::EnrichResult::Archived { .. } => {
                    log::info!("Enriched '{}' → archived", filename);
                    enriched_count += 1;
                }
                crate::processor::enrich::EnrichResult::Error { message } => {
                    log::warn!("Enrichment failed for '{}': {}", filename, message);
                }
            }
        }

        if needs_enrichment.len() > MAX_ENRICHMENTS_PER_BATCH {
            log::info!(
                "Inbox batch: {} files deferred to next batch",
                needs_enrichment.len() - MAX_ENRICHMENTS_PER_BATCH
            );
        }

        log::info!(
            "Inbox batch complete: {} routed, {} enriched",
            routed_count,
            enriched_count
        );

        // Update execution record
        let finished_at = Utc::now();
        self.state.update_execution_record(execution_id, |r| {
            r.finished_at = Some(finished_at);
            r.success = true;
        });

        // Update last scheduled run time
        if matches!(
            trigger,
            ExecutionTrigger::Scheduled | ExecutionTrigger::Missed
        ) {
            self.state
                .set_last_scheduled_run(WorkflowId::InboxBatch, Utc::now());
        }

        // Emit inbox-updated so frontend refreshes
        let _ = self.app_handle.emit("inbox-updated", ());

        Ok(())
    }

    /// Execute the Week workflow using per-operation pipeline (matches Today pattern).
    ///
    /// Sequence:
    /// 1. Phase 1: Rust-native prepare (fetches APIs, writes directive)
    /// 2. Mechanical delivery: deliver_week() — instant, data visible immediately
    /// 3. AI enrichment: enrich_week() — progressive, fault-tolerant
    async fn execute_week(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
        record: &crate::types::ExecutionRecord,
    ) -> Result<(), ExecutionError> {
        // --- Phase 1: Prepare (Rust-native) ---
        self.emit_status_event(
            WorkflowId::Week,
            WorkflowStatus::Running {
                started_at: record.started_at,
                phase: WorkflowPhase::Preparing,
                execution_id: execution_id.to_string(),
            },
        );

        log::info!("Week pipeline Phase 1: Rust-native prepare");
        crate::prepare::orchestrate::prepare_week(&self.state, workspace).await?;

        // --- Phase 2: Mechanical delivery (instant) ---
        self.emit_status_event(
            WorkflowId::Week,
            WorkflowStatus::Running {
                started_at: Utc::now(),
                phase: WorkflowPhase::Delivering,
                execution_id: execution_id.to_string(),
            },
        );

        log::info!("Week pipeline Phase 2: mechanical delivery");
        crate::prepare::orchestrate::deliver_week(workspace)
            .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;
        let _ = self.app_handle.emit("operation-delivered", "week-overview");

        // --- Phase 3: AI enrichment (fault-tolerant) ---
        self.emit_status_event(
            WorkflowId::Week,
            WorkflowStatus::Running {
                started_at: Utc::now(),
                phase: WorkflowPhase::Enriching,
                execution_id: execution_id.to_string(),
            },
        );

        let data_dir = workspace.join("_today").join("data");
        let user_ctx = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or(crate::types::UserContext {
                name: None,
                company: None,
                title: None,
                focus: None,
            });

        let synthesis_pty = PtyManager::for_tier(ModelTier::Synthesis, &self.ai_model_config());
        let mut enrichment_error: Option<String> = None;
        if let Err(e) = crate::workflow::deliver::enrich_week(
            &data_dir,
            &synthesis_pty,
            workspace,
            &user_ctx,
            &self.state,
        ) {
            log::warn!("Week enrichment failed (non-fatal): {}", e);
            enrichment_error = Some(format!("Week enrichment incomplete: {}", e));
        }
        let _ = self.app_handle.emit("operation-delivered", "week-enriched");

        // --- Completion ---
        let finished_at = Utc::now();
        let duration_secs = (finished_at - record.started_at).num_seconds() as u64;

        self.state.update_execution_record(execution_id, |r| {
            r.finished_at = Some(finished_at);
            r.duration_secs = Some(duration_secs);
            r.success = true;
            r.error_message = enrichment_error.clone();
            r.error_phase = if enrichment_error.is_some() {
                Some(WorkflowPhase::Enriching)
            } else {
                None
            };
            r.can_retry = if enrichment_error.is_some() {
                Some(true)
            } else {
                None
            };
        });

        if matches!(
            trigger,
            ExecutionTrigger::Scheduled | ExecutionTrigger::Missed
        ) {
            self.state
                .set_last_scheduled_run(WorkflowId::Week, record.started_at);
        }

        self.emit_status_event(
            WorkflowId::Week,
            WorkflowStatus::Completed {
                finished_at,
                duration_secs,
                execution_id: execution_id.to_string(),
            },
        );

        let _ = send_notification(
            &self.app_handle,
            "Your week is ready",
            "DailyOS has prepared your weekly overview",
        );

        Ok(())
    }

    /// Execute the Today workflow using per-operation pipelines (ADR-0042).
    ///
    /// Sequence:
    /// 1. Phase 1: Rust-native prepare (fetches APIs, writes directive)
    /// 2. Load directive JSON
    /// 3. Deliver mechanical operations (schedule, actions, preps) — instant
    /// 4. Sync actions to SQLite
    /// 5. Write manifest (partial: true initially, partial: false when done)
    /// 6. AI enrichment for emails + briefing narrative
    ///
    /// Each mechanical operation emits an `operation-delivered:{op}` event
    /// so the frontend can progressively render sections as they land.
    async fn execute_today_pipeline(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
        record: &crate::types::ExecutionRecord,
    ) -> Result<(), ExecutionError> {
        // --- Phase 1: Prepare (Rust-native, ADR-0049) ---
        self.emit_status_event(
            WorkflowId::Today,
            WorkflowStatus::Running {
                started_at: record.started_at,
                phase: WorkflowPhase::Preparing,
                execution_id: execution_id.to_string(),
            },
        );

        log::info!("Today pipeline Phase 1: Rust-native prepare");
        crate::prepare::orchestrate::prepare_today(&self.state, workspace).await?;

        // --- Phase 2+3: Rust-native mechanical delivery ---
        self.emit_status_event(
            WorkflowId::Today,
            WorkflowStatus::Running {
                started_at: Utc::now(),
                phase: WorkflowPhase::Delivering,
                execution_id: execution_id.to_string(),
            },
        );

        let today_dir = workspace.join("_today");
        let data_dir = today_dir.join("data");

        // Load the directive produced by Phase 1
        let directive = crate::json_loader::load_directive(&today_dir)
            .map_err(|e| ExecutionError::ParseError(format!("Failed to load directive: {}", e)))?;

        // Deliver schedule + actions (with DB for entity ID resolution + dedup).
        // Own DB connection to avoid starving foreground IPC commands.
        let own_db = crate::db::ActionDb::open().ok();
        let schedule_data = {
            let db_ref = own_db.as_ref();
            crate::workflow::deliver::deliver_schedule(&directive, &data_dir, db_ref)
                .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?
        };
        let _ = self.app_handle.emit("operation-delivered", "schedule");
        log::info!("Today pipeline: schedule delivered");
        let actions_data = {
            let db_ref = own_db.as_ref();
            crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)
                .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?
        };
        let _ = self.app_handle.emit("operation-delivered", "actions");
        log::info!("Today pipeline: actions delivered");

        // Sync actions to SQLite (same as old post-processing)
        if let Some(ref db) = own_db {
            match crate::workflow::today::sync_actions_to_db(workspace, db) {
                Ok(count) => log::info!("Today pipeline: synced {} actions to DB", count),
                Err(e) => log::warn!("Today pipeline: action sync failed (non-fatal): {}", e),
            }
        }

        // Deliver preps (feature-gated I39)
        let prep_enabled = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .map(|c| crate::types::is_feature_enabled(c, "meetingPrep"))
            })
            .unwrap_or(true);
        let prep_paths = if prep_enabled {
            let paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)
                .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;
            // I166: reconcile hasPrep flags based on actual content
            let _ = crate::workflow::deliver::reconcile_prep_flags(&data_dir);
            let _ = self.app_handle.emit("operation-delivered", "preps");
            log::info!("Today pipeline: preps delivered");
            paths
        } else {
            log::info!("Today pipeline: preps skipped (feature disabled)");
            Vec::new()
        };

        // Deliver emails (mechanical — instant, feature-gated I39)
        let email_enabled = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .map(|c| crate::types::is_feature_enabled(c, "emailTriage"))
            })
            .unwrap_or(true);
        let mut emails_data = if email_enabled {
            match crate::workflow::deliver::deliver_emails(&directive, &data_dir) {
                Ok(data) => {
                    let _ = self.app_handle.emit("operation-delivered", "emails");
                    log::info!("Today pipeline: emails delivered");
                    if let Some(sync) = crate::workflow::deliver::extract_email_sync_status(&data) {
                        self.emit_email_sync_status(&sync);
                    }
                    data
                }
                Err(e) => {
                    log::warn!("Email delivery failed (non-fatal): {}", e);
                    let sync = self.build_email_sync_status(
                        EmailSyncState::Error,
                        EmailSyncStage::Deliver,
                        Some("email_delivery_failed".to_string()),
                        Some(format!("Email delivery failed: {}", e)),
                        Some(data_dir.join("emails.json").exists()),
                    );
                    self.emit_email_sync_status(&sync);
                    let _ = self
                        .app_handle
                        .emit("email-error", format!("Email delivery failed: {}", e));
                    crate::workflow::deliver::set_email_sync_status(&data_dir, &sync)
                        .unwrap_or_else(|_| {
                            crate::workflow::deliver::empty_emails_payload(Some(&sync))
                        })
                }
            }
        } else {
            log::info!("Today pipeline: emails skipped (feature disabled)");
            crate::workflow::deliver::empty_emails_payload(None)
        };

        // Write manifest (partial: true — AI enrichment not yet done)
        crate::workflow::deliver::deliver_manifest(
            &directive,
            &schedule_data,
            &actions_data,
            &emails_data,
            &prep_paths,
            &data_dir,
            true,
        )
        .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;

        // --- AI enrichment (progressive, fault-tolerant) ---
        self.emit_status_event(
            WorkflowId::Today,
            WorkflowStatus::Running {
                started_at: Utc::now(),
                phase: WorkflowPhase::Enriching,
                execution_id: execution_id.to_string(),
            },
        );

        // Build user context for AI enrichment prompts
        let user_ctx = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or(crate::types::UserContext {
                name: None,
                company: None,
                title: None,
                focus: None,
            });

        // Create per-tier PtyManagers (I174)
        let ai_config = self.ai_model_config();
        let extraction_pty = PtyManager::for_tier(ModelTier::Extraction, &ai_config);
        let synthesis_pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_config);

        // AI: Enrich emails (high-priority only, feature-gated I39)
        if email_enabled {
            if let Err(e) = self.enrich_emails_with_fallback(
                &data_dir,
                workspace,
                &user_ctx,
                &extraction_pty,
                &synthesis_pty,
            ) {
                log::warn!("Email enrichment failed (non-fatal): {}", e);
                let sync = self.build_email_sync_status(
                    EmailSyncState::Warning,
                    EmailSyncStage::Enrich,
                    Some("email_enrichment_failed".to_string()),
                    Some(format!("Email AI summaries unavailable: {}", e)),
                    Some(true),
                );
                if let Ok(updated) =
                    crate::workflow::deliver::set_email_sync_status(&data_dir, &sync)
                {
                    emails_data = updated;
                }
                self.emit_email_sync_status(&sync);
                let _ = self.app_handle.emit(
                    "email-enrichment-warning",
                    format!("Email AI summaries unavailable: {}", e),
                );
            }
            let _ = self
                .app_handle
                .emit("operation-delivered", "emails-enriched");
        }

        // AI: Enrich prep agendas (feature-gated I39)
        if prep_enabled {
            if let Err(e) =
                crate::workflow::deliver::enrich_preps(&data_dir, &extraction_pty, workspace)
            {
                log::warn!("Prep enrichment failed (non-fatal): {}", e);
            }
            let _ = self
                .app_handle
                .emit("operation-delivered", "preps-enriched");
        }

        // AI: Generate briefing narrative
        if let Err(e) = crate::workflow::deliver::enrich_briefing(
            &data_dir,
            &synthesis_pty,
            workspace,
            &user_ctx,
            &self.state,
        ) {
            log::warn!("Briefing narrative failed (non-fatal): {}", e);
        }
        let _ = self.app_handle.emit("operation-delivered", "briefing");

        // Final manifest (partial: false — all ops complete)
        crate::workflow::deliver::deliver_manifest(
            &directive,
            &schedule_data,
            &actions_data,
            &emails_data,
            &prep_paths,
            &data_dir,
            false,
        )
        .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;

        // --- Completion ---
        let finished_at = Utc::now();
        let duration_secs = (finished_at - record.started_at).num_seconds() as u64;

        self.state.update_execution_record(execution_id, |r| {
            r.finished_at = Some(finished_at);
            r.duration_secs = Some(duration_secs);
            r.success = true;
        });

        if matches!(
            trigger,
            ExecutionTrigger::Scheduled | ExecutionTrigger::Missed
        ) {
            self.state
                .set_last_scheduled_run(WorkflowId::Today, record.started_at);
        }

        self.emit_status_event(
            WorkflowId::Today,
            WorkflowStatus::Completed {
                finished_at,
                duration_secs,
                execution_id: execution_id.to_string(),
            },
        );

        let _ = send_notification(
            &self.app_handle,
            "Your day is ready",
            "DailyOS has prepared your briefing",
        );

        Ok(())
    }

    /// Run the three-phase week workflow (Rust-native, ADR-0049)
    /// Get workspace path from config
    fn get_workspace_path(&self) -> Result<PathBuf, ExecutionError> {
        let config = self
            .state
            .config
            .read()
            .map_err(|_| ExecutionError::ConfigurationError("Lock poisoned".to_string()))?;

        let config = config.as_ref().ok_or_else(|| {
            ExecutionError::ConfigurationError("No configuration loaded".to_string())
        })?;

        let path = PathBuf::from(&config.workspace_path);
        if !path.exists() {
            return Err(ExecutionError::WorkspaceNotFound(path));
        }

        Ok(path)
    }

    /// Execute standalone email refresh (I20, ADR-0049 Rust-native).
    ///
    /// 1. Check that /today pipeline is not currently running
    /// 2. Fetch emails from Gmail, classify, write directive (Rust-native)
    /// 3. Read refresh directive and deliver via deliver_emails()
    /// 4. Optionally run AI enrichment (fault-tolerant)
    /// 5. Emit operation-delivered for frontend refresh
    /// 6. Clean up refresh directive
    pub async fn execute_email_refresh(&self, workspace: &Path) -> Result<(), String> {
        // Guard: reject if /today pipeline is currently running
        let today_status = self.state.get_workflow_status(WorkflowId::Today);
        if matches!(today_status, WorkflowStatus::Running { .. }) {
            return Err("Cannot refresh emails while /today pipeline is running".to_string());
        }
        let data_dir = workspace.join("_today").join("data");

        // Step 1: Rust-native email fetch + classify
        log::info!("Email refresh: Rust-native fetch + classify");
        if let Err(e) = crate::prepare::orchestrate::refresh_emails(&self.state, workspace).await {
            let sync = self.build_email_sync_status(
                EmailSyncState::Error,
                EmailSyncStage::Refresh,
                Some("email_refresh_failed".to_string()),
                Some(format!("Email refresh failed: {}", e)),
                Some(data_dir.join("emails.json").exists()),
            );
            let _ = crate::workflow::deliver::set_email_sync_status(&data_dir, &sync);
            self.emit_email_sync_status(&sync);
            let _ = self
                .app_handle
                .emit("email-error", format!("Email refresh failed: {}", e));
            return Err(format!("Email refresh failed: {}", e));
        }

        // Step 2: Read refresh directive
        let refresh_path = data_dir.join("email-refresh-directive.json");

        if !refresh_path.exists() {
            return Err("Email refresh did not produce directive".to_string());
        }

        let raw = std::fs::read_to_string(&refresh_path)
            .map_err(|e| format!("Failed to read refresh directive: {}", e))?;
        let refresh_data: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse refresh directive: {}", e))?;

        // Step 3: Build emails data matching deliver_emails output shape
        let emails_section = refresh_data.get("emails").cloned().unwrap_or(json!({}));

        let map_email = |e: &serde_json::Value, default_priority: &str| -> serde_json::Value {
            json!({
                "id": e.get("id").and_then(serde_json::Value::as_str).unwrap_or(""),
                "sender": e.get("from").and_then(serde_json::Value::as_str).unwrap_or(""),
                "senderEmail": e.get("from_email").and_then(serde_json::Value::as_str).unwrap_or(""),
                "subject": e.get("subject").and_then(serde_json::Value::as_str).unwrap_or(""),
                "snippet": e.get("snippet").and_then(serde_json::Value::as_str).unwrap_or(""),
                "priority": e.get("priority").and_then(serde_json::Value::as_str).unwrap_or(default_priority),
            })
        };

        let high_priority: Vec<serde_json::Value> = emails_section
            .get("highPriority")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().map(|e| map_email(e, "high")).collect())
            .unwrap_or_default();

        // Classified contains medium + low emails
        let classified = emails_section
            .get("classified")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut medium_priority: Vec<serde_json::Value> = Vec::new();
        let mut low_priority: Vec<serde_json::Value> = Vec::new();
        for e in &classified {
            let prio = e
                .get("priority")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("medium");
            let mapped = map_email(e, prio);
            match prio {
                "low" => low_priority.push(mapped),
                _ => medium_priority.push(mapped),
            }
        }

        let now = Utc::now().to_rfc3339();
        let sync = EmailSyncStatus {
            state: EmailSyncState::Ok,
            stage: EmailSyncStage::Refresh,
            code: None,
            message: None,
            using_last_known_good: Some(false),
            can_retry: Some(true),
            last_attempt_at: Some(now.clone()),
            last_success_at: Some(now),
        };
        let emails_json = json!({
            "highPriority": high_priority,
            "mediumPriority": medium_priority,
            "lowPriority": low_priority,
            "stats": {
                "highCount": high_priority.len(),
                "mediumCount": medium_priority.len(),
                "lowCount": low_priority.len(),
                "total": high_priority.len() + medium_priority.len() + low_priority.len(),
            },
            "sync": serde_json::to_value(&sync)
                .map_err(|e| format!("Failed to serialize email sync status: {}", e))?,
        });

        crate::workflow::deliver::write_json(&data_dir.join("emails.json"), &emails_json)?;
        let _ = self.app_handle.emit("operation-delivered", "emails");
        self.emit_email_sync_status(&sync);
        log::info!(
            "Email refresh: emails.json written ({} high, {} medium, {} low)",
            high_priority.len(),
            medium_priority.len(),
            low_priority.len()
        );

        // Step 4: AI enrichment (fault-tolerant)
        let user_ctx = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or(crate::types::UserContext {
                name: None,
                company: None,
                title: None,
                focus: None,
            });
        let ai_config = self.ai_model_config();
        let extraction_pty = PtyManager::for_tier(ModelTier::Extraction, &ai_config);
        let synthesis_pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_config);
        if let Err(e) = self.enrich_emails_with_fallback(
            &data_dir,
            workspace,
            &user_ctx,
            &extraction_pty,
            &synthesis_pty,
        ) {
            log::warn!("Email refresh: AI enrichment failed (non-fatal): {}", e);
            let sync = self.build_email_sync_status(
                EmailSyncState::Warning,
                EmailSyncStage::Enrich,
                Some("email_enrichment_failed".to_string()),
                Some(format!("Email AI summaries unavailable: {}", e)),
                Some(true),
            );
            let _ = crate::workflow::deliver::set_email_sync_status(&data_dir, &sync);
            self.emit_email_sync_status(&sync);
            let _ = self.app_handle.emit(
                "email-enrichment-warning",
                format!("Email AI summaries unavailable: {}", e),
            );
        }
        let _ = self
            .app_handle
            .emit("operation-delivered", "emails-enriched");

        // Step 5: Clean up refresh directive
        let _ = std::fs::remove_file(&refresh_path);

        log::info!("Email refresh complete");
        Ok(())
    }

    /// Refresh only focus/briefing narrative without running full /today pipeline.
    ///
    /// Re-runs `enrich_briefing()` against existing `_today/data/schedule.json`
    /// so users can update the focus statement and narrative independently.
    pub async fn execute_focus_refresh(&self, workspace: &Path) -> Result<(), String> {
        // Guard: reject if /today pipeline is currently running.
        // Note: TOCTOU race between this check and execution start is accepted —
        // same pattern as execute_email_refresh. Practical risk is negligible:
        // both operations are user-initiated in a single-user desktop app.
        let today_status = self.state.get_workflow_status(WorkflowId::Today);
        if matches!(today_status, WorkflowStatus::Running { .. }) {
            return Err("Cannot refresh focus while /today pipeline is running".to_string());
        }

        let data_dir = workspace.join("_today").join("data");
        let schedule_path = data_dir.join("schedule.json");
        if !schedule_path.exists() {
            return Err("No daily briefing data found. Run briefing first.".to_string());
        }

        let user_ctx = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or(crate::types::UserContext {
                name: None,
                company: None,
                title: None,
                focus: None,
            });

        let ai_config = self.ai_model_config();
        let synthesis_pty = PtyManager::for_tier(ModelTier::Synthesis, &ai_config);

        crate::workflow::deliver::enrich_briefing(
            &data_dir,
            &synthesis_pty,
            workspace,
            &user_ctx,
            &self.state,
        )?;

        let _ = self.app_handle.emit("operation-delivered", "briefing");
        log::info!("Focus refresh complete");
        Ok(())
    }

    /// Emit a workflow status event to the frontend
    fn emit_status_event(&self, workflow: WorkflowId, status: WorkflowStatus) {
        // Update state
        self.state.set_workflow_status(workflow, status.clone());

        // Emit to frontend
        let event_name = format!("workflow-status-{}", workflow);
        if let Err(e) = self.app_handle.emit(&event_name, &status) {
            log::error!("Failed to emit status event: {}", e);
        }

        // Also emit generic workflow event
        let _ = self.app_handle.emit("workflow-status", &status);

        // Emit completed event for dashboard refresh
        if matches!(status, WorkflowStatus::Completed { .. }) {
            let _ = self.app_handle.emit("workflow-completed", workflow);
        }
    }
}

/// Request a manual workflow execution
pub fn request_workflow_execution(
    sender: &mpsc::Sender<SchedulerMessage>,
    workflow: WorkflowId,
) -> Result<(), String> {
    sender
        .try_send(SchedulerMessage {
            workflow,
            trigger: ExecutionTrigger::Manual,
        })
        .map_err(|e| format!("Failed to queue workflow: {}", e))
}

#[cfg(test)]
mod tests {
    use super::Executor;

    #[test]
    fn test_model_unavailable_error_detection() {
        assert!(Executor::is_model_unavailable_error(
            "Configuration error: model_unavailable: unknown model"
        ));
        assert!(!Executor::is_model_unavailable_error(
            "Claude enrichment failed: timeout"
        ));
    }
}
