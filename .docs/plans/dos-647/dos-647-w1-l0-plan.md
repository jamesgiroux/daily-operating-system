# DOS-758 L0 Packet (V2, Track-B-only) — Request-Scoped MCP Handler Context (Single Owned Connection)

**Version:** v1.4.9 · W1a (DB ownership)
**Issue:** [DOS-758](https://linear.app/a8c/issue/DOS-758) (647-B). Parent umbrella [DOS-647](https://linear.app/a8c/issue/DOS-647). 647-A (DOS-757) shipped; 647-D (DOS-760) canceled.
**Author date:** 2026-05-31
**Tier:** Tier 3 (markdown-only). **Scope tier:** **Standard** (codex challenge + 1 planning reviewer + K-in).
**Supersedes:** the V1 647 packet (`c288d1f6:.docs/plans/dos-647/dos-647-l0-plan.md`, BLOCKED 2026-05-21).
**Re-scope note (L0 cycle-1 → cycle-2):** the V2-draft's Track C ("route MCP DB access through the app single writer") was returned **wrong-cure** by codex + ce-feasibility — MCP V2 is an *out-of-process sidecar* with no in-process `DbService`, so single-process-writer ownership requires the IPC/transport layer that **W2 (DOS-833)** owns. Track C is removed from this packet and handed to a W2-coupled follow-up (see §4). This packet now carries **only Track B**, the in-MCP-process work all three reviewers found independently landable.

---

## §0 Origination + Scope + Topology

- **Origination class:** Debug-driven. Root incident: the 2026-05-28 production DB-loss + the recurring `database is locked` / `pairing_authority_unavailable: database is locked` lock-storm class (documented: `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`). One named instance of that class is the MCP surface opening **multiple independent connections** instead of a single owned one.
- **Scope tier:** **Standard.** Single domain (`services/mcp_v2/` only — the shared-`db_service` re-entrancy guard moved to §4 so this packet touches no shared substrate), no cross-cutting contract change beyond the internal `invoke` signature, no schema change, no auth-shape change. Reviewers confirmed Track-B-alone is Standard.
- **Trust topology:** `local-to-local single-user`, MCP carve-out preserved. This packet changes *how many connections the MCP process opens and how handlers receive DB access* — it does **not** change what MCP may read. ADR-0125 sensitivity gating on MCP egress is untouched (ADR-0125 §2: `Confidential`/`UserOnly` never cross MCP).
- **Migration slots:** none. Runtime-ownership + interface change only.

### §0.1 Symptom-to-Failure Trace

1. **Symptom (verbatim class):** `pairing_authority_unavailable: database is locked` / `database is locked` on MCP surfaces; corruption when a writer is killed mid-WAL.
2. **Process topology (verified `dev`, 2026-05-31 — the fact the V2-draft got wrong):**
   - MCP V2 is a **separate sidecar binary** `dailyos-mcp` (`src-tauri/Cargo.toml:139-142 [[bin]]`; `src-tauri/tauri.conf.json:33 externalBin`), spawned as its own OS process. `src/mcp/main.rs:1678 fn main` runs its own tokio runtime → `run_v2_server` (`mcp/main.rs:1695`).
   - The sidecar **does not install the global `DbService`** (`state.rs:1438 install_global` is app-process only; `db_service.rs:1044 GLOBAL` is process-local). `try_global()` returns `None` in the sidecar.
   - Production stdio mode constructs the transport with **no connection**: `transport.rs:81 from_local_stdio(... conn: None ...)`, dispatching via the no-connection branch in `handle_local_stdio_tool_call` (call at `transport.rs:255`). The `from_verified_pairing(conn)` path has no production caller (it is the paired-transport surface W2 is stripping).
3. **Residual failure point (in the sidecar):** because there is no pool, each handler and the audit path **self-opens its own connection**:
   - `services/mcp_v2/audit.rs:167 ActionDb::open(Arc::new(LocalKeychain::new()))` — independent **writable** open (the `try_global()` writer path at `audit.rs:139-165` is dead in the sidecar; only the fallback fires).
   - `services/mcp_v2/handlers/tool_account_status.rs:234 ActionDb::open_readonly(...)` — handler self-open (file is ~1,659 lines on current `dev`). `invoke` has no DB/services parameter (trait `contracts.rs:325-329`), which forces it.
   - Handlers also attach **service adapters that open DB outside `mcp_v2/`** (`services/context.rs:129/429/1024`) — so a handler-dir-only grep gate would miss real MCP DB access (codex P1).
   - Net: N independent connections per MCP process, each racing the app writer's WAL cross-process → the lock-storm/corruption class. (Collapsing N→1 reduces the sidecar's WAL-race footprint; *eliminating* the cross-process race is W2, §4.)
4. **Hypotheses already explored and rejected — do not re-walk:**
   - ❌ *"Relocation, not rewrite"* (V1 premise). False — handlers self-open, so the fix is an **interface change** (request-scoped context), not a file move. → this packet. Re-confirmed: no `McpHandlerContext` on `dev` (grep: 0 hits).
   - ❌ *"Add retry at the `db_read`/`db_write` helper."* The helpers stringified errors. **Resolved** by DOS-757 (`097d8171`): typed `DbAccessError` + `Retryable`. (The retry *consumer* moves to the Track-C/W2 follow-up, not this packet.)
   - ❌ *"Build a writer-mutex priority lane"* (V1 Track 2). **Superseded** by the merged foreground-contention work (PR #365) and **explicitly forbidden** by ADR-0133 §3 (the gate is policy-free by invariant). ⚠️ Note: `.docs/plans/v1.4.9-waves.md:77,89` still labels DOS-647 "single-process DB ownership **+ writer-priority lane**" — that phrasing is **stale**; ADR-0133 and this packet exclude the lane. Reviewers reading the waves doc must not re-import it.
   - ❌ *"Route MCP through the app single writer"* (V2-draft Track C). **wrong-cure** — MCP is out-of-process (§0.1.2); that needs IPC = W2. Removed (§4).

---

## §1 Substrate Audit (Ground Truth)

> `dev`-current as of 2026-05-31, verified by the L0 panel (codex + ce-feasibility opened each file). L1 re-confirms before editing.

### §1.1 What we build on (already shipped — do not rebuild; §1.2 of `db-lock-storm-class` doc)

- **Typed errors (DOS-757, `097d8171`):** `DbAccessError` + `class()`/`is_retryable()` at `db_service.rs:114-160`.
- **Writer thread + FIFO (no priority field by ADR-0133 invariant):** `db_service.rs:354-398`; `CallMessage` at `db_service.rs:78`. `call_sync_labeled` `db_service.rs:468-486`.
- **Fresh-open serialization:** `db_service.rs:838-869 open_fresh_serialized`.
- **Foreground contention (PR #365), DB-throughput W0 (`81292e62`), DB-mode 820-A/B (#418/#419):** merged; out of scope.
- **Write-transaction holder (diagnostics only, no re-entrancy prevention):** `db/core.rs:85-130`. The `PooledConnection`-level `ReentrantCall` guard does **not** exist (`db_service.rs` grep: 0 hits) — that guard is deferred to the §4 follow-up.
- **Documented nested-transaction contract (for the §4 follow-up guard):** ADR-0102 §11.2 — `ServiceContext` holds `Option<TxHandle>`; nested transactions forbidden via `Error::NestedTransactionsForbidden`. The deferred pool-layer guard *extends* this; it does not invent a parallel contract.

### §1.2 The residual (genuinely unbuilt — this packet)

- **No request-scoped handler context** — `McpToolHandler::invoke` (`contracts.rs:322-329`) takes only `(actor, params)`; can't thread DB/services. Handlers self-open as a result.
- **N independent connections per MCP process** — `audit.rs:167` + handler self-opens (`tool_account_status.rs:234`) + service-adapter opens (`services/context.rs:129/429/1024`).

(The nested-writer re-entrancy guard is *not* this packet — it has no deadlock surface in the pool-less sidecar; deferred to the §4 W2-coupled follow-up.)

### §1.3 Recoverable prior art (PR #355)

PR #355 (CLOSED, intact at `public/dos-758-mcp-handler-interface@fa4cd6f5`) designed `McpHandlerContext<'a>`, the reshaped `invoke(ctx, actor, params)`, the `ReentrantCall` guard, and the CI gate. **Design sound; cannot merge** — `dev` rewrote `gateway.rs` (real gateway, scope-manifest/audit) and `tool_account_status.rs` (real 374-line handler) and reworked `db_service.rs`. Treat as blueprint; re-derive.

---

## §2 Scope (Track B / DOS-758)

A request-scoped context that gives the MCP process **one owned connection threaded to every handler** and removes the handler/audit self-opens. (The nested-writer guard that hardens the *future* in-process-hosting interface is deferred to §4 — it has no deadlock surface in today's pool-less sidecar.)

1. **`McpHandlerContext`** — re-derive PR #355's shape against current `gateway.rs`. **Connection shape (codex-settled):** the sidecar holds **one owned `ActionDb`/connection on the server handler** for the process lifetime; each tool call builds a per-call `McpHandlerContext` that **borrows (or briefly locks) that one connection** + the service context. Not a per-handler open, not a new pool. Mirrors ADR-0102 §5's `AbilityContext` (abilities never open DB connections; they receive a `ServiceContext`).
2. **Reshape `McpToolHandler::invoke`** to `invoke(&self, ctx: &McpHandlerContext, actor, params)`. Handlers obtain DB/services from `ctx` — never `ActionDb::open` / `open_readonly` / `LocalKeychain::new`.
3. **Port `tool_account_status`** (and any other real handler + the audit write path `audit.rs:167`) onto the context. Reads via the context's read path; the audit write uses the single owned connection.
4. **CI gate (bounded static manifest + behavioral test):** A grep gate **cannot prove Rust call-graph reachability in-crate** (codex P1: `tool_placement` → `attach_live_workspace_readers_with_signal_engine` (`handlers/tool_placement.rs:128`) → `workspace_intake` (`services/context.rs:116`) → an open at `services/workspace_ingestion/workspace_intake_impl.rs:351`, outside both `mcp_v2/` and the cited `context.rs` sites). So the gate is scoped to a **bounded, enumerated manifest** of MCP-handler-reachable adapter files (maintained in the gate script header), forbidding `ActionDb::open` / `open_readonly` / `LocalKeychain::new` in `services/mcp_v2/handlers/*` **and** that manifest. Re-derive `scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh` (absent on `dev` — written from the PR #355 blueprint); wire into `preflight.sh` + rust workflow. Per `capability-boundary-needs-crate-split-not-grep-2026-05-18.md`, the grep gate is necessary-but-insufficient — **the behavioral test (B-AC2: single-open assertion across a multi-tool dispatch) carries the real weight**; the gate catches the obvious regression.
5. **Second-`DbService` negative check:** static check for `DbService::` construction / `install_global` under `src/mcp` + MCP-v2 wiring (codex P2 — make B-AC6 enforceable, not just a review assertion).

### Out of scope (handed off, not dropped)
- **Cross-process single-writer / IPC** (the sidecar reaching the app writer) → **Track C / W2-coupled follow-up** (§4). The retry-on-busy consumer of `DbAccessError` goes there too.
- **Nested-writer re-entrancy guard** (a `call_sync` re-entering the same pooled worker returning `PooledCallError::ReentrantCall` instead of deadlocking) → **moved to §4 follow-up.** Rationale (reviewer dissent resolved toward convergence): it modifies shared `db_service` + public `PooledCallError` (breaking this packet's single-domain/Standard claim — codex P1) and is *preventive for the future in-app-hosting path*, not load-bearing for the current pool-less sidecar (feasibility confirmed: handlers don't run on the app worker thread today, so the sidecar cannot hit this deadlock). It lands with the in-app/IPC work that actually creates the deadlock surface.
- **Signed-route transport / auth shape** → W2 (DOS-833).
- **The forbidden fake fix:** do **not** install a second `DbService` inside `dailyos-mcp` — that is "one writer per process" still racing the app writer cross-process (codex P2). One *owned connection* per process is the goal; not a second pool.
- Priority lane (superseded). Read models (foreground-contention Phase 2). Closing every `ActionDb::open` repo-wide.

---

## §3 Acceptance Criteria

- **B-AC1.** `McpToolHandler::invoke` takes a request-scoped `McpHandlerContext`; no handler — and no service adapter in the §2.4 manifest — self-opens a DB connection or keychain. Verified by the bounded CI gate **and** the behavioral test (static-only insufficient per K-in).
- **B-AC2.** The MCP process holds **one owned connection** on the server handler, borrowed per-call via the context — not N per-handler/per-audit opens. **Load-bearing test:** assert a single open across a multi-tool dispatch.
- **B-AC3.** `tool_account_status` returns output identical to current `dev` for the same inputs (parity test), now sourcing DB via `ctx`; the `audit.rs` write uses the owned connection.
- **B-AC4.** CI gate present, wired (`preflight.sh` + rust workflow), red on a planted `ActionDb::open` in a handler **and** on a planted open in a §2.4-manifest service-adapter file.
- **B-AC5.** Static negative check rejects any `DbService::` construction / `install_global` under `src/mcp` + MCP-v2 wiring (no second pool in the sidecar — §2.5).
- **B-AC6.** `cargo clippy -- -D warnings && cargo test` green.

---

## §4 Handoff: Track C (cross-process single-writer) → W2-coupled follow-up

The genuine single-writer fix for MCP — the sidecar's writes reaching the app's one writer — is a **process-boundary/IPC change inseparable from the transport layer W2 owns**. It is recorded as a follow-up on DOS-759 (which the reconciliation ledger already re-scoped away from signed-route transport):
- Decide the boundary: host MCP in-app, or make `dailyos-mcp` a proxy over local IPC to the app writer.
- Add the retry-on-busy consumer of `DbAccessError::is_retryable()` at that managed-writer boundary.
- **Nested-writer re-entrancy guard** (`PooledCallError::ReentrantCall` on same-worker `call_sync` re-entry, extending ADR-0102 §11.2). The deadlock surface only exists once MCP handlers run on the app's pooled worker (in-app hosting), so the guard lands with that change, not before.
- State MCP reader-tier placement per **ADR-0134 §1-2** (MCP joining the read path is amendment-worthy; name the tier).
- Sequenced with/after W2's settled auth/transport model. **Not a W3/W4 gate** — the correction-loop critical path runs on app+file surfaces, already single-writer per §1.1.

---

## §5 Test Plan

```bash
cargo clippy -- -D warnings
cargo test
```
Focused:
- Single-owned-connection-per-dispatch test (B-AC2 — load-bearing).
- `tool_account_status` context-parity test (B-AC3).
- CI-gate red-on-planted-open, handler **and** §2.4-manifest service-adapter (B-AC4).
- No-second-DbService static check (B-AC5).

## §6 Intelligence Loop Integration Check

Substrate/runtime change only — no claim, table, or user-visible field. Provenance unchanged (`Actor::McpClient` vs `Actor::User` preserved). ADR-0125 §2 MCP-egress sensitivity gating untouched. No signal/invalidation/feedback change.

## §7 Reviewer Dispatch (L0)

- **Default (Standard, 2):** `/codex challenge` + **`ce-feasibility-reviewer`** (architecture/dependency fit on `dev`-current internals).
- **K-in (mandatory, parallel):** `ce-learnings-researcher` — confirm consume-not-reinvent (done cycle-1; citations folded into §8).
- This is **L0 cycle-2**. Per pacing rule, a third non-converging cycle escalates to L6.

## §8 K-In Findings (folded from cycle-1 `ce-learnings-researcher`)

- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` — the named class; this packet is a faithful instance (consume). Reinventing it = BLOCK.
- **ADR-0133** §1/§8 — single mutating connection per process; CI-gate is mandated, not gold-plating. §3 forbids the priority lane (excluded here).
- **ADR-0102** §5 (`AbilityContext` pattern the `McpHandlerContext` mirrors) + **§11.2** (`NestedTransactionsForbidden` — the documented contract the §4 deferred re-entrancy guard extends).
- **ADR-0101** — `ActionDb::open` outside services is the boundary-bypass root; the self-opens are that class.
- **ADR-0134** §1-2 — reader-tier inventory + "new surface → amendment"; the MCP reader tier is named in the **§4 follow-up** (Track C), not here.
- **ADR-0125** §2 — per-claim sensitivity + MCP-egress rule; corroborates "gating untouched."
- **ADR-0067** — `with_db_*` / latency-rollups origin (substrate the §4 retry consumer will extend).
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — grep gate porous within a crate ⇒ behavioral test required (B-AC1).
- **K-out flag (for L3):** when 758 lands, capture *why PR #355 couldn't merge as-is* in `docs/solutions/` so the next person doesn't revive the stale branch.

## §9 Definition of Done

- B acceptance criteria validated with real data (MCP tool call end-to-end via `ctx`; one owned connection; no handler/adapter self-open).
- Bounded CI gate + load-bearing behavioral test + no-second-`DbService` check in place.
- Track C + the nested-writer guard explicitly handed to the W2-coupled follow-up on DOS-759 (recorded, not dropped).
- Gates green: `cargo clippy -- -D warnings && cargo test`.
