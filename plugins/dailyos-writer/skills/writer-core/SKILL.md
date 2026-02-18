---
name: writer-core
description: Multi-phase editorial workflow orchestrator with specialized voices, internal review cycles, and challenger gates. Creates thought leadership, strategic documents, status reports, and customer communications with writer's room quality control.
---

# Writer Core - Editorial Workflow Orchestrator

An orchestrated writing workflow with specialized voices, internal review cycles, and challenger gates. Work goes through "writer's room" review before reaching the human.

## Activation

This skill activates when the user initiates a writing project, requests content creation, or invokes the write command. It orchestrates the full editorial pipeline from discovery through polish.

---

## The Workflow

### Core Principle: Writer's Room Quality Control

Each phase has **internal review cycles** before the human sees output. The human is not the first reviewer; they are the decision-maker receiving already-debated, refined work.

```
+-------------------------------------------------------------------+
|  PHASE 0: DISCOVERY                                                |
|  ------------------                                                |
|  1. Scan project directory for existing drafts                     |
|  2. Check if topic already has a draft in progress                 |
|  3. If draft exists: offer to review/revise vs. start fresh        |
|  4. If no draft: proceed to ideation                               |
|                                                                    |
|  --> Ensures we don't duplicate work or miss existing content      |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 1: IDEATION                                                 |
|  ----------------                                                  |
|  1. Determine content type (prompt if unclear)                     |
|  2. Load voice profile + template options                          |
|  3. Develop thesis, audience analysis, content brief               |
|  4. [INTERNAL] The challenger skill tests the premise              |
|  5. Refine based on challenge                                      |
|                                                                    |
|  --> HUMAN GATE: Approve direction?                                |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 2: RESEARCH                                                 |
|  ---------------                                                   |
|  1. When in a DailyOS workspace, read entity intelligence,         |
|     meeting archives, and stakeholder quotes FIRST                 |
|  2. Search existing documents for evidence                         |
|  3. Pull customer quotes, data points, outcomes                    |
|  4. Web research for external validation, fact-checking,           |
|     background context, and reference material                     |
|  5. Identify gaps requiring user input                             |
|                                                                    |
|  --> HUMAN GATE: Review evidence, fill gaps                        |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 3: STRUCTURE                                                |
|  ----------------                                                  |
|  1. Present template options for content type                      |
|  2. Generate outline with selected template                        |
|  3. Map evidence to sections                                       |
|  4. [INTERNAL] The challenger + structural-review skills activate   |
|  5. Refine outline                                                 |
|                                                                    |
|  --> HUMAN GATE: Approve outline?                                  |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 4: DRAFTING                                                 |
|  ---------------                                                   |
|  1. Write section by section following outline                     |
|  2. Apply voice profile                                            |
|  3. Integrate evidence naturally                                   |
|  4. Flag uncertain passages                                        |
|                                                                    |
|  --> No gate - proceeds to review                                  |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 5: REVIEW (Multi-Pass)                                      |
|  ----------------------------                                      |
|  Pass A: The mechanical-review skill activates (automated scripts) |
|  Pass B: The structural-review skill activates (logic, flow)       |
|  Pass C: The voice-review skill activates (voice fidelity)         |
|  Pass D: The authenticity-review skill activates (anti-formula)    |
|  Pass E: The scrutiny skill activates (exec-facing only)           |
|  Pass F: The challenger skill activates (delivery on promise)      |
|                                                                    |
|  --> HUMAN GATE: Review draft with flagged items                   |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 6: REVISION                                                 |
|  ---------------                                                   |
|  1. Incorporate human feedback                                     |
|  2. Re-run review passes on changed sections                       |
|  3. Maintain what's working                                        |
|                                                                    |
|  --> Loop to Review or proceed to Polish                           |
+-------------------------------------------------------------------+
                             |
                             v
+-------------------------------------------------------------------+
|  PHASE 7: POLISH                                                   |
|  --------------                                                    |
|  1. Final mechanics pass                                           |
|  2. Format for target platform                                     |
|  3. Add metadata/frontmatter                                       |
|  4. Generate supporting assets (titles, excerpt)                   |
|                                                                    |
|  --> HUMAN GATE: Confirm ready to publish                          |
+-------------------------------------------------------------------+
```

---

## Content Type Detection

If unclear from the user's prompt, ask:

```
What type of content are we creating?

- Thought Leadership (HBR-style articles for practitioners)
- Strategic Update (Partnership, competitive, executive summary)
- Status Report (Weekly, monthly, quarterly)
- Vision Document (Strategy, planning, roadmap)
- Video Script (Documentary, thought leadership video)
- Podcast Outline (Interview, discussion)
- Customer Communication (QBR, renewal, executive briefing)
- Blog Post (Long-form blog content)
- Other (describe it)
```

### Content Type to Voice Profile Mapping

| Content Type | Voice Profile | Default Template |
|--------------|---------------|------------------|
| Thought Leadership | `skills/voices/thought-leadership.yaml` | hook-problem-reframe |
| Strategic Update | `skills/voices/strategic.yaml` | bluf-standard |
| Status Report | `skills/voices/status-report.yaml` | weekly-impact / monthly-rollup |
| Vision Document | `skills/voices/strategic.yaml` | strategy-memo |
| Video Script | `skills/voices/narrative.yaml` | documentary-arc |
| Customer Communication | `skills/voices/customer.yaml` | qbr-narrative |
| Blog Post | `skills/voices/blog.yaml` | hook-problem-reframe |

---

## Input Sources

The writer can work from multiple starting points:

| Source | How to Invoke | What Happens |
|--------|---------------|--------------|
| **Topic from calendar** | "Week 5 from Leadership Content" | Load topic context, thread, related articles |
| **Document/Transcript** | "I have a transcript..." | Extract key points, quotes, evidence |
| **Prompt/Idea** | "I want to write about..." | Ideation from scratch |
| **Existing outline** | "Here's my outline..." | Skip to drafting |
| **Content brief** | "Here's the brief..." | Skip ideation, go to research |

---

## Phase Details

### Phase 1: Ideation

**Objective**: Define what we are writing and why, challenged before human approval.

**Process**:
1. Identify content type from user input (prompt if unclear)
2. Load appropriate voice profile from `skills/voices/`
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

### Phase 2: Research

**Objective**: Assemble supporting material from workspace data, internal documents, and external sources.

**DailyOS Workspace-First Research**: When operating in a DailyOS workspace, the research phase reads entity intelligence, meeting archives, and stakeholder quotes BEFORE any web search. This ensures content is grounded in real organizational context.

**Process**:
1. When in a DailyOS workspace, the research skill activates workspace-first search
2. Search existing documents for relevant quotes/data
3. Pull customer evidence (transcripts, metrics, outcomes)
4. **Web research** (content-type dependent):
   - **Strategic content**: Fact-check claims, gather market data, validate competitive positioning
   - **Thought leadership**: Find supporting frameworks, identify existing thinking to reference or contrast
   - **Customer communications**: Research customer's recent news, earnings, strategic priorities
   - **All types**: Verify terminology, find authoritative sources to cite
5. Identify gaps requiring user input

**Output: Evidence Inventory**

```markdown
## Evidence Inventory

### Workspace Evidence (DailyOS)
#### Entity Intelligence
- [Entity] - [key insight] - Source: [intelligence file]

#### Meeting Archives
- [Quote/Decision] - Source: [archive file/date]

#### Stakeholder Context
- [Context point] - Source: [person file]

### Internal Evidence
#### Customer Quotes
- [Quote] - Source: [file/date]

#### Data Points
- [Metric] - Source: [file/date]

#### Stories/Examples
- [Story summary] - Source: [file/date]

### External Evidence (Web Research)
#### Market/Industry Context
- [Finding] - Source: [URL]

#### Framework References
- [Framework/concept] - Source: [publication/author]

#### Customer Background
- [Recent news/priority] - Source: [URL]

#### Fact Checks
- [Claim verified/corrected] - Source: [URL]

### Gaps (Need User Input)
- [ ] [What's missing and why it matters]
```

---

### Phase 3: Structure

**Objective**: Create the skeleton with internal review.

**Process**:
1. Present template options for content type (templates located in `skills/templates/`):
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

### Phase 4: Drafting

**Objective**: Get words on paper.

**Process**:
1. Write section by section following outline
2. Apply voice profile (load from `skills/voices/[type].yaml`)
3. Integrate evidence naturally
4. Flag uncertain passages

**Exit Criteria**:
- All sections written
- No placeholder text
- Evidence integrated (not just listed)

---

### Phase 5: Review (Multi-Pass)

**Objective**: Internal quality control before human review.

#### Pass A: Mechanical (Automated)

The mechanical-review skill activates. Run scripts:
```bash
python3 skills/scripts/lint_typography.py <file>
python3 skills/scripts/detect_patterns.py <file>
```

Checks:
- Em-dashes: replace with parentheses or periods
- Curly quotes, Oxford comma
- Terminology consistency
- Contrast framing patterns
- Throat-clearing phrases

#### Pass B: Structural

The structural-review skill activates. Questions:
- Does the opening earn attention?
- Does each section advance the argument?
- Are transitions explicit or jarring?
- Does evidence support claims (or just sit nearby)?
- Does the conclusion deliver on the opening promise?
- Are next steps specific (owners, dates)?

#### Pass C: Voice (Content-Type Specific)

The voice-review skill activates. Loads voice profile from `skills/voices/` and checks content-type-specific criteria.

#### Pass D: Authenticity (The Human Test)

The authenticity-review skill activates. Checks for:
- AI-tell detection (rigid structure, transition stuffing, generic openings)
- Burstiness (paragraph and sentence length variation)
- Template overreliance
- Genuine insight presence

#### Pass E: Scrutiny (Exec-Facing Content Only)

The scrutiny skill activates on: executive briefings, success plans, EBRs, QBRs, renewal narratives, expansion proposals.

Skip for: Thought leadership, internal drafts, status reports.

#### Pass F: Challenger

The challenger skill activates on the draft:
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
- Mechanical issues: 0 remaining
- Structural issues: 0 critical, at most 2 minor (flagged for human)
- Voice issues: 0 critical
- Authenticity: No AI-tells detected
- Scrutiny (exec-facing only): All vague claims have specifics, timelines exist, metrics quantified
- Challenger: Verdict is PUBLISH or REVISE (not RECONSIDER/KILL)
- Max 3 iterations

---

### Phase 6: Revision

**Objective**: Incorporate human feedback.

**Process**:
1. Address specific feedback points
2. Maintain what is working
3. Re-run relevant review passes on changed sections
4. Update review summary

---

### Phase 7: Polish

**Objective**: Production-ready output.

**Process**:
1. Final mechanics pass
2. Format for target platform
3. Add metadata/frontmatter
4. Generate supporting assets:
   - Title options (3-5)
   - BLUF/excerpt
   - Social snippets (if applicable)

---

## Specialized Skills

The orchestrator relies on specialized skills for review phases:

| Skill | Purpose |
|-------|---------|
| research | Evidence gathering from workspace data, internal docs, and web |
| challenger | Premise testing, "so what" assessment, value gate |
| scrutiny | Executive specificity: vague claims, timelines, metrics |
| mechanical-review | Pattern detection, typography, linting scripts |
| structural-review | Logic, flow, coherence |
| voice-review | Voice fidelity per content type |
| authenticity-review | Anti-formula, AI-tell detection |

---

## Resources

### Voice Profiles
Located in `skills/voices/`:
- `strategic.yaml` - Executive communications, partnership updates
- `thought-leadership.yaml` - HBR-style articles for practitioners
- `narrative.yaml` - Video scripts, documentary content
- `status-report.yaml` - Weekly, monthly, quarterly reports
- `customer.yaml` - QBR narratives, renewal cases
- `blog.yaml` - Long-form blog content

### Templates
Located in `skills/templates/`:

**thought-leadership/** (5 templates)
- `hook-problem-reframe.md` - Standard arc for introducing new perspectives
- `counterintuitive-claim.md` - Provocative thesis, then earn it
- `framework-introduction.md` - Teaching borrowed frameworks
- `story-driven.md` - Personal story as spine
- `comparison.md` - Contrasting two approaches

**strategic/** (5 templates)
- `bluf-standard.md` - Bottom Line Up Front
- `scqa.md` - Situation-Complication-Question-Answer
- `pyramid-principle.md` - Conclusion first, then support
- `executive-briefing.md` - One-pager for exec consumption
- `competitive-win.md` - Win narrative with evidence

**status-reports/** (4 templates)
- `weekly-impact.md` - Weekly impact capture
- `monthly-rollup.md` - Monthly aggregation
- `quarterly-review.md` - Quarterly performance narrative
- `project-update.md` - Multi-week project tracking

**vision/** (4 templates)
- `strategy-memo.md` - Strategic proposals
- `roadmap-narrative.md` - Multi-phase plans
- `build-showcase-sell.md` - GTM framework
- `investment-case.md` - Resource requests

**narrative/** (3 templates)
- `documentary-arc.md` - Problem-solution with earned resolution
- `explainer.md` - Teaching through story
- `future-vision.md` - Aspirational destination painting

**podcast/** (3 templates)
- `interview-outline.md` - Guest conversations
- `discussion-outline.md` - Multi-person panels
- `solo-monologue.md` - Single-host episodes

**customer/** (4 templates)
- `qbr-narrative.md` - Quarterly business review storytelling
- `renewal-case.md` - Value delivered, retention justification
- `expansion-proposal.md` - Upsell and cross-sell proposals
- `customer-executive-briefing.md` - Pre-meeting context docs

### Shared Rules
Located in `skills/shared/`:
- `MECHANICS.md` - Grammar, typography (all content types)
- `TERMINOLOGY.md` - Product names, spelling (all content types)
- `ANTI-PATTERNS.md` - Universal patterns to avoid
- `DISTRIBUTION.md` - Format adaptation, repurposing, sanitization

### Scripts
Located in `skills/scripts/`:
- `lint_typography.py` - Automated typography checks
- `detect_patterns.py` - Automated anti-pattern detection

---

## Distribution and Repurposing

Content can be repurposed across channels with proper adaptation. See `skills/shared/DISTRIBUTION.md` for full guidance.

### Format Quick Reference

| Channel | Word Count | Key Adaptation |
|---------|-----------|----------------|
| Internal doc | 800-1500 | Headers, bullets, full depth |
| Slack | 50-150 | One takeaway, no headers |
| Executive email | 200-400 | BLUF, bold conclusions |
| LinkedIn | 150-300 | Hook first, personal voice |
| Personal blog | 1200-2500 | More personality, industry framing |

### Sanitization for External Use

Before publishing internally-created content externally:
- [ ] Remove customer names (use industry descriptors)
- [ ] Generalize revenue figures ("six-figure" not "$400K")
- [ ] Remove internal project names and terminology
- [ ] Reframe from company positioning to personal perspective
- [ ] Verify no confidential strategy is revealed

### Secondary Distribution Planning

During ideation, consider:
- **Primary purpose**: Where is this content for?
- **Secondary potential**: Where else could it be valuable?
- **Adaptation needed**: What changes for each channel?

---

## Framework Usage: Borrowed vs. Invented

**Default**: Use established frameworks (Pyramid Principle, SCQA, JTBD, MECE). The value is in application, not invention.

**New frameworks** only when:
- You have genuinely discovered a pattern no one has named
- Existing frameworks do not capture the specific nuance
- You have evidence across multiple instances
- You are willing to refine it over time

**Challenger questions for framework usage**:
- "Is there an established framework that already covers this?"
- "What is genuinely novel here vs. repackaging existing wisdom?"
- "Would a senior consultant roll their eyes at this?"
- "If we use an existing framework, does the piece still have value?"
