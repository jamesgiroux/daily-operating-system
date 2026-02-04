# UI Design: Daybreak

> **Status:** Exploratory. This document captures early thinking about interface design. It will be revisited and refined after architectural decisions are made.

See [PHILOSOPHY.md](PHILOSOPHY.md), [PRINCIPLES.md](PRINCIPLES.md), and [VISION.md](VISION.md) for foundational decisions that inform this design.

---

## UI Design Principles

### 1. Consumption First
The primary interaction is reading, not writing. Every screen should be optimized for scanning and comprehension.

### 2. Progressive Disclosure
- **Level 1:** Rendered output (most users, most of the time)
- **Level 2:** Inline editing (when you need to change something)
- **Level 3:** Terminal access (power users only)

### 3. Calm Technology
- No notification spam
- Smart scheduling (morning brief, evening wrap)
- Ambient awareness, not constant demands

### 4. Trust the File System
- Markdown files are the source of truth
- The app is a lens, not a database
- You can always use other tools on the same files

---

## Information Architecture

```
Daybreak
├── Today (default view)
│   ├── Overview card
│   ├── Calendar strip
│   ├── Meeting prep cards
│   ├── Action items
│   └── Waiting on (delegated)
│
├── Accounts (Customer Success / Sales)
│   ├── List view (health indicators)
│   └── Account detail
│       ├── Recent meetings
│       ├── Action items
│       └── Notes
│
├── Projects (PM / Engineering)
│   ├── Active
│   ├── Planning
│   └── Completed
│
├── Areas (ongoing responsibilities)
│
└── Settings
    ├── Workspace path
    ├── Schedule times
    ├── Google API status
    └── Advanced (show terminal, etc.)
```

---

## Screen Designs

### Today View (Primary)

```
┌──────────────────────────────────────────────────────────────┐
│  Daybreak                                    [−] [□] [×]     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Good morning, James.                     Tuesday, Feb 4     │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  TODAY'S FOCUS                                      │    │
│  │                                                     │    │
│  │  • Finalize Q1 planning deck                        │    │
│  │  • Acme Corp renewal conversation                   │    │
│  │  • Review hiring pipeline                           │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  CALENDAR                                           │    │
│  │                                                     │    │
│  │  9:00   Acme Corp - Quarterly Review      [Prep ▶]  │    │
│  │  11:00  1:1 with Sarah                              │    │
│  │  2:00   Product Sync                                │    │
│  │  4:00   Interview - Sr. Engineer                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  ACTION ITEMS                              12 total │    │
│  │                                                     │    │
│  │  ☐ Send proposal to BigCorp           Due today    │    │
│  │  ☐ Review Sarah's promotion doc       Due today    │    │
│  │  ☐ Expense report                     Overdue 2d   │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌──────────────────────┐ ┌────────────────────────────┐    │
│  │  WAITING ON          │ │  INBOX                     │    │
│  │                      │ │                            │    │
│  │  Legal review - 5d   │ │  3 items to process        │    │
│  │  Budget approval     │ │  [Process Now]             │    │
│  └──────────────────────┘ └────────────────────────────┘    │
│                                                              │
│                                          [↻ Refresh]        │
└──────────────────────────────────────────────────────────────┘
```

### Meeting Prep (Expanded Card)

```
┌─────────────────────────────────────────────────────────────┐
│  ACME CORP - QUARTERLY REVIEW                     9:00 AM   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Attendees: John Smith (CEO), Jane Doe (VP Ops), You       │
│                                                             │
│  CONTEXT                                                    │
│  ─────────────────────────────────────────────────────────  │
│  Last meeting (Jan 15): Discussed expansion timeline.       │
│  They're evaluating competitors. Decision expected Q1.      │
│                                                             │
│  OPEN ITEMS                                                 │
│  ─────────────────────────────────────────────────────────  │
│  • Security questionnaire - sent Jan 20, awaiting response  │
│  • Pricing proposal - they requested enterprise tier        │
│                                                             │
│  SUGGESTED TALKING POINTS                                   │
│  ─────────────────────────────────────────────────────────  │
│  1. Follow up on security questionnaire status              │
│  2. Present case studies from similar-size companies        │
│  3. Discuss implementation timeline for Q2 start            │
│                                                             │
│                              [Edit] [Add to Agenda] [Done]  │
└─────────────────────────────────────────────────────────────┘
```

---

## Interaction Patterns

### Click to Edit
Any text block can be clicked to enter edit mode:
1. Click on card content
2. Markdown editor appears inline
3. Changes auto-save after 1 second idle
4. Click elsewhere to exit edit mode

### Refresh
- Manual: Click refresh button
- Scheduled: App runs /today at configured time
- Watch: File system changes trigger UI update

### Navigation
- Sidebar for main sections (Today, Accounts, Projects, Areas)
- Breadcrumbs for drill-down
- Cmd+K for quick jump (search)

---

## Visual Design

### Typography
- **Headlines:** System sans-serif, bold, generous size
- **Body:** System sans-serif, regular, comfortable reading size
- **Monospace:** Only for code blocks (rare)

### Colors
- **Background:** Warm white (#FAFAF8) or dark (#1A1A1A)
- **Cards:** White (#FFFFFF) or elevated dark (#252525)
- **Accent:** Amber/gold (#F59E0B) - warmth, morning light
- **Text:** High contrast, no gray-on-gray

### Spacing
- Generous whitespace
- Cards with clear boundaries
- Breathing room between sections

### Principles
- No harsh borders (subtle shadows instead)
- Rounded corners (warmth)
- Content is the hero (minimal chrome)

---

## States

### Loading
- Skeleton screens, not spinners
- Progressive loading (show what you have)

### Empty
- Friendly messages
- Clear calls to action
- Never just blank space

### Error
- Inline error messages
- Recovery actions prominent
- Technical details hidden (expandable)

---

## Accessibility

- Keyboard navigable (tab order, focus rings)
- Screen reader labels
- Sufficient color contrast
- Respects system preferences (reduce motion, dark mode)

---

## Future Considerations

### Mobile Companion
- Read-only view of today
- Push notifications
- Quick capture (voice/text)

### Team Features
- Shared workspaces
- Delegated items visibility
- Meeting notes sharing

### Integrations
- Slack status sync
- Calendar blocking
- Email draft sending
