# Packet F — Codex Consult Cycle-4 Review

**Reviewer:** codex consult
**Packet version:** V1.3
**Date:** 2026-05-19
**Verdict:** CONDITIONAL APPROVE — one LOW; does not block lock if condition is confirmed resolved by the L0 coordinator.

---

## Verdict line

CONDITIONAL APPROVE. The cycle-3 codex consult condition (audit-order contradiction) is confirmed folded and correct in V1.3. Four of five cycle-4 verification tasks come back clean. One LOW survives: §9 invariant #10 still cites `surface_nonce.rs:1235` for `NonceAuditContext` after V1.3 corrected the same citation to `:1158` everywhere else — this is a stale grep anchor that will misguide L1 and the CI gate. Invariants #6 and #7 also carry invented rejection-reason strings that contradict the cycle-3 finding that V1.3 "uses the existing enum names everywhere." If these two stale strings survive, the §8.3/§8.4 tests will assert the wrong `rejection_reason` literals at runtime.

---

## Two-sentence summary

V1.3 correctly resolves the cycle-3 codex consult LOW (audit event order), grounds the pseudocode against the real `verify_and_consume` API, extends `VerifiedNonce` to carry binding fields, and fixes the actor format — all five of the codex-challenge HIGH/MEDIUM findings from cycle 3 are substantively folded. Two residual encoding inconsistencies survive in §9: invariant #10 cites the wrong line for `NonceAuditContext` (`:1235` vs the V1.3-corrected `:1158`), and invariants #6 and #7 assert `"already_consumed"` and `"wp_user_mismatch"` as `rejection_reason` strings when the packet elsewhere locks use of the existing `PresenceNonceRejectReason` enum variants (`Replayed`, `WrongUser`) verbatim.

---

## Cycle-4 verification tasks

### Task 1 — Audit event order fix (cycle-3 codex consult condition)

**VERIFIED RESOLVED.**

V1.3 §5.8 step 5 (packet line 450): "Audit log contains: `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded` (V1.3 correction — `presence_nonce_verified` fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation BEFORE `record_claim_feedback` is called)."

V1.3 §8.10 (packet line 534): "verify the chain `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded` (V1.3 correction — `presence_nonce_verified` fires inside `verify_and_consume` BEFORE `record_claim_feedback`)"

Both now match §5.4's "Audit ordering" paragraph which states: "Audit order on the happy path: `presence_nonce_issued` (phase 1) → `presence_nonce_verified` (phase 2 consume) → `claim_feedback_recorded` (phase 3 substrate write)."

The cycle-3 codex consult condition is fully satisfied. No ambiguity survives for L1 writing §8.10.

---

### Task 2 — Cross-layer integration consistency

#### 2a — §5.4 `VerifiedNonce` extension and sequencing in §10

**SOUND.**

V1.3 §5.4 extends `VerifiedNonce` (substrate-side struct change) in commit 3 scope. Commit sequencing in §10:

- Commit 1 extends `PresenceNonceAction` enum + adds `From` impl. No dependency on `VerifiedNonce` extension.
- Commit 2 adds `payload_json` to `PresenceNonceBindingFields` + `NonceAuditContext` slot additions. `VerifiedNonce` is not yet extended here; the new binding field is set but not yet carried out.
- Commit 3 adds the `VerifiedNonce` extension (the new fields) + the handler wire-through (`db_read → db_write`) + the consume-then-record orchestration + orphan retirement from `surface_runtime/mod.rs`. `VerifiedNonce::claim_id`, `action`, `payload_json`, `wp_user_id`, `session_id` are available to the handler only after commit 3. Commits 1 and 2 do not reference these fields on `VerifiedNonce`, so the dependency order is correct.

The `verify_and_consume` function lives in `surface_nonce.rs` and the `VerifiedNonce` struct definition is also in `surface_nonce.rs:583`. The extension and the handler that reads the extended struct both land in commit 3. This is a single-commit, self-consistent change — no forward reference across commits. Sequencing is sound.

**One advisory note (50, not blocking):** Commit 3 is now the widest commit: `VerifiedNonce` struct extension + `verify_and_consume` body modification (to capture binding fields) + handler upgrade + orphan retirement (both the Rust allowlist entries and `submit_feedback()` PHP retirement are noted as commit 3 + commit 4 scope in §10). Cross-referencing §10 line 560: "Commit 3: `verify_nonce` → `record_claim_feedback` wire-through... retire orphan `/v1/surface/feedback` allowlist entries at `mod.rs:1243, 4730, 4755` + retire `runtime-client.php:163` `submit_feedback()`." The WP-side PHP retirement is listed under commit 3 here but also under commit 4 in the WP REST section. This is minor framing drift; §5.6 step 4 and §9 invariant #12 are authoritative on what the retirement contains. Advisory for L1 to reconcile; not a plan-level error.

#### 2b — Orphan retirement atomicity

**SOUND.**

The orphan retirement is confirmed atomic: both the Rust allowlist removal (`surface_runtime/mod.rs:1243, 4730, 4755`) and the WP `submit_feedback()` deletion happen in the same commit pair (commits 3-4 per §10). §9 invariant #12 adds a grep gate preventing reintroduction. AC K names both sides. The cycle-3 codex consult Task 6 verdict (orphan retirement correct in V1.2) carries forward; V1.3 did not change this section. Atomicity is preserved.

#### 2c — §5.9 manual gate equivalence vs. W6-E auto-gate

**SOUND FOR V1.4.3; RESIDUAL IN §9 INVARIANT #11.**

The cycle-3 codex challenge MEDIUM finding (W6-E gate does not exist) is correctly folded in V1.3: §5.9 now specifies a **manual** PHPUnit test at `wp/dailyos/tests/blocks/FeedbackPayloadRedactionTest.php` (§8.9) as the v1.4.3 coverage gate. The W6-E `#[non_exhaustive]` + `ALL` sweep backfill is explicitly deferred to a maintenance ticket. V1.3 §5.9 states: "§8.9 is the **manual** redaction-coverage gate for V1.4.3: a PHPUnit assertion that the WP block-render output for a feedback projection contains the appropriate redactions for `payload_json` user-authored fields." The §8.9 test enumerates the channel explicitly rather than relying on auto-sweep.

This is mechanically equivalent to what the W6-E auto-gate would provide for v1.4.3 purposes: the same assertion (WpBlockRenders projection applies payload_json redaction) is authored and will run in CI. The auto-gate would catch future channel additions; the manual test covers today's surface. Appropriate for v1.4.3 scope.

**However**, §9 invariant #11 still reads: "**`RenderPolicyChannel::ALL` MUST include `WpBlockRenders`.** Compile-time check via the `#[non_exhaustive]` channel-sweep gate from W6-E." The W6-E gate does not exist. This invariant was not updated in V1.3 to reflect the manual §8.9 gate disposition. The invariant text points at a non-existent mechanism. This is captured as Finding 1 below (LOW).

---

### Task 3 — Scope creep on V1.3 additions

**CLEAN.**

V1.3 additions against W4 scope:

- **`VerifiedNonce` extension** (§5.4 §10 commit 3): required by the cycle-3 codex challenge HIGH #1 finding — the cycle-2 pseudocode named a non-existent API. V1.3's extension is the minimum-scope fix: capture binding fields into `VerifiedNonce` inside `verify_and_consume`'s existing Mutex critical section before returning. No new function, no new struct, one struct field set added. Proportionate correction; not scope expansion.

- **Full `PresenceNonceRejectReason` enumeration** (§3, §5.4 table, §6 #11, AC F): required by cycle-3 codex challenge HIGH #2 finding — the "3 variants cover all paths" claim was empirically false. The V1.3 correction enumerates the existing 17-variant enum and states W4 emits the existing subset verbatim. Zero new deliverables; constraint language only. Not scope expansion.

- **Manual §8.9 PHPUnit redaction gate** (§5.9): required by cycle-3 codex challenge MEDIUM #3 finding — the W6-E auto-gate does not exist. The manual test is the minimum-scope substitute that closes the cycle-2 channel-classification gap for v1.4.3. Not scope expansion.

- **§16 L1 appendix** (8 implementation notes): all 8 are FYI notes captured from cycle-3 reviewer advisories. They do not change packet spec text; they are explicitly framed as "L1 author concerns." None introduce net-new W4 deliverables. The crate-boundary note for `From<PresenceNonceAction> for FeedbackAction` (note 3) is a real L1 architectural decision (pub promotion vs. re-export vs. third crate), but the packet correctly delegates it to L1 with "pick the smallest visibility change" rather than locking one option at L0. This is appropriate: the decision has no architectural ambiguity and multiple valid implementations. Not scope expansion.

All V1.3 additions are within W4 scope as established at cycle-1 (reframe from parallel-substrate to substrate-extension). None introduce new abstractions, new tables, or new service layers.

---

### Task 4 — §16 appendix scope

**APPROPRIATELY SCOPED.**

The 8 L1 implementation notes in §16 are assessed individually:

1. `verify_and_consume` on `SurfaceNonceStore` vs `SurfaceNonceService` — L1 structural disambiguation. Correct as L1 note.
2. `[Self; 9]` → `[Self; 10]` for the `ALL` array — single-constant L1 edit. Correct.
3. Crate boundary for `From` impl — L1 visibility decision with guidance. Appropriate.
4. Deployment ordering for orphan retirement — ops advisory for non-local-to-local scenarios. Correct framing; no v1.4.3 action required.
5. Phase-3 failure-budget charging — L1 `charge_failure_best_effort` call site on the `record_claim_feedback` error path. Appropriate L1 detail.
6. HKDF `info` parameter distinctness — single constant string decision. L1. Correct.
7. Permission callback literal reuse (`can_issue_presence_nonce`) — L1 implementation guidance. Correct.
8. WP `payload_json` non-plain-object rejection — L1 defensive validation. Correct.

All 8 are genuinely L1-author concerns: single-constant choices, naming precision, defensive hygiene, ops context. None require a §6 decision lock or AC change. The appendix correctly functions as a handoff checklist, not a packet-spec amendment. Scope of §16 is appropriate.

---

### Task 5 — §12 open questions state

**Questions #3, #4, #5, #6 are all well-scoped for cycle-4 lock.**

- **#3 (auto-detect surface payload for `SurfaceInappropriate` + `NotRelevantHere`):** Self-resolving at L1. The packet recommends auto-detect with confirmation + override. The recommendation is consistent with the component shape described in §5.7 (conditional surface picker / invocation picker). No architectural ambiguity; L1 implements the recommendation. Lockable.

- **#4 (JS bundle size ~8-12 KB):** The packet states it stays within existing per-block budget and recommends shipping with L4 audit as a safety net. The information need is resolved (quantified estimate; within budget). Lockable.

- **#5 (PHP validation shape mirroring):** The packet recommends duplicate-in-PHP + parity test. The cycle-3 codex consult Task 3 assessment confirmed this is a bounded L1 implementation choice with no architectural ambiguity (5 variants, small surface area, parity test closes drift risk). Lockable.

- **#6 (db_read → db_write granularity):** The packet recommends grep callers + promote-in-place if the codepath is verify-only. The decision logic is unambiguous. Lockable.

None of the four require resolution before lock. All four have self-answering recommended paths with clear L1 decision rules.

---

## Findings

### Finding 1 — LOW: §9 invariant #10 cites wrong line for `NonceAuditContext` after V1.3 corrected it elsewhere

**Severity:** LOW — will misdirect the CI grep gate and L1 structural anchor.

**Evidence:**

V1.3 changelog (packet line 83-84): "(codex challenge LOW #4) §3 + §5.5 + §10 + inv #10 cited `NonceAuditContext` at `:1235`. Actual location is `:1158`. `:1235` is `audit_event`. V1.3 fixes citations."

§3 substrate table (packet line 124): "`NonceAuditContext` (audit-event builder) | `surface_nonce.rs:1158` (V1.3 correction...)" — CORRECT.

§5.5 (packet line 390): "`surface_nonce.rs:1158` (V1.3 correction — V1.2 cited `:1235` which is `audit_event`, the event constructor)" — CORRECT.

§9 invariant #10 (packet line 547): "`NonceAuditContext` MUST carry `attempted_wp_user_id`, `attempted_surface_client_id`, `ip_hash`, `user_agent_hash` slots. Grep gate on the struct definition at **`surface_nonce.rs:1235`**." — STALE. The correction did not propagate to the invariant.

The CI grep gate in invariant #10 points at `:1235` (`audit_event`), not `:1158` (`NonceAuditContext`). A grep against `:1235` looking for struct field names will either pass spuriously (if `audit_event` happens to be near those strings) or fail with no useful diagnostic. L1 implementing the gate will target the wrong line.

**Condition:** Update §9 invariant #10 to read `surface_nonce.rs:1158` (matching §3 and §5.5 which V1.3 already corrected).

---

### Finding 2 — LOW: §9 invariants #6 and #7 assert invented rejection-reason strings that contradict the cycle-3 correction

**Severity:** LOW — will cause §8.3 and §8.4 integration tests to assert wrong runtime strings, producing false-pass or false-fail depending on how the test serializes the enum.

**Evidence:**

§6 decision #11 (packet line 496): "W4 emits the existing subset; no new variants, no renaming. AC E, F, §8.3, §8.4 reference the enum surface, not a 3-variant subset." — V1.3 correction locks use of the existing `PresenceNonceRejectReason` enum verbatim.

§5.4 rejection-reason table (packet lines 360-371): all entries use the enum variant names (`Replayed`, `WrongUser`, `Invalidated`, `Expired`, etc.).

§9 invariant #6 (packet line 543): "Replay-rejection MUST emit `presence_nonce_rejected` with `rejection_reason: **"already_consumed"**`."

§9 invariant #7 (packet line 544): "`wp_user_id` mismatch MUST emit `presence_nonce_rejected` with `rejection_reason: **"wp_user_mismatch"**`."

The string `"already_consumed"` is not a `PresenceNonceRejectReason` variant name. The correct variant is `Replayed` (at `surface_nonce.rs:1078`). Similarly, `"wp_user_mismatch"` is not a variant name; the correct variant is `WrongUser`. These invented strings were the cycle-2 finding that V1.3 folded everywhere else — §6 #11, §5.4 table, AC E/F — but not in invariants #6 and #7.

If L1 writes §8.3 and §8.4 against invariants #6 and #7, the tests will assert `"already_consumed"` and `"wp_user_mismatch"` as the wire-format rejection reason, but the substrate will serialize the enum as `"Replayed"` and `"WrongUser"` (or their snake_case equivalents depending on the serializer). The tests will fail at runtime or pass only if the serializer happens to produce the invented strings — neither outcome is correct.

**Condition:** Update §9 invariant #6 to use `PresenceNonceRejectReason::Replayed` (and its serialized form, whatever the wire format is — AC F and §8.3 establish this) and invariant #7 to use `PresenceNonceRejectReason::WrongUser`.

---

### Advisory (50) — §9 invariant #11 references the non-existent W6-E gate

**Not blocking;** already visible in the cycle-3 chain.

§9 invariant #11 reads: "`RenderPolicyChannel::ALL` MUST include `WpBlockRenders`. Compile-time check via the `#[non_exhaustive]` channel-sweep gate from W6-E."

V1.3 §5.9 correctly states the W6-E gate "was never built" and designates §8.9 as the manual coverage gate. The invariant text was not updated to match. L1 will look for the W6-E gate to verify this invariant and find nothing. Recommended fix: update invariant #11 to reference the §8.9 PHPUnit assertion rather than the non-existent compile-time gate, consistent with the §5.9 and AC J V1.3 corrections. Surfaces as observation; the §8.9 test itself is correctly specified.

---

## Scope alignment summary

V1.3 is right-sized. The VerifiedNonce extension (one struct with 5 added fields, modified only inside the existing `verify_and_consume` body), the full enum enumeration (documentation-only change to §6 #11 and §5.4 table), the manual §8.9 gate (one PHPUnit file), and the §16 L1 appendix (no spec changes) are all proportionate to the cycle-3 findings they close. No new abstractions, tables, or service layers. File count unchanged from V1.2. The W4 goal (feedback writes through the substrate claim path on WP block render) is fully served by the current scope.

---

## Lock criteria update

- [x] Cycle-3 codex consult Finding 1 (audit event order): VERIFIED FOLDED. §5.8 step 5 + §8.10 now both assert `presence_nonce_issued → presence_nonce_verified → claim_feedback_recorded`. CLOSED.
- [x] Cycle-3 codex challenge HIGH #1 (verify_and_consume API mismatch + actor format): FOLDED. §5.4 pseudocode now references `SurfaceNonceStore::verify_and_consume` by name; `VerifiedNonce` extension specified; actor format corrected to `"user:wp:{wp_user_id}"`.
- [x] Cycle-3 codex challenge HIGH #2 (17-variant enum): FOLDED. §3, §5.4 table, §6 #11, AC F all updated to the full enum.
- [x] Cycle-3 codex challenge MEDIUM #3 (W6-E gate absent): FOLDED in §5.9. Manual §8.9 gate specified. Maintenance ticket for W6-E backfill filed.
- [x] Cycle-3 codex challenge LOW #4 (NonceAuditContext line citation): PARTIALLY FOLDED — §3 and §5.5 corrected to `:1158`; §9 invariant #10 NOT updated. See Finding 1.
- [x] Cycle-3 codex challenge LOW #5 (submit_feedback def line): FOLDED. §3 corrected to `:149`.
- [x] §12 open questions #3–#6: well-scoped for cycle-4 lock. CLOSED.
- [x] §16 L1 appendix: appropriately scoped. ACCEPTED.
- [ ] **Finding 1 condition (LOW):** Update §9 invariant #10 from `surface_nonce.rs:1235` to `surface_nonce.rs:1158`.
- [ ] **Finding 2 condition (LOW):** Update §9 invariants #6 and #7 from `"already_consumed"` / `"wp_user_mismatch"` to the `PresenceNonceRejectReason` variant names (`Replayed` / `WrongUser`), consistent with §6 #11 and the §5.4 rejection-reason table.
- [ ] **Advisory (not blocking):** Update §9 invariant #11 to reference §8.9 PHPUnit gate rather than the non-existent W6-E compile-time gate.

Packet locks when Findings 1 and 2 are folded. Both conditions are verifiable from packet text alone; no implementation required. Advisory can be folded at the same time.
