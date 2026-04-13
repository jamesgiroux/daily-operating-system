//! I535: Structured prompt builders for Glean MCP `chat` tool.
//!
//! These prompts request the full I508 + I554 enriched schema as JSON.
//! The output must be parseable by `parse_intelligence_response()` — the
//! same parser used for PTY output. Both paths produce identical types.

use super::prompts::IntelligenceContext;

/// Build the Glean enrichment prompt for an entity.
///
/// Requests the full IntelligenceJson schema as JSON output.
/// Injects local context so Glean supplements rather than duplicates.
pub fn build_glean_enrichment_prompt(
    entity_name: &str,
    entity_type: &str,
    relationship: Option<&str>,
    ctx: &IntelligenceContext,
    is_incremental: bool,
) -> String {
    let mut prompt = String::with_capacity(8192);

    // System role
    prompt.push_str("You are a customer success intelligence system. Analyze the ");
    prompt.push_str(match entity_type {
        "account" => "customer account",
        "project" => "project",
        "person" => "contact",
        _ => "entity",
    });
    prompt.push_str(&format!(" \"{}\"", entity_name));
    prompt.push_str(
        " using ALL available data sources (Salesforce, Zendesk, Gong, Slack, internal docs, org directory).\n\n",
    );

    // Relationship context
    if let Some(rel) = relationship {
        prompt.push_str(&format!("This is a {} relationship.\n\n", rel));
    }

    // Mode
    if is_incremental {
        prompt.push_str("This is an INCREMENTAL update — prior intelligence exists. Focus on what changed, new signals, and updated assessments. Do not repeat unchanged information verbatim.\n\n");
    } else {
        prompt.push_str("This is an INITIAL intelligence build — no prior assessment exists. Be comprehensive.\n\n");
    }

    // Local context block
    if !ctx.facts_block.is_empty()
        || !ctx.meeting_history.is_empty()
        || !ctx.open_actions.is_empty()
        || !ctx.recent_captures.is_empty()
    {
        prompt.push_str("## Local Context (from DailyOS — do not contradict, supplement with org knowledge)\n\n");

        if !ctx.facts_block.is_empty() {
            prompt.push_str("### Current Facts\n");
            prompt.push_str(&ctx.facts_block);
            prompt.push_str("\n\n");
        }

        if !ctx.meeting_history.is_empty() {
            prompt.push_str("### Meeting History (last 90 days)\n");
            prompt.push_str(&ctx.meeting_history);
            prompt.push_str("\n\n");
        }

        if !ctx.open_actions.is_empty() {
            prompt.push_str("### Open Actions\n");
            prompt.push_str(&ctx.open_actions);
            prompt.push_str("\n\n");
        }

        if !ctx.recent_captures.is_empty() {
            prompt.push_str("### Recent Captures (wins/risks/decisions)\n");
            prompt.push_str(&ctx.recent_captures);
            prompt.push_str("\n\n");
        }

        if !ctx.recent_email_signals.is_empty() {
            prompt.push_str("### Email Signals\n");
            prompt.push_str(&ctx.recent_email_signals);
            prompt.push_str("\n\n");
        }

        if !ctx.stakeholders.is_empty() {
            prompt.push_str("### Known Stakeholders\n");
            prompt.push_str(&ctx.stakeholders);
            prompt.push_str("\n\n");
        }

        if let Some(ref user_ctx) = ctx.user_context {
            if !user_ctx.is_empty() {
                prompt.push_str("### User Professional Context\n");
                prompt.push_str(user_ctx);
                prompt.push_str("\n\n");
            }
        }

        if let Some(ref entity_ctx) = ctx.entity_context {
            if !entity_ctx.is_empty() {
                prompt.push_str("### User Notes About This Entity\n");
                prompt.push_str(entity_ctx);
                prompt.push_str("\n\n");
            }
        }

        // I555 extra blocks (engagement patterns, champion health, commitments)
        for block in &ctx.extra_blocks {
            prompt.push_str(block);
            prompt.push_str("\n\n");
        }

        if let Some(ref prior) = ctx.prior_intelligence {
            if !prior.is_empty() {
                prompt.push_str("### Prior Intelligence Assessment\n");
                // Truncate to 3000 chars to leave room for Glean's own synthesis
                let truncated = if prior.len() > 3000 {
                    &prior[..3000]
                } else {
                    prior
                };
                prompt.push_str(truncated);
                prompt.push_str("\n\n");
            }
        }
    }

    // Instructions — format enforcement at the end for recency bias
    prompt.push_str("## Task\n\n");
    prompt.push_str("Search ALL available data sources in the organization for this ");
    prompt.push_str(if entity_type == "account" {
        "account"
    } else {
        "entity"
    });
    prompt.push_str(". Supplement the local context above with org-level knowledge.\n");
    prompt.push_str("When recent local meeting, transcript, attendee, or user-edited context conflicts with older enterprise documents, prefer the recent local context.\n\n");

    prompt.push_str("## Required Output Format\n\n");
    prompt.push_str("You MUST respond with a single JSON object. No prose, no markdown fences, no commentary before or after the JSON. Your entire response must be parseable by `JSON.parse()`.\n\n");
    prompt.push_str("The JSON object must have these fields:\n\n");
    prompt.push_str(&build_json_schema(entity_type));

    // Final anchor — the last thing the model sees
    prompt.push_str("\nYour response begins with `{` and ends with `}`. Nothing else.\n");

    prompt
}

/// Build the JSON schema instructions for the Glean prompt.
///
/// Mirrors the full IntelligenceJson struct with I508 base + I554 enrichments.
fn build_json_schema(entity_type: &str) -> String {
    let mut schema = String::from("```json\n{\n");

    // Core fields (all entity types)
    schema.push_str(r#"  "executiveAssessment": "2-4 paragraph narrative: current health verdict, top risk with evidence, biggest opportunity, key unknowns",
  "pullQuote": "One impactful sentence — the single most important thing about this account right now. Written as an editorial pull quote, not a summary. Max 30 words.",
  "risks": [{ "text": "specific risk with named people/timelines", "urgency": "critical|watch|low", "source": "where this was found" }],
  "recentWins": [{ "text": "specific verifiable win", "impact": "high|medium|low", "subType": "adoption|expansion|value_realized|relationship|commercial|advocacy" }],
  "currentState": {
    "working": ["specific positive with evidence"],
    "notWorking": ["specific concern with evidence"],
    "unknowns": ["what we don't know but should"]
  },
  "stakeholderInsights": [{ "name": "full name", "role": "job title", "assessment": "1 sentence about engagement", "engagement": "high|medium|low" }],
  "valueDelivered": [{ "date": "YYYY-MM-DD", "statement": "quantified measurable outcome — must include a number", "source": "where this was found", "impact": "revenue|cost|risk|speed" }],
  "nextMeetingReadiness": { "prepItems": ["max 3 items to prepare"] },
"#);

    // I554 enrichments
    schema.push_str(r#"  "championHealth": { "name": "champion name or null", "status": "strong|weak|lost|none", "evidence": "behavioral evidence", "risk": "if weak/lost, the risk and recommended action" },
  "commitments": [{ "content": "what was committed", "ownedBy": "us|them|joint", "successCriteria": "how we know it's done", "targetDate": "YYYY-MM-DD or null" }],
  "successPlanSignals": {
    "statedObjectives": [{ "objective": "strategic goal", "source": "where stated", "owner": "who owns it", "targetDate": "YYYY-MM-DD or null", "confidence": "high|medium|low" }],
    "mutualSuccessCriteria": [{ "criterion": "how success is measured", "ownedBy": "us|them|joint", "status": "not_started|in_progress|achieved|at_risk" }],
    "milestoneCandidates": [{ "milestone": "checkpoint", "expectedBy": "YYYY-MM-DD or null", "detectedFrom": "source", "autoDetectEvent": "lifecycle event type or null" }]
  },
  "roleChanges": [{ "personName": "full name", "oldStatus": "previous role/status", "newStatus": "new role/status", "evidence": "source" }],
"#);

    // Account-specific fields
    if entity_type == "account" {
        schema.push_str(r#"  "companyContext": { "description": "1-2 sentences about the company", "industry": "industry", "size": "employee count or band", "headquarters": "location" },
  "health": { "score": 65, "band": "green|yellow|red", "confidence": "high|medium|low", "narrative": "1-2 sentence health explanation", "recommendedActions": ["actionable next step"] },
  "competitiveContext": [{ "competitor": "name", "context": "what we know", "threat": "high|medium|low" }],
  "strategicPriorities": [{ "priority": "what matters to them", "alignment": "how we align" }],
  "coverageAssessment": { "summary": "stakeholder coverage quality narrative", "gaps": ["missing roles or relationships"] },
  "organizationalChanges": [{ "change": "what changed", "when": "approximate date", "impact": "how it affects us" }],
  "blockers": [{ "blocker": "what's blocking", "severity": "critical|moderate|low" }],
  "contractContext": { "summary": "contract details if known" },
  "expansionSignals": [{ "signal": "opportunity description", "strength": "strong|moderate|early", "source": "where found" }],
  "renewalOutlook": { "assessment": "narrative about renewal likelihood", "confidence": "high|medium|low" },
"#);

        // Glean-specific fields (data only Glean can produce)
        schema.push_str(r#"  "supportHealth": { "openTickets": 0, "recentTrend": "improving|stable|declining", "criticalIssues": ["issue description"], "summary": "1-2 sentence support quality" },
  "salesforceContext": { "renewalProbability": null, "dealStage": null, "forecastCloseDate": null, "pipelineValue": null },
  "gongCallSummaries": [{ "title": "call title", "date": "YYYY-MM-DD", "participants": ["name"], "keyTopics": "summary", "sentiment": "positive|neutral|negative" }],
  "orgChartChanges": [{ "person": "name", "change": "what changed", "when": "date", "impact": "how it affects the relationship" }],
"#);

        schema.push_str(r#"  "productAdoption": { "adoptionRate": 0.0, "trend": "growing|stable|declining", "featureAdoption": ["product or feature name: usage%"], "lastActive": "YYYY-MM-DD or null", "source": "glean" },
  "successMetrics": [{ "name": "short KPI label (max 5 words)", "target": "short target (e.g. 95%, $500K)", "current": "short value — max 15 chars, number/grade only, NEVER a sentence", "status": "on-track|at-risk|behind|achieved" }],
  "openCommitments": [{ "description": "what was promised", "owner": "us|them|joint", "dueDate": "YYYY-MM-DD or null", "status": "open|delivered|at-risk" }],
  "relationshipDepth": { "championStrength": "strong|adequate|weak|none", "executiveAccess": "yes|limited|none", "stakeholderCoverage": "narrative", "coverageGaps": ["gaps"] },
  "recommendedActions": [{ "title": "verb-phrase action", "rationale": "why — reference specific people/signals", "priority": 2, "suggestedDue": "YYYY-MM-DD or null" }]
"#);
    } else if entity_type == "person" {
        schema.push_str(r#"  "network": { "health": "narrative", "keyRelationships": [{ "name": "person", "type": "peer|manager|collaborator" }], "influenceRadius": "narrative" }
"#);
    }

    schema.push_str("}\n```\n\n");

    // Quality guidance
    schema.push_str("IMPORTANT:\n");
    schema.push_str("- Return ONLY valid JSON. No markdown, no commentary before or after.\n");
    schema.push_str("- For risks: RED urgency = champion departure, competitor eval, budget cut. YELLOW = usage decline, reorg. Use \"critical\"/\"watch\"/\"low\".\n");
    schema.push_str("- For wins: only extract verifiable outcomes, not vague sentiment. \"Customer seems happy\" is NOT a win.\n");
    schema.push_str("- For valueDelivered: must include a number (dollars, percentages, time saved). Reject vague usage statements.\n");
    schema.push_str("- For championHealth: strong = power + vested interest + actively advocates internally. weak = helpful but lacks influence. lost = departed or disengaged.\n");
    schema.push_str("- recommendedActions: 2-3 specific, concrete actions. Reference specific people, meetings, or signals. Priority: 1=urgent, 2=high, 3=medium, 4=low. Leave empty for sparse accounts.\n");
    schema.push_str("- Omit any field you don't have data for — do not fabricate.\n");
    schema.push_str("- Use the local context above as ground truth. Supplement with org knowledge, don't contradict.\n");
    schema.push_str("- Prefer evidence from the last 12 months. Do not treat older snippets as the current state when newer local meeting evidence exists.\n");

    schema
}

/// Build an ephemeral account query prompt for a named account.
///
/// Used by I495 to produce a one-shot briefing about an account that may not
/// be in the local database yet. Returns prose-friendly structured JSON.
pub fn build_ephemeral_query_prompt(name: &str) -> String {
    format!(
        r#"Tell me everything you know about the company or account "{name}" from ALL available data sources (Salesforce, Zendesk, Gong, Slack, internal documents, org directory).

Return ONLY a JSON object (no markdown, no commentary):
{{
  "summary": "2-3 paragraph comprehensive overview of this account — who they are, what they do, and our relationship with them",
  "sections": [
    {{
      "title": "section title (e.g. Relationship Overview, Support History, Recent Activity, Key Contacts, Product Usage)",
      "content": "detailed content for this section",
      "source": "primary data source for this section (salesforce, zendesk, gong, slack, docs, or null)"
    }}
  ],
  "sourceCount": 3
}}

Instructions:
- Include as many sections as you have data for — group by topic, not by source
- The summary should be readable as a standalone briefing
- For each section, cite which data source the information primarily came from
- sourceCount = how many distinct data sources contributed information
- If you have no data about this account, return {{"summary": "No information found for {name} across available data sources.", "sections": [], "sourceCount": 0}}
- Omit sections where you have no real data — do not fabricate
- Return ONLY valid JSON. No markdown fences, no commentary before or after."#,
    )
}

/// Build the account discovery prompt for a user email.
pub fn build_account_discovery_prompt(user_email: &str, user_name: &str) -> String {
    format!(
        r#"I am {} ({}). Find all customer accounts I own, manage, or am actively involved with. Search Salesforce account ownership, Gong call participation, Zendesk ticket assignments, and internal documents.

Return ONLY a JSON object (no markdown, no commentary):
{{
  "accounts": [{{
    "name": "account name",
    "myRole": "owner|tam|csm|involved",
    "evidence": "how you know I'm involved (specific source)",
    "source": "salesforce|gong|zendesk|slack|docs",
    "domain": "company domain if known",
    "industry": "industry if known",
    "contextPreview": "1-2 sentence summary of the account"
  }}]
}}"#,
        user_name, user_email
    )
}
