# Packet F — codex challenge — cycle 3

**Reviewer:** codex challenge (adversarial)
**Packet version:** V1.2 (commit d29d2884 on docs/v143-l0-packet-f-v1.1)
**Date:** 2026-05-19
**Verdict:** **CONDITIONAL APPROVE** — packet does NOT lock at cycle 3.

## 2-sentence summary

The V1.2 atomicity rewrite (§5.4) accurately drops the impossible single-tx framing and grounds against the existing Mutex-guarded HashMap store, but its pseudocode names a `try_mark_consumed(&request)` API that does not exist at that signature — the real consume orchestration is `SurfaceNonceStore::verify_and_consume` (returns `VerifiedNonce`, NOT the binding the pseudocode then dereferences). Three additional mechanical gaps surface: the `actor` string format `"wp_user:{}"` will be rejected by `validate_feedback_actor`, the §6 #11 "3 reject reasons cover all paths" claim is empirically false (5+ other variants are already emitted by `verify_and_consume`), and the W6-E `#[non_exhaustive]` channel-sweep gate that §5.9 / invariant #11 / AC J leans on does not exist in the tree today.

---

## Fold verification — per cycle-2 condition

### 1. §5.4 atomicity rewrite — PARTIAL FOLD

The framing is correct (Mutex IS the atomicity primitive; SQL-tx wrap was impossible; consume-then-record with fail-closed; `db_read → db_write` upgrade). All correct against substrate.

**But the pseudocode is mechanically wrong in three places:**

1. **API surface mismatch at packet line 245.** Packet writes:
   ```rust
   let binding = surface_nonce_service.try_mark_consumed(&request)?;
   ```
   The actual `try_mark_consumed` at `surface_nonce.rs:518` is `pub fn try_mark_consumed(&mut self, now: DateTime<Utc>) -> Result<(), LifecycleRace>` — a method on `PresenceNonceBinding` that takes only `now`, returns `()` (or a `LifecycleRace` enum), and lives INSIDE the Mutex critical section of `verify_and_consume` at `:725`. It is NOT a service-level method that takes a `VerifyNonceRequest` and returns a binding. The orchestration the packet describes (digest lookup → binding compare → consume → return binding) is the body of `SurfaceNonceStore::verify_and_consume` at `:640-742`, which returns `VerifiedNonce { consumed_at, nonce_digest, expected_claim_version, expected_composition_version }` — note: NO `action`, NO `claim_id`, NO `payload_json`, NO `wp_user_id`, NO `session_id` on the return shape. The Mutex is dropped before return; the binding stays in the store.

2. **Phase-2 field access at packet lines 250-257.** Pseudocode reads `binding.action`, `binding.claim_id`, `binding.wp_user_id`, `binding.session_id`, `binding.payload_json`. These live on `PresenceNonceBindingFields`, not `VerifiedNonce`. Either the consume API has to return the binding (mutate `verify_and_consume`'s signature), OR phase-2 has to do a second store lookup-by-digest (which races against composition invalidation between consume and lookup), OR these fields need to be plumbed into `VerifiedNonce`. The packet picks none of the three and asserts the fields are reachable. **L1 will discover this within minutes and rewrite §5.4 ad hoc** — that re-architecting belongs in the packet, not in implementation.

3. **Actor format string at packet line 254.** Pseudocode constructs `actor: format!("wp_user:{}", binding.wp_user_id)`. `record_claim_feedback` calls `validate_feedback_actor(&input.actor)` at `claims.rs:6721`, which calls `actor_class_for_actor` at `:5112`. That function splits the actor string on `:` / `/` / `@`, takes the head, and matches against the literal allowlist `["user", "human", …]`. The head `"wp_user"` matches NONE of the User-class prefixes; `actor_class_for_actor` returns `None`; the validator returns `Err(ClaimError::InvalidFeedback("actor 'wp_user:42' does not map to a registered actor class"))`. **Every feedback verify request in the packet's design crashes here at run time.** The fix is trivial (use `format!("user:wp/{}", wp_user_id)` or similar), but the packet locks an actor format that is mechanically broken; AC D / AC I will both fail when L1 ships the pseudocode literally.

**Verdict on §5.4:** the *atomicity model* is right; the *pseudocode encoding* of that model is wrong at three load-bearing points. Cycle-3 fix: tighten §5.4 to reference `SurfaceNonceStore::verify_and_consume` by name (not `try_mark_consumed`), specify how the binding fields reach phase-2 (signature change OR second lookup OR field plumbing into `VerifiedNonce` — pick one), and update the actor format to one that survives `actor_class_for_actor`.

### 2. §5.6 WP /verify + allowlist + orphan retirement — REAL FOLD

- WP-side allowlist at `class-dailyos-plugin.php:907` confirmed hardcoded to the old 4 strings (`'correct', 'dismiss', 'corroborate', 'contradict'`). §5.6 item 2 will mechanically work.
- `submit_feedback()` at `runtime-client.php:149` (def; :163 is the path call — minor citation drift in packet, calls the def line "163"): zero live PHP callers (grepped `wp/` tree). Safe to retire. Note: one Rust test fixture at `dos567_fixture_wrong_user_rejected.rs:108` uses the string `"submit_feedback"` as an ability name, NOT a call to the PHP method — does not block retirement.
- `/v1/surface/feedback` orphan allowlist entries at `surface_runtime/mod.rs:1243, 4730, 4755` confirmed (cycle-2 already verified; not re-checking).

This fold is real.

### 3. §5.9 `RenderPolicyChannel::WpBlockRenders` — FOLD MISLEADS ON GATE STATUS

The enum at `bridges/types.rs:84-94` confirmed at 9 variants today. Adding `WpBlockRenders` as the 10th is mechanically trivial: update enum, update `const ALL: [Self; 9]` to `[Self; 10]`, update `as_str` match. Buildable.

**But:** the W6-E channel-sweep gate that §5.9 / invariant #11 / AC J cite as the consuming mechanism **does not exist in the tree**. Specifically:
- `RenderPolicyChannel` is NOT annotated `#[non_exhaustive]` (the W6-E packet's stated mechanism).
- `RenderPolicyChannel::ALL` has zero consumers outside its own definition file (grepped `src-tauri/src/`).
- The "W2 DOS-477 leak-guard machinery" the packet says "applies automatically" via the channel sweep — does not iterate this enum.

The W6-E packet at `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:189` describes the **design** of the gate; it isn't wired. So §5.9's effect is: add a 10th variant to a registry that nothing reads. AC J's claim that adding the variant "is included in the W6-E channel-sweep gate" cannot be tested because that gate doesn't run yet.

This is a NEW finding for cycle 3 (cycle 2 didn't check whether the gate exists, only that the channel count was 9). Either the gate ships in W4 as part of the §5.9 work (scope expansion — needs explicit packet decision), or §5.9 + AC J are downgraded to "register the channel; gate consumption deferred to whichever wave ships W6-E."

### 4. §6 decisions — INCOMPLETE ON #11

- **#11 (use existing PresenceNonceRejectReason) — fold is partial.** Packet says "3 existing variants (`Replayed`, `WrongUser`, `MalformedRequest`) cover all W4 reject paths" (§5.4 reject table + §6 #11 + AC E/F + §8.3/§8.4). Grep of `surface_nonce.rs:1067-1085` shows 17 variants in the enum and current emission of at LEAST these from `verify_and_consume` + `compare_binding_tuple`: `Invalidated` (`:651`), `Expired` (`:679`), `ClaimVersionStale` (`:694`), `CompositionVersionStale` (`:718`), `WrongClaim` (`:690`), `WrongField` (`:138`), `MismatchedAction` (`:1815`), `WrongActor` (`:1288`), `WrongSession` (`:1309`), `RateLimited` (`:1046`). Wiring the verify route to `record_claim_feedback` does NOT eliminate these — they fire BEFORE the wire-through, inside `verify_and_consume`. AC F / §8.4 assertions that `rejection_reason` equals one of {Replayed, WrongUser, MalformedRequest} on the WrongUser test fixture will pass for that test case, but the broader claim "these 3 cover all W4 paths" is wrong and will leak into how L1 writes the audit-correlation test (§8.10) — they'll either miss assertions on the other emission paths or write false assertions.
  - **Cycle-3 fix:** §6 #11 narrows from "3 variants cover all paths" to "W4 reuses the existing 17-variant `PresenceNonceRejectReason`; tests assert exact variants per-scenario; no invented strings." Tables in §5.4 + §5.5 + AC F drop the "all paths covered by 3" framing.

- **#12 (LLM injection forward-constraint on `corrected_text`)** — clean fold. Documents current state (no LLM consumer), names the constraint for future work, points at the existing projection filter. No issue.

- **#13 (consume-succeeded-record-failed is real, not TODO)** — clean fold. Explicit fail-closed semantics, no rollback, user retries with new nonce. Good.

**Missing from §6:** no decision locks the resolution of the §5.4 pseudocode mismatch (signature change vs. second lookup vs. `VerifiedNonce` field plumbing — see Finding 1 above). This is a load-bearing decision; without it L1 is choosing the integration shape ad hoc.

### 5. §14 AC — TESTABILITY GAPS

Walked each AC against packet text alone.

- **AC A** (9 variants, `as_str ↔ parse` round-trip, `From<PresenceNonceAction> for FeedbackAction` exhaustive match): testable from packet alone. Clean.
- **AC B** ("tamper-resistant via in-memory store isolation"): testable. The wording correction from cycle 2 is faithful.
- **AC C** (`wp_user_id` derive-from-session + binding check at `:1339`): testable. Cycle-2 fold is real.
- **AC D** (verify → record_claim_feedback wire; `db_write` at `:2570`; Mutex on `SurfaceNonceStore` as atomicity primitive): **partially testable**. Wiring + handler upgrade are testable. The "Mutex on `SurfaceNonceStore`" claim needs the §5.4 pseudocode to be mechanically right first (see Finding 1).
- **AC E** (replay rejection via Mutex; "exactly one's `try_mark_consumed` returns `Ok(binding)`, the rest return `Replayed`"): **untestable as worded**. `try_mark_consumed` does not return a binding (see Finding 1). The behavior described IS what `verify_and_consume` does, but AC E names the wrong API. L1 will either rename or rewrite.
- **AC F** (audit payload extensions + `rejection_reason ∈ {Replayed, WrongUser, MalformedRequest}`): see Finding 4. The subset claim is wrong; AC F needs widening or per-scenario asserting.
- **AC G** (WP REST handler + 9-variant allowlist + payload_json validation): testable. Cycle-2 fold is real.
- **AC H** (JS affordance): testable. UX detail, not adversarial concern.
- **AC I** (E2E fixture for 4 variants): **blocked on Finding 1**. The fixture will hit the actor-format wall (`"wp_user:N"` → `actor_class_for_actor` returns `None`) on the first feedback write.
- **AC J** (no leak through `payload_json` projection; `WpBlockRenders` in sweep): **partially testable**. The leak test is testable. The "in the W6-E sweep" claim is not — the sweep doesn't exist (see Finding 3).
- **AC K** (orphan retired): testable. Clean fold.
- **AC L** (CI gates): testable in principle; invariant #11 has the same gate-doesn't-exist issue as AC J.
- **AC M** (L2 unanimous): meta — n/a.
- **AC N** (L4 hands-on): operational — n/a.
- **AC O** (CSO + security-auditor APPROVE): meta — n/a.
- **AC P** (no regressions): testable in principle.

**Net AC gap:** ACs D, E, F, I, J, L hinge on three findings (1, 3, 4) above. None are handwaves about *what* should happen; they are encoding errors about *which API + which signature + which gate*. Cycle-3 fixes are surgical.

---

## Top findings (cycle 3)

| # | Severity | Section | Issue |
|---|----------|---------|-------|
| 1 | **HIGH** | §5.4 pseudocode | Names `try_mark_consumed(&request)` returning a binding; actual consume orchestration is `SurfaceNonceStore::verify_and_consume` returning `VerifiedNonce` (no binding fields exposed). Pseudocode field reads at lines 250-257 require either signature change, second lookup, or `VerifiedNonce` field plumbing — packet picks none. Plus the `actor: format!("wp_user:{}", …)` format is rejected by `actor_class_for_actor` at `claims.rs:5112`. AC D / E / I encode the broken shape. |
| 2 | **HIGH** | §6 #11 + §5.4 table + AC F + §8.3-8.4 | "3 reject reasons (`Replayed`, `WrongUser`, `MalformedRequest`) cover all W4 paths" is empirically false. `verify_and_consume` already emits `Invalidated`, `Expired`, `ClaimVersionStale`, `CompositionVersionStale`, `WrongClaim`, `WrongField`, `MismatchedAction`, `WrongActor`, `WrongSession`, `RateLimited` per existing call sites in `surface_nonce.rs`. AC F's subset assertion is too narrow; widen to "use the existing 17-variant enum; tests assert exact variants per-scenario." |
| 3 | **MEDIUM** | §5.9 + invariant #11 + AC J + AC L | The W6-E `#[non_exhaustive]` channel-sweep gate the packet leans on does not exist in `src-tauri/`. `RenderPolicyChannel` has no `#[non_exhaustive]` attribute and zero non-definition consumers of `ALL`. Adding `WpBlockRenders` works mechanically; the consuming gate that AC J asserts auto-applies does not exist. Either ship the gate in W4 (scope decision) or downgrade §5.9 / AC J to "register variant; consumer gate deferred." |
| 4 | **LOW** | §3 + §5.5 + §10 commit 2 + invariant #10 | `NonceAuditContext` cited at `:1235` — actual location `:1158`. `audit_event` is at `:1235`. Citation drift on a structural anchor invariant #10 + commit 2 lock onto. Update to `:1158`. |
| 5 | **LOW** | §3 + AC K + §5.6 item 4 | `submit_feedback()` definition is at `runtime-client.php:149`; `:163` is the path-call line inside the function body. Packet's "del runtime-client.php:163 submit_feedback()" reads as if `:163` IS the def. Update to "delete the `submit_feedback()` function spanning `:149-167` (or actual end line)" to avoid an L1 partial-deletion footgun. |

## Verdict rationale

Cycle-2 verdict was CONDITIONAL APPROVE with two HIGH findings (§5.4 + §5.6) + 1 LOW (channel verified at 9). V1.2 folded those correctly **at the framing level** but the §5.4 pseudocode encoding introduces THREE mechanical gaps (Finding 1) that L1 will hit within the first commit, and §6 #11 + §5.9 introduce TWO new soft-claim findings (Findings 2, 3) that read clean in prose but break against substrate. None are class-pattern recurrence (cycle-2 was about atomicity; cycle-3 finds encoding-vs-substrate drift in the rewrite). All are surgical — pseudocode patch + decision tightening + gate-status acknowledgment.

**Per `feedback_review_loop_l6_policy.md`:** 3 cycles, no class-pattern, surgical conditions. Keep looping. Cycle-3 close-out fold needed; cycle-4 should be the lock cycle if cycle-3 fixes Findings 1-3.

**Per §15 lock criteria:** packet does NOT lock at cycle 3. Findings 1 + 2 are HIGH and block AC D/E/F/I testability from packet text alone.

