# W4-Sub L0 packet — substrate writer-contention sweep + scheduler

Date: 2026-05-16 (V2)
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 4 — substrate hardening discovered during W4-A L4
Issue: TBD (Linear ticket to be filed concurrent with L0)
Linear: TBD
Working branch: TBD
Worktree: TBD (`/private/tmp/dailyos-w4-sub`)

This packet captures the W4-Sub plan contract for L0 review. Linear remains
the canonical execution surface.

W4-Sub is the substrate hardening sweep prompted by W4-A L4 attempts that
exposed substrate-quality blockers preventing the W4-A Gutenberg block from
rendering against a live runtime. The sweep folds in four small substrate
parity gaps and a writer-contention sweep, with a single pilot ability
migration that establishes the pattern for follow-up source-poller migration.

W4-Sub is NOT a feature wave. It is the substrate hardening that turns the
W4-A acceptance §59 + §60 ("L1 proof includes screenshots or rendered HTML
under DailyOS theme and stock theme" + "stale banner case and tamper banner
case") from structurally-uncrossable into reliably-passable.

## V1 → V2 changelog (folded L0 findings)

V1 went through L0 lanes /plan-eng-review, /codex challenge, /codex consult,
/cso on 2026-05-16. Cross-model consensus on 18 findings drove V2. The
material changes from V1:

1. **A2 inverted.** V1 proposed migrating the WP plugin from
   `/v1/surface/project-composition` to `/v1/surface/invoke`. Codex challenge
   + codex consult + /cso all flagged that this would bypass the W4-D
   projection contract, the cache layer, and the projection-signing
   boundary, returning raw `AbilityOutput` instead of the required
   `ProjectedComposition + cache_hint_token`. V2 inverts: the runtime adds
   `/v1/surface/project-composition` as a canonical orchestrator route.
2. **AC §60 mis-cited.** V1 referenced AC §60 as "screenshots." Current
   W4-A packet has §59 = screenshots, §60 = stale + tamper banners. V2 AC13
   covers both items.
3. **A4 silent-site survey re-anchored.** V1 cited line 1075 as a critical
   HMAC swallow site; that line is session-refresh JSON parse, and HMAC
   failures already log via `log_signing_failure` at line 842 / call site
   727. V2 re-defines A4 as a survey commitment from current dev with no
   pre-baked line citations, plus a CSO-mandated log-hygiene CI lint.
4. **`pause_for` correctness claim dropped.** V1 claimed p95 < 500ms
   bounded latency. Codex challenge + consult: `db_write` is a FIFO mpsc
   queue; pause cannot preempt in-flight writes or reorder already-queued
   writes. V2 frames `pause_for` as best-effort throttling with explicit
   non-correctness semantics. A separate optional B+ tier (DbService
   priority lane) is named but deferred to a follow-up wave.
5. **Class C reduced.** V1 migrated 6 source pollers to abilities-runtime
   in one wave. Consensus flagged this as wide and not load-bearing for L4
   unblock. V2 ships one pilot poller in W4-Sub (Quill — smallest,
   simplest) and stages the remaining 5 as W4-Sub-2 with its own L0.
6. **`recurrence` moved off `AbilityPolicy`.** V1 added `recurrence` to
   `AbilityPolicy`. Consensus: scheduler metadata doesn't belong in
   invocation policy. V2 introduces a separate `RecurringAbilityDescriptor`
   discovered from the ability inventory, not from AbilityPolicy.
7. **ADR-0067 amendment promoted to pre-L0 dependency.** V1 deferred the
   amendment to L3 paperwork. /cso: the fairness policy must be named
   before B1 can be evaluated. V2 names the ADR-0067 amendment as a
   precondition to W4-Sub implementation; amendment is part of L0 package.
8. **Line citations re-verified.** V1 had stale citations. V2 cites:
   - `is_safe_ability_name` at `surface_runtime/mod.rs:2298`
   - HMAC log site `log_signing_failure` at `surface_runtime/mod.rs:842`,
     called at `:727-728`
   - Diagnostic log line (added during chase) at
     `surface_runtime/mod.rs:796-805`
   - 12 true background spawn sites in `lib.rs` (323 was
     `SurfaceRuntimeEndpoint`, foreground HTTP infra — excluded)
9. **Queue pause claim corrected.** V1 claimed `MeetingPrepQueue.pause()`
   and `EmbeddingQueue.pause()` existed. Only `IntelligenceQueue` has
   `pause()/resume()/drain_pending()` (at `intel_queue.rs:182-204`). V2
   either limits the coordination claim to `IntelligenceQueue` only OR
   adds pause checkpoints to the other queue processors (decision in AC8).
10. **A1 leading-slash policy named.** V2 explicitly rejects
    `/leading-slash` and `//double-slash`, accepts `vendor/name` form only.
11. **B1 pause semantics chosen.** V2 uses wall-clock-clamped (max-wins)
    semantics — single global `last_pause_started_at + cap` AtomicU64;
    pause requests within active window extend up to cap, no stacking. Rate
    of pause calls is irrelevant.
12. **B5 cadence Tauri commands scoped.** V2 marks Tauri-only, admin-gated
    setter, CI assertion of MCP-inventory absence.
13. **B5 cadence UI dropped.** V1 added cadence-tuning UI in System Status.
    Conflicts with the Tauri React UI freeze
    (`feedback_tauri_ui_freeze`, 2026-05-15). V2 ships config file +
    env override only; no UI.
14. **C1 cadence floor named.** V2 adds `MIN_RECURRENCE_INTERVAL = 30s`
    enforced at scheduler-registration (macro-time enforcement out of scope
    because `recurrence` moved off AbilityPolicy).
15. **C2 actor attribution named.** V2 introduces
    `Actor::RecurringSource { source }` per ADR-0102 §7.1; per-source
    `required_scopes` mirror user-initiated refresh scopes.
16. **v179 migration form.** SQLite has no `ADD COLUMN IF NOT EXISTS`. V2
    uses a Rust-gated migration following the v172-style pattern in
    `src-tauri/src/migrations/v172_dos_567_w4b_versions_and_outbox.rs`.
17. **Spawn count corrected.** V2 references 12 background workers (lib.rs
    sites 358, 367, 376, 385, 394, 403, 414, 423, 432, 440, 447, 454),
    explicitly excluding `SurfaceRuntimeEndpoint` at :323 which is
    foreground HTTP infra.
18. **Survey-from-dev commitment added.** V2 AC4 + AC5 commit to
    re-surveying actual silent Err arms and existing primitives against
    `dev` before implementation kickoff, not from V1's stale notes.

## Status snapshot

- W4-Sub is wave-4 expansion, not a new wave number. Runs after
  W4-A0/B/C/D/E (merged) and runs in parallel with W4-A rendering work
  that still cannot complete L4.
- The scope was discovered during the W4-A L4 chase session on 2026-05-16
  documented in `tasks/handover-v1.4.2-w4-shipped-l4-blocked.md`.
- This packet is documentation only.
- This packet does not implement code.
- This packet does not commit.
- This packet does not create a PR.
- W4-Sub implementation tasks are blocked by:
  - Unanimous L0 APPROVE on this V2 packet across /plan-eng-review,
    /codex challenge, /codex consult, /cso.
  - Filing of ADR-0067 amendment naming Stage 2.5 fairness policy
    (no preemption; best-effort pause; FIFO writer queue retained).

## Load-bearing outcome

```text
DailyOS runtime is foregrounded
  -> WP plugin's existing project_composition_for_surface() call to
     /v1/surface/project-composition succeeds because the runtime now
     exposes that orchestrator route
  -> orchestrator route runs: signed-session validate → cache lookup →
     invoke account_overview ability → W4-D scope-filter projection →
     projection signing (W4-C) → cache store → return ProjectedComposition
     + cache_hint_token
  -> handshake / session refresh / signed-session validation all acquire
     the writer with best-effort BackgroundScheduler pause hint reducing
     contention during user-initiated bursts (no latency guarantee)
  -> on every Err arm in any signed route, log::warn! with request_id
     and error.code() + hashed identifiers makes the underlying cause
     visible without leaking secrets
  -> 12 background workers run through BackgroundScheduler with cadence
     class + startup jitter + best-effort pause hooks; Quill poller is the
     pilot ability migration demonstrating the pattern for W4-Sub-2
  -> WP renders the returned ProjectedComposition in the Gutenberg block
     editor (DailyOS theme + stock theme + stale banner + tamper banner
     cases all proven)
```

The outcome unblocks W4-A AC §59 + AC §60, fixes the four substrate parity
gaps that surfaced during L4 chase, reduces (not eliminates) writer
contention, and establishes the abilities-runtime recurring-invocation
pattern via one pilot migration.

## Scope (what we will do)

W4-Sub bundles four substrate work classes. All are AC §59/§60 violations
because each independently blocks the L4 render path. None defer to
v1.4.3+ per CLAUDE.md "no deferrals — period." Class C is reduced from V1
to a single pilot; the rest of source-poller migration lives in W4-Sub-2.

### Class A — substrate parity gaps (small fixes that each block render)

**A1. `is_safe_ability_name` allow-list missing `/`.** The validator at
`src-tauri/src/surface_runtime/mod.rs:2298` allows
`[a-zA-Z0-9._\-:]` but not `/`. The ability is canonically registered as
`dailyos/account-overview`
(`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`); bridge
tests use slash form
(`src-tauri/src/bridges/surface_client.rs:1530`); WP plugin payloads use
slash form. Validator rejects every well-formed ability invoke. Fix: add
`b'/'` to the allow-list match, add regression test covering:

- `is_safe_ability_name("dailyos/account-overview") == true`
- `is_safe_ability_name("a/b/c") == true` (multi-segment)
- `is_safe_ability_name("a/b") == true`
- `is_safe_ability_name("/leading-slash") == false` (defense-in-depth;
  registry would reject anyway, but explicit policy)
- `is_safe_ability_name("//double-slash") == false` (defense-in-depth)
- `is_safe_ability_name("a/b ") == false` (space still rejected)
- `is_safe_ability_name("a/b@") == false` (other punctuation rejected)
- Empty string and >128 chars still rejected

**A2. Runtime adds `/v1/surface/project-composition` orchestrator route.**
The WP plugin's `project_composition_for_surface()` at
`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:116-137`
posts to `/v1/surface/project-composition` and expects a
`ProjectedComposition + cache_hint_token` response per the W4-A contract
(`render-functions.php:55`). The runtime's signed-route allow-list at
`surface_runtime/mod.rs:815-826` does not include
`/v1/surface/project-composition` — `is_supported_signed_route` only knows
`/v1/surface/invoke`, `/v1/surface/feedback`, `/v1/surface/abilities`,
`/v1/surface/keyring`, plus event-log + pairing-status GETs + nonce routes.

V1 proposed migrating WP to `/v1/surface/invoke`. V2 inverts because:
- W4-A contract requires `ProjectedComposition + cache_hint_token`, not
  raw `AbilityOutput`
  (`.docs/plans/dos-546/v1.4.2-project/W4-A-L0-packet.md:542`).
- Direct invoke bypasses W4-D scope-filter projection, W4-C projection
  signing, the surface-side cache, and the audit-drain contract.
- The orchestrator route is the W4-A producer/renderer split.

Fix: add `/v1/surface/project-composition` as a canonical signed POST
route in `is_supported_signed_route`. Implement the route handler in
`surface_runtime/mod.rs` performing:

1. Signed-session validate (existing path)
2. Cache lookup by `(composition_id, composition_version,
   cache_hint_token)`
3. On cache miss: invoke `dailyos/account-overview` (or whatever ability
   the `composition_id` resolves to) via the existing
   `invoke_registry_json_for_actor` path
4. Apply W4-D scope-filter projection
5. Sign the projection via W4-C
6. Cache store with cache_hint_token rotation
7. Return `ProjectedComposition + cache_hint_token` per the W4-A
   transport contract shape

WP plugin transport unchanged — `project_composition_for_surface()`
already constructs the correct request shape and parses the correct
response shape.

**A3. Schema drift — `surface_client_sessions.throttled_until_at`.**
Migration 169 at
`src-tauri/src/migrations/169_dos_559_surface_client_pairings.sql:79`
declares the column but was edited post-deploy. DBs that ran the original
migration are missing it. `load_session_pairing` at
`src-tauri/src/services/surface_pairing.rs:1606` selects
`s.throttled_until_at` and fails with `no such column`. Fix: add a
Rust-gated migration v179 following the `v172_dos_567_w4b_versions_and_outbox.rs`
pattern at `src-tauri/src/migrations/v172_dos_567_w4b_versions_and_outbox.rs`.
The migration checks for column presence via `PRAGMA table_info` and
ALTERs only if missing. Idempotent on both fresh installs (column present)
and partial-applied DBs (column missing). Integration test covers both
paths, plus an end-to-end test: on a DB missing the column, after v179
runs, `record_signed_transport_failure` writes a throttle that
`load_session_pairing` honors on the next request.

**A4. Silent error swallowing in surface_runtime Err arms — re-surveyed.**
V1's specific line citations were stale. V2 commits to a survey from
current `dev` (specifically the `wave3-l2-integration` worktree as the
working base):

- Inventory every Err arm in `src-tauri/src/surface_runtime/mod.rs` that
  returns `error_response(SurfaceHttpError::...)` without a preceding
  `log::warn!` within the same arm.
- Classify each by route + auth class (pairing, session, signed-route,
  nonce, healthcheck).
- Add `log::warn!` with `target: "dailyos_lib::surface_runtime"`,
  `request_id`, and `error.code()` (NOT raw `{error}`) plus hashed
  identifiers (mirror `log_signing_failure` at `surface_runtime/mod.rs:842`
  pattern: `session_id_hash`, `surface_client_id_hash`).
- **/cso log-hygiene constraint (Finding 3):** in any arm under
  `pairing_handshake_response`, `surface_session_refresh_response`,
  signed-route dispatch, or session-key handoff, the log line MUST use
  `error.code()` or a `privacy_hash(...)` wrapper, NEVER raw `{error}`
  interpolation. Raw `Display` for these error types may echo input
  fragments (session IDs, pairing codes, HMAC material).
- CI lint at `tests/lint/surface_route_err_logging.sh` enforces:
  1. Every Err → `error_response(SurfaceHttpError::...)` site has a
     preceding `log::warn!` within the same arm.
  2. No `{error}` interpolation appears in `log::*` calls inside Err arms
     under the four sensitive routes above (use `error.code()` instead).

### Class B — BackgroundScheduler service (architectural, narrowed from V1)

**B1. Introduce `BackgroundScheduler` service.**
`src-tauri/src/services/background_scheduler.rs` (new). Thin layer on top
of the existing `task_supervisor::spawn_supervised` primitive
(`src-tauri/src/task_supervisor.rs:6-34`). Responsibilities trimmed from V1:

- **Registry**: every background worker registers a `BackgroundWorker
  { name, cadence_class, wake_source, run }` record.
- **Cadence classes**: `SourcePoll(interval_minutes)`,
  `QueueDrain(idle_poll_ms)`, `OnSignal(wake_source)`,
  `IntervalLoop(min_interval, max_interval, jitter_pct)`. Replaces ad-hoc
  sleep/wake mix across 12 worker files.
- **Startup jitter**: random offset (0 .. interval/4) per worker so
  co-spawned workers don't cluster their wake-ups.
- **Best-effort pause hint**: `scheduler.pause_for(Duration)` and
  `PauseGuard` drop-resume. Workers check `scheduler.is_paused()` between
  iterations — NOT mid-transaction. Pause does NOT preempt the writer
  mutex or reorder queued writes. Frames as throttling-aid, not
  correctness primitive (per L0 finding 4 + 7 + /cso finding 4 + 10).
- **Pause semantics (V2 chosen):** wall-clock-clamped, max-wins. Single
  global `last_pause_until: AtomicU64` (unix-millis). `pause_for(d)` sets
  `last_pause_until = max(last_pause_until, now + min(d, CAP))` where
  `CAP = Duration::from_secs(5)`. Workers `is_paused()` returns
  `now < last_pause_until.load()`. Rate of pause calls is irrelevant —
  the cap is wall-clock. Eliminates the V1 DoS surface per /cso Finding 4.
- **Activity integration (B4 below)** — scheduler reads
  `ActivityMonitor::level()` and applies a multiplier per cadence class.

**Explicitly NOT in B1 (deferred to W4-Sub-2 or beyond):**
- DbService priority lane. The current FIFO mpsc model in
  `src-tauri/src/db_service.rs:116` cannot guarantee foreground-write
  latency bounds. Adding a priority lane is materially larger scope and
  needs its own L0. V2 names this as the "B+ tier" — known future work,
  not part of W4-Sub.
- Cadence-tuning Tauri UI. Conflicts with Tauri React UI freeze. V2
  ships config file + env override only.

**B2. Migrate 12 supervised background tasks into BackgroundScheduler.**
The 12 background `task_supervisor::spawn_supervised(...)` sites in
`src-tauri/src/lib.rs` become `scheduler.register(...)`:
- `:358` CalendarPoller
- `:367` EmailPoller
- `:376` CaptureLoop
- `:385` IntelProcessor
- `:394` MeetingPrepProcessor
- `:403` EmbeddingProcessor
- `:414` HygieneLoop
- `:423` QuillPoller
- `:432` GranolaPoller
- `:440` EnrichmentProcessor
- `:447` LinearPoller
- `:454` DrivePoller

Explicitly **excluded**: `SurfaceRuntimeEndpoint` at `:323`. That's the
foreground HTTP server. Putting it in a scheduler that pauses for surface
operations risks self-interference (codex-challenge Finding 6).

The 9 Notify wake fields on `IntegrationState`
(`src-tauri/src/state.rs:135-149`) are preserved as `OnSignal` wake
sources. The 5 ResourcePermits semaphores
(`src-tauri/src/state.rs:263-292`) remain in place (concurrency-gate
layer separate from scheduling).

**B3. Fix the 250ms idle-poll storm.**
`src-tauri/src/services/invalidation_jobs.rs:15`
`const TARGETED_REPAIR_IDLE_POLL_MS: u64 = 250` busy-loops the writer 4
times/sec even when there's no work. Raise to 5000ms. Worker registers
as `QueueDrain { idle_poll_ms: 5000 }` against BackgroundScheduler.

**B4. Reverse the activity-adaptive inversion.**
`src-tauri/src/activity.rs:137-143`
`adaptive_network_interval` has `Active=120s / Idle=60s / Background=30s`
— more polling when less attended. Replace with
`Active=60s, Idle=300s, Background=1800s`. Integrate through
BackgroundScheduler's `ActivityMultiplier` policy. Callers in
`src-tauri/src/google_drive/poller.rs:81` migrate to consume through
the scheduler.

**B5. Sensible cadence defaults via config + env, no UI.**
Defaults live in `BackgroundCadenceConfig::default()` in
`background_scheduler.rs`. Configurable via `~/.dailyos/config.json`
extension; env override `DAILYOS_BACKGROUND_CADENCES_JSON` (for dev/test);
no Tauri command, no React UI per `feedback_tauri_ui_freeze`.

Defaults:
- Calendar: 15 min (no change)
- Email: 15 min (was 5 min)
- Drive: 60 min (was activity-adaptive 30-120s)
- Linear: 60 min (no change)
- Granola: 15 min (was 10 min)
- Quill: 15 min (was 5 min)
- Glean enrichment: woken on signal OR 60-min sweep

Each source still wakes on its `Notify` when a user-initiated trigger
needs fresh data — cadence is the floor, not the only path.

**B6. Pause checkpoints in non-IntelligenceQueue processors.**
V1 falsely claimed `MeetingPrepQueue.pause()` and `EmbeddingQueue.pause()`
existed. Only `IntelligenceQueue` has pause/resume/drain at
`intel_queue.rs:182-204`. V2 adds checkpoint reads of
`scheduler.is_paused()` in:
- `src-tauri/src/meeting_prep_queue.rs` — before each `dequeue` call and
  between per-item processing.
- `src-tauri/src/processor/embeddings.rs` — same.

The processors continue their existing in-flight work to completion but
pause before starting the next iteration. This is purely advisory — no
correctness guarantee, no preemption.

### Class C — pilot poller migration (Quill only)

**C1. Define `RecurringAbilityDescriptor` (NOT on AbilityPolicy).**
V1 added `recurrence` to `AbilityPolicy`. Consensus across codex-challenge
(Finding 3), codex-consult (Finding 4), and the survey
(`AbilityPolicy` is invocation policy per ADR-0102 §7.1, not scheduler
metadata): scheduler metadata belongs on a separate descriptor.

V2 introduces `RecurringAbilityDescriptor` in
`src-tauri/abilities-runtime/src/registry.rs` (or equivalent canonical
location adjacent to the inventory). The descriptor carries:
- `ability_name: String`
- `recurrence: RecurrencePolicy { cadence_class, wake_source,
  pause_respect }`
- `actor: Actor::RecurringSource { source: String }` (see C3)
- `required_scopes: Vec<SurfaceScope>`

Discovered by BackgroundScheduler at startup via a new inventory
iterator method. Does NOT modify `AbilityPolicy` schema. Preserves W1-B
canonical schema invariant.

**Cadence floor (per /cso Finding 6):**
`MIN_RECURRENCE_INTERVAL = 30s`. Enforced at scheduler-registration time:
any `RecurringAbilityDescriptor` with `cadence < MIN_RECURRENCE_INTERVAL`
fails registration with an error. L2 lint test asserts the pilot
descriptor's cadence ≥ floor.

**C2. Pilot migration: Quill poller as recurring ability.**
Quill is the smallest, simplest source. Migration:
- New file `src-tauri/abilities-runtime/src/abilities/sources/quill_poll.rs`
  declaring the ability with `#[ability]` macro.
- Body of the ability wraps the per-cycle logic of the existing
  `run_quill_poller` in `src-tauri/src/quill/poller.rs:28`.
- New `RecurringAbilityDescriptor` registered with the inventory.
- BackgroundScheduler discovers the descriptor at startup and dispatches
  per cadence.
- Existing `task_supervisor::spawn_supervised("QuillPoller", ...)` at
  `lib.rs:423` removed.
- Cursor/state preservation: the existing poller's state lives in
  `IntegrationState`; the ability body reads/writes the same state. No
  data migration needed.

5 other source pollers (Calendar, Email, Drive, Linear, Granola) are
explicitly **out of scope** for W4-Sub. They live in W4-Sub-2 with its
own L0. Reasoning: 6-poller migration is wide, not load-bearing for L4
unblock, and benefits from learning from the pilot before fan-out.

**C3. `Actor::RecurringSource` variant.**
Per /cso Finding 7, the recurring ability needs distinct actor
attribution to preserve scope/audit semantics. V2 introduces
`Actor::RecurringSource { source: String }` in
`src-tauri/abilities-runtime/src/actor.rs` (adjacent to existing
`Actor::SurfaceClient` per W1-A).
- The Quill pilot ability runs under
  `Actor::RecurringSource { source: "quill" }`.
- `required_scopes: vec!["read.transcript", ...]` (the same scopes a
  user-initiated Quill refresh requires).
- `RecurringSource` is NOT a privilege escalation — it goes through the
  same `ensure_required_scopes` gate at
  `src-tauri/src/bridges/surface_client.rs:956`.
- L2 lint test asserts the pilot ability does not declare
  `required_scopes: []` and routes through scope enforcement.

**Substrate ownership of source-poller writes:**
The Quill pilot pattern demonstrates that source-poller writes can flow
through abilities-runtime. Per codex-consult Finding 6, NOT every
source-poller write is a `commit_composition` — pollers write emails,
meetings, captures, sync tokens, transcript metadata. The pilot's
boundary is: composition-state writes go through `commit_composition`;
source-state writes go through typed source services with the same
actor/scope attribution. AC12 names the boundary check.

### Class D — ADR-0067 amendment (pre-L0 dependency)

**D1. File ADR-0067 amendment naming Stage 2.5 fairness policy.**
Per /cso Finding 10, the existing ADR-0067 staged-split-lock strategy
does not authorize the BackgroundScheduler pause primitive without an
amendment naming the fairness policy. Amendment text (draft):

> **Stage 2.5 — Best-effort pause hint over FIFO writer queue.**
> BackgroundScheduler (W4-Sub) introduces `pause_for(Duration)` as a
> best-effort hint that background workers SHOULD check between
> iterations. The writer mutex semantics are unchanged: single
> connection, FIFO mpsc queue (`src-tauri/src/db_service.rs`). Pause
> does NOT preempt in-flight writes, does NOT reorder queued writes,
> does NOT provide latency guarantees to foreground operations.
> Foreground writers benefit when background workers cooperate; they
> are not isolated from background contention by `pause_for`.
>
> **Stage 3 (deferred) — DbService priority lane.** Real
> foreground/background isolation requires a priority-aware writer
> queue or split-pool architecture in DbService. Triggers for
> activation: BackgroundScheduler `pause_for` proves insufficient in
> production observation (p95 foreground write > 1s under nominal
> background load), or a class of latency-bound surface workflows
> emerges that cannot tolerate best-effort semantics. Deferred to a
> dedicated wave with its own L0.

ADR amendment lives at `.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md`
as a `## Amendment 1 — Stage 2.5 (2026-05-16)` section.

W4-Sub implementation does NOT begin until ADR amendment is filed and
acknowledged by /cso lane.

## Out of scope (what we will NOT do)

- **No new sources, no new pollers.** Migration only; no new external
  systems.
- **No 5-of-6 source-poller abilities.** Calendar, Email, Drive, Linear,
  Granola move to W4-Sub-2. Quill is the pilot.
- **No DbService priority lane / split-pool / writer-queue overhaul.**
  Stage 3 of ADR-0067 deferred to its own wave.
- **No Tauri React UI changes.** Per `feedback_tauri_ui_freeze`
  (2026-05-15). Cadence-tuning is config + env, no UI.
- **No abilities-runtime contract changes beyond `RecurringAbilityDescriptor`
  + `Actor::RecurringSource`.** `AbilityPolicy` schema stays frozen per
  W1-B / ADR-0102.
- **No changes to `IntelligenceQueue` pause/drain semantics.**
  BackgroundScheduler coordinates with `IntelligenceQueue.pause()` /
  `.resume()` / `.drain_pending()` via existing public methods. No
  absorption.
- **No new ADRs beyond the ADR-0067 amendment.** All other work executes
  within ADR-0067 (as amended), ADR-0102, ADR-0105, ADR-0129, ADR-0058.
- **No PII in commit messages, PR titles, code comments, or test
  fixtures.** Per CLAUDE.md cardinal rule.
- **No deferrals.** "No deferrals — period." Reduction of Class C from 6
  pollers to 1 pilot is scope-reshaping per consensus L0 finding, not
  deferral — the remaining 5 land in W4-Sub-2 with their own L0 and
  Linear ticket, not "later."

## Upstream contracts (what we depend on)

- **`task_supervisor::spawn_supervised`** at
  `src-tauri/src/task_supervisor.rs:6-34` — supervision + panic-recovery
  primitive. W4-Sub extends; does not replace.
- **`IntegrationState`** wake fields at `src-tauri/src/state.rs:135-149`
  — 9 `Notify` channels. W4-Sub registers them as `OnSignal` wake
  sources.
- **`ResourcePermits`** at `src-tauri/src/state.rs:263-292` — 5
  `Semaphore` serialization gates. W4-Sub leaves them in place.
- **`db_write` / `db_read`** helpers per ADR-0067 (and its amendment) —
  single writer connection, FIFO mpsc queue. W4-Sub's `pause_for` is a
  best-effort hint over this; does NOT change helper signatures or
  queue semantics.
- **`commit_composition`** at
  `src-tauri/src/services/compositions.rs:67` — W4-B chokepoint for
  composition version assignment. The new
  `/v1/surface/project-composition` orchestrator route calls
  `invoke_registry_json_for_actor` which routes through this for
  composition writes.
- **W4-D scope-filter projection** + **W4-C projection signing** — the
  orchestrator route applies these in sequence before returning
  `ProjectedComposition + cache_hint_token`.
- **`#[ability]` macro** per ADR-0102 §7.1, W1-B canonical schema, and
  `src-tauri/abilities-macro/src/lib.rs`. W4-Sub uses unchanged.
- **W4-A0 producer pattern** at
  `src-tauri/abilities-runtime/src/abilities/account_overview.rs` —
  template for the Quill pilot.
- **`IntelligenceQueue.pause()/.resume()/.drain_pending()`** at
  `src-tauri/src/intel_queue.rs:182-204` — coordinated with
  BackgroundScheduler via existing public methods.

## Risks (what could go wrong)

R1. **BackgroundScheduler becomes the new bottleneck.** Mitigation:
    registry is `RwLock` with read-bias; pause flag is single `AtomicU64`
    (max-wins); cadence config is `parking_lot::RwLock`; scheduler holds
    no lock during worker dispatch.

R2. **`pause_for` provides no latency guarantee.** This is now explicit
    per V2 (vs implicit in V1). Surface-route consumers MUST NOT rely on
    pause for correctness. Pause is throttling-aid. The L4 render proof
    must be achievable WITHOUT the writer being free — the orchestrator
    route's write path tolerates contention through normal busy_timeout
    behavior (5s default). If foreground latency proves insufficient in
    production, ADR-0067 Stage 3 activation is the structural fix.

R3. **`/v1/surface/project-composition` orchestrator route is new
    substrate.** New code path through cache + invoke + projection +
    signing. Mitigation: orchestrator composes existing primitives
    (invoke_registry_json_for_actor, W4-D projection, W4-C signing,
    cache); doesn't reinvent. Integration tests cover full round-trip
    via WP transport. /cso reviews route handler as trust-boundary diff.

R4. **`is_safe_ability_name` allow-list expansion.** Per /cso Finding 2,
    safe. Validator is defense-in-depth; registry lookup is the real
    gate. Regression test asserts only `/` added; all other
    non-allowed chars still rejected; `/leading-slash` and
    `//double-slash` explicitly rejected.

R5. **Migration v179 races concurrent waves.** v178 is the current
    highest. V2 reserves v179 in `.docs/plans/v1.4.2-waves.md` slot
    reservation table per
    `feedback_protocol_amendments_belong_in_protocol_doc`.

R6. **Activity-adaptive direction reversal changes behavior.** Users
    who left the app backgrounded with active Drive work see staler
    data. Mitigation: cadence is configurable via config file + env;
    wake source still fires on user-initiated triggers; default aligns
    with "personal intelligence" semantics.

R7. **Quill pilot ability migration breaks Quill sync continuity.**
    Cursor/state lives in `IntegrationState` and persists across
    migration. Mitigation: integration test asserts cursor preservation
    across the migration; rollback path is a one-commit revert.

R8. **`Actor::RecurringSource` introduces a new actor variant.** Per
    ADR-0102 §7.1, actor enum changes need careful auth-path review.
    Mitigation: /cso reviews; L2 lint asserts no `ensure_required_scopes`
    bypass; pilot ability declares non-empty `required_scopes`.

R9. **A4 log-hygiene sweep introduces secret leakage if done naively.**
    Per /cso Finding 3. Mitigation: CI lint enforces `error.code()` +
    hashed identifiers in sensitive arms; no raw `{error}` interpolation
    in pairing/session/signed-route/session-key paths.

R10. **ADR-0067 amendment is filed but interpreted differently in
     implementation.** Mitigation: amendment text is in V2 packet
     verbatim; implementation reviewers (L2) check
     `BackgroundScheduler::pause_for` implementation matches the named
     semantics (best-effort, no preemption, no reordering).

## Intelligence Loop fit (CLAUDE.md 5-question mandatory check)

1. **Claim model.** No new claims. W4-Sub does not introduce new
   entities or subject types. Existing claims continue through
   `commit_composition`; Quill pilot ability calls the same claim-write
   services as the legacy poller.

2. **Provenance + trust.** No new provenance fields. `source_asof`,
   source attribution, trust scoring preserved. Quill pilot migration
   preserves provenance carriage via `AbilityOutput<Composition>` shape.

3. **Signals + invalidation.** Class B BackgroundScheduler preserves
   the existing `Notify` wake mechanism. No new signal types. No
   invalidation propagation paths change. Quill pilot ability emits
   the same signals the legacy poller emits today.

4. **Runtime + surfaces.** Class A2 orchestrator route is the canonical
   composition-render entry point — observable through MCP only via
   the existing surface-routes inventory. Class B BackgroundScheduler
   sits between `tokio::spawn` and worker fn, transparent to abilities,
   surface runtime, MCP. Quill pilot is discoverable as a recurring
   ability through the inventory (`get_dailyos_abilities`) but is NOT
   MCP-exposed unless the inventory has `mcp_exposure: …` set (it
   doesn't, by default).

5. **Feedback loop.** Quill pilot's failures emit through the
   abilities-runtime audit emission, same path that user corrections
   flow through. Fail-improve loop work (per ADR-0058) can learn from
   both pilot recurring runs and user-initiated invokes without
   per-source plumbing.

## Acceptance criteria

AC1. **A1 validator + tests.** `is_safe_ability_name` accepts
     `dailyos/account-overview` and the 8 regression cases listed in §A1.
     Regression test in `src-tauri/src/surface_runtime/mod.rs` tests
     module. Patch already applied in
     `wave3-l2-integration` worktree at line 2298; needs backport to PR
     #292 (DOS-572 W4-A).

AC2. **A2 orchestrator route.** Runtime exposes
     `POST /v1/surface/project-composition` as a canonical signed route
     in `is_supported_signed_route`. Handler performs the 7-step pipeline
     in §A2. Integration test in `src-tauri/tests/` covers cache miss +
     hit paths, invalid composition_id, signature verification, and
     end-to-end with the WP transport. PHPUnit tests in
     `wp/dailyos/tests/` cover the existing transport call against the
     new route. Runtime tests assert the response shape matches what
     `render-functions.php:55` expects.

AC3. **A3 migration v179 (Rust-gated).** New file
     `src-tauri/src/migrations/v179_dos_???_surface_sessions_throttled_until_at.rs`
     following the v172 pattern. Checks for column presence via
     `PRAGMA table_info`, ALTERs only if missing. Integration tests:
     - Fresh schema: no-op (column already present from 169).
     - Partial-applied DB (169 ran before column existed): ALTER adds
       column.
     - End-to-end: on a DB missing the column, after v179 runs, a
       freshly-issued throttle via `record_signed_transport_failure`
       lands in the column and is honored on the next
       `load_session_pairing` request (per /cso Finding 8).

AC4. **A4 silent-site sweep + CI lint.** Survey from current dev (the
     `wave3-l2-integration` worktree as working base). Inventory of
     silent Err arms in `surface_runtime/mod.rs` with file:line refs is
     produced as part of L1 self-validation. Every Err →
     `error_response(SurfaceHttpError::...)` site gains a preceding
     `log::warn!` with `request_id` and `error.code()`. In sensitive
     arms (pairing_handshake_response, surface_session_refresh_response,
     signed-route dispatch, session-key handoff), the log uses
     `error.code()` + hashed identifiers — never raw `{error}`. CI lint
     `tests/lint/surface_route_err_logging.sh` enforces both invariants.

AC5. **B1 BackgroundScheduler service.**
     `src-tauri/src/services/background_scheduler.rs` exists. Public API:

     ```rust
     pub struct BackgroundScheduler { /* ... */ }
     impl BackgroundScheduler {
         pub fn new(state: Arc<AppState>) -> Arc<Self>;
         pub fn register(&self, worker: BackgroundWorker)
             -> Result<RegisteredWorkerId, SchedulerError>;
         pub fn start_all(&self) -> Result<(), SchedulerError>;
         pub fn pause_for(&self, d: Duration) -> PauseGuard;
         pub fn is_paused(&self) -> bool;
         pub fn set_cadence(&self, worker_id: RegisteredWorkerId,
                            cadence: CadenceClass) -> Result<(), SchedulerError>;
         pub fn get_cadences(&self) -> Vec<(String, CadenceClass)>;
     }
     ```

     Pause semantics implemented per §B1 (wall-clock-clamped, max-wins,
     `CAP = Duration::from_secs(5)`). Tests cover register, start_all,
     pause_for return + cap behavior, is_paused state, set/get cadence
     round-trip.

AC6. **B2 migration of 12 supervised tasks.** `src-tauri/src/lib.rs`
     calls `scheduler.register(...)` 12 times in place of
     `task_supervisor::spawn_supervised(...)` at lines 358, 367, 376,
     385, 394, 403, 414, 423, 432, 440, 447, 454. `SurfaceRuntimeEndpoint`
     at :323 stays on direct `task_supervisor::spawn_supervised` (NOT
     registered with BackgroundScheduler). The 9 IntegrationState
     Notify fields remain on `state.rs:135-149` and are referenced by
     `OnSignal` cadence-class variants in the registrations.

AC7. **B3 idle-poll fix.** `TARGETED_REPAIR_IDLE_POLL_MS = 5000` (was
     250) at `src-tauri/src/services/invalidation_jobs.rs:15`. Worker
     registers as `QueueDrain { idle_poll_ms: 5000 }`.

AC8. **B4 activity-adaptive reversal.**
     `src-tauri/src/activity.rs:137-143` `adaptive_network_interval`
     reversed to `Active=60s, Idle=300s, Background=1800s`. Callers in
     `src-tauri/src/google_drive/poller.rs:81` migrate to consume
     through BackgroundScheduler's `ActivityMultiplier` policy.

AC9. **B5 config-only cadence defaults.** New cadence defaults match
     the table in §B5. Defaults in `BackgroundCadenceConfig::default()`.
     Config file extension at `~/.dailyos/config.json` with new
     `background_cadences` section. Env override
     `DAILYOS_BACKGROUND_CADENCES_JSON`. **No Tauri command, no React
     UI.** CI lint asserts no new Tauri command for cadence
     get/set exists.

AC10. **B6 pause checkpoints in non-IntelligenceQueue processors.**
      `meeting_prep_queue.rs` and `processor/embeddings.rs` add
      `if scheduler.is_paused() { sleep(short); continue; }` checks
      before each `dequeue` and between per-item processing. Tests
      assert checkpoint is honored.

AC11. **C1 `RecurringAbilityDescriptor` (NOT on AbilityPolicy).** New
      type in `src-tauri/abilities-runtime/src/registry.rs` carrying
      `ability_name`, `recurrence: RecurrencePolicy`, `actor`,
      `required_scopes`. Discovered via inventory iterator. AbilityPolicy
      schema unchanged. `MIN_RECURRENCE_INTERVAL = 30s` enforced at
      scheduler-registration with explicit error on violation.

AC12. **C2 Quill pilot ability.** New file
      `src-tauri/abilities-runtime/src/abilities/sources/quill_poll.rs`
      with `#[ability]` declaration, body wrapping
      `run_quill_poller`'s per-cycle logic. `RecurringAbilityDescriptor`
      registered. BackgroundScheduler dispatches per cadence. Legacy
      `task_supervisor::spawn_supervised("QuillPoller", ...)` at
      `lib.rs:423` removed. Integration test asserts cursor preservation
      across migration. CI grep gate at
      `tests/lint/quill_pilot_no_raw_db_writes.sh` asserts the pilot
      ability does not write the DB outside `commit_composition` /
      typed source services.

AC13. **C3 `Actor::RecurringSource` variant.** New variant in
      `src-tauri/abilities-runtime/src/actor.rs` per ADR-0102 §7.1.
      Pilot ability runs under
      `Actor::RecurringSource { source: "quill" }`. `required_scopes`
      declared per Quill's user-initiated refresh scope set. L2 lint
      asserts the pilot ability has non-empty `required_scopes` and
      routes through `ensure_required_scopes` at
      `src-tauri/src/bridges/surface_client.rs:956`.

AC14. **D1 ADR-0067 amendment filed.** New `## Amendment 1 — Stage 2.5
      (2026-05-16)` section in
      `.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md`
      with the verbatim text from §D1. Amendment filed before
      implementation begins.

AC15. **Render proof — W4-A AC §59 + §60 unblock.** After AC1-AC14,
      restart Tauri, re-pair WP, render the Gutenberg block in the WP
      editor for a known account. Capture:
      - §59: screenshots under DailyOS theme AND stock theme.
      - §60: stale banner case (composition_version older than
        current) AND tamper banner case (signature mismatch). Both
        cases verified through the new `/v1/surface/project-composition`
        orchestrator's projection-signing path.
      Proof attached to W4-Sub Linear ticket and to PR #292
      (DOS-572 W4-A).

## Test plan

T1. **Unit tests** for `is_safe_ability_name` (AC1), `BackgroundScheduler`
    register/start/pause-cap-behavior/cadence-roundtrip (AC5),
    `RecurringAbilityDescriptor` registration with MIN_RECURRENCE_INTERVAL
    rejection (AC11), pause checkpoints in queue processors (AC10).

T2. **Integration tests** for migration v179 idempotency + end-to-end
    throttle-write/throttle-read (AC3), `/v1/surface/project-composition`
    orchestrator route round-trip (AC2), WP transport against new route
    (AC2 / PHPUnit), Quill pilot ability dispatch through
    BackgroundScheduler with cursor preservation (AC12), pilot ability
    actor + scope enforcement (AC13).

T3. **No concurrency test for foreground p95 latency.** V2 dropped the
    latency guarantee per L0 finding. The render proof (AC15) is the
    end-to-end demonstration that the orchestrator route works under
    nominal background load; it does NOT assert a latency bound.

T4. **CI lint gates:**
    - `tests/lint/surface_route_err_logging.sh` (AC4)
    - `tests/lint/quill_pilot_no_raw_db_writes.sh` (AC12)
    - `tests/lint/no_cadence_tauri_commands.sh` (AC9)
    - `tests/lint/no_recurrence_on_ability_policy.sh` (AC11)

T5. **Manual L4 proof** for AC15 — screenshots + stale banner + tamper
    banner cases captured against a real WP install paired with a live
    runtime.

T6. **Suite S (security)** runs against integrated diff: validator
    expansion doesn't enable injection (AC1); `pause_for` cannot be
    abused to stall the system (AC5 wall-clock-clamped semantics);
    orchestrator route preserves auth ordering (AC2); pilot ability
    respects actor-scope/sensitivity/claim-write rules (AC13); A4 log
    sweep doesn't leak PII or secrets (AC4 lint).

T7. **Suite P (performance)** runs against integrated diff: scheduler
    dispatch overhead per iteration < 1ms; pause-flag read overhead
    < 100ns. **No latency guarantee for foreground writes** per V2
    framing.

T8. **Suite E (edge cases)** runs against integrated diff: crash mid-pause
    recovery (scheduler restart clears pause via AtomicU64 reset on
    init); duplicate-registration rejection (AC5); worker fn panic
    doesn't break scheduler (via task_supervisor); pilot ability cursor
    preservation across BackgroundScheduler restart (AC12).

## Rollout

R1. **Pre-implementation:** file ADR-0067 amendment per AC14. Re-survey
    silent-site inventory per AC4. Reserve migration slot v179 in
    `.docs/plans/v1.4.2-waves.md`. File Linear ticket for W4-Sub.

R2. **Stage 1 (Class A) lands first.** Four parity fixes
    (A1/A2/A3/A4) — independently mergeable. A1 already patched in
    working copy; needs backport to PR #292. A2 is the largest of the
    four (new substrate route). A3 + A4 are small. Once Class A lands,
    AC15 render proof becomes achievable with the existing scheduler
    stack (no Class B/C/D dependency for §59 screenshots; §60 stale +
    tamper banners need A2's projection-signing path).

R3. **Stage 2 (Class B + D) lands second.** BackgroundScheduler service +
    12 supervised-task migration + cadence/jitter/pause + ADR-0067
    amendment. Reduces (does not eliminate) writer contention.

R4. **Stage 3 (Class C pilot) lands third.** Quill ability migration
    establishes pattern. W4-Sub-2 follows in its own L0 + Linear ticket
    for the remaining 5 source-poller migrations.

R5. **Compatibility.** Old DB files missing `throttled_until_at`
    migrated by v179. Old config files without `background_cadences`
    use defaults. WP plugin update NOT required because A2 is a
    runtime addition — existing WP plugin versions already call
    `/v1/surface/project-composition` and now get a response instead of
    404. No breaking change at the runtime or WP boundary.

R6. **Release gate.** W4-Sub Linear ticket closes when AC1-AC15 are
    proven (proof bundle attached). PR #292 (DOS-572 W4-A) gets A1
    backport + L4 proof attached. PR #291 (DOS-589) gets the diagnostic
    `log::warn!` backport at `surface_runtime/mod.rs:796-805` as part
    of the A4 sweep.

## ADR cross-references

- **ADR-0067 (resume latency + DB concurrency guardrails)** + V2's
  Amendment 1 — names Stage 2.5 best-effort pause semantics; defers
  Stage 3 DbService priority lane.
- **ADR-0058 (proactive intelligence maintenance)** — preserved.
  BackgroundScheduler respects hygiene budget; discovers hygiene loop
  as a registered worker.
- **ADR-0102 (ability runtime + AbilityPolicy)** — preserved.
  `RecurringAbilityDescriptor` is registry-level metadata, not
  AbilityPolicy field. `Actor::RecurringSource` extends Actor enum per
  §7.1.
- **ADR-0105 (provenance envelope)** — preserved. Quill pilot carries
  `source_asof` + source attribution through `AbilityOutput<Composition>`.
- **ADR-0129 (WordPress as primary surface)** — W4-Sub unblocks W4-A
  first-block render which is the load-bearing AC for v1.4.2 surface
  foundation per ADR-0129.

## Linear / git artifacts

- Linear ticket: TBD — "W4-Sub: substrate writer-contention sweep +
  scheduler + pilot ability migration" linking to this packet.
- Parent: DOS-546 (v1.4.2 project).
- Branch: `dos-???-w4-sub`.
- Worktree: `/private/tmp/dailyos-w4-sub`.
- Migration slot reserved: v179.
- W4-Sub-2 follow-on Linear ticket: "W4-Sub-2: 5-poller abilities-runtime
  migration" — filed concurrent with W4-Sub merge.

## Open questions for L0 reviewers (V2)

V1 had 5 open questions; V2 resolves Q1 (scope), Q2 (pause cap), Q3
(cadence UI), Q5 (ADR amendment timing) explicitly. Q4 (validator
extension) retained as a sanity check.

Q1. **(RESOLVED V2)** Class B + Class C scope. V2: Class B in W4-Sub
    (full scheduler), Class C pilot only (Quill), remaining 5 pollers
    in W4-Sub-2.

Q2. **(RESOLVED V2)** Pause cap default. V2: 5 seconds, wall-clock-
    clamped max-wins semantics. Configurable via the same cadence
    config surface.

Q3. **(RESOLVED V2)** Cadence-tuning UI surface. V2: NO UI (Tauri React
    UI freeze). Config file + env override only.

Q4. **(OPEN)** Should `is_safe_ability_name` be extended further (e.g.
    allow `@` for `vendor@namespace/name` patterns) or keep minimal at
    `/`? **Recommendation**: minimal — only `/`. Other expansions wait
    until concrete need.

Q5. **(RESOLVED V2)** ADR-0067 amendment timing. V2: filed pre-L0 as
    AC14, not L3 paperwork. Required for /cso lane approval.

Q6. **(NEW)** Should W4-Sub-2 also block on a fresh L0, or can it
    inherit W4-Sub's L0 with delta-review only? **Recommendation**:
    fresh L0, because the 5-poller fan-out has its own risk surface
    (cursor preservation, oauth/credential state, per-source actor
    attribution).

## Version history

- **V1 (2026-05-16):** Initial L0 packet authored against survey
  findings from the W4-A L4 chase session. Reviewed by /plan-eng-review,
  /codex challenge, /codex consult, /cso. 18 findings drove V2.

- **V2 (2026-05-16):** Folded 18 L0 findings. Material changes:
  A2 inverted (runtime adds orchestrator route, not WP migrates to
  invoke); AC §59 + §60 correctly cited; A4 re-anchored as survey
  commitment with CSO log-hygiene CI lint; `pause_for` correctness claim
  dropped (best-effort only); Class C reduced from 6 pollers to 1 pilot
  (Quill); `recurrence` moved off AbilityPolicy to
  RecurringAbilityDescriptor; ADR-0067 amendment promoted to pre-L0
  dependency (AC14); 12 background worker count corrected
  (SurfaceRuntimeEndpoint excluded); MeetingPrepQueue/EmbeddingQueue
  pause checkpoints added explicitly (AC10); v179 migration form Rust-
  gated; all line citations re-verified; A1 leading-/double-slash
  policy named; B1 pause-budget semantics chosen (wall-clock-clamped);
  B5 cadence UI dropped; C1 cadence floor named (MIN_RECURRENCE_INTERVAL
  = 30s); C2 actor attribution named (Actor::RecurringSource).

  Pending L0 lanes for V2: re-run of /plan-eng-review, /codex challenge,
  /codex consult, /cso.
