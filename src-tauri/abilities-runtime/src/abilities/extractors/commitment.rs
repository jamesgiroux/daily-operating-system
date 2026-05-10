//! Typed commitment claim extraction primitives.
//!
//! This module is intentionally pure: no database lookups, no clock reads, and
//! no source mutation. Runtime services provide owner resolution and trust
//! inputs, then use these value objects to derive stable commitment identity.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::abilities::claims::{metadata_for_name, ClaimTypeMetadata};
use crate::abilities::trust::{TrustBand, TrustScore};

pub const COMMITMENT_CLAIM_TYPE: &str = "commitment";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentClaim {
    pub commitment_id: String,
    pub account_id: String,
    pub title: String,
    pub title_normalized: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_normalized: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_raw: Option<String>,
    pub owner: OwnerRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<CommitmentTrust>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OwnerRef {
    Person {
        person_id: String,
        display_name: String,
        confidence: f64,
        source: String,
    },
    Team {
        label: String,
        confidence: f64,
        source: String,
    },
    Ambiguous {
        raw: String,
        reason: String,
        candidates: Vec<OwnerCandidate>,
    },
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OwnerCandidate {
    pub person_id: String,
    pub display_name: String,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentTrust {
    pub score: TrustScore,
    pub band: TrustBand,
}

impl CommitmentClaim {
    pub fn new(
        account_id: impl Into<String>,
        title: impl Into<String>,
        due: Option<&str>,
        owner_raw: Option<&str>,
        owner: OwnerRef,
        trust: Option<CommitmentTrust>,
    ) -> Self {
        let account_id = account_id.into();
        let title = title.into();
        let title_normalized = normalize_commitment_title(&title);
        let due_normalized = due.and_then(normalize_due_date);
        let owner_raw_normalized = owner_raw.and_then(normalize_owner_raw);
        let commitment_id = derive_commitment_id_from_normalized(
            &title_normalized,
            &account_id,
            due_normalized.as_deref(),
            owner_raw_normalized.as_deref(),
        );

        Self {
            commitment_id,
            account_id,
            title,
            title_normalized,
            due_normalized,
            owner_raw: owner_raw_normalized,
            owner,
            trust,
        }
    }

    pub fn claim_type_metadata() -> Option<&'static ClaimTypeMetadata> {
        metadata_for_name(COMMITMENT_CLAIM_TYPE)
    }
}

pub fn derive_commitment_id(
    title: &str,
    account_id: &str,
    due: Option<&str>,
    owner_raw: Option<&str>,
) -> String {
    derive_commitment_id_from_normalized(
        &normalize_commitment_title(title),
        account_id,
        due.and_then(normalize_due_date).as_deref(),
        owner_raw.and_then(normalize_owner_raw).as_deref(),
    )
}

pub fn normalize_commitment_title(title: &str) -> String {
    normalize_commitment_identity_text(title)
}

pub fn normalize_commitment_identity_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_punctuation() {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

pub fn normalize_due_date(due: &str) -> Option<String> {
    let trimmed = due.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Preserve date-only values. Timestamp inputs are normalized to their date
    // prefix because action due dates are stored as YYYY-MM-DD.
    let candidate = trimmed.get(..10).unwrap_or(trimmed);
    if is_yyyy_mm_dd(candidate) {
        Some(candidate.to_string())
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

pub fn normalize_owner_raw(owner_raw: &str) -> Option<String> {
    let normalized = normalize_commitment_identity_text(owner_raw);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn derive_commitment_id_from_normalized(
    title_normalized: &str,
    account_id: &str,
    due_normalized: Option<&str>,
    owner_raw: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    update_component(&mut hasher, b"title", title_normalized.as_bytes());
    update_component(&mut hasher, b"account_id", account_id.trim().as_bytes());
    update_component(
        &mut hasher,
        b"due_normalized",
        due_normalized.unwrap_or("").as_bytes(),
    );
    update_component(&mut hasher, b"owner_raw", owner_raw.unwrap_or("").as_bytes());
    format!("commitment:{}", hex::encode(hasher.finalize()))
}

fn update_component(hasher: &mut Sha256, label: &[u8], value: &[u8]) {
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label);
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn is_yyyy_mm_dd(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(idx, b)| idx == 4 || idx == 7 || b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_id_is_stable_for_equivalent_whitespace() {
        let a = derive_commitment_id(
            " Send   Renewal Deck ",
            "acct-1",
            Some("2026-05-12T10:00:00Z"),
            Some(" Alex   Chen "),
        );
        let b = derive_commitment_id(
            "send renewal deck",
            "acct-1",
            Some("2026-05-12"),
            Some("Alex Chen"),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn commitment_id_is_stable_for_case_and_punctuation() {
        let a = derive_commitment_id(
            " Phase 2: Budget! ",
            "acct-1",
            Some("2026-05-12"),
            Some(" Alex.Chen "),
        );
        let b = derive_commitment_id(
            "phase 2 budget",
            "acct-1",
            Some("2026-05-12"),
            Some("alex chen"),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn commitment_id_changes_with_structural_owner() {
        let alex = derive_commitment_id("Send renewal deck", "acct-1", None, Some("Alex"));
        let jamie = derive_commitment_id("Send renewal deck", "acct-1", None, Some("Jamie"));
        assert_ne!(alex, jamie);
    }

    #[test]
    fn commitment_claim_resolves_through_claim_type_registry() {
        let metadata = CommitmentClaim::claim_type_metadata().expect("commitment registered");
        assert_eq!(metadata.name, COMMITMENT_CLAIM_TYPE);
    }
}
