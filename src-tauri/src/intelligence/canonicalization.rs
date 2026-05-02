//! Canonical hashing for tombstone item identity.
//! Shared by W0 callers and DOS-7 commit_claim/propose_claim.

use sha2::{Digest, Sha256};

/// The kind of intelligence item being hashed; reserved for forward-compat
/// with DOS-7's claim_type registry (ADR-0125). For W0 we only emit `Risk`/`Win`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::manual_non_exhaustive)]
pub enum ItemKind {
    /// Risk item text.
    Risk,
    /// Recent-win item text.
    Win,
    /// Reserved for DOS-7 expansion; not emitted in W0.
    #[doc(hidden)]
    _Reserved,
}

/// Canonical hash for tombstone matching.
///
/// Stable rule (locked for v1.4.0):
/// 1. Trim leading/trailing whitespace.
/// 2. NFC-normalize Unicode.
/// 3. Collapse internal whitespace runs to a single space.
/// 4. SHA-256 hex digest.
///
/// This rule is intentionally not case-folded and not punctuation-stripped.
/// Changing it is a data migration because existing tombstone hashes are
/// persisted and compared byte-for-byte.
pub fn item_hash(_kind: ItemKind, text: &str) -> String {
    let canonical = canonical_text(text);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Maps a known content-bearing field to its ItemKind, for callers that
/// need to compute a suppression hash. Returns None for fields whose
/// item_key is a structural identifier (e.g., signal_id, action_id).
///
/// Adding a new content-bearing field requires registering it here AND
/// in the writer that emits items into that field, so the reader and
/// writer agree on hash semantics.
pub fn item_kind_for_content_field(field: &str) -> Option<ItemKind> {
    match field {
        "risks" => Some(ItemKind::Risk),
        "recentWins" => Some(ItemKind::Win),
        _ => None,
    }
}

/// Convenience: hash the item text if and only if the field is content-bearing.
/// Returns None for structural-id fields where hash-match doesn't help
/// (e.g., signal_id, action_id, callout_id) — those rely on exact item_key match.
pub fn maybe_item_hash_for_field(field: &str, item_text: Option<&str>) -> Option<String> {
    match (item_kind_for_content_field(field), item_text) {
        (Some(kind), Some(text)) if !text.is_empty() => Some(item_hash(kind, text)),
        _ => None,
    }
}

fn canonical_text(text: &str) -> String {
    let trimmed = text.trim();
    let nfc = normalize_nfc(trimmed);
    nfc.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_nfc(input: &str) -> String {
    unicode_normalization::UnicodeNormalization::nfc(input).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::{item_hash, item_kind_for_content_field, maybe_item_hash_for_field, ItemKind};

    #[test]
    fn canonicalization_basic_ascii() {
        assert_eq!(
            item_hash(ItemKind::Risk, "ARR at risk"),
            item_hash(ItemKind::Risk, "ARR at risk")
        );
    }

    #[test]
    fn canonicalization_nfc_normalization() {
        assert_eq!(
            item_hash(ItemKind::Risk, "Cafe\u{301} renewal"),
            item_hash(ItemKind::Risk, "Café renewal")
        );
    }

    #[test]
    fn canonicalization_whitespace_collapse() {
        assert_eq!(
            item_hash(ItemKind::Win, "  New\tchampion\nidentified  "),
            item_hash(ItemKind::Win, "New champion identified")
        );
    }

    #[test]
    fn canonicalization_mixed_unicode() {
        assert_eq!(
            item_hash(ItemKind::Risk, "  Cafe\u{301}\t東京  "),
            item_hash(ItemKind::Risk, "Café 東京")
        );
    }

    #[test]
    fn canonicalization_idempotent_same_input() {
        let first = item_hash(ItemKind::Risk, "Procurement risk?");
        let second = item_hash(ItemKind::Risk, "Procurement risk?");
        assert_eq!(first, second);
    }

    #[test]
    fn item_hash_is_stable_for_combining_marks() {
        assert_eq!(
            item_hash(ItemKind::Risk, "e\u{0301}"),
            item_hash(ItemKind::Risk, "é")
        );
    }

    #[test]
    fn item_hash_is_stable_for_other_combining_marks() {
        assert_eq!(
            item_hash(ItemKind::Risk, "a\u{0308}"),
            item_hash(ItemKind::Risk, "ä")
        );
    }

    #[test]
    fn item_kind_for_content_field_known_fields_return_kind() {
        assert_eq!(item_kind_for_content_field("risks"), Some(ItemKind::Risk));
        assert_eq!(
            item_kind_for_content_field("recentWins"),
            Some(ItemKind::Win)
        );
    }

    #[test]
    fn item_kind_for_content_field_unknown_field_returns_none() {
        assert_eq!(item_kind_for_content_field("signal_id"), None);
    }

    #[test]
    fn maybe_item_hash_for_field_risks_with_text() {
        let text = "ARR at risk";
        assert_eq!(
            maybe_item_hash_for_field("risks", Some(text)),
            Some(item_hash(ItemKind::Risk, text))
        );
    }

    #[test]
    fn maybe_item_hash_for_field_unknown_field_returns_none() {
        assert_eq!(
            maybe_item_hash_for_field("accountSignal", Some("opaque-id")),
            None
        );
    }

    #[test]
    fn maybe_item_hash_for_field_no_text_returns_none() {
        assert_eq!(maybe_item_hash_for_field("risks", None), None);
    }
}
