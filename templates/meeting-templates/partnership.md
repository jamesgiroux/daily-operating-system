# Template: Partnership / Vendor Meeting

## When Used

Applied when external attendees match a known partner organization's domain. Partners are distinct from customers -- they are organizations you work *with*, not *for*. Examples: technology partners, referral partners, system integrators, vendors, agencies.

Partner matching is done by domain cross-reference against a partner registry (if maintained in the workspace). If no partner registry exists, this template may be triggered by title keywords like "Partner", "Vendor", "Integration", or "Joint" combined with external attendees.

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, attendee list, calendar description
2. **Partnership status notes** -- Search for files in `_reference/partners/` or any workspace location matching the partner name
3. **Joint initiative tracker** -- Any tracked projects, integrations, or co-marketing efforts with this partner
4. **Recent correspondence** -- Search archive for prior meetings with attendees from the same domain
5. **Account overlap** (CSM only) -- Accounts that use this partner's product/integration, relevant joint deals

## Output Sections

### 1. Partnership Context

Brief summary of the relationship:

- **Partner:** {Organization Name}
- **Type:** Technology partner / Referral partner / SI / Vendor / Agency
- **Relationship status:** Active / New / Dormant
- **Agreement:** {Active through date, if known}
- **Your role:** {Owner / Contributor / Attendee}

If the partner is not in a registry, note: "No formal partner record found. Relationship context based on meeting history."

### 2. Joint Initiative Status

Current shared projects or efforts:

| Initiative | Status | Last Update |
|-----------|--------|-------------|
| Integration v2 development | On track | API spec finalized Feb 1 |
| Joint webinar series | Planning | Waiting on their marketing team |
| Referral pipeline | Active | 2 deals in progress |

If no tracked initiatives exist: "No joint initiatives currently tracked."

### 3. Since Last Meeting

What has happened since the last interaction with this partner:

- Completed API certification process (Jan 20)
- Received 2 referral leads from their sales team (Jan 28)
- Their product shipped v3.0 with new integration hooks (Feb 1)

Source from archived meeting summaries with same-domain attendees. If no prior meetings exist: "No prior meeting history found with {partner domain}."

### 4. Discussion Points

3-5 suggested topics based on available context:

1. **Integration timeline** -- v2 API development progress and any blockers
2. **Co-marketing plans** -- Q2 webinar series logistics
3. **Customer feedback** -- What joint customers are saying about the integration
4. **Pipeline review** -- Status of referral deals in progress
5. **Roadmap alignment** -- Upcoming features that affect the partnership

If minimal context is available, generate generic partnership discussion points:
1. Relationship health check
2. Mutual commitments and timelines
3. Joint opportunity pipeline
4. Escalations or blockers

### 5. Open Items

Action items related to this partnership:

- [ ] Share updated API documentation (owner: you, due: Feb 7)
- [ ] Intro to {Customer} for joint opportunity (owner: partner, no due date)

If no items tracked: "No open items tracked for this partnership."

## Formatting Guidelines

- First line: `# {Partner Name} - {Meeting Title}`
- Second line: `**Time:** {start} - {end}`
- Target length: 200-300 words
- Tone: Professional, collaborative -- partnerships are peer relationships
- Avoid language that implies hierarchy (not "our vendor" but "our partner")
- Keep initiative tracking factual -- do not assess the partner's performance

## Profile Variations

- **CSM:** Add a section between Discussion Points and Open Items:

  ### Account Integration Context
  Accounts that are affected by or benefit from this partnership:
  - **Acme Corp** -- Uses partner's SSO integration, escalation pending on auth issues
  - **GlobalTech** -- Joint implementation in progress, partner providing SI services

  Also flag any joint escalations where a customer issue involves the partner.

- **General:** Skip account integration context. Focus on partnership deliverables, timelines, and mutual commitments. If the "partner" is really a vendor (you are the customer), adjust the tone: focus on what you need from them, SLA adherence, and upcoming renewals of the vendor contract.
