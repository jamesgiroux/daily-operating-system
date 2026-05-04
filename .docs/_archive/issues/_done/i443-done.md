# I443 — internal_team_roles Wired

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Backend / Frontend

## Summary

The `internal_team_roles` field is defined in all 9 preset JSONs, typed in TypeScript, and validated — but consumed by nothing. The account team management UI (the flow for adding internal team members to an account) uses a hardcoded role list that reflects CS vocabulary regardless of the active preset. This issue replaces the hardcoded list with the active preset's `internal_team_roles` array. Existing team member role labels stored in the DB are preserved and displayed as-is.

## Acceptance Criteria

1. When adding a team member to an account (the account team management UI), the role dropdown shows options from the active preset's `internal_team_roles`. With Customer Success preset: "CSM," "TAM," "Account Executive" etc. With Partnerships preset: "Partner Manager," "Solutions Engineer" etc.
2. Existing team member role labels are preserved — not retroactively changed when preset switches.

## Dependencies

- I441 (useActivePreset cache) — the dropdown needs the active preset from the shared context. Build I441 first.
- Can be built in parallel with I442 — same pattern, different component and different preset field.

## Notes / Rationale

Same pattern as I442 (stakeholder_roles). The `internal_team_roles` array is already defined per preset JSON and mapped into the TypeScript type. The only change is in the account team management component's role dropdown: replace the hardcoded options with `activePreset.internal_team_roles`.

Backward compatibility: role labels are stored as plain strings in the relevant join table. Existing values display as-is regardless of preset switches. No migration required.

This is a small, bounded frontend change. The account team management flow may be in `AccountDetailEditorial.tsx` or a child component — locate the role dropdown and swap the options source. Verify the preset field name matches what is actually defined in the preset JSON files (may be `internal_team_roles` or `internalTeamRoles` depending on how the JSON is deserialized into TypeScript).
