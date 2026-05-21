//! Input/output contracts for `list_people`. See `list_accounts::contracts`
//! for shape rationale — this module mirrors that structure for the
//! Person index.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::abilities::get_entity_intelligence::contracts::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonListInput {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<PersonListFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonListFilter {
    /// Restrict to people whose `role` exactly equals this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Restrict to people linked to this canonical account id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_account_id: Option<String>,
    /// Restrict to people whose display name contains this substring
    /// (case-insensitive). Used by the W2 list shell's filter input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersonSummary {
    pub person_id: String,
    pub display_name: String,
    /// Canonical account this person primarily belongs to, when one is
    /// known. `None` if the person is not yet linked to any account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_account_id: Option<String>,
    pub role: String,
    /// RFC3339 timestamp of the most recent touchpoint reaching this
    /// person, or `None` if the substrate has no touchpoint-backed claim
    /// yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_touchpoint_at: Option<String>,
}
