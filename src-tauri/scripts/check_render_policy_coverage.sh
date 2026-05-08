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
if "render_ability_data(surface, data, &provenance)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: AbilityResponseJson.data is not built from render_ability_data(surface, data, &provenance)")
if not re.search(r"BridgeSurface::McpTool\s*\|\s*BridgeSurface::McpToolDetail\s*=>\s*\{?\s*render_mcp_ability_data_with_authoritative_claims", bridge, re.S):
    violations.append("src-tauri/src/bridges/types.rs: MCP surfaces do not call the authoritative ability-data redactor")
if "ActionDb::open_readonly()" not in bridge or "render_mcp_ability_data_for_surface_with_provenance(&db, data, provenance)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: MCP ability data redactor does not pass ActionDb and provenance into render_mcp_ability_data_for_surface_with_provenance")
if "render_mcp_ability_data_without_claim_lookup(data)" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: MCP ability data redactor lacks fail-closed no-claim-lookup fallback")
if not re.search(r"BridgeSurface::TauriApp\s*\|\s*BridgeSurface::Worker\s*\|\s*BridgeSurface::Eval\s*=>\s*data", bridge):
    violations.append("src-tauri/src/bridges/types.rs: non-MCP surfaces must pass ability data through unchanged")
if "string leaf has exactly three possible outcomes" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor lacks deny-by-default documentation")
if "load_claim_by_id(db.conn_ref(), claim_id)" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor does not reload authoritative claims")
if "MCP_ABILITY_METADATA_STRING_ALLOWLIST" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor lacks explicit path-scoped metadata allowlist")
if "render_mcp_ability_metadata_string(path, &text)" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: string leaves do not flow through the path-scoped metadata validator")
for forbidden in [
    "fn metadata_key_for_path",
    "fn is_identifier_metadata_key",
    "fn is_timestamp_metadata_key",
    "fn matches_meeting_metadata_name_path",
]:
    if forbidden in service:
        violations.append(f"src-tauri/src/services/sensitivity.rs: key-name metadata allowlist remains (`{forbidden}`)")
if re.search(r"matches!\s*\(\s*key\s*,", service):
    violations.append("src-tauri/src/services/sensitivity.rs: metadata allowlist still matches by leaf key name")
if re.search(r"minimal_policy_claim\s*\(\s*tagged\.sensitivity", service):
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor still constructs synthetic policy claims from DTO sensitivity")
if "claim.sensitivity != tagged.sensitivity" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor does not fail closed on DTO/stored sensitivity mismatch")
if "stored_text != tagged.text" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: MCP ability data redactor does not fail closed on DTO/stored text mismatch")
if "renderable_from_decision(&claim, &stored_text" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer does not render authoritative stored claim text")
if "claim.claim_state != ClaimState::Active" not in service or "claim.surfacing_state != SurfacingState::Active" not in service:
    violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer does not require active surfaced stored claims")

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
    if "verify_and_render_authoritative_claim(tagged, RenderSurface::McpTool, load_claim)" not in tagged_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer does not use the shared authoritative claim verifier")
    if re.search(r"\bobject\s*:", tagged_renderer) or "object.insert" in tagged_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer mutates the original object instead of stripping siblings")
    if "safe_tagged_mcp_claim_text_object(rendered)" not in tagged_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: tagged claim renderer does not return the minimal safe allowlist object")

static_claim_renderer = function_source(service, "render_mcp_claim_text_for_surface")
if not static_claim_renderer:
    violations.append("src-tauri/src/services/sensitivity.rs: missing render_mcp_claim_text_for_surface")
else:
    if "verify_and_render_authoritative_claim(" not in static_claim_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: static MCP claim renderer does not use the shared authoritative claim verifier")
    if "renderable_claim_text_with_value" in static_claim_renderer:
        violations.append("src-tauri/src/services/sensitivity.rs: static MCP claim renderer still renders DTO text directly")

authoritative_renderer = function_source(service, "verify_and_render_authoritative_claim")
if not authoritative_renderer:
    violations.append("src-tauri/src/services/sensitivity.rs: missing shared authoritative MCP claim verifier")
else:
    for required in [
        "let claim = load_claim(&tagged.claim_id)?;",
        "let stored_text = stored_mcp_claim_text(&claim, &tagged.claim_id, tagged.stored_projection)?;",
        "claim.sensitivity != tagged.sensitivity",
        "stored_text != tagged.text",
        "renderable_from_decision(&claim, &stored_text, surface, decision)",
    ]:
        if required not in authoritative_renderer:
            violations.append(f"src-tauri/src/services/sensitivity.rs: shared authoritative MCP claim verifier missing `{required}`")

safe_object = function_source(service, "safe_tagged_mcp_claim_text_object")
if not safe_object:
    violations.append("src-tauri/src/services/sensitivity.rs: missing minimal tagged-object allowlist helper")
else:
    for forbidden in ["source_text", "sourceSummary", "evidenceText", "rawText", "quote", "claim_id", "sensitivity", "originating_actor"]:
        if f'"{forbidden}"' in safe_object:
            violations.append(f"src-tauri/src/services/sensitivity.rs: minimal tagged-object allowlist preserves forbidden field `{forbidden}`")

ABILITY_ROOTS = [Path("src-tauri/abilities-runtime/src/abilities")]

agent_abilities = []
for root in ABILITY_ROOTS:
  for path in root.rglob("*.rs"):
    text = path.read_text()
    for match in re.finditer(r"#\[ability\((.*?)\)\]", text, re.S):
        block = match.group(1)
        name = re.search(r'name\s*=\s*"([^"]+)"', block)
        actors = re.search(r"allowed_actors\s*=\s*\[([^\]]*)\]", block, re.S)
        if actors and re.search(r"\bAgent\b", actors.group(1)):
            agent_abilities.append((str(path), name.group(1) if name else "<unknown>"))

if agent_abilities and "render_mcp_ability_data_with_authoritative_claims(data, provenance)" not in bridge:
    for path, name in agent_abilities:
        violations.append(f"{path}: Agent-allowed ability `{name}` would bypass the MCP ability data redactor")

if "impl Serialize for AbilityResponseJson" not in bridge or "include_diagnostics" not in bridge:
    violations.append("src-tauri/src/bridges/types.rs: AbilityResponseJson serialization does not gate diagnostics by surface")
if not re.search(r"rendered_provenance\.surface,\s*BridgeSurface::McpTool\s*\|\s*BridgeSurface::McpToolDetail", bridge, re.S):
    violations.append("src-tauri/src/bridges/types.rs: serialized MCP ability responses do not omit diagnostics for MCP surfaces")

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

ability_output_violations="$(
  python3 - <<'PY'
from pathlib import Path
import re

ROOTS = [
    Path("src-tauri/abilities-runtime/src/abilities"),
    Path("src-tauri/abilities-runtime/src/types.rs"),
]
source_by_path = {path: path.read_text() for root in ROOTS for path in ([root] if root.is_file() else root.rglob("*.rs"))}
combined = "\n".join(source_by_path.values())

SAFE_STRING_FIELDS = {
    "EntityContextEntry": {
        "id": "identifier metadata",
        "entity_type": "enum metadata",
        "entity_id": "identifier metadata",
        "title": "claim/provenance-attested",
        "content": "claim/provenance-attested",
        "created_at": "timestamp metadata",
        "updated_at": "timestamp metadata",
    },
    "MeetingSummary": {
        "id": "identifier metadata",
        "title": "meeting title metadata",
        "starts_at": "timestamp metadata",
        "ends_at": "timestamp metadata",
    },
    "MeetingAttendee": {
        "name": "attendee name metadata",
        "email": "fail-closed on MCP",
        "person_id": "identifier metadata",
        "account_id": "identifier metadata",
        "domain": "fail-closed on MCP",
    },
    "BriefSubjectRef": {
        "kind": "enum metadata",
        "id": "identifier metadata",
    },
    "Topic": {
        "title": "claim/provenance-attested",
        "detail": "claim/provenance-attested",
    },
    "AttendeeContext": {
        "attendee": "claim/provenance-attested",
        "context": "claim/provenance-attested",
    },
    "OpenLoop": {
        "description": "claim/provenance-attested",
        "owner": "claim/provenance-attested",
    },
    "ChangeMarker": {
        "description": "claim/provenance-attested",
    },
    "SuggestedOutcome": {
        "outcome": "claim/provenance-attested",
        "rationale": "claim/provenance-attested",
    },
    "BriefTemporalScope": {
        "occurred_at": "timestamp metadata",
        "window_start": "timestamp metadata",
        "window_end": "timestamp metadata",
    },
}

NESTED_OUTPUT_STRUCTS = {
    "MeetingBrief": [
        "MeetingSummary",
        "Topic",
        "AttendeeContext",
        "OpenLoop",
        "ChangeMarker",
        "SuggestedOutcome",
    ],
    "MeetingSummary": ["MeetingAttendee"],
    "Topic": ["BriefSubjectRef", "BriefTemporalScope"],
    "AttendeeContext": ["BriefSubjectRef", "BriefTemporalScope"],
    "OpenLoop": ["BriefSubjectRef", "BriefTemporalScope"],
    "ChangeMarker": ["BriefSubjectRef", "BriefTemporalScope"],
    "SuggestedOutcome": ["BriefSubjectRef", "BriefTemporalScope"],
}

EXPECTED_AGENT_OUTPUTS = {
    "get_entity_context": "EntityContextEntry",
    "prepare_meeting": "MeetingBrief",
}

SAFE_WRAPPERS = ("RenderableMcpClaimText", "RenderableMcpEntityName")

def normalize_type(type_text: str) -> str:
    value = re.sub(r"\s+", " ", type_text.strip())
    value = value.replace("crate::types::", "")
    value = value.replace("synthesis::", "")
    while True:
        match = re.fullmatch(r"(Vec|Option)<(.+)>", value)
        if not match:
            break
        value = match.group(2).strip()
    return value.split("::")[-1]

def raw_string_type(type_text: str) -> bool:
    text = re.sub(r"\s+", "", type_text)
    return text in {"String", "Option<String>", "Vec<String>"}

def struct_body(name: str):
    marker = f"pub struct {name}"
    start = combined.find(marker)
    if start == -1:
        marker = f"struct {name}"
        start = combined.find(marker)
    if start == -1:
        return None
    brace = combined.find("{", start)
    if brace == -1:
        return None
    depth = 0
    for index in range(brace, len(combined)):
        char = combined[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return combined[brace + 1:index]
    return None

def enum_body(name: str):
    marker = f"pub enum {name}"
    start = combined.find(marker)
    if start == -1:
        return None
    brace = combined.find("{", start)
    if brace == -1:
        return None
    depth = 0
    for index in range(brace, len(combined)):
        char = combined[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return combined[brace + 1:index]
    return None

def struct_fields(name: str):
    body = struct_body(name)
    if body is None:
        return []
    return re.findall(r"pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,\n]+)", body)

def enum_string_fields(name: str):
    body = enum_body(name)
    if body is None:
        return []
    return re.findall(r"([A-Za-z_][A-Za-z0-9_]*)\s*:\s*String", body)

def safe_string_field(struct_name: str, field_name: str, type_text: str) -> bool:
    if any(wrapper in type_text for wrapper in SAFE_WRAPPERS):
        return True
    return field_name in SAFE_STRING_FIELDS.get(struct_name, {})

def inspect_struct(struct_name: str, seen: set[str], violations: list[str]):
    if struct_name in seen:
        return
    seen.add(struct_name)
    fields = struct_fields(struct_name)
    if not fields and struct_name != "BriefTemporalScope":
        violations.append(f"{struct_name}: output struct not found for Agent-allowed ability audit")
        return
    for field_name, type_text in fields:
        if raw_string_type(type_text) and not safe_string_field(struct_name, field_name, type_text):
            violations.append(
                f"{struct_name}.{field_name}: raw `{type_text.strip()}` is not RenderableMcpClaimText, RenderableMcpEntityName, or an audited metadata/fail-closed field"
            )
        nested = normalize_type(type_text)
        if nested in SAFE_STRING_FIELDS or nested in NESTED_OUTPUT_STRUCTS:
            inspect_struct(nested, seen, violations)
    for field_name in enum_string_fields(struct_name):
        if field_name not in SAFE_STRING_FIELDS.get(struct_name, {}):
            violations.append(f"{struct_name}.{field_name}: enum string field is not audited for MCP")
    for nested in NESTED_OUTPUT_STRUCTS.get(struct_name, []):
        inspect_struct(nested, seen, violations)

def agent_ability_outputs():
    outputs = {}
    for path in Path("src-tauri/abilities-runtime/src/abilities").rglob("*.rs"):
        text = path.read_text()
        for match in re.finditer(r"#\[ability\((.*?)\)\]\s*pub\s+async\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)", text, re.S):
            block = match.group(1)
            name_match = re.search(r'name\s*=\s*"([^"]+)"', block)
            actors = re.search(r"allowed_actors\s*=\s*\[([^\]]*)\]", block, re.S)
            if not actors or not re.search(r"\bAgent\b", actors.group(1)):
                continue
            signature = text[match.end(): text.find("{", match.end())]
            output = re.search(r"->\s*AbilityResult<(.+)>\s*$", signature.strip(), re.S)
            ability_name = name_match.group(1) if name_match else match.group(2)
            outputs[ability_name] = normalize_type(output.group(1)) if output else "<unknown>"
    return outputs

violations = []
outputs = agent_ability_outputs()
if outputs != EXPECTED_AGENT_OUTPUTS:
    violations.append(f"Agent-allowed ability output set changed: expected {EXPECTED_AGENT_OUTPUTS}, found {outputs}")

for ability_name, output_type in outputs.items():
    inspect_struct(output_type, set(), violations)

synthetic_violations = []
if safe_string_field("SyntheticAgentOutput", "text", "String"):
    synthetic_violations.append("synthetic raw String text field unexpectedly passed")
if not synthetic_violations:
    synthetic_violations.append("SyntheticAgentOutput.text: raw `String` is not RenderableMcpClaimText, RenderableMcpEntityName, or an audited metadata/fail-closed field")
if not any("SyntheticAgentOutput.text" in item for item in synthetic_violations):
    violations.append("synthetic deny-by-default lint regression did not catch raw String text")

print("\n".join(dict.fromkeys(violations)))
PY
)"

if [[ -n "$ability_output_violations" ]]; then
  cat >&2 <<'MSG'
DOS-412 Agent ability output coverage failed.

Every Agent-allowed ability output must have its string fields classified as
claim/provenance-attested, RenderableMcpClaimText, RenderableMcpEntityName,
explicit metadata, or fail-closed-on-MCP. New raw String text fields must fail
this lint until audited.

Violations:
MSG
  echo "$ability_output_violations" >&2
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
