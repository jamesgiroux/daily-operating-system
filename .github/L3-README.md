# L3 Release Review

Adversarial review of a release bundle. Fires when a PR is opened or updated against `trunk` (the stable-release branch). Reviews everything in `trunk..dev` (or whatever the release PR's `base..head` is).

## Why this scope

- **One fire per release**, not per wave. Minutes scale with releases, not PR volume.
- **Right concern at right time**: integrated-state issues matter most at the release boundary, not mid-wave when more PRs may still land.
- **Wave bundling natural**: a release contains N waves (or M Linear projects, or any mix). Reviewing the release naturally reviews everything together.
- **Branch semantics aligned**: dev = active development, trunk = stable releases (per `CLAUDE.md`). L3 (adversarial, expensive) at the release gate makes architectural sense.

## Triggering

**Auto** (default): open a PR with `dev` → `trunk` (the release PR). L3 fires on `opened`, `synchronize`, and `reopened` events. Concurrency is per-PR with cancel-in-progress, so new dev commits during release prep replace the in-flight review with a fresh one.

**Manual** (escape hatch):

```
gh workflow run l3-review.yml \
  -f scope-id="release-v1.4.1" \
  -f base="trunk" \
  -f head="dev"
```

Useful for re-running, ad-hoc reviews against arbitrary refs, or testing.

## scope-id auto-derivation

For PR-triggered runs:
- If PR title matches `Release vX.Y.Z` (case-insensitive), scope-id = `release-vX.Y.Z`
- Otherwise scope-id = `release-pr-<number>`

scope-id is sanitized to filesystem-safe characters; non-alnum becomes underscore. It controls where artifacts land — under `.docs/plans/l3-reviews/{scope-id}/` and `.docs/perf-baselines/scope-{id}.json`.

## What runs

| Stage | Job | Coverage |
|---|---|---|
| Panel | `panel-codex` | Adversarial codex challenge against integrated diff |
| Panel | `panel-architect` | architect-reviewer agent on integrated state |
| Suite S | `suite-s` | All CI policy invariants on integrated state (service-layer boundary, write-fence, ability-drift, durable-source-comments, OAuth secret scan, clippy `-D warnings`, cargo-audit) |
| Suite P | `suite-p` | criterion benches vs prior scope baseline; >10% regression fails. First scope seeds empty baseline. |
| Suite E | `suite-e` | `pnpm test` + (W2+/user-visible) attestation file `.docs/plans/l3-reviews/{scope-id}/l3-suite-e-attest.md` containing `L3-suite-e-attest: passed` |

## Output

- **All green**: comments verdict on the release PR; opens auto-PR `l3-proof/{scope-id}` against dev with `proof-bundle.md` + updated baseline.
- **Any red**: comments verdict on the release PR; workflow exits non-zero. Findings + raw responses are uploaded as workflow artifacts. CI fail email lands; the active polling session reads artifacts and fixes forward. No Slack/L6 escalation.

## Adding a criterion bench (Suite P)

Add a bench under `src-tauri/benches/` (or any workspace member's `benches/`). Suite P will pick it up on the next release cycle.

## Suite E attestation (W2+ / user-visible work)

For releases that include user-visible behavior, write `.docs/plans/l3-reviews/{scope-id}/l3-suite-e-attest.md` with the line `L3-suite-e-attest: passed` and a list of surfaces QA'd. Commit and push to dev; the next L3 firing on the release PR will pick it up.

Until headless Tauri lands, this is the explicit gate. False attestations are on the human; the workflow doesn't try to verify the surfaces themselves.

## Reviewer verdict format

Both reviewer prompts require the response end with two lines:

```
VERDICT: approve|changes-requested|blocked
FINDINGS: critical=N high=N medium=N low=N
```

`approve` with non-zero critical/high requires `tracked-followup` markers in the body, otherwise the verdict is rejected.

## Failure mode philosophy

Per `feedback_no_escalation_infra_for_ci_failures`: workflow exits non-zero, attaches artifacts, comments verdict on the PR, stops. CI fail email + the active polling session handle remediation. No Slack DM, no Linear status flips, no escalation infrastructure layered on.
