# W3 wave-bundle session handoff (v2)

Date: 2026-05-14
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Wave: W3 (DOS-563 W3-0 spike, DOS-564 W3-A skeleton, DOS-565 W3-B transport, DOS-566 W3-C MCP)

Supersedes the original handoff at `W3-handoff.md` (`eae1c267`). That session resumed from L3 cycle-1 and drove the wave through L3 closure, the L0 V5 amendment, end-to-end DOS-605 architectural fix, and partial L4 verification.

## Where we are

PR #274 is **MERGEABLE** (UNSTABLE only because Rust CI is mid-run on the latest push). All other 11 checks green. Working tree clean. Branch is 1 commit behind dev (`2b4d3ffc` — non-conflicting maintenance batch).

L3 closed unanimously across 4 reviewer cycles. L4 surface QA partially complete — the V5 signed-surface protocol is validated end-to-end (refresh endpoint returns hmac_key, MCP `tools/list` returns the dailyos ability, settings page renders); the final MCP `tools/call` invoke is blocked by an unrelated runtime DB-lock issue tracked as DOS-610.

### Branches + commits

| Ref | Commit | State |
|---|---|---|
| `dev` (public/dev) | `2b4d3ffc` | maintenance batch #272 just merged on dev; doesn't conflict |
| `dos-546-w3-wordpress-foundation` (public) | `1cfbc0ce` | 16 commits on branch + merge with dev. CI mid-Rust. |

### Branch commit log (newest first)

```
1cfbc0ce DOS-565 W3-B: store pairing wp_user_id in marker for cross-user signed requests
f9726ce1 Merge public/dev into dos-546-w3-wordpress-foundation
32926af6 fix: dos7_d5 test uses run_migrations() instead of inline schema
56083953 fix(W3): bump migration count assertions for v171
98654fd0 chore(W3): rewrite migrations.rs comment to drop ephemeral issue ref
5d8c3799 Merge public/dev into dos-546-w3-wordpress-foundation; renumber W3 migration to v171
2c6a0459 DOS-605 W3 V5: drop bearer auth + add /v1/surface/session/refresh
792f3275 DOS-566 W3-C: phpcs:ignore for custom-cap user_can check in credential store
f10e3888 DOS-564/565/566 W3 L4 fold: ability registration + scope resolver + DOS-599
43171c82 DOS-566 W3-C: instantiate McpAdapter at bootstrap so the REST route registers
0fceabc4 DOS-565 W3-B: omit empty multisite_blog_id from pairing handshake body
95568aac DOS-546 W3 L3 cycle-4 closure: codex challenge APPROVE — L3 unanimous
e64a7ecd DOS-564 W3 L3 cycle-4 fold: align marker shape gates with real runtime output
1714917f DOS-564 W3 L3 cycle-3 fold: expand namespace vacancy + marker format gates
497dc109 DOS-565 W3 L3 cycle-2 fold: align signer timestamp to RFC3339-Z
eae1c267 DOS-546 W3 handoff doc — session checkpoint for next-session pickup
68e1ed9a DOS-564/565/566 W3 L3 fold: complete 5-fix implementation polish
```

### CI

| Workflow / job | Status |
|---|---|
| L2 (3 checks) | ✅ pass |
| WP Plugin: Lint + PHPStan + audit + grep-gates | ✅ pass |
| WP Plugin: PHPUnit (8.1, 8.2, 8.3, 8.4) | ✅ pass |
| WP Plugin: WordPress Plugin Check | ✅ pass |
| frontend | ✅ pass |
| lint-and-checks (durable-comments + render-policy + canonicalization-drift) | ✅ pass |
| **rust** | 🟡 in progress (~30 min) |

## Ladder closure

| Layer | State | Notes |
|---|---|---|
| L0 | **CLOSED V4 + V5 amendment** | V5 amendment landed in `W3-L0-packet.md` line 106: bearer dropped, HMAC-only auth, `/v1/surface/session/refresh` endpoint defined. |
| L1 | **PASSING** | All gates green locally. |
| L2 | **CLOSED Cycle 2** | 3-of-4 reviewer APPROVE + codex CONDITIONAL re-classified. Findings filed as maintenance DOS-585..588. |
| L3 | **CLOSED Cycle 4 (unanimous)** | architect / Suite E / Suite P / Suite S / codex challenge all APPROVE. Verdict artifacts at `.docs/plans/dos-546/v1.4.2-project/W3-L3-rerun-*.md`. |
| L4 | **PARTIAL** | See "L4 status" below. |

## L4 status

| Item | Status |
|---|---|
| Plugin activates | ✅ |
| Pair via WP admin → DailyOS handshake | ✅ |
| Revoke + re-pair | ✅ |
| Runtime-restart recovery | ⏸ deferred per DOS-604 — local-only single-user design question |
| MCP REST endpoint mounted (`/wp-json/dailyos/v1/mcp`) | ✅ |
| MCP `initialize` returns session ID | ✅ |
| MCP `tools/list` returns `dailyos-account-overview` | ✅ |
| `/v1/surface/session/refresh` returns hex hmac_key | ✅ — V5 protocol validated |
| MCP `tools/call` end-to-end | ⏸ blocked by DOS-610 — runtime `Self::Write` swallows `database is locked` as `pairing_authority_unavailable`. Not a W3 contract issue. |
| Settings page renders (instance ID + scopes + endpoint + last-use) | ✅ — 841 bytes HTML, all marker fields surface correctly |
| HMAC redaction in `WP_DEBUG_LOG` | ⏳ not exercised — W3-B `__debugInfo` / `__toString` / `jsonSerialize` should redact; assert with a deliberate dump |
| `wp dailyos status` CLI | ⏳ not exercised |
| `wp dailyos repair-namespace` CLI | ⏳ not exercised |

**The three ⏳ items are quick manual checks (~5 min each).** None should block merge — they're verification artifacts for the wave proof bundle, not contract validation.

## What's done

### DOS-605 — closed by this PR

Removed bearer from the signed-surface auth path; HMAC alone authenticates. New `POST /v1/surface/session/refresh` endpoint returns the derived per-session `hmac_key` given a matching identity bundle. WP filter `dailyos_wp_bridge_session_key` wraps this and provides session material per-request without persistence.

L0 V5 amendment captured the protocol change. Migration v171 drops the `surface_client_sessions.bearer_token_hash` column.

### DOS-608 — closed by this PR

`dos7_d5_ghost_resurrection_test.rs` now uses `run_migrations()` instead of a hand-curated inline schema. Removes ~232 lines of stale SQL and immunizes the test against future migration drift.

### DOS-565 wp_user_id binding — closed by this PR

The runtime binds `wp_user_hash` at pairing time to whatever `wp_user_id` signed the handshake. MCP invocations later run as the substrate user (per L0 W3-C), so the WP signer must always present the paired `wp_user_id`, not the request-time `current_user_id`. Fix: marker now stores `paired_wp_user_id`; `canonical_identity()` reads from there.

### Migration slot

Per CLAUDE.md parallel-wave reservation rule, W3 should have claimed a slot block in `.docs/plans/v1.4.2-waves.md`. That file doesn't exist; W3 + W4-B both took v170 independently. Resolved by renumbering W3 to **v171** during merge conflict resolution. Filing wave-plan gap as DOS-607.

## Open / accumulated path-α (Linear maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`)

Carried from prior cycles:

- DOS-585 — DX "this was my edit" attribution for Studio wp_cli mutations (Low)
- DOS-586 — Forged-marker + clean-runtime → quarantine negative fixture (Medium)
- DOS-587 — Unauthenticated REST schema introspection negative fixture (Medium)
- DOS-588 — Bearer-token credential retrieval evaluation gate (Low) — partially superseded by V5 amendment

Filed during this session:

- DOS-595 — actor_instance non-empty validation (Low)
- DOS-596 — runtime URL filter memoize (Low)
- DOS-597 — substrate user adoption ownership (Medium)
- DOS-598 — HMAC trim charset alignment (Low)
- DOS-599 — substrate user session-key gate (High; partially addressed; full fix in DOS-605)
- DOS-600 — marker runtime-attestation deeper hardening (Medium)
- DOS-601 — substrate emit `projection_version` (Medium)
- DOS-602 — clear PHPStan baseline (Low)
- DOS-603 — WordPress connector tile in DailyOS Settings (Medium)
- DOS-604 — reconsider runtime-restart invalidation for local-only port-to-port (Medium)
- DOS-606 — e2e MCP integration test (Medium)
- DOS-607 — claim v1.4.2 migration slot blocks (Medium)
- DOS-609 — UI shows stale pairings as Active after runtime restart (Medium)
- DOS-610 — `SurfacePairingError::Write` swallows `database is locked` as `pairing_authority_unavailable` (High)

Closed by this PR:

- DOS-605 (session credential retrieval mechanism)
- DOS-608 (dos7_d5 inline schema drift)

## Environment

### Local repo

- **Path:** `/Users/jamesgiroux/Documents/dailyos-repo`
- **Remote `public`:** `git@github.com:jamesgiroux/daily-operating-system.git`
- **PR:** https://github.com/jamesgiroux/daily-operating-system/pull/274

### Worktree

| Path | Branch |
|---|---|
| `/private/tmp/dailyos-w3-0` | `dos-546-w3-wordpress-foundation` |

### Studio dev site

- **Path:** `~/Studio/dailyos-dev`
- **URL:** `http://localhost:8884`
- **WP / PHP:** **7.1-alpha-62359** / 8.4 — upgraded mid-session via `studio site set --wp=nightly` because the WordPress AI plugin (v0.9.0) requires WP 7.0+ for `wp_supports_ai()`.
- **DailyOS plugin:** symlinked at `~/Studio/dailyos-dev/wp-content/plugins/dailyos` → `/private/tmp/dailyos-w3-0/wp/dailyos`, active
- **WordPress AI plugin (Abilities Explorer):** cloned from GitHub at `~/Studio/dailyos-dev/wp-content/plugins/ai`, built (`npm ci && npm run build`), active. `wpai_features_enabled` option seeded with `abilities_explorer => true`. Access via WP Admin → Tools → Abilities Explorer.
- **Plugin activated?** Yes (DailyOS) and Yes (AI).
- **Pairing state:** paired (marker carries `paired_wp_user_id=1`, session_id `sess_6c15c102070840b1a9cd7dc4536faf37`, runtime_url `http://127.0.0.1:53722`).

### Tauri runtime

- DailyOS Tauri app running from the W3 branch via `pnpm dev` (or whichever launcher).
- Runtime version: includes V5 protocol (bearer dropped, refresh endpoint, migration v171 applied).
- Runtime DB at `~/.dailyos/dailyos-dev.db` (SQLCipher).

### Substrate user app password (for MCP curl probes)

- User: `dailyos_substrate` (user_id 2)
- App password: `q7JvdHELQSSM0Dv3ninlNZJu` (label: "L4 MCP probe")
- WP admin app password (for admin-context probes): `c7OOHAdAMZl2k1xQAsFLoKPA` (label: "L4 settings probe")

## What's next

### Option A — finish L4 verification (recommended)

Three ⏳ items left, all quick:

1. **HMAC redaction in `WP_DEBUG_LOG`.** Enable `WP_DEBUG_LOG` in `~/Studio/dailyos-dev/wp-config.php`, then run a `wp eval` that deliberately dumps a `DailyOS_Session_Credential` and `DailyOS_Hmac_Key` via `var_dump`, `print_r`, `json_encode`. Confirm log file shows `***REDACTED***` not raw bytes.
2. **`wp dailyos status`.** Run `studio wp --path=/Users/jamesgiroux/Studio/dailyos-dev dailyos status`. Should report pairing status, instance ID, last-use, granted scopes.
3. **`wp dailyos repair-namespace`.** Run `studio wp --path=/Users/jamesgiroux/Studio/dailyos-dev dailyos repair-namespace`. Should report namespace state and any dirty items. Safe to run; doesn't delete on a clean install.

Then mark L4 complete and move to merge.

### Option B — unblock MCP `tools/call`

Fix DOS-610: differentiate `database is locked` (transient retry) from `pairing_authority_unavailable` (permanent) in `surface_runtime/mod.rs::signed_transport_response`, OR add retry-with-backoff for `SQLITE_BUSY` on the `validate_signed_session` write path. Once landed, `tools/call` end-to-end demonstrates the full V5 chain. Not in PR #274 scope; new PR.

### Option C — merge PR #274 now

Rust CI lands → merge. DOS-610 is runtime infrastructure unrelated to W3 protocol. L4 ⏳ items don't block; they verify documentation/CLI/redaction invariants that the code already enforces.

Recommendation: **A then C**. Run the three quick L4 checks (10-15 min total) for clean wave proof bundle, then merge.

## Resume protocol

When resuming, verify in this order:

1. `cd /private/tmp/dailyos-w3-0 && git status --short` — should be clean
2. `cd /private/tmp/dailyos-w3-0 && git log --oneline -3` — top should be `1cfbc0ce DOS-565 W3-B: store pairing wp_user_id in marker`
3. `gh pr checks 274 | head -15` — confirm Rust state (should be ✅ pass by now)
4. `gh pr view 274 --json mergeStateStatus` — should be CLEAN (after Rust passes)

If `mergeStateStatus` is DIRTY/CONFLICTING again, merge `public/dev` into the branch using the conflict-resolution memory rules (`feedback_l0_reconcile_against_dev`).

If CI is still pending or failed: check `gh run view --job <id> --log-failed`. The most-recent failure modes seen this session were:
- Transient `rustup-init: error: unexpected argument 'test'` — retry via `gh run rerun <id> --failed`
- `database is locked` test failure in `dos7_d5` — already fixed via `run_migrations()` refactor

## Key file references

- `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md` — V4 + V5 amendment (authoritative acceptance contract)
- `.docs/plans/dos-546/v1.4.2-project/W3-L3-rerun-*.md` — reviewer verdicts cycle-1 through cycle-4
- `.docs/plans/dos-546/v1.4.2-project/W3-handoff.md` — original (cycle-1) handoff; superseded by this doc
- `wp/dailyos/` — plugin source
- `wp/dailyos/includes/class-dailyos-plugin.php:158` — `dailyos_wp_bridge_session_key` filter wiring refresh endpoint
- `wp/dailyos/includes/transport/class-dailyos-credential-store.php` — V5 session material shape (no bearer)
- `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:485` — `canonical_identity` reads `paired_wp_user_id` from marker
- `src-tauri/src/surface_runtime/mod.rs:1041` — `surface_session_refresh_response` handler
- `src-tauri/src/services/surface_pairing.rs:841` — `verify_session_refresh_identity`
- `src-tauri/src/migrations/171_dos_565_drop_surface_bearer_token_hash.sql` — V5 migration
