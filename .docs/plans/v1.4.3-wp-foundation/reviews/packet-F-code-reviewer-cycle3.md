# Packet F — code-reviewer cycle-3 review

**Date:** 2026-05-19
**Reviewer:** code-reviewer (feasibility lens)
**Packet:** L0-packet-F-feedback-write-infrastructure.md V1.2
**Prior cycle-2 verdict:** CONDITIONAL APPROVE

## Verdict: APPROVE

V1.2 grounds every substrate claim against actually-grepped file paths and resolves the three cycle-2 HIGH findings (§5.4 atomicity rewrite, rejection-reason naming, `/v1/surface/feedback` orphan retirement) with mechanisms that are buildable as described. The remaining nits are documentation precision, not implementability blockers — surfaced as FYI so commit-1 author doesn't trip on them.

## Verification of each cycle-3 check

### 1. §5.4 atomicity model — VERIFIED BUILDABLE

`PresenceNonceBinding::try_mark_consumed` at `surface_nonce.rs:518` confirmed: `pub fn try_mark_consumed(&mut self, now: DateTime<Utc>) -> Result<(), LifecycleRace>`. It is a method on **`PresenceNonceBinding`**, NOT `SurfaceNonceService` — i.e., it mutates a single binding once the Mutex-guarded `SurfaceNonceStore` has handed it out. The packet's pseudocode line `surface_nonce_service.try_mark_consumed(&request)?` is therefore *shorthand* for "go through the existing public entry point `SurfaceNonceService::verify_nonce` at `:218` which internally calls `store.verify_and_consume` at `:640` which holds the Mutex and calls `binding.try_mark_consumed`."

`record_claim_feedback` at `claims.rs:6700` confirmed: `pub fn record_claim_feedback(ctx: &ServiceContext<'_>, db: &ActionDb, input: ClaimFeedbackInput) -> Result<ClaimFeedbackOutcome, ClaimError>`. Takes `&ActionDb`; opens its own `with_claim_transaction` at `:6717`. Confirmed: cannot be wrapped in a caller-side tx.

Net implementability: BUILDABLE. The §5.4 pseudocode reads correctly as the *flow*, but L1 implementer must understand the public surface is `SurfaceNonceService::verify_nonce` (which returns `SurfaceNonceVerify`), not a hypothetical `try_mark_consumed(&request)` on the service. Suggest tightening pseudocode to call `verify_nonce` directly and then call `record_claim_feedback` from the wrapper at `surface_runtime/mod.rs:2019` — this is what commit 3 actually does. **FYI, not blocking.**

### 2. §5.6 WP REST handlers — VERIFIED ACCURATELY DESCRIBED

`class-dailyos-plugin.php:568-602` confirmed: only three routes registered — `/dailyos/v1/nonce` (POST), `/dailyos/v1/account-overview/preview` (POST), `/dailyos/v1/account-overview/accounts` (GET). No `/nonce/verify` exists. V1.2 §5.6 item 1 (register `/dailyos/v1/nonce/verify`) is accurate.

`class-dailyos-plugin.php:907` confirmed: hardcoded array literal `[ 'correct', 'dismiss', 'corroborate', 'contradict' ]`. V1.2 §5.6 item 2 + AC G + invariant #9 (replace with the 9 `FeedbackAction` variants) is a mechanical change. Buildable as one-line edit + matching PHPUnit (§8.8).

### 3. §5.9 `RenderPolicyChannel` — VERIFIED ACCURATE

`bridges/types.rs:84-94` confirmed 9 variants. `ALL` at `:97-107` is `const ALL: [Self; 9]` (note: literal size `9`, not a generic) — L1 implementer must update the type annotation to `[Self; 10]` (or refactor to a slice) when adding `WpBlockRenders`. The packet's V1.2 §5.9 illustrative code uses `&'static [RenderPolicyChannel]` (a slice), which works but is a different shape than the existing `[Self; 9]` fixed array. Suggest L1 keep the fixed-array shape with size `10` so the `#[non_exhaustive]` channel-sweep gate can still depend on `Self::ALL.len()`. **FYI, not blocking.**

### 4. §3 substrate table spot-check — VERIFIED ACCURATE

Spot-checked 4 lines:
- `try_mark_consumed` at `surface_nonce.rs:518` — confirmed (line 518 exact).
- `RenderPolicyChannel::ALL` at `bridges/types.rs:84-126` — confirmed (84 start, 126 close brace).
- `PresenceNonceAction` 4 variants at `surface_nonce.rs:445` — confirmed (variants at 446-449).
- WP issue handler at `class-dailyos-plugin.php:573-601` — confirmed (`/nonce` POST registration spans 573-581; full `register_rest_routes` ends at 602).

Substrate-grep K-in obligation satisfied; no parallel-substrate risk remaining.

### 5. §10 commit 1 (`PresenceNonceAction` extension) — VERIFIED INDEPENDENTLY BUILDABLE

Production callers of `PresenceNonceAction::*` enumerated via grep: `surface_nonce.rs:854` + `:914` (the `parse` call inside `IssueNonceRequest::parse` / `VerifyNonceRequest::parse`), `:1252` (audit `as_str` rendering), and `:1690` (the test fixture caller flagged in V1.2 cycle-2 finding #10). All four are inside `surface_nonce.rs` itself — no external crate touches `PresenceNonceAction::Correct` etc. directly. Confirmed: extension from 4 → 9 + adding `From<PresenceNonceAction> for FeedbackAction` impl + updating `:1690` fixture is a self-contained commit with a self-contained Rust unit test (§8.1).

**Minor implementability nit (FYI):** the `parse`/`as_str` methods on `PresenceNonceAction` are currently crate-private (no `pub` qualifier at `:452-471`). The `From<PresenceNonceAction> for FeedbackAction` impl planned for commit 1 must live in the same crate (`src-tauri`), OR the methods need to be promoted to `pub`. Confirmed `FeedbackAction` is defined in `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` — a *different* crate. Commit 1 will need to either:
   (a) promote `parse` and `as_str` to `pub`,
   (b) move the `From` impl into the `surface_nonce` module, or
   (c) re-export via a public type alias.

This is a 2-line decision, not an architectural one. Suggest L1 author note the choice in the commit message. **FYI, not blocking.**

## Findings summary

- **0 BLOCK**
- **0 HIGH**
- **0 MEDIUM**
- **3 FYI** (pseudocode shorthand for `try_mark_consumed`, `ALL` array-shape vs slice-shape, `From` impl crate-boundary choice) — implementability nits that L1 will resolve at the keyboard; no architectural decision deferred.

## Class-pattern check

V1.2 carries no recurrence of the V1.0 parallel-substrate class-pattern. Cycle-2 verdicts were 5/5 CONDITIONAL APPROVE with surgical conditions; V1.2's fold addressed each named condition. Per `feedback_l2_path_alpha_to_maintenance_project.md`, the remaining FYIs (above) are pure path-α implementability nits and would file as maintenance follow-ups at most — but they're load-light enough to land inside the commit-1/3/5 PRs as part of the L1 implementer's normal "wait, this signature needs a `pub`" loop.

The packet is locked-ready: an implementer can start commit 1 tomorrow against `surface_nonce.rs:445-471` without making architectural decisions the packet should have made.
