#!/usr/bin/env python3
"""
scope-modules.py — Prefix every class selector in *.module.css with the module
name, mimicking what CSS Modules do at build time. Lets us load multiple
*.module.css files as plain CSS without class-name collisions.

Run from .docs/design/reference/_shared/:
    python3 scope-modules.py

Idempotent: running twice is a no-op (already-prefixed classes are skipped).
"""
import re
from pathlib import Path

STYLES_DIR = Path(__file__).parent / "styles"

# Match `.classname` where classname is a simple identifier
# - leading char [A-Za-z_]
# - subsequent chars [A-Za-z0-9_-]
# - bounded by a non-name char (or end of string)
CLASS_RE = re.compile(r"\.([A-Za-z_][A-Za-z0-9_-]*)\b")

# Lines we must NOT touch (they contain `.foo` patterns that aren't selectors)
SKIP_PREFIXES = ("@import", "@font-face", "src:", "url(", "//")


def prefix_line(line: str, mod: str) -> str:
    stripped = line.lstrip()
    if stripped.startswith(SKIP_PREFIXES):
        return line
    # Skip lines inside @import/url strings — quick heuristic: if line has
    # quoted string with .css/.woff/.svg extension, skip
    if re.search(r'["\'].*?\.(css|woff2?|svg|png|jpg|ttf|eot)["\']', line):
        return line

    def sub(m: re.Match) -> str:
        name = m.group(1)
        # Already-prefixed (idempotent)
        if name.startswith(f"{mod}_"):
            return f".{name}"
        return f".{mod}_{name}"

    return CLASS_RE.sub(sub, line)


def strip_comments_and_process(text: str, mod: str) -> str:
    """Walk the file, skipping /* ... */ comments so we don't prefix words inside them."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        # Comment start
        if text[i:i+2] == "/*":
            end = text.find("*/", i + 2)
            if end == -1:
                out.append(text[i:])
                break
            out.append(text[i:end+2])
            i = end + 2
            continue
        # String literal — skip to matching close (rough but adequate for CSS)
        if text[i] in ("'", '"'):
            quote = text[i]
            j = i + 1
            while j < n and text[j] != quote:
                if text[j] == "\\":
                    j += 2
                    continue
                j += 1
            out.append(text[i:j+1])
            i = j + 1
            continue
        # Regular char — accumulate until next comment/string boundary, then prefix
        j = i
        while j < n and text[j] not in ("/", "'", '"'):
            j += 1
        # If we stopped on '/' that's NOT a comment start, include it
        while j < n and text[j] == "/" and text[j:j+2] != "/*":
            j += 1
            while j < n and text[j] not in ("/", "'", '"'):
                j += 1
        chunk = text[i:j]
        out.append(prefix_chunk(chunk, mod))
        i = j
    return "".join(out)


def prefix_chunk(chunk: str, mod: str) -> str:
    def sub(m: re.Match) -> str:
        name = m.group(1)
        if name.startswith(f"{mod}_"):
            return f".{name}"
        return f".{mod}_{name}"
    return CLASS_RE.sub(sub, chunk)


def process(path: Path) -> None:
    mod = path.stem.replace(".module", "")
    raw = path.read_text()
    new = strip_comments_and_process(raw, mod)
    if new != raw:
        path.write_text(new)
        print(f"prefixed: {path.name} (module: {mod})")
    else:
        print(f"unchanged: {path.name}")


def main() -> None:
    for css in sorted(STYLES_DIR.glob("*.module.css")):
        process(css)


if __name__ == "__main__":
    main()
