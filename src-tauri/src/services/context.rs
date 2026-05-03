//!  `ServiceContext` substrate per ADR-0104.
//!
//! ## What this module owns
//!
//! - `ExecutionMode { Live | Simulate | Evaluate }` — the mode-routing enum
//!   every service mutation gates against via `ctx.check_mutation_allowed()?`.
//! - `Clock` + `SeededRng` traits — injection seams replacing direct
//!   `Utc::now()` / `rand::thread_rng()` in service + ability code.
//! - `ServiceContext<'a>` — per-call carrier with public read capabilities
//!   (`mode`, `clock`, `rng`, `external`) and `pub(in crate::services)`
//!   service-internal fields.
//! - `ExternalClients` — named wrapper struct for `glean` / `slack` /
//!   `gmail` / `salesforce`; live in `Live`, replay/fixture in
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

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;

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
        Self { now: Mutex::new(at) }
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

/// External-services wrapper struct. Each field is a thin handle that
/// delegates to live clients in `Live` and replay/fixture in
/// `Simulate`/`Evaluate`. Live wrappers hold concrete client `Arc`s;
/// non-Live wrappers hold `None` and any side-effecting call returns
/// `WriteBlockedByMode` from the caller's `check_mutation_allowed` gate.
#[derive(Default, Clone)]
pub struct ExternalClients {
    pub glean: GleanClientHandle,
    pub slack: SlackClientHandle,
    pub gmail: GmailClientHandle,
    pub salesforce: SalesforceClientHandle,
}

/// Mode-aware Glean client wrapper. Holds `Some(arc)` when configured;
/// `None` in Local Live mode and in Simulate/Evaluate.
#[derive(Default, Clone)]
pub struct GleanClientHandle {
    inner: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl GleanClientHandle {
    pub fn is_configured(&self) -> bool {
        self.inner.is_some()
    }
}

/// Mode-aware Slack client wrapper. Placeholder — Slack integration
/// lands in; the seam reserves the API surface so abilities
/// can call it without re-plumbing later.
#[derive(Default, Clone)]
pub struct SlackClientHandle;

/// Mode-aware Gmail client wrapper. Live mode wraps `crate::google_api`;
/// non-Live modes return `WriteBlockedByMode` from any send/modify call
/// via the caller's gate.
#[derive(Default, Clone)]
pub struct GmailClientHandle;

/// Mode-aware Salesforce client wrapper. Placeholder — Glean is the SF
/// data plane today; the seam reserves direct-integration scope.
#[derive(Default, Clone)]
pub struct SalesforceClientHandle;

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

/// Per-call service execution context.
///
/// `mode`, `clock`, `rng`, `external` are public read capabilities
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
    pub external: &'a ExternalClients,
    pub(in crate::services) tx: Option<TxHandle>,
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
            external,
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
            external,
            tx: None,
        }
    }

    /// `Evaluate` constructor — fixture DB only.
    ///
    /// `external` MUST contain replay/fixture client wrappers — Live
    /// wrappers are a programming error in this mode. The wrapper types
    /// do not enforce this themselves; the caller invariant is that
    /// replay fixtures populate `external` before construction.
    ///
    /// **Boot-time guard for production-DB-path rejection** lands in
    /// the DB-plumbing phase; the fixture-DB invariant is documented
    /// here but unverified at the substrate level.
    pub fn new_evaluate(
        clock: &'a dyn Clock,
        rng: &'a dyn SeededRng,
        external: &'a ExternalClients,
    ) -> Self {
        Self {
            mode: ExecutionMode::Evaluate,
            clock,
            rng,
            external,
            tx: None,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixture_external() -> ExternalClients {
        ExternalClients::default()
    }
    fn fixture_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 12, 0, 0).unwrap())
    }
    fn fixture_rng() -> SeedableRng {
        SeedableRng::new(42)
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
        let ext = fixture_external();
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
        let ext = fixture_external();
        let live = ServiceContext::test_live(&clk, &rng, &ext);
        assert_eq!(live.mode, ExecutionMode::Live);
        let sim = ServiceContext::new_simulate(&clk, &rng, &ext);
        assert_eq!(sim.mode, ExecutionMode::Simulate);
        let eval = ServiceContext::test_evaluate(&clk, &rng, &ext);
        assert_eq!(eval.mode, ExecutionMode::Evaluate);
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
}
