//! DOS-287: Cross-entity contamination validator.
//!
//! Second-line defense against Glean/PTY enrichment writing content about a
//! different customer into the target account's intelligence record. Runs
//! immediately before persistence: if the generated narrative mentions
//! another account's domain, WP-VIP host, or company name (and the target's
//! own name is not also present), the write is rejected and a signal is
//! emitted so the frontend can toast.
//!
//! Heuristics (evaluated in order; first hit wins per unique token):
//!   1. Foreign domain match — any `account_domains.domain` for a non-target
//!      non-archived account, whole-word.
//!   2. Foreign vip-*.com host — matches `vip[0-9]*-[a-z]+\.com` and
//!      `vip[0-9]+\.[a-z]+\.com`. If the host is not in the target's known
//!      domains, it's foreign.
//!   3. Foreign company name — any `accounts.name` for a non-target
//!      non-archived account, normalized, whole-word, ≥4 chars, excluding
//!      `STOPLIST` terms. Suppressed when the target's own name also appears
//!      in the text (legitimate comparison).
//!
//! Matching is case-insensitive; bounded by non-alphanumeric characters.
//!
//! TODO(DOS-282): Regression fixtures for vip2-msd.com / Merck cross-bleed
//! should live with that ticket.

use crate::db::ActionDb;

/// A single cross-entity contamination finding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContaminationHit {
    /// The foreign token we matched, verbatim (lowercased).
    pub foreign_token: String,
    /// What kind of token it is.
    pub kind: ContaminationKind,
    /// The account this token belongs to, if we can identify one. `None`
    /// means we detected the pattern (e.g. a WP-VIP host) but the host
    /// itself isn't registered under any known account.
    pub source_account_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContaminationKind {
    Domain,
    InfrastructureId,
    CompanyName,
}

/// Stopwords that overlap with common account names but are not meaningful
/// for linking — intentionally a superset of
/// `services::entity_linking::rules::p5_title_evidence::STOPLIST`. Kept local
/// so this module has no circular-dependency risk and can be unit-tested in
/// isolation.
const STOPLIST: &[&str] = &[
    "open", "pilot", "plan", "monday", "notion", "mercury", "ramp",
    "handshake", "bridge", "flow", "base", "peak", "note", "space",
    "link", "next", "sync", "ready", "clear", "front", "core", "post",
    "meet", "call", "talk", "chat", "dash", "pulse", "track", "task",
    "work", "team", "loop", "zoom", "slack", "linear", "email",
    "meeting", "customer", "account", "company", "product", "group",
];

/// Scan `text` for references to accounts OTHER than `target_account_id`.
/// Returns every foreign token found. Empty Vec means clean.
///
/// Caller decides whether to reject the write based on the result.
pub fn detect_cross_entity_contamination(
    text: &str,
    target_account_id: &str,
    target_domains: &[String],
    _target_stakeholder_emails: &[String],
    db: &ActionDb,
) -> Vec<ContaminationHit> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let text_lower = text.to_lowercase();
    let target_domains_lower: Vec<String> =
        target_domains.iter().map(|d| d.to_lowercase()).collect();

    // Fetch non-archived accounts with their domains. On DB error, fail open
    // (return no hits) rather than block writes on a read failure.
    let accounts = match db.get_all_accounts_with_domains(false) {
        Ok(a) => a,
        Err(e) => {
            log::warn!(
                "[DOS-287] contamination scan: failed to load accounts: {e}; failing open"
            );
            return Vec::new();
        }
    };

    // Target name (for legitimate-comparison suppression on CompanyName hits).
    let target_name_lower = accounts
        .iter()
        .find(|(a, _)| a.id == target_account_id)
        .map(|(a, _)| a.name.to_lowercase());

    let target_name_in_text = target_name_lower
        .as_deref()
        .map(|n| whole_word_contains(&text_lower, n))
        .unwrap_or(false);

    let mut hits: Vec<ContaminationHit> = Vec::new();
    let mut seen_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Heuristic 1 — foreign domain match.
    for (acct, domains) in &accounts {
        if acct.id == target_account_id {
            continue;
        }
        for d in domains {
            let d_lower = d.to_lowercase();
            if d_lower.is_empty() {
                continue;
            }
            if is_target_owned(&d_lower, &target_domains_lower) {
                continue;
            }
            if !seen_tokens.insert(d_lower.clone()) {
                continue;
            }
            if whole_word_contains(&text_lower, &d_lower) {
                hits.push(ContaminationHit {
                    foreign_token: d_lower,
                    kind: ContaminationKind::Domain,
                    source_account_id: Some(acct.id.clone()),
                });
            }
        }
    }

    // Heuristic 2 — foreign WordPress VIP host pattern.
    // Detect both `vip-*.com` and `vip<digits>-<name>.com` / `vip<digits>.<name>.com`.
    for cap in extract_vip_hosts(&text_lower) {
        if is_target_owned(&cap, &target_domains_lower) {
            continue;
        }
        if !seen_tokens.insert(cap.clone()) {
            continue;
        }
        // Try to attribute to an account if it matches another account's domain.
        let source = accounts
            .iter()
            .find(|(a, ds)| a.id != target_account_id && ds.iter().any(|d| d.to_lowercase() == cap))
            .map(|(a, _)| a.id.clone());
        hits.push(ContaminationHit {
            foreign_token: cap,
            kind: ContaminationKind::InfrastructureId,
            source_account_id: source,
        });
    }

    // Heuristic 3 — foreign company name.
    // Suppressed when target name also appears (legitimate comparison case).
    if !target_name_in_text {
        for (acct, _domains) in &accounts {
            if acct.id == target_account_id {
                continue;
            }
            let name_lower = acct.name.to_lowercase();
            if name_lower.len() < 4 {
                continue;
            }
            // Skip multi-word names where the significant tokens are all stop
            // words or too short — the whole-name match still needs to pass.
            if STOPLIST.contains(&name_lower.as_str()) {
                continue;
            }
            if !seen_tokens.insert(name_lower.clone()) {
                continue;
            }
            if whole_word_contains(&text_lower, &name_lower) {
                hits.push(ContaminationHit {
                    foreign_token: name_lower,
                    kind: ContaminationKind::CompanyName,
                    source_account_id: Some(acct.id.clone()),
                });
            }
        }
    }

    hits
}

/// Concatenate all narrative prose fields from an `IntelligenceJson` so the
/// contamination scanner has a single text block to examine.
///
/// Includes: `executive_assessment`, `pull_quote`, `current_state` prose,
/// risk/win text, stakeholder assessments, company context description,
/// strategic priority rationale, value delivered statements, commitment
/// descriptions, blocker descriptions, expansion opportunity text.
pub fn collect_narrative_text(intel: &super::io::IntelligenceJson) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(ref s) = intel.executive_assessment {
        parts.push(s.clone());
    }
    if let Some(ref s) = intel.pull_quote {
        parts.push(s.clone());
    }
    if let Some(ref cs) = intel.current_state {
        parts.extend(cs.working.iter().cloned());
        parts.extend(cs.not_working.iter().cloned());
        parts.extend(cs.unknowns.iter().cloned());
    }
    for r in &intel.risks {
        parts.push(r.text.clone());
    }
    for w in &intel.recent_wins {
        parts.push(w.text.clone());
    }
    for s in &intel.stakeholder_insights {
        if let Some(ref a) = s.assessment {
            parts.push(a.clone());
        }
    }
    if let Some(ref cc) = intel.company_context {
        if let Some(ref d) = cc.description {
            parts.push(d.clone());
        }
        if let Some(ref ac) = cc.additional_context {
            parts.push(ac.clone());
        }
    }
    for p in &intel.strategic_priorities {
        parts.push(p.priority.clone());
        if let Some(ref c) = p.context {
            parts.push(c.clone());
        }
    }
    for v in &intel.value_delivered {
        parts.push(v.statement.clone());
    }
    if let Some(ref commits) = intel.open_commitments {
        for c in commits {
            parts.push(c.description.clone());
        }
    }
    for b in &intel.blockers {
        parts.push(b.description.clone());
    }
    for e in &intel.expansion_signals {
        parts.push(e.opportunity.clone());
    }
    for m in &intel.market_context {
        parts.push(m.title.clone());
        parts.push(m.body.clone());
    }

    parts.join("\n")
}

/// Runtime policy for the cross-entity contamination validator.
///
/// Default: `ShadowMode` — detect, log, emit signals, surface a toast,
/// but write the data anyway. The contamination signal is *evidence*
/// for the user (and for downstream trust scoring landing in v1.4.0),
/// not a hard gate that blocks the write. With local-only sources the
/// AI has less context and produces more shapes that look like foreign
/// tokens; gating writes on the detector would silently strand fresh
/// accounts at empty state.
///
/// Operators in paranoid environments can opt into hard rejection by
/// setting `DAILYOS_CONTAMINATION_VALIDATION=strict` (or
/// `=reject`/`=reject-on-hit`). Setting it to `off` disables the scan
/// entirely so no signals fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContaminationValidation {
    /// Run the scan; on hit, emit signals/events and REJECT the write.
    RejectOnHit,
    /// Run the scan; on hit, emit signals/events but ALLOW the write.
    ShadowMode,
    /// Do not run the scan.
    Off,
}

impl ContaminationValidation {
    /// Read the policy from the `DAILYOS_CONTAMINATION_VALIDATION` env var.
    /// Unknown / unset values produce the default (`ShadowMode`).
    pub fn from_env() -> Self {
        Self::from_env_value(std::env::var("DAILYOS_CONTAMINATION_VALIDATION").ok().as_deref())
    }

    /// Pure decoder — parses a raw string (or absent) into a policy.
    /// Split out for test purposes so we don't race on process-global env vars.
    pub fn from_env_value(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            Some("off" | "disabled" | "false" | "0") => Self::Off,
            Some("strict" | "reject" | "reject-on-hit" | "reject_on_hit") => Self::RejectOnHit,
            // Default + explicit shadow values land in ShadowMode.
            _ => Self::ShadowMode,
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn rejects(&self) -> bool {
        matches!(self, Self::RejectOnHit)
    }
}

/// Returns true when `token` is exactly one of the target's domains OR a
/// subdomain of one of them. Both inputs expected lowercase already.
///
/// Used by the contamination heuristics so a target's own subdomains
/// (`vip.target.com`, `privacy.target.com`, `www.target.com`) don't get
/// flagged as foreign tokens. DOS-319.
fn is_target_owned(token: &str, target_domains: &[String]) -> bool {
    target_domains.iter().any(|td| {
        if td.is_empty() {
            return false;
        }
        token == td || token.ends_with(&format!(".{td}"))
    })
}

/// Whole-word substring check — `needle` must be bounded by either
/// string boundaries or non-alphanumeric characters on both sides.
/// Both inputs are expected lowercase already.
fn whole_word_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    let mut i = 0;
    while i + n.len() <= h.len() {
        if &h[i..i + n.len()] == n {
            let left_ok = i == 0 || !h[i - 1].is_ascii_alphanumeric();
            let right_idx = i + n.len();
            let right_ok = right_idx == h.len() || !h[right_idx].is_ascii_alphanumeric();
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Extract all WordPress VIP host patterns from `text` (assumed lowercase).
/// Matches `vip-foo.com`, `vip2-msd.com`, and `vip5.something.com` — the host
/// naming used by WordPress VIP's customer infrastructure.
fn extract_vip_hosts(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 3] == b"vip" {
            // Require word boundary on the left.
            if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
                i += 1;
                continue;
            }
            // Walk forward consuming [a-z0-9.\-] until we hit ".com" or a
            // non-host character. We want the full token including dots.
            let mut j = i + 3;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_alphanumeric() || c == b'-' || c == b'.' {
                    j += 1;
                } else {
                    break;
                }
            }
            let candidate = &text[i..j];
            // Accept only if it actually ends in a TLD-shaped suffix like
            // ".com" — we don't want to match bare "vip5" tokens.
            if candidate.ends_with(".com") && candidate.len() > 4 && candidate.contains('-')
                || candidate.ends_with(".com") && candidate.matches('.').count() >= 2
            {
                // Strip any trailing dot just in case.
                let s = candidate.trim_end_matches('.').to_string();
                out.push(s);
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn insert_account(db: &ActionDb, id: &str, name: &str, domains: &[&str]) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, account_type, updated_at, archived) \
                 VALUES (?1, ?2, 'customer', '2026-04-23T00:00:00Z', 0)",
                rusqlite::params![id, name],
            )
            .unwrap();
        for d in domains {
            db.conn_ref()
                .execute(
                    "INSERT INTO account_domains (account_id, domain, source) VALUES (?1, ?2, 'user')",
                    rusqlite::params![id, d.to_lowercase()],
                )
                .unwrap();
        }
    }

    #[test]
    fn empty_text_returns_no_hits() {
        let db = test_db();
        insert_account(&db, "a1", "Alpha", &["alpha.com"]);
        let hits = detect_cross_entity_contamination("", "a1", &["alpha.com".into()], &[], &db);
        assert!(hits.is_empty());
    }

    #[test]
    fn detects_foreign_domain_in_text() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "other", "Merck", &["msd.com", "merck.com"]);

        let text =
            "The latest review mentions vip2-msd.com performance and merck.com customer success.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        let domains: Vec<&str> = hits
            .iter()
            .filter(|h| h.kind == ContaminationKind::Domain)
            .map(|h| h.foreign_token.as_str())
            .collect();
        assert!(domains.contains(&"merck.com"));
    }

    #[test]
    fn allows_target_domain_in_text() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com", "jane.io"]);
        insert_account(&db, "other", "Merck", &["merck.com"]);

        let text = "Jane's deployment at jane.com is healthy.";
        let hits = detect_cross_entity_contamination(
            text,
            "target",
            &["jane.com".into(), "jane.io".into()],
            &[],
            &db,
        );
        assert_eq!(hits.len(), 0, "target-only text should produce no hits");
    }

    #[test]
    fn detects_wpvip_host_pattern_not_in_target_domains() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        // Note: msd.com is NOT registered as an account domain — we still want to
        // flag vip2-msd.com as a foreign WP-VIP host.
        let text = "Performance at vip2-msd.com is concerning.";
        let hits =
            detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        assert!(
            hits.iter()
                .any(|h| h.kind == ContaminationKind::InfrastructureId
                    && h.foreign_token == "vip2-msd.com"),
            "expected InfrastructureId hit for vip2-msd.com, got {:?}",
            hits
        );
    }

    #[test]
    fn detects_foreign_company_name() {
        let db = test_db();
        insert_account(&db, "target", "Jane Software", &["jane.com"]);
        insert_account(&db, "other", "Globex Corporation", &["globex.com"]);

        let text = "Met with Globex Corporation leadership today.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        assert!(hits
            .iter()
            .any(|h| h.kind == ContaminationKind::CompanyName
                && h.foreign_token == "globex corporation"));
    }

    #[test]
    fn allows_foreign_name_when_target_name_also_present() {
        let db = test_db();
        insert_account(&db, "target", "Jane Software", &["jane.com"]);
        insert_account(&db, "other", "Globex Corporation", &["globex.com"]);

        let text =
            "Compared Jane Software's approach with Globex Corporation's during the review.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        // Globex should be suppressed — target name is also in the text.
        assert!(!hits
            .iter()
            .any(|h| h.kind == ContaminationKind::CompanyName));
    }

    #[test]
    fn filters_short_names_and_stop_words() {
        let db = test_db();
        insert_account(&db, "target", "Jane Software", &["jane.com"]);
        // Short name and a stopword-ish name.
        insert_account(&db, "a", "Abc", &[]);
        insert_account(&db, "b", "sync", &[]);
        insert_account(&db, "c", "Chat", &[]);

        let text = "Quick sync call via Abc Chat for Jane's team.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        // None of Abc / sync / Chat should trigger — short / stoplist.
        assert!(!hits
            .iter()
            .any(|h| h.kind == ContaminationKind::CompanyName),
            "unexpected CompanyName hits: {:?}",
            hits);
    }

    #[test]
    fn case_insensitive_matching() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "other", "Merck", &["MSD.com"]);

        let text = "Reference to MSD.COM in the notes.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        assert!(hits
            .iter()
            .any(|h| h.kind == ContaminationKind::Domain && h.foreign_token == "msd.com"));
    }

    #[test]
    fn collect_narrative_text_pulls_all_prose_fields() {
        use crate::intelligence::io::*;
        let intel = IntelligenceJson {
            executive_assessment: Some("Assessment text.".into()),
            pull_quote: Some("One sentence.".into()),
            current_state: Some(CurrentState {
                working: vec!["adoption strong".into()],
                not_working: vec!["support slow".into()],
                unknowns: vec!["renewal status".into()],
            }),
            risks: vec![IntelRisk {
                text: "Churn risk on vip2-msd.com".into(),
                source: None,
                urgency: "watch".into(),
                item_source: None,
                discrepancy: None,

                ..Default::default()

            }],
            recent_wins: vec![IntelWin {
                text: "Saved $50K".into(),
                source: None,
                impact: None,
                item_source: None,
                discrepancy: None,
            }],
            ..Default::default()
        };
        let narrative = collect_narrative_text(&intel);
        assert!(narrative.contains("Assessment text."));
        assert!(narrative.contains("One sentence."));
        assert!(narrative.contains("adoption strong"));
        assert!(narrative.contains("support slow"));
        assert!(narrative.contains("renewal status"));
        assert!(narrative.contains("Churn risk on vip2-msd.com"));
        assert!(narrative.contains("Saved $50K"));
    }

    #[test]
    fn collect_narrative_text_on_empty_intel_is_empty() {
        let intel = crate::intelligence::io::IntelligenceJson::default();
        let narrative = collect_narrative_text(&intel);
        assert_eq!(narrative.trim(), "");
    }

    #[test]
    fn contamination_validation_default_is_shadow_mode() {
        // Default policy: detect + log + emit, but allow the write.
        // Strict rejection is opt-in only.
        let p = ContaminationValidation::from_env_value(None);
        assert_eq!(p, ContaminationValidation::ShadowMode);
        assert!(p.is_enabled());
        assert!(!p.rejects());
    }

    #[test]
    fn contamination_validation_shadow_mode_parse() {
        for raw in ["shadow", "SHADOW", "shadow_mode", "log-only"] {
            let p = ContaminationValidation::from_env_value(Some(raw));
            assert_eq!(p, ContaminationValidation::ShadowMode, "raw={raw}");
            assert!(p.is_enabled());
            assert!(!p.rejects());
        }
    }

    #[test]
    fn contamination_validation_strict_mode_parse() {
        // Strict rejection is opt-in via explicit env var values.
        for raw in ["strict", "reject", "reject-on-hit", "reject_on_hit", "STRICT"] {
            let p = ContaminationValidation::from_env_value(Some(raw));
            assert_eq!(p, ContaminationValidation::RejectOnHit, "raw={raw}");
            assert!(p.is_enabled());
            assert!(p.rejects());
        }
    }

    #[test]
    fn contamination_validation_off_parse() {
        for raw in ["off", "disabled", "false", "0"] {
            let p = ContaminationValidation::from_env_value(Some(raw));
            assert_eq!(p, ContaminationValidation::Off, "raw={raw}");
            assert!(!p.is_enabled());
        }
    }

    #[test]
    fn contamination_validation_unknown_value_defaults_to_shadow() {
        let p = ContaminationValidation::from_env_value(Some("nonsense"));
        assert_eq!(p, ContaminationValidation::ShadowMode);
    }

    // Integration-style: hit on a cross-entity narrative field produces hits
    // that the write-site would reject. Uses `collect_narrative_text` + the
    // scanner directly so we exercise the same code path as the production
    // integration point in `intel_queue::write_enrichment_results`.
    #[test]
    fn rejects_write_when_output_contains_foreign_domain() {
        use crate::intelligence::io::*;
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "merck", "Merck", &["vip2-msd.com", "merck.com"]);

        let intel = IntelligenceJson {
            executive_assessment: Some(
                "WordPress VIP performance at vip2-msd.com remains stable.".into(),
            ),
            ..Default::default()
        };
        let narrative = collect_narrative_text(&intel);
        let hits =
            detect_cross_entity_contamination(&narrative, "target", &["jane.com".into()], &[], &db);
        assert!(!hits.is_empty(), "expected at least one contamination hit");
        assert!(hits
            .iter()
            .any(|h| h.foreign_token == "vip2-msd.com" || h.foreign_token == "merck"
                || h.kind == ContaminationKind::InfrastructureId));
    }

    #[test]
    fn allows_write_when_output_is_clean() {
        use crate::intelligence::io::*;
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "merck", "Merck", &["vip2-msd.com"]);

        let intel = IntelligenceJson {
            executive_assessment: Some(
                "Jane's team delivered the migration on time; jane.com infra is stable.".into(),
            ),
            ..Default::default()
        };
        let narrative = collect_narrative_text(&intel);
        let hits =
            detect_cross_entity_contamination(&narrative, "target", &["jane.com".into()], &[], &db);
        assert!(hits.is_empty(), "expected clean narrative, got: {:?}", hits);
    }

    #[test]
    fn reject_hit_carries_source_account_id_when_attributable() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "merck", "Merck", &["msd.com"]);

        let text = "Reference to msd.com in the notes.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        let merck_hit = hits.iter().find(|h| h.foreign_token == "msd.com").unwrap();
        assert_eq!(merck_hit.source_account_id.as_deref(), Some("merck"));
    }

    // DOS-319: subdomains of the target's own domains must not be flagged.
    #[test]
    fn target_owned_subdomain_not_flagged_as_vip_host() {
        let db = test_db();
        insert_account(&db, "target", "Globex", &["globex.com"]);
        let text = "Performance at vip.globex.com remains stable.";
        let hits = detect_cross_entity_contamination(
            text,
            "target",
            &["globex.com".into()],
            &[],
            &db,
        );
        assert!(
            hits.is_empty(),
            "expected zero hits for target's own subdomain, got: {:?}",
            hits
        );
    }

    #[test]
    fn target_owned_multi_level_subdomain_not_flagged() {
        let db = test_db();
        insert_account(&db, "target", "Globex", &["globex.com"]);
        let text = "Routed via internal.privacy.globex.com tonight.";
        let hits = detect_cross_entity_contamination(
            text,
            "target",
            &["globex.com".into()],
            &[],
            &db,
        );
        assert!(
            hits.is_empty(),
            "expected zero hits for multi-level target subdomain, got: {:?}",
            hits
        );
    }

    #[test]
    fn genuine_foreign_vip_host_still_flagged_after_subdomain_fix() {
        let db = test_db();
        insert_account(&db, "target", "Globex", &["globex.com"]);
        let text = "Performance at vip-othercustomer.com is degraded.";
        let hits = detect_cross_entity_contamination(
            text,
            "target",
            &["globex.com".into()],
            &[],
            &db,
        );
        let infra_hits: Vec<_> = hits
            .iter()
            .filter(|h| h.kind == ContaminationKind::InfrastructureId)
            .collect();
        assert_eq!(
            infra_hits.len(),
            1,
            "expected exactly one InfrastructureId hit for foreign VIP host, got: {:?}",
            hits
        );
        assert_eq!(infra_hits[0].foreign_token, "vip-othercustomer.com");
    }

    #[test]
    fn target_with_multiple_domains_subdomain_not_flagged() {
        let db = test_db();
        insert_account(&db, "target", "Globex", &["globex.com", "globex.io"]);
        let text = "Customer routes through app.globex.io for staging.";
        let hits = detect_cross_entity_contamination(
            text,
            "target",
            &["globex.com".into(), "globex.io".into()],
            &[],
            &db,
        );
        assert!(
            hits.is_empty(),
            "expected zero hits when subdomain belongs to a secondary target domain, got: {:?}",
            hits
        );
    }

    #[test]
    fn whole_word_bounded_not_substring() {
        let db = test_db();
        insert_account(&db, "target", "Jane", &["jane.com"]);
        insert_account(&db, "other", "Merck", &["merck.com"]);

        // "merck.com" as a pure substring inside another word shouldn't match.
        let text = "See prefix-merck.com-suffix.foo.";
        let hits = detect_cross_entity_contamination(text, "target", &["jane.com".into()], &[], &db);
        // Domains use `.` / `-` as boundaries, so prefix- and -suffix both
        // make this a valid whole-word match. That's the desired behavior for
        // a domain (dots and hyphens are boundaries). The test documents it.
        assert!(
            hits.iter().any(|h| h.foreign_token == "merck.com"),
            "expected merck.com to match at non-alphanumeric boundary"
        );
    }
}
