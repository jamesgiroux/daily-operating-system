# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** 581 Rust tests. v0.7.0-alpha shipped. Sprints 1-14 complete. Sprint 15 closed (I188 carryover). Sprint 16 in progress. 0.7.1 fast-follow parallel.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I158** | OAuth PKCE + Keychain storage | Blocker | Security |
| **I149** | Cargo clippy zero warnings | P0 | Infra |
| **I150** | Dependency security audit | P0 | Security |
| **I151** | Input validation (IPC boundary) | P0 | Security |
| **I152** | Error handling (eliminate panics) | P0 | Infra |
| **I188** | Agenda-anchored AI enrichment (ADR-0064 P4) | P1 | Meetings |
| **I153** | Binary size + startup perf | P1 | Infra |
| **I197** | Resume responsiveness hardening (startup + auth checks) | P1 | Infra |
| **I154** | Frontend bundle audit | P1 | Infra |
| **I155** | Rate limiting + retry hardening | P1 | Infra |
| **I157** | Frontend component audit (radix-ui) | P1 | UX |
| **I164** | Inbox processing status indicators | P1 | UX |
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

**I158: OAuth PKCE + credential hardening**
Three layers: (1) PKCE flow (RFC 7636) — eliminates `client_secret` from source. (2) macOS Keychain for token storage — move from plaintext `~/.dailyos/google/token.json`. (3) Rotate current credentials after PKCE ships.

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
| P1 | I197 | Resume responsiveness hardening (startup + auth checks) | Closed |
| P1 | I154 | Frontend bundle audit | Open |
| P1 | I155 | Rate limiting + retry hardening | Open |
| P1 | I157 | Frontend component audit (radix-ui) | Open |

---

## Sprint 14 — Meeting Intelligence Foundation

*Calendar descriptions, enriched account snapshots, route migration, email fix. The data layer and plumbing that enables the prep page redesign (ADR-0064/0065/0066).*

| Priority | Issue | Scope | Depends On | Status |
|----------|-------|-------|------------|--------|
| Blocker | I177 | Email sync fix — surface failures, fallback to mechanical | — | Closed |
| Blocker | I173 | Enrichment responsiveness — split-lock pattern + nice | — | Closed |
| P0 | I185 | Calendar description pipeline — schema + plumb through 5 stages | — | Closed |
| P0 | I186 | Account Snapshot enrichment — intelligence signals in prep | — | Closed |
<a name="i190"></a>| P0 | I190 | Meeting route migration — /meeting/$meetingId + unified command | — | Closed |
| P1 | I159 | People-aware prep for internal meetings | — | Closed |

**Rationale:** Phases 1-2 of ADR-0064 and Phase 1 of ADR-0066 are pure plumbing — mechanical schema changes, data flow fixes, and route migration. No AI prompt redesign, no layout overhaul. They unblock Sprint 15 (the visual redesign + agenda-anchored enrichment). The two blockers (I177, I173) ship alongside because they affect daily usability. I159 extends prep coverage to internal meetings while we're already in the prep pipeline.

**Closed in Sprint 14:** I177, I173, I185, I186, I190, I159.  
**Carryover to Sprint 15:** I188 (partial).

---

## Sprint 15 — Meeting Intelligence Report

*Report-style prep UX and semantic cleanup on top of Sprint 14 plumbing.*

| Priority | Issue | Scope | Status |
|----------|-------|-------|--------|
<a name="i187"></a>| P1 | I187 | Prep page three-tier layout (ADR-0064 P3) | Closed |
<a name="i188"></a>| P1 | I188 | Agenda-anchored AI enrichment (ADR-0064 P4) | Partial |
<a name="i189"></a>| P1 | I189 | Meeting prep editability (ADR-0065) | Closed |
| P1 | I191 | Card-detail visual unification (ADR-0066 P2-3) | Closed |
| P1 | I196 | Prep agenda/wins semantic split + source governance | Closed |

### Meeting Preview Context (ADR-0063)

Recent Meetings cards now surface a trimmed `prepContext` (intelligence summary, agenda excerpt, and risk/action/question counts) so the account detail page surfaces evaluation-ready context without navigating to the prep detail. This builds on the `/meeting/$meetingId` migration (I190) to fetch the prep context for each meeting.

## Sprint 16 — Meeting Permanence + Identity Hardening

*Prelaunch contract-hardening for durable meeting records, canonical identity, and unified historical/current reads.*

| Priority | Issue | Scope | Status |
|----------|-------|-------|--------|
| P0 | I178 | Focus available-time correctness via live calendar query module (ADR-0062 completion) | Closed |
| P0 | ADR-0065 | DB-authoritative user agenda/notes + freeze-aware editability | Closed |
| P0 | ADR-0066 | Unified `get_meeting_intelligence` backend payload + single meeting detail path | Closed |
| P0 | ADR-0061 | Event ID canonicalization/backfill across meeting-linked tables | Closed |
| P0 | Permanence | Immutable prep snapshot freeze at archive with SQLite metadata | Closed |

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

### Infra & Runtime

**I197: Resume responsiveness hardening (startup + auth checks)**
Goal: eliminate avoidable UI stalls after focus return and on cold startup.

Phase 1 completed (2026-02-12):
- moved startup sync/indexing off `AppState::new()` onto a background task
- bounded Claude auth check with timeout + forced process cleanup
- applied lock-scope reductions and non-blocking reads in dashboard/focus paths
- added lock-contention fallbacks + latency instrumentation for focus-polled commands
- throttled/deduped focus refresh requests in dashboard hook

Phase 2 completed (2026-02-12):
- added in-memory latency rollups (`p50`/`p95`/max, budget violations, degraded counters) and exposed diagnostics via `get_latency_rollups` + devtools panel
- expanded instrumentation coverage for startup/resume-sensitive commands (`get_dashboard_data`, `get_focus_data`, `check_claude_status`, `get_google_auth_status`, workflow status/history scheduling reads)
- introduced `AppState` DB helper API (`with_db_try_read`, `with_db_read`, `with_db_write`) and migrated highest-contention hot reads to helper-based non-blocking access
- documented staged split-lock DB strategy and migration roadmap in ADR-0067

### UX & Polish

**I157: Frontend component audit**
Audit all `src/components/ui/` for remaining standalone `@radix-ui/*` imports, stale forwardRef patterns, hand-rolled UI that shadcn provides. ADR-0060.

**I110: Portfolio alerts on accounts sidebar/list**
IntelligenceCard removed (ADR-0055). Renewal + stale contact alerts need a new home. `intelligence.rs` computation exists — purely frontend wiring.

**I164: Inbox file processing status**
Processing state lives only in React memory. Cross-reference inbox files with `processing_log` on load. Show status indicators (unprocessed vs processed). Make Process button visible by default.

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
