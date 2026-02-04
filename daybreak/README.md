# Daybreak

> The native app for DailyOS. Consume, don't produce.

**Status:** Exploration / Pre-Alpha

---

## Vision

Daybreak is the "iOS" version of DailyOS—a native desktop application that lets knowledge workers consume AI-generated productivity insights without touching a terminal.

**Core principle:** The AI works for you, invisibly. You read beautiful output. You get on with your day.

### What Daybreak Is

- A **native app** (Tauri) that opens like any other app
- A **consumption-first interface**—read your day, don't configure it
- **Markdown files as truth**—same PARA structure, same files
- **Claude Code as engine**—runs in background, you never see it

### What Daybreak Is Not

- Not a Claude Code replacement (power users keep terminal access)
- Not a cloud service (local-first, always)
- Not a general-purpose note app (opinionated for daily productivity)

---

## User Experience

### Morning

1. Open Daybreak (app icon in dock)
2. Today overview already there—generated at 6am
3. Calendar, priorities, prep notes, action items
4. Beautifully rendered. You read it with coffee.
5. Click "Ready" and minimize

### During the Day

1. Notification: "Meeting with Acme Corp in 30 min"
2. Click → meeting prep appears
3. After meeting, drop transcript in folder
4. App processes it automatically

### Evening

1. Notification: "Ready to wrap up?"
2. Click → wins, completed items, tomorrow preview
3. Scan it, click "Done"
4. App archives, you close laptop

**You never:**
- Open a terminal
- Type a command
- See markdown syntax
- Think about Claude Code

---

## Technical Architecture

```
┌─────────────────────────────────────────┐
│           Daybreak App (Tauri)          │
├─────────────────────────────────────────┤
│  Consumption Layer                      │
│  - Today overview                       │
│  - Account/project views                │
│  - Meeting prep cards                   │
│  - Action item lists                    │
├─────────────────────────────────────────┤
│  Editing Layer                          │
│  - Inline markdown editor               │
│  - Quick capture                        │
│  - Task toggles                         │
├─────────────────────────────────────────┤
│  AI Layer (hidden by default)           │
│  - Claude Code subprocess               │
│  - Scheduled runs (launchd/cron)        │
│  - Background processing                │
└─────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────┐
│     File System (unchanged)             │
│  _today/, Accounts/, .claude/skills/    │
└─────────────────────────────────────────┘
```

### Tech Stack

| Layer | Technology | Why |
|-------|------------|-----|
| App shell | [Tauri](https://tauri.app/) | Lightweight (~10MB), cross-platform, Rust backend |
| Frontend | HTML/CSS/JS or React | Flexible, can start simple |
| Markdown | CodeMirror or Monaco | Inline editing when needed |
| Backend | Rust (Tauri) + Python (Claude Code) | Tauri for app, Python for AI |
| Scheduling | launchd (macOS) / Task Scheduler (Win) | Native OS scheduling |

---

## Roadmap

### Phase 1: Proof of Concept (4 weeks)
- [ ] Basic Tauri app that launches
- [ ] Single view: render `_today/00-overview.md` beautifully
- [ ] "Refresh" button that invokes Claude Code in background
- [ ] File watching: UI updates when files change
- [ ] Ship to 5-10 beta testers

### Phase 2: Core Experience (8 weeks)
- [ ] Navigation: Today, Accounts, Projects, Areas
- [ ] Inline editing (click to edit, auto-save)
- [ ] Scheduled execution (morning /today, evening /wrap prompt)
- [ ] System notifications
- [ ] Settings panel (workspace path, schedule times)

### Phase 3: Polish (ongoing)
- [ ] Keyboard shortcuts
- [ ] Search (with index, not O(N) scan)
- [ ] Themes (light/dark, accent colors)
- [ ] "Power user" mode (show terminal pane)
- [ ] Windows/Linux support

---

## Relationship to DailyOS

```
dailyos-skills (repo)          DailyOS (current repo)
     │                              │
     │  Claude Code users           │  Power users who want both
     │  install skills directly     │  terminal and app
     │                              │
     └──────────┬───────────────────┘
                │
                ▼
         Daybreak (this)
              │
              │  Knowledge workers
              │  who want outcomes,
              │  not tools
              │
              ▼
         Mass market
```

Daybreak uses the same skills/agents/file structure. It's a different interface, not a different product.

---

## Open Questions

1. **Electron vs Tauri?** Tauri is lighter but newer. Electron has more ecosystem.
2. **React vs vanilla JS?** Start simple or invest in component architecture?
3. **How to handle Claude Code auth?** App needs to invoke `claude` CLI.
4. **Scheduling mechanism?** OS-native (launchd) or app-managed?
5. **Offline mode?** What happens when Claude Code can't reach API?

---

## Getting Started (for contributors)

```bash
# Prerequisites
# - Rust: https://rustup.rs/
# - Node.js 18+
# - Tauri CLI: cargo install tauri-cli

# Setup
cd daybreak
npm install
cargo tauri dev
```

---

## Philosophy

From the DailyOS README:

> **Consuming, not producing.** You shouldn't have to maintain your productivity tools. They should just be productive.

Daybreak is this philosophy made tangible. The app does the work. You reap the benefits.

---

*Code name: Daybreak — the moment the day begins.*
