use std::collections::{BTreeSet, HashMap};
use std::io::Write;
use std::process::{Command, Stdio};

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
pub const PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL: &str = "signature_verified_retired_key";

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
    fn get_private_key(
        &self,
        account_ref: &str,
    ) -> Result<Zeroizing<Vec<u8>>, ProjectionSigningError>;
    fn put_private_key(
        &self,
        account_ref: &str,
        pkcs8: &[u8],
    ) -> Result<(), ProjectionSigningError>;
}

#[derive(Debug, Default)]
pub struct MacOsProjectionKeyStore;

impl ProjectionKeyStore for MacOsProjectionKeyStore {
    fn get_private_key(
        &self,
        account_ref: &str,
    ) -> Result<Zeroizing<Vec<u8>>, ProjectionSigningError> {
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

    fn put_private_key(
        &self,
        account_ref: &str,
        pkcs8: &[u8],
    ) -> Result<(), ProjectionSigningError> {
        let encoded = Zeroizing::new(URL_SAFE_NO_PAD.encode(pkcs8));
        let runtime_binary = runtime_binary_acl_path()?;
        let mut child = Command::new("security")
            .arg("add-generic-password")
            .arg("-s")
            .arg(PROJECTION_KEYCHAIN_SERVICE)
            .arg("-a")
            .arg(account_ref)
            .arg("-U")
            .arg("-T")
            .arg(runtime_binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                ProjectionSigningError::Keychain(
                    "projection signing key write failed: stdin unavailable".to_string(),
                )
            })?;
            stdin
                .write_all(encoded.as_bytes())
                .and_then(|()| stdin.write_all(b"\n"))
                .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?;
        }

        let output = child
            .wait_with_output()
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

fn runtime_binary_acl_path() -> Result<String, ProjectionSigningError> {
    std::env::current_exe()
        .map_err(|error| ProjectionSigningError::Keychain(error.to_string()))?
        .into_os_string()
        .into_string()
        .map_err(|_| {
            ProjectionSigningError::Keychain("runtime binary path is not UTF-8".to_string())
        })
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
    VerifiedRetired,
    MissingSignature,
    MalformedEnvelope,
    UnsupportedAlgorithm,
    UnsupportedCanonicalization,
    UnknownKey,
    RevokedKey,
    SignatureInvalid,
    VersionRollback,
    WrongRuntimeAnchor,
    Tombstoned,
}

impl ProjectionVerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::VerifiedRetired => "verified_retired",
            Self::MissingSignature => "missing_signature",
            Self::MalformedEnvelope => "malformed_envelope",
            Self::UnsupportedAlgorithm => "unsupported_algorithm",
            Self::UnsupportedCanonicalization => "unsupported_canonicalization",
            Self::UnknownKey => "unknown_key",
            Self::RevokedKey => "revoked_key",
            Self::SignatureInvalid => "signature_invalid",
            Self::VersionRollback => "rollback",
            Self::WrongRuntimeAnchor => "wrong_runtime_anchor",
            Self::Tombstoned => "tombstoned",
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
    pub key_status: Option<ProjectionKeyStatus>,
}

#[derive(Debug, Clone)]
pub struct ProjectionVerificationInput {
    pub projection_id: String,
    pub surface: ProjectionSurface,
    pub surface_locator: String,
    pub expected_runtime_anchor_id: Option<String>,
    pub payload: SignedProjectionPayload,
    pub signature_envelope_b64url: Option<String>,
    pub observed_payload_bytes: Option<Vec<u8>>,
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
    locator_status: String,
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
    verify_projection_read_internal::<fn() -> Result<(), ProjectionSigningError>>(
        ctx, db, input, None,
    )
}

pub fn verify_with_unknown_key_refresh<F>(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ProjectionVerificationInput,
    refresh_once: F,
) -> Result<ProjectionVerificationOutcome, ProjectionSigningError>
where
    F: FnOnce() -> Result<(), ProjectionSigningError>,
{
    verify_projection_read_internal(ctx, db, input, Some(refresh_once))
}

fn verify_projection_read_internal<F>(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ProjectionVerificationInput,
    mut refresh_once: Option<F>,
) -> Result<ProjectionVerificationOutcome, ProjectionSigningError>
where
    F: FnOnce() -> Result<(), ProjectionSigningError>,
{
    let Some(envelope_b64url) = input.signature_envelope_b64url.as_deref() else {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            None,
            ProjectionVerificationStatus::MissingSignature,
        );
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
        if envelope.dailyos_source_runtime != expected
            || input.payload.dailyos_source_runtime != expected
        {
            return quarantine_tamper(
                ctx,
                db,
                &input,
                Some(&envelope),
                ProjectionVerificationStatus::WrongRuntimeAnchor,
            );
        }
    }

    let key = loop {
        if let Some(key) = load_signing_key(db, &envelope.key_id)? {
            break key;
        }
        if let Some(refresh) = refresh_once.take() {
            refresh()?;
            continue;
        }
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
    if public_key
        .verify(&canonical_bytes, &signature_bytes)
        .is_err()
    {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::SignatureInvalid,
        );
    }

    if projection_locator_is_tombstoned(db, &input.projection_id)? {
        return quarantine_tamper(
            ctx,
            db,
            &input,
            Some(&envelope),
            ProjectionVerificationStatus::Tombstoned,
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
            key_status: Some(key.key_status),
        });
    }

    let verification_status = if key.key_status == ProjectionKeyStatus::Retired {
        ProjectionVerificationStatus::VerifiedRetired
    } else {
        ProjectionVerificationStatus::Verified
    };

    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE projection_ledger
                    SET last_verified_at = ?2,
                        verification_status = ?3,
                        quarantine_state = CASE
                            WHEN quarantine_state = 'quarantined' THEN quarantine_state
                            ELSE 'none'
                        END
                  WHERE projection_id = ?1",
                params![
                    &input.projection_id,
                    timestamp(ctx),
                    verification_status.as_str()
                ],
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
                "signature_id": &envelope.signature_id,
                "key_id": &envelope.key_id,
                "key_status": key.key_status.as_str(),
                "verification_status": verification_status.as_str(),
            }),
        )
        .map_err(|error| error.to_string())?;
        if verification_status == ProjectionVerificationStatus::VerifiedRetired {
            emit_projection_status_signal_in_tx(
                ctx,
                tx,
                "projection",
                &input.projection_id,
                PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL,
                json!({
                    "signature_id": &envelope.signature_id,
                    "key_id": &envelope.key_id,
                    "verification_status": verification_status.as_str(),
                }),
            )?;
            for claim_id in claim_ids_from_payload(&input.payload) {
                emit_projection_status_signal_in_tx(
                    ctx,
                    tx,
                    "claim",
                    &claim_id,
                    PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL,
                    json!({
                        "projection_id": &input.projection_id,
                        "signature_id": &envelope.signature_id,
                        "key_id": &envelope.key_id,
                        "verification_status": verification_status.as_str(),
                    }),
                )?;
            }
        }
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;

    Ok(ProjectionVerificationOutcome {
        status: verification_status,
        projection_id: input.projection_id,
        quarantine_id: None,
        failure: None,
        key_status: Some(key.key_status),
    })
}

pub fn projection_signature_enforcement_mode(
    db: &ActionDb,
) -> Result<crate::services::context::ProjectionSignatureEnforcementMode, ProjectionSigningError> {
    let mode = db
        .conn_ref()
        .query_row(
            "SELECT mode
               FROM projection_signature_enforcement_state
              WHERE state_id = 'projection_signature_enforcement'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(mode
        .as_deref()
        .and_then(crate::services::context::ProjectionSignatureEnforcementMode::parse_config)
        .unwrap_or_default())
}

pub fn set_projection_signature_enforcement_mode(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    mode: crate::services::context::ProjectionSignatureEnforcementMode,
    reason: &str,
) -> Result<(), ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let now = timestamp(ctx);
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO projection_signature_enforcement_state
                    (state_id, mode, updated_at, actor_kind)
                 VALUES ('projection_signature_enforcement', 'shadow', ?1, ?2)",
                params![&now, actor_kind(ctx.actor)],
            )
            .map_err(|error| error.to_string())?;
        let previous = tx
            .conn_ref()
            .query_row(
                "SELECT mode
                   FROM projection_signature_enforcement_state
                  WHERE state_id = 'projection_signature_enforcement'",
                [],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| error.to_string())?;
        if previous == mode.as_str() {
            return Ok(());
        }
        tx.conn_ref()
            .execute(
                "UPDATE projection_signature_enforcement_state
                    SET mode = ?1,
                        updated_at = ?2,
                        actor_kind = ?3
                  WHERE state_id = 'projection_signature_enforcement'",
                params![mode.as_str(), &now, actor_kind(ctx.actor)],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "INSERT INTO projection_enforcement_mode_events
                    (event_id, previous_mode, next_mode, reason, created_at, actor_kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("peme_{}", Uuid::new_v4().simple()),
                    previous,
                    mode.as_str(),
                    reason,
                    &now,
                    actor_kind(ctx.actor),
                ],
            )
            .map_err(|error| error.to_string())?;
        emit_projection_status_signal_in_tx(
            ctx,
            tx,
            "projection_signature_enforcement",
            "projection_signature_enforcement",
            "projection.enforcement_mode_changed",
            json!({
                "previous_mode": previous,
                "next_mode": mode.as_str(),
                "reason": reason,
            }),
        )?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)
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
                key_status: ProjectionKeyStatus::from_str(
                    row.get::<_, String>("key_status")?.as_str(),
                ),
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
        keyring_version: current_keyring_version(db)?,
        max_age_seconds: KEYRING_MAX_AGE_SECONDS,
        keys,
    })
}

fn current_keyring_version(db: &ActionDb) -> Result<u64, ProjectionSigningError> {
    let version = db
        .conn_ref()
        .query_row(
            "SELECT current_version
               FROM projection_keyring_state
              WHERE state_id = 'projection_keyring'",
            [],
            |row| row.get::<_, u64>(0),
        )
        .optional()?;
    Ok(version.unwrap_or(1))
}

fn bump_keyring_version_in_tx(tx: &ActionDb, now: &str) -> Result<u64, ProjectionSigningError> {
    tx.conn_ref().execute(
        "INSERT OR IGNORE INTO projection_keyring_state
            (state_id, current_version, updated_at)
         VALUES ('projection_keyring', 1, ?1)",
        params![now],
    )?;
    tx.conn_ref().execute(
        "UPDATE projection_keyring_state
            SET current_version = current_version + 1,
                updated_at = ?1
          WHERE state_id = 'projection_keyring'",
        params![now],
    )?;
    tx.conn_ref()
        .query_row(
            "SELECT current_version
           FROM projection_keyring_state
          WHERE state_id = 'projection_keyring'",
            [],
            |row| row.get::<_, u64>(0),
        )
        .map_err(ProjectionSigningError::from)
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
    let replacement =
        create_signing_key(ctx, db, key_store, ProjectionKeyStatus::Rotating, reason)?;
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
        insert_key_status_event(
            ctx,
            tx,
            &replacement.key_id,
            Some("rotating"),
            "active",
            reason,
            &now,
        )
        .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "INSERT INTO projection_replacement_keys
                    (replacement_id, old_key_id, new_key_id, reason, provisioned_at, activated_at,
                     completed_at, recovery_status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL, 'pending')",
                params![
                    format!("repl_{}", Uuid::new_v4().simple()),
                    key_id,
                    &replacement.key_id,
                    reason,
                    now
                ],
            )
            .map_err(|error| error.to_string())?;
        bump_keyring_version_in_tx(tx, &now).map_err(|error| error.to_string())?;
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
    reconcile_projection_signing_keychain(ctx, db, key_store)?;
    if let Some(key) = load_active_signing_key(db)? {
        return Ok(key);
    }
    create_signing_key(
        ctx,
        db,
        key_store,
        ProjectionKeyStatus::Active,
        "initial_projection_key",
    )
}

pub fn reconcile_projection_signing_keychain(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
) -> Result<usize, ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let mut stmt = db.conn_ref().prepare(
        "SELECT key_id, keychain_account_ref
           FROM projection_signing_keys
          WHERE key_status IN ('active', 'rotating')",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let missing = rows
        .into_iter()
        .filter_map(|(key_id, account_ref)| {
            key_store
                .get_private_key(&account_ref)
                .is_err()
                .then_some(key_id)
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(0);
    }
    let now = timestamp(ctx);
    db.with_transaction(|tx| {
        for key_id in &missing {
            let previous = tx
                .conn_ref()
                .query_row(
                    "SELECT key_status FROM projection_signing_keys WHERE key_id = ?1",
                    params![key_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            tx.conn_ref()
                .execute(
                    "UPDATE projection_signing_keys
                        SET key_status = 'revoked', revoked_at = ?2
                      WHERE key_id = ?1",
                    params![key_id, &now],
                )
                .map_err(|error| error.to_string())?;
            insert_key_status_event(
                ctx,
                tx,
                key_id,
                previous.as_deref(),
                "revoked",
                "keychain_missing_reconciled",
                &now,
            )
            .map_err(|error| error.to_string())?;
        }
        bump_keyring_version_in_tx(tx, &now).map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;
    Ok(missing.len())
}

fn create_signing_key(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    status: ProjectionKeyStatus,
    reason: &str,
) -> Result<ProjectionSigningKey, ProjectionSigningError> {
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|error| {
        ProjectionSigningError::Crypto(format!("key generation failed: {error}"))
    })?;
    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|error| {
        ProjectionSigningError::Crypto(format!("generated key rejected: {error}"))
    })?;
    let key_id = format!("psk_{}", Uuid::new_v4().simple());
    let account_ref = format!("projection-signing/{key_id}");
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
        bump_keyring_version_in_tx(tx, &now).map_err(|error| error.to_string())?;
        insert_key_status_event(ctx, tx, &key_id, None, status_str, reason, &now)
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)?;

    if let Err(error) = key_store.put_private_key(&account_ref, pkcs8.as_ref()) {
        let cleanup_result = db.with_transaction(|tx| {
            tx.conn_ref()
                .execute(
                    "DELETE FROM projection_key_status_events WHERE key_id = ?1",
                    params![&key_id],
                )
                .map_err(|error| error.to_string())?;
            tx.conn_ref()
                .execute(
                    "DELETE FROM projection_signing_keys WHERE key_id = ?1",
                    params![&key_id],
                )
                .map_err(|error| error.to_string())?;
            bump_keyring_version_in_tx(tx, &timestamp(ctx)).map_err(|error| error.to_string())?;
            Ok(())
        });
        if let Err(cleanup_error) = cleanup_result {
            log::warn!(
                "projection signing key db cleanup failed after keychain write failure for {}: {}",
                key_id,
                cleanup_error
            );
        }
        return Err(error);
    }

    load_signing_key(db, &key_id)?.ok_or(ProjectionSigningError::ProjectionNotFound(key_id))
}

fn load_active_signing_key(
    db: &ActionDb,
) -> Result<Option<ProjectionSigningKey>, ProjectionSigningError> {
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
            "signature_id": &envelope.signature_id,
            "key_id": &envelope.key_id,
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
        key_status: None,
    })
}

fn emit_projection_status_signal_in_tx(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    payload: Value,
) -> Result<(), String> {
    let side_effect_ctx =
        ServiceContext::new_live(ctx.clock, ctx.rng, ctx.external).with_actor(ctx.actor);
    crate::services::signals::emit_in_transaction(
        &side_effect_ctx,
        tx,
        entity_type,
        entity_id,
        signal_type,
        "projection_signing",
        payload,
    )
    .map(|_| ())
}

fn record_quarantine(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: &ProjectionVerificationInput,
    envelope: Option<&ProjectionSignatureEnvelope>,
    status: ProjectionVerificationStatus,
) -> Result<String, ProjectionSigningError> {
    let canonical_payload = canonical_json_bytes(&input.payload)?;
    let observed_payload_bytes = input
        .observed_payload_bytes
        .clone()
        .unwrap_or_else(|| canonical_payload.clone());
    let observed_payload_hash = sha256_hex(&observed_payload_bytes);
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
    let coalesced_until =
        (ctx.clock.now() + Duration::seconds(60)).to_rfc3339_opts(SecondsFormat::Secs, true);
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
                            observed_payload_bytes = ?4,
                            observed_signature_b64 = ?5,
                            verification_error = ?6,
                            coalesced_until = ?7
                      WHERE quarantine_id = ?1",
                    params![
                        existing_id,
                        now,
                        observed_payload_hash,
                        observed_payload_bytes,
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
                         observed_payload_hash, observed_payload_bytes, observed_signature_b64,
                         expected_signature_id, verification_error, field_pointer, byte_range_start,
                         byte_range_end, sanitized_observed_excerpt_hash, detected_by, detected_at,
                         last_seen_at, seen_count, coalesced_until, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL, NULL, NULL,
                             'projection_signing', ?10, ?10, 1, ?11, 'open')",
                    params![
                        quarantine_id,
                        projection_id,
                        input.surface.as_str(),
                        surface_locator_hash,
                        observed_payload_hash,
                        observed_payload_bytes,
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

        emit_projection_status_signal_in_tx(
            ctx,
            tx,
            "projection",
            &projection_id,
            PROJECTION_SIGNATURE_INVALID_SIGNAL,
            json!({
                "projection_id": projection_id,
                "quarantine_id": quarantine_id,
                "verification_error": status.as_str(),
            }),
        )?;
        for claim_id in &claim_ids {
            emit_projection_status_signal_in_tx(
                ctx,
                tx,
                "claim",
                claim_id,
                PROJECTION_SIGNATURE_INVALID_SIGNAL,
                json!({
                    "projection_id": projection_id,
                    "quarantine_id": quarantine_id,
                    "verification_error": status.as_str(),
                }),
            )?;
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
    let current_signature_mismatch = current.current_signature_id.as_deref()
        != Some(&envelope.signature_id)
        || current.key_id.as_deref() != Some(&envelope.key_id)
        || current.signature_status.as_deref() != Some("active")
        || current.locator_status != "live";
    let composition_rollback = input.payload.composition_version < current.composition_version;
    let payload_hash_mismatch = canonical_hash != current.canonical_signed_payload_sha256
        || claim_watermark != current.claim_watermark_sha256;
    let claim_rollback = input
        .payload
        .blocks
        .iter()
        .flat_map(|block| block.claim_refs.iter())
        .find_map(|claim_ref| {
            current
                .claim_versions
                .get(&claim_ref.claim_id)
                .and_then(|ledger_version| {
                    (claim_ref.claim_version < *ledger_version)
                        .then_some((claim_ref.claim_version, *ledger_version))
                })
        });

    if current_signature_mismatch
        || composition_rollback
        || payload_hash_mismatch
        || claim_rollback.is_some()
    {
        let (signed_claim_version, ledger_claim_version) = claim_rollback
            .map(|(signed, ledger)| (Some(signed), Some(ledger)))
            .unwrap_or_else(|| {
                (
                    max_payload_claim_version(&input.payload),
                    max_ledger_claim_version(&current),
                )
            });
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

fn projection_locator_is_tombstoned(
    db: &ActionDb,
    projection_id: &str,
) -> Result<bool, ProjectionSigningError> {
    let status = db
        .conn_ref()
        .query_row(
            "SELECT locator_status FROM projection_ledger WHERE projection_id = ?1",
            params![projection_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(status.as_deref() == Some("tombstoned"))
}

fn load_ledger_currentness(
    db: &ActionDb,
    projection_id: &str,
) -> Result<Option<LedgerCurrentness>, ProjectionSigningError> {
    let current = db
        .conn_ref()
        .query_row(
            "SELECT l.current_signature_id, l.locator_status, l.composition_version,
                    l.canonical_signed_payload_sha256, l.claim_watermark_sha256,
                    s.key_id, s.signature_status
               FROM projection_ledger l
               LEFT JOIN projection_signatures s ON s.signature_id = l.current_signature_id
              WHERE l.projection_id = ?1",
            params![projection_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((
        current_signature_id,
        locator_status,
        composition_version,
        canonical_hash,
        claim_watermark,
        key_id,
        signature_status,
    )) = current
    else {
        return Ok(None);
    };
    let mut stmt = db.conn_ref().prepare(
        "SELECT claim_id, MAX(claim_version)
           FROM projection_ledger_block_refs
          WHERE projection_id = ?1
          GROUP BY claim_id",
    )?;
    let claim_versions = stmt
        .query_map(params![projection_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(Some(LedgerCurrentness {
        current_signature_id,
        key_id,
        signature_status,
        locator_status,
        composition_version,
        claim_versions,
        canonical_signed_payload_sha256: canonical_hash,
        claim_watermark_sha256: claim_watermark,
    }))
}

pub fn read_quarantine_observed_payload_bytes_admin(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    quarantine_id: &str,
) -> Result<Option<Vec<u8>>, ProjectionSigningError> {
    if actor_kind(ctx.actor) != "admin" {
        return Err(ProjectionSigningError::Service(
            "observed projection payload bytes require admin scope".to_string(),
        ));
    }
    db.conn_ref()
        .query_row(
            "SELECT observed_payload_bytes
               FROM projection_quarantine
              WHERE quarantine_id = ?1",
            params![quarantine_id],
            |row| row.get::<_, Option<Vec<u8>>>(0),
        )
        .optional()
        .map(|value| value.flatten())
        .map_err(ProjectionSigningError::from)
}

pub fn tombstone_projection_locator(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    projection_id: &str,
    reason: &str,
) -> Result<(), ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let now = timestamp(ctx);
    db.with_transaction(|tx| {
        let changed = tx
            .conn_ref()
            .execute(
                "UPDATE projection_ledger
                    SET locator_status = 'tombstoned',
                        verification_status = 'tombstoned',
                        last_verified_at = ?2
                  WHERE projection_id = ?1
                    AND locator_status != 'tombstoned'",
                params![projection_id, &now],
            )
            .map_err(|error| error.to_string())?;
        if changed > 0 {
            emit_projection_status_signal_in_tx(
                ctx,
                tx,
                "projection",
                projection_id,
                "projection_locator_tombstoned",
                json!({
                    "projection_id": projection_id,
                    "reason": reason,
                    "tombstoned_at": now,
                }),
            )?;
        }
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)
}

fn signed_payload_from_input(
    input: &ProjectionWriteInput,
) -> Result<SignedProjectionPayload, ProjectionSigningError> {
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

/// Return canonical JSON bytes for projection signing.
///
/// W4-C's RFC 8785 subset rejects non-integer JSON numbers at sign time.
/// Fractional numeric values must be string-encoded by the caller before they
/// enter `SignedProjectionPayload`; this prevents cross-runtime float spelling
/// drift from changing signed bytes.
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ProjectionSigningError> {
    let value = serde_json::to_value(value)?;
    validate_canonical_json_numbers(&value, "$")?;
    let canonical = canonicalize_json_value(value);
    serde_json::to_vec(&canonical).map_err(ProjectionSigningError::from)
}

fn validate_canonical_json_numbers(
    value: &Value,
    path: &str,
) -> Result<(), ProjectionSigningError> {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_canonical_json_numbers(item, &format!("{path}[{index}]"))?;
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                validate_canonical_json_numbers(item, &format!("{path}.{key}"))?;
            }
        }
        Value::Number(number) if !(number.is_i64() || number.is_u64()) => {
            return Err(ProjectionSigningError::Serialization(format!(
                "projection canonical JSON rejects non-integer numeric value at {path}; encode fractional values as strings"
            )));
        }
        _ => {}
    }
    Ok(())
}

fn canonicalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => {
            Value::Array(items.into_iter().map(canonicalize_json_value).collect())
        }
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
        .map_err(|_| ProjectionVerificationStatus::MalformedEnvelope)?;
    serde_json::from_slice(&bytes).map_err(|_| ProjectionVerificationStatus::MalformedEnvelope)
}

fn claim_watermark_hash(
    payload: &SignedProjectionPayload,
) -> Result<String, ProjectionSigningError> {
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
        .flat_map(|block| {
            block
                .claim_refs
                .iter()
                .map(|claim_ref| claim_ref.claim_id.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn max_payload_claim_version(payload: &SignedProjectionPayload) -> Option<u64> {
    payload
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .claim_refs
                .iter()
                .map(|claim_ref| claim_ref.claim_version)
        })
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
        .query_map(params![old_key_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (projection_id, old_signature_id) in rows {
        tx.conn_ref().execute(
            "INSERT OR IGNORE INTO projection_resign_queue
                (queue_id, projection_id, old_signature_id, old_key_id, new_key_id, reason,
                 status, attempts, max_attempts, last_error, last_resign_at, last_retampered_at,
                 operator_escalated_at, queued_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', 0, 5, NULL, NULL, NULL, NULL, ?7, ?7, NULL)",
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

#[derive(Debug)]
struct ResignQueueRow {
    queue_id: String,
    projection_id: String,
    old_signature_id: Option<String>,
    old_key_id: String,
    new_key_id: String,
    reason: String,
}

struct ResignSourceProjection {
    current_signature_id: Option<String>,
    dailyos_source_runtime: String,
    dailyos_projection_version: u64,
    quarantine_state: String,
    last_quarantine_event_at: Option<String>,
    canonical_signed_payload_bytes: Vec<u8>,
    canonical_signed_payload_sha256: String,
}

pub fn drain_resign_queue(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    limit: usize,
) -> Result<usize, ProjectionSigningError> {
    ctx.check_mutation_allowed()
        .map_err(|error| ProjectionSigningError::Service(error.to_string()))?;
    let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);
    let mut stmt = db.conn_ref().prepare(
        "SELECT queue_id, projection_id, old_signature_id, old_key_id, new_key_id, reason
           FROM projection_resign_queue
          WHERE status IN ('pending', 'failed')
            AND attempts < max_attempts
            AND operator_escalated_at IS NULL
          ORDER BY queued_at ASC
          LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok(ResignQueueRow {
                queue_id: row.get("queue_id")?,
                projection_id: row.get("projection_id")?,
                old_signature_id: row.get("old_signature_id")?,
                old_key_id: row.get("old_key_id")?,
                new_key_id: row.get("new_key_id")?,
                reason: row.get("reason")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut completed = 0;
    for row in rows {
        match process_resign_queue_row(ctx, db, key_store, &row) {
            Ok(()) => completed += 1,
            Err(error) => {
                mark_resign_failure(ctx, db, &row.queue_id, &error.to_string())?;
            }
        }
    }
    Ok(completed)
}

fn process_resign_queue_row(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    key_store: &dyn ProjectionKeyStore,
    row: &ResignQueueRow,
) -> Result<(), ProjectionSigningError> {
    let now = timestamp(ctx);
    let source = load_resign_source_projection(db, row)?;
    if source.current_signature_id != row.old_signature_id {
        mark_resign_completed(ctx, db, row, None, "already_superseded")?;
        return Ok(());
    }
    if projection_recently_retampered(ctx, &source) {
        mark_resign_retampered(ctx, db, &row.queue_id, &now)?;
        return Ok(());
    }

    let new_key = load_signing_key(db, &row.new_key_id)?
        .ok_or_else(|| ProjectionSigningError::ProjectionNotFound(row.new_key_id.clone()))?;
    if new_key.key_status == ProjectionKeyStatus::Revoked {
        return Err(ProjectionSigningError::Service(
            "replacement projection signing key is revoked".to_string(),
        ));
    }
    let private_key = key_store.get_private_key(&new_key.keychain_account_ref)?;
    let key_pair = Ed25519KeyPair::from_pkcs8(private_key.as_ref())
        .map_err(|error| ProjectionSigningError::Crypto(format!("invalid Ed25519 key: {error}")))?;
    let signature = key_pair.sign(&source.canonical_signed_payload_bytes);
    let signature_id = format!("sig_{}", Uuid::new_v4().simple());
    let envelope = ProjectionSignatureEnvelope {
        signature_id: signature_id.clone(),
        key_id: row.new_key_id.clone(),
        alg: PROJECTION_SIGNATURE_ALG.to_string(),
        canonicalization: PROJECTION_CANONICALIZATION.to_string(),
        signed_at: now.clone(),
        signature_b64url: URL_SAFE_NO_PAD.encode(signature.as_ref()),
        dailyos_source_runtime: source.dailyos_source_runtime,
        dailyos_projection_version: source.dailyos_projection_version,
    };
    let envelope_b64url = URL_SAFE_NO_PAD.encode(canonical_json_bytes(&envelope)?);

    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE projection_resign_queue
                    SET status = 'processing',
                        attempts = attempts + 1,
                        updated_at = ?2,
                        last_error = NULL
                  WHERE queue_id = ?1",
                params![&row.queue_id, &now],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE projection_signatures
                    SET signature_status = 'superseded',
                        superseded_by_signature_id = ?2,
                        retired_at = COALESCE(retired_at, ?3)
                  WHERE signature_id = ?1",
                params![row.old_signature_id.as_deref(), &signature_id, &now],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "INSERT INTO projection_signatures
                    (signature_id, projection_id, key_id, signature_status, alg, canonicalization,
                     canonical_signed_payload_bytes, canonical_signed_payload_sha256,
                     signature_bytes, signature_envelope_b64url, issued_at,
                     superseded_by_signature_id, revoked_at, retired_at)
                 VALUES (?1, ?2, ?3, 'active', ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL, NULL)",
                params![
                    &signature_id,
                    &row.projection_id,
                    &row.new_key_id,
                    PROJECTION_SIGNATURE_ALG,
                    PROJECTION_CANONICALIZATION,
                    &source.canonical_signed_payload_bytes,
                    &source.canonical_signed_payload_sha256,
                    signature.as_ref(),
                    &envelope_b64url,
                    &now,
                ],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE projection_ledger
                    SET current_signature_id = ?2,
                        verification_status = 'verified',
                        last_verified_at = ?3
                  WHERE projection_id = ?1",
                params![&row.projection_id, &signature_id, &now],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE projection_resign_queue
                    SET status = 'completed',
                        last_resign_at = ?2,
                        updated_at = ?2,
                        completed_at = ?2,
                        last_error = NULL
                  WHERE queue_id = ?1",
                params![&row.queue_id, &now],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE projection_replacement_keys
                    SET completed_at = COALESCE(completed_at, ?3),
                        recovery_status = 'completed'
                  WHERE old_key_id = ?1
                    AND new_key_id = ?2
                    AND NOT EXISTS (
                        SELECT 1
                          FROM projection_resign_queue
                         WHERE old_key_id = ?1
                           AND new_key_id = ?2
                           AND status != 'completed'
                    )",
                params![&row.old_key_id, &row.new_key_id, &now],
            )
            .map_err(|error| error.to_string())?;
        emit_projection_status_signal_in_tx(
            ctx,
            tx,
            "projection",
            &row.projection_id,
            "projection_resigned",
            json!({
                "old_key_id": row.old_key_id,
                "new_key_id": row.new_key_id,
                "old_signature_id": row.old_signature_id,
                "new_signature_id": signature_id,
                "reason": row.reason,
            }),
        )?;
        Ok(())
    })
    .map_err(ProjectionSigningError::Database)
}

fn load_resign_source_projection(
    db: &ActionDb,
    row: &ResignQueueRow,
) -> Result<ResignSourceProjection, ProjectionSigningError> {
    db.conn_ref()
        .query_row(
            "SELECT l.current_signature_id, l.dailyos_source_runtime,
                    l.dailyos_projection_version, l.quarantine_state,
                    l.last_quarantine_event_at, s.canonical_signed_payload_bytes,
                    s.canonical_signed_payload_sha256
               FROM projection_ledger l
               JOIN projection_signatures s ON s.signature_id = ?2
              WHERE l.projection_id = ?1
                AND l.locator_status = 'live'",
            params![&row.projection_id, row.old_signature_id.as_deref()],
            |row| {
                Ok(ResignSourceProjection {
                    current_signature_id: row.get(0)?,
                    dailyos_source_runtime: row.get(1)?,
                    dailyos_projection_version: row.get(2)?,
                    quarantine_state: row.get(3)?,
                    last_quarantine_event_at: row.get(4)?,
                    canonical_signed_payload_bytes: row.get(5)?,
                    canonical_signed_payload_sha256: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| ProjectionSigningError::ProjectionNotFound(row.projection_id.clone()))
}

fn projection_recently_retampered(
    ctx: &ServiceContext<'_>,
    source: &ResignSourceProjection,
) -> bool {
    if source.quarantine_state != "quarantined" {
        return false;
    }
    source
        .last_quarantine_event_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|detected_at| {
            ctx.clock
                .now()
                .signed_duration_since(detected_at.with_timezone(&chrono::Utc))
                <= Duration::seconds(120)
        })
        .unwrap_or(false)
}

fn mark_resign_completed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    row: &ResignQueueRow,
    last_resign_at: Option<&str>,
    reason: &str,
) -> Result<(), ProjectionSigningError> {
    let now = timestamp(ctx);
    db.conn_ref().execute(
        "UPDATE projection_resign_queue
            SET status = 'completed',
                updated_at = ?2,
                completed_at = ?2,
                last_resign_at = COALESCE(?3, last_resign_at),
                last_error = ?4
          WHERE queue_id = ?1",
        params![&row.queue_id, &now, last_resign_at, reason],
    )?;
    Ok(())
}

fn mark_resign_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    queue_id: &str,
    error: &str,
) -> Result<(), ProjectionSigningError> {
    let now = timestamp(ctx);
    let error = truncate_queue_error(error);
    db.conn_ref().execute(
        "UPDATE projection_resign_queue
            SET attempts = attempts + 1,
                status = CASE WHEN attempts + 1 >= max_attempts THEN 'failed' ELSE 'pending' END,
                last_error = ?2,
                updated_at = ?3,
                operator_escalated_at = CASE
                    WHEN attempts + 1 >= max_attempts THEN COALESCE(operator_escalated_at, ?3)
                    ELSE operator_escalated_at
                END
          WHERE queue_id = ?1",
        params![queue_id, error, now],
    )?;
    Ok(())
}

fn mark_resign_retampered(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    queue_id: &str,
    now: &str,
) -> Result<(), ProjectionSigningError> {
    db.conn_ref().execute(
        "UPDATE projection_resign_queue
            SET attempts = max_attempts,
                status = 'failed',
                last_error = 'projection_retampered_after_resign',
                last_retampered_at = ?2,
                operator_escalated_at = COALESCE(operator_escalated_at, ?2),
                updated_at = ?2
          WHERE queue_id = ?1",
        params![queue_id, now],
    )?;
    emit_projection_status_signal_in_tx(
        ctx,
        db,
        "projection_resign_queue",
        queue_id,
        "projection_resign_operator_escalated",
        json!({
            "queue_id": queue_id,
            "reason": "projection_retampered_after_resign",
            "detected_at": now,
            "actor_kind": actor_kind(ctx.actor),
        }),
    )
    .map_err(ProjectionSigningError::Service)?;
    Ok(())
}

fn truncate_queue_error(error: &str) -> String {
    const MAX_ERROR_CHARS: usize = 240;
    error.chars().take(MAX_ERROR_CHARS).collect()
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
            self.keys
                .lock()
                .insert(account_ref.to_string(), pkcs8.to_vec());
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

        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
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
                observed_payload_bytes: None,
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
        assert_ne!(
            signed.signature_envelope_b64url,
            signed.canonical_signed_payload_sha256
        );
    }

    #[test]
    fn dos569_fixture_block_ordering_mutation_is_quarantined() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
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
                observed_payload_bytes: None,
            },
        )
        .expect("tamper outcome");

        assert_eq!(
            outcome.status,
            ProjectionVerificationStatus::SignatureInvalid
        );
        assert!(outcome.quarantine_id.is_some());
        assert!(matches!(
            outcome.failure,
            Some(ProjectionVerificationFailure::Tampered(_))
        ));
    }

    #[test]
    fn dos569_fixture_unsupported_alg_is_quarantined() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
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
                observed_payload_bytes: None,
            },
        )
        .expect("unsupported alg outcome");

        assert_eq!(
            outcome.status,
            ProjectionVerificationStatus::UnsupportedAlgorithm
        );
        let seen_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM projection_quarantine", [], |row| {
                row.get(0)
            })
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
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
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
                    observed_payload_bytes: None,
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
        let old_signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign old");
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
                observed_payload_bytes: None,
            },
        )
        .expect("rollback outcome");

        assert_eq!(
            outcome.status,
            ProjectionVerificationStatus::VersionRollback
        );
        assert!(matches!(
            outcome.failure,
            Some(ProjectionVerificationFailure::VersionRollback(_))
        ));
    }

    #[test]
    fn dos569_fixture_key_revoked_queues_replacement_resign() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
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
            .query_row("SELECT COUNT(*) FROM projection_resign_queue", [], |row| {
                row.get(0)
            })
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
                observed_payload_bytes: None,
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
        assert_eq!(
            canonical_json_bytes(&a).unwrap(),
            canonical_json_bytes(&b).unwrap()
        );
        assert_ne!(
            canonical_json_bytes(&a).unwrap(),
            canonical_json_bytes(&c).unwrap()
        );
    }

    #[test]
    fn dos569_cycle2_malformed_envelope_records_distinct_status() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: signed.payload.projection_id.clone(),
                surface: signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: signed.payload,
                signature_envelope_b64url: Some("not-valid-base64*".to_string()),
                observed_payload_bytes: None,
            },
        )
        .expect("malformed envelope outcome");

        assert_eq!(
            outcome.status,
            ProjectionVerificationStatus::MalformedEnvelope
        );
        let stored_status: String = db
            .conn_ref()
            .query_row(
                "SELECT verification_error FROM projection_quarantine WHERE quarantine_id = ?1",
                params![outcome.quarantine_id.as_deref()],
                |row| row.get(0),
            )
            .expect("quarantine status");
        assert_eq!(stored_status, "malformed_envelope");
    }

    #[test]
    fn dos569_cycle2_non_integer_numbers_rejected_before_signing() {
        let mut input = projection_input();
        input.blocks[0].block_payload = json!({"ratio": 1.5});

        let error = signed_payload_from_input(&input).expect_err("fractional payload rejected");
        assert!(
            error
                .to_string()
                .contains("rejects non-integer numeric value"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn dos569_cycle2_unknown_key_refreshes_once_then_falls_through() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let mut envelope = signed.signature_envelope.clone();
        envelope.key_id = "psk_missing_refresh_once".to_string();
        let envelope_b64 = URL_SAFE_NO_PAD.encode(canonical_json_bytes(&envelope).unwrap());
        let mut refresh_calls = 0;

        let outcome = verify_with_unknown_key_refresh(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: signed.payload.projection_id.clone(),
                surface: signed.payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: signed.payload,
                signature_envelope_b64url: Some(envelope_b64),
                observed_payload_bytes: None,
            },
            || {
                refresh_calls += 1;
                Ok(())
            },
        )
        .expect("unknown key outcome");

        assert_eq!(refresh_calls, 1);
        assert_eq!(outcome.status, ProjectionVerificationStatus::UnknownKey);
    }

    #[test]
    fn dos569_cycle2_keyring_version_is_monotonic_across_rotation() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let before = public_keyring(&db, "runtime-anchor").expect("keyring before revoke");

        revoke_projection_signing_key(
            &ctx,
            &db,
            &key_store,
            &signed.signature_envelope.key_id,
            "test_rotation",
        )
        .expect("revoke projection key");
        let after = public_keyring(&db, "runtime-anchor").expect("keyring after revoke");

        assert!(after.keyring_version > before.keyring_version);
        assert!(after.keyring_version > after.keys.len() as u64);
    }

    #[test]
    fn dos569_cycle2_retired_key_verifies_with_degraded_status() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        db.conn_ref()
            .execute(
                "UPDATE projection_signing_keys
                    SET key_status = 'retired', retired_at = ?2
                  WHERE key_id = ?1",
                params![&signed.signature_envelope.key_id, timestamp(&ctx)],
            )
            .expect("retire key");

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
                observed_payload_bytes: None,
            },
        )
        .expect("retired verify outcome");

        assert_eq!(
            outcome.status,
            ProjectionVerificationStatus::VerifiedRetired
        );
        assert_eq!(outcome.key_status, Some(ProjectionKeyStatus::Retired));
        let retired_signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = ?1",
                params![PROJECTION_SIGNATURE_RETIRED_KEY_SIGNAL],
                |row| row.get(0),
            )
            .expect("retired signal count");
        assert!(retired_signal_count >= 1);
    }

    #[test]
    fn dos569_cycle2_tombstoned_locator_blocks_verified_read() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");

        tombstone_projection_locator(&ctx, &db, &signed.payload.projection_id, "test_removal")
            .expect("tombstone projection locator");
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
                observed_payload_bytes: None,
            },
        )
        .expect("tombstoned verify outcome");

        assert_eq!(outcome.status, ProjectionVerificationStatus::Tombstoned);
        let locator_status: String = db
            .conn_ref()
            .query_row(
                "SELECT locator_status FROM projection_ledger WHERE projection_id = ?1",
                params![outcome.projection_id],
                |row| row.get(0),
            )
            .expect("locator status");
        assert_eq!(locator_status, "tombstoned");
    }

    #[test]
    fn dos569_cycle2_quarantine_preserves_observed_bytes_admin_only() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let admin_ctx = ServiceContext::test_live(&clock, &rng, &ext).with_actor("admin:test");
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let mut tampered_payload = signed.payload.clone();
        tampered_payload.blocks[0].block_payload = json!({"a": 99});
        let observed_bytes = br#"{"raw":"tampered wp row"}"#.to_vec();

        let outcome = verify_projection_read(
            &ctx,
            &db,
            ProjectionVerificationInput {
                projection_id: tampered_payload.projection_id.clone(),
                surface: tampered_payload.surface,
                surface_locator: "wp:post:42:block:account-overview".to_string(),
                expected_runtime_anchor_id: None,
                payload: tampered_payload,
                signature_envelope_b64url: Some(signed.signature_envelope_b64url),
                observed_payload_bytes: Some(observed_bytes.clone()),
            },
        )
        .expect("tampered outcome");
        let quarantine_id = outcome.quarantine_id.expect("quarantine id");

        let stored = read_quarantine_observed_payload_bytes_admin(&admin_ctx, &db, &quarantine_id)
            .expect("admin read observed bytes")
            .expect("stored observed bytes");
        assert_eq!(stored, observed_bytes);
        assert!(read_quarantine_observed_payload_bytes_admin(&ctx, &db, &quarantine_id).is_err());
    }

    #[test]
    fn dos569_cycle2_enforcement_mode_transition_is_audited() {
        let db = test_db();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        assert_eq!(
            projection_signature_enforcement_mode(&db).expect("default mode"),
            crate::services::context::ProjectionSignatureEnforcementMode::Shadow
        );
        set_projection_signature_enforcement_mode(
            &ctx,
            &db,
            crate::services::context::ProjectionSignatureEnforcementMode::Enforce,
            "test_enforce",
        )
        .expect("set enforce mode");
        set_projection_signature_enforcement_mode(
            &ctx,
            &db,
            crate::services::context::ProjectionSignatureEnforcementMode::Enforce,
            "idempotent",
        )
        .expect("set enforce mode idempotently");

        assert_eq!(
            projection_signature_enforcement_mode(&db).expect("updated mode"),
            crate::services::context::ProjectionSignatureEnforcementMode::Enforce
        );
        let event_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM projection_enforcement_mode_events",
                [],
                |row| row.get(0),
            )
            .expect("event count");
        assert_eq!(event_count, 1);
        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = 'projection.enforcement_mode_changed'",
                [],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(signal_count, 1);
    }

    #[test]
    fn dos569_cycle2_resign_queue_drain_reissues_active_signature() {
        let db = test_db();
        let key_store = InMemoryProjectionKeyStore::default();
        let clock = fixture_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signed =
            sign_projection(&ctx, &db, &key_store, projection_input()).expect("sign projection");
        let old_signature_id = signed.signature_envelope.signature_id.clone();
        let old_key_id = signed.signature_envelope.key_id.clone();
        let replacement_key_id = revoke_projection_signing_key(
            &ctx,
            &db,
            &key_store,
            &old_key_id,
            "test_key_compromise",
        )
        .expect("revoke key");

        let drained = drain_resign_queue(&ctx, &db, &key_store, 10).expect("drain resign queue");

        assert_eq!(drained, 1);
        let queue_status: String = db
            .conn_ref()
            .query_row("SELECT status FROM projection_resign_queue", [], |row| {
                row.get(0)
            })
            .expect("queue status");
        assert_eq!(queue_status, "completed");
        let active_key_id: String = db
            .conn_ref()
            .query_row(
                "SELECT s.key_id
                   FROM projection_ledger l
                   JOIN projection_signatures s ON s.signature_id = l.current_signature_id
                  WHERE l.projection_id = ?1",
                params![signed.payload.projection_id],
                |row| row.get(0),
            )
            .expect("active key id");
        assert_eq!(active_key_id, replacement_key_id);
        let old_status: String = db
            .conn_ref()
            .query_row(
                "SELECT signature_status FROM projection_signatures WHERE signature_id = ?1",
                params![old_signature_id],
                |row| row.get(0),
            )
            .expect("old signature status");
        assert_eq!(old_status, "superseded");
        let recovery_status: String = db
            .conn_ref()
            .query_row(
                "SELECT recovery_status FROM projection_replacement_keys WHERE old_key_id = ?1",
                params![old_key_id],
                |row| row.get(0),
            )
            .expect("replacement status");
        assert_eq!(recovery_status, "completed");
    }
}
