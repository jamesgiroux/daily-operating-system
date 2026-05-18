# L0 Packet B — WP preview/runtime render stabilization

**Current revision: V1.1.1 (cycle-2 text fixes, 2026-05-17). See §2 Changelog.**

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

This packet covers two confirmed defects in the WP editor → preview → runtime
render path surfaced by v1.4.2 W4-F L4 hands-on (PR #298 merged
2026-05-17). Both root causes are confirmed in code (not session TTL — the
investigation explicitly rejects that framing).

**Intelligence Loop integration check — exempt.** No claim/table/surface
added; no provenance/trust impact; no signal change; no runtime context
surface consumes new state; no feedback loop change. Purely render-path
operational hardening on existing primitives. CLAUDE.md §"Critical Rules —
Intelligence Loop integration check" does not apply. (Acknowledgement: the
v1.4.3 fix preserves signal-propagation triggers for the producer — see
§5.4 V1.1; nothing about the existing producer/projection/renderer contract
changes.)

This packet is intentionally narrow. It does NOT include the keychain +
lifecycle hardening (DOS-673/674/675) — that is L0 Packet A, separate review
track, separate PR.

## 2. Changelog

- **V1.1.1 (2026-05-17, cycle-2 text fixes):** All 4 cycle-2 reviewers
  returned non-BLOCK (CSO APPROVE, codex challenge CONDITIONAL APPROVE,
  code-reviewer CONDITIONAL APPROVE, codex consult CONDITIONAL APPROVE).
  All CONDITIONAL APPROVE findings are text-only — no architecture change.
  Folded inline:
  - **§5.3 + §5.4 clarification** (codex challenge LOW): "state movement"
    means `composition_versions` row advancement through
    `commit_composition`; upstream claim/source movement reaches this
    cache through the existing DOS-589 / W4-A0 recomposition contract.
    The cache invalidation proof is exact for committed composition-version
    movement.
  - **§5.5 type-shape correction** (codex consult): the existing substrate
    `charge_ability_scope` is a boolean (`surface_client.rs:642`), not an
    enum. V1.1.1 specifies a boolean parameter. Also cites the existing
    precedent at `surface_client.rs:936` where another helper already
    constructs requests with `charge_ability_scope=false`.
  - **§5.2 ESLint suppression note** (codex consult): WP-side
    `wp-scripts lint-js` may warn on `useEffect([reloadTrigger])` omitting
    `reload` from the dep list. Spec: use an explicit ESLint disable
    comment with rationale — or, if `react-hooks/exhaustive-deps` is set
    to `warn` rather than `error`, the warning is non-blocking. Both
    options preserve the architectural shape.
  - **§5.6 `consistency_failure` marked as fail-safe-only** (codex consult):
    `consistency_failure` is not verified as an emittable runtime code in
    the current Rust surface. V1.1.1 keeps it as a renderer fail-safe
    bucket (for any future code that maps to consistency) rather than
    claiming it's an active runtime code. Removed from fixture #14's
    code-matrix list.
  - **§5.6 AC #14 / fixture #14 sync** (codex consult): AC #14 listed
    `missing_expected_claim_version` and `mid_flight_mutation`; fixture
    #14's 11-code matrix omitted both. V1.1.1 adds them to fixture #14's
    enumeration. Now AC #14 and fixture #14 cover the same 13-code set
    (11 from V1.1 + 2 omitted).
  - **§5.6 session-repair arm extended** (code-reviewer F2 residual):
    extend switch's session-repair arm to include `identity_mismatch`,
    `wp_user_mismatch`, `pairing_code_invalid`, `pairing_code_expired`,
    `pairing_code_consumed`, `pairing_code_limited`, `pairing_suspended`,
    `pairing_revoked`, `pairing_expired`, `site_binding_mismatch`,
    `session_expired`, `session_throttled`, `restored_stale_pairing`,
    `scope_denied`, `auth_missing`. Plus a new "renderer-input-invalid"
    arm for `request_body_too_large`, `request_body_unreadable`,
    `handshake_body_invalid`, `session_refresh_body_invalid`,
    `surface_invoke_invalid`, `event_log_id_invalid`. Reasoning: V1.1's
    "unknown code → verification banner" fail-safe was functionally
    correct but mis-attributed pairing/identity failures to projection
    consistency. The extended arms route them to user-actionable notices.
    Verification banner stays reserved for true projection-consistency
    failures + unknown-code fail-safe.
  - **§8 new fixture #17 — AC #16 coverage** (code-reviewer F8): Rust
    test `dos672_authorize_local_render_enforces_scope` — when validated
    session lacks `read.account_overview` scope, `authorize_local_render`
    rejects with `ScopeDenied`, proving `charge_ability_scope=false` only
    bypasses rate-budget consumption, NOT authorization gates.
  - **§8 new fixture #18 — external composition-version invalidation**
    (codex challenge LOW): Rust test that seeds cache at version N,
    advances current DB composition_version outside the request watermark
    (e.g., via an external producer path), renders with stale
    `request.composition_version=N`, asserts lookup misses old key,
    producer path runs, cache stored at the emitted version, subsequent
    render hits. Concrete proof of the §5.4 reframe's invalidation claim.
  - **§8.1 AC↔fixture mapping table added** (codex consult Validation 6 +
    code-reviewer): the V1.1 "16 ACs ↔ 16 fixtures, 1:1" claim was
    overstated — several ACs map to CI invariants or are covered by
    multiple fixtures. V1.1.1 adds an explicit mapping table (Packet A
    V1.1.1 §8.1 pattern). After fixture #17 + #18 additions, count is
    18 fixtures covering 16 ACs + 2 CI-only invariants.
  - **§9 whitespace tolerance note** (code-reviewer F7): invariants #2/#3
    grep gates must tolerate whitespace inside dep array literals (e.g.,
    `[ x, y ]` vs `[x, y]`). Spec: collapse whitespace before regex
    match, or use a normalized-AST check via simple JS parse. L1
    implementer detail; documented here so the gate doesn't become a
    brittle literal-substring grep.

- **V1.1 (2026-05-17, cycle-1 fold + key reframing):** Cycle-1 verdicts:
  codex challenge BLOCK, CSO CA, code-reviewer CA, codex consult CA. The
  BLOCK was justified — V1.0 misdiagnosed the §5.4 fix. Key insight from
  codex consult HIGH #2: **`commit_composition` does not persist a reusable
  composition payload; it only updates `composition_versions` row + emits a
  version event.** Removing producer commit-on-render leaves nothing in the
  DB to project from on cache miss. The producer commit IS the
  materialization step — it cannot be removed without inventing persistent
  projection storage.

  The actual local fix: **make the cache key effective (option a) so cache
  HITS dominate**. Producer runs only on true cache miss; under steady-state
  rendering, producer runs rarely; commit_composition fires rarely; no write
  flood. The render-loop symptom comes from the cache key using the surface's
  stale watermark (`request.composition_version`), which guarantees a cache
  miss on every render and re-invokes the producer every time.

  V1.1 trims §5.4 from "remove producer commit" to "fix the cache key so
  producer runs only when needed". The architecturally contested removal is
  withdrawn; the substrate's W4-F producer contract is unchanged. CSO + codex
  challenge cycle-1 concerns about signal-propagation invalidation become
  moot — signal-driven producer invocations still commit and naturally
  populate the cache for the next render.

  - **FOLDED (real local issues):**
    - **§5.4 reframed** — producer commit STAYS on cache miss; the fix is
      cache key correction (§5.3), not commit removal. The §5.4 "remove
      render-path producer-commit" architecturally contested change is
      withdrawn. The W4-F producer contract at `mod.rs:2348-2353` is left
      intact; the comment's claim that "the producer always advances
      composition_version" remains true (it just runs less often now).
    - **§5.3 picks option (a)** — cache key `(actor, composition_id,
      current_db_version, scopes_canonical_id)`. Codex consult cycle-1
      recommended (a) on implementation grounds (option (b) requires a new
      invalidation API on the orchestrator that doesn't exist). Move
      `current_composition_version_for_composition_id` read BEFORE cache
      lookup. Trade-off: +1 `db_read` per warm-path render — acceptable for
      local-to-local (code-reviewer F5). Store cache by
      `projection.composition_version.unwrap_or(current_db_version)`.
    - **§5.2 stale-closure fix** — keep `reload` callback's full dep list
      (`[composition_id, composition_version, cache_hint_token,
      setAttributes]` — preserves manual-reload correctness per codex
      challenge HIGH #2 + codex consult MEDIUM). Add a separate effect
      trigger:
      ```js
      const reloadTrigger = `${attributes.account_id}|${attributes.composition_id ? '1' : '0'}`;
      useEffect(() => { reload(); }, [reloadTrigger]);
      ```
      The trigger key changes only when account_id or composition_id-presence
      changes — NOT when version+token change. Manual button keeps using the
      live `reload` closure (latest deps). No stale-closure risk; no
      auto-reload loop.
    - **§5.2 Gutenberg lifecycle enumeration** (code-reviewer F4) —
      expected-fire: initial mount, account_id change, composition_id
      empty→non-empty transition. Expected-not-fire: window focus, autosave,
      undo of cache_hint_token write, block-list reorder remount (which
      fires the effect with current attributes once — benign). Add positive
      fixture "first-mount fires exactly one reload" alongside the negative
      fixtures.
    - **§5.1 wrapper preservation** (code-reviewer F1) — `dailyos_account_overview_render`
      retains its single-fetch contract for the block.json front-end render
      path (`wp/dailyos/blocks/account-overview/render.php:20-25`) and the 6
      test fixtures (`tests/blocks/AccountOverviewBlockTest.php` at lines 52,
      61, 96, 131, 169, 189). The new pure helper
      `dailyos_account_overview_render_from_projection($response, $attributes)`
      is called ONLY by `account_overview_preview`, replacing the
      `render_block_with_filter` re-entry that caused the double-fetch.
    - **§5.6 error envelope rewrite** (code-reviewer F2 + codex consult
      MEDIUM) — runtime returns `{"error":{"code","message","request_id",...}}`
      NOT top-level `{ok:false, code:...}`. PHP transport
      (`class-dailyos-runtime-client.php:393-509`) re-wraps non-2xx as
      `{ok:false, error:{code, message}}`. Renderer's mapping table reworked
      against this two-channel surface (WP_Error from transport-layer failure
      vs runtime envelope from non-2xx). Match `error.code` by string —
      `session_requires_repair` is a `SurfacePairingError` variant
      (`surface_pairing.rs:277`) surfaced via `from_pairing_error`, not a
      dedicated `SurfaceHttpError` constructor.
    - **§5.5 narrow to `charge_ability_scope=false`** (codex consult MEDIUM
      + code-reviewer F3 + CSO LOW) — add an `authorize_local_render`
      variant of `authorize` that passes `charge_ability_scope=false` to
      `check_and_consume`. Identity buckets (surface_client, wp_user,
      wp_site) continue to be consumed; ability bucket
      (`standard_read_composition`) and scope bucket (`scope.read`) are
      bypassed. Authorization (descriptor, actor, mode, scope check,
      browser-direct-executable guard) remains mandatory. AC explicitly
      states "local-render decharge omits the
      `RateLimitOutcome::Allowed.audit_events` tighten-event emission by
      construction" (code-reviewer F3 acceptance).
    - **§8 JS fixtures → PHP equivalents** (code-reviewer F6) — switch
      fixtures #2, #3, #8, #10 to PHP integration tests against the existing
      fake-runtime-client pattern at `tests/blocks/AccountOverviewBlockTest.php`.
      Assertions: request count to runtime, payload shape, post-render HTML
      shape. Avoids adding wp-scripts jest setup as net-new infrastructure.
    - **§9 grep gates instead of ESLint rules** (code-reviewer F7) —
      replace ESLint rule invariants #2 and #3 with grep gates on
      `edit.js` (literal dep array shape + literal trigger key shape).
      ESLint authoring filed to maintenance.
    - **§5.4 framing narrowed** (code-reviewer F9) — the contested anchor
      at `mod.rs:2348-2353` is a DOS-670 producer-side OCC workaround, NOT
      a v1.4.2 W4-F V3.2 contract. The reframed V1.1 §5.4 (cache fix, not
      commit removal) doesn't contest either; the substrate keeps its
      always-forward-version behavior, just running less often. Removed the
      "L6 trigger" prose from §14 since the §5.4 reframe eliminates the
      contested decision.
  - **DEFERRED to v1.x-federation maintenance:**
    - **Signal-propagation cache invalidation bus** (CSO MEDIUM + codex
      challenge MEDIUM cycle-1): moot in V1.1 because §5.4 reframed keeps
      producer commits as the invalidation channel. Federation/multi-writer
      scenarios where producer commits can occur from non-local sources
      would benefit from an explicit invalidation API; v1.4.3 local-single-
      runtime doesn't need it. **Maintenance ticket title:** "Signal-driven
      cache invalidation API for composition_render_orchestrator (federation
      scope)".
    - **Hostile co-resident plugin error-code fingerprinting** (codex
      challenge LOW cycle-1): the WP plugin is a trusted same-UID surface
      principal in v1.4.3 local-to-local. A hostile co-resident plugin with
      PHP execution already has ambient keychain/DB/loopback access. Finer
      error-code redaction would be remote-shape defense. Filed as
      maintenance ticket "Bounded error taxonomy for multi-tenant /
      multi-plugin WordPress deployments".
    - **Render-volume audit signal** (code-reviewer F3 maintenance note):
      operator observability for local-render-decharge volume tracking
      isn't security-load-bearing locally. Filed as maintenance ticket
      "Local-render audit signal for operator observability".
    - **ESLint rule authoring** for editor `useEffect` dep arrays
      (code-reviewer F7): grep gates cover L0 closure; proper ESLint rule
      is maintenance.

  - **REJECTED (not findings against V1.1):**
    - Codex challenge cycle-1 HIGH #1 "no replacement materialization
      path" — moot because V1.1 doesn't remove the materialization path.
    - Codex challenge cycle-1 MEDIUM "cache option (b) needs invalidation
      bus" — moot because V1.1 picks option (a).
    - V1.0 §12 Q1, Q2, Q3 — all resolved by the V1.1 reframe / picks.

  - **NET V1.1 RESULT:** packet acceptance criteria count: 16 (was 15;
    +1 for the decharge tighten-event acceptance, +1 for first-mount fixture,
    -1 because AC #15 "merge no longer combines" becomes tautological after
    §5.1). Fixtures: 14 (was 13; +1 first-mount-fires-once,
    JS-to-PHP equivalents add no count change). §5.4 collapses from 5
    architecturally contested sub-bullets to "preserved unchanged from W4-F";
    the §5.4 architectural-contest section is replaced with "no architectural
    contest". The CORE remains: single-fetch PHP, editor reload guard,
    effective cache key, decharge for local render, typed error mapping. That
    CORE is the right local fix for the user-visible render-loop bug.

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

### 5.1 Single-fetch PHP preview (DOS-671 + DOS-672) — V1.1 wrapper preservation

V1.0's framing made the wrapper "optional" — code-reviewer F1 caught this is
wrong. `dailyos_account_overview_render` has TWO production callers (not
just the preview route):
1. `wp/dailyos/blocks/account-overview/render.php:20-25` — block.json
   front-end render path. This MUST keep the single-fetch behavior (one
   runtime call per render request).
2. `wp/dailyos/includes/class-dailyos-plugin.php:587-612` — preview REST
   route. THIS is where the double-fetch happens (preview-endpoint calls
   runtime, then re-enters `dailyos_account_overview_render` via
   `render_block_with_filter` at `:682-690`, causing a second runtime call).

Plus 6 test fixtures at `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php`
(lines 52, 61, 96, 131, 169, 189) that call `dailyos_account_overview_render`
directly and rely on the single-fetch wrapper behavior.

**V1.1 refactor preserves the wrapper, adds a pure helper:**

```php
function dailyos_account_overview_render( array $attributes ): string {
    // Existing wrapper — unchanged behavior for render.php + 6 test fixtures.
    // Validates composition_id, fetches runtime client, calls
    // project_composition_for_surface once, delegates to
    // dailyos_account_overview_render_from_projection.
    $response = dailyos_account_overview_fetch_projection( $attributes );
    return dailyos_account_overview_render_from_projection( $response, $attributes );
}

function dailyos_account_overview_render_from_projection( $response, $attributes ): string {
    // Pure projection-to-HTML — no runtime call.
    // Accepts both runtime success envelope (top-level `projection`,
    // `cache_hint_token`, `served_from_cache`) AND WP transport error
    // envelope (`{ok:false, error:{code,message}}` from class-dailyos-
    // runtime-client.php:393-509) and WP_Error from transport-layer
    // failure.
}
```

`account_overview_preview` (at `class-dailyos-plugin.php:587-612`) is the
ONLY caller that switches: it calls `project_composition_for_surface` once,
then calls `dailyos_account_overview_render_from_projection` directly with
the response (NOT `render_block_with_filter` → `dailyos_account_overview_render`).
One preview = one runtime call.

The existing test fixtures at `AccountOverviewBlockTest.php` continue to
call `dailyos_account_overview_render($attributes)` unchanged. They keep
passing because the wrapper's behavior is unchanged for them.

### 5.2 Editor reload guard (DOS-671 + DOS-672, shared) — V1.1 stale-closure-safe

V1.0's approach (remove `composition_version` and `cache_hint_token` from
the `reload` callback's dep list) was unsafe — `reload` reads those values
when building the POST body (`edit.js:54-56`), so a manual reload after a
successful render would send STALE version/token (codex challenge HIGH #2 +
codex consult MEDIUM).

**V1.1 keeps `reload`'s full dep list AND adds a separate trigger key for
the useEffect:**

```js
const reload = useCallback(() => {
    // ... existing body, reads composition_id, composition_version, cache_hint_token
}, [attributes.composition_id, attributes.composition_version, attributes.cache_hint_token, setAttributes]);

const reloadTrigger = `${attributes.account_id || ''}|${attributes.composition_id ? '1' : '0'}`;
useEffect(() => {
    reload();
}, [reloadTrigger]);
```

- `reloadTrigger` is a derived string that changes ONLY when `account_id`
  changes OR `composition_id` transitions empty↔non-empty.
- Successful reload's `setAttributes` write of `composition_version` /
  `watermarks` / `cache_hint_token` does NOT change `reloadTrigger` → no
  auto-reload retrigger.
- Manual button continues to use the live `reload` closure (always has
  latest deps including version+token) — no stale POST body.

**Gutenberg lifecycle enumeration** (code-reviewer F4):
- **Expected to fire:** initial mount (React effect fires on first render
  with current attributes); `account_id` change via account selector;
  `composition_id` empty→non-empty transition (i.e., user picks an account
  for the first time).
- **Expected NOT to fire:** window focus (no remount, dep value unchanged);
  autosave (no edit.js dep change); undo/redo that only flips
  `cache_hint_token` or `composition_version` (those aren't in the trigger
  key); block-list reorder remount (effect fires once on remount with the
  current `reloadTrigger`, which equals the pre-reorder value — benign
  no-op if `account_id` and `composition_id` are unchanged).

**Failed-reload behavior:**
- Preserve last-good `preview` state.
- Show an `error` notice in the editor but do NOT replace rendered content
  with the verification banner envelope.

**ESLint suppression note (V1.1.1 per codex consult):** WP-side
`wp-scripts lint-js` may emit `react-hooks/exhaustive-deps` warnings on
`useEffect([reloadTrigger])` because `reload` is reachable from the body
but not in the dep list. Two L1 implementer options, both architecturally
correct:
- Explicit ESLint disable comment with rationale (preferred for clarity):
  ```js
  // eslint-disable-next-line react-hooks/exhaustive-deps -- intentional:
  // reload's identity is recomputed every render (full dep list); the
  // auto-reload trigger gate is the derived reloadTrigger string. See
  // L0 Packet B V1.1 §5.2.
  useEffect(() => { reload(); }, [reloadTrigger]);
  ```
- OR set `react-hooks/exhaustive-deps` to `warn` not `error` at the WP
  project level (root project already does this at `eslint.config.js:74`).

### 5.3 Runtime cache-key correction — option (a) picked (V1.1)

At `src-tauri/src/surface_runtime/mod.rs:2317-2321` and `:2425-2430`, the cache
is keyed on the request's `composition_version` (which is the surface's stale
watermark — every render misses the cache, forcing producer invocation).

**V1.1 picks option (a)** — cache key by current DB version. Codex consult
cycle-1 recommended this on implementation grounds (option (b) requires a
new orchestrator invalidation API that doesn't exist; option (a) uses the
existing `current_composition_version_for_composition_id` reader).

**Concrete implementation:**

1. Move the existing `current_composition_version_for_composition_id` read
   (`mod.rs:2355-2370`) from "after cache miss, before producer invocation"
   to "BEFORE cache lookup". This adds one `db_read` per warm-path render —
   accepted trade-off for local-to-local (code-reviewer F5).

2. Cache lookup key becomes `(actor, composition_id, current_db_version,
   scopes_canonical_id)` (the orchestrator's existing key shape with
   `composition_version` replaced from `request.composition_version` to
   `current_db_version`). Lookup at `mod.rs:2317-2321`.

3. Cache store key uses
   `projection.composition_version.unwrap_or(current_db_version)`. Store
   at `mod.rs:2425-2430`. `ProjectedComposition.composition_version` is
   defined at `abilities-runtime/src/abilities/fallback_projection.rs:29-35`
   and is set after producer invocation.

4. The orchestrator's existing `(composition_id, composition_version,
   scopes_canonical_id)` key tuple at
   `composition_render_orchestrator.rs:51-56` is preserved — only the
   value passed for `composition_version` changes.

**Trade-off analysis** (code-reviewer F5):
- V1.0 / current: warm-path is "zero DB-touch, cache hit" but ALSO "cache miss every time because key uses stale watermark".
- V1.1 / option (a): warm-path is "one `db_read`, then cache hit". Net win: producer commits stop firing on every render.
- Option (b) would preserve zero-DB-touch on warm path BUT requires net-new invalidation infrastructure (CSO recommended this; codex consult rejected on implementation cost). Deferred to v1.x-federation maintenance.

**Race analysis:** between the new `current_db_version` read and the
cache lookup, a concurrent commit could occur. For local single-runtime,
producer invocations are serialized by the SurfaceClient bridge's writer
mutex; there is effectively no concurrent commit race. For
federation/multi-writer scenarios, this would need the invalidation API
from option (b) — out of scope for v1.4.3.

**"State movement" definition** (added V1.1.1 per codex challenge cycle-2
LOW): "state movement" in this packet means `composition_versions` row
advancement through `commit_composition`. Upstream claim/source movement
(claim version, source freshness, account_subject signals) reaches this
cache through the existing DOS-589 / W4-A0 recomposition contract:
upstream signal → producer re-invocation by some path → producer commits
new composition version → next render's `current_db_version` read returns
the new value → cache lookup misses old key → producer runs (this time
from the actual render path) → cache populated at new version → subsequent
renders hit. The cache invalidation proof is exact for committed
composition-version movement; pre-commit upstream movement is invisible to
this cache by design (and that's correct — without a committed projection
to serve, there's nothing to invalidate or refresh).

### 5.4 Render-path producer behavior — V1.1 reframe: producer commits preserved

**V1.0 proposed removing producer commit from the render path.** V1.1
withdraws that proposal. The reframe rationale is in §2 changelog V1.1:
- `commit_composition` does not persist a reusable projection payload (only
  `composition_versions` row + version event — `services/compositions.rs:271-368`).
- Removing producer commit-on-render leaves nothing in the DB to project from
  on cache miss.
- The producer commit IS the materialization step; it cannot be removed
  without inventing persistent projection storage (which would be substantial
  net-new infrastructure, federation-scale).

**The actual local fix is §5.3 — make the cache key effective.** Under V1.1:
- Producer runs ONLY on cache miss (unchanged from today).
- Cache misses become RARE once the cache key uses current_db_version
  instead of the surface's stale watermark.
- Under steady-state rendering with stable account state, all renders after
  the first hit the cache → producer doesn't run → commit_composition doesn't
  fire → no write flood.
- Producer commits naturally fire when account state actually moves (signal
  propagation triggers a new producer invocation through other paths, that
  invocation commits a new version, which invalidates the cache via the
  cache-key-by-current-db-version pattern).

**The W4-F producer contract at `mod.rs:2348-2353` is preserved.** The
comment's claim that "the producer always advances composition_version" stays
true. The DOS-670 substrate-side fix (always forward current DB version to
the producer) stays. Nothing about the producer's OCC contract changes.
There is no architectural contest; V1.0's framing was wrong.

**What V1.1 §5.4 actually contains:** nothing net-new. The previous V1.0
§5.4 "remove producer commit on render path" is withdrawn. The fix surface
is entirely §5.3 (cache key correction). This section exists in V1.1 only to
document the reframe rationale.

**Removed in V1.1 (from V1.0):**
- "Render path reads existing projected state from cache or DB" — DB doesn't have it; cache is the only read source, and cache-miss must invoke the producer.
- "Producer commit happens ONLY on explicit refresh / signal-propagation / initial creation" — withdrawn. Producer commit happens on cache miss, which becomes rare once §5.3 lands.
- The `§12 Q2` open question about "does removing render-path commit break the DOS-670 producer OCC contract" — moot; nothing is being removed.
- The "L6 escalation trigger if CSO and codex challenge disagree on §5.4" — moot; no contested change remains.

### 5.5 Render-read decharge — V1.1: option (a) with `charge_ability_scope=false`

V1.1 picks option (a) (codex consult cycle-1 implementation guidance):
local-render reads use a new `authorize_local_render` variant that calls
`check_and_consume` with `charge_ability_scope=false`. Identity buckets
(`surface_client`, `wp_user`, `wp_site`) continue to be consumed; ability
bucket (`standard_read_composition`) and scope bucket (`scope.read`) are
bypassed for paired-loopback local render reads.

**Mandatory checks preserved** (codex consult evidence — `surface_client.rs:301-355`):
- Descriptor exists (ability is registered).
- Actor / mode / experimental gates.
- Required scopes (the actor must have read.account_overview etc).
- Browser-direct-executable guard.

**Decharge contract — bypass scope:**
- `charge_ability_scope=false` (codex consult evidence — `surface_client.rs:409`,
  `:795-822`) skips the ability_class and scope candidates inside
  `check_and_consume`.
- Identity candidates (surface_client/wp_user/wp_site read budgets at
  `:766-793`) are STILL consumed. These remain as a per-principal limit on
  total local-render volume — DoS protection without blocking normal usage.

**Acceptance: tighten-event emission omitted by construction** (code-reviewer
F3 + CSO LOW):
`RateLimitOutcome::Allowed.audit_events` carries `rate_limit_audit_event(...,
"tightened", ...)` when a previously-rejected window enters early-retry
tightening (`surface_client.rs:411-415`, `:742-752`). When ability/scope
candidates are bypassed, no consumption happens for those candidates → no
rejection possible for them → no tightening → no tightened-event emitted.
This is internally consistent for local-to-local. V1.1 codifies this with
AC #15 ("local-render decharge emits zero `rate_limit_audit_event` entries
for the ability/scope bucket"). Maintenance ticket filed for operator
observability of local-render volume.

**Implementation shape (V1.1.1 — corrected type-shape per codex consult):**

The existing substrate uses a **boolean** `charge_ability_scope` field
(`surface_client.rs:642`), not an enum. V1.1's pseudo-code naming
`ChargeAbilityScope::Off` was misleading; corrected:

```rust
// New variant — same descriptor/actor/mode/scope checks, then bypass-ability-scope rate-limit
pub fn authorize_local_render(
    &self,
    registry: &AbilityRegistry,
    validated: &SignedSessionContext,
    ability_name: &str,
    request_id: &str,
) -> Result<Authorization, SurfaceClientBridgeError> {
    self.authorize_for_path(
        registry,
        validated,
        ability_name,
        request_id,
        false,  // charge_ability_scope — today hardcoded to true at :387/:409
    )
}
```

**Precedent:** there is already an identity-only helper at
`surface_client.rs:936` that constructs requests with
`charge_ability_scope=false`. V1.1.1's `authorize_local_render` follows
the same pattern.

Route handler at `surface_runtime/mod.rs:2288-2311` calls
`authorize_local_render` instead of `authorize` for the `project_composition`
read path. Other surface client routes continue to use `authorize`.

### 5.6 Typed transport/session error mapping (DOS-672) — V1.1 envelope rewrite

V1.0's error envelope shape was wrong. Per code-reviewer F2 + codex consult
MEDIUM:
- Runtime returns `{"error":{"code","message","request_id","remediation",...}}` (NOT top-level `{ok:false, code:...}`). HTTP status conveys success/failure. Defined at `src-tauri/src/surface_runtime/mod.rs:3514-3548`.
- WP transport (`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:393-509`) re-wraps non-2xx responses as `{ok:false, error:{code, message}}`.
- Local transport failures (signing failure before request, `wp_remote_post` failure, JSON parse) produce `WP_Error` OR `{ok:false, error:{code, message}}` with codes like `runtime_request_failed`, `runtime_invalid_json`, `runtime_http_error`.

`session_requires_repair` is a `SurfacePairingError` variant
(`surface_pairing.rs:277`) surfaced via `from_pairing_error`; there is NO
dedicated `SurfaceHttpError::session_requires_repair()` constructor. Match
by string on `error.code`.

**V1.1 renderer mapping (replaces `render-functions.php:61-70` blanket banner):**

```php
// In dailyos_account_overview_render_from_projection(...)
if ( is_wp_error( $response ) ) {
    // Transport-layer failure (signing, network). No runtime envelope.
    return dailyos_account_overview_render_runtime_unavailable_notice();
}

if ( isset( $response['ok'] ) && $response['ok'] === false ) {
    $code = isset( $response['error']['code'] ) ? (string) $response['error']['code'] : 'runtime_request_failed';
    switch ( $code ) {
        case 'rate_limited':
            return dailyos_account_overview_render_throttled_notice();
        // Session-repair-shaped: user action needed to restore pairing/session.
        case 'session_requires_repair':
        case 'session_not_found':
        case 'session_expired':
        case 'session_throttled':
        case 'identity_mismatch':
        case 'wp_user_mismatch':
        case 'pairing_code_invalid':
        case 'pairing_code_expired':
        case 'pairing_code_consumed':
        case 'pairing_code_limited':
        case 'pairing_suspended':
        case 'pairing_revoked':
        case 'pairing_expired':
        case 'site_binding_mismatch':
        case 'restored_stale_pairing':
        case 'scope_denied':
        case 'auth_missing':
            return dailyos_account_overview_render_session_repair_notice();
        // Runtime-unavailable: transient infrastructure problem; retry.
        case 'runtime_unavailable':
        case 'runtime_request_failed':
        case 'runtime_invalid_json':
        case 'runtime_http_error':
        case 'host_invalid':
        case 'browser_origin_forbidden':
        case 'route_not_found':
            return dailyos_account_overview_render_runtime_unavailable_notice();
        // Renderer-input-invalid: defensive — the renderer or its caller
        // produced something the runtime can't process. Operator
        // notice; not a user-actionable retry.
        case 'request_body_too_large':
        case 'request_body_unreadable':
        case 'handshake_body_invalid':
        case 'session_refresh_body_invalid':
        case 'surface_invoke_invalid':
        case 'event_log_id_invalid':
            return dailyos_account_overview_render_invalid_request_notice();
        // Projection-consistency failures — verification banner correct.
        case 'projection_tampered':
        case 'projection_version_rollback':
        case 'stale_composition_watermark':
        case 'missing_expected_claim_version':
        case 'mid_flight_mutation':
            return dailyos_account_overview_render_verification_banner();
        default:
            // Unknown code: fail-safe to verification banner. Operator
            // should add a typed mapping when a new code appears.
            return dailyos_account_overview_render_verification_banner();
    }
}

if ( ! isset( $response['projection'] ) || ! is_array( $response['projection'] ) ) {
    // Successful response shape without projection — defensive fallback.
    return dailyos_account_overview_render_verification_banner();
}

// Success path — render the projection.
return dailyos_account_overview_render_projection_payload( $response, $attributes );
```

New helpers (small, local copy):
- `dailyos_account_overview_render_throttled_notice()` — "Runtime is throttling; retry shortly."
- `dailyos_account_overview_render_session_repair_notice()` — "Surface session needs repair; reconnect from DailyOS settings."
- `dailyos_account_overview_render_runtime_unavailable_notice()` — "Runtime unavailable; retry."
- `dailyos_account_overview_render_invalid_request_notice()` — "Editor sent a request the runtime couldn't process. Reload the editor."

The verification banner stays reserved for genuine projection-consistency
failures + unknown codes (fail-safe).

**Note on `consistency_failure` (V1.1.1 per codex consult):**
`consistency_failure` is not verified as an actively emitted runtime code
in the current Rust surface. The 5 specific consistency codes in the
verification-banner switch arm (`projection_tampered`,
`projection_version_rollback`, `stale_composition_watermark`,
`missing_expected_claim_version`, `mid_flight_mutation`) ARE emittable
(`surface_runtime/mod.rs:3010-3049`). The renderer's fail-safe default
arm catches any new consistency code added later — including a future
`consistency_failure` if introduced.

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

### 6.4 V1.1: producer-commit-on-render is PRESERVED; the fix is cache effectiveness

V1.0 framed this as "the contested change" requiring L6 escalation. V1.1
withdraws the contest entirely. Codex consult cycle-1 surfaced the key fact:
`commit_composition` does not persist a reusable projection payload, so
removing producer-commit-on-render breaks materialization. The producer
commit IS the materialization step.

**Decision (V1.1):** producer commit on cache miss stays unchanged. The
render-loop user-visible bug is fixed by §5.3 cache key correction, which
causes cache hits to dominate → producer rarely runs → commit rarely fires.
The W4-F producer contract at `mod.rs:2348-2353` is preserved. No
architectural contest with v1.4.2 / W4-F remains. The DOS-670 producer-side
OCC workaround is intact.

**§12 Q2 RESOLVED** — moot; nothing is being removed.

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

1. PHP preview makes exactly one `project_composition_for_surface` runtime call per preview request. `dailyos_account_overview_render` (the wrapper) keeps its single-fetch behavior for `render.php` + existing test fixtures.
2. Editor `reload` callback's dep list keeps `[composition_id, composition_version, cache_hint_token, setAttributes]` — manual reload reads latest version+token, NO stale-closure risk.
3. Editor `useEffect`-driven auto-reload fires ONLY when a separate derived `reloadTrigger` string changes — i.e., on `account_id` change OR `composition_id` empty↔non-empty transition. NOT when `composition_version` or `cache_hint_token` change.
4. Successful reload's attribute write does NOT schedule another reload (the `reloadTrigger` value is unchanged).
5. First-mount fires exactly ONE reload (initial materialization).
6. Runtime `project_composition` cache lookup uses key `(actor, composition_id, current_db_version, scopes_canonical_id)` (NOT `request.composition_version`). Stale watermarks no longer cause cache misses.
7. Runtime cache store uses `projection.composition_version.unwrap_or(current_db_version)` so the stored entry's version matches what the producer actually emitted.
8. After initial render, the block remains visible for ≥60 seconds without further user action AND across two browser-window focus changes (the L4 user-visible target).

### DOS-672 (reload-on-window-switch + second-reload verification banner)

9. Manual "Reload from runtime" fires exactly one preview request per click.
10. Window-focus change does NOT trigger reload (no remount, no auto-reload retrigger).
11. Second consecutive manual reload succeeds (returns projection, not banner) — assuming first reload succeeded.
12. Failed reload (any cause) preserves last-good `preview` state in the editor.
13. Failed reload surfaces an `error` notice; does NOT replace rendered content with verification-banner HTML.
14. PHP renderer maps `error.code` via switch table (V1.1.1 expanded coverage): `rate_limited` → throttled notice; session-repair-shaped codes (`session_requires_repair`, `session_not_found`, `session_expired`, `session_throttled`, `identity_mismatch`, `wp_user_mismatch`, `pairing_*`, `site_binding_mismatch`, `restored_stale_pairing`, `scope_denied`, `auth_missing`) → session-repair notice; runtime-unavailable codes (`runtime_unavailable`, `runtime_request_failed`, `runtime_invalid_json`, `runtime_http_error`, `host_invalid`, `browser_origin_forbidden`, `route_not_found`) → runtime-unavailable notice; renderer-input-invalid codes (`request_body_*`, `handshake_body_invalid`, `session_refresh_body_invalid`, `surface_invoke_invalid`, `event_log_id_invalid`) → invalid-request notice; projection-consistency codes (`projection_tampered`, `projection_version_rollback`, `stale_composition_watermark`, `missing_expected_claim_version`, `mid_flight_mutation`) → verification banner; unknown codes → verification banner (fail-safe).
15. **Local-render decharge by construction:** `authorize_local_render` calls `check_and_consume` with `charge_ability_scope=false`. Ability/scope buckets (`standard_read_composition`, `scope.read`) are NOT consumed; identity buckets (`surface_client`, `wp_user`, `wp_site`) ARE consumed. As a direct consequence, the `RateLimitOutcome::Allowed.audit_events` "tightened" emission for ability/scope is OMITTED — no observability event for that bucket because no consumption happens.
16. Auth/scope mandatory checks (descriptor, actor, mode, experimental, required scopes, browser-direct guard) are NOT bypassed; only rate-budget consumption for ability/scope changes.

## 8. Negative fixtures

All fixtures use PHP test infrastructure (existing
`tests/blocks/AccountOverviewBlockTest.php` pattern with fake-runtime-client
injection) for editor-side behavior assertions — no net-new jest setup (code-reviewer F6).

| # | Fixture | Asserts |
|---|---|---|
| 1 | `dos671_preview_single_fetch` | PHP test: preview REST route's fake runtime client receives exactly ONE `project_composition_for_surface` call per preview request (not two as today via `render_block_with_filter` re-entry) |
| 2 | `dos671_wrapper_preserves_single_fetch` | PHP test: `dailyos_account_overview_render($attrs)` called by `render.php` triggers exactly one runtime call (regression guard for the wrapper) |
| 3 | `dos671_editor_dep_array_literal` | PHP/grep gate: `edit.js` `reload` useCallback dep array literal contains `[attributes.composition_id, attributes.composition_version, attributes.cache_hint_token, setAttributes]` (manual reload reads latest) |
| 4 | `dos671_editor_trigger_key_literal` | PHP/grep gate: `edit.js` defines `reloadTrigger` and `useEffect([reloadTrigger])` shape — trigger derived from account_id + composition_id-presence ONLY |
| 5 | `dos671_first_mount_fires_one_reload` | PHP integration test simulating editor mount: exactly ONE reload request issued (initial materialization) |
| 6 | `dos671_cache_key_uses_current_db_version` | Rust test: lookup with stale `request.composition_version=1` HITS cache when current_db_version=5 and prior render populated cache at version 5 |
| 7 | `dos671_cache_hit_avoids_producer` | Rust test: cache hit returns projection WITHOUT invoking producer (verified by producer-invocation counter) |
| 8 | `dos671_cache_miss_invokes_producer_and_commits` | Rust test: cache miss DOES invoke producer; producer commits a new version; cache is populated with the new version's projection (the materialization path is preserved) |
| 9 | `dos671_local_render_no_ability_scope_consumption` | Rust test: tight `standard_read_composition` budget (1/min); 50 paired-loopback render reads do NOT fail with `rate_limited`; the ability_class bucket counter shows zero consumption |
| 10 | `dos671_local_render_consumes_identity_buckets` | Rust test: paired-loopback render reads DO consume `surface_client_read` bucket (identity bucket still gated) |
| 11 | `dos671_local_render_no_tighten_event` | Rust test: 100 paired-loopback render reads produce ZERO `rate_limit_audit_event` entries (ability/scope bucket bypassed → no rejection → no tightening) |
| 12 | `dos672_manual_reload_single_request` | PHP integration test: simulated button click via REST → exactly one runtime call (not two) |
| 13 | `dos672_failed_reload_preserves_last_good` | PHP integration test: first reload succeeds → preview HTML rendered; second reload fails (mocked error) → response body returns error notice + last-good preview HTML unchanged in the response shape |
| 14 | `dos672_typed_error_mapping` | PHP test matrix (V1.1.1 expanded): per-arm grouping — throttled (`rate_limited`); session-repair (`session_requires_repair`, `session_not_found`, `session_expired`, `session_throttled`, `identity_mismatch`, `wp_user_mismatch`, `pairing_code_invalid`, `pairing_code_expired`, `pairing_code_consumed`, `pairing_code_limited`, `pairing_suspended`, `pairing_revoked`, `pairing_expired`, `site_binding_mismatch`, `restored_stale_pairing`, `scope_denied`, `auth_missing`); runtime-unavailable (`runtime_unavailable`, `runtime_request_failed`, `runtime_invalid_json`, `runtime_http_error`, `host_invalid`, `browser_origin_forbidden`, `route_not_found`); invalid-request (`request_body_too_large`, `request_body_unreadable`, `handshake_body_invalid`, `session_refresh_body_invalid`, `surface_invoke_invalid`, `event_log_id_invalid`); verification banner (`projection_tampered`, `projection_version_rollback`, `stale_composition_watermark`, `missing_expected_claim_version`, `mid_flight_mutation`). Each emittable code → expected user-facing string per §5.6 table. |
| 15 | `dos672_unknown_code_failsafe` | PHP test: runtime response `{ok:false, error:{code:'unknown_xyz'}}` → verification banner renders (fail-safe) |
| 16 | `dos671_l4_hands_on_log` | Hands-on log captured: initial render → wait 60s → focus switch x2 → manual reload x2; content remains visible throughout (the user-visible L4 target) |
| 17 | `dos672_authorize_local_render_enforces_scope` | Rust test (added V1.1.1 for AC #16): when validated session lacks `read.account_overview` scope, `authorize_local_render` rejects with `ScopeDenied`; proves `charge_ability_scope=false` only bypasses rate-budget consumption, NOT authorization gates. Negative-control fixture against the §5.5 decharge. |
| 18 | `dos671_external_version_advance_invalidates_cache` | Rust test (added V1.1.1 per codex challenge LOW): seed cache at version N for composition_id X; advance current_db_version for X outside the render request (e.g., external producer commit via a different ability path); render with `request.composition_version=N` (stale watermark); assert (a) cache lookup misses the V1.1 key `(actor, X, N+, scopes)` because current_db_version is now N+; (b) producer path runs and commits the projection at version N+; (c) cache is stored at N+; (d) immediately-subsequent render hits the cache. Proof of the §5.4 reframe's invalidation claim end-to-end.

### 8.1 AC → fixture / invariant mapping (added V1.1.1 per codex consult + code-reviewer)

V1.1's "16 ACs ↔ 16 fixtures, 1:1" claim was overstated. Explicit mapping:

| AC | Coverage |
|---|---|
| AC #1 (single-fetch — wrapper + preview) | Fixtures #1 (preview single-fetch) + #2 (wrapper preserves single-fetch). Two-fixture coverage because the AC has two clauses. |
| AC #2 (`reload` callback dep list) | §9 CI invariant #2 (grep gate) — not a runtime fixture |
| AC #3 (useEffect trigger key) | §9 CI invariant #3 (grep gate) |
| AC #4 (success no-retrigger) | Inferred from fixture #5 (first-mount fires ONE reload + V1.1.1 ESLint suppression note ensures Gutenberg attribute writes don't refire). For explicit assertion, fixture #5 should also assert no second reload happens within N ms of the first success — fold into fixture #5 description for L1. |
| AC #5 (first-mount fires one reload) | Fixture #5 |
| AC #6 (cache lookup keyed by current_db_version) | Fixtures #6, #7, #8, #18 + §9 CI invariant #4 (grep gate) |
| AC #7 (cache store uses projection.composition_version) | Fixture #8 |
| AC #8 (60s visibility + 2 focus changes) | Fixture #16 (L4 hands-on) |
| AC #9 (manual reload single request) | Fixture #12 |
| AC #10 (window-focus no reload) | Covered in fixture #16 hands-on log. For explicit automated assertion, fold into a sub-step of fixture #5 or add as L1 hardening. |
| AC #11 (second manual reload succeeds) | Covered implicitly by fixture #12's repeatable-call shape + fixture #16 hands-on log |
| AC #12 (failed reload preserves last-good) | Fixture #13 (PHP integration test asserts response shape contains last-good HTML; the editor-side React state preservation is exercised via fixture #5's React harness if available, else proven in fixture #16 hands-on) |
| AC #13 (failed reload surfaces error notice) | Fixture #13 |
| AC #14 (typed error switch — full coverage) | Fixture #14 (V1.1.1-expanded code matrix) + fixture #15 (unknown-code fail-safe) |
| AC #15 (decharge omits tighten-event) | Fixtures #9 (no `rate_limited` failure) + #11 (zero `rate_limit_audit_event` entries) |
| AC #16 (auth/scope checks not bypassed) | Fixture #17 (V1.1.1 — `authorize_local_render` still rejects on scope denial) |

Total: 18 fixtures (was 16) covering 16 ACs + 4 CI invariants (§9 #2/#3/#4 — grep-gate enforcement of ACs that aren't runtime-testable).

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | `account_overview_preview` calls `dailyos_account_overview_render_from_projection` directly; does NOT route the response back through `render_block_with_filter` → `dailyos_account_overview_render` | grep gate on `account_overview_preview` body |
| 2 | Editor `reload` callback dep list keeps composition_id + composition_version + cache_hint_token + setAttributes (preserves manual-reload correctness) | grep gate on `edit.js` for the literal dep array shape (V1.1.1: regex MUST collapse whitespace inside brackets — current `edit.js:82` uses prettier-style spaces `[ x, y ]`. Implementer detail per code-reviewer F7 / cycle-2 note: use `\s*` between tokens, or normalize via a small AST parse step. Brittle literal-substring grep will fail.) |
| 3 | Editor `useEffect` for auto-reload depends on `[reloadTrigger]` (NOT `[reload]`) | grep gate on `edit.js` (same whitespace tolerance as invariant #2) |
| 4 | Runtime `project_composition` cache lookup is keyed by current_db_version (NOT request.composition_version) | grep gate on `mod.rs` cache_lookup call site — verify the version argument is the current_db_version variable |
| 5 | Cache hit rate ≥ 95% under workload that varies `request.composition_version` but holds `composition_id` constant | Rust integration test in existing `rust.yml` workflow |
| 6 | `authorize_local_render` uses `charge_ability_scope=false`; main `authorize` continues to use `true` | grep gate on `surface_client.rs` for both variants |
| 7 | PHP error mapping uses the typed switch table in §5.6; verification banner is emitted ONLY from the consistency-failure branch + unknown-code fail-safe | grep gate on `render-functions.php` for `dailyos_account_overview_render_verification_banner` call sites — verify all callers are inside the consistency-failure switch arm or the unknown-code default arm |

V1.0 invariant #4 ("Runtime `project_composition` handler does NOT call
`commit_composition` on the steady-state read path") REMOVED in V1.1: the
§5.4 reframe preserves producer-commit-on-cache-miss. The user-visible
write-flood reduction comes from §5.3 (cache hits dominate), measurable by
invariant #5 above.

## 10. Interlocks

DOS-671 and DOS-672 share every file in scope (`edit.js`,
`render-functions.php`, `class-dailyos-plugin.php`,
`class-dailyos-runtime-client.php`, `surface_runtime/mod.rs`,
`composition_render_orchestrator.rs`, `surface_client.rs`).

**Landing shape (V1.1):** single v1.4.3 stabilization-B PR with four commit groups:
1. PHP single-fetch refactor (§5.1) + typed error mapping (§5.6) — wraps the wrapper, adds the pure helper, adds the typed switch table.
2. Editor reload guard + last-good preserve (§5.2) — reloadTrigger pattern, full reload dep list, Gutenberg lifecycle handling.
3. Runtime cache-key correction (§5.3) — move current_db_version read upstream, switch lookup/store keys to current_db_version / projection.composition_version.
4. Render-read decharge (§5.5) — new `authorize_local_render` variant, route handler switches.

V1.0's commit group #4 "render-path producer-commit removal" REMOVED — the
§5.4 reframe withdraws the architecturally contested change.

Splitting this PR is NOT viable. Each fix in isolation produces false
confidence: fix the editor without the PHP double-fetch and 50% of reloads
still hang; fix PHP without the cache-key issue and the cache stays useless;
etc. The investigation §"Coordination — Recommended Landing Shape" calls
this out explicitly.

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
- **Signal-propagation cache invalidation bus.** Cycle-1 CSO MEDIUM +
  codex challenge MEDIUM — moot in V1.1 because §5.4 reframe preserves
  producer-commit-on-cache-miss as the invalidation channel. Federation /
  multi-writer deployments would benefit from an explicit invalidation API
  on `composition_render_orchestrator`. Filed as v1.x-federation maintenance.
- **Hostile co-resident WordPress plugin error-code fingerprinting.**
  Cycle-1 codex challenge LOW — under same-UID local model, a co-resident
  plugin with PHP execution already has ambient keychain/DB/loopback
  access; finer error-code redaction would be remote-shape defense. Filed
  as v1.x-federation maintenance ("Bounded error taxonomy for
  multi-tenant / multi-plugin WordPress deployments").
- **Render-volume audit signal for operator observability.** Cycle-1
  code-reviewer F3 maintenance note — local-render-decharge volume
  observability isn't security-load-bearing locally. Filed as v1.x
  maintenance ("Local-render audit signal for operator observability").
- **ESLint rule authoring for editor `useEffect` dep arrays.** Cycle-1
  code-reviewer F7 — grep gates cover L0 closure; a proper ESLint rule is
  maintenance.
- **Persistent projection storage** (would enable producer-commit removal —
  not v1.4.3 scope). Filed as v1.x-federation maintenance — substantial
  new infrastructure, only beneficial when render-path mutation needs to
  be strictly eliminated (federation / multi-writer).
- **Hot-path performance budgeting** — cache hit rate ≥95% is the L0
  invariant target; actual latency budgeting (warm-path p95, cold-path
  p95) is operational tuning, not L0 scope.

## 12. Open questions for L0 reviewers — all V1.0 questions RESOLVED in V1.1

1. **V1.0 Q1 (cache key shape):** RESOLVED — V1.1 picks option (a), key by `current_db_version`. Codex consult cycle-1 recommended on implementation grounds; CSO recommended (b) but that requires net-new invalidation API filed to maintenance.
2. **V1.0 Q2 (producer-commit removal vs DOS-670):** RESOLVED — moot. V1.1 withdraws the proposed removal. Producer commit stays on cache miss; the user-visible write reduction comes from §5.3 cache effectiveness.
3. **V1.0 Q3 (render-read decharge shape):** RESOLVED — V1.1 picks option (a) with `charge_ability_scope=false`. Identity buckets still charge; ability/scope buckets bypassed.
4. **V1.0 Q4 (30s disappearance reproduction):** Partially RESOLVED — the L4 fixture #16 captures the user-visible target. Concrete timing-chain reproduction script can be authored during implementation as part of the L1 self-validation; not L0 closure-blocking.
5. **V1.0 Q5 (typed error fingerprinting):** RESOLVED — same-UID local model; deferred to v1.x-federation maintenance per §11.
6. **V1.0 Q6 (explicit refresh trigger surface):** RESOLVED — moot, since V1.1 doesn't require distinguishing "explicit refresh" from "render read". All triggers route through the same `project_composition` route; cache key determines whether producer runs.

### New open questions for V1.1 cycle 2

V1.1 has no new open questions — all cycle-1 findings folded with explicit
disposition (FOLD / DEFER / REJECT). Cycle 2 reviewers should validate that:
- The §5.4 reframe (producer commit preserved) correctly resolves the cycle-1 BLOCK without reintroducing the user-visible loop.
- The §5.3 option (a) implementation correctly avoids the cache-miss-on-stale-watermark loop.
- The §5.2 reloadTrigger pattern + full reload dep list correctly resolves the stale-closure risk.
- The §5.6 envelope rewrite matches the actual two-channel error surface (WP_Error from transport, runtime envelope from non-2xx).

## 13. Linear dependency edges

- v1.4.3 stabilization-B PR closes DOS-671, DOS-672.
- No upstream Linear dependencies — substrate already exists from v1.4.2 W4-F (PR #298 merged 2026-05-17).
- Soft dependency on Packet A: if Packet A's keychain classification lands first, the typed error mapping in §5.6 has one more error code to cover (`session_requires_repair` from a transient lookup `Unavailable`). Packet B can absorb this in the same enum.
- Downstream: every v1.4.3+ WP block depends on the stabilized render path; without these fixes, the C1 starter kit's reference implementation is the same broken pattern.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | V1.1 cycle-2 focus |
|---|---|---|
| `/codex challenge` | adversarial | Re-verify the §5.4 reframe (producer commit preserved + cache effectiveness fix). Is there a scenario where the §5.3 option (a) cache fails to invalidate when state actually moves? Stress the stale-closure resolution in §5.2 — is `reloadTrigger`'s derived-string pattern actually stable across React's render cycle? Confirm the V1.1 deferrals are correctly classified as remote/federation, not local-shipping. |
| `code-reviewer` (claude) | domain | Verify §5.1 wrapper preservation against the 6 existing test fixtures. Verify the §5.6 typed-error switch table is exhaustive against existing runtime error codes. Verify the §9 grep gates are enforceable. |
| `/codex consult` | implementation feasibility | Walk the §5.3 option (a) implementation through `composition_render_orchestrator::cache_lookup` / `cache_store` and `mod.rs:2317-2430`. Verify §5.5 `authorize_local_render` plumbing through `authorize_for_path` with the new flag. Verify the §5.6 PHP switch table compiles cleanly with the existing renderer error helpers. |
| `/cso` | **mandatory** | Confirm the §5.4 reframe preserves the W4-F producer contract without weakening any trust boundary. Specifically: (a) cache-key-by-current-db-version still requires authorization BEFORE lookup (same as today); (b) `charge_ability_scope=false` decharge does NOT bypass scope check or authorization; (c) the typed error mapping does not expose any new attack vector under same-UID local model. |

**Convergence rule:** unanimous APPROVE required before code lands. Any reviewer
returning CONDITIONAL APPROVE → fold finding into V1.2 (or maintenance backlog
if remote-shape) and re-run all four reviewers. Cycle cap: 3 cycles before
escalation to L6.

**Cycle 2 special handling for codex challenge:** if codex challenge re-flags
the V1.1 deferred items as local-blocking, the deferral classification must
be re-examined — but the L0 reviewer panel is not allowed to overturn a
Path-α trim that aligns with the v1.4.2 explicit threat-model commitment.
Persistent disagreement → L6 (James) decides. The §5.4 architecturally
contested-change L6 trigger from V1.0 is removed in V1.1 since the contested
change is withdrawn.

## 15. Acceptance for L0 closure

- [ ] All 4 reviewers returned APPROVE (cycle 2 or later — V1.1.1 cycle-2 text fixes folded inline).
- [ ] All 16 acceptance criteria (§7) testable; coverage mapped to §8 fixtures (18) + §9 CI invariants per §8.1 mapping table.
- [ ] All 7 CI invariants (§9 V1.1.1) have concrete grep/AST/runtime enforcement.
- [ ] All §12 V1.0 open questions resolved in V1.1 (6/6 — see §12).
- [ ] §5.3 cache key shape picked: option (a) — current_db_version key.
- [ ] §5.5 render-read decharge approach picked: option (a) — `charge_ability_scope=false` boolean.
- [ ] §5.4 producer-commit-removal WITHDRAWN — no architectural contest remains.
- [ ] V1.1 deferred items filed as Linear maintenance tickets under project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` (DailyOS Maintenance & Production Quality). 6 deferral ticket titles in §2 V1.1 changelog.
- [ ] Landing shape (§10) confirmed: single PR with 4 commit groups (was 5; removed the producer-commit-removal group), no split.
- [ ] No outstanding L0-cycle findings; packet is implementation-ready.

When all nine boxes check, L0 is closed and implementation begins. L1 (self)
proof bundle includes: PHP unit output for single-fetch + typed-error mapping,
JS test output for reload lifecycle, Rust unit output for cache hit rate + no-commit-on-read,
hands-on log (initial render → 60s → focus x2 → manual reload x2), audit-log
excerpt showing no render-path composition commit and no rate-budget consumption
on local render reads.
