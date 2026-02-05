//! Executor for three-phase workflow orchestration
//!
//! Runs workflows through: Prepare → Enrich → Deliver
//! - Phase 1: Python script prepares data
//! - Phase 2: Claude Code enriches with AI
//! - Phase 3: Python script delivers output

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::error::{ExecutionError, WorkflowError};
use crate::notification::send_notification;
use crate::pty::{run_python_script, PtyManager};
use crate::scheduler::SchedulerMessage;
use crate::state::{create_execution_record, AppState};
use crate::types::{ExecutionTrigger, WorkflowId, WorkflowPhase, WorkflowStatus};
use crate::workflow::Workflow;

/// Timeout for Python scripts (60 seconds)
const SCRIPT_TIMEOUT_SECS: u64 = 60;

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

        // Other workflows: three-phase pattern with notifications

        // Get the workflow implementation
        let workflow = Workflow::from_id(workflow_id);

        // Emit started event
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: record.started_at,
            phase: WorkflowPhase::Preparing,
            execution_id: execution_id.clone(),
        });

        let result = self
            .run_three_phase(&workflow, &workspace, &execution_id, workflow_id)
            .await;

        // Update execution record
        let finished_at = Utc::now();
        let duration_secs = (finished_at - record.started_at).num_seconds() as u64;

        match &result {
            Ok(_) => {
                self.state.update_execution_record(&execution_id, |r| {
                    r.finished_at = Some(finished_at);
                    r.duration_secs = Some(duration_secs);
                    r.success = true;
                });

                // Update last scheduled run time
                if matches!(trigger, ExecutionTrigger::Scheduled | ExecutionTrigger::Missed) {
                    self.state
                        .set_last_scheduled_run(workflow_id, record.started_at);
                }

                // Emit completed event
                self.emit_status_event(workflow_id, WorkflowStatus::Completed {
                    finished_at,
                    duration_secs,
                    execution_id: execution_id.clone(),
                });

                // Send success notification
                let _ = send_notification(
                    &self.app_handle,
                    "Your day is ready",
                    "DailyOS has prepared your briefing",
                );
            }
            Err(e) => {
                self.state.update_execution_record(&execution_id, |r| {
                    r.finished_at = Some(finished_at);
                    r.duration_secs = Some(duration_secs);
                    r.success = false;
                    r.error_message = Some(e.to_string());
                });

                // Emit failed event
                self.emit_status_event(workflow_id, WorkflowStatus::Failed {
                    error: WorkflowError::from(e),
                    execution_id: execution_id.clone(),
                });

                // Send error notification
                let _ = send_notification(
                    &self.app_handle,
                    "Workflow failed",
                    &e.to_string(),
                );
            }
        }

        result
    }

    /// Execute archive workflow (pure Rust, silent operation)
    ///
    /// Archive is special: no three-phase pattern, no AI, no notification.
    /// Just moves _today/*.md files to archive/YYYY-MM-DD/.
    async fn execute_archive(
        &self,
        workspace: &Path,
        execution_id: &str,
        trigger: ExecutionTrigger,
    ) -> Result<(), ExecutionError> {
        use crate::workflow::archive::run_archive;

        log::info!("Running archive workflow (silent)");

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

    /// Run the three-phase workflow
    async fn run_three_phase(
        &self,
        workflow: &Workflow,
        workspace: &Path,
        execution_id: &str,
        workflow_id: WorkflowId,
    ) -> Result<(), ExecutionError> {
        // Phase 1: Prepare
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Preparing,
            execution_id: execution_id.to_string(),
        });

        let prepare_script = get_script_path(workspace, workflow.prepare_script());
        log::info!("Phase 1: Running {}", prepare_script.display());
        run_python_script(&prepare_script, workspace, SCRIPT_TIMEOUT_SECS)?;

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

        // Phase 3: Deliver
        self.emit_status_event(workflow_id, WorkflowStatus::Running {
            started_at: Utc::now(),
            phase: WorkflowPhase::Delivering,
            execution_id: execution_id.to_string(),
        });

        let deliver_script = get_script_path(workspace, workflow.deliver_script());
        log::info!("Phase 3: Running {}", deliver_script.display());
        run_python_script(&deliver_script, workspace, SCRIPT_TIMEOUT_SECS)?;

        Ok(())
    }

    /// Get workspace path from config
    fn get_workspace_path(&self) -> Result<PathBuf, ExecutionError> {
        let config = self
            .state
            .config
            .lock()
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

/// Get the path to a script, checking workspace override first
fn get_script_path(workspace: &Path, script_name: &str) -> PathBuf {
    // First check for workspace override
    let workspace_script = workspace.join("_tools").join(script_name);
    if workspace_script.exists() {
        return workspace_script;
    }

    // Fall back to bundled scripts (extracted at runtime)
    // For now, we expect scripts in _tools directory
    // TODO: Bundle scripts with app and extract to temp
    workspace_script
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
