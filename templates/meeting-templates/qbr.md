# Template: Quarterly Business Review (QBR)

## When Used

Applied when the meeting title contains "QBR", "Business Review", or "Quarterly Review" (case-insensitive). This is the most comprehensive prep template. QBRs are high-stakes, executive-facing meetings that require full account context and trend analysis.

Title keyword overrides attendee-based classification -- a meeting titled "Acme QBR" will use this template even if the attendee matching would classify it as a regular customer call.

## Required Context (from directive refs)

Claude should read ALL of the following. QBR prep demands depth.

1. **Full account dashboard** -- `Accounts/{account}/dashboard.md` -- Every metric, every section
2. **90-day meeting history** -- All archived meeting summaries for this account from the past quarter (`_archive/*/XX-HHMM-*{account}*`)
3. **Account tracker data** -- From Google Sheets (if available in directive's `account_data`) -- ARR changes, user counts, adoption metrics over time
4. **Stakeholder map** -- `Accounts/{account}/stakeholders.md` -- Full org chart context, especially executive attendees
5. **Strategic programs** -- `Accounts/{account}/programs.md` or `strategy.md` -- Initiative status, milestones, blockers
6. **Account actions** -- Complete action history, not just open items -- shows execution track record
7. **Meeting event data** -- From directive JSON: title, time, duration, full attendee list, calendar description

If any ref is missing, note it explicitly. A QBR with gaps in data should flag those gaps so the user can fill them before the meeting.

## Output Sections

Generate all sections in order. QBR prep is long by design -- this is the one meeting type where thoroughness matters more than brevity.

### 1. Executive Summary

One paragraph (4-6 sentences) summarizing the account's quarter. Written for the user to internalize, not to present directly. Cover: overall health trajectory, biggest win, biggest concern, and the strategic question going into the QBR.

### 2. Account Overview

Key metrics table with quarter-over-quarter comparison:

| Metric | Previous Quarter | This Quarter | Trend |
|--------|-----------------|--------------|-------|
| ARR | $400K | $450K | Up |
| Active Users | 245 | 312 | Up |
| Health Score | Yellow | Yellow | Flat |
| NPS/CSAT | (if available) | | |

Below the table, a 2-3 sentence narrative on what the numbers mean. "ARR grew 12% driven by the Platform team expansion. Health remains Yellow due to unresolved APAC adoption issues despite user growth."

If quarter-over-quarter data is not available, show current state only and note: "Historical comparison unavailable. Showing current state."

### 3. Quarter Summary

Chronological highlights from the past 90 days, organized by month:

**January:**
- Completed Phase 1 integration (Jan 12)
- Resolved escalation on auth issues (Jan 18)

**February:**
- Onboarded 3 new teams (Feb 5)
- Champion flagged concerns about mobile access (Feb 14)

**March:**
- Regional training delivered to APAC (Mar 2)

Source from archived meeting summaries and action completion records.

### 4. Strategic Program Status

For each tracked program/initiative:

| Program | Status | Key Update |
|---------|--------|------------|
| Phase 1 Rollout | Complete | Delivered Jan 12, all KPIs met |
| API Integration | In Progress | 60% complete, ETA March 15 |
| Enterprise SSO | Blocked | Waiting on customer IT team |

Include a sentence of context for any blocked or at-risk programs.

### 5. Risks and Mitigation

More detailed than a regular customer call. For each risk:

**Risk:** Champion (Sarah Chen) transitioning roles in Q2
**Impact:** Loss of technical advocacy, potential adoption stall
**Mitigation:** Identify successor, schedule transition meeting, document institutional knowledge
**Status:** Needs action before QBR

Limit to 3-5 risks, prioritized by impact.

### 6. Value Delivered

Concrete outcomes to present in the QBR. These should be quantifiable where possible:

- Reduced ticket resolution time by 23% (measured Jan-Mar)
- Onboarded 67 new users across 3 teams
- Achieved 99.7% uptime during migration

Frame as "value we delivered together" not "value we gave you." QBRs are partnerships.

### 7. Renewal Position (CSM only)

- Contract end date
- Renewal conversation target date
- Expansion opportunities (specific: "+2 teams, ~$50K")
- Contraction risks (specific: "APAC team may not renew their seats")
- Competitive threats (if known)

### 8. Discussion Agenda

Suggested QBR agenda with time allocations:

1. Value review (10 min)
2. Challenges and solutions (10 min)
3. Roadmap alignment (15 min)
4. Success planning (15 min)
5. Next steps (10 min)

Note: This is a suggested structure. The user may already have a QBR deck or format. This agenda helps them organize their thinking, not replace their process.

### 9. Stakeholder Prep

For each attendee, especially executives:

- **Sarah Chen** (VP Engineering) -- Technical champion. Likely to ask about API timeline. Has expressed frustration with mobile gaps.
- **David Park** (CFO) -- First time in a QBR. Will focus on ROI. Prepare cost-avoidance narrative.

Flag any new attendees or unexpected additions to the invite.

### 10. Appendix: Detailed Metrics (optional)

If detailed metrics are available from tracker data, include them here. This section is reference material, not narrative:

- Monthly active user counts
- Feature adoption percentages
- Support ticket trends
- Integration usage stats

## Formatting Guidelines

- Use markdown headers (`##`) for each section
- First line: `# {Account Name} - Quarterly Business Review`
- Second line: `**Time:** {start} - {end} | **Quarter:** {Q# YYYY}`
- QBR preps can be 800-1200 words -- this is acceptable
- Write in second person, strategic tone
- Quantify everything possible -- QBRs live on numbers
- Flag data gaps explicitly: "(Data unavailable -- check with [source])"

## Profile Variations

- **CSM:** All 10 sections. Full metrics, renewal analysis, expansion/contraction sizing, competitive intelligence. This is the flagship prep document for a CSM.

- **General:** This template is rarely triggered for General profile users. If it is (e.g., a "Project Review" meeting misclassified by title keywords), adapt to a project review format: skip sections 2, 7, and 10. Replace "Account Overview" with "Project Status." Replace "Value Delivered" with "Milestones Achieved." Replace "Renewal Position" with "Next Phase Planning."
