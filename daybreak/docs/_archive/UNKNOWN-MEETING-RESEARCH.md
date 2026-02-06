# Unknown Meeting Research

> When we don't have context for a meeting, we do the homework - not the user.

---

## Philosophy

From `PRINCIPLES.md`:

> **The system operates. You leverage.**

If we encounter an external meeting with no account match, no history, no context - we don't punt to the user with "add context." We research.

**Bad:** "Unknown meeting - please add context"
**Good:** "Here's what we found about Globex Corp and the attendees"

---

## Research Hierarchy

When a meeting has unknown external attendees, we attempt research in this order:

### Level 1: Local Search (Fast, Always)

```
1. Grep archive for attendee email domains
   → "Found 3 mentions of @globex.com in archive"

2. Grep archive for attendee names
   → "Found reference to 'Jane Smith' in project notes"

3. Check _inbox for related items
   → "Found email thread with jane@globex.com"

4. Search project folders for company name
   → "Found Globex mentioned in 1-projects/partnership-exploration/"
```

### Level 2: Web Research (Slower, Phase 2)

If local search yields nothing, Claude Code performs web research:

```
1. Company lookup
   → Company website, LinkedIn company page
   → What do they do? Size? Industry?

2. Attendee lookup
   → LinkedIn profiles (name + company)
   → Role, background, mutual connections

3. News/context
   → Recent company news
   → Industry context
```

### Level 3: Inference (Fallback)

If web research is limited:

```
1. Domain analysis
   → "globex.com appears to be a mid-size tech company based on..."

2. Meeting title parsing
   → "Partnership Discussion" suggests business development context

3. Attendee count/pattern
   → "3 external attendees suggests this is a formal meeting, not casual"
```

---

## Implementation

### Phase 1 (Prepare): Local Research

```python
def research_unknown_meeting(meeting: dict, workspace: Path) -> dict:
    """
    Gather whatever context we can from local sources.
    """
    external_attendees = meeting['external_attendees']
    domains = {extract_domain(e) for e in external_attendees}

    research = {
        'archive_mentions': [],
        'inbox_threads': [],
        'project_references': [],
    }

    # Search archive
    for domain in domains:
        matches = grep_archive(workspace, f"@{domain}")
        research['archive_mentions'].extend(matches[:5])

    for attendee in external_attendees:
        name = extract_name(attendee)
        if name:
            matches = grep_archive(workspace, name)
            research['archive_mentions'].extend(matches[:3])

    # Search inbox
    for email in external_attendees:
        threads = find_inbox_threads(workspace, email)
        research['inbox_threads'].extend(threads[:3])

    # Search projects
    company_name = infer_company_name(domains)
    if company_name:
        matches = grep_projects(workspace, company_name)
        research['project_references'].extend(matches[:3])

    return research
```

### Phase 2 (Enrich): Web Research

The directive includes an AI task for unknown meetings:

```json
{
  "type": "research_unknown_meeting",
  "priority": "medium",
  "meeting_id": "ev_789",
  "attendees": ["jane.smith@globex.com", "bob.jones@globex.com"],
  "company_domain": "globex.com",
  "local_research": {
    "archive_mentions": [],
    "inbox_threads": [],
    "project_references": []
  }
}
```

Claude Code then:

1. Searches for company information
2. Looks up attendees on LinkedIn
3. Synthesizes into a prep brief

### Output Structure

```json
{
  "meeting_id": "ev_789",
  "research_depth": "web",  // "local", "web", or "inference"

  "company": {
    "name": "Globex Corporation",
    "website": "https://globex.com",
    "industry": "Enterprise Software",
    "size": "500-1000 employees",
    "description": "Globex provides supply chain management software...",
    "source": "LinkedIn Company Page"
  },

  "attendees": [
    {
      "name": "Jane Smith",
      "email": "jane.smith@globex.com",
      "role": "Director of Partnerships",
      "linkedin": "https://linkedin.com/in/janesmith",
      "background": "Previously at Acme Corp, 10 years in partnerships",
      "source": "LinkedIn"
    }
  ],

  "context_notes": [
    "This appears to be an initial partnership exploration meeting",
    "Globex recently announced expansion into our market segment",
    "No prior relationship found in workspace"
  ],

  "suggested_prep": {
    "talking_points": [
      "Understand what partnership model they're exploring",
      "Learn about their customer base and overlap",
      "Identify potential mutual value"
    ],
    "questions": [
      "What prompted this conversation?",
      "What does partnership success look like for Globex?",
      "Who else is involved in partnership decisions?"
    ]
  }
}
```

---

## Prep Output for Unknown Meetings

Even without account context, we generate useful prep:

```markdown
# Meeting: Partnership Discussion with Globex

**Time:** 3:00 PM - 3:30 PM
**Research depth:** Web search (no prior relationship found)

## About Globex Corporation

Globex provides supply chain management software for mid-market manufacturers.
~750 employees, headquartered in Chicago. Recently raised Series C funding
and announced expansion into enterprise segment.

## Attendees

- **Jane Smith** - Director of Partnerships
  Previously at Acme Corp for 10 years in partnership roles.
  LinkedIn: linkedin.com/in/janesmith

- **Bob Jones** - VP Business Development
  Joined Globex 2 years ago from Oracle.
  LinkedIn: linkedin.com/in/bobjones

## Meeting Context

This appears to be an initial partnership exploration. No prior interactions
found in workspace. Meeting was initiated by Globex (based on calendar invite).

## Suggested Approach

Since this is a first meeting with no established context:

1. **Discovery focus** - Understand what they're looking for
2. **Listen first** - Let them explain the opportunity
3. **Find mutual value** - What can each side bring?

## Questions to Explore

- What prompted reaching out about partnership?
- What does partnership success look like for Globex?
- What's your timeline for partnership decisions?
- Who else is involved in evaluating partnerships?

---

*Note: This prep was generated from web research. No account record exists for Globex.*
```

---

## When Research Fails

If we find nothing (no local matches, web search fails):

```markdown
# Meeting: Call with Unknown Attendees

**Time:** 4:00 PM - 4:30 PM
**Research depth:** Limited (no information found)

## What We Know

- **Attendees:** person@unknowndomain.xyz
- **Title:** "Quick Call"
- **No prior relationship** found in workspace
- **Web search** returned no results for domain or attendees

## Recommendation

This meeting has no available context. Consider:

1. Check the calendar invite for additional details
2. Review any email thread that led to this meeting
3. Ask the organizer for context before the call

---

*Note: Unable to gather prep context for this meeting.*
```

Even here, we've done the work. We searched. We came up empty. We're transparent about it.

---

## Integration with Meeting Types

| Meeting Type | Research Level |
|--------------|----------------|
| Customer (matched) | None needed - use account context |
| Customer (domain match only) | Local search for prior interactions |
| External (unknown) | Full research: local + web |
| Partnership (matched) | Use partner record |
| Partnership (new) | Full research: local + web |
| Internal | None needed |

---

## Privacy Considerations

- Web research is read-only (no accounts created, no tracking)
- LinkedIn lookups use public profiles only
- Company info from public sources
- No data stored externally - all research results stay local

---

*Document Version: 1.0*
*Created: 2026-02-05*
