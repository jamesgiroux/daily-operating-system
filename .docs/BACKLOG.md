# Product Backlog

Active issues, known risks, and dependencies. Closed issues live in [CHANGELOG.md](./CHANGELOG.md).

**Convention:** Issues use `I` prefix. When resolved, move to CHANGELOG with a one-line resolution.

**Current state:** 915 Rust tests. v0.12.1 shipped (email intelligence, product language, UX polish). 0.13.0 ready to tag (meeting intelligence lifecycle, unified surfaces, surface restructure — 18 issues). 0.13.1 planned (email as intelligence input — ADR-0085). 0.13.2 planned (structural clarity — E2E audit, service extraction, god file refactor — ADR-0086). 0.13.3 planned (entity hierarchy intelligence — partner type, portfolio surfaces, bidirectional propagation — ADR-0087). 0.13.4 planned (project hierarchy intelligence — ADR-0087 applied to projects, preset entityModeDefault wired to surface ordering). 0.13.5 planned (people relationship network intelligence — graph model, typed edges, network signals — ADR-0088). 0.14.0 planned (reports + distribution). 1.0.0 = beta gate.

---

## Index

| ID | Title | Priority | Area |
|----|-------|----------|------|
| **I329** | Intelligence quality indicators — replace hasPrep badge, correct vocabulary | P1 | UX |
| **I332** | Signal-triggered meeting intelligence refresh — calendar polling trigger + pre-meeting scheduler + prep invalidation | P1 | Pipeline |
| **I341** | Product vocabulary audit — translate 5 remaining system-term strings in user-facing copy | P1 | UX |
| **I342** | Surface restructure Phase 4 — Lead Story → Up Next, Review + Priorities → Attention | P1 | UX / Product |
| **I343** | Inline editing — replace AccountFieldsDrawer + ProjectFieldsDrawer with EditableText/List | P1 | UX |
| **I351** | Actions chapter on PersonDetailEditorial | P1 | Entity / UX |
| **I356** | Thread position UI — Replies Needed subsection in daily briefing Attention section | P1 | UX / Email |
| **I358** | Email page — promote to first-class nav surface with meeting-centric organization | P1 | UX / Email |
| **I225** | Gong integration — sales call intelligence + transcripts | P1 | Integrations |
| **I280** | Beta hardening umbrella — dependency, DB, token, DRY audit | P1 | Code Quality |
| **I56** | Onboarding: educational redesign — demo data, guided tour | P0 | Onboarding |
| **I57** | Onboarding: add accounts/projects + user domain configuration | P0 | Onboarding |
| **I88** | Monthly Book Intelligence — portfolio report | P2 | Intelligence |
| **I90** | Product telemetry + analytics infrastructure | P2 | Infra |
| **I115** | Multi-line action extraction | P2 | Data |
| **I141** | AI content tagging during enrichment | P2 | Data |
| **I142** | Account Plan artifact | P2 | Entity |
| **I198** | Account merge + transcript reassignment | P2 | Entity |
| **I199** | Archived account recovery UX — restore + relink | P2 | Entity |
| **I227** | Gainsight integration — CS platform data sync | P2 | Integrations |
| **I230** | Claude Cowork integration — project/task sync | P2 | Integrations |
| **I258** | Report Mode — export account detail as leadership-ready deck/PDF | P2 | UX |
| **I277** | Marketplace repo for community preset discoverability | P3 | Integrations |
| **I302** | Shareable PDF export for intelligence reports (editorial-styled) | P2 | UX |
| **I340** | Glean integration — enterprise knowledge enrichment | P2 | Integrations |
| **I347** | SWOT report type — account analysis from existing intelligence | P2 | Intelligence / Reports |
| **I348** | Email digest push — DailyOS intelligence summaries via scheduled email | P2 | Distribution |
| **I350** | In-app notifications — release announcements, what's new, system status | P1 | UX / Infra |
| ~~I357~~ | ~~Semantic email reclassification~~ — absorbed by I367 (mandatory enrichment) | — | — |
| **I359** | Vocabulary-driven prompts — inject all 7 role fields into enrichment + briefing prompts | P2 | Intelligence |
| **I360** | Community preset import UI — frontend caller for existing import backend | P2 | UX |
| **I361** | Timeline meeting filtering — skip personal/focus/blocked types to match daily briefing | P1 | Backend |
| **I386** | Calendar lifecycle gaps — future meeting cancellation detection, rescheduling sync, continuous future polling | P1 | Backend / Calendar |
| **I362** | Shared meeting card — extract core rendering from BriefingMeetingCard for cross-surface reuse | P1 | UX / Component |
| **I363** | Timeline data enrichment — display time + duration on TimelineMeeting | P1 | Backend / Types |
| **I364** | Weekly forecast timeline adoption — replace MeetingRow with shared meeting card | P1 | UX |
| **I365** | Inbox-anchored email fetch — `in:inbox` replaces `is:unread newer_than:1d` | P0 | Backend / Gmail API |
| **I366** | Inbox reconciliation — vanished emails removed from DailyOS on each poll | P0 | Backend / Pipeline |
| **I367** | Mandatory email enrichment — every email AI-processed, retry on failure | P0 | Backend / Pipeline |
| **I368** | Persist email metadata to SQLite — DB as source of truth, not JSON files | P1 | Backend / DB |
| **I369** | Contextual email synthesis — entity-aware smart summaries (ADR-0085) | P1 | Backend / Intelligence |
| **I370** | Thread position refresh — detect user replies between polls | P1 | Backend / Gmail API |
| **I371** | Meeting email context rendering — wire recentEmailSignals to meeting detail UI | P1 | Frontend / UX |
| **I372** | Email-entity signal compounding — email signals flow into entity intelligence | P1 | Backend / Signals |
| **I373** | Email sync status indicator — show last fetch time, stage, error state | P2 | Frontend / UX |
| **I374** | Email dismissal learning — dismissed senders/types adjust future classification | P2 | Backend / Intelligence |
| **I375** | Refresh button audit — design continuity with add buttons + action/surface alignment | P2 | UX / Code Quality |
| **I376** | AI enrichment site audit — map every PTY/AI call site, verify ADR-0086 compliance | P1 | Code Quality / Architecture |
| **I377** | Signal system completeness — emitter/propagation/consumer map, remove dead rules | P1 | Code Quality / Signals |
| **I378** | Intelligence schema alignment — intelligence.json ↔ entity_intel ↔ frontend types | P1 | Code Quality / Architecture |
| **I379** | Vector DB audit — map embedding writes vs. queries, disable orphaned paths | P2 | Code Quality / Architecture |
| **I380** | commands.rs service extraction Phase 1 — complete services/ per SERVICE-CONTRACTS.md | P1 | Code Quality / Refactor |
| **I381** | db/mod.rs domain migration — move queries into domain modules per SERVICE-CONTRACTS.md | P2 | Code Quality / Refactor |
| **I382** | Partner entity type — `partner` account type, badge, partner-appropriate prompt shape | P1 | Backend / Entity |
| **I383** | AccountsPage three-group layout — Your Book / Your Team / Your Partners | P1 | Frontend / UX |
| **I384** | Parent account portfolio intelligence — two-layer intelligence.json (portfolio synthesis + own signals) | P1 | Backend / Intelligence |
| **I385** | Bidirectional entity hierarchy signal propagation — upward accumulation, downward fan-out | P1 | Backend / Signals |
| **I386** | Parent account detail page — portfolio surface (hotspots, cross-BU patterns, portfolio narrative) | P1 | Frontend / UX |
| **I387** | Multi-entity signal extraction from parent-level meetings — content-level entity resolution in transcript processor | P3 | Backend / Pipeline |
| **I388** | Project hierarchy intelligence — two-layer intelligence.json + bidirectional propagation for project entities (ADR-0087 applied to projects, v0.13.4 candidate) | P1 | Backend / Intelligence |
| **I389** | Entity-mode-aware surface ordering — nav and primary surface emphasis adapts to preset's entityModeDefault (project-mode users see Projects first) | P2 | Frontend / UX |
| **I390** | Person relationship graph — `person_relationships` table, typed edges (champion/blocker/sponsor/peer etc.), confidence scores, context scoping | P1 | Backend / Entity |
| **I391** | People network intelligence — two-layer person intelligence.json (own signals + network section), person→person signal propagation, edge detection from transcripts/emails | P1 | Backend / Intelligence |
| **I392** | Relationship cluster view on person detail — Network chapter, typed relationship list, cluster summary, network risks/opportunities, vocabulary-shaped per preset | P1 | Frontend / UX |

---

## Version Planning

### 0.8.2 through 0.9.2 — CLOSED

All issues resolved. See CHANGELOG.

---

### 0.10.0 — Signal Intelligence — CLOSED

All issues (I305, I306, I307, I308) closed in v0.10.0. Bayesian signal fusion, Thompson Sampling correction learning, cross-entity propagation, event-driven processing. See CHANGELOG.

---

### 0.10.1 — User Feedback & Onboarding — CLOSED

All issues (I344, I345, I346) closed in v0.10.1. Gmail teammate suggestions, Linear data layer, onboarding back-nav fix. See CHANGELOG.

---

### 0.11.0 — Role Presets & Entity Architecture — CLOSED

All issues (I309, I310, I311, I312, I313, I314, I315, I316, I143a, I143b, I352) closed in v0.11.0. 9 embedded role presets, preset-driven vitals, metadata storage migration, n-level nesting, shared entity hooks. Remaining work on actions standardization (I351) and prompt field injection (I359) carried forward. See CHANGELOG.

---

### 0.12.0 — Email Intelligence — CLOSED

All issues (I317, I318, I319, I320, I321, I322, I323, I324, I337, I338, I353, I354) closed in v0.12.0. Meeting-aware email digest, thread position tracking, entity cadence monitoring, hybrid classification, commitment extraction, briefing narrative, auto-archive, 1:1 relationship intelligence, self-healing hygiene. See CHANGELOG.

---

### 0.12.1 — Product Language & UX Polish — CLOSED

All issues (I341 partial, I342 Phases 1–3, I343 partial, I349, I355, I356 partial, I358 partial) closed in v0.12.1. Vocabulary pass, surface cuts, settings redesign, email intelligence UI. Remaining work on vocabulary (I341), inline editing (I343), surface restructure Phase 4 (I342), and email surface (I356, I358) carried into v0.13.0. See CHANGELOG.

---

### 0.13.0 — Event-Driven Meeting Intelligence + Unified Surfaces [IN PROGRESS]

**Theme:** Every meeting gets intelligence before you need it. One meeting entity, one visual identity everywhere. Signal-triggered refresh, surface restructure, shared meeting card.

| ID | Title | Status |
|----|-------|--------|
| I326 | Per-meeting intelligence lifecycle — detect, enrich, update, archive | Done |
| I327 | Advance intelligence generation — weekly pipeline + polling cadence | Done |
| I328 | Classification expansion — all meeting types get intelligence | Done |
| I329 | Intelligence quality indicators — replace hasPrep badge, correct vocabulary | Done |
| I330 | Week page ±7-day meeting intelligence timeline | Done |
| I331 | Daily briefing intelligence assembly — always-live, no empty state | Done |
| I332 | Signal-triggered refresh — calendar polling trigger + prep invalidation | Done |
| I333 | Meeting intelligence collaboration — share, request input, draft agenda | Done |
| I341 | Product vocabulary — system-term strings in user-facing copy | Done |
| I342 | Surface restructure Phase 4 | Done |
| I343 | Inline editing — AccountFieldsDrawer + ProjectFieldsDrawer → EditableText/List | Done |
| I351 | Actions chapter on PersonDetailEditorial | Done |
| I356 | Thread position UI — Replies Needed in daily briefing Attention section | Done |
| I358 | Email page — first-class nav surface | Done |
| I361 | Timeline meeting filtering — skip personal/focus/blocked types | Done |
| I362 | Shared meeting card — extract core rendering from BriefingMeetingCard | Done |
| I363 | Timeline data enrichment — display time + duration on TimelineMeeting | Done |
| I364 | Weekly forecast timeline adoption — replace MeetingRow with shared card | Done |

---

### 0.13.1 — Email as Intelligence Input [PLANNED]

**Theme:** Email is an intelligence input, not a display surface. Every email AI-processed, entity-resolved, contextually synthesized. DailyOS doesn't show you emails — it tells you what your emails mean. (ADR-0085)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I365 | Inbox-anchored email fetch — `in:inbox` replaces `is:unread newer_than:1d` | P0 | Backend / Gmail API |
| I366 | Inbox reconciliation — vanished emails removed from DailyOS on each poll | P0 | Backend / Pipeline |
| I367 | Mandatory email enrichment — every email AI-processed, retry on failure | P0 | Backend / Pipeline |
| I368 | Persist email metadata to SQLite — DB as source of truth, not JSON files | P1 | Backend / DB |
| I369 | Contextual email synthesis — entity-aware smart summaries | P1 | Backend / Intelligence |
| I370 | Thread position refresh — detect user replies between polls | P1 | Backend / Gmail API |
| I371 | Meeting email context rendering — wire recentEmailSignals to meeting detail UI | P1 | Frontend / UX |
| I372 | Email-entity signal compounding — email signals flow into entity intelligence | P1 | Backend / Signals |
| I373 | Email sync status indicator — show last fetch time, stage, error state | P2 | Frontend / UX |
| I374 | Email dismissal learning — dismissed senders/types adjust future classification | P2 | Backend / Intelligence |
| I375 | Refresh button audit — design continuity with add buttons + action/surface alignment | P2 | UX / Code Quality |
| I386 | Calendar lifecycle gaps — future meeting cancellation detection, rescheduling sync, continuous future polling | P1 | Backend / Calendar |

Note: I357 (semantic email reclassification) is absorbed by I367 — enrichment is mandatory, not opt-in.

---

### 0.13.2 — Structural Clarity [PLANNED]

**Theme:** Know what you built before you build the next layer. ADR-0086 defines the intended architecture. This version audits whether reality matches it, documents the full E2E intelligence chain, and does the structural refactoring that makes the system maintainable heading into v0.14.0.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I376 | AI enrichment site audit — map every PTY/AI call site, verify ADR-0086 compliance | P1 | Code Quality / Architecture |
| I377 | Signal system completeness — emitter/propagation/consumer map, remove dead rules | P1 | Code Quality / Signals |
| I378 | Intelligence schema alignment — intelligence.json ↔ entity_intel ↔ frontend types | P1 | Code Quality / Architecture |
| I379 | Vector DB audit — map embedding writes vs. queries, disable orphaned paths | P2 | Code Quality / Architecture |
| I380 | commands.rs service extraction Phase 1 — complete services/ per SERVICE-CONTRACTS.md | P1 | Code Quality / Refactor |
| I381 | db/mod.rs domain migration — move queries into domain modules per SERVICE-CONTRACTS.md | P2 | Code Quality / Refactor |

---

### 0.13.3 — Entity Hierarchy Intelligence [PLANNED]

**Theme:** Parent accounts are portfolio surfaces, not folders. Partners are a distinct entity type. Signals flow up from BUs and down from the parent. The AccountsPage reflects how users actually think about their work. (ADR-0087)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I382 | Partner entity type — `partner` account type, badge, partner-appropriate prompt shape | P1 | Backend / Entity |
| I383 | AccountsPage three-group layout — Your Book / Your Team / Your Partners | P1 | Frontend / UX |
| I384 | Parent account portfolio intelligence — two-layer intelligence.json | P1 | Backend / Intelligence |
| I385 | Bidirectional entity hierarchy signal propagation — upward accumulation, downward fan-out | P1 | Backend / Signals |
| I386 | Parent account detail page — portfolio surface (hotspots, cross-BU patterns, portfolio narrative) | P1 | Frontend / UX |

Note: I387 (multi-entity signal extraction from parent-level meetings) is deferred — P3, not version-locked. User behavior is to tag meetings at the parent level; bidirectional propagation (I385) covers the majority of the use case.

---

### 0.13.4 — Project Hierarchy Intelligence [PLANNED]

**Theme:** ADR-0087 applied to project entities. Parent projects become portfolio surfaces for Marketing, Product, and Agency users — same two-layer intelligence model, same bidirectional propagation, project-appropriate vocabulary. The ADR-0079 `entityModeDefault: "project"` preset users get the same portfolio capability account-mode users get in v0.13.3.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I388 | Project hierarchy intelligence — two-layer intelligence.json + bidirectional propagation for project entities | P1 | Backend / Intelligence |
| I389 | Entity-mode-aware surface ordering — nav/primary surface adapts to preset's entityModeDefault | P2 | Frontend / UX |

---

### 0.13.5 — People Relationship Network Intelligence [PLANNED]

**Theme:** People are not rows — they're nodes in a relationship network. A buying committee, a product team, a marketing cluster — these are graphs of individuals with influence flows, not isolated contacts. This version makes those relationships visible, persistent, and intelligent. (ADR-0088)

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I390 | Person relationship graph — `person_relationships` table, typed edges, confidence scoring, context scoping | P1 | Backend / Entity |
| I391 | People network intelligence — two-layer intelligence.json + network section + person→person signal propagation | P1 | Backend / Intelligence |
| I392 | Relationship cluster view on person detail — Network chapter, cluster summary, risks/opportunities, preset vocabulary | P1 | Frontend / UX |

---

### 0.14.0 — Reports + Distribution [PLANNED]

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I258 | Report Mode — export account detail as leadership-ready deck/PDF | P2 | UX |
| I302 | Shareable PDF export for intelligence reports (editorial-styled) | P2 | UX |
| I347 | SWOT report type — account analysis from existing intelligence | P2 | Intelligence |
| I348 | Email digest push — DailyOS intelligence summaries via scheduled email | P2 | Distribution |
| I350 | In-app notifications — release announcements, what's new, system status | P1 | UX / Infra |

---

### 1.0.0 — Beta Gate

Requires I56 (onboarding redesign), I57 (user setup), I280 (beta hardening), and a 30-day stability window with no open P0 issues.

---

## Integrations Queue

Not version-locked. Pulled in when capacity allows.

| ID | Title | Priority | Dependency |
|----|-------|----------|------------|
| I225 | Gong integration — sales call intelligence + transcripts | P1 | Gong API access |
| I227 | Gainsight integration — CS platform data sync | P2 | Gainsight API |
| I230 | Claude Cowork integration — project/task sync | P2 | API TBD |
| I340 | Glean integration — enterprise knowledge enrichment | P2 | Glean account access |
| I277 | Marketplace repo — preset community discoverability | P3 | — |

---

## Parking Lot

Acknowledged, not scheduled.

| ID | Title | Priority | Area |
|----|-------|----------|------|
| I88 | Monthly Book Intelligence — portfolio report | P2 | Intelligence |
| I90 | Product telemetry + analytics infrastructure | P2 | Infra |
| I115 | Multi-line action extraction | P2 | Data |
| I141 | AI content tagging during enrichment | P2 | Data |
| I142 | Account Plan artifact | P2 | Entity |
| I198 | Account merge + transcript reassignment | P2 | Entity |
| I199 | Archived account recovery UX — restore + relink | P2 | Entity |
| I359 | Vocabulary-driven prompts — inject all 7 role fields into enrichment + briefing prompts | P2 | Intelligence |
| I360 | Community preset import UI — frontend caller for existing import backend | P2 | UX |
