# L2 (Diff) — /cso review — Wave v1.4.4 W1+W2 integrated, cycle 1

- **Branch:** `wave/v1.4.4-w1-stage1a`
- **HEAD:** `bdade2cc`
- **Base:** `0f8533e1`
- **Reviewer:** /cso (daily mode, 8/10 confidence gate)
- **Scope:** W2 new write paths + audience-boundary surfaces only (W1 cycle-1 CSO findings already path-α'd to DOS-749 / DOS-750)
- **Date:** 2026-05-21

---

## VERDICT: APPROVE WITH ONE BLOCKING FINDING (CRITICAL)

W2 trust-boundary discipline is otherwise clean: MergeIntent payload contract matches ADR-0123 V1.1, the ADR-0108 §3 sanitizer is wired for `supporting_evidence`, AgentMcp denial is universal for all feedback writes (the `!actor.is_user()` gate at `claim_receipt/feedback.rs:266`), the three new Read abilities declare `allowed_actors = [User, Agent]` with `mcp_exposure = None` (correctly excludes McpClient), the audience filter is consistently invoked via `dailyos_envelope_consume_claim()` across every entity-detail composite + inner block, and the WP layer never touches `$wpdb` for claim-feedback writes — every mutation routes through `record_claim_feedback`.

One blocking finding: v245 migration repeats the same class of bug v244 just retroactively fixed (non-transactional multi-statement table rebuild). This is a literal regression against the L3 cycle-2 F3 K-out lesson captured at `migrations.rs:949-957`. Fix is two lines.

---

## Finding 1: v245 `claim_feedback` table rebuild is not wrapped in `BEGIN IMMEDIATE; ... COMMIT;` — same class as v243→v244 retro fix

* **Severity:** CRITICAL
* **Confidence:** 9/10
* **Status:** VERIFIED (file inspection + cross-reference to v244 retro comment)
* **Phase:** P5 — Infrastructure / migration safety (matches CLAUDE.md "Two similar bugs in a row → system-wide class review")
* **Category:** OWASP A04 Insecure Design + migration availability invariant
* **File:** `src-tauri/src/migrations/245_dos_484_feedback_merge_intent.sql:19-59`
* **ADR cite:** v244 retro comment at `src-tauri/src/migrations.rs:949-957` is canonical — "v244 re-runs the same rebuild inside `BEGIN IMMEDIATE; ... COMMIT;` so the writer holds the write lock across the entire sequence and no other connection can see a missing view." v245 does **not** apply the same fix to its own `CREATE TABLE _new` → `INSERT ... SELECT` → `DROP TABLE` → `ALTER TABLE ... RENAME` → `CREATE INDEX` sequence.

* **Description:** The migration runner's `execute_batch` call at `migrations.rs:3573` (referenced verbatim in the v244 retro comment) does **not** implicitly wrap statements in a transaction. v245 performs a 5-statement table rebuild on `claim_feedback` (the substrate that backs every feedback write across all 10 FeedbackAction variants — including MergeIntent, the entire raison d'être of this migration). Between `DROP TABLE claim_feedback;` and `ALTER TABLE claim_feedback_new RENAME TO claim_feedback;`, multi-process readers (Tauri host + dailyos-mcp + any background worker holding a separate connection in WAL mode) can observe a state where `claim_feedback` does not exist at all, then briefly exists without its indexes. Any concurrent `record_claim_feedback` call during the migration window will hit "no such table: claim_feedback" or "no such index: idx_feedback_claim" depending on phase.

* **Exploit / failure scenario:**
  1. User upgrades to v1.4.4. Tauri host launches and starts migration to v245.
  2. dailyos-mcp subprocess (separate SQLite connection, started earlier or restarted by the OS) is mid-flight handling a claim-feedback write from an MCP client.
  3. The MCP write lands between v245's `DROP TABLE claim_feedback;` and `ALTER TABLE claim_feedback_new RENAME TO claim_feedback;`. SQLite returns "no such table" or the operation succeeds against the wrong table-id and then disappears with the rebuild.
  4. End user sees a non-deterministic feedback-submission failure during upgrade. This is the exact class the v244 retro called out: "multi-process readers could observe the gap between DROP VIEW and CREATE VIEW and fail on 'no such view'."
  5. Worse: if the JSON `INSERT INTO claim_feedback_new ... SELECT FROM claim_feedback` fails partway (disk pressure, locking), the auto-commit per statement leaves the DB in a partial state with no rollback — `claim_feedback_new` exists with partial rows, `claim_feedback` may already be dropped.

* **Impact:** Migration failure mode that loses claim_feedback rows on hard interruption (no ROLLBACK because no BEGIN); transient "no such table" errors for any concurrent reader/writer during the migration window. The substrate this migration is widening to support (MergeIntent) is itself unreachable until migration completes successfully, so a partial-state failure on upgrade is non-trivial to recover.

* **Recommendation (two-line fix):** Wrap the migration in an explicit transaction, matching v244's pattern verbatim:

  ```sql
  BEGIN IMMEDIATE;

  CREATE TABLE claim_feedback_new (
      ...
  );
  INSERT INTO claim_feedback_new ...;
  DROP TABLE claim_feedback;
  ALTER TABLE claim_feedback_new RENAME TO claim_feedback;
  CREATE INDEX IF NOT EXISTS idx_feedback_claim ON claim_feedback(claim_id);
  CREATE INDEX IF NOT EXISTS idx_feedback_type ON claim_feedback(feedback_type, submitted_at);

  COMMIT;
  ```

  `BEGIN IMMEDIATE` acquires the writer lock up-front, blocking other writers/readers from observing intermediate states; `COMMIT` releases atomically. This is the exact remediation v244 applied to the meeting-prep view rebuild.

* **Active-verification hook:** Add a regression test similar to v244's that opens two SQLite connections to a fixture DB, kicks off the v245 migration on connection A, and asserts connection B never observes "no such table: claim_feedback" during the migration window. If a class-level CI gate is desired (CLAUDE.md "same shape twice → audit the class, add a gate"), grep `src-tauri/src/migrations/*.sql` for the pattern `DROP TABLE\|DROP VIEW\|DROP INDEX` and require an enclosing `BEGIN IMMEDIATE; ... COMMIT;`. This would have caught v245 mechanically. Recommend tracking this gate as a Codebase Maintenance follow-up since it spans the migration substrate, not v1.4.4 W2 scope.

* **Why not path-α:** This is a literal regression against a K-out lesson captured exactly one migration ago (v244 is v245's immediate predecessor). The same shape, the same file family, the same retro comment sitting directly above the v245 entry. Routing to maintenance would defeat the K-channel feedback loop entirely.

---

## Items audited and cleared (no findings)

### A. MergeIntent FeedbackAction — ADR-0123 V1.1 compliance

**Verified clean.**

* `FeedbackAction::MergeIntent` (10th variant) declared at `src-tauri/abilities-runtime/src/abilities/feedback.rs:80` with full doc-comment describing the typed-proposal-only semantics (no lifecycle ratchet, no trust impact, no repair queue).
* Serde wire format `merge_intent` round-trips correctly; `feedback_action_serializes_only_ten_closed_values` and `merge_intent_round_trips_through_json` tests pass at `feedback.rs:418-454`.
* `feedback_semantics(MergeIntent)` returns `(Active, NONE, None, Default, requires_action_metadata=true, is_truth_feedback=false)` at `feedback.rs:359-378` — matches ADR-0123 V1.1 declared shape exactly.
* Render-policy uniqueness invariant preserved: `merge_intent_is_typed_proposal_no_lifecycle_or_trust_change` test at `feedback.rs:486-498` asserts the policy bucket; `every_action_has_distinct_render_policy` test at `feedback.rs:617-651` confirms `Default` policy is uniquely owned by MergeIntent (all 9 prior variants have distinct non-Default policies — verified by reading lines 254-372).

### B. ADR-0108 §3 sanitizer applied to `supporting_evidence`

**Verified clean.**

* Payload validation at `src-tauri/src/services/claim_receipt/feedback.rs:632-681` enforces the canonical contract:
  - `merge_target` is required, deep-decoded as `SubjectRef` (ADR-0125 conformance).
  - `supporting_evidence` is optional; null is normalized to absent; non-string rejected with `BadRequest`.
  - Character budget enforced: `evidence.chars().count() > MAX_NOTE_CHARS` (`MAX_NOTE_CHARS = 500` at `feedback.rs:413`) — exactly matches the AC.
  - Sanitizer routed through `abilities_runtime::abilities::provenance::render::sanitize_explanation_for_render` at `feedback.rs:708-731`, which is the ADR-0108 §3 pipeline. Warnings surface as `SanitizerWarning` with field-path tagging.
* Allowed-keys list at `feedback.rs:704` is the exact `["merge_target", "supporting_evidence"]` set — no extra-field smuggling possible (unknown-key check at `feedback.rs:480-490` rejects).

### C. Agent actor denied for MergeIntent + new list_* abilities (AC-8.13)

**Verified clean.**

* MergeIntent: `record_claim_feedback` at `claim_receipt/feedback.rs:266-268` enforces `if !actor.is_user() { return Err(FeedbackError::AgentActorDenied); }` *before* any per-action validation runs. This is a universal gate across all 10 FeedbackAction variants — no MergeIntent-specific bypass exists.
* `list_accounts`: `#[ability(... allowed_actors = [User, Agent], mcp_exposure = None ...)]` at `list_accounts/mod.rs:31-45` — `Actor::McpClient` is correctly excluded from `allowed_actors`, and `mcp_exposure = None` keeps it out of MCP tool introspection. Test `agent_mcp_actor_is_denied` at `list_accounts/mod.rs:346` covers the deny path.
* `list_people`: same shape at `list_people/mod.rs:21-34`; test at line 265.
* `list_projects`: same shape; test at line 269.
* Worth noting in the doc-comment at `list_accounts/mod.rs:11-14`: the rationale ("not yet exposed to agent-side MCP, and `mcp_exposure = None` keeps them out of MCP introspection") matches the cycle-2 CSO discipline applied to W1's list abilities. Consistent.

### D. AgentMcp audience aggregate-only filter on touchpoints across 4 entity-detail composites

**Verified clean.**

* Account Detail (24 inner blocks), Project Detail (15), Person Detail, Meeting Detail (10) all route claim-bearing rows through `dailyos_envelope_consume_claim()`, which delegates audience filtering to `build_receipt_for_audience` (DOS-341 W1 substrate) server-side. The boundary lives in Rust, not PHP — WP cannot bypass it.
* Spot-checked 6 inner blocks across all 4 composites (`quote-wall`, `value-commitments`, `on-track-chapter`, `stakeholder-grid`, `triage-section`, `outlook-panel`); every header doc-comment cites DOS-341 and the audience-filter chain. No direct envelope-payload reads found.
* The risk surface that would matter — direct `$wpdb->get_results()` or raw envelope-payload reads in inner blocks — is absent across the W2 surface.

### E. WP merge picker path-α (WP emits intent, Tauri executes — no direct DB write from WP)

**Verified clean.**

* Person Detail merge affordance (`person-detail/render-functions.php:176-209`) forwards a `FeedbackAction::MergeIntent` claim_ref through the standard feedback ability invocation — no direct DB write.
* People-index per-row merge affordance (`people-index/view.js:42-131`) emits a feedback request via the runtime client; never touches WP DB.
* Recommended-actions Suggest-merge button (`recommended-actions/render-functions.php:90-200`) carries the canonical MergeIntent payload shape and routes through `record_claim_feedback` server-side. The button is a render affordance; the write boundary stays in Rust.
* Metadata-proposal-drawer (`metadata-proposal-drawer/render-functions.php:30-184`) — DOS-328 — explicitly routes accept/reject through `services::claims::record_claim_feedback`; doc-comment line 30: "no W1 reopen." No `$wpdb` writes anywhere in `wp/dailyos/blocks/metadata-proposal-drawer/`.

---

## Filter stats (transparency)

* Candidates surfaced during scan: 7
* Hard-exclusion filtered (test fixtures, doc files, infrastructure-only configs): 3
* Confidence-gate filtered (below 8/10): 2
* Path-α routed (theoretical hardening, not literal AC violation): 1 — "v245 could benefit from a CHECK-rebuild CI gate that mechanically detects DROP-without-BEGIN" (filed as recommendation, not a finding; tracked in Finding 1's verification hook section)
* Reported: 1 (Finding 1, CRITICAL)

## Recommendation to wave orchestrator

Block merge until Finding 1 is fixed. The fix is a 2-line wrap (`BEGIN IMMEDIATE;` / `COMMIT;`) inside `245_dos_484_feedback_merge_intent.sql`. No regression risk — it strictly tightens the migration's atomicity guarantee. Re-run cargo test after the change to confirm no test depends on the non-transactional execution order (none should; the migrations test fixture uses a single connection, so the bug only manifests in multi-process production).

After Finding 1 is fixed, this verdict converts to **APPROVE** for L2 cycle-1.

---

## Disclaimer

This tool is not a substitute for a professional security audit. /cso is an AI-assisted scan that catches common vulnerability patterns — it is not comprehensive, not guaranteed, and not a replacement for hiring a qualified security firm. LLMs can miss subtle vulnerabilities, misunderstand complex auth flows, and produce false negatives. For production systems handling sensitive data, payments, or PII, engage a professional penetration testing firm. Use /cso as a first pass to catch low-hanging fruit and improve your security posture between professional audits — not as your only line of defense.
