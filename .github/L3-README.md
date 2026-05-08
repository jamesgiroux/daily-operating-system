# L3 Wave Review

Runs after the last PR of a wave merges to dev (or via `workflow_dispatch`).

## Triggers

- `pull_request` closed with merge=true and a `wave-WN` label, when no other
  open PRs share the label
- Manual: `gh workflow run l3-review.yml -f wave=W0`

## What runs

| Stage | Job | Coverage |
|---|---|---|
| Panel | `panel-codex` | Adversarial codex challenge against integrated wave diff |
| Panel | `panel-architect` | architect-reviewer agent on integrated state |
| Suite S | `suite-s` | All CI policy invariants on integrated state (service-layer boundary, write-fence, ability-drift, durable-source-comments, OAuth secret scan, clippy `-D warnings`, cargo-audit) |
| Suite P | `suite-p` | criterion benches vs prior wave baseline; >10% regression fails |
| Suite E | `suite-e` | `pnpm test` + (W2+ only) attestation file `.docs/plans/wave-WN/l3-suite-e-attest.md` containing `L3-suite-e-attest: passed` |

## Output

- **All green**: opens auto-PR `l3-proof/wave-WN` with `proof-bundle.md` written under `.docs/plans/wave-WN/` plus updated baseline under `.docs/perf-baselines/`
- **Any red**: workflow exits non-zero. Findings + raw responses are uploaded as workflow artifacts. CI fail email lands; the active polling session reads artifacts and fixes forward. No Slack/L6 escalation.

## Adding a new bench

Add a criterion bench under `src-tauri/benches/` (or any workspace member's `benches/`). Suite P will pick it up on the next wave's run.

## Manual attestation for Suite E (W2+ waves)

Before re-dispatching L3 for a W2+ wave that ships user-visible surfaces, write `.docs/plans/wave-WN/l3-suite-e-attest.md` with the line `L3-suite-e-attest: passed` and a list of surfaces QA'd. Commit and push, then re-dispatch L3.

Until headless Tauri lands, this is the explicit gate. False attestations are on the human; the workflow doesn't try to verify the surfaces themselves.

## Adversarial review

The two reviewer slots use prompts at `.github/reviewer-prompts/l3-{codex-challenge,architect-reviewer}.md`. They expect a verdict format ending in two lines: `VERDICT:` followed by approve/changes-requested/blocked, and `FINDINGS:` followed by `critical=N high=N medium=N low=N`.

`approve` with non-zero critical/high requires `tracked-followup` markers in the body, otherwise the verdict is rejected by `parse-verdict` logic in the composite action.

## Failure mode philosophy

Per `feedback_no_escalation_infra_for_ci_failures`: the workflow exits non-zero, attaches artifacts, and stops. The CI fail email + the active polling session handle remediation. No Slack DM, no Linear status flips, no escalation infrastructure layered on.
