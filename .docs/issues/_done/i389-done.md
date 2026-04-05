# I389 — Entity-Mode-Aware Surface Ordering — Nav/Primary Surface Adapts to Preset's entityModeDefault

**Status:** Open (0.13.4)
**Priority:** P2
**Version:** 0.13.4
**Area:** Frontend / UX

## Summary

Each role preset (v0.11.0) has an `entityModeDefault` field that declares which entity type is primary for that role — `account`, `project`, or `both`. Currently, the app navigation and primary surfaces are the same regardless of the active preset: the nav always prioritizes the same order. This issue makes the navigation and primary surface emphasis adapt to the preset's `entityModeDefault`: a user with a project-mode preset (Marketing, Product) sees the Projects surface more prominently; an account-mode user (CS, Sales) sees the Accounts surface first.

## Acceptance Criteria

Not yet specified. Will be detailed in the v0.13.4 version brief. At minimum:

- A user with `entityModeDefault: "project"` sees Projects before Accounts in the navigation.
- A user with `entityModeDefault: "account"` sees Accounts before Projects in the navigation.
- A user with `entityModeDefault: "both"` sees the current ordering (no change).
- Switching presets in Settings updates the navigation order without requiring a restart.

## Dependencies

- Related to I388 (project hierarchy intelligence) — project-mode users should see the portfolio surface for projects, which requires I388.
- Depends on the role preset system (v0.11.0).
- See ADR-0087 decision 5 and ADR-0079.

## Notes / Rationale

Navigation order is a subtle but meaningful signal about what the app thinks is most important. A Marketing Manager using DailyOS shouldn't have to navigate past "Accounts" every time they want to get to their project portfolio. The `entityModeDefault` field was added to presets in v0.11.0 precisely to support this kind of mode-aware adaptation — this issue implements that adaptation in the navigation layer.
