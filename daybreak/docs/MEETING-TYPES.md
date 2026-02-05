# Meeting Types

> Classification, jobs-to-be-done, and prep requirements for each meeting type.

---

## Classification Logic

### Step 1: Determine Internal vs External

**User's email domain** (from Google OAuth) is the baseline:
- User domain: `@company.com`
- Attendee `@company.com` → Internal
- Attendee `@other.com` → External

```python
user_domain = get_user_domain_from_oauth()  # e.g., "company.com"

def is_internal(email):
    return email.endswith(f"@{user_domain}")

external_attendees = [a for a in attendees if not is_internal(a)]
internal_attendees = [a for a in attendees if is_internal(a)]
```

### Step 2: Determine Meeting Scale

| Attendee Count | Scale | Typical Type |
|----------------|-------|--------------|
| 2 | 1:1 | 1:1, Customer Call |
| 3-10 | Team | Sync, Customer Call, Training |
| 11-50 | Group | Department meeting |
| 51-500 | Division | Large org meeting |
| 500+ | Company | All Hands |

### Step 3: Cross-Reference Attendees

For external attendees, check against known entities:

```python
def classify_external_meeting(external_attendees):
    # Check accounts (CSM profile)
    for account in get_accounts():
        if any(e in account.contacts for e in external_attendees):
            return ("customer", account.id)

    # Check partners
    for partner in get_partners():
        if any(e.endswith(f"@{partner.domain}") for e in external_attendees):
            return ("partnership", partner.id)

    # Unknown external
    return ("external", None)
```

### Step 4: Title Keywords (Tiebreaker)

| Keywords | Override To |
|----------|-------------|
| "QBR", "Business Review", "Quarterly" | QBR |
| "Training", "Enablement", "Workshop" | Training |
| "Standup", "Daily", "Scrum" | Team Sync |
| "All Hands", "Town Hall", "Company" | All Hands |
| "1:1", "1-1", "One on One" | 1:1 |

### Classification Algorithm

```python
def classify_meeting(meeting, user_domain):
    attendees = meeting.attendees
    title = meeting.title.lower()
    count = len(attendees)

    external = [a for a in attendees if not a.endswith(f"@{user_domain}")]
    has_external = len(external) > 0

    # Scale-based overrides
    if count >= 50:
        return "all_hands"

    # Title-based overrides
    if any(kw in title for kw in ["qbr", "business review", "quarterly review"]):
        return "qbr"
    if any(kw in title for kw in ["training", "enablement", "workshop"]):
        return "training"
    if any(kw in title for kw in ["all hands", "town hall"]):
        return "all_hands"

    # External attendee classification
    if has_external:
        match_type, entity_id = classify_external_meeting(external)
        if match_type == "customer":
            return "customer"
        if match_type == "partnership":
            return "partnership"
        return "external"

    # Internal classification
    if count == 2:
        return "one_on_one"
    if any(kw in title for kw in ["standup", "sync", "scrum", "daily"]):
        return "team_sync"

    return "internal"
```

---

## Meeting Type Definitions

### Customer Call

**Job to be done:** "Help me walk into a customer conversation prepared with context, so I can be present and strategic rather than scrambling to remember details."

| Attribute | Value |
|-----------|-------|
| Classification | External attendees match known account contacts |
| Prep depth | Full |
| History lookback | Last 2-3 meetings with this account |

**Prep Template:**

```markdown
# [Account Name] - [Meeting Title]
**Time:** 9:00 AM - 9:45 AM

## Quick Context
| Metric | Value |
|--------|-------|
| Ring | 2 |
| ARR | $450,000 |
| Health | Yellow |
| Renewal | June 30, 2026 |

## Key Attendees
- **Sarah Chen** (VP Engineering) - Technical champion, drives adoption
- **Mike Torres** (Procurement) - Budget authority

## Since Last Meeting (Jan 28)
- Completed POC with Platform team
- Resolved authentication blockers
- Training scheduled for March

## Current Strategic Programs
- ✓ Phase 1 rollout complete
- ○ API integration (in progress)
- ○ Enterprise SSO (blocked on IT)

## Risks to Monitor
- Champion (Sarah) moving to new role in Q2
- Budget cycle ends March 15

## Suggested Talking Points
1. Acknowledge POC success, explore expansion opportunities
2. Probe on Sarah's transition - who will be new champion?
3. Plant seeds for renewal conversation

## Open Items
- [ ] Send API documentation (due: Feb 7) - Mike requested
- [ ] Schedule SSO planning call with IT

## Questions to Surface
- What's the decision timeline for renewal?
- Who will own the relationship after Sarah's transition?
```

---

### QBR (Quarterly Business Review)

**Job to be done:** "Help me run a strategic business review that demonstrates value, addresses concerns proactively, and positions us for renewal/expansion."

| Attribute | Value |
|-----------|-------|
| Classification | "QBR" or "Business Review" in title |
| Prep depth | Comprehensive |
| History lookback | Full quarter |

**Prep Template:**

```markdown
# [Account Name] - Quarterly Business Review
**Time:** 2:00 PM - 3:00 PM
**Quarter:** Q1 2026

## Executive Summary
[One paragraph on account health, key wins, and focus areas]

## Quick Context
| Metric | Q4 2025 | Q1 2026 | Trend |
|--------|---------|---------|-------|
| ARR | $400K | $450K | ↑ |
| Active Users | 245 | 312 | ↑ |
| Health Score | Yellow | Yellow | → |

## Value Delivered This Quarter
- Reduced ticket resolution time by 23%
- Onboarded 3 new teams (67 users)
- Completed Phase 1 integration

## Challenges & How We Addressed Them
- **Auth issues** → Resolved with SSO integration
- **Adoption lag in APAC** → Scheduled regional training

## Roadmap Alignment
| Their Priority | Our Solution | Status |
|----------------|--------------|--------|
| API automation | v2.3 release | Shipped |
| Mobile access | Q2 roadmap | Planned |

## Renewal Position
- Contract end: June 30, 2026
- Renewal conversation target: April
- Expansion opportunity: +2 teams ($50K)

## QBR Agenda
1. Value review (10 min)
2. Challenges & solutions (10 min)
3. Roadmap alignment (15 min)
4. Success planning (15 min)
5. Next steps (10 min)
```

---

### Training Call

**Job to be done:** "Help me deliver effective training that drives adoption and leaves attendees confident in using the product."

| Attribute | Value |
|-----------|-------|
| Classification | "Training", "Enablement", "Workshop" in title |
| Prep depth | Moderate |
| History lookback | Previous trainings for this account |

**Prep Template:**

```markdown
# [Account Name] - Training Session
**Time:** 11:00 AM - 12:00 PM
**Topic:** Advanced Reporting

## Training Context
- **Audience:** Finance team (8 attendees)
- **Skill level:** Intermediate (completed basics)
- **Previous sessions:** Intro (Jan 15), Dashboards (Jan 22)

## Session Objectives
1. Build custom reports using filters
2. Schedule automated report delivery
3. Export data for external analysis

## Attendee Notes
- Lisa (Finance Manager) - Primary champion, very engaged
- New attendees: Tom, Rachel (first session)

## Materials
- [ ] Demo environment ready
- [ ] Slide deck updated
- [ ] Handout PDF prepared

## Follow-up Items from Last Session
- [ ] Share report template library
- [ ] Send recording of Dashboards session
```

---

### Internal Team Sync

**Job to be done:** "Help me come to team meetings informed about what's happening, so I can contribute meaningfully and not waste everyone's time."

| Attribute | Value |
|-----------|-------|
| Classification | All internal attendees, recurring, "sync"/"standup" in title |
| Prep depth | Light |
| History lookback | Last meeting only |

**Prep Template:**

```markdown
# Team Sync
**Time:** 2:00 PM - 2:30 PM

## My Updates
- Completed: [Items from last week]
- In progress: [Current focus]
- Blocked: [Any blockers]

## Topics to Raise
- [Discussion items you want to bring up]

## Open Actions (Mine)
- [ ] Review PR #234 (due today)
- [ ] Update Q1 forecast

## Notes from Last Sync
- Decision: Moving to bi-weekly releases
- Action: Sarah to draft new process doc
```

---

### 1:1

**Job to be done:** "Help me have meaningful 1:1s that address what matters - career growth, blockers, feedback - rather than status updates."

| Attribute | Value |
|-----------|-------|
| Classification | Exactly 2 attendees, one is manager/report |
| Prep depth | Personal |
| History lookback | Last 2-3 1:1s |

**Prep Template:**

```markdown
# 1:1 with [Name]
**Time:** 3:00 PM - 3:30 PM

## Check-in
- How are they doing? (personal, workload, energy)

## Topics to Discuss
- [Career/growth items]
- [Feedback to give]
- [Support they need]

## Their Open Items
- [Things they're working on]
- [Blockers you can help with]

## Running Notes
- Last 1:1 (Jan 29): Discussed promotion path, agreed on...
- Two weeks ago: Mentioned feeling stretched on Project X
```

---

### Partnership Call

**Job to be done:** "Help me manage partner relationships effectively by tracking joint initiatives and maintaining context across conversations."

| Attribute | Value |
|-----------|-------|
| Classification | External attendees from known partner organizations |
| Prep depth | Moderate |
| History lookback | Last 2-3 partner interactions |

**Prep Template:**

```markdown
# [Partner Name] Sync
**Time:** 10:00 AM - 10:30 AM

## Partnership Context
- **Type:** Technology partner / Referral partner / SI
- **Agreement:** Active through Dec 2026
- **Owner:** [Your name]

## Current Joint Initiatives
- Integration v2 development (on track)
- Joint webinar series (planning)

## Since Last Meeting
- Completed API certification
- 2 joint deals in pipeline

## Discussion Topics
1. Integration timeline update
2. Q2 co-marketing plans
3. Customer feedback on joint solution

## Open Items
- [ ] Share updated API docs
- [ ] Intro to [Customer] for joint opportunity
```

---

### All Hands / Town Hall

**Job to be done:** "Just tell me when and where - I don't need prep for company-wide meetings."

| Attribute | Value |
|-----------|-------|
| Classification | Large attendee count (20+), "All Hands", "Town Hall" in title |
| Prep depth | None |
| History lookback | None |

**Prep Template:**

```markdown
# All Hands
**Time:** 4:00 PM - 5:00 PM
**Location:** Main conference room / Zoom

No prep needed - this is a company-wide broadcast.
```

---

### Personal

**Job to be done:** "Keep my personal appointments visible but don't clutter my work prep."

| Attribute | Value |
|-----------|-------|
| Classification | Personal calendar, non-work domain attendees |
| Prep depth | None |
| History lookback | None |

**Display:** Show time and title only, no prep card.

---

## Classification Priority

When multiple rules match, use this priority:

1. **Attendee count** (All Hands if 20+)
2. **Title keywords** (QBR, Training override)
3. **Attendee matching** (Account contacts → Customer)
4. **Domain analysis** (External vs internal)
5. **Default** → Internal meeting

---

## Data Requirements by Type

| Type | Account Data | History | Attendee Intel | Actions |
|------|--------------|---------|----------------|---------|
| Customer Call | ✓ Full | ✓ 2-3 meetings | ✓ Stakeholder map | ✓ Account actions |
| QBR | ✓ Full + trends | ✓ Full quarter | ✓ Full org chart | ✓ All account items |
| Training | ✓ Basic | ✓ Previous trainings | ✓ Attendee list | — |
| Internal Sync | — | ✓ Last meeting | — | ✓ My actions |
| 1:1 | — | ✓ 2-3 1:1s | ✓ Their context | ✓ Their items |
| Partnership | ✓ Partner record | ✓ 2-3 meetings | ✓ Partner contacts | ✓ Joint items |
| All Hands | — | — | — | — |
| Personal | — | — | — | — |

---

*Draft: 2026-02-05*
