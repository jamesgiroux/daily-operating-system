---
title: "PR template `security_auditor_invoked` regex collides with the template's own hint text — validator reports 'missing' on bodies that have the field"
problem_type: workflow_issue
track: knowledge
module: .github/scripts/validate-pr-template.py, .github/pull_request_template.md
tags: [ci, pr-template, validation, regex-gotcha, l2-gate, security-auditor]
date: 2026-05-20
related_linear: DOS-745, DOS-746
related_memories: []
---

## Context

While shipping DOS-745 (PR #340) and DOS-746 (PR #342), both PRs failed the `L2 / validate-pr-template` CI check with the same error:

```
🔒 PR-template validation: `security_auditor_invoked` field missing.
```

Both PR bodies explicitly had `` `security_auditor_invoked: true` `` in the §4 Security section. The field was clearly present.

## Root cause

`.github/scripts/validate-pr-template.py::extract_field()` uses this regex against the PR body (with fenced code blocks stripped):

```python
pattern = re.compile(
    r"`?security_auditor_invoked`?\s*:\s*(true|false)\b",
    re.IGNORECASE,
)
matches = pattern.findall(cleaned)
if len(matches) > 1 and len(set(m.lower() for m in matches)) > 1:
    return "ambiguous"
```

The original PR template (`.github/pull_request_template.md`) includes this literal hint line:

```markdown
**Exemption ID** (if `security_auditor_invoked: false`): `EXEMPT-DOC-ONLY` | ...
```

When an author copies the template and sets `security_auditor_invoked: true` while leaving the hint text intact, the regex matches **both** the truth-value (`true`) and the hint's literal `false`. The result is `"ambiguous"`, and the downstream validator reports it as `"missing"` (the user-visible message).

## Fix

Edit the §4 Security section to remove the literal `false` mention from the hint:

```markdown
**Exemption ID**: _none_
```

(Drop the `(if \`security_auditor_invoked: false\`)` parenthetical entirely.)

Verify with:

```bash
gh pr view <N> --json body --jq '.body' | grep -c "security_auditor_invoked"
# Must print 1 for the validator to pass
```

## Prevention

Two paths:

1. **Author-side**: when copying the PR template, strip the hint parentheticals that mention specific values (`true`/`false`) the validator regex would re-match.

2. **Template-side** (recommended): rephrase the hint to avoid the literal value. For example:

   ```markdown
   **Exemption ID** (required only when invocation was skipped): ...
   ```

The downstream `gh run rerun` does NOT re-fetch the PR body — see [gh-pr-edit-body-not-picked-up-by-run-rerun-2026-05-20](./gh-pr-edit-body-not-picked-up-by-run-rerun-2026-05-20.md) for the corollary gotcha that compounds this one.

## Symptom signature

CI job `L2 / validate-pr-template` fails with `🔒 PR-template validation: \`security_auditor_invoked\` field missing.` even when the field is plainly visible in the rendered PR description.

`gh pr view <N> --json body --jq '.body' | grep -c "security_auditor_invoked"` returning `2` is the diagnostic — should be `1`.
