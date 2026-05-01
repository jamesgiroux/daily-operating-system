# Codex Agent Briefing — v1.4.0 Wave 5 L0 Plan Drafts

You are codex, drafting an L0 implementation plan for the v1.4.0 Abilities Runtime Spine. This is **plan-only work**. No code changes anywhere. No git operations.

## Repository

- CWD: `/Users/jamesgiroux/Documents/dailyos-repo`
- Branch: `dev` (do not switch, do not commit, do not push, do not stash)
- Waves W2, W3, W4 plans are committed. W3 implementation is in flight. W5 is the pilot abilities wave — the first two real abilities running on the new substrate.

## Your deliverable

A single file: `.docs/plans/wave-W5/<DOS-NNN>-plan.md` matching the 10-section template at `.docs/plans/v1.4.0-waves.md` §"Plan-review template" (lines ~87-133 of that file).

**Format reference (the bar):** `.docs/plans/wave-W2/DOS-259-plan.md`. Read it first. Your plan must match that quality bar — dense, ~120 lines, grounded in current-code line numbers via grep, ADR-pinned shapes, concrete file paths and test names. NOT a template fill.

## How to draft (workflow)

1. **Read the Linear ticket via the linear MCP** — get the full body, including all amendments. Quote load-bearing lines verbatim in §1.
2. **Read the wave plan section** for your agent slot in `.docs/plans/v1.4.0-waves.md` — find your "Files owned" + "Don't touch" + "Done when" + reviewer matrix slot.
3. **Read referenced ADRs** under `.docs/decisions/` — cite them in §3 Key decisions.
4. **Read the W3 and W4 L0 plans** for your upstream dependencies in `.docs/plans/wave-W3/` and `.docs/plans/wave-W4/` — these contain frozen contracts your plan must build on.
5. **Grep the legacy implementation** being migrated — understand the current code shape before describing the ability replacement. Cite file:line for every claim.
6. **Read the W2 reference plan** for tone/depth: `.docs/plans/wave-W2/DOS-259-plan.md`.
7. **Write your plan file.**

## Hard constraints

- Write **exactly one file**: `.docs/plans/wave-W5/<DOS-NNN>-plan.md`. Nothing else.
- Do NOT modify source code, ADRs, other plans, or the wave doc.
- Do NOT run `git`, do NOT commit, do NOT push, do NOT create branches.
- Do NOT touch `services/context.rs` or `intelligence/provider.rs` — W2 frozen seams.
- Do NOT invent shapes. If the ticket is silent on a structural choice, surface it as an Open Question in §10.
- If a 2-cycle revision still doesn't converge, return what you have with `STATUS: NEEDS L6` at the top.

## Plan template (write to this exact structure)

```markdown
# Implementation Plan: DOS-NNN

## Revision history
- v1 (YYYY-MM-DD) — initial L0 draft.

## 1. Contract restated
Agent's reading of the ticket in own words. Quote load-bearing lines verbatim.
Identify which 2026-04-24 amendments apply.

## 2. Approach
Files to create/modify, key data structures, the specific algorithm for non-trivial
pieces. End-state alignment: how this moves us toward the v1.4.0 end-state in the
project description, and what it forecloses.

## 3. Key decisions
Every place the ticket left a choice — the pick and the reasoning.

## 4. Security
New attack surfaces, auth/authz checks, input validation, cross-tenant exposure
paths, secrets/PII handling.

## 5. Performance
Hot paths touched, query plans, cache implications, lock contention. Expected
budget vs current baseline.

## 6. Coding standards
Services-only mutations adherence; Intelligence Loop 5-question check (CLAUDE.md);
no `Utc::now()`/`thread_rng()` direct in services or abilities; no customer data
in fixtures; clippy budget.

## 7. Integration with parallel wave-mates
Files this agent reads from / writes to that another agent owns. Migration
numbering coordination if applicable.

## 8. Failure modes + rollback
What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
universal write fence: yes/how.

## 9. Test evidence to be produced
Concrete test names + the wave merge-gate artifact + this PR's contribution to
Suites S/P/E.

## 10. Open questions
Sanity-check requests before coding.
```
