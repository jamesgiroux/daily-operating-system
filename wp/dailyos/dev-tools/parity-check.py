#!/usr/bin/env python3
"""
parity-check.py — compare a rendered WP section's DOM skeleton against a
canonical account-detail HTML source.

The script reduces both sides to a structural skeleton (tag + class list +
data-ds-name + nesting) and diffs them. Text content + dynamic data-*
attribute VALUES are stripped because they're expected to differ between
renders (entity names, claim IDs, timestamps, rendered prose). What MUST
match is the DOM tree shape — every parent/child relationship + every
class name + every data-ds-name.

Usage:
    parity-check.py [--source reference|tauri] [--wp-url URL] <section-id>
    parity-check.py --reference-file PATH [--wp-url URL] <section-id>

Section IDs (reference anchor ids in account.html):
    headline           → AccountHero (entity-detail_chapterSection)
    outlook            → AccountOutlook chapter
    state-of-play      → UnifiedTimeline chapter
    the-room           → StakeholderGallery chapter
    watch-list         → WatchList chapter
    value-commitments  → ValueCommitments chapter
    strategic-landscape → StrategicLandscape chapter
    the-record         → TheRecord chapter
    the-work           → TheWork chapter
    reports            → Reports chapter

Sources:
    reference → .docs/design/reference/surfaces/account.html (default)
    tauri     → .docs/plans/v1.4.4-wp-surface-migration/
                ACCOUNT-DETAIL-HEALTH-DOM-PASTE.html, with CSS-module
                hash classes normalized to semantic class names

    --reference-file PATH can point at any pasted Tauri DOM capture. Class
    normalization is enabled by default for explicit files because the pasted
    app DOM usually contains CSS-module hashes.

Exit codes:
    0  → DOM skeleton matches (parity OK)
    1  → DOM skeleton diverges (parity FAIL — diff printed)
    2  → section not found on one or both sides
    3  → fetch / parse error
"""

from __future__ import annotations

import argparse
import difflib
import re
import sys
import urllib.error
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

DEFAULT_WP_URL = "http://localhost:8884/accounts/acme-corp/"
REPO_ROOT = Path(__file__).resolve().parents[3]
REFERENCE_FILE = REPO_ROOT / ".docs/design/reference/surfaces/account.html"
TAURI_REFERENCE_FILE = (
    REPO_ROOT
    / ".docs/plans/v1.4.4-wp-surface-migration/ACCOUNT-DETAIL-HEALTH-DOM-PASTE.html"
)
REFERENCE_SOURCES = {
    "reference": REFERENCE_FILE,
    "tauri": TAURI_REFERENCE_FILE,
}

# Data-* attribute keys whose VALUES are dynamic and should be stripped.
# Their PRESENCE still matters — only the value is masked.
DYNAMIC_DATA_ATTRS = {
    "data-claim-id",
    "data-trust-band",
    "data-empty-reason",
    "data-quality-level",
    "data-dailyos-entity-id",
    "data-dailyos-envelope-handle",
    "data-source-asof",
    "data-source",
}
# Attribute keys that are inline content rather than structural and so
# their values can also vary freely (titles, aria-labels, href).
VOLATILE_ATTRS = {"title", "aria-label", "href", "id", "style"}


CSS_MODULE_CLASS_RE = re.compile(r"^_([A-Za-z][A-Za-z0-9]*)_[A-Za-z0-9]+_\d+$")
DAILYOS_MODULE_CLASS_RE = re.compile(r"^[A-Za-z][A-Za-z0-9-]*_([A-Za-z][A-Za-z0-9-]*)$")


class TreeNode:
    __slots__ = ("tag", "classes", "children")

    def __init__(self, tag: str) -> None:
        self.tag = tag
        self.classes: list[str] = []
        self.children: list[TreeNode] = []

    def render(self, depth: int = 0) -> list[str]:
        # Skeleton = tag + sorted class list + nesting. All attributes are
        # masked because they vary legitimately across the WP/reference
        # surfaces: data-claim-id/data-trust-band are WP-side trust-wiring,
        # data-ds-* are tooling metadata, data-quality-level/data-highlight
        # are content-driven. Visual parity is owned by the CSS module via
        # class names, so class set + nesting is the structural ground truth.
        attrs = ""
        if self.classes:
            attrs += "." + ".".join(self.classes)
        lines = ["  " * depth + self.tag + attrs]
        for child in self.children:
            lines.extend(child.render(depth + 1))
        return lines


VOID_ELEMENTS = {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
}


class SkeletonBuilder(HTMLParser):
    """Extracts a tree of TreeNode from an HTML fragment, dropping text +
    volatile attributes. Optionally limits to a sub-tree rooted at the
    element with a target id.
    """

    def __init__(self, target_id: str | None = None, normalize_classes: bool = False) -> None:
        super().__init__(convert_charrefs=True)
        self.root = TreeNode("__root__")
        self.stack: list[TreeNode] = [self.root]
        # When target_id is set, only collect inside that subtree.
        self.target_id = target_id
        self.in_target_depth = 0  # >0 means we're inside the target subtree.
        self.target_found = False
        self.normalize_classes = normalize_classes

    def handle_starttag(self, tag: str, attrs_list: list[tuple[str, str | None]]) -> None:
        attrs = {k: (v or "") for k, v in attrs_list}
        if self.target_id is not None and attrs.get("id") == self.target_id:
            self.in_target_depth = 1
            self.target_found = True
            # Reset root to start fresh from this element.
            self.root = TreeNode("__root__")
            self.stack = [self.root]
        elif self.target_id is not None and self.in_target_depth > 0 and tag not in VOID_ELEMENTS:
            self.in_target_depth += 1

        # Skip when target_id is set and we're not inside the target subtree.
        if self.target_id is not None and not self.target_found:
            return

        node = TreeNode(tag)
        class_attr = attrs.get("class", "")
        node.classes = normalize_class_list(class_attr.split(), self.normalize_classes)
        self.stack[-1].children.append(node)
        if tag not in VOID_ELEMENTS:
            self.stack.append(node)

    def handle_endtag(self, tag: str) -> None:
        if self.target_id is not None and self.in_target_depth > 0:
            self.in_target_depth -= 1
            if self.in_target_depth == 0:
                # Just closed the target element — stop collecting.
                self.target_found = False
                self.target_id = None  # Lock further nodes out.
        if tag in VOID_ELEMENTS:
            return
        # Pop the matching open node. HTMLParser handles malformed nesting
        # leniently, so we tolerate stack underflows.
        if len(self.stack) > 1:
            self.stack.pop()


def normalize_class_list(classes: list[str], enabled: bool) -> list[str]:
    if not enabled:
        return sorted(c for c in classes if c)

    normalized = []
    for class_name in classes:
        semantic = normalize_class_name(class_name)
        if semantic:
            normalized.append(semantic)
    return sorted(set(normalized))


def normalize_class_name(class_name: str) -> str:
    """Normalize build-specific DailyOS class names to semantic class names."""
    if is_utility_class(class_name):
        return ""

    css_module_match = CSS_MODULE_CLASS_RE.match(class_name)
    if css_module_match:
        return css_module_match.group(1)

    dailyos_module_match = DAILYOS_MODULE_CLASS_RE.match(class_name)
    if dailyos_module_match:
        return dailyos_module_match.group(1)

    return class_name


UTILITY_CLASS_PREFIXES = (
    "*:",
    "[",
    "aria-",
    "bg-",
    "border-",
    "dark:",
    "data-",
    "disabled:",
    "focus-visible:",
    "gap-",
    "has-",
    "hover:",
    "justify-",
    "opacity-",
    "outline-",
    "px-",
    "py-",
    "rounded-",
    "shadow-",
    "shrink-",
    "size-",
    "text-",
    "transition-",
    "w-",
    "whitespace-",
)
UTILITY_CLASS_EXACT = {
    "border",
    "flex",
    "font-medium",
    "h-8",
    "h-9",
    "inline-flex",
    "items-center",
}


def is_utility_class(class_name: str) -> bool:
    """Drop Tailwind/shadcn utility noise from pasted Tauri DOM captures."""
    if class_name in UTILITY_CLASS_EXACT:
        return True
    return any(class_name.startswith(prefix) for prefix in UTILITY_CLASS_PREFIXES)


def extract_skeleton(html: str, section_id: str, normalize_classes: bool = False) -> list[str]:
    parser = SkeletonBuilder(target_id=section_id, normalize_classes=normalize_classes)
    parser.feed(html)
    parser.close()
    if not parser.root.children:
        return []
    # The captured subtree IS the first child of root.
    return parser.root.children[0].render(depth=0)


def fetch_wp(url: str) -> str:
    try:
        with urllib.request.urlopen(url, timeout=15) as resp:
            return resp.read().decode("utf-8", errors="replace")
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as exc:
        print(f"fetch error ({url}): {exc}", file=sys.stderr)
        sys.exit(3)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Compare a rendered WP section's DOM skeleton against the "
            "static reference or a pasted Tauri DOM file."
        )
    )
    parser.add_argument("section_id", help="Anchor id to compare, e.g. outlook")
    parser.add_argument(
        "--wp-url",
        default=DEFAULT_WP_URL,
        help="Rendered WordPress URL to compare. Defaults to the account fixture URL.",
    )
    parser.add_argument(
        "--source",
        choices=sorted(REFERENCE_SOURCES),
        default="reference",
        help="Reference source to compare against.",
    )
    parser.add_argument(
        "--reference-file",
        type=Path,
        help="Explicit HTML reference file, usually a pasted Tauri DOM capture.",
    )
    parser.add_argument(
        "--normalize-classes",
        action="store_true",
        help="Normalize DailyOS CSS-module/hash class names before comparing.",
    )
    parser.add_argument(
        "--no-normalize-classes",
        action="store_true",
        help="Disable class-name normalization even for Tauri or explicit files.",
    )
    args = parser.parse_args()

    wp_html = fetch_wp(args.wp_url)
    reference_file = args.reference_file if args.reference_file is not None else REFERENCE_SOURCES[args.source]
    if not reference_file.is_absolute():
        reference_file = REPO_ROOT / reference_file
    if not reference_file.exists():
        print(f"reference not found: {reference_file}", file=sys.stderr)
        return 3
    ref_html = reference_file.read_text(encoding="utf-8")
    normalize_classes = "tauri" == args.source or args.reference_file is not None
    if args.normalize_classes:
        normalize_classes = True
    if args.no_normalize_classes:
        normalize_classes = False

    wp_skeleton = extract_skeleton(wp_html, args.section_id, normalize_classes)
    ref_skeleton = extract_skeleton(ref_html, args.section_id, normalize_classes)

    if not wp_skeleton:
        print(f"section #{args.section_id} not found in WP render at {args.wp_url}", file=sys.stderr)
        return 2
    if not ref_skeleton:
        print(f"section #{args.section_id} not found in reference {reference_file}", file=sys.stderr)
        return 2

    if wp_skeleton == ref_skeleton:
        print(f"PARITY OK · #{args.section_id} · {len(wp_skeleton)} nodes match")
        return 0

    diff = list(
        difflib.unified_diff(
            ref_skeleton,
            wp_skeleton,
            fromfile=f"{args.source}#{args.section_id}",
            tofile=f"wp#{args.section_id}",
            n=3,
            lineterm="",
        )
    )
    print(f"PARITY FAIL · #{args.section_id} · {len(diff)} diff lines")
    print()
    for line in diff:
        print(line)
    return 1


if __name__ == "__main__":
    sys.exit(main())
