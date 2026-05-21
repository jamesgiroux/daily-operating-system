# L2 (Diff) — gstack `/review` — v1.4.4 W1+W2 wave, cycle 1

**Reviewer:** gstack `/review` (Claude Opus 4.7 1M + codex cross-model)
**Branch:** `wave/v1.4.4-w1-stage1a` @ `bdade2cc`
**Base:** `0f8533e1` (V1.1 packets commit; wave forked here)
**Diff range:** `0f8533e1..bdade2cc` — 418 files, +43,179/-81
**Scope:** W1 already L2/L3 unanimous at `e3907a63`; this cycle reviews W1 extensions (×3) + W2 surfaces (4 outer composites + 61 inner blocks + 3 list shells + 2 metadata-proposal blocks + 6 primitive folds)
**Bounding (per memory `feedback_l2_must_review_against_acceptance_criteria` + path-α gate):** L2 blockers limited to AC violations, ADR-named contract violations, regressions in PR-touched code. Theoretical hardening / dormant edges → path-α (maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).

---

## VERDICT: **BLOCK** — 1 CRITICAL regression of L3 cycle-2 F3 fix (v245 lacks `BEGIN IMMEDIATE/COMMIT` wrapper that v244 was created to introduce)

Wave is otherwise structurally correct against W2 §10 invariants. The lone blocker is mechanical and ~5-line fix. Cross-model codex confirms the finding and the recommendation.

---

## Findings

### F1 — CRITICAL — Migration v245 regresses the v243→v244 transactional fix (BLOCKER)

**Severity:** CRITICAL — same-shape defect class as L3 cycle-2 F3 (DOS-335 v244)
**File:** `src-tauri/src/migrations/245_dos_484_feedback_merge_intent.sql:19-59`
**Contract cite:** comment block at `src-tauri/src/migrations.rs:950-957` documents the v243→v244 fix: `conn.execute_batch(sql)` at `migrations.rs:3601` does NOT implicitly wrap a batch in a transaction; multi-process readers can observe the gap between destructive statements; v244 was added explicitly to wrap the rebuild in `BEGIN IMMEDIATE; ... COMMIT;`.
**Defect:** v245 rebuilds `claim_feedback` via `CREATE TABLE claim_feedback_new` → `INSERT … SELECT` → `DROP TABLE claim_feedback` → `ALTER TABLE claim_feedback_new RENAME TO claim_feedback` → `CREATE INDEX IF NOT EXISTS …` with NO `BEGIN IMMEDIATE/COMMIT` wrap. The migration's own comment ("Safe across multi-process readers: SQLite serializes table-rebuilds at the writer lock") is **false** — serialization holds per-statement, not across the multi-statement batch.
**Failure mode:**
  1. Multi-process: a reader connection between `DROP TABLE claim_feedback` and `ALTER TABLE … RENAME TO claim_feedback` sees `no such table: claim_feedback` on any prepared statement against the canonical name (claim feedback is the primary write path for ADR-0123, used across the W2 wave's per-claim feedback affordances).
  2. Crash recovery: a crash between `DROP` and `RENAME` leaves `claim_feedback_new` populated but `claim_feedback` missing — the schema_version row is also not written (it's a separate `conn.execute` at `migrations.rs:3635`), so the next start retries the migration, fails on the existing `claim_feedback_new`, and refuses to boot.
**Why this isn't path-α:** This is a regression of an already-shipped fix (L3 cycle-2 F3 verdict was unanimous APPROVE on the v243→v244 patch at `e3907a63`). Per CLAUDE.md "Same-shape findings twice = class-wide sweep, not a third patch" — but this is the recurrence, the sweep IS to wrap v245 + audit v246+. Per memory `feedback_systemic_look_for_recurring_issue_classes` and `feedback_zoom_out_for_class_pattern_in_l2_loop`, this is the structural gate not a one-off patch.
**Recommended fix:** Wrap the v245 body in `BEGIN IMMEDIATE;` … `COMMIT;` exactly as v244 does. Update the misleading "Safe across multi-process readers" comment. Add a class-level gate (greppable CI check) that every migration with `DROP TABLE` / `DROP VIEW` / `ALTER TABLE … RENAME` contains a `BEGIN IMMEDIATE` token before the first destructive statement.
**Cross-model:** codex independent pass returns "**Recommendation: BLOCK because v245 reintroduces the exact untransactional destructive migration window v244 was created to close.**"

---

### F2 — PATH-α — Unsigned list cursor allows offset forgery (NON-BLOCKING — file to maintenance)

**Severity:** PATH-α — explicit `v1.4.6/v1.4.7` deferral in code + packet
**File:** `src-tauri/abilities-runtime/src/abilities/list_pagination.rs:7-15, 30-53` + `get_entity_intelligence/contracts.rs:113`
**Contract cite:** `L0-packet-W1-substrate-gaps.md:1334` (V1.1 §13 Q11 resolution): "Residual key-management concern (where the cursor-signing key lives, rotation policy) is correctness R1 — flag to v1.4.6/v1.4.7 if cursor signing key becomes a security-token concern, not a substrate change for W1."
**Defect:** Cursor is base64-encoded JSON `{offset, watermark}` with no HMAC. Client can forge any `offset` value as long as `watermark` matches the current request fingerprint. The watermark check at `list_accounts/producer.rs:52` defends against filter-shape spoofing but not offset-skip. In local-only WP-surface deployment (v1.4.4 W2 is loopback HTTP per memory `feedback_wp_is_local_surface_not_remote`), the attacker model is "client app code injecting a forged cursor"; impact is pagination skip/seek, not row access (every row in scope of the filter is already authorized for the principal at the watermark).
**Why path-α:** Code comment explicitly cites v1.4.6/v1.4.7 substrate work for HMAC signing. V1.1 W1 packet explicitly resolves this as not-a-W1-substrate-change. ADR-0123 / DOS-746 (per memory `feedback_canonical_signing_changes_invalidate_pairings`) is the substrate track for HMAC canonical signing changes.
**Recommended fix:** File maintenance ticket against project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` titled "v1.4.6/v1.4.7 — HMAC-sign opaque pagination cursor (`list_pagination::encode_cursor`)". Reference DOS-746 substrate track. Cross-link from `list_pagination.rs:7-10` doc comment.

---

### F3 — PATH-α — v242 missing transaction wrap (NON-BLOCKING, but file alongside F1 sweep)

**Severity:** PATH-α — purely additive, no destructive multi-process window
**File:** `src-tauri/src/migrations/242_meeting_prep_status_dismissals.sql`
**Contract cite:** same as F1 — `migrations.rs:3601` does not auto-wrap
**Defect:** v242 is `CREATE TABLE IF NOT EXISTS` + `CREATE INDEX IF NOT EXISTS` — purely additive. A concurrent reader could observe the new table before its indexes finish, which is cosmetic (no missing pre-existing object). v241 is even more clearly safe (single `CREATE VIEW IF NOT EXISTS`).
**Why path-α:** Codex independent analysis: "Wrapping v242 would be cleaner, but the specific v244 multi-process concern is not materially present there." No ADR or AC cites force transactional wrapping for purely additive DDL.
**Recommended fix:** When the F1 sweep introduces the class-level CI gate, define the gate to fire ONLY on migrations containing `DROP TABLE` / `DROP VIEW` / `ALTER TABLE … RENAME` — additive `CREATE … IF NOT EXISTS` is allowed to skip. Add v242 to a follow-up "wrap-for-clarity" maintenance ticket if desired but do not block the wave.

---

## Wave-scope invariant audit (W2 §10 + V1.2 named gates)

All five invariants the requester asked me to check are PASSING:

1. **3-arg `invoke_ability($producer, $payload, $scope_set)`** — `bash src-tauri/scripts/check_w1_consumer_skeleton.sh` returns `AC-W1.9 PASS`. Manually verified outer composites (account-detail, project-detail, person-detail, meeting-detail), inner consumer hooks (`dailyos_meeting_detail_claim_inner_consumer` et al), and metadata-proposal blocks all use the 3-arg form. No 2-arg regressions.
2. **`dailyos-empty-chip` emission per §10 (NEVER silent-hidden)** — every inner block projecting envelope sections emits either `dailyos-empty-chip` or `dailyos-chrome-empty` (shared helper at `wp/dailyos/blocks/_shared/chrome/render-empty.php`) or `is-unavailable` chrome on null-payload branches. The 5 primitives lacking the literal token (TrustBandBadge, TrendStrip, Pill, HealthBadge, IntelligenceQualityBadge) always render visible content with a default band fallback — they have no null-render branch by design, satisfying "no silent hidden states."
3. **Inline styles ONLY `--dailyos-*` custom properties** — `bash src-tauri/scripts/check_no_inline_style_exception.sh` returns `AC-W2.7 PASS`.
4. **AgentMcp audience filter for claim-bearing blocks** — verified in `metadata-proposal-cue/render-functions.php:74-83`, `account-detail/render-functions.php:157`, all 13 project-detail inner blocks' header docstrings cite `build_receipt_for_audience` (DOS-341), and `touchpoints-feed/render-functions.php:75-83` carries the AgentMcp aggregate-only branch (`agent_mcp === audience` → aggregate render). Substrate-side filter lives in `src-tauri/src/services/claim_receipt/{privacy,render}.rs` (PR-touched, retains DOS-341 contract).
5. **`envelopeHandle` composition** — outer composites emit handle via `dailyos_envelope_handle_from_response()` + `providesContext: { "dailyos/envelopeHandle": "envelope_handle" }` + `$GLOBALS['dailyos_envelope_handle_for_request']`; inner blocks consume via `$block->context['dailyos/envelopeHandle']` with `$GLOBALS` fallback, then `dailyos_resolve_envelope($handle, $entity_type, $entity_id, $scope_set)`. Pattern is consistent across all 4 outer composites.

**W1 extensions ×3 — compose cleanly with existing substrate:**
- Meeting `EntityKind` (`87df7cf6`) — extends `get_entity_intelligence` envelope; matched in `meeting-detail/render-functions.php:74-81`; no regression of the L3 cycle-2 F2 touchpoint audience filter at the consumer.
- `FeedbackAction::MergeIntent` 10th variant (`01d0cff3`) — feedback.rs additive enum + render policy `Default` bucket + JSON round-trip tests at `feedback.rs:445-452`. ADR-0123 V1.1 amendment cited inline. **Migration v245 is the defect (F1 above) — the Rust enum extension itself is clean.**
- `list_accounts/list_people/list_projects` Read abilities (`b8625a9d`) — shared `list_pagination.rs` opaque cursor + watermark; consumers in `wp/dailyos/blocks/{accounts,people,projects}-index/view.js` use `wp.dailyosShared.useAbilityCursor`. F2 path-α applies.

**Tests:** WAKEUP.md commit `bdade2cc` reports `cargo test 2648/0`. Not re-run by reviewer (deferred to acceptance gate).

---

## Recommendation summary

1. **Fix F1 in one ~5-line edit** — wrap v245 body in `BEGIN IMMEDIATE; … COMMIT;` exactly as v244, update the misleading "Safe across multi-process readers" comment to point at the wrap. Commit message should explicitly cite "regression of L3 cycle-2 F3" to make the K-out searchable. Memory `feedback_zoom_out_for_class_pattern_in_l2_loop` says the third instance triggers a structural sweep — at the same edit, add a greppable CI gate (`grep -L 'BEGIN IMMEDIATE' migrations/*.sql | grep -E '(DROP TABLE|DROP VIEW|ALTER TABLE.*RENAME)'`) so v246+ cannot regress.
2. **File F2 + F3 to maintenance project** `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per memory `feedback_l2_path_alpha_to_maintenance_project`. Reference: F2 = "HMAC-sign opaque list pagination cursor (v1.4.6/v1.4.7)", F3 = "Wrap-for-clarity audit of additive migrations v241/v242".
3. **Re-run this L2 cycle after F1 patch.** Expected outcome: cycle-2 APPROVE on the wave-integrated diff.

— gstack `/review` cycle-1, 2026-05-21
