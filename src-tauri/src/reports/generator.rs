//! Shared two-phase PTY dispatch for all report types.
//!
//! Pattern: gather input under brief DB lock → release lock →
//! run PTY (long-running) → write result to DB.

use std::path::PathBuf;

use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;

/// Everything needed to run a report generation PTY call.
/// All data is owned so it can be sent to a blocking task.
pub struct ReportGeneratorInput {
    pub entity_id: String,
    pub entity_type: String,
    pub report_type: String,
    pub entity_name: String,
    pub workspace: PathBuf,
    pub prompt: String,
    pub ai_models: AiModelConfig,
    pub intel_hash: String,
    /// Optional serialized extra data for multi-phase reports (e.g. BookMetrics).
    #[allow(dead_code)]
    pub extra_data: Option<String>,
}

/// Phase 2: Run the PTY call for report generation.
/// No DB lock held — this is the long-running operation.
pub fn run_report_generation(input: &ReportGeneratorInput) -> Result<String, String> {
    let timeout_secs = match input.report_type.as_str() {
        // These report types still use monolithic prompts for now.
        // Keep a larger timeout until they are decomposed like BoB/SWOT.
        "monthly_wrapped" | "weekly_impact" | "ebr_qbr" => 180,
        "account_health" => 120,
        _ => 30,
    };
    let pty = PtyManager::for_tier(ModelTier::Synthesis, &input.ai_models)
        .with_timeout(timeout_secs)
        .with_nice_priority(10);

    let output = pty
        .spawn_claude(&input.workspace, &input.prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // Audit trail
    let _ = crate::audit::write_audit_entry(
        &input.workspace,
        &format!("report_{}", input.report_type),
        &input.entity_id,
        &output.stdout,
    );

    log::info!(
        "reports: generated {} for {} ({})",
        input.report_type,
        input.entity_name,
        input.entity_id
    );

    Ok(output.stdout)
}
