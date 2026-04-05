# Hook vs. DailyOS + Glean: Product Gap Analysis

**Date:** 2026-02-28 (updated same day with segmentation analysis, Glean Slack findings, prioritization)
**Context:** Evaluating Hook (hook.co) as a potential Gainsight replacement. This assessment examines whether DailyOS — powered by Glean in enterprise mode — can fill the same gap without introducing a new tool into the stack.
**Status:** Glean dual-mode context shipped in v0.15.2. Glean is available as a production context source.

---

## 1. What Hook Is

Hook is an AI agent platform for Customer Success with two named agents and a conversational interface:

| Component | Function |
|---|---|
| **Echo** | Risk detection agent. Monitors product usage, conversations, support tickets, and engagement signals. Claims 90–93% churn prediction accuracy up to 180 days before renewal. |
| **Activator** | Onboarding automation agent. Identifies optimal activation paths per customer, generates playbooks, and can execute actions autonomously or queue them for review. |
| **Hook Chat** | Natural language portfolio queries. Role-aware (CSM sees their book, CRO sees portfolio trends). "Which accounts haven't had an exec meeting in 90 days?" |

**Core value proposition vs. Gainsight:** Gainsight is a platform you configure; Hook is an agent that acts. Gainsight gives you health scores and dashboards; Hook detects risk and executes playbooks. The pitch is "autonomous CS operations" — less manual work, faster response to churn signals.

**Data sources:** Product usage telemetry, meeting recordings/transcripts, support tickets, email, CRM records, feature adoption metrics.

**Outputs:** Risk scores with cited evidence, auto-generated playbooks, workflow triggers (Slack, email, CRM updates, task assignment), executive dashboards.

---

## 2. Before Features: What Problem Are We Actually Solving?

Feature comparisons are useful but secondary. The first question is: *why is the team reaching for Hook in the first place?*

### The Book Is Two Books

Our customer base splits into two fundamentally different operating motions:

| Segment | Volume | Revenue | Motion | What "Good" Looks Like |
|---|---|---|---|---|
| **Standard** (80% of accounts) | High | ~20% of revenue | Digital / scaled | Automation, playbooks, auto-nudges, low-touch consistency |
| **Key Accounts** (20% of accounts) | Low | ~80% of revenue | High-touch TAM/RM/CSM | Intelligence, preparation, judgment, relationship depth |

Hook is built for the Standard motion. Auto-detect risk, auto-fire playbook, auto-send email. That's genuinely useful when you have hundreds of accounts and 1 CSM per 50–80. Nobody's reading every signal manually at that scale.

But the VP of Key Accounts doesn't have that problem. Their team *knows* their accounts. They're in weekly calls. They have relationships. So why are we evaluating Hook?

### Four Hypotheses for What's Actually Broken

**1. Gainsight produced telemetry but CTAs were noise nobody acted on.**
Gainsight likely generated health scores and CTAs from product usage, but if the team learned to ignore them — if CTAs felt mechanical and disconnected from what was actually happening in the relationship — then the problem isn't "we need better automation." It's "the signals we had weren't trustworthy." Hook promises better signal quality (93% accuracy), but the underlying issue is *signal-to-noise ratio*, not tooling. If the team didn't trust Gainsight's output, they'll eventually stop trusting Hook's too.

**2. The VP can't see portfolio health without polling the team.**
The real ask might be: "Which of my key accounts are at risk right now and why?" — and the VP can't answer that without asking each TAM individually. Gainsight was supposed to provide that portfolio view, but either the data wasn't maintained or the scores didn't reflect reality. What the VP actually wants is a reliable, low-maintenance portfolio health view that updates itself.

**3. Operational consistency varies across the team.**
Different TAMs/CSMs operate differently. Some do pre-call research, some wing it. Some track commitments, some don't. The problem isn't automation — it's that there's no shared operational standard. Hook imposes one by automating it. But for key accounts, you don't want imposed automation — you want *enabled consistency*. Every TAM walks into every call equally prepared, every account gets the same intelligence depth, every review answers the same questions.

**4. The "what about accounts I'm not looking at" anxiety.**
Even in a key account book of 15–20, there are accounts you're actively engaged with and accounts that are quiet. Quiet isn't safe — it might mean the champion left and nobody noticed. The fear is: *what's happening in the accounts I haven't checked on lately?* Hook's Echo agent addresses this by scanning everything continuously. The question is whether DailyOS's signal monitoring already does this (it does, but surfacing is passive — you see it when you open the app, there's no push alert saying "Acme went quiet").

### Which Problem Does DailyOS Actually Solve?

| Problem | Hook's Answer | DailyOS + Glean's Answer |
|---|---|---|
| CTAs were noise | Better ML model | Better signal fusion (Bayesian, cited evidence, feedback loop). Same risk though — if input signals are weak, output is too. |
| VP needs portfolio visibility | Dashboard with aggregate scores | **Gap today.** Per-account intelligence is deep but not aggregated. |
| Operational consistency | Forced automation | **DailyOS's sweet spot.** Every TAM gets the same pre-meeting briefing, the same account health review, the same signal depth. Consistency through preparation, not automation. |
| "What am I not looking at?" | Continuous scanning + alerts | Signal bus + hygiene sweeps do this, but surfacing is passive. No push notification saying "Acme went quiet." |

### The Honest Segmentation

- **Standard book** — Hook (or something like it) is probably the right tool. You need automation at scale. DailyOS isn't built for that and shouldn't be. The chief-of-staff doesn't scale to 500 accounts per operator.
- **Key Account book** — Hook is the wrong tool. Auto-sending emails to your largest customers is a liability. What the VP needs is intelligence, visibility, and operational consistency — not automation.

The risk of adopting Hook for both segments is that the automation designed for Standard accounts bleeds into Key Accounts where it does more harm than good. The segmentation exists for a reason.

---

## 3. Feature-Level Comparison

### Where DailyOS + Glean Already Covers

| Hook Capability | DailyOS Equivalent | Version | Assessment |
|---|---|---|---|
| Account health scoring | `health_score` + `health_trend` on entity intelligence | v0.14.0 (shipped) | **Covered.** LLM-assessed with signal citations. |
| Risk detection & alerting | Risk Briefing report (6-slide SCQA executive format) | v0.15.0 (shipped) | **Covered.** Arguably deeper — structured executive narrative, not just a score. |
| Account Health Review | `AccountHealthReview` report — risks, stakeholders, engagement cadence, recommended actions | v0.15.0 (shipped) | **Covered.** |
| EBR/QBR generation | `EbrQbr` report — 8-section customer-facing quarterly review | v0.15.0 (shipped) | **Covered.** |
| Stakeholder mapping | Person relationship graph, `relationship_depth`, champion/exec alignment | v0.13.5 (shipped) | **Covered.** Richer than Hook — individual relationship context, not just a list. |
| Conversation intelligence | Meeting transcripts (Granola/Quill), email signal extraction | Shipped | **Partial.** No product usage, but conversation + email coverage is strong. |
| Org-wide context (Zendesk, Salesforce, Confluence, Gong) | Glean integration — pulls from all Glean-indexed systems | v0.15.2 (dev) | **Covered via Glean.** Glean indexes these sources; DailyOS synthesizes them into intelligence. |
| Signal monitoring | Signal bus with Bayesian fusion, time-decay, source reliability weighting | Shipped | **Covered.** More sophisticated than Hook's signal model (Thompson Sampling, feedback closure). |
| Renewal tracking | Renewal Readiness report | v1.0.1 (planned) | **Gap (planned).** I458 in backlog. |

### Real Gaps

#### Gap 1: Product Usage Telemetry (HIGH for Standard, LOW for Key Accounts)

Hook's primary signal source is product usage data — login frequency, feature adoption curves, usage trends, activation milestones. DailyOS has zero product telemetry integration. It knows what you *discussed* about a customer but not how they're *using the product*.

**Mitigation via Glean:** If product usage data lands in any Glean-indexed system (Snowflake dashboards in Confluence, Looker reports, Amplitude exports, internal wikis), Glean mode would surface it during enrichment. But this is passive — usage data appears as context in intelligence synthesis, not as structured metrics with threshold alerts.

**Honest assessment:** This is a real gap for the Standard book where churn signals are usage-led. For Key Accounts where churn signals are relationship-led (executive disengagement, champion departure, meeting cadence drop), DailyOS already covers the leading indicators. Product usage drops are a lagging indicator at the enterprise tier — by the time usage drops, the relationship problem happened months ago.

#### Gap 2: Autonomous Outbound Actions (HIGH for Standard, HARMFUL for Key Accounts)

Hook auto-sends emails, creates CRM tasks, fires Slack nudges, and executes playbooks without human intervention. DailyOS is **consume-only by design** — the chief-of-staff metaphor means it prepares you, it doesn't act as you.

**Honest assessment:** For Standard accounts, this automation is the whole point — scale demands it. For Key Accounts, autonomous outbound is a liability. Your VP of Key Accounts does not want an AI sending emails to their largest customers. The segmentation here is the answer: Hook for Standard, DailyOS for Key.

#### Gap 3: Executive Dashboard / Presentation Layer (MODERATE)

Hook and Gainsight both provide visual dashboards — portfolio health grids, trend charts, aggregate scores. Executives love numbers on screens. DailyOS's editorial magazine aesthetic is optimized for individual consumption (the CSM's morning briefing), not for a VP scrolling through portfolio health on a TV in the office.

**Honest assessment:** This is a real perception gap. DailyOS produces richer intelligence per-account than Hook, but it doesn't aggregate into a portfolio view that a VP can glance at. See Section 6 for a deeper treatment of this.

#### Gap 4: Natural Language Portfolio Queries (LOW)

"Which accounts haven't had an exec meeting in 90 days?" — Hook Chat answers these. DailyOS doesn't have a chat interface today.

**Mitigation:** The architecture already supports this. DailyOS exposes its SQLite database to Claude Desktop for natural language queries. Cmd+K search (I427, planned v0.16.1) covers structured lookups. A conversational interface for portfolio-level questions is tractable — the data is there, the query surface isn't. See Section 7.

#### Gap 5: Trained Churn Prediction Model (LOW at current scale)

Hook claims a trained ML classifier for churn prediction at 90%+ accuracy using historical patterns across the customer base. DailyOS's health scoring is LLM-assessed per-entity — it reads signals and synthesizes a health narrative with a numeric score.

**Honest assessment:** Trained models shine at scale (1000+ accounts) where cross-portfolio pattern recognition matters. For Key Account books of 15–50, the LLM approach with cited evidence may be *more actionable* because it explains *why* an account is at risk, not just *that* it is.

---

## 4. What DailyOS Does That Hook Cannot

| DailyOS Advantage | Why It Matters |
|---|---|
| **Meeting pre-briefs** — intelligence assembled before every meeting | Hook is account-level; DailyOS is meeting-level. You walk into every call prepared. |
| **Person-level relationship intelligence** — individual stakeholder context, relationship graph, interaction history | Hook has basic stakeholder lists. DailyOS tracks relationship strength, champion alignment, engagement cadence per person. |
| **Editorial daily briefing** — morning operating cadence | Not alerts or dashboards — a curated briefing that shapes your day. |
| **Weekly/Monthly impact reports** — personal performance narrative | No equivalent in Hook. CS team members can see their own impact. |
| **Multi-entity hierarchy** — parent accounts, project trees, cross-BU patterns | Hook has a flat account list. DailyOS sees portfolio structure. |
| **Email intelligence** — thread-level signal extraction with entity linkage | Hook relies on CRM data. DailyOS extracts signals directly from email. |
| **Local-first architecture** — all data on the user's machine | No SaaS vendor risk. No data leaving the device (in Local mode). |
| **Role presets** — CS, sales, product, leadership personas shape all output | Hook is one-size-fits-all. DailyOS adapts vocabulary and priorities per role. |
| **Signal sophistication** — Bayesian fusion, Thompson Sampling, time-decay, feedback closure | Hook's signal model is opaque. DailyOS's is auditable and self-improving. |
| **Custom report templates** — org-designed reports that encode operational expectations | Hook generates generic playbooks. DailyOS can deliver exactly the questions leadership wants answered. See Section 7. |

---

## 5. Facts vs. Interpretation: Why DailyOS Isn't Redundant to Glean

This is the core conceptual distinction that answers the inevitable "but we already have Glean" pushback.

**Glean retrieves facts. DailyOS interprets them. The human decides.**

Glean is a retrieval engine. It finds what exists across indexed sources. "What tickets does Acme have open?" "When was the last Gong call?" "What does the account plan say?" Those are lookups against documents that already exist somewhere in Salesforce, Zendesk, Confluence, or Gong. Glean is excellent at this because it has the connectors, the permissions model, and the search infrastructure.

But "Is Acme at risk?" is not a retrieval question. There is no document in Salesforce or Confluence that contains the sentence "Acme is at risk." Risk is an *interpretation* — it emerges from synthesizing multiple signals across multiple systems:

- The champion hasn't been in the last 3 meetings (calendar)
- Email response times have lengthened from hours to days (email signals)
- The renewal is in 90 days and no expansion conversation has started (CRM + calendar)
- The exec sponsor was reorged and the new one hasn't been introduced (relationship graph)
- Support tickets shifted from feature requests to complaints (Zendesk via Glean)

No single source contains that conclusion. Each fact lives in a different system. The interpretation — "this pattern means risk" — requires a layer that reads across all of them, weighs them by reliability and recency, tracks how they change over time, and produces a judgment with cited evidence.

That's what DailyOS does. The signal bus ingests facts from everywhere (including Glean). The Bayesian fusion scores them. The time-decay weights recency. The feedback loop learns from corrections. The health narrative synthesizes everything into a judgment the human can act on.

### The Three-Layer Model

| Layer | Owner | Question It Answers | Example |
|---|---|---|---|
| **Facts** | Glean | "What does the org know?" | "Acme has 3 open P1 tickets, last Gong call was 45 days ago, renewal date is June 15" |
| **Interpretation** | DailyOS | "What does this mean for you?" | "Acme is at moderate risk — champion engagement dropped post-reorg, support sentiment shifted negative, renewal in 90 days with no expansion motion started" |
| **Decision** | The human | "What should I do?" | "I'm going to escalate to my VP and request an exec alignment meeting next week" |

Glean feeds DailyOS. DailyOS feeds the human. The human decides.

### Could Glean Build This Interpretation Layer?

In theory, yes — Glean's Agent Builder supports custom agents with custom logic. But that agent would need to:

- Maintain per-user relationship history across months of interactions
- Track signal patterns over time (not just retrieve current state)
- Apply confidence weighting, time-decay, and source reliability scoring
- Learn from user corrections via feedback loops
- Produce narrative assessments shaped by the user's role, priorities, and professional context
- Generate structured reports (health reviews, risk briefings, EBR/QBRs) from synthesized intelligence

That's not a Glean agent — that's a product. It's DailyOS.

### Why This Matters for the Hook Evaluation

Hook tries to be all three layers: it retrieves signals, interprets risk, and acts autonomously. The problem is that collapsing these layers means you can't choose different tools for different strengths. Glean is better at retrieval than Hook. DailyOS is better at interpretation than Hook. And for Key Accounts, the human should always be the one deciding.

The separated model — Glean for facts, DailyOS for interpretation, humans for decisions — plays to each layer's strength and avoids the failure mode where a single tool's mediocre interpretation triggers an autonomous action on a key account.

---

## 6. The Glean Opportunity: Beyond Gap-Filling

### 5a. Glean as Account Discovery Engine

Today, DailyOS only knows about accounts the user has manually created. If you want intelligence on an account not in your local list, the answer is: create it first, wait for enrichment.

In Glean mode, there's an untapped opportunity: **Glean-driven account discovery.** Glean indexes Salesforce, so it knows every account in the org's CRM. A "Discover from Glean" flow could:

1. Query Glean for accounts assigned to the user (or in their territory)
2. Present a list of accounts not yet in DailyOS
3. One-click import with Glean-sourced context pre-populated (industry, ARR, key contacts, recent tickets)
4. Intelligence enrichment starts immediately with Glean context as the seed

This eliminates the cold-start problem for new DailyOS users and makes account creation feel like "connecting" rather than "entering data." For a VP evaluating the tool, this is the difference between "go set up all your accounts" and "here are your accounts — we already know about them."

### 5b. On-Demand Account Intelligence (Ephemeral Queries)

Beyond persistent accounts, there's a lighter pattern: **ephemeral account queries.** "Tell me about Acme Corp" when Acme isn't in your local list. DailyOS queries Glean, synthesizes available org knowledge, and presents a one-time briefing without creating a persistent entity.

This sits between "search" and "account creation" — useful for:
- Pre-call prep for a prospect you haven't added yet
- Quick context when someone mentions an account in a meeting
- Territory planning and account prioritization
- A VP asking "what's going on with [account they don't own]?"

The intelligence wouldn't persist (no local entity), but it would demonstrate immediate value and could offer "Add to my accounts" as a follow-up action.

### 5c. Glean-Powered Account Population

For a VP deploying DailyOS across their team, the onboarding story matters. Rather than each TAM manually creating their accounts, a Glean-powered setup flow could:

1. Authenticate with Glean
2. Pull the user's assigned accounts from CRM
3. Auto-create entities with Glean-sourced context
4. Begin enrichment immediately

Day-one value instead of a week of data entry. This is what makes DailyOS viable as a team tool, not just a personal one.

---

## 7. The VP Problem: Visibility, Control, and the Reporting Lever

### What the VP Actually Wants

A VP of Key Accounts has a small number of things they care about deeply:

1. **Risk and expansion visibility** — which accounts are at risk, which have expansion potential, and how confident can I be in those assessments?
2. **Operational consistency** — are my TAMs/CSMs doing the work? Are they prepared? Are accounts getting the right level of attention?
3. **Predictability** — can I forecast retention and expansion with confidence?
4. **A sense of control** — the ability to look at a surface and feel like they know what's happening across their book.

Hook addresses #4 with dashboards and aggregate scores. But dashboards create a false sense of control — a green/yellow/red grid tells you *what color the box is*, not *what's actually happening*. Gainsight had dashboards too, and the CTAs were noise nobody acted on. The problem isn't the absence of a dashboard. The problem is the absence of *trusted intelligence that answers the questions leadership actually asks*.

### Reports as Operational Rigor

Here's where DailyOS has a structural advantage that Hook doesn't: **we design the report templates.**

If the VP decides they want account reviews that answer a specific set of questions — "What value have we delivered this quarter? Where is the champion's confidence? What commitments are open? What's the expansion opportunity?" — DailyOS can create a report template that delivers exactly that, populated from real intelligence, for every account in the book.

This is more powerful than a dashboard because:

- **It encodes the operational standard.** The report template *is* the definition of "what good looks like." Every TAM's account gets the same treatment because the system produces the same report shape.
- **It answers real questions, not abstract scores.** A health score of 72 means nothing. "Champion engagement has declined since the reorg in January; last executive touchpoint was 47 days ago; renewal is in 90 days with no expansion conversation started" — that's actionable.
- **It's push, not pull.** The VP doesn't have to go hunting. Reports generate automatically when intelligence updates. "Your Key Account Reviews are ready" — open them, read them, act on them.
- **It evolves with the team.** When the VP says "I also want to see competitive mentions," that becomes a field in the template. The system adapts to what leadership wants to know, not the other way around.

### The Portfolio Health Surface

The reports solve the per-account depth problem. But the VP also needs the glanceable aggregate view — "across my whole book, how does it look?" This is the portfolio dashboard:

- All accounts' health scores in a grid or heatmap
- Risk distribution (X accounts green, Y amber, Z red)
- Trend arrows (improving/stable/declining)
- Upcoming renewals with readiness indicators
- Aggregate metrics: meetings this week, signals detected, actions pending

This doesn't require abandoning the editorial aesthetic — it extends it. Think "editor's summary page" at the front of a magazine: a glanceable overview that links deeper into per-account editorial content. The dashboard is the table of contents; the reports are the articles.

### Giving Control Without Enabling Micromanagement

Here's the subtle design challenge: the VP wants a sense of control over risk and expansion. That desire is legitimate — it's their job. But tools that promise total visibility can enable negative behaviors: daily check-ins on every account, questioning every TAM's judgment, treating the dashboard as a surveillance system.

The design principle should be: **give the VP confidence, not surveillance.**

- **Confidence through narrative, not numbers.** A health score invites "why is it 72 and not 78?" — a conversation nobody benefits from. A narrative like "Acme is stable with strong champion alignment; expansion conversation deferred to Q3 by mutual agreement" gives the VP what they need without creating a number to argue about.
- **Exception-based surfacing.** Don't show all accounts equally. Show the ones that need attention. "3 of your 18 Key Accounts have signals worth reviewing this week." The rest are fine — the system is watching them so the VP doesn't have to.
- **Team-level patterns, not individual tracking.** "Your team averaged 2.3 executive touchpoints per account this quarter, up from 1.8" is useful. "Sarah had 1 executive touchpoint on Acme while James had 4 on Beta" invites comparison that may not be fair.
- **Predictability through consistency, not control.** If every account gets the same intelligence treatment, the same report shape, the same signal monitoring — the VP can trust the system's coverage without checking each one. The rigor is built into the tool, not imposed by the manager.
- **Make the reports the conversation.** When a VP and TAM sit down to discuss an account, they should be looking at the same DailyOS report. The report becomes the shared artifact — not a surveillance tool the VP uses to quiz the TAM, but a shared intelligence surface they both consume. "The report says champion confidence has dropped — what's your read?" is a collaborative conversation. "Your health score dropped 5 points — explain" is not.

---

## 8. The Conversational Interface

### Beyond Cmd+K: Natural Language Account Queries

DailyOS already exposes its SQLite database to Claude Desktop for natural language queries. Cmd+K (I427) will handle structured search — find an account, jump to a meeting, locate a person. But the VP persona needs something different: the ability to *ask questions* about their portfolio.

"Which of my accounts haven't had an executive meeting in the last 60 days?"
"What are the top 3 risks across my book right now?"
"Summarize what happened with Acme this quarter."
"Which accounts have renewals in Q2 and what's their readiness?"

This is Hook Chat's value proposition, and it's the right idea. The difference is data depth — Hook Chat queries Hook's data (usage metrics, CRM fields, basic signals). A DailyOS conversational interface queries the full intelligence layer: signal history, relationship graphs, meeting transcripts, email intelligence, entity hierarchies, and Glean-sourced org knowledge.

### Glean Already Has a Slack Bot

Before building anything, we should assess what Glean's existing Slack integration already provides. As of February 2026, Glean offers:

- **@Glean in Slack** — a conversational AI assistant that answers questions from the full Glean-indexed corpus (Salesforce, Zendesk, Confluence, Gong, Jira, etc.)
- **Persistent sidebar** — keeps the Glean assistant open while working in other channels
- **Suggested prompts** — context-aware question suggestions based on the conversation
- **Thread history** — organized conversation threads with a History tab
- **Permission-enforced** — all answers respect the user's org-level access controls
- **Custom Glean Agents** — Glean's Agent Builder allows creating domain-specific agents that run in Slack with custom knowledge, governance, and orchestration

This means a VP could already say "@Glean what do we know about Acme?" in Slack and get an answer drawn from Salesforce records, Zendesk tickets, Gong calls, and Confluence docs. The question is: **what does DailyOS add that Glean alone doesn't provide?**

### What Glean's Slack Bot Knows vs. What DailyOS Knows

| Data Layer | Glean Knows | DailyOS Knows |
|---|---|---|
| CRM records (Salesforce) | Yes | No (Glean-sourced) |
| Support tickets (Zendesk) | Yes | No (Glean-sourced) |
| Sales calls (Gong) | Yes | No (Glean-sourced) |
| Internal docs (Confluence) | Yes | No (Glean-sourced) |
| Product usage (if indexed) | Yes | No |
| **Your meeting history + prep** | No | **Yes** |
| **Your email signal patterns** | Partial (indexes email) | **Yes** (signal extraction, entity linkage) |
| **Relationship depth per person** | No | **Yes** (graph, confidence, interaction cadence) |
| **Health score with cited evidence** | No | **Yes** (Bayesian, decay, feedback loop) |
| **Account health narrative** | No | **Yes** (LLM-synthesized from all signals) |
| **Meeting pre-briefs** | No | **Yes** |
| **Cross-entity hierarchy patterns** | No | **Yes** (parent/child, portfolio) |
| **Your personal priorities + context** | No | **Yes** (user entity, role preset) |

Glean answers "what does the org know about Acme?" DailyOS answers "what do *you* know about Acme, and what should you do about it?" These are complementary — and the most powerful answer combines both.

### The Slack Surface: Three Options

**Option A: Use Glean's Slack bot as-is.** For "what does the org know?" questions, this is already solved. No development needed. The VP can @Glean in any channel and get CRM/ticket/call context immediately.

**Option B: Build a DailyOS Slack bot.** "@dailyos pull up the account health report for Acme" — this surfaces DailyOS intelligence (health narrative, relationship depth, risk briefing) directly into Slack. Richer per-account intelligence than Glean alone, because it includes the user's personal signal history and relationship context. Development cost is real but tractable — DailyOS has the data, it's a surface question.

**Option C: Build a custom Glean Agent powered by DailyOS data.** Glean's Agent Builder supports custom agents with custom knowledge sources. A "DailyOS Agent" in Glean could combine Glean's org-wide context with DailyOS's per-user intelligence. The VP asks one agent and gets both layers. This is architecturally elegant but depends on Glean's Agent Builder capabilities and whether DailyOS can feed data into a custom Glean agent.

**Recommendation:** Start with Option A — Glean's Slack bot is free and already deployed. Evaluate whether it answers the VP's questions adequately. If the gap is "I need relationship depth and health narratives, not just CRM records," then Option B (DailyOS Slack bot) fills that gap. Option C is the long-term play but requires deeper Glean platform evaluation.

The most powerful Slack interaction is probably this: the VP is in a Slack thread discussing a customer, they @Glean for the org context (tickets, CRM, calls), and they @dailyos for the relationship intelligence (health, risks, stakeholder alignment, recommended actions). Two complementary lenses in the same conversation.

---

## 9. Coverage Summary

| Hook Value Area | DailyOS Coverage | Key Account Relevance | Gap Severity |
|---|---|---|---|
| Account health scoring | Shipped, signal-driven | High | None |
| Risk detection & briefing | Shipped, deeper format | High | None |
| EBR/QBR generation | Shipped | High | None |
| Stakeholder mapping | Shipped, richer model | High | None |
| Org-wide context | Glean integration (dev) | High | None (with Glean) |
| Conversation intelligence | Shipped (transcripts + email) | High | None |
| Product usage telemetry | Not built; partial via Glean | Low (lagging indicator for enterprise) | **Low** for Key Accounts |
| Autonomous outbound actions | By design: consume-only | **Harmful** for Key Accounts | None (correct design) |
| Churn prediction model | LLM-assessed, not trained ML | Moderate | **Low** (at current scale) |
| NL portfolio queries | Architecture ready, surface not built | High (VP persona) | **Moderate** (tractable) |
| Executive dashboard / portfolio view | Not built | High (VP persona) | **Moderate** (buildable) |
| Account discovery from CRM | Not built | High (onboarding) | **Moderate** (cold-start) |
| VP-designed report templates | Report infra shipped, templates are extensible | High | **Low** (extend existing) |
| Team-level aggregation | Not built | Moderate (manager view) | **Moderate** (governance design needed) |

**For the Key Account book:** DailyOS + Glean covers ~80% of what the VP actually needs. The remaining 20% is portfolio-level surfaces (dashboard, NL queries, team aggregation) — all buildable on top of existing intelligence infrastructure.

**For the Standard book:** DailyOS is not the right tool. Hook or similar automation-first platforms serve the scaled motion better. The two segments need different tools, and that's fine.

---

## 10. Recommended Actions (Prioritized)

### Already Done

- **Glean integration shipped** (v0.15.2) — Glean as a production context source. Dual-mode context architecture (Local / Glean Additive / Glean Governed) is live. Every gap marked "partial via Glean" is now "covered."

### Priority 1: Zero-Cost Wins (No development)

1. **Evaluate Glean's Slack bot for VP queries.** Glean already has @Glean in Slack with full access to Salesforce, Zendesk, Gong, Confluence. Test whether it answers the VP's most common questions ("what's going on with Acme?", "any open tickets?", "when was the last Gong call?"). If it does, the "conversational interface" gap is already partially closed — for free.

### Priority 2: Near-term (v1.0.1 or post-1.0 sprint)

2. **VP Account Review report template** — a report type designed around the specific questions leadership wants answered per account. This is the operational rigor lever. The template *is* the standard. Push-generated when intelligence updates. Low development cost (report infrastructure is shipped), high value for VP adoption. *This is the single most important thing to build for the VP persona.*

3. **Portfolio health surface** — aggregate view of all accounts' health scores, trends, risk distribution. Exception-based: "3 of 18 accounts need attention this week." A new page or a prominent section on AccountsPage. Narrative-first, not score-first — give confidence without inviting score-arguing. *This is what the VP opens in the morning.*

4. **Account discovery via Glean** — import accounts from Glean-indexed CRM with pre-populated context. Eliminates cold-start. Makes onboarding feel like "connecting" not "data entry." Could integrate with v0.16.0 onboarding flow. *This is what makes DailyOS viable as a team deployment.*

### Priority 3: Medium-term (post-1.0)

5. **Ephemeral account queries** — "tell me about X" without creating a persistent entity. Queries Glean, synthesizes a one-time briefing. Low effort, high wow factor for demos and VP adoption.

6. **DailyOS Slack bot** — "@dailyos pull up the account health report for Acme." Surfaces DailyOS intelligence (health narratives, relationship depth, risk briefings) directly into Slack conversations. Complements Glean's Slack bot: Glean provides org context, DailyOS provides personal intelligence. *This is the "sell" moment — the VP sees intelligence appear in the tool they already live in.*

7. **Conversational portfolio interface** — NL queries against the full intelligence layer via Claude Desktop or Slack. "Which accounts have renewals in Q2 and what's their readiness?" Architectural foundation exists (SQLite + Claude Desktop MCP). Surface design is the work.

### Priority 4: Strategic (v2.x)

8. **Action dispatch** — one-click push of DailyOS recommendations to Slack/Linear/email. Bridges the "autonomous actions" gap without full autonomy. Stays in the chief-of-staff metaphor: "here's what I'd do — shall I?"

9. **Renewal Readiness report** (I458) — already planned. Depends on VP Account Review template for format consistency.

10. **Team-level health sync** — lightweight aggregation for manager/VP view. Design challenge: give confidence without enabling surveillance. Consider writing health summaries back to a Glean-indexed location so the VP's Glean instance can aggregate across the team.

11. **Local-first AI** (was v1.1.0, now v2.1.0) — IntelligenceProvider abstraction, Ollama, OpenAI. Deferred: VP-facing surfaces take priority over LLM backend flexibility.

12. **Document intelligence** (was v1.2.0, now v2.2.0) — In-app markdown reader, document search. Deferred: same reason.

---

## 11. Conclusion

The question isn't "Hook or DailyOS?" — it's "what does the VP of Key Accounts actually need, and how do we give it to them without repeating the Gainsight pattern?"

Gainsight failed because it produced numbers nobody trusted and CTAs nobody acted on. Hook promises better numbers and smarter CTAs — but it's the same category of tool: automation-first, score-driven, designed for scaled motions. For the Standard book (80% of accounts, 20% of revenue), that category is correct. For Key Accounts (20% of accounts, 80% of revenue) where the relationship *is* the product, the category is wrong.

What the VP of Key Accounts actually needs:

| Need | Status |
|---|---|
| Trusted intelligence that updates itself | **Shipped** (DailyOS + Glean, v0.15.2) |
| Confidence that the team is prepared | **Shipped** (meeting prep + briefings) |
| Operational consistency without micromanagement | **Buildable** (VP-designed report templates) |
| Portfolio-level visibility that surfaces exceptions | **Buildable** (portfolio health surface) |
| On-demand answers about any account | **Partially available** (Glean Slack bot for org context); **buildable** (DailyOS Slack bot for relationship intelligence) |
| A sense of control over risk and expansion | **Design challenge** — give confidence through narrative and exception-surfacing, not surveillance through scores and dashboards |

DailyOS is already the strongest tool for the Key Account motion. The gaps are surfaces — portfolio dashboard, conversational queries, report templates designed for leadership — not capabilities. The intelligence engine, the signal fusion, the relationship depth, the Glean integration — all shipped and in production.

Meanwhile, Glean's existing Slack bot may already cover a significant portion of the "conversational interface" gap for org-level queries. The first step is evaluating what @Glean in Slack can do before building anything new.

**Recommendations:**

1. **Don't adopt Hook for the Key Account book.** It solves the wrong problem for high-touch accounts and risks the wrong kind of automation reaching your most valuable customers.
2. **Evaluate Hook independently for the Standard book.** That's a different product decision with different economics. DailyOS isn't the answer for 500-account scaled motions.
3. **Test Glean's Slack bot immediately** against the VP's most common questions. This is free and may already close the conversational gap.
4. **Build three VP-facing surfaces** in priority order: (a) VP Account Review report template, (b) portfolio health surface, (c) Glean-powered account discovery.
5. **Defer local-first AI (v1.1.0 → v2.1.0) and document intelligence (v1.2.0 → v2.2.0).** VP-facing intelligence surfaces are the post-1.0 priority. LLM backend flexibility and document features are valuable but not urgent relative to the Gainsight replacement question.

---

## 12. Implementation Plan: v1.1.x CS Intelligence

Based on this analysis, the post-1.0 roadmap has been restructured as v1.1.x — three incremental versions that close the VP-facing gaps identified above.

| Version | Theme | Issues |
|---------|-------|--------|
| **v1.1.0** | Intelligence foundation + CS report suite | I484-I491, I496-I498 |
| **v1.1.1** | Portfolio surfaces + account detail enrichment | I492-I493 |
| **v1.1.2** | Glean account discovery + ephemeral queries | I494-I495 |

**v1.0.1 dissolved** — all CS report types (I458-I461) absorbed into v1.1.0 with richer acceptance criteria because they now build on a fixed intelligence foundation rather than hollow data.

**Key architectural decision:** Fix the foundation first. Health scores are null for ~80% of accounts, inferred relationships are extracted then discarded, Glean data flows into prompts but never into the signal bus or people table. Reports built on hollow intelligence are hollow reports.

Version briefs: `.docs/plans/v1.1.0.md`, `.docs/plans/v1.1.1.md`, `.docs/plans/v1.1.2.md`.

### Glean Validation Gates

Before each phase, Glean capabilities must be validated with real queries:

- **Before Phase 1:** What person/org data does Glean return? Structured fields or just snippets? Signal types? Change notifications?
- **Before Phase 2:** CRM fields (ARR, renewal date)? Account-to-owner filtering? Competitive intelligence?
- **Before Phase 4:** Can Glean enumerate CRM accounts? Filter by territory? What metadata comes back?

Acceptance criteria will be adjusted based on what Glean actually provides vs. what we assume.

---

## Appendix: Sources

- [Hook Products](https://hook.co/products)
- [Glean Slack Integration](https://www.glean.com/connectors/slack)
- [Glean Slack Agents](https://www.glean.com/agents/slack)
- [Glean + Salesforce AI in Slack](https://www.glean.com/blog/glean-slack-ai-app-container)
- [Glean Slack Marketplace](https://www.glean.com/blog/slack-glean-marketplace)
- DailyOS ADR-0095 (Dual-Mode Context Architecture)
- DailyOS ADR-0096 (Glean Mode Local Footprint)
- DailyOS `.docs/research/glean-integration-analysis.md`
