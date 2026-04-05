# I391 — People Network Intelligence — Two-Layer intelligence.json + Network Section + Person→Person Signal Propagation + Inferred Edge Extraction

**Status:** Open (0.13.5)
**Priority:** P1
**Version:** 0.13.5
**Area:** Backend / Intelligence

## Summary

Person intelligence gains a second layer: network signals derived from the person's relationship graph (I390). The AI enrichment prompt for a person includes their own signals (Layer 1 — unchanged) plus the state of their relationship edges (with decayed confidence) and brief summaries of connected persons' recent signals. The enrichment produces a `network` section in `intelligence.json` containing: network health, key relationships with confidence and signal summaries, network risks and opportunities, influence radius, and a cluster summary paragraph.

Additionally, person→person signal propagation is established: when a person emits a signal above a confidence threshold (0.65), derived signals are generated for relationship-graph neighbors, weighted by relationship confidence and an edge-type-sensitive multiplier (0.4 base, modified per type). Propagation is limited to 1 hop per cycle, and propagation loops are prevented by the existing source-tag mechanism (`propagation:network`).

The enrichment pipeline also extracts inferred relationship edges when it detects relationship patterns in meeting/email context, writing them to `person_relationships` with `source = 'inferred'` and low initial confidence (~0.4).

## Acceptance Criteria

1. Person enrichment prompt includes: the person's own signals (Layer 1), their relationship edges from `person_relationships` (with decayed confidence), and brief signal summaries for each connected person. Verify by adding a debug log that prints the assembled prompt for one person with ≥2 known relationships — confirm the logged prompt contains real relationship type data and connected person names.

2. Person `intelligence.json` includes a `network` top-level field. A person with ≥2 relationship edges has non-null values in `key_relationships` (array with real entries), `cluster_summary` (non-empty string), and `influence_radius` (integer > 0).

3. A person with zero relationship edges in the DB has `network.health = "unknown"` and empty arrays (`key_relationships: []`, `risks: []`, `opportunities: []`). The `network` field exists but is sparse — not missing entirely.

4. Person→person signal propagation: Emit a high-confidence signal (≥0.65) for Person A who has a `champion` edge to Person B with edge confidence 0.8. Within one `intel_queue` propagation cycle, `signal_events` contains a row for Person B with `source = 'propagation:network'` and `confidence = (original_signal_confidence * 0.8 * 0.4)`. Verify with real data by querying `signal_events`.

5. Edge-type-sensitive propagation: A person has a `blocker` edge (confidence 0.8) to another person. Emit a 0.65 confidence signal for the first person. The derived signal confidence should be `0.65 * 0.8 * 0.4 * 1.2 ≈ 0.250` (0.4 base × 1.2 blocker multiplier = 0.48 effective multiplier). Verify in `signal_events` that the derived signal has confidence ≈ 0.25.

6. No propagation cascade: Person A emits a signal that propagates to Person B (A's neighbor). Person B does NOT generate a propagated signal to Person C (B's neighbor) in the same cycle. Query `signal_events` — confirm no `propagation:network` signal has itself spawned another `propagation:network` signal in the same cycle.

7. Network enrichment is triggered when a person's relationship graph changes. Add a new relationship edge for a person; within one `intel_queue` cycle, that person's `entity_intel.updated_at` timestamp advances. Verify by querying `entity_intel` for the person before and after adding an edge.

8. Inferred edges from enrichment: After enriching a person who attends meetings with another known person, if the enrichment output identifies a relationship pattern, a row appears in `person_relationships` with `source = 'inferred'` and confidence ~0.4. Verify by checking `person_relationships` before and after enrichment for a person with shared meeting attendance.

## Implementation Notes

- **Propagation rule**: Add `rule_person_network` to `signals/rules.rs` following the same pattern as the 7 existing rules. Register in `propagation.rs` via `engine.register("rule_person_network", ...)`.
- **Loop prevention**: Check `signal.source.contains("propagation:network")` — same pattern as hierarchy rules using `"propagation:hierarchy"`. No overlap.
- **Intelligence struct**: Add `network: Option<NetworkIntelligence>` to `IntelligenceJson` in `intelligence/io.rs`, following the `portfolio: Option<PortfolioIntelligence>` pattern.
- **Prompt expansion**: `intelligence/prompts.rs` person enrichment currently includes facts, meetings, email signals, entity connections. Add relationship edges + neighbor signal summaries as a new context block.
- **Edge-type multipliers** (ADR-0088 Table): 1.0x for champion/sponsor/decision_maker, 1.2x for blocker/detractor, 0.7x for peer/ally, skip collaborator below high confidence, context-gated for dependency.
- **Inferred edge extraction**: Parse enrichment response for relationship identifications. Write to `person_relationships` via the same DB functions I390 exposes. Edge extraction is forward-looking only — no bulk historical extraction.

## Dependencies

- Blocked by I390 (person_relationships table must exist first; inferred edges write to it).
- Required by I392 (the "Their Network" chapter on person detail renders from `network` section of `intelligence.json`).
- See ADR-0088 decisions 2, 3, and 4.

## Notes / Rationale

Layer 2 (network signals) is what makes person intelligence qualitatively different from a CRM record. A CRM tells you what someone's title is and when you last talked. Network intelligence tells you: "Your champion (Jim Park) changed roles and his influence path to the CFO is unclear. Sarah Chen's relationship with the CFO creates an alternative path — surface this before Thursday's renewal call." That kind of intelligence requires knowing how people relate to each other, not just their individual attributes.
