# docs/solutions/ — DailyOS Knowledge Store

Documented problem→fix entries and durable engineering learnings, organized by category with YAML frontmatter for grep + cross-reference. **Companion** to `.docs/decisions/` (ADRs).

This directory is one half of the **Knowledge Channel (K)** in the Engineering Ladder. See `.docs/plans/engineering-ladder.md` for the full ladder.

## When to consult this directory

**Always at L0 (Plan review).** Before approving a plan, L0 reviewers grep `docs/solutions/` and `.docs/decisions/` for prior solutions and architectural decisions that match the work. Reinventing documented substrate = BLOCKED finding, cite the path.

**Always at L1 (authoring).** Before writing new substrate, agents grep for the entity / module / pattern being touched. If a prior solution exists, consume it — don't reinvent.

**Always at L2 (Diff review).** Domain reviewers cite matching entries when a finding repeats a documented class.

## When entries are written

**K-out (capture) fires at:**

- **End of every L3 wave retro** — autonomous `/ce-compound` runs for each class-pattern finding, each "substrate-already-existed" finding, and each cross-wave drift signal surfaced by L5.
- **End of L5 drift sweep** — `/ce-compound-refresh` on stale entries.
- **L4 surface-bug recurrence** — when the same surface bug surfaces in 2+ waves.
- **Manual** — James or any agent running `/ce-compound` on a solved problem worth indexing.

`/ce-compound` runs interactively (asks Full vs Lightweight, session-history opt-in) or headless (`/ce-compound mode:headless`) for batch capture during retros.

## Category structure

**Bug track** — problem→fix entries:

- `build-errors/` — compile, lint, build pipeline failures
- `test-failures/` — flaky tests, fixture issues, infra failures
- `runtime-errors/` — panics, exceptions, crashes
- `performance-issues/` — slow queries, hot paths, regressions
- `database-issues/` — schema, migrations, queries, locking
- `security-issues/` — vulnerabilities, trust boundary violations
- `ui-bugs/` — rendering, interaction, accessibility surface defects
- `integration-issues/` — cross-component, MCP, abilities, surface wiring
- `logic-errors/` — incorrect behavior with no obvious crash signal

**Knowledge track** — durable learnings:

- `architecture-patterns/` — architectural decisions (agent/skill/pipeline/workflow shape)
- `design-patterns/` — reusable non-architectural design approaches
- `tooling-decisions/` — language, library, or tool choices with durable rationale
- `conventions/` — team-agreed practices captured to survive turnover
- `workflow-issues/` — workflow friction and the fix
- `developer-experience/` — DX learnings (tooling pain, ergonomics)
- `documentation-gaps/` — surfaces where docs failed agents
- `best-practices/` — fallback only when no narrower category fits

## YAML frontmatter schema

Every entry starts with:

```yaml
---
title: <one-line problem statement>
problem_type: <bug-track or knowledge-track category, e.g. database_issue>
track: <bug | knowledge>
module: <affected component / path>
tags: [<keyword>, <keyword>, <keyword>]
date: <YYYY-MM-DD>
last_updated: <YYYY-MM-DD>   # added when an entry is refreshed
related_adr: <ADR-NNNN if applicable, omit otherwise>
related_linear: <DOS-NNN if applicable, omit otherwise>
---
```

Followed by either bug-track sections (Problem / Symptoms / What Didn't Work / Solution / Why This Works / Prevention) or knowledge-track sections (Context / Guidance / Why This Matters / When to Apply / Examples). Section structure managed by `/ce-compound` from `assets/resolution-template.md` in the plugin.

## Companion surfaces

- **`.docs/decisions/`** — Architecture Decision Records (ADRs). Use for durable architectural calls; `docs/solutions/` for tactical fixes and learnings.
- **`MEMORY.md`** — Claude's auto-memory. Personal-shaped (James + Claude). Use for ways-of-working preferences; `docs/solutions/` for repo-shaped knowledge.
- **Linear** — Canonical ticket / decision audit trail. Use for in-flight work; `docs/solutions/` for "what we already learned."
- **`.docs/plans/`** — Wave plans, version planning, proposals. Use for forward-looking work.

## How this directory compounds

The first time we solve a problem takes research. Document it, and the next agent (or future you) finds it via grep in seconds. **Each unit of engineering work should make subsequent units of work easier — not harder.**

The compounding only works if K-out runs every retro and K-in (the substrate-grep obligation at L0 / L1 / L2) is enforced. Without both loops closed, this becomes a doc graveyard.
