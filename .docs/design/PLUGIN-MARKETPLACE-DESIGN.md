# DailyOS Plugin Marketplace Design

> "I've got to put together a Risk Report on Nielsen." That's all you say. And it just happens.

---

## The DailyOS Loop

DailyOS exists in a three-layer ecosystem (VISION.md):

```
Layer 1: Domain Intelligence     → CRM, Transcripts, Email, Calendar
Layer 2: Operational Memory      → DailyOS app (maintains readiness)
Layer 3: Creative/Analytical     → Claude Code / Cowork (does the work)
```

The app handles Layer 2 — it runs at 6am, maintains entity intelligence, generates briefings and meeting prep, tracks actions, builds relationship history. The user opens the app, consumes what's ready, and goes to work.

**But then what?**

The briefing told Sarah her Nielsen renewal is in 60 days and health just went yellow. The prep showed a champion transitioning roles. The action trail has two overdue commitments.

Now Sarah needs to _do something_. Write a risk assessment. Draft an executive outreach email. Build a success plan revision. Prepare her VP for the escalation call.

This is where Layer 3 enters. Sarah opens Claude Code or starts a Cowork session. And this is where the plugin lives — **the bridge between knowing and doing**.

```
┌────────────────────────────────────────────────────────────────┐
│  THE DAILYOS LOOP                                              │
│                                                                │
│  DailyOS App ──maintains──► Workspace Files                    │
│       ▲                          │                             │
│       │                          │ plugin reads                │
│       │                          ▼                             │
│  enriches                   Claude Code + Plugin               │
│  workspace                       │                             │
│       │                          │ produces                    │
│       │                          ▼                             │
│       └──────────────────── Deliverable + Loop-back            │
│                                                                │
│  The workspace gets smarter every time the loop completes.     │
└────────────────────────────────────────────────────────────────┘
```

**What makes this different from every other AI tool:**

When Sarah says "put together a risk report on Nielsen," Claude Code doesn't ask "tell me about Nielsen." It doesn't need six rounds of context-building. It doesn't produce a generic template. It reads the workspace that DailyOS has been maintaining for months — intelligence, meeting history, stakeholder map, action trail, email signals, engagement patterns — and produces a risk report that's specific, accurate, and ready to send.

**No startup tax. No context development. No "let me get you up to speed."**

The plugin IS the context. DailyOS was in every meeting. It knows.

---

## Six Personas, Six Scenarios

Before defining plugin structure, let's trace what actually happens for six different users. Each scenario follows the same pattern: what they say → what the plugin reads → what it produces → what loops back.

### 1. Sarah — Customer Success Manager

**Scenario: "Put together a risk report on Nielsen"**

**What she's dealing with:**
Sarah manages 15 enterprise accounts. Nielsen's renewal is 60 days out. Health score dropped from Green to Yellow two weeks ago. Their technical champion Sarah Chen mentioned transitioning roles in the last call. Two action items from that call are overdue. The executive sponsor hasn't been in a meeting since October.

**What the plugin reads:**
```
Accounts/Nielsen/
├── dashboard.json          → ARR, health, renewal date, lifecycle stage
├── intelligence.json       → executive_assessment, risks, wins, current_state
├── stakeholders.md         → Champion, executive sponsor, buyer roles
└── actions.md              → Open items, overdue commitments

People/Sarah Chen/
├── person.json             → Role, relationship classification
└── person.md               → Meeting signals, temperature, last seen

People/David Park/            (executive sponsor)
├── person.md               → Last meeting: October. Temperature: cooling.

_archive/2026-01/           → Last 3 meeting summaries with Nielsen
_archive/2026-02/           → Most recent call notes

data/actions.json           → Filtered for Nielsen: 2 overdue items
data/emails.json            → Recent email signals from Nielsen domain
```

**What it produces:**

A structured risk assessment document, vocabulary shaped by the CS preset:

```markdown
# Nielsen — Risk Assessment
**Date:** February 17, 2026 | **Renewal:** April 18, 2026 (60 days)
**Health:** Yellow (declined from Green, Feb 3)

## Executive Summary

Nielsen's renewal is at moderate risk. Three converging signals demand
attention: a declining health trajectory, an impending champion transition,
and an absent executive sponsor. The account has delivered strong value
(Phase 2 rollout completed, 23% ticket resolution improvement) but the
relationship infrastructure is weakening at exactly the wrong time.

## Risk Factors

### 1. Champion Transition (Critical)
Sarah Chen (VP Engineering) disclosed a role change effective Q2. She has
been the primary technical advocate since onboarding. No successor has
been identified. In the Jan 28 call, she mentioned "making sure someone
picks this up" — language suggesting she doesn't have a transition plan.

**Evidence:** Jan 28 meeting summary, stakeholder map
**Impact:** Loss of technical advocacy during renewal window
**Recommended action:** Schedule dedicated transition meeting by Feb 24

### 2. Executive Sponsor Gap (High)
David Park (CFO) has not attended a meeting since October 14. His last
engagement was the Q3 QBR where he raised questions about ROI measurement
that were addressed in November's follow-up — but he wasn't present for
the follow-up. Four months of silence from the budget authority heading
into renewal.

**Evidence:** Meeting signals (People/David Park), Q3 QBR archive
**Impact:** No executive air cover for renewal conversation
**Recommended action:** Request EBR with David Park, frame as renewal planning

### 3. Overdue Commitments (Medium)
Two action items from the Jan 28 call remain open:
- Send updated API documentation (due Feb 7 — 10 days overdue)
- Schedule APAC training session (due Feb 14 — 3 days overdue)

Undelivered commitments erode trust during a renewal window. Both are
internally owned.

**Evidence:** Action trail, Jan 28 meeting summary
**Impact:** Trust erosion at a critical moment
**Recommended action:** Clear both this week, acknowledge delay proactively

## Strengths to Leverage

- Phase 2 rollout completed successfully (Jan 12)
- 23% improvement in ticket resolution time (measurable value)
- 67 new users onboarded across 3 teams (adoption growth)
- Sarah Chen remains a strong advocate through her transition

## Recommended Actions (Priority Order)

1. Clear overdue API doc and APAC training items by Feb 21
2. Schedule champion transition meeting with Sarah Chen by Feb 24
3. Request EBR with David Park, positioned as renewal planning
4. Prepare value narrative connecting Phase 2 outcomes to renewal case
5. Identify and begin building relationship with Sarah's successor

## 60-Day Renewal Timeline

| Week | Action | Owner |
|------|--------|-------|
| This week | Clear overdue items, draft EBR request | Sarah (you) |
| Feb 24 | Champion transition meeting | Sarah + Sarah Chen |
| Mar 3 | EBR with David Park | Sarah + VP |
| Mar 10 | Renewal proposal draft | Sarah |
| Mar 17 | Internal renewal review | Sarah + VP |
| Apr 1 | Renewal presentation to Nielsen | Sarah + VP |
```

**What loops back:**
- Risk report saved to `Accounts/Nielsen/risk-report-2026-02.md`
- Five recommended actions created in the action system
- Intelligence.json updated: `champion_transition_risk` added to risks
- Timeline becomes trackable through the action trail

**Why this is impossible without DailyOS:**
A generic AI would ask "tell me about your relationship with Nielsen." Then "what are the risks?" Then "who are the stakeholders?" Then "what's the renewal timeline?" Six rounds of context-building before you get a draft that's probably wrong. With the plugin, the workspace IS the context. One sentence → deliverable.

---

### 2. Marcus — Account Executive

**Scenario: "Write a follow-up email to David at TechCorp after our call today"**

**What he's dealing with:**
Marcus had a demo call with TechCorp this afternoon. David (VP Product) was engaged but raised concerns about integration complexity. The deal is in evaluation stage, $180K ACV. A competitor (Vendorly) was mentioned for the first time. Marcus promised to send a technical architecture overview and connect David with a solutions engineer.

**What the plugin reads:**
```
Accounts/TechCorp/
├── dashboard.json          → Stage: Evaluation, ACV: $180K, champion: David
├── intelligence.json       → Current state, competitive landscape
└── stakeholders.md         → David's priorities, concerns, decision role

People/David Martinez/
├── person.md               → Communication preferences, meeting history

_archive/2026-02/           → Today's meeting notes (if captured)
data/actions.json           → Commitments from today's call
```

**What it produces:**

Not a template. An email grounded in the actual conversation:

```
Subject: Technical Architecture Overview + SE Introduction — TechCorp

David,

Thanks for the thorough walkthrough today. Your questions about the
integration layer told me you're thinking about this the right way —
especially the concern about maintaining your existing event pipeline
during migration.

Two things I committed to:

1. Technical architecture overview — attached. Section 3 specifically
   addresses the event pipeline coexistence model you asked about.
   The short answer: parallel operation during migration, no cutover
   required until you're confident.

2. Solutions engineer introduction — I'm connecting you with Alex Torres,
   who led the integration for [similar customer]. He can walk through
   the specifics of what a phased migration looks like for your stack.
   I'll get a time on the calendar this week.

On the evaluation timeline: you mentioned wanting to present options to
your engineering leads by March 7. I'd suggest we get Alex's technical
deep-dive done by end of next week so you have the architecture clarity
before that conversation.

Worth a 20-minute sync Thursday to make sure we're aligned on what your
team needs to see?

Marcus
```

**What loops back:**
- Email draft saved to workspace for reference
- Two commitments tracked as actions (architecture doc, SE intro)
- Competitive signal (Vendorly mention) logged in entity intelligence
- Next step (Thursday sync) flagged for calendar follow-up

---

### 3. Elena — Agency Account Director

**Scenario: "I need a client status update for Meridian, we have our weekly call in an hour"**

**What she's dealing with:**
Elena manages three accounts at her agency. Meridian is her biggest client — a rebrand campaign in execution. The design team delivered revised concepts last week. The client's marketing director had concerns about the color palette. A scope change request came in via email yesterday (adding social media templates to the deliverable set). The team is already at capacity.

**What the plugin reads:**
```
Accounts/Meridian/              (or Projects/Meridian-Rebrand/)
├── dashboard.json          → Project status, deliverables, timeline
├── intelligence.json       → Client satisfaction signals, scope history

People/Rachel Torres/         (marketing director)
├── person.md               → Preferences, communication style, concerns

_archive/2026-02/           → Last week's review meeting, concept feedback
data/actions.json           → Open deliverables, feedback items
data/emails.json            → Yesterday's scope change request
```

**What it produces:**

A client-ready status update — not internal jargon, but language appropriate for the client call:

```markdown
# Meridian Rebrand — Weekly Status
**Week of February 17, 2026**

## This Week

**Concept revisions delivered.** The updated brand concepts incorporating
Rachel's feedback on typography hierarchy and spacing went out Wednesday.
Three directions are ready for review:
- Direction A: Refined original with adjusted color weight
- Direction B: New palette exploration per Rachel's color concerns
- Direction C: Hybrid approach

**Awaiting:** Rachel's preferred direction by Thursday to keep the
March 7 signoff timeline intact.

## Flag: Scope Change Request

Received a request yesterday to add social media templates (Instagram,
LinkedIn, Twitter/X) to the deliverable set. This was not in the
original SOW.

**Options to discuss:**
1. Add to current scope — extends timeline by ~2 weeks, additional
   investment of $12K based on template complexity
2. Phase 2 engagement — deliver brand system first, templates as
   fast-follow after launch
3. Simplified set — 2 platforms instead of 3, fits within original
   timeline with minimal additional investment ($4K)

**Recommendation:** Option 2 (Phase 2). Keeps the launch date firm
and gives us a clean deliverable boundary.

## Open Items

- [ ] Rachel: Select preferred concept direction (due: Thu Feb 19)
- [ ] Elena: Send production timeline for approved direction (due: day after selection)
- [ ] Scope change: Decision needed on social templates approach
- [x] Design team: Deliver revised concepts ✓ (completed Feb 12)

## Risk Watch

**Timeline pressure:** If concept selection slips past Thursday, the
March 7 signoff is at risk. Every day of delay compresses production.
Recommend framing this urgency in today's call without creating pressure
— "We want to protect your launch date."
```

**What loops back:**
- Status update archived in project directory
- Scope change noted in intelligence (scope drift signal)
- Timeline risk flagged in dashboard
- Decision items tracked as actions

---

### 4. David — VP of Customer Success

**Scenario: "Build my section for the board deck — customer health and retention"**

**What he's dealing with:**
David manages a team of 8 CSMs covering a $24M portfolio. Two renewals this week. One account in escalation. The team closed 3 expansions last month. One CSM is stretched across too many accounts. The board meeting is Thursday.

**What the plugin reads:**
```
Accounts/*/
├── dashboard.json          → Every account: ARR, health, renewal date
├── intelligence.json       → Every account: risks, wins, assessment

data/actions.json           → Portfolio-wide action health
People/*/person.md          → Stakeholder engagement signals

_archive/2026-01/           → Last month's meeting summaries (expansion signals)
_archive/2026-02/           → This month to date
```

**What it produces:**

Not a data dump. A board-ready narrative:

```markdown
# Customer Health & Retention — Board Update
**Q1 2026 (through February 17)**

## Portfolio Summary

| Metric | Current | QoQ | YoY |
|--------|---------|-----|-----|
| Total ARR | $24.2M | +3.4% | +18% |
| Net Revenue Retention | 112% | — | +4pp |
| Gross Retention | 96% | — | +2pp |
| Accounts Green | 71% (34/48) | +2 | +8 |
| Accounts Yellow | 23% (11/48) | -1 | -4 |
| Accounts Red | 6% (3/48) | -1 | -4 |

## The Story

Portfolio health is the strongest it's been since launch. Net retention
at 112% reflects a pattern of organic expansion from accounts that
graduated from implementation to value realization. Three expansions
closed in January totaling $890K — all driven by the product's platform
team adoption play that was deprioritized last quarter and then
resurged through champion advocacy.

The risk concentration has narrowed: three red accounts (down from four)
representing $2.1M ARR. Two are actively managed with executive
involvement. The third (Nielsen, $450K) is the watch item — see below.

## Key Wins (January-February)

**Acme Corp expansion: +$340K ARR.** Went from 3 teams to 7 after their
VP Engineering became an internal champion. This followed the pattern
we've seen in 5 of our last 8 expansions: platform team success →
engineering leadership advocacy → enterprise-wide adoption.

**GlobalTech saved from churn.** Was Red in December after a failed
migration. Dedicated SE engagement + weekly executive syncs brought them
back to Yellow with a signed renewal at flat rate. Avoiding $280K in
churn.

**Atlas Partners NPS: 42 → 68.** Direct result of the dedicated
onboarding program launched in Q4. Atlas is now our strongest reference
customer in the financial services vertical.

## Watch Items

**Nielsen ($450K, renewal April 18).** Health declined to Yellow.
Champion transitioning roles, executive sponsor disengaged since October.
CSM has a risk mitigation plan in execution — champion transition
meeting scheduled Feb 24, EBR with CFO targeted for March 3. This is
the renewal most likely to need executive involvement from our side.

**Team capacity.** The CSM covering our financial services segment
has 18 accounts — 3 above target. Two of those accounts (including
Atlas) are in expansion conversations that require intensive engagement.
Recommending a hire or redistribution in Q2 planning.

## Renewals This Quarter

| Account | ARR | Date | Confidence | Risk |
|---------|-----|------|-----------|------|
| FastTrack | $320K | Feb 21 | High | None |
| Beacon Labs | $195K | Feb 28 | High | None |
| Nielsen | $450K | Apr 18 | Medium | Champion + sponsor gap |
| 4 others | $1.8M | Mar-Apr | High | No flags |

**Q1 renewal forecast: 97% gross retention expected.** Nielsen is the
swing factor between 97% and 95%.

## Ask

1. Executive engagement for the Nielsen renewal — request a joint call
   with their CFO in early March
2. Q2 headcount approval to address financial services capacity gap
```

**What loops back:**
- Board narrative archived in a leadership directory
- Portfolio health snapshot preserved for trend analysis
- The very act of synthesis surfaces patterns (the "5 of 8 expansions follow the same pattern" insight) that feed back into strategic intelligence

---

### 5. Kai — Product Manager

**Scenario: "Synthesize everything we've heard about the search feature in the last 6 weeks"**

**What he's dealing with:**
Kai owns the search experience. The feature shipped 6 weeks ago. Feedback is scattered across customer calls, internal syncs, and email threads. He has a roadmap planning session next week and needs to understand what's working, what's not, and what to prioritize.

**What the plugin reads:**
```
_archive/2026-01/           → All meeting summaries mentioning "search"
_archive/2026-02/           → All meeting summaries mentioning "search"
Accounts/*/intelligence.json → Current state sections mentioning search
People/*/person.md          → Contacts who discussed search
data/emails.json            → Email threads mentioning search
Projects/Search-V2/         → Project dashboard if exists
```

**What it produces:**

A user research synthesis — not a list of complaints, but a structured analysis:

```markdown
# Search Feature — User Feedback Synthesis
**Period:** January 6 — February 17, 2026 (6 weeks post-launch)
**Sources:** 14 customer meetings, 6 internal syncs, 8 email threads

## Summary

Search is delivering on its core promise — users find what they need
faster. But three friction patterns have emerged that risk undermining
adoption in larger deployments. The loudest feedback (speed) is actually
the least critical. The quietest (permission scoping) is the most urgent.

## What's Working

**Core search quality praised in 9 of 14 customer conversations.**
Typical quote: "It actually finds what I'm looking for now" (Sarah Chen,
Nielsen, Jan 28). Users with less than 500 documents report high
satisfaction. The relevance algorithm is doing its job.

**Adoption is organic.** Three accounts reported teams discovering and
using search without being trained. This matches the design goal of
zero-configuration utility.

## Friction Patterns

### 1. Permission Scoping (Critical — 6 mentions)

**The pattern:** In enterprise accounts with role-based access, search
returns results users shouldn't see. Not a security breach (they can't
open the documents), but the titles and snippets in search results leak
information about projects, clients, or initiatives they're not involved in.

**Who raised it:**
- David Park, Nielsen CFO (Jan QBR): "If my team can see project names
  from other departments in search results, that's a problem."
- Rachel Torres, Meridian (Feb 5): "We need to scope search by team."
- Engineering team (internal sync Feb 10): Confirmed this is an
  architectural limitation of the current indexing approach.

**Impact:** Blocks adoption at enterprise accounts. Nielsen and two
others have restricted search rollout to pilot teams.

**Recommendation:** P0. Fix before next sales cycle. Without permission
scoping, search can't be positioned as enterprise-ready.

### 2. Large Corpus Performance (Medium — 4 mentions)

**The pattern:** Accounts with 2,000+ documents experience 3-5 second
search latency. Users expect sub-second results. Two accounts reported
users abandoning search after initial slow experience.

**Who raised it:**
- Acme Corp engineering leads (Feb 3): "Our team tried it once, saw
  the spinner, and went back to Ctrl+F."
- Internal performance review (Feb 7): Confirmed O(n) scan on
  unindexed metadata fields.

**Impact:** Adoption ceiling at large accounts. Not a blocker but a
growth limiter.

**Recommendation:** P1. Index optimization is a known path. Schedule
for next sprint.

### 3. Search Context (Low — 3 mentions)

**The pattern:** Results show document title and a snippet but don't
indicate why the result matched or where in the document the match
lives. Users want highlighted matches and section-level context.

**Impact:** Quality-of-life, not blocking adoption. Nice-to-have for
power users.

**Recommendation:** P2. Design improvement, not engineering emergency.

## Stakeholder Sentiment Map

| Person | Account | Sentiment | Key Quote |
|--------|---------|-----------|-----------|
| Sarah Chen | Nielsen | Positive | "Actually finds what I'm looking for" |
| David Park | Nielsen | Concerned | "Title leakage is a problem" |
| Rachel Torres | Meridian | Mixed | "Love it for our team, can't roll out wider" |
| Engineering | Acme | Neutral→Negative | "Tried once, too slow, went back to Ctrl+F" |

## Roadmap Recommendation

1. **Now:** Permission scoping (unblocks enterprise, P0)
2. **Next sprint:** Index optimization for large corpus (P1)
3. **Q2:** Search context / highlighting (P2)
4. **Watch:** Two accounts requested search-as-API for their own tools —
   potential platform play worth exploring in H2
```

**What loops back:**
- Synthesis saved to `Projects/Search-V2/feedback-synthesis-2026-02.md`
- Stakeholder sentiment updates People intelligence
- The permission scoping pattern flagged across multiple accounts feeds back into those accounts' intelligence ("search deployment blocked by permission scoping")
- Roadmap recommendation becomes trackable project actions

---

### 6. Priya — Content Marketing Manager

**Scenario: "I need to decide whether we should pivot the Q2 launch to focus on the developer audience instead of enterprise buyers"**

**What she's dealing with:**
Priya leads content and campaigns for a B2B SaaS company. The Q2 product launch is 6 weeks out. The original positioning targets enterprise buyers — the company's traditional audience. But three signals are converging: developer-led adoption is driving 60% of new signups (per the Product team's dashboard), their biggest competitor just announced a developer-first rebrand, and the last two enterprise campaigns underperformed against benchmarks. Priya's CMO wants a recommendation by Thursday.

This isn't a "write something" task. This is a "think through something" task. The plugin needs to help her reason, not just produce.

**What the plugin reads:**
```
Accounts/*/
├── dashboard.json          → Which accounts came through developer channels vs enterprise sales
├── intelligence.json       → Competitive signals, win/loss patterns

Projects/Q2-Launch/
├── dashboard.json          → Campaign status, timeline, deliverables
├── intelligence.json       → Positioning research, audience analysis

People/*/person.md          → Stakeholders who need to be aligned on this decision
_archive/2026-01/           → Last two campaign performance postmortems
_archive/2026-02/           → Recent meeting notes discussing audience shift
data/emails.json            → Thread with CMO about positioning concerns
```

**What it produces:**

Priya says "decide whether we should pivot" and the plugin runs a structured analysis — not a pros/cons list, but a rigorous problem decomposition:

```markdown
# Q2 Launch Positioning Decision
**Date:** February 17, 2026 | **Decision deadline:** February 20

## The Question

Should the Q2 product launch pivot from enterprise buyer positioning to
developer-first positioning?

## Problem Structure (SCQA)

**Situation:** The Q2 launch is 6 weeks out, positioned for enterprise
buyers — the company's historical strength and $18M of the $24M ARR base.

**Complication:** Three converging signals challenge the enterprise-first
assumption:
1. Developer-led signups are 60% of new activations (up from 35% in Q3)
2. Apex (primary competitor) announced developer-first rebrand on Feb 3,
   with $12M series B earmarked for developer community
3. Last two enterprise campaigns (Q4 webinar series, Jan thought leadership
   push) underperformed: 40% below target registration, 2.1% conversion
   vs 4.5% benchmark

**Question:** Given that the product launch window is fixed, should we
reposition for the developer audience — and if so, can we execute the
pivot in 6 weeks?

## Issue Tree

```
Should we pivot to developer-first?
├── Is the developer signal real or noise?
│   ├── Is 60% developer share durable? → YES. 3 consecutive quarters of growth
│   │   (35% → 47% → 60%). Not seasonal, not campaign-driven.
│   ├── Are developer-led accounts converting to paid? → PARTIALLY. 23% trial
│   │   conversion vs 31% for enterprise-led. But ACV is growing ($8K → $14K
│   │   over 6 months as teams expand).
│   └── Is the competitor move forcing our hand? → NO, but it raises the stakes.
│       Apex's rebrand doesn't change our product, but it will own developer
│       mindshare if we cede the narrative.
│
├── Can we execute the pivot in 6 weeks?
│   ├── Content readiness → TIGHT. 4 of 7 planned assets are enterprise-framed.
│   │   Developer positioning requires new messaging, new proof points, new CTAs.
│   │   Realistically: 3 strong developer assets + 2 repurposed, not 7.
│   ├── Channel readiness → MIXED. Dev community channels (blog, GitHub, Discord)
│   │   are underinvested. Strong on LinkedIn (enterprise), weak on dev channels.
│   └── Team capacity → STRETCHED. Two team members on the enterprise path would
│       need to shift. No net new headcount available.
│
└── What's the cost of getting it wrong?
    ├── If we pivot and developers don't convert at volume → Q2 pipeline miss,
    │   enterprise buyers confused by mixed messaging. Recovery: Q3 repositioning.
    ├── If we stay enterprise and developers go to Apex → Losing a growing segment
    │   to a well-funded competitor. Recovery: much harder, community is sticky.
    └── If we do both poorly → Diluted message to everyone. Worst outcome.
```

## Analysis

### The case FOR pivoting

The trajectory is clear: developer-led is where growth lives. The 60%
share isn't a blip — it's three quarters of acceleration. More importantly,
the developer-led accounts show an expansion pattern (individual → team →
department) that mirrors how the strongest enterprise accounts started.
Investing in developer positioning is investing in future enterprise revenue.

The competitive threat amplifies urgency. Apex's developer-first rebrand
will eat mindshare in a community that rewards first movers. Ceding this
ground now means fighting uphill later.

**Evidence:** Developer signups trend (Product dashboard), account expansion
data (Accounts/*/dashboard.json), Apex competitive brief (_archive/2026-02/
competitive-analysis.md).

### The case AGAINST pivoting

Six weeks is dangerously tight for a positioning pivot. The enterprise
assets are 60% complete. The developer channel infrastructure is
underbuilt. A half-committed pivot produces weaker materials for both
audiences — the worst outcome in the cost analysis.

Enterprise isn't failing — it's the campaign execution that underperformed,
not the audience. The Q4 webinar had a targeting problem (wrong persona
titles), and the Jan thought leadership was generic (postmortem notes:
"could apply to any industry"). Better enterprise execution might recover
performance without the risk of a pivot.

**Evidence:** Campaign postmortems (_archive/2026-01/), asset completion
tracker (Projects/Q2-Launch/dashboard.json), channel audit (Feb 12 meeting
with Growth team).

### Red team challenge

**"The developer signal is product-led growth, not marketing-driven. You
don't need to reposition the launch — you need to not break what's already
working."** This is the strongest counter-argument. Developers are finding
the product through documentation and word-of-mouth, not campaigns. A
marketing pivot might not accelerate what's already organic, and it risks
disrupting the enterprise engine that still drives 70% of revenue.

## Recommendation

**Don't pivot. Expand.**

The developer signal is real, but the 6-week window doesn't support a clean
repositioning. Instead:

### Dual-track launch (70/30 split)

1. **Enterprise track (70% of resources):** Execute the Q2 launch as planned
   but fix the targeting and specificity problems identified in the campaign
   postmortems. Better enterprise execution, not different enterprise messaging.

2. **Developer track (30% of resources):** Launch a parallel developer-focused
   content stream — 2-3 technical deep-dives, a getting-started guide, and a
   developer community presence (GitHub, Discord). Not a repositioned launch,
   but an explicit acknowledgment that developers are a first-class audience.

3. **Q3 decision point:** If the developer track outperforms the enterprise
   track (measure: pipeline generated per marketing dollar), the Q3 campaign
   goes developer-first with full investment. This gives you data instead of
   a bet.

### Why this is better than a pivot

- Preserves 6 weeks of enterprise prep work ($0 waste)
- Explicitly invests in developers without betting the quarter
- Creates a controlled experiment with measurable signal
- Gives the CMO a recommendation backed by evidence, not instinct
- Positions you to go harder in Q3 with data, not assumption

### Stakeholder alignment needed

| Who | Why | Meeting |
|-----|-----|---------|
| CMO (Rachel) | Approves the dual-track approach | Thursday decision meeting |
| Product lead (Kai) | Provides developer usage data for content | This week |
| Growth team | Channel strategy for developer track | Next week |
| Content team | Resource allocation for 70/30 split | After CMO approval |

## Actions

1. Prepare dual-track proposal for Thursday CMO meeting
2. Request developer funnel data from Kai by Wednesday
3. Draft developer content brief (3 pieces) for parallel stream
4. Schedule growth team channel strategy session for next week
5. Update Projects/Q2-Launch/dashboard.json with dual-track structure
```

**What loops back:**
- Decision analysis saved to `Projects/Q2-Launch/positioning-decision-2026-02.md`
- Five actions created and assigned
- Competitive intelligence on Apex updated in relevant account intelligence
- Stakeholder alignment needs tracked as meeting preparation items
- The 70/30 framework becomes a trackable project structure

**Why this is different from "write me a pros and cons list":**
The plugin didn't just weigh options — it decomposed the problem using an issue tree, tested assumptions against workspace evidence, ran a red team challenge, and produced a recommendation with a specific implementation path. Every assertion cites actual workspace data (developer signup trends, campaign postmortems, meeting notes). Priya can walk into Thursday's meeting with a decision brief that holds up to scrutiny because the analysis is rigorous, not because the formatting is nice.

This is the `/dailyos:decide` capability — structured analytical thinking grounded in accumulated workspace intelligence. Strategy consulting methodology, universalized for any role.

---

## Capability Patterns

From the six scenarios, nine patterns emerge. These aren't command categories — they're modes of work that the plugin must support fluently.

### Pattern 1: Assess

**What it is:** Evaluate the current state of an entity — risks, health, trajectory, position.

**Examples across roles:**
- CS: Risk report, health assessment, renewal readiness
- Sales: Deal review, pipeline health, competitive position
- Agency: Client satisfaction check, scope drift analysis
- Product: Feature adoption assessment, technical debt evaluation
- Leadership: Portfolio health, team capacity, strategic position

**What it reads:** Entity dashboard + intelligence + meeting history + action trail + people signals

**What it produces:** A structured assessment document with evidence-backed analysis, not opinions

**Proactive trigger:** Auto-surfaces when an entity's health changes, a renewal approaches, or signal patterns shift

### Pattern 2: Produce

**What it is:** Generate a specific deliverable — a document someone else will read.

**Examples across roles:**
- CS: QBR narrative, executive business review, success plan
- Sales: Business case, mutual action plan, competitive displacement strategy
- Agency: Client status update, creative brief, project proposal
- Product: PRD, roadmap narrative, feature spec
- Leadership: Board contribution, strategic memo, headcount justification

**What it reads:** Everything relevant to the entity + the deliverable format expectations from the role preset

**What it produces:** A polished, ready-to-use document in the right voice and format

**Proactive trigger:** Calendar shows a QBR/review/board meeting → suggests generating the narrative beforehand

### Pattern 3: Communicate

**What it is:** Draft a message to a specific person, grounded in shared context.

**Examples across roles:**
- CS: Executive outreach, renewal conversation opener, escalation brief
- Sales: Follow-up email, proposal cover letter, cold outreach based on signal
- Agency: Scope change negotiation, feedback response, project kickoff email
- Product: Stakeholder update, cross-functional request, launch announcement
- Leadership: Team communication, executive briefing email, delegation request

**What it reads:** The person's file + relationship history + recent meetings + the specific context driving the communication

**What it produces:** A draft message in the right tone, referencing actual shared history, with specific asks

**Proactive trigger:** Detects overdue action involving another person → suggests a follow-up message

### Pattern 4: Plan

**What it is:** Create a forward-looking plan for an entity — what to do, when, and why.

**Examples across roles:**
- CS: Success plan, renewal strategy, expansion playbook, escalation response plan
- Sales: Deal strategy, territory plan, pipeline acceleration plan
- Agency: Project timeline revision, resource allocation plan, account growth strategy
- Product: Sprint plan, launch plan, migration strategy
- Leadership: Quarterly plan, hiring plan, strategic initiative roadmap

**What it reads:** Entity intelligence + historical trajectory + action trail + people map + the current situation

**What it produces:** A time-bound plan with specific actions, owners, and milestones — connected to the action system

**Proactive trigger:** Renewal within 90 days without a plan → suggests creating one. Health declines → suggests response plan.

### Pattern 5: Synthesize

**What it is:** Find patterns across multiple entities, time periods, or conversations.

**Examples across roles:**
- CS: Portfolio trends, churn pattern analysis, expansion commonalities
- Sales: Win/loss patterns, pipeline velocity analysis, competitive landscape
- Agency: Cross-client capability development, utilization patterns
- Product: User feedback synthesis, feature request clustering, adoption patterns
- Leadership: Organization-wide themes, strategic signal aggregation, quarterly narrative

**What it reads:** Multiple entities, archive spans, cross-cutting signals

**What it produces:** A synthesis document that elevates individual signals into strategic patterns — the "5 of 8 expansions follow the same pattern" insight that no single-entity analysis would surface

**Proactive trigger:** Weekly/monthly cadence → auto-generates synthesis at role-appropriate intervals

### Pattern 6: Capture

**What it is:** Process raw input into the workspace in the right format and location.

**Examples across roles:**
- All roles: Transcript → meeting summary + actions + people updates + entity intelligence
- All roles: Research notes → entity enrichment
- All roles: Email thread → action extraction + signal logging
- All roles: External document → workspace-native reference

**What it reads:** The raw input + workspace schema knowledge to know where things go

**What it produces:** Properly formatted workspace artifacts in the right directories

**Loop-back:** This IS the loop-back. Capture is the mechanism by which the workspace gets smarter.

**Proactive trigger:** New file detected in `_inbox/` → auto-processes. Transcript from today's meeting detected → processes immediately.

### Pattern 7: Enrich

**What it is:** Deepen intelligence on an entity beyond what the workspace currently knows.

**Examples across roles:**
- CS: Research a customer's recent earnings, strategic priorities, org changes
- Sales: Prospect research, competitive intelligence, market analysis
- Agency: Client industry trends, competitive creative work, audience research
- Product: Technology landscape, user behavior research, market sizing
- Leadership: Industry trends, competitive positioning, market intelligence

**What it reads:** Current workspace intelligence + web sources + any connected data sources (via MCP)

**What it produces:** Updated intelligence artifacts — enriched entity files, new stakeholder profiles, competitive briefs

**Proactive trigger:** Before a high-stakes meeting, auto-enriches key attendees and the entity. Detects stale intelligence → suggests refresh.

### Pattern 8: Decide

**What it is:** Structured analytical thinking for decisions with ambiguity — the opposite of "just give me a pros/cons list."

This pattern comes from management consulting methodology (SCQA → Issue Trees → Hypothesis Testing → Red Team → Recommendation) but universalized for any role. The user doesn't need to know the framework names. They say "should we..." and the plugin runs the analytical engine.

**The workflow:**
1. **Frame the problem** — Situation, Complication, Question, Answer hypothesis (SCQA)
2. **Decompose** — Issue tree that breaks the question into MECE (mutually exclusive, collectively exhaustive) sub-questions
3. **Test** — For each branch: What Would Have To Be True for this to be right? What evidence supports or contradicts?
4. **Challenge** — Red team the emerging answer. What's the strongest argument against?
5. **Recommend** — Specific, actionable recommendation with implementation path

**Examples across roles:**
- CS: "Should we invest executive time in saving this account?" → Risk/value analysis with specific intervention plan
- Sales: "Should we pursue this enterprise deal or focus on mid-market?" → Market analysis grounded in pipeline data
- Marketing: "Should we pivot the Q2 campaign to developers?" → Audience analysis with controlled experiment design (see Priya scenario)
- Agency: "Should we take on this scope change or push back?" → Scope/timeline/relationship impact analysis
- Product: "Should we build this feature or buy/integrate?" → Build vs. buy with evidence from customer conversations
- Leadership: "Should we restructure the team?" → Capacity analysis grounded in portfolio data and meeting frequency signals

**What it reads:** Everything relevant to the decision — entity intelligence, historical meeting notes, action trails, stakeholder positions, competitive signals. The issue tree drives which workspace files are relevant.

**What it produces:** A decision brief with structured problem decomposition, evidence-backed analysis, red team challenge, and specific recommendation. Not a document about the decision — a tool FOR deciding.

**Eight analytical frameworks available** (selected automatically based on question type):
| Question Type | Framework | When |
|--------------|-----------|------|
| "Should we..." | SCQA + Issue Tree | Complex decisions with multiple variables |
| "Why is this happening?" | Issue Tree (diagnostic) | Root cause analysis |
| "What's really going on?" | What Would Have To Be True | Testing assumptions |
| "What are our options?" | 2×2 Matrix | Comparing alternatives on two dimensions |
| "How big is this?" | Fermi Estimation | Sizing an opportunity or risk without perfect data |
| "What's the competitive landscape?" | Porter's Five Forces / 3Cs | Market and competitive analysis |
| "Where should we focus?" | 80/20 Analysis | Finding the vital few in the trivial many |
| "Is this a good idea?" | SCQA + Red Team | Pre-mortem and challenger analysis |

**Quality gates:**
- Problem definition must be specific (not "should we grow" but "should we expand into the developer segment with a $200K Q2 investment")
- Every issue tree branch must be testable against workspace evidence
- Red team challenge must present the strongest counter-argument, not a strawman
- Recommendation must include specific next steps with owners

**Proactive trigger:** When the user asks a question that starts with "should we," "what if," "is it worth," or similar decision-framing language, the decide capability activates. Also triggers when an entity's intelligence contains conflicting signals that suggest an unresolved decision.

### Pattern 9: Navigate

**What it is:** Political intelligence and relationship navigation — understanding what's really happening in a relationship and how to move through it effectively.

This goes deeper than the `relationship-context` skill (Pattern 5 territory — "who is this person and what's our history"). Navigate is about dynamics: power structures, influence chains, unspoken tensions, strategic positioning. It's the difference between knowing someone's title and understanding their motivations.

**Five capabilities:**

**1. Pre-Conversation Prep**
Before a sensitive conversation, Navigate loads the full relationship context and produces a tactical brief:
- What does this person care about right now? (from meeting history, email signals)
- What's the power dynamic? (their position, your position, what each side needs)
- What's the subtext? (reading between the lines of recent interactions)
- What to say, what not to say, and why
- Likely objections and how to address them

**2. Communication Review**
Before sending a high-stakes message, Navigate reviews for political implications:
- Does the tone match the relationship? (you can be direct with a long-term champion, not with a new executive)
- Are there hidden commitments in the language? ("We'll explore that" reads as a promise)
- Is the audience right? (who should be CC'd, who shouldn't, and why)
- What power dynamics does this message create or shift?

**3. Situation Analysis**
When something feels off in a relationship or account, Navigate maps the landscape:
- Who has influence over the outcome?
- What are the competing interests?
- Where are the alliances and tensions?
- What's the path from here to the outcome you need?

**4. Post-Meeting Debrief**
After a significant conversation, Navigate helps process what happened:
- What was said vs. what was meant?
- Did the dynamic shift? In whose favor?
- What commitments were made (explicit and implicit)?
- What needs to happen next to maintain or shift momentum?

**5. Stakeholder Strategy**
For multi-stakeholder environments (enterprises, agencies, partnerships), Navigate builds an influence map:
- Who are the decision-makers, influencers, blockers, and champions?
- What motivates each stakeholder?
- What's the engagement strategy for each?
- Where should you invest relationship capital?

**Examples across roles:**
- CS: Champion is transitioning roles → Navigate maps the influence gap and produces a specific plan for building the successor relationship
- Sales: Deal involves 5 stakeholders with competing interests → Navigate maps the dynamics and identifies the path to consensus
- Agency: Client's marketing director has concerns she hasn't voiced directly → Navigate reads the signals from recent meetings and helps prepare for the conversation
- Product: Engineering lead is blocking your feature prioritization → Navigate analyzes the competing interests and suggests an approach
- Leadership: Board member raised a pointed question last meeting → Navigate helps decode the subtext and prepare the response

**What it reads:** Person profiles + meeting history (especially tone and topics) + email signals + entity stakeholder maps + action trail (who followed through, who didn't) + the specific situation context

**What it produces:** Confidential tactical briefs. Navigate output is NEVER shared externally — it's your private playbook for understanding and moving through professional relationships. Every output is marked as internal-only.

**Critical constraint:** Navigate is direct. No corporate euphemisms. "David hasn't attended a meeting in 4 months — he's either disengaged or actively avoiding the conversation. Either way, you need to force the interaction" is more useful than "Consider re-engaging David." Honest assessment of power dynamics and motivations, not diplomacy.

**Proactive trigger:** Before any meeting with a person whose temperature is "cooling" or an entity whose health is declining, Navigate can surface a tactical brief. Detects when a stakeholder relationship shows warning signs (frequency drop, shorter meetings, fewer responses) and suggests a situation analysis.

---

## Plugin Architecture

### Why Two Plugins, Not Six

The philosophy says "opinionated defaults." Six shallow plugins means six installation decisions, six things to explain, six surfaces to maintain. The user asked for depth over breadth. So:

**Plugin 1: `dailyos`** — The workspace intelligence plugin. Every DailyOS user installs this. It handles all seven capability patterns. It's the "one plugin to rule them all" for workspace-native work.

**Plugin 2: `dailyos-writer`** — The editorial production plugin. For users who need the full writer's room workflow (7-phase, multi-agent review, voice profiles, templates). This is a distinct mode of deep work that warrants its own plugin — it's already a rich skill with 7 agents and 30+ templates.

**Why separate the writer?** Because producing a risk report (Pattern 2: Produce) and producing a thought leadership article (writer) are fundamentally different work modes. A risk report takes 2 minutes and synthesizes existing workspace intelligence. A thought leadership article takes 2 hours and involves ideation, research, multiple review passes, and editorial judgment. Bundling them would make the core plugin unwieldy and confuse the interaction model.

### Plugin 1: `dailyos`

```
dailyos/
├── .claude-plugin/
│   └── plugin.json
├── .mcp.json                          # Quill MCP sidecar connector
├── CONNECTORS.md
├── README.md
├── commands/
│   ├── start.md                       # Initialize workspace fluency
│   ├── assess.md                      # Risk reports, health checks, deal reviews
│   ├── produce.md                     # Status updates, QBR narratives, board decks
│   ├── compose.md                     # Emails, messages, communications
│   ├── plan.md                        # Success plans, strategies, playbooks
│   ├── synthesize.md                  # Cross-entity patterns, portfolio views
│   ├── capture.md                     # Process inputs into workspace
│   ├── enrich.md                      # Deepen entity intelligence
│   ├── decide.md                      # Structured analytical decisions (SCQA, Issue Trees)
│   └── navigate.md                    # Relationship navigation, political intelligence
└── skills/
    ├── workspace-fluency/
    │   └── SKILL.md                   # Workspace schema, file layout, conventions
    ├── entity-intelligence/
    │   └── SKILL.md                   # Auto-loads entity context when referenced
    ├── meeting-intelligence/
    │   └── SKILL.md                   # Meeting prep enhancement, calendar awareness
    ├── action-awareness/
    │   └── SKILL.md                   # Commitment tracking, overdue surfacing
    ├── relationship-context/
    │   └── SKILL.md                   # Person/stakeholder intelligence auto-loading
    ├── political-intelligence/
    │   └── SKILL.md                   # Influence dynamics, power structures, subtext
    ├── analytical-frameworks/
    │   └── SKILL.md                   # SCQA, Issue Trees, WWHTBT, Red Team, Fermi
    ├── role-vocabulary/
    │   └── SKILL.md                   # Preset-aware vocabulary shaping
    └── loop-back/
        └── SKILL.md                   # Write-back conventions, workspace enrichment
```

#### Commands — Deep Design

Each command implements one of the seven capability patterns. The command's SKILL.md teaches Claude Code *how to do this specific type of work using the DailyOS workspace*. Not a thin wrapper — a deep workflow.

**`/dailyos:assess {entity}`** — The Assessment Engine

What it does when you say "risk report on Nielsen":
1. Reads the entity's full workspace context (dashboard, intelligence, people, history)
2. Identifies the assessment frame from the role preset vocabulary (CS: health/renewal frame, Sales: deal/pipeline frame, Agency: satisfaction/scope frame)
3. Generates a structured assessment with evidence-backed claims
4. Every assertion is sourced to a specific meeting, signal, or data point
5. Produces recommended actions connected to the assessment
6. Offers to write back: save the report, create actions, update intelligence

Assessment types per role preset:
- **CS:** Risk report, renewal readiness, health trajectory, champion stability
- **Sales:** Deal review, competitive position, pipeline health, forecast confidence
- **Marketing:** Campaign performance, audience engagement, brand health, content pipeline readiness
- **Partnerships:** Mutual value balance, joint opportunity pipeline, integration health
- **Agency:** Client satisfaction, scope drift analysis, budget tracking, relationship pulse
- **Consulting:** Engagement health, deliverable confidence, stakeholder alignment
- **Product:** Feature adoption, technical debt, user satisfaction synthesis
- **Leadership:** Portfolio health, team capacity, strategic position, risk concentration
- **The Desk:** Flexible assessment of any entity against user-defined criteria

**`/dailyos:produce {type} [entity]`** — The Deliverables Engine

What it does when you say "build my board section on customer health":
1. Identifies the deliverable type and its structure (board contribution, QBR narrative, status update, success plan)
2. Reads ALL relevant workspace context — scope depends on deliverable type (single entity for a QBR narrative, full portfolio for a board deck)
3. Generates in the appropriate voice and format — not just content, but structure that matches how these documents actually look in the real world
4. Includes quantitative evidence where available, qualitative narrative where appropriate
5. Flags data gaps explicitly ("Historical comparison unavailable — current state shown")
6. Ready to copy-paste into the final format (Docs, Slides, whatever)

Deliverable types per role preset:
- **CS:** QBR narrative, executive business review, success plan, value summary, expansion case, renewal brief
- **Sales:** Business case, proposal narrative, competitive displacement strategy, mutual action plan, pipeline review
- **Marketing:** Campaign brief, content strategy, competitive positioning doc, launch plan, audience analysis, performance narrative
- **Partnerships:** Joint business review, partner scorecard, co-marketing proposal
- **Agency:** Client status update, creative brief, project proposal, scope change documentation, case study
- **Consulting:** Engagement summary, deliverable brief, recommendations document
- **Product:** PRD, feature spec, roadmap narrative, launch plan, user research synthesis
- **Leadership:** Board contribution, strategic memo, headcount justification, quarterly narrative, team update
- **The Desk:** Flexible document generation with user-specified structure

**`/dailyos:compose [recipient] [about]`** — The Communications Engine

What it does when you say "write a follow-up to David at TechCorp":
1. Identifies the recipient from People/ directory (with domain alias resolution)
2. Loads full relationship context — meeting history, communication patterns, open items
3. Determines communication type from context (follow-up, outreach, escalation, update)
4. Drafts in the appropriate tone — formal for executives, collaborative for peers, specific for technical stakeholders
5. References actual shared history — not "as we discussed" but "the integration timeline you asked about on Tuesday"
6. Includes specific commitments and next steps, trackable as actions

**`/dailyos:plan {entity}`** — The Strategy Engine

What it does when you say "build a success plan for Nielsen":
1. Reads the entity's full intelligence — current state, trajectory, risks, wins
2. Reads the action trail — what's been promised, what's been delivered, what's overdue
3. Identifies the planning frame from the role preset (renewal plan for CS, deal strategy for Sales, project timeline for Agency)
4. Generates a time-bound plan with milestones, actions, owners
5. Connects the plan to reality — "Based on the last 3 meetings, the champion responds well to data-driven presentations. The March EBR should lead with metrics."
6. Actions from the plan become trackable items in the workspace

**`/dailyos:synthesize [scope] [timeframe]`** — The Pattern Engine

What it does when you say "what patterns do we see across our enterprise accounts this quarter":
1. Reads across multiple entities — configurable scope (all accounts, a segment, a portfolio)
2. Identifies cross-cutting patterns (the "5 of 8 expansions follow the same pattern" insight)
3. Generates a synthesis that elevates individual signals into strategic observations
4. Quantifies where possible, qualifies where appropriate
5. Produces actionable strategic recommendations, not just observations

**`/dailyos:capture [input]`** — The Intake Engine

What it does when you say "process this transcript":
1. Identifies input type (transcript, notes, email thread, research, document)
2. Routes to the right workspace location based on type and content
3. Extracts structured data — actions, people mentions, entity references, signals
4. Updates relevant workspace artifacts — meeting summaries, people intelligence, entity intelligence
5. Generates a capture report: "Processed: 4 actions created, 2 people updated, entity intelligence refreshed for Nielsen and Acme"

**`/dailyos:enrich {entity}`** — The Research Engine

What it does when you say "go deeper on Nielsen before the QBR":
1. Reads current workspace intelligence for the entity
2. Identifies intelligence gaps — stale data, missing stakeholders, outdated competitive info
3. Conducts web research — recent news, earnings, executive changes, strategic priorities
4. Updates workspace artifacts with enriched intelligence
5. Generates an enrichment report: "Updated: 2 new stakeholder profiles, recent earnings summary, competitive landscape refreshed"

**`/dailyos:decide {question}`** — The Analytical Engine

What it does when you say "should we pivot the Q2 campaign to developers":
1. **Frames the problem** using SCQA (Situation, Complication, Question, Answer hypothesis). Reads workspace context to populate each element with real data — not generic framing, but grounding in what the workspace actually knows.
2. **Decomposes the question** into an issue tree. Each branch represents a sub-question that must be answered to resolve the main question. Branches are MECE — mutually exclusive (no overlap) and collectively exhaustive (nothing missing). The issue tree drives which workspace files get read — branches about competitive position pull from intelligence.json, branches about team capacity pull from meeting signals and action trails.
3. **Tests each branch** using WWHTBT (What Would Have To Be True). For each possible answer, states the conditions that would need to hold — then checks workspace evidence for or against. This prevents confirmation bias: instead of "is this true?" the question becomes "what would the world look like if this were true, and does our evidence match?"
4. **Challenges the emerging answer** with a red team pass. Takes the strongest counter-argument seriously. Not a strawman — the genuine best case for the alternative. Uses the partner-critic mental model: "If I were advising the other side, what would I say?"
5. **Produces a recommendation** with specific implementation path, stakeholder alignment needs, and created actions. Not "we should probably consider..." but "do X by date Y with owner Z."

The eight analytical frameworks are selected automatically based on question type:
- **SCQA + Issue Tree** — Default for complex decisions ("should we...")
- **Diagnostic Issue Tree** — Root cause analysis ("why is this happening...")
- **WWHTBT** — Assumption testing ("is this really true...")
- **2×2 Matrix** — Comparing alternatives on two dimensions ("which of these options...")
- **Fermi Estimation** — Sizing without perfect data ("how big is this opportunity...")
- **Porter's Five Forces / 3Cs** — Competitive and market analysis ("what's the landscape...")
- **80/20 Analysis** — Finding leverage points ("where should we focus...")
- **Pre-Mortem + Red Team** — Stress-testing a plan ("what could go wrong...")

Quality gates at each phase:
- Problem definition must be specific enough to test (not "should we grow" → "should we invest $200K in developer marketing in Q2")
- Issue tree must be MECE (mutual exclusivity test: "can two branches be true at the same time?" + exhaustion test: "is there a scenario not covered?")
- Evidence must be workspace-grounded where available, explicitly flagged as external/assumed where not
- Red team must present the strongest counter-argument, not a strawman
- Recommendation must include specific actions with owners and dates

Decision types per role preset:
- **CS:** Save/invest decision, renewal strategy, resource allocation, escalation decision
- **Sales:** Pursue/pass decision, deal strategy, territory prioritization, competitive response
- **Marketing:** Campaign pivot, audience strategy, channel investment, positioning decision
- **Partnerships:** Joint investment decision, partner selection, co-marketing allocation
- **Agency:** Scope change response, pricing strategy, staffing decision, client retention calculus
- **Consulting:** Engagement scope, methodology selection, recommendation framing
- **Product:** Build/buy/integrate, feature prioritization, technical debt investment, launch timing
- **Leadership:** Team restructuring, strategic initiative selection, budget allocation, hiring decision
- **The Desk:** Any structured decision with user-provided context

**`/dailyos:navigate {person|entity|situation}`** — The Political Intelligence Engine

What it does when you say "I need to handle the David Park situation before the renewal":
1. **Loads the full relationship context** — not just the person file, but the entire relationship arc: every meeting they've attended, meeting frequency trends, email signal patterns, the entity context they exist within, other stakeholders in the same account, and the specific situation driving the request.
2. **Maps the dynamics** — Who has influence? What are the competing interests? Where does this person sit in the decision-making structure? What motivates them (from observed behavior in meetings, not assumptions)?
3. **Reads the subtext** — Meeting frequency drops, shorter responses, topic avoidance, delegation patterns. These are signals the workspace has been accumulating. Navigate surfaces what the data implies about where the relationship actually stands vs. where it appears to stand.
4. **Produces a tactical brief** that is direct, honest, and actionable. No corporate euphemism. "David hasn't been in a meeting in 4 months. He's either disengaged or actively avoiding the conversation. You need to force the interaction — here's how." Navigate tells you what's actually happening and what to do about it.
5. **Marks everything as internal-only.** Navigate output is never shared externally. It's your private playbook.

Five capability modes (selected by context):

**Pre-conversation prep** — Before a sensitive meeting or call:
- What does this person care about right now?
- What's the power dynamic between you?
- What to say, what NOT to say, and why
- Likely objections with specific responses
- The one thing that would change the trajectory of this conversation

**Communication review** — Before sending a high-stakes message:
- Does the tone match the relationship depth?
- Are there hidden commitments in the language?
- Who should see this? Who should NOT?
- What power dynamics does this create?
- Rewrite suggestions where tone is wrong

**Situation analysis** — When something feels off:
- Influence map: decision-makers, influencers, blockers, champions
- Competing interests and where they collide
- The path from here to the outcome you need
- Where to invest and where to conserve relationship capital

**Post-meeting debrief** — After a significant conversation:
- What was said vs. what was meant
- Did the dynamic shift? In whose favor?
- Commitments made (explicit AND implicit)
- What needs to happen in the next 48 hours

**Stakeholder strategy** — For multi-stakeholder environments:
- Full influence map with motivations for each player
- Engagement strategy per stakeholder
- Sequence: who to talk to first and why
- Coalition building: which stakeholders reinforce each other

Navigate types per role preset:
- **CS:** Champion transition navigation, executive re-engagement, multi-stakeholder renewal strategy
- **Sales:** Multi-threaded deal navigation, competitive displacement dynamics, procurement process reading
- **Marketing:** Cross-functional alignment (sales, product, leadership), agency relationship management
- **Partnerships:** Joint venture politics, co-selling dynamics, partner executive engagement
- **Agency:** Client personality management, scope negotiation dynamics, internal stakeholder alignment
- **Consulting:** Client politics navigation, steering committee dynamics, recommendation delivery strategy
- **Product:** Engineering relationship building, cross-functional prioritization politics, executive buy-in
- **Leadership:** Board dynamics, team restructuring communication, organizational change navigation
- **The Desk:** Any relationship situation with user-provided context

**Critical design constraint:** Navigate is the most sensitive capability. Its output is always marked `<!-- internal-only -->`. It never uses corporate euphemisms. It reads real signals from real data. It tells you what you need to hear, not what sounds nice. This is where the accumulated operational memory — 6 months of meeting signals, email patterns, engagement trends — becomes genuinely powerful.

#### Skills — Auto-Activating Intelligence

These fire automatically when context is relevant. They don't wait for commands.

**`workspace-fluency`** — Always active in a DailyOS workspace. Teaches Claude Code:
- Workspace directory structure (`Accounts/`, `Projects/`, `People/`, `_archive/`, `_inbox/`, `data/`)
- File format conventions (dashboard.json, intelligence.json, person.json, person.md)
- DailyOS schemas (schedule, prep, actions, emails, manifest)
- The role preset system and how vocabulary shapes output
- The loop-back convention: deliverables should be offered for workspace archival

**`entity-intelligence`** — Fires when any entity name is mentioned in conversation. Silently reads the entity's workspace directory and loads context. The user says "I'm thinking about the Nielsen situation" and Claude already has Nielsen's full context loaded — without being asked.

**`meeting-intelligence`** — Fires when meeting prep or scheduling context appears. Knows the meeting template system (customer-call, QBR, partnership, 1:1, all-hands). Can generate deep prep beyond what the app auto-generates — competitive research, scenario planning, question frameworks.

**`action-awareness`** — Fires when commitments, tasks, or follow-ups are discussed. Knows the action schema. Surfaces relevant overdue items. Tracks new commitments. Connects actions to entities and people.

**`relationship-context`** — Fires when a person is mentioned. Loads their full People/ profile — meeting signals, temperature, organization, role, relationship history. Makes every conversation about a person informed by the full relationship arc.

**`political-intelligence`** — Fires alongside relationship-context but goes deeper. When a person or entity is mentioned in a context that involves dynamics, tension, or navigation (detected via language cues like "handle," "approach," "deal with," "convince," "situation"), this skill activates the navigate capability's analytical lens. It reads stakeholder maps, engagement frequency patterns, meeting attendance gaps, and communication signals to provide a political read of the landscape. Works invisibly — enriches compose, assess, and plan commands with awareness of who has influence and what motivates them.

**`analytical-frameworks`** — Fires when the user asks a question that implies structured analysis: "should we," "why is this," "what if," "how big is," "where should we focus." Loads the eight-framework toolkit (SCQA, Issue Trees, WWHTBT, 2×2 Matrix, Fermi Estimation, Porter's Five Forces, 3Cs, 80/20) and selects the appropriate framework based on question type. Also enriches other commands — when `/dailyos:assess` encounters conflicting signals, the analytical-frameworks skill can decompose the conflict into testable sub-questions.

**`role-vocabulary`** — Fires based on the active role preset. Shapes how Claude interprets and generates content. A CS user asking about "risk" gets health/renewal framing. A Sales user gets deal/pipeline framing. Same capability, different language.

**`loop-back`** — Fires when Claude produces a deliverable. Suggests the right workspace location, offers to create actions from recommendations, proposes intelligence updates from new insights. Closes the loop so the workspace gets smarter.

### Plugin 2: `dailyos-writer`

Already exists as a skill (`.claude/skills/writer/`). The marketplace plugin repackages it with two key enhancements:

1. **Workspace-first evidence:** When the writer's research phase runs, it reads the DailyOS workspace first — before going to the web. Customer quotes come from actual meeting transcripts. Data points come from actual dashboards. Evidence comes from lived experience, not Google searches.

2. **Agents become skills.** The existing writer uses 7 agents (challenger, research, mechanical-review, structural-review, voice-review, authenticity-review, scrutiny). In the plugin format, these become auto-activating skills — domain knowledge that enriches the workflow phases without requiring separate agent processes. Each skill carries the full depth of its agent counterpart.

```
dailyos-writer/
├── .claude-plugin/
│   └── plugin.json
├── README.md
├── commands/
│   ├── write.md                       # Start a new writing project
│   ├── challenge.md                   # Run challenger on a draft
│   ├── review.md                      # Full review cycle (triggers all review skills)
│   └── mechanical.md                  # Mechanical checks only
└── skills/
    ├── writer-core/
    │   └── SKILL.md                   # Full 7-phase workflow orchestration
    ├── challenger/
    │   └── SKILL.md                   # Red-team gate: "Should this even be written?"
    │                                  # Premise, claim, value, and framework challenges
    │                                  # Compression test: one sentence, what does the reader learn?
    │                                  # Verdicts: PROCEED / SHARPEN / RECONSIDER / KILL
    │                                  # "Use KILL more often than feels comfortable"
    ├── research/
    │   └── SKILL.md                   # Evidence gathering by content type
    │                                  # Internal search (workspace first), external search
    │                                  # Evidence quality standards with citation requirements
    │                                  # Output: structured evidence inventory with gaps
    │                                  # DailyOS enhancement: reads entity intelligence,
    │                                  # meeting archives, stakeholder quotes BEFORE web
    ├── scrutiny/
    │   └── SKILL.md                   # Executive specificity review
    │                                  # Catches: capability vagueness, timeline gaps,
    │                                  # unquantified impact, missing proof points,
    │                                  # resource hand-waving, ownership ambiguity
    │                                  # Severity: CRITICAL (block) / HIGH / MEDIUM
    ├── mechanical-review/
    │   └── SKILL.md                   # Typography, terminology, anti-pattern detection
    │                                  # Em dashes, Oxford commas, product name styling
    │                                  # Contrast framing, negative parallels, AI tropes
    │                                  # Template artifact detection (generic headers)
    │                                  # Auto-fixes where possible, flags for human decision
    ├── structural-review/
    │   └── SKILL.md                   # Logic, flow, evidence integration, argument coherence
    │                                  # Opening: attention-earning + thesis clarity
    │                                  # Section-by-section purpose and advancement check
    │                                  # Redundancy detection (two sections saying same thing)
    │                                  # Evidence-claim pairing audit
    │                                  # Conclusion: delivers on opening promise?
    ├── voice-review/
    │   └── SKILL.md                   # Voice fidelity against content type profiles
    │                                  # Evaluates tone, authority level, pronoun usage
    │                                  # Anti-pattern detection per voice profile
    │                                  # Fidelity scoring: Strong / Adequate / Weak
    ├── authenticity-review/
    │   └── SKILL.md                   # AI-tell and formulaic pattern detection
    │                                  # Structure tells: rigid paragraphs, predictable rhythm
    │                                  # Language tells: transition stuffing, hedge stacking
    │                                  # Burstiness analysis (natural rhythm variation)
    │                                  # Formula detection: template-as-prison, framework overuse
    │                                  # The Human Test: "Would James actually write it this way?"
    ├── voices/
    │   ├── strategic.yaml
    │   ├── thought-leadership.yaml
    │   ├── narrative.yaml
    │   ├── status-report.yaml
    │   └── customer.yaml
    ├── templates/                     # 30+ templates (existing)
    │   ├── thought-leadership/
    │   ├── strategic/
    │   ├── status-reports/
    │   ├── vision/
    │   ├── narrative/
    │   ├── podcast/
    │   └── customer/
    └── shared/
        ├── MECHANICS.md
        ├── TERMINOLOGY.md
        ├── ANTI-PATTERNS.md
        └── DISTRIBUTION.md
```

**How the review workflow uses skills:**

When `/dailyos-writer:review` runs, it triggers the review skills in sequence — each one auto-activates based on the phase:

```
Phase 1: mechanical-review  → catches the easy stuff so humans focus on substance
Phase 2: structural-review  → ensures argument flows and evidence supports claims
Phase 3: voice-review       → checks voice fidelity against the content type profile
Phase 4: authenticity-review → catches AI-tells and formulaic patterns
Phase 5: scrutiny           → executive specificity — vagueness kills credibility
Phase 6: challenger         → final gate: is this worth publishing?
```

Each skill carries the full depth of its original agent — the same question banks, the same evaluation frameworks, the same severity levels. The difference is packaging: skills activate within the conversation context rather than spawning separate processes, and they can read the DailyOS workspace for evidence grounding.

---

## Proactive Patterns

The philosophy says "the system operates, you leverage." Proactive patterns are where the plugin anticipates needs before the user articulates them.

### 1. Workspace Awareness on Entry

When a user opens Claude Code in a DailyOS workspace directory, the workspace-fluency skill activates. It reads today's briefing data and silently loads context. The user doesn't have to say "I'm working with DailyOS." The plugin just knows.

If there are high-priority items (a QBR today, overdue actions on an at-risk account, a renewal this week), the plugin can surface them: "You have a QBR with Nielsen in 3 hours. Their health declined to Yellow and your champion is transitioning. Would you like me to draft talking points for the executive conversation?"

### 2. Entity Auto-Loading

When the user mentions any entity name in conversation — even casually — the entity-intelligence skill fires and loads that entity's full context. No "first, let me look that up." The context is just... there.

### 3. Meeting-Driven Prep Amplification

The app generates standard prep. The plugin can go deeper on demand or proactively for high-stakes meetings. A QBR triggers automatic deep prep. A meeting with a new executive triggers automatic stakeholder research. A meeting at an at-risk account triggers automatic risk assessment surfacing.

### 4. Action Trail Accountability

When a user is working on anything related to an entity, the action-awareness skill surfaces relevant overdue items. Not as a guilt mechanism (P1: Zero-Guilt), but as context: "Note: you have 2 overdue items for Nielsen that might come up in the conversation."

### 5. Post-Meeting Processing

After a meeting appears on the calendar and a transcript or notes appear in `_inbox/`, the plugin can proactively offer to capture: "I see notes from your Nielsen call. Want me to process them? I'll extract actions, update stakeholder intelligence, and archive the summary."

### 6. Periodic Synthesis

For leadership roles, the plugin can suggest weekly or monthly synthesis at appropriate intervals: "It's Friday. Want me to synthesize your portfolio signals from this week? I see 3 health changes, 2 expansion signals, and 1 escalation resolution."

### 7. Intelligence Staleness Detection

The enrich skill can notice when entity intelligence is stale — no meeting in 30+ days, no signal updates, approaching a milestone. It surfaces this quietly: "Nielsen's intelligence was last updated 45 days ago and their renewal is in 60 days. Want me to run an enrichment pass?"

### 8. Decision Detection

When the user's language signals an unresolved decision — "I'm not sure whether," "we need to figure out," "should we," "I keep going back and forth on" — the analytical-frameworks skill suggests structured analysis. Not by jumping into a 5-page SCQA decomposition, but by offering: "This sounds like a structured decision. Want me to run a proper analysis? I can see workspace evidence on both sides."

### 9. Relationship Warning Signals

The political-intelligence skill monitors stakeholder engagement patterns passively. When it detects warning signals — a key contact's meeting frequency dropped from weekly to monthly, an executive sponsor hasn't attended in 90+ days, email response times lengthening, someone who used to bring their team now comes alone — it surfaces these as quiet navigational intelligence: "David Park hasn't been in a Nielsen meeting since October. His engagement pattern suggests disengagement, not just scheduling conflict. This may need a direct approach before the renewal conversation."

---

## Marketplace Structure

```
dailyos/claude-code-plugins/
├── .claude-plugin/
│   └── marketplace.json
├── dailyos/                           # Plugin 1: Workspace Intelligence
│   ├── .claude-plugin/plugin.json
│   ├── .mcp.json
│   ├── commands/
│   │   ├── start.md
│   │   ├── assess.md
│   │   ├── produce.md
│   │   ├── compose.md
│   │   ├── plan.md
│   │   ├── synthesize.md
│   │   ├── capture.md
│   │   ├── enrich.md
│   │   ├── decide.md
│   │   └── navigate.md
│   └── skills/
│       ├── workspace-fluency/SKILL.md
│       ├── entity-intelligence/SKILL.md
│       ├── meeting-intelligence/SKILL.md
│       ├── action-awareness/SKILL.md
│       ├── relationship-context/SKILL.md
│       ├── political-intelligence/SKILL.md
│       ├── analytical-frameworks/SKILL.md
│       ├── role-vocabulary/SKILL.md
│       └── loop-back/SKILL.md
├── dailyos-writer/                    # Plugin 2: Editorial Production
│   ├── .claude-plugin/plugin.json
│   ├── commands/
│   │   ├── write.md
│   │   ├── challenge.md
│   │   ├── review.md
│   │   └── mechanical.md
│   └── skills/
│       ├── writer-core/SKILL.md
│       ├── challenger/SKILL.md
│       ├── research/SKILL.md
│       ├── scrutiny/SKILL.md
│       ├── mechanical-review/SKILL.md
│       ├── structural-review/SKILL.md
│       ├── voice-review/SKILL.md
│       ├── authenticity-review/SKILL.md
│       ├── voices/
│       ├── templates/
│       └── shared/
└── README.md
```

**marketplace.json:**
```json
{
  "name": "dailyos-plugins",
  "owner": {
    "name": "DailyOS"
  },
  "plugins": [
    {
      "name": "dailyos",
      "source": "./dailyos",
      "description": "Workspace intelligence for Claude Code. Gives Claude full fluency in your DailyOS workspace — entities, people, meetings, actions, and intelligence. Ten commands: assess risks, produce deliverables, compose communications, plan strategies, synthesize patterns, capture inputs, enrich intelligence, make structured decisions, and navigate relationships. No startup tax — your workspace is the context."
    },
    {
      "name": "dailyos-writer",
      "source": "./dailyos-writer",
      "description": "Editorial production with writer's room quality control. Seven-phase workflow with specialized voices, internal review cycles, and challenger gates. Creates thought leadership, strategic documents, status reports, and customer communications from your DailyOS workspace intelligence."
    }
  ]
}
```

---

## The Quality Bar

Every command output must pass these tests:

1. **Could you send this?** Not "does it look okay" — could you actually send it to your VP, your client, your board? If not, it's not ready.

2. **Is every claim sourced?** No assertion without evidence from the workspace. "Health declined" must point to the actual signal that changed. "Champion gap" must reference the actual meeting where the transition was mentioned.

3. **Does it know things a generic AI couldn't?** If the output could have been produced by pasting the entity name into ChatGPT, the plugin has failed. The value is in the accumulated operational memory — the three-meeting arc, the overdue action from six weeks ago, the executive who went quiet.

4. **Does it loop back?** Every deliverable should offer to enrich the workspace. A risk report creates actions. A synthesis surfaces patterns. A capture enriches intelligence. The workspace gets smarter every time the loop completes.

5. **Would you feel guilty if you didn't use it?** If yes, redesign. (P1: Zero-Guilt)

---

## Marketing Role Preset

Marketing was not in the original 8 shipped presets (ADR-0079) but the plugin marketplace work makes the case for adding it. Marketers are the most natural users of the DailyOS Loop — they constantly produce deliverables (campaigns, briefs, content), they work across entities (brands, products, audiences), and their decisions benefit enormously from accumulated intelligence (what messaging worked, what audiences engaged, what competitive moves happened).

### Preset JSON

```jsonc
{
  "id": "marketing",
  "name": "Marketing",
  "description": "For Content Marketers, Brand Managers, Growth Leads, and Marketing Directors",
  "entityModeDefault": "both",
  "metadata": {
    "account": [
      { "key": "brand_health", "label": "Brand Health", "type": "select", "options": ["Strong", "Stable", "Declining"] },
      { "key": "audience_segment", "label": "Audience", "type": "text" },
      { "key": "campaign_status", "label": "Campaign Status", "type": "select", "options": ["Planning", "Active", "Complete", "Paused"] },
      { "key": "channel_mix", "label": "Channels", "type": "text" },
      { "key": "content_pipeline", "label": "Pipeline Items", "type": "number" },
      { "key": "next_launch", "label": "Next Launch", "type": "date" },
      { "key": "competitive_position", "label": "Competitive Position", "type": "text" },
      { "key": "budget_remaining", "label": "Budget Remaining", "type": "currency" }
    ],
    "project": [
      { "key": "campaign_type", "label": "Campaign Type", "type": "select", "options": ["Launch", "Brand", "Content", "Demand Gen", "Event", "Partner"] },
      { "key": "target_audience", "label": "Target Audience", "type": "text" },
      { "key": "kpi_primary", "label": "Primary KPI", "type": "text" },
      { "key": "launch_date", "label": "Launch Date", "type": "date" },
      { "key": "budget", "label": "Budget", "type": "currency" }
    ]
  },
  "vocabulary": {
    "entityNoun": "brand",
    "healthFrame": "brand health",
    "riskVocabulary": ["brand dilution", "audience fatigue", "content gap", "campaign underperformance", "message drift", "competitive displacement", "channel saturation"],
    "winVocabulary": ["audience growth", "engagement spike", "viral moment", "brand lift", "conversion improvement", "share of voice gain", "organic traction"],
    "urgencySignals": ["campaign launch approaching", "content deadline", "brand crisis", "competitive move", "trending topic window", "budget deadline"]
  },
  "vitals": ["brand_health", "campaign_status", "content_pipeline", "next_launch", "audience_segment", "competitive_position"],
  "lifecycleEvents": ["Campaign Launch", "Content Publish", "Brand Refresh", "Competitive Response", "Audience Shift", "Channel Expansion"],
  "prioritization": {
    "primary": "launch_proximity",
    "secondary": ["campaign_performance", "competitive_moves", "content_deadlines"]
  },
  "briefingEmphasis": "Campaign readiness, content pipeline health, competitive moves, audience signals, approaching launches and deadlines"
}
```

### Why Marketing Earns Its Own Preset

1. **Unique vocabulary.** Marketing talks about brands, audiences, campaigns, channels, positioning — fundamentally different from accounts/deals/renewals. A CS preset would produce "churn risk" where Marketing needs "audience fatigue." A Sales preset would produce "deal velocity" where Marketing needs "content pipeline health."

2. **Both entity modes.** Marketers track brands/accounts (entity mode: account) AND campaigns/launches (entity mode: project). A product launch is a project. A brand is an account. They need both.

3. **High plugin usage.** Marketing is the heaviest user of Produce, Decide, and the Writer plugin. They're constantly creating deliverables (campaign briefs, positioning docs, content strategies), making analytical decisions (audience targeting, channel allocation, competitive response), and writing (thought leadership, customer stories, internal narratives). The plugin is more valuable to them than almost any other role.

4. **DailyOS competitive differentiation.** Most AI tools help marketers write. DailyOS helps marketers think AND write — from accumulated intelligence about what's actually working, what competitors are actually doing, and what audiences actually care about. The workspace memory means the second campaign brief is better than the first because the system learned from the first campaign's results.

---

## Capability Cross-Reference

How the nine patterns and ten commands serve each role:

| Capability | CS | Sales | Marketing | Agency | Product | Leadership |
|-----------|-----|-------|-----------|--------|---------|------------|
| **Assess** | Health, renewal | Deal, pipeline | Campaign, brand | Client, scope | Feature, debt | Portfolio, team |
| **Produce** | QBR, success plan | Business case | Campaign brief, strategy | Status update, proposal | PRD, spec | Board deck, memo |
| **Compose** | Exec outreach | Follow-up, proposal | Launch email, partner ask | Client update | Stakeholder update | Team comms |
| **Plan** | Renewal strategy | Deal strategy | Campaign plan, content calendar | Project timeline | Sprint plan | Quarterly plan |
| **Synthesize** | Portfolio trends | Win/loss patterns | Audience insights, performance | Cross-client themes | Feature clustering | Org-wide themes |
| **Capture** | Meeting → actions | Call → CRM notes | Campaign results → learnings | Client feedback → intelligence | User research → insights | Team updates → patterns |
| **Enrich** | Customer research | Prospect research | Competitor creative, market trends | Client industry | Technology landscape | Industry trends |
| **Decide** | Save/invest | Pursue/pass | Pivot/hold, audience strategy | Scope change | Build/buy | Restructure |
| **Navigate** | Champion transition | Multi-thread deal | Cross-functional alignment | Client politics | Eng relationship | Board dynamics |

---

*This document represents plugin marketplace design for I276. The architecture follows the DailyOS philosophy: the system operates, you leverage. The plugin bridges operational memory (what DailyOS knows) to productive action (what you need to do).*
