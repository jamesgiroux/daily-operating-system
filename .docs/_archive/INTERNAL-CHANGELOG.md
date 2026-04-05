# Changelog

Closed issues organized by sprint. For active work, see [BACKLOG.md](./BACKLOG.md).

---

## 0.8.3 — Cleanup ✓ SHIPPED

*Carry-forward code quality, type-safety fixes, prompt hardening, and people deduplication.*

### Code Quality
- **I303:** Fixed `LinkedEntity.entityType` type narrowing in `meeting-entity-chips.tsx`. The `entityType` field now uses the correct union type at source, eliminating unsafe casts downstream.
- **I304:** Prompt audit — hardened AI prompts across enrichment pipeline. Signal types constrained to enum values, null field handling improved, output-tailored guidance added to reduce hallucination.
- **Migration idempotency:** Hardened migration framework — removed orphan SQL file (`007_chat_sessions.sql`), ensured all migrations are idempotent with `IF NOT EXISTS` guards.

### People Deduplication (I302 follow-up)
- **Person email aliasing:** New `person_emails` table (migration 012) tracks all known emails per person. `get_person_by_email_or_alias()` checks both `people.email` and `person_emails` for lookups.
- **Cross-domain resolution:** `get_sibling_domains_for_email()` uses `account_domains` groups + `user_domains` config to find sibling domains. `find_person_by_domain_alias()` constructs `local_part@sibling` variants to match existing people. Personal email domains (gmail, yahoo, etc.) are never aliased.
- **Alias-aware calendar sync:** `populate_people_from_events()` now tries exact → alias → domain-alias resolution before creating new people. All 4 other `get_person_by_email` call sites (commands, meeting_context, executor, people sync) upgraded to `get_person_by_email_or_alias`.
- **Hygiene dedup:** `dedup_people_by_domain_alias()` added to hygiene scanner — groups people by `(local_part, domain_group)`, merges duplicates via existing `merge_people()`, keeps highest meeting count. Wired into mechanical fixes phase.
- **Merge transfer:** `merge_people()` now transfers email aliases from removed to kept person. `delete_person()` cleans up `person_emails`. `upsert_person()` auto-seeds primary email.
- **707 Rust tests** (up from 700). 6 new tests: alias lookup, domain alias search, sibling domains, CRUD, merge transfer, end-to-end integration.

### QA Fixes
- Fixed stale agenda overwrite when hiding attendees.
- Fixed transcript action FK violation and path traversal (security audit).
- Added input bounds on user agenda and polished transcript button UX.

---

## 0.8.2 — Polish & Technical Debt ✓ SHIPPED

*Editorial polish, meeting intelligence redesign, audit trail, transcript processing, calendar enhancements. 700 Rust tests.*

### Meeting Intelligence Report Redesign
- **Editorial briefing format:** Redesigned meeting intelligence report from structured data grid to narrative editorial briefing. Meeting outcomes always shown at top. Intelligence presented as editorial prose with clear section hierarchy.
- **QuickContext on schedule cards:** Replaced PrepGrid with QuickContext on daily schedule cards. Internal stakeholders filtered out for cleaner external-focused meeting prep.
- **Transcript attach button:** Added folio bar button on all meeting detail pages for attaching transcripts. Consistent transcript UX across pre-meeting and post-meeting views.

### Audit Trail (I297)
- **Audit trail module:** New `audit.rs` module for tracking AI-generated data provenance. Records model, prompt hash, and generation timestamp for all enrichment outputs.
- **Pipeline integration:** Audit trail wired into enrichment pipeline — all AI-generated intelligence now carries provenance metadata.
- **Transcript processing improvements:** Improved transcript and enrichment processing with better error handling and status tracking.

### Calendar & People
- **RSVP status:** Carried attendee RSVP status (accepted/declined/tentative) through the full calendar pipeline for meeting intelligence enrichment (I301).
- **Person email aliasing (I302):** Foundation for cross-domain person dedup — `person_emails` schema, alias-aware lookup, domain-group resolution.

### Editorial Polish
- **Print styles:** Added print CSS for clean briefing PDF output.
- **Risk briefing UX:** Moved Regenerate button to folio bar, made byline editable.
- **Briefing export:** `export_briefing_html` command for browser-based PDF export.
- **Meeting entity chips:** Optimistic local state for entity chip interactions.
- **Featured meeting:** Kept featured meeting visible in schedule list, added onRefresh prop.
- **Prep summaries:** Enriched prep summaries from entity intelligence fields.

### Infrastructure
- **Claude Code skills:** Added skill templates for workspace distribution.
- **MCP sidecar:** Fixed missing executable permission on MCP sidecar binary.
- **IPC fix:** Fixed `create_action` IPC to use request object parameter.

---

## 0.8.1 — Hardening ✓ SHIPPED

*Release pipeline hardening, CI sidecar fixes, editorial polish, marketing site redesign.*

### Release Pipeline
- **Sidecar chicken-and-egg fix:** Tauri's `build.rs` validates `externalBin` paths during any `cargo build`. Fixed by creating empty stub file before `cargo build`, then overwriting with real binary. Added `touch build.rs` to force Cargo rerun after sidecar build.
- **CI hardening:** Removed uncommitted audit refs, busted stale Cargo cache with key prefix bump, fixed clippy `ptr_arg` warning, cleared stale `build.rs` cache in test workflow.
- **Release checklist:** Updated `.docs/RELEASE-CHECKLIST.md` with sidecar build lessons and CI smoke test step.

### Code Quality (I232, I236, I263, I287, I289, I290, I291)
- **P3 hardening sprint:** 7 code quality and performance items — duplicate code removal, unused import cleanup, error handling improvements, type safety fixes.

### Editorial & UX
- **Folio bar navigation:** Brand mark now navigates to daily briefing on click.
- **Business unit button:** Added "+ Business Unit" button to account detail folio bar.
- **Meeting card:** Fixed padding and hydrated prep summaries from prep files.
- **Emails page:** Fixed crash when email has no signals array.
- **Intelligence prompts:** Removed footnote references and source citations from intelligence output for cleaner editorial prose.
- **Density pass:** v0.8.2 polish prep — density pass, hygiene narrative improvements, weekly priorities refinement.

### Marketing Site
- **Editorial redesign:** Rebuilt `docs/` as editorial magazine experience.
- **Brand assets (I300):** Added og-image, regenerated favicon PNGs from asterisk mark, glass-tinted app icon background.

---

## 0.8.0 — Editorial ✓ SHIPPED

*Complete visual overhaul, semantic search, MCP server, risk briefing, security hardening, beta audit. Subsumes work previously tracked as Sprints 25-28 + 31. Sprint numbering retired in favor of semantic versioning.*

### Risk Briefing v3 (I278)
- **I278:** Risk briefing restructured from 10 analytical chapters (1:1 SCQA mapping) to 6 narrative slides (Cover, Bottom Line, What Happened, The Stakes, The Plan, The Ask). SCQA remains the internal AI thinking tool; the output is now a presentation structure. New Rust data model (6-slide types replace 10-section types, 15 orphan types deleted). Prompt rewritten as "senior strategy consultant preparing a 6-slide executive risk briefing" with hard word limits (60-word narrative, 10-word key losses, 20-word headline). Scroll-snap viewport slides with keyboard navigation (1-6, arrows). All text fields are click-to-edit via new `EditableText` component — hover shows subtle background hint, click opens matched-style input/textarea, blur auto-saves. Debounced persistence (500ms) writes edited `risk-briefing.json` back to disk via `save_risk_briefing` Tauri command. FolioBar shows transient "Saved" indicator in sage green. Wider layouts (800px), larger fonts, higher contrast for screen-share readability.

### MCP Server — Claude Desktop Integration (ADR-0075)
- **MCP binary:** Standalone `dailyos-mcp` binary exposes 4 read-only tools via stdio transport (rmcp 0.1): `get_briefing` (today's schedule/emails/actions/briefing), `query_entity` (entity detail + intelligence + actions + meetings), `list_entities` (portfolio with health/action counts), `search_meetings` (SQL LIKE across meeting history). Feature-gated behind `mcp` Cargo feature with `[[bin]]` target. Shares `dailyos_lib` — zero code duplication.
- **Read-only DB access:** `ActionDb::open_readonly()` with `SQLITE_OPEN_READ_ONLY` flag + WAL mode. MCP binary cannot write to the database; the Tauri app owns all writes.
- **Settings UI:** `ClaudeDesktopCard` component in Integrations tab. One-click `configure_claude_desktop` command reads/creates `~/Library/Application Support/Claude/claude_desktop_config.json` and adds `mcpServers.dailyos` entry with resolved binary path.

### Database & Infrastructure
- **DB rename:** `actions.db` → `dailyos.db` — the database outgrew its original actions-only scope (now stores accounts, projects, people, meetings, intelligence, embeddings, chat sessions). Seamless one-time rename on next app launch with WAL checkpoint before rename to prevent data loss.
- **BrandMark SVG:** Replaced TTF text asterisk (`*`) with `BrandMark.tsx` SVG component extracted from Montserrat ExtraBold glyph outline via `fonttools`. Eliminates runtime font dependency for the brand mark. Replaced in FloatingNavIsland, FolioBar, AtmosphereLayer, all entity heroes (Account/Person/Project), FinisMarker, WeekPage, DashboardEmpty. CSS updated: `font-family`/`font-weight`/`font-size` → `width`/`height` for SVG sizing.

### Beta Hardening Audit (I280 umbrella)
- **I280:** Comprehensive three-part codebase audit (dependency/bundle, database schema, token/prompt efficiency). 12 sub-issues created (I281-I291) as beta gate. Audit found: 3 unused npm packages, 13% FK enforcement in DB, ~60% token reduction possible, known DRY debt formalized.
- **I281:** Removed `date-fns`, `react-markdown`, `remark-gfm` — zero imports across codebase, 97 packages pruned from lockfile (~235KB dead weight). Rust side clean: all 26 Cargo crates actively used with optimal feature flags.
- **I282:** `useExecutiveIntelligence` hook confirmed as working scaffolding for I55 (Executive Intelligence, parking lot). Hook + `get_executive_intelligence` command + `compute_executive_intelligence` in intelligence.rs produce real signals (decisions, delegations, portfolio alerts). No UI consumer yet — keep as-is.
- **I283:** Migration 008 adds three missing indexes: `meeting_entities(meeting_id)` eliminates full table scan on entity detail pages, unique `meetings_history(calendar_event_id)` prevents duplicate calendar imports, composite `actions(status, due_date)` improves filtered+sorted action queries.
- **I284:** Audit false positive — `upsert_account()` already calls `ensure_entity_for_account()` at db.rs:1213, function exists at db.rs:2534. All three entity types (accounts, projects, people) correctly sync to the entities table.

### Security Hardening (I292, I293, I294)
- **I292:** CSP header added to `tauri.conf.json` — restricts script/style/img/connect sources to `'self'` (plus `'unsafe-inline'` for styles, `data:`/`blob:` for images, `ipc:`/`http://ipc.localhost` for Tauri IPC). Defense-in-depth against XSS from dependency vulnerabilities.
- **I293:** `reveal_in_finder` now canonicalizes the requested path and validates it's within the workspace directory or `~/.dailyos/`. Rejects anything outside those boundaries. Prevents arbitrary filesystem traversal via IPC.
- **I294:** `copy_to_inbox` now restricts source paths to `~/Documents`, `~/Desktop`, and `~/Downloads` via canonicalization + prefix check. Rejected paths logged and skipped. Prevents file exfiltration (e.g., `~/.ssh/id_rsa` → `_inbox/` → `get_inbox_file_content`).

### Folio Bar Consistency
- Account Detail "Reports" button and WeekPage "Refresh" button converted from shadcn `Button variant="ghost"` to editorial monospace pill style (mono 11px, 600 weight, uppercase, entity-colored border). Now consistent with Add/New buttons on all list pages.

### Editorial Polish
- **WeekPage:** Chapter reorder (Shape before Meetings), editorial skeleton loading states, `GeneratingProgress` component for workflow progress, meeting day separators, action caps. FolioBar Refresh button uses consistent editorial monospace pill style.
- **List pages:** Accounts, Projects, People, Actions pages restyled with editorial design language — serif headings, entity-colored accents, magazine-shell integration.
- **DailyBriefing:** New `DailyBriefing.tsx` and `BriefingMeetingCard.tsx` components for consumption-first daily dashboard redesign.
- **Backfill meetings:** `backfill_meetings.rs` module for historical meeting import from transcript filenames.

---

## Sprint 28 — Claude Cowork Integration ✓ COMPLETE (now part of 0.8.0)

*Publish DailyOS Cowork plugin. Bridge operational intelligence to the agentic productivity layer.*

### Cowork Plugin (I244 umbrella)
- **I274:** Restructured Cowork plugin to match `.claude-plugin/` spec. Manifest at `cowork-plugin/.claude-plugin/plugin.json` with license/homepage/repository metadata. 6 commands (brief, prep, agenda, review, report, complete), 4 skills (operational-intelligence marked non-user-invocable, account-context, meeting-prep, action-execution), 1 agent (researcher, restricted to read-only tools). Plugin README added.
- **I275:** Workspace CLAUDE.md generation from `initialize_workspace()`. On app version bump, writes managed `CLAUDE.md` (workspace structure, entity conventions, file locations) + `.claude/settings.json` (permissions) to the DailyOS workspace. Version-stamped to avoid unnecessary rewrites. Enables zero-config Claude Code activation — users open their workspace and Claude automatically understands the structure.

**I276 deferred** — app-managed plugin distribution (Settings UI + auto-write) not yet needed; manual ZIP install works. **I245 deferred** — "Open in Cowork" blocked by missing `claude://` URL scheme.

---

## Sprint 27 — Reliability & Intelligence Polish (now part of 0.8.0)

*PTY hardening, hygiene configurability, person intelligence, CI/infra fixes. Cross-cutting reliability work between editorial redesign and Cowork integration.*

### Reliability
- **I257:** PTY enrichment pipeline hardened across all 10 enrichment call sites. `TERM=dumb` on PTY CommandBuilder suppresses ANSI escape codes. PTY widened from 80 to 4096 columns to prevent hard line wrapping splitting JSON/markers mid-line. `strip_ansi()` safety net on all output before parsing. Debug logging of raw Claude output (first 500 bytes).
- **Claude CLI resolution:** Resolve `claude` binary from common install locations (`~/.local/bin`, `/usr/local/bin`, `/opt/homebrew/bin`) when shell PATH is unavailable — macOS apps launched from Finder don't inherit shell profile. Email retry now gated by backend event confirmation.

### Intelligence
- **I271:** Hygiene system polish — three config fields (scan interval, AI budget, pre-meeting window) with pill-button selectors in Settings. Fix functions return narrative descriptions instead of counts. Scan duration shown inline. Files that fail extraction marked terminal (no re-scan). SQL format filter corrected (PlainText not Text). Duplicate merge gated behind confidence threshold with confirmation dialog. Overnight window uses local timezone. First run scans all orphaned meetings with no lookback limit.
- **Person intelligence prompts:** Relationship-aware enrichment — internal teammates get collaboration framing, external stakeholders get relationship health framing. Prompt adapts verdict, writing rules, and risk focus based on relationship type.
- **Shared FileListSection:** Extracted from Account/Project appendixes into `src/components/entity/FileListSection.tsx` to eliminate duplication.
- **Schema migrations:** 006 `content_embeddings` (vector search, ADR-0074) and 007 `chat_sessions`/`chat_turns` (conversational interface, ADR-0075) SQL files added.

### Infrastructure
- **I242:** Re-enabled Apple notarization in CI release pipeline with App Store Connect API key auth, 45min timeout, .p8 key cleanup.
- **CI hardening:** Codesign keychain search list + auto-lock timeout fix. Heredoc delimiter newline fix. Clippy/test reorder, cargo-audit caching, skip tag duplicates.
- **License:** `package.json` license corrected to match root LICENSE (GPL-3.0-only).
- **Version:** Bumped to v0.7.4 (Claude CLI fix) → v0.7.5 (PTY hardening).

---

## Sprint 25 — Editorial Design Language ✓ COMPLETE (now part of 0.8.0)

*Magazine-layout editorial redesign. New app shell, design tokens, entity-generic components, all detail pages rebuilt.*

### App Shell & Design Tokens
- **I238 (partial):** Design tokens established in `styles/design-tokens.css` — editorial typography (Newsreader serif, DM Sans, JetBrains Mono), spacing scale, radius system, color tokens, transition presets. 142 lines of CSS custom properties.
- **New layout components:** FolioBar (editorial masthead with date, readiness, search), FloatingNavIsland (dual app/chapter navigation modes with 36px icon buttons and tooltip labels), AtmosphereLayer (page-specific radial gradients with breathing animation and watermark asterisk), MagazinePageLayout (single-column prose wrapper with chapter scroll).
- **Editorial primitives:** ChapterHeading (serif chapter titles), PullQuote (accent-bordered callout), StateBlock, WatchItem, TimelineEntry (timeline dot + prose), FinisMarker (three-asterisk end mark with enrichment timestamp).
- **MagazineShellContext:** React context for page→shell communication — pages register their FolioBar config and chapter definitions, shell renders them. Replaced regex-based route matching.
- **Hooks:** `useChapterObserver` (IntersectionObserver-based active chapter detection), `useRevealObserver` (scroll-reveal fade-in animation), `useMagazineShell` (shell registration).

### Account Detail (I224 partial, I233)
- **Account editorial redesign:** Replaced 2,929-line `AccountDetailPage.tsx` with magazine-layout `AccountDetailEditorial.tsx`. Single-column prose flow with 7 chapters: Headline, State of Play, The Room, Watch List, The Record, The Work, Appendix. Scroll-linked fade-in reveals, icon-based chapter navigation with 800ms editorial smooth scroll. Full feature parity: field editing drawer, team management drawer, lifecycle event drawer, inline program editing, action creation, notes, intelligence enrichment, archive, child account creation, file indexing.
- **Code review follow-up:** 24 findings addressed — router shell switching via route ID set + React context (not regex), DRY extractions (capitalize, formatMeetingType, ActionRow, CompanyContextBlock), useAccountDetail split into useAccountFields + useTeamManagement composed sub-hooks, FolioBar href→onClick fix, drawer close-before-save race fixes, useChapterObserver stale state guard. Net -553 lines.

### Entity Template Extraction (I237 partial)
- **Shared entity components:** Extracted 6 account-specific components to `src/components/entity/`: VitalsStrip, StateOfPlay, StakeholderGallery, WatchList, UnifiedTimeline, TheWork — all with narrow interfaces via structural typing so accounts, projects, and people can share them.

### Project Detail (I224 partial)
- **Project editorial redesign:** Magazine-layout with olive theme. Initial 6 chapters mirrored from accounts, then refined to trajectory-first narrative: Mission → Trajectory → The Horizon → The Landscape → The Team → The Record. TheWork chapter dropped; open actions moved to Appendix as reference material. New components: TrajectoryChapter, HorizonChapter, ProjectFieldsDrawer, WatchListMilestones. `useProjectDetail` hook.

### Person Detail (I224 partial)
- **Person editorial redesign:** Magazine-layout with larkspur theme, 5 chapters. Circular initial avatar, relationship + temperature badges. PersonInsightChapter for intelligence display. PersonNetwork for entity connections. PersonAppendix for files/actions reference. `usePersonDetail` hook with richer data loading.

### Action Detail
- **Action editorial redesign:** Single-screen practical layout (no magazine shell — actions are reference, not narrative). Serif title, mono metadata labels, priority accent colors (P1=terracotta, P2=turmeric, P3=larkspur), reference grid, inline save indicator. Native date input replaced with shadcn Popover + Calendar date picker. Action ID validation widened to accept spaces and dots from AI-generated IDs.

---

## Sprint 26 — OpenClaw Learnings ✓ COMPLETE (now part of 0.8.0)

*Semantic retrieval and conversational interfaces to enhance entity intelligence quality. 664 Rust tests.*

### Vector Search Pipeline (ADR-0074, ADR-0078)
- **I246:** ADR-0074 written and accepted — vector search architecture for entity content (storage schema, hybrid scoring, query API, chunking strategy).
- **I248:** Embedding model integration shipped. Originally `ort` + `tokenizers` + `ndarray` for snowflake-arctic-embed-s, then switched to `fastembed` crate with nomic-embed-text-v1.5 (ADR-0078). Model auto-downloads to `~/.dailyos/models/` on first launch. Hash-based fallback when unavailable.
- **I249:** Schema migration 006 — `content_embeddings` table with chunk-level vectors (FK to `content_index`, cascade delete). `embeddings_generated_at` watermark column on `content_index`.
- **I250:** Background embedding processor (`processor/embeddings.rs`). Startup enqueue, watcher enqueue on file change, periodic sweep every 5 min. Priority-based dedup queue. Per-paragraph chunking (~500 tokens, 80-token overlap).
- **I251:** Semantic search query function (`queries/search.rs`). Hybrid scoring: 70% vector cosine similarity + 30% BM25 keyword. Corpus-computed avgdl. Asymmetric prefixes for nomic retrieval.
- **I252:** Semantic search integrated into `build_intelligence_context()` in `entity_intel.rs`. Gap-targeted queries surface relevant historical content.

### Conversational Interface (ADR-0075)
- **I247:** ADR-0075 written and accepted — Phase 1 external via MCP, Phase 2 in-app if validated.
- **I253:** Schema migration 007 — `chat_sessions` + `chat_turns` tables for conversational memory persistence.
- **I254:** MCP chat tools shipped as Tauri commands: `chat_query_entity`, `chat_search_content`, `chat_get_briefing`, `chat_list_entities`. Session persistence with auto-create/reuse.

### Documentation
- **I255:** OpenClaw learnings research archived at `daybreak/docs/research/2026-02-14-openclaw-learnings.md`.

### Sprint 26 Follow-Up (Code Review — I264-I270)
- **I264:** Real ONNX inference via fastembed (nomic-embed-text-v1.5, 768d, ~137MB INT8, Apache 2.0). +6.83 NDCG@10 vs snowflake-arctic-embed-s. ADR-0078 supersedes ADR-0074 model choice.
- **I265:** Asymmetric query/document prefixes (`"search_query: "` / `"search_document: "`).
- **I266:** Restored 0.7/0.3 hybrid search weights at all 3 call sites (was 0.0/1.0 during hash-embed phase).
- **I267:** Fixed silent lock poisoning in EmbeddingModel — proper error returns instead of swallowed failures.
- **I268:** DRY extraction — `meetings_to_json()` helper and `ChatEntityListItem` moved to types.rs, replacing 3 duplicate blocks.
- **I269:** Closed — `ActionDb::open()` is the correct codebase pattern (22 uses, well-documented). Not a bug.
- **I270:** BM25 avgdl computed from actual corpus token counts instead of hardcoded 200.

**Cleanup:** Removed placeholder ONNX model file, cleared `tauri.conf.json` resources (fastembed downloads at runtime), removed `ort`/`tokenizers`/`ndarray` dependencies.

---

## Sprint 17 — Pipeline Reliability & Error Handling

*Critical workflow reliability fixes + operational diagnostics.*

- **I204:** Weekly partial-delivery resilience completed. Week page now renders explicit "enrichment incomplete" state when AI enrichment fails, while mechanical data remains available. Added `retry_week_enrichment` command and UI action to retry enrichment-only without rerunning full week prepare/deliver.
- **I205:** Settings operational visibility shipped via `DeliveryHistoryCard`. History is bounded (14-day window, max 50 records) and surfaces workflow status (success/partial/failed), trigger, duration, error summary, phase context, retryability, and retry actions.
- **I206:** Durable prep visibility fixed. Dashboard meeting hydration now falls back to persisted `prep_context_json` from SQLite when prep files are ephemeral/missing. MeetingCard "View Prep" routes by canonical meeting id so historical prep loads from DB snapshot.
- **I203:** Inbox drop duplicate bug fixed with dual guardrails: frontend drop-event burst dedupe (signature + debounce window) and backend source-path dedupe in `copy_to_inbox`.
- **I164:** Inbox processing indicators hardened. Inbox rows now show persistent status (including explicit unprocessed) sourced from SQLite `processing_log`, with Process action consistently visible.
- **I208:** OAuth architecture hardened for production builds. Embedded credentials now read `client_secret` at compile time via `option_env!(\"DAILYOS_GOOGLE_SECRET\")`. Release workflow validates/injects `DAILYOS_GOOGLE_SECRET` during Tauri build. Added `google-auth-failed` event plumbing plus frontend timeout/error states to prevent silent hangs during auth.
- **Polish follow-up:** Execution history records now include explicit `error_phase` + `can_retry` metadata, enabling deterministic diagnostics and retry affordances in Settings (instead of heuristic message parsing).

---

## Sprint 18 — Focus & Meeting Intelligence ✓ COMPLETE

*Focus page polish + meeting prep improvements.*

- **I179:** Focus page action prioritization intelligence shipped. Live capacity engine with deterministic action prioritization. Top 3 recommended actions based on urgency and feasibility scoring.
- **I214:** Focus page "Other Priorities" capped to 5 P1 actions with "View All Actions" link for viewing complete prioritized action list.
- **I188:** Agenda-anchored AI enrichment completed (ADR-0064 Phase 4). Prep generation now anchors to calendar event description notes for richer context-aware enrichment.

---

## Sprint 19 — Settings & Intelligence Hygiene ✓ COMPLETE

*Settings UX + hygiene system reporting.*

- **I212:** Settings page reorganization delivered with tabbed navigation (Profile, Integrations, Workflows, Intelligence, Hygiene, Diagnostics) for improved scannability and logical grouping.
- **I213:** Intelligence Hygiene reporting with actionable gap cards and smart navigation to People page filters. Clear status summary for entity enrichment health.
- **I140:** Branded OAuth experience shipped with success/error/cancel callback pages, inline auth errors, and visual consistency with DailyOS branding.

---

## Sprint 20 — Internal Teams (ADR-0070) ✓ COMPLETE

*Internal team entities + onboarding context priming.*

- **I209:** Internal organization + team entities implemented (ADR-0070). Parent-child account hierarchy with `account_team` migration. Internal teams now fully supported alongside external customers.
- **I210:** BU/child entity creation UI shipped. Shared bulk create form component for BU/child entity creation working across both external and internal accounts.
- **I211:** Onboarding to first briefing flow enhanced with calendar-aware context priming. People linking during setup. Focus-loss bug fixes. Entity intelligence primed from day one.

---

## Sprint 21 — Entity Management & People ✓ COMPLETE

*Account teams, people deduplication, entity lifecycle.*

- **I207:** Account team model upgraded with People entity links and roles. Account detail page shows person lists and child accounts section.
- **I172:** Duplicate people detection via hygiene scanner. Detects and suggests merging of duplicate person records.
- **I161:** Auto-unarchive suggestion on meeting detection. System suggests unarchiving accounts when meetings from archived accounts are detected.
- **I162:** Bulk account creation form shipped for efficient account onboarding.

---

## Sprint 22 — Proactive Intelligence ✓ COMPLETE

*Week page suggestions, live query layer, prep assistance, email intelligence extraction.*

- **I200:** Week page renders proactive suggestions from week-overview artifact. Surfaces suggestions in UI with contextual reasoning.
- **I201:** Live proactive suggestions via query layer (ADR-0062). Real-time suggestion generation from current state instead of point-in-time briefing data.
- **I202:** Prep prefill + draft agenda actions (ADR-0065-aware). Actionable suggestions that prepopulate prep fields and generate agenda drafts for meetings.
- **I215:** Email intelligence extraction + entity linkage. Extracts intelligence signals from emails (expansion signals, questions, timeline changes) and flows into entity intelligence for persistent meeting context.

**Delivered (Sprint 22):** Proactive suggestions layer shipped end-to-end. Week page renders real-time suggestions with live query-based computation. Email intelligence extraction feeds meeting prep with persistent entity linkage. Prep actions now actionable via prefill + agenda generation. All four issues unified as comprehensive proactive intelligence system.

---

## Sprint 23 — Auto-Update Infrastructure (ADR-0071, ADR-0072)

*Schema migration framework + Tauri auto-updater. Eliminates manual DMG distribution.*

### Schema Migration Framework (ADR-0071)
- **I175 Phase 1:** Numbered SQL migration system (`migrations.rs`). Compile-time embedded SQL files via `include_str!`. `schema_version` table tracks applied migrations. Bootstrap detection marks v1 as applied for existing databases (zero SQL runs on upgrade). Forward-compat guard rejects databases from newer DailyOS versions. Pre-migration hot backup via `rusqlite::backup::Backup`. Consolidated `schema.sql` + 22 inline ALTER TABLE migrations into `001_baseline.sql`. Deleted `schema.sql`. 5 new migration tests.

### Auto-Updater (ADR-0072)
- **I175 Phase 2:** Tauri updater plugin with Ed25519 update signing. GitHub Releases as update manifest server (`latest.json`). `UpdateCard` on SettingsPage — check for updates, download + install, app restart via `@tauri-apps/plugin-process`.
- **I175 Phase 3:** CI workflow rewrite — Apple Developer ID Application code signing, notarization, signed DMG + `.tar.gz` + `.tar.gz.sig` updater artifacts, `latest.json` generation, version-tag consistency check. Keychain cleanup step.
- **I175 Phase 4:** `UpdateCard` component with states: idle → checking → available (version + release notes) → installing → restarted. Uses `@tauri-apps/plugin-updater` JS API directly.

### Version Alignment
- Version bumped to `0.7.3` across `tauri.conf.json`, `Cargo.toml`, `package.json`.
- `createUpdaterArtifacts: true` in bundle config.
- macOS `minimumSystemVersion: "13.0"` (Ventura+).
- `updater:default` + `process:allow-restart` capabilities added.

---

## Sprint 24 — Delight & Personality ✓ COMPLETE

*Whimsy, fun, and moments of delight. Personality system, empty state humor, monthly celebration, notifications.*

- **I216:** Personality/tone picker shipped in Settings. Witty mode as default with Professional/Encouraging fallbacks. User-selectable personality preference.
- **I217:** Empty state personality with easter egg cringe humor. Comprehensive quote library from diverse comedy sources with 5-10% easter egg discovery rate.
- **I218:** Monthly "Wrapped" celebration feature. Stats + compliment quotes from transcripts. Emotional storytelling celebrating user achievements.
- **I219:** User name capture for transcript identification. Enables personalized features and proper attribution in generated content.
- **I87:** In-app notifications with personality support. Toast notifications for updates, workflows, and Wrapped celebration with personality-aware messaging.

---

## 0.7.2 — OAuth Hotfix

- **I208:** OAuth callback no longer shows "Authorization successful" before token exchange completes. Browser now waits for the full exchange + Keychain save, and shows the actual error on failure. Added logging at every step for diagnostics.

---

## Sprint 16 — Meeting Permanence + Identity Hardening

*Prelaunch refactor for durable meeting records, stable identity, and unified current/historical detail access.*

### Lifecycle + Durability
- **ADR-0065 completion:** User-authored prep fields (`userAgenda`, `userNotes`) are now DB-authoritative in `meetings_history` and protected by freeze/past editability rules.
- **Archive freeze ordering:** archive executor now persists and freezes meetings before `_today/data` cleanup, preventing prep loss.
- **Immutable snapshots:** archive path writes one frozen prep snapshot per meeting to entity `Meeting-Notes/` (fallback `_archive/meetings/YYYY/MM/`), with hash/path/frozen metadata persisted in SQLite.

### Identity + Contract
- **ADR-0066 completion:** Added unified `get_meeting_intelligence(meeting_id)` backend contract and converted `get_meeting_prep` into compatibility wrapper mode.
- **Meeting identity normalization:** event ID (sanitized) is now the canonical meeting key across poller, DB persistence, reviewed state, and dependent references (`captures`, `meeting_entities`, `meeting_attendees`, transcript actions).
- **Single meeting route behavior:** frontend now resolves historical/current meeting detail through canonical `/meeting/$meetingId`; `/meeting/history/$meetingId` now redirects.

### Focus/Capacity
- **I178:** Closed. Focus available blocks now compute from live calendar events with schedule `startIso` fallback only when live events are unavailable (ADR-0062 completion).
- **I179:** Closed. Focus now ranks pending/waiting actions deterministically with urgency and feasibility scoring, surfaces top 3 recommendations, and flags at-risk actions.

### Security/Auth Hardening
- **I158:** OAuth hardening completed with PKCE (`S256`) + state validation, macOS Keychain token storage (legacy `~/.dailyos/google/token.json` one-time migration + removal), and secretless default token exchange/refresh paths with compatibility fallback for legacy clients.

### Outcomes
- Outcomes retrieval no longer requires transcript-record file state; DB transcript metadata + captures/actions now drive outcomes durability.

---

## Sprint 15 — Meeting Intelligence Report

*Report-grade prep UX and semantic cleanup built on Sprint 14 foundation.*

### Meeting Prep Experience
<a name="i187"></a>- **I187:** Prep page three-tier layout delivered on `MeetingDetailPage` with executive brief hero, agenda-first flow, deep-context appendix, and report-style visual hierarchy.
<a name="i189"></a>- **I189:** Meeting prep editability shipped: user agenda + notes persisted and editable from prep detail with future-meeting guardrails.
<a name="i191"></a>- **I191:** Card-detail unification landed: prep/outcomes flow moved toward a unified meeting record presentation.
<a name="i194"></a>- **I194:** User agenda and notes edits persist immediately to meeting prep JSON via new Tauri commands (`update_meeting_user_agenda`/`update_meeting_user_notes`), and the UI surfaces inline edit controls plus save/failure states (`ADR-0065`).
<a name="i195"></a>- **I195:** Meeting outcomes now render inside the prep/outcomes section (depending on `MeetingOutcomes`/`MeetingDetailPage` wiring) so outcomes surface at the top even when post-meeting captures arrive earlier (`ADR-0066`).

### Prep Semantics
- **I196:** Prep agenda/wins semantic split + source governance completed. `recentWins` and `recentWinSources` added as first-class prep fields (additive/backward compatible with `talkingPoints`). Enrichment parser now supports distinct `AGENDA` and `WINS` blocks, strips inline `source:` tails from display text, and persists source provenance structurally. Mechanical agenda generation now prioritizes open items/risks/questions and only falls back to wins when needed. Added one-time migration command `backfill_prep_semantics(dry_run)` to upgrade `_today/data/preps/*.json` and `meetings_history.prep_context_json`.

### Backlog & ADR Alignment
- **I95:** Week proactive suggestions scope split into three executable tracks in `BACKLOG.md`: `I200` (Week UI rendering from week artifact), `I201` (live proactive suggestions via ADR-0062 query boundary), and `I202` (prep prefill/draft agenda actions aligned with ADR-0065 additive edit model). ADR-0052 now includes a dated alignment note recording shipped vs remaining Phase 3 scope.

### Runtime Reliability
- **I197:** Resume responsiveness hardening completed. Added in-memory command latency rollups (`p50`/`p95`/max, budget violations, degraded counters) via `get_latency_rollups` + devtools panel, expanded instrumentation for startup/resume-sensitive commands, and standardized hot-path DB access with `AppState` helper methods (`with_db_try_read`/`with_db_read`/`with_db_write`) plus staged split-lock migration guidance (ADR-0067).
- **0.7.1 fast-follow (in progress):** Production-target Rust clippy now passes with `-D warnings` (I149). Added CI gates for clippy + dependency audit (`cargo audit`, `pnpm audit`) and repository `audit.toml` (I150). Began IPC boundary hardening by moving action create/update commands to request DTOs with centralized validators (I151). Added shared Google API retry/backoff helper and wired auth/calendar/gmail calls through it (I155). Migrated UI primitives off `radix-ui` umbrella imports to explicit `@radix-ui/react-*` imports in `src/components/ui` (I157). Added repeatable measurement scripts/docs scaffolding for binary/bundle metrics (I153/I154).

---

## Sprint 14 — Meeting Intelligence Foundation

*Calendar/plumbing reliability work that unblocked the report redesign.*

### Closed
- **I177:** Email sync reliability completed end-to-end. `emails.json` now carries structured `sync` health metadata, fetch/delivery failures preserve last-known-good email lists, dashboard surfaces persistent email sync state, and manual refresh now returns blocking failures with explicit retry. Email enrichment retries once with synthesis model when extraction model is unavailable.
- **I173:** Enrichment responsiveness fixed with split-lock enrichment path reuse and `nice -n 10` PTY execution support.
- **I185:** Calendar description pipeline completed end-to-end and exposed in prep as `calendarNotes`.
- **I186:** Account snapshot enrichment completed with compact prep snapshot rendering and sanitization.
- **I190:** Meeting route migration completed (`/meeting/$meetingId`) with DB/disk fallback prep loading.
- **I159:** People-aware prep support for internal meeting types added via person-prep eligibility path.

---

## Sprint 13 — Entity Relationships & Domain Intelligence

*Auto-linking people to entities via meetings. Multi-entity MeetingCard. Multi-domain reclassification. Theme toggle fix. Entity archive/unarchive. Strategic programs UI.*

### Entity Relationships (I184)
- **I184:** Person-entity auto-linking via meeting attendance. `cascade_meeting_entity_to_people()` links external attendees to the meeting's entity (idempotent INSERT OR IGNORE). Multi-entity MeetingCard — `add_meeting_entity` / `remove_meeting_entity` commands with full cascade (people, intelligence queue, legacy account_id). Entity chips with X to unlink, EntityPicker always available for adding more. Organization field on PersonDetailPage/PeoplePage replaced with linked account entity names (clickable links to account detail). `account_names` via GROUP_CONCAT subquery on PersonListItem. 6 new Rust tests.

### Domain Intelligence (I171)
- **I171:** Multi-domain user config — tag/chip input UX on SettingsPage (comma/Enter/Tab adds domain, X removes, Backspace deletes last, auto-save). `reclassify_people_for_domains()` re-derives internal/external relationship from email domain. `reclassify_meeting_types_from_attendees()` updates meeting types when attendee relationships change (preserves title-based types like QBR, training, all_hands). Runs on every domain config change.

### Entity Management
- **I176:** Entity archive/unarchive — `archived INTEGER DEFAULT 0` on accounts/projects/people. Archive commands with parent cascade. Archived tabs on list pages. Archive button + unarchive banner on detail pages. DB flag only, filesystem untouched.
- **I163:** Strategic programs edit UI — inline-editable ProgramRow component on AccountDetailPage. Name input, status dropdown (Active/Planning/On Hold/Complete), notes field, delete button. Debounced auto-save via `update_account_programs`.

### UX & Polish
- **I156:** Theme toggle fixed — replaced broken DropdownMenu (radix-ui dual-install issue, ADR-0060) with segmented button group (Light / Dark / System). No more dropdown portal disconnect.

### Architecture
- **I180:** Resolved by ADR-0062 (briefing artifacts vs. live queries). schedule.json stays as briefing document. Time-aware features compute from live layer via `src-tauri/src/queries/`.

---

## Sprint 12 — Meeting Intelligence Persistence

*Enriched prep context persisted to history. Meeting search across entities.*

### Meeting History & Persistence
- **I181:** Persist enriched meeting context as durable record. `prep_context_json TEXT` column in `meetings_history` (auto-migrated). `persist_meetings()` reads prep files during reconciliation, validates substantiveness, stores with COALESCE to avoid overwrites. `PrepContext` struct + `PrepContextCard` component render agenda, talking points, risks, stakeholder insights, and open items on MeetingHistoryDetailPage.
- **I183:** Meeting search — cross-entity historical lookup. `search_meetings` Tauri command with SQL LIKE over title, summary, prep_context_json (LIMIT 50). CommandMenu (Cmd+K) wired with debounced search (250ms, min 2 chars) + navigation to meeting detail. Also fixed CommandMenu nav items to actually route.
- **I182:** Wire daily preps to consume entity intelligence — already delivered in Sprint 9 (I135). `inject_entity_intelligence()` reads intelligence.json, `entityReadiness`, `intelligenceSummary`, `entityRisks`, `stakeholderInsights` flow into enriched preps.

---

## Sprint 11 — Meeting Identity + Prep Reliability

*Calendar event IDs become the canonical meeting key. Prep detection fixed. People merge shipped.*

### Meeting Identity
- **I165:** Calendar event ID as meeting primary key (ADR-0061). `meeting_primary_id()` prefers Google Calendar event ID, falls back to slug. Prep + schedule use same ID function.
- **I168:** Account resolution fallback — junction table lookup + attendee inference when `guess_account_name` fails.
- **I160:** Calendar-to-people sync — `populate_people_from_events()` records attendance on every poll. Meeting counts, last_seen, temperature/trend signals now work.

### Prep Quality
- **I166:** Empty prep page fix — `is_substantive_prep()` checks for real content, `reconcile_prep_flags()` updates schedule.json. Frontend shows "generating" message instead of blank.

### Operations
- **I174:** Model tiering for AI operations — `ModelTier` enum (Synthesis/Extraction/Mechanical), `AiModelConfig` with serde defaults (sonnet/sonnet/haiku), `PtyManager::for_tier()`, Settings UI.
- **I170:** People merge + delete — full cascade (attendees, entities, actions, intelligence), filesystem cleanup, PersonDetailPage merge/delete UI, AlertDialog component. Phase 1 of merge/dedup.

### Polish
- **I167:** Calendar poller polls immediately on startup (5s auth delay) instead of sleeping first.
- **I169:** People page refresh button spins during fetch, disables to prevent double-clicks.

---

## Sprint 10 — Entity Intelligence Architecture

*Nine-phase intelligence pipeline. Proactive maintenance. Content indexing. ADR-0057/0058.*

### Entity Intelligence Pipeline (ADR-0057)
- **I130:** intelligence.json schema, `entity_intelligence` DB table, TypeScript types. Foundation.
- **I131:** Full enrichment engine — context builder, entity-parameterized prompt (initial + incremental), structured parser, PTY orchestrator.
- **I132:** IntelligenceQueue with priority-based dedup, debounce, background processor. Watcher + inbox pipeline integration.
- **I133:** AccountDetailPage intelligence-first redesign — executive assessment, attention items, meeting readiness, stakeholder intelligence, evidence history.
- **I134:** Shared `format_intelligence_markdown()` in entity_intel.rs. Accounts, projects, and people share markdown generation.
- **I135:** meeting_context.rs reads intelligence.json for prep. Calendar-triggered readiness refresh.
- **I136:** People intelligence enrichment from SQLite signals (meetings, entity connections, captures).
- **I137:** Briefing + weekly enrichment prompts include cached entity intelligence. Brief DB lock pattern (microsecond read, release before PTY).
- **I138:** Project content indexing delegates to shared `sync_content_index_for_entity()`. Watcher integration.

### Proactive Intelligence Maintenance (ADR-0058)
- **I145:** Hygiene scanner — gap detection, mechanical fixes (reclassification, orphan linking, meeting recount, file summary backfill), 4-hour background cycle. 19 tests.
- **I146:** Email display name extraction, auto-link people by domain, AI-budgeted gap filling with daily reset.
- **I147:** Pre-meeting intelligence refresh (2h window, 7d staleness). Overnight batch with expanded AI budget.
- **I148:** `get_hygiene_report` command. System Health card on SettingsPage.

### Content Indexing
- **I124:** `content_index` table, recursive directory scanner, startup sync, `get_entity_files` + `reveal_in_finder` commands, Files card on AccountDetailPage. 409 tests.
- **I125:** `AccountContent` watch source, debounced content change events, "new files detected" banner.
- **I126:** Superseded by I130. Basic `build_file_context()` delivered with I124.

### Entity Pages & CRUD
- **I50:** Projects as first-class entities — overlay table, CRUD, ProjectsPage + ProjectDetailPage.
- **I52:** Meeting-entity M2M junction table. Auto-association from attendee domains. EntityPicker on MeetingCard. Entity directory template (ADR-0059).
- **I129:** People editability — editable names, account linking, manual creation, promoted notes.
- **I127:** `create_action` with full field support. `useActions` hook. Inline "Add action" on ActionsPage + AccountDetailPage.
- **I128:** `update_action` with partial-field updates. ActionDetailPage with click-to-edit fields.

### Other
- **I94:** Week AI enrichment — `weekNarrative` + `topPriority` fields. ADR-0052.
- **I119:** Gmail header extraction works correctly in Rust-native `gmail.rs`.
- **I123:** Production Google OAuth credentials embedded. DailyOS Google Cloud project.
- **I139:** File summary extraction via hygiene scanner backfill.
- **I144:** `archive_emails` via Gmail `batchModify` API. "Archive all" button on FYI section.

---

## Sprint 9 — Entity Relationship Graph

*Accounts, projects, and people become connected entities with M2M relationships.*

- **I50:** Projects as first-class entities.
- **I52:** Meeting-entity M2M junction table + EntityPicker on MeetingCard.
- **I129:** People entity editability.

---

## Sprint 8 — Python Elimination (ADR-0049)

*All Google API calls ported to Rust via `reqwest`. Python runtime removed entirely.*

- **I83:** Rust-native Google API client (`google_api/` module).
- **I84:** Phase 1 operations ported to Rust (`prepare/` module).
- **I85:** Orchestrators ported, `scripts/` directory deleted.
- **I91:** Universal file extraction (`processor/extract.rs`). ADR-0050.

---

## Sprint 7 — UX Redesign

*Schedule-first dashboard (ADR-0055), list page redesign (ADR-0054), focus page.*

### Dashboard
<a name="i97"></a>
- **I97:** Readiness strip (later removed by ADR-0055).
<a name="i98"></a>
- **I98:** Action/email sidebar order flipped.
<a name="i99"></a>
- **I99:** Greeting removed, Focus promoted.
<a name="i100"></a>
- **I100:** ActionList maxVisible 3 → 5.
<a name="i101"></a>
- **I101:** Full-width summary (later superseded by ADR-0055 two-column).
<a name="i109"></a>
- **I109:** Focus page — `get_focus_data` from schedule.json + SQLite + gap analysis.
- **I111:** Dashboard visual rhythm — removed chrome, tapered spacing, breathing room.
- **I112:** Graceful empty state — missing schedule.json returns `Empty` not `Error`.

### List Pages (ADR-0054)
- **I102:** Shared `ListRow` + `ListColumn` primitives.
- **I103:** AccountsPage flat rows with health dot.
- **I104:** PeoplePage flat rows with temperature + trend.
- **I105:** PeoplePage shared component consolidation (SearchInput, TabFilter).
- **I106:** `PersonListItem` struct + batch `get_people_with_signals()` query.
- **I107:** Action detail page at `/actions/$actionId`.

### Week Page (ADR-0052)
- **I93:** Week page mechanical redesign — consumption-first layout.
- **I96:** Week planning wizard retired.

### Entity Hierarchy (ADR-0056)
- **I113:** Workspace transition detection. Auto-scaffold, skip `_`-prefixed folders.
- **I114:** Parent-child accounts — `parent_id` FK, expandable rows, breadcrumb, rollup.
<a name="i116"></a>
- **I116:** ActionsPage account name resolution via `ActionListItem`.
<a name="i117"></a>
- **I117:** `guess_account_name()` discovers child BU directories.
- **I118:** Timezone formatting in `deliver.rs`, `orchestrate.rs`, `calendar_merge.rs`.

---

## Sprint 6 — Account Pages & Enrichment

*Entity dashboard system, enrichment pipeline, account detail pages.*

- **I72:** AccountsPage + AccountDetailPage. 6 Tauri commands.
- **I73:** Entity dashboard template system, two-file pattern, three-way sync. ADR-0047.
- **I74:** Account enrichment via Claude Code websearch.
- **I75:** Entity dashboard external edit detection via watcher.
- **I76:** SQLite backup + rebuild-from-filesystem.
- **I77:** Filesystem writeback audit.
- **I78:** Onboarding: inbox-first behavior chapter.
- **I79:** Onboarding: Claude Code validation chapter.
- **I80:** Proposed Agenda in meeting prep.
- **I81:** People dynamics in meeting prep UI.
- **I82:** Copy-to-clipboard for meeting prep.

---

## Sprint 5 — Onboarding & Security Hardening

*Educational onboarding flow, atomic writes, security audit.*

### Onboarding
- **I56:** Onboarding redesign — 9-chapter educational flow.
- **I57:** Onboarding: populate workspace before first briefing.
- **I58:** User profile context in AI enrichment prompts.

### Security & Robustness
- **I59:** Script path resolution for production builds.
- **I60:** Path traversal validation in inbox/workspace.
- **I61:** TOCTOU sentinel for transcript immutability.
- **I62:** `.unwrap()` panics replaced with graceful handling.
- **I63:** Python script timeout handling.
- **I64:** Atomic file writes via `atomic_write_str()`.
- **I65:** Impact log append safety.
- **I66:** Safe prep delivery (write-first, then remove stale).
- **I67:** Scheduler boundary widened 60 → 120 seconds.
- **I68:** `Mutex<T>` → `RwLock<T>` for read-heavy AppState.
- **I69:** File router duplicate destination handling.
- **I70:** `sanitize_for_filesystem()` strips unsafe characters.
- **I71:** Low-severity edge hardening (9 items).

---

## Sprint 4 — Workflow & Intelligence

*Archive reconciliation, executive intelligence, entity abstraction.*

- **I34:** Archive reconciliation (`workflow/reconcile.rs`).
- **I36:** Daily impact rollup (`workflow/impact_rollup.rs`).
<a name="i37"></a>
- **I37:** Density-aware dashboard overview.
<a name="i38"></a>
- **I38:** Rust-native delivery + AI enrichment ops. ADR-0042.
- **I39:** Feature toggle runtime with `is_feature_enabled()`.
<a name="i41"></a>
- **I41:** Reactive meeting:prep wiring via calendar poller.
<a name="i42"></a>
- **I42:** CoS executive intelligence layer (`intelligence.rs`).
<a name="i43"></a>
- **I43:** Stakeholder context in meeting prep.
<a name="i44"></a>
- **I44:** Meeting-scoped transcript intake. ADR-0044.
<a name="i45"></a>
- **I45:** Post-transcript outcome interaction UI.
- **I46:** Meeting prep context expanded beyond customer/QBR/training. ADR-0043.
- **I47:** Entity abstraction with `entities` table + bridge pattern. ADR-0045.
- **I48:** Workspace scaffolding on initialization.
- **I49:** Graceful degradation without Google auth.
- **I51:** People sub-entity — universal person tracking. 3 tables, 8 commands.

---

## Sprint 3 — Inbox Pipeline

*File processing, transcript extraction, account intelligence updates.*

<a name="i30"></a>
- **I30:** Inbox action extraction with rich metadata (`processor/metadata.rs`).
<a name="i31"></a>
- **I31:** Inbox transcript summarization with `detect_transcript()` heuristic.
- **I32:** Inbox processor updates account intelligence via WINS/RISKS extraction.
<a name="i33"></a>
- **I33:** Wins/risks resurface in meeting preps via 14-day lookback.

---

## Sprints 1-2 — Foundation

*Core app shell, data loading, pages, CI.*

- **I1:** Config directory renamed `.daybreak` → `.dailyos`.
- **I2:** JSON-first meeting cards render at multiple fidelity levels.
- **I4:** Superseded by I89 (personality system).
- **I5:** Focus, Week, Emails all have defined roles. ADR-0010.
- **I6:** Processing history page + `get_processing_history` command.
- **I7:** Settings workspace path change with directory picker.
- **I8:** GitHub Actions CI — unsigned arm64 DMG on tag push. Product website (daily-os.com).
- **I9:** Focus and Week pages fully implemented.
- **I10:** Closed, won't do. Types are the data model.
- **I11:** Email enrichment parsed and merged into `emails.json`.
- **I12:** Email page shows AI context per priority tier.
- **I13:** Onboarding wizard with 5-step flow.
<a name="i14"></a>
- **I14:** MeetingCard "View Prep" button.
- **I15:** Entity-mode switcher in Settings.
- **I16:** Schedule editing UI with human-readable cron.
<a name="i17"></a>
- **I17:** Non-briefing actions merge into dashboard.
<a name="i18"></a>
- **I18:** Google API credential caching.
- **I19:** "Limited prep" badge for AI enrichment failures.
- **I20:** Standalone email refresh.
- **I21:** Expanded FYI email classification.
- **I22:** Action completion writeback to source markdown.
- **I23:** Three-layer cross-briefing action deduplication.
<a name="i24"></a>
- **I24:** `calendarEventId` field alongside local slug.
- **I25:** `computeMeetingDisplayState()` unified badge rendering.
<a name="i29"></a>
- **I29:** Superseded by I73 template system + kit issues.
- **I120:** Closed, won't fix. Legacy action import — starting clean.
- **I121:** Closed, won't fix. Legacy prep generation — clean start.
