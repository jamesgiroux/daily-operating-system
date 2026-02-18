---
name: action-awareness
description: "Contextual surfacing of commitments, follow-ups, and overdue items with zero-guilt framing"
---

# Action Awareness

This skill fires when commitments, tasks, follow-ups, or deadlines appear in conversation. It knows the action schema, detects new commitments in natural language, surfaces relevant items contextually, and uses zero-guilt framing when presenting overdue work.

## Activation Triggers

Activate when:
- User discusses tasks, follow-ups, or commitments
- User makes a commitment ("I'll send that by Friday", "Need to follow up with Sarah")
- An entity or person is mentioned that has associated actions
- User asks about what is due, overdue, or pending
- Meeting prep is being generated (open actions are relevant context)

## Action Schema

Actions in `data/actions.json` follow this structure:

```json
{
  "id": "unique-id",
  "text": "Send updated proposal with revised pricing",
  "entity": "Nielsen",
  "person": "Sarah Chen",
  "due_date": "2026-02-20",
  "status": "open",
  "source_meeting": "Nielsen Weekly Sync - Feb 14",
  "created_at": "2026-02-14T10:30:00Z"
}
```

**Status values:** open, in_progress, completed, overdue

An action is overdue when `status` is "open" or "in_progress" and `due_date` is in the past. Actions do not automatically update their status field — staleness is computed at read time.

## Contextual Surfacing

When actions are relevant to the current conversation, surface them naturally rather than dumping a full list.

### During Entity Discussion
When an entity is being discussed (entity-intelligence skill active), filter `data/actions.json` for that entity and surface:
- Overdue items first (these represent broken commitments)
- Items due this week (approaching deadlines)
- Recently completed items (momentum indicators)

### During Person Discussion
When a person is mentioned (relationship-context skill active), filter for actions where `person` matches and surface:
- What you owe them (actions you committed to for them)
- What they owe you (actions they committed to)
- Completion history (do they follow through? do you?)

### During Meeting Prep
When preparing for a meeting (meeting-intelligence skill active), surface:
- Open actions from previous meetings with these attendees
- Overdue items tied to the meeting's entity
- Commitments made at the last meeting with overlap attendees

## Zero-Guilt Framing

When surfacing overdue actions, inform without nagging. The purpose is awareness, not shame.

**Do this:**
- "There are 3 open items for Nielsen from the Feb 7 sync, including the pricing proposal that was due Tuesday."
- "You committed to the integration timeline for Sarah on Feb 10. It is still open."

**Do not do this:**
- "You have OVERDUE items that need IMMEDIATE attention!"
- "You failed to complete the following tasks..."
- "Reminder: these are past due and should have been done already."

The framing is: here is the state of things. The user decides what to do about it.

## New Commitment Detection

When the user makes statements that imply a trackable commitment, recognize them:

**Commitment language patterns:**
- "I'll [action] by [date]" — Clear commitment with deadline
- "Need to follow up with [person] about [topic]" — Follow-up action
- "Let's get [deliverable] to them by [date]" — Deliverable commitment
- "I promised [person] I would [action]" — Existing commitment being stated
- "Action item: [text]" — Explicit action creation
- "Can you track that I need to [action]" — Direct tracking request

When detected, acknowledge and offer to create:
"Got it — 'Send updated proposal to Sarah by Friday.' Want me to add this to the action tracker for Nielsen?"

Extract:
- `text` — The action description
- `entity` — From conversation context
- `person` — From the commitment language
- `due_date` — From the stated timeline (convert relative dates to absolute)
- `source_meeting` — If this came up during meeting discussion

## Connection to Entities and People

Actions are the connective tissue between entities, people, and meetings:

- Every action should ideally have both an `entity` and a `person` field
- When creating actions, infer entity and person from context if not explicitly stated
- When an entity's health is being assessed, action completion rate is a signal
- When a person's reliability is being evaluated, their action follow-through matters
- When a relationship is cooling, check if there are unresolved actions between the parties

## Interaction with Other Skills

- **entity-intelligence** provides entity context for filtering actions
- **relationship-context** provides person context and follow-through patterns
- **meeting-intelligence** uses actions for pre-meeting briefing and post-meeting capture
- **loop-back** handles writing new actions to `data/actions.json`
- **role-vocabulary** shapes how actions are described (urgency signals vary by preset)
