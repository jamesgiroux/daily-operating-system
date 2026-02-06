# Template: All Hands / Town Hall

## When Used

Applied when:
- Attendee count is 50 or more (scale-based override, highest priority in classification), OR
- Title contains "All Hands" or "Town Hall" (title-based override)

This template produces **minimal output by design**. All-hands meetings are broadcast format -- prep is done by the presenter, not the audience. Generating context, talking points, or action items for a company-wide meeting would be noise.

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, location/link

That is all. Do not read account files, archives, action lists, or stakeholder maps for this meeting type. Do not search for prior all-hands notes.

## Output Sections

### 1. Meeting Info

```
# All Hands
**Time:** 4:00 PM - 5:00 PM
**Location:** {Conference room name or video link from calendar event}
**Expected duration:** {duration from calendar}
```

### 2. Note

A single line:

> No prep needed -- this is a company-wide broadcast.

That is the entire output. Do not add sections, suggestions, or context.

## Formatting Guidelines

- First line: `# {Meeting Title}` (use the calendar title, e.g., "Q1 All Hands", "February Town Hall")
- Maximum output: 5 lines
- No bullet points, no tables, no analysis
- If the calendar description contains an agenda, do NOT reproduce it -- the organizer will present it

## Rationale

This template exists to prevent the system from wasting AI tokens on meetings that do not benefit from individual prep. It also prevents the dashboard from showing a prep card with empty or irrelevant content. The meeting should appear on the timeline with time and title only -- no expandable prep section.

## Profile Variations

- **CSM:** Same output. No differences.
- **General:** Same output. No differences.
