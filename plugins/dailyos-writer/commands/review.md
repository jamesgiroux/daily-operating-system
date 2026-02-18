---
description: Run the full 6-pass review cycle on a draft
---

# /review - Full Review Cycle

Run the complete 6-pass editorial review cycle on a draft. Each pass uses a specialized skill to evaluate a different dimension of content quality.

$ARGUMENTS: File path to the draft to review.

## Workflow

### Step 1: Read the Draft

Read the file at the provided path. If no path is given, ask the user which file to review.

Identify the content type from the document (frontmatter, structure, or context). This determines which review passes apply and which voice profile to use.

### Step 2: Run 6 Review Passes in Sequence

Each pass builds on the previous, so they run in order:

#### Phase 1: Mechanical Review

The mechanical-review skill activates.

- Run typography linting: `python3 skills/scripts/lint_typography.py <file>`
- Run pattern detection: `python3 skills/scripts/detect_patterns.py <file>`
- Check for em dashes, curly quotes, Oxford comma, terminology
- Detect contrast framing, negative parallels, AI tropes, stylistic crutches
- Flag template artifacts (section dividers, template headers, generic closings)
- Verify BLUF/TL;DR is present (house style requirement)
- Check heading hierarchy, list parallelism, active voice, paragraph length
- Auto-fix where possible, flag others for human decision

**Output**: Issue count, auto-fixes applied, items requiring attention.

#### Phase 2: Structural Review

The structural-review skill activates.

- Evaluate opening: hook, thesis clarity, BLUF quality
- Check each section: clear purpose, logical flow, advances argument
- Redundancy check: flag sections covering the same ground
- Assess transitions: explicit connections between sections
- Evidence audit: claims paired with nearby, supporting evidence
- Evaluate conclusion: delivers on opening promise, clear takeaways, actionable next steps
- Content-type specific checks (Pyramid Principle for strategic, problem-sitting for thought leadership)

**Output**: Critical/major/minor issue counts, section-by-section assessment.

#### Phase 3: Voice Review

The voice-review skill activates.

- Load voice profile from `skills/voices/` based on content type
- Evaluate tone consistency throughout the document
- Check for required voice elements (per profile)
- Check for prohibited voice elements (per profile)
- Detect anti-patterns specific to the content type
- Score voice fidelity: Strong / Adequate / Weak

**Output**: Voice fidelity score, deviations found, suggested corrections.

#### Phase 4: Authenticity Review

The authenticity-review skill activates.

- Detect structure tells: rigid paragraphs, predictable length, list defaulting, false balance, perfect symmetry
- Detect language tells: transition stuffing, summary crutches, generic openings, hedge stacking, enthusiasm inflation
- Burstiness analysis: paragraph length variation, sentence length variation, structural variety, rhythm
- Template overreliance check: content forced into sections, padding, structure visible through content
- Framework overuse check: acronym overload, lists-of-three, generic application
- Genuine insight test: is there something new here?
- Authentic voice test: is author personality visible?

**Output**: Authenticity score (Genuine / Adequate / Formulaic), AI-tells detected, formula issues.

#### Phase 5: Scrutiny (Exec-Facing Content Only)

The scrutiny skill activates for executive briefings, success plans, EBRs, QBRs, renewal narratives, and expansion proposals.

**Skip for**: Thought leadership, internal drafts, status reports.

- Flag capability vagueness ("content optimization capabilities" -> which capabilities?)
- Flag timeline gaps (actions without dates)
- Flag unquantified impact (claims without metrics)
- Flag missing proof points (claims without evidence)
- Flag resource hand-waving (vague resource asks)
- Flag ownership ambiguity (actions without named owners)
- When in a DailyOS workspace, suggest actual metrics from dashboard.json/intelligence files

**Output**: Specificity score, executive readiness assessment, vagueness inventory.

#### Phase 6: Challenger

The challenger skill activates as the final gate.

- Apply first-pass filter (should this even exist?)
- Run question bank: premise, claims, value, framework challenges
- Apply compression test
- Deliver verdict: PROCEED / SHARPEN / RECONSIDER / KILL

**Output**: Challenger verdict with reasoning.

### Step 3: Compile Review Summary

Present the combined results:

```markdown
## Review Summary

### Overall Assessment
| Pass | Issues Found | Resolved | Remaining | Verdict |
|------|-------------|----------|-----------|---------|
| Mechanical | [n] | [n] | [n] | PASS/FIX |
| Structural | [n] | [n] | [n] | PASS/REVISE/RESTRUCTURE |
| Voice | [n] | [n] | [n] | PASS/REVISE |
| Authenticity | [n] | [n] | [n] | PASS/REVISE |
| Scrutiny | [n] | [n] | [n] | Ready/Needs Work/N/A |
| Challenger | [n] | [n] | [n] | PROCEED/SHARPEN/RECONSIDER/KILL |

### Flagged Items for Human Decision

1. [Pass] [Description] - Options: [A] / [B]
2. [Pass] [Description] - Options: [A] / [B]
...

### Auto-Fixed Items
[List of changes made automatically during review]

### Verdict
[READY TO PUBLISH / NEEDS INPUT ON FLAGGED ITEMS / NEEDS REVISION]
```

### Step 4: Present for Human Decision

If there are flagged items, present them clearly with options. The human decides on each flagged item. Do not auto-resolve items that require judgment.

### Exit Criteria

| Condition | Status |
|-----------|--------|
| Mechanical issues remaining | Must be 0 |
| Structural critical issues | Must be 0 |
| Structural minor issues | At most 2, flagged for human |
| Voice critical deviations | Must be 0 |
| Authenticity AI-tells | Must be 0 |
| Scrutiny (if applicable) | All vague claims addressed |
| Challenger verdict | Must be PUBLISH or REVISE (not RECONSIDER/KILL) |
| Review iterations | Max 3 |

If all conditions are met: **READY TO PUBLISH**
If flagged items remain: **NEEDS INPUT ON FLAGGED ITEMS**
If critical issues remain: **NEEDS REVISION** (loop back through affected passes)
