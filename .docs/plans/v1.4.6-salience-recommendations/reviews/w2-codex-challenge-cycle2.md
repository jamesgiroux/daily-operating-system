# W2 L0 codex challenge — cycle 2

Verdict: BLOCK

Findings:

- Trigger → salience → surfacing still had split ownership that could produce divergent durable salience references. W2-A required every mutation-path surfacing evaluation to call `recompute_salience_for_claim`, while W2-B also recomputed salience before handing candidates to W2-A. Each recompute creates a new stored `evaluation_id`, so `triggers_log.salience_evaluation_ids_json` could fail to match `surfacing_decisions.salience_evaluation_id`.

Required fix:

- Make one component own recompute. Cycle 3 assigns durable recompute ownership to W2-A. W2-B calls W2-A for candidates and copies W2-A's returned salience evaluation IDs and surfacing decision IDs into `triggers_log`. Add retry coverage proving trigger-log salience IDs exactly match the surfacing decisions they caused.
