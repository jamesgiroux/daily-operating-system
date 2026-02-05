# DailyOS Roadmap

> From proof-of-concept to product in three phases.

---

## Current Status (2026-02-05)

**Phase 1 is functionally complete.** The core promise works:
- Briefing runs automatically at scheduled time
- Dashboard shows the day ready
- Archive runs silently at midnight
- No terminal required for daily workflow

**Remaining:** 7-day crash-free validation period.

---

## Phase Overview

| Phase | Name | Core Delivery | Key Features |
|-------|------|---------------|--------------|
| **Phase 1** | "Your day is ready" | Passive consumption | F1 Briefing, F7 Dashboard, F6 Tray, F3 Archive |
| **Phase 2** | "Active processing" | Inbox automation | F4 Processing Queue |
| **Phase 3** | "Intelligent prompts" | Event-driven UX | F2 Post-Meeting, F5 Weekly Planning |

---

## Dependency Graph

```mermaid
flowchart TB
    subgraph Foundation["FOUNDATION (Build First)"]
        Shell["Tauri App Shell<br/>Window, Tray, IPC"]
        State["State Manager<br/>Config, Status"]
        PTY["PTY Manager<br/>Claude Code spawn"]
    end

    subgraph Phase1["PHASE 1: Your Day Is Ready"]
        Scheduler["Scheduler<br/>Cron jobs, timers"]
        Executor["Workflow Executor<br/>Three-phase orchestration"]
        Dashboard["Dashboard UI<br/>Overview, Cards, Actions"]
        Archive["Background Archive<br/>Nightly cleanup"]

        Shell --> Scheduler
        Shell --> Dashboard
        State --> Scheduler
        State --> Executor
        PTY --> Executor
        Scheduler --> Executor
        Executor --> Dashboard
        Scheduler --> Archive
    end

    subgraph Phase2["PHASE 2: Active Processing"]
        Watcher["File Watcher<br/>_inbox/ monitoring"]
        Queue["Processing Queue<br/>Quick + Full processing"]
        QueueUI["Queue UI Panel<br/>Status, Review flow"]

        Shell --> Watcher
        Watcher --> Queue
        Executor --> Queue
        Queue --> QueueUI
        Dashboard --> QueueUI
    end

    subgraph Phase3["PHASE 3: Intelligent Prompts"]
        Calendar["Calendar Polling<br/>Meeting detection"]
        PostMeeting["Post-Meeting Capture<br/>Event-driven prompts"]
        WeeklyPlan["Weekly Planning<br/>Interactive flow"]

        Scheduler --> Calendar
        Calendar --> PostMeeting
        Dashboard --> PostMeeting
        Scheduler --> WeeklyPlan
        Dashboard --> WeeklyPlan
    end

    Phase1 --> Phase2 --> Phase3
```

---

## Phase 1: "Your Day Is Ready"

**Goal:** Prove the core value proposition—the system prepares, you consume.

### Milestones

| Milestone | Deliverable | Definition of Done |
|-----------|-------------|-------------------|
| **1.1 App Shell** | Tauri app with system tray | App launches, shows in tray, opens window |
| **1.2 Scheduler** | Time-based job execution | Jobs run at configured times, survive restart |
| **1.3 Executor** | Three-phase workflow orchestration | `/today` runs via DailyOS, produces `_today/` files |
| **1.4 Dashboard** | UI renders briefing content | Overview, meeting cards, action list display correctly |
| **1.5 Integration** | End-to-end happy path | Briefing runs at 8am, notification appears, dashboard shows content |
| **1.6 Archive** | Background cleanup | Nightly archive runs, old files moved |

### Phase 1 Acceptance Criteria

- [x] Briefing runs automatically at scheduled time
- [x] Notification appears when briefing completes
- [x] Clicking tray icon opens dashboard
- [x] Dashboard renders `_today/` content as UI (not markdown)
- [x] Meeting cards show prep content (expandable)
- [x] Action list shows due/overdue items
- [ ] No crashes over 7-day test period
- [x] User doesn't need terminal for daily workflow

### What's NOT in Phase 1

- File watching (no inbox processing)
- Post-meeting prompts
- Weekly planning flow
- Preferences UI
- Onboarding wizard

---

## Phase 2: "Active Processing"

**Goal:** Automate the inbox—files process without commands.

### Prerequisites

- Phase 1 complete and stable
- Executor proven reliable
- Dashboard rendering solid

### Milestones

| Milestone | Deliverable | Definition of Done |
|-----------|-------------|-------------------|
| **2.1 File Watcher** | Monitor `_inbox/` directory | Detects new `.md` files within 30 seconds |
| **2.2 Quick Processing** | Immediate classification | Files classified and routed in < 5 seconds |
| **2.3 Full Processing** | Batched AI enrichment | Queued files processed every 2 hours |
| **2.4 Queue UI** | Processing status panel | Shows pending, processing, completed, review states |
| **2.5 Review Flow** | Handle edge cases | User can resolve ambiguous files |

### Phase 2 Acceptance Criteria

- [ ] New files in `_inbox/` detected automatically
- [ ] Simple files routed immediately (no AI needed)
- [ ] Complex files queued and batch-processed
- [ ] Queue panel shows real-time status
- [ ] "Review needed" items have clear resolution path
- [ ] User doesn't run `/inbox` command manually

---

## Phase 3: "Intelligent Prompts"

**Goal:** Context-aware interactions at natural moments.

### Prerequisites

- Phase 2 complete and stable
- File watcher reliable
- Dashboard interaction patterns established

### Milestones

| Milestone | Deliverable | Definition of Done |
|-----------|-------------|-------------------|
| **3.1 Calendar Polling** | Meeting awareness | Knows when meetings start/end |
| **3.2 Post-Meeting Capture** | Event-driven prompt | Prompt appears 5 min after meeting ends |
| **3.3 Capture UI** | Quick outcome entry | Win/Risk/Action capture in < 10 seconds |
| **3.4 Weekly Prep** | Monday data preparation | Week context ready by Monday 8am |
| **3.5 Weekly Planning UI** | Interactive flow | Priority setting, week overview, focus blocks |

### Phase 3 Acceptance Criteria

- [ ] System knows when meetings end
- [ ] Post-meeting prompt appears at appropriate time
- [ ] Prompt is dismissible without guilt
- [ ] Captured outcomes persist correctly
- [ ] Weekly planning flow completes in < 2 minutes
- [ ] Skipping weekly planning has sensible defaults

---

## Technical Milestones (Cross-Phase)

### Foundation (Before Phase 1)

| Task | Description |
|------|-------------|
| Project scaffolding | Tauri + React + TypeScript setup |
| Build pipeline | Dev server, production build |
| IPC bridge | Rust ↔ TypeScript communication |
| Config schema | `~/.daybreak/config.json` structure |

### Infrastructure (During Phase 1)

| Task | Description |
|------|-------------|
| Error handling | Retry logic, user-facing errors |
| Logging | Structured logs for debugging |
| State persistence | Workflow status survives restart |

### Polish (Post-Phase 3)

| Task | Description |
|------|-------------|
| Preferences UI | Visual config editing |
| Onboarding wizard | First-run setup flow |
| Auto-update | In-app update mechanism |
| Telemetry (opt-in) | Usage analytics |

---

## Risk Mitigation by Phase

### Phase 1 Risks

| Risk | Mitigation |
|------|------------|
| PTY spawn failures | Retry logic, queue mechanism, clear error states |
| Claude Code timeout | Configurable timeout, partial result handling |
| Google API token expiry | Detect and prompt for re-auth |
| Scheduler drift on sleep | Re-sync on wake events |

### Phase 2 Risks

| Risk | Mitigation |
|------|------------|
| File watcher misses events | Periodic polling backup |
| Quick processing misclassifies | "Review needed" escape hatch |
| Batch processing too slow | Prioritize by file age |

### Phase 3 Risks

| Risk | Mitigation |
|------|------------|
| Post-meeting prompt fatigue | Easy disable, smart filtering (external meetings only) |
| Calendar API rate limits | Cache calendar data, poll less frequently |
| Weekly planning abandonment | Timeout with sensible defaults |

---

## Decision Log

See `RAIDD.md` for the canonical decision log (DEC1-DEC23).

---

## Open Questions

1. ~~**Onboarding:** Should Phase 1 include minimal onboarding, or document manual setup?~~ → Manual setup for MVP, wizard in Phase 2+
2. ~~**Preferences:** Config file editing acceptable for MVP, or need basic UI?~~ → Config file acceptable for MVP
3. **Google API:** Require for MVP, or graceful degradation with manual calendar?
4. **Testing:** What's the testing strategy for Rust backend? (Unit tests exist for archive, scheduler)

---

*Update this roadmap as phases complete and learnings emerge.*
