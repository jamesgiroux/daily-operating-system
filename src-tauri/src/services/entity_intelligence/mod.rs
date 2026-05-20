//! DOS-477 — entity-detail trust-boundary helpers.
//!
//! Per L0 packet `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.4.
//!
//! This module is the substrate-side boundary the W2 entity detail surfaces compose against.
//! It does NOT re-implement the sensitivity gate; it composes the shipped primitives in
//! `abilities_runtime::sensitivity` (`render_policy_for_surface`, `renderable_claim_text_with_value`).
//!
//! Receipt-allowlist contract (CSO Finding 2, AC-477.12) — the receipt allowlist
//! (`RECEIPT_ALLOWED_FIELDS`, sibling §5.8 DOS-340) is the **primary** boundary contract.
//! The denylist (`AUDIT_ONLY_DENYLIST`) becomes a redundant CI lint. `filter_for_receipt`
//! panics on unknown field names (fail-loud per Rule 11).

pub mod auth;
pub mod envelope_cache;
pub mod touchpoints;
