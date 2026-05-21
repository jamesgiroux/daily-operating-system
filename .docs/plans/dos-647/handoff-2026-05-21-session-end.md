# DOS-647 + MCP V2 W2-A — Session Handoff (2026-05-21)

**Author:** Claude Opus 4.7 (1M context)
**Worktree:** `/private/tmp/dailyos-v147-w1a`
**Branch:** `v1.4.7-w1-foundation`
**Last commits:** `9e5009d7` (W2-A wiring), `10aecb28` (request-scoped dispatch + live readers + version pre-read)

---

## Outcome of this session

Started: wire MCP V2 W2-A so Claude Desktop could compare DailyOS substrate to Glean side-by-side.

Pivoted to: pause MCP V2 demo work and tackle DOS-647 first — the underlying DB concurrency substrate that's the real blocker for ANY surface client (WP, MCP, future) on James's production DB.

Reason for pivot: the W2-A wiring is correct (proven by a successful live `bring-a-trailer` Composition response — v5, 36 claims). What blocked the demo was DB-level lock contention between the MCP V2 binary's `LocalKeychain.get_or_create_key` (which runs SQLCipher key verification by opening a fresh Connection and querying sqlite_master) and the running Tauri app's WAL writes. Same root as DOS-647, different surface.

---

## Track A (DB pool maturity, intra-process) — scope agreed

DOS-647 as originally filed. The v1.4.4 W1 work added `src-tauri/src/db_service.rs` (1 writer + 2 readers, dedicated threads, FIFO closure dispatch) plus `db_read` / `db_write` helpers on `AppState`. This fixed the Arc<Mutex<ActionDb>> contention but stopped short of:

1. **Typed `RetryableError`.** `surface_runtime/mod.rs:1226` collapses every `db_write` error including `SQLITE_BUSY` to `SurfacePairingError::Write`. The block silently renders ConsistencyFindingBanner; operator can't distinguish "DB is busy, retry in 200ms" from "DB is genuinely broken."
2. **No retry-with-backoff.** The 5s `PRAGMA busy_timeout` is the only retry; after exhaustion → hard fail. No exponential backoff, no jitter.
3. **No priority dispatch.** Signed-route writes and background-job writes (intel_queue, embeddings, claim_invalidation, entity_linking, startup sync) all FIFO through the same writer thread. A startup-sync burst starves a foreground signed route for the full 5s.

Track A scope:
- A1: Typed `RetryableError` for `SQLITE_BUSY` in `db_write` helper, exposed up to HTTP 503 + `Retry-After` from signed-route endpoints
- A2: Priority lanes on the writer thread (foreground signed-routes > background jobs) — adapt the existing `mpsc::channel` to a priority structure
- A3: Retry-with-backoff at the `db_write` helper level (exponential + jitter, bounded retry count)
- A4: Audit background-job transaction scopes per DOS-647 fix sketch §3 — some appear to hold long transactions across foreign DB calls (entity_linking `find_or_create_person` in a loop with CHECK constraint failures suggests retry-without-backoff inside a tx); shorten lock windows

---

## Track B (MCP V2 surface-client unification, cross-process) — scope agreed

User-confirmed scope: **MCP V2 (and any future binary surface) routes through the Tauri app's HTTP signed-route server instead of opening its own SQLCipher Connection.** Covers reads AND writes uniformly per the catalog (10 tools: account_status, daily_briefing, meeting_briefing, portfolio_attention, workspace_memory, workspace_source_provenance, place_document, submit.{note,action,action_status}) plus claim feedback per ADR-0123 / DOS-683.

Architecture:
- Tauri app's `surface_runtime` already exposes `/v1/surface/invoke`, `/v1/surface/project-composition`, `/v1/surface/nonce/{issue,verify}`, `/v1/surface/pairing/{handshake,refresh-scopes}`. The WP transport at `wp/dailyos/includes/transport/class-dailyos-runtime-client.php` is the precedent — generic signed-route client.
- W1-A gateway code (auth, audit, dispatch — currently in `src-tauri/src/services/mcp_v2/{gateway,auth,audit,actor_policy,contracts}.rs`) **moves into the Tauri app's `surface_runtime`** alongside existing routes. Probably under new `/v1/mcp/*` paths (invoke, list, pairing) or absorbed into the existing surface routes with an MCP actor tag.
- W2-A handler code (`tool_account_status.rs`, `tool_briefing.rs`, etc.) keeps its `McpToolHandler` trait + `invoke_registry_json_for_actor` dispatch — just runs in the Tauri process. Mostly a relocation, not a rewrite.
- `dailyos-mcp-v2` binary shrinks to a stdin/stdout JSON-RPC ↔ HTTP signed-route adapter (~200 lines). No DB access, no SQLCipher key opens, no migration concerns. Pairing flow becomes a thin wrapper over the existing `/v1/pairing/handshake`.

Cross-process race vanishes by construction: only one process owns the SQLCipher Connection.

---

## What v1.4.4 already shipped (substrate audit)

Read this section before any L0 work — it's the ground truth so reviewers don't get re-traversed:

- **`src-tauri/src/db_service.rs`** (282 lines, new in v1.4.4 W1): 1 writer + 2 readers pool. `NUM_READERS = 2` (line 38). Writer + readers spawned on dedicated OS threads. `mpsc::channel` for closure dispatch. `PRAGMA busy_timeout = 5000` on every pool connection (line 208).
- **`src-tauri/src/state.rs:1453-1533`**: `db_read` / `db_write` async helpers. Route closures through the pool. On-demand init at line 1461. Fallback to fresh `ActionDb::open()` if pool isn't installed (line 1488).
- **`ActionDb::from_conn(&Connection) -> &Self`** (`db/core.rs:86`): repr(transparent) zero-cost view borrow.
- **Cross-process today**: No advisory locks, no flock, no single-writer election. Each process (Tauri app, MCP V1 binary, MCP V2 binary) opens its own SQLCipher Connection. SQLite WAL is the only coordinator.
- **SQLITE_BUSY handling**: Only `PRAGMA busy_timeout = 5000` + a classification at `services/derived_state.rs:486` (`TargetTableLocked`). No retry loops, no typed errors, no backoff.

---

## What's committed and where it lives if Track B reshapes it

Two commits on `v1.4.7-w1-foundation` (current branch HEAD):

1. **`9e5009d7`** — `feat(mcp_v2): W2-A Phase-A — wire account_status MCP tool to abilities-runtime`
   - account_overview ability: added McpClient to allowed_actors
   - `services/mcp_v2/handlers/tool_account_status.rs` (handler impl)
   - `services/mcp_v2/handlers/registration.rs` (wave-scoped registration helper)
   - `services/mcp_v2/transport.rs` (spawn_blocking wrap of gateway.dispatch)
   - L0 packet at `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md`
   - 6 unit tests + 1 integration smoke

2. **`10aecb28`** — `fix(mcp_v2): request-scoped dispatch + live readers + version pre-read`
   - Switched handler to `invoke_registry_json_for_actor` so `Actor::McpClient` flows through (vs prior `BridgeActor::Agent` which mapped to `Actor::Agent` and was rejected by `account_overview.allowed_actors`)
   - `attach_live_workspace_readers` instead of narrow `McpWorkspaceReaders.attach_to` (gets the composition_commit handle that account_overview needs)
   - Pre-read current composition version via `current_composition_version_for_composition_id` before invoking — mirrors `surface_runtime/mod.rs:2545` pattern; avoids StaleComposition error against non-fresh compositions
   - Removed migration run from binary's `open_conn` — Tauri app owns migration lifecycle; concurrent run from binary was colliding with app WAL writes
   - Precondition check that W1-A schema is present (clear error if not)

**Under Track B these files relocate but don't get rewritten:**
- `tool_account_status.rs` → moves into the Tauri app's surface_runtime module
- `gateway.rs`, `auth.rs`, `audit.rs`, `actor_policy.rs`, `contracts.rs` → move into the Tauri app, hosted by surface_runtime
- `transport.rs` → mostly gone (transport becomes HTTP, not stdio)
- `mcp_v2/main.rs` → shrinks to ~200 lines: stdin/stdout JSON-RPC ↔ HTTP adapter

The substrate decisions (HMAC envelope, nonce ledger, conversation_handle minting, scope manifest) all stay valid. The hosting model is what shifts.

---

## Open questions for next-session L0 draft

1. **MCP routes under existing `/v1/surface/*` or new `/v1/mcp/*` namespace?** The WP block already uses `/v1/surface/invoke`. Reusing it for MCP keeps one route surface; segregating clarifies audit + rate-limit tracking. Lean toward `/v1/mcp/*` for clear actor-class differentiation.

2. **Track A and Track B independence — confirm landability.** Track A is incremental on `db_service.rs`. Track B is a larger restructure but doesn't depend on A semantically (Track B benefits from A but works without it; it just experiences the same pool-starvation issue as the WP surface today). Landable independently in either order.

3. **Migration of W1-A schema into Track B model.** `mcp_client_manifest` + `mcp_tool_grant` + `mcp_conversation_handle` + `mcp_transport_nonce_ledger` were authored for an in-binary gateway. They remain useful for the in-process gateway too — just consulted by surface_runtime instead of the standalone binary. No schema migration needed.

4. **Does Track B obsolete `dos-175-l0-plan.md`?** The L0 packet for W2-A targets the standalone-binary architecture. After Track B lands, it's a different gateway location. Either: (a) supersede dos-175 with a dos-175-cycle-3 update that reflects post-Track-B layout, (b) close dos-175 as Phase-A complete and file a new ticket for the Track B migration, or (c) merge dos-175 into the DOS-647 packet since the W2-A delivery is no longer wave-scoped. Lean toward (b).

5. **Track B impacts the WP surface how?** WP doesn't change — it's already an HTTP signed-route client. But Track B adds new MCP-flavored routes, and audit log fields will need a new actor_kind tag (`mcp_client_via_http` or similar) to distinguish from `surface_client`. Minor schema review.

---

## Next-session start

1. Read this handoff
2. Read DOS-647 in full via `mcp__plugin_linear_linear__get_issue DOS-647`
3. Re-read ADR-0067 (`.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md`) — the deferred-clause language
4. Skim ADR-0111 (`.docs/decisions/0111-surface-independent-ability-invocation.md`) — surface-independence contract
5. Draft `.docs/plans/dos-647/dos-647-l0-plan.md` with:
   - §0 metadata + threat topology (local-to-local single-user)
   - §1 substrate context (lift from the audit section of this handoff)
   - §2 Track A scope + AC (5-7 ACs)
   - §3 Track B scope + AC (5-7 ACs)
   - §4 test plan per track
   - §5 explicit out-of-scope
   - §6 open questions (the 5 above)
   - §7 reviewer dispatch (codex challenge + architect + devex + consult; K-in mandatory)
   - §8 definition of done per track
6. Dispatch L0 reviewers in parallel, fold convergent findings, iterate to unanimous APPROVE

---

## Critical context for next agent

- **Do NOT pair Claude Desktop against the V2 binary.** Config was reverted to V1 at end of session. The backup at `~/Library/Application Support/Claude/claude_desktop_config.json.bak-w2a-20260521-143224` has the V2 entry if needed for testing, but the V1 binary is what's wired now.
- **DB is fine.** The "orphan index" / "malformed schema" errors that appeared during the demo were transient lock contention between the MCP V2 binary and the running Tauri app — NOT actual schema corruption. The Tauri app continues to work normally against the same DB.
- **W2-A code is solid.** The two commits are clean tests, lint, and ran successfully against real substrate. Don't rewrite — relocate.
- **Memory citations**: `feedback_local_to_local_security_overreach_primary_concern`, `feedback_wire_existing_substrate_not_future_producer`, `feedback_check_substrate_before_authoring_primitives`, `feedback_review_loop_diminishing_returns_means_scope_is_wrong` all applied to this session and shaped the W2-A packet — relevant for DOS-647 too.
- **`feedback_l0_partial_convergence_when_class_recurs`** applied at session end of W2-A L0 — cycle-1 reviewer findings were folded directly without a cycle-2 panel because the findings were source-verified substrate facts. If DOS-647 reviewer feedback follows the same pattern, same play applies.
