# Implementation Plan

> Frontend-first phased approach to MVP and beyond.

---

## Approach

**Frontend-first** — Build UI with mock data, then add backend to match. This:
- Reveals data shapes before backend investment
- Avoids speculative infrastructure
- Gets visible product sooner
- Constrains backend to actual UI needs

---

## Phase 0: Foundation

**Goal:** Tauri app launches, shows window, appears in system tray.

### Frontend
- Scaffold React + TypeScript (Vite)
- Install shadcn/ui (Button, Card)
- Configure Tailwind with DailyOS colors
- Create App shell with theme provider

### Backend (Rust)
- Scaffold Tauri 2.x project
- Configure window (size, title)
- Add system tray (icon, menu)
- Handle window close → minimize to tray

### Files
```
daybreak/
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── index.css
│   └── components/ui/
├── src-tauri/
│   ├── src/main.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/
├── package.json
├── vite.config.ts
├── tailwind.config.js
└── tsconfig.json
```

### Done When
- [x] `pnpm tauri dev` launches successfully
- [x] System tray icon visible
- [x] Click tray opens window
- [x] Window styled with DailyOS colors
- [x] Closing window hides (doesn't quit)
- [x] Theme toggle works

---

## Phase 1A: Static Dashboard

**Goal:** Dashboard UI renders with mock data. Looks like finished product.

### Frontend
- Dashboard layout component
- MeetingTimeline + MeetingCard components
- ActionList + ActionItem components
- Overview + StatsRow components
- Skeleton loading states
- Mock data module (realistic test data)

### Backend
None. All data hardcoded in frontend.

### Files
```
src/
├── components/
│   ├── Dashboard.tsx
│   ├── MeetingTimeline.tsx
│   ├── MeetingCard.tsx
│   ├── ActionList.tsx
│   ├── ActionItem.tsx
│   ├── Overview.tsx
│   └── StatsRow.tsx
├── lib/
│   └── mock-data.ts
└── types/
    └── index.ts
```

### Done When
- [x] Dashboard renders complete mock day
- [x] Meeting cards expand to show prep
- [x] Action list shows priority grouping
- [x] Hover states and animations work
- [x] Responsive to window resize
- [x] Indistinguishable from working product

---

## Phase 1B: File Reading

**Goal:** Dashboard reads actual `_today/` files from workspace.

### Frontend
- Replace mock data with Tauri commands
- useTauri hook for IPC
- Loading states (skeleton)
- Empty states ("No briefing yet")
- Error states

### Backend (Rust)
- State module (config persistence)
- `get_config` command
- `get_dashboard_data` command (JSON-only, DEC4)
- JSON loader (`json_loader.rs`) for `_today/data/*.json`
- Type definitions for dashboard, meetings, actions, emails

### Files
```
src/
├── hooks/
│   ├── useTauri.ts
│   ├── useConfig.ts
│   ├── useOverview.ts
│   └── useMeetings.ts
└── lib/
    └── types.ts

src-tauri/src/
├── state.rs
├── commands.rs
├── parser.rs
└── types.rs
```

### Done When
- [x] Dashboard reads from configured workspace
- [x] Config loaded from `~/.daybreak/config.json`
- [x] Content populated from `_today/` files
- [x] Empty state if `_today/` missing
- [x] Error state if workspace not configured

---

## Phase 1C: Scheduler + Executor

**Goal:** Briefing runs automatically, populates dashboard.

### Frontend
- Status indicator in tray (Idle/Running/Error)
- "Last run" timestamp
- "Run Briefing Now" button
- Refresh indicator
- Error display

### Backend (Rust)
- Scheduler module (cron-like jobs)
- Executor module (workflow runner)
- PTY manager (Claude Code subprocess)
- `run_workflow` command
- `get_workflow_status` command
- Native notifications
- Sleep/wake handling

### Files
```
src-tauri/src/
├── scheduler.rs
├── executor.rs
├── pty.rs
├── workflow/
│   ├── mod.rs
│   └── today.rs
└── notification.rs
```

### Done When
- [x] Scheduler runs jobs at configured times
- [x] `/today` workflow executes via Claude Code
- [x] Notification: "Your day is ready"
- [x] Dashboard auto-refreshes after workflow
- [x] "Run Now" triggers immediate execution
- [x] Status indicator works
- [x] Schedule survives restart
- [x] Missed jobs run on wake

### Implementation Notes (Phase 1C)
- Rust modules: `scheduler.rs`, `executor.rs`, `pty.rs`, `notification.rs`, `error.rs`, `workflow/`
- Frontend: `useWorkflow.ts` hook, `StatusIndicator.tsx`, `RunNowButton.tsx`
- Config extended with `schedules.today` and `schedules.archive` entries
- Execution history persisted to `~/.daybreak/execution_history.json`
- PTY-based Claude Code invocation via `portable-pty` crate
- Cron parsing via `cron` crate with timezone support (`chrono-tz`)
- Sleep/wake detection via time-jump polling (>5 min gap triggers missed job check)

---

## Phase 1D: Background Archive

**Goal:** Nightly archive runs automatically, zero interaction.

### Frontend
None.

### Backend (Rust)
- Archive workflow (move files)
- Schedule archive job (midnight)
- Handle empty days gracefully

### Files
```
src-tauri/src/workflow/
└── archive.rs
```

### Done When
- [x] Archive runs at midnight
- [x] Files moved to `archive/YYYY-MM-DD/`
- [x] Silent operation (no notification)
- [x] No error if nothing to archive

### Implementation Notes (Phase 1D)
- Pure Rust archive implementation (no Python scripts, no Claude Code)
- Separate execution path in `executor.rs` — skips three-phase pattern entirely
- Preserves `week-*.md` files (weekly view needs them)
- Archive schedule defaults to `0 0 * * *` (midnight daily)
- Graceful handling: empty `_today/`, missing `_today/`, already-archived day
- No `workflow-completed` event emitted (no dashboard refresh needed)
- No notification sent (truly silent operation)

---

## MVP Complete

**Success Criteria:**
- [x] Briefing runs automatically without user intervention
- [x] Notification appears when complete
- [x] Dashboard shows day ready (meetings, actions, overview)
- [x] Archive runs at midnight
- [x] No terminal required for daily workflow
- [ ] 7 crash-free days ← In validation period

**Status (2026-02-05):** MVP is functionally complete. Running 7-day stability validation.

---

## Phase 1.5: Nav & UI Refactor

**Goal:** Align the UI with the nav architecture decisions (DEC7-DEC13) before building Phase 2 features. Current sidebar has dead pages and missing pages.

This phase runs in parallel with Phase 2 Pre-work (data architecture). UI refactor is frontend; data architecture is backend. No dependency between them.

### 1.5a: Sidebar Simplification (DEC10, DEC11)

Remove pages that don't belong as standalone routes:

**Remove from sidebar:**
- Focus (becomes dashboard section)
- Week (post-MVP, add back when built)
- Emails (already on dashboard)

**Keep:**
- Dashboard
- Actions
- Settings

**Add:**
- Inbox
- Accounts (CS) / Projects (GA)

**Sidebar groups:** "Today" (Dashboard) + "Workspace" (Actions, Inbox, Accounts/Projects)

### Files Changed
```
src/components/layout/AppSidebar.tsx  ← Rewrite nav items, add profile-aware rendering
src/router.tsx                        ← Remove focus/week/emails routes
src/pages/FocusPage.tsx              ← Delete
src/pages/WeekPage.tsx               ← Delete (re-add in Phase 3C)
src/pages/EmailsPage.tsx             ← Delete
```

### Done When
- [x] Sidebar shows: Dashboard, Actions, Inbox, [Accounts/Projects], Settings
- [x] Focus, Week, Emails routes removed
- [x] Sidebar group labels: "Today" and "Workspace"
- [x] No dead routes (clicking removed nav items doesn't 404)

---

### 1.5b: Inbox Page

**Goal:** Basic page for viewing `_inbox/` contents. Prerequisite for Phase 2A (File Watcher).

### Frontend
- `InboxPage.tsx` — List files from `_inbox/` directory
- File list with name, size, modified date
- Empty state: "Inbox is clear" (positive framing)
- Manual refresh button

### Backend (Rust)
- `InboxFile` type in `types.rs`
- `list_inbox_files()` in `commands.rs` — Read `_inbox/` directory
- Return file metadata (name, size, modified timestamp)

### Files
```
src/pages/InboxPage.tsx              ← New
src/router.tsx                       ← Add /inbox route
src-tauri/src/types.rs               ← Add InboxFile type
src-tauri/src/commands.rs            ← Add get_inbox_files command
```

### Done When
- [x] `/inbox` route renders InboxPage
- [x] Lists all files in `_inbox/` from workspace
- [x] Empty state when no files
- [x] Sidebar Inbox item navigates correctly

---

### 1.5c: Profile-Aware Sidebar (DEC8)

**Goal:** Sidebar adapts to active profile. CS shows Accounts; GA shows Projects.

### Frontend
- Read profile from config (already loaded via `get_config`)
- Conditionally render third Workspace nav item based on profile
- Profile indicator text below app name in sidebar header

### Backend
- No changes (profile already in config)

### Files
```
src/components/layout/AppSidebar.tsx  ← Conditional nav item rendering
src/types/index.ts                    ← Add Profile type
```

### Done When
- [x] CS profile: sidebar shows "Accounts" nav item
- [x] GA profile: sidebar shows "Projects" nav item
- [x] Profile name displayed in sidebar header
- [x] Switching profile in config.json changes sidebar on next load

---

### 1.5d: Actions Page Refactor

**Goal:** Actions page backed by SQLite for interactive status updates. Depends on 2.0b (SQLite Setup).

### Frontend
- Filter bar: status (pending/completed/waiting), priority (P1/P2/P3)
- Account/project filter (profile-dependent)
- Mark complete / mark waiting toggles
- Source attribution on each action

### Backend (Rust)
- `get_actions` command (query SQLite)
- `update_action_status` command (mark complete/waiting)
- Filter params: status, priority, account_id/project_id

### Files
```
src/pages/ActionsPage.tsx            ← Rewrite with filters and interactive updates
src/hooks/useActions.ts              ← New hook for SQLite-backed actions
src-tauri/src/commands.rs            ← Add get_actions, update_action_status
src-tauri/src/db.rs                  ← SQLite connection (from 2.0b)
```

### Done When
- [x] Actions list loads from SQLite
- [x] Filter by status, priority, account/project
- [x] Mark action complete updates SQLite
- [x] Overdue items highlighted
- [x] Source attribution shown

**Dependency:** Requires 2.0b (SQLite Setup) to be complete first.

---

## Phase 2 Pre-work: Data Architecture

**Goal:** Establish the data infrastructure needed before Phase 2 features. Runs in parallel with Phase 1.5 (UI refactor).

### 2.0a: JSON-Primary Migration

Phase 1B implemented JSON loading. Phase 2 completes the migration:

1. Update Phase 3 Python (`deliver_today.py`) to generate JSON as source of truth
2. Generate markdown FROM JSON (optional human-readable view)
3. Remove markdown fallback from Rust parser

### 2.0b: SQLite Setup

Introduce `~/.dailyos/actions.db` as disposable cache (DEC18):

- `actions` table — Cross-day action tracking
- `meetings_history` table — Historical meeting lookup
- `accounts` table (CSM profile only)
- `projects` table (GA profile only)
- Rebuilt from workspace markdown files if corrupted

See `ACTIONS-SCHEMA.md` for full schema.

### 2.0c: Profile Selection

Implement profile system (DEC20):

- Profile selection during first-run setup
- CSM profile: Account-focused PARA, meeting classification with account cross-reference
- General profile: Project-focused PARA, no accounts concept
- Non-destructive switching (DEC9)

See `PROFILES.md` for full specification.

### 2.0d: Meeting Type Templates

Create Claude templates per meeting type (DEC21):

- Customer Call, QBR, Training, Internal Sync, 1:1, Partnership, All Hands
- Each template defines what Claude generates for that meeting type
- Profile-aware: CSM gets account context, General gets attendee context

See `MEETING-TYPES.md` for type definitions.

### 2.0e: Reference Approach for Directives

Update directive JSON to use file references instead of embedded content (DEC19):

- Directive contains file paths, not copied data
- Claude loads referenced files selectively during Phase 2
- Key metrics inline (ARR, ring, health), detail by reference

See `PREPARE-PHASE.md` for directive schema.

### 2.0f: Unknown Meeting Research

Implement proactive research for unknown external meetings (DEC22):

- Phase 1: Local search (grep archive for attendee/company mentions)
- Phase 2: Claude performs web research (company website, LinkedIn profiles)
- Output: Research brief even when no prior history exists

See `UNKNOWN-MEETING-RESEARCH.md` for research hierarchy.

### Done When
- [x] Phase 3 generates JSON as source of truth (2.0a)
- [x] SQLite actions.db created and populated (2.0b)
- [x] Profile selection UI in first-run dialog (2.0c)
- [x] Type-specific Claude templates for all meeting types (2.0d)
- [x] Directive uses reference approach (2.0e)
- [x] Unknown meeting research pipeline working (2.0f)

---

## Phase 2A: File Watcher

**Goal:** Detect new files in `_inbox/` automatically.

**Prerequisite:** Phase 1.5b (Inbox Page must exist to display watched files).

### Frontend
- Inbox badge (pending count) on sidebar nav item
- Tray icon badge
- Real-time file list updates on InboxPage

### Backend (Rust)
- Watcher module (monitor `_inbox/`)
- Debouncing (500ms)
- Filter to `.md` files
- Emit events to frontend (`inbox-updated`)

### Files
```
src-tauri/src/
└── watcher.rs

src/hooks/
└── useInbox.ts              ← Add event listener for real-time updates
src/components/layout/
└── AppSidebar.tsx           ← Add badge count to Inbox nav item
```

### Done When
- [x] New `.md` files detected within 30 seconds
- [x] Events debounced
- [x] Badge updates in real-time
- [x] InboxPage refreshes on new files

---

## Phase 2B: Quick Processing

**Goal:** Immediate classification and routing of simple files.

### Frontend
- ProcessingQueue component
- Queue panel in dashboard

### Backend (Rust)
- Quick processor (pattern-based)
- File routing (PARA locations)
- Tag files needing AI

### Files
```
src-tauri/src/processor/
├── mod.rs
├── classifier.rs
└── router.rs

src/components/
└── ProcessingQueue.tsx
```

### Done When
- [x] Simple files route within 5 seconds
- [x] Queue shows real-time status
- [x] Files tagged `.needs-enrichment` if AI required

---

## Phase 2C: Full Processing

**Goal:** AI enrichment runs on queued files periodically.

### Frontend
- "Review Needed" state
- Review flow for edge cases

### Backend (Rust)
- Batch processor
- `/inbox` workflow (three-phase)
- Schedule batch runs (every 2 hours)

### Files
```
src-tauri/src/workflow/
└── inbox.rs

src/components/
└── ReviewFlow.tsx
```

### Done When
- [x] Queued files batch-process on schedule
- [x] AI generates summaries, extracts actions
- [x] Files route to PARA locations
- [x] Review flow handles ambiguous cases

### Implementation Notes (Phase 2C)
- `InboxBatch` added as third workflow in scheduler alongside Today and Archive
- Default schedule: `0 */2 * * 1-5` (every 2 hours, weekdays)
- Execution path: direct `processor::process_all()` → `enrich::enrich_file()` for unknowns
- NOT a three-phase workflow — calls processor module directly from Rust
- Cap of 5 enrichments per batch (2 min AI timeout each = 10 min max per batch)
- Remaining files deferred to next batch run
- Emits `inbox-updated` Tauri event so frontend refreshes automatically
- Review handled inline on InboxPage (auto-escalates Unknown → AI enrich)
- Config: `schedules.inboxBatch` in `~/.daybreak/config.json`

### 2C+ Post-Enrichment Engine

**Goal:** After AI enrichment completes on a file, run mechanical updates to propagate intelligence through the workspace. This is the critical step that closes the compound intelligence loop (DEC26).

Post-enrichment is **not AI** — it's Rust code that takes structured AI output and updates downstream files. It runs as the final step of inbox processing, after Claude Code has generated summaries and extracted actions.

**Post-enrichment steps (extension-provided):**

| Step | Owner Extension | What It Does |
|------|----------------|--------------|
| Insert actions into SQLite | Core | Parse action items from enriched file, insert into `actions` table |
| Sync actions to account markdown | CS extension | Write actions to `Accounts/{Account}/04-Action-Items/` files |
| Update account dashboard | CS extension | Update `dashboard.json`: Last Contact, Recent Wins, Value Delivered, Next Actions (DEC28) |
| Update account index | CS extension | Append new files to `Accounts/{Account}/00-Index.md` Recent Activity section |
| Append to impact log | ProDev extension | Extract wins and append to weekly impact file |
| Bidirectional action sync | Core | When action marked complete in SQLite, update source markdown file (checkbox `[ ]` → `[x]`) |

**Architecture:**

```
AI Enrichment (Claude Code)
    ↓ produces: summary.md, actions.md (with structured frontmatter)
Post-Enrichment Engine (Rust)
    ↓ reads enriched files
    ├── Core: SQLite action insert
    ├── CS Extension: dashboard.json update, account index, action file sync
    └── ProDev Extension: impact log append
    ↓ emits: enrichment-complete event
Frontend refresh
```

**Key design principle:** Post-enrichment hooks are registered by extensions (DEC26). Core always runs its hooks (SQLite insert). Profile-activated extensions run theirs. The engine iterates registered hooks — it doesn't know about specific extensions.

**Files:**
```
src-tauri/src/
├── enrichment/
│   ├── mod.rs           # Post-enrichment engine, hook registry
│   ├── actions.rs       # Core: SQLite insert + bidirectional sync
│   └── hooks.rs         # Extension hook trait + registration
```

Extension-specific hooks live in extension modules (Phase 4), but the engine and core hooks ship in Phase 2C.

---

## Phase 3A: Calendar Polling

**Goal:** System knows when meetings start and end.

### Frontend
- Highlight current meeting in timeline

### Backend (Rust)
- Calendar poller (Google Calendar API)
- Track meeting state
- Emit meeting events

### Done When
- [ ] Calendar polled periodically
- [ ] Current meeting highlighted
- [ ] Meeting end time tracked

---

## Phase 3B: Post-Meeting Intelligence

**Goal:** After meetings end, the system automatically processes transcripts and enriches account intelligence. Manual capture is a lightweight fallback, not the primary path.

**Primary path (transcript exists):**
Meeting ends → user drops transcript in `_inbox/` → file watcher detects → Rust classifies as transcript for account → AI enrichment generates summary + extracts actions + infers wins/risks → post-enrichment engine (Phase 2C+) updates dashboard, syncs actions, appends impact log → zero user input required.

**Fallback path (no transcript):**
Meeting ends → configurable delay passes → no transcript detected in `_inbox/` → lightweight prompt: "Quick note about [Meeting]? Or skip — we'll process the transcript if one arrives later." → capture is optional, zero-guilt, ~10 seconds. If transcript arrives later, full processing happens automatically and supersedes the quick note.

### Frontend
- PostMeetingPrompt component (fallback overlay, bottom-right)
- Lightweight: meeting title + "Quick note?" + text field + [Save] [Skip]
- Auto-dismiss after 60 seconds
- Native notification if window hidden (click opens overlay)
- Manual trigger: "Outcomes" button on past MeetingCards (already built)

### Backend (Rust)
- Meeting end detection via calendar polling (Phase 3A)
- Transcript detection: watch `_inbox/` for files matching meeting account + date
- Prompt trigger: only if no transcript detected after delay
- Defer prompt if user is in another meeting
- Persist quick notes to SQLite `captures` table
- Quick notes feed into post-enrichment engine (same as transcript path)

### Files
```
src/components/
└── PostMeetingPrompt.tsx      # Lightweight fallback overlay

src-tauri/src/
└── capture.rs                 # Meeting end detection + transcript watching + prompt logic
```

### Done When
- [ ] Transcripts in `_inbox/` auto-process through enrichment pipeline
- [ ] Post-enrichment updates account dashboard, syncs actions, appends impact
- [ ] Fallback prompt only appears when no transcript detected
- [ ] Prompt is dismissible without guilt
- [ ] Quick note capture in under 10 seconds
- [ ] Manual "Outcomes" button on past meeting cards triggers prompt
- [ ] Native notification when window hidden

---

## Phase 3C: Weekly Planning

**Goal:** Interactive Monday planning flow.

### Frontend
- Planning wizard (multi-step)
- Priority selection UI
- Week overview visualization
- Focus blocks selector

### Backend (Rust)
- Prepare week data (Sunday/Monday)
- `/week` workflow
- Handle timeout (defaults)

### Files
```
src/components/WeeklyPlanning/
├── index.tsx
├── PriorityPicker.tsx
├── WeekOverview.tsx
└── FocusBlocks.tsx

src-tauri/src/workflow/
└── week.rs
```

### Done When
- [ ] Prompt appears Monday when app opens
- [ ] Priority selection is visual
- [ ] Flow completes in under 2 minutes
- [ ] Skipping uses sensible defaults

---

## Dependency Graph

```
Phase 0 (Foundation)
    └── Phase 1A (Static Dashboard)
            └── Phase 1B (File Reading)
                    └── Phase 1C (Scheduler + Executor)
                            └── Phase 1D (Archive)
                                    │
                              ══════╪═══════ MVP COMPLETE ══════════
                                    │
                    ┌───────────────┼───────────────┐
                    │ (parallel)    │               │
                    ▼               ▼               │
           Phase 1.5         Phase 2 Pre-work       │
           (Nav Refactor)    (Data Architecture)    │
              │                    │                │
              ├── 1.5a Sidebar     ├── 2.0a JSON    │
              ├── 1.5b Inbox Page  ├── 2.0b SQLite ─┤
              ├── 1.5c Profile Nav ├── 2.0c Profile │
              └── 1.5d Actions ────┘  2.0d Templates│
                    │                 2.0e Refs      │
                    │                 2.0f Research   │
                    ▼                                │
              Phase 2A (File Watcher)               │
                    │    (needs 1.5b Inbox Page)     │
                    └── Phase 2B (Quick Processing)  │
                            └── Phase 2C (Full)      │
                                                     │
                              Phase 3A (Calendar Polling)
                                    ├── Phase 3B (Post-Meeting Intelligence)
                                    │        ↑ depends on 2C+ (Post-Enrichment Engine)
                                    └── Phase 3C (Weekly Planning + Week Page)
```

**Key dependencies:**
- 1.5a-c are independent and can be done in any order
- 1.5d (Actions refactor) depends on 2.0b (SQLite)
- 2A (File Watcher) depends on 1.5b (Inbox Page)
- Phase 1.5 and Phase 2 Pre-work run in parallel

---

## Risks

| Risk | Mitigation |
|------|------------|
| Claude Code PTY issues | Retry logic, test on different machines |
| Google API token expiry | Detect early, prompt re-auth |
| File watcher unreliability | Periodic polling backup |
| Scheduler drift on sleep | Re-sync on wake events |

---

## Open Decisions

1. ~~**Onboarding:** Add minimal setup in Phase 1C, or document manual config?~~ → Manual config for MVP
2. **Error UX:** Modal vs banner vs tray notification? → Currently using tray notifications
3. ~~**Config UI:** Edit JSON acceptable through MVP?~~ → Yes, JSON editing is acceptable

## Decisions Made

See `RAIDD.md` for the canonical decision log (DEC1-DEC28).

---

*Document Version: 1.3*
*Last Updated: 2026-02-05 — Added post-enrichment engine (2C+), revised Phase 3B to post-meeting intelligence, added extension hook architecture*
