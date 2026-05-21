# L2 cycle-2 CSO verdict — wave/v1.4.4-w1-stage1a

**Mode:** `/cso` daily (8/10 confidence gate)
**Diff range:** `9544d930..HEAD` (cycle-2 delta only)
**Scope bound:** `envelope_cache.rs`, `claim_receipt/event_bridge.rs`, `claim_receipt/render.rs`
**Cycle-1 findings:** already routed to path-α — NOT re-litigated.
**Date:** 2026-05-20
**Reviewer:** CSO (daily mode)

---

## VERDICT: APPROVE — NO BLOCKING FINDINGS

Cycle-2 patches do not introduce literal AC violations, ADR-named contract violations,
or regressions versus cycle-1 approval. The three modules audited implement the
expected cycle-1 fix shape:

1. **`envelope_cache.rs`** — server-side substrate primitive that solves the cycle-1 F3
   tautological-binding finding by giving `submit_claim_feedback_command` a way to bind
   the target claim against a *real* envelope set rather than against itself. The W1
   middle-ground (cache-miss fallback + `path_alpha_envelope_cache_v2` marker) is
   explicitly documented and matches the parent agent's specified deferral. The other
   authorization layers (sensitivity gate at `privacy.rs`, agent-actor denial at
   `claim_feedback.rs`, action metadata schema, source content hash validation) remain
   the load-bearing security boundary in v1.4.4 — envelope_cache strengthens defense in
   depth, it is not the only gate.

2. **`event_bridge.rs`** — minimal signal→Tauri-event bridge. Payload is correctly
   scoped to non-sensitive fields (`signal_type`, `claim_id`, `from`/`to`
   verification-state strings). No sensitivity, no subject_id, no rendered text, no
   trust scores, no provenance. The `OnceLock<AppHandle>` pattern matches the existing
   precedent at `claim_feedback.rs::IDEMPOTENCY_CACHE` and `intel_queue.rs`. Best-effort
   emission (`return false` on missing handle) means substrate writes never block on
   the bridge.

3. **`render.rs` audience routing** — `audience_for_surface()` is the production
   wiring that ADR-0129 + AC-341.4 requires: `SurfaceContext::Mcp → Audience::AgentMcp`,
   all four Tauri surfaces → `Audience::UserTauri`. Audience is INPUT to
   `build_receipt_for_audience()`, NOT a post-construction filter — confirmed by
   reading `privacy.rs:170-225` (the audience arm is selected before any field is
   populated). Three new tests lock the contract: `ac_341_4_mcp_surface_routes_through_agent_mcp_audience`,
   `ac_341_4_tauri_surface_routes_through_user_tauri_audience`,
   `audience_for_surface_mapping`. A regression broadening Mcp back to UserTauri
   (cycle-1 F1 leak) would fail all three.

The audited cycle-2 delta is sound. Defense-in-depth is preserved or improved on every
substrate boundary touched.

---

## Per-finding (path-α only — no blockers)

### Finding 1 (path-α) — `envelope_cache.rs:49` — Cache eviction is best-effort, not bounded under burst

* **Severity:** MEDIUM (theoretical hardening, NOT blocker)
* **Confidence:** 8/10
* **Status:** VERIFIED via code read
* **Category:** Phase 5 (Infrastructure) / resource bound
* **Description:** `CACHE_MAX_ENTRIES = 2_048` enforces a soft cap via "drop oldest by
  `inserted_at`" linear-scan eviction on insert. Under concurrent insert bursts the
  write lock serializes, but the eviction path is O(N) per insert at capacity
  (`min_by_key` over the full map). At 2048 entries with 5-minute TTL, steady-state
  arrival above ~7/sec would keep the map continuously near cap and pay O(N) on every
  insert. Not a security finding (memory bounded; no path to OOM the host), but worth
  filing for the LRU upgrade.
* **Exploit scenario:** None directly exploitable. An attacker with the ability to
  produce many envelope render events could degrade the cache to O(N) inserts. The
  attacker must first be authenticated and able to invoke ability producers — by
  which point much cheaper DoS surfaces exist.
* **Path-α reason:** Not a literal AC violation, not an ADR contract violation, not a
  cycle-2 regression. The cap exists; it just isn't an LRU. Memory cannot grow
  unbounded.
* **Recommended fix (maintenance ticket):** Replace `min_by_key` linear scan with a
  proper LRU (e.g. `lru::LruCache` already in workspace, or a `VecDeque<String>`
  recency log alongside the HashMap). File under DailyOS Codebase Maintenance &
  Production Quality (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) as "envelope_cache LRU
  upgrade — convert O(N) eviction to O(1)."

---

### Finding 2 (path-α) — `event_bridge.rs:88` — Tauri event broadcasts globally to all windows/listeners

* **Severity:** MEDIUM (theoretical, defense in depth)
* **Confidence:** 7/10
* **Status:** UNVERIFIED (Tauri internals; documented `emit` is global, `emit_to`
  scopes to a label)
* **Category:** Phase 6 (Integrations) / information flow
* **Description:** `handle.emit(CLAIM_RECEIPT_INVALIDATION_EVENT, payload)` broadcasts
  to every window. In v1.4.4 the macOS app has one main window so this is moot.
  However, when v1.4.4+ adds secondary windows (e.g. detached MCP inspector, multi-
  document workspace), every listener receives every claim_id invalidation regardless
  of which window's surface owns that claim. The TS hook filters
  (`event.payload.claimId !== claimId`) so this is not a leak by construction — the
  payload contains only `claim_id` + verification-state strings, no sensitivity, no
  subject_id, no body — but it does broaden the event surface area.
* **Exploit scenario:** No direct exploit. A future window with a compromised
  renderer process (not in current threat model) could observe claim mutation
  frequency without subscribing through the receipt-render command path.
* **Path-α reason:** Not a literal AC violation. ADR-0129 doesn't speak to Tauri
  event scoping. No cycle-2 regression (cycle-1 had no bridge at all).
* **Recommended fix (maintenance ticket):** When secondary windows ship, evaluate
  `emit_to(window_label, ...)` scoping, or move to a per-window subscription
  primitive that filters at emit time. Capture as substrate refinement when
  multi-window support is added.

---

### Finding 3 (path-α) — `event_bridge.rs:33-66` — No coalesce window at emit site

* **Severity:** LOW (already mitigated downstream)
* **Confidence:** 8/10
* **Status:** VERIFIED — coalesce is enforced in the TS hook, not the bridge
* **Category:** Phase 6 (Integrations) / event flooding
* **Description:** The bridge emits a Tauri event for every signal commit with no
  rate limit, throttle, or coalesce window. A burst (e.g. user triages five claims
  rapidly) produces five IPC events. Mitigation lives in
  `useClaimReceiptSubscription.ts:46` — `RECEIPT_COALESCE_WINDOW_MS = 250` per
  `(claimId, surface)` pair via trailing-edge debounce, locked by AC-339.6. Cost is
  IPC frame overhead for the discarded events, not duplicated DB or render work.
* **Exploit scenario:** Authenticated user can produce N events per N feedback
  actions. Bounded by Tauri command rate (sensitivity gate + idempotency cache at
  `commands::claim_feedback`).
* **Path-α reason:** AC-339.6 places coalesce at hook side, not bridge side. Cycle-2
  bridge faithfully implements the contract the hook expects.
* **Recommended fix:** None required. If burst IPC volume ever shows up in profiles,
  a tokio-side coalesce at the bridge keyed on `(claim_id, signal_type)` with a small
  window would compress before crossing the boundary. File only if observed.

---

## Confirmations (not findings — positive verification)

### C1 — `event_bridge.rs` payload scope is correct

`ClaimReceiptInvalidationPayload` carries exactly four fields: `signal_type`,
`claim_id`, `from`, `to`. None of these are sensitive in isolation:
- `claim_id` is an opaque internal identifier (UUID-shaped); the hook filters by
  this value before re-fetching, so a listener on another claim sees only "some
  claim mutated, not mine."
- `from`/`to` are verification-state strings (`active`, `contested`, etc.) — public
  vocabulary defined in `ClaimVerificationState`.
- No subject_id, no sensitivity tag, no rendered text, no provenance, no trust
  score, no source label, no source_asof.

The full receipt projection (which IS audience-scoped) is fetched separately via
`render_claim_receipt` command which routes through `audience_for_surface()`. The
event is just a nudge; the privacy gate is the round-trip.

### C2 — Audience routing has no bypass path

`render_receipt_for` (render.rs:59) is the ONLY public render entry point in the
new module. Every code path inside reaches `build_receipt_for_audience(target,
audience, conn)` with `audience = audience_for_surface(surface)`. There is no
field-by-field post-filter; the audience arm in `privacy.rs:215-225` selects the
ENTIRE builder. AC-341.4 satisfied; AC-341.12 (claim_id + subject_id scrubbing for
AgentMcp) verified by `ac_341_4_mcp_surface_routes_through_agent_mcp_audience`
test which asserts both are empty strings on the Mcp path.

### C3 — `OnceLock<AppHandle>` semantics are safe

`set_app_handle` uses `OnceLock::set`, returning `Err` on already-initialized
(treated as a debug log, not a real error). `lib.rs:174` calls it exactly once
during Tauri setup. There is no path to install a fraudulent handle from outside
the Tauri runtime — the value installed comes from `app.handle()` which is owned
by Tauri itself. No code path mutates the handle after install. Matches the
established `intel_queue.rs:626` pattern.

### C4 — Cache-miss fallback does not escalate authority

When the envelope_render_id is absent or stale (cache miss), the fallback at
`commands/claim_feedback.rs:114-141` builds a single-claim envelope from the
target's own id and logs a warning. This is the SAME envelope shape that existed
pre-cycle-2 — the cache addition is strictly an *upgrade* path, never a
demotion. The path-α v2 deferral is correct: when ability-side wiring lands in
W2, every renderable producer will populate the cache and the fallback path
becomes warning-only.

---

## Active verification performed

- **Audience routing variant search:** Greppped for `build_receipt_for_audience(`
  callers across `src-tauri/src/services/claim_receipt/render.rs` and consumer
  surfaces. Only one production call site (render.rs:87); tests in privacy.rs
  exercise the other variants directly. No bypass found.
- **Bridge payload audit:** Read every field on `ClaimReceiptInvalidationPayload`,
  confirmed against the TS interface in `useClaimReceiptSubscription.ts:48-56`.
  Camel-case serialization locked by unit test `payload_serializes_to_camel_case`.
- **OnceLock install path:** Traced `set_app_handle` callers — exactly one
  install site at `lib.rs:174` under Tauri setup.
- **Cache eviction safety:** Read `EnvelopeCache::insert` (envelope_cache.rs:86)
  — write lock held during eviction + insert, no panic paths, `min_by_key` is
  total over a non-empty map (guard at `>= CACHE_MAX_ENTRIES`).

---

## Filter stats

- Candidates scanned: 6 (3 modules × 2 threat classes each)
- Hard-exclusion filtered: 1 (memory exhaustion in memory-safe language without
  exploit path — F1 finding rule 9)
- Confidence-gate filtered: 0 (all surviving candidates ≥7/10)
- Reported as blockers: 0
- Reported as path-α: 3

---

## Disclaimer

This tool is not a substitute for a professional security audit. /cso is an
AI-assisted scan bounded to the cycle-2 delta on the three modules specified by
the L2 orchestrator. It does NOT re-audit cycle-1 substrate or PR scope outside
the diff range.
