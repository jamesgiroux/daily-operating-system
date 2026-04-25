//! DOS-15: Glean leading-signal enrichment for the Health & Outlook tab.
//!
//! Supplemental enrichment pass that runs after the main per-dimension Glean
//! enrichment. Extracts high-leverage leading signals (champion risk, usage
//! trends, sentiment divergence, transcript extraction, commercial signals,
//! advocacy track, quote wall) that the base enrichment does not surface.
//!
//! Canonical prompt: `.docs/mockups/glean-prompt-health-outlook-signals.md`.
//!
//! Silent fallback: when Glean is not configured, this module is skipped
//! entirely from `intel_queue.rs` — no errors surfaced to the user.

use serde::{Deserialize, Serialize};

use super::glean_provider::extract_json_object;

/// Root struct persisted to `entity_assessment.health_outlook_signals_json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthOutlookSignals {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub champion_risk: Option<ChampionRisk>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub product_usage_trend: Option<ProductUsageTrend>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub channel_sentiment: Option<ChannelSentiment>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transcript_extraction: Option<TranscriptExtraction>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub commercial_signals: Option<CommercialSignals>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub advocacy_track: Option<AdvocacyTrack>,
    #[serde(default)]
    pub quote_wall: Vec<QuoteWallEntry>,
    /// Trend signals from a separate PTY pass over usage telemetry and
    /// per-message sentiment. Populated by chapter-by-chapter enrichment
    /// (DOS-204); left as `None` when that pass has not produced output.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub trends: Option<TrendSignals>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionRisk {
    #[serde(default)]
    pub champion_name: Option<String>,
    #[serde(default)]
    pub at_risk: bool,
    #[serde(default)]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub risk_evidence: Vec<String>,
    #[serde(default)]
    pub tenure_signal: Option<String>,
    #[serde(default)]
    pub recent_role_change: Option<String>,
    #[serde(default)]
    pub email_sentiment_trend_30d: Option<String>,
    #[serde(default)]
    pub email_response_time_trend: Option<String>,
    #[serde(default)]
    pub backup_champion_candidates: Vec<BackupChampion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupChampion {
    pub name: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub engagement_level: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUsageTrend {
    #[serde(default)]
    pub overall_trend_30d: Option<String>,
    #[serde(default)]
    pub overall_trend_90d: Option<String>,
    #[serde(default)]
    pub features: Vec<FeatureUsage>,
    #[serde(default)]
    pub underutilized_features: Vec<UnderutilizedFeature>,
    #[serde(default)]
    pub highly_sticky_features: Vec<StickyFeature>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureUsage {
    pub name: String,
    #[serde(default)]
    pub adoption_status: Option<String>,
    #[serde(default)]
    pub active_users_estimate: Option<serde_json::Value>,
    #[serde(default)]
    pub usage_trend_30d: Option<String>,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnderutilizedFeature {
    pub name: String,
    #[serde(default)]
    pub licensed_but_unused_days: Option<i64>,
    #[serde(default)]
    pub coaching_opportunity: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickyFeature {
    pub name: String,
    #[serde(default)]
    pub why_sticky: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSentiment {
    #[serde(default)]
    pub email: Option<ChannelReading>,
    #[serde(default)]
    pub meetings: Option<ChannelReading>,
    #[serde(default)]
    pub support_tickets: Option<ChannelReading>,
    #[serde(default)]
    pub slack: Option<ChannelReading>,
    #[serde(default)]
    pub divergence_detected: bool,
    #[serde(default)]
    pub divergence_summary: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelReading {
    #[serde(default)]
    pub sentiment: Option<String>,
    #[serde(default)]
    pub trend_30d: Option<String>,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptExtraction {
    #[serde(default)]
    pub churn_adjacent_questions: Vec<TranscriptQuestion>,
    #[serde(default)]
    pub expansion_adjacent_questions: Vec<TranscriptQuestion>,
    #[serde(default)]
    pub competitor_benchmarks: Vec<CompetitorBenchmark>,
    #[serde(default)]
    pub decision_maker_shifts: Vec<DecisionMakerShift>,
    #[serde(default)]
    pub budget_cycle_signals: Vec<BudgetCycleSignal>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptQuestion {
    pub question: String,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub risk_signal: Option<String>,
    #[serde(default)]
    pub opportunity_signal: Option<String>,
    #[serde(default)]
    pub estimated_arr_upside: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitorBenchmark {
    pub competitor: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub threat_level: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionMakerShift {
    pub shift: String,
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub implication: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetCycleSignal {
    pub signal: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub implication: Option<String>,
    /// Normalized flag — true when the signal describes a locked/frozen budget window.
    /// Drives the `budget_cycle_locked` signal emission.
    #[serde(default)]
    pub locked: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialSignals {
    #[serde(default)]
    pub arr_trend_12mo: Vec<ArrTrendPoint>,
    #[serde(default)]
    pub arr_direction: Option<String>,
    #[serde(default)]
    pub payment_behavior: Option<String>,
    #[serde(default)]
    pub payment_evidence: Option<String>,
    #[serde(default)]
    pub discount_history: Vec<DiscountEntry>,
    #[serde(default)]
    pub discount_appetite_remaining: Option<String>,
    #[serde(default)]
    pub budget_cycle_alignment: Option<String>,
    #[serde(default)]
    pub procurement_complexity: Option<ProcurementComplexity>,
    #[serde(default)]
    pub previous_renewal_outcome: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrTrendPoint {
    pub period: String,
    #[serde(default)]
    pub arr: Option<f64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscountEntry {
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub percent_or_amount: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcurementComplexity {
    #[serde(default)]
    pub last_cycle_length_days: Option<i64>,
    #[serde(default)]
    pub signers_required: Option<i64>,
    #[serde(default)]
    pub legal_review_required: Option<bool>,
    #[serde(default)]
    pub known_gotchas: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvocacyTrack {
    #[serde(default)]
    pub is_reference_customer: Option<bool>,
    #[serde(default)]
    pub logo_permission: Option<String>,
    #[serde(default)]
    pub case_study: Option<CaseStudy>,
    #[serde(default)]
    pub speaking_slots: Vec<SpeakingSlot>,
    #[serde(default)]
    pub beta_programs_in: Vec<BetaProgram>,
    #[serde(default)]
    pub referrals_made: Vec<Referral>,
    #[serde(default)]
    pub nps_history: Vec<NpsEntry>,
    #[serde(default)]
    pub advocacy_trend: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseStudy {
    #[serde(default)]
    pub published: Option<bool>,
    #[serde(default)]
    pub in_progress: Option<bool>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub publish_date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakingSlot {
    pub event: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BetaProgram {
    pub program: String,
    #[serde(default)]
    pub enrolled_date: Option<String>,
    #[serde(default)]
    pub engagement_level: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Referral {
    pub referred_company: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpsEntry {
    #[serde(default)]
    pub survey_date: Option<String>,
    #[serde(default)]
    pub score: Option<i64>,
    #[serde(default)]
    pub verbatim: Option<String>,
    #[serde(default)]
    pub respondent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteWallEntry {
    pub quote: String,
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub sentiment: Option<String>,
    #[serde(default)]
    pub why_it_matters: Option<String>,
}

/// Populated by a separate PTY pass reading usage telemetry and per-message
/// sentiment (DOS-204 chapter-by-chapter enrichment). Empty vectors when no
/// pass has run for the entity yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendSignals {
    #[serde(default)]
    pub usage_trajectory: Vec<serde_json::Value>,
    #[serde(default)]
    pub sentiment_over_time: Vec<serde_json::Value>,
}

/// Build the supplemental leading-signals prompt, parameterised on account name
/// and structured disambiguators.
///
/// DOS-287: Prefixes the prompt with an `## Entity disambiguation`,
/// `## Retrieval scope`, and `## Grounding rule` block — same shape as
/// `build_glean_dimension_prompt` — so Glean's retrieval is biased toward
/// documents that reference at least one explicit identifier.
///
/// When `disambiguators` is `None` or empty, the prompt degrades to the
/// original name-only form (no "(none)" lines injected).
pub fn build_leading_signals_prompt(
    account_name: &str,
    disambiguators: Option<&crate::intelligence::prompts::EntityDisambiguators>,
) -> String {
    // DOS-287: Build the structured disambiguation preamble up-front.
    let preamble = build_disambiguation_preamble(account_name, disambiguators);

    format!(
        r#"You are a customer success intelligence system. For the customer account "{account_name}", search ALL available data sources (Salesforce, Zendesk, Gong, Slack, internal documents, LinkedIn data if indexed, org directory, Google Workspace, Notion/Confluence if configured) and extract HIGH-LEVERAGE leading signals that are often missed by standard enrichment.

{preamble}

Focus on EARLY WARNING signals, TRENDS, and DIVERGENCES — not static state. We already have the base account intelligence (ARR, renewal date, stakeholder list, support tickets, recent wins). Do NOT duplicate that. This pull is specifically for the signals below.

## Required Output Format

Respond with a SINGLE JSON object. No prose, no markdown fences, no commentary before or after. Your entire response must be parseable by JSON.parse(). Begin with {{ and end with }}. Nothing else.

The JSON object must have these fields. Omit any field you have no data for — do not fabricate. Return `null` for scalar fields with no data and `[]` for list fields with no data.

{{
  "champion_risk": {{
    "champion_name": "full name of current champion, or null",
    "at_risk": true,
    "risk_level": "low|moderate|high",
    "risk_evidence": ["specific dated signals with sources"],
    "tenure_signal": "e.g. '3.2 years at company, recently promoted'",
    "recent_role_change": "description or null",
    "email_sentiment_trend_30d": "warming|stable|cooling",
    "email_response_time_trend": "faster|stable|slower|unknown",
    "backup_champion_candidates": [
      {{ "name": "full name", "role": "title", "why": "signal", "engagement_level": "high|medium|low" }}
    ]
  }},
  "product_usage_trend": {{
    "overall_trend_30d": "growing|stable|declining|unknown",
    "overall_trend_90d": "growing|stable|declining|unknown",
    "features": [{{ "name": "...", "adoption_status": "active|growing|stable|declining|dormant", "active_users_estimate": null, "usage_trend_30d": "...", "evidence": "..." }}],
    "underutilized_features": [{{ "name": "...", "licensed_but_unused_days": 60, "coaching_opportunity": "..." }}],
    "highly_sticky_features": [{{ "name": "...", "why_sticky": "..." }}],
    "summary": "1-2 sentence rollup"
  }},
  "channel_sentiment": {{
    "email": {{ "sentiment": "...", "trend_30d": "...", "evidence": "..." }},
    "meetings": {{ "sentiment": "...", "trend_30d": "...", "evidence": "..." }},
    "support_tickets": {{ "sentiment": "...", "trend_30d": "...", "evidence": "..." }},
    "slack": {{ "sentiment": "...", "trend_30d": "...", "evidence": "..." }},
    "divergence_detected": true,
    "divergence_summary": "e.g. 'Meetings positive / tickets frustrated'"
  }},
  "transcript_extraction": {{
    "churn_adjacent_questions": [{{ "question": "verbatim", "speaker": "...", "date": "YYYY-MM-DD", "source": "...", "risk_signal": "..." }}],
    "expansion_adjacent_questions": [{{ "question": "verbatim", "speaker": "...", "date": "YYYY-MM-DD", "source": "...", "opportunity_signal": "...", "estimated_arr_upside": null }}],
    "competitor_benchmarks": [{{ "competitor": "...", "context": "...", "threat_level": "mentioned|evaluating|actively_comparing|decision_relevant", "date": "YYYY-MM-DD", "source": "..." }}],
    "decision_maker_shifts": [{{ "shift": "...", "who": "...", "date": "YYYY-MM-DD", "source": "...", "implication": "..." }}],
    "budget_cycle_signals": [{{ "signal": "...", "date": "YYYY-MM-DD", "source": "...", "implication": "...", "locked": false }}]
  }},
  "commercial_signals": {{
    "arr_trend_12mo": [{{ "period": "YYYY-MM", "arr": 185400, "note": "..." }}],
    "arr_direction": "growing|flat|shrinking",
    "payment_behavior": "on-time|occasional-late|chronically-late|disputes|unknown",
    "payment_evidence": "...",
    "discount_history": [{{ "date": "YYYY-MM-DD", "percent_or_amount": "15% or $25K", "reason": "..." }}],
    "discount_appetite_remaining": "full|partial|exhausted|unknown",
    "budget_cycle_alignment": "...",
    "procurement_complexity": {{ "last_cycle_length_days": 45, "signers_required": 3, "legal_review_required": true, "known_gotchas": "..." }},
    "previous_renewal_outcome": "..."
  }},
  "advocacy_track": {{
    "is_reference_customer": true,
    "logo_permission": "yes|no|requested|unknown",
    "case_study": {{ "published": false, "in_progress": false, "topic": "...", "publish_date": null }},
    "speaking_slots": [],
    "beta_programs_in": [],
    "referrals_made": [],
    "nps_history": [],
    "advocacy_trend": "strengthening|stable|cooling"
  }},
  "quote_wall": [
    {{ "quote": "verbatim", "speaker": "full name", "role": "...", "date": "YYYY-MM-DD", "source": "...", "sentiment": "positive|neutral|negative|mixed", "why_it_matters": "..." }}
  ]
}}

## Quality Guidance

- Evidence required. Every signal needs a date and a source.
- Leading over lagging — earliest indicator wins.
- Verbatim quotes only. If you're paraphrasing, don't include it.
- Divergence is signal — flag channel_sentiment.divergence_detected when channels disagree.
- Omit rather than fabricate. Empty arrays and nulls are expected.
- No markdown, no prose, no commentary.

Your response begins with `{{` and ends with `}}`. Nothing else."#,
        account_name = account_name,
        preamble = preamble,
    )
}

/// DOS-287: Render the structured disambiguation preamble shared with
/// dimension prompts. Returns an empty string (no extra newlines) when no
/// disambiguator data is available, so the prompt gracefully degrades.
fn build_disambiguation_preamble(
    account_name: &str,
    disambiguators: Option<&crate::intelligence::prompts::EntityDisambiguators>,
) -> String {
    let mut out = String::new();
    out.push_str("## Entity disambiguation\n");
    out.push_str(&format!("- Name: {}\n", account_name));

    if let Some(d) = disambiguators {
        if !d.known_domains.is_empty() {
            out.push_str(&format!(
                "- Known domains: {}\n",
                d.known_domains.join(", ")
            ));
        }
        if !d.known_contacts.is_empty() {
            out.push_str(&format!(
                "- Known contacts: {}\n",
                d.known_contacts.join(", ")
            ));
        }
        if let Some(ref parent) = d.parent_context {
            if parent.domains.is_empty() {
                out.push_str(&format!("- Parent company: {}\n", parent.name));
            } else {
                out.push_str(&format!(
                    "- Parent company: {} (domains: {})\n",
                    parent.name,
                    parent.domains.join(", ")
                ));
            }
        }
        match d.salesforce_account_id.as_deref() {
            Some(id) => out.push_str(&format!("- Salesforce account ID: {}\n", id)),
            None => out.push_str("- Salesforce account ID: not provided\n"),
        }
    } else {
        out.push_str("- Salesforce account ID: not provided\n");
    }

    out.push_str("\n## Retrieval scope\n");
    out.push_str(&format!(
        "- Prefer documents that reference at least one identifier above (name \"{}\", a known domain, a known contact email, the parent company, or the Salesforce account ID). Treat those as first-class evidence.\n",
        account_name
    ));
    out.push_str(
        "- EXCLUDE documents whose only signal is a different customer's identifier. A document mentioning a different `vip-*.com` host, a different Salesforce account ID, a different customer name, or a different company domain is evidence that document is NOT about this entity — do not draw from it.\n",
    );
    out.push_str(
        "- `wordpress-vip2@assistant.gong.io` and similar shared Gong/Slack bots are multi-tenant note-takers. Their presence in a document says nothing about which specific customer the document concerns.\n",
    );
    if let Some(d) = disambiguators {
        if !d.known_domains.is_empty() {
            out.push_str(&format!(
                "- For this entity, the allowed domain set is exactly: {}. Any other customer domain disqualifies a document.\n",
                d.known_domains.join(", ")
            ));
        }
    }

    out.push_str("\n## Grounding rule\n");
    out.push_str(&format!(
        "Every sentence in your output must be supported by a document that mentions at least one of the known identifiers for \"{}\" above. If you cannot point to such a document for a claim, OMIT the claim entirely — do not fabricate, do not paraphrase adjacent customers, do not substitute a plausible-sounding alternative. Omission is always preferable to cross-customer contamination.",
        account_name
    ));
    out
}

/// Parse a raw Glean chat response into a normalized `HealthOutlookSignals`.
///
/// Tolerant of minor prose preamble — extracts the first balanced JSON object
/// via `extract_json_object`. Glean's prompt contract returns snake_case keys;
/// the struct canonical shape (frontend + on-disk storage) is camelCase. We
/// normalize the parsed JSON keys from snake_case to camelCase recursively
/// before deserializing so that round-tripping write→read never loses data.
/// Unknown fields are ignored; missing fields default to empty/None.
/// Returns `Err` only when no JSON object is present or the JSON is structurally invalid.
pub fn parse_leading_signals(raw: &str) -> Result<HealthOutlookSignals, String> {
    let json_text = extract_json_object(raw)
        .ok_or_else(|| "Glean leading-signals response contained no JSON object".to_string())?;

    let mut value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| format!("Failed to parse leading-signals JSON: {}", e))?;
    snake_to_camel_keys(&mut value);

    serde_json::from_value::<HealthOutlookSignals>(value)
        .map_err(|e| format!("Failed to deserialize leading-signals JSON: {}", e))
}

/// Recursively rewrite object keys from `snake_case` to `camelCase`.
///
/// Used by `parse_leading_signals` so Glean's snake_case output deserializes
/// into the struct's canonical camelCase shape. Keys that are already
/// camelCase (no underscores) pass through unchanged. Array element keys are
/// rewritten in place; scalar values are untouched.
fn snake_to_camel_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let camel = snake_to_camel(&key);
                let mut inner = map.remove(&key).unwrap();
                snake_to_camel_keys(&mut inner);
                map.insert(camel, inner);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                snake_to_camel_keys(item);
            }
        }
        _ => {}
    }
}

fn snake_to_camel(s: &str) -> String {
    if !s.contains('_') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for ch in s.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Derived signal emissions from a parsed `HealthOutlookSignals`.
///
/// Called by `intel_queue.rs` after the signals are persisted so that the
/// 4 new callout-worthy signal types registered in `signals/callouts.rs`
/// flow through the Intelligence Loop (propagation → health → callouts →
/// Bayesian feedback).
pub struct DerivedSignals {
    pub champion_at_risk: Option<String>,
    pub sentiment_divergence: Option<String>,
    pub competitor_decision_relevant: Vec<String>,
    pub budget_cycle_locked: Option<String>,
}

impl HealthOutlookSignals {
    /// Extract the four signal-worthy conditions from the signals bundle.
    ///
    /// Each returned value is a compact JSON payload string suitable for the
    /// `emit_signal_and_propagate` value parameter. `None` / empty means no
    /// emission for that signal type.
    pub fn derive_signals(&self) -> DerivedSignals {
        let champion_at_risk = self.champion_risk.as_ref().and_then(|cr| {
            if cr.at_risk {
                Some(
                    serde_json::json!({
                        "champion": cr.champion_name,
                        "level": cr.risk_level,
                        "evidence": cr.risk_evidence,
                    })
                    .to_string(),
                )
            } else {
                None
            }
        });

        let sentiment_divergence = self.channel_sentiment.as_ref().and_then(|cs| {
            if cs.divergence_detected {
                Some(
                    serde_json::json!({
                        "summary": cs.divergence_summary,
                    })
                    .to_string(),
                )
            } else {
                None
            }
        });

        let competitor_decision_relevant: Vec<String> = self
            .transcript_extraction
            .as_ref()
            .map(|te| {
                te.competitor_benchmarks
                    .iter()
                    .filter(|c| {
                        matches!(
                            c.threat_level.as_deref(),
                            Some("decision_relevant") | Some("actively_comparing")
                        )
                    })
                    .map(|c| {
                        serde_json::json!({
                            "competitor": c.competitor,
                            "threat_level": c.threat_level,
                            "context": c.context,
                            "date": c.date,
                            "source": c.source,
                        })
                        .to_string()
                    })
                    .collect()
            })
            .unwrap_or_default();

        let budget_cycle_locked = self.transcript_extraction.as_ref().and_then(|te| {
            let locked: Vec<_> = te
                .budget_cycle_signals
                .iter()
                .filter(|b| b.locked)
                .collect();
            if locked.is_empty() {
                None
            } else {
                Some(
                    serde_json::json!({
                        "signals": locked.iter().map(|b| serde_json::json!({
                            "signal": b.signal,
                            "date": b.date,
                            "implication": b.implication,
                        })).collect::<Vec<_>>()
                    })
                    .to_string(),
                )
            }
        });

        DerivedSignals {
            champion_at_risk,
            sentiment_divergence,
            competitor_decision_relevant,
            budget_cycle_locked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fully_populated_sample() {
        let raw = r#"{
          "champion_risk": {
            "champion_name": "Ada Lovelace",
            "at_risk": true,
            "risk_level": "moderate",
            "risk_evidence": ["Email response time slowed from 4h to 18h"],
            "backup_champion_candidates": [
              {"name": "Grace Hopper", "role": "VP Eng", "why": "multi-thread engaged", "engagement_level": "high"}
            ]
          },
          "channel_sentiment": {
            "email": {"sentiment": "neutral", "trend_30d": "cooling", "evidence": "3 short replies"},
            "support_tickets": {"sentiment": "frustrated", "trend_30d": "cooling", "evidence": "ticket tone"},
            "divergence_detected": true,
            "divergence_summary": "Meetings positive / tickets frustrated"
          },
          "transcript_extraction": {
            "competitor_benchmarks": [
              {"competitor": "Acme Rival", "context": "evaluating alternatives for Q3", "threat_level": "decision_relevant", "date": "2026-03-01", "source": "Gong call"}
            ],
            "budget_cycle_signals": [
              {"signal": "Q3 budget locked", "date": "2026-02-15", "source": "Gong", "implication": "no net-new spend", "locked": true}
            ]
          },
          "commercial_signals": {
            "arr_direction": "shrinking",
            "payment_behavior": "occasional-late"
          },
          "quote_wall": [
            {"quote": "This saved our quarter.", "speaker": "Ada Lovelace", "role": "VP Data", "date": "2026-03-10", "source": "Gong", "sentiment": "positive", "why_it_matters": "strong reference candidate"}
          ]
        }"#;

        let parsed = parse_leading_signals(raw).expect("parse ok");
        assert!(parsed.champion_risk.unwrap().at_risk);
        let cs = parsed.channel_sentiment.unwrap();
        assert!(cs.divergence_detected);
        assert_eq!(parsed.quote_wall.len(), 1);
    }

    #[test]
    fn parses_sparse_sample_with_nulls() {
        let raw = r#"{"champion_risk": null, "quote_wall": []}"#;
        let parsed = parse_leading_signals(raw).expect("parse ok");
        assert!(parsed.champion_risk.is_none());
        assert!(parsed.quote_wall.is_empty());
    }

    #[test]
    fn tolerates_prose_preamble() {
        let raw = "Sure, here is the JSON:\n\n{\"quote_wall\": []}";
        let parsed = parse_leading_signals(raw).expect("parse ok");
        assert!(parsed.quote_wall.is_empty());
    }

    #[test]
    fn rejects_non_json() {
        assert!(parse_leading_signals("I cannot help with that.").is_err());
    }

    #[test]
    fn derives_champion_at_risk_signal() {
        let mut signals = HealthOutlookSignals::default();
        signals.champion_risk = Some(ChampionRisk {
            champion_name: Some("Ada".into()),
            at_risk: true,
            risk_level: Some("high".into()),
            risk_evidence: vec!["promotion".into()],
            ..Default::default()
        });
        let derived = signals.derive_signals();
        assert!(derived.champion_at_risk.is_some());
        assert!(derived.sentiment_divergence.is_none());
    }

    #[test]
    fn derives_divergence_only_when_flagged() {
        let mut signals = HealthOutlookSignals::default();
        signals.channel_sentiment = Some(ChannelSentiment {
            divergence_detected: false,
            ..Default::default()
        });
        assert!(signals.derive_signals().sentiment_divergence.is_none());

        signals.channel_sentiment = Some(ChannelSentiment {
            divergence_detected: true,
            divergence_summary: Some("meetings vs tickets".into()),
            ..Default::default()
        });
        assert!(signals.derive_signals().sentiment_divergence.is_some());
    }

    #[test]
    fn derives_only_decision_relevant_competitors() {
        let mut signals = HealthOutlookSignals::default();
        signals.transcript_extraction = Some(TranscriptExtraction {
            competitor_benchmarks: vec![
                CompetitorBenchmark {
                    competitor: "A".into(),
                    threat_level: Some("mentioned".into()),
                    ..Default::default()
                },
                CompetitorBenchmark {
                    competitor: "B".into(),
                    threat_level: Some("decision_relevant".into()),
                    ..Default::default()
                },
                CompetitorBenchmark {
                    competitor: "C".into(),
                    threat_level: Some("actively_comparing".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let derived = signals.derive_signals();
        assert_eq!(derived.competitor_decision_relevant.len(), 2);
    }

    #[test]
    fn derives_budget_locked_only_when_locked_flag_set() {
        let mut signals = HealthOutlookSignals::default();
        signals.transcript_extraction = Some(TranscriptExtraction {
            budget_cycle_signals: vec![
                BudgetCycleSignal {
                    signal: "planning".into(),
                    locked: false,
                    ..Default::default()
                },
                BudgetCycleSignal {
                    signal: "Q3 locked".into(),
                    locked: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        assert!(signals.derive_signals().budget_cycle_locked.is_some());
    }

    /// The real DOS-15 correctness gate: parse Glean's snake_case, serialize
    /// as camelCase for storage, then deserialize that camelCase blob back
    /// into the same struct. Every populated field must survive the full
    /// round-trip. Regression guard for the write→read data-loss bug where
    /// camelCase storage couldn't deserialize under a snake_case-only reader.
    #[test]
    fn roundtrip_snake_case_glean_through_camelcase_storage_preserves_fields() {
        let glean_output = r#"{
            "champion_risk": {
                "champion_name": "Alex Morgan",
                "at_risk": true,
                "risk_level": "high",
                "risk_evidence": ["two tickets unanswered", "role change March 4"],
                "backup_champion_candidates": [
                    { "name": "Jordan Kim", "role": "Director of Ops", "engagement_level": "medium" }
                ]
            },
            "channel_sentiment": {
                "divergence_detected": true,
                "divergence_summary": "meetings cordial, tickets frustrated"
            },
            "transcript_extraction": {
                "budget_cycle_signals": [
                    { "signal": "Q3 budget locked", "locked": true }
                ]
            },
            "quote_wall": [
                { "quote": "we love the latest release", "speaker": "Alex Morgan", "sentiment": "positive" }
            ]
        }"#;

        let parsed = parse_leading_signals(glean_output).expect("parse");
        let stored = serde_json::to_string(&parsed).expect("serialize");
        let reread: HealthOutlookSignals = serde_json::from_str(&stored).expect("deserialize");

        let cr = reread.champion_risk.as_ref().expect("champion_risk survives");
        assert_eq!(cr.champion_name.as_deref(), Some("Alex Morgan"));
        assert!(cr.at_risk);
        assert_eq!(cr.risk_level.as_deref(), Some("high"));
        assert_eq!(cr.risk_evidence.len(), 2);
        assert_eq!(cr.backup_champion_candidates.len(), 1);
        assert_eq!(cr.backup_champion_candidates[0].name, "Jordan Kim");

        let cs = reread.channel_sentiment.as_ref().expect("channel_sentiment survives");
        assert!(cs.divergence_detected);
        assert_eq!(cs.divergence_summary.as_deref(), Some("meetings cordial, tickets frustrated"));

        let te = reread.transcript_extraction.as_ref().expect("transcript_extraction survives");
        assert_eq!(te.budget_cycle_signals.len(), 1);
        assert!(te.budget_cycle_signals[0].locked);

        assert_eq!(reread.quote_wall.len(), 1);
        assert_eq!(reread.quote_wall[0].quote, "we love the latest release");

        // Derived signals still fire correctly after round-trip.
        let derived = reread.derive_signals();
        assert!(derived.champion_at_risk.is_some(), "champion risk derives after roundtrip");
        assert!(derived.sentiment_divergence.is_some(), "divergence derives after roundtrip");
        assert!(derived.budget_cycle_locked.is_some(), "budget locked derives after roundtrip");
    }

    #[test]
    fn snake_to_camel_preserves_already_camel() {
        assert_eq!(snake_to_camel("championRisk"), "championRisk");
        assert_eq!(snake_to_camel("champion_risk"), "championRisk");
        assert_eq!(snake_to_camel("nested_a_b_c"), "nestedABC");
        assert_eq!(snake_to_camel("simple"), "simple");
        assert_eq!(snake_to_camel(""), "");
    }
}
