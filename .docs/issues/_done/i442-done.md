# I442 — stakeholder_roles Wired

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Backend / Frontend

## Summary

The `stakeholder_roles` field is defined in all 9 preset JSONs, typed in TypeScript, and validated — but consumed by nothing. The relationship type dropdown on the person-to-account entity linker uses a hardcoded list of role labels that reflects CS vocabulary regardless of the active preset. This issue replaces the hardcoded list with the active preset's `stakeholder_roles` array, making the dropdown role-aware. Existing relationship types stored in the DB are preserved and displayed as-is — the change is backward compatible.

## Acceptance Criteria

1. When linking a person to an account (the entity linker component), the relationship type dropdown shows options from the active preset's `stakeholder_roles` array rather than a hardcoded list. Verify: with Customer Success preset, the options include "Champion," "Economic Buyer," "Technical Evaluator" etc. With Sales preset, the options reflect sales relationship vocabulary.
2. Previously linked relationships that used hardcoded role labels are not broken — their existing `relationship_type` string is displayed as-is even if it does not match the current preset's vocabulary.
3. `SELECT DISTINCT relationship_type FROM entity_people` — existing relationship types are preserved in the DB. New links use the preset-sourced labels.

## Dependencies

- I441 (useActivePreset cache) — the dropdown needs the active preset from the shared context. Build I441 first.
- No backend changes required beyond reading `stakeholder_roles` from the preset. The `relationship_type` column already exists in `entity_people`.

## Notes / Rationale

The `stakeholder_roles` array is already defined in each preset JSON and mapped into the TypeScript preset type. The only change required is in the frontend component that renders the relationship type dropdown — replace the hardcoded options array with `activePreset.stakeholder_roles` (or `activePreset.stakeholder_roles.map(r => r.label)` depending on the preset's field structure).

Backward compatibility is automatic: the `relationship_type` column in `entity_people` is a plain string. Existing values continue to display as-is. When a preset switches and a previously stored role label does not exist in the new preset's vocabulary, it is shown as a freeform value (the same way custom tags work). No migration, no data loss.

The entity linker component is shared — the same dropdown is used both when first linking a person to an account and when editing an existing link. Both call sites benefit automatically once the dropdown source changes.
