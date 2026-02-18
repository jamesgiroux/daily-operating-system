---
description: Start a new writing project with full editorial workflow
---

# /write - Full Editorial Workflow

Start a new writing project using the complete 7-phase editorial pipeline with writer's room quality control.

## Workflow

### Step 1: Content Type Detection

Determine the content type from the user's input. If unclear, prompt:

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

| Content Type | Voice Profile | Default Template Directory |
|--------------|---------------|---------------------------|
| Thought Leadership | `skills/voices/thought-leadership.yaml` | `skills/templates/thought-leadership/` |
| Strategic Update | `skills/voices/strategic.yaml` | `skills/templates/strategic/` |
| Status Report | `skills/voices/status-report.yaml` | `skills/templates/status-reports/` |
| Vision Document | `skills/voices/strategic.yaml` | `skills/templates/vision/` |
| Video Script | `skills/voices/narrative.yaml` | `skills/templates/narrative/` |
| Podcast Outline | `skills/voices/narrative.yaml` | `skills/templates/podcast/` |
| Customer Communication | `skills/voices/customer.yaml` | `skills/templates/customer/` |
| Blog Post | `skills/voices/blog.yaml` | `skills/templates/thought-leadership/` |

### Step 2: Activate Writer Core

The writer-core skill activates and runs the full 7-phase workflow:

**Phase 0 - Discovery**: Scan for existing drafts on this topic. If found, offer to review/revise vs. start fresh.

**Phase 1 - Ideation**: Develop thesis, audience analysis, and content brief. The challenger skill tests the premise internally. Present content brief for human approval.

**HUMAN GATE**: Approve direction before proceeding.

**Phase 2 - Research**: The research skill gathers evidence.
- **DailyOS workspace-first**: When in a DailyOS workspace, read entity intelligence, meeting archives, stakeholder quotes, action trails, and email signals BEFORE web search. Workspace data provides the most specific and authentic material.
- Search internal documents for supporting evidence.
- Conduct web research for external validation and fact-checking.
- Compile evidence inventory and flag gaps.

**HUMAN GATE**: Review evidence, fill identified gaps.

**Phase 3 - Structure**: Present template options for the content type. Generate outline, map evidence to sections. The challenger and structural-review skills activate internally to refine. Present outline for human approval.

**HUMAN GATE**: Approve outline before drafting.

**Phase 4 - Drafting**: Write section by section. Apply voice profile. Integrate evidence naturally. Flag uncertain passages.

**Phase 5 - Review**: Run the full 6-pass review cycle:
1. **mechanical-review** skill: Typography, terminology, anti-patterns (automated scripts)
2. **structural-review** skill: Logic, flow, evidence integration
3. **voice-review** skill: Voice fidelity against content type profile
4. **authenticity-review** skill: AI-tells, formulaic pattern detection
5. **scrutiny** skill: Executive specificity (skip for non-exec content)
6. **challenger** skill: Final gate on delivery vs. promise

Issues are resolved internally where possible. Remaining items are flagged for human decision.

**HUMAN GATE**: Review draft with flagged items. Provide feedback.

**Phase 6 - Revision**: Incorporate human feedback. Re-run review passes on changed sections. Maintain what works.

**Phase 7 - Polish**: Final mechanics pass. Format for target platform. Generate title options, BLUF/excerpt, and social snippets if applicable.

**HUMAN GATE**: Confirm ready to publish.

### Input Source Handling

| Source | Detection | Workflow Entry |
|--------|-----------|----------------|
| Topic from calendar | User references a scheduled topic | Load context, start at Phase 1 |
| Document/Transcript | User provides or references a file | Extract key points, start at Phase 1 |
| Prompt/Idea | User describes a topic | Full workflow from Phase 0 |
| Existing outline | User provides structure | Skip to Phase 4 (Drafting) |
| Content brief | User provides brief | Skip Phase 1, start at Phase 2 (Research) |

### Review Summary Output

At the end of the review phase, present a summary table:

```markdown
### Review Summary
| Pass | Issues Found | Issues Resolved | Remaining |
|------|--------------|-----------------|-----------|
| Mechanical | [n] | [n] | [n] |
| Structural | [n] | [n] | [n] |
| Voice | [n] | [n] | [n] |
| Authenticity | [n] | [n] | [n] |
| Scrutiny | [n] | [n] | [n] |
| Challenger | [n] | [n] | [n] |

### Flagged Items for Human Decision
[Numbered list of items requiring human input]

### Recommendation
[READY TO PUBLISH / NEEDS YOUR INPUT ON FLAGGED ITEMS / NEEDS REVISION]
```

### Exit Criteria (per phase)

| Phase | Exit Criteria |
|-------|---------------|
| Ideation | Thesis passes "so what" test, challenger verdict PROCEED or SHARPEN |
| Research | Evidence inventory complete, gaps identified and flagged |
| Structure | All sections have clear purpose, evidence mapped, structural review passes |
| Drafting | All sections written, no placeholders, evidence integrated |
| Review | Mechanical: 0 remaining. Structural: 0 critical. Voice: 0 critical. Authenticity: no AI-tells. Challenger: PUBLISH or REVISE. Max 3 iterations. |
| Polish | Platform-formatted, metadata added, supporting assets generated |

### Distribution Planning

During ideation, consider secondary distribution:
- Primary purpose and channel
- Secondary channels where the content could be valuable
- Adaptation needed per channel (see `skills/shared/DISTRIBUTION.md`)

| Channel | Word Count | Key Adaptation |
|---------|-----------|----------------|
| Internal doc | 800-1500 | Headers, bullets, full depth |
| Slack | 50-150 | One takeaway, no headers |
| Executive email | 200-400 | BLUF, bold conclusions |
| LinkedIn | 150-300 | Hook first, personal voice |
| Personal blog | 1200-2500 | More personality, industry framing |
