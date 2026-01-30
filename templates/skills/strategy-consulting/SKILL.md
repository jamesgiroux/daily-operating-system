---
name: strategy-consulting
description: McKinsey-style strategic analysis using multi-agent consulting workflow. Applies SCQA framing, Issue Trees, MECE decomposition, and Pyramid Principle to produce executive-ready recommendations.
allowed-tools: Read, Write, Edit, Glob, Grep, WebSearch, WebFetch, Task
---

# Strategy Consulting Skill

McKinsey-style strategic analysis using a multi-agent consulting workflow.

## Overview

This skill applies professional consulting methodologies to strategic questions, producing executive-ready analysis and recommendations.

## Philosophy

**Structure before content** - Frame the problem correctly before diving into analysis.

**Hypothesis-driven** - Start with a point of view, then validate or refute with evidence.

**So what?** - Every insight must answer "why does this matter?"

**Pyramid Principle** - Lead with conclusions, support with evidence.

## When to Use

Invoke this skill when you need to:
- Analyze a strategic question systematically
- Build a business case for a decision
- Evaluate multiple options with tradeoffs
- Produce executive-ready recommendations
- Structure complex problems into manageable pieces

## Quick Start

```
"Use strategy consulting to analyze [question]"
```

or

```
"/strategy-consulting"
```

## The Seven-Step Workflow

### Step 1: Problem Definition (Engagement Manager)

**Frame the question using SCQA:**

- **S**ituation: What's the current state?
- **C**omplication: What changed or is problematic?
- **Q**uestion: What specifically are we trying to answer?
- **A**nswer: What's our Day 1 hypothesis?

**Output:**
```markdown
## Problem Definition

**Situation**: [Current state]

**Complication**: [What changed/problem]

**Question**: [Specific question to answer]

**Day 1 Hypothesis**: [Initial point of view to test]
```

### Step 2: Scope and Boundaries

Define what's in and out of scope:
- What decisions does this analysis inform?
- What decisions are explicitly out of scope?
- What constraints exist?
- Who are the stakeholders?

### Step 3: Issue Tree (Framework Strategist)

Build MECE decomposition of the question:

```markdown
## Issue Tree

**Core Question**: [From Step 1]

### Branch 1: [First MECE branch]
- Sub-question 1.1
- Sub-question 1.2
- Sub-question 1.3

### Branch 2: [Second MECE branch]
- Sub-question 2.1
- Sub-question 2.2

### Branch 3: [Third MECE branch]
- Sub-question 3.1
- Sub-question 3.2
```

**MECE = Mutually Exclusive, Collectively Exhaustive**
- No overlap between branches
- All possibilities covered

### Step 4: Quality Check (Partner Critic)

Red-team the framing:
- Is the question the right question?
- Is the hypothesis falsifiable?
- Are the branches truly MECE?
- What's missing?

### Step 5: Evidence Gathering (Analyst)

For each branch of the issue tree:
- Gather relevant data
- Apply Fermi estimation where data is missing
- Assess source credibility
- Identify the vital few factors (80/20)

**Evidence standards:**
- Explicit confidence levels
- Source citations
- Assumptions stated
- Ranges, not false precision

### Step 6: Quality Check 2 (Partner Critic)

Validate the analysis:
- Is the evidence sufficient?
- Are conclusions supported?
- What are the key risks?
- What would change our mind?

### Step 7: Output Generation (Executive Storyteller)

Produce final deliverables using Pyramid Principle:

**Structure:**
1. Lead with the answer
2. Support with key arguments
3. Back up with evidence
4. Address alternatives considered

## Output Formats

### P2 Memo (Default)

```markdown
# [Title]: [Action-Oriented Headline]

## The Bottom Line

[2-3 sentences: Answer + So What + Recommended Action]

## Key Arguments

### 1. [Argument with action title]

[Evidence and reasoning]

### 2. [Argument with action title]

[Evidence and reasoning]

### 3. [Argument with action title]

[Evidence and reasoning]

## Alternatives Considered

| Option | Pros | Cons | Verdict |
|--------|------|------|---------|
| [Option A] | | | Rejected: [reason] |
| [Option B] | | | Selected |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| | | | |

## Next Steps

1. [Action] - Owner: [Name] - By: [Date]
2. [Action] - Owner: [Name] - By: [Date]
```

### Deck-Ready Content

For presentation slides:

```markdown
# Slide 1: Executive Summary
**Headline**: [Action-oriented conclusion]
**Bullets**:
- Key point 1
- Key point 2
- Key point 3

# Slide 2: [Topic]
**Headline**: [Conclusion for this topic]
**Visual**: [Chart/diagram description]
**Talking points**:
- Point 1
- Point 2
```

### Slack Summary

For quick communication:

```
[One-line answer]

Key points:
• [Point 1]
• [Point 2]
• [Point 3]

Recommendation: [Action]
Link to full analysis: [link]
```

## Agents Used

| Agent | Role | When Invoked |
|-------|------|--------------|
| **engagement-manager** | Problem framing (SCQA + Day 1 Hypothesis) | Step 1 |
| **framework-strategist** | Issue tree construction (MECE) | Step 3 |
| **partner-critic** | Quality control and red team | Steps 4, 6 |
| **analyst-research-logic** | Evidence gathering and validation | Step 5 |
| **executive-storyteller** | Output generation (Pyramid Principle) | Step 7 |

## Mental Models Applied

### WWHTBT (What Would Have to Be True)

For each option, identify assumptions that must hold:
```
For [Option X] to be the right choice:
1. [Assumption 1] would have to be true
2. [Assumption 2] would have to be true
3. [Assumption 3] would have to be true
```

Then prioritize testing the riskiest assumptions.

### Fermi Estimation

When precise data isn't available:
```
Question: How many X?
= [Factor 1] × [Factor 2] × [Factor 3]
= [Estimate 1] × [Estimate 2] × [Estimate 3]
= [Range: Low - Mid - High]

Confidence: [Level] because [reasoning]
```

### 80/20 Analysis

Identify the vital few factors:
```
Of [total factors], [top 20%] drive [80%] of [outcome].
Focus on: [Factor 1], [Factor 2]
Deprioritize: [Other factors]
```

## Example Workflow

**Question**: "Should we expand into the healthcare vertical?"

**Step 1 - SCQA**:
- S: We serve enterprise clients in media and retail
- C: Healthcare vertical showing interest; competitors entering
- Q: Should we prioritize healthcare expansion in 2026?
- A: Yes, but with focused entry through existing client referrals

**Step 3 - Issue Tree**:
1. Is the market attractive?
   - Market size
   - Growth rate
   - Competitive intensity
2. Can we win?
   - Capability fit
   - Reference customers
   - Competitive differentiation
3. Is it worth it?
   - Investment required
   - Timeline to profitability
   - Opportunity cost

**Step 7 - Output**:
P2 memo with recommendation, supporting evidence, and implementation plan.

## Best Practices

1. **Start with the answer** - Have a hypothesis before diving into analysis
2. **Be MECE** - Structure thinking to be complete and non-overlapping
3. **Show your work** - Make reasoning transparent
4. **Quantify when possible** - Numbers beat adjectives
5. **Address alternatives** - Show you considered other options
6. **Name the risks** - Credibility comes from acknowledging uncertainty

## Troubleshooting

**Analysis going in circles:**
- Revisit the problem definition
- Check if the question is answerable
- Narrow the scope

**Not enough evidence:**
- Use Fermi estimation
- State assumptions explicitly
- Acknowledge uncertainty

**Stakeholders disagree:**
- Return to SCQA
- Find the root disagreement
- Build shared understanding of the question first
