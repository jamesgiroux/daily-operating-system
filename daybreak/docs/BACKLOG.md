# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** 560 Rust tests. v0.7.0-alpha shipped. Sprints 1-13 complete. Sprint 14 closed. Sprint 15 in progress. 0.7.1 fast-follow parallel.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I177** | Email sync silently fails post-model-tiering | Blocker | Data |
| **I158** | OAuth PKCE + Keychain storage | Blocker | Security |
| **I178** | Focus page available time is incorrect | P0 | UX |
| **I179** | Focus page action prioritization intelligence | P0 | UX |
| **I149** | Cargo clippy zero warnings | P0 | Infra |
| **I150** | Dependency security audit | P0 | Security |
| **I151** | Input validation (IPC boundary) | P0 | Security |
| **I152** | Error handling (eliminate panics) | P0 | Infra |
| **I188** | Agenda-anchored AI enrichment (ADR-0064 P4) | P1 | Meetings |
| **I153** | Binary size + startup perf | P1 | Infra |
| **I154** | Frontend bundle audit | P1 | Infra |
| **I155** | Rate limiting + retry hardening | P1 | Infra |
| **I157** | Frontend component audit (radix-ui) | P1 | UX |
| **I164** | Inbox processing status indicators | P1 | UX |
| **I161** | Auto-unarchive on meeting detection | P2 | Entity |
| **I162** | Bulk account creation | P2 | Entity |
| **I172** | Duplicate people detection | P2 | Entity |
| **I110** | Portfolio alerts on sidebar | P2 | UX |
| **I115** | Multi-line action extraction | P2 | Data |
| **I122** | Sunday briefing mislabeled as "today" | P2 | Meetings |
| **I26** | Web search for unknown meetings | P2 | Meetings |
| **I95** | Week page proactive suggestions | P2 | Meetings |
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

**I177: Email sync silently fails post-model-tiering — CRITICAL BUG**
Daily Briefing now emits enrichment warning events, but mechanical delivery failures still fall back to empty payloads and need a fully-visible user path. Remaining work: (1) unify `email-error` + `email-enrichment-warning` handling in frontend to always show failure state, (2) confirm model-tier fallback behavior when extraction model is unavailable, (3) verify low-friction recovery action from UI.

**I158: OAuth PKCE + credential hardening**
Three layers: (1) PKCE flow (RFC 7636) — eliminates `client_secret` from source. (2) macOS Keychain for token storage — move from plaintext `~/.dailyos/google/token.json`. (3) Rotate current credentials after PKCE ships.

---

## P0 Critical Issues

**I178: Focus page available time is incorrect — doesn't read actual calendar**
Focus page shows full-day available time even when schedule is packed with meetings. Root causes: (1) **No calendar integration:** available time calculation doesn't reference actual calendar/schedule. (2) **No definition of "available":** unclear what the metric means (contiguous blocks? fragmented gaps? excludes prep time?). (3) **No deep work concept:** doesn't distinguish between meeting gaps (15 min) and meaningful work blocks (60+ min). (4) **Silent failure:** if calculation breaks, user doesn't know. Impact: User can't assess actual capacity for the day. Architectural decision: ADR-0062 (query from live layer, not schedule.json). Fix: (a) Wire available time to actual calendar events from schedule.json. (b) Define rules (e.g., "contiguous blocks 30+ min," exclude buffer before/after meetings, account for context switching). (c) Add concept of "deep work" blocks (e.g., 60+ min uninterrupted). (d) Show breakdown (X meetings, Y hours in meetings, Z hours available, W hours deep work potential).

**I179: Focus page actions are not prioritized — missing intelligence layer**
Focus page lists all top actions but doesn't prioritize based on time/capacity. Three problems: (1) **No top 3:** shows a flat list, not "if you do nothing else, these 3 things." (2) **Ignores actual available time:** doesn't filter/rank by feasibility given meeting load. (3) **Missing implications:** no synthesis about what's achievable vs. at-risk. If user has 2 hours and 5 actions, which ones matter? If day is 90% meetings, which 1 action is critical? Impact: User stares at 8 actions with no guidance on what to prioritize. Scope: (a) Calculate achievable action count given available time (I178 feeds this). (b) AI-enrich action list with urgency/impact signals (due date, blocking other actions, customer-facing). (c) Synthesize top 3 with rationale ("You have 90 min; recommend these 3 because..."). (d) Flag at-risk items (blocked by unavailable time or dependencies). Depends on I178 (available time calculation).

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

## Sprint 14 — Meeting Intelligence Foundation

*Calendar descriptions, enriched account snapshots, route migration, email fix. The data layer and plumbing that enables the prep page redesign (ADR-0064/0065/0066).*

| Priority | Issue | Scope | Depends On | Status |
|----------|-------|-------|------------|--------|
| Blocker | I177 | Email sync fix — surface failures, fallback to mechanical | — | Partial |
| Blocker | I173 | Enrichment responsiveness — split-lock pattern + nice | — | Closed |
| P0 | I185 | Calendar description pipeline — schema + plumb through 5 stages | — | Closed |
| P0 | I186 | Account Snapshot enrichment — intelligence signals in prep | — | Closed |
| P0 | I190 | Meeting route migration — /meeting/$meetingId + unified command | — | Closed |
| P1 | I159 | People-aware prep for internal meetings | — | Closed |

**Rationale:** Phases 1-2 of ADR-0064 and Phase 1 of ADR-0066 are pure plumbing — mechanical schema changes, data flow fixes, and route migration. No AI prompt redesign, no layout overhaul. They unblock Sprint 15 (the visual redesign + agenda-anchored enrichment). The two blockers (I177, I173) ship alongside because they affect daily usability. I159 extends prep coverage to internal meetings while we're already in the prep pipeline.

**Closed in Sprint 14:** I173, I185, I186, I190, I159.  
**Carryover to Sprint 15:** I177 (partial), I188 (partial).

---

## Sprint 15 — Meeting Intelligence Report

*Report-style prep UX and semantic cleanup on top of Sprint 14 plumbing.*

| Priority | Issue | Scope | Status |
|----------|-------|-------|--------|
| P1 | I187 | Prep page three-tier layout (ADR-0064 P3) | Closed |
| P1 | I188 | Agenda-anchored AI enrichment (ADR-0064 P4) | Partial |
| P1 | I189 | Meeting prep editability (ADR-0065) | Closed |
| P1 | I191 | Card-detail visual unification (ADR-0066 P2-3) | Closed |
| P1 | I196 | Prep agenda/wins semantic split + source governance | Closed |

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

**I95: Week page Phase 3 — proactive suggestions (ADR-0052)**
Draft agenda requests, pre-fill preps, suggest tasks for open blocks. Time blocking as AI-driven setting.

### Entity Management

**I161: Auto-unarchive suggestion on meeting detection**
When classification matches an archived account's domain, surface suggestion on MeetingCard rather than silently unarchiving. Depends on I176 (shipped Sprint 13).

**I162: Bulk account creation**
Multi-line textarea mode on AccountsPage/ProjectsPage inline create. One name per line, batch create. Extract shared `BulkCreateForm` component.

**I172: Duplicate people detection**
Hygiene scanner heuristics: group by email domain → compare normalized names. `DuplicateCandidate` type. PeoplePage banner + PersonDetailPage merge shortcut. Phase 3 of merge/dedup.

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
| I27 | Entity-mode architecture (umbrella) | — |
| I40 | CS Kit — account-mode fields + templates | I27 |
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
