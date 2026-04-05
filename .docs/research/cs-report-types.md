# CS Report Types — Reference for v0.14.0 Report Implementation

This document covers the CS report types DailyOS will generate, their purpose, structure, and the `intelligence.json` fields each requires. Use this as the authoritative reference when building I396 (intelligence fields), I397 (report infrastructure), I399 (Account Health Review), and I400 (EBR/QBR).

---

## 1. Executive Business Review (EBR / QBR)

**Frequency:** Quarterly (bi-annual for smaller accounts)
**Audience:** Customer executives + internal CS leadership
**Purpose:** Demonstrate value delivered, align on strategy, secure renewal or expansion commitment
**When generated:** 2–3 weeks before the review meeting

### Sections

1. **Partnership overview** — tenure, ARR, team, account type
2. **Goals recap** — commitments made at last review or onboarding
3. **Value delivered** — measurable outcomes, adoption wins, ROI evidence, case study moments
4. **Success metrics** — KPI dashboard: targets vs. actuals for agreed metrics
5. **Challenges & resolutions** — what went wrong, how it was addressed (honesty builds credibility)
6. **Strategic roadmap** — what's coming from vendor; how it serves customer's goals
7. **Customer asks** — open feature requests, support needs, resource requests
8. **Next period priorities** — 3–5 agreed action items with owners and dates

### Intelligence fields required

`value_delivered`, `success_metrics`, `open_commitments`, `health_trend`, `strategic_programs`, `stakeholder_insights`

### Notes

The EBR is the highest-stakes CS deliverable. A pre-populated draft from DailyOS intelligence that the CSM reviews and edits before the meeting is the flagship use case for the reports infrastructure. Quality here determines whether CS teams adopt DailyOS reports at all.

---

## 2. Account Health Review

**Frequency:** Monthly (internal team), quarterly (manager/leadership)
**Audience:** CSM, CS manager, CS leadership
**Purpose:** Assess account health, identify risk/opportunity signals, plan interventions before they become escalations
**When generated:** Weekly or monthly — quick scan

### Sections

1. **Health summary** — one paragraph executive assessment with trend direction
2. **Health score & trend** — normalized score, trajectory (improving / stable / declining / volatile)
3. **Key risks** — top 3–5 active risk signals with urgency rating
4. **Stakeholder coverage** — champion strength, executive access, contacts with no recent engagement
5. **Engagement cadence** — meeting frequency, email response time, last meaningful interaction
6. **Open commitments & actions** — unresolved items from both sides
7. **Renewal outlook** — if renewal intelligence exists

### Intelligence fields required

`health_score`, `health_trend`, `risks`, `relationship_depth`, `open_commitments`, `stakeholder_insights`

### Notes

Simpler than EBR. Purely internal. Validates the report infrastructure before tackling EBR complexity. Good first report to implement — low AI risk, high structural clarity.

---

## 3. Risk Report / At-Risk Briefing

**Frequency:** On-demand when an account enters at-risk state
**Audience:** CSM manager, CS VP, Account Executive, sometimes escalation team
**Purpose:** Escalate a risk, align on action, request resources or executive engagement

**Current status:** EXISTS in DailyOS as a 6-slide SCQA briefing (Cover → Bottom Line → What Happened → The Stakes → The Plan → The Ask). Pre-ADR-0086: reads from disk files, runs a separate PTY call, stores in `risk-briefing.json`.

**Migration scope (I397):** Read entity intelligence from DB (`entity_intel` table), store output in `reports` table, invalidate when entity intel updates. Sections are unchanged.

### Sections

1. **Cover** — account name, risk level, escalation date, owner
2. **Bottom line** — the one thing leadership needs to know
3. **What happened** — factual timeline, signal history
4. **The stakes** — ARR at risk, relationship impact, strategic implications
5. **The plan** — specific action items, owners, timeline
6. **The ask** — what the CSM needs from leadership

### Intelligence fields required

`risks`, `health_trend`, `stakeholder_insights`, meeting history (already in `entity_intel`)

---

## 4. Renewal Readiness Assessment

**Frequency:** Generated 90–120 days before renewal date
**Audience:** CSM, Account Executive, CS manager
**Purpose:** Assess renewal probability, surface risks, align internal team on plan

### Sections

1. **Renewal profile** — date, ARR, contract type, risk rating
2. **Health assessment & trend** — current state with trajectory
3. **Champion & executive alignment** — strength of sponsorship, decision-maker access
4. **Competitive landscape** — known competitive presence or evaluation
5. **Open items before renewal** — what must be resolved to secure renewal
6. **Recommended actions** — prioritized steps with owners

### Intelligence fields required

`renewal_context` (from preset metadata), `health_trend`, `relationship_depth`, `competitive_context`, `open_commitments`

### Notes

v0.14.1 candidate — requires `renewal_context` field and `competitive_context` which may not be in initial I395 scope. Defer until both fields are available.

---

## 5. Stakeholder Map

**Frequency:** Created at onboarding, updated quarterly
**Audience:** Internal CS/Sales team
**Purpose:** Visualize org structure, identify coverage gaps, plan expansion or renewal strategy

### Sections

1. **Contact roster** — all known contacts with title, role, last interaction
2. **Champion / sponsor / blocker identification** — typed relationships
3. **Engagement level per contact** — active / warm / cold / unknown
4. **Coverage gaps** — buying committee members with no engagement
5. **Recommended next actions** — who to engage and why

### Intelligence fields required

`stakeholder_insights` (expanded), `relationship_depth`

### Notes

v0.14.1 candidate — benefits significantly from I390–I392 (people relationship graph from v0.13.5). Can be built without it using `stakeholder_insights` only, but the relationship graph makes it substantially richer. Do not build this until I390–I392 land.

---

## 6. Book of Business Overview

**Frequency:** Monthly (CSM self-view), weekly (manager view)
**Audience:** CS manager, CS leadership
**Purpose:** Portfolio health visibility across all accounts — identify patterns, capacity planning, risk concentration

### Sections

1. **Portfolio health distribution** — healthy / at-risk / expanding by count and ARR
2. **Renewals coming up** — next 90/180 days with health rating
3. **Accounts needing attention** — at-risk or declining accounts
4. **Expansion opportunities** — accounts with positive signals
5. **Engagement gaps** — accounts without recent meetings

### Notes

This is a multi-entity, portfolio-level report. Requires parent account hierarchy (v0.13.3 I384/I393) for parent/BU accounts. v0.15.0 candidate. Do not attempt this in v0.14.x — it is architecturally distinct from single-entity reports.

---

## 7. Success Plan

**Frequency:** Created at onboarding, reviewed quarterly
**Audience:** Customer-facing
**Purpose:** Document mutual commitments, success criteria, and ownership

### Sections

1. **Shared objectives** — what success looks like for the customer
2. **Success metrics** — agreed KPIs with targets
3. **Milestones & timeline** — key checkpoints
4. **Responsibilities** — what vendor delivers, what customer commits to
5. **Progress-to-date**

### Notes

v0.14.1 candidate. Requires `success_metrics` field. Success plans are living documents — the editable/collaborative aspect matters more here than in other report types. The inline editing capability in `ReportShell` is particularly important for this one.

---

## v0.14.0 CS Report Priority Order

| Priority | Report | Reason |
|----------|--------|--------|
| 1 | EBR/QBR | Highest business value, differentiating, flagship use case |
| 2 | Account Health Review | Internal, simpler, validates infrastructure before EBR |
| 3 | Risk Report (migration) | Existing feature, ADR-0086 alignment |
| 4 | SWOT | Quick win, useful for EBR prep, bundled into infrastructure |

---

## Intelligence Fields Summary

| Field | Type | Reports Using It | Priority |
|-------|------|-----------------|----------|
| `health_score` | number (0–100) | Health Review, Renewal Readiness | v0.14.0 |
| `health_trend` | enum + rationale | EBR, Health Review, Risk, Renewal | v0.14.0 |
| `value_delivered` | array of outcomes | EBR, Success Plan | v0.14.0 |
| `success_metrics` | array of KPIs | EBR, Success Plan, Renewal | v0.14.0 |
| `open_commitments` | array from signals | EBR, Health Review, Renewal | v0.14.0 |
| `relationship_depth` | structured object | Health Review, Stakeholder Map, Renewal | v0.14.0 |
| `renewal_context` | object (date, ARR, risk) | Renewal Readiness | v0.14.1 |
| `competitive_context` | array | Renewal Readiness, Account Plan | v0.14.1 |
