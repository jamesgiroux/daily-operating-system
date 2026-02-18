---
description: Run challenger gate on a draft -- PROCEED, SHARPEN, RECONSIDER, or KILL
---

# /challenge - Challenger Assessment

Run the challenger gate on a draft to get an honest assessment of whether the content is worth publishing. Returns a verdict of PROCEED, SHARPEN, RECONSIDER, or KILL with detailed reasoning.

$ARGUMENTS: File path to the draft, or "current draft" to assess the most recent working draft.

## Workflow

### Step 1: Read the Draft

Read the provided file path or locate the current working draft. If no file is specified and no draft is in progress, ask the user which file to assess.

If the user provides a content brief or outline instead of a draft, shift to premise evaluation mode rather than delivery evaluation.

### Step 2: Activate Challenger Skill

The challenger skill activates and performs the full assessment.

### Step 3: First-Pass Filter

Before detailed analysis, answer the threshold question:
- "Has this been written a thousand times already?"
- "Is there a genuine, lived insight here?"
- "Would we be adding to the pile or saying something new?"

If the topic is well-trodden territory with no differentiated angle, recommend KILL immediately. Do not try to salvage it.

### Step 4: Full Question Bank

**For briefs/premises** (evaluating "is there an article here?"):

SO WHAT?
- "Why should anyone care about this?"
- "What's the actual insight here?"
- "Who benefits from reading this, specifically?"

SUBSTANCE CHECK
- "Is there actually an article here, or just an observation?"
- "Strip away the frameworks - what's left?"
- "Could someone write the opposite and be equally convincing?"

SPECIFICITY TEST
- "Can you summarize the insight in one sentence that isn't obvious?"
- "What does the reader know after reading that they didn't before?"

**For drafts** (evaluating "did this deliver on its promise?"):

EVIDENCE AUDIT
- "Where's the evidence for this?"
- "Are we cherry-picking? What evidence would contradict this?"

ROSY PICTURE CHECK
- "What's the downside you're not mentioning?"
- "Who would disagree and why?"

QUALITY TESTS
- "Would you send this to your smartest peer?" (cringe test)
- "What would they learn that they don't already know?" (net-new test)
- "Can the reader do something different tomorrow?" (action test)

FRAMEWORK CHALLENGES
- "Is there an established framework that already covers this?"
- "What's genuinely novel here vs. repackaging existing wisdom?"

**DailyOS workspace awareness**: When in a DailyOS workspace, check entity intelligence and meeting archives for evidence supporting or contradicting the thesis.

### Step 5: Compression Test

Apply the compression test:
> "In one sentence, what does the reader learn that they didn't know before?"

- **Obvious**: No article here
- **Vague**: Needs sharpening
- **Specific + Non-obvious**: There is something here

### Step 6: Deliver Verdict

Present the challenger assessment in the standard format:

```markdown
## Challenger Assessment

### Verdict: [PROCEED / SHARPEN / RECONSIDER / KILL]

### The Core Question
[One sentence: What is this piece trying to say?]

### Does It Pass the "So What" Test?
[Honest assessment]

### Assumption Audit
[Key assumptions challenged or validated]

### Claim Audit
[Key claims with evidence status]

### The Rosy Picture
[What we are not saying]

### The Steel-Man Opposition
[Best argument against this piece]

### Net-New Value Assessment
Score: [High / Medium / Low / None]

### Recommendation
[What needs to change, or why to proceed/kill]
```

### Verdict Definitions

| Verdict | Meaning | Action |
|---------|---------|--------|
| **PROCEED** | Solid premise, supported claims, clear value | Continue to next phase |
| **SHARPEN** | Good bones, needs focus and refinement | Refine before continuing |
| **RECONSIDER** | Significant issues with premise or approach | May need fundamental rethinking |
| **KILL** | Not worth pursuing | Stop and redirect effort to better topic |
