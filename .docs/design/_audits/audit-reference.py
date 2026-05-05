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
from html.parser import HTMLParser
import json
import re
import sys
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[3]
MANIFEST = REPO / ".docs/design/_audits/surface-manifest.json"
REPORT = REPO / ".docs/design/_audits/reference-fidelity.md"
BASELINE = REPO / ".docs/design/_audits/.fidelity-baseline.json"
REFERENCE_ROOT = REPO / ".docs/design/reference"
DESIGN_ROOT = REPO / ".docs/design"
ROUTER = REPO / "src/router.tsx"
RUNTIME_TOKENS = REPO / "src/styles/design-tokens.css"
REFERENCE_TOKEN_EXPORTS = [
    REPO / ".docs/design/reference/_shared/tokens.css",
    REPO / ".docs/design/reference/_shared/styles/design-tokens.css",
]

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

# local href/src references in HTML
RE_HTML_ASSET_REF = re.compile(r'''(?:href|src)=["']([^"']+)["']''')

# style={{...}} or style={...} in TSX
RE_TSX_INLINE_STYLE = re.compile(r"style=\{\{?([^}]+)\}?\}", re.DOTALL)

# .scoped_name { ... } in module CSS — captures full class name with prefix
RE_CSS_CLASS = re.compile(r"^\s*\.([A-Za-z_][A-Za-z0-9_-]*)\s*[,{:]", re.MULTILINE)

# import Foo from "@/pages/Foo" / import { Foo } from "@/components/Foo"
RE_ROUTER_DEFAULT_IMPORT = re.compile(r'import\s+([A-Za-z_][A-Za-z0-9_]*)\s+from\s+["\']@/([^"\']+)["\']')
RE_ROUTER_NAMED_IMPORT = re.compile(r'import\s+\{([^}]+)\}\s+from\s+["\']@/([^"\']+)["\']')
RE_ROUTE_DECL = re.compile(r"const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*createRoute\(\{", re.MULTILINE)
RE_ROUTE_PATH = re.compile(r'path:\s*["\']([^"\']+)["\']')
RE_ROUTE_COMPONENT = re.compile(r"component:\s*([A-Za-z_][A-Za-z0-9_]*)")
RE_CSS_CUSTOM_PROP = re.compile(r"(--[A-Za-z0-9_-]+)\s*:\s*([^;]+);")
RE_CSS_IMPORT = re.compile(r"@import\s+(?:url\()?['\"]([^'\"]+)['\"]\)?\s*;")
RE_SPEC_DS_NAME = re.compile(r"\*\*`data-ds-name`:\*\*\s*`([^`]+)`")
RE_SPEC_DS_SPEC = re.compile(r"\*\*`data-ds-spec`:\*\*\s*`([^`]+)`")
RE_SPEC_TIER = re.compile(r"\*\*Tier:\*\*\s*([A-Za-z-]+)")
RE_SPEC_STATUS = re.compile(r"\*\*Status:\*\*\s*([^\n]+)")
ROUTE_COVERED_STATUSES = {"covered", "covered_by", "covered-by", "referenced", "referenced+spec"}


# --- helpers ----------------------------------------------------------------

def read(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text()


def rel(path: Path) -> str:
    """Repo-relative path for stable report output."""
    try:
        return str(path.relative_to(REPO))
    except ValueError:
        return str(path)


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


class DsAttrParser(HTMLParser):
    """Collect elements that opt into reference inspector metadata."""

    def __init__(self, source: Path) -> None:
        super().__init__(convert_charrefs=True)
        self.source = source
        self.elements: list[dict[str, Any]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attr_map = {name: value or "" for name, value in attrs}
        if any(name.startswith("data-ds-") for name in attr_map):
            line, _ = self.getpos()
            self.elements.append({
                "file": rel(self.source),
                "line": line,
                "tag": tag,
                "attrs": attr_map,
            })


def extract_reference_ds_elements(html_path: Path) -> list[dict[str, Any]]:
    parser = DsAttrParser(html_path)
    parser.feed(read(html_path))
    return parser.elements


def normalize_tier(tier: str | None) -> str | None:
    if tier is None:
        return None
    normalized = tier.strip().lower()
    if normalized == "tokens":
        return "token"
    return normalized


def normalize_status(status: str | None) -> str:
    if status is None:
        return "unknown"
    normalized = re.sub(r"[`*]", "", status).strip().lower()
    if "planned" in normalized or "roadmap" in normalized or "promotion" in normalized:
        return "planned"
    if "canonical" in normalized:
        return "canonical"
    return normalized or "unknown"


def extract_spec_metadata(markdown: str) -> dict[str, str | None]:
    name = RE_SPEC_DS_NAME.search(markdown)
    spec = RE_SPEC_DS_SPEC.search(markdown)
    tier = RE_SPEC_TIER.search(markdown)
    status = RE_SPEC_STATUS.search(markdown)
    return {
        "data-ds-name": name.group(1) if name else None,
        "data-ds-spec": spec.group(1) if spec else None,
        "tier": normalize_tier(tier.group(1)) if tier else None,
        "status": normalize_status(status.group(1) if status else None),
    }


def validate_spec_metadata(
    *,
    target: Path,
    expected_name: str | None,
    expected_spec: str,
    expected_tier: str | None,
) -> list[dict[str, str | None]]:
    metadata = extract_spec_metadata(read(target))
    mismatches: list[dict[str, str | None]] = []

    checks = [
        ("data-ds-name", expected_name, metadata["data-ds-name"]),
        ("data-ds-spec", expected_spec, metadata["data-ds-spec"]),
        ("tier", normalize_tier(expected_tier), metadata["tier"]),
    ]
    for field, expected, actual in checks:
        if expected is None:
            continue
        if actual != expected:
            mismatches.append({
                "field": field,
                "expected": expected,
                "actual": actual,
            })
    return mismatches


def audit_reference_metadata() -> dict[str, Any]:
    """Validate data-ds-* instrumentation and spec links across reference HTML."""
    required = {"data-ds-tier", "data-ds-name", "data-ds-spec"}
    missing_required: list[dict[str, Any]] = []
    broken_specs: list[dict[str, Any]] = []
    spec_metadata_mismatches: list[dict[str, Any]] = []

    for html_path in sorted(REFERENCE_ROOT.rglob("*.html")):
        for element in extract_reference_ds_elements(html_path):
            attrs = element["attrs"]
            missing = sorted(required - set(attrs))
            if missing:
                missing_required.append({
                    "file": element["file"],
                    "line": element["line"],
                    "tag": element["tag"],
                    "name": attrs.get("data-ds-name"),
                    "missing": missing,
                })

            spec = attrs.get("data-ds-spec")
            if spec:
                # Specs are intentionally repo-local design docs, not URLs.
                if re.match(r"^[a-z]+://", spec) or spec.startswith("/"):
                    target = None
                else:
                    target = DESIGN_ROOT / spec
                if target is None or not target.exists():
                    broken_specs.append({
                        "file": element["file"],
                        "line": element["line"],
                        "name": attrs.get("data-ds-name"),
                        "spec": spec,
                    })
                else:
                    mismatches = validate_spec_metadata(
                        target=target,
                        expected_name=attrs.get("data-ds-name"),
                        expected_spec=spec,
                        expected_tier=attrs.get("data-ds-tier"),
                    )
                    if mismatches:
                        spec_metadata_mismatches.append({
                            "file": element["file"],
                            "line": element["line"],
                            "name": attrs.get("data-ds-name"),
                            "spec": spec,
                            "target": rel(target),
                            "mismatches": mismatches,
                        })

    return {
        "missing_required": missing_required,
        "broken_specs": broken_specs,
        "spec_metadata_mismatches": spec_metadata_mismatches,
    }


def extract_article_chunk_for_spec(html: str, spec: str) -> str:
    marker = f'data-ds-spec="{spec}"'
    offset = html.find(marker)
    if offset == -1:
        marker = f"data-ds-spec='{spec}'"
        offset = html.find(marker)
    if offset == -1:
        return ""

    start = html.rfind("<article", 0, offset)
    end = html.find("</article>", offset)
    if start == -1 or end == -1:
        return html[max(0, offset - 600): offset + 600]
    return html[start:end]


def has_planned_reference_marker(html: str, spec: str) -> bool:
    article = extract_article_chunk_for_spec(html, spec)
    return 'data-status="planned"' in article or "data-status='planned'" in article


def audit_system_reference_coverage() -> dict[str, Any]:
    """Ensure every primitive/pattern spec is represented in the system reference UI."""
    coverage: dict[str, Any] = {}

    for tier, directory, page_rel in (
        ("primitive", "primitives", "system/primitives.html"),
        ("pattern", "patterns", "system/patterns.html"),
    ):
        page = REFERENCE_ROOT / page_rel
        html = read(page)
        represented_specs = {
            element["attrs"].get("data-ds-spec", "")
            for element in extract_reference_ds_elements(page)
            if normalize_tier(element["attrs"].get("data-ds-tier")) == tier
        }
        represented_specs.discard("")

        specs: list[dict[str, Any]] = []
        missing_specs: list[dict[str, str]] = []
        planned_without_status: list[dict[str, str]] = []
        canonical_marked_planned: list[dict[str, str]] = []

        for spec_path in sorted((DESIGN_ROOT / directory).glob("*.md")):
            if spec_path.name == "README.md":
                continue
            metadata = extract_spec_metadata(read(spec_path))
            spec = metadata["data-ds-spec"] or f"{directory}/{spec_path.name}"
            name = metadata["data-ds-name"] or spec_path.stem
            status = normalize_status(metadata["status"])
            represented = spec in represented_specs
            spec_record = {
                "name": name,
                "spec": spec,
                "file": rel(spec_path),
                "status": status,
                "represented": represented,
            }
            specs.append(spec_record)

            if not represented:
                missing_specs.append(spec_record)
                continue

            planned_marker = has_planned_reference_marker(html, spec)
            if status == "planned" and not planned_marker:
                planned_without_status.append(spec_record)
            if status == "canonical" and planned_marker:
                canonical_marked_planned.append(spec_record)

        expected_specs = {item["spec"] for item in specs}
        extra_reference_specs = sorted(represented_specs - expected_specs)
        coverage[tier] = {
            "page": rel(page),
            "specs": specs,
            "spec_count": len(specs),
            "canonical_count": sum(1 for item in specs if item["status"] == "canonical"),
            "planned_count": sum(1 for item in specs if item["status"] == "planned"),
            "represented_count": sum(1 for item in specs if item["represented"]),
            "missing_specs": missing_specs,
            "planned_without_status": planned_without_status,
            "canonical_marked_planned": canonical_marked_planned,
            "extra_reference_specs": extra_reference_specs,
        }

    return coverage


def resolve_local_reference(source: Path, raw_ref: str, absolute_root: Path) -> Path | None:
    ref = raw_ref.split("#", 1)[0].split("?", 1)[0]
    if not ref or ref.startswith("#"):
        return None
    if ref.startswith(("http://", "https://", "mailto:", "data:", "javascript:")):
        return None
    if re.match(r"^[a-z]+://", ref):
        return None
    if ref.startswith("/"):
        return (absolute_root / ref.lstrip("/")).resolve()
    return (source.parent / ref).resolve()


def audit_reference_asset_links() -> dict[str, Any]:
    """Validate local HTML href/src refs and CSS @import refs under the design docs root."""
    broken_html_refs: list[dict[str, str]] = []
    broken_css_imports: list[dict[str, str]] = []

    for html_path in sorted(REFERENCE_ROOT.rglob("*.html")):
        html = read(html_path)
        for match in RE_HTML_ASSET_REF.finditer(html):
            raw_ref = match.group(1)
            target = resolve_local_reference(html_path, raw_ref, DESIGN_ROOT)
            if target is not None and not target.exists():
                broken_html_refs.append({
                    "file": rel(html_path),
                    "ref": raw_ref,
                    "resolved": rel(target),
                })

    for css_path in sorted(REFERENCE_ROOT.rglob("*.css")):
        css = read(css_path)
        for raw_ref in RE_CSS_IMPORT.findall(css):
            target = resolve_local_reference(css_path, raw_ref, DESIGN_ROOT)
            if target is not None and not target.exists():
                broken_css_imports.append({
                    "file": rel(css_path),
                    "ref": raw_ref,
                    "resolved": rel(target),
                })

    return {
        "broken_html_refs": broken_html_refs,
        "broken_css_imports": broken_css_imports,
    }


def strip_css_comments(css: str) -> str:
    return re.sub(r"/\*.*?\*/", "", css, flags=re.DOTALL)


def read_css_with_imports(path: Path, seen: set[Path] | None = None) -> str:
    """Read CSS and inline simple local @import files for token comparisons."""
    if seen is None:
        seen = set()
    if not path.exists() or path in seen:
        return ""
    seen.add(path)
    css = read(path)
    chunks: list[str] = []
    for import_path in RE_CSS_IMPORT.findall(css):
        if re.match(r"^[a-z]+://", import_path) or import_path.startswith("/"):
            continue
        chunks.append(read_css_with_imports((path.parent / import_path).resolve(), seen))
    chunks.append(css)
    return "\n".join(chunks)


def extract_custom_properties(css: str) -> dict[str, str]:
    """Return custom property definitions with whitespace-normalized values."""
    props: dict[str, str] = {}
    for name, value in RE_CSS_CUSTOM_PROP.findall(strip_css_comments(css)):
        props[name] = " ".join(value.strip().split())
    return props


def audit_token_exports() -> dict[str, Any]:
    """Compare duplicate reference token exports to runtime design tokens."""
    runtime = extract_custom_properties(read(RUNTIME_TOKENS))
    exports: list[dict[str, Any]] = []

    for token_path in REFERENCE_TOKEN_EXPORTS:
        current = extract_custom_properties(read_css_with_imports(token_path))
        missing = sorted(set(runtime) - set(current))
        extra = sorted(set(current) - set(runtime))
        mismatches = [
            {
                "token": name,
                "runtime": runtime[name],
                "reference": current[name],
            }
            for name in sorted(set(runtime) & set(current))
            if runtime[name] != current[name]
        ]
        exports.append({
            "file": rel(token_path),
            "exists": token_path.exists(),
            "runtime_count": len(runtime),
            "reference_count": len(current),
            "missing": missing,
            "extra": extra,
            "value_mismatches": mismatches,
        })

    duplicate_drift: list[dict[str, Any]] = []
    if len(REFERENCE_TOKEN_EXPORTS) >= 2:
        first_path = REFERENCE_TOKEN_EXPORTS[0]
        first = extract_custom_properties(read_css_with_imports(first_path))
        for other_path in REFERENCE_TOKEN_EXPORTS[1:]:
            other = extract_custom_properties(read_css_with_imports(other_path))
            duplicate_drift.append({
                "left": rel(first_path),
                "right": rel(other_path),
                "left_only": sorted(set(first) - set(other)),
                "right_only": sorted(set(other) - set(first)),
                "value_mismatches": [
                    {
                        "token": name,
                        "left": first[name],
                        "right": other[name],
                    }
                    for name in sorted(set(first) & set(other))
                    if first[name] != other[name]
                ],
            })

    return {
        "runtime": rel(RUNTIME_TOKENS),
        "exports": exports,
        "duplicate_drift": duplicate_drift,
    }


def router_import_map(router_source: str) -> dict[str, str]:
    imports: dict[str, str] = {}
    for name, import_path in RE_ROUTER_DEFAULT_IMPORT.findall(router_source):
        imports[name] = f"src/{import_path}.tsx"
    for names, import_path in RE_ROUTER_NAMED_IMPORT.findall(router_source):
        for raw in names.split(","):
            cleaned = raw.replace("type ", "").strip()
            if not cleaned:
                continue
            exported = cleaned.split(" as ")[0].strip()
            local = cleaned.split(" as ")[-1].strip()
            if exported and local and local[0].isupper():
                imports[local] = f"src/{import_path}.tsx"
    # DashboardPage is local router glue around DailyBriefing's routed content.
    imports["DashboardPage"] = "src/components/dashboard/DailyBriefing.tsx"
    return imports


def extract_router_routes() -> list[dict[str, Any]]:
    source = read(ROUTER)
    imports = router_import_map(source)
    routes: list[dict[str, Any]] = []

    for match in RE_ROUTE_DECL.finditer(source):
        route_name = match.group(1)
        end = source.find("\n});", match.end())
        if end == -1:
            continue
        block = source[match.end():end]
        path_match = RE_ROUTE_PATH.search(block)
        component_match = RE_ROUTE_COMPONENT.search(block)
        if not path_match or not component_match:
            continue
        component = component_match.group(1)
        routes.append({
            "route": route_name,
            "path": path_match.group(1),
            "component": component,
            "source": imports.get(component),
        })

    return routes


def manifest_coverage(manifest: dict[str, Any]) -> tuple[set[str], dict[str, dict[str, Any]]]:
    """Return covered source paths plus optional route status keyed by path/component."""
    covered: set[str] = set()
    statuses: dict[str, dict[str, Any]] = {}

    for entry in manifest.get("surfaces", []):
        status = entry.get("status", "covered")
        primary = entry.get("primary")
        if primary and status not in {"missing", "gap"}:
            covered.add(primary)
        covered_by_values = entry.get("covered_by", [])
        if isinstance(covered_by_values, str):
            covered_by_values = [covered_by_values]
        for covered_by in covered_by_values:
            if isinstance(covered_by, str) and covered_by.startswith("src/"):
                covered.add(covered_by)
        for source in entry.get("route_sources", []):
            if status not in {"missing", "gap"}:
                covered.add(source)
        keys = [k for k in [primary, entry.get("component")] if k]
        keys.extend(entry.get("route_sources", []))
        keys.extend(entry.get("route_paths", []))
        for key in keys:
            statuses[key] = entry

    for entry in manifest.get("route_coverage", []):
        status = entry.get("status", "covered")
        keys = [k for k in [entry.get("source"), entry.get("component"), entry.get("path"), entry.get("route")] if k]
        for key in keys:
            statuses[key] = entry
        source = entry.get("source")
        covered_by = entry.get("covered_by")
        if source and status in ROUTE_COVERED_STATUSES:
            covered.add(source)
        if isinstance(covered_by, str) and covered_by.startswith("src/") and status in ROUTE_COVERED_STATUSES:
            covered.add(covered_by)

    return covered, statuses


def audit_router_coverage(manifest: dict[str, Any]) -> dict[str, Any]:
    covered, statuses = manifest_coverage(manifest)
    routes = extract_router_routes()
    missing: list[dict[str, Any]] = []
    unresolved: list[dict[str, Any]] = []
    acknowledged: list[dict[str, Any]] = []

    for route in routes:
        source = route.get("source")
        status_entry = statuses.get(route["path"]) or statuses.get(route["component"]) or (statuses.get(source) if source else None)
        status = status_entry.get("status") if status_entry else None

        if status and status not in ROUTE_COVERED_STATUSES:
            acknowledged.append({
                **route,
                "status": status,
                "reason": status_entry.get("reason") or status_entry.get("notes"),
            })
            continue
        if status in ROUTE_COVERED_STATUSES:
            continue
        if source is None:
            unresolved.append(route)
            continue
        if source not in covered:
            missing.append(route)

    return {
        "router": rel(ROUTER),
        "routes": routes,
        "missing_manifest_coverage": missing,
        "unresolved_components": unresolved,
        "acknowledged": acknowledged,
    }


def audit_global(manifest: dict[str, Any]) -> dict[str, Any]:
    return {
        "reference_metadata": audit_reference_metadata(),
        "system_reference_coverage": audit_system_reference_coverage(),
        "reference_asset_links": audit_reference_asset_links(),
        "router_coverage": audit_router_coverage(manifest),
        "token_exports": audit_token_exports(),
    }


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


def has_token_export_drift(token_exports: dict[str, Any]) -> bool:
    for export in token_exports["exports"]:
        if (
            not export["exists"]
            or export["missing"]
            or export["extra"]
            or export["value_mismatches"]
        ):
            return True
    for drift in token_exports["duplicate_drift"]:
        if drift["left_only"] or drift["right_only"] or drift["value_mismatches"]:
            return True
    return False


def system_coverage_totals(system_coverage: dict[str, Any]) -> dict[str, int]:
    totals = {
        "spec_count": 0,
        "represented_count": 0,
        "missing_specs": 0,
        "planned_without_status": 0,
        "canonical_marked_planned": 0,
        "extra_reference_specs": 0,
    }
    for coverage in system_coverage.values():
        totals["spec_count"] += coverage["spec_count"]
        totals["represented_count"] += coverage["represented_count"]
        totals["missing_specs"] += len(coverage["missing_specs"])
        totals["planned_without_status"] += len(coverage["planned_without_status"])
        totals["canonical_marked_planned"] += len(coverage["canonical_marked_planned"])
        totals["extra_reference_specs"] += len(coverage["extra_reference_specs"])
    return totals


def global_severity(global_findings: dict[str, Any]) -> str:
    metadata = global_findings["reference_metadata"]
    system = global_findings["system_reference_coverage"]
    assets = global_findings["reference_asset_links"]
    router = global_findings["router_coverage"]
    system_totals = system_coverage_totals(system)
    if (
        metadata["broken_specs"]
        or assets["broken_html_refs"]
        or assets["broken_css_imports"]
        or router["missing_manifest_coverage"]
        or system_totals["missing_specs"]
    ):
        return "critical"
    if (
        metadata["missing_required"]
        or metadata["spec_metadata_mismatches"]
        or system_totals["planned_without_status"]
        or system_totals["canonical_marked_planned"]
        or system_totals["extra_reference_specs"]
        or router["unresolved_components"]
        or has_token_export_drift(global_findings["token_exports"])
    ):
        return "major"
    return "clean"


def render_global_md(global_findings: dict[str, Any]) -> str:
    out: list[str] = []
    metadata = global_findings["reference_metadata"]
    system = global_findings["system_reference_coverage"]
    assets = global_findings["reference_asset_links"]
    router = global_findings["router_coverage"]
    tokens = global_findings["token_exports"]
    system_totals = system_coverage_totals(system)

    out.append("## Global checks\n\n")
    out.append(f"- **Global severity**: {global_severity(global_findings)}\n")
    out.append(f"- **Broken `data-ds-spec` links**: {len(metadata['broken_specs'])}\n")
    out.append(f"- **Reference DS elements missing required attrs**: {len(metadata['missing_required'])}\n")
    out.append(f"- **Spec metadata mismatches**: {len(metadata['spec_metadata_mismatches'])}\n")
    out.append(f"- **Broken local HTML refs**: {len(assets['broken_html_refs'])}\n")
    out.append(f"- **Broken local CSS imports**: {len(assets['broken_css_imports'])}\n")
    out.append(f"- **Primitive/pattern specs represented in reference UI**: {system_totals['represented_count']} / {system_totals['spec_count']}\n")
    out.append(f"- **Primitive/pattern specs missing from reference UI**: {system_totals['missing_specs']}\n")
    out.append(f"- **Planned specs missing roadmap marker**: {system_totals['planned_without_status']}\n")
    out.append(f"- **Router routes missing manifest coverage**: {len(router['missing_manifest_coverage'])}\n")
    out.append(f"- **Router routes acknowledged by manifest status**: {len(router['acknowledged'])}\n")
    out.append(f"- **Token export files checked**: {len(tokens['exports'])}\n\n")

    if metadata["broken_specs"]:
        out.append("### Broken `data-ds-spec` links\n\n")
        for item in metadata["broken_specs"][:40]:
            out.append(f"- `{item['file']}:{item['line']}` `{item.get('name') or '<unnamed>'}` → `{item['spec']}`\n")
        if len(metadata["broken_specs"]) > 40:
            out.append(f"- … +{len(metadata['broken_specs']) - 40} more\n")
        out.append("\n")

    if metadata["missing_required"]:
        out.append("### Missing required `data-ds-*` attrs\n\n")
        for item in metadata["missing_required"][:40]:
            missing = ", ".join(f"`{m}`" for m in item["missing"])
            out.append(f"- `{item['file']}:{item['line']}` `{item.get('name') or '<unnamed>'}` missing {missing}\n")
        if len(metadata["missing_required"]) > 40:
            out.append(f"- … +{len(metadata['missing_required']) - 40} more\n")
        out.append("\n")

    if metadata["spec_metadata_mismatches"]:
        out.append("### Spec metadata mismatches\n\n")
        for item in metadata["spec_metadata_mismatches"][:40]:
            out.append(f"- `{item['file']}:{item['line']}` `{item.get('name') or '<unnamed>'}` → `{item['spec']}`")
            mismatch_text = "; ".join(
                f"{m['field']} expected `{m['expected']}` actual `{m['actual']}`"
                for m in item["mismatches"]
            )
            out.append(f" ({mismatch_text})\n")
        if len(metadata["spec_metadata_mismatches"]) > 40:
            out.append(f"- … +{len(metadata['spec_metadata_mismatches']) - 40} more\n")
        out.append("\n")

    if assets["broken_html_refs"]:
        out.append("### Broken local HTML refs\n\n")
        for item in assets["broken_html_refs"][:60]:
            out.append(f"- `{item['file']}` `{item['ref']}` → `{item['resolved']}`\n")
        if len(assets["broken_html_refs"]) > 60:
            out.append(f"- … +{len(assets['broken_html_refs']) - 60} more\n")
        out.append("\n")

    if assets["broken_css_imports"]:
        out.append("### Broken local CSS imports\n\n")
        for item in assets["broken_css_imports"][:60]:
            out.append(f"- `{item['file']}` `{item['ref']}` → `{item['resolved']}`\n")
        if len(assets["broken_css_imports"]) > 60:
            out.append(f"- … +{len(assets['broken_css_imports']) - 60} more\n")
        out.append("\n")

    for tier, coverage in system.items():
        if not (
            coverage["missing_specs"]
            or coverage["planned_without_status"]
            or coverage["canonical_marked_planned"]
            or coverage["extra_reference_specs"]
        ):
            continue
        out.append(f"### {tier.title()} reference coverage\n\n")
        out.append(f"- `{coverage['page']}` represents {coverage['represented_count']} / {coverage['spec_count']} specs")
        out.append(f" ({coverage['canonical_count']} canonical, {coverage['planned_count']} planned)\n")
        for item in coverage["missing_specs"]:
            out.append(f"- missing `{item['spec']}` (`{item['name']}`, status `{item['status']}`)\n")
        for item in coverage["planned_without_status"]:
            out.append(f"- planned spec missing roadmap marker: `{item['spec']}` (`{item['name']}`)\n")
        for item in coverage["canonical_marked_planned"]:
            out.append(f"- canonical spec marked as planned in reference UI: `{item['spec']}` (`{item['name']}`)\n")
        for spec in coverage["extra_reference_specs"]:
            out.append(f"- reference links to non-spec or out-of-band item: `{spec}`\n")
        out.append("\n")

    if router["missing_manifest_coverage"]:
        out.append("### Router routes missing manifest coverage\n\n")
        for route in router["missing_manifest_coverage"]:
            out.append(f"- `{route['path']}` uses `{route['component']}`")
            if route.get("source"):
                out.append(f" (`{route['source']}`)")
            out.append("\n")
        out.append("\n")

    if router["unresolved_components"]:
        out.append("### Router route components not resolved to source files\n\n")
        for route in router["unresolved_components"]:
            out.append(f"- `{route['path']}` uses `{route['component']}`\n")
        out.append("\n")

    if router["acknowledged"]:
        out.append("### Router routes acknowledged by manifest status\n\n")
        for route in router["acknowledged"]:
            reason = f" — {route['reason']}" if route.get("reason") else ""
            out.append(f"- `{route['path']}` `{route['component']}` status `{route['status']}`{reason}\n")
        out.append("\n")

    if has_token_export_drift(tokens):
        out.append("### Token export drift\n\n")
        for export in tokens["exports"]:
            if not (export["missing"] or export["extra"] or export["value_mismatches"] or not export["exists"]):
                continue
            out.append(f"- `{export['file']}` vs `{tokens['runtime']}`: ")
            out.append(f"{len(export['missing'])} missing, {len(export['extra'])} extra, {len(export['value_mismatches'])} value mismatches\n")
            for name in export["missing"][:8]:
                out.append(f"  - missing `{name}`\n")
            if len(export["missing"]) > 8:
                out.append(f"  - … +{len(export['missing']) - 8} more missing\n")
            for name in export["extra"][:8]:
                out.append(f"  - extra `{name}`\n")
            if len(export["extra"]) > 8:
                out.append(f"  - … +{len(export['extra']) - 8} more extra\n")
            for mismatch in export["value_mismatches"][:8]:
                out.append(f"  - `{mismatch['token']}` runtime `{mismatch['runtime']}` reference `{mismatch['reference']}`\n")
            if len(export["value_mismatches"]) > 8:
                out.append(f"  - … +{len(export['value_mismatches']) - 8} more value mismatches\n")
        for drift in tokens["duplicate_drift"]:
            if not (drift["left_only"] or drift["right_only"] or drift["value_mismatches"]):
                continue
            out.append(f"- Duplicate exports `{drift['left']}` vs `{drift['right']}`: ")
            out.append(f"{len(drift['left_only'])} left-only, {len(drift['right_only'])} right-only, {len(drift['value_mismatches'])} value mismatches\n")
        out.append("\n")

    return "".join(out)


def render_md(all_findings: list[dict[str, Any]], global_findings: dict[str, Any] | None = None) -> str:
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

    if global_findings is not None:
        out.append(render_global_md(global_findings))

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
    global_findings = None if args.surface else audit_global(manifest)

    if args.write_baseline:
        write_baseline(all_findings)
        print(f"Baseline written: {BASELINE.relative_to(REPO)}")
        return 0

    if args.enforce_baseline:
        baseline = load_baseline()
        regressions: list[tuple[str, str, str]] = []
        improvements: list[tuple[str, str, str]] = []
        blocked = False
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
            blocked = True

        if global_findings is not None:
            sev = global_severity(global_findings)
            if sev in ("critical", "major"):
                metadata = global_findings["reference_metadata"]
                system_totals = system_coverage_totals(global_findings["system_reference_coverage"])
                assets = global_findings["reference_asset_links"]
                router = global_findings["router_coverage"]
                print("\n🚫 Reference fidelity GLOBAL CHECKS FAILED — block commit:", file=sys.stderr)
                print(f"  Global severity: {sev}", file=sys.stderr)
                if metadata["broken_specs"]:
                    print(f"  Broken data-ds-spec links: {len(metadata['broken_specs'])}", file=sys.stderr)
                if metadata["missing_required"]:
                    print(f"  Reference DS elements missing required attrs: {len(metadata['missing_required'])}", file=sys.stderr)
                if metadata["spec_metadata_mismatches"]:
                    print(f"  Spec metadata mismatches: {len(metadata['spec_metadata_mismatches'])}", file=sys.stderr)
                if assets["broken_html_refs"]:
                    print(f"  Broken local HTML refs: {len(assets['broken_html_refs'])}", file=sys.stderr)
                if assets["broken_css_imports"]:
                    print(f"  Broken local CSS imports: {len(assets['broken_css_imports'])}", file=sys.stderr)
                if system_totals["missing_specs"]:
                    print(f"  Primitive/pattern specs missing from reference UI: {system_totals['missing_specs']}", file=sys.stderr)
                if system_totals["planned_without_status"]:
                    print(f"  Planned specs missing roadmap marker: {system_totals['planned_without_status']}", file=sys.stderr)
                if system_totals["canonical_marked_planned"]:
                    print(f"  Canonical specs marked as planned in reference UI: {system_totals['canonical_marked_planned']}", file=sys.stderr)
                if system_totals["extra_reference_specs"]:
                    print(f"  Reference UI links to out-of-band primitive/pattern specs: {system_totals['extra_reference_specs']}", file=sys.stderr)
                if router["missing_manifest_coverage"]:
                    print(f"  Router routes missing manifest coverage: {len(router['missing_manifest_coverage'])}", file=sys.stderr)
                if router["unresolved_components"]:
                    print(f"  Router routes with unresolved source components: {len(router['unresolved_components'])}", file=sys.stderr)
                print("\nRun  python3 .docs/design/_audits/audit-reference.py --strict", file=sys.stderr)
                print("for the full report. Fix global drift before committing.", file=sys.stderr)
                blocked = True

        if blocked:
            return 1
        return 0

    if args.json:
        payload: dict[str, Any] = {"findings": all_findings}
        if global_findings is not None:
            payload["global"] = global_findings
        json.dump(payload, sys.stdout, indent=2)
        print()
    else:
        REPORT.write_text(render_md(all_findings, global_findings))
        print(f"Report: {REPORT.relative_to(REPO)}")
        if global_findings is not None:
            print(f"  [global  ] {global_severity(global_findings)}")
        for f in all_findings:
            sev = severity(f)
            print(f"  [{sev:8s}] {f['html']}")

    if args.strict:
        bad = [f for f in all_findings if severity(f) in ("critical", "major")]
        global_bad = global_findings is not None and global_severity(global_findings) in ("critical", "major")
        if bad or global_bad:
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
