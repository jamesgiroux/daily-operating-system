# DOS-307 — System-owned claim verification + low-burden feedback (substrate scope)

**Status:** substrate principle satisfied at v1.4.0 wave tip (`658dbd07`). Product-UX layer items deferred to v1.4.x.
**Acceptance walk last refreshed:** 2026-05-07.

## Contract

DailyOS owns claim verification by default. The system verifies claims automatically where sources allow; suppresses or downgrades claims it cannot justify; repairs claims asynchronously when evidence is weak or contradicted; only asks the user when the decision is high-impact, ambiguous, or preference/intent-dependent. Feedback is exception/escalation, not the primary verification loop.

This document captures the **substrate** layer that implements the principle. The **product-UX** layer (review queue with daily budget, Activity Log triage classification, lint findings separation, copy review) is filed as v1.4.x follow-ups; rationale below.

## Substrate scope — verified satisfied

### Claims land in system-owned states automatically

Every claim entering `intelligence_claims` is automatically classified by the substrate without user input:

| State | Substrate signal | Implementation |
|---|---|---|
| `verified_current` (`likely_current` band) | trust_score ≥ 0.70 | DOS-5 trust factors |
| `likely_current` (band) | trust_score ≥ 0.70 with caveats | DOS-5 + ConfidenceCaveat |
| `needs_corroboration` (`use_with_caution` band) | corr factor low, single-source | DOS-5 corr factor noisy-OR over `claim_corroborations.strength` |
| `stale` | `source_asof` past freshness window | DOS-10 + freshness_weight floor |
| `contradicted` | `TrustGateKind::AuthoritativeContradiction` | abilities/trust/mod.rs:108-113 |
| `wrong_subject_suspected` | subject-fit gate fails at commit or render | abilities/provenance/ownership.rs::validate_serialized_subject_ownership |
| `source_unavailable` | `SourceLifecycle::Withdrawn` / `Restricted` + TrustGateKind::SourceWithdrawn | trust gates + lifecycle field |
| `suppressed_by_policy` | sensitivity gate (Confidential/UserOnly per surface) | services/sensitivity::render_policy_for_surface (DOS-412) |
| `needs_user_decision` | tombstone resurrection attempt with no fresh independent evidence; explicit `corrected` feedback path | services/claims.rs::commit_claim pre-gate |

Only `needs_user_decision` produces an explicit user task. The other states are system-owned outcomes — user does not see a "please verify" prompt for any of them.

### Evidence-first repair before asking the user — substrate hooks present

`commit_claim` and the Trust Compiler recompute pipeline run automated repair work before any user interaction:
- **Find corroborating evidence** — `CorroborationStrengthened` signal (DOS-5 amendment) triggers recompute; new corroborations boost the corr factor automatically.
- **Check source lifecycle and permissions** — `TrustGateKind::SourceWithdrawn` gates (W4-A).
- **Compare source_asof against claim temporal scope** — `freshness_weight` factor downweights stale claims under the temporal_scope policy from ADR-0125.
- **Test subject fit** — pre-commit + pre-render gate via `validate_serialized_subject_ownership` (W6 boundary cycle-15 APPROVE).
- **Check tombstones / corrections** — `commit_claim` pre-gate rejects resurrection without fresh independent evidence (DOS-303 §Tombstones).
- **Canonicalize duplicates** — DOS-280 canonicalization collapses near-duplicate claims at write time.
- **Downgrade / suppress / qualify if evidence insufficient** — band rendering rules (DOS-320): `likely_current` in main body, `use_with_caution` in collapsed Background section, `needs_verification` hidden behind "Show all evidence" toggle.

### Confidence-based UI exposure (substrate side)

Trust band → render policy mapping is enforced at the substrate boundary, not at each call site:
- `likely_current` → main body (no per-claim affordance needed; default state)
- `use_with_caution` → Background section, collapsed by default
- `needs_verification` → hidden by default; "Show all evidence" per-session toggle
- `unscored` → trust compiler hasn't run yet; rendered with the open-ring marker per DOS-320 redesign

The trust-band UI redesign at commit `30a58a93` shipped the open-ring marker on visible bands (`use_with_caution`, `needs_verification`, `unscored`), with `likely_current` rendering nothing — i.e. high-trust claims get NO inline feedback affordance, exactly the "feedback behind details/menu" rule for high-trust.

### UI copy avoids guilt-inducing language at substrate level

The trust-band tooltips (DOS-320 redesign) describe trust state as system property, not user task:
- "Use with caution: this evidence has caveats — it may be stale, lightly sourced, or carry an unverified timestamp."
- "Needs verification: confidence is low or a trust gate fired. Confirm against a primary source before acting on it."
- "Unscored: the trust compiler has not scored this evidence yet."

None of these tell the user "you need to verify." The framing is the system describing its own confidence, not assigning chores. This satisfies the substrate-level copy criterion.

## Acceptance criteria — verification (substrate scope)

| Criterion | Status |
|---|---|
| DOS-294 feedback semantics state feedback is exception/escalation | satisfied — DOS-294 typed feedback shipped; matrix in DOS-303 doc |
| DOS-303 trust/feedback contract includes system-owned claim states + `needs_user_decision` | satisfied — DOS-303 contract doc lists all 9 states |
| Subject-fit / tombstone / duplicate / contradiction repair runs before user review at the **substrate level** | satisfied — pre-commit + pre-render gates documented above |
| UI copy avoids guilt-inducing language for system-owned uncertainty | satisfied at trust-band tooltip layer (DOS-320 redesign) |

## Deferred to v1.4.x (product-UX layer)

The following criteria from DOS-307 are real product-UX work that was not in the v1.4.0 substrate scope. Filed as separate v1.4.x tickets so the deferral is tracked.

| Criterion | v1.4.x ticket | Why deferred |
|---|---|---|
| User review queue with daily budget + prioritization | (filed) | New product feature: the queue itself, budget config, prioritization heuristic. Substrate makes this possible (typed states + signals); the surface is its own feature. |
| Activity Log summarizes automatic verification/repair work | (filed) | Activity Log triage classification UI — separating `fixed automatically` / `watching` / `needs evidence` / `needs user decision`. UI work over an existing Activity Log surface. |
| Lint mode separates automatic findings from user-action-required findings | (filed) | Same as above for the Lint surface. |
| Golden Daily Loop bundle includes hundred-claims-asked-only-prioritized-subset case | (filed) | Bundle data + assertion on the prioritization output. Depends on the queue feature. |
| Low-impact unresolved claims suppressed / qualified rather than user chores at the surface layer (beyond band rendering) | (filed) | Surface-specific render rules per claim_type; substrate provides the band, surface decides whether to even render. |

## References

- ADR-0123 — Typed feedback semantics
- ADR-0125 — Temporal scope + sensitivity + claim type registry  
- ADR-0126 — Retrieval invariants (compression vs distortion)
- DOS-294 (typed feedback), DOS-5 (Trust Compiler), DOS-303 (trust/feedback/tombstone contract), DOS-320 (trust-band rendering), DOS-411 (Tauri claim-backed lifecycle), DOS-412 (ADR-0108 sensitivity rendering)
- W4 / W5 / W6 proof bundles
