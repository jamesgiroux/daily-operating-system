# User Profiles

> Role-based configuration that shapes workspace structure, meeting classification, and prep templates.

---

## Why Profiles?

Different roles have different workflows:

| Role | Primary Focus | Key Entities | Meeting Types |
|------|---------------|--------------|---------------|
| **Customer Success** | Account health, renewals, adoption | Accounts, Stakeholders | Customer calls, QBRs, Training |
| **Sales** | Pipeline, deals, relationships | Opportunities, Contacts | Discovery, Demo, Negotiation |
| **Engineering** | Projects, sprints, technical work | Projects, PRs, Issues | Standups, Planning, 1:1s |
| **General** | Tasks, projects, calendar | Projects, Tasks | Meetings, 1:1s, All Hands |

A profile configures:
1. **PARA structure** - What folders exist and what they contain
2. **Meeting classification** - How meetings are categorized
3. **Prep templates** - What context is pulled for each meeting type
4. **Data sources** - What trackers/files the system reads from

---

## Profile: Customer Success

For TAMs, CSMs, RMs, and other customer-facing roles.

### PARA Structure

```
workspace/
├── 1-projects/
│   ├── [account]-[initiative]/     # Account-specific projects
│   │   ├── README.md
│   │   └── notes/
│   └── internal/                   # Internal projects
│
├── 2-areas/
│   ├── accounts/                   # Account management
│   │   ├── _tracker.csv            # Master account tracker
│   │   └── [account]/
│   │       ├── dashboard.md        # Account health dashboard
│   │       ├── stakeholders.md     # Key contacts
│   │       ├── actions.md          # Open items for this account
│   │       └── notes/              # Meeting notes by date
│   ├── renewals/                   # Renewal pipeline
│   └── adoption/                   # Adoption tracking
│
├── 3-resources/
│   ├── playbooks/                  # CS playbooks
│   ├── templates/                  # Email templates, decks
│   └── competitive/                # Competitive intel
│
├── 4-archive/
│   └── YYYY-MM-DD/                 # Daily archives
│
├── _today/                         # Today's briefing
└── _inbox/                         # Incoming items
```

### Meeting Types

| Type | Indicators | Prep Focus |
|------|------------|------------|
| **Customer Call** | External attendees match account contacts | Account health, risks, open items |
| **QBR** | "QBR", "Business Review" in title | Full account review, metrics, roadmap |
| **Training** | "Training", "Enablement" in title | Training agenda, materials, attendees |
| **Internal Sync** | All internal attendees, "sync", "standup" | Team updates, blockers |
| **1:1** | 2 attendees, one is manager/report | Career, feedback, support |
| **Partnership** | External attendees from partner orgs | Partnership status, joint initiatives |
| **All Hands** | Large attendee count, company-wide | Just time/location, no prep |

### Data Sources

| Source | Location | Used For |
|--------|----------|----------|
| Account Tracker | `2-areas/accounts/_tracker.csv` | ARR, ring, health, renewal dates |
| Account Actions | `2-areas/accounts/[acct]/actions.md` | Open items per account |
| Stakeholder Map | `2-areas/accounts/[acct]/stakeholders.md` | Contact roles, influence |
| Meeting Notes | `4-archive/YYYY-MM-DD/` | Historical context |
| Master Task List | `_today/master-task-list.md` | Cross-account priorities |

---

## Profile: General

For knowledge workers without account-specific workflows.

### PARA Structure

```
workspace/
├── 1-projects/
│   └── [project-name]/
│       ├── README.md
│       └── notes/
│
├── 2-areas/
│   ├── work/
│   ├── personal/
│   └── learning/
│
├── 3-resources/
│   ├── references/
│   └── templates/
│
├── 4-archive/
│   └── YYYY-MM-DD/
│
├── _today/
└── _inbox/
```

### Meeting Types

| Type | Indicators | Prep Focus |
|------|------------|------------|
| **External** | External attendees | Context on who they are |
| **Team Meeting** | Multiple internal attendees | Agenda, action items |
| **1:1** | 2 attendees | Topics to discuss, feedback |
| **All Hands** | Large count, company-wide | Just time, no prep |
| **Personal** | Personal calendar, no work domain | No prep |

### Data Sources

| Source | Location | Used For |
|--------|----------|----------|
| Project Tracker | `1-projects/` | Active projects, status |
| Task List | `_today/master-task-list.md` | Priorities |
| Meeting Notes | `4-archive/` | Historical context |

---

## Profile Selection

During setup:

```
Welcome to DailyOS!

What best describes your role?

[ ] Customer Success (TAM, CSM, RM)
    → Account-focused workspace with customer tracking

[ ] Sales (AE, SDR, Sales Eng)
    → Pipeline-focused workspace with deal tracking

[ ] Engineering (Dev, PM, Designer)
    → Project-focused workspace with sprint tracking

[ ] General
    → Flexible workspace for any knowledge worker
```

Profile selection:
1. Creates appropriate PARA folder structure
2. Seeds example tracker files
3. Configures meeting classification rules
4. Sets up prep templates

---

## Profile Configuration

Stored in `~/.daybreak/config.json`:

```json
{
  "workspacePath": "/path/to/workspace",
  "profile": "customer-success",
  "profileConfig": {
    "accountTrackerPath": "2-areas/accounts/_tracker.csv",
    "meetingNotesPath": "4-archive",
    "historyLookbackDays": 30,
    "historyMeetingCount": 3
  }
}
```

---

## Future Profiles

| Profile | Target User | Key Differentiator |
|---------|-------------|-------------------|
| Sales | AEs, SDRs | Opportunity/deal tracking |
| Engineering | Developers, PMs | Sprint/issue tracking |
| Executive | Directors, VPs | Cross-functional view |
| Consultant | External consultants | Client/engagement tracking |

---

## UI Implications (DEC8, DEC9, DEC12)

Profiles shape the UI in three ways: navigation, component content, and data sources.

### Navigation Differences

| Sidebar Item | CS | GA | Notes |
|-------------|----|----|-------|
| Dashboard | Yes | Yes | Content differs, shell identical |
| Actions | Yes | Yes | CS: account filter. GA: project filter |
| Inbox | Yes | Yes | Identical |
| Accounts | **Yes** | No | CS portfolio: ARR, ring, health, renewal |
| Projects | No | **Yes** | GA portfolio: status, deadline, progress |
| Settings | Yes | Yes | Identical |

The third "Workspace" nav item is the profile's primary entity: Accounts for CS, Projects for GA. Same component layout, different data source and metrics.

### Component Content Differences

| Component | CS Profile | GA Profile |
|-----------|-----------|------------|
| Meeting card border | Gold for customer meetings | No account-based coloring |
| Meeting card metadata | Account name, ARR, ring, health badge | Attendee names only |
| Prep summary | Full: risks, wins, strategic programs, stakeholder influence | Light: attendee context, last meeting |
| Actions panel | Grouped by account | Flat list |
| Email section | Linked to accounts | Sender name only |
| Stats row | "3 customer meetings" | "2 external meetings" |

### Profile Switching (DEC9)

**Non-destructive.** Switching profiles:

Changes:
- Meeting classification rules
- Sidebar navigation items
- Card metadata and rendering
- Prepare phase data collection

Does NOT change:
- File system structure
- PARA directories
- Existing data or notes
- Archive history

### Profile Indicator

Current profile shown in sidebar header below app name. Phase 2: clickable dropdown to switch.

```
┌─────────────────────┐
│ ⚡ DailyOS           │
│ Customer Success  ▾  │
└─────────────────────┘
```

---

## Implementation Notes

### Phase 1 (MVP)
- Customer Success profile (your workflow)
- General profile (fallback)
- Manual profile selection during setup
- Profile stored in `~/.daybreak/config.json`

### Phase 2
- Profile indicator in sidebar header
- Clickable profile switcher (non-destructive)
- Profile detection from existing workspace

### Phase 3
- Custom profile creation
- Profile marketplace (community templates)
- Profile inheritance (base + customizations)

---

*Updated: 2026-02-05*
