use abilities_runtime::types::ClaimSensitivity;
use chrono::{DateTime, Duration, Utc};

use crate::services::claim_receipt::contracts::{Freshness, RedactionLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReceiptRenderKind {
    Render,
    Redacted,
}

pub(crate) fn redaction_for(
    kind: ReceiptRenderKind,
    chain_max: &ClaimSensitivity,
) -> RedactionLevel {
    match (kind, chain_max) {
        (ReceiptRenderKind::Render, _) => RedactionLevel::None,
        (
            ReceiptRenderKind::Redacted,
            ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly,
        ) => RedactionLevel::Partial,
        (ReceiptRenderKind::Redacted, _) => RedactionLevel::Full,
    }
}

pub(crate) fn freshness_for(source_asof: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Freshness {
    let Some(source_asof) = source_asof else {
        return Freshness::Unknown;
    };
    let age = now.signed_duration_since(source_asof);
    if age <= Duration::days(7) {
        Freshness::Current
    } else if age <= Duration::days(30) {
        Freshness::Aging
    } else {
        Freshness::Stale
    }
}

pub(crate) fn generic_source_label(data_source: &str) -> String {
    let lowered = data_source.to_ascii_lowercase();
    if lowered.contains("gmail") || lowered.contains("mail") {
        "email source".to_string()
    } else if lowered.contains("slack") || lowered.contains("chat") {
        "chat source".to_string()
    } else if lowered.contains("calendar") || lowered.contains("event") {
        "calendar source".to_string()
    } else if lowered.contains("doc") || lowered.contains("drive") {
        "document source".to_string()
    } else if lowered.contains("crm") {
        "crm source".to_string()
    } else {
        "primary source".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_boundaries_are_inclusive() {
        let now = DateTime::parse_from_rfc3339("2026-06-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(
            freshness_for(Some(now - Duration::days(7)), now),
            Freshness::Current
        );
        assert_eq!(
            freshness_for(Some(now - Duration::days(30)), now),
            Freshness::Aging
        );
        assert_eq!(
            freshness_for(Some(now - Duration::days(31)), now),
            Freshness::Stale
        );
        assert_eq!(freshness_for(None, now), Freshness::Unknown);
    }

    #[test]
    fn redaction_maps_chain_max_to_receipt_level() {
        assert_eq!(
            redaction_for(ReceiptRenderKind::Render, &ClaimSensitivity::UserOnly),
            RedactionLevel::None
        );
        assert_eq!(
            redaction_for(ReceiptRenderKind::Redacted, &ClaimSensitivity::Confidential),
            RedactionLevel::Partial
        );
        assert_eq!(
            redaction_for(ReceiptRenderKind::Redacted, &ClaimSensitivity::Internal),
            RedactionLevel::Full
        );
    }

    #[test]
    fn source_labels_are_generic() {
        assert_eq!(generic_source_label("gmail_message"), "email source");
        assert_eq!(generic_source_label("calendar_event"), "calendar source");
        assert_eq!(generic_source_label("unknown_system"), "primary source");
    }
}
