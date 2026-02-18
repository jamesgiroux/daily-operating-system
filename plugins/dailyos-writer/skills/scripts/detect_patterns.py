#!/usr/bin/env python3
"""
Pattern detector for VIP editorial anti-patterns.

Checks for:
- Contrast framing ("not X, it's Y")
- Negative parallels ("unlike X")
- AI tropes and buzzwords
- Excessive hedging
- Stylistic crutches ("here's the thing", "the truth is", etc.)

Usage:
    python detect_patterns.py <file.md>
"""

import re
import sys
from pathlib import Path
from typing import List

class PatternIssue:
    def __init__(self, line_num: int, line: str, pattern_type: str, message: str, suggestion: str = None):
        self.line_num = line_num
        self.line = line
        self.pattern_type = pattern_type
        self.message = message
        self.suggestion = suggestion

def check_contrast_framing(line: str, line_num: int) -> List[PatternIssue]:
    """Check for contrast framing patterns."""
    issues = []

    patterns = [
        (r"\b(?:isn't|wasn't|aren't|weren't|not)\s+(?:just\s+)?[\w\s]+[,\.]?\s+(?:it's|they're|we're|it is|they are|we are)",
         "Contrast framing detected: 'not X, it's Y'",
         "State Y directly without contrasting with X"),

        (r"\baren't\s+asking\s+(?:if|whether)[\w\s]+\.\s+They're\s+asking",
         "Contrast framing: 'aren't asking if... they're asking'",
         "State what they're doing directly"),

        (r"\bnot\s+treating[\w\s]+as\s+[\w\s]+\.\s+They're\s+treating",
         "Contrast framing: 'not treating... they're treating'",
         "State how they're treating directly"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "contrast-framing",
                message, suggestion
            ))

    return issues

def check_negative_parallels(line: str, line_num: int) -> List[PatternIssue]:
    """Check for negative parallel comparisons."""
    issues = []

    patterns = [
        (r"\bunlike\s+\w+", "Negative parallel: 'unlike X'",
         "State our strengths directly without comparing"),

        (r"\bwhereas\s+\w+", "Negative parallel: 'whereas X'",
         "State our approach without contrasting competitors"),

        (r"\bin contrast to", "Negative parallel: 'in contrast to'",
         "Focus on our capabilities rather than competitor weaknesses"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "negative-parallel",
                message, suggestion
            ))

    return issues

def check_ai_tropes(line: str, line_num: int) -> List[PatternIssue]:
    """Check for AI buzzword tropes."""
    issues = []

    tropes = [
        "game-changing", "revolutionary", "paradigm shift",
        "transformative", "disruptive", "unprecedented",
        "cutting-edge", "next-generation", "world-class",
        "best-in-class", "industry-leading", "market-leading"
    ]

    for trope in tropes:
        if re.search(r'\b' + re.escape(trope) + r'\b', line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "ai-trope",
                f"AI buzzword detected: '{trope}'",
                "Replace with specific, evidence-based description"
            ))

    return issues

def check_excessive_hedging(line: str, line_num: int) -> List[PatternIssue]:
    """Check for excessive hedging language."""
    issues = []

    patterns = [
        (r"\bseems to be\b", "Hedging: 'seems to be'", "Be direct: 'is'"),
        (r"\bappears to\b", "Hedging: 'appears to'", "Be direct: state what it does"),
        (r"\bpotentially could\b", "Double hedging: 'potentially could'", "Use 'could' or 'might'"),
        (r"\bmight possibly\b", "Double hedging: 'might possibly'", "Use 'might'"),
        (r"\bmay or may not\b", "Excessive hedge: 'may or may not'", "State what's known"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "excessive-hedging",
                message, suggestion
            ))

    return issues

def check_vague_claims(line: str, line_num: int) -> List[PatternIssue]:
    """Check for vague claims that need evidence."""
    issues = []

    patterns = [
        (r"\bclearly\s+\w+ing", "Weak claim: 'clearly X-ing'",
         "Provide specific evidence instead of asserting clarity"),

        (r"\bsignificant(?:ly)?\s+(?:impact|momentum|traction)", "Vague claim: 'significant X'",
         "Quantify with specific metrics or examples"),

        (r"\breally\s+\w+ing", "Vague intensifier: 'really X-ing'",
         "Provide evidence instead of intensifying"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "vague-claim",
                message, suggestion
            ))

    return issues

def check_stylistic_crutches(line: str, line_num: int) -> List[PatternIssue]:
    """Check for overused stylistic phrases that become formulaic."""
    issues = []

    patterns = [
        (r"\b(?:but\s+)?here's\s+the\s+thing", "Stylistic crutch: 'here's the thing'",
         "State the insight directly without the windup"),

        (r"\bthe\s+truth\s+is\b", "Stylistic crutch: 'the truth is'",
         "State the truth directly"),

        (r"\blet\s+me\s+be\s+(?:clear|honest)\b", "Stylistic crutch: 'let me be clear/honest'",
         "Just be clear or honest without announcing it"),

        (r"\bat\s+the\s+end\s+of\s+the\s+day\b", "Stylistic crutch: 'at the end of the day'",
         "State the conclusion directly"),

        (r"\bthe\s+reality\s+is\b", "Stylistic crutch: 'the reality is'",
         "State the reality directly"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "stylistic-crutch",
                message, suggestion
            ))

    return issues

def detect_patterns(file_path: Path) -> List[PatternIssue]:
    """Detect editorial patterns in a file."""
    issues = []

    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    in_code_block = False
    for line_num, line in enumerate(lines, 1):
        # Skip code blocks
        if line.strip().startswith('```'):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        # Skip frontmatter
        if line_num <= 20 and line.strip().startswith('---'):
            continue

        # Run checks
        issues.extend(check_contrast_framing(line, line_num))
        issues.extend(check_negative_parallels(line, line_num))
        issues.extend(check_ai_tropes(line, line_num))
        issues.extend(check_excessive_hedging(line, line_num))
        issues.extend(check_vague_claims(line, line_num))
        issues.extend(check_stylistic_crutches(line, line_num))

    return issues

def print_issues(issues: List[PatternIssue]):
    """Print issues in a readable format."""
    if not issues:
        print("✓ No editorial pattern issues found!")
        return

    print(f"\n⚠️  Found {len(issues)} editorial pattern issue(s):\n")

    # Group by type
    by_type = {}
    for issue in issues:
        if issue.pattern_type not in by_type:
            by_type[issue.pattern_type] = []
        by_type[issue.pattern_type].append(issue)

    for pattern_type, type_issues in by_type.items():
        print(f"## {pattern_type.upper().replace('-', ' ')} ({len(type_issues)} issue(s)):")
        for issue in type_issues:
            print(f"\nLine {issue.line_num}:")
            print(f"  {issue.line.strip()}")
            print(f"  → {issue.message}")
            if issue.suggestion:
                print(f"  💡 {issue.suggestion}")
        print()

def main():
    if len(sys.argv) < 2:
        print("Usage: python detect_patterns.py <file.md>")
        sys.exit(1)

    file_path = Path(sys.argv[1])

    if not file_path.exists():
        print(f"Error: File not found: {file_path}")
        sys.exit(1)

    issues = detect_patterns(file_path)
    print_issues(issues)

    sys.exit(0 if not issues else 1)

if __name__ == '__main__':
    main()
