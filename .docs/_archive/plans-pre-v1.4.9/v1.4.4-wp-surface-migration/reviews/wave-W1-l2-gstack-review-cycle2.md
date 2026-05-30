# L2 (Diff) gstack `/review` — v1.4.4 W1 Wave (Cycle 2)

**Date:** 2026-05-20
**Reviewer:** gstack `/review` skill (re-review of cycle-1 blockers + cycle-2 substrate scan)
**Branch:** `wave/v1.4.4-w1-stage1a`
**Base ref:** `0f8533e1`
**Diff range:** `0f8533e1..HEAD` — 138 files, 21,284 insertions, 51 deletions, 25 commits
**Cycle-2 patches under review:**
- `c2e857b8` — feat(w1): L2 cycle-2 consumer skeletons + CI lint wiring (AC-W1.2 + AC-W1.9)
- `0a586218` — fix(w1): L2 cycle-2 substrate patches — DOS-339/341/477 wiring + bugfixes
- `446fc640` + `9d78f07b` — worktree-agent merges
- `f82d53ce` — docs(v1.4.4 W1): L2 cycle-1 verdicts (3 reviewers)

**Bounding:** per `feedback_l2_must_review_against_acceptance_criteria` + path-α gate. Re-verified cycle-1 HIGH findings against the cycle-2 patch surface. New findings only surfaced if they meet L2 blocker criteria (literal AC violation, ADR-named contract violation, regression in PR-touched code).

**Active verification this cycle:**
- `cargo check` — green (33.89s, no warnings).
- All 4 lint scripts executed locally — all pass (exit 0).
- `bash src-tauri/scripts/check_w1_consumer_skeleton.sh` → `AC-W1.9 PASS: every W1 producer has at least one downstream WP block consumer skeleton.`
- Render-functions.php files grepped — all 5 named W1 producers (`get_entity_intelligence`, `get_daily_briefing`, `claim_receipt`, `meeting_prep_status`, `record_claim_feedback`) have at least one consumer block invocation.
- CI workflow file `.github/workflows/lint-frontend.yml` lines 121–131 — 4 lint-script steps named & wired.
- New substrate modules (`event_bridge.rs`, `envelope_cache.rs`, migration `243_…sql`, `render.rs` 304-line reshape) read in full for regression patterns.
- `PrivacyError` → `RenderError` mapping at `render.rs:122-141` verified exhaustive over all 6 enum variants.

---

## VERDICT

**APPROVE.** All three cycle-1 HIGH blockers are resolved by the cycle-2 patches. Cycle-2 introduced ~700 LOC of net-new substrate (`event_bridge.rs`, `envelope_cache.rs`, `render.rs` reshape, migration v243) — scanned for regressions in PR-touched code per `feedback_l2_must_review_against_acceptance_criteria`. No new L2-blocker findings surfaced. The wave clears L2 fully.

Cycle-1 path-α residuals (Findings 4–10 in cycle-1 verdict) remain in scope for the maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — not re-litigated here.

---

## Cycle-1 Finding Resolution Status

| Finding | Severity | AC anchor | Cycle-2 patch | Status | Evidence |
|---|---|---|---|---|---|
| 1 | HIGH | AC-W1.9 — `check_w1_consumer_skeleton.sh` missing | `c2e857b8` | ✅ RESOLVED | Script exists at `src-tauri/scripts/check_w1_consumer_skeleton.sh` (80 LOC), wired at `.github/workflows/lint-frontend.yml:131` as named step "Enforce v1.4.4 W1 consumer-skeleton wiring (AC-W1.9)"; local execution exits 0 with PASS message; negative-fixture harness present at `*.test`. |
| 2 | HIGH | AC-W1.2 — no downstream consumer skeletons | `c2e857b8` | ✅ RESOLVED | 4 block render-functions.php files added (`account-detail`, `project-detail`, `person-detail`, `daily-briefing`); all 5 W1 producer identifiers covered (`get_entity_intelligence` in 3 entity-detail blocks; `get_daily_briefing` + `meeting_prep_status` + `claim_receipt` + `record_claim_feedback` in `daily-briefing` and `account-detail`). Lint passes. |
| 3 | HIGH | AC-340.2 / AC-340.7 / AC-477.11 — lint scripts not wired in CI | `c2e857b8` | ✅ RESOLVED | All 3 cycle-1-named scripts now wired as separate named workflow steps at `.github/workflows/lint-frontend.yml:121-128`; AC anchors named in step titles. Local execution: all 3 scripts exit 0. |
| 4 | MEDIUM | AC-477.2 — Tauri command envelope tautology | `0a586218` | ✅ RESOLVED (substrate added; path-α fallback documented) | New `envelope_cache.rs` (200 LOC) provides server-side envelope cache; `claim_feedback.rs` now accepts `envelope_render_id`, looks up cached envelope, falls back to single-claim shape on miss with logged warning + explicit `path_alpha_envelope_cache_v2` marker. W2 wiring path is explicit. |
| 5–10 | MEDIUM / LOW / Informational | path-α only | — | DEFERRED to maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — not re-reviewed this cycle. |

Additional cycle-1 findings from sibling code-reviewer track that cycle-2 also addressed:
- **DOS-341 AC-341.10 wiring** — `render_receipt_for` now dispatches through `build_receipt_for_audience` rather than rendering directly; was a code-reviewer cycle-1 F1 finding. Verified at `claim_receipt/render.rs:55-115`. `PrivacyError`→`RenderError` transport via tag prefixes; exhaustive over all 6 variants.
- **DOS-339 signal→event bridge** — new `claim_receipt/event_bridge.rs` (148 LOC) wires `ClaimVerificationStateChanged` signal to `claim_receipt:invalidated` Tauri event; 3 unit tests assert camelCase serialization, optional field omission, no-op on uninstalled handle. Was a codex review cycle-1 P1.
- **DOS-335 v241 view non-determinism** — new migration `243_…sql` aggregates `meeting_entities` to one row per meeting via per-meeting MIN aggregation. Was codex review cycle-1 P2.

---

## New Findings (cycle-2 substrate scan)

**None at L2-blocker tier.**

Scanned surfaces:
- `src-tauri/src/services/claim_receipt/event_bridge.rs` (148 LOC, new module) — best-effort emit, OnceLock pattern matches established `IDEMPOTENCY_CACHE` convention, no panic paths, unit tests cover absence + serde rename + Option skip.
- `src-tauri/src/services/entity_intelligence/envelope_cache.rs` (200 LOC, new module) — `parking_lot::RwLock` (already in deps), 5-min TTL, 2,048 cap, evict-on-read, `path_alpha_envelope_cache_v2` marker fn present per cycle-1 commitment.
- `src-tauri/src/services/claim_receipt/render.rs` (+304 LOC reshape) — clean dispatch through privacy layer; tag-string round-trip across `db_read`'s `Result<T, String>` boundary; mapping exhaustive over all 6 `PrivacyError` variants.
- `src-tauri/src/commands/claim_feedback.rs` (+114 LOC) — accepts `Option<String>` for `envelope_render_id`, falls back to logged-warning single-claim path on cache miss (cycle-1 behaviour preserved). New `ResolvedEnvelope` enum encapsulates cached vs fallback.
- `src-tauri/src/migrations/243_meeting_prep_status_indexed_view_deterministic.sql` (59 LOC) — `DROP VIEW IF EXISTS` + deterministic `MIN()` aggregation; matches v241's column list 1:1; SQLite-compatible composition (two grouped aggregates since SQLite lacks tuple MIN).
- `.github/workflows/lint-frontend.yml` (+12 lines) — four separate named steps, no shell-injection surface, executable invocations only.

Path-α observations from substrate scan (file to maintenance, NOT re-reviewed):
- `envelope_cache.rs` cap of 2,048 entries + 5-min TTL: hard limits, no LRU pressure metric. Acceptable for v1.4.4 W1; revisit with W2 producer wiring.
- `event_bridge.rs` AppHandle global OnceLock: documented pattern parity with `IDEMPOTENCY_CACHE`. Architectural debt (substrate-wide AppState threading would refactor both) is acknowledged in the module doc.

Both are path-α per `feedback_l2_path_alpha_to_maintenance_project` and outside L2 cycle-2 blocker scope.

---

## ACs verified met (delta over cycle-1)

In addition to all ACs verified in cycle-1 verdict, cycle-2 closes:
- **AC-W1.2** — at least one downstream WP block consumer skeleton ships per W1 producer.
- **AC-W1.9** — `check_w1_consumer_skeleton.sh` exists, wired in CI, lint passes locally.
- **AC-340.2 / AC-340.7 / AC-477.11** — three named lint scripts wired into `.github/workflows/lint-frontend.yml` as separate named steps with AC anchors in titles.
- **AC-341.10 production wiring** — `render_receipt_for` dispatches through `build_receipt_for_audience`; per-audience field allowlists govern every rendered receipt.
- **AC-339 signal→event bridge** — `ClaimVerificationStateChanged` signal now reaches `useClaimReceiptSubscription` hook via Tauri event.

---

## Recommendation

**APPROVE for merge to `dev`.** Wave clears L2 fully. Path-α residuals from cycle-1 (Findings 4–10) + new substrate caveats (envelope cache cap, AppHandle global) route to the Codebase Maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — they do not block this PR.

Proceed to L3 (wave-scoped retro + `/ce-compound` K-out per `.docs/plans/engineering-ladder.md`).
