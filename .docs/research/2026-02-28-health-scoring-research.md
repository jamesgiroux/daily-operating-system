# Health Scoring Research: Industry Analysis + DailyOS Architecture

**Date:** 2026-02-28
**Status:** Complete — feeds into ADR-0097
**Context:** Researched as part of v1.1.0 planning. Original I484 ("remove sparsity gate") was too narrow — expanded into a full health scoring architecture after validating actual system state and industry best practice.

---

## Methodology

1. Audited DailyOS codebase: traced every health-related field, signal, prompt, and UI surface
2. Queried Glean for VIP's actual org health model — validated signal inputs with JHI as test account
3. Researched 6 CS platforms (Gainsight, Hook, Vitally, Totango, ChurnZero, Planhat) for health scoring patterns
4. Synthesized universal dimensions, identified gaps, designed architecture

## Key Findings

### DailyOS Health Scoring State (Pre-ADR-0097)

- **Two disconnected health systems:** `accounts.health` (user-set RAG, green/yellow/red) and `entity_intelligence.health_score` (LLM-assessed 0-100). Never merged.
- **health_score is invisible:** Computed and stored but never rendered to users. Only feeds Account Health Review report narrative.
- **~80% null rate:** Sparsity gate in prompt tells LLM to return null for accounts with < 3 signals and < 2 meetings. Inconsistent thresholds.
- **No rubric:** The LLM invents the meaning of health_score on each enrichment. No defined dimensions, weights, or lifecycle awareness.
- **No transcript sentiment:** Email enrichment extracts structured sentiment. Transcript processing does not.
- **No Glean health parsing:** Org health data flows through Glean as free text, never parsed structurally.

### VIP's Org Health Model (Validated via Glean)

JHI (Janus Henderson Investors) as test account. Health: Green (score 3/3).

**Signal dimensions in the org model:**
1. **Commercial/Renewal:** ACV ~$397k, renewal likelihood Green, renewal 2026-11-28, contract-based pricing
2. **ICP/Fit:** Good Fit (WPVIP Account ICP), Excellent Fit (Govt profile), Enterprise segment
3. **Product Adoption:** Customer stage: Adoption, Enhanced CMS, multiple environments + add-ons (APM, Geo Redundancy, Hourly Backups, etc.)
4. **Support Experience:** Enhanced support tier, 12hr/30min SLAs, 99.99% uptime SLA, steady ticket volume with some highly_negative sentiment but SLA adherence maintained
5. **Relationship/Coverage:** TAM assigned (James Giroux), RM + expansion AEs, Tier 1 – Expansion growth tier

**Key insight:** The org's health model covers dimensions DailyOS will never replicate natively (product usage, support SLAs, ICP fit). DailyOS should consume this as baseline, not compete with it.

### Gong Data via Glean

Validated: Gong provides via Glean:
- Full call transcripts
- Structured metadata (account, opportunity, participants, timestamps, owner)
- Stakeholder presence patterns

NOT available via Glean: talk ratios, conversation trackers, deal health scores, engagement scoring. These remain in Gong's native UI.

**Future direction:** DailyOS could independently compute meeting-level analytics from transcripts it already processes (talk time ratios, question density, forward-looking language, escalation language). This is architecturally enabled by ADR-0097's transcript sentiment extraction but scoped as post-v1.1.0. See ADR-0097 "Future Direction: Meeting Analytics" section.

### Industry Health Scoring Consensus

**Universal dimensions (5-6 of 6 platforms):**
- Product usage (frequency, depth, breadth) — 70-80% of churn prediction per Hook
- Support health (tickets, SLA, severity)
- Engagement cadence (meetings, emails, response time)
- CSM/human assessment (manual RAG)
- NPS/survey data
- Communication sentiment (email/call tone) — becoming table-stakes

**Sparse data handling best practices:**
- Vitally: null-exclusion with proportional weight redistribution
- Planhat: additive-from-neutral (start at 5/10, signals move up/down)
- Both adopted in ADR-0097

**Manual override consensus:** Every platform except Hook includes CSM override as ONE dimension (10-20% weight), not a trump card.

**Lifecycle-aware weighting:** 5/6 platforms support different score configurations per lifecycle stage.

### DailyOS's Unique Advantage

DailyOS has **relationship intelligence** that exceeds all 6 platforms:
- Deep stakeholder coverage with champion strength, executive access, coverage gaps
- Meeting-level intelligence (pre-briefs, transcript analysis)
- Cross-entity signal propagation with Bayesian learning
- Personal context shaping interpretation

The relationship signals are **leading indicators** — champion disengagement shows up in meeting cadence weeks before it affects product usage or support volume.

## Architecture Decision

See ADR-0097: `.docs/decisions/0097-account-health-scoring-architecture.md`

**Summary:** One score, two layers (baseline from org/computed + relationship context from DailyOS). Divergence detection when relationship signals contradict baseline. LLM explains the numbers, doesn't pick them.

## Implementation

I484 superseded by 5 new issues: I499-I503. See `.docs/plans/v1.1.0.md`.

## Sources

### Industry Research
- Gainsight Scorecards, Staircase AI Health Score, DEAR Framework
- Hook Products (Echo agent), Build vs Buy health scoring guide
- Vitally Health Scores documentation, Health Framework, Lifecycle Stage guide
- Totango Multidimensional Health Configuration
- ChurnZero Health Score Dashboard, 4-Step Strategy, Handbook
- Planhat Health Scores (What You Can Include, How Health Is Calculated, Set Up Profiles)

### Internal
- DailyOS codebase audit (intelligence/io.rs, prompts.rs, intel_queue.rs, signals/, db/)
- Glean queries against VIP instance (JHI test account)
- `.docs/research/2026-02-28-hook-gap-analysis.md`
