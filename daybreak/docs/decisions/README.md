# Architecture Decision Records

Architectural decisions for DailyOS, recorded as lightweight ADRs.

**Convention:** One file per decision. Decisions are immutable once accepted — if you change your mind, create a new ADR that supersedes the old one. Never edit a decision after accepting it.

**When to write an ADR:**
- Choosing between technologies or approaches
- Deciding how data flows through the system
- Establishing a pattern that future code will follow
- Any decision you'd want to explain to your future self

**When NOT to write an ADR:**
- Bug fixes (just fix it)
- UI tweaks that don't affect architecture
- Refactoring that preserves behavior

---

## Index

| ADR | Decision | Status |
|-----|----------|--------|
| [0001](0001-use-tauri-over-electron.md) | Use Tauri over Electron | Accepted |
| [0002](0002-frontend-first-implementation.md) | Frontend-first implementation approach | Accepted |
| [0003](0003-config-json-no-ui-for-mvp.md) | Config in JSON file, no UI for MVP | Accepted |
| [0004](0004-hybrid-json-markdown-architecture.md) | Hybrid JSON + Markdown architecture | Accepted |
| [0005](0005-archives-remain-markdown-only.md) | Archives remain markdown-only | Accepted |
| [0006](0006-determinism-boundary.md) | Phase 3 generates JSON, not Claude | Accepted |
| [0007](0007-dashboard-is-the-product.md) | Dashboard is the product, not a page among pages | Accepted |
| [0008](0008-profile-aware-navigation.md) | Profile-aware navigation with entity pattern | Accepted |
| [0009](0009-non-destructive-profile-switching.md) | Profile switching is non-destructive | Accepted |
| [0010](0010-simplified-sidebar.md) | Focus, Week, Emails removed from sidebar | Accepted |
| [0011](0011-sidebar-groups.md) | Sidebar groups: Today + Workspace | Accepted |
| [0012](0012-profile-indicator-in-sidebar.md) | Profile indicator in sidebar header | Accepted |
| [0013](0013-meeting-detail-drill-down.md) | Meeting detail is a drill-down, not a nav item | Accepted |
| [0014](0014-mvp-scope.md) | MVP = F1 + F7 + F6 + F3 | Accepted |
| [0015](0015-defer-inbox-to-phase-2.md) | Defer inbox processing to Phase 2 | Superseded by [0036](0036-inbox-processing-in-phase-1.md) |
| [0016](0016-defer-post-meeting-to-phase-3.md) | Defer post-meeting capture to Phase 3 | Superseded by [0037](0037-post-meeting-capture-in-phase-1.md) |
| [0017](0017-pure-rust-archive.md) | Pure Rust archive, no three-phase | Accepted |
| [0018](0018-hybrid-storage-markdown-sqlite.md) | Hybrid storage: Markdown + SQLite | Superseded by [0048](0048-three-tier-data-model.md) |
| [0019](0019-reference-approach-for-directives.md) | Reference approach for directives | Accepted |
| [0020](0020-profile-dependent-accounts.md) | Profile system — role-based configuration | Superseded by [0038](0038-cs-first-development.md) |
| [0021](0021-multi-signal-meeting-classification.md) | Multi-signal meeting classification | Accepted |
| [0022](0022-proactive-research-unknown-meetings.md) | Proactive research for unknown meetings | Accepted |
| [0023](0023-post-meeting-capture-replaces-wrap.md) | /wrap replaced by post-meeting capture | Accepted |
| [0024](0024-email-ai-triage-not-client.md) | Email = AI triage, not email client | Accepted |
| [0025](0025-app-native-governance.md) | App-native governance, not ported CLI tools | Accepted |
| [0026](0026-extension-architecture.md) | Extension architecture with profile-activated modules | Superseded by [0046](0046-entity-mode-architecture.md) |
| [0027](0027-mcp-dual-mode.md) | MCP integration: dual-mode server + client | Accepted |
| [0028](0028-structured-document-schemas.md) | Structured document schemas (JSON-first templates) | Accepted |
| [0029](0029-three-tier-email-priority.md) | Three-tier email priority with AI-enriched context | Accepted |
| [0030](0030-weekly-prep-with-daily-refresh.md) | Composable workflow operations | Accepted |
| [0031](0031-actions-source-of-truth.md) | Actions: SQLite as working store, markdown as archive | Accepted |
| [0032](0032-calendar-source-of-truth.md) | Calendar source of truth: hybrid overlay | Accepted |
| [0033](0033-meeting-entity-unification.md) | Meeting entity unification | Accepted |
| [0034](0034-adaptive-dashboard.md) | Adaptive dashboard: density-aware layout | Deprecated (backlog I37) |
| [0035](0035-incremental-prep-generation.md) | Incremental prep generation for new meetings | Superseded by [0030](0030-weekly-prep-with-daily-refresh.md) |
| [0036](0036-inbox-processing-in-phase-1.md) | Inbox processing implemented in Phase 1 | Accepted |
| [0037](0037-post-meeting-capture-in-phase-1.md) | Post-meeting capture implemented in Phase 1 | Accepted |
| [0038](0038-cs-first-development.md) | CS-first development focus | Accepted |
| [0039](0039-feature-toggle-architecture.md) | Feature toggle architecture | Accepted |
| [0040](0040-archive-reconciliation.md) | Archive reconciliation (end-of-day mechanical cleanup) | Accepted |
| [0041](0041-two-sided-impact-model.md) | Two-sided impact model (CS outcomes vs personal impact) | Accepted |
| [0042](0042-per-operation-pipelines.md) | Per-operation pipelines with progressive delivery | Accepted |
| [0043](0043-meeting-intelligence-is-core.md) | Meeting intelligence is core, not extension | Accepted |
| [0044](0044-meeting-scoped-transcript-intake.md) | Meeting-scoped transcript intake from dashboard | Accepted |
| [0045](0045-entity-abstraction.md) | Profile-agnostic entity abstraction | Accepted |
| [0046](0046-entity-mode-architecture.md) | Entity-mode architecture with orthogonal integrations | Accepted |
| [0047](0047-entity-dashboard-architecture.md) | Entity dashboard architecture — two-file pattern with bidirectional sync | Accepted |
| [0048](0048-three-tier-data-model.md) | Three-tier data model — filesystem, SQLite, app memory | Accepted |
| [0049](0049-eliminate-python-runtime.md) | Eliminate Python runtime dependency | Accepted |
| [0050](0050-universal-file-extraction.md) | Universal file extraction for inbox pipeline | Accepted |
| [0051](0051-user-configurable-metadata-settings.md) | User-configurable metadata settings | Proposed |
| [0052](0052-week-page-redesign.md) | Week page redesign — consumption-first weekly briefing | Accepted |
| [0054](0054-list-page-design-pattern.md) | List page design pattern — signal-first flat rows | Accepted |
| [0053](0053-dashboard-ux-redesign.md) | Dashboard UX redesign — readiness-first overview | Accepted |
| [0055](0055-schedule-first-dashboard-layout.md) | Schedule-first dashboard layout — two-column with sidebar | Accepted |
| [0056](0056-parent-child-accounts.md) | Parent-child accounts — enterprise BU hierarchy | Accepted |
| [0057](0057-entity-intelligence-architecture.md) | Entity intelligence architecture — three-file pattern, auto-triggered enrichment, persistent entity prep | Accepted |
| [0058](0058-proactive-intelligence-maintenance.md) | Proactive intelligence maintenance — self-healing gap detection, calendar-driven refresh, overnight batch processing | Accepted |
| [0059](0059-entity-directory-template.md) | Core entity directory template — 3-folder scaffold, README convention, content-aware routing | Accepted |

---

## Template

```markdown
# ADR-NNNN: Title

**Date:** YYYY-MM-DD
**Status:** proposed | accepted | deprecated | superseded by [ADR-NNNN](link)

## Context

What is the situation? What forces are at play?

## Decision

What did we decide?

## Consequences

What becomes easier? What becomes harder? What are the trade-offs?
```
