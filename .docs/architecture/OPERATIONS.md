# Operations Contract

DailyOS operations are the kebab-case contract view over the ADR-0102 abilities
runtime. The source of truth is `src-tauri/src/operations/mod.rs::OPERATIONS`.
It does not replace `AbilityRegistry`; operation executors dispatch into the
existing ability bridge or into explicitly local internal maintenance code.

## Phase 1 Shape

Each `OperationDef` declares:

- `name`: stable kebab-case operation name.
- `description`: human-readable tool description.
- `remote`: explicit exposure flag. `true` operations may appear in MCP-style
  remote tool discovery; `false` operations are local-only.
- `category`: `Read`, `Transform`, `Publish`, or `Maintenance`.
- `input_schema` and `output_schema`: checked-in JSON schemas included at
  compile time.
- `requires_scope`: optional external scope label.
- `executor`: typed `OperationExecutor`.

Phase 1 registers two proofs:

- `get-entity-context`: `Read`, `remote=true`, dispatches to the existing
  `get_entity_context` ability through `TauriAbilityBridge`.
- `internal-debug-dump`: `Maintenance`, `remote=false`, returns local diagnostic
  counts and is intentionally omitted from `mcp_tool_list()`.

## Enforcement

The `operation_def!` macro requires an explicit `remote` field and verifies the
executor has the `OperationExecutor` signature. `build.rs` checks the operation
source on every build: names must be kebab-case, schema files referenced through
`include_str!` must exist, executor names must match their category prefix, and
the Tauri surface must expose only `operations::invoke_operation` for operation
dispatch.

MCP discovery uses `mcp_tool_list()`, which filters to `remote=true` operations.
The round-trip test asserts that every advertised tool maps back to an operation
and that `remote=false` operations are excluded.
