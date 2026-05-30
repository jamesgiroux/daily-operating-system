# W2 L0 Codex challenge — amendment cycle 5

Verdict: REQUEST_CHANGES

Blocking gaps:

- Budget overflow semantics were contradictory: the W2 packet said primary over-budget high-salience candidates defer with `BudgetExhausted`, while the broader wave plan still said over-budget candidates land in quiet/background and tests expected `Defer/Quiet`.
- `triggers_log.result_kind = rendered` could reintroduce delivery semantics despite W2 having no consumer-facing delivery authority.
- Active W2 wave text referenced `{primary_factor.rationale}`, but the frozen contract stores `WhyThisNow.primary_factor` as `SalienceFactorKind`; W2 needs to derive the typed reason from the selected `SalienceFactor.rationale` and store safe deterministic text/JSON.

Required fold:

- Align the wave plan with the packet: primary budget exhaustion is `Defer(BudgetExhausted)`; background/quiet are explicit policy states, not generic overflow buckets.
- Rename the trigger result code to a non-delivery code such as `render_decision_recorded`.
- Fix the `why_this_now` template wording to use `factor_label`, typed reason, and trigger summary without `Debug` formatting or raw rationale/source strings.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
