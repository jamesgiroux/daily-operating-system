# `.github/` — Phase 2 L2 review workflow

Sets up the L2 reviewer panel as a GitHub Action on the `public` repo. Every PR opened against `dev` or `trunk` triggers parallel reviewers; branch protection requires their status checks before merge.

Phase 2 of the orchestration v1-lite plan (`.docs/plans/orchestration/v1-lite.md`).

## What ships

| Path | Purpose |
|---|---|
| `workflows/l2-review.yml` | The L2 workflow. Triggers on PR open/synchronize/reopen. Runs code-reviewer + matched domain reviewers in parallel. |
| `reviewer-prompts/code-reviewer.md` | General L2 code-quality reviewer prompt |
| `reviewer-prompts/architect-reviewer.md` | Architectural domain reviewer prompt |
| `reviewer-prompts/security-auditor.md` | Security / OWASP domain reviewer prompt |
| `reviewer-prompts/performance-engineer.md` | Performance domain reviewer prompt |
| `reviewer-prompts/accessibility-tester.md` | A11y domain reviewer prompt |
| `reviewer-prompts/matrix.yml` | Path-prefix → reviewer mapping; the workflow consults this to decide which domain reviewers to invoke per PR |
| `pull_request_template.md` | PR description template; pre-fills DoD checklist + `security_auditor_invoked` field |
| `scripts/configure-branch-protection.sh` | One-time `gh`-CLI script to enforce L2 status checks as merge gates on `dev` and `trunk` |

## Required GitHub setup (one-time)

These are manual steps you do in GitHub UI / CLI. Without them the workflow won't function.

### 1. Install the Anthropic Claude Code GitHub App

The workflow calls `anthropics/claude-code-action@v1`. You need the GitHub App installed on the `public` repo so the Action can authenticate.

Easiest path: from a Claude Code session, run `/install-github-app`. It walks through the OAuth flow, installs the app on the repo you choose, and configures the `ANTHROPIC_API_KEY` secret.

Manual alternative:
1. Visit https://github.com/apps/claude
2. Install on `jamesgiroux/daily-operating-system`
3. Add `ANTHROPIC_API_KEY` as a repo secret (Settings → Secrets and variables → Actions → New repository secret)

### 2. Configure branch protection

Run the script to require L2 status checks on `dev` and `trunk`:

```bash
bash .github/scripts/configure-branch-protection.sh
```

It uses `gh api` and is idempotent. Re-run after adding/removing reviewer jobs in the workflow.

UI alternative: GitHub → repo → Settings → Branches → Add rule for `dev` and `trunk`. Toggle:
- ✓ Require status checks to pass before merging
- ✓ Require branches to be up to date before merging
- ✓ Require linear history
- Add required checks (visible after first L2 workflow run): `L2 / code-reviewer`, plus any domain reviewers you want as hard requirements.

## How the workflow runs

1. PR opened against `dev` or `trunk` triggers the workflow.
3. **`resolve-matrix`** reads `reviewer-prompts/matrix.yml`, matches changed files against the path globs, outputs the list of reviewers to invoke.
4. **`code-reviewer`** runs always (general slot).
5. **Domain reviewers** (`architect-reviewer`, `security-auditor`, `performance-engineer`, `accessibility-tester`) run only when their matrix entries match the PR's changed files.
6. Each reviewer is a Claude Code Action run with its prompt file. The Action posts a PR review with the verdict + structured findings, and sets a status check.
7. Branch protection blocks merge until all required status checks are green.

## Updating reviewer behavior

Edit the corresponding `reviewer-prompts/*.md` file. Changes go through L2 review themselves — the prompt files are version-controlled, and the security-auditor matrix triggers on `.github/reviewer-prompts/**`, so any prompt change pulls security-auditor into its own L2 panel. This closes the routine-prompt circular-trust loop from earlier orchestration designs.

## Adding a new domain reviewer

1. Drop a new `reviewer-prompts/<name>.md` (follow the existing prompt structure).
2. Add an entry in `reviewer-prompts/matrix.yml` with `reviewer: <name>` and the path globs that should trigger it.
3. Add a job block in `workflows/l2-review.yml` for the new reviewer (copy an existing domain reviewer's job, change the prompt path).
4. Re-run `configure-branch-protection.sh` if you want the new reviewer's status check as a hard merge requirement.

## What Phase 2 does NOT include

- **`/codex review` slot** — codex CLI in a runner is non-trivial. Phase 2.1 follow-up. Until then, codex L2 happens in-cycle as developer practice.
- **Linear comment mirroring** — the Action posts to the PR (native GitHub UI). Phase 3 wires claudebot to mirror PR review comments onto the Linear ticket so Linear stays canonical per v1-lite §7.
- **Auto-merge** — Phase 5 (wave-driver) earns merge authority later. Phase 2 just gates merge readiness; you press the button.

## Bypass

`--no-verify` works on local hooks (Phase 1), not on this workflow. CI status checks are the gate. To bypass in a true emergency: an admin can override via the GitHub UI's "Merge without waiting for requirements to be met" button. That action is logged.

## Troubleshooting

- **Workflow doesn't trigger on PR.** Confirm the GitHub App is installed and `ANTHROPIC_API_KEY` is set. Confirm the PR base branch is `dev` or `trunk`.
- **A reviewer job posts no comment.** Check the workflow run logs for the Anthropic Claude Code Action step. The Action handles its own commenting; no output usually means the Action couldn't reach the API.
- **Required status check is "expected" but never runs.** A reviewer matched the matrix but the workflow file has a typo in the job name, or the matrix entry has a broken glob. Run a small test PR (docs typo) and inspect the `resolve-matrix` step's output.
