# W2 L0 security lens review — amendment cycle 1

Verdict: REQUEST_CHANGES

Blocking gap:

- Trigger/audit privacy tests did not cover every W2 storage surface. The packet forbids raw paths, claim text, prompt/provider output, user-authored free text, and raw evidence sources in W2 tables/signals, but the test list only explicitly covered `trigger_refs_json`, `why_this_now_json`, and emitted surfacing signal values.

Required fold:

- Add a W2-B privacy regression that feeds raw-path-looking, prompt-looking, provider-output-looking, raw evidence-source-looking, and free-text-looking values through trigger input/evidence/error paths.
- Assert every persisted `triggers_log` JSON/string field plus emitted signal payload contains only allowlisted IDs, enum/signal codes, opaque refs, keyed signatures, and closed error codes.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
- `.docs/plans/v1.4.6-waves.html`
