# Template: Internal Team Sync

## When Used

Applied when all attendees are internal (same email domain as user) and the meeting is either:
- A recurring sync/standup (title contains "sync", "standup", "scrum", "daily"), OR
- A group meeting (3+ internal attendees) that does not match any title-based override (QBR, training, all-hands)

This is intentionally light prep. The goal is not to over-prepare for routine team meetings but to give you just enough context to not walk in cold.

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, attendee list
2. **Last meeting recap** -- Search archive for the most recent meeting with the same title or same attendee set. Look for notes, decisions, or action items.
3. **Your open action items** -- From master task list, filtered to items owned by the user that are either overdue or due within the next 3 days
4. **Project status** (if meeting title references a project) -- Check `Projects/{name}/` for any status files

Claude should spend minimal time on this template. If context files do not exist, produce the skeleton sections with placeholder prompts rather than searching extensively.

## Output Sections

### 1. Last Meeting Recap

If a prior meeting summary exists in the archive:

**Last sync:** {date}
- Decision: Moving to bi-weekly releases
- Action: Sarah to draft new process doc (status: unknown)
- Discussion: Budget reallocation for Q2

If no prior meeting is found: "No previous meeting notes found in archive."

Keep to 3-5 bullet points maximum. Prioritize decisions and open action items over discussion summaries.

### 2. My Updates

A structured prompt for the user to mentally review before the meeting:

- **Completed since last sync:** {list items from recently completed actions, or leave as prompt}
- **In progress:** {current focus items}
- **Blocked:** {any blockers to raise}

If recent action completions can be identified from the task list, populate the "Completed" section. Otherwise, leave as prompts: "(Review your recent work and note 2-3 items.)"

### 3. Discussion Topics

Items worth raising in this meeting, sourced from:

- Overdue actions assigned to meeting attendees
- Decisions flagged as pending in prior meeting notes
- Cross-cutting items from your current project work

If nothing specific is found, write: "No flagged topics. Check if there's a standing agenda."

### 4. Open Actions (Mine)

Action items owned by the user that are relevant to this meeting's attendees or topics:

- [ ] Review PR #234 (due: today)
- [ ] Update Q1 forecast (due: Feb 7)

Filter to items that are overdue or due within 3 days. If there are more than 5, show the top 5 by urgency and note "(+N more)".

If no relevant actions: "No actions due. You're clear."

## Formatting Guidelines

- First line: `# {Meeting Title}`
- Second line: `**Time:** {start} - {end}`
- Target length: 150-200 words maximum
- Use bullet points, not paragraphs -- this should scan in 30 seconds
- No strategic analysis, no deep context -- just the facts
- If the meeting is a daily standup, sections 1 and 3 can be omitted entirely (just show "My Updates" and "Open Actions")

## Profile Variations

- **CSM:** Same template. If the sync involves discussing specific accounts (e.g., "CS Team Sync"), you may add a one-liner per account mentioned in prior meeting notes: "Acme: renewal conversation started. GlobalTech: escalation resolved." Do not pull full account context.

- **General:** Same template. No differences. Internal syncs are profile-agnostic.
