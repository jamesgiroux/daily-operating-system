# DOS-777 / W5 L0 Packet - MCP-First Parity, Feedback Writes, and Source-Aware Reads

- **Version:** v1.4.9 - W5 MCP-first parity
- **Primary issue:** [DOS-777](https://linear.app/a8c/issue/DOS-777)
- **Related issues:** DOS-171, DOS-479, DOS-172, DOS-481, DOS-482, DOS-169, DOS-170, DOS-8. DOS-173 resource support is explicitly moved out of W5 by cycle 2; it needs its own transport/resource packet.
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Scope tier:** Wave-scope MCP, claim/provenance, read/write substrate. Formal Wave L0 uses `/codex challenge`, two planning reviewers (`ce-security-lens-reviewer` and `ce-feasibility-reviewer`), and mandatory K-in. API, product, and data-migration specialist findings are subchecks folded into those planning verdicts rather than extra ladder slots. Add CSO/daily-briefing security review only if daily briefing is reintroduced.
- **Status:** L0 approved on 2026-06-04 after cyclic review. W5 L1 remains gated on W2/DOS-833 merge and rebase before gateway/session/transport/conversation-handle/target-handle implementation.

---

## Section 0 - Origination, Scope, and Trust Topology

**Origination class:** Extension. W5 is the headless half of the v1.4.9 judgment loop: the same claim, trust, provenance, feedback, and source-aware read substrate the app uses must work through MCP for data that is allowed to cross the MCP boundary.

**Headline contract:** A local MCP client can read DailyOS intelligence with the same claim/trust/provenance semantics as the app for MCP-eligible data, submit typed corrections through the same `claim_feedback` path as the app, and submit the ADR-0128 Section D note/action/action-status writes through `services::*`. MCP never becomes a raw DB API, a generic filesystem search layer, or a broad mutation surface.

**Trust topology:**

1. **Local app/file surfaces:** same OS user, local personal runtime.
2. **MCP stdio surface:** same OS user, but the MCP host model is a third-party AI consumer. MCP remains an egress boundary: `Confidential` and `UserOnly` claim content never crosses MCP.
3. **Remote MCP:** out of scope. A future remote transport needs fresh L0/ADR work.

**Non-negotiables:**

- Preserve `Actor::McpClient` for MCP-originated reads and corrections. MCP feedback must not be silently reclassified as `Actor::User`.
- Route all mutations through `services::*`; no MCP handler may write application tables directly.
- Reuse the shipped feedback writer, receipt validation, action services, ability runtime, and MCP v2 gateway where they fit. New primitives require substrate-type grep evidence.
- Filter sensitivity before pagination, truncation, projection, audit detail, provenance detail, handle minting, and host-model output.
- Treat external/workspace/customer content as evidence only. MCP tool params and source text cannot choose actor, sensitivity, source authority, or tool side.
- Do not advertise catalog-only tools as usable. A listed tool must have a registered handler, tool description, selection fixtures, and tests.

### Section 0.1 - Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w5-mcp-parity-l0` from `public/dev`. Current code has a v2 MCP gateway, taxonomy catalog, local stdio path, and two registered handlers (`dailyos.read.account_status`, `dailyos.write.place_document`). The W5 catalog already names the broader surface, but most W5 handler modules are placeholders. L1 must rebase onto the latest W1-W4 authority before implementation and reconcile any changed tool names or ADR amendments.

W2/DOS-833 is a hard W5 entry gate for gateway, session, transport, conversation-handle, and MCP target-handle work. W5 does not itself decide the MCP auth ceremony. Until W2/DOS-833 merges a superseding ADR, the currently accepted ADR-0102/ADR-0128 pairing, HMAC, manifest, scope, and conversation-handle rules remain authoritative. W5 L0 approval is therefore approval of the post-W2 packet shape, not permission to begin L1 before W2 authority lands. L1 must rebase against the merged W2/DOS-833 authority before implementation; if W2 does not land the local-stdio auth model this packet expects, W5 returns to L0 rather than implementing against future authority. `blocked_by_w2` is not an acceptable W5 proof status for MCP writes. W5 preserves `Actor::McpClient`, MCP egress sensitivity gates, audit attribution, and hostile-input handling.

This packet chooses the open W5 actor decision: extend the feedback writer and receipt path to accept explicit MCP-client feedback attribution. Do not encode MCP corrections as app-user feedback with only side metadata.

### Section 0.2 - L0 Challenge Cycle 1 Decisions

Cycle 1 failed the draft packet on four L0 blockers that are now explicit W5 contracts:

1. **MCP feedback target binding needs a buildable opaque target-handle contract.** MCP reads intentionally omit raw claim ids, while `ReceiptTarget::Claim` needs an internal `claim_id`. W5 must mint server-issued feedback target handles from prior MCP reads and resolve them internally before calling receipt feedback. Guessed raw ids stay rejected without existence leaks.
2. **Daily briefing MCP exposure needs security re-approval.** `get_daily_briefing` is currently `allowed_actors = [User]` with `mcp_exposure = None`. W5 cannot expose or wrap it for MCP until a DailyOS security review approves the exact projected envelope and transitive sensitivity behavior.
3. **W2/DOS-833 is an entry gate, not a rebase note.** W5 cannot safely touch gateway/session/transport/target-handle continuity until the local-stdio auth right-size authority is merged and this packet is rebased.
4. **`dailyos.submit.action_status` public semantics must be exact before listing.** The catalog words `done`, `deferred`, and `dropped` do not match the existing persisted action status vocabulary. W5 must define service-backed semantics or hide/rename the option before exposure.

### Section 0.3 - L0 Challenge Cycle 2 Decisions

Cycle 2 found additional L0 blockers that are now explicit W5 contracts:

1. **No raw internal IDs across MCP.** W5's no-raw-id rule applies to claim, source, receipt, action, entity, subject, meeting, workspace, and resource identifiers. Public request/response fields use server-issued handles such as `feedback_target_handle`, `action_handle`, `entity_handle`, and `source_provenance_handle`, or natural-language lookup fields where a handler deliberately supports them.
2. **All mutation handles share the same replay contract.** Feedback handles, action handles, entity handles, and place-document/source handles bind client, conversation, tool, target kind, version/watermark, sensitivity/render policy, expiry, revocation, and stale-policy behavior. A mutation cursor is mandatory for successful writes, but it is not a target handle.
3. **Daily briefing is cut from W5 implementation scope.** The existing DTO contains raw meeting ids, linked entity ids, superseded claim ids, source ids, and meeting rows that lack a W5 MCP sensitivity classifier. W5 removes `dailyos.read.daily_briefing` from `tools/list` and does not register a compatibility handler. Direct invocation returns the existing gateway unknown/exposure error with no data. A later packet may define an MCP-safe envelope.
4. **Meeting briefing is cut from W5 implementation scope.** `prepare_meeting` is a transform and current meeting refresh paths are mutating. W5 removes `dailyos.read.meeting_briefing` from `tools/list` and does not register a compatibility handler. Direct invocation returns the existing gateway unknown/exposure error with no data until a read-only producer and response contract exist.
5. **MCP resources are cut from W5 implementation scope.** The current v2 transport advertises tools only and `tool_resources.rs` is placeholder-only. W5 provides source provenance through `dailyos.read.workspace_source_provenance` as a tool, not MCP resource list/read.
6. **Public result shapes are pinned.** Domain target problems return typed success payload statuses, not ad hoc `ToolError` variants. Gateway auth failures keep using existing gateway errors.

### Section 0.4 - L0 Challenge Cycle 3 Decisions

Cycle 3 found final contract blockers that are now W5 decisions:

1. **Hidden tools are truly hidden.** W5 does not add hidden-but-invocable compatibility handlers. Daily briefing, meeting briefing, portfolio attention, MCP resources, place document, and workspace-memory search are removed from `tools/list`; direct invocation uses the existing gateway unknown/exposure error path and returns no partial data.
2. **`dailyos.submit.claim_feedback` gets a narrow ADR-0128 amendment.** The concrete tool name is approved only as the typed ADR-0123 feedback carrier; it does not broaden MCP writes beyond feedback and submit-class note/action/action-status writes.
3. **Audit privacy is part of W5.** MCP audit rows must not persist raw params, raw responses, raw handles, or hidden source/entity/claim/action details. W5 changes the gateway/audit detail path, not only host-visible output.
4. **Target handles use server-side registry state.** W5 uses a persistent `mcp_target_handles` registry so expiry, revocation, last-used, stale-policy, and cross-client rejection are enforceable. This requires data-migration review.
5. **Workspace memory search and portfolio attention are cut from W5.** No current claim-backed workspace-memory search producer exists, and current recommendation/salience producers are not MCP-visible. `dailyos.search.workspace_memory` and `dailyos.read.portfolio_attention` are removed from W5 `tools/list` until later packets name real MCP-safe producers and response contracts.
6. **Submit schemas return handles, not ids.** `dailyos.submit.note` and `dailyos.submit.action` cannot return `note_id` or `action_id`; they return opaque handles and mutation cursors.
7. **`dropped` maps exactly.** For accepted/open actions it stores `cancelled`; for backlog/suggested rows it uses the existing suggested-action dismissal path and stores `archived` while returning `semantic_status = "dropped"`.

---

## Section 1 - Existing Substrate W5 Must Consume

### Section 1.1 - MCP v2 Gateway and Catalog

Current MCP v2 pieces already provide the dispatch shape W5 should extend:

- `src-tauri/src/services/mcp_v2/gateway.rs` handles local stdio calls, registered-handler lookup, invocable exposure, reserved-param rejection, envelope-level conversation handles, audit attribution, mutation-cursor checks, and invoked/rejected signals.
- `src-tauri/src/services/mcp_v2/contracts.rs` defines `McpActor::Client`, `McpToolRequestEnvelope`, `McpToolResponseEnvelope`, `McpToolResult`, `ToolDescription`, `Side`, and `McpToolHandler`.
- `src-tauri/src/services/mcp_v2/actor_policy.rs` projects a resolved local MCP call to `Actor::McpClient`.
- `src-tauri/src/services/mcp_v2/handlers/registration.rs` registers only `dailyos.read.account_status` and `dailyos.write.place_document` on this base.
- `src-tauri/src/services/mcp_v2/handlers/tool_account_status.rs` is the reference handler: it builds a request-scoped `Actor::McpClient`, invokes the ability runtime, uses `BridgeSurface::McpTool`, and projects claim-shaped output through `runtime_projection`.
- `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml` names ten tools: account status, daily briefing, meeting briefing, portfolio attention, workspace memory search, workspace source provenance, place document, note submit, action submit, and action status submit.
- Placeholder files exist for briefing, note, create-action, update-action-status, pagination, resources, workspace search, workspace source provenance, and portfolio handlers.

W5 must close the gap between catalog and runtime. It is invalid for `tools/list` to present a tool as ready when the handler is still a placeholder or routes to legacy/static MCP v1 code.

### Section 1.2 - Read Abilities and Projection

The useful read substrate already exists, but exposure is uneven:

- `get_entity_intelligence` is MCP-invocable, accepts `McpClient`, returns claim-shaped facts/open loops/relationships/touchpoints/records with trust, sensitivity, source-as-of, provenance, and section state.
- `runtime_projection.rs` already summarizes runtime envelopes for MCP with compact provenance, redacted source ids, bounded arrays, truncation metadata, caveats, and `rawClaimIdsIncluded = false`.
- `list_open_loops` is MCP-invocable and filters claims through the prompt-input sensitivity gate before producing open-loop data.
- `get_daily_briefing` exists and composes daily readiness, meeting prep status, and entity intelligence. It currently has `allowed_actors = [User]` and `mcp_exposure = None`; W5 does not expose it. The current DTO includes raw meeting/source/claim/entity identifiers and meeting rows without a W5 MCP sensitivity classifier, so daily briefing stays hidden/typed-unavailable until a later security-approved packet defines an MCP-safe envelope.
- `get_daily_readiness` is already MCP-invocable, but it is not a replacement for the W5 daily briefing envelope.
- `prepare_meeting` exists as a transform ability and is not itself the W5 meeting-read handler. W5 does not expose meeting briefing because no approved read producer and MCP-safe response contract exists on this base.
- Recommendation/salience read abilities and action/open-loop evidence exist, but portfolio attention is not a ready MCP handler on this base. Recommendation abilities are not MCP-exposed and current render visibility excludes `ActorKind::McpClient`, so W5 cuts `dailyos.read.portfolio_attention` from the advertised surface.
- Workspace/source registry, source-management ledger, markdown-preview, and workspace-ingestion graph services expose source handles and lifecycle/provenance data. They can support `dailyos.read.workspace_source_provenance`; they do not provide a claim-backed `dailyos.search.workspace_memory` producer.

W5 should extend the v2 handler pattern over these abilities/services. It should not teach MCP to read legacy reports, exported files, or ad hoc cached prose as authority.

### Section 1.3 - Feedback Writer and Actor Blocker

Typed claim feedback already exists:

- ADR-0123 defines the 10 `FeedbackAction` values.
- `src-tauri/src/services/claim_receipt/feedback.rs::submit_claim_feedback` validates the receipt target, checks surface access, validates and sanitizes action metadata, mints idempotency, and calls `services::claims::record_claim_feedback`.
- `services::claims::record_claim_feedback` inserts append-only `claim_feedback`, updates verification/lifecycle state, emits signals, bumps invalidation/version state, and queues repair where required.

The blocker is real:

- `submit_claim_feedback` denies non-user render actors.
- `record_claim_feedback` maps `user|human` to User, `system|...` to System, `agent|ai|...` to Agent, and currently accepts feedback only from User.

W5 resolves this by extending the receipt and writer path to accept MCP feedback as a distinct local authorized actor class. The stored feedback must preserve MCP attribution, for example `actor = "mcp_client"` or `actor = "mcp_client:<opaque-client-id>"`, with `actor_id` carrying the opaque client id when available. Any conversation handle, tool name, and surface metadata must be server-derived and sanitized; callers must not be able to supply or override it.

Do not solve this by passing `Actor::User`, `actor = "user"`, or `ServiceContext::with_actor("user")` for MCP-origin corrections unless a later L6 decision explicitly reverses this packet.

### Section 1.4 - ADR-0128 Write Surface

ADR-0128 Section 5 already frames MCP writes as feedback: corrections, dismissals, corroborations, contradictions, and tombstones flowing through claim feedback. The 2026-05-19 Section D amendment separately names submit-class writes: `dailyos.submit.note`, `dailyos.submit.action`, and `dailyos.submit.action_status`.

W5 therefore needs two write lanes:

1. **Typed claim feedback:** add an explicit MCP tool for ADR-0123 feedback named `dailyos.submit.claim_feedback`. Generic note text must not be inferred into claim feedback.
2. **Submit-class writes:** reconcile DOS-169 and DOS-170 against Section D. `dailyos.submit.action` and `dailyos.submit.action_status` route through `services::actions`; `dailyos.submit.note` routes through approved claim/source services.

This packet includes the required ADR-0128 clarification in `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md`. L1 must preserve that text before registering `dailyos.submit.claim_feedback`:

> `dailyos.submit.claim_feedback` is the concrete ADR-0123 typed feedback carrier for MCP. It is not a fourth submit-class write and does not authorize claim creation, direct claim edit, file generation, calendar/message mutation, external writes, or free-form note inference. It may only target a server-issued `feedback_target_handle` from an MCP read and must route through the existing receipt/claim-feedback services with MCP attribution.

### Section 1.5 - Action Services

`src-tauri/src/services/actions.rs` already validates and mutates actions through service-owned paths:

- `create_action` validates bounded text, priority, due date, entity ids, source label, inserts through `ActionDb`, syncs action claims, emits action signals, scans decisions, and best-effort links objectives.
- `complete_action` and `reopen_action` update status through service paths, sync claims, and emit signals.
- `reject_suggested_action` and `dismiss_suggested_action` archive/suppress suggested actions with distinct trust/signal semantics.
- `update_action` validates field updates and applies them through a service boundary.

The catalog currently advertises `action_status` values `done`, `deferred`, and `dropped`, while backend status constants are `completed`, `unstarted`, `started`, `cancelled`, and `archived`. W5 must use the exact service-backed semantic mapping in §2.6 before exposing the tool. `deferred` is not a persisted action status; `dropped` is not a raw SQL archive shortcut.

### Section 1.6 - Sensitivity, Provenance, and Prompt-Injection Guardrails

Prior K-in entries matter here:

- `prompt-channel-sensitivity-class-sweep-2026-05-18.md` centralized prompt-channel sensitivity gates after repeated leaks. W5 must use the existing gate, not copy sensitivity matches into handlers.
- `k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` records the failure mode of grepping only for a proposed primitive name. W5 K-in must search for substrate types: `record_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`, `McpToolHandler`, `ToolDescription`, `Actor::McpClient`, `Paginated`, `Cursor`, `source_asof`, `render_policy`, `services::actions`, and workspace source/provenance readers.
- ADR-0093 remains relevant for workspace memory and note/action text. Source text and tool text are evidence, not instructions.

MCP read handlers must render through approved policy and strip raw identifiers that the current MCP projection intentionally omits. MCP feedback handlers must sanitize free text through the receipt path.

---

## Section 2 - Chosen Architecture

### Section 2.1 - Tool Surface

W5 ships MCP v2 parity through registered, catalog-backed handlers for the MCP-eligible subset:

1. `dailyos.read.account_status` - already implemented; keep as the reference pattern.
2. `dailyos.read.daily_briefing` - out of W5 implementation scope. The tool is absent from `tools/list` and not registered. Direct invocation follows the existing gateway unknown/exposure error path with no handler compatibility and no data. Do not flip `get_daily_briefing` to `Actor::McpClient` or `mcp_exposure = Invocable` in W5.
3. `dailyos.read.meeting_briefing` - out of W5 implementation scope. The tool is absent from `tools/list` and not registered. Direct invocation follows the existing gateway unknown/exposure error path with no handler compatibility and no data. Do not use `prepare_meeting` or mutating meeting refresh paths as a read handler.
4. `dailyos.read.portfolio_attention` - out of W5 implementation scope. Recommendation/salience producers are not MCP-exposed on this base, and render visibility excludes `ActorKind::McpClient`. Remove it from `tools/list` until a later packet names an MCP-safe producer and response contract.
5. `dailyos.search.workspace_memory` - out of W5 implementation scope. The current v2 handler is placeholder-only and no claim-backed workspace-memory producer exists. Remove it from `tools/list` until a later packet names the producer and response contract.
6. `dailyos.read.workspace_source_provenance` - return display-safe source provenance for a previously returned source/provenance reference; no raw source ids, paths, or forbidden sensitivity tiers.
7. `dailyos.submit.claim_feedback` - new explicit typed feedback tool, authorized by the narrow ADR-0128 clarification in §1.4.
8. `dailyos.submit.note` - submit a bounded note/observation through claim/source services; do not infer typed feedback.
9. `dailyos.submit.action` - submit a bounded action through `services::actions::create_action`.
10. `dailyos.submit.action_status` - expose only the service-backed semantics in §2.6. If any requested status cannot be backed by service behavior, change or hide the catalog vocabulary before listing.

`dailyos.write.place_document` is already implemented, but it cannot remain an advertised W5 tool with raw `entity_id` or caller-controlled idempotency semantics. W5 removes it from the advertised parity surface. A later packet may reintroduce it with `entity_handle`, source handles, server-derived idempotency, and the registry/audit rules below.

`dailyos.read.account_status` remains advertised, but W5 upgrades its public response schema before it can serve as the reference handler:

- response `schemaVersion` becomes `mcp.account_status.v2`;
- `subject.id`, `resolution.resolvedEntityId`, and ambiguous candidate `entityId` are removed;
- resolved subjects return `entity_handle` plus display-safe `displayLabel`, `entityType`, `resolutionKind`, `matchConfidence`, and caveats;
- ambiguous candidates return display labels and `entity_handle` values only when those candidates are MCP-visible; otherwise candidates are collapsed to an indistinguishable clarification response without ids or handles;
- provenance/follow-up fields use `source_provenance_handle` and `feedback_target_handle`, not invocation ids or raw params;
- catalog examples, return schema, golden fixtures, and host-selection evals are updated together so v1 raw-id shapes cannot remain silently accepted.

### Section 2.1.1 - Daily Briefing MCP Security Gate

`get_daily_briefing` is currently user-only. Exposing it to MCP changes the egress boundary for a composed surface. W5 does not expose it; this section records the minimum future gate so L1 cannot accidentally register it.

Required before any later packet flips `mcp_exposure`, adds `Actor::McpClient`, or registers `dailyos.read.daily_briefing` as invocable:

- run the dedicated DailyOS security review (`/cso`) or project-approved equivalent against an exact field allowlist/denylist;
- prove the wrapper uses the existing producer plus MCP render policy, not a parallel SQL/prose path;
- replace or omit raw `meeting_id`, `linked_entity_id`, `subject_id`, `proposal_id`, `superseded_claim_ids`, `claim_id_a`, `claim_id_b`, source ids, `source_asof_inputs.source`, and any raw entity/meeting/source identifiers with server-issued handles or aggregate counts;
- prove meeting rows have an MCP sensitivity classifier before title, attendee hint, entity hint, prep status, freshness/advisory id, pagination total, or source label can cross MCP;
- prove Confidential/UserOnly data is removed before section composition, pagination, truncation metadata, provenance/source labels, entity expansions, meeting prep snippets, agenda/prep user-authored layers, and surface-hash explanations;
- prove derived summaries, paraphrases, recommendation explanations, meeting titles/attendee hints, and source labels cannot imply hidden UserOnly/Confidential details;
- document whether any section is omitted, degraded, or returned with a future approved blocked-status contract for MCP.

For this W5 packet, `dailyos.read.daily_briefing` is absent from `tools/list` and not registered. Reintroducing it as an advertised tool requires a later approved packet.

### Section 2.2 - Feedback Path

The MCP typed feedback flow is:

1. Handler receives a `McpActor::Client` from the v2 gateway.
2. Handler validates params against the catalog schema and rejects caller-supplied actor, surface, sensitivity, source authority, idempotency key, granted scopes, or conversation internals.
3. Handler resolves a server-issued `feedback_target_handle` (`McpFeedbackTargetHandle`) from a prior MCP read and rejects guessed/out-of-scope raw internal ids with the typed unavailable shape in §2.7.
4. The server derives MCP origin metadata from the envelope: client id, conversation handle, tool name, and `ClaimDismissalSurface::McpTool`.
5. Handler builds a receipt-shaped `ClaimFeedbackRequest` with ADR-0123 action metadata.
6. `claim_receipt::feedback::submit_claim_feedback` admits a new MCP render actor path and keeps envelope target binding, sanitizer, idempotency, and surface checks.
7. `record_claim_feedback` accepts the explicit MCP actor class for feedback only and writes append-only `claim_feedback` with MCP attribution. The current hardcoded `ServiceContext::with_actor("user")` feedback bridge must be replaced or wrapped so writer/audit attribution remains MCP-specific.
8. Existing feedback signals, invalidation, repair, and receipt re-render behavior run unchanged.
9. On success, the handler returns a bounded response with mandatory `mutation_cursor`, lifecycle/repair flags, sanitized warnings, optional follow-up `feedback_target_handle` for the re-rendered target, and no raw claim/source/entity/action ids.

This preserves actor truth: the correction came from a local MCP client acting for the user through a host model, not directly from the app user.

#### Section 2.2.1 - Opaque MCP Target Handles

W5 target binding is handle-first. MCP hosts never submit raw `claim_id`, raw source id, raw receipt source ref, raw `action_id`, raw account/person/project/meeting/entity ids, raw workspace ids, or internal field ids.

Every W5 read handler that returns a feedback-eligible claim, open loop, receipt, provenance item, entity/subject reference, meeting reference, or action-backed claim returns the relevant server-issued handle:

- `feedback_target_handle` for `dailyos.submit.claim_feedback`;
- `action_handle` for `dailyos.submit.action_status`;
- `entity_handle` for entity-scoped reads or writes;
- `source_provenance_handle` for source/provenance detail;
- `workspace_source_handle` for workspace/source placement or source-aware memory results.

`receipt_handle` is not a public W5 feedback field. If a later read-only receipt lookup needs a handle, it gets a separate contract and cannot be accepted by `dailyos.submit.claim_feedback`.

Handles are non-semantic to the host and are implemented as server-side registry rows bound to `mcp_conversation_handle`. Stateless encrypted tokens are rejected for W5 because they cannot support per-handle last-used, revocation, and stale-policy behavior.

The handle contract includes:

- public field name (`feedback_target_handle`, `action_handle`, `entity_handle`, `source_provenance_handle`, or `workspace_source_handle`);
- registry `handle_id`;
- bound `client_id` and `conversation_handle`;
- originating tool name, result item path, target kind, field path, and subject hash;
- internal target reference available only to server code, such as `ReceiptTarget::Claim { claim_id, subject, field_path }`, action id, entity id, meeting id, or source id;
- render-policy version, sensitivity tier at mint time, source-as-of/provenance hash, and substrate watermark or claim version;
- mint time, last-used time, expiry/revocation behavior, and PII-safe failure reason.

The required registry table is `mcp_target_handles`, reserved as registered schema version `290` for W5 after rebase, per `.docs/plans/v1.4.9-waves.md`. The registered version is authoritative; L1 must name the SQL file according to the active `migrations.rs` convention on the rebased base and explicitly reconcile `docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md` during data-migration review. If a newer base has already claimed registered version 290, L1 must renumber the slot in the wave plan and update this packet before coding; it may not silently share a slot.

Pinned SQL shape:

```sql
CREATE TABLE IF NOT EXISTS mcp_target_handles (
  handle_lookup_hash TEXT PRIMARY KEY,
  client_id TEXT NOT NULL,
  conversation_handle_hash TEXT NOT NULL,
  originating_tool TEXT NOT NULL,
  result_item_path TEXT NOT NULL,
  target_kind TEXT NOT NULL CHECK (
    target_kind IN ('claim', 'action', 'entity', 'source_provenance', 'workspace_source')
  ),
  target_ref_ciphertext BLOB NOT NULL,
  target_ref_key_version INTEGER NOT NULL,
  render_policy_version TEXT NOT NULL,
  sensitivity_tier TEXT NOT NULL,
  provenance_hash TEXT NOT NULL,
  target_watermark_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_used_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  revoked_at TEXT,
  revoked_reason_code TEXT
);

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_client_conversation
  ON mcp_target_handles (client_id, conversation_handle_hash, originating_tool);
CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_expiry
  ON mcp_target_handles (expires_at);
CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_revoked
  ON mcp_target_handles (revoked_at)
  WHERE revoked_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_target_watermark
  ON mcp_target_handles (target_kind, target_watermark_hash);
```

Public handles are random opaque strings such as `mth_<base64url-random>`. The table stores only `handle_lookup_hash`, a keyed HMAC-SHA256 of the public handle. Audit rows also store only handle hashes/reason codes, never public handle strings. `target_ref_ciphertext` is AEAD-encrypted JSON containing internal target ids and field paths; AAD includes `client_id`, `conversation_handle_hash`, `originating_tool`, `target_kind`, and `render_policy_version`. `target_ref_key_version` records the local encryption key version so L1 can rotate keys without changing public handle shape.

`target_watermark_hash` is a keyed HMAC-SHA256 over target-kind-specific watermark material. The raw material is computed transiently during mint/resolve and is never stored in plaintext or indexed columns. Watermark material by `target_kind`:

- `claim`: claim id plus claim version/lifecycle watermark used by the receipt/runtime projection.
- `action`: action id plus `updated_at`, stored status, and action-claim sync version where available.
- `entity`: entity type plus entity id plus entity `updated_at`/archive state.
- `source_provenance`: source/provenance ref plus source-as-of, source lifecycle state, and provenance hash.
- `workspace_source`: workspace source handle plus source lifecycle version, source policy state, and file/source-as-of hash.

If the HMAC of the current target watermark material differs from the stored `target_watermark_hash`, handle resolution returns `unavailable` with `refresh_required = true` for read-side lookups or the submit/write `unavailable` payload with no-op `mutation_cursor`; it does not attempt a best-effort remap.

Retention and cleanup:

- Initial `expires_at` is `created_at + 24 hours` or the conversation expiry, whichever is earlier.
- Successful resolution updates `last_used_at` and may extend `expires_at` to at most 24 hours from that use, never beyond the conversation expiry.
- Gateway startup and post-invocation cleanup delete expired rows older than 7 days and revoked rows older than 7 days. Fresh expired/revoked rows stay briefly so repeated stale handles return indistinguishable `unavailable` without recreating state.
- Rollback before W5 ships drops `mcp_target_handles` and indexes because no released tools depend on it. Rollback after W5 ships disables the W5 tools and leaves the inert table for cleanup; it must not expose raw target refs or require destructive data surgery.

Raw target ids never leave encrypted server storage or audit detail.

Resolution rules:

- Revalidate actor, client, conversation handle, tool side, target visibility, sensitivity, lifecycle state, and field path before feedback writes.
- If the handle is expired, revoked, unknown, stale, hidden by sensitivity, or no longer maps to a visible target, return the typed `unavailable` payload in §2.7. Do not reveal whether the underlying claim/action/entity/source exists.
- A mutation cursor is not a feedback target handle. Cursor continuity may prove mutation ordering, but feedback still requires a target handle minted by a read response.
- `rawClaimIdsIncluded = false` remains true for MCP projections. W5 does not relax receipt privacy to make feedback easier.
- `rawEntityIdsIncluded = false`, `rawActionIdsIncluded = false`, and `rawSourceIdsIncluded = false` are W5 projection invariants for new/changed W5 tools. Existing implemented tools with raw `entity_id`, `resolvedEntityId`, or raw action/source ids must be hidden, renamed to handle semantics, or explicitly left outside the W5 proof until handleized.
- L1 must invoke the data-migrations reviewer and reserve/reconcile a migration slot before implementation.

Required tests:

- MCP read returns `feedback_target_handle` for feedback-eligible MCP-visible items and the relevant non-feedback handles for action/entity/source targets, but no raw internal ids.
- `dailyos.submit.claim_feedback` accepts a valid `feedback_target_handle` and writes through `record_claim_feedback`.
- `dailyos.submit.action_status` accepts a valid `action_handle` and rejects raw action ids.
- guessed raw ids, malformed handles, cross-client handles, expired/revoked handles, stale-policy handles, and sensitivity-hidden handles all return the §2.7 unavailable payload.
- losing W2 conversation/session continuity returns the §2.7 refresh-required payload, not a best-effort raw-id fallback.

### Section 2.3 - Read Path

Every W5 read handler must be an adapter over an ability or service-owned read model:

- Invoke abilities with request-scoped `Actor::McpClient`, `BridgeSurface::McpTool`, and `ClaimDismissalSurface::McpTool`.
- Use existing runtime projection for claim-shaped envelopes where possible.
- Include top-level MCP metadata: schema version, tool name, status, invocation/provenance handle when available, truncation state, and section states.
- Return host-visible resolver variants through the §2.7 payload status shapes for not-found, ambiguous, unavailable, hidden, refresh-required, and render-policy-blocked cases.
- Project provenance as display-safe source labels, source types, source-as-of values, trust bands, handles, and redaction flags. Do not return raw claim, source, action, entity, subject, meeting, workspace, or internal field ids by default.
- Keep dynamic text behind renderable evidence wrappers or allowlisted static text. No free-form source text is returned without policy rendering.

If an expected producer is absent, the handler should return a typed unavailable result or the catalog should omit the tool. Do not implement a direct SQL shortcut because the tool name exists.

### Section 2.4 - Pagination and Resources

DOS-172 pagination is a contract across W5 read handlers, not a standalone fake tool:

- Filter by sensitivity and lifecycle before applying page size.
- Use opaque cursor values. Existing `Paginated<T>`, `Cursor`, and `list_pagination` helpers are preferred.
- Include cursor invalidation behavior when filters, request shape, or substrate watermark changes.
- Bound page size, serialized payload bytes, and per-section item counts.
- Include truncation metadata so the host model can ask for the next page rather than hallucinating completeness.

DOS-173 MCP resources are out of W5 implementation scope:

- The current MCP v2 transport advertises tools only; W5 does not add resource list/read hooks.
- `tool_resources.rs` remains hidden/not registered; no placeholder resource handler satisfies W5. Direct invocation of a resource-like tool name uses the existing gateway unknown/exposure error path and returns no resource URI or payload schema.
- Source provenance detail ships, if at all, through `dailyos.read.workspace_source_provenance` as a tool with `source_provenance_handle`, not an MCP resource URI.
- Reintroducing DOS-173 requires a later L0 packet with concrete resource URI names, list/read request shapes, response schemas, pagination, sensitivity policy, unavailable/error status, and transport hooks.

### Section 2.5 - Host Selection, Evals, and E2E

DOS-481 and DOS-482 turn tool descriptions into product-surface tests:

- Every shipped tool has positive selection fixtures and two negative classes: broad-corpus/external tool should win, and adjacent DailyOS-but-wrong-tool should win.
- Hidden/cut tools cannot appear as expected adjacent winners. When `workspace_memory`, `place_document`, portfolio attention, daily/meeting briefing, or resources are removed from W5 `tools/list`, surviving fixtures must point to a shipped DailyOS tool or an explicit external/no-DailyOS outcome.
- Fixtures use generic entities only.
- Evals assert selection and output contract, not prose style.
- E2E uses the local stdio MCP v2 path against a synthetic seeded database or fixture services: list tools, call representative reads, submit typed feedback, submit/create action, update action status, verify mutation cursor, and verify forbidden sensitivity does not cross MCP.
- The E2E proof compares app/ability output and MCP output for the MCP-eligible set. Parity is modulo the sensitivity gate.

### Section 2.5.1 - Submit Note and Action Schemas

`dailyos.submit.note` request shape:

- `text`: required bounded string, sanitized as hostile input.
- `entity_handle`: required server-issued handle from a prior MCP read. Raw `entity_id`, account id, person id, project id, meeting id, or workspace id are rejected.
- `subject_text`: optional bounded natural-language display/context hint. It does not select a DB entity by itself and cannot substitute for `entity_handle`.
- `source_provenance_handle` or `workspace_source_handle`: optional server-issued source handle from a prior MCP read. Raw file paths and source ids are rejected.

`dailyos.submit.note` success response:

- `status = "ok"`;
- mandatory `mutation_cursor`;
- optional `note_handle` if a follow-up read needs to reference the note;
- optional `feedback_target_handle` only if the service creates an MCP-visible claim/receipt target;
- no `note_id`, raw claim id, raw source id, raw entity id, or raw file path.

Unscoped/global notes are out of W5. Missing `entity_handle` fails catalog validation as `BadParams` rather than creating an unowned note or guessing from `subject_text`.

`dailyos.submit.action` request shape:

- `description`: required bounded string, sanitized as hostile input.
- `due_date`: optional `YYYY-MM-DD` date string matching `services::actions::CreateActionRequest`. W5 rejects time-bearing timestamps instead of truncating them silently.
- `priority`: optional catalog-approved priority enum, mapped through `services::actions`.
- `entity_handle`: optional server-issued entity/subject handle from a prior MCP read. Raw entity/account/person/project/meeting ids are rejected.
- `source_provenance_handle` or `workspace_source_handle`: optional server-issued source context handle. Raw file paths and source ids are rejected.

`dailyos.submit.action` success response:

- `status = "ok"`;
- mandatory `mutation_cursor`;
- `action_handle` for follow-up `dailyos.submit.action_status`;
- `semantic_status` and `stored_status`;
- no `action_id`, raw entity id, raw source id, or raw file path.

The W5 catalog YAML, examples, and selection fixtures must be updated to these shapes before the tools are advertised.

### Section 2.6 - Action Status Public Semantics

`dailyos.submit.action_status` is a semantic MCP API over existing action services, not a direct projection of stored `actions.status`.

Request shape:

- `action_handle`: server-issued handle from an MCP read or action creation response. Raw action ids are rejected unless L0 later approves an explicit render-policy contract.
- `status`: one of the exposed semantic values below.
- `defer_date`: required for `deferred` as a `YYYY-MM-DD` date string; forbidden for `done` and `dropped`.
- `reason`: optional bounded text, sanitized as hostile input and stored only through approved service metadata.

Approved semantics:

| MCP status | Service behavior | Stored status result | Notes |
| --- | --- | --- | --- |
| `done` | Calls `services::actions::complete_action` for open actions; returns `already_current` for already-completed actions. | `completed` | Must sync action claims and emit existing completion signals. Repeated `done` calls are idempotent and include `mutation_cursor`. |
| `deferred` | Requires `defer_date` and calls a service wrapper around `update_action` to move `due_date` while preserving the current open stored status. | `unstarted` or `started`, unchanged except due/context fields | `deferred` is not a persisted status. Response must expose `semantic_status = "deferred"` and `stored_status`, so API users do not believe a database status was created. Time-of-day reminders are out of W5 scope. |
| `dropped` | Calls a service-owned terminal wrapper. Backlog/suggested rows use `dismiss_suggested_action` and preserve the existing suggestion-dismissal semantics. Accepted/open rows use a new/approved service wrapper that sets `cancelled`, syncs action claims, emits signals, and records MCP source/reason. | `archived` for backlog/suggested rows; `cancelled` for accepted/open rows | Response always exposes `semantic_status = "dropped"` and `stored_status`. No raw `db.archive_action` from the handler. |

If L1 cannot implement one row through service semantics, the catalog must hide or rename that value before `tools/list` exposes it. Tests must cover request/response schema, stored status result, idempotency for repeated calls, unavailable hidden handles, action-claim sync, emitted signals, and no direct DB writes from MCP handlers.

### Section 2.7 - Public Result Status Shapes

Gateway, manifest, unknown-tool, exposure, and conversation/session failures keep using existing `McpToolResult::Error` / `ToolError` variants such as `BadParams`, `Unauthorized`, `PairingRevoked`, `ConversationRevoked`, `ExposureForbidden`, `RateLimited`, `UpstreamFailure`, and `Internal`.

Domain target-resolution failures inside an advertised handler are successful tool payloads with a stable `status` field. They do not use `ToolError::NotFound`, because hidden and unknown targets must be indistinguishable. For `Side::Write` and `Side::SubmitCorrection`, every successful payload status below includes a `mutation_cursor`; for rejected target statuses this is a no-op continuity cursor and does not imply a mutation was applied.

| Status | When returned | Required fields | Forbidden fields |
| --- | --- | --- | --- |
| `ok` | Tool completed and returned data or applied mutation. | `status`, `schema_version`, tool-specific payload. Submit/write tools also include mandatory `mutation_cursor`. | Raw internal ids unless a future render policy explicitly allows them. |
| `unavailable` | Handle/target is unknown, hidden, expired, revoked, stale, sensitivity-blocked, lifecycle-blocked, cross-client, or not visible under current render policy. | `status`, `reason = "target_unavailable"`, `refresh_required` boolean, optional `retry_after` for transient stale state, and `mutation_cursor` for submit/write tools. | Claim id, source id, action id, entity id, meeting id, existence hint, sensitivity tier, source label, subject label, or different reason per hidden/unknown case. |
| `refresh_required` | Read-side target handles cannot be validated against current W2 authority but the gateway session itself is valid. Submit/write conversation failures use gateway errors, not this payload. | `status`, `reason = "conversation_refresh_required"`, optional `conversation_handle` from the response envelope if the gateway minted one. | Any best-effort fallback target, raw id request, or mutation attempt. |
| `already_current` | Idempotent repeat of an already-applied semantic status such as `done`. | `status`, `mutation_cursor`, stored/semantic status when relevant. | Raw internal ids. |

Tests must assert byte-shape equality for hidden vs unknown vs sensitivity-blocked target failures and must verify `mutation_cursor` is present on every successful submit/write response.

### Section 2.8 - Audit Privacy

W5 must bring `src-tauri/src/services/mcp_v2/gateway.rs` and `src-tauri/src/services/mcp_v2/audit.rs` back into the ADR-0102 audit contract:

- Audit detail stores actor, tool name, request id, side, bounded mutation cursor, result status, and keyed/HMAC hashes of params and response payloads.
- Audit detail does not persist raw params, raw responses, raw handles, raw source labels, raw entity names, raw claim/action/source/entity ids, prompt text, note/action bodies, or hidden target details.
- Read-side and write-side audit rows use the same privacy posture. Read rows do not get a plaintext exception.
- Target-handle resolution failures log only PII-safe reason codes such as `target_unavailable`, `conversation_refresh_required`, or `gateway_rejected`.
- Tests must seed a sensitive read and a rejected submit/write and assert the audit detail contains hashes/status/cursor only, not the original params, response, source labels, handles, or user text.

---

## Section 3 - Acceptance Criteria

**AC1 - Catalog and handlers converge.** Every tool advertised by MCP v2 has a registered handler, catalog entry, schema, selection fixtures, and tests. Placeholder handlers are removed or the corresponding catalog entries are hidden until implemented. `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, and resource entries are absent from `tools/list` and have no hidden compatibility handlers.

**AC2 - v2 gateway path only.** W5 tools route through `services::mcp_v2::{gateway,transport,contracts,handlers}` and request-scoped `Actor::McpClient`. They do not route through legacy static MCP v1 handlers except as explicitly approved compatibility shims with tests proving identical policy.

**AC3 - Explicit claim feedback tool.** W5 implements `dailyos.submit.claim_feedback` as the typed ADR-0123 feedback carrier after landing the narrow ADR-0128 clarification in §1.4. It supports ADR-0123 actions, request field `feedback_target_handle`, server-issued `McpFeedbackTargetHandle` binding from prior MCP reads, action metadata validation, sanitizer warnings, server-minted idempotency, mandatory `mutation_cursor` on every successful submit response, and optional follow-up `feedback_target_handle`. Guessed raw claim ids are rejected without revealing existence. Generic notes are not inferred into claim feedback.

**AC4 - MCP actor accepted narrowly.** `submit_claim_feedback` and `record_claim_feedback` accept MCP-origin feedback as an explicit MCP actor class or namespace. Tests prove MCP feedback is accepted, Agent/System feedback remains rejected where appropriate, MCP is not stored as app-user feedback, and no hardcoded user `ServiceContext` path is used for MCP feedback writes.

**AC5 - Feedback propagation reused.** MCP feedback writes an append-only `claim_feedback` row through `record_claim_feedback`, emits the same feedback/invalidation/repair signals as app feedback, and re-renders or returns the same receipt state for MCP-eligible claims.

**AC6 - Server-derived MCP provenance and target handles.** Feedback origin metadata is derived by the gateway/handler, not caller params. Caller-supplied actor, actor_id, surface, sensitivity, idempotency key, granted scopes, raw conversation id, tool side, raw claim id, raw action id, raw source id, raw entity/subject id, raw meeting id, raw workspace id, or raw internal field id are rejected unless the value is a server-issued opaque handle from an earlier MCP response. Target handles bind client, conversation, originating tool, item path, target kind, render-policy version, sensitivity, provenance hash, and target version/watermark; resolution revalidates visibility before internal target construction. Existing raw-id handlers, including `dailyos.write.place_document`, are removed from the advertised W5 surface.

**AC7 - Sensitivity egress.** `Confidential` and `UserOnly` claims, source details, receipts, provenance detail, entity/meeting/action/source handles, and derived chain content never cross MCP except through approved opaque handles that do not reveal the underlying identifier. `dailyos.read.account_status` is upgraded to the §2.1 v2 handleized response contract before it remains advertised. Tests cover direct claims, composed/derived claims, pagination, provenance detail tools, handle resolution, account-status resolver responses, and feedback/action attempts against hidden/unknown targets.

**AC8 - Daily briefing hidden in W5.** `dailyos.read.daily_briefing` is not exposed in W5, is absent from `tools/list`, and has no hidden compatibility handler. W5 does not change `get_daily_briefing` `allowed_actors` or `mcp_exposure`; no parallel daily-briefing SQL/prose path ships. Future exposure requires the §2.1.1 security-approved envelope.

**AC9 - Meeting briefing hidden in W5.** `dailyos.read.meeting_briefing` is not exposed in W5, is absent from `tools/list`, and has no hidden compatibility handler. W5 does not use `prepare_meeting` or mutating meeting refresh paths as read handlers.

**AC10 - Source-aware reads.** `workspace_source_provenance` returns source-aware results with source-as-of, trust band, sensitivity/redaction state, lifecycle/suppression caveats, display-safe source labels, and `source_provenance_handle` where follow-up detail is allowed. It uses source-management/provenance services rather than generic filesystem search. Raw file paths, raw source ids, raw entity ids, hidden claim ids, or prompt text are not returned by default. `dailyos.search.workspace_memory` is hidden in W5 because no claim-backed producer exists.

**AC11 - Pagination.** All paginated W5 reads filter sensitivity/lifecycle before page cap, use opaque cursors, bound page size and serialized bytes, surface invalidated cursors, and include truncation/next-page metadata.

**AC12 - Resources hidden in W5.** DOS-173 resource support is removed from W5 scope. The v2 server remains tool-only for this packet; resource catalog/placeholder handlers are absent from `tools/list` and no resource compatibility handler is registered. Source provenance detail is tool-based through `dailyos.read.workspace_source_provenance`.

**AC13 - Submit note service boundary.** `dailyos.submit.note` routes through approved entity-scoped claim/source services with the §2.5.1 request/response schema, bounded text, required `entity_handle`, source/provenance handles, sensitivity defaults, signals, hostile-input sanitization, and mandatory `mutation_cursor` on every successful submit response. It returns `note_handle` when a follow-up handle is needed, never `note_id`. It does not directly insert claims, infer typed feedback, or create unscoped/global notes.

**AC14 - Submit action service boundary.** `dailyos.submit.action` routes through `services::actions::create_action` or a service wrapper that preserves validation, signals, action-claim sync, and objective-link behavior. It uses the §2.5.1 request/response schema. Success returns mandatory `mutation_cursor` and an `action_handle` for follow-up status changes, not `action_id` or any raw entity/source id. No direct DB writes from MCP handlers.

**AC15 - Action status mapping.** `dailyos.submit.action_status` implements the exact §2.6 semantic mapping backed by service functions. It accepts `action_handle`, not raw action id. `done`, `deferred`, and `dropped` are implemented with durable service semantics, stored-status response fields, mandatory `mutation_cursor` on every successful submit response including `unavailable`, action-claim sync, signals, idempotency, and hidden-handle rejection; otherwise the unsupported value is hidden before the tool is exposed.

**AC16 - Host-selection evals.** Every shipped W5 tool has passing positive, broad-corpus negative, and adjacent-wrong-tool selection fixtures. Tool descriptions include `when_to_call` and `when_NOT_to_call` language matching ADR-0128 displacement framing. Catalog and golden fixtures are pruned after hidden-tool removal: no remaining shipped-tool fixture may expect `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, or resource entries as adjacent winners.

**AC17 - Local stdio E2E.** A synthetic local stdio MCP v2 test lists tools, proves daily briefing/meeting briefing/portfolio attention/workspace memory/place document/resources are absent from `tools/list` and direct invocation returns existing gateway errors with no partial data, calls representative reads including handleized account status, receives feedback target handles, submits typed claim feedback through a valid handle, rejects guessed/stale/cross-client handles with the same §2.7 unavailable payload and no-op mutation cursor for submit/write tools, submits an action, updates action status through §2.6 semantics using `action_handle`, verifies mutation cursors, and proves forbidden sensitivity is excluded.

**AC18 - App/MCP parity proof.** For MCP-eligible fixtures, a correction through MCP changes the same claim/trust/receipt state as app feedback and a follow-up read reflects it. Parity excludes `Confidential` and `UserOnly` data by design.

**AC19 - No production data or PII fixtures.** Tests, docs, commit messages, PR bodies, fixtures, screenshots, and eval outputs use generic entities such as `account_01`, `project_01`, `person_01`, and synthetic domains only when needed.

**AC20 - Gates.** Focused W5 tests plus full gates pass before implementation ships:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
```

W5 L1 cannot start gateway/session/transport/target-handle implementation until W2/DOS-833 is merged and this branch is rebased.

**AC21 - Target-handle registry migration.** W5 implements server-side `mcp_target_handles` storage per §2.2.1 using reserved registered schema version `290` unless rebase forces an explicit wave-plan and packet update. L1 implements the pinned columns, constraints, indexes, public-handle HMAC lookup, `target_watermark_hash`, encrypted target-ref shape, 24-hour expiry, 7-day cleanup retention, startup/post-invocation cleanup, and rollback behavior. L1 must prove the SQL filename and `migrations.rs` registration follow the active migration naming convention, with registered version 290 as the schema-version assertion. Tests cover expiry, revocation, last-used update, cross-client rejection, stale-policy rejection, hidden-target rejection, cleanup, and no raw public handles or target ids in registry plaintext/indexed columns, public responses, audit detail, or outbox fallback rows.

**AC22 - Audit privacy.** W5 updates `mcp_v2/gateway.rs` and `mcp_v2/audit.rs` so read and write audit rows store hashes/status/cursor only, per §2.8. Tests prove raw params, raw responses, note/action text, source labels, raw ids, and opaque handles do not persist in audit rows or outbox fallback rows.

---

## Section 4 - Intelligence Loop Integration Check

**1. Claim model.** W5 read outputs are projections over claims, source-aware read models, and service-owned runtime evidence. MCP typed corrections are `claim_feedback`, not display-only flags. Submit notes may become claim/source proposals only through approved services with explicit subject, sensitivity, lifecycle, and source attribution.

**2. Provenance + trust.** MCP reads expose source-as-of, trust band, source type, redaction state, and caveats through render policy. MCP feedback preserves `Actor::McpClient` attribution so source reliability and trust effects can distinguish host-model-origin corrections from direct app actions.

**3. Signals + invalidation.** Feedback and submit-class writes keep existing service signal paths. MCP action writes sync action claims and emit action signals. Feedback emits the same claim feedback/invalidation/repair signals as app feedback.

**4. Runtime + surfaces.** Tauri and MCP consume the same ability/service substrate for MCP-eligible data. MCP responses differ only by surface projection, truncation, handle indirection, audit hashing, and sensitivity egress gates. W5 uses MCP-approved producers such as `get_entity_intelligence`, source-management/provenance readers, and service write paths. `get_daily_briefing`, `prepare_meeting`, recommendation/salience portfolio producers, generic workspace memory search, and MCP resource list/read are explicitly not W5 runtime consumers.

**5. Feedback loop.** MCP corrections feed source reliability, claim lifecycle/trust, repair jobs, receipt state, and downstream surfacing through the same feedback loop as app corrections. Submit notes/actions do not silently alter claim truth unless promoted through services-owned claim/provenance paths.

---

## Section 5 - Implementation Surface

Likely files/modules:

- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` for the narrow `dailyos.submit.claim_feedback` clarification in §1.4.
- `src-tauri/src/services/mcp_v2/handlers/registration.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs` only to remove/avoid registration for daily/meeting briefing; it must not expose a briefing producer in W5
- `src-tauri/src/services/mcp_v2/handlers/tool_workspace_search.rs` only to remove/avoid registration for `dailyos.search.workspace_memory`; it must not implement generic search in W5
- `src-tauri/src/services/mcp_v2/handlers/tool_workspace_source_provenance.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_portfolio.rs` only to remove/avoid registration for `dailyos.read.portfolio_attention`; it must not expose recommendation/salience producers in W5
- `src-tauri/src/services/mcp_v2/handlers/tool_note.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_create_action.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_update_action_status.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_resources.rs` only to keep resource entries hidden/not registered; it must not implement DOS-173 resource list/read in W5
- `src-tauri/src/services/mcp_v2/handlers/tool_pagination.rs` if kept as a helper/test module
- `src-tauri/src/services/mcp_v2/target_handles.rs` or equivalent registry-backed resolver
- `src-tauri/src/services/mcp_v2/gateway.rs`
- `src-tauri/src/services/mcp_v2/audit.rs`
- `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml`
- `src-tauri/src/services/mcp_v2/runtime_projection.rs`
- `src-tauri/src/services/claim_receipt/feedback.rs`
- `src-tauri/src/services/claims.rs`
- `src-tauri/src/services/actions.rs`
- `src-tauri/src/services/source_management_ledger.rs`, `src-tauri/src/services/markdown_preview.rs`, and workspace-ingestion provenance helpers as needed for source-provenance reads
- `src-tauri/src/migrations.rs` and a new migration for `mcp_target_handles`
- MCP v2 integration tests under `src-tauri/tests/` or the local test convention used by existing MCP v2 tests

Do not modify the dirty main checkout. Do not add migrations except the required target-handle registry migration after slot reconciliation. Do not edit `get_daily_briefing` or meeting-prep ability policy in W5.

---

## Section 6 - Review Dispatch

Formal Wave L0 lanes:

- `/codex challenge` - adversarial review against the full packet and current code.
- `ce-security-lens-reviewer` - MCP egress, actor attribution, hostile input, provenance detail, handle binding, and write boundaries.
- `ce-feasibility-reviewer` - producer availability for the remaining W5 tools, action status mapping, handle implementation, and test feasibility.
- `ce-learnings-researcher` - mandatory K-in over `docs/solutions/` and `.docs/decisions/`.

Specialist subchecks folded into the formal planning verdicts:

- `ce-api-contract-reviewer` - MCP tool names, request/response schemas, pagination, mutation cursors, hidden-tool cleanup, and catalog/handler consistency.
- `ce-product-lens-reviewer` - tool descriptions, host-selection/displacement fixtures, and whether the exposed MCP surface matches ADR-0128's product framing.
- `ce-data-migrations-reviewer` - target-handle registry schema, migration slot, indexes, retention, cleanup, and rollback proof.
- `/cso` or project-approved DailyOS security equivalent only if daily briefing, meeting briefing, or any other composed briefing surface is reintroduced.

Cycle reviewer prompts must include these known pressure points:

1. Does the §1.4 ADR-0128 clarification authorize `dailyos.submit.claim_feedback` narrowly enough without broadening MCP writes?
2. Does explicit MCP actor acceptance preserve feedback trust semantics without making Agent/System feedback too broad?
3. Is the opaque handle contract sufficient to bridge MCP projection privacy to internal feedback, action, entity, source, and workspace targets without raw id exposure or replay leaks?
4. Which W5 catalog tools have real producers today, and which must be hidden or amended before implementation?
5. Is the `done/deferred/dropped` action-status vocabulary implementable through service semantics exactly as §2.6 states?
6. Are daily briefing, meeting briefing, portfolio attention, workspace memory search, place document, and DOS-173 resources fully absent from `tools/list`, with no hidden compatibility handler?
7. Are sensitivity gates applied before pagination, provenance detail, handle minting, audit detail, and projection rather than only at final projection?
8. Are mutation cursors mandatory on every successful submit/write response and clearly separate from target handles?
9. Is the `mcp_target_handles` registry migration sufficient for expiry, revocation, last-used, stale-policy, cross-client rejection, and retention?

---

## Section 7 - K-In Evidence

Substrate-type searches performed before authoring this packet covered:

- MCP gateway/catalog/handler primitives: `McpToolHandler`, `ToolDescription`, `McpActor`, `Actor::McpClient`, `runtime_projection`, `registration`.
- Target/session primitives: `mcp_conversation_handle`, `OpaqueConversationHandle`, `mutation_cursor`, `ReceiptTarget`, `rawClaimIdsIncluded`, and existing handle/resource patterns.
- Feedback primitives: `record_claim_feedback`, `submit_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`, `validate_feedback_actor`, `actor_class_for_actor`.
- Read/provenance primitives: `get_entity_intelligence`, `get_daily_briefing`, `list_open_loops`, `Paginated`, `Cursor`, `source_asof`, `EnvelopeProvenance`, `RenderSurface::McpTool`, `source_management_ledger`, `markdown_preview`, and workspace-ingestion source handles.
- Write primitives: `create_action`, `complete_action`, `reopen_action`, `reject_suggested_action`, `dismiss_suggested_action`, `ActionStatus`.
- Prior decisions/solutions: ADR-0128, ADR-0125, ADR-0126, ADR-0093, `prompt-channel-sensitivity-class-sweep-2026-05-18.md`, `k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`, producer remediation notes, and the v1.4.9 wave plan.

Important findings:

- W5 is not greenfield. The v2 gateway, local stdio dispatch, taxonomy catalog, account-status handler, runtime projection, claim feedback writer, receipt validation, action services, source/provenance readers, and pagination helpers already exist.
- The feedback actor blocker is current code, not stale plan text. L1 must extend or deliberately preserve it; bypass is not acceptable.
- The catalog/handler mismatch is current code. W5 must close it or hide unready entries, including `dailyos.search.workspace_memory` and `dailyos.read.portfolio_attention`.
- The daily briefing producer exists but is not MCP-exposed and has raw-id/sensitivity envelope gaps for MCP. W5 keeps it hidden; future exposure needs the §2.1.1 security gate instead of a parallel reimplementation.
- Source/provenance and sensitivity gates are documented class-pattern risks. Tests must cover the whole boundary class, not one happy path.

---

## Section 8 - L0 Verdict Record

2026-06-04 final cycle verdict: **APPROVE**.

Passing lanes:

- `/codex challenge` - APPROVE.
- `ce-security-lens-reviewer` - APPROVE.
- `ce-feasibility-reviewer` - APPROVE.
- `ce-learnings-researcher` / K-in - APPROVE.
- `ce-api-contract-reviewer` subcheck - APPROVE.
- `ce-product-lens-reviewer` subcheck - APPROVE.
- `ce-data-migrations-reviewer` subcheck - APPROVE.

Resolved final-cycle blockers:

- Wave-plan migration reservations now reserve W5 as registered schema version 290 and shift W6 to v291-v296.
- W5 packet and wave plan now reserve a registered schema version rather than hardcoding a migration filename; L1 must reconcile the active `migrations.rs` filename convention during data-migration review.
- Wave-plan execution sequencing now makes W2 parallel to W3/W4 but a hard W5 MCP L1 entry gate.
- Wave-plan overview and detail now agree that daily briefing, meeting briefing, portfolio attention, workspace-memory search, `dailyos.write.place_document`, and DOS-173 resources are out of W5 until later approved packets.

Residual gate: W5 L1 cannot begin gateway/session/transport/conversation-handle/target-handle implementation until W2/DOS-833 lands and W5 rebases. If W2 lands a different auth/gateway authority than this packet assumes, W5 returns to L0.
