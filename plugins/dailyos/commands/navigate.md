---
description: "Relationship navigation and political intelligence — always internal-only"
---

# /navigate

Tactical relationship intelligence for sensitive situations. Pre-conversation prep, communication review, situation analysis, post-meeting debrief, and stakeholder strategy. Direct, honest, actionable. No corporate euphemisms. Everything produced by this command is internal-only — it never appears in anything shared externally.

<!-- internal-only -->

## Arguments

- `$ARGUMENTS[0]` — Person name, entity name, or situation description (required).

Examples:
- `/navigate David Park` — Load full relationship intelligence and surface the tactical picture
- `/navigate Nielsen stakeholders` — Map the full influence landscape for an entity
- `/navigate "Sarah seems off in last two meetings"` — Situation analysis from signals
- `/navigate "prep for QBR with David"` — Pre-conversation tactical brief
- `/navigate "just finished the exec call"` — Post-meeting debrief mode

## Navigate Types by Role Preset

| Preset | Common Navigate Scenarios |
|---|---|
| Customer Success | Champion navigation, escalation management, exec sponsor engagement, renewal politics |
| Sales | Buyer navigation, competitive counter-positioning, multi-thread strategy, procurement dynamics |
| Partnerships | Alliance management, partner exec alignment, co-sell navigation, channel conflict |
| Agency | Client stakeholder management, scope negotiation, creative direction disputes |
| Consulting | Steering committee dynamics, implementation resistance, change management |
| Product | Cross-functional influence, engineering/design negotiation, executive buy-in |
| Leadership | Board management, peer dynamics, team politics, organizational change |
| The Desk | Auto-detect from situation context |

## Five Capability Modes

The command auto-selects the mode based on context. The user does not need to specify — the language and situation make it clear.

### Mode 1: Pre-Conversation Prep

**Triggers:** "prep for meeting with...", "talking to X tomorrow", "how should I approach..."

**Purpose:** Tactical brief before a sensitive meeting or conversation.

**What it reads:**
- Person profile from People/ (person.json + person.md)
- Meeting history from _archive/ — what has been discussed, what was the tone, what were the outcomes
- Entity context if the conversation is entity-related
- Stakeholder map — where this person sits in the power structure
- Action trail — what you owe them, what they owe you, what has been delivered and what has not
- Email signals — recent correspondence tone and content

**What it produces:**

```markdown
<!-- internal-only -->
## Pre-Conversation Brief: {Person Name}

### What They Care About
{Based on meeting history and communication patterns. Not what they say they care about — what their behavior reveals they actually care about.}

### Power Dynamic
{Your position relative to theirs. Who needs whom more right now? What leverage exists on each side?}

### What to Say
{Specific talking points grounded in shared history. Not generic advice — actual sentences and framings.}

### What NOT to Say
{Topics to avoid, framings that will backfire, history that should not be raised.}

### Likely Objections
{Based on their past behavior and current position, what they will push back on and how to respond.}

### The One Thing
{The single intervention that would most change the trajectory of this relationship or conversation. Be specific.}
```

**Example output:**

```markdown
<!-- internal-only -->
## Pre-Conversation Brief: David Park (CTO, Nielsen)

### What They Care About
David cares about technical credibility and vendor consolidation. His DataConf keynote was about reducing vendor sprawl — he is building a narrative internally that fewer, deeper vendor relationships are better. He will evaluate you through this lens: are you consolidating his stack or adding to the sprawl?

### Power Dynamic
David has veto power on vendor renewals (he killed the Snowflake deal in October). You need him more than he needs you — he has alternatives (Datadog evaluation), and you have a $2.4M renewal at stake. Your leverage: Sarah Chen (his VP) is your champion and recently promoted, giving her more influence relative to David.

### What to Say
- Lead with the analytics capabilities roadmap. He needs to see a technical path forward, not a relationship pitch.
- Reference his DataConf keynote positively — "Your point about vendor consolidation resonates. Here is how our platform is designed to be the consolidation layer, not an addition to sprawl."
- Bring the proof-of-concept concept. He will respect a show-me-don't-tell-me approach.

### What NOT to Say
- Do not reference his absence from meetings. He knows he has been absent. Pointing it out creates defensiveness.
- Do not lead with relationship or partnership language. He reads that as sales pressure.
- Do not badmouth Datadog. He is evaluating them. Disparaging his options makes you look insecure.

### Likely Objections
- "We are evaluating our entire analytics stack." Response: "Makes sense. We want to be part of that evaluation, not outside it. Can we set up a technical comparison?"
- "I need to see capability X." Response: Be honest. If the capability exists, demo it. If it does not, say so and give a timeline. David respects honesty about gaps more than overselling.

### The One Thing
Get David to agree to a technical proof-of-concept. Not a meeting, not a call, not a presentation. A hands-on evaluation. He makes decisions through direct technical assessment, not through slide decks. If you can get him into a POC, you shift the dynamic from "vendor being evaluated" to "partner building something together."
```

### Mode 2: Communication Review

**Triggers:** "review this email before I send...", "how does this message read?", "is this the right tone?"

**Purpose:** Review a high-stakes communication before it goes out.

**What it analyzes:**
- Tone vs. relationship depth — is the formality level appropriate for where the relationship actually is?
- Hidden commitments — does the message commit you to something the user did not intend?
- Audience awareness — who should see this? Who should NOT see this?
- Power dynamics — does the message position the user correctly relative to the recipient?
- Subtext — what does the message imply beyond what it says?

**What it produces:**

```markdown
<!-- internal-only -->
## Communication Review

### Tone Assessment
{Is the tone matched to the relationship? Too formal? Too casual? Too deferential? Too aggressive?}

### Hidden Commitments
{Anything in the message that could be read as a commitment the user did not intend.}

### Who Should/Shouldn't See This
{If there are CC/FW implications. Who would react well to this message? Who would react poorly?}

### Power Dynamic
{How this message positions the sender. Does it give away leverage? Does it create obligation?}

### Suggested Changes
{Specific edits with reasoning. Not rewriting — surgical changes to improve effectiveness.}
```

### Mode 3: Situation Analysis

**Triggers:** "something feels off with...", "what's going on with...", "the situation with..."

**Purpose:** Analyze a relationship or political situation using workspace signals.

**What it reads:**
- All relevant People/ profiles
- Entity stakeholder maps
- Meeting history patterns (frequency, attendance, tone)
- Email signal patterns
- Action trail (follow-through patterns)

**What it produces:**

```markdown
<!-- internal-only -->
## Situation Analysis: {description}

### Influence Map
| Person | Power | Motivation | Current Stance | Engagement Trend |
|---|---|---|---|---|
| {Name} | {decision-maker/influencer/blocker/champion} | {what they want} | {supportive/neutral/opposed} | {increasing/stable/decreasing} |

### Competing Interests
{Who wants what? Where do interests align and where do they conflict?}

### What the Signals Say
{Specific behavioral evidence. Frequency changes, attendance shifts, tone shifts, delegation patterns.}

### Path to Outcome
{Tactical recommendation. Who to engage, in what order, with what message.}
```

### Mode 4: Post-Meeting Debrief

**Triggers:** "just finished the call with...", "debrief on the meeting", "what just happened?"

**Purpose:** Analyze what actually happened in a meeting beyond the surface.

**What it produces:**

```markdown
<!-- internal-only -->
## Debrief: {meeting description}

### What Was Said vs. What Was Meant
{Read between the lines. When David said "we are evaluating our options," he meant "I have alternatives and I want you to know it."}

### Dynamic Shift
{Did the power dynamic change during the meeting? Did anyone gain or lose ground?}

### Commitments — Explicit and Implicit
{Explicit: what was clearly agreed. Implicit: what was implied or assumed but not stated.}

### Next 48 Hours
{What should the user do in the immediate aftermath? Who to email, what to send, what to NOT do.}
```

### Mode 5: Stakeholder Strategy

**Triggers:** "stakeholder strategy for...", "how to navigate the [entity] stakeholders", mentions multiple people in a political context

**Purpose:** Multi-stakeholder navigation in complex environments.

**What it produces:**

```markdown
<!-- internal-only -->
## Stakeholder Strategy: {entity or situation}

### Influence Map
{Full stakeholder map with power dynamics, motivations, and current stance.}

### Engagement Strategy per Stakeholder
| Stakeholder | Approach | Cadence | Key Message | Avoid |
|---|---|---|---|---|
| {Name} | {how to engage them} | {how often} | {what they need to hear} | {what not to say} |

### Sequence: Who First, and Why
{The order of engagement matters. Engaging the executive sponsor before aligning the champion can backfire. Lay out the sequence with reasoning.}

### Coalition Building
{Who can be aligned together? Which stakeholders reinforce each other? Where are natural alliances and where are fault lines?}

### Red Lines
{What would blow up the situation? What must be avoided at all costs?}
```

## Critical Constraints

### Internal-Only

Every piece of output from this command is marked `<!-- internal-only -->`. This intelligence must NEVER appear in:
- Emails, messages, or documents shared externally
- Meeting prep visible to attendees
- Deliverables created by the produce command
- Anything that leaves the user's personal workspace

When navigate intelligence informs other commands (compose uses relationship context, assess references stakeholder dynamics), the political intelligence shapes the output but is invisible in it. The email is professional. The navigate brief is tactical.

### Direct Tone

This command does not use corporate language. It uses plain, direct, honest language:
- "David is avoiding you. Force the interaction." Not "Consider proactively re-engaging the CTO."
- "Sarah has more power now than David realizes. Use that." Not "Leverage the champion's enhanced organizational position."
- "This meeting is a trap. They want to benchmark you against Datadog." Not "Be aware of potential competitive dynamics in the discussion."

### Evidence-Grounded

Every tactical read must trace to observable signals:
- "David's disengaged" — because he has not attended in 4 months (archive data)
- "Sarah's gaining influence" — because she was promoted and is now in budget meetings (person.md)
- "The silence after the proposal is concerning" — because response time averaged 2 hours previously and it has been 5 days (email signals)

Intuition informed by patterns is fine. Unsupported speculation is not.

## Loop-Back

Navigate output is typically consumed in the moment, not saved. However, offer:

```
Would you like me to:
1. Save this brief to a private notes file (not in entity directory)
2. Create any actions from the tactical recommendations
3. Update People/{name}/person.md with the new signals identified

Note: navigate output is internal-only and will not be saved to entity directories.
```

If saved, use a private location — not the entity directory where others might access it.

## Skills That Contribute

- **political-intelligence** — The primary intelligence engine; fires automatically to provide power dynamics, shift detection, and subtext reading
- **relationship-context** — Provides person profiles, temperature, and meeting history
- **entity-intelligence** — Provides entity context for entity-related navigation
- **meeting-intelligence** — Provides meeting history for pre-conversation and debrief modes
- **action-awareness** — Provides the follow-through record (who delivers, who does not)
- **role-vocabulary** — Shapes navigate types and stakeholder role terminology
- **analytical-frameworks** — May activate when the situation requires structured decomposition
