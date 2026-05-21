# The Engineering Ladder (L0–L6)

**Status:** Canonical reference (replaces "Review Ladder" naming)
**Adopted:** 2026-05-18
**See also:** `.docs/plans/v1.4.0-waves.md` § Engineering Ladder, `CLAUDE.md` § The Engineering Ladder, `docs/solutions/README.md`

The Engineering Ladder defines how work moves from intake to merge across L0–L6, with each rung naming the **Plan**, **Implement**, **Review**, and **Capture** skills that apply at that phase. Numbering is preserved (L0–L6) — load-bearing in the `commit-msg` hook, agent prompts, ADRs, Linear comments, and memory entries.

Used in chat: "L2 looks good." Used in docs/headings: "L2 (Diff) review verdict: approve."

## Skill matrix

| Rung | Phase | Plan | Implement | Review | Capture |
|---|---|---|---|---|---|
| **L0 Prep** | Plan hardening | `/plan-eng-review` (always) + `/plan-ceo-review` (product/domain) + `/plan-design-review` (UI/workflow) + `/plan-devex-review` (MCP/API/DX) + `/ce-plan` (structure + open-question surfacing) + `/ce-strategy` (only when no STRATEGY exists) | n/a | n/a | `/ce-sessions` — pull prior-session findings into the plan |
| **L0 Plan** | Plan review (gate) | n/a | n/a | `/codex challenge` + `architect-reviewer` (default domain) + `security-auditor` (when paths match Amendment 3) + `/codex consult` + `/ce-doc-review` (optional fifth opinion when scope warrants) | **K-in (mandatory)**: each L0 reviewer greps `docs/solutions/` + `.docs/decisions/` for substrate the plan claims is net new; cites hits in verdict. Unanimous required. |
| **L1 Implementation** | Implementation + self-validation | n/a | `/ce-work` (executes against plan) + `/ce-debug` (root-cause-first when stuck) + `/ce-simplify-code` (pre-PR polish) + manual editing | self-validation: tests pass, proof artifacts captured, demo evidence captured before opening the PR | `/ce-sessions` when picking up mid-task across sessions |
| **L2 Diff** | PR review (gate) | n/a | n/a | `/review` (gstack pre-landing) + `/codex review` + `code-reviewer` subagent + domain reviewer per matrix. `/ce-code-review` allowed as sanity-check second opinion, not substitute. `/ce-resolve-pr-feedback` for resolving review threads after L2 verdict. | **K-in**: domain reviewer cites matching `docs/solutions/` entries when finding repeats a known class |
| **L3 Wave** | Integrated review (gate) | n/a | n/a | `/codex challenge` against integrated wave + ADRs + `architect-reviewer` on integrated state + Suite S/P/E | **K-out (mandatory)**: at retro close, run `/ce-compound` for each class-pattern finding and each substrate-already-existed finding. Headless mode for batch. Output committed to `docs/solutions/<category>/`. |
| **L4 Surface** | User-facing QA | n/a | n/a | `/qa-only` first, `/qa` if remediation needed, `accessibility-tester` for user-facing, `/ce-test-browser` if no `/qa` config | **K-out**: capture surface bugs that recur across waves via `/ce-compound` |
| **L5 Drift** | Architecture drift | n/a | n/a | `/plan-eng-review` + `architect-reviewer` comparing integrated state to planned end-state | **K-out**: run `/ce-compound-refresh` on stale `.docs/decisions/` and `docs/solutions/` entries the drift sweep surfaces |
| **L6 Human** | Escalation | n/a | n/a | James — decision posted as Linear comment on affected ticket | **K-out**: L6 decisions captured as Linear comments (existing) + ADR if decision is architecturally durable |

## Pass rules (unchanged from waves.md)

- **L0 Plan**: unanimous approval from the panel
- **L2 Diff**: all three reviewers approve (codex review + code-reviewer + domain reviewer)
- **L3 Wave**: codex challenge + architect-reviewer approve; Suites S / P / E green
- **L4 Surface**: zero blockers
- **L5 Drift**: no drift
- **L6 Human**: James's call

**Pacing rule.** No wave starts until prior wave clears L3 *and* L5 (where applicable). No agent codes before L0 clears unanimously. 2 revision cycles on the same plan or PR without convergence ⇒ L6 escalation.

**Bounding.** L2 reviews are bounded by acceptance criteria (memory `feedback_l2_must_review_against_acceptance_criteria`).

**Path-α** — what it is, what it isn't.

The path-α maintenance channel exists for one purpose: prevent L2-cycle token loss on findings that surface during review but aren't part of the PR's acceptance contract. A finding qualifies as path-α when **all** of these are true:

1. Surfaced by an L2 reviewer during a review cycle (codex, code-reviewer, domain reviewer).
2. **Not** a literal acceptance-criterion violation from the ticket.
3. **Not** an ADR-named contract violation introduced by this PR.
4. **Not** a regression in PR-touched code (touching code makes its existing behavior fair game).
5. Theoretical hardening, dormant edge case, latent risk, or generic codebase improvement.

A qualifying finding files to the Codebase Maintenance & Production Quality project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) with priority reflecting impact + likelihood, and the substrate PR unblocks. The mechanism is a stop-loss on cycle-N+1 review attention, not a parking lot.

**What path-α is NOT for:**

- **Original-spec scope.** If a deliverable is named in the ticket, wave plan, or L0 packet, finishing it is part of L1, not a path-α candidate. "We didn't get to X" → reopen the cycle or expand the wave, never park as maintenance.
- **Scope reduction that isn't acknowledged.** Cutting scope is a deliberate decision posted to Linear as a scope amendment, not a quiet "file as path-α and move on."
- **Punting hard problems.** A finding that's hard to fix is still in-scope if it meets the criteria above. Path-α status reflects relevance to *this PR's acceptance contract*, not difficulty.
- **Tracking debt accumulation.** Memory `feedback_no_deferrals_period` applies: once scope is agreed, finish it. Path-α exists to keep the L2 loop bounded, not to enable structural deferrals.

When in doubt: if removing the finding from this PR would change whether the PR delivers what was agreed, it isn't path-α.

**Threat-topology framing (L0 reviewer scoping).** Every L0 packet declares its trust topology in §1 header: `local-to-local single-user | local-to-local multi-user | remote-to-local | remote-to-remote`. `/cso`, `security-auditor`, and codex challenge scope their threat model to that topology — they do **not** enforce multi-actor gates on single-actor surfaces. DailyOS's WP block → loopback Tauri runtime is local-to-local single-user; most multi-actor gates (confirmation tokens, principal differentiation, cross-actor poisoning, scope-gated redaction of data the user already has filesystem access to) collapse to non-issues. What still applies regardless of topology: compile bugs, crate-boundary rules, slug/path validation for data hygiene, indirect prompt injection from untrusted document content (ADR-0093), and sensitivity redaction in logs/screenshots (ADR-0108). Reviewers who flag multi-actor gates on a single-actor surface waste a cycle; reviewer prompts must cite the topology and constrain accordingly. See memory entry on local-to-local security overreach for the full pattern.

## The Knowledge Channel (K)

`K` is a continuous feedback channel parallel to L0–L6, not a rung.

```
                     Knowledge channel (K)
        ┌──────────────────────────────────────────┐
        │  docs/solutions/    .docs/decisions/     │
        │  (CE writes)        (ADRs, manual)       │
        └──────────────────────────────────────────┘
              ▲ (K-out)              │ (K-in)
              │                       ▼
   L3 retro ──┘              ┌─── L0 reviewer must grep
   L5 drift ──┘              │    before approving
   L4 surface bugs ──┘       └─── L1 author may grep
                                  before drafting
```

### K-in (consume) — substrate-grep obligation

**At L0 (mandatory).** Every L0 reviewer prompt includes this obligation:

> Before scoring this plan, grep `docs/solutions/` and `.docs/decisions/` for substrate this plan claims to be net new. Cite any hits in your verdict. Reinvented documented substrate = **BLOCKED**, cite the path.

**At L1 (advisory).** Implementing agents should grep these directories for the entity / module / pattern being touched before authoring. Memory `feedback_check_substrate_before_authoring_primitives` codifies this.

**At L2 (advisory).** Domain reviewers cite matching `docs/solutions/` entries when a finding repeats a documented class.

### K-out (capture) — three tiers, defense-in-depth

K-out runs `/ce-compound` to write findings to `docs/solutions/<category>/<slug>-<date>.md`. Three layers ensure it actually happens:

**Tier 1 — Autonomous (default).** Claude runs `/ce-compound mode:headless` for each qualifying finding as part of the L3 retro work. Same set-and-forget authorization as the wave protocol covers for impl→L1→commit→L2→fix→retro→tag. The retro is already the parked context at wave-end; K-out is the closing step.

**Tier 2 — Reminder hook (safety net).** `.claude/hooks/k-out-reminder.sh` (a Claude Code Stop hook, not a git hook) scans the last assistant turn for trigger phrases (`class-pattern finding`, `same-shape twice`, `substrate already existed`, `reinvented`, `L3 retro complete`) and prints a one-line nudge when matched.

**Tier 3 — Retro template gate (blocking).** The wave `retro.md` checklist includes:

```markdown
- [ ] K-out runs complete (paths of /ce-compound docs created):
      - docs/solutions/<category>/<slug>-YYYY-MM-DD.md
      - ...
```

This blocks retro-close until the autonomous run's outputs are documented.

### K-out cadence

**Default: every L3 retro.** At wave end, `/ce-compound` runs for:

- Class-pattern findings (memory `feedback_zoom_out_for_class_pattern_in_l2_loop`)
- Substrate-already-existed findings (memory `feedback_check_substrate_before_authoring_primitives`)
- Cross-wave drift findings surfaced by L5

**Also fires at:** L5 drift sweep close (via `/ce-compound-refresh`), L4 surface-bug recurrence (2+ waves), manual invocation by James or any agent.

## Reviewer matrix (carries forward from waves.md)

The third reviewer slot at L0 and L2 is a domain specialist keyed to the agent's risk profile. Slots stack:

| Agent profile | Domain reviewer |
|---|---|
| Substrate / schema | `architect-reviewer` |
| New SQL write path or Tauri command | `security-auditor` |
| Migration / projection / hot-path | `performance-engineer` |
| User-facing surface | `accessibility-tester` |
| Test infrastructure | `qa-expert` |

L0 panel becomes a quartet when `security-auditor` triggers per Amendment 3.

## Test suites (carries forward from waves.md)

- **Suite S — Security**: SQL injection, cross-tenant exposure, PII/secrets in logs, immutability allowlist bypass. Owner: `penetration-tester` + `security-auditor`.
- **Suite P — Performance**: budgets locked at end of W1; no regression in subsequent waves. Owner: `performance-engineer` + benchmarks.
- **Suite E — Edge cases**: property tests, fuzz on validators, bundle coverage. Owner: `qa-expert` + harness.

Suites feed into L3.

## L6 escalation policy (carries forward from v1-lite.md §6)

**MUST escalate**: cycle cap exceeded, reviewer flags "needs human judgment" with a specific structural question, L5 drift without remediation path, suite gate failure, reviewer infrastructure failure, net-new scope expansion, contract amendment.

**MUST NOT escalate**: routine review iteration, single flaky codex output, lint/type/test failures, reviewer dissent (investigate, don't break ties), same finding class repeats (sweep instead), scope discovery within DoD, equivalent-implementation choice (pick more complete), "I don't know" without a specific question, deferral requests, half-finished implementations, asking for context already in codebase/memory/plan, tool choice when convention exists.

## How this doc relates to others

- **`CLAUDE.md`** keeps a ~5-line stub section pointing here for the matrix.
- **`.docs/plans/v1.4.0-waves.md`** keeps the wave protocol and reviewer matrix; references this doc for the skill assignments per rung.
- **`docs/solutions/README.md`** describes the knowledge store structure and K-in / K-out flow from the consumer side.
- **`.docs/plans/orchestration/v1-lite.md`** describes the async orchestration design (cloud routines, claudebot DMs, daily digest) — **none of which has shipped**. Treat as design archive until parts ship. Local workflow (this doc) is the canonical operating model.
