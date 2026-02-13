# Internal Changelog

## Sprint 18 - Focus and Meeting Intelligence (2026-02-13)

### I214 - Focus curation

- Updated focus view model to compute:
  - P1-only "Other Priorities"
  - max 5 visible rows
  - conditional "View All Actions" visibility
  - total pending action count for link context
- Updated Focus page rendering to consume capped/filtered view-model fields.
- Added focused frontend tests for filtering, capping, and link visibility behavior.

### I188 - Agenda-anchored prep enrichment

- Added calendar agenda extraction from `calendarNotes` (supports agenda sections, bullets, and numbered items).
- Updated mechanical agenda ordering to prioritize `calendar_note` topics before derived risk/action/question/win topics.
- Expanded prep enrichment prompt context to include calendar notes and explicit agenda anchoring instructions.
- Added UI source mapping for `calendar_note` items in meeting agenda badges.
- Added Rust tests for calendar agenda extraction and calendar-first agenda ordering.

### I179 - Verification-only

- No ranking algorithm changes made in Sprint 18.
- Focus prioritization regression tests passed.

### Unplanned - Isolated Focus refresh

- Added backend command `refresh_focus` to re-run briefing focus/narrative enrichment without executing the full /today pipeline.
- Added executor path `execute_focus_refresh()` with guardrails (rejects while /today is running; requires existing `_today/data/schedule.json`).
- Registered command in Tauri invoke handler.
- Updated Focus page to:
  - expose a manual `Refresh Focus` button,
  - auto-refresh when `workflow-completed` is emitted,
  - auto-refresh when `operation-delivered` emits `briefing`.

### Validation

- `pnpm test -- src/pages/focusViewModel.test.ts`
- `pnpm build`
- `cargo test --manifest-path src-tauri/Cargo.toml parse_agenda_enrichment`
- `cargo test --manifest-path src-tauri/Cargo.toml generate_mechanical_agenda`
- `cargo test --manifest-path src-tauri/Cargo.toml extract_calendar_agenda_items`
- `cargo test --manifest-path src-tauri/Cargo.toml focus_prioritization`
