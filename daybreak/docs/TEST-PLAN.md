# DailyOS Daybreak Test Plan

> Comprehensive QA test plan for dogfooding validation.
>
> Date: 2026-02-06
> Status: Draft
> Scope: All features F1-F7, edge cases, stress tests, integration tests

---

## How to Use This Document

Each test scenario has a priority level:
- **P0 (Blocking)** — App cannot ship or be used daily if this fails
- **P1 (Critical)** — Core workflow broken; must fix before sustained dogfooding
- **P2 (Important)** — Noticeable quality gap; fix within validation sprint
- **P3 (Nice-to-have)** — Polish item; defer if needed

Test IDs use the format `{Feature}-{Number}` (e.g., `F1-01`). Edge case tests use `EC-{Number}`, stress tests use `ST-{Number}`, and integration tests use `IT-{Number}`.

**Notation:** `[workspace]` refers to `~/Documents/VIP`. `[config]` refers to `~/.dailyos/config.json`. `[state]` refers to `~/.dailyos/`.

---

## 1. Manual Test Scenarios by Feature

### F1: Morning Briefing

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F1-01 | Scheduled briefing runs automatically | Config has `schedules.today.enabled: true`, cron set to a time 1 minute from now, Claude Code installed and authenticated, Python 3 available | 1. Set cron to fire within 1 minute. 2. Wait. 3. Observe logs and `_today/` directory. | Python prepare script runs, Claude enriches, Python deliver script runs. `_today/data/` contains `schedule.json`, `actions.json`. macOS notification: "Your day is ready". Execution history records success. | P0 |
| F1-02 | Dashboard shows briefing data | F1-01 completed successfully. `_today/data/schedule.json` and `_today/data/actions.json` exist. | 1. Open app. 2. Navigate to Dashboard (root `/`). | Dashboard renders: greeting, date, summary, meeting cards, action list. Stats show correct meeting/action counts. No error or empty state. | P0 |
| F1-03 | Meeting cards expand with prep | F1-01 completed. At least one meeting has a prep file in `_today/data/preps/`. | 1. Open Dashboard. 2. Click on a meeting card that shows "has prep" indicator. | Card expands to show prep details: metrics, risks, wins, actions, stakeholders, questions. Navigation to `/meeting/$prepFile` works. | P1 |
| F1-04 | Manual "Run Briefing Now" from tray | App running in system tray. Claude Code installed. | 1. Right-click system tray icon. 2. Click "Run Briefing Now". 3. Observe logs and `_today/`. | Briefing workflow queues and executes. Dashboard updates when complete. Notification sent. Execution history records trigger as "manual". | P0 |
| F1-05 | Briefing failure shows error in dashboard | Claude Code not authenticated (or `claude` not on PATH). | 1. Trigger briefing manually. 2. Open Dashboard. | DashboardError renders with the specific error message (e.g., "Claude Code CLI not found"). Error notification sent. Execution history records failure with error message. | P1 |
| F1-06 | Actions sync to SQLite post-briefing | F1-01 completed. `_today/data/actions.json` contains at least 3 actions. | 1. After briefing completes, invoke `get_actions_from_db` via the Actions page. | Actions from JSON appear in SQLite. Overdue actions appear first. Priority ordering is correct. IDs are stable across re-runs (upsert, not duplicate). | P1 |
| F1-07 | Email summary renders on dashboard | Briefing output includes `_today/data/emails.json`. | 1. Open Dashboard. | Email section shows sender, subject, priority badge. High-priority emails distinguished visually from normal. | P2 |
| F1-08 | Briefing works with no Google auth | Google not configured (`google.enabled: false`). | 1. Run briefing manually. | Briefing completes using only local data. No crash or error from missing calendar data. Dashboard shows meetings from JSON (if any) or empty meeting section. | P1 |

### F2: Post-Meeting Capture

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F2-01 | Capture prompt appears after customer meeting ends | Google auth connected. Calendar has a "customer" type meeting ending within 5 min. `post_meeting_capture.enabled: true`. | 1. Wait for meeting to end (as detected by calendar poller). 2. Observe UI. | After `delay_minutes` (default 5), if no transcript detected in `_inbox/` within `transcript_wait_minutes` (default 10), a fallback prompt appears in bottom-right corner with meeting title. Auto-dismiss progress bar counts down 60 seconds. | P1 |
| F2-02 | Capture prompt does NOT appear for internal meetings | Calendar has an "internal" or "team_sync" meeting ending. | 1. Wait for meeting to end. | No prompt appears. `should_prompt()` returns false for non-customer/external types. | P1 |
| F2-03 | Skip button dismisses prompt without penalty | F2-01 triggered. Prompt visible. | 1. Click "Skip" button. | Prompt disappears. Meeting ID added to `capture_dismissed` set. Prompt does not reappear for this meeting. No error logged. | P1 |
| F2-04 | Capture a win, risk, and action | F2-01 triggered with full capture prompt (not fallback). | 1. Click "Win" button. 2. Type "Renewed for 3 years" and press Enter. 3. In confirm phase, click "Add more". 4. Click "Risk". 5. Type "Budget concerns for Q3" and press Enter. 6. Click "Add more". 7. Click "Action". 8. Type "Send renewal paperwork" and press Enter. 9. Click "Done". | All three items listed in confirm phase. On "Done": wins saved to `90-impact-log.md` in `_today/`, risks saved to captures table, action saved to actions table in SQLite with `source_type = "post_meeting"`. Prompt closes. | P1 |
| F2-05 | Fallback prompt captures quick note | F2-01 fallback variant shown (no transcript detected). | 1. Type a quick note in the input field. 2. Press Enter or click "Save". | Note saved as a "win" in the CapturedOutcome. Prompt closes. Impact log updated. | P2 |
| F2-06 | Prompt auto-dismisses after 60 seconds | F2-01 triggered. User takes no action. | 1. Wait 60 seconds. | Progress bar depletes to 0%. Prompt disappears silently. No data saved. No error. Meeting can still be captured manually if a mechanism exists. | P2 |
| F2-07 | No prompt when user is in another meeting | Two back-to-back customer meetings. First one ends. Second is currently in progress. | 1. First meeting ends. 2. Observe: `current_in_progress` is not empty because second meeting is active. | No prompt shown. `FallbackReady` state waits for `current_in_progress.is_empty()` before triggering. Prompt fires only after second meeting ends (if applicable). | P2 |
| F2-08 | Transcript detection suppresses prompt | Customer meeting ends. Within 10 minutes, drop a file named `otter-transcript-acme-2026-02-06.md` into `_inbox/`. | 1. Meeting ends. Capture loop enters `WaitingForTranscript` state. 2. Drop transcript file. | `check_for_transcript()` detects the file. State transitions to `TranscriptDetected`. No prompt shown. File processed through normal inbox pipeline. | P1 |
| F2-09 | Capture disabled in settings | Set `post_meeting_capture.enabled: false` in Settings. | 1. Have a customer meeting end. | Capture loop skips all processing when `enabled` is false. No prompts appear. | P2 |

### F3: Background Archive

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F3-01 | Archive runs at midnight | `schedules.archive.enabled: true`, cron set to midnight. `_today/` contains markdown files. | 1. Wait for midnight (or set cron to 1 minute from now for testing). | All `.md` files in `_today/` moved to `_archive/YYYY-MM-DD/`. Data directory and JSON files may or may not be archived (depends on implementation). No notification sent. No dashboard refresh event. Execution record created with success. | P0 |
| F3-02 | Archive with empty `_today/` | `_today/` exists but contains no files (or only the `data/` subdirectory). | 1. Trigger archive manually. | Archive completes without error. "0 files moved" logged. No crash. | P1 |
| F3-03 | Archive creates date directory | `_archive/` exists but no subdirectory for today's date. | 1. Trigger archive. | Directory `_archive/YYYY-MM-DD/` created. Files moved into it. | P1 |
| F3-04 | Archive is silent | Archive running. | 1. Observe UI during archive. | No notification. No toast. No status event emitted to frontend. No visual indication in the app. (This is by design -- Principle 9: "Hide the Plumbing".) | P2 |
| F3-05 | Missed archive runs on wake | Laptop was asleep through midnight. | 1. Close laptop lid before midnight. 2. Open laptop after midnight (within 2-hour grace period). | Scheduler detects time jump > 300 seconds. `check_missed_jobs` finds archive was due. Archive runs with `ExecutionTrigger::Missed`. | P1 |

### F4: Processing Queue (Active Inbox)

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F4-01 | File watcher detects new file | App running. `_inbox/` directory exists. | 1. Copy a `.md` file into `_inbox/`. 2. Observe frontend within 1 second. | `inbox-updated` event emitted after 500ms debounce. Sidebar badge count updates. Inbox page shows new file with filename, size, modified date, and preview. | P0 |
| F4-02 | Manual process single file | `_inbox/` has a recognizable file (e.g., `acme-meeting-notes-2026-02-06.md`). | 1. Navigate to Inbox page. 2. Click "Process" on the file. | Classifier identifies type (e.g., `meeting_notes`). Router moves file to correct PARA destination (e.g., `Accounts/acme/`). File disappears from inbox list. Processing result shown. Processing log entry created in SQLite. | P1 |
| F4-03 | Manual enrich single file | `_inbox/` has an unrecognizable file that returns `NeedsEnrichment` from quick classify. Claude Code available. | 1. Navigate to Inbox page. 2. Click "Enrich" on the file. 3. Wait for Claude to process (1-2 minutes). | AI classifies file. File routed to destination. Post-enrichment hooks run. Actions extracted to SQLite if applicable. File disappears from inbox. | P1 |
| F4-04 | Batch process all inbox files | `_inbox/` has 5 files: 3 recognizable, 2 unknown. | 1. Click "Process All" on Inbox page. | 3 files classified and routed immediately. 2 files flagged as `NeedsEnrichment`. Results shown per file. | P1 |
| F4-05 | Inbox page shows empty state | `_inbox/` is empty. | 1. Navigate to Inbox page. | "Inbox is clear" empty state message. No error. Count shows 0. | P2 |
| F4-06 | Copy to inbox via drag-drop | App has drop zone functionality. | 1. Drag a file from Finder onto the inbox area. | `copy_to_inbox` command copies file to `_inbox/`. File watcher detects it. Inbox count updates. Original file untouched. | P2 |
| F4-07 | Duplicate filename handling | `_inbox/` already has `notes.md`. | 1. Copy another `notes.md` to inbox via `copy_to_inbox`. | File saved as `notes (1).md`. No overwrite. No error. | P2 |
| F4-08 | Binary file preview | `_inbox/` has a `.png` image. | 1. Navigate to Inbox page. 2. Click to preview the file. | Preview shows `[Binary file -- .png -- NNNN bytes]` message instead of garbled text. "Process" button still available. | P2 |
| F4-09 | Path traversal prevention | Attacker tries to read `../../etc/passwd` via `get_inbox_file_content`. | 1. Call `get_inbox_file_content` with filename `../../etc/passwd`. | Returns "Invalid filename" error. File path must start with `workspace/_inbox/`. | P1 |
| F4-10 | Inbox batch schedule fires | `schedules.inbox_batch.enabled: true`. Cron set to fire within 1 minute. | 1. Wait for scheduled time. | Inbox batch executes: classifies all files, enriches up to 5, emits `inbox-updated`. | P2 |

### F5: Weekly Planning

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F5-01 | Week workflow prepares data | `schedules.week.enabled: true`. Google auth connected (for calendar data). Scripts `prepare_week.py` and `deliver_week.py` exist. | 1. Trigger week workflow manually (or wait for Monday 5 AM schedule). | Three-phase workflow: prepare script runs, Claude enriches, deliver script writes `_today/data/week-overview.json`. `week-data-ready` event emitted. `week_planning_state` set to `DataReady`. Notification: "Your week is ready". | P1 |
| F5-02 | Weekly planning wizard opens on Monday | F5-01 completed. Today is Monday. `planningState` is `dataready`. | 1. Open app on Monday after week data is ready. | `useWeekPlanning` hook detects `dataready` state and `getDay() === 1`. Wizard overlay opens automatically. Week overview (calendar grid, action summary, hygiene alerts) visible. | P2 |
| F5-03 | Priority selection step | Wizard open. Step 0 (priority picker) visible. | 1. Select 3 priorities from suggested list. 2. Click "Next". | `submit_week_priorities` called with 3 items. `week-priorities.json` written to `_today/data/`. Planning state changes to `InProgress`. Wizard advances to step 1. | P2 |
| F5-04 | Focus blocks step | Wizard at step 2 (focus blocks). Available time blocks shown with toggles. | 1. Toggle 2 focus blocks on. 2. Click "Finish". | `submit_focus_blocks` called with selected blocks. `week-focus-selected.json` written. Planning state changes to `Completed`. Wizard closes. | P2 |
| F5-05 | Skip weekly planning | Wizard open at any step. | 1. Click "Skip" or "Do Later". | If "Skip All": `skip_week_planning` called, state becomes `DefaultsApplied`, wizard closes. If "Do Later": wizard hides but state unchanged, can re-open. | P2 |
| F5-06 | Wizard does not open on non-Monday | F5-01 completed. Today is Tuesday-Sunday. | 1. Open app. | Wizard does NOT auto-open. `getDay() !== 1` check prevents it. Week data still accessible via `/week` page. | P2 |
| F5-07 | Week page shows data without wizard | Navigate to `/week` route directly. Week data exists. | 1. Click "Week" in sidebar. | WeekPage renders week overview: calendar grid per day, meeting list, action summary, hygiene alerts, available time blocks. No wizard required. | P2 |

### F6: System Tray Presence

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F6-01 | Tray icon visible on launch | App launched via `pnpm tauri dev` or built binary. | 1. Launch app. 2. Look at macOS menubar. | Tray icon visible as monochrome template image. Correct appearance in both light and dark menubar modes. | P0 |
| F6-02 | Left-click opens main window | App running. Main window hidden. | 1. Left-click tray icon. | Main window shows and gains focus. If window was already visible, it gains focus. | P0 |
| F6-03 | Right-click shows menu | App running. | 1. Right-click tray icon. | Menu shows: "Open DailyOS", "Run Briefing Now", "Quit". All items enabled. | P0 |
| F6-04 | "Open DailyOS" menu item | Main window hidden. | 1. Right-click tray. 2. Click "Open DailyOS". | Main window shows and gains focus. | P1 |
| F6-05 | "Run Briefing Now" menu item | Claude Code available. | 1. Right-click tray. 2. Click "Run Briefing Now". | Briefing workflow queued. Status visible in execution history. Same behavior as F1-04. | P1 |
| F6-06 | "Quit" menu item | App running. | 1. Right-click tray. 2. Click "Quit". | App exits completely. Process no longer running. Tray icon disappears. | P0 |
| F6-07 | Window close hides instead of quits | Main window visible. | 1. Click the red close button (traffic light). | Window hides. App continues running in tray. Tray icon still visible. Clicking tray restores window. | P0 |
| F6-08 | Notification appears after briefing | Briefing workflow completes successfully. | 1. Observe macOS Notification Center. | Native notification: title "Your day is ready", body "DailyOS has prepared your briefing". Clicking notification (if supported) should open the app. | P1 |

### F7: Dashboard

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| F7-01 | Dashboard loads under 1 second | `_today/data/` has valid JSON files. App cold start. | 1. Launch app. 2. Time from window visible to dashboard content rendered. | Content visible within 1 second. Skeleton shows during loading. No flash of error state. | P0 |
| F7-02 | Dashboard empty state (no briefing) | `_today/` directory does not exist. | 1. Open Dashboard. | DashboardEmpty component renders: "No briefing yet. Run /today to generate your daily overview." Positive framing. No error. | P1 |
| F7-03 | Dashboard empty state (no data dir) | `_today/` exists but `_today/data/` does not. | 1. Open Dashboard. | DashboardEmpty: "No data found. Run /today to generate your daily briefing." | P1 |
| F7-04 | Dashboard error state (malformed JSON) | `_today/data/schedule.json` exists but contains invalid JSON. | 1. Open Dashboard. | DashboardError renders with "Failed to load schedule" message. Retry button available. Error details visible. | P1 |
| F7-05 | Dashboard refreshes after workflow completes | Dashboard open. Briefing runs in background. | 1. Trigger briefing. 2. Wait for completion. 3. Observe dashboard. | `workflow-completed` event triggers `useDashboardData` refresh. Dashboard updates to show new data without manual page reload. | P1 |
| F7-06 | Meeting card types visually distinct | Dashboard has customer, internal, and personal meetings. | 1. Inspect meeting cards. | Customer meetings have gold left-border and blue badge. Internal meetings have default styling and gray badge. Personal meetings have sage/green indicator. Meeting type badge text is correct. | P2 |
| F7-07 | Stats bar shows correct counts | Dashboard loaded with 4 meetings (2 customer), 6 actions, 3 inbox files. | 1. Inspect stats bar. | Shows: "4 meetings", "2 customer", "6 actions", "3 inbox". Counts match actual data. | P2 |
| F7-08 | Dashboard responsive to window resize | Dashboard loaded. | 1. Resize window from full-size to minimum width. 2. Resize back. | Layout adapts. No content overflow. No horizontal scrollbar. Cards stack or reflow. Sidebar collapses to icon mode. | P2 |
| F7-09 | Dark mode toggle | Dashboard loaded in light mode. | 1. Toggle theme via Settings or system preference. | All colors switch correctly. Gold accent unchanged. Cream bg becomes charcoal. Text remains readable. No FOUC. | P2 |

### Supporting Features: Actions Page

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| ACT-01 | Actions page loads from JSON | `_today/data/actions.json` exists with mixed priorities. | 1. Navigate to `/actions`. | All actions rendered with correct priority badges, due dates, account tags. Overdue items visually distinguished. | P1 |
| ACT-02 | Mark action complete | Action visible on Actions page. SQLite DB available. | 1. Click checkbox on an action. | `complete_action` IPC called. Action status changes to "completed". `completed_at` timestamp set. Visual state updates (strikethrough or fade). | P1 |
| ACT-03 | Filter actions by status | Actions page has both pending and completed actions. | 1. Use filter controls to toggle pending/completed. | List updates to show only matching actions. Counts in filter labels are correct. | P2 |

### Supporting Features: Settings Page

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| SET-01 | Settings page renders all cards | Config loaded. | 1. Navigate to `/settings`. | All settings cards visible: Profile, Google Account, Schedules, Post-Meeting Capture, Workspace info. No errors. Current values displayed. | P1 |
| SET-02 | Profile switch persists | Profile currently "general". | 1. Change profile to "customer-success" in Settings. | `set_profile` IPC called. Config written to disk. Sidebar updates to show CS-specific items (Accounts). Page reloads to reflect new profile. | P1 |
| SET-03 | Google connect flow | Google not configured. | 1. Click "Connect Google Account" in Settings. | `start_google_auth` IPC called. Browser opens for OAuth flow. After completing auth, Settings shows authenticated email. Auth status event emitted. Calendar poller starts polling. | P1 |
| SET-04 | Google disconnect flow | Google authenticated. | 1. Click "Disconnect" on Google card in Settings. | `disconnect_google` called. Token file removed. Auth status changes to `NotConfigured`. Calendar events cleared. Auth status event emitted. | P1 |
| SET-05 | Toggle post-meeting capture | Capture currently enabled. | 1. Toggle capture off in Settings. | `set_capture_enabled(false)` called. Config updated on disk. Capture loop stops processing. Toggle back on restores functionality. | P2 |
| SET-06 | Change capture delay | Current delay is 5 minutes. | 1. Change delay to 10 minutes in Settings. | `set_capture_delay(10)` called. Config updated. Next post-meeting prompt will use 10-minute delay. | P3 |

### Supporting Features: Navigation and Layout

| ID | Description | Preconditions | Steps | Expected Result | Priority |
|----|-------------|---------------|-------|-----------------|----------|
| NAV-01 | Sidebar navigation | App loaded. | 1. Click each sidebar item: Dashboard, Actions, Inbox, Settings. | Each page loads without error. Active item highlighted in sidebar. URL updates. | P0 |
| NAV-02 | Command menu (Cmd+K) | App focused. | 1. Press Cmd+K. 2. Type "inbox". 3. Select result. | Command menu opens. Search works. Navigation occurs. Menu closes. | P2 |
| NAV-03 | Profile selector on first launch | Config has no `profile` field (or empty). | 1. Launch app. | Profile selector modal appears. User must choose "Customer Success" or "General". Choice persists to config. App reloads with new profile. | P1 |
| NAV-04 | Titlebar drag region | macOS overlay titlebar visible. | 1. Click and drag on the header area (above content, near traffic lights). | Window moves. Traffic lights functional. No dead zones where drag doesn't work. | P2 |
| NAV-05 | CS-specific sidebar items | Profile is "customer-success". | 1. Inspect sidebar. | "Accounts" item visible. "Projects" may be visible. "Emails" and "Focus" accessible. | P2 |

---

## 2. Edge Case Tests

| ID | Description | Setup | Steps | Expected Result | Priority |
|----|-------------|-------|-------|-----------------|----------|
| EC-01 | Google OAuth token expires mid-workflow | Valid token exists. Simulate expiry by modifying `~/.dailyos/google/token.json` to remove `refresh_token`. | 1. Wait for calendar poll cycle. | `poll_calendar` returns `PollError::AuthExpired`. Google auth status set to `TokenExpired`. `google-auth-changed` event emitted with `TokenExpired`. Settings page shows "Token Expired" state. No crash. Calendar events cleared. | P1 |
| EC-02 | Claude Code not installed | `claude` binary not on PATH. | 1. Trigger briefing manually. | `PtyManager::is_claude_available()` returns false. `ExecutionError::ClaudeCodeNotFound` raised. Error notification: "Claude Code CLI not found". Dashboard shows error with recovery suggestion: "Install Claude Code from https://claude.ai/code". No crash. | P0 |
| EC-03 | Claude Code not authenticated | `claude` binary exists but not logged in. | 1. Trigger briefing. | PTY output contains "not authenticated" or "login required". `ExecutionError::ClaudeCodeNotAuthenticated` raised. Error notification with recovery suggestion: "Run 'claude login' in your terminal". | P1 |
| EC-04 | Workspace path does not exist | `config.json` points to `/nonexistent/path`. | 1. Launch app. | `load_config()` returns error: "Workspace path does not exist: /nonexistent/path". Dashboard shows error state. Watcher disabled (logs warning). Scheduler runs but executor fails with `WorkspaceNotFound`. | P0 |
| EC-05 | Workspace missing `_inbox/` directory | Workspace exists but `_inbox/` does not. | 1. Launch app. | Watcher creates `_inbox/` directory automatically (`create_dir_all`). Initial inbox count emitted as 0. No error. | P1 |
| EC-06 | Workspace missing `_today/` directory | Workspace exists but `_today/` does not. | 1. Open Dashboard. | `DashboardResult::Empty` returned with message "No briefing yet." No crash. Briefing can still be triggered to create the directory. | P1 |
| EC-07 | Non-text files in `_inbox/` | `_inbox/` contains `photo.png`, `spreadsheet.xlsx`, `binary.dat`. | 1. Navigate to Inbox page. 2. Try to preview each file. | Files listed with correct `InboxFileType` (Image, Spreadsheet, Other). Preview returns binary file message for non-text files. "Process" button available. No crash on binary read attempt. | P1 |
| EC-08 | Malformed `_today/data/schedule.json` | Replace `schedule.json` content with `{invalid json`. | 1. Open Dashboard. | `DashboardResult::Error` with "Failed to load schedule" message. Other JSON files (actions, emails) not loaded either (schedule is required). Error is recoverable via retry or re-running briefing. | P1 |
| EC-09 | Malformed `config.json` | Replace config with `{invalid`. | 1. Launch app. | `load_config()` fails: "Failed to parse config". App starts but in degraded state. Dashboard shows config error. No crash. Tray still works. | P0 |
| EC-10 | `config.json` missing required fields | Config has `{}` (empty object, missing `workspacePath`). | 1. Launch app. | Serde deserialization fails because `workspacePath` has no default. Error message is clear about what's missing. | P1 |
| EC-11 | Two meetings end at the same time | Calendar has Meeting A and Meeting B both ending at 2:00 PM. Both are customer type. | 1. Wait for 2:00 PM. Capture loop detects both left `previous_in_progress`. | Two `PendingPrompt` entries created. Prompts fire sequentially (one at a time, since `current_in_progress` check applies). Second prompt fires after first is dismissed/captured. No duplicate prompts. | P2 |
| EC-12 | File watcher burst (50 files) | Prepare 50 `.md` files. | 1. Copy all 50 files into `_inbox/` at once (e.g., `cp *.md _inbox/`). | Debounce window (500ms) coalesces events. A single `inbox-updated` event fires after settling. Count reflects all 50 files. No event flooding. Channel buffer (64) handles burst. | P1 |
| EC-13 | Config missing Google section | Config has `workspacePath` but no `google` key. | 1. Launch app. | `GoogleConfig::default()` applies: `enabled: false`, poll interval 5 min, work hours 8-18. Calendar poller runs but `should_poll` returns false (not authenticated). No crash. | P1 |
| EC-14 | SQLite database corrupted | Replace `~/.dailyos/actions.db` with random bytes. | 1. Launch app. | `ActionDb::open()` fails. `db` field in AppState is `None`. Warning logged. DB-dependent features degrade gracefully: actions page shows JSON data only, `complete_action` returns "Database not initialized", processing log not written. App otherwise functional. | P1 |
| EC-15 | SQLite database locked | Another process holds a write lock on `actions.db`. | 1. Open a second SQLite connection with a long-running transaction. 2. Launch DailyOS app. | WAL mode mitigates most locking issues. If lock persists, operations fail with SQLite busy error. Failures are non-fatal (warnings logged). Core app functionality (dashboard, tray) unaffected. | P2 |
| EC-16 | System wakes from sleep during scheduled workflow | Laptop sleeps at 7:55 AM. Briefing scheduled for 8:00 AM. Wake at 8:30 AM. | 1. Sleep laptop. 2. Wake within 2-hour grace period. | Time jump detected (> 300 seconds). `check_missed_jobs` finds briefing was due at 8:00 AM. Briefing runs with `ExecutionTrigger::Missed`. Archive (midnight) also detected if applicable. | P1 |
| EC-17 | System wakes after grace period | Sleep through entire grace period (> 2 hours past scheduled time). | 1. Sleep at 7:55 AM. Wake at 10:30 AM. | `find_missed_job` scans grace period window. If no scheduled time found within grace, job is NOT run. This prevents stale briefings from firing hours late. Normal next-scheduled-time applies. | P2 |
| EC-18 | Timezone change (travel) | Config timezone is `America/New_York`. User physically moves to `America/Los_Angeles`. | 1. Change system timezone. 2. Observe scheduler behavior. | Scheduler uses the timezone from `config.json` (not system timezone) for cron evaluation. Briefing still fires at 8 AM Eastern regardless of system clock. This may be surprising but is correct per config. Document this behavior. | P2 |
| EC-19 | Disk nearly full | Less than 10MB free disk space. | 1. Trigger briefing. 2. Try to archive. 3. Try inbox processing. | Write operations fail with IO errors. `ExecutionError::IoError` surfaces. Error notification sent. No data corruption. Existing files preserved. App remains responsive (UI is in-memory). | P3 |
| EC-20 | Partial file write in `_inbox/` | A large file is being written to `_inbox/` by another process. Watcher triggers mid-write. | 1. Start writing a 10MB file to `_inbox/`. 2. Observe watcher behavior. | Watcher fires event. Debounce (500ms) may catch the initial write. If classifier reads partial content, it may misclassify or error. Processing should not corrupt the file. Worst case: `NeedsEnrichment` classification. | P2 |
| EC-21 | `_today/data/` has extra unknown JSON files | `_today/data/` has `custom-field.json` alongside expected files. | 1. Open Dashboard. | Extra files ignored. Dashboard loads normally. JSON loader only reads expected files (`schedule.json`, `actions.json`, `emails.json`). No error. | P3 |
| EC-22 | Config `workspacePath` has spaces | `workspacePath` set to `/Users/jane/My Documents/VIP`. | 1. Launch app. 2. Trigger briefing. | Path handled correctly throughout: config parsing, workspace validation, script execution (`current_dir`), file operations. No shell escaping issues. | P1 |
| EC-23 | Execution history exceeds limit | Run 150 workflows (manual trigger in rapid succession). | 1. Check execution history. | History capped at 100 entries (`MAX_HISTORY_SIZE`). Oldest entries truncated. JSON file on disk reflects capped list. No memory growth. | P3 |
| EC-24 | Google token file exists but is empty JSON object | `~/.dailyos/google/token.json` contains `{}`. | 1. Launch app. 2. Check Google auth status. | `detect_google_auth()` checks for `token` or `refresh_token` fields. Empty object returns `NotConfigured`. No "authenticated" false positive. | P1 |
| EC-25 | Python not installed | `python3` not on PATH. | 1. Trigger briefing. | `run_python_script` detects `PythonNotFound`. Error notification with recovery: "Install Python 3.8+ from https://python.org". | P1 |
| EC-26 | Prepare script fails (non-zero exit) | `prepare_today.py` fails with exit code 1. | 1. Trigger briefing. | `ScriptFailed` error raised with exit code and stderr. Workflow stops -- Phase 2 (Claude) and Phase 3 (deliver) do NOT run. Error recorded in execution history. Notification sent. | P1 |
| EC-27 | Claude Code hits subscription limit | Claude output contains "subscription" and "limit". | 1. Trigger briefing during period of high usage. | `ExecutionError::ClaudeSubscriptionLimit` detected via PTY output pattern matching. Error notification with recovery: "Your Claude subscription limit was reached." | P2 |
| EC-28 | Claude Code times out (5 minute limit) | Claude takes longer than 300 seconds. | 1. Trigger briefing with complex workspace. | `recv_timeout` fires after 5 minutes. `ExecutionError::Timeout(300)` raised. Workflow fails. Error notification. PTY reader thread eventually terminates when Claude exits. | P2 |

---

## 3. Stress Tests

| ID | Description | Setup | Execution | Expected Result | Priority |
|----|-------------|-------|-----------|-----------------|----------|
| ST-01 | Rapid file drops in `_inbox/` | Prepare 10 `.md` files. | Copy all 10 files in under 5 seconds: `for f in *.md; do cp "$f" _inbox/; done` | Watcher debounce coalesces to 1-2 events. Final inbox count accurate. No events lost. Channel buffer (64) not exhausted. Frontend shows correct count. | P1 |
| ST-02 | Very large file in `_inbox/` (10MB transcript) | Create a 10MB `.md` file filled with meeting transcript text. | 1. Copy to `_inbox/`. 2. Preview it. 3. Process it. | Preview loads (potentially truncated in UI). Classifier reads full content without OOM. Processing completes. File routed or flagged for enrichment. No timeout on read. | P2 |
| ST-03 | 20+ meetings in a day | `_today/data/schedule.json` has 25 meeting entries with preps. | 1. Open Dashboard. 2. Scroll through meeting list. 3. Expand several meetings. | All meetings render. Scroll is smooth. Expand/collapse works. No layout overflow. Stats show "25 meetings". Performance acceptable (< 100ms render). | P2 |
| ST-04 | 500+ actions in SQLite | Insert 500 actions into `actions.db` via test script. | 1. Navigate to Actions page. 2. Filter and scroll. | Actions page loads. Pagination or virtual scroll handles large list. Query returns results in < 500ms. Filter operations responsive. | P2 |
| ST-05 | App running 24+ hours | Leave app running overnight and through the next day. | 1. Verify tray icon still present after 24 hours. 2. Open Dashboard. 3. Trigger manual briefing. | No memory leaks. No scheduler drift. Tray responsive. Dashboard loads. Briefing executes. Archive ran at midnight. Calendar poller still running (if configured). | P1 |
| ST-06 | Rapid workflow triggers | Click "Run Briefing Now" 5 times in quick succession. | 1. Right-click tray -> "Run Briefing Now" x5 rapidly. | Channel buffer (32) handles queued messages. Workflows execute sequentially (receiver processes one at a time). No duplicate concurrent executions. No channel overflow panic. | P2 |
| ST-07 | Many prep files | `_today/data/preps/` has 20 JSON prep files. | 1. Open Dashboard. 2. Click through several meeting cards. | Each prep loads on demand via `get_meeting_prep`. No preloading of all preps at dashboard load. Individual prep loads quickly. | P3 |

---

## 4. Integration Tests

These test full end-to-end flows across multiple system components.

### IT-01: Full Briefing Cycle

**Goal:** Validate the complete path from scheduler to dashboard.

**Preconditions:** Config valid. Claude Code installed and authenticated. Python 3 available. Google auth optional.

**Steps:**
1. Set `schedules.today.cron` to fire in 1 minute.
2. Wait for scheduler to detect due time.
3. Observe: `SchedulerMessage` sent with `WorkflowId::Today` and `ExecutionTrigger::Scheduled`.
4. Observe executor receives message.
5. Phase 1: `prepare_today.py` runs with workspace as cwd. `WORKSPACE` env var set.
6. Verify: `.today-directive.json` or equivalent output exists.
7. Phase 2: PTY spawns `claude --print "/today"` in workspace.
8. Verify: Claude output captured.
9. Phase 3: `deliver_today.py` runs. `_today/data/schedule.json`, `actions.json` written.
10. Post-processing: actions synced to SQLite via `sync_actions_to_db`.
11. Execution record updated: `success: true`, `finished_at` set, `duration_secs` calculated.
12. `workflow-completed` event emitted.
13. macOS notification: "Your day is ready".
14. Dashboard: `useDashboardData` receives event, re-fetches, renders new data.

**Pass Criteria:** All 14 steps complete without error. Dashboard shows fresh data. Execution history has 1 new successful record.

### IT-02: Full Inbox Processing Cycle

**Goal:** Validate file drop through routing and action extraction.

**Preconditions:** App running. DB available.

**Steps:**
1. Drop `acme-meeting-notes-2026-02-06.md` into `_inbox/` (file contains `- [ ] Follow up with Acme on renewal`).
2. Watcher detects file (within 500ms debounce).
3. `inbox-updated` event fires. Sidebar badge shows 1.
4. Navigate to Inbox page. File listed.
5. Click "Process" on the file.
6. Classifier identifies `meeting_notes` pattern (filename contains "meeting-notes" and "acme").
7. Router resolves destination: `Accounts/acme/` (or similar PARA path).
8. `move_file` moves the file.
9. Post-enrichment hooks run: `sync_actions_to_sqlite` extracts the checkbox action. `sync_completion_to_markdown` runs (no-op since action is new).
10. Processing log entry written to SQLite.
11. Inbox refreshes: file no longer listed.
12. Navigate to Actions page: "Follow up with Acme on renewal" appears with `source_type: inbox`.

**Pass Criteria:** File successfully classified, routed, and actions extracted. End state: file in PARA location, action in SQLite, processing log recorded.

### IT-03: Full Post-Meeting Capture Cycle

**Goal:** Validate meeting end detection through capture persistence.

**Preconditions:** Google auth connected. Calendar has a customer meeting ending within 5 minutes. Capture enabled.

**Steps:**
1. Calendar poller fetches events. Customer meeting currently in progress.
2. Meeting ends. Next poll cycle: meeting no longer in `current_in_progress`.
3. `capture.rs` detects ended meeting. `PendingPrompt` created with `WaitingForTranscript` state.
4. Wait `transcript_wait_minutes` (default 10). No transcript detected in `_inbox/`.
5. State transitions to `FallbackReady`.
6. After `delay_minutes` (default 5): no other meeting in progress.
7. `post-meeting-prompt-fallback` event emitted with meeting data.
8. `usePostMeetingCapture` hook receives event. `PostMeetingPrompt` renders (fallback variant).
9. User types "Good discussion on roadmap alignment" and clicks "Save".
10. `capture_meeting_outcome` IPC called with outcome.
11. Backend: meeting ID added to `capture_captured` set. Win saved to `captures` table. `90-impact-log.md` created/appended.
12. Prompt closes.

**Pass Criteria:** Capture detected, prompted, captured, and persisted. Impact log file exists with the win entry.

### IT-04: Full Calendar Integration Cycle

**Goal:** Validate Google OAuth through calendar event display.

**Preconditions:** Google API credentials available (`google_auth.py` script works).

**Steps:**
1. In Settings, click "Connect Google Account".
2. Browser opens. Complete OAuth flow.
3. Token saved to `~/.dailyos/google/token.json`.
4. `start_google_auth` returns email. Auth status changes to `Authenticated`.
5. `google-auth-changed` event emitted.
6. Calendar poller: `should_poll` now returns true (authenticated + within work hours).
7. Poller calls `calendar_poll.py`. Script returns JSON array of events.
8. Events parsed and stored in `AppState::calendar_events`.
9. `calendar-updated` event emitted.
10. `useCalendar` hook receives event, fetches fresh events.
11. `currentMeeting` and `nextMeeting` derived from events.
12. Dashboard meeting cards (from briefing) enriched with live calendar context.

**Pass Criteria:** Auth flow completes. Events polled and stored. Frontend reflects live calendar state.

### IT-05: Token Refresh / Re-authentication Cycle

**Goal:** Validate graceful handling of expired Google tokens.

**Steps:**
1. Start with authenticated state (IT-04 completed).
2. Manually invalidate token (delete `refresh_token` from `token.json`, or wait for natural expiry).
3. Calendar poller attempts poll. Script exits with code 2.
4. `PollError::AuthExpired` detected.
5. Auth status changes to `TokenExpired`.
6. `google-auth-changed` event emitted with `TokenExpired`.
7. Settings page shows "Token Expired" state with re-auth option.
8. User clicks "Reconnect".
9. Auth flow re-runs. New token saved.
10. Poller resumes on next cycle.

**Pass Criteria:** Expiry detected gracefully. User guided to re-auth. No crash. No stale data displayed.

### IT-06: Action Completion Bidirectional Sync

**Goal:** Validate that completing an action in the UI updates both SQLite and source markdown.

**Preconditions:** Briefing ran. `_today/` has action items in both JSON and a markdown file. Actions synced to SQLite.

**Steps:**
1. Navigate to Actions page.
2. Mark an action as complete (click checkbox).
3. `complete_action` IPC sets status to "completed" in SQLite.
4. Drop a file into `_inbox/` to trigger processing with post-enrichment hooks.
5. `sync_completion_to_markdown` hook queries recently completed actions.
6. Finds the completed action with `source_label` pointing to a `.md` file.
7. Reads the source file, replaces `- [ ] {title}` with `- [x] {title}`.
8. Writes the updated file back.

**Pass Criteria:** Checkbox in markdown file changes from `[ ]` to `[x]`. SQLite status is "completed". Both representations in sync.

---

## 5. Regression Checklist

Quick-run checklist after any code change. Estimated time: 10-15 minutes.

### App Launch and Core UI

- [ ] `pnpm tauri dev` launches without errors in terminal
- [ ] Main window appears with content (not blank white screen)
- [ ] Tray icon visible in macOS menubar
- [ ] Left-click tray opens window
- [ ] Right-click tray shows menu with 3 items
- [ ] Window close (red traffic light) hides window, does not quit
- [ ] Re-click tray restores hidden window

### Navigation

- [ ] Sidebar renders with correct items for active profile
- [ ] Click Dashboard (/) -- page loads
- [ ] Click Actions (/actions) -- page loads
- [ ] Click Inbox (/inbox) -- page loads
- [ ] Click Settings (/settings) -- page loads
- [ ] Cmd+K opens command menu
- [ ] Command menu search filters results

### Dashboard

- [ ] Dashboard shows data (or appropriate empty/error state)
- [ ] Greeting and date visible
- [ ] Meeting cards render with type indicators
- [ ] Action list renders with priority badges
- [ ] Stats bar shows counts
- [ ] Meeting card expand/collapse works (if prep available)

### Actions Page

- [ ] Actions load from JSON or SQLite
- [ ] Filter controls present and functional
- [ ] Mark action complete changes visual state

### Inbox Page

- [ ] Inbox shows files or "clear" empty state
- [ ] File count matches sidebar badge
- [ ] File preview works for text files
- [ ] Process button triggers classification

### Settings Page

- [ ] All settings cards render
- [ ] Profile displayed correctly
- [ ] Google auth status shown (Connected/Not configured/Expired)
- [ ] Schedule information visible
- [ ] Capture toggle present

### Workflow Execution

- [ ] Manual "Run Briefing Now" from tray queues workflow
- [ ] Workflow status events visible in console/logs
- [ ] Success notification appears on briefing completion
- [ ] Error notification appears on briefing failure

### Theme

- [ ] Light mode renders correctly
- [ ] Dark mode renders correctly
- [ ] Theme toggle persists across app restart

---

## 6. Test Environment Setup

### Prerequisites

| Dependency | Version | Check Command |
|-----------|---------|---------------|
| Rust | 1.70+ | `rustc --version` |
| Node.js | 18+ | `node --version` |
| pnpm | 8+ | `pnpm --version` |
| Python | 3.8+ | `python3 --version` |
| Claude Code CLI | Latest | `claude --version` |
| Tauri CLI | 2.x | `cargo tauri --version` |

### Test Workspace Setup

For isolated testing, create a test workspace:

```bash
mkdir -p /tmp/dailyos-test/{_today/data,_inbox,_archive,_reference,Accounts,Projects}
```

Create a minimal config:

```bash
mkdir -p ~/.dailyos
cat > ~/.dailyos/config-test.json << 'EOF'
{
  "workspacePath": "/tmp/dailyos-test",
  "profile": "general",
  "schedules": {
    "today": { "enabled": false, "cron": "0 8 * * 1-5", "timezone": "America/New_York" },
    "archive": { "enabled": false, "cron": "0 0 * * *", "timezone": "America/New_York" },
    "inboxBatch": { "enabled": false, "cron": "0 */2 * * 1-5", "timezone": "America/New_York" },
    "week": { "enabled": false, "cron": "0 5 * * 1", "timezone": "America/New_York" }
  }
}
EOF
```

### Sample Test Data

Create minimal `_today/data/` fixtures for dashboard testing:

```bash
# schedule.json
cat > /tmp/dailyos-test/_today/data/schedule.json << 'EOF'
{
  "overview": {
    "greeting": "Good morning",
    "date": "Thursday, February 6, 2026",
    "summary": "3 meetings today, including 1 customer call.",
    "focus": "Focus on Acme renewal preparation."
  },
  "meetings": [
    {
      "id": "mtg-001",
      "time": "09:00",
      "endTime": "09:30",
      "title": "Acme Corp Check-in",
      "type": "customer",
      "account": "Acme Corp",
      "hasPrepFile": "acme-corp-check-in",
      "hasPrep": true
    },
    {
      "id": "mtg-002",
      "time": "11:00",
      "endTime": "11:30",
      "title": "Team Standup",
      "type": "internal",
      "hasPrep": false
    }
  ]
}
EOF

# actions.json
cat > /tmp/dailyos-test/_today/data/actions.json << 'EOF'
[
  {
    "id": "act-001",
    "title": "Send Acme renewal proposal",
    "account": "Acme Corp",
    "dueDate": "2026-02-07",
    "priority": "P1",
    "status": "pending"
  },
  {
    "id": "act-002",
    "title": "Review Q1 metrics dashboard",
    "priority": "P2",
    "status": "pending"
  }
]
EOF
```

---

## 7. Known Gaps and Risks from Product Assessment

These items from `PRODUCT-ASSESSMENT.md` represent the highest-risk areas. Prioritize testing here during dogfooding.

| Risk ID | Description | Test Coverage |
|---------|-------------|---------------|
| R1 | Claude Code PTY issues across machines | EC-02, EC-03, EC-27, EC-28 |
| R2 | Google API token expiry | EC-01, EC-24, IT-04, IT-05 |
| R3 | File watcher unreliability | F4-01, EC-12, EC-20, ST-01 |
| R4 | Scheduler drift on sleep/wake | F3-05, EC-16, EC-17, ST-05 |
| A1 | User may not have Claude Code | EC-02, EC-25 |
| A2 | Workspace PARA structure assumed | EC-04, EC-05, EC-06 |
| A3 | `_today/` file format assumed | EC-08, EC-21, F7-04 |

---

*This test plan is a living document. Update pass/fail status during validation sprints. Add new scenarios as bugs are discovered.*
