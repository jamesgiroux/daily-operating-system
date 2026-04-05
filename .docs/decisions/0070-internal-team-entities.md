# ADR-0070: Internal team entities via internal organization account

**Date:** 2026-02-13
**Status:** Accepted
**Builds on:** [ADR-0046](0046-entity-mode-architecture.md) (entity-mode architecture), [ADR-0056](0056-hierarchical-accounts.md) (parent-child accounts)

## Context

DailyOS provides rich entity tracking for external relationships (customers, partners, prospects) via the account entity system. Account entities support:
- Intelligence enrichment (executive assessment, risks, recent wins)
- Content indexing (documents, transcripts, notes)
- Meeting association via attendee domain matching
- Parent-child hierarchy (accounts with BU subdirectories)

However, **internal teams have no equivalent entity structure**. This creates gaps for:

1. **Project-based users** who work across internal departments (Engineering, Marketing, Finance, Ops). Meetings involve internal stakeholders from multiple teams. There's no entity to associate these meetings with or track team dynamics.

2. **Account-based users** who also manage internal relationships. A CS leader needs to track their own CS team, collaborate with Sales, coordinate with Engineering. These internal meetings and stakeholders are unstructured.

3. **Internal meeting prep**. When meeting with your own team or another internal department, the system has no entity context to enrich prep with (team priorities, recent work, stakeholder dynamics).

We already have the infrastructure to solve this:
- User's email domains are configured in settings/onboarding
- Domain-based classification distinguishes internal vs. external meetings
- Account entities have all the capabilities internal teams need
- Parent-child hierarchy supports team subdivisions

But we lack a conceptual model: **What entity represents the user's internal organization and its teams?**

## Decision

**Internal teams are accounts.** Specifically:

### 1. Internal Organization Account

On workspace initialization or first-run setup, create a special **Internal Organization** account that represents the user's company:

- **Name**: Derived from user's company name (collected during onboarding) or inferred from primary email domain
- **Domains**: Associated with all user email domains from config (`userDomains` array)
- **Type indicator**: `is_internal: true` flag in accounts table (or `account_type: 'internal'` enum)
- **Directory**: `~/Documents/DailyOS/Accounts/{Company}` (same directory as external accounts — your company is an account, not a separate namespace)
- **Intelligence**: Same as external accounts (executive assessment, stakeholder insights, etc.)

This internal organization account is the **root entity for all internal work**.

### 2. Internal Teams as Child Entities

Individual teams are **child accounts** (BUs) under the internal organization, following the same parent-child pattern established in ADR-0056:

- **Engineering** → `Accounts/Acme/Engineering/`
- **Marketing** → `Accounts/Acme/Marketing/`
- **Sales** → `Accounts/Acme/Sales/`
- **CS** → `Accounts/Acme/CS/`

Teams inherit the internal organization's domains and internal flag. Teams can have their own:
- Intelligence (team priorities, recent work, dynamics)
- Content (team documents, 1:1 transcripts, planning docs)
- Meeting associations (team standups, 1:1s, cross-functional syncs)

### 3. Meeting Association

Internal meetings (where all attendees share user's email domain) automatically associate with:
- **Default**: Internal organization account (if no team specified)
- **Team-specific**: Child team entity (via manual assignment or AI inference)

Association logic:
1. Attendee domain matching identifies meeting as internal
2. If meeting title/attendees suggest a specific team → associate with that team entity
3. Otherwise → associate with root internal organization
4. User can manually correct via EntityPicker (same as external accounts)

### 4. People Sub-Entity Behavior

Internal people (colleagues) link to the internal organization or team entities, not external accounts:
- Engineering teammate → linked to Internal/Acme/Engineering
- Cross-functional stakeholder → linked to Internal/Acme (root org)

This provides stakeholder context in internal meeting prep (just like external stakeholders in customer meetings).

### 5. Intelligence Enrichment

Internal teams receive the same intelligence enrichment as external accounts:
- **Executive assessment**: Team priorities, blockers, momentum
- **Stakeholder insights**: Key people, communication patterns, relationship dynamics
- **Recent wins/risks**: Team accomplishments, emerging issues
- **Next meeting readiness**: Prep context for 1:1s, standups, cross-functional syncs

The enrichment prompt is entity-type-agnostic. Internal entities enrich from:
- Internal meeting transcripts
- Team documents in entity directory
- Actions/captures associated with internal meetings
- Communication patterns (1:1 frequency, topic clusters)

### 6. Unified Entity Model

This approach means:
- **Accounts work for both external and internal entities** — no new entity type
- **One entity model to maintain** — intelligence, content indexing, meeting association all reuse existing infrastructure
- **"Both" mode already handles it** — users in "both" entity mode see external accounts AND internal teams in the same unified view
- **Existing UI works** — AccountsPage, AccountDetailPage, EntityPicker all work for internal teams (with visual distinction)

### UI Distinction and Naming

**Data model**: Internal teams are `accounts` with `is_internal: true` flag.
**User-facing name**: The internal organization account is named after the user's actual company — "Automattic", "Anthropic", "Acme Corp" — not "Internal." The `is_internal` flag drives behavior (domain matching, meeting classification) but the entity name is the real company name. Child teams are named naturally: "Customer Success", "Platform Engineering", "VIP Division" — not "Internal/CS" or "Internal Teams/Engineering."

This means the user's company appears alongside external accounts in the entity list, distinguished by a subtle visual indicator — not segregated into an "Internal" ghetto. The mental model: your company is an entity you navigate just like any other, it just happens to be yours.

Visual distinction:
- **Badge/icon**: Small house icon or "Your org" badge on the company entity and its children — subtle, not dominant
- **Color/styling**: Subtle visual differentiation (sage accent vs. gold)
- **List filtering**: "Your org" / "External" / "All" tab filter on AccountsPage
- **Navigation**: Company entity expandable in the same list as external accounts

Core UI is identical — detail page, intelligence card, file browser, meeting history all work the same. The distinction is purely visual.

## Alternatives Considered

### Alternative 1: Separate "Team" entity type

Create a new entity type parallel to accounts/projects.

**Pros:**
- Explicit semantic distinction between external accounts and internal teams
- Can have team-specific schema fields without `account_type` flag

**Cons:**
- Duplicates all account infrastructure (intelligence, content indexing, meeting association, hierarchy)
- Increases maintenance burden (two entity types to keep in sync)
- Doesn't solve the "where do internal meetings go?" problem — still need association logic
- Breaks "Both" mode unified entity view (now three entity types)

**Rejected** because it adds complexity without material benefit. Internal teams need exactly what accounts already provide.

### Alternative 2: People entity with team grouping

Use the people entity and add a "team" grouping field. Internal meetings associate with people, not teams.

**Pros:**
- Lightweight — no new entity type or account variant
- People already exist as universal sub-entity

**Cons:**
- People are individuals, not organizational units — this inverts the model
- No entity directory for team documents/transcripts
- No team-level intelligence (only person-level)
- Meeting association becomes many-to-many with people (every meeting → all attendees) instead of one-to-one with team
- Doesn't match user mental model (meetings are "with Engineering" not "with Alice, Bob, Charlie")

**Rejected** because people are the wrong granularity. Teams are first-class organizational units that need entity-level tracking.

### Alternative 3: Keep teams informal/unstructured (status quo)

Don't create internal team entities. Let internal meetings remain unassociated or manually tag them.

**Pros:**
- No implementation work
- Simpler model (fewer entities)

**Cons:**
- Internal meetings have no entity context for prep enrichment
- Internal stakeholders have no team affiliation for relationship tracking
- Project-based users have no way to organize internal cross-functional work
- Violates principle of "prepared, not empty" — internal meetings get worse prep than external ones

**Rejected** because it's the current inadequate state. Internal teams deserve entity-quality tracking.

### Onboarding: Internal Team Setup Wizard

Onboarding includes a dedicated "Internal Team Setup" chapter that:

1. **Collects company information**:
   - Company name (for internal organization account name)
   - All email domains for the organization (not just one — users may have multiple brands/divisions/acquisitions)
   - Example: Anthropic might have `@anthropic.com`, `@claude.ai`, `@anthropic.ai`

2. **Establishes user context**:
   - User's role/title (e.g., "Customer Success Manager", "Engineering Manager")
   - User's immediate team (e.g., "Customer Success", "Platform Engineering")
   - This creates the user's team entity automatically

3. **Populates initial colleagues**:
   - Name + email for immediate teammates (creates People entities)
   - Optional: role/title for each person
   - These people are auto-linked to the user's team entity via `entity_people` junction

**Benefits of wizard-driven setup**:
- Avoids confusion about internal meetings from day one (they have an entity to associate with)
- Bootstraps People area with real colleagues (not empty state)
- Provides immediate stakeholder context for internal meeting prep
- User's team is enrichable from the start (1:1 transcripts, team documents)

**Migration for existing users**:
- First launch after upgrade shows "Internal Team Setup" wizard as a one-time onboarding step
- Pre-fills company name from existing config if available
- Imports user domains from config
- Suggests team names from calendar analysis (common attendee groups in internal meetings)

## Consequences

**Easier:**
- Reuses all existing account infrastructure — intelligence, content indexing, meeting association, parent-child hierarchy
- Unified entity model — one codebase for both external and internal entities
- "Both" mode handles it naturally — no special cases
- Migration path is clear — wizard creates internal org + user's team + initial colleagues
- Domain-based classification becomes bidirectional — external domains → accounts, internal domains → internal org
- Visual distinction is purely UI-level — same data model, different badges/filters
- **People area gets populated naturally** — colleague setup during onboarding seeds the People entity system

**Harder:**
- Onboarding must collect company name and create internal organization account
- UI needs subtle visual distinction (badge/icon) for internal vs. external — not a separate section
- Meeting association logic must handle both external and internal domain matching

**Trade-offs:**
- Chose entity reuse over semantic purity — internal teams are technically "accounts" in the data model, even though users think of them as teams. The benefit (unified infrastructure) outweighs the semantic oddness.
- Chose automatic internal org creation over opt-in — every user gets an internal organization by default (even if they don't immediately populate teams). This ensures internal meetings have an entity to associate with.
- Chose parent-child hierarchy over flat team list — teams as BUs under internal org. More structured, supports nested teams (Engineering → Platform → Infrastructure), but requires directory-based team creation initially (I205 will add UI).

**Enables:**
- **Project-based users** can track internal cross-functional work with entity-quality context
- **Account-based users** can enrich internal meetings (team 1:1s, department syncs) with stakeholder context
- **Internal meeting prep** gains entity intelligence (team priorities, recent wins, stakeholder dynamics)
- **Unified "Both" mode** shows external customers and internal teams side-by-side with appropriate context
- **Future**: Internal team health signals (standup frequency, blocker count, 1:1 coverage) parallel to account health

**Implementation path:**
- I204: Create internal organization + team management (onboarding, automatic creation, team setup UI)
- I205: BU/child entity creation UI (general capability for both external accounts and internal teams)
- Phase 1: Automatic internal org creation during onboarding
- Phase 2: Team setup wizard (suggest teams from calendar analysis)
- Phase 3: Team-level intelligence and attention signals
