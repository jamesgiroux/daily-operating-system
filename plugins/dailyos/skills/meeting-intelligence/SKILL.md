---
name: meeting-intelligence
description: "Deep meeting preparation, template awareness, and post-meeting capture intelligence"
---

# Meeting Intelligence

This skill fires when meeting preparation, scheduling, or post-meeting processing appears in conversation. It knows the 8 meeting template types, how to generate deep prep beyond what the app auto-generates, and how to process meeting outputs.

## Activation Triggers

Activate when:
- User mentions a specific meeting or asks about upcoming meetings
- User asks for meeting prep, talking points, or agenda
- `data/schedule.json` or `data/prep.json` is referenced
- A transcript or meeting notes appear in `_inbox/`
- User mentions a meeting they just finished

## The 8 Meeting Templates

Each template type drives what prep is generated and what output structure is expected.

### 1. customer-call
**Purpose:** Regular customer touchpoint — check-in, update, issue resolution.
**Prep focus:** Entity health snapshot, open actions for this customer, recent signals (emails, last meeting outcomes), any at-risk indicators.
**Key questions:** What do they need from us? What do we need from them? Any unresolved items?

### 2. qbr (Quarterly Business Review)
**Purpose:** Formal business review with customer stakeholders.
**Prep focus:** Full entity assessment (health, metrics, trajectory), stakeholder map with power dynamics, ROI/value narrative, risks to address proactively, success stories to highlight.
**Key questions:** What story are the numbers telling? Where are the gaps before they ask? What is the renewal/expansion conversation?

### 3. partnership
**Purpose:** Partner relationship management — joint planning, co-selling, integration.
**Prep focus:** Partner entity context, joint pipeline or project status, open commitments from both sides, strategic alignment check.
**Key questions:** What are we each bringing to the table? Where is alignment drifting? What is the next concrete joint action?

### 4. one-on-one
**Purpose:** Manager-report or peer 1:1 meeting.
**Prep focus:** Person profile and relationship history, open actions assigned to/from this person, topics from recent meetings that need follow-up, career or project context.
**Key questions:** What matters to them right now? What do I owe them? What support do they need?

### 5. internal-sync
**Purpose:** Team sync, standup, internal alignment meeting.
**Prep focus:** Relevant entity or project updates, cross-team dependencies, blocked items, decisions needed.
**Key questions:** What has changed since last sync? What is blocked? What decisions do we need?

### 6. all-hands
**Purpose:** Company or department-wide meeting.
**Prep focus:** Portfolio-level metrics if presenting, key wins and risks across entities, strategic themes.
**Key questions:** What is the narrative? What do people need to know vs. want to hear?

### 7. training
**Purpose:** Training session, workshop, enablement.
**Prep focus:** Audience context, learning objectives, materials needed, follow-up plan.
**Key questions:** What should they walk away knowing? How do we measure success?

### 8. external-unknown
**Purpose:** Catch-all for external meetings that do not match other templates.
**Prep focus:** Attendee research (People/ profiles + web lookup if needed), organization context, best guess at agenda and purpose.
**Key questions:** Who are these people? What do they likely want? What should I know going in?

## Deep Prep Generation

When the user asks for meeting prep, go beyond the app's auto-generated prep by layering these intelligence passes:

### Pass 1: Context Assembly
1. Read the meeting entry from `data/schedule.json`
2. Read any existing prep from `data/prep.json` for this meeting
3. Identify the meeting template type (from `meeting_type` field or infer from title/attendees)
4. Resolve the entity if one is associated

### Pass 2: Attendee Intelligence
1. For each attendee, check `People/` for existing profiles
2. Load person.json and person.md for known attendees
3. Note relationship temperature (warming/stable/cooling)
4. Identify the last time you met with each person and what was discussed
5. Flag any attendees with no People/ profile as intelligence gaps

### Pass 3: Entity Context
1. If an entity is associated, the entity-intelligence skill will have loaded full context
2. Pull relevant vitals, risks, wins, and stakeholder dynamics
3. Identify open actions tied to this entity, especially any that are overdue or due soon
4. Check recent email signals from `data/emails.json` related to this entity

### Pass 4: Strategic Layer
Based on template type, add:
- **Competitive context:** For customer-call and qbr, note any competitive signals
- **Scenario planning:** For qbr and partnership, prepare for 2-3 likely conversation directions
- **Question framework:** For each template, generate 3-5 high-value questions to ask (not generic — grounded in actual workspace data)
- **Risk briefing:** For any meeting with at-risk entity or cooling relationship, prepare defensive talking points

### Pass 5: Output Structure
Generate prep output matching the template:
```
## Meeting Prep: {title}
**Date:** {date/time}
**Template:** {type}
**Entity:** {name} ({health status})

### Attendees
- {name} — {role} | Last met: {date} | Temperature: {status}
  - Key context: {relevant recent signal or history}

### Current State
{Entity or topic summary with evidence}

### Talking Points
1. {Specific, grounded talking point}
2. {Another, with source reference}
3. {Third}

### Open Items
- {Action item} — due {date}, status {status}

### Questions to Ask
1. {Targeted question grounded in workspace data}
2. {Another}

### Watch For
- {Risk or dynamic to monitor during meeting}
- {Signal that would change your approach}
```

## Post-Meeting Awareness

When a transcript, meeting notes, or summary appears in `_inbox/` or is pasted into conversation:

1. Identify the meeting it corresponds to (match by date, attendees, or title)
2. Suggest processing via the capture command
3. Flag extractable items: new actions, relationship signals, entity intelligence updates
4. Offer to archive the processed output to `_archive/YYYY-MM/`

## Interaction with Other Skills

- **entity-intelligence** auto-fires when meeting has an associated entity
- **relationship-context** auto-fires for each attendee with a People/ profile
- **political-intelligence** may fire if attendee dynamics suggest sensitivity
- **action-awareness** surfaces open actions relevant to meeting context
- **role-vocabulary** shapes the prep language to match the active preset
- **loop-back** handles saving prep output and post-meeting artifacts
