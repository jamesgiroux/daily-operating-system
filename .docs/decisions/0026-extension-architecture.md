# ADR-0026: Extension architecture with profile-activated modules

**Date:** 2026-02
**Status:** Accepted

## Context

Domain-specific features (Customer Success workflows, Professional Development tracking, CRM integration) shouldn't be forced on all users. But they share infrastructure (dashboard, actions, briefing).

## Decision

Extensions are internal module boundaries that activate together based on profile or explicit opt-in. Not a plugin marketplace — that's Phase 5+.

**Core vs. Extension:**

| Layer | Core (always active) | Extension (profile-activated) |
|-------|---------------------|-------------------------------|
| Workflows | Briefing, Archive, Inbox | — |
| Data | `_today/`, `_inbox/`, `_archive/`, SQLite | Account dashboards, success plans, registry |
| UI | Dashboard, Actions, Inbox, Settings | Accounts page, portfolio triage |

**What an extension provides:**
1. Post-enrichment hooks — mechanical updates after AI enrichment
2. Data schemas — JSON schemas for structured documents
3. UI contributions — sidebar items, dashboard sections, page routes
4. Workflow hooks — steps in existing workflows
5. Templates — domain-specific document templates

**Profile → Extension mapping:**
- CSM → Core + Customer Success (default), optional: ProDev, CRM
- General → Core, optional: ProDev, CRM

## Consequences

- Clean internal boundaries make future SDK formalization easier
- Extensions can't break core — they hook into defined extension points
- Phase 4 for extensions, Phase 5+ for public SDK/community plugins
- Designing clean boundaries now prevents retrofit pain later
