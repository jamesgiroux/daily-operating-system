# L3 Review

Adversarial review of an integrated unit-of-work. The unit can be any
group of merged PRs you decide forms a coherent thing to review:
a release wave, a Linear project, a Linear milestone, a small ad-hoc
batch, even a single big PR.

## Triggering

Manual only:

```
gh workflow run l3-review.yml \
  -f scope-id="v1.4.1-W0" \
  -f pr-numbers="225,226,227,228,229,230,231"
```

`scope-id` is free-form (filesystem-safe characters; non-alnum becomes
underscore). It controls where artifacts land — under
`.docs/plans/l3-reviews/{scope-id}/` and `.docs/perf-baselines/scope-{id}.json`.

`pr-numbers` is comma-separated merged PR numbers. The workflow finds the
oldest one's merge-commit parent as the integration "before" SHA; current
dev HEAD is the "after". The review covers the diff between those two SHAs.

Auto-trigger via Linear webhook (when a project closes / milestone hits 100%)
is a follow-up that needs Linear → GitHub webhook plumbing.

## What runs

| Stage | Job | Coverage |
|---|---|---|
| Panel | `panel-codex` | Adversarial codex challenge against integrated diff |
| Panel | `panel-architect` | architect-reviewer agent on integrated state |
| Suite S | `suite-s` | All CI policy invariants on integrated state (service-layer boundary, write-fence, ability-drift, durable-source-comments, OAuth secret scan, clippy `-D warnings`, cargo-audit) |
| Suite P | `suite-p` | criterion benches vs prior scope baseline; >10% regression fails. First scope seeds empty baseline. |
| Suite E | `suite-e` | `pnpm test` + (if scope id contains W2+ or matches user-visible work) attestation file `.docs/plans/l3-reviews/{scope-id}/l3-suite-e-attest.md` containing `L3-suite-e-attest: passed` |

## Output

- **All green**: opens auto-PR `l3-proof/{scope-id}` with `proof-bundle.md` written under `.docs/plans/l3-reviews/{scope-id}/` plus updated baseline under `.docs/perf-baselines/`.
- **Any red**: workflow exits non-zero. Findings + raw responses are uploaded as workflow artifacts. CI fail email lands; the active polling session reads artifacts and fixes forward. No Slack/L6 escalation.

## Adding a criterion bench (Suite P)

Add a bench under `src-tauri/benches/` (or any workspace member's `benches/`). Suite P will pick it up on the next run.

## Suite E attestation (W2+ / user-visible work)

For scopes that include user-visible behavior, write `.docs/plans/l3-reviews/{scope-id}/l3-suite-e-attest.md` with the line `L3-suite-e-attest: passed` and a short list of surfaces QA'd. Commit and push, then re-dispatch L3.

Until headless Tauri lands, this is the explicit gate. False attestations are on the human; the workflow doesn't try to verify the surfaces themselves.

## Adversarial review verdict format

The two reviewer slots' prompts are at `.github/reviewer-prompts/l3-{codex-challenge,architect-reviewer}.md`. Reviewers must end with two lines: `VERDICT:` (approve / changes-requested / blocked) and `FINDINGS:` (`critical=N high=N medium=N low=N`).

`approve` with non-zero critical/high requires `tracked-followup` markers in the body, otherwise the verdict is rejected.

## Failure mode philosophy

Per `feedback_no_escalation_infra_for_ci_failures`: workflow exits non-zero, attaches artifacts, stops. CI fail email + the active polling session handle remediation. No Slack DM, no Linear status flips, no escalation infrastructure layered on.
