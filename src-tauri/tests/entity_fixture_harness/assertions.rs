//! AC-461.6a / 6b — assertion library.
//!
//! - **`assert_no_bypass`** (AC-461.6a, static check): the rendered surface
//!   sources its claim-substantive text ONLY from the supplied envelope. The
//!   harness enforces this by running source-text scans of the simulated
//!   render snippet for legacy producer identifiers.
//! - **`check_binding`** (AC-461.6a, runtime binding check): every visible
//!   claim-substantive string in the rendered DOM traces back to a
//!   `[data-claim-id]` ancestor. Text-without-binding fails.
//! - **`check_stale_vs_bypass`** (AC-461.6b): unresolved `data-claim-id`
//!   renders as **stale**, NOT **bypass**. The harness produces
//!   `stale_render_count` and `bypass_count` as distinct counters.
//!
//! Bypass denylist covers the producer identifiers spelled out in §5.3:
//! legacy AI JSON, `get_entity_context_entries`, raw SQL, `AppState`,
//! `ActionDb`, page-local intelligence composers, legacy direct detail
//! commands. PHP block render entry points and React `useAccountDetail` /
//! `useProjectDetail` / `usePersonDetail` hooks are NOT denylisted themselves
//! — the hooks are legitimate consumers when they consume the envelope.
//! Bypass is when their internals call non-envelope producers.

use std::collections::BTreeSet;

use abilities_runtime::abilities::get_entity_intelligence::EntityIntelligenceEnvelope;

use crate::harness::dom::{extract_claim_id_attributes, extract_text_with_bindings};

/// Identifiers that, if present in the simulated rendered-source text, signal
/// the renderer bypassed the envelope. Matched case-insensitive substring; the
/// renderer's actual source-of-rendered-text is what's scanned (not just DOM).
const BYPASS_DENYLIST: &[&str] = &[
    // legacy AI JSON path
    "ai_intelligence_json",
    "intelligence_json_v1",
    // legacy direct readers
    "get_entity_context_entries",
    // raw SQL on intelligence tables
    "SELECT FROM intelligence_claims",
    "SELECT * FROM intelligence_claims",
    "FROM intelligence_claims",
    // direct in-process state handles
    "AppState::get_intelligence",
    "ActionDb::query_claims",
    // page-local intelligence composers (the regression class that motivated
    // visually-polished surface that recreates its own composer)
    "compose_account_intelligence",
    "compose_project_intelligence",
    "compose_person_intelligence",
    // legacy direct detail commands (Tauri invoke surface)
    "get_account_detail",
    "get_project_detail",
    "get_person_detail",
];

/// Result of a no-bypass + binding + stale assertion pass.
#[derive(Debug, Default, Clone)]
pub struct RenderAuditReport {
    pub bypass_count: usize,
    pub stale_render_count: usize,
    pub unbound_text_count: usize,
    pub bypass_matches: Vec<String>,
    pub stale_claim_ids: Vec<String>,
    pub unbound_texts: Vec<String>,
}

impl RenderAuditReport {
    pub fn passes(&self) -> bool {
        self.bypass_count == 0
            && self.stale_render_count == 0
            && self.unbound_text_count == 0
    }
}

/// Single-shot audit combining all three checks. Use this in proof tests;
/// individual `check_*` helpers are for diagnostic granularity.
pub fn audit_render(
    rendered_source: &str,
    rendered_dom: &str,
    envelope: &EntityIntelligenceEnvelope,
) -> RenderAuditReport {
    let mut report = RenderAuditReport::default();

    // 1. Static bypass scan — done against the *source* of the renderer,
    //    not the DOM itself. The two are different: PHP render functions
    //    may produce clean DOM while internally calling a bypassed reader.
    //    The harness intentionally inspects the renderer's text source.
    let needle_haystack = rendered_source.to_ascii_lowercase();
    for needle in BYPASS_DENYLIST {
        if needle_haystack.contains(&needle.to_ascii_lowercase()) {
            report.bypass_count += 1;
            report.bypass_matches.push((*needle).to_string());
        }
    }

    // 2. Binding check — every text node traces back to a [data-claim-id]
    //    ancestor (or text is a known chrome string — see allowlist below).
    let texts = extract_text_with_bindings(rendered_dom);
    let chrome_allowlist = chrome_text_allowlist();
    for text in &texts {
        if text.binding.is_some() {
            continue;
        }
        if is_chrome_text(&text.text, &chrome_allowlist) {
            continue;
        }
        report.unbound_text_count += 1;
        report.unbound_texts.push(text.text.clone());
    }

    // 3. Stale vs bypass — every data-claim-id present in the DOM must
    //    resolve to a claim_id known to the envelope. Unresolved IDs are
    //    *stale*, not bypass (per AC-461.6b).
    let envelope_claim_ids = collect_envelope_claim_ids(envelope);
    let dom_claim_ids = extract_claim_id_attributes(rendered_dom);
    for id in &dom_claim_ids {
        if !envelope_claim_ids.contains(id) {
            report.stale_render_count += 1;
            report.stale_claim_ids.push(id.clone());
        }
    }

    report
}

/// AC-461.6a static check — fails if `rendered_source` mentions any legacy
/// producer identifier from `BYPASS_DENYLIST`. Returns the matched needles.
pub fn check_no_bypass(rendered_source: &str) -> Vec<&'static str> {
    let needle_haystack = rendered_source.to_ascii_lowercase();
    BYPASS_DENYLIST
        .iter()
        .filter(|needle| needle_haystack.contains(&needle.to_ascii_lowercase()))
        .copied()
        .collect()
}

/// AC-461.6a runtime binding check — every visible text node has a
/// `[data-claim-id]` ancestor OR is in the chrome-text allowlist.
pub fn check_binding(rendered_dom: &str) -> Vec<String> {
    let allowlist = chrome_text_allowlist();
    extract_text_with_bindings(rendered_dom)
        .into_iter()
        .filter(|t| t.binding.is_none() && !is_chrome_text(&t.text, &allowlist))
        .map(|t| t.text)
        .collect()
}

/// AC-461.6b stale-vs-bypass — distinguishes unresolved `data-claim-id` (stale,
/// not bypass) from no-binding (bypass-shape). Returns `(stale_ids, bypass_ids)`.
pub fn check_stale_vs_bypass(
    rendered_dom: &str,
    envelope: &EntityIntelligenceEnvelope,
) -> (Vec<String>, Vec<String>) {
    let known = collect_envelope_claim_ids(envelope);
    let mut stale = Vec::new();
    for id in extract_claim_id_attributes(rendered_dom) {
        if !known.contains(&id) {
            stale.push(id);
        }
    }
    // Bypass per this check = text without binding (rendered intelligence
    // string with no claim trace). That set is already produced by
    // `check_binding`; we return it here so callers don't have to splice
    // two functions for the "what's bypass vs stale" question.
    let bypass_texts = check_binding(rendered_dom);
    (stale, bypass_texts)
}

fn collect_envelope_claim_ids(envelope: &EntityIntelligenceEnvelope) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for fact in &envelope.facts.items {
        set.insert(fact.claim_id.clone());
    }
    if let Some(health) = &envelope.health_story {
        for row in &health.rows {
            for id in &row.evidence_claim_ids {
                set.insert(id.clone());
            }
        }
    }
    for proposal in &envelope.metadata_proposals.items {
        set.insert(proposal.proposal_id.clone());
    }
    for open_loop in &envelope.open_loops.items {
        set.insert(open_loop.open_loop.id.clone());
        set.insert(open_loop.receipt_target.claim_id.clone());
    }
    for thread in &envelope.threads.items {
        set.insert(thread.thread_id.clone());
    }
    for entry in &envelope.record_entries.items {
        set.insert(entry.claim_id.clone());
    }
    set
}

/// Chrome strings (labels, section headings, empty-state copy) that the
/// renderer legitimately emits without a `data-claim-id` ancestor. Kept
/// short on purpose — anything that looks like claim text should NOT be here.
fn chrome_text_allowlist() -> BTreeSet<&'static str> {
    let mut set = BTreeSet::new();
    let entries = [
        // Section headings
        "Facts",
        "Health",
        "Metadata proposals",
        "Open loops",
        "Touchpoints",
        "Upcoming",
        "Recent",
        "Threads",
        "Record",
        // Trust band labels (these are derived from envelope but are chrome — the
        // claim-substantive content lives in the row, this is just a band marker)
        "Likely current",
        "Use with caution",
        "Needs verification",
        "Unscored",
        // Empty-state copy
        "Nothing here yet",
        "Not connected",
        "Not processed yet",
        "Stale",
        "No relevant touchpoints",
        // Action affordance labels
        "Confidential claim hidden",
        "Click to reveal",
        // Common micro-copy
        "•",
        "—",
        ":",
    ];
    for e in entries {
        set.insert(e);
    }
    set
}

fn is_chrome_text(text: &str, allowlist: &BTreeSet<&'static str>) -> bool {
    allowlist.contains(text)
}

#[cfg(test)]
mod assertion_unit_tests {
    use super::*;

    #[test]
    fn check_no_bypass_flags_legacy_reader() {
        let src = r#"$ctx = get_entity_context_entries($id);"#;
        let matches = check_no_bypass(src);
        assert!(matches.contains(&"get_entity_context_entries"));
    }

    #[test]
    fn check_no_bypass_clean_envelope_source() {
        let src = r#"$envelope = get_entity_intelligence($input);"#;
        assert!(check_no_bypass(src).is_empty());
    }

    #[test]
    fn check_binding_flags_unbound_claim_text() {
        let html = r#"<div><span>renegade rendered claim text</span></div>"#;
        let unbound = check_binding(html);
        assert_eq!(unbound, vec!["renegade rendered claim text".to_string()]);
    }

    #[test]
    fn check_binding_accepts_chrome_text() {
        let html = r#"<section><h2>Facts</h2></section>"#;
        assert!(check_binding(html).is_empty());
    }
}
