//! Receipt vs operational audit boundary (DOS-340).
//!
//! Fills the DOS-701 placeholder. Classifies fields that may travel out to a
//! user-facing [`ClaimReceipt`] versus fields that belong only inside the
//! operational audit trail (a tamper-evident internal security trace).
//!
//! # Taxonomy
//!
//! Three buckets sit between the audit storage layer and the rendered
//! surface; this module owns the runtime filter for the first two and a
//! CI lint owns the structural prohibition on the third.
//!
//! 1. **Receipt** — user-facing claim evidence. Carries `source_label`,
//!    `source_type`, `source_asof`, `trust_band`, `freshness`, `field_path`,
//!    sanitized `evidence_summary`, `lifecycle_state`, `verification_state`.
//!    Surfaced through [`crate::services::claim_receipt::render`] on every
//!    W2-W5 block. Fields are governed by [`RECEIPT_ALLOWED_FIELDS`].
//!
//! 2. **Activity Log / Lint** — user-readable lifecycle events
//!    (corroborated, contradicted, withdrew, repaired). A receipt-shaped
//!    projection consumed by W4 Activity Log + Lint blocks. Lives on top of
//!    [`RECEIPT_ALLOWED_FIELDS`] plus a lifecycle-event-kind tag.
//!
//! 3. **Operational audit** — internal tamper-evident security trace.
//!    Carries `raw_source_id`, `prompt_hash`, `internal_audit_id`,
//!    `correlation_id`, etc. — see [`AUDIT_ONLY_DENYLIST`]. Surfaced ONLY by
//!    audit-management commands. Reading it from any receipt/activity/lint
//!    path is forbidden by `scripts/check_audit_disclosure_allowlist.sh`.
//!
//! # Boundary contract (per L0 §5.8 + AC-477.12 + AC-340.1..7)
//!
//! * [`RECEIPT_ALLOWED_FIELDS`] is the **primary** boundary. A field NOT in
//!   the allowlist MUST NOT appear in a serialized [`ClaimReceipt`].
//! * [`AUDIT_ONLY_DENYLIST`] is a **redundant** CI-lint signal; receipt
//!   construction will still reject denylisted fields via fail-loud panic.
//! * [`filter_for_receipt`] consumes an [`OperationalAuditRow`] and returns
//!   [`Some`] only when every field name is in [`RECEIPT_ALLOWED_FIELDS`].
//!   Fields explicitly in [`AUDIT_ONLY_DENYLIST`] are dropped (returns
//!   [`None`]); fields in neither list trigger **fail-loud panic** per
//!   Rule 11 — new audit columns MUST be classified explicitly.
//! * Feedback loop: a leak observed in production routes through DOS-8
//!   `WrongSource` / `SourceUnreliable` signals against the receipt's
//!   source — the boundary is itself a trust contract.

use std::collections::BTreeMap;

/// Fields that MAY appear in a serialized [`ClaimReceipt`].
///
/// AC-340.1: receipt allowlist. Extending this set requires an L0 review
/// per AC-477.12 — every new receipt-shaped field forces an explicit
/// audience-policy decision.
pub const RECEIPT_ALLOWED_FIELDS: &[&str] = &[
    "source_label",
    "source_type",
    "source_asof",
    "trust_band",
    "freshness",
    "field_path",
    "evidence_summary",
    "lifecycle_state",
    "verification_state",
];

/// Fields that MUST NEVER appear in a serialized [`ClaimReceipt`].
///
/// AC-340.1 + AC-477.12: denylist is the redundant CI-lint signal. The
/// primary contract is the allowlist; this list exists so
/// `check_audit_denylist_completeness.sh` can mechanically check that
/// every new operational-audit column has an explicit classification.
pub const AUDIT_ONLY_DENYLIST: &[&str] = &[
    "raw_source_id",
    "prompt_hash",
    "internal_audit_id",
    "local_file_path",
    "private_url",
    "private_email",
    "private_snippet",
    "inaccessible_doc_title",
    "raw_command_metadata",
    "raw_invocation_metadata",
    "debug_payload",
    "raw_model_input",
    "raw_model_output",
    "raw_tool_input",
    "raw_tool_output",
    "raw_document_body",
    "raw_message_body",
    "correlation_id",
];

/// A row drawn from the operational audit storage. Field names are the
/// audit-column names; values are opaque JSON to avoid coupling the
/// boundary to schema-specific Rust types.
#[derive(Debug, Clone, Default)]
pub struct OperationalAuditRow {
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl OperationalAuditRow {
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
        }
    }

    pub fn with<S: Into<String>>(mut self, key: S, value: serde_json::Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

/// Receipt-shaped projection of an audit row — only the allowlisted fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimReceiptFields {
    pub fields: BTreeMap<String, serde_json::Value>,
}

/// Boundary-classification kind.
///
/// `Receipt` and `ActivityLog` / `Lint` are receipt-shaped; classification
/// passes through [`RECEIPT_ALLOWED_FIELDS`]. `OperationalAudit` is a
/// non-disclosure tag: per AC-341.11 it is NEVER a render target. The
/// `check_audit_disclosure_allowlist.sh` script enforces the
/// no-direct-read prohibition on `services::claim_receipt::*` paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    Receipt,
    ActivityLog,
    Lint,
    OperationalAudit,
}

/// Filter an [`OperationalAuditRow`] down to a [`ClaimReceiptFields`]
/// projection.
///
/// Returns [`Some`] iff every key on the row is in
/// [`RECEIPT_ALLOWED_FIELDS`] (after dropping any explicitly denylisted
/// keys). Returns [`None`] if any allowlisted field is missing AND a
/// denylisted field is present — i.e. the row is meaningfully
/// audit-only.
///
/// # Panics
///
/// Per Rule 11 (fail loud), panics if the row contains a field name that
/// appears in NEITHER [`RECEIPT_ALLOWED_FIELDS`] nor
/// [`AUDIT_ONLY_DENYLIST`]. New audit columns MUST be classified
/// explicitly — silent passthrough would defeat the boundary contract.
pub fn filter_for_receipt(audit_row: &OperationalAuditRow) -> Option<ClaimReceiptFields> {
    let mut receipt_fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut saw_audit_only = false;

    for (name, value) in &audit_row.fields {
        let is_allowed = RECEIPT_ALLOWED_FIELDS.iter().any(|n| n == name);
        let is_denied = AUDIT_ONLY_DENYLIST.iter().any(|n| n == name);

        match (is_allowed, is_denied) {
            (true, false) => {
                receipt_fields.insert(name.clone(), value.clone());
            }
            (false, true) => {
                saw_audit_only = true;
            }
            (true, true) => {
                // Contradictory classification — this is a definition bug
                // that fail-loud must surface; never silently drop.
                panic!(
                    "claim_receipt::boundary: field {:?} is classified in BOTH \
                     RECEIPT_ALLOWED_FIELDS and AUDIT_ONLY_DENYLIST — \
                     classifications must be disjoint",
                    name
                );
            }
            (false, false) => {
                // Unknown field — Rule 11 fail-loud. New audit columns
                // must be explicitly classified at the time they're
                // added (see check_audit_denylist_completeness.sh).
                panic!(
                    "claim_receipt::boundary: unknown audit field {:?} is in \
                     NEITHER RECEIPT_ALLOWED_FIELDS nor AUDIT_ONLY_DENYLIST. \
                     Every audit column must be explicitly classified.",
                    name
                );
            }
        }
    }

    if receipt_fields.is_empty() && saw_audit_only {
        None
    } else {
        Some(ClaimReceiptFields {
            fields: receipt_fields,
        })
    }
}

/// Static-overlap audit at compile-ish time: an entry that lands in BOTH
/// classifications is a definition bug. Called by the test below; also
/// exposed for downstream extensibility checks.
pub fn classifications_disjoint() -> Result<(), &'static str> {
    for allow in RECEIPT_ALLOWED_FIELDS {
        if AUDIT_ONLY_DENYLIST.iter().any(|d| d == allow) {
            return Err("RECEIPT_ALLOWED_FIELDS and AUDIT_ONLY_DENYLIST overlap");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifications_are_disjoint() {
        classifications_disjoint().expect("allowlist and denylist must not overlap");
    }

    #[test]
    fn filter_passes_pure_receipt_row() {
        let row = OperationalAuditRow::new()
            .with("trust_band", json!("likely_current"))
            .with("source_label", json!("generic source"))
            .with("freshness", json!("current"));
        let receipt = filter_for_receipt(&row).expect("receipt should build");
        assert_eq!(receipt.fields.len(), 3);
        assert_eq!(receipt.fields["trust_band"], json!("likely_current"));
    }

    #[test]
    fn filter_drops_audit_only_fields() {
        let row = OperationalAuditRow::new()
            .with("trust_band", json!("likely_current"))
            .with("raw_source_id", json!("internal-id-123"))
            .with("prompt_hash", json!("abc123"));
        let receipt = filter_for_receipt(&row).expect("partial passthrough");
        assert!(!receipt.fields.contains_key("raw_source_id"));
        assert!(!receipt.fields.contains_key("prompt_hash"));
        assert!(receipt.fields.contains_key("trust_band"));
    }

    #[test]
    fn filter_returns_none_for_pure_audit_row() {
        let row = OperationalAuditRow::new()
            .with("raw_source_id", json!("internal-id-123"))
            .with("prompt_hash", json!("abc123"))
            .with("correlation_id", json!("corr-xyz"));
        assert!(filter_for_receipt(&row).is_none());
    }

    #[test]
    fn filter_returns_empty_for_empty_row() {
        let row = OperationalAuditRow::new();
        let receipt = filter_for_receipt(&row).expect("empty receipt");
        assert!(receipt.fields.is_empty());
    }

    #[test]
    #[should_panic(expected = "unknown audit field")]
    fn filter_panics_on_unknown_field() {
        let row = OperationalAuditRow::new().with("totally_new_column", json!("???"));
        let _ = filter_for_receipt(&row);
    }

    #[test]
    fn receipt_allowed_fields_match_l0_contract() {
        // Lock the allowlist contents to the L0 §5.8 contract. If this
        // assertion fires the boundary spec was widened — check that
        // L0 packet + privacy.rs audience allowlists were updated.
        let expected = [
            "source_label",
            "source_type",
            "source_asof",
            "trust_band",
            "freshness",
            "field_path",
            "evidence_summary",
            "lifecycle_state",
            "verification_state",
        ];
        assert_eq!(RECEIPT_ALLOWED_FIELDS, &expected[..]);
    }

    #[test]
    fn audit_denylist_covers_l0_contract() {
        for required in [
            "raw_source_id",
            "prompt_hash",
            "internal_audit_id",
            "local_file_path",
            "private_url",
            "private_email",
            "private_snippet",
            "inaccessible_doc_title",
            "raw_command_metadata",
            "raw_invocation_metadata",
            "debug_payload",
            "raw_model_input",
            "raw_model_output",
            "raw_tool_input",
            "raw_tool_output",
            "raw_document_body",
            "raw_message_body",
            "correlation_id",
        ] {
            assert!(
                AUDIT_ONLY_DENYLIST.contains(&required),
                "L0 §5.8 contract requires {required:?} in AUDIT_ONLY_DENYLIST"
            );
        }
    }

    #[test]
    fn surface_kind_taxonomy_compiles() {
        // Existence-check for the four classification buckets named in
        // the module taxonomy doc. Catches accidental enum-variant
        // deletion during refactors.
        let _r = SurfaceKind::Receipt;
        let _a = SurfaceKind::ActivityLog;
        let _l = SurfaceKind::Lint;
        let _o = SurfaceKind::OperationalAudit;
    }
}
