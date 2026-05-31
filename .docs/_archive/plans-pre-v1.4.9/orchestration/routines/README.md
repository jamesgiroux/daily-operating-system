<!-- protocol-doc: orchestration-routines -->

# Orchestration routines

Version-controlled prompts for the Anthropic Routines that drive the L0–L6 wave protocol asynchronously. Each `.md` file in this directory is the prompt body for one routine.

## How routines work

Anthropic Routines run Claude Code on Anthropic's cloud against a fresh clone of the repo, on a schedule or webhook. Each routine has:

- A **prompt** (the markdown file in this directory) — what Claude does
- A **trigger** — cron schedule, GitHub webhook event, or API fire
- **MCP connectors** — Linear, Slack (configured at the routine level)

The routine prompt is stored on Anthropic's side when you create the routine; this directory is the version-controlled source. **Always copy from here into the routine UI** so prompt changes go through L0 review.

## Setting up a routine

1. From a Claude Code session: `/schedule <prompt>` or visit `claude.ai/code/routines`
2. Paste the prompt body from this directory
3. Configure the trigger (cron/webhook/API)
4. Enable required MCP connectors (Linear, Slack)
5. Test by firing manually before enabling auto-fire

## Routine inventory

| File | Phase | Trigger | Purpose |
|---|---|---|---|
| `linear-plan-drafter.md` | 4 | cron every 30 min, work hours | Picks up Linear tickets in `Ready for plan`, atomically claims one, drafts a plan as a structured Linear comment, transitions to `Plan ready for L0` |
| `l0-reviewer.md` | 4 | Linear webhook on `Plan ready for L0` status | Runs the L0 panel (codex challenge + architect-reviewer + domain reviewer + codex consult), posts verdicts as Linear comments, transitions to `Approved for code` on unanimous |
| `l3-wave-adversarial.md` | 6 | GitHub webhook on last `wave-WN` PR merge | Post-merge integrated wave review: codex challenge + architect-reviewer + Suite S/P/E. Writes proof bundle. |
| `l5-drift-check.md` | 6 | GitHub webhook after W3 and W5 wave-complete | Compares integrated state to planned end-state; `/plan-eng-review` + architect-reviewer |
| `daily-digest.md` | 7 | cron weekday 8am | Queries Linear for recent activity, posts digest to `#dailyos-escalations` |
| `cross-wave-reflection.md` | 7 | cron weekly | Looks across waves + codebase, surfaces patterns as Linear discovery proposals |

## Common patterns

**Linear-as-canonical-record:** every routine that produces output posts to a Linear comment on the relevant ticket. No parallel jsonl/log files. Ephemeral state (caches, retry counters) lives in routine memory only.

**Async-message format:** when a routine posts an L6 escalation to Slack or surfaces something to James, follow `feedback_async_messages_zero_context_with_recommendation`:
1. Catch-up context (3–5 plain-language bullets, 9th-grade reading level)
2. The question or finding (one sentence)
3. Options with tradeoffs (≤4-word labels)
4. A **Recommended** option with reasoning citing specific evidence
5. Escape hatches (`Tell me more`, `Direction call`)

**Cycle limits + L6 triggers:** every routine adheres to Amendment 1 conventions. Convergence triggers (same finding class twice, unresolved high/critical after retry, infrastructure failure) escalate L6. Cycle hard caps (L0=7, L2=7) escalate independently.

**Codex outage handling (Amendment 2):** retry up to 3× with backoff (1/3/5 min), escalate L6 on `reviewer_infrastructure_failure` after 3 retries. Never substitute another reviewer into the codex slot.

**Idempotency:** every routine is safe to re-fire. Use Linear status + custom field compare-and-set to claim work atomically. Re-firing a routine that already posted should detect prior output and exit cleanly.

## Editing a routine prompt

Routine prompt edits trigger `security-auditor` in L2 (per `matrix.yml` → `.docs/plans/orchestration/**`). Treat them as security-relevant changes.
