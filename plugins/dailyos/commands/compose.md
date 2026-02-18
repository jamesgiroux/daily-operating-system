---
description: "Draft communications grounded in shared relationship history"
---

# /compose

Draft communications that reference actual shared history, respect relationship context, and include specific commitments. Not "as we discussed" but "the integration timeline you asked about on Tuesday."

## Arguments

- `$ARGUMENTS[0]` — Recipient name (optional). If omitted, ask or infer from context.
- `$ARGUMENTS[1]` — Topic or purpose (optional). "follow up on QBR", "escalation about timeline", "intro to Sarah."

## Workflow

### Step 1: Identify Recipient

Resolve the recipient from People/:
1. Match name against `People/` directories
2. Load person.json for role, organization, classification
3. Load person.md for relationship intelligence
4. Check data/emails.json for email address and recent correspondence
5. If recipient not found, check if they appear in any entity stakeholders.md or meeting attendees

**Domain alias resolution:** If the user says "email Marcus" and there is no People/Marcus but there is a People/Marcus-Rivera, resolve it. If ambiguous, ask.

### Step 2: Load Full Relationship Context

The relationship-context skill will auto-fire. Ensure you have:

- **Meeting history** — When did you last meet? What was discussed? What was the tone?
- **Communication patterns** — How do they prefer to communicate? What formality level?
- **Temperature** — Warming, stable, or cooling? What are the recent signals?
- **Open actions** — What do you owe them? What do they owe you?
- **Entity context** — If they are associated with an entity, what is the entity's current state?
- **Stakeholder role** — What position do they hold (champion, executive sponsor, technical buyer)?

### Step 3: Determine Communication Type

Based on context and purpose, categorize:

- **Follow-up** — After a meeting or previous conversation. Reference specific items discussed.
- **Outreach** — Initiating contact after a gap. Acknowledge the gap naturally, not apologetically.
- **Escalation** — Raising an issue to someone with authority. Clear, factual, specific about what is needed.
- **Update** — Providing information they requested or expect. Lead with the key information.
- **Introduction** — Connecting two people. Explain why each would value the connection.
- **Request** — Asking for something. Be direct about what you need and by when.

### Step 4: Draft the Communication

**Tone calibration by relationship:**
- **Executives** — Respect their time. Lead with the bottom line. Be specific about what you need from them. Short paragraphs.
- **Champions** — Warm but professional. Reference shared work. Show you remember what matters to them.
- **Peers** — Collaborative tone. Equal footing. Problem-solving language.
- **Technical contacts** — Precise. Data-oriented. Skip the preamble.
- **New relationships** — Professional, clear, with enough context that they can place you.

**Grounding in shared history:**

Do not write generic communication. Reference actual workspace data:

- Instead of "As we discussed..." write "The integration timeline you asked about in Tuesday's sync..."
- Instead of "I wanted to follow up..." write "You mentioned the Q3 budget concern during our January EBR — here's what we've done since..."
- Instead of "I hope this email finds you well..." write nothing. Start with substance.

**Include specific commitments:**
- State what you will do and by when
- State what you need from them and by when
- Reference any open actions that are relevant

### Step 5: Quality Check

Before presenting, verify:
- Tone matches the relationship depth and the person's communication preferences
- References to shared history are accurate (check against _archive/ and People/ data)
- No generic filler phrases ("I hope this finds you well", "per my last email", "just circling back")
- Commitments are specific and trackable
- Length is appropriate for the communication type and the recipient's preferences

**Quality exemplar — grounded follow-up email:**

```
Subject: Integration timeline update + QBR prep

Hi Marcus,

Following up on the timeline question from Tuesday's sync. We've confirmed the API migration can complete by March 15, which keeps us ahead of your Q2 planning cycle. Sarah is coordinating the technical handoff with your engineering team this week.

Two items for the QBR on March 8:
1. I'll have the adoption metrics report ready by March 3 for your review before the meeting
2. Can you confirm whether David will attend? His input on the platform roadmap discussion would change how we structure that section.

The renewal conversation is a separate track — I'd like to schedule 30 minutes with you and Elena the week of March 10 to walk through the proposal before it goes to procurement. Does that work?

Thanks,
{name}
```

Note what this email does:
- References the specific meeting where the question was asked (Tuesday's sync)
- Names the specific date and the dependency (March 15, Q2 planning cycle)
- References people by name with specific context (Sarah, David, Elena)
- Makes specific requests with dates
- Separates the QBR from the renewal conversation (shows awareness of process)

### Step 6: Output

Present the draft with:
- Subject line (for emails)
- Full body text
- Any notes about tone decisions or alternatives

If the political-intelligence skill fired (e.g., the recipient is in a sensitive situation), the draft will be informed by that intelligence but will not contain it. The email to David is professional. The internal note about why you are writing to David is separate.

### Step 7: Loop-Back

After presenting the draft:

```
Would you like me to:
1. Track the commitments in this email as actions in data/actions.json
   - "Send adoption metrics report to Marcus" — due Mar 3
   - "Schedule renewal discussion with Marcus and Elena" — due week of Mar 10
2. Update People/Marcus-Rivera/person.md with today's outreach

Or adjust the draft first?
```

## Skills That Contribute

- **relationship-context** — Provides the full relationship profile for tone and reference grounding
- **political-intelligence** — Enriches drafting when dynamics are sensitive (the intelligence shapes the draft but does not appear in it)
- **entity-intelligence** — Provides entity context when the communication involves entity business
- **action-awareness** — Surfaces open actions between you and the recipient
- **meeting-intelligence** — Provides meeting history for specific references
- **role-vocabulary** — Shapes tone calibration for the role context
- **loop-back** — Handles tracking commitments and updating relationship records
