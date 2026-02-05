# Data Model

> What we collect, where it lives, and how it flows.

---

## Key Decisions (Resolved)

| Question | Decision | Rationale |
|----------|----------|-----------|
| Different templates per meeting type? | **Yes** | Customer, Internal, Partnership, 1:1, etc. all need different prep focus |
| Where do actions live? | **Local file (CSV or SQLite)** | Must be local-first per philosophy; enables state tracking across days |
| Account data: copy or reference? | **Profile-dependent** | CSM profile has accounts; General profile doesn't. Accounts are a "plugin" |
| How much history? | **~30 days, 2-3 meetings** | Enough to establish trends without noise |
| Meeting classification? | **Multi-signal** | Attendees + account/project cross-ref + title keywords |

---

## Core Entities

### 1. Meeting

**Question:** Do different meeting types need different data models?

| Field | Customer | Internal | Personal | Source |
|-------|----------|----------|----------|--------|
| `id` | ✓ | ✓ | ✓ | Generated |
| `time` | ✓ | ✓ | ✓ | Calendar API |
| `end_time` | ✓ | ✓ | ✓ | Calendar API |
| `title` | ✓ | ✓ | ✓ | Calendar API |
| `type` | ✓ | ✓ | ✓ | Inferred or calendar category |
| `account` | ✓ | — | — | Account lookup by attendees |
| `attendees` | ✓ | ✓ | — | Calendar API |
| `location` | ✓ | ✓ | ✓ | Calendar API |
| `description` | ✓ | ✓ | ✓ | Calendar API |

**Prep data (varies by type):**

| Field | Customer | Internal | Personal | Source |
|-------|----------|----------|----------|--------|
| `quick_context` | ARR, Ring, Health, Renewal | Sprint day, blockers | — | Account dashboard / Project tracker |
| `stakeholders` | ✓ (with roles, influence) | ✓ (just names) | — | CRM / manual |
| `since_last` | ✓ | ✓ | — | Previous meeting notes |
| `strategic_programs` | ✓ | — | — | Account tracker |
| `risks` | ✓ | ✓ (blockers) | — | Account tracker / Project tracker |
| `talking_points` | ✓ | ✓ | — | AI-generated |
| `open_items` | ✓ | ✓ | — | Action tracker |
| `questions` | ✓ | ✓ | — | AI-generated |

**Decision:** Option A — One flexible `Meeting` model with optional fields per type. Simpler to implement, still captures differences. Type-specific fields are optional.

---

### 2. Action

| Field | Required | Source |
|-------|----------|--------|
| `id` | ✓ | Generated |
| `title` | ✓ | Meeting notes / Manual |
| `account` | — | Linked from meeting |
| `project` | — | PARA project link |
| `priority` | ✓ | Manual / AI-inferred |
| `status` | ✓ | Manual |
| `due_date` | — | Manual |
| `context` | — | Where this came from |
| `source` | — | "Meeting: Acme Sync (Feb 3)" |
| `owner` | — | Who's responsible |
| `waiting_on` | — | If blocked, who are we waiting on |

**Decision:** SQLite as disposable cache, markdown notes as source of truth.

- Master state: `~/.daybreak/actions.db` (rebuilt from files if corrupted)
- Source of truth: Meeting notes in archive (human-readable, portable)
- Daily views: Generated from SQLite for performance

See `ACTIONS-SCHEMA.md` for full schema.

---

### 3. Email

| Field | Required | Source |
|-------|----------|--------|
| `id` | ✓ | Gmail API |
| `sender` | ✓ | Gmail API |
| `sender_email` | ✓ | Gmail API |
| `subject` | ✓ | Gmail API |
| `snippet` | — | Gmail API |
| `priority` | ✓ | AI-inferred |
| `received` | — | Gmail API |
| `thread_id` | — | Gmail API (for conversation) |
| `labels` | — | Gmail API |
| `conversation_arc` | — | AI-generated ("3rd message in negotiation") |
| `recommended_action` | — | AI-generated |
| `linked_account` | — | Inferred from sender domain |

---

### 4. Account (Customer Context)

| Field | Source |
|-------|--------|
| `name` | Account tracker |
| `ring` | Account tracker (1-4) |
| `arr` | Account tracker |
| `health` | Account tracker (Red/Yellow/Green) |
| `contract_end` | Account tracker |
| `csm` | Account tracker |
| `champion` | Manual / CRM |
| `recent_wins` | Meeting notes / Account tracker |
| `current_risks` | Meeting notes / Account tracker |
| `strategic_programs` | Account tracker |
| `last_contact` | Calendar / Email |
| `next_meeting` | Calendar |

**Decision:** Reference approach — Claude builds context from file references, not embedded data.

- Directive contains file paths/references, not embedded content
- Claude loads referenced files as needed during Phase 2
- Reduces directive size, allows Claude to be selective about depth
- Key metrics (ARR, ring, health) included inline for quick context

---

## Data Sources

| Source | What It Provides | Integration |
|--------|------------------|-------------|
| **Google Calendar** | Meetings, times, attendees | API (prepare phase) |
| **Gmail** | Emails, threads, labels | API (prepare phase) |
| **Account Tracker** | ARR, ring, health, programs | CSV in `3-resources/accounts/` |
| **Project Tracker** | Sprint status, blockers | Markdown in `1-projects/` |
| **Meeting Notes** | Historical context, actions | Archive markdown |
| **Manual Input** | Priorities, custom context | Config / inline |

---

## File Structure

```
_today/
├── data/
│   ├── manifest.json       # Index of what's available
│   ├── schedule.json       # Today's calendar + embedded prep summaries
│   ├── actions.json        # All actions due
│   ├── emails.json         # Flagged emails
│   └── preps/
│       ├── 0900-acme-sync.json
│       └── 1400-team-standup.json
│
├── 00-overview.md          # Human-readable daily briefing
├── 01-0900-customer-acme-prep.md
├── 02-1400-internal-standup-prep.md
├── 80-actions-due.md
└── 83-email-summary.md
```

---

## Questions (Resolved)

### Q1: Meeting Type Templates ✓

**Decision:** Yes, different templates for each meeting type.

See `MEETING-TYPES.md` for full definitions:
- Customer Call, QBR, Training → Account-focused prep
- Internal Sync, 1:1 → Team/personal-focused prep
- Partnership → Partner-focused prep
- All Hands, Personal → No prep needed

### Q2: Prep Summary vs Full Prep ✓

**Decision:** Option C - JSON prep has all fields, summary is derived.

- Full prep stored in `data/preps/{meeting-id}.json`
- Summary is first N items from each section
- Dashboard shows summary, detail page shows full

### Q3: Action Source of Truth ✓

**Decision:** Centralized local file with daily views.

- Master list: `actions.csv` or SQLite DB in workspace
- Per-account actions: Linked/filtered from master
- Daily view: `80-actions-due.md` generated from master
- Completion updates master file (local-first, no cloud)

### Q4: Account Data ✓

**Decision:** Profile-dependent. Accounts are a "plugin" for CSM profile.

- CSM profile: Account tracker in `2-areas/accounts/_tracker.csv`
- General profile: No accounts concept
- Data freshness: Read fresh each prep, show last-updated

### Q5: Historical Context ✓

**Decision:** 30-day lookback, 2-3 meetings, establish trends.

- Search archive for last 2-3 meetings with same attendees/account
- Pull meeting notes summaries
- Pull open actions from those meetings
- Goal: Show trends, not just last state

---

## Next Steps

1. ~~Finalize the data model~~ ✓ Decisions made above
2. ~~Create JSON schemas~~ ✓ Done in `templates/schemas/`
3. ~~Simplify Rust parser~~ ✓ JSON-only in `commands.rs`
4. ~~Define profiles~~ ✓ See `PROFILES.md`
5. ~~Define meeting types~~ ✓ See `MEETING-TYPES.md`
6. ~~Design action state management~~ ✓ See `ACTIONS-SCHEMA.md`
7. ~~Design prepare phase~~ ✓ See `PREPARE-PHASE.md`
8. **Create type-specific templates** - One per meeting type for Claude
9. **Update deliver phase** - Output JSON (markdown is optional human view)
10. **Build profile selection UI** - Setup wizard

---

## Related Documents

- `PROFILES.md` - Role-based workspace configuration
- `MEETING-TYPES.md` - Meeting classification and prep templates
- `ACTIONS-SCHEMA.md` - SQLite schema for action state
- `PREPARE-PHASE.md` - Prepare phase architecture and directive schema
- `UNKNOWN-MEETING-RESEARCH.md` - Research hierarchy for unknown external meetings
- `ARCHITECTURE.md` - Technical architecture overview

---

*Updated: 2026-02-05*
