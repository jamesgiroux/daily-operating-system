# Template: Customer Call

## When Used

Applied when the meeting classification returns `customer` -- meaning at least one external attendee matches a known account contact in the workspace's `Accounts/` directory. This is the most common prep-intensive template for CSM profiles.

## Required Context (from directive refs)

Claude should read the following files referenced in the directive's `meeting_contexts` entry for this meeting:

1. **Account dashboard** -- `Accounts/{account}/dashboard.md` -- ARR, ring, health, renewal date, strategic programs
2. **Recent meeting summaries** -- Last 2-3 files matching `_archive/*/XX-HHMM-*{account}*` -- What was discussed previously
3. **Stakeholder map** -- `Accounts/{account}/stakeholders.md` (if exists) -- Roles, champions, detractors
4. **Account actions** -- `Accounts/{account}/actions.md` or entries in master task list filtered by account -- Open items, overdue tasks
5. **Meeting event data** -- From directive JSON: title, time, duration, attendee list, calendar description

Do NOT read files that are not referenced. If a ref path does not exist, skip that section gracefully and note the gap.

## Output Sections

Generate the following sections in order. Every section is required unless marked optional.

### 1. Context Paragraph

A 2-3 sentence narrative summary of the account state and what matters going into this meeting. Written in second person ("You're meeting with..."). Should mention the most salient fact: a risk, a recent win, a renewal approaching, or a relationship shift.

Example tone: "You're meeting with Acme Corp's engineering leads. Since your last call two weeks ago, the Phase 2 rollout completed successfully. The main thing to watch: Sarah Chen is transitioning roles in Q2 and you don't have a confirmed successor champion."

### 2. Quick Context (CSM only)

A key-value table of account health metrics:

| Metric | Value |
|--------|-------|
| Ring | {1-4} |
| ARR | ${amount} |
| Health | {Green/Yellow/Red} |
| Renewal | {date} |

If any metric is unavailable from the dashboard, omit the row rather than guessing.

### 3. Recent History

Bullet list of 3-5 items summarizing what happened since the last meeting with this account. Source from archived meeting summaries and account actions. Include dates.

- Completed POC with Platform team (Jan 28)
- Resolved authentication blockers (Jan 30)
- Training scheduled for March (Feb 1)

If no prior meeting summaries exist in the archive, write: "No prior meeting summaries found in archive. This may be a first tracked meeting with this account."

### 4. Risks and Wins

Two sub-sections:

**Wins** -- Recent positive outcomes worth acknowledging. These build relationship capital.

**Risks** -- Current concerns that may come up or should be proactively addressed. Pull from dashboard health indicators, overdue actions, or flagged items.

Keep each list to 2-3 items maximum. If there are no risks, say "No active risks flagged." Do not fabricate risks.

### 5. Open Actions

Action items related to this account, formatted as a checklist:

- [ ] Send API documentation (due: Feb 7) -- Mike requested
- [ ] Schedule SSO planning call with IT (no due date)

Mark overdue items with `OVERDUE:` prefix. Sort overdue first, then by due date.

If no actions exist for this account, write: "No open actions tracked for this account."

### 6. Discussion Points

3-5 suggested talking points synthesized from the context. These should be strategic, not just status updates. Each point should have a brief rationale.

1. **Acknowledge POC success, explore expansion** -- They invested effort; recognize it and plant seeds
2. **Probe on champion transition** -- Sarah's move creates risk; need to identify successor early
3. **Renewal conversation timing** -- 4 months out; appropriate to start framing

### 7. Stakeholders Attending

For each attendee on the calendar invite who appears in the stakeholder map:

- **Sarah Chen** (VP Engineering) -- Technical champion, drives adoption decisions
- **Mike Torres** (Procurement) -- Budget authority, detail-oriented

For attendees NOT in the stakeholder map, list them separately:

- **Unknown:** j.smith@acme.com -- Not in stakeholder map

## Formatting Guidelines

- Use markdown headers (`##`) for each section
- The first line should be `# {Account Name} - {Meeting Title}`
- Second line: `**Time:** {start} - {end}`
- Keep the entire prep under 400 words (excluding the Quick Context table)
- Write in present tense, second person ("You're meeting with...", "Watch for...")
- Do not include preamble like "Here's your meeting prep" -- start with the content directly

## Profile Variations

- **CSM:** Include all 7 sections. Quick Context table includes ARR, ring, health badge, renewal date. Discussion points should reference strategic programs and renewal positioning. If the account has a `strategy.md` or `programs.md`, reference those programs in the context paragraph.

- **General:** Skip section 2 (Quick Context). Reduce section 3 to "last interaction" rather than full meeting history. Discussion points focus on attendee context and meeting objectives rather than account metrics. The account directory may not exist for General profile; rely on archive search results and attendee information from the calendar event.
