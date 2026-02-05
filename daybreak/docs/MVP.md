# MVP Scope Definition

> **One sentence:** MVP delivers the core promise—"Open the app. Your day is ready."

---

## MVP Scope Declaration

**MVP = F1 (Morning Briefing) + F7 (Dashboard) + F6 (System Tray) + F3 (Background Automation)**

That's it. Four features that prove the value proposition:

1. The system prepares your day automatically
2. You open the app and consume it
3. The app lives in your system tray
4. Housekeeping happens invisibly

---

## In Scope (MVP)

| Feature | What It Does | Why MVP |
|---------|--------------|---------|
| **F1: Morning Briefing** | Runs at scheduled time, generates `_today/` files | Core value prop—the system operates |
| **F7: Dashboard** | Renders `_today/` as polished UI | Consumption interface—buttons not files |
| **F6: System Tray** | App presence, notifications, quick actions | Native app basics—always available |
| **F3: Background Automation** | Nightly archive, cleanup | Invisible housekeeping—zero maintenance |

### MVP User Story

> As a knowledge worker, I want to open DailyOS in the morning and see my day already prepared, so I can review my meetings and actions without running commands or reading markdown files.

### MVP Happy Path

```
6:00 AM  — Scheduler triggers F1 (Morning Briefing)
         — Python: prepare_today.py runs
         — Claude: AI enrichment completes
         — Python: deliver_today.py writes _today/*.md
         — Notification: "Your day is ready"

8:00 AM  — User clicks system tray icon
         — Dashboard opens, renders _today/ files
         — User sees: overview, meeting cards, actions
         — User reads, reviews (2-5 minutes)
         — User minimizes app, starts work

Midnight — F3 archives _today/ to archive/YYYY-MM-DD/
         — Clean slate for tomorrow
```

---

## Out of Scope (Post-MVP)

| Feature | What It Does | Why Deferred |
|---------|--------------|--------------|
| **F4: Processing Queue** | Active inbox with file watching | Complex—two-tier processing, needs solid foundation first |
| **F2: Post-Meeting Capture** | Event-driven prompts after meetings | Requires calendar polling, prompt UX—adds complexity |
| **F5: Weekly Planning** | Interactive Monday planning flow | Interactive multi-step—build after passive consumption works |
| Manual refresh button | Re-run briefing on demand | Nice-to-have, not essential for MVP |
| Preferences UI | Configure schedules, integrations | Can use config file for MVP |

### Explicitly NOT in MVP

- No file watching (inbox processing is manual via CLI)
- No post-meeting prompts
- No weekly planning flow
- No preferences UI (edit config.json directly)
- No multi-workspace support
- No onboarding wizard (document manual setup)

---

## MVP Success Criteria

### Quantitative

| Metric | Target | Status |
|--------|--------|--------|
| Briefing success rate | 95%+ | ✅ Tracking |
| Briefing completion time | < 3 minutes | ✅ Achieved |
| Dashboard load time | < 1 second | ✅ Achieved |
| Crash-free days | 7+ consecutive | ⏳ In progress |

### Qualitative

| Outcome | Status |
|---------|--------|
| "Day is ready" feeling | ✅ Validated — briefing runs overnight, ready at 8am |
| No CLI required | ✅ Validated — daily workflow via app only |
| Trust in automation | ✅ Validated — user didn't manually check |

### MVP Kill Criteria

**Stop and reconsider if:**

- Briefing fails > 20% of the time after 1 week
- Users still open CLI to run `/today` manually
- Dashboard feels slower than reading markdown directly
- Google API integration is too fragile for daily use

---

## MVP Technical Scope

### Must Build

| Component | Description |
|-----------|-------------|
| Tauri app shell | Window management, system tray, IPC bridge |
| Scheduler | Cron-like job scheduling with timezone support |
| Workflow executor | Three-phase orchestration (Python → Claude → Python) |
| Dashboard UI | React components for overview, meeting cards, actions |
| State manager | Config persistence, workflow status tracking |
| PTY manager | Claude Code subprocess spawning |

### Can Reuse from DailyOS

| Component | Location | Status |
|-----------|----------|--------|
| `prepare_today.py` | `templates/scripts/` | ✅ Working |
| `deliver_today.py` | `templates/scripts/` | ✅ Working |
| `/today` command template | `templates/commands/` | ✅ Working |
| Google API integration | Workspace `.config/` | ✅ Working |
| Three-phase directive pattern | Documented in ARCHITECTURE.md | ✅ Proven |

### Must Defer

- File watcher (notify crate integration)
- Calendar polling for meeting detection
- Preferences UI
- Onboarding wizard

---

## MVP Assumptions

1. **User has DailyOS workspace configured** — MVP doesn't include workspace setup wizard
2. **Google API tokens exist** — MVP assumes `dailyos google-setup` was already run
3. **Claude Code is installed** — MVP doesn't install Claude Code
4. **Single workspace** — MVP doesn't support multiple workspaces

---

## Relationship to Other Docs

| Document | How MVP.md Relates |
|----------|-------------------|
| PRD.md | MVP.md scopes PRD features into phases |
| ROADMAP.md | MVP.md defines Phase 1; ROADMAP defines all phases |
| ARCHITECTURE.md | MVP.md identifies which components to build first |

---

*This is a living document. Update when scope changes. Any scope expansion requires explicit decision.*
