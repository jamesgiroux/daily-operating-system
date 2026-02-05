# /quarter - Quarterly Pre-Population

Prepare for quarterly review by aggregating evidence and pre-filling templates.

## When to Use

Run 1-2 weeks before quarterly review cycle. This command:
- Aggregates all monthly reports for the quarter
- Pre-fills quarterly review template with evidence
- Identifies achievements and growth areas
- Prepares talking points for review conversations

## Philosophy

**Evidence over memory** - Build your quarterly narrative from documented achievements, not recollection.

**Pre-populate, then polish** - The system does the heavy lifting of gathering evidence; you add the narrative and context.

## Execution Steps

### Step 1: Identify Quarter and Timeframe

```python
from datetime import datetime

current_date = datetime.now()
quarter = (current_date.month - 1) // 3 + 1
year = current_date.year

# Quarter date ranges
quarter_ranges = {
    1: ('01-01', '03-31'),
    2: ('04-01', '06-30'),
    3: ('07-01', '09-30'),
    4: ('10-01', '12-31'),
}
```

### Step 2: Load Monthly Reports

```bash
# Find monthly reports for current quarter
# Q1: Jan, Feb, Mar → months 01, 02, 03
# Q2: Apr, May, Jun → months 04, 05, 06
# etc.

ls Leadership/impact/ | grep "monthly-report"
```

### Step 3: Aggregate Quarterly Data

Combine all monthly reports into quarterly view:

```python
def aggregate_quarterly_data(monthly_files):
    """
    Combine monthly reports into quarterly aggregate
    """
    quarterly = {
        'total_meetings': 0,
        'accounts_engaged': set(),
        'value_delivered': [],
        'relationships': [],
        'contributions': [],
        'learnings': [],
        'challenges': [],
    }

    for month_file in monthly_files:
        month_data = parse_monthly_report(month_file)
        # Aggregate...

    return quarterly
```

### Step 4: Calculate Key Metrics

```python
metrics = {
    'customer_meetings': total_meetings,
    'accounts_touched': len(accounts_engaged),
    'value_items': len(value_delivered),
    'action_completion_rate': completed / total_actions,
    'relationship_depth': new_stakeholders_met,
}
```

### Step 5: Generate Quarterly Template

Create `Leadership/reviews/[YYYY]-Q[N]-review.md`:

```markdown
---
area: Leadership
doc_type: quarterly-review
status: draft
date: [YYYY-MM-DD]
quarter: Q[N] [YYYY]
tags: [review, quarterly, performance]
---

# Quarterly Review - Q[N] [YYYY]

## Executive Summary

[To be completed - 2-3 sentence summary of the quarter]

## By the Numbers

| Metric | Q[N] | Trend |
|--------|------|-------|
| Customer Meetings | [X] | |
| Accounts Engaged | [X] | |
| Value Items Delivered | [X] | |
| Action Completion Rate | [X]% | |

## Key Achievements

### Value Delivered

*Pre-populated from monthly reports - curate top items:*

| Month | Account | Achievement |
|-------|---------|-------------|
| [Month] | [Account] | [Description] |

**Highlight 1:** [Expand on most significant achievement]

**Highlight 2:** [Second most significant]

**Highlight 3:** [Third]

### Relationship Growth

*From monthly relationship progress:*

- [Relationship achievement 1]
- [Relationship achievement 2]

### Cross-Functional Impact

*From monthly contributions:*

- [Contribution 1]
- [Contribution 2]

## Growth Areas

### Skills Developed

- [Skill 1]
- [Skill 2]

### Challenges Overcome

*From monthly challenges:*

- [Challenge 1] → [How addressed]

### Areas for Development

- [Area 1]
- [Area 2]

## Looking Ahead - Q[N+1]

### Priorities

1. [Priority 1]
2. [Priority 2]
3. [Priority 3]

### Key Goals

| Goal | Metric | Target |
|------|--------|--------|
| [Goal 1] | [Metric] | [Target] |

### Development Focus

- [Development area]

## Talking Points for Review

*Pre-populated based on achievements:*

1. **[Topic 1]**: [Key point to discuss]
2. **[Topic 2]**: [Key point to discuss]
3. **[Topic 3]**: [Key point to discuss]

## Supporting Evidence

| Document | Location | Relevant For |
|----------|----------|--------------|
| [Month] Monthly | `Leadership/impact/[file]` | Value delivered |
| [Meeting Summary] | `Accounts/[path]` | Achievement context |

---
*Pre-populated from Q[N] monthly reports*
*Last updated: [Date]*
```

### Step 6: Prompt for Curation

```
"Quarterly review pre-populated for Q[N]:

- [X] achievements aggregated
- [X] metrics calculated
- [X] talking points suggested

Review and curate?

The template includes all captured data. You should:
1. Select top 3-5 achievements to highlight
2. Add narrative context
3. Identify growth areas
4. Set Q[N+1] goals

Open template in editor? [Yes / Show summary first]"
```

### Step 7: Create Reference Links

Add links to source documents for easy reference during review conversations.

## Output Structure

```
Leadership/
├── impact/
│   ├── 2026-01-monthly-report.md
│   ├── 2026-02-monthly-report.md
│   └── 2026-03-monthly-report.md
└── reviews/
    └── 2026-Q1-review.md          # NEW
```

## Dependencies

**Data Sources:**
- `Leadership/impact/[YYYY]-[MM]-monthly-report.md` files
- Account meeting summaries (for context)
- Action item history

## Related Commands

- `/month` - Monthly roll-up (creates source data)
- `/week` - Weekly review (feeds monthly)
- `/today` - Daily operations
