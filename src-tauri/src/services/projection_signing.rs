use std::collections::{BTreeSet, HashMap};
use std::process::Command;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, SecondsFormat};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

pub const PROJECTION_KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.projection-signing";
pub const PROJECTION_SIGNATURE_ALG: &str = "Ed25519";
pub const PROJECTION_CANONICALIZATION: &str = "RFC8785-JSON";
pub const WP_PROJECTION_DOMAIN: &str = "dailyos.wp_studio.projection.v1";
pub const MARKDOWN_PROJECTION_DOMAIN: &str = "dailyos.markdown.projection.v1";
pub const KEYRING_MAX_AGE_SECONDS: u64 = 300;
pub const PROJECTION_SIGNATURE_INVALID_SIGNAL: &str = "signature_invalid";

#[derive(Debug, thiserror::Error)]
pub enum ProjectionSigningError {
    #[error("database error: {0}")]
    Database(String),
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("service error: {0}")]
    Service(String),
    #[error("projection not found: {0}")]
    ProjectionNotFound(String),
}

impl From<rusqlite::Error> for ProjectionSigningError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<serde_json::Error> for ProjectionSigningError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error.to_string())
    }
}

pub trait ProjectionKeyStore: Send + Sync {
    fn get_private_key(&self, account_ref: &str) -> Result<Zeroizing<Vec<u8>>, ProjectionSigningError>;
    fn put_private_key(&self, account_ref: &str, pkcs8: &[u8]) -> Result<(), ProjectionSigningError>;
}

#[derive(Debug, Default)]
pub struct MacOsProjectionKeyStore;

impl ProjectionKeyStore for MacOsProjectionKeyStore {
    fn get_private_key(&self, account_ref: &str) -> Result<Zeroizing<Vec<u8>>, ProjectionSigningError> {
        let output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                PROJECTION_KEYCHAIN_SERVICE,
                "-a",
                account_ref,
                "-w",
            ])
            .output()
            .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectionSigningError::Keychain(format!(
                "projection signing key read failed: {}",
                stderr.trim()
            )));
        }

        let encoded = String::from_utf8(output.stdout)
            .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded.trim())
            .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;
        Ok(Zeroizing::new(bytes))
    }

    fn put_private_key(&self, account_ref: &str, pkcs8: &[u8]) -> Result<(), ProjectionSigningError> {
        let encoded = URL_SAFE_NO_PAD.encode(pkcs8);
        let output = Command::new("security")
            .arg("add-generic-password")
            .arg("-s")
            .arg(PROJECTION_KEYCHAIN_SERVICE)
            .arg("-a")
            .arg(account_ref)
            .arg("-w")
            .arg(encoded)
            .arg("-U")
            .output()
            .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectionSigningError::Keychain(format!(
                "projection signing key write failed: {}",
                stderr.trim()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionSurface {
    WordpressDb,
    MarkdownFile,
}

impl ProjectionSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WordpressDb => "wordpress_db",
            Self::MarkdownFile => "markdown_file",
        }
    }

    pub fn domain_separator(self) -> &'static str {
        match self {
            Self::WordpressDb => WP_PROJECTION_DOMAIN,
            Self::MarkdownFile => MARKDOWN_PROJECTION_DOMAIN,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKeyStatus {
    Active,
    Rotating,
    Retired,
    Revoked,
}

impl ProjectionKeyStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Rotating => "rotating",
            Self::Retired => "retired",
            Self::Revoked => "revoked",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "rotating" => Self::Rotating,
            "retired" => Self::Retired,
            "revoked" => Self::Revoked,
            _ => Self::Active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionClaimRefInput {
    pub claim_id: String,
    pub claim_version: u64,
    pub field_path: Option<String>,
    pub provenance_invocation_id: Option<String>,
    pub provenance_field_path: Option<String>,
    pub scope_grant_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionBlockInput {
    pub block_id: String,
    pub block_order: u64,
    pub block_type: String,
    pub block_payload: Value,
    pub claim_refs: Vec<ProjectionClaimRefInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionWriteInput {
    pub projection_id: String,
    pub surface: ProjectionSurface,
    pub surface_locator: String,
    pub dailyos_canonical_id: String,
    pub dailyos_source_runtime: String,
    pub dailyos_projection_version: u64,
    pub composition_id: String,
    pub composition_version: u64,
    pub blocks: Vec<ProjectionBlockInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedProjectionPayload {
    pub domain_separator: String,
    pub projection_id: String,
    pub surface: ProjectionSurface,
    pub surface_locator_hash: String,
    pub dailyos_canonical_id: String,
    pub dailyos_source_runtime: String,
    pub dailyos_projection_version: u64,
    pub composition_id: String,
    pub composition_version: u64,
    pub blocks: Vec<SignedProjectionBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedProjectionBlock {
    pub block_id: String,
    pub block_order: u64,
    pub block_type: String,
    pub block_payload: Value,
    pub block_payload_sha256: String,
    pub claim_refs: Vec<SignedProjectionClaimRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedProjectionClaimRef {
    pub claim_id: String,
    pub claim_version: u64,
    pub field_path: Option<String>,
    pub provenance_invocation_id: Option<String>,
    pub provenance_field_path: Option<String>,
    pub scope_grant_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionSignatureEnvelope {
    pub signature_id: String,
    pub key_id: String,
    pub alg: String,
    pub canonicalization: String,
    pub signed_at: String,
    pub signature_b64url: String,
    pub dailyos_source_runtime: String,
    pub dailyos_projection_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedProjectionWrite {
    pub payload: SignedProjectionPayload,
    pub signature_envelope: ProjectionSignatureEnvelope,
    pub signature_envelope_b64url: String,
    pub canonical_signed_payload_sha256: String,
    pub claim_watermark_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionKeyringResponse {
    pub ok: bool,
    pub runtime_anchor_id: String,
    pub keyring_version: u64,
    pub max_age_seconds: u64,
    pub keys: Vec<ProjectionKeyringKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionKeyringKey {
    pub key_id: String,
    pub public_key_b64: String,
    pub key_status: ProjectionKeyStatus,
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub retired_at: Option<String>,
    pub revoked_at: Option<String>,
    pub replacement_key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionVerificationStatus {
    Verified,
    MissingSignature,
    UnsupportedAlgorithm,
    UnsupportedCanonicalization,
    UnknownKey,
    RevokedKey,
    SignatureInvalid,
    VersionRollback,
    WrongRuntimeAnchor,
}

impl ProjectionVerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::MissingSignature => "missing_signature",
            Self::UnsupportedAlgorithm => "unsupported_algorithm",
            Self::UnsupportedCanonicalization => "unsupported_canonicalization",
            Self::UnknownKey => "unknown_key",
            Self::RevokedKey => "revoked_key",
            Self::SignatureInvalid => "signature_invalid",
            Self::VersionRollback => "rollback",
            Self::WrongRuntimeAnchor => "wrong_runtime_anchor",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionTamperBridgeFields {
    pub projection_id: String,
    pub signature_id: String,
    pub key_id: String,
    pub observed_signature_status: String,
    pub quarantine_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionRollbackBridgeFields {
    pub projection_id: String,
    pub signed_composition_version: u64,
    pub ledger_composition_version: u64,
    pub signed_claim_version: Option<u64>,
    pub ledger_claim_version: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum ProjectionVerificationFailure {
    Tampered(ProjectionTamperBridgeFields),
    VersionRollback(ProjectionRollbackBridgeFields),
}

#[derive(Debug, Clone)]
pub struct ProjectionVerificationOutcome {
    pub status: ProjectionVerificationStatus,
    pub projection_id: String,
    pub quarantine_id: Option<String>,
    pub failure: Option<ProjectionVerificationFailure>,
}

#[derive(Debug, Clone)]
pub struct ProjectionVerificationInput {
    pub projection_id: String,
    pub surface: ProjectionSurface,
    pub surface_locator: String,
    pub expected_runtime_anchor_id: Option<String>,
    pub payload: SignedProjectionPayload,
    pub signature_envelope_b64url: Option<String>,
}

#[derive(Debug, Clone)]
struct ProjectionSigningKey {
    key_id: String,
    public_key_b64: String,
    key_status: ProjectionKeyStatus,
    keychain_account_ref: String,
    valid_from: String,
    valid_until: Option<String>,
    retired_at: Option<String>,
    revoked_at: Option<String>,
    replacement_key_id: Option<String>,
}

#[derive(Debug, Clone)]
struct LedgerCurrentness {
    current_signature_id: Option<String>,
    key_id: Option<String>,
    signature_status: Option<String>,
    composition_version: u64,
    claim_versions: HashMap<String, u64>,
    canonical_signed_payload_sha256: String,
    claim_watermark_sha256: String,
}

pub fn sign_projection(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    input: ProjectionWriteInput,
) -> Result<SignedProjectionWrite, ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let active_key = ensure_active_signing_key(ctx, db, key_store)?;
    let private_key = key_store.get_private_key(&active_key.keychain_account_ref)?;
    let key_pair = Ed25519KeyPair::from_pkcs8(private_key.as_ref())
        .map_err(|error| ProjectionSigningError::Crypto(format!("invalid Ed25519 key: {error}")))?;

    let payload = signed_payload_from_input(&input)?;
    let canonical_bytes = canonical_json_bytes(&payload)?;
    let canonical_hash = sha256_hex(&canonical_bytes);
    let claim_watermark = claim_watermark_hash(&payload)?;
    let signature = key_pair.sign(&canonical_bytes);
    let signature_id = format!("sig_{}", Uuid::new_v4().simple());
    let signed_at = timestamp(ctx);
    let envelope = ProjectionSignatureEnvelope {
        signature_id: signature_id.clone(),
        key_id: active_key.key_id.clone(),
        alg: PROJECTION_SIGNATURE_ALG.to_string(),
        canonicalization: PROJECTION_CANONICALIZATION.to_string(),
        signed_at: signed_at.clone(),
        signature_b64url: URL_SAFE_NO_PAD.encode(signature.as_ref()),
        dailyos_source_runtime: input.dailyos_source_runtime.clone(),
        dailyos_projection_version: input.dailyos_projection_version,
    };
    let envelope_b64url = URL_SAFE_NO_PAD.encode(canonical_json_bytes(&envelope)?);

    let result = SignedProjectionWrite {
        payload: payload.clone(),
        signature_envelope: envelope.clone(),
        signature_envelope_b64url: envelope_b64url.clone(),
        canonical_signed_payload_sha256: canonical_hash.clone(),
        claim_watermark_sha256: claim_watermark.clone(),
    };

    db.with_transaction(|tx| {
        upsert_ledger_and_signature(
            ctx,
            tx,
            &input,
            &payload,
            &envelope,
            &canonical_bytes,
            &canonical_hash,
            &claim_watermark,
            signature.as_ref(),
            &envelope_b64url,
            &signed_at,
        )
        .map_err(|error| error.to_string())
    })
    .map_err(ProjectionSigningError::Database)?;

    Ok(result)
}

pub fn verify_projection_read(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ProjectionVerificationInput,
) -> Result<ProjectionVerificationOutcome, ProjectionSigningError> {
    let Some(envelope_b64url) = input.signature_envelope_b64url.as_deref() else {
        return quarantine_tamper(ctx, db, &input, None, ProjectionVerificationStatus::MissingSignature);
    };

    let envelope = match decode_signature_envelope(envelope_b64url) {
        Ok(envelope) => envelope,
        Err(status) => return quarantine_tamper(ctx, db, &input, None, status),
    };

    if envelope.alg != PROJECTION_SIGNATURE_ALG {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::UnsupportedAlgorithm,
        );
    }
    if envelope.canonicalization != PROJECTION_CANONICALIZATION {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::UnsupportedCanonicalization,
        );
    }
    if let Some(expected) = input.expected_runtime_anchor_id.as_deref() {
        if envelope.dailyos_source_runtime != expected || input.payload.dailyos_source_runtime != expected {
            return quarantine_tamper(
                ctx,
                db,
                &input,
                Some(&envelope),
                ProjectionVerificationStatus::WrongRuntimeAnchor,
            );
        }
    }

    let Some(key) = load_signing_key(db, &envelope.key_id)? else {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::UnknownKey,
        );
    };
    if key.key_status == ProjectionKeyStatus::Revoked {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::RevokedKey,
        );
    }

    let canonical_bytes = canonical_json_bytes(&input.payload)?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(envelope.signature_b64url.as_bytes())
        .map_err(|error| ProjectionSigningError::Serialization(error.to_string()))?;
    let public_key_bytes = URL_SAFE_NO_PAD
        .decode(key.public_key_b64.as_bytes())
        .map_err(|error| ProjectionSigningError::Serialization(error.to_string()))?;
    let public_key = UnparsedPublicKey::new(&ED25519, public_key_bytes);
    if public_key.verify(&canonical_bytes, &signature_bytes).is_err() {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::SignatureInvalid,
        );
    }

    if let Some(rollback) = detect_currentness_rollback(db, &input, &envelope, &canonical_bytes)? {
        let quarantine_id = record_quarantine(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::VersionRollback,
        )?;
        return Ok(ProjectionVerificationOutcome {
            status: ProjectionVerificationStatus::VersionRollback,
            projection_id: input.projection_id,
            quarantine_id: Some(quarantine_id),
            failure: Some(ProjectionVerificationFailure::VersionRollback(rollback)),
        });
    }

    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE projection_ledger
                    SET last_verified_at = ?2,
                        verification_status = 'verified',
                        quarantine_state = CASE
                            WHEN quarantine_state = 'quarantined' THEN quarantine_state
                            ELSE 'none'
                        END
                  WHERE projection_id = ?1",
                params![input.projection_id, timestamp(ctx)],
            )
            .map_err(|error| error.to_string())?;
        crate::services::signals::emit_in_transaction(
            ctx,
            tx,
            "projection",
            &input.projection_id,
            "projection_verified",
            "projection_signing",
            json!({
                "signature_id": envelope.signature_id,
                "key_id": envelope.key_id,
            }),
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;

    Ok(ProjectionVerificationOutcome {
        status: ProjectionVerificationStatus::Verified,
        projection_id: input.projection_id,
        quarantine_id: None,
        failure: None,
    })
}

pub fn public_keyring(
    db: &ActionDb,
    runtime_anchor_id: impl Into<String>,
) -> Result<ProjectionKeyringResponse, ProjectionSigningError> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT key_id, public_key_b64, key_status, valid_from, valid_until,
                retired_at, revoked_at, replacement_key_id
           FROM projection_signing_keys
          ORDER BY created_at DESC, key_id DESC",
    )?;
    let keys = stmt
        .query_map([], |row| {
            Ok(ProjectionKeyringKey {
                key_id: row.get("key_id")?,
                public_key_b64: row.get("public_key_b64")?,
                key_status: ProjectionKeyStatus::from_str(row.get::<_, String>("key_status")?.as_str()),
                valid_from: row.get("valid_from")?,
                valid_until: row.get("valid_until")?,
                retired_at: row.get("retired_at")?,
                revoked_at: row.get("revoked_at")?,
                replacement_key_id: row.get("replacement_key_id")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProjectionKeyringResponse {
        ok: true,
        runtime_anchor_id: runtime_anchor_id.into(),
        keyring_version: keys.len() as u64,
        max_age_seconds: KEYRING_MAX_AGE_SECONDS,
        keys,
    })
}

pub fn revoke_projection_signing_key(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    key_id: &str,
    reason: &str,
) -> Result<String, ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let replacement = create_signing_key(ctx, db, key_store, ProjectionKeyStatus::Rotating, reason)?;
    let now = timestamp(ctx);
    db.with_transaction(|tx| {
        let previous = tx
            .conn_ref()
            .query_row(
                "SELECT key_status FROM projection_signing_keys WHERE key_id = ?1",
                params![key_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| "active".to_string());
        tx.conn_ref()
            .execute(
                "UPDATE projection_signing_keys
                    SET key_status = 'revoked', revoked_at = ?2, replacement_key_id = ?3
                  WHERE key_id = ?1",
                params![key_id, now, &replacement.key_id],
            )
            .map_err(|error| error.to_string())?;
        insert_key_status_event(ctx, tx, key_id, Some(&previous), "revoked", reason, &now)
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE projection_signing_keys SET key_status = 'active' WHERE key_id = ?1",
                params![&replacement.key_id],
            )
            .map_err(|error| error.to_string())?;
        insert_key_status_event(ctx, tx, &replacement.key_id, Some("rotating"), "active", reason, &now)
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "INSERT INTO projection_replacement_keys
                    (replacement_id, old_key_id, new_key_id, reason, provisioned_at, activated_at,
                     completed_at, recovery_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, 'pending')",
                params![format!("repl_{}", Uuid::new_v4().simple()), key_id, &replacement.key_id, reason, now],
            )
            .map_err(|error| error.to_string())?;
        queue_resign_for_key(ctx, tx, key_id, &replacement.key_id, reason, &now)
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;
    Ok(replacement.key_id)
}

fn ensure_active_signing_key(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
) -> Result<ProjectionSigningKey, ProjectionSigningError> {
    if let Some(key) = load_active_signing_key(db)? {
        return Ok(key);
    }
    create_signing_key(ctx, db, key_store, ProjectionKeyStatus::Active, "initial_projection_key")
}

fn create_signing_key(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    status: ProjectionKeyStatus,
    reason: &str,
) -> Result<ProjectionSigningKey, ProjectionSigningError> {
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|error| ProjectionSigningError::Crypto(format!("key generation failed: {error}")))?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
        .map_err(|error| ProjectionSigningError::Crypto(format!("generated key rejected: {error}")))?;
    let key_id = format!("psk_{}", Uuid::new_v4().simple());
    let account_ref = format!("projection-signing/{key_id}");
    key_store.put_private_key(&account_ref, pkcs8.as_ref())?;
    let public_key_b64 = URL_SAFE_NO_PAD.encode(key_pair.public_key().as_ref());
    let now = timestamp(ctx);
    let status_str = status.as_str();

    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "INSERT INTO projection_signing_keys
                    (key_id, public_key_b64, key_status, created_at, valid_from, valid_until,
                     retired_at, revoked_at, replacement_key_id, keychain_service, keychain_account_ref)
                 VALUES (?1, ?2, ?3, ?4, ?4, NULL, NULL, NULL, NULL, ?5, ?6)",
                params![&key_id, &public_key_b64, status_str, &now, PROJECTION_KEYCHAIN_SERVICE, &account_ref],
            )
            .map_err(|error| error.to_string())?;
        insert_key_status_event(ctx, tx, &key_id, None, status_str, reason, &now)
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;

    load_signing_key(db, &key_id)?.ok_or(ProjectionSigningError::ProjectionNotFound(key_id))
}

fn load_active_signing_key(db: &ActionDb) -> Result<Option<ProjectionSigningKey>, ProjectionSigningError> {
    db.conn_ref()
        .query_row(
            "SELECT key_id, public_key_b64, key_status, keychain_account_ref,
                    valid_from, valid_until, retired_at, revoked_at, replacement_key_id
               FROM projection_signing_keys
              WHERE key_status = 'active'
              ORDER BY created_at DESC
              LIMIT 1",
            [],
            map_signing_key,
        )
        .optional()
        .map_err(ProjectionSigningError::from)
}

fn load_signing_key(
    db: &ActionDb,
    key_id: &str,
) -> Result<Option<ProjectionSigningKey>, ProjectionSigningError> {
    db.conn_ref()
        .query_row(
            "SELECT key_id, public_key_b64, key_status, keychain_account_ref,
                    valid_from, valid_until, retired_at, revoked_at, replacement_key_id
               FROM projection_signing_keys
              WHERE key_id = ?1",
            params![key_id],
            map_signing_key,
        )
        .optional()
        .map_err(ProjectionSigningError::from)
}

fn map_signing_key(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectionSigningKey> {
    let status: String = row.get("key_status")?;
    Ok(ProjectionSigningKey {
        key_id: row.get("key_id")?,
        public_key_b64: row.get("public_key_b64")?,
        key_status: ProjectionKeyStatus::from_str(&status),
        keychain_account_ref: row.get("keychain_account_ref")?,
        valid_from: row.get("valid_from")?,
        valid_until: row.get("valid_until")?,
        retired_at: row.get("retired_at")?,
        revoked_at: row.get("revoked_at")?,
        replacement_key_id: row.get("replacement_key_id")?,
    })
}

#[allow(clippy::too_many_arguments)]
fn upsert_ledger_and_signature(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    input: &ProjectionWriteInput,
    payload: &SignedProjectionPayload,
    envelope: &ProjectionSignatureEnvelope,
    canonical_bytes: &[u8],
    canonical_hash: &str,
    claim_watermark: &str,
    signature_bytes: &[u8],
    envelope_b64url: &str,
    signed_at: &str,
) -> Result<(), ProjectionSigningError> {
    let conn = tx.conn_ref();
    let old_active_signature = conn
        .query_row(
            "SELECT signature_id
               FROM projection_signatures
              WHERE projection_id = ?1 AND signature_status = 'active'
              LIMIT 1",
            params![input.projection_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    conn.execute(
        "INSERT INTO projection_ledger
            (projection_id, surface, surface_locator, surface_locator_hash, locator_status,
             dailyos_canonical_id, dailyos_source_runtime, dailyos_projection_version,
             composition_id, composition_version, current_signature_id,
             canonical_signed_payload_sha256, claim_watermark_sha256, last_verified_at,
             verification_status, quarantine_state, last_quarantine_event_at,
             quarantine_event_count)
         VALUES (?1, ?2, ?3, ?4, 'live', ?5, ?6, ?7, ?8, ?9, NULL, ?10, ?11, ?12,
                 'verified', 'none', NULL, 0)
         ON CONFLICT(projection_id) DO UPDATE SET
             surface = excluded.surface,
             surface_locator = excluded.surface_locator,
             surface_locator_hash = excluded.surface_locator_hash,
             locator_status = 'live',
             dailyos_canonical_id = excluded.dailyos_canonical_id,
             dailyos_source_runtime = excluded.dailyos_source_runtime,
             dailyos_projection_version = excluded.dailyos_projection_version,
             composition_id = excluded.composition_id,
             composition_version = excluded.composition_version,
             canonical_signed_payload_sha256 = excluded.canonical_signed_payload_sha256,
             claim_watermark_sha256 = excluded.claim_watermark_sha256,
             last_verified_at = excluded.last_verified_at,
             verification_status = 'verified',
             quarantine_state = CASE
                 WHEN projection_ledger.quarantine_state = 'quarantined' THEN projection_ledger.quarantine_state
                 ELSE 'none'
             END",
        params![
            input.projection_id,
            input.surface.as_str(),
            input.surface_locator,
            payload.surface_locator_hash,
            input.dailyos_canonical_id,
            input.dailyos_source_runtime,
            input.dailyos_projection_version,
            input.composition_id,
            input.composition_version,
            canonical_hash,
            claim_watermark,
            signed_at,
        ],
    )?;

    conn.execute(
        "UPDATE projection_signatures
            SET signature_status = 'superseded'
          WHERE projection_id = ?1 AND signature_status = 'active'",
        params![input.projection_id],
    )?;
    conn.execute(
        "INSERT INTO projection_signatures
            (signature_id, projection_id, key_id, signature_status, alg, canonicalization,
             canonical_signed_payload_bytes, canonical_signed_payload_sha256, signature_bytes,
             signature_envelope_b64url, issued_at, superseded_by_signature_id, revoked_at, retired_at)
         VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, NULL)",
        params![
            envelope.signature_id,
            input.projection_id,
            envelope.key_id,
            PROJECTION_SIGNATURE_ALG,
            PROJECTION_CANONICALIZATION,
            canonical_bytes,
            canonical_hash,
            signature_bytes,
            envelope_b64url,
            signed_at,
        ],
    )?;
    if let Some(old_signature_id) = old_active_signature {
        conn.execute(
            "UPDATE projection_signatures
                SET superseded_by_signature_id = ?2
              WHERE signature_id = ?1",
            params![old_signature_id, envelope.signature_id],
        )?;
    }
    conn.execute(
        "UPDATE projection_ledger SET current_signature_id = ?2 WHERE projection_id = ?1",
        params![input.projection_id, envelope.signature_id],
    )?;

    conn.execute(
        "DELETE FROM projection_ledger_block_refs WHERE projection_id = ?1",
        params![input.projection_id],
    )?;
    conn.execute(
        "DELETE FROM projection_ledger_blocks WHERE projection_id = ?1",
        params![input.projection_id],
    )?;
    for block in &payload.blocks {
        conn.execute(
            "INSERT INTO projection_ledger_blocks
                (projection_id, block_id, block_order, block_type, block_payload_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.projection_id,
                block.block_id,
                block.block_order,
                block.block_type,
                block.block_payload_sha256,
            ],
        )?;
        for (index, claim_ref) in block.claim_refs.iter().enumerate() {
            conn.execute(
                "INSERT INTO projection_ledger_block_refs
                    (projection_id, block_id, claim_ref_index, claim_id, claim_version,
                     field_path, provenance_invocation_id, provenance_field_path, scope_grant_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    input.projection_id,
                    block.block_id,
                    index as u64,
                    claim_ref.claim_id,
                    claim_ref.claim_version,
                    claim_ref.field_path,
                    claim_ref.provenance_invocation_id,
                    claim_ref.provenance_field_path,
                    claim_ref.scope_grant_hash,
                ],
            )?;
        }
    }

    crate::services::signals::emit_in_transaction(
        ctx,
        tx,
        "projection",
        &input.projection_id,
        "projection_signature_issued",
        "projection_signing",
        json!({
            "signature_id": envelope.signature_id,
            "key_id": envelope.key_id,
            "composition_id": input.composition_id,
            "composition_version": input.composition_version,
            "claim_watermark_sha256": claim_watermark,
        }),
    )
    .map_err(ProjectionSigningError::Service)?;
    Ok(())
}

fn quarantine_tamper(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &ProjectionVerificationInput,
    envelope: Option<&ProjectionSignatureEnvelope>,
    status: ProjectionVerificationStatus,
) -> Result<ProjectionVerificationOutcome, ProjectionSigningError> {
    let quarantine_id = record_quarantine(ctx, db, input, envelope, status.clone())?;
    let bridge_fields = ProjectionTamperBridgeFields {
        projection_id: input.projection_id.clone(),
        signature_id: envelope
            .map(|e| e.signature_id.clone())
            .unwrap_or_else(|| "missing".to_string()),
        key_id: envelope
            .map(|e| e.key_id.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        observed_signature_status: status.as_str().to_string(),
        quarantine_id: quarantine_id.clone(),
    };
    Ok(ProjectionVerificationOutcome {
        status,
        projection_id: input.projection_id.clone(),
        quarantine_id: Some(quarantine_id),
        failure: Some(ProjectionVerificationFailure::Tampered(bridge_fields)),
    })
}

fn record_quarantine(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &ProjectionVerificationInput,
    envelope: Option<&ProjectionSignatureEnvelope>,
    status: ProjectionVerificationStatus,
) -> Result<String, ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let canonical_payload = canonical_json_bytes(&input.payload)?;
    let observed_payload_hash = sha256_hex(&canonical_payload);
    let surface_locator_hash = sha256_hex(input.surface_locator.as_bytes());
    let observed_signature = envelope.map(|e| e.signature_b64url.clone());
    let expected_signature_id = db
        .conn_ref()
        .query_row(
            "SELECT current_signature_id FROM projection_ledger WHERE projection_id = ?1",
            params![input.projection_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?;
    let expected_signature_id = expected_signature_id.flatten();
    let now = timestamp(ctx);
    let coalesced_until = (ctx.clock.now() + Duration::seconds(60))
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let claim_ids = claim_ids_from_payload(&input.payload);
    let projection_id = input.projection_id.clone();
    let verification_error = status.as_str().to_string();

    db.with_transaction(|tx| {
        let existing_id = tx
            .conn_ref()
            .query_row(
                "SELECT quarantine_id
                   FROM projection_quarantine
                  WHERE projection_id = ?1 AND status = 'open' AND coalesced_until >= ?2
                  ORDER BY detected_at DESC
                  LIMIT 1",
                params![projection_id, now],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let quarantine_id = if let Some(existing_id) = existing_id {
            tx.conn_ref()
                .execute(
                    "UPDATE projection_quarantine
                        SET last_seen_at = ?2,
                            seen_count = seen_count + 1,
                            observed_payload_hash = ?3,
                            observed_signature_b64 = ?4,
                            verification_error = ?5,
                            coalesced_until = ?6
                      WHERE quarantine_id = ?1",
                    params![
                        existing_id,
                        now,
                        observed_payload_hash,
                        observed_signature,
                        verification_error,
                        coalesced_until,
                    ],
                )
                .map_err(|error| error.to_string())?;
            existing_id
        } else {
            let quarantine_id = format!("pq_{}", Uuid::new_v4().simple());
            tx.conn_ref()
                .execute(
                    "INSERT INTO projection_quarantine
                        (quarantine_id, projection_id, surface, surface_locator_hash,
                         observed_payload_hash, observed_signature_b64, expected_signature_id,
                         verification_error, field_pointer, byte_range_start, byte_range_end,
                         sanitized_observed_excerpt_hash, detected_by, detected_at, last_seen_at,
                         seen_count, coalesced_until, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, NULL,
                             'projection_signing', ?9, ?9, 1, ?10, 'open')",
                    params![
                        quarantine_id,
                        projection_id,
                        input.surface.as_str(),
                        surface_locator_hash,
                        observed_payload_hash,
                        observed_signature,
                        expected_signature_id,
                        verification_error,
                        now,
                        coalesced_until,
                    ],
                )
                .map_err(|error| error.to_string())?;
            quarantine_id
        };

        tx.conn_ref()
            .execute(
                "UPDATE projection_ledger
                    SET verification_status = ?2,
                        quarantine_state = 'quarantined',
                        last_quarantine_event_at = ?3,
                        quarantine_event_count = quarantine_event_count + 1
                  WHERE projection_id = ?1",
                params![projection_id, status.as_str(), now],
            )
            .map_err(|error| error.to_string())?;

        crate::services::signals::emit_in_transaction(
            ctx,
            tx,
            "projection",
            &projection_id,
            PROJECTION_SIGNATURE_INVALID_SIGNAL,
            "projection_signing",
            json!({
                "projection_id": projection_id,
                "quarantine_id": quarantine_id,
                "verification_error": status.as_str(),
            }),
        )
        .map_err(|error| error.to_string())?;
        for claim_id in &claim_ids {
            crate::services::signals::emit_in_transaction(
                ctx,
                tx,
                "claim",
                claim_id,
                PROJECTION_SIGNATURE_INVALID_SIGNAL,
                "projection_signing",
                json!({
                    "projection_id": projection_id,
                    "quarantine_id": quarantine_id,
                    "verification_error": status.as_str(),
                }),
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(quarantine_id)
    })
    .map_err(ProjectionSigningError::Database)
}

fn detect_currentness_rollback(
    db: &ActionDb,
    input: &ProjectionVerificationInput,
    envelope: &ProjectionSignatureEnvelope,
    canonical_bytes: &[u8],
) -> Result<Option<ProjectionRollbackBridgeFields>, ProjectionSigningError> {
    let Some(current) = load_ledger_currentness(db, &input.projection_id)? else {
        return Ok(Some(ProjectionRollbackBridgeFields {
            projection_id: input.projection_id.clone(),
            signed_composition_version: input.payload.composition_version,
            ledger_composition_version: 0,
            signed_claim_version: max_payload_claim_version(&input.payload),
            ledger_claim_version: None,
        }));
    };

    let canonical_hash = sha256_hex(canonical_bytes);
    let claim_watermark = claim_watermark_hash(&input.payload)?;
    let current_signature_mismatch = current.current_signature_id.as_deref() != Some(&envelope.signature_id)
        || current.key_id.as_deref() != Some(&envelope.key_id)
        || current.signature_status.as_deref() != Some("active");
    let composition_rollback = input.payload.composition_version < current.composition_version;
    let payload_hash_mismatch = canonical_hash != current.canonical_signed_payload_sha256
        || claim_watermark != current.claim_watermark_sha256;
    let claim_rollback = input
        .payload
        .blocks
        .iter()
        .flat_map(|block| block.claim_refs.iter())
        .find_map(|claim_ref| {
            current.claim_versions.get(&claim_ref.claim_id).and_then(|ledger_version| {
                (claim_ref.claim_version < *ledger_version).then_some((
                    claim_ref.claim_version,
                    *ledger_version,
                ))
            })
        });

    if current_signature_mismatch || composition_rollback || payload_hash_mismatch || claim_rollback.is_some() {
        let (signed_claim_version, ledger_claim_version) = claim_rollback
            .map(|(signed, ledger)| (Some(signed), Some(ledger)))
            .unwrap_or_else(|| (max_payload_claim_version(&input.payload), max_ledger_claim_version(&current)));
        return Ok(Some(ProjectionRollbackBridgeFields {
            projection_id: input.projection_id.clone(),
            signed_composition_version: input.payload.composition_version,
            ledger_composition_version: current.composition_version,
            signed_claim_version,
            ledger_claim_version,
        }));
    }
    Ok(None)
}

fn load_ledger_currentness(
    db: &ActionDb,
    projection_id: &str,
) -> Result<Option<LedgerCurrentness>, ProjectionSigningError> {
    let current = db
        .conn_ref()
        .query_row(
            "SELECT l.current_signature_id, l.composition_version,
                    l.canonical_signed_payload_sha256, l.claim_watermark_sha256,
                    s.key_id, s.signature_status
               FROM projection_ledger l
               LEFT JOIN projection_signatures s ON s.signature_id = l.current_signature_id
              WHERE l.projection_id = ?1",
            params![projection_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, u64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((current_signature_id, composition_version, canonical_hash, claim_watermark, key_id, signature_status)) = current else {
        return Ok(None);
    };
    let mut stmt = db.conn_ref().prepare(
        "SELECT claim_id, MAX(claim_version)
           FROM projection_ledger_block_refs
          WHERE projection_id = ?1
          GROUP BY claim_id",
    )?;
    let claim_versions = stmt
        .query_map(params![projection_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)))?
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(Some(LedgerCurrentness {
        current_signature_id,
        key_id,
        signature_status,
        composition_version,
        claim_versions,
        canonical_signed_payload_sha256: canonical_hash,
        claim_watermark_sha256: claim_watermark,
    }))
}

fn signed_payload_from_input(input: &ProjectionWriteInput) -> Result<SignedProjectionPayload, ProjectionSigningError> {
    let mut blocks = input
        .blocks
        .iter()
        .map(|block| {
            let canonical_payload = canonical_json_bytes(&block.block_payload)?;
            Ok(SignedProjectionBlock {
                block_id: block.block_id.clone(),
                block_order: block.block_order,
                block_type: block.block_type.clone(),
                block_payload: canonicalize_json_value(block.block_payload.clone()),
                block_payload_sha256: sha256_hex(&canonical_payload),
                claim_refs: block
                    .claim_refs
                    .iter()
                    .map(|claim_ref| SignedProjectionClaimRef {
                        claim_id: claim_ref.claim_id.clone(),
                        claim_version: claim_ref.claim_version,
                        field_path: claim_ref.field_path.clone(),
                        provenance_invocation_id: claim_ref.provenance_invocation_id.clone(),
                        provenance_field_path: claim_ref.provenance_field_path.clone(),
                        scope_grant_hash: claim_ref.scope_grant_hash.clone(),
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, ProjectionSigningError>>()?;
    blocks.sort_by_key(|block| block.block_order);
    Ok(SignedProjectionPayload {
        domain_separator: input.surface.domain_separator().to_string(),
        projection_id: input.projection_id.clone(),
        surface: input.surface,
        surface_locator_hash: sha256_hex(input.surface_locator.as_bytes()),
        dailyos_canonical_id: input.dailyos_canonical_id.clone(),
        dailyos_source_runtime: input.dailyos_source_runtime.clone(),
        dailyos_projection_version: input.dailyos_projection_version,
        composition_id: input.composition_id.clone(),
        composition_version: input.composition_version,
        blocks,
    })
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ProjectionSigningError> {
    let value = serde_json::to_value(value)?;
    let canonical = canonicalize_json_value(value);
    serde_json::to_vec(&canonical).map_err(ProjectionSigningError::from)
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize_json_value).collect()),
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(value) = map.get(&key) {
                    sorted.insert(key, canonicalize_json_value(value.clone()));
                }
            }
            Value::Object(sorted)
        }
        other => other,
    }
}

fn decode_signature_envelope(
    envelope_b64url: &str,
) -> Result<ProjectionSignatureEnvelope, ProjectionVerificationStatus> {
    let bytes = URL_SAFE_NO_PAD
        .decode(envelope_b64url.as_bytes())
        .map_err(|_| ProjectionVerificationStatus::MissingSignature)?;
    serde_json::from_slice(&bytes).map_err(|_| ProjectionVerificationStatus::MissingSignature)
}

fn claim_watermark_hash(payload: &SignedProjectionPayload) -> Result<String, ProjectionSigningError> {
    let refs = payload
        .blocks
        .iter()
        .flat_map(|block| {
            block.claim_refs.iter().map(|claim_ref| {
                json!({
                    "block_id": block.block_id,
                    "claim_id": claim_ref.claim_id,
                    "claim_version": claim_ref.claim_version,
                    "field_path": claim_ref.field_path,
                    "provenance_invocation_id": claim_ref.provenance_invocation_id,
                    "provenance_field_path": claim_ref.provenance_field_path,
                    "scope_grant_hash": claim_ref.scope_grant_hash,
                })
            })
        })
        .collect::<Vec<_>>();
    Ok(sha256_hex(&canonical_json_bytes(&refs)?))
}

fn claim_ids_from_payload(payload: &SignedProjectionPayload) -> Vec<String> {
    payload
        .blocks
        .iter()
        .flat_map(|block| block.claim_refs.iter().map(|claim_ref| claim_ref.claim_id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn max_payload_claim_version(payload: &SignedProjectionPayload) -> Option<u64> {
    payload
        .blocks
        .iter()
        .flat_map(|block| block.claim_refs.iter().map(|claim_ref| claim_ref.claim_version))
        .max()
}

fn max_ledger_claim_version(current: &LedgerCurrentness) -> Option<u64> {
    current.claim_versions.values().copied().max()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

fn timestamp(ctx: &ServiceContext<'_>) -> String {
    ctx.clock.now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn insert_key_status_event(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    key_id: &str,
    previous_status: Option<&str>,
    next_status: &str,
    reason: &str,
    created_at: &str,
) -> Result<(), ProjectionSigningError> {
    tx.conn_ref().execute(
        "INSERT INTO projection_key_status_events
            (event_id, key_id, previous_status, next_status, reason, created_at, actor_kind)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("pkse_{}", Uuid::new_v4().simple()),
            key_id,
            previous_status,
            next_status,
            reason,
            created_at,
            actor_kind(ctx.actor),
        ],
    )?;
    Ok(())
}

fn actor_kind(actor: &str) -> &'static str {
    if actor.contains("surface") {
        "surface_client"
    } else if actor.starts_with("user") {
        "user"
    } else if actor.starts_with("agent") {
        "agent"
    } else if actor.starts_with("admin") {
        "admin"
    } else {
        "system"
    }
}

fn queue_resign_for_key(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    old_key_id: &str,
    new_key_id: &str,
    reason: &str,
    now: &str,
) -> Result<(), ProjectionSigningError> {
    let mut stmt = tx.conn_ref().prepare(
        "SELECT l.projection_id, l.current_signature_id
           FROM projection_ledger l
           JOIN projection_signatures s ON s.signature_id = l.current_signature_id
          WHERE s.key_id = ?1 AND l.locator_status = 'live'",
    )?;
    let rows = stmt
        .query_map(params![old_key_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (projection_id, old_signature_id) in rows {
        tx.conn_ref().execute(
            "INSERT OR IGNORE INTO projection_resign_queue
                (queue_id, projection_id, old_signature_id, old_key_id, new_key_id, reason,
                 status, attempts, max_attempts, last_error, last_resign_at, last_retampered_at,
                 operator_escalated_at, queued_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, 3, NULL, NULL, NULL, NULL, ?7, ?7, NULL)",
            params![
                format!("prq_{}", Uuid::new_v4().simple()),
                projection_id,
                old_signature_id,
                old_key_id,
                new_key_id,
                reason,
                now,
            ],
        )?;
        crate::services::signals::emit_in_transaction(
            ctx,
            tx,
            "projection",
            &projection_id,
            "projection_resign_queued",
            "projection_signing",
            json!({
                "old_key_id": old_key_id,
                "new_key_id": new_key_id,
                "reason": reason,
            }),
        )
        .map_err(ProjectionSigningError::Service)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use chrono::{TimeZone, Utc};
    use parking_lot::Mutex;

    #[derive(Default)]
    struct InMemoryProjectionKeyStore {
        keys: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl ProjectionKeyStore for InMemoryProjectionKeyStore {
        fn get_private_key(
            &self,
            account_ref: &str,
        ) -> Result<Zeroizing<Vec<u8>>, ProjectionSigningError> {
            let keys = self.keys.lock();
            keys.get(account_ref)
                .cloned()
                .map(Zeroizing::new)
                .ok_or_else(|| ProjectionSigningError::Keychain("missing test key".to_string()))
        }

        fn put_private_key(
            &self,
            account_ref: &str,
            pkcs8: &[u8],
        ) -> Result<(), ProjectionSigningError> {
            self.keys.lock().insert(account_ref.to_string(), pkcs8.to_vec());
            Ok(())
        }
    }

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext).with_actor("agent:test")
    }

    fn fixture_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap())
    }

    fn projection_input() -> ProjectionWriteInput {
        ProjectionWriteInput {
            projection_id: "proj-account-overview-1".to_string(),
            surface: ProjectionSurface::WordpressDb,
            surface_locator: "wp:post:42:block:account-overview".to_string(),
            dailyos_canonical_id: "composition:account-overview:acct-1".to_string(),
            dailyos_source_runtime: "dailyos-runtime-test".to_string(),
            dailyos_projection_version: 1,
            composition_id: "comp-account-overview-1".to_string(),
            composition_version: 7,
            blocks: vec![
                ProjectionBlockInput {
                    block_id: "block-summary".to_string(),
                    block_order: 0,
                    block_type: "dailyos/account-summary".to_string(),
                    block_payload: json!({"b": 2, "a": 1}),
                    claim_refs: vec![ProjectionClaimRefInput {
                        claim_id: "claim-renewal-risk".to_string(),
                        claim_version: 3,
                        field_path: Some("/summary".to_string()),
                        provenance_invocation_id: Some("invocation-1".to_string()),
                        provenance_field_path: Some("/output/summary".to_string()),
                        scope_grant_hash: Some("scope-hash-1".to_string()),
                    }],
                },
                ProjectionBlockInput {
                    block_id: "block-timeline".to_string(),
                    block_order: 1,
                    block_type: "dailyos/account-timeline".to_string(),
                    block_payload: json!([{"event": "generic renewal"}]),
                    claim_refs: vec![ProjectionClaimRefInput {
                        claim_id: "claim-renewal-date".to_string(),
                        claim_version: 2,
                        field_path: Some("/timeline/0".to_string()),
                        provenance_invocation_id: Some("invocation-2".to_string()),
                        provenance_field_path: Some("/output/timeline/0".to_string()),
                        scope_grant_hash: Some("scope-hash-2".to_string()),
                    }],
                },
            ],
        }
    }

    fn projection_input_with_currentness(
        composition_version: u64,
        first_claim_version: u64,
    ) -> ProjectionWriteInput {
        let base = projection_input();
        let first_block = base.blocks[0].clone();
        let second_block = base.blocks[1].clone();

        ProjectionWriteInput {
            composition_version,
            blocks: vec![
                ProjectionBlockInput {
                    claim_refs: vec![ProjectionClaimRefInput {
                        claim_version: first_claim_version,
                        ..first_block.claim_refs[0].clone()
                    }],
                    ..first_block
                },
                second_block,
            ],
            ..base
        }
    }

    #[test]
    fn dos569_fixture_nominal_wp_projection_signs_and_verifies() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: signed.payload.projection_id.clone(),
                surface: signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: Some("dailyos-runtime-test".to_string()),
                payload: signed.payload.clone(),
                signature_envelope_b64url: Some(signed.signature_envelope_b64url.clone()),
            },
        )
        .expect("verify projection");

        assert_eq!(outcome.status, ProjectionVerificationStatus::Verified);
        let ledger_status: String = db
            .conn_ref()
            .query_row(
                "SELECT verification_status FROM projection_ledger WHERE projection_id = ?1",
                params![signed.payload.projection_id],
                |row| row.get(0),
            )
            .expect("ledger row");
        assert_eq!(ledger_status, "verified");
        assert_ne!(signed.signature_envelope_b64url, signed.canonical_signed_payload_sha256);
    }

    #[test]
    fn dos569_fixture_block_ordering_mutation_is_quarantined() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let mut tampered_payload = signed.payload.clone();
        tampered_payload.blocks.swap(0, 1);

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: tampered_payload.projection_id.clone(),
                surface: tampered_payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: tampered_payload,
                signature_envelope_b64url: Some(signed.signature_envelope_b64url.clone()),
            },
        )
        .expect("tamper outcome");

        assert_eq!(outcome.status, ProjectionVerificationStatus::SignatureInvalid);
        assert!(outcome.quarantine_id.is_some());
        assert!(matches!(outcome.failure, Some(ProjectionVerificationFailure::Tampered(_))));
    }

    #[test]
    fn dos569_fixture_unsupported_alg_is_quarantined() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let mut envelope = signed.signature_envelope.clone();
        envelope.alg = "HMAC-SHA256".to_string();
        let envelope_b64 = URL_SAFE_NO_PAD.encode(canonical_json_bytes(&envelope).unwrap());

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: signed.payload.projection_id.clone(),
                surface: signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: signed.payload,
                signature_envelope_b64url: Some(envelope_b64),
            },
        )
        .expect("unsupported alg outcome");

        assert_eq!(outcome.status, ProjectionVerificationStatus::UnsupportedAlgorithm);
        let seen_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM projection_quarantine", [], |row| row.get(0))
            .expect("quarantine count");
        assert_eq!(seen_count, 1);
    }

    #[test]
    fn dos569_fixture_quarantine_coalesces_for_sixty_seconds() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let mut tampered_payload = signed.payload.clone();
        tampered_payload.blocks[0].block_payload = json!({"a": 99});

        for _ in 0..2 {
            verify_projection_read(
                &ctx,
                &db,
                ProjectionVerificationInput {
                    projection_id: tampered_payload.projection_id.clone(),
                    surface: tampered_payload.surface,
                    surface_locator: "wp:post:42:block:account-overview".to_string(),
                    expected_runtime_anchor_id: None,
                    payload: tampered_payload.clone(),
                    signature_envelope_b64url: Some(signed.signature_envelope_b64url.clone()),
                },
            )
            .expect("tamper outcome");
        }

        let (rows, seen_count): (i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*), MAX(seen_count) FROM projection_quarantine",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("coalesced quarantine");
        assert_eq!(rows, 1);
        assert_eq!(seen_count, 2);
    }

    #[test]
    fn dos569_fixture_current_signature_required() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let old_signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign old");
        let next = projection_input_with_currentness(8, 4);
        let _new_signed = sign_projection(&ctx, &db, &key_store, next).expect("sign new");

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: old_signed.payload.projection_id.clone(),
                surface: old_signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: old_signed.payload,
                signature_envelope_b64url: Some(old_signed.signature_envelope_b64url),
            },
        )
        .expect("rollback outcome");

        assert_eq!(outcome.status, ProjectionVerificationStatus::VersionRollback);
        assert!(matches!(outcome.failure, Some(ProjectionVerificationFailure::VersionRollback(_))));
    }

    #[test]
    fn dos569_fixture_key_revoked_queues_replacement_resign() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed = sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let old_key_id = signed.signature_envelope.key_id.clone();
        let replacement_key_id = revoke_projection_signing_key(
            &ctx,
            &db,
            &key_store,
            &old_key_id,
            "test_key_compromise",
        )
        .expect("revoke key");

        assert_ne!(replacement_key_id, old_key_id);
        let queue_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM projection_resign_queue", [], |row| row.get(0))
            .expect("queue count");
        assert_eq!(queue_count, 1);

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: signed.payload.projection_id.clone(),
                surface: signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: signed.payload,
                signature_envelope_b64url: Some(signed.signature_envelope_b64url),
            },
        )
        .expect("revoked outcome");
        assert_eq!(outcome.status, ProjectionVerificationStatus::RevokedKey);
    }

    #[test]
    fn dos569_fixture_domain_separator_changes_payload_hash() {
        let mut wp = projection_input();
        let mut md = projection_input();
        md.surface = ProjectionSurface::MarkdownFile;
        let wp_payload = signed_payload_from_input(&wp).expect("wp payload");
        let md_payload = signed_payload_from_input(&md).expect("md payload");
        assert_ne!(wp_payload.domain_separator, md_payload.domain_separator);
        assert_ne!(
            sha256_hex(&canonical_json_bytes(&wp_payload).unwrap()),
            sha256_hex(&canonical_json_bytes(&md_payload).unwrap())
        );
        wp.surface = ProjectionSurface::WordpressDb;
    }

    #[test]
    fn dos569_property_canonicalization_sorts_objects_but_preserves_array_order() {
        let a = json!({"b": 2, "a": [1, 2]});
        let b = json!({"a": [1, 2], "b": 2});
        let c = json!({"a": [2, 1], "b": 2});
        assert_eq!(canonical_json_bytes(&a).unwrap(), canonical_json_bytes(&b).unwrap());
        assert_ne!(canonical_json_bytes(&a).unwrap(), canonical_json_bytes(&c).unwrap());
    }
}
