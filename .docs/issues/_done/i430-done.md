# I430 — Privacy Clarity

**Status:** Open
**Priority:** P1
**Version:** 1.0.0
**Area:** Frontend

## Summary

The app stores meeting context, email metadata, and AI-generated intelligence locally and sends enrichment context to Claude's API. This needs to be surfaced transparently — in plain language, in the app, without requiring users to read a privacy policy. The Settings → Data section should tell users exactly what's stored, where, and for how long, and give them actionable controls to clear or delete that data.

## Acceptance Criteria

1. Settings → Data contains a "What DailyOS stores" section listing: entity data (accounts, people, projects), meeting history (titles, attendees, prep), email metadata (subject, sender — never body), signal events, AI-generated intelligence summaries, embedding vectors. Each item has a one-sentence plain-language description.
2. A "Data retention" section explains: data is stored locally in `~/.dailyos/dailyos.db`. Nothing is sent to DailyOS servers. Google OAuth tokens are stored in the macOS keychain. AI enrichment calls are made to Claude's API (Anthropic) — meeting context is sent but not retained by default per Anthropic's API policies.
3. A "Clear intelligence" button deletes all AI-generated content (`entity_intel` table, `signal_events` table, `emails.contextual_summary` column) while retaining entity structure (accounts, people, projects still exist). This forces fresh enrichment on next use. `SELECT count(*) FROM entity_intel` returns 0 after clearing.
4. A "Delete all data" button clears the entire DB (`~/.dailyos/dailyos.db`) and workspace files. Requires typing "DELETE" to confirm. App returns to first-run state. This is irreversible — a clear warning is shown.

## Dependencies

- "Clear intelligence" requires a new Tauri command `clear_intelligence() -> Result<(), String>` that DELETEs from `entity_intelligence`, `signal_events`, and NULLs `emails.contextual_summary`. Register in `commands.rs`. **Coordination with I528:** I528 defines `purge_source()` which deletes data by provenance source. `clear_intelligence()` is a broader "wipe everything AI-generated" action. Implementation should use I528's `DataSource` enum to identify AI-sourced data where possible, but `clear_intelligence()` also deletes all entity_intelligence rows regardless of source. These are complementary operations, not duplicates.
- "Delete all data" requires a `delete_all_data() -> Result<(), String>` command that closes the DB connection, deletes the DB file, and clears the workspace directory. The app should restart to first-run state after this action. If FTS5 search index exists (I427), it is deleted with the DB file.
- Settings → Data is the same section as I429 (export). Coordinate these two PRs — the export button and privacy section should live in the same settings panel without visual crowding.
- Review `.docs/design/DESIGN-SYSTEM.md` and `.docs/design/COMPONENT-INVENTORY.md` before building — use existing typography, spacing, and section rule patterns.

## Notes / Rationale

The "clear intelligence" option is distinct from "delete all data." It preserves the user's entity structure (accounts, people, projects still exist with their names and metadata) while wiping all AI-generated content so they can start fresh. This is useful if enrichment quality degrades or if the user changes their role preset and wants to regenerate all intelligence from scratch.

The "delete all data" button should be visually in a destructive zone (red border, warning copy) and require typing "DELETE" as a confirmation rather than a simple OK/Cancel dialog. The typing requirement prevents accidental deletion and makes the user consciously own the action.

The "Data retention" section (criterion 2) should include a link to Anthropic's API usage policy for users who want to read the primary source. The language should be factual and not defensive — the app has a strong privacy story (local-first, no DailyOS servers) and the copy should reflect confidence in that story.

Email metadata clarification: the app stores email subjects, sender names, and sender email addresses from the Gmail API. It does NOT store email bodies. The sync fetches metadata only. This distinction should be explicit in the "What DailyOS stores" section.
