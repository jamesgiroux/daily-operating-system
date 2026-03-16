//! I576/Phase 5b: Per-dimension prompt builders and merge logic.
//!
//! Instead of one monolithic prompt requesting all ~30 fields, this module
//! produces 6 focused prompts — one per dimension group — each requesting
//! only the fields relevant to that group. This keeps each prompt at ~2-4K
//! chars, improving signal-to-noise and enabling parallel fan-out.
//!
//! Dimension groups:
//! 1. core_assessment      — executiveAssessment, currentState, risks, recentWins
//! 2. stakeholder_champion — stakeholderInsights, championHealth, coverageAssessment,
//!    organizationalChanges, internalTeam (roleChanges mapped to orgChanges)
//! 3. commercial_financial — health, contractContext, renewalOutlook, expansionSignals, blockers
//! 4. strategic_context    — companyContext, competitiveContext, strategicPriorities
//! 5. value_success        — valueDelivered, successMetrics, successPlanSignals,
//!    openCommitments (the `open_commitments` field on IntelligenceJson)
//! 6. engagement_signals   — meetingCadence, emailResponsiveness, productAdoption,
//!    supportHealth, gongCallSummaries, npsCsat

use super::io::IntelligenceJson;
use super::prompts::IntelligenceContext;

/// Canonical dimension group names used by fan-out orchestration.
pub const DIMENSION_NAMES: &[&str] = &[
    "core_assessment",
    "stakeholder_champion",
    "commercial_financial",
    "strategic_context",
    "value_success",
    "engagement_signals",
];

// =============================================================================
// PTY dimension prompt builder
// =============================================================================

/// Build a focused PTY prompt for a single dimension group.
///
/// Each prompt is ~2-4K chars (vs 16K monolithic) and requests only the JSON
/// fields belonging to the specified dimension. Source attribution
/// (`itemSource`) is requested on every item.
pub fn build_dimension_prompt(
    dimension: &str,
    entity_name: &str,
    entity_type: &str,
    relationship: Option<&str>,
    ctx: &IntelligenceContext,
    is_incremental: bool,
) -> String {
    let mut prompt = String::with_capacity(4096);

    // System role — dimension-specific framing
    let role_desc = dimension_role_description(dimension);
    let entity_label = entity_label_for(entity_type, relationship);
    prompt.push_str(&format!(
        "You are {} for the {} \"{}\".\n\n",
        role_desc, entity_label, entity_name
    ));

    // Mode
    if is_incremental {
        prompt.push_str(
            "This is an INCREMENTAL update — prior intelligence exists. \
             Focus on what changed. Do NOT use web search.\n\n",
        );
    } else {
        prompt.push_str(
            "This is an INITIAL intelligence build. Use all available context below. \
             Do NOT use web search.\n\n",
        );
    }

    // Inject only the context relevant to this dimension
    inject_dimension_context(&mut prompt, dimension, ctx);

    // Extra blocks (I555 engagement patterns, champion health, commitments)
    if matches!(
        dimension,
        "stakeholder_champion" | "engagement_signals" | "value_success"
    ) {
        for block in &ctx.extra_blocks {
            prompt.push_str(block);
            prompt.push_str("\n\n");
        }
    }

    // I576: Source-tagged existing intelligence injection
    inject_existing_intelligence(&mut prompt, dimension, ctx);

    // JSON schema for this dimension's fields
    prompt.push_str("## Required Output Format\n\n");
    prompt.push_str(
        "Return ONLY a JSON object — no other text before or after. \
         The JSON must conform exactly to this schema:\n\n",
    );
    prompt.push_str(&dimension_json_schema(dimension, entity_type, ctx));

    // Source attribution instructions
    prompt.push_str(
        "\nFor every array item, include an `\"itemSource\"` object:\n\
         ```json\n\
         \"itemSource\": { \"source\": \"transcript|email|local_file|pty_synthesis\", \
         \"confidence\": 0.7, \"sourcedAt\": \"2026-03-15T00:00:00Z\", \
         \"reference\": \"meeting 2026-03-10\" }\n\
         ```\n\n",
    );

    // I576: Reconciliation rules
    prompt.push_str(RECONCILIATION_RULES_PTY);

    // Format anchor — last thing the model sees
    prompt.push_str("Your response begins with `{` and ends with `}`. Nothing else.\n");

    prompt
}

// =============================================================================
// Glean dimension prompt builder
// =============================================================================

/// Build a focused Glean prompt for a single dimension group.
///
/// Similar to the PTY builder but instructs Glean to search ALL data sources
/// and uses stronger format enforcement (Glean needs more nudging for JSON).
pub fn build_glean_dimension_prompt(
    dimension: &str,
    entity_name: &str,
    entity_type: &str,
    relationship: Option<&str>,
    ctx: &IntelligenceContext,
    is_incremental: bool,
) -> String {
    let mut prompt = String::with_capacity(4096);

    let role_desc = dimension_role_description(dimension);
    let entity_label = entity_label_for(entity_type, relationship);

    // System role — Glean-specific
    prompt.push_str(&format!(
        "You are {} for the {} \"{}\". \
         Search ALL available data sources (Salesforce, Zendesk, Gong, Slack, \
         internal docs, org directory) for this dimension.\n\n",
        role_desc, entity_label, entity_name
    ));

    // Relationship context
    if let Some(rel) = relationship {
        prompt.push_str(&format!("This is a {} relationship.\n\n", rel));
    }

    // Mode
    if is_incremental {
        prompt.push_str(
            "This is an INCREMENTAL update — prior intelligence exists. \
             Focus on what changed, new signals, and updated assessments. \
             Do not repeat unchanged information verbatim.\n\n",
        );
    } else {
        prompt.push_str(
            "This is an INITIAL intelligence build — no prior assessment exists. \
             Be comprehensive.\n\n",
        );
    }

    // Local context (source-tagged for Glean supplementation)
    prompt.push_str(
        "## Local Context (from DailyOS — do not contradict, supplement with org knowledge)\n\n",
    );
    inject_dimension_context(&mut prompt, dimension, ctx);

    // Extra blocks
    if matches!(
        dimension,
        "stakeholder_champion" | "engagement_signals" | "value_success"
    ) {
        for block in &ctx.extra_blocks {
            prompt.push_str(block);
            prompt.push_str("\n\n");
        }
    }

    // I576: Source-tagged existing intelligence injection
    inject_existing_intelligence(&mut prompt, dimension, ctx);

    // Task instruction
    prompt.push_str("## Task\n\n");
    prompt.push_str(&format!(
        "Search ALL available data sources for {} intelligence on this {}. \
         Supplement the local context above with org-level knowledge.\n\n",
        dimension_human_name(dimension),
        if entity_type == "account" {
            "account"
        } else {
            "entity"
        }
    ));

    // JSON schema — stronger format enforcement for Glean
    prompt.push_str("## Required Output Format\n\n");
    prompt.push_str(
        "You MUST respond with a single JSON object. No prose, no markdown fences, \
         no commentary before or after the JSON. Your entire response must be \
         parseable by `JSON.parse()`.\n\n",
    );
    prompt.push_str("The JSON object must have these fields:\n\n");
    prompt.push_str(&dimension_json_schema(dimension, entity_type, ctx));

    // Source attribution for Glean
    prompt.push_str(
        "\nFor every array item, include an `\"itemSource\"` object:\n\
         ```json\n\
         \"itemSource\": { \"source\": \"glean_crm|glean_zendesk|glean_gong|glean_chat|transcript\", \
         \"confidence\": 0.9, \"sourcedAt\": \"2026-03-15T00:00:00Z\", \
         \"reference\": \"Salesforce opportunity\" }\n\
         ```\n\n",
    );

    // I576: Reconciliation rules
    prompt.push_str(RECONCILIATION_RULES_GLEAN);

    prompt.push_str("IMPORTANT:\n");
    prompt.push_str("- Return ONLY valid JSON. No markdown, no commentary before or after.\n");
    prompt.push_str(
        "- Omit any field you don't have data for — do not fabricate.\n",
    );
    prompt.push_str(
        "- Use the local context above as ground truth. Supplement with org knowledge, don't contradict.\n\n",
    );

    // Format anchor
    prompt.push_str("Your response begins with `{` and ends with `}`. Nothing else.\n");

    prompt
}

// =============================================================================
// merge_dimension_into — merges one dimension's output into existing intel
// =============================================================================

/// Merge one dimension's parsed output into an existing IntelligenceJson.
///
/// Only touches the fields belonging to the specified dimension group.
/// For Vec fields, only replaces if the partial has non-empty data.
/// For Option fields, only replaces if the partial has Some.
/// This prevents one dimension's empty defaults from wiping another dimension's data.
pub fn merge_dimension_into(
    existing: &mut IntelligenceJson,
    dimension: &str,
    partial: &IntelligenceJson,
) -> Result<(), String> {
    match dimension {
        "core_assessment" => {
            if partial.executive_assessment.is_some() {
                existing.executive_assessment = partial.executive_assessment.clone();
            }
            if partial.pull_quote.is_some() {
                existing.pull_quote = partial.pull_quote.clone();
            }
            if partial.current_state.is_some() {
                existing.current_state = partial.current_state.clone();
            }
            if !partial.risks.is_empty() {
                existing.risks = partial.risks.clone();
            }
            if !partial.recent_wins.is_empty() {
                existing.recent_wins = partial.recent_wins.clone();
            }
        }

        "stakeholder_champion" => {
            if !partial.stakeholder_insights.is_empty() {
                existing.stakeholder_insights = partial.stakeholder_insights.clone();
            }
            if partial.coverage_assessment.is_some() {
                existing.coverage_assessment = partial.coverage_assessment.clone();
            }
            if !partial.organizational_changes.is_empty() {
                existing.organizational_changes = partial.organizational_changes.clone();
            }
            if !partial.internal_team.is_empty() {
                existing.internal_team = partial.internal_team.clone();
            }
            // relationship_depth is stakeholder-adjacent
            if partial.relationship_depth.is_some() {
                existing.relationship_depth = partial.relationship_depth.clone();
            }
        }

        "commercial_financial" => {
            if partial.health.is_some() {
                existing.health = partial.health.clone();
            }
            if partial.contract_context.is_some() {
                existing.contract_context = partial.contract_context.clone();
            }
            if partial.renewal_outlook.is_some() {
                existing.renewal_outlook = partial.renewal_outlook.clone();
            }
            if !partial.expansion_signals.is_empty() {
                existing.expansion_signals = partial.expansion_signals.clone();
            }
            if !partial.blockers.is_empty() {
                existing.blockers = partial.blockers.clone();
            }
        }

        "strategic_context" => {
            if partial.company_context.is_some() {
                existing.company_context = partial.company_context.clone();
            }
            if !partial.competitive_context.is_empty() {
                existing.competitive_context = partial.competitive_context.clone();
            }
            if !partial.strategic_priorities.is_empty() {
                existing.strategic_priorities = partial.strategic_priorities.clone();
            }
        }

        "value_success" => {
            if !partial.value_delivered.is_empty() {
                existing.value_delivered = partial.value_delivered.clone();
            }
            if partial.success_metrics.is_some() {
                existing.success_metrics = partial.success_metrics.clone();
            }
            if partial.success_plan_signals.is_some() {
                existing.success_plan_signals = partial.success_plan_signals.clone();
            }
            if partial.open_commitments.is_some() {
                existing.open_commitments = partial.open_commitments.clone();
            }
        }

        "engagement_signals" => {
            if partial.meeting_cadence.is_some() {
                existing.meeting_cadence = partial.meeting_cadence.clone();
            }
            if partial.email_responsiveness.is_some() {
                existing.email_responsiveness = partial.email_responsiveness.clone();
            }
            if partial.product_adoption.is_some() {
                existing.product_adoption = partial.product_adoption.clone();
            }
            if partial.support_health.is_some() {
                existing.support_health = partial.support_health.clone();
            }
            if !partial.gong_call_summaries.is_empty() {
                existing.gong_call_summaries = partial.gong_call_summaries.clone();
            }
            if partial.nps_csat.is_some() {
                existing.nps_csat = partial.nps_csat.clone();
            }
        }

        _ => {
            return Err(format!("Unknown dimension group: {}", dimension));
        }
    }

    Ok(())
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Human-readable label for an entity type + relationship.
fn entity_label_for(entity_type: &str, relationship: Option<&str>) -> &'static str {
    match entity_type {
        "account" => match relationship {
            Some("partner") => "partner organization",
            Some("internal") => "internal organization",
            _ => "customer account",
        },
        "project" => "project",
        "person" => match relationship {
            Some("internal") => "internal teammate",
            Some("external") => "external stakeholder",
            _ => "professional contact",
        },
        _ => "entity",
    }
}

/// Role description for the system prompt, per dimension group.
fn dimension_role_description(dimension: &str) -> &'static str {
    match dimension {
        "core_assessment" => {
            "analyzing the overall health and trajectory, producing an executive assessment"
        }
        "stakeholder_champion" => {
            "analyzing stakeholder relationships, champion strength, and organizational coverage"
        }
        "commercial_financial" => {
            "analyzing commercial health, contract status, renewal outlook, and expansion signals"
        }
        "strategic_context" => {
            "analyzing strategic context, competitive landscape, and business priorities"
        }
        "value_success" => {
            "analyzing value delivered, success metrics, success plan signals, and commitments"
        }
        "engagement_signals" => {
            "analyzing engagement patterns: meeting cadence, email responsiveness, product adoption, and support health"
        }
        _ => "building an intelligence assessment",
    }
}

/// Human-readable dimension name for task descriptions.
fn dimension_human_name(dimension: &str) -> &'static str {
    match dimension {
        "core_assessment" => "core assessment",
        "stakeholder_champion" => "stakeholder and champion",
        "commercial_financial" => "commercial and financial",
        "strategic_context" => "strategic context",
        "value_success" => "value and success",
        "engagement_signals" => "engagement signals",
        _ => "general",
    }
}

/// Inject only the context fields relevant to a dimension group.
fn inject_dimension_context(prompt: &mut String, dimension: &str, ctx: &IntelligenceContext) {
    match dimension {
        "core_assessment" => {
            // Needs broad context for overall assessment
            push_section(prompt, "## Current Facts", &ctx.facts_block);
            push_section(prompt, "## Meeting History (last 90 days)", &ctx.meeting_history);
            push_section(prompt, "## Recent Captures (wins/risks/decisions)", &ctx.recent_captures);
            push_opt_section(prompt, "## User Professional Context", &ctx.user_context);
            push_opt_section(prompt, "## User Notes About This Entity", &ctx.entity_context);
        }
        "stakeholder_champion" => {
            push_section(prompt, "## Known Stakeholders", &ctx.stakeholders);
            push_section(prompt, "## Meeting History (last 90 days)", &ctx.meeting_history);
            push_opt_section(
                prompt,
                "## Known Contacts (canonical names)",
                &ctx.canonical_contacts,
            );
            push_opt_section(
                prompt,
                "## Verified Stakeholder Meeting Presence",
                &ctx.verified_stakeholder_presence,
            );
            if !ctx.recent_transcripts.is_empty() {
                prompt.push_str("## Recent Call Transcripts\n");
                prompt.push_str(&ctx.recent_transcripts);
                prompt.push_str("\n\n");
            }
        }
        "commercial_financial" => {
            push_section(prompt, "## Current Facts", &ctx.facts_block);
            push_section(prompt, "## Recent Captures", &ctx.recent_captures);
            push_opt_section(prompt, "## User Notes About This Entity", &ctx.entity_context);
            if let Some(ref computed) = ctx.computed_health {
                prompt.push_str(&format!(
                    "## Pre-Computed Account Health (Algorithmic)\n\
                     Score: {:.0}/100 ({}) | Confidence: {:.0}%\n\n",
                    computed.score,
                    computed.band,
                    computed.confidence * 100.0,
                ));
            }
        }
        "strategic_context" => {
            push_section(prompt, "## Current Facts", &ctx.facts_block);
            push_section(prompt, "## Meeting History (last 90 days)", &ctx.meeting_history);
            push_section(prompt, "## Recent Captures", &ctx.recent_captures);
            if !ctx.file_contents.is_empty() {
                prompt.push_str("## Workspace Files [source: local_file, confidence: 0.85]\n");
                prompt.push_str("Items derived from these files MUST use itemSource.source = \"local_file\" with confidence 0.85.\n");
                prompt.push_str(&ctx.file_contents);
                prompt.push_str("\n\n");
            }
        }
        "value_success" => {
            push_section(prompt, "## Current Facts", &ctx.facts_block);
            push_section(prompt, "## Meeting History (last 90 days)", &ctx.meeting_history);
            push_section(prompt, "## Open Actions", &ctx.open_actions);
            push_section(prompt, "## Recent Captures", &ctx.recent_captures);
        }
        "engagement_signals" => {
            push_section(prompt, "## Meeting History (last 90 days)", &ctx.meeting_history);
            push_section(prompt, "## Recent Email Signals", &ctx.recent_email_signals);
            if !ctx.recent_transcripts.is_empty() {
                prompt.push_str("## Recent Call Transcripts\n");
                prompt.push_str(&ctx.recent_transcripts);
                prompt.push_str("\n\n");
            }
        }
        _ => {}
    }
}

/// Push a section if its content is non-empty.
fn push_section(prompt: &mut String, heading: &str, content: &str) {
    if !content.is_empty() {
        prompt.push_str(heading);
        prompt.push('\n');
        prompt.push_str(content);
        prompt.push_str("\n\n");
    }
}

/// Push a section if the Option is Some and non-empty.
fn push_opt_section(prompt: &mut String, heading: &str, content: &Option<String>) {
    if let Some(ref s) = content {
        if !s.is_empty() {
            prompt.push_str(heading);
            prompt.push('\n');
            prompt.push_str(s);
            prompt.push_str("\n\n");
        }
    }
}

// =============================================================================
// I576: Reconciliation rules injected into prompts
// =============================================================================

const RECONCILIATION_RULES_PTY: &str = "\
RECONCILIATION RULES:\n\
- Items tagged [user_correction] are SACRED — include them verbatim in your output, never modify or drop\n\
- Items tagged [transcript] are personal observations — preserve even if you have no corroborating data\n\
- If your data CONTRADICTS an existing item, include BOTH with \"discrepancy\": true on yours\n\
- Tag every item in your output with \"itemSource\": {\"source\": \"pty_synthesis\", \"confidence\": 0.5, \"sourcedAt\": \"ISO timestamp\"}\n\n";

const RECONCILIATION_RULES_GLEAN: &str = "\
RECONCILIATION RULES:\n\
- Items tagged [user_correction] are SACRED — include them verbatim in your output, never modify or drop\n\
- Items tagged [transcript] are personal observations — preserve even if you have no corroborating data\n\
- If your data CONTRADICTS an existing item, include BOTH with \"discrepancy\": true on yours\n\
- Tag every item with \"itemSource\": {\"source\": \"glean_crm|glean_zendesk|glean_gong|glean_chat\", \"confidence\": 0.7-0.9, \"sourcedAt\": \"ISO timestamp\", \"reference\": \"data source name\"}\n\n";

/// I576: Inject source-tagged existing intelligence items into the prompt.
///
/// When `prior_intelligence` is available, deserializes it and formats each
/// item with source/confidence tags so the LLM knows what to preserve.
fn inject_existing_intelligence(
    prompt: &mut String,
    dimension: &str,
    ctx: &IntelligenceContext,
) {
    use super::io::HasSource;

    let prior_str = match ctx.prior_intelligence {
        Some(ref s) if !s.is_empty() => s,
        _ => return,
    };

    // Try to parse prior intelligence as IntelligenceJson
    let prior: IntelligenceJson = match serde_json::from_str(prior_str) {
        Ok(p) => p,
        Err(_) => {
            // Fall back to raw text injection if not parseable
            let truncated = if prior_str.len() > 2000 {
                &prior_str[..2000]
            } else {
                prior_str
            };
            prompt.push_str("## Prior Intelligence (update, don't replace wholesale)\n");
            prompt.push_str(truncated);
            prompt.push_str("\n\n");
            return;
        }
    };

    let mut items_block = String::new();

    match dimension {
        "core_assessment" => {
            for r in &prior.risks {
                items_block.push_str(&format_tagged_item("Risk", &r.text, r.item_source()));
            }
            for w in &prior.recent_wins {
                items_block.push_str(&format_tagged_item("Win", &w.text, w.item_source()));
            }
        }
        "stakeholder_champion" => {
            for s in &prior.stakeholder_insights {
                let label = format!("Stakeholder: {}", s.name);
                items_block.push_str(&format_tagged_item(
                    &label,
                    s.assessment.as_deref().unwrap_or(""),
                    s.item_source(),
                ));
            }
            for o in &prior.organizational_changes {
                items_block.push_str(&format_tagged_item(
                    "Org Change",
                    &o.person,
                    o.item_source(),
                ));
            }
        }
        "commercial_financial" => {
            for e in &prior.expansion_signals {
                items_block.push_str(&format_tagged_item(
                    "Expansion",
                    &e.opportunity,
                    e.item_source(),
                ));
            }
            if let Some(ref ocs) = prior.open_commitments {
                for c in ocs {
                    items_block.push_str(&format_tagged_item(
                        "Commitment",
                        &c.description,
                        c.item_source(),
                    ));
                }
            }
        }
        "strategic_context" => {
            for c in &prior.competitive_context {
                items_block.push_str(&format_tagged_item(
                    "Competitive",
                    &c.competitor,
                    c.item_source(),
                ));
            }
        }
        "value_success" => {
            for v in &prior.value_delivered {
                items_block.push_str(&format_tagged_item(
                    "Value",
                    &v.statement,
                    v.item_source(),
                ));
            }
        }
        _ => {}
    }

    if !items_block.is_empty() {
        prompt.push_str(
            "## Existing Intelligence (preserve unless you have contradicting evidence)\n",
        );
        prompt.push_str(&items_block);
        prompt.push('\n');
    }
}

/// Format one tagged item line for prompt injection.
fn format_tagged_item(
    label: &str,
    text: &str,
    item_source: Option<&super::io::ItemSource>,
) -> String {
    if text.is_empty() {
        return String::new();
    }
    match item_source {
        Some(src) => {
            let ref_part = src.reference.as_deref().map(|r| format!(", {r}")).unwrap_or_default();
            format!(
                "[{}, {:.1}{ref_part}] {label}: \"{text}\"\n",
                src.source, src.confidence,
            )
        }
        None => {
            // Legacy items without source attribution
            format!("[pty_synthesis, 0.5] {label}: \"{text}\"\n")
        }
    }
}

/// Build the JSON schema snippet for a single dimension group.
fn dimension_json_schema(
    dimension: &str,
    entity_type: &str,
    ctx: &IntelligenceContext,
) -> String {
    let mut s = String::from("```json\n{\n");

    match dimension {
        "core_assessment" => {
            s.push_str(
                r#"  "executiveAssessment": "2-4 paragraphs. P1: one-sentence verdict. P2: top risk. P3: biggest opportunity. P4 (optional): key unknowns. Max 250 words.",
  "risks": [{"text": "specific risk with named people/timelines", "urgency": "critical|watch|low", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "recentWins": [{"text": "verifiable win", "impact": "high|medium|low", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "pullQuote": "One impactful sentence — the single most important thing about this account right now. Written as an editorial pull quote, not a summary. Max 30 words.",
  "currentState": {
    "working": ["specific positive with evidence"],
    "notWorking": ["specific concern with evidence"],
    "unknowns": ["what we don't know but should"]
  }
"#,
            );
        }
        "stakeholder_champion" => {
            s.push_str(
                r#"  "stakeholderInsights": [{"name": "full name", "role": "job title", "assessment": "1-2 sentences about engagement", "engagement": "high|medium|low|unknown", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "coverageAssessment": {"roleFillRate": 0.0, "gaps": ["missing role"], "covered": ["filled role"], "level": "strong|adequate|thin|critical"},
  "organizationalChanges": [{"changeType": "departure|hire|promotion|reorg|role_change", "person": "name", "from": "...", "to": "...", "detectedAt": "ISO date", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "internalTeam": [{"name": "...", "role": "RM|AE|TAM|Division Lead|etc", "source": "glean|user|crm"}],
  "relationshipDepth": {"championStrength": "strong|moderate|weak|none", "executiveAccess": "direct|indirect|none", "stakeholderCoverage": "broad|narrow|single_threaded", "coverageGaps": ["role or team with no relationship"]}
"#,
            );
        }
        "commercial_financial" => {
            // Health schema depends on whether pre-computed health is available
            if ctx.computed_health.is_some() {
                s.push_str(
                    r#"  "health": {
    "narrative": "2-3 sentences explaining the pre-computed health score in business context",
    "recommendedActions": ["3 specific actions to improve or maintain account health"]
  },
"#,
                );
            } else {
                s.push_str(
                    r#"  "health": {
    "score": "0-100", "band": "green|yellow|red", "source": "computed",
    "confidence": "0.0-1.0",
    "trend": {"direction": "improving|stable|declining|volatile", "rationale": "1 sentence", "timeframe": "30d|90d", "confidence": "0.0-1.0"},
    "recommendedActions": ["specific next action"]
  },
"#,
                );
            }
            s.push_str(
                r#"  "contractContext": {"contractType": "annual|multi_year|month_to_month", "autoRenew": true, "renewalDate": "ISO date", "currentArr": 0.0},
  "renewalOutlook": {"confidence": "high|moderate|low", "riskFactors": ["..."], "expansionPotential": "...", "recommendedStart": "ISO date"},
  "expansionSignals": [{"opportunity": "...", "arrImpact": 0.0, "stage": "exploring|evaluating|committed|blocked", "strength": "strong|moderate|early", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "blockers": [{"description": "...", "owner": "...", "since": "ISO date", "impact": "critical|high|moderate|low"}]
"#,
            );
        }
        "strategic_context" => {
            s.push_str(
                r#"  "companyContext": {"description": "1-2 sentences about the company", "industry": "industry", "size": "employee count or band", "headquarters": "location"},"#,
            );

            if entity_type == "account" {
                s.push_str(
                    r#"
  "competitiveContext": [{"competitor": "name", "threatLevel": "displacement|evaluation|mentioned|incumbent", "context": "1 sentence", "detectedAt": "ISO date or null", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "strategicPriorities": [{"priority": "...", "status": "active|at_risk|completed|paused", "owner": "...", "timeline": "..."}]
"#,
                );
            } else {
                s.push('\n');
            }
        }
        "value_success" => {
            s.push_str(
                r#"  "valueDelivered": [{"date": "ISO date", "statement": "quantified outcome — must include a number", "source": "meeting|email|capture", "impact": "revenue|cost|risk|speed", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}],
  "successMetrics": [{"name": "short KPI label (max 5 words)", "target": "short target (e.g. 95%, $500K, 8+)", "current": "short current value — max 15 chars, number/grade only, NEVER a sentence", "status": "on_track|at_risk|behind|achieved", "owner": "who owns this metric"}],
  "successPlanSignals": {
    "statedObjectives": [{"objective": "...", "source": "meeting|email|file", "owner": "...", "targetDate": "ISO or null", "confidence": "high|medium|low"}],
    "mutualSuccessCriteria": [{"criterion": "...", "ownedBy": "us|them|joint", "status": "not_started|in_progress|achieved|at_risk"}],
    "milestoneCandidates": [{"milestone": "...", "expectedBy": "ISO or null", "detectedFrom": "source", "autoDetectEvent": "lifecycle event type or null"}]
  },
  "openCommitments": [{"description": "what was committed", "owner": "who owns it", "dueDate": "ISO date or null", "source": "meeting/email where committed", "status": "open|in_progress|overdue|completed", "itemSource": {"source": "...", "confidence": 0.7, "sourcedAt": "...", "reference": "..."}}]
"#,
            );
        }
        "engagement_signals" => {
            s.push_str(
                r#"  "meetingCadence": {"meetingsPerMonth": 0.0, "trend": "increasing|stable|declining|erratic", "daysSinceLast": 0, "assessment": "healthy|adequate|sparse|cold", "evidence": ["signal"]},
  "emailResponsiveness": {"trend": "improving|stable|slowing|gone_quiet", "volumeTrend": "increasing|stable|decreasing", "assessment": "responsive|normal|slow|unresponsive", "evidence": ["signal"]},"#,
            );

            if entity_type == "account" {
                s.push_str(
                    r#"
  "productAdoption": {"adoptionRate": 0.0, "trend": "growing|stable|declining", "featureAdoption": ["..."], "lastActive": "ISO date"},
  "supportHealth": {"openTickets": 0, "criticalTickets": 0, "trend": "improving|stable|degrading", "csat": 0.0},
  "gongCallSummaries": [{"title": "call title", "date": "ISO date", "participants": ["name"], "keyTopics": "summary", "sentiment": "positive|neutral|negative"}],
  "npsCsat": {"nps": 0, "csat": 0.0, "surveyDate": "ISO date", "verbatim": "quote"}
"#,
                );
            } else {
                s.push('\n');
            }
        }
        _ => {
            s.push_str("  // Unknown dimension\n");
        }
    }

    s.push_str("}\n```\n");
    s
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_intel() -> IntelligenceJson {
        IntelligenceJson::default()
    }

    fn make_ctx() -> IntelligenceContext {
        IntelligenceContext {
            facts_block: "ARR: $120K".to_string(),
            meeting_history: "2026-03-10: QBR".to_string(),
            stakeholders: "Jane Doe — VP Engineering".to_string(),
            recent_email_signals: "3 emails last week".to_string(),
            recent_captures: "Win: reduced churn 20%".to_string(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // merge_dimension_into tests
    // -----------------------------------------------------------------------

    #[test]
    fn merge_core_assessment_only_touches_core_fields() {
        let mut existing = empty_intel();
        existing.stakeholder_insights = vec![super::super::io::StakeholderInsight {
            name: "Alice".to_string(),
            role: Some("VP".to_string()),
            assessment: None,
            engagement: None,
            source: None,
            person_id: None,
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }];

        let mut partial = empty_intel();
        partial.executive_assessment = Some("Looking good".to_string());
        partial.risks = vec![super::super::io::IntelRisk {
            text: "Budget cut risk".to_string(),
            source: None,
            urgency: "watch".to_string(),
            item_source: None,
            discrepancy: None,
        }];
        // Partial has empty stakeholder_insights — should NOT wipe existing
        assert!(partial.stakeholder_insights.is_empty());

        merge_dimension_into(&mut existing, "core_assessment", &partial).unwrap();

        // Core fields updated
        assert_eq!(
            existing.executive_assessment,
            Some("Looking good".to_string())
        );
        assert_eq!(existing.risks.len(), 1);

        // Stakeholder field untouched
        assert_eq!(existing.stakeholder_insights.len(), 1);
        assert_eq!(existing.stakeholder_insights[0].name, "Alice");
    }

    #[test]
    fn merge_stakeholder_champion_only_touches_stakeholder_fields() {
        let mut existing = empty_intel();
        existing.executive_assessment = Some("Existing assessment".to_string());
        existing.contract_context = Some(super::super::io::ContractContext {
            contract_type: Some("annual".to_string()),
            ..Default::default()
        });

        let mut partial = empty_intel();
        partial.stakeholder_insights = vec![super::super::io::StakeholderInsight {
            name: "Bob".to_string(),
            role: Some("CTO".to_string()),
            assessment: Some("Engaged".to_string()),
            engagement: Some("high".to_string()),
            source: None,
            person_id: None,
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }];

        merge_dimension_into(&mut existing, "stakeholder_champion", &partial).unwrap();

        // Stakeholder updated
        assert_eq!(existing.stakeholder_insights.len(), 1);
        assert_eq!(existing.stakeholder_insights[0].name, "Bob");

        // Other fields untouched
        assert_eq!(
            existing.executive_assessment,
            Some("Existing assessment".to_string())
        );
        assert!(existing.contract_context.is_some());
    }

    #[test]
    fn merge_commercial_financial_only_touches_commercial_fields() {
        let mut existing = empty_intel();
        existing.executive_assessment = Some("Keep me".to_string());
        existing.strategic_priorities = vec![super::super::io::StrategicPriority {
            priority: "Grow ARR".to_string(),
            status: None,
            owner: None,
            source: None,
            timeline: None,
        }];

        let mut partial = empty_intel();
        partial.contract_context = Some(super::super::io::ContractContext {
            contract_type: Some("multi_year".to_string()),
            ..Default::default()
        });
        partial.blockers = vec![super::super::io::Blocker {
            description: "Legal review".to_string(),
            owner: Some("Legal team".to_string()),
            since: None,
            impact: Some("high".to_string()),
            source: None,
        }];

        merge_dimension_into(&mut existing, "commercial_financial", &partial).unwrap();

        // Commercial fields updated
        assert!(existing.contract_context.is_some());
        assert_eq!(existing.blockers.len(), 1);

        // Other fields untouched
        assert_eq!(
            existing.executive_assessment,
            Some("Keep me".to_string())
        );
        assert_eq!(existing.strategic_priorities.len(), 1);
    }

    #[test]
    fn merge_strategic_context_only_touches_strategic_fields() {
        let mut existing = empty_intel();
        existing.risks = vec![super::super::io::IntelRisk {
            text: "Keep me".to_string(),
            source: None,
            urgency: "watch".to_string(),
            item_source: None,
            discrepancy: None,
        }];

        let mut partial = empty_intel();
        partial.company_context = Some(super::super::io::CompanyContext {
            description: Some("SaaS company".to_string()),
            industry: Some("Technology".to_string()),
            size: None,
            headquarters: None,
            additional_context: None,
        });
        partial.competitive_context = vec![super::super::io::CompetitiveInsight {
            competitor: "Rival Inc".to_string(),
            threat_level: Some("evaluation".to_string()),
            context: None,
            source: None,
            detected_at: None,
            item_source: None,
            discrepancy: None,
        }];

        merge_dimension_into(&mut existing, "strategic_context", &partial).unwrap();

        assert!(existing.company_context.is_some());
        assert_eq!(existing.competitive_context.len(), 1);
        // Risks untouched
        assert_eq!(existing.risks.len(), 1);
    }

    #[test]
    fn merge_value_success_only_touches_value_fields() {
        let mut existing = empty_intel();
        existing.meeting_cadence = Some(super::super::io::CadenceAssessment {
            meetings_per_month: Some(4.0),
            ..Default::default()
        });

        let mut partial = empty_intel();
        partial.value_delivered = vec![super::super::io::ValueItem {
            date: Some("2026-03-01".to_string()),
            statement: "Saved $50K".to_string(),
            source: None,
            impact: Some("cost".to_string()),
            item_source: None,
            discrepancy: None,
        }];

        merge_dimension_into(&mut existing, "value_success", &partial).unwrap();

        assert_eq!(existing.value_delivered.len(), 1);
        // Engagement field untouched
        assert!(existing.meeting_cadence.is_some());
    }

    #[test]
    fn merge_engagement_signals_only_touches_engagement_fields() {
        let mut existing = empty_intel();
        existing.executive_assessment = Some("Keep me".to_string());
        existing.value_delivered = vec![super::super::io::ValueItem {
            date: None,
            statement: "Keep me".to_string(),
            source: None,
            impact: None,
            item_source: None,
            discrepancy: None,
        }];

        let mut partial = empty_intel();
        partial.meeting_cadence = Some(super::super::io::CadenceAssessment {
            meetings_per_month: Some(2.0),
            trend: Some("declining".to_string()),
            ..Default::default()
        });
        partial.email_responsiveness = Some(super::super::io::ResponsivenessAssessment {
            trend: Some("slowing".to_string()),
            ..Default::default()
        });

        merge_dimension_into(&mut existing, "engagement_signals", &partial).unwrap();

        assert!(existing.meeting_cadence.is_some());
        assert!(existing.email_responsiveness.is_some());
        // Other fields untouched
        assert_eq!(
            existing.executive_assessment,
            Some("Keep me".to_string())
        );
        assert_eq!(existing.value_delivered.len(), 1);
    }

    #[test]
    fn merge_empty_partial_does_not_wipe_existing() {
        let mut existing = empty_intel();
        existing.executive_assessment = Some("Existing".to_string());
        existing.risks = vec![super::super::io::IntelRisk {
            text: "Existing risk".to_string(),
            source: None,
            urgency: "critical".to_string(),
            item_source: None,
            discrepancy: None,
        }];

        let partial = empty_intel(); // All defaults — empty vecs, None options

        merge_dimension_into(&mut existing, "core_assessment", &partial).unwrap();

        // Should NOT wipe because partial is empty
        assert_eq!(
            existing.executive_assessment,
            Some("Existing".to_string())
        );
        assert_eq!(existing.risks.len(), 1);
    }

    #[test]
    fn merge_unknown_dimension_returns_error() {
        let mut existing = empty_intel();
        let partial = empty_intel();
        let result = merge_dimension_into(&mut existing, "nonexistent", &partial);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown dimension group"));
    }

    // -----------------------------------------------------------------------
    // Prompt builder tests
    // -----------------------------------------------------------------------

    #[test]
    fn pty_prompt_contains_dimension_schema() {
        let ctx = make_ctx();
        for dim in DIMENSION_NAMES {
            let prompt = build_dimension_prompt(dim, "Acme Corp", "account", None, &ctx, false);
            assert!(
                prompt.contains("```json"),
                "Dimension {} missing JSON schema",
                dim
            );
            assert!(
                prompt.contains("Your response begins with `{`"),
                "Dimension {} missing format anchor",
                dim
            );
        }
    }

    #[test]
    fn glean_prompt_contains_search_instruction() {
        let ctx = make_ctx();
        for dim in DIMENSION_NAMES {
            let prompt =
                build_glean_dimension_prompt(dim, "Acme Corp", "account", None, &ctx, false);
            assert!(
                prompt.contains("Search ALL available data sources"),
                "Dimension {} missing Glean search instruction",
                dim
            );
            assert!(
                prompt.contains("JSON.parse()"),
                "Dimension {} missing JSON enforcement",
                dim
            );
        }
    }

    #[test]
    fn core_assessment_prompt_includes_facts_not_stakeholders() {
        let ctx = make_ctx();
        let prompt =
            build_dimension_prompt("core_assessment", "Acme Corp", "account", None, &ctx, false);
        assert!(prompt.contains("ARR: $120K"), "Missing facts_block");
        assert!(
            !prompt.contains("Jane Doe — VP Engineering"),
            "Should not include stakeholders in core_assessment"
        );
    }

    #[test]
    fn stakeholder_prompt_includes_stakeholders_not_email() {
        let ctx = make_ctx();
        let prompt = build_dimension_prompt(
            "stakeholder_champion",
            "Acme Corp",
            "account",
            None,
            &ctx,
            false,
        );
        assert!(
            prompt.contains("Jane Doe — VP Engineering"),
            "Missing stakeholders"
        );
        assert!(
            !prompt.contains("3 emails last week"),
            "Should not include email signals in stakeholder_champion"
        );
    }

    #[test]
    fn engagement_prompt_includes_email_signals() {
        let ctx = make_ctx();
        let prompt = build_dimension_prompt(
            "engagement_signals",
            "Acme Corp",
            "account",
            None,
            &ctx,
            false,
        );
        assert!(
            prompt.contains("3 emails last week"),
            "Missing email signals"
        );
        assert!(
            prompt.contains("meetingCadence"),
            "Missing meeting cadence schema"
        );
    }

    #[test]
    fn dimension_names_constant_has_six_entries() {
        assert_eq!(DIMENSION_NAMES.len(), 6);
    }
}
