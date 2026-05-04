#!/usr/bin/env python3
"""
audit-reference.py — verify each reference HTML faithfully mirrors its TSX.

For each (HTML, TSX) pair in surface-manifest.json, report:
  - INVENTED CLASSES   — class in HTML but not in module CSS (or unknown prefix)
  - MISSING CLASSES    — styles.X in TSX but corresponding Module_X not in HTML
  - INLINE STYLES      — style="..." in HTML where TSX uses className/styles instead
  - MISSING IMPORTS    — TSX imports a component whose class prefix doesn't appear in HTML
  - TEXT DELTAS        — string literals in TSX (h1/h2/p text, button labels) absent from HTML

Usage:
  python3 .docs/design/_audits/audit-reference.py [--surface NAME] [--json] [--strict]

Reports go to .docs/design/_audits/reference-fidelity.md by default.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[3]
MANIFEST = REPO / ".docs/design/_audits/surface-manifest.json"
REPORT = REPO / ".docs/design/_audits/reference-fidelity.md"
BASELINE = REPO / ".docs/design/_audits/.fidelity-baseline.json"

SEVERITY_RANK = {"clean": 0, "minor": 1, "major": 2, "critical": 3}

# --- regexes ----------------------------------------------------------------

# styles.foo or styles["foo"]
RE_STYLES_REF = re.compile(r"styles\.([A-Za-z_][A-Za-z0-9_]*)|styles\[['\"]([A-Za-z_][A-Za-z0-9_]*)['\"]\]")

# import { Foo, Bar } from "@/components/..."
RE_TSX_IMPORT = re.compile(
    r'import\s+(?:type\s+)?\{([^}]+)\}\s+from\s+["\']@/(components|pages|features)/([^"\']+)["\']'
)

# text inside JSX tags: >Some text<
# crude but sufficient for catching button labels, headings
RE_JSX_TEXT = re.compile(r">([A-Z][A-Za-z][^<>{]{2,80})<")

# any class= attribute value
RE_HTML_CLASS = re.compile(r'class=["\']([^"\']+)["\']')

# inline style="..." in HTML
RE_HTML_INLINE_STYLE = re.compile(r'style=["\']([^"\']+)["\']')

# style={{...}} or style={...} in TSX
RE_TSX_INLINE_STYLE = re.compile(r"style=\{\{?([^}]+)\}?\}", re.DOTALL)

# .scoped_name { ... } in module CSS — captures full class name with prefix
RE_CSS_CLASS = re.compile(r"^\.([A-Za-z_][A-Za-z0-9_-]*)\s*[,{:]", re.MULTILINE)


# --- helpers ----------------------------------------------------------------

def read(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text()


def extract_styles_refs(tsx: str) -> set[str]:
    """All `styles.X` references in TSX → returns {X1, X2, ...}."""
    refs: set[str] = set()
    for m in RE_STYLES_REF.finditer(tsx):
        refs.add(m.group(1) or m.group(2))
    return refs


def extract_tsx_imports(tsx: str) -> list[tuple[str, str]]:
    """All component imports → list of (componentName, fullPathFromSrc).

    fullPathFromSrc includes the 'components/' (or 'pages/'/'features/') prefix
    so callers can resolve module CSS via REPO/src/<fullPathFromSrc>.module.css.
    """
    out: list[tuple[str, str]] = []
    for m in RE_TSX_IMPORT.finditer(tsx):
        names = [n.strip() for n in m.group(1).split(",") if n.strip()]
        kind = m.group(2)
        path = m.group(3)
        full = f"{kind}/{path}"
        for name in names:
            # strip "type X" / "X as Y" / aliases
            cleaned = name.replace("type ", "").split(" as ")[0].strip()
            if cleaned and cleaned[0].isupper():
                out.append((cleaned, full))
    return out


def extract_html_classes(html: str) -> set[str]:
    """All distinct class names used in HTML."""
    out: set[str] = set()
    for m in RE_HTML_CLASS.finditer(html):
        for cls in m.group(1).split():
            out.add(cls)
    return out


def extract_css_classes(css: str) -> set[str]:
    """All class names defined in a CSS file (after scope-modules.py prefixing)."""
    return set(RE_CSS_CLASS.findall(css))


def extract_html_inline_styles(html: str) -> list[str]:
    """All inline style attribute values."""
    return [m.group(1).strip() for m in RE_HTML_INLINE_STYLE.finditer(html)]


def has_tsx_inline_style(tsx: str) -> bool:
    """True if TSX uses any style={{...}}."""
    return bool(RE_TSX_INLINE_STYLE.search(tsx))


def extract_jsx_text(tsx: str) -> set[str]:
    """Heuristic JSX text content — captures longer strings between tags."""
    out: set[str] = set()
    for m in RE_JSX_TEXT.finditer(tsx):
        s = m.group(1).strip()
        # filter out JS literals masquerading as text (e.g. boolean values)
        if len(s) >= 6 and not s.startswith(("/", "*", "//")):
            out.add(s)
    return out


def find_module_css(component_path: str) -> Path | None:
    """Given a component import path, find its .module.css if present."""
    base = REPO / "src" / component_path
    candidates = [
        Path(str(base) + ".module.css"),
        base.parent / (base.stem + ".module.css"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


# --- core audit -------------------------------------------------------------

def audit_surface(entry: dict[str, Any]) -> dict[str, Any]:
    """Return findings dict for a single surface."""
    html_path = REPO / entry["html"]
    tsx_path = REPO / entry["primary"]
    module_path = REPO / entry["module"] if entry.get("module") else None
    prefix = entry.get("module_prefix")

    findings: dict[str, Any] = {
        "html": entry["html"],
        "tsx": entry["primary"],
        "module_prefix": prefix,
        "exists": {"html": html_path.exists(), "tsx": tsx_path.exists()},
        "invented_classes": [],
        "missing_classes": [],
        "inline_styles_html_only": [],
        "missing_imports": [],
        "text_deltas": [],
    }

    if not html_path.exists() or not tsx_path.exists():
        return findings

    html = read(html_path)
    tsx = read(tsx_path)

    # ── invented + missing classes ──────────────────────────────────────────
    html_classes = extract_html_classes(html)
    tsx_styles = extract_styles_refs(tsx)

    if prefix and module_path and module_path.exists():
        # The mirror under _shared/styles/ holds the SCOPED (prefixed) class names
        mirror_path = REPO / ".docs/design/reference/_shared/styles" / module_path.name
        scoped_css = read(mirror_path) if mirror_path.exists() else ""
        defined_scoped = extract_css_classes(scoped_css)

        # invented: HTML uses {prefix}_X but X is not defined in mirror
        for cls in sorted(html_classes):
            if cls.startswith(f"{prefix}_"):
                if cls not in defined_scoped:
                    findings["invented_classes"].append(cls)

        # missing: TSX uses styles.X, HTML should have {prefix}_X
        for raw in sorted(tsx_styles):
            scoped = f"{prefix}_{raw}"
            if scoped in defined_scoped and scoped not in html_classes:
                findings["missing_classes"].append({"tsx": raw, "expected_html": scoped})

    # ── inline styles in HTML but not TSX ───────────────────────────────────
    html_inlines = extract_html_inline_styles(html)
    tsx_uses_inline = has_tsx_inline_style(tsx)
    # Skip benign tokenized inline styles (CSS var only, no hardcoded values).
    # Also skip the .ob-button-primary <style> block helper which is intentional.
    suspicious = []
    for s in html_inlines:
        # Allow common reference-only patterns that don't represent layout invention
        if re.fullmatch(r'\s*(--[a-z-]+:\s*[^;]+;\s*)+', s):
            continue
        # If TSX uses zero inline style, ANY inline style in HTML is suspect.
        # If TSX uses inline style, allow but flag for human review.
        if not tsx_uses_inline:
            suspicious.append(s)
    findings["inline_styles_html_only"] = suspicious

    # ── missing imports ─────────────────────────────────────────────────────
    # Chrome components are rendered at runtime by chrome.js, so their CSS
    # prefixes never appear in the static reference HTML. Don't flag them.
    CHROME_RUNTIME = {"FolioBar", "FloatingNavIsland", "AtmosphereLayer"}

    imports = extract_tsx_imports(tsx)
    for component, path in imports:
        # if the component has its own module.css, look for that prefix in HTML
        css_path = find_module_css(path)
        if not css_path:
            continue
        stem = css_path.stem.replace(".module", "")
        if stem in CHROME_RUNTIME:
            continue
        # Heuristic: confirm the component is actually used in JSX before
        # complaining (cuts false positives where a component is imported
        # but only conditionally rendered or used as a type).
        if not re.search(rf"<\s*{re.escape(component)}\b", tsx):
            continue
        if not any(cls.startswith(f"{stem}_") for cls in html_classes):
            # Component imported AND used in TSX but its CSS prefix not in HTML
            findings["missing_imports"].append({"component": component, "module": stem})

    # ── text deltas ─────────────────────────────────────────────────────────
    tsx_strings = extract_jsx_text(tsx)
    # Drop strings that are obviously dynamic (e.g., contain {variable})
    static_strings = {s for s in tsx_strings if "{" not in s and "}" not in s}
    for s in sorted(static_strings):
        if s not in html:
            findings["text_deltas"].append(s)

    return findings


def severity(f: dict[str, Any]) -> str:
    """Categorize: critical / major / minor / clean."""
    if f["missing_imports"] or len(f["inline_styles_html_only"]) > 5:
        return "critical"
    if f["invented_classes"] or f["missing_classes"] or f["inline_styles_html_only"]:
        return "major"
    if f["text_deltas"]:
        return "minor"
    return "clean"


def render_md(all_findings: list[dict[str, Any]]) -> str:
    out: list[str] = []
    out.append("# Reference fidelity audit\n")
    out.append("Generated by `.docs/design/_audits/audit-reference.py`. ")
    out.append("Each reference HTML is compared against its canonical TSX. ")
    out.append("See the script docstring for what each finding means.\n\n")

    by_sev: dict[str, list[dict[str, Any]]] = {"critical": [], "major": [], "minor": [], "clean": []}
    for f in all_findings:
        by_sev[severity(f)].append(f)

    out.append("## Summary\n")
    out.append(f"- **Critical**: {len(by_sev['critical'])} surfaces (missing imports or >5 inline-style violations)\n")
    out.append(f"- **Major**: {len(by_sev['major'])} surfaces (invented/missing classes, or some inline styles)\n")
    out.append(f"- **Minor**: {len(by_sev['minor'])} surfaces (text deltas only)\n")
    out.append(f"- **Clean**: {len(by_sev['clean'])} surfaces\n\n")

    for sev_name in ("critical", "major", "minor", "clean"):
        items = by_sev[sev_name]
        if not items:
            continue
        out.append(f"## {sev_name.title()} ({len(items)})\n\n")
        for f in items:
            out.append(f"### `{f['html']}`\n")
            out.append(f"vs `{f['tsx']}`\n\n")

            if f["missing_imports"]:
                out.append("**Missing imports** (TSX imports a component whose CSS prefix isn't in HTML):\n")
                for mi in f["missing_imports"]:
                    out.append(f"- `{mi['component']}` — expected `{mi['module']}_*` classes\n")
                out.append("\n")

            if f["invented_classes"]:
                out.append(f"**Invented classes** ({len(f['invented_classes'])}): ")
                out.append(", ".join(f"`{c}`" for c in f["invented_classes"][:15]))
                if len(f["invented_classes"]) > 15:
                    out.append(f" … +{len(f['invented_classes']) - 15} more")
                out.append("\n\n")

            if f["missing_classes"]:
                out.append(f"**Missing classes** ({len(f['missing_classes'])}): TSX `styles.X` not in HTML:\n")
                for mc in f["missing_classes"][:10]:
                    out.append(f"- `{mc['tsx']}` → expected `{mc['expected_html']}`\n")
                if len(f["missing_classes"]) > 10:
                    out.append(f"- … +{len(f['missing_classes']) - 10} more\n")
                out.append("\n")

            if f["inline_styles_html_only"]:
                out.append(f"**Suspicious inline styles** ({len(f['inline_styles_html_only'])}, TSX uses none):\n")
                for s in f["inline_styles_html_only"][:5]:
                    out.append(f"- `{s[:120]}{'…' if len(s) > 120 else ''}`\n")
                if len(f["inline_styles_html_only"]) > 5:
                    out.append(f"- … +{len(f['inline_styles_html_only']) - 5} more\n")
                out.append("\n")

            if f["text_deltas"]:
                out.append(f"**Text deltas** ({len(f['text_deltas'])} strings in TSX absent from HTML):\n")
                for t in f["text_deltas"][:5]:
                    out.append(f"- “{t[:80]}{'…' if len(t) > 80 else ''}”\n")
                if len(f["text_deltas"]) > 5:
                    out.append(f"- … +{len(f['text_deltas']) - 5} more\n")
                out.append("\n")

            out.append("---\n\n")

    return "".join(out)


def load_baseline() -> dict[str, str]:
    """Returns {html_path: severity} from baseline file, or {} if absent."""
    if not BASELINE.exists():
        return {}
    return json.loads(BASELINE.read_text())


def write_baseline(findings: list[dict[str, Any]]) -> None:
    """Snapshot current severity per surface."""
    snapshot = {f["html"]: severity(f) for f in findings}
    BASELINE.write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--surface", help="Audit only the surface whose HTML basename matches")
    ap.add_argument("--json", action="store_true", help="Emit JSON to stdout instead of writing markdown")
    ap.add_argument("--strict", action="store_true", help="Exit non-zero if any critical or major findings")
    ap.add_argument("--enforce-baseline", action="store_true",
                    help="Exit non-zero if any surface regressed vs baseline (used by pre-commit hook)")
    ap.add_argument("--write-baseline", action="store_true",
                    help="Snapshot current findings as the new baseline")
    args = ap.parse_args()

    manifest = json.loads(MANIFEST.read_text())
    entries = manifest["surfaces"]
    if args.surface:
        entries = [e for e in entries if Path(e["html"]).name == args.surface or args.surface in e["html"]]
        if not entries:
            print(f"No surface matched: {args.surface}", file=sys.stderr)
            return 2

    all_findings = [audit_surface(e) for e in entries]

    if args.write_baseline:
        write_baseline(all_findings)
        print(f"Baseline written: {BASELINE.relative_to(REPO)}")
        return 0

    if args.enforce_baseline:
        baseline = load_baseline()
        regressions: list[tuple[str, str, str]] = []
        improvements: list[tuple[str, str, str]] = []
        for f in all_findings:
            current = severity(f)
            previous = baseline.get(f["html"], "clean")
            if SEVERITY_RANK[current] > SEVERITY_RANK[previous]:
                regressions.append((f["html"], previous, current))
            elif SEVERITY_RANK[current] < SEVERITY_RANK[previous]:
                improvements.append((f["html"], previous, current))
        if improvements:
            print("Reference fidelity improvements (run --write-baseline to lock these in):", file=sys.stderr)
            for path, prev, curr in improvements:
                print(f"  {prev} → {curr}: {path}", file=sys.stderr)
        if regressions:
            print("\n🚫 Reference fidelity REGRESSED — block commit:", file=sys.stderr)
            for path, prev, curr in regressions:
                print(f"  {prev} → {curr}: {path}", file=sys.stderr)
            print("\nRun  python3 .docs/design/_audits/audit-reference.py", file=sys.stderr)
            print("for the full report. Fix or rebaseline before committing.", file=sys.stderr)
            return 1
        return 0

    if args.json:
        json.dump({"findings": all_findings}, sys.stdout, indent=2)
        print()
    else:
        REPORT.write_text(render_md(all_findings))
        print(f"Report: {REPORT.relative_to(REPO)}")
        for f in all_findings:
            sev = severity(f)
            print(f"  [{sev:8s}] {f['html']}")

    if args.strict:
        bad = [f for f in all_findings if severity(f) in ("critical", "major")]
        if bad:
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
