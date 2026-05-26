# W2 L0 codex challenge — cycle 3

Verdict: BLOCK

Finding:

- Cycle-2 salience ownership closure was consistent in the W2 packet but stale in the wave-plan anchor. The amended packet made W2-A the sole durable recompute owner and required W2-B to copy W2-A returned IDs, but `.docs/plans/v1.4.6-waves.md` still said W2-B's signal is "consumed by W1-B salience re-compute path" and that the scan "invokes salience re-compute." That could reintroduce split ownership and divergent `salience_evaluation_id`s.

Required fix:

- Amend the W2 wave section to say W2-B calls W2-A `evaluate_surfacing_for_claim`; only W2-A calls `recompute_salience_for_claim`; W2-B copies returned salience and surfacing decision IDs into `triggers_log`.
