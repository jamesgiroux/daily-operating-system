---
name: inbox
description: Three-phase document processing workflow that transforms raw inputs into organized, enriched documents. Processes transcripts, meeting notes, and other documents from inbox to PARA locations.
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Task
---

# Inbox Processing Skill (/inbox)

A three-phase document processing workflow that transforms raw inputs into organized, enriched documents in your PARA structure.

## Overview

This skill processes documents from your `_inbox/` directory through a systematic workflow:

1. **Phase 1: Preparation** - Python script analyzes files and generates agent directives
2. **Phase 2: Enrichment** - Claude processes files according to directives (summaries, actions, tags)
3. **Phase 3: Delivery** - Python script moves processed files to canonical PARA locations

## Philosophy

**Inbox zero for documents** - Just like email, your document inbox should be systematically processed.

**Enrich before filing** - Documents gain value through summarization, tagging, and action extraction.

**Canonical locations** - Every document has one correct home in your PARA structure.

## When to Use

Invoke this skill when:
- You have unprocessed documents in `_inbox/`
- `/today` or `/wrap` flags unprocessed transcripts
- You receive new documents that need classification
- You want to batch-process a collection of files

## Quick Start

```
"Process the inbox"
```

or

```
"/inbox"
```

## Document Types Handled

| Type | Detection | Enrichment | Destination |
|------|-----------|------------|-------------|
| **Transcripts** | `*transcript*.md`, `*call*.md` | Summary, actions, decisions | `[Account]/meetings/` |
| **Meeting Notes** | `*notes*.md`, `*summary*.md` | Format, actions | `[Account]/meetings/` |
| **Strategy Docs** | `*strategy*.md`, `*analysis*.md` | Executive summary, key insights | `Projects/` or `Resources/` |
| **Reports** | `*report*.md` | Summary, highlights | Context-dependent |
| **General** | Other `.md` files | Classification, tagging | PARA-based routing |

## Phase Details

### Phase 1: Preparation

Run the preparation script:

```bash
python3 _tools/prepare_inbox.py
```

The script:
1. Scans `_inbox/` for processable files
2. Analyzes each file for type, context, and routing hints
3. Generates `.processing-state.json` with agent directives
4. Creates backup of original files

**Output:**
```json
{
  "files": [
    {
      "path": "_inbox/2026-01-15-client-transcript.md",
      "type": "transcript",
      "account": "ClientName",
      "date": "2026-01-15",
      "directives": {
        "summarize": true,
        "extract_actions": true,
        "extract_decisions": true,
        "destination": "Accounts/ClientName/meetings/"
      }
    }
  ],
  "prepared_at": "2026-01-15T10:30:00"
}
```

### Phase 2: Enrichment

Claude processes each file according to directives:

#### For Transcripts

**Input**: Raw meeting transcript
**Output**: Enriched document with:

```markdown
---
account: [Account Name]
date: [YYYY-MM-DD]
meeting_type: [sync/review/planning/etc]
attendees: [List]
doc_type: meeting-summary
tags: [relevant, tags]
---

# [Meeting Title] - [Date]

## Executive Summary

[2-3 sentence summary of the meeting]

## Key Discussion Points

### [Topic 1]
- [Point]
- [Point]

### [Topic 2]
- [Point]

## Decisions Made

- **[Decision 1]**: [Details]
- **[Decision 2]**: [Details]

## Action Items

- [ ] **[Action]** - Owner: [Name] - Due: [Date]
- [ ] **[Action]** - Owner: [Name] - Due: [Date]

## Next Steps

- [Next step]

## Raw Transcript

[Original transcript preserved below or linked]
```

#### For Strategy Documents

**Input**: Strategy analysis or planning document
**Output**: Enriched with executive summary and key insights

#### For General Documents

**Input**: Any document
**Output**: Classified, tagged, and routed

### Phase 3: Delivery

Run the delivery script:

```bash
python3 _tools/deliver_inbox.py
```

The script:
1. Reads `.processing-state.json`
2. Verifies enrichment completed
3. Moves files to canonical destinations
4. Updates any indexes or dashboards
5. Archives processing state

## Frontmatter Standards

All processed documents receive standardized frontmatter:

```yaml
---
# Required
doc_type: [meeting-summary|transcript|strategy|report|note]
date: YYYY-MM-DD
tags: []

# Context-dependent
account: [Account name if customer-related]
project: [Project name if project-related]
area: [Area if area-related]

# Processing metadata
source: inbox-processing
processed_date: YYYY-MM-DD
original_file: [original filename]
---
```

## PARA Routing Rules

Documents are routed based on:

1. **Explicit account/project reference** → That account/project folder
2. **Meeting with external attendees** → Account folder
3. **Internal project meeting** → Project folder
4. **Strategy/reference material** → Resources/
5. **Historical/completed** → Archive/

## Agents Used

This skill coordinates with these agents:

| Agent | Purpose |
|-------|---------|
| **file-organizer** | Determines canonical location for each file |
| **integration-linker** | Creates links to external systems (tickets, issues) |

## Error Handling

**If account not found:**
- Create minimal structure in `Accounts/[Name]/`
- Route file there
- Flag for review

**If enrichment incomplete:**
- Keep in inbox
- Add `.needs-review` suffix
- Note in daily overview

**If duplicate detected:**
- Compare content
- Merge if appropriate
- Archive older version

## Integration with Commands

| Command | Integration |
|---------|-------------|
| `/today` | Flags unprocessed transcripts in overview |
| `/wrap` | Offers to process day's transcripts |
| `/week` | Processes week's accumulated files |

## Configuration

### File Type Patterns

Configure in `_tools/inbox-config.yaml`:

```yaml
file_patterns:
  transcript:
    - "*transcript*.md"
    - "*call*.md"
    - "*recording*.md"
  notes:
    - "*notes*.md"
    - "*summary*.md"
  strategy:
    - "*strategy*.md"
    - "*analysis*.md"
    - "*proposal*.md"
```

### Account Detection

Configure domain-to-account mapping for automatic routing:

```yaml
account_domains:
  "clienta.com": "Client A"
  "clientb.org": "Client B"
```

## Output Structure

After processing:

```
_inbox/
├── .processing-state.json    # Current state (cleared after delivery)
└── [new unprocessed files]

Accounts/ClientName/meetings/
└── 2026-01-15-summary-clientname-sync.md    # Delivered

_archive/inbox-processing/
└── 2026-01-15-batch/         # Archived processing records
    ├── processing-state.json
    └── [original file backups]
```

## Best Practices

1. **Process daily** - Don't let inbox accumulate
2. **Review enrichment** - Verify summaries and actions are accurate
3. **Maintain mappings** - Keep account domain mappings current
4. **Archive originals** - Keep raw transcripts for reference

## Troubleshooting

**Files stuck in inbox:**
- Check `.processing-state.json` for errors
- Verify enrichment directives completed
- Run delivery script manually

**Wrong destination:**
- Check account/project detection rules
- Update domain mappings
- Move manually and update indexes
