# DOS-647 L0 Packet — Single-Process DB Ownership + Writer Priority Lane

> **STATUS: V1 BLOCKED 2026-05-21 — split into 4 sub-tickets.**
>
> Codex challenge identified 4 P1 substrate facts that invalidate the packet's core premises ("relocation, not rewrite" is false — handlers self-open `ActionDb` at `tool_account_status.rs:122`; `db_read`/`db_write` stringify errors at `state.rs:1453`, breaking the Track 2 typed-retry chain; §5 has at least 2 silent dependencies). Architecture / reliability / security reviewers approved with 16 combined findings, all AC sharpening — but codex's P1s required interface changes that materially expanded scope. Per James (2026-05-21), this is the W4-Sub V2 trajectory and we don't repeat it.
>
> **Split:**
>
> - **DOS-647-A** — preserve typed DB errors end-to-end through `db_read`/`db_write` (substrate prerequisite, no behavior change visible). Lands first. No heavy L0.
> - **DOS-647-B** — `McpToolHandler::invoke` interface refactor: request-scoped DB/services/readers in, forbid nested writer calls. Unblocks in-process MCP hosting. Lands second. No heavy L0.
> - **DOS-647-C** — `/v1/mcp/*` signed routes + writer priority lane + 14-site (+ extension per codex P2-7) log sweep + background-worker priority annotation. Full L0 reviewer matrix once A+B land.
> - **DOS-647-D** — investigation: why is the Tauri React UI hitting "pairing_authority_unavailable: database is locked" right now? Non-blocking; diagnosis informs whether C scope is correct or needs adjustment.
>
> Parent DOS-647 becomes umbrella. This V1 packet preserved as the substrate-audit record + reviewer-findings trail; do NOT implement as written.

**Status:** L0 V1 — BLOCKED (see header)
**Author:** Claude Opus 4.7 (1M context) — 2026-05-21
**Linear:** [DOS-647](https://linear.app/a8c/issue/DOS-647) (moved to v1.4.7 — MCP Server v2; priority Urgent)
**Branch:** `v1.4.7-w1-foundation` at `/private/tmp/dailyos-v147-w1a`
**Predecessors:** [DOS-655 W4-F](https://linear.app/a8c/issue/DOS-655) (SHIPPED 2026-05-17, zero-write read path); [DOS-653 W4-Sub](https://linear.app/a8c/issue/DOS-653) (CANCELED 2026-05-20, wide-scope swing)

---

## §0 Metadata + Threat Topology

**Surface impact:**
- MCP V2 (Claude Desktop pairing demo) is reproducibly broken against James's prod DB due to cross-process SQLCipher Connection race.
- WP signed-route WRITE paths (claim feedback, action submit, future surface writes) hit the same starvation pattern as DOS-647's original read-path symptom — DOS-655 fixed reads, writes remain.
- Tauri app perceived slowness over recent weeks attributed to FIFO writer-mutex contention between background workers (intel_queue, embeddings, claim_invalidation, entity_linking, source pollers) and foreground writes.

**Threat topology — local-to-local single-user.** Per ADR-0129, ADR-0111 §8, and memory `feedback_local_to_local_security_overreach_primary_concern`:

- The Tauri app, MCP V2 binary, and WP plugin are all local processes running as the same OS user on the same machine.
- The SQLCipher DB lives at `~/.dailyos/dailyos.db`. OS-level filesystem ACLs already gate any cross-user access.
- The MCP V2 binary is launched as a subprocess by Claude Desktop, running under the user's OS account. It is a `SurfaceClient` instance per ADR-0111 §8 (per-instance identity = host MCP client = Claude Desktop / Cursor / Codex / etc.).
- Trust shape preserved from ADR-0111 §8: every operation logs SurfaceClient instance identity; writes carry user-presence nonces (per DOS-655 + ADR-0123); scope grants gated at pairing time.
- **Do NOT introduce additional inter-process attestation, signed envelopes, or principal-authentication layers beyond what already exists for WP.** WP is the precedent and it is sufficient. The cross-process boundary already crossed (OS process boundary, HMAC-signed HTTP) is the trust surface; adding more does not increase safety, only friction.

**In scope for this L0:** writer-dispatch priority lane, typed `RetryableError`, log-line at Write Err arms, MCP V2 → HTTP signed-route adapter relocation.

**Out of scope** (carry-forward from DOS-653 cancellation, each separately ticketed if pursued):
- BackgroundScheduler abstraction / cadence overhaul
- Source-poller migration to abilities-runtime
- `composition_version` churn
- Recurring-actor authorization path
- ADR-0067 Stage 3 (split-pool / multi-writer)
- 12-task background-worker fan-out audit
- `commit_composition` cache-eviction churn
- ADR-0067 amendment 1 (Stage 2.5 vs Stage 3 naming)

---

## §1 Substrate Audit (Ground Truth)

K-in grep was substrate-type, not proposed-name, per `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`. Queries: `writer_tx`, `NUM_READERS`, `busy_timeout`, `SQLITE_BUSY`, `TargetTableLocked`, `RetryableError`, `validate_signed_session`, `Connection::open`, `/v1/surface/`, `McpClient`.

### What v1.4.4 W1 shipped (`db_service.rs`)

- **1 writer + 2 readers**, each on a dedicated OS thread. `NUM_READERS = 2` at `db_service.rs:38`.
- Closure dispatch via `mpsc::channel` — both async (`fn call` → `oneshot::Sender`) and sync (`fn call_sync` → `mpsc::Sender`). Single FIFO queue per connection.
- `PRAGMA busy_timeout = 5000` on every pool connection (line 208). `PRAGMA journal_mode = WAL`, `synchronous = NORMAL`, `query_only = ON` for readers.
- `open_fresh_serialized` serializes fresh `Connection::open()` calls through the writer thread to avoid SQLCipher WAL-read-verify races (the `SQLITE_NOTADB` failure mode tested at `dos_229_sqlcipher_open_fresh_serialized_no_notadb`).
- Process-wide `GLOBAL` singleton — `try_global()`, `install_global()`, `uninstall_global()`. `ActionDb::open()` routes through the pool when installed; falls back to fresh-open path otherwise.
- Key rotation atomic: `rekey_database` shuts down pool, rekeys, reopens. Tested at `key_rotation_reopens_active_db_service_pool` + `action_db_open_key_fetch_and_fresh_open_are_rotation_atomic`.

### What DOS-655 W4-F shipped (`services/surface_pairing.rs`)

- `fn validate_signed_session_readonly` (line 901) — zero-write validation. Returns 6 no-write `SignedSessionFailure` variants. Only consults `absolute_expires_at` (V3.1 §7 #7); `inactive_expires_at` retained forensically only.
- Session lifecycle moved from inactivity-expiry to absolute-lifetime. `last_seen_at` / `last_used_at` lazy-flushed on graceful shutdown only.
- Sentinel file at `~/.dailyos/runtime-endpoint.json` for port discovery; keychain persistence rehydrates HMAC session keys on Tauri restart.
- CI gate forbids DB writes in `validate_signed_session` and GET-shape `_response` handlers (Layer 1 runtime counter live; Layer 2 AST deferred to DOS-664).

### What `surface_runtime/mod.rs` already exposes (Track 1 target)

Full signed-route HTTP server. Routes:
- `GET /v1/surface/health` (line 1088)
- `POST /v1/surface/session/refresh` (1092)
- `POST /v1/surface/invoke` (2075)
- `GET /v1/surface/abilities` (2017)
- `GET /v1/surface/keyring` (2000)
- `POST /v1/surface/project-composition` (2057)
- `POST /v1/surface/subscribe` (2066), `POST /v1/surface/replay` (2069)
- `POST /v1/surface/nonce/issue` (2035), `POST /v1/surface/nonce/verify` (2046)
- `POST /v1/surface/pairing/refresh-scopes` (2072)
- `GET /v1/surface/event-log/{cursor}` (1240)
- Pairing handshake routes — `POST /v1/surface/pairing/handshake` (verified separately)

Actor classes already routed:
- `Actor::SurfaceClient { scopes, .. }` (line 1946) — full WP flow
- `Actor::McpClient { .. }` (line 1954) — already wired through `invoke_registry_json_for_actor`; W2-A commit `10aecb28` proved it returns successful Composition responses

### What MCP V2 binary opens today (Track 1 source of pain)

- `src-tauri/src/mcp_v2/main.rs:128 fn open_conn` — calls `Connection::open(db_path)` directly, bypassing `DbService`.
- Called at line 201, 336, 420 — three independent open sites in the binary.
- Line 136: `Arc::new(dailyos_lib::db::LocalKeychain::new())` — fresh keychain handle in binary process, separate from app's keychain handle.
- Result: 4+ SQLCipher Connections (1 binary writer + 2 app readers + 1 app writer + N fresh-opens from `open_fresh_serialized` callers) racing on the same WAL stream.
- SQLCipher per-connection key-verification (`SELECT count(*) FROM sqlite_master LIMIT 1`) is the contention point — it runs at every Connection open and must see a consistent WAL snapshot.

### Existing SQLITE_BUSY classification precedent

`src-tauri/src/services/derived_state.rs:485-487`:

```rust
rusqlite::Error::SqliteFailure(ref sqlite_error, _) => match sqlite_error.code {
    rusqlite::ErrorCode::DatabaseBusy
    | rusqlite::ErrorCode::DatabaseLocked
    => { /* classified as TargetTableLocked */ }
```

Track 2 reuses this exact classification at the `db_write` helper level — no new code style.

### 14 sites mapping `SurfacePairingError::Write` without log line

`grep -n "SurfacePairingError::Write(error)" src-tauri/src/surface_runtime/mod.rs` returns 14 sites at lines 1226, 1332, 1393, 1446, 1535, 1850, 1938, 2242, 2297, 2343, 2431, 2832, 2962, 3283. Class-wide log sweep, not per-site patch (memory: `feedback_systemic_look_for_recurring_issue_classes`).

### Reference: WP plugin transport (Track 1 model)

`wp/dailyos/includes/transport/class-dailyos-runtime-client.php` — generic signed-route client. HMAC envelope, nonce flow, scope-refresh, error mapping. Track 1's MCP adapter is the same shape over stdio JSON-RPC instead of WP HTTP.

---

## §2 Track 1 — Single-Process DB Ownership (MCP V2 → HTTP signed-route adapter)

### Scope

**Relocation, not rewrite.** The W2-A handler code already authored in commits `9e5009d7` + `10aecb28` is sound. It moves from `src-tauri/src/services/mcp_v2/` to `src-tauri/src/surface_runtime/` (or hosted by `surface_runtime` from `services/mcp_v2/`). The MCP V2 binary shrinks from "DB-opening process with handlers" to "stdio JSON-RPC ↔ HTTP adapter."

### Architecture

- **Inside Tauri app** (`surface_runtime` module):
  - W2-A handlers (`tool_account_status.rs`, future `tool_briefing.rs`, etc.) registered against `McpClient` actor flow
  - `gateway::dispatch` (currently `services/mcp_v2/gateway.rs:271`) becomes an in-process dispatcher
  - New route family: `POST /v1/mcp/invoke`, `GET /v1/mcp/list-tools` (namespace-segregated from `/v1/surface/*` for clear actor-class differentiation, per handoff Open Question 1)
  - Pairing for MCP clients reuses `/v1/surface/pairing/handshake` with `McpClient` actor type in the request
  - Audit log already supports `McpClient` actor; no schema change

- **In MCP V2 binary** (`src-tauri/src/mcp_v2/main.rs`):
  - Remove all `Connection::open` calls (lines 201, 336, 420 + `fn open_conn`)
  - Remove `LocalKeychain::new()` (line 136)
  - Remove migration pre-checks (already removed in commit `10aecb28`)
  - Add: stdio JSON-RPC reader → HTTP signed-route client (model: WP transport)
  - Read sentinel `~/.dailyos/runtime-endpoint.json` for port discovery (same as WP)
  - HMAC-signed POSTs to `/v1/mcp/invoke` with pairing-issued session key from OS keychain
  - Estimated final size: ~200-300 LOC

### Acceptance Criteria

**T1-AC1.** MCP V2 binary contains zero `rusqlite::Connection::open`, zero `LocalKeychain`, zero migration calls. CI gate (extend Layer 1 runtime counter from W4-F) blocks regression.

**T1-AC2.** `services/mcp_v2/gateway.rs`, `actor_policy.rs`, `audit.rs`, `contracts.rs`, `handlers/tool_account_status.rs`, `handlers/registration.rs`, `transport.rs` relocated under `surface_runtime/mcp/` (or composed in via re-export). Behavior parity tests carried over: 6 unit tests + 1 integration smoke from W2-A all green at the new location.

**T1-AC3.** New HTTP routes `POST /v1/mcp/invoke` + `GET /v1/mcp/list-tools` live in `surface_runtime/mod.rs`. Both require valid pairing + scope check + actor=McpClient. Routes added to the signed-route allowlist alongside existing `/v1/surface/*` entries.

**T1-AC4.** MCP V2 binary stdio adapter routes `account_status` invocations to `/v1/mcp/invoke`, parses the signed response, returns JSON-RPC reply. Live test: pair Claude Desktop against the binary against prod DB; `account_status` for `bring-a-trailer` returns the same Composition (v5, 36 claims) the prior W2-A test produced.

**T1-AC5.** Concurrent-load smoke: app running with prod DB + Tauri React UI doing reads + WP block render + MCP V2 binary doing 50 sequential `account_status` invocations → zero `SQLITE_BUSY`, zero `database is locked`, zero `not a database`, zero "malformed schema" errors. Demonstrated against the same fixture James reproduced the pairing failure with.

**T1-AC6.** WP plugin signed-route flow regression-test green (no behavior change to WP — verifies the namespace split didn't break the WP transport).

**T1-AC7.** Audit log entries for MCP V2 invocations show `actor_kind = McpClient` with instance identity. ADR-0111 §8 trust shape preserved (instance ID logged, scope grants enforced, pairing token verified, no presence-nonce required for reads, presence-nonce required for any future writes — out of scope here, just contract).

### Out of scope for Track 1

- Migrating remaining W2-A tools (briefing, portfolio_attention, etc.) — they get authored fresh against the new in-process model in subsequent tickets
- MCP V1 binary deprecation timeline — V1 still works, sunset later
- Per-tool rate limiting beyond what gateway already does
- Tool discovery filtering beyond what `Actor::McpClient` already enforces

---

## §3 Track 2 — Writer-Mutex Priority Lane (intra-process)

### Scope

Three changes to `db_service.rs` + one cross-cutting log sweep:

1. Replace `mpsc::channel` with a 2-priority dispatch (`Priority::Foreground` vs `Priority::Background`). Foreground writes drain before background writes when both are queued.
2. Add typed `RetryableError` to `PooledCallError` enum. Classify `rusqlite::ErrorCode::DatabaseBusy | DatabaseLocked` as `Retryable` (precedent: `derived_state.rs:485-487`).
3. Add bounded retry-with-backoff helper at the `db_write` / `db_read` call sites in `state.rs` (currently lines 1453-1533). Exponential backoff with jitter, max 3 retries, total budget 1.5s (under the 5s `busy_timeout` floor).
4. Sweep the 14 `SurfacePairingError::Write(error)` sites in `surface_runtime/mod.rs` — add `log::warn!` with the underlying error before mapping. Single class-wide pass, not per-site.

### Acceptance Criteria

**T2-AC1.** `db_service.rs` writer dispatch routes `Priority::Foreground` calls ahead of `Priority::Background` in the worker thread's receive loop. Implementation: replace single `mpsc::channel` with two channels + `select!` favoring foreground, OR a `crossbeam_channel::select_biased!` equivalent (decision in §6 below).

**T2-AC2.** Call sites annotated with priority. Foreground = signed-route handler write paths + Tauri-app user-initiated commands + MCP V2 invocations (via Track 1 routing). Background = intel_queue, embeddings, claim_invalidation, entity_linking, source pollers, startup sync. Audit list compiled at L1 and reviewed against `state.rs` + service callers.

**T2-AC3.** `PooledCallError::Retryable(rusqlite::Error)` variant added. `db_service.rs::run_task` classifies `SqliteFailure` with code `DatabaseBusy | DatabaseLocked` as `Retryable`; other rusqlite errors remain `Rusqlite`.

**T2-AC4.** `state.rs::db_write` and `state.rs::db_read` helpers retry `Retryable` errors with exponential backoff (50ms / 150ms / 450ms) + ±20% jitter. After 3 retries, surface as `Retryable` to the caller (not collapsed to generic error).

**T2-AC5.** Signed-route endpoints in `surface_runtime/mod.rs` map `Retryable` to HTTP 503 + `Retry-After: 1` header. WP transport + future MCP adapter render a "busy, retry" affordance distinct from the ConsistencyFindingBanner.

**T2-AC6.** 14 `SurfacePairingError::Write` sites in `surface_runtime/mod.rs` emit `log::warn!("signed-route write failed at {site}: {underlying}", site = "<route name>", underlying = error)` BEFORE mapping. Sweep is a single PR section, not 14 commits.

**T2-AC7.** Concurrent fairness fixture: spawn 4 background writer tasks doing 100 writes each at 250ms cadence; while running, dispatch 20 foreground signed-route writes; p95 foreground latency ≤ 200ms. (Without priority lane, p95 is currently unbounded — starvation under heavy background churn.)

**T2-AC8.** Unit test for `Retryable` classification + retry-with-backoff: inject a `SqliteFailure(DatabaseBusy)` into a fake closure; assert 3 retries with backoff bounds, then propagated as `Retryable`. Inject `SqliteFailure(ConstraintViolation)`; assert no retry, propagated as `Rusqlite` immediately.

### Out of scope for Track 2 (separately ticketed)

- Background-job transaction-scope audit (DOS-653 finding #2 — separately scoped)
- BackgroundScheduler abstraction (DOS-653 finding #3)
- Source-poller cadence overhaul / jitter / pause API (DOS-653 finding #2)
- `commit_composition` version churn (DOS-653 finding #7)
- Split-pool / multi-writer (ADR-0067 Stage 3 — separately scoped after this lands and we measure)

---

## §4 Test Plan

### Track 1 tests

- **Unit (`services/mcp_v2/handlers/` relocated)** — existing 6 unit tests for `tool_account_status` carry over to new location, no logic change.
- **Integration** — `tests/it/mcp_v2_via_signed_routes.rs` (new): boot Tauri app with test DB, start MCP V2 binary as subprocess, drive 5 `account_status` invocations through stdio JSON-RPC, assert: zero direct `Connection::open` in binary (process-level check), all 5 invocations succeed, audit log has 5 `actor_kind=McpClient` entries.
- **Live smoke** (L4) — James pairs Claude Desktop against the new binary against prod DB; runs `account_status bring-a-trailer`; verifies same Composition v5, 36 claims, zero collisions visible in Tauri app log. Same fixture as W2-A live test.
- **CI gate** — extend W4-F Layer 1 runtime counter to assert MCP V2 binary's process makes zero `sqlite3_open*` syscalls (linux: `strace -e openat`; macOS: `dtrace` or process-level mock).

### Track 2 tests

- **Unit (`db_service.rs`)** — `retryable_classification`, `foreground_priority_drains_first`, `backoff_bounds_within_budget`, `non_retryable_passes_through`.
- **Fairness fixture (`tests/it/writer_priority_fairness.rs`)** — described in T2-AC7. Must be deterministic enough to not flake; use bounded background workload + measured spinup, not wall-clock-bound.
- **Integration (`surface_runtime`)** — `signed_route_503_on_busy`: inject `DatabaseBusy` into `db_write`; assert HTTP 503 + `Retry-After: 1` response.
- **L4 manual** — observe Tauri app perceived responsiveness improvement under James's normal workload over 24h (qualitative — captured in retro, not gating).

### Cross-track regression

- All existing 2460 lib tests + DOS-655 W4-F integration tests + WP block render path remain green.
- `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` per CLAUDE.md DoD.

---

## §5 Explicit Out-of-Scope

**Reiterated from §0 — separately ticketed if pursued.** These are real substrate work that James's "not actionable as a single wave" decision (2026-05-16) gates. Folding them in here re-runs the W4-Sub V2 trajectory (BLOCKED twice, canceled).

1. BackgroundScheduler abstraction / cadence overhaul (DOS-653 finding #3)
2. Source-poller migration to abilities-runtime (DOS-653 finding #4)
3. `composition_version` churn fixes (DOS-653 finding #7)
4. Recurring-actor authorization path (DOS-653 finding #5)
5. ADR-0067 Stage 3 (split-pool / queue-of-queues — re-evaluate post-this-ticket)
6. 12-task background-worker fan-out audit (DOS-653 finding #2)
7. `commit_composition` cache-eviction churn (DOS-653 finding #7)
8. `is_safe_ability_name` structural validation (DOS-653 finding #8)
9. WP plugin `/v1/surface/project-composition` orchestrator-route work (DOS-653 finding #9 — separate ticket)
10. ADR-0067 amendment 1 (Stage 2.5 vs Stage 3 naming — separate doc ticket)

Any of these surfacing in L0 review = "valid concern, file separate ticket, do not fold." Memory: `feedback_dont_swing_past_center_when_correcting`, `feedback_review_loop_diminishing_returns_means_scope_is_wrong`.

---

## §6 Open Questions

1. **`/v1/mcp/*` namespace or fold into `/v1/surface/invoke`?** Leaning new namespace for actor-class differentiation in audit logs + future rate-limit budgets. Adding `/v1/mcp/invoke` is one route registration in `surface_runtime/mod.rs`. Reviewer call: namespace-segregated wins unless there's a strong audit-collation argument for fold-in.

2. **Priority dispatch implementation: two `mpsc` channels + custom select, or `crossbeam_channel::select_biased!`?** `crossbeam` is already in the workspace (need to verify). If yes, `select_biased!` is the smallest change. If no, two `mpsc` channels + a thread-local select loop (drain foreground; if empty, take from background) is std-only.

3. **Migration v180+ slot collision risk.** Track 2 does not add a migration. Track 1 does not add a migration (audit fields already exist). Verify against PR queue before L1 implementation (memory: `project_v14x_renumber_2026_05_17` parallel-wave migration slot reservations).

4. **W2-A commits supersession.** Commits `9e5009d7` + `10aecb28` are sound as foundation. They land before this packet's L1 work and get refactored in-place during Track 1 relocation (not reverted). The L0 packet for W2-A (`dos-175-l0-plan.md`) closes as Phase-A complete; Track 1 becomes its Phase-B continuation.

5. **Track A / Track B independence.** Track 1 is landable without Track 2 (just changes which process owns the Connection; doesn't change dispatch fairness). Track 2 is landable without Track 1 (improves foreground latency for both WP and the existing MCP V1 + V2 paths). Recommend land Track 1 FIRST — it eliminates the cross-process race that's currently the most visible symptom; Track 2 then quiets the remaining intra-process contention.

---

## §7 Reviewer Dispatch Matrix

Per the engineering ladder L0 reviewer rules + memory `feedback_l0_partial_convergence_when_class_recurs`:

| Reviewer | Subagent | Lens | Rationale |
| -------- | -------- | ---- | --------- |
| Architecture | `ce-architecture-strategist` | Substrate fit, pattern compliance | Track 1 is an architectural relocation; verify against ADR-0111 §8 + ADR-0129 |
| Codex challenge | `codex challenge` (via `/codex` skill) | Adversarial — try to break the plan | "What does the W4-Sub V2 reviewer cohort find here that I missed?" |
| Codex consult | `codex consult` | Independent design opinion on priority-lane dispatch + retry semantics | Track 2 has subtle concurrency surface (drain ordering, backoff jitter, retry budget interaction with `busy_timeout`) |
| Plan-eng | `plan-eng-review` (gstack) | Execution feasibility, AC tightness | Validate the AC numbers (p95 ≤ 200ms, 3-retry budget, 1.5s total) are measurable + non-flaky |
| CSO | `cso` | Local-to-local trust boundary | Track 1 expands a network surface (one new HTTP route family); confirm SurfaceClient trust shape preserved + no new exfiltration path |

**Dispatch in parallel.** Fold convergent findings on cycle 1. If cycle 2 surfaces ≥3 net-new findings from the same reviewer, scope drift — STOP, re-frame per James, do not iterate further.

K-in obligation: every reviewer asserts they grepped substrate-type before scoring (per `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`).

---

## §8 Definition of Done

### Track 1 DoD

- All Track 1 ACs (T1-AC1 through T1-AC7) green
- MCP V2 binary builds, runs, pairs against Claude Desktop, serves `account_status` against prod DB — zero SQLCipher errors in app log during sustained invocation
- L4 hands-on by James: pair Claude Desktop, run 10 invocations, confirm parity with prior W2-A successful test
- Audit log inspection: 10 `actor_kind=McpClient` entries with consistent instance identity
- `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green
- ADR drafted for the in-process MCP hosting model (new ADR or amendment to ADR-0027 / ADR-0111)

### Track 2 DoD

- All Track 2 ACs (T2-AC1 through T2-AC8) green
- Fairness fixture green deterministically in CI 10× consecutive runs (no flake)
- 14-site log-sweep PR section reviewed for missed sites (grep on `SurfacePairingError::Write(error)` post-merge returns the same 14 lines, all preceded by log calls)
- ADR-0067 amendment drafted naming Stage 2.5 (priority-lane + retry-with-backoff) as the landing point this ticket reaches — Stage 3 (split-pool) re-evaluated post-merge after measurement

### Joint DoD

- L2 reviewers (correctness + concurrency + reliability + project-standards) all APPROVE bounded by these ACs
- Retro filed including a K-out entry on the "path-α as sneaky deferral" pattern (already captured in memory `feedback_path_alpha_as_sneaky_deferral`)
- DOS-647 closed; carry-forward tickets filed for any §5 items James wants to keep alive
