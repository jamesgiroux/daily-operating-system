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
| A4 | Users have Google Workspace (Calendar + Gmail) | No | MVP scope — personal Gmail, Outlook, iCloud Calendar not supported |

---

## Issues

| ID | Issue | Priority | Owner | Status |
|----|-------|----------|-------|--------|
| I1 | Config directory named `.daybreak` should be `.dailyos` for brand consistency | Low | — | Closed |
| I2 | Compact `meetings.md` format for dashboard dropdowns | Low | — | Explore |
| I3 | Browser extension for web page capture to `_inbox/` | Low | — | Explore |
| I4 | Motivational quotes as personality layer | Low | — | Explore |
| I5 | Orphaned pages need role definition (Focus, Week, Emails) | Medium | — | Closed |
| I6 | Processing history page — "where did my file go?" | Low | — | Explore |
| I7 | Settings page can't change workspace path — only refresh it | Medium | — | Open |
| I8 | No app update/distribution mechanism decided | Medium | — | Open |
| I9 | Focus page and Week priorities are disconnected stubs | Medium | — | Open |
| I10 | No shared glossary of app terms and workflow names | Low | — | Open |
| I11 | Phase 2 email enrichment not fed back to JSON pipeline | High | — | Resolved |
| I12 | Email page missing AI context (summary, arc, recommended action) | High | — | Resolved |
| I13 | No onboarding flow — first-time user hits dead end after profile selection | High | — | Open |
| I14 | Dashboard meeting cards don't link to meeting detail page | High | — | Open |
| I15 | Profile switching unavailable in Settings (promised at onboarding) | Medium | — | Open |
| I16 | Schedule editing requires manual config.json editing (cron expressions) | Medium | — | Open |
| I17 | Post-meeting capture outcomes don't visibly resurface in briefings | Medium | — | Open |
| I18 | Google API calls not coordinated across callers (no shared cache) | Medium | — | Open |
| I19 | AI enrichment failure not communicated to user (briefing feels "thin") | Low | — | Open |
| I20 | No standalone email refresh — emails only update with full briefing | Medium | — | Open |

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

### I5 Notes (Closed)
Three pages existed in `src/pages/` unrouted after DEC10 sidebar simplification. All resolved:

- **FocusPage.tsx** — Drill-down from Overview focus indicator. Route: `/focus`. Not in sidebar (per DEC10). Back arrow returns to dashboard.
- **WeekPage.tsx** — In sidebar as "This Week" (Phase 1.5 nav refactor). Route: `/week`.
- **EmailsPage.tsx** — Drill-down from Dashboard email card. Route: `/emails`. Not in sidebar (per DEC10). Back arrow returns to dashboard.

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

### I7 Notes
Settings page displays workspace path as read-only text with a refresh button. The refresh button calls `reload_configuration` which re-reads `~/.dailyos/config.json` from disk — so if a user manually edits the JSON, it works. But there's no UI to *change* the path.

**What's needed:**
- Tauri `dialog::FileDialogBuilder` to let user pick a directory
- New command: `set_workspace_path(path)` — updates config.json and reloads
- Validation: confirm the selected directory exists and looks like a PARA workspace (has `_today/`, `_inbox/`, etc.)
- The "Edit config.json" hint is an acceptable power-user escape, but a button should be the primary path (Principle 3: Buttons, Not Commands)

**Scope:** Small. One Tauri command + one dialog + one button swap on Settings page.

### I8 Notes
DEC25 references "standard app distribution (DMG, auto-update)" but no mechanism has been decided. The CLI era used `git pull` on `~/.dailyos/` — that doesn't apply to a native app.

**Options to evaluate:**
- **Tauri's built-in updater** — `tauri-plugin-updater` supports checking a remote endpoint for new versions, downloading, and applying. Requires hosting a JSON manifest + signed binaries somewhere (S3, GitHub Releases, custom server).
- **GitHub Releases + Sparkle** — macOS-native update framework. Well-understood, but Tauri's built-in may be sufficient.
- **Manual DMG distribution** — Simplest. User downloads new version from a URL. No auto-update. Acceptable for early alpha but doesn't scale.
- **Mac App Store** — Handles distribution and updates natively. Adds review process overhead and Apple's 30% cut. May conflict with Claude Code CLI dependency (sandboxing restrictions).

**Open questions:**
- Where are signed binaries hosted? (GitHub Releases is the path of least resistance)
- Code signing: Apple Developer ID required for notarization. Without it, macOS Gatekeeper blocks the app.
- How do we handle workspace schema migrations between versions? (e.g., new required fields in `accounts-mapping.json`)
- Update frequency: silent auto-update vs. notify-and-confirm?
- Is there a Windows/Linux story or is this macOS-only for now?

**Not blocking MVP** — the app can ship as a manual DMG install. But this needs a decision before any public distribution.

### I9 Notes
Focus page shows daily priorities and time blocks. Week page shows weekly priorities (`focusAreas`). These are conceptually related but currently disconnected:

- **Focus** data comes from `focus.json` (currently a stub — `get_focus_data` returns "not yet implemented")
- **Week priorities** come from `week-overview.json` → `focusAreas` array (populated by Phase 2 enrichment of the week directive)

**The relationship should be:** `/week` sets weekly priorities. `/today` derives daily focus from weekly priorities + today's specific meetings. Focus page is a drill-down of the dashboard focus indicator — it should show today's recommended focus areas with time blocks, drawing from both weekly priorities and today's schedule gaps.

**What needs to happen:**
1. `deliver_today.py` should generate `focus.json` from the directive (extracting focus from `81-suggested-focus.md` or the directive's context)
2. Weekly priorities should flow into daily focus as a "this week's themes" section
3. The Focus page and Week priorities card should share the same underlying data, viewed at different time horizons
4. "Plan This Week" should be the user-facing action that triggers `/week` — the questionnaire answers become Phase 1 input for AI enrichment

### I10 Notes
Multiple features use overlapping terms without shared definitions:

| Term | Used Where | Current Meaning |
|------|-----------|----------------|
| **Briefing** | Dashboard, Settings | The morning `/today` output (Phase 1→2→3) |
| **Generate Briefing** | Dashboard empty state | Triggers the `/today` workflow |
| **Plan This Week** | Week page | Triggers the `/week` workflow |
| **Run /week** | Week page button | Same as above — inconsistent label |
| **Focus** | Dashboard card, Focus page | Daily priority areas (stub) |
| **Weekly Priorities** | Week page card | Weekly focus areas from `/week` |
| **Inbox** | Sidebar, Inbox page | Document processing queue (`_inbox/`) |
| **Email Scan** | Email page button | On-demand email refresh workflow |
| **Capture** | Post-meeting prompt | Win/Risk/Action quick entry after meetings |

**What needs to happen:** A shared glossary (could live in `DEVELOPMENT.md` or a new `GLOSSARY.md`) defining each term, its user-facing label, and which workflow/data source backs it. Button labels and page titles should be consistent.

### I11 Notes — RESOLVED
Phase 2 email enrichment is now parsed back into the JSON pipeline.

`deliver_today.py` gained `parse_email_enrichment()` which reads `83-email-summary.md`, extracts per-email enrichment (summary, recommended action, conversation arc, email type, action owner), and merges it into `emails.json` by fuzzy subject matching.

Three-tier priority restored: high / medium / low (was collapsed to high / normal). Full stack updated: Python → Rust types + JSON loader → TypeScript types → frontend.

### I12 Notes — RESOLVED
Email page now shows AI context from Phase 2 enrichment:
- **High priority:** Summary, recommended action (gold `→` prefix), conversation arc (italic)
- **Medium priority:** Summary
- **Low priority:** Subject only (collapsed section)

Dashboard email widget shows recommended action or summary on high-priority items.

Also removed fake "Scan emails" button that was a no-op (called nonexistent `email_scan` workflow). See I20.

### I13 Notes
**Ref:** USER-JOURNEYS.md Journey 1 (First Launch)

The app goes from profile selection directly to the dashboard. If `~/.dailyos/config.json` doesn't exist or has no workspace path, behavior is undefined. If Google isn't connected, "Generate Briefing" fails because `prepare_today.py` can't fetch calendar or email data.

**What a minimal onboarding needs:**
1. Profile selection (exists, works)
2. Google account connection (exists in Settings, not surfaced during setup)
3. Workspace path selection (I7 — display exists, editing doesn't)
4. First briefing trigger with progress feedback

**Design constraint:** Principle 4 (Opinionated Defaults, Escapable Constraints). We could create `~/Documents/DailyOS/` as default workspace and skip the path picker entirely. Google connection is the only truly mandatory step.

**Open question:** Can the app show anything useful before Google is connected? If yes, onboarding is "connect Google when you're ready." If no, onboarding must gate the dashboard behind Google auth.

### I14 Notes
**Ref:** USER-JOURNEYS.md Journey 2 (Morning Briefing)

DEC13 decided meeting detail is a drill-down from the dashboard (not a sidebar item). The detail page exists at `/meeting/$prepFile` and renders prep data from `preps/*.json`. But the dashboard meeting cards (`MeetingCard.tsx`) don't link to it. The most important user action on the dashboard — "I see my next meeting, let me review the prep" — is a dead end.

**What's needed:** `MeetingCard` gets an `onClick` or `Link` that navigates to `/meeting/{prepFile}` when a prep file exists. If no prep exists, the card is not clickable (or shows "No prep available" on click).

**Scope:** Small. One `Link` wrapper + conditional logic. But it's the highest-impact UX fix because it completes the core promise.

### I15 Notes
**Ref:** USER-JOURNEYS.md Journey 9 (Settings)

The profile selector at first launch says "You can change this later in Settings." But the Settings page has no profile switching UI. DEC12 mentions "Phase 2: clickable to switch" in the sidebar header, but neither location currently supports it.

**What's needed:** A profile selector in Settings (dropdown or radio) that writes to `~/.dailyos/config.json` and triggers a config reload. The sidebar profile label should update without an app restart.

**Scope:** Medium. Tauri command to update config + UI control + live reload.

### I16 Notes
**Ref:** USER-JOURNEYS.md Journey 9 (Settings)

The Settings page displays schedule cron expressions (`0 6 * * *`) that are meaningless to non-technical users. Schedule editing requires manually editing `~/.dailyos/config.json`.

**What's needed:** A time picker ("Briefing time: 6:00 AM [change]") that writes the equivalent cron to config. Hide the cron syntax entirely. Power users can still edit the JSON directly (Principle 4: Escapable Constraints).

**Scope:** Medium. Time picker component + Tauri command to update schedule config.

### I17 Notes
**Ref:** USER-JOURNEYS.md Journey 5 (Post-Meeting Capture)

When the user captures wins, risks, and actions after a meeting, data goes to SQLite and the impact log markdown. But none of it visibly appears in the next day's briefing or on the dashboard. The user captures data and it vanishes into the system.

**This erodes trust.** If the user types "Win: Acme agreed to expand" and never sees that reflected anywhere, they'll stop capturing.

**Where captured data should resurface:**
- **Actions** → Next day's Actions list (SQLite already handles this)
- **Wins/Risks** → Next meeting prep for the same account (requires `prepare_today.py` to query SQLite for recent captures by account)
- **Weekly summary** → "This week: 4 wins captured, 2 risks flagged" (requires aggregation query)

**Depends on:** DEC31 (action source of truth), CS extension architecture for account-scoped data.

### I18 Notes
**Ref:** USER-JOURNEYS.md Cross-Journey (Google API Efficiency)

Multiple callers hit Google APIs independently:
- `prepare_today.py` fetches Calendar + Gmail during briefing
- Calendar poller (`scheduler.rs`) fetches Calendar every 5 minutes
- Manual "Generate Briefing" re-fetches everything

No caching or coordination between these callers. A shared API cache with TTL (e.g., calendar data valid for 5 minutes, email data valid for 15 minutes) would reduce redundant calls and make manual re-runs faster.

**Not blocking MVP** but becomes important as more features consume calendar/email data.

### I19 Notes
**Ref:** USER-JOURNEYS.md Cross-Journey (AI Enrichment Timing)

When Phase 2 (Claude Code enrichment) fails or is skipped, the briefing renders with Phase 1 data only. The dashboard looks functional but is notably thinner — no AI summary, no prep context, no focus suggestions. The user has no indication of why.

**Options:**
- Quiet indicator: "AI-enriched" badge on overview when enrichment succeeded (absence = not enriched)
- Explicit note: "Quick briefing — AI enrichment will run on next refresh" when Phase 2 was skipped
- Nothing: just let the briefing be what it is (current behavior)

**Recommendation:** The quiet indicator (option 1) fits Principle 9 (Show the Work, Hide the Plumbing) — inform without alarming.

### I20 Notes
The email page and dashboard widget had a "Scan emails" button that called `run_workflow({ workflow: "email_scan" })`. No such workflow exists — `WorkflowId` only has Today, Archive, InboxBatch, Week. The button was silently catching the error and spinning for 3 seconds to fake activity.

**Removed** the dead buttons. Emails now correctly refresh only as part of the `/today` briefing workflow (prepare → enrich → deliver).

**Future consideration:** A lightweight email-only refresh pipeline (just email fetch + classify + deliver, skipping calendar/meetings/actions) could be valuable. But it introduces a question: if email data refreshes independently, does the rest of the briefing show stale data? Need to think about partial refresh semantics before building this. DEC6 (determinism boundary) still applies — any new pipeline must go through Python, not be a frontend-initiated API call.

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
| DEC25 | App-native governance, not ported CLI tools | 2026-02 | Workspace governance (classification, routing, validation, account registry) is Rust-native in the Tauri app. The app reads `~/.dailyos/` registries and workspace structure directly — no Python subprocess calls for core operations. App is the authority on first install; CLI and MCP are secondary interfaces that share the same registries. Existing `_tools/` Python scripts inform the design but are not ported 1:1. | Port Python tools to Rust (cargo-culting CLI patterns), App calls Python subprocess (fragile, extra dependency), Shared Python scripts (CLI-era pattern, not app-native) |
| DEC26 | Extension architecture with profile-activated modules | 2026-02 | Domain-specific features (Customer Success, Professional Development) are extensions, not core. Profiles activate default extension sets. Internal module boundaries designed now; public SDK/community plugins are a future concern. Extensions can add post-enrichment steps, dashboard sections, sidebar items, data schemas, and workflow hooks. | Monolithic app (every feature for every user), Plugin marketplace day-one (premature), Separate apps per role (fragmented) |
| DEC27 | MCP integration: dual-mode server + client | 2026-02 | The app is both an MCP server (exposes workspace tools/resources to external AI like Claude Desktop) and an MCP client (consumes external services like Clay). Positions DailyOS as an AI-native integration hub. App, CLI, and MCP share the same registries (DEC25). | API-only (no standard protocol), MCP server only (can't consume), Custom integration per service (doesn't scale) |
| DEC28 | Structured document schemas (JSON-first templates) | 2026-02 | Account dashboards, success plans, and other structured documents get JSON schemas. Rust can mechanically update specific sections (Last Contact, Recent Wins, etc.) without AI. Markdown generated from JSON for human readability. Extends DEC4 pattern to all structured workspace documents. | Markdown-only with regex updates (fragile), AI-only updates (expensive, non-deterministic), Separate database (violates local-first) |
| DEC29 | Three-tier email priority with AI-enriched context | 2026-02 | **PENDING.** Keep three tiers (high/medium/low) throughout the pipeline instead of collapsing to two. High = customer/urgent (show with full AI context: summary, conversation arc, recommended action). Medium = internal/meeting-related (show in collapsible section, not silently archived). Low = automated/newsletters (archived with reviewable manifest per DEC24). Phase 2 enrichment writes structured data back to JSON, not just markdown. User-configurable rules are Phase 4+ (extension point). | Two tiers high/normal (loses medium nuance), User classifies manually (violates zero-guilt), All emails shown equally (information overload) |
| DEC30 | Weekly prep generation with daily refresh | 2026-02 | **PENDING.** `/week` (running Sunday) generates prep documents for all eligible meetings in the coming week. `/today` then refreshes rather than creating from scratch — updating with latest data, new emails, recent actions. Preps live in `_today/data/preps/` (ephemeral, regenerated). Week-generated preps cached in `_today/data/week-cache/` with staleness tracking. Trade-off: faster daily briefings vs. potential staleness by Thursday. Requires freshness check (manifest date comparison). | Daily-only preps (slower briefings, no look-ahead), Week preps with no daily refresh (stale by midweek), Persistent prep store (complexity, storage) |
| DEC31 | Actions: SQLite as working store, markdown as historical record | 2026-02 | **PENDING.** Actions live in SQLite for speed and queryability. Daily `actions.json` is a snapshot for the dashboard. No single `master-task-list.md` — actions are scoped to their source (meeting, email, manual). Completion in the app updates SQLite; post-enrichment hooks can write back to source markdown if a `source_label` points to a specific file. The Actions page is the canonical view; markdown files in `_today/` and `_archive/` are historical records. Bidirectional sync (markdown → SQLite) happens during inbox processing and briefing generation. | Single markdown file (slow, merge conflicts), SQLite only (not portable, not human-readable), Separate task app (fragmented) |
| DEC32 | Calendar source of truth: briefing snapshot vs live poll vs hybrid | 2026-02 | **PENDING.** See notes. | Briefing-only (stale), Live-only (no enrichment), Hybrid overlay (complex but correct) |
| DEC33 | Meeting entity unification: shared state vs independent views | 2026-02 | **PENDING.** See notes. | Unified Meeting entity in SQLite (correct but architectural change), Independent views (current, fragmented), Shared ID mapping (middle ground) |
| DEC34 | Adaptive dashboard: density-aware layout or static | 2026-02 | **PENDING.** See notes. | Time-aware adaptive layout (complex, polished), Static single layout (simple, adequate), Density hint in overview text only (middle ground) |

### DEC25 Notes

**What "governance" means here:**
The workspace (`~/Documents/VIP/`) follows a PARA directory structure with DailyOS conventions — `_inbox/`, `_today/`, `Accounts/`, `Projects/`, etc. "Governance" is the set of operations that keep this structure consistent: classifying incoming files, routing them to the right PARA location, validating naming conventions, enforcing frontmatter standards, and maintaining the account/project registry.

**What existed in the CLI era:**
The original DailyOS CLI solved this with Python scripts in a `_tools/` directory inside the workspace:

- `account-registry.py` — CRUD for customer accounts (creates 13-directory scaffold, manages aliases, business units)
- `move_to_canonical.py` — Reads YAML frontmatter (`doc_type`, `account`, `context`) and routes files to the correct PARA directory
- `standardize_frontmatter.py` — Enforces frontmatter templates per document type (required fields: area, account, doc_type, date, status, privacy, tags)
- `validate_naming.py` — Ensures `YYYY-MM-DD` naming convention
- `validate_cascade.py` — Post-generation validation that all outputs meet standards
- `accounts-mapping.json` — The canonical registry (account names, aliases, packages, parent companies)

The CLI also had a "core" architecture (`~/.dailyos/`) with versioning, symlinks into the workspace, and an eject/reset pattern for user customization. Updates pulled via git. `dailyos doctor` validated health; `dailyos repair` fixed broken symlinks.

**Why not just port the Python tools?**
The Tauri app is a fundamentally different runtime. The CLI was a developer tool — users ran commands in a terminal and had Python available. The app targets a broader audience where:

1. **Python is not guaranteed.** Requiring Python as a runtime dependency for a native app is fragile and adds install complexity.
2. **Subprocess calls are the wrong primitive.** The CLI shelled out to scripts because that's what CLIs do. The app has Rust, SQLite, and a file watcher — these are better primitives for the same operations.
3. **The symlink/eject pattern is CLI-specific.** Users updated CLI tools via `git pull` on `~/.dailyos/` and ejected to customize. The app ships bundled resources and updates via native app distribution (see I8 for open questions). The eject/reset concept doesn't map.
4. **The app sees the workspace holistically.** The CLI ran one script at a time. The app has persistent state (SQLite), a file watcher, and can react to changes in real time. Governance can be continuous, not invoked.

**What this means in practice:**

- **Classification and routing** become Rust functions in the app. Same rules as `move_to_canonical.py` (frontmatter-based routing, filename patterns), but compiled, tested, and running inside the app process.
- **Account/project registry** is read from `accounts-mapping.json` (or equivalent) in `~/.dailyos/`. The app reads and writes this registry natively. No Python intermediary.
- **Validation** runs automatically — the file watcher can validate incoming files as they appear in `_inbox/`, not just when a user remembers to run a script.
- **Interface parity** is preserved. The app, CLI, and Claude Code (via MCP) all read the same registries and workspace structure. Any interface can perform the same operations. But the app is the primary authoring interface for governance state — it's what creates the initial workspace structure, scaffolds accounts, and manages registries on first install.
- **CLI becomes optional.** A user who never opens a terminal still gets full governance. Power users who prefer CLI get the same capabilities through commands that read/write the same files.

**The key principle:** The app doesn't call tools — the app *is* the tool. Governance logic lives in compiled Rust, reads shared registries, and operates continuously via the file watcher. The CLI and MCP are alternative interfaces to the same underlying state, not the source of truth.

### DEC26 Notes

**What "extension" means here:**
An extension is a module that adds domain-specific capabilities to the core app. It is NOT a plugin marketplace or third-party SDK (that's future work). Today, an extension is an internal boundary — a set of features that activate together based on the user's profile or explicit opt-in.

**Core vs. Extension:**

| Layer | Core (always active) | Extension (profile-activated) |
|-------|---------------------|-------------------------------|
| Workflows | Briefing, Archive, Inbox Processing | — |
| Processing | Classification, PARA routing, frontmatter validation | Post-enrichment: dashboard updates, action sync to account files |
| Data | `_today/`, `_inbox/`, `_archive/`, SQLite actions | Account dashboards, success plans, account registry |
| UI | Dashboard, Actions, Inbox, Settings | Accounts page, account health indicators, portfolio triage |
| Impact | — | Impact capture (customer + personal), weekly roll-up |

**Extension examples:**

- **Customer Success** (`cs`): Account dashboards (JSON schemas from DEC28), daily-csm workflows (meeting prep, dashboard refresh, action tracking, health monitoring, value capture, impact reporting, renewal countdown, portfolio health), account registry CRUD, Google Sheet sync, engagement tracking. Activated by CSM profile. Includes templates from `enterprise-account-dashboard-v2`.
- **Professional Development** (`prodev`): Coaching reflections, two-sided impact capture (customer outcomes vs. personal impact), weekly/monthly/quarterly impact roll-ups, leadership metrics. Optional add-on for any profile.
- **Relationship Management** (`crm`): Clay integration (MCP client), contact notes after meetings, contact creation for unknowns. Optional add-on requiring Clay MCP.

**What an extension provides (internal contract):**

1. **Post-enrichment hooks** — After AI enrichment completes on a file, extensions can run mechanical updates. The CS extension updates account dashboards; the ProDev extension captures impact entries. These are Rust functions, not AI calls.
2. **Data schemas** — JSON schemas for structured documents the extension manages. The CS extension defines the account dashboard schema; Rust validates and updates sections mechanically.
3. **UI contributions** — Sidebar items, dashboard sections, page routes. The CS extension adds "Accounts" to the sidebar; ProDev might add an "Impact" section to the dashboard.
4. **Workflow hooks** — Extensions can register steps in existing workflows. The CS extension adds a post-enrichment step to inbox processing; ProDev adds impact prompts to the wrap flow.
5. **Templates** — Document templates for the extension's domain. The CS extension ships ring-specific dashboard templates, success plan templates, etc.

**Profile → Extension mapping:**

| Profile | Default Extensions | Available Add-ons |
|---------|-------------------|-------------------|
| CSM | Core + Customer Success | Professional Development, Relationship Management |
| General | Core | Professional Development, Relationship Management |

**Why not a plugin SDK now:**
The internal extension boundaries are what matter for Phase 2-3 implementation. A public SDK requires: versioned APIs, documentation, review process, distribution mechanism, sandboxing. That's Phase 5+ work. But designing clean internal module boundaries now means the SDK is a formalization of what already works, not a retrofit.

**What the CLI skills become:**
Existing CLI skills (`daily-csm`, `inbox-processing`, etc.) inform the extension design but are not ported 1:1 (per DEC25). The daily-csm skill's 7 workflows become Rust functions in the CS extension. The coaching/impact workflows become Rust functions in the ProDev extension. Claude Code still handles AI enrichment (Phase 2 of three-phase), but the mechanical pre/post steps are extension-owned Rust code.

### DEC27 Notes

**MCP Server mode (the app exposes capabilities):**
Other AI tools (Claude Desktop, custom agents, automation scripts) can interact with DailyOS data through MCP. The app registers as an MCP server and exposes:

- **Resources**: Workspace structure, today's briefing, account dashboards, action lists, meeting schedule, processing queue status
- **Tools**: Create action items, mark actions complete, trigger inbox processing, scaffold new accounts, query account health, get meeting prep

This means a user in Claude Desktop can say "what's on my schedule today?" and get DailyOS data without opening the app. Or an automation can create action items programmatically.

**MCP Client mode (the app consumes external services):**
The app can call external MCP servers for integrations:

- **Clay** (Automattic): Contact lookup, note creation, contact creation after meetings
- **Google APIs**: Calendar polling, Sheet updates (if not handled via Python scripts)
- **Future**: Slack, Linear, Notion, or any MCP-compatible service

**Interface parity (DEC25 extended):**
Three interfaces share the same registries and workspace:

```
App (Tauri)  ─┐
CLI (Claude)  ─┤── Same registries (~/.dailyos/)
MCP (Server)  ─┘   Same workspace (~/Documents/VIP/)
```

Any interface can perform the same operations. The app is the primary authority (DEC25), but MCP enables AI-native automation on top of the same data.

**Security considerations:**
- MCP server should require explicit user consent for tool execution (read vs. write)
- Sensitive data (account financials, contact info) should respect privacy levels from frontmatter
- MCP server binds to localhost only — no remote access
- Rate limiting for write operations to prevent runaway automation

**Scope for implementation:**
MCP server is a Phase 4 feature. The architectural decision is recorded now so Phase 2-3 work doesn't accidentally create barriers (e.g., tight coupling between Rust state and UI that can't be exposed via MCP). Design IPC commands (DEC25) to be MCP-exposable from the start.

### DEC28 Notes

**Why JSON schemas for documents:**
The account dashboard template (`enterprise-account-dashboard-v2-template.md`) is a 448-line markdown document with 14 major sections. When a meeting ends, the app needs to update "Last Contact", append to "Recent Wins", and insert new actions. Doing this with regex on markdown is fragile (DEC4 rationale). With a JSON schema, it's a structured update:

```json
// Post-enrichment update from CS extension
{
  "account": "Heroku",
  "updates": {
    "quick_view.last_contact": "2026-02-05",
    "critical_information.recent_wins": { "append": { "title": "...", "date": "..." } },
    "critical_information.next_actions": { "append": { "action": "...", "owner": "..." } }
  }
}
```

Rust reads the JSON, applies the update, writes the JSON back. Then optionally regenerates the markdown view.

**What gets a JSON schema (Phase 2-3):**

| Document | Schema Priority | Reason |
|----------|----------------|--------|
| Account dashboard | High | Updated mechanically after every meeting |
| Action items (per account) | High | Bidirectional sync between SQLite and files |
| Success plans | Medium | Updated during quarterly reviews, less frequent |
| Meeting prep output | Already done | DEC4/DEC6 — `_today/data/*.json` |
| Impact capture | Medium | Weekly roll-up needs structured data |

**What does NOT get a JSON schema:**
- Raw transcripts (unstructured by nature)
- Meeting summaries (prose, AI-generated)
- Archive files (historical, read-only — DEC5)
- User notes (freeform, not the system's job to structure)

**Relationship to DEC4:**
DEC4 established JSON-first for `_today/` runtime data. DEC28 extends this to all structured workspace documents that the app updates mechanically. Same principle: JSON for machines, markdown for humans. The difference is that `_today/` JSON is ephemeral (regenerated daily), while account dashboard JSON is persistent (accumulates over time).

**Template → Schema conversion:**
The existing `enterprise-account-dashboard-v2-template.md` sections map to JSON schema objects. The schema lives in `~/.dailyos/schemas/` (or bundled with the CS extension). When creating a new account, Rust generates the JSON from the schema with defaults, then renders the markdown view. Both files live in the account directory:

```
Accounts/Heroku/
├── 01-Customer-Information/
│   ├── dashboard.json          # Machine-readable (app updates this)
│   └── dashboard.md            # Human-readable (generated from JSON)
```

**Migration path:**
Existing markdown dashboards can be parsed into JSON using a one-time migration script (Python, since it's a batch operation). New accounts get JSON from day one. The CS extension's post-enrichment hook reads and writes `dashboard.json`; a watcher or post-write hook regenerates `dashboard.md`.

### DEC29 Notes (PENDING)

**Problem statement:** `prepare_today.py` classifies emails into 3 tiers (high/medium/low), but `deliver_today.py` collapses to 2 (high/normal). Medium-priority emails (internal colleagues, meeting-related) are counted but not individually visible. Users can't review which emails got classified as medium — they're silently buried.

**Current classification rules:**

| Tier | Rule | Examples |
|------|------|---------|
| **High** | Customer domain, account domain hint match, urgency keywords (`urgent`, `asap`, `critical`, `deadline`, `escalation`) | Email from customer@acme.com, subject "URGENT: deployment blocked" |
| **Medium** | Internal domain, meeting-related keywords (`meeting`, `calendar`, `invite`) | Colleague's meeting follow-up, calendar invite |
| **Low** | Newsletter signals (`newsletter`, `digest`, `noreply`, `unsubscribe`), GitHub notifications | GitHub PR notification, marketing digest |

**Proposed three-tier model:**
- **High:** Full card with AI-enriched context (summary, conversation arc, recommended action, action owner). These are the emails that need a response today.
- **Medium:** Compact row in a visible section. Not demanding attention, but reviewable. "Glance at these when you have time."
- **Low:** Archived with a one-line manifest entry. "These were auto-archived. Tap to review the list."

**User-configurable rules (Phase 4+):**
The classification rules are currently hardcoded keyword lists. A future Settings section could expose:
- Custom high-priority domains (beyond auto-detected customer domains)
- Additional urgency keywords
- Allowlist/blocklist for specific senders
- Per-sender priority overrides ("always high-priority from this person")

This aligns with the extension architecture (DEC26) — the CS extension could add customer-specific email rules.

**Decision needed:** Confirm three tiers in JSON pipeline + AI enrichment for high-priority. Then I11 and I12 can proceed.

### DEC30 Notes (PENDING)

**Current state:**
- `/today` generates prep docs for today's eligible meetings (customer, QBR, partnership) during Phase 2.
- `/week` shows a calendar grid with meeting types and prep status, but doesn't generate any prep content.
- Every meeting on the Week page shows "Prep needed" because no prep exists until `/today` runs for that day.

**Proposed model:**

```
Sunday evening:
  /week runs → Phase 1 gathers all meetings for Mon-Fri
             → Phase 2 generates lightweight prep for each eligible meeting
             → Phase 3 writes week-cache/preps/*.json

Each morning:
  /today runs → Phase 1 checks week-cache for existing preps
             → Phase 2 refreshes preps with latest data (new emails, actions, etc.)
             → Phase 3 writes _today/data/preps/*.json (final, up-to-date)
             → Week page prep status updates to "prep_ready"
```

**Key design questions:**
1. **Where does week-cached prep live?** Proposal: `_today/data/week-cache/preps/` — cleared weekly, not daily
2. **How does /today know to refresh vs. create?** Check `week-cache/preps/{meeting_id}.json` exists; if so, pass it as context to Phase 2 for update
3. **Staleness threshold:** If a cached prep is >48h old and the meeting is tomorrow, force a full regeneration
4. **Phase 2 time budget:** Week preps are "lightweight" (context + talking points). Day-of preps are "full" (metrics, stakeholder details, open items). This keeps Sunday's /week run reasonable.

**Trade-offs:**
- **Pro:** User sees "Prep ready" on Wednesday meetings from Monday morning. Reduces daily briefing time by ~30%.
- **Pro:** Running on Sunday = invisible to user (Principle 9: Hide the Plumbing)
- **Con:** Wednesday prep generated Sunday may miss Monday's email thread. The daily refresh mitigates this.
- **Con:** More complex data flow. Two sources of prep truth that need merging.

**Decision needed:** Confirm the week-cache + daily-refresh model. Then define the lightweight vs. full prep schema difference.

### DEC31 Notes (PENDING)

**Problem statement:** Where do actions canonically live? The CLI era had `master-task-list.md`. The current app has SQLite (`actions` table) and daily JSON snapshots (`actions.json`). Neither is clearly the source of truth.

**Current data flow:**
```
prepare_today.py → reads workspace markdown for action mentions
                 → queries SQLite for existing actions
                 → writes actions to directive JSON

deliver_today.py → reads directive
                 → writes _today/data/actions.json (daily snapshot)

App frontend    → reads actions.json for dashboard
                → reads SQLite for Actions page (90-day window)
                → writes completions to SQLite only
```

**The gap:** When a user completes an action in the app, SQLite is updated but the source markdown isn't. When `/today` runs next, it may re-extract the same action from markdown. Deduplication relies on title matching (fragile).

**Proposed model (confirm/adjust):**

| Store | Role | Lifetime |
|-------|------|----------|
| **SQLite** | Working store. App reads/writes here. Fast queries, filtering, completion tracking. | Disposable (DEC18) but persistent across days until rebuilt. |
| **actions.json** | Daily snapshot for dashboard. Generated by deliver_today.py. Read-only for frontend. | Ephemeral — regenerated each briefing. |
| **Markdown files** | Historical record. Source of new action extraction. Post-enrichment hooks write completion markers (`[x]`) back to source files. | Persistent — user-owned files. |

**Key principle:** SQLite is the working view. Markdown is the archive. There is no single `master-task-list.md` — actions are scoped to their source context (meeting prep, email summary, manual entry). The Actions page aggregates across sources.

**What "disposable" means for SQLite (DEC18):** If the SQLite database is deleted, actions can be rebuilt by re-running `/today` and `/week`. In-progress/completed status would be lost — this is the trade-off of disposable cache. A periodic markdown writeback (post-enrichment hooks) mitigates this by recording completions in the durable store.

**Decision needed:** Confirm this model. Then: (1) implement the post-enrichment writeback for action completions, (2) add deduplication logic to prepare_today.py that checks SQLite before extracting from markdown, (3) decide whether manual action creation (from the app UI) writes to SQLite only or also creates a markdown file.

### DEC32 Notes (PENDING)
**Ref:** USER-JOURNEYS.md Cross-Journey (Two-Source Calendar Problem)

**Problem statement:** The app has two sources of calendar data that can disagree:

| Source | Generated | Freshness | Enrichment | Used By |
|--------|-----------|-----------|------------|---------|
| `schedule.json` | Once at briefing time | Stale within hours | AI prep summaries, meeting classification, context | Dashboard meeting timeline |
| `calendar_events` (AppState) | Polled every 5 min | Near-real-time | None | Header meeting count, future live features |

A meeting cancelled at 7 AM still shows on the dashboard (briefing source) but not in the header count (live source). A meeting added at 2 PM shows in the header but not the dashboard.

**Options:**

1. **Briefing-only** (current): Dashboard shows the 6 AM snapshot. Live data is ignored for display purposes. Stale but enriched.
   - Pro: Simple, consistent, already works
   - Con: Cancelled meetings linger, new meetings invisible

2. **Live-only**: Dashboard reads from calendar poller. No enrichment on cards.
   - Pro: Always current
   - Con: Loses the entire value of meeting prep, classification, AI context

3. **Hybrid overlay** (recommended): Live calendar is the source of truth for *which meetings exist*. Briefing enrichment (prep, classification, context) is overlaid onto live events by matching on calendar event ID.
   - If meeting exists in briefing but not live → cancelled, hide it (or show greyed with "Cancelled" badge)
   - If meeting exists in live but not briefing → new, show it bare with "No prep" indicator
   - If meeting exists in both → show live timing + briefing enrichment
   - Pro: Current AND enriched
   - Con: Requires stable event ID matching between Google Calendar API and `prepare_today.py` output. Requires refactoring how the dashboard consumes meeting data.

**Key question:** Does `schedule.json` include Google Calendar event IDs? If not, matching between live and briefing requires fuzzy matching on title + time (fragile).

**Affects:** Journey 2 (Morning Briefing), Journey 3 (Busy Day), Journey 6 (Weekly Planning).

### DEC33 Notes (PENDING)
**Ref:** USER-JOURNEYS.md Cross-Journey (Meeting Card Unification)

**Problem statement:** Meetings exist in three independent forms with no shared state:

| View | Data Source | State Tracked | Linked? |
|------|------------|---------------|---------|
| Daily dashboard card | `schedule.json` | None | No link to detail page (I14) |
| Weekly grid cell | `week-overview.json` | Prep status badge (decorative) | No link to prep |
| Meeting detail page | `preps/*.json` | Full prep content | Orphaned (no entry point) |

Marking prep as "reviewed" on the daily view has no effect on the weekly view. A meeting that's "happening now" on the daily view has no time-aware equivalent on the weekly view.

**Options:**

1. **Unified Meeting entity** (most correct): Single `meetings` table in SQLite. Every meeting has a stable ID (calendar event ID), state (prep status, notes, outcomes), and lifecycle. All views read from the same source.
   - Pro: Consistent everywhere, enables real features (prep tracking, outcome linking)
   - Con: Significant refactoring. Requires DEC32 (calendar source of truth) to be decided first. Changes the data flow for both daily and weekly workflows.

2. **Independent views** (current): Each workflow generates its own meeting data. Views don't share state.
   - Pro: No refactoring needed
   - Con: Fragmented experience, broken flows (I14), decorative-only prep badges

3. **Shared ID mapping** (middle ground): Keep independent data sources but add a lookup table that maps calendar event IDs across views. A "prep reviewed" flag in SQLite can be queried by any view.
   - Pro: Less refactoring than option 1, solves the state-sharing problem
   - Con: Data still generated independently, potential for drift

**Recommendation:** Start with option 3 for near-term (fixes I14, enables prep tracking), plan for option 1 as a Phase 3 architectural goal.

**Depends on:** DEC32 (calendar source of truth determines the ID scheme).

### DEC34 Notes (PENDING)
**Ref:** USER-JOURNEYS.md Journey 3 (Busy Day), Journey 4 (Light Day)

**Problem statement:** The dashboard layout is identical whether the user has 0 meetings or 9. On a busy day, the full timeline is overwhelming and the user needs a "what's next" view. On a light day, the sparse timeline wastes space and the user needs focus/coaching content.

**Options:**

1. **Time-aware adaptive layout**: Dashboard adjusts based on meeting count and current time. Busy day: "now → next → later" with collapsed future meetings. Light day: expanded focus section, action coaching. Between meetings: focused HUD showing next meeting + countdown.
   - Pro: Feels like a personal assistant, serves each day type well
   - Con: Complex to build and test, many layout states, potential for confusing transitions

2. **Static single layout** (current): Same layout always. Works adequately for all day types.
   - Pro: Simple, predictable, fewer bugs
   - Con: Not optimized for any specific scenario, wastes the light-day opportunity

3. **Density hint in overview only** (recommended for near-term): Keep the current layout but make the AI-generated overview density-aware. Busy day: "Packed day — your 9 AM Acme call is the priority." Light day: "Open afternoon — good day to tackle that overdue Globex proposal." The text coaches; the layout stays stable.
   - Pro: Low implementation cost, leverages existing AI enrichment, no layout changes
   - Con: Doesn't solve the "between meetings" UX problem on busy days

**Recommendation:** Option 3 for now. Option 1 is a Phase 3+ aspirational goal. The "between meetings HUD" is a separate feature that could be explored as a tray popover rather than a dashboard layout change.

**Not blocking MVP.** The current static layout works. But this decision should be revisited when post-meeting capture (DEC16, Phase 3) ships, since that's when the "between meetings" UX becomes critical.

---

*Last updated: 2026-02-06*
