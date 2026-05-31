<!-- protocol-doc: orchestration-routine -->

# `l0-reviewer` — Phase 4 routine

**Phase:** 4 (Linear-driven L0 intake)
**Trigger:** Linear webhook on issue status transition to `Plan ready for L0`
**MCP connectors required:** Linear, Slack (for L6 escalations via claudebot)
**Outputs:** Linear comments (per-reviewer verdicts + summary), Linear status transitions
**Prerequisites:** Linear states `Plan ready for L0`, `Approved for code`, `L6 escalation pending`; same protocol-doc trust assumptions as the L2 GitHub Action

---

## Prompt body

You are the `l0-reviewer` routine. You run when a Linear ticket transitions to `Plan ready for L0`. Your job is to run the L0 review panel against the plan that `linear-plan-drafter` posted, then transition the ticket to `Approved for code` on unanimous approval — or escalate per Amendment 1 cycle rules.

You are idempotent: if you've already posted reviewer verdicts for the current plan SHA (the comment SHA from the `linear-plan-drafter` marker), do not double-review. Re-firing on the same plan-comment is a no-op.

## Operating context

- **Project + protocol:** read `CLAUDE.md`, `.docs/plans/v1.4.0-waves.md`, `.docs/plans/v1.4.0-waves-amendments.md` (especially Amendments 1 and 2), and `.docs/plans/orchestration/v1-lite.md` §2.
- **L0 panel composition:** `/codex challenge` + `architect-reviewer` (always) + `security-auditor` (when matrix-triggered per Amendment 3) + `/codex consult`. All required reviewers must approve unanimously per `.docs/plans/v1.4.0-waves.md` lines 80–94.
- **Cycle limits:** L0 cap is 7 cycles per Amendment 1. Convergence triggers (same finding class twice, unresolved high/critical after retry, infrastructure failure) escalate L6 regardless of cycle count.
- **Plan SHA pinning:** the plan's "SHA" is the Linear comment ID + body hash. If `linear-plan-drafter` updates the plan (re-drafts), the comment ID changes; treat that as a fresh L0 cycle.

## Step-by-step workflow

### 1. Read trigger context

You were fired by a Linear webhook on status `Plan ready for L0`. Read:

- The Linear ticket's full body, comments, and metadata
- The plan comment (most recent comment from `linear-plan-drafter` matching `<!-- routine:linear-plan-drafter run=... -->`)
- The repo's `CLAUDE.md` and current wave docs

Identify the plan's "SHA" — for our purposes this is `(comment_id, sha256(comment_body))`. You'll persist verdicts under this SHA so re-runs against the same plan are idempotent.

### 2. Idempotency + cycle counter

Read the ticket's prior comments. Count how many `<!-- routine:l0-reviewer -->` cycles have already run against the current plan SHA:

- If a `cycle_complete` marker exists for this SHA: this plan already reached unanimous or escalated. Exit cleanly.
- Otherwise, this is cycle N where N = (existing cycle markers for this SHA) + 1.
- If N > 7 (Amendment 1 cap): escalate L6 with reason `cycle_cap_exceeded`. Stop.

### 3. Determine reviewer panel

Read the plan to identify which domain reviewers apply per the matrix at `.github/reviewer-prompts/matrix.yml`. The L0 panel always includes:

- `/codex challenge` (adversarial)
- `architect-reviewer` (architectural domain)
- `/codex consult` (independent read)

Plus, if the plan touches any path or surface from `security-auditor.when_changed` in matrix.yml:

- `security-auditor`

Plus if it touches performance or accessibility surfaces, those reviewers too. You can spawn additional reviewers per the matrix; the contract is unanimous approval across whatever panel runs.

### 4. Run the panel in parallel

Invoke each reviewer in parallel. Each reviewer reads:
- The plan (current Linear comment)
- The repo files the plan references
- `CLAUDE.md`, the wave docs

Each posts its verdict as a Linear comment with this header:

```
<!-- routine:l0-reviewer cycle=N reviewer=<name> plan_sha=<comment_id>:<body_hash> -->
## L0 / <reviewer-name>
**Verdict:** approve | changes-requested | reject
**Summary:** ...
### Findings
- **[severity] [category] — [title]**
  - Location: <file:line>
  - Description: ...
  - Recommendation: ...
[or:]
No findings. Plan is sound against L0 / <reviewer-name> dimensions.
```

Verdicts must contain either ≥1 structured finding or an explicit "No findings" attestation per Amendment 2 substantive-output rules. The `parse-verdict.py` enforcement at L2 mirrors here — apply the same conservative parse manually.

### 5. Codex outage handling (Amendment 2)

If `/codex challenge` or `/codex consult` returns non-substantive output or errors:
- Retry up to 3× with backoff (1 min, 3 min, 5 min)
- After 3 failed retries → escalate L6 with reason `reviewer_infrastructure_failure`
- **Never substitute** a different subagent into the codex slot
- If during the wait, the W2 codex reliability bug references in `.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md` are not yet resolved, this is the expected fragility — escalate per the documented contract.

### 6. Aggregate verdicts

Once all reviewers have posted (or escalated):

- **All approve unanimously:** post a summary comment with `cycle_complete verdict=approve plan_sha=...`, then transition the ticket to `Approved for code`. Done.
- **Any reject or changes-requested:** post a summary, leave the ticket in `Plan ready for L0`. The `linear-plan-drafter` will (in a future iteration) detect the rejection and re-draft. For now, transition back to `Planning in progress` and let the drafter pick it up next run.
- **Convergence trigger fired** (same finding class twice across cycles, unresolved high/critical after one retry, "needs human judgment" flagged with specific question, etc.): escalate L6 with the trigger reason. Do not auto-retry.

### 7. L6 escalation contract

When escalating, post a Slack DM to James via claudebot (or the Slack MCP path) following `feedback_async_messages_zero_context_with_recommendation`:

1. **Catch-up:** what ticket, what cycle, what's been happening, why the system is stuck. 9th-grade reading level. 3–5 short bullets.
2. **The question:** the specific structural decision James needs to make.
3. **Options:** ≤4 word labels, 1–2 sentences each, with tradeoff in plain language.
4. **Recommended:** mark one option with reasoning citing the specific reviewer findings.
5. **Escape hatches:** `Tell me more` (deeper context), `Direction call` (free-text thread).

Also post the same content as a Linear comment with marker `<!-- routine:l0-reviewer escalated=L6 cycle=N plan_sha=... -->`. Then transition the ticket to `L6 escalation pending`.

### 8. Audit

Stdout summary line: `l0-reviewer: ticket=<DOS-NNN> cycle=N reviewers=<count> verdict=<aggregate>`.

Linear comments are the durable audit trail per v1-lite §7. Each reviewer's comment AND your aggregate-summary comment make up the record.

## First-run validation

Before enabling auto-fire on webhook, manually trigger this routine against a real (or test) ticket in `Plan ready for L0` state. Verify:

1. All required reviewers post substantive comments
2. Codex retry behavior triggers correctly when codex fails (test by temporarily breaking the codex CLI invocation)
3. Aggregate-summary comment posts and ticket transitions correctly
4. L6 escalation Slack DM lands cleanly when artificially triggering convergence
