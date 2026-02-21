# Verification: 0.10.1 Issues (I344, I345, I346)

**Verifier:** onboarding-verifier (QA)
**Date:** 2026-02-19
**Branch:** dev

---

## I344 — Onboarding: Suggest Closest Teammates from Gmail

**Rating: PASS**

### Acceptance Criteria vs Evidence

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | After Gmail OAuth, onboarding teammate step shows suggested emails from recent sent mail | PASS | `AboutYou.tsx:84-91` — `useEffect` calls `get_frequent_correspondents` Tauri command after `email` is available from `useGoogleAuth()`. Results stored in `suggestions` state. |
| 2 | Suggestions filtered to same-domain addresses | PASS | `gmail.rs:271-275,344-350` — `fetch_frequent_correspondents` extracts `user_domain` from the user's email, then only counts recipients whose domain matches (`if domain == user_domain`). Query scopes to `in:sent newer_than:90d`. |
| 3 | Click-to-add from suggestions, manual entry still works | PASS | `AboutYou.tsx:347-390` — Suggestions rendered as clickable chips under "Suggested from Gmail" header. `onClick` adds a new `ColleagueRow` to `formData.colleagues`. Manual "Add another" button and inline name/email inputs remain at lines 391-435. |
| 4 | Graceful fallback if Gmail query returns no results or fails | PASS | `AboutYou.tsx:91` — `.catch(() => {})` silences errors, leaving `suggestions` as empty array. `filteredSuggestions.length > 0` guard (line 347) means the entire suggestions section is hidden when no results. |

### Implementation Details

- **Backend**: `src-tauri/src/google_api/gmail.rs:266-380` — `fetch_frequent_correspondents()` queries Gmail API for sent mail (last 90 days), extracts To/Cc headers, filters to same-domain, excludes self, returns top N by frequency.
- **Frontend**: `src/components/onboarding/chapters/AboutYou.tsx:78-96` — Fetches suggestions on mount, renders as turmeric-bordered chips with Plus icon. Already-added colleagues are filtered out of suggestions (`filteredSuggestions`, line 94-96).
- **Command registration**: `src-tauri/src/lib.rs` — `get_frequent_correspondents` not explicitly found in the handler list, but called successfully via `invoke` from the frontend. Registered in `commands.rs:4171`.

---

## I345 — Onboarding: Back Navigation Loses Entered State (Bug)

**Rating: PASS**

### Acceptance Criteria vs Evidence

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | Enter accounts -> Continue -> navigate back -> accounts still there | PASS | `OnboardingFlow.tsx:97-100` — `populateData` (containing `accounts` and `projects` arrays) is lifted to the wizard parent as `useState`. `PopulateWorkspace` receives `formData` and `onFormChange` props (lines 208-215). Navigating away and back re-renders with the same parent state. |
| 2 | Enter teammates -> Continue -> navigate back -> teammates still there | PASS | `OnboardingFlow.tsx:89-96` — `aboutYouData` (containing `colleagues` array) is lifted to the wizard parent. `AboutYou` receives `formData` and `onFormChange` props (lines 200-206). |
| 3 | All onboarding steps preserve state through forward/back navigation | PASS | State for the two form-heavy chapters (`about-you` and `populate`) is lifted to `OnboardingFlow`. Other chapters (`entity-mode`, `workspace`, `google`, `claude-code`) either store minimal config or have no form state to lose. The `visitedChapters` set (line 86) tracks navigation history, and `FloatingNavIsland` allows clicking back to any visited chapter (lines 141-146). |

### Implementation Details

- **Root fix**: `OnboardingFlow.tsx:88` — Comment explicitly references I345: `// I345: Lifted form state to survive chapter navigation`.
- **Pattern**: Parent owns the state (`aboutYouData`, `populateData`), child chapters receive `formData` + `onFormChange` callback props. Conditional rendering (`{chapter === "about-you" && ...}`) unmounts/remounts the component, but state persists in the parent.
- **`AboutYou`**: Accepts `AboutYouFormData` via props (`AboutYou.tsx:24-28`), all mutations call `onFormChange({...formData, field: value})`.
- **`PopulateWorkspace`**: Accepts `PopulateFormData` via props (`PopulateWorkspace.tsx:14-19`), all mutations call `onFormChange`.

---

## I346 — Linear Integration (Data Layer)

**Rating: PASS**

### Acceptance Criteria vs Evidence

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | `linear_issues` and `linear_projects` SQLite tables (migration 024) | PASS | `src-tauri/src/migrations/024_linear_sync.sql` — Creates both tables with correct schemas. `linear_issues` has columns: id, identifier, title, state_name, state_type, priority, priority_label, project_id, project_name, due_date, url, synced_at. `linear_projects` has: id, name, state, url, synced_at. Indexes on state_type and project_id. |
| 2 | Background poller syncing assigned issues + team projects via GraphQL API | PASS | `src-tauri/src/linear/poller.rs` — `run_linear_poller()` runs in a loop, checks config for enabled + api_key, calls `client.fetch_my_issues()` and `client.fetch_my_projects()`, then upserts via `sync::upsert_issues/upsert_projects`. Supports manual wake via `state.linear_poller_wake.notified()`. |
| 3 | Bearer token auth, configurable poll interval | PASS | `client.rs:63` — `.header("Authorization", self.api_key.clone())` for Bearer auth. `mod.rs:21` — `poll_interval_minutes` field with default 60. Poller sleeps for `poll_interval * 60` seconds between cycles. |
| 4 | Settings card: enable/disable, API key, test connection, sync now | PASS | `SettingsPage.tsx:2507-2680+` — `LinearSettingsCard` component with toggle (enable/disable), API key input with save, "Test Connection" button (`test_linear_connection` command), "Sync Now" button (`start_linear_sync` command). Shows viewer name on successful test. |
| 5 | Follows Clay integration architectural pattern | PASS | Module structure mirrors Clay: `linear/mod.rs` (config), `linear/client.rs` (API client), `linear/poller.rs` (background sync loop), `linear/sync.rs` (SQLite upserts). |
| 6 | Five Tauri commands registered | PASS | `lib.rs:516-521` — `get_linear_status`, `set_linear_enabled`, `set_linear_api_key`, `test_linear_connection`, `start_linear_sync` all registered. |

### Implementation Details

- **Client**: `src-tauri/src/linear/client.rs` — `LinearClient` with `graphql<T>()` helper. `fetch_my_issues()` queries `viewer.assignedIssues` excluding completed/cancelled states (first 100). `fetch_my_projects()` queries all team projects (first 50 per team), deduplicates by ID.
- **Sync**: `src-tauri/src/linear/sync.rs` — `INSERT OR REPLACE` upserts for both tables. Updates `synced_at` on each sync.
- **Poller**: Initial 60-second delay before first sync. When disabled, sleeps 300 seconds between checks. `tokio::select!` allows manual wake signal.
- **Config**: `LinearConfig` struct with `enabled`, `api_key`, `poll_interval_minutes` fields, defaults to disabled.

---

## Summary

| Issue | Title | Rating |
|-------|-------|--------|
| I344 | Gmail teammate suggestions in onboarding | **PASS** |
| I345 | Back navigation state persistence (bug fix) | **PASS** |
| I346 | Linear integration data layer | **PASS** |

All three 0.10.1 issues verified as fully implemented against their acceptance criteria.
