# I427 — Full-Text Search (Cmd+K Command Palette)

**Status:** Open
**Priority:** P1
**Version:** 1.0.0
**Area:** Frontend + Backend

## Summary

Add a Cmd+K command palette that searches across all local data — accounts, people, projects, meetings, actions, and email subjects — and navigates to any entity's detail page. The search backend is SQLite FTS5, which is built into SQLite and requires no external dependency. Results appear within 300ms of keystroke and work fully offline.

## Acceptance Criteria

1. Cmd+K opens a search palette from any screen. The palette is a modal overlay that does not navigate away from the current page.
2. Typing searches across: account names, project names, person names, meeting titles, action titles, email subjects (from the emails table). Results appear within 300ms of keystroke.
3. Each result shows: entity type icon, name, and a one-line context (account: health indicator; person: title + company; meeting: date + entity link; action: status). Clicking a result navigates to that entity's detail page and closes the palette.
4. Results are ranked by recency and relevance: exact name matches first, then partial matches, then fuzzy matches. Recent meetings (last 30 days) rank above historical.
5. The search backend uses SQLite FTS5 (full-text search) over the key text columns. `SELECT * FROM sqlite_master WHERE type='table' AND name LIKE '%fts%'` — returns at least one FTS virtual table.
6. Cmd+K works offline (searches local DB only — no API calls required).

### Glean Federated Search (when Glean connected)
7. When Glean is connected, Cmd+K results include a "Search company knowledge" section below local results. This section calls Glean `chat` with the search query and displays Glean-sourced results (documents, people, accounts from CRM/Zendesk/Gong) with source attribution.
8. Local FTS5 results appear immediately (< 300ms). Glean results stream in asynchronously (2-5s) and append below local results. The UI does not block or show a loading state for local results while waiting for Glean.
9. Glean search results show: document title, source app (Salesforce, Zendesk, Gong, Confluence, etc.), snippet, and a link. Clicking a Glean result opens the source URL in the browser (not a DailyOS page — Glean documents don't have local detail pages).
10. Glean people results show: name, title, department, email. Clicking navigates to the person's DailyOS detail page if they exist locally, or shows "Add to contacts" if they don't.
11. When Glean is NOT connected, Cmd+K shows only local results — no "Search company knowledge" section, no error state. Identical to pure local behavior.
12. Glean search query timeout: 10s. If Glean doesn't respond, the "Search company knowledge" section shows "Glean search unavailable" in muted text. Does not affect local results.

## Dependencies

- SQLite FTS5 is available in the rusqlite build already in use — no new Cargo dependencies required.
- The search palette is a new frontend component. Review `.docs/design/COMPONENT-INVENTORY.md` before building — use existing modal and input components where available.
- FTS5 virtual table must be populated via a migration (new migration file) and updated incrementally when entity rows change.
- Tauri command `search_all(query: String) -> Vec<SearchResult>` must be registered in `commands.rs`.

## Notes / Rationale

FTS5 is built into SQLite — no external dependency needed, no additional binary to distribute. The search index should be built at startup if not already present (via content= shadow table or explicit INSERT triggers) and updated incrementally when entities change. The Cmd+K pattern is already referenced in the release checklist; this is the implementation.

The palette should be rendered at the root layout level (above routing) so that Cmd+K works from any page. Consider using a `useSearchPalette` hook that manages open/close state and is triggered by a global keydown listener attached in the root component.

Ranking hint: FTS5 supports `rank` and `bm25()` scoring natively. Use `ORDER BY rank` for initial relevance, then apply a recency multiplier in the Rust layer for meetings and actions where `created_at` or `event_date` is within 30 days.
