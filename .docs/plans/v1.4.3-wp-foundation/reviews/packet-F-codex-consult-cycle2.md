# Packet F — Codex Consult Cycle-2 Review

**Reviewer:** codex consult
**Date:** 2026-05-19
**Verdict:** CONDITIONAL APPROVE
**Confidence:** 75

---

## Verdict line

CONDITIONAL APPROVE — two surgical conditions must lock before implementation begins; all other cycle-1 findings are satisfied.

## Two-sentence summary

V1.1 correctly folds the four cycle-1 sequencing, affordance, fixture, and nonce-table findings: PR ordering is sound, single-trigger + reveal menu is locked, the e2e fixture is in-process, and the no-migration decision is explicit. Two new findings surface on close reading: the §5.4 atomicity pseudocode describes a SQL-row mechanism that does not match the actual in-memory Mutex store (the packet leaves the "OR idempotency_key" branch undecided), and §12 open question #1 (WP render channel classification) remains unresolved in V1.1 and must be closed before implementation because it determines whether `RenderPolicyChannel` needs a 10th variant before W4 tests can be written.

---

## Cycle-1 finding confirmations

### Confirmation 1 — PR sequencing (cycle-1 finding #1)
SATISFIED.

§10 orders 6 commits:
1. `PresenceNonceAction` extension + unit test §8.1
2. `surface_nonce.rs` `payload_json` + `wp_user_id` + audit payload extensions (§5.5) + tests §8.2, §8.4, §8.5
3. `verify_nonce` → `record_claim_feedback` wire-through + §8.3 replay test + §8.10 audit correlation test
4. WP REST extension + PHPUnit §8.7–§8.9
5. JS affordance
6. End-to-end fixture §8.6

Audit payload extension (§5.5) is commit-2; §8.10 (the audit correlation test that depends on the extended payload shape) is commit-3. Runtime route extension is commit-3; PHP/REST tests are commit-4. Both sequencing invariants from cycle-1 are satisfied.

### Confirmation 2 — JS affordance shape (cycle-1 finding #3)
SATISFIED.

§5.7 locks single-trigger button + reveal menu (9 entries). The conditional editors (textarea for `NeedsNuance`/`WrongSubject`, source-index picker for `WrongSource`, surface-picker for `SurfaceInappropriate`, invocation-picker for `NotRelevantHere`) are not deferred — they are load-bearing for AC-B and AC-H. Without them, `payload_json` cannot be captured for the 5 variants that require it, breaking the `validate_feedback_payload` check at `claims.rs:5150`. Scope is justified.

### Confirmation 3 — end-to-end fixture (cycle-1 finding #2)
SATISFIED.

§5.8 specifies in-process runtime + handler fixture using a real SQLite DB and real `record_claim_feedback` path. No full Tauri/macOS app boot. Fixture exercises 4 representative variants including payload-bearing ones (`WrongSubject` with `corrected_to`, `NeedsNuance` with `corrected_text`).

### Confirmation 4 — no-migration / no-table-backed nonce (cycle-1 finding #4)
SATISFIED.

Decision #9 explicitly removes V1.0's migrations (v181–v183). The actual store is confirmed in-memory: `surface_nonce.rs` uses `Mutex<SurfaceNonceStoreInner>` (a `HashMap<NonceDigest, PresenceNonceBinding>` at lines 567–573) with no SQL nonce table in any migration file. The 60s in-memory TTL contract is locked in decision #6.

---

## Top-3 findings

### Finding 1 — MEDIUM: §5.4 atomicity pseudocode describes a SQL mechanism that does not match the actual in-memory substrate; the decision between the two stated options is unresolved

**Severity:** MEDIUM (implementation ambiguity that will hit L1; not a correctness hole in the design intent, but a wrong-substrate spec that will cause confusion at the diff)

The §5.4 pseudocode calls `mark_consumed_if_unclaimed(tx, &nonce.digest)` with a `rows_affected == 0` check, which implies a SQL `UPDATE WHERE consumed_at IS NULL` on a DB-persisted nonce row. The actual implementation uses `blocking_lock()` on a `Mutex<HashMap>` at `surface_nonce.rs:649` — the nonce store has no SQLite rows and no migration has ever created one (confirmed by grepping all migration files). The "atomic single-transaction" framing in §5.4 and AC-D ("Single transaction") cannot mean a single SQLite transaction spanning nonce consumption and feedback write: the two subsystems are structurally separate (in-memory Mutex vs. SQLite WAL).

The partial-failure case that matters: nonce is consumed (`try_mark_consumed` succeeds in-memory at `surface_nonce.rs:725`) and then `record_claim_feedback` fails (SQLite write error). The nonce is spent; the user cannot retry with the same token; the 60s TTL means they need a new nonce. Decision #10 (fail-closed) covers the race case but not this partial-failure recovery case. §3 (line 81) offers "OR `record_claim_feedback`'s row carries `idempotency_key = nonce_digest` for safe retry" as an alternative — but V1.1 never locks which path.

Additionally, `surface_nonce_verify_response` at `surface_runtime/mod.rs:2571` currently calls `.db_read(...)`. `record_claim_feedback` uses `with_claim_transaction` (a write at `claims.rs:6717`) which will either panic or silently fail on a read-only connection. The packet does not mention upgrading this handler to `db_write`.

**Condition to lock before implementation:**
- §5.4 must choose one mechanism: (a) in-memory Mutex consumption is the anti-replay gate; `record_claim_feedback` is called separately on the writer connection; partial failure is accepted with a clear user-visible error ("feedback could not be saved — please try again with a new feedback action"); OR (b) add an `idempotency_key` column to `claim_feedback` bound to `nonce_digest` so a retry with a new nonce for the same feedback intent is idempotent. Option (a) is simpler and consistent with the 60s TTL's intent.
- §10 commit-3 must note the `db_read → db_write` upgrade for `surface_nonce_verify_response` at `surface_runtime/mod.rs:2571`.

### Finding 2 — MEDIUM: §12 open question #1 (WP render channel in `RenderPolicyChannel`) remains unresolved and blocks test writing

**Severity:** MEDIUM (unresolved design question that must be locked before L1 can write §8.9 and the §5.5 redaction invariant tests)

`RenderPolicyChannel::ALL` has exactly 9 variants at `src-tauri/src/bridges/types.rs:97` — confirmed by reading the file. None of the 9 covers WP block rendering: the closest is `TauriRenders`, but WP blocks render server-side (PHP) and client-side (React in Gutenberg), not through the Tauri render pipeline. ADR-0108 governs sensitivity policy across surfaces but predates v1.4.2 and does not classify the WP surface as a channel variant.

§8.9 (`FeedbackPayloadRedactionTest.php`) and §9 invariant #4 (no raw `payload_json` in audit payloads) are both anchored to the redaction policy that ADR-0108 governs through `RenderPolicyChannel`. If the WP render surface is not enumerated as a channel, the `#[non_exhaustive]` gate that W6-E relies on (W6-E L0 packet §4.2, line 189) will silently miss WP blocks as a new channel when the W6-E channel-sweep test runs. V1.1 explicitly asks cycle-2 to resolve this by reading the registry, but stops short of making the call.

**Condition to lock before implementation:**
One of the two options must be chosen and written into §12 (closing the open question) or into a named decision in §6:
- (a) WP blocks constitute a new `WpBlockRenders` channel in `RenderPolicyChannel`; this requires a one-line enum addition at `bridges/types.rs` and a corresponding `ALL` array update — trivial, and the `#[non_exhaustive]` gate catches any future WP channel gap; OR
- (b) WP block rendering is covered by `TauriRenders` with a documented rationale that both surfaces share the same actor-filtered provenance contract and the same claim-render projection path. This must be an ADR-0108 amendment comment, not just a packet assertion.
Option (a) is recommended: it is accurate (WP render is not Tauri render) and costs one enum variant.

### Finding 3 — LOW: §5.1 extend-to-9 recommendation is correct; mapping shim is not justified

**Severity:** LOW / scope-guardian confirmation (not a blocking condition)

The cycle-2 task asked whether the extend-to-9 path adds unjustified abstraction complexity relative to a mapping shim. It does not. The mapping shim would translate 4 existing names (`Correct`, `Dismiss`, `Corroborate`, `Contradict`) into 9 substrate names at the `verify_nonce` boundary — adding a layer that serves no purpose other than preserving the old 4 names that have no consumers at the WP layer. The `From<PresenceNonceAction> for FeedbackAction` impl at compile time (§9 invariant #8) gives free exhaustiveness checking; the shim would require a runtime or compile-time match that is more complex for zero benefit. Extending to 9 is the minimum-abstraction choice, not the maximum-abstraction choice.

The old 4 variant names (`Correct`, `Dismiss`, `Corroborate`, `Contradict`) are informal; grepping confirms they have no callers outside `surface_nonce.rs` itself and the one existing test at `surface_nonce.rs:1690`. Renaming is safe.

---

## Scope alignment summary

The §13 v1.4.4+ backlog is clean — no deferred work has been quietly pulled into V1.1. Per-claim action filtering, admin health panel, MCP feedback-as-ability, and durable feedback queue all remain explicitly deferred. The conditional JS editors (finding confirmed in Confirmation 2) are load-bearing, not scope creep.

The 6-commit PR shape is right-sized. Complexity smell test: no new abstractions beyond the enum extension and one new field; no new files beyond test files and the JS affordance component. File count is proportional.

---

## Lock criteria for this packet

- [ ] Finding 1 condition: §5.4 commits to one atomicity mechanism; `db_read → db_write` upgrade noted in commit-3 scope.
- [ ] Finding 2 condition: §12 open question #1 closed with explicit channel classification decision (option a or b above, stated rationale).
- [ ] Both conditions are verifiable from packet text alone (no implementation required to verify).
- [ ] CSO and security-auditor cycle-2 verdict in place (mandatory gates per Amendment 3).
