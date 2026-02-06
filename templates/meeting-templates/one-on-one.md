# Template: One-on-One

## When Used

Applied when a meeting has exactly 2 attendees (including the user) and both are internal. May also be triggered by title keywords: "1:1", "1-1", "One on One".

1:1s are personal and relationship-focused. This template treats them differently from all other meeting types: the tone is reflective, not strategic. The prep should help the user have a meaningful conversation, not execute an agenda.

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, the other attendee's name and email
2. **Past 1:1 notes** -- Search archive for the 2-3 most recent meetings with the same attendee pair. Look for patterns, recurring themes, and open threads.
3. **Recent feedback items** -- Any notes tagged as feedback, recognition, or development for this person (if such files exist in the workspace)
4. **Shared action items** -- From master task list, items where the other person is owner or collaborator

This is sensitive context. Claude should:
- Report observations, not judgments
- Use neutral language ("mentioned feeling stretched" not "complained about workload")
- Never fabricate personal context that is not in the source files

## Output Sections

### 1. Topics to Discuss

Suggested discussion topics drawn from prior 1:1 notes and current context. Categorize by type:

**Carry-forward items:**
- Follow up on promotion path discussion from Jan 29
- Check in on Project X workload concern raised two weeks ago

**New items:**
- {Any recent changes, milestones, or events relevant to this person}

If no prior 1:1 notes exist: "First tracked 1:1 with {name}. Consider asking about: current priorities, how they're feeling about workload, and what support they need."

### 2. Recent Context

What has happened since the last 1:1 that might be relevant:

- **Last 1:1:** {date} -- {brief summary of key topics}
- **Since then:** {any notable events: project completions, team changes, deadlines passed}

Keep to 2-4 bullet points. This is background, not an agenda.

### 3. Their Open Items

Action items or commitments the other person owns:

- Draft new process doc (from team sync, due: Feb 10)
- Complete onboarding for new hire (no due date)

This helps you ask informed questions without micromanaging. Present as awareness, not accountability.

If no items found: "No tracked action items for {name}."

### 4. Support and Development

Themes from prior 1:1s related to growth, challenges, or support needs:

- **Career theme:** Interested in moving toward architecture role (discussed Nov, Jan)
- **Workload:** Has mentioned feeling stretched across 3 projects (Jan 15, Jan 29)
- **Wins to recognize:** Led the migration project successfully

If no prior development themes exist, omit this section entirely. Do not generate speculative career advice.

## Formatting Guidelines

- First line: `# 1:1 with {Name}`
- Second line: `**Time:** {start} - {end}`
- Target length: 150-250 words
- Tone: Warm, observational, non-directive
- Use "they" pronouns unless the person's pronouns are known from context
- Do not include metrics, KPIs, or performance ratings
- Do not structure this like a performance review -- it is a conversation guide

## Sensitivity Rules

These rules override all other formatting guidelines:

1. **Never assess performance.** Report what the person said or did, not how well they did it.
2. **Never diagnose emotions.** "Mentioned feeling stretched" is fine. "Seems burned out" is not.
3. **Never recommend personnel actions.** No suggestions about PIPs, promotions, or role changes unless the user explicitly documented those discussions in prior notes.
4. **Attribute sources.** "Per your Jan 29 notes..." not "They seem to want..."
5. **When in doubt, omit.** If a piece of context feels too personal or inferential, leave it out.

## Profile Variations

- **CSM:** Same template. No differences. 1:1s are about people, not accounts.

- **General:** Same template. No differences. 1:1 prep is profile-agnostic.
