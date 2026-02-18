---
description: "Process raw inputs into workspace-native artifacts"
---

# /capture

Process raw inputs — transcripts, notes, email threads, research documents — into structured workspace artifacts. Extract actions, update people profiles, refresh entity intelligence, and archive the processed output.

## Arguments

- `$ARGUMENTS[0]` — Input file path or content (optional). If omitted, scan `_inbox/` for unprocessed files.

## Workflow

### Step 1: Identify Input

**If a file path is provided:**
- Read the specified file
- Detect input type from content and filename

**If content is pasted directly:**
- Accept inline content
- Detect input type from content structure

**If no argument provided:**
- Scan `_inbox/` for files
- List what is found: "Found 3 items in _inbox/: meeting-transcript-2026-02-14.txt, research-notes.md, email-thread-nielsen.eml. Which should I process?"
- If only one item, proceed with it after confirmation

### Step 2: Classify Input Type

Detect the input type to determine processing strategy:

| Input Type | Indicators | Processing Focus |
|---|---|---|
| **Meeting transcript** | Timestamps, speaker labels, conversational format | Actions, decisions, sentiment, attendee signals |
| **Meeting notes** | Bullet points, agenda items, shorter format | Actions, decisions, key takeaways |
| **Email thread** | From/To/Subject headers, reply chains | Requests, commitments, sentiment, urgency signals |
| **Research document** | Structured prose, external sources, analysis | Entity intelligence, competitive data, market context |
| **Generic document** | Does not match above patterns | Best-effort extraction based on content |

### Step 3: Extract Structured Data

Process the input and extract:

**Actions:**
- Scan for commitment language ("I'll...", "We need to...", "Action item:", "By [date]...")
- For each action, extract: text, person (who committed), entity (if identifiable), due_date (if stated)
- Convert relative dates to absolute ("next Friday" becomes specific date)
- Distinguish between actions assigned to the user vs. actions assigned to others

**People mentions:**
- Identify all people mentioned by name
- Cross-reference against `People/` directory
- For known people: note new signals (what they said, how they engaged, any sentiment indicators)
- For unknown people: flag as potential new profiles

**Entity references:**
- Identify entities mentioned (company names, project names)
- Cross-reference against `Accounts/` and `Projects/`
- Extract entity-relevant signals: health indicators, risk mentions, win mentions, strategic shifts

**Decisions:**
- Capture explicit decisions made ("We decided to...", "The plan is...", "Going with...")
- Note who made the decision and who was present
- Link decisions to entities and actions

**Signals:**
- Sentiment indicators (positive momentum, concern, frustration, enthusiasm)
- Relationship signals (warmth, distance, engagement shifts)
- Risk signals (timeline pressure, resource constraints, competitive mentions)
- Opportunity signals (expansion interest, new use cases, referral willingness)

### Step 4: Route to Workspace Locations

Determine where each extracted artifact should go:

**Meeting summaries:**
- Destination: `_archive/YYYY-MM/{meeting-title}-{YYYY-MM-DD}.md`
- Format: Structured markdown with Date, Attendees, Key Discussion Points, Decisions, Actions, Signals
- Use the meeting date for the archive path, not today's date

**Actions:**
- Destination: `data/actions.json` (append to existing array)
- Each action formatted per schema: id, text, entity, person, due_date, status ("open"), source_meeting

**People intelligence updates:**
- Destination: `People/{name}/person.md` (append to relevant sections)
- New meeting signals, temperature observations, communication data points

**Entity intelligence updates:**
- Destination: `{entity-path}/intelligence.json` (merge with existing)
- New risks, wins, state changes, stakeholder insights

### Step 5: Generate Capture Report

Present a summary of everything extracted before writing anything:

```markdown
## Capture Report: Nielsen Weekly Sync — Feb 14

### Source
Meeting transcript, 47 minutes, 4 attendees

### Extracted Actions (5)
1. "Send updated proposal with revised pricing" — You, for Nielsen, due Feb 20
2. "Share API documentation with engineering team" — Sarah Chen, for Nielsen, due Feb 18
3. "Schedule EBR with executive sponsors" — You, for Nielsen, due Feb 28
4. "Review integration test results" — David Park, due Feb 21
5. "Prepare adoption metrics deck for QBR" — You, for Nielsen, due Mar 3

### People Signals
- **Sarah Chen** — Engaged, asked detailed questions about roadmap. Temperature: warming.
- **David Park** — Joined late, left early, delegated action to his team. Temperature: cooling signal.
- **Elena Rodriguez** — First time attending weekly sync. Signal: increasing involvement.

### Entity Intelligence
- **Nielsen** — Expansion discussion initiated by their side (positive signal). Integration timeline is the primary dependency for renewal. Competitive mention: they are evaluating Datadog for monitoring layer.

### Decisions
- Pricing revision approved — moving to volume-based model
- QBR date confirmed for March 8

### Signals
- Positive: Expansion interest, champion engagement strong
- Watch: David's disengagement pattern continues, competitive evaluation in adjacent space
```

### Step 6: Confirm Before Writing

Present the capture report and ask for confirmation:

```
Ready to write:
1. Archive summary to _archive/2026-02/nielsen-weekly-sync-2026-02-14.md
2. Create 5 actions in data/actions.json
3. Update People/Sarah-Chen/person.md with warming signal
4. Update People/David-Park/person.md with cooling signal
5. Add Elena Rodriguez to People/ (new profile)
6. Update Accounts/Nielsen/intelligence.json with expansion signal and competitive mention

Proceed with all, or adjust first?
```

Never write back to workspace files without explicit user confirmation. The user may want to modify, skip certain items, or change routing.

### Step 7: Execute Writes

After confirmation, execute the approved writes:
- Read existing files before modifying (preserve content, append/merge)
- Create new directories as needed (_archive/YYYY-MM/, new People/ profiles)
- For JSON files (actions.json, intelligence.json), merge carefully — do not overwrite existing entries
- Report completion: "Done. 5 actions created, 3 people profiles updated, Nielsen intelligence refreshed, summary archived."

## Loop-Back

The entire command is a loop-back operation — it processes raw input into workspace artifacts. The confirmation step (Step 6) is the loop-back gate. All writes are offered, none are forced.

## Skills That Contribute

- **workspace-fluency** — Provides file structure knowledge for routing
- **entity-intelligence** — Auto-fires when entity names are detected in the input
- **relationship-context** — Auto-fires when people are mentioned, informing signal interpretation
- **action-awareness** — Guides action extraction and formatting
- **meeting-intelligence** — Informs meeting transcript processing and template detection
- **loop-back** — The routing and confirmation workflow
