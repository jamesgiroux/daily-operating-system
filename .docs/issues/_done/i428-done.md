# I428 — Offline / Degraded Mode

**Status:** Open
**Priority:** P1
**Version:** 1.0.0
**Area:** Backend

## Summary

When Google APIs, the Claude Code subprocess, or other external services are unavailable, the app must serve cached intelligence gracefully rather than showing error states or blank surfaces. All intelligence, meetings, and actions already live in the local SQLite DB — this issue is primarily about ensuring the app reads from that DB as the authoritative source when live data is unavailable, and surfaces staleness honestly rather than failing.

## Acceptance Criteria

1. When Google Calendar API returns a network error or 5xx, the app shows the last-cached calendar data with a "Last updated X minutes ago" indicator in the folio bar. The daily briefing does not show an error state — it shows cached intelligence with the staleness indicator.
2. When the Claude Code subprocess is unavailable (not installed, network issue, rate limit), the app continues to serve all cached intelligence from the DB. The intel_queue logs failures and retries with backoff. No surfaces show empty states due to PTY unavailability — they show cached content.
3. When Google Gmail API is unavailable, the email page shows the last-fetched emails from the `emails` DB table with a "Mail sync paused" notice. Existing emails remain usable.
4. The system status indicator (Settings → System) shows live connectivity status for: Google Calendar, Gmail, Clay, Gravatar, Linear, and Claude Code. Green/amber/red with last-successful-sync timestamp for each.
5. App startup completes and renders cached data regardless of network status. Background sync attempts happen but failures are silent — they log to `enrichment_log` and retry with backoff. The UI never shows a loading spinner or "could not connect" error on startup. All cached intelligence, meetings, and actions load from DB immediately.

## Dependencies

- The `with_db_try_read` and `with_db_read` DB access patterns already exist — the primary work is ensuring API call failure paths fall back to DB reads rather than propagating errors to the frontend.
- A `last_synced_at` timestamp must be tracked per integration (Calendar, Gmail, Clay, etc.) to power the staleness indicator and the Settings → System status view. No `sync_status` table exists today — this issue needs to create one (migration) or add columns to `workspace_config`.
- **Depends on I511** (schema decomposition) — the sync_status table should be designed against the post-I511 schema, not the current one.
- The system status panel in Settings → System is a new frontend section.

## Notes / Rationale

The DB already has all the cached data needed. This is primarily about ensuring the app reads from DB when API calls fail rather than showing error states. The `with_db_try_read` / `with_db_read` patterns already exist — they just need to be the fallback when live data is unavailable.

The key behavioral contract: a user opening the app on an airplane should see their full daily briefing, accounts, and action queue from the last sync — not a loading spinner or "could not connect" error. Degraded mode is the normal mode when connectivity is absent.

The intel_queue retry-with-backoff (criterion 2) should log failures to the `enrichment_log` table with a `status = 'failed'` and `retry_at` timestamp. The queue poller checks `retry_at` before attempting. Exponential backoff: 1 min, 5 min, 15 min, 1 hour cap.

The system status panel (criterion 4) should poll each connector's last-synced timestamp from DB, not make live health checks. A connector is "green" if last sync was < 30 minutes ago, "amber" if 30 min–4 hours, "red" if > 4 hours or never synced.
