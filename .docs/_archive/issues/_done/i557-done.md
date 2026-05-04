# I557 — Surface Hidden Intelligence on Account Detail Page

**Version:** v1.0.0 Phase 4
**Depends on:** I550 (account detail editorial redesign — provides the visual framework), I555 (interaction dynamics persistence — provides champion/dynamics data)
**Type:** Enhancement — frontend intelligence surfacing
**Scope:** Frontend only. All data already exists in `entity_assessment` — this issue renders it.

---

## Problem

The entity intelligence pipeline computes ~15 structured intelligence fields that are stored in `entity_assessment` but never rendered on any page. Users get a massive LLM synthesis producing rich, specific intelligence — and then can't see it.

### Fields computed but never surfaced

| Field | Schema Location | What It Contains | Why It Matters |
|-------|----------------|-----------------|----------------|
| `valueDelivered[]` | `entity_assessment.value_delivered` | Quantified business outcomes with dates and sources | The proof that justifies renewal. CSMs need this for QBRs and leadership asks. |
| `successMetrics[]` | `entity_assessment.success_metrics` | KPI targets, current values, status, owners | Shows whether the customer is hitting their goals. Core to value realization. |
| `openCommitments[]` | `entity_assessment.open_commitments` | Promises made with owners, due dates, sources, status | Tracks what's been promised. Broken commitments are a top churn driver. |
| `relationshipDepth` | `entity_assessment.relationship_depth` | Champion strength, executive access, stakeholder coverage, coverage gaps | Strategic view of relationship quality. Coverage gaps are actionable. |
| `competitiveContext[]` | `entity_assessment.dimensions_json` | Competitors mentioned, context, positioning | Know your competitive landscape per account. |
| `strategicPriorities[]` | `entity_assessment.dimensions_json` | Customer's stated strategic goals and how you align | Understanding their priorities lets you frame value in their language. |
| `coverageAssessment` | `entity_assessment.dimensions_json` | Stakeholder coverage quality, recommendations | Direct input to stakeholder strategy. |
| `organizationalChanges[]` | `entity_assessment.dimensions_json` | Reorgs, leadership changes, hiring/layoffs | Context that affects everything — renewal, expansion, risk. |
| `expansionSignals[]` | `entity_assessment.dimensions_json` | Growth opportunities with strength classification | Revenue intelligence. |
| `renewalOutlook` | `entity_assessment.dimensions_json` | AI assessment of renewal probability and factors | The most important forward-looking signal for CS. |
| `contractContext` | `entity_assessment.dimensions_json` | Contract details, terms, special conditions | Important context for renewal and expansion conversations. |
| `meetingCadence` | `entity_assessment.dimensions_json` | Engagement rhythm assessment | Health signal. |
| `emailResponsiveness` | `entity_assessment.dimensions_json` | Response patterns and engagement quality | Health signal. |
| `blockers[]` | `entity_assessment.dimensions_json` | Active impediments to success | Must-address items. |

Additionally, after I555 lands:
- Champion health history (from `meeting_champion_health`)
- Interaction dynamics trends (from `meeting_interaction_dynamics`)

---

## Solution

Surface these fields in the Account Detail page within the existing editorial chapter structure (I550's margin label layout). Three new chapters + enrichment of two existing chapters:

### New Chapter: "Value & Commitments" (between Watch List and The Work)

This chapter surfaces proof of value delivery and tracks commitments. It belongs after the watch list (risks/wins) and before upcoming work — it's the "what has been accomplished and promised" bridge.

**Sections:**

**Value Delivered** — editorial table
| Column | Source |
|--------|--------|
| Date | `valueDelivered[].date` |
| Outcome | `valueDelivered[].statement` |
| Impact type | `valueDelivered[].impact` badge (revenue/cost/risk/speed) |
| Source | `valueDelivered[].source` |

Empty state: "No verified outcomes yet. Value items appear when customers articulate measurable results in meetings."

**Success Metrics** — compact KPI cards in a 2-column grid
| Element | Source |
|---------|--------|
| Metric name | `successMetrics[].name` |
| Target | `successMetrics[].target` |
| Current | `successMetrics[].current` |
| Status badge | `successMetrics[].status` (on-track/at-risk/behind/achieved) |
| Owner | `successMetrics[].owner` |

**Open Commitments** — timeline-style list
| Element | Source |
|---------|--------|
| Description | `openCommitments[].description` |
| Owner | `openCommitments[].owner` |
| Due date | `openCommitments[].dueDate` with overdue highlighting |
| Source | `openCommitments[].source` |
| Status | `openCommitments[].status` (open/delivered/at-risk) |

### New Chapter: "Competitive & Strategic Landscape" (after Relationship Health)

This chapter surfaces the competitive and strategic intelligence that shapes account strategy.

**Sections:**

**Strategic Priorities** — the customer's stated goals
- Rendered as serif text items with alignment context (how your product connects to their priority)
- Source: `strategicPriorities[]`

**Competitive Context** — competitors in play
- Competitor name + context paragraph
- Source: `competitiveContext[]`
- Only shown when non-empty

**Organizational Changes** — reorgs, leadership moves
- Timeline items: what changed, when, impact assessment
- Source: `organizationalChanges[]`
- Only shown when non-empty

**Blockers** — active impediments
- Terracotta-accented items (same visual language as risks)
- Source: `blockers[]`

### New Chapter: "Outlook" (before Reports chapter, after The Work)

Forward-looking synthesis of expansion potential, renewal readiness, and contract context.

**Sections:**

**Renewal Outlook** — AI assessment of renewal
- Narrative text with key factors
- Only shown for accounts with renewal dates
- Source: `renewalOutlook`

**Expansion Signals** — growth opportunities
- Signal text + strength badge (strong/moderate/early)
- Source: `expansionSignals[]`

**Contract Context** — terms and conditions context
- Compact text block
- Only shown when non-empty
- Source: `contractContext`

### Enrich existing chapter: "The Room" (Stakeholder Gallery)

Add to the existing stakeholder section:

**Relationship Depth Summary** (below stakeholder cards)
- Champion strength indicator: strong/adequate/weak/none — from `relationshipDepth.championStrength`
- Executive access: yes/limited/none — from `relationshipDepth.executiveAccess`
- Coverage quality: text — from `relationshipDepth.stakeholderCoverage`
- Coverage gaps: bulleted list — from `relationshipDepth.coverageGaps[]`

**Coverage Assessment** (below relationship depth)
- Narrative assessment from `coverageAssessment`
- Recommendations for improving coverage

### Enrich existing chapter: "Relationship Health"

Add engagement cadence context below the 6 dimension bars:

**Engagement Cadence** (below dimension bars)
- Meeting cadence narrative: `meetingCadence`
- Email responsiveness narrative: `emailResponsiveness`
- These provide prose context for the numeric dimension scores above them

---

## TypeScript Types

All types already exist in `src/types/index.ts` within `EntityIntelligence`. No new type definitions needed — the fields are already typed but the components don't read them. Specifically:

- `intelligence.valueDelivered: ValueItem[]`
- `intelligence.successMetrics: SuccessMetric[]`
- `intelligence.openCommitments: OpenCommitment[]`
- `intelligence.relationshipDepth: RelationshipDepth`
- `intelligence.dimensions.competitiveContext: CompetitiveInsight[]`
- `intelligence.dimensions.strategicPriorities: StrategicPriority[]`
- `intelligence.dimensions.coverageAssessment: CoverageAssessment`
- `intelligence.dimensions.organizationalChanges: OrgChange[]`
- `intelligence.dimensions.expansionSignals: ExpansionSignal[]`
- `intelligence.dimensions.renewalOutlook: RenewalOutlook`
- `intelligence.dimensions.contractContext: ContractContext`
- `intelligence.dimensions.meetingCadence: CadenceAssessment`
- `intelligence.dimensions.emailResponsiveness: ResponsivenessAssessment`
- `intelligence.dimensions.blockers: Blocker[]`

If any of these types don't exist yet in the frontend, they need to be added to match the Rust `IntelligenceJson` output schema.

---

## Files

| File | Changes |
|------|---------|
| `src/pages/AccountDetailEditorial.tsx` | Add 3 new chapter sections (Value & Commitments, Competitive & Strategic, Outlook). Enrich Stakeholder Gallery and Relationship Health chapters. |
| `src/pages/AccountDetailEditorial.module.css` | Styles for new sections — editorial table, KPI cards, timeline items. Follow I550 margin label patterns. |
| `src/components/entity/ValueCommitments.tsx` | New component: value delivered table + success metrics cards + commitments timeline |
| `src/components/entity/ValueCommitments.module.css` | Styles |
| `src/components/entity/StrategicLandscape.tsx` | New component: strategic priorities + competitive context + org changes + blockers |
| `src/components/entity/StrategicLandscape.module.css` | Styles |
| `src/components/entity/AccountOutlook.tsx` | New component: renewal outlook + expansion signals + contract context |
| `src/components/entity/AccountOutlook.module.css` | Styles |
| `src/components/entity/StakeholderGallery.tsx` | Add relationship depth summary + coverage assessment below cards |
| `src/types/index.ts` | Verify/add any missing dimension sub-types from `IntelligenceJson` |

---

## Design Principles

- **Graceful degradation**: Every new section collapses entirely when its data is empty. No "No data available" placeholders cluttering the page.
- **Editorial treatment**: Serif typography for narrative content. Sans for metadata. Mono for dates and labels. Section rules between chapters. All per ADR-0073/0077.
- **Inline editing**: All AI-generated text fields are editable via `EditableText` (per I529 pattern). Edits emit `user_correction` signals.
- **Progressive reveal**: New chapters participate in I550's scroll-driven reveal animations.
- **Information hierarchy**: Value & Commitments is the most important new chapter (proof of value for renewals). Competitive & Strategic is context. Outlook is forward-looking synthesis. Order reflects importance.

---

## Out of Scope

- Backend changes or new intelligence fields (data already exists)
- Person detail or project detail pages (can follow later using same components)
- Meeting detail page (that's I558)
- Report rendering changes

---

## Acceptance Criteria

1. Account with `valueDelivered` data shows a "Value & Commitments" chapter with value table, success metric cards, and commitment timeline.
2. Account with `competitiveContext` data shows a "Competitive & Strategic Landscape" chapter.
3. Account with `renewalOutlook` or `expansionSignals` shows an "Outlook" chapter.
4. All new chapters collapse entirely when their intelligence data is empty — no empty section headings.
5. Stakeholder Gallery shows relationship depth summary (champion strength, executive access, coverage quality, coverage gaps) below stakeholder cards when `relationshipDepth` is present.
6. Relationship Health chapter shows engagement cadence context (meeting cadence and email responsiveness narratives) below dimension bars.
7. New chapters use I550 margin label layout (110px gutter + sticky label) when that redesign has landed. Falls back to standard chapter layout otherwise.
8. All AI-generated text fields are inline-editable. Edits persist and emit signals.
9. Success metric cards show on-track/at-risk/behind/achieved status with appropriate color coding (sage/turmeric/terracotta/larkspur).
10. Open commitments with past due dates show overdue highlighting (terracotta).
11. Expansion signals show strength badges (strong/moderate/early).
12. Zero inline styles. All CSS in module files.
13. Zero ADR-0083 vocabulary violations.
14. Page load performance is not degraded — new sections render from existing `EntityIntelligence` payload (no additional API calls).
15. Responsive: new sections work correctly on narrow viewports (single column collapse).
16. After applying `full` mock scenario (with I555 mock data), all new chapters render with realistic data — value delivered table, success metrics, open commitments, competitive context, expansion signals, renewal outlook. No empty chapters for fully-seeded accounts.
