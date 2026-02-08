# Product Backlog

Active issues, known risks, assumptions, and dependencies.

**Convention:** Issues use `I` prefix. When an issue is resolved, mark it `Closed` with a one-line resolution. Don't delete — future you wants to know what was considered.

---

## Issues

<!-- Sprint-oriented grouping (2026-02-08, revised after I51/I72/I73/I59 completion):

  TEST BED: ~/Documents/test-workspace/ — clean workspace for end-to-end validation.
  Every sprint milestone is tested here, not in VIP/. See ROADMAP.md for full sprint plan.

  COMPLETED:
    Sprint 1: "First Run to Working Briefing" — I48, I49, I7, I15, I16, I13. 155 tests.
    Sprint 2: "Make it Smarter" — I42, I43, I41, I31. 168 tests.
    Sprint 3: "Make it Reliable" — I39, I18, I20, I21, I37, I6. 176 tests.
    Sprint 4a: "Entity Intelligence" — I51 (people), I72+I73 (account dashboards),
      I59 (script paths), I56 (onboarding 80%), demo data expansion. 189 tests.

  ═══════════════════════════════════════════════════════════════════
  SPRINT 5: "Ship It" — 4 parallel tracks
  ═══════════════════════════════════════════════════════════════════

    Track A — Onboarding (sequential):
      ✅ I56 finish (wire PopulateWorkspace chapter to Tauri commands) — DONE
      ✅ I57 (populate workspace — accounts/projects + userDomain, ship blocker) — DONE
      I78 (inbox-first behavior training — inbox drop chapter between I57 and dashboard tour, ship blocker)
      I79 (Claude Code validation/installation step, ship blocker)
      I58 (user profile context into enrichment prompts, depends on I57 profile fields)

    Track B — Security & Stability (parallel, independent fixes):
      I60 (path traversal in inbox/workspace commands, ship blocker)
      I62 (.unwrap() panics crash background tasks, ship blocker)
      I64 (non-atomic file writes — config corruption on force quit, ship blocker)
      I63 (script timeout enforcement — hangs forever)
      I65 (impact log append race condition)

    Track C — Distribution (independent, ship blocker):
      I8 (DMG, notarization, updater — ship blocker, no update path without it)

    Track D — Polish (independent, small):
      I25 (meeting badge/status unification)

    Done when: DMG installs cleanly, onboarding → first briefing works,
    7-day crash-free validation on test-workspace.

  ═══════════════════════════════════════════════════════════════════
  SPRINT 6: "Harden" — All parallel, small independent fixes
  ═══════════════════════════════════════════════════════════════════

    I66 (deliver_preps safe writes — don't clear before writing)
    I67 (scheduler boundary miss — widen window)
    I69 (file router overwrites duplicates — append suffix)
    I70 (sanitize_account_dir unsafe chars)
    I61 (TOCTOU race in transcript immutability)
    I71 (assorted edge hardening batch — 9 items)
    I19 (AI enrichment failure badge)

  ═══════════════════════════════════════════════════════════════════
  SPRINT 7: "Enrich & Protect" — 3 parallel tracks
  ═══════════════════════════════════════════════════════════════════

    Track A — Intelligence:
      I80 (proposed agenda in meeting prep — AI-synthesized agenda from prep data)
      I81 (people dynamics in meeting prep UI — render attendeeContext)
      I74 (account enrichment via Claude Code websearch)
      I55 (Executive Intelligence — decision framing, delegation tracking)

    Track B — Durability (ADR-0048):
      I76 (SQLite backup + rebuild-from-filesystem)
      I77 (filesystem writeback audit)
      I75 (external edit detection + reconciliation)

    Track C — Performance:
      I68 (Mutex → RwLock for read-heavy AppState fields)

  ═══════════════════════════════════════════════════════════════════
  PARKING LOT (post-ship, needs real usage data)
  ═══════════════════════════════════════════════════════════════════

    Entity-mode architecture (ADR-0046, I27 umbrella):
      I50 (projects table), I52 (meeting-entity M2M), I53 (entity-mode config)
      I54 (MCP integration framework), I28 (MCP server + client)
      I29 (non-entity structured document schemas)
    Kits: I40 (CS Kit)
    Intelligence: I35 (ProDev Intelligence)
    Research: I26 (web search for unknown meetings)
    Low: I2, I3, I4, I10
-->

### Open — Medium Priority

**I8: No app update/distribution mechanism** — Ship blocker
Options: Tauri's built-in updater, GitHub Releases + Sparkle, manual DMG, Mac App Store. Needs Apple Developer ID for notarization. Without this, no update path to users — ship blocker.

**I9: Focus page and Week priorities are disconnected stubs** — Closed
Both `FocusPage.tsx` and `WeekPage.tsx` are fully implemented: data loading, workflow execution with progress tracking, meeting cards, time blocks, action summaries, priority rendering. Not stubs anymore. Closed Sprint 4a.

**I40: CS Kit — account-mode fields, templates, and vocabulary**
ADR-0046 replaces the CS extension with a CS Kit (entity-mode-specific overlay). What remains CS-specific after ADR-0043 narrowed extensions: CS account fields (ARR, renewal dates, health scores, ring classification), account dashboard templates, success plan templates, value driver categories, ring-based cadence thresholds, Google Sheets sync (Last Engagement Date writeback). CRM data sources (Clay, Gainsight, Salesforce) are now integrations (I54), not Kit responsibilities. The existing `accounts` table IS the CS Kit's schema contribution — it carries CS-specific fields on top of the universal `entities` table. Kit also contributes enrichment prompt fragments for CS vocabulary (value delivery moments, renewal signals, health indicators). Remaining work: formalize Kit registration, schema contribution mechanism, template system, prompt fragment composition. ADR-0047 defines the entity dashboard architecture — the CS Kit contributes the `structured` fields (ARR, health, ring, renewal, csm, champion) to the account `dashboard.json` schema, and CS-specific sections (commercial summary, renewal strategy) as `customSections` entries. Without the CS Kit enabled, account dashboards show generic company overview + stakeholders + activity. With the Kit, they add the commercial lens. Blocked by I27 umbrella. Reference: `~/Documents/VIP/.claude/skills/daily-csm/`.

**I25: Unify meeting badge/status rendering**
MeetingCard has 5 independent status signals (isCurrent, hasPrep, isPast, overlayStatus, type) each with their own conditional. Consolidate into a computed MeetingDisplayState. Relates to ADR-0033.

**I56: Onboarding redesign — teach the philosophy, not just configure settings**
Current I13 onboarding wizard is a config flow (entity mode → workspace → Google → generate briefing). "Generate First Briefing" is broken by design: a new user has no files, no transcripts, no data — there's nothing for AI to process. More fundamentally, the wizard treats onboarding as setup when it should be education and delight. A first-timer doesn't know what they don't know — this is our opportunity to teach the *why*, not just the *what*. Required content: calendar connection context (what DailyOS does with your calendar), email connection context (how triage works), anatomy of the dashboard (what each section means), where actions are sourced and how they flow, how emails are presented and prioritized, the meeting card lifecycle (prep → current → outcomes), best practices for enabling prep, workspace folder structure rationale (`_today/`, `_inbox/`, `_archive/`, `Accounts/`, `Projects/`), how to use inbox (drop files in, system processes them). Should replace the current "Generate First Briefing" step with something meaningful — either seed content to demonstrate with, or guided walkthrough of a mock dashboard. Supersedes I13's implementation (I13 remains Closed as the config mechanics are correct; this issue addresses the UX layer above them).

**Status: DONE.** OnboardingFlow.tsx with 9-chapter educational flow complete. All Tauri commands wired (install_demo_data, populate_workspace, set_user_profile). Demo data fixtures operational. PopulateWorkspace chapter connected to backend. Closed Sprint 5.

**I57: Onboarding: add accounts/projects — populate workspace before first briefing**
The first real briefing is only as good as the data in the workspace. Without account/project folders, meeting-entity association fails — meetings appear but with zero context (no talking points, no risks, no history). This step sits between the Dashboard Tour and the Ready screen in the onboarding flow.

**What it collects (entity-mode-aware):**
- Account-based: "Add your key accounts" — name + optional notes. Creates `Accounts/<Name>/` folders.
- Project-based: "Add your active projects" — name + optional notes. Creates `Projects/<Name>/` folders.
- Both: both inputs.
- All modes: user's own email domain (e.g. `@mycompany.com`) to distinguish internal vs external attendees.

**Domain inference:** Instead of asking users to manually enter customer domains, AI infers domains from account names (e.g. "Acme Corp" → `acme.com`, `acmecorp.com`) and confirms. This aligns with Principle 6 (AI-native, not AI-assisted). The user's own domain is the only required manual input — everything else can be inferred and corrected.

**What it creates:**
- Workspace folders: `Accounts/<Name>/` or `Projects/<Name>/`
- SQLite entity records (via `upsert_account` / future `upsert_project`)
- Domain hints for meeting-entity association
- Config: `userDomain` field in `~/.dailyos/config.json`

**Why it matters:** With even 3-5 account/project folders, the first briefing shows "Acme Corp Quarterly Sync → *Acme Corp*" instead of a disconnected meeting. That association is the minimum bar for the briefing to feel intelligent. Without it, the product fails Principle 2 (Prepared, Not Empty).

**Copy for the chapter:**
- h2 (account): "Add your accounts"
- h2 (project): "Add your projects"
- h2 (both): "Add your accounts and projects"
- Subhead: "These are the companies, clients, or initiatives you work with most. DailyOS uses them to connect your meetings to the right context."
- Input: simple name field + "Add" button, list of added items with remove
- Domain section: "Your email domain" — single field, e.g. `mycompany.com`. "This helps DailyOS distinguish your internal meetings from external ones."
- Minimum: 1 entry required (soft gate — "Add at least one to get started, you can always add more in Settings")
- Maximum: no limit, but prompt suggests "start with 3-5"
- Footer: "You can add more anytime from Settings."

**Status: DONE.** `populate_workspace` command creates folders + upserts accounts. `set_user_profile` saves userDomain. PopulateWorkspace.tsx chapter wired. Closed Sprint 5.

**I78: Onboarding: teach inbox-first behavior as the paradigm shift**
The number one way DailyOS becomes useful is when users feed it context about their work. The current onboarding teaches *setup* (connect Google, add accounts). But the real paradigm shift is behavioral: users are trained by every other productivity app to *manage* — DailyOS flips that script to "drop things in, intelligence comes out." Inbox is the purest expression of this, and onboarding should train that muscle memory.

**Proposed flow revision (refines I56/I57):**
1. Tell us about your accounts/projects (I57 — gives the system entities to link against)
2. **Inbox training:** guided first inbox drop — user drops a transcript, meeting notes, or document. System kicks off processing in the background. Visual progress sequence (similar to `/week` setup) shows the system is working — file received, classifying, extracting, linking. User doesn't wait for enrichment to finish.
3. Dashboard tour with demo data (I56 — reliable, curated, teaches the UI). Demo data is the teaching tool; the inbox drop is the behavior training. Two different jobs.
4. **Ready chapter gains a processing summary:** before exiting onboarding, a compact summary of what the background inbox processing found/is doing. "We found 3 action items and linked this to Acme Corp" or "Still processing — check your inbox page in a few minutes." Complements the existing Ready chapter.
5. Drop into the real app — "add to inbox" is already established behavior.

**Key insight:** Don't expect enrichment to complete during onboarding. The demo data drives the dashboard tour (reliable). The inbox drop trains the *behavior*. The summary at the end ties the two together — the user sees that what they dropped is already becoming intelligence.

**Key tension:** Inbox processing quality depends on having entities to link against (chicken-and-egg). Sequencing accounts *before* the inbox drop solves this — the system has context, so the first drop produces a good result, not a cold one.

**What this changes:** Doesn't replace I56/I57 — refines the sequencing and adds an inbox training chapter between I57 (populate workspace) and the dashboard tour. Extends the Ready chapter with a processing summary. Demo data remains the primary teaching tool for the dashboard. **Ship blocker** — without this, users don't learn the core AI-native behavior that makes DailyOS useful.

Relates to I56, I57. Post-ship refinement (current onboarding flow ships first, this improves it).

**I79: Onboarding: Claude Code validation and installation step**
Claude Code is the AI engine — without it installed and authenticated, enrichment produces nothing (briefing narrative, email triage, inbox processing, transcript insights all fail silently). This is a hard dependency that should be validated during onboarding, not discovered when the first briefing comes back flat.

**What the step needs to do:**
- Detect whether Claude Code CLI is installed (check PATH / known install locations)
- Detect whether it's authenticated (API key or login session valid)
- If missing: guide installation with platform-appropriate instructions (macOS: brew, npm, direct download)
- If installed but not authenticated: guide auth flow
- If both valid: green checkmark, move on

**UX considerations:**
- This is a technical dependency that non-technical users may not understand. Frame it as "Connect your AI" — parallel to "Connect Google." The user doesn't need to know it's a CLI tool.
- Should be skippable (like Google Connect) with a clear warning: "Without this, DailyOS can organize your day but can't provide AI insights."
- Placement: near Google Connect in the onboarding flow — both are "connect external dependencies" steps.

**Existing code:** `PtyManager::is_claude_authenticated()` in `pty.rs` already checks auth status. `pty.rs` spawns Claude Code subprocesses for enrichment. Detection logic exists but isn't surfaced to the user.

Relates to I56. **Ship blocker** — without Claude Code, the product's core promise ("AI-native daily productivity") doesn't work.

**I80: Proposed Agenda in meeting prep — Resolved.**
Mechanical agenda generation in `generate_mechanical_agenda()` (deliver.rs) assembles structured agenda from prep data: overdue items first, then risks (limit 2), talking points (limit 3), questions (limit 2), non-overdue open items (limit 2), capped at 7 items. AI enrichment via `enrich_preps()` refines ordering, adds "why" rationale, and incorporates people dynamics — follows `enrich_emails()` fault-tolerant pattern (AI failure leaves mechanical agenda intact). `AgendaItem` type added across Rust (`types.rs`), JSON loader (`json_loader.rs`), and TypeScript (`types/index.ts`). "Proposed Agenda" card renders as first card on prep page with numbered items, source badges, and copy button. Demo data updated for all 3 fixtures. Prep page restructured: Agenda → Quick Context → People → Reference Material (pyramid principle). 199 tests passing.

**I81: People dynamics in meeting prep UI — Resolved.**
Replaced flat "Key Attendees" card with rich "People in the Room" component in `MeetingDetailPage.tsx`. Renders `attendeeContext` data (already computed server-side) with: temperature badges (hot/warm/cool/cold with color coding), meeting count, last seen date, organization, notes excerpt, "New contact" flags, cold-contact warnings, and links to `/people/$personId`. Falls back gracefully to simple name/role/focus display when no `attendeeContext` exists (internal meetings). Copy button with `formatAttendeeContext()` formatter. Pure frontend — no backend changes needed.

**I82: Copy-to-clipboard for meeting prep page** — Resolved.
Added copy-to-clipboard support to `MeetingDetailPage.tsx`. "Copy All" outline button in the header bar exports the full prep as clean markdown (title, time, all sections with `## Heading` separators). Per-section `<CopyButton>` in each `CardHeader` copies individual sections. Output uses light markdown (bullets, numbered lists, key-value lines) that renders well in Slack and reads cleanly as plaintext in email/docs. Icon transitions from clipboard to green check on copy (2s auto-reset), no toast. Reusable `useCopyToClipboard` hook (`src/hooks/useCopyToClipboard.ts`) and `CopyButton` component (`src/components/ui/copy-button.tsx`) available for future extension to Overview, Outcomes, etc. No backend changes, no new dependencies.

**I26: Web search for unknown external meetings not implemented**
ADR-0022 specifies proactive research via local archive + web for unknown meetings. Local archive search works in `ops/meeting_prep.py`. Web search does not exist. Likely a Phase 2 task — Claude can invoke web search during enrichment (Phase 2). Low urgency since archive search provides some coverage.

**I27: Entity-mode architecture — umbrella issue**
ADR-0046 replaces profile-activated extensions (ADR-0026) with three-layer architecture: Core + Entity Mode + Integrations. Entity mode (account-based, project-based, or both) replaces profile as the organizing principle. Integrations (MCP data sources) are orthogonal to entity mode. Two overlay types: **Kits** (entity-mode-specific: CS Kit, Sales Kit) contribute fields + templates + vocabulary; **Intelligence** (entity-mode-agnostic: Executive, ProDev) contribute analytical perspective via enrichment prompt fragments. Sub-issues: I50 (projects table), I51 (people table), I52 (meeting-entity M2M), I53 (entity-mode config/onboarding), I54 (MCP integration framework), I55 (Executive Intelligence). Current state: `entities` table and `accounts` overlay exist (ADR-0045), bridge pattern proven. Post-Sprint 4.

**I28: MCP server and client not implemented**
ADR-0027 accepts dual-mode MCP (server exposes workspace tools to Claude Desktop, client consumes Clay/Slack/Linear). ADR-0046 elevates MCP client to the integration protocol — every external data source (Gong, Salesforce, Linear, etc.) is an MCP server consumed by the app. IPC commands are designed to be MCP-exposable (good foundation from ADR-0025). No MCP protocol code exists. Server side exposes DailyOS tools; client side is the integration layer. See I54 for client framework.

**I29: Structured document schemas not implemented**
ADR-0028 accepts JSON-first schemas for account dashboards, success plans, and structured documents (`dashboard.json` + `dashboard.md` pattern). Briefing JSON pattern exists as a template. Account dashboard UI is a stub. No schema validation system. Less coupled to extensions post-ADR-0046 — core entity schemas are universal, domain overlays contribute additional fields. ADR-0047 refines this for entity dashboards specifically: two-file pattern (JSON write interface + markdown read artifact), three-way sync (JSON ↔ SQLite ↔ markdown), external edit detection. Entity dashboard implementation is I73. Non-entity structured documents (success plans, etc.) remain in this issue's scope. Blocked by I27 umbrella for overlay-contributed schemas.

**I50: Projects overlay table and project entity support**
ADR-0046 requires a `projects` overlay table parallel to `accounts`. Fields: id, name, status, milestone, owner, target_date. Bridge pattern: `upsert_project()` auto-mirrors to `entities` table (same mechanism as `upsert_account()` → `ensure_entity_for_account()`). CRUD commands: `upsert_project`, `get_project`, `get_projects_by_status`. Frontend: Projects page (parallel to Accounts page), project entity in sidebar for project-based and both modes. Blocked by I27.

**I51: People sub-entity table and entity-people relationships**
ADR-0046 establishes people as universal sub-entities. Create `people` table (id, name, email, organization, role, last_contact) and `entity_people` junction (entity_id, person_id, relationship_type). People are populated from: meeting attendees (automatic), CRM integrations (I54), manual entry. Enriches meeting prep with stakeholder context (interaction history, relationship signals). Population strategy: attendee-seeded on first briefing, CRM-enriched when integrations are connected, user-correctable. Blocked by I27.

**I52: Meeting-entity many-to-many association**
Replace `account_id` FK on `meetings_history`, `actions`, `captures` with `meeting_entities` junction table. Enables meetings to associate with multiple entities (an account AND a project). Deferred explicitly from ADR-0045 to I27. Migration: existing `account_id` values become rows in `meeting_entities`. Association logic: account-based uses domain matching (existing), project-based uses integration links + AI inference + manual correction. Blocked by I50 (projects must exist first).

**I53: Entity-mode config, onboarding, and UI adaptation**
Replace `profile` config field with `entityMode` (account | project | both) + `integrations` + `domainOverlay`. Update onboarding: entity-mode selector ("How do you organize your work?") → integration checklist → optional role shortcut. Update sidebar to render Accounts and/or Projects based on entity mode. Update dashboard portfolio attention to compute signals for active entity types. Migration: `profile: "customer-success"` → `entityMode: "account"` + `domainOverlay: "customer-success"`. `profile: "general"` → `entityMode: "project"`. Blocked by I50, I52.

**I54: MCP client integration framework**
Build MCP client infrastructure in Rust for consuming external data sources per ADR-0046 and ADR-0027. Requirements: auth flow per integration (OAuth where needed), sync cadence configuration, error handling and retry, integration settings in Settings page. Start with one integration per category to prove the pattern: one transcript source (Gong or Granola), one CRM (Salesforce), one task tool (Linear). Each integration is an MCP server the app consumes — community can build new ones without touching core. Evolves I28 (MCP client side). Blocked by I27.

**I35: ProDev Intelligence — personal impact capture and career narrative**
ADR-0046 classifies ProDev as an Intelligence layer (entity-mode-agnostic). Works with any entity mode — personal impact is orthogonal to how work is organized. ADR-0041 establishes scope: daily end-of-day reflection prompt ("What did you move forward today?"), weekly narrative summary, monthly/quarterly rollup for performance reviews. Contributes enrichment prompt fragments: "Capture personal impact — what did the user demonstrate, influence, or move forward?", "Identify skill demonstrations and career-narrative-worthy moments", "Note cross-functional contributions and leadership signals." Distinct from CS outcomes (which are captured via transcripts and post-meeting prompts). Blocked by overlay registration + prompt fragment mechanism (I27). Reference: `/wrap` "Personal Impact" section, `/month`, `/quarter`.

**I55: Executive Intelligence — decision framing, delegation tracking, and strategic analysis**
ADR-0046 classifies Executive as an Intelligence layer (entity-mode-agnostic). Works with any entity mode — a CS leader, engineering director, or consultant all benefit. Contributes enrichment prompt fragments: decision quality assessment (SCQA framing, reversibility/stakes), delegation tracking ("WAITING ON" with staleness detection), time protection (cancelable meeting identification), political dynamics (stakeholder alignment, power shifts), noise filtering (what doesn't need attention today). Draws from `/cos` (decision surfacing, delegation tracking), `strategy-consulting` (analytical frameworks: SCQA, WWHTBT, options analysis), and `/veep` (political intelligence, relationship temperature). Manifests as: dashboard intelligence signals, meeting prep with political context, post-meeting decision quality assessment, delegation staleness alerts. Blocked by I27 umbrella + prompt fragment mechanism.

**I58: Feed user profile context into AI enrichment prompts**
User profile (name, company, title, focus) collected during onboarding should be injected into AI enrichment operations to personalize output. Scope: enrich_emails(), enrich_briefing(), meeting prep directives. The profile fields live in config.json (user_name, user_company, user_title, user_focus) and are set via the "About You" onboarding chapter or future Settings page. Depends on I57 (profile collection UI).

**I72: Entity dashboard pages — list + detail views for accounts and projects**
ADR-0047 establishes the two-file pattern (JSON + markdown). This issue is the actual UI: account list page (table with health/ARR/ring/renewal/last contact/open actions, sortable and filterable) + account detail page (composite dashboard: structured fields from SQLite, narrative content from `dashboard.json`, live data from SQLite queries). Project list page and project detail page follow the same pattern with different field sets (status/milestone/owner/target date instead of ARR/health/ring/renewal). The detail page is a card-based layout — not a rendered markdown document. Each card maps to a data source: Quick Context (SQLite), Company Overview (dashboard.json, refreshable), Key Stakeholders (people table from I51), Strategic Programs (dashboard.json, editable), Recent Activity (SQLite live query), Open Items (SQLite live query), Intelligence Signals (intelligence.rs live), Notes (dashboard.json, editable). Account side can ship with existing `accounts` table. Project side depends on I50 (projects table). Both sides depend on I73 (template system) for the JSON schema and markdown generation.

**I73: Entity dashboard template system — JSON schema, in-app editing, markdown generation, file watching**
ADR-0047 defines the data model. This issue implements: (1) JSON schema types in Rust (`AccountDashboard`, `ProjectDashboard` with `structured`, `companyOverview`, `strategicPrograms`, `notes`, `customSections` fields). (2) Read/write commands: `get_entity_dashboard`, `update_entity_dashboard` (reads/writes `dashboard.json`). (3) `render_entity_dashboard_md()` function that combines JSON content + SQLite live data into comprehensive markdown. (4) Regeneration triggers: after briefing delivery, after meeting capture, after in-app edits, after enrichment, after external JSON change detected. (5) File watching for `dashboard.json` changes (mtime-based, checked on entity access). (6) Three-way sync bridge: JSON `structured` fields ↔ SQLite `accounts`/`projects` tables. JSON is canonical — if they disagree, JSON wins on next sync. Markdown is always generated output. Refines I29 (structured document schemas) for entity-specific use.

**I74: Account enrichment via Claude Code websearch**
On account creation or on-demand refresh, spawn Claude Code to websearch company name and populate `dashboard.json` company overview. Flow: (1) `enrich_account()` spawns Claude Code with `--print` and a structured prompt: "Research [company name]. Return JSON with description, industry, company size, headquarters, and 3-5 key public facts." (2) App parses response, writes to `dashboard.json` `companyOverview` section with `enrichedAt` timestamp. (3) App regenerates `dashboard.md`. (4) Fault-tolerant: enrichment failure leaves empty overview card, not a broken page (per ADR-0042 pattern). Same PTY infrastructure as email/briefing enrichment. Refresh button on account detail page triggers re-enrichment. Could also infer email domains for meeting association (helpful for I57 onboarding). On-demand only in v1 — no scheduled enrichment. Future: enrich accounts with meetings today as part of prep generation for high-ring accounts.

**I76: SQLite durability — backup strategy and rebuild-from-filesystem command**
ADR-0048 establishes SQLite as a working store, not a disposable cache. This requires: (1) **Periodic backup:** copy `~/.dailyos/actions.db` to a backup location (e.g., `~/.dailyos/backup/actions-YYYY-MM-DD.db`) on a schedule — daily before archive workflow is natural. Use SQLite backup API for consistency. Keep last N backups (configurable, default 7). (2) **`rebuild_database` command:** reconstructs SQLite from workspace filesystem when the database is lost or corrupted. Reads `Accounts/*/dashboard.json` → rebuilds accounts + entities tables. Reads `_archive/*/day-summary.json` → rebuilds partial meeting history. Reads `_archive/*/actions/` and briefing JSON → rebuilds actions (without completion state for actions not written back). Cannot rebuild: AI-extracted captures (requires re-running transcript processing), processing history, computed stakeholder signals. The command reports what it recovered and what's missing. (3) **Corruption detection:** on startup, run `PRAGMA integrity_check`. If corrupt, offer to restore from backup or rebuild from filesystem.

**I77: Filesystem writeback audit — ensure important SQLite state reaches files**
ADR-0048 requires that important data eventually reaches the filesystem. Audit all SQLite tables and ensure writeback paths exist:

| Table | Writeback exists? | Path | Gap |
|-------|------------------|------|-----|
| `actions` | Yes | `hooks.rs` `sync_completion_to_markdown` | Covers completion markers; priority/edit state not written back |
| `captures` | Partial | `impact_rollup.rs` writes weekly impact file | Only wins/risks; decisions not included. Individual capture edits (I45) are SQLite-only |
| `accounts` | Yes | `accounts.rs` `write_json` + `write_markdown` (I73) | Full ADR-0047 two-file pattern with three-way sync |
| `entities` | Yes | Mirrored via account/people bridge pattern | Entity table is derived from overlay tables |
| `meetings_history` | Partial | `reconcile.rs` writes `day-summary.json` | Outcomes included; full meeting record not archived |
| `processing_log` | No | — | Low priority — operational metadata, acceptable loss |
| `people` | Yes | `people.rs` `write_person_json` + `write_person_markdown` | Full ADR-0047 two-file pattern with entity link durability (ADR-0048) |

Priority gaps to close: (1) Action priority/edit state → extend `sync_completion_to_markdown` or add a separate writeback hook. (2) Capture edits → decide if inline edits should write back to transcript source or only live in SQLite.

**I75: Entity dashboard external edit detection and reconciliation**
ADR-0047 specifies that external tools should write to `dashboard.json` (the write interface), but some will edit `dashboard.md` directly. This issue implements: (1) Change detection: track `last_generated_at` timestamp per entity dashboard. On entity access, compare markdown file mtime — if newer, show "externally modified" indicator. (2) JSON change detection: if `dashboard.json` mtime is newer than app's last read, re-read JSON, sync structured fields to SQLite, regenerate markdown. This is the happy path (external tool followed the protocol). (3) Markdown reconciliation (future/stretch): when markdown was edited directly, user can trigger AI-powered reconciliation — Claude reads the markdown diff, extracts changes, applies to JSON, regenerates markdown. Without reconciliation, next regeneration overwrites external markdown changes (with warning). (4) Conflict resolution UI: when both JSON and markdown have external changes, show diff and let user choose. Depends on I73 (template system). The JSON detection path (step 2) is the priority — it handles the recommended external write flow. Markdown reconciliation (step 3) is a stretch goal.

**I59: `CARGO_MANIFEST_DIR` makes Python scripts unfindable in release builds**
`executor.rs:820` and `google.rs:264` use `env!("CARGO_MANIFEST_DIR")` to locate Python scripts. This macro bakes the developer's local filesystem path at compile time. In a production DMG distributed to other machines, the path won't exist and all Phase 1 script execution fails with "Script not found." Fix: use Tauri's resource resolver or bundle scripts, falling back to `CARGO_MANIFEST_DIR` only under `cfg!(debug_assertions)`. Sprint 4 ship blocker.

**I60: Path traversal in inbox processing and workspace population commands**
`process_inbox_file` and `enrich_inbox_file` accept an arbitrary `filename` string without validating it stays within `_inbox/`. A filename like `../../.dailyos/config.json` causes the processor to read/move/delete files outside the inbox. `get_inbox_file_content` already has the correct `starts_with` guard — extract into a shared `validate_inbox_filename()` and apply to all three commands. Separately, `populate_workspace` passes user-provided names directly to `workspace.join("Accounts").join(name)` — a name containing `../` could escape the workspace. Validate names contain no path separators or `..`. Sprint 4 ship blocker (security). QA ref: F5, F25.

**I61: TOCTOU race in transcript immutability check**
`attach_meeting_transcript` checks `transcript_processed` under a lock, drops the lock, then does async processing, then records the result. Two concurrent calls for the same meeting can both pass the check. Fix: insert a sentinel value (e.g., a `TranscriptRecord` with status "processing") into the map before releasing the lock. Remove sentinel on failure. QA ref: F1.

**I62: `.unwrap()` panics in JSON mutation paths crash background tasks**
`workflow/deliver.rs` has 5 instances of `.as_object_mut().unwrap()` on `serde_json::Value` loaded from prep/schedule/email JSON files. If any file is malformed (corrupted write, truncated), this panics. In `google.rs`, the same pattern crashes the calendar poller (a `tokio::spawn` task that dies silently). Replace with `if let Some(obj) = val.as_object_mut()` or `.ok_or()` with graceful skip + warning log. Sprint 4 ship blocker (crashes). QA ref: F6.

**I63: `run_python_script` ignores `timeout_secs` parameter — scripts can hang forever**
`pty.rs:172` accepts `timeout_secs` but calls `cmd.output()` which blocks indefinitely. If a Python script hangs on a network call (Google API timeout, DNS failure), the executor thread is blocked forever. The PTY manager correctly implements timeout via channels — `run_python_script` should use `spawn()` + `wait_timeout` or similar. QA ref: F26.

**I64: Non-atomic file writes risk corruption on crash**
`state.rs` (config), `impact_rollup.rs`, `commands.rs` (impact log), and Python `config.py` all use direct `fs::write()` / `path.write_text()`. If the process is killed mid-write (force quit, power loss), files are left truncated. For `config.json`, this means the app cannot start. Fix: write to `.tmp` then `fs::rename()` (atomic on same filesystem). QA ref: F2, F17.

**I65: Impact log append uses read-modify-write instead of atomic append**
`capture_meeting_outcome` and `append_to_impact_log` in `transcript.rs` read existing content, concatenate, and write back. Two simultaneous captures (two meetings ending at the same time) race — one write overwrites the other's append. Fix: use `OpenOptions::new().append(true).create(true)` followed by `write_all()`. QA ref: F3.

**I66: `deliver_preps` clears existing preps before writing new ones**
`workflow/deliver.rs` removes all `*.json` from `preps/` before writing new ones. If the write fails partway (disk full, permission error), the user loses preps with no recovery. Fix: write new preps to temp names first, then remove old and rename, or use a swap directory. QA ref: F13.

**I67: Scheduler `should_run_now` window can miss jobs near boundary**
The poll loop sleeps 60s but the forward window check is `diff < 60`. If system load delays the poll to 61s after the scheduled minute, the job is missed until the 2-hour grace period catches it. Fix: widen to `diff < 120` or use a `last_check` comparison instead of absolute windows. QA ref: F14.

**I68: `Mutex` contention on read-heavy `AppState` fields**
All 11 `AppState` fields use `std::sync::Mutex`. Read-heavy fields (`config`, `calendar_events`, `google_auth`) take exclusive locks on every IPC command, serializing concurrent dashboard component polls. Fix: replace with `RwLock` for read-heavy fields to allow concurrent reads. QA ref: F19.

**I69: File router silently overwrites duplicate destinations**
`processor/router.rs` uses copy-then-delete without checking if the destination already exists. Two files with the same name routed to the same account directory — the first is silently overwritten. Fix: check existence and append a date/sequence suffix. QA ref: F9.

**I70: `sanitize_account_dir` doesn't strip filesystem-unsafe characters**
`processor/transcript.rs` converts account names to title case for directory names but doesn't strip `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`. An account named `Acme/Corp` creates nested directories instead of a single folder. Fix: strip or replace filesystem-unsafe characters, or use `slugify()` for the directory component. QA ref: F22.

### Open — Low Priority

**I2: Compact meetings.md format for dashboard dropdowns**
Archive contains a compact format with structured prep summaries. Could be useful for quick-glance meeting cards. Post-MVP.

**I3: Browser extension for web page capture to _inbox/**
Chromium extension for page capture to markdown in `_inbox/`. Aligns with "system does the work." Post-Phase 2 when inbox processing is stable.

**I4: Motivational quotes as personality layer**
Viable placements: overview greeting (daily rotating), empty states ("you crushed it"). Rejected approach: welcome interstitial (adds required click, violates Principle 2).

**I10: No shared glossary of app terms**
Overlapping terms (briefing, workflow, capture, focus, etc.) used inconsistently. Needs shared definitions in DEVELOPMENT.md or a GLOSSARY.md.

**I71: Assorted low-severity edge hardening**
Batch of minor issues from QA audit, none individually blocking but worth addressing for robustness: (1) Unbounded loop in `copy_to_inbox` duplicate naming — add upper bound or use UUID suffix (F4). (2) `google_token_path()` uses `unwrap_or_default()` on `home_dir()` — returns empty PathBuf if HOME unset (F10). (3) Config write doesn't validate workspace path still exists (F11). (4) Regex compiled on every call in `make_meeting_id` — use `OnceLock` like `metadata.rs` (F12). (5) `process_file` gives confusing UTF-8 error on binary files — check extension first (F15). (6) Reconciliation inbox flag logic only flags first unprocessed file — aggregate into one flag with count (F16). (7) Calendar merge time sort needs test coverage for "9:00 AM" < "10:00 AM" ordering (F18). (8) No length limit on `update_capture` content (F20). (9) Execution history 100-entry cap silently truncates — no UI indication (F24).

**I19: AI enrichment failure not communicated to user**
When Phase 2 fails, briefing renders thin with no indication. Recommended: quiet "AI-enriched" badge (absence = not enriched). Fits Principle 9.

### Closed

**I1: Config directory naming** — Resolved. Renamed `.daybreak` → `.dailyos`.

**I5: Orphaned pages (Focus, Week, Emails)** — Resolved. All three now have defined roles: Focus = drill-down from dashboard, Week = sidebar item (Phase 2+), Emails = drill-down from dashboard. See ADR-0010.

**I11: Phase 2 email enrichment not fed to JSON** — Resolved. `deliver_today.py` gained `parse_email_enrichment()` which reads `83-email-summary.md` and merges into `emails.json`.

**I12: Email page missing AI context** — Resolved. Email page shows summary, recommended action, conversation arc per priority tier. Removed fake "Scan emails" button.

**I14: Dashboard meeting cards don't link to detail page** — Resolved. MeetingCard renders "View Prep" button linking to `/meeting/$prepFile` when prep exists. Added in Phase 1.5.

**I17: Post-meeting capture outcomes don't resurface in briefings** — Resolved (actions side). Non-briefing actions (post-meeting, inbox) now merge into dashboard via `get_non_briefing_pending_actions()` with title-based dedup. Wins/risks resurfacing split to I33.

**I22: Action completion doesn't write back to source markdown** — Resolved. `sync_completion_to_markdown()` in `hooks.rs` runs during post-enrichment hooks. Queries recently completed actions with `source_label`, writes `[x]` markers back to source files. Lazy writeback is acceptable — SQLite is working store, markdown is archive.

**I24: schedule.json meeting IDs are local slugs, not Google Calendar event IDs** — Resolved. Added `calendarEventId` field alongside the local slug `id` in both `schedule.json` and `preps/*.json`. Local slug preserved for routing/filenames; calendar event ID available for cross-source matching (ADR-0032, ADR-0033).

**I30: Inbox action extraction lacks rich metadata** — Resolved. Added `processor/metadata.rs` with regex-based extraction of priority (`P1`/`P2`/`P3`), `@Account`, `due: YYYY-MM-DD`, `#context`, and waiting/blocked status. Both inbox (Path A) and AI enrichment (Path B) paths now populate all `DbAction` fields. AI prompt enhanced with metadata token guidance. Title-based dedup widened to prevent duplicate pending actions. Waiting actions now visible in dashboard query.

**I34: Archive workflow lacks end-of-day reconciliation** — Resolved. Added `workflow/reconcile.rs` with mechanical reconciliation that runs before archive: reads schedule.json to identify completed meetings, checks transcript status in `Accounts/` and `_inbox/`, computes action stats from SQLite, writes `day-summary.json` to archive directory and `next-morning-flags.json` to `_today/` for tomorrow's briefing. Pure Rust, no AI (ADR-0040).

**I23: No cross-briefing action deduplication** — Resolved. Three layers: (1) `action_parse.py` SQLite pre-check (`_load_existing_titles()`) skips known titles during Phase 1 parsing. (2) `deliver_today.py` `_make_id()` uses category-agnostic `action-` prefix so the same action gets the same ID regardless of overdue/today/week bucket, plus within-briefing dedup by ID, plus its own SQLite pre-check before JSON generation. (3) Rust-side `upsert_action_if_not_completed()` title-based dedup as final guard.

**I33: Captured wins/risks don't resurface in meeting preps** — Resolved. ADR-0030 `meeting_prep.py` queries `captures` table via `_get_captures_for_account()` for recent wins/risks by account_id (14-day lookback). Also queries open actions and meeting history per account. Rust `db.rs` gained `get_captures_for_account()` method with test.

**I38: Deliver script decomposition** — Resolved. ADR-0042 Chunk 1 replaces deliver_today.py with Rust-native per-operation delivery (`workflow/deliver.rs`). Chunk 3 adds AI enrichment ops: `deliver_emails()` (mechanical email mapping), `enrich_emails()` (PTY-spawned Claude for summaries/actions/arcs per high-priority email), `enrich_briefing()` (PTY-spawned Claude for 2-3 sentence day narrative patched into schedule.json). All AI ops are fault-tolerant — if Claude fails, mechanical data renders fine. Pipeline: Phase 1 Python → mechanical delivery (instant) → partial manifest → AI enrichment (progressive) → final manifest. Week delivery (deliver_week.py) remains monolithic (ADR-0042 Chunk 6).

**I36: Daily impact rollup for CS extension** — Resolved. Added `workflow/impact_rollup.rs` with `rollup_daily_impact()` that queries today's captures from SQLite, groups wins/risks by account, and appends to `Weekly-Impact/{YYYY}-W{WW}-impact-capture.md`. Runs in archive workflow between reconciliation and file moves, profile-gated to `customer-success`, non-fatal. Idempotent (day header check prevents double-writes). Creates file with template if missing. `db.rs` gained `get_captures_for_date()`. 9 new tests.

**I45: Post-transcript outcome interaction UI** — Resolved. `MeetingOutcomes.tsx` renders AI-extracted summary, wins, risks, decisions, and actions inside MeetingCard's collapsible area. Actions: checkbox completion (`complete_action`/`reopen_action`), priority cycling via `update_action_priority` command. Wins/risks/decisions: inline text editing via `update_capture` command. All changes write to SQLite (working store). Markdown writeback for actions already exists via `sync_completion_to_markdown` hook; non-action capture edits stay SQLite-only (consistent with ADR-0018 — SQLite is disposable cache, originals from transcript processing are the markdown source of truth).

**I44: Meeting-scoped transcript intake from dashboard** — Resolved. ADR-0044. Both surfaces have transcript attachment: `PostMeetingPrompt` file picker and `MeetingCard` attach affordance. `processor/transcript.rs` handles full pipeline — frontmatter stamping, AI enrichment via Claude (`--print`), extraction of summary/actions/wins/risks/decisions, routing to account location. Immutability enforced via `transcript_processed` state map. Frontend: `MeetingOutcomes.tsx` + `useMeetingOutcomes.ts`. Meeting card is now a lifecycle view: prep → current → outcomes.

**I32: Inbox processor doesn't update account intelligence** — Resolved. AI enrichment prompt extracts WINS/RISKS sections. Post-enrichment `entity_intelligence` hook writes captures (with synthetic `inbox-{filename}` meeting IDs) and touches `accounts.updated_at` as last-contact signal. Read side (`get_captures_for_account`) + write side both wired.

**I47: Profile-agnostic entity abstraction** — Resolved. Introduced `entities` table and `EntityType` enum (ADR-0045). Bridge pattern: `upsert_account()` auto-mirrors to entities table, backfill migration populates from existing accounts on DB open. `entity_intelligence()` hook replaces profile-gated `cs_account_intelligence()` — now runs for all profiles as core behavior (ADR-0043). `account_id` FK migration deferred to I27.

**I42: CoS executive intelligence layer** — Resolved. New `intelligence.rs` module computes five signal types from existing SQLite data + schedule: decisions due (AI-flagged `needs_decision` actions ≤72h), stale delegations (waiting actions >3 days), portfolio alerts (renewals ≤60d, stale contacts >30d, CS-only), cancelable meetings (internal + no prep), skip-today (AI enrichment). New `IntelligenceCard.tsx` renders signal counts as badges with expandable detail sections. Schema migration adds `needs_decision` column. 13 new tests.

**I43: Stakeholder context in meeting prep** — Resolved. `db.rs` gained `get_stakeholder_signals()` which computes meeting frequency (30d/90d), last contact, relationship temperature (hot/warm/cool/cold), and trend (increasing/stable/decreasing) from `meetings_history` and `accounts` tables. Signals computed live at prep load time in `get_meeting_prep` command (always fresh from SQLite, not baked into prep files). `RelationshipContext` component in `MeetingDetailPage.tsx` shows four-metric grid. 5 new tests.

**I41: Reactive meeting:prep wiring** — Resolved. `google.rs` calendar poller now generates lightweight prep JSON for new prep-eligible meetings (customer/qbr/partnership) after each poll cycle. Checks both meeting ID and calendar event ID to avoid duplicates. Enriches preps from SQLite account data (Ring, ARR, Health, Renewal, open actions). Emits `prep-ready` event; `useDashboardData` listens for silent refresh. Rust-native (ADR-0025), no Python subprocess. 8 new tests.

**I31: Inbox transcript summarization** — Resolved. `enrich.rs` gained `detect_transcript()` heuristic (filename keywords, speaker label ratio >40%, timestamp ratio >20%, minimum 10 lines) and richer enrichment prompt for transcripts: 2-3 sentence executive summary + discussion highlights block. Parser handles `DISCUSSION:` / `END_DISCUSSION` markers. Non-transcript files unchanged (backward compatible). 12 enrich tests.

**I39: Feature toggle runtime** — Resolved. `features: HashMap<String, bool>` on Config with `#[serde(default)]` for zero-config upgrade. `default_features(profile)` returns profile-conditional defaults (CS-only features off for general). `is_feature_enabled()` priority chain: explicit override → profile default → true. Executor gates: emailTriage, meetingPrep, inboxProcessing, impactRollup. Settings UI: FeaturesCard with toggle list. 7 new tests.

**I18: Google API credential caching** — Resolved. Module-level `_cached_credentials` and `_cached_services` dict in `ops/config.py`. `build_google_credentials()`, `build_calendar_service()`, `build_gmail_service()` check cache first, return cached if valid, refresh if expired. Per-process only (separate subprocesses don't share). Eliminates double token refresh within `prepare_today.py`.

**I20: Standalone email refresh** — Resolved. New `scripts/refresh_emails.py` thin orchestrator (reuses `ops.email_fetch`). `execute_email_refresh()` in executor.rs spawns script, reads output, calls `deliver_emails()` + optional `enrich_emails()`. `refresh_emails` Tauri command with WorkflowStatus guard (rejects if pipeline Running). Refresh button in EmailList.tsx. Emits `operation-delivered: emails` for frontend refresh.

**I21: FYI email classification expansion** — Resolved. Expanded `LOW_PRIORITY_SIGNALS` with marketing/promo/noreply terms. Added `BULK_SENDER_DOMAINS` frozenset (mailchimp, sendgrid, hubspot, etc.), `NOREPLY_LOCAL_PARTS` set. Enhanced classification: List-Unsubscribe header → low, Precedence bulk/list → low, bulk sender domain → low, noreply local part → low. Customer domain check still runs first (high priority wins). 16 new Python tests.

**I37: Density-aware dashboard overview** — Resolved. `classify_meeting_density()` in `deliver.rs` categorizes day as light (0-2), moderate (3-5), busy (6-8), packed (9+). Density guidance injected into `enrich_briefing()` prompt so Claude adapts narrative tone. First meeting time included in context. 4 new tests.

**I6: Processing history page** — Resolved. `get_processing_history` Tauri command (reads `processing_log` table, default limit 50). `HistoryPage.tsx` with table rendering (filename, classification badge, status badge, destination, timestamp, error). Route at `/history`, sidebar nav item under Workspace group.

**I48: Workspace scaffolding on initialization** — Resolved. `initialize_workspace()` in `state.rs` creates `_today/`, `_today/data/`, `_inbox/`, `_archive/`, `Projects/` always, `Accounts/` conditional on entity mode (account/both). Called by `set_workspace_path` command. Idempotent, validates parent directory exists. 4 new tests.

**I49: Graceful degradation without Google authentication** — Resolved. Python pipeline already returns empty data when Google auth missing. `DashboardResult` now includes `google_auth` status. `DashboardEmpty` shows "Connect Google" CTA when unauthenticated. Dashboard renders meaningfully without Google data.

**I7: Settings page can change workspace path** — Resolved. `set_workspace_path` Tauri command with directory picker via `@tauri-apps/plugin-dialog`. Validates path, calls `initialize_workspace()`, updates config. `WorkspaceCard` component in SettingsPage with change button + toast feedback.

**I13: Onboarding wizard** — Resolved. `OnboardingWizard.tsx` with 5-step flow: Welcome → Entity Mode → Workspace → Google Auth (skippable) → Generate First Briefing. Replaces `ProfileSelector`. Router detects missing config or workspace path and shows wizard. All three entity modes + both auth paths work end-to-end.

**I15: Entity-mode switcher in Settings** — Resolved. `set_entity_mode` Tauri command validates mode, sets `entity_mode` + derives `profile` for backend compat. `EntityModeCard` component in SettingsPage with three radio-style options (account-based, project-based, both). App reloads on change. Supersedes profile switching per ADR-0046.

**I16: Schedule editing UI** — Resolved. `set_schedule` Tauri command generates cron from hour/minute/timezone. `cronToHumanTime()` helper replaces raw cron display with "6:00 AM" format. `ScheduleRow` in SettingsPage now shows human-readable time.

**I51: People sub-entity** — Resolved. Universal person tracking with ADR-0048 compliance. 3 new tables (`people`, `meeting_attendees`, `entity_people`), ~15 DB functions, `people.rs` file I/O module (ADR-0047 two-file pattern with entity link durability), `util.rs` helpers, 8 Tauri commands, calendar auto-population, file watcher extension, startup sync, person signals, executive intelligence alerts. Frontend: PeoplePage (filterable list), PersonDetailPage (editable detail + signals). 189 Rust tests.

**I72: Entity dashboard pages** — Resolved. Account list page (`AccountsPage.tsx`) with sortable table (health, ARR, ring, renewal, last contact, open actions) + account detail page (`AccountDetailPage.tsx`) with card-based layout (quick context, company overview, programs, notes, recent meetings, people). 6 Tauri commands. Route at `/accounts` and `/accounts/$accountId`.

**I73: Entity dashboard template system** — Resolved. ADR-0047 two-file pattern: `dashboard.json` (canonical write interface) + `dashboard.md` (generated artifact). `accounts.rs` module with `AccountJson` schema, read/write/sync functions, markdown generation from JSON + SQLite live data. Three-way sync: JSON ↔ SQLite ↔ markdown. File watching via mtime comparison on entity access. `sync_from_workspace()` startup scan for offline edits.

**I59: `CARGO_MANIFEST_DIR` runtime resolution** — Resolved. `resolve_scripts_dir()` uses Tauri resource resolver in release builds, falls back to `CARGO_MANIFEST_DIR` in debug. Scripts bundled via `tauri.conf.json` resources array.

**I46: Meeting prep context limited to customer/QBR/training meetings** — Resolved. `meeting_prep.py` only gathered rich context (SQLite history, captures, open actions) for customer meetings with account-based queries. Internal syncs, 1:1s, and partnership meetings got at most a single archive ref. Per ADR-0043 (meeting intelligence is core), expanded with title-based SQLite queries (`_get_meeting_history_by_title`, `_get_captures_by_meeting_title`, `_get_all_pending_actions`) so all non-personal/non-all-hands types get meeting history, captures, and actions context. 1:1s get deeper lookback (60-day history, 3 archive refs). Partnership meetings try account match first, fall back to title-based. No schema or orchestrator changes.

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |
| R5 | **Open format = no switching cost.** Markdown portability means users can leave as easily as they arrive. The moat (archive quality) only works if DailyOS maintains the archive better than users could themselves — and better than a competitor wrapping the same open files. | High | Medium | Archive must be demonstrably better than DIY. Enrichment quality is the lock-in, not format. | Open |
| R6 | **N=1 validation.** All architecture designed from one user in one role (CS leader). Entity modes, Kits, Intelligence untested with actual project-based, sales, or engineering users. Assumptions about "how work is organized" may not survive contact with diverse roles. | High | High | Recruit 3-5 beta users across different roles before implementing I27. Validate entity-mode assumptions with real workflows. | Open |
| R7 | **Org cascade needs adoption density.** Organizational intelligence (Thursday Updates, cascading contributions) requires multiple DailyOS users on the same team. Single-user value must stand alone — org features are years away from being testable. | Medium | High | Ship individual product first. Don't invest in org features until adoption density exists. Keep it in Vision, not Roadmap. | Open |
| R8 | **AI reliability gap.** "Zero discipline" promise depends on AI enrichment being consistently good. Current fault-tolerant design (mechanical data survives AI failure) mitigates data loss but not quality — a bad briefing erodes trust faster than no briefing. | High | Medium | Invest in enrichment quality metrics. Surface confidence signals to users. Make AI outputs editable/correctable. | Open |
| R9 | **Composability untested at scale.** Kit + Intelligence + Integration composition is designed on paper (ADR-0046) but never built. Enrichment prompt fragment ordering, conflicts between multiple Intelligence layers, and "both" entity mode UX are all theoretical. | Medium | Medium | Build one Kit (CS) + one Intelligence (Executive) first. Validate composition with two overlays before designing more. | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | No | Need onboarding check (I13) |
| A2 | Workspace follows PARA structure | No | Should handle variations gracefully |
| A3 | `_today/` files use expected markdown format | Partial | Parser handles basic cases |
| A4 | Users have Google Workspace (Calendar + Gmail) | No | Personal Gmail, Outlook, iCloud not supported in MVP |

---

## Dependencies

| ID | Dependency | Type | Status | Notes |
|----|------------|------|--------|-------|
| D1 | Claude Code CLI | Runtime | Available | Requires user subscription |
| D2 | Tauri 2.x | Build | Stable | Using latest stable |
| D3 | Google Calendar API | Runtime | Optional | For calendar features |

---

*Migrated from RAIDD.md on 2026-02-06. Decisions are now tracked in [docs/decisions/](decisions/README.md).*
