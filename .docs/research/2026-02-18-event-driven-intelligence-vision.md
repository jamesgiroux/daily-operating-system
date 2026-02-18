# From Scheduled Pipelines to Event-Driven Intelligence

**Date:** 2026-02-18
**Type:** Vision addendum to ADR-0080 (Signal Intelligence Architecture)
**Participants:** James Giroux, Claude Code

---

## The Evolution

DailyOS was built as a pipeline system. Calendar data flows in, AI enriches it, documents flow out. Every stage runs on a timer: 8am briefing, 4-hour hygiene scans, 24-hour enrichment sweeps. This was the right architecture for v0.7 through v0.9 — it proved the core value proposition that AI can prepare your workday before you open the app.

But after months of daily use, the cracks are visible. The system doesn't respond to what happened — it responds to what time it is. When you correct an agenda item, that correction dies in the prep file. When an internal call surfaces an action for a customer account, the system can't connect those dots. When Clay says a contact changed jobs, nothing happens until the next scheduled sweep finds it.

The next evolution isn't a bigger pipeline or a faster schedule. It's a system that *notices things* and *learns from you*.

---

## What "Always-On AI" Actually Means

The industry narrative around "always-on AI agents" (OpenClaw, Skywork, etc.) suggests a persistent AI process watching everything. The reality is simpler and more practical:

**OpenClaw's "heartbeat"** is not a background agent. It's a memory flush triggered when context compaction approaches a threshold. The model consolidates memories into Markdown files, then goes idle. The "always-on" feeling comes from good memory retrieval — next session loads relevant context — not from continuous processing.

**What DailyOS needs** isn't a persistent AI process either. It's an **event-driven signal system** where:

1. Things happen (calendar change, email arrives, user corrects something, enrichment completes)
2. The system notices (event → signal)
3. The system reasons about it (signal fusion → confidence → threshold)
4. The system acts or waits (enrich, flag, surface, learn, or do nothing)

This is computationally lightweight. SQLite queries, embedding cosine similarity, Bayesian math. No persistent LLM burning CPU. The AI (Claude Code) is called only when the system decides it needs to generate or re-generate intelligence — and that decision is informed by signals, not a clock.

---

## The User's Experience Today vs Tomorrow

### Today: Scheduled Pipeline

```
6:30 AM  — Scheduler wakes up, runs full briefing pipeline
7:00 AM  — User opens app, sees briefing
7:15 AM  — User notices "Bring a Trailer" meeting has wrong agenda items
           (internal discussion points mixed with customer-facing agenda)
7:16 AM  — User manually edits agenda, removes irrelevant actions
7:17 AM  — Edits saved to prep file. System learns nothing.
7:30 AM  — User notices an action from yesterday's internal call
           should be linked to an account. Manually fixes it.
7:31 AM  — Junction table updated. System learns nothing.
           Next briefing will repeat the same mistakes.
```

### Tomorrow: Event-Driven Intelligence

```
6:30 AM  — Briefing pipeline runs (scheduled, still the safety net)
7:00 AM  — User opens app, sees briefing
7:15 AM  — User edits "Bring a Trailer" agenda, removes 3 items
           → SIGNAL: user_correction on meeting prep
           → System records: these 3 items were sourced from
             internal meeting transcript (wrong context)
           → Learning: internal call content should not flow into
             customer-facing agenda for this account
           → Next briefing: internal vs external context separation
             has higher weight for Bring a Trailer meetings
7:30 AM  — User links an action from internal call to customer account
           → SIGNAL: entity_correction on action
           → System records: this action was extracted from internal
             meeting but belongs to external account
           → Propagation: re-enrich the account with this new action context
           → Learning: when Renan discusses [account], actions may belong
             to the account even if the meeting is internal
9:00 AM  — Calendar event added: "Agentforce Demo — Jefferies"
           → SIGNAL: new_meeting
           → Entity resolution runs immediately (not at next scheduled sweep)
           → Title embedding matches Agentforce project (0.91 similarity)
           → Attendee lookup finds Jefferies contacts
           → Auto-links: Agentforce (project) + Jefferies (account)
           → Queues prep enrichment with both entities' intelligence
           → By the time user looks, it's already tagged correctly
2:00 PM  — Clay enrichment returns: contact at Acme changed title to CRO
           → SIGNAL: clay_title_change
           → Propagation: Acme account flagged — champion promoted
           → Cross-reference: Acme renewal in 45 days
           → Surfaces in tomorrow's briefing: "Sarah promoted to CRO at Acme
             — renewal in 45 days, this could be expansion opportunity or risk
             depending on her new priorities"
```

The difference isn't that the system runs more AI. It's that the system *pays attention* to what you do and what changes around you, and uses that to make better decisions about when and how to act.

---

## The Three Minds of DailyOS

The system evolves through three complementary intelligence modes:

### Mind 1: The Scheduler (v0.7–v0.9, exists today)

Timer-driven. Runs pipelines on cron schedules. The reliable workhorse that ensures your day is ready by 8am. This never goes away — it's the safety net.

**Strength:** Predictable, debuggable, ensures baseline readiness.
**Weakness:** Doesn't respond to change. Repeats mistakes. No learning.

### Mind 2: The Signal Engine (v0.10, ADR-0080)

Event-driven. Watches for changes, fuses signals, learns from corrections. The pattern-recognizer that connects dots across entities.

**Strength:** Responsive, learning, proportional (acts when confidence is high, asks when it's not).
**Weakness:** Only as good as its signals. Cold-start problem for new users. Can't generate new insight — only combine existing signals.

### Mind 3: The Reasoning Layer (v1.0+, this vision)

Inference-driven. Uses the embedding model and AI calls to generate novel insights from signal patterns. The "always-on intelligence" that feels like the system understands you.

**Strength:** Can surface things no single signal contains. "Sarah's promotion + Acme's renewal + dropped meeting frequency = risk pattern."
**Weakness:** Expensive (AI calls), slower, harder to debug. Must be triggered judiciously.

The three minds work together:

```
Event happens
    → Signal Engine scores it (cheap, instant, local)
    → If confidence > threshold: act deterministically (link entity, update weight)
    → If confidence is ambiguous: queue for Reasoning Layer
    → Reasoning Layer uses embeddings + AI to resolve ambiguity
    → Result feeds back into Signal Engine weights
    → Scheduler ensures nothing falls through the cracks
```

---

## Every Correction Is a Signal

The most underutilized data source in DailyOS is user behavior. Every interaction is a signal:

### Explicit Corrections (highest value)

| User action | Signal | What the system should learn |
|-------------|--------|------------------------------|
| Edit meeting agenda items | `prep_correction` | Which source content was wrong for this meeting type |
| Remove suggested actions | `action_rejection` | These action patterns don't belong in this context |
| Re-tag meeting entity | `entity_correction` | Which resolution signal was wrong, what was right |
| Edit executive assessment | `intelligence_correction` | AI synthesis was off for this entity |
| Dismiss hygiene suggestion | `suggestion_rejection` | This type of suggestion isn't useful |
| Accept hygiene suggestion | `suggestion_acceptance` | This type of suggestion is valuable |

### Implicit Signals (lower value, high volume)

| User behavior | Signal | What the system can infer |
|---------------|--------|--------------------------|
| Time spent on entity page | `engagement` | This entity matters to the user right now |
| Meeting prep opened before meeting | `prep_consumption` | Prep was useful — maintain quality for this entity |
| Meeting prep NOT opened | `prep_ignored` | Prep may not be relevant for this meeting type |
| Action completed same day | `action_urgency` | This action pattern correlates with real urgency |
| Action ignored for 7+ days | `action_irrelevance` | This action pattern may not be worth surfacing |

### The Bring a Trailer Example

The user edited the Bring a Trailer agenda and removed items that came from an internal discussion between colleagues. This is a rich signal:

1. **What happened:** Internal meeting transcript content leaked into customer-facing agenda
2. **Why it happened:** The enrichment pipeline doesn't distinguish "things discussed internally about the account" from "things to discuss with the account"
3. **What the system should learn:** For this account (and likely all accounts), internal meeting context should be tagged as `internal_context` not `customer_agenda`. When building a customer-facing prep, prioritize content from customer-facing meetings and email threads over internal discussions.
4. **How it applies going forward:** The signal engine records that internal transcript sourced items were rejected for customer prep. Over time, the weight of internal-transcript-sourced items in customer-facing agendas decreases. The system starts separating "what we know about the account" from "what to discuss with the account."

This is exactly the kind of learning that can't happen with a scheduled pipeline. The pipeline would make the same mistake tomorrow because it has no memory of the correction.

---

## Internal vs External Context: A Concrete Example

One of the most common enrichment failures is conflating internal and external context. When Renan and James discuss Bring a Trailer internally, the transcript produces:

- Actions (follow up on pricing, schedule demo)
- Discussion points (concerns about timeline, feature gaps)
- Decisions (prioritize mobile, defer API work)

When the system builds prep for a customer meeting with Bring a Trailer, it should use some of this context but not all:

| From internal call | Customer prep? | Why |
|-------------------|---------------|-----|
| "Follow up on pricing" | Yes — action item | Directly relevant to customer relationship |
| "They seemed concerned about timeline" | Yes — as risk context | Informs how to approach the meeting |
| "We need to prioritize mobile" | No — internal roadmap | Customer doesn't need to know about internal prioritization |
| "Defer API work until Q2" | No — internal decision | Could undermine confidence if surfaced to customer |
| "Renan will lead the demo" | Maybe — as prep note | Useful for meeting readiness, not for agenda |

Today the system can't make these distinctions because it doesn't track the source context type (internal meeting vs customer meeting vs email vs document). The signal engine changes this:

1. Every content chunk carries a `source_context` tag: `internal_meeting`, `customer_meeting`, `inbound_email`, `outbound_email`, `document`, `user_authored`
2. When building customer-facing prep, weight customer-context sources higher than internal-context sources
3. When the user removes an internal-sourced item from customer prep, that's a signal: increase the penalty for internal sources in customer prep for this entity
4. Over time, the system learns which internal insights are useful externally (risks, timeline concerns) and which aren't (roadmap decisions, internal resource allocation)

---

## The Role of the Embedding Model

The local embedding model (nomic-embed-text-v1.5, already running via ONNX Runtime) is the bridge between deterministic signals and intelligent behavior. It enables reasoning without AI calls:

### 1. Semantic Entity Resolution

When a new meeting appears titled "Q1 Platform Migration Review":
- Embed the title → compare against all project name embeddings
- Cosine similarity to "Q1 Platform Migration" project: 0.93 → high-confidence match
- No keyword rules, no regex patterns, no AI call. Just vector math.

### 2. Context Relevance Scoring

When building meeting prep from multiple entity intelligence sources:
- Embed the meeting title + attendee context as a "query"
- Score each intelligence chunk (risks, wins, stakeholder insights) against the query
- Rank by relevance → top-N chunks enter the prep context
- Stale or irrelevant intelligence naturally drops out without explicit rules

### 3. Change Detection

When Clay enrichment updates a person's profile:
- Embed the new profile → compare against the stored embedding
- High cosine distance (>0.3) = meaningful change (job title, company, role)
- Low cosine distance (<0.1) = cosmetic change (bio wording, photo update)
- Only meaningful changes propagate as signals. Cosmetic changes are stored but don't trigger actions.

### 4. Signal Clustering

When multiple signals arrive about the same entity:
- Embed each signal's text content
- Cluster by semantic similarity
- Signals that cluster together compound (Bayesian fusion)
- Signals that don't cluster are independent evidence

### 5. Memory Retrieval for Reasoning

When the Reasoning Layer (Mind 3) needs to resolve an ambiguity:
- Embed the question: "Is this meeting about Agentforce or Salesforce Security?"
- Search across: past meeting preps, transcripts, entity intelligence, user corrections
- Retrieve the most relevant context
- Pass to Claude Code with focused context (not the entire entity history)

The embedding model turns every text artifact in the system into a queryable vector. Combined with the signal bus, it means the system can answer "what do I know that's relevant to this situation?" instantly, locally, without an AI call.

---

## Email: The Unrealized Signal Layer

Email is the most underutilized data source in DailyOS. The system fetches Gmail, runs AI priority classification (high/medium/low), and surfaces email signals on entity pages. But email intelligence today is a display feature — it doesn't feed back into the intelligence loop.

### What email knows that nothing else does

| Signal | What email reveals | Current status |
|--------|-------------------|----------------|
| **Pre-meeting context** | The invite thread, agenda attachments, pre-reads shared by the customer | Not connected to meeting prep |
| **Relationship temperature** | Response times, tone shifts, escalation language | Extracted as `email_signals` but not fused with entity intelligence |
| **Account health** | Support tickets mentioned, feature requests, frustration patterns | Classified for display, not for risk scoring |
| **Action provenance** | "Can you send me the proposal by Friday?" → action with deadline and owner | Extracted but not linked to the right entity with confidence |
| **Entity resolution** | Email thread participants + subject line → which account/project this is about | Not used for meeting-entity resolution |
| **Internal vs external** | Whether a thread is between colleagues or with a customer | Not tagged, so internal email discussions about a customer can leak into customer-facing context |

### How email should feed the signal engine

**Tier 1 — Entity resolution signal (I305 Phase 2):**
When resolving a meeting's entity, check email threads from the past 48 hours involving the same participants. If an email thread between you and the meeting attendees mentions "Acme renewal" or "Agentforce demo," that's a high-confidence entity signal. This requires correlating email participants with meeting attendees — both are email addresses, so the join is straightforward.

**Tier 2 — Relationship and sentiment signals:**
Email response patterns are leading indicators of account health. When a champion who usually responds in 2 hours starts taking 3 days, that's a signal — not individually actionable, but it compounds with other signals (meeting frequency drop, negative transcript sentiment) into a risk pattern. The signal engine should track email response cadence per person and flag deviations.

**Tier 3 — Action extraction with entity context:**
Today email-extracted actions often lack entity context ("send the proposal" — for which account?). The email thread's participants and subject line carry entity context that should propagate to extracted actions. When an email to sarah@acme.com says "send the proposal by Friday," the action should auto-link to Acme.

**Tier 4 — Pre-meeting intelligence:**
The 24-48 hours of email before a meeting are some of the richest context available. Pre-reads shared by the customer, agenda suggestions, "I'd like to discuss X" signals. Today this context is invisible to meeting prep. The signal engine should surface recent email threads involving meeting attendees as prep context, weighted by recency and relevance (embedding similarity to the meeting title).

### The email-calendar bridge

The most powerful email signal is the one that connects email threads to calendar events. Today these are siloed: calendar knows when you meet, email knows what you discussed before and after. Bridging them:

1. **Pre-meeting:** 48 hours before a meeting, find email threads with overlapping participants. Surface relevant excerpts in prep context.
2. **Post-meeting:** After a meeting ends, find email threads that start within 24 hours involving the same participants. These are likely follow-ups — extract actions, link to meeting.
3. **Entity confirmation:** If email intelligence has classified threads to Account X, and a meeting has the same participants, that's a strong entity resolution signal even if the meeting title is ambiguous.

Email is the connective tissue between meetings, actions, and entities. The system has the data — it just doesn't use it as a signal source yet.

---

## What We Can Learn from OpenClaw (Beyond the Feb 14 Research)

### What OpenClaw got right:

1. **Files as source of truth.** Memory is Markdown on disk, not a black-box embedding store. DailyOS already does this with entity intelligence files. We should extend this pattern to signal history — make it inspectable, not just queryable.

2. **Graceful degradation.** OpenClaw's memory works without `sqlite-vec`, falling back to in-memory cosine similarity. DailyOS should ensure the signal engine works at full quality with the embedding model but degrades gracefully without it (e.g., keyword-only matching if embeddings unavailable).

3. **User-directed memory.** OpenClaw doesn't auto-extract memories — the user says "remember this." DailyOS should honor user-authored content as the highest-fidelity signal. When a user writes notes on a person page, that's more valuable than any AI-generated assessment.

4. **Temporal decay with configurable half-life.** OpenClaw uses 30-day default half-life for daily notes. ADR-0080 proposes similar decay by source type. The insight is that half-life should also vary by entity importance — a churning customer's signals should decay slower during renewal season.

### What OpenClaw got wrong (for DailyOS's use case):

1. **Chat-first interaction.** OpenClaw assumes you'll tell it what to remember. DailyOS should observe and infer. The user shouldn't have to tell the system "that meeting was about Agentforce" — the system should figure it out.

2. **Flat memory model.** OpenClaw's memories are unstructured Markdown. DailyOS already has structured entity intelligence (typed fields, JSON schemas). The signal engine should maintain this structure while adding the flexibility of unstructured signals.

3. **No cross-entity reasoning.** OpenClaw's memory is per-conversation. It can't connect "Sarah mentioned churn risk at Acme" (from one conversation) with "Acme renewal in 45 days" (from account metadata) with "meeting frequency dropped" (from calendar patterns). DailyOS's entity graph enables this.

4. **No learning from corrections.** OpenClaw doesn't track when the user overrides the AI or corrects a memory. DailyOS's correction feedback loop (ADR-0080, Section 7) is a meaningful differentiator.

---

## The Path from Here

### v0.9 (shipped): Integration plumbing
More data sources (Clay, Gravatar, Granola). More signals flowing in. But no intelligence about those signals — they write to entity fields and that's it.

### v0.10 (next): Signal foundation
- **I305** — Intelligent meeting-entity resolution (project keywords, re-enrichment on correction, confidence thresholds in hygiene)
- **I306** — Signal bus foundation (signal_events table, Bayesian fusion, confidence scoring, email-calendar bridge for entity resolution)
- **I307** — Correction learning and context tagging (Thompson Sampling weights, internal vs external source tagging, calendar description mining, attendee group patterns)

### v0.11: Event-driven intelligence
- **I308** — Event-driven signal processing and cross-entity propagation (calendar change → immediate resolution, Clay job change → account risk, email sentiment → relationship signal, embedding-based relevance scoring, fastembed reranker)
- Scheduled pipelines remain as safety net, not primary driver
- Email pre-meeting intelligence (48-hour thread surfacing in prep context)
- Email post-meeting correlation (follow-up thread → action extraction with entity context)

### v1.0: Compound intelligence
- Pattern detection across entities and time (frequency analysis, sentiment trends)
- Proactive surfacing of novel insights (things no single signal contains)
- Personalized signal weights that reflect how this specific user works
- The system gets measurably better each week as it learns from corrections
- Email relationship cadence tracking and deviation detection

---

## Measuring Success

How do you know the intelligence layer is working?

1. **Correction rate decreases over time.** If the user corrects entity assignments 5 times per day in week 1 and 1 time per day in week 8, the system is learning.

2. **Prep relevance increases.** If users stop removing agenda items from meeting prep, the context selection is improving.

3. **Proactive suggestions accepted.** If hygiene suggestions are accepted more often than dismissed, the confidence thresholds are calibrated correctly.

4. **Time-to-briefing decreases.** If the user spends less time editing the briefing each morning, the system is doing more of the work.

5. **Entity resolution accuracy.** Track: of meetings auto-linked to entities, what percentage were corrected by the user? This is the core metric.

None of these metrics require telemetry infrastructure or external analytics. They can all be computed from the local signal event log. The system measures itself.

---

## First Principle, Revisited

> "AI produces, users consume."

The scheduled pipeline achieved this for the happy path: open the app, your day is ready. But every correction the user makes is a moment where they're producing instead of consuming. The signal engine's job is to reduce those moments — not to zero (the user should always have editorial control) but to the point where corrections feel like personal preference, not system failure.

The "always-on AI" vision isn't about running an AI in the background. It's about building a system that remembers, learns, and improves — so that each morning's briefing is better than the last, not because the AI got smarter, but because the system understood you better.
