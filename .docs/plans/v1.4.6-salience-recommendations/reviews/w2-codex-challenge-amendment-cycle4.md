# W2 L0 Codex challenge — amendment cycle 4

Verdict: REQUEST_CHANGES

Blocking gaps:

- `selected_channel` was still internally contradictory and could be read as delivery authority. The packet said no selected-channel semantics while also storing/testing `selected_channel` with an ambiguous `surface` value.
- The broader wave doc still allowed debug/eval consumers to inspect `triggers_log` directly, while another row said W3+ owns debug/user-facing inspection paths.

Required fold:

- Rename/define the trigger field as a non-delivery internal audit/handoff disposition, with an explicit mapping into W2-A `surface_class`.
- Remove ambiguous `surface` channel value.
- Reword the `triggers_log` consumer rule so W2 writes safe audit state; product/debug inspection goes through W3+ ability projection/block. W5/test-only DB assertions remain separate proof evidence.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
