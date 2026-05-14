use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use ring::hmac;
use serde::Serialize;
use serde_json::json;

use crate::abilities::registry::{AbilityRateLimit, ScopeSet, SurfaceClientId, SurfaceScope};
use crate::abilities::{AbilityCategory, AbilityDescriptor, AbilityRegistry, Actor, ActorKind};
use crate::bridges::BridgeSurfaceError;
use crate::services::context::ExecutionMode;
use crate::services::surface_pairing::{SurfacePairingAuditEvent, ValidatedSurfaceSession};

const EARLY_RETRY_TIGHTEN_DIVISOR: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClientRateLimitAxis {
    SurfaceClient,
    WpSite,
    WpUser,
    Scope,
    Ability,
}

impl SurfaceClientRateLimitAxis {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SurfaceClient => "surface_client",
            Self::WpSite => "wp_site",
            Self::WpUser => "wp_user",
            Self::Scope => "scope",
            Self::Ability => "ability",
        }
    }

    const fn precedence(self) -> u8 {
        match self {
            Self::SurfaceClient => 0,
            Self::WpSite => 1,
            Self::WpUser => 2,
            Self::Scope => 3,
            Self::Ability => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClientRequestClass {
    Read,
    Write,
    Admin,
}

impl SurfaceClientRequestClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceClientAbilityClass {
    CheapRead,
    StandardReadComposition,
    HeavyTransform,
    FeedbackWrite,
    AdminAbility,
}

impl SurfaceClientAbilityClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CheapRead => "cheap_read",
            Self::StandardReadComposition => "standard_read_composition",
            Self::HeavyTransform => "heavy_transform",
            Self::FeedbackWrite => "feedback_write",
            Self::AdminAbility => "admin_ability",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceClientRateLimitBudget {
    pub requests_per_minute: u32,
    pub burst_per_second: u32,
}

impl SurfaceClientRateLimitBudget {
    const fn new(requests_per_minute: u32, burst_per_second: u32) -> Self {
        Self {
            requests_per_minute,
            burst_per_second,
        }
    }

    fn refill_per_second(self) -> f64 {
        f64::from(self.requests_per_minute.max(1)) / 60.0
    }

    fn capacity(self) -> u32 {
        self.burst_per_second.max(1)
    }

    fn tightened(self) -> Self {
        Self {
            requests_per_minute: (self.requests_per_minute / EARLY_RETRY_TIGHTEN_DIVISOR).max(1),
            burst_per_second: (self.burst_per_second / EARLY_RETRY_TIGHTEN_DIVISOR).max(1),
        }
    }

    fn lower_only(self, override_limit: Option<AbilityRateLimit>) -> Self {
        let Some(override_limit) = override_limit else {
            return self;
        };
        Self {
            requests_per_minute: self
                .requests_per_minute
                .min(override_limit.requests_per_minute.max(1)),
            burst_per_second: self
                .burst_per_second
                .min(override_limit.burst_per_second.max(1)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceClientRequestClassLimits {
    pub read: SurfaceClientRateLimitBudget,
    pub write: SurfaceClientRateLimitBudget,
    pub admin: SurfaceClientRateLimitBudget,
}

impl SurfaceClientRequestClassLimits {
    fn budget(&self, class: SurfaceClientRequestClass) -> SurfaceClientRateLimitBudget {
        match class {
            SurfaceClientRequestClass::Read => self.read,
            SurfaceClientRequestClass::Write => self.write,
            SurfaceClientRequestClass::Admin => self.admin,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceClientAbilityClassLimits {
    pub cheap_read: SurfaceClientRateLimitBudget,
    pub standard_read_composition: SurfaceClientRateLimitBudget,
    pub heavy_transform: SurfaceClientRateLimitBudget,
    pub feedback_write: SurfaceClientRateLimitBudget,
    pub admin_ability: SurfaceClientRateLimitBudget,
}

impl SurfaceClientAbilityClassLimits {
    fn budget(&self, class: SurfaceClientAbilityClass) -> SurfaceClientRateLimitBudget {
        match class {
            SurfaceClientAbilityClass::CheapRead => self.cheap_read,
            SurfaceClientAbilityClass::StandardReadComposition => self.standard_read_composition,
            SurfaceClientAbilityClass::HeavyTransform => self.heavy_transform,
            SurfaceClientAbilityClass::FeedbackWrite => self.feedback_write,
            SurfaceClientAbilityClass::AdminAbility => self.admin_ability,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceClientBridgeConfig {
    pub surface_client: SurfaceClientRequestClassLimits,
    pub wp_user: SurfaceClientRequestClassLimits,
    pub wp_site: SurfaceClientRequestClassLimits,
    pub ability: SurfaceClientAbilityClassLimits,
    pub scope: SurfaceClientRequestClassLimits,
    pub early_retry_tighten_window: Duration,
}

impl Default for SurfaceClientBridgeConfig {
    fn default() -> Self {
        Self {
            surface_client: SurfaceClientRequestClassLimits {
                read: SurfaceClientRateLimitBudget::new(300, 20),
                write: SurfaceClientRateLimitBudget::new(30, 2),
                admin: SurfaceClientRateLimitBudget::new(6, 1),
            },
            wp_user: SurfaceClientRequestClassLimits {
                read: SurfaceClientRateLimitBudget::new(120, 8),
                write: SurfaceClientRateLimitBudget::new(12, 1),
                admin: SurfaceClientRateLimitBudget::new(3, 1),
            },
            wp_site: SurfaceClientRequestClassLimits {
                read: SurfaceClientRateLimitBudget::new(600, 40),
                write: SurfaceClientRateLimitBudget::new(60, 4),
                admin: SurfaceClientRateLimitBudget::new(12, 2),
            },
            ability: SurfaceClientAbilityClassLimits {
                cheap_read: SurfaceClientRateLimitBudget::new(120, 10),
                standard_read_composition: SurfaceClientRateLimitBudget::new(60, 5),
                heavy_transform: SurfaceClientRateLimitBudget::new(12, 2),
                feedback_write: SurfaceClientRateLimitBudget::new(6, 1),
                admin_ability: SurfaceClientRateLimitBudget::new(3, 1),
            },
            scope: SurfaceClientRequestClassLimits {
                read: SurfaceClientRateLimitBudget::new(240, 16),
                write: SurfaceClientRateLimitBudget::new(24, 2),
                admin: SurfaceClientRateLimitBudget::new(6, 1),
            },
            early_retry_tighten_window: Duration::from_secs(5 * 60),
        }
    }
}

pub trait SurfaceClientMonotonicClock: Send + Sync {
    fn now(&self) -> Instant;
}

#[derive(Debug)]
struct SystemSurfaceClientClock;

impl SurfaceClientMonotonicClock for SystemSurfaceClientClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[derive(Clone)]
pub struct SurfaceClientBridge {
    limiter: Arc<SurfaceClientRateLimiter>,
}

impl SurfaceClientBridge {
    pub fn new(config: SurfaceClientBridgeConfig) -> Self {
        Self::with_clock(config, Arc::new(SystemSurfaceClientClock))
    }

    pub fn with_clock(
        config: SurfaceClientBridgeConfig,
        clock: Arc<dyn SurfaceClientMonotonicClock>,
    ) -> Self {
        Self {
            limiter: Arc::new(SurfaceClientRateLimiter::new(config, clock)),
        }
    }

    pub fn authorize(
        &self,
        registry: &AbilityRegistry,
        session: &ValidatedSurfaceSession,
        ability_name: &str,
        request_id: &str,
    ) -> Result<SurfaceClientAuthorization, SurfaceClientBridgeError> {
        let audit_hash_secret = format!("{}:{}", session.session_id, session.site_nonce);
        let Some(descriptor) = registry
            .iter_all()
            .find(|descriptor| descriptor.name == ability_name)
        else {
            let request = identity_rate_limit_request(
                session,
                ability_name,
                request_id,
                SurfaceClientRequestClass::Read,
                SurfaceClientAbilityClass::CheapRead,
                &audit_hash_secret,
            );
            return self.reject_after_identity_rate_limit(
                request,
                SurfaceClientBridgeError::AbilityUnavailable,
            );
        };

        if !descriptor
            .policy
            .allowed_actors
            .contains(&ActorKind::SurfaceClient)
            || !descriptor
                .policy
                .allowed_modes
                .contains(&ExecutionMode::Live)
            || descriptor.experimental
        {
            let request = identity_rate_limit_request(
                session,
                descriptor.name,
                request_id,
                request_class_for_descriptor(descriptor),
                ability_class_for_descriptor(descriptor),
                &audit_hash_secret,
            );
            return self.reject_after_identity_rate_limit(
                request,
                SurfaceClientBridgeError::AbilityUnavailable,
            );
        }

        if !descriptor.policy.client_side_executable {
            let request = identity_rate_limit_request(
                session,
                descriptor.name,
                request_id,
                request_class_for_descriptor(descriptor),
                ability_class_for_descriptor(descriptor),
                &audit_hash_secret,
            );
            return self.reject_after_identity_rate_limit(
                request,
                SurfaceClientBridgeError::AbilityUnavailable,
            );
        }
        if ensure_required_scopes(session, descriptor).is_err() {
            let request = identity_rate_limit_request(
                session,
                descriptor.name,
                request_id,
                request_class_for_descriptor(descriptor),
                ability_class_for_descriptor(descriptor),
                &audit_hash_secret,
            );
            return self
                .reject_after_identity_rate_limit(request, SurfaceClientBridgeError::ScopeDenied);
        }

        let request_class = request_class_for_descriptor(descriptor);
        let ability_class = ability_class_for_descriptor(descriptor);
        let required_scope_hashes = descriptor
            .policy
            .required_scopes
            .iter()
            .map(|scope| privacy_hash("surface-scope", &audit_hash_secret, scope))
            .collect::<Vec<_>>();
        let scope_classes = descriptor
            .policy
            .required_scopes
            .iter()
            .map(|scope| scope_class_for_scope(scope))
            .collect::<Vec<_>>();

        match self
            .limiter
            .check_and_consume(SurfaceClientRateLimitRequest {
                surface_client_id: session.surface_client_id.clone(),
                wp_user_hash: session.wp_user_hash.clone(),
                wp_site_id: session.wp_site_id.clone(),
                wp_site_id_hash: session.wp_site_id_hash.clone(),
                site_binding_digest: session.site_binding_digest.clone(),
                ability_id_hash: privacy_hash(
                    "surface-ability",
                    &audit_hash_secret,
                    descriptor.name,
                ),
                ability_name: descriptor.name.to_string(),
                request_class,
                ability_class,
                scope_classes,
                required_scope_hashes,
                request_id: request_id.to_string(),
                policy_rate_limit: descriptor.policy.rate_limit,
                wp_user_id: session.wp_user_id,
                actor_scopes: session.granted_scopes.clone(),
                charge_ability_scope: true,
            }) {
            RateLimitOutcome::Allowed { audit_events } => Ok(SurfaceClientAuthorization {
                canonical_ability_name: descriptor.name.to_string(),
                request_class,
                ability_class,
                audit_events,
            }),
            RateLimitOutcome::Rejected(rejection) => {
                Err(SurfaceClientBridgeError::RateLimited(Box::new(rejection)))
            }
        }
    }

    fn reject_after_identity_rate_limit(
        &self,
        request: SurfaceClientRateLimitRequest,
        fallback: SurfaceClientBridgeError,
    ) -> Result<SurfaceClientAuthorization, SurfaceClientBridgeError> {
        match self.limiter.check_and_consume(request) {
            RateLimitOutcome::Allowed { .. } => Err(fallback),
            RateLimitOutcome::Rejected(rejection) => {
                Err(SurfaceClientBridgeError::RateLimited(Box::new(rejection)))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrongUserRejection {
    pub asserted_wp_user_id: u64,
    pub session_wp_user_id: Option<u64>,
    pub surface_client_id: String,
}

pub fn validate_session_bound_wp_user_id(
    session: &ValidatedSurfaceSession,
    payload: &serde_json::Value,
) -> Result<(), WrongUserRejection> {
    let mut asserted = Vec::new();
    collect_wp_user_ids(payload, &mut asserted);
    for asserted_wp_user_id in asserted {
        if session.wp_user_id != Some(asserted_wp_user_id) {
            return Err(WrongUserRejection {
                asserted_wp_user_id,
                session_wp_user_id: session.wp_user_id,
                surface_client_id: session.surface_client_id.clone(),
            });
        }
    }
    Ok(())
}

/// Channels through which a SurfaceClient request can carry a `wp_user_id`
/// assertion. Per packet §17 + L2 cycle-2 M1, ALL channels must be checked
/// before dispatch — the JSON-body-only walker was a class shape bug
/// (the spec literally enumerates "body, query string, header, or any
/// other request channel"; class-pattern says enumerate-then-centralise).
#[derive(Clone, Copy, Debug)]
pub enum WpUserIdChannel {
    Body,
    Query,
    Header,
}

/// Request-aware wrapper around `validate_session_bound_wp_user_id`. Walks
/// the JSON body (if any), the URL query string (form-urlencoded), and the
/// case-insensitive `wp_user_id` / `x-wp-user-id` headers. First mismatch
/// short-circuits with a `WrongUserRejection` carrying the channel that
/// produced the asserted value.
pub fn validate_session_bound_wp_user_id_for_request(
    session: &ValidatedSurfaceSession,
    body_payload: Option<&serde_json::Value>,
    query: Option<&str>,
    headers: &http::HeaderMap,
) -> Result<(), WrongUserRejection> {
    if let Some(body) = body_payload {
        let mut asserted = Vec::new();
        collect_wp_user_ids(body, &mut asserted);
        check_channel(session, asserted, WpUserIdChannel::Body)?;
    }
    if let Some(query) = query {
        let asserted = parse_wp_user_ids_from_query(query);
        check_channel(session, asserted, WpUserIdChannel::Query)?;
    }
    let header_asserted = parse_wp_user_ids_from_headers(headers);
    check_channel(session, header_asserted, WpUserIdChannel::Header)?;
    Ok(())
}

fn check_channel(
    session: &ValidatedSurfaceSession,
    asserted: Vec<u64>,
    _channel: WpUserIdChannel,
) -> Result<(), WrongUserRejection> {
    for asserted_wp_user_id in asserted {
        if session.wp_user_id != Some(asserted_wp_user_id) {
            return Err(WrongUserRejection {
                asserted_wp_user_id,
                session_wp_user_id: session.wp_user_id,
                surface_client_id: session.surface_client_id.clone(),
            });
        }
    }
    Ok(())
}

fn parse_wp_user_ids_from_query(query: &str) -> Vec<u64> {
    let mut out = Vec::new();
    for pair in query.split('&') {
        let mut iter = pair.splitn(2, '=');
        let key = iter.next().unwrap_or("");
        let raw = iter.next().unwrap_or("");
        if key.eq_ignore_ascii_case("wp_user_id") {
            // Lenient form-urlencode: only '+' → ' ' and %XX → byte. We
            // only need the digits.
            let decoded = url_decode_minimal(raw);
            if let Ok(id) = decoded.trim().parse::<u64>() {
                out.push(id);
            }
        }
    }
    out
}

fn url_decode_minimal(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            out.push(' ');
            i += 1;
        } else if b == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            match u8::from_str_radix(hex, 16) {
                Ok(decoded) => {
                    out.push(decoded as char);
                    i += 3;
                }
                Err(_) => {
                    out.push(b as char);
                    i += 1;
                }
            }
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn parse_wp_user_ids_from_headers(headers: &http::HeaderMap) -> Vec<u64> {
    let mut out = Vec::new();
    for name in &["wp_user_id", "x-wp-user-id"] {
        for value in headers.get_all(*name).iter() {
            if let Ok(raw) = value.to_str() {
                if let Ok(id) = raw.trim().parse::<u64>() {
                    out.push(id);
                }
            }
        }
    }
    out
}

fn collect_wp_user_ids(value: &serde_json::Value, output: &mut Vec<u64>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if key == "wp_user_id" {
                    if let Some(id) = value.as_u64().or_else(|| {
                        value
                            .as_str()
                            .and_then(|raw| raw.trim().parse::<u64>().ok())
                    }) {
                        output.push(id);
                    }
                }
                collect_wp_user_ids(value, output);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_wp_user_ids(value, output);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceClientAuthorization {
    pub canonical_ability_name: String,
    pub request_class: SurfaceClientRequestClass,
    pub ability_class: SurfaceClientAbilityClass,
    pub audit_events: Vec<SurfacePairingAuditEvent>,
}

#[derive(Clone, Debug)]
pub enum SurfaceClientBridgeError {
    AbilityUnavailable,
    ScopeDenied,
    RateLimited(Box<SurfaceClientRateLimitRejection>),
}

impl SurfaceClientBridgeError {
    pub fn as_surface_error(&self) -> Option<BridgeSurfaceError> {
        match self {
            Self::AbilityUnavailable | Self::ScopeDenied => {
                Some(BridgeSurfaceError::AbilityUnavailable)
            }
            Self::RateLimited(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceClientRateLimitRejection {
    pub axis: SurfaceClientRateLimitAxis,
    pub retry_after: Duration,
    pub audit_event: SurfacePairingAuditEvent,
}

#[derive(Clone, Debug)]
pub struct SurfaceClientRateLimitRequest {
    pub surface_client_id: String,
    pub wp_user_hash: Option<String>,
    pub wp_site_id: String,
    pub wp_site_id_hash: String,
    pub site_binding_digest: String,
    pub ability_id_hash: String,
    pub ability_name: String,
    pub request_class: SurfaceClientRequestClass,
    pub ability_class: SurfaceClientAbilityClass,
    pub scope_classes: Vec<SurfaceClientRequestClass>,
    pub required_scope_hashes: Vec<String>,
    pub request_id: String,
    pub policy_rate_limit: Option<AbilityRateLimit>,
    pub wp_user_id: Option<u64>,
    pub actor_scopes: Vec<String>,
    pub charge_ability_scope: bool,
}

enum RateLimitOutcome {
    Allowed {
        audit_events: Vec<SurfacePairingAuditEvent>,
    },
    Rejected(SurfaceClientRateLimitRejection),
}

struct SurfaceClientRateLimiter {
    config: SurfaceClientBridgeConfig,
    clock: Arc<dyn SurfaceClientMonotonicClock>,
    inner: Mutex<RateLimiterState>,
}

impl SurfaceClientRateLimiter {
    fn new(config: SurfaceClientBridgeConfig, clock: Arc<dyn SurfaceClientMonotonicClock>) -> Self {
        Self {
            config,
            clock,
            inner: Mutex::new(RateLimiterState::default()),
        }
    }

    fn check_and_consume(&self, request: SurfaceClientRateLimitRequest) -> RateLimitOutcome {
        let now = self.clock.now();
        let mut inner = self.inner.lock();
        let early_retry = inner
            .retry_after_until
            .get(&request.surface_client_id)
            .is_some_and(|deadline| now < *deadline);
        let surface_tightened = inner
            .tightened_until
            .get(&request.surface_client_id)
            .is_some_and(|deadline| now < *deadline)
            || early_retry;

        let candidates = self.candidates(&request, surface_tightened);
        let mut checked = Vec::with_capacity(candidates.len());
        let mut exhausted = Vec::new();
        for candidate in &candidates {
            let mut bucket = inner
                .buckets
                .get(&candidate.key)
                .cloned()
                .unwrap_or_else(|| RateLimitBucket::new(candidate.budget, now));
            bucket.reconfigure(candidate.budget, now);
            if let Some(retry_after) = bucket.retry_after(now) {
                exhausted.push((candidate.axis, retry_after));
            }
            checked.push((candidate.key.clone(), bucket));
        }

        if !exhausted.is_empty() {
            let (axis, retry_after) = choose_public_exhausted_axis(&exhausted);
            inner.retry_after_until.insert(
                request.surface_client_id.clone(),
                now + retry_after.max(Duration::from_millis(1)),
            );
            return RateLimitOutcome::Rejected(SurfaceClientRateLimitRejection {
                axis,
                retry_after,
                audit_event: rate_limit_audit_event(&request, "rejected", Some(axis), retry_after),
            });
        }

        if early_retry {
            inner.tightened_until.insert(
                request.surface_client_id.clone(),
                now + self.config.early_retry_tighten_window,
            );
        }
        for (key, bucket) in checked {
            inner.buckets.insert(key, bucket);
        }
        for candidate in &candidates {
            let bucket = inner
                .buckets
                .get_mut(&candidate.key)
                .expect("candidate bucket exists after precheck");
            bucket.consume_one();
        }

        let audit_events = if early_retry {
            vec![rate_limit_audit_event(
                &request,
                "early_retry",
                None,
                Duration::from_millis(0),
            )]
        } else {
            Vec::new()
        };
        RateLimitOutcome::Allowed { audit_events }
    }

    fn candidates(
        &self,
        request: &SurfaceClientRateLimitRequest,
        surface_tightened: bool,
    ) -> Vec<RateLimitCandidate> {
        let surface_budget = self.config.surface_client.budget(request.request_class);
        let surface_budget = if surface_tightened {
            surface_budget.tightened()
        } else {
            surface_budget
        };
        let mut candidates = vec![
            RateLimitCandidate::new(
                SurfaceClientRateLimitAxis::SurfaceClient,
                format!(
                    "surface_client:{}:{}",
                    request.request_class.as_str(),
                    request.surface_client_id
                ),
                surface_budget,
            ),
            RateLimitCandidate::new(
                SurfaceClientRateLimitAxis::WpSite,
                format!(
                    "wp_site:{}:{}",
                    request.request_class.as_str(),
                    request.site_binding_digest
                ),
                self.config.wp_site.budget(request.request_class),
            ),
        ];

        if let Some(wp_user_hash) = request.wp_user_hash.as_ref() {
            candidates.push(RateLimitCandidate::new(
                SurfaceClientRateLimitAxis::WpUser,
                format!("wp_user:{}:{wp_user_hash}", request.request_class.as_str()),
                self.config.wp_user.budget(request.request_class),
            ));
        }

        if request.charge_ability_scope {
            for (index, scope_class) in request.scope_classes.iter().enumerate() {
                if request.wp_user_hash.is_none() {
                    continue;
                }
                let scope_hash = request
                    .required_scope_hashes
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| "hmac-sha256:unknown".to_string());
                candidates.push(RateLimitCandidate::new(
                    SurfaceClientRateLimitAxis::Scope,
                    format!("scope:{}:{scope_hash}", scope_class.as_str()),
                    self.config.scope.budget(*scope_class),
                ));
            }
            candidates.push(RateLimitCandidate::new(
                SurfaceClientRateLimitAxis::Ability,
                format!(
                    "ability:{}:{}",
                    request.ability_class.as_str(),
                    request.ability_id_hash
                ),
                self.config
                    .ability
                    .budget(request.ability_class)
                    .lower_only(request.policy_rate_limit),
            ));
        }

        candidates
    }
}

#[derive(Default)]
struct RateLimiterState {
    buckets: HashMap<RateLimitBucketKey, RateLimitBucket>,
    retry_after_until: HashMap<String, Instant>,
    tightened_until: HashMap<String, Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RateLimitBucketKey {
    axis: SurfaceClientRateLimitAxis,
    key: String,
}

struct RateLimitCandidate {
    axis: SurfaceClientRateLimitAxis,
    key: RateLimitBucketKey,
    budget: SurfaceClientRateLimitBudget,
}

impl RateLimitCandidate {
    fn new(
        axis: SurfaceClientRateLimitAxis,
        key: String,
        budget: SurfaceClientRateLimitBudget,
    ) -> Self {
        Self {
            axis,
            key: RateLimitBucketKey { axis, key },
            budget,
        }
    }
}

#[derive(Clone, Debug)]
struct RateLimitBucket {
    budget: SurfaceClientRateLimitBudget,
    tokens: f64,
    last_refill: Instant,
}

impl RateLimitBucket {
    fn new(budget: SurfaceClientRateLimitBudget, now: Instant) -> Self {
        Self {
            tokens: f64::from(budget.capacity()),
            budget,
            last_refill: now,
        }
    }

    fn reconfigure(&mut self, budget: SurfaceClientRateLimitBudget, now: Instant) {
        self.refill(now);
        self.budget = budget;
        self.tokens = self.tokens.min(f64::from(budget.capacity()));
    }

    fn retry_after(&mut self, now: Instant) -> Option<Duration> {
        self.refill(now);
        if self.tokens >= 1.0 {
            return None;
        }
        let refill_per_second = self.budget.refill_per_second().max(f64::EPSILON);
        Some(Duration::from_secs_f64(
            (1.0 - self.tokens) / refill_per_second,
        ))
    }

    fn consume_one(&mut self) {
        self.tokens = (self.tokens - 1.0).max(0.0);
    }

    fn refill(&mut self, now: Instant) {
        if now <= self.last_refill {
            return;
        }
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let next_tokens = self.tokens + elapsed * self.budget.refill_per_second();
        self.tokens = next_tokens.min(f64::from(self.budget.capacity()));
        self.last_refill = now;
    }
}

fn choose_public_exhausted_axis(
    exhausted: &[(SurfaceClientRateLimitAxis, Duration)],
) -> (SurfaceClientRateLimitAxis, Duration) {
    let axis = exhausted
        .iter()
        .map(|(axis, _)| *axis)
        .min_by_key(|axis| axis.precedence())
        .expect("exhausted list is non-empty");
    let retry_after = exhausted
        .iter()
        .filter_map(|(candidate_axis, retry_after)| {
            (*candidate_axis == axis).then_some(*retry_after)
        })
        .max()
        .unwrap_or_else(|| Duration::from_secs(1));
    (axis, retry_after)
}

fn identity_rate_limit_request(
    session: &ValidatedSurfaceSession,
    ability_name: &str,
    request_id: &str,
    request_class: SurfaceClientRequestClass,
    ability_class: SurfaceClientAbilityClass,
    audit_hash_secret: &str,
) -> SurfaceClientRateLimitRequest {
    SurfaceClientRateLimitRequest {
        surface_client_id: session.surface_client_id.clone(),
        wp_user_hash: session.wp_user_hash.clone(),
        wp_site_id: session.wp_site_id.clone(),
        wp_site_id_hash: session.wp_site_id_hash.clone(),
        site_binding_digest: session.site_binding_digest.clone(),
        ability_id_hash: privacy_hash("surface-ability", audit_hash_secret, ability_name),
        ability_name: ability_name.to_string(),
        request_class,
        ability_class,
        scope_classes: Vec::new(),
        required_scope_hashes: Vec::new(),
        request_id: request_id.to_string(),
        policy_rate_limit: None,
        wp_user_id: session.wp_user_id,
        actor_scopes: session.granted_scopes.clone(),
        charge_ability_scope: false,
    }
}

fn ensure_required_scopes(
    session: &ValidatedSurfaceSession,
    descriptor: &AbilityDescriptor,
) -> Result<(), SurfaceClientBridgeError> {
    let granted = session
        .granted_scopes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let missing = descriptor
        .policy
        .required_scopes
        .iter()
        .any(|scope| !granted.contains(scope));
    if missing {
        return Err(SurfaceClientBridgeError::ScopeDenied);
    }
    if let Actor::SurfaceClient { scopes, .. } = &session.actor {
        for scope in descriptor.policy.required_scopes {
            if !scopes.contains(&SurfaceScope::new(*scope)) {
                return Err(SurfaceClientBridgeError::ScopeDenied);
            }
        }
        return Ok(());
    }
    Err(SurfaceClientBridgeError::ScopeDenied)
}

fn request_class_for_descriptor(descriptor: &AbilityDescriptor) -> SurfaceClientRequestClass {
    if descriptor.category == AbilityCategory::Maintenance || has_admin_scope(descriptor) {
        SurfaceClientRequestClass::Admin
    } else if descriptor.category == AbilityCategory::Publish
        || descriptor.policy.required_scopes.iter().any(|scope| {
            matches!(
                scope_class_for_scope(scope),
                SurfaceClientRequestClass::Write
            )
        })
    {
        SurfaceClientRequestClass::Write
    } else {
        SurfaceClientRequestClass::Read
    }
}

fn ability_class_for_descriptor(descriptor: &AbilityDescriptor) -> SurfaceClientAbilityClass {
    if descriptor.category == AbilityCategory::Maintenance || has_admin_scope(descriptor) {
        SurfaceClientAbilityClass::AdminAbility
    } else if descriptor.category == AbilityCategory::Publish
        || descriptor
            .policy
            .required_scopes
            .contains(&"submit.feedback")
    {
        SurfaceClientAbilityClass::FeedbackWrite
    } else if descriptor.category == AbilityCategory::Transform {
        SurfaceClientAbilityClass::HeavyTransform
    } else if descriptor.category == AbilityCategory::Read
        && descriptor.composes.is_empty()
        && descriptor.policy.required_scopes.len() <= 1
    {
        SurfaceClientAbilityClass::CheapRead
    } else {
        SurfaceClientAbilityClass::StandardReadComposition
    }
}

fn has_admin_scope(descriptor: &AbilityDescriptor) -> bool {
    descriptor.policy.required_scopes.iter().any(|scope| {
        matches!(
            scope_class_for_scope(scope),
            SurfaceClientRequestClass::Admin
        )
    })
}

fn scope_class_for_scope(scope: &str) -> SurfaceClientRequestClass {
    if scope.starts_with("admin.")
        || scope.starts_with("manage.")
        || scope.contains(".admin")
        || scope.contains(".manage.")
        || scope.ends_with(".manage")
    {
        SurfaceClientRequestClass::Admin
    } else if scope.starts_with("write.") || scope.starts_with("submit.") {
        SurfaceClientRequestClass::Write
    } else {
        SurfaceClientRequestClass::Read
    }
}

fn privacy_hash(namespace: &str, secret: &str, value: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let mut context = hmac::Context::with_key(&key);
    context.update(namespace.as_bytes());
    context.update(&[0]);
    context.update(value.as_bytes());
    let digest = context.sign();
    format!("hmac-sha256:{}", hex::encode(&digest.as_ref()[..16]))
}

fn rate_limit_audit_event(
    request: &SurfaceClientRateLimitRequest,
    decision: &'static str,
    axis: Option<SurfaceClientRateLimitAxis>,
    retry_after: Duration,
) -> SurfacePairingAuditEvent {
    let mut detail = json!({
        "decision": decision,
        "surface_client_id": request.surface_client_id,
        "site_binding_digest": request.site_binding_digest,
        "wp_site_id": request.wp_site_id,
        "wp_site_id_hash": request.wp_site_id_hash,
        "ability_name": request.ability_name,
        "ability_id_hash": request.ability_id_hash,
        "required_scopes_hashes": request.required_scope_hashes,
        "request_class": request.request_class.as_str(),
        "ability_class": request.ability_class.as_str(),
        "request_id": request.request_id,
        "retry_after_ms": duration_ms(retry_after),
    });
    if let Some(axis) = axis {
        detail["exhausted_axis"] = json!(axis.as_str());
        detail["axis_exhausted"] = json!(axis.as_str());
    }
    let scope_class = request
        .scope_classes
        .iter()
        .map(|class| class.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    detail["scope_class"] = json!(scope_class);

    SurfacePairingAuditEvent {
        event_kind: "surface_client.rate_limit",
        category: "security",
        actor: Actor::SurfaceClient {
            instance: SurfaceClientId::new(request.surface_client_id.clone()),
            scopes: ScopeSet::new(
                request
                    .actor_scopes
                    .iter()
                    .map(|scope| SurfaceScope::new(scope.clone())),
            )
            .expect("SurfaceClient rate-limit audit requires a non-empty granted scope set"),
        },
        wp_user_id: request.wp_user_id,
        wp_user_hash: request.wp_user_hash.clone(),
        detail,
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{
        AbilityContext, AbilityPolicy, ErasedAbilityFuture, McpExposure, ScopeSet, SignalPolicy,
        SurfaceClientId,
    };
    use crate::abilities::ActorKind;
    use serde_json::json;
    use std::sync::Barrier;
    use std::thread;

    #[derive(Clone)]
    struct FixedClock {
        now: Arc<Mutex<Instant>>,
    }

    impl FixedClock {
        fn new(now: Instant) -> Self {
            Self {
                now: Arc::new(Mutex::new(now)),
            }
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock();
            *now += duration;
        }
    }

    impl SurfaceClientMonotonicClock for FixedClock {
        fn now(&self) -> Instant {
            *self.now.lock()
        }
    }

    fn ok_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedAbilityFuture<'a> {
        Box::pin(async {
            Ok(json!({"data": {}, "provenance": {"invocation_id": "inv_test"}, "diagnostics": {}}))
        })
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": true
        })
    }

    fn descriptor(
        name: &'static str,
        category: AbilityCategory,
        scopes: &'static [&'static str],
        rate_limit: Option<AbilityRateLimit>,
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: &[ActorKind::SurfaceClient],
                allowed_modes: &[ExecutionMode::Live],
                requires_confirmation: false,
                may_publish: false,
                required_scopes: scopes,
                mcp_exposure: McpExposure::None,
                client_side_executable: true,
                rate_limit,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased: ok_erased,
            input_schema: schema,
            output_schema: schema,
        }
    }

    fn session(scopes: &[&str]) -> ValidatedSurfaceSession {
        let scope_strings = scopes
            .iter()
            .map(|scope| (*scope).to_string())
            .collect::<Vec<_>>();
        let scope_set = ScopeSet::new(scopes.iter().map(|scope| SurfaceScope::new(*scope)))
            .expect("test scopes are non-empty");
        ValidatedSurfaceSession {
            surface_client_id: "sc_test".to_string(),
            session_id: "sess_test".to_string(),
            actor: Actor::SurfaceClient {
                instance: SurfaceClientId::new("sc_test"),
                scopes: scope_set,
            },
            wp_user_id: Some(42),
            wp_user_hash: Some("wp_user_hash_test".to_string()),
            wp_site_id: "wp_site_test".to_string(),
            wp_site_id_hash: "wp_site_hash_test".to_string(),
            site_binding_digest: "site_digest_test".to_string(),
            site_nonce: "site_nonce_test".to_string(),
            scope_digest: "scope_digest_test".to_string(),
            granted_scopes: scope_strings,
        }
    }

    fn test_config() -> SurfaceClientBridgeConfig {
        let generous = SurfaceClientRateLimitBudget::new(60_000, 10_000);
        SurfaceClientBridgeConfig {
            surface_client: SurfaceClientRequestClassLimits {
                read: generous,
                write: generous,
                admin: generous,
            },
            wp_user: SurfaceClientRequestClassLimits {
                read: generous,
                write: generous,
                admin: generous,
            },
            wp_site: SurfaceClientRequestClassLimits {
                read: generous,
                write: generous,
                admin: generous,
            },
            ability: SurfaceClientAbilityClassLimits {
                cheap_read: generous,
                standard_read_composition: generous,
                heavy_transform: generous,
                feedback_write: generous,
                admin_ability: generous,
            },
            scope: SurfaceClientRequestClassLimits {
                read: generous,
                write: generous,
                admin: generous,
            },
            early_retry_tighten_window: Duration::from_secs(300),
        }
    }

    fn bridge_with(config: SurfaceClientBridgeConfig, clock: FixedClock) -> SurfaceClientBridge {
        SurfaceClientBridge::with_clock(config, Arc::new(clock))
    }

    #[test]
    fn simultaneous_exhaustion_uses_fixed_public_axis_precedence() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.surface_client.read = SurfaceClientRateLimitBudget::new(60, 1);
        config.wp_site.read = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = bridge_with(config, clock.clone());
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_read",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
            ]);
        let session = session(&["read.account_overview"]);

        assert!(bridge
            .authorize(&registry, &session, "surface_read", "req_1")
            .is_ok());
        let err = bridge
            .authorize(&registry, &session, "surface_read", "req_2")
            .unwrap_err();

        match err {
            SurfaceClientBridgeError::RateLimited(rejection) => {
                assert_eq!(rejection.axis, SurfaceClientRateLimitAxis::SurfaceClient);
                assert_eq!(
                    rejection.audit_event.detail["axis_exhausted"],
                    json!("surface_client")
                );
            }
            other => panic!("expected rate-limit rejection, got {other:?}"),
        }
    }

    #[test]
    fn each_rate_limit_axis_exhausts_independently() {
        for expected_axis in [
            SurfaceClientRateLimitAxis::SurfaceClient,
            SurfaceClientRateLimitAxis::WpSite,
            SurfaceClientRateLimitAxis::WpUser,
            SurfaceClientRateLimitAxis::Scope,
            SurfaceClientRateLimitAxis::Ability,
        ] {
            let clock = FixedClock::new(Instant::now());
            let mut config = test_config();
            let one_per_second = SurfaceClientRateLimitBudget::new(60, 1);
            match expected_axis {
                SurfaceClientRateLimitAxis::SurfaceClient => {
                    config.surface_client.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::WpSite => {
                    config.wp_site.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::WpUser => {
                    config.wp_user.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::Scope => {
                    config.scope.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::Ability => {
                    config.ability.cheap_read = one_per_second;
                    config.ability.standard_read_composition = one_per_second;
                }
            }
            let bridge = bridge_with(config, clock);
            let registry =
                AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                    descriptor(
                        "surface_read",
                        AbilityCategory::Read,
                        &["read.account_overview"],
                        None,
                    ),
                ]);
            let session = session(&["read.account_overview"]);

            assert!(bridge
                .authorize(&registry, &session, "surface_read", "req_1")
                .is_ok());
            let err = bridge
                .authorize(&registry, &session, "surface_read", "req_2")
                .unwrap_err();
            assert!(matches!(
                err,
                SurfaceClientBridgeError::RateLimited(rejection) if rejection.axis == expected_axis
            ));
        }
    }

    #[test]
    fn denied_scope_request_does_not_consume_unexhausted_ability_bucket() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.scope.read = SurfaceClientRateLimitBudget::new(60, 1);
        config.ability.cheap_read = SurfaceClientRateLimitBudget::new(1, 1);
        config.ability.standard_read_composition = SurfaceClientRateLimitBudget::new(1, 1);
        let bridge = bridge_with(config, clock.clone());
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_a",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
                descriptor(
                    "surface_b",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
            ]);
        let session = session(&["read.account_overview"]);

        assert!(bridge
            .authorize(&registry, &session, "surface_a", "req_1")
            .is_ok());
        let rejected = bridge
            .authorize(&registry, &session, "surface_b", "req_2")
            .unwrap_err();
        assert!(matches!(
            rejected,
            SurfaceClientBridgeError::RateLimited(rejection)
                if rejection.axis == SurfaceClientRateLimitAxis::Scope
        ));

        clock.advance(Duration::from_secs(1));
        assert!(bridge
            .authorize(&registry, &session, "surface_b", "req_3")
            .is_ok());
    }

    #[test]
    fn missing_wp_user_context_skips_user_and_scope_axes() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.wp_user.read = SurfaceClientRateLimitBudget::new(60, 1);
        config.scope.read = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = bridge_with(config, clock);
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_read",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
            ]);
        let mut session = session(&["read.account_overview"]);
        session.wp_user_id = None;
        session.wp_user_hash = None;

        assert!(bridge
            .authorize(&registry, &session, "surface_read", "req_1")
            .is_ok());
        assert!(bridge
            .authorize(&registry, &session, "surface_read", "req_2")
            .is_ok());
    }

    #[test]
    fn manage_scopes_use_admin_scope_budget() {
        assert_eq!(
            scope_class_for_scope("manage.pairing"),
            SurfaceClientRequestClass::Admin
        );
        assert_eq!(
            scope_class_for_scope("manage.scopes"),
            SurfaceClientRequestClass::Admin
        );
        assert_eq!(
            scope_class_for_scope("manage.site_registration"),
            SurfaceClientRequestClass::Admin
        );

        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.scope.admin = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = bridge_with(config, clock);
        let request = |request_id: &str| SurfaceClientRateLimitRequest {
            surface_client_id: "sc_admin_test".to_string(),
            wp_user_hash: Some("wp_hash_admin_test".to_string()),
            wp_site_id: "site_admin_test".to_string(),
            wp_site_id_hash: "wp_site_hash_admin_test".to_string(),
            site_binding_digest: "site_digest_admin_test".to_string(),
            ability_id_hash: privacy_hash("surface-ability", "audit_secret", "surface_admin"),
            ability_name: "surface_admin".to_string(),
            request_class: SurfaceClientRequestClass::Read,
            ability_class: SurfaceClientAbilityClass::CheapRead,
            scope_classes: vec![scope_class_for_scope("manage.pairing")],
            required_scope_hashes: vec![privacy_hash(
                "surface-scope",
                "audit_secret",
                "manage.pairing",
            )],
            request_id: request_id.to_string(),
            policy_rate_limit: None,
            wp_user_id: Some(42),
            actor_scopes: vec!["read.account_overview".to_string()],
            charge_ability_scope: true,
        };

        assert!(matches!(
            bridge.limiter.check_and_consume(request("req_1")),
            RateLimitOutcome::Allowed { .. }
        ));
        assert!(matches!(
            bridge.limiter.check_and_consume(request("req_2")),
            RateLimitOutcome::Rejected(SurfaceClientRateLimitRejection {
                axis: SurfaceClientRateLimitAxis::Scope,
                ..
            })
        ));
    }

    #[test]
    fn policy_rejected_invocations_consume_identity_axes() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.surface_client.read = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = bridge_with(config, clock);
        let mut disabled = descriptor(
            "surface_disabled",
            AbilityCategory::Read,
            &["read.account_overview"],
            None,
        );
        disabled.policy.client_side_executable = false;
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                disabled,
            ]);
        let session = session(&["read.account_overview"]);

        assert!(matches!(
            bridge
                .authorize(&registry, &session, "surface_disabled", "req_1")
                .unwrap_err(),
            SurfaceClientBridgeError::AbilityUnavailable
        ));
        assert!(matches!(
            bridge
                .authorize(&registry, &session, "surface_disabled", "req_2")
                .unwrap_err(),
            SurfaceClientBridgeError::RateLimited(rejection)
                if rejection.axis == SurfaceClientRateLimitAxis::SurfaceClient
        ));
    }

    #[test]
    fn ability_policy_rate_limit_is_lower_only() {
        let base = SurfaceClientRateLimitBudget::new(60, 1);
        assert_eq!(
            base.lower_only(Some(AbilityRateLimit {
                requests_per_minute: 600,
                burst_per_second: 100,
            })),
            base
        );
        assert_eq!(
            base.lower_only(Some(AbilityRateLimit {
                requests_per_minute: 6,
                burst_per_second: 1,
            })),
            SurfaceClientRateLimitBudget::new(6, 1)
        );
    }

    #[test]
    fn policy_override_can_exhaust_ability_axis_without_lowering_other_axes() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.ability.standard_read_composition = SurfaceClientRateLimitBudget::new(60_000, 10);
        let bridge = bridge_with(config, clock.clone());
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_read",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    Some(AbilityRateLimit {
                        requests_per_minute: 60,
                        burst_per_second: 1,
                    }),
                ),
            ]);
        let session = session(&["read.account_overview"]);

        assert!(bridge
            .authorize(&registry, &session, "surface_read", "req_1")
            .is_ok());
        let err = bridge
            .authorize(&registry, &session, "surface_read", "req_2")
            .unwrap_err();
        assert!(matches!(
            err,
            SurfaceClientBridgeError::RateLimited(rejection)
                if rejection.axis == SurfaceClientRateLimitAxis::Ability
        ));
    }

    #[test]
    fn early_retry_emits_audit_event_and_tightens_surface_bucket() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.surface_client.read = SurfaceClientRateLimitBudget::new(60, 2);
        config.ability.cheap_read = SurfaceClientRateLimitBudget::new(60, 1);
        config.ability.standard_read_composition = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = bridge_with(config, clock.clone());
        let registry =
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_a",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
                descriptor(
                    "surface_b",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
                descriptor(
                    "surface_c",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
            ]);
        let session = session(&["read.account_overview"]);

        assert!(bridge
            .authorize(&registry, &session, "surface_a", "req_1")
            .is_ok());
        assert!(matches!(
            bridge
                .authorize(&registry, &session, "surface_a", "req_2")
                .unwrap_err(),
            SurfaceClientBridgeError::RateLimited(_)
        ));

        let allowed = bridge
            .authorize(&registry, &session, "surface_b", "req_3")
            .expect("surface bucket still has one token before early-retry tightening");
        assert_eq!(allowed.audit_events.len(), 1);
        assert_eq!(
            allowed.audit_events[0].detail["decision"],
            json!("early_retry")
        );

        assert!(matches!(
            bridge
                .authorize(&registry, &session, "surface_c", "req_4")
                .unwrap_err(),
            SurfaceClientBridgeError::RateLimited(rejection)
                if rejection.axis == SurfaceClientRateLimitAxis::SurfaceClient
        ));
    }

    #[test]
    fn concurrent_requests_do_not_overdraw_bucket() {
        let clock = FixedClock::new(Instant::now());
        let mut config = test_config();
        config.surface_client.read = SurfaceClientRateLimitBudget::new(60, 1);
        let bridge = Arc::new(bridge_with(config, clock));
        let registry = Arc::new(
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                descriptor(
                    "surface_read",
                    AbilityCategory::Read,
                    &["read.account_overview"],
                    None,
                ),
            ]),
        );
        let session = Arc::new(session(&["read.account_overview"]));
        let barrier = Arc::new(Barrier::new(8));
        let handles = (0..8)
            .map(|index| {
                let bridge = Arc::clone(&bridge);
                let registry = Arc::clone(&registry);
                let session = Arc::clone(&session);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    bridge
                        .authorize(&registry, &session, "surface_read", &format!("req_{index}"))
                        .is_ok()
                })
            })
            .collect::<Vec<_>>();

        let allowed = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread joins"))
            .filter(|allowed| *allowed)
            .count();
        assert_eq!(allowed, 1);
    }

    #[test]
    fn rate_limit_audit_detail_is_privacy_safe() {
        let request = SurfaceClientRateLimitRequest {
            surface_client_id: "sc_test".to_string(),
            wp_user_hash: Some("wp_hash_test".to_string()),
            wp_site_id: "site_1".to_string(),
            wp_site_id_hash: "wp_site_hash_test".to_string(),
            site_binding_digest: "site_digest_test".to_string(),
            ability_id_hash: privacy_hash("surface-ability", "audit_secret", "raw.ability"),
            ability_name: "raw.ability".to_string(),
            request_class: SurfaceClientRequestClass::Read,
            ability_class: SurfaceClientAbilityClass::StandardReadComposition,
            scope_classes: vec![SurfaceClientRequestClass::Read],
            required_scope_hashes: vec![privacy_hash(
                "surface-scope",
                "audit_secret",
                "read.account_overview",
            )],
            request_id: "req_test".to_string(),
            policy_rate_limit: None,
            wp_user_id: Some(42),
            actor_scopes: vec!["read.account_overview".to_string()],
            charge_ability_scope: true,
        };

        let event = rate_limit_audit_event(
            &request,
            "rejected",
            Some(SurfaceClientRateLimitAxis::Ability),
            Duration::from_millis(250),
        );
        let detail = event.detail.to_string();
        assert!(!detail.contains("read.account_overview"));
        assert!(!detail.contains("\"wp_user_id\""));
        assert!(detail.contains("raw.ability"));
        assert!(detail.contains("site_1"));
        assert!(detail.contains("ability_id_hash"));
        assert!(detail.contains("ability_name"));
        assert!(detail.contains("required_scopes_hashes"));
        assert!(detail.contains("wp_site_id"));
        assert!(detail.contains("wp_site_id_hash"));
        assert!(detail.contains("exhausted_axis"));
    }
}
