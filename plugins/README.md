# DailyOS Plugin Marketplace

Plugins for Claude Code that bridge DailyOS workspace intelligence to productive action.

## The DailyOS Loop

```
DailyOS App ──maintains──► Workspace Files
     ▲                          │
     │                          │ plugin reads
     │                          ▼
enriches                   Claude Code + Plugin
workspace                       │
     │                          │ produces
     │                          ▼
     └──────────────────── Deliverable + Loop-back

The workspace gets smarter every time the loop completes.
```

DailyOS maintains your operational memory — entity intelligence, meeting history, stakeholder context, action trails. The plugins give Claude Code full fluency in that workspace, so when you say "put together a risk report on Nielsen," Claude already knows everything DailyOS knows. No startup tax. No context development.

## Plugins

### `dailyos` — Workspace Intelligence

The core plugin. Every DailyOS user installs this.

**Commands:**

| Command | What it does |
|---------|-------------|
| `/dailyos:start` | Initialize workspace fluency, surface today's priorities |
| `/dailyos:assess` | Risk reports, health checks, deal reviews — every claim sourced |
| `/dailyos:produce` | Status updates, QBR narratives, board decks — ready to send |
| `/dailyos:compose` | Emails and messages grounded in shared relationship history |
| `/dailyos:plan` | Success plans, strategies with milestones connected to actions |
| `/dailyos:synthesize` | Cross-entity pattern detection, portfolio-level insights |
| `/dailyos:capture` | Process transcripts and notes into workspace-native artifacts |
| `/dailyos:enrich` | Fill intelligence gaps with research, update workspace |
| `/dailyos:decide` | Structured analytical decisions (SCQA, Issue Trees, Red Team) |
| `/dailyos:navigate` | Relationship navigation, political intelligence — internal only |

**Skills** (auto-activating):
- `workspace-fluency` — Directory structure, schemas, conventions
- `entity-intelligence` — Auto-loads entity context when mentioned
- `meeting-intelligence` — Meeting prep, template awareness
- `action-awareness` — Commitment tracking, overdue surfacing
- `relationship-context` — Person/stakeholder intelligence
- `political-intelligence` — Influence dynamics, power structures
- `analytical-frameworks` — SCQA, Issue Trees, WWHTBT, Red Team, Fermi
- `role-vocabulary` — Preset-aware vocabulary shaping
- `loop-back` — Write-back conventions, workspace enrichment

### `dailyos-writer` — Editorial Production

For users who need the full writer's room workflow.

**Commands:**

| Command | What it does |
|---------|-------------|
| `/dailyos-writer:write` | Full 7-phase editorial workflow |
| `/dailyos-writer:challenge` | Run challenger gate on a draft |
| `/dailyos-writer:review` | Full 6-pass review cycle |
| `/dailyos-writer:mechanical` | Quick typography and terminology checks |

**Skills** (auto-activating during workflow):
- `writer-core` — 7-phase orchestration
- `challenger` — Red-team gate (PROCEED / SHARPEN / RECONSIDER / KILL)
- `research` — Workspace-first evidence gathering
- `scrutiny` — Executive specificity review
- `mechanical-review` — Typography, terminology, anti-patterns
- `structural-review` — Logic, flow, argument coherence
- `voice-review` — Voice fidelity per content type
- `authenticity-review` — AI-tell and formula detection

## Installation

```bash
# Install the core workspace intelligence plugin
claude plugin install ./dailyos

# Optionally install the editorial production plugin
claude plugin install ./dailyos-writer
```

## Requirements

- A DailyOS workspace (directories: `Accounts/`, `Projects/`, `People/`, `_archive/`, `data/`)
- Claude Code with plugin support
- Optional: DailyOS app running (for Quill MCP sidecar access to real-time data)
