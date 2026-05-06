//!  `ServiceContext` substrate per ADR-0104.
//!
//! ## What this module owns
//!
//! - `ExecutionMode { Live | Simulate | Evaluate }` — the mode-routing enum
//!   every service mutation gates against via `ctx.check_mutation_allowed()?`.
//! - `Clock` + `SeededRng` traits — injection seams replacing direct
//!   `Utc::now()` / `rand::thread_rng()` in service + ability code.
//! - `ServiceContext<'a>` — per-call carrier with public read capabilities
//!   (`mode`, `clock`, `rng`, `actor`, `external`) and `pub(in crate::services)`
//!   service-internal fields.
//! - `ExternalClients` — named wrapper struct for `glean` / `slack` /
//!   `gmail` / `redacted`; live in `Live`, replay/fixture in
//!   `Simulate`/`Evaluate`.
//! - `TxCtx<'tx>` — transaction-scoped context. Has no external clients
//!   and no `IntelligenceProvider` per ADR-0104's ban on external/LLM
//!   calls inside transactions.
//! - `ServiceError` — service-layer error surface with
//!   `WriteBlockedByMode(ExecutionMode)` + `NestedTransactionsForbidden`.
//!
//! ## What this module does NOT own
//!
//! - The 228-mutator catalogue (`src-tauri/tests/dos209_mutation_catalog.txt`)
//!   ships alongside the per-mutator `check_mutation_allowed()?` migration.
//! - The `IntelligenceProvider` seam — W2-B /  owns that on
//!   `AbilityContext`, not `ServiceContext`.
//! - DB plumbing — `with_transaction_async` lands in a follow-up phase
//!   once the mutator migration starts.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Utc};
use http::HeaderMap;
use parking_lot::Mutex;
use serde::de::DeserializeOwned;

use crate::db::claims::IntelligenceClaim;
use crate::services::external_replay::{
    AuthScopeId, ExternalReplayFixture, ExternalReplayFixtureMissing, JsonExternalReplayFixture,
    ReplayResponse, RequestKey,
};
use crate::types::EntityContextEntry;

const DEFAULT_EVALUATE_AUTH_SCOPE_ID: &str = "test-tenant-default";

/// Execution mode for ability + service workflows per ADR-0104.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    /// Production runtime — DB writes, signal emissions, external side
    /// effects all execute against live systems.
    Live,
    /// Developer simulation — replay fixtures stand in for external
    /// services; DB writes blocked; signals route to in-memory ring buffer.
    Simulate,
    /// Evaluation harness (ADR-0110) — fixture DB; live writes
    /// + LLM calls structurally forbidden; replay-only providers.
    Evaluate,
}

impl ExecutionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ExecutionMode::Live => "live",
            ExecutionMode::Simulate => "simulate",
            ExecutionMode::Evaluate => "evaluate",
        }
    }

    /// True iff this mode permits live mutations.
    pub fn permits_writes(self) -> bool {
        matches!(self, ExecutionMode::Live)
    }
}

/// Injection seam for wall-clock reads in services / abilities.
///
/// Replaces direct `Utc::now()` / `chrono::Utc::now()` calls so Simulate
/// + Evaluate modes can supply deterministic clocks (per ADR-0104 §3.2).
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Default `Clock` reading the system wall clock. Used by `new_live`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        // dos209-exempt: Live-mode SystemClock by definition reads wall clock.
        Utc::now()
    }
}

/// Fixed clock for tests + Simulate / Evaluate modes.
pub struct FixedClock {
    now: Mutex<DateTime<Utc>>,
}

impl FixedClock {
    pub fn new(at: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(at),
        }
    }

    pub fn advance(&self, delta: chrono::Duration) {
        let mut guard = self.now.lock();
        *guard += delta;
    }

    pub fn set(&self, at: DateTime<Utc>) {
        *self.now.lock() = at;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock()
    }
}

/// Injection seam for non-cryptographic randomness in services / abilities.
///
/// Replaces direct `rand::thread_rng()` / `rand::rng()` calls so Simulate
/// + Evaluate modes can supply deterministic seeded RNGs.
///
/// The trait is dyn-compatible — only object-safe methods. Generic
/// helpers (e.g., `shuffle_in_place<T>`) live as free functions taking
/// `&dyn SeededRng` so the trait stays usable behind a vtable.
pub trait SeededRng: Send + Sync {
    /// Uniform u64.
    fn random_u64(&self) -> u64;
    /// Uniform f64 in [0, 1).
    fn random_f64(&self) -> f64;
}

/// Shuffle a slice in place via a `&dyn SeededRng`. Fisher-Yates over
/// `random_u64`. Lives outside the trait so the trait stays
/// dyn-compatible (generic methods break vtable construction).
pub fn shuffle_in_place<T>(rng: &dyn SeededRng, slice: &mut [T]) {
    for i in (1..slice.len()).rev() {
        let j = (rng.random_u64() % (i as u64 + 1)) as usize;
        slice.swap(i, j);
    }
}

/// System-RNG implementation for `Live` mode. Wraps `rand::random` so
/// production behavior is unchanged.
#[derive(Debug, Default)]
pub struct SystemRng;

impl SeededRng for SystemRng {
    fn random_u64(&self) -> u64 {
        // dos209-exempt: Live-mode SystemRng wraps the system RNG by definition.
        rand::random::<u64>()
    }

    fn random_f64(&self) -> f64 {
        // dos209-exempt: Live-mode SystemRng wraps the system RNG by definition.
        rand::random::<f64>()
    }
}

/// Deterministic seeded RNG for tests + Simulate / Evaluate.
pub struct SeedableRng {
    state: Mutex<u64>,
}

impl SeedableRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: Mutex::new(seed.max(1)),
        }
    }
}

impl SeededRng for SeedableRng {
    fn random_u64(&self) -> u64 {
        // xorshift64* — fast, deterministic, sufficient for non-crypto needs.
        let mut s = self.state.lock();
        let mut x = *s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *s = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn random_f64(&self) -> f64 {
        let n = self.random_u64();
        (n >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

/// External-services wrapper struct. Each field is a thin handle with the
/// service API shape used by mode-aware services. Replay mode is fully
/// wired through fixtures. Live mode intentionally fails closed with
/// `ExternalClientError::LiveNotYetWired` until each service has a typed
/// live adapter exposing the same method surface as its replay handle.
/// Default construction stays live; callers opt into replay with
/// `ExternalClients::from_replay`.
#[derive(Default, Clone)]
pub struct ExternalClients {
    pub glean: GleanClientHandle,
    pub slack: SlackClientHandle,
    pub gmail: GmailClientHandle,
    pub redacted: SalesforceClientHandle,
}

impl ExternalClients {
    pub fn from_replay<T>(fixture: Arc<dyn ExternalReplayFixture>, auth_scope_id: T) -> Self
    where
        T: TryInto<AuthScopeId>,
        T::Error: std::fmt::Display,
    {
        let auth_scope_id = auth_scope_id_or_panic(auth_scope_id);
        Self {
            glean: ReplayGleanClient::new(fixture.clone(), auth_scope_id.clone()).into(),
            slack: ReplaySlackClient::new(fixture.clone(), auth_scope_id.clone()).into(),
            gmail: ReplayGmailClient::new(fixture.clone(), auth_scope_id.clone()).into(),
            redacted: ReplaySalesforceClient::new(fixture, auth_scope_id).into(),
        }
    }

    pub fn is_replay_mode(&self) -> bool {
        self.glean.is_replay()
            && self.slack.is_replay()
            && self.gmail.is_replay()
            && self.redacted.is_replay()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExternalClientError {
    #[error(transparent)]
    ReplayFixtureMissing(#[from] ExternalReplayFixtureMissing),

    #[error("{client} replay response decode failed: {source}")]
    ReplayResponseDecode {
        client: &'static str,
        #[source]
        source: serde_json::Error,
    },

    /// The wrapper's live API contract exists, but the matching typed
    /// adapter has not landed yet. Replay mode remains the executable path.
    #[error("{client} live client is not yet wired")]
    LiveNotYetWired { client: &'static str },
}

/// Mode-aware Glean client wrapper.
///
/// Replay mode is fully wired. Live mode currently reserves the
/// account-facts API shape; its inner slot is opaque because no typed live
/// Glean client in this crate exposes `fetch_account_facts` yet. Live
/// calls return `ExternalClientError::LiveNotYetWired` until that adapter
/// lands.
#[derive(Clone, Default)]
pub struct GleanClientHandle {
    mode: GleanClientMode,
}

impl GleanClientHandle {
    pub fn is_configured(&self) -> bool {
        match &self.mode {
            GleanClientMode::Live(inner) => inner.is_some(),
            GleanClientMode::Replay(_) => true,
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self.mode, GleanClientMode::Live(_))
    }

    pub fn is_replay(&self) -> bool {
        matches!(self.mode, GleanClientMode::Replay(_))
    }

    pub fn fetch_account_facts(
        &self,
        account_id: &str,
    ) -> Result<GleanAccountFacts, ExternalClientError> {
        match &self.mode {
            GleanClientMode::Live(_) => {
                Err(ExternalClientError::LiveNotYetWired { client: "glean" })
            }
            GleanClientMode::Replay(client) => client.fetch_account_facts(account_id),
        }
    }

    pub fn request_key_for_fetch_account_facts(
        account_id: &str,
        auth_scope_id: &str,
    ) -> RequestKey {
        let auth_scope_id = auth_scope_id_or_panic(auth_scope_id);
        replay_request_key(
            "GET",
            &glean_account_facts_url(account_id),
            b"",
            &auth_scope_id,
        )
    }
}

impl From<ReplayGleanClient> for GleanClientHandle {
    fn from(client: ReplayGleanClient) -> Self {
        Self {
            mode: GleanClientMode::Replay(client),
        }
    }
}

#[derive(Clone)]
pub enum GleanClientMode {
    /// Placeholder for a future typed live Glean adapter. `Some` means a
    /// caller supplied a live object, but this wrapper cannot safely call it
    /// until the object exposes `fetch_account_facts`.
    Live(Option<Arc<dyn std::any::Any + Send + Sync>>),
    Replay(ReplayGleanClient),
}

impl Default for GleanClientMode {
    fn default() -> Self {
        Self::Live(None)
    }
}

#[derive(Clone)]
pub struct ReplayGleanClient {
    fixture: Arc<dyn ExternalReplayFixture>,
    auth_scope_id: AuthScopeId,
}

impl ReplayGleanClient {
    pub fn new<T>(fixture: Arc<dyn ExternalReplayFixture>, auth_scope_id: T) -> Self
    where
        T: TryInto<AuthScopeId>,
        T::Error: std::fmt::Display,
    {
        Self {
            fixture,
            auth_scope_id: auth_scope_id_or_panic(auth_scope_id),
        }
    }

    pub fn fetch_account_facts(
        &self,
        account_id: &str,
    ) -> Result<GleanAccountFacts, ExternalClientError> {
        let url = glean_account_facts_url(account_id);
        let key = replay_request_key("GET", &url, b"", &self.auth_scope_id);
        let response = lookup_replay(&self.fixture, &key, "GET", &url)?;
        decode_replay_json("glean", response)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
pub struct GleanAccountFacts {
    pub account_id: String,
    pub facts: Vec<String>,
}

/// Mode-aware Slack client wrapper.
///
/// Replay mode is fully wired. Live mode is a contract placeholder: no
/// typed Slack adapter currently exposes this generic JSON request surface,
/// so live calls fail closed with `ExternalClientError::LiveNotYetWired`.
#[derive(Clone, Default)]
pub struct SlackClientHandle {
    mode: SlackClientMode,
}

impl SlackClientHandle {
    pub fn is_live(&self) -> bool {
        matches!(self.mode, SlackClientMode::Live)
    }

    pub fn is_replay(&self) -> bool {
        matches!(self.mode, SlackClientMode::Replay(_))
    }

    pub fn replay_json<T>(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
    ) -> Result<T, ExternalClientError>
    where
        T: DeserializeOwned,
    {
        match &self.mode {
            SlackClientMode::Live => Err(ExternalClientError::LiveNotYetWired { client: "slack" }),
            SlackClientMode::Replay(client) => client.replay_json(method, url, body),
        }
    }
}

impl From<ReplaySlackClient> for SlackClientHandle {
    fn from(client: ReplaySlackClient) -> Self {
        Self {
            mode: SlackClientMode::Replay(client),
        }
    }
}

#[derive(Clone, Default)]
pub enum SlackClientMode {
    /// Placeholder for a future typed live Slack adapter.
    #[default]
    Live,
    Replay(ReplaySlackClient),
}

#[derive(Clone)]
pub struct ReplaySlackClient {
    fixture: Arc<dyn ExternalReplayFixture>,
    auth_scope_id: AuthScopeId,
}

impl ReplaySlackClient {
    pub fn new<T>(fixture: Arc<dyn ExternalReplayFixture>, auth_scope_id: T) -> Self
    where
        T: TryInto<AuthScopeId>,
        T::Error: std::fmt::Display,
    {
        Self {
            fixture,
            auth_scope_id: auth_scope_id_or_panic(auth_scope_id),
        }
    }

    pub fn replay_json<T>(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
    ) -> Result<T, ExternalClientError>
    where
        T: DeserializeOwned,
    {
        let key = replay_request_key(method, url, body, &self.auth_scope_id);
        let response = lookup_replay(&self.fixture, &key, method, url)?;
        decode_replay_json("slack", response)
    }
}

/// Mode-aware Gmail client wrapper.
///
/// Replay mode is fully wired. Gmail HTTP helpers exist under
/// `crate::google_api`, but there is no typed live adapter matching this
/// generic JSON request surface yet, so live calls fail closed with
/// `ExternalClientError::LiveNotYetWired`.
#[derive(Clone, Default)]
pub struct GmailClientHandle {
    mode: GmailClientMode,
}

impl GmailClientHandle {
    pub fn is_live(&self) -> bool {
        matches!(self.mode, GmailClientMode::Live)
    }

    pub fn is_replay(&self) -> bool {
        matches!(self.mode, GmailClientMode::Replay(_))
    }

    pub fn replay_json<T>(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
    ) -> Result<T, ExternalClientError>
    where
        T: DeserializeOwned,
    {
        match &self.mode {
            GmailClientMode::Live => Err(ExternalClientError::LiveNotYetWired { client: "gmail" }),
            GmailClientMode::Replay(client) => client.replay_json(method, url, body),
        }
    }
}

impl From<ReplayGmailClient> for GmailClientHandle {
    fn from(client: ReplayGmailClient) -> Self {
        Self {
            mode: GmailClientMode::Replay(client),
        }
    }
}

#[derive(Clone, Default)]
pub enum GmailClientMode {
    /// Placeholder for a future typed live Gmail adapter.
    #[default]
    Live,
    Replay(ReplayGmailClient),
}

#[derive(Clone)]
pub struct ReplayGmailClient {
    fixture: Arc<dyn ExternalReplayFixture>,
    auth_scope_id: AuthScopeId,
}

impl ReplayGmailClient {
    pub fn new<T>(fixture: Arc<dyn ExternalReplayFixture>, auth_scope_id: T) -> Self
    where
        T: TryInto<AuthScopeId>,
        T::Error: std::fmt::Display,
    {
        Self {
            fixture,
            auth_scope_id: auth_scope_id_or_panic(auth_scope_id),
        }
    }

    pub fn replay_json<T>(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
    ) -> Result<T, ExternalClientError>
    where
        T: DeserializeOwned,
    {
        let key = replay_request_key(method, url, body, &self.auth_scope_id);
        let response = lookup_replay(&self.fixture, &key, method, url)?;
        decode_replay_json("gmail", response)
    }
}

/// Mode-aware Salesforce client wrapper.
///
/// Replay mode is fully wired. Direct live Salesforce integration has not
/// landed yet, so live calls fail closed with
/// `ExternalClientError::LiveNotYetWired`.
#[derive(Clone, Default)]
pub struct SalesforceClientHandle {
    mode: SalesforceClientMode,
}

impl SalesforceClientHandle {
    pub fn is_live(&self) -> bool {
        matches!(self.mode, SalesforceClientMode::Live)
    }

    pub fn is_replay(&self) -> bool {
        matches!(self.mode, SalesforceClientMode::Replay(_))
    }

    pub fn fetch_account(
        &self,
        account_id: &str,
    ) -> Result<SalesforceAccountRecord, ExternalClientError> {
        match &self.mode {
            SalesforceClientMode::Live => {
                Err(ExternalClientError::LiveNotYetWired { client: "redacted" })
            }
            SalesforceClientMode::Replay(client) => client.fetch_account(account_id),
        }
    }

    pub fn request_key_for_fetch_account(account_id: &str, auth_scope_id: &str) -> RequestKey {
        let auth_scope_id = auth_scope_id_or_panic(auth_scope_id);
        replay_request_key(
            "GET",
            &redacted_account_url(account_id),
            b"",
            &auth_scope_id,
        )
    }
}

impl From<ReplaySalesforceClient> for SalesforceClientHandle {
    fn from(client: ReplaySalesforceClient) -> Self {
        Self {
            mode: SalesforceClientMode::Replay(client),
        }
    }
}

#[derive(Clone, Default)]
pub enum SalesforceClientMode {
    /// Placeholder for a future typed live Salesforce adapter.
    #[default]
    Live,
    Replay(ReplaySalesforceClient),
}

#[derive(Clone)]
pub struct ReplaySalesforceClient {
    fixture: Arc<dyn ExternalReplayFixture>,
    auth_scope_id: AuthScopeId,
}

impl ReplaySalesforceClient {
    pub fn new<T>(fixture: Arc<dyn ExternalReplayFixture>, auth_scope_id: T) -> Self
    where
        T: TryInto<AuthScopeId>,
        T::Error: std::fmt::Display,
    {
        Self {
            fixture,
            auth_scope_id: auth_scope_id_or_panic(auth_scope_id),
        }
    }

    pub fn fetch_account(
        &self,
        account_id: &str,
    ) -> Result<SalesforceAccountRecord, ExternalClientError> {
        let url = redacted_account_url(account_id);
        let key = replay_request_key("GET", &url, b"", &self.auth_scope_id);
        let response = lookup_replay(&self.fixture, &key, "GET", &url)?;
        decode_replay_json("redacted", response)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize)]
pub struct SalesforceAccountRecord {
    pub account_id: String,
    pub account_name: String,
}

fn lookup_replay(
    fixture: &Arc<dyn ExternalReplayFixture>,
    key: &RequestKey,
    method: &str,
    url: &str,
) -> Result<ReplayResponse, ExternalReplayFixtureMissing> {
    fixture.lookup(key, method, url)
}

fn decode_replay_json<T>(
    client: &'static str,
    response: ReplayResponse,
) -> Result<T, ExternalClientError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(&response.body)
        .map_err(|source| ExternalClientError::ReplayResponseDecode { client, source })
}

fn replay_request_key(
    method: &str,
    url: &str,
    body: &[u8],
    auth_scope_id: &AuthScopeId,
) -> RequestKey {
    RequestKey::canonicalize(method, url, &HeaderMap::new(), body, auth_scope_id)
}

fn auth_scope_id_or_panic<T>(auth_scope_id: T) -> AuthScopeId
where
    T: TryInto<AuthScopeId>,
    T::Error: std::fmt::Display,
{
    auth_scope_id
        .try_into()
        .unwrap_or_else(|err| panic!("invalid auth_scope_id: {err}"))
}

fn glean_account_facts_url(account_id: &str) -> String {
    format!(
        "https://glean.example.com/v1/facts?account_id={}",
        url_encode(account_id)
    )
}

fn redacted_account_url(account_id: &str) -> String {
    format!(
        "https://redacted.example.com/v1/accounts/{}",
        url_encode(account_id)
    )
}

fn url_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Service-layer error surface.
///
/// `WriteBlockedByMode` and `NestedTransactionsForbidden` are the
/// mode-boundary errors every public mutator surfaces.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("write blocked by execution mode: {0:?}")]
    WriteBlockedByMode(ExecutionMode),

    #[error("nested transactions forbidden — caller must not invoke with_transaction inside a transaction body")]
    NestedTransactionsForbidden,

    #[error("database error: {0}")]
    Db(String),

    #[error("invariant violation: {0}")]
    Invariant(String),

    #[error("service error: {0}")]
    Other(String),
}

impl From<rusqlite::Error> for ServiceError {
    fn from(e: rusqlite::Error) -> Self {
        ServiceError::Db(e.to_string())
    }
}

impl From<crate::db::types::DbError> for ServiceError {
    fn from(e: crate::db::types::DbError) -> Self {
        ServiceError::Db(e.to_string())
    }
}

#[cfg(feature = "harness-hermetic")]
pub fn validate_harness_hermetic_db_path(db_path: &str) -> Result<(), ServiceError> {
    let db_path = db_path.trim();
    if db_path == ":memory:" {
        return Ok(());
    }

    if db_path.is_empty() {
        return Err(ServiceError::Invariant(
            "harness-hermetic requires a non-empty DB path".to_string(),
        ));
    }

    let path = std::path::Path::new(db_path);
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ServiceError::Invariant(format!(
            "harness-hermetic DB path must not contain '..': {db_path}"
        )));
    }

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("tests").join("fixtures");
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        manifest_dir.join(path)
    };

    if absolute_path.starts_with(&fixtures_dir) {
        Ok(())
    } else {
        Err(ServiceError::Invariant(format!(
            "harness-hermetic DB path must be :memory: or under {}; got {db_path}",
            fixtures_dir.display()
        )))
    }
}

/// Per-call service execution context.
///
/// `mode`, `clock`, `rng`, `actor`, `external` are public read capabilities
/// (ability code may read them). `tx` is `pub(in crate::services)` —
/// service implementation code reads it; ability-facing code does not.
///
/// **Phase contract:** this initial substrate ships the mode/clock/rng
/// seams + `check_mutation_allowed()` gate. The DB / signals / intel-queue
/// handles + `with_transaction_async` primitive land in subsequent phases
/// alongside the per-service mutator migration. Until then, services
/// continue to take their existing `&ActionDb` arguments and pass a
/// `&ServiceContext` as the new first parameter for the gate + clock/rng.
pub struct ServiceContext<'a> {
    pub mode: ExecutionMode,
    pub clock: &'a dyn Clock,
    pub rng: &'a dyn SeededRng,
    pub actor: &'a str,
    pub external: &'a ExternalClients,
    entity_context_reader: Option<Arc<dyn EntityContextReadHandle>>,
    entity_context_claim_reader: Option<Arc<dyn EntityContextClaimReadHandle>>,
    pub(in crate::services) tx: Option<TxHandle>,
}

pub type EntityContextReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<EntityContextEntry>, String>> + Send + 'a>>;

/// Narrow read handle for DOS-218. This keeps the ability on `AbilityContext`
/// while avoiding `AppState` or raw database handles in ability code.
pub trait EntityContextReadHandle: Send + Sync {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a>;
}

pub type EntityContextClaimReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<IntelligenceClaim>, String>> + Send + 'a>>;

/// Claims-backed read handle for DOS-218. Tests can inject this without
/// exposing raw database handles to ability code.
pub trait EntityContextClaimReadHandle: Send + Sync {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a>;
}

/// Transaction handle (private). Becomes a `TxCtx` for closures inside
/// `with_transaction_async` / `with_transaction_sync` (lands later phase).
#[derive(Default)]
pub(in crate::services) struct TxHandle {
    pub(in crate::services) depth: u32,
}

/// Transaction-scoped context exposed to `with_transaction_*` closures.
///
/// Same `mode`/`clock`/`rng` as the parent `ServiceContext` plus a
/// transaction-bound DB cursor (lands in the DB-plumbing phase).
/// **Has no `external` clients and no `IntelligenceProvider`** per
/// ADR-0104's ban on external/LLM calls inside transactions.
pub struct TxCtx<'tx> {
    pub mode: ExecutionMode,
    pub clock: &'tx dyn Clock,
    pub rng: &'tx dyn SeededRng,
    /// Signal emissions made inside the transaction stage here and
    /// flush after commit; rollback discards.
    pub(in crate::services) staged_signals: Mutex<Vec<StagedSignal>>,
}

impl<'tx> TxCtx<'tx> {
    /// Mutation gate inside a transaction. Returns the same
    /// `WriteBlockedByMode` error as `ServiceContext::check_mutation_allowed`
    /// when the parent context was non-Live.
    pub fn check_mutation_allowed(&self) -> Result<(), ServiceError> {
        if self.mode.permits_writes() {
            Ok(())
        } else {
            Err(ServiceError::WriteBlockedByMode(self.mode))
        }
    }
}

/// Signal emission staged inside a transaction — flushed on commit,
/// discarded on rollback. Concrete fields land alongside the DB plumbing.
pub(in crate::services) struct StagedSignal {
    pub entity_type: String,
    pub entity_id: String,
    pub signal_type: String,
    pub source: String,
    pub payload: Option<String>,
    pub confidence: f64,
}

impl<'a> ServiceContext<'a> {
    /// `Live` constructor — production callers (Tauri commands,
    /// background workers) build this from injected clock/rng/external
    /// references. Typical pattern:
    ///
    /// ```ignore
    /// let clock = SystemClock;
    /// let rng = SystemRng;
    /// let ext = state.external_clients();
    /// let ctx = ServiceContext::new_live(&clock, &rng, &ext);
    /// services::accounts::create_account(&ctx, db, ...).await?;
    /// ```
    pub fn new_live(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self {
            mode: ExecutionMode::Live,
            clock,
            rng,
            actor: "system",
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            tx: None,
        }
    }

    /// `Simulate` constructor — replay clients in `external`, fixture
    /// clock, deterministic RNG. DB writes are blocked at the
    /// `check_mutation_allowed` boundary.
    pub fn new_simulate(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self {
            mode: ExecutionMode::Simulate,
            clock,
            rng,
            actor: "system",
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            tx: None,
        }
    }

    /// `Evaluate` constructor — fixture DB only.
    ///
    /// `external` MUST contain replay/fixture client wrappers — Live
    /// wrappers are a programming error in this mode. This constructor
    /// asserts that replay fixtures populate `external` before construction.
    ///
    /// With `harness-hermetic`, the harness runner must call
    /// `validate_harness_hermetic_db_path` before constructing this context.
    /// The runtime replay-mode assertion remains active in all builds.
    pub fn new_evaluate(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        assert!(
            external.is_replay_mode(),
            "Evaluate ServiceContext requires replay-mode ExternalClients"
        );

        Self {
            mode: ExecutionMode::Evaluate,
            clock,
            rng,
            actor: "system",
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            tx: None,
        }
    }

    /// Convenience constructor for trivial Evaluate-mode tests that do not
    /// need fixture-specific external responses.
    pub fn new_evaluate_default(clock: &'a dyn Clock, rng: &'a dyn SeededRng) -> Self {
        Self::new_evaluate(clock, rng, default_evaluate_external_clients())
    }

    /// Override the actor label associated with this service call.
    pub fn with_actor(mut self, actor: &'a str) -> Self {
        self.actor = actor;
        self
    }

    pub fn with_entity_context_reader(mut self, reader: Arc<dyn EntityContextReadHandle>) -> Self {
        self.entity_context_reader = Some(reader);
        self
    }

    pub fn with_entity_context_claim_reader(
        mut self,
        reader: Arc<dyn EntityContextClaimReadHandle>,
    ) -> Self {
        self.entity_context_claim_reader = Some(reader);
        self
    }

    pub async fn read_entity_context_claims(
        &self,
        entity_type: String,
        entity_id: String,
        depth: usize,
    ) -> Result<Vec<IntelligenceClaim>, String> {
        if let Some(reader) = &self.entity_context_claim_reader {
            return reader
                .read_entity_context_claims(entity_type, entity_id, depth)
                .await;
        }

        tokio::task::spawn_blocking(move || {
            let db = crate::db::ActionDb::open()
                .map_err(|error| format!("Database unavailable: {error}"))?;
            crate::services::claims::load_entity_context_claims_active(
                &db,
                &entity_type,
                &entity_id,
                depth,
            )
            .map_err(|error| format!("Entity context claim read failed: {error}"))
        })
        .await
        .map_err(|error| format!("Entity context claim read task failed: {error}"))?
    }

    pub async fn read_entity_context_entries(
        &self,
        entity_type: String,
        entity_id: String,
    ) -> Result<Vec<EntityContextEntry>, String> {
        if let Some(reader) = &self.entity_context_reader {
            return reader
                .read_entity_context_entries(entity_type, entity_id)
                .await;
        }

        tokio::task::spawn_blocking(move || {
            let db = crate::db::ActionDb::open()
                .map_err(|error| format!("Database unavailable: {error}"))?;
            read_entity_context_entries_from_db(&db, &entity_type, &entity_id)
        })
        .await
        .map_err(|error| format!("Entity context read task failed: {error}"))?
    }

    /// Test-only `Live` constructor.
    #[cfg(test)]
    pub fn test_live(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self::new_live(clock, rng, external)
    }

    /// Test-only `Evaluate` constructor.
    #[cfg(test)]
    pub fn test_evaluate(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self::new_evaluate(clock, rng, external)
    }

    /// Mutation gate. **Every public mutation function in `services/`
    /// MUST call this as its first line.** Returns `WriteBlockedByMode`
    /// in non-Live modes; ability-execution boundaries surface this to
    /// the caller as a typed structural rejection (per ADR-0104).
    pub fn check_mutation_allowed(&self) -> Result<(), ServiceError> {
        if self.mode.permits_writes() {
            Ok(())
        } else {
            Err(ServiceError::WriteBlockedByMode(self.mode))
        }
    }
}

impl EntityContextReadHandle for crate::db_service::PooledConnection {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        let reader = self.clone();
        Box::pin(async move {
            let entries = reader
                .call(move |conn| {
                    let db = crate::db::ActionDb::from_conn(conn);
                    read_entity_context_entries_from_db(db, &entity_type, &entity_id)
                        .map_err(rusqlite::Error::InvalidParameterName)
                })
                .await
                .map_err(|error| format!("DB read error: {error}"))?;
            Ok(entries)
        })
    }
}

fn read_entity_context_entries_from_db(
    db: &crate::db::ActionDb,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<EntityContextEntry>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
             FROM entity_context_entries
             WHERE entity_type = ?1 AND entity_id = ?2
             ORDER BY created_at DESC",
        )
        .map_err(|error| format!("Failed to prepare entity context query: {error}"))?;

    let entries = stmt
        .query_map(rusqlite::params![entity_type, entity_id], |row| {
            Ok(EntityContextEntry {
                id: row.get("id")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                title: row.get("title")?,
                content: row.get("content")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(|error| format!("Failed to query entity context entries: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to map entity context entries: {error}"))?;

    Ok(entries)
}

fn default_evaluate_external_clients() -> &'static ExternalClients {
    static DEFAULT_CLIENTS: OnceLock<ExternalClients> = OnceLock::new();

    DEFAULT_CLIENTS.get_or_init(|| {
        let fixture = JsonExternalReplayFixture::from_json_value(
            &serde_json::json!({
                "version": 1,
                "fixtures": [],
            }),
            "default",
        )
        .expect("empty default external replay fixture must load");

        ExternalClients::from_replay(
            Arc::new(fixture),
            DEFAULT_EVALUATE_AUTH_SCOPE_ID.to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::collections::HashMap;

    #[derive(Default)]
    struct StaticReplayFixture {
        responses: HashMap<RequestKey, ReplayResponse>,
    }

    impl StaticReplayFixture {
        fn with_response(mut self, key: RequestKey, body: &[u8]) -> Self {
            self.responses.insert(
                key,
                ReplayResponse {
                    status: 200,
                    headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                    body: body.to_vec(),
                },
            );
            self
        }
    }

    impl ExternalReplayFixture for StaticReplayFixture {
        fn lookup(
            &self,
            key: &RequestKey,
            method: &str,
            url: &str,
        ) -> Result<ReplayResponse, ExternalReplayFixtureMissing> {
            self.responses
                .get(key)
                .cloned()
                .ok_or_else(|| ExternalReplayFixtureMissing::new(key, method, url))
        }
    }

    fn fixture_external() -> ExternalClients {
        ExternalClients::default()
    }
    fn fixture_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap())
    }
    fn fixture_rng() -> SeedableRng {
        SeedableRng::new(42)
    }

    fn replay_external(fixture: StaticReplayFixture, auth_scope_id: &str) -> ExternalClients {
        ExternalClients::from_replay(Arc::new(fixture), auth_scope_id.to_string())
    }

    fn assert_replay_missing(
        err: ExternalClientError,
        expected_key: RequestKey,
        expected_method: &str,
        expected_url: &str,
    ) {
        match err {
            ExternalClientError::ReplayFixtureMissing(missing) => {
                assert_eq!(missing.request_key_hex, expected_key.to_hex());
                assert_eq!(missing.method, expected_method);
                assert_eq!(missing.url_redacted, expected_url);
            }
            other => panic!("expected ReplayFixtureMissing, got {other:?}"),
        }
    }

    fn assert_live_not_yet_wired(err: ExternalClientError, expected_client: &str) {
        match err {
            ExternalClientError::LiveNotYetWired { client } => {
                assert_eq!(client, expected_client);
            }
            other => panic!("expected LiveNotYetWired, got {other:?}"),
        }
    }

    #[test]
    fn execution_mode_permits_writes_only_in_live() {
        assert!(ExecutionMode::Live.permits_writes());
        assert!(!ExecutionMode::Simulate.permits_writes());
        assert!(!ExecutionMode::Evaluate.permits_writes());
    }

    #[test]
    fn execution_mode_as_str_is_stable() {
        assert_eq!(ExecutionMode::Live.as_str(), "live");
        assert_eq!(ExecutionMode::Simulate.as_str(), "simulate");
        assert_eq!(ExecutionMode::Evaluate.as_str(), "evaluate");
    }

    #[test]
    fn fixed_clock_returns_set_time() {
        let t = Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap();
        let c = FixedClock::new(t);
        assert_eq!(c.now(), t);
    }

    #[test]
    fn fixed_clock_advances() {
        let t0 = Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap();
        let c = FixedClock::new(t0);
        c.advance(chrono::Duration::hours(1));
        assert_eq!(c.now(), t0 + chrono::Duration::hours(1));
    }

    #[test]
    fn seedable_rng_is_deterministic_for_same_seed() {
        let a = SeedableRng::new(42);
        let b = SeedableRng::new(42);
        for _ in 0..16 {
            assert_eq!(a.random_u64(), b.random_u64());
        }
    }

    #[test]
    fn seedable_rng_diverges_for_different_seeds() {
        let a = SeedableRng::new(1);
        let b = SeedableRng::new(2);
        let mut differences = 0;
        for _ in 0..16 {
            if a.random_u64() != b.random_u64() {
                differences += 1;
            }
        }
        assert!(
            differences > 8,
            "different seeds should diverge often (got {differences}/16)"
        );
    }

    #[test]
    fn seedable_rng_random_f64_is_in_unit_interval() {
        let r = SeedableRng::new(42);
        for _ in 0..32 {
            let v = r.random_f64();
            assert!((0.0..1.0).contains(&v), "f64 out of [0,1): {v}");
        }
    }

    #[test]
    fn check_mutation_allowed_accepts_live() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let ext = fixture_external();
        let ctx = ServiceContext::test_live(&clk, &rng, &ext);
        assert!(ctx.check_mutation_allowed().is_ok());
    }

    #[test]
    fn check_mutation_allowed_rejects_evaluate() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let ext = replay_external(StaticReplayFixture::default(), "auth-scope-test-evaluate");
        let ctx = ServiceContext::test_evaluate(&clk, &rng, &ext);
        match ctx.check_mutation_allowed() {
            Err(ServiceError::WriteBlockedByMode(ExecutionMode::Evaluate)) => {}
            other => panic!("expected WriteBlockedByMode(Evaluate), got {other:?}"),
        }
    }

    #[test]
    fn check_mutation_allowed_rejects_simulate() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let ext = fixture_external();
        let ctx = ServiceContext::new_simulate(&clk, &rng, &ext);
        match ctx.check_mutation_allowed() {
            Err(ServiceError::WriteBlockedByMode(ExecutionMode::Simulate)) => {}
            other => panic!("expected WriteBlockedByMode(Simulate), got {other:?}"),
        }
    }

    #[test]
    fn constructors_set_expected_modes() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let live_ext = fixture_external();
        let eval_ext = replay_external(StaticReplayFixture::default(), "auth-scope-test-evaluate");
        let live = ServiceContext::test_live(&clk, &rng, &live_ext);
        assert_eq!(live.mode, ExecutionMode::Live);
        let sim = ServiceContext::new_simulate(&clk, &rng, &live_ext);
        assert_eq!(sim.mode, ExecutionMode::Simulate);
        let eval = ServiceContext::test_evaluate(&clk, &rng, &eval_ext);
        assert_eq!(eval.mode, ExecutionMode::Evaluate);
    }

    #[test]
    fn service_context_new_evaluate_with_replay_external_clients_is_consistent() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let ext = replay_external(StaticReplayFixture::default(), "auth-scope-test-evaluate");

        let ctx = ServiceContext::new_evaluate(&clk, &rng, &ext);

        assert_eq!(ctx.mode, ExecutionMode::Evaluate);
        assert!(ctx.external.is_replay_mode());
        assert!(std::ptr::eq(ctx.external, &ext));
    }

    #[test]
    #[should_panic(expected = "Evaluate ServiceContext requires replay-mode ExternalClients")]
    fn service_context_new_evaluate_panics_or_errors_on_live_external_clients() {
        let clk = fixture_clock();
        let rng = fixture_rng();
        let ext = ExternalClients::default();

        let _ = ServiceContext::new_evaluate(&clk, &rng, &ext);
    }

    #[test]
    fn service_context_new_evaluate_default_constructor_uses_replay_with_empty_fixture() {
        let clk = fixture_clock();
        let rng = fixture_rng();

        let ctx = ServiceContext::new_evaluate_default(&clk, &rng);
        let err = ctx
            .external
            .glean
            .fetch_account_facts("acct-empty-fixture")
            .unwrap_err();

        assert_eq!(ctx.mode, ExecutionMode::Evaluate);
        assert!(ctx.external.is_replay_mode());
        assert!(matches!(err, ExternalClientError::ReplayFixtureMissing(_)));
    }

    #[test]
    fn shuffle_in_place_is_deterministic_for_same_seed() {
        let mut a = (0..16).collect::<Vec<u32>>();
        let mut b = (0..16).collect::<Vec<u32>>();
        let r1 = SeedableRng::new(42);
        let r2 = SeedableRng::new(42);
        shuffle_in_place(&r1, &mut a);
        shuffle_in_place(&r2, &mut b);
        assert_eq!(a, b);
    }

    #[test]
    fn external_clients_from_replay_constructs_all_clients_in_replay_mode() {
        let clients = replay_external(StaticReplayFixture::default(), "auth-scope-test-1");

        assert!(clients.is_replay_mode());
        assert!(clients.glean.is_replay());
        assert!(clients.slack.is_replay());
        assert!(clients.gmail.is_replay());
        assert!(clients.redacted.is_replay());
        assert!(!clients.glean.is_live());
        assert!(!clients.slack.is_live());
        assert!(!clients.gmail.is_live());
        assert!(!clients.redacted.is_live());
    }

    #[test]
    fn live_glean_configured_placeholder_reports_not_yet_wired() {
        struct PlaceholderGleanLiveClient;

        let inner: Arc<dyn std::any::Any + Send + Sync> = Arc::new(PlaceholderGleanLiveClient);
        let client = GleanClientHandle {
            mode: GleanClientMode::Live(Some(inner)),
        };

        let err = client.fetch_account_facts("acme.example.com").unwrap_err();

        assert!(client.is_configured());
        assert_live_not_yet_wired(err, "glean");
    }

    #[test]
    fn live_slack_placeholder_reports_not_yet_wired() {
        let client = SlackClientHandle::default();

        let err = client
            .replay_json::<serde_json::Value>(
                "GET",
                "https://slack.example.com/api/conversations.history",
                b"",
            )
            .unwrap_err();

        assert_live_not_yet_wired(err, "slack");
    }

    #[test]
    fn live_gmail_placeholder_reports_not_yet_wired() {
        let client = GmailClientHandle::default();

        let err = client
            .replay_json::<serde_json::Value>("GET", "https://gmail.example.com/api/messages", b"")
            .unwrap_err();

        assert_live_not_yet_wired(err, "gmail");
    }

    #[test]
    fn live_salesforce_placeholder_reports_not_yet_wired() {
        let client = SalesforceClientHandle::default();

        let err = client.fetch_account("acme.example.com").unwrap_err();

        assert_live_not_yet_wired(err, "redacted");
    }

    #[test]
    fn replay_glean_client_returns_fixture_response_for_known_request_key() {
        let key = GleanClientHandle::request_key_for_fetch_account_facts(
            "acct-test-1",
            "auth-scope-test-1",
        );
        let fixture = StaticReplayFixture::default().with_response(
            key,
            br#"{"account_id":"acct-test-1","facts":["example fact"]}"#,
        );
        let clients = replay_external(fixture, "auth-scope-test-1");

        let response = clients.glean.fetch_account_facts("acct-test-1").unwrap();

        assert_eq!(
            response,
            GleanAccountFacts {
                account_id: "acct-test-1".to_string(),
                facts: vec!["example fact".to_string()],
            }
        );
    }

    #[test]
    fn replay_glean_client_returns_typed_missing_error_for_unknown_request_key() {
        let clients = replay_external(StaticReplayFixture::default(), "auth-scope-test-1");
        let expected_key = GleanClientHandle::request_key_for_fetch_account_facts(
            "acct-test-1",
            "auth-scope-test-1",
        );

        let err = clients
            .glean
            .fetch_account_facts("acct-test-1")
            .unwrap_err();

        assert_replay_missing(
            err,
            expected_key,
            "GET",
            "https://glean.example.com/<redacted>",
        );
    }

    #[test]
    fn replay_redacted_client_returns_typed_missing_error_for_unknown_request_key() {
        let clients = replay_external(StaticReplayFixture::default(), "auth-scope-test-1");
        let expected_key = SalesforceClientHandle::request_key_for_fetch_account(
            "acct-test-1",
            "auth-scope-test-1",
        );

        let err = clients.redacted.fetch_account("acct-test-1").unwrap_err();

        assert_replay_missing(
            err,
            expected_key,
            "GET",
            "https://redacted.example.com/<redacted>",
        );
    }

    #[test]
    fn replay_clients_use_auth_scope_id_for_tenant_isolation() {
        let scoped_key = GleanClientHandle::request_key_for_fetch_account_facts(
            "acct-test-1",
            "auth-scope-test-1",
        );
        let other_key = GleanClientHandle::request_key_for_fetch_account_facts(
            "acct-test-1",
            "auth-scope-test-2",
        );
        let fixture = StaticReplayFixture::default().with_response(
            scoped_key,
            br#"{"account_id":"acct-test-1","facts":["scoped fixture fact"]}"#,
        );
        let fixture = Arc::new(fixture);
        let scoped_clients =
            ExternalClients::from_replay(fixture.clone(), "auth-scope-test-1".to_string());
        let other_clients = ExternalClients::from_replay(fixture, "auth-scope-test-2".to_string());

        let scoped_response = scoped_clients
            .glean
            .fetch_account_facts("acct-test-1")
            .unwrap();
        let other_err = other_clients
            .glean
            .fetch_account_facts("acct-test-1")
            .unwrap_err();

        assert_ne!(scoped_key, other_key);
        assert_eq!(
            scoped_response,
            GleanAccountFacts {
                account_id: "acct-test-1".to_string(),
                facts: vec!["scoped fixture fact".to_string()],
            }
        );
        assert_replay_missing(
            other_err,
            other_key,
            "GET",
            "https://glean.example.com/<redacted>",
        );
    }

    #[test]
    fn external_clients_default_lives_in_live_mode_not_replay() {
        let clients = ExternalClients::default();

        assert!(clients.glean.is_live());
        assert!(clients.slack.is_live());
        assert!(clients.gmail.is_live());
        assert!(clients.redacted.is_live());
        assert!(!clients.glean.is_replay());
        assert!(!clients.slack.is_replay());
        assert!(!clients.gmail.is_replay());
        assert!(!clients.redacted.is_replay());
    }
}
