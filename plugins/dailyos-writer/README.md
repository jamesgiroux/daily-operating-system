# dailyos-writer — Editorial Production Plugin

Writer's room quality control for Claude Code. Seven-phase workflow from ideation through polish, with specialized voices, internal review cycles, and challenger gates. Creates thought leadership, strategic documents, status reports, and customer communications.

## What Makes This Different

The writer plugin doesn't just generate text. It runs a full editorial workflow:

1. **Discovery** — Check for existing drafts, avoid duplication
2. **Ideation** — Thesis development with challenger gate
3. **Research** — Workspace-first evidence (entity intel, meeting archives, stakeholder quotes), then web
4. **Structure** — Template selection, outline with internal review
5. **Drafting** — Section-by-section with voice profile
6. **Review** — Six-pass internal quality control
7. **Polish** — Production-ready formatting

The review cycle runs six specialized passes before the human sees output:

```
mechanical → structural → voice → authenticity → scrutiny → challenger
```

## Commands

| Command | What it does |
|---------|-------------|
| `write` | Start a new writing project — full 7-phase workflow |
| `challenge` | Run challenger gate on a draft (PROCEED / SHARPEN / RECONSIDER / KILL) |
| `review` | Trigger all 6 review passes in sequence |
| `mechanical` | Quick typography + terminology + anti-pattern checks only |

## Content Types

| Type | Voice Profile | Template Examples |
|------|--------------|-------------------|
| Thought Leadership | thought-leadership | hook-problem-reframe, counterintuitive-claim, story-driven |
| Strategic Update | strategic | bluf-standard, scqa, pyramid-principle |
| Status Report | status-report | weekly-impact, monthly-rollup, quarterly-review |
| Vision Document | strategic | strategy-memo, roadmap-narrative, investment-case |
| Customer Communication | customer | qbr-narrative, renewal-case, expansion-proposal |
| Narrative | narrative | documentary-arc, explainer, future-vision |

## DailyOS Enhancement

When running in a DailyOS workspace, the writer gains workspace-first evidence gathering:

- **Research phase** reads entity intelligence, meeting archives, and stakeholder quotes before web search
- **Customer communications** ground claims in actual dashboard metrics and meeting history
- **Scrutiny review** checks `dashboard.json` for available metrics when flagging unquantified impact
- **Evidence inventory** includes workspace sources with file paths and dates

## Resources

- `voices/` — 6 YAML voice profiles
- `templates/` — 28+ templates across 7 categories
- `shared/` — Mechanics, terminology, anti-patterns, distribution rules
- `scripts/` — Typography linting and pattern detection
