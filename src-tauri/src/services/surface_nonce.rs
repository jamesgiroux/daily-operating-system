use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use http::StatusCode;
use parking_lot::Mutex as ParkingMutex;
use rand::RngExt;
use ring::{hkdf, hmac};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::Mutex;

use abilities_runtime::abilities::registry::Actor;

use crate::audit_log::{emit_surface_audit, AuditError, AuditFields, AuditLogger};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::services::surface_pairing::ValidatedSurfaceSession;
use crate::services::{claims, compositions};

const NONCE_BYTES: usize = 32;
const DIGEST_BYTES: usize = 32;
const DEFAULT_TTL_SECONDS: i64 = 60;
const DEFAULT_MAX_LIVE_BINDINGS: usize = 4096;
const DEFAULT_MAX_OUTSTANDING_PER_SESSION: usize = 64;
const DEFAULT_BUDGET_PER_MINUTE: u32 = 240;
const PRESENCE_NONCE_KEY_SALT: &[u8] = b"DAILYOS-SURFACE-PRESENCE-NONCE-SALT-V1";
const PRESENCE_NONCE_KEY_INFO: &[u8] = b"dailyos.surface.presence_nonce.digest.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceNonceConfig {
    pub ttl: Duration,
    pub max_live_bindings: usize,
    pub max_outstanding_per_session: usize,
    pub issue_budget_per_minute: u32,
    pub verify_budget_per_minute: u32,
    pub failure_budget_per_minute: u32,
}

impl Default for SurfaceNonceConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::seconds(DEFAULT_TTL_SECONDS),
            max_live_bindings: DEFAULT_MAX_LIVE_BINDINGS,
            max_outstanding_per_session: DEFAULT_MAX_OUTSTANDING_PER_SESSION,
            issue_budget_per_minute: DEFAULT_BUDGET_PER_MINUTE,
            verify_budget_per_minute: DEFAULT_BUDGET_PER_MINUTE,
            failure_budget_per_minute: DEFAULT_BUDGET_PER_MINUTE,
        }
    }
}

#[derive(Clone)]
pub struct SurfaceNonceService {
    inner: Arc<SurfaceNonceServiceInner>,
}

struct SurfaceNonceServiceInner {
    config: SurfaceNonceConfig,
    digest_key: PresenceNonceDigestKey,
    store: SurfaceNonceStore,
    budgets: ParkingMutex<NonceBudgetState>,
    rng: Arc<dyn PresenceNonceRandom>,
}

impl SurfaceNonceService {
    pub fn new_from_w2b_secret(secret_material: [u8; 32]) -> Result<Self, SurfaceNonceKeyError> {
        Self::with_config_and_rng(
            SurfaceNonceConfig::default(),
            secret_material,
            Arc::new(SystemPresenceNonceRandom),
        )
    }

    fn with_config_and_rng(
        mut config: SurfaceNonceConfig,
        mut secret_material: [u8; 32],
        rng: Arc<dyn PresenceNonceRandom>,
    ) -> Result<Self, SurfaceNonceKeyError> {
        if config.ttl > Duration::seconds(DEFAULT_TTL_SECONDS) {
            config.ttl = Duration::seconds(DEFAULT_TTL_SECONDS);
        }
        config.max_live_bindings = config.max_live_bindings.max(1);
        config.max_outstanding_per_session = config.max_outstanding_per_session.max(1);
        config.issue_budget_per_minute = config.issue_budget_per_minute.max(1);
        config.verify_budget_per_minute = config.verify_budget_per_minute.max(1);
        config.failure_budget_per_minute = config.failure_budget_per_minute.max(1);

        let digest_key = PresenceNonceDigestKey::derive_from_w2b_secret(&secret_material)?;
        secret_material.fill(0);
        Ok(Self {
            inner: Arc::new(SurfaceNonceServiceInner {
                config,
                digest_key,
                store: SurfaceNonceStore::default(),
                budgets: ParkingMutex::new(NonceBudgetState::default()),
                rng,
            }),
        })
    }

    pub fn issue_nonce(
        &self,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        session: &ValidatedSurfaceSession,
        payload: Value,
        fallback_request_id: &str,
    ) -> Result<SurfaceNonceIssue, SurfaceNonceError> {
        ensure_surface_client(session, fallback_request_id)?;
        let request = IssueNonceRequest::parse(payload, fallback_request_id)
            .map_err(|error| self.shape_error(session, error))?;
        let audit = NonceAuditContext::from_issue(session, &request);
        ensure_session_tuple(session, &request.session_id, request.wp_user_id, &audit)?;

        let issue_budget_key =
            request.budget_key(&session.surface_client_id, NonceBudgetClass::Issue);
        self.charge_budget(
            NonceBudgetClass::Issue,
            Some(&issue_budget_key),
            session,
            &audit,
        )?;

        let claim = claims::load_claim_by_id(db.conn_ref(), &request.claim_id)
            .map_err(|_| {
                SurfaceNonceError::rejected(PresenceNonceRejectReason::WrongClaim, audit.clone())
            })?
            .ok_or_else(|| {
                SurfaceNonceError::rejected(PresenceNonceRejectReason::WrongClaim, audit.clone())
            })?;
        if claim.field_path.as_deref().unwrap_or_default() != request.field_path {
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::WrongField,
                audit,
            ));
        }
        let current_claim_version = claims::current_claim_version_for_claim_id(
            ctx,
            db,
            &request.claim_id,
        )
        .map_err(|_| {
            SurfaceNonceError::rejected(PresenceNonceRejectReason::WrongClaim, audit.clone())
        })?;
        if current_claim_version != request.claim_version {
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::ClaimVersionStale,
                audit.with_current_versions(Some(current_claim_version), None),
            ));
        }

        let current_composition_version =
            compositions::current_composition_version_for_composition_id(
                ctx,
                db,
                &request.composition_id,
            )
            .map_err(|_| {
                SurfaceNonceError::rejected(
                    PresenceNonceRejectReason::CompositionVersionStale,
                    audit.clone(),
                )
            })?;
        if current_composition_version != request.composition_version {
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::CompositionVersionStale,
                audit.with_current_versions(None, Some(current_composition_version)),
            ));
        }

        let now = ctx.clock.now();
        let expires_at = now + self.inner.config.ttl;
        let fields = PresenceNonceBindingFields {
            surface_client_id: session.surface_client_id.clone(),
            session_id: request.session_id.clone(),
            wp_user_id: request.wp_user_id,
            claim_id: request.claim_id.clone(),
            field_path: request.field_path.clone(),
            action: request.action,
            claim_version: request.claim_version,
            composition_id: request.composition_id.clone(),
            composition_version: request.composition_version,
            generated_at: now,
            expires_at,
        };

        let issued = self.inner.store.issue(
            &self.inner.digest_key,
            self.inner.rng.as_ref(),
            &self.inner.config,
            fields,
            now,
            audit.clone(),
        )?;
        let mut audit_events = issued.audit_events;
        audit_events.push(audit_event(
            "presence_nonce_issued",
            session,
            audit.with_nonce_digest(issued.nonce_digest.clone()),
            "issued",
            None,
        ));

        Ok(SurfaceNonceIssue {
            presence_nonce: issued.presence_nonce,
            expires_at,
            ttl_seconds: self.inner.config.ttl.num_seconds().try_into().unwrap_or(60),
            request_id: request.request_id,
            audit_events,
        })
    }

    pub fn verify_nonce(
        &self,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        session: &ValidatedSurfaceSession,
        payload: Value,
        fallback_request_id: &str,
    ) -> Result<SurfaceNonceVerify, SurfaceNonceError> {
        ensure_surface_client(session, fallback_request_id)?;
        let request = VerifyNonceRequest::parse(payload, fallback_request_id)
            .map_err(|error| self.shape_error(session, error))?;
        let audit = NonceAuditContext::from_verify(session, &request);
        ensure_session_tuple(session, &request.session_id, request.wp_user_id, &audit)?;
        let verify_budget_key =
            request.budget_key(&session.surface_client_id, NonceBudgetClass::Verify);
        let failure_budget_key =
            request.budget_key(&session.surface_client_id, NonceBudgetClass::Failure);
        self.charge_budget(
            NonceBudgetClass::Verify,
            Some(&verify_budget_key),
            session,
            &audit,
        )?;

        let nonce_bytes = decode_presence_nonce(&request.presence_nonce).map_err(|reason| {
            self.charge_failure_best_effort(session, Some(&failure_budget_key), &audit);
            SurfaceNonceError::rejected(reason, audit.clone())
        })?;
        let digest = self.inner.digest_key.digest(&nonce_bytes);

        match self.inner.store.verify_and_consume(
            ctx,
            db,
            digest,
            &request,
            ctx.clock.now(),
            audit.clone(),
        ) {
            Ok(verified) => {
                let audit_events = vec![audit_event(
                    "presence_nonce_verified",
                    session,
                    audit.with_nonce_digest(verified.nonce_digest.clone()),
                    "verified",
                    None,
                )];
                Ok(SurfaceNonceVerify {
                    consumed_at: verified.consumed_at,
                    request_id: request.request_id,
                    expected_claim_version: verified.expected_claim_version,
                    expected_composition_version: verified.expected_composition_version,
                    audit_events,
                })
            }
            Err(mut error) => {
                self.charge_failure_best_effort(session, Some(&failure_budget_key), &audit);
                if error.audit_events.is_empty() {
                    error.audit_events.push(audit_event(
                        "presence_nonce_rejected",
                        session,
                        (*error.audit).clone(),
                        "rejected",
                        Some(error.reason),
                    ));
                }
                Err(error)
            }
        }
    }

    pub fn invalidate_composition(
        &self,
        session: &ValidatedSurfaceSession,
        composition_id: &str,
        current_version: u64,
        now: DateTime<Utc>,
        request_id: &str,
    ) -> Vec<SurfaceNonceAuditEvent> {
        self.inner.store.invalidate_composition(
            composition_id,
            current_version,
            now,
            NonceAuditContext::from_session(session, request_id),
        )
    }

    pub fn emit_audit_events(
        logger: &mut AuditLogger,
        events: &[SurfaceNonceAuditEvent],
    ) -> Result<(), AuditError> {
        for event in events {
            let mut fields = AuditFields::new("security", event.detail.clone());
            if let Some(wp_user_id) = event.wp_user_id {
                fields = fields.with_wp_user_id(wp_user_id);
            }
            if let Some(wp_user_hash) = event.wp_user_hash.as_ref() {
                fields = fields.with_wp_user_hash(wp_user_hash.clone());
            }
            emit_surface_audit(logger, event.event_kind, &event.actor, fields)?;
        }
        Ok(())
    }

    fn shape_error(
        &self,
        session: &ValidatedSurfaceSession,
        error: RequestShapeError,
    ) -> SurfaceNonceError {
        let audit = NonceAuditContext::from_session(session, &error.request_id);
        self.charge_failure_best_effort(session, None, &audit);
        SurfaceNonceError::rejected(error.reason, audit)
    }

    fn charge_budget(
        &self,
        class: NonceBudgetClass,
        key: Option<&NonceBudgetKey>,
        session: &ValidatedSurfaceSession,
        audit: &NonceAuditContext,
    ) -> Result<(), SurfaceNonceError> {
        let mut budgets = self.inner.budgets.lock();
        let budget_key = key
            .cloned()
            .unwrap_or_else(|| NonceBudgetKey::narrow(session, class));
        let limit = match class {
            NonceBudgetClass::Issue => self.inner.config.issue_budget_per_minute,
            NonceBudgetClass::Verify => self.inner.config.verify_budget_per_minute,
            NonceBudgetClass::Failure => self.inner.config.failure_budget_per_minute,
        };
        budgets
            .check_and_consume(budget_key, limit, audit.now)
            .map_err(|retry_after| SurfaceNonceError::rate_limited(audit.clone(), retry_after))
    }

    fn charge_failure_best_effort(
        &self,
        session: &ValidatedSurfaceSession,
        key: Option<&NonceBudgetKey>,
        audit: &NonceAuditContext,
    ) {
        drop(self.charge_budget(NonceBudgetClass::Failure, key, session, audit));
    }
}

#[derive(Debug, Error)]
pub enum SurfaceNonceKeyError {
    #[error("presence nonce key derivation failed")]
    Derive,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NonceDigest([u8; DIGEST_BYTES]);

impl NonceDigest {
    fn prefix(&self) -> String {
        hex::encode(&self.0[..8])
    }
}

#[derive(Clone)]
struct PresenceNonceDigestKey([u8; DIGEST_BYTES]);

impl PresenceNonceDigestKey {
    fn derive_from_w2b_secret(secret_material: &[u8; 32]) -> Result<Self, SurfaceNonceKeyError> {
        struct DigestKeyLen;
        impl hkdf::KeyType for DigestKeyLen {
            fn len(&self) -> usize {
                DIGEST_BYTES
            }
        }

        let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, PRESENCE_NONCE_KEY_SALT);
        let prk = salt.extract(secret_material);
        let okm = prk
            .expand(&[PRESENCE_NONCE_KEY_INFO], DigestKeyLen)
            .map_err(|_| SurfaceNonceKeyError::Derive)?;
        let mut key = [0_u8; DIGEST_BYTES];
        okm.fill(&mut key)
            .map_err(|_| SurfaceNonceKeyError::Derive)?;
        Ok(Self(key))
    }

    fn digest(&self, nonce_bytes: &[u8]) -> NonceDigest {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0);
        let digest = hmac::sign(&key, nonce_bytes);
        let mut out = [0_u8; DIGEST_BYTES];
        out.copy_from_slice(digest.as_ref());
        NonceDigest(out)
    }
}

trait PresenceNonceRandom: Send + Sync {
    fn fill_nonce(&self, bytes: &mut [u8; NONCE_BYTES]) -> Result<(), SurfaceNonceError>;
}

struct SystemPresenceNonceRandom;

impl PresenceNonceRandom for SystemPresenceNonceRandom {
    fn fill_nonce(&self, bytes: &mut [u8; NONCE_BYTES]) -> Result<(), SurfaceNonceError> {
        rand::rng().fill(bytes);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceNonceAction {
    Correct,
    Dismiss,
    Corroborate,
    Contradict,
}

impl PresenceNonceAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Correct => "correct",
            Self::Dismiss => "dismiss",
            Self::Corroborate => "corroborate",
            Self::Contradict => "contradict",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "correct" => Some(Self::Correct),
            "dismiss" => Some(Self::Dismiss),
            "corroborate" => Some(Self::Corroborate),
            "contradict" => Some(Self::Contradict),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PresenceNonceBinding {
    nonce_digest: NonceDigest,
    fields: PresenceNonceBindingFields,
    lifecycle: PresenceNonceLifecycle,
    _sealed: private::Sealed,
}

#[derive(Clone, Debug)]
pub struct PresenceNonceBindingFields {
    surface_client_id: String,
    session_id: String,
    wp_user_id: u64,
    claim_id: String,
    field_path: String,
    action: PresenceNonceAction,
    claim_version: u64,
    composition_id: String,
    composition_version: u64,
    generated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct PresenceNonceLifecycle {
    consumed_at: Option<DateTime<Utc>>,
    invalidated_at: Option<DateTime<Utc>>,
    invalidation_reason: Option<InvalidationReason>,
}

mod private {
    #[derive(Clone, Debug)]
    pub struct Sealed;
}

impl PresenceNonceBinding {
    pub fn new(fields: PresenceNonceBindingFields, nonce_digest: NonceDigest) -> Self {
        Self {
            nonce_digest,
            fields,
            lifecycle: PresenceNonceLifecycle::default(),
            _sealed: private::Sealed,
        }
    }

    pub fn try_mark_consumed(&mut self, now: DateTime<Utc>) -> Result<(), LifecycleRace> {
        if self.lifecycle.consumed_at.is_some() {
            return Err(LifecycleRace::AlreadyConsumed);
        }
        if self.lifecycle.invalidated_at.is_some() {
            return Err(LifecycleRace::AlreadyInvalidated);
        }
        self.lifecycle.consumed_at = Some(now);
        Ok(())
    }

    pub fn try_mark_invalidated(
        &mut self,
        now: DateTime<Utc>,
        reason: InvalidationReason,
    ) -> Result<(), LifecycleRace> {
        if self.lifecycle.consumed_at.is_some() {
            return Err(LifecycleRace::AlreadyConsumed);
        }
        if self.lifecycle.invalidated_at.is_some() {
            return Err(LifecycleRace::AlreadyInvalidated);
        }
        self.lifecycle.invalidated_at = Some(now);
        self.lifecycle.invalidation_reason = Some(reason);
        Ok(())
    }

    fn is_live_at(&self, now: DateTime<Utc>) -> bool {
        self.lifecycle.consumed_at.is_none()
            && self.lifecycle.invalidated_at.is_none()
            && self.fields.expires_at >= now
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleRace {
    AlreadyConsumed,
    AlreadyInvalidated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidationReason {
    CompositionRefresh,
    StorePressure,
    Expired,
}

#[derive(Default)]
pub struct SurfaceNonceStore {
    inner: Mutex<SurfaceNonceStoreInner>,
}

#[derive(Default)]
struct SurfaceNonceStoreInner {
    by_digest: HashMap<NonceDigest, PresenceNonceBinding>,
    by_composition: HashMap<String, HashSet<NonceDigest>>,
    lru: VecDeque<NonceDigest>,
}

struct IssuedNonce {
    presence_nonce: String,
    nonce_digest: NonceDigest,
    audit_events: Vec<SurfaceNonceAuditEvent>,
}

struct VerifiedNonce {
    consumed_at: DateTime<Utc>,
    nonce_digest: NonceDigest,
    expected_claim_version: u64,
    expected_composition_version: u64,
}

impl SurfaceNonceStore {
    fn issue(
        &self,
        digest_key: &PresenceNonceDigestKey,
        rng: &dyn PresenceNonceRandom,
        config: &SurfaceNonceConfig,
        fields: PresenceNonceBindingFields,
        now: DateTime<Utc>,
        audit: NonceAuditContext,
    ) -> Result<IssuedNonce, SurfaceNonceError> {
        let mut inner = self.inner.blocking_lock();
        let outstanding = inner
            .by_digest
            .values()
            .filter(|binding| {
                binding.fields.session_id == fields.session_id && binding.is_live_at(now)
            })
            .count();
        if outstanding >= config.max_outstanding_per_session {
            return Err(SurfaceNonceError::rate_limited(
                audit,
                StdDuration::from_secs(60),
            ));
        }

        let mut nonce_bytes = [0_u8; NONCE_BYTES];
        rng.fill_nonce(&mut nonce_bytes)?;
        let nonce_digest = digest_key.digest(&nonce_bytes);
        let presence_nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce_bytes);
        nonce_bytes.fill(0);

        inner
            .by_composition
            .entry(fields.composition_id.clone())
            .or_default()
            .insert(nonce_digest.clone());
        inner.lru.push_back(nonce_digest.clone());
        inner.by_digest.insert(
            nonce_digest.clone(),
            PresenceNonceBinding::new(fields, nonce_digest.clone()),
        );
        let audit_events = inner.evict_under_pressure(config.max_live_bindings, now, audit);

        Ok(IssuedNonce {
            presence_nonce,
            nonce_digest,
            audit_events,
        })
    }

    fn verify_and_consume(
        &self,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        digest: NonceDigest,
        request: &VerifyNonceRequest,
        now: DateTime<Utc>,
        audit: NonceAuditContext,
    ) -> Result<VerifiedNonce, SurfaceNonceError> {
        let mut inner = self.inner.blocking_lock();
        let binding = inner.by_digest.get_mut(&digest).ok_or_else(|| {
            SurfaceNonceError::rejected(PresenceNonceRejectReason::Invalidated, audit.clone())
        })?;
        compare_binding_tuple(binding, request, &audit)?;

        if let Some(reason) = binding.lifecycle.invalidation_reason {
            let reject = match reason {
                InvalidationReason::CompositionRefresh => {
                    PresenceNonceRejectReason::CompositionVersionStale
                }
                InvalidationReason::StorePressure => PresenceNonceRejectReason::Invalidated,
                InvalidationReason::Expired => PresenceNonceRejectReason::Expired,
            };
            return Err(SurfaceNonceError::rejected(
                reject,
                audit.with_nonce_digest(digest),
            ));
        }
        if binding.lifecycle.consumed_at.is_some() {
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::Replayed,
                audit.with_nonce_digest(digest),
            ));
        }
        if binding.fields.expires_at < now {
            match binding.try_mark_invalidated(now, InvalidationReason::Expired) {
                Ok(()) | Err(_) => {}
            }
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::Expired,
                audit.with_nonce_digest(digest),
            ));
        }

        let current_claim_version = claims::current_claim_version_for_claim_id(
            ctx,
            db,
            &binding.fields.claim_id,
        )
        .map_err(|_| {
            SurfaceNonceError::rejected(PresenceNonceRejectReason::WrongClaim, audit.clone())
        })?;
        if current_claim_version != binding.fields.claim_version {
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::ClaimVersionStale,
                audit
                    .with_nonce_digest(digest)
                    .with_current_versions(Some(current_claim_version), None),
            ));
        }

        let current_composition_version =
            compositions::current_composition_version_for_composition_id(
                ctx,
                db,
                &binding.fields.composition_id,
            )
            .map_err(|_| {
                SurfaceNonceError::rejected(
                    PresenceNonceRejectReason::CompositionVersionStale,
                    audit.clone(),
                )
            })?;
        if current_composition_version != binding.fields.composition_version {
            match binding.try_mark_invalidated(now, InvalidationReason::CompositionRefresh) {
                Ok(()) | Err(_) => {}
            }
            return Err(SurfaceNonceError::rejected(
                PresenceNonceRejectReason::CompositionVersionStale,
                audit
                    .with_nonce_digest(digest)
                    .with_current_versions(None, Some(current_composition_version)),
            ));
        }

        binding.try_mark_consumed(now).map_err(|race| match race {
            LifecycleRace::AlreadyConsumed => SurfaceNonceError::rejected(
                PresenceNonceRejectReason::Replayed,
                audit.clone().with_nonce_digest(digest.clone()),
            ),
            LifecycleRace::AlreadyInvalidated => SurfaceNonceError::rejected(
                PresenceNonceRejectReason::Invalidated,
                audit.clone().with_nonce_digest(digest.clone()),
            ),
        })?;

        Ok(VerifiedNonce {
            consumed_at: now,
            nonce_digest: digest,
            expected_claim_version: binding.fields.claim_version,
            expected_composition_version: binding.fields.composition_version,
        })
    }

    fn invalidate_composition(
        &self,
        composition_id: &str,
        current_version: u64,
        now: DateTime<Utc>,
        audit: NonceAuditContext,
    ) -> Vec<SurfaceNonceAuditEvent> {
        let mut inner = self.inner.blocking_lock();
        let digests = inner
            .by_composition
            .get(composition_id)
            .cloned()
            .unwrap_or_default();
        let mut count = 0_usize;
        for digest in digests {
            if let Some(binding) = inner.by_digest.get_mut(&digest) {
                if binding.fields.composition_version < current_version
                    && binding
                        .try_mark_invalidated(now, InvalidationReason::CompositionRefresh)
                        .is_ok()
                {
                    count += 1;
                }
            }
        }
        if count == 0 {
            return Vec::new();
        }
        let session = audit.session.clone();
        vec![audit_event(
            "presence_nonce_invalidated",
            &session,
            audit.with_composition(composition_id.to_string(), current_version),
            "invalidated",
            Some(PresenceNonceRejectReason::CompositionVersionStale),
        )
        .with_count(count)]
    }
}

impl SurfaceNonceStoreInner {
    fn live_count(&self, now: DateTime<Utc>) -> usize {
        self.by_digest
            .values()
            .filter(|binding| binding.is_live_at(now))
            .count()
    }

    fn evict_under_pressure(
        &mut self,
        max_live_bindings: usize,
        now: DateTime<Utc>,
        audit: NonceAuditContext,
    ) -> Vec<SurfaceNonceAuditEvent> {
        let mut evicted = 0_usize;
        while self.live_count(now) > max_live_bindings {
            let Some(digest) = self.lru.pop_front() else {
                break;
            };
            let Some(binding) = self.by_digest.get_mut(&digest) else {
                continue;
            };
            if binding
                .try_mark_invalidated(now, InvalidationReason::StorePressure)
                .is_ok()
            {
                evicted += 1;
            }
        }
        if evicted == 0 {
            return Vec::new();
        }
        let session = audit.session.clone();
        vec![audit_event(
            "presence_nonce_invalidated",
            &session,
            audit,
            "invalidated",
            Some(PresenceNonceRejectReason::Invalidated),
        )
        .with_reason("store_pressure")
        .with_count(evicted)]
    }
}

#[derive(Clone, Debug)]
struct IssueNonceRequest {
    session_id: String,
    wp_user_id: u64,
    claim_id: String,
    field_path: String,
    action: PresenceNonceAction,
    claim_version: u64,
    composition_id: String,
    composition_version: u64,
    request_id: String,
}

impl IssueNonceRequest {
    fn parse(payload: Value, fallback_request_id: &str) -> Result<Self, RequestShapeError> {
        let object = payload.as_object().ok_or_else(|| {
            RequestShapeError::new(
                PresenceNonceRejectReason::MalformedRequest,
                fallback_request_id,
            )
        })?;
        let request_id = optional_string(object.get("request_id"))
            .unwrap_or_else(|| fallback_request_id.to_string());
        let claim_version = required_claim_version(object.get("claim_version"), &request_id)?;
        let action =
            PresenceNonceAction::parse(&required_string(object.get("action"), &request_id)?)
                .ok_or_else(|| {
                    RequestShapeError::new(PresenceNonceRejectReason::MalformedRequest, &request_id)
                })?;
        Ok(Self {
            session_id: required_string(object.get("session_id"), &request_id)?,
            wp_user_id: required_u64(object.get("wp_user_id"), &request_id)?,
            claim_id: required_string(object.get("claim_id"), &request_id)?,
            field_path: required_string(object.get("field_path"), &request_id)?,
            action,
            claim_version,
            composition_id: required_string(object.get("composition_id"), &request_id)?,
            composition_version: required_u64(object.get("composition_version"), &request_id)?,
            request_id,
        })
    }

    fn budget_key(&self, surface_client_id: &str, class: NonceBudgetClass) -> NonceBudgetKey {
        NonceBudgetKey::full(
            class,
            surface_client_id,
            self.wp_user_id,
            &self.claim_id,
            &self.field_path,
            self.action,
        )
    }
}

#[derive(Clone, Debug)]
struct VerifyNonceRequest {
    presence_nonce: String,
    session_id: String,
    wp_user_id: u64,
    claim_id: String,
    field_path: String,
    action: PresenceNonceAction,
    claim_version: u64,
    composition_id: String,
    composition_version: u64,
    request_id: String,
}

impl VerifyNonceRequest {
    fn parse(payload: Value, fallback_request_id: &str) -> Result<Self, RequestShapeError> {
        let object = payload.as_object().ok_or_else(|| {
            RequestShapeError::new(
                PresenceNonceRejectReason::MalformedRequest,
                fallback_request_id,
            )
        })?;
        let request_id = optional_string(object.get("feedback_request_id"))
            .or_else(|| optional_string(object.get("request_id")))
            .unwrap_or_else(|| fallback_request_id.to_string());
        let presence_nonce =
            required_string(object.get("presence_nonce"), &request_id).map_err(|_| {
                RequestShapeError::new(PresenceNonceRejectReason::MissingNonce, &request_id)
            })?;
        let claim_version = required_claim_version(object.get("claim_version"), &request_id)?;
        let action =
            PresenceNonceAction::parse(&required_string(object.get("action"), &request_id)?)
                .ok_or_else(|| {
                    RequestShapeError::new(PresenceNonceRejectReason::MalformedRequest, &request_id)
                })?;
        Ok(Self {
            presence_nonce,
            session_id: required_string(object.get("session_id"), &request_id)?,
            wp_user_id: required_u64(object.get("wp_user_id"), &request_id)?,
            claim_id: required_string(object.get("claim_id"), &request_id)?,
            field_path: required_string(object.get("field_path"), &request_id)?,
            action,
            claim_version,
            composition_id: required_string(object.get("composition_id"), &request_id)?,
            composition_version: required_u64(object.get("composition_version"), &request_id)?,
            request_id,
        })
    }

    fn budget_key(&self, surface_client_id: &str, class: NonceBudgetClass) -> NonceBudgetKey {
        NonceBudgetKey::full(
            class,
            surface_client_id,
            self.wp_user_id,
            &self.claim_id,
            &self.field_path,
            self.action,
        )
    }
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?.trim();
    (!raw.is_empty() && raw.len() <= 128).then(|| raw.to_string())
}

fn required_string(value: Option<&Value>, request_id: &str) -> Result<String, RequestShapeError> {
    optional_string(value).ok_or_else(|| {
        RequestShapeError::new(PresenceNonceRejectReason::MalformedRequest, request_id)
    })
}

fn required_u64(value: Option<&Value>, request_id: &str) -> Result<u64, RequestShapeError> {
    value.and_then(Value::as_u64).ok_or_else(|| {
        RequestShapeError::new(PresenceNonceRejectReason::MalformedRequest, request_id)
    })
}

fn required_claim_version(
    value: Option<&Value>,
    request_id: &str,
) -> Result<u64, RequestShapeError> {
    value.and_then(Value::as_u64).ok_or_else(|| {
        RequestShapeError::new(PresenceNonceRejectReason::MalformedClaimVersion, request_id)
    })
}

fn decode_presence_nonce(token: &str) -> Result<Vec<u8>, PresenceNonceRejectReason> {
    if token.trim().is_empty() {
        return Err(PresenceNonceRejectReason::MissingNonce);
    }
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token.as_bytes())
        .map_err(|_| PresenceNonceRejectReason::MalformedRequest)?;
    if decoded.len() != NONCE_BYTES {
        return Err(PresenceNonceRejectReason::MalformedRequest);
    }
    Ok(decoded)
}

#[derive(Clone, Debug)]
struct RequestShapeError {
    reason: PresenceNonceRejectReason,
    request_id: String,
}

impl RequestShapeError {
    fn new(reason: PresenceNonceRejectReason, request_id: &str) -> Self {
        Self {
            reason,
            request_id: request_id.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceNonceIssue {
    pub presence_nonce: String,
    pub expires_at: DateTime<Utc>,
    pub ttl_seconds: u64,
    pub request_id: String,
    pub audit_events: Vec<SurfaceNonceAuditEvent>,
}

#[derive(Clone, Debug)]
pub struct SurfaceNonceVerify {
    pub consumed_at: DateTime<Utc>,
    pub request_id: String,
    pub expected_claim_version: u64,
    pub expected_composition_version: u64,
    pub audit_events: Vec<SurfaceNonceAuditEvent>,
}

#[derive(Clone, Debug)]
pub struct SurfaceNonceError {
    pub reason: PresenceNonceRejectReason,
    pub status: StatusCode,
    pub request_id: String,
    pub retry_after: Option<StdDuration>,
    pub audit_events: Vec<SurfaceNonceAuditEvent>,
    audit: Box<NonceAuditContext>,
}

impl SurfaceNonceError {
    fn rejected(reason: PresenceNonceRejectReason, audit: NonceAuditContext) -> Self {
        let event = audit_event(
            "presence_nonce_rejected",
            &audit.session,
            audit.clone(),
            "rejected",
            Some(reason),
        );
        Self {
            reason,
            status: reason.status(),
            request_id: audit.request_id.clone(),
            retry_after: None,
            audit_events: vec![event],
            audit: Box::new(audit),
        }
    }

    fn rate_limited(audit: NonceAuditContext, retry_after: StdDuration) -> Self {
        let reason = PresenceNonceRejectReason::RateLimited;
        let event = audit_event(
            "presence_nonce_rejected",
            &audit.session,
            audit.clone(),
            "rejected",
            Some(reason),
        );
        Self {
            reason,
            status: reason.status(),
            request_id: audit.request_id.clone(),
            retry_after: Some(retry_after),
            audit_events: vec![event],
            audit: Box::new(audit),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresenceNonceRejectReason {
    MalformedRequest,
    MissingNonce,
    MalformedClaimVersion,
    UnauthenticatedSurface,
    WrongActor,
    ScopeDenied,
    WrongSession,
    WrongUser,
    WrongClaim,
    WrongField,
    MismatchedAction,
    Expired,
    Replayed,
    Invalidated,
    ClaimVersionStale,
    CompositionVersionStale,
    RateLimited,
}

impl PresenceNonceRejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MalformedRequest => "malformed_request",
            Self::MissingNonce => "missing_nonce",
            Self::MalformedClaimVersion => "malformed_claim_version",
            Self::UnauthenticatedSurface => "unauthenticated_surface",
            Self::WrongActor => "wrong_actor",
            Self::ScopeDenied => "scope_denied",
            Self::WrongSession => "wrong_session",
            Self::WrongUser => "wrong_user",
            Self::WrongClaim => "wrong_claim",
            Self::WrongField => "wrong_field",
            Self::MismatchedAction => "mismatched_action",
            Self::Expired => "expired",
            Self::Replayed => "replayed",
            Self::Invalidated => "invalidated",
            Self::ClaimVersionStale => "claim_version_stale",
            Self::CompositionVersionStale => "composition_version_stale",
            Self::RateLimited => "rate_limited",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::MalformedRequest | Self::MissingNonce | Self::MalformedClaimVersion => {
                StatusCode::BAD_REQUEST
            }
            Self::UnauthenticatedSurface => StatusCode::UNAUTHORIZED,
            Self::ClaimVersionStale | Self::CompositionVersionStale => StatusCode::CONFLICT,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::FORBIDDEN,
        }
    }

    fn rejection_class(self) -> &'static str {
        match self {
            Self::WrongActor | Self::WrongUser | Self::WrongSession => "actor",
            Self::UnauthenticatedSurface => "auth",
            Self::MalformedRequest | Self::MissingNonce | Self::MalformedClaimVersion => "shape",
            Self::WrongClaim | Self::WrongField | Self::MismatchedAction => "binding",
            Self::Expired | Self::Replayed | Self::Invalidated => "lifecycle",
            Self::ClaimVersionStale | Self::CompositionVersionStale => "freshness",
            Self::RateLimited => "budget",
            Self::ScopeDenied => "auth",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceNonceAuditEvent {
    pub event_kind: &'static str,
    pub actor: Actor,
    pub wp_user_id: Option<u64>,
    pub wp_user_hash: Option<String>,
    pub detail: Value,
}

impl SurfaceNonceAuditEvent {
    fn with_count(mut self, count: usize) -> Self {
        self.detail["count"] = json!(count);
        self
    }

    fn with_reason(mut self, reason: &'static str) -> Self {
        self.detail["reason"] = json!(reason);
        self
    }
}

#[derive(Clone, Debug)]
struct NonceAuditContext {
    session: ValidatedSurfaceSession,
    request_id: String,
    nonce_digest: Option<NonceDigest>,
    claim_id: Option<String>,
    field_path: Option<String>,
    presented_claim_version: Option<u64>,
    composition_id: Option<String>,
    presented_composition_version: Option<u64>,
    action: Option<PresenceNonceAction>,
    current_claim_version: Option<u64>,
    current_composition_version: Option<u64>,
    now: DateTime<Utc>,
}

impl NonceAuditContext {
    fn from_session(session: &ValidatedSurfaceSession, request_id: &str) -> Self {
        Self {
            session: session.clone(),
            request_id: request_id.to_string(),
            nonce_digest: None,
            claim_id: None,
            field_path: None,
            presented_claim_version: None,
            composition_id: None,
            presented_composition_version: None,
            action: None,
            current_claim_version: None,
            current_composition_version: None,
            now: Utc::now(),
        }
    }

    fn from_issue(session: &ValidatedSurfaceSession, request: &IssueNonceRequest) -> Self {
        let mut audit = Self::from_session(session, &request.request_id);
        audit.claim_id = Some(request.claim_id.clone());
        audit.field_path = Some(request.field_path.clone());
        audit.presented_claim_version = Some(request.claim_version);
        audit.composition_id = Some(request.composition_id.clone());
        audit.presented_composition_version = Some(request.composition_version);
        audit.action = Some(request.action);
        audit
    }

    fn from_verify(session: &ValidatedSurfaceSession, request: &VerifyNonceRequest) -> Self {
        let mut audit = Self::from_session(session, &request.request_id);
        audit.claim_id = Some(request.claim_id.clone());
        audit.field_path = Some(request.field_path.clone());
        audit.presented_claim_version = Some(request.claim_version);
        audit.composition_id = Some(request.composition_id.clone());
        audit.presented_composition_version = Some(request.composition_version);
        audit.action = Some(request.action);
        audit
    }

    fn with_nonce_digest(mut self, digest: NonceDigest) -> Self {
        self.nonce_digest = Some(digest);
        self
    }

    fn with_current_versions(
        mut self,
        claim_version: Option<u64>,
        composition_version: Option<u64>,
    ) -> Self {
        self.current_claim_version = claim_version;
        self.current_composition_version = composition_version;
        self
    }

    fn with_composition(mut self, composition_id: String, composition_version: u64) -> Self {
        self.composition_id = Some(composition_id);
        self.presented_composition_version = Some(composition_version);
        self
    }
}

fn audit_event(
    event_kind: &'static str,
    session: &ValidatedSurfaceSession,
    audit: NonceAuditContext,
    result: &'static str,
    reason: Option<PresenceNonceRejectReason>,
) -> SurfaceNonceAuditEvent {
    let mut detail = json!({
        "request_id": audit.request_id,
        "surface_client_id": session.surface_client_id,
        "session_id_hash": session_hash(&session.session_id),
        "nonce_digest_prefix": audit.nonce_digest.as_ref().map(NonceDigest::prefix),
        "claim_id": audit.claim_id,
        "field_path": audit.field_path,
        "claim_version": audit.presented_claim_version,
        "composition_id": audit.composition_id,
        "composition_version": audit.presented_composition_version,
        "action": audit.action.map(PresenceNonceAction::as_str),
        "result": result,
    });
    if let Some(reason) = reason {
        detail["reason"] = json!(reason.as_str());
        detail["rejection_class"] = json!(reason.rejection_class());
    }
    if let Some(current_claim_version) = audit.current_claim_version {
        detail["current_claim_version"] = json!(current_claim_version);
    }
    if let Some(current_composition_version) = audit.current_composition_version {
        detail["current_composition_version"] = json!(current_composition_version);
    }
    SurfaceNonceAuditEvent {
        event_kind,
        actor: session.actor.clone(),
        wp_user_id: session.wp_user_id,
        wp_user_hash: session.wp_user_hash.clone(),
        detail,
    }
}

fn session_hash(session_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"DAILYOS-PRESENCE-NONCE-SESSION-V1\n");
    hasher.update(session_id.as_bytes());
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(&digest[..16]))
}

fn ensure_surface_client(
    session: &ValidatedSurfaceSession,
    request_id: &str,
) -> Result<(), SurfaceNonceError> {
    if !matches!(session.actor, Actor::SurfaceClient { .. }) {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongActor,
            NonceAuditContext::from_session(session, request_id),
        ));
    }
    if session.wp_user_id.is_none() {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongUser,
            NonceAuditContext::from_session(session, request_id),
        ));
    }
    Ok(())
}

fn ensure_session_tuple(
    session: &ValidatedSurfaceSession,
    request_session_id: &str,
    request_wp_user_id: u64,
    audit: &NonceAuditContext,
) -> Result<(), SurfaceNonceError> {
    if session.session_id != request_session_id {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongSession,
            audit.clone(),
        ));
    }
    if session.wp_user_id != Some(request_wp_user_id) {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongUser,
            audit.clone(),
        ));
    }
    Ok(())
}

fn compare_binding_tuple(
    binding: &PresenceNonceBinding,
    request: &VerifyNonceRequest,
    audit: &NonceAuditContext,
) -> Result<(), SurfaceNonceError> {
    if binding.fields.surface_client_id != audit.session.surface_client_id {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongActor,
            audit.clone(),
        ));
    }
    if binding.fields.session_id != request.session_id {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongSession,
            audit.clone(),
        ));
    }
    if binding.fields.wp_user_id != request.wp_user_id {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongUser,
            audit.clone(),
        ));
    }
    if binding.fields.claim_id != request.claim_id {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongClaim,
            audit.clone(),
        ));
    }
    if binding.fields.field_path != request.field_path {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::WrongField,
            audit.clone(),
        ));
    }
    if binding.fields.action != request.action {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::MismatchedAction,
            audit.clone(),
        ));
    }
    if binding.fields.claim_version != request.claim_version {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::ClaimVersionStale,
            audit.clone(),
        ));
    }
    if binding.fields.composition_id != request.composition_id
        || binding.fields.composition_version != request.composition_version
    {
        return Err(SurfaceNonceError::rejected(
            PresenceNonceRejectReason::CompositionVersionStale,
            audit.clone(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum NonceBudgetClass {
    Issue,
    Verify,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NonceBudgetKey {
    class: NonceBudgetClass,
    key: String,
}

impl NonceBudgetKey {
    fn full(
        class: NonceBudgetClass,
        surface_client_id: &str,
        wp_user_id: u64,
        claim_id: &str,
        field_path: &str,
        action: PresenceNonceAction,
    ) -> Self {
        Self {
            class,
            key: format!(
                "surface_client={surface_client_id}|wp_user_id={wp_user_id}|claim_id={claim_id}|field_path={field_path}|action={}",
                action.as_str()
            ),
        }
    }

    fn narrow(session: &ValidatedSurfaceSession, class: NonceBudgetClass) -> Self {
        Self {
            class,
            key: format!(
                "surface_client={}|session_id={}",
                session.surface_client_id, session.session_id
            ),
        }
    }
}

#[derive(Default)]
struct NonceBudgetState {
    buckets: HashMap<NonceBudgetKey, NonceBudgetBucket>,
}

impl NonceBudgetState {
    fn check_and_consume(
        &mut self,
        key: NonceBudgetKey,
        requests_per_minute: u32,
        now: DateTime<Utc>,
    ) -> Result<(), StdDuration> {
        let bucket = self
            .buckets
            .entry(key)
            .or_insert_with(|| NonceBudgetBucket {
                window_started_at: now,
                consumed: 0,
            });
        if now - bucket.window_started_at >= Duration::seconds(60) {
            bucket.window_started_at = now;
            bucket.consumed = 0;
        }
        if bucket.consumed >= requests_per_minute {
            let elapsed = now - bucket.window_started_at;
            let retry = Duration::seconds(60)
                .checked_sub(&elapsed)
                .unwrap_or_else(|| Duration::seconds(1));
            return Err(retry.to_std().unwrap_or_else(|_| StdDuration::from_secs(1)));
        }
        bucket.consumed += 1;
        Ok(())
    }
}

struct NonceBudgetBucket {
    window_started_at: DateTime<Utc>,
    consumed: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use abilities_runtime::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};
    use chrono::TimeZone;
    use rusqlite::params;

    use crate::db::claims::{ClaimSensitivity, TemporalScope};
    use crate::db::ActionDb;
    use crate::services::claims::{ClaimProposal, CommittedClaim, DeterministicInsertProposal};
    use crate::services::context::{
        Clock, ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };

    struct DeterministicPresenceNonceRandom(ParkingMutex<u8>);

    impl PresenceNonceRandom for DeterministicPresenceNonceRandom {
        fn fill_nonce(&self, bytes: &mut [u8; NONCE_BYTES]) -> Result<(), SurfaceNonceError> {
            let mut next = self.0.lock();
            for byte in bytes.iter_mut() {
                *byte = *next;
                *next = next.wrapping_add(1);
            }
            Ok(())
        }
    }

    fn service(config: SurfaceNonceConfig) -> SurfaceNonceService {
        SurfaceNonceService::with_config_and_rng(
            config,
            [9_u8; 32],
            Arc::new(DeterministicPresenceNonceRandom(ParkingMutex::new(1))),
        )
        .expect("service")
    }

    fn session(session_id: &str, wp_user_id: u64) -> ValidatedSurfaceSession {
        let scopes = ScopeSet::new([SurfaceScope::new("submit.feedback")]).expect("scopes");
        ValidatedSurfaceSession {
            surface_client_id: "surface-client-test".to_string(),
            session_id: session_id.to_string(),
            actor: Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface-client-test"),
                scopes,
            },
            wp_user_id: Some(wp_user_id),
            wp_user_hash: Some("wp-user-hash".to_string()),
            wp_site_id: "site-1".to_string(),
            wp_site_id_hash: "site-hash".to_string(),
            site_binding_digest: "site-digest".to_string(),
            site_nonce: "site-nonce".to_string(),
            scope_digest: "scope-digest".to_string(),
            granted_scopes: vec!["submit.feedback".to_string()],
        }
    }

    fn ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext).with_actor("surface_client")
    }

    fn seed_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nonce.sqlite");
        std::mem::forget(dir);
        let db = ActionDb::open_at_unencrypted(path).expect("db");
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-1", "Account 1", "2026-05-13T12:00:00Z"],
            )
            .expect("account");
        seed_claim_version(&db, 7);
        db.conn_ref()
            .execute(
                "INSERT INTO composition_versions (
                    composition_id, composition_version, generated_at,
                    generated_by_invocation_id, generated_by_actor_kind
                 ) VALUES ('composition-1', 17, '2026-05-13T12:00:00Z', 'inv-1', 'agent')",
                [],
            )
            .expect("composition");
        db
    }

    fn claim_proposal(source_ref: &str) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: serde_json::json!({"kind": "account", "id": "acct-1"}).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("claims[0].summary".to_string()),
            topic_key: None,
            text: "Claim text".to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some(source_ref.to_string()),
            source_asof: Some("2026-05-13T12:00:00Z".to_string()),
            observed_at: "2026-05-13T12:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn seed_claim_version(db: &ActionDb, target_version: u64) {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(99);
        let ext = ExternalClients::default();
        let ctx = seed_ctx(&clock, &rng, &ext);
        let inserted = claims::commit_claim(
            &ctx,
            db,
            DeterministicInsertProposal::new("claim-1".to_string(), claim_proposal("source-1")),
        )
        .expect("seed claim");
        assert_claim_version(inserted, 1);
        reinforce_claim_to_version(db, target_version);
    }

    fn reinforce_claim_to_version(db: &ActionDb, target_version: u64) {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(100);
        let ext = ExternalClients::default();
        let ctx = seed_ctx(&clock, &rng, &ext);
        let current = claims::current_claim_version_for_claim_id(&ctx, db, "claim-1")
            .expect("current claim version");
        for version in (current + 1)..=target_version {
            let committed =
                claims::commit_claim(&ctx, db, claim_proposal(&format!("source-{version}")))
                    .expect("reinforce claim");
            assert_claim_version(committed, version);
        }
    }

    fn assert_claim_version(committed: CommittedClaim, expected: u64) {
        match committed {
            CommittedClaim::Inserted { claim } | CommittedClaim::Reinforced { claim, .. } => {
                assert_eq!(claim.id, "claim-1");
                assert_eq!(claim.claim_version, expected);
            }
            other => panic!("expected inserted/reinforced claim, got {other:?}"),
        }
    }

    fn issue_payload() -> Value {
        json!({
            "session_id": "session-1",
            "wp_user_id": 42,
            "claim_id": "claim-1",
            "field_path": "claims[0].summary",
            "action": "correct",
            "claim_version": 7,
            "composition_id": "composition-1",
            "composition_version": 17,
            "request_id": "request-1"
        })
    }

    fn verify_payload(token: &str) -> Value {
        json!({
            "presence_nonce": token,
            "session_id": "session-1",
            "wp_user_id": 42,
            "claim_id": "claim-1",
            "field_path": "claims[0].summary",
            "action": "correct",
            "claim_version": 7,
            "composition_id": "composition-1",
            "composition_version": 17,
            "feedback_request_id": "request-2"
        })
    }

    fn issue_token(
        service: &SurfaceNonceService,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        session: &ValidatedSurfaceSession,
        request_id: &str,
    ) -> String {
        let mut payload = issue_payload();
        payload["request_id"] = json!(request_id);
        service
            .issue_nonce(ctx, db, session, payload, request_id)
            .expect("issue")
            .presence_nonce
    }

    fn assert_verify_rejects(
        service: &SurfaceNonceService,
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        session: &ValidatedSurfaceSession,
        payload: Value,
        expected_reason: PresenceNonceRejectReason,
    ) -> SurfaceNonceError {
        let error = service
            .verify_nonce(ctx, db, session, payload, "request-reject")
            .expect_err("verify rejection");
        assert_eq!(error.reason, expected_reason);
        error
    }

    #[test]
    fn dos571_fixture_lifecycle_cas() {
        let mut binding = PresenceNonceBinding::new(
            PresenceNonceBindingFields {
                surface_client_id: "sc".into(),
                session_id: "session".into(),
                wp_user_id: 1,
                claim_id: "claim".into(),
                field_path: "field".into(),
                action: PresenceNonceAction::Correct,
                claim_version: 1,
                composition_id: "composition".into(),
                composition_version: 1,
                generated_at: Utc::now(),
                expires_at: Utc::now(),
            },
            NonceDigest([1_u8; DIGEST_BYTES]),
        );
        let now = Utc::now();
        assert!(binding.try_mark_consumed(now).is_ok());
        assert_eq!(
            binding.try_mark_invalidated(now, InvalidationReason::StorePressure),
            Err(LifecycleRace::AlreadyConsumed)
        );
    }

    #[test]
    fn issue_and_verify_consumes_nonce_once() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);

        let issued = service
            .issue_nonce(&ctx, &db, &session, issue_payload(), "request-1")
            .expect("issue");
        let verified = service
            .verify_nonce(
                &ctx,
                &db,
                &session,
                verify_payload(&issued.presence_nonce),
                "request-2",
            )
            .expect("verify");
        assert_eq!(verified.expected_claim_version, 7);
        assert_eq!(verified.expected_composition_version, 17);

        let replay = service
            .verify_nonce(
                &ctx,
                &db,
                &session,
                verify_payload(&issued.presence_nonce),
                "request-3",
            )
            .expect_err("replay");
        assert_eq!(replay.reason, PresenceNonceRejectReason::Replayed);
        assert_eq!(replay.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn dos571_fixture_expired_nonce() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig {
            ttl: Duration::seconds(1),
            ..SurfaceNonceConfig::default()
        });
        let session = session("session-1", 42);
        let token = issue_token(&service, &ctx, &db, &session, "request-expired");

        clock.advance(Duration::seconds(2));

        let error = assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            verify_payload(&token),
            PresenceNonceRejectReason::Expired,
        );
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn dos571_fixture_binding_tuple_rejections() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);

        let token = issue_token(&service, &ctx, &db, &session, "request-wrong-user");
        let mut payload = verify_payload(&token);
        payload["wp_user_id"] = json!(43);
        assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::WrongUser,
        );

        let token = issue_token(&service, &ctx, &db, &session, "request-wrong-session");
        let mut payload = verify_payload(&token);
        payload["session_id"] = json!("session-2");
        assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::WrongSession,
        );

        let token = issue_token(&service, &ctx, &db, &session, "request-wrong-action");
        let mut payload = verify_payload(&token);
        payload["action"] = json!("dismiss");
        assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::MismatchedAction,
        );

        let token = issue_token(&service, &ctx, &db, &session, "request-wrong-field");
        let mut payload = verify_payload(&token);
        payload["field_path"] = json!("claims[1].summary");
        assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::WrongField,
        );

        let token = issue_token(&service, &ctx, &db, &session, "request-wrong-claim");
        let mut payload = verify_payload(&token);
        payload["claim_id"] = json!("claim-2");
        assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::WrongClaim,
        );
    }

    #[test]
    fn dos571_fixture_missing_and_unknown_nonce_rejections() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);

        let token = issue_token(&service, &ctx, &db, &session, "request-missing");
        let mut payload = verify_payload(&token);
        payload.as_object_mut().unwrap().remove("presence_nonce");
        let error = assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::MissingNonce,
        );
        assert_eq!(error.status, StatusCode::BAD_REQUEST);

        let unknown = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0_u8; NONCE_BYTES]);
        let error = assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            verify_payload(&unknown),
            PresenceNonceRejectReason::Invalidated,
        );
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn dos571_fixture_missing_composition_version_rejects_before_issue() {
        let mut payload = issue_payload();
        payload
            .as_object_mut()
            .unwrap()
            .remove("composition_version");
        let error =
            IssueNonceRequest::parse(payload, "request").expect_err("missing composition version");
        assert_eq!(error.reason, PresenceNonceRejectReason::MalformedRequest);
    }

    #[test]
    fn dos571_fixture_composition_refresh_invalidates_prior_outstanding_nonce() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);
        let token = issue_token(&service, &ctx, &db, &session, "request-refresh");

        let audit_events = service.invalidate_composition(
            &session,
            "composition-1",
            18,
            clock.now(),
            "request-refresh-event",
        );
        assert!(audit_events.iter().any(|event| {
            event.event_kind == "presence_nonce_invalidated" && event.detail["count"] == 1
        }));

        let error = assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            verify_payload(&token),
            PresenceNonceRejectReason::CompositionVersionStale,
        );
        assert_eq!(error.status, StatusCode::CONFLICT);
    }

    #[test]
    fn dos571_fixture_reject_audit_shape_is_safe() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);
        let token = issue_token(&service, &ctx, &db, &session, "request-audit");
        let mut payload = verify_payload(&token);
        payload["action"] = json!("dismiss");

        let error = assert_verify_rejects(
            &service,
            &ctx,
            &db,
            &session,
            payload,
            PresenceNonceRejectReason::MismatchedAction,
        );
        let event = error.audit_events.first().expect("audit event");
        assert_eq!(event.event_kind, "presence_nonce_rejected");
        assert_eq!(event.detail["reason"], "mismatched_action");
        assert_eq!(event.detail["rejection_class"], "binding");
        assert!(event.detail.get("presence_nonce").is_none());
        assert!(event.detail.get("site_nonce").is_none());
        assert!(event.detail.get("hmac_key").is_none());
        assert!(event.detail.get("session_id").is_none());
        assert!(event.detail.get("session_id_hash").is_some());
    }

    #[test]
    fn dos571_fixture_claim_version_missing() {
        let mut payload = issue_payload();
        payload.as_object_mut().unwrap().remove("claim_version");
        let error = IssueNonceRequest::parse(payload, "request").expect_err("missing");
        assert_eq!(
            error.reason,
            PresenceNonceRejectReason::MalformedClaimVersion
        );
    }

    #[test]
    fn dos571_fixture_claim_version_wrong_type() {
        for value in [
            json!(""),
            json!("7"),
            json!(null),
            json!({"value": 7}),
            json!(7.5),
            json!(-1),
        ] {
            let mut payload = issue_payload();
            payload["claim_version"] = value;
            let error = IssueNonceRequest::parse(payload, "request").expect_err("wrong type");
            assert_eq!(
                error.reason,
                PresenceNonceRejectReason::MalformedClaimVersion
            );
        }
    }

    #[test]
    fn dos571_fixture_issue_rate_exhaustion() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig {
            max_outstanding_per_session: 32,
            ..SurfaceNonceConfig::default()
        });
        let session = session("session-1", 42);
        let mut rejected_at = None;
        for attempt in 0..1000 {
            let mut payload = issue_payload();
            payload["request_id"] = json!(format!("request-{attempt}"));
            match service.issue_nonce(&ctx, &db, &session, payload, "request") {
                Ok(_) => {}
                Err(error) => {
                    assert_eq!(error.reason, PresenceNonceRejectReason::RateLimited);
                    rejected_at = Some(attempt);
                    break;
                }
            }
        }
        assert!(rejected_at.is_some_and(|attempt| attempt < 1000));
    }

    #[test]
    fn dos571_fixture_store_pressure_lru() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig {
            max_live_bindings: 1,
            max_outstanding_per_session: 10,
            ..SurfaceNonceConfig::default()
        });
        let session = session("session-1", 42);
        let first = service
            .issue_nonce(&ctx, &db, &session, issue_payload(), "request-1")
            .expect("first");
        let mut second_payload = issue_payload();
        second_payload["request_id"] = json!("request-2");
        let second = service
            .issue_nonce(&ctx, &db, &session, second_payload, "request-2")
            .expect("second");
        assert!(second.audit_events.iter().any(|event| {
            event.event_kind == "presence_nonce_invalidated"
                && event.detail["reason"] == "store_pressure"
        }));
        let error = service
            .verify_nonce(
                &ctx,
                &db,
                &session,
                verify_payload(&first.presence_nonce),
                "request-3",
            )
            .expect_err("evicted");
        assert_eq!(error.reason, PresenceNonceRejectReason::Invalidated);
    }

    #[test]
    fn dos571_fixture_claim_version_drift() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);
        let issued = service
            .issue_nonce(&ctx, &db, &session, issue_payload(), "request-1")
            .expect("issue");
        reinforce_claim_to_version(&db, 8);
        let error = service
            .verify_nonce(
                &ctx,
                &db,
                &session,
                verify_payload(&issued.presence_nonce),
                "request-2",
            )
            .expect_err("stale");
        assert_eq!(error.reason, PresenceNonceRejectReason::ClaimVersionStale);
        assert_eq!(error.status, StatusCode::CONFLICT);
    }

    #[test]
    fn dos571_fixture_lazy_invalidation() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let session = session("session-1", 42);
        let issued = service
            .issue_nonce(&ctx, &db, &session, issue_payload(), "request-1")
            .expect("issue");
        db.conn_ref().execute("UPDATE composition_versions SET composition_version = 18 WHERE composition_id = 'composition-1'", []).expect("bump");
        let error = service
            .verify_nonce(
                &ctx,
                &db,
                &session,
                verify_payload(&issued.presence_nonce),
                "request-2",
            )
            .expect_err("stale");
        assert_eq!(
            error.reason,
            PresenceNonceRejectReason::CompositionVersionStale
        );
        assert_eq!(error.status, StatusCode::CONFLICT);
    }

    #[test]
    fn dos571_fixture_wrong_actor() {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(1);
        let ext = ExternalClients::default();
        let ctx = ctx(&clock, &rng, &ext);
        let db = db();
        let service = service(SurfaceNonceConfig::default());
        let mut bad_session = session("session-1", 42);
        bad_session.actor = Actor::System;

        let error = service
            .issue_nonce(&ctx, &db, &bad_session, issue_payload(), "request")
            .expect_err("wrong actor issue");
        assert_eq!(error.reason, PresenceNonceRejectReason::WrongActor);

        let error = service
            .verify_nonce(&ctx, &db, &bad_session, verify_payload("token"), "request")
            .expect_err("wrong actor verify");
        assert_eq!(error.reason, PresenceNonceRejectReason::WrongActor);
    }
}
