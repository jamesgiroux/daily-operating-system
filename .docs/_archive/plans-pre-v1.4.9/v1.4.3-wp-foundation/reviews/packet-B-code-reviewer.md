# L0 Packet B — code-reviewer (domain) verdict

**Reviewer:** code-reviewer (claude domain pass)
**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md` V1.0
**Cycle:** 0 (first pass)
**Date:** 2026-05-17
**Scope:** Render-path integration smells; substrate reuse completeness;
testability of the §8/§9 acceptance/CI gates against existing tooling.

## Verdict: **CONDITIONAL APPROVE**

The diagnosis is correct, the architectural challenge in §5.4 is well-framed,
and the local-to-local scope check holds. The fixes route cleanly through
existing primitives in §4. Three integration smells, two testability gaps,
and one error-envelope-shape inaccuracy are below acceptance-blocking but
need V2 folds.

Each finding is bounded by the AC in §7 and the local-to-local frame from
W4-F V3.2. Theoretical hardening is filed against the Maintenance project,
not folded back into cycle 1.

---

## Findings — by area

### F1 — §5.1 `render_block_with_filter` has a second caller the packet does not list (CONDITIONAL, blocking)

`wp/dailyos/blocks/account-overview/render.php:20-25` is the production
front-end render path called by the block-registration runtime via
block.json's `render` field:

```php
if ( ! function_exists( 'dailyos_account_overview_render' ) ) {
    require_once __DIR__ . '/render-functions.php';
}
return dailyos_account_overview_render( $attributes );
```

§5.1 proposes splitting `dailyos_account_overview_render` into a "fetch +
render" wrapper plus a pure `..._render_from_projection` helper, and routing
`account_overview_preview` to the helper directly. That is the right shape
for preview parity, but the front-end render path at `render.php` still
needs the wrapper to behave exactly as today (one runtime call → projection
→ HTML). The packet currently implies the wrapper is "optional" — please
rephrase §5.1 to: the wrapper retains its single-fetch contract for the
block.json render path; the preview route stops invoking the wrapper.

Also `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php` calls
`dailyos_account_overview_render` directly at six sites (lines 52, 61, 96,
131, 169, 189). The new helper signature MUST keep these fixtures green; if
the helper takes `($response, $attributes)` as drafted in §5.1, the existing
tests don't change. Confirm this is the intent in V2.

**Required fold:** §5.1 explicitly lists both callers (`render.php` + the
six test fixtures), states the wrapper's behavior is unchanged for them,
and that only `account_overview_preview` switches to the helper.

### F2 — §5.6 error envelope shape mis-states the runtime contract (CONDITIONAL, blocking)

§5.6 says "runtime returns `{ ok: false, code: \"...\", ... }`". The actual
runtime envelope (`surface_runtime/mod.rs:3514-3548`) is:

```json
{"error": {"code": "...", "message": "...", "request_id": "...", "remediation": "...", "retry_after_ms": ..., "axis": "..."}}
```

— with HTTP status conveying success/failure and `RETRY_AFTER` /
`x-ratelimit-exhausted-axis` headers carrying rate-limit detail. There is
no top-level `ok: false` field on the runtime side.

The WP transport at `class-dailyos-runtime-client.php:462-509` re-wraps
non-2xx responses into its own `{ok: false, error: {code, message}}` shape.
So `render-functions.php` receives either:
- a `WP_Error` (transport layer — `wp_remote_post` failure, signing failure
  before the request is built), or
- a `{ok: false, error: {code, message}}` envelope (runtime non-2xx).

§5.6's mapping table should be re-anchored to these two surfaces. Also
worth confirming: `session_requires_repair` is a `SurfacePairingError`
variant (`surface_pairing.rs:277`) surfaced through `from_pairing_error` —
it preserves the same string in `error.code`. There is no dedicated
`SurfaceHttpError::session_requires_repair()`, so the mapping is by string
match on `error.code`, not by a typed enum on the runtime side.

**Required fold:** §5.6 rewrites the mapping table against the actual
two-channel error surface (WP_Error from transport, runtime envelope from
non-2xx). List the concrete string codes against `surface_runtime/mod.rs`
line numbers: `rate_limited` (3419), `session_not_found` (3361),
`session_requires_repair` (via `from_pairing_error`, `surface_pairing.rs:277`),
`runtime_unavailable` (3447), `host_invalid` (3285), `auth_missing` (3303).

### F3 — §5.5 decharge breaks the early-retry-tighten audit channel (CONDITIONAL, needs explicit acceptance)

`surface_client.rs:411-415` and `:742-752` show that `RateLimitOutcome::Allowed`
carries `audit_events` populated by `rate_limit_audit_event(..., "tightened", ...)`
when a previously-rejected window enters early-retry tightening. Bypassing
`check_and_consume` for paired-loopback render reads (§5.5 option a) ALSO
bypasses this audit emission.

For local-to-local, the early-retry-tighten event is operator-side
observability for "this surface client previously got rate-limited and is
being treated more strictly for a window" — not security-load-bearing
because no consumption means no rejection means no tightening. The
semantics is internally consistent. But the audit-trail expectation
documented in W4-F V3.2 §"What's missing" / DOS-655 around bridge audit
emission needs an explicit "decharge omits tighten-event emission, by
construction" acceptance line.

**Required fold:** Add an acceptance criterion to §7 (DOS-672 column)
asserting that local-render-decharge sessions emit zero
`rate_limit_audit_event` entries (regardless of `decision`), and document
this as expected per local-to-local. Don't escalate — file the audit-coverage
maintenance note (operator observability of local-only render budget) to the
maintenance project per CLAUDE.md Path-α rule.

### F4 — §5.2 Gutenberg lifecycle paths the trimmed deps may miss (CONDITIONAL, scope question)

The packet trims `useEffect([reload])` to fire only on `composition_id`
empty→non-empty or `account_id` change. Gutenberg's editor has additional
component-remount triggers that React WILL run the effect for, regardless
of dep list:
- Initial mount on editor open / block insertion.
- Block-list reorder (siblings change) — Gutenberg may remount; this is
  benign because mount fires the effect once with current attributes.
- Undo/redo — Gutenberg replays attribute mutations, which would change
  `composition_id`/`account_id` and SHOULD trigger reload (correct
  behavior, not a leak).
- Autosave / save-pending — these don't remount the edit component but
  may invoke `apiFetch` patterns the editor doesn't directly control.
  The packet's §12 Q6 already calls this out for codex consult; flag here
  that the L0 closure needs the Q6 answer documented before §5.2 fixtures
  are sized.

**Required fold:** §5.2 explicitly enumerates the remount triggers it
EXPECTS to fire reload (mount, account change, composition_id transition)
and the ones it expects to NOT (focus, autosave, undo of cache_hint_token
write). The fixture list §8 #2/#3 covers the negative; add a positive
fixture for "first mount triggers exactly one reload" so the trimmed
behavior doesn't accidentally suppress the initial render.

### F5 — §5.3 cache-key options both have a subtle audit-trail consequence (NON-BLOCKING, document)

Option (a) "key on `(actor, composition_id, current_db_version)`" requires
a `db_read` of `current_composition_version_for_composition_id` (per
`mod.rs:2355-2370`) BEFORE the cache lookup, on EVERY render. Today that
read happens only on cache miss (after the existing cache lookup at :2317).
Moving the read upstream of the lookup means even cache HITS pay one
`db_read` round-trip. That's acceptable for local-to-local but should be
called out — the cache stops being a "no DB hit on warm path" primitive
and becomes a "one read on warm path, full producer commit on cold path"
shape. Option (b) preserves the warm-path zero-DB-touch property.

§12 Q1 asks reviewers to pick. Recommend §12 Q1 resolution include a one-line
"warm-path DB read footprint" comparison so the L0 record makes the
trade-off legible. No blocker — just record the cost.

### F6 — §8 JS fixtures have no test runner today (CONDITIONAL, infrastructure)

`wp/dailyos/package.json` has no `test` script, no jest/vitest dep, no
`*.test.js` files. Fixtures #2, #3, #8, #10 are JS unit tests against
`apiFetch` mocks. Either (a) the implementation must add the test runner
as part of this PR's scope, or (b) the JS coverage must be expressed via
PHP-side preview tests that assert request count + payload shape against
the fake runtime client (the pattern already used at
`tests/blocks/AccountOverviewBlockTest.php`).

The packet should NOT assume "JS test infrastructure exists" without
either step in scope. Adding a wp-scripts jest runner is small (well under
a day) but is net-new infrastructure that wasn't enumerated in §4 substrate
reuse.

**Required fold:** §8 declares whether JS fixtures land as (a) new
wp-scripts jest setup (added to §4 substrate reuse audit as net-new) or
(b) PHP integration tests asserting equivalent behavior. If (a), §10
landing-shape commit groups grow to six (jest setup as commit 0).

### F7 — §9 CI invariants reference enforcement mechanisms — most exist, one doesn't (CONDITIONAL, infrastructure)

Surveyed `.githooks/` (commit-msg, pre-commit, pre-push) and
`.github/workflows/` (rust.yml, wp-plugin.yml, lint-frontend.yml,
l2-review.yml, l3-review.yml, load-test.yml, release.yml,
security-audit.yml). Mapping:

| §9 invariant | Mechanism | Status |
|---|---|---|
| #1 single-fetch grep | `wp-plugin.yml` can run a phpcs custom sniff or a grep step | Need new step, not gate |
| #2 reload dep array ESLint | `lint-frontend.yml` runs wp-scripts lint; custom ESLint rule needs writing | Net-new ESLint rule |
| #3 useEffect narrow trigger | Same as #2 | Net-new ESLint rule |
| #4 no commit_composition on read path | Rust grep / AST check — `rust.yml` step | Net-new step, mechanism viable |
| #5 cache hit rate ≥95% test | Rust integration test in `rust.yml` | Existing harness |
| #6 local-render decharge test | Existing rate-limit test harness in `bridges/surface_client.rs` | Existing harness |
| #7 verification banner usage grep | `wp-plugin.yml` phpcs / grep step | Net-new step |

Invariants #2 and #3 require an ESLint rule that doesn't exist
(`.eslintrc*` not found in `wp/dailyos`). Authoring a custom ESLint rule
for "useEffect dep array equals this literal" is real work — possible via
`eslint-plugin-react-hooks/exhaustive-deps` configuration plus a custom
plugin, or a simpler grep gate that checks the dep array literal in
`edit.js`. Recommend the simpler grep gate for L0, file the ESLint rule
as a maintenance ticket.

**Required fold:** §9 enforcement column distinguishes which gates are
"grep step in existing workflow" vs "net-new ESLint rule" vs "existing
Rust test harness". The two ESLint rules should either land as grep
checks or be filed as maintenance with a less-strict interim gate.

### F8 — Intelligence Loop integration check (NON-BLOCKING, exempt)

Per CLAUDE.md's 5-question gate: this packet does NOT introduce new claims,
schema columns, lifecycle states, or user-visible intelligence surfaces.
The change set is pure operational hardening of the render path — fewer
runtime calls, narrower cache key, fewer DB writes, narrower reload
triggers. The Intelligence Loop check is legitimately exempt.

The ONE adjacent concern: §5.4 removes `commit_composition` from the
steady-state render path. `commit_composition` is the substrate's
projection-write primitive; signal-propagation must still trigger it when
upstream claims change. The packet's §5.4 already names "signal-propagation
invalidation (a watermark moved upstream — `account_subject.claim_changed`,
`claim.lifecycle`)" as a producer-commit trigger, which preserves the
intelligence-loop contract. Acceptance #5 should add: "Producer commit on
signal-propagation IS exercised in fixture #5 against a real
`account_subject.claim_changed` signal" — not a mocked trigger — so we
prove the intelligence loop still closes through the new path.

### F9 — §5.4 producer-commit removal: alignment with W4-F V3.2 (NON-BLOCKING, defer to CSO + codex challenge)

The packet's §5.4 challenges the comment at `mod.rs:2348-2353` but the
W4-F V3.2 packet (read on `docs/v143-carry-forward` branch) does NOT
include "always commit on render" as a load-bearing element of the V3.2
trim. W4-F V3.2's substrate fix targets the dispatch-site contention
(`db_write` → `db_read`) and the OK-path write removal in
`validate_signed_session`. The producer-commit-on-render at
`mod.rs:2348-2353` is documented as an OCC-related workaround for the
producer's own watermark check — not as part of the W4-F V3.2 contract.

This means §5.4 is LESS contested than the packet positions it. The L0
question "does removing render-commit break the DOS-670 producer OCC
contract" is still real and should go to CSO + codex challenge as §12 Q2
specifies, but it doesn't conflict with W4-F V3.2's commitments. The
packet's framing of "this contests W4-F" overstates — V3.2 doesn't own
the render-commit decision. Suggest §5.4 prose say "this contests the
DOS-670 producer-side workaround documented at `mod.rs:2348-2353`," not
"contests v1.4.2 W4-F."

**Required fold:** Rephrase §5.4 opening + §6.4 to attribute the contested
decision to DOS-670's producer-side fix, not to W4-F as a whole. This
narrows the L6 escalation surface — only the DOS-670 author(s) need be
consulted, not the full W4-F reviewer set.

---

## Per-area summary against §7 acceptance criteria

| AC# | Status | Notes |
|---|---|---|
| 1 (single-fetch) | covered by F1 fold | requires explicit caller enumeration |
| 2-4 (editor reload) | covered by F4 fold | add positive "first-mount fires reload" fixture |
| 5 (no render commit) | covered by F8 + F9 | tie to signal-propagation in fixture #5 |
| 6 (cache hit) | covered by F5 fold | record DB-read cost of option (a) |
| 7 (decharge) | covered by F3 fold | acceptance criterion for zero tighten-events |
| 8 (60s + 2 focus changes) | as-stated | L4 hands-on per §15 |
| 9-13 (reload semantics) | covered by F4 fold | |
| 14 (typed error mapping) | covered by F2 fold | rewrite mapping to actual envelope |
| 15 (merge no longer combines) | as-stated | tautological after §5.1 |

## CI invariants vs available enforcement

See F7 — 3 of 7 invariants need net-new gates (grep or ESLint). Recommend
grep gates for L0 closure; file proper ESLint rule authoring to
maintenance.

## Acceptance for cycle-1 closure (this reviewer)

Fold F1, F2, F3, F4, F6, F7, F9 into V1.1. F5 and F8 are non-blocking but
worth recording in the V1.1 changelog. After V1.1: APPROVE.
