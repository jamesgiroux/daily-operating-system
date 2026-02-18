---
description: Quick typography, terminology, and anti-pattern checks
---

# /mechanical - Quick Mechanical Check

Run a fast mechanical review on a file without the full review pipeline. Checks typography, terminology, and anti-patterns, auto-fixing where possible.

$ARGUMENTS: File path to check.

## Workflow

### Step 1: Read the File

Read the file at the provided path. If no path is given, ask the user which file to check.

### Step 2: Activate Mechanical Review Skill

The mechanical-review skill activates and performs its full check suite.

### Step 3: Run Lint Scripts

Execute the automated checking scripts:

```bash
python3 skills/scripts/lint_typography.py <file>
python3 skills/scripts/detect_patterns.py <file>
```

### Step 4: Typography Checks

| Check | Rule | Action |
|-------|------|--------|
| Em dashes | No em dashes in prose | Replace with parentheses, periods, or restructure |
| Quotation marks | Use curly quotes in prose | Replace straight quotes with curly |
| Oxford comma | Always use serial comma | Add comma before "and" in lists |
| Product names | WordPress, Agentforce, Data Cloud | Fix capitalization and styling |

### Step 5: Anti-Pattern Detection

| Pattern | Fix |
|---------|-----|
| Contrast framing ("not X, it's Y") | State Y directly |
| Negative parallels ("unlike competitors") | State capability positively |
| AI tropes ("game-changing", "revolutionary") | Use specific description |
| Excessive hedging ("seems to be", "potentially could") | Be direct |
| Vague claims ("significant traction") | Quantify with specifics |
| Stylistic crutches ("here's the thing:") | Delete, start with substance |

### Step 6: Template Artifact Check

- Section dividers (`---`) between sections: remove
- Template headers ("The Problem", "The Close"): replace with useful headers
- Generic closings ("## Conclusion"): remove or replace
- Missing BLUF/TL;DR: flag as blocking issue

### Step 7: Auto-Fix Where Possible

Automatically correct:
- Em dashes to parentheses (simple cases)
- Terminology corrections (product name capitalization)
- Straight quotes to curly quotes

Flag for human judgment:
- Sentence restructuring needs
- Contrast framing rewrites
- Evidence addition for vague claims

### Step 8: Report

Present the results:

```markdown
## Mechanical Check

### Summary
- **Issues found**: [total count]
- **Auto-fixed**: [count]
- **Requires attention**: [count]

### Auto-Fixed
[List of changes made automatically]

### Requires Attention
[List of issues needing human decision, with suggested fixes]

### Result
[CLEAN - no issues remaining / NEEDS ATTENTION - [n] items for review]
```

This is a quick check only. For the full 6-pass review cycle including structural, voice, authenticity, scrutiny, and challenger assessments, use the `/review` command instead.
