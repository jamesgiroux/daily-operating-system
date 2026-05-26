# W2 L0 Codex challenge — amendment cycle 2

Verdict: REQUEST_CHANGES

Blocking gap:

- Privacy test coverage was still not exhaustive for `triggers_log`. The packet defined more persisted string/JSON fields than the test enumerated, including `run_id`, `policy_version`, `trigger_kind`, `trigger_class`, `source_signal_id`, `entity_type`, `entity_id`, `selected_channel`, `candidate_claim_ids_json`, `salience_evaluation_ids_json`, `surfacing_decision_ids_json`, `status`, `result_kind`, and `downstream_policy`.

Required fold:

- Make the W2 test requirement exhaustive against every current and future `triggers_log` string/JSON column through an explicit matrix or schema-driven assertion.
- Include emitted signal payload string leaves in the same invariant.
- Classify each field as allowlisted enum/code, opaque ID/ref, JSON ID array, or keyed signature/hash.
- Exclude raw path, claim text, prompt/provider output, user-authored free text, raw evidence source string, and raw error text.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
- `.docs/plans/v1.4.6-waves.html`
