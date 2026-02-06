# RAIDD Log

> Risks, Assumptions, Issues, Dependencies, Decisions

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | No | Need onboarding check |
| A2 | Workspace follows PARA structure | No | Should gracefully handle variations |
| A3 | `_today/` files use expected markdown format | Partial | Parser handles basic cases |

---

## Issues

| ID | Issue | Priority | Owner | Status |
|----|-------|----------|-------|--------|
| I1 | Config directory named `.daybreak` should be `.dailyos` for brand consistency | Low | — | Open |
| I2 | Compact `meetings.md` format for dashboard dropdowns | Low | — | Explore |
| I3 | Browser extension for web page capture to `_inbox/` | Low | — | Explore |
| I4 | Motivational quotes as personality layer | Low | — | Explore |
| I5 | Orphaned pages need role definition (Focus, Week, Emails) | Medium | — | Open |
| I6 | Processing history page — "where did my file go?" | Low | — | Explore |

### I2 Notes
The archive from 2026-02-04 contains a compact `meetings.md` format with structured prep summaries:
```markdown
## 1:00 PM - Meeting Title
type: customer
account: Account Name
end: 1:45 PM

### Prep
**Context**: Brief context with key metrics (ARR, renewal date, etc.)
**Wins**: Bullet list of recent wins
**Risks**: Bullet list of current risks
**Actions**: Bullet list of discussion items
```
This format could be useful for:
- Dashboard meeting card dropdowns (quick glance without full prep)
- Role-specific templates (CSM/Sales may need this more than others)
- Generating consolidated daily meeting summary

Consider adding as a Claude Code template output for `/today` command post-MVP.

### I3 Notes
Chromium-based browser extension that captures a page's text content and drops it as a markdown file into `_inbox/`. This turns the browser into another input source for the inbox processing pipeline (F4).

**Why it fits:**
- Aligns with "system does the work" — user clicks one button, DailyOS processes and routes
- `_inbox/` already handles document classification, routing, and enrichment
- No new processing pipeline needed — just a new input channel

**Considerations:**
- Chromium extension API for page content extraction (text, not full HTML)
- File naming convention: `clip-YYYY-MM-DD-HHMMSS-page-title.md`
- Frontmatter with source URL, capture timestamp, page title
- Could include selection-only capture (highlight text → clip to inbox)
- Manifest V3 compatibility
- Explore post-Phase 2 when inbox processing (F4) is stable

### I4 Notes
Cheesy motivational quotes (Chris Farley "van down by the river" energy) as a personality layer. Adds humor and delight without friction.

**Rejected approach:** Welcome interstitial screen before dashboard. Adds a required daily click, conflicts with "Open the app. Your day is ready." and Principle 2 (Prepared, Not Empty).

**Viable placements (explore post-MVP):**
- **Overview greeting** — daily rotating quote replaces "Good morning" in the left column. First thing you see, zero extra clicks. "Let's go" energy.
- **Empty states** — no meetings, no actions, inbox clear. Flips "nothing here" into "you crushed it" energy. Rewards being caught up instead of showing a dead end.
- These are complementary, not mutually exclusive — different quote pools, different tones.

**Open questions:**
- Quote source: curated list vs generated? Curated is safer for tone.
- How many quotes before repeats? 365 would cover a year.
- Should quotes be deterministic per date (same quote every Feb 5) or random?

### I5 Notes
Three pages exist in `src/pages/` but are unrouted after DEC10 sidebar simplification:

- **FocusPage.tsx** — Was a standalone Focus view. DEC10 says Focus becomes a dashboard section, but the page may have value as a drill-down from the Overview focus indicator.
- **WeekPage.tsx** — Weekly planning view. Needed for Phase 3C. Keep for re-integration.
- **EmailsPage.tsx** — Standalone email view. Dashboard shows emails inline, but a full-page drill-down may be needed as email volume grows.

**Action:** Review each page's role when its parent phase arrives. Don't delete — assess whether they become drill-downs, standalone views, or get absorbed into existing pages.

### I6 Notes
A `processing_log` table already exists in SQLite (`schema.sql`) and records every file processed through the inbox pipeline: filename, classification, destination path, status, timestamps, and error messages. `db.rs` has `get_processing_log(limit)` to query it.

**What's missing:**
- No Tauri command exposes the log to the frontend
- No UI renders the history
- Users have no way to trace where a file ended up after processing

**Proposed feature:**
- Wire `get_processing_log` as a Tauri command
- Build a History view accessible from the Inbox page (or as a tab/section)
- Each entry shows: filename, classification type, destination path, timestamp, status
- Clicking a destination path could open the file in Finder
- Supports Principle 9 (Show the Work, Hide the Plumbing) — the system routed your file, now you can trace it

**Why it matters:**
Discovered while dogfooding: after processing a call transcript, there's no feedback about where it went. The file disappears from the inbox, but the destination is invisible. Trust requires traceability.

**Infrastructure ready:** Table, queries, and logging already exist. This is primarily a frontend feature + one Tauri command.

---

## Dependencies

| ID | Dependency | Type | Status | Notes |
|----|------------|------|--------|-------|
| D1 | Claude Code CLI | Runtime | Available | Requires user subscription |
| D2 | Tauri 2.x | Build | Stable | Using latest stable |
| D3 | Google Calendar API | Runtime | Optional | For calendar features (Phase 3) |

---

## Decisions

| ID | Decision | Date | Rationale | Alternatives Considered |
|----|----------|------|-----------|------------------------|
| DEC1 | Use Tauri over Electron | 2024-01 | Smaller binary, Rust backend, native performance | Electron (too heavy), native Swift (platform lock-in) |
| DEC2 | Frontend-first implementation | 2024-01 | Reveals data shapes before backend investment | Backend-first (speculative) |
| DEC3 | Config in JSON file, no UI for MVP | 2024-02 | Reduces scope, power users can edit | Settings UI (adds complexity) |
| DEC4 | Hybrid JSON + Markdown architecture | 2026-02 | JSON for machine consumption, markdown for humans. Eliminates fragile regex parsing. | Markdown-only (fragile), JSON-only (not human-readable) |
| DEC5 | Archives remain markdown-only | 2026-02 | Historical data is for human reference. JSON generation happens at runtime for active `_today/` only. | Full JSON archives (unnecessary complexity) |
| DEC6 | Phase 3 generates JSON (not Claude) | 2026-02 | Maintains determinism boundary. Claude outputs markdown (its strength), Python converts to validated JSON. | Claude outputs JSON directly (less reliable) |
| DEC7 | Dashboard is the product, not a page among pages | 2026-02 | 80% of time is spent here. Meetings, actions, emails all render on dashboard. Other pages are drill-downs or supporting views. | Equal-weight pages (spreads attention, loses focus) |
| DEC8 | Profile-aware navigation with entity pattern | 2026-02 | Each profile has a primary entity: CS=Accounts (`2-areas/accounts/`), GA=Projects (`1-projects/`). Same sidebar structure, same portfolio page component, different data shape. Neither profile is "stripped down." | Single nav for all (irrelevant items), CS-only entity page (GA gets nothing) |
| DEC9 | Profile switching is non-destructive | 2026-02 | Switching changes classification rules, sidebar items, and card metadata. Does NOT move, delete, or restructure files. PARA directories persist across switches. | Destructive switch (data loss risk, violates zero-guilt) |
| DEC10 | Focus, Week, Emails removed from sidebar | 2026-02 | Focus is a dashboard section. Week is post-MVP. Emails are on the dashboard already. Sidebar should have 3-4 items max, not 6+. | Keep all nav items (cluttered, confusing hierarchy) |
| DEC11 | Sidebar groups: Today + Workspace | 2026-02 | "Today" holds Dashboard. "Workspace" holds Actions, Inbox, and profile entity (Accounts/Projects). Clean two-group structure, 4 items total for both profiles. | Three+ groups (over-categorized for 4 items) |
| DEC12 | Profile indicator in sidebar header | 2026-02 | Shows current profile below app name. Phase 2: clickable to switch. Visible but not intrusive (Slack workspace switcher pattern). | Settings-only (hidden, hard to discover), Onboarding-only (can't change later) |
| DEC13 | Meeting detail is a drill-down, not a nav item | 2026-02 | Accessed by clicking meeting cards on dashboard. Back button returns to dashboard. No sidebar entry needed. | Meeting list page (redundant with dashboard timeline) |
| DEC14 | MVP = F1 + F7 + F6 + F3 | 2026-02 | Prove core value prop first: briefing, dashboard, tray, archive. | Larger MVP (too much risk), smaller MVP (not enough value) |
| DEC15 | Defer inbox processing to Phase 2 | 2026-02 | Two-tier processing adds complexity. Core value is passive consumption. | Include in MVP (scope creep) |
| DEC16 | Defer post-meeting capture to Phase 3 | 2026-02 | Requires calendar integration working first. | Include in Phase 2 (dependency chain) |
| DEC17 | Pure Rust archive (no three-phase) | 2026-02 | Archive doesn't need AI. Simpler, faster, no Python dependency. | Three-phase archive (unnecessary AI), Python archive (extra dependency) |
| DEC18 | Hybrid storage: Markdown + SQLite | 2026-02 | User content stays markdown (portable, human-readable). System state in SQLite (performant, queryable). SQLite is disposable cache rebuilt from files. | Markdown-only (slow queries), SQLite-only (not portable) |
| DEC19 | Reference approach for directives | 2026-02 | Directive JSON contains file refs, not embedded content. Claude loads files selectively during Phase 2. Reduces directive size, gives Claude control over depth. | Embedded context (large directives, fixed depth) |
| DEC20 | Profile-dependent accounts (CSM plugin) | 2026-02 | Accounts are a "plugin" for CSM profile, not required for General. Profile selection configures workspace structure, meeting classification, prep templates. | Accounts for all users (irrelevant for non-CSM), no profiles (one-size-fits-all) |
| DEC21 | Multi-signal meeting classification | 2026-02 | Attendee count → title keywords → attendee cross-reference → internal heuristics. Uses OAuth domain for internal/external detection. | Title-only (unreliable), manual classification (user burden) |
| DEC22 | Proactive research for unknown meetings | 2026-02 | System searches local archive + web for context on unknown external meetings. Per PRINCIPLES.md: "The system operates. You leverage." | Ask user to fill in gaps (violates zero-guilt), skip unknown meetings (missed value) |
| DEC23 | `/wrap` replaced by post-meeting capture | 2026-02 | Batch end-of-day closure is unnatural. Most wrap functions (archive, reconciliation) happen automatically. Post-meeting capture (F2) is more natural. See PRD.md Appendix A. | Keep /wrap (artificial ritual) |
| DEC24 | Email = AI triage, not email client | 2026-02 | App shows AI-curated summaries and suggested actions, not raw emails. Morning briefing auto-archives low-priority with a reviewable manifest. On-demand `/email-scan` refreshes throughout the day. CLI can draft replies and archive — app surfaces intelligence, CLI does actions. | Build email client (scope creep), Show all emails (information overload), No auto-archive (manual triage burden) |

---

*Last updated: 2026-02-05*
