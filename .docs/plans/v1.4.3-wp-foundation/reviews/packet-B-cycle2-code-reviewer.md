# L0 Packet B — code-reviewer cycle-2 verdict

**Reviewer:** code-reviewer (claude domain pass)
**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md` V1.1
**Cycle:** 2 (cycle-1 fold validation)
**Date:** 2026-05-17
**Prior:** `reviews/packet-B-code-reviewer.md` (cycle-1, CONDITIONAL APPROVE, 9 findings)

## Verdict: **CONDITIONAL APPROVE**

V1.1 folds the 7 blocking cycle-1 findings cleanly and the 2 non-blocking
findings are addressed by changelog framing. Two narrow residuals remain
(§5.6 switch coverage of session-repair-shaped codes; AC↔fixture 1:1 mapping
gap on AC #16). Both are V1.2-foldable in one pass; neither contests the
architectural shape.

---

## Per-cycle-1-finding validation

### F1 (second caller) — RESOLVED
§5.1 V1.1 explicitly enumerates both production callers (`render.php:20-25`,
`account_overview_preview` at `class-dailyos-plugin.php:587-612`) plus the 6
test fixtures (lines 52, 61, 96, 131, 169, 189). Verified against
`AccountOverviewBlockTest.php`: every call site uses the signature
`dailyos_account_overview_render( array $attributes ): string` with a single
arg. V1.1's preserved wrapper signature is identical. Fixtures stay green.

### F2 (error envelope) — RESOLVED with residual
§5.6 V1.1 correctly re-anchors against the actual two-channel surface:
- runtime envelope: `{"error":{"code","message","request_id","remediation",...}}` — confirmed at `surface_runtime/mod.rs:3514-3548`.
- WP transport re-wrap to `{ok:false, error:{code,message}}` — confirmed at `class-dailyos-runtime-client.php:393-509` (the `if ( 200 > $status_code || 299 < $status_code )` branch + `$envelope['ok'] = false`).
- `WP_Error` channel for transport-layer failure — confirmed.
- `session_requires_repair` correctly identified as a `SurfacePairingError` variant via `from_pairing_error` (not a `SurfaceHttpError` constructor).

**Residual (V1.2-foldable, NON-BLOCKING):** the §5.6 switch covers
`rate_limited`, `session_requires_repair`, `session_not_found`,
`runtime_unavailable`, `runtime_request_failed`, `runtime_invalid_json`,
`runtime_http_error`, plus 6 consistency codes. It does NOT enumerate the
following codes the runtime actually emits (verified against `mod.rs:3285-3447`
constructors + `from_pairing_error` exhaustive match):

- Session-repair-shaped (should route to session-repair notice, not verification banner): `identity_mismatch`, `wrong_user`, `pairing_code_expired`, `pairing_code_consumed`, `pairing_code_invalid`, `pairing_code_limited`, `site_binding_mismatch`, `pairing_suspended`, `pairing_revoked`, `pairing_expired`, `session_expired`, `session_throttled`, `wp_user_mismatch`, `restored_stale_pairing`, `scope_denied`, `auth_missing`.
- Configuration-shaped (likely runtime-unavailable notice): `host_invalid`, `browser_origin_forbidden`, `route_not_found`.
- Bad-request codes (renderer probably shouldn't render at all; defensive bad-request notice): `request_body_too_large`, `request_body_unreadable`, `handshake_body_invalid`, `session_refresh_body_invalid`, `surface_invoke_invalid`, `event_log_id_invalid`.

V1.1's "unknown code → verification banner (fail-safe)" handles all of these,
so this is not a correctness bug — but `identity_mismatch` / `wp_user_mismatch`
/ `pairing_*` rendering as "verification banner" mis-attributes the failure
to projection consistency when the real cause is session/pairing. Recommend
V1.2 extends the §5.6 switch's session-repair arm to include the pairing
codes + identity codes, and adds a "renderer-input-invalid" arm for the
bad-request codes. File as V1.2-fold (or maintenance if §5.6 hits its
expressive ceiling) — does NOT block L0 closure.

### F3 (decharge audit) — RESOLVED
AC #15 V1.1 codifies "local-render decharge omits tighten-event by
construction" with concrete mechanism (`charge_ability_scope=false` → no
consumption → no rejection → no tightening). Fixture #11
(`dos671_local_render_no_tighten_event`) asserts zero `rate_limit_audit_event`
entries across 100 reads. Render-volume audit signal correctly deferred to
maintenance per §11.

### F4 (Gutenberg lifecycle) — RESOLVED
§5.2 V1.1 enumerates expected-fire (mount, account_id change, composition_id
empty↔non-empty) and expected-not-fire (window focus, autosave, undo of
cache_hint_token/version, block-list reorder remount as benign no-op).
Fixture #5 (`dos671_first_mount_fires_one_reload`) covers the positive case
called out by cycle-1.

### F6 (JS test runner) — RESOLVED
§8 V1.1 switched fixtures #2, #3, #8, #10 to PHP equivalents against the
existing `tests/blocks/AccountOverviewBlockTest.php` fake-runtime-client
pattern. Net-new jest infrastructure correctly avoided.

### F7 (ESLint) — RESOLVED with grep-gate note
§9 V1.1 replaced invariants #2/#3 ESLint rules with grep gates. ESLint rule
authoring correctly filed to maintenance per §11.

**Grep-gate stability note (NON-BLOCKING):** the §9 invariant #2 dep array
literal is `[attributes.composition_id, attributes.composition_version,
attributes.cache_hint_token, setAttributes]`. Current `edit.js:82` formats
this as `[ attributes.composition_id, attributes.composition_version,
attributes.cache_hint_token, setAttributes ]` (prettier-style spaces inside
brackets). The grep gate MUST tolerate whitespace (suggest a regex collapsing
whitespace, or a normalized-AST check via `node -e` on a simple parse).
Same caveat for invariant #3 `[reloadTrigger]`. Implementation detail for L1,
not L0-blocking, but worth flagging here so the implementer doesn't write a
brittle literal-substring grep.

### F9 (framing) — RESOLVED
§5.4 V1.1 narrows correctly: the contested anchor at `mod.rs:2348-2353` is
the DOS-670 producer-side OCC workaround. V1.1 doesn't contest it at all
(the producer commit is preserved); the L6 trigger is removed from §14.
Framing is now accurate.

---

## Numbered validations from cycle-2 brief

1. **F1/F2/F3/F4/F6/F7/F9 folded:** all 7 verified above. F5 and F8 (non-blocking) acknowledged in V1.1 changelog narrative.

2. **§5.6 switch table exhaustiveness:** see F2 residual above. The switch covers the operationally common codes; ~16 additional codes fall through to verification-banner via the default arm. Functionally correct (fail-safe) but mis-attributes pairing/identity failures to consistency. V1.2 fold recommended; not L0-blocking.

3. **§5.2 reloadTrigger stale-closure:** idiomatic. `reload` is a `useCallback` whose body reads `attributes.*` at call time via its declared dep list — manual button gets latest version+token. The `useEffect([reloadTrigger])` invokes `reload` from the render's closure; since `reload` is intentionally NOT in the effect's deps, the effect captures whichever `reload` reference was current when `reloadTrigger` last changed. This is correct because trigger changes coincide with attribute changes that already invalidate `reload`. The Gutenberg attribute write order (`setAttributes` flushes before the next render) ensures the reload-on-trigger sees the new attributes via the next render's `reload`. No stale-closure pathology.

4. **§5.1 wrapper signature vs 6 fixtures:** all 6 fixtures (lines 52, 61, 96, 131, 169, 189) invoke `dailyos_account_overview_render( [...] )` with a single attribute array. V1.1's preserved wrapper signature `dailyos_account_overview_render( array $attributes ): string` is byte-identical to today. Internal refactor delegating to `..._render_from_projection( $response, $attributes )` is invisible to callers. Fixtures stay green.

5. **§9 grep gates realism:** invariants #1, #4, #6, #7 grep on stable PHP/Rust call-site names — realistic. Invariants #2, #3 grep on JS literal shape — workable IF the gate tolerates whitespace (see F7 note above). Invariant #5 (cache hit rate ≥95%) is a Rust integration test, properly anchored in existing `rust.yml`.

6. **§10 4-commit-group landing:** group separation is clean — PHP (5.1+5.6), JS (5.2), Rust cache (5.3), Rust decharge (5.5). No missed integration. The §5.3 Rust change is upstream of §5.5 (cache lookup runs before authorize? No — verified at `mod.rs:2288-2321`: authorize runs first, THEN cache_lookup, so the §5.5 decharge wraps the existing authorize call and §5.3 modifies the subsequent cache key). Order-independent within the same PR.

7. **§11 deferrals — local-shipping safety check:**
   - Signal-propagation invalidation bus → federation scope; local-single-runtime relies on producer-commit-on-cache-miss as the natural invalidation channel. Correct.
   - Hostile co-resident plugin error fingerprinting → remote-shape; same-UID local trust boundary is correct.
   - Render-volume audit signal → operator-observability, not security-load-bearing locally. Correct.
   - ESLint rule authoring → grep gates cover L0. Correct.
   - Persistent projection storage → would enable producer-commit removal; massive scope; federation-driven. Correct.
   - Hot-path latency budgeting → operational tuning. Correct.

   None hide a local-shipping bug.

8. **AC↔fixture mapping (16 ACs ↔ 16 fixtures):**

   | AC | Fixture(s) | Status |
   |---|---|---|
   | 1 single-fetch | #1, #2 | covered |
   | 2 reload dep list | #3 | covered |
   | 3 useEffect trigger key | #4 | covered |
   | 4 successful reload no-retrigger | implicit in #5 | covered |
   | 5 first-mount fires one reload | #5 | covered |
   | 6 cache lookup uses current_db_version | #6 | covered |
   | 7 cache store uses projection.composition_version | #7, #8 | covered |
   | 8 60s + 2 focus changes | #16 | covered |
   | 9 manual reload single request | #12 | covered |
   | 10 window-focus no reload | #16 covers via hands-on | covered |
   | 11 second manual reload succeeds | #12 implicit | covered |
   | 12 failed reload preserves last-good | #13 | covered |
   | 13 failed reload surfaces error notice | #13 | covered |
   | 14 typed error switch | #14, #15 | covered |
   | 15 decharge tighten-event omitted | #11 | covered |
   | 16 auth/scope checks not bypassed | **GAP** | no dedicated fixture |

   **AC #16** ("descriptor, actor, mode, experimental, required scopes,
   browser-direct guard NOT bypassed; only rate-budget consumption for
   ability/scope changes") needs an explicit fixture. Suggest #17:
   `dos672_local_render_authorize_checks_still_enforced` — Rust test that
   `authorize_local_render` rejects with `ScopeDenied` / `actor_mismatch`
   when the validated session lacks `read.account_overview` scope, proving
   `charge_ability_scope=false` only skips rate-budget consumption, not
   authorization gates.

   Use Packet A V1.1.1 §8.1 mapping-table pattern to make this explicit in
   §8 and re-count to 17 fixtures (or fold the assertion into existing
   fixture #9 and call it out in the AC#16→#9 mapping). V1.2-foldable in
   one pass.

---

## Acceptance for cycle-2 closure

V1.2 folds required:
- (a) §5.6 switch extends session-repair arm to include `identity_mismatch`, `wrong_user`, `wp_user_mismatch`, `pairing_*`, `site_binding_mismatch`, `session_expired`, `session_throttled`, `restored_stale_pairing`, `scope_denied`, `auth_missing`; adds "renderer-input-invalid" arm for `request_body_*` / `*_invalid` codes; OR explicitly accepts default-banner-fallback for these and adds a one-line note in §5.6 that the fall-through is intentional under current shape.
- (b) AC #16 gets a fixture (new #17 or fold into #9); §15 closure box "16 ACs ↔ 16 fixtures, 1:1 mapping verified" updated to reflect actual count.
- (c) §9 invariants #2/#3 grep-gate spec includes a whitespace-normalization note (one sentence in §9 row description).

After V1.2: APPROVE.

Path-α file-to-maintenance acceptable for (a) if the implementer prefers to
keep §5.6 narrow and accept the fail-safe default. (b) and (c) are local-
shipping safety items and should land in V1.2.
