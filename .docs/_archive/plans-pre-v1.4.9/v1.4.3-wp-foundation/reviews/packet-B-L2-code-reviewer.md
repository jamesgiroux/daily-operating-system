# Packet B — L2 code-reviewer pass (claude domain)

**Branch:** `dos-671-672-render-stabilization` (3 commits since `a594cd4d`)
**Anchor:** L0 Packet B V1.1.1
**Scope:** AC-adjacent. Findings outside §7 ACs / ADRs / PR-introduced regressions go to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.

## Verdict: APPROVE

All 16 §7 ACs implemented; 18 fixtures landed; 4 of 7 §9 CI invariants enforced by grep/test gates in this diff (the remaining 3 are either runtime-test-style invariants that exist as behavioral fixtures or pre-existing CI workflow assertions). No PR-introduced regressions. Services/ ownership intact. Authorization preserved.

Two path-α findings (below). Neither is an AC violation; both file to maintenance.

---

## 1. Commits-vs-L0-§5 mapping

| L0 §5 section | Commit | Status |
|---|---|---|
| §5.1 single-fetch PHP preview (wrapper preserved) | `84bc4baa` | OK — wrapper present at `render-functions.php:32-34`; `render.php` still calls wrapper; preview route at `class-dailyos-plugin.php:587-621` calls `render_from_projection` directly (no `render_block_with_filter` re-entry). |
| §5.2 editor reloadTrigger pattern | `5d125fee` | OK — `edit.js:94` keeps full `reload` dep list `[composition_id, composition_version, cache_hint_token, setAttributes]`; `:96` defines `reloadTrigger` exactly per L0; `:97-102` `useEffect([reloadTrigger])` with eslint-disable + rationale. |
| §5.3 cache key by current_db_version | `ee1805f5` | OK — `mod.rs:2336-2354` moves the `current_composition_version_for_composition_id` read upstream of cache lookup; lookup at `:2361` uses `current_db_version`; store at `:2444-2448` uses `projection.composition_version.unwrap_or(current_db_version)`. |
| §5.4 producer-commit reframe (no-op) | n/a | OK — withdrawn in L0 V1.1; nothing to implement. The producer commit path is preserved; only the cache-key correction affects how often it fires. |
| §5.5 `authorize_local_render` decharge | `ee1805f5` | OK — new pub fn at `surface_client.rs:277-291` delegates to `authorize_for_path` with `charge_ability_scope=false`; route handler `mod.rs:2317` switches to it. |
| §5.6 typed error switch | `84bc4baa` | OK — full code matrix from L0 §5.6 lives at `render-functions.php:85-141` with 4 typed notice helpers + verification banner reserved for consistency-failure / unknown-code branches. |

## 2. Substrate reuse (§4)

Confirmed: no net-new primitives. The diff uses `compositions::current_composition_version_for_composition_id` (existing), `CompositionRenderOrchestrator::cache_lookup`/`cache_store` (existing, key shape unchanged at `composition_render_orchestrator.rs:51-56`), `authorize_for_path` (existing private helper — new `authorize_local_render` is just a thin variant), and existing `RuntimeNotice` design-system pattern. Wrapper preservation for `dailyos_account_overview_render` keeps all 6 prior test fixtures intact (verified by inspection of `AccountOverviewBlockTest.php`).

## 3. §9 CI invariants enforcement in diff

| # | Invariant | Status |
|---|---|---|
| 1 | preview route bypasses `render_block_with_filter` re-entry | Behavioral fixture `test_preview_route_uses_single_runtime_request` asserts `$GLOBALS['dailyos_test_remote_post_calls']` count==1. No grep gate, but the runtime-call assertion is stronger. |
| 2 | editor `reload` callback dep array | `test_editor_reload_callback_dep_array_literal` regex matches the dep literal with prettier-tolerant whitespace `\s*` per L0 V1.1.1 note. |
| 3 | `useEffect([reloadTrigger])` (NOT `[reload]`) | `test_editor_reload_trigger_key_literal` matches the trigger string + `useEffect([reloadTrigger])` shape + `assertDoesNotMatchRegularExpression` on `[reload]`. |
| 4 | cache lookup keyed by `current_db_version` | Rust grep gate `dos671_project_composition_cache_lookup_current_db_version_grep_gate` at `mod.rs:5465-5485` — positive + negative assertions both present. |
| 5 | Cache hit rate ≥95% under varying request.composition_version | Not added as a workload test in this diff; behavioral coverage exists via `dos671_cache_key_uses_current_db_version` + `dos671_external_version_advance_invalidates_cache`. Path-α — fold quantitative gate into workload test in maintenance backlog. |
| 6 | `authorize_local_render`=false; `authorize`=true | `dos672_project_composition_route_uses_local_render_authorization_grep_gate` confirms route uses `authorize_local_render`; behavioral coverage via `authorize_local_render_does_not_charge_ability_or_scope_buckets` (and the symmetric `authorize` test in the same file). No grep gate asserts that `authorize_for_path` is called with `true` from the main `authorize` and `false` from `authorize_local_render`. Path-α — add a literal grep gate (minor reinforcement). |
| 7 | verification banner only from consistency / default branches | Three call sites confirmed: consistency-failure switch arm, default arm, and the "successful response without projection" defensive branch (`render-functions.php:133, 135, 143`). The third call site is technically outside the §9 #7 wording ("emitted ONLY from the consistency-failure branch + unknown-code fail-safe") but is a fail-safe for malformed success envelope — semantically equivalent intent. Path-α — clarify §9 #7 to include "defensive shape-mismatch fail-safe" OR remove this branch in favor of `RuntimeUnavailableNotice`. Maintenance, not a block. |

## 4. PHP code quality

- Wrapper preservation: `dailyos_account_overview_render($attributes)` retained with identical signature at `render-functions.php:32-34`. All 6 prior test fixtures continue to call the wrapper. Verified inline in the test diff.
- `dailyos_account_overview_fetch_projection` and `dailyos_account_overview_render_from_projection` split is clean; `render_from_projection` is pure (no runtime call).
- `is_string($response)` first-line guard at `:82-84` handles the empty-state HTML return from `fetch_projection` (composition_id missing, runtime-client construction failure, etc.) — sane and matches L0 §5.1 contract.
- Notice helpers use existing `RuntimeNotice` design-system tier with stable `data-ds-name` slugs — token-only, no inline CSS.

## 5. JS code quality — reloadTrigger pattern

The `reloadTrigger = `${account_id || ''}|${composition_id ? '1' : '0'}`` derives a stable string from primitives. Successful reload's `setAttributes` write of `composition_version`/`watermarks`/`cache_hint_token` does NOT change `reloadTrigger` value → no re-trigger loop. No infinite re-render risk. Manual reload via `reload()` continues to read latest `composition_version`/`cache_hint_token` from attrs because `reload`'s dep list is unchanged. The eslint-disable carries the required rationale.

Failed-reload path (`response.ok === false`) takes an early return BEFORE `setPreview(response)`, preserving prior `preview` state. AC #12/#13 satisfied.

## 6. Rust code quality — cache key change + authorize_local_render

**Cache key change does NOT bypass authorization.** Order in `surface_project_composition_response`:
1. Registry resolution (`mod.rs:2289-2300`).
2. `authorize_local_render` call (`:2317`) — scope check, descriptor check, actor/mode gates ALL preserved (Cf. `surface_client.rs:277-291` → `authorize_for_path` → existing `check_and_consume` gating).
3. THEN current_db_version read (`:2336-2354`).
4. THEN cache lookup (`:2361`).

Authorization gates the request before cache is touched. The `scopes_canonical_id` in the cache key (`composition_render_orchestrator.rs:55,96`) already prevents cross-actor cache poisoning — change to version dimension only affects same-actor staleness. Correct.

**`authorize_local_render` preserves scope check.** Confirmed by fixture `authorize_local_render_enforces_required_scope` (AC #16). The `charge_ability_scope=false` boolean only short-circuits ability/scope rate-budget consumption inside `check_and_consume`; the required-scope gate (`surface_client.rs:301-355` per L0 §5.5 evidence) executes before. Identity bucket consumption preserved per `authorize_local_render_still_charges_identity_buckets`.

**Race analysis:** between the new DB read and cache lookup, a concurrent producer commit could land. For local single-runtime, producer invocations are serialized by the SurfaceClient writer mutex (per L0 §5.3). Worst case is one stale serve, healed on next render. Acceptable per L0.

## 7. Cross-commit regressions

Read `git diff a594cd4d..HEAD` end-to-end. No regressions identified:
- `linear_issue_signals.rs` change routes bus emit through `services::signals::emit_and_propagate` — actually a services/-ownership improvement aligned with CLAUDE.md "all mutations go through services/". Out of L0 §5 scope but defensible per commit message ("guards … so the requested validation gates can run on this worktree"). NOT a regression.
- `v178_dos_285_linear_issue_state.rs` adds a `table_exists` guard so the migration is idempotent when run on worktrees without the `linear_issues` table (other test contexts). NOT a regression — strictly additive safety.
- `check_claim_writer_allowlist.sh` extension for `W6-A-meta-*` fixture path is unrelated to packet B (looks like cross-branch contamination from a different worktree). Strictly additive to the allowlist. NOT a regression but flagged as scope-creep below.

## 8. Services/ ownership

Mutating call (`commit_composition` in test infra at `mod.rs:5247-5285`) routed through `services::compositions::commit_composition`. Cache reads/writes use `services::composition_render_orchestrator` API. Linear signal emission now goes through `services::signals::emit_and_propagate` (the change in commit 1 actually closes a previous direct-bus call). All compliant with CLAUDE.md Critical Rules.

---

## Path-α findings → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`

1. **§9 invariant #5 quantitative workload gate not added.** L0 spec calls for "Rust integration test in existing rust.yml workflow" asserting cache hit rate ≥95% under workload with varying `request.composition_version`. The diff provides behavioral coverage (3 deterministic cache-hit/miss tests + 1 external-advance invalidation test) but no quantitative workload assertion. File maintenance ticket: "Add cache-hit-rate workload integration test for project_composition route".

2. **§9 invariant #6 reinforcement grep gate.** Behavioral coverage exists. A literal grep asserting `authorize_for_path(..., true)` in `authorize`/`authorize_signed_invoke` and `authorize_for_path(..., false)` in `authorize_local_render` would harden against future drift where a refactor flips the boolean. File maintenance ticket: "Add grep gate for `authorize_for_path` charge_ability_scope literal per surface_client variant".

3. **§9 invariant #7 third-banner-call-site.** `render-functions.php:143` emits the verification banner on a successful runtime response that lacks a `projection` key (defensive fail-safe). Strictly outside L0 §9 #7 wording. Either widen the invariant text OR replace with `RuntimeUnavailableNotice` in a follow-up. File maintenance ticket: "Clarify §9 invariant #7 or migrate shape-mismatch fail-safe to RuntimeUnavailableNotice".

4. **Scope-creep observation (not a finding).** Commit 1 includes `check_claim_writer_allowlist.sh` `W6-A-meta-*` extension + `linear_issue_signals.rs` + `v178` migration guard. None of these are L0 §5 work for Packet B. They appear to be local-worktree validation-gate enablers per commit message. The migration guard and signals refactor are net-positive; the W6 fixture allowlist looks like cross-branch contamination. Author should confirm provenance before merge; no action required from L2 reviewer.

---

## Summary

L0 §5.1 / §5.2 / §5.3 / §5.5 / §5.6 implementation is complete and matches plan. §5.4 is correctly a no-op. ACs #1-#16 all have coverage (4 ACs via grep gates per L0 §8.1 mapping, 12 via runtime fixtures, 1 via L4 hands-on log placeholder). Authorization preserved. Services/ ownership preserved. No PR-introduced regressions. Approve for L2 close pending other reviewers.
