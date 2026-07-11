#!/usr/bin/env python3
"""
Pattern detector for editorial anti-patterns and AI writing tells.

Checks for:
- Contrast framing ("not X, it's Y")
- Negative parallels ("unlike X")
- AI tropes and buzzwords
- Excessive hedging
- Stylistic crutches ("here's the thing", "the truth is", etc.)
- Abstract-noun equations ("X is the gate/unlock/forcing function") [current-gen tell]
- Setup-colons in prose ("The reality: ...") [voice tell]
- Inflated AI diction ("delve", "tapestry", "leverage", etc.)

See shared/AI-TELLS.md for the full taxonomy and the aphorism-mode vs.
narrator-mode framing. This script catches the regex-detectable subset; the
Authenticity pass catches the rest by judgment.

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
        (r"\b(?:but\s+)?here's\s+the\s+(?:thing|part|bit|point|kicker|rub)\b",
         "Stylistic crutch: 'here's the thing/part/bit' windup",
         "State the insight directly without the windup"),

        (r"\b(?:what|the\s+(?:part|thing|bit|one\s+thing))\s+I\s+keep\s+(?:coming|going)\s+back\s+to\b",
         "Stylistic crutch: 'the part I keep coming back to'",
         "False-intimacy windup. State the idea directly."),

        (r"\bthat's\s+the\s+\w+\s+I\s+(?:spent|was|kept|keep|wanted|set\s+out)\b",
         "Pointer windup: 'that's the X I …'",
         "Demonstrative back-reference that relabels the prior idea. Fold it into the substance or cut it."),

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

def check_abstract_noun_equations(line: str, line_num: int) -> List[PatternIssue]:
    """Check for the 'X is the gate' family — the priority current-gen tell.

    Matches a subject (this/that/it/the <noun>) equated to an abstract
    strategic noun via is/becomes/remains. See AI-TELLS.md Class 1.
    """
    issues = []

    strategic_nouns = (
        r"gate|unlock|wedge|lever|forcing\s+function|moment|through-?line|"
        r"north\s+star|tell|crux|linchpin|fulcrum|inflection\s+point|"
        r"litmus\s+test|tip\s+of\s+the\s+spear|key|catalyst|engine|"
        r"foundation|cornerstone|bedrock|backbone"
    )

    pattern = (
        r"\b(?:this|that|it|these|those|the\s+\w+)\s+"
        r"(?:is|are|was|were|becomes?|remains?)\s+"
        r"(?:really\s+|simply\s+|ultimately\s+|just\s+)?the\s+"
        r"(?:" + strategic_nouns + r")\b"
    )

    if re.search(pattern, line, re.IGNORECASE):
        issues.append(PatternIssue(
            line_num, line, "abstract-noun-equation",
            "Abstract-noun equation ('X is the gate/unlock/forcing function...')",
            "Aphorism-mode tell. State the plain thing and the concrete stakes. "
            "'This meeting is the gate' -> 'If this call goes well, legal review starts next week.'"
        ))

    return issues

def check_setup_colons(line: str, line_num: int) -> List[PatternIssue]:
    """Check for rhetorical setup-colons in prose (a voice tell).

    Matches a short lead-in phrase (1-4 words) followed by a colon that runs
    up to the real sentence. Avoids list markers, times, ratios, and headings.
    """
    issues = []

    # Skip markdown headings and list items — colons there are structural.
    stripped = line.lstrip()
    if stripped.startswith(("#", "-", "*", ">", "|")) or re.match(r"^\d+[.)]", stripped):
        return issues

    setup_phrases = (
        r"the\s+(?:reality|truth|point|bottom\s+line|thing|kicker|catch|upshot|"
        r"result|takeaway|problem|question)|my\s+(?:take|read|point|sense)|"
        r"bottom\s+line|here'?s\s+(?:the\s+)?(?:thing|deal|kicker|catch)|"
        r"net\s+net|in\s+short|translation"
    )

    pattern = r"(?:^|\.\s+|\?\s+|!\s+)(?:" + setup_phrases + r")\s*:\s+\S"

    if re.search(pattern, line, re.IGNORECASE):
        issues.append(PatternIssue(
            line_num, line, "setup-colon",
            "Setup-colon in prose ('The reality: ...')",
            "Delete the lead-in and the colon. 'The reality: we're behind.' -> 'We're behind.'"
        ))

    return issues

def check_inflated_diction(line: str, line_num: int) -> List[PatternIssue]:
    """Check for inflated AI diction. See AI-TELLS.md Class 8."""
    issues = []

    words = {
        "delve": "look at / dig into",
        "tapestry": "(cut the metaphor)",
        "underscore": "show / highlight",
        "boasts": "has",
        "robust": "solid / works",
        "seamless": "(say what's actually smooth)",
        "seamlessly": "(say how)",
        "ever-evolving": "changing",
        "fast-paced": "(cut)",
        "multifaceted": "(name the facets)",
        "testament to": "shows",
        "in the realm of": "in",
        "navigate the landscape": "(say the actual task)",
    }

    for word, fix in words.items():
        if re.search(r'\b' + re.escape(word) + r'\b', line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "inflated-diction",
                f"Inflated AI diction: '{word}'",
                f"Use the plain word: {fix}"
            ))

    return issues

def check_copula_avoidance(line: str, line_num: int) -> List[PatternIssue]:
    """Check for wordy stand-ins for is/has/was. See AI-TELLS.md Class 11."""
    issues = []

    patterns = [
        (r"\bserves\s+as\s+(?:a|an|the)\b", "Copula avoidance: 'serves as a'", "Use 'is a'"),
        (r"\bstands\s+as\s+(?:a|an|the)\b", "Copula avoidance: 'stands as a'", "Use 'is a'"),
        (r"\bboasts\s+(?:a|an|\d|its|the)\b", "Copula avoidance: 'boasts'", "Use 'has'"),
        (r"\brepresents\s+(?:a|an|the)\b", "Copula avoidance: 'represents a'", "Use 'is a'"),
    ]

    for pattern, message, suggestion in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "copula-avoidance", message, suggestion
            ))

    return issues

def check_vague_attribution(line: str, line_num: int) -> List[PatternIssue]:
    """Check for fog-sourced claims. See AI-TELLS.md Class 13."""
    issues = []

    patterns = [
        (r"\bexperts\s+(?:argue|say|agree|believe|note)\b", "Vague attribution: 'experts …'"),
        (r"\bobservers\s+have\s+(?:noted|cited|argued)\b", "Vague attribution: 'observers have …'"),
        (r"\b(?:some\s+)?critics\s+(?:argue|say|note|contend)\b", "Vague attribution: 'critics …'"),
        (r"\bindustry\s+reports\s+(?:suggest|show|indicate)\b", "Vague attribution: 'industry reports …'"),
        (r"\bit\s+is\s+widely\s+(?:regarded|considered|believed|seen)\b", "Vague attribution: 'it is widely …'"),
        (r"\bstudies\s+(?:show|suggest|have\s+shown)\b", "Vague attribution: 'studies show …'"),
    ]

    for pattern, message in patterns:
        if re.search(pattern, line, re.IGNORECASE):
            issues.append(PatternIssue(
                line_num, line, "vague-attribution", message,
                "Name the source, own it in first person ('my read is'), or cut it."
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
        issues.extend(check_abstract_noun_equations(line, line_num))
        issues.extend(check_setup_colons(line, line_num))
        issues.extend(check_inflated_diction(line, line_num))
        issues.extend(check_copula_avoidance(line, line_num))
        issues.extend(check_vague_attribution(line, line_num))

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
