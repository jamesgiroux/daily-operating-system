use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::net::{Ipv4Addr, SocketAddr};
use std::panic;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::header::{self, HeaderMap, HeaderValue};
use http::{Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::{AbortHandle, JoinHandle, JoinSet};
use uuid::Uuid;

use crate::state::AppState;

pub const SURFACE_ENDPOINT_VERSION: &str = "v1";
const DEFAULT_MAX_BIND_ATTEMPTS: u16 = 10;
const DEFAULT_LOOPBACK_REQUESTS_PER_MINUTE: u32 = 60;
const DEFAULT_LOOPBACK_BURST_PER_SECOND: u32 = 10;
const DEFAULT_PAIRING_CODE_FAILED_ATTEMPTS: u32 = 5;
const MAX_HANDSHAKE_BODY_BYTES: usize = 4 * 1024;

type ResponseBody = Full<Bytes>;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceEndpointConfig {
    pub max_bind_attempts: u16,
    pub loopback_abuse: TokenBucketConfig,
    pub pairing_attempts: PairingAttemptConfig,
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
    paired_site_origin: Arc<RwLock<Option<String>>>,
    pairing_attempts: Arc<Mutex<PairingAttemptLimiter>>,
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

    #[cfg(test)]
    pub async fn start(
        self: Arc<Self>,
        config: SurfaceEndpointConfig,
    ) -> Result<SurfaceEndpointSnapshot, SurfaceEndpointStartError> {
        let (snapshot, _listener) = self.start_listener(config).await?;
        Ok(snapshot)
    }

    async fn start_listener(
        self: Arc<Self>,
        config: SurfaceEndpointConfig,
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
        *self.paired_site_origin.write() = None;

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
                        bound_port,
                        loopback_bucket: Mutex::new(TokenBucket::new(config.loopback_abuse)),
                        pairing_attempts: Arc::clone(&self.pairing_attempts),
                        paired_site_origin: Arc::clone(&self.paired_site_origin),
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
    ) -> Result<(), SurfaceEndpointStartError> {
        let (snapshot, listener) = self.clone().start_listener(config).await?;
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
        *self.paired_site_origin.write() = site_url.and_then(normalize_origin);
    }

    fn clear_pairing_state(&self) {
        self.pairing_attempts.lock().attempts_by_code.clear();
        *self.paired_site_origin.write() = None;
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
        .run_until_stopped(config)
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
        }
    }
}

struct EndpointRuntime {
    bound_port: u16,
    loopback_bucket: Mutex<TokenBucket>,
    pairing_attempts: Arc<Mutex<PairingAttemptLimiter>>,
    paired_site_origin: Arc<RwLock<Option<String>>>,
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
    let transport_check = validate_transport_headers(
        request.headers(),
        runtime.bound_port,
        runtime.paired_site_origin.read().as_deref(),
    );
    if let Err(error) = transport_check {
        return Ok(error_response(error.with_request_id(request_id)));
    }

    let rate_decision = runtime.loopback_bucket.lock().try_acquire(Instant::now());
    if let Err(retry_after) = rate_decision {
        return Ok(error_response(
            SurfaceHttpError::rate_limited(retry_after).with_request_id(request_id),
        ));
    }

    let method = request.method().clone();
    let uri = request.uri().clone();
    let body = if method == Method::POST && uri.path() == "/v1/pairing/handshake" {
        match collect_limited_body(request.into_body(), MAX_HANDSHAKE_BODY_BYTES).await {
            Ok(body) => body,
            Err(error) => {
                return Ok(error_response(error.with_request_id(request_id)));
            }
        }
    } else {
        Bytes::new()
    };

    Ok(dispatch_surface_request(
        SurfaceHttpRequest { method, uri, body },
        runtime,
        request_id,
    ))
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
    body: Bytes,
}

fn dispatch_surface_request(
    request: SurfaceHttpRequest,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
) -> Response<ResponseBody> {
    let path = request.uri.path().to_string();
    match (request.method, path.as_str()) {
        (Method::GET, "/v1/surface/health") => health_response(request_id),
        (Method::POST, "/v1/pairing/handshake") => {
            pairing_handshake_skeleton_response(request.body, runtime, request_id)
        }
        (Method::GET, "/v1/pairing/status")
        | (Method::POST, "/v1/surface/invoke")
        | (Method::POST, "/v1/surface/feedback")
        | (Method::GET, "/v1/surface/abilities")
        | (Method::GET, "/v1/surface/keyring") => {
            error_response(SurfaceHttpError::auth_missing().with_request_id(request_id))
        }
        _ => error_response(SurfaceHttpError::route_not_found().with_request_id(request_id)),
    }
}

fn pairing_handshake_skeleton_response(
    body: Bytes,
    runtime: Arc<EndpointRuntime>,
    request_id: String,
) -> Response<ResponseBody> {
    if let Some(pairing_code) = pairing_code_from_body(&body) {
        if runtime
            .pairing_attempts
            .lock()
            .record_failed_attempt(&pairing_code)
            .is_limited()
        {
            return error_response(
                SurfaceHttpError::rate_limited_without_retry()
                    .with_message("Too many failed pairing attempts.")
                    .with_remediation("Generate a fresh pairing string in DailyOS and retry.")
                    .with_request_id(request_id),
            );
        }
    }

    error_response(SurfaceHttpError::runtime_unavailable().with_request_id(request_id))
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

    fn rate_limited(retry_after: Duration) -> Self {
        let retry_after_ms = retry_after.as_millis().try_into().unwrap_or(u64::MAX);
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "The DailyOS runtime is receiving too many surface requests.",
            "Wait before retrying the request.",
        )
        .with_retry_after_ms(retry_after_ms)
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

    json_response(error.status, body)
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
    paired_site_origin: Option<&str>,
) -> Result<(), SurfaceHttpError> {
    validate_host(headers, bound_port)?;
    validate_origin(headers, paired_site_origin)?;
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
    paired_site_origin: Option<&str>,
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

    match (normalize_origin(origin), paired_site_origin) {
        (Some(origin), Some(allowed)) if origin == allowed => Ok(()),
        _ => Err(SurfaceHttpError::browser_origin_forbidden()),
    }
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
    use std::io::{Read, Write};

    fn runtime_for_tests(port: u16) -> Arc<EndpointRuntime> {
        Arc::new(EndpointRuntime {
            bound_port: port,
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
            paired_site_origin: Arc::new(RwLock::new(None)),
        })
    }

    fn request_for_tests(method: Method, path: &str, body: Bytes) -> SurfaceHttpRequest {
        SurfaceHttpRequest {
            method,
            uri: path.parse::<Uri>().unwrap(),
            body,
        }
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
        let headers = HeaderMap::new();
        assert!(validate_origin(&headers, None).is_ok());

        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static(""));
        assert!(validate_origin(&headers, None).is_ok());

        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static("null"));
        assert!(validate_origin(&headers, None).is_err());

        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("https://subsidiary.com"),
        );
        assert!(validate_origin(&headers, None).is_err());
        assert!(validate_origin(&headers, Some("https://subsidiary.com")).is_ok());
        assert!(validate_origin(&headers, Some("https://parent.com")).is_err());
    }

    #[test]
    fn protected_routes_return_nested_auth_envelope() {
        for (method, path) in [
            (Method::GET, "/v1/pairing/status"),
            (Method::POST, "/v1/surface/invoke"),
            (Method::POST, "/v1/surface/feedback"),
            (Method::GET, "/v1/surface/abilities"),
            (Method::GET, "/v1/surface/keyring"),
        ] {
            let response = dispatch_surface_request(
                request_for_tests(method, path, Bytes::new()),
                runtime_for_tests(49152),
                "req_test".to_string(),
            );
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let body = body_json(response);
            assert_eq!(body["error"]["code"], "auth_missing");
            assert_eq!(body["error"]["request_id"], "req_test");
            assert!(body.get("error").unwrap().get("message").is_some());
            assert!(body.get("error").unwrap().get("remediation").is_some());
        }
    }

    #[test]
    fn health_response_is_low_information() {
        let response = dispatch_surface_request(
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
        assert!(validate_origin(&headers, endpoint.paired_site_origin.read().as_deref()).is_ok());

        endpoint.stop();

        assert!(validate_origin(&headers, endpoint.paired_site_origin.read().as_deref()).is_err());
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
        };
        let endpoint_config = SurfaceEndpointConfig::from(&config);
        assert_eq!(endpoint_config.max_bind_attempts, DEFAULT_MAX_BIND_ATTEMPTS);
        assert_eq!(endpoint_config.loopback_abuse.capacity, 1);
        assert_eq!(endpoint_config.loopback_abuse.refill_per_second, 0.5);
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
    fn handshake_skeleton_uses_pairing_attempt_bucket() {
        let runtime = runtime_for_tests(49152);
        let body = Bytes::from_static(br#"{"pairing_code":"123-456"}"#);
        for attempt in 0..5 {
            let response = dispatch_surface_request(
                request_for_tests(Method::POST, "/v1/pairing/handshake", body.clone()),
                Arc::clone(&runtime),
                format!("req_{attempt}"),
            );
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(body_json(response)["error"]["code"], "runtime_unavailable");
        }
        let response = dispatch_surface_request(
            request_for_tests(Method::POST, "/v1/pairing/handshake", body),
            runtime,
            "req_limited".to_string(),
        );
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body_json(response)["error"]["code"], "rate_limited");
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
}
