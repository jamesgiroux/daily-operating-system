# Packet F — codex challenge — cycle 4 (lock verification)

**Reviewer:** codex challenge (adversarial)
**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md`
**Version under review:** V1.3 (commit `d2f0c4ae` on `docs/v143-l0-packet-f-v1.1`)
**Date:** 2026-05-19
**Verdict:** **CONDITIONAL APPROVE** with two strictly surgical conditions on a single section (§5.4 / §16 L1-note #1); cycle-4 lock holds if both are folded inline. Otherwise BLOCK and re-spin cycle-5.

---

## Summary

V1.3 folds 4 of the 5 cycle-3 findings cleanly and three of them (HIGH #2 full enum, LOW #4 `:1158`, LOW #5 `:149`, codex-consult LOW audit order) are verifiable against substrate to the file:line. The MEDIUM #3 (W6-E gate) fold reaches the right disposition (manual §8.9 + maintenance backfill) but rests on two factually wrong substrate claims, and the HIGH #1 (§5.4 rewrite) fold swings past center — the pseudocode now points L1 at the wrong API entry point (`app_state.surface_nonce_store.verify_and_consume(...)`), bypassing the validation, budget-charging, and audit-event construction that `SurfaceNonceService::verify_nonce` does around `verify_and_consume`.

---

## Per-fold verification

### HIGH #1 — §5.4 pseudocode + actor format — **PARTIAL FOLD**

Substrate facts:
- `SurfaceNonceStore::verify_and_consume` at `surface_nonce.rs:640` — declared `fn` (private). Confirmed.
- `VerifiedNonce` at `surface_nonce.rs:583` — currently `consumed_at, nonce_digest, expected_claim_version, expected_composition_version` only. Packet's commit-3 "extend with `claim_id, action, payload_json, wp_user_id, session_id`" is structurally valid. Confirmed.
- Actor format `"user:wp:{wp_user_id}"` — head splits on `:` per `actor_class_for_actor`; `"user"` is in the allowlist. Confirmed.
- `db_read → db_write` named at §5.4 + §10 commit 3. Confirmed.

**Problem:** the packet's step-2 pseudocode at lines 287–339 shows:
```
app_state.db_write(|db, ctx| {
    ...
    let verified = app_state.surface_nonce_store.verify_and_consume(ctx, db, digest, &request, now, audit)?;
    ...
})
```

Two structural issues with this call site:

1. **There is no `app_state.surface_nonce_store` field.** The store is `SurfaceNonceService.inner.store` (private `SurfaceNonceServiceInner`). The runtime carries `runtime.surface_nonce: SurfaceNonceService` (`surface_runtime/mod.rs:597`). To reach `verify_and_consume` from a handler, the substrate either needs a `pub` promotion of `SurfaceNonceStore` / `verify_and_consume` / `VerifiedNonce` (widens the crate API surface and exposes a private struct), or a new `pub fn` wrapper on `SurfaceNonceService` (creates a parallel verify path next to the existing `verify_nonce` at `:218`). §16 L1-note #1 frames this as a "noun chain" fix; it is actually a packet-level choice with two real options and different blast radii.

2. **`verify_and_consume` is not the right call site.** The existing `SurfaceNonceService::verify_nonce` at `:218` does load-bearing pre-work around `verify_and_consume`: `ensure_surface_client`, `VerifyNonceRequest::parse`, `NonceAuditContext::from_verify`, `ensure_session_tuple`, `charge_budget(NonceBudgetClass::Verify)`, `decode_presence_nonce`, digest computation, `charge_failure_best_effort` on the failure path, AND the `presence_nonce_verified` audit-event CONSTRUCTION at `:257-263` (the event is built in `verify_nonce`, not in `verify_and_consume` or `try_mark_consumed`). The packet's pseudocode calls `verify_and_consume` directly with a pre-built audit context and bypasses all of that.

   The correct shape: extend `SurfaceNonceService::verify_nonce` (or add a sibling `pub fn verify_nonce_and_record_feedback`) that performs the existing pre-work, calls `verify_and_consume`, threads the extended `VerifiedNonce` fields out, and proceeds into `record_claim_feedback`. Pseudocode + §16 L1-note #1 currently point L1 at a path that skips validation gates and audit emission.

3. **Related audit-ordering claim is shaky.** V1.3 fold #6 (codex-consult LOW) says: "`presence_nonce_verified` fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation, BEFORE `record_claim_feedback` is called." Substantively the audit DOES fire before `record_claim_feedback`, but the mechanism in the parenthetical is wrong: the audit event is constructed by `verify_nonce` at `:257-263` after `verify_and_consume` returns, not inside the Mutex-guarded mutation. The §5.4 / §8.10 audit-order assertion holds; the mechanism-of-emission gloss does not.

**Condition for lock:** §5.4 step-2 pseudocode and §16 L1-note #1 must be rewritten to (a) target `SurfaceNonceService::verify_nonce` (or a new `pub fn` sibling on the service) as the extension site, NOT `app_state.surface_nonce_store.verify_and_consume`; (b) explicitly call out that the existing `ensure_surface_client → parse → ensure_session_tuple → charge_budget → decode → digest → verify_and_consume → audit construction → emit` chain at `:218-286` is the orchestration that wraps `verify_and_consume`; (c) clarify in §5.8 #5 / §8.10 that the audit event is constructed in the service method, not inside the Mutex-guarded mutation (the temporal claim "before record_claim_feedback" is unaffected).

### HIGH #2 — full `PresenceNonceRejectReason` enum — **CLEAN FOLD**

Substrate: `surface_nonce.rs:1065-1085` defines the enum with 17 variants in the exact order the packet enumerates (`MalformedRequest, MissingNonce, MalformedClaimVersion, UnauthenticatedSurface, WrongActor, ScopeDenied, WrongSession, WrongUser, WrongClaim, WrongField, MismatchedAction, Expired, Replayed, Invalidated, ClaimVersionStale, CompositionVersionStale, RateLimited`). `as_str` impl at `:1087` confirms the snake_case wire names. §3 substrate table, §5.4 failure-mode mapping, §6 #11, AC F all reference the full enum verbatim. No new variants invented, no renaming. Fold verifiable from packet text alone.

### MEDIUM #3 — W6-E gate doesn't exist — **WRONG SUBSTRATE READ; RIGHT DISPOSITION**

Two substrate claims in V1.3 §5.9 are factually wrong:

1. Packet says: "`RenderPolicyChannel` at `bridges/types.rs:82-94` is plain `#[derive]`, NOT `#[non_exhaustive]`."  
   **Substrate (`bridges/types.rs:81-94`):** `#[non_exhaustive]` IS present on the enum at line 81. The packet read the wrong line range.

2. Packet says: "`ALL` at `:97` is `[Self; 9]` fixed-size; zero non-definition consumers (`grep -r "RenderPolicyChannel::ALL"` returns 0 hits outside the definition site)."  
   **Substrate:** the `pub` accessor is `pub const fn all() -> &'static [Self]` at `:109`, NOT `pub const ALL`. A grep for the literal `::ALL` returns 0 because consumers use `::all()`. The actual consumer is `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs` at `:359` (`assert_eq!(RenderPolicyChannel::all().len(), 9)`), `:372` (iterates rows against `all().len()`), `:378` (fails with "matrix channel {channel} is not registered in `RenderPolicyChannel::all()`"), `:654`. There IS an active sweep gate; it is in `bundle-17`, not in `bridges/types.rs`. Adding `WpBlockRenders` will break that test (the `len() == 9` assertion + the matrix-rows comparison) unless §10 commit 5 also updates `bundle17_source_lifecycle_actor_provenance_substrate_test.rs` and the bundle-17 fixtures (`metadata.json`, `expected_state.json`, `expected_output.json` all reference `RenderPolicyChannel::all`).

The DISPOSITION — drop the auto-application claim, lean §8.9 as the manual coverage gate, file backfill as maintenance — is still correct (the bundle-17 test asserts channel count and matrix coverage but does NOT enforce that every test site sweeps `ALL` for projection redaction; that gap is still real). But two of the three reasoning steps the packet uses to reach that disposition are factually wrong, and the §10 commit 5 task is bigger than the packet says (must also touch bundle-17 test + fixtures, otherwise commit 5 ships red).

**Condition for lock:** §5.9 must (a) correct the `#[non_exhaustive]` claim — it IS present; the gap is the absence of compile-time consumer enforcement, not the attribute; (b) name `bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359,372,378,654` + the three `tests/fixtures/bundle-17/` JSON files as the existing channel-sweep test that §10 commit 5 must update (`len() == 9 → 10`, matrix rows, expected_channel_matrix snapshot); (c) the maintenance-backfill DOS-### TBD then narrows to "extend bundle-17 sweep into a compile-time enforcement via `match` exhaustiveness on `ALL`," not "add `#[non_exhaustive]`."

### LOW #4 — `NonceAuditContext` at `:1158` — **CLEAN FOLD**

Substrate: `struct NonceAuditContext` at `:1158`. Confirmed. §3 substrate table, §5.5, §10, §9 inv #10 citations consistent. (Note: §9 inv #10 still says `surface_nonce.rs:1235` in the grep-gate target line — minor residual; doesn't affect fold validity but is a typo carrier.)

**Advisory (not lock-blocking):** §9 invariant #10 line still says "Grep gate on the struct definition at `surface_nonce.rs:1235`." Should be `:1158` to match §3 + §5.5.

### LOW #5 — `submit_feedback()` at `:149` — **CLEAN FOLD**

Substrate: `runtime-client.php:149` declares `public function submit_feedback(...)`; `:163` is the `signed_post('/v1/surface/feedback', $body_bytes)` line. Packet correctly distinguishes the two and points L1 at `:149` for deletion. AC K, §3, §5.6 step 4, §13 cite `:149` (def) and call out `:163` only as the internal-call line.

**Advisory (not lock-blocking):** §10 commit 3 line still says "retire `runtime-client.php:163` `submit_feedback()`" — should be `:149`. AC K also says "`runtime-client.php:163` `submit_feedback()` deleted" — should be `:149`. §16 doesn't touch this; both residual `:163` mentions in §10 + AC K survive from V1.2.

### codex-consult LOW — audit ordering — **PARTIAL FOLD**

V1.3 corrects the order to `issued → verified → claim_feedback_recorded` at §5.4, §5.8 #5, §8.10. Correct directionally. The mechanism-of-emission gloss ("fires inside `verify_and_consume`'s Mutex-guarded HashMap mutation") at §5.8 #5 and §8.10 is wrong — see HIGH #1 condition above. Already covered.

---

## §16 L1 implementation notes appendix

8 items. Verification:

1. **§5.4 noun chain (verify_and_consume on SurfaceNonceStore not SurfaceNonceService).** Load-bearing but **understates the choice** — see HIGH #1 partial-fold condition. L1 isn't picking between `app_state.surface_nonce_store` vs "service-level wrapper" as two equivalent reachability paths; the substrate has no `surface_nonce_store` accessor and the right answer is "extend the service method, don't reach past it." This note papers over the HIGH #1 finding.

2. **`[Self; 9] → [Self; 10]`.** Verified at `bridges/types.rs:97`. Load-bearing; correct.

3. **`From<PresenceNonceAction> for FeedbackAction` crate boundary.** Verified — `PresenceNonceAction` is in `src-tauri/src/services/surface_nonce.rs` (`dailyos_lib` crate); `FeedbackAction` is in `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` (separate crate). Real choice point; the note correctly enumerates three options. Load-bearing.

4. **Deployment ordering for orphan retirement.** Load-bearing for any future remote deploy; correctly scoped as advisory for local-to-local current model. Fine.

5. **Phase-3 failure budget charging.** Load-bearing — without this, an attacker can exhaust nonces by repeatedly triggering record_claim_feedback failures with shape-valid-on-WP-but-invalid-on-Rust payloads. Real attack path; not nice-to-have.

6. **HKDF `info` parameter distinct.** Load-bearing — cryptographic-purpose binding is the kind of thing where "we'll just reuse the existing constant" is the silent footgun. Good capture.

7. **Permission callback literal reuse.** Defensive; not strictly load-bearing but cheap and prevents drift. Fine.

8. **WP `payload_json` non-plain-object rejection.** Defensive; pairs with §5.6 step 3's variant-specific shape validation. Fine.

**Net:** 7 of 8 notes are load-bearing or genuinely defensive. Note #1 papers over the HIGH #1 fold's structural misdirection — it should be promoted into a packet-level §5.4 correction rather than left as an L1 FYI.

---

## Adversarial probe — new gaps the V1.3 rewrite may have introduced

1. **`VerifiedNonce` extension scope.** Packet says commit 3 extends `VerifiedNonce` with 5 binding fields. `VerifiedNonce` is a private struct at `:583`; the only caller is `SurfaceNonceService::verify_nonce` at `:248-285`. Extension is structurally safe (no callers outside `surface_nonce.rs`). However, the extension changes `verify_and_consume`'s return-value width — if any future caller (post-W4) wants the original lean `VerifiedNonce`, they pay the cost. This is fine for W4 lock but worth noting as a tight-coupling decision; the alternative would be a separate `VerifiedNonceForFeedback` returned by a new service method. Not lock-blocking; advisory.

2. **§5.9 backout vs. existing bundle-17 sweep.** Already covered under MEDIUM #3 above — adding `WpBlockRenders` will break `bundle-17` tests unless §10 commit 5 also updates the test + 3 fixtures. The packet currently does NOT name this in commit 5 (commit 5 says only "extension at `bridges/types.rs` + 1 ALL entry + W6-E `#[non_exhaustive]` channel-sweep gate exercise" — the last clause is now wrong since the auto-gate doesn't exist; the real work is the bundle-17 fixture update).

3. **§9 invariant #11 stale claim.** Still says: "`RenderPolicyChannel::ALL` MUST include `WpBlockRenders`. Compile-time check via the `#[non_exhaustive]` channel-sweep gate from W6-E." V1.3 §5.9 correctly drops the W6-E claim — but §9 invariant #11 retains the stale wording. Should be: "Compile-time check via the bundle-17 channel matrix at `tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs:372-378` (or equivalent). W6-E `#[non_exhaustive]` auto-gate is backfill-only — see maintenance ticket." Currently AC J references "the W6-E `#[non_exhaustive]` channel-sweep auto-gate doesn't exist today (W6-E planned but not built)" — that resolution is locked in AC J, but §9 inv #11 wasn't updated to match.

4. **§16 note #1 papers over HIGH #1.** Already covered above.

---

## Lock disposition

V1.3 is **close enough to lock** that cycle-5 is not warranted. The remaining gaps are:

- **(structural)** §5.4 step-2 pseudocode + §16 note #1 — wrong API entry point. Rewrite to target `SurfaceNonceService::verify_nonce`-or-sibling, not `app_state.surface_nonce_store.verify_and_consume`.
- **(structural)** §5.9 — correct the two factual substrate misreads (`#[non_exhaustive]` IS present; sweep consumer exists at bundle-17); name the bundle-17 test + 3 fixture updates as in-scope for §10 commit 5.
- **(consistency)** §9 inv #10 still says `:1235`; should be `:1158`.
- **(consistency)** §10 commit 3 + AC K still say `runtime-client.php:163`; should be `:149`.
- **(consistency)** §9 inv #11 wording still leans on the non-existent W6-E auto-gate; AC J resolves it correctly but the invariant text drifted.
- **(commit-5)** §10 commit 5's "W6-E `#[non_exhaustive]` channel-sweep gate exercise" clause is dead — replace with "update `bundle17_source_lifecycle_actor_provenance_substrate_test.rs:359,372,378,654` + three `tests/fixtures/bundle-17/*.json` matrix snapshots to include `WpBlockRenders`."

**Verdict: CONDITIONAL APPROVE.** Two folds (HIGH #1, MEDIUM #3) are partial — the rewrites are directionally correct but rest on substrate misreads that point L1 at wrong code. The 4 cleanup-consistency items (§9 inv #10 `:1235`, §9 inv #11 wording, §10 commit 3 + AC K `:163`, §10 commit 5 dead clause) are surgical citation/text fixes. If all six are folded inline as a V1.4 patch and re-verified against substrate (no cycle-5 reviewer set required since these are corrections-to-corrections), the packet locks.

If they are NOT folded, **BLOCK** — L1 will hit the wrong API at `verify_and_consume` and ship a commit 5 that fails CI on the bundle-17 sweep test.
