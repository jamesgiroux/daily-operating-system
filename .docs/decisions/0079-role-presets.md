# ADR-0079: Role Presets Replace Kits and Configurable Metadata

**Date:** 2026-02-16
**Status:** Accepted
**Supersedes:** [ADR-0046](0046-entity-mode-architecture.md) sections on Kits and Intelligence (sections 4-5, "Kits and Intelligence" and "Enrichment prompt fragments"). Entity modes and integrations from ADR-0046 are unchanged.
**Supersedes:** [ADR-0051](0051-user-configurable-metadata-settings.md) (user-configurable metadata settings — entire ADR)
**Replaces:** I92 (user-configurable metadata fields) scope definition
**Target:** v0.10.0 (Renewal)

## Context

ADR-0046 established entity modes (account/project/both) and proposed a Kit + Intelligence overlay system where domain-specific modules contribute fields, templates, vocabulary, and enrichment prompt fragments. Kits were entity-mode-specific (CS Kit for account mode, PM Kit for project mode). Intelligence was entity-mode-agnostic (Executive Intelligence, ProDev Intelligence). ADR-0051 deferred configurable metadata to post-ship, recommending a hybrid approach of universal core fields + Kit defaults + user overrides.

Since those decisions were written, the app has matured significantly:

1. **Entity intelligence is built and shipping.** The IntelligenceJson schema (executive_assessment, risks, wins, current_state, stakeholder_insights) is entity-generic. It works for accounts, projects, and people without modification. The analytical frame — what's working, what's not, what we don't know — is universal.

2. **The editorial layout is role-agnostic.** The 7-chapter entity detail (Headline, State of Play, The Room, Watch List, The Record, The Work, Appendix) works for any entity type. "State of Play" assesses account health for CS, deal momentum for Sales, client satisfaction for Agency, feature confidence for PM. The chapter is the same; the vocabulary differs.

3. **Meeting prep transfers across roles.** Stakeholder context, historical notes, open items, questions to ask, risk/win signals — every external meeting benefits from this regardless of whether the user is a CSM, AE, or Agency Director.

4. **The daily briefing is universal.** Hero, Focus, The Meeting, Schedule, Priorities — none of these are CS-specific. What changes is how the AI frames urgency: renewal proximity (CS) vs close date (Sales) vs deadline (Agency/PM).

5. **Kits were never built.** No Kit code exists. The Kit concept requires module infrastructure (registration, activation, field injection, template management) that is over-engineered for what turns out to be a configuration problem.

6. **The real differentiator between roles is vocabulary, not architecture.** "Churn risk" vs "deal stalled" vs "scope creep" vs "blocked by dependency" — same Watch List component, different words. The AI prompts are the product surface that matters most per role, not the UI layout or data model.

## Decision

**Replace Kits and Intelligence overlays with Role Presets** — lightweight JSON configurations that adjust metadata fields, AI vocabulary, prioritization signals, lifecycle events, and entity mode defaults.

### What a Role Preset Is

A role preset is a JSON document that configures how DailyOS presents and interprets work for a specific role. It changes:

1. **Metadata fields** — Which fields appear on entity detail pages and vitals strips
2. **AI vocabulary** — Prompt fragments that shape how intelligence enrichment frames analysis
3. **Prioritization signals** — What urgency means and how the briefing ranks attention
4. **Lifecycle event types** — Which events are available for recording (renewal, stage change, launch, etc.)
5. **Entity mode default** — Account, project, or both (user can override)
6. **Briefing emphasis** — What the daily briefing prioritizes for this role

A role preset does **not** change:
- The 7-chapter entity detail layout
- The daily briefing structure (Hero, Focus, The Meeting, Schedule, Priorities, Finis)
- Meeting prep sections
- People/stakeholder management
- Action system
- Archive structure
- Semantic search
- IntelligenceJson schema

### Shipped Presets (9)

| Preset | Entity Mode | Who It's For |
|--------|------------|-------------|
| **Customer Success** | Account | CSMs, TAMs, Account Managers in SaaS |
| **Sales** | Account | AEs, SDRs, Sales Engineers, Revenue leaders |
| **Marketing** | Project | Marketing managers, demand gen, product marketing, brand leads |
| **Partnerships** | Both | Partner Managers, BD leads, Channel/Alliance Managers |
| **Agency** | Both | Account Directors, Client Services leads, Studio Managers |
| **Consulting** | Both | Management/strategy consultants, advisory firms |
| **Product** | Project | PMs, Technical PMs, Product Owners |
| **Leadership** | Both | VPs, Directors, Chiefs of Staff, Department heads, Founders |
| **The Desk** | Both | Researchers, freelancers, academics, non-profit managers, anyone who prefers a blank canvas |

"Marketing" is project-mode by default. Campaigns, launches, and initiatives are the primary entities. Internal teams (vertical, product, agency, contractors) are managed via internal accounts with n-level nesting (I316). Vocabulary frames urgency around launch dates and campaign deadlines rather than renewals or deal stages.

"The Desk" is not a lesser preset. Named for the DailyOS brand metaphor — your desk, arranged the way you work. It ships with minimal neutral metadata and clean vocabulary, serving as both a dignified catch-all and the natural base for community-created presets.

### Community Presets

Role presets are JSON files with a documented schema. The format is:
- **Human-readable** — editable in any text editor
- **AI-generatable** — "Create a DailyOS role preset for a recruiter" is a one-shot prompt
- **Shareable** — drop in a GitHub repo, share a link
- **Importable** — Settings page supports import from file

Community examples that don't need to ship with the app: Venture Capital (portfolio companies, board prep), Recruiting (candidates, requisitions), Non-Profit (grants, programs), Real Estate (properties, clients), Academic Research (papers, grants, collaborators), Journalism (sources, stories), Developer Relations (community health, conference pipeline).

The long tail of roles is infinite. We ship 9 good ones; the community builds the rest. This is the open-source contribution surface that doesn't require writing Rust.

### Preset Schema

```jsonc
{
  "id": "customer-success",
  "name": "Customer Success",
  "description": "For CSMs, TAMs, and Account Managers managing customer portfolios",
  "entityModeDefault": "account",
  "metadata": {
    "account": [
      { "key": "arr", "label": "ARR", "type": "currency" },
      { "key": "health", "label": "Health", "type": "select", "options": ["Green", "Yellow", "Red"] },
      { "key": "lifecycle", "label": "Lifecycle", "type": "text" },
      { "key": "renewal_date", "label": "Renewal Date", "type": "date" },
      { "key": "nps", "label": "NPS", "type": "number" },
      { "key": "contract_start", "label": "Contract Start", "type": "date" },
      { "key": "contract_end", "label": "Contract End", "type": "date" },
      { "key": "support_tier", "label": "Support Tier", "type": "text" }
    ],
    "project": []
  },
  "vocabulary": {
    "entityNoun": "account",
    "healthFrame": "health",
    "riskVocabulary": ["churn risk", "executive sponsor gap", "adoption decline", "renewal at risk", "competitor displacement"],
    "winVocabulary": ["value delivered", "expansion signal", "NPS improvement", "executive alignment", "adoption milestone"],
    "urgencySignals": ["renewal approaching", "health declined", "contact gone cold", "overdue commitment"]
  },
  "vitals": ["arr", "health", "lifecycle", "renewal_date", "nps", "meeting_frequency"],
  "lifecycleEvents": ["Renewal", "Expansion", "Contraction", "Churn", "Escalation", "Executive Review"],
  "prioritization": {
    "primary": "renewal_proximity",
    "secondary": ["health_decline", "meeting_gap", "overdue_actions"]
  },
  "briefingEmphasis": "Accounts needing attention, upcoming renewals, relationship gaps, prep for customer calls"
}
```

### Why Not Kits

Kits (ADR-0046) were designed as installable modules with code-level integration: field injection, template contribution, vocabulary fragments, enrichment prompt composition. This is plugin architecture.

Role Presets are configuration, not code. The difference matters:

| Concern | Kits (ADR-0046) | Role Presets |
|---------|----------------|-------------|
| Implementation | Module registration, activation hooks, field injection API | JSON file loaded at startup |
| Composition | Multiple Kits + multiple Intelligence modules compose | One active preset at a time (simpler) |
| Extensibility | Requires code contribution (Rust) | Requires JSON file (any text editor, any AI) |
| Community | High barrier (write Rust, understand module API) | Low barrier (edit JSON, share a file) |
| Maintenance | Module API is a support surface | JSON schema is a documentation page |
| Intelligence overlays | Separate concept (Executive, ProDev) composing with Kits | Folded into vocabulary + prioritization per preset |

The Kit model assumed we'd need composable domain modules (CS Kit + Executive Intelligence + ProDev Intelligence). In practice, Intelligence is baked into the core enrichment and doesn't need per-role composition — the IntelligenceJson schema already captures executive assessment, risks, wins, and stakeholder insights for every entity. What changes per role is vocabulary, not analytical capability.

### Why Not a Custom Field Builder

ADR-0051 Option B (fully user-defined custom fields) is a schema-builder disguised as a settings page. It violates P4 (Opinionated Defaults) by forcing the user to design their own data model. It violates P7 (Consumption Over Production) by making configuration a production activity. It violates P3 (Buttons, Not Commands) by requiring expertise in data modeling.

Role Presets honor P4: pick your role, get the right fields. Users who need something different import a community preset or edit The Desk's JSON. The escape hatch exists without the schema-builder.

### Metadata Storage

Moving from hardcoded columns (`arr`, `nps`, `health` in the `accounts` table) to a flexible store that supports any preset's field definitions.

**Approach:** JSON metadata column on entity tables.

```sql
ALTER TABLE accounts ADD COLUMN metadata TEXT DEFAULT '{}';
ALTER TABLE projects ADD COLUMN metadata TEXT DEFAULT '{}';
```

Existing hardcoded columns remain for backwards compatibility and query efficiency. The `metadata` JSON column stores preset-specific fields. The vitals strip and detail page read from both: hardcoded columns (if present) + metadata JSON (for everything else).

Migration path: existing accounts with ARR/health/NPS continue working. The CS preset maps its field keys to the existing columns. Other presets use the metadata JSON column.

### Onboarding Impact

Role selection replaces the current entity mode selection step. A role selection grid (8 presets, each with name + one-line description) subsumes both the entity mode selector and any future "Kit selection" step. Selecting a role implies an entity mode default, but the user can override it. One choice, not three.

### Config Evolution

```json
{
  "role": "customer-success",
  "entityMode": "account",
  "customPresetPath": null
}
```

`role` replaces `profile`. The `profile` field is retained for backwards compatibility and derived from role + entity mode for any backend code that still references it.

`customPresetPath` points to a user-provided JSON preset file (for community presets or user customization). When set, it overrides the shipped preset.

## Consequences

**Simpler:**
- No module registration, activation, or composition infrastructure
- No Kit API to maintain or document
- No Intelligence overlay composition rules
- Community contribution barrier drops from "write Rust" to "edit JSON"
- One role selection replaces three onboarding choices (entity mode + Kit + Intelligence)
- Prompt engineering per role is contained in a JSON file, not scattered across module code

**Trade-offs:**
- One active preset at a time (no composition). A user can't combine "Sales vocabulary" with "Executive Intelligence" as separate layers. This is intentional — composition adds complexity without proportional value. A Leadership preset can include sales-adjacent vocabulary if needed.
- Hardcoded columns remain in the DB alongside flexible metadata. This is pragmatic — CS is the primary user, and ARR/health/NPS queries benefit from indexed columns. Other presets use the JSON column.
- The preset schema is a contract. Changing it affects all community presets. Version the schema from day one.

**Risks:**
- Prompt quality per role. Each preset's vocabulary shapes intelligence enrichment. Generic presets produce generic intelligence. Investment in per-role prompt engineering is essential.
- Schema migration. Moving from hardcoded fields to preset-driven display requires careful migration for existing users.
- Community quality. Community presets may be poorly designed. Consider a curation/review process for any preset gallery.
