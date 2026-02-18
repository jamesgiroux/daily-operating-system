# Rethinking Roles, Metadata & I92

**Date:** 2026-02-16
**Context:** I92 (user-configurable metadata fields) was written when the app was a CLI tool with a CS profile. The app is now a native Tauri app with editorial intelligence surfaces, entity modes (account/project/both), semantic search, and a mature daily briefing. Time to reassess.

---

## What the App Actually Delivers Today

The core value loop is:

1. **Daily Briefing** — Finite, editorial, read-it-and-you're-done. Focus recommendation, lead meeting prep, capacity-aware action prioritization.
2. **Meeting Prep** — Stakeholder context, historical notes, open items, questions to ask, entity-linked risks/wins. Generated per external meeting.
3. **Entity Intelligence** — AI-synthesized executive assessment, risks, wins, current state, stakeholder insights. Auto-enriched on content change.
4. **Action Tracking** — Entity-linked, meeting-sourced, priority-scored with "why now?" reasoning.
5. **People/Stakeholders** — Relationship warmth, engagement tracking, meeting history, cross-entity network.
6. **The Archive** — Structured markdown knowledge base. AI-consumable. Any tool can read it.

**The critical observation:** This loop is not CS-specific. It's relationship-intelligence-specific. Anyone who manages external relationships through meetings and needs to stay prepared benefits from this exact loop.

---

## The Roles

### Account-Based Roles

#### 1. Customer Success Manager (CSM/TAM)

**Current primary user.** The app was built for this person.

**How they use the app:**
- Portfolio of 15-40 customer accounts
- Daily briefing surfaces which accounts need attention (renewals, risk signals, overdue actions)
- Meeting prep gives them full context before every customer call
- Entity intelligence tracks health trajectory over time
- Actions track commitments made in meetings

**Where the app is strong:**
- Meeting prep is exactly right — stakeholder context + historical + risks + wins
- Daily briefing's "The Meeting" section is the killer feature
- Watch list (risks/wins/unknowns) maps perfectly to CS work
- People/stakeholder management is essential for multi-threaded accounts
- The Record gives chronological interaction history

**Where the app is weak:**
- Metadata is hardcoded (ARR, NPS, health, renewal date) — works for CS, but not configurable
- No renewal workflow (forecast, risk mitigation playbook, expansion tracking)
- Programs section is underbuilt — CS needs success plan tracking, QBR scheduling

**Metadata needs:** ARR, health, lifecycle, renewal date, NPS, CSM owner, champion, support tier, contract dates

---

#### 2. Account Executive (Sales)

**How they'd use the app:**
- Portfolio of 20-60 prospects + active deals
- Daily briefing surfaces deals needing attention (stalled, close date approaching, champion gone quiet)
- Meeting prep is *essential* — sales calls need stakeholder mapping, competitive context, objection prep
- Entity intelligence could track deal momentum, buying committee engagement
- Actions track follow-ups, proposal deliverables, internal coordination

**Where the app is strong:**
- Meeting prep transfers almost perfectly — sales needs the same stakeholder + context + questions pattern
- Daily briefing's prioritization works — swap "renewal proximity" for "close date proximity"
- People management is critical for sales (champion tracking, org chart, multi-threading)
- The Record captures deal history chronologically
- Action tracking with "why now?" reasoning works for follow-up discipline

**Where the app is weak:**
- Metadata is wrong — Sales doesn't have ARR/NPS/health. They have pipeline stage, deal size, forecast, close date, win probability
- "Health" framing doesn't fit — "momentum" or "engagement" is the sales analog
- No pipeline view or funnel visualization
- Watch list needs different vocabulary — "champion risk" not "churn risk"
- No competitive intelligence tracking per deal
- Lifecycle events don't apply — deal stage progression is the analog

**Metadata needs:** Deal size, pipeline stage, close date, win probability, forecast category, champion, economic buyer, competition, next step

**Adaptation required:** Medium. The intelligence loop transfers. Metadata and vocabulary need reconfiguration. No new surfaces needed — the same 7 chapters work with different field labels.

---

#### 3. Agency Account Manager

**How they'd use the app:**
- Portfolio of 8-20 client accounts, each with 1-5 active projects
- Classic "both" entity mode — accounts are clients, projects are engagements/campaigns
- Daily briefing surfaces client fires + project deadlines
- Meeting prep is essential for client calls (satisfaction, deliverable status, scope concerns)
- Entity intelligence on both dimensions: client relationship + project delivery

**Where the app is strong:**
- "Both" entity mode already exists — Accounts (clients) + Projects (engagements)
- Meeting prep works — client calls need relationship context + project status
- People management critical (client stakeholders + internal team)
- Watch list works — scope creep, satisfaction risk, deadline risk all fit the pattern
- The Record tracks client interaction history

**Where the app is weak:**
- Account metadata is CS-specific (ARR doesn't apply — retainer/budget is the analog)
- Project metadata is thin (status + milestones, but no budget/hours/scope tracking)
- No cross-entity dashboard ("Client X has 3 active projects, 2 are behind schedule")
- Intelligence doesn't synthesize across entities well yet
- No deliverable or milestone tracking within projects

**Metadata needs (accounts):** Retainer value, client satisfaction, contract dates, billing contact, service level
**Metadata needs (projects):** Budget, hours, scope status, deliverable count, launch date, campaign type

**Adaptation required:** Medium-high. Entity mode works. Cross-entity intelligence is the gap. Metadata needs reconfiguration on both entity types.

---

### Project-Based Roles

#### 4. Product Manager

**How they'd use the app:**
- Portfolio of 3-8 features/initiatives
- Daily briefing surfaces stakeholder alignment needs, decision deadlines, blocked items
- Meeting prep for stakeholder reviews, sprint planning, design reviews
- Entity intelligence tracks feature trajectory, decision history, dependency risks
- Actions track decisions needed, specs to write, reviews to complete

**Where the app is strong:**
- Meeting prep works — PM meetings need stakeholder context + decision history + open questions
- Watch list maps to PM risks — dependency risks, scope risks, alignment gaps
- The Record tracks decision history chronologically
- People management works for stakeholder mapping (eng lead, design, executives)
- Action tracking with entity links is exactly right

**Where the app is weak:**
- No sprint/velocity metadata
- "Health" doesn't map cleanly — "confidence" or "on-track/at-risk/blocked" is the PM analog
- No dependency tracking between projects
- Intelligence prompts are relationship-focused, not delivery-focused
- No specification/PRD management
- Briefing doesn't surface "decisions needed today" as a first-class concept

**Adaptation required:** Medium. The intelligence loop transfers if prompts are rewritten for delivery context instead of relationship context.

---

### "Both" Mode Roles

#### 5. Consultant / Freelancer

Similar to Agency Account Manager but solo. Clients (accounts) + engagements (projects). Lighter metadata needs but same cross-entity pattern.

#### 6. Startup Founder

Key relationships (investors, customers, partners) as accounts + company initiatives as projects. Unusual but valid. The briefing + prep loop is extremely valuable for someone context-switching between fundraising, customer development, and product work.

---

## What Actually Transfers vs. What Doesn't

### Universal (works for every role today)

| Capability | Why It Transfers |
|-----------|-----------------|
| Daily briefing | Everyone has meetings and priorities |
| Meeting prep | Every external meeting benefits from stakeholder + context + questions |
| Entity intelligence (executive assessment, risks, wins, current state) | The analytical frame applies to any entity |
| People / stakeholder management | Every role manages relationships |
| Action tracking with entity links | Every role makes commitments in meetings |
| The Record (unified timeline) | Chronological history is universally useful |
| Watch list (risks/wins/unknowns) | Every role tracks positive and negative signals |
| Semantic search over entity content | Every role accumulates docs/transcripts/notes |
| The archive (structured markdown) | AI-consumable knowledge base for any domain |

### CS-Specific (needs role adaptation)

| Feature | CS Version | What Other Roles Need |
|---------|-----------|----------------------|
| ARR vital | Annual recurring revenue | Deal size (Sales), Retainer (Agency), Budget (PM) |
| Health status | green/yellow/red | Momentum (Sales), Satisfaction (Agency), Confidence (PM) |
| Renewal date | Contract renewal countdown | Close date (Sales), Launch date (Agency/PM) |
| NPS score | Net promoter score | Win probability (Sales), Client score (Agency), Stakeholder alignment (PM) |
| Lifecycle | Free-text lifecycle stage | Pipeline stage (Sales), Engagement phase (Agency), Initiative phase (PM) |
| Lifecycle events | Renewal, expansion, churn | Won/lost (Sales), Launched/completed (Agency), Shipped/cancelled (PM) |
| Strategic programs | Success plan items | Proposal tracks (Sales), Workstreams (Agency), Epics (PM) |
| Intelligence vocabulary | "churn risk", "value delivered" | "deal stalled", "competitive threat" (Sales) |

### Architectural Gaps (not built yet, needed for multi-role)

| Gap | What's Needed |
|-----|--------------|
| Configurable metadata fields | Replace hardcoded ARR/NPS/health with role-appropriate fields |
| Role-aware AI prompts | Intelligence enrichment should use vocabulary matching the user's role |
| Cross-entity synthesis | "Client X has 3 projects, 2 at risk" — intelligence across entity boundaries |
| Customizable vitals strip | Show different stats based on role |
| Lifecycle event types | Role-specific event categories |

---

## Reconsidering I92: What It Looks Like Today

### Original I92 Vision (Written Early)

"User-configurable metadata fields" with 27 fields from a real TAM workflow CSV. Settings UI to toggle fields. CSV import/export. Blocked by I27 (entity-mode architecture).

### Why This Is Wrong Now

1. **It's too granular.** 27 custom fields is a database admin tool, not a productivity app. DailyOS is opinionated (P4). We should pick the right fields per role, not expose a field builder.

2. **It conflates configuration with role selection.** The user shouldn't configure individual fields — they should pick their role and get the right fields automatically. P3 (Buttons, Not Commands) means "Select your role" not "Configure your schema."

3. **Kits are overbuilt.** ADR-0046 proposed Kits as installable modules with templates, vocabulary, enrichment fragments, and field schemas. That's plugin architecture. We don't need plugins — we need presets.

4. **The real need is vocabulary, not fields.** The biggest difference between a CSM and an AE using DailyOS isn't which metadata fields appear. It's how the AI talks about their work. "Churn risk" vs "deal stalled." "Value delivered" vs "ROI demonstrated." "Renewal in 30d" vs "Close date in 30d." The intelligence prompts are the product surface that matters most.

### What I92 Should Become

**Role presets** — a lightweight configuration layer that adjusts:

1. **Metadata fields** — Which fields appear on entity detail pages and vitals strips
2. **AI vocabulary** — How intelligence prompts frame analysis
3. **Prioritization signals** — What urgency means (renewal proximity vs close date vs deadline)
4. **Event types** — What lifecycle events are available (renewal/churn vs won/lost vs launched/completed)
5. **Default entity mode** — Account, project, or both

**Not:** a custom field builder, CSV schema manager, or plugin system.

---

## Proposed Role Presets

### The Shipped Presets (v0.10.0)

These ship with the app. Each is opinionated enough to feel purposeful on day one, but escapable (P4) — users can always override fields and vocabulary later.

---

#### Customer Success

The founding role. The one we know best.

**Entity mode default:** Account
**Who this is for:** CSMs, TAMs, Account Managers in SaaS, anyone responsible for post-sale customer relationships
**Metadata fields:** ARR, Health (green/yellow/red), Lifecycle (free text), Renewal Date, NPS, Contract Dates, Support Tier
**Vitals strip:** ARR → Health → Lifecycle → Renewal countdown → NPS → Meeting frequency
**AI vocabulary:** "account health", "churn risk", "value delivered", "renewal readiness", "expansion opportunity", "multi-threaded", "executive sponsor"
**Lifecycle events:** Renewal, Expansion, Contraction, Churn, Escalation
**Prioritization:** Renewal proximity + health decline + meeting gap
**What the briefing emphasizes:** Accounts needing attention, upcoming renewals, relationship gaps, prep for customer calls

---

#### Sales

The closest cousin to CS. Same account-based structure, different temporal frame — Sales is forward-looking (close the deal) where CS is present-tense (keep the relationship).

**Entity mode default:** Account
**Who this is for:** Account Executives, SDRs/BDRs, Sales Engineers, Revenue leaders
**Metadata fields:** Deal Size, Pipeline Stage (Prospect / Discovery / Proposal / Negotiation / Closed Won / Closed Lost), Close Date, Win Probability, Forecast Category, Competition
**Vitals strip:** Deal Size → Stage → Close Date countdown → Win Probability → Meeting frequency
**AI vocabulary:** "deal momentum", "champion engagement", "competitive threat", "buying committee", "next step", "pipeline risk", "stalled", "multi-threaded"
**Lifecycle events:** Stage Advance, Won, Lost, Reopen, Slipped
**Prioritization:** Close date proximity + stage stall duration + champion silence + deal size weighting
**What the briefing emphasizes:** Deals at risk of stalling, upcoming close dates, champion engagement gaps, competitive intel before meetings

---

#### Partnerships

Partnerships lives in the same account-based world as Sales but operates on longer timescales and mutual value. A partner isn't a customer to retain or a deal to close — it's a relationship to cultivate where both sides need to win.

**Entity mode default:** Both (partners as accounts, joint initiatives as projects)
**Who this is for:** Partner Managers, BD leads, Channel Managers, Alliance Managers, Ecosystem leads
**Metadata fields (accounts):** Partner Tier, Revenue Share/Referral Value, Integration Status, Agreement Dates, Joint Customers
**Metadata fields (projects):** Initiative Type (co-sell / co-build / co-market), Status, Launch Target, Shared KPIs
**Vitals strip:** Partner Tier → Revenue Impact → Integration Status → Agreement countdown → Joint activity frequency
**AI vocabulary:** "mutual value", "integration health", "co-sell pipeline", "partner engagement", "enablement gap", "ecosystem fit", "channel momentum"
**Lifecycle events:** Agreement signed, Integration launched, Renewal, Tier change, Wind-down
**Prioritization:** Agreement renewal proximity + integration stall + co-sell activity decline + joint meeting prep
**What the briefing emphasizes:** Partner relationships needing investment, stalled integrations, upcoming joint activities, enablement opportunities

---

#### Agency

Client service with a delivery dimension. Accounts are relationships; projects are the work.

**Entity mode default:** Both
**Who this is for:** Account Directors at agencies, Client Services leads, Studio Managers, anyone managing client relationships alongside project delivery
**Metadata fields (accounts):** Retainer/Budget, Client Satisfaction, Contract Dates, Service Level, Account Lead
**Metadata fields (projects):** Budget, Hours Allocated, Status, Launch Date, Deliverable Count, Brief Status
**Vitals strip (accounts):** Retainer → Satisfaction → Contract countdown → Active projects → Meeting frequency
**AI vocabulary:** "client satisfaction", "scope risk", "deliverable status", "retainer utilization", "creative brief", "campaign performance", "client feedback"
**Lifecycle events (accounts):** Contract renewal, Scope expansion, Wind-down
**Lifecycle events (projects):** Kickoff, Creative review, Launch, Wrap, Post-mortem
**Prioritization:** Client satisfaction signals + deadline proximity + overdue deliverables + scope creep alerts
**What the briefing emphasizes:** Client fires, approaching deadlines, deliverable status across clients, scope at risk

---

#### Consulting

Consulting shares DNA with Agency (client + engagement) but the deliverable is insight, not creative output. The vocabulary is different — findings, recommendations, workstreams — and the relationship dynamic skews toward trusted advisor rather than vendor.

**Entity mode default:** Both (clients as accounts, engagements as projects)
**Who this is for:** Management consultants, Strategy consultants, Implementation leads, Advisory firms
**Metadata fields (accounts):** Engagement Value, Client Relationship Stage, Decision-Maker Access, Firm/Practice Area
**Metadata fields (projects):** Engagement Type (strategy / implementation / advisory), Phase, Deliverable Status, Steering Committee Date
**Vitals strip (accounts):** Engagement Value → Relationship Stage → Decision-Maker Access → Active engagements → Meeting frequency
**AI vocabulary:** "client relationship", "findings", "recommendations", "workstream", "steering committee", "deliverable", "advisor access", "follow-on opportunity"
**Lifecycle events (accounts):** Engagement won, SOW signed, Engagement completed, Follow-on, Relationship dormant
**Lifecycle events (projects):** Discovery, Analysis, Synthesis, Presentation, Implementation support
**Prioritization:** Steering committee proximity + deliverable deadlines + client access gaps + follow-on signals
**What the briefing emphasizes:** Upcoming client presentations, workstream blockers, relationship depth, findings readiness

---

#### Product

The project-native role. Entities are features, initiatives, or bets — not customer relationships.

**Entity mode default:** Project
**Who this is for:** Product Managers, Technical PMs, Product Owners, Group PMs
**Metadata fields:** Status (planning / active / blocked / shipped), Confidence (high / medium / low), Target Date, Milestone Count, Stakeholder Count
**Vitals strip:** Status → Confidence → Target Date countdown → Milestones completed → Open decisions
**AI vocabulary:** "stakeholder alignment", "dependency risk", "scope decision needed", "shipped", "blocked by", "decision deadline", "user impact", "trade-off"
**Lifecycle events:** Planning started, Development started, Alpha/Beta, Shipped, Sunset, Descoped
**Prioritization:** Decision deadlines + blocked items + stakeholder meeting proximity + confidence decline
**What the briefing emphasizes:** Decisions needed today, blocked initiatives, upcoming stakeholder reviews, cross-team dependencies

---

#### Leadership

An executive or senior leader whose day is defined by people, decisions, and strategic oversight — not direct delivery. The entity model inverts: direct reports and key stakeholders are as important as initiatives. Cross-entity synthesis matters more here than anywhere else.

**Entity mode default:** Both (key relationships/teams as accounts, strategic initiatives as projects)
**Who this is for:** VPs, Directors, Chiefs of Staff, GMs, Department heads, Founders who've scaled past doing everything themselves
**Metadata fields (accounts/teams):** Team Size, Budget, Key Metric, Review Cadence, Owner
**Metadata fields (projects):** Strategic Priority (P0/P1/P2), Status, Executive Sponsor, Board Visibility, Target Date
**Vitals strip:** Strategic priorities in-flight → Decisions pending → 1:1s this week → Team health signals → Meeting load
**AI vocabulary:** "delegation", "waiting on", "decision needed", "strategic alignment", "team capacity", "escalation", "board-ready", "cross-functional"
**Lifecycle events (teams):** Reorg, Hire, Departure, Budget change, OKR cycle
**Lifecycle events (initiatives):** Greenlit, Funded, Milestone, Shipped, Killed, Pivoted
**Prioritization:** Decision urgency + delegation follow-ups + 1:1 prep quality + strategic initiative blockers
**What the briefing emphasizes:** What needs your decision today, who needs your attention, which initiatives are off-track, upcoming board/leadership moments

---

#### The Desk

The catch-all — but not a lesser preset. Named for the DailyOS brand metaphor: your desk, arranged the way you work. This is for people whose work doesn't fit a predefined role, or who wear multiple hats that no single preset captures. Researchers, freelancers, academics, non-profit leaders, independent operators.

The Desk ships with minimal, neutral metadata and clean vocabulary. It's a starting point, not a destination — and it's the natural base for community-created presets.

**Entity mode default:** Both
**Who this is for:** Researchers, freelancers, academics, non-profit program managers, people who wear too many hats for one label, anyone who wants a blank canvas
**Metadata fields (accounts):** Status (free text), Priority (free text), Key Date
**Metadata fields (projects):** Status (free text), Priority (free text), Target Date
**Vitals strip:** Status → Priority → Key Date → Meeting frequency → Open actions
**AI vocabulary:** Neutral professional language — "relationship", "progress", "risk", "opportunity", "next step", "blockers"
**Lifecycle events:** Started, Milestone, Completed, Paused, Archived
**Prioritization:** Due date proximity + meeting prep readiness + action overdue count
**What the briefing emphasizes:** What's active, what needs attention, who you're meeting, what's due

---

### Community Presets (v0.10.0+)

Role presets are JSON files. The schema is simple enough that anyone — or any AI — can create one:

```jsonc
{
  "name": "Venture Capital",
  "description": "For investors managing a portfolio of companies",
  "author": "community",
  "entityModeDefault": "account",
  "metadata": {
    "account": [
      { "key": "investment_amount", "label": "Investment", "type": "currency" },
      { "key": "stage", "label": "Stage", "type": "select", "options": ["Pre-Seed", "Seed", "Series A", "Series B", "Growth"] },
      { "key": "board_seat", "label": "Board Seat", "type": "boolean" },
      { "key": "next_raise", "label": "Next Raise", "type": "date" },
      { "key": "runway_months", "label": "Runway", "type": "number", "suffix": "months" }
    ]
  },
  "vocabulary": {
    "entity_noun": "portfolio company",
    "health_frame": "trajectory",
    "risk_vocabulary": ["burn rate concern", "founder risk", "market shift", "competitive pressure"],
    "win_vocabulary": ["PMF signal", "revenue milestone", "key hire", "fundraise closed"],
    "urgency_signals": ["runway < 6 months", "board meeting approaching", "fundraise in progress"]
  },
  "vitals": ["investment_amount", "stage", "runway_months", "next_raise", "meeting_frequency"],
  "lifecycleEvents": ["Investment", "Board meeting", "Follow-on", "Fundraise", "Exit", "Write-down"],
  "prioritization": {
    "primary": "board_meeting_proximity",
    "secondary": ["runway_urgency", "fundraise_activity", "founder_engagement"]
  }
}
```

This format is:
- **Human-readable** — a user can edit it in any text editor
- **AI-generatable** — "Create a DailyOS role preset for a recruiter who manages candidates and open requisitions" is a one-shot prompt
- **Shareable** — drop it in a GitHub repo, share a link, import via Settings
- **Discoverable** — a community gallery (even just a GitHub repo with a README) lets people browse and fork presets

**Example community presets people might create:**
- **Venture Capital** — Portfolio companies, board prep, runway tracking
- **Recruiting** — Candidates as people-first entities, roles as projects, pipeline stages
- **Non-Profit Program Manager** — Grants as accounts, programs as projects, funder relationships
- **Real Estate Agent** — Properties as projects, clients as accounts, transaction stages
- **Academic Researcher** — Papers as projects, collaborators as people, grant cycles
- **Journalist** — Sources as people, stories as projects, editorial calendar
- **Event Planner** — Clients as accounts, events as projects, vendor relationships

The long tail is infinite. We ship 8 good ones; the community builds the rest. This is the open-source surface that invites contribution without requiring anyone to write Rust.

---

## Implementation Thinking

### What Changes Per Role

| Layer | What Changes | Effort |
|-------|-------------|--------|
| **Config** | `role` field replaces `profile`. Role implies entity mode default + field set. | Low |
| **DB schema** | Metadata stored as JSON blob or key-value pairs, not hardcoded columns. | Medium |
| **Vitals strip** | Reads field config to determine which vitals to show. | Low |
| **Entity detail page** | Field editor drawer shows role-appropriate fields. | Medium |
| **Intelligence prompts** | Role-specific vocabulary injected into enrichment prompts. | Low |
| **Briefing prioritization** | Role-specific urgency signals in scoring. | Low |
| **Lifecycle events** | Event type enum configured per role. | Low |

### What Doesn't Change

- The 7-chapter entity detail layout (Headline, State of Play, The Room, Watch List, The Record, The Work, Appendix)
- Daily briefing structure (Hero, Focus, The Meeting, Schedule, Priorities, Finis)
- Meeting prep sections (stakeholders, context, risks, wins, questions, open items)
- People/stakeholder management
- Action system
- Archive structure
- Semantic search
- Intelligence JSON schema (executive_assessment, risks, wins, current_state — all still apply)

### The Key Insight

The editorial intelligence surfaces are role-agnostic. **Chapters are universal; fields are role-specific.** "State of Play" works whether you're assessing account health (CS), deal momentum (Sales), client satisfaction (Agency), or feature confidence (PM). The analytical frame — what's working, what's not, what we don't know — is universal.

The same is true for Watch List (risks/wins/unknowns), The Room (stakeholders), The Record (timeline), and The Work (actions + readiness). These aren't CS concepts — they're relationship intelligence concepts.

**What makes DailyOS CS-specific today is vocabulary and metadata, not architecture.**

---

## Risks and Open Questions

1. **Scope vs focus.** Going multi-role before CS is excellent risks making the app mediocre for everyone. CS should remain the primary, most-polished experience.

2. **Metadata as JSON blob vs columns.** Moving from hardcoded columns (`arr`, `nps`, `health`) to a flexible store is a schema migration with real complexity. Worth it for multi-role, but not trivial.

3. **Prompt engineering per role.** Intelligence quality depends on vocabulary precision. Generic prompts that try to serve all roles will be worse than role-specific ones. Each role preset needs its own prompt fragments.

4. **Onboarding.** Role selection should happen at onboarding (between entity mode and workspace setup). The whole onboarding flow adapts to the selected role.

5. **Switching roles.** What happens to metadata when someone switches from Sales to CS? (Answer: nothing. The fields are additive. You just see different ones. No data loss.)

6. **Role as identity vs role as lens.** Some users have multiple roles (founder who does sales AND product). Should role be a global setting or a per-entity-mode lens? For v1, global is simpler and sufficient.

7. **"Both" mode for Sales.** AEs increasingly manage both prospecting (projects/campaigns) and accounts. "Both" mode might be more common in Sales than we assumed.

---

## Recommendation

**Rewrite I92 as "Role Presets"** — a lightweight configuration layer that ships with 8 opinionated presets and a JSON format for community-created ones.

**Shipped presets (v0.10.0):** Customer Success, Sales, Partnerships, Agency, Consulting, Product, Leadership, The Desk

**Do not build:**
- A custom field builder UI
- A kit/plugin system with code hooks
- CSV schema management
- Per-field toggle settings

**Build:**
- Role selection in onboarding + settings
- JSON-based role preset schema (metadata fields, vocabulary, prioritization, lifecycle events)
- Role-specific metadata stored as flexible key-value, displayed by role config
- Role-specific prompt fragments for intelligence enrichment
- Role-aware vitals strip
- Role-specific lifecycle event types
- Preset import from file (for community presets)

**Priority:** 0.10.0 (Renewal). This is the market expansion unlock. CS must be excellent first (0.8.0 Editorial → 0.8.x Hardening → 0.9.0 Integrations → then 0.10.0).

**Estimated scope:** 1-2 sprints. Most work is prompt engineering per role + metadata schema migration. No new editorial surfaces — the 7-chapter layout and daily briefing structure are unchanged.

**Supersedes:** I92 (user-configurable metadata fields), the Kit concept from ADR-0046 (CS Kit, Sales Kit, PM Kit, Marketing Kit), ADR-0051 (metadata configurability approach).
