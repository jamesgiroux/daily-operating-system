# Contributing to the Daily Operating System

Thank you for your interest in contributing to the Daily Operating System! This document provides guidelines for development, testing, and submitting changes.

## Development Setup

### Prerequisites

- Python 3.8+
- Node.js 18+ (for web dashboard)
- Claude Code CLI installed and configured
- Git

### Initial Setup

1. **Clone or access the workspace**
   ```bash
   cd /Users/[username]/Documents/VIP
   ```

2. **Install Python dependencies** (if any)
   ```bash
   pip install -r requirements.txt
   ```

3. **Set up Google API credentials**
   - Follow instructions in `.docs/CONFIGURATION.md`
   - Place `credentials.json` in `.config/google/`

4. **Run initial verification**
   ```bash
   # Test Python utilities
   python3 _tools/validate_naming.py _inbox

   # Test Google API
   python3 .config/google/google_api.py calendar list 1
   ```

### Development Workflow

1. **Create feature branch** (if using branches)
   ```bash
   git checkout -b feature/your-feature
   ```

2. **Make changes**
   - Commands: `.claude/commands/`
   - Skills: `.claude/skills/`
   - Agents: `.claude/agents/`
   - Utilities: `_tools/`

3. **Test changes**
   - Run the modified command manually
   - Check output files are correct
   - Verify no regressions

4. **Commit with proper message**
   ```bash
   git add -p  # Stage specific changes
   git commit -m "feat: Add new feature X

   Detailed description of changes.

   Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
   ```

---

## Code Style Guidelines

### Markdown Files (Commands, Skills, Agents)

- Use clear section headers (H2 for major sections)
- Include execution steps in numbered format
- Document dependencies and error handling
- Use code blocks for examples
- Keep lines under 120 characters where practical

**Example Structure:**
```markdown
# /command-name - Title

Description of command.

## When to Use

Context for when this command applies.

## Execution Steps

### Step 1: Name

Detailed instructions.

### Step 2: Name

More instructions.

## Output Structure

Expected file outputs.

## Dependencies

- API 1
- API 2

## Error Handling

What to do when things go wrong.
```

### Python Files

- Follow PEP 8 style guide
- Use type hints where helpful
- Include docstrings for functions
- Handle errors gracefully with clear messages

**Example:**
```python
def process_file(filepath: str) -> dict:
    """
    Process a single file from the inbox.

    Args:
        filepath: Path to the file to process

    Returns:
        Dictionary with processing results

    Raises:
        FileNotFoundError: If file doesn't exist
        ValueError: If file format is invalid
    """
    # Implementation
```

### YAML Frontmatter

- Always include required fields
- Use consistent field ordering
- Validate against frontmatter standards

**Required order:**
```yaml
---
area:
account: # if applicable
project: # if applicable
doc_type:
status:
date:
tags:
privacy:
---
```

---

## Testing Requirements

### Manual Testing Checklist

Before submitting changes, verify:

#### For Commands

- [ ] Command executes without errors
- [ ] Output files are created in correct locations
- [ ] Frontmatter is valid
- [ ] Related files are updated appropriately
- [ ] Graceful degradation when dependencies missing

#### For Skills

- [ ] Skill triggers correctly
- [ ] All steps are executable
- [ ] Output matches expected format
- [ ] Agents invoked correctly (if applicable)

#### For Agents

- [ ] Agent produces expected output
- [ ] Quality gates are applied
- [ ] Integration with skills works

#### For Python Utilities

- [ ] Script runs without errors
- [ ] Input validation works
- [ ] Error messages are helpful
- [ ] Edge cases handled

### Testing Workflow

1. **Test in isolation**
   ```bash
   # For commands, invoke directly
   /today

   # For skills, invoke directly
   /strategy-consulting [prompt]

   # For utilities, run with test data
   python3 _tools/script.py _inbox/test-file.md
   ```

2. **Test integration**
   - Run full workflow: `/today` → work → `/wrap`
   - Verify file routing works end-to-end

3. **Test edge cases**
   - Empty inputs
   - Missing dependencies
   - Invalid data

---

## Pull Request Process

### Before Submitting

1. **Self-review your changes**
   - Read through all modified files
   - Check for typos and formatting
   - Verify documentation is complete

2. **Update relevant documentation**
   - `.docs/` files if APIs changed
   - CLAUDE.md if commands changed
   - ADRs for significant decisions

3. **Test thoroughly**
   - Complete testing checklist
   - Document any known issues

### PR Template

```markdown
## Summary

Brief description of changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Refactoring

## Changes Made

- Change 1
- Change 2

## Testing Done

- [ ] Manual testing completed
- [ ] Edge cases tested
- [ ] Documentation updated

## Related Issues

Fixes #123 (if applicable)
```

### Review Process

1. **Automated checks** (if configured)
   - Frontmatter validation
   - Naming convention checks

2. **Code review**
   - Check code style compliance
   - Verify logic is correct
   - Ensure error handling

3. **Merge**
   - Squash commits if many small changes
   - Use meaningful merge commit message

---

## Issue Templates

### Bug Report

```markdown
## Bug Description

Clear description of what's wrong.

## Steps to Reproduce

1. Run command X
2. Do action Y
3. Observe error Z

## Expected Behavior

What should happen.

## Actual Behavior

What actually happens.

## Environment

- OS: macOS 14.x
- Python: 3.10
- Claude Code: 1.0.x
```

### Feature Request

```markdown
## Feature Description

What you'd like to add.

## Use Case

Why this feature is needed.

## Proposed Solution

How you think it should work.

## Alternatives Considered

Other approaches you've thought about.
```

---

## Architecture Decisions

### When to Write an ADR

Write an Architecture Decision Record when:
- Making significant architectural choices
- Choosing between competing approaches
- Establishing patterns for future development
- Making decisions that are hard to reverse

### ADR Process

1. Create new file in `.docs/decisions/`
2. Use format: `ADR-NNN-short-title.md`
3. Follow template in `.docs/decisions/README.md`
4. Get review before finalizing

---

## Questions and Support

### Getting Help

- Check `.docs/TROUBLESHOOTING.md` for common issues
- Review existing documentation
- Ask in relevant channels

### Reporting Issues

- Use issue templates
- Provide complete reproduction steps
- Include environment details
- Attach relevant logs

---

## License

This is a personal workspace. Contribution guidelines are primarily for the author's own reference and for any future collaborators.
