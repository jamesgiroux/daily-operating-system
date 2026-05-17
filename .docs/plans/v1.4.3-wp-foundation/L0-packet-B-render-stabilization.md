# L0 Packet B — WP preview/runtime render stabilization

**Current revision: V1.0 (initial draft, 2026-05-17). See §2 Changelog.**

## 1. Header

Date: 2026-05-17
Project: v1.4.3 — WordPress Foundation
Wave: Stabilization (parallel with Packet A — distinct file surface, distinct PR)
Issues:
- DOS-671 — block content disappears within ~30s after render
- DOS-672 — reload-from-runtime fires on every window switch; second reload returns verification banner
Surface: WordPress block editor preview + Tauri runtime project-composition route
Primary code:
- `wp/dailyos/blocks/account-overview/edit.js`
- `wp/dailyos/blocks/account-overview/render-functions.php`
- `wp/dailyos/includes/class-dailyos-plugin.php`
- `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`
- `src-tauri/src/surface_runtime/mod.rs` (project-composition route)
- `src-tauri/src/services/composition_render_orchestrator.rs`
- `src-tauri/src/bridges/surface_client.rs`
Primary anchor: `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` §"Threat model: local-to-local"
Diagnostic anchor: `.docs/plans/v1.4.3-wp-foundation/stabilization-investigation.md`
Architectural anchor (CHALLENGED): `src-tauri/src/surface_runtime/mod.rs:2348-2353` — the v1.4.2 "producer always advances composition_version monotonically and commits unconditionally" comment.

This packet covers two confirmed defects in the WP editor → preview → runtime
render path surfaced by v1.4.2 W4-F L4 hands-on (PR #298 merged
2026-05-17). Both root causes are confirmed in code (not session TTL — the
investigation explicitly rejects that framing).

This packet challenges one architectural decision from v1.4.2 W4-F: the
"always advance composition_version on render" pattern at `surface_runtime/mod.rs:2348`.
That pattern violates the local-to-local read contract (reads don't write).
The L0 reviewers — particularly CSO — MUST evaluate whether removing it
breaks the producer's OCC invariant that v1.4.2 was repairing.

This packet is intentionally narrow. It does NOT include the keychain +
lifecycle hardening (DOS-673/674/675) — that is L0 Packet A, separate review
track, separate PR.

## 2. Changelog

- **V1.0 (2026-05-17):** Initial L0 draft. Authored from
  `stabilization-investigation.md` plus direct verification of the cited
  file:line anchors (verified: render-path producer commit at `mod.rs:2348-2377`,
  cache key vs request version mismatch at `mod.rs:2317-2321` and `2425-2430`,
  PHP double-fetch at `class-dailyos-plugin.php:587-612` → `render-functions.php:55-59`,
  editor reload loop at `edit.js:62-68` + `:82` + `:84-86`, rate-limit budget
  at `surface_client.rs:213` = 5/sec composition-read burst). Reviewer panel
  set to codex challenge + code-reviewer + codex consult + CSO (mandatory
  given render-path authorization + rate-budget + producer-commit semantics
  are all in scope).

## 3. Status Snapshot

- Linear tickets: DOS-671, DOS-672 (both Backlog, both in v1.4.3 — WordPress Foundation).
- Investigation evidence: `stabilization-investigation.md` §"DOS-671 Confirmed Root Cause" / §"DOS-672 Confirmed Root Cause" / §"Executive Summary".
- Both root causes confirmed against current code (sha 9a33d347 on dev).
- The investigation explicitly rejects the ticket's "session TTL too tight" framing for both. The actual cause is a refresh-loop across editor + PHP preview + runtime render route, plus the project-composition route's render-path producer commit + cache-key-by-stale-watermark + render-read rate-budget consumption.
- Acceptance criteria specified per ticket in §7 below.
- Recommended landing shape: single PR. See §10 Interlocks.
- Reviewer panel: see §14.

## 4. Pre-work — substrate reuse audit

This packet REUSES the following existing primitives. No net-new substrate is
introduced; every change is an in-place modification of existing functions:

| Capability | Existing primitive | File:line |
|---|---|---|
| WP runtime client transport | `DailyOS_Runtime_Client::project_composition_for_surface` | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:279-288` |
| WP block render entry | `dailyos_account_overview_render` | `wp/dailyos/blocks/account-overview/render-functions.php:32` |
| WP preview REST route | `account_overview_preview` | `wp/dailyos/includes/class-dailyos-plugin.php:587` |
| WP nested render bridge | `render_block_with_filter` | `wp/dailyos/includes/class-dailyos-plugin.php:682` |
| Editor reload callback | `reload` in `AccountOverviewEdit` | `wp/dailyos/blocks/account-overview/edit.js:43-82` |
| Runtime project-composition route | `surface_runtime::project_composition` (handler) | `src-tauri/src/surface_runtime/mod.rs:~2240+` |
| Runtime authorization | `runtime.surface_client_bridge.authorize(...)` | `src-tauri/src/surface_runtime/mod.rs:2288-2311` |
| Projection cache | `orchestrator.cache_lookup` / `cache_store` | `src-tauri/src/services/composition_render_orchestrator.rs:51-56` |
| Producer invocation | `invoke_registry_json_for_actor` | `src-tauri/src/surface_runtime/mod.rs:2378` |
| Composition commit | `LiveCompositionCommitter` via `commit_composition` | `src-tauri/src/services/context.rs:108-136` |
| Standard-read-composition rate budget | `SurfaceClientAbilityClassLimits::standard_read_composition` | `src-tauri/src/bridges/surface_client.rs:213` |

## 5. What this packet authors net-new

### 5.1 Single-fetch PHP preview (DOS-671 + DOS-672, shared)

Refactor `dailyos_account_overview_render` so the runtime call is **optional**:

```php
function dailyos_account_overview_render( array $attributes ): string {
    // ... validation as today ...
    $response = dailyos_account_overview_fetch_projection( $attributes );
    return dailyos_account_overview_render_from_projection( $response, $attributes );
}

function dailyos_account_overview_render_from_projection( $response, $attributes ): string {
    // pure projection-to-HTML — no runtime call
}
```

`account_overview_preview` calls `project_composition_for_surface` once, then
calls `dailyos_account_overview_render_from_projection` directly (NOT
`render_block_with_filter` → `dailyos_account_overview_render` which would
re-call the runtime). One preview = one runtime call.

### 5.2 Editor reload guard (DOS-671 + DOS-672, shared)

Replace `useEffect( () => reload(), [reload] )` at
`wp/dailyos/blocks/account-overview/edit.js:84-86` with an explicit trigger
that fires ONLY when:
- `attributes.composition_id` transitions from empty to non-empty, OR
- `attributes.account_id` changes,
- but NOT when `composition_version` or `cache_hint_token` change.

The `reload` callback's dependency list at `:82` is also trimmed:
- Remove `attributes.composition_version` and `attributes.cache_hint_token` from deps.
- Reload reads them at call time via the latest `attributes` reference.

Failed-reload behavior:
- Preserve last-good `preview` state.
- Show an `error` notice in the editor but do NOT replace rendered content with the verification banner envelope.

### 5.3 Runtime cache-key correction (DOS-671 + DOS-672)

At `src-tauri/src/surface_runtime/mod.rs:2317-2321` and `:2425-2430`, the cache
is keyed on the request's `composition_version` (which is the surface's stale
watermark). After the fix, cache is keyed on the **effective projection
version** (the one the producer is about to emit or has just emitted), so
repeated requests with stale watermarks all hit the same cache entry.

Two viable shapes — pick one in L0:
- **(a)** Look up cache by `(actor, composition_id, current_db_version)`, not request version. Store by the projection's actual version (already fetched at `:2355-2370`).
- **(b)** Cache becomes version-agnostic: lookup by `(actor, composition_id)`, invalidate when producer commits a new version.

§12 Q1 picks between (a) and (b).

### 5.4 Render-path producer-commit removal (DOS-671 — ARCHITECTURAL CHALLENGE)

This is the contested change. Today at `surface_runtime/mod.rs:2348-2353`:

> The producer always advances composition_version monotonically and commits unconditionally. The surface's previously-rendered version is structurally meaningless as an OCC token because the surface has no write path — only the producer mutates compositions. Forward the current substrate version on every render so the producer's commit step never trips its own watermark check on stale surface claims.

The local-to-local threat model says reads don't write to `surface_client_sessions`,
audit tables, rate-limit buckets, **or any other DB state**. The composition
table is "any other DB state" — and producer commit on every render writes to it.

The investigation proposes:
- Render path reads existing projected state from cache or DB.
- Producer commit happens ONLY on:
  - Explicit user-initiated refresh (the editor's "Reload from runtime" button),
  - Signal-propagation invalidation (a watermark moved upstream — `account_subject.claim_changed`, `claim.lifecycle`, etc).
  - Initial composition creation (first request for a composition_id).

The producer's OCC-token issue that v1.4.2 W4-F was repairing (DOS-670 substrate-side fix) had a narrower scope: the producer's commit step needs the current DB version, not a stale surface watermark. That requirement is still satisfied if we only commit on triggers, not on every read.

**§12 Q2** is the explicit reviewer prompt for whether removing render-path
commit breaks DOS-670's substrate fix.

### 5.5 Render-read decharge from `standard_read_composition` budget (DOS-672)

At `src-tauri/src/bridges/surface_client.rs:213`, the default budget is 60/min
+ burst 5. Even on a single user with one editor open, the auto-reload loop
plus PHP double-fetch can exhaust the 5-burst within seconds.

Two viable changes — pick one in L0:
- **(a)** Local-render reads from `surface_client` actor on a paired loopback session are NOT gated by `standard_read_composition`. Authorization (scope check, actor allowlist) remains mandatory.
- **(b)** The budget is raised significantly for local sessions (e.g. 600/min / burst 50), keeping the gate but sizing it for local.

§12 Q3 picks between (a) and (b).

### 5.6 Typed transport/session error mapping (DOS-672)

At `wp/dailyos/blocks/account-overview/render-functions.php:61-70`, ALL error
states currently collapse into the verification banner (
"Something about this account doesn't line up. Verify before acting.").

The fix: distinguish at minimum:
- `rate_limited` (HTTP 429) → "Runtime is throttling; retry shortly."
- `session_requires_repair` (per W4-F error code) → "Surface session needs repair; reconnect."
- `session_not_found` → same as repair.
- `runtime_request_failed` (transport / timeout / 5xx) → "Runtime unavailable; retry."
- consistency-failure (the actual case the banner was written for: projection inconsistency, contradicting evidence) → keep the verification banner.

The PHP renderer needs a typed error shape from the runtime response (already
present — runtime returns `{ ok: false, code: "...", ... }`); the renderer
maps `code` → user-facing string. The verification banner is reserved for
true consistency failures.

## 6. Directional decisions resolved at L0

### 6.1 The investigation's "session TTL too tight" framing is WRONG and must be rejected

The runtime sets `SESSION_ABSOLUTE_TTL_SECONDS` to 365 days
(`surface_pairing.rs:20-25`). Read validation does NOT mutate session state on
success (`surface_pairing.rs:959-963`). DOS-671/672 are NOT session lifetime
problems. Do not fix by shortening or refreshing sessions.

### 6.2 The investigation's "double fetch" diagnosis is the load-bearing root cause

PHP preview calls runtime once at `class-dailyos-plugin.php:587-591`, then
calls `render_block_with_filter` which calls
`dailyos_account_overview_render` which calls runtime AGAIN at
`render-functions.php:55-59`. Two runtime calls per preview is the primary
cause of both 30s disappearance (the second call can hang up to the 30s signed-post
timeout — `class-dailyos-runtime-client.php:279-288`) and verification banner
(the second call can fail while the first succeeded — `render-functions.php:61-70`
emits the banner; `class-dailyos-plugin.php:613-618` merges the first-call response with the second-call banner HTML).

### 6.3 The editor's auto-reload loop is the secondary root cause

The success path writes `composition_version` + `cache_hint_token` back to
attributes (`edit.js:62-68`); those are dependencies of `reload`
(`edit.js:82`); `useEffect([reload])` retriggers on callback identity change
(`edit.js:84-86`). Every successful reload schedules another reload. Combined
with the 5/sec burst budget, this exhausts rate limits and produces the
"reload-on-window-switch" symptom.

### 6.4 Removing render-path producer-commit is the right local-shaped fix — but is L0-CONTESTED

The investigation argues for removal. v1.4.2 W4-F explicitly chose
"always-forward DB version + producer commits unconditionally" as the
DOS-670 substrate-side fix. The two appear to conflict.

**§12 Q2** is the explicit L0 question for the reviewer panel: does the
DOS-670 producer OCC contract require commit-on-render, or only
commit-on-trigger?

### 6.5 Render-read decharge does not weaken authorization

Authorization (scope check, actor allowlist) remains mandatory before serving
even cache hits — that's the v1.4.2 W4-F load-bearing rule, restated in
`stabilization-investigation.md` §"DOS-672 Risk and Blast Radius". What
changes is whether render reads CONSUME rate-budget. Authorization-without-consumption
is a coherent local-shape; remote shapes would consume.

### 6.6 Last-good preview is editor-side state, not runtime contract

The "preserve last-good preview on error" rule lives in `edit.js`. The
runtime keeps returning typed errors; the editor decides whether to clobber
its rendered state. No runtime contract change for this rule.

## 7. Acceptance criteria

### DOS-671 (block content disappearance ≤30s)

1. PHP preview makes exactly one `project_composition_for_surface` runtime call per preview request.
2. Editor `useEffect`-driven auto-reload fires ONLY on `composition_id` empty→non-empty or `account_id` change.
3. Editor `reload` callback's dependency list does NOT include `composition_version` or `cache_hint_token`.
4. Successful reload's attribute write does NOT schedule another reload.
5. Runtime `project_composition` route does NOT commit a new composition on every render. Commit happens on: explicit refresh, signal-propagation invalidation, initial composition creation.
6. Runtime cache lookup hits regardless of stale request `composition_version` (cache keyed on current DB version OR version-agnostic per §5.3 outcome).
7. Local-render reads from paired-loopback `surface_client` sessions do NOT consume `standard_read_composition` rate budget (or budget is raised per §5.5 outcome).
8. After initial render, the block remains visible for ≥60 seconds without further user action AND across two browser-window focus changes.

### DOS-672 (reload-on-window-switch + second-reload verification banner)

9. Manual "Reload from runtime" fires exactly one preview request per click.
10. Window-focus change does NOT trigger reload (no remount).
11. Second consecutive manual reload succeeds (returns projection, not banner) — assuming first reload succeeded.
12. Failed reload (any cause) preserves last-good `preview` state in the editor.
13. Failed reload surfaces an `error` notice; does NOT replace rendered content with verification-banner HTML.
14. PHP renderer maps `rate_limited`, `session_requires_repair`, `session_not_found`, `runtime_request_failed` to distinct typed messages; verification banner reserved for true consistency-failure (projection contradiction / missing evidence).
15. PHP preview response merge (`class-dailyos-plugin.php:613-618`) does NOT combine successful first-call projection with failed second-call banner HTML. After §5.1, there IS no second call.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `dos671_preview_single_fetch` | PHP fake runtime returns projection on first call, asserts no second call happens during preview render |
| 2 | `dos671_editor_no_reload_on_cache_token_change` | JS test with fake `apiFetch`: setAttributes({cache_hint_token: 'new'}) does NOT trigger a reload |
| 3 | `dos671_editor_no_reload_on_version_change` | JS test with fake `apiFetch`: setAttributes({composition_version: 5}) does NOT trigger a reload |
| 4 | `dos671_render_path_no_commit` | Rust test: 10 consecutive `project_composition` calls without external signal → no composition commit, no version advance |
| 5 | `dos671_render_path_commits_on_trigger` | Rust test: signal propagation (`account_subject.claim_changed`) → next render commits new version |
| 6 | `dos671_cache_hits_with_stale_request_version` | Rust test: lookup with request_version=1 hits cache populated by request_version=5 (or version-agnostic) |
| 7 | `dos671_local_render_no_rate_consumption` | Rust test: tight `standard_read_composition` budget (1/min), 50 render reads from paired-loopback do NOT fail with `rate_limited` |
| 8 | `dos672_manual_reload_single_request` | JS test: button click → exactly one `apiFetch` call |
| 9 | `dos672_second_reload_returns_projection_not_banner` | PHP fake runtime returns projection on both calls; HTML output of second preview render contains block payload, NOT verification banner |
| 10 | `dos672_failed_reload_preserves_last_good` | JS test: first reload succeeds → preview set; second reload fails (mocked error) → preview unchanged, error notice shown |
| 11 | `dos672_typed_error_mapping` | PHP test: runtime response `{ok: false, code: "rate_limited"}` → user-facing string "Runtime is throttling..."; NOT the verification banner |
| 12 | `dos672_verification_banner_reserved_for_consistency` | PHP test: runtime response `{ok: false, code: "consistency_failure"}` → verification banner DOES render |
| 13 | `dos671_l4_hands_on_log` | Hands-on log captured: initial render → wait 60s → focus switch x2 → manual reload x2; content remains visible throughout |

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | `dailyos_account_overview_render` is called at most once per preview request | grep / lint: `account_overview_preview` body does not call `render_block_with_filter` for the block-render path; calls `dailyos_account_overview_render_from_projection` directly |
| 2 | Editor `reload` callback dep list contains exactly `[attributes.composition_id, attributes.account_id, setAttributes]` | ESLint rule or test that asserts the dep array literal |
| 3 | `useEffect` for auto-reload depends on a narrow trigger value, not `reload` | ESLint rule or test |
| 4 | Runtime `project_composition` handler does NOT call `commit_composition` on the steady-state read path | grep / AST check on the read-path branch body — no `commit_composition` call without a `requires_commit` guard |
| 5 | Projection cache key (effective shape per §5.3 outcome) is invariant under request_version changes | runtime test asserts cache hit rate ≥ 95% in a workload that varies request_version but holds composition_id constant |
| 6 | Local-render read decharge: `standard_read_composition` budget is not consumed for paired-loopback render | runtime test with tight budget (already enumerated as fixture #7) |
| 7 | PHP error mapping uses a typed code → string lookup table | grep for `dailyos_account_overview_render_verification_banner` outside the consistency-failure branch fails CI |

## 10. Interlocks

DOS-671 and DOS-672 share every file in scope (`edit.js`,
`render-functions.php`, `class-dailyos-plugin.php`,
`class-dailyos-runtime-client.php`, `surface_runtime/mod.rs`,
`composition_render_orchestrator.rs`, `surface_client.rs`).

**Landing shape:** single v1.4.3 stabilization-B PR with five commit groups:
1. PHP single-fetch refactor (DOS-671 §5.1).
2. Editor reload guard + last-good preserve (DOS-671 §5.2, DOS-672 §5.6 editor side).
3. Runtime cache-key correction (DOS-671 §5.3).
4. Render-path producer-commit removal (DOS-671 §5.4) — **the architecturally contested one**.
5. Render-read decharge + typed error mapping (DOS-672 §5.5, §5.6 runtime side).

Splitting this PR is NOT viable. Each fix in isolation produces false
confidence: fix the editor without the PHP double-fetch and 50% of reloads
still hang; fix PHP without the cache-key issue and the cache stays useless;
fix the cache without removing render-path commit and you still write
unnecessarily on read; etc. The investigation §"Coordination — Recommended
Landing Shape" calls this out explicitly.

**Cross-packet interlock:** Packet A (lifecycle hardening) and Packet B
(render stabilization) touch `src-tauri/src/surface_runtime/mod.rs` but in
disjoint regions (Packet A: startup rehydration §667-740 + shutdown §327-340;
Packet B: project-composition handler §2280-2440). They can land in either
order; rebase cost is minimal.

## 11. What this packet explicitly does NOT own

- **DOS-673 / DOS-674 / DOS-675 (keychain + lifecycle hardening).** Separate
  L0 packet (A). Distinct file surface (keychain + pairing + endpoint state),
  distinct reviewer concerns.
- **W4-F substrate (PR #298) producer/projection/renderer contract.** Already
  landed. This packet REUSES the contract; it does NOT change `ProjectedBlock`
  / `ProjectedComposition` shape, the field-binding path semantics, or the
  trust-band hoist behavior.
- **C1 starter kit** (block.json / render.php / producer / projection rule /
  integration fixture). Distinct work track; will get its own L0 packet after
  the stabilization tickets land.
- **WP block REST routes other than `account-overview/preview` and
  `account-overview/accounts`.** Out of scope.
- **Composition write paths** (feedback writes, edit affordances). Distinct
  work track — v1.4.3 W3/W4 feedback infrastructure has its own L0 packet to
  follow.
- **Studio sandbox runtime discovery** (C3 in v1.4.3 project description).
  Distinct work track.

## 12. Open questions for L0 reviewers

1. **(For codex consult + code-reviewer):** Cache key shape per §5.3 — (a) key on `current_db_version` (lookup races: between current_db_version read and cache_lookup, producer could commit) vs (b) version-agnostic key with explicit invalidation on commit. Pick one with implementation cost + correctness analysis.
2. **(For CSO + codex challenge):** Removing render-path producer-commit per §5.4 — does this break the DOS-670 substrate-side OCC contract that v1.4.2 W4-F shipped? Specifically: when the producer commits on EXPLICIT refresh, can the surface still receive a projection that reflects the committed version, or does the commit-then-read window introduce a new race?
3. **(For code-reviewer + codex consult):** Render-read decharge per §5.5 — (a) bypass `standard_read_composition` for paired-loopback render entirely vs (b) raise the budget significantly. Pick one.
4. **(For codex challenge):** The investigation says "30s disappearance" is "the 30s signed-post timeout plus retrigger". Can codex propose a concrete reproduction script that confirms the timing chain end-to-end (browser DevTools network + Tauri audit log)?
5. **(For CSO):** Typed error mapping per §5.6 — does exposing distinct error codes (`rate_limited`, `session_requires_repair`, etc) to the WP-side give an attacker a fingerprinting channel? Local-to-local says yes-because-WP-is-trusted, but please verify against any threat model that contemplates a hostile WP plugin co-resident on the same machine.
6. **(For codex consult):** What's the actual user-trigger surface for "explicit refresh"? Today the editor has a button (`reload`); is there also a Gutenberg context that auto-saves, and does auto-save count as explicit refresh?

## 13. Linear dependency edges

- v1.4.3 stabilization-B PR closes DOS-671, DOS-672.
- No upstream Linear dependencies — substrate already exists from v1.4.2 W4-F (PR #298 merged 2026-05-17).
- Soft dependency on Packet A: if Packet A's keychain classification lands first, the typed error mapping in §5.6 has one more error code to cover (`session_requires_repair` from a transient lookup `Unavailable`). Packet B can absorb this in the same enum.
- Downstream: every v1.4.3+ WP block depends on the stabilized render path; without these fixes, the C1 starter kit's reference implementation is the same broken pattern.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | Why |
|---|---|---|
| `/codex challenge` | adversarial | Specifically: stress the producer-commit-removal claim. Construct a scenario where the cache returns a projection that doesn't match the substrate's current state because the producer hasn't run yet. Stress the cache-key change for stale-version + concurrent commit races. |
| `code-reviewer` (claude) | domain | The render route is the hottest path in the system. Independent read by the domain reviewer catches places where the fix shape conflicts with existing patterns (request-id propagation, audit emission, error envelope shape, http status code semantics). |
| `/codex consult` | implementation feasibility | Walk the proposed single-fetch refactor through every existing caller of `dailyos_account_overview_render` (block render path, REST preview, any new caller v1.4.3+ might introduce); confirm no behavioral break. |
| `/cso` | **mandatory** | Render-path authorization (must NOT be bypassed), rate-budget consumption semantics (the decharge is policy-load-bearing), producer-commit semantics (write-on-read removal touches the substrate's trust-of-its-own-version invariant), and typed-error-mapping (fingerprinting risk vs operability). Every one of these is trust-boundary. |

**Convergence rule:** unanimous APPROVE required before code lands. Any reviewer
returning CONDITIONAL APPROVE → fold finding into V2 of this packet, re-run all
four reviewers. Cycle cap: 3 cycles before escalation to L6.

**Specific L6 trigger:** if CSO and codex challenge disagree on §5.4 (render-path
producer-commit removal), escalate to L6 immediately. This is the contested
architectural call, and the v1.4.2 W4-F authors deliberately chose the current
shape. James decides.

## 15. Acceptance for L0 closure

- [ ] All 4 reviewers returned APPROVE.
- [ ] All 15 acceptance criteria (§7) are testable; per-criterion fixture mapped (§8 has 13 — gap analysis in cycle 1).
- [ ] All 7 CI invariants (§9) have concrete grep/AST/runtime enforcement.
- [ ] All §12 open questions resolved.
- [ ] §5.4 producer-commit-removal explicitly approved by CSO (not just non-blocking).
- [ ] §5.3 cache key shape picked (a or b) with implementation rationale recorded.
- [ ] §5.5 render-read decharge approach picked (a or b) with implementation rationale recorded.
- [ ] Landing shape (§10) confirmed: single PR with 5 commit groups, no split.
- [ ] No outstanding L0-cycle findings; packet is implementation-ready.

When all nine boxes check, L0 is closed and implementation begins. L1 (self)
proof bundle includes: PHP unit output for single-fetch + typed-error mapping,
JS test output for reload lifecycle, Rust unit output for cache hit rate + no-commit-on-read,
hands-on log (initial render → 60s → focus x2 → manual reload x2), audit-log
excerpt showing no render-path composition commit and no rate-budget consumption
on local render reads.
