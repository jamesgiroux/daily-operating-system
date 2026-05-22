# L0 Packet — Foreground DB Contention and Read-Path Stabilization

**Current revision:** V0.3 (follow-up L0 scope fix, 2026-05-22)

**Review status:** L0 approved on V0.3. V0.1 returned request-changes from adversarial, performance, security, and scope review. V0.2 resolved the substantive blockers; V0.3 splits PR1-required Settings status cleanup from PR2-conditional Settings UI fan-out.

## 0. Origination

- **Origination class:** Debug-driven.
- **Scope tier:** Wave-tier review, standard-tier implementation. The packet touches DB access policy, background processors, foreground route loaders, and user-visible latency. It does not introduce a new product capability.
- **Trust topology:** local-to-local single-user. The threat model is a local Tauri desktop app with encrypted local SQLite/SQLCipher storage and optional local surface clients.
- **Branch:** `l0-foreground-db-contention`.
- **Worktree:** `/Users/jamesgiroux/Documents/dailyos-repo/.worktrees/l0-foreground-db-contention`.
- **Migration slots claimed:** none in Phase 1. No Phase 1 migration is expected. Any later read-model table requires a packet amendment and a fresh slot claim before implementation.
- **Primary symptom:** Settings takes about 10s to load; switching into a meeting briefing or account detail page takes roughly 2-5s. Regression began during v1.4.0-v1.4.3 work.

### 0.1 Symptom-To-Failure Trace

Evidence collected before this packet:

1. A live `sample` of the running `target/debug/dailyos` process captured a 5-second stall in:
   `google::run_calendar_poller -> populate_people_from_events -> db::people::record_meeting_attendance -> sqlite3BtreeBeginTrans -> sqlite3InvokeBusyHandler -> usleep`.
   This is a real SQLite busy wait, not a speculative architecture concern.
2. Current DB pragmas set `busy_timeout = 5000` in both `src-tauri/src/db_service.rs` and `src-tauri/src/db/core.rs`, matching the observed 5s chunking.
3. `DbService` has one writer and two readers (`NUM_READERS = 2`), but `ActionDb::open()` only serializes fresh connection creation through `DbService::open_fresh_serialized()` and then returns an independent writable connection. Background work can still hold SQLite write locks outside the foreground writer queue.
4. Foreground "read" routes await writes:
   - `services::accounts::get_account_detail` awaits `state.db_write(...)` to ensure lifecycle state before the detail read.
   - `services::meetings::get_meeting_intelligence` awaits a post-read `db_write(...)` to mark prep reviewed and clear new-signal flags.
5. Settings eager-mounts every section; `ConnectorsGrid` loops through all connector status commands on mount, and some status commands still use `ActionDb::open()` / `with_db` paths.
6. Local read-only DB diagnostics show the substrate is large enough for unbounded signal/claim reads to matter:
   - `signal_events`: 177,711 rows, all active; max active signals per entity: 9,354.
   - `intelligence_claims`: 16,068 rows; active+active: 15,789.
   - `mutation_attempts` / `version_events`: 14,207 rows each.

### 0.2 Hypotheses Rejected Or Deprioritized

- **"Remote processing is required."** Rejected for this packet. The evidence points to local foreground/background scheduling and write-lock discipline, not inherent impossibility of local processing.
- **"Abilities runtime alone is the root cause."** Deprioritized. Abilities and claims add background load, but the sampled stall is a calendar poller write. Abilities are an amplifier through claims/signals/fresh-open paths, not the only culprit.
- **"Separate read DB first."** Rejected for Phase 1. A second encrypted DB would add sync, freshness, key rotation, purge, and crash-recovery complexity before proving that simpler foreground write isolation is insufficient.

## 1. Goal

Restore fast, predictable foreground navigation under normal background intelligence load:

- Settings first usable render under 1.5s p95 on an existing local DB.
- Account detail and meeting briefing core payload under 1.0s p95 warm, under 2.0s p95 cold.
- No foreground `get_*` route blocks on a best-effort write.
- No active background writer in the proof run holds an unclassified independent SQLite write lock during foreground navigation.
- Read-model/caching work is explicitly out of Phase 1. Any cache/read model is versioned, measured, and approved by a later amendment.

## 2. Non-Goals

- No separate read-only database file in this packet.
- No broad CQRS rewrite.
- No new remote processing path.
- No full `ActionDb::open()` compile-time closure across the whole repo.
- No new persistent read-model table, materialized projection, or cross-render cache in Phase 1.
- No semantic redesign of claims, trust scoring, abilities, or surface runtime.
- No user-visible copy changes except possibly loading-state behavior needed to avoid blocking.

## 3. Chosen Architecture

This packet chooses a bounded stabilization path with explicit PR boundaries:

- **PR1:** instrumentation, account/meeting foreground read purity, calendar attendee hot-path write batching, Settings status-command DB access cleanup, and proof bundle.
- **PR2:** Settings UI mount fan-out reduction only if PR1 evidence still misses the Settings target, or as a separate frontend follow-up after backend-only before/after measurements are captured.
- **Phase 2:** materialized read models only by later amendment if PR1/PR2 measurements prove that write-path cleanup and mount deferral are insufficient.

### 3.1 Phase 1 — Stop Foreground Reads From Waiting On Writes

Make slow foreground surfaces pure reads. A `get_*` route must not cause direct, awaited, spawned, scheduled, or post-return mutation work.

Required changes:

1. `get_account_detail` becomes read-only.
   - Remove the awaited lifecycle `db_write` from the foreground detail command.
   - Lifecycle reconciliation moves to explicit mutation/signal paths or a named lifecycle worker not triggered by `get_account_detail`.
   - Any lifecycle mutation still goes through `services/` and keeps actor/audit semantics.

2. `get_meeting_intelligence` returns after read assembly.
   - Remove the live-calendar auto-persist branch from the foreground read path. If a meeting exists only in the live calendar cache, build a read-only briefing from that cache or return the current read-only fallback.
   - Move `mark_prep_reviewed` and `clear_meeting_new_signals` to explicit service mutations or a named worker not triggered by `get_meeting_intelligence`.
   - If review/new-signal flags are stale for one render, the UI may show a harmless stale indicator; it must not block or mutate from the briefing read.

3. The sampled calendar attendee write path moves into a service-owned batch.
   - `google::populate_people_from_events` must not loop direct `ActionDb::open()` writes for meeting/person/attendance changes from the poller path.
   - Add or reuse a service function that processes a batch of meeting/person attendance changes in one bounded writer closure.
   - The service emits or preserves required signals for new people / changed meetings.
   - Compute and de-duplicate the batch outside the writer closure.
   - Keep the writer closure DB-only and bounded; filesystem side effects and artifact writes run after commit.

4. Background workers active during the proof run are inventoried.
   - Each active worker is classified as service-owned writer, read-only `db_read`, or out-of-scope with rationale.
   - If an active worker remains an uninstrumented writable `ActionDb::open()` path, L1 must either route it through a read-only/service path or exclude it from the proof run with explicit rationale.
   - This packet does not claim to close every `ActionDb::open()` call site across the repo.

5. Settings first-screen status commands stop using fresh writable opens.
   - Convert DB-backed status reads named by the review (`get_context_mode`, `get_gravatar_status`, `get_linear_status`) to `AppState::db_read`, read-only service calls, or non-DB config state where appropriate.
   - Instrument these commands so Settings latency can separate command time from DB queue/open time.
   - Frontend concurrency limiting is not a substitute for this backend cleanup.

### 3.2 Phase 1A — Instrument Before And After

Add lightweight, PII-safe latency instrumentation:

- Queue wait and execution duration for:
  - `DbService` reader calls.
  - `DbService` writer calls.
  - `DbService::open_fresh_serialized`.
  - `AppState::db_read` and `AppState::db_write`.
- Foreground command spans:
  - `get_account_detail`
  - `get_meeting_intelligence`
  - `get_context_mode`
  - `get_linear_status`
  - `get_gravatar_status`
  - `get_audit_log_records`
- Tags must be stable command/caller labels only. No account IDs, meeting titles, emails, domains, company names, claim text, or raw SQL bind values in logs or latency payloads.
- Measurements must distinguish queue wait, execution, and `open_fresh_serialized` time.

The current `latency` module already provides process-local command rollups. Extend that shape rather than adding persistent telemetry.

### 3.3 PR2 — Reduce Settings Mount Fan-Out If Needed

Settings should not fire every diagnostic/status read at once, but this is a separate step after backend-only proof unless PR1 measurements show Settings still misses target.

Required changes:

- Keep first-screen Settings content mountable immediately.
- Lazy-load below-fold sections (`Data`, `System`, dev-only `Diagnostics`) when visible or after initial idle.
- Connector status checks should be concurrency-limited and grouped by cost:
  - cheap in-memory/config status first,
  - filesystem/keychain/API-backed status later,
  - dev diagnostics only on demand or after idle.
- `ContextSourceSection` retry backoff must not make one slow `get_context_mode` command hold Settings hostage for ~3.75s.

### 3.4 Phase 2 — Materialized Read Models Only If Metrics Still Fail

If PR1/PR2 still miss targets, a later packet amendment may add narrowly-scoped read-model tables in the same encrypted DB. Do not introduce a second DB file without a separate ADR/L0 packet.

The amendment must identify the narrowest failing read from measurements before naming tables. This packet intentionally does not authorize candidate table names.

Rules:

- Cache miss never blocks foreground render. Return last-known data with freshness metadata and enqueue rebuild.
- Read models store derived state plus provenance/claim refs; they do not become new claim authority.
- Invalidation keys use existing claim/signal/version watermarks. No timer-only freshness.
- Feedback/corrections update canonical claim/signal state first; projection rebuild follows.
- Any reusable projection key includes audience/principal binding: actor class, surface, policy version, sensitivity policy version, and `SurfaceClient` instance or scope-grant digest where relevant. Last-known fallback must never cross that audience key.

## 4. Data Flow

### 4.1 Current Failure Shape

```text
Foreground route
  -> Tauri command
     -> read assembly
     -> awaited small write
        -> single writer queue OR independent SQLite writer
           -> busy_timeout 5000ms
              -> UI waits

Background poller
  -> ActionDb::open()
     -> fresh writable connection
        -> per-row writes
           -> SQLite write lock
```

### 4.2 Target Phase 1 Shape

```text
Foreground route
  -> Tauri command
     -> db_read or read-only live cache fallback
     -> payload returns

Explicit mutation / signal / lifecycle path
  -> service mutation
     -> AppState::db_write / managed writer

Background poller
  -> service batch request
     -> AppState::db_write / managed writer
        -> bounded DB-only transaction
        -> signals emitted through services
     -> filesystem/artifact side effects after commit
```

### 4.3 Target Phase 2 Shape

```text
Claim/signal/source mutation
  -> canonical services write
  -> signal/version watermark bump
  -> invalidation job
  -> read-model projector
  -> cheap foreground read
```

## 5. Files Likely Touched

Exact implementation may adjust, but the packet owns this surface:

- `src-tauri/src/db_service.rs` — queue/execution instrumentation; no pool redesign.
- `src-tauri/src/state.rs` — `db_read` / `db_write` timing wrapper.
- `src-tauri/src/latency.rs` and `src-tauri/src/commands/app_support.rs` — extend rollups if needed.
- `src-tauri/src/google.rs` — remove direct per-row poller writes from hot path.
- `src-tauri/src/commands/integrations.rs` — route Settings status reads away from fresh writable opens.
- `src-tauri/src/services/*` — add service-owned calendar/person/attendance batch mutation as needed.
- `src-tauri/src/services/accounts.rs` — make `get_account_detail` foreground path read-only.
- `src-tauri/src/services/meetings.rs` — make `get_meeting_intelligence` foreground path read-only.
- `src/pages/SettingsPage.tsx`
- `src/features/settings-ui/ConnectorsGrid.tsx`
- `src/features/settings-ui/ContextSourceSection.tsx`
- `src/features/settings-ui/SystemStatus.tsx`
- Focused tests under the existing Rust and frontend test locations.

Phase 2, if promoted by metrics, may add a migration and read-model service files. That promotion requires a packet amendment before code starts.

## 6. Intelligence Loop Integration Check

Phase 1 adds no claim/table/user-visible intelligence field. It changes scheduling, instrumentation, and read/write boundaries.

Phase 2 read models, if promoted, must answer the five Intelligence Loop questions as follows:

1. **Claim model:** read-model rows are projections, not claims. They store refs to canonical claims and source watermarks. They must not create ad-hoc facts.
2. **Provenance + trust:** rendered bundles include claim refs, trust bands, source/version watermarks, and `generated_at`. Trust remains computed by canonical claim/trust services.
3. **Signals + invalidation:** read models rebuild only from claim/signal/source/version changes. Invalidation jobs must name the producer, subject, source watermark, and projection version.
4. **Runtime + surfaces:** Tauri foreground pages may consume read models. MCP/surface runtime may consume them only if actor/surface policy matches canonical claim visibility rules.
5. **Feedback loop:** user corrections and dismissals write through existing feedback/claim services first. Read models observe changed watermarks; they never accept feedback writes directly.

## 7. Security And Privacy

- No PII in latency logs, diagnostics, plan artifacts, commits, or reviewer output.
- A separate read DB is explicitly out of scope because it duplicates sensitive data and complicates SQLCipher key rotation, purge semantics, backup/export, and crash recovery.
- Any later read-model table lives in the existing encrypted DB and inherits existing backup/export/delete-all-data behavior.
- Read models must not weaken actor/surface filtering. If a projection is reused across surfaces, its key includes actor class, surface, policy version, sensitivity policy version, and `SurfaceClient` instance or scope-grant digest where relevant.
- Stale projections must be visibly stale or capped in trust treatment where relevant; they must never present stale derived intelligence as freshly verified.
- Touched poller/batch diagnostics must use counts, stable labels, and opaque IDs only. No meeting titles, emails, domains, company names, person names, claim text, SQL bind values, or raw provider payload snippets.

## 8. Acceptance Criteria

### AC1 — Measured Baseline And Delta

- Capture before/after p50/p95/max for:
  - Settings route initial mount.
  - Account detail `get_account_detail`.
  - Meeting detail `get_meeting_intelligence`.
  - DB reader/writer queue wait and execution.
  - `open_fresh_serialized`.
- Evidence must include:
  - same existing local DB before/after;
  - same build mode before/after, with build mode recorded;
  - at least 20 warm samples for each primary foreground route;
  - at least 3 cold-launch samples for Settings initial mount;
  - calendar/intelligence background workers active, or explicitly inventoried as disabled with rationale;
  - Settings frontend performance marks for first usable render and status-section completion;
  - labels separating `db_read` queue wait, `db_read` execution, `db_write` queue wait, `db_write` execution, and `open_fresh_serialized`.

### AC2 — Foreground Read Purity

- `get_account_detail` performs no direct, awaited, spawned, scheduled, or post-return mutation caused by the detail read.
- `get_meeting_intelligence` performs no direct, awaited, spawned, scheduled, or post-return mutation caused by the briefing read.
- The live-calendar-only meeting branch is handled without auto-persisting before the briefing payload returns.
- Add behavioral tests plus a static check for these two routes; static checks alone are not sufficient.

### AC3 — Background Writer And Calendar Hot-Path Discipline

- Background workers active during the proof run are inventoried and classified as service-owned writer, read-only `db_read`, or explicitly out-of-scope with rationale.
- Calendar poller attendance/person/meeting mutations in the sampled hot path route through services.
- Attendance is batched or transaction-bounded.
- Batch computation and de-duplication happen outside the writer closure.
- The writer closure is DB-only; filesystem/artifact side effects happen after commit.
- No per-attendee foreground-independent write loop in the sampled path can hold an untracked SQLite write lock for a full busy-timeout window.

### AC4 — Settings Status DB Access And Conditional Fan-Out

PR1 requirements:

- `get_context_mode`, `get_gravatar_status`, and `get_linear_status` no longer use fresh writable opens for status reads.
- Settings status commands are included in latency rollups.
- PR1 proof records Settings first usable render before and after backend status cleanup.

PR2 requirements, only if PR1 measurements still miss the Settings target or a separate frontend follow-up is opened:

- Settings first render does not mount dev diagnostics or every below-fold status section synchronously.
- Connector status checks are concurrency-limited.
- User can interact with first-screen settings while slower diagnostics continue.

### AC5 — No Phase 1 Cache Or Separate Read DB

- No new database file, sync process, keychain item, export path, or purge path is introduced in Phase 1.
- No new persistent read-model table, materialized projection, or cross-render cache is introduced in Phase 1.
- Any Phase 2 read model requires an explicit packet amendment with migration slot claim and Intelligence Loop answers.

### AC6 — Regression Tests

- Rust tests cover account and meeting foreground read purity.
- Rust/service tests cover calendar batch write behavior and signal preservation.
- Frontend tests cover Settings lazy/deferred behavior and connector status concurrency if PR2 is triggered.
- Latency instrumentation tests prove tags are PII-free and rollups cap bounded samples.
- Touched poller/batch diagnostics have a static or unit check proving they do not log meeting titles, emails, domains, company names, person names, claim text, raw SQL bind values, or provider payload snippets.

## 9. Test Plan

Commands before L2:

```bash
cargo clippy -- -D warnings
cargo test
pnpm tsc --noEmit
pnpm test
```

Focused checks:

- Unit tests for `DbService` timing wrappers without relying on real sleep.
- Service tests for calendar attendance batch idempotency.
- Account detail test proving lifecycle reconciliation is not triggered by the detail read.
- Meeting detail tests proving live-calendar-only fallback, review flag changes, and new-signal flag changes are not triggered by the briefing read.
- Settings status command tests proving `get_context_mode`, `get_gravatar_status`, and `get_linear_status` use read-only paths or non-DB config state.
- Settings component tests for lazy section mount and connector concurrency cap if PR2 is triggered.
- Manual/local proof: run app with background poller enabled, navigate Settings/account/meeting, collect latency rollups and a short process sample if stalls remain.

## 10. K-In Findings

Mandatory substrate grep hits:

- `.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md` already established p95 command rollups and hot-read degradation as the intended latency strategy. This packet extends that existing direction; it does not invent a new telemetry substrate.
- `.docs/decisions/0062-briefing-artifacts-vs-live-queries.md` cautions that caching should follow profiling, not precede it. Phase 1 therefore forbids read models and cross-render caches.
- `.docs/decisions/0101-service-boundary-enforcement.md` explicitly names `ActionDb::open()` outside services as the root of service-boundary bypasses. The sampled calendar poller path is the same class.
- `.docs/decisions/0102-abilities-as-runtime-contract.md` says abilities never open DB connections and mechanical reads remain services/commands. Phase 1 keeps ability/runtime changes out of scope and moves mutation discipline back toward services.
- `.docs/decisions/0103-maintenance-ability-safety-constraints.md` forbids slow provider/external work inside transactions and treats maintenance latency as background-budgeted. This packet applies the same principle to local DB writes: synthesize/read first, then write in bounded service transactions.
- `.docs/decisions/0104-execution-mode-and-mode-aware-services.md` confirms long SQLite write transactions block foreground work; lock scope is therefore an acceptance criterion, not an implementation detail.
- `.docs/decisions/0111-surface-independent-ability-invocation.md` requires surface-client identity, scopes, revocation, and audit attribution. Any later projection/cache must key by principal/scope, not only actor class.
- `.docs/decisions/0130-surface-independent-composition-contract.md` defines derived compositions/projections as consumers of claims/signals/trust, not new authority. Phase 2 read models must follow that pattern.
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` warns that grep/static checks inside one crate are porous. AC2 therefore requires behavioral tests in addition to static checks.
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` is relevant to any Phase 2 claim projection consumed by prompt channels. Read-model rows must preserve sensitivity gates and actor/surface policy.
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` supports narrowing Phase 1 rather than growing this packet into a cache/CQRS redesign.

## 11. L0 Review Status

V0.1 review panel ran with four independent reviewers:

- Adversarial challenge: request changes.
- Architecture/performance reviewer: request changes.
- Security/CSO reviewer: request changes.
- Scope guardian: request changes.

V0.2 resolves those findings by:

- removing hidden post-return writes from foreground `get_*` routes;
- adding the live-calendar auto-persist branch to the meeting read-purity scope;
- narrowing the background-worker claim to inventoried proof-run workers and the sampled calendar hot path;
- adding backend Settings status-read cleanup before frontend fan-out work;
- forbidding Phase 1 caches/read models;
- adding Phase 2 principal/audience binding requirements;
- making measurement reproducible.
- splitting PR1 Settings status cleanup from PR2 conditional frontend fan-out.

Follow-up review approved V0.3:

- Adversarial challenge: approved.
- Architecture/performance reviewer: approved.
- Security/CSO reviewer: approved.
- Scope guardian: approved after AC4 and AC6 were split into PR1-required and PR2-conditional criteria.

L0 is approved for L1 implementation against the PR1 boundary in this packet.

## 12. Resolved Review Questions

1. **Phase 1 scope:** PR1 is backend contention and measurement. Settings UI fan-out is PR2 unless backend-only measurements still miss the target.
2. **Instrumentation level:** command/service labels plus `db_read`/`db_write` queue/execution/open timing are required. Caller-tagged `ActionDb::open()` can be added only where it materially helps the proof bundle.
3. **Post-return mutations:** rejected. Foreground `get_*` routes must not cause direct, awaited, spawned, scheduled, or post-return mutations.
4. **Phase 2 read model:** not selected. A later amendment must use PR1/PR2 measurements to name the narrowest failing read before proposing tables.
