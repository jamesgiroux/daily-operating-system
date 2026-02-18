---
name: analytical-frameworks
description: "Strategy consulting frameworks for structured analysis — 8 frameworks with selection guide and quality gates"
---

# Analytical Frameworks

This skill fires when a question implies structured analysis is needed. It provides 8 strategy consulting frameworks, a selection guide for choosing the right one, a 7-step analytical workflow, and quality gates that enforce rigor. This skill both operates standalone (via the decide command) and enriches other commands when structured thinking is needed.

## Activation Triggers

Activate when language implies analysis, decision-making, or structured thinking:
- "Should we..." — Decision needed
- "Why is this happening..." — Diagnostic needed
- "Is this really true..." — Hypothesis testing needed
- "Which of these..." — Comparison needed
- "How big is this..." — Estimation needed
- "What's the landscape..." — Environmental scan needed
- "Where should we focus..." — Prioritization needed
- "What could go wrong..." — Risk assessment needed

Also activate as an enrichment layer when other commands encounter ambiguity:
- assess finds conflicting signals — decompose with a diagnostic issue tree
- plan encounters a fork in strategy — frame with SCQA and issue tree
- synthesize reveals a pattern that needs explanation — apply diagnostic framework

## Framework Selection Guide

| Question Pattern | Framework | When to Use |
|---|---|---|
| "Should we [do X]?" | SCQA + Issue Tree | Binary or multi-option decision with clear alternatives |
| "Why is [X] happening?" | Diagnostic Issue Tree | Root cause analysis when symptoms are visible but cause is unclear |
| "Is [X] really true?" | WWHTBT (What Would Have to Be True) | Testing an assumption or hypothesis against evidence |
| "Which of [A, B, C]?" | 2x2 Matrix | Choosing between options along two key dimensions |
| "How big is [X]?" | Fermi Estimation | Sizing an opportunity, market, risk, or impact when data is incomplete |
| "What's the landscape?" | Porter's Five Forces / 3Cs | Understanding competitive dynamics and strategic position |
| "Where should we focus?" | 80/20 Analysis | Finding the vital few among the trivial many |
| "What could go wrong?" | Pre-Mortem + Red Team | Stress-testing a plan or strategy before execution |

## The 7-Step Analytical Workflow

Every structured analysis follows this workflow, regardless of which framework is selected:

### Step 1: Problem Definition (SCQA)

Frame the question using SCQA:
- **Situation** — What is the current state? Ground this in workspace data (entity dashboard, intelligence, signals).
- **Complication** — What changed or is threatening to change? What makes this a problem now?
- **Question** — State the specific question to answer. Must be precise enough to test — not "What should we do about Nielsen?" but "Should we invest in a dedicated CSM for Nielsen before their Q3 renewal given the declining health trajectory?"
- **Answer** — Hypothesis. State your best guess before analyzing. This creates a testable proposition.

**Quality gate:** The question must be specific enough that you could definitively answer it with evidence. If it is too broad, decompose it.

### Step 2: Scope

Define boundaries:
- What is in scope and out of scope
- What timeframe are we analyzing
- What data sources are available in the workspace
- What data is missing (flag gaps early)

### Step 3: Issue Tree (MECE Decomposition)

Break the question into sub-questions using the selected framework. The tree must be:
- **Mutually Exclusive** — No overlap between branches. Each piece of evidence maps to exactly one branch.
- **Collectively Exhaustive** — All possibilities covered. No scenario falls outside the tree.

Each branch should be testable against workspace evidence. The tree structure drives which files get read.

**Quality gate:** Review the tree. Can you point to workspace evidence for each branch? If a branch has no evidence path, flag it as a data gap — do not fill it with speculation.

### Step 4: Evidence Gathering

For each branch of the issue tree:
1. Identify which workspace files contain relevant evidence
2. Read the files and extract specific data points
3. Assess evidence strength: strong (quantitative, recent), moderate (qualitative, recent), weak (old, indirect)
4. Note contradictions — when two sources disagree, that is important, not inconvenient

**Quality gate:** Every claim must be sourced to a specific file, meeting, signal, or data point. "Nielsen health is declining" is not evidence. "Nielsen health moved from Green to Yellow on January 15, with ARR declining 12% QoQ per dashboard.json" is evidence.

### Step 5: Quality Check — WWHTBT Pass

For the leading hypothesis, ask: "What Would Have to Be True for this to be the right answer?"

List the conditions. Then check each condition against workspace evidence:
- Confirmed by evidence
- Contradicted by evidence
- No evidence available (unknown)

If multiple conditions are contradicted, the hypothesis needs revision.

### Step 6: Quality Check — Red Team Pass

Apply the Pre-Mortem + Red Team lens to the leading recommendation:
- **Pre-mortem:** Assume this recommendation was implemented and failed. What went wrong? Be specific.
- **Red Team:** If you were advising the opposing position, what is the strongest argument? Not a strawman — the genuinely strongest counter-argument.
- **Partner-critic frame:** "If I were advising the other side of this decision, I would say..."

The red team pass must produce at least one argument strong enough to give the user pause. If it does not, you have not tried hard enough.

### Step 7: Output (Pyramid Principle)

Structure the output as a P2 Memo:
1. **Bottom Line** — The recommendation, stated first. One to two sentences.
2. **Key Arguments** — The 2-4 supporting arguments, each with evidence.
3. **Alternatives Considered** — What else was evaluated and why it was not recommended.
4. **Risks** — What could go wrong with this recommendation and how to mitigate.
5. **Next Steps** — Specific actions with owners and dates.

## The 8 Frameworks in Detail

### 1. SCQA + Issue Tree
**When:** Making a decision between clear alternatives.
**Steps:** Frame with SCQA, decompose with MECE issue tree, test branches against evidence, synthesize into recommendation.
**Output:** P2 Memo with decision and supporting analysis.

### 2. Diagnostic Issue Tree
**When:** Something is happening and you need to know why.
**Steps:** Define the symptom, brainstorm possible causes (MECE), test each cause against evidence, identify root cause(s).
**Output:** Root cause analysis with evidence chain and corrective actions.

### 3. WWHTBT (What Would Have to Be True)
**When:** Testing whether a belief, assumption, or strategy holds up.
**Steps:** State the hypothesis, list conditions that must be true, test each condition against evidence, assess overall validity.
**Output:** Hypothesis validation with confidence assessment and gaps.

### 4. 2x2 Matrix
**When:** Choosing between options that vary on two important dimensions.
**Steps:** Identify the two most important dimensions, plot options on the matrix, analyze each quadrant, identify the preferred quadrant and which options land there.
**Output:** Visual matrix (text-based) with quadrant analysis and recommendation.

### 5. Fermi Estimation
**When:** Sizing something when precise data is unavailable.
**Steps:** Break the estimate into component assumptions, estimate each component, multiply through, sanity-check against known anchors, state confidence range.
**Output:** Estimate with assumptions table, range (low/base/high), and sensitivity analysis on key assumptions.

### 6. Porter's Five Forces / 3Cs
**When:** Understanding competitive position and market dynamics.
**Steps:** Analyze each force (or each C: Company, Customer, Competitor), identify where power lies, determine strategic implications.
**Output:** Environmental analysis with strategic positioning recommendations.

### 7. 80/20 Analysis
**When:** Too many things competing for attention and you need to find what matters most.
**Steps:** List all items, estimate impact of each, rank by impact, identify the 20% driving 80% of outcomes, recommend focus.
**Output:** Prioritized list with impact estimates and focus recommendation.

### 8. Pre-Mortem + Red Team
**When:** Stress-testing a plan before committing.
**Steps:** Assume the plan was executed and failed — write the post-mortem. Then switch to adversarial perspective and build the strongest counter-argument. Identify vulnerabilities and mitigations.
**Output:** Risk assessment with failure scenarios, counter-arguments, and mitigation plan.

## Quality Standards

- **Specific over generic.** "Revenue might decline" is not analysis. "ARR is likely to contract 15-20% at renewal based on the usage decline from 850 to 620 DAU over Q4" is analysis.
- **Evidence over opinion.** Every assertion traces to a source. If there is no source, label it as assumption.
- **Intellectual honesty.** If the evidence does not clearly support a recommendation, say so. Ambiguity is information.
- **Workspace-grounded.** Frameworks operate on workspace data, not abstract theory. The issue tree branches should map to readable files.

## Interaction with Other Skills

- **entity-intelligence** provides the data this skill analyzes
- **relationship-context** provides people data for stakeholder-sensitive decisions
- **political-intelligence** provides the power dynamics layer when decisions have political dimensions
- **role-vocabulary** shapes how analysis is framed (different presets have different decision types)
- **action-awareness** connects recommendations to trackable next steps
