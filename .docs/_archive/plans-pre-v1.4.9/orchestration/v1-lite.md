# Orchestration v1-lite

**Status:** Active design — replaces the 7-plan roadmap and the heavy waves-amendments thread
**Owner:** James Giroux
**Date:** 2026-05-07

## Goal

Move the L0–L6 wave protocol from synchronous (James monitoring waves doc, watching cycles tick) to asynchronous (cloud routines drive; James intervenes only on L6 escalations and direction calls). DoD per James's original brief:

A process that proactively reviews issues, writes excellent plans, writes code, reviews that code for coherence/DoD/acceptance/unintended-consequences/quality/elegance, QA where possible, security, performance, cleanup + design-system hooks, with periodic reflection that surfaces patterns ("we keep solving X nine ways") through the same intake.

## Threat model (explicit)

Single developer, his laptop, his repos, his Slack workspace, his Linear. If Slack/GitHub/Linear/laptop are compromised, the orchestration is the least of his problems — that's the same boundary that protects everything else James does. Don't engineer past it.

## Topology

`origin` has no GitHub Actions. `public` does. Cloud routines push to `origin`. A local mirror-gate scans diff + commit message against `.claude/pii-blocklist.txt` before mirroring to `public`. The pre-commit gate at `.claude/hooks/pre-commit-gate.sh` covers James's terminal commits.

## Flow

```
Linear ticket  ─┐
                ├─▶  L0 panel  ─▶  Approved label  ─▶  Agent codes  ─▶  PR on public
Cross-wave      │   (codex challenge                                   │
reflection      │    + architect                                       ▼
proposals       │    + security-auditor                              L2 GitHub Action
(emergent,      │      where security-relevant                       (codex review
not prescribed) │    + codex consult)                                 + code-reviewer
                │                                                     + domain reviewer)
                │                                                       │
                │                                                       ▼
                │                                                     Merge to dev
                │                                                       │
                ▼                                                       ▼
              L6 escalation via                                       Wave gates
              claudebot DM                                            L3 (codex + architect + Suite S/P/E)
              with buttons                                            L4 (qa, a11y) [W4+]
                                                                      L5 (drift) [after W3, W5]
```

The wave gates (L3/L4/L5) are already defined in `v1.4.0-waves.md`. They run as routines on wave-merge events; nothing new needed at the protocol level.

## Architecture

### 1. Intake — Linear
- James creates tickets normally
- One or two reflective routines look across waves and the codebase, surface cross-wave patterns to Linear as `discovery-proposal` tickets. The "nine ways" example was *illustrative* — these routines find whatever's actually interesting; L0 decides which proposals turn into real plans.

### 2. L0 — turns intake into reasonably defined plans
- **Plan lives on the Linear ticket itself**, not as a `.docs/plans/wave-WN/DOS-NNN-plan.md` file. Linear is already canonical (per CLAUDE.md); putting the plan on the ticket eliminates the git-vs-Linear drift problem and the SHA-pinning ceremony. `updatedAt` is the optimistic-concurrency token.
- `linear-plan-drafter` routine: every 30 min, picks up tickets in `Ready for plan`, drafts the plan into a structured section on the ticket (description or a designated `Plan` field/comment), atomically transitions status to `Planning in progress` to claim it
- `l0-reviewer` routine: reads the plan from the ticket, runs the L0 panel
  - `/codex challenge`
  - `architect-reviewer` (default domain reviewer)
  - `security-auditor` when the changeset matches security-relevant paths (see lean amendment)
  - `/codex consult`
- Each reviewer posts its verdict + findings as a Linear comment on the ticket. Iterations happen by editing the plan section; reviewers see the diff via Linear's history.
- Unanimous → applies `Approved for code` Linear label
- Disagreement / cycle limit / outage → claudebot DM to James with buttons

### 3. Implementation
- `impl-driver` routine: fires on `Approved for code` label, reads the plan from the Linear ticket (the frozen contract), spawns headless `claude -p` in a worktree, codes against the plan, opens PR on `public` via `gh` CLI. The PR description links back to the Linear ticket.

### 4. L2 — per-PR (two moments, same review level)

L2 runs in two places: **in-cycle** during development (fast feedback while context is fresh) and **at PR-open** as the formal merge gate. Same review level, different moments. Re-running at PR-open is a safety re-validation that catches drift between in-cycle review and final pushed state, and catches anything cloud routines bypass since they don't run in-cycle L2 locally.

**In-cycle L2** — developer practice, no automation gate. The implementing agent (or James) invokes `/codex review` + `code-reviewer` + domain reviewer locally during the cycle. Speeds development; not blocking.

**PR-open L2** — GitHub Action on `public`, triggered by `pull_request: [opened, synchronize, reopened]` against `dev` and `trunk`. Runs four jobs in parallel, each setting its own status check:
- `/codex review` — adversarial diff read
- `code-reviewer` subagent — DoD adherence, acceptance-criteria match, unintended consequences, SRP/DRY, elegance
- Domain reviewer per matrix — `performance-engineer` for perf paths, `security-auditor` for security/trust-boundary paths, `accessibility-tester` for user-facing, etc.
- `blocklist-scan` — re-runs the Phase 1 scanner against the PR commit range; catches cloud-routine commits that bypass local hooks

Branch protection blocks merge until all status checks are green. Wave-driver earns merge authority after Phase 2+3 ship a wave end-to-end with James watching.

### 5. Wave gates — existing protocol
- **L3** wave adversarial after all wave PRs merge: `/codex challenge` + `architect-reviewer` + Suite S (security) / Suite P (performance) / Suite E (edge cases)
- **L4** surface QA (W4+): `/qa-only` first, `/qa` if remediation needed, `accessibility-tester` for user-facing
- **L5** drift check (after W3, W5): `/plan-eng-review` + `architect-reviewer` comparing integrated state to planned end-state

These already exist. The orchestration just runs them as scheduled routines on the right events.

### 6. L6 — claudebot DM with buttons

- Slack DM to James for any L6 trigger
- Block Kit interactive buttons for the decision options
- Slack signing verifies the request came from Slack; James's Slack account = authority. Same boundary as anything else James does in Slack — accepted.
- Free-text in DM = informal context. Buttons = authority.

**Message content requirements.** L6 is asynchronous — James is context-switching and may be hours/days away from the work when the DM lands. Every message catches him up from zero before asking for a decision. Concretely:

1. **Catch-up context.** Explain in plain language what this work is, what's been happening, and why the system is stuck. Write as if to a 9th grader with no prior context on the codebase or the wave. No jargon without a plain-English gloss. ~3–5 short bullets is the target — enough to orient, not enough to skim past.
2. **The question.** State the actual decision in one sentence.
3. **Options.** For each option:
   - A short button label (≤4 words)
   - 1–2 sentences of what choosing it does
   - The tradeoff in plain language
4. **Recommendation.** One option marked **Recommended**, with 1–2 sentences of reasoning that cites specific evidence — which reviewer flagged what, what the agent tried, what failed. A real opinion, not "all options are roughly equal."
5. **Escape hatches.** `Tell me more` button (collapses deeper context — full reviewer transcripts, plan diff, etc.) and `Direction call` button (kicks to a free-text thread for when none of the options fit).

**Payload schema** (signed by Slack on submit):
```json
{
  "escalation_id": "esc_01HXXX...",
  "linear_issue": "DOS-309",
  "target_sha": "abc1234",
  "options": ["approve_a", "approve_b", "hold", "direction_call"],
  "expires_at": "2026-05-07T18:00:00Z",
  "context_url": "https://linear.app/..."
}
```

When James clicks a button, claudebot posts the decision as a comment on the Linear ticket the escalation was about, with `(escalation_id, target_sha, choice, decided_at, slack_user_id)`. Linear is the durable record; Slack is just the surface where the click happens.

#### L6 escalation policy — what goes to James, what doesn't

L6 is expensive: every escalation is a context-switch for James. The orchestration's job is to handle as much as possible downstream so L6 is reserved for genuine human-judgment calls.

**MUST escalate** (objective triggers — agents do not get to skip these):

1. Cycle cap exceeded (per Amendment 1)
2. Reviewer flags "needs human judgment" with a **specific structural question** (not "I'm stuck")
3. L5 drift detected without remediation path
4. Suite gate failure requiring regression acceptance or budget relaxation
5. Reviewer infrastructure failure (codex outage after 3 retries; per Amendment 2)
6. **Net new scope expansion** — work genuinely outside the agreed DoD/outcomes. *Not* "more tickets to complete what's already in scope" — that's normal scope discovery, see below.
7. Contract amendment — plan needs a fundamental change, not just a revision

**MUST NOT escalate** (handle downstream):

1. Routine review iteration within cycle cap — read findings, fix, resubmit
2. Single flaky codex output — retry per Amendment 2
3. Lint, type-check, test failures — fix them
4. **Reviewer dissent (1/3 or 2/3 against).** When one reviewer flags high/critical and others approve, the dissenter is often right. Investigate, validate the finding, fix if valid, re-review. Don't treat dissent as a tie to break — treat it as signal that one reviewer caught something the others missed. With 3 reviewers, post-fix alignment will land 2/3 or 3/3. (Memory: `feedback_reviewer_dissent_is_signal`.)
5. **Same finding class repeats twice.** Don't escalate — do a class-level sweep of the codebase, find all instances, plan a full patch. Attach new Linear tickets to the same wave if the sweep produces work. (Memory: `feedback_systemic_look_for_recurring_issue_classes`.)
6. **Scope discovery within DoD.** If completing the agreed outcome needs additional tickets, create them in Linear, attach to the same wave, and continue. The goal is true completion and delivery — not adherence to a single ticket's original (possibly poorly-scoped) prose. (Memory: `feedback_think_in_dod_and_outcomes_not_ticket_scope`.)
7. Choice between equivalent implementations — pick the more complete option that prevents tech debt; less code wins only when both *equally* complete the ask. (See agent guidance below.)
8. "I don't know what to do" without a specific question — explore until you do, or reformulate as #2 in MUST escalate with a real structural question
9. **Deferral requests.** "Park for v1.4.x" is not a valid escape valve. Once scope is agreed, finish it. (Memory: `feedback_no_deferrals_period`.)
10. Half-finished implementations — finish what's scoped, OR escalate as net-new-scope-expansion (#6 in MUST escalate). One or the other, not both.
11. Asking for context already in the codebase, memory, or the plan — read it first.
12. Tool/library choice when the codebase has a convention — match the existing pattern unless the pattern is the bug being fixed.

**Agent guidance — principles to apply before reaching for L6:**

- **Think in definition of done and outcomes, not single-ticket scope.** Tickets may be poorly scoped at intake; deliver the agreed outcome. If completion requires slicing into multiple Linear tickets attached to the wave, do that. (Memory: `feedback_think_in_dod_and_outcomes_not_ticket_scope`.)
- **No deferrals.** Once scope is agreed, finish it. (Memory: `feedback_no_deferrals_period`.)
- **Pick the more complete option that prevents tech debt.** When two implementations work, default to the more complete one. Less code wins only when both *equally* complete the ask. If genuinely unsure which is more complete, escalate with a specific structural question. (Memory: `feedback_pick_more_complete_option_over_simpler`.)
- **Match codebase conventions** unless the pattern is the bug being fixed. (Memory: `feedback_ground_first_drafts_in_real_codebase`.)
- **Treat reviewer dissent as signal.** A 1/3 reviewer who flagged high/critical when others approved is usually right about something. Investigate before assuming majority is correct. (Memory: `feedback_reviewer_dissent_is_signal`.)
- **No fixes without root cause.** Diagnose before patching.
- **Verify before claiming done.** Build the harness; don't self-report. (Memory: `feedback_verify_before_claiming_fidelity`.)
- **2 failed fixes on the same surface = step back, get independent diagnosis** (spawn `code-reviewer` or `architect-reviewer` for a fresh read). If the mechanism still feels wrong after that, *that* is a "needs human judgment" L6 with a real structural question. (Memory: `feedback_step_back_after_repeated_patches`.)
- **Same-shape findings recurring = class-level sweep, not a third one-off patch.** Audit the codebase for all instances; plan a full patch; create Linear tickets attached to the wave. (Memory: `feedback_systemic_look_for_recurring_issue_classes`.)

If after applying all of the above the agent still can't make a sane choice, the L6 message must state specifically: which guidance was applied, what was tried, and what the open structural question is. An L6 message that's just "I'm stuck" without that catch-up + specific question is rejected by the orchestration and reposted to the agent for self-resolution.

### 7. Audit + visibility — Linear is the canonical record

**Everything that needs to be auditable lives as a Linear comment** on the relevant ticket. No parallel jsonl files, no separate audit infrastructure to maintain.

- L0 / L2 / L3 / L5 **reviewer verdicts** — comments on the ticket (already specified in §2 / §4)
- **L6 decisions** — comment on the ticket when James clicks a button (per §6)
- **Mirror-gate holds** — comment on the ticket associated with the blocked commit (resolved from PR title or commit message); plus a Slack DM to James for the alert
- **Routine failures and anomalies** — comment on the relevant ticket (or on a designated ops-tracking ticket if no ticket is in scope) plus a Slack DM
- **Wave proof bundles, retros, ADRs** — these stay in git per existing wave protocol; they're wave-scoped not ticket-scoped

`daily-digest` routine queries Linear for recent ticket activity (escalations pending, mirror-gate holds, anomalies, ships) and posts a digest to `#dailyos-escalations` weekday mornings. `#dailyos` gets ship announcements. **Slack is a visibility surface; Linear is the durable record.**

If a small local state file helps a routine (e.g., mirror-gate caches "last mirrored SHA per ref" for performance), that's fine as ephemeral local state — not as the audit record.

## Wave protocol amendments (the lean version)

Three small rules added to `v1.4.0-waves.md`. Full text in `v1.4.0-waves-amendments.md`:

1. **Cycle cap stays at 2** for v1.4.0 (existing line-96 rule). If it bites in practice, James escalates the loosening as a normal L6 ruling per the existing tuning protocol. No phased activation contract.
2. **Codex outage**: 3 retries with backoff, then escalate L6 with `reviewer_infrastructure_failure`. Never substitute a different reviewer into the codex slot.
3. **`security-auditor` joins the L0 panel by default** when the changeset touches security-relevant paths (`services/`, `abilities/`, `intelligence/`, schema, prompt templates, MCP configs, gate workflows, `.docs/plans/v1.4.0-waves*.md`, `.docs/plans/orchestration/`).

That's the whole amendment.

## Build order

Each phase shippable independently. Stop at any point and you have working pieces.

| Phase | Ships | Effort |
|---|---|---|
| 1 | Mirror-gate (`mirror-gate.sh` with refspec allowlist + blocklist scan) + `commit-msg` hook | Hours | **Done** — running locally (14/14 unit, 6/6 e2e) |
| 2 | GitHub Action on `public` for L2 (one workflow file: codex review + code-reviewer + domain reviewer) | A day | **Code complete (path α)** — awaiting GitHub UI setup + test PR (task #14) |
| 3 | Claudebot DM with Slack interactive blocks + Linear comment writer | A day | Not started — outbound DMs work today via Slack MCP |
| 4 | `linear-plan-drafter` + `l0-reviewer` routines (cloud, via `/schedule`) | A few days | **Routine prompts drafted** at `.docs/plans/orchestration/routines/{linear-plan-drafter,l0-reviewer}.md` — pending L0 review then Anthropic Routines deployment |
| 5 | `impl-driver` routine | A day | Not started — gated on Phase 5 codex residuals (task #17 — identity-provenance hardening) |
| 6 | Wave gate routines (`l3-wave-adversarial`, `l5-drift-check`, `l4-surface-qa`) | A few days | **L3 + L5 prompts drafted** at `.docs/plans/orchestration/routines/` — L4 + deployment pending |
| 7 | `daily-digest` + cross-wave reflection routine | A day or two | **Both prompts drafted** at `.docs/plans/orchestration/routines/` — pending L0 review then deployment |

Phase 1 is the safety floor and goes first. Phases 2–5 are the orchestration spine. Phases 6–7 are visibility and continuous reflection.

## What we're explicitly NOT building

- Parallel audit-trail infrastructure (jsonl files, hash-chained logs, signed decision records). Linear is the audit trail; no second source of truth to maintain.
- Codex transcript-token verifiers (the L2 GitHub Action already gates on the codex output)
- Three-identity activation contracts (one user, no separation of duties needed)
- Magic-comment content-based protocol-doc detection (path-prefix matching covers it; if a path slips through, retro adds it)
- Phased activation gates (everything ships when its phase ships)
- WebAuthn / Sigstore Rekor (Linear comment + Slack DM is fine for one user)

If the orchestration goes wrong: revert the commit. That's the failure mode.

## Open questions

- **Approval surface preference** — Linear status change vs Slack DM buttons for L0 plan approval. Both work; pick based on UX (Slack mobile-friendly, Linear in-context).
- **Cross-wave reflection cadence** — weekly? after each wave? per-Linear-area?
- **Mirror-gate liveness off-hours** — accept origin lag while laptop sleeps; public Action fires when machine wakes. Or run the gate on a tiny always-on machine if lag is annoying.