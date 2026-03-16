//! Shared prompt building blocks for all report types.

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::intelligence::user_context::{search_user_attachments, search_user_context};
use crate::intelligence::IntelligenceContext;

/// Build the shared preamble for any report prompt.
/// Includes entity facts, user role context, and report type framing.
pub fn build_report_preamble(entity_name: &str, report_type: &str, entity_type: &str) -> String {
    let mut preamble = String::with_capacity(2048);

    let framing = match report_type {
        "swot" => "a SWOT analysis (Strengths, Weaknesses, Opportunities, Threats)",
        "account_health" => "an Account Health Review",
        "ebr_qbr" => "an Executive Business Review (EBR/QBR)",
        "weekly_impact" => "a Weekly Impact Report",
        "monthly_wrapped" => "a Monthly Wrapped report",
        "book_of_business" => "a Book of Business Review",
        _ => "a strategic report",
    };

    preamble.push_str(&format!(
        "You are a senior customer success strategist preparing {} for **{}** ({}).\n",
        framing,
        crate::util::sanitize_external_field(entity_name),
        entity_type
    ));
    preamble.push_str("Ground every claim in the data provided. Cite sources by event ID or meeting date when possible.\n");
    preamble.push_str("Use executive-ready language — direct, specific, no filler.\n\n");

    preamble
}

/// Append intelligence context data to a prompt.
/// Builds a structured data block from the IntelligenceContext.
///
/// Order: synthesized intelligence first (authoritative), raw data second (supporting).
pub fn append_intel_context(prompt: &mut String, ctx: &IntelligenceContext) {
    // Synthesized entity intelligence — always first, always the primary source.
    // This is the result of deep enrichment from transcripts, signals, and workspace
    // files. It should be treated as more authoritative than the raw data below.
    if let Some(ref intel_json) = ctx.prior_intelligence {
        prompt.push_str("## Entity Intelligence Assessment\n");
        prompt.push_str("(Synthesized from enrichment — treat as authoritative. Raw data below is supporting context.)\n");
        prompt.push_str(&crate::util::wrap_user_data(intel_json));
        prompt.push_str("\n\n");
    }

    if !ctx.facts_block.is_empty() {
        prompt.push_str("## Entity Facts\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.facts_block));
        prompt.push_str("\n\n");
    }

    if !ctx.meeting_history.is_empty() {
        prompt.push_str("## Recent Meeting History (last 90 days)\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.meeting_history));
        prompt.push_str("\n\n");
    }

    if !ctx.open_actions.is_empty() {
        prompt.push_str("## Open Actions\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.open_actions));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_captures.is_empty() {
        prompt.push_str("## Recent Captures (wins/risks/decisions)\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.recent_captures));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_email_signals.is_empty() {
        prompt.push_str("## Email Signals\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.recent_email_signals));
        prompt.push_str("\n\n");
    }

    if !ctx.stakeholders.is_empty() {
        prompt.push_str("## Stakeholders\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.stakeholders));
        prompt.push_str("\n\n");
    }

    if !ctx.file_contents.is_empty() {
        prompt.push_str("## Workspace Files\n");
        prompt.push_str(&crate::util::wrap_user_data(&ctx.file_contents));
        prompt.push_str("\n\n");
    }
}

/// Query user context entries and attachments for context relevant to a report.
/// Appends matching content as a "## Relevant Product Context" section.
///
/// Threshold: 0.70 cosine similarity — only high-confidence matches.
/// Returns silently if the embedding model is unavailable or no matches exceed the threshold.
pub fn append_user_context(
    prompt: &mut String,
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    query: &str,
) {
    const THRESHOLD: f32 = 0.70;
    const LIMIT: usize = 1;

    let mut context_parts: Vec<String> = Vec::new();

    // Search user context entries
    let entry_matches = search_user_context(db, model, query, LIMIT, THRESHOLD);
    for m in &entry_matches {
        context_parts.push(format!("**{}**\n{}", m.title, m.content));
    }

    // Search file attachments
    let attachment_matches = search_user_attachments(db, model, query, LIMIT, THRESHOLD);
    for m in &attachment_matches {
        context_parts.push(format!("**{}** (attachment)\n{}", m.title, m.content));
    }

    if context_parts.is_empty() {
        return;
    }

    prompt.push_str("## Relevant Product Context\n");
    for part in &context_parts {
        prompt.push_str(&crate::util::wrap_user_data(part));
        prompt.push_str("\n\n");
    }
}
