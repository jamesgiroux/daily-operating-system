//! Fail-improve loop capture for deterministic-first classifiers.
//!
//! The loop is intentionally app-service owned, not ability-runtime owned. It
//! captures LLM fallback examples for human review and later deterministic test
//! promotion without mutating classifier rules automatically.

use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use uuid::Uuid;

#[cfg(test)]
use crate::intelligence::provider::Completion;
use crate::intelligence::provider::{IntelligenceProvider, ModelTier, PromptInput, ProviderError};
use crate::services::context::ServiceContext;
use crate::signals::policy_registry::PayloadPrivacy;
use crate::signals::policy_registry::SignalType;

const MAX_JSONL_ENTRIES: usize = 1_000;
const TEST_CASE_SOURCE: &str = "fail_improve_loop";
const SIGNAL_TYPE_RESOLUTION_WAIT: Duration = Duration::from_millis(100);
pub const UNKNOWN_SIGNAL_TYPES_OPERATION: &str = "unknown_signal_types";

static FAIL_IMPROVE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static UNKNOWN_SIGNAL_OBSERVATION_TX: OnceLock<
    Mutex<Option<UnboundedSender<UnknownSignalObservation>>>,
> = OnceLock::new();
static SIGNAL_TYPE_RESOLUTION_CACHE: OnceLock<RwLock<HashMap<String, SignalType>>> =
    OnceLock::new();
static SIGNAL_TYPE_RESOLUTION_IN_FLIGHT: OnceLock<SignalTypeResolutionInFlight> = OnceLock::new();

type SignalTypeResolutionInFlight = Arc<Mutex<HashMap<String, Vec<oneshot::Sender<SignalType>>>>>;

#[cfg(test)]
static SIGNAL_TYPE_LLM_CALLS_FOR_TEST: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(test)]
static SIGNAL_TYPE_LLM_DELAY_MS_FOR_TEST: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn fail_improve_lock() -> &'static Mutex<()> {
    FAIL_IMPROVE_LOCK.get_or_init(|| Mutex::new(()))
}

fn unknown_signal_observation_sender(
) -> &'static Mutex<Option<UnboundedSender<UnknownSignalObservation>>> {
    UNKNOWN_SIGNAL_OBSERVATION_TX.get_or_init(|| Mutex::new(None))
}

fn signal_type_resolution_cache() -> &'static RwLock<HashMap<String, SignalType>> {
    SIGNAL_TYPE_RESOLUTION_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn signal_type_resolution_in_flight() -> &'static SignalTypeResolutionInFlight {
    SIGNAL_TYPE_RESOLUTION_IN_FLIGHT.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

pub fn install_unknown_signal_observation_worker(
) -> Option<UnboundedReceiver<UnknownSignalObservation>> {
    let mut sender = unknown_signal_observation_sender().lock();
    if sender.is_some() {
        return None;
    }

    let (tx, rx) = mpsc::unbounded_channel();
    *sender = Some(tx);
    Some(rx)
}

pub async fn run_unknown_signal_observation_worker(
    mut receiver: UnboundedReceiver<UnknownSignalObservation>,
) {
    while let Some(observation) = receiver.recv().await {
        match tokio::task::spawn_blocking(move || {
            FailImproveLoop::default().record_unknown_signal_observation(observation)
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(err)) => log::warn!("unknown signal observation write failed: {err}"),
            Err(err) => log::warn!("unknown signal observation worker task failed: {err}"),
        }
    }
}

pub fn enqueue_unknown_signal_observation(observation: UnknownSignalObservation) {
    let sender = unknown_signal_observation_sender().lock().clone();
    let Some(sender) = sender else {
        return;
    };

    if sender.send(observation).is_err() {
        log::debug!("unknown signal observation dropped because worker channel is closed");
    }
}

#[cfg(test)]
pub(crate) fn replace_unknown_signal_observation_sender_for_test(
    sender: Option<UnboundedSender<UnknownSignalObservation>>,
) -> Option<UnboundedSender<UnknownSignalObservation>> {
    std::mem::replace(&mut *unknown_signal_observation_sender().lock(), sender)
}

#[cfg(test)]
pub(crate) fn reset_signal_type_resolution_state_for_test() {
    signal_type_resolution_cache().write().clear();
    signal_type_resolution_in_flight().lock().clear();
    SIGNAL_TYPE_LLM_CALLS_FOR_TEST.store(0, std::sync::atomic::Ordering::SeqCst);
    SIGNAL_TYPE_LLM_DELAY_MS_FOR_TEST.store(0, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) fn signal_type_llm_calls_for_test() -> usize {
    SIGNAL_TYPE_LLM_CALLS_FOR_TEST.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
pub(crate) fn set_signal_type_llm_delay_for_test(delay: Duration) {
    SIGNAL_TYPE_LLM_DELAY_MS_FOR_TEST.store(
        delay.as_millis().try_into().unwrap_or(u64::MAX),
        std::sync::atomic::Ordering::SeqCst,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum FailImproveError {
    #[error("failed to serialize fail-improve data: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("fail-improve I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid fail-improve operation `{0}`")]
    InvalidOperation(String),
    #[error("deterministic classifier failed: {0}")]
    Deterministic(String),
    #[error("LLM fallback failed: {0}")]
    Llm(String),
}

pub type Result<T> = std::result::Result<T, FailImproveError>;

#[derive(Debug, Clone)]
pub struct FailImproveLoop {
    root: PathBuf,
}

impl Default for FailImproveLoop {
    fn default() -> Self {
        Self {
            root: default_fail_improve_root(),
        }
    }
}

impl FailImproveLoop {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> PathBuf {
        default_fail_improve_root()
    }

    /// Resolve registry signal typing through the fail-improve loop without
    /// letting unknown signal emission block on the full LLM budget.
    pub fn resolve_signal_type(name: &str) -> SignalType {
        let name = name.trim();
        if let Some(signal) = signal_type_resolution_cache().read().get(name).cloned() {
            return signal;
        }

        let deterministic = SignalType::from_name(name);
        if !matches!(deterministic, SignalType::Legacy { .. }) {
            signal_type_resolution_cache()
                .write()
                .insert(name.to_string(), deterministic.clone());
            return deterministic;
        }

        let (tx, mut rx) = oneshot::channel();
        let should_spawn = {
            let mut in_flight = signal_type_resolution_in_flight().lock();
            if let Some(signal) = signal_type_resolution_cache().read().get(name).cloned() {
                return signal;
            }

            if let Some(waiters) = in_flight.get_mut(name) {
                waiters.push(tx);
                false
            } else {
                in_flight.insert(name.to_string(), vec![tx]);
                true
            }
        };

        if should_spawn {
            Self::spawn_signal_type_resolution(name.to_string());
        }

        let started_at = Instant::now();
        loop {
            match rx.try_recv() {
                Ok(signal) => return signal,
                Err(oneshot::error::TryRecvError::Closed) => return Self::legacy_signal_type(name),
                Err(oneshot::error::TryRecvError::Empty) => {
                    if started_at.elapsed() >= SIGNAL_TYPE_RESOLUTION_WAIT {
                        return Self::legacy_signal_type(name);
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    fn spawn_signal_type_resolution(name: String) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            drop(handle.spawn(async move {
                Self::finish_signal_type_resolution(name).await;
            }));
            return;
        }

        let fallback_name = name.clone();
        match std::thread::Builder::new()
            .name("signal-type-resolution".to_string())
            .spawn(move || {
                match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime.block_on(Self::finish_signal_type_resolution(name)),
                    Err(err) => {
                        log::warn!("signal type resolution runtime failed: {err}");
                        Self::complete_signal_type_resolution(
                            name.clone(),
                            Self::legacy_signal_type(&name),
                        );
                    }
                }
            }) {
            Ok(join_handle) => drop(join_handle),
            Err(err) => {
                log::warn!("signal type resolution thread failed to spawn: {err}");
                Self::complete_signal_type_resolution(
                    fallback_name.clone(),
                    Self::legacy_signal_type(&fallback_name),
                );
            }
        }
    }

    async fn finish_signal_type_resolution(name: String) {
        let resolved = Self::resolve_signal_type_with_llm(&name).await;
        Self::complete_signal_type_resolution(name, resolved);
    }

    async fn resolve_signal_type_with_llm(name: &str) -> SignalType {
        let input = SignalTypingInput {
            signal_type: name.to_string(),
        };
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ext = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &ext);

        match classify_registry_signal_type_with_llm(&input, &ctx).await {
            Ok(signal_type) => SignalType::from_name(&signal_type),
            Err(err) => {
                log::warn!("signal type LLM fallback failed for `{name}`: {err}");
                Self::legacy_signal_type(name)
            }
        }
    }

    fn complete_signal_type_resolution(name: String, signal: SignalType) {
        signal_type_resolution_cache()
            .write()
            .insert(name.clone(), signal.clone());

        let waiters = signal_type_resolution_in_flight()
            .lock()
            .remove(&name)
            .unwrap_or_default();
        for waiter in waiters {
            drop(waiter.send(signal.clone()));
        }
    }

    fn legacy_signal_type(name: &str) -> SignalType {
        SignalType::Legacy {
            name: name.to_string(),
        }
    }

    /// Run a deterministic classifier first. When it returns `None`, call the
    /// LLM fallback, persist the fallback example, update stats, and return the
    /// LLM result.
    pub async fn execute<I, T, D, DFut, L, LFut>(
        &self,
        operation: &str,
        input: I,
        deterministic: D,
        llm_fallback: L,
        ctx: &ServiceContext<'_>,
    ) -> Result<T>
    where
        I: Serialize,
        T: Serialize,
        D: FnOnce(&I) -> DFut,
        DFut: Future<Output = Result<Option<T>>>,
        L: FnOnce(&I) -> LFut,
        LFut: Future<Output = Result<T>>,
    {
        let operation = operation_file_stem(operation)?;
        if let Some(result) = deterministic(&input).await? {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "fail-improve telemetry must never block the classifier result"
            )]
            let _ = self
                .record_stats(&operation, true, ctx)
                .map_err(|err| log::error!("fail-improve stats write failed: {err}"));
            return Ok(result);
        }

        let result = llm_fallback(&input).await?;
        let entry = FailImproveEntry {
            ts: ctx.clock.now().to_rfc3339(),
            operation: operation.clone(),
            input: serde_json::to_value(&input)?,
            deterministic_result: None,
            llm_result: serde_json::to_value(&result)?,
            invocation_id: format!("fi-{:016x}", ctx.rng.random_u64()),
        };

        #[allow(
            clippy::let_underscore_must_use,
            reason = "fail-improve telemetry is best-effort; callers still need the LLM classification"
        )]
        let _ = self
            .record_fallback_artifacts(&operation, &entry, ctx)
            .map_err(|err| log::error!("fail-improve fallback artifact write failed: {err}"));
        Ok(result)
    }

    pub fn generate_test_cases(&self, operation: &str) -> Result<Vec<TestCase>> {
        let operation = operation_file_stem(operation)?;
        let Some(raw) = self.read_artifact_to_string(&operation, ArtifactKind::Jsonl)? else {
            return Ok(Vec::new());
        };

        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let entry: FailImproveEntry = serde_json::from_str(line)?;
                Ok(TestCase {
                    operation: entry.operation,
                    input: entry.input,
                    expected: entry.llm_result,
                    invocation_id: entry.invocation_id,
                    ts: entry.ts,
                    source: TEST_CASE_SOURCE.to_string(),
                })
            })
            .collect()
    }

    fn record_unknown_signal_observation(
        &self,
        observation: UnknownSignalObservation,
    ) -> Result<()> {
        if !captures_unknown_signal_payload(observation.payload_privacy) {
            return Ok(());
        }

        let operation = operation_file_stem(UNKNOWN_SIGNAL_TYPES_OPERATION)?;
        let now = Utc::now().to_rfc3339();
        let entry = UnknownSignalTypeObservation {
            ts: now.clone(),
            operation: operation.clone(),
            signal_type: observation.signal_type,
            payload: observation.payload,
            invocation_id: format!("fi-{}", Uuid::new_v4().simple()),
        };

        let _guard = fail_improve_lock().lock();
        std::fs::create_dir_all(&self.root)?;
        let (counts, trend) = self.updated_counts_at_unlocked(&operation, false, now)?;
        let writes = self.fallback_artifact_writes(&operation, &entry, &counts, &trend)?;
        write_artifacts_atomically(writes)
    }

    pub fn diagnostics(&self) -> Result<Vec<FailImproveOperationDiagnostics>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut diagnostics = Vec::new();
        for operation in self.operation_names()? {
            let Some(raw) = self.read_artifact_to_string(&operation, ArtifactKind::Counts)? else {
                continue;
            };
            let counts: FailImproveCounts = serde_json::from_str(&raw)?;
            diagnostics.push(FailImproveOperationDiagnostics {
                operation: operation.clone(),
                total_calls: counts.total_calls,
                deterministic_hits: counts.deterministic_hits,
                llm_fallbacks: counts.llm_fallbacks,
                deterministic_rate: counts.deterministic_rate,
                updated_at: counts.updated_at.clone(),
                trend: self.read_trend(&operation)?,
            });
        }

        diagnostics.sort_by(|a, b| a.operation.cmp(&b.operation));
        Ok(diagnostics)
    }

    fn fallback_artifact_writes<T: Serialize>(
        &self,
        operation: &str,
        entry: &T,
        counts: &FailImproveCounts,
        trend: &FailImproveTrendPoint,
    ) -> Result<Vec<PendingArtifactWrite>> {
        let jsonl_bytes =
            self.append_jsonl_artifact_value_bytes(operation, ArtifactKind::Jsonl, entry)?;
        let mut writes = self.stats_artifact_writes(operation, counts, trend)?;
        writes.insert(
            0,
            PendingArtifactWrite {
                path: self.artifact_write_path(operation, ArtifactKind::Jsonl),
                bytes: jsonl_bytes,
            },
        );
        Ok(writes)
    }

    fn record_fallback_artifacts(
        &self,
        operation: &str,
        entry: &FailImproveEntry,
        ctx: &ServiceContext<'_>,
    ) -> Result<()> {
        let _guard = fail_improve_lock().lock();
        std::fs::create_dir_all(&self.root)?;

        let (counts, trend) = self.updated_counts_unlocked(operation, false, ctx)?;
        let writes = self.fallback_artifact_writes(operation, entry, &counts, &trend)?;
        write_artifacts_atomically(writes)
    }

    fn record_stats(
        &self,
        operation: &str,
        deterministic_hit: bool,
        ctx: &ServiceContext<'_>,
    ) -> Result<()> {
        let _guard = fail_improve_lock().lock();
        std::fs::create_dir_all(&self.root)?;
        let (counts, trend) = self.updated_counts_unlocked(operation, deterministic_hit, ctx)?;
        let writes = self.stats_artifact_writes(operation, &counts, &trend)?;
        write_artifacts_atomically(writes)
    }

    fn updated_counts_unlocked(
        &self,
        operation: &str,
        deterministic_hit: bool,
        ctx: &ServiceContext<'_>,
    ) -> Result<(FailImproveCounts, FailImproveTrendPoint)> {
        self.updated_counts_at_unlocked(operation, deterministic_hit, ctx.clock.now().to_rfc3339())
    }

    fn updated_counts_at_unlocked(
        &self,
        operation: &str,
        deterministic_hit: bool,
        updated_at: String,
    ) -> Result<(FailImproveCounts, FailImproveTrendPoint)> {
        let mut counts =
            if let Some(raw) = self.read_artifact_to_string(operation, ArtifactKind::Counts)? {
                serde_json::from_str::<FailImproveCounts>(&raw)?
            } else {
                FailImproveCounts::default()
            };

        counts.total_calls += 1;
        if deterministic_hit {
            counts.deterministic_hits += 1;
        } else {
            counts.llm_fallbacks += 1;
        }
        counts.deterministic_rate = if counts.total_calls == 0 {
            0.0
        } else {
            counts.deterministic_hits as f64 / counts.total_calls as f64
        };
        counts.updated_at = updated_at;

        let trend = FailImproveTrendPoint {
            ts: counts.updated_at.clone(),
            total_calls: counts.total_calls,
            deterministic_hits: counts.deterministic_hits,
            llm_fallbacks: counts.llm_fallbacks,
            deterministic_rate: counts.deterministic_rate,
        };
        Ok((counts, trend))
    }

    fn stats_artifact_writes(
        &self,
        operation: &str,
        counts: &FailImproveCounts,
        trend: &FailImproveTrendPoint,
    ) -> Result<Vec<PendingArtifactWrite>> {
        let bytes = serde_json::to_vec_pretty(counts)?;
        let trend_bytes =
            self.append_jsonl_artifact_value_bytes(operation, ArtifactKind::Trend, trend)?;
        Ok(vec![
            PendingArtifactWrite {
                path: self.artifact_write_path(operation, ArtifactKind::Counts),
                bytes,
            },
            PendingArtifactWrite {
                path: self.artifact_write_path(operation, ArtifactKind::Trend),
                bytes: trend_bytes,
            },
        ])
    }

    fn read_trend(&self, operation: &str) -> Result<Vec<FailImproveTrendPoint>> {
        let Some(raw) = self.read_artifact_to_string(operation, ArtifactKind::Trend)? else {
            return Ok(Vec::new());
        };
        let mut points = raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<FailImproveTrendPoint>(line).ok())
            .collect::<Vec<_>>();
        if points.len() > 20 {
            points = points[points.len() - 20..].to_vec();
        }
        Ok(points)
    }

    fn operation_names(&self) -> Result<Vec<String>> {
        let mut operations = BTreeSet::new();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Some(operation) = name.strip_suffix(".current") {
                operations.insert(operation.to_string());
            } else if let Some(operation) = name.strip_suffix(".counts.json") {
                operations.insert(operation.to_string());
            }
        }
        Ok(operations.into_iter().collect())
    }

    fn append_jsonl_artifact_value_bytes<T: Serialize>(
        &self,
        operation: &str,
        kind: ArtifactKind,
        value: &T,
    ) -> Result<Vec<u8>> {
        let mut lines = self.read_jsonl_artifact_lines(operation, kind)?;
        if lines.len() >= MAX_JSONL_ENTRIES {
            lines = lines[lines.len() + 1 - MAX_JSONL_ENTRIES..].to_vec();
        }
        lines.push(serde_json::to_string(value)?);
        Ok(jsonl_lines_bytes(&lines))
    }

    fn read_jsonl_artifact_lines(
        &self,
        operation: &str,
        kind: ArtifactKind,
    ) -> Result<Vec<String>> {
        let Some(raw) = self.read_artifact_to_string(operation, kind)? else {
            return Ok(Vec::new());
        };
        Ok(raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect())
    }

    fn read_artifact_to_string(
        &self,
        operation: &str,
        kind: ArtifactKind,
    ) -> Result<Option<String>> {
        if let Some(path) = self.current_bundle_artifact_path(operation, kind)? {
            if path.exists() {
                return Ok(Some(std::fs::read_to_string(path)?));
            }
        }

        let legacy_path = self.artifact_write_path(operation, kind);
        if legacy_path.exists() {
            return Ok(Some(std::fs::read_to_string(legacy_path)?));
        }

        Ok(None)
    }

    fn current_bundle_artifact_path(
        &self,
        operation: &str,
        kind: ArtifactKind,
    ) -> Result<Option<PathBuf>> {
        let pointer_path = self.bundle_pointer_path(operation);
        if !pointer_path.exists() {
            return Ok(None);
        }

        let bundle_id = std::fs::read_to_string(pointer_path)?.trim().to_string();
        if !valid_bundle_id(&bundle_id) {
            return Err(FailImproveError::InvalidOperation(bundle_id));
        }

        let bundle_dir = self.bundle_dir(operation).join(bundle_id);
        if !bundle_dir.is_dir() {
            return Ok(None);
        }

        Ok(Some(bundle_dir.join(artifact_file_name(operation, kind))))
    }

    fn artifact_write_path(&self, operation: &str, kind: ArtifactKind) -> PathBuf {
        self.root.join(artifact_file_name(operation, kind))
    }

    fn bundle_dir(&self, operation: &str) -> PathBuf {
        self.root.join(format!("{operation}.bundles"))
    }

    fn bundle_pointer_path(&self, operation: &str) -> PathBuf {
        self.root.join(format!("{operation}.current"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailImproveEntry {
    pub ts: String,
    pub operation: String,
    pub input: Value,
    pub deterministic_result: Option<Value>,
    pub llm_result: Value,
    pub invocation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownSignalTypeObservation {
    pub ts: String,
    pub operation: String,
    pub signal_type: String,
    pub payload: Value,
    pub invocation_id: String,
}

#[derive(Debug, Clone)]
pub struct UnknownSignalObservation {
    pub signal_type: String,
    pub payload: Value,
    pub payload_privacy: PayloadPrivacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub operation: String,
    pub input: Value,
    pub expected: Value,
    pub invocation_id: String,
    pub ts: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FailImproveCounts {
    pub total_calls: u64,
    pub deterministic_hits: u64,
    pub llm_fallbacks: u64,
    pub deterministic_rate: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailImproveOperationDiagnostics {
    pub operation: String,
    pub total_calls: u64,
    pub deterministic_hits: u64,
    pub llm_fallbacks: u64,
    pub deterministic_rate: f64,
    pub updated_at: String,
    pub trend: Vec<FailImproveTrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailImproveTrendPoint {
    pub ts: String,
    pub total_calls: u64,
    pub deterministic_hits: u64,
    pub llm_fallbacks: u64,
    pub deterministic_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalTypingInput {
    pub signal_type: String,
}

pub const SIGNAL_TYPING_OPERATION: &str = "signal_typing";

/// Complete ADR-0115 registry signal typing at the IntelligenceProvider boundary.
///
/// The deterministic registry check runs before provider completion. A registry
/// miss calls `IntelligenceProvider::complete`, parses the provider response,
/// and records the miss as a fail-improve artifact.
pub async fn complete_registry_signal_typing<P>(
    provider: &P,
    input: SignalTypingInput,
    prompt: PromptInput,
    tier: ModelTier,
    ctx: &ServiceContext<'_>,
) -> Result<String>
where
    P: IntelligenceProvider + ?Sized,
{
    let loop_ = FailImproveLoop::default();
    complete_registry_signal_typing_with_loop(&loop_, provider, input, prompt, tier, ctx).await
}

async fn complete_registry_signal_typing_with_loop<P>(
    loop_: &FailImproveLoop,
    provider: &P,
    input: SignalTypingInput,
    prompt: PromptInput,
    tier: ModelTier,
    ctx: &ServiceContext<'_>,
) -> Result<String>
where
    P: IntelligenceProvider + ?Sized,
{
    loop_
        .execute(
            SIGNAL_TYPING_OPERATION,
            input,
            |input| {
                let result = deterministic_registry_signal_type(input);
                async move { Ok(result) }
            },
            move |input| {
                let original = input.signal_type.clone();
                async move {
                    let completion = provider
                        .complete(prompt, tier)
                        .await
                        .map_err(provider_error_to_fail_improve)?;
                    parse_registry_signal_typing_response(&original, &completion.text)
                }
            },
            ctx,
        )
        .await
}

/// Legacy direct-fallback adapter. Registry signal typing fail-improve wrapping
/// lives at `IntelligenceProvider::complete` via `complete_registry_signal_typing`.
pub async fn execute_registry_signal_typing<L, LFut>(
    input: SignalTypingInput,
    llm_fallback: L,
    _ctx: &ServiceContext<'_>,
) -> Result<String>
where
    L: FnOnce(&SignalTypingInput) -> LFut,
    LFut: Future<Output = Result<String>>,
{
    llm_fallback(&input).await
}

pub fn record_unknown_signal_type(
    signal_type: &str,
    payload: Value,
    payload_privacy: PayloadPrivacy,
) {
    enqueue_unknown_signal_observation(UnknownSignalObservation {
        signal_type: signal_type.to_string(),
        payload,
        payload_privacy,
    });
}

/// ADR-0115 registry-classifier deterministic arm. It returns `None` for
/// registry misses so callers can route misses through `FailImproveLoop`.
pub fn deterministic_registry_signal_type(input: &SignalTypingInput) -> Option<String> {
    let signal = crate::signals::policy_registry::SignalType::from_name(input.signal_type.trim());
    if matches!(
        signal,
        crate::signals::policy_registry::SignalType::Legacy { .. }
    ) {
        None
    } else {
        Some(signal.canonical_name().to_string())
    }
}

#[cfg(not(test))]
pub async fn classify_registry_signal_type_with_llm(
    input: &SignalTypingInput,
    ctx: &ServiceContext<'_>,
) -> Result<String> {
    let config = crate::state::load_config().ok();
    let ai_models = config
        .as_ref()
        .map(|config| config.ai_models.clone())
        .unwrap_or_default();
    let workspace = config
        .as_ref()
        .map(|config| PathBuf::from(&config.workspace_path))
        .filter(|path| path.exists())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let provider = crate::intelligence::pty_provider::PtyClaudeCode::new(
        Arc::new(ai_models),
        workspace.clone(),
        crate::pty::AiUsageContext::new("signals", SIGNAL_TYPING_OPERATION)
            .with_trigger("fail_improve_llm_fallback")
            .with_tier(ModelTier::Mechanical),
    );
    let prompt = registry_signal_typing_prompt_input(input, Some(&workspace));

    complete_registry_signal_typing(&provider, input.clone(), prompt, ModelTier::Mechanical, ctx)
        .await
}

#[cfg(test)]
pub async fn classify_registry_signal_type_with_llm(
    input: &SignalTypingInput,
    ctx: &ServiceContext<'_>,
) -> Result<String> {
    let provider = SignalTypingTestProvider;
    let prompt = registry_signal_typing_prompt_input(input, None);
    complete_registry_signal_typing(&provider, input.clone(), prompt, ModelTier::Mechanical, ctx)
        .await
}

fn registry_signal_typing_prompt(signal_type: &str) -> String {
    let allowed = crate::signals::policy_registry::known_signal_type_names().join(", ");
    format!(
        "Classify this DailyOS signal_type into the closest canonical registry signal type.\n\
         Unknown signal_type: {signal_type}\n\
         Allowed canonical signal_type values: {allowed}\n\
         Return only compact JSON in this exact shape: {{\"signal_type\":\"<one allowed value or null>\"}}.\n\
         If no allowed value is a confident semantic fit, use null."
    )
}

fn registry_signal_typing_prompt_input(
    input: &SignalTypingInput,
    workspace: Option<&Path>,
) -> PromptInput {
    let mut prompt = PromptInput::new(registry_signal_typing_prompt(input.signal_type.trim()))
        .with_template(
            "signals.registry_signal_typing",
            "1",
            "registry_signal_typing_prompt_v1",
        )
        .with_canonical_json_inputs(serde_json::json!({
            "signal_type": input.signal_type.trim(),
        }));
    if let Some(workspace) = workspace {
        prompt = prompt.with_workspace(workspace);
    }
    prompt
}

fn provider_error_to_fail_improve(error: ProviderError) -> FailImproveError {
    FailImproveError::Llm(error.to_string())
}

#[cfg(test)]
struct SignalTypingTestProvider;

#[cfg(test)]
#[async_trait::async_trait]
impl IntelligenceProvider for SignalTypingTestProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        _tier: ModelTier,
    ) -> std::result::Result<Completion, ProviderError> {
        SIGNAL_TYPE_LLM_CALLS_FOR_TEST.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let delay_ms = SIGNAL_TYPE_LLM_DELAY_MS_FOR_TEST.load(std::sync::atomic::Ordering::SeqCst);
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        let signal_type = prompt
            .canonical_json_inputs
            .as_ref()
            .and_then(|value| value.get("signal_type"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = serde_json::json!({
            "signal_type": serde_json::Value::Null,
            "original": signal_type,
        });
        Ok(Completion {
            text: response.to_string(),
            fingerprint_metadata: Default::default(),
        })
    }

    fn provider_kind(&self) -> crate::intelligence::provider::ProviderKind {
        crate::intelligence::provider::ProviderKind::Other("fail_improve_test")
    }

    fn current_model(&self, _tier: ModelTier) -> crate::intelligence::provider::ModelName {
        crate::intelligence::provider::ModelName::new("fail-improve-test")
    }
}

fn parse_registry_signal_typing_response(original: &str, output: &str) -> Result<String> {
    #[derive(Deserialize)]
    struct Response {
        signal_type: Option<String>,
    }

    let trimmed = output.trim();
    let json_slice = if trimmed.starts_with('{') {
        trimmed
    } else {
        let start = trimmed
            .find('{')
            .ok_or_else(|| FailImproveError::Llm("missing JSON object".to_string()))?;
        let end = trimmed
            .rfind('}')
            .ok_or_else(|| FailImproveError::Llm("unterminated JSON object".to_string()))?;
        &trimmed[start..=end]
    };

    let response: Response = serde_json::from_str(json_slice)
        .map_err(|err| FailImproveError::Llm(format!("invalid JSON response: {err}")))?;
    let Some(signal_type) = response.signal_type else {
        return Ok(original.to_string());
    };
    let candidate = signal_type.trim();
    if candidate.is_empty() {
        return Err(FailImproveError::Llm(
            "empty signal_type classification".to_string(),
        ));
    }

    let signal = crate::signals::policy_registry::SignalType::from_name(candidate);
    if matches!(
        signal,
        crate::signals::policy_registry::SignalType::Legacy { .. }
    ) {
        return Err(FailImproveError::Llm(format!(
            "LLM classified `{original}` as non-registry signal `{candidate}`"
        )));
    }

    Ok(signal.canonical_name().to_string())
}

fn default_fail_improve_root() -> PathBuf {
    if let Ok(root) = std::env::var("DAILYOS_FAIL_IMPROVE_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    #[cfg(test)]
    {
        std::env::temp_dir().join(format!("dailyos-fail-improve-tests-{}", std::process::id()))
    }

    #[cfg(not(test))]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("com.dailyos.app")
            .join("fail-improve")
    }
}

fn operation_file_stem(operation: &str) -> Result<String> {
    let trimmed = operation.trim();
    if trimmed.is_empty() {
        return Err(FailImproveError::InvalidOperation(operation.to_string()));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(FailImproveError::InvalidOperation(operation.to_string()));
    }

    let sanitized = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() || sanitized.contains("..") {
        return Err(FailImproveError::InvalidOperation(operation.to_string()));
    }
    Ok(sanitized)
}

fn jsonl_lines_bytes(lines: &[String]) -> Vec<u8> {
    let mut content = lines.join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    content.into_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArtifactKind {
    Jsonl,
    Counts,
    Trend,
}

fn artifact_file_name(operation: &str, kind: ArtifactKind) -> String {
    match kind {
        ArtifactKind::Jsonl => format!("{operation}.jsonl"),
        ArtifactKind::Counts => format!("{operation}.counts.json"),
        ArtifactKind::Trend => format!("{operation}.trend.jsonl"),
    }
}

fn operation_from_artifact_file_name(name: &str) -> Option<&str> {
    name.strip_suffix(".counts.json")
        .or_else(|| name.strip_suffix(".trend.jsonl"))
        .or_else(|| name.strip_suffix(".jsonl"))
        .filter(|operation| !operation.is_empty())
}

fn valid_bundle_id(bundle_id: &str) -> bool {
    !bundle_id.is_empty()
        && !bundle_id.contains("..")
        && !bundle_id.contains('/')
        && !bundle_id.contains('\\')
}

fn captures_unknown_signal_payload(payload_privacy: PayloadPrivacy) -> bool {
    matches!(payload_privacy, PayloadPrivacy::NonPiiMetadata)
}

struct PendingArtifactWrite {
    path: PathBuf,
    bytes: Vec<u8>,
}

fn write_artifacts_atomically(writes: Vec<PendingArtifactWrite>) -> Result<()> {
    write_artifacts_atomically_inner(writes, false)
}

fn write_artifacts_atomically_inner(
    writes: Vec<PendingArtifactWrite>,
    simulate_crash_after_bundle_rename: bool,
) -> Result<()> {
    let Some((root, operation)) = artifact_write_context(&writes)? else {
        return Ok(());
    };

    std::fs::create_dir_all(&root)?;
    let bundles_dir = root.join(format!("{operation}.bundles"));
    std::fs::create_dir_all(&bundles_dir)?;

    let bundle_id = format!(
        "{}-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        Uuid::new_v4().simple()
    );
    let temp_dir = bundles_dir.join(format!(".{bundle_id}.tmp"));
    let final_dir = bundles_dir.join(&bundle_id);

    let result = (|| -> Result<()> {
        std::fs::create_dir(&temp_dir)?;
        for write in writes {
            let Some(file_name) = write.path.file_name().and_then(|name| name.to_str()) else {
                return Err(FailImproveError::InvalidOperation(
                    write.path.display().to_string(),
                ));
            };
            let mut file = std::fs::File::create(temp_dir.join(file_name))?;
            file.write_all(&write.bytes)?;
            file.flush()?;
            file.sync_all()?;
        }

        std::fs::rename(&temp_dir, &final_dir)?;
        if simulate_crash_after_bundle_rename {
            return Err(FailImproveError::Io(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "simulated crash after artifact bundle rename",
            )));
        }

        let pointer_path = root.join(format!("{operation}.current"));
        let mut pointer_tmp = tempfile::NamedTempFile::new_in(&root)?;
        writeln!(pointer_tmp, "{bundle_id}")?;
        pointer_tmp.flush()?;
        pointer_tmp.as_file().sync_all()?;
        pointer_tmp.persist(pointer_path).map_err(|e| e.error)?;
        Ok(())
    })();

    if result.is_err() && temp_dir.exists() {
        if let Err(err) = std::fs::remove_dir_all(&temp_dir) {
            log::debug!(
                "failed to remove incomplete fail-improve artifact bundle {}: {err}",
                temp_dir.display()
            );
        }
    }

    result
}

fn artifact_write_context(writes: &[PendingArtifactWrite]) -> Result<Option<(PathBuf, String)>> {
    let Some(first) = writes.first() else {
        return Ok(None);
    };
    let Some(root) = first.path.parent() else {
        return Err(FailImproveError::InvalidOperation(
            first.path.display().to_string(),
        ));
    };
    let Some(first_name) = first.path.file_name().and_then(|name| name.to_str()) else {
        return Err(FailImproveError::InvalidOperation(
            first.path.display().to_string(),
        ));
    };
    let Some(operation) = operation_from_artifact_file_name(first_name) else {
        return Err(FailImproveError::InvalidOperation(first_name.to_string()));
    };
    let operation = operation_file_stem(operation)?;

    for write in writes {
        if write.path.parent() != Some(root) {
            return Err(FailImproveError::InvalidOperation(
                write.path.display().to_string(),
            ));
        }
        let Some(name) = write.path.file_name().and_then(|name| name.to_str()) else {
            return Err(FailImproveError::InvalidOperation(
                write.path.display().to_string(),
            ));
        };
        if operation_from_artifact_file_name(name)
            .map(operation_file_stem)
            .transpose()?
            != Some(operation.clone())
        {
            return Err(FailImproveError::InvalidOperation(name.to_string()));
        }
    }

    Ok(Some((root.to_path_buf(), operation)))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn test_ctx() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 9, 14, 30, 0).unwrap()),
            SeedableRng::new(42),
            ExternalClients::default(),
        )
    }

    fn read_artifact(loop_: &FailImproveLoop, operation: &str, kind: ArtifactKind) -> String {
        loop_
            .read_artifact_to_string(operation, kind)
            .expect("read artifact")
            .expect("artifact exists")
    }

    fn read_counts(loop_: &FailImproveLoop, operation: &str) -> FailImproveCounts {
        serde_json::from_str(&read_artifact(loop_, operation, ArtifactKind::Counts))
            .expect("counts json")
    }

    struct RegistryCompletionFixtureProvider;

    #[async_trait::async_trait]
    impl IntelligenceProvider for RegistryCompletionFixtureProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> std::result::Result<Completion, ProviderError> {
            Ok(Completion {
                text: serde_json::json!({
                    "signal_type": "account_risk",
                })
                .to_string(),
                fingerprint_metadata: Default::default(),
            })
        }

        fn provider_kind(&self) -> crate::intelligence::provider::ProviderKind {
            crate::intelligence::provider::ProviderKind::Other("fixture")
        }

        fn current_model(&self, _tier: ModelTier) -> crate::intelligence::provider::ModelName {
            crate::intelligence::provider::ModelName::new("fixture")
        }
    }

    #[tokio::test]
    async fn deterministic_hit_updates_counts_without_jsonl_entry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
        let (clock, rng, ext) = test_ctx();
        let ctx = ServiceContext::new_live(&clock, &rng, &ext);

        let output = loop_
            .execute(
                "signal_typing",
                SignalTypingInput {
                    signal_type: "account_created".to_string(),
                },
                |input| {
                    let result = deterministic_registry_signal_type(input);
                    async move { Ok(result) }
                },
                |_| async { Err(FailImproveError::Llm("should not run".to_string())) },
                &ctx,
            )
            .await
            .expect("execute");

        assert_eq!(output, "account_created");
        assert!(loop_
            .read_artifact_to_string("signal_typing", ArtifactKind::Jsonl)
            .expect("read jsonl")
            .is_none());

        let counts = read_counts(&loop_, "signal_typing");
        assert_eq!(counts.total_calls, 1);
        assert_eq!(counts.deterministic_hits, 1);
        assert_eq!(counts.llm_fallbacks, 0);
        assert_eq!(counts.deterministic_rate, 1.0);
        assert_eq!(counts.updated_at, "2026-05-09T14:30:00+00:00");
    }

    #[tokio::test]
    async fn llm_fallback_appends_entry_and_generates_test_case() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
        let (clock, rng, ext) = test_ctx();
        let ctx = ServiceContext::new_live(&clock, &rng, &ext);

        let output = loop_
            .execute(
                "signal_typing",
                SignalTypingInput {
                    signal_type: "customer-health-risk".to_string(),
                },
                |_| async { Ok(None) },
                |_| async { Ok("account_risk".to_string()) },
                &ctx,
            )
            .await
            .expect("execute");

        assert_eq!(output, "account_risk");

        let raw = read_artifact(&loop_, "signal_typing", ArtifactKind::Jsonl);
        let lines = raw.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let entry: FailImproveEntry = serde_json::from_str(lines[0]).expect("entry");
        assert_eq!(entry.ts, "2026-05-09T14:30:00+00:00");
        assert_eq!(entry.operation, "signal_typing");
        assert!(entry.deterministic_result.is_none());
        assert_eq!(entry.llm_result, Value::String("account_risk".to_string()));
        assert!(entry.invocation_id.starts_with("fi-"));

        let cases = loop_
            .generate_test_cases("signal_typing")
            .expect("test cases");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].source, TEST_CASE_SOURCE);
        assert_eq!(cases[0].expected, Value::String("account_risk".to_string()));

        let counts = read_counts(&loop_, "signal_typing");
        assert_eq!(counts.total_calls, 1);
        assert_eq!(counts.deterministic_hits, 0);
        assert_eq!(counts.llm_fallbacks, 1);
        assert_eq!(counts.deterministic_rate, 0.0);
    }

    #[tokio::test]
    async fn intelligence_provider_completion_miss_records_fail_improve_artifact() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
        let (clock, rng, ext) = test_ctx();
        let ctx = ServiceContext::new_live(&clock, &rng, &ext);
        let input = SignalTypingInput {
            signal_type: "customer-health-risk".to_string(),
        };
        let prompt = registry_signal_typing_prompt_input(&input, None);
        let provider = RegistryCompletionFixtureProvider;

        let output = complete_registry_signal_typing_with_loop(
            &loop_,
            &provider,
            input.clone(),
            prompt,
            ModelTier::Mechanical,
            &ctx,
        )
        .await
        .expect("provider completion");

        assert_eq!(output, "account_risk");
        let raw = read_artifact(&loop_, SIGNAL_TYPING_OPERATION, ArtifactKind::Jsonl);
        let entry: FailImproveEntry =
            serde_json::from_str(raw.lines().next().expect("jsonl line")).expect("entry");
        assert_eq!(
            entry.input,
            serde_json::json!({
                "signal_type": input.signal_type,
            })
        );
        assert_eq!(entry.llm_result, Value::String("account_risk".to_string()));
    }

    #[test]
    fn unknown_signal_observation_appends_without_signal_typing_test_case() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());

        loop_
            .record_unknown_signal_observation(UnknownSignalObservation {
                signal_type: "customer_health_moved".to_string(),
                payload: serde_json::json!({
                    "signalType": "customer_health_moved",
                    "entityType": "account",
                }),
                payload_privacy: PayloadPrivacy::NonPiiMetadata,
            })
            .expect("record observation");

        let raw = read_artifact(&loop_, UNKNOWN_SIGNAL_TYPES_OPERATION, ArtifactKind::Jsonl);
        let lines = raw.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let entry: UnknownSignalTypeObservation =
            serde_json::from_str(lines[0]).expect("observation");
        assert_eq!(entry.operation, UNKNOWN_SIGNAL_TYPES_OPERATION);
        assert_eq!(entry.signal_type, "customer_health_moved");
        assert_eq!(
            entry
                .payload
                .get("signalType")
                .and_then(serde_json::Value::as_str),
            Some("customer_health_moved")
        );

        let cases = loop_
            .generate_test_cases(SIGNAL_TYPING_OPERATION)
            .expect("test cases");
        assert!(cases.is_empty());

        let counts = read_counts(&loop_, UNKNOWN_SIGNAL_TYPES_OPERATION);
        assert_eq!(counts.total_calls, 1);
        assert_eq!(counts.deterministic_hits, 0);
        assert_eq!(counts.llm_fallbacks, 1);
    }

    #[test]
    fn unknown_signal_observation_skips_sensitive_payload_policy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());

        loop_
            .record_unknown_signal_observation(UnknownSignalObservation {
                signal_type: "legacy_user_feedback".to_string(),
                payload: serde_json::json!({
                    "signalType": "legacy_user_feedback",
                    "value": "sensitive user-authored text",
                    "sourceContext": "private note",
                }),
                payload_privacy: PayloadPrivacy::UserAuthoredText,
            })
            .expect("record observation");

        assert!(
            loop_
                .read_artifact_to_string(UNKNOWN_SIGNAL_TYPES_OPERATION, ArtifactKind::Jsonl)
                .expect("read jsonl")
                .is_none(),
            "sensitive unknown signal payload should not be persisted"
        );
    }

    #[test]
    fn crash_after_bundle_rename_leaves_prior_bundle_intact() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
        let operation = "signal_typing";

        let initial_counts = FailImproveCounts {
            total_calls: 1,
            deterministic_hits: 1,
            llm_fallbacks: 0,
            deterministic_rate: 1.0,
            updated_at: "2026-05-09T14:30:00+00:00".to_string(),
        };
        let initial_trend = FailImproveTrendPoint {
            ts: initial_counts.updated_at.clone(),
            total_calls: 1,
            deterministic_hits: 1,
            llm_fallbacks: 0,
            deterministic_rate: 1.0,
        };
        write_artifacts_atomically(
            loop_
                .stats_artifact_writes(operation, &initial_counts, &initial_trend)
                .expect("initial writes"),
        )
        .expect("initial bundle write");

        let next_counts = FailImproveCounts {
            total_calls: 2,
            deterministic_hits: 1,
            llm_fallbacks: 1,
            deterministic_rate: 0.5,
            updated_at: "2026-05-09T14:31:00+00:00".to_string(),
        };
        let next_trend = FailImproveTrendPoint {
            ts: next_counts.updated_at.clone(),
            total_calls: 2,
            deterministic_hits: 1,
            llm_fallbacks: 1,
            deterministic_rate: 0.5,
        };
        let err = write_artifacts_atomically_inner(
            loop_
                .stats_artifact_writes(operation, &next_counts, &next_trend)
                .expect("next writes"),
            true,
        )
        .expect_err("simulated crash should abort pointer update");
        assert!(matches!(err, FailImproveError::Io(_)));

        let observed = read_counts(&loop_, operation);
        assert_eq!(observed.total_calls, 1);
        assert_eq!(observed.deterministic_hits, 1);
        assert_eq!(observed.llm_fallbacks, 0);
        assert!(!tmp.path().join("signal_typing.counts.json").exists());
    }

    #[tokio::test]
    async fn jsonl_rotates_at_1000_entries() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
        let (clock, rng, ext) = test_ctx();
        let ctx = ServiceContext::new_live(&clock, &rng, &ext);

        for i in 0..1005 {
            loop_
                .execute(
                    "signal_typing",
                    SignalTypingInput {
                        signal_type: format!("unknown_{i}"),
                    },
                    |_| async { Ok(None) },
                    move |_| async move { Ok(format!("legacy_{i}")) },
                    &ctx,
                )
                .await
                .expect("execute");
        }

        let raw = read_artifact(&loop_, "signal_typing", ArtifactKind::Jsonl);
        let lines = raw.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), MAX_JSONL_ENTRIES);
        let first: FailImproveEntry = serde_json::from_str(lines[0]).expect("first entry");
        assert_eq!(first.llm_result, Value::String("legacy_5".to_string()));
    }

    #[tokio::test]
    async fn aborted_runs_emit_no_entry_property() {
        for seed in 1..25 {
            let tmp = tempfile::tempdir().expect("tempdir");
            let loop_ = FailImproveLoop::new(tmp.path().to_path_buf());
            let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 9, 14, 30, 0).unwrap());
            let rng = SeedableRng::new(seed);
            let ext = ExternalClients::default();
            let ctx = ServiceContext::new_live(&clock, &rng, &ext);

            let pending = loop_.execute(
                "signal_typing",
                SignalTypingInput {
                    signal_type: format!("unknown_{seed}"),
                },
                |_| async { Ok(None) },
                |_| async { std::future::pending::<Result<String>>().await },
                &ctx,
            );
            tokio::pin!(pending);
            tokio::select! {
                result = &mut pending => panic!("pending fallback unexpectedly completed: {result:?}"),
                _ = tokio::time::sleep(std::time::Duration::from_millis(1)) => {}
            }
            drop(pending);

            assert!(loop_
                .read_artifact_to_string("signal_typing", ArtifactKind::Jsonl)
                .expect("read jsonl")
                .is_none());
            assert!(loop_
                .read_artifact_to_string("signal_typing", ArtifactKind::Counts)
                .expect("read counts")
                .is_none());
        }
    }
}
