use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::panic;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use bytes::Bytes;
use chrono::Utc;
use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use http::{Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use parking_lot::{Mutex, RwLock};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use uuid::Uuid;

use crate::abilities::NOOP_ABILITY_TRACER;
use crate::bridges::correction_payload::{
    project_claim_for_scope, project_composition_for_scope, CorrectionPayload,
};
use crate::bridges::surface_client::{
    is_surface_nonce_route, validate_session_bound_wp_user_id_for_request,
    SurfaceClientAbilityClassLimits, SurfaceClientBridge, SurfaceClientBridgeConfig,
    SurfaceClientBridgeError, SurfaceClientRateLimitAxis, SurfaceClientRateLimitBudget,
    SurfaceClientRequestClassLimits,
};
use crate::bridges::types::{
    invoke_registry_json_for_actor, provider_from_context_snapshot, surface_error,
    AbilityResponseJson, BridgeActor, BridgeSurface, RequestScopedInvocation,
};
use crate::bridges::BridgeSurfaceError;
use crate::services::context::ClaimDismissalSurface;
use crate::services::surface_nonce::{SurfaceNonceError, SurfaceNonceService};
use crate::services::surface_pairing::{
    self, PairingCodeFailureInput, PairingHandshakeCapacityInput, PairingHandshakeInput,
    PairingHandshakeRequest, SignedSessionValidationInput, SignedSiteClaimsInput,
    SignedTransportFailureInput, SurfacePairingAuditEvent, SurfacePairingError,
    SurfaceSessionRefreshIdentity, SurfaceSessionRefreshInput, ValidatedSurfaceSession,
};
use crate::state::AppState;
use abilities_runtime::abilities::registry::{Actor, ScopeSet, SurfaceClientId};

mod hmac;

pub const SURFACE_ENDPOINT_VERSION: &str = "v1";
const DEFAULT_MAX_BIND_ATTEMPTS: u16 = 10;
const DEFAULT_LOOPBACK_REQUESTS_PER_MINUTE: u32 = 60;
const DEFAULT_LOOPBACK_BURST_PER_SECOND: u32 = 10;
const DEFAULT_PAIRING_CODE_FAILED_ATTEMPTS: u32 = 5;
const MAX_HANDSHAKE_BODY_BYTES: usize = 4 * 1024;
const MAX_SESSION_REFRESH_BODY_BYTES: usize = 1024;
const DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES: usize = 256 * 1024;

type ResponseBody = Full<Bytes>;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceEndpointConfig {
    pub max_bind_attempts: u16,
    pub loopback_abuse: TokenBucketConfig,
    pub pairing_attempts: PairingAttemptConfig,
    pub(crate) signed_transport: hmac::SignedTransportConfig,
    pub(crate) signed_request_max_body_bytes: usize,
    pub(crate) surface_client_bridge: SurfaceClientBridgeConfig,
}

impl Default for SurfaceEndpointConfig {
    fn default() -> Self {
        Self {
            max_bind_attempts: DEFAULT_MAX_BIND_ATTEMPTS,
            loopback_abuse: TokenBucketConfig {
                capacity: DEFAULT_LOOPBACK_BURST_PER_SECOND,
                refill_per_second: f64::from(DEFAULT_LOOPBACK_REQUESTS_PER_MINUTE) / 60.0,
            },
            pairing_attempts: PairingAttemptConfig {
                max_failed_attempts_per_code: DEFAULT_PAIRING_CODE_FAILED_ATTEMPTS,
            },
            signed_transport: hmac::SignedTransportConfig::default(),
            signed_request_max_body_bytes: DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES,
            surface_client_bridge: SurfaceClientBridgeConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TokenBucketConfig {
    pub capacity: u32,
    pub refill_per_second: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairingAttemptConfig {
    pub max_failed_attempts_per_code: u32,
}

impl Default for PairingAttemptConfig {
    fn default() -> Self {
        Self {
            max_failed_attempts_per_code: DEFAULT_PAIRING_CODE_FAILED_ATTEMPTS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceEndpointAvailability {
    Unavailable,
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceEndpointSnapshot {
    pub availability: SurfaceEndpointAvailability,
    pub bound_port: Option<u16>,
    pub startup_id: Option<Uuid>,
    pub endpoint_version: &'static str,
    pub can_cancel: bool,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceEndpointPairingStatus {
    pub availability: SurfaceEndpointAvailability,
    pub bound_port: Option<u16>,
    pub endpoint_version: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePairingContext {
    pub startup_id: Uuid,
    pub bound_port: u16,
    pub runtime_anchor_id: String,
}

#[derive(Debug)]
pub struct SurfaceEndpointStartError {
    message: String,
}

impl std::fmt::Display for SurfaceEndpointStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SurfaceEndpointStartError {}

#[derive(Default)]
pub struct SurfaceEndpointState {
    inner: Mutex<EndpointInner>,
    paired_site_origins: Arc<RwLock<HashSet<String>>>,
    pairing_attempts: Arc<Mutex<PairingAttemptLimiter>>,
    signed_transport: hmac::SignedTransportState,
}

#[derive(Default)]
struct EndpointInner {
    availability: Option<SurfaceEndpointAvailability>,
    running: Option<RunningEndpoint>,
    last_error: Option<String>,
}

struct RunningEndpoint {
    startup_id: Uuid,
    bound_port: u16,
    runtime_anchor_id: String,
    shutdown: watch::Sender<bool>,
    abort: AbortHandle,
}

impl SurfaceEndpointState {
    pub fn snapshot(&self) -> SurfaceEndpointSnapshot {
        let inner = self.inner.lock();
        let running = inner.running.as_ref();
        SurfaceEndpointSnapshot {
            availability: inner
                .availability
                .unwrap_or(SurfaceEndpointAvailability::Unavailable),
            bound_port: running.map(|endpoint| endpoint.bound_port),
            startup_id: running.map(|endpoint| endpoint.startup_id),
            endpoint_version: SURFACE_ENDPOINT_VERSION,
            can_cancel: running.is_some(),
            last_error: inner.last_error.clone(),
        }
    }

    pub fn pairing_status(&self) -> SurfaceEndpointPairingStatus {
        let inner = self.inner.lock();
        SurfaceEndpointPairingStatus {
            availability: inner
                .availability
                .unwrap_or(SurfaceEndpointAvailability::Unavailable),
            bound_port: inner.running.as_ref().map(|endpoint| endpoint.bound_port),
            endpoint_version: SURFACE_ENDPOINT_VERSION,
        }
    }

    pub fn runtime_pairing_context(&self) -> Result<RuntimePairingContext, String> {
        let inner = self.inner.lock();
        let running = inner
            .running
            .as_ref()
            .ok_or_else(|| "Surface runtime endpoint is not running.".to_string())?;
        Ok(RuntimePairingContext {
            startup_id: running.startup_id,
            bound_port: running.bound_port,
            runtime_anchor_id: running.runtime_anchor_id.clone(),
        })
    }

    #[cfg(test)]
    pub async fn start(
        self: Arc<Self>,
        config: SurfaceEndpointConfig,
    ) -> Result<SurfaceEndpointSnapshot, SurfaceEndpointStartError> {
        let (snapshot, _listener) = self.start_listener(config, None).await?;
        Ok(snapshot)
    }

    async fn start_listener(
        self: Arc<Self>,
        config: SurfaceEndpointConfig,
        app_state: Option<Arc<AppState>>,
    ) -> Result<(SurfaceEndpointSnapshot, JoinHandle<()>), SurfaceEndpointStartError> {
        self.stop();
        {
            let mut inner = self.inner.lock();
            inner.availability = Some(SurfaceEndpointAvailability::Starting);
            inner.last_error = None;
        }
        {
            let mut attempts = self.pairing_attempts.lock();
            attempts.config = config.pairing_attempts;
            attempts.attempts_by_code.clear();
        }
        self.signed_transport
            .configure(config.signed_transport.clone());
        self.paired_site_origins.write().clear();
        let surface_nonce = SurfaceNonceService::new_from_w2b_secret(
            self.signed_transport.presence_nonce_secret_material(),
        )
        .map_err(|error| {
            let message = format!("surface endpoint presence nonce key unavailable: {error}");
            self.mark_failed(message.clone());
            SurfaceEndpointStartError::new(message)
        })?;
        let runtime_anchor_id = if app_state.is_some() {
            crate::db::key_provider::get_or_create_surface_runtime_anchor_id().map_err(|error| {
                let message = format!("surface endpoint runtime anchor unavailable: {error}");
                self.mark_failed(message.clone());
                SurfaceEndpointStartError::new(message)
            })?
        } else {
            "test_runtime_anchor".to_string()
        };

        let max_attempts = config.max_bind_attempts.clamp(1, DEFAULT_MAX_BIND_ATTEMPTS);
        let mut last_error = None;
        for _attempt in 1..=max_attempts {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await {
                Ok(listener) => {
                    let local_addr = listener.local_addr().map_err(|error| {
                        SurfaceEndpointStartError::new(format!(
                            "surface endpoint bound but local address lookup failed: {error}"
                        ))
                    })?;
                    let SocketAddr::V4(addr) = local_addr else {
                        let message = format!(
                            "surface endpoint refused non-IPv4 listener address {local_addr}"
                        );
                        self.mark_failed(message.clone());
                        return Err(SurfaceEndpointStartError::new(message));
                    };
                    let bound_port = addr.port();
                    let startup_id = Uuid::new_v4();
                    let (shutdown, shutdown_rx) = watch::channel(false);
                    let runtime = Arc::new(EndpointRuntime {
                        startup_id,
                        bound_port,
                        runtime_anchor_id: runtime_anchor_id.clone(),
                        loopback_bucket: Mutex::new(TokenBucket::new(config.loopback_abuse)),
                        pairing_attempts: Arc::clone(&self.pairing_attempts),
                        paired_site_origins: Arc::clone(&self.paired_site_origins),
                        signed_transport: self.signed_transport.clone(),
                        signed_request_max_body_bytes: config.signed_request_max_body_bytes,
                        surface_client_bridge: SurfaceClientBridge::new(
                            config.surface_client_bridge.clone(),
                        ),
                        surface_nonce: surface_nonce.clone(),
                        #[cfg(test)]
                        ability_registry_override: None,
                        app_state: app_state.clone(),
                    });
                    let endpoint_state = Arc::clone(&self);
                    let join = tokio::spawn(async move {
                        run_listener(listener, runtime, shutdown_rx).await;
                        endpoint_state.mark_stopped_if_current(startup_id);
                    });
                    let abort = join.abort_handle();

                    {
                        let mut inner = self.inner.lock();
                        inner.availability = Some(SurfaceEndpointAvailability::Running);
                        inner.last_error = None;
                        inner.running = Some(RunningEndpoint {
                            startup_id,
                            bound_port,
                            runtime_anchor_id: runtime_anchor_id.clone(),
                            shutdown,
                            abort,
                        });
                    }
                    return Ok((self.snapshot(), join));
                }
                Err(error) => {
                    last_error = Some(format!("surface endpoint bind failed: {error}"));
                }
            }
        }

        let message = last_error.unwrap_or_else(|| {
            "surface endpoint bind failed before a concrete socket error was reported".to_string()
        });
        self.mark_failed(message.clone());
        Err(SurfaceEndpointStartError::new(message))
    }

    pub async fn run_until_stopped(
        self: Arc<Self>,
        config: SurfaceEndpointConfig,
        app_state: Arc<AppState>,
    ) -> Result<(), SurfaceEndpointStartError> {
        let (snapshot, listener) = self.clone().start_listener(config, Some(app_state)).await?;
        let startup_id = snapshot.startup_id.unwrap_or_else(Uuid::new_v4);
        match listener.await {
            Ok(()) => {
                self.mark_stopped_if_current(startup_id);
                Ok(())
            }
            Err(error) if error.is_panic() => {
                self.mark_failed("surface endpoint listener stopped unexpectedly".to_string());
                panic::resume_unwind(error.into_panic());
            }
            Err(error) => {
                log::debug!("surface endpoint listener task stopped: {error}");
                self.mark_stopped_if_current(startup_id);
                Ok(())
            }
        }
    }

    pub fn stop(&self) {
        self.clear_pairing_state();
        let running = {
            let mut inner = self.inner.lock();
            let running = inner.running.take();
            if running.is_some() {
                inner.availability = Some(SurfaceEndpointAvailability::Stopped);
            }
            running
        };

        if let Some(endpoint) = running {
            if endpoint.shutdown.send(true).is_err() {
                log::debug!("surface endpoint shutdown signal had no active listener");
            }
            endpoint.abort.abort();
        }
    }

    pub fn set_paired_site_url_for_origin_guard(&self, site_url: Option<&str>) {
        let mut origins = self.paired_site_origins.write();
        origins.clear();
        if let Some(origin) = site_url.and_then(normalize_origin) {
            origins.insert(origin);
        }
    }

    pub fn forget_surface_client_sessions(&self, surface_client_id: &str) {
        self.signed_transport
            .remove_sessions_for_surface_client(surface_client_id);
    }

    fn clear_pairing_state(&self) {
        self.pairing_attempts.lock().attempts_by_code.clear();
        self.paired_site_origins.write().clear();
        self.signed_transport.clear_runtime_state();
    }

    fn mark_failed(&self, message: String) {
        let mut inner = self.inner.lock();
        inner.availability = Some(SurfaceEndpointAvailability::Failed);
        inner.running = None;
        inner.last_error = Some(message);
    }

    fn mark_stopped_if_current(&self, startup_id: Uuid) {
        let mut inner = self.inner.lock();
        if inner
            .running
            .as_ref()
            .is_some_and(|running| running.startup_id == startup_id)
        {
            inner.running = None;
            inner.availability = Some(SurfaceEndpointAvailability::Stopped);
        }
    }
}

impl SurfaceEndpointStartError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl Drop for SurfaceEndpointState {
    fn drop(&mut self) {
        let running = self.inner.get_mut().running.take();
        if let Some(endpoint) = running {
            if endpoint.shutdown.send(true).is_err() {
                log::debug!("surface endpoint drop found no active listener");
            }
            endpoint.abort.abort();
        }
    }
}

pub async fn run_supervised_http_endpoint(state: Arc<AppState>) {
    let config = state
        .config_read_or_recover()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .map(|config| SurfaceEndpointConfig::from(&config.surface_runtime))
        })
        .unwrap_or_default();

    if let Err(error) = state
        .surface_runtime_endpoint
        .clone()
        .run_until_stopped(config, state.clone())
        .await
    {
        log::warn!("Surface runtime endpoint unavailable: {error}");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

impl From<&crate::types::SurfaceRuntimeConfig> for SurfaceEndpointConfig {
    fn from(config: &crate::types::SurfaceRuntimeConfig) -> Self {
        Self {
            max_bind_attempts: config.max_bind_attempts.clamp(1, DEFAULT_MAX_BIND_ATTEMPTS),
            loopback_abuse: TokenBucketConfig {
                capacity: config.unauthenticated_loopback_burst_per_second.max(1),
                refill_per_second: f64::from(
                    config.unauthenticated_loopback_requests_per_minute.max(1),
                ) / 60.0,
            },
            pairing_attempts: PairingAttemptConfig {
                max_failed_attempts_per_code: config.pairing_code_max_failed_attempts,
            },
            signed_transport: hmac::SignedTransportConfig {
                parseable_session_bucket: hmac::SignedTokenBucketConfig {
                    capacity: config.signed_session_burst_per_second.max(1),
                    refill_per_second: f64::from(config.signed_session_requests_per_minute.max(1))
                        / 60.0,
                },
                stale_window: Duration::from_secs(config.signature_stale_window_seconds.max(1)),
                future_skew: Duration::from_secs(config.signature_future_skew_seconds),
                cleanup_slack: Duration::from_secs(config.signature_nonce_cleanup_slack_seconds),
                pending_nonce_ttl: Duration::from_secs(
                    config.signature_nonce_pending_ttl_seconds.max(1),
                ),
                nonce_records_per_session: config.signature_nonce_records_per_session.max(1),
                max_active_sessions: config.signature_max_active_sessions.max(1),
                global_nonce_records: config.signature_global_nonce_records.max(1),
            },
            signed_request_max_body_bytes: usize::try_from(
                config.signed_request_max_body_bytes.max(1),
            )
            .unwrap_or(DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES),
            surface_client_bridge: SurfaceClientBridgeConfig::from(
                &config.surface_client_rate_limits,
            ),
        }
    }
}

impl From<&crate::types::SurfaceClientRateLimitConfig> for SurfaceClientBridgeConfig {
    fn from(config: &crate::types::SurfaceClientRateLimitConfig) -> Self {
        Self {
            surface_client: SurfaceClientRequestClassLimits::from(&config.surface_client),
            wp_user: SurfaceClientRequestClassLimits::from(&config.wp_user),
            wp_site: SurfaceClientRequestClassLimits::from(&config.wp_site),
            ability: SurfaceClientAbilityClassLimits {
                cheap_read: SurfaceClientRateLimitBudget::from(&config.ability.cheap_read),
                standard_read_composition: SurfaceClientRateLimitBudget::from(
                    &config.ability.standard_read_composition,
                ),
                heavy_transform: SurfaceClientRateLimitBudget::from(
                    &config.ability.heavy_transform,
                ),
                feedback_write: SurfaceClientRateLimitBudget::from(&config.ability.feedback_write),
                admin_ability: SurfaceClientRateLimitBudget::from(&config.ability.admin_ability),
            },
            scope: SurfaceClientRequestClassLimits::from(&config.scope),
            early_retry_tighten_window: Duration::from_secs(
                config.early_retry_tighten_window_seconds.max(1),
            ),
        }
    }
}

impl From<&crate::types::SurfaceClientRequestRateLimitConfig> for SurfaceClientRequestClassLimits {
    fn from(config: &crate::types::SurfaceClientRequestRateLimitConfig) -> Self {
        Self {
            read: SurfaceClientRateLimitBudget::from(&config.read),
            write: SurfaceClientRateLimitBudget::from(&config.write),
            admin: SurfaceClientRateLimitBudget::from(&config.admin),
        }
    }
}

impl From<&crate::types::SurfaceClientRateLimitBudgetConfig> for SurfaceClientRateLimitBudget {
    fn from(config: &crate::types::SurfaceClientRateLimitBudgetConfig) -> Self {
        Self {
            requests_per_minute: config.requests_per_minute.max(1),
            burst_per_second: config.burst_per_second.max(1),
        }
    }
}

struct EndpointRuntime {
    startup_id: Uuid,
    bound_port: u16,
    runtime_anchor_id: String,
    loopback_bucket: Mutex<TokenBucket>,
    pairing_attempts: Arc<Mutex<PairingAttemptLimiter>>,
    paired_site_origins: Arc<RwLock<HashSet<String>>>,
    signed_transport: hmac::SignedTransportState,
    signed_request_max_body_bytes: usize,
    surface_client_bridge: SurfaceClientBridge,
    surface_nonce: SurfaceNonceService,
    #[cfg(test)]
    ability_registry_override: Option<Arc<crate::abilities::AbilityRegistry>>,
    app_state: Option<Arc<AppState>>,
}

async fn run_listener(
    listener: TcpListener,
    runtime: Arc<EndpointRuntime>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut connection_tasks = JoinSet::new();
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, _peer_addr)) => {
                        let runtime = Arc::clone(&runtime);
                        connection_tasks.spawn(async move {
                            let io = TokioIo::new(stream);
                            let service = service_fn(move |request: Request<Incoming>| {
                                let runtime = Arc::clone(&runtime);
                                async move { handle_hyper_request(request, runtime).await }
                            });
                            if let Err(error) = http1::Builder::new().serve_connection(io, service).await {
                                log::debug!("surface endpoint connection ended with error: {error}");
                            }
                        });
                    }
                    Err(error) => {
                        log::warn!("surface endpoint listener accept failed: {error}");
                        break;
                    }
                }
            }
            completed = connection_tasks.join_next(), if !connection_tasks.is_empty() => {
                if let Some(Err(error)) = completed {
                    log::debug!("surface endpoint connection task ended with error: {error}");
                }
            }
        }
    }

    connection_tasks.abort_all();
    while connection_tasks.join_next().await.is_some() {}
}

async fn handle_hyper_request(
    request: Request<Incoming>,
    runtime: Arc<EndpointRuntime>,
) -> Result<Response<ResponseBody>, Infallible> {
    let request_id = request_id_from_headers(request.headers());
    let transport_check = {
        let origins = runtime.paired_site_origins.read();
        validate_transport_headers(request.headers(), runtime.bound_port, &origins)
    };
    if let Err(error) = transport_check {
        return Ok(error_response(error.with_request_id(request_id)));
    }

    let rate_decision = runtime.loopback_bucket.lock().try_acquire(Instant::now());
    if let Err(retry_after) = rate_decision {
        return Ok(error_response(
            SurfaceHttpError::loopback_rate_limited(retry_after).with_request_id(request_id),
        ));
    }

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body_limit = if method == Method::POST && uri.path() == "/v1/pairing/handshake" {
        Some(MAX_HANDSHAKE_BODY_BYTES)
    } else if method == Method::POST && uri.path() == "/v1/surface/session/refresh" {
        Some(MAX_SESSION_REFRESH_BODY_BYTES)
    } else if is_signed_route_candidate(uri.path()) {
        Some(runtime.signed_request_max_body_bytes)
    } else {
        None
    };
    let body = if let Some(max_body_bytes) = body_limit {
        match collect_limited_body(request.into_body(), max_body_bytes).await {
            Ok(body) => body,
            Err(error) => {
                return Ok(error_response(error.with_request_id(request_id)));
            }
        }
    } else {
        Bytes::new()
    };

    Ok(dispatch_surface_request(
        SurfaceHttpRequest {
            method,
            uri,
            headers,
            body,
        },
        runtime,
        request_id,
    )
    .await)
}

async fn collect_limited_body<B>(body: B, max_bytes: usize) -> Result<Bytes, SurfaceHttpError>
where
    B: hyper::body::Body<Data = Bytes>,
    B::Error: Into<Box<dyn Error + Send + Sync>>,
{
    match Limited::new(body, max_bytes).collect().await {
        Ok(collected) => Ok(collected.to_bytes()),
        Err(error) if error.downcast_ref::<LengthLimitError>().is_some() => {
            Err(SurfaceHttpError::payload_too_large())
        }
        Err(_error) => Err(SurfaceHttpError::bad_request("request_body_unreadable")
            .with_safe_message("The request body could not be read safely.")),
    }
}

#[derive(Clone)]
struct SurfaceHttpRequest {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
}

async fn dispatch_surface_request(
    request: SurfaceHttpRequest,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
) -> Response<ResponseBody> {
    let path = request.uri.path().to_string();
    match (&request.method, path.as_str()) {
        (&Method::GET, "/v1/surface/health") => health_response(request_id),
        (&Method::POST, "/v1/pairing/handshake") => {
            pairing_handshake_response(request.body, runtime, request_id).await
        }
        (&Method::POST, "/v1/surface/session/refresh") => {
            surface_session_refresh_response(request.body, runtime, request_id).await
        }
        _ if is_signed_route_candidate(path.as_str()) => {
            let route_supported = is_supported_signed_route(&request.method, path.as_str());
            signed_transport_response(request, runtime, request_id, route_supported).await
        }
        _ => error_response(SurfaceHttpError::route_not_found().with_request_id(request_id)),
    }
}

async fn signed_transport_response(
    request: SurfaceHttpRequest,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
    route_supported: bool,
) -> Response<ResponseBody> {
    let verified = match runtime.signed_transport.verify(hmac::SignedRequest {
        headers: &request.headers,
        method: &request.method,
        uri: &request.uri,
        body: &request.body,
        now: Utc::now(),
        instant: Instant::now(),
    }) {
        Ok(verified) => verified,
        Err(error) => {
            log_signing_failure(&request, &request_id, &error);
            record_signed_transport_failure(&runtime, &request, &error).await;
            return error_response(
                SurfaceHttpError::from_signed_transport(error).with_request_id(request_id),
            );
        }
    };

    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };

    let validation_input = SignedSessionValidationInput {
        session_id: verified.session_id.clone(),
        surface_client_id: verified.surface_client_id.clone(),
        runtime_anchor_id: runtime.runtime_anchor_id.clone(),
        site_claims: SignedSiteClaimsInput {
            wp_site_id: verified.wp_site_id.clone(),
            home_url: verified.home_url.clone(),
            site_url: verified.site_url.clone(),
            wp_install_uuid: verified.wp_install_uuid.clone(),
            plugin_instance_uuid: verified.plugin_instance_uuid.clone(),
            multisite_blog_id: verified.multisite_blog_id.clone(),
        },
        site_nonce: verified.site_nonce.clone(),
        wp_user_id: verified.wp_user_id,
        wp_user_hash: verified.wp_user_hash.clone(),
        now: Utc::now(),
    };
    let audit_session_id = verified.session_id.clone();
    let audit_surface_client_id = verified.surface_client_id.clone();
    let validated = match app_state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            let result = surface_pairing::validate_signed_session(&ctx, db, validation_input);
            // On error, recover scopes from the pairing record for audit attribution.
            // Best-effort: returns None if the row is already gone or scopes_json
            // is corrupted, in which case the audit emission falls back to
            // `Actor::System`.
            let scopes_for_audit = match &result {
                Err(_) => surface_pairing::load_session_scope_set_for_audit(
                    db,
                    &audit_session_id,
                    &audit_surface_client_id,
                ),
                Ok(_) => None,
            };
            Ok((result, scopes_for_audit))
        })
        .await
    {
        Ok((Ok(validated), _)) => validated,
        Ok((Err(error), scopes_for_audit)) => {
            for event in validation_rejection_events(&verified, &error, scopes_for_audit.as_ref()) {
                emit_pairing_audit_event(&app_state, &event);
            }
            evict_cached_session_after_validation_error(
                &runtime.signed_transport,
                &verified.surface_client_id,
                &error,
            );
            return error_response(
                SurfaceHttpError::from_pairing_error(error).with_request_id(request_id),
            );
        }
        Err(error) => {
            log::warn!(
                target: "dailyos_lib::surface_runtime",
                "validate_signed_session db_write failed (request_id={request_id}): {error}"
            );
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };

    if !route_supported {
        return error_response(SurfaceHttpError::route_not_found().with_request_id(request_id));
    }

    signed_route_response(&request, &runtime, validated, request_id).await
}

fn is_supported_signed_route(method: &Method, path: &str) -> bool {
    if method == Method::GET && path.starts_with("/v1/surface/event-log/") {
        return true;
    }
    if is_surface_nonce_route(method, path) {
        return true;
    }
    matches!(
        (method, path),
        (&Method::GET, "/v1/pairing/status")
            | (&Method::POST, "/v1/surface/invoke")
            | (&Method::POST, "/v1/surface/feedback")
            | (&Method::GET, "/v1/surface/abilities")
            | (&Method::GET, "/v1/surface/keyring")
            | (&Method::POST, "/v1/surface/subscribe")
            | (&Method::POST, "/v1/surface/replay")
            | (&Method::POST, "/v1/surface/project-composition")
    )
}

fn is_signed_route_candidate(path: &str) -> bool {
    (path.starts_with("/v1/surface/")
        && path != "/v1/surface/health"
        && path != "/v1/surface/session/refresh")
        || (path.starts_with("/v1/pairing/") && path != "/v1/pairing/handshake")
}

fn log_signing_failure(
    request: &SurfaceHttpRequest,
    request_id: &str,
    error: &hmac::SignedTransportError,
) {
    let path_query = request
        .uri
        .path_and_query()
        .map(|path_query| path_query.as_str())
        .unwrap_or_else(|| request.uri.path());
    log::warn!(
        "dailyos.wp_bridge.signing failure code={} request_id={} session_id_hash={} surface_client_hash={} method={} path_hash={} reason={}",
        error.code(),
        request_id,
        error.session_id_hash.as_deref().unwrap_or("absent"),
        error.surface_client_id_hash.as_deref().unwrap_or("absent"),
        request.method,
        hmac::hash_prefix(path_query),
        error.reason
    );
}

async fn pairing_handshake_response(
    body: Bytes,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
) -> Response<ResponseBody> {
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };

    let request = match serde_json::from_slice::<PairingHandshakeRequest>(&body) {
        Ok(request) => request,
        Err(_error) => {
            if let Some(pairing_code) = pairing_code_from_body(&body) {
                let max_failed_attempts = runtime
                    .pairing_attempts
                    .lock()
                    .config
                    .max_failed_attempts_per_code;
                let input = PairingCodeFailureInput {
                    endpoint_startup_id: runtime.startup_id.to_string(),
                    bound_port: runtime.bound_port,
                    pairing_code,
                    max_failed_attempts,
                    now: Utc::now(),
                };
                if let Some(error) = match app_state
                    .db_write(move |db| {
                        let clock = crate::services::context::SystemClock;
                        let rng = crate::services::context::SystemRng;
                        let external = crate::services::context::ExternalClients::default();
                        let ctx = crate::services::context::ServiceContext::new_live(
                            &clock, &rng, &external,
                        );
                        surface_pairing::record_pairing_code_failure_with_audit(&ctx, db, input)
                            .map_err(|error| error.to_string())
                    })
                    .await
                {
                    Ok(outcome) => {
                        emit_pairing_audit_event(&app_state, &outcome.audit);
                        outcome.error
                    }
                    Err(error) => {
                        return error_response(
                            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                                .with_request_id(request_id),
                        );
                    }
                } {
                    if matches!(error, SurfacePairingError::PairingCodeLimited) {
                        return error_response(
                            SurfaceHttpError::from_pairing_error(error).with_request_id(request_id),
                        );
                    }
                }
            }
            return error_response(
                SurfaceHttpError::bad_request("handshake_body_invalid")
                    .with_safe_message("The pairing handshake payload is invalid.")
                    .with_request_id(request_id),
            );
        }
    };

    let max_failed_attempts = runtime
        .pairing_attempts
        .lock()
        .config
        .max_failed_attempts_per_code;
    let failure_audit_input = PairingCodeFailureInput {
        endpoint_startup_id: runtime.startup_id.to_string(),
        bound_port: runtime.bound_port,
        pairing_code: request.pairing_code.clone(),
        max_failed_attempts,
        now: Utc::now(),
    };
    let capacity_input = PairingHandshakeCapacityInput {
        runtime_anchor_id: runtime.runtime_anchor_id.clone(),
        request: request.clone(),
    };
    let replaceable_surface_client_ids = match app_state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            Ok(
                surface_pairing::replaceable_surface_client_ids_for_handshake(
                    &ctx,
                    db,
                    capacity_input,
                ),
            )
        })
        .await
    {
        Ok(Ok(ids)) => Some(ids),
        Ok(Err(SurfacePairingError::BadRequest(_))) => None,
        Ok(Err(error)) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(error).with_request_id(request_id),
            );
        }
        Err(error) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };
    let capacity_reservation =
        if let Some(surface_client_ids) = replaceable_surface_client_ids.as_ref() {
            match runtime
                .signed_transport
                .reserve_session_capacity_after_removing_surface_clients(surface_client_ids)
            {
                Ok(reservation) => Some(reservation),
                Err(error) => {
                    return error_response(
                        SurfaceHttpError::from_signed_transport(error).with_request_id(request_id),
                    );
                }
            }
        } else {
            None
        };
    let input = PairingHandshakeInput {
        runtime_anchor_id: runtime.runtime_anchor_id.clone(),
        endpoint_startup_id: runtime.startup_id.to_string(),
        bound_port: runtime.bound_port,
        endpoint_version: SURFACE_ENDPOINT_VERSION,
        max_failed_attempts,
        now: Utc::now(),
        request,
    };
    let outcome = match app_state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            Ok(surface_pairing::complete_handshake(&ctx, db, input))
        })
        .await
    {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(error)) => {
            let audit = surface_pairing::pairing_code_failure_audit_event(
                &failure_audit_input,
                Some(&error),
            );
            emit_pairing_audit_event(&app_state, &audit);
            return error_response(
                SurfaceHttpError::from_pairing_error(error).with_request_id(request_id),
            );
        }
        Err(error) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };

    let session = hmac::SignedSurfaceSession::new_active(
        outcome.session.session_id.clone(),
        outcome.session.surface_client_id.clone(),
        outcome.session.hmac_master_key,
    );
    let registration_result = match capacity_reservation {
        Some(reservation) => reservation
            .register_after_removing_surface_clients(&outcome.revoked_surface_client_ids, session),
        None => register_session_after_revocations(
            &runtime.signed_transport,
            &outcome.revoked_surface_client_ids,
            session,
        ),
    };
    if let Err(error) = registration_result {
        compensate_failed_session_registration(
            &app_state,
            outcome.session.surface_client_id.clone(),
            Utc::now(),
        )
        .await;
        return error_response(
            SurfaceHttpError::from_signed_transport(error).with_request_id(request_id),
        );
    }

    if let Some(origin) = normalize_origin(&outcome.paired_origin) {
        runtime.paired_site_origins.write().insert(origin);
    }
    if let Some(event) = outcome.revocation_audit.as_ref() {
        emit_pairing_audit_event(&app_state, event);
    }
    emit_pairing_audit_event(&app_state, &outcome.audit);
    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "request_id": request_id,
            "pairing": outcome.response,
        }),
    )
}

async fn surface_session_refresh_response(
    body: Bytes,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
) -> Response<ResponseBody> {
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };

    let request = match serde_json::from_slice::<SurfaceSessionRefreshRequest>(&body) {
        Ok(request) => request,
        Err(_error) => {
            return error_response(
                SurfaceHttpError::bad_request("session_refresh_body_invalid")
                    .with_safe_message("The session refresh payload is invalid.")
                    .with_request_id(request_id),
            );
        }
    };
    let session_id = request.session_id.clone();
    let input = SurfaceSessionRefreshInput {
        session_id: request.session_id,
        site_binding_digest: request.site_binding_digest,
        wp_install_uuid: request.wp_install_uuid,
        plugin_instance_uuid: request.plugin_instance_uuid,
    };

    let identity = match app_state
        .db_read(move |db| {
            surface_pairing::verify_session_refresh_identity(db, input)
                .map_err(|error| error.to_string())
        })
        .await
    {
        Ok(identity) => identity,
        Err(error) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };

    match identity {
        SurfaceSessionRefreshIdentity::Matched => {}
        SurfaceSessionRefreshIdentity::SessionNotFound => {
            return error_response(
                SurfaceHttpError::session_not_found().with_request_id(request_id),
            );
        }
        SurfaceSessionRefreshIdentity::IdentityMismatch => {
            return error_response(
                SurfaceHttpError::identity_mismatch().with_request_id(request_id),
            );
        }
    }

    let Some(hmac_key) = runtime
        .signed_transport
        .derive_active_session_key(&session_id)
    else {
        return error_response(SurfaceHttpError::session_not_found().with_request_id(request_id));
    };

    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "request_id": request_id,
            "session_id": session_id,
            "hmac_key": hex::encode(hmac_key),
        }),
    )
}

async fn record_signed_transport_failure(
    runtime: &EndpointRuntime,
    request: &SurfaceHttpRequest,
    error: &hmac::SignedTransportError,
) {
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return;
    };
    let Some(session_id) = runtime
        .signed_transport
        .active_session_id_from_headers_for_failure(&request.headers)
    else {
        return;
    };
    let surface_client_id = safe_header_value(&request.headers, "x-dailyos-surfaceclient").ok();
    let direct_event =
        signed_transport_failure_event(&session_id, surface_client_id.as_deref(), error.code());
    let presented_surface_client_id = surface_client_id.clone();
    let input = SignedTransportFailureInput {
        session_id,
        surface_client_id,
        failure_code: error.code().to_string(),
        now: Utc::now(),
    };
    let events = match app_state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            Ok(surface_pairing::record_signed_transport_failure(
                &ctx, db, input,
            ))
        })
        .await
    {
        Ok(Ok(events)) => events,
        Ok(Err(error)) => {
            log::warn!(
                "surface signing failure audit unavailable: {}",
                error.code()
            );
            return;
        }
        Err(error) => {
            log::warn!("surface signing failure audit write failed: {error}");
            return;
        }
    };
    if let Some(event) = direct_event {
        emit_pairing_audit_event(&app_state, &event);
    }
    for event in events {
        if event.event_kind == "pairing_revoked" {
            if let Some(surface_client_id) = event
                .detail
                .get("surface_client_id")
                .and_then(serde_json::Value::as_str)
                .or(presented_surface_client_id.as_deref())
            {
                runtime
                    .signed_transport
                    .remove_sessions_for_surface_client(surface_client_id);
            }
        }
        emit_pairing_audit_event(&app_state, &event);
    }
}

async fn compensate_failed_session_registration(
    app_state: &AppState,
    surface_client_id: String,
    now: chrono::DateTime<Utc>,
) {
    let input = surface_pairing::RevokePairingInput {
        surface_client_id,
        reason: "session_registration_failed".to_string(),
        now,
    };
    match app_state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            Ok(surface_pairing::revoke_pairing(&ctx, db, input))
        })
        .await
    {
        Ok(Ok(event)) => emit_pairing_audit_event(app_state, &event),
        Ok(Err(error)) => {
            log::warn!(
                "surface pairing compensation failed after session registration error: {}",
                error.code()
            );
        }
        Err(error) => {
            log::warn!("surface pairing compensation write failed: {error}");
        }
    }
}

#[derive(Debug)]
struct SurfaceVersionEventRow {
    event_seq: i64,
    cursor: String,
    event_kind: String,
    claim_id: Option<String>,
    composition_id: Option<String>,
    previous_version: Option<i64>,
    current_version: i64,
    reason: Option<String>,
    scope_redacted: bool,
    correction_event_log_id: Option<String>,
    mutation_id: Option<String>,
    created_at: String,
    actor_kind: String,
}

impl SurfaceVersionEventRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            event_seq: row.get(0)?,
            cursor: row.get(1)?,
            event_kind: row.get(2)?,
            claim_id: row.get(3)?,
            composition_id: row.get(4)?,
            previous_version: row.get(5)?,
            current_version: row.get(6)?,
            reason: row.get(7)?,
            scope_redacted: row.get::<_, i64>(8)? != 0,
            correction_event_log_id: row.get(9)?,
            mutation_id: row.get(10)?,
            created_at: row.get(11)?,
            actor_kind: row.get(12)?,
        })
    }

    fn full_event(&self) -> Value {
        json!({
            "event_seq": self.event_seq,
            "cursor": &self.cursor,
            "event_kind": &self.event_kind,
            "claim_id": &self.claim_id,
            "composition_id": &self.composition_id,
            "previous_version": self.previous_version,
            "current_version": self.current_version,
            "reason": &self.reason,
            "scope_redacted": self.scope_redacted,
            "correction_event_log_id": &self.correction_event_log_id,
            "mutation_id": &self.mutation_id,
            "created_at": &self.created_at,
            "actor_kind": &self.actor_kind,
        })
    }

    fn redacted_event(&self) -> Value {
        json!({
            "cursor": &self.cursor,
            "created_at": &self.created_at,
            "scope_redacted": true,
        })
    }
}

async fn surface_event_log_response(
    runtime: &EndpointRuntime,
    path: &str,
    request_id: String,
    actor: Actor,
) -> Response<ResponseBody> {
    let event_log_id = path
        .trim_start_matches("/v1/surface/event-log/")
        .trim()
        .to_string();
    if event_log_id.is_empty() || event_log_id.len() > 128 {
        return error_response(
            SurfaceHttpError::bad_request("event_log_id_invalid").with_request_id(request_id),
        );
    }
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    let lookup_id = event_log_id.clone();
    let projection = match app_state
        .db_read(move |db| {
            let row = db
                .conn_ref()
                .query_row(
                    "SELECT event_seq, cursor, event_kind, claim_id, composition_id,
                            previous_version, current_version, reason, scope_redacted,
                            correction_event_log_id, mutation_id, created_at, actor_kind
                     FROM version_events
                     WHERE cursor = ?1 OR correction_event_log_id = ?1
                     ORDER BY event_seq DESC
                     LIMIT 1",
                    rusqlite::params![lookup_id],
                    SurfaceVersionEventRow::from_row,
                )
                .optional()
                .map_err(|error| error.to_string())?;

            let Some(row) = row else {
                return Ok(None);
            };

            // Scope-gate every direct-key fetch per packet §16. The
            // version_events row carries either a claim_id or a
            // composition_id (XOR-enforced by migration 172). Both channels
            // route through the same projection pipeline so that out-of-scope
            // requesters receive a redacted envelope — never raw composition
            // identifiers, version trajectory, mutation_id, or actor_kind.
            //
            // Terminal-event cursor durability (§7): when the target row is
            // missing AND the event is a terminal kind (mutation_aborted,
            // claim.write_rejected), the cursor must still resolve to a
            // redacted envelope. A 423-loser handed this cursor would
            // otherwise hit 404 forever if the holder aborted before
            // creating its `intelligence_claims` row. Non-terminal kinds
            // against a missing row remain 404 (the claim was deleted; the
            // event is non-recoverable).
            let is_terminal_kind = matches!(
                row.event_kind.as_str(),
                "mutation_aborted" | "claim.write_rejected" | "composition.write_rejected"
            );
            let correction = match (row.claim_id.as_deref(), row.composition_id.as_deref()) {
                (Some(claim_id), _) => match project_claim_for_scope(db, claim_id, &actor) {
                    Some(correction) => correction,
                    None if is_terminal_kind => CorrectionPayload::out_of_scope(),
                    None => return Ok(None),
                },
                (None, Some(composition_id)) => {
                    match project_composition_for_scope(db, composition_id, &actor) {
                        Some(correction) => correction,
                        None if is_terminal_kind => CorrectionPayload::out_of_scope(),
                        None => return Ok(None),
                    }
                }
                // Defensive: a row without either pointer is not addressable
                // by a scope projection. Treat as redacted rather than
                // leaking the full event envelope.
                (None, None) => CorrectionPayload::out_of_scope(),
            };
            let event = if correction.scope_redacted {
                row.redacted_event()
            } else {
                row.full_event()
            };
            let mut correction =
                serde_json::to_value(correction).map_err(|error| error.to_string())?;
            correction["cursor"] = json!(row.cursor);
            Ok(Some((event, correction)))
        })
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };
    let Some((event, correction)) = projection else {
        return error_response(SurfaceHttpError::route_not_found().with_request_id(request_id));
    };
    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "request_id": request_id,
            "event": event,
            "correction": correction,
        }),
    )
}

async fn surface_keyring_response(
    runtime: &EndpointRuntime,
    request_id: String,
    validated: &ValidatedSurfaceSession,
) -> Response<ResponseBody> {
    if !surface_client_can_read_keyring(&validated.actor) {
        if let Some(app_state) = runtime.app_state.as_ref() {
            emit_pairing_audit_event(
                app_state,
                &SurfacePairingAuditEvent {
                    event_kind: "projection_keyring_scope_denied",
                    category: "security",
                    actor: validated.actor.clone(),
                    wp_user_id: validated.wp_user_id,
                    wp_user_hash: validated.wp_user_hash.clone(),
                    detail: json!({
                        "surface_client_id": validated.surface_client_id,
                        "decision": "rejected",
                        "reason": "scope_denied"
                    }),
                },
            );
        }
        return error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::ScopeDenied)
                .with_request_id(request_id),
        );
    }

    let Some(app_state) = runtime.app_state.as_ref() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    let runtime_anchor_id = validated.wp_site_id.clone();
    match app_state
        .db_read(move |db| {
            crate::services::projection_signing::public_keyring(db, runtime_anchor_id)
                .map_err(|error| error.to_string())
        })
        .await
    {
        Ok(keyring) => {
            emit_pairing_audit_event(
                app_state,
                &SurfacePairingAuditEvent {
                    event_kind: "projection_keyring_read",
                    category: "data_access",
                    actor: validated.actor.clone(),
                    wp_user_id: validated.wp_user_id,
                    wp_user_hash: validated.wp_user_hash.clone(),
                    detail: json!({
                        "surface_client_id": validated.surface_client_id,
                        "scope_digest": validated.scope_digest,
                        "key_count": keyring.keys.len(),
                    }),
                },
            );
            json_response(
                StatusCode::OK,
                json!({
                    "ok": true,
                    "request_id": request_id,
                    "endpoint_version": SURFACE_ENDPOINT_VERSION,
                    "keyring": keyring,
                }),
            )
        }
        Err(error) => error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                .with_request_id(request_id),
        ),
    }
}

fn surface_client_can_read_keyring(actor: &Actor) -> bool {
    match actor {
        Actor::SurfaceClient { scopes, .. } => scopes
            .iter()
            .any(|scope| scope.as_str().starts_with("read.") || scope.as_str() == "manage.pairing"),
        Actor::Admin | Actor::System => true,
        Actor::Agent | Actor::User => false,
    }
}

async fn signed_route_response(
    request: &SurfaceHttpRequest,
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    request_id: String,
) -> Response<ResponseBody> {
    // Walk body + query string + headers per packet §17 (extended in L2
    // cycle-2 M1 from JSON-body-only). The class shape is that any
    // wp_user_id channel into the bridge must be checked against the
    // session-bound value before further dispatch.
    let body_payload = serde_json::from_slice::<serde_json::Value>(&request.body).ok();
    let validation = validate_session_bound_wp_user_id_for_request(
        &validated,
        body_payload.as_ref(),
        request.uri.query(),
        &request.headers,
    );
    if let Err(rejection) = validation {
        if let Some(app_state) = runtime.app_state.as_ref() {
            emit_pairing_audit_event(
                app_state,
                &SurfacePairingAuditEvent {
                    event_kind: "wrong_user_rejected",
                    category: "security",
                    actor: validated.actor.clone(),
                    wp_user_id: validated.wp_user_id,
                    wp_user_hash: validated.wp_user_hash.clone(),
                    detail: json!({
                        "surface_client_id": rejection.surface_client_id,
                        "session_wp_user_id": rejection.session_wp_user_id,
                        "asserted_wp_user_id": rejection.asserted_wp_user_id,
                        "decision": "rejected",
                        "reason": "wrong_user"
                    }),
                },
            );
        }
        return error_response(SurfaceHttpError::wrong_user().with_request_id(request_id));
    }

    match (request.method.clone(), request.uri.path()) {
        (Method::GET, "/v1/surface/keyring") => {
            surface_keyring_response(runtime, request_id, &validated).await
        }
        (Method::GET, "/v1/pairing/status") => json_response(
            StatusCode::OK,
            json!({
                "ok": true,
                "request_id": request_id,
                "endpoint_version": SURFACE_ENDPOINT_VERSION,
                "surface_client_id": validated.surface_client_id,
                "scope_digest": validated.scope_digest,
                "site_binding_digest": validated.site_binding_digest,
            }),
        ),
        (Method::GET, path) if path.starts_with("/v1/surface/event-log/") => {
            surface_event_log_response(runtime, path, request_id, validated.actor.clone()).await
        }
        (Method::GET, "/v1/surface/abilities") => {
            match surface_pairing::authorized_ability_projection(&validated.granted_scopes) {
                Ok(projection) => json_response(
                    StatusCode::OK,
                    json!({
                        "ok": true,
                        "request_id": request_id,
                        "endpoint_version": SURFACE_ENDPOINT_VERSION,
                        "surface_client_id": validated.surface_client_id,
                        "scope_digest": validated.scope_digest,
                        "ability_projection": projection,
                    }),
                ),
                Err(error) => error_response(
                    SurfaceHttpError::from_pairing_error(error).with_request_id(request_id),
                ),
            }
        }
        (Method::POST, "/v1/surface/nonce/issue") => {
            surface_nonce_issue_response(runtime, validated, request.body.clone(), request_id).await
        }
        (Method::POST, "/v1/surface/nonce/verify") => {
            surface_nonce_verify_response(runtime, validated, request.body.clone(), request_id)
                .await
        }
        (Method::POST, "/v1/surface/subscribe") => {
            surface_subscribe_response(runtime, validated, request.body.clone(), request_id).await
        }
        (Method::POST, "/v1/surface/replay") => {
            surface_replay_response(runtime, validated, request.body.clone(), request_id).await
        }
        (Method::POST, "/v1/surface/project-composition") => {
            surface_project_composition_response(
                runtime,
                validated.clone(),
                request.body.clone(),
                request_id,
            )
            .await
        }
        (Method::POST, "/v1/surface/invoke") => {
            let invoke = match serde_json::from_slice::<SurfaceInvokeRequest>(&request.body) {
                Ok(invoke) if is_safe_ability_name(&invoke.ability) => invoke,
                Ok(_) | Err(_) => {
                    return error_response(
                        SurfaceHttpError::bad_request("surface_invoke_invalid")
                            .with_request_id(request_id),
                    );
                }
            };
            if let Some(projection) = invoke.projection_verification.clone() {
                match surface_projection_preflight_response(
                    runtime,
                    &validated,
                    projection,
                    request_id.clone(),
                )
                .await
                {
                    Ok(Some(response)) => return response,
                    Ok(None) => {}
                    Err(error) => return error_response(error.with_request_id(request_id)),
                }
            }
            #[cfg(test)]
            let registry_override = runtime.ability_registry_override.clone();
            #[cfg(test)]
            let registry = if let Some(registry) = registry_override.as_deref() {
                registry
            } else {
                match crate::abilities::AbilityRegistry::global_checked() {
                    Ok(registry) => registry,
                    Err(_) => {
                        return error_response(
                            SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
                        );
                    }
                }
            };
            #[cfg(not(test))]
            let registry = match crate::abilities::AbilityRegistry::global_checked() {
                Ok(registry) => registry,
                Err(_) => {
                    return error_response(
                        SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
                    );
                }
            };
            match runtime.surface_client_bridge.authorize(
                registry,
                &validated,
                &invoke.ability,
                &request_id,
            ) {
                Ok(authorization) => {
                    if let Some(app_state) = runtime.app_state.as_ref() {
                        for event in authorization.audit_events {
                            emit_pairing_audit_event(app_state, &event);
                        }
                    }
                    let Some(app_state) = runtime.app_state.as_ref() else {
                        return error_response(
                            SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
                        );
                    };
                    let snapshot = app_state.context_snapshot();
                    let provider = provider_from_context_snapshot(&snapshot);
                    let services = app_state
                        .live_service_context()
                        .with_actor("surface_client");
                    let audit_input = invoke.input.clone();
                    match invoke_registry_json_for_actor(
                        registry,
                        &services,
                        provider,
                        &NOOP_ABILITY_TRACER,
                        RequestScopedInvocation {
                            registry_actor: validated.actor.clone(),
                            response_actor: BridgeActor::SurfaceClient,
                            surface: BridgeSurface::SurfaceClient,
                            claim_dismissal_surface: ClaimDismissalSurface::LogStructured,
                        },
                        &authorization.canonical_ability_name,
                        invoke.input,
                    )
                    .await
                    {
                        Ok(ability) => {
                            let audit = successful_surface_invocation_audit_event(
                                &validated,
                                &request_id,
                                &audit_input,
                                &ability,
                            );
                            emit_pairing_audit_event(app_state, &audit);
                            json_response(
                                StatusCode::OK,
                                json!({
                                    "ok": true,
                                    "request_id": request_id,
                                    "ability": ability,
                                }),
                            )
                        }
                        Err(error) => {
                            bridge_surface_error_response(
                                surface_error(error),
                                app_state,
                                &validated,
                                request_id,
                            )
                            .await
                        }
                    }
                }
                Err(SurfaceClientBridgeError::RateLimited(rejection)) => {
                    let rejection = *rejection;
                    if let Some(app_state) = runtime.app_state.as_ref() {
                        emit_pairing_audit_event(app_state, &rejection.audit_event);
                    }
                    error_response(
                        SurfaceHttpError::rate_limited(rejection.axis, rejection.retry_after)
                            .with_request_id(request_id),
                    )
                }
                Err(error) => {
                    error_response(surface_bridge_error(error).with_request_id(request_id))
                }
            }
        }
        _ => error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id)),
    }
}

async fn surface_subscribe_response(
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    body: Bytes,
    request_id: String,
) -> Response<ResponseBody> {
    use crate::services::version_dispatcher::SubscribeRequest;
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    let request: SubscribeRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => {
            return error_response(
                SurfaceHttpError::bad_request("subscribe_invalid").with_request_id(request_id),
            );
        }
    };
    let actor = validated.actor.clone();
    let dispatcher = app_state.version_dispatcher.clone();
    let result = app_state
        .db_write(move |db| {
            dispatcher
                .subscribe_stateless(db, &request, actor)
                .map_err(|e| e.to_string())
        })
        .await;
    match result {
        Ok(ack) => json_response(StatusCode::OK, serde_json::to_value(&ack).unwrap_or(json!({}))),
        Err(error) => error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                .with_request_id(request_id),
        ),
    }
}

async fn surface_replay_response(
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    body: Bytes,
    request_id: String,
) -> Response<ResponseBody> {
    use crate::services::version_dispatcher::{ReplayRequest, SubjectFilter};
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    #[derive(serde::Deserialize)]
    struct WireReplay {
        #[serde(flatten)]
        replay: ReplayRequest,
        #[serde(default)]
        subjects: SubjectFilter,
    }
    let wire: WireReplay = match serde_json::from_slice(&body) {
        Ok(w) => w,
        Err(_) => {
            return error_response(
                SurfaceHttpError::bad_request("replay_invalid").with_request_id(request_id),
            );
        }
    };
    let actor = validated.actor.clone();
    let dispatcher = app_state.version_dispatcher.clone();
    let result = app_state
        .db_write(move |db| {
            dispatcher
                .replay_stateless(db, &wire.replay, &actor, &wire.subjects)
                .map_err(|e| e.to_string())
        })
        .await;
    match result {
        Ok(response) => {
            json_response(StatusCode::OK, serde_json::to_value(&response).unwrap_or(json!({})))
        }
        Err(error) => error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                .with_request_id(request_id),
        ),
    }
}

async fn surface_nonce_issue_response(
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    body: Bytes,
    request_id: String,
) -> Response<ResponseBody> {
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    let payload = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    let service = runtime.surface_nonce.clone();
    let request_id_for_work = request_id.clone();
    let session_for_work = validated;
    let result = app_state
        .db_read(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("surface_client");
            Ok(service.issue_nonce(&ctx, db, &session_for_work, payload, &request_id_for_work))
        })
        .await;

    match result {
        Ok(Ok(issue)) => {
            emit_surface_nonce_audit_events(&app_state, &issue.audit_events);
            json_response(
                StatusCode::OK,
                json!({
                    "ok": true,
                    "presence_nonce": issue.presence_nonce,
                    "expires_at": issue.expires_at,
                    "ttl_seconds": issue.ttl_seconds,
                    "request_id": issue.request_id,
                }),
            )
        }
        Ok(Err(error)) => {
            emit_surface_nonce_audit_events(&app_state, &error.audit_events);
            surface_nonce_error_response(error)
        }
        Err(error) => error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                .with_request_id(request_id),
        ),
    }
}

#[derive(serde::Deserialize)]
struct SurfaceProjectCompositionRequest {
    composition_id: String,
    composition_version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_hint_token: Option<String>,
}

async fn surface_project_composition_response(
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    body: Bytes,
    request_id: String,
) -> Response<ResponseBody> {
    use crate::services::composition_render_orchestrator::{
        extract_account_id_from_composition_id, project_from_ability_data,
        resolve_producer_ability_name, FALLBACK_POLICY_VERSION,
    };

    let request: SurfaceProjectCompositionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => {
            return error_response(
                SurfaceHttpError::bad_request("project_composition_invalid")
                    .with_request_id(request_id),
            );
        }
    };

    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };

    let actor = validated.actor.clone();
    let orchestrator = app_state.composition_render_orchestrator.clone();

    // Authorize FIRST — cache hits must not bypass ability authorization,
    // rate limiting, or audit emission. The bridge gates allowed_actors,
    // required_scopes, and per-instance/per-class budgets. Skipping this
    // step on cache hits would let an unauthorized SurfaceClient serve
    // projections cached by an authorized peer with the same scope set.
    let Some(ability_name) = resolve_producer_ability_name(&request.composition_id) else {
        return error_response(
            SurfaceHttpError::bad_request("project_composition_unknown_producer")
                .with_request_id(request_id),
        );
    };
    let Some(account_id) = extract_account_id_from_composition_id(&request.composition_id) else {
        return error_response(
            SurfaceHttpError::bad_request("project_composition_invalid_id")
                .with_request_id(request_id),
        );
    };

    let registry = match crate::abilities::AbilityRegistry::global_checked() {
        Ok(registry) => registry,
        Err(_) => {
            return error_response(
                SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
            );
        }
    };

    let authorization = match runtime.surface_client_bridge.authorize(
        registry,
        &validated,
        ability_name,
        &request_id,
    ) {
        Ok(authorization) => {
            for event in authorization.audit_events.clone() {
                emit_pairing_audit_event(&app_state, &event);
            }
            authorization
        }
        Err(SurfaceClientBridgeError::RateLimited(rejection)) => {
            let rejection = *rejection;
            emit_pairing_audit_event(&app_state, &rejection.audit_event);
            return error_response(
                SurfaceHttpError::rate_limited(rejection.axis, rejection.retry_after)
                    .with_request_id(request_id),
            );
        }
        Err(error) => {
            return error_response(surface_bridge_error(error).with_request_id(request_id));
        }
    };

    // After authorization succeeds, check the cache. Cache key includes the
    // scopes-canonical id, so a different actor with different scopes will
    // miss naturally; authorization above gates the allowed_actors/required_
    // scopes/rate-limit policy that the cache key alone doesn't enforce.
    if let Some(cached) = orchestrator.cache_lookup(
        &actor,
        &request.composition_id,
        request.composition_version,
    ) {
        let projection_json = match serde_json::to_value(&cached.projection) {
            Ok(value) => value,
            Err(_) => {
                return error_response(
                    SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
                );
            }
        };
        return json_response(
            StatusCode::OK,
            json!({
                "ok": true,
                "request_id": request_id,
                "projection": projection_json,
                "cache_hint_token": cached.cache_hint_token,
                "served_from_cache": true,
            }),
        );
    }

    let snapshot = app_state.context_snapshot();
    let provider = provider_from_context_snapshot(&snapshot);
    let services = app_state
        .live_service_context()
        .with_actor("surface_client");

    let input = json!({
        "account_id": account_id,
        "composition_id": request.composition_id,
        "schema_version": 1,
    });
    let invoke_outcome = invoke_registry_json_for_actor(
        registry,
        &services,
        provider,
        &NOOP_ABILITY_TRACER,
        RequestScopedInvocation {
            registry_actor: validated.actor.clone(),
            response_actor: BridgeActor::SurfaceClient,
            surface: BridgeSurface::SurfaceClient,
            claim_dismissal_surface: ClaimDismissalSurface::LogStructured,
        },
        &authorization.canonical_ability_name,
        input,
    )
    .await;

    let response_json = match invoke_outcome {
        Ok(response_json) => response_json,
        Err(error) => {
            return bridge_surface_error_response(
                surface_error(error),
                &app_state,
                &validated,
                request_id,
            )
            .await;
        }
    };

    let (projection, _audits) = match project_from_ability_data(
        &response_json.data,
        validated.actor.clone(),
        FALLBACK_POLICY_VERSION,
    ) {
        Ok(tuple) => tuple,
        Err(_) => {
            return error_response(
                SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
            );
        }
    };

    // Audit intents from the projector are operator-side diagnostics. The
    // substrate audit-logger drain is intentionally not wired in this
    // commit; see the v1.4.2 wave maintenance backlog for the
    // audit-emission interlock.
    let cache_hint_token = orchestrator
        .cache_store(
            &validated.actor,
            &request.composition_id,
            request.composition_version,
            projection.clone(),
        )
        .unwrap_or_default();
    let projection_json = match serde_json::to_value(&projection) {
        Ok(value) => value,
        Err(_) => {
            return error_response(
                SurfaceHttpError::runtime_unavailable().with_request_id(request_id),
            );
        }
    };
    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "request_id": request_id,
            "projection": projection_json,
            "cache_hint_token": cache_hint_token,
            "served_from_cache": false,
        }),
    )
}

async fn surface_nonce_verify_response(
    runtime: &EndpointRuntime,
    validated: ValidatedSurfaceSession,
    body: Bytes,
    request_id: String,
) -> Response<ResponseBody> {
    let Some(app_state) = runtime.app_state.as_ref().cloned() else {
        return error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id));
    };
    let payload = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    let service = runtime.surface_nonce.clone();
    let request_id_for_work = request_id.clone();
    let session_for_work = validated;
    let result = app_state
        .db_read(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("surface_client");
            Ok(service.verify_nonce(&ctx, db, &session_for_work, payload, &request_id_for_work))
        })
        .await;

    match result {
        Ok(Ok(verify)) => {
            emit_surface_nonce_audit_events(&app_state, &verify.audit_events);
            json_response(
                StatusCode::OK,
                json!({
                    "ok": true,
                    "verified": true,
                    "consumed_at": verify.consumed_at,
                    "request_id": verify.request_id,
                    "expected": {
                        "claim_version": verify.expected_claim_version,
                        "composition_version": verify.expected_composition_version,
                    },
                }),
            )
        }
        Ok(Err(error)) => {
            emit_surface_nonce_audit_events(&app_state, &error.audit_events);
            surface_nonce_error_response(error)
        }
        Err(error) => error_response(
            SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                .with_request_id(request_id),
        ),
    }
}

fn emit_surface_nonce_audit_events(
    app_state: &AppState,
    events: &[crate::services::surface_nonce::SurfaceNonceAuditEvent],
) {
    let mut audit = app_state.audit_log.lock();
    if let Err(error) = SurfaceNonceService::emit_audit_events(&mut audit, events) {
        log::warn!("surface nonce audit write failed: {error}");
    }
}

fn surface_nonce_error_response(error: SurfaceNonceError) -> Response<ResponseBody> {
    let retry_after_ms = error
        .retry_after
        .map(|retry_after| retry_after.as_millis().try_into().unwrap_or(u64::MAX));
    let mut body = json!({
        "ok": false,
        "error": "presence_nonce_rejected",
        "reason": error.reason.as_str(),
        "message": "Refresh this block and try again.",
        "request_id": error.request_id,
    });
    if let Some(retry_after_ms) = retry_after_ms {
        body["retry_after_ms"] = json!(retry_after_ms);
    }
    let mut response = json_response(error.status, body);
    if let Some(retry_after_ms) = retry_after_ms {
        let retry_after_seconds = retry_after_ms.div_ceil(1_000).max(1);
        let header_value = HeaderValue::from_str(&retry_after_seconds.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("1"));
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header_value);
    }
    response
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
struct SurfaceSessionRefreshRequest {
    session_id: String,
    site_binding_digest: String,
    wp_install_uuid: String,
    plugin_instance_uuid: String,
}

#[derive(Clone, Deserialize)]
struct SurfaceProjectionVerificationRequest {
    projection_id: String,
    surface: crate::services::projection_signing::ProjectionSurface,
    surface_locator: String,
    #[serde(default)]
    expected_runtime_anchor_id: Option<String>,
    payload: crate::services::projection_signing::SignedProjectionPayload,
    #[serde(default)]
    signature_envelope_b64url: Option<String>,
    #[serde(default)]
    observed_payload_b64url: Option<String>,
}

#[derive(Deserialize)]
struct SurfaceInvokeRequest {
    ability: String,
    #[allow(dead_code)]
    #[serde(default)]
    input: serde_json::Value,
    #[serde(default, alias = "projectionRead", alias = "projection_read")]
    projection_verification: Option<SurfaceProjectionVerificationRequest>,
}

async fn surface_projection_preflight_response(
    runtime: &EndpointRuntime,
    session: &ValidatedSurfaceSession,
    projection: SurfaceProjectionVerificationRequest,
    request_id: String,
) -> Result<Option<Response<ResponseBody>>, SurfaceHttpError> {
    let Some(app_state) = runtime.app_state.as_ref() else {
        return Err(SurfaceHttpError::runtime_unavailable());
    };
    let observed_payload_bytes = match projection.observed_payload_b64url.as_deref() {
        Some(value) => Some(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(value.as_bytes())
                .map_err(|_| {
                    SurfaceHttpError::bad_request("projection_observed_payload_invalid")
                })?,
        ),
        None => None,
    };
    let expected_runtime_anchor_id = projection
        .expected_runtime_anchor_id
        .clone()
        .or_else(|| Some(runtime.runtime_anchor_id.clone()));
    let input = crate::services::projection_signing::ProjectionVerificationInput {
        projection_id: projection.projection_id,
        surface: projection.surface,
        surface_locator: projection.surface_locator,
        expected_runtime_anchor_id,
        payload: projection.payload,
        signature_envelope_b64url: projection.signature_envelope_b64url,
        observed_payload_bytes,
    };
    let verification = app_state
        .db_write(move |db| {
            let mode =
                crate::services::projection_signing::projection_signature_enforcement_mode(db)
                    .map_err(|error| error.to_string())?;
            if mode == crate::services::context::ProjectionSignatureEnforcementMode::Disabled {
                return Ok((mode, None));
            }
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("surface_client");
            let outcome = crate::services::projection_signing::verify_with_unknown_key_refresh(
                &ctx,
                db,
                input,
                || Ok(()),
            )
            .map_err(|error| error.to_string())?;
            Ok((mode, Some(outcome)))
        })
        .await
        .map_err(|error| SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error)))?;

    let (mode, Some(outcome)) = verification else {
        return Ok(None);
    };
    if outcome.failure.is_none() {
        return Ok(None);
    }
    if mode == crate::services::context::ProjectionSignatureEnforcementMode::Shadow {
        log::warn!(
            "surface projection verification shadow failure status={} projection_id={} request_id={}",
            outcome.status.as_str(),
            outcome.projection_id,
            request_id
        );
        return Ok(None);
    }

    let Some(failure) = outcome.failure else {
        return Ok(None);
    };
    let error = bridge_error_from_projection_failure(failure);
    Ok(Some(
        bridge_surface_error_response(error, app_state, session, request_id).await,
    ))
}

fn bridge_error_from_projection_failure(
    failure: crate::services::projection_signing::ProjectionVerificationFailure,
) -> BridgeSurfaceError {
    match failure {
        crate::services::projection_signing::ProjectionVerificationFailure::Tampered(fields) => {
            BridgeSurfaceError::ProjectionTampered {
                projection_id: fields.projection_id,
                signature_id: fields.signature_id,
                key_id: fields.key_id,
                observed_signature_status: fields.observed_signature_status,
                quarantine_id: fields.quarantine_id,
            }
        }
        crate::services::projection_signing::ProjectionVerificationFailure::VersionRollback(
            fields,
        ) => BridgeSurfaceError::ProjectionVersionRollback {
            projection_id: fields.projection_id,
            signed_composition_version: fields.signed_composition_version,
            ledger_composition_version: fields.ledger_composition_version,
            signed_claim_version: fields.signed_claim_version,
            ledger_claim_version: fields.ledger_claim_version,
        },
    }
}

fn is_safe_ability_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/'))
}

async fn bridge_surface_error_response(
    error: BridgeSurfaceError,
    app_state: &AppState,
    session: &ValidatedSurfaceSession,
    request_id: String,
) -> Response<ResponseBody> {
    // Emit a JSONL audit row for every bridge-surface rejection that
    // carries a watermark-class signature per packet §6.5 + ac §34.
    // Inline 409 stale_watermark callers also emit a domain-specific row;
    // this is the cross-class audit channel.
    emit_bridge_rejection_audit(app_state, session, &error);
    match error {
        BridgeSurfaceError::StaleVersion {
            claim_id,
            expected,
            current,
            correction: _,
        } => {
            stale_version_error_response(
                app_state,
                &session.actor,
                request_id,
                claim_id,
                expected,
                current,
            )
            .await
        }
        BridgeSurfaceError::MidFlightMutation {
            claim_id,
            mutation_id,
            retry_after_event,
        } => {
            mid_flight_mutation_error_response(request_id, claim_id, mutation_id, retry_after_event)
        }
        error => error_response(bridge_surface_error(error).with_request_id(request_id)),
    }
}

/// Build the 423 `mid_flight_mutation` HTTP response body. The base
/// envelope (`code`, `message`, `request_id`, `remediation`) is carried by
/// `SurfaceHttpError`; we layer the contention-resolution payload —
/// `claim_id`, `mutation_id`, and `retry_after_event.cursor` — alongside
/// the `error` object so surface clients can subscribe to the holder's
/// `mutation_attempts.cursor` for the terminal event without needing a
/// separate lookup. Mirrors the shape used by `stale_version_error_response`
/// for `stale_watermark` corrections (ac §7 + ac §24).
fn mid_flight_mutation_error_response(
    request_id: String,
    claim_id: String,
    mutation_id: String,
    retry_after_event: String,
) -> Response<ResponseBody> {
    let body = build_mid_flight_mutation_body(request_id, claim_id, mutation_id, retry_after_event);
    json_response(StatusCode::LOCKED, body)
}

pub(crate) fn build_mid_flight_mutation_body(
    request_id: String,
    claim_id: String,
    mutation_id: String,
    retry_after_event: String,
) -> serde_json::Value {
    let envelope = bridge_surface_error(BridgeSurfaceError::MidFlightMutation {
        claim_id: claim_id.clone(),
        mutation_id: mutation_id.clone(),
        retry_after_event: retry_after_event.clone(),
    })
    .with_request_id(request_id);
    let resolved_request_id = envelope.request_id.unwrap_or_else(new_request_id);
    json!({
        "error": {
            "code": envelope.code,
            "message": envelope.message,
            "request_id": resolved_request_id,
            "remediation": envelope.remediation,
        },
        "claim_id": claim_id,
        "mutation_id": mutation_id,
        "retry_after_event": {
            "cursor": retry_after_event,
        },
    })
}

/// Emit a JSONL audit row for a bridge-surface rejection per packet ac §34.
///
/// Pins the `detail` shape to `{ rejection_reason, expected_version,
/// current_version, claim_id, composition_id, mutation_id, scope_redacted,
/// actor_instance, actor_scopes, wp_user_id }`. `wp_user_id` is sent for
/// audit hash binding (the JSONL writer drops the raw value and stores
/// `wp_user_hash`); the unredacted claim/composition ids are caller-supplied
/// strings, not PII. `scope_redacted` is `true` whenever the rejection
/// envelope returned to the client suppressed claim content.
fn emit_bridge_rejection_audit(
    app_state: &AppState,
    session: &ValidatedSurfaceSession,
    error: &BridgeSurfaceError,
) {
    let (event_kind, detail) = match error {
        BridgeSurfaceError::StaleVersion {
            claim_id,
            expected,
            current,
            correction: _,
        } => (
            "claim.write_rejected",
            json!({
                "rejection_reason": "stale_watermark",
                "claim_id": claim_id,
                "composition_id": null,
                "expected_version": expected,
                "current_version": current,
                "mutation_id": null,
                "scope_redacted": true,
            }),
        ),
        BridgeSurfaceError::StaleComposition {
            composition_id,
            expected,
            current,
        } => (
            "composition.write_rejected",
            json!({
                "rejection_reason": "stale_composition_watermark",
                "claim_id": null,
                "composition_id": composition_id,
                "expected_version": expected,
                "current_version": current,
                "mutation_id": null,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::MidFlightMutation {
            claim_id,
            mutation_id,
            retry_after_event: _,
        } => (
            "claim.write_rejected",
            json!({
                "rejection_reason": "mid_flight_mutation",
                "claim_id": claim_id,
                "composition_id": null,
                "expected_version": null,
                "current_version": null,
                "mutation_id": mutation_id,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::MissingExpectedClaimVersion { claim_id } => (
            "claim.write_rejected",
            json!({
                "rejection_reason": "missing_expected_claim_version",
                "claim_id": claim_id,
                "composition_id": null,
                "expected_version": null,
                "current_version": null,
                "mutation_id": null,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::ClaimVersionOverflow { claim_id } => (
            "claim.write_rejected",
            json!({
                "rejection_reason": "claim_version_overflow",
                "claim_id": claim_id,
                "composition_id": null,
                "expected_version": null,
                "current_version": null,
                "mutation_id": null,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::CompositionVersionOverflow { composition_id } => (
            "composition.write_rejected",
            json!({
                "rejection_reason": "composition_version_overflow",
                "claim_id": null,
                "composition_id": composition_id,
                "expected_version": null,
                "current_version": null,
                "mutation_id": null,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::ProjectionTampered {
            projection_id,
            signature_id,
            key_id,
            observed_signature_status,
            quarantine_id,
        } => (
            "projection.tampered",
            json!({
                "rejection_reason": "projection_tampered",
                "projection_id": projection_id,
                "signature_id": signature_id,
                "key_id": key_id,
                "observed_signature_status": observed_signature_status,
                "quarantine_id": quarantine_id,
                "scope_redacted": false,
            }),
        ),
        BridgeSurfaceError::ProjectionVersionRollback {
            projection_id,
            signed_composition_version,
            ledger_composition_version,
            signed_claim_version,
            ledger_claim_version,
        } => (
            "projection.version_rollback",
            json!({
                "rejection_reason": "projection_version_rollback",
                "projection_id": projection_id,
                "signed_composition_version": signed_composition_version,
                "ledger_composition_version": ledger_composition_version,
                "signed_claim_version": signed_claim_version,
                "ledger_claim_version": ledger_claim_version,
                "scope_redacted": false,
            }),
        ),
        // Validation / AbilityUnavailable / Ownership do not carry a watermark signature.
        _ => return,
    };
    emit_pairing_audit_event(
        app_state,
        &SurfacePairingAuditEvent {
            event_kind,
            category: "data_access",
            actor: session.actor.clone(),
            wp_user_id: session.wp_user_id,
            wp_user_hash: session.wp_user_hash.clone(),
            detail,
        },
    );
}

async fn stale_version_error_response(
    app_state: &AppState,
    actor: &Actor,
    request_id: String,
    claim_id: String,
    expected: u64,
    current: u64,
) -> Response<ResponseBody> {
    let actor = actor.clone();
    let claim_id_for_lookup = claim_id.clone();
    let projection = match app_state
        .db_read(move |db| {
            let correction = project_claim_for_scope(db, &claim_id_for_lookup, &actor)
                .unwrap_or_else(CorrectionPayload::out_of_scope);
            let cursor = latest_claim_version_cursor(db, &claim_id_for_lookup, current)?;
            Ok((correction, cursor))
        })
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            return error_response(
                SurfaceHttpError::from_pairing_error(SurfacePairingError::Write(error))
                    .with_request_id(request_id),
            );
        }
    };

    let (correction, cursor) = projection;
    let mut correction = serde_json::to_value(correction).unwrap_or_else(|_| {
        json!({
            "claim": null,
            "scope_redacted": true,
            "reason": "out_of_scope",
        })
    });
    correction["retry_after_ms"] = Value::Null;
    correction["cursor"] = cursor.map(Value::String).unwrap_or(Value::Null);

    json_response(
        StatusCode::CONFLICT,
        json!({
            "ok": false,
            "error": "stale_watermark",
            "request_id": request_id,
            "claim_id": claim_id,
            "expected": { "claim_version": expected },
            "current": { "claim_version": current },
            "correction": correction,
        }),
    )
}

fn latest_claim_version_cursor(
    db: &crate::db::ActionDb,
    claim_id: &str,
    current: u64,
) -> Result<Option<String>, String> {
    let current = i64::try_from(current).map_err(|_| "claim_version_out_of_range".to_string())?;
    db.conn_ref()
        .query_row(
            "SELECT cursor
             FROM version_events
             WHERE claim_id = ?1 AND current_version = ?2
             ORDER BY event_seq DESC
             LIMIT 1",
            rusqlite::params![claim_id, current],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn surface_bridge_error(error: SurfaceClientBridgeError) -> SurfaceHttpError {
    if let Some(_surface_error) = error.as_surface_error() {
        return SurfaceHttpError::auth_missing()
            .with_message("The requested DailyOS surface ability is not available.")
            .with_remediation("Use an ability exposed to this paired surface.");
    }
    SurfaceHttpError::runtime_unavailable()
}

fn bridge_surface_error(error: BridgeSurfaceError) -> SurfaceHttpError {
    match error {
        BridgeSurfaceError::ProjectionTampered { .. } => SurfaceHttpError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "projection_tampered",
            "The submitted projection could not be verified.",
            "Refresh the DailyOS projection and retry.",
        ),
        BridgeSurfaceError::ProjectionVersionRollback { .. } => SurfaceHttpError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "projection_version_rollback",
            "The submitted projection is older than the current DailyOS ledger.",
            "Refresh the DailyOS projection and retry.",
        ),
        BridgeSurfaceError::MissingExpectedClaimVersion { .. } => {
            SurfaceHttpError::bad_request("missing_expected_claim_version")
        }
        BridgeSurfaceError::MidFlightMutation { .. } => SurfaceHttpError::new(
            StatusCode::LOCKED,
            "mid_flight_mutation",
            "Another accepted mutation is still being finalized.",
            "Wait for the mutation cursor event, then retry if needed.",
        ),
        BridgeSurfaceError::ClaimVersionOverflow { .. } => SurfaceHttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "claim_version_overflow",
            "DailyOS could not assign a new claim version.",
            "Restart DailyOS and run diagnostics before retrying.",
        ),
        BridgeSurfaceError::StaleVersion { .. } => SurfaceHttpError::new(
            StatusCode::CONFLICT,
            "stale_watermark",
            "The submitted claim version is stale.",
            "Refresh the claim projection and retry.",
        ),
        BridgeSurfaceError::StaleComposition { .. } => SurfaceHttpError::new(
            StatusCode::CONFLICT,
            "stale_composition_watermark",
            "The submitted composition version is stale.",
            "Refresh the composition projection and retry.",
        ),
        BridgeSurfaceError::CompositionVersionOverflow { .. } => SurfaceHttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "composition_version_overflow",
            "DailyOS could not assign a new composition version.",
            "Restart DailyOS and run diagnostics before retrying.",
        ),
        BridgeSurfaceError::Validation(_) => {
            SurfaceHttpError::bad_request("surface_invoke_invalid")
        }
        BridgeSurfaceError::AbilityUnavailable | BridgeSurfaceError::Ownership(_) => {
            SurfaceHttpError::auth_missing()
                .with_message("The requested DailyOS surface ability is not available.")
                .with_remediation("Use an ability exposed to this paired surface.")
        }
    }
}

fn register_session_after_revocations(
    signed_transport: &hmac::SignedTransportState,
    revoked_surface_client_ids: &[String],
    session: hmac::SignedSurfaceSession,
) -> Result<(), hmac::SignedTransportError> {
    for surface_client_id in revoked_surface_client_ids {
        signed_transport.remove_sessions_for_surface_client(surface_client_id);
    }
    signed_transport.register_session(session)
}

fn evict_cached_session_after_validation_error(
    signed_transport: &hmac::SignedTransportState,
    surface_client_id: &str,
    error: &SurfacePairingError,
) {
    if validation_error_invalidates_cached_session(error) {
        signed_transport.remove_sessions_for_surface_client(surface_client_id);
    }
}

fn validation_error_invalidates_cached_session(error: &SurfacePairingError) -> bool {
    matches!(
        error,
        SurfacePairingError::UnknownRuntimeAnchor
            | SurfacePairingError::SessionInvalid
            | SurfacePairingError::SessionExpired
            | SurfacePairingError::PairingSuspended
            | SurfacePairingError::PairingRevoked
            | SurfacePairingError::PairingExpired
            | SurfacePairingError::RestoredStalePairing
            | SurfacePairingError::SiteBindingMismatch
            | SurfacePairingError::WpUserMismatch
            | SurfacePairingError::ScopeDenied
    )
}

fn validation_rejection_events(
    verified: &hmac::VerifiedSignedRequest,
    error: &SurfacePairingError,
    scopes_for_audit: Option<&ScopeSet>,
) -> Vec<SurfacePairingAuditEvent> {
    let mut event_kinds: Vec<&'static str> = match error {
        SurfacePairingError::UnknownRuntimeAnchor => {
            vec!["pairing.reinstall.runtime_anchor_missing"]
        }
        SurfacePairingError::RestoredStalePairing => {
            vec!["pairing.restore.stale_epoch_detected"]
        }
        SurfacePairingError::PairingRevoked => {
            vec!["pairing.restore.revoked_proof_presented"]
        }
        SurfacePairingError::PairingExpired | SurfacePairingError::SessionExpired => {
            vec!["pairing.restore.expired_proof_presented"]
        }
        SurfacePairingError::SiteBindingMismatch | SurfacePairingError::WpUserMismatch => {
            vec!["pairing.site_binding.mismatch_detected"]
        }
        _ => Vec::new(),
    };
    if matches!(error, SurfacePairingError::SiteBindingMismatch) {
        event_kinds.push("pairing.exfiltration.off_host_binding_failure");
    }

    // Attribute the rejected attempt to the claimed SurfaceClient identity when
    // we can recover its granted scope set; the runtime is rejecting the
    // request but the requester's identity was HMAC-verified upstream and
    // belongs on the audit row for forensic traceability. Fall back to
    // `Actor::System` if the pairing record is gone (extremely rare — the
    // session_id reached HMAC verification, so a paired row almost certainly
    // existed at request time).
    let (actor, wp_user_id, wp_user_hash) = match scopes_for_audit {
        Some(scopes) => (
            Actor::SurfaceClient {
                instance: SurfaceClientId::new(verified.surface_client_id.clone()),
                scopes: scopes.clone(),
            },
            Some(verified.wp_user_id),
            Some(verified.wp_user_hash.clone()),
        ),
        None => (Actor::System, None, None),
    };

    event_kinds
        .into_iter()
        .map(|event_kind| SurfacePairingAuditEvent {
            event_kind,
            category: "security",
            actor: actor.clone(),
            wp_user_id,
            wp_user_hash: wp_user_hash.clone(),
            detail: json!({
                "surface_client_id": verified.surface_client_id,
                "session_id_hash": hmac::hash_prefix(&verified.session_id),
                "presented_site_binding_digest": verified.site_binding_digest,
                "presented_site_nonce_hash": hmac::hash_prefix(&verified.site_nonce),
                "reason": error.code(),
                "decision": "rejected"
            }),
        })
        .collect()
}

fn signed_transport_failure_event(
    verified_session_id: &str,
    surface_client_id: Option<&str>,
    failure_code: &str,
) -> Option<SurfacePairingAuditEvent> {
    (failure_code == "nonce_replay").then(|| SurfacePairingAuditEvent {
        event_kind: "pairing.exfiltration.nonce_replay",
        category: "security",
        actor: abilities_runtime::abilities::registry::Actor::System,
        wp_user_id: None,
        wp_user_hash: None,
        detail: json!({
            "surface_client_id": surface_client_id,
            "session_id_hash": hmac::hash_prefix(verified_session_id),
            "reason": failure_code,
            "decision": "rejected"
        }),
    })
}

fn successful_surface_invocation_audit_event(
    session: &ValidatedSurfaceSession,
    request_id: &str,
    input: &Value,
    ability: &AbilityResponseJson,
) -> SurfacePairingAuditEvent {
    let mut detail = json!({
        "surface_client_id": &session.surface_client_id,
        "actor_instance": &session.surface_client_id,
        "ability_name": &ability.ability_name,
        "ability_version": &ability.ability_version,
        "schema_version": ability.schema_version,
        "request_id": request_id,
        "claim_ref_count": composition_claim_ref_count(&ability.data),
    });
    if let Some(account_id) = input
        .get("account_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        detail["account_id"] = json!(account_id);
    }
    if let Some(composition_id) = ability.data.get("id").and_then(Value::as_str) {
        detail["composition_id"] = json!(composition_id);
    }
    if let Some(composition_version) = ability
        .data
        .pointer("/metadata/composition_version")
        .and_then(Value::as_u64)
    {
        detail["composition_version"] = json!(composition_version);
    }

    SurfacePairingAuditEvent {
        event_kind: "ability_invoked",
        category: "data_access",
        actor: session.actor.clone(),
        wp_user_id: session.wp_user_id,
        wp_user_hash: session.wp_user_hash.clone(),
        detail,
    }
}

fn composition_claim_ref_count(composition: &Value) -> usize {
    composition
        .get("sections")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|section| {
            section
                .get("blocks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|block| {
            block
                .get("claim_refs")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        })
        .sum()
}

fn emit_pairing_audit_event(app_state: &AppState, event: &SurfacePairingAuditEvent) {
    let mut audit = app_state.audit_log.lock();
    if let Err(error) = surface_pairing::emit_pairing_audit(&mut audit, event) {
        log::warn!("surface pairing audit write failed: {error}");
    }
}

fn health_response(request_id: String) -> Response<ResponseBody> {
    json_response(
        StatusCode::OK,
        json!({
            "ok": true,
            "endpoint_version": SURFACE_ENDPOINT_VERSION,
            "request_id": request_id,
        }),
    )
}

#[derive(Clone, Debug)]
struct SurfaceHttpError {
    status: StatusCode,
    code: &'static str,
    message: String,
    request_id: Option<String>,
    remediation: String,
    retry_after_ms: Option<u64>,
    rate_limit_axis: Option<SurfaceClientRateLimitAxis>,
}

impl SurfaceHttpError {
    fn host_invalid() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "host_invalid",
            "Use the paired DailyOS endpoint exactly.",
            "Use the endpoint host shown by DailyOS during pairing.",
        )
    }

    fn browser_origin_forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "browser_origin_forbidden",
            "Browser-originated requests cannot call the DailyOS runtime directly.",
            "Route the request through the paired WordPress server client.",
        )
    }

    fn auth_missing() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "auth_missing",
            "The request is missing DailyOS surface authentication.",
            "Pair the surface with DailyOS and retry with signed credentials.",
        )
    }

    fn from_signed_transport(error: hmac::SignedTransportError) -> Self {
        let remediation = match error.kind {
            hmac::SignedTransportErrorKind::NonceReplay => {
                "Generate a fresh signed request and retry."
            }
            hmac::SignedTransportErrorKind::TransportAbuseLimited => {
                "Wait before retrying the signed request."
            }
            _ => "Refresh the paired DailyOS session and retry.",
        };
        Self::new(
            error.status(),
            error.code(),
            "DailyOS request signing failed.",
            remediation,
        )
    }

    fn from_pairing_error(error: SurfacePairingError) -> Self {
        let remediation = match error {
            SurfacePairingError::PairingCodeExpired
            | SurfacePairingError::PairingCodeConsumed
            | SurfacePairingError::PairingCodeInvalid
            | SurfacePairingError::PairingCodeLimited => {
                "Generate a fresh pairing string in DailyOS and retry."
            }
            SurfacePairingError::SiteBindingMismatch => {
                "Review the paired site identity in DailyOS and pair the site again."
            }
            SurfacePairingError::PairingSuspended
            | SurfacePairingError::PairingRevoked
            | SurfacePairingError::PairingExpired
            | SurfacePairingError::SessionExpired
            | SurfacePairingError::SessionThrottled
            | SurfacePairingError::WpUserMismatch
            | SurfacePairingError::RestoredStalePairing => {
                "Pair the surface with DailyOS again before retrying."
            }
            SurfacePairingError::Write(_) => "Retry after restarting DailyOS.",
            _ => "Refresh the paired DailyOS session and retry.",
        };
        Self::new(
            error.status(),
            error.code(),
            error.safe_message(),
            remediation,
        )
    }

    fn session_not_found() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "session_not_found",
            "The DailyOS surface session could not be refreshed.",
            "Pair the surface with DailyOS again before retrying.",
        )
    }

    fn identity_mismatch() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "identity_mismatch",
            "The DailyOS surface session identity does not match the paired site.",
            "Review the paired site identity in DailyOS and pair the site again.",
        )
    }

    fn wrong_user() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "wrong_user",
            "The request user does not match the paired DailyOS surface session.",
            "Refresh the paired DailyOS session and retry as the same WordPress user.",
        )
    }

    fn route_not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "route_not_found",
            "No DailyOS surface endpoint is available for this request.",
            "Use the W2 surface endpoint route set.",
        )
    }

    fn bad_request(code: &'static str) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            code,
            "The request could not be processed safely.",
            "Retry with the canonical DailyOS surface request shape.",
        )
    }

    fn payload_too_large() -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "request_body_too_large",
            "The request body is too large.",
            "Generate a fresh pairing string and retry with the expected request shape.",
        )
    }

    fn loopback_rate_limited(retry_after: Duration) -> Self {
        let retry_after_ms = retry_after.as_millis().try_into().unwrap_or(u64::MAX);
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "The DailyOS runtime is receiving too many surface requests.",
            "Wait before retrying the request.",
        )
        .with_retry_after_ms(retry_after_ms)
    }

    fn rate_limited(axis: SurfaceClientRateLimitAxis, retry_after: Duration) -> Self {
        let retry_after_ms = retry_after.as_millis().try_into().unwrap_or(u64::MAX);
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "The DailyOS runtime is receiving too many surface requests.",
            "Wait before retrying the request.",
        )
        .with_retry_after_ms(retry_after_ms)
        .with_rate_limit_axis(axis)
    }

    fn rate_limited_without_retry() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "The DailyOS runtime is receiving too many surface requests.",
            "Wait before retrying the request.",
        )
    }

    fn runtime_unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime_unavailable",
            "The DailyOS surface runtime is not available for this request.",
            "Open DailyOS, generate a fresh pairing string, and retry.",
        )
    }

    #[cfg(test)]
    fn version_skew() -> Self {
        Self::new(
            StatusCode::CONFLICT,
            "version_skew",
            "The submitted composition version is stale.",
            "Refresh the composition from DailyOS and retry.",
        )
    }

    fn new(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id: None,
            remediation: remediation.into(),
            retry_after_ms: None,
            rate_limit_axis: None,
        }
    }

    fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    fn with_safe_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = remediation.into();
        self
    }

    fn with_retry_after_ms(mut self, retry_after_ms: u64) -> Self {
        self.retry_after_ms = Some(retry_after_ms);
        self
    }

    fn with_rate_limit_axis(mut self, axis: SurfaceClientRateLimitAxis) -> Self {
        self.rate_limit_axis = Some(axis);
        self
    }
}

fn error_response(error: SurfaceHttpError) -> Response<ResponseBody> {
    let request_id = error.request_id.unwrap_or_else(new_request_id);
    let mut body = json!({
        "error": {
            "code": error.code,
            "message": error.message,
            "request_id": request_id,
            "remediation": error.remediation,
        }
    });

    if let Some(retry_after_ms) = error.retry_after_ms {
        body["error"]["retry_after_ms"] = json!(retry_after_ms);
    }
    if let Some(axis) = error.rate_limit_axis {
        body["error"]["axis"] = json!(axis.as_str());
    }

    let mut response = json_response(error.status, body);
    if let Some(retry_after_ms) = error.retry_after_ms {
        let retry_after_seconds = retry_after_ms.div_ceil(1_000).max(1);
        let header_value = HeaderValue::from_str(&retry_after_seconds.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("1"));
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, header_value);
    }
    if let Some(axis) = error.rate_limit_axis {
        response.headers_mut().insert(
            HeaderName::from_static("x-ratelimit-exhausted-axis"),
            HeaderValue::from_static(axis.as_str()),
        );
    }
    response
}

fn json_response(status: StatusCode, value: serde_json::Value) -> Response<ResponseBody> {
    let bytes = serde_json::to_vec(&value).unwrap_or_else(|error| {
        log::warn!("surface endpoint failed to serialize response: {error}");
        b"{\"error\":{\"code\":\"runtime_unavailable\",\"message\":\"The DailyOS surface runtime is not available for this request.\",\"request_id\":\"serialization_failed\",\"remediation\":\"Retry after restarting DailyOS.\"}}".to_vec()
    });

    let mut response = Response::new(Full::new(Bytes::from(bytes)));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn validate_transport_headers(
    headers: &HeaderMap,
    bound_port: u16,
    paired_site_origins: &HashSet<String>,
) -> Result<(), SurfaceHttpError> {
    validate_host(headers, bound_port)?;
    validate_origin(headers, paired_site_origins)?;
    Ok(())
}

fn validate_host(headers: &HeaderMap, bound_port: u16) -> Result<(), SurfaceHttpError> {
    let mut host_values = headers.get_all(header::HOST).iter();
    let Some(host) = host_values.next() else {
        return Err(SurfaceHttpError::host_invalid());
    };
    if host_values.next().is_some() {
        return Err(SurfaceHttpError::host_invalid());
    }
    let Ok(host) = host.to_str() else {
        return Err(SurfaceHttpError::host_invalid());
    };
    let expected = format!("127.0.0.1:{bound_port}");
    if host == expected {
        Ok(())
    } else {
        Err(SurfaceHttpError::host_invalid())
    }
}

fn validate_origin(
    headers: &HeaderMap,
    paired_site_origins: &HashSet<String>,
) -> Result<(), SurfaceHttpError> {
    let mut origin_values = headers.get_all(header::ORIGIN).iter();
    let Some(origin) = origin_values.next() else {
        return Ok(());
    };
    if origin_values.next().is_some() {
        return Err(SurfaceHttpError::browser_origin_forbidden());
    }
    let Ok(origin) = origin.to_str() else {
        return Err(SurfaceHttpError::browser_origin_forbidden());
    };
    let origin = origin.trim();
    if origin.is_empty() {
        return Ok(());
    }
    if origin.eq_ignore_ascii_case("null") {
        return Err(SurfaceHttpError::browser_origin_forbidden());
    }

    match normalize_origin(origin) {
        Some(origin) if paired_site_origins.contains(&origin) => Ok(()),
        _ => Err(SurfaceHttpError::browser_origin_forbidden()),
    }
}

fn safe_header_value(headers: &HeaderMap, name: &'static str) -> Result<String, ()> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Err(());
    };
    if values.next().is_some() {
        return Err(());
    }
    let value = value.to_str().map_err(|_| ())?.trim();
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(());
    }
    Ok(value.to_string())
}

fn normalize_origin(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    if url.cannot_be_a_base() {
        return None;
    }
    Some(url.origin().unicode_serialization())
}

fn request_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-dailyos-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| is_safe_request_id(value))
        .map(ToString::to_string)
        .unwrap_or_else(new_request_id)
}

fn is_safe_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn new_request_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Deserialize)]
struct PairingHandshakeProbe {
    pairing_code: Option<String>,
}

fn pairing_code_from_body(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<PairingHandshakeProbe>(body)
        .ok()
        .and_then(|probe| probe.pairing_code)
        .map(|code| code.trim().to_string())
        .filter(|code| !code.is_empty() && code.len() <= 128)
}

#[derive(Clone, Debug)]
struct TokenBucket {
    config: TokenBucketConfig,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(config: TokenBucketConfig) -> Self {
        Self {
            tokens: f64::from(config.capacity),
            config,
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self, now: Instant) -> Result<(), Duration> {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return Ok(());
        }

        let refill_per_second = self.config.refill_per_second.max(f64::EPSILON);
        let seconds_until_next = (1.0 - self.tokens) / refill_per_second;
        Err(Duration::from_secs_f64(seconds_until_next))
    }

    fn refill(&mut self, now: Instant) {
        if now <= self.last_refill {
            return;
        }
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let refill = elapsed * self.config.refill_per_second.max(f64::EPSILON);
        self.tokens = (self.tokens + refill).min(f64::from(self.config.capacity));
        self.last_refill = now;
    }
}

#[derive(Clone, Debug, Default)]
struct PairingAttemptLimiter {
    config: PairingAttemptConfig,
    attempts_by_code: HashMap<String, u32>,
}

impl PairingAttemptLimiter {
    fn record_failed_attempt(&mut self, code: &str) -> PairingAttemptDecision {
        let max_attempts = self.config.max_failed_attempts_per_code.max(1);
        let attempts = self.attempts_by_code.entry(code.to_string()).or_insert(0);
        if *attempts >= max_attempts {
            return PairingAttemptDecision::Limited;
        }
        *attempts += 1;
        PairingAttemptDecision::Allowed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairingAttemptDecision {
    Allowed,
    Limited,
}

impl PairingAttemptDecision {
    fn is_limited(self) -> bool {
        matches!(self, Self::Limited)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{
        AbilityContext, AbilityDescriptor, AbilityPolicy, McpExposure, ScopeSet, SignalPolicy,
        SurfaceClientId, SurfaceScope,
    };
    use crate::abilities::{AbilityCategory, AbilityError, Actor, ActorKind};
    use std::future::Future;
    use std::io::{Read, Write};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static SURFACE_ROUTE_DISPATCH_COUNT: AtomicUsize = AtomicUsize::new(0);
    static SURFACE_ROUTE_LIMIT_COUNT: AtomicUsize = AtomicUsize::new(0);

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn surface_route_dispatch_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        SURFACE_ROUTE_DISPATCH_COUNT.fetch_add(1, Ordering::SeqCst);
        surface_route_output(ctx, input, "surface_route_test")
    }

    fn surface_route_limit_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        SURFACE_ROUTE_LIMIT_COUNT.fetch_add(1, Ordering::SeqCst);
        surface_route_output(ctx, input, "surface_route_limited_test")
    }

    fn surface_route_output<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
        ability_name: &'static str,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(json!({
                "data": {
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                },
                "provenance": {
                    "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                    "ability_name": ability_name,
                    "ability_version": { "major": 1, "minor": 0 },
                    "ability_schema_version": 1,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                    "warnings": []
                },
                "diagnostics": { "warnings": [] }
            }))
        })
    }

    fn surface_route_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "value": { "type": "number" }
            }
        })
    }

    fn runtime_for_tests(port: u16) -> Arc<EndpointRuntime> {
        Arc::new(EndpointRuntime {
            startup_id: Uuid::new_v4(),
            bound_port: port,
            runtime_anchor_id: "test_runtime_anchor".to_string(),
            loopback_bucket: Mutex::new(TokenBucket::new(TokenBucketConfig {
                capacity: 100,
                refill_per_second: 100.0,
            })),
            pairing_attempts: Arc::new(Mutex::new(PairingAttemptLimiter {
                config: PairingAttemptConfig {
                    max_failed_attempts_per_code: 5,
                },
                attempts_by_code: HashMap::new(),
            })),
            paired_site_origins: Arc::new(RwLock::new(HashSet::new())),
            signed_transport: hmac::SignedTransportState::default(),
            signed_request_max_body_bytes: DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES,
            surface_client_bridge: SurfaceClientBridge::new(SurfaceClientBridgeConfig::default()),
            surface_nonce: SurfaceNonceService::new_from_w2b_secret([7_u8; 32])
                .expect("nonce service"),
            ability_registry_override: None,
            app_state: None,
        })
    }

    fn surface_route_descriptor(
        name: &'static str,
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
    ) -> AbilityDescriptor {
        surface_route_descriptor_with_client_side_policy(name, invoke_erased, true)
    }

    fn surface_route_descriptor_with_client_side_policy(
        name: &'static str,
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
        client_side_executable: bool,
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category: AbilityCategory::Read,
            policy: AbilityPolicy {
                allowed_actors: &[ActorKind::SurfaceClient],
                allowed_modes: &[crate::services::context::ExecutionMode::Live],
                requires_confirmation: false,
                may_publish: false,
                required_scopes: &["read.account_overview"],
                mcp_exposure: McpExposure::None,
                client_side_executable,
                rate_limit: None,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy::default(),
            invoke_erased,
            input_schema: surface_route_schema,
            output_schema: surface_route_schema,
        }
    }

    fn runtime_for_surface_route_tests(
        registry: Arc<crate::abilities::AbilityRegistry>,
        bridge_config: SurfaceClientBridgeConfig,
    ) -> Arc<EndpointRuntime> {
        let app_state = Arc::new(AppState::new());
        let audit_path = std::env::temp_dir().join(format!(
            "dailyos-surface-runtime-test-{}.jsonl",
            Uuid::new_v4()
        ));
        *app_state.audit_log.lock() = crate::audit_log::AuditLogger::new(audit_path);
        Arc::new(EndpointRuntime {
            startup_id: Uuid::new_v4(),
            bound_port: 49152,
            runtime_anchor_id: "test_runtime_anchor".to_string(),
            loopback_bucket: Mutex::new(TokenBucket::new(TokenBucketConfig {
                capacity: 100,
                refill_per_second: 100.0,
            })),
            pairing_attempts: Arc::new(Mutex::new(PairingAttemptLimiter {
                config: PairingAttemptConfig {
                    max_failed_attempts_per_code: 5,
                },
                attempts_by_code: HashMap::new(),
            })),
            paired_site_origins: Arc::new(RwLock::new(HashSet::new())),
            signed_transport: hmac::SignedTransportState::default(),
            signed_request_max_body_bytes: DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES,
            surface_client_bridge: SurfaceClientBridge::new(bridge_config),
            surface_nonce: SurfaceNonceService::new_from_w2b_secret([7_u8; 32])
                .expect("nonce service"),
            ability_registry_override: Some(registry),
            app_state: Some(app_state),
        })
    }

    fn surface_route_dispatch_registry() -> Arc<crate::abilities::AbilityRegistry> {
        surface_route_dispatch_registry_with_client_side_policy(true)
    }

    fn surface_route_dispatch_registry_with_client_side_policy(
        client_side_executable: bool,
    ) -> Arc<crate::abilities::AbilityRegistry> {
        SURFACE_ROUTE_DISPATCH_COUNT.store(0, Ordering::SeqCst);
        Arc::new(
            crate::abilities::AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(
                vec![surface_route_descriptor_with_client_side_policy(
                    "surface_route_test",
                    surface_route_dispatch_erased,
                    client_side_executable,
                )],
            ),
        )
    }

    fn surface_route_limit_registry() -> Arc<crate::abilities::AbilityRegistry> {
        SURFACE_ROUTE_LIMIT_COUNT.store(0, Ordering::SeqCst);
        Arc::new(
            crate::abilities::AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(
                vec![surface_route_descriptor(
                    "surface_route_limited_test",
                    surface_route_limit_erased,
                )],
            ),
        )
    }

    fn validated_surface_session_for_tests() -> ValidatedSurfaceSession {
        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
            .expect("test scope grant is non-empty");
        ValidatedSurfaceSession {
            surface_client_id: "surface_client_test".to_string(),
            session_id: "sess_test_1234567890".to_string(),
            actor: Actor::SurfaceClient {
                instance: SurfaceClientId::new("surface_client_test"),
                scopes,
            },
            wp_user_id: Some(42),
            wp_user_hash: Some("wp_user_hash_test".to_string()),
            wp_site_id: hmac::TEST_WP_SITE_ID.to_string(),
            wp_site_id_hash: "wp_site_hash_test".to_string(),
            site_binding_digest: hmac::TEST_SITE_BINDING_DIGEST.to_string(),
            site_nonce: hmac::TEST_SITE_NONCE.to_string(),
            scope_digest: "scope_digest_test".to_string(),
            granted_scopes: vec!["read.account_overview".to_string()],
        }
    }

    fn signed_route_for_tests(
        request: &SurfaceHttpRequest,
        runtime: &EndpointRuntime,
        validated: ValidatedSurfaceSession,
        request_id: &str,
    ) -> Response<ResponseBody> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(signed_route_response(
                request,
                runtime,
                validated,
                request_id.to_string(),
            ))
    }

    fn dispatch_for_tests(
        request: SurfaceHttpRequest,
        runtime: Arc<EndpointRuntime>,
        request_id: String,
    ) -> Response<ResponseBody> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(dispatch_surface_request(request, runtime, request_id))
    }

    fn request_for_tests(method: Method, path: &str, body: Bytes) -> SurfaceHttpRequest {
        request_with_headers_for_tests(method, path, HeaderMap::new(), body)
    }

    fn request_with_headers_for_tests(
        method: Method,
        path: &str,
        headers: HeaderMap,
        body: Bytes,
    ) -> SurfaceHttpRequest {
        SurfaceHttpRequest {
            method,
            uri: path.parse::<Uri>().unwrap(),
            headers,
            body,
        }
    }

    fn test_master_key() -> [u8; 32] {
        [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
            0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b,
            0x2c, 0x2d, 0x2e, 0x2f,
        ]
    }

    fn register_signed_session(runtime: &Arc<EndpointRuntime>) {
        runtime
            .signed_transport
            .register_session(hmac::SignedSurfaceSession::new_active(
                "sess_test_1234567890",
                "surface_client_test",
                test_master_key(),
            ))
            .unwrap();
    }

    fn alternate_master_key() -> [u8; 32] {
        [
            0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d,
            0x9e, 0x9f, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab,
            0xac, 0xad, 0xae, 0xaf,
        ]
    }

    #[test]
    fn replacement_registration_frees_revoked_sessions_before_capacity_check() {
        let runtime = runtime_for_tests(4411);
        runtime
            .signed_transport
            .configure(hmac::SignedTransportConfig {
                max_active_sessions: 1,
                ..hmac::SignedTransportConfig::default()
            });
        register_signed_session(&runtime);

        assert_eq!(
            runtime
                .signed_transport
                .register_session(hmac::SignedSurfaceSession::new_active(
                    "sess_replacement",
                    "surface_replacement",
                    alternate_master_key(),
                ))
                .unwrap_err()
                .kind,
            hmac::SignedTransportErrorKind::TransportAbuseLimited
        );

        register_session_after_revocations(
            &runtime.signed_transport,
            &["surface_client_test".to_string()],
            hmac::SignedSurfaceSession::new_active(
                "sess_replacement",
                "surface_replacement",
                alternate_master_key(),
            ),
        )
        .unwrap();
    }

    #[test]
    fn terminal_validation_errors_evict_cached_transport_sessions() {
        let runtime = runtime_for_tests(4411);
        runtime
            .signed_transport
            .configure(hmac::SignedTransportConfig {
                max_active_sessions: 1,
                ..hmac::SignedTransportConfig::default()
            });
        register_signed_session(&runtime);

        evict_cached_session_after_validation_error(
            &runtime.signed_transport,
            "surface_client_test",
            &SurfacePairingError::SessionExpired,
        );

        runtime.signed_transport.ensure_session_capacity().unwrap();
        register_signed_session(&runtime);
        evict_cached_session_after_validation_error(
            &runtime.signed_transport,
            "surface_client_test",
            &SurfacePairingError::SessionThrottled,
        );

        assert_eq!(
            runtime
                .signed_transport
                .ensure_session_capacity()
                .unwrap_err()
                .kind,
            hmac::SignedTransportErrorKind::TransportAbuseLimited
        );
        assert!(validation_error_invalidates_cached_session(
            &SurfacePairingError::SiteBindingMismatch
        ));
        assert!(validation_error_invalidates_cached_session(
            &SurfacePairingError::ScopeDenied
        ));
        assert!(!validation_error_invalidates_cached_session(
            &SurfacePairingError::Write("temporary db failure".into())
        ));
    }

    fn signed_headers_for_tests(
        method: &Method,
        path: &str,
        body: &[u8],
        nonce: &str,
    ) -> HeaderMap {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        signed_headers_with_timestamp_for_tests(method, path, body, nonce, &timestamp)
    }

    fn signed_headers_with_timestamp_for_tests(
        method: &Method,
        path: &str,
        body: &[u8],
        nonce: &str,
        timestamp: &str,
    ) -> HeaderMap {
        let uri = path.parse::<Uri>().unwrap();
        let signature = hmac::sign_request_for_tests(
            test_master_key(),
            "sess_test_1234567890",
            hmac::CanonicalRequest {
                method,
                uri: &uri,
                content_type: "application/json",
                body,
                identity: hmac::CanonicalIdentity {
                    site_binding_digest: hmac::TEST_SITE_BINDING_DIGEST,
                    site_nonce: hmac::TEST_SITE_NONCE,
                    wp_user_id: hmac::TEST_WP_USER_ID,
                    wp_site_id: hmac::TEST_WP_SITE_ID,
                    home_url: hmac::TEST_HOME_URL,
                    site_url: hmac::TEST_SITE_URL,
                    wp_install_uuid: hmac::TEST_WP_INSTALL_UUID,
                    plugin_instance_uuid: hmac::TEST_PLUGIN_INSTANCE_UUID,
                    multisite_blog_id: "",
                },
                nonce,
                timestamp,
            },
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-dailyos-surfaceclient",
            HeaderValue::from_static("surface_client_test"),
        );
        headers.insert(
            hmac::HEADER_SESSION_ID,
            HeaderValue::from_static("sess_test_1234567890"),
        );
        headers.insert(
            "x-dailyos-signature",
            HeaderValue::from_str(&format!("v1={signature}")).unwrap(),
        );
        headers.insert(
            "x-dailyos-timestamp",
            HeaderValue::from_str(timestamp).unwrap(),
        );
        headers.insert("x-dailyos-nonce", HeaderValue::from_str(nonce).unwrap());
        headers.insert(
            hmac::HEADER_SITE_BINDING_DIGEST,
            HeaderValue::from_static(hmac::TEST_SITE_BINDING_DIGEST),
        );
        headers.insert(
            hmac::HEADER_SITE_NONCE,
            HeaderValue::from_static(hmac::TEST_SITE_NONCE),
        );
        headers.insert(
            hmac::HEADER_WP_USER_ID,
            HeaderValue::from_static(hmac::TEST_WP_USER_ID),
        );
        headers.insert(
            hmac::HEADER_WP_SITE_ID,
            HeaderValue::from_static(hmac::TEST_WP_SITE_ID),
        );
        headers.insert(
            hmac::HEADER_HOME_URL,
            HeaderValue::from_static(hmac::TEST_HOME_URL),
        );
        headers.insert(
            hmac::HEADER_SITE_URL,
            HeaderValue::from_static(hmac::TEST_SITE_URL),
        );
        headers.insert(
            hmac::HEADER_WP_INSTALL_UUID,
            HeaderValue::from_static(hmac::TEST_WP_INSTALL_UUID),
        );
        headers.insert(
            hmac::HEADER_PLUGIN_INSTANCE_UUID,
            HeaderValue::from_static(hmac::TEST_PLUGIN_INSTANCE_UUID),
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers
    }

    #[test]
    fn host_guard_accepts_exact_loopback_port_only() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:49152"));
        assert!(validate_host(&headers, 49152).is_ok());

        for value in [
            "",
            "localhost:49152",
            "[::1]:49152",
            "0.0.0.0:49152",
            "127.0.0.1:49153",
            "127.000.000.001:49152",
            "127.0.0.1:49152 ",
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::HOST, HeaderValue::from_str(value).unwrap());
            assert!(validate_host(&headers, 49152).is_err(), "accepted {value}");
        }

        let headers = HeaderMap::new();
        assert!(validate_host(&headers, 49152).is_err());
    }

    #[test]
    fn origin_guard_is_php_curl_primary_positive_allowlist() {
        let no_origins = HashSet::new();
        let headers = HeaderMap::new();
        assert!(validate_origin(&headers, &no_origins).is_ok());

        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static(""));
        assert!(validate_origin(&headers, &no_origins).is_ok());

        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static("null"));
        assert!(validate_origin(&headers, &no_origins).is_err());

        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://subsidiary.com"),
        );
        assert!(validate_origin(&headers, &no_origins).is_err());
        let mut allowed_origins = HashSet::new();
        allowed_origins.insert("https://subsidiary.com".to_string());
        allowed_origins.insert("https://partner.com".to_string());
        assert!(validate_origin(&headers, &allowed_origins).is_ok());
        assert!(
            validate_origin(&headers, &HashSet::from(["https://parent.com".to_string()])).is_err()
        );
    }

    #[test]
    fn protected_routes_return_nested_auth_envelope() {
        for (method, path) in [
            (Method::GET, "/v1/pairing/status"),
            (Method::POST, "/v1/surface/invoke"),
            (Method::POST, "/v1/surface/feedback"),
            (Method::POST, "/v1/surface/nonce/issue"),
            (Method::POST, "/v1/surface/nonce/verify"),
            (Method::GET, "/v1/surface/abilities"),
            (Method::GET, "/v1/surface/keyring"),
        ] {
            let response = dispatch_for_tests(
                request_for_tests(method, path, Bytes::new()),
                runtime_for_tests(49152),
                "req_test".to_string(),
            );
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let body = body_json(response);
            assert_eq!(body["error"]["code"], "token_invalid");
            assert_eq!(body["error"]["request_id"], "req_test");
            assert!(body.get("error").unwrap().get("message").is_some());
            assert!(body.get("error").unwrap().get("remediation").is_some());
        }
    }

    #[test]
    fn signed_route_candidates_include_canonicalization_drift_paths() {
        for path in [
            "/v1/pairing/status/",
            "/v1/surface/invoke/",
            "/v1/surface/feedback/",
            "/v1/surface/nonce/issue/",
            "/v1/surface/nonce/verify/",
            "/v1/surface/abilities/",
            "/v1/surface/keyring/",
            "/v1/surface/unknown",
        ] {
            assert!(is_signed_route_candidate(path), "candidate missed {path}");
        }

        assert!(!is_signed_route_candidate("/v1/surface/health"));
        assert!(!is_signed_route_candidate("/v1/surface/session/refresh"));
        assert!(!is_signed_route_candidate("/v1/pairing/handshake"));
    }

    fn refresh_pairing_request(pairing_code: String) -> PairingHandshakeRequest {
        PairingHandshakeRequest {
            pairing_code,
            wp_user_id: 42,
            wp_site_id: hmac::TEST_WP_SITE_ID.to_string(),
            home_url: hmac::TEST_HOME_URL.to_string(),
            site_url: hmac::TEST_SITE_URL.to_string(),
            wp_install_uuid: hmac::TEST_WP_INSTALL_UUID.to_string(),
            plugin_instance_uuid: hmac::TEST_PLUGIN_INSTANCE_UUID.to_string(),
            multisite_blog_id: None,
            request_id: Some("req_refresh_pair".to_string()),
            client_metadata: Some(json!({"plugin_version": "0.0.0"})),
        }
    }

    fn runtime_with_refresh_pairing_for_tests() -> (
        Arc<EndpointRuntime>,
        surface_pairing::PairingHandshakeOutcome,
    ) {
        ScopeSet::set_allowlist_for_tests([
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
            SurfaceScope::new("submit.feedback"),
        ]);
        let tokio_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let db_path =
            std::env::temp_dir().join(format!("dailyos-session-refresh-{}.sqlite", Uuid::new_v4()));
        let db_service = tokio_runtime
            .block_on(crate::db_service::DbService::open_at_unencrypted(db_path))
            .unwrap();
        let app_state = Arc::new(AppState::test_with_db_service(db_service));
        let outcome = tokio_runtime.block_on({
            let app_state = Arc::clone(&app_state);
            async move {
                app_state
                    .db_write(|db| {
                        let clock = crate::services::context::SystemClock;
                        let rng = crate::services::context::SystemRng;
                        let external = crate::services::context::ExternalClients::default();
                        let ctx = crate::services::context::ServiceContext::new_live(
                            &clock, &rng, &external,
                        );
                        let now = Utc::now();
                        let issued = surface_pairing::issue_pairing_code(
                            &ctx,
                            db,
                            surface_pairing::PairingCodeIssueInput {
                                runtime_anchor_id: "anchor_refresh_test".to_string(),
                                endpoint_startup_id: "startup_refresh_test".to_string(),
                                bound_port: 49152,
                                now,
                            },
                        )
                        .map_err(|error| error.to_string())?;
                        surface_pairing::complete_handshake(
                            &ctx,
                            db,
                            PairingHandshakeInput {
                                runtime_anchor_id: "anchor_refresh_test".to_string(),
                                endpoint_startup_id: "startup_refresh_test".to_string(),
                                bound_port: 49152,
                                endpoint_version: SURFACE_ENDPOINT_VERSION,
                                max_failed_attempts: 5,
                                now,
                                request: refresh_pairing_request(issued.pairing_string),
                            },
                        )
                        .map_err(|error| error.to_string())
                    })
                    .await
                    .unwrap()
            }
        });
        let runtime = Arc::new(EndpointRuntime {
            startup_id: Uuid::new_v4(),
            bound_port: 49152,
            runtime_anchor_id: "anchor_refresh_test".to_string(),
            loopback_bucket: Mutex::new(TokenBucket::new(TokenBucketConfig {
                capacity: 100,
                refill_per_second: 100.0,
            })),
            pairing_attempts: Arc::new(Mutex::new(PairingAttemptLimiter {
                config: PairingAttemptConfig {
                    max_failed_attempts_per_code: 5,
                },
                attempts_by_code: HashMap::new(),
            })),
            paired_site_origins: Arc::new(RwLock::new(HashSet::new())),
            signed_transport: hmac::SignedTransportState::default(),
            signed_request_max_body_bytes: DEFAULT_SIGNED_REQUEST_MAX_BODY_BYTES,
            surface_client_bridge: SurfaceClientBridge::new(SurfaceClientBridgeConfig::default()),
            surface_nonce: SurfaceNonceService::new_from_w2b_secret([7_u8; 32])
                .expect("nonce service"),
            ability_registry_override: None,
            app_state: Some(app_state),
        });
        runtime
            .signed_transport
            .register_session(hmac::SignedSurfaceSession::new_active(
                outcome.session.session_id.clone(),
                outcome.session.surface_client_id.clone(),
                outcome.session.hmac_master_key,
            ))
            .unwrap();
        (runtime, outcome)
    }

    fn refresh_body_for_tests(
        outcome: &surface_pairing::PairingHandshakeOutcome,
        session_id: &str,
        plugin_instance_uuid: &str,
    ) -> Bytes {
        Bytes::from(
            serde_json::to_vec(&json!({
                "session_id": session_id,
                "site_binding_digest": outcome.response.site_binding_digest.as_str(),
                "wp_install_uuid": hmac::TEST_WP_INSTALL_UUID,
                "plugin_instance_uuid": plugin_instance_uuid,
            }))
            .unwrap(),
        )
    }

    #[test]
    fn session_refresh_returns_hex_hmac_key_for_matching_identity() {
        let (runtime, outcome) = runtime_with_refresh_pairing_for_tests();
        let response = dispatch_for_tests(
            request_for_tests(
                Method::POST,
                "/v1/surface/session/refresh",
                refresh_body_for_tests(
                    &outcome,
                    &outcome.session.session_id,
                    hmac::TEST_PLUGIN_INSTANCE_UUID,
                ),
            ),
            runtime,
            "req_refresh_success".to_string(),
        );

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response);
        assert_eq!(body["ok"], true);
        assert_eq!(body["request_id"], "req_refresh_success");
        assert_eq!(
            body["session_id"].as_str(),
            Some(outcome.session.session_id.as_str())
        );
        let expected_hmac_key = hex::encode(hmac::derive_session_key(
            outcome.session.hmac_master_key,
            &outcome.session.session_id,
        ));
        assert_eq!(body["hmac_key"].as_str(), Some(expected_hmac_key.as_str()));
    }

    #[test]
    fn session_refresh_rejects_identity_mismatch() {
        let (runtime, outcome) = runtime_with_refresh_pairing_for_tests();
        let response = dispatch_for_tests(
            request_for_tests(
                Method::POST,
                "/v1/surface/session/refresh",
                refresh_body_for_tests(&outcome, &outcome.session.session_id, "plugin_mismatch"),
            ),
            runtime,
            "req_refresh_mismatch".to_string(),
        );

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "identity_mismatch");
        assert_eq!(body["error"]["request_id"], "req_refresh_mismatch");
    }

    #[test]
    fn session_refresh_rejects_unknown_session_id() {
        let (runtime, outcome) = runtime_with_refresh_pairing_for_tests();
        let response = dispatch_for_tests(
            request_for_tests(
                Method::POST,
                "/v1/surface/session/refresh",
                refresh_body_for_tests(
                    &outcome,
                    "sess_missing_refresh",
                    hmac::TEST_PLUGIN_INSTANCE_UUID,
                ),
            ),
            runtime,
            "req_refresh_missing".to_string(),
        );

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "session_not_found");
        assert_eq!(body["error"]["request_id"], "req_refresh_missing");
    }

    #[test]
    fn valid_signed_request_reaches_protected_route_stub() {
        let runtime = runtime_for_tests(49152);
        register_signed_session(&runtime);
        let method = Method::POST;
        let path = "/v1/surface/invoke?ability=briefing.daily";
        let body = Bytes::from_static(br#"{"depth":"standard"}"#);
        let headers =
            signed_headers_for_tests(&method, path, &body, "0123456789abcdef0123456789abcdef");

        let response = dispatch_for_tests(
            request_with_headers_for_tests(method, path, headers, body),
            runtime,
            "req_signed".to_string(),
        );
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "runtime_unavailable");
        assert_eq!(body["error"]["request_id"], "req_signed");
    }

    #[test]
    fn valid_signed_unknown_surface_route_requires_session_gate_in_test_runtime() {
        let runtime = runtime_for_tests(49152);
        register_signed_session(&runtime);
        let method = Method::POST;
        let path = "/v1/surface/invoke/?ability=briefing.daily";
        let body = Bytes::from_static(br#"{"depth":"standard"}"#);
        let headers =
            signed_headers_for_tests(&method, path, &body, "1123456789abcdef0123456789abcdef");

        let response = dispatch_for_tests(
            request_with_headers_for_tests(method, path, headers, body),
            runtime,
            "req_unknown_signed".to_string(),
        );
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "runtime_unavailable");
        assert_eq!(body["error"]["request_id"], "req_unknown_signed");
    }

    #[test]
    fn tampered_protected_path_rejects_at_signing_before_route_not_found() {
        let runtime = runtime_for_tests(49152);
        register_signed_session(&runtime);
        let method = Method::POST;
        let signed_path = "/v1/surface/invoke?ability=briefing.daily";
        let sent_path = "/v1/surface/invoke/?ability=briefing.daily";
        let body = Bytes::from_static(br#"{"depth":"standard"}"#);
        let headers = signed_headers_for_tests(
            &method,
            signed_path,
            &body,
            "2123456789abcdef0123456789abcdef",
        );

        let response = dispatch_for_tests(
            request_with_headers_for_tests(method, sent_path, headers, body),
            runtime,
            "req_tampered_path".to_string(),
        );
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "signature_invalid");
        assert_eq!(body["error"]["message"], "DailyOS request signing failed.");
        assert_eq!(body["error"]["request_id"], "req_tampered_path");
    }

    #[test]
    fn invalid_signed_request_stops_before_protected_route_stub() {
        let runtime = runtime_for_tests(49152);
        register_signed_session(&runtime);
        let method = Method::POST;
        let path = "/v1/surface/invoke?ability=briefing.daily";
        let signed_body = br#"{"depth":"standard"}"#;
        let sent_body = Bytes::from_static(br#"{"depth":"deep"}"#);
        let headers = signed_headers_for_tests(
            &method,
            path,
            signed_body,
            "1123456789abcdef0123456789abcdef",
        );

        let response = dispatch_for_tests(
            request_with_headers_for_tests(method, path, headers, sent_body),
            runtime,
            "req_bad_sig".to_string(),
        );
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "signature_invalid");
        assert_eq!(body["error"]["message"], "DailyOS request signing failed.");
        assert_eq!(body["error"]["request_id"], "req_bad_sig");
    }

    #[test]
    fn health_response_is_low_information() {
        let response = dispatch_for_tests(
            request_for_tests(Method::GET, "/v1/surface/health", Bytes::new()),
            runtime_for_tests(49152),
            "req_health".to_string(),
        );
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response);
        assert_eq!(body["ok"], true);
        assert_eq!(body["endpoint_version"], SURFACE_ENDPOINT_VERSION);
        let text = body.to_string();
        for forbidden in [
            "49152",
            "startup",
            "surface_client",
            "Bearer",
            "HMAC",
            "ability",
            "grants",
            "provenance",
        ] {
            assert!(
                !text.contains(forbidden),
                "health leaked {forbidden}: {text}"
            );
        }
    }

    #[test]
    fn pairing_status_omits_internal_lifecycle_fields() {
        let endpoint = SurfaceEndpointState::default();
        let status = serde_json::to_value(endpoint.pairing_status()).unwrap();
        assert!(status.get("availability").is_some());
        assert!(status.get("boundPort").is_some());
        assert!(status.get("endpointVersion").is_some());
        assert!(status.get("startupId").is_none());
        assert!(status.get("lastError").is_none());
        assert!(status.get("canCancel").is_none());
    }

    #[test]
    fn stop_clears_paired_origin_state() {
        let endpoint = SurfaceEndpointState::default();
        endpoint.set_paired_site_url_for_origin_guard(Some("https://subsidiary.com"));
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://subsidiary.com"),
        );
        assert!(validate_origin(&headers, &endpoint.paired_site_origins.read()).is_ok());

        endpoint.stop();

        assert!(validate_origin(&headers, &endpoint.paired_site_origins.read()).is_err());
    }

    #[test]
    fn typed_error_envelope_supports_409_shape() {
        let response =
            error_response(SurfaceHttpError::version_skew().with_request_id("req_409".into()));
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = body_json(response);
        assert_eq!(body["error"]["code"], "version_skew");
        assert_eq!(body["error"]["request_id"], "req_409");
        assert!(body["error"]["remediation"]
            .as_str()
            .unwrap()
            .contains("Refresh"));
    }

    #[test]
    fn token_bucket_exhaustion_returns_retry_after() {
        let mut bucket = TokenBucket::new(TokenBucketConfig {
            capacity: 1,
            refill_per_second: 1.0,
        });
        let now = Instant::now();
        assert!(bucket.try_acquire(now).is_ok());
        let retry_after = bucket.try_acquire(now).unwrap_err();
        assert!(retry_after >= Duration::from_millis(900));
    }

    #[test]
    fn token_bucket_honors_sub_sixty_per_minute_refill() {
        let mut bucket = TokenBucket::new(TokenBucketConfig {
            capacity: 1,
            refill_per_second: 30.0 / 60.0,
        });
        let now = Instant::now();
        assert!(bucket.try_acquire(now).is_ok());
        let retry_after = bucket.try_acquire(now).unwrap_err();
        assert!(retry_after >= Duration::from_millis(1_900));
    }

    #[test]
    fn surface_runtime_config_mapping_clamps_bind_attempts_and_preserves_rate() {
        let config = crate::types::SurfaceRuntimeConfig {
            max_bind_attempts: 99,
            unauthenticated_loopback_requests_per_minute: 30,
            unauthenticated_loopback_burst_per_second: 0,
            pairing_code_max_failed_attempts: 5,
            signed_session_requests_per_minute: 120,
            signed_session_burst_per_second: 10,
            signature_stale_window_seconds: 30,
            signature_future_skew_seconds: 5,
            signature_nonce_cleanup_slack_seconds: 5,
            signature_nonce_pending_ttl_seconds: 5,
            signature_nonce_records_per_session: 4096,
            signature_max_active_sessions: 128,
            signature_global_nonce_records: 65_536,
            signed_request_max_body_bytes: 256 * 1024,
            surface_client_rate_limits: crate::types::SurfaceClientRateLimitConfig::default(),
        };
        let endpoint_config = SurfaceEndpointConfig::from(&config);
        assert_eq!(endpoint_config.max_bind_attempts, DEFAULT_MAX_BIND_ATTEMPTS);
        assert_eq!(endpoint_config.loopback_abuse.capacity, 1);
        assert_eq!(endpoint_config.loopback_abuse.refill_per_second, 0.5);
        assert_eq!(
            endpoint_config
                .signed_transport
                .parseable_session_bucket
                .capacity,
            10
        );
        assert_eq!(
            endpoint_config
                .signed_transport
                .parseable_session_bucket
                .refill_per_second,
            2.0
        );
        assert_eq!(
            endpoint_config.signed_transport.stale_window,
            Duration::from_secs(30)
        );
        assert_eq!(
            endpoint_config.signed_transport.future_skew,
            Duration::from_secs(5)
        );
        assert_eq!(
            endpoint_config.signed_transport.nonce_records_per_session,
            4096
        );
        assert_eq!(endpoint_config.signed_request_max_body_bytes, 256 * 1024);
    }

    #[test]
    fn pairing_attempt_bucket_limits_after_configured_failures() {
        let mut limiter = PairingAttemptLimiter {
            config: PairingAttemptConfig {
                max_failed_attempts_per_code: 5,
            },
            attempts_by_code: HashMap::new(),
        };
        for _ in 0..5 {
            assert_eq!(
                limiter.record_failed_attempt("123-456"),
                PairingAttemptDecision::Allowed
            );
        }
        assert_eq!(
            limiter.record_failed_attempt("123-456"),
            PairingAttemptDecision::Limited
        );
        assert_eq!(
            limiter.record_failed_attempt("789-000"),
            PairingAttemptDecision::Allowed
        );
    }

    #[test]
    fn handshake_without_app_state_does_not_claim_pairing_attempt_authority() {
        let runtime = runtime_for_tests(49152);
        let body = Bytes::from_static(br#"{"pairing_code":"123-456"}"#);
        for attempt in 0..6 {
            let response = dispatch_for_tests(
                request_for_tests(Method::POST, "/v1/pairing/handshake", body.clone()),
                Arc::clone(&runtime),
                format!("req_{attempt}"),
            );
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body_json(response)["error"]["code"], "runtime_unavailable");
        }
    }

    #[test]
    fn mid_flight_mutation_body_carries_claim_mutation_and_cursor() {
        // The 423 response body must carry the contention-resolution payload
        // (claim_id, mutation_id, retry_after_event.cursor) alongside the
        // standard error envelope so surface clients can subscribe to the
        // holder's mutation_attempts.cursor for the terminal event.
        let body = build_mid_flight_mutation_body(
            "req-mid-flight-1".to_string(),
            "claim-mid-flight".to_string(),
            "mutation-holder".to_string(),
            "abcdef12-3456-4789-9abc-def012345678".to_string(),
        );
        assert_eq!(body["claim_id"], "claim-mid-flight");
        assert_eq!(body["mutation_id"], "mutation-holder");
        assert_eq!(
            body["retry_after_event"]["cursor"],
            "abcdef12-3456-4789-9abc-def012345678"
        );
        assert_eq!(body["error"]["code"], "mid_flight_mutation");
        assert_eq!(body["error"]["request_id"], "req-mid-flight-1");
        assert!(body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("being finalized"));
        assert!(body["error"]["remediation"]
            .as_str()
            .unwrap_or_default()
            .contains("mutation cursor event"));
    }

    #[test]
    fn surface_invoke_route_dispatches_after_bridge_authorization() {
        let registry = surface_route_dispatch_registry();
        let runtime =
            runtime_for_surface_route_tests(registry, SurfaceClientBridgeConfig::default());
        let request = request_for_tests(
            Method::POST,
            "/v1/surface/invoke",
            Bytes::from_static(br#"{"ability":"surface_route_test","input":{"value":217}}"#),
        );

        let response = signed_route_for_tests(
            &request,
            &runtime,
            validated_surface_session_for_tests(),
            "req_surface_invoke",
        );

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(SURFACE_ROUTE_DISPATCH_COUNT.load(Ordering::SeqCst), 1);
        let body = body_json(response);
        assert_eq!(body["ok"], true);
        assert_eq!(body["request_id"], "req_surface_invoke");
        assert_eq!(body["ability"]["data"]["input"]["value"], 217);
        assert_eq!(
            body["ability"]["rendered_provenance"]["surface"],
            "surface_client"
        );
        assert!(body["ability"].get("diagnostics").is_none());

        let audit_path = runtime
            .app_state
            .as_ref()
            .expect("test app state")
            .audit_log
            .lock()
            .path()
            .to_path_buf();
        let audit_records = crate::audit_log::read_records(&audit_path, 10, Some("data_access"));
        let invoked = audit_records
            .iter()
            .find(|record| record.event == "ability_invoked")
            .expect("successful invoke audit is written");
        assert_eq!(invoked.actor_kind.as_deref(), Some("surface_client"));
        assert_eq!(
            invoked.actor_instance.as_ref().map(SurfaceClientId::as_str),
            Some("surface_client_test")
        );
        assert_eq!(invoked.wp_user_hash.as_deref(), Some("wp_user_hash_test"));
        assert_eq!(invoked.detail["ability_name"], json!("surface_route_test"));
        assert_eq!(invoked.detail["claim_ref_count"], json!(0));
    }

    #[test]
    fn surface_invoke_route_allows_signed_surface_client_for_client_side_disabled_ability() {
        let registry = surface_route_dispatch_registry_with_client_side_policy(false);
        let runtime =
            runtime_for_surface_route_tests(registry, SurfaceClientBridgeConfig::default());
        let request = request_for_tests(
            Method::POST,
            "/v1/surface/invoke",
            Bytes::from_static(br#"{"ability":"surface_route_test","input":{"value":568}}"#),
        );

        let response = signed_route_for_tests(
            &request,
            &runtime,
            validated_surface_session_for_tests(),
            "req_surface_invoke_client_side_disabled",
        );

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(SURFACE_ROUTE_DISPATCH_COUNT.load(Ordering::SeqCst), 1);
        let body = body_json(response);
        assert_eq!(body["ok"], true);
        assert_eq!(
            body["request_id"],
            "req_surface_invoke_client_side_disabled"
        );
        assert_eq!(body["ability"]["data"]["input"]["value"], 568);
        assert_eq!(
            body["ability"]["rendered_provenance"]["surface"],
            "surface_client"
        );
    }

    #[test]
    fn account_overview_success_audit_carries_composition_fields_and_writes() {
        let session = validated_surface_session_for_tests();
        let ability: AbilityResponseJson = serde_json::from_value(json!({
            "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "ability_name": "dailyos/account-overview",
            "ability_version": "1.0.0",
            "schema_version": 1,
            "data": {
                "id": "acct-overview-fixture",
                "metadata": { "composition_version": 7 },
                "sections": [{
                    "blocks": [
                        { "claim_refs": [{ "claim_id": "claim-a" }, { "claim_id": "claim-b" }] },
                        { "claim_refs": [{ "claim_id": "claim-c" }] }
                    ]
                }]
            },
            "rendered_provenance": { "surface": "surface_client", "value": {} },
            "diagnostics": { "warnings": [] }
        }))
        .expect("ability response fixture deserializes");
        let event = successful_surface_invocation_audit_event(
            &session,
            "req_account_overview",
            &json!({ "account_id": "acct-fixture-1" }),
            &ability,
        );

        assert_eq!(event.event_kind, "ability_invoked");
        assert_eq!(event.wp_user_id, Some(42));
        assert_eq!(event.detail["actor_instance"], json!("surface_client_test"));
        assert_eq!(event.detail["account_id"], json!("acct-fixture-1"));
        assert_eq!(
            event.detail["composition_id"],
            json!("acct-overview-fixture")
        );
        assert_eq!(event.detail["composition_version"], json!(7));
        assert_eq!(event.detail["claim_ref_count"], json!(3));

        let audit_path = std::env::temp_dir().join(format!(
            "dailyos-account-overview-audit-test-{}.jsonl",
            Uuid::new_v4()
        ));
        let mut logger = crate::audit_log::AuditLogger::new(audit_path.clone());
        surface_pairing::emit_pairing_audit(&mut logger, &event)
            .expect("SurfaceClient success audit includes wp_user_id/hash");
        let raw = std::fs::read_to_string(&audit_path).expect("audit file is readable");
        assert!(!raw.contains("\"wp_user_id\""));
        let records = crate::audit_log::read_records(&audit_path, 10, Some("data_access"));
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].event, "ability_invoked");
        assert_eq!(records[0].actor_kind.as_deref(), Some("surface_client"));
        assert_eq!(
            records[0].wp_user_hash.as_deref(),
            Some("wp_user_hash_test")
        );
    }

    #[test]
    fn surface_invoke_rate_limit_denial_skips_ability_body_for_each_axis() {
        for expected_axis in [
            SurfaceClientRateLimitAxis::SurfaceClient,
            SurfaceClientRateLimitAxis::WpSite,
            SurfaceClientRateLimitAxis::WpUser,
            SurfaceClientRateLimitAxis::Scope,
            SurfaceClientRateLimitAxis::Ability,
        ] {
            let registry = surface_route_limit_registry();
            let mut bridge_config = SurfaceClientBridgeConfig::default();
            let one_per_second = SurfaceClientRateLimitBudget {
                requests_per_minute: 60,
                burst_per_second: 1,
            };
            match expected_axis {
                SurfaceClientRateLimitAxis::SurfaceClient => {
                    bridge_config.surface_client.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::WpSite => {
                    bridge_config.wp_site.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::WpUser => {
                    bridge_config.wp_user.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::Scope => {
                    bridge_config.scope.read = one_per_second;
                }
                SurfaceClientRateLimitAxis::Ability => {
                    bridge_config.ability.cheap_read = one_per_second;
                }
            }
            let runtime = runtime_for_surface_route_tests(registry, bridge_config);
            let body = Bytes::from_static(
                br#"{"ability":"surface_route_limited_test","input":{"value":217}}"#,
            );
            let first = request_for_tests(Method::POST, "/v1/surface/invoke", body.clone());
            let first_response = signed_route_for_tests(
                &first,
                &runtime,
                validated_surface_session_for_tests(),
                &format!("req_surface_first_{}", expected_axis.as_str()),
            );
            assert_eq!(first_response.status(), StatusCode::OK);
            assert_eq!(SURFACE_ROUTE_LIMIT_COUNT.load(Ordering::SeqCst), 1);

            let second = request_for_tests(Method::POST, "/v1/surface/invoke", body);
            let second_response = signed_route_for_tests(
                &second,
                &runtime,
                validated_surface_session_for_tests(),
                &format!("req_surface_second_{}", expected_axis.as_str()),
            );

            assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(SURFACE_ROUTE_LIMIT_COUNT.load(Ordering::SeqCst), 1);
            assert_eq!(second_response.headers()[header::RETRY_AFTER], "1");
            assert_eq!(
                second_response.headers()[HeaderName::from_static("x-ratelimit-exhausted-axis")],
                expected_axis.as_str()
            );
            let body = body_json(second_response);
            assert_eq!(body["error"]["code"], "rate_limited");
            assert_eq!(body["error"]["axis"], expected_axis.as_str());
            assert_eq!(
                body["error"]["request_id"],
                format!("req_surface_second_{}", expected_axis.as_str())
            );
            assert!(body["error"].get("rendered_provenance").is_none());
        }
    }

    #[tokio::test]
    async fn rate_limit_response_uses_nested_envelope_and_axis_headers() {
        let response = error_response(
            SurfaceHttpError::rate_limited(
                SurfaceClientRateLimitAxis::WpUser,
                Duration::from_millis(1_200),
            )
            .with_request_id("req_rate_limited".to_string()),
        );

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "2");
        assert_eq!(
            response.headers()[HeaderName::from_static("x-ratelimit-exhausted-axis")],
            "wp_user"
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "rate_limited");
        assert_eq!(body["error"]["axis"], "wp_user");
        assert_eq!(body["error"]["retry_after_ms"], 1_200);
        assert_eq!(body["error"]["request_id"], "req_rate_limited");
    }

    #[tokio::test]
    async fn handshake_body_reader_rejects_oversized_payload() {
        let body = Full::new(Bytes::from(vec![b'a'; MAX_HANDSHAKE_BODY_BYTES + 1]));
        let error = collect_limited_body(body, MAX_HANDSHAKE_BODY_BYTES)
            .await
            .unwrap_err();
        assert_eq!(error.code, "request_body_too_large");
        let response = error_response(error.with_request_id("req_large".into()));
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "request_body_too_large");
    }

    #[tokio::test]
    async fn endpoint_lifecycle_start_stop_closes_listener() {
        let endpoint = Arc::new(SurfaceEndpointState::default());
        let snapshot = endpoint
            .clone()
            .start(SurfaceEndpointConfig::default())
            .await
            .unwrap();
        assert_eq!(snapshot.availability, SurfaceEndpointAvailability::Running);
        let port = snapshot.bound_port.unwrap();
        let response = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/v1/surface/health"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        endpoint.stop();

        let mut closed = false;
        for _ in 0..20 {
            match tokio::time::timeout(
                Duration::from_millis(50),
                tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port)),
            )
            .await
            {
                Ok(Err(_)) | Err(_) => {
                    closed = true;
                    break;
                }
                Ok(Ok(stream)) => {
                    drop(stream);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            }
        }
        assert!(closed, "surface endpoint listener stayed open after stop");
    }

    #[tokio::test]
    async fn endpoint_stop_closes_keepalive_connections() {
        let endpoint = Arc::new(SurfaceEndpointState::default());
        let snapshot = endpoint
            .clone()
            .start(SurfaceEndpointConfig::default())
            .await
            .unwrap();
        let port = snapshot.bound_port.unwrap();
        let endpoint_for_blocking = Arc::clone(&endpoint);
        tokio::task::spawn_blocking(move || {
            let mut stream = std::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            stream
                .write_all(
                    format!(
                        "GET /v1/surface/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: keep-alive\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .unwrap();
            let mut buf = [0_u8; 512];
            let read = stream.read(&mut buf).unwrap();
            assert!(String::from_utf8_lossy(&buf[..read]).contains("200 OK"));

            endpoint_for_blocking.stop();

            let mut closed = false;
            for _ in 0..20 {
                let _ = stream.write_all(
                    format!(
                        "GET /v1/surface/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: keep-alive\r\n\r\n"
                    )
                    .as_bytes(),
                );
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        closed = true;
                        break;
                    }
                    Ok(_) => std::thread::sleep(Duration::from_millis(20)),
                }
            }
            assert!(closed, "keep-alive connection stayed usable after stop");
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn endpoint_restart_changes_startup_id() {
        let endpoint = Arc::new(SurfaceEndpointState::default());
        let first = endpoint
            .clone()
            .start(SurfaceEndpointConfig::default())
            .await
            .unwrap();
        let second = endpoint
            .clone()
            .start(SurfaceEndpointConfig::default())
            .await
            .unwrap();
        assert_ne!(first.startup_id, second.startup_id);
        endpoint.stop();
    }

    fn body_json(response: Response<ResponseBody>) -> serde_json::Value {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let bytes =
            runtime.block_on(async { response.into_body().collect().await.unwrap().to_bytes() });
        serde_json::from_slice(&bytes).unwrap()
    }

    fn verified_request_for_test() -> hmac::VerifiedSignedRequest {
        hmac::VerifiedSignedRequest {
            session_id: "sess_test_attribution".into(),
            surface_client_id: "surface_client_test_attribution".into(),
            site_binding_digest: "presented_digest_test".into(),
            site_nonce: "presented_nonce_test".into(),
            wp_user_id: 4242,
            wp_user_hash: "wp_user_hash_attribution".into(),
            wp_site_id: "wp_site_test".into(),
            home_url: "https://test.local".into(),
            site_url: "https://test.local".into(),
            wp_install_uuid: "install-uuid-test".into(),
            plugin_instance_uuid: "plugin-uuid-test".into(),
            multisite_blog_id: None,
        }
    }

    #[test]
    fn validation_rejection_events_attribute_surface_client_when_scopes_recovered() {
        let verified = verified_request_for_test();
        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
            .expect("non-empty scope set");
        let events = validation_rejection_events(
            &verified,
            &SurfacePairingError::SiteBindingMismatch,
            Some(&scopes),
        );

        assert!(
            !events.is_empty(),
            "SiteBindingMismatch should emit at least one event"
        );
        for event in &events {
            match &event.actor {
                Actor::SurfaceClient {
                    instance,
                    scopes: actor_scopes,
                } => {
                    assert_eq!(
                        instance.as_str(),
                        verified.surface_client_id.as_str(),
                        "actor_instance preserves verified surface_client_id"
                    );
                    let actor_strs: Vec<String> = actor_scopes
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect();
                    let expected_strs: Vec<String> =
                        scopes.iter().map(|s| s.as_str().to_string()).collect();
                    assert_eq!(
                        actor_strs, expected_strs,
                        "actor scopes match the recovered pairing scopes"
                    );
                }
                other => panic!("expected Actor::SurfaceClient, got {:?}", other),
            }
            assert_eq!(
                event.wp_user_id,
                Some(verified.wp_user_id),
                "wp_user_id preserved from VerifiedSignedRequest"
            );
            assert_eq!(
                event.wp_user_hash.as_deref(),
                Some(verified.wp_user_hash.as_str()),
                "wp_user_hash preserved from VerifiedSignedRequest"
            );
        }
    }

    #[test]
    fn validation_rejection_events_fall_back_to_system_when_scopes_unavailable() {
        let verified = verified_request_for_test();
        let events = validation_rejection_events(
            &verified,
            &SurfacePairingError::UnknownRuntimeAnchor,
            None,
        );

        assert!(!events.is_empty(), "expected at least one event");
        for event in &events {
            assert!(
                matches!(event.actor, Actor::System),
                "fallback to Actor::System when scopes unrecoverable"
            );
            assert!(
                event.wp_user_id.is_none(),
                "no wp_user_id without attribution"
            );
            assert!(
                event.wp_user_hash.is_none(),
                "no wp_user_hash without attribution"
            );
        }
    }
}
