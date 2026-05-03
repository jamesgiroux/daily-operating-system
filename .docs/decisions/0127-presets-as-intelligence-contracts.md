# ADR-0101 — Presets as Intelligence Contracts

**Status:** Accepted
**Date:** 2026-04-24
**Authors:** James Giroux, Claude
**Issues:** DOS-177, DOS-178, DOS-179, DOS-180, DOS-176

---

## Context

DailyOS supports multiple professional roles (Customer Success, affiliates/partnerships, product marketing, general). Each role has different vocabulary, priorities, and interpretation of the same raw signals. Early implementations hardcoded CS-specific terminology ("Champion Health", "Financial Proximity") throughout the codebase, making it impossible to serve other roles without forking logic.

The v1.2.x series introduced role presets as structured JSON configs. The question became: what scope should a preset cover? UI vocabulary only? Or the full intelligence pipeline?

## Decision

Presets are intelligence contracts, not UI themes. A preset shapes every layer of the stack where role-specific interpretation applies:

1. **Schema** (DOS-179) — renamed `champion_health` → `key_advocate_health` and `agreement_outlook` → `agreement_outlook` in the DB to use role-neutral field names. The preset's `keyAdvocateLabel` and `closeConcept` fields carry the role-specific vocabulary.

2. **Schema renames** (DOS-179) — frontend `championHealth` → `keyAdvocateHealth` on `RelationshipDimensions`. The TS type is role-neutral; the display label comes from the preset.

3. **Preset threading** (DOS-178) — health scoring, intel prompts, and meeting prep all receive the active preset and apply its `dimensionWeights`, `systemRole`, `closeConcept`, and `keyAdvocateLabel` when computing scores and writing AI prompts.

4. **Signal and email classification** (DOS-176) — `state::build_merged_signal_config` merges base signal rules with the preset's `signalKeywords` and `emailSignalTypes`, so what counts as a high-weight signal differs by role.

5. **Feature flag + display labels** (DOS-177) — the `role_presets_enabled` flag gates the preset selection UI. Health dimension labels (Meeting Cadence, Champion Health, etc.) resolve from `preset.intelligence.dimensionLabels` at render time, falling back to hardcoded ADR-0083 vocabulary.

## Consequences

**Positive**

- The affiliates-partnerships use case works out of the box: a user on that preset sees "Campaign Cadence", "Partner Lead Health", and "Partnership Momentum" instead of CS vocabulary — without any code change.
- Intelligence quality improves per-role: weights, prompt framing, and keyword sensitivity are tuned to each role's actual priorities.
- Adding a new role requires one JSON file plus wiring; no scattered conditionals.

**Negative**

- Every new preset must define all 6 `dimensionLabels`, all 6 `dimensionWeights` (summing to 1.0), a `systemRole`, a `closeConcept`, and a `keyAdvocateLabel`. The validator in `loader::validate_preset` enforces this, but it is a non-trivial authoring burden.
- Any new intelligence surface (new prompt section, new scoring dimension) must be added to the `PresetIntelligenceConfig` struct and to every existing preset JSON before shipping.
- We accept both tradeoffs: correctness across roles outweighs the maintenance cost.

## Implementation reference

| Layer | Issue | What shipped |
|-------|-------|--------------|
| 1 — Schema | DOS-179 | DB column renames + serde alias for backward compat |
| 2 — Struct | DOS-180 | `PresetIntelligenceConfig` Rust struct |
| 3 — Threading | DOS-178 | Preset passed into health scoring + intel + prep |
| 4 — Signal config | DOS-176 | `build_merged_signal_config` merges preset keywords |
| 5 — Flag + labels | DOS-177 | `get_feature_flags` reads config; dimension labels from preset |
