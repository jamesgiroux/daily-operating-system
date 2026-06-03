# DOS-777 / W5 L0 Packet - MCP-First Parity, Feedback Writes, and Source-Aware Reads

- **Version:** v1.4.9 - W5 MCP-first parity
- **Primary issue:** [DOS-777](https://linear.app/a8c/issue/DOS-777)
- **Related issues:** DOS-171, DOS-479, DOS-172, DOS-173, DOS-481, DOS-482, DOS-169, DOS-170, DOS-8
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Scope tier:** Wave-scope MCP, claim/provenance, read/write substrate. L0 requires `/codex challenge` or a project-approved equivalent, `ce-api-contract-reviewer`, `ce-security-lens-reviewer`, `ce-feasibility-reviewer`, and mandatory K-in. Add `ce-product-lens-reviewer` for tool-description / displacement-eval changes; add `ce-data-migrations-reviewer` only if the chosen feedback-origin storage requires a migration.
- **Status:** Draft for L0 review; not approved until adversarial, API/contract, security-lens, feasibility, product if invoked, and K-in verdicts are recorded.

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
- Filter sensitivity before pagination, truncation, projection, audit detail, provenance detail, resource reads, and host-model output.
- Treat external/workspace/customer content as evidence only. MCP tool params and source text cannot choose actor, sensitivity, source authority, or tool side.
- Do not advertise catalog-only tools as usable. A listed tool must have a registered handler, tool description, selection fixtures, and tests.

### Section 0.1 - Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w5-mcp-parity-l0` from `public/dev`. Current code has a v2 MCP gateway, taxonomy catalog, local stdio path, and two registered handlers (`dailyos.read.account_status`, `dailyos.write.place_document`). The W5 catalog already names the broader surface, but most W5 handler modules are placeholders. L1 must rebase onto the latest W1-W4 authority before implementation and reconcile any changed tool names or ADR amendments.

L1 must also rebase against the W2/DOS-833 auth right-size authority before touching gateway/session code. W5 preserves `Actor::McpClient`, MCP egress sensitivity gates, audit attribution, and hostile-input handling; it must not reintroduce HMAC/pairing/scope-grant ceremony that W2 removes for local stdio.

This packet chooses the open W5 actor decision: extend the feedback writer and receipt path to accept explicit MCP-client feedback attribution. Do not encode MCP corrections as app-user feedback with only side metadata.

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
- `get_daily_briefing` exists and composes daily readiness, meeting prep status, and entity intelligence. It currently has `allowed_actors = [User]` and `mcp_exposure = None`; W5 must deliberately expose or wrap it for MCP instead of building a parallel briefing query.
- `get_daily_readiness` is already MCP-invocable, but it is not a replacement for the W5 daily briefing envelope.
- `prepare_meeting` exists as a transform ability and is not itself the W5 meeting-read handler. The meeting briefing mapping remains ambiguous unless L1 identifies an approved read producer or service-backed read shape.
- Recommendation/salience read abilities and action/open-loop evidence exist, but portfolio attention is not yet a ready MCP handler on this base.

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

1. **Typed claim feedback:** add an explicit MCP tool for ADR-0123 feedback, proposed name `dailyos.submit.claim_feedback`, or prove that an existing approved tool is the typed feedback carrier. Generic note text must not be inferred into claim feedback.
2. **Submit-class writes:** reconcile DOS-169 and DOS-170 against Section D. `dailyos.submit.action` and `dailyos.submit.action_status` route through `services::actions`; `dailyos.submit.note` routes through approved claim/source services.

If reviewers decide `dailyos.submit.claim_feedback` needs a clarifying ADR-0128 amendment because Section D's concrete list omitted it, L1 must author that amendment before implementation. The amendment should clarify naming only; it must not broaden MCP beyond feedback and lightweight submit-class writes.

### Section 1.5 - Action Services

`src-tauri/src/services/actions.rs` already validates and mutates actions through service-owned paths:

- `create_action` validates bounded text, priority, due date, entity ids, source label, inserts through `ActionDb`, syncs action claims, emits action signals, scans decisions, and best-effort links objectives.
- `complete_action` and `reopen_action` update status through service paths, sync claims, and emit signals.
- `reject_suggested_action` and `dismiss_suggested_action` archive/suppress suggested actions with distinct trust/signal semantics.
- `update_action` validates field updates and applies them through a service boundary.

The catalog currently advertises `action_status` values `done`, `deferred`, and `dropped`, while backend status constants are `completed`, `unstarted`, `started`, `cancelled`, and `archived`. W5 must define a service-backed mapping before exposing the tool. At minimum:

- `done` maps to `complete_action`.
- `dropped` must map to an approved service-level terminal transition, not raw SQL. If existing services only support suggestion rejection/dismissal, add a service wrapper or narrow the tool contract.
- `deferred` must map to an explicit service behavior such as due-date/context update or an approved status vocabulary change. If no durable deferred state exists, remove or rename the catalog option before listing it.

### Section 1.6 - Sensitivity, Provenance, and Prompt-Injection Guardrails

Prior K-in entries matter here:

- `prompt-channel-sensitivity-class-sweep-2026-05-18.md` centralized prompt-channel sensitivity gates after repeated leaks. W5 must use the existing gate, not copy sensitivity matches into handlers.
- `k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` records the failure mode of grepping only for a proposed primitive name. W5 K-in must search for substrate types: `record_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`, `McpToolHandler`, `ToolDescription`, `Actor::McpClient`, `Paginated`, `Cursor`, `source_asof`, `render_policy`, `services::actions`, and workspace source/provenance readers.
- ADR-0093 remains relevant for workspace memory and note/action text. Source text and tool text are evidence, not instructions.

MCP read handlers must render through approved policy and strip raw identifiers that the current MCP projection intentionally omits. MCP feedback handlers must sanitize free text through the receipt path.

---

## Section 2 - Chosen Architecture

### Section 2.1 - Tool Surface

W5 ships MCP v2 parity through registered, catalog-backed handlers:

1. `dailyos.read.account_status` - already implemented; keep as the reference pattern.
2. `dailyos.read.daily_briefing` - expose/wrap `get_daily_briefing` for MCP with `Actor::McpClient`, MCP render surface, bounded output, and existing cursor fields.
3. `dailyos.read.meeting_briefing` - only ship if L1 identifies a service/ability-backed read producer. Otherwise amend the catalog and L0 scope before implementation rather than shipping a placeholder.
4. `dailyos.read.portfolio_attention` - route through recommendation/salience or entity-intelligence substrate, with trust/caveat output and no account-only assumptions.
5. `dailyos.search.workspace_memory` - read from workspace/source/claim substrate with source-aware, trust-bearing results. It is not generic filesystem grep.
6. `dailyos.read.workspace_source_provenance` - return display-safe source provenance for a previously returned source/provenance reference; no raw source ids, paths, or forbidden sensitivity tiers.
7. `dailyos.submit.claim_feedback` - new explicit typed feedback tool unless L0 reviewers approve an existing carrier.
8. `dailyos.submit.note` - submit a bounded note/observation through claim/source services; do not infer typed feedback.
9. `dailyos.submit.action` - submit a bounded action through `services::actions::create_action`.
10. `dailyos.submit.action_status` - map the approved status vocabulary to service-level transitions.

`dailyos.write.place_document` is already implemented and remains outside the W5 feedback/read parity proof unless L1 uses it in E2E fixtures.

### Section 2.2 - Feedback Path

The MCP typed feedback flow is:

1. Handler receives a `McpActor::Client` from the v2 gateway.
2. Handler validates params against the catalog schema and rejects caller-supplied actor, surface, sensitivity, source authority, idempotency key, granted scopes, or conversation internals.
3. Handler resolves a server-issued receipt/provenance target from a prior MCP read or receipt handle and rejects guessed/out-of-scope raw claim ids with the same unavailable shape used for hidden/unknown targets.
4. The server derives MCP origin metadata from the envelope: client id, conversation handle, tool name, and `ClaimDismissalSurface::McpTool`.
5. Handler builds a receipt-shaped `ClaimFeedbackRequest` with ADR-0123 action metadata.
6. `claim_receipt::feedback::submit_claim_feedback` admits a new MCP render actor path and keeps envelope target binding, sanitizer, idempotency, and surface checks.
7. `record_claim_feedback` accepts the explicit MCP actor class for feedback only and writes append-only `claim_feedback` with MCP attribution. The current hardcoded `ServiceContext::with_actor("user")` feedback bridge must be replaced or wrapped so writer/audit attribution remains MCP-specific.
8. Existing feedback signals, invalidation, repair, and receipt re-render behavior run unchanged.
9. The handler returns a bounded response with an opaque mutation cursor or receipt handle, lifecycle/repair flags, sanitized warnings, and no raw claim/source ids unless an explicit render-policy contract allows them.

This preserves actor truth: the correction came from a local MCP client acting for the user through a host model, not directly from the app user.

### Section 2.3 - Read Path

Every W5 read handler must be an adapter over an ability or service-owned read model:

- Invoke abilities with request-scoped `Actor::McpClient`, `BridgeSurface::McpTool`, and `ClaimDismissalSurface::McpTool`.
- Use existing runtime projection for claim-shaped envelopes where possible.
- Include top-level MCP metadata: schema version, tool name, status, invocation/provenance handle when available, truncation state, and section states.
- Return host-visible resolver variants for not-found, ambiguous, unavailable, hidden, and render-policy-blocked cases.
- Project provenance as display-safe source labels, source types, source-as-of values, trust bands, and redaction flags. Do not return raw claim ids or raw source ids by default.
- Keep dynamic text behind renderable evidence wrappers or allowlisted static text. No free-form source text is returned without policy rendering.

If an expected producer is absent, the handler should return a typed unavailable result or the catalog should omit the tool. Do not implement a direct SQL shortcut because the tool name exists.

### Section 2.4 - Pagination and Resources

DOS-172 pagination is a contract across W5 read handlers, not a standalone fake tool:

- Filter by sensitivity and lifecycle before applying page size.
- Use opaque cursor values. Existing `Paginated<T>`, `Cursor`, and `list_pagination` helpers are preferred.
- Include cursor invalidation behavior when filters, request shape, or substrate watermark changes.
- Bound page size, serialized payload bytes, and per-section item counts.
- Include truncation metadata so the host model can ask for the next page rather than hallucinating completeness.

DOS-173 resources must be real if W5 claims it:

- If the current MCP v2 transport supports resources, implement list/read resource handlers for approved resource types such as tool catalog help, source provenance detail, or receipt/provenance detail.
- If the transport lacks resource hooks, L1 must either add them or return to L0 with a scope amendment. A placeholder `tool_resources.rs` does not satisfy DOS-173.
- Resource reads apply the same MCP sensitivity and provenance rules as tools.

### Section 2.5 - Host Selection, Evals, and E2E

DOS-481 and DOS-482 turn tool descriptions into product-surface tests:

- Every shipped tool has positive selection fixtures and two negative classes: broad-corpus/external tool should win, and adjacent DailyOS-but-wrong-tool should win.
- Fixtures use generic entities only.
- Evals assert selection and output contract, not prose style.
- E2E uses the local stdio MCP v2 path against a synthetic seeded database or fixture services: list tools, call representative reads, submit typed feedback, submit/create action, update action status, verify mutation cursor, and verify forbidden sensitivity does not cross MCP.
- The E2E proof compares app/ability output and MCP output for the MCP-eligible set. Parity is modulo the sensitivity gate.

---

## Section 3 - Acceptance Criteria

**AC1 - Catalog and handlers converge.** Every tool advertised by MCP v2 has a registered handler, catalog entry, schema, selection fixtures, and tests. Placeholder handlers are removed or the corresponding catalog entries are hidden until implemented.

**AC2 - v2 gateway path only.** W5 tools route through `services::mcp_v2::{gateway,transport,contracts,handlers}` and request-scoped `Actor::McpClient`. They do not route through legacy static MCP v1 handlers except as explicitly approved compatibility shims with tests proving identical policy.

**AC3 - Explicit claim feedback tool.** W5 implements `dailyos.submit.claim_feedback` or an L0-approved equivalent typed feedback carrier. It supports ADR-0123 actions, server-issued receipt/provenance target binding, action metadata validation, sanitizer warnings, server-minted idempotency, and mutation cursor. Guessed raw claim ids are rejected without revealing existence. Generic notes are not inferred into claim feedback.

**AC4 - MCP actor accepted narrowly.** `submit_claim_feedback` and `record_claim_feedback` accept MCP-origin feedback as an explicit MCP actor class or namespace. Tests prove MCP feedback is accepted, Agent/System feedback remains rejected where appropriate, MCP is not stored as app-user feedback, and no hardcoded user `ServiceContext` path is used for MCP feedback writes.

**AC5 - Feedback propagation reused.** MCP feedback writes an append-only `claim_feedback` row through `record_claim_feedback`, emits the same feedback/invalidation/repair signals as app feedback, and re-renders or returns the same receipt state for MCP-eligible claims.

**AC6 - Server-derived MCP provenance.** Feedback origin metadata is derived by the gateway/handler, not caller params. Caller-supplied actor, actor_id, surface, sensitivity, idempotency key, granted scopes, raw conversation id, tool side, raw claim id, or raw source id are rejected unless the value is a server-issued opaque handle from an earlier MCP response.

**AC7 - Sensitivity egress.** `Confidential` and `UserOnly` claims, source details, receipts, provenance resources, and derived chain content never cross MCP. Tests cover direct claims, composed/derived claims, pagination, provenance detail, resource reads, and feedback attempts against hidden/unknown targets.

**AC8 - Daily briefing re-homed.** `dailyos.read.daily_briefing` wraps or exposes `get_daily_briefing` with `Actor::McpClient`, `mcp_exposure = Invocable`, MCP render policy, bounded output, and cursor support. No parallel daily-briefing SQL/prose path ships.

**AC9 - Meeting briefing decision.** `dailyos.read.meeting_briefing` either maps to a named service/ability-backed read producer with tests, or the catalog omits it and the L0 packet is amended. A placeholder or generic `prepare_meeting` transform call does not satisfy this AC.

**AC10 - Source-aware reads.** `workspace_memory` and `workspace_source_provenance` return claim/source-aware results with source-as-of, trust band, sensitivity/redaction state, lifecycle/suppression caveats, and display-safe source labels. Raw file paths, raw source ids, hidden claim ids, or prompt text are not returned by default.

**AC11 - Pagination.** All paginated W5 reads filter sensitivity/lifecycle before page cap, use opaque cursors, bound page size and serialized bytes, surface invalidated cursors, and include truncation/next-page metadata.

**AC12 - Resources.** DOS-173 resource support is implemented through real MCP resource list/read behavior or removed from W5 scope through L0 amendment. Resource payloads use the same render policy as tools.

**AC13 - Submit note service boundary.** `dailyos.submit.note` routes through approved claim/source services with bounded text, explicit subject handling, source/provenance fields, sensitivity defaults, signals, and hostile-input sanitization. It does not directly insert claims or infer typed feedback.

**AC14 - Submit action service boundary.** `dailyos.submit.action` routes through `services::actions::create_action` or a service wrapper that preserves validation, signals, action-claim sync, and objective-link behavior. No direct DB writes from MCP handlers.

**AC15 - Action status mapping.** `dailyos.submit.action_status` has an explicit status mapping backed by service functions. `done`, `deferred`, and `dropped` are either implemented with durable service semantics or the catalog vocabulary is changed before the tool is exposed.

**AC16 - Host-selection evals.** Every shipped W5 tool has passing positive, broad-corpus negative, and adjacent-wrong-tool selection fixtures. Tool descriptions include `when_to_call` and `when_NOT_to_call` language matching ADR-0128 displacement framing.

**AC17 - Local stdio E2E.** A synthetic local stdio MCP v2 test lists tools, calls representative reads, submits typed claim feedback, submits an action, updates action status, verifies mutation cursors, and proves forbidden sensitivity is excluded.

**AC18 - App/MCP parity proof.** For MCP-eligible fixtures, a correction through MCP changes the same claim/trust/receipt state as app feedback and a follow-up read reflects it. Parity excludes `Confidential` and `UserOnly` data by design.

**AC19 - No production data or PII fixtures.** Tests, docs, commit messages, PR bodies, fixtures, screenshots, and eval outputs use generic entities such as `account_01`, `project_01`, `person_01`, and synthetic domains only when needed.

**AC20 - Gates.** Focused W5 tests plus full gates pass before implementation ships:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
```

---

## Section 4 - Intelligence Loop Integration Check

**1. Claim model.** W5 read outputs are projections over claims, source-aware read models, and service-owned runtime evidence. MCP typed corrections are `claim_feedback`, not display-only flags. Submit notes may become claim/source proposals only through approved services with explicit subject, sensitivity, lifecycle, and source attribution.

**2. Provenance + trust.** MCP reads expose source-as-of, trust band, source type, redaction state, and caveats through render policy. MCP feedback preserves `Actor::McpClient` attribution so source reliability and trust effects can distinguish host-model-origin corrections from direct app actions.

**3. Signals + invalidation.** Feedback and submit-class writes keep existing service signal paths. MCP action writes sync action claims and emit action signals. Feedback emits the same claim feedback/invalidation/repair signals as app feedback.

**4. Runtime + surfaces.** Tauri and MCP consume the same ability/service substrate. MCP responses differ only by surface projection, truncation, and sensitivity egress gates. `get_daily_briefing`, `get_entity_intelligence`, recommendation/read-model producers, and source-provenance readers are the expected runtime consumers.

**5. Feedback loop.** MCP corrections feed source reliability, claim lifecycle/trust, repair jobs, receipt state, and downstream surfacing through the same feedback loop as app corrections. Submit notes/actions do not silently alter claim truth unless promoted through services-owned claim/provenance paths.

---

## Section 5 - Implementation Surface

Likely files/modules:

- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` if the explicit `dailyos.submit.claim_feedback` name needs a clarifying amendment.
- `src-tauri/src/services/mcp_v2/handlers/registration.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_workspace_search.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_workspace_source_provenance.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_portfolio.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_note.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_create_action.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_update_action_status.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_resources.rs`
- `src-tauri/src/services/mcp_v2/handlers/tool_pagination.rs` if kept as a helper/test module
- `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml`
- `src-tauri/src/services/mcp_v2/runtime_projection.rs`
- `src-tauri/src/services/claim_receipt/feedback.rs`
- `src-tauri/src/services/claims.rs`
- `src-tauri/src/services/actions.rs`
- `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs`
- source/workspace/provenance read services identified during L1
- MCP v2 integration tests under `src-tauri/tests/` or the local test convention used by existing MCP v2 tests

Do not modify the dirty main checkout. Do not add migrations unless L1 proves existing `claim_feedback.actor` / `actor_id` cannot carry MCP attribution safely and migration slots are reconciled.

---

## Section 6 - Review Dispatch

Required L0 lanes:

- `/codex challenge` - adversarial review against the full packet and current code.
- `ce-api-contract-reviewer` - MCP tool names, request/response schemas, resources, pagination, mutation cursors, and catalog/handler consistency.
- `ce-security-lens-reviewer` - MCP egress, actor attribution, hostile input, provenance/resource detail, and write boundaries.
- `ce-feasibility-reviewer` - producer availability, action status mapping, resource support, and test feasibility.
- `ce-learnings-researcher` - mandatory K-in over `docs/solutions/` and `.docs/decisions/`.

Add-on lanes:

- `ce-product-lens-reviewer` for tool-description displacement fixtures and whether the final exposed surface matches ADR-0128's product framing.
- `ce-data-migrations-reviewer` only if L1 proposes a schema change for MCP feedback origin metadata or resource state.

Cycle-1 reviewer prompts must include these known pressure points:

1. Is `dailyos.submit.claim_feedback` an authorized concrete name under ADR-0128 Section 5, or does it require a narrow amendment?
2. Does explicit MCP actor acceptance preserve feedback trust semantics without making Agent/System feedback too broad?
3. Which W5 catalog tools have real producers today, and which must be hidden or amended before implementation?
4. Is the `done/deferred/dropped` action-status vocabulary implementable through service semantics?
5. Does DOS-173 resource support exist in transport, or is a scope amendment required?
6. Are sensitivity gates applied before pagination/resource/provenance detail and not only at final projection?

---

## Section 7 - K-In Evidence

Substrate-type searches performed before authoring this packet covered:

- MCP gateway/catalog/handler primitives: `McpToolHandler`, `ToolDescription`, `McpActor`, `Actor::McpClient`, `runtime_projection`, `registration`.
- Feedback primitives: `record_claim_feedback`, `submit_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`, `validate_feedback_actor`, `actor_class_for_actor`.
- Read/provenance primitives: `get_entity_intelligence`, `get_daily_briefing`, `list_open_loops`, `Paginated`, `Cursor`, `source_asof`, `EnvelopeProvenance`, `RenderSurface::McpTool`.
- Write primitives: `create_action`, `complete_action`, `reopen_action`, `reject_suggested_action`, `dismiss_suggested_action`, `ActionStatus`.
- Prior decisions/solutions: ADR-0128, ADR-0125, ADR-0126, ADR-0093, `prompt-channel-sensitivity-class-sweep-2026-05-18.md`, `k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`, producer remediation notes, and the v1.4.9 wave plan.

Important findings:

- W5 is not greenfield. The v2 gateway, local stdio dispatch, taxonomy catalog, account-status handler, runtime projection, claim feedback writer, receipt validation, action services, daily briefing producer, and pagination helpers already exist.
- The feedback actor blocker is current code, not stale plan text. L1 must extend or deliberately preserve it; bypass is not acceptable.
- The catalog/handler mismatch is current code. W5 must close it or hide unready entries.
- The daily briefing producer exists but is not MCP-exposed. W5 should re-home it instead of reimplementing it.
- Source/provenance and sensitivity gates are documented class-pattern risks. Tests must cover the whole boundary class, not one happy path.
