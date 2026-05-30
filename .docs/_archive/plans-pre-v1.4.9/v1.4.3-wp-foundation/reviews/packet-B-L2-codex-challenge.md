# Packet B - L2 Codex Challenge Review

Branch: `dos-671-672-render-stabilization`
Base: `a594cd4d`
Diff reviewed: `git -C /private/tmp/dailyos-pb diff a594cd4d..HEAD`
Anchor: `L0-packet-B-render-stabilization.md` V1.1.1, §7 ACs 1-16

## Verdict: APPROVE

The PHP single-fetch split, editor `reloadTrigger`, cache-key correction, and
`authorize_local_render` decharge are directionally correct. One AC-linked
block remains: the typed PHP error switch is not exhaustive against codes the
actual project-composition/signed runtime path can already emit, so existing
non-consistency failures still fall through to the verification banner.

## Findings by severity

### Critical

None.

### High

1. `wp/dailyos/blocks/account-overview/render-functions.php:90`
   What: `dailyos_account_overview_render_from_projection()` switches on
   `error.code`, but the handled matrix at `render-functions.php:93-135` omits
   already-emittable project-composition/signed-runtime codes. Examples:
   `project_composition_invalid`, `project_composition_unknown_producer`, and
   `project_composition_invalid_id` are emitted by
   `src-tauri/src/surface_runtime/mod.rs:2243-2275`; signed transport can emit
   `signature_invalid`, `canonicalization_mismatch`, `timestamp_stale`,
   `timestamp_future`, `key_not_found`, `key_rotated`, `token_invalid`,
   `nonce_replay`, and `transport_abuse_limited` via
   `src-tauri/src/surface_runtime/hmac.rs:792-803`; pairing validation can emit
   `unknown_runtime_anchor`, `session_invalid`, and
   `pairing_authority_unavailable` via
   `src-tauri/src/services/surface_pairing.rs:260-281`; the route can emit
   `composition_version_overflow` via
   `src-tauri/src/surface_runtime/mod.rs:3071-3076`.
   Why it matters: these are not unknown future codes. Today they hit the
   `default` arm at `render-functions.php:134-135`, returning
   `dailyos_account_overview_render_verification_banner()` for auth/signing,
   request-shape, pairing-authority, and overflow failures. That violates the
   packet intent that the verification banner is reserved for projection
   consistency plus true unknown-code fail-safe, and it leaves a path for the
   DOS-672 "second reload returns verification banner" symptom under real
   non-consistency failures.
   AC-linked: AC #14 typed error switch exhaustive coverage.

2. `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:448`
   What: fixture `test_typed_error_mapping` uses
   `typed_error_mapping_provider()` at
   `AccountOverviewBlockTest.php:448-505`, but the provider mirrors the same
   incomplete matrix as the PHP switch and never exercises the existing
   runtime codes listed above. In particular, no fixture row covers
   `project_composition_invalid`, `project_composition_unknown_producer`,
   `project_composition_invalid_id`, signed-transport errors from
   `hmac.rs:792-803`, `session_invalid`, `unknown_runtime_anchor`,
   `pairing_authority_unavailable`, or `composition_version_overflow`.
   Why it matters: fixture #14 is the stated AC #14 proof point, so the suite
   can pass while an existing runtime error still renders as a consistency
   banner. This is a test coverage gap for the same blocking behavior above,
   not an independent product issue.
   AC-linked: AC #14 / fixture #14.

### Medium

None.

### Low

1. `src-tauri/src/surface_runtime/mod.rs:2336`
   What: the route reads `current_db_version_for_producer` at
   `mod.rs:2336-2352`, then performs cache lookup at `mod.rs:2360-2362`, then
   invokes the producer on miss at `mod.rs:2396-2410`. There is no route-level
   mutex across read -> cache lookup -> producer invocation; the actual
   serialization is only at DB writer-call granularity
   (`src-tauri/src/state.rs:1493-1524`, `src-tauri/src/db_service.rs:116-172`).
   Two concurrent cold local renders for the same composition can both read
   version N, miss cache, and race into producer commits; `commit_composition`
   accepts exactly one expected-version writer and rejects the loser as stale
   (`src-tauri/src/services/compositions.rs:270-310`).
   Why it matters: this contradicts the L0 race-analysis sentence that local
   producer invocations are serialized by a SurfaceClient writer mutex. It is
   local-single-runtime relevant, but not a literal §7 blocker because the ACs
   and fixtures cover sequential render/manual reload behavior; the editor also
   disables the manual reload button while loading at
   `wp/dailyos/blocks/account-overview/edit.js:149-154`.
   AC-linked or path-alpha: path-α.
   → Linear maintenance: Serialize cold project-composition misses per composition

2. `src-tauri/src/surface_runtime/mod.rs:5317`
   What: fixture `dos671_external_version_advance_invalidates_cache` proves the
   route does not serve the stale key by observing `served_from_cache=false`,
   but it also asserts the old cache key remains present at
   `mod.rs:5371-5378`. That is acceptable for the current orchestrator API, yet
   it does not harden cache eviction or coalescing around external version
   advancement.
   Why it matters: this is fine for the L0 option-a keying contract, but it
   leaves future maintainers with a cache full of stale-but-addressable entries
   until TTL. Not a block; TTL is 60s and route lookup uses current DB version.
   AC-linked or path-alpha: path-α.
   → Linear maintenance: Add stale project-composition cache pruning or coalescing

3. `src-tauri/scripts/check_claim_writer_allowlist.sh:28`
   What: Packet B includes an allowlist expansion for
   `tests/fixtures/W6-A-meta-[0-9]+/state.sql`, plus unrelated Linear migration
   and signal-facade changes at
   `src-tauri/src/migrations/v178_dos_285_linear_issue_state.rs:16-19` and
   `src-tauri/src/services/linear_issue_signals.rs:98-144`.
   Why it matters: I found no AC regression in those changes, but they are
   outside Packet B's render-stabilization ownership and increase review
   coupling.
   AC-linked or path-alpha: path-α.
   → Linear maintenance: Split validation-gate rebase artifacts out of Packet B

## Axis results

- Axis 1: pass. Wrapper signature remains
  `dailyos_account_overview_render( array $attributes ): string` at
  `wp/dailyos/blocks/account-overview/render-functions.php:31`; `render.php`
  still calls it at `wp/dailyos/blocks/account-overview/render.php:24-25`;
  the direct fixture file is
  `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php`, with wrapper calls at
  `:53`, `:62`, `:83`, `:114`, `:233`, `:271`, and `:291`.
- Axis 2: pass. `reload` keeps live manual inputs at
  `wp/dailyos/blocks/account-overview/edit.js:43-94`; `reloadTrigger` is
  derived from account id plus composition-id presence at `edit.js:96`; the
  auto effect depends on `[ reloadTrigger ]` at `edit.js:97-102`; successful
  writes update only version/watermarks/token at `edit.js:72-80`.
- Axis 3: conditional pass with path-α note above. The AC key correction is
  present: current DB version read at `mod.rs:2336-2352`, lookup uses it at
  `mod.rs:2360-2362`, and store uses
  `projection.composition_version.unwrap_or(current_db_version_for_producer)`
  at `mod.rs:2443-2453`.
- Axis 4: pass. `authorize_local_render` delegates with
  `charge_ability_scope=false` at
  `src-tauri/src/bridges/surface_client.rs:277-291`; required-scope check runs
  before rate consumption at `surface_client.rs:363-374`; ability/scope
  candidates alone are gated by `charge_ability_scope` at
  `surface_client.rs:815-843`; identity buckets are unconditional at
  `surface_client.rs:786-813`.
- Axis 5: fail. See High findings 1-2.
- Axis 6: no additional AC-blocking cross-commit regression found beyond the
  PHP typed-error matrix failing to absorb runtime codes surfaced by the later
  Rust route/cache/decharge commit.

## Cycle 2 re-verify

1. §5.6 switch verdict: PASS — the targeted runtime codes are now handled
   before the default verification-banner fail-safe.
   Evidence: `src-tauri/src/surface_runtime/mod.rs:2247`,
   `src-tauri/src/surface_runtime/mod.rs:2255`,
   `src-tauri/src/surface_runtime/mod.rs:2268`, and
   `src-tauri/src/surface_runtime/mod.rs:2274` emit
   `project_composition_invalid`, `runtime_unavailable`,
   `project_composition_unknown_producer`, and
   `project_composition_invalid_id`.
   Evidence: `wp/dailyos/blocks/account-overview/render-functions.php:127`
   handles `runtime_unavailable`; `wp/dailyos/blocks/account-overview/render-functions.php:142-144`
   handles all three `project_composition_*` codes.
   Evidence: `src-tauri/src/surface_runtime/hmac.rs:794-802` emits
   `signature_invalid`, `canonicalization_mismatch`, `timestamp_stale`,
   `timestamp_future`, `key_not_found`, `key_rotated`, `token_invalid`,
   `nonce_replay`, and `transport_abuse_limited`.
   Evidence: `wp/dailyos/blocks/account-overview/render-functions.php:94`
   handles `transport_abuse_limited`; `wp/dailyos/blocks/account-overview/render-functions.php:117-124`
   handles the remaining signed-transport codes.
   Evidence: `src-tauri/src/services/surface_pairing.rs:264-280` emits
   pairing/session/scope codes including `unknown_runtime_anchor`,
   `session_invalid`, and `pairing_authority_unavailable`; those are handled
   at `wp/dailyos/blocks/account-overview/render-functions.php:97-115`.
   Evidence: `src-tauri/src/surface_runtime/mod.rs:3071-3076` emits
   `composition_version_overflow`; `wp/dailyos/blocks/account-overview/render-functions.php:152`
   maps it to the verification banner intentionally.
   Missing from switch: none from the requested literal/code ranges.

2. Fixture `typed_error_mapping_provider` verdict: PASS — fixture #14 covers
   the same targeted code set as the §5.6 switch.
   Evidence: `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:450`
   covers `transport_abuse_limited`.
   Evidence: `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:459-486`
   covers the pairing/session/signed-transport set, including
   `session_invalid`, `unknown_runtime_anchor`, `pairing_authority_unavailable`,
   `signature_invalid`, `canonicalization_mismatch`, `timestamp_*`, `key_*`,
   `token_invalid`, and `nonce_replay`.
   Evidence: `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:491`
   covers `runtime_unavailable`; `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:508-510`
   covers all three `project_composition_*` codes; `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:520`
   covers `composition_version_overflow`.
   Missing from fixture: none from the requested literal/code ranges.

3. Remaining path-α findings verdict: PASS — they remain filed as path-α
   maintenance notes and are not Packet B blockers.
   Evidence: the cold-miss serialization finding is marked path-α at
   `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md:72-90`,
   with non-blocking rationale at
   `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md:83-89`.
   Evidence: the stale cache pruning finding is marked path-α at
   `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md:92-103`,
   with non-blocking TTL/current-version rationale at
   `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md:99-102`.

Final verdict: APPROVE — commit `97db0de2` satisfies AC #14, and the two
remaining path-α items are already documented as non-blocking maintenance.
