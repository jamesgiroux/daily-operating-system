---
name: relationship-context
description: "Auto-loads person profiles, meeting history, temperature signals, and stakeholder positions when anyone is mentioned"
---

# Relationship Context

This skill fires when a person is mentioned in conversation. It loads the full relationship profile from People/ and cross-references their position within entities and meeting history. The goal is to ensure every interaction with or about a person is informed by the relationship's full context.

## Activation Triggers

Activate when:
- A person's name is mentioned in conversation
- An attendee list is being processed for meeting prep
- Stakeholders are being discussed for an entity
- A compose or navigate command involves a specific person
- Actions reference a person by name

## Person Resolution

1. List directories under `People/`
2. Match the mentioned name against directory names (case-insensitive, partial match, handle "first last" and "last, first" formats)
3. If exact match found, load silently
4. If multiple partial matches, clarify: "Did you mean People/Sarah-Chen or People/Sarah-Martinez?"
5. If no match in People/, check `data/emails.json` for email address matching — the person may exist in email signals but not yet have a profile

## Profile Loading

### person.json — Structured Data

Read `People/{name}/person.json` for:
- **name** — Full name
- **role** — Their function (VP Engineering, Account Manager, CEO)
- **title** — Formal title
- **organization** — Their company or team
- **classification** — Internal (your org), External (their org), Partner
- **relationship_type** — Champion, Stakeholder, Executive, Peer, Report, Partner Contact

### person.md — Relationship Intelligence

Read `People/{name}/person.md` for the qualitative relationship picture:

**Meeting signals:**
- Frequency — How often you meet (weekly, monthly, quarterly)
- Topics — What you typically discuss
- Tone patterns — Collaborative, transactional, guarded, warm
- Trend — Meeting frequency increasing or decreasing

**Temperature model:**
- **Warming** — Engagement increasing, responsiveness improving, deeper topics, proactive outreach from them
- **Stable** — Consistent engagement pattern, no significant shifts
- **Cooling** — Declining engagement, shorter meetings, delayed responses, topic avoidance, delegation to others

Temperature is always backed by evidence. Not "I think they're cooling" but "Response time has doubled in February, they delegated the last two meetings to their team lead, and they declined the EBR invitation."

**Communication preferences:**
- Format — Email, Slack, phone, in-person
- Frequency — How often they expect/want to hear from you
- Formality level — Formal (Mr./Ms., structured agendas), Professional (first name, organized but flexible), Casual (informal, quick messages)
- Best approach — What resonates with them (data-driven, story-driven, bottom-line first)

**Relationship arc:**
- How the relationship started
- Key moments that built or eroded trust
- Current trajectory and what is driving it

## Stakeholder Position

Cross-reference the person against entity stakeholder maps:

1. For each entity in `Accounts/` and `Projects/`, check `stakeholders.md`
2. Find this person's role within each entity: Champion, Executive Sponsor, Buyer, Influencer, Blocker, User
3. Note their power level and engagement level within each entity context
4. A person can hold different roles across different entities

This provides the political context — not just who someone is, but what power they hold and where.

## Meeting History

Scan `_archive/` for meetings involving this person:

1. Search recent monthly directories in `_archive/YYYY-MM/`
2. Find meeting summaries that mention this person's name
3. Extract: dates, topics discussed, outcomes, commitments made
4. Calculate meeting frequency and identify trends
5. Note the last meeting date and how long ago it was

Meeting history tells the relationship story. If you met someone weekly for three months and then not at all for six weeks, that is a signal.

## Context Assembly

After loading, the following context is available:

```
Person: {name}
Role: {role} at {organization}
Classification: {classification}
Relationship Type: {relationship_type}

Temperature: {warming|stable|cooling}
  Evidence: {specific signals}

Last Contact: {date} — {meeting or communication type}
Meeting Frequency: {pattern}
Communication Preference: {format} / {formality}

Entity Roles:
  - {Entity1}: {stakeholder role} — {engagement level}
  - {Entity2}: {stakeholder role} — {engagement level}

Open Actions:
  - You owe them: {count}
  - They owe you: {count}

Recent History:
  - {date}: {meeting summary}
  - {date}: {meeting summary}
```

## Behavior Rules

1. **Silent loading.** Load person context without announcing it. The user should just notice that responses are informed.
2. **Relationship-aware output.** When the user asks about someone, lead with the relationship context, not just raw data. "Sarah is the champion at Nielsen — you meet weekly, she's been warming since the January EBR, and she owes you the stakeholder list from last Tuesday."
3. **Multi-person support.** If multiple people are mentioned, load context for each. Keep distinct.
4. **Temperature is directional.** It matters not just where the temperature is, but which way it is moving. A stable relationship is different from one that was warming and just plateaued.
5. **Absence is a signal.** If someone has no recent meetings, no recent actions, and no recent emails — that absence itself is meaningful context.

## Interaction with Other Skills

- **entity-intelligence** provides entity context for stakeholder role cross-referencing
- **political-intelligence** fires alongside this skill when language suggests dynamics or tension — it goes deeper into power structures and subtext
- **action-awareness** uses person context to filter and present relevant actions
- **meeting-intelligence** uses person profiles for attendee intelligence in meeting prep
- **role-vocabulary** shapes how relationship status is described
- **loop-back** handles updating person.md with new relationship intelligence
