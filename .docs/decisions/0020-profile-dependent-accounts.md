# ADR-0020: Profile system — role-based configuration

**Date:** 2026-02
**Status:** Accepted

## Context

Different professional roles have fundamentally different workflows. A CSM tracks accounts, renewals, and customer health. A PM tracks projects, sprints, and roadmaps. A generalist tracks tasks and calendar. Making every feature available to every user adds noise — accounts are meaningless to a PM, sprint tracking is meaningless to a CSM.

## Decision

The app uses a profile system that shapes the entire experience based on the user's role. A profile configures:

1. **PARA structure** — Which workspace folders exist and what they contain (CSM gets `Accounts/` with per-account scaffolding; General gets `Projects/`)
2. **Meeting classification** — How calendar events are categorized (CSM: attendee domain → account mapping, customer/QBR/training types; General: no account mapping, external/internal only)
3. **Prep templates** — What context is gathered per meeting type (CSM: account metrics, stakeholder map, risks, wins; General: attendee info, last meeting notes)
4. **Data sources** — What files/trackers the system reads from (CSM: account tracker, stakeholder maps, dashboards; General: project folders)
5. **Default extensions** — Which extension modules activate (CSM: Customer Success extension per ADR-0026; General: core only)
6. **Navigation** — Sidebar entity page (CSM: Accounts; General: Projects per ADR-0008), card metadata, grouping

**Implemented profiles:** Customer Success, General. Future profiles (Sales, Engineering) follow the same pattern — define entity type, classification rules, data sources, and default extensions.

**Switching is non-destructive** (ADR-0009). Files persist across switches. Only classification, navigation, and card rendering change.

## Consequences

- Each profile feels purpose-built, not stripped-down — both CSM and General are intentional experiences
- Adding a new profile means: define its entity, classification rules, data sources, prep templates, and default extensions
- Profile is stored in `~/.dailyos/config.json` and selected during onboarding
- The extension architecture (ADR-0026) handles domain-specific features that go beyond configuration (workflows, post-enrichment hooks, dashboard sections)
