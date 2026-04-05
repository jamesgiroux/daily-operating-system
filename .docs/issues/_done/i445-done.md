# I445 — prioritization Wired

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Backend / Frontend

## Summary

The `prioritization` object in each preset — containing `primary_signal`, `secondary_signal`, and `urgency_drivers` — is defined, typed, and validated across all 9 presets but consumed by nothing. The AccountsPage sort order is hardcoded and ignores the preset entirely. The weekly forecast risk section ranks attention items without reference to the preset's urgency drivers. This issue wires both: the accounts list default sort reflects the active preset's `primary_signal`, and the weekly forecast attention ranking is boosted by matches to `urgency_drivers`.

## Acceptance Criteria

1. The AccountsPage default sort order is informed by the active preset's `prioritization.primary_signal`. With Customer Success preset (`primary_signal: "renewal_proximity"`): accounts sorted by nearest renewal date first within each health tier. With Sales preset (`primary_signal: "deal_stage"`): accounts sorted by deal stage progression. With Leadership preset: sorted by ARR descending.
2. The sort is a soft suggestion, not a hard override — user-selected sorts (alphabetical, recently viewed) take precedence. The preset-driven sort is the "default" that applies when no explicit sort is selected.
3. The weekly forecast's risk section uses `prioritization.urgency_drivers` to rank attention items. If `urgency_drivers` includes "renewal_at_risk" and "no_recent_meeting," accounts matching these conditions surface higher in the attention section.
4. `cargo test` passes. No regressions on existing sort behavior.

## Dependencies

- AccountsPage sort: I441 (useActivePreset cache) for the frontend sort to read the active preset.
- Weekly forecast ranking: depends on whether the ranking logic is frontend or backend. If backend (`workflow/today.rs` or `queries/proactive.rs`), the Rust side reads the active preset from `workspace_config` at query time.
- No hard blockers beyond I441 for the frontend sort change.

## Notes / Rationale

**primary_signal → sort criteria mapping:**
- `renewal_proximity` → sort by `metadata.contract_end` or the `renewal_date` metadata field, ascending (soonest first), nulls last
- `deal_stage` → sort by deal stage progression metadata; define a stage order (Prospecting < Discovery < Proposal < Negotiation < Closed Won/Lost) and sort by index ascending
- `arr_descending` (Leadership) → sort by `arr` metadata field descending, nulls last
- `launch_proximity` (Marketing, Product) → sort by next launch or milestone date ascending
- `project_deadline` (Consulting, Agency) → sort by active project end date ascending
- Default (The Desk, Partnerships, and any unrecognised signal) → sort alphabetically by account name as a safe neutral fallback

The sort is a default only. A `sort_preference` persisted in local state (e.g., localStorage or a React ref) takes priority. When the user explicitly selects a sort (alphabetical, recently viewed, etc.) that selection is sticky for the session. Switching presets while a user-selected sort is active does not override it — the preset sort only applies when no explicit selection has been made.

**urgency_drivers → attention ranking:**
The `urgency_drivers` array contains signal type strings (e.g., "renewal_at_risk", "no_recent_meeting", "health_declined"). These should map to signal event types in `signal_events`. When building the weekly forecast's attention ranking, accounts that have a matching open signal receive a ranking boost. The boost is additive on top of existing signal confidence scoring — do not replace the existing scoring, augment it.

If the weekly forecast attention ranking is assembled in `src-tauri/src/queries/proactive.rs` or `src-tauri/src/workflow/today.rs`, the change lives there. Read the active preset's `urgency_drivers` from `workspace_config`, map each to the signal event type string, and boost accounts that have open events of those types.
