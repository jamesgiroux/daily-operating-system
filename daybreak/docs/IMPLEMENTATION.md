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
- `get_today_overview` command
- `get_meetings` command
- `get_actions` command
- Markdown parser (frontmatter + content)

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

## Phase 2A: File Watcher

**Goal:** Detect new files in `_inbox/` automatically.

### Frontend
- Inbox indicator (pending count badge)
- Tray icon badge

### Backend (Rust)
- Watcher module (monitor `_inbox/`)
- Debouncing (500ms)
- Filter to `.md` files
- Emit events to frontend

### Files
```
src-tauri/src/
└── watcher.rs

src/hooks/
└── useInbox.ts
```

### Done When
- [ ] New `.md` files detected within 30 seconds
- [ ] Events debounced
- [ ] Badge updates in real-time

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
- [ ] Simple files route within 5 seconds
- [ ] Queue shows real-time status
- [ ] Files tagged `.needs-enrichment` if AI required

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
- [ ] Queued files batch-process on schedule
- [ ] AI generates summaries, extracts actions
- [ ] Files route to PARA locations
- [ ] Review flow handles ambiguous cases

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

## Phase 3B: Post-Meeting Capture

**Goal:** Prompt appears after meetings for quick outcome capture.

### Frontend
- PostMeetingPrompt component
- CaptureModal (Win/Risk/Action)
- Easy dismissal

### Backend (Rust)
- Trigger prompt on meeting end (5 min delay)
- Persist captured data
- Filter to customer meetings only

### Files
```
src/components/
├── PostMeetingPrompt.tsx
├── CaptureModal.tsx
└── CaptureInput.tsx

src-tauri/src/
└── capture.rs
```

### Done When
- [ ] Prompt appears 5 minutes after meeting
- [ ] Only for customer meetings
- [ ] Capture in under 10 seconds
- [ ] Skip is prominent and guilt-free

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
                                    ├── Phase 2A (File Watcher)
                                    │       └── Phase 2B (Quick Processing)
                                    │               └── Phase 2C (Full Processing)
                                    │
                                    └── Phase 3A (Calendar Polling)
                                            ├── Phase 3B (Post-Meeting Capture)
                                            └── Phase 3C (Weekly Planning)
```

Phases 2 and 3 can run in parallel after MVP.

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

| Decision | Rationale |
|----------|-----------|
| Pure Rust archive | No AI needed; simpler, faster than three-phase |
| Sidebar collapsed by default | Better first impression, more breathing room |
| Window 1180px default | Comfortable layout without immediate resize |
| SQLite in Phase 2 | JSON fine for MVP; need SQLite for queue state |

---

*Document Version: 1.1*
*Last Updated: 2026-02-05*
