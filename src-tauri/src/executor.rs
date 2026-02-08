//! Workflow execution engine
//!
//! Each workflow has its own execution strategy:
//! - Today: per-operation pipeline (ADR-0042) — Rust-native prepare + delivery
//! - Week: three-phase (Prepare → Enrich → Deliver)
//! - Archive: pure Rust reconciliation + file moves
//! - InboxBatch: direct processor calls

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::error::{ExecutionError, WorkflowError};
use crate::notification::send_notification;
use crate::pty::PtyManager;
use crate::scheduler::SchedulerMessage;
use crate::state::{create_execution_record, AppState};
use crate::types::{ExecutionTrigger, WorkflowId, WorkflowPhase, WorkflowStatus};
use crate::workflow::Workflow;

/// Executor manages workflow execution
pub struct Executor {
    state: Arc<AppState>,
    app_handle: AppHandle,
    pty_manager: PtyManager,
}

impl Executor {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Self {
        Self {
            state,
            app_handle,
            pty_manager: PtyManager::new(),
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

        // Archive workflow: pure Rust, no three-phase, no notification
        if workflow_id == WorkflowId::Archive {
            return self
                .execute_archive(&workspace, &execution_id, trigger)
                .await;
        }

        // Inbox batch: direct processor calls, no three-phase
        if workflow_id == WorkflowId::InboxBatch {
            return self
                .execute_inbox_batch(&workspace, &execution_id, trigger)
                .await;
        }

        // Week workflow: three-phase
        if workflow_id == WorkflowId::Week {
            return self
                .execute_week(&workspace, &execution_id, trigger, &record)
                .await;
        }

        // Today workflow: per-operation pipeline (ADR-0042)
        // Phase 1 (Python) then Rust-native mechanical delivery — no Phase 2/3
        return self
            .execute_today_pipeline(&workspace, &execution_id, trigger, &record)
            .await;
    }

    /// Execute archive workflow (pure Rust, silent operation)
    ///
    /// Sequence (ADR-0040):
    /// 1. Reconcile: read schedule.json, check transcript status, compute stats
    /// 2. Archive: move files, clean data/
    /// 3. Persist: write day-summary.json, next-morning-flags.json, meetings to DB
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
        // Lock DB briefly for reconciliation, then drop before the await
        let recon = {
            let db_guard = self.state.db.lock().ok();
            let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
            let r = reconcile::run_reconciliation(workspace, db_ref);
            // db_guard dropped here
            r
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
                .and_then(|g| g.as_ref().map(|c| crate::types::is_feature_enabled(c, "impactRollup")))
                .unwrap_or(false);

            if impact_enabled {
                if let Ok(db_guard) = self.state.db.lock() {
                    if let Some(db) = db_guard.as_ref() {
                        match crate::workflow::impact_rollup::rollup_daily_impact(
                            workspace,
                            db,
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
        }

        // Step 2: Archive (move files, clean data/)
        let result = run_archive(workspace).await.map_err(|e| {
            ExecutionError::ScriptFailed {
                code: 1,
                stderr: e,
            }
        })?;

        log::info!(
            "Archive complete: {} files moved{}",
            result.files_archived,
            if result.archive_path.is_empty() {
                String::new()
            } else {
                format!(" to {}", result.archive_path)
            }
        );

        // Step 3: Persist reconciliation results
        // Write day-summary.json to archive directory (if files were archived)
        if !result.archive_path.is_empty() {
            let archive_path = std::path::Path::new(&result.archive_path);
            if let Err(e) = reconcile::write_day_summary(archive_path, &recon, result.files_archived) {
                log::warn!("Failed to write day summary: {}", e);
            }
        }

        // Write next-morning-flags.json to _today/data/ (survives until next archive)
        let today_dir = workspace.join("_today");
        if let Err(e) = reconcile::write_morning_flags(&today_dir, &recon) {
            log::warn!("Failed to write morning flags: {}", e);
        }

        // Re-lock DB briefly to persist meetings
        if let Ok(db_guard) = self.state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                reconcile::persist_meetings(db, &recon);
            }
        }

        // Update execution record
        let finished_at = Utc::now();
        self.state.update_execution_record(execution_id, |r| {
            r.finished_at = Some(finished_at);
            r.success = true;
        });

        // Update last scheduled run time
        if matches!(trigger, ExecutionTrigger::Scheduled | ExecutionTrigger::Missed) {
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
            .and_then(|g| g.as_ref().map(|c| crate::types::is_feature_enabled(c, "inboxProcessing")))
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

        // Get DB reference
        let db_guard = self.state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());

        // Step 1: Quick-classify all inbox files
        let results = crate::processor::process_all(workspace, db_ref, &profile);

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

        // Build user context for AI prompts
        let user_ctx = self.state.config.read().ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or_default();

        for filename in to_enrich {
            log::info!("AI enriching '{}'", filename);
            let result = crate::processor::enrich::enrich_file(workspace, filename, db_ref, &profile, Some(&user_ctx));
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
        if matches!(trigger, ExecutionTrigger::Scheduled | ExecutionTrigger::Missed) {
            self.state
                .set_last_scheduled_run(WorkflowId::InboxBatch, Utc::now());
        }

        // Emit inbox-updated so frontend refreshes
        let _ = self.app_handle.emit("inbox-updated", ());

        Ok(())
    }

    /// Execute week workflow (three-phase)
    async fn execute_week(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
        record: &crate::types::ExecutionRecord,
    ) -> Result<(), ExecutionError> {
        let workflow = Workflow::from_id(WorkflowId::Week);

        // Emit started event
        self.emit_status_event(WorkflowId::Week, WorkflowStatus::Running {
            started_at: record.started_at,
            phase: WorkflowPhase::Preparing,
            execution_id: execution_id.to_string(),
        });

        let result = self
            .run_three_phase(&workflow, workspace, execution_id, WorkflowId::Week)
            .await;

        let finished_at = Utc::now();
        let duration_secs = (finished_at - record.started_at).num_seconds() as u64;

        match &result {
            Ok(_) => {
                self.state.update_execution_record(execution_id, |r| {
                    r.finished_at = Some(finished_at);
                    r.duration_secs = Some(duration_secs);
                    r.success = true;
                });

                if matches!(trigger, ExecutionTrigger::Scheduled | ExecutionTrigger::Missed) {
                    self.state
                        .set_last_scheduled_run(WorkflowId::Week, record.started_at);
                }

                self.emit_status_event(WorkflowId::Week, WorkflowStatus::Completed {
                    finished_at,
                    duration_secs,
                    execution_id: execution_id.to_string(),
                });

                let _ = send_notification(
                    &self.app_handle,
                    "Your week is ready",
                    "DailyOS has prepared your weekly overview",
                );
            }
            Err(e) => {
                self.state.update_execution_record(execution_id, |r| {
                    r.finished_at = Some(finished_at);
                    r.duration_secs = Some(duration_secs);
                    r.success = false;
                    r.error_message = Some(e.to_string());
                });

                self.emit_status_event(WorkflowId::Week, WorkflowStatus::Failed {
                    error: WorkflowError::from(e),
                    execution_id: execution_id.to_string(),
                });

                let _ = send_notification(
                    &self.app_handle,
                    "Week workflow failed",
                    &e.to_string(),
                );
            }
        }

        result
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
        self.emit_status_event(WorkflowId::Today, WorkflowStatus::Running {
            started_at: record.started_at,
            phase: WorkflowPhase::Preparing,
            execution_id: execution_id.to_string(),
        });

        log::info!("Today pipeline Phase 1: Rust-native prepare");
        crate::prepare::orchestrate::prepare_today(&self.state, workspace).await?;

        // --- Phase 2+3: Rust-native mechanical delivery ---
        self.emit_status_event(WorkflowId::Today, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Delivering,
            execution_id: execution_id.to_string(),
        });

        let today_dir = workspace.join("_today");
        let data_dir = today_dir.join("data");

        // Load the directive produced by Phase 1
        let directive = crate::json_loader::load_directive(&today_dir).map_err(|e| {
            ExecutionError::ParseError(format!("Failed to load directive: {}", e))
        })?;

        // Deliver schedule
        let schedule_data = crate::workflow::deliver::deliver_schedule(&directive, &data_dir)
            .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;
        let _ = self.app_handle.emit("operation-delivered", "schedule");
        log::info!("Today pipeline: schedule delivered");

        // Deliver actions (with DB for dedup)
        let db_guard = self.state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        let actions_data =
            crate::workflow::deliver::deliver_actions(&directive, &data_dir, db_ref)
                .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;
        let _ = self.app_handle.emit("operation-delivered", "actions");
        log::info!("Today pipeline: actions delivered");

        // Sync actions to SQLite (same as old post-processing)
        if let Some(db) = db_ref {
            match crate::workflow::today::sync_actions_to_db(workspace, db) {
                Ok(count) => log::info!("Today pipeline: synced {} actions to DB", count),
                Err(e) => log::warn!("Today pipeline: action sync failed (non-fatal): {}", e),
            }
        }
        // Drop DB guard before any awaits
        drop(db_guard);

        // Deliver preps (feature-gated I39)
        let prep_enabled = self
            .state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| crate::types::is_feature_enabled(c, "meetingPrep")))
            .unwrap_or(true);
        let prep_paths = if prep_enabled {
            let paths = crate::workflow::deliver::deliver_preps(&directive, &data_dir)
                .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;
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
            .and_then(|g| g.as_ref().map(|c| crate::types::is_feature_enabled(c, "emailTriage")))
            .unwrap_or(true);
        let emails_data = if email_enabled {
            let data = crate::workflow::deliver::deliver_emails(&directive, &data_dir)
                .unwrap_or_else(|e| {
                    log::warn!("Email delivery failed (non-fatal): {}", e);
                    json!({})
                });
            let _ = self.app_handle.emit("operation-delivered", "emails");
            log::info!("Today pipeline: emails delivered");
            data
        } else {
            log::info!("Today pipeline: emails skipped (feature disabled)");
            json!({})
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
        self.emit_status_event(WorkflowId::Today, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Enriching,
            execution_id: execution_id.to_string(),
        });

        // Build user context for AI enrichment prompts
        let user_ctx = self.state.config.read().ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or_else(|| crate::types::UserContext { name: None, company: None, title: None, focus: None });

        // AI: Enrich emails (high-priority only, feature-gated I39)
        if email_enabled {
            if let Err(e) = crate::workflow::deliver::enrich_emails(
                &data_dir,
                &self.pty_manager,
                &workspace,
                &user_ctx,
            ) {
                log::warn!("Email enrichment failed (non-fatal): {}", e);
            }
            let _ = self.app_handle.emit("operation-delivered", "emails-enriched");
        }

        // AI: Enrich prep agendas (feature-gated I39)
        if prep_enabled {
            if let Err(e) = crate::workflow::deliver::enrich_preps(
                &data_dir,
                &self.pty_manager,
                &workspace,
            ) {
                log::warn!("Prep enrichment failed (non-fatal): {}", e);
            }
            let _ = self.app_handle.emit("operation-delivered", "preps-enriched");
        }

        // AI: Generate briefing narrative
        if let Err(e) = crate::workflow::deliver::enrich_briefing(
            &data_dir,
            &self.pty_manager,
            &workspace,
            &user_ctx,
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

        if matches!(trigger, ExecutionTrigger::Scheduled | ExecutionTrigger::Missed) {
            self.state
                .set_last_scheduled_run(WorkflowId::Today, record.started_at);
        }

        self.emit_status_event(WorkflowId::Today, WorkflowStatus::Completed {
            finished_at,
            duration_secs,
            execution_id: execution_id.to_string(),
        });

        let _ = send_notification(
            &self.app_handle,
            "Your day is ready",
            "DailyOS has prepared your briefing",
        );

        Ok(())
    }

    /// Run the three-phase week workflow (Rust-native, ADR-0049)
    async fn run_three_phase(
        &self,
        workflow: &Workflow,
        workspace: &Path,
        execution_id: &str,
        workflow_id: WorkflowId,
    ) -> Result<(), ExecutionError> {
        // Phase 1: Prepare (Rust-native)
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Preparing,
            execution_id: execution_id.to_string(),
        });

        log::info!("Phase 1: Rust-native prepare for {:?}", workflow_id);
        crate::prepare::orchestrate::prepare_week(&self.state, workspace).await?;

        // Phase 2: Enrich with Claude
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Enriching,
            execution_id: execution_id.to_string(),
        });

        log::info!("Phase 2: Running Claude with command '{}'", workflow.claude_command());
        let _output = self
            .pty_manager
            .spawn_claude(workspace, workflow.claude_command())?;

        // Phase 3: Deliver (Rust-native)
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Delivering,
            execution_id: execution_id.to_string(),
        });

        log::info!("Phase 3: Rust-native deliver for {:?}", workflow_id);
        crate::prepare::orchestrate::deliver_week(workspace)
            .map_err(|e| ExecutionError::ScriptFailed { code: 1, stderr: e })?;

        Ok(())
    }

    /// Get workspace path from config
    fn get_workspace_path(&self) -> Result<PathBuf, ExecutionError> {
        let config = self
            .state
            .config
            .read()
            .map_err(|_| ExecutionError::ConfigurationError("Lock poisoned".to_string()))?;

        let config = config
            .as_ref()
            .ok_or_else(|| ExecutionError::ConfigurationError("No configuration loaded".to_string()))?;

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
    pub async fn execute_email_refresh(
        &self,
        workspace: &Path,
    ) -> Result<(), String> {
        // Guard: reject if /today pipeline is currently running
        let today_status = self.state.get_workflow_status(WorkflowId::Today);
        if matches!(today_status, WorkflowStatus::Running { .. }) {
            return Err("Cannot refresh emails while /today pipeline is running".to_string());
        }

        // Step 1: Rust-native email fetch + classify
        log::info!("Email refresh: Rust-native fetch + classify");
        crate::prepare::orchestrate::refresh_emails(&self.state, workspace)
            .await
            .map_err(|e| format!("Email refresh failed: {}", e))?;

        // Step 2: Read refresh directive
        let data_dir = workspace.join("_today").join("data");
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

        let high_priority = emails_section
            .get("highPriority")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let medium_count = emails_section
            .get("mediumCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let low_count = emails_section
            .get("lowCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let emails_json = json!({
            "highPriority": high_priority.iter().map(|e| {
                json!({
                    "id": e.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    "sender": e.get("from").and_then(|v| v.as_str()).unwrap_or(""),
                    "senderEmail": e.get("from_email").and_then(|v| v.as_str()).unwrap_or(""),
                    "subject": e.get("subject").and_then(|v| v.as_str()).unwrap_or(""),
                    "snippet": e.get("snippet").and_then(|v| v.as_str()).unwrap_or(""),
                    "priority": "high",
                })
            }).collect::<Vec<_>>(),
            "stats": {
                "highCount": high_priority.len(),
                "mediumCount": medium_count,
                "lowCount": low_count,
                "total": high_priority.len() as u64 + medium_count + low_count,
            }
        });

        crate::workflow::deliver::write_json(
            &data_dir.join("emails.json"),
            &emails_json,
        )?;
        let _ = self.app_handle.emit("operation-delivered", "emails");
        log::info!("Email refresh: emails.json written ({} high)", high_priority.len());

        // Step 4: AI enrichment (fault-tolerant)
        let user_ctx = self.state.config.read().ok()
            .and_then(|g| g.as_ref().map(crate::types::UserContext::from_config))
            .unwrap_or_else(|| crate::types::UserContext { name: None, company: None, title: None, focus: None });
        if let Err(e) = crate::workflow::deliver::enrich_emails(
            &data_dir,
            &self.pty_manager,
            workspace,
            &user_ctx,
        ) {
            log::warn!("Email refresh: AI enrichment failed (non-fatal): {}", e);
        }
        let _ = self.app_handle.emit("operation-delivered", "emails-enriched");

        // Step 5: Clean up refresh directive
        let _ = std::fs::remove_file(&refresh_path);

        log::info!("Email refresh complete");
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
