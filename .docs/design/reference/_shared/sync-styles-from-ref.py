#!/usr/bin/env python3
"""
sync-styles-from-ref.py — regenerate every _shared stylesheet from app sources
at a git ref (default: public/main, i.e. PRODUCTION).

The reference library renders with these mirrors; if they are hand-curated or
synced from a dev branch, every surface render drifts from the shipped app even
when the markup is structurally perfect. This script makes the CSS layer
production-by-construction:

  - _shared/styles/<Name>.module.css  ← git show REF:src/**/<Name>.module.css,
    relative @import lines stripped, classes prefixed `<Name>_` (same transform
    as scope-modules.py)
  - _shared/styles/design-tokens.css and _shared/tokens.css ← verbatim
    git show REF:src/styles/design-tokens.css
  - _shared/fonts.css ← git show REF:src/styles/fonts.css with font URLs
    rewritten from /src/assets/fonts/ to ./fonts/ (binaries copied as needed)

Mirrors with no source file at the ref are reported, never deleted.

Usage:
  python3 .docs/design/reference/_shared/sync-styles-from-ref.py [--ref REF] [--check]

--check: exit 1 if any mirror differs from what the ref would generate
         (used by the fidelity auditor / CI); writes nothing.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

SHARED = Path(__file__).resolve().parent
STYLES = SHARED / "styles"
REPO = SHARED.parents[3]

RE_RELATIVE_IMPORT = re.compile(r'^\s*@import\s+["\']\.\.?/.*["\'];\s*$', re.MULTILINE)
RE_CLASS = re.compile(r"\.([A-Za-z_][A-Za-z0-9_-]*)\b")
SKIP_PREFIXES = ("@import", "@font-face", "src:", "url(")

# Mirrors whose module name is not the file stem (src/styles/*.module.css files
# use their stem as the scoped prefix too, so no special cases today).
VERBATIM = {
    "design-tokens.css": "src/styles/design-tokens.css",
}


def git_show(ref: str, path: str) -> str | None:
    proc = subprocess.run(["git", "-C", str(REPO), "show", f"{ref}:{path}"],
                          capture_output=True, text=True)
    return proc.stdout if proc.returncode == 0 else None


def git_show_bytes(ref: str, path: str) -> bytes | None:
    proc = subprocess.run(["git", "-C", str(REPO), "show", f"{ref}:{path}"],
                          capture_output=True)
    return proc.stdout if proc.returncode == 0 else None


def ref_src_files(ref: str) -> list[str]:
    proc = subprocess.run(["git", "-C", str(REPO), "ls-tree", "-r", "--name-only", ref, "--", "src"],
                          capture_output=True, text=True)
    if proc.returncode != 0:
        raise SystemExit(f"git ls-tree failed for {ref!r}: {proc.stderr.strip()}")
    return proc.stdout.split()


def prefix_css(css: str, mod: str) -> str:
    """Prefix class selectors with `{mod}_`, comment- and string-aware —
    the same transform scope-modules.py applies."""
    out: list[str] = []
    i, n = 0, len(css)
    while i < n:
        if css[i:i + 2] == "/*":
            end = css.find("*/", i + 2)
            end = n if end == -1 else end + 2
            out.append(css[i:end])
            i = end
            continue
        if css[i] in ("'", '"'):
            quote = css[i]
            j = i + 1
            while j < n and css[j] != quote:
                j += 2 if css[j] == "\\" else 1
            out.append(css[i:j + 1])
            i = j + 1
            continue
        j = i
        while j < n and css[j] not in ("/", "'", '"'):
            j += 1
        while j < n and css[j] == "/" and css[j:j + 2] != "/*":
            j += 1
            while j < n and css[j] not in ("/", "'", '"'):
                j += 1
        chunk = css[i:j]
        out.append(RE_CLASS.sub(
            lambda m: f".{m.group(1)}" if m.group(1).startswith(f"{mod}_") else f".{mod}_{m.group(1)}",
            chunk,
        ))
        i = j
    return "".join(out)


def generate_mirror(ref: str, src_path: str, mod: str) -> str:
    raw = git_show(ref, src_path)
    assert raw is not None
    raw = RE_RELATIVE_IMPORT.sub("", raw)
    return prefix_css(raw, mod).lstrip("\n")


def sync_fonts(ref: str, check: bool) -> list[str]:
    """fonts.css with URLs rewritten to _shared/fonts/; returns drift notes."""
    notes: list[str] = []
    src = git_show(ref, "src/styles/fonts.css")
    if src is None:
        return ["fonts: src/styles/fonts.css missing at ref"]
    rewritten = src.replace("url('/src/assets/fonts/", "url('./fonts/")
    rewritten = rewritten.replace('url("/src/assets/fonts/', 'url("./fonts/')
    target = SHARED / "fonts.css"
    current = target.read_text() if target.exists() else ""
    if current != rewritten:
        if check:
            notes.append("fonts.css differs from ref")
        else:
            target.write_text(rewritten)
    # Ensure referenced binaries exist.
    fonts_dir = SHARED / "fonts"
    for name in re.findall(r"\./fonts/([A-Za-z0-9._-]+\.woff2?)", rewritten):
        dest = fonts_dir / name
        if not dest.exists():
            blob = git_show_bytes(ref, f"src/assets/fonts/{name}")
            if blob is None:
                notes.append(f"fonts: missing binary {name} (not at ref either)")
            elif not check:
                fonts_dir.mkdir(exist_ok=True)
                dest.write_bytes(blob)
            else:
                notes.append(f"fonts: binary {name} missing locally")
    return notes


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", default="public/main")
    ap.add_argument("--check", action="store_true",
                    help="report drift, write nothing, exit 1 on any drift")
    args = ap.parse_args()

    src_files = ref_src_files(args.ref)
    by_name: dict[str, list[str]] = {}
    for p in src_files:
        if p.endswith(".module.css") or p.endswith(".css"):
            by_name.setdefault(Path(p).name, []).append(p)

    drift: list[str] = []
    unmatched: list[str] = []
    ambiguous: list[str] = []
    synced = 0

    for mirror in sorted(STYLES.glob("*.css")):
        name = mirror.name
        if name in VERBATIM:
            want = git_show(args.ref, VERBATIM[name]) or ""
            if mirror.read_text() != want:
                drift.append(name)
                if not args.check:
                    mirror.write_text(want)
            synced += 1
            continue
        candidates = by_name.get(name, [])
        if not candidates:
            unmatched.append(name)
            continue
        if len(candidates) > 1:
            # Prefer the candidate whose raw class set overlaps the mirror most.
            mirror_classes = set(RE_CLASS.findall(mirror.read_text()))
            def overlap(path: str) -> int:
                mod = name.replace(".module.css", "").replace(".css", "")
                raw = set(RE_CLASS.findall(git_show(args.ref, path) or ""))
                scoped = {c if c.startswith(f"{mod}_") else f"{mod}_{c}" for c in raw}
                return len(scoped & mirror_classes)
            candidates = sorted(candidates, key=overlap, reverse=True)
            ambiguous.append(f"{name}: chose {candidates[0]} over {candidates[1:]}")
        mod = name.replace(".module.css", "").replace(".css", "")
        want = generate_mirror(args.ref, candidates[0], mod)
        if mirror.read_text() != want:
            drift.append(name)
            if not args.check:
                mirror.write_text(want)
        synced += 1

    # tokens.css at _shared root is the second token export.
    root_tokens = SHARED / "tokens.css"
    if root_tokens.exists():
        want = git_show(args.ref, "src/styles/design-tokens.css") or ""
        if root_tokens.read_text() != want:
            drift.append("tokens.css")
            if not args.check:
                root_tokens.write_text(want)

    drift += sync_fonts(args.ref, args.check)

    verb = "drifted" if args.check else "regenerated"
    print(f"{synced} mirrors checked against {args.ref}; {len(drift)} {verb}")
    for d in drift:
        print(f"  ~ {d}")
    if unmatched:
        print(f"{len(unmatched)} mirrors have no source at {args.ref} (left untouched):")
        for u in unmatched:
            print(f"  ? {u}")
    for a in ambiguous:
        print(f"  ! {a}")

    if args.check and drift:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
