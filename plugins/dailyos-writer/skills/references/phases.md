# Writer Pipeline — Phase Detail

Full detail for each phase of the writer workflow. `writer-core/SKILL.md` has the overview and the human gates; read this file when you are actually running a phase and need the process, outputs, and exit criteria.

## Contents
- [Phase 1: Ideation](#phase-1-ideation)
- [Phase 2: Research](#phase-2-research)
- [Phase 3: Structure](#phase-3-structure)
- [Phase 4: Drafting](#phase-4-drafting)
- [Phase 5: Review (Multi-Pass)](#phase-5-review-multi-pass)
- [Phase 6: Revision](#phase-6-revision)
- [Phase 7: Polish](#phase-7-polish)
- [Phase 8: Capture (the learn loop)](#phase-8-capture)

---

## Phase 1: Ideation

**Objective**: Define what we are writing and why, challenged before human approval.

**Process**:
1. Identify content type from user input (prompt if unclear)
2. Load the content-type voice profile from `skills/voices/`, plus `author.yaml` if the project has one, plus `voices/LEARNINGS.md`
3. Develop:
   - Working title
   - Core thesis (one sentence, specific enough someone could disagree)
   - Target reader (named, not "everyone")
   - Desired outcome / reader action
   - Key evidence needed
4. The challenger skill activates to assess the premise
5. Refine based on challenge

**Output: Content Brief**

```yaml
# CORE
title_working: "Influence Without Authority"
one_sentence: "How to get things done when you can't tell people what to do"
content_type: thought-leadership
template: hook-problem-reframe

# AUDIENCE
target_reader: "Practitioners who feel stuck without positional power"
reader_goal: "Understand how to influence cross-functional stakeholders"
reader_current_state: "Frustrated by lack of authority, defaulting to escalation"
reader_desired_state: "Confident in influence tactics, sees paths forward"

# MESSAGE
core_thesis: "Influence flows from understanding incentives, not building rapport"
key_insight: "Most people focus on relationships when they should focus on what stakeholders are measured on"
so_what: "Reframe from 'how do I get them to like me' to 'how do I make their goals easier'"
desired_action: "Reader identifies one stakeholder and maps their incentive structure"

# EVIDENCE NEEDED
must_have:
  - Personal story demonstrating the insight
  - Counter-example showing the wrong approach
  - Concrete tactic or framework to apply
nice_to_have:
  - Customer quote or example
  - Data point if available
gaps_to_fill: []

# SUCCESS CRITERIA
reader_test: "Would the target reader share this with their manager?"
cringe_test: "Would the author be proud to have their smartest peer read this?"
action_test: "Can the reader do something different tomorrow?"
```

**Exit Criteria**:
- One-sentence description is specific and non-obvious
- Target reader is named (not "everyone")
- Core thesis passes "so what" test
- Challenger verdict is PROCEED or SHARPEN (not RECONSIDER/KILL)

---

## Phase 2: Research

**Objective**: Assemble supporting material from workspace data, internal documents, and external sources.

**DailyOS Workspace-First Research**: When operating in a DailyOS workspace, the research phase reads entity intelligence, meeting archives, and stakeholder quotes BEFORE any web search. This ensures content is grounded in real organizational context.

**Process**:
1. When in a DailyOS workspace, the research skill activates workspace-first search
2. Search existing documents for relevant quotes/data
3. Pull supporting evidence (transcripts, metrics, outcomes)
4. **Web research** (content-type dependent):
   - **Strategic content**: Fact-check claims, gather market data, validate competitive positioning
   - **Thought leadership**: Find supporting frameworks, identify existing thinking to reference or contrast
   - **Customer communications**: Research the customer's recent news, earnings, strategic priorities
   - **All types**: Verify terminology, find authoritative sources to cite
5. Identify gaps requiring user input

**Never write an absence claim** ("nobody does X", "no competitor offers Y") without per-subject evidence. Narrow or cut. (See `shared/AI-TELLS.md` Class 13.)

**Output: Evidence Inventory**

```markdown
## Evidence Inventory

### Workspace Evidence (DailyOS)
- [Entity] - [key insight] - Source: [intelligence file]
- [Quote/Decision] - Source: [archive file/date]

### Internal Evidence
- [Quote / data point / story] - Source: [file/date]

### External Evidence (Web Research)
- [Finding / framework / fact check] - Source: [URL]

### Gaps (Need User Input)
- [ ] [What's missing and why it matters]
```

---

## Phase 3: Structure

**Objective**: Create the skeleton with internal review.

**Process**:
1. Present template options for content type (templates in `skills/templates/`):
   ```
   You're writing a thought-leadership article. Which format fits best?

   - Hook -> Problem -> Reframe (standard)
   - Counterintuitive Claim
   - Framework Introduction
   - Story-Driven
   - Comparison
   - Custom
   ```
2. Generate outline with selected template
3. Map evidence to sections
4. The challenger and structural-review skills activate internally
5. Refine outline

**Lived-story override**: if the user briefed this with a real story or journey, that arc IS the outline. Don't impose a framework on top of it.

**Output: Outline for Approval**

```markdown
## Outline for Approval

### Structure
[The outline with section summaries]

### Internal Review Notes
- Challenger flagged Section 3 as weakest - strengthened by [change]
- Structural review suggested reordering X and Y for better flow
- Evidence gap identified in Section 2 - will need [specific source]

### Recommendation
[PROCEED / NEEDS DISCUSSION]
```

**Exit Criteria**:
- All sections have clear purpose
- Evidence is mapped to sections (no orphan claims)
- Transitions are planned
- Structural review passes

---

## Phase 4: Drafting

**Objective**: Get words on paper.

**Process**:
1. Write section by section following outline
2. Apply the content-type voice profile, plus `author.yaml` if present, at the right register
3. Integrate evidence naturally
4. Flag uncertain passages; leave `[bracketed gaps]` for facts you do not have rather than inventing them

**Exit Criteria**:
- All sections written
- No placeholder text (other than intentional `[bracketed gaps]`)
- Evidence integrated (not just listed)

---

## Phase 5: Review (Multi-Pass)

**Objective**: Internal quality control before human review.

### Pass A: Mechanical (Automated)

The mechanical-review skill activates:
```bash
python3 skills/scripts/lint_typography.py <file>
python3 skills/scripts/detect_patterns.py <file>
```
`detect_patterns.py` flags: contrast framing, throat-clearing, abstract-noun equations ("X is the gate"), setup-colons, inflated diction, copula avoidance, and vague attribution. See `shared/AI-TELLS.md`.

### Pass B: Structural
- Does the opening earn attention?
- Does each section advance the argument?
- Are transitions explicit or jarring?
- Does evidence support claims (or just sit nearby)?
- Does the conclusion deliver on the opening promise?
- Are next steps specific (owners, dates)?

### Pass C: Voice
The voice-review skill activates. It loads the content-type profile, then `author.yaml` if present, then `voices/LEARNINGS.md` (which wins ties), and checks structure/register plus any author hard-rules.

### Pass D: Authenticity (the AI-tell gate)
The authenticity-review skill activates. Its primary reference is `shared/AI-TELLS.md` (aphorism vs. narrator) plus `voices/LEARNINGS.md`. It does not just flag — it **rewrites** flagged lines into narrator mode. The bar: a reader actively suspicious of AI writing can't point at a line and say "a bot wrote that."

### Pass E: Scrutiny (exec-facing content only)
The scrutiny skill activates on executive briefings, success plans, EBRs, QBRs, renewal narratives, expansion proposals. Skip for thought leadership, internal drafts, status reports.

### Pass F: Challenger
- Did we deliver on the promise?
- Is the insight genuine or obvious?
- Would a smart reader learn something new?
- Verdict: PUBLISH / REVISE / RECONSIDER / KILL

**Output: Draft for Review**

```markdown
## Draft for Review

### The Draft
[Full draft]

### Review Summary
| Pass | Issues Found | Issues Resolved | Remaining |
|------|--------------|-----------------|-----------|
| Mechanical | 12 | 12 | 0 |
| Structural | 3 | 2 | 1 (flagged) |
| Voice | 2 | 2 | 0 |
| Authenticity | 1 | 1 | 0 |
| Scrutiny | 4 | 3 | 1 (flagged) |
| Challenger | 2 | 1 | 1 (flagged) |

### Flagged Items for Human Decision
1. [Structural] Section 4 transition feels abrupt - two options proposed
2. [Challenger] "The reframe isn't counterintuitive enough" - your call

### Recommendation
[READY TO PUBLISH / NEEDS YOUR INPUT ON FLAGGED ITEMS]
```

**Exit Criteria**:
- Mechanical: 0 remaining
- Structural: 0 critical, at most 2 minor (flagged)
- Voice: 0 critical (author hard-rules satisfied if author.yaml present)
- Authenticity: no AI-tells survive; no class appears 3+ times
- Scrutiny (exec-facing only): vague claims specified, timelines exist, metrics quantified
- Challenger: PUBLISH or REVISE
- Max 3 iterations

---

## Phase 6: Revision

**Objective**: Incorporate human feedback.

1. Address specific feedback points
2. Maintain what is working
3. Re-run relevant review passes on changed sections
4. Update review summary
5. **Any voice correction the human makes here is a candidate for Phase 8 capture.**

---

## Phase 7: Polish

**Objective**: Production-ready output.

1. Final mechanics pass
2. Format for target platform — see `shared/DISTRIBUTION.md`
3. Add metadata/frontmatter
4. Generate supporting assets: title options (3-5), BLUF/excerpt, social snippets (if applicable)

---

## Phase 8: Capture

**Objective**: Make the writer more author-shaped every time the author corrects it.

Trigger: end of a session where the human changed the voice of a draft, or an explicit `/writer:learn`.

Process:
1. Diff what the human changed against what the skill produced. Isolate the *voice* corrections (not content/fact edits).
2. For each, write a `voices/LEARNINGS.md` entry: scope, before, after, the generalizable rule, status `active`.
3. If a rule already in LEARNINGS just recurred (same shape twice), **graduate it**: move it into `voices/author.yaml` (author-voice rule) or `shared/AI-TELLS.md` + `scripts/detect_patterns.py` (mechanical tell), and mark the entry `graduated`.
4. Keep entries specific and example-driven.

See `references/voice-system.md` for the full loop.
