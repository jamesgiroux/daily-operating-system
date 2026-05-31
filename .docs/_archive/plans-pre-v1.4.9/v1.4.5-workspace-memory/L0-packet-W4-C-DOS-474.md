# L0 Packet - v1.4.5 W4-C - DOS-474 Workspace Placement Write Contract

**Issue:** DOS-474
**Branch/worktree:** `codex/v1.4.5-w4-c-dos-474`
**Base:** `public/dev` at `52c9255c`
**Revision:** V1.3 approved
**Status:** L0 passed in cycle 11 after PR #366 merged/rebased and the coordinated MCP resource placeholder reconciliation was included. No substrate implementation code edits have started in this worktree.
**Prepared:** 2026-05-24

## 0. Executive Verdict

W4-C passed L0 in cycle 11. Implementation may start after James has shared the exact intended substrate write set with the other active session. The lane remains valuable: it freezes the v1.4.5 placement ability/service contract that v1.4.7 DOS-480 can expose through MCP later, without direct filesystem writes, direct SQLite writes, or claim-writer bypasses.

Cycle 1 found that the V0.1 plan was still too loose for a write boundary. Cycle 2 found that V0.2 still assumed transport, migration, and concurrency behavior that the live substrate does not provide. Cycle 3 found that V0.3 crossed the v1.4.7 ownership boundary. Cycle 4 found two remaining schema contradictions: optional `category` conflicted with idempotency/receipt requirements, and the sample error envelope contradicted the optional-field rules. Cycle 5 found that V0.5 still left client validation, identity, dry-run ordering, path-race safety, target authorization, source handles, transaction scope, link attribution, and downstream-doc drift underspecified. Cycle 6 found stale wave text, unbounded pre-decode payload work, JSON trust ambiguity, source-handle storage ambiguity, DOS-480 error-transport ownership gaps, and target-authorization ambiguity. V0.9 makes these binding changes:

- W4-C owns the v1.4.5 `workspace_place_document` ability/service contract, not the v1.4.7 `dailyos.write.place_document` MCP handler, taxonomy, grant provisioning, rmcp transport, or conversation-handle envelope.
- W4-C consumes the existing `Actor::McpClient` / `ActorKind::McpClient` substrate already present on current `public/dev`; it does not introduce or edit MCP actor, gateway, auth, or transport primitives.
- Keep the existing downstream scope string `write.workspace_place_document`; v1.4.7 DOS-480 owns tool exposure with `dailyos.write.place_document`.
- Add `workspace_place_document` as a runtime ability with `mcp_exposure = Invocable`.
- Implement the service boundary inside the already-exported workspace intake owner path. Do not edit `workspace_ingestion/mod.rs` or add a new workspace ingestion submodule.
- Use migration slot v262 for W4-C L1 implementation, the next contiguous slot on current `public/dev`, so the max-version migration runner cannot skip future lower migrations.
- Leave the v1.4.7 gateway short-window limit (10/minute) to DOS-480. W4-C enforces only the service long-window limit (200/hour per actor/tool) in the placement service ledger.
- Make the service hourly ledger an atomic reservation with rollback/commit semantics, not a prune/count/insert race.
- Store only scope-neutral idempotency facts. Never cache `resolved_path` or any receipt field whose visibility depends on the current invocation redaction context.
- Generate public `document_handle` / `idempotency_id` values as random UUIDs or server-secret HMACs. Never expose unkeyed content, entity, or path digests.
- Include `mutation_cursor` in every successful write response.
- Treat `dry_run` as a live preview that may write placement attempt-audit and service rate rows, but no business placement, lifecycle, run, or file rows. Dry-run receipts use a preview cursor with no durable/content-derived handle.
- Resolve entity display/path material server-side from entity id. The MCP client never supplies `entity_name`.
- Remove `source_asof_hint` from the public request. W4-C does not accept caller-provided source-time hints.
- Require `category` in v1. A missing category returns `invalid_request_shape`; W4-C does not default or auto-detect category before idempotency.
- Freeze a typed `PlacementError` envelope with `code`, safe `message`, and code-specific optional fields that are omitted when not applicable. DOS-480 must map this through a transport-visible MCP error envelope.
- Freeze exact validation rules for schema version, base64, content size, `filename_hint`, `client_dedup_key`, and failure-code mapping.
- Use the server-issued `Actor::McpClient.client_id` as the placement `actor_id` for rate, idempotency, and audit. No caller-supplied id, label, session id, or conversation handle participates in replay keys.
- Add a non-enumerating target authorization check before path resolution, placement idempotency, or raw target audit fields. Missing and unauthorized targets both return `target_not_found_or_unauthorized`.
- Make `source_handle` a public opaque handle, never a raw ingestion run id. Live success receipts require a non-null public source handle; dry-run receipts return `null`.
- Move dry-run after the same non-mutating validations as live writes and explicitly forbid dry-run directory/file creation.
- Require handle-relative write traversal (`openat`/`mkdirat` style or platform equivalent), retained parent handles, and no-follow/no-reparse checks per component.
- Require short placement transactions only; no placement idempotency transaction may stay open across file I/O or `IngestPipeline::run`.
- Define the `LinkAttributionSource::McpPlacement` seam instead of assuming the current entity-intake hardcode can produce it.
- Treat stale v1.4.5/v1.4.7 wave text and the existing MCP tool description as superseded placeholders. W4-C L1 is blocked until downstream docs/taxonomy are reconciled by DOS-480 or by a coordinated scope amendment.
- Replace the old W4-C wave-contract block with a non-implementable pointer to this packet.
- Reconcile the existing `dailyos.write.place_document` tool-description resource placeholder by explicit coordination only, including live and dry-run receipt examples that match this packet; this does not authorize W4-C MCP handler, gateway, auth, transport, registration, grant, or tools/list code edits.
- Add encoded and transport-size limits before expensive decode/validation work.
- Treat all placed content, including `application/json`, as untrusted document bytes; provenance, trust, actor, lifecycle, link attribution, source time, and claim identity remain server/pipeline-derived only.
- Persist opaque `source_handle` in the placement idempotency row for stable replay.
- Freeze v1 target authorization as "write grant plus existing local routable entity row"; future per-entity ACLs require an L0 amendment.
- Treat placement attempt audit targets as HMAC-only for all outcomes. Raw `entity_type`/`entity_id` may appear in the placement idempotency row only after target authorization succeeds; the audit table never stores the raw target tuple.
- Treat workspace signal payload ids as internal service events. W4-C must not expose or mirror `file_id`, `ingestion_run_id`, `run_id`, or internal signal `entity_id` values through MCP receipts, tool metadata, audit rows, or user-visible surfaces. The public request/receipt `entity_id` is the service-owned opaque entity handle returned by authorized read/search tools. In the current local store that handle is also the canonical entity row key, but MCP clients must treat it as opaque and never derive names, paths, or signal ids from it. Any future exported signal stream requires opaque/HMAC translation by its owner.

W4-C L1 is not authorized for MCP v2 code beyond the coordinated `tool_descriptions.yaml` resource placeholder reconciliation already present in this packet. Public MCP exposure is a v1.4.7 DOS-480 responsibility and remains blocked until that branch restores or defines the concrete contracts for manifest grants, 10/minute enforcement, write audit, tools/list taxonomy, public error transport, and conversation-handle round-trip. Before editing implementation files, the agent must post the exact substrate write set for cross-session coordination; this is an execution checkpoint, not an L0 design blocker.

## 1. Active PR Coordination

Checked open PRs against `dev` on 2026-05-24:

| PR | State | W4-C consequence |
| --- | --- | --- |
| #367 `feat/v1.4.4-wp-visual-parity` | open, dirty | No direct MCP placement overlap. Avoid WP composition edits. |
| #366 `account-fact-claim-producer` | merged into `dev` on 2026-05-24 13:08:36Z; W4-C rebased on `public/dev` | Do not touch account/Glean producer code, `services/claims.rs`, trust recompute, source-purge helpers, or claim lifecycle helpers. Re-read merged context, migration, and workspace intake shapes before implementation. |
| #355 `dos-758-mcp-handler-interface` | draft, dirty | v1.4.7/DOS-480 dependency only. W4-C must not edit MCP v2 handler, gateway, auth, taxonomy, transport, or registration files. |
| #296 preservation PR | draft, dirty | No direct W4-C implementation dependency. |

## 2. K-In Evidence

Knowledge-store and live-code grep was run before drafting.

| Source | Constraint |
| --- | --- |
| AGENTS.md Intelligence Loop rule | Placement is a write path into memory. It must preserve claim/provenance/trust/signal/feedback semantics by routing through services and the ingestion pipeline. |
| ADR-0102 | Ability/MCP contracts are explicit runtime contracts. `write.workspace_place_document` is a pre-amendment scope that remains unprefixed, while the tool name uses `dailyos.write.place_document`. |
| ADR-0105 | `source_asof` is first-class. Caller hints cannot silently become trusted source time. |
| ADR-0107 | Workspace placement uses `WorkspaceFileKind::McpPlacement`; do not invent a new data-source taxonomy. |
| ADR-0108 | Do not expose raw paths, raw internal IDs, raw claim text, raw provenance JSON, prompt bodies, or output bodies across surface/MCP boundaries. |
| ADR-0126 | Claim text/type/subject/source_asof are immutable after commit; W4-C must not patch claims after pipeline commit. |
| `workspace_ingestion/mod.rs` shape gate | Downstream v1.4.5 lanes fill existing placeholder files and never edit `mod.rs` or create new workspace-ingestion submodules. |
| `workspace_ingestion/registry.rs` | `WorkspaceCategoryRegistry::resolve_path` needs server-owned `entity_name`/slug input and category validation before path construction. |
| `workspace_ingestion/workspace_intake_impl.rs` | This is the existing service bridge owner. W4-C should extend this owner rather than adding a new `workspace_ingestion` module. |
| `workspace_ingestion/pipeline.rs` | The staged pipeline owns lifecycle, category, extraction, claim commit, and signal hooks. W4-C calls it rather than writing claims or rows directly. |
| `workspace_ingestion/runs.rs` | Existing run idempotency is `(file_id, content_sha256, mode)` and guards ingestion runs only. W4-C still needs a placement-level fence before file write. |
| `abilities-runtime/src/inventory.rs` + `mcp_v2/actor_policy.rs` | Current `public/dev` already contains `McpClient` actor inventory/projection. W4-C consumes that primitive and does not introduce actor or gateway substrate. |
| `.docs/plans/v1.4.5-waves.md` | W4-C owns the headless placement contract/ability and explicitly does not touch v1.4.7 MCP Server v2 tooling. |
| `.docs/plans/v1.4.7-waves.md` | DOS-480 owns `dailyos.write.place_document`, `tool_placement.rs`, MCP grants, conversation handle behavior, and exact MCP exposure of the v1.4.5 receipt. |
| `mcp_v2/transport.rs` | The live rmcp adapter currently drops response conversation handles and hides `ToolError.detail`; W4-C must not depend on it. DOS-480 owns the transport-visible mapping. |

## 3. Scope

### In Scope

- Runtime ability `workspace_place_document`.
- Headless v1.4.5 placement contract consumed later by v1.4.7 DOS-480.
- Existing downstream scope string `write.workspace_place_document`.
- Service-owned file placement under the configured DailyOS workspace.
- Placement idempotency by `(actor_id, entity_type, entity_id, content_sha256, content_type, normalized_category_slug, client_dedup_key.unwrap_or_default())`, with `category` required before key construction.
- Coordinated migration slot for placement idempotency, service hourly-rate ledger, and sanitized placement-attempt audit tables. The L1 slot is **v262**, the next contiguous slot after the current live max v261.
- Service long-window limit: 200/hour per actor/tool, including dry runs and rejected live attempts after schema validation.
- Dry-run mode that validates and previews the target without writing content, placement idempotency rows, lifecycle rows, ingestion runs, or files.
- Privacy-safe receipt with no MCP transport assumptions.
- Typed `PlacementError` envelope for downstream MCP mapping.

### Out of Scope

- Account/Glean claim producer changes.
- Any direct call to `services::claims::commit_claim` outside the existing ingestion pipeline.
- Claim retraction, source purge, trust recompute, or claim lifecycle helper edits.
- General file upload to Drive, Slack, email, or external storage.
- Full PARA taxonomy.
- WP source-management or markdown-preview UI edits.
- v1.4.7 MCP handler, gateway, auth, taxonomy, transport, registration, grants, tools/list, or conversation-handle behavior.
- Changes to lower-level signal bus semantics.
- Changes to `workspace_ingestion/mod.rs`.

## 4. Public Contract

### Downstream MCP Tool

W4-C does not implement the MCP tool. It freezes the v1.4.5 ability/service shape that v1.4.7 DOS-480 exposes later as:

```text
dailyos.write.place_document
```

Required scope:

```text
write.workspace_place_document
```

DOS-480 owns `tool_placement.rs`, `tools/list`, grants, rmcp transport, public error projection, and conversation-handle round-trip. DOS-480 must not change the W4-C input/receipt/error shape without a v1.4.5 amendment.

Downstream reconciliation precondition: W4-C L1 may not start while `.docs/plans/v1.4.7-waves.md` or `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` still advertises the old placement surface (`topic`, plain `content`, `document_id`, optional `category`, or the old idempotency key) as an active contract. James explicitly coordinated a W4-C amendment to update only `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml`; no MCP handler, gateway, auth, transport, registration, grant, or tools/list code is authorized here. DOS-480 or W1-A must also own a transport-visible domain-error carrier so every `PlacementError.code` maps to public MCP error data instead of hidden `ToolError.detail`.

### Ability Descriptor

Required ability metadata:

- `name = "workspace_place_document"`
- `category = Transform`
- `may_publish = true`
- `allowed_actors = [McpClient]`
- `allowed_modes = [Live]`
- `requires_confirmation = false`
- `required_scopes = ["write.workspace_place_document"]`
- `mcp_exposure = Invocable`

Compensating controls for `Transform + may_publish = true`:

- Only `McpClient` actors can invoke the ability.
- DOS-480 must resolve a manifest grant for `dailyos.write.place_document` before dispatch.
- The downstream grant must include `write.workspace_place_document`.
- DOS-480 registers the MCP tool as `Side::Write`, so success payloads must include `mutation_cursor` and write audit redaction applies.
- Placement idempotency, path validation, rate limiting, and ingestion are service-owned. The ability does not write filesystem or database state directly.

Do not switch to `AbilityCategory::Publish` unless PR #355 or a later MCP branch lands a confirmation-proof write transport and L0 is rerun.

### Request

```json
{
  "schema_version": 1,
  "entity": {
    "entity_type": "account",
    "entity_id": "acct_123"
  },
  "content_b64": "base64-encoded bytes",
  "content_type": "text/markdown",
  "filename_hint": "renewal-notes.md",
  "category": "notes",
  "client_dedup_key": "optional-client-key",
  "dry_run": false
}
```

Schema constraints:

| Field | Required | Constraint |
| --- | --- | --- |
| `schema_version` | yes | integer constant `1` |
| `entity` | yes | closed object; required fields `entity_type`, `entity_id`; no additional properties |
| `entity.entity_type` | yes | enum: `account`, `person`, `project`; `other` is not routable for W4-C |
| `entity.entity_id` | yes | non-empty opaque id, 1-128 bytes, ASCII letters/digits plus `._:-`; target authorization checked server-side |
| `content_b64` | yes | RFC 4648 standard base64 alphabet with required padding, no whitespace, no URL-safe alphabet; encoded length must be `<= 13_981_016` bytes before decode; decoded size must be `<= 10_485_760` bytes; decoded bytes must be valid UTF-8 because all v1 placement content types are textual |
| `content_type` | yes | enum: `text/markdown`, `text/plain`, `application/json`; `application/json` is ingested as untrusted UTF-8 text, not trusted structured provenance |
| `filename_hint` | no | hint only; if present must be a string, ASCII-trimmed to 1-128 bytes, and match `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$`; no hidden-file prefix, dot-dot segment, slash, backslash, drive prefix, NUL, whitespace, or shell metacharacters |
| `category` | yes | required normalized slug; `WorkspaceCategory::from_slug` plus `WorkspaceCategoryRegistry::validate(conn, category, entity_type)` |
| `client_dedup_key` | no | if present must be a string, ASCII-trimmed to 0-128 bytes, and match `^[A-Za-z0-9._:-]{0,128}$`; empty/missing collapses to `""`; participates in idempotency |
| `dry_run` | no | defaults to `false` |

The request object is closed: unknown top-level or nested fields return `invalid_request_shape`.

Missing or non-integer `schema_version` returns `invalid_request_shape`. Integer schema versions other than `1` return `unsupported_schema_version`. Null for any optional string field is invalid; omit the field instead. Invalid base64 returns `invalid_content_encoding`; oversize decoded content returns `content_too_large`; invalid `filename_hint` returns `invalid_filename_hint`; invalid `client_dedup_key` returns `invalid_client_dedup_key`.

The MCP request `arguments` JSON body for this tool must be rejected at `> 14_100_000` UTF-8 bytes before full deserialization or base64 allocation by the MCP owner. DOS-480 owns that raw transport envelope check before dispatch. W4-C cannot see the raw request body after ability-runtime deserialization, so W4-C enforces the `content_b64` encoded-length cap at the ability boundary and a secondary serialized-argument cap before typed request parsing. Oversized malformed payloads and oversized payloads missing required fields return without service rows.

`application/json` is accepted only as untrusted document content. Caller-supplied JSON keys that look like provenance, trust, actor, lifecycle, link attribution, claim metadata, source identifiers, source time, or claim identity are ignored as substrate metadata and treated as ordinary document bytes for extraction. All authoritative `source_asof`, provenance, trust band, actor, lifecycle, link attribution, source handles, and claim identity values are server- or pipeline-derived.

Clients obtain valid `entity_id` values from existing read/search tools that expose service-owned opaque entity handles under their grants. They do not pass display names, workspace path segments, or ids from any non-authorized source.

W4-C v1 does not implement category auto-detection for public placement. Missing, null, or empty `category` returns `invalid_request_shape` before service rows, path resolution, receipt rendering, dry-run preview, or idempotency-key construction. A later auto-detect mode would require an explicit contract amendment because it changes replay keys and receipt semantics.

### Receipt

```json
{
  "schema_version": 1,
  "document_handle": "placement_9e8f6d2c-72c9-42e4-8fe8-8b75c4e9c8d0",
  "source_handle": "source_9e8f6d2c-72c9-42e4-8fe8-8b75c4e9c8d0",
  "entity_type": "account",
  "entity_id": "acct_123",
  "category": "notes",
  "workspace_file_kind": "mcp_placement",
  "source_asof": "2026-05-24T00:00:00Z",
  "lifecycle_state": "ingested",
  "claim_count_produced": 3,
  "idempotent_replay": false,
  "dry_run": false,
  "resolved_path": null,
  "mutation_cursor": {
    "kind": "workspace_placement",
    "document_handle": "placement_9e8f6d2c-72c9-42e4-8fe8-8b75c4e9c8d0",
    "source_handle": "source_9e8f6d2c-72c9-42e4-8fe8-8b75c4e9c8d0",
    "idempotency_id": "placement_9e8f6d2c-72c9-42e4-8fe8-8b75c4e9c8d0"
  }
}
```

Receipt rules:

- W4-C returns this receipt at the ability/service boundary. DOS-480 owns the MCP wire wrapper and must preserve this shape when exposing the tool.
- `document_handle` is a random UUID-backed handle or server-secret HMAC-backed handle. It is never a raw filesystem path, raw `file_id`, unkeyed content digest, unkeyed entity digest, or unkeyed path digest.
- `source_handle` is a public opaque handle backed by a server-side mapping or server-secret HMAC over the ingestion run id. It is never the raw `run_id`, raw `ingestion_run_id`, raw `file_id`, or any other DB identifier. Live success requires a non-null `source_handle`; dry-run returns `null`.
- Receipt `entity_id` is the public opaque entity handle echoed from the request. It is not a workspace path, display name, source id, file id, run id, or internal signal `entity_id`.
- `resolved_path` is rendered at response time from the current invocation redaction context. It is `null` unless the downstream MCP invocation context proves the client also has `read.entity_names`.
- Idempotency rows store only scope-neutral facts. They do not store full receipt JSON.
- No raw absolute path, canonical path, raw internal `file_id`, raw entity display name, raw claim text, raw provenance JSON, prompt body, output body, or content bytes may appear in the receipt or mutation cursor.

Dry-run receipt delta:

```json
{
  "schema_version": 1,
  "document_handle": null,
  "source_handle": null,
  "entity_type": "account",
  "entity_id": "acct_123",
  "category": "notes",
  "workspace_file_kind": "mcp_placement",
  "source_asof": null,
  "lifecycle_state": "not_written",
  "claim_count_produced": 0,
  "idempotent_replay": false,
  "dry_run": true,
  "resolved_path": null,
  "mutation_cursor": {
    "kind": "workspace_placement_preview",
    "dry_run": true
  }
}
```

Dry-run receipts include the required top-level receipt handle fields with `document_handle = null` and `source_handle = null`. They never include `idempotency_id`, content-derived values, path-derived values, or a durable mutation cursor; the preview `mutation_cursor` contains only `kind = "workspace_placement_preview"` and `dry_run = true`.

## 5. Placement Error Taxonomy

W4-C freezes a typed `PlacementError` envelope. DOS-480 owns mapping this envelope to a transport-visible MCP error response.

Required error data shape:

```json
{
  "code": "category_not_allowed",
  "message": "category is not allowed for this entity type",
  "allowed": ["notes", "contracts"]
}
```

Rules:

- `code` is always present and stable.
- `message` is safe prose with no raw path, content, claim text, prompt/output body, or PII.
- `allowed` is present only for `category_not_allowed` and contains safe category slugs.
- `retry_after_seconds` is present only for `rate_limited` and `idempotency_in_progress`.
- `trace_id` is present only for `placement_internal`.
- Optional fields are omitted when they are not applicable; they are not serialized as `null`.

| Code | Recoverability |
| --- | --- |
| `invalid_request_shape` | Fix request JSON/schema. |
| `invalid_entity_type` | Use `account`, `person`, or `project`. |
| `invalid_entity_id` | Use an entity id returned by a read/search tool. |
| `target_not_found_or_unauthorized` | Refresh entity list/search or request access; missing and unauthorized targets are intentionally indistinguishable. |
| `entity_not_routable` | Entity type cannot receive workspace placement in v1. |
| `invalid_category` | Use a registered category slug. |
| `category_not_allowed` | Choose from safe `allowed` slugs in error data. |
| `invalid_content_encoding` | Fix base64. |
| `invalid_content_type` | Use an allowed content type. |
| `content_too_large` | Reduce decoded content below the limit. |
| `invalid_filename_hint` | Fix or omit the hint. |
| `invalid_client_dedup_key` | Fix or omit the deduplication key. |
| `unsupported_schema_version` | Use `schema_version = 1`. |
| `idempotency_in_progress` | Retry after `retry_after_seconds`; no second write was attempted. |
| `previous_attempt_failed` | Use a new `client_dedup_key` after user/client review. |
| `placement_path_rejected` | Candidate path failed path-boundary validation. |
| `rate_limited` | Retry after `retry_after_seconds`. |
| `ingestion_failed` | Source was not accepted by the ingestion pipeline. |
| `placement_internal` | Report `trace_id`; no payload data leaks. |

## 6. Service Flow

```text
workspace_place_document ability
  -> closed-schema parse + PlacementError mapping
  -> WorkspacePlacementService::place_document
  -> atomic service 200/hour rate reservation
  -> content/category/filename normalization
  -> server-side target authorization + canonical entity path material
  -> non-mutating path-boundary preview
  -> dry-run receipt or live write branch
  -> placement-level idempotency fence
  -> write bytes under workspace root
  -> read-side WorkspaceSourceRegistry::open_validated on final relative path
  -> IngestPipeline::run with WorkspaceFileKind::McpPlacement and McpPlacement link attribution
  -> persist LinkAttributionSource::McpPlacement for document_entity_links
  -> receipt rendering + mutation_cursor
```

Implementation ownership:

- Put the abilities-runtime service trait/DTOs in the existing `src-tauri/abilities-runtime/src/services/workspace_intake.rs` owner file, or a renamed owner only if PR #355/#366 already moved the seam.
- Implement the Tauri service in `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs`.
- Do not add a new `workspace_ingestion` submodule.
- Do not edit MCP v2 handler, gateway, auth, taxonomy code, transport, registration, grant, or tools/list files in W4-C. The only coordinated MCP v2 path in this packet is the resource metadata reconciliation at `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml`.
- Pass a server-only `PlacementInvocationContext` from the ability boundary to the service. It is constructed from runtime actor state, never from public params, and contains `actor_id`, `tool_name = "dailyos.write.place_document"`, and `can_read_entity_names`. `actor_id` is exactly the server-issued `Actor::McpClient.client_id` projected by the gateway after pairing and manifest lookup. Do not include conversation handle, session id, client-provided labels, or request params in `actor_id`. If the ability boundary cannot prove the downstream `read.entity_names` grant, `can_read_entity_names` defaults to `false`; DOS-480 may set it from the gateway-resolved grant when it exposes the tool. The service uses `actor_id/tool_name` for rate, audit, and idempotency rows and `can_read_entity_names` for receipt redaction.
- Add a service-owned link attribution seam before implementation. Preferred shape: extend the workspace intake/IngestRequest DTO with `link_attribution_source`, default it to `EntityIntake` for existing callers, and set it to `McpPlacement` for W4-C. Acceptable fallback: pre-create the entity link through the service/link owner before pipeline execution with the same short-transaction rules. Do not rely on the current pipeline hardcode of `LinkAttributionSource::EntityIntake`.

Entity path source:

- The service authorizes `actor_id + entity_type + entity_id` before path resolution, idempotency, or raw target audit fields. For v1.4.5, authorization means the invocation has already passed the `write.workspace_place_document` grant and the server finds an existing local row for the requested routable entity handle. There is no per-entity ACL in this release. If a future per-entity ACL lands before W4-C, rerun L0 before adding it. Missing rows, future unauthorized targets, and unroutable targets use the same public `target_not_found_or_unauthorized` shape unless the request is syntactically invalid.
- Authorization returns the canonical display name/path material for allowed targets using the local entity tables (`accounts`, `people`, `projects`) through `services/`/DB helpers. Reuse the existing pipeline `canonical_entity_name` discipline or an equivalent service-local helper after rebase; do not add a handler-side lookup.
- The service derives the path segment with the existing slug discipline (`crate::util::slugify` plus `is_valid_path_segment_slug`-equivalent validation).
- Empty or invalid derived slugs return `entity_not_routable`.
- Client-supplied `entity_name` is not accepted.

Write-side path boundary:

- Decode content, enforce size/content-type limits, and compute `content_sha256` before any file write.
- Validate `filename_hint`, but do not include it in the idempotency key. If implementation uses it in the final display filename, it must uniquify the hint with the placement `idempotency_id`, store that chosen filename as scope-neutral metadata, and never fail a distinct placement solely because another placement used the same hint. Otherwise use a deterministic basename derived from `idempotency_id`.
- Resolve a workspace-relative candidate path with `WorkspaceCategoryRegistry::resolve_path`.
- Canonicalize the workspace root before path creation.
- Create/walk parent directories with handle-relative traversal (`openat`/`mkdirat` style on Unix or a Windows equivalent) from a retained workspace-root directory handle. Check every component with no-follow/no-reparse semantics before descent, retain parent handles while opening children, and reject symlinks, `..`, cross-device escapes, hidden internal targets, and non-directory parents.
- Open the final file relative to the retained parent handle with create-new semantics and no-follow/no-reparse behavior. Platforms without equivalent protections must fail closed rather than silently downgrading the write boundary.
- Dry-run performs all non-mutating validation and path-boundary preview but does not create directories, open the final file, or write bytes. Existing ancestors may be inspected; missing ancestors are treated as creatable only if their lexical path and nearest existing parent pass the same boundary checks.
- Re-open through `WorkspaceSourceRegistry::open_validated` before passing the file to the ingestion pipeline. `open_validated` is a read-side verification step, not the write-side boundary.

Failure policy:

- Closed-schema validation failures stay at the ability boundary and write no service rows. After schema validation, placement attempts must write a sanitized attempt-audit row for both success and failure, including stable error code only.
- Target authorization failures after schema validation write sanitized audit using a server-secret target HMAC only; do not store raw `entity_type` or `entity_id` until target authorization succeeds.
- Validation and dry-run failures write no placement idempotency row, no lifecycle row, no ingestion run, and no file.
- If the placement fence is acquired and the file write fails, mark the placement row `failed`, remove any partial file best-effort, and return a typed error.
- If the file is written and the ingestion pipeline creates lifecycle/run state but fails or quarantines, keep the lifecycle/run evidence and mark the placement row `failed`. Do not retract claims and do not call source-purge helpers in W4-C.
- A failed idempotency row is terminal for that key; clients must choose a new `client_dedup_key` for an explicit retry after inspecting the error.

## 7. Idempotency And Rate-Limit Storage

DOS-474 freezes the idempotency key:

```text
(actor_id, entity_type, entity_id, content_sha256, content_type, normalized_category_slug, client_dedup_key.unwrap_or_default())
```

`actor_id` is the server-issued `Actor::McpClient.client_id` after pairing and manifest lookup. It is stable across calls by the same paired MCP client, isolated across clients, and independent of conversation handle/session id. If a future DOS-480 gateway introduces a distinct human-principal id in addition to `McpClientId`, W4-C must rerun L0 before adding it to this key.

The L1 migration slot is **v262**. On the rebased `public/dev`, the live max is v261. Earlier L0 cycles used provisional v280 to avoid reservation collisions, but L2 rejected shipping an executable v280 before lower slots because the live migration runner advances by `MAX(version)`. W4-C is therefore globally renumbered into the next contiguous slot on current `dev`; any later branch that also needs v262 must rebase and take the next available contiguous slot.

The migration must be atomic and rerun-safe under the live runner. Use `BEGIN IMMEDIATE; ... COMMIT;` plus `IF NOT EXISTS`, or implement a `Migration::Fn`.

```sql
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS workspace_placement_idempotency (
  idempotency_id TEXT PRIMARY KEY,
  actor_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  content_sha256 TEXT NOT NULL,
  content_type TEXT NOT NULL,
  category_slug TEXT NOT NULL,
  client_dedup_key TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL CHECK (status IN ('in_progress','succeeded','failed')),
  document_handle TEXT,
  source_handle TEXT,
  file_id TEXT,
  run_id TEXT,
  chosen_filename TEXT,
  source_asof TEXT,
  lifecycle_state TEXT,
  claim_count_produced INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  stale_after TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now', '+1 hour')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(actor_id, entity_type, entity_id, content_sha256, content_type, category_slug, client_dedup_key)
);

CREATE TABLE IF NOT EXISTS workspace_placement_rate_ledger (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  called_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workspace_placement_rate_window
  ON workspace_placement_rate_ledger(actor_id, tool_name, called_at);

CREATE TABLE IF NOT EXISTS workspace_placement_attempt_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  target_audit_key TEXT,
  category_slug TEXT,
  dry_run INTEGER NOT NULL CHECK (dry_run IN (0, 1)),
  outcome TEXT NOT NULL CHECK (outcome IN ('succeeded','failed')),
  error_code TEXT,
  occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

COMMIT;
```

Fence algorithm:

1. DOS-480 may reserve its MCP gateway rate limit before invoking the ability; W4-C does not edit or depend on that gateway path.
2. Ability parses request enough to reject schema failures without service rows.
3. Service opens a short `BEGIN IMMEDIATE` transaction, prunes `workspace_placement_rate_ledger` rows older than one hour, counts rows for `(actor_id, dailyos.write.place_document)`, inserts a reservation row or returns `rate_limited`, then commits or rolls back immediately. This reservation covers dry runs and rejected live attempts after schema validation.
4. Service decodes content, enforces exact validation rules, authorizes the target, validates category, computes `content_sha256`, resolves the candidate path, and performs non-mutating path-boundary preview. Validation failures write sanitized failure audit in a separate short transaction. All placement attempt audit rows store only `target_audit_key` for the target, not raw target fields.
5. For `dry_run = true`, write sanitized success audit in a separate short transaction, return the dry-run receipt, and stop before inserting `workspace_placement_idempotency`, creating directories, opening the final file, writing content, lifecycle rows, or ingestion runs.
6. For live writes, use a short `BEGIN IMMEDIATE` transaction to insert an `in_progress` placement row with random/HMAC-backed `idempotency_id` and `document_handle`, then commit before any filesystem write or pipeline call.
7. On unique conflict, load the row in that same short transaction and commit/rollback before returning or continuing:
   - `succeeded`: render a fresh receipt from scope-neutral row facts and the current invocation redaction context, with `idempotent_replay = true`.
   - `in_progress`: if `stale_after` has not passed, return `idempotency_in_progress` with retry guidance. If stale, reconcile deterministically: either reconstruct success from durable file/run evidence or mark the row `failed` with `previous_attempt_failed` after best-effort orphan-file cleanup.
   - `failed`: return `previous_attempt_failed`; require a new `client_dedup_key` for retry.
8. After the idempotency insert transaction commits, create directories/write the file with the handle-relative path algorithm and call `IngestPipeline::run`. The placement service must not hold an idempotency or rate-ledger write transaction across file I/O or the pipeline call; the pipeline may manage its own service-owned transaction internally.
9. On success, mint `source_handle` as a random UUID-backed or server-secret HMAC-backed public handle and update the row with `status = succeeded`, `file_id`, `run_id`, `document_handle`, `source_handle`, `source_asof`, `lifecycle_state`, and `claim_count_produced` in a separate short transaction, then write sanitized success audit.
10. On failure after schema validation, update the idempotency row when one exists and write sanitized failure audit with stable `error_code` only, both through short transactions.

No raw content, raw path, raw claim text, raw provenance JSON, prompt body, or output body is stored in any placement table. `actor_id` is stored only as the server-minted MCP client id, never as a user-supplied label. Raw `entity_type` and `entity_id` may be stored only in the idempotency row after target authorization succeeds; placement attempt audit stores only `target_audit_key`, a server-secret HMAC over the attempted target tuple, for every outcome.

## 8. Coordination-Gated Write Set

Do not edit these files until James has shared the exact intent with the other substrate session:

- `src-tauri/abilities-runtime/src/abilities/workspace_place_document/{mod.rs,contracts.rs,producer.rs}`
- `src-tauri/abilities-runtime/src/abilities/mod.rs`
- `src-tauri/abilities-runtime/src/services/workspace_intake.rs`
- `src-tauri/abilities-runtime/src/services/context.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/context.rs`
- `src-tauri/src/services/workspace_ingestion/contracts.rs`
- `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs`
- `src-tauri/src/services/workspace_ingestion/registry.rs`
- `src-tauri/src/services/workspace_ingestion/pipeline.rs`
- `src-tauri/src/services/workspace_ingestion/runs.rs`
- `src-tauri/src/services/workspace_ingestion/link.rs`
- `src-tauri/src/services/workspace_ingestion/wiring.rs`
- `src-tauri/src/migrations.rs`
- `src-tauri/src/migrations/262_workspace_placement_idempotency.sql`
- `tools/dailyos-abilities.json`

Already coordinated resource-only edit:

- `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` — remove the stale placement placeholder contract and mirror the frozen W4-C request/receipt shape for downstream taxonomy discovery. No handler/gateway/auth/transport behavior changes are included.

Explicit do-not-touch unless a later L0 amendment reopens scope:

- `src-tauri/src/services/claims.rs`
- `src-tauri/src/services/account_fact_claims.rs`
- `src-tauri/src/services/trust_recompute.rs`
- `src-tauri/src/db/data_lifecycle.rs`
- source-purge helpers
- account/Glean producer code
- `src-tauri/src/signals/bus.rs`
- `src-tauri/src/services/workspace_ingestion/mod.rs`
- `src-tauri/src/services/mcp_v2/**` except the coordinated resource-only `tool_descriptions.yaml` reconciliation named above
- W4-A/W4-B WordPress block files

## 9. Intelligence Loop Check

1. **Claim model:** W4-C writes no claims directly. It places source content and invokes the existing ingestion pipeline, which may produce claim proposals and commit through W3-A's `commit_claim` path.
2. **Provenance + trust:** The placed document is a `WorkspaceFileKind::McpPlacement` source. `source_asof` is server-derived; caller source-time hints are rejected. Derived claims inherit source attribution through the ingestion pipeline.
3. **Signals + invalidation:** W4-C relies on W3-B's workspace signal emitter through the pipeline. It must not change signal-bus semantics or create an exported signal stream. Existing workspace signal ids are internal service payloads only; MCP receipts/tool metadata/audit rows/user-visible surfaces must use the packet's opaque handles or HMAC audit keys instead.
4. **Runtime + surfaces:** W4-C exposes the `workspace_place_document` ability/receipt contract. v1.4.7 DOS-480 later exposes it as `dailyos.write.place_document` over MCP. W4-A/W3-C can later read the placed source through source-management and graph projection.
5. **Feedback loop:** User corrections/relinks/quarantine remain in source-management and claim-feedback flows. W4-C must not add a separate correction path.

## 10. Test Plan

Required before L2:

- Ability descriptor exposes `mcp_exposure = Invocable`, requires `write.workspace_place_document`, admits only `McpClient`, allows only `Live`, and is not client-side executable.
- `scripts/check_ability_inventory.sh` passes after regenerating `tools/dailyos-abilities.json`.
- Request parsing rejects missing/non-integer schema versions as `invalid_request_shape` and integer versions other than `1` as `unsupported_schema_version`.
- Golden validation tests cover RFC 4648 standard padded base64 only, encoded `content_b64` limit `13_981_016` bytes, decoded limit `10_485_760` bytes, the W4-C secondary serialized-argument cap, `filename_hint` regex, `client_dedup_key` regex, null optional fields, and failure-code mapping. DOS-480 must add the raw MCP arguments body limit test before public MCP exposure.
- Oversized malformed payloads and oversized payloads missing required fields produce no service rows and no rate rows in W4-C. DOS-480 owns the pre-deserialization bounded-memory transport test for the raw MCP body.
- JSON placement fixtures that attempt to forge provenance, trust, actor, lifecycle, link attribution, source time, source handles, or claim identity still commit only server/pipeline-derived metadata.
- Placement invocation context derives `actor_id` from server-issued `Actor::McpClient.client_id`; conversation handles, sessions, and request params do not participate in rate or idempotency keys.
- Idempotency replay is stable across calls by the same paired MCP client and isolated across different MCP client ids.
- Target authorization runs before path resolution, placement idempotency, and raw target audit fields; missing and unauthorized targets both return `target_not_found_or_unauthorized`.
- v1 target authorization tests freeze the rule: `write.workspace_place_document` grant plus existing local routable entity row is sufficient; missing entity rows use the same public error shape as future unauthorized targets.
- All placement attempt audit rows store only a server-secret HMAC target key, not raw entity id/type.
- Placement service ledger enforces 200/hour with atomic reservation semantics, including dry runs and post-schema failures.
- Concurrent service-ledger calls cannot exceed the 200/hour boundary.
- `dry_run = true` may consume placement attempt-audit and service rate rows, performs the same non-mutating validations as live writes, and writes no directory, file, placement idempotency row, lifecycle row, or ingestion run.
- Dry-run validation failures write sanitized failure audit after schema validation and return the same error codes as live validation failures.
- Dry-run receipt uses `document_handle = null`, `source_handle = null`, `source_asof = null`, `lifecycle_state = not_written`, `claim_count_produced = 0`, and a `workspace_placement_preview` cursor with no durable/content-derived handle.
- Live call writes only under workspace root and rejects path traversal, absolute paths, dot-dot, NUL, slashes in filename hints, symlink races, reparse races, parent-directory swaps, cross-device escapes, hidden-file targets, and unsupported content types.
- Path-boundary tests include handle-relative traversal/opening and symlink/reparse swap attempts on every component.
- Platforms without no-follow/no-reparse create protections fail closed.
- Category validation rejects malformed or unregistered category slugs before file write.
- Missing category returns `invalid_request_shape` before service rows, dry-run preview, path resolution, receipt rendering, or idempotency-key construction.
- Entity lookup rejects missing ids and never trusts client-supplied entity names.
- Public request rejects `source_asof_hint`; receipt/source `source_asof` is server-derived.
- Idempotent replay returns a freshly rendered scope-aware receipt for the same entity/content/content-type/category/client key and does not create a second file or second run.
- Idempotency key includes `actor_id`, normalized category, and content type; different paired MCP clients cannot replay each other's placement state, and same content/entity/dedup with different category or content type does not replay the old placement.
- Placement service tests prove no placement rate or idempotency write transaction is held across filesystem writes or `IngestPipeline::run`.
- Replay without `read.entity_names` cannot receive a `resolved_path` cached from an earlier broader grant.
- Concurrent same-key live calls produce one `in_progress` placement row; the loser receives `idempotency_in_progress`.
- Stale `in_progress` rows reconcile deterministically after expiry; they cannot wedge a key forever.
- Failed idempotency rows require a new `client_dedup_key` for retry.
- Different `client_dedup_key` values can create distinct placements for the same entity/content.
- Public `document_handle`, stored `source_handle`, and `idempotency_id` values are random or server-secret HMAC-backed and never expose raw run ids, file ids, unkeyed content/path/entity digests, or DB identifiers.
- Idempotent replay returns the stored opaque `source_handle` for live success without exposing raw `run_id` or `file_id`.
- Success receipt includes `mutation_cursor`; ability tests fail if it is omitted.
- `PlacementError` serialization includes stable `code`, safe `message`, `allowed` for `category_not_allowed`, `retry_after_seconds` for `rate_limited` / `idempotency_in_progress`, and `trace_id` for `placement_internal`; optional fields are omitted when not applicable.
- Receipt and cursor never include raw absolute path, raw `file_id`, raw `run_id`, raw `ingestion_run_id`, internal entity DB ids, raw claim text, raw provenance JSON, prompt/output bodies, or content bytes.
- W4-C adds no exported signal payloads. Existing workspace signal ids remain internal-only service payloads and are never copied into MCP receipts, tool metadata, audit rows, or user-visible surfaces.
- Ingestion path uses `WorkspaceFileKind::McpPlacement`, persists `LinkAttributionSource::McpPlacement`, and does not call `commit_claim` outside the pipeline.
- Existing entity-intake callers still persist `LinkAttributionSource::EntityIntake`; W4-C placement persists `LinkAttributionSource::McpPlacement`.
- Post-schema placement attempts write sanitized audit for success and failure with stable error code only.
- No account/Glean producer, claim lifecycle, trust recompute, signal-bus, `workspace_ingestion/mod.rs`, MCP v2 code beyond the coordinated `tool_descriptions.yaml` resource reconciliation, or WP block files in the diff.

## 11. L0 Review Matrix

Run cycles until all-pass:

- `/plan-eng-review` for service shape, idempotency, transaction/cleanup, and migration slot.
- `/plan-devex-review` for headless ability ergonomics, request/receipt/error schema, and v1.4.7 DOS-480 compatibility.
- `/cso` for MCP write boundary, path placement, rate limiting, redaction, and idempotency replay.
- Codex challenge for stale substrate assumptions, PR #355/#366 overlap, and contradiction with account/Glean/claim producer work.

## 12. Cycle 1 Findings Fold

| Finding | V0.2 fold |
| --- | --- |
| Public error taxonomy missing | Added stable codes and current `ToolError` mapping in section 5. |
| Missing `mutation_cursor` | Added receipt/cursor contract and L2 tests. |
| `dry_run` said no DB rows | Clarified that gateway/audit/conversation/rate rows may be written; only business placement/ingestion rows are skipped. |
| Dual rate limit unsupported by gateway | Kept gateway 10/minute and added service-level 200/hour ledger. |
| Request schema not frozen | Added required/closed schema constraints and entity-id acquisition rule. |
| Idempotency receipt caching leaks scope-dependent fields | Removed `receipt_json`; rows store scope-neutral facts and receipts render per current grant. |
| Concurrent idempotency lacks pending fence | Added `in_progress` placement row acquired under `BEGIN IMMEDIATE` before file write. |
| Write path reused read-only `open_validated` | Added write-side path boundary and retained `open_validated` only as post-write read verification. |
| `Transform + may_publish` lacked explicit controls | Added compensating controls. |
| Entity path source missing | Added server-side entity lookup and slug derivation rule. |
| `workspace_ingestion/mod.rs` conflict | Removed new-submodule option; use existing owner files only. |
| Migration slot v266 conflict | Moved the proposed migration away from v266 and made slot reassignment an explicit coordination gate if lower-slot plans changed. |
| #366/#355 overlap under-scoped | Expanded coordination-gated write set to context, migration, MCP handler/gateway/contract files. |

## 13. Cycle 2 Findings Fold

| Finding | V0.3 fold |
| --- | --- |
| `v280` can skip lower future migrations | Renumbered W4-C to the next contiguous slot, v262, after L2 confirmed the executable migration chain must not ship provisional v280. |
| Migration not atomic/rerun-safe | Required `BEGIN IMMEDIATE` plus `IF NOT EXISTS`, or `Migration::Fn`. |
| Public error codes hidden by rmcp transport | Made `ToolError.detail` audit-only and required a transport-visible public `code` field plus golden wire tests. |
| Service rate ledger race | Required atomic service reservation with concurrency tests. |
| Post-schema failures not audited | Added sanitized `workspace_placement_attempt_audit` table and success/failure audit requirements. |
| Idempotency handles could expose unkeyed digests | Required random UUID or server-secret HMAC-backed handles. |
| Dry-run receipt underspecified | Added exact dry-run receipt values and preview cursor shape. |
| Service receipt redaction lacked grant context | Added server-only `PlacementInvocationContext` carrying gateway-resolved grant facts. |
| `allowed_modes` conflicted with service writes | Made the ability Live-only; `dry_run` is a live preview. |
| Same content/category mismatch could replay old placement | Added `content_type` and normalized category to the idempotency key. |
| `in_progress` rows could wedge forever | Added `stale_after` and deterministic stale reconciliation. |
| PR #355 handler context not binding enough | Made post-#355 `McpHandlerContext` use and no-direct-DB-open gate explicit. |
| Link attribution only checked file kind | Required `LinkAttributionSource::McpPlacement` persistence and tests. |
| Non-Unix write protections could silently downgrade | Required fail-closed behavior without no-follow/no-reparse equivalents. |
| `source_asof_hint` had ambiguous audit semantics | Removed it from the public schema. |
| Missing grant/tools-list/inventory gates | Added exact grant provisioning, `tools/list`, and `scripts/check_ability_inventory.sh` tests. |

## 14. Cycle 3 Findings Fold

| Finding | V0.4 fold |
| --- | --- |
| W4-C crossed v1.4.7 DOS-480 ownership | Narrowed W4-C to v1.4.5 ability/service/receipt/error contract; MCP v2 handler, taxonomy, grants, transport, tools/list, and conversation handles are out of scope. |
| PR #355 changes more than handler context | Removed W4-C dependency on MCP v2 internals; DOS-480 must restore/define manifest grants, short-window rate limits, write audit, tools/list, public error transport, and conversation handle round-trip before MCP exposure. |
| Placement idempotency omitted actor identity | Added a client-derived identity to the idempotency key in V0.4; V0.6 renames and freezes it as server-issued `actor_id`. |
| Error contract froze only `code` | Froze the full `PlacementError` envelope: `code`, safe `message`, optional `allowed`, optional `retry_after_seconds`, and optional `trace_id`. |
| Conversation-handle transport gap | Made conversation-handle behavior explicitly DOS-480-owned and removed W4-C reliance on rmcp round-trip behavior. |

## 15. Cycle 4 Findings Fold

| Finding | V0.5 fold |
| --- | --- |
| Optional `category` conflicted with idempotency/receipt requirements | Made `category` required in v1 and required missing/null/empty category to return `invalid_request_shape` before service rows, dry-run preview, path resolution, receipt rendering, or idempotency-key construction. |
| Error sample serialized optional fields as null | Chose the omit-when-not-applicable contract and updated the sample, rules, and L2 tests accordingly. |

## 16. Cycle 5 Findings Fold

| Finding | V0.6 fold |
| --- | --- |
| Unsupported future schema versions had no stable error | Added `unsupported_schema_version`; missing/non-integer versions remain `invalid_request_shape`; added golden tests. |
| Client-visible validation rules were still implementation-defined | Froze exact base64 variant, decoded byte limit, filename/dedup regexes, trim/null behavior, and failure-code mapping. |
| Live `source_handle` nullability and raw run id leakage | Made live success `source_handle` non-null and public-opaque; banned raw `run_id`/`ingestion_run_id` from receipt and cursor. |
| `client_id` conflated actor/client identity | Renamed placement identity to `actor_id`, sourced only from server-issued `Actor::McpClient.client_id`; excluded conversation/session/request data. |
| Dry-run returned before validation | Moved dry-run after content, target, category, filename, and non-mutating path-boundary validation; forbade dry-run directory/file creation. |
| Write path lacked race-safe traversal contract | Required handle-relative traversal/opening with per-component no-follow/no-reparse checks, retained parent handles, and swap tests. |
| Target existence check could become enumeration oracle | Added server-side target authorization and non-enumerating `target_not_found_or_unauthorized`; raw target audit fields only after authorization. |
| Placement transactions could span file I/O/pipeline | Required short transactions for rate, idempotency insert/conflict, success/failure updates, and audit; no placement write tx across file I/O or `IngestPipeline::run`. |
| `McpPlacement` link attribution seam was missing | Added implementation seam to pass/set `LinkAttributionSource::McpPlacement` rather than relying on the current `EntityIntake` hardcode. |
| v1.4.5/v1.4.7 docs and live MCP placeholder were stale | Added supersession requirements and a W4-C L1 blocker until downstream docs/taxonomy match or scope is coordinated. |
| #366 overlap omitted shared wiring files | Added `src-tauri/src/lib.rs`, `src-tauri/src/services/mod.rs`, and workspace-ingestion contract/link/wiring files to the coordination-gated write set. |

## 17. Cycle 6 Findings Fold

| Finding | V0.7 fold |
| --- | --- |
| v1.4.5 W4-C wave section still exposed old active contract | Replaced the old contract/tests/done-when block with `SUPERSEDED - DO NOT IMPLEMENT` and a pointer to this packet. |
| Existing MCP placement placeholder still exposes old `topic`/`content`/`document_id` contract | Kept W4-C blocked from implementation until DOS-480 or an explicit coordinated amendment updates/removes `tool_descriptions.yaml`; W4-C still does not edit MCP v2 files without coordination. |
| Obsolete W4-C MCP write allowlist gate referenced a missing script and unowned surface | Replaced the W4-C allowlist invariant/checklist entries with a no-direct-MCP-v2-edits gate and DOS-480 ownership of placement exposure. |
| `content_b64` lacked a pre-decode/transport cap | Added encoded `content_b64` max and a W4-C secondary serialized-argument cap; clarified that DOS-480 owns the raw MCP body cap, bounded parsing requirement, and pre-deserialization malformed-payload tests. |
| JSON content could forge provenance/trust metadata | Declared all placement content untrusted document bytes and added forged-JSON tests proving authoritative metadata is server/pipeline-derived. |
| `source_handle` replay/storage was inconsistent | Added `source_handle TEXT` to placement idempotency storage and replay tests for the stored opaque handle. |
| DOS-480 error transport could hide `PlacementError.code` | Added a downstream precondition that DOS-480/W1-A owns a public domain-error carrier and golden mappings for every placement code. |
| Target authorization had no implementable v1 rule | Froze v1 target auth as write grant plus existing local routable entity row; future per-entity ACLs require L0 amendment. |

## 18. Cycles 7-10 Preflight Fold

| Finding | V0.8 fold |
| --- | --- |
| PR #366 has merged and dev is rebaseable | Rebased W4-C onto `public/dev` at `52c9255c` and updated active coordination to treat account/Glean work as merged context, not an open dirty branch. |
| Existing MCP placement placeholder still exposed the old contract after explicit coordination | Updated only `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` to mirror the W4-C request/receipt shape and preserve the no-MCP-code boundary. |
| Cycle-7 review found the resource example still drifted from the receipt | Updated the coordinated resource metadata to use `mcp_placement`, `ingested`, `workspace_placement`, `idempotency_id`, `resolved_path = null`, and a dry-run receipt example. |
| v1.4.7 W1-B still named the pre-W0 resource path | Reconciled v1.4.7 references to the live `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` path. |
| PR #366 was still named as a future migration blocker | Reframed the migration gate around the current live max v261 plus v1.4.6/unmerged lower slots. |
| Attempt audit target privacy was ambiguous | Made placement attempt audit HMAC-only for all target outcomes; raw target tuple is allowed only in the idempotency row after authorization. |
| Existing workspace signal ids could be mistaken for exported payloads | Classified workspace signal ids as internal service payloads and banned copying them into MCP receipts, metadata, audit rows, or user-visible surfaces. |
| Public receipt `entity_id` conflicted with the internal-id ban | Clarified that public `entity_id` is a service-owned opaque entity handle from authorized read/search tools and distinct from workspace file/run ids and internal signal payload ids. |
| Resource metadata cursor schema allowed incomplete live cursors | Split the coordinated metadata cursor schema into live and dry-run variants so live cursors require `document_handle`, `source_handle`, and `idempotency_id`. |
| W4-C appeared circularly dependent on v1.4.7 MCP actor substrate | Cited current-dev `Actor::McpClient` / `ActorKind::McpClient` as already present substrate that W4-C consumes without editing gateway/auth/transport code. |
| Migration ownership remained unresolved | Assigned W4-C to v262 after PR #366 landed and current `dev` had live max v261; later migration branches must rebase and take the next contiguous slot. |
| v1.4.7 migration plan still reserved stale v220-v239 slots | Updated v1.4.7 wave text so v220-v239 is explicitly historical/invalid on current `dev`; future v1.4.7 migrations must claim slots above the live max and active reservations. |
| v1.4.7 actor-substrate gates still treated `Actor::McpClient` as future work | Updated v1.4.7 ADR/W0/L0 language to consume and document the already-landed `Actor::McpClient` / `ActorKind::McpClient` substrate instead of assigning a later lane to introduce it. |
| Dry-run receipt handle wording contradicted required null fields | Clarified that dry-run receipts include required top-level `document_handle = null` and `source_handle = null`, while the preview cursor omits durable handles and `idempotency_id`. |
| v1.4.5 migration table still assigned DOS-474 to W4's old v266-v267 block | Added an explicit DOS-474/W4-C exception to the v1.4.5 migration summary/table/footer, then renumbered the executable migration to v262 during L2. |
| v1.4.7 `ToolDescription` field casing contradicted the checked-in YAML | Updated the v1.4.7 contract snippet to serde-rename `scopes_required` and `expected_response_shape` to the existing YAML wire fields `scopesRequired` and `expectedResponseShape`. |

## 19. Cycle 11 L0 Verdict

Cycle 11 passed with no blocking findings from Eng, DevEx, Security, or Codex challenge.

| Reviewer | Verdict | Notes |
| --- | --- | --- |
| Eng | PASS | Confirmed dry-run receipt shape, migration-slot coordination, ToolDescription/YAML compatibility, and post-PR-366 boundaries. |
| DevEx | PASS | Confirmed request/receipt/error schema, YAML examples, W4-C/DOS-480 ownership split, and migration-slot clarity. Noted historical optional-category wording as advisory only because this packet and the W4-C lane supersede it. |
| Security | PASS | Confirmed privacy-safe receipts/cursors, opaque handles, dry-run safety, target-auth ordering, service-rate boundary, and no account/Glean/MCP handler scope drift. |
| Codex challenge | PASS | No blockers found. |

L1 implementation remains subject to the cross-session checkpoint before substrate or bridge files are edited.
