#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! rg -n "render_policy_for_surface" src-tauri/src/services/sensitivity.rs >/dev/null; then
  echo "DOS-412 render policy helper missing from src-tauri/src/services/sensitivity.rs" >&2
  exit 1
fi

if ! rg -n "render_mcp_ability_data_for_surface" src-tauri/src/services/sensitivity.rs >/dev/null; then
  echo "DOS-412 MCP ability data redactor missing from src-tauri/src/services/sensitivity.rs" >&2
  exit 1
fi

if ! rg -n "render_mcp_ability_data_for_surface" src-tauri/src/bridges/types.rs >/dev/null; then
  echo "DOS-412 MCP ability data redactor is not wired through src-tauri/src/bridges/types.rs" >&2
  exit 1
fi

ability_redactor_violations="$(
  python3 - <<'PY'
from pathlib import Path
import re

bridge = Path("src-tauri/src/bridges/types.rs").read_text()
service = Path("src-tauri/src/services/sensitivity.rs").read_text()

violations = []
if "fn render_ability_data(" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: missing render_ability_data bridge hook")
if "render_ability_data(surface, data)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: AbilityResponseJson.data is not built from render_ability_data(surface, data)")
if not re.search(r"BridgeSurface::McpTool\s*\|\s*BridgeSurface::McpToolDetail\s*=>\s*\{?\s*render_mcp_ability_data_with_authoritative_claims", bridge, re.S):
    violations.append("src-tauri/src/bridges/types.rs: MCP surfaces do not call the authoritative ability-data redactor")
if "ActionDb::open_readonly()" not in bridge or "render_mcp_ability_data_for_surface(&db, data)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: MCP ability data redactor does not pass an ActionDb into render_mcp_ability_data_for_surface")
if "render_mcp_ability_data_without_claim_lookup(data)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: MCP ability data redactor lacks fail-closed no-claim-lookup fallback")
if not re.search(r"BridgeSurface::TauriApp\s*\|\s*BridgeSurface::Worker\s*\|\s*BridgeSurface::Eval\s*=>\s*data", bridge):
    violations.append("src-tauri/src/bridges/types.rs: non-MCP surfaces must pass ability data through unchanged")
if "Tagged carrier objects use design A" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor lacks design-A fail-closed documentation")
if "load_claim_by_id(db.conn_ref(), claim_id)" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor does not reload authoritative claims")
if re.search(r"minimal_policy_claim\s*\(\s*tagged\.sensitivity", service):
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor still constructs synthetic policy claims from DTO sensitivity")
if "claim.sensitivity != tagged.sensitivity" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor does not fail closed on DTO/stored sensitivity mismatch")

def function_source(source: str, name: str) -> str:
    marker = f"fn {name}"
    start = source.find(marker)
    if start == -1:
        return ""
    brace = source.find("{", start)
    if brace == -1:
        return source[start:]
    depth = 0
    for index in range(brace, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[start:index + 1]
    return source[start:]

tagged_renderer = function_source(service, "render_tagged_mcp_claim_text")
if not tagged_renderer:
    violations.append("src-tauri/src/services/sensitivity.rs: missing render_tagged_mcp_claim_text")
else:
    if re.search(r"\bobject\s*:", tagged_renderer) or "object.insert" in tagged_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer mutates the original object instead of stripping siblings")
    if "safe_tagged_mcp_claim_text_object(rendered)" not in tagged_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer does not return the minimal safe allowlist object")

safe_object = function_source(service, "safe_tagged_mcp_claim_text_object")
if not safe_object:
    violations.append("src-tauri/src/services/sensitivity.rs: missing minimal tagged-object allowlist helper")
else:
    for forbidden in ["source_text", "sourceSummary", "evidenceText", "rawText", "quote", "claim_id", "sensitivity", "originating_actor"]:
        if f'"{forbidden}"' in safe_object:
            violations.append(f"src-tauri/src/services/sensitivity.rs: minimal tagged-object allowlist preserves forbidden field `{forbidden}`")

agent_abilities = []
for path in Path("src-tauri/src/abilities").rglob("*.rs"):
    text = path.read_text()
    for match in re.finditer(r"#\[ability\((.*?)\)\]", text, re.S):
        block = match.group(1)
        name = re.search(r'name\s*=\s*"([^"]+)"', block)
        actors = re.search(r"allowed_actors\s*=\s*\[([^\]]*)\]", block, re.S)
        if actors and re.search(r"\bAgent\b", actors.group(1)):
            agent_abilities.append((str(path), name.group(1) if name else "<unknown>"))

if agent_abilities and "render_mcp_ability_data_with_authoritative_claims(data)" not in bridge:
    for path, name in agent_abilities:
        violations.append(f"{path}: Agent-allowed ability `{name}` would bypass the MCP ability data redactor")

print("\n".join(violations))
PY
)"

if [[ -n "$ability_redactor_violations" ]]; then
  cat >&2 <<'MSG'
DOS-412 MCP ability data coverage failed.

Every Agent-allowed registry ability reaches MCP through the bridge-level
render_mcp_ability_data_for_surface hook. Tauri/worker/eval surfaces must stay
raw.

Violations:
MSG
  echo "$ability_redactor_violations" >&2
  exit 1
fi

violations="$(
  rg -n --pcre2 \
    "(claim\\.text|SELECT\\s+[^;]*\\btext\\b[^;]*\\bFROM\\s+intelligence_claims|source\\.text|source_text)" \
    src-tauri/src/commands src-tauri/src/mcp \
    -g '*.rs' \
    | rg -v "dos412-render-policy-covered|renderable_claim_text|RenderableClaimText|reveal_claim_text_for_tauri" \
    || true
)"

if [[ -n "$violations" ]]; then
  cat >&2 <<'MSG'
DOS-412 render policy coverage failed.

Claim-derived text leaving commands/* or mcp/ must be wrapped through
services::sensitivity::{renderable_claim_text, renderable_claim_text_with_value,
reveal_claim_text_for_tauri} or be explicitly marked with a
dos412-render-policy-covered justification.

Violations:
MSG
  echo "$violations" >&2
  exit 1
fi

if ! rg -n "render_mcp_static_(text|json)_for_surface" src-tauri/src/mcp/main.rs >/dev/null; then
  echo "DOS-412 MCP static surface helpers missing from src-tauri/src/mcp/main.rs" >&2
  exit 1
fi

mcp_violations="$(
  python3 - <<'PY'
from pathlib import Path
import re

source = Path("src-tauri/src/mcp/main.rs").read_text()
lines = source.splitlines()

text_fields = {
    "actions",
    "briefing",
    "content",
    "context",
    "description",
    "emails",
    "intelligence_summary",
    "open_actions",
    "schedule",
    "snippet",
    "summary",
    "text",
    "title",
}
allowed_markers = (
    "render_mcp_static_text_for_surface",
    "render_mcp_static_json_for_surface",
    "mcp_entity_summary",
    "dos412-render-policy-covered",
)
declaration_markers = ("Option<", "Vec<", "String", "serde_json::Value")

violations = []

for idx, line in enumerate(lines, start=1):
    stripped = line.strip()
    if not stripped or stripped.startswith("//"):
        continue

    match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.*)", stripped)
    if not match:
        continue
    field, rhs = match.groups()
    if field not in text_fields:
        continue
    if any(marker in stripped for marker in declaration_markers):
        continue
    if "RequestContext<" in stripped or re.search(r":\s*&\[", stripped):
        continue

    block = stripped
    depth = stripped.count("(") + stripped.count("[") + stripped.count("{")
    depth -= stripped.count(")") + stripped.count("]") + stripped.count("}")
    next_idx = idx
    while next_idx < len(lines) and (not block.rstrip().endswith(",") or depth > 0):
        next_idx += 1
        continuation = lines[next_idx - 1].strip()
        block += "\n" + continuation
        depth += continuation.count("(") + continuation.count("[") + continuation.count("{")
        depth -= continuation.count(")") + continuation.count("]") + continuation.count("}")
        if next_idx - idx > 80:
            break

    if any(marker in block for marker in allowed_markers):
        continue
    if re.search(r":\s*(None|Some\(|Vec::new\(|\"|format!\()", stripped):
        continue
    if re.search(r":\s*(title|match_snippet|open_actions|results),?$", stripped):
        continue
    violations.append(f"src-tauri/src/mcp/main.rs:{idx}:{stripped}")

raw_sensitive_patterns = [
    (r"\bm\.chunk_text\b", "semantic chunk text"),
    (r"\bexecutive_assessment\b", "entity intelligence summary"),
    (r"\ba\.title\.clone\(\)", "action title"),
    (r"\bprep_context_json\b", "meeting prep JSON"),
]
for pattern, label in raw_sensitive_patterns:
    for match in re.finditer(pattern, source):
        line_no = source.count("\n", 0, match.start()) + 1
        line = lines[line_no - 1].strip()
        surrounding = "\n".join(lines[max(0, line_no - 8):min(len(lines), line_no + 8)])
        if any(marker in surrounding for marker in allowed_markers):
            continue
        if (
            "SELECT " in line
            or "FROM " in line
            or "LEFT JOIN" in line
            or line.startswith("OR ")
            or "mt.summary" in line
        ):
            continue
        if "raw_chunk" in line and "m.chunk_text" in line:
            continue
        if "legacy_intel" in line or ".and_then(|i| i.executive_assessment)" in line:
            continue
        if "prep_context_json" in line and ("SELECT" in line or "read" in line):
            continue
        violations.append(f"src-tauri/src/mcp/main.rs:{line_no}:{label}: {line}")

print("\n".join(dict.fromkeys(violations)))
PY
)"

if [[ -n "$mcp_violations" ]]; then
  cat >&2 <<'MSG'
DOS-412 MCP static surface coverage failed.

MCP static tools that return text-bearing DTO fields must route those fields
through render_mcp_static_text_for_surface/render_mcp_static_json_for_surface
or carry an explicit dos412-render-policy-covered justification.

Violations:
MSG
  echo "$mcp_violations" >&2
  exit 1
fi

echo "DOS-412 render policy coverage OK"
