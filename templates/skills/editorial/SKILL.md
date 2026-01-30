---
name: editorial
description: Multi-stage writing review process ensuring quality, voice consistency, and authenticity. Applies mechanical, structural, voice, craft, and authenticity checks to documents.
allowed-tools: Read, Glob, Grep, Bash
---

# Editorial Skill

Multi-stage writing review process ensuring quality, voice consistency, and authenticity.

## Overview

This skill provides a comprehensive editorial review workflow for documents, posts, and communications. It applies multiple review lenses in sequence to catch different types of issues.

## Philosophy

**Readers first** - Every word should serve the reader, not the writer.

**Earned authority** - Don't claim, demonstrate. Build credibility through substance.

**Voice consistency** - Match the content type's expected voice and tone.

**Authenticity over polish** - Better to be genuine than perfectly formulaic.

## When to Use

Invoke this skill when:
- Writing strategic documents or P2 posts
- Creating executive communications
- Drafting content that represents you professionally
- Preparing materials for senior stakeholders

## Quick Start

```
"Review this document using editorial standards"
```

or

```
"/editorial"
```

## The Review Phases

### Phase 1: Mechanical Review

**Agent**: writer-mechanical-review

Catches technical issues that can be detected programmatically:
- Typography (em-dashes, smart quotes, spacing)
- Terminology consistency
- Anti-patterns (passive voice, weak verbs)
- Formatting standards

**Output**: List of mechanical issues with line numbers.

### Phase 2: Structural Review

**Agent**: writer-structural-review

Evaluates the architecture of the argument:
- Logic flow (does each section follow from the previous?)
- Evidence integration (are claims supported?)
- Reader journey (can someone follow without backtracking?)
- Appropriate depth (too shallow? too detailed?)

**Output**: Structural feedback with specific recommendations.

### Phase 3: Voice Review

**Agent**: writer-voice-review

Checks voice fidelity for the content type:

| Content Type | Expected Voice |
|--------------|----------------|
| Executive memo | Direct, confident, evidence-based |
| Strategy doc | Structured, analytical, balanced |
| Slack update | Conversational, concise, human |
| Blog post | Accessible, engaging, personal |

**Output**: Voice consistency assessment and adjustments.

### Phase 4: Craft Review

**Agent**: writer-craft-review

Evaluates at two levels:

**Soul** (the deeper qualities):
- Permission: Have we earned the right to say this here?
- Reader journey: Does it take readers somewhere valuable?
- Authority: Demonstrated through substance, not claims?

**Mechanics** (the craft elements):
- Hooks: Does it grab attention appropriately?
- Quotable lines: Are key points memorable?
- Rhythm: Does it read well aloud?

**Output**: Craft assessment with improvement suggestions.

### Phase 5: Authenticity Review

**Agent**: writer-authenticity-review

Detects patterns that make content feel mechanical or AI-generated:

**Formulaic Patterns**:
- Predictable section ordering
- Template-driven phrasing
- Unnecessary hedging
- Over-explanation

**AI Tells**:
- Excessive parallelism
- Empty transitional phrases
- Artificial enthusiasm
- Generic conclusions

**Output**: Authenticity score with specific patterns flagged.

### Phase 6: Red Team (Optional)

**Agent**: writer-challenger

Challenges the content's premises:
- Do the claims hold up?
- Is the value proposition defensible?
- What would a skeptic say?
- What's missing?

**Output**: Critical review with challenges to address.

### Phase 7: Executive Scrutiny (For Exec-Facing)

**Agent**: writer-scrutiny

For content going to senior stakeholders, demands precision:

| Check | Question |
|-------|----------|
| Vague claims | "What does 'significant improvement' mean?" |
| Missing timelines | "When will this happen?" |
| Unquantified impact | "How much? How many?" |
| Passive ownership | "Who specifically is responsible?" |

**Output**: Scrutiny report with items requiring specificity.

## Review Workflow

### Standard Review (3 phases)

For most content:
1. Mechanical Review
2. Structural Review
3. Voice Review

### Full Review (5 phases)

For important documents:
1. Mechanical Review
2. Structural Review
3. Voice Review
4. Craft Review
5. Authenticity Review

### Executive Review (7 phases)

For exec-facing content:
1-5. Full Review phases
6. Writer Challenger (red team)
7. Writer Scrutiny (exec specificity)

## Voice Profiles by Content Type

### Executive Memo

```yaml
voice:
  tone: confident, direct
  formality: high
  personality: minimal
  structure: pyramid (answer first)

avoid:
  - hedging language
  - unnecessary context
  - passive voice
  - vague timelines
```

### Strategy Document

```yaml
voice:
  tone: analytical, balanced
  formality: high
  personality: minimal
  structure: MECE, issue-tree

avoid:
  - unsubstantiated claims
  - false precision
  - hidden assumptions
```

### Slack Summary

```yaml
voice:
  tone: conversational, efficient
  formality: low
  personality: appropriate
  structure: bullets, short

avoid:
  - formal language
  - excessive detail
  - corporate speak
```

### Blog Post

```yaml
voice:
  tone: accessible, engaging
  formality: medium
  personality: high
  structure: narrative

avoid:
  - jargon without explanation
  - AI-generated feel
  - generic conclusions
```

## Mechanical Standards

### Typography

| Element | Standard | Example |
|---------|----------|---------|
| Em-dash | No spaces | "word—word" not "word — word" |
| Lists | Parallel structure | All items same form |
| Numbers | Spell out 1-9, digits 10+ | "five items" but "12 items" |
| Acronyms | Define on first use | "Technical Account Manager (TAM)" |

### Anti-Patterns to Avoid

| Pattern | Problem | Better |
|---------|---------|--------|
| "In order to" | Wordy | "To" |
| "It is important to note that" | Filler | [just state it] |
| "Leverage" | Jargon | "Use" |
| "At this point in time" | Wordy | "Now" |
| Passive voice | Weak | Active voice |

## Authenticity Patterns

### AI Tells to Avoid

1. **Excessive parallelism**: Not every list needs perfect structure
2. **Triple emphasis**: "critical, essential, and vital" - pick one
3. **Transition padding**: "Moving forward, it's important to consider..."
4. **Artificial balance**: Not every argument needs equal treatment
5. **Generic conclusions**: "In conclusion, X is important for Y"

### Signs of Genuine Writing

1. **Asymmetric structure**: Real points have different weights
2. **Specific examples**: Concrete beats abstract
3. **Confident omission**: Leaving out the obvious
4. **Personal voice**: Occasional first person, opinions stated directly
5. **Imperfect flow**: Natural writing has some rough edges

## Integration with Other Skills

| Skill | Integration |
|-------|-------------|
| strategy-consulting | Editorial review of strategy outputs |
| inbox-processing | Review summaries before filing |

## Best Practices

1. **Don't over-polish** - Authentic beats perfect
2. **Match the context** - A Slack message shouldn't read like a memo
3. **Read aloud** - Catches rhythm and awkward phrasing
4. **Kill your darlings** - Cut clever lines that don't serve the reader
5. **State, don't claim** - "Results improved 20%" not "Significant improvement achieved"

## Example Review Output

```markdown
## Editorial Review: Q1 Strategy Memo

### Mechanical Issues (3)
- Line 12: "In order to" → "To"
- Line 45: Number format inconsistent (mix of spelled/digits)
- Line 67: Missing serial comma

### Structural Feedback
- Strong: Pyramid structure, answer first
- Improve: Section 3 jumps topics without transition
- Consider: Move risk discussion earlier

### Voice Assessment
- Content type: Executive memo
- Voice match: 85%
- Issues: Paragraph 2 too conversational for context

### Authenticity Score: 78/100
- Flagged: Excessive parallelism in bullet points
- Flagged: Conclusion feels generic
- Suggestion: Vary structure, add specific example

### Executive Scrutiny
- "Significant improvement" - needs quantification
- "Soon" - needs specific timeline
- "Team" - needs named owner
```

## Troubleshooting

**Content feels flat after review:**
- May have over-corrected
- Re-inject some personality
- Let some rough edges remain

**Taking too long:**
- Use Standard Review for most content
- Reserve Full/Executive for high-stakes

**Feedback conflicts:**
- Voice > Mechanical for style issues
- Substance > Style always
