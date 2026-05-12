use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use http::header::{self, HeaderMap};
use http::{Method, StatusCode, Uri};
use parking_lot::{Mutex, RwLock};
use ring::{digest, hkdf, hmac};
use subtle::ConstantTimeEq;
use zeroize::Zeroize;

pub(super) const DAILYOS_HMAC_DOMAIN: &str = "DAILYOS-WP-BRIDGE-HMAC-V1";
const HKDF_INFO: &[u8] = b"dailyos-wp-bridge-v1";
const NONCE_HASH_DOMAIN: &[u8] = b"DAILYOS-WP-BRIDGE-NONCE-V1\n";
const SIGNATURE_PREFIX: &str = "v1=";
const SIGNATURE_BYTES: usize = 32;
const SIGNATURE_HEX_BYTES: usize = SIGNATURE_BYTES * 2;
const MAX_SIGNED_IDENTIFIER_BYTES: usize = 128;

const HEADER_SURFACE_CLIENT: &str = "x-dailyos-surfaceclient";
const HEADER_KEY_ID: &str = "x-dailyos-key-id";
const HEADER_SIGNATURE: &str = "x-dailyos-signature";
const HEADER_TIMESTAMP: &str = "x-dailyos-timestamp";
const HEADER_NONCE: &str = "x-dailyos-nonce";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SignedTransportConfig {
    pub parseable_session_bucket: SignedTokenBucketConfig,
    pub stale_window: Duration,
    pub future_skew: Duration,
    pub cleanup_slack: Duration,
    pub pending_nonce_ttl: Duration,
    pub nonce_records_per_session: usize,
    pub max_active_sessions: usize,
    pub global_nonce_records: usize,
}

impl Default for SignedTransportConfig {
    fn default() -> Self {
        Self {
            parseable_session_bucket: SignedTokenBucketConfig {
                capacity: 10,
                refill_per_second: 120.0 / 60.0,
            },
            stale_window: Duration::from_secs(30),
            future_skew: Duration::from_secs(5),
            cleanup_slack: Duration::from_secs(5),
            pending_nonce_ttl: Duration::from_secs(5),
            nonce_records_per_session: 4096,
            max_active_sessions: 128,
            global_nonce_records: 65_536,
        }
    }
}

impl SignedTransportConfig {
    fn nonce_replay_ttl(&self) -> Duration {
        self.stale_window + self.future_skew + self.cleanup_slack
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SignedTokenBucketConfig {
    pub capacity: u32,
    pub refill_per_second: f64,
}

#[derive(Clone)]
pub(super) struct SignedTransportState {
    inner: Arc<SignedTransportInner>,
}

struct SignedTransportInner {
    config: RwLock<SignedTransportConfig>,
    sessions: RwLock<HashMap<String, SignedSurfaceSession>>,
    nonce_store: Mutex<NonceReplayStore>,
    session_buckets: Mutex<SessionAbuseBuckets>,
}

impl Default for SignedTransportState {
    fn default() -> Self {
        Self {
            inner: Arc::new(SignedTransportInner {
                config: RwLock::new(SignedTransportConfig::default()),
                sessions: RwLock::new(HashMap::new()),
                nonce_store: Mutex::new(NonceReplayStore::default()),
                session_buckets: Mutex::new(SessionAbuseBuckets::default()),
            }),
        }
    }
}

impl SignedTransportState {
    pub(super) fn configure(&self, config: SignedTransportConfig) {
        *self.inner.config.write() = config;
    }

    pub(super) fn clear_runtime_state(&self) {
        self.inner.sessions.write().clear();
        self.inner.nonce_store.lock().records.clear();
        self.inner.session_buckets.lock().buckets.clear();
    }

    pub(super) fn register_session(
        &self,
        session: SignedSurfaceSession,
    ) -> Result<(), SignedTransportError> {
        let config = self.inner.config.read().clone();
        let mut sessions = self.inner.sessions.write();
        if !sessions.contains_key(&session.session_id)
            && sessions.len() >= config.max_active_sessions.max(1)
        {
            return Err(SignedTransportError::transport_abuse_limited(
                "active_session_cap",
            ));
        }
        sessions.insert(session.session_id.clone(), session);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn clear_sessions(&self) {
        self.inner.sessions.write().clear();
        self.inner.nonce_store.lock().records.clear();
        self.inner.session_buckets.lock().buckets.clear();
    }

    pub(super) fn verify(
        &self,
        request: SignedRequest<'_>,
    ) -> Result<VerifiedSignedRequest, SignedTransportError> {
        let config = self.inner.config.read().clone();
        self.apply_parseable_session_bucket(request.headers, &config, request.instant)?;

        let headers = ParsedSigningHeaders::parse(request.headers)?;
        validate_timestamp(headers.timestamp, &config, request.now)?;

        let session = self
            .inner
            .sessions
            .read()
            .get(headers.key_id)
            .cloned()
            .ok_or_else(|| {
                SignedTransportError::key_not_found("session_missing")
                    .with_session_id(headers.key_id)
                    .with_surface_client_id(headers.surface_client_id)
            })?;

        if session.state == SignedSessionState::Rotated {
            return Err(SignedTransportError::key_rotated("rotated")
                .with_session_id(headers.key_id)
                .with_surface_client_id(headers.surface_client_id));
        }
        if session.surface_client_id != headers.surface_client_id {
            return Err(
                SignedTransportError::token_invalid("surface_client_mismatch")
                    .with_session_id(headers.key_id)
                    .with_surface_client_id(headers.surface_client_id),
            );
        }
        if !session.matches_bearer(headers.bearer_token) {
            return Err(SignedTransportError::token_invalid("bearer_mismatch")
                .with_session_id(headers.key_id)
                .with_surface_client_id(headers.surface_client_id));
        }

        let signing_key = session.derive_signing_key();
        let nonce_hash = nonce_hash(&signing_key, headers.nonce);
        self.reserve_nonce(headers.key_id, nonce_hash, &config, request.instant)?;

        let canonical = match canonicalize_signed_request(&request, &headers) {
            Ok(canonical) => canonical,
            Err(error) => {
                self.mark_nonce_consumed(headers.key_id, nonce_hash, &config, request.instant);
                return Err(error
                    .with_session_id(headers.key_id)
                    .with_surface_client_id(headers.surface_client_id));
            }
        };
        let computed = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, &signing_key),
            canonical.as_slice(),
        );
        let comparison = computed.as_ref().ct_eq(headers.signature.as_slice());

        self.mark_nonce_consumed(headers.key_id, nonce_hash, &config, request.instant);
        if !bool::from(comparison) {
            return Err(
                SignedTransportError::signature_invalid("hmac_compare_failed")
                    .with_session_id(headers.key_id)
                    .with_surface_client_id(headers.surface_client_id),
            );
        }

        Ok(VerifiedSignedRequest {
            session_id: headers.key_id.to_string(),
            surface_client_id: headers.surface_client_id.to_string(),
        })
    }

    fn apply_parseable_session_bucket(
        &self,
        headers: &HeaderMap,
        config: &SignedTransportConfig,
        now: Instant,
    ) -> Result<(), SignedTransportError> {
        let Some(session_id) = parseable_active_session_id(headers, &self.inner.sessions.read())
        else {
            return Ok(());
        };

        self.inner
            .session_buckets
            .lock()
            .try_acquire(&session_id, config.parseable_session_bucket, now)
            .map_err(|_| {
                SignedTransportError::transport_abuse_limited("parseable_session_bucket")
                    .with_session_id(&session_id)
            })
    }

    fn reserve_nonce(
        &self,
        session_id: &str,
        nonce_hash: [u8; 32],
        config: &SignedTransportConfig,
        now: Instant,
    ) -> Result<(), SignedTransportError> {
        self.inner
            .nonce_store
            .lock()
            .reserve(session_id, nonce_hash, config, now)
            .map_err(|decision| match decision {
                NonceDecision::Replay => {
                    SignedTransportError::nonce_replay("nonce_seen").with_session_id(session_id)
                }
                NonceDecision::Limited => {
                    SignedTransportError::transport_abuse_limited("nonce_table_cap")
                        .with_session_id(session_id)
                }
            })
    }

    fn mark_nonce_consumed(
        &self,
        session_id: &str,
        nonce_hash: [u8; 32],
        config: &SignedTransportConfig,
        now: Instant,
    ) {
        self.inner
            .nonce_store
            .lock()
            .mark_consumed(session_id, nonce_hash, config, now);
    }
}

fn canonicalize_signed_request(
    request: &SignedRequest<'_>,
    headers: &ParsedSigningHeaders<'_>,
) -> Result<Vec<u8>, SignedTransportError> {
    reject_unsupported_content_encoding(request.headers)?;
    let content_type = content_type_for_canonical_request(request.headers)?;
    canonical_request_bytes(
        request.method,
        request.uri,
        &content_type,
        request.body,
        headers.nonce,
        headers.timestamp_raw,
    )
}

pub(super) struct SignedRequest<'a> {
    pub headers: &'a HeaderMap,
    pub method: &'a Method,
    pub uri: &'a Uri,
    pub body: &'a [u8],
    pub now: DateTime<Utc>,
    pub instant: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VerifiedSignedRequest {
    pub session_id: String,
    pub surface_client_id: String,
}

#[derive(Clone)]
pub(super) struct SignedSurfaceSession {
    session_id: String,
    surface_client_id: String,
    bearer_token_hash: [u8; 32],
    master_key: SecretBytes32,
    state: SignedSessionState,
}

impl fmt::Debug for SignedSurfaceSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedSurfaceSession")
            .field("session_id_hash", &hash_prefix(&self.session_id))
            .field(
                "surface_client_id_hash",
                &hash_prefix(&self.surface_client_id),
            )
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl SignedSurfaceSession {
    pub(super) fn new_active(
        session_id: impl Into<String>,
        surface_client_id: impl Into<String>,
        bearer_token: &str,
        master_key: [u8; 32],
    ) -> Self {
        Self::new(
            session_id,
            surface_client_id,
            bearer_token,
            master_key,
            SignedSessionState::Active,
        )
    }

    #[cfg(test)]
    pub(super) fn new_rotated_for_tests(
        session_id: impl Into<String>,
        surface_client_id: impl Into<String>,
        bearer_token: &str,
        master_key: [u8; 32],
    ) -> Self {
        Self::new(
            session_id,
            surface_client_id,
            bearer_token,
            master_key,
            SignedSessionState::Rotated,
        )
    }

    fn new(
        session_id: impl Into<String>,
        surface_client_id: impl Into<String>,
        bearer_token: &str,
        master_key: [u8; 32],
        state: SignedSessionState,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            surface_client_id: surface_client_id.into(),
            bearer_token_hash: sha256_bytes(bearer_token.as_bytes()),
            master_key: SecretBytes32(master_key),
            state,
        }
    }

    fn matches_bearer(&self, bearer_token: &str) -> bool {
        bool::from(
            self.bearer_token_hash
                .as_slice()
                .ct_eq(sha256_bytes(bearer_token.as_bytes()).as_slice()),
        )
    }

    pub(super) fn derive_signing_key(&self) -> [u8; 32] {
        derive_session_key(self.master_key.0, &self.session_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SignedSessionState {
    Active,
    Rotated,
}

#[derive(Clone)]
struct SecretBytes32([u8; 32]);

impl Drop for SecretBytes32 {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

struct ParsedSigningHeaders<'a> {
    bearer_token: &'a str,
    surface_client_id: &'a str,
    key_id: &'a str,
    signature: [u8; SIGNATURE_BYTES],
    timestamp_raw: &'a str,
    timestamp: DateTime<Utc>,
    nonce: &'a str,
}

impl<'a> ParsedSigningHeaders<'a> {
    fn parse(headers: &'a HeaderMap) -> Result<Self, SignedTransportError> {
        let bearer_token = parse_authorization_bearer(headers)?;
        let surface_client_id = required_identifier_header(headers, HEADER_SURFACE_CLIENT)?;
        let key_id = required_identifier_header(headers, HEADER_KEY_ID)?;
        let signature = parse_signature_header(required_single_header(headers, HEADER_SIGNATURE)?)?;
        let timestamp_raw = required_single_header(headers, HEADER_TIMESTAMP)?;
        let timestamp = parse_timestamp(timestamp_raw)?;
        let nonce = parse_nonce(required_single_header(headers, HEADER_NONCE)?)?;

        Ok(Self {
            bearer_token,
            surface_client_id,
            key_id,
            signature,
            timestamp_raw,
            timestamp,
            nonce,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SignedTransportError {
    pub kind: SignedTransportErrorKind,
    pub reason: &'static str,
    pub session_id_hash: Option<String>,
    pub surface_client_id_hash: Option<String>,
}

impl SignedTransportError {
    fn signature_invalid(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::SignatureInvalid, reason)
    }

    fn canonicalization_mismatch(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::CanonicalizationMismatch, reason)
    }

    fn timestamp_stale(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::TimestampStale, reason)
    }

    fn timestamp_future(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::TimestampFuture, reason)
    }

    fn key_not_found(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::KeyNotFound, reason)
    }

    fn key_rotated(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::KeyRotated, reason)
    }

    fn token_invalid(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::TokenInvalid, reason)
    }

    fn nonce_replay(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::NonceReplay, reason)
    }

    fn transport_abuse_limited(reason: &'static str) -> Self {
        Self::new(SignedTransportErrorKind::TransportAbuseLimited, reason)
    }

    fn new(kind: SignedTransportErrorKind, reason: &'static str) -> Self {
        Self {
            kind,
            reason,
            session_id_hash: None,
            surface_client_id_hash: None,
        }
    }

    fn with_session_id(mut self, session_id: &str) -> Self {
        self.session_id_hash = Some(hash_prefix(session_id));
        self
    }

    fn with_surface_client_id(mut self, surface_client_id: &str) -> Self {
        self.surface_client_id_hash = Some(hash_prefix(surface_client_id));
        self
    }

    pub(super) fn status(&self) -> StatusCode {
        match self.kind {
            SignedTransportErrorKind::CanonicalizationMismatch => StatusCode::BAD_REQUEST,
            SignedTransportErrorKind::NonceReplay => StatusCode::CONFLICT,
            SignedTransportErrorKind::TransportAbuseLimited => StatusCode::TOO_MANY_REQUESTS,
            SignedTransportErrorKind::SignatureInvalid
            | SignedTransportErrorKind::TimestampStale
            | SignedTransportErrorKind::TimestampFuture
            | SignedTransportErrorKind::KeyNotFound
            | SignedTransportErrorKind::KeyRotated
            | SignedTransportErrorKind::TokenInvalid => StatusCode::UNAUTHORIZED,
        }
    }

    pub(super) fn code(&self) -> &'static str {
        match self.kind {
            SignedTransportErrorKind::SignatureInvalid => "signature_invalid",
            SignedTransportErrorKind::CanonicalizationMismatch => "canonicalization_mismatch",
            SignedTransportErrorKind::TimestampStale => "timestamp_stale",
            SignedTransportErrorKind::TimestampFuture => "timestamp_future",
            SignedTransportErrorKind::KeyNotFound => "key_not_found",
            SignedTransportErrorKind::KeyRotated => "key_rotated",
            SignedTransportErrorKind::TokenInvalid => "token_invalid",
            SignedTransportErrorKind::NonceReplay => "nonce_replay",
            SignedTransportErrorKind::TransportAbuseLimited => "transport_abuse_limited",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SignedTransportErrorKind {
    SignatureInvalid,
    CanonicalizationMismatch,
    TimestampStale,
    TimestampFuture,
    KeyNotFound,
    KeyRotated,
    TokenInvalid,
    NonceReplay,
    TransportAbuseLimited,
}

pub(super) fn canonical_request_bytes(
    method: &Method,
    uri: &Uri,
    content_type: &str,
    body: &[u8],
    nonce: &str,
    timestamp: &str,
) -> Result<Vec<u8>, SignedTransportError> {
    if !method.as_str().bytes().all(|byte| byte.is_ascii()) {
        return Err(SignedTransportError::canonicalization_mismatch(
            "method_non_ascii",
        ));
    }
    let method = method.as_str().to_ascii_uppercase();
    let path_query = uri
        .path_and_query()
        .map(|path_query| path_query.as_str())
        .unwrap_or_else(|| uri.path());

    let mut bytes = Vec::new();
    bytes.extend_from_slice(DAILYOS_HMAC_DOMAIN.as_bytes());
    bytes.push(b'\n');
    append_canonical_field(&mut bytes, "method", method.as_bytes());
    append_canonical_field(&mut bytes, "path_query", path_query.as_bytes());
    append_canonical_field(&mut bytes, "content_type", content_type.as_bytes());
    append_canonical_field(&mut bytes, "body", body);
    append_canonical_field(&mut bytes, "nonce", nonce.as_bytes());
    append_canonical_field(&mut bytes, "timestamp", timestamp.as_bytes());
    Ok(bytes)
}

pub(super) fn derive_session_key(master_key: [u8; 32], session_id: &str) -> [u8; 32] {
    struct SessionKeyLen;
    impl hkdf::KeyType for SessionKeyLen {
        fn len(&self) -> usize {
            SIGNATURE_BYTES
        }
    }

    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, session_id.as_bytes());
    let prk = salt.extract(master_key.as_slice());
    let okm = prk
        .expand(&[HKDF_INFO], SessionKeyLen)
        .expect("fixed HKDF output length is valid");
    let mut key = [0_u8; SIGNATURE_BYTES];
    okm.fill(&mut key)
        .expect("fixed HKDF output buffer has the advertised length");
    key
}

#[cfg(test)]
pub(super) fn sign_request_for_tests(
    master_key: [u8; 32],
    session_id: &str,
    method: &Method,
    uri: &Uri,
    content_type: &str,
    body: &[u8],
    nonce: &str,
    timestamp: &str,
) -> String {
    let key = derive_session_key(master_key, session_id);
    let canonical =
        canonical_request_bytes(method, uri, content_type, body, nonce, timestamp).unwrap();
    let signature = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, &key), &canonical);
    hex::encode(signature.as_ref())
}

fn append_canonical_field(bytes: &mut Vec<u8>, label: &str, value: &[u8]) {
    bytes.extend_from_slice(label.as_bytes());
    bytes.push(b':');
    bytes.extend_from_slice(value.len().to_string().as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(value);
    bytes.push(b'\n');
}

fn reject_unsupported_content_encoding(headers: &HeaderMap) -> Result<(), SignedTransportError> {
    for value in headers.get_all(header::CONTENT_ENCODING) {
        let Ok(value) = value.to_str() else {
            return Err(SignedTransportError::canonicalization_mismatch(
                "content_encoding_non_utf8",
            ));
        };
        if !value.trim().eq_ignore_ascii_case("identity") {
            return Err(SignedTransportError::canonicalization_mismatch(
                "content_encoding_unsupported",
            ));
        }
    }
    Ok(())
}

fn content_type_for_canonical_request(headers: &HeaderMap) -> Result<String, SignedTransportError> {
    let mut values = headers.get_all(header::CONTENT_TYPE).iter();
    let Some(value) = values.next() else {
        return Ok(String::new());
    };
    if values.next().is_some() {
        return Err(SignedTransportError::canonicalization_mismatch(
            "content_type_multiple",
        ));
    }
    let value = value
        .to_str()
        .map_err(|_| SignedTransportError::canonicalization_mismatch("content_type_non_utf8"))?;
    Ok(trim_ascii_whitespace(value).to_string())
}

fn parse_authorization_bearer(headers: &HeaderMap) -> Result<&str, SignedTransportError> {
    let value = required_single_header(headers, header::AUTHORIZATION.as_str())
        .map_err(|_| SignedTransportError::token_invalid("authorization_missing"))?;
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(SignedTransportError::token_invalid(
            "authorization_malformed",
        ));
    };
    if token.is_empty() || token.len() > 512 || token.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(SignedTransportError::token_invalid(
            "authorization_malformed",
        ));
    }
    Ok(token)
}

fn required_identifier_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, SignedTransportError> {
    let value = required_single_header(headers, name)?;
    if !is_safe_identifier(value) {
        return Err(SignedTransportError::signature_invalid(
            "identifier_malformed",
        ));
    }
    Ok(value)
}

fn required_single_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, SignedTransportError> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Err(SignedTransportError::signature_invalid("header_missing"));
    };
    if values.next().is_some() {
        return Err(SignedTransportError::signature_invalid("header_multiple"));
    }
    value
        .to_str()
        .map_err(|_| SignedTransportError::signature_invalid("header_non_utf8"))
}

fn parse_signature_header(value: &str) -> Result<[u8; SIGNATURE_BYTES], SignedTransportError> {
    let Some(hex_signature) = value.strip_prefix(SIGNATURE_PREFIX) else {
        return Err(SignedTransportError::signature_invalid(
            "signature_prefix_malformed",
        ));
    };
    if hex_signature.len() != SIGNATURE_HEX_BYTES
        || !hex_signature
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(SignedTransportError::signature_invalid(
            "signature_hex_malformed",
        ));
    }
    let bytes = hex::decode(hex_signature)
        .map_err(|_| SignedTransportError::signature_invalid("signature_hex_malformed"))?;
    let mut signature = [0_u8; SIGNATURE_BYTES];
    signature.copy_from_slice(&bytes);
    Ok(signature)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, SignedTransportError> {
    if !value.ends_with('Z') {
        return Err(SignedTransportError::signature_invalid(
            "timestamp_not_utc_z",
        ));
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|_| SignedTransportError::signature_invalid("timestamp_malformed"))
}

fn validate_timestamp(
    timestamp: DateTime<Utc>,
    config: &SignedTransportConfig,
    now: DateTime<Utc>,
) -> Result<(), SignedTransportError> {
    if timestamp <= now {
        let age = (now - timestamp).to_std().unwrap_or(Duration::MAX);
        if age > config.stale_window {
            return Err(SignedTransportError::timestamp_stale(
                "timestamp_age_exceeded",
            ));
        }
        return Ok(());
    }

    let future = (timestamp - now).to_std().unwrap_or(Duration::MAX);
    if future > config.future_skew {
        return Err(SignedTransportError::timestamp_future(
            "timestamp_future_exceeded",
        ));
    }
    Ok(())
}

fn parse_nonce(value: &str) -> Result<&str, SignedTransportError> {
    if value.is_empty()
        || value.len() > 256
        || !(is_lowercase_hex_nonce(value) || is_base64url_nonce(value))
    {
        return Err(SignedTransportError::signature_invalid("nonce_malformed"));
    }
    Ok(value)
}

fn is_lowercase_hex_nonce(value: &str) -> bool {
    value.len() >= 32
        && value.len().is_multiple_of(2)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_base64url_nonce(value: &str) -> bool {
    value.len() >= 22
        && !value.contains('=')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SIGNED_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn trim_ascii_whitespace(value: &str) -> &str {
    value.trim_matches(|character: char| character.is_ascii_whitespace())
}

fn parseable_active_session_id(
    headers: &HeaderMap,
    sessions: &HashMap<String, SignedSurfaceSession>,
) -> Option<String> {
    if let Some(key_id) = optional_safe_single_header(headers, HEADER_KEY_ID) {
        if sessions
            .get(key_id)
            .is_some_and(|session| session.state == SignedSessionState::Active)
        {
            return Some(key_id.to_string());
        }
    }

    let surface_client_id = optional_safe_single_header(headers, HEADER_SURFACE_CLIENT)?;
    sessions
        .values()
        .find(|session| {
            session.state == SignedSessionState::Active
                && session.surface_client_id == surface_client_id
        })
        .map(|session| session.session_id.clone())
}

fn optional_safe_single_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    let value = value.to_str().ok()?;
    is_safe_identifier(value).then_some(value)
}

fn nonce_hash(signing_key: &[u8; 32], nonce: &str) -> [u8; 32] {
    let key = hmac::Key::new(hmac::HMAC_SHA256, signing_key);
    let mut context = hmac::Context::with_key(&key);
    context.update(NONCE_HASH_DOMAIN);
    context.update(nonce.as_bytes());
    let tag = context.sign();
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(tag.as_ref());
    hash
}

fn sha256_bytes(value: &[u8]) -> [u8; 32] {
    let digest = digest::digest(&digest::SHA256, value);
    let mut bytes = [0_u8; 32];
    bytes.copy_from_slice(digest.as_ref());
    bytes
}

pub(super) fn hash_prefix(value: &str) -> String {
    let digest = digest::digest(&digest::SHA256, value.as_bytes());
    hex::encode(&digest.as_ref()[..8])
}

#[derive(Default)]
struct SessionAbuseBuckets {
    buckets: HashMap<String, SignedTokenBucket>,
}

impl SessionAbuseBuckets {
    fn try_acquire(
        &mut self,
        session_id: &str,
        config: SignedTokenBucketConfig,
        now: Instant,
    ) -> Result<(), Duration> {
        self.buckets
            .entry(session_id.to_string())
            .or_insert_with(|| SignedTokenBucket::new(config))
            .try_acquire(now)
    }
}

#[derive(Clone, Debug)]
struct SignedTokenBucket {
    config: SignedTokenBucketConfig,
    tokens: f64,
    last_refill: Instant,
}

impl SignedTokenBucket {
    fn new(config: SignedTokenBucketConfig) -> Self {
        Self {
            tokens: f64::from(config.capacity.max(1)),
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
        self.tokens = (self.tokens + refill).min(f64::from(self.config.capacity.max(1)));
        self.last_refill = now;
    }
}

#[derive(Default)]
struct NonceReplayStore {
    records: HashMap<NonceKey, NonceRecord>,
}

impl NonceReplayStore {
    fn reserve(
        &mut self,
        session_id: &str,
        nonce_hash: [u8; 32],
        config: &SignedTransportConfig,
        now: Instant,
    ) -> Result<(), NonceDecision> {
        self.prune_expired(now);
        let key = NonceKey::new(session_id, nonce_hash);
        if self.records.contains_key(&key) {
            return Err(NonceDecision::Replay);
        }
        if self.would_exceed_caps(session_id, config) {
            self.prune_expired(now);
            if self.would_exceed_caps(session_id, config) {
                return Err(NonceDecision::Limited);
            }
        }
        self.records.insert(
            key,
            NonceRecord {
                state: NonceState::Pending,
                expires_at: now + config.pending_nonce_ttl,
            },
        );
        Ok(())
    }

    fn mark_consumed(
        &mut self,
        session_id: &str,
        nonce_hash: [u8; 32],
        config: &SignedTransportConfig,
        now: Instant,
    ) {
        let key = NonceKey::new(session_id, nonce_hash);
        if let Some(record) = self.records.get_mut(&key) {
            record.state = NonceState::Consumed;
            record.expires_at = now + config.nonce_replay_ttl();
        }
    }

    fn prune_expired(&mut self, now: Instant) {
        let mut expired_pending = 0_usize;
        let mut expired_consumed = 0_usize;
        self.records.retain(|_, record| {
            if record.expires_at > now {
                return true;
            }
            match record.state {
                NonceState::Pending => expired_pending += 1,
                NonceState::Consumed => expired_consumed += 1,
            }
            false
        });
        if expired_pending > 0 {
            log::warn!(
                "dailyos.wp_bridge.signing nonce_prune expired_pending={} expired_consumed={}",
                expired_pending,
                expired_consumed
            );
        }
    }

    fn would_exceed_caps(&self, session_id: &str, config: &SignedTransportConfig) -> bool {
        let nonce_records_per_session = config.nonce_records_per_session.max(1);
        let max_active_sessions = config.max_active_sessions.max(1);
        let global_nonce_records = config.global_nonce_records.max(1);

        if self.records.len() >= global_nonce_records {
            return true;
        }

        let mut session_count = 0_usize;
        let mut known_sessions = HashSet::new();
        let mut records_for_session = 0_usize;
        for key in self.records.keys() {
            if known_sessions.insert(key.session_id.clone()) {
                session_count += 1;
            }
            if key.session_id == session_id {
                records_for_session += 1;
            }
        }
        if !known_sessions.contains(session_id) {
            session_count += 1;
        }

        session_count > max_active_sessions || records_for_session >= nonce_records_per_session
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NonceKey {
    session_id: String,
    nonce_hash: [u8; 32],
}

impl NonceKey {
    fn new(session_id: &str, nonce_hash: [u8; 32]) -> Self {
        Self {
            session_id: session_id.to_string(),
            nonce_hash,
        }
    }
}

#[derive(Clone, Debug)]
struct NonceRecord {
    state: NonceState,
    expires_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonceState {
    Pending,
    Consumed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonceDecision {
    Replay,
    Limited,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use http::HeaderValue;
    use std::sync::{Arc, Barrier};
    use std::thread;

    const SESSION_ID: &str = "sess_test_1234567890";
    const SURFACE_CLIENT_ID: &str = "surface_client_test";
    const BEARER_TOKEN: &str = "bearer_token_test";
    const BODY: &[u8] = br#"{"depth":"standard"}"#;
    const TIMESTAMP: &str = "2026-05-10T17:20:31Z";
    const NONCE: &str = "0123456789abcdef0123456789abcdef";

    fn master_key() -> [u8; 32] {
        [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
            0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b,
            0x2c, 0x2d, 0x2e, 0x2f,
        ]
    }

    fn alternate_master_key() -> [u8; 32] {
        [
            0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
            0xfe, 0xff, 0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb,
            0xec, 0xed, 0xee, 0xef,
        ]
    }

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 10, 17, 20, 32).unwrap()
    }

    fn state_with_session() -> SignedTransportState {
        let state = SignedTransportState::default();
        state
            .register_session(SignedSurfaceSession::new_active(
                SESSION_ID,
                SURFACE_CLIENT_ID,
                BEARER_TOKEN,
                master_key(),
            ))
            .unwrap();
        state
    }

    fn signed_headers(method: &Method, uri: &Uri, body: &[u8], nonce: &str) -> HeaderMap {
        let signature = sign_request_for_tests(
            master_key(),
            SESSION_ID,
            method,
            uri,
            "application/json",
            body,
            nonce,
            TIMESTAMP,
        );
        headers_with_signature(&format!("v1={signature}"), nonce, TIMESTAMP)
    }

    fn headers_with_signature(signature: &str, nonce: &str, timestamp: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer bearer_token_test"),
        );
        headers.insert(
            HEADER_SURFACE_CLIENT,
            HeaderValue::from_static(SURFACE_CLIENT_ID),
        );
        headers.insert(HEADER_KEY_ID, HeaderValue::from_static(SESSION_ID));
        headers.insert(HEADER_SIGNATURE, HeaderValue::from_str(signature).unwrap());
        headers.insert(HEADER_TIMESTAMP, HeaderValue::from_str(timestamp).unwrap());
        headers.insert(HEADER_NONCE, HeaderValue::from_str(nonce).unwrap());
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers
    }

    fn verify(
        state: &SignedTransportState,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<VerifiedSignedRequest, SignedTransportError> {
        state.verify(SignedRequest {
            headers,
            method,
            uri,
            body,
            now: fixed_now(),
            instant: Instant::now(),
        })
    }

    #[test]
    fn canonical_request_matches_hmac_v1_fixture_vector() {
        let method = Method::POST;
        let uri = "/v1/surface/invoke?ability=briefing.daily&ability=briefing.daily"
            .parse::<Uri>()
            .unwrap();
        let canonical =
            canonical_request_bytes(&method, &uri, "application/json", BODY, NONCE, TIMESTAMP)
                .unwrap();
        let expected = concat!(
            "DAILYOS-WP-BRIDGE-HMAC-V1\n",
            "method:4\n",
            "POST\n",
            "path_query:64\n",
            "/v1/surface/invoke?ability=briefing.daily&ability=briefing.daily\n",
            "content_type:16\n",
            "application/json\n",
            "body:20\n",
            "{\"depth\":\"standard\"}\n",
            "nonce:32\n",
            "0123456789abcdef0123456789abcdef\n",
            "timestamp:20\n",
            "2026-05-10T17:20:31Z\n",
        );
        assert_eq!(canonical, expected.as_bytes());
        assert_eq!(
            hex::encode(&canonical),
            "4441494c594f532d57502d4252494447452d484d41432d56310a6d6574686f643a340a504f53540a706174685f71756572793a36340a2f76312f737572666163652f696e766f6b653f6162696c6974793d6272696566696e672e6461696c79266162696c6974793d6272696566696e672e6461696c790a636f6e74656e745f747970653a31360a6170706c69636174696f6e2f6a736f6e0a626f64793a32300a7b226465707468223a227374616e64617264227d0a6e6f6e63653a33320a30313233343536373839616263646566303132333435363738396162636465660a74696d657374616d703a32300a323032362d30352d31305431373a32303a33315a0a"
        );
        assert_eq!(
            hex::encode(derive_session_key(master_key(), SESSION_ID)),
            "0351c2c90ac640592fc5c96a9054a37c70da407a9942f525361743fcad0cbf0f"
        );
        assert_eq!(
            sign_request_for_tests(
                master_key(),
                SESSION_ID,
                &method,
                &uri,
                "application/json",
                BODY,
                NONCE,
                TIMESTAMP,
            ),
            "6d1929abb9b0300a1de1878fa68cd30327df80c84f5f0e436fe130046507ed5c"
        );
    }

    #[test]
    fn canonical_request_preserves_empty_body_and_duplicate_query_order() {
        let method = Method::GET;
        let uri = "/v1/surface/abilities?a=1&a=2".parse::<Uri>().unwrap();
        let canonical = canonical_request_bytes(&method, &uri, "", b"", NONCE, TIMESTAMP).unwrap();
        let rendered = String::from_utf8(canonical).unwrap();
        assert!(rendered.contains("path_query:29\n/v1/surface/abilities?a=1&a=2\n"));
        assert!(rendered.contains("content_type:0\n\n"));
        assert!(rendered.contains("body:0\n\n"));
    }

    #[test]
    fn valid_signature_accepts_and_binds_session_client() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke?ability=briefing.daily"
            .parse::<Uri>()
            .unwrap();
        let headers = signed_headers(&method, &uri, BODY, NONCE);
        let verified = verify(&state, &method, &uri, &headers, BODY).unwrap();
        assert_eq!(verified.session_id, SESSION_ID);
        assert_eq!(verified.surface_client_id, SURFACE_CLIENT_ID);
    }

    #[test]
    fn uppercase_hex_and_base64_signatures_are_rejected() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let headers = signed_headers(&method, &uri, BODY, NONCE);
        let uppercase = headers
            .get(HEADER_SIGNATURE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_uppercase();
        let uppercase_headers = headers_with_signature(&uppercase, NONCE, TIMESTAMP);
        let error = verify(&state, &method, &uri, &uppercase_headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::SignatureInvalid);

        let base64_headers = headers_with_signature("v1=YWJjZA", NONCE, TIMESTAMP);
        let error = verify(&state, &method, &uri, &base64_headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::SignatureInvalid);
    }

    #[test]
    fn mismatched_session_key_rejects_signature() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let signature = sign_request_for_tests(
            alternate_master_key(),
            SESSION_ID,
            &method,
            &uri,
            "application/json",
            BODY,
            NONCE,
            TIMESTAMP,
        );
        let headers = headers_with_signature(&format!("v1={signature}"), NONCE, TIMESTAMP);

        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::SignatureInvalid);
    }

    #[test]
    fn tampered_method_body_path_query_and_content_type_reject() {
        let state = state_with_session();
        let method = Method::POST;
        let signed_uri = "/v1/surface/invoke?a=1&a=2".parse::<Uri>().unwrap();
        let headers = signed_headers(&method, &signed_uri, BODY, NONCE);

        let tampered_body = verify(
            &state,
            &method,
            &signed_uri,
            &headers,
            br#"{"depth":"deep"}"#,
        )
        .unwrap_err();
        assert_eq!(
            tampered_body.kind,
            SignedTransportErrorKind::SignatureInvalid
        );

        let state = state_with_session();
        let method_error = verify(&state, &Method::GET, &signed_uri, &headers, BODY).unwrap_err();
        assert_eq!(
            method_error.kind,
            SignedTransportErrorKind::SignatureInvalid
        );

        let state = state_with_session();
        let sent_uri = "/v1/surface/feedback?a=1&a=2".parse::<Uri>().unwrap();
        let path_error = verify(&state, &method, &sent_uri, &headers, BODY).unwrap_err();
        assert_eq!(path_error.kind, SignedTransportErrorKind::SignatureInvalid);

        let state = state_with_session();
        let sent_uri = "/v1/surface/invoke?a=2&a=1".parse::<Uri>().unwrap();
        let query_error = verify(&state, &method, &sent_uri, &headers, BODY).unwrap_err();
        assert_eq!(query_error.kind, SignedTransportErrorKind::SignatureInvalid);

        let state = state_with_session();
        let mut content_type_headers = headers;
        content_type_headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("Application/JSON"),
        );
        let content_type_error =
            verify(&state, &method, &signed_uri, &content_type_headers, BODY).unwrap_err();
        assert_eq!(
            content_type_error.kind,
            SignedTransportErrorKind::SignatureInvalid
        );
    }

    #[test]
    fn timestamp_thresholds_are_config_driven() {
        let state = state_with_session();
        state.configure(SignedTransportConfig {
            stale_window: Duration::from_secs(10),
            future_skew: Duration::from_secs(2),
            ..SignedTransportConfig::default()
        });
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();

        let stale = "2026-05-10T17:20:21Z";
        let stale_signature = sign_request_for_tests(
            master_key(),
            SESSION_ID,
            &method,
            &uri,
            "application/json",
            BODY,
            NONCE,
            stale,
        );
        let stale_headers = headers_with_signature(&format!("v1={stale_signature}"), NONCE, stale);
        let stale_error = verify(&state, &method, &uri, &stale_headers, BODY).unwrap_err();
        assert_eq!(stale_error.kind, SignedTransportErrorKind::TimestampStale);

        let future = "2026-05-10T17:20:35Z";
        let future_signature = sign_request_for_tests(
            master_key(),
            SESSION_ID,
            &method,
            &uri,
            "application/json",
            BODY,
            "1123456789abcdef0123456789abcdef",
            future,
        );
        let future_headers = headers_with_signature(
            &format!("v1={future_signature}"),
            "1123456789abcdef0123456789abcdef",
            future,
        );
        let future_error = verify(&state, &method, &uri, &future_headers, BODY).unwrap_err();
        assert_eq!(future_error.kind, SignedTransportErrorKind::TimestampFuture);
    }

    #[test]
    fn replay_and_invalid_hmac_consume_nonce() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let headers = signed_headers(&method, &uri, BODY, NONCE);
        verify(&state, &method, &uri, &headers, BODY).unwrap();
        let replay = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(replay.kind, SignedTransportErrorKind::NonceReplay);
        assert_eq!(replay.status(), StatusCode::CONFLICT);

        let state = state_with_session();
        let bad_headers = headers_with_signature(
            "v1=0000000000000000000000000000000000000000000000000000000000000000",
            "1123456789abcdef0123456789abcdef",
            TIMESTAMP,
        );
        let first = verify(&state, &method, &uri, &bad_headers, BODY).unwrap_err();
        assert_eq!(first.kind, SignedTransportErrorKind::SignatureInvalid);
        let corrected_signature = sign_request_for_tests(
            master_key(),
            SESSION_ID,
            &method,
            &uri,
            "application/json",
            BODY,
            "1123456789abcdef0123456789abcdef",
            TIMESTAMP,
        );
        let corrected_headers = headers_with_signature(
            &format!("v1={corrected_signature}"),
            "1123456789abcdef0123456789abcdef",
            TIMESTAMP,
        );
        let replay = verify(&state, &method, &uri, &corrected_headers, BODY).unwrap_err();
        assert_eq!(replay.kind, SignedTransportErrorKind::NonceReplay);
    }

    #[test]
    fn malformed_headers_reject_before_content_encoding_canonicalization() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));

        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::TokenInvalid);
    }

    #[test]
    fn canonicalization_mismatch_after_nonce_reservation_consumes_nonce() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let headers = signed_headers(&method, &uri, BODY, NONCE);
        let mut duplicate_content_type_headers = headers.clone();
        duplicate_content_type_headers.append(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/vnd.dailyos+json"),
        );

        let first =
            verify(&state, &method, &uri, &duplicate_content_type_headers, BODY).unwrap_err();
        assert_eq!(
            first.kind,
            SignedTransportErrorKind::CanonicalizationMismatch
        );
        let replay = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(replay.kind, SignedTransportErrorKind::NonceReplay);
    }

    #[test]
    fn session_token_and_rotation_fail_before_nonce_work() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let mut headers = signed_headers(&method, &uri, BODY, NONCE);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer wrong"),
        );
        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::TokenInvalid);

        let state = SignedTransportState::default();
        state
            .register_session(SignedSurfaceSession::new_rotated_for_tests(
                SESSION_ID,
                SURFACE_CLIENT_ID,
                BEARER_TOKEN,
                master_key(),
            ))
            .unwrap();
        let headers = signed_headers(&method, &uri, BODY, "2123456789abcdef0123456789abcdef");
        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::KeyRotated);

        let state = SignedTransportState::default();
        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(error.kind, SignedTransportErrorKind::KeyNotFound);
    }

    #[test]
    fn nonce_caps_and_parseable_session_bucket_return_transport_abuse_limited() {
        let state = state_with_session();
        state.configure(SignedTransportConfig {
            nonce_records_per_session: 1,
            parseable_session_bucket: SignedTokenBucketConfig {
                capacity: 100,
                refill_per_second: 100.0,
            },
            ..SignedTransportConfig::default()
        });
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let first_headers = signed_headers(&method, &uri, BODY, NONCE);
        verify(&state, &method, &uri, &first_headers, BODY).unwrap();
        let second_headers =
            signed_headers(&method, &uri, BODY, "3123456789abcdef0123456789abcdef");
        let capped = verify(&state, &method, &uri, &second_headers, BODY).unwrap_err();
        assert_eq!(capped.kind, SignedTransportErrorKind::TransportAbuseLimited);

        let state = state_with_session();
        state.configure(SignedTransportConfig {
            parseable_session_bucket: SignedTokenBucketConfig {
                capacity: 1,
                refill_per_second: f64::EPSILON,
            },
            ..SignedTransportConfig::default()
        });
        let bad_a = headers_with_signature(
            "v1=0000000000000000000000000000000000000000000000000000000000000000",
            "4123456789abcdef0123456789abcdef",
            TIMESTAMP,
        );
        let first = verify(&state, &method, &uri, &bad_a, BODY).unwrap_err();
        assert_eq!(first.kind, SignedTransportErrorKind::SignatureInvalid);
        let bad_b = headers_with_signature(
            "v1=1111111111111111111111111111111111111111111111111111111111111111",
            "5123456789abcdef0123456789abcdef",
            TIMESTAMP,
        );
        let limited = verify(&state, &method, &uri, &bad_b, BODY).unwrap_err();
        assert_eq!(
            limited.kind,
            SignedTransportErrorKind::TransportAbuseLimited
        );
    }

    #[test]
    fn racing_duplicate_nonce_allows_exactly_one_request() {
        let state = state_with_session();
        state.configure(SignedTransportConfig {
            parseable_session_bucket: SignedTokenBucketConfig {
                capacity: 16,
                refill_per_second: 16.0,
            },
            ..SignedTransportConfig::default()
        });
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let headers = signed_headers(&method, &uri, BODY, NONCE);
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let state = state.clone();
            let headers = headers.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let method = Method::POST;
                let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
                barrier.wait();
                verify(&state, &method, &uri, &headers, BODY).map(|_| ())
            }));
        }

        let mut accepted = 0;
        let mut replays = 0;
        for handle in handles {
            match handle.join().unwrap() {
                Ok(()) => accepted += 1,
                Err(error) if error.kind == SignedTransportErrorKind::NonceReplay => {
                    replays += 1;
                }
                Err(error) => panic!("unexpected duplicate nonce result: {error:?}"),
            }
        }
        assert_eq!(accepted, 1);
        assert_eq!(replays, 7);
    }

    #[test]
    fn non_identity_content_encoding_rejects_signed_request() {
        let state = state_with_session();
        let method = Method::POST;
        let uri = "/v1/surface/invoke".parse::<Uri>().unwrap();
        let mut headers = signed_headers(&method, &uri, BODY, NONCE);
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        let error = verify(&state, &method, &uri, &headers, BODY).unwrap_err();
        assert_eq!(
            error.kind,
            SignedTransportErrorKind::CanonicalizationMismatch
        );
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    }
}
