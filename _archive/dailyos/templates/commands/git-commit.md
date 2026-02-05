# /git-commit - Atomic Commit Workflow

Create clean, atomic commits with meaningful messages.

## When to Use

Run when you have changes ready to commit. This command:
- Reviews all staged and unstaged changes
- Groups changes into logical commits
- Generates meaningful commit messages
- Pushes to remote (with confirmation)

## Philosophy

**Atomic commits** - Each commit should represent one logical change.

**Meaningful messages** - Commit messages should explain WHY, not just WHAT.

**Clean history** - A readable git history is documentation.

## Execution Steps

### Step 1: Check Repository Status

```bash
git status
```

Identify:
- Staged changes (ready to commit)
- Unstaged changes (modified but not staged)
- Untracked files (new files not in git)

### Step 2: Review Changes

```bash
# Staged changes
git diff --staged

# Unstaged changes
git diff

# Recent commit history for style reference
git log --oneline -10
```

### Step 3: Analyze and Group

```python
def analyze_changes(status):
    """
    Group changes into logical commits
    """
    groups = []

    # Group by type of change
    for file in status['modified']:
        change_type = classify_change(file)
        # e.g., 'feature', 'fix', 'refactor', 'docs', 'style'

        # Find or create group
        group = find_group(groups, change_type)
        group['files'].append(file)

    return groups
```

**Change types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes nor adds
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Step 4: Generate Commit Messages

For each logical group:

```
"I've identified [X] logical change groups:

1. **docs**: Updated meeting notes (3 files)
   Suggested: 'docs: Update meeting notes for Client A and B'

2. **feat**: New dashboard template (2 files)
   Suggested: 'feat: Add account dashboard template'

3. **fix**: Corrected action item dates (1 file)
   Suggested: 'fix: Correct due dates in task list'

Accept these commits? [Yes / Edit messages / Combine / Skip]"
```

### Step 5: Stage and Commit

For each approved commit:

```bash
# Stage specific files
git add [file1] [file2]

# Commit with message
git commit -m "$(cat <<'EOF'
[type]: [Short description]

[Longer explanation if needed]

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

### Step 6: Push to Remote

```
"[X] commits created locally. Push to remote?

Branch: [current-branch]
Remote: origin

[Push / Push with force (careful!) / Skip push]"
```

```bash
git push origin [branch]
```

## Commit Message Format

```
[type]: [Short description (50 chars max)]

[Optional body - wrap at 72 chars]

[Optional footer - references, co-authors]

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

**Examples:**

```
docs: Update weekly impact capture for W03

Added customer meeting outcomes and personal impact
sections for the week.

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

```
feat: Add email triage to daily overview

The /today command now includes email scanning when
Gmail API is configured. High priority emails are
surfaced in the overview with full thread summaries.

Closes #12

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

## Safety Rules

**NEVER:**
- Commit credentials or secrets (.env, tokens, etc.)
- Force push to main/master without explicit confirmation
- Amend commits that have been pushed
- Skip pre-commit hooks without explicit request

**ALWAYS:**
- Review staged changes before committing
- Use meaningful commit messages
- Add Co-Authored-By for AI-assisted commits
- Confirm before pushing

## .gitignore Awareness

Check for sensitive files before committing:

```python
sensitive_patterns = [
    '.env',
    'credentials.json',
    'token.json',
    '*.secret',
    '*.key',
]

for file in staged_files:
    if matches_sensitive(file):
        warn(f"Sensitive file detected: {file}")
        confirm("Include this file?")
```

## Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
GIT COMMIT COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Commits created: 3
Files committed: 7
Pushed to: origin/main

Commits:
1. docs: Update meeting notes for Client A
2. feat: Add email scanning to daily overview
3. chore: Clean up old archive files

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## Related Commands

- `/today` - May trigger git-commit for daily files
- `/wrap` - End-of-day may include commit step
