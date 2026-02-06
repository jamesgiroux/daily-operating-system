# Template: Training / Enablement Session

## When Used

Applied when the meeting title contains "Training", "Enablement", or "Workshop" (case-insensitive). Title keywords override attendee-based classification -- an internal workshop uses this template just as an external customer training does.

This template covers two scenarios:
- **External training:** Delivering product training to a customer or partner audience
- **Internal training:** Participating in or delivering an internal workshop, learning session, or enablement event

The distinction is determined by whether external attendees are present on the invite.

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, attendee list, calendar description (often contains the training agenda or topic)
2. **Attendee list** -- All attendees with internal/external classification
3. **Previous training sessions** -- Search archive for prior meetings with same account/title containing "training" or "workshop" keywords
4. **Account dashboard** (external training only) -- `Accounts/{account}/dashboard.md` -- For product adoption metrics context
5. **Pre-work materials** -- Any files referenced in the calendar description or recently added to the account folder

## Output Sections

### 1. Session Overview

Brief summary of what this training session covers. Pull from the calendar description if available. If no description exists, infer from the title and attendee context.

- **Topic:** Advanced Reporting (from title/description)
- **Format:** {Live demo / Workshop / Presentation / Hands-on lab} (infer from title keywords)
- **Duration:** {time range}
- **Audience:** {team name or role} ({count} attendees)

### 2. Attendee Readiness

For external training:
- What training sessions has this group completed previously?
- Any new attendees who may need extra context?
- Skill level assessment: Beginner / Intermediate / Advanced (infer from training history)

For internal training:
- Who is presenting vs. attending?
- Any pre-work that was assigned?

If no prior training history exists: "First tracked training session with this group."

### 3. Key Objectives

3-5 learning objectives for the session. If the calendar description lists objectives, use those. Otherwise, generate reasonable objectives based on the topic and audience level.

1. Build custom reports using advanced filters
2. Configure automated report scheduling
3. Export and integrate data with external tools

Frame as outcomes ("Attendees will be able to...") not activities ("We will cover...").

### 4. Materials Checklist

A practical checklist of preparation items:

- [ ] Demo environment ready and tested
- [ ] Slide deck / materials updated for this audience
- [ ] Recording set up (if applicable)
- [ ] Handouts or follow-up resources prepared
- [ ] Backup plan if live demo fails

This section is always generated. Adjust items based on the training format (demo vs. workshop vs. presentation).

### 5. Context from Previous Sessions (if available)

If prior training sessions are found in the archive:

- **Last session:** {date} -- {topic}
- **Key takeaways:** {what was covered}
- **Follow-up items from last session:**
  - [ ] Share report template library (still open)
  - [ ] Send recording of Dashboards session (completed Jan 25)

If no prior sessions exist, omit this section entirely.

### 6. Post-Session Follow-up Plan

Suggested follow-up actions to generate after the training:

- Send recording and materials to attendees
- Schedule next session (if part of a series)
- Check adoption metrics in 2 weeks
- Capture feedback on session effectiveness

This is a reminder list, not something to present during the meeting.

## Formatting Guidelines

- First line: `# {Account Name or Team} - Training: {Topic}`
- Second line: `**Time:** {start} - {end}`
- Keep prep under 300 words -- training prep should be quick to scan
- Focus on logistics and readiness, not deep strategy
- The checklist format is intentional -- training prep is task-oriented

## Profile Variations

- **CSM:** Include product adoption metrics from the account dashboard (e.g., "Current feature adoption: 45% of licensed users active on reporting module"). Link training objectives to adoption goals. If this is part of an onboarding sequence, note where it falls in the overall enablement plan. Reference any strategic programs that depend on training completion.

- **General:** Skip adoption metrics. Focus on session logistics, objectives, and materials readiness. For internal workshops, emphasize pre-work completion and your role (presenter vs. participant). If you are attending (not delivering), the prep is lighter: just objectives, pre-work status, and any questions you want to ask.
