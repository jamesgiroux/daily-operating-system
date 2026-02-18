#!/usr/bin/env python3
"""
Typography linter for VIP editorial standards.

Checks for:
- Em dashes (should use parentheses or periods)
- Straight quotes (should use curly quotes)
- WordPress VIP terminology
- Salesforce product names
- Oxford comma issues

Usage:
    python lint_typography.py <file.md>
"""

import re
import sys
from pathlib import Path
from typing import List

class TypographyIssue:
    def __init__(self, line_num: int, line: str, issue_type: str, message: str, suggestion: str = None):
        self.line_num = line_num
        self.line = line
        self.issue_type = issue_type
        self.message = message
        self.suggestion = suggestion

def check_em_dashes(line: str, line_num: int) -> List[TypographyIssue]:
    """Check for em dashes (—) that should be replaced."""
    issues = []
    if '—' in line:
        count = line.count('—')
        suggestion = "Replace em dashes with parentheses, periods, or restructure the sentence."
        issues.append(TypographyIssue(
            line_num, line, "em-dash",
            f"Found {count} em dash(es). Em dashes should be avoided in VIP content.",
            suggestion
        ))
    return issues

def check_wordpress_terminology(line: str, line_num: int) -> List[TypographyIssue]:
    """Check for incorrect WordPress VIP terminology."""
    issues = []

    patterns = [
        (r'\bWordpress\b', "WordPress", "Use 'WordPress' (capital P)"),
        (r'\bWordPress-VIP\b', "WordPress VIP", "Use 'WordPress VIP' (no hyphen)"),
        (r'\bWP VIP\b', "WordPress VIP", "Use full 'WordPress VIP' in formal content"),
        (r'\bAgent Force\b', "Agentforce", "Salesforce product is 'Agentforce' (one word)"),
        (r'\bAgentForce\b', "Agentforce", "Use 'Agentforce' (not camelCase)"),
        (r'\bDataCloud\b', "Data Cloud", "Use 'Data Cloud' (two words)"),
    ]

    for pattern, correct, message in patterns:
        if re.search(pattern, line):
            issues.append(TypographyIssue(
                line_num, line, "terminology",
                message,
                f"Replace with '{correct}'"
            ))

    return issues

def check_quotes(line: str, line_num: int) -> List[TypographyIssue]:
    """Check for straight quotes that should be curly."""
    issues = []

    # Skip code blocks and technical content
    if line.strip().startswith('```') or line.strip().startswith('    '):
        return issues

    # Check for straight double quotes
    if '"' in line and not line.strip().startswith('>'):  # Skip blockquotes
        issues.append(TypographyIssue(
            line_num, line, "quotes",
            "Straight quotes found. Use curly quotes for prose.",
            "Replace \" with " and ""
        ))

    return issues

def check_oxford_comma(line: str, line_num: int) -> List[TypographyIssue]:
    """Check for potential Oxford comma violations."""
    issues = []

    # Pattern: word, word and word (missing comma before 'and')
    pattern = r'\b\w+,\s+\w+\s+and\s+\w+\b'
    if re.search(pattern, line):
        issues.append(TypographyIssue(
            line_num, line, "oxford-comma",
            "Possible missing Oxford comma before 'and'.",
            "Review list structure for serial comma"
        ))

    return issues

def lint_file(file_path: Path) -> List[TypographyIssue]:
    """Lint a file for typography issues."""
    issues = []

    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    in_code_block = False
    in_frontmatter = False
    frontmatter_count = 0

    for line_num, line in enumerate(lines, 1):
        # Track frontmatter (between first two --- markers)
        if line.strip() == '---':
            frontmatter_count += 1
            if frontmatter_count == 1:
                in_frontmatter = True
            elif frontmatter_count == 2:
                in_frontmatter = False
            continue

        # Skip frontmatter
        if in_frontmatter:
            continue

        # Skip code blocks
        if line.strip().startswith('```'):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        # Run checks
        issues.extend(check_em_dashes(line, line_num))
        issues.extend(check_wordpress_terminology(line, line_num))
        issues.extend(check_quotes(line, line_num))
        issues.extend(check_oxford_comma(line, line_num))

    return issues

def print_issues(issues: List[TypographyIssue]):
    """Print issues in a readable format."""
    if not issues:
        print("✓ No typography issues found!")
        return

    print(f"\n❌ Found {len(issues)} typography issue(s):\n")

    # Group by type
    by_type = {}
    for issue in issues:
        if issue.issue_type not in by_type:
            by_type[issue.issue_type] = []
        by_type[issue.issue_type].append(issue)

    for issue_type, type_issues in by_type.items():
        print(f"## {issue_type.upper()} ({len(type_issues)} issue(s)):")
        for issue in type_issues:
            print(f"\nLine {issue.line_num}:")
            print(f"  {issue.line.strip()}")
            print(f"  → {issue.message}")
            if issue.suggestion:
                print(f"  💡 {issue.suggestion}")
        print()

def main():
    if len(sys.argv) < 2:
        print("Usage: python lint_typography.py <file.md>")
        sys.exit(1)

    file_path = Path(sys.argv[1])

    if not file_path.exists():
        print(f"Error: File not found: {file_path}")
        sys.exit(1)

    issues = lint_file(file_path)
    print_issues(issues)

    sys.exit(0 if not issues else 1)

if __name__ == '__main__':
    main()
