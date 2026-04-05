# I554 — Transcript Extraction Signal Fidelity: CS-Grounded Prompt Definitions

**Version:** v1.0.0 Phase 4
**Depends on:** None (prompt-only changes to existing pipeline)
**Type:** Enhancement — transcript intelligence quality
**Scope:** Backend prompt templates only. No schema changes, no frontend changes.

---

## Problem

The transcript extraction prompt (`processor/transcript.rs`) gives the LLM one-line instructions for its most important extraction targets:

```
WINS: - <customer win, positive outcome, expansion signal>
RISKS: - <churn signal, concern, blocker>
```

This produces generic, untyped output. A customer saying "things are going fine" gets extracted as a win alongside "we cut reporting from 5 days to same-day, saving $30K/month." A vague concern gets the same treatment as a champion departure announcement. The LLM has no framework for distinguishing signal from noise, no urgency tiers, no evidence thresholds, and no sub-categorization.

Additionally:
- **Value Delivered** is only extracted at entity intelligence level, not from transcripts directly. The entity prompt asks for `{date, statement, source, impact}` but gives no guidance on what qualifies as value vs. usage.
- **Champion Health** is binary (`champion_present: yes|no`, `champion_engaged: yes|no`) instead of the MEDDPICC three-dimensional model (Power, Vested Interest, Active Advocacy).
- **Stakeholder role changes** mentioned in transcripts aren't flagged for extraction.
- **Sentiment markers** miss ownership language ("our tool" vs "your product"), past-tense product references, and data export questions — all proven churn predictors.

This issue enriches the transcript extraction and entity intelligence prompts with CS-research-grounded definitions that produce specific, categorized, actionable intelligence.

---

## Solution

### 1. WINS — Six Sub-Types with Evidence Threshold

Replace the one-line WINS instruction with:

```
WINS:
Extract only verifiable positive outcomes — not vague sentiment. Each win MUST include
a specific, observable event. "Customer seems happy" is NOT a win.

Sub-types (tag each):
- ADOPTION: milestone crossed, feature activated, user activation target met, integration completed
- EXPANSION: interest in additional scope, new department/team, usage ceiling hit, cross-functional mention
- VALUE_REALIZED: customer articulates ROI in their own words, KPI improvement attributed to product, results shared with leadership
- RELATIONSHIP: executive sponsor actively engaged, new champion identified, reference/case study agreement, advisory board join
- COMMERCIAL: renewal confirmed (especially early), upsell/cross-sell, multi-year commitment, budget increase
- ADVOCACY: public endorsement, referral, conference speaking, internal win-sharing to leadership

Format: - [SUB_TYPE] <specific win with evidence> #"verbatim quote if available"
END_WINS
```

### 2. RISKS — Red/Yellow/Green Urgency Tiers

Replace the one-line RISKS instruction with:

```
RISKS:
Categorize each risk by urgency. Be specific — name the person, the competitor, the timeline.

RED (critical — requires immediate action):
- Champion departure or executive sponsor disengagement
- Active competitor evaluation or piloting
- Severe usage collapse (<50% utilization mentioned)
- Active escalation (unresolved critical issue)
- Budget elimination or review
- Explicit renewal doubt

YELLOW (moderate — needs a recovery plan):
- Usage decline mentioned but not severe
- Champion role change (internal move)
- Delayed implementation or milestone pushback
- Organizational restructuring affecting ownership
- Repeated feature complaints without resolution
- Reduced meeting attendance by key stakeholders

GREEN_WATCH (early warning — monitor):
- Vague dissatisfaction without specific cause
- New leadership reviewing vendor relationships
- Industry/company headwinds (layoffs, funding concerns)
- Reduced energy or engagement without stated reason

Format: - [RED|YELLOW|GREEN_WATCH] <specific risk with named people/timelines> #"verbatim quote"
END_RISKS
```

### 3. DECISIONS — Add Owner and Commitment Type

```
DECISIONS:
- [CUSTOMER_COMMITMENT|INTERNAL_DECISION|JOINT_AGREEMENT] <decision> @owner #"verbatim quote"
END_DECISIONS
```

### 4. SENTIMENT — Expanded Markers

Add to the existing SENTIMENT block:

```
- ownership_language: customer|vendor|mixed  (does the customer say "our tool" or "your product"?)
- past_tense_references: yes|no  (does the customer refer to using the product in past tense?)
- data_export_interest: yes|no  (did the customer ask about data export, portability, or switching?)
- internal_advocacy_visible: yes|no  (did the customer mention sharing results internally?)
- roadmap_interest: yes|no  (did the customer ask about future features or roadmap?)
```

### 5. Champion Health — Three-Level Assessment

Replace `champion_present`/`champion_engaged` binary fields with:

```
CHAMPION_HEALTH:
- champion_name: <name or "unidentified">
- champion_status: strong|weak|lost|none
  strong = has power/influence + personally invested + actively advocates internally
  weak = present and helpful but lacks influence, personal stake unclear, or not advocating
  lost = champion departed, moved roles, or disengaged
  none = no identifiable champion in the meeting
- champion_evidence: <specific behavioral evidence from the call>
- champion_risk: <if weak/lost, what is the risk and recommended action>
END_CHAMPION_HEALTH
```

### 6. Stakeholder Role Changes

Add a new extraction section:

```
ROLE_CHANGES:
- <person name>: <old role/status> -> <new role/status> #"evidence quote"
END_ROLE_CHANGES
```

This captures: departures, promotions, lateral moves, new hires, org restructuring mentions.

### 7. Entity Intelligence Prompt — Value Delivered Guidance

In `intelligence/prompts.rs`, add guidance to the `valueDelivered` output field:

```
"valueDelivered": [
  // ONLY include when the customer articulates a measurable business outcome.
  // Must be: quantified (includes a number), attributed (customer connects to your product),
  // and business-relevant (ties to revenue, cost, risk, or speed).
  // BAD: "The product is useful" / "They use it daily" / "Team likes it"
  // GOOD: "Reduced troubleshooting time by 65%, saving ~$30K/month"
  // GOOD: "Onboarded 500 users in 2 weeks vs previous 6 weeks"
  { "date": "YYYY-MM-DD", "statement": "quantified outcome", "source": "meeting|email|capture", "impact": "revenue|cost|risk|speed" }
]
```

### 8. Entity Intelligence Prompt — Expansion Signals Guidance

Add to `expansionSignals` field guidance:

```
"expansionSignals": [
  // Cross-departmental interest, usage ceiling hits, proactive internal advocacy,
  // organizational growth (hiring, acquisitions), questions about roadmap/pricing,
  // budget increase mentions. Each must cite specific evidence.
  { "signal": "...", "source": "...", "strength": "strong|moderate|early" }
]
```

### 9. COMMITMENTS Extraction Block (absorbed from I551)

Add a new `COMMITMENTS` section to the transcript prompt, alongside ACTIONS/WINS/RISKS/DECISIONS:

```
COMMITMENTS:
  Mutual agreements, stated goals, success criteria, or outcome targets discussed.
  Focus on strategic commitments (not individual action items).
  Examples: "Achieve 50% adoption across 3 teams by Q3", "Deliver ROI report before
  renewal", "Resolve integration blockers before go-live".
  - <commitment> by: YYYY-MM-DD owned_by: us|them|joint #"success criteria"
END_COMMITMENTS
```

Also add to the file enrichment prompt (`processor/enrich.rs`) for inbox files detected as transcripts.

### 10. Entity Intelligence Prompt — Success Plan Signals (absorbed from I551)

Add a `successPlanSignals` section to the intelligence enrichment JSON schema:

```json
"successPlanSignals": {
  "statedObjectives": [
    { "objective": "...", "source": "...", "owner": "...", "targetDate": "...", "confidence": "high|medium|low" }
  ],
  "mutualSuccessCriteria": [
    { "criterion": "...", "ownedBy": "us|them|joint", "status": "not_started|in_progress|achieved|at_risk" }
  ],
  "milestoneCandidates": [
    { "milestone": "...", "expectedBy": "...", "detectedFrom": "...", "autoDetectEvent": "lifecycle event type or null" }
  ]
}
```

With prompt instruction:
```
successPlanSignals — Synthesize the strategic objectives for this account from aggregate
context. What is this customer trying to achieve with us? What have we mutually committed
to? Focus on explicitly stated goals ("our goal is...", "success looks like..."), mutual
commitments beyond individual action items, measurable criteria, and key milestones.
Confidence: "high" = explicitly stated, "medium" = inferred from multiple signals,
"low" = extrapolated from limited data. Do NOT fabricate objectives — return empty arrays
if no stated goals exist.
```

Storage: `success_plan_signals_json TEXT` column on `entity_assessment` (migration in I555).

---

## Files

| File | Changes |
|------|---------|
| `src-tauri/src/processor/transcript.rs` | Replace WINS/RISKS/DECISIONS one-liners with structured sub-type instructions. Expand SENTIMENT. Add CHAMPION_HEALTH and ROLE_CHANGES sections. Update parser to handle new format. |
| `src-tauri/src/intelligence/prompts.rs` | Add value delivered guidance, expansion signal guidance, champion health criteria, and `successPlanSignals` schema to entity intelligence output instructions. |
| `src-tauri/src/intelligence/io.rs` | Add `SuccessPlanSignals`, `StatedObjective`, `MutualSuccessCriterion`, `MilestoneCandidate` types for the new intelligence output field. |
| `src-tauri/src/processor/transcript.rs` (parser) | Update `parse_transcript_output()` to extract sub-types, urgency levels, verbatim quotes, champion health struct, role changes, and commitments from the new format. |
| `src-tauri/src/processor/enrich.rs` | Add COMMITMENTS block to file enrichment prompt for transcript-detected files. |
| `src-tauri/src/processor/transcript.rs` (Glean) | When Glean connected, query for prior Gong call context before building transcript prompt. Inject as "PRIOR CALL HISTORY" block. Graceful degradation on failure/timeout. |

### 11. Glean Gong Pre-Context for Transcript Processing (when Glean connected)

When processing a transcript for a meeting linked to an entity, and Glean is connected, query Glean for prior Gong call context before building the transcript prompt:

```rust
// In process_transcript(), before build_transcript_prompt():
let gong_pre_context = if let Some(glean) = &state.glean_intelligence_provider {
    glean.chat(&format!(
        "Find recent Gong call recordings for the account {}. \
         For each call in the last 90 days, return: title, date, \
         key topics discussed, sentiment, and any commitments made. \
         Return as JSON array.",
        entity_name
    )).await.ok()
} else {
    None
};
```

Inject into the transcript prompt as a pre-context block:

```
PRIOR CALL HISTORY (from Gong):
{gong_pre_context or "No prior call recordings available"}

Use this context to:
- Identify follow-up items from prior calls that were discussed in this meeting
- Detect sentiment trajectory (improving/declining across calls)
- Flag commitments from prior calls that were or weren't addressed
- Note if the champion's engagement pattern is changing across calls
```

This gives the transcript extraction pipeline cross-call continuity that local-only processing can't provide. The LLM can detect "they committed to X on the last call but didn't mention it this time" — a powerful risk signal.

**Graceful degradation:** If Glean is not connected or the query fails, the prompt proceeds without the pre-context block. No error, no blocking.

---

## Parser Compatibility

The new format is backward-compatible with the existing parser's section-delimited approach (START/END markers). New sub-type tags (`[ADOPTION]`, `[RED]`, etc.) are prefix markers within existing sections. The parser should:

1. Strip the `[TAG]` prefix and store it as metadata alongside the content
2. Extract `#"..."` suffixes as verbatim quotes
3. Fall back gracefully if the LLM omits tags (treat as uncategorized)

Champion health and role changes are new sections with their own START/END markers, following the existing pattern.

---

## Out of Scope

- Schema changes to `captures` table (that's I555)
- Frontend rendering changes (I557, I558)
- Report pipeline changes (I556)
- Interaction dynamics persistence (I555)

---

## Acceptance Criteria

1. Transcript extraction prompt includes 6 win sub-types with evidence threshold guidance. "Customer seems happy" no longer extracted as a win.
2. Transcript extraction prompt includes Red/Yellow/Green risk urgency tiers with specific risk type examples.
3. Each extracted win includes a sub-type tag (`ADOPTION`, `EXPANSION`, `VALUE_REALIZED`, `RELATIONSHIP`, `COMMERCIAL`, `ADVOCACY`).
4. Each extracted risk includes an urgency tag (`RED`, `YELLOW`, `GREEN_WATCH`).
5. Verbatim quotes extracted when present (via `#"..."` suffix pattern).
6. CHAMPION_HEALTH section extracted with `champion_name`, `champion_status` (strong/weak/lost/none), `champion_evidence`, `champion_risk`.
7. ROLE_CHANGES section extracted when stakeholder movements are mentioned.
8. SENTIMENT block includes `ownership_language`, `past_tense_references`, `data_export_interest`, `internal_advocacy_visible`, `roadmap_interest`.
9. Entity intelligence prompt `valueDelivered` field includes quantification guidance — rejects vague usage statements.
10. Entity intelligence prompt `expansionSignals` field includes strength classification.
11. COMMITMENTS block extracted from transcript with `owned_by` (us/them/joint), `success_criteria`, and `target_date` when present.
12. Entity intelligence enrichment produces `successPlanSignals` JSON with `statedObjectives`, `mutualSuccessCriteria`, and `milestoneCandidates`. Stored in `entity_assessment.success_plan_signals_json`.
13. `SuccessPlanSignals` types defined in `intelligence/io.rs` matching the prompt schema.
14. Parser handles new format without breaking on transcripts that don't include all new sections (graceful fallback).
15. Existing transcript re-processing (manual re-attach) works with the new prompt format.
16. Run 3+ real transcripts through the updated pipeline. Verify extracted wins are specific and categorized, risks have urgency tiers, and champion health reflects meeting content.

### Glean Gong Pre-Context
17. When Glean is connected, transcript processing for a meeting linked to an entity queries Glean for prior Gong call data. The transcript prompt includes a "PRIOR CALL HISTORY" context block with call titles, dates, topics, and commitments.
18. Prior call context influences extraction: if a commitment from a prior Gong call is not mentioned in the current transcript, it appears as a risk or open item. Verify with a real transcript for an account with Gong call history.
19. When Glean is NOT connected, transcript processing proceeds without Gong pre-context — no error, no degradation, identical behavior to pre-Phase 5.
20. Glean Gong query timeout does not block transcript processing. If the query takes >10s, proceed without the pre-context.
