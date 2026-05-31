# L0 codex-consult — W1 Substrate Gaps, Cycle 1

**Reviewer:** codex-consult (architecture + correctness spot-check; scoped per request — CSO already covered surface-level findings)
**Date:** 2026-05-20
**Scope:** Verify (1) cited file paths + line numbers, (2) ADR cite accuracy for §5.6/§5.7/§5.9, (3) DOS-701 substrate shape vs §5.6.

## VERDICT: APPROVE WITH FINDINGS (2 minor sketch drifts; no blockers)

Cited symbols + line numbers verified. ADR citations accurate. DOS-701 substrate shape matches §5.6 modulo two call-signature drifts in §5.6/§5.7 pseudocode that need a one-line correction before W1 implementation dispatch.

## Findings

### F1 — minor — §5.7 DOS-8 sketch — `auth::can_surface_for` signature drift
**Claim (line 601, line 357):** sketch calls `services::claim_receipt::auth::can_surface_for(actor, surface, claim_id)`.
**Evidence:** Actual shipped signature at `src-tauri/src/services/claim_receipt/auth.rs:19-24` is `pub async fn can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>`. Two drifts: (a) `state: &AppState` is a required first arg, missing from the sketch; (b) types are `RenderActor` / `RenderSurface` (from `abilities_runtime::sensitivity`), not the packet-introduced `Actor` / `SurfaceContext`.
**Fix:** Update §5.7 sketch step 2 + §5.3 fixture note (line 357 comment) to show `&AppState` arg and `RenderActor`/`RenderSurface` types. AC-8.3 authorization matrix should explicitly call out the Actor→RenderActor adapter as a sub-task (mapping `Actor::{User,Agent,SurfaceClient,System}` ↔ `RenderActor`).

### F2 — minor — §5.6 DOS-339 sketch — `render_receipt_for` signature drift
**Claim (line 605):** sketch comment "fetch updated receipt via services::claim_receipt::render::render_receipt_for".
**Evidence:** Shipped signature at `claim_receipt/render.rs:18-22` is `pub async fn render_receipt_for(state: &AppState, target: ReceiptTarget, surface: SurfaceContext) -> Result<ClaimReceipt, RenderError>` — requires `&AppState` as first arg. The wrapper service for DOS-8 needs to thread state through. Not a substrate gap; just a sketch precision issue.
**Fix:** Add `&AppState` to step-6 sketch + AC-339.1 hook signature (`useClaimReceiptSubscription` is TS-side; the Tauri command behind it needs the state param wired).

### F3 — needs-confirmation — §5.6/§5.7 — `Proposal` + `WorkItem` receipt targets not yet renderable
**Claim:** §5.6 says shipped `render_receipt_for` covers `ReceiptTarget::{Claim, Proposal, WorkItem}` (per the L0 Addendum DTO). §5.7 sketch step 6 fetches receipt after `Proposal` feedback as well as `Claim`.
**Evidence:** `claim_receipt/render.rs:23-32` only handles the `Claim` arm; `Proposal { .. } | WorkItem { .. } => return Err(RenderError::TargetNotFound)`. DTO ships all three target variants but render only implements Claim.
**Fix:** Either (a) clarify in §5.6 that Proposal/WorkItem rendering is DOS-701-deferred and §5.7 step 6 falls back to a typed "no receipt yet, target-only" response for those variants; or (b) add a W1 sub-task to fill in the Proposal/WorkItem render arms. CSO surface allowlist already constrains where Proposal receipts render, so (a) is the lighter path; flag explicitly so W4 (Actions/Work) doesn't assume proposal receipts work end-to-end.

### F4 — info — Header (line 23) LOC drift vs file totals
**Claim (line 23):** "auth.rs (319 LOC)" + Changelog "319+290+175 LOC". 
**Evidence:** `wc -l` confirms auth.rs=319, contracts.rs=175, render.rs=290 (matches the changelog's `319+290+175` tuple in the order auth+render+contracts). Header line 23 reads naturally as auth=319; consistent. **No drift — note for the record only.**

### F5 — info — ADR cite spot-check, all accurate
- **ADR-0102 §3** (Read-ability non-mutation) — verified at `.docs/decisions/0102-abilities-as-runtime-contract.md:80-89`: "Read | No service mutation anywhere in the call graph. May emit ephemeral logs and telemetry but never writes domain state or emits propagating signals." Matches §5.10 cite for `get_daily_briefing` as Read ability.
- **ADR-0123 §1** (9-variant feedback enum) — verified at `.docs/decisions/0123-typed-claim-feedback-semantics.md:24-89`: nine variants enumerated (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`). Matches §5.7 cite.
- **ADR-0125 §2** (sensitivity default) — verified at `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:57-86`: `DEFAULT 'internal'` SQL + "Default `Internal` is the conservative choice". Matches §5.4 / §5.9 implicit cite.

## Cited-line verification

- `services/claims.rs:6700` — `pub fn record_claim_feedback(` confirmed. ✓
- `services/claims.rs:8906` — `pub fn reconcile_contradiction(` confirmed. ✓
- 9-variant test at `services/claims.rs:13283` (`record_claim_feedback_persists_a_row_per_action_for_each_of_9_variants`) — confirmed. ✓ (packet does not cite but it's load-bearing for AC-8.1).

## Recommendation

APPROVE pending F1+F2 sketch corrections (one-line fixes; not a re-review cycle). F3 deserves an explicit §5.6 sentence noting Proposal/WorkItem render arms are DOS-701-deferred so W4 doesn't budget against them. F4/F5 are info-only.
