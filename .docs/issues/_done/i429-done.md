# I429 — Data Export

**Status:** Open
**Priority:** P1
**Version:** 1.0.0
**Area:** Backend

## Summary

Users considering using DailyOS for real work need the assurance they can leave. A one-click export from Settings → Data produces a ZIP of all user data and AI-generated intelligence in human-readable JSON. The export reads from local DB only — no cloud required — and aligns with the local-first philosophy that makes the product trustworthy.

## Acceptance Criteria

1. Settings → Data → "Export all data" produces a ZIP file containing:
   - `accounts.json` — all accounts with their entity intel
   - `people.json` — all people with enrichment data
   - `projects.json` — all projects
   - `meetings.json` — all meetings_history with their prep content
   - `actions.json` — all actions
   - `signals.json` — signal_events (last 90 days)
   - `intelligence.json` — entity_intel for all entities
2. The export is completeable offline (reads from local DB only). Export of a typical workspace (50 accounts, 200 people, 500 meetings) completes in under 30 seconds.
3. Exported JSON is human-readable and self-describing — field names match what the user would recognize from the UI (account name, not entity_id). Foreign key IDs are resolved to names where practical (e.g., `account_name` alongside `account_id`).
4. A "What's in your export" description in the UI explains each file before the user exports. No surprises about what's included.
5. The export does NOT include: Google OAuth tokens, keychain credentials, or any credentials. Only user data and intelligence.

## Dependencies

- A new Tauri command `export_all_data(dest_path: String) -> Result<(), String>` must be written and registered in `commands.rs`.
- The `zip` crate (or equivalent) must be added to `src-tauri/Cargo.toml` for ZIP file creation.
- The frontend Settings → Data section is a new or extended section (see I430 — Privacy clarity — which lives in the same Settings area; coordinate these two PRs so they don't conflict).
- The save-panel dialog should use Tauri's `dialog::save` to let the user choose the export destination.

## Notes / Rationale

The export is a trust signal, not just a feature. A user who knows they can export is more willing to invest in the product. The design should feel intentional — not a hidden "download my data" link buried in a danger zone.

Field name resolution (criterion 3): the Rust export layer should JOIN account_id → account name, entity_id → entity name for the top-level arrays. This makes the JSON readable without a schema reference. Internal UUIDs can still be included as `id` fields for re-import tooling.

The ZIP approach (rather than a flat JSON dump) is intentional: it lets users open just the file they want, and it maps naturally to the entity model. Each top-level entity type gets its own file.

Signals export is capped at 90 days to keep export size reasonable. An advanced option to export full signal history is out of scope for 1.0.

Embedding vectors are excluded from the export — they are large, not human-readable, and will be regenerated from the source content on re-import. The `intelligence.json` includes the text content (executive_assessment, etc.) that the embeddings were derived from.
