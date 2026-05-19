# Packet F — Codex Consult Cycle-3 Review

**Reviewer:** codex consult
**Date:** 2026-05-19
**Verdict:** CONDITIONAL APPROVE
**Confidence:** 75

---

## Verdict line

CONDITIONAL APPROVE — one LOW condition (audit-order contradiction between §5.4 and §5.8 / §8.10) must be resolved before the packet locks; all six cycle-3 verification tasks come back clean or with advisory observations only.

## Two-sentence summary

The V1.2 folds are coherent: the §5.4 atomicity rewrite correctly describes the Mutex-guarded consume-then-record model, the `db_read → db_write` upgrade is named in both §5.4 and §10 commit 3, `§5.9 WpBlockRenders` closes the channel-registry gap and unblocks §8.9 and §9 invariant #4, and the §12 open questions #5 and #6 are well-scoped for cycle-3 lock. One internal inconsistency survives: §5.4 asserts the audit event order as `issued → verified → claim_feedback_recorded`, while §5.8 step 5 and §8.10 both assert the opposite (`issued → claim_feedback_recorded → verified`); L1 will write contradictory tests against these two sections unless one version is struck.

---

## Cycle-3 verification tasks

### Task 1 — §5.4 atomicity rewrite coherence

SATISFIED.

The V1.2 pseudocode correctly drops the SQL-transaction framing. The atomicity primitive is now stated as the `Mutex<HashMap>` on `SurfaceNonceStore` at `surface_nonce.rs:565-575`. The `try_mark_consumed` primitive at `:518` is the named call site. The idempotency-key branch (the "OR" option in V1.1) is gone — the packet commits to consume-then-record with fail-closed semantics and no rollback of the consume step. Decision #13 (§6) explicitly locks this as a real choice, not a TODO. The `db_read → db_write` upgrade is named in both §5.4 (last paragraph of the section) and in §10 commit 3 scope. Both cycle-2 MEDIUM conditions on Finding 1 are satisfied at the text level.

**Residual — LOW (audit-order contradiction, surfaces below as Finding 1).**

### Task 2 — §5.9 `WpBlockRenders` unblocks §8.9 and §9 invariant #4

SATISFIED.

§5.9 adds `WpBlockRenders` as the 10th `RenderPolicyChannel` variant and adds it to `ALL`. The W6-E `#[non_exhaustive]` channel-sweep gate then covers WP block-rendered claim payloads automatically. §8.9 (`FeedbackPayloadRedactionTest.php`) tests that user-authored fields in `payload_json` never reach non-originating actors; that test is meaningful only once the WP render surface is classified as a channel. §9 invariant #4 (no raw `payload_json` in audit payloads) is cross-referenced in AC J. Both dependency chains are now closed. AC J explicitly names `RenderPolicyChannel::WpBlockRenders` as a required element. §9 invariant #11 adds a compile-time check via the W6-E gate.

Cycle-2 Finding 2 condition is fully satisfied.

### Task 3 — §12 open questions #5 and #6 scope assessment

**#5 (validate_feedback_payload mirroring in PHP):** Well-scoped. The question is bounded: duplicate the variant-specific shape spec in PHP vs. call a runtime validation endpoint before mint. The packet gives a recommended answer (duplicate + parity test) and declares it cycle-3 lockable. The surface area is small — 5 variants with structured payloads, already enumerated in §5.2. A parity test asserting the PHP allowlist matches the Rust enum closes the drift risk without adding a round-trip. Locking "duplicate in PHP + parity test" at cycle-3 is correct; this is a L1 implementation choice with no architectural ambiguity.

**#6 (db_read → db_write granularity):** Well-scoped but has a specific call to verify at L1. The question asks whether the `surface_nonce_verify_response` codepath has other read-only invocations that would lose the `db_read` optimization when promoted. The packet gives the right answer: grep callers + audit; if the codepath is only the verify route, promote in place; if shared, split. This is a standard L1 grep task — no ambiguity in the decision logic. Cycle-3 lockable as stated.

Both open questions are resolved by the packet's own recommended answers. No additional cycle needed for either.

### Task 4 — §10 PR shape and commit sequencing

7 commits. Sequencing analysis:

**Commit 1 (PresenceNonceAction extension) vs Commit 2 (payload_json plumbing):**
Commit 1 is not independently shippable in the absence of commit 2 if any test in commit 1 exercises `payload_json` flow. Looking at §10 commit 1 scope: it covers the `PresenceNonceAction` enum extension + `From<PresenceNonceAction> for FeedbackAction` + test fixture update at `surface_nonce.rs:1690` + §8.1 unit test. §8.1 is a round-trip `as_str`/`parse` test — it does not require `payload_json` plumbing. The `From` impl requires `FeedbackAction` to exist (already does, in `abilities-runtime`). Commit 1 is independently buildable and testable. The sequencing is correct: commit 1 can land first; commit 2 adds the field and the tests that exercise it.

**Commit 5 (RenderPolicyChannel::WpBlockRenders) position:**
Commit 5 adds 1 enum variant + 1 `ALL` entry to `bridges/types.rs`. It has no dependency on commits 2, 3, or 4 (nonce field, wire-through, WP REST). It could ship after commit 1 or even as commit 2. However, it also does not block any commit in the current sequence — commit 4 (WP REST) does not reference `RenderPolicyChannel`. Moving commit 5 earlier is an option but not a correctness requirement. The current position (after WP REST, before JS) is acceptable: §8.9 (PHPUnit redaction test) is in commit 4 scope, and the `WpBlockRenders` channel it depends on is available at commit 5. Since §8.9 is in commit 4 and `WpBlockRenders` is in commit 5, there is a sequencing gap: the §8.9 test is written before the channel variant it depends on is added.

**Advisory (50):** Move commit 5 (`RenderPolicyChannel::WpBlockRenders`) before commit 4 (WP REST + PHPUnit), or move §8.9 into commit 5/6 scope. The current ordering means §8.9's compile-time channel dependency lands one commit after the test is written. This does not cause a build failure (the PHPUnit test does not import Rust types directly), but it means commit 4 cannot assert the compile-time invariant in §9 #11 until commit 5 lands. Not a blocking sequencing error; advisory for L1.

**Commit 4 expansion (WP REST):**
Commit 4 now covers: register `/dailyos/v1/nonce/verify` handler + update action allowlist to 9 variants + extend issue handler with `payload_json` validation + retire `submit_feedback()` transport method + retire `/v1/surface/feedback` allowlist entries + PHPUnit §8.7/§8.8/§8.9. This is a wide commit touching both PHP and Rust (the orphan retirement at `surface_runtime/mod.rs:1243, 4730, 4755`). The Rust-side retirement and the PHP-side registration are in the same commit. This is acceptable because the retirement and the registration are two sides of the same migration (dead path removed, live path added simultaneously). Splitting would leave a window where neither path works. One commit is correct.

**Sequencing verdict:** sound. The one advisory (commit 5 position relative to §8.9) is a L1 implementation ordering note, not a plan-time correctness issue.

### Task 5 — Scope creep audit

V1.2 additions against W4 scope as defined in `.docs/plans/v1.4.3-waves.md` §W4 (lines 203–218):

The waves plan §W4 was authored against the V1.0 parallel-substrate design. V1.0 was BLOCKed in cycle-1 because the substrate already existed. V1.2's reframe is the correct response to the BLOCK finding — not scope expansion. The items in V1.2 that were not in the waves plan §W4 text:

- **§5.9 WpBlockRenders:** Not in waves §W4. Required by the actual substrate (W6-E channel gate predates W4; WP render must be classified before W4 tests can be written). This is a prerequisite gap, not added scope — the §8.9 test is in scope and cannot be written correctly without the channel classification.
- **§6 decisions #11–#13 (V1.2 new):** Locks on rejection reason names, LLM-injection forward-constraint, and fail-closed semantics. These are constraint declarations against existing substrate behavior, not new deliverables. Zero implementation cost.
- **§9 invariant #12 (orphan grep gate):** Retirement of `/v1/surface/feedback` was an orphan identified at cycle-2. Retiring a dead path is inside W4 scope (W4 wires the replacement; the orphan is the obsolete predecessor).
- **7th commit (vs. 6 in V1.1):** Commit 5 (`WpBlockRenders`) added. Single enum variant + one `ALL` entry. Proportionate to §5.9.

**Conclusion:** nothing in V1.2 is outside W4 scope. The waves plan §W4 text describes the now-superseded V1.0 parallel-substrate design; V1.2 implements the same goal (feedback writes flow through the substrate claim path) via the correct mechanism. No scope creep.

### Task 6 — Orphan `/v1/surface/feedback` retirement sequencing (Rust + WP)

The orphan retirement is bundled into commit 4 (WP REST changes). Specifically:
- Rust side: remove from `surface_runtime/mod.rs:1243, 4730, 4755` allowlist
- WP side: delete `submit_feedback()` from `runtime-client.php:163`

Both are in the same commit 4. The WP side posts to the Rust route; the Rust route has no handler match block (orphan). Correct retirement order: remove the Rust allowlist entry first (or simultaneously) so that any in-flight WP call to the dead path gets a 404 from the runtime rather than being silently swallowed. Since commit 4 bundles both sides atomically, neither can ship without the other. This is the right choice — a split would leave either a WP caller pointing at a now-unauthorized Rust route (if Rust retires first) or a dead Rust route still on the allowlist (if WP retires first). Atomic retirement in one commit is correct.

The §9 invariant #12 grep gate prevents reintroduction. AC K names both sides of the retirement as required.

No sequencing problem. Atomicity for the retirement is satisfied by the single-commit boundary.

---

## Top findings

### Finding 1 — LOW: Audit event order contradicts between §5.4 and §5.8 / §8.10

**Severity:** LOW — implementation ambiguity; will cause L1 to write contradictory test assertions against two different sections of the same packet.

**Evidence:**

§5.4 "Audit ordering" paragraph (line 281): "Audit order on the happy path: `presence_nonce_issued` (phase 1) → `presence_nonce_verified` (phase 2 consume) → `claim_feedback_recorded` (phase 3 substrate write)."

§5.8 step 5 (line 367): "Audit log contains: `presence_nonce_issued → claim_feedback_recorded → presence_nonce_verified` (in that order — the claim_feedback row commits inside the tx, the verified event emits post-commit)."

§8.10 (line 447): "verify the chain `presence_nonce_issued → claim_feedback_recorded → presence_nonce_verified`"

The two orderings are:
- §5.4: issued → **verified** → claim_feedback_recorded
- §5.8 + §8.10: issued → **claim_feedback_recorded** → verified

The mechanistic question: `presence_nonce_verified` is emitted inside `try_mark_consumed` (from `SurfaceNonceService::verify_nonce`), which runs BEFORE the `record_claim_feedback` call in §5.4's pseudocode. That means §5.4's ordering is mechanically correct — verified fires inside `try_mark_consumed` before `record_claim_feedback` is called. But §5.8 claims the `verified` event emits "post-commit" (after the claims tx commits), which would make it fire after `claim_feedback_recorded`. These cannot both be true with the current substrate.

The §5.8 parenthetical "(the claim_feedback row commits inside the tx, the verified event emits post-commit)" appears to describe a desired behavior where the `_verified` event is held until after the claims write commits — which would require post-tx hook plumbing not mentioned anywhere else in the packet and not present in the existing `try_mark_consumed` substrate.

**Condition to lock before implementation:**
One version must be struck. §5.4's ordering (issued → verified → claim_feedback_recorded) is mechanically consistent with the existing `try_mark_consumed` substrate — `verified` fires inside the Mutex-guarded HashMap mutation, before the function returns, before `record_claim_feedback` is called. §5.8's assertion of the reverse order and its parenthetical explanation would require a post-transaction hook that is not specified. Strike the §5.8 step 5 parenthetical and correct the event order to match §5.4. Update §8.10 to assert `issued → verified → claim_feedback_recorded`.

---

## Scope alignment summary

V1.2 added 7 items to §6, added §5.9, and expanded §10 to 7 commits. None of these constitute scope creep against the W4 goal (feedback writes through the claim path). The waves plan §W4 text reflects the V1.0 parallel-substrate design that was BLOCKed; V1.2 supersedes the mechanism, not the goal. The 7-commit PR is right-sized: 5 commits touch distinct code surfaces (Rust enum, Rust nonce service, Rust runtime handler, PHP/REST, Rust channel registry, JS), one adds the JS affordance, one adds the e2e fixture. File count is proportionate; no new abstractions beyond the enum extension, one new field, and one new enum variant in `RenderPolicyChannel`.

---

## Lock criteria update

- [x] Cycle-2 Finding 1: §5.4 atomicity model coherent with actual substrate (Mutex + consume-then-record + fail-closed). `db_read → db_write` upgrade named. SATISFIED.
- [x] Cycle-2 Finding 2: `RenderPolicyChannel::WpBlockRenders` added (§5.9); §8.9 and §9 invariant #4 unblocked. SATISFIED.
- [x] §12 #1 and #2 CLOSED in V1.2. #3 and #4 are informational (self-answering). #5 and #6 cycle-3 lockable via recommended answers.
- [ ] **NEW — Finding 1 condition:** audit event ordering contradiction between §5.4 and §5.8 / §8.10. Strike §5.8 step 5 parenthetical; correct §5.8 step 5 order and §8.10 order to match §5.4 (`issued → verified → claim_feedback_recorded`). Verifiable from packet text alone; no implementation required.
- [x] Scope creep audit clean: no W4 scope additions.
- [x] Orphan retirement atomicity correct: single commit 4 bundles both sides.
- [x] Commit 1 independently shippable (confirmed: only unit tests, no `payload_json` dependency).

**Advisory (not blocking):** Move commit 5 (`WpBlockRenders`) before commit 4, or move §8.9 into commit 5/6 scope, so the compile-time channel invariant in §9 #11 is satisfied within the commit that writes the test.

Packet locks when Finding 1 condition is folded into V1.3 and verifiable from packet text.
