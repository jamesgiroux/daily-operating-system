# W2 L0 architecture / K-in review — cycle 1

Verdict: BLOCK

Findings:

- Budget policy conflicted with ADR-0126 because it was a hard-coded budget per `surface_key`, and the decision row lacked `claim_type` and `sensitivity` for audit.
- Mutation-path surfacing could log preview salience without a persisted `salience_evaluation_id`; W1-B only returns an evaluation id from `recompute_salience_for_claim`.

Reuse checks:

- Recommendation substrate reuse is correct: no parallel `recommendation_claims` table.
- Signal boundary is acceptable: variants are already present and W2 uses `services::signals`.
- Mock data and migration coverage are acknowledged.

Cycle 2 packet changes: key budgets by `actor_kind + local_day + claim_type + sensitivity + surface_class`, add versioned recommendation surfacing policy rows, add `claim_type`/`sensitivity` to decision rows, and require W1-B durable recompute before mutation-path audit writes.
