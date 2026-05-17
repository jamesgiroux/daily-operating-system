use std::collections::BTreeSet;

use base64::Engine as _;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use http::StatusCode;
use rand::Rng;
use ring::{hkdf, hmac};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use abilities_runtime::abilities::registry::{Actor, ScopeSet, SurfaceClientId, SurfaceScope};

use crate::audit_log::{emit_surface_audit, AuditFields, AuditLogger};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;

pub const PAIRING_CODE_TTL_SECONDS: i64 = 5 * 60;
pub const SESSION_INACTIVE_TTL_SECONDS: i64 = 15 * 60;
pub const SESSION_ABSOLUTE_TTL_SECONDS: i64 = 8 * 60 * 60;
pub const SESSION_SUSPICIOUS_THROTTLE_SECONDS: i64 = 60;
const DEFAULT_GRANTED_SCOPES: &[&str] = &["read.account_overview", "submit.feedback"];
const HMAC_SESSION_KEY_INFO: &[u8] = b"dailyos-wp-bridge-v1";
const HMAC_SESSION_KEY_BYTES: usize = 32;

#[derive(Debug, Clone)]
pub struct PairingCodeIssueInput {
    pub runtime_anchor_id: String,
    pub endpoint_startup_id: String,
    pub bound_port: u16,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCodeIssue {
    pub pairing_string: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PairingHandshakeRequest {
    pub pairing_code: String,
    pub wp_user_id: u64,
    pub wp_site_id: String,
    pub home_url: String,
    pub site_url: String,
    pub wp_install_uuid: String,
    pub plugin_instance_uuid: String,
    #[serde(default)]
    pub multisite_blog_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub client_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct PairingHandshakeInput {
    pub runtime_anchor_id: String,
    pub endpoint_startup_id: String,
    pub bound_port: u16,
    pub endpoint_version: &'static str,
    pub max_failed_attempts: u32,
    pub now: DateTime<Utc>,
    pub request: PairingHandshakeRequest,
}

#[derive(Debug, Clone)]
pub struct PairingHandshakeCapacityInput {
    pub runtime_anchor_id: String,
    pub request: PairingHandshakeRequest,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingHandshakeResponse {
    pub surface_client_id: String,
    pub session_id: String,
    pub hmac_key_id: String,
    pub hmac_key: String,
    pub endpoint_version: &'static str,
    pub granted_scopes: Vec<String>,
    pub scope_digest: String,
    pub site_binding_digest: String,
    pub site_nonce: String,
    pub pairing_epoch: i64,
    pub inactive_expires_at: String,
    pub absolute_expires_at: String,
    pub ability_projection: Vec<SurfaceAbilityProjection>,
}

#[derive(Debug, Clone)]
pub struct IssuedSessionMaterial {
    pub session_id: String,
    pub surface_client_id: String,
    pub hmac_master_key: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct PairingHandshakeOutcome {
    pub response: PairingHandshakeResponse,
    pub session: IssuedSessionMaterial,
    pub audit: SurfacePairingAuditEvent,
    pub revocation_audit: Option<SurfacePairingAuditEvent>,
    pub paired_origin: String,
    pub revoked_surface_client_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SurfaceSessionRefreshInput {
    pub session_id: String,
    pub site_binding_digest: String,
    pub wp_install_uuid: String,
    pub plugin_instance_uuid: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceSessionRefreshIdentity {
    Matched,
    SessionNotFound,
    IdentityMismatch,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceAbilityProjection {
    pub name_hash: String,
    pub required_scope_hashes: Vec<String>,
    pub client_side_executable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceClientPairingSummary {
    pub surface_client_id: String,
    pub surface_client_display_id: String,
    pub site_binding_digest: String,
    pub scope_digest: String,
    pub lifecycle_state: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SignedSessionValidationInput {
    pub session_id: String,
    pub surface_client_id: String,
    pub runtime_anchor_id: String,
    pub site_claims: SignedSiteClaimsInput,
    pub site_nonce: String,
    pub wp_user_id: u64,
    pub wp_user_hash: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SignedSiteClaimsInput {
    pub wp_site_id: String,
    pub home_url: String,
    pub site_url: String,
    pub wp_install_uuid: String,
    pub plugin_instance_uuid: String,
    pub multisite_blog_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedSurfaceSession {
    pub surface_client_id: String,
    pub session_id: String,
    pub actor: Actor,
    pub wp_user_id: Option<u64>,
    pub wp_user_hash: Option<String>,
    pub wp_site_id: String,
    pub wp_site_id_hash: String,
    pub site_binding_digest: String,
    pub site_nonce: String,
    pub scope_digest: String,
    pub granted_scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PairingCodeFailureInput {
    pub endpoint_startup_id: String,
    pub bound_port: u16,
    pub pairing_code: String,
    pub max_failed_attempts: u32,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PairingCodeFailureOutcome {
    pub error: Option<SurfacePairingError>,
    pub audit: SurfacePairingAuditEvent,
}

#[derive(Debug, Clone)]
pub struct SignedTransportFailureInput {
    pub session_id: String,
    pub surface_client_id: Option<String>,
    pub failure_code: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RevokePairingInput {
    pub surface_client_id: String,
    pub reason: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SurfacePairingAuditEvent {
    pub event_kind: &'static str,
    pub category: &'static str,
    pub actor: Actor,
    pub wp_user_id: Option<u64>,
    pub wp_user_hash: Option<String>,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfacePairingError {
    BadRequest(&'static str),
    PairingCodeInvalid,
    PairingCodeExpired,
    PairingCodeConsumed,
    PairingCodeLimited,
    UnknownRuntimeAnchor,
    RestoredStalePairing,
    SiteBindingMismatch,
    PairingSuspended,
    PairingRevoked,
    PairingExpired,
    SessionInvalid,
    SessionExpired,
    SessionThrottled,
    WpUserMismatch,
    ScopeDenied,
    Write(String),
}

impl SurfacePairingError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(code) => code,
            Self::PairingCodeInvalid => "pairing_code_invalid",
            Self::PairingCodeExpired => "pairing_code_expired",
            Self::PairingCodeConsumed => "pairing_code_consumed",
            Self::PairingCodeLimited => "pairing_code_limited",
            Self::UnknownRuntimeAnchor => "unknown_runtime_anchor",
            Self::RestoredStalePairing => "restored_stale_pairing",
            Self::SiteBindingMismatch => "site_binding_mismatch",
            Self::PairingSuspended => "pairing_suspended",
            Self::PairingRevoked => "pairing_revoked",
            Self::PairingExpired => "pairing_expired",
            Self::SessionInvalid => "session_invalid",
            Self::SessionExpired => "session_expired",
            Self::SessionThrottled => "session_throttled",
            Self::WpUserMismatch => "wp_user_mismatch",
            Self::ScopeDenied => "scope_denied",
            Self::Write(_) => "pairing_authority_unavailable",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::PairingCodeLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::SessionThrottled => StatusCode::TOO_MANY_REQUESTS,
            Self::SiteBindingMismatch
            | Self::PairingSuspended
            | Self::PairingRevoked
            | Self::PairingExpired
            | Self::RestoredStalePairing
            | Self::WpUserMismatch
            | Self::ScopeDenied => StatusCode::FORBIDDEN,
            Self::Write(_) => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::UNAUTHORIZED,
        }
    }

    pub fn safe_message(&self) -> &'static str {
        match self {
            Self::SiteBindingMismatch => "The paired site identity changed.",
            Self::PairingCodeLimited => "Too many failed pairing attempts.",
            Self::SessionThrottled => "The paired surface session is temporarily throttled.",
            Self::WpUserMismatch => "The paired user identity changed.",
            Self::PairingRevoked => "This surface pairing was revoked.",
            Self::PairingExpired | Self::SessionExpired => "This surface session expired.",
            Self::Write(_) => "The DailyOS pairing authority is unavailable.",
            _ => "DailyOS surface pairing validation failed.",
        }
    }
}

impl std::fmt::Display for SurfacePairingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write(message) => write!(formatter, "{}: {message}", self.code()),
            _ => formatter.write_str(self.code()),
        }
    }
}

impl std::error::Error for SurfacePairingError {}

impl From<SurfacePairingError> for String {
    fn from(error: SurfacePairingError) -> Self {
        error.to_string()
    }
}

impl From<String> for SurfacePairingError {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pairing_code_invalid" => Self::PairingCodeInvalid,
            "pairing_code_expired" => Self::PairingCodeExpired,
            "pairing_code_consumed" => Self::PairingCodeConsumed,
            "pairing_code_limited" => Self::PairingCodeLimited,
            "unknown_runtime_anchor" => Self::UnknownRuntimeAnchor,
            "restored_stale_pairing" => Self::RestoredStalePairing,
            "site_binding_mismatch" => Self::SiteBindingMismatch,
            "pairing_suspended" => Self::PairingSuspended,
            "pairing_revoked" => Self::PairingRevoked,
            "pairing_expired" => Self::PairingExpired,
            "session_invalid" => Self::SessionInvalid,
            "session_expired" => Self::SessionExpired,
            "session_throttled" => Self::SessionThrottled,
            "wp_user_mismatch" => Self::WpUserMismatch,
            "scope_denied" => Self::ScopeDenied,
            _ => Self::Write(value),
        }
    }
}

pub fn issue_pairing_code(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PairingCodeIssueInput,
) -> Result<PairingCodeIssue, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;

    let token = random_url_token(24);
    let code_hash = pairing_code_hash(&token);
    let issued_at = format_ts(input.now);
    let expires_at = format_ts(input.now + Duration::seconds(PAIRING_CODE_TTL_SECONDS));
    db.conn_ref()
        .execute(
            "INSERT INTO surface_pairing_codes (
                code_hash, endpoint_startup_id, bound_port, issued_at, expires_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                code_hash,
                input.endpoint_startup_id,
                i64::from(input.bound_port),
                issued_at,
                expires_at
            ],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;

    let pairing_string = format!("dailyos://pair?port={}&code={token}", input.bound_port);
    Ok(PairingCodeIssue {
        pairing_string,
        expires_at,
    })
}

pub fn record_pairing_code_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PairingCodeFailureInput,
) -> Result<Option<SurfacePairingError>, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let code = pairing_code_token(&input.pairing_code)
        .ok_or(SurfacePairingError::BadRequest("pairing_code_invalid"))?;
    let code_hash = pairing_code_hash(&code);
    let now = format_ts(input.now);
    db.with_transaction(|tx| {
        record_pairing_code_failure_tx(
            tx,
            &code_hash,
            &input.endpoint_startup_id,
            input.bound_port,
            &now,
            input.max_failed_attempts,
        )
    })
    .map_err(SurfacePairingError::Write)
}

pub fn record_pairing_code_failure_with_audit(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PairingCodeFailureInput,
) -> Result<PairingCodeFailureOutcome, SurfacePairingError> {
    let error = match record_pairing_code_failure(ctx, db, input.clone()) {
        Ok(error) => error,
        Err(SurfacePairingError::BadRequest("pairing_code_invalid")) => {
            Some(SurfacePairingError::PairingCodeInvalid)
        }
        Err(error) => return Err(error),
    };
    Ok(PairingCodeFailureOutcome {
        audit: pairing_code_failure_audit_event(&input, error.as_ref()),
        error,
    })
}

pub fn pairing_code_failure_audit_event(
    input: &PairingCodeFailureInput,
    error: Option<&SurfacePairingError>,
) -> SurfacePairingAuditEvent {
    let pairing_code_hash = pairing_code_token(&input.pairing_code)
        .map(|code| pairing_code_hash(&code))
        .unwrap_or_else(|| stable_hash("pairing_code_invalid", input.pairing_code.trim()));
    SurfacePairingAuditEvent {
        event_kind: "pairing_code_failed",
        category: "security",
        actor: Actor::System,
        wp_user_id: None,
        wp_user_hash: None,
        detail: json!({
            "pairing_code_hash": pairing_code_hash,
            "endpoint_startup_id_hash": stable_hash("endpoint_startup_id", &input.endpoint_startup_id),
            "bound_port": input.bound_port,
            "reason": error.map(SurfacePairingError::code).unwrap_or("handshake_body_invalid"),
            "decision": "rejected"
        }),
    }
}

pub fn complete_handshake(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PairingHandshakeInput,
) -> Result<PairingHandshakeOutcome, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;

    let code = pairing_code_token(&input.request.pairing_code)
        .ok_or(SurfacePairingError::BadRequest("pairing_code_invalid"))?;
    let code_hash = pairing_code_hash(&code);
    let claims = match SiteClaims::from_request(&input.request) {
        Ok(claims) => claims,
        Err(error) => {
            let limit_error = record_failed_handshake_for_input(db, &code_hash, &input)?;
            return Err(limit_error.unwrap_or(error));
        }
    };
    let site_nonce = random_url_token(18);
    let site_binding_digest = claims.site_binding_digest();
    let site_origin = match claims.site_origin() {
        Some(origin) => origin,
        None => {
            let limit_error = record_failed_handshake_for_input(db, &code_hash, &input)?;
            return Err(limit_error.unwrap_or(SurfacePairingError::BadRequest("site_url_invalid")));
        }
    };
    let wp_install_uuid_hash = stable_hash("wp_install_uuid", &claims.wp_install_uuid);
    let plugin_instance_uuid_hash =
        stable_hash("plugin_instance_uuid", &claims.plugin_instance_uuid);
    let scopes = default_granted_scopes();
    let scope_digest = scope_digest(&scopes);
    let scope_set = scope_set_from_strings(&scopes)?;
    let surface_client_id = format!("sc_{}", Uuid::new_v4().simple());
    let pairing_id = format!("sp_{}", Uuid::new_v4().simple());
    let session_id = format!("sess_{}", Uuid::new_v4().simple());
    let hmac_master_key = random_key32();
    let hmac_key_id = session_id.clone();
    let issued_at = format_ts(input.now);
    let inactive_expires_at =
        format_ts(input.now + Duration::seconds(SESSION_INACTIVE_TTL_SECONDS));
    let absolute_expires_at =
        format_ts(input.now + Duration::seconds(SESSION_ABSOLUTE_TTL_SECONDS));
    let pairing_expires_at = absolute_expires_at.clone();
    let site_binding_claims_json = serde_json::to_string(&claims)
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let ability_projection = ability_projection_for_scopes(&scope_set);
    let ability_projection_json = serde_json::to_string(&ability_projection)
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let scopes_json = serde_json::to_string(&scopes)
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let wp_user_hash = wp_user_hash(
        &hmac_master_key,
        &site_binding_digest,
        input.request.wp_user_id,
    );
    let audit_id = format!("surface_pairing_{}", Uuid::new_v4().simple());
    let wp_request_id_hash = input
        .request
        .request_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| stable_hash("wp_request_id", value.trim()));
    let client_metadata_hash = input
        .request
        .client_metadata
        .as_ref()
        .map(|value| stable_hash("client_metadata", &value.to_string()));

    let write_result = db.with_transaction(|tx| {
            if let PairingCodeClaim::Rejected(error) = consume_pairing_code(tx, &code_hash, &input)? {
                return Ok(Err(error));
            }
            let previous_pairing = revoke_existing_pairing_for_site(
                tx,
                &input.runtime_anchor_id,
                &site_binding_digest,
                &input.now,
                "replaced_by_repairing",
            )?;
            let previous_pairing_id = previous_pairing
                .as_ref()
                .map(|pairing| pairing.pairing_id.clone());
            let next_epoch =
                next_pairing_epoch(tx, &input.runtime_anchor_id, &site_binding_digest)?;
            tx.conn_ref()
                .execute(
                    "INSERT INTO surface_client_pairings (
                        pairing_id, surface_client_id, runtime_anchor_id, pairing_epoch,
                        lifecycle_state, previous_pairing_id, site_binding_digest,
                        site_binding_claims_json, wp_install_uuid_hash,
                        plugin_instance_uuid_hash, site_nonce, scopes_json, scope_digest,
                        endpoint_version, ability_projection_json, created_at, activated_at,
                        last_used_at, expires_at, audit_id
                    ) VALUES (
                        ?1, ?2, ?3, ?4, 'active', ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                        ?12, ?13, ?14, ?15, ?15, ?15, ?16, ?17
                    )",
                    params![
                        pairing_id,
                        surface_client_id,
                        input.runtime_anchor_id,
                        next_epoch,
                        previous_pairing_id,
                        site_binding_digest,
                        site_binding_claims_json,
                        wp_install_uuid_hash,
                        plugin_instance_uuid_hash,
                        site_nonce,
                        scopes_json,
                        scope_digest,
                        input.endpoint_version,
                        ability_projection_json,
                        issued_at,
                        pairing_expires_at,
                        audit_id
                    ],
                )
                .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
            tx.conn_ref()
                .execute(
                    "INSERT INTO surface_client_epoch_floors (
                        runtime_anchor_id, site_binding_digest, highest_pairing_epoch, updated_at
                    ) VALUES (?1, ?2, ?3, ?4)
                    ON CONFLICT(runtime_anchor_id, site_binding_digest)
                    DO UPDATE SET
                        highest_pairing_epoch = MAX(highest_pairing_epoch, excluded.highest_pairing_epoch),
                        updated_at = excluded.updated_at",
                    params![
                        input.runtime_anchor_id,
                        site_binding_digest,
                        next_epoch,
                        issued_at
                    ],
                )
                .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
            tx.conn_ref()
                .execute(
                    "INSERT INTO surface_client_sessions (
                        session_id, surface_client_id, pairing_epoch, hmac_key_id,
                        issued_at, last_seen_at, inactive_expires_at, absolute_expires_at,
                        scope_digest, site_binding_digest, wp_user_hash
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        session_id,
                        surface_client_id,
                        next_epoch,
                        hmac_key_id,
                        issued_at,
                        inactive_expires_at,
                        absolute_expires_at,
                        scope_digest,
                        site_binding_digest,
                        wp_user_hash
                    ],
                )
                .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
            Ok(Ok((next_epoch, previous_pairing)))
        })?;
    let (pairing_epoch, previous_pairing) = write_result?;
    let previous_pairing_id = previous_pairing
        .as_ref()
        .map(|pairing| pairing.pairing_id.clone());
    let hmac_session_key = derive_session_hmac_key(hmac_master_key, &session_id);
    let response = PairingHandshakeResponse {
        surface_client_id: surface_client_id.clone(),
        session_id: session_id.clone(),
        hmac_key_id: hmac_key_id.clone(),
        hmac_key: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hmac_session_key),
        endpoint_version: input.endpoint_version,
        granted_scopes: scopes.clone(),
        scope_digest: scope_digest.clone(),
        site_binding_digest: site_binding_digest.clone(),
        site_nonce: site_nonce.clone(),
        pairing_epoch,
        inactive_expires_at,
        absolute_expires_at,
        ability_projection: ability_projection.clone(),
    };
    let actor = Actor::SurfaceClient {
        instance: SurfaceClientId::new(surface_client_id.clone()),
        scopes: scope_set.clone(),
    };
    let revocation_audit = previous_pairing.as_ref().map(|pairing| {
        pairing_revoked_audit_event(
            RevocationAuditInput {
                surface_client_id: &pairing.surface_client_id,
                pairing_epoch: pairing.pairing_epoch,
                site_binding_digest: &pairing.site_binding_digest,
                scope_digest: &pairing.scope_digest,
                site_binding_claims_json: pairing.site_binding_claims_json.as_deref(),
                stored_wp_user_hash: pairing.wp_user_hash.as_deref(),
                reason: "replaced_by_repairing",
            },
            Actor::SurfaceClient {
                instance: SurfaceClientId::new(pairing.surface_client_id.clone()),
                scopes: scope_set.clone(),
            },
            Some(input.request.wp_user_id),
            pairing.wp_user_hash.clone(),
        )
    });
    let revoked_surface_client_ids = previous_pairing
        .into_iter()
        .map(|pairing| pairing.surface_client_id)
        .collect();
    let audit = SurfacePairingAuditEvent {
        event_kind: if previous_pairing_id.is_some() {
            "pairing_refreshed"
        } else {
            "pairing_created"
        },
        category: "security",
        actor,
        wp_user_id: Some(input.request.wp_user_id),
        wp_user_hash: Some(wp_user_hash.clone()),
        detail: json!({
            "surface_client_id": surface_client_id,
            "site_binding_digest": site_binding_digest,
            "scope_digest": scope_digest,
            "wp_site_id_hash": stable_hash("wp_site_id", &input.request.wp_site_id),
            "pairing_epoch": pairing_epoch,
            "previous_pairing_id": previous_pairing_id,
            "wp_request_id_hash": wp_request_id_hash,
            "client_metadata_hash": client_metadata_hash,
            "decision": "allowed"
        }),
    };

    Ok(PairingHandshakeOutcome {
        response,
        session: IssuedSessionMaterial {
            session_id,
            surface_client_id,
            hmac_master_key,
        },
        audit,
        revocation_audit,
        paired_origin: site_origin,
        revoked_surface_client_ids,
    })
}

pub fn replaceable_surface_client_ids_for_handshake(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PairingHandshakeCapacityInput,
) -> Result<Vec<String>, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let claims = SiteClaims::from_request(&input.request)?;
    let site_binding_digest = claims.site_binding_digest();
    db.conn_ref()
        .query_row(
            "SELECT surface_client_id
             FROM surface_client_pairings
             WHERE runtime_anchor_id = ?1
               AND site_binding_digest = ?2
               AND lifecycle_state IN ('active', 'suspended', 'issued')
             ORDER BY pairing_epoch DESC
             LIMIT 1",
            params![input.runtime_anchor_id, site_binding_digest],
            |row| row.get::<_, String>("surface_client_id"),
        )
        .optional()
        .map(|maybe_id| maybe_id.into_iter().collect())
        .map_err(|error| SurfacePairingError::Write(error.to_string()))
}

pub fn validate_signed_session(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: SignedSessionValidationInput,
) -> Result<ValidatedSurfaceSession, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let now = format_ts(input.now);
    let presented_site_binding_digest =
        SiteClaims::from_signed(&input.site_claims)?.site_binding_digest();
    let row = load_session_pairing(db, &input.session_id, &input.surface_client_id)?
        .ok_or(SurfacePairingError::SessionInvalid)?;

    if row.runtime_anchor_id != input.runtime_anchor_id {
        return Err(SurfacePairingError::UnknownRuntimeAnchor);
    }
    if ts_before_or_equal(&row.inactive_expires_at, &now)
        || ts_before_or_equal(&row.absolute_expires_at, &now)
    {
        mark_session_revoked(db, &input.session_id, &now, "session_expired")?;
        return Err(SurfacePairingError::SessionExpired);
    }
    if row.pairing_epoch < row.epoch_floor {
        return Err(SurfacePairingError::RestoredStalePairing);
    }
    if row.revocation_id.is_some() {
        return Err(SurfacePairingError::PairingRevoked);
    }
    match row.lifecycle_state.as_str() {
        "active" => {}
        "suspended" => return Err(SurfacePairingError::PairingSuspended),
        "revoked" => return Err(SurfacePairingError::PairingRevoked),
        "expired" => return Err(SurfacePairingError::PairingExpired),
        _ => return Err(SurfacePairingError::SessionInvalid),
    }
    if row.session_revoked_at.is_some() {
        return Err(SurfacePairingError::SessionInvalid);
    }
    if row
        .throttled_until_at
        .as_ref()
        .is_some_and(|until| now.as_str() < until.as_str())
    {
        return Err(SurfacePairingError::SessionThrottled);
    }
    if ts_before_or_equal(&row.pairing_expires_at, &now) {
        mark_pairing_expired(db, &row.surface_client_id, &now)?;
        return Err(SurfacePairingError::PairingExpired);
    }
    if row.site_nonce != input.site_nonce {
        suspend_pairing(db, &row.surface_client_id, &now, "site_nonce_mismatch")?;
        return Err(SurfacePairingError::SiteBindingMismatch);
    }
    if row.site_binding_digest != presented_site_binding_digest {
        suspend_pairing(db, &row.surface_client_id, &now, "site_binding_mismatch")?;
        return Err(SurfacePairingError::SiteBindingMismatch);
    }

    let scopes = scopes_from_json(&row.scopes_json)?;
    let scope_set = scope_set_from_strings(&scopes)?;
    if row.wp_user_hash != input.wp_user_hash {
        suspend_pairing(db, &row.surface_client_id, &now, "wp_user_mismatch")?;
        return Err(SurfacePairingError::WpUserMismatch);
    }
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE surface_client_sessions
                 SET last_seen_at = ?1,
                     inactive_expires_at = ?2
                 WHERE session_id = ?3",
                params![
                    now,
                    format_ts(input.now + Duration::seconds(SESSION_INACTIVE_TTL_SECONDS)),
                    input.session_id
                ],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "UPDATE surface_client_pairings
                 SET last_used_at = ?1
                 WHERE surface_client_id = ?2",
                params![now, row.surface_client_id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(SurfacePairingError::Write)?;

    Ok(ValidatedSurfaceSession {
        surface_client_id: row.surface_client_id.clone(),
        session_id: input.session_id,
        actor: Actor::SurfaceClient {
            instance: SurfaceClientId::new(row.surface_client_id.clone()),
            scopes: scope_set,
        },
        wp_user_id: Some(input.wp_user_id),
        wp_user_hash: Some(input.wp_user_hash),
        wp_site_id: input.site_claims.wp_site_id.clone(),
        wp_site_id_hash: private_audit_hash(
            "wp_site_id",
            &row.site_nonce,
            &input.site_claims.wp_site_id,
        ),
        site_binding_digest: row.site_binding_digest,
        site_nonce: row.site_nonce,
        scope_digest: row.scope_digest,
        granted_scopes: scopes,
    })
}

/// Read-only signed-session validation per W4-F (DOS-655).
///
/// Performs ONLY SELECTs. Returns a `SignedSessionFailure` enum identifying
/// which check tripped; the caller (dispatch handler) decides whether to
/// escalate to a writer-mutex acquisition via `apply_signed_session_write_action`
/// for the 5 quarantine-write variants, or return the error directly for the
/// 6 no-write variants.
///
/// Session validity rule (V3.1 §7 #7): `revoked_at IS NULL AND
/// absolute_expires_at > ?now AND lifecycle_state = 'active'`.
/// `inactive_expires_at` is NOT consulted (retained for forensic preservation
/// per migration v180; see W4-F §6.8b for v179 rollback note).
pub fn validate_signed_session_readonly(
    db: &ActionDb,
    input: SignedSessionValidationInput,
) -> Result<ValidatedSurfaceSession, SignedSessionFailure> {
    let now = format_ts(input.now);
    let presented_site_binding_digest = SiteClaims::from_signed(&input.site_claims)
        .map_err(|_| SignedSessionFailure::SessionInvalid)?
        .site_binding_digest();
    let row = match load_session_pairing(db, &input.session_id, &input.surface_client_id) {
        Ok(Some(r)) => r,
        Ok(None) => return Err(SignedSessionFailure::SessionInvalid),
        Err(_) => return Err(SignedSessionFailure::SessionInvalid),
    };

    if row.runtime_anchor_id != input.runtime_anchor_id {
        return Err(SignedSessionFailure::UnknownRuntimeAnchor);
    }
    // W4-F V3.1 §7 #7: only absolute_expires_at gates validity. inactive_expires_at
    // is no longer consulted (V3 §6.8b rollback note: v179 still does, so rollback
    // requires re-pair for rows with stale inactive_expires_at).
    if ts_before_or_equal(&row.absolute_expires_at, &now) {
        return Err(SignedSessionFailure::SessionExpired {
            session_id: input.session_id.clone(),
        });
    }
    if row.pairing_epoch < row.epoch_floor {
        return Err(SignedSessionFailure::RestoredStalePairing);
    }
    if row.revocation_id.is_some() {
        return Err(SignedSessionFailure::PairingRevoked);
    }
    match row.lifecycle_state.as_str() {
        "active" => {}
        "suspended" => return Err(SignedSessionFailure::PairingSuspended),
        "revoked" => return Err(SignedSessionFailure::PairingRevoked),
        "expired" => {
            return Err(SignedSessionFailure::PairingExpired {
                surface_client_id: row.surface_client_id.clone(),
            });
        }
        _ => return Err(SignedSessionFailure::SessionInvalid),
    }
    if row.session_revoked_at.is_some() {
        return Err(SignedSessionFailure::SessionInvalid);
    }
    if row
        .throttled_until_at
        .as_ref()
        .is_some_and(|until| now.as_str() < until.as_str())
    {
        return Err(SignedSessionFailure::SessionThrottled);
    }
    if ts_before_or_equal(&row.pairing_expires_at, &now) {
        return Err(SignedSessionFailure::PairingExpired {
            surface_client_id: row.surface_client_id.clone(),
        });
    }
    if row.site_nonce != input.site_nonce {
        return Err(SignedSessionFailure::SiteNonceMismatch {
            surface_client_id: row.surface_client_id.clone(),
        });
    }
    if row.site_binding_digest != presented_site_binding_digest {
        return Err(SignedSessionFailure::SiteBindingDigestMismatch {
            surface_client_id: row.surface_client_id.clone(),
        });
    }

    let scopes =
        scopes_from_json(&row.scopes_json).map_err(|_| SignedSessionFailure::SessionInvalid)?;
    let scope_set =
        scope_set_from_strings(&scopes).map_err(|_| SignedSessionFailure::SessionInvalid)?;
    if row.wp_user_hash != input.wp_user_hash {
        return Err(SignedSessionFailure::WpUserHashMismatch {
            surface_client_id: row.surface_client_id.clone(),
        });
    }

    // W4-F V3.2 §6.8: NO writes on Ok-path. last_seen_at and last_used_at
    // are lazy-flushed on graceful shutdown only (not implemented in this
    // commit; staged for follow-up). The cycle-1 Ok-path writes at the old
    // line 774-798 are removed.
    Ok(ValidatedSurfaceSession {
        surface_client_id: row.surface_client_id.clone(),
        session_id: input.session_id,
        actor: Actor::SurfaceClient {
            instance: SurfaceClientId::new(row.surface_client_id.clone()),
            scopes: scope_set,
        },
        wp_user_id: Some(input.wp_user_id),
        wp_user_hash: Some(input.wp_user_hash),
        wp_site_id: input.site_claims.wp_site_id.clone(),
        wp_site_id_hash: private_audit_hash(
            "wp_site_id",
            &row.site_nonce,
            &input.site_claims.wp_site_id,
        ),
        site_binding_digest: row.site_binding_digest,
        site_nonce: row.site_nonce,
        scope_digest: row.scope_digest,
        granted_scopes: scopes,
    })
}

/// Routing enum returned by [`validate_signed_session_readonly`] (W4-F V3.1 §5).
///
/// Carries enough context for the dispatch handler to:
/// - Determine if a writer-mutex acquisition is needed (`write_action()`).
/// - Convert to `SurfacePairingError` for the wire response (`to_pairing_error()`).
///
/// Per V3.2: the 5 quarantine-write variants (`SessionExpired`, `PairingExpired`,
/// `SiteNonceMismatch`, `SiteBindingDigestMismatch`, `WpUserHashMismatch`) split
/// out the current-code `SiteBindingMismatch` variant into its two underlying
/// checks. The dispatch handler maps both `SiteNonceMismatch` and
/// `SiteBindingDigestMismatch` back to `SurfacePairingError::SiteBindingMismatch`
/// for wire compatibility.
#[derive(Debug, Clone)]
pub enum SignedSessionFailure {
    UnknownRuntimeAnchor,
    SessionExpired { session_id: String },
    RestoredStalePairing,
    PairingRevoked,
    PairingSuspended,
    PairingExpired { surface_client_id: String },
    SessionInvalid,
    SessionThrottled,
    SiteNonceMismatch { surface_client_id: String },
    SiteBindingDigestMismatch { surface_client_id: String },
    WpUserHashMismatch { surface_client_id: String },
}

/// Writer action escalated from the dispatch handler's Err arm to a fresh
/// `db_write` acquisition (W4-F V3 §6.9). The 5 variants here correspond to
/// the 5 write-needing `SignedSessionFailure` variants.
#[derive(Debug, Clone)]
pub enum SignedSessionWriteAction {
    MarkSessionRevoked {
        session_id: String,
        reason: &'static str,
    },
    MarkPairingExpired {
        surface_client_id: String,
    },
    SuspendPairing {
        surface_client_id: String,
        reason: &'static str,
    },
}

impl SignedSessionFailure {
    /// Returns `Some(action)` for the 5 quarantine-write variants; `None`
    /// for the 6 no-write variants. The exhaustive match (no wildcard) is
    /// the V3.1 §9.11 enforcement — new variants must declare their write
    /// footprint or fail to compile.
    pub fn write_action(&self) -> Option<SignedSessionWriteAction> {
        match self {
            Self::SessionExpired { session_id } => {
                Some(SignedSessionWriteAction::MarkSessionRevoked {
                    session_id: session_id.clone(),
                    reason: "session_expired",
                })
            }
            Self::PairingExpired { surface_client_id } => {
                Some(SignedSessionWriteAction::MarkPairingExpired {
                    surface_client_id: surface_client_id.clone(),
                })
            }
            Self::SiteNonceMismatch { surface_client_id } => {
                Some(SignedSessionWriteAction::SuspendPairing {
                    surface_client_id: surface_client_id.clone(),
                    reason: "site_nonce_mismatch",
                })
            }
            Self::SiteBindingDigestMismatch { surface_client_id } => {
                Some(SignedSessionWriteAction::SuspendPairing {
                    surface_client_id: surface_client_id.clone(),
                    reason: "site_binding_mismatch",
                })
            }
            Self::WpUserHashMismatch { surface_client_id } => {
                Some(SignedSessionWriteAction::SuspendPairing {
                    surface_client_id: surface_client_id.clone(),
                    reason: "wp_user_mismatch",
                })
            }
            Self::UnknownRuntimeAnchor
            | Self::RestoredStalePairing
            | Self::PairingRevoked
            | Self::PairingSuspended
            | Self::SessionInvalid
            | Self::SessionThrottled => None,
        }
    }

    /// Convert to the wire-error type. The two split variants
    /// (`SiteNonceMismatch`, `SiteBindingDigestMismatch`) both map back to
    /// `SurfacePairingError::SiteBindingMismatch` for wire compatibility —
    /// the split exists only for internal write-action routing.
    pub fn to_pairing_error(&self) -> SurfacePairingError {
        match self {
            Self::UnknownRuntimeAnchor => SurfacePairingError::UnknownRuntimeAnchor,
            Self::SessionExpired { .. } => SurfacePairingError::SessionExpired,
            Self::RestoredStalePairing => SurfacePairingError::RestoredStalePairing,
            Self::PairingRevoked => SurfacePairingError::PairingRevoked,
            Self::PairingSuspended => SurfacePairingError::PairingSuspended,
            Self::PairingExpired { .. } => SurfacePairingError::PairingExpired,
            Self::SessionInvalid => SurfacePairingError::SessionInvalid,
            Self::SessionThrottled => SurfacePairingError::SessionThrottled,
            Self::SiteNonceMismatch { .. } => SurfacePairingError::SiteBindingMismatch,
            Self::SiteBindingDigestMismatch { .. } => SurfacePairingError::SiteBindingMismatch,
            Self::WpUserHashMismatch { .. } => SurfacePairingError::WpUserMismatch,
        }
    }
}

/// Apply a write action escalated from a failed [`validate_signed_session_readonly`].
/// Called by the dispatch handler's Err arm inside a fresh `db_write` block.
pub fn apply_signed_session_write_action(
    db: &ActionDb,
    action: SignedSessionWriteAction,
    now: DateTime<Utc>,
) -> Result<(), SurfacePairingError> {
    let now_str = format_ts(now);
    match action {
        SignedSessionWriteAction::MarkSessionRevoked { session_id, reason } => {
            mark_session_revoked(db, &session_id, &now_str, reason)
        }
        SignedSessionWriteAction::MarkPairingExpired { surface_client_id } => {
            mark_pairing_expired(db, &surface_client_id, &now_str)
        }
        SignedSessionWriteAction::SuspendPairing {
            surface_client_id,
            reason,
        } => suspend_pairing(db, &surface_client_id, &now_str, reason),
    }
}

/// Look up the scope set granted to a paired surface_client for audit attribution.
///
/// Used by transport-layer rejection paths that have an HMAC-verified request
/// but where `validate_signed_session_readonly` failed before producing a
/// `ValidatedSurfaceSession`. Returns `None` on any error (pairing row gone,
/// scopes_json corrupted, etc.) — callers fall back to `Actor::System`
/// attribution in that case.
pub fn load_session_scope_set_for_audit(
    db: &ActionDb,
    session_id: &str,
    surface_client_id: &str,
) -> Option<ScopeSet> {
    let row = load_session_pairing(db, session_id, surface_client_id)
        .ok()
        .flatten()?;
    let scopes = scopes_from_json(&row.scopes_json).ok()?;
    scope_set_from_strings(&scopes).ok()
}

pub fn verify_session_refresh_identity(
    db: &ActionDb,
    input: SurfaceSessionRefreshInput,
) -> Result<SurfaceSessionRefreshIdentity, SurfacePairingError> {
    struct RefreshIdentityRow {
        site_binding_digest: String,
        wp_install_uuid_hash: String,
        plugin_instance_uuid_hash: String,
    }

    let row = db
        .conn_ref()
        .query_row(
            "SELECT p.site_binding_digest, p.wp_install_uuid_hash, p.plugin_instance_uuid_hash
             FROM surface_client_sessions s
             JOIN surface_client_pairings p
               ON p.surface_client_id = s.surface_client_id
              AND p.pairing_epoch = s.pairing_epoch
             WHERE s.session_id = ?1
               AND s.revoked_at IS NULL
               AND p.lifecycle_state = 'active'
               AND p.revoked_at IS NULL",
            params![input.session_id],
            |row| {
                Ok(RefreshIdentityRow {
                    site_binding_digest: row.get("site_binding_digest")?,
                    wp_install_uuid_hash: row.get("wp_install_uuid_hash")?,
                    plugin_instance_uuid_hash: row.get("plugin_instance_uuid_hash")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let Some(row) = row else {
        return Ok(SurfaceSessionRefreshIdentity::SessionNotFound);
    };

    if row.site_binding_digest != input.site_binding_digest
        || row.wp_install_uuid_hash != stable_hash("wp_install_uuid", &input.wp_install_uuid)
        || row.plugin_instance_uuid_hash
            != stable_hash("plugin_instance_uuid", &input.plugin_instance_uuid)
    {
        return Ok(SurfaceSessionRefreshIdentity::IdentityMismatch);
    }

    Ok(SurfaceSessionRefreshIdentity::Matched)
}

pub fn list_pairings(
    db: &ActionDb,
) -> Result<Vec<SurfaceClientPairingSummary>, SurfacePairingError> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT surface_client_id, site_binding_digest, scope_digest, lifecycle_state,
                    created_at, last_used_at, expires_at, revoked_at
             FROM surface_client_pairings
             ORDER BY datetime(created_at) DESC",
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let surface_client_id: String = row.get("surface_client_id")?;
            Ok(SurfaceClientPairingSummary {
                surface_client_display_id: display_id(&surface_client_id),
                surface_client_id,
                site_binding_digest: display_hash(row.get::<_, String>("site_binding_digest")?),
                scope_digest: display_hash(row.get::<_, String>("scope_digest")?),
                lifecycle_state: row.get("lifecycle_state")?,
                created_at: row.get("created_at")?,
                last_used_at: row.get("last_used_at")?,
                expires_at: row.get("expires_at")?,
                revoked_at: row.get("revoked_at")?,
            })
        })
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(rows)
}

pub fn revoke_pairing(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: RevokePairingInput,
) -> Result<SurfacePairingAuditEvent, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let now = format_ts(input.now);
    let row = db
        .conn_ref()
        .query_row(
            "SELECT pairing_id, surface_client_id, pairing_epoch, runtime_anchor_id,
                    site_binding_digest, scope_digest, previous_pairing_id,
                    site_binding_claims_json,
                    (
                        SELECT s.wp_user_hash
                        FROM surface_client_sessions s
                        WHERE s.surface_client_id = p.surface_client_id
                          AND s.pairing_epoch = p.pairing_epoch
                        ORDER BY datetime(s.issued_at) DESC
                        LIMIT 1
                    ) AS wp_user_hash
             FROM surface_client_pairings p
             WHERE p.surface_client_id = ?1
             ORDER BY p.pairing_epoch DESC
             LIMIT 1",
            params![input.surface_client_id],
            |row| {
                Ok(RevocationTarget {
                    pairing_id: row.get("pairing_id")?,
                    surface_client_id: row.get("surface_client_id")?,
                    pairing_epoch: row.get("pairing_epoch")?,
                    runtime_anchor_id: row.get("runtime_anchor_id")?,
                    site_binding_digest: row.get("site_binding_digest")?,
                    scope_digest: row.get("scope_digest")?,
                    previous_pairing_id: row.get("previous_pairing_id")?,
                    site_binding_claims_json: row.get("site_binding_claims_json")?,
                    wp_user_hash: row.get("wp_user_hash")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?
        .ok_or(SurfacePairingError::SessionInvalid)?;
    db.with_transaction(|tx| {
        revoke_pairing_row(tx, &row, &now, &input.reason).map_err(|error| error.to_string())
    })
    .map_err(SurfacePairingError::Write)?;
    Ok(pairing_revoked_audit_event(
        RevocationAuditInput {
            surface_client_id: &row.surface_client_id,
            pairing_epoch: row.pairing_epoch,
            site_binding_digest: &row.site_binding_digest,
            scope_digest: &row.scope_digest,
            site_binding_claims_json: row.site_binding_claims_json.as_deref(),
            stored_wp_user_hash: row.wp_user_hash.as_deref(),
            reason: &input.reason,
        },
        Actor::User,
        None,
        None,
    ))
}

pub fn record_signed_transport_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: SignedTransportFailureInput,
) -> Result<Vec<SurfacePairingAuditEvent>, SurfacePairingError> {
    ctx.check_mutation_allowed()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let now = format_ts(input.now);
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "INSERT INTO surface_client_session_failures (
                    id, session_id, surface_client_id, failure_code, occurred_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    input.session_id,
                    input.surface_client_id,
                    input.failure_code,
                    now
                ],
            )
            .map_err(|error| error.to_string())?;
        tx.conn_ref()
            .execute(
                "DELETE FROM surface_client_session_failures
                 WHERE datetime(occurred_at) < datetime(?1, '-5 minutes')",
                params![now],
            )
            .map_err(|error| error.to_string())?;

        let mut events = Vec::new();
        let suspicious_count: i64 = tx
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM surface_client_session_failures
                 WHERE session_id = ?1
                   AND failure_code IN ('nonce_replay', 'canonicalization_mismatch', 'timestamp_future')
                   AND datetime(occurred_at) >= datetime(?2, '-60 seconds')",
                params![input.session_id, now],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if suspicious_count >= 3 {
            if let Some(row) = target_for_session(tx, &input.session_id)? {
                let throttled_until = format_ts(
                    input.now + Duration::seconds(SESSION_SUSPICIOUS_THROTTLE_SECONDS),
                );
                tx.conn_ref()
                    .execute(
                        "UPDATE surface_client_sessions
                         SET throttled_until_at = CASE
                             WHEN throttled_until_at IS NULL OR throttled_until_at < ?1 THEN ?1
                             ELSE throttled_until_at
                         END
                         WHERE session_id = ?2
                           AND revoked_at IS NULL",
                        params![throttled_until, input.session_id],
                    )
                    .map_err(|error| error.to_string())?;
                events.push(SurfacePairingAuditEvent {
                    event_kind: "pairing.exfiltration.suspected_replay",
                    category: "security",
                    actor: Actor::System,
                    wp_user_id: None,
                    wp_user_hash: None,
                    detail: json!({
                        "surface_client_id": row.surface_client_id,
                        "site_binding_digest": row.site_binding_digest,
                        "scope_digest": row.scope_digest,
                        "failure_code": input.failure_code,
                        "failure_count": suspicious_count,
                        "decision": "throttle"
                    }),
                });
            }
        }

        if input.failure_code == "nonce_replay" {
            let replay_count: i64 = tx
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*)
                     FROM surface_client_session_failures
                     WHERE session_id = ?1
                       AND failure_code = 'nonce_replay'
                       AND datetime(occurred_at) >= datetime(?2, '-5 minutes')",
                    params![input.session_id, now],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if replay_count >= 5 {
                if let Some(row) = target_for_session(tx, &input.session_id)? {
                    revoke_pairing_row(tx, &row, &now, "suspicious_replay")?;
                    events.push(pairing_revoked_audit_event(
                        RevocationAuditInput {
                            surface_client_id: &row.surface_client_id,
                            pairing_epoch: row.pairing_epoch,
                            site_binding_digest: &row.site_binding_digest,
                            scope_digest: &row.scope_digest,
                            site_binding_claims_json: row.site_binding_claims_json.as_deref(),
                            stored_wp_user_hash: row.wp_user_hash.as_deref(),
                            reason: "suspicious_replay",
                        },
                        Actor::System,
                        None,
                        None,
                    ));
                }
            }
        }

        Ok(events)
    })
    .map_err(SurfacePairingError::Write)
}

pub fn emit_pairing_audit(
    logger: &mut AuditLogger,
    event: &SurfacePairingAuditEvent,
) -> Result<(), String> {
    let mut fields = AuditFields::new(event.category, event.detail.clone());
    if let Some(wp_user_id) = event.wp_user_id {
        fields = fields.with_wp_user_id(wp_user_id);
    }
    if let Some(wp_user_hash) = event.wp_user_hash.as_ref() {
        fields = fields.with_wp_user_hash(wp_user_hash.clone());
    }
    emit_surface_audit(logger, event.event_kind, &event.actor, fields)
        .map_err(|error| error.to_string())
}

struct RevocationAuditInput<'a> {
    surface_client_id: &'a str,
    pairing_epoch: i64,
    site_binding_digest: &'a str,
    scope_digest: &'a str,
    site_binding_claims_json: Option<&'a str>,
    stored_wp_user_hash: Option<&'a str>,
    reason: &'a str,
}

fn pairing_revoked_audit_event(
    input: RevocationAuditInput<'_>,
    actor: Actor,
    wp_user_id: Option<u64>,
    wp_user_hash: Option<String>,
) -> SurfacePairingAuditEvent {
    let mut detail = json!({
        "surface_client_id": input.surface_client_id,
        "site_binding_digest": input.site_binding_digest,
        "scope_digest": input.scope_digest,
        "pairing_epoch": input.pairing_epoch,
        "reason": input.reason,
        "decision": "revoked"
    });
    if let Some(wp_site_id_hash) = wp_site_id_hash_from_claims_json(input.site_binding_claims_json)
    {
        detail["wp_site_id_hash"] = json!(wp_site_id_hash);
    }
    if let Some(stored_wp_user_hash) = input.stored_wp_user_hash {
        detail["wp_user_hash"] = json!(stored_wp_user_hash);
    }
    SurfacePairingAuditEvent {
        event_kind: "pairing_revoked",
        category: "security",
        actor,
        wp_user_id,
        wp_user_hash,
        detail,
    }
}

pub fn authorized_ability_projection(
    granted_scopes: &[String],
) -> Result<Vec<SurfaceAbilityProjection>, SurfacePairingError> {
    let scope_set = scope_set_from_strings(granted_scopes)?;
    Ok(ability_projection_for_scopes(&scope_set))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SiteClaims {
    wp_site_id: String,
    home_url: String,
    site_url: String,
    wp_install_uuid: String,
    plugin_instance_uuid: String,
    multisite_blog_id: Option<String>,
}

impl SiteClaims {
    fn from_request(request: &PairingHandshakeRequest) -> Result<Self, SurfacePairingError> {
        Self::from_parts(SiteClaimParts {
            wp_site_id: &request.wp_site_id,
            home_url: &request.home_url,
            site_url: &request.site_url,
            wp_install_uuid: &request.wp_install_uuid,
            plugin_instance_uuid: &request.plugin_instance_uuid,
            multisite_blog_id: request.multisite_blog_id.as_deref(),
        })
    }

    fn from_signed(input: &SignedSiteClaimsInput) -> Result<Self, SurfacePairingError> {
        Self::from_parts(SiteClaimParts {
            wp_site_id: &input.wp_site_id,
            home_url: &input.home_url,
            site_url: &input.site_url,
            wp_install_uuid: &input.wp_install_uuid,
            plugin_instance_uuid: &input.plugin_instance_uuid,
            multisite_blog_id: input.multisite_blog_id.as_deref(),
        })
    }

    fn from_parts(parts: SiteClaimParts<'_>) -> Result<Self, SurfacePairingError> {
        let home_url = normalize_site_url(parts.home_url)
            .ok_or(SurfacePairingError::BadRequest("home_url_invalid"))?;
        let site_url = normalize_site_url(parts.site_url)
            .ok_or(SurfacePairingError::BadRequest("site_url_invalid"))?;
        Ok(Self {
            wp_site_id: sanitize_identifier(parts.wp_site_id, "wp_site_id_invalid")?,
            home_url,
            site_url,
            wp_install_uuid: sanitize_identifier(parts.wp_install_uuid, "wp_install_uuid_invalid")?,
            plugin_instance_uuid: sanitize_identifier(
                parts.plugin_instance_uuid,
                "plugin_instance_uuid_invalid",
            )?,
            multisite_blog_id: parts
                .multisite_blog_id
                .map(|value| sanitize_identifier(value, "multisite_blog_id_invalid"))
                .transpose()?,
        })
    }

    fn site_binding_digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"DAILYOS-SITE-BINDING-V1\n");
        hasher.update(self.wp_site_id.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.home_url.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.site_url.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.wp_install_uuid.as_bytes());
        hasher.update(b"\n");
        hasher.update(self.plugin_instance_uuid.as_bytes());
        hasher.update(b"\n");
        if let Some(blog_id) = &self.multisite_blog_id {
            hasher.update(blog_id.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    fn site_origin(&self) -> Option<String> {
        let parsed = url::Url::parse(&self.site_url).ok()?;
        Some(parsed.origin().unicode_serialization())
    }
}

struct SiteClaimParts<'a> {
    wp_site_id: &'a str,
    home_url: &'a str,
    site_url: &'a str,
    wp_install_uuid: &'a str,
    plugin_instance_uuid: &'a str,
    multisite_blog_id: Option<&'a str>,
}

#[derive(Debug)]
struct PairingCodeRow {
    endpoint_startup_id: String,
    bound_port: i64,
    expires_at: String,
    consumed_at: Option<String>,
    failed_attempt_count: i64,
}

#[derive(Debug)]
enum PairingCodeClaim {
    Consumed,
    Rejected(SurfacePairingError),
}

#[derive(Debug)]
struct SessionPairingRow {
    surface_client_id: String,
    runtime_anchor_id: String,
    pairing_epoch: i64,
    epoch_floor: i64,
    lifecycle_state: String,
    pairing_expires_at: String,
    session_revoked_at: Option<String>,
    revocation_id: Option<String>,
    inactive_expires_at: String,
    absolute_expires_at: String,
    throttled_until_at: Option<String>,
    site_binding_digest: String,
    site_nonce: String,
    scope_digest: String,
    scopes_json: String,
    wp_user_hash: String,
}

#[derive(Debug)]
struct RevocationTarget {
    pairing_id: String,
    surface_client_id: String,
    pairing_epoch: i64,
    runtime_anchor_id: String,
    site_binding_digest: String,
    scope_digest: String,
    previous_pairing_id: Option<String>,
    site_binding_claims_json: Option<String>,
    wp_user_hash: Option<String>,
}

#[derive(Debug, Clone)]
struct RevokedPairingRef {
    pairing_id: String,
    surface_client_id: String,
    pairing_epoch: i64,
    site_binding_digest: String,
    scope_digest: String,
    site_binding_claims_json: Option<String>,
    wp_user_hash: Option<String>,
}

fn consume_pairing_code(
    db: &ActionDb,
    code_hash: &str,
    input: &PairingHandshakeInput,
) -> Result<PairingCodeClaim, SurfacePairingError> {
    let now = format_ts(input.now);
    let row = db
        .conn_ref()
        .query_row(
            "SELECT endpoint_startup_id, bound_port, expires_at, consumed_at, failed_attempt_count
             FROM surface_pairing_codes
             WHERE code_hash = ?1",
            params![code_hash],
            |row| {
                Ok(PairingCodeRow {
                    endpoint_startup_id: row.get("endpoint_startup_id")?,
                    bound_port: row.get("bound_port")?,
                    expires_at: row.get("expires_at")?,
                    consumed_at: row.get("consumed_at")?,
                    failed_attempt_count: row.get("failed_attempt_count")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?
        .ok_or(SurfacePairingError::PairingCodeInvalid)?;

    if row.consumed_at.is_some() {
        return Ok(PairingCodeClaim::Rejected(
            SurfacePairingError::PairingCodeConsumed,
        ));
    }
    if ts_before_or_equal(&row.expires_at, &now) {
        consume_code(db, code_hash, &now)?;
        return Ok(PairingCodeClaim::Rejected(
            SurfacePairingError::PairingCodeExpired,
        ));
    }
    if row.failed_attempt_count >= i64::from(input.max_failed_attempts.max(1)) {
        consume_code(db, code_hash, &now)?;
        return Ok(PairingCodeClaim::Rejected(
            SurfacePairingError::PairingCodeLimited,
        ));
    }
    if row.endpoint_startup_id != input.endpoint_startup_id
        || row.bound_port != i64::from(input.bound_port)
    {
        if record_failed_code_attempt(db, code_hash, &now, input.max_failed_attempts)? {
            return Ok(PairingCodeClaim::Rejected(
                SurfacePairingError::PairingCodeLimited,
            ));
        }
        return Ok(PairingCodeClaim::Rejected(
            SurfacePairingError::PairingCodeInvalid,
        ));
    }

    consume_code(db, code_hash, &now)?;
    Ok(PairingCodeClaim::Consumed)
}

fn record_failed_handshake_for_input(
    db: &ActionDb,
    code_hash: &str,
    input: &PairingHandshakeInput,
) -> Result<Option<SurfacePairingError>, SurfacePairingError> {
    let now = format_ts(input.now);
    db.with_transaction(|tx| {
        record_pairing_code_failure_tx(
            tx,
            code_hash,
            &input.endpoint_startup_id,
            input.bound_port,
            &now,
            input.max_failed_attempts,
        )
    })
    .map_err(SurfacePairingError::Write)
}

fn record_pairing_code_failure_tx(
    db: &ActionDb,
    code_hash: &str,
    endpoint_startup_id: &str,
    bound_port: u16,
    now: &str,
    max_failed_attempts: u32,
) -> Result<Option<SurfacePairingError>, String> {
    let Some(row) = load_pairing_code_row(db, code_hash)? else {
        return Ok(Some(SurfacePairingError::PairingCodeInvalid));
    };
    if row.consumed_at.is_some() {
        return Ok(Some(SurfacePairingError::PairingCodeConsumed));
    }
    if ts_before_or_equal(&row.expires_at, now) {
        consume_code(db, code_hash, now).map_err(|error| error.to_string())?;
        return Ok(Some(SurfacePairingError::PairingCodeExpired));
    }
    if row.failed_attempt_count >= i64::from(max_failed_attempts.max(1)) {
        consume_code(db, code_hash, now).map_err(|error| error.to_string())?;
        return Ok(Some(SurfacePairingError::PairingCodeLimited));
    }
    if row.endpoint_startup_id != endpoint_startup_id || row.bound_port != i64::from(bound_port) {
        let limited = record_failed_code_attempt(db, code_hash, now, max_failed_attempts)
            .map_err(|error| error.to_string())?;
        return Ok(limited.then_some(SurfacePairingError::PairingCodeLimited));
    }
    let limited = record_failed_code_attempt(db, code_hash, now, max_failed_attempts)
        .map_err(|error| error.to_string())?;
    Ok(limited.then_some(SurfacePairingError::PairingCodeLimited))
}

fn load_pairing_code_row(db: &ActionDb, code_hash: &str) -> Result<Option<PairingCodeRow>, String> {
    db.conn_ref()
        .query_row(
            "SELECT endpoint_startup_id, bound_port, expires_at, consumed_at, failed_attempt_count
             FROM surface_pairing_codes
             WHERE code_hash = ?1",
            params![code_hash],
            |row| {
                Ok(PairingCodeRow {
                    endpoint_startup_id: row.get("endpoint_startup_id")?,
                    bound_port: row.get("bound_port")?,
                    expires_at: row.get("expires_at")?,
                    consumed_at: row.get("consumed_at")?,
                    failed_attempt_count: row.get("failed_attempt_count")?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn record_failed_code_attempt(
    db: &ActionDb,
    code_hash: &str,
    now: &str,
    max_attempts: u32,
) -> Result<bool, SurfacePairingError> {
    let changed = db
        .conn_ref()
        .execute(
            "UPDATE surface_pairing_codes
             SET failed_attempt_count = failed_attempt_count + 1,
                 last_failed_at = ?1,
                 consumed_at = CASE
                    WHEN failed_attempt_count + 1 >= ?2 THEN ?1
                    ELSE consumed_at
                 END
             WHERE code_hash = ?3",
            params![now, i64::from(max_attempts.max(1)), code_hash],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    if changed == 0 {
        return Err(SurfacePairingError::PairingCodeInvalid);
    }
    let count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT failed_attempt_count
             FROM surface_pairing_codes
             WHERE code_hash = ?1",
            params![code_hash],
            |row| row.get(0),
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(count >= i64::from(max_attempts.max(1)))
}

fn consume_code(db: &ActionDb, code_hash: &str, now: &str) -> Result<(), SurfacePairingError> {
    let changed = db
        .conn_ref()
        .execute(
            "UPDATE surface_pairing_codes
             SET consumed_at = ?1
             WHERE code_hash = ?2 AND consumed_at IS NULL",
            params![now, code_hash],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    if changed == 0 {
        return Err(SurfacePairingError::PairingCodeConsumed);
    }
    Ok(())
}

fn revoke_existing_pairing_for_site(
    db: &ActionDb,
    runtime_anchor_id: &str,
    site_binding_digest: &str,
    now: &DateTime<Utc>,
    reason: &str,
) -> Result<Option<RevokedPairingRef>, SurfacePairingError> {
    let previous: Option<RevocationTarget> = db
        .conn_ref()
        .query_row(
            "SELECT pairing_id, surface_client_id, pairing_epoch, runtime_anchor_id,
                    site_binding_digest, scope_digest, previous_pairing_id,
                    site_binding_claims_json,
                    (
                        SELECT s.wp_user_hash
                        FROM surface_client_sessions s
                        WHERE s.surface_client_id = p.surface_client_id
                          AND s.pairing_epoch = p.pairing_epoch
                        ORDER BY datetime(s.issued_at) DESC
                        LIMIT 1
                    ) AS wp_user_hash
             FROM surface_client_pairings p
             WHERE p.runtime_anchor_id = ?1
               AND p.site_binding_digest = ?2
               AND p.lifecycle_state IN ('active', 'suspended', 'issued')
             ORDER BY p.pairing_epoch DESC
             LIMIT 1",
            params![runtime_anchor_id, site_binding_digest],
            |row| {
                Ok(RevocationTarget {
                    pairing_id: row.get("pairing_id")?,
                    surface_client_id: row.get("surface_client_id")?,
                    pairing_epoch: row.get("pairing_epoch")?,
                    runtime_anchor_id: row.get("runtime_anchor_id")?,
                    site_binding_digest: row.get("site_binding_digest")?,
                    scope_digest: row.get("scope_digest")?,
                    previous_pairing_id: row.get("previous_pairing_id")?,
                    site_binding_claims_json: row.get("site_binding_claims_json")?,
                    wp_user_hash: row.get("wp_user_hash")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    if let Some(row) = previous {
        let previous_pairing_id = row.pairing_id.clone();
        let previous_surface_client_id = row.surface_client_id.clone();
        let previous_pairing_epoch = row.pairing_epoch;
        let previous_site_binding_digest = row.site_binding_digest.clone();
        let previous_scope_digest = row.scope_digest.clone();
        let previous_site_binding_claims_json = row.site_binding_claims_json.clone();
        let previous_wp_user_hash = row.wp_user_hash.clone();
        revoke_pairing_row(db, &row, &format_ts(*now), reason)?;
        Ok(Some(RevokedPairingRef {
            pairing_id: previous_pairing_id,
            surface_client_id: previous_surface_client_id,
            pairing_epoch: previous_pairing_epoch,
            site_binding_digest: previous_site_binding_digest,
            scope_digest: previous_scope_digest,
            site_binding_claims_json: previous_site_binding_claims_json,
            wp_user_hash: previous_wp_user_hash,
        }))
    } else {
        Ok(None)
    }
}

fn next_pairing_epoch(
    db: &ActionDb,
    runtime_anchor_id: &str,
    site_binding_digest: &str,
) -> Result<i64, SurfacePairingError> {
    let floor: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(MAX(highest_pairing_epoch), 0)
             FROM surface_client_epoch_floors
             WHERE runtime_anchor_id = ?1 AND site_binding_digest = ?2",
            params![runtime_anchor_id, site_binding_digest],
            |row| row.get(0),
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    let max_pairing: i64 = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(MAX(pairing_epoch), 0)
             FROM surface_client_pairings
             WHERE runtime_anchor_id = ?1 AND site_binding_digest = ?2",
            params![runtime_anchor_id, site_binding_digest],
            |row| row.get(0),
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(floor.max(max_pairing) + 1)
}

fn load_session_pairing(
    db: &ActionDb,
    session_id: &str,
    surface_client_id: &str,
) -> Result<Option<SessionPairingRow>, SurfacePairingError> {
    db.conn_ref()
        .query_row(
            "SELECT s.surface_client_id, p.runtime_anchor_id, s.pairing_epoch,
                    COALESCE(f.highest_pairing_epoch, 0) AS epoch_floor,
                    p.lifecycle_state, p.expires_at AS pairing_expires_at,
                    s.revoked_at AS session_revoked_at,
                    r.revocation_id,
                    s.inactive_expires_at, s.absolute_expires_at, s.throttled_until_at,
                    p.site_binding_digest, p.site_nonce, p.scope_digest, p.scopes_json,
                    s.wp_user_hash
             FROM surface_client_sessions s
             JOIN surface_client_pairings p
               ON p.surface_client_id = s.surface_client_id
              AND p.pairing_epoch = s.pairing_epoch
             LEFT JOIN surface_client_epoch_floors f
               ON f.runtime_anchor_id = p.runtime_anchor_id
              AND f.site_binding_digest = p.site_binding_digest
             LEFT JOIN surface_client_revocations r
               ON r.surface_client_id = p.surface_client_id
              AND r.pairing_epoch = p.pairing_epoch
             WHERE s.session_id = ?1 AND s.surface_client_id = ?2",
            params![session_id, surface_client_id],
            |row| {
                Ok(SessionPairingRow {
                    surface_client_id: row.get("surface_client_id")?,
                    runtime_anchor_id: row.get("runtime_anchor_id")?,
                    pairing_epoch: row.get("pairing_epoch")?,
                    epoch_floor: row.get("epoch_floor")?,
                    lifecycle_state: row.get("lifecycle_state")?,
                    pairing_expires_at: row.get("pairing_expires_at")?,
                    session_revoked_at: row.get("session_revoked_at")?,
                    revocation_id: row.get("revocation_id")?,
                    inactive_expires_at: row.get("inactive_expires_at")?,
                    absolute_expires_at: row.get("absolute_expires_at")?,
                    throttled_until_at: row.get("throttled_until_at")?,
                    site_binding_digest: row.get("site_binding_digest")?,
                    site_nonce: row.get("site_nonce")?,
                    scope_digest: row.get("scope_digest")?,
                    scopes_json: row.get("scopes_json")?,
                    wp_user_hash: row.get("wp_user_hash")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))
}

fn target_for_session(
    db: &ActionDb,
    session_id: &str,
) -> Result<Option<RevocationTarget>, SurfacePairingError> {
    db.conn_ref()
        .query_row(
            "SELECT p.pairing_id, p.surface_client_id, p.pairing_epoch, p.runtime_anchor_id,
                    p.site_binding_digest, p.scope_digest, p.previous_pairing_id,
                    p.site_binding_claims_json,
                    (
                        SELECT s2.wp_user_hash
                        FROM surface_client_sessions s2
                        WHERE s2.surface_client_id = p.surface_client_id
                          AND s2.pairing_epoch = p.pairing_epoch
                        ORDER BY datetime(s2.issued_at) DESC
                        LIMIT 1
                    ) AS wp_user_hash
             FROM surface_client_sessions s
             JOIN surface_client_pairings p
               ON p.surface_client_id = s.surface_client_id
              AND p.pairing_epoch = s.pairing_epoch
             WHERE s.session_id = ?1",
            params![session_id],
            |row| {
                Ok(RevocationTarget {
                    pairing_id: row.get("pairing_id")?,
                    surface_client_id: row.get("surface_client_id")?,
                    pairing_epoch: row.get("pairing_epoch")?,
                    runtime_anchor_id: row.get("runtime_anchor_id")?,
                    site_binding_digest: row.get("site_binding_digest")?,
                    scope_digest: row.get("scope_digest")?,
                    previous_pairing_id: row.get("previous_pairing_id")?,
                    site_binding_claims_json: row.get("site_binding_claims_json")?,
                    wp_user_hash: row.get("wp_user_hash")?,
                })
            },
        )
        .optional()
        .map_err(|error| SurfacePairingError::Write(error.to_string()))
}

fn revoke_pairing_row(
    db: &ActionDb,
    row: &RevocationTarget,
    now: &str,
    reason: &str,
) -> Result<(), SurfacePairingError> {
    db.conn_ref()
        .execute(
            "UPDATE surface_client_pairings
             SET lifecycle_state = 'revoked',
                 revoked_at = ?1,
                 revoked_reason = ?2
             WHERE surface_client_id = ?3
               AND lifecycle_state != 'revoked'",
            params![now, reason, row.surface_client_id],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    db.conn_ref()
        .execute(
            "UPDATE surface_client_sessions
             SET revoked_at = ?1,
                 revoked_reason = ?2
             WHERE surface_client_id = ?3
               AND revoked_at IS NULL",
            params![now, reason, row.surface_client_id],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    db.conn_ref()
        .execute(
            "INSERT INTO surface_client_revocations (
                revocation_id, surface_client_id, pairing_epoch, runtime_anchor_id,
                site_binding_digest, scope_digest, revoked_at, reason, previous_pairing_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                Uuid::new_v4().to_string(),
                row.surface_client_id,
                row.pairing_epoch,
                row.runtime_anchor_id,
                row.site_binding_digest,
                row.scope_digest,
                now,
                reason,
                row.previous_pairing_id
            ],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(())
}

fn mark_session_revoked(
    db: &ActionDb,
    session_id: &str,
    now: &str,
    reason: &str,
) -> Result<(), SurfacePairingError> {
    db.conn_ref()
        .execute(
            "UPDATE surface_client_sessions
             SET revoked_at = ?1, revoked_reason = ?2
             WHERE session_id = ?3 AND revoked_at IS NULL",
            params![now, reason, session_id],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(())
}

fn mark_pairing_expired(
    db: &ActionDb,
    surface_client_id: &str,
    now: &str,
) -> Result<(), SurfacePairingError> {
    db.conn_ref()
        .execute(
            "UPDATE surface_client_pairings
             SET lifecycle_state = 'expired'
             WHERE surface_client_id = ?1
               AND lifecycle_state = 'active'
               AND datetime(expires_at) <= datetime(?2)",
            params![surface_client_id, now],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(())
}

fn suspend_pairing(
    db: &ActionDb,
    surface_client_id: &str,
    now: &str,
    reason: &str,
) -> Result<(), SurfacePairingError> {
    db.conn_ref()
        .execute(
            "UPDATE surface_client_pairings
             SET lifecycle_state = 'suspended',
                 revoked_reason = ?1,
                 last_used_at = ?2
             WHERE surface_client_id = ?3
               AND lifecycle_state = 'active'",
            params![reason, now, surface_client_id],
        )
        .map_err(|error| SurfacePairingError::Write(error.to_string()))?;
    Ok(())
}

fn default_granted_scopes() -> Vec<String> {
    DEFAULT_GRANTED_SCOPES
        .iter()
        .map(|scope| (*scope).to_string())
        .collect()
}

fn scope_set_from_strings(scopes: &[String]) -> Result<ScopeSet, SurfacePairingError> {
    ScopeSet::new(scopes.iter().map(|scope| SurfaceScope::new(scope.clone())))
        .map_err(|_| SurfacePairingError::ScopeDenied)
}

fn scopes_from_json(value: &str) -> Result<Vec<String>, SurfacePairingError> {
    serde_json::from_str::<Vec<String>>(value)
        .map_err(|error| SurfacePairingError::Write(error.to_string()))
}

fn ability_projection_for_scopes(scope_set: &ScopeSet) -> Vec<SurfaceAbilityProjection> {
    let Ok(registry) = abilities_runtime::abilities::registry::AbilityRegistry::global_checked()
    else {
        return Vec::new();
    };
    let actor = Actor::SurfaceClient {
        instance: SurfaceClientId::new("projection"),
        scopes: scope_set.clone(),
    };
    let mut projection = Vec::new();
    for descriptor in registry.iter_for(actor) {
        if !descriptor.policy.client_side_executable {
            continue;
        }
        let required = descriptor.policy.required_scopes_typed();
        if required.iter().all(|scope| scope_set.contains(scope)) {
            projection.push(SurfaceAbilityProjection {
                name_hash: stable_hash("ability_id", descriptor.name),
                required_scope_hashes: required
                    .iter()
                    .map(|scope| stable_hash("required_scope", scope.as_str()))
                    .collect(),
                client_side_executable: true,
            });
        }
    }
    projection
}

fn random_url_token(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_key32() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

fn pairing_code_hash(code: &str) -> String {
    stable_hash("pairing_code", code)
}

fn pairing_code_token(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with("dailyos://") {
        let url = url::Url::parse(value).ok()?;
        url.query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, value)| value.to_string())
            .filter(|code| is_pairing_token(code))
    } else if is_pairing_token(value) {
        Some(value.to_string())
    } else {
        None
    }
}

fn is_pairing_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn sanitize_identifier(value: &str, code: &'static str) -> Result<String, SurfacePairingError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > 160
        || !trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(SurfacePairingError::BadRequest(code));
    }
    Ok(trimmed.to_string())
}

fn normalize_site_url(value: &str) -> Option<String> {
    let mut url = url::Url::parse(value.trim()).ok()?;
    if url.cannot_be_a_base() {
        return None;
    }
    url.set_fragment(None);
    url.set_query(None);
    let scheme = url.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let port = url
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    let path = url.path().trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    Some(format!("{scheme}://{host}{port}{path}"))
}

fn scope_digest(scopes: &[String]) -> String {
    let sorted: BTreeSet<&str> = scopes.iter().map(String::as_str).collect();
    let mut hasher = Sha256::new();
    hasher.update(b"DAILYOS-SURFACE-SCOPE-DIGEST-V1\n");
    for scope in sorted {
        hasher.update(scope.as_bytes());
        hasher.update(b"\n");
    }
    hex::encode(hasher.finalize())
}

pub fn wp_user_hash(
    session_secret: &[u8; 32],
    site_binding_digest: &str,
    wp_user_id: u64,
) -> String {
    let key = ring::hmac::Key::new(ring::hmac::HMAC_SHA256, session_secret);
    let mut message = Vec::new();
    message.extend_from_slice(b"DAILYOS-WP-USER-HASH-V2\n");
    message.extend_from_slice(site_binding_digest.as_bytes());
    message.push(b'\n');
    message.extend_from_slice(wp_user_id.to_string().as_bytes());
    hex::encode(ring::hmac::sign(&key, &message).as_ref())
}

pub fn derive_session_hmac_key(master_key: [u8; 32], session_id: &str) -> [u8; 32] {
    struct SessionKeyLen;
    impl hkdf::KeyType for SessionKeyLen {
        fn len(&self) -> usize {
            HMAC_SESSION_KEY_BYTES
        }
    }

    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, session_id.as_bytes());
    let prk = salt.extract(master_key.as_slice());
    let okm = prk
        .expand(&[HMAC_SESSION_KEY_INFO], SessionKeyLen)
        .expect("fixed HKDF output length is valid");
    let mut key = [0_u8; HMAC_SESSION_KEY_BYTES];
    okm.fill(&mut key)
        .expect("fixed HKDF output buffer has the advertised length");
    key
}

fn wp_site_id_hash_from_claims_json(value: Option<&str>) -> Option<String> {
    let claims = serde_json::from_str::<SiteClaims>(value?).ok()?;
    Some(stable_hash("wp_site_id", &claims.wp_site_id))
}

fn stable_hash(domain: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"DAILYOS-SURFACE-HASH-V1\n");
    hasher.update(domain.as_bytes());
    hasher.update(b"\n");
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

fn private_audit_hash(domain: &str, secret: &str, value: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let mut context = hmac::Context::with_key(&key);
    context.update(b"DAILYOS-SURFACE-AUDIT-HASH-V1\n");
    context.update(domain.as_bytes());
    context.update(b"\n");
    context.update(value.as_bytes());
    format!(
        "hmac-sha256:{}",
        hex::encode(&context.sign().as_ref()[..16])
    )
}

fn display_hash(value: String) -> String {
    value.chars().take(12).collect()
}

fn display_id(value: &str) -> String {
    let suffix: String = value
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("Surface {suffix}")
}

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn ts_before_or_equal(left: &str, right: &str) -> bool {
    left <= right
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::registry::ScopeSet;
    use rusqlite::Connection;

    fn db() -> ActionDb {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        ActionDb::from_connection_for_tests(conn)
    }

    fn ctx_parts() -> (
        crate::services::context::SystemClock,
        crate::services::context::SeedableRng,
        crate::services::context::ExternalClients,
    ) {
        (
            crate::services::context::SystemClock,
            crate::services::context::SeedableRng::new(1),
            crate::services::context::ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a crate::services::context::SystemClock,
        rng: &'a crate::services::context::SeedableRng,
        external: &'a crate::services::context::ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::new_live(clock, rng, external)
    }

    fn request(code: String) -> PairingHandshakeRequest {
        PairingHandshakeRequest {
            pairing_code: code,
            wp_user_id: 42,
            wp_site_id: "site_1".to_string(),
            home_url: "https://subsidiary.com".to_string(),
            site_url: "https://subsidiary.com/wp".to_string(),
            wp_install_uuid: "install_1".to_string(),
            plugin_instance_uuid: "plugin_1".to_string(),
            multisite_blog_id: None,
            request_id: Some("req_test".to_string()),
            client_metadata: Some(json!({"plugin_version": "0.0.0"})),
        }
    }

    fn signed_site_claims() -> SignedSiteClaimsInput {
        SignedSiteClaimsInput {
            wp_site_id: "site_1".to_string(),
            home_url: "https://subsidiary.com".to_string(),
            site_url: "https://subsidiary.com/wp".to_string(),
            wp_install_uuid: "install_1".to_string(),
            plugin_instance_uuid: "plugin_1".to_string(),
            multisite_blog_id: None,
        }
    }

    fn allow_surface_scopes() {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
    }

    fn issue_code(ctx: &ServiceContext<'_>, db: &ActionDb, now: DateTime<Utc>) -> PairingCodeIssue {
        issue_pairing_code(
            ctx,
            db,
            PairingCodeIssueInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                now,
            },
        )
        .unwrap()
    }

    fn complete_test_handshake(
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        now: DateTime<Utc>,
        code: String,
    ) -> PairingHandshakeOutcome {
        complete_handshake(
            ctx,
            db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request(code),
            },
        )
        .unwrap()
    }

    #[test]
    fn capacity_preflight_reports_same_site_pairing_that_handshake_would_replace() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let first =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);

        let mut same_site_request = request("dailyos://pair?port=4411&code=not_consumed".into());
        let replaceable = replaceable_surface_client_ids_for_handshake(
            &ctx,
            &db,
            PairingHandshakeCapacityInput {
                runtime_anchor_id: "anchor_1".into(),
                request: same_site_request.clone(),
            },
        )
        .unwrap();
        assert_eq!(replaceable, vec![first.session.surface_client_id]);

        same_site_request.plugin_instance_uuid = "plugin_2".to_string();
        let not_replaceable = replaceable_surface_client_ids_for_handshake(
            &ctx,
            &db,
            PairingHandshakeCapacityInput {
                runtime_anchor_id: "anchor_1".into(),
                request: same_site_request,
            },
        )
        .unwrap();
        assert!(not_replaceable.is_empty());
    }

    #[test]
    fn empty_pairing_code_is_rejected_before_pairing_authority_is_created() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();

        let err = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request("".to_string()),
            },
        )
        .unwrap_err();

        assert_eq!(err, SurfacePairingError::BadRequest("pairing_code_invalid"));
        assert!(list_pairings(&db).unwrap().is_empty());
    }

    #[test]
    fn handshake_request_rejects_null_wp_site_id_at_wire_boundary() {
        let raw = json!({
            "pairing_code": "dailyos://pair?port=4411&code=test_code",
            "wp_user_id": 42,
            "wp_site_id": null,
            "home_url": "https://subsidiary.com",
            "site_url": "https://subsidiary.com/wp",
            "wp_install_uuid": "install_1",
            "plugin_instance_uuid": "plugin_1"
        });

        assert!(serde_json::from_value::<PairingHandshakeRequest>(raw).is_err());
    }

    #[test]
    fn invalid_wp_site_id_is_rejected_without_consuming_pairing_code() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_code(&ctx, &db, now);
        let mut bad_request = request(issued.pairing_string.clone());
        bad_request.wp_site_id = "".to_string();

        let err = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: bad_request,
            },
        )
        .unwrap_err();

        assert_eq!(err, SurfacePairingError::BadRequest("wp_site_id_invalid"));
        assert!(list_pairings(&db).unwrap().is_empty());
        complete_test_handshake(&ctx, &db, now + Duration::seconds(1), issued.pairing_string);
    }

    fn validation_input(
        outcome: &PairingHandshakeOutcome,
        now: DateTime<Utc>,
    ) -> SignedSessionValidationInput {
        SignedSessionValidationInput {
            session_id: outcome.session.session_id.clone(),
            surface_client_id: outcome.session.surface_client_id.clone(),
            runtime_anchor_id: "anchor_1".into(),
            site_claims: signed_site_claims(),
            site_nonce: outcome.response.site_nonce.clone(),
            wp_user_id: 42,
            wp_user_hash: wp_user_hash(
                &outcome.session.hmac_master_key,
                &outcome.response.site_binding_digest,
                42,
            ),
            now,
        }
    }

    #[test]
    fn pairing_code_is_single_use_and_bound_to_startup() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_pairing_code(
            &ctx,
            &db,
            PairingCodeIssueInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                now,
            },
        )
        .unwrap();

        let bad = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_2".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request(issued.pairing_string.clone()),
            },
        )
        .unwrap_err();
        assert_eq!(bad, SurfacePairingError::PairingCodeInvalid);

        let ok = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request(issued.pairing_string.clone()),
            },
        )
        .unwrap();
        assert_eq!(ok.response.endpoint_version, "v1");
        assert!(!ok.response.hmac_key.is_empty());
        let returned_hmac_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(ok.response.hmac_key.as_bytes())
            .unwrap();
        assert_eq!(
            returned_hmac_key.as_slice(),
            derive_session_hmac_key(ok.session.hmac_master_key, &ok.session.session_id).as_slice()
        );
        assert_ne!(
            returned_hmac_key.as_slice(),
            ok.session.hmac_master_key.as_slice()
        );

        let replay = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request(issued.pairing_string),
            },
        )
        .unwrap_err();
        assert_eq!(replay, SurfacePairingError::PairingCodeConsumed);
    }

    #[test]
    fn signed_session_validation_rejects_site_switch_and_suspends_pairing() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_pairing_code(
            &ctx,
            &db,
            PairingCodeIssueInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                now,
            },
        )
        .unwrap();
        let outcome = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now,
                request: request(issued.pairing_string),
            },
        )
        .unwrap();

        let err = validate_signed_session(
            &ctx,
            &db,
            SignedSessionValidationInput {
                session_id: outcome.session.session_id.clone(),
                surface_client_id: outcome.session.surface_client_id.clone(),
                runtime_anchor_id: "anchor_1".into(),
                site_claims: SignedSiteClaimsInput {
                    site_url: "https://clone.subsidiary.com/wp".to_string(),
                    ..signed_site_claims()
                },
                site_nonce: outcome.response.site_nonce.clone(),
                wp_user_id: 42,
                wp_user_hash: wp_user_hash(
                    &outcome.session.hmac_master_key,
                    &outcome.response.site_binding_digest,
                    42,
                ),
                now,
            },
        )
        .unwrap_err();
        assert_eq!(err, SurfacePairingError::SiteBindingMismatch);
        let pairings = list_pairings(&db).unwrap();
        assert_eq!(pairings[0].lifecycle_state, "suspended");
    }

    #[test]
    fn failed_pairing_attempts_consume_code_in_runtime_authority() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_code(&ctx, &db, now);

        for _ in 0..2 {
            let result = record_pairing_code_failure(
                &ctx,
                &db,
                PairingCodeFailureInput {
                    endpoint_startup_id: "startup_1".into(),
                    bound_port: 4411,
                    pairing_code: issued.pairing_string.clone(),
                    max_failed_attempts: 3,
                    now,
                },
            )
            .unwrap();
            assert_eq!(result, None);
        }

        let limited = record_pairing_code_failure(
            &ctx,
            &db,
            PairingCodeFailureInput {
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                pairing_code: issued.pairing_string.clone(),
                max_failed_attempts: 3,
                now,
            },
        )
        .unwrap();
        assert_eq!(limited, Some(SurfacePairingError::PairingCodeLimited));

        let replay = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 3,
                now,
                request: request(issued.pairing_string),
            },
        )
        .unwrap_err();
        assert_eq!(replay, SurfacePairingError::PairingCodeConsumed);
    }

    #[test]
    fn failed_pairing_attempts_return_redacted_audit_event() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_code(&ctx, &db, now);
        let outcome = record_pairing_code_failure_with_audit(
            &ctx,
            &db,
            PairingCodeFailureInput {
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                pairing_code: issued.pairing_string.clone(),
                max_failed_attempts: 3,
                now,
            },
        )
        .unwrap();

        assert_eq!(outcome.error, None);
        assert_eq!(outcome.audit.event_kind, "pairing_code_failed");
        assert_eq!(outcome.audit.actor, Actor::System);
        assert_eq!(outcome.audit.detail["decision"], "rejected");
        assert_eq!(outcome.audit.detail["reason"], "handshake_body_invalid");
        let serialized = serde_json::to_string(&outcome.audit.detail).unwrap();
        assert!(!serialized.contains(&issued.pairing_string));
        assert!(outcome.audit.detail.get("pairing_code_hash").is_some());
    }

    #[test]
    fn malformed_pairing_code_failures_still_return_redacted_audit_event() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let outcome = record_pairing_code_failure_with_audit(
            &ctx,
            &db,
            PairingCodeFailureInput {
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                pairing_code: "not a token with spaces".into(),
                max_failed_attempts: 3,
                now: Utc::now(),
            },
        )
        .unwrap();

        assert_eq!(outcome.error, Some(SurfacePairingError::PairingCodeInvalid));
        assert_eq!(outcome.audit.event_kind, "pairing_code_failed");
        assert_eq!(outcome.audit.detail["reason"], "pairing_code_invalid");
        let serialized = serde_json::to_string(&outcome.audit.detail).unwrap();
        assert!(!serialized.contains("not a token with spaces"));
        assert!(outcome.audit.detail.get("pairing_code_hash").is_some());
    }

    #[test]
    fn expired_pairing_code_and_session_expiry_are_rejected() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let expired = issue_code(&ctx, &db, now);

        let expired_code = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now: now + Duration::seconds(PAIRING_CODE_TTL_SECONDS + 1),
                request: request(expired.pairing_string.clone()),
            },
        )
        .unwrap_err();
        assert_eq!(expired_code, SurfacePairingError::PairingCodeExpired);

        let expired_replay = complete_handshake(
            &ctx,
            &db,
            PairingHandshakeInput {
                runtime_anchor_id: "anchor_1".into(),
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                endpoint_version: "v1",
                max_failed_attempts: 5,
                now: now + Duration::seconds(PAIRING_CODE_TTL_SECONDS + 2),
                request: request(expired.pairing_string),
            },
        )
        .unwrap_err();
        assert_eq!(expired_replay, SurfacePairingError::PairingCodeConsumed);

        let expired_failure = issue_code(&ctx, &db, now);
        let failure_result = record_pairing_code_failure(
            &ctx,
            &db,
            PairingCodeFailureInput {
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                pairing_code: expired_failure.pairing_string.clone(),
                max_failed_attempts: 5,
                now: now + Duration::seconds(PAIRING_CODE_TTL_SECONDS + 1),
            },
        )
        .unwrap();
        assert_eq!(
            failure_result,
            Some(SurfacePairingError::PairingCodeExpired)
        );
        let failure_replay = record_pairing_code_failure(
            &ctx,
            &db,
            PairingCodeFailureInput {
                endpoint_startup_id: "startup_1".into(),
                bound_port: 4411,
                pairing_code: expired_failure.pairing_string,
                max_failed_attempts: 5,
                now: now + Duration::seconds(PAIRING_CODE_TTL_SECONDS + 2),
            },
        )
        .unwrap();
        assert_eq!(
            failure_replay,
            Some(SurfacePairingError::PairingCodeConsumed)
        );

        let outcome =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);
        let expired_session_at = now + Duration::seconds(SESSION_INACTIVE_TTL_SECONDS + 1);
        assert_eq!(
            validate_signed_session(&ctx, &db, validation_input(&outcome, expired_session_at))
                .unwrap_err(),
            SurfacePairingError::SessionExpired
        );
    }

    #[test]
    fn reinstall_and_wp_user_changes_reject_existing_session() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let issued = issue_code(&ctx, &db, now);
        let outcome = complete_test_handshake(&ctx, &db, now, issued.pairing_string);

        let mut reinstall = validation_input(&outcome, now);
        reinstall.runtime_anchor_id = "anchor_2".into();
        assert_eq!(
            validate_signed_session(&ctx, &db, reinstall).unwrap_err(),
            SurfacePairingError::UnknownRuntimeAnchor
        );

        let mut changed_user = validation_input(&outcome, now);
        changed_user.wp_user_id = 7;
        changed_user.wp_user_hash = wp_user_hash(
            &outcome.session.hmac_master_key,
            &outcome.response.site_binding_digest,
            7,
        );
        assert_eq!(
            validate_signed_session(&ctx, &db, changed_user).unwrap_err(),
            SurfacePairingError::WpUserMismatch
        );
        assert_eq!(list_pairings(&db).unwrap()[0].lifecycle_state, "suspended");
    }

    #[test]
    fn repairing_same_site_revokes_old_epoch_and_blocks_restore() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let first =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);
        let second = complete_test_handshake(
            &ctx,
            &db,
            now + Duration::seconds(1),
            issue_code(&ctx, &db, now + Duration::seconds(1)).pairing_string,
        );
        assert_eq!(
            second.response.pairing_epoch,
            first.response.pairing_epoch + 1
        );
        assert_eq!(
            second.revoked_surface_client_ids,
            vec![first.session.surface_client_id.clone()]
        );
        let revocation_audit = second
            .revocation_audit
            .as_ref()
            .expect("re-pair emits revocation audit for the old pairing");
        let first_wp_user_hash = wp_user_hash(
            &first.session.hmac_master_key,
            &first.response.site_binding_digest,
            42,
        );
        assert_eq!(revocation_audit.event_kind, "pairing_revoked");
        assert_eq!(revocation_audit.wp_user_id, Some(42));
        assert_eq!(
            revocation_audit.wp_user_hash.as_deref(),
            Some(first_wp_user_hash.as_str())
        );
        assert_eq!(
            revocation_audit.detail["surface_client_id"].as_str(),
            Some(first.session.surface_client_id.as_str())
        );
        assert_eq!(
            revocation_audit.detail["reason"].as_str(),
            Some("replaced_by_repairing")
        );
        assert!(revocation_audit.detail.get("wp_site_id_hash").is_some());

        assert_eq!(
            validate_signed_session(&ctx, &db, validation_input(&first, now)).unwrap_err(),
            SurfacePairingError::RestoredStalePairing
        );

        db.conn_ref()
            .execute(
                "UPDATE surface_client_pairings
                 SET lifecycle_state = 'active', revoked_at = NULL, revoked_reason = NULL
                 WHERE surface_client_id = ?1",
                params![first.session.surface_client_id],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "UPDATE surface_client_sessions
                 SET revoked_at = NULL, revoked_reason = NULL
                 WHERE session_id = ?1",
                params![first.session.session_id],
            )
            .unwrap();

        assert_eq!(
            validate_signed_session(&ctx, &db, validation_input(&first, now)).unwrap_err(),
            SurfacePairingError::RestoredStalePairing
        );
    }

    #[test]
    fn revoked_same_epoch_tombstone_blocks_stale_restore() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let outcome =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);

        let event = revoke_pairing(
            &ctx,
            &db,
            RevokePairingInput {
                surface_client_id: outcome.session.surface_client_id.clone(),
                reason: "user_revoked".to_string(),
                now,
            },
        )
        .unwrap();
        assert_eq!(event.event_kind, "pairing_revoked");
        assert_eq!(
            event.detail["surface_client_id"].as_str(),
            Some(outcome.session.surface_client_id.as_str())
        );
        assert_eq!(event.detail["reason"].as_str(), Some("user_revoked"));
        assert!(event.detail.get("wp_site_id_hash").is_some());
        assert!(event.detail.get("wp_user_hash").is_some());

        db.conn_ref()
            .execute(
                "UPDATE surface_client_pairings
                 SET lifecycle_state = 'active', revoked_at = NULL, revoked_reason = NULL
                 WHERE surface_client_id = ?1",
                params![outcome.session.surface_client_id],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "UPDATE surface_client_sessions
                 SET revoked_at = NULL, revoked_reason = NULL
                 WHERE session_id = ?1",
                params![outcome.session.session_id],
            )
            .unwrap();

        assert_eq!(
            validate_signed_session(&ctx, &db, validation_input(&outcome, now)).unwrap_err(),
            SurfacePairingError::PairingRevoked
        );
    }

    #[test]
    fn site_nonce_mismatch_suspends_before_ability_use() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let outcome =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);
        let mut input = validation_input(&outcome, now);
        input.site_nonce = "siteNonceForClonedSite".to_string();

        assert_eq!(
            validate_signed_session(&ctx, &db, input).unwrap_err(),
            SurfacePairingError::SiteBindingMismatch
        );
        assert_eq!(list_pairings(&db).unwrap()[0].lifecycle_state, "suspended");
    }

    #[test]
    fn suspicious_replay_throttles_then_revokes_session() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let outcome =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);

        let mut throttle_events = Vec::new();
        for offset in 0..3 {
            throttle_events = record_signed_transport_failure(
                &ctx,
                &db,
                SignedTransportFailureInput {
                    session_id: outcome.session.session_id.clone(),
                    surface_client_id: Some(outcome.session.surface_client_id.clone()),
                    failure_code: "timestamp_future".to_string(),
                    now: now + Duration::seconds(offset),
                },
            )
            .unwrap();
        }
        assert!(throttle_events
            .iter()
            .any(|event| event.event_kind == "pairing.exfiltration.suspected_replay"));
        assert_eq!(
            validate_signed_session(&ctx, &db, validation_input(&outcome, now)).unwrap_err(),
            SurfacePairingError::SessionThrottled
        );

        assert!(validate_signed_session(
            &ctx,
            &db,
            validation_input(&outcome, now + Duration::seconds(90))
        )
        .is_ok());

        let mut revoke_events = Vec::new();
        for offset in 0..5 {
            revoke_events = record_signed_transport_failure(
                &ctx,
                &db,
                SignedTransportFailureInput {
                    session_id: outcome.session.session_id.clone(),
                    surface_client_id: Some(outcome.session.surface_client_id.clone()),
                    failure_code: "nonce_replay".to_string(),
                    now: now + Duration::seconds(120 + offset),
                },
            )
            .unwrap();
        }
        assert!(revoke_events
            .iter()
            .any(|event| event.event_kind == "pairing_revoked"));
        assert_eq!(
            validate_signed_session(
                &ctx,
                &db,
                validation_input(&outcome, now + Duration::seconds(130))
            )
            .unwrap_err(),
            SurfacePairingError::PairingRevoked
        );
    }

    #[test]
    fn revocation_rolls_back_if_tombstone_write_fails() {
        allow_surface_scopes();
        let db = db();
        let (clock, rng, external) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &external);
        let now = Utc::now();
        let outcome =
            complete_test_handshake(&ctx, &db, now, issue_code(&ctx, &db, now).pairing_string);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_surface_revocation_insert
                 BEFORE INSERT ON surface_client_revocations
                 BEGIN
                    SELECT RAISE(ABORT, 'revocation tombstone failed');
                 END;",
            )
            .unwrap();

        let error = revoke_pairing(
            &ctx,
            &db,
            RevokePairingInput {
                surface_client_id: outcome.session.surface_client_id,
                reason: "user_revoked".to_string(),
                now,
            },
        )
        .unwrap_err();
        assert!(matches!(error, SurfacePairingError::Write(_)));
        assert_eq!(list_pairings(&db).unwrap()[0].lifecycle_state, "active");
    }

    #[test]
    fn audit_serialization_never_contains_raw_wp_values() {
        allow_surface_scopes();
        let _dir = tempfile::tempdir().expect("tempdir");
        let path = _dir.path().join("audit.log");
        let mut logger = AuditLogger::new(path);
        let secret = [7_u8; 32];
        let wp_user_hash = wp_user_hash(&secret, "site_digest", 42);
        let event = SurfacePairingAuditEvent {
            event_kind: "pairing_created",
            category: "security",
            actor: Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface_test"),
                scopes: scope_set_from_strings(&default_granted_scopes()).unwrap(),
            },
            wp_user_id: Some(42),
            wp_user_hash: Some(wp_user_hash),
            detail: json!({
                "surface_client_id": "surface_test",
                "site_binding_digest": "site_digest"
            }),
        };
        emit_pairing_audit(&mut logger, &event).unwrap();
        let raw = std::fs::read_to_string(logger.path()).unwrap();
        assert!(!raw.contains("\"wp_user_id\""));
        assert!(!raw.contains("read.account_overview"));
        assert!(!raw.contains("submit.feedback"));
        assert!(!raw.contains("subsidiary.com"));
        assert!(raw.contains("wp_user_hash"));
        let record: crate::audit_log::AuditRecord =
            serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        assert!(record.detail.get("wp_user_hash").is_none());
    }

    #[test]
    fn wp_user_hash_is_keyed_by_session_secret() {
        let first_secret = [1_u8; 32];
        let second_secret = [2_u8; 32];
        let first = wp_user_hash(&first_secret, "site_digest", 42);
        assert_eq!(first, wp_user_hash(&first_secret, "site_digest", 42));
        assert_ne!(first, wp_user_hash(&second_secret, "site_digest", 42));
    }
}
