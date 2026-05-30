# W2 L0 Codex challenge — amendment cycle 1

Verdict: REQUEST_CHANGES

Findings:

- HIGH: Quiet/background semantics were inconsistent across the packet and current W2 code shape. The packet made `quiet` a policy state, while current schema/code collapsed quiet into background and introduced unplanned `external` scope.
- HIGH: Trigger source privacy was under-specified. The packet required safe source tokens, but `source_signal_type`, dedupe keys, suppression keys, and current implementation shape could persist caller-provided source text.
- MEDIUM: The packet described W2 as two file-disjoint lanes even though DOS-331 and DOS-334 share `migrations.rs`, `devtools/mod.rs`, and the W2-B -> W2-A handoff.

Required fold:

- Make `quiet` first-class auditable policy state and explicitly forbid W2 `external` surface/channel semantics.
- Require `source_signal_type`, dedupe keys, suppression keys, evidence refs/signatures, reason codes, and error codes to be assembled from canonical/allowlisted tokens, opaque refs, or keyed signatures only.
- Reword W2 as one coordinated PR with two logical lanes, shared registration files, and a trigger -> surfacing handoff.

Folded into:

- `.docs/plans/v1.4.6-salience-recommendations/L0-packet-W2-DOS-331-DOS-334.md`
- `.docs/plans/v1.4.6-waves.md`
- `.docs/plans/v1.4.6-waves.html`
