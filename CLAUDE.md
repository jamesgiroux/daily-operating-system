# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

---

## Working Style: Challenge Me

**Do not just execute requests.** Push back when something doesn't fit best practice. Your job is to:

- **Challenge scope creep** — If I ask for something that doesn't belong in the file/feature I'm pointing at, say so
- **Enforce separation of concerns** — Decisions are ADRs, issues are backlog, architecture is code. Don't let me conflate them.
- **Red-team my thinking** — If I'm proposing something that contradicts stated principles or creates maintenance burden, call it out
- **Ask clarifying questions** — Don't assume. If the request is ambiguous, probe before executing.
- **Suggest alternatives** — If there's a better way, propose it even if I didn't ask

I'd rather have you tell me I'm wrong than silently build the wrong thing. Disagree respectfully, explain your reasoning, and let me decide — but don't just comply.

---

## Documentation Discipline

**Code is the source of truth for how things work. Docs are the source of truth for why decisions were made.**

### Rules

1. **Types are the data model.** `src/types/index.ts` and `src-tauri/src/types.rs` define the data shapes. If someone asks "what does a Meeting look like?" — read the type definition, not a doc. Never create a separate data model document.

2. **ADRs are append-only.** Decisions live in `daybreak/docs/decisions/`. Once accepted, never edit an ADR. If the decision changes, create a new ADR that supersedes it. See the [ADR README](daybreak/docs/decisions/README.md) for template and index.

3. **The backlog is the backlog.** Issues, risks, and known problems live in `daybreak/docs/BACKLOG.md`. Don't scatter "Open Questions" across multiple docs. If something is unresolved, it's a backlog item.

4. **Don't document what the code already says.** If a function's behavior is clear from reading it, don't add a doc describing that behavior. If a component's props are typed, don't maintain a separate "component API" doc.

5. **Design docs are disposable.** When planning a feature: write a short design doc or ADR → build it → the code becomes the spec → archive the design doc. Don't maintain living spec docs that describe implemented code.

6. **Docs should shrink over time.** As features ship, move detail from PRD.md into the code (types, tests, behavior). The PRD should eventually describe only unbuilt features.

### When to Write an ADR

Write one when:
- Choosing between technologies or approaches
- Deciding how data flows through the system
- Establishing a pattern that future code will follow
- Any decision you'd want to explain to your future self

Don't write one for: bug fixes, UI tweaks, refactoring that preserves behavior.

### When I Ask You to Write Docs

Push back if:
- I'm writing a spec for something that already exists in code — suggest reading the code instead
- I'm adding "open questions" to a design doc — suggest creating backlog items instead
- I'm creating a new document that overlaps with an existing one — point to the existing doc
- I'm documenting resolved decisions in a design doc — suggest writing an ADR
- I'm pre-writing comprehensive specs before building — suggest building first, documenting the decision after

**Example:** If I say "let's document the email classification rules," respond with: "Those rules are in `prepare_today.py:classify_email()`. Should I read the code and explain it, or are you proposing changes? If changes, let's write an ADR."

### Anti-Patterns to Enforce

| Anti-Pattern | What to Do Instead |
|--------------|-------------------|
| New doc describing existing code | Point to the code |
| "Open questions" in docs | Create backlog items |
| Spec that mirrors implemented types | Delete the spec, read the types |
| Living doc tracking completed work | Archive it |
| Assessment/audit docs as living files | Extract actions to backlog, archive the assessment |
| Cross-referencing docs that say the same thing | Consolidate to one source |

---

## Specialist Skills

Call these skills when their expertise is needed:

| Skill | When to Use |
|-------|-------------|
| `/ux` | Designing interfaces, reviewing user flows, evaluating interactions |
| `/red-team` | Stress-testing ideas, challenging assumptions, poking holes |
| `/pm` | Defining features, challenging requirements, validating product decisions |
| `/arch` | Designing systems, evaluating technical approaches, reviewing architecture |
| `/eng` | Writing code, implementing features, debugging, reviewing implementations |

**Use proactively.** When discussing UI, invoke `/ux`. When proposing features, invoke `/pm`. When the idea needs holes poked, invoke `/red-team`.

---

## Project Context

**DailyOS** is a native desktop app (Tauri v2) for AI-native daily productivity. **Daybreak** is the codename for the native app rewrite on the `feature/daybreak` branch.

The core promise: **"Open the app. Your day is ready."**

### Philosophy & Principles

Read these before making any design decisions:
- `daybreak/docs/PHILOSOPHY.md` — The manifesto (why we're building this)
- `daybreak/docs/PRINCIPLES.md` — 10 design principles + decision framework
- `daybreak/docs/VISION.md` — Product vision and user experience

### The Prime Directive

> **The system operates. You leverage.**

If a feature requires the user to maintain it, it's wrong. If skipping a day creates debt, it's wrong.

### The 10 Design Principles

| # | Principle | Quick Test |
|---|-----------|------------|
| 1 | **Zero-Guilt by Default** | What happens if user skips a week? (Nothing should break) |
| 2 | **Prepared, Not Empty** | Is default state "ready" or "waiting"? (Must be ready) |
| 3 | **Buttons, Not Commands** | Can non-technical user do this? (One-click > command) |
| 4 | **Opinionated Defaults, Escapable Constraints** | Works out-of-box but overridable? |
| 5 | **Local-First, Always** | Data on user's machine in files they own? |
| 6 | **AI-Native, Not AI-Assisted** | AI is the engine, not a feature? |
| 7 | **Consumption Over Production** | 80% reading, 20% writing? |
| 8 | **Forgiveness Built In** | System recovers gracefully from neglect? |
| 9 | **Show the Work, Hide the Plumbing** | Users see outputs, not processes? |
| 10 | **Outcomes Over Activity** | Measuring effectiveness, not engagement? |

### Anti-Patterns to Avoid

| Anti-Pattern | Why Wrong |
|--------------|-----------|
| Streak counters | Creates guilt |
| Unread counts | Implies obligation |
| "You haven't..." notifications | Shames non-usage |
| Empty states requiring setup | Demands labor before value |
| Required daily actions | Creates debt when skipped |
| Proprietary formats | Hostage data |
| Cloud-required features | Dependency on infrastructure |

---

## Documentation Structure

```
daybreak/docs/
├── PHILOSOPHY.md          # The manifesto — why we're building this
├── PRINCIPLES.md          # 10 design principles + decision framework
├── VISION.md              # Product vision and user experience
├── JTBD.md                # Jobs to be done — user needs
├── PRD.md                 # Product requirements (shrinks as features ship)
├── ARCHITECTURE.md        # Technical architecture
├── ROADMAP.md             # Phase overview and milestones
├── BACKLOG.md             # Issues, risks, assumptions, dependencies
├── UI-SPEC.md             # Design system, colors, components
├── DEVELOPMENT.md         # Build and run instructions
├── SKILLS.md              # Command catalog
├── MEETING-TYPES.md       # Classification algorithm reference
├── PREPARE-PHASE.md       # Prepare phase architecture reference
├── PROFILES.md            # Profile system reference
├── ACTIONS-SCHEMA.md      # SQLite schema reference
├── decisions/             # Architecture Decision Records (append-only)
│   ├── README.md          # Index + template
│   └── 0001-*.md … 0034-*.md
├── research/              # Market research, user research (reference)
└── _archive/              # Retired docs (historical reference only)
```

---

## Feature Development Flow

When building a new feature:

1. **Identify the job** — Which JTBD.md job does this serve? If none, challenge whether it should exist.
2. **Check for an ADR** — Has this approach been decided? If not, write one. If the decision is pending (status: Proposed), resolve it first.
3. **Build it** — Code is the spec. Types define the data model. Tests verify the behavior.
4. **Update the backlog** — Close any issues this resolves. Note any new issues discovered.
5. **Write an ADR if needed** — If you made a significant decision during implementation, record it.
6. **Shrink the docs** — If PRD.md described this feature in detail, trim it. The code is the truth now.

**Don't:** Write a comprehensive spec before building. Don't create a design doc you'll need to maintain. Don't add the feature to three different documents.

---

## Development Commands

### Daybreak (Native App)

```bash
# Prerequisites: Rust 1.70+, Node.js 18+, pnpm 8+, Tauri CLI

# Development (hot reload)
pnpm install
pnpm tauri dev

# Build for production
pnpm tauri build

# Tests
pnpm test                      # Frontend tests
cd src-tauri && cargo test     # Rust backend tests
```

### DailyOS CLI (Proof-of-Concept)

```bash
python3 -m pytest tests/ -v   # Run tests
dailyos doctor                 # Check workspace health
dailyos google-setup           # Configure Google API
```

---

## App Structure

The Tauri app lives at the **repo root**, not under `daybreak/`. `daybreak/docs/` contains design documentation only.

```
./                           # Tauri app root
├── src/                     # Frontend (React + TypeScript)
│   ├── App.tsx
│   ├── router.tsx           # TanStack Router
│   ├── components/          # UI components
│   │   ├── dashboard/       # Dashboard-specific
│   │   ├── layout/          # AppSidebar, CommandMenu
│   │   └── ui/              # shadcn/ui primitives
│   ├── hooks/               # useDashboardData, useWorkflow
│   ├── pages/               # ActionsPage, InboxPage, SettingsPage
│   ├── types/               # TypeScript type definitions (THE data model)
│   └── lib/                 # Utilities
│
├── src-tauri/               # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs           # Tauri builder, handler registration
│   │   ├── commands.rs      # Tauri IPC commands
│   │   ├── types.rs         # Rust type definitions (THE data model)
│   │   ├── json_loader.rs   # JSON data loading
│   │   ├── scheduler.rs     # Cron-like job scheduling
│   │   ├── executor.rs      # Workflow execution
│   │   ├── pty.rs           # Claude Code subprocess
│   │   └── workflow/        # today.rs, archive.rs
│   └── Cargo.toml
│
├── scripts/                 # Python Phase 1/3 scripts
├── daybreak/docs/           # Design documentation + ADRs
└── CLAUDE.md                # This file
```

### Workspace (~/Documents/VIP)

```
~/Documents/VIP/
├── _today/              # Daily briefing output + data/
├── _inbox/              # Incoming files for processing
├── _archive/            # Archived daily briefings
├── Accounts/            # CS profile: customer accounts
└── Projects/            # Project folders
```

---

## Architecture

### Key Decisions

See `daybreak/docs/decisions/` for full ADRs. Summary of foundational choices:

| Decision | Choice | ADR |
|----------|--------|-----|
| App framework | Tauri v2 (Rust + React) | [0001](daybreak/docs/decisions/0001-use-tauri-over-electron.md) |
| Data architecture | JSON for machines, markdown for humans | [0004](daybreak/docs/decisions/0004-hybrid-json-markdown-architecture.md) |
| Determinism boundary | Python phases wrap AI phase | [0006](daybreak/docs/decisions/0006-determinism-boundary.md) |
| Storage | Markdown (source of truth) + SQLite (disposable cache) | [0018](daybreak/docs/decisions/0018-hybrid-storage-markdown-sqlite.md) |
| Governance | App-native Rust, not ported CLI scripts | [0025](daybreak/docs/decisions/0025-app-native-governance.md) |
| Extensions | Profile-activated modules, not monolith | [0026](daybreak/docs/decisions/0026-extension-architecture.md) |

### Three-Phase Pattern

Every workflow follows: **Prepare → Enrich → Deliver**

| Phase | Executor | Purpose |
|-------|----------|---------|
| Phase 1 | Python (`prepare_*.py`) | Fetch APIs, generate directive JSON |
| Phase 2 | Claude Code (via PTY) | AI enrichment, synthesis |
| Phase 3 | Python (`deliver_*.py`) | Write validated JSON + markdown |

### Config

- Config directory: `~/.dailyos/`
- Config file: `~/.dailyos/config.json` — `workspacePath` + `profile` fields
- Environment variables: `DAILYOS_*` prefix

---

## UI Design System

See `daybreak/docs/UI-SPEC.md` for full specification.

| Token | Hex | Usage |
|-------|-----|-------|
| `cream` | `#f5f2ef` | Primary background |
| `charcoal` | `#1a1f24` | Primary text, dark UI |
| `gold` | `#c9a227` | Primary accent, customer items |
| `peach` | `#e8967a` | Errors, warnings |
| `sage` | `#7fb685` | Success, personal items |

**Fonts:** DM Sans (body), JetBrains Mono (code/times)

---

## Branching

- `main` — Stable releases (tagged)
- `feature/daybreak` — Active development (this branch)
