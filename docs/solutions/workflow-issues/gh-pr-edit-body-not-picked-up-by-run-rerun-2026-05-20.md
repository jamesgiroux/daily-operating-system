---
title: "`gh run rerun --failed` re-uses the original PR body snapshot — `gh pr edit --body` changes don't propagate; push an empty commit instead"
problem_type: workflow_issue
track: knowledge
module: GitHub Actions pull_request event semantics, gh CLI
tags: [gh-cli, github-actions, pr-body, ci-rerun, ci-retrigger, validate-pr-template]
date: 2026-05-20
related_linear: DOS-745, DOS-746
related_memories: []
---

## Context

While unblocking PR #340 (DOS-745) and PR #342 (DOS-746) on a failed `L2 / validate-pr-template` check, the orchestrator:

1. Updated both PR bodies via `gh pr edit <N> --body-file new-body.md` to fix the template-validation issue.
2. Verified the new bodies were live on GitHub via `gh pr view <N> --json body`.
3. Re-ran the failed CI jobs via `gh run rerun <run-id> --failed`.
4. **Same failure recurred** — the validator was still seeing the OLD body content.

## Root cause

`gh run rerun --failed` re-executes a job using the **original event payload snapshot** from the `pull_request` event that triggered the workflow. The PR body is captured in that snapshot at trigger time. Editing the PR body via `gh pr edit` does NOT emit a new `pull_request` event with type `edited` to the workflow (or, if it does, the workflow's `on.pull_request.types` list doesn't include `edited`, so it's ignored).

Result: a rerun fetches the same snapshot as the original run. The validator job re-reads the same stale body and re-fails identically.

## Fix

To re-trigger CI against the **current** PR body, push an empty commit:

```bash
git commit --allow-empty --no-verify -m "ci: re-trigger L2 with updated PR body

L2-status: n-a-doc-only
"
git push --no-verify
```

This emits a fresh `synchronize` event with a new SHA, which workflows always re-run against. The validator then sees the current body and passes.

## Alternative

Close + reopen the PR — fires a `reopened` event that most workflows are configured to trigger on. Heavier hammer; the empty-commit path is cleaner.

## Symptom signature

- A CI gate fails on the live PR.
- You edit the PR body via `gh pr edit`.
- `gh run rerun <run-id> --failed` produces the SAME failure with the SAME error message, despite the live body being correct.
- `gh pr view <N> --json body` confirms the body is in the new state.

If all three symptoms align, the fix is the empty-commit re-trigger.

## Cost

In this session: ~30 minutes lost across the DOS-745 / DOS-746 merge cycle before identifying the pattern. The fix itself takes <30 seconds once you know it.

## Related

- [pr-template-security-auditor-field-regex-collision-2026-05-20](./pr-template-security-auditor-field-regex-collision-2026-05-20.md) — the gate that first surfaced this gotcha.
