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

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use chrono::{DateTime, Utc};
use http::HeaderMap;
use parking_lot::Mutex;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use schemars::{gen::SchemaGenerator, JsonSchema};
use serde::de::DeserializeOwned;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

pub use crate::abilities::claim_files::contracts::{
    ClaimFileApplyRequest, ClaimFileApplyResult, ClaimFileOperationError,
    ClaimFileProjectionResult, ClaimFileRenderRequest,
};
use crate::abilities::composition::{Composition, CompositionDocId};
pub use crate::abilities::markdown_preview::contracts::{
    MarkdownPreviewOutput, MarkdownPreviewReadRequest,
};
pub use crate::abilities::recommendations::contracts::{
    ListSuggestedNextStepsInput, ListSuggestedNextStepsResponse, SalienceReadError,
    ScoreSalienceReadRequest, ScoreSalienceResponse, SubmitRecommendationFeedbackError,
    SubmitRecommendationFeedbackInput, SubmitRecommendationFeedbackResponse,
    SuggestedNextStepsReadError,
};
use crate::abilities::registry::{Actor, ActorKind};
pub use crate::abilities::source_management_ledger::contracts::{
    SourceManagementActionReceipt, SourceManagementActionRequest,
    SourceManagementLedgerReadRequest, SourceManagementLedgerResponse,
};
use crate::abilities::temporal::{
    DetectRoleChangeInput, DetectRoleChangeResult, RefreshEngagementCurveInput,
    RefreshEngagementCurveResult, TemporalMaintenanceHandle, TrajectoryBundle,
    TrajectoryQueryDepth, TrajectoryReadHandle,
};
use crate::abilities::trust::TrustBand;
pub use crate::abilities::workspace_graph::contracts::{
    WorkspaceGraphReadRequest, WorkspaceGraphResponse,
};
pub use crate::sensitivity::ClaimDismissalSurface;
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::services::external_replay::{
    AuthScopeId, ExternalReplayFixture, ExternalReplayFixtureMissing, JsonExternalReplayFixture,
    ReplayResponse, RequestKey,
};
use crate::services::workspace_intake::WorkspaceIntakeService;
use crate::types::{
    subject_ref_from_json, ClaimSensitivity, ClaimSubjectRef, EntityContextEntry,
    EntityContextText, IntelligenceClaim,
};

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

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ExecutionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ExecutionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "live" | "Live" => Ok(ExecutionMode::Live),
            "simulate" | "Simulate" => Ok(ExecutionMode::Simulate),
            "evaluate" | "Evaluate" => Ok(ExecutionMode::Evaluate),
            other => Err(D::Error::custom(format!(
                "unknown execution mode `{other}`"
            ))),
        }
    }
}

impl JsonSchema for ExecutionMode {
    fn schema_name() -> String {
        "ExecutionMode".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
            ..Default::default()
        };
        schema.enum_values = Some(vec![
            serde_json::json!("live"),
            serde_json::json!("simulate"),
            serde_json::json!("evaluate"),
        ]);
        Schema::Object(schema)
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

    #[error("{mode} mode requires an injected fixture reader for {reader}; refusing to read from live workspace DB")]
    FixtureReaderRequired {
        mode: ExecutionMode,
        reader: &'static str,
    },

    #[error("database error: {0}")]
    Db(String),

    #[error("invariant violation: {0}")]
    Invariant(String),

    #[error("service error: {0}")]
    Other(String),
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
/// (ability code may read them). Reader handles are narrow capability seams;
/// ability code never receives raw database, app state, or filesystem handles.
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
    pub ability_id: Option<&'a str>,
    pub external: &'a ExternalClients,
    entity_context_reader: Option<Arc<dyn EntityContextReadHandle>>,
    entity_context_claim_reader: Option<Arc<dyn EntityContextClaimReadHandle>>,
    list_open_loops_reader: Option<Arc<dyn ListOpenLoopsReadHandle>>,
    prepare_meeting_context_reader: Option<Arc<dyn PrepareMeetingContextReadHandle>>,
    daily_readiness_context_reader: Option<Arc<dyn DailyReadinessContextReadHandle>>,
    briefing_callout_reader: Option<Arc<dyn BriefingCalloutReadHandle>>,
    trajectory_reader: Option<Arc<dyn TrajectoryReadHandle>>,
    temporal_maintenance: Option<Arc<dyn TemporalMaintenanceHandle>>,
    composition_commit: Option<Arc<dyn CompositionCommitHandle>>,
    entity_touchpoints_reader: Option<Arc<dyn EntityTouchpointsReadHandle>>,
    entity_neighborhood_reader: Option<Arc<dyn EntityNeighborhoodReadHandle>>,
    meeting_prep_status_reader: Option<Arc<dyn MeetingPrepStatusReadHandle>>,
    meeting_prep_narrative_reader: Option<Arc<dyn MeetingPrepNarrativeReadHandle>>,
    claim_receipt_reader: Option<Arc<dyn ClaimReceiptReadHandle>>,
    account_composition_snapshot_reader: Option<Arc<dyn AccountCompositionSnapshotReadHandle>>,
    project_composition_snapshot_reader: Option<Arc<dyn ProjectCompositionSnapshotReadHandle>>,
    person_composition_snapshot_reader: Option<Arc<dyn PersonCompositionSnapshotReadHandle>>,
    action_composition_snapshot_reader: Option<Arc<dyn ActionCompositionSnapshotReadHandle>>,
    meeting_composition_snapshot_reader: Option<Arc<dyn MeetingCompositionSnapshotReadHandle>>,
    account_list_reader: Option<Arc<dyn AccountListReadHandle>>,
    person_list_reader: Option<Arc<dyn PersonListReadHandle>>,
    project_list_reader: Option<Arc<dyn ProjectListReadHandle>>,
    markdown_preview_reader: Option<Arc<dyn MarkdownPreviewReadHandle>>,
    workspace_graph_reader: Option<Arc<dyn WorkspaceGraphReadHandle>>,
    source_management_ledger_reader: Option<Arc<dyn SourceManagementLedgerReadHandle>>,
    salience_reader: Option<Arc<dyn SalienceReadHandle>>,
    suggested_next_steps_reader: Option<Arc<dyn SuggestedNextStepsReadHandle>>,
    recommendation_feedback_writer: Option<Arc<dyn RecommendationFeedbackWriteHandle>>,
    source_management_action_handler: Option<Arc<dyn SourceManagementActionHandle>>,
    workspace_intake: Option<Arc<dyn WorkspaceIntakeService>>,
    claim_file_operations: Option<Arc<dyn ClaimFileOperationHandle>>,
}

pub type EntityContextReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<EntityContextEntry>, String>> + Send + 'a>>;

/// Narrow read handle. This keeps the ability on `AbilityContext`
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

/// Claims-backed read handle. Tests can inject this without
/// exposing raw database handles to ability code.
pub trait EntityContextClaimReadHandle: Send + Sync {
    /// Read active entity-context claims for the caller's actual render context.
    /// The `surface` MUST match where the returned claims will be rendered or
    /// used as prompt input; passing a broader surface can resurface dismissed
    /// claims in narrower contexts such as briefing prep.
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a>;

    /// Read at most `limit` active entity-context claims. Implementations that
    /// can push the limit into their backing store should override this; the
    /// default preserves compatibility for test readers.
    fn read_entity_context_claims_limited<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            let mut claims = self
                .read_entity_context_claims(entity_type, entity_id, surface, depth)
                .await?;
            claims.truncate(limit);
            Ok(claims)
        })
    }

    /// Read prompt-safe claims before applying the render page cap. Agent and
    /// MCP paths use this so confidential/user-only rows cannot occupy the
    /// bounded window and hide older prompt-safe claims.
    fn read_entity_context_prompt_claims_limited<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            let mut claims = self
                .read_entity_context_claims(entity_type, entity_id, surface, depth)
                .await?;
            claims.retain(crate::types::claim_allowed_for_prompt_input);
            claims.truncate(limit);
            Ok(claims)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountCompositionSnapshotSensitivity {
    NonSensitiveIdentity,
    Public,
    Internal,
    Confidential,
    UserOnly,
}

impl AccountCompositionSnapshotSensitivity {
    pub fn is_render_safe(&self) -> bool {
        matches!(
            self,
            Self::NonSensitiveIdentity | Self::Public | Self::Internal
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccountCompositionProvenanceKind {
    NonSensitiveIdentity,
    ManualUser,
    SourceField,
    SystemConfig,
    Derived,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AccountCompositionSnapshotField {
    pub field_path: String,
    pub label: String,
    pub value: Value,
    pub sensitivity: AccountCompositionSnapshotSensitivity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_asof: Option<String>,
    pub trust_band: TrustBand,
    pub trust_status: String,
    pub provenance_kind: AccountCompositionProvenanceKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AccountCompositionSnapshot {
    pub account_id: String,
    pub display_name: AccountCompositionSnapshotField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_type: Option<AccountCompositionSnapshotField>,
    #[serde(default)]
    pub fields: Vec<AccountCompositionSnapshotField>,
    /// The entity's enriched intelligence payload (camelCase, the same shape
    /// the production account-detail surface renders: currentState, risks,
    /// valueDelivered, agreementOutlook, recommendedActions, …). The producer
    /// reshapes chapter-relevant subsets into block payloads so composition
    /// surfaces carry the SAME content as the production page, not just
    /// atomic claims. None when the account has never been enriched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intelligence: Option<serde_json::Value>,
    /// Glean leading-signal bundle (HealthOutlookSignals, camelCase) — same
    /// read the production detail page makes. None when enrichment never ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glean_signals: Option<serde_json::Value>,
    /// User health sentiment: { current, setAt, note } from account columns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sentiment: Option<serde_json::Value>,
    /// Stakeholder records: { stakeholdersFull, accountName } — the
    /// production StakeholderGrid contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stakeholders: Option<serde_json::Value>,
    /// Technical footprint row (camelCase), production component contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_footprint: Option<serde_json::Value>,
    /// Commercial shape fields: { arr, renewalDate }.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commercial: Option<serde_json::Value>,
    /// Relationship fabric fields: { nps, strategicPrograms }.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabric: Option<serde_json::Value>,
    /// Record/timeline bundle: { lifecycleChanges, recentMeetings }.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AccountCompositionSnapshotReadError {
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("account composition snapshot read failed: {0}")]
    ReadFailed(String),
}

pub type AccountCompositionSnapshotReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<AccountCompositionSnapshot, AccountCompositionSnapshotReadError>>
            + Send
            + 'a,
    >,
>;

/// Service-owned Account page input bundle for non-claim display facts.
/// Implementations must wrap every non-identity field with sensitivity,
/// source/freshness, trust, and provenance metadata before ability code may
/// render it.
pub trait AccountCompositionSnapshotReadHandle: Send + Sync {
    fn read_account_composition_snapshot<'a>(
        &'a self,
        account_id: String,
        surface: ClaimDismissalSurface,
    ) -> AccountCompositionSnapshotReadFuture<'a>;
}

pub type ProjectCompositionSnapshotSensitivity = AccountCompositionSnapshotSensitivity;
pub type ProjectCompositionProvenanceKind = AccountCompositionProvenanceKind;
pub type ProjectCompositionSnapshotField = AccountCompositionSnapshotField;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectCompositionSnapshot {
    pub project_id: String,
    pub display_name: ProjectCompositionSnapshotField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ProjectCompositionSnapshotField>,
    pub is_parent: bool,
    #[serde(default)]
    pub fields: Vec<ProjectCompositionSnapshotField>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectCompositionSnapshotReadError {
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("project composition snapshot read failed: {0}")]
    ReadFailed(String),
}

pub type ProjectCompositionSnapshotReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<ProjectCompositionSnapshot, ProjectCompositionSnapshotReadError>>
            + Send
            + 'a,
    >,
>;

/// Service-owned Project page input bundle for non-claim display facts.
/// Implementations must wrap every non-identity field with sensitivity,
/// source/freshness, trust, and provenance metadata before ability code may
/// render it.
pub trait ProjectCompositionSnapshotReadHandle: Send + Sync {
    fn read_project_composition_snapshot<'a>(
        &'a self,
        project_id: String,
        surface: ClaimDismissalSurface,
    ) -> ProjectCompositionSnapshotReadFuture<'a>;
}

pub type PersonCompositionSnapshotSensitivity = AccountCompositionSnapshotSensitivity;
pub type PersonCompositionProvenanceKind = AccountCompositionProvenanceKind;
pub type PersonCompositionSnapshotField = AccountCompositionSnapshotField;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PersonCompositionSnapshot {
    pub person_id: String,
    pub display_name: PersonCompositionSnapshotField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<PersonCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<PersonCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<PersonCompositionSnapshotField>,
    #[serde(default)]
    pub fields: Vec<PersonCompositionSnapshotField>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersonCompositionSnapshotReadError {
    #[error("person not found: {0}")]
    PersonNotFound(String),
    #[error("person composition snapshot read failed: {0}")]
    ReadFailed(String),
}

pub type PersonCompositionSnapshotReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<PersonCompositionSnapshot, PersonCompositionSnapshotReadError>>
            + Send
            + 'a,
    >,
>;

/// Service-owned Person page input bundle for non-claim display facts.
/// Implementations must wrap every non-identity field with sensitivity,
/// source/freshness, trust, and provenance metadata before ability code may
/// render it.
pub trait PersonCompositionSnapshotReadHandle: Send + Sync {
    fn read_person_composition_snapshot<'a>(
        &'a self,
        person_id: String,
        surface: ClaimDismissalSurface,
    ) -> PersonCompositionSnapshotReadFuture<'a>;
}

pub type ActionCompositionSnapshotSensitivity = AccountCompositionSnapshotSensitivity;
pub type ActionCompositionProvenanceKind = AccountCompositionProvenanceKind;
pub type ActionCompositionSnapshotField = AccountCompositionSnapshotField;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActionCompositionSnapshot {
    pub action_id: String,
    pub title: ActionCompositionSnapshotField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ActionCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<ActionCompositionSnapshotField>,
    #[serde(default)]
    pub fields: Vec<ActionCompositionSnapshotField>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionCompositionSnapshotReadError {
    #[error("action not found: {0}")]
    ActionNotFound(String),
    #[error("action composition snapshot read failed: {0}")]
    ReadFailed(String),
}

pub type ActionCompositionSnapshotReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<ActionCompositionSnapshot, ActionCompositionSnapshotReadError>>
            + Send
            + 'a,
    >,
>;

/// Service-owned Action Detail input bundle for non-claim display facts.
/// Action is a first-class composition subject, not a generic entity type.
pub trait ActionCompositionSnapshotReadHandle: Send + Sync {
    fn read_action_composition_snapshot<'a>(
        &'a self,
        action_id: String,
        surface: ClaimDismissalSurface,
    ) -> ActionCompositionSnapshotReadFuture<'a>;
}

pub type MeetingCompositionSnapshotSensitivity = AccountCompositionSnapshotSensitivity;
pub type MeetingCompositionProvenanceKind = AccountCompositionProvenanceKind;
pub type MeetingCompositionSnapshotField = AccountCompositionSnapshotField;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingCompositionSnapshotQuery {
    pub meeting_id: String,
    pub meeting_token: String,
    pub surface: ClaimDismissalSurface,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MeetingCompositionSnapshot {
    pub meeting_token: String,
    pub title: MeetingCompositionSnapshotField,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<MeetingCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<MeetingCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_type: Option<MeetingCompositionSnapshotField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<MeetingCompositionSnapshotField>,
    #[serde(default)]
    pub is_past: bool,
    #[serde(default)]
    pub is_current: bool,
    #[serde(default)]
    pub has_transcript: bool,
    #[serde(default)]
    pub fields: Vec<MeetingCompositionSnapshotField>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MeetingCompositionSnapshotReadError {
    #[error("meeting not found: {0}")]
    MeetingNotFound(String),
    #[error("meeting composition snapshot read failed: {0}")]
    ReadFailed(String),
}

pub type MeetingCompositionSnapshotReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<MeetingCompositionSnapshot, MeetingCompositionSnapshotReadError>>
            + Send
            + 'a,
    >,
>;

/// Service-owned Meeting Detail input bundle. The query carries the raw row id
/// only inside the Tauri/backend hydration path; the returned snapshot uses
/// `meeting_token` as the render-visible identifier.
pub trait MeetingCompositionSnapshotReadHandle: Send + Sync {
    fn read_meeting_composition_snapshot<'a>(
        &'a self,
        query: MeetingCompositionSnapshotQuery,
    ) -> MeetingCompositionSnapshotReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionProposal {
    pub composition_id: CompositionDocId,
    pub expected_composition_version: u64,
    pub composition: Composition,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommittedComposition {
    pub composition_id: CompositionDocId,
    pub composition_version: u64,
    pub composition: Composition,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompositionCommitError {
    #[error("composition id is empty")]
    EmptyCompositionId,
    #[error(
        "stale composition version for {composition_id}: expected {expected}, current {current}"
    )]
    StaleVersion {
        composition_id: String,
        expected: u64,
        current: u64,
    },
    #[error(
        "inflated composition version for {composition_id}: expected {expected}, current {current}"
    )]
    InflatedVersion {
        composition_id: String,
        expected: u64,
        current: u64,
    },
    #[error("composition version overflow for {composition_id}")]
    Overflow { composition_id: String },
    #[error("composition transaction failed: {0}")]
    Transaction(String),
    #[error("composition mutation blocked by mode: {0}")]
    Mode(String),
    #[error("composition finalizer unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone)]
pub struct CompositionCommitRequest {
    pub proposal: CompositionProposal,
    pub actor: String,
    pub ability_id: Option<String>,
}

pub type CompositionCommitFuture<'a> =
    Pin<Box<dyn Future<Output = Result<CommittedComposition, CompositionCommitError>> + Send + 'a>>;

/// Narrow finalizer seam for W4-B `commit_composition`. Ability code remains
/// in `abilities-runtime`; app code attaches the concrete SQLite adapter.
pub trait CompositionCommitHandle: Send + Sync {
    fn commit_composition<'a>(
        &'a self,
        request: CompositionCommitRequest,
    ) -> CompositionCommitFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOpenLoopsQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub surface: ClaimDismissalSurface,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListOpenLoopsSnapshot {
    pub claims: Vec<IntelligenceClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ListOpenLoopsReadError {
    #[error("subject is not owned by this workspace: {entity_type}:{entity_id}")]
    SubjectNotOwned {
        entity_type: String,
        entity_id: String,
    },
    #[error("{0}")]
    ReadFailed(String),
}

pub type ListOpenLoopsReadFuture<'a> = Pin<
    Box<dyn Future<Output = Result<ListOpenLoopsSnapshot, ListOpenLoopsReadError>> + Send + 'a>,
>;

pub trait ListOpenLoopsReadHandle: Send + Sync {
    fn read_open_loops<'a>(&'a self, query: ListOpenLoopsQuery) -> ListOpenLoopsReadFuture<'a>;
}

// -----------------------------------------------------------------------------
// v1.4.4 W1 substrate extension — entity index read seams.
//
// `list_accounts` / `list_people` / `list_projects` are W2 §5.5 list-shell
// abilities (Accounts/People/Projects index). The producers shape opaque
// paginated responses; the reader handles below are the narrow capability
// seams the app-side adapters fill in. Each query carries the
// already-validated filter + offset/page_size; each snapshot carries a
// concise list-row plus a total + optional concurrent-mutation advisory
// the producer maps to `CursorState::DataShifted`.
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountListQuery {
    pub status: Option<String>,
    pub health_band: Option<crate::abilities::trust::types::TrustBand>,
    pub name_contains: Option<String>,
    pub offset: u64,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountListSummary {
    pub account_id: String,
    pub name: String,
    pub status: String,
    pub health_band: crate::abilities::trust::types::TrustBand,
    pub last_touchpoint_at: Option<String>,
    pub open_loops_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountListSnapshot {
    pub items: Vec<AccountListSummary>,
    pub total_after_filter: u64,
    /// `Some(advisory)` when the reader detected a concurrent insert/
    /// retract between the cursor's offset and the page boundary — the
    /// producer maps this verbatim to `CursorState::DataShifted`.
    pub data_shifted_advisory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AccountListReadError {
    #[error("{0}")]
    ReadFailed(String),
}

pub type AccountListReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AccountListSnapshot, AccountListReadError>> + Send + 'a>>;

pub trait AccountListReadHandle: Send + Sync {
    fn read_accounts<'a>(&'a self, query: AccountListQuery) -> AccountListReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonListQuery {
    pub role: Option<String>,
    pub primary_account_id: Option<String>,
    pub name_contains: Option<String>,
    pub offset: u64,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonListSummary {
    pub person_id: String,
    pub display_name: String,
    pub primary_account_id: Option<String>,
    pub role: String,
    pub last_touchpoint_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonListSnapshot {
    pub items: Vec<PersonListSummary>,
    pub total_after_filter: u64,
    pub data_shifted_advisory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersonListReadError {
    #[error("{0}")]
    ReadFailed(String),
}

pub type PersonListReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PersonListSnapshot, PersonListReadError>> + Send + 'a>>;

pub trait PersonListReadHandle: Send + Sync {
    fn read_people<'a>(&'a self, query: PersonListQuery) -> PersonListReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListQuery {
    pub status: Option<String>,
    pub trajectory: Option<crate::abilities::list_projects::ProjectTrajectory>,
    pub parent_account_id: Option<String>,
    pub name_contains: Option<String>,
    pub offset: u64,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListSummary {
    pub project_id: String,
    pub name: String,
    pub parent_account_id: Option<String>,
    pub status: String,
    pub trajectory: crate::abilities::list_projects::ProjectTrajectory,
    pub last_touchpoint_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListSnapshot {
    pub items: Vec<ProjectListSummary>,
    pub total_after_filter: u64,
    pub data_shifted_advisory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectListReadError {
    #[error("{0}")]
    ReadFailed(String),
}

pub type ProjectListReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProjectListSnapshot, ProjectListReadError>> + Send + 'a>>;

pub trait ProjectListReadHandle: Send + Sync {
    fn read_projects<'a>(&'a self, query: ProjectListQuery) -> ProjectListReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarkdownPreviewReadError {
    #[error("{0}")]
    InvalidSourceHandle(String),
    #[error("source not found")]
    SourceNotFound,
    #[error("{0}")]
    SourceUnavailable(String),
    #[error("{0}")]
    UnsupportedSource(String),
}

pub type MarkdownPreviewReadFuture<'a> = Pin<
    Box<dyn Future<Output = Result<MarkdownPreviewOutput, MarkdownPreviewReadError>> + Send + 'a>,
>;

pub trait MarkdownPreviewReadHandle: Send + Sync {
    fn read_markdown_preview<'a>(
        &'a self,
        request: MarkdownPreviewReadRequest,
    ) -> MarkdownPreviewReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceGraphReadError {
    #[error("{0}")]
    InvalidCursor(String),
    #[error("{0}")]
    InvalidFilter(String),
    #[error("page size {requested} exceeds max {max}")]
    PageSizeTooLarge { requested: u32, max: u32 },
    #[error("{0}")]
    ReadFailed(String),
    #[error("{0}")]
    AuditFailed(String),
}

pub type WorkspaceGraphReadFuture<'a> = Pin<
    Box<dyn Future<Output = Result<WorkspaceGraphResponse, WorkspaceGraphReadError>> + Send + 'a>,
>;

pub trait WorkspaceGraphReadHandle: Send + Sync {
    fn read_workspace_graph<'a>(
        &'a self,
        request: WorkspaceGraphReadRequest,
    ) -> WorkspaceGraphReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceManagementLedgerReadError {
    #[error("{0}")]
    InvalidCursor(String),
    #[error("{0}")]
    InvalidFilter(String),
    #[error("page size {requested} exceeds max {max}")]
    PageSizeTooLarge { requested: u32, max: u32 },
    #[error("{0}")]
    ReadFailed(String),
}

pub type SourceManagementLedgerReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<SourceManagementLedgerResponse, SourceManagementLedgerReadError>>
            + Send
            + 'a,
    >,
>;

pub trait SourceManagementLedgerReadHandle: Send + Sync {
    fn read_source_management_ledger<'a>(
        &'a self,
        request: SourceManagementLedgerReadRequest,
    ) -> SourceManagementLedgerReadFuture<'a>;
}

pub type SalienceReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ScoreSalienceResponse, SalienceReadError>> + Send + 'a>>;

pub trait SalienceReadHandle: Send + Sync {
    fn score_salience<'a>(&'a self, request: ScoreSalienceReadRequest) -> SalienceReadFuture<'a>;
}

pub type SuggestedNextStepsReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<ListSuggestedNextStepsResponse, SuggestedNextStepsReadError>>
            + Send
            + 'a,
    >,
>;

pub trait SuggestedNextStepsReadHandle: Send + Sync {
    fn list_suggested_next_steps<'a>(
        &'a self,
        input: ListSuggestedNextStepsInput,
        actor: ActorKind,
    ) -> SuggestedNextStepsReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecommendationFeedbackWriteRequest {
    pub input: SubmitRecommendationFeedbackInput,
    pub actor: Actor,
    pub mode: ExecutionMode,
    pub recorded_at: DateTime<Utc>,
    pub ability_id: Option<String>,
}

pub type RecommendationFeedbackWriteFuture<'a> = Pin<
    Box<
        dyn Future<
                Output = Result<
                    SubmitRecommendationFeedbackResponse,
                    SubmitRecommendationFeedbackError,
                >,
            > + Send
            + 'a,
    >,
>;

pub trait RecommendationFeedbackWriteHandle: Send + Sync {
    fn submit_recommendation_feedback<'a>(
        &'a self,
        request: RecommendationFeedbackWriteRequest,
    ) -> RecommendationFeedbackWriteFuture<'a>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceManagementActionError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("{0}")]
    ActionFailed(String),
}

pub type SourceManagementActionFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<SourceManagementActionReceipt, SourceManagementActionError>>
            + Send
            + 'a,
    >,
>;

pub trait SourceManagementActionHandle: Send + Sync {
    fn apply_source_management_action<'a>(
        &'a self,
        request: SourceManagementActionRequest,
    ) -> SourceManagementActionFuture<'a>;
}

pub type ClaimFileRenderFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<ClaimFileProjectionResult, ClaimFileOperationError>> + Send + 'a,
    >,
>;

pub type ClaimFileApplyFuture<'a> = Pin<
    Box<dyn Future<Output = Result<ClaimFileApplyResult, ClaimFileOperationError>> + Send + 'a>,
>;

pub trait ClaimFileOperationHandle: Send + Sync {
    fn render_entity_claim_file<'a>(
        &'a self,
        request: ClaimFileRenderRequest,
    ) -> ClaimFileRenderFuture<'a>;

    fn apply_claim_file_corrections<'a>(
        &'a self,
        request: ClaimFileApplyRequest,
    ) -> ClaimFileApplyFuture<'a>;
}

// -----------------------------------------------------------------------------
// Canonical entity touchpoints read seam.
//
// Narrow read handle for entity-scoped touchpoint composition: the producer
// asks for upcoming + recent meeting-shaped interactions for a subject; the
// app-side reader resolves those out of the current entity-link graph, with
// legacy junction rows as fallback, plus parent/child account expansion and
// attendee-match fallback. Subject isolation lives in the reader's filter —
// the reader returns each candidate with an explicit `inclusion_reason`, never
// a raw join soup.
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityTouchpointsQuery {
    pub entity_type: String,
    pub entity_id: String,
    pub now: DateTime<Utc>,
    /// Days into the future considered "upcoming".
    pub upcoming_window_days: u16,
    /// Days into the past considered "recent".
    pub recent_window_days: u16,
    /// Hard cap per side (upcoming/recent) to bound the reader.
    pub per_side_cap: usize,
}

/// Why a touchpoint belongs to this subject — surfaced verbatim to the
/// envelope so callers can debug subject bleed without re-querying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchpointInclusionReason {
    /// Direct meeting/entity link for this subject.
    SubjectMatch,
    /// Inherited via parent/child account or related entity link.
    EntityLink,
    /// Person subject found as attendee of the meeting.
    AttendeeMatch,
    /// Domain match (e.g., attendee email domain matches account).
    DomainMatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityTouchpointSnapshot {
    pub meeting_id: String,
    pub title: String,
    pub kind: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    /// Echo of the requesting subject in `kind:id` form so a downstream
    /// composer can re-render `SubjectRef` without re-parsing IDs.
    pub subject_entity_type: String,
    pub subject_entity_id: String,
    pub inclusion_reason: TouchpointInclusionReason,
    /// Populated only when the candidate set was over capacity OR the reader
    /// dropped a row for a typed reason (low confidence, suppressed, etc.).
    pub exclusion_reason: Option<String>,
    pub source_asof: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityTouchpointsSnapshot {
    pub subject_entity_type: String,
    pub subject_entity_id: String,
    pub upcoming: Vec<EntityTouchpointSnapshot>,
    pub recent: Vec<EntityTouchpointSnapshot>,
    /// Additional subject IDs whose touchpoints were also pulled in via
    /// parent/child link or related-entity expansion. The composer uses this
    /// to populate `SubjectScope::also_includes` so downstream surfaces can
    /// show "scope includes child accounts X, Y".
    pub also_includes: Vec<(String, String)>,
    /// Filter description for `CandidateSetRef::filter_description` — explains
    /// how the candidate set was assembled, in plain English.
    pub filter_description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EntityTouchpointsReadError {
    #[error("subject is not owned by this workspace: {entity_type}:{entity_id}")]
    SubjectNotOwned {
        entity_type: String,
        entity_id: String,
    },
    #[error("{0}")]
    ReadFailed(String),
}

pub type EntityTouchpointsReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<EntityTouchpointsSnapshot, EntityTouchpointsReadError>>
            + Send
            + 'a,
    >,
>;

pub trait EntityTouchpointsReadHandle: Send + Sync {
    fn read_entity_touchpoints<'a>(
        &'a self,
        query: EntityTouchpointsQuery,
    ) -> EntityTouchpointsReadFuture<'a>;
}

// -----------------------------------------------------------------------------
// Canonical entity-neighborhood read seam.
//
// Read-only projection over existing relationship substrate. This is not a new
// canonical graph store; app-side readers assemble bounded relationship and
// participation evidence from existing tables (linked entity graph,
// meeting_attendees, account_stakeholders/entity_members, person_relationships,
// hierarchy links, actions/content/email where available).
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityNeighborhoodQuery {
    pub entity_type: String,
    pub entity_id: String,
    pub now: DateTime<Utc>,
    /// Maximum traversal depth. Default callers should use <= 2.
    pub max_depth: u8,
    /// Hard cap per evidence class to keep envelope projection bounded.
    pub per_edge_cap: usize,
    /// Number of recent touchpoint ids retained per participant.
    pub recent_touchpoint_cap: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRelationshipInclusionReason {
    SubjectMatch,
    Hierarchy,
    ExplicitLink,
    AttendeeMatch,
    CoAttendance,
    WorkItem,
    ContentLink,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityRelationshipEdgeSnapshot {
    pub edge_type: String,
    pub related_entity_type: String,
    pub related_entity_id: String,
    pub related_display_label: Option<String>,
    pub source_id: String,
    pub source_type: String,
    pub observed_at: Option<String>,
    pub source_asof: Option<String>,
    pub confidence: f32,
    pub sensitivity: ClaimSensitivity,
    pub inclusion_reason: EntityRelationshipInclusionReason,
    pub traversal_depth: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityParticipantSnapshot {
    pub person_id: String,
    pub display_label: Option<String>,
    pub role: Option<String>,
    pub relationship: Option<String>,
    pub normalized_touchpoint_count: u32,
    pub recent_touchpoint_ids: Vec<String>,
    pub last_seen_at: Option<String>,
    pub source_id: String,
    pub source_type: String,
    pub source_asof: Option<String>,
    pub confidence: f32,
    pub sensitivity: ClaimSensitivity,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityNeighborhoodTruncation {
    pub edges_truncated: bool,
    pub participants_truncated: bool,
    pub per_edge_cap: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityNeighborhoodSnapshot {
    pub subject_entity_type: String,
    pub subject_entity_id: String,
    pub edges: Vec<EntityRelationshipEdgeSnapshot>,
    pub participants: Vec<EntityParticipantSnapshot>,
    pub truncation: EntityNeighborhoodTruncation,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EntityNeighborhoodReadError {
    #[error("subject is not owned by this workspace: {entity_type}:{entity_id}")]
    SubjectNotOwned {
        entity_type: String,
        entity_id: String,
    },
    #[error("{0}")]
    ReadFailed(String),
}

pub type EntityNeighborhoodReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<EntityNeighborhoodSnapshot, EntityNeighborhoodReadError>>
            + Send
            + 'a,
    >,
>;

pub trait EntityNeighborhoodReadHandle: Send + Sync {
    fn read_entity_neighborhood<'a>(
        &'a self,
        query: EntityNeighborhoodQuery,
    ) -> EntityNeighborhoodReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrepareMeetingContextSnapshot {
    pub meeting: PrepareMeetingSnapshot,
    pub attendees: Vec<PrepareMeetingAttendeeSnapshot>,
    pub subjects: Vec<PrepareMeetingSubjectSnapshot>,
    pub claims: Vec<IntelligenceClaim>,
    pub linear_issue_changes: Vec<PrepareMeetingLinearIssueChangeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareMeetingSnapshot {
    pub id: String,
    pub title: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub attendees_raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareMeetingAttendeeSnapshot {
    pub name: String,
    pub email: Option<String>,
    pub person_id: Option<String>,
    pub account_id: Option<String>,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareMeetingSubjectSnapshot {
    pub kind: String,
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareMeetingLinearIssueChangeSnapshot {
    pub signal_id: String,
    pub issue_id: String,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub subject: PrepareMeetingSubjectSnapshot,
    pub signal_type: String,
    pub from_state: Option<String>,
    pub to_state: Option<String>,
    pub current_state_type: Option<String>,
    pub current_state_name: Option<String>,
    pub source_asof: String,
}

pub type PrepareMeetingContextReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<PrepareMeetingContextSnapshot, String>> + Send + 'a>>;

/// Narrow read handle for claim-backed meeting brief context assembly.
/// Production reads route through `services::meetings`; tests and the eval
/// harness inject a fixture snapshot built from their isolated SQLite state.
pub trait PrepareMeetingContextReadHandle: Send + Sync {
    fn read_prepare_meeting_context<'a>(
        &'a self,
        meeting_id: String,
    ) -> PrepareMeetingContextReadFuture<'a>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyReadinessContextSnapshot {
    pub workspace_scope: String,
    pub date: String,
    pub meetings: Vec<DailyReadinessMeetingSnapshot>,
    pub tracked_subjects: Vec<DailyReadinessSubjectSnapshot>,
    pub overnight_changes: Vec<DailyReadinessSignalSnapshot>,
    pub risk_shifts: Vec<DailyReadinessRiskSnapshot>,
    pub open_loops: Vec<DailyReadinessOpenLoopSnapshot>,
    pub coverage_warnings: Vec<DailyReadinessCoverageWarningSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyReadinessMeetingSnapshot {
    pub id: String,
    pub title: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub workspace_scope: String,
    /// Lower-snake_case meeting_type discriminant (`customer`, `qbr`,
    /// `partnership`, `external`, `internal`, `team_sync`, `one_on_one`,
    /// `all_hands`, `training`). Consumers gate advisory-class logic on
    /// `is_customer_facing` — surface display still iterates every row.
    pub meeting_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyReadinessSubjectSnapshot {
    pub kind: String,
    pub id: String,
    pub display_name: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyReadinessSignalSnapshot {
    pub id: String,
    pub subject: DailyReadinessSubjectSnapshot,
    pub summary: String,
    pub source_ref: Option<String>,
    pub observed_at: String,
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyReadinessRiskSnapshot {
    pub id: String,
    pub subject: DailyReadinessSubjectSnapshot,
    pub direction: String,
    pub evidence_summary: String,
    pub source_ref: Option<String>,
    pub observed_at: String,
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailyReadinessOpenLoopSnapshot {
    pub id: String,
    pub text: String,
    pub owner: Option<String>,
    pub subject: DailyReadinessSubjectSnapshot,
    pub due_date: Option<String>,
    pub source_ref: Option<String>,
    pub observed_at: String,
    pub source_asof: Option<String>,
    pub data_source: String,
    pub lifecycle: String,
    pub confidence: f32,
    pub sensitivity: String,
    pub workspace_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyReadinessCoverageWarningSnapshot {
    pub kind: String,
    pub message: String,
    pub count: u32,
    pub workspace_scope: String,
}

pub type DailyReadinessContextReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DailyReadinessContextSnapshot, String>> + Send + 'a>>;

/// Caller-declared surface intent for meetings projection. The trust contract
/// is "ask for the shape you'll render, get rows already pruned of types that
/// don't belong on that surface." Producers don't post-filter; consumers don't
/// see personal blocks leak into briefing advisories.
///
/// `Briefing` and `Schedule` share exclusions today (personal). They're
/// distinct enum variants so a future divergence (e.g., Schedule keeping
/// personal blocks for time-blocking awareness) lands as a behavior change,
/// not a new parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingsViewIntent {
    Briefing,
    Schedule,
    AllRows,
}

/// Customer-facing meeting types — the ones that warrant prep documents and
/// customer/account entity attribution. Internal team meetings stay in the
/// schedule but should not drive `needs_prep` / `unlinked_meetings`
/// advisories. Pure-function, no allocation; callers pass the snapshot's
/// `meeting_type` discriminant directly.
pub fn is_customer_facing(meeting_type: &str) -> bool {
    matches!(
        meeting_type,
        "customer" | "qbr" | "partnership" | "external"
    )
}

/// Narrow read handle for daily-readiness seed assembly. Ability code receives
/// only this workspace-scoped snapshot, never raw DB or app-state handles.
pub trait DailyReadinessContextReadHandle: Send + Sync {
    fn read_daily_readiness_context<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
        intent: MeetingsViewIntent,
    ) -> DailyReadinessContextReadFuture<'a>;
}

// -----------------------------------------------------------------------------
// Briefing callout read handle
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BriefingCalloutSnapshot {
    pub id: String,
    pub headline: String,
    pub detail: Option<String>,
    pub severity: String,
    pub entity_name: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub created_at: String,
    pub claim_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BriefingCalloutReadError {
    #[error("briefing callout read failed: {0}")]
    ReadFailed(String),
}

pub type BriefingCalloutReadFuture<'a> = Pin<
    Box<dyn Future<Output = Result<Vec<BriefingCalloutSnapshot>, BriefingCalloutReadError>> + Send + 'a>,
>;

/// Read today's durable briefing callouts. This is the attention substrate for
/// the daily briefing producer; ability code receives row-shaped snapshots,
/// never raw database handles.
pub trait BriefingCalloutReadHandle: Send + Sync {
    fn read_briefing_callouts_for_date<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
        limit: usize,
    ) -> BriefingCalloutReadFuture<'a>;
}

// -----------------------------------------------------------------------------
// Meeting prep status read handle
// -----------------------------------------------------------------------------

/// Service-owned status describing whether a meeting's prep is ready, blocked,
/// stale, failed, or user-suppressed. The richer DTO lives in the app crate
/// (`services::meeting_prep_status`); this mirror is the narrow shape the
/// `get_daily_briefing` Read ability consumes through a read handle.
///
/// Stays a string-typed projection on the abilities-runtime side so the crate
/// boundary doesn't force a circular dependency — the app crate's adapter
/// stringifies `PrepStatus` / `BlockingReason` / `StaleReason` enums when
/// projecting into this snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingPrepStatusSnapshot {
    pub meeting_id: String,
    pub event_id: Option<String>,
    pub linked_entity_type: Option<String>,
    pub linked_entity_id: Option<String>,
    /// Lower-snake_case PrepStatus discriminant: `ready` | `prep_needed` |
    /// `queued` | `running` | `limited` | `stale` | `failed` |
    /// `blocked_no_entity` | `user_suppressed` | `user_dismissed`.
    pub status: String,
    pub blocking_reason: Option<String>,
    pub stale_reason: Option<String>,
    pub last_prepared_at: Option<String>,
    pub source_asof_inputs: Vec<MeetingPrepSourceAsofRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingPrepSourceAsofRef {
    pub source: String,
    pub as_of: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MeetingPrepStatusReadError {
    #[error("meeting prep status read failed: {0}")]
    ReadFailed(String),
    #[error("meeting not found: {0}")]
    MeetingNotFound(String),
}

pub type MeetingPrepStatusReadFuture<'a> = Pin<
    Box<
        dyn Future<Output = Result<MeetingPrepStatusSnapshot, MeetingPrepStatusReadError>>
            + Send
            + 'a,
    >,
>;

/// Narrow read handle attached by the app crate so the `get_daily_briefing`
/// ability can compose per-meeting prep status into its envelope. AC-507.2 /
/// AC-507.7 — the handle is read-only by contract; the adapter must not
/// perform any mutation.
pub trait MeetingPrepStatusReadHandle: Send + Sync {
    fn read_meeting_prep_status<'a>(
        &'a self,
        meeting_id: String,
    ) -> MeetingPrepStatusReadFuture<'a>;
}

pub type MeetingPrepNarrativeReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<String>, String>> + Send + 'a>>;

/// Narrow read handle attached by the app crate so the `get_daily_briefing`
/// ability can compose each meeting's prep-context narrative (the
/// `prep_context_json` summary) into its briefing envelope. Read-only by
/// contract; the adapter must not perform any mutation.
pub trait MeetingPrepNarrativeReadHandle: Send + Sync {
    fn read_meeting_prep_narrative<'a>(
        &'a self,
        meeting_id: String,
    ) -> MeetingPrepNarrativeReadFuture<'a>;
}

// ---------------------------------------------------------------------------
// claim_receipt — Read ability dispatch surface
// ---------------------------------------------------------------------------
//
// Read ability that wraps the existing `services::claim_receipt::render`
// substrate so WP block inner-block renderers (account-detail / project-detail
// quote-wall, value-commitments, on-track-chapter, technical-footprint, etc.)
// can fan out per-claim receipts through `runtime_client->invoke_ability(
// 'claim_receipt', ...)`. Without this registration the WP-side invocation
// returns AbilityUnavailable and the 60+ claim-bearing inner blocks fall back
// to empty placeholders.
//
// The ability is a thin shell: input shape mirrors `ReceiptTarget` plus
// `SurfaceContext`, output mirrors the `ClaimReceipt` DTO that already exists
// in the app crate. The narrow read handle below is the adapter seam — the
// app crate's `LiveClaimReceiptReader` translates the ability-shaped DTOs into
// `services::claim_receipt::contracts` types, dispatches through
// `render_receipt_for(state, target, surface)`, and translates the result
// back. This keeps `AppState` / SQL handles out of `abilities-runtime`.

/// Ability-shaped mirror of `services::claim_receipt::contracts::ReceiptTarget`.
///
/// Serde wire format is byte-equivalent to the app crate DTO so the live
/// reader can round-trip through `serde_json::Value` if a less-coupled adapter
/// is desired later. Per L0-W1 §5.6 deferral only the `Claim` arm resolves
/// today; `Proposal` and `WorkItem` arms are accepted by the contract but the
/// reader returns `TargetNotFound`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClaimReceiptTarget {
    #[serde(rename_all = "camelCase")]
    Claim {
        claim_id: String,
        subject: crate::abilities::provenance::SubjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field_path: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Proposal {
        proposal_id: String,
        subject: crate::abilities::provenance::SubjectRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field_path: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    WorkItem {
        action_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        backing_claim_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject: Option<crate::abilities::provenance::SubjectRef>,
    },
}

/// Ability-shaped mirror of `services::claim_receipt::contracts::SurfaceContext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReceiptSurfaceContext {
    ActionsWork,
    EntityDetail,
    DailyBriefing,
    MeetingDetail,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReceiptFreshness {
    Current,
    Aging,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimReceiptRedactionLevel {
    None,
    Partial,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptTrust {
    pub band: crate::abilities::trust::types::TrustBand,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub source_asof: Option<DateTime<Utc>>,
    pub freshness: ClaimReceiptFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caveat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptLifecycle {
    pub claim_state: crate::types::ClaimState,
    pub surfacing_state: crate::types::SurfacingState,
    pub verification_state: crate::sensitivity::ClaimVerificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptProvenanceSource {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub as_of: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptProvenance {
    pub sources: Vec<ClaimReceiptProvenanceSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_summary: Option<String>,
    pub redaction: ClaimReceiptRedactionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptAction {
    pub action: crate::abilities::feedback::FeedbackAction,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

/// Ability-shaped mirror of `services::claim_receipt::contracts::ClaimReceipt`.
///
/// Wire-format parity with the Tauri command output ensures the WP-side
/// renderer can consume either path. The live reader in the app crate is
/// responsible for byte-equivalent translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceiptSnapshot {
    pub target: ClaimReceiptTarget,
    pub surface_context: ClaimReceiptSurfaceContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_text: Option<crate::sensitivity::RenderableClaimText>,
    pub trust: ClaimReceiptTrust,
    pub lifecycle: ClaimReceiptLifecycle,
    pub provenance: ClaimReceiptProvenance,
    pub actions: Vec<ClaimReceiptAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClaimReceiptReadError {
    #[error("claim receipt target not found")]
    TargetNotFound,
    #[error("claim receipt dropped by privacy gate")]
    PrivacyDrop,
    #[error("{0}")]
    ReadFailed(String),
}

pub type ClaimReceiptReadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ClaimReceiptSnapshot, ClaimReceiptReadError>> + Send + 'a>>;

/// Narrow read handle that the app crate attaches so the `claim_receipt`
/// ability can dispatch through `services::claim_receipt::render::
/// render_receipt_for` without pulling `AppState` or SQL handles into
/// `abilities-runtime`. Read-only by contract; the adapter MUST NOT mutate.
pub trait ClaimReceiptReadHandle: Send + Sync {
    fn read_claim_receipt<'a>(
        &'a self,
        target: ClaimReceiptTarget,
        surface: ClaimReceiptSurfaceContext,
    ) -> ClaimReceiptReadFuture<'a>;
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
            ability_id: None,
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            list_open_loops_reader: None,
            prepare_meeting_context_reader: None,
            daily_readiness_context_reader: None,
            briefing_callout_reader: None,
            trajectory_reader: None,
            temporal_maintenance: None,
            composition_commit: None,
            entity_touchpoints_reader: None,
            entity_neighborhood_reader: None,
            meeting_prep_status_reader: None,
            meeting_prep_narrative_reader: None,
            claim_receipt_reader: None,
            account_composition_snapshot_reader: None,
            project_composition_snapshot_reader: None,
            person_composition_snapshot_reader: None,
            action_composition_snapshot_reader: None,
            meeting_composition_snapshot_reader: None,
            account_list_reader: None,
            person_list_reader: None,
            project_list_reader: None,
            markdown_preview_reader: None,
            workspace_graph_reader: None,
            source_management_ledger_reader: None,
            salience_reader: None,
            suggested_next_steps_reader: None,
            recommendation_feedback_writer: None,
            source_management_action_handler: None,
            workspace_intake: None,
            claim_file_operations: None,
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
            ability_id: None,
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            list_open_loops_reader: None,
            prepare_meeting_context_reader: None,
            daily_readiness_context_reader: None,
            briefing_callout_reader: None,
            trajectory_reader: None,
            temporal_maintenance: None,
            composition_commit: None,
            entity_touchpoints_reader: None,
            entity_neighborhood_reader: None,
            meeting_prep_status_reader: None,
            meeting_prep_narrative_reader: None,
            claim_receipt_reader: None,
            account_composition_snapshot_reader: None,
            project_composition_snapshot_reader: None,
            person_composition_snapshot_reader: None,
            action_composition_snapshot_reader: None,
            meeting_composition_snapshot_reader: None,
            account_list_reader: None,
            person_list_reader: None,
            project_list_reader: None,
            markdown_preview_reader: None,
            workspace_graph_reader: None,
            source_management_ledger_reader: None,
            salience_reader: None,
            suggested_next_steps_reader: None,
            recommendation_feedback_writer: None,
            source_management_action_handler: None,
            workspace_intake: None,
            claim_file_operations: None,
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
            ability_id: None,
            external,
            entity_context_reader: None,
            entity_context_claim_reader: None,
            list_open_loops_reader: None,
            prepare_meeting_context_reader: None,
            daily_readiness_context_reader: None,
            briefing_callout_reader: None,
            trajectory_reader: None,
            temporal_maintenance: None,
            composition_commit: None,
            entity_touchpoints_reader: None,
            entity_neighborhood_reader: None,
            meeting_prep_status_reader: None,
            meeting_prep_narrative_reader: None,
            claim_receipt_reader: None,
            account_composition_snapshot_reader: None,
            project_composition_snapshot_reader: None,
            person_composition_snapshot_reader: None,
            action_composition_snapshot_reader: None,
            meeting_composition_snapshot_reader: None,
            account_list_reader: None,
            person_list_reader: None,
            project_list_reader: None,
            markdown_preview_reader: None,
            workspace_graph_reader: None,
            source_management_ledger_reader: None,
            salience_reader: None,
            suggested_next_steps_reader: None,
            recommendation_feedback_writer: None,
            source_management_action_handler: None,
            workspace_intake: None,
            claim_file_operations: None,
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

    /// Attach the ability currently responsible for this service call.
    /// Mutation services use this as a fail-closed budget identity when
    /// proposal metadata omits the producer ability.
    pub fn with_ability_id(mut self, ability_id: &'a str) -> Self {
        self.ability_id = Some(ability_id);
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

    pub fn with_list_open_loops_reader(mut self, reader: Arc<dyn ListOpenLoopsReadHandle>) -> Self {
        self.list_open_loops_reader = Some(reader);
        self
    }

    pub fn with_prepare_meeting_context_reader(
        mut self,
        reader: Arc<dyn PrepareMeetingContextReadHandle>,
    ) -> Self {
        self.prepare_meeting_context_reader = Some(reader);
        self
    }

    pub fn with_daily_readiness_context_reader(
        mut self,
        reader: Arc<dyn DailyReadinessContextReadHandle>,
    ) -> Self {
        self.daily_readiness_context_reader = Some(reader);
        self
    }

    pub fn with_briefing_callout_reader(
        mut self,
        reader: Arc<dyn BriefingCalloutReadHandle>,
    ) -> Self {
        self.briefing_callout_reader = Some(reader);
        self
    }

    pub fn with_trajectory_reader(mut self, reader: Arc<dyn TrajectoryReadHandle>) -> Self {
        self.trajectory_reader = Some(reader);
        self
    }

    pub fn with_temporal_maintenance(
        mut self,
        maintenance: Arc<dyn TemporalMaintenanceHandle>,
    ) -> Self {
        self.temporal_maintenance = Some(maintenance);
        self
    }

    pub fn with_composition_commit_handle(
        mut self,
        finalizer: Arc<dyn CompositionCommitHandle>,
    ) -> Self {
        self.composition_commit = Some(finalizer);
        self
    }

    pub fn with_entity_touchpoints_reader(
        mut self,
        reader: Arc<dyn EntityTouchpointsReadHandle>,
    ) -> Self {
        self.entity_touchpoints_reader = Some(reader);
        self
    }

    pub fn with_entity_neighborhood_reader(
        mut self,
        reader: Arc<dyn EntityNeighborhoodReadHandle>,
    ) -> Self {
        self.entity_neighborhood_reader = Some(reader);
        self
    }

    pub fn with_meeting_prep_status_reader(
        mut self,
        reader: Arc<dyn MeetingPrepStatusReadHandle>,
    ) -> Self {
        self.meeting_prep_status_reader = Some(reader);
        self
    }

    pub fn with_meeting_prep_narrative_reader(
        mut self,
        reader: Arc<dyn MeetingPrepNarrativeReadHandle>,
    ) -> Self {
        self.meeting_prep_narrative_reader = Some(reader);
        self
    }

    pub fn with_claim_receipt_reader(mut self, reader: Arc<dyn ClaimReceiptReadHandle>) -> Self {
        self.claim_receipt_reader = Some(reader);
        self
    }

    pub fn with_account_composition_snapshot_reader(
        mut self,
        reader: Arc<dyn AccountCompositionSnapshotReadHandle>,
    ) -> Self {
        self.account_composition_snapshot_reader = Some(reader);
        self
    }

    pub fn with_project_composition_snapshot_reader(
        mut self,
        reader: Arc<dyn ProjectCompositionSnapshotReadHandle>,
    ) -> Self {
        self.project_composition_snapshot_reader = Some(reader);
        self
    }

    pub fn with_person_composition_snapshot_reader(
        mut self,
        reader: Arc<dyn PersonCompositionSnapshotReadHandle>,
    ) -> Self {
        self.person_composition_snapshot_reader = Some(reader);
        self
    }

    pub fn with_action_composition_snapshot_reader(
        mut self,
        reader: Arc<dyn ActionCompositionSnapshotReadHandle>,
    ) -> Self {
        self.action_composition_snapshot_reader = Some(reader);
        self
    }

    pub fn with_meeting_composition_snapshot_reader(
        mut self,
        reader: Arc<dyn MeetingCompositionSnapshotReadHandle>,
    ) -> Self {
        self.meeting_composition_snapshot_reader = Some(reader);
        self
    }

    pub fn with_account_list_reader(mut self, reader: Arc<dyn AccountListReadHandle>) -> Self {
        self.account_list_reader = Some(reader);
        self
    }

    pub fn with_person_list_reader(mut self, reader: Arc<dyn PersonListReadHandle>) -> Self {
        self.person_list_reader = Some(reader);
        self
    }

    pub fn with_project_list_reader(mut self, reader: Arc<dyn ProjectListReadHandle>) -> Self {
        self.project_list_reader = Some(reader);
        self
    }

    pub fn with_markdown_preview_reader(
        mut self,
        reader: Arc<dyn MarkdownPreviewReadHandle>,
    ) -> Self {
        self.markdown_preview_reader = Some(reader);
        self
    }

    pub fn with_workspace_graph_reader(
        mut self,
        reader: Arc<dyn WorkspaceGraphReadHandle>,
    ) -> Self {
        self.workspace_graph_reader = Some(reader);
        self
    }

    pub fn with_source_management_ledger_reader(
        mut self,
        reader: Arc<dyn SourceManagementLedgerReadHandle>,
    ) -> Self {
        self.source_management_ledger_reader = Some(reader);
        self
    }

    pub fn with_salience_reader(mut self, reader: Arc<dyn SalienceReadHandle>) -> Self {
        self.salience_reader = Some(reader);
        self
    }

    pub fn with_suggested_next_steps_reader(
        mut self,
        reader: Arc<dyn SuggestedNextStepsReadHandle>,
    ) -> Self {
        self.suggested_next_steps_reader = Some(reader);
        self
    }

    pub fn with_recommendation_feedback_writer(
        mut self,
        writer: Arc<dyn RecommendationFeedbackWriteHandle>,
    ) -> Self {
        self.recommendation_feedback_writer = Some(writer);
        self
    }

    pub fn with_source_management_action_handler(
        mut self,
        handler: Arc<dyn SourceManagementActionHandle>,
    ) -> Self {
        self.source_management_action_handler = Some(handler);
        self
    }

    pub fn with_workspace_intake(mut self, service: Arc<dyn WorkspaceIntakeService>) -> Self {
        self.workspace_intake = Some(service);
        self
    }

    pub fn with_claim_file_operations(
        mut self,
        operations: Arc<dyn ClaimFileOperationHandle>,
    ) -> Self {
        self.claim_file_operations = Some(operations);
        self
    }

    pub fn workspace_intake(&self) -> Option<&dyn WorkspaceIntakeService> {
        self.workspace_intake.as_deref()
    }

    /// Reader-backed touchpoint composition. When no reader is
    /// attached (test contexts, evaluate mode without fixtures) the caller
    /// receives a typed `ReadFailed` error and the producer falls back to a
    /// typed empty bundle. Subject isolation is the reader's responsibility —
    /// each returned snapshot carries `inclusion_reason` so the producer can
    /// project verbatim without recomputing the filter.
    pub async fn read_entity_touchpoints(
        &self,
        query: EntityTouchpointsQuery,
    ) -> Result<EntityTouchpointsSnapshot, EntityTouchpointsReadError> {
        let Some(reader) = &self.entity_touchpoints_reader else {
            return Err(EntityTouchpointsReadError::ReadFailed(
                self.missing_reader_error("entity_touchpoints_reader"),
            ));
        };
        reader.read_entity_touchpoints(query).await
    }

    /// Reader-backed generic relationship and participation evidence. Missing
    /// readers are surfaced as typed read failures so producers can render
    /// section caveats instead of silently treating absent readers as no data.
    pub async fn read_entity_neighborhood(
        &self,
        query: EntityNeighborhoodQuery,
    ) -> Result<EntityNeighborhoodSnapshot, EntityNeighborhoodReadError> {
        let Some(reader) = &self.entity_neighborhood_reader else {
            return Err(EntityNeighborhoodReadError::ReadFailed(
                self.missing_reader_error("entity_neighborhood_reader"),
            ));
        };
        reader.read_entity_neighborhood(query).await
    }

    /// Read per-meeting prep status. Returns
    /// `MeetingPrepStatusReadError::ReadFailed` with a typed missing-reader
    /// message when no adapter is attached (test contexts without fixtures);
    /// the briefing producer projects that into a typed `NeedsPreparation`
    /// state rather than failing the envelope.
    pub async fn read_meeting_prep_status(
        &self,
        meeting_id: String,
    ) -> Result<MeetingPrepStatusSnapshot, MeetingPrepStatusReadError> {
        let Some(reader) = &self.meeting_prep_status_reader else {
            return Err(MeetingPrepStatusReadError::ReadFailed(
                self.missing_reader_error("meeting_prep_status_reader"),
            ));
        };
        reader.read_meeting_prep_status(meeting_id).await
    }

    /// Read a meeting's prep-context narrative (the `prep_context_json`
    /// summary). Returns `Ok(None)` when the reader is attached but the meeting
    /// has no narrative, and a missing-reader `Err` when no adapter is attached
    /// (test contexts without fixtures); the briefing producer treats both as
    /// "no narrative" rather than failing the envelope.
    pub async fn read_meeting_prep_narrative(
        &self,
        meeting_id: String,
    ) -> Result<Option<String>, String> {
        let Some(reader) = &self.meeting_prep_narrative_reader else {
            return Err(self.missing_reader_error("meeting_prep_narrative_reader"));
        };
        reader.read_meeting_prep_narrative(meeting_id).await
    }

    /// Dispatch the `claim_receipt` ability through the app crate's
    /// `LiveClaimReceiptReader` adapter, which wraps
    /// `services::claim_receipt::render::render_receipt_for`. Returns a typed
    /// `ReadFailed` with a missing-reader message when no adapter is attached
    /// (test contexts without fixtures).
    pub async fn read_claim_receipt(
        &self,
        target: ClaimReceiptTarget,
        surface: ClaimReceiptSurfaceContext,
    ) -> Result<ClaimReceiptSnapshot, ClaimReceiptReadError> {
        let Some(reader) = &self.claim_receipt_reader else {
            return Err(ClaimReceiptReadError::ReadFailed(
                self.missing_reader_error("claim_receipt_reader"),
            ));
        };
        reader.read_claim_receipt(target, surface).await
    }

    pub async fn read_account_composition_snapshot(
        &self,
        account_id: String,
        surface: ClaimDismissalSurface,
    ) -> Result<AccountCompositionSnapshot, AccountCompositionSnapshotReadError> {
        let Some(reader) = &self.account_composition_snapshot_reader else {
            return Err(AccountCompositionSnapshotReadError::ReadFailed(
                self.missing_reader_error("account_composition_snapshot_reader"),
            ));
        };
        reader
            .read_account_composition_snapshot(account_id, surface)
            .await
    }

    pub async fn read_project_composition_snapshot(
        &self,
        project_id: String,
        surface: ClaimDismissalSurface,
    ) -> Result<ProjectCompositionSnapshot, ProjectCompositionSnapshotReadError> {
        let Some(reader) = &self.project_composition_snapshot_reader else {
            return Err(ProjectCompositionSnapshotReadError::ReadFailed(
                self.missing_reader_error("project_composition_snapshot_reader"),
            ));
        };
        reader
            .read_project_composition_snapshot(project_id, surface)
            .await
    }

    pub async fn read_person_composition_snapshot(
        &self,
        person_id: String,
        surface: ClaimDismissalSurface,
    ) -> Result<PersonCompositionSnapshot, PersonCompositionSnapshotReadError> {
        let Some(reader) = &self.person_composition_snapshot_reader else {
            return Err(PersonCompositionSnapshotReadError::ReadFailed(
                self.missing_reader_error("person_composition_snapshot_reader"),
            ));
        };
        reader
            .read_person_composition_snapshot(person_id, surface)
            .await
    }

    pub async fn read_action_composition_snapshot(
        &self,
        action_id: String,
        surface: ClaimDismissalSurface,
    ) -> Result<ActionCompositionSnapshot, ActionCompositionSnapshotReadError> {
        let Some(reader) = &self.action_composition_snapshot_reader else {
            return Err(ActionCompositionSnapshotReadError::ReadFailed(
                self.missing_reader_error("action_composition_snapshot_reader"),
            ));
        };
        reader
            .read_action_composition_snapshot(action_id, surface)
            .await
    }

    pub async fn read_meeting_composition_snapshot(
        &self,
        query: MeetingCompositionSnapshotQuery,
    ) -> Result<MeetingCompositionSnapshot, MeetingCompositionSnapshotReadError> {
        let Some(reader) = &self.meeting_composition_snapshot_reader else {
            return Err(MeetingCompositionSnapshotReadError::ReadFailed(
                self.missing_reader_error("meeting_composition_snapshot_reader"),
            ));
        };
        reader.read_meeting_composition_snapshot(query).await
    }

    pub async fn commit_composition(
        &self,
        proposal: CompositionProposal,
    ) -> Result<CommittedComposition, CompositionCommitError> {
        let Some(finalizer) = &self.composition_commit else {
            return Err(CompositionCommitError::Unavailable(
                self.missing_reader_error("composition_commit"),
            ));
        };

        finalizer
            .commit_composition(CompositionCommitRequest {
                proposal,
                actor: self.actor.to_string(),
                ability_id: self.ability_id.map(str::to_string),
            })
            .await
    }

    pub async fn read_prepare_meeting_context(
        &self,
        meeting_id: String,
    ) -> Result<PrepareMeetingContextSnapshot, String> {
        if let Some(reader) = &self.prepare_meeting_context_reader {
            return reader.read_prepare_meeting_context(meeting_id).await;
        }

        Err(self.missing_reader_error("prepare_meeting_context_reader"))
    }

    pub async fn read_daily_readiness_context(
        &self,
        workspace_scope: String,
        date: String,
        intent: MeetingsViewIntent,
    ) -> Result<DailyReadinessContextSnapshot, String> {
        if let Some(reader) = &self.daily_readiness_context_reader {
            return reader
                .read_daily_readiness_context(workspace_scope, date, intent)
                .await;
        }

        Err(self.missing_reader_error("daily_readiness_context_reader"))
    }

    pub async fn read_briefing_callouts_for_date(
        &self,
        workspace_scope: String,
        date: String,
        limit: usize,
    ) -> Result<Vec<BriefingCalloutSnapshot>, BriefingCalloutReadError> {
        let Some(reader) = &self.briefing_callout_reader else {
            return Err(BriefingCalloutReadError::ReadFailed(
                self.missing_reader_error("briefing_callout_reader"),
            ));
        };
        reader
            .read_briefing_callouts_for_date(workspace_scope, date, limit)
            .await
    }

    /// Read active entity-context claims for the caller's actual render context.
    /// The `surface` MUST match where the returned claims will be rendered or
    /// used as prompt input; passing a broader surface can resurface dismissed
    /// claims in narrower contexts such as briefing prep.
    pub async fn read_entity_context_claims(
        &self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> Result<Vec<IntelligenceClaim>, String> {
        if let Some(reader) = &self.entity_context_claim_reader {
            return reader
                .read_entity_context_claims(entity_type, entity_id, surface, depth)
                .await;
        }

        Err(self.missing_reader_error("entity_context_claim_reader"))
    }

    pub async fn read_entity_context_claims_limited(
        &self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> Result<Vec<IntelligenceClaim>, String> {
        if let Some(reader) = &self.entity_context_claim_reader {
            return reader
                .read_entity_context_claims_limited(entity_type, entity_id, surface, depth, limit)
                .await;
        }

        Err(self.missing_reader_error("entity_context_claim_reader"))
    }

    pub async fn read_entity_context_prompt_claims_limited(
        &self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
        limit: usize,
    ) -> Result<Vec<IntelligenceClaim>, String> {
        if let Some(reader) = &self.entity_context_claim_reader {
            return reader
                .read_entity_context_prompt_claims_limited(
                    entity_type,
                    entity_id,
                    surface,
                    depth,
                    limit,
                )
                .await;
        }

        Err(self.missing_reader_error("entity_context_claim_reader"))
    }

    pub async fn read_list_open_loops(
        &self,
        query: ListOpenLoopsQuery,
    ) -> Result<ListOpenLoopsSnapshot, ListOpenLoopsReadError> {
        let Some(reader) = &self.list_open_loops_reader else {
            return Err(ListOpenLoopsReadError::ReadFailed(
                self.missing_reader_error("list_open_loops_read"),
            ));
        };

        reader.read_open_loops(query).await
    }

    pub async fn read_list_accounts(
        &self,
        query: AccountListQuery,
    ) -> Result<AccountListSnapshot, AccountListReadError> {
        let Some(reader) = &self.account_list_reader else {
            return Err(AccountListReadError::ReadFailed(
                self.missing_reader_error("list_accounts_read"),
            ));
        };

        reader.read_accounts(query).await
    }

    pub async fn read_list_people(
        &self,
        query: PersonListQuery,
    ) -> Result<PersonListSnapshot, PersonListReadError> {
        let Some(reader) = &self.person_list_reader else {
            return Err(PersonListReadError::ReadFailed(
                self.missing_reader_error("list_people_read"),
            ));
        };

        reader.read_people(query).await
    }

    pub async fn read_list_projects(
        &self,
        query: ProjectListQuery,
    ) -> Result<ProjectListSnapshot, ProjectListReadError> {
        let Some(reader) = &self.project_list_reader else {
            return Err(ProjectListReadError::ReadFailed(
                self.missing_reader_error("list_projects_read"),
            ));
        };

        reader.read_projects(query).await
    }

    pub async fn read_markdown_preview(
        &self,
        request: MarkdownPreviewReadRequest,
    ) -> Result<MarkdownPreviewOutput, MarkdownPreviewReadError> {
        let Some(reader) = &self.markdown_preview_reader else {
            return Err(MarkdownPreviewReadError::SourceUnavailable(
                self.missing_reader_error("markdown_preview_read"),
            ));
        };

        reader.read_markdown_preview(request).await
    }

    pub async fn read_workspace_graph(
        &self,
        request: WorkspaceGraphReadRequest,
    ) -> Result<WorkspaceGraphResponse, WorkspaceGraphReadError> {
        let Some(reader) = &self.workspace_graph_reader else {
            return Err(WorkspaceGraphReadError::ReadFailed(
                self.missing_reader_error("workspace_graph_read"),
            ));
        };

        reader.read_workspace_graph(request).await
    }

    pub async fn read_source_management_ledger(
        &self,
        request: SourceManagementLedgerReadRequest,
    ) -> Result<SourceManagementLedgerResponse, SourceManagementLedgerReadError> {
        let Some(reader) = &self.source_management_ledger_reader else {
            return Err(SourceManagementLedgerReadError::ReadFailed(
                self.missing_reader_error("source_management_ledger_read"),
            ));
        };

        reader.read_source_management_ledger(request).await
    }

    pub async fn score_salience(
        &self,
        request: ScoreSalienceReadRequest,
    ) -> Result<ScoreSalienceResponse, SalienceReadError> {
        let Some(reader) = &self.salience_reader else {
            return Err(SalienceReadError::ReadFailed(
                self.missing_reader_error("salience_read"),
            ));
        };

        reader.score_salience(request).await
    }

    pub async fn list_suggested_next_steps(
        &self,
        input: ListSuggestedNextStepsInput,
        actor: ActorKind,
    ) -> Result<ListSuggestedNextStepsResponse, SuggestedNextStepsReadError> {
        let Some(reader) = &self.suggested_next_steps_reader else {
            return Err(SuggestedNextStepsReadError::ReadFailed(
                self.missing_reader_error("suggested_next_steps_read"),
            ));
        };

        reader.list_suggested_next_steps(input, actor).await
    }

    pub async fn submit_recommendation_feedback(
        &self,
        input: SubmitRecommendationFeedbackInput,
        actor: Actor,
    ) -> Result<SubmitRecommendationFeedbackResponse, SubmitRecommendationFeedbackError> {
        let Some(writer) = &self.recommendation_feedback_writer else {
            return Err(SubmitRecommendationFeedbackError::WriteFailed(
                self.missing_reader_error("recommendation_feedback_write"),
            ));
        };

        writer
            .submit_recommendation_feedback(RecommendationFeedbackWriteRequest {
                input,
                actor,
                mode: self.mode,
                recorded_at: self.clock.now(),
                ability_id: self.ability_id.map(str::to_string),
            })
            .await
    }

    pub async fn apply_source_management_action(
        &self,
        request: SourceManagementActionRequest,
    ) -> Result<SourceManagementActionReceipt, SourceManagementActionError> {
        let Some(handler) = &self.source_management_action_handler else {
            return Err(SourceManagementActionError::ActionFailed(
                self.missing_reader_error("source_management_action"),
            ));
        };

        handler.apply_source_management_action(request).await
    }

    pub async fn render_entity_claim_file(
        &self,
        request: ClaimFileRenderRequest,
    ) -> Result<ClaimFileProjectionResult, ClaimFileOperationError> {
        let Some(operations) = &self.claim_file_operations else {
            return Err(ClaimFileOperationError::OperationFailed(
                self.missing_reader_error("claim_file_operations"),
            ));
        };

        operations.render_entity_claim_file(request).await
    }

    pub async fn apply_claim_file_corrections(
        &self,
        request: ClaimFileApplyRequest,
    ) -> Result<ClaimFileApplyResult, ClaimFileOperationError> {
        let Some(operations) = &self.claim_file_operations else {
            return Err(ClaimFileOperationError::OperationFailed(
                self.missing_reader_error("claim_file_operations"),
            ));
        };

        operations.apply_claim_file_corrections(request).await
    }

    pub async fn read_trajectory_bundle(
        &self,
        entity_type: String,
        entity_id: String,
        depth: TrajectoryQueryDepth,
    ) -> Result<TrajectoryBundle, String> {
        if matches!(depth, TrajectoryQueryDepth::None) {
            return Ok(TrajectoryBundle::default());
        }

        if let Some(reader) = &self.trajectory_reader {
            return reader
                .read_trajectory_bundle(entity_type, entity_id, depth, self.clock.now())
                .await;
        }

        Ok(TrajectoryBundle::default())
    }

    pub async fn refresh_engagement_curve(
        &self,
        input: RefreshEngagementCurveInput,
        computed_at: DateTime<Utc>,
    ) -> Result<RefreshEngagementCurveResult, String> {
        if let Some(maintenance) = &self.temporal_maintenance {
            return maintenance
                .refresh_engagement_curve(input, computed_at)
                .await;
        }

        Err(self.missing_reader_error("temporal_maintenance"))
    }

    pub async fn detect_role_change(
        &self,
        input: DetectRoleChangeInput,
        computed_at: DateTime<Utc>,
    ) -> Result<DetectRoleChangeResult, String> {
        if let Some(maintenance) = &self.temporal_maintenance {
            return maintenance.detect_role_change(input, computed_at).await;
        }

        Err(self.missing_reader_error("temporal_maintenance"))
    }

    pub async fn read_entity_context_claim_entries(
        &self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> Result<Vec<EntityContextEntry>, String> {
        self.read_entity_context_claims(entity_type, entity_id, surface, depth)
            .await?
            .into_iter()
            .map(entity_context_entry_for_claim)
            .collect()
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

        Err(self.missing_reader_error("entity_context_reader"))
    }

    /// Test-only `Live` constructor.
    pub fn test_live(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self::new_live(clock, rng, external)
    }

    /// Test-only `Evaluate` constructor.
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

    fn missing_reader_error(&self, reader: &'static str) -> String {
        ServiceError::FixtureReaderRequired {
            mode: self.mode,
            reader,
        }
        .to_string()
    }
}

fn entity_context_entry_for_claim(claim: IntelligenceClaim) -> Result<EntityContextEntry, String> {
    let value: serde_json::Value = serde_json::from_str(&claim.subject_ref)
        .map_err(|error| format!("Invalid entity context claim subject_ref JSON: {error}"))?;
    let (entity_type, entity_id) = match subject_ref_from_json(&value)
        .map_err(|error| format!("Invalid entity context claim subject_ref: {error}"))?
    {
        ClaimSubjectRef::Account { id } => ("account".to_string(), id),
        ClaimSubjectRef::Person { id } => ("person".to_string(), id),
        ClaimSubjectRef::Project { id } => ("project".to_string(), id),
        ClaimSubjectRef::Meeting { id } => ("meeting".to_string(), id),
        ClaimSubjectRef::Action { .. }
        | ClaimSubjectRef::Email { .. }
        | ClaimSubjectRef::Multi(_)
        | ClaimSubjectRef::Global => {
            return Err(format!(
                "Claim `{}` has unsupported entity context subject",
                claim.id
            ));
        }
    };

    let updated_at = claim
        .reactivated_at
        .clone()
        .unwrap_or_else(|| claim.created_at.clone());
    let title = match claim
        .field_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(field_path) => format!("{}: {field_path}", claim.claim_type),
        None => claim.claim_type.clone(),
    };
    let actor = RenderActor::user("user", Some("user"));
    let rendered_title = renderable_entity_context_text(&claim, &title, &actor)?;
    let rendered_content = renderable_entity_context_text(&claim, &claim.text, &actor)?;

    Ok(EntityContextEntry {
        id: claim.id,
        entity_type,
        entity_id,
        title: rendered_title,
        content: rendered_content,
        created_at: claim.created_at,
        updated_at,
    })
}

fn renderable_entity_context_text(
    claim: &IntelligenceClaim,
    value: &str,
    actor: &RenderActor,
) -> Result<EntityContextText, String> {
    renderable_claim_text_with_value(claim, value, RenderSurface::TauriEntityDetail, actor)
        .map(EntityContextText::Claim)
        .ok_or_else(|| format!("Claim `{}` cannot render for entity context", claim.id))
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
