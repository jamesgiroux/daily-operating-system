# /email-scan - Email Inbox Triage

Systematically process your email inbox to surface what matters and archive noise.

## When to Use

Run during dedicated "inbox zero" blocks. This command:
- Fetches recent/unread emails
- Classifies by priority and type
- Surfaces important items requiring attention
- Helps draft responses
- Archives low-priority noise

## Philosophy

**Triage, not browse** - Process emails systematically rather than reactively scrolling.

**Surface what matters** - Important emails shouldn't compete for attention with newsletters.

**Draft, don't send** - Create drafts for review before sending anything.

## Execution Steps

### Step 1: Fetch Inbox

```bash
python3 .config/google/google_api.py gmail search "is:unread in:inbox" 50
```

Or for broader scan:
```bash
python3 .config/google/google_api.py gmail list 50
```

### Step 2: Classify Each Email

For each email, determine:

```python
def classify_email(email, known_domains):
    """
    Classify email by priority and type

    Returns: {
        'priority': 'HIGH' | 'MEDIUM' | 'LOW',
        'type': 'OPPORTUNITY' | 'INFORMATIONAL' | 'RISK' | 'ACTION_NEEDED',
        'category': 'customer' | 'internal' | 'newsletter' | 'automated',
        'action': 'respond' | 'review' | 'archive' | 'defer'
    }
    """
    sender_domain = extract_domain(email['from'])
    subject = email['subject'].lower()

    # HIGH priority indicators
    high_priority = (
        sender_domain in known_domains['customers'] or
        sender_domain in known_domains['leadership'] or
        any(word in subject for word in ['urgent', 'asap', 'deadline', 'critical'])
    )

    # LOW priority indicators
    low_priority = (
        sender_domain in known_domains['newsletters'] or
        'unsubscribe' in email.get('body', '').lower() or
        email.get('labels', []).contains('CATEGORY_PROMOTIONS')
    )

    # Type classification
    if any(word in subject for word in ['opportunity', 'expansion', 'upsell']):
        email_type = 'OPPORTUNITY'
    elif any(word in subject for word in ['concern', 'issue', 'problem', 'urgent']):
        email_type = 'RISK'
    elif any(word in subject for word in ['action', 'request', 'need', 'please']):
        email_type = 'ACTION_NEEDED'
    else:
        email_type = 'INFORMATIONAL'

    return {
        'priority': 'HIGH' if high_priority else ('LOW' if low_priority else 'MEDIUM'),
        'type': email_type,
        # ... etc
    }
```

### Step 3: Classification Rules

| Priority | Criteria | Icon |
|----------|----------|------|
| **HIGH** | From customer/client, from leadership, contains action words | ! |
| **MEDIUM** | Internal colleagues, meeting-related, P2 notifications | - |
| **LOW** | Newsletters, GitHub notifications (no @mention), automated | . |

| Type | Indicator | Icon |
|------|-----------|------|
| OPPORTUNITY | Expansion, new work, positive signal | + |
| INFORMATIONAL | FYI, status update, no action needed | i |
| RISK | Concern, complaint, churn signal, blocker | X |
| ACTION NEEDED | Explicit ask for you | ! |

### Step 4: Thread Summarization

For HIGH priority emails, especially threaded conversations:

```python
def summarize_thread(thread_id):
    """
    For threaded emails, fetch full thread and summarize
    """
    # Fetch thread
    thread = get_thread(thread_id)

    # Summarize
    summary = {
        'participants': extract_participants(thread),
        'initiated_by': thread[0]['from'],
        'topic': extract_topic(thread),
        'current_status': summarize_latest(thread[-1]),
        'action_for_you': identify_action(thread),
        'owner': determine_owner(thread),
    }

    return summary
```

**For each HIGH priority email, answer:**
- What's the conversation arc?
- Is there a specific ask for you?
- Who is the owner?
- What action (if any) should you take?

### Step 5: Generate Summary Output

Create summary grouped by priority:

```markdown
# Email Scan - [Date] [Time]

## HIGH Priority ([count])

### From: [Sender] - [Subject]
**Type**: [OPPORTUNITY / RISK / ACTION / INFO]
**Thread**: [X] messages since [date]

**Conversation Arc:**
[Who initiated, what's being discussed, where it stands]

**Ask for You:**
[Specific action requested, if any]

**Owner:**
[Who should handle this - you or someone else]

**Recommended Action:**
[What to do: Respond, Review, Delegate, Monitor]

---

### From: [Sender 2] - [Subject 2]
...

## MEDIUM Priority ([count])

| From | Subject | Type | Quick Note |
|------|---------|------|------------|
| [sender] | [subject] | INFO | [one-liner] |

## LOW Priority - Archived ([count])

Automatically archived:
- [X] newsletters
- [X] GitHub notifications
- [X] automated alerts
```

### Step 6: Offer Actions

For HIGH priority items:

```
"HIGH priority emails found: [X]

1. [Sender]: [Subject]
   Ask: [action requested]

   Options:
   - [Draft response]
   - [Add to task list]
   - [Mark as reviewed]
   - [Skip]

2. [Sender 2]: [Subject 2]
   ...
"
```

### Step 7: Draft Responses

If drafting requested:

```bash
python3 .config/google/google_api.py gmail draft \
  "recipient@example.com" \
  "Re: [Subject]" \
  "[Draft body]"
```

**IMPORTANT**: Creates draft only, does not send.

### Step 8: Archive Low Priority

```python
def archive_low_priority(emails):
    """
    Archive (remove from inbox) low priority emails
    """
    for email in emails:
        if email['priority'] == 'LOW':
            # Remove INBOX label
            remove_label(email['id'], 'INBOX')
            # Optionally add category label
            add_label(email['id'], 'auto-archived')
```

```
"Archive [X] low-priority emails?

- [X] newsletters
- [X] GitHub notifications (no @mention)
- [X] automated notifications

[Archive all / Select individually / Skip]"
```

### Step 9: Add to Daily Overview

If run as part of /today, results feed into `83-email-summary.md`:

```markdown
# Email Summary - [Date]

## Needs Attention ([X])

| From | Subject | Type | Action |
|------|---------|------|--------|
| [sender] | [subject] | ACTION | Draft response |
| [sender] | [subject] | RISK | Review with team |

## Summary

- **High Priority**: [X] (responded: [Y], pending: [Z])
- **Medium Priority**: [X] (labeled for later)
- **Archived**: [X] (newsletters, automated)

## Drafts Created

1. Re: [Subject] → To: [recipient]
   Status: In drafts, awaiting review
```

## Domain Configuration

Configure known domains in your CLAUDE.md or a config file:

```yaml
email_domains:
  customers:
    - clienta.com
    - clientb.org
  leadership:
    - mycompany.com
  newsletters:
    - substack.com
    - mailchimp.com
    - marketing.*
  automated:
    - github.com
    - notifications.*
```

## Dependencies

**APIs:**
- Gmail (read, draft, labels)

**Configuration:**
- Domain classification rules
- Account/customer domain mapping

## Related Commands

- `/today` - Integrates email scan into daily overview
- `/wrap` - End-of-day closure
