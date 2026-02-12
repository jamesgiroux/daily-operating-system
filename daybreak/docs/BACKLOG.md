# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** 581 Rust tests. v0.7.0-alpha shipped. Sprints 1-16 complete. Sprint 17 planning. 0.7.1 fast-follow parallel.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I179** | Focus page action prioritization intelligence | P0 | UX |
| **I149** | Cargo clippy zero warnings | P0 | Infra |
| **I150** | Dependency security audit | P0 | Security |
| **I151** | Input validation (IPC boundary) | P0 | Security |
| **I152** | Error handling (eliminate panics) | P0 | Infra |
| **I153** | Binary size + startup perf | P1 | Infra |
| **I154** | Frontend bundle audit | P1 | Infra |
| **I155** | Rate limiting + retry hardening | P1 | Infra |
| **I157** | Frontend component audit (radix-ui) | P1 | UX |
| **I164** | Inbox processing status indicators | P1 | UX |
| **I203** | Inbox dropzone duplicate file bug | P1 | UX |
| **I188** | Agenda-anchored AI enrichment (ADR-0064 P4) | P2 | Meetings |
| **I161** | Auto-unarchive on meeting detection | P2 | Entity |
| **I162** | Bulk account creation | P2 | Entity |
| **I172** | Duplicate people detection | P2 | Entity |
| **I198** | Account merge + transcript reassignment | P2 | Entity |
| **I199** | Archived account recovery UX (restore + relink) | P2 | Entity |
| **I110** | Portfolio alerts on sidebar | P2 | UX |
| **I115** | Multi-line action extraction | P2 | Data |
| **I122** | Sunday briefing mislabeled as "today" | P2 | Meetings |
| **I26** | Web search for unknown meetings | P2 | Meetings |
| **I200** | Week page renders proactive suggestions from week-overview | P2 | Meetings |
| **I201** | Live proactive suggestions via query layer (ADR-0062) | P2 | Meetings |
| **I202** | Prep prefill + draft agenda actions (ADR-0065-aware) | P2 | Meetings |
| **I140** | Branded OAuth success page | P2 | UX |
| **I141** | AI content tagging during enrichment | P2 | Data |
| **I142** | Account Plan artifact | P3 | Entity |
| **I143** | Renewal lifecycle tracking | P3 | Entity |
| **I3** | Low-friction web capture to _inbox | P3 | Data |
| **I175** | Auto-update + schema migration | Beta | Infra |
| **I87** | In-app notifications | Parked | UX |
| **I88** | Monthly Book Intelligence | Parked | Intelligence |
| **I89** | Personality system (voice picker) | Parked | UX |
| **I90** | Product telemetry | Parked | Infra |
| **I92** | User-configurable metadata fields | Parked | Entity |

---

## Ship Blockers

No open ship blockers. Last blocker closed: **I158** (OAuth PKCE + Keychain token hardening).

---

## P0 Critical Issues

---

## 0.7.1 Fast-Follow

| Priority | Issue | Scope | Status |
|----------|-------|-------|--------|
| P0 | I149 | Cargo clippy sweep (70+ warnings) | Open |
| P0 | I150 | Dependency security audit | Open |
| P0 | I151 | Input validation (Tauri IPC boundary) | Open |
| P0 | I152 | Error handling (eliminate panics) | Open |
| P1 | I153 | Binary size + startup perf | Open |
| P1 | I154 | Frontend bundle audit | Open |
| P1 | I155 | Rate limiting + retry hardening | Open |
| P1 | I157 | Frontend component audit (radix-ui) | Open |

---

## Open Issues

### Meeting Intelligence (ADR-0064, 0065, 0066)

**I188: Agenda-anchored AI enrichment (ADR-0064 Phase 4)**
Partial: agenda/wins are now semantically split (`recentWins`/`proposedAgenda`) and enrichment prompt/parser treat them separately, but explicit calendar-description agenda extraction and agenda-first anchoring logic still need dedicated completion criteria.

### Meetings & Prep

**I122: Sunday briefing fetches Monday calendar labeled as "today"**
Running briefing on Sunday produces Monday's meetings labeled "today." Either intentional (UI should say "Tomorrow") or needs calendar day fix.

**I26: Web search for unknown external meetings**
When meeting involves unrecognized people/companies, prep is thin. Extend I74 websearch pattern to unknown attendee domains. Not blocked by I27.

**I200: Week page renders proactive suggestions from week-overview**
The week pipeline already computes `dayShapes.availableBlocks` and AI can write `suggestedUse`, but WeekPage does not display these blocks today. Ship a Week section that surfaces available blocks + suggestions and links suggestions to actionable destinations where possible.

Acceptance criteria:
- WeekPage shows per-day available blocks from `dayShapes[].availableBlocks` with `start/end/duration`.
- `suggestedUse` text is visible when present.
- Suggestion rows are keyboard-accessible and render sensible empty states (no blocks / no suggestions).
- For suggestions that map to an action or meeting, UI includes a deep link (`/actions/$id` or `/meeting/$id`).

**I201: Live proactive suggestions via query layer (ADR-0062)**
Week artifact suggestions are point-in-time. For current-state recommendations, add a live query-backed suggestion path using the ADR-0062 boundary (live calendar + SQLite), not rewrites of briefing artifacts.

Acceptance criteria:
- New query functions compute current open blocks and action feasibility from live data sources.
- A Tauri command returns live proactive suggestions without mutating `schedule.json`/`week-overview.json`.
- Suggestion output includes deterministic scoring fields (capacity fit, urgency/impact, confidence) for UI ordering.
- Tests cover stale-artifact vs live-data divergence (meeting added/removed after morning run).

**I202: Prep prefill + draft agenda actions (ADR-0065-aware)**
Implement Phase 3 prep-side suggestions as explicit actions: draft agenda message and prefill prep content. Must respect ADR-0065 editability model (`userAgenda`/`userNotes`) and avoid overwriting generated prep fields.

Acceptance criteria:
- User can trigger "Draft agenda message" from week/meeting context and copy or send via explicit confirmation flow.
- User can apply "Prefill prep" suggestions into `userAgenda` and/or `userNotes`.
- Applying prefill is additive and idempotent (no clobber of `proposedAgenda`/`talkingPoints`).
- Conflict behavior is explicit when user-edited content already exists (append, merge, or confirm replace).

### Entity Management

**I161: Auto-unarchive suggestion on meeting detection**
When classification matches an archived account's domain, surface suggestion on MeetingCard rather than silently unarchiving. Depends on I176 (shipped Sprint 13).

**I162: Bulk account creation**
Multi-line textarea mode on AccountsPage/ProjectsPage inline create. One name per line, batch create. Extract shared `BulkCreateForm` component.

**I172: Duplicate people detection**
Hygiene scanner heuristics: group by email domain → compare normalized names. `DuplicateCandidate` type. PeoplePage banner + PersonDetailPage merge shortcut. Phase 3 of merge/dedup.

**I198: Account merge + transcript reassignment**
No account-level merge path today (unlike people). Need source→target merge with deterministic cascade across `meeting_entities`, `meetings_history.account_id`, `actions`, `captures`, and intelligence queue refresh. Include filesystem move/relink strategy for account folders/transcripts and conflict policy.

**I199: Archived account recovery UX (restore + relink)**
Unarchive exists but recovery flow is fragmented when users need to restore an account and reattach meetings/files. Add direct "Restore and Link" flow from meeting/account surfaces with clear archived-state affordances and post-restore reassignment actions.

**I142: Account Plan — leadership-facing artifact**
Structured Account Plan (exec summary, 90-day focus, risk table, products/adoption) generated from intelligence.json + dashboard.json. Markdown output in account directory. UI entry point on AccountDetailPage.

**I143: Renewal lifecycle tracking**
(a) Auto-rollover when renewal passes without churn. (b) Lifecycle event markers (churn, expansion, renewal) in `account_events` table. (c) UI for recording events on AccountDetailPage.

### UX & Polish

**I157: Frontend component audit**
Audit all `src/components/ui/` for remaining standalone `@radix-ui/*` imports, stale forwardRef patterns, hand-rolled UI that shadcn provides. ADR-0060.

**I110: Portfolio alerts on accounts sidebar/list**
IntelligenceCard removed (ADR-0055). Renewal + stale contact alerts need a new home. `intelligence.rs` computation exists — purely frontend wiring.

**I164: Inbox file processing status**
Processing state lives only in React memory. Cross-reference inbox files with `processing_log` on load. Show status indicators (unprocessed vs processed). Make Process button visible by default.

**I203: Inbox dropzone duplicate file bug**
When dragging and dropping a new document into the Inbox dropzone, the file appears twice in the inbox list. Likely due to double-handling in the drop event handler or sync conflict between UI state and file watcher. Root cause and fix needed.

Acceptance criteria:
- Single drag-drop of a new file adds exactly one entry to the inbox list.
- No duplicate entries appear on subsequent file syncs.
- Drop handler and file watcher don't trigger conflicting updates.

**I140: Branded Google OAuth success page**
Static HTML on localhost callback — on-brand confirmation + "what happens next" guidance. DailyOS design tokens.

### Data & Pipeline

**I115: Multi-line action extraction**
`extract_and_sync_actions()` only parses single-line checkboxes. Add look-ahead for indented `- Key: Value` sub-lines.

**I141: AI content tagging during enrichment**
Piggyback on existing enrichment call — add output field for file relevance ratings + classification tags. Store in `content_index.tags` column. Zero extra AI cost.

**I3: Low-friction web capture to _inbox/**
Browser extension, macOS share sheet, bookmarklet, or "paste URL" in-app. Form factor TBD.

---

## Beta Blocker

**I175: Auto-update + schema migration framework**
Required before 0.9 beta (20-50 users). (1) Tauri auto-updater checking GitHub Releases, signed builds. (2) Schema migration runner with `schema_version` table and numbered SQL files. Forward compat check. Alpha continues with manual DMG distribution.

---

## Parking Lot

*Post-ship. Blocked by I27 (entity-mode architecture) or needs usage data.*

| ID | Title | Blocked By |
|----|-------|------------|
<a name="i27"></a>| I27 | Entity-mode architecture (umbrella) | — |
<a name="i40"></a>| I40 | CS Kit — account-mode fields + templates | I27 |
| I53 | Entity-mode config + onboarding | I27 |
| I54 | MCP client integration framework | I27 |
| I28 | MCP server and client | I27 |
| I35 | ProDev Intelligence | I27 |
| I55 | Executive Intelligence | I27 |
| I86 | First-party integrations | I54 |
| I87 | In-app notifications | — |
| I88 | Monthly Book Intelligence | — |
| I89 | Personality system (voice picker) | — |
| I90 | Product telemetry | — |
| I92 | User-configurable metadata fields | I27 |

---

## RAIDD

### Risks

| ID | Risk | Impact | Likelihood | Mitigation |
|----|------|--------|------------|------------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events |
| R5 | Open format = no switching cost | High | Medium | Enrichment quality is the moat |
| R6 | N=1 validation — one user/role | High | High | Beta users across roles before I27 |
| R7 | Org cascade needs adoption density | Medium | High | Ship individual product first |
| R8 | Bad briefing erodes trust faster than no briefing | High | Medium | Quality metrics, confidence signals |
| R9 | Kit + Intelligence composition untested at scale | Medium | Medium | Build one Kit + one Intelligence first |

### Assumptions

| ID | Assumption | Validated |
|----|------------|-----------|
| A1 | Users have Claude Code CLI installed and authenticated | Partial |
| A2 | Workspace follows PARA structure | No |
| A3 | `_today/` files use expected markdown format | Partial |
| A4 | Users have Google Workspace (Calendar + Gmail) | No |

### Dependencies

| ID | Dependency | Type | Status |
|----|------------|------|--------|
| D1 | Claude Code CLI | Runtime | Available |
| D2 | Tauri 2.x | Build | Stable |
| D3 | Google Calendar API | Runtime | Optional |
