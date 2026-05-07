# Wave W6 Proof Bundle

**Wave:** W6 (Validation + release gate — DOS-283 mock data, DOS-288 ownership validator, DOS-320 trust-band rendering, DOS-411 Tauri claim-backed lifecycle, DOS-412 ADR-0108 sensitivity audit, DOS-281 Golden Daily Loop release gate)
**Status:** W6 boundary cycle-15 APPROVE; DOS-281 cycle-3 closes acceptance criteria. Cycle-4 findings filed as v1.4.x follow-ups, not v1.4.0 blockers.
**Date:** 2026-05-07

---

## Initial wave landing

W6 expanded to no-deferrals scope on 2026-05-06 (`657aee7f`), pulling DOS-411 and DOS-412 from v1.4.1 follow-ups into v1.4.0 mandatory. Six tickets landed across `89a0124f` → `8afc41c3` → `6d3da85e` → `0d6d128b` → `138b1571` → `1b463201` covering bundle-1/5 mock-data patches, the `user_note` claim type with Tauri lifecycle cutover, the production ownership validator, per-field trust-band shape on the bridge envelope, the centralized `services::sensitivity::render_policy_for_surface` helper, and per-surface MCP/Tauri integration of the sensitivity affordance.

---

## L2 review closure — W6 boundary cycle chain

| Commit | Cycle | What |
|---|---|---|
| `74a3e234` | 1 | Ownership policy parameterization for `invoke_ability` + bundle-5 double-refresh resurrection regression |
| `09fd0b51` | 2 | Metadata-driven MCP rendering + ability data sensitivity layer |
| `91b9b2da` | 3 | Authoritative claim lookup helper + minimal-allowlist sanitization |
| `cabedc8a` | 4 | MCP boundary inverted to deny-by-default — class-level fix after repeated allow-list patches |
| `ff9b3419` | 5 | Stored-text invariant (only emit STORED bytes, never DTO bytes) + path-scoped JSON-pointer allowlist + diagnostics absence on MCP envelope |
| `74963003` | 6 | Static MCP renderer shares cycle-5 stored-text invariant via `verify_and_render_authoritative_claim` |
| `af19235b` | 7 | Tauri reveal path active+surfaced lifecycle gate |
| `981bd9e3` | 8 | `get_entity_context` wraps claim text in `RenderableClaimText` carriers for Tauri policy |
| `4a85a585` | 9 | Frontend reveal cache reset + `UnifiedTimeline` carrier preservation through `TimelineEntry` |
| `9cca58f1` | 10 | Reveal idempotency — synchronous `useRef` guard + per-carrier session id + audit `INSERT OR IGNORE` |
| `3baf82f7` | 11 | Backend-deterministic reveal idempotency (5-second time bucket) — class-level inversion after caller-bypass + carrier-swap leaks |
| `ec697fb5` | 12 | Caller UUID action token + forward-only migration repair + surface-aware cache — bucket boundary issues forced re-architecture |
| `a503c784` | 13 | Migration table-rebuild + UUID canonicalization (lowercase hyphenated) — closes ALTER abort + multi-form UUID duplicate rows |
| `c41d483d` | 14 | Function-based migration framework + retry-safe v144 with column inspection + token preservation |
| (no commit) | **15 APPROVE** | No material findings on the W6 boundary |

**Cycle 15 verdict on the W6 boundary: APPROVE.**

## L2 review closure — DOS-281 release gate cycle chain

DOS-281 landed after the W6 boundary closed and went through its own L2 ladder against the release-gate binary's specific acceptance criteria.

| Commit | Cycle | What |
|---|---|---|
| `b1a690fc` | landing | Release-gate binary + harness library extraction (`tests/harness/*` → `src/harness/*`); 16 tests; pnpm script |
| `8f5d020a` | 2 | Mandatory bundles hard-fail on any failure (FailSoft non-blocking only for tracked); harness/release_gate gated behind `release-gate` Cargo feature; cached evidence binds to git_sha + fixtures_hash; `HashOnly` newtype + typed enums on manual evidence; bundle-1/5/13 fixtures rebaselined against post-W6 substrate |
| `6410b393` | 3 | DOS-288 subprocess invocation includes `--features release-gate` (closes vacuous-pass bypass); `build.rs` embeds `BUILD_GIT_SHA` at compile time, `--git-sha` rejected if disagreeing with embedded value; `package.json` cargo separator restored |

**Cycle-3 closes DOS-281's acceptance criteria.** The L2 reviewer flagged 3 additional findings at cycle-4 (subprocess timeout absence, worktree-aware SHA watching, source-only no-git fail-fast) — none describe a path that breaks v1.4.0 acceptance in a real environment we operate in. Filed as v1.4.x follow-ups.

---

## Architectural decisions and pattern shifts

### Class-level fixes when L2 found multiple instances of one bug shape (cycles 4, 11, 12)

Cycle-4 found 3+ allowlist-style patches on the MCP rendering boundary; the class-level fix was inversion to deny-by-default. Cycle-11/12 found multiple bypass paths against caller-supplied audit idempotency (None forwarding, fixed-id replay, carrier-swap leaks); the class-level fix was first a backend-deterministic time bucket, then (after that surface had its own boundary issues) a backend-validated mandatory action token with canonicalization. Pattern: when 2+ similar findings recur, audit ALL channels into the boundary and centralize the gate rather than picking off one finding at a time. Memory: `feedback_systemic_look_for_recurring_issue_classes`, `feedback_enumerate_channels_before_patching`.

### Migration framework extension to support function-based migrations (cycle 14)

Cycles 12-13 patched the v144 migration SQL through three iterations (in-place rewrite, table-rebuild, UUID canonicalization). Cycle-14 found rebuild atomicity gaps that pure SQL couldn't safely close — partial-completion crash window + re-run token loss. The fix extended `Migration` to an enum (`Sql{version, sql}` | `Fn{version, apply: fn(&Connection)}`) and converted v144 to a Rust callback that opens `BEGIN IMMEDIATE`, inspects column existence, and rebuilds idempotently. Other 142 migrations stay as `Migration::Sql` unchanged.

### DOS-281 release-gate as the v1.4.0 ship criterion

The Golden Daily Loop is now an executable ship gate (`pnpm release-gate -- --mode hermetic`). Mandatory bundles 1, 5, 13 must pass; tracked 2-4, 6-12 are recorded but non-blocking unless they expose a 1/5/13 invariant. Manual mode opens dev DB read-only and validates submitted dogfood evidence shape. Evidence v1 is bound to compile-time git SHA + a deterministic SHA-256 hash over `tests/fixtures/bundle-*` so cached reports cannot be replayed against a different commit.

---

## Verification at wave tip

Run from `dev` at `30a58a93`:

- `cargo clippy --no-default-features -- -D warnings` — clean
- `cargo clippy --no-default-features --features test-harness -- -D warnings` — clean
- `cargo clippy --no-default-features --features release-gate -- -D warnings` — clean
- `cargo test --no-default-features --tests` — clean (2,286 unit + integration green)
- `cargo test --no-default-features --features test-harness --tests` — clean
- `cargo test --no-default-features --features release-gate --tests` — clean (release_gate_dos288_subprocess_test confirms gate fails on forced bleed regression)
- `pnpm tsc --noEmit` — clean
- `pnpm test` — 29 files / 204 tests green
- `pnpm release-gate -- --mode hermetic` — exit 0; bundles 1, 5, 13 all `pass`; both DOS-288 selectors `pass`; evidence written to `src-tauri/target/release-gate/evidence.json` + `evidence.md`

## Outstanding work for v1.4.0 ship (not automatable)

- Manual dogfood evidence: `pnpm release-gate -- --mode manual --db ~/.dailyos/dailyos-dev.db --manual-evidence path/to/manual.json` against a captured ≥20-real-meeting run, verifying DOS-411 claim-backed Tauri lifecycle and DOS-412 ADR-0108 sensitivity rendering work end-to-end on a dev workspace.
- Release checklist walked by maintainer; UI validation hands-on (trust-band rendering, prep grid layout, `ClaimTextRenderer` reveal flows).
- v1.4.0 tag on `trunk` after the above and a `dev` → `trunk` merge.

## v1.4.x follow-ups filed during the wave

- DOS-281 subprocess timeout in `run_dos288_selector` (CI hardening)
- `build.rs` worktree-aware SHA watching (linked-worktree contributor support)
- `build.rs` source-only no-git fail-fast (CI tarball flow that does not currently exist)
- Trust-band UI: `BriefingMeetingCard.prepItemBody` indicator-on-new-line cleanup (separate from the W6 substrate work)
