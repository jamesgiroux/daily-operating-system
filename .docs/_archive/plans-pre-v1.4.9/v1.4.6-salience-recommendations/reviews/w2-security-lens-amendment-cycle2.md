# W2 L0 security lens review — amendment cycle 2

Verdict: REQUEST_CHANGES

Blocking gap:

- The W2-B privacy regression still did not cover every persisted `triggers_log` JSON/string field it claimed to cover. The table included fields such as `candidate_claim_ids_json`, `salience_evaluation_ids_json`, `surfacing_decision_ids_json`, `downstream_policy`, `source_signal_id`, and closed enum/string fields, but the test fold only enumerated a subset.

Required fold:

- Add an explicit persisted-field privacy matrix for `triggers_log`.
- Classify every string/JSON field as generated ID, closed enum/code, allowlisted token, opaque ref, keyed signature/hash, or JSON ID array.
- Update the regression test requirement to assert raw-path/prompt/provider/raw-evidence/free-text/error sentinels are absent from every matrix field plus emitted signal payloads.
- Explicitly classify and test `downstream_policy` as closed/allowlisted.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
- `.docs/plans/v1.4.6-waves.html`
