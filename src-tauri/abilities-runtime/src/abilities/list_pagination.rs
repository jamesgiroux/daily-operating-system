//! Shared opaque-cursor helpers for entity list-shape Read abilities
//! (`list_accounts`, `list_people`, `list_projects`).
//!
//! Per W1 envelope cursor convention (see
//! `get_entity_intelligence::contracts::Cursor` doc): the cursor is an
//! opaque string the client round-trips as-is. The signing key + rotation
//! policy is a v1.4.6/v1.4.7 concern; at W1 we transport an opaque
//! base64-encoded JSON payload `{offset, watermark}` and let later
//! substrate layer in `HMAC` signing without changing the over-the-wire
//! shape (still an opaque string).
//!
//! The `watermark` is a stable hash of the request's filter + page_size:
//! when a follow-up page arrives with a cursor whose watermark no longer
//! matches the current request, the producer returns
//! `CursorState::Invalidated { restart_required: true }` per §13 / wave §10.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::abilities::get_entity_intelligence::contracts::{Cursor, Paginated};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};

/// Internal cursor payload — what the opaque string base64-decodes to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct CursorPayload {
    pub offset: u64,
    pub watermark: String,
}

/// Encode `(offset, watermark)` into an opaque base64 cursor token.
pub(crate) fn encode_cursor(offset: u64, watermark: &str) -> Cursor {
    let payload = CursorPayload {
        offset,
        watermark: watermark.to_string(),
    };
    let bytes = serde_json::to_vec(&payload).expect("cursor payload serializes");
    Cursor::new(URL_SAFE_NO_PAD.encode(bytes))
}

/// Decode an opaque cursor token. Returns `None` for malformed input —
/// callers map that to `CursorState::Invalidated` so a malformed cursor
/// from a buggy/forged client triggers restart rather than panicking.
pub(crate) fn decode_cursor(cursor: &Cursor) -> Option<CursorPayload> {
    let bytes = URL_SAFE_NO_PAD.decode(cursor.as_str()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Stable hash of an arbitrary request fingerprint (canonical JSON of
/// the filter shape + page_size). Truncated to 16 hex chars — collisions
/// are not a security concern because the gate is "restart on
/// mismatch", not "trust the cursor".
pub(crate) fn watermark_from_request(fingerprint: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fingerprint.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).fold(String::new(), |mut acc, byte| {
        use std::fmt::Write as _;
        write!(acc, "{byte:02x}").expect("writing to a String never fails");
        acc
    })
}

/// Wrap a `Paginated<T>` in an `AbilityOutput` with the minimal provenance
/// envelope a Read ability needs. The list_* abilities are simple
/// projections of underlying claim/entity substrate; the per-row
/// provenance reference lives in the substrate (or the entity-
/// intelligence envelope) — this envelope records the producer call
/// itself, with the subject either named (when a filter pins a single
/// entity) or `Global` (open index).
pub(crate) fn finalize_pagination<T: serde::Serialize + Clone>(
    ctx: &AbilityContext<'_>,
    ability_name: &'static str,
    ability_schema_version: u32,
    subject: SubjectRef,
    body: Paginated<T>,
) -> AbilityResult<Paginated<T>> {
    let mut config = ProvenanceBuilderConfig::new(ability_name, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(1, 0);
    config.ability_schema_version = SchemaVersion(ability_schema_version);
    config.actor = provenance_actor(ctx.actor.clone(), ability_name);
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;

    let mut builder = ProvenanceBuilder::new(config);
    let subject_attr = SubjectAttribution::direct_confident(subject);
    builder.set_subject(subject_attr.clone());

    let make_field_error = |error: &dyn std::fmt::Display| AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("field attribution path failed: {error}"),
    };
    let make_provenance_error = |error: &dyn std::fmt::Display| AbilityError {
        kind: AbilityErrorKind::Validation,
        message: format!("provenance construction failed: {error}"),
    };

    // List abilities are simple read projections — the entire payload
    // (items + cursor metadata) attributes to the ability's subject as a
    // single subtree. Per-row substrate provenance lives in the underlying
    // claim/entity layer (see `get_entity_intelligence::EnvelopeProvenance`);
    // the list ability is a thin index.
    builder
        .attribute_subtree(
            FieldPath::new("").map_err(|error| make_field_error(&error))?,
            FieldAttribution::constant(subject_attr),
        )
        .map_err(|error| make_provenance_error(&error))?;

    builder
        .finalize(body)
        .map_err(|error| make_provenance_error(&error))
}

fn provenance_actor(
    actor: Actor,
    ability_name: &'static str,
) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: format!("agent:{ability_name}"),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::System {
            component: "surface_client".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp".to_string(),
            version: "unknown".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let watermark = watermark_from_request("{\"page_size\":25}");
        let cursor = encode_cursor(50, &watermark);
        let payload = decode_cursor(&cursor).expect("decode round-trips");
        assert_eq!(payload.offset, 50);
        assert_eq!(payload.watermark, watermark);
    }

    #[test]
    fn decode_rejects_garbage() {
        let bad = Cursor::new("!!!not-base64!!!");
        assert!(decode_cursor(&bad).is_none());
    }

    #[test]
    fn watermark_is_stable_and_distinguishes_inputs() {
        let a = watermark_from_request("{\"page_size\":25}");
        let b = watermark_from_request("{\"page_size\":25}");
        let c = watermark_from_request("{\"page_size\":50}");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
