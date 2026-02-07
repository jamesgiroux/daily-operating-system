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
| [0015](0015-defer-inbox-to-phase-2.md) | Defer inbox processing to Phase 2 | Accepted |
| [0016](0016-defer-post-meeting-to-phase-3.md) | Defer post-meeting capture to Phase 3 | Accepted |
| [0017](0017-pure-rust-archive.md) | Pure Rust archive, no three-phase | Accepted |
| [0018](0018-hybrid-storage-markdown-sqlite.md) | Hybrid storage: Markdown + SQLite | Accepted |
| [0019](0019-reference-approach-for-directives.md) | Reference approach for directives | Accepted |
| [0020](0020-profile-dependent-accounts.md) | Profile system — role-based configuration | Accepted |
| [0021](0021-multi-signal-meeting-classification.md) | Multi-signal meeting classification | Accepted |
| [0022](0022-proactive-research-unknown-meetings.md) | Proactive research for unknown meetings | Accepted |
| [0023](0023-post-meeting-capture-replaces-wrap.md) | /wrap replaced by post-meeting capture | Accepted |
| [0024](0024-email-ai-triage-not-client.md) | Email = AI triage, not email client | Accepted |
| [0025](0025-app-native-governance.md) | App-native governance, not ported CLI tools | Accepted |
| [0026](0026-extension-architecture.md) | Extension architecture with profile-activated modules | Accepted |
| [0027](0027-mcp-dual-mode.md) | MCP integration: dual-mode server + client | Accepted |
| [0028](0028-structured-document-schemas.md) | Structured document schemas (JSON-first templates) | Accepted |
| [0029](0029-three-tier-email-priority.md) | Three-tier email priority with AI-enriched context | Accepted |
| [0030](0030-weekly-prep-with-daily-refresh.md) | Weekly prep generation with daily refresh | Proposed |
| [0031](0031-actions-source-of-truth.md) | Actions: SQLite as working store, markdown as archive | Proposed |
| [0032](0032-calendar-source-of-truth.md) | Calendar source of truth: hybrid overlay | Proposed |
| [0033](0033-meeting-entity-unification.md) | Meeting entity unification | Proposed |
| [0034](0034-adaptive-dashboard.md) | Adaptive dashboard: density-aware layout | Proposed |

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
