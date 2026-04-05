# Weekly Commercial Update Report: Feasibility Research

**Date:** 2026-03-02
**Status:** Research — blocked on Glean/Salesforce authentication validation
**Context:** A real weekly attrition/expansion Slack report was analyzed to determine if DailyOS could replicate and improve on it.

---

## The Report

A VP-level weekly Slack post tracking:
- **Attrition by product line** (CMS, Analytics): budget vs confirmed vs at-risk, WoW deltas, per-account movements
- **Expansion pipeline**: Closed/Commit/Probable/Best with WoW deltas
- **Multi-quarter view**: current quarter + next quarter
- **Highlights**: saves (accounts pulled from attrition) and surprises (unexpected churn without prior at-risk signal)

This is produced manually each week. Someone aggregates Salesforce data, calls RMs for status updates, computes WoW deltas against last week's numbers, and formats a Slack message.

## Three Data Layers

| Layer | What | Source | DailyOS Can Provide? |
|-------|------|--------|---------------------|
| **Structured financials** | ARR, budget targets, pipeline stages, product line | Salesforce (CRM) | Only via Glean — blocked today |
| **Status transitions** | WoW deltas, At Risk → Confirmed, surprise flags | CRM + human judgment | Partially — health signals detect movement, but dollar-precise transitions need CRM |
| **Narrative intelligence** | WHY accounts moved, what led to the surprise, what signals were missed | DailyOS intelligence | **Yes — this is the unique value add** |

## Where DailyOS Adds Value the Manual Report Can't

The manual report is numbers without narrative. It says:

> Le Conservateur $29.6k SURPRISE attrition

DailyOS could say:

> **Le Conservateur — $29.6k SURPRISE attrition.** Champion went silent 6 weeks ago. Final QBR was positive, but decision-maker (CFO) absent for second consecutive meeting. Health score declined 72→41 over 8 weeks. Engagement cadence flagged declining meeting quality (monologue risk, low question density). No at-risk signal was raised because CSM's last meeting notes were optimistic — but the interaction dynamics data told a different story.

That narrative layer — cross-referencing health trends, stakeholder engagement, meeting dynamics (I509), and signal history — is what no one has time to produce manually for 40+ accounts every week.

Similarly, for saves:

> AXIOS $34k removed / SAVED!!

Becomes:

> **AXIOS — $34k SAVED.** Executive sponsor re-engaged after VP escalation on Feb 12. Three follow-up meetings in 2 weeks, champion sentiment shifted from cautious to positive. Health score recovered 38→67. Key factor: competitive evaluation of [competitor] paused after custom migration plan presented.

## Consumption Patterns by Org Level

The insight from analyzing this report: **different org levels consume the same underlying data differently.**

| Level | Primary Interest | Detail Level | Follow-up Pattern |
|-------|-----------------|-------------|-------------------|
| **IC (CSM/RM)** | My accounts — what changed, what do I do next | Deep — per-account, per-stakeholder | Doesn't need this report; lives in the detail daily |
| **Territory Lead** | My territory — which accounts moved, which ICs need help | Medium — account-level movements, exception-driven | "Tell me more about [account]" → drills into IC intelligence |
| **VP** | The book — are we on track, what surprises, what do I escalate | High — financial aggregates, saves/surprises, trends | "Why did [account] churn without warning?" → drills into dynamics |

**Key insight:** ICs don't need this report format — they're already in the detail. Leads and VPs need it because they're managing across accounts they don't personally touch. The **drill-down** is where DailyOS shines: when a VP sees a surprise or a big save, they want to understand WHY, and that narrative intelligence is exactly what DailyOS produces.

This suggests the report has two modes:
1. **The roll-up** (numbers, deltas, highlights) — requires CRM data
2. **The drill-down** (narrative intelligence behind each movement) — this is DailyOS's lane

## Glean/Salesforce Blocker

**Status:** Glean can query Salesforce via SOQL, but Salesforce authentication is not configured in the current workspace.

**What Glean reported:**
- Glean has an internal Salesforce Search action that accepts SOQL-style queries
- The action returned `401 "SoqlAction is not authenticated"`
- This is a one-time workspace-level authentication setup — once connected, Glean can pull ARR, pipeline stages, opportunity amounts, and account metadata from Salesforce
- Glean designed a prompt that would recreate the full report, but couldn't execute it without auth

**What this means for DailyOS:**
- If Salesforce is authenticated in Glean, DailyOS could query Glean for CRM data to populate the financial scaffolding
- The structured financial data (ARR, budget, pipeline stages) would come from Glean's Salesforce connector
- DailyOS adds the intelligence narrative on top
- Without Salesforce in Glean, DailyOS can only work with manually entered vitals (ARR, renewal date) — insufficient for the full report

**Validation gate:** Before scoping this as an issue, confirm:
1. Can Salesforce be authenticated in Glean for this workspace?
2. What fields does Glean surface from Salesforce? (ARR, product line, pipeline stage, opportunity amount, close date, attrition flag)
3. Can Glean return structured data (not just search snippets) for aggregation?
4. Is there a Glean API for SOQL queries, or only the conversational UI?

## New Capabilities Required

If this becomes a report type (`ReportType::CommercialUpdate` or `WeeklyBookUpdate`):

1. **Weekly commercial state snapshots** — DailyOS would need to capture a point-in-time snapshot of account commercial data each week to compute WoW deltas. This doesn't exist today.
2. **Product line segmentation** — accounts need a product line or business unit attribute. This could be a tag, a custom field, or inferred from Glean/Salesforce data.
3. **Pipeline stage model** — expansion pipeline stages (Closed/Commit/Probable/Best) need to be represented. This is CRM-native data; DailyOS would need to pull and store it.
4. **Budget targets** — quarterly budget by product line. Could be a user-entered configuration or pulled from Salesforce.
5. **Attrition status model** — "At Risk" vs "Confirmed" attrition status per account. Today, DailyOS has health scores and renewal dates but not explicit attrition categorization.

## Relationship to Existing Issues

- **I491 (Portfolio Health Summary)** — related but different. I491 is narrative health synthesis; this is financially structured with WoW deltas. Could be a variant of I491 or a separate report type.
- **I499 (Health scoring)** — health scores feed the "why" narrative but don't provide the financial scaffolding.
- **I508 (Intelligence schema)** — Commercial Context dimension includes renewal terms, expansion signals, budget cycle. This is the intelligence layer that enriches the financial report.
- **I509 (Transcript dynamics)** — interaction dynamics explain WHY surprises happen (champion sentiment shifted, decision-maker disengaged).
- **Portfolio architecture (hybrid)** — the lead/VP version of this report needs the shared intelligence layer (v1.2.0+).

## Recommendation

**Park as a future report type.** The financial scaffolding is gated on Glean/Salesforce authentication — without it, the report can't be produced at the fidelity shown in the example. Once the Glean validation gate is cleared:

1. If Salesforce data is available via Glean: scope as a v1.2.0 report type that combines CRM aggregates with DailyOS intelligence narrative
2. If Salesforce data is not available: scope as a lighter "Weekly Intelligence Update" that tracks health movements, saves, surprises using DailyOS-native data only (less financial precision, more relationship intelligence)

Either way, the drill-down capability (narrative intelligence behind each account movement) should ship as part of v1.1.0's report suite — it's the value add regardless of whether the financial roll-up exists.
