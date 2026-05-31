---
status: spec:ready
date: 2026-05-10
related_adrs: [0102, 0111, 0129]
open_questions: see ./INDEX.md (routed to W1-D, W0-D, W1-C L0 Prep)
---

# Ability-Surface Inventory Format

## Context

DOS-546 needs one machine-readable inventory that downstream generators can use to
register the same DailyOS ability across three surfaces:

- WordPress Abilities API registrations.
- MCP server tool registrations.
- SurfaceClient policy and introspection.

The inventory is not a replacement for the runtime ability descriptor. It is the
surface-facing catalog entry that binds an existing ability contract to concrete
exposure rules, copy, permissions, and composition behavior.

ADR-0102 establishes abilities as the runtime contract and makes registry-backed
invocation the path for synthesized outputs. ADR-0111 establishes that surfaces
construct invocation context and call the shared registry instead of hand-rolling
capability paths. ADR-0129, as referenced by DOS-546, adds WordPress Studio as a
primary composable surface and requires the inventory to accommodate WP-side
ability registration through the WP Abilities API and MCP Adapter.

This document defines the canonical inventory shape for Phase 0. The shape is
strict enough for CI and generators, but small enough that ability authors can
fill it out by hand while the first WordPress Studio spike is still proving the
surface.

The inventory also extends the existing repository content gate to ability
descriptions. A description is model-facing, user-facing, and generator-facing
copy. It must therefore be scanned with the same PII blocklist and
internal-vocabulary rules that apply to committed source, fixtures, and release
copy.

## Schema

### Canonical TypeScript Interface

```ts
export type AbilityActor =
  | "user"
  | "runtime"
  | "mcp_client"
  | "surface_client";

export type AbilityCategory =
  | "read"
  | "transform"
  | "publish"
  | "maintenance";

export type McpExposure =
  | "none"
  | "metadata_only"
  | "invocable";

export type IdempotencyClass =
  | "idempotent"
  | "safe_retry"
  | "side_effect";

export type CompositionKind =
  | { produces_blocks: false; block_types: [] }
  | { produces_blocks: true; block_types: CompositionBlockType[] };

export type CompositionBlockType =
  | "account_overview"
  | "claim_summary"
  | "evidence_list"
  | "health_snapshot"
  | "relationship_map"
  | "risk_callout"
  | "action_list"
  | "markdown_document"
  | "custom";

export interface AbilitySurfaceInventoryEntry {
  name: string;
  description: string;
  category: AbilityCategory;
  annotations: Record<string, string | number | boolean | null>;
  wp_permission: string;
  allowed_actors: AbilityActor[];
  required_scopes: string[];
  mcp_exposure: McpExposure;
  client_side_executable: boolean;
  idempotency_class: IdempotencyClass;
  composition_kind: CompositionKind;
}
```

### Starting Taxonomy

Use the ADR-0102 categories as the starting taxonomy:

- `read`: no domain mutation, no external write, usually no model call.
- `transform`: no domain mutation, may synthesize from claim substrate or model output.
- `publish`: writes externally or creates a shareable artifact outside the substrate.
- `maintenance`: mutates internal state through services.

Generators must treat category as a behavioral constraint, not a display label.
If an ability's implementation mutates state, it cannot be categorized as
`read` or `transform` even if its output looks like a report.

### Field Specifications

| Field | Type | Required? | Description | Validation Rule | Example |
|---|---|---:|---|---|---|
| `name` | string | yes | Canonical ability id used by all generators and introspection surfaces. | Must match `^[a-z][a-z0-9-]*/[a-z][a-z0-9-]*(?:-[a-z0-9]+)*$`; prefix is the namespace, suffix is the ability slug; unique across inventory. | `dailyos/account-overview` |
| `description` | string | yes | One-paragraph human-readable description used for WP, MCP, and SurfaceClient discovery. | 80-600 chars; no Markdown tables; no raw HTML; exactly one paragraph; must pass PII blocklist and internal-vocabulary scans. | `Produces a current account overview from confirmed and attributed claim state...` |
| `category` | enum | yes | Taxonomy bucket aligned to ADR-0102 call-graph behavior. | One of `read`, `transform`, `publish`, `maintenance`; must agree with registry category check. | `transform` |
| `annotations` | object | yes | Free-form generator hints. Reserved keys are listed below; unknown keys are preserved but ignored by core generators. | JSON object; values must be string, number, boolean, or null; reserved keys must satisfy their specific validation. | `{ "owner": "dailyos", "surface_priority": 10 }` |
| `wp_permission` | string | yes | WordPress capability slug required by the WP Abilities API registration, or `none` for runtime-only abilities that WordPress can render but not invoke directly. | Either `none` or `^[a-z][a-z0-9_]{2,63}$`; if `surface_client` can invoke through WordPress, value must not be `none`. | `dailyos_view_accounts` |
| `allowed_actors` | string[] | yes | Actors that may see and invoke this ability through their surface bridge. | Non-empty array; each item one of `user`, `runtime`, `mcp_client`, `surface_client`; no duplicates; `mcp_client` requires `mcp_exposure != "none"`. | `[ "user", "surface_client", "mcp_client" ]` |
| `required_scopes` | string[] | yes | Fine-grained runtime scopes needed before invocation. These complement actor checks and map to surface-scoped permissions. | Array of `domain:action` strings matching `^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$`; sorted ascending; no duplicates; empty only for metadata-only/runtime-only records. | `[ "accounts:read", "claims:read" ]` |
| `mcp_exposure` | enum | yes | MCP exposure level for the network-facing MCP surface. | One of `none`, `metadata_only`, `invocable`; `invocable` requires `allowed_actors` to include `mcp_client`; `metadata_only` allows discovery metadata without handler registration. | `invocable` |
| `client_side_executable` | boolean | yes | Whether a trusted in-process SurfaceClient may invoke the ability directly through its bridge after actor, permission, and scope checks. | Boolean; `true` requires `allowed_actors` to include `surface_client`; `true` requires `wp_permission != "none"` when WordPress is the invoking surface. | `true` |
| `idempotency_class` | enum | yes | Retry and deduplication class for surface bridges and generated clients. | One of `idempotent`, `safe_retry`, `side_effect`; `publish` and `maintenance` abilities default to `side_effect` unless an idempotency key contract is documented in `annotations.idempotency_key`. | `idempotent` |
| `composition_kind` | object | yes | Declares whether this ability produces composition blocks for WordPress Studio and, if so, which block types may be emitted. | Object must be either `{ produces_blocks: false, block_types: [] }` or `{ produces_blocks: true, block_types: non-empty array }`; block types must be known enum values unless `custom` is paired with `annotations.custom_block_type`. | `{ "produces_blocks": true, "block_types": [ "account_overview", "health_snapshot" ] }` |

### Reserved Annotation Keys

The `annotations` field is intentionally small and flat so it can be consumed by
Rust, TypeScript, PHP, and JSON Schema validators without bespoke nested
decoders.

Reserved keys:

- `owner`: owning team or module; string matching `^[a-z][a-z0-9-]*$`.
- `stability`: `experimental`, `beta`, or `stable`.
- `surface_priority`: integer from 0 to 100; higher means more prominent.
- `input_schema_ref`: relative path or URI to the ability input schema.
- `output_schema_ref`: relative path or URI to the ability output schema.
- `wp_ability_name`: explicit WP Abilities API name if different from `name`.
- `wp_block_namespace`: WordPress block namespace, such as `dailyos/account-overview`.
- `custom_block_type`: required when `composition_kind.block_types` contains `custom`.
- `freshness_sla`: short freshness target such as `live`, `15m`, `1h`, or `24h`.
- `data_classification`: `public`, `workspace`, `sensitive`, or `restricted`.
- `feature_flag`: feature flag required before registration.
- `idempotency_key`: description of the idempotency key contract for safe retries.

Unknown annotation keys are allowed during Phase 0, but generators must log them
at warning level. If a key becomes common across two generators, promote it to
this reserved list instead of relying on tribal knowledge.

### JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://dailyos.local/schemas/ability-surface-inventory-entry.schema.json",
  "title": "AbilitySurfaceInventoryEntry",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "name",
    "description",
    "category",
    "annotations",
    "wp_permission",
    "allowed_actors",
    "required_scopes",
    "mcp_exposure",
    "client_side_executable",
    "idempotency_class",
    "composition_kind"
  ],
  "properties": {
    "name": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*/[a-z][a-z0-9-]*(?:-[a-z0-9]+)*$"
    },
    "description": {
      "type": "string",
      "minLength": 80,
      "maxLength": 600,
      "pattern": "^[^\\n]+$"
    },
    "category": {
      "type": "string",
      "enum": ["read", "transform", "publish", "maintenance"]
    },
    "annotations": {
      "type": "object",
      "additionalProperties": {
        "type": ["string", "number", "boolean", "null"]
      },
      "properties": {
        "owner": {
          "type": "string",
          "pattern": "^[a-z][a-z0-9-]*$"
        },
        "stability": {
          "type": "string",
          "enum": ["experimental", "beta", "stable"]
        },
        "surface_priority": {
          "type": "integer",
          "minimum": 0,
          "maximum": 100
        },
        "input_schema_ref": {
          "type": "string",
          "minLength": 1
        },
        "output_schema_ref": {
          "type": "string",
          "minLength": 1
        },
        "wp_ability_name": {
          "type": "string",
          "pattern": "^[a-z][a-z0-9-]*/[a-z][a-z0-9-]*(?:-[a-z0-9]+)*$"
        },
        "wp_block_namespace": {
          "type": "string",
          "pattern": "^[a-z][a-z0-9-]*/[a-z][a-z0-9-]*(?:-[a-z0-9]+)*$"
        },
        "custom_block_type": {
          "type": "string",
          "pattern": "^[a-z][a-z0-9-]*(?:-[a-z0-9]+)*$"
        },
        "freshness_sla": {
          "type": "string",
          "pattern": "^(live|[0-9]+[mhd])$"
        },
        "data_classification": {
          "type": "string",
          "enum": ["public", "workspace", "sensitive", "restricted"]
        },
        "feature_flag": {
          "type": "string",
          "pattern": "^[a-z][a-z0-9_]*$"
        },
        "idempotency_key": {
          "type": "string",
          "minLength": 1
        }
      }
    },
    "wp_permission": {
      "type": "string",
      "pattern": "^(none|[a-z][a-z0-9_]{2,63})$"
    },
    "allowed_actors": {
      "type": "array",
      "minItems": 1,
      "uniqueItems": true,
      "items": {
        "type": "string",
        "enum": ["user", "runtime", "mcp_client", "surface_client"]
      }
    },
    "required_scopes": {
      "type": "array",
      "uniqueItems": true,
      "items": {
        "type": "string",
        "pattern": "^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$"
      }
    },
    "mcp_exposure": {
      "type": "string",
      "enum": ["none", "metadata_only", "invocable"]
    },
    "client_side_executable": {
      "type": "boolean"
    },
    "idempotency_class": {
      "type": "string",
      "enum": ["idempotent", "safe_retry", "side_effect"]
    },
    "composition_kind": {
      "oneOf": [
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["produces_blocks", "block_types"],
          "properties": {
            "produces_blocks": { "const": false },
            "block_types": {
              "type": "array",
              "maxItems": 0
            }
          }
        },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["produces_blocks", "block_types"],
          "properties": {
            "produces_blocks": { "const": true },
            "block_types": {
              "type": "array",
              "minItems": 1,
              "uniqueItems": true,
              "items": {
                "type": "string",
                "enum": [
                  "account_overview",
                  "claim_summary",
                  "evidence_list",
                  "health_snapshot",
                  "relationship_map",
                  "risk_callout",
                  "action_list",
                  "markdown_document",
                  "custom"
                ]
              }
            }
          }
        }
      ]
    }
  },
  "allOf": [
    {
      "if": {
        "properties": { "mcp_exposure": { "const": "invocable" } },
        "required": ["mcp_exposure"]
      },
      "then": {
        "properties": {
          "allowed_actors": { "contains": { "const": "mcp_client" } }
        }
      }
    },
    {
      "if": {
        "properties": { "client_side_executable": { "const": true } },
        "required": ["client_side_executable"]
      },
      "then": {
        "properties": {
          "allowed_actors": { "contains": { "const": "surface_client" } },
          "wp_permission": { "not": { "const": "none" } }
        }
      }
    },
    {
      "if": {
        "properties": {
          "composition_kind": {
            "properties": {
              "block_types": { "contains": { "const": "custom" } }
            }
          }
        }
      },
      "then": {
        "properties": {
          "annotations": { "required": ["custom_block_type"] }
        }
      }
    }
  ]
}
```

### Cross-Field Rules Not Expressible Portably in JSON Schema

The CI validator must also enforce these rules:

1. `required_scopes` must be sorted lexicographically.
2. `category = "publish"` requires `idempotency_class = "side_effect"` unless
   `annotations.idempotency_key` is present.
3. `category = "maintenance"` cannot include `mcp_client` in `allowed_actors`.
4. `mcp_exposure = "metadata_only"` allows `mcp_client` discovery metadata but
   must not generate an MCP invocation handler.
5. `client_side_executable = true` must generate a SurfaceClient invoke binding,
   but only after WordPress capability and runtime scope checks pass.
6. `description` must not contain any term from the PII blocklist.
7. `description` must not contain raw internal pipeline vocabulary.
8. `composition_kind.produces_blocks = true` requires `surface_client` in
   `allowed_actors`; otherwise a generator could create blocks that no surface
   may request.

## mcp_exposure vs client_side_executable resolution

The EoP P2 naming issue is resolved by keeping one MCP-specific exposure field
and one client-execution field, while rejecting the ambiguous
`client_side_exposure` name.

Keep `mcp_exposure` and `client_side_executable` separate. They govern different
trust boundaries and should not be collapsed into one overloaded exposure enum.
`mcp_exposure` controls a network-facing tool surface where a host model or
agent discovers tools, sends JSON input, and receives filtered output. Its
states need to distinguish no exposure, metadata-only discovery, and invocation
because MCP tool lists are themselves sensitive: ability names, descriptions,
schemas, and blast radius can leak product capabilities even when invocation is
blocked.

`client_side_executable` controls an in-process or same-product SurfaceClient
path. The risk is not model-facing network discovery; the risk is whether a
trusted UI surface can call the ability after WordPress capability checks,
runtime scopes, and actor filtering. A WordPress Studio block may need to invoke
an ability to hydrate a composition block even when that ability should never be
listed as an MCP tool. Conversely, an MCP tool may be invocable by an agent while
the WordPress client only renders returned data and should not call the ability
directly.

The inventory therefore uses `mcp_exposure` for MCP registration semantics and
`client_side_executable` for SurfaceClient invocation semantics. The field name
`client_side_exposure` is intentionally not used because it suggests a symmetric
public exposure model with MCP. The boolean `client_side_executable` names the
actual decision: after policy has admitted the surface, may the client-side
bridge execute this ability or only inspect/render metadata produced elsewhere?

## Worked Example

The following complete record describes `dailyos/account-overview`, an ability
that produces a current account overview from the claim substrate.

```yaml
name: dailyos/account-overview
description: Produces a current account overview from confirmed and attributed account context, including health, recent changes, open commitments, relationship context, and evidence-backed risks suitable for WordPress Studio composition blocks and MCP answers.
category: transform
annotations:
  owner: dailyos
  stability: beta
  surface_priority: 90
  input_schema_ref: src-tauri/abilities-runtime/schemas/dailyos/account-overview.input.schema.json
  output_schema_ref: src-tauri/abilities-runtime/schemas/dailyos/account-overview.output.schema.json
  wp_ability_name: dailyos/account-overview
  wp_block_namespace: dailyos/account-overview
  freshness_sla: 15m
  data_classification: sensitive
wp_permission: dailyos_view_accounts
allowed_actors:
  - user
  - runtime
  - mcp_client
  - surface_client
required_scopes:
  - accounts:read
  - claims:read
  - provenance:read
mcp_exposure: invocable
client_side_executable: true
idempotency_class: idempotent
composition_kind:
  produces_blocks: true
  block_types:
    - account_overview
    - health_snapshot
    - relationship_map
    - risk_callout
    - action_list
```

Generator expectations for this record:

- WP Abilities API registers `dailyos/account-overview` with the
  `dailyos_view_accounts` capability.
- MCP registers an invocable tool because `mcp_exposure = "invocable"` and
  `allowed_actors` includes `mcp_client`.
- SurfaceClient introspection returns the entry to surfaces admitted as
  `surface_client`.
- SurfaceClient execution is allowed because `client_side_executable = true`,
  the WordPress permission is not `none`, and required scopes are explicit.
- WordPress Studio may render the listed composition block types from the
  ability output.

## CI Gate Specification

The ability-description gate runs on every commit that touches an ability
inventory entry or any source field that generates `description`.

### Trigger

Run the gate when staged files match any of:

- `.docs/plans/**/ability*.md`
- `.docs/plans/**/ability-surface*.md`
- `.docs/abilities/**/*.json`
- `.docs/abilities/**/*.yaml`
- `.docs/abilities/**/*.yml`
- `src-tauri/**/abilities/**/*.rs`
- `src-tauri/**/abilities/**/*.json`
- `src-tauri/**/abilities/**/*.yaml`
- `src-tauri/**/abilities/**/*.yml`

The gate inspects staged added lines, but schema validation must parse the full
candidate inventory file so unchanged invalid context cannot ride along.

### Check 1: PII Blocklist Scan

The scanner reads `.claude/pii-blocklist.txt` when present. The committed
pre-commit hook already treats that file as the source for the repo-wide PII
blocklist; this gate applies the same source specifically to ability
descriptions.

Algorithm:

1. Load `.claude/pii-blocklist.txt`.
2. Drop blank lines and lines beginning with `#`.
3. Escape each remaining entry for regex use.
4. Join entries with `|` into one case-insensitive word-boundary regex.
5. For each changed inventory record, scan:
   - `description`
   - `annotations.wp_ability_name`
   - `annotations.wp_block_namespace`
   - any generated MCP tool description
   - any generated WP Abilities API description
6. Fail if any value matches the blocklist.

The scanner must report the file, line number, field name, and matched text. It
must not print surrounding private data beyond the offending line.

### Check 2: Internal-Vocabulary Scan

Ability descriptions are product copy and model-facing tool copy. They must not
leak raw implementation vocabulary. The gate blocks the same class of terms that
release copy and report prompts already prohibit.

Forbidden terms for Phase 0:

- `enrichment`
- `AI enrichment`
- `re-enrichment`
- `intelligence pipeline`
- `pipeline`
- `entity intelligence`
- `entity`
- `signals`
- `signal_events`
- `claim substrate`
- `intel queue`
- `PTY`
- `Glean enrichment`
- `runtime evaluator`
- `harness`
- `LLM`
- `prompt`
- `model output`

This rule applies to `description` and any generated model-facing or
user-facing description. It does not apply to `name`, `required_scopes`,
`annotations.input_schema_ref`, or other technical fields where controlled
vocabulary is intentional.

Descriptions should use user-facing language such as "confirmed and attributed
account context" instead of implementation terms. Generator tests should include
at least one negative fixture that proves blocked vocabulary is rejected.

### Check 3: Schema Validation

Every changed inventory record must validate against the schema in this
document.

The validator must:

1. Parse YAML and JSON records into the canonical object shape.
2. Run JSON Schema validation.
3. Run cross-field rules that JSON Schema cannot express portably.
4. Verify `name` uniqueness across all inventory files.
5. Verify `required_scopes` are sorted.
6. Verify every `composition_kind.block_types` value is supported by the
   WordPress Studio generator or is `custom` with `annotations.custom_block_type`.
7. Verify records with `mcp_exposure = "invocable"` produce a handler only when
   `allowed_actors` includes `mcp_client`.
8. Verify records with `client_side_executable = true` include
   `surface_client`, non-`none` `wp_permission`, and at least one scope unless a
   documented exception is present.

### Failure Modes

The gate blocks the commit with exit code `2`.

The failure output must include:

- Failing file path.
- Line number when available.
- Field name.
- Rule id.
- Offending value or offending line.
- Remediation hint.

Example failure:

```text
Ability surface inventory gate failed:

.docs/abilities/dailyos-account-overview.yaml:2
  field: description
  rule: no-internal-vocabulary
  value: "Produces an account overview from the claim substrate..."
  remediation: Replace internal implementation terms with user-facing language,
  e.g. "confirmed and attributed account context."
```

Example schema failure:

```text
Ability surface inventory gate failed:

.docs/abilities/dailyos-account-overview.yaml:13
  field: allowed_actors
  rule: client-side-executable-requires-surface-client
  value: ["user", "mcp_client"]
  remediation: Add "surface_client" or set client_side_executable to false.
```

Example PII failure:

```text
Ability surface inventory gate failed:

.docs/abilities/dailyos-account-overview.yaml:2
  field: description
  rule: pii-blocklist
  value: "..."
  remediation: Replace customer names, domains, and account-specific IDs with
  generic placeholders before committing.
```

### Implementation Notes

The current committed pre-commit entry point is `.githooks/pre-commit`. It notes
that it replaced the per-developer `.claude/hooks/pre-commit-gate.sh` as the
committed source of truth. The ability-description gate should therefore be
implemented as a script called by `.githooks/pre-commit`, while still reading
`.claude/pii-blocklist.txt` if that file exists in the developer environment.

Recommended script name:

```text
scripts/check_ability_surface_inventory.sh
```

Recommended validator flow:

```text
git diff --cached --name-only
  -> filter ability inventory and ability source files
  -> extract full changed inventory records
  -> run schema validation
  -> run description PII scan
  -> run description internal-vocabulary scan
  -> print grouped failures
  -> exit 2 on any failure
```

The same validator should be callable in CI over the full repository so a branch
cannot pass locally with a partial staged-diff view and fail only after merge.

## Open Questions

1. ~~ADR-0102 in this checkout defines `AbilityPolicy` with actor, mode, confirmation, and publish fields, while the DOS-546 prompt references `required_scopes` and `mcp_exposure` as canonical fields. ADR-0102 should be amended or cross-linked so the two contracts cannot drift.~~ **Resolved 2026-05-11 (W0-D):** ADR-0102 §7.1 amended to extend `AbilityPolicy` with `required_scopes: Vec<SurfaceClientScope>`, `mcp_exposure: McpExposure { None | MetadataOnly | Invocable }`, and orthogonal `client_side_executable: bool`. The amendment carries the explicit field separation reasoning from lines 383-412 of this artifact. Surface-inventory fields in this document now match the canonical runtime descriptor exactly — both contracts share one source of truth.
2. ADR-0129 is referenced as the WordPress Studio surface decision, but this
   checkout does not include a `.docs/decisions/0129-*` file. Once that ADR is
   present, verify that `wp_permission`, `wp_ability_name`, and
   `wp_block_namespace` match its final WordPress registration vocabulary.
3. The final list of WordPress Studio block types should come from the first
   WP-side prototype. The Phase 0 enum is intentionally narrow and should be
   revised after the first generator proves what it can render.
4. `client_side_executable` is boolean in Phase 0. If future surfaces need
   metadata-only client discovery separate from execution, add a new
   `client_side_introspection` field rather than overloading MCP exposure.
5. The internal-vocabulary list should be centralized in a committed file so
   release notes, reports, ability descriptions, and generated MCP tool copy all
   use the same scanner.
6. The inventory currently describes exposure and composition. It does not
   include input/output example payloads. Add examples only if generators need
   them for WP documentation or MCP host-model steering.
