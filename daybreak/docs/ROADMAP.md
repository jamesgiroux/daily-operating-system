# DailyOS Roadmap

> What's next, what's blocked, and what the code already proves.

---

## Where We Are (2026-02-06)

**Phase 1 is complete.** The core promise works: briefing runs automatically, dashboard renders, archive cleans up at midnight, no terminal required.

**Significant Phase 2/3 code exists but is unvalidated:**
- Email triage pipeline (three-tier priority, AI-enriched context)
- Calendar data loading and meeting prep generation
- Action sync between dashboard and workspace
- Focus extraction from briefing data
- Post-meeting capture data model (types defined, no UI)

The gap is not "what to build" but "what works reliably and what doesn't."

---

## Phase Map

| Phase | Promise | Status |
|-------|---------|--------|
| **Phase 1** | "Your day is ready" — passive consumption | **Complete** |
| **Phase 1.5** | Nav refactor, sidebar, profile-aware UI | **Complete** |
| **Phase 2** | Active processing — inbox, actions, data hygiene | **In progress** |
| **Phase 3** | Intelligent prompts — calendar awareness, post-meeting, weekly | Pending decisions |
| **Phase 4** | Extensible platform — extensions, MCP | Future |

---

## Phase 2: "Active Processing"

**Goal:** The data layer works reliably. Inbox processes. Actions sync. Stale data is handled gracefully.

### Critical Gaps (from [BACKLOG](BACKLOG.md))

| Issue | Problem | Why it matters |
|-------|---------|---------------|
| **I13** | No onboarding flow | First-time user hits dead end |
| **I14** | Meeting cards don't link to detail | Most important UX action is broken |
| **I7** | Can't change workspace path in Settings | Requires manual config editing |
| **I15** | Profile switching unavailable | Settings says "change later" but can't |

### Pending Decisions (from [ADRs](decisions/README.md))

These proposed ADRs gate Phase 2/3 work — they need to be accepted or revised before building:

| ADR | Decision | Gates |
|-----|----------|-------|
| [0031](decisions/0031-actions-source-of-truth.md) | Actions: SQLite as working store | Action sync design |
| [0032](decisions/0032-calendar-source-of-truth.md) | Calendar source of truth | Calendar data flow |
| [0033](decisions/0033-meeting-entity-unification.md) | Meeting entity unification | Meeting card links (I14) |
| [0034](decisions/0034-adaptive-dashboard.md) | Adaptive dashboard density | Dashboard layout evolution |

### What "Done" Looks Like

- [ ] Onboarding flow: profile, Google auth, workspace path, first briefing (I13)
- [ ] Meeting cards drill down to detail page (I14, depends on ADR-0033)
- [ ] `_today/data/` cleaned on archive — stale JSON doesn't linger
- [ ] Data freshness indicator — "Last updated Tuesday" not empty state
- [ ] "Generate Briefing" button replaces "Run /today" text
- [ ] Settings: workspace path picker (I7), profile switcher (I15)
- [ ] Actions reliable across briefing cycles (depends on ADR-0031)
- [ ] 7-day crash-free validation

### What Exists but Needs Validation

Code paths that are built but not tested end-to-end:

| Feature | Code exists in | Validation needed |
|---------|---------------|-------------------|
| Email three-tier triage | `deliver_today.py`, email page components | Does AI enrichment flow through to JSON? |
| Focus extraction | `json_loader.rs`, focus page | Does multi-strategy fallback work with real data? |
| Action list rendering | Dashboard components, `json_loader.rs` | Do actions survive re-briefing? What's the source of truth? |
| Calendar data loading | `json_loader.rs`, schedule components | Does meeting prep link to the right detail page? |

---

## Phase 3: "Intelligent Prompts"

**Goal:** Context-aware interactions at natural moments.

### Prerequisites

- Phase 2 stable (data layer reliable, actions trustworthy)
- ADR-0032 accepted (calendar source of truth)
- ADR-0033 accepted (meeting entity unification)

### Features

| Feature | Key question | Related ADR |
|---------|-------------|-------------|
| Calendar polling | How does real-time calendar data layer over daily briefing? | ADR-0032 |
| Post-meeting capture | What UI? What persists where? How does it feed back into next briefing? | ADR-0023, I17 |
| Weekly planning | Interactive or generated? What does "skipping" look like? | ADR-0030 |

### What "Done" Looks Like

- [ ] System knows when meetings end (calendar polling)
- [ ] Post-meeting prompt appears at natural moment, dismissible without guilt
- [ ] Captured outcomes appear in next day's briefing (I17)
- [ ] Weekly prep generated with daily refresh (ADR-0030)
- [ ] Skipping weekly planning has sensible defaults (zero-guilt)

---

## Phase 4: "Extensible Platform"

**Goal:** Domain-specific features as extensions. MCP integration.

Not scheduled. Depends on Phase 3 stability. Key decisions already made:

- Extension architecture: [ADR-0026](decisions/0026-extension-architecture.md)
- MCP dual-mode: [ADR-0027](decisions/0027-mcp-dual-mode.md)
- Profile system: [ADR-0020](decisions/0020-profile-dependent-accounts.md)

Core extensions: Customer Success (CSM profile default), Professional Development (opt-in), CRM/Clay (opt-in).

Phase 5+ is public SDK and community plugins.

---

## Risks and Dependencies

Tracked in [BACKLOG.md](BACKLOG.md) (R1-R4, D1-D3). Not duplicated here.

---

*Decisions: [docs/decisions/](decisions/README.md). Issues: [BACKLOG.md](BACKLOG.md). Code is the proof of what's done.*
