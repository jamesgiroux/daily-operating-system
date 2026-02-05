# Architecture Proposal: Hybrid JSON + Markdown

> Status: DRAFT - Pending approval

## Overview

This proposal addresses the fundamental mismatch between templates designed for human reading and the need for reliable machine parsing in the Daybreak app.

## Core Insight

**Templates were designed for humans. Apps need machine-readable data.**

Current pain points:
- Fragile regex parsing of markdown
- Inconsistent section formats between meeting types
- No schema validation
- Hard to extend without breaking parsers

## Proposed Architecture

### Directory Structure

```
_today/
├── data/                           # Machine-readable (new)
│   ├── manifest.json               # Index of today's files
│   ├── schedule.json               # Meetings with embedded prep summaries
│   ├── actions.json                # All actions with context
│   ├── emails.json                 # Email summary data
│   └── preps/                      # Individual meeting prep data
│       ├── 0900-acme-sync.json
│       └── 1400-internal-standup.json
│
├── 00-overview.md                  # Human-readable (generated from data/)
├── 01-0900-customer-acme-prep.md   # Human-readable (generated from data/)
├── 80-actions-due.md               # Human-readable (generated from data/)
└── 83-email-summary.md             # Human-readable (generated from data/)
```

### Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        Three-Phase Pattern                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Phase 1: Prepare (Python)                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ • Fetch calendar, emails, account data                   │   │
│  │ • Generate .today-directive.json                         │   │
│  │ • Output: Structured inputs for Claude                   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│  Phase 2: Enrich (Claude Code)                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ • Process directive with skills/agents                   │   │
│  │ • Generate enriched markdown content                     │   │
│  │ • Output: _today/*.md files (rich, human-readable)       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│                              ▼                                  │
│  Phase 3: Deliver (Python)                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ • Parse Claude's markdown output                         │   │
│  │ • Extract structured data into JSON schemas              │   │
│  │ • Write to _today/data/*.json                           │   │
│  │ • Regenerate markdown from JSON (canonical source)       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Why Phase 3 Does the Conversion

1. **Determinism boundary** - JSON generation is deterministic, testable
2. **Schema validation** - Python can validate against JSON Schema
3. **Error recovery** - If Claude outputs malformed markdown, Python can handle gracefully
4. **Extensibility** - Adding new fields doesn't require changing Claude's prompts

---

## JSON Schemas

### manifest.json

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "date": { "type": "string", "format": "date" },
    "generated_at": { "type": "string", "format": "date-time" },
    "files": {
      "type": "object",
      "properties": {
        "schedule": { "type": "string" },
        "actions": { "type": "string" },
        "emails": { "type": "string" },
        "preps": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "stats": {
      "type": "object",
      "properties": {
        "total_meetings": { "type": "integer" },
        "customer_meetings": { "type": "integer" },
        "actions_due": { "type": "integer" },
        "emails_flagged": { "type": "integer" }
      }
    }
  }
}
```

### schedule.json

```json
{
  "date": "2026-02-05",
  "greeting": "Good morning",
  "summary": "3 meetings today, 2 customer calls. Focus on renewal discussions.",
  "focus": "Acme renewal prep",
  "meetings": [
    {
      "id": "0900-acme-sync",
      "time": "9:00 AM",
      "end_time": "9:45 AM",
      "title": "Acme Weekly Sync",
      "type": "customer",
      "account": "Acme Corp",
      "has_prep": true,
      "prep_file": "preps/0900-acme-sync.json",
      "prep_summary": {
        "at_a_glance": ["Ring: 2", "ARR: $450K", "Health: Yellow"],
        "discuss": ["Renewal timeline", "Feature request status"],
        "watch": ["Champion leaving in Q2"],
        "wins": ["Expanded to 3 new teams"]
      }
    },
    {
      "id": "1400-standup",
      "time": "2:00 PM",
      "end_time": "2:30 PM",
      "title": "Team Standup",
      "type": "internal",
      "has_prep": true,
      "prep_file": "preps/1400-standup.json",
      "prep_summary": {
        "at_a_glance": ["Sprint Day 8", "3 PRs pending review"],
        "discuss": ["Blocker on auth flow", "QA feedback"],
        "watch": [],
        "wins": []
      }
    }
  ]
}
```

### preps/{meeting-id}.json

```json
{
  "meeting_id": "0900-acme-sync",
  "title": "Acme Weekly Sync",
  "time_range": "9:00 AM - 9:45 AM",
  "type": "customer",
  "account": "Acme Corp",

  "quick_context": {
    "Ring": "2",
    "ARR": "$450,000",
    "Contract End": "2026-06-30",
    "Health Score": "Yellow"
  },

  "attendees": [
    { "name": "Sarah Chen", "role": "VP Engineering", "focus": "Technical adoption" },
    { "name": "Mike Torres", "role": "Procurement", "focus": "Budget approval" }
  ],

  "since_last": [
    "Completed POC with Platform team",
    "Resolved authentication issues",
    "Training scheduled for March"
  ],

  "strategic_programs": [
    { "name": "Enterprise rollout", "status": "in_progress" },
    { "name": "API integration", "status": "completed" }
  ],

  "risks": [
    "Champion (Sarah) moving to new role in Q2",
    "Budget cycle ends March 15"
  ],

  "talking_points": [
    "Acknowledge POC success, ask about next steps",
    "Probe on Sarah's transition timeline",
    "Discuss renewal terms early"
  ],

  "open_items": [
    {
      "title": "Send API documentation",
      "due_date": "2026-02-07",
      "context": "Mike requested for procurement review",
      "is_overdue": false
    }
  ],

  "questions": [
    "What's the decision timeline for renewal?",
    "Who will be Sarah's replacement?"
  ],

  "key_principles": [
    "Don't rush the renewal conversation",
    "Focus on value delivered, not features"
  ],

  "references": [
    { "label": "Account Dashboard", "path": "3-resources/accounts/acme/dashboard.md" },
    { "label": "Last QBR Notes", "path": "archive/2026-01-15/acme-qbr.md" }
  ]
}
```

### actions.json

```json
{
  "date": "2026-02-05",
  "summary": {
    "overdue": 2,
    "due_today": 3,
    "due_this_week": 8
  },
  "actions": [
    {
      "id": "action-001",
      "title": "Send API documentation to Acme",
      "account": "Acme Corp",
      "priority": "P1",
      "status": "pending",
      "due_date": "2026-02-07",
      "is_overdue": false,
      "context": "Mike from procurement needs this for budget approval process",
      "source": "Acme Weekly Sync (2026-02-03)"
    },
    {
      "id": "action-002",
      "title": "Review Beta Corp renewal proposal",
      "account": "Beta Corp",
      "priority": "P1",
      "status": "pending",
      "due_date": "2026-02-04",
      "is_overdue": true,
      "days_overdue": 1,
      "context": "Legal needs sign-off before sending to customer",
      "source": "Email from Jane (2026-02-01)"
    }
  ]
}
```

### emails.json

```json
{
  "date": "2026-02-05",
  "stats": {
    "high_priority": 3,
    "normal_priority": 12,
    "needs_action": 5
  },
  "emails": [
    {
      "id": "email-001",
      "sender": "Sarah Chen",
      "sender_email": "sarah@acme.com",
      "subject": "RE: Renewal Discussion",
      "priority": "high",
      "snippet": "Thanks for the proposal. We have a few questions...",
      "received": "2026-02-05T08:23:00Z",
      "email_type": "customer_response",
      "conversation_arc": "Third exchange in renewal negotiation",
      "recommended_action": "Reply with clarifications today",
      "action_owner": "You"
    }
  ]
}
```

---

## Rust Parser Changes

With JSON files, the Rust parser becomes trivial:

```rust
// Before: Fragile regex parsing
fn parse_overview(path: &Path) -> Result<Overview, String> {
    let content = fs::read_to_string(path)?;
    // 100+ lines of regex matching...
}

// After: Simple JSON deserialization
fn load_schedule(data_dir: &Path) -> Result<Schedule, String> {
    let path = data_dir.join("schedule.json");
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read schedule: {}", e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse schedule: {}", e))
}
```

---

## Migration Strategy

### Phase 1: Dual-Write (Non-Breaking)

1. Update Phase 3 Python scripts to:
   - Continue generating markdown as before
   - Additionally write JSON to `_today/data/`

2. Update Rust parser to:
   - Check for `_today/data/` first
   - Fall back to markdown parsing if not found

### Phase 2: JSON-Primary

1. Make JSON the source of truth
2. Generate markdown FROM JSON (for human reading)
3. Deprecate markdown parsing in Rust

### Phase 3: Clean Up

1. Remove markdown parsing code from Rust
2. Simplify Phase 3 Python (no more dual formats)

---

## Archive Handling

**Decision: Archives remain markdown-only**

Rationale:
- Archives are for historical reference, not active consumption
- Keeps archive portable and human-readable
- No need to migrate historical data

If someone needs to load historical data programmatically, they can:
1. Use the markdown parser (kept for backwards compatibility)
2. Or re-run Phase 3 on archived markdown to generate JSON

---

## Benefits

| Before | After |
|--------|-------|
| Fragile regex parsing | Typed JSON deserialization |
| Inconsistent formats | Schema-validated structure |
| Hard to extend | Add fields without breaking parsers |
| Parse errors at runtime | Validation at generation time |
| 500+ lines of parsing code | ~50 lines of serde |

---

## Open Questions

1. **Schema versioning**: How do we handle schema changes over time?
   - Proposal: Include `schema_version` in manifest.json

2. **Partial generation**: What if Phase 2 fails mid-way?
   - Proposal: Phase 3 validates completeness, marks manifest as `partial: true`

3. **Real-time updates**: When file watching is added, do we re-parse or use events?
   - Proposal: File watcher triggers re-read of specific JSON file

---

## Next Steps

1. [ ] Create JSON Schema files in `templates/schemas/`
2. [ ] Update `deliver_today.py` to write JSON alongside markdown
3. [ ] Simplify Rust parser to read JSON with markdown fallback
4. [ ] Test with existing workspace data
5. [ ] Document schema for template authors

---

*Proposed: 2026-02-05*
*Status: Awaiting approval*
