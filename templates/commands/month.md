# /month - Monthly Roll-Up

Aggregate weekly impacts into a monthly report and prepare for the upcoming month.

## When to Use

Run on the last Friday of each month OR first Monday of next month. This command:
- Aggregates all weekly impact captures into monthly summary
- Identifies patterns across weeks
- Prepares monthly report for stakeholders
- Sets up next month's tracking

## Philosophy

**Compound small gains** - Individual daily wins become weekly impacts become monthly achievements. This command surfaces the cumulative effect.

**Evidence-based narrative** - Build your monthly story from actual captured data, not memory.

## Execution Steps

### Step 1: Identify Month and Weekly Files

```python
from datetime import datetime

current_date = datetime.now()
month = current_date.month
year = current_date.year

# Find all weekly impact files for this month
# Files named: YYYY-W[NN]-impact-capture.md
# Need to identify which weeks fall within this month
```

### Step 2: Load Weekly Impact Files

```bash
# Find all weekly files for current month
ls Leadership/impact/ | grep "$YEAR-W"
```

Parse each weekly file for:
- Customer meetings and outcomes
- Value delivered
- Relationship progress
- Cross-functional contributions
- Key learnings

### Step 3: Aggregate Impacts by Category

```python
def aggregate_monthly_impacts(weekly_files):
    """
    Combine all weekly impacts into monthly categories
    """
    monthly = {
        'meetings': [],
        'value_delivered': [],
        'relationships': [],
        'contributions': [],
        'learnings': [],
    }

    for week_file in weekly_files:
        week_data = parse_weekly_impact(week_file)

        monthly['meetings'].extend(week_data.get('meetings', []))
        monthly['value_delivered'].extend(week_data.get('value', []))
        monthly['relationships'].extend(week_data.get('relationships', []))
        monthly['contributions'].extend(week_data.get('contributions', []))
        monthly['learnings'].extend(week_data.get('learnings', []))

    return monthly
```

### Step 4: Identify Patterns and Highlights

**Prompt for curation:**
```
"I've aggregated your impacts for [Month]:

Meetings: [X] customer meetings across [Y] accounts
Value items: [X] captured
Relationships: [X] items
Contributions: [X] items

Would you like me to:
1. Auto-generate monthly summary
2. Let you curate the highlights first
3. Show raw aggregation for review
"
```

### Step 5: Generate Monthly Report

Create `Leadership/impact/[YYYY]-[MM]-monthly-report.md`:

```markdown
---
area: Leadership
doc_type: impact-report
status: draft
date: [YYYY-MM-DD]
month: [Month Year]
tags: [impact, monthly]
---

# Monthly Impact Report - [Month Year]

## Executive Summary

[Auto-generated 2-3 sentence summary of the month]

## By the Numbers

| Metric | Count | Notes |
|--------|-------|-------|
| Customer Meetings | [X] | Across [Y] accounts |
| Value Items Delivered | [X] | |
| Action Items Completed | [X] | |
| New Relationships | [X] | |

## Value Delivered

### Customer/Client Wins

| Week | Account | Win |
|------|---------|-----|
| W[X] | [Account] | [Description] |
| W[Y] | [Account] | [Description] |

### Technical Outcomes

- [Outcome 1]
- [Outcome 2]

## Relationship Progress

### Stakeholder Engagement

- [Engagement 1]
- [Engagement 2]

### New Connections

- [Connection 1]

## Cross-Functional Contributions

- [Contribution 1]
- [Contribution 2]

## Key Learnings

- [Learning 1]
- [Learning 2]

## Challenges Faced

- [Challenge 1]
- [Challenge 2]

## Looking Ahead - [Next Month]

### Priorities
1. [Priority 1]
2. [Priority 2]

### Key Dates
- [Date]: [Event]

---
*Generated from W[X]-W[Y] weekly impact captures*
```

### Step 6: Update Status to Final

Once reviewed, update frontmatter status from `draft` to `final`.

### Step 7: Prepare Next Month

```
"Monthly report generated. Set up tracking for [Next Month]?"

If yes:
- Create placeholder weekly files for next month
- Carry forward any incomplete items
```

## Output Structure

```
Leadership/impact/
├── 2026-W01-impact-capture.md
├── 2026-W02-impact-capture.md
├── 2026-W03-impact-capture.md
├── 2026-W04-impact-capture.md
└── 2026-01-monthly-report.md    # NEW
```

## Dependencies

**Data Sources:**
- `Leadership/impact/[YYYY]-W[NN]-impact-capture.md` files

## Related Commands

- `/week` - Weekly review (creates the source files)
- `/quarter` - Quarterly pre-population
- `/today` - Daily operations
