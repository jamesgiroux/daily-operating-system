# .github/ — orchestration scaffolding for L2 + L3

DailyOS uses a two-tier review model:

| Tier | What | Where it runs |
|---|---|---|
| **L2** | Pre-merge review of a single PR | **Locally**, before pushing — codex review + code-reviewer + domain reviewer per `reviewer-prompts/matrix.yml` |
| **L3** | Adversarial review of a release bundle | **CI**, on PR open against `main` |

L2 is a developer discipline. The CI side of L2 is intentionally lightweight — just the trust-boundary fence and PR-template validation. The actual reviewer panel runs in your local session before the PR opens.

L3 fires once per release on the `dev → main` PR; it reviews the integrated bundle (codex challenge + architect-reviewer + Suite S/P/E).

## Local L2 enforcement: the commit-msg hook

Every commit that touches code files must declare `L2-status` in its message:

```
L2-status: passed                  # L2 review ran clean
L2-status: not-run-acknowledged    # L2 was explicitly skipped (will surface to user)
L2-status: n-a-doc-only            # code-pattern file but actually doc-only
```

The `.githooks/commit-msg` hook fails commits without a valid declaration. Doc-only commits (no code files staged) are exempt.

**One-time install per clone:**
```bash
./scripts/install-hooks.sh
```

Bypass with `--no-verify` is available but defeats the point — use only with explicit user authorization.

## What ships in CI

| File | Purpose |
|---|---|
| `workflows/l2-review.yml` | Fires on PR open/sync vs dev. Runs **config-fence** (self-modification trust boundary) + **validate-pr-template** (security_auditor_invoked field check) + **l2-summary** (aggregator). No reviewer panel — that runs locally. |
| `workflows/l3-review.yml` | Fires on PR open/sync vs main. Runs the codex/architect panel + Suite S/P/E against the release bundle. |
| `workflows/lint-frontend.yml` | `Lint & Frontend` workflow — Linux fast-dev gate: TS type-check, ESLint, Stylelint, ~15 guard scripts (service-layer boundary, write-fence usage, ability surface drift, prompt fingerprint boundary, composition authorship, etc.), OAuth secret scan, reference fidelity audit, and `pnpm test`. Rust runs in `rust.yml` on macOS, main-branch-only. |
| `actions/l3-reviewer-job/` | Composite action for L3 panel slots. |
| `scripts/check-config-fence.sh` | Trust-boundary check used by L2's fence. |
| `scripts/configure-branch-protection.sh` | One-time branch-protection setup. |
| `scripts/validate-pr-template.py` | Verifies `security_auditor_invoked` field per PR template. |
| `reviewer-prompts/matrix.yml` | Mapping of file-path triggers → reviewer roles. **Local-L2 reference**, not CI-driven. |
| `reviewer-prompts/{accessibility,architect,code,performance,security}-{tester,reviewer,auditor,engineer}.md` | Role definitions for local-L2 reviewers. Used by your local session when running L2 before pushing. |
| `reviewer-prompts/l3-{codex-challenge,architect-reviewer}.md` | Prompts for CI-side L3 reviewers. |
| `pull_request_template.md` | Template for PR bodies (with required §4 Security checklist). |

## Required GitHub setup (one-time)

### 1. Configure branch protection

```bash
./.github/scripts/configure-branch-protection.sh
```

Required status checks added: `L2 / config-fence`, `L2 / validate-pr-template`, `L2 / l2-summary` for `dev`; `L3 / aggregate` for `main`.

### 2. Install hooks locally

Hooks are installed automatically via `pnpm install` (postinstall step calls `scripts/install-hooks.sh`). To install manually:

```bash
./scripts/install-hooks.sh
```

This wires `core.hooksPath` to `.githooks/`. Required for the commit-msg L2 acknowledgment gate, the pre-commit clippy + tsc + lint-staged gates, and the pre-push PII scan + preflight.

**CI environments skip this** — `install-hooks.sh` exits cleanly when `CI=true` or `GITHUB_ACTIONS` is set.

## Bypass

For a legitimate emergency or known-safe class:

- CI fence: `--admin` merge override (documented bootstrap path for L2/L3 self-modifying PRs)
- commit-msg hook: `git commit --no-verify` (use sparingly; surface to user)

## Troubleshooting

- **`L2 / config-fence fails`**: PR modifies L2's own config; admin override required (the bootstrap pattern).
- **`L2 / validate-pr-template fails`**: PR body missing `security_auditor_invoked: true|false`. Add to body and re-push.
- **commit-msg hook fires on a doc-only change**: a file in your commit matches a code-pattern (e.g., `.toml`). If the change is genuinely doc-only, use `L2-status: n-a-doc-only`. If you intend to skip L2 review, use `not-run-acknowledged`.
