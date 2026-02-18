---
description: "Structured analytical thinking for decisions with ambiguity"
---

# /decide

Apply rigorous strategy consulting frameworks to decisions that involve ambiguity. This is the deepest command — it walks through problem framing, decomposition, evidence testing, adversarial challenge, and recommendation. Every phase has quality gates. The output is a P2 Memo: Bottom Line first, then the reasoning.

## Arguments

- `$ARGUMENTS[0]` — The question (required). State the decision or question as specifically as possible.

Examples:
- "Should we invest in a dedicated CSM for Nielsen before their Q3 renewal?"
- "Why is mid-market churn accelerating despite improved NPS?"
- "Which of these three expansion opportunities should we prioritize?"
- "How big is the AI analytics opportunity for our platform?"
- "What could go wrong with the Datadog partnership strategy?"

## Decision Types by Role Preset

| Preset | Common Decision Types |
|---|---|
| Customer Success | Renewal investment, intervention strategy, resource allocation, escalation decisions |
| Sales | Deal prioritization, pricing strategy, competitive response, resource deployment |
| Partnerships | Partner selection, co-investment, integration priority, alliance structure |
| Agency | Scope decisions, pricing, team allocation, client retention investment |
| Consulting | Methodology selection, engagement structure, follow-on strategy |
| Product | Build/buy/partner, feature priority, market entry, deprecation |
| Leadership | Strategic priority, organizational design, investment allocation, talent decisions |
| The Desk | Auto-detect from question context |

## Workflow

### Phase 1: Frame (SCQA)

Frame the question using the SCQA structure. Ground every element in workspace data.

**Situation** — What is the current state? Read from entity dashboards, intelligence, and portfolio context. This is factual baseline, not interpretation.

"Nielsen is a $2.4M ARR account in the Growth lifecycle stage with Green health as of February. They are 47 days from renewal. Sarah Chen is the champion (warming temperature, weekly engagement), David Park (CTO) is disengaged (has not attended in 4 months)."

**Complication** — What changed or is threatening? What makes this a decision point now?

"David's vendor consolidation keynote at DataConf suggests he may be building a case to reduce vendor count. Datadog launched a competing analytics module in January. The renewal conversation starts in March, and we do not have CTO alignment."

**Question** — State the specific, testable question. Refine until it is precise enough to answer with evidence.

Not: "What should we do about Nielsen?"
Yes: "Should we invest $40K in a dedicated technical CSM for Nielsen for the 8 weeks before renewal, specifically to re-engage the CTO and counter the Datadog competitive threat?"

**Answer (Hypothesis)** — State your best guess before analyzing. This is what you will test.

"Yes, the dedicated CSM investment is justified because CTO re-engagement is the critical path to multi-year renewal, and the competitive threat requires technical depth we currently lack in the account."

**Quality gate:** Is the question specific enough that you could definitively answer it with evidence? If "it depends" is the only honest answer, decompose the question into sub-questions that CAN be answered.

### Phase 2: Decompose (Issue Tree)

Break the question into testable sub-questions. The tree must be MECE (Mutually Exclusive, Collectively Exhaustive).

**Auto-select framework from the analytical-frameworks skill:**

| If the question sounds like... | Use this framework |
|---|---|
| "Should we [do X]?" | SCQA + Issue Tree — decompose into conditions for success |
| "Why is [X] happening?" | Diagnostic Issue Tree — decompose into possible causes |
| "Is [X] really true?" | WWHTBT — list conditions that must hold |
| "Which of [A, B, C]?" | 2x2 Matrix — identify key dimensions and plot options |
| "How big is [X]?" | Fermi Estimation — decompose into estimable components |
| "What's the landscape?" | Porter's Five Forces / 3Cs — map competitive dynamics |
| "Where should we focus?" | 80/20 Analysis — rank by impact |
| "What could go wrong?" | Pre-Mortem + Red Team — enumerate failure modes |

**Example issue tree for the Nielsen CSM question:**

```
Should we invest in dedicated CSM for Nielsen?
├── Is CTO re-engagement critical to renewal?
│   ├── Does David have decision authority or veto power?
│   ├── Can we renew without CTO alignment?
│   └── What is the risk of not re-engaging?
├── Is the competitive threat real and imminent?
│   ├── How far along is the Datadog evaluation?
│   ├── Is this a technical evaluation or a strategic comparison?
│   └── Can our platform address the analytics gap?
├── Will a dedicated CSM achieve re-engagement?
│   ├── Has a similar approach worked at other accounts?
│   ├── Does our CSM candidate have the technical depth?
│   └── Is 8 weeks enough time?
└── Is the investment justified by the financial outcome?
    ├── What is the renewal ARR at risk?
    ├── What is the expected ROI of the CSM investment?
    └── What is the opportunity cost (where else could $40K go)?
```

The tree structure drives which workspace files get read. Each branch maps to evidence sources.

**Quality gate:** Is the tree MECE? Check: Does every piece of evidence map to exactly one branch? Are there scenarios that fall outside the tree? If yes, add branches.

### Phase 3: Test (Evidence Gathering + WWHTBT)

For each branch of the issue tree, gather workspace evidence.

**What Would Have to Be True (WWHTBT) for the hypothesis to be correct:**

1. "David has meaningful decision authority over vendor renewals" — Check stakeholders.md, meeting history. **Evidence:** David is listed as Technical Buyer with veto power. Last vendor decision (Snowflake, per October meeting) was his call.
2. "The Datadog evaluation is serious enough to threaten renewal" — Check intelligence.json, recent meeting notes. **Evidence:** Competitive mention in Feb 14 sync was initiated by their side. David's DataConf keynote on vendor sprawl was January 28. Timeline: serious.
3. "A technical CSM can re-engage David within 8 weeks" — Check People/David-Park for preferences, check _archive/ for past engagement success patterns. **Evidence:** David engaged deeply when our platform architect joined the Q3 technical review. He responds to technical depth, not relationship management.
4. "$2.4M ARR justifies $40K investment" — Calculate ROI. **Evidence:** 60:1 ARR to investment ratio. Even protecting 50% renewal probability makes this net positive.

For each condition, assess:
- **Confirmed** by evidence (with specific source)
- **Contradicted** by evidence (with specific source)
- **Unknown** — no evidence available (flag as a gap and a risk)

**Quality gate:** Every claim sourced to a specific file, meeting, signal, or data point. No assertions without evidence. If evidence does not exist, state "no workspace evidence" and assess based on what is known.

### Phase 4: Challenge (Red Team)

This is not a formality. The red team pass must produce genuine counter-arguments.

**Pre-mortem:** Assume the dedicated CSM was deployed and the strategy failed. What went wrong?

"The CSM had technical depth but no relationship with David. David saw the engagement as sales pressure rather than genuine technical partnership. He doubled down on the Datadog evaluation to prove he had alternatives. The $40K was spent, and the renewal still downsized by 30%."

**Red Team — strongest counter-argument:**

"The real risk is not David's disengagement — it is that the product gap in analytics is real and a CSM cannot close it. If the platform genuinely does not compete with Datadog's analytics module, re-engaging David just accelerates his conclusion that we need to be replaced. The money would be better spent on an engineering sprint to close the product gap before renewal."

**Partner-critic frame:** "If I were advising Nielsen's CTO, I would tell him: the dedicated CSM is a retention play, not a value play. Push for a technical proof-of-concept instead. If they can match Datadog's analytics capabilities, great. If not, you have your answer."

**Quality gate:** Does the red team argument give the decision-maker pause? If it does not, the argument is too weak. Try harder. Find the genuinely uncomfortable counter-argument.

### Phase 5: Recommend (P2 Memo)

Structure the final output using the Pyramid Principle — Bottom Line first:

```markdown
# Decision: {restated question}
**Date:** {today}
**Status:** Recommendation

## Bottom Line
{The recommendation in 1-2 sentences. Specific. Actionable. "Do X by Y with Z."}

Deploy a dedicated technical CSM for Nielsen for 8 weeks (Feb 17 - Apr 14), with the specific mandate to re-engage David Park through a technical proof-of-concept on analytics capabilities, not through relationship management. Cost: $40K. Expected outcome: CTO alignment restored before renewal conversation begins in March.

## Key Arguments

### 1. CTO re-engagement is the critical path
{Evidence chain: David has veto power (stakeholders.md), last vendor decision was his (Q3 meeting), he is disengaged (4 months absent). Without his alignment, renewal risk is HIGH regardless of champion strength.}

### 2. The competitive threat requires technical response
{Evidence chain: Datadog evaluation mentioned in Feb 14 sync, David's vendor sprawl keynote, analytics gap is the specific attack vector. A relationship-only response will not counter a technical comparison.}

### 3. The investment math works at any reasonable probability
{Evidence: $2.4M ARR, $40K cost. Even at 30% probability improvement (from 50% to 80% renewal), expected value is $720K.}

## Alternatives Considered

### Alternative A: Do nothing, rely on champion
{Why considered. Why rejected: David's veto power makes champion-only strategy insufficient. Evidence: {source}.}

### Alternative B: Invest in product engineering sprint instead
{The red team argument. Why it's a genuine alternative. Why the CSM approach is still preferred: the CSM can validate whether the product gap is the real issue before committing engineering resources. The approaches are sequential, not mutually exclusive.}

## Risks
| Risk | Probability | Mitigation |
|---|---|---|
| David interprets CSM as sales pressure | Medium | Frame as technical partnership, lead with proof-of-concept |
| Product gap is real and CSM cannot close it | Medium | CSM conducts honest gap assessment in week 2; if gap is real, pivot to engineering response |
| 8 weeks is not enough time | Low | Parallel-track: CSM engagement + early renewal conversation with Sarah |

## Next Steps
1. **Identify technical CSM candidate** — VP CS, by Feb 21
2. **Brief CSM on David's preferences and vendor sprawl concerns** — You, by Feb 24
3. **Schedule technical proof-of-concept meeting with David** — CSM + You, target week of Mar 3
4. **Prepare analytics capability comparison** — CSM, by Mar 7
5. **Checkpoint: assess CTO re-engagement progress** — You, Mar 14
```

**Quality gate:** Is the recommendation specific enough to act on? Not "we should probably consider..." but "do X by date Y with owner Z." Is the red team argument addressed in Alternatives Considered? Are Next Steps trackable?

### Phase 6: Output and Loop-Back

Present the P2 Memo. Then offer:

```
Would you like me to:
1. Save this decision memo to Accounts/Nielsen/decision-csm-investment-2026-02.md
2. Create {N} actions from Next Steps in data/actions.json
3. Update Accounts/Nielsen/intelligence.json with the competitive analysis and CTO re-engagement strategy

Or adjust the analysis first?
```

## Skills That Contribute

- **analytical-frameworks** — Provides the framework selection, SCQA structure, MECE testing, WWHTBT, and red team methodology
- **entity-intelligence** — Auto-fires to load all entity context for evidence gathering
- **relationship-context** — Provides stakeholder profiles for people-dependent decisions
- **political-intelligence** — Enriches when decisions have political dimensions or stakeholder dynamics
- **role-vocabulary** — Shapes decision types and vocabulary
- **action-awareness** — Provides action history and receives new actions from recommendations
- **loop-back** — Handles saving the memo and creating trackable next steps
