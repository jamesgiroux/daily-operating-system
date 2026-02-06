# Prepare Phase Architecture

> How we collect, classify, and structure data before AI enrichment.

---

## Reference Approach (Key Design Decision)

**Directive contains references, not embedded content.**

Rather than dumping all context into a massive directive JSON, we provide Claude with file paths and references. Claude builds its own context by reading referenced files as needed.

| Approach | Directive Size | Claude Control | Flexibility |
|----------|----------------|----------------|-------------|
| Embedded (old) | Large | None | Fixed depth |
| **Reference (new)** | Compact | Full | Selective |

**What goes inline vs by reference:**

| Inline (in directive) | By Reference (file paths) |
|----------------------|---------------------------|
| Key metrics (ARR, ring, health) | Full account dashboard |
| Stakeholder names + roles | Detailed stakeholder notes |
| Meeting title + time | Full meeting history notes |
| Action titles | Action context and history |
| Email subject + sender | Full email thread |

**Example reference pattern:**

```json
{
  "context": {
    "account_metrics": {
      "ring": 2,
      "arr": "$450,000",
      "health": "yellow"
    },
    "refs": {
      "account_dashboard": "2-areas/accounts/acme-corp/dashboard.md",
      "meeting_history": [
        "4-archive/2026-01-28/01-0900-customer-acme-notes.md",
        "4-archive/2026-01-21/02-1400-customer-acme-notes.md"
      ],
      "stakeholder_map": "2-areas/accounts/acme-corp/stakeholders.md"
    }
  }
}
```

Claude then reads the files it needs based on meeting type and prep depth.

---

## Overview

The prepare phase is **Phase 1** of the three-phase pattern. It handles all deterministic operations:

1. Fetch data from external APIs (Calendar, Gmail, Sheets)
2. Read local workspace files (actions, accounts, meeting history)
3. Classify meetings using multi-signal logic
4. Gather historical context for each meeting
5. Output a structured directive JSON for Phase 2

**Key principle:** No AI in Phase 1. Everything is deterministic and testable.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     PREPARE PHASE ORCHESTRATOR                   │
│                      (prepare_today.py)                          │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  DATA FETCHERS │   │  CLASSIFIERS  │   │   CONTEXT     │
│               │   │               │   │   GATHERERS   │
│ • Calendar    │   │ • Meeting     │   │               │
│ • Gmail       │   │   classifier  │   │ • History     │
│ • Sheets      │   │ • Email       │   │ • Account     │
│ • SQLite      │   │   prioritizer │   │ • Actions     │
└───────────────┘   └───────────────┘   └───────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                              ▼
                ┌─────────────────────────┐
                │   DIRECTIVE BUILDER     │
                │                         │
                │ .today-directive.json   │
                └─────────────────────────┘
```

---

## Data Flow

### Step 1: Fetch External Data

```python
# Parallel fetches (independent operations)
calendar_events = fetch_calendar_events(days=1)  # Today's meetings
emails = fetch_emails(max_results=30)            # Unread inbox
account_data = fetch_account_data()              # From Sheets (CSM profile)
```

### Step 2: Classify Meetings

For each calendar event, run the multi-signal classifier:

```python
for event in calendar_events:
    classification = classify_meeting(
        event=event,
        user_domain=user_domain,
        accounts=account_data,        # CSM profile only
        partners=partner_data,        # If configured
        history_db=history_db         # SQLite for lookups
    )
```

### Step 3: Gather Context (Per Meeting)

For each classified meeting, gather relevant context:

```python
for meeting in classified_meetings:
    if meeting.type in ['customer', 'qbr', 'training']:
        context = gather_customer_context(meeting, account_data, history_db)
    elif meeting.type == 'one_on_one':
        context = gather_1on1_context(meeting, history_db)
    elif meeting.type == 'team_sync':
        context = gather_internal_context(meeting, history_db)
    # ... etc
```

### Step 4: Build Directive

Assemble all data into the directive JSON structure.

---

## Meeting Classification

The classifier uses multi-signal logic to determine meeting type, prep template, and prep depth.

**Full algorithm and implementation:** See `MEETING-TYPES.md` (canonical source).

**Summary of classification priority:**
1. Attendee count (50+ → All Hands)
2. Title keywords (QBR, Training, All Hands overrides)
3. External attendee cross-reference (accounts, partners, unknown)
4. Internal heuristics (1:1, team sync, default internal)

---

## Profile-Aware Data Collection

### CSM Profile

CSM users have accounts. The prepare phase:

1. **Loads account tracker** from `2-areas/accounts/_tracker.csv`
2. **Builds contact lookup** for meeting classification
3. **Gathers account-specific context** for customer meetings

```python
class CSMDataCollector:
    def __init__(self, workspace_path: Path):
        self.account_tracker = self.load_tracker()
        self.contact_lookup = self.build_contact_lookup()

    def gather_context(self, meeting: MeetingClassification) -> dict:
        if meeting.type == 'customer':
            account = self.account_tracker.get(meeting.entity_id)
            return {
                'account_metrics': {
                    'ring': account.ring,
                    'arr': account.arr,
                    'health': account.health,
                    'renewal_date': account.contract_end,
                },
                'stakeholders': self.get_stakeholders(account),
                'history': self.get_meeting_history(account, limit=3),
                'open_actions': self.get_account_actions(account),
                'strategic_programs': account.strategic_programs,
                'risks': account.current_risks,
            }
        # ... other meeting types
```

### General Profile

General users don't have accounts. The prepare phase:

1. **Skips account loading** entirely
2. **Classifies external meetings** as 'external' (not 'customer')
3. **Gathers simpler context** (just attendees, history)

```python
class GeneralDataCollector:
    def gather_context(self, meeting: MeetingClassification) -> dict:
        if meeting.type == 'external':
            return {
                'attendees': self.get_attendee_info(meeting),
                'history': self.get_meeting_history_by_attendees(meeting, limit=2),
            }
        # ... other meeting types
```

---

## Historical Context Gathering

### What We Gather

| Meeting Type | History Source | Lookback | Meeting Count |
|--------------|----------------|----------|---------------|
| Customer | Account folder + archive | 30 days | 2-3 meetings |
| QBR | Account folder + full quarter | 90 days | All in quarter |
| Training | Previous trainings | 90 days | All trainings |
| 1:1 | Archive by person | 30 days | 2-3 1:1s |
| Team Sync | Archive by meeting | 7 days | Last meeting |
| Partnership | Partner folder | 30 days | 2-3 meetings |

### Implementation

```python
def get_meeting_history(
    db: ActionDb,
    account_id: Optional[str] = None,
    attendees: Optional[List[str]] = None,
    meeting_type: Optional[str] = None,
    lookback_days: int = 30,
    limit: int = 3
) -> List[MeetingHistoryEntry]:
    """
    Query meeting history from SQLite.

    Supports multiple query patterns:
    - By account (customer meetings)
    - By attendees (1:1, external)
    - By meeting type (trainings)
    """
    query = """
        SELECT * FROM meetings_history
        WHERE start_time >= date('now', ?)
    """
    params = [f'-{lookback_days} days']

    if account_id:
        query += " AND account_id = ?"
        params.append(account_id)

    if attendees:
        # JSON array contains any of the attendees
        query += " AND EXISTS (SELECT 1 FROM json_each(attendees) WHERE value IN (?))"
        params.append(','.join(attendees))

    if meeting_type:
        query += " AND meeting_type = ?"
        params.append(meeting_type)

    query += " ORDER BY start_time DESC LIMIT ?"
    params.append(limit)

    return db.query(query, params)
```

### History Entry Structure

```python
@dataclass
class MeetingHistoryEntry:
    id: str
    title: str
    meeting_type: str
    start_time: datetime
    account_id: Optional[str]
    attendees: List[str]
    notes_path: Optional[str]  # Path to archived notes
    summary: Optional[str]     # AI-generated summary from wrap
    action_items: List[str]    # Extracted actions (if any)
```

---

## Directive JSON Schema

The directive is the contract between Phase 1 (prepare) and Phase 2 (enrich).

### Top-Level Structure

```json
{
  "version": "2.0",
  "command": "today",
  "generated_at": "2026-02-05T08:00:00Z",
  "profile": "customer-success",

  "context": { /* Date/time context */ },
  "api_status": { /* What APIs are available */ },
  "warnings": [ /* Resilience warnings */ ],

  "schedule": { /* Today's calendar */ },
  "meetings": [ /* Classified meetings with context */ ],
  "actions": { /* Due actions */ },
  "emails": { /* Email summary */ },

  "ai_tasks": [ /* What Claude should do */ ]
}
```

### Full Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["version", "command", "generated_at", "context", "meetings", "ai_tasks"],

  "properties": {
    "version": {
      "type": "string",
      "const": "2.0"
    },

    "command": {
      "type": "string",
      "enum": ["today", "wrap", "week", "inbox"]
    },

    "generated_at": {
      "type": "string",
      "format": "date-time"
    },

    "profile": {
      "type": "string",
      "enum": ["customer-success", "general", "sales", "engineering"]
    },

    "context": {
      "type": "object",
      "properties": {
        "date": { "type": "string", "format": "date" },
        "day_of_week": { "type": "string" },
        "week_number": { "type": "integer" },
        "year": { "type": "integer" },
        "timezone": { "type": "string" }
      }
    },

    "api_status": {
      "type": "object",
      "properties": {
        "calendar": { "type": "boolean" },
        "gmail": { "type": "boolean" },
        "sheets": { "type": "boolean" },
        "reason": { "type": "string" }
      }
    },

    "warnings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "level": { "enum": ["info", "warning", "error"] },
          "message": { "type": "string" },
          "action": { "type": "string" }
        }
      }
    },

    "schedule": {
      "type": "object",
      "properties": {
        "events": {
          "type": "array",
          "items": { "$ref": "#/$defs/calendarEvent" }
        },
        "gaps": {
          "type": "array",
          "items": { "$ref": "#/$defs/timeGap" }
        }
      }
    },

    "meetings": {
      "type": "array",
      "items": { "$ref": "#/$defs/classifiedMeeting" }
    },

    "actions": {
      "type": "object",
      "properties": {
        "overdue": { "type": "array", "items": { "$ref": "#/$defs/action" } },
        "due_today": { "type": "array", "items": { "$ref": "#/$defs/action" } },
        "due_this_week": { "type": "array", "items": { "$ref": "#/$defs/action" } },
        "waiting_on": { "type": "array", "items": { "$ref": "#/$defs/waitingItem" } }
      }
    },

    "emails": {
      "type": "object",
      "properties": {
        "high_priority": { "type": "array", "items": { "$ref": "#/$defs/email" } },
        "medium_count": { "type": "integer" },
        "low_count": { "type": "integer" }
      }
    },

    "ai_tasks": {
      "type": "array",
      "items": { "$ref": "#/$defs/aiTask" }
    }
  },

  "$defs": {
    "calendarEvent": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "start": { "type": "string", "format": "date-time" },
        "end": { "type": "string", "format": "date-time" },
        "location": { "type": "string" },
        "attendees": { "type": "array", "items": { "type": "string" } }
      }
    },

    "timeGap": {
      "type": "object",
      "properties": {
        "start": { "type": "string", "format": "date-time" },
        "end": { "type": "string", "format": "date-time" },
        "duration_minutes": { "type": "integer" }
      }
    },

    "classifiedMeeting": {
      "type": "object",
      "required": ["event_id", "type"],
      "properties": {
        "event_id": { "type": "string" },
        "type": {
          "enum": ["customer", "qbr", "training", "team_sync", "one_on_one",
                   "partnership", "all_hands", "external", "internal", "personal"]
        },
        "classification_confidence": { "enum": ["high", "medium", "low"] },

        "event": { "$ref": "#/$defs/calendarEvent" },

        "entity_id": { "type": "string" },
        "entity_name": { "type": "string" },

        "context": { "$ref": "#/$defs/meetingContext" },

        "prep_template": { "type": "string" },
        "prep_depth": { "enum": ["full", "moderate", "light", "none"] }
      }
    },

    "meetingContext": {
      "type": "object",
      "description": "Uses reference approach: key metrics inline, detail by file reference",
      "properties": {
        "account_metrics": {
          "description": "Quick-glance metrics (inline for convenience)",
          "type": "object",
          "properties": {
            "ring": { "type": "integer" },
            "arr": { "type": "string" },
            "health": { "enum": ["green", "yellow", "red"] },
            "renewal_date": { "type": "string", "format": "date" }
          }
        },

        "stakeholders": {
          "description": "Attendee names + roles inline, detail in refs.stakeholder_map",
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "name": { "type": "string" },
              "email": { "type": "string" },
              "role": { "type": "string" },
              "influence": { "enum": ["champion", "decision_maker", "user", "blocker"] }
            }
          }
        },

        "history_summary": {
          "description": "Brief summary inline, full notes in refs.meeting_history",
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "date": { "type": "string", "format": "date" },
              "title": { "type": "string" }
            }
          }
        },

        "open_action_count": {
          "description": "Count inline, full list in refs.account_actions",
          "type": "integer"
        },

        "refs": {
          "description": "File paths for Claude to load as needed",
          "type": "object",
          "properties": {
            "account_dashboard": { "type": "string" },
            "meeting_history": {
              "type": "array",
              "items": { "type": "string" }
            },
            "stakeholder_map": { "type": "string" },
            "account_actions": { "type": "string" },
            "inbox_threads": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        }
      }
    },

    "action": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "priority": { "enum": ["P1", "P2", "P3"] },
        "status": { "enum": ["pending", "completed", "waiting", "cancelled"] },
        "due_date": { "type": "string", "format": "date" },
        "days_overdue": { "type": "integer" },
        "account_id": { "type": "string" },
        "source": { "type": "string" },
        "context": { "type": "string" }
      }
    },

    "waitingItem": {
      "type": "object",
      "properties": {
        "who": { "type": "string" },
        "what": { "type": "string" },
        "asked_date": { "type": "string", "format": "date" },
        "days_waiting": { "type": "integer" },
        "context": { "type": "string" }
      }
    },

    "email": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "thread_id": { "type": "string" },
        "from": { "type": "string" },
        "subject": { "type": "string" },
        "snippet": { "type": "string" },
        "received": { "type": "string", "format": "date-time" },
        "priority": { "enum": ["high", "medium", "low"] },
        "linked_account": { "type": "string" }
      }
    },

    "aiTask": {
      "type": "object",
      "required": ["type", "priority"],
      "properties": {
        "type": {
          "enum": [
            "generate_customer_prep",
            "generate_qbr_prep",
            "generate_training_prep",
            "generate_internal_prep",
            "generate_1on1_prep",
            "generate_partnership_prep",
            "summarize_email",
            "generate_agenda_draft"
          ]
        },
        "priority": { "enum": ["high", "medium", "low"] },
        "meeting_id": { "type": "string" },
        "email_id": { "type": "string" },
        "template": { "type": "string" },
        "context_ref": { "type": "string" }
      }
    }
  }
}
```

---

## Example Directive Output

### CSM Profile - Customer Meeting

```json
{
  "version": "2.0",
  "command": "today",
  "generated_at": "2026-02-05T08:00:00-05:00",
  "profile": "customer-success",

  "context": {
    "date": "2026-02-05",
    "day_of_week": "Thursday",
    "week_number": 6,
    "year": 2026,
    "timezone": "America/New_York"
  },

  "api_status": {
    "calendar": true,
    "gmail": true,
    "sheets": true
  },

  "warnings": [],

  "schedule": {
    "events": [
      {
        "id": "ev_123",
        "title": "Acme Corp - Weekly Sync",
        "start": "2026-02-05T09:00:00-05:00",
        "end": "2026-02-05T09:45:00-05:00",
        "location": "Zoom",
        "attendees": ["sarah.chen@acme.com", "mike.torres@acme.com", "me@company.com"]
      },
      {
        "id": "ev_456",
        "title": "Team Standup",
        "start": "2026-02-05T14:00:00-05:00",
        "end": "2026-02-05T14:30:00-05:00",
        "attendees": ["colleague1@company.com", "colleague2@company.com", "me@company.com"]
      }
    ],
    "gaps": [
      {
        "start": "2026-02-05T10:00:00-05:00",
        "end": "2026-02-05T14:00:00-05:00",
        "duration_minutes": 240
      }
    ]
  },

  "meetings": [
    {
      "event_id": "ev_123",
      "type": "customer",
      "classification_confidence": "high",

      "event": {
        "id": "ev_123",
        "title": "Acme Corp - Weekly Sync",
        "start": "2026-02-05T09:00:00-05:00",
        "end": "2026-02-05T09:45:00-05:00"
      },

      "entity_id": "acme-corp",
      "entity_name": "Acme Corp",

      "context": {
        "account_metrics": {
          "ring": 2,
          "arr": "$450,000",
          "health": "yellow",
          "renewal_date": "2026-06-30"
        },

        "stakeholders": [
          {
            "name": "Sarah Chen",
            "email": "sarah.chen@acme.com",
            "role": "VP Engineering",
            "influence": "champion"
          },
          {
            "name": "Mike Torres",
            "email": "mike.torres@acme.com",
            "role": "Procurement",
            "influence": "decision_maker"
          }
        ],

        "history_summary": [
          { "date": "2026-01-28", "title": "Acme Corp - Weekly Sync" },
          { "date": "2026-01-21", "title": "Acme Corp - Technical Review" }
        ],

        "open_action_count": 2,

        "refs": {
          "account_dashboard": "2-areas/accounts/acme-corp/dashboard.md",
          "meeting_history": [
            "4-archive/2026-01-28/01-0900-customer-acme-notes.md",
            "4-archive/2026-01-21/02-1400-customer-acme-notes.md"
          ],
          "stakeholder_map": "2-areas/accounts/acme-corp/stakeholders.md",
          "account_actions": "2-areas/accounts/acme-corp/actions.md"
        }
      },

      "prep_template": "customer_call",
      "prep_depth": "full"
    },

    {
      "event_id": "ev_456",
      "type": "team_sync",
      "classification_confidence": "high",

      "event": {
        "id": "ev_456",
        "title": "Team Standup",
        "start": "2026-02-05T14:00:00-05:00",
        "end": "2026-02-05T14:30:00-05:00"
      },

      "context": {
        "history_summary": [
          { "date": "2026-02-04", "title": "Team Standup" }
        ],

        "open_action_count": 1,

        "refs": {
          "meeting_history": [
            "4-archive/2026-02-04/02-1400-internal-standup-notes.md"
          ],
          "my_actions": "_today/data/actions.json"
        }
      },

      "prep_template": "internal_sync",
      "prep_depth": "light"
    }
  ],

  "actions": {
    "overdue": [],
    "due_today": [
      {
        "id": "act_010",
        "title": "Review PR #234",
        "priority": "P1",
        "due_date": "2026-02-05",
        "source": "Team Standup"
      }
    ],
    "due_this_week": [
      {
        "id": "act_001",
        "title": "Send API documentation to Mike",
        "priority": "P2",
        "due_date": "2026-02-07",
        "account_id": "acme-corp"
      }
    ],
    "waiting_on": [
      {
        "who": "Acme IT Team",
        "what": "SSO requirements document",
        "asked_date": "2026-01-21",
        "days_waiting": 15,
        "context": "Needed for SSO planning"
      }
    ]
  },

  "emails": {
    "high_priority": [
      {
        "id": "email_001",
        "thread_id": "thread_001",
        "from": "sarah.chen@acme.com",
        "subject": "RE: API Integration Timeline",
        "snippet": "Hi, wanted to follow up on the API docs. Mike is asking about...",
        "received": "2026-02-05T07:30:00-05:00",
        "priority": "high",
        "linked_account": "acme-corp"
      }
    ],
    "medium_count": 12,
    "low_count": 8
  },

  "ai_tasks": [
    {
      "type": "generate_customer_prep",
      "priority": "high",
      "meeting_id": "ev_123",
      "template": "customer_call",
      "context_ref": "meetings[0].context"
    },
    {
      "type": "generate_internal_prep",
      "priority": "low",
      "meeting_id": "ev_456",
      "template": "internal_sync"
    },
    {
      "type": "summarize_email",
      "priority": "medium",
      "email_id": "email_001"
    }
  ]
}
```

---

## How Claude Uses References (Phase 2)

When Claude receives the directive, it reads referenced files based on meeting prep needs:

### Customer Meeting (Full Prep)

1. Read `refs.account_dashboard` for strategic context
2. Read each file in `refs.meeting_history` for recent interactions
3. Read `refs.stakeholder_map` for detailed relationship notes
4. Read `refs.account_actions` for open items with context
5. Synthesize into prep document

### Internal Meeting (Light Prep)

1. Read last meeting notes from `refs.meeting_history[0]`
2. Read user's actions from `refs.my_actions`
3. Generate concise prep

### Unknown External Meeting

When `classification` is `external` with no entity match, Phase 1 triggers local research:

```json
{
  "type": "external",
  "research": {
    "local_search_performed": true,
    "archive_mentions": ["4-archive/2026-01-15/email-globex.md"],
    "inbox_threads": [],
    "company_domain": "globex.com",
    "attendee_names": ["Jane Smith", "Bob Jones"]
  },
  "ai_task": {
    "type": "research_unknown_meeting",
    "priority": "medium"
  }
}
```

Claude then performs web research:
1. Company lookup (website, LinkedIn company page)
2. Attendee lookup (LinkedIn profiles)
3. Synthesizes into prep brief

See `UNKNOWN-MEETING-RESEARCH.md` for full research hierarchy.

---

## Error Handling

### Graceful Degradation

The prepare phase should handle API failures gracefully:

```python
def prepare_today():
    directive = DirectiveBuilder()

    # Calendar - Required for meeting prep
    try:
        events = fetch_calendar_events()
        directive.set_calendar(events)
    except APIError as e:
        directive.add_warning('error', f'Calendar unavailable: {e}')
        directive.set_api_status('calendar', False)
        # Continue with empty schedule

    # Gmail - Optional (nice to have)
    try:
        emails = fetch_emails()
        directive.set_emails(emails)
    except APIError as e:
        directive.add_warning('warning', f'Email unavailable: {e}')
        directive.set_api_status('gmail', False)

    # Sheets - Profile-dependent
    if config.profile == 'customer-success':
        try:
            accounts = fetch_account_data()
            directive.set_accounts(accounts)
        except APIError as e:
            directive.add_warning('warning', f'Account data unavailable: {e}')
            # Fall back to local CSV if available
            accounts = load_local_account_csv()
            if accounts:
                directive.set_accounts(accounts)
```

### Validation

Before writing the directive, validate it:

```python
def validate_directive(directive: dict) -> List[str]:
    errors = []

    # Required fields
    if not directive.get('meetings'):
        errors.append('No meetings in directive')

    # Context completeness
    for meeting in directive.get('meetings', []):
        if meeting['type'] == 'customer' and not meeting.get('context', {}).get('account_metrics'):
            errors.append(f"Customer meeting {meeting['event_id']} missing account metrics")

    # AI tasks match meetings
    meeting_ids = {m['event_id'] for m in directive.get('meetings', [])}
    for task in directive.get('ai_tasks', []):
        if task.get('meeting_id') and task['meeting_id'] not in meeting_ids:
            errors.append(f"AI task references unknown meeting {task['meeting_id']}")

    return errors
```

---

## File Structure

```
_tools/
├── prepare_today.py          # Main orchestrator
├── lib/
│   ├── __init__.py
│   ├── calendar_utils.py     # Calendar API wrapper
│   ├── email_utils.py        # Gmail API wrapper
│   ├── account_utils.py      # Account data (CSM profile)
│   ├── meeting_classifier.py # Multi-signal classification
│   ├── context_gatherer.py   # Historical context queries
│   ├── action_utils.py       # SQLite action queries
│   └── directive_builder.py  # Directive assembly
```

---

## Next Steps

1. **Implement meeting classifier** - Port existing `meeting_utils.py` to new multi-signal design
2. **Add SQLite queries** - Implement history lookback using `meetings_history` table
3. **Create profile-aware collectors** - CSM vs General data gathering
4. **Define AI task templates** - What Claude should do for each meeting type
5. **Build directive validation** - Ensure completeness before Phase 2

---

## Related Documents

- `DATA-MODEL.md` - Entity definitions and sources
- `MEETING-TYPES.md` - Classification logic and prep templates
- `PROFILES.md` - Profile-specific configuration
- `ACTIONS-SCHEMA.md` - SQLite schema for actions/history
- `UNKNOWN-MEETING-RESEARCH.md` - Research hierarchy for unknown meetings

---

*Document Version: 1.1*
*Updated: 2026-02-05 - Added reference approach, unknown meeting research*
