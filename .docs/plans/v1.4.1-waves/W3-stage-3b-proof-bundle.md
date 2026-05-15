# Wave W3 Stage-3b Proof Bundle (DOS-6 trust scoring shadow-mode rollout)

**Wave:** v1.4.1 W3 — stage 3b (shadow bake closure)
**Status:** PARTIAL — instrumentation landed, data sufficiency criteria 2/6 met
**Date:** 2026-05-15

This bundle records what shipped under PR #260 against the wave-plan stage-3b
closure criteria at `.docs/plans/v1.4.1-waves.md` §437-442. Five of six data
sufficiency criteria are unmet; W6 hard precondition is therefore not
satisfied. Residual work is filed as Linear tickets enumerated at the end of
this document.

---

## What shipped (PR #260, commit `ff00d475`)

| Surface | Detail |
|---|---|
| Feature flag | `FEATURE_TRUST_COMPILER_SHADOW` (default-on in dev) |
| Migration | v155 — adds `intelligence_claims.shadow_trust_score`, `shadow_trust_computed_at`, `shadow_trust_version` columns |
| Migration | v157 — shadow trust reconciliation (commit `012d4ca2`) |
| Call site (1 of 5) | `src-tauri/src/services/meetings.rs:3732` `run_trust_compiler_shadow_if_enabled()`, invoked at `meetings.rs:3800` |
| L2 outcome | 9-cycle codex-adversarial + code-reviewer + domain-reviewer APPROVE (unanimous) |
| CI annotations | `dos7-allowed` annotations on shadow-column UPDATE/INSERT (cycle-12, commit `7f6a05ca`) |
| Out of band defer | divergence-monitor cron, alert canary, v157 DB `CHECK` constraint — path-α |

---

## Stage-3b data sufficiency criteria (from `v1.4.1-waves.md` §437-442)

Measured against dev DB (`~/.dailyos/dailyos.db`) at 2026-05-15.

### Criterion 1 — Volume (≥1,000 shadow scoring events) — **PASS**

- `signal_events` total: **169,198**
- `signal_events` last 7 days: **22,724**
- Shadow-scored claims: **4,490**
- Sample size is sufficient for tuning per the criterion's "before tuning"
  intent.

### Criterion 2 — Surface coverage (5 surfaces) — **FAIL**

Wave plan names five required surfaces: briefing, meeting detail, entity
detail, actions, email.

| Surface | Wired? | Evidence |
|---|---|---|
| Briefing | ✅ | `services/meetings.rs:3800` — only call site to `run_trust_compiler_shadow_if_enabled` in the entire repo |
| Meeting detail | ❌ | No call site |
| Entity detail | ❌ | No call site |
| Actions | ❌ | No call site |
| Email | ❌ | No call site |

The six distinct `claim_type` values that show shadow scores (`entity_win`,
`entity_risk`, `value_delivered`, `entity_summary`, `entity_current_state`,
`company_context`) fan out from the single briefing call site as it iterates
over meeting-related claims; they are not independent surfaces.

### Criterion 3 — Trust-band distribution (3 bands ≥50 each) — **FAIL**

Output of `scripts/trust_distribution.sql` against
`intelligence_claims WHERE shadow_trust_version = 1401003`:

| Band | Count | Avg | Min | Max |
|---|---|---|---|---|
| `likely_current` (≥0.75) | 4,489 | 0.8556 | 0.7527 | 1.0000 |
| `use_with_caution` (≥0.50 & <0.75) | 1 | 0.7223 | 0.7223 | 0.7223 |
| `needs_verification` (<0.50) | 0 | — | — | — |

99.98% of scored claims land in `likely_current`. The wave plan calls a band
that never fires "evidence of a broken threshold, not absence of risk" — and
two bands have ≤1 entries each. This is a threshold problem, not a data-volume
problem. Tuning is required before this criterion can pass; tuning is the
W3-D-folded-into-W3-C work that did not land in PR #260.

### Criterion 4 — Alert-path canary — **FAIL**

PR #260 deferred the divergence monitor and alert canary to path-α. No
synthetic divergence event has been injected, and the alert path has therefore
never been exercised.

### Criterion 5 — Threshold deltas captured in an ADR — **FAIL**

No ADR-0132 (or equivalent) documenting before/after thresholds for
feedback-delta tuning or subject-fit-floor tuning exists in `.docs/decisions/`.
This ADR cannot be written until tuning runs (criterion 3 blocks it).

### Criterion 6 — No regression (Suite E bundles 1-13) — **N/A (in shadow mode)**

Shadow mode does not mutate live `trust_score`; Suite E bundles continue to
exercise live trust scoring unchanged. This criterion is technically vacuously
satisfied for shadow-mode itself but becomes load-bearing at the cutover stage
that is no longer in v1.4.1 scope.

---

## Review Ladder coverage

| Level | Required by wave plan §457-463 | Status |
|---|---|---|
| L0 | Plan approvals per matrix | ✅ Completed pre-PR-260 |
| L2 | Diff approval on W3-C PR | ✅ 9-cycle unanimous APPROVE on PR #260 |
| L3 | Wave adversarial (codex-challenge + Suite P + Suite E) | ❌ Not run on the full W3 wave (W3-A + W3-B + W3-C integrated) |
| L4 | `/qa-only` confirms shadow produces non-trivial distributions | ❌ Distribution measured here is trivially skewed; QA pass would fail |
| L5 | Drift check vs v1.4.1 §Trust depth — threshold deltas documented | ❌ Blocked on criterion 5 |
| Retro | Mandatory — first shadow-mode rollout | ❌ Not yet authored |

---

## Conclusion

PR #260 represents the **instrumentation** half of W3 stage-3b: feature flag,
schema, one call site, L2-clean code. It does **not** represent stage-3b
closure as the wave plan originally defined it. The wave plan's stage-3b is
gated on data sufficiency, and only criterion 1 (volume) and the inapplicable
criterion 6 are met today.

Per `feedback_wire_existing_substrate_not_future_producer`, shipping
instrumentation while declaring full stage-3b closed would be the failure
mode that memory was written to prevent. This bundle does not do that;
it records the partial state honestly.

---

## Closure decision

The four surfaces named in criterion 2 (meeting detail, entity detail,
actions, email) are macOS-app surface call sites. v1.4.2 is intentionally a
WordPress-surface spike. If that spike succeeds, WordPress becomes the
primary surface and wiring those four macOS call sites would be churn
against a likely-deprecated surface layer. Criteria 3, 4, and 5 are
downstream of surface coverage (tuning needs diverse inputs; the ADR
documents the tuning; the canary verifies the tuned alert path) and are
therefore also surface-contingent.

Per James's 2026-05-15 decision, stage-3b closure is **routed to the v1.4.2
spike outcome** rather than completed inline under v1.4.1. The wave protocol
amendment encoding this is at
`.docs/plans/v1.4.1-waves-amendments.md` Amendment 1.

The amendment in summary:

- Stage-3b is recategorized as **instrumentation-complete,
  data-sufficiency-pending**, not failed-or-open.
- W6's hard precondition is relaxed to **instrumentation-complete** so W6
  starts on the partial baseline.
- If the v1.4.2 WordPress spike succeeds: the four macOS-surface
  requirements are **superseded**; new tickets attach to v1.4.2 or a
  successor wave plan.
- If the spike fails or rolls back: the residual tickets enumerated below
  are filed at that decision point and stage-3b returns to OPEN.
- v1.4.1 release notes state that shadow trust scoring is instrumented but
  not tuned; trust-band rendering still operates on live trust scoring.

No Linear tickets are filed at this proof-bundle authoring time.

---

## Residual work — authoring checklist (not filed)

Retained here as a checklist for the spike-outcome decision, **not as
pending Linear tickets**. Filed only at the v1.4.2-outcome decision point
under whichever surface layer ends up canonical.

| # | Criterion | Scope |
|---|---|---|
| 1 | 2 | Surface wiring (4 call sites OR WordPress equivalents) |
| 2 | 3 | feedback-delta tuning against shadow data |
| 3 | 3 | subject-fit-floor tuning against shadow data |
| 4 | 5 | ADR-0132 — threshold deltas + rationale |
| 5 | 4 | Divergence-monitor cron (promote `scripts/trust_distribution.sql`) |
| 6 | 4 | Alert-path canary — synthetic divergence end-to-end |
| 7 | L3 | Wave adversarial: codex-challenge + Suite P + Suite E |
| 8 | L4 | `/qa-only` against tuned non-trivial distribution |
| 9 | L5 | Drift check vs v1.4.1 DoD |
| 10 | retro | Mandatory shadow-mode retro |

Memory `feedback_linear_is_the_audit_trail`: when these become real, they
land as Linear tickets at that point.
