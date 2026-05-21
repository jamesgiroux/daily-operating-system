//! Input/output contracts for `list_accounts`.
//!
//! Shape decisions.
//!
//! Input mirrors the W1 envelope-page contract: an optional `cursor`,
//! a required `page_size`, and an optional typed `filter` struct.
//! Unknown filter keys are rejected at the producer via
//! `deny_unknown_fields`.
//!
//! Output is `Paginated<AccountSummary>` from
//! `get_entity_intelligence::contracts` so the client-side
//! `useAbilityCursor` hook treats every list ability uniformly.
//!
//! `AccountSummary` is intentionally the list-row shape — concise
//! identity + status + freshness pointers. Per-account intelligence
//! composition lives in `get_entity_intelligence`; this is the index.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::Cursor;
use crate::abilities::trust::types::TrustBand;

/// Wire input for `list_accounts`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountListInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<AccountListFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    pub page_size: u32,
}

/// Typed filter envelope. Unknown keys reject at deserialization so a
/// client typo doesn't silently match-everything.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountListFilter {
    /// Restrict to accounts whose `status` exactly equals this value.
    /// Empty string is rejected (use `None` to omit the filter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Restrict to accounts whose `health_band` exactly equals this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_band: Option<TrustBand>,
    /// Restrict to accounts whose name contains this substring
    /// (case-insensitive). Used by the W2 list shell's filter input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

/// Concise list-row shape for the W2 Accounts index. Per-account
/// intelligence (claims, facts, touchpoints) is composed separately by
/// `get_entity_intelligence` and is intentionally NOT inlined here.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub account_id: String,
    pub name: String,
    pub status: String,
    pub health_band: TrustBand,
    /// RFC3339 timestamp of the most recent touchpoint reaching this
    /// account, or `None` if the substrate has no touchpoint backed
    /// claim yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_touchpoint_at: Option<String>,
    /// Convenience count of open loops on this account. Cheap to compute
    /// reader-side and avoids a fan-out to `list_open_loops` from the
    /// list shell. Per intelligence-loop discipline this is a derived
    /// projection of the same claim substrate that `list_open_loops`
    /// scans — provenance lives with those claims, not here.
    pub open_loops_count: u32,
}
