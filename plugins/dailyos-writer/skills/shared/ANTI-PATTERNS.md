# Universal Anti-Patterns

Patterns to avoid across all content types. These are detected by `scripts/detect_patterns.py` and flagged during the Mechanical review pass.

## Voice Anti-Patterns

### 1. Contrast Framing

**Pattern**: "They weren't asking if we could help. They were asking how fast we could move."

**Fix**: "They wanted to move fast."

**Why**: Contrast framing creates unnecessary tension and sounds like you're arguing against a position no one took.

**More examples**:

| Contrast framing | Direct statement |
|-----------------|------------------|
| "They're not treating WordPress VIP as a vendor. They're treating us as strategic partners." | "The Salesforce team is treating WordPress VIP as a strategic partner." |
| "This isn't just about one opportunity." | "Salesforce is building the infrastructure..." |
| "What we're seeing isn't speculative interest. It's commercial traction." | "We're seeing commercial traction." |

### 2. Negative Parallels

**Pattern**: "Unlike our competitors who lock customers in..."

**Fix**: "WordPress VIP connects to what enterprises already use."

**Why**: Positioning by what you're NOT creates defensive framing. Lead with what you ARE.

### 3. Over-the-Top Validation

**Pattern**: "You're absolutely right and this is such brilliant thinking!"

**Fix**: "That analysis aligns with what we're seeing from customer conversations."

**Why**: Excessive validation feels insincere. Objective agreement is more credible.

### 4. Claiming vs. Demonstrating

**Pattern**: "Our strategic analysis shows..."

**Fix**: Show the analysis through evidence, cross-functional insights, and pattern recognition.

**Why**: Strategic thinking is demonstrated through the quality of your insights, not announced.

### 5. Unjustified Superlatives

**Pattern**: "This is a revolutionary game-changing moment for the industry!"

**Fix**: "Dreamforce created visibility. These conversations are creating outcomes."

**Why**: Superlatives without specific evidence sound like marketing copy.

### 6. Conversational Throat-Clearing

**Pattern**: Short phrases (3-4 words) followed by a colon that act as run-up to the real sentence.

**Examples to avoid**:
- "Here's the thing:"
- "Here's what I've learned:"
- "The bottom line:"
- "The point is:"
- "The reality is:"
- "The truth is:"

**Fix**: Delete the throat-clearing and start with the substance.

| Throat-clearing | Direct |
|-----------------|--------|
| "Here's the thing. Busy executives don't have time." | "Busy executives don't have time." |
| "The bottom line: we need to move faster." | "We need to move faster." |

**Why**: These phrases delay the point and feel like verbal tics.

## AI-Tell Anti-Patterns

Patterns that make content feel AI-generated rather than authentically written.

### Structure Tells

- **Rigid paragraph structure**: Every paragraph follows topic sentence → support → summary
- **Predictable sentence length**: Alternating short-long-short without natural variation
- **List defaulting**: Using bullets when narrative would be stronger
- **False balance**: "On one hand... on the other hand..." when you have a clear view

### Language Tells

- **Transition word stuffing**: "Furthermore", "Moreover", "Additionally" without need
- **Summary crutches**: "In summary", "In conclusion", "To recap"
- **Generic openings**: "In today's fast-paced world...", "It's no secret that..."
- **Hedge stacking**: "It could potentially perhaps be argued that..."
- **Enthusiasm inflation**: "Incredibly", "Absolutely", "Truly remarkable"

### Burstiness Issues

Good writing has natural rhythm variation:
- Short punch. Then longer exploration that develops the idea fully.
- Paragraph lengths vary based on content needs.
- Sentence structure changes to match the point being made.

**Red flag**: If all paragraphs are roughly the same length and all sentences follow similar patterns, the writing feels monotone and formulaic.

## Evidence Anti-Patterns

### Vague Claims

| Vague | Specific |
|-------|----------|
| "Significant traction" | "Three enterprise deals in pipeline totaling $400K" |
| "Strong momentum" | "Partnership formalization moving to legal review" |
| "Positive feedback" | "Keith from Salesforce said: '[specific quote]'" |

### Orphan Claims

Claims that sit in the document without supporting evidence nearby.

**Bad**: "Our positioning is resonating with the market."
**Good**: "Our positioning is resonating. In the NXP deal, Salesforce AEs brought us in within 24 hours of learning about the requirement."

### Evidence Dumping

Listing evidence without integrating it into the narrative.

**Bad**:
```
Here's what we heard:
- Quote 1
- Quote 2
- Quote 3
```

**Good**: Weave quotes into the argument where they support specific points.

## Momentum vs. Hype

### Momentum (Good)

Describing real progress with measurable traction:
- "The momentum is translating into commercial activity"
- "Partnership formalization is moving forward"
- "Three AEs reached out in the same week"

### Hype (Bad)

Overstating significance with marketing language:
- "This is absolutely game-changing for the entire industry!"
- "We're going to completely dominate this space!"
- "Nobody else can even come close!"

## The Read-Aloud Test

If it sounds like you're trying to convince someone who's skeptical, you're probably using contrast framing or defensive language.

**Sounds defensive**: "The doors aren't closed. They're wide open and we need to move!"

**Sounds collaborative**: "The doors are open."

Good strategic writing sounds like you're sharing observations with a peer, not arguing a case.
