# I199 — Archived Account Recovery UX — Restore + Relink

**Status:** Open (Parking Lot)
**Priority:** P2
**Version:** Unscheduled
**Area:** Entity

## Summary

When a user archives an account (e.g., a churned customer), the account and all its associated data is flagged as archived in the DB. If that account later becomes active again (win-back, re-engagement, acquisition), the user needs a way to restore it and relink any meetings or people that were associated with it.

The archive system (I176, Sprint 13) only sets a DB flag — it doesn't touch the filesystem. Recovery should be straightforward in principle but there is currently no UI for it beyond the archived accounts tab.

## Acceptance Criteria

### Already Implemented
- [x] Unarchive button on archived account detail page (AccountHero, visible when `archived === true`)
- [x] Archived banner messaging ("This account is archived and hidden from active views")
- [x] Archived accounts tab on AccountsPage with `get_archived_accounts` query
- [x] Backend `archive_account(id, archived: bool)` toggle command — frontend uses this for unarchive
- [x] Backend `restore_account(account_id, restore_children: bool)` command with child cascade
- [x] DB layer: `restore_account()` unarchives with optional child restoration

### Remaining Work
- [ ] Wire frontend to use `restore_account` (with `restore_children`) instead of generic `archive_account` toggle — currently the specialized command is dead code
- [ ] Add restore dialog for parent accounts: "This account has N archived child accounts. Restore them too?" with Yes/No
- [ ] Fix confirmation dialog title (currently says "Archive Account" when unarchiving)
- [ ] Relink flow: detect meetings/people that were associated with the account pre-archive and offer to relink them (or validate they're still linked)

## Dependencies

- Depends on I176 (entity archive/unarchive, Sprint 13) which established the DB-flag-only archive model.
- Related to I198 (account merge) — both deal with account lifecycle management edge cases.

## Notes / Rationale

The auto-unarchive suggestion (I161, Sprint 21) handles the case where a meeting from an archived account is detected. This issue covers the explicit user-initiated restoration path. Parked because the need arises infrequently and the workaround (manually editing the DB) is available for sophisticated users.
