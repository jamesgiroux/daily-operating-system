# dailyos-writer — Editorial Production Plugin

Writer's room quality control for Claude Code. Eight-phase workflow from ideation through review to capture, with specialized voices, internal review cycles, challenger gates, and a current-generation AI-tell detection layer. Creates thought leadership, strategic documents, status reports, and customer communications that read like a person wrote them.

## What Makes This Different

The writer plugin doesn't just generate text. It runs a full editorial workflow:

1. **Discovery** — Check for existing drafts, avoid duplication
2. **Ideation** — Thesis development with challenger gate
3. **Research** — Workspace-first evidence (entity intel, meeting archives, stakeholder quotes), then web
4. **Structure** — Template selection, outline with internal review
5. **Drafting** — Section-by-section with voice profile
6. **Review** — Six-pass internal quality control
7. **Polish** — Production-ready formatting
8. **Capture** — Log voice corrections so the next draft is better

The review cycle runs six specialized passes before the human sees output:

```
mechanical → structural → voice → authenticity → scrutiny → challenger
```

## Voice system & AI-tell defense

The thing that makes output read as human, not machine:

- **Aphorism mode vs. narrator mode** — the authenticity pass catches the current generation of AI tells (the "X is the gate" abstract-noun equation, "not X, but Y" antithesis, drumroll reveals, inflated diction) and rewrites them into something a person would actually say. Taxonomy in `skills/shared/AI-TELLS.md`, grounded in the [Wikipedia "Signs of AI writing"](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing) catalog.
- **Optional author layer** — copy `skills/voices/author.example.yaml` to `author.yaml` to give every piece a consistent personal voice on top of the content-type profiles.
- **Learning loop** — `learn` captures a voice correction into `skills/voices/LEARNINGS.md` (ships empty); the voice and authenticity passes read it on every run, so the writer gets more your-shaped over time.

## Commands

| Command | What it does |
|---------|-------------|
| `write` | Start a new writing project — full 8-phase workflow |
| `challenge` | Run challenger gate on a draft (PROCEED / SHARPEN / RECONSIDER / KILL) |
| `review` | Trigger all 6 review passes in sequence |
| `mechanical` | Quick typography + terminology + anti-pattern checks only |
| `learn` | Capture a voice correction into the learning loop |

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
- **Scrutiny review** checks DailyOS runtime/MCP for available metrics when flagging unquantified impact
- **Evidence inventory** includes workspace sources with file paths and dates

## Resources

- `skills/voices/` — 5 content-type voice profiles + `author.example.yaml` (personal layer template) + `LEARNINGS.md` (the learning log)
- `skills/templates/` — 28+ templates across 7 categories
- `skills/shared/` — `AI-TELLS.md` (the AI-tell taxonomy), mechanics, terminology, anti-patterns, distribution rules
- `skills/references/` — Phase detail, voice-system guide, framework guidance (loaded on demand)
- `skills/scripts/` — Typography linting and pattern detection
