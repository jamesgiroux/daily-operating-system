# DOS-833 / W2 L0 Packet - Local MCP Auth Right-Size and Hostile-Input Proof

- **Version:** v1.4.9 - W2 security right-size
- **Primary issue:** [DOS-833](https://linear.app/a8c/issue/DOS-833)
- **Related issues:** DOS-510, DOS-759, DOS-760, DOS-169, DOS-170
- **Author date:** 2026-06-03
- **Tier:** Tier 3 markdown-only
- **Scope tier:** Wave-scope security substrate. Per the Engineering Ladder, L0 requires `/codex challenge`, two planning reviewers (`ce-security-lens-reviewer` + `ce-feasibility-reviewer`), and mandatory K-in. Add `ce-data-migrations-reviewer` if any migration drops ceremony tables.
- **Status:** L0 approved after 2026-06-05 re-review; adversarial, security-lens, feasibility, and K-in verdicts pass on current packet text.

---

## Section 0 - Origination, Scope, and Trust Topology

**Origination class:** Security right-size. v1.4.9 reset the security model to match the actual product topology: a local personal runtime on the same OS user, plus a headless MCP surface consumed by local host-model clients. DOS-833 removes remote-style ceremony that does not buy security in that topology, while keeping the boundaries that still matter: actor attribution, service-only writes, tool exposure, audit, prompt-injection hardening, and sensitivity egress.

**Headline contract:** Local invoke and MCP stdio loopback work without an HMAC signing ceremony, pairing handshake, presence nonce, or caller-specific scope grant. MCP-originated calls remain attributable as `Actor::McpClient`, do not become `Actor::User`, and cannot bypass tool registration, side classification, sensitivity gates, audit, service boundaries, or hostile-input protections.

**Trust topology:**

1. **Tauri/app/file surface:** same local OS user and same local runtime. No per-client remote auth ceremony.
2. **MCP stdio surface:** same OS user, but a third-party host model consumes the output. MCP stays an egress boundary: `Confidential` and `UserOnly` content never crosses MCP unless a later approved ADR changes the sensitivity model.
3. **Remote/network MCP:** out of scope. If a future transport exposes DailyOS outside the same OS-user boundary, it needs a new L0/ADR and a real authentication design.

**Non-negotiables:**

- Preserve `Actor::McpClient` versus `Actor::User`; the distinction is provenance and local surface attribution, not remote-client authentication.
- Do not remove service-boundary enforcement. MCP write handlers must still call `services::*`, never write DB tables directly.
- Do not broaden the ADR-0128 write surface. W2 does not authorize new MCP writes beyond the existing submit/action/action-status decisions that W5 reconciles.
- Local stdio write exposure is allowlist-only. Until ADR-0128 is amended by L0/API/security/product review, `dailyos.write.place_document` is not an invocable W2 local-stdio write.
- Do not delete `surface_runtime/hmac.rs`, `services/surface_pairing.rs`, or `services/surface_nonce.rs` as a side effect. Those are SurfaceClient/WordPress substrate unless a call site is proved MCP-specific.
- Do not treat document, email, calendar, transcript, or workspace content as instructions. External and derived content is evidence only.
- Do not derive MCP audit digest keys from the SQLCipher/local DB key, SurfaceClient transport keys, pairing keys, or HMAC transport material.
- Do not make local stdio client identity caller asserted. Production local stdio identity must be server-owned and stable per install.

**Migration slots:** The wave plan reserved W2 `v278-v279`, while this branch's `public/dev` schema head is `v276`. L1 must reconcile slot ownership in `.docs/plans/v1.4.9-waves.md` before adding migrations. `v261` already drops the obsolete `mcp_transport_nonce_ledger`; any W2 migration should target only remaining obsolete MCP ceremony state such as `mcp_client_manifest`, `mcp_tool_grant`, `mcp_conversation_handle`, or `mcp_tool_call_ledger` if L1 proves they are no longer read.

### Section 0.1 - Branch and Authority Notes

This packet was authored on `codex/v1.4.9-w2-dos833-l0` from `public/dev`. The wave plan already records the plan-altitude decision: DOS-833 must supersede or amend both ADR-0102 and ADR-0128, because they still mandate pairing/HMAC/scope-manifests while the current MCP server has already moved toward a local stdio shape.

The packet does not claim a final ADR number. On this base, `.docs/decisions/0136-layout-overlay-surface-preference.md` already exists, while the v1.4.9 wave text references DOS-831 coordination around ADR-0136. L1 must choose the next free ADR/amendment number after rebasing onto current `dev`, then update the ADR index if the repo convention requires it.

### Section 0.2 - Codex Challenge Cycle 1 Findings Folded In

Cycle 1 failed this packet on three L0 blockers that are now L1 acceptance criteria:

1. Current v2 local stdio derives invocable exposure from registered tools, while the current registry includes `dailyos.write.place_document`. W2 must add a write allowlist or amend ADR-0128 before that tool is invocable.
2. `DAILYOS_MCP_LEGACY_V1=1` still starts a legacy MCP server with its own `ServerHandler`, `list_tools`, and `call_tool` path outside the v2 gateway. W2 must delete, quarantine, or prove this path cannot start in production.
3. Current MCP audit writes read `params` and `response` into append-only JSONL audit storage. W1 is retiring SQLCipher, so "encrypted local read detail" is not a real option unless L1 adds a new encryption primitive. W2 chooses keyed read-audit digests or an explicit ADR that approves plaintext local read audit.

### Section 0.3 - L0 Challenge Cycle 2 Decisions

Cycle 2 found that this packet still left three local-security contracts ambiguous. W2 now makes these decisions before L1:

1. **Audit digest key custody is W2-owned unless DOS-831 has already landed a compatible primitive.** L1 must add a dedicated install-local MCP audit digest key provider if no shared local-secret provider exists after rebase. The provider generates 32 random bytes on first use, stores them in the OS keychain under a DailyOS-specific audit-digest service/account, and exposes only a signing interface to MCP audit code. It is independent of SQLCipher/local DB keys, SurfaceClient/HMAC transport keys, and any pairing material. Tests use an injectable deterministic provider; production never falls back to a static key. Rotation is manual/operator-triggered only; append-only audit rows are not rebuilt in place, and proof must document that old digest rows remain comparable only while the old key is retained. If the key cannot be loaded, or if a required read-audit row cannot be appended to either the primary audit log or approved audit outbox, the MCP read call fails before returning the tool result to the host model. It must not "fail closed" merely by omitting plaintext audit detail while still sending the read response across MCP.
2. **Conversation continuity stays as product metadata, not auth, and uses a buildable rmcp carrier.** W2 must not delete continuity without replacement. The installed `rmcp` shape exposes only `CallToolRequestParam { name, arguments }` and `CallToolResult { content, is_error }`, so local stdio cannot rely on a custom top-level transport field. W2 therefore uses a reserved transport-metadata argument key, `"_dailyos"`, and a JSON text-content response envelope. Success responses are serialized as `{"dailyos":{"conversationHandle":"..."},"result":<typed tool value>}`. Follow-up calls echo the handle as `arguments._dailyos.conversationHandle`; the transport/gateway strips `_dailyos` before handler schema validation. Public `tools/list` input schemas must advertise optional reserved `_dailyos: { conversationHandle }` for every tool that can participate in continuity even when handler-specific schemas use `additionalProperties: false`, and tool descriptions or output schemas/docs must describe the JSON text-content envelope. `_dailyos` may contain only `conversationHandle`; attempts to pass actor, client id, scopes, side, sensitivity, raw `conversation_id`, or arbitrary metadata are rejected. The continuity store may reuse an existing table only if the schema/commentary is amended to be auth-free local continuity; otherwise L1 adds a small local continuity store with handle hash/id, local client id, issued/last-seen/expires/revoked timestamps, and no raw prompt or payload data. A host that cannot echo a handle may still make first-call writes because the gateway mints a fresh handle, but it cannot claim continuity with a prior receipt. Lifecycle tests must cover mint, echo, expiry, revocation, first-write minting, `_dailyos` stripping, malformed `_dailyos` rejection, raw `conversation_id` rejection, and a schema-validating rmcp client path that accepts the advertised `_dailyos` schema and echoes the handle call-to-call.
3. **Local MCP client identity is server-owned.** `DAILYOS_MCP_CLIENT_ID` must not be the production authority for `Actor::McpClient.client_id`. L1 must mint or load a stable per-install local MCP client id through server-owned storage and ignore caller-supplied client ids in tool params. A test-only override may exist behind `#[cfg(test)]` or explicit fixture wiring. The ADR/proof wording must not claim per-host authentication: in personal local stdio, the guarantee is stable local MCP surface attribution plus `Actor::McpClient` separation from `Actor::User`, not proof of which third-party host process invoked the server.

### Section 0.4 - L0 Challenge Cycle 4 Finding Folded In

Cycle 4 found that AC9a could pass with hand-built `rmcp` loopback requests while failing schema-following MCP hosts. The current transport advertises handler params with `additionalProperties: false`; if `_dailyos` remains hidden from `tools/list`, hosts such as Claude Desktop, Cursor, or schema-validating SDK clients may reject the echo request or never send it. W2 therefore makes schema-level discoverability part of the carrier contract: L1 must either expose optional `_dailyos.conversationHandle` in public `tools/list` schemas and document the response envelope, then prove a schema-validating rmcp client can echo the handle, or provide production-host fixtures showing hidden `_dailyos` survives despite strict schemas. The default path is schema advertisement; hidden-carrier proof is acceptable only as supplemental evidence from the actual host targets.

This also amends ADR-0111's tool-registration contract. ADR-0111 currently says MCP publishes the ability descriptor schema directly and validates inputs against that ability schema. W2 must update that ADR to allow MCP v2 public schemas to be gateway-owned transport wrappers: `handler/ability input schema + optional reserved _dailyos.conversationHandle`. `_dailyos` is transport metadata only, stripped before handler/ability validation, and cannot carry actor, client, scope, side, sensitivity, raw conversation id, or arbitrary authority. The typed `result` inside the JSON text-content envelope remains the ADR-0105/ADR-0108 ability/tool output with provenance, trust rendering, attribution, and sensitivity filtering intact.

---

## Section 1 - Existing Substrate W2 Must Consume

### Section 1.1 - MCP v2 Already Has a Ceremony-Free Local Path

`src-tauri/src/mcp/main.rs::run_v2_server` currently builds the v2 gateway, registers handlers, derives a local client id from `DAILYOS_MCP_CLIENT_ID` or a default local id, derives grants from registered tool descriptions, and starts `V2ServerHandler::from_local_stdio`.

`src-tauri/src/services/mcp_v2/transport.rs` already has two paths:

- `from_verified_pairing`, which takes a DB connection and routes through manifest-backed auth.
- `from_local_stdio`, which avoids MCP startup/auth writes and holds local stdio tool exposure in memory.

W2 should promote the local stdio path to the canonical personal-tier path. The replacement path is:

1. Registered handlers and the taxonomy catalog define the tool surface.
2. Local stdio startup loads or mints a server-owned opaque local MCP client id for attribution.
3. The transport builds normal MCP `{ name, arguments }` requests into `McpToolRequestEnvelope`.
4. The gateway admits only registered, invocable tools and rejects caller-asserted internal fields.
5. The handler invokes services or abilities with `Actor::McpClient`.
6. Audit and signal paths record MCP actor/client/tool attribution.

This is not a scope-grant model. The current `ToolGrant` shape can be retained temporarily as an implementation detail, but L1 should rename or replace it if that is the clearest way to prevent the old "per-client manifest grant" model from leaking back into the code.

### Section 1.2 - ADR-0102 and ADR-0128 Are Now Over-Specified for Local MCP

ADR-0102's 2026-05-19 amendment still requires:

- server-minted pairing handshake;
- HMAC-SHA256 transport signing;
- server-side per-client scope manifest;
- per-client/tool rate limits;
- client revocation;
- audit attribution with keyed parameter/response hashes;
- conversation-handle lifecycle;
- scope namespace freeze.

ADR-0128 cites the same trust-boundary contract while preserving the headless MCP product surface and narrow write surface.

DOS-833 must supersede or amend the current language this way:

- Keep `Actor::McpClient`, `ActorKind::McpClient`, server-owned local MCP client id, tool taxonomy, side classification, tool-description discipline, and service-only writes.
- Remove pairing handshake, transport HMAC, caller-specific scope grants, and presence nonce as requirements for local stdio / same-OS-user loopback.
- Treat tool exposure as server-owned product configuration derived from registered handlers, taxonomy, and ability policy, not as a caller-negotiated grant.
- Preserve "caller cannot assert scopes/conversation internals in params" as a request-shape invariant.
- Treat rate limiting as local backpressure/quality of service if retained, not as client authorization.
- Treat revocation as disabling a local tool/surface configuration, not revoking a remote pairing.
- Preserve audit attribution. Read-side `params` and `response` must be stored as install-local keyed digests using the dedicated W2/DOS-831-compatible audit digest key provider, unless a superseding ADR explicitly approves plaintext local read audit detail. "Encrypted local read detail" is not available on this base because audit storage is append-only JSONL and W1b retires the SQLCipher at-rest contract.
- Keep `OpaqueConversationHandle` as continuity metadata, not an auth credential. On rmcp v2 it rides in the W2-defined `_dailyos.conversationHandle` request metadata and the JSON text-content response envelope, not a custom top-level rmcp field. A host that cannot echo a handle must not be rejected solely because the old remote-client contract expected one, but it also cannot claim continuity with earlier reads/receipts unless it echoes the server-minted handle.

### Section 1.3 - Live Gateway/Auth Dependencies

The current manifest-backed path still lives in:

- `src-tauri/src/services/mcp_v2/auth.rs`: `PairingHandshake`, `pair_client`, `load_client_record`, `resolve_tool_grant`, `resolve_or_mint_handle`, `revoke_client`, `revoke_handle`, `list_invocable_tool_grants`, and `ensure_local_stdio_client_grants`.
- `src-tauri/src/services/mcp_v2/gateway.rs`: `dispatch` loads a client record, resolves a grant, checks exposure, checks scope subset, resolves/mints a conversation handle, and reserves rate-limit rows. `dispatch_local_stdio` has a parallel in-memory path.
- `src-tauri/src/services/mcp_v2/transport.rs`: `from_verified_pairing` keeps the DB-backed path reachable, while `from_local_stdio` is the active local path.
- `src-tauri/src/services/mcp_v2/contracts.rs`: scope/grant/conversation comments still cite ADR-0102's old model.
- `src-tauri/src/migrations/255_mcp_client_manifest.sql`, `256_mcp_conversation_handle.sql`, and `258_mcp_rate_limit_and_audit_outbox.sql`: old ceremony/rate-limit/audit-outbox tables.

L1 must either remove unused ceremony code or leave a deliberate compatibility seam with tests proving it is not part of local stdio auth. Leaving dead pairing APIs that tests still call as if they were canonical is not acceptable.

### Section 1.4 - MCP Sensitivity and Product Boundaries Still Matter

Right-sizing auth does not weaken MCP egress policy:

- `services::claims::prompt_input_sensitivity_allowed` centralizes the Public/Internal prompt-input gate.
- `services::claim_receipt::privacy` has an `AgentMcp` audience path with a strict allowlist and chain-max sensitivity logic.
- `abilities-runtime` provenance rendering has MCP-specific allowlists/redaction behavior.
- ADR-0128 still says MCP is a product head over the substrate, not an export pipe or broad write layer.

W2 must explicitly keep `Confidential` and `UserOnly` out of MCP. Removing scope grants is not permission to expose all local data to host models.

### Section 1.4.1 - ADR-0128 Write Surface Is an Allowlist

ADR-0128 Section D currently authorizes the submit-class writes `dailyos.submit.note`, `dailyos.submit.action`, and `dailyos.submit.action_status`. It does not authorize a broad write/file-placement surface. Current code registers `dailyos.write.place_document` and local stdio currently derives exposure from registered handlers, so W2 has to close that mismatch.

L1 has only two valid paths:

1. Hide or reject `dailyos.write.place_document` for local stdio, while preserving any non-MCP compatibility path only if explicitly quarantined and tested.
2. Return to L0 with an ADR-0128 amendment and API/security/product review that intentionally authorizes the tool and its service boundary.

Do not let a registered-handler convenience function become the write-surface policy.

### Section 1.5 - Hostile-Input Substrate Exists

ADR-0093 defines indirect prompt injection as the relevant threat: adversarial instructions embedded in documents, emails, calendar content, transcripts, or derived intelligence can try to escape into prompts or poison future memory.

Current code already has several enforcement seams:

- `src-tauri/src/util.rs` defines `INJECTION_PREAMBLE`, `wrap_user_data`, `strip_invisible_unicode`, `sanitize_external_field`, and `encode_high_risk_field`.
- `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs::placement_content_text` treats JSON and other accepted placement content as untrusted UTF-8 text bytes, not trusted structured instructions.
- `src-tauri/src/services/workspace_ingestion/extract.rs` ignores frontmatter authority, requires a pipeline-verified linked subject, emits only narrow `UserNote` claims, assigns `UserOnly` sensitivity, and routes conversion through `WorkspaceClaimProposal` / `services::claims::commit_claim`.
- `src-tauri/src/abilities/prompts/prepare_meeting_prep.v1.0.0.txt` instructs the model to use only evidence in context and treat source text as untrusted.

DOS-510 should prove these seams on the W2 threat model. It should not introduce a speculative security scanner or let document body/frontmatter set subject, sensitivity, source ids, or tool instructions.

---

## Section 2 - Chosen Architecture

### Section 2.1 - Local MCP Auth Model

The local personal-tier MCP contract is:

```text
same OS user + registered tool + server-owned exposure + Actor::McpClient attribution
```

The old contract is not:

```text
pairing handshake + shared HMAC key + caller-specific scope grant + per-client revocation
```

Canonical local stdio flow:

1. `run_v2_server` builds the gateway and registers handlers.
2. `gateway.seal()` verifies handler/catalog consistency.
3. Local stdio exposure records are derived from registered handler descriptions and the W2 MCP write allowlist. Registration alone is not authority for write exposure.
4. `V2ServerHandler::from_local_stdio` starts without opening auth tables or writing a manifest.
5. `tools/list` exposes only registered invocable tools.
6. `call_tool` builds an envelope with no hidden auth params. The only reserved transport metadata is `arguments._dailyos.conversationHandle`, which is continuity metadata, is stripped before handler invocation, and cannot carry actor/client/scope/side/sensitivity authority.
7. The gateway checks registered tool, side/exposure, request-shape invariants, optional local rate budget, and handler schema.
8. The handler invokes through service/ability boundaries as `Actor::McpClient`.
9. Audit and signal emission record client id, tool name, side, mutation cursor where applicable, and sanitized detail.

### Section 2.2 - Code-Path Replacement

L1 must make one path canonical enough that future engineers do not re-add ceremony by following old tests/comments:

- `mcp/main.rs`: keep the local stdio path as the default and test that no pairing setup is required for startup, `tools/list`, or a representative tool call.
- `transport.rs`: make `from_local_stdio` the primary constructor. Remove or mark `from_verified_pairing` test-only/legacy only if it cannot be deleted in one PR.
- `gateway.rs`: collapse duplicated `dispatch` / `dispatch_local_stdio` logic where feasible, but keep changes scoped. The important invariant is no DB-backed manifest authorization on local stdio.
- legacy MCP v1 path: delete it, make it compile/test-only, or guard it behind a non-production build flag that cannot be activated by `DAILYOS_MCP_LEGACY_V1` in the shipping binary. A production-startable legacy `ServerHandler` outside the v2 gateway fails W2.
- `auth.rs`: remove, deprecate, or quarantine pairing/grant APIs. `ensure_local_stdio_client_grants` should either disappear or become an in-memory exposure builder outside DB auth.
- `contracts.rs` and `taxonomy.rs`: rewrite comments so `Scope` is tool metadata / legacy vocabulary, not a local auth grant. Preserve tool-name namespace validation.
- local client identity: replace production `DAILYOS_MCP_CLIENT_ID` authority/default shared id with a server-owned per-install id provider. Environment/config values may be test fixtures or display aliases only; they cannot decide `Actor::McpClient.client_id` in production.
- conversation continuity: keep or replace `OpaqueConversationHandle` storage as an auth-free local continuity store. Implement the rmcp-visible carrier: extract and strip `arguments._dailyos.conversationHandle` before typed handler validation, reject disallowed `_dailyos` fields and raw `conversation_id`, and wrap successful `CallToolResult` content as `{"dailyos":{"conversationHandle":"..."},"result":...}`. Do not let removal of pairing/grants remove first-write minting, expiry, revocation, or raw `conversation_id` rejection.
- `audit.rs`: preserve actor attribution and write-payload sanitization. Replace read `params`/`response` plaintext with canonical keyed digests using the dedicated install-local audit digest key provider unless a superseding ADR approves plaintext local read audit. For read-class MCP calls, digest creation plus successful append to the primary audit log or approved audit outbox is part of the response precondition; key load failure, digest failure, and audit append/outbox failure return an MCP error without exposing the read result.
- migrations: drop or leave inert obsolete ceremony tables only after proving no production path reads them. If dropping tables, update `verify_required_schema`, reconcile slots, provide rollback notes, and keep `mcp_audit_outbox` if audit fallback still uses it.

### Section 2.3 - What "Strip Scope-Grant" Means

W2 does not mean "all registered code can run with no policy." It means:

- No caller-chosen or operator-paired per-client scope grant gates local stdio.
- No tool invocation can smuggle `granted_scopes` or `conversation_id` through params.
- Tool exposure is derived from the server's registered handler/catalog/ability policy state.
- Write exposure is additionally restricted to the ADR-0128 submit-class allowlist unless an approved ADR amendment expands it.
- Read/write side is still explicit (`Side::Read`, `Side::Write`, `Side::SubmitCorrection`).
- Write and submit-correction handlers still need bounded payloads and mutation cursors.
- MCP outputs still pass the sensitivity/provenance render policy for MCP surfaces.

If L1 retains a `Scope` vector to reuse catalog validation, it must be named and documented as server metadata, not caller authority.

### Section 2.4 - DOS-510 Evidence-Only Hostile-Input Contract

The enforcement point is the ingestion/claim/prompt path:

1. External content enters as bytes or text.
2. The ingestion service validates encoding, size, path, source handle, and linked subject.
3. Extractors may use document body as evidence text only.
4. Frontmatter/body/path cannot choose subject, sensitivity, source id, actor, lifecycle, or tool invocation.
5. Claim proposals are committed only through `services::claims::commit_claim` with explicit source attribution, `source_asof`, sensitivity, and lifecycle.
6. Any later prompt interpolation wraps external or derived content with ADR-0093 utilities and preambles.
7. Model output is validated against schema and source refs; hostile text cannot become instructions merely because it appeared in a source document.

The W2 proof must include document-content injection fixtures and derived-content re-read fixtures. A test that only checks `wrap_user_data` escapes a closing tag is necessary but not sufficient.

### Section 2.5 - Surface and Issue Boundaries

W2 is substrate work that unblocks later W5 MCP parity and W4 correction-loop proof. It does not implement the W5 tool suite or change W5's open actor decision for claim feedback writes.

DOS-169 and DOS-170 remain reconciled against ADR-0128 Section D, not re-authorized here. W2 may keep the auth model from blocking those tools, but it does not broaden the write surface.

---

## Section 3 - Acceptance Criteria

**AC1 - ADR supersession.** L1 adds or amends an ADR that supersedes ADR-0102's local MCP pairing/HMAC/scope-manifest requirements, updates ADR-0128's trust-boundary citation, and amends ADR-0111's MCP registration contract to allow MCP v2 public tool schemas to be gateway-owned transport wrappers over handler/ability schemas. The wrapper may add only optional reserved `_dailyos.conversationHandle`, must strip `_dailyos` before handler/ability validation, must keep `_dailyos` out of `AbilityDescriptor.input_schema`, and must not let `_dailyos` carry actor/client/scope/side/sensitivity authority. It preserves `Actor::McpClient`, MCP product-surface framing, narrow write surface, ADR-0105/ADR-0108 typed output/provenance obligations, and sensitivity egress gates.

**AC2 - Ceremony-free local startup.** The MCP v2 default stdio server starts, lists tools, and invokes at least one representative read tool without `pair_client`, `mcp_client_manifest`, `mcp_tool_grant`, transport HMAC, presence nonce, or user/operator pairing setup. If `mcp_conversation_handle` remains, it is used only as the auth-free continuity store named in this packet, not as startup/auth ceremony.

**AC3 - Actor attribution preserved.** MCP-originated handler calls still project to `Actor::McpClient` with a server-owned opaque local client id and, where available, conversation metadata. No MCP path silently reclassifies calls as `Actor::User`. Production local stdio does not trust `DAILYOS_MCP_CLIENT_ID`, tool params, or host-provided ids as the actor id.

**AC4 - Tool exposure remains server-owned.** `tools/list` and `call_tool` expose only registered handler/catalog entries with invocable exposure. Caller params cannot assert scopes, grants, raw conversation ids, tool side, actor, or sensitivity.

**AC5 - Scope-grant removal is real.** Static review and tests prove local stdio authorization does not depend on `mcp_tool_grant` rows, per-client manifests, or caller/operator selected scope grants. Any remaining `Scope` use is documented as taxonomy metadata or legacy compatibility, not local auth.

**AC6 - Obsolete code/tables resolved.** Pairing, manifest, remote-client revocation, and rate-ledger code/tables are deleted, migrated away, or quarantined behind explicit non-default legacy/test seams. Conversation-handle substrate is either retained as an auth-free continuity store or migrated to a replacement continuity store with the lifecycle contract in Section 0.3. The proof names every retained old symbol and why it remains.

**AC7 - Local stdio write allowlist.** `tools/list`, `call_tool`, and local exposure construction cannot expose writes outside ADR-0128's authorized submit-class trio unless an L0-approved ADR amendment expands the write surface. Tests prove `dailyos.write.place_document` is hidden or rejected for local stdio on the W2 path, or the ADR amendment explicitly authorizes it.

**AC8 - Legacy MCP v1 quarantined.** The shipping `dailyos-mcp` binary cannot start a production legacy MCP v1 server outside the v2 gateway. L1 either deletes the legacy path, makes it test-only/non-production, or adds a static/runtime check proving `DAILYOS_MCP_LEGACY_V1` cannot activate a production command path. Tests cover the old `ServerHandler` / `list_tools` / `call_tool` path or its removal.

**AC9 - Audit privacy resolved.** Audit still records MCP actor/client/tool/side attribution and mutation cursors. Write and submit-correction payloads are sanitized. Read `params` and `response` are stored as install-local keyed digests from the dedicated audit digest key provider, independent of transport HMAC, SurfaceClient keys, and SQLCipher/local DB keys, unless a superseding ADR explicitly approves plaintext local read audit. No implementation may claim encrypted local read detail without adding a real encryption mechanism and tests. For read-class MCP calls, audit success is an egress precondition: key load failure, digest failure, primary audit append failure plus outbox failure, or any other required read-audit persistence failure returns an MCP error and withholds the read response from the host model. Proof covers key source, storage, generation, fixture injection, response-withheld failure behavior, manual rotation/rebuild implications, and DOS-831 coordination.

**AC9a - Conversation continuity preserved without auth ceremony.** Local stdio returns server-minted `OpaqueConversationHandle` metadata through the W2 rmcp carrier: success `CallToolResult` content is a JSON text envelope with `dailyos.conversationHandle`, and follow-up calls echo it as `arguments._dailyos.conversationHandle`. Public `tools/list` input schemas include optional reserved `_dailyos: { conversationHandle }` for continuity-capable tools even when handler-specific params keep `additionalProperties: false`, and host-facing tool descriptions or output schema docs describe the `{"dailyos":...,"result":...}` text-content envelope. The transport/gateway strips `_dailyos` before handler schema validation, accepts only `conversationHandle`, rejects raw `conversation_id` params, rejects any `_dailyos` actor/client/scope/side/sensitivity fields, and persists enough auth-free metadata to support mint, echo, expiry, revocation, and first-write minting. The final emitted envelope for at least one ability-backed read path proves `result` retains ADR-0105/ADR-0108 actor-filtered provenance, trust, attribution, and sensitivity behavior. If L1 chooses a `provenanceHandle` plus detail-tool path instead of inline provenance/trust fields, the proof must list the canonical v2 local-stdio `tools/list`, verify the named detail tool is exposed there, invoke it with the emitted handle or invocation id, and assert the returned detail is actor-filtered, sensitivity-safe, and free of `_dailyos` in typed input/result; otherwise inline provenance/trust fields are required in `result`. `_dailyos` must be absent from both the typed ability input and typed result. Hosts that do not echo handles remain usable but receive fresh continuity. Tests prove W2 does not silently delete the ADR-0128 continuity affordance, that schema-validating rmcp clients accept the advertised `_dailyos` field and echo the handle call-to-call, that the envelope preserves typed output/provenance obligations, and that hidden-carrier compatibility is not assumed unless real target-host fixtures prove it.

**AC10 - Service-boundary writes.** MCP write and submit-correction handlers route through `services::*`. Static review shows no direct DB writes from MCP command/handler code except approved audit/outbox or service-owned writer helpers.

**AC11 - Sensitivity gate unchanged.** `Confidential` and `UserOnly` claims do not cross MCP. Tests cover at least one allowed `Internal` or `Public` fixture and one forbidden `Confidential` or `UserOnly` fixture.

**AC12 - No SurfaceClient overreach.** W2 does not remove or weaken SurfaceClient/WordPress HMAC, pairing, session, or presence-nonce code unless the touched call path is proved MCP-specific and the ADR says so.

**AC13 - Hostile document content blocked.** DOS-510 fixtures prove document body/frontmatter cannot issue instructions, choose subject, change sensitivity, invent source ids, set tool params, or create claims outside `services::claims::commit_claim`.

**AC14 - Derived-content injection blocked.** Fixtures prove derived intelligence or workspace claims re-read into prompts are treated as Tier 3 evidence: wrapped/sanitized, schema constrained, and unable to override system instructions or future memory.

**AC15 - Migration verifier alignment.** If W2 drops or makes inert any MCP auth/grant/rate table, `src-tauri/src/migrations.rs::verify_required_schema`, migration slot ownership, transactional migration proof, and rollback notes are updated in the same PR. It is not acceptable to drop tables while the schema verifier still requires them for `version >= 258`.

**AC16 - No PII fixtures.** Committed fixtures use generic entities (`account_01`, `person_01`, `project_01`) and synthetic domains only if a domain is necessary (`subsidiary.com`, `parent.com`). No real customer/account/person/domain names, local paths, emails, tokens, or raw prompt logs appear in source, tests, docs, PR text, or proof bundles.

**AC17 - Gates.** Focused W2 tests, targeted security scripts, and full gates pass before implementation ships:

```bash
src-tauri/scripts/check_ability_surface_drift.sh
src-tauri/scripts/check_audit_disclosure_allowlist.sh
src-tauri/scripts/check_audit_denylist_completeness.sh
src-tauri/scripts/check_sensitivity_gate_composition.sh
src-tauri/scripts/check_migrations_transactional.sh
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
```

---

## Section 4 - Intelligence Loop Integration Check

**1. Claim model.** DOS-833 itself does not create new claims. DOS-510 touches claim ingestion: document content can become only typed claims through `WorkspaceClaimProposal` and `services::claims::commit_claim`, with explicit subject, source attribution, `source_asof`, sensitivity, lifecycle, and claim type. No display-only hostile-input flag is enough.

**2. Provenance + trust.** MCP invocations preserve `Actor::McpClient` attribution. Workspace/document claims preserve source attribution and sensitivity. Hostile content never supplies provenance authority; it is evidence text only. Trust scoring continues to consume the claim substrate, not MCP auth state.

**3. Signals + invalidation.** W2 must not bypass existing service mutation paths that emit signals. Workspace ingestion claims and MCP submit/write handlers keep their existing service-owned signal and invalidation behavior. If W2 drops old auth tables, no signal path may depend on them.

**4. Runtime + surfaces.** Tauri/local app and MCP consume the same substrate, modulo MCP sensitivity gates. Local invoke and MCP loopback parity is required for permitted data. MCP cannot become a parallel feature surface or an unchecked export API.

**5. Feedback loop.** User corrections remain W4/W5 feedback substrate work. W2 preserves the actor/provenance shape those later corrections need and prevents hostile content from poisoning memory that future feedback/trust loops would consume.

---

## Section 5 - Implementation Surface

Likely files/modules:

- `.docs/decisions/0102-abilities-as-runtime-contract.md`
- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md`
- `.docs/decisions/0111-surface-independent-ability-invocation.md`
- `.docs/decisions/README.md` if a new ADR file is added
- `src-tauri/src/mcp/main.rs`
- `src-tauri/src/services/mcp_v2/{auth,gateway,transport,contracts,audit,taxonomy}.rs`
- `src-tauri/src/services/mcp_v2/handlers/*`
- `src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml`
- `src-tauri/src/migrations.rs`
- `src-tauri/src/migrations/278_*` / `279_*` only after slot reconciliation
- `src-tauri/src/util.rs`
- `src-tauri/src/services/workspace_ingestion/{workspace_intake_impl,extract,contracts}.rs`
- `src-tauri/src/services/claims.rs`
- prompt builders that include external or derived content
- MCP and workspace ingestion tests

Avoid:

- No new auth framework.
- No remote MCP assumptions.
- No caller-owned scopes.
- No second MCP command path outside the v2 gateway.
- No invocable local-stdio write outside the ADR-0128 submit trio unless W2 carries the approved ADR amendment.
- No direct DB writes from handlers.
- No broad write surface expansion.
- No plaintext MCP read audit by default; require keyed digests unless a superseding ADR accepts plaintext local audit.
- No weakening SurfaceClient HMAC/pairing/nonce substrate.
- No AI prompt scanner as the primary injection defense.
- No real user/customer data in fixtures or proof.

---

## Section 6 - Test and Proof Plan

Focused MCP tests:

- Default v2 stdio startup builds handler/catalog state without opening/writing MCP auth tables.
- Default v2 stdio startup loads or mints a server-owned local MCP client id; production actor id is not caller/environment asserted.
- Local stdio returns and accepts `OpaqueConversationHandle` through the rmcp-visible carrier: success `CallToolResult` JSON text content contains `dailyos.conversationHandle`, follow-up request `arguments._dailyos.conversationHandle` is accepted and stripped before handler validation, malformed/disallowed `_dailyos` fields are rejected, and tests cover mint, echo, expiry, revocation, first-write minting, and raw `conversation_id` rejection.
- `tools/list` schemas advertise optional reserved `_dailyos.conversationHandle` for continuity-capable tools despite handler-specific `additionalProperties: false`; tool descriptions or output schema docs document the `{"dailyos":...,"result":...}` text-content envelope; a schema-validating rmcp client test proves the advertised schema accepts the echoed handle and that the gateway still strips `_dailyos` before typed handler validation.
- A final `CallToolResult` envelope test for at least one ability-backed read parses the emitted JSON text, asserts `dailyos.conversationHandle`, asserts `result` retains ADR-0105/ADR-0108 actor-filtered provenance/trust/attribution/sensitivity behavior, and asserts `_dailyos` is absent from typed ability input and typed result. If L1 uses `provenanceHandle` instead of inline provenance/trust fields, the same proof must list the canonical v2 local-stdio `tools/list`, verify the named detail tool is exposed there, invoke it with the emitted handle or invocation id, and assert the returned detail is actor-filtered, sensitivity-safe, and has no `_dailyos` in typed input/result.
- `tools/list` returns registered invocable handlers from local exposure records.
- `tools/list` and `call_tool` hide or reject `dailyos.write.place_document` unless ADR-0128 is amended in this branch.
- Representative `dailyos.read.*` call succeeds with no pairing/HMAC/manifest setup.
- Caller-provided `granted_scopes`, `conversation_id`, side, actor, or sensitivity params are rejected or ignored per schema.
- `Actor::McpClient` reaches handler/service code; a regression test fails if MCP is projected as `Actor::User`.
- Forbidden sensitivity fixture does not appear in MCP output.
- Read audit stores canonical keyed parameter/response digests from the dedicated audit digest key provider or an ADR-approved plaintext detail; write/submit-correction audit omits raw payload fields and includes only IDs/mutation cursor.
- Audit digest key tests cover first-use generation, stable reuse, deterministic fixture provider, missing-key response-withheld behavior, primary append failure with outbox success, primary append plus outbox failure returning an MCP error with no read result, and proof that the provider does not call the SQLCipher/local DB key provider or SurfaceClient/HMAC key paths.
- Static or unit test proves no default local stdio path calls `pair_client`, `load_client_record`, `resolve_tool_grant`, or `resolve_or_mint_handle` for auth.
- Static or unit test proves `DAILYOS_MCP_LEGACY_V1` cannot activate a production legacy MCP server path outside the v2 gateway.

Focused DOS-510 tests:

- Workspace placement content with prompt-injection text is accepted only as text evidence or dropped; it cannot alter subject, sensitivity, source id, or actor.
- Frontmatter claiming authority over subject/sensitivity/source is ignored and produces the existing warning/dropped-fact behavior.
- JSON-looking content is treated as untrusted UTF-8 text, not trusted structured instructions.
- Derived workspace/user-note content re-read into prompts is wrapped with ADR-0093 utilities and preamble.
- Model-output validation rejects invented source refs or source ids.

Proof bundle:

- L0 reviewer verdicts and K-in findings.
- ADR amendment/new ADR diff.
- Migration slot reconciliation note if schema changed.
- Migration verifier and transactional-script proof if old MCP auth/rate tables are dropped or made inert.
- Focused MCP and DOS-510 test output.
- Targeted MCP/security script output.
- Full gate output.
- PII-safe local invoke plus MCP loopback transcript showing no signing/pairing ceremony.
- PII-safe hostile-input fixture report showing blocked instruction effects.

---

## Section 7 - K-in Findings

Knowledge-store discovery ran against `docs/solutions/`, `.docs/decisions/`, the v1.4.9 wave plan, and live MCP/workspace-ingestion code for auth, HMAC, pairing, scope, nonce, hostile input, prompt injection, and substrate type names.

Relevant hits:

- `.docs/plans/v1.4.9-waves.md` - W2 explicitly scopes DOS-833 to stripping HMAC/pairing/presence-nonce/scope-grant for local same-OS-user and DOS-510 to ADR-0093 evidence-only handling.
- `.docs/plans/v1.4.9-reconciliation-ledger-2026-05-29.md` - W2 disposition is right-size security to OS boundary; close signed-route/pairing follow-ups; keep hostile-input proof.
- `.docs/decisions/0102-abilities-as-runtime-contract.md` - current MCP amendment still mandates pairing/HMAC/scope manifest/rate limits/revocation/audit hash language that DOS-833 must supersede.
- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` - MCP remains a headless product surface with a narrow write surface and trust-boundary citation to update.
- `.docs/decisions/0093-prompt-injection-hardening.md` - external and derived content must be treated as untrusted data, wrapped/sanitized, and schema constrained.
- `docs/solutions/workflow-issues/debug-vortex-dispatch-codex-for-fresh-diagnosis-after-2h-2026-05-22.md` - prior diagnosis records that MCP v2 substrate had already moved to a simplified shape and transport ceremony was never the real blocker.
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` - avoid folding broad substrate into the wrong ticket; W2 must stay right-sized, not become a remote-auth redesign.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` - K-in must search substrate primitives such as nonce, pairing, scope, and handler registration, not only the proposed DOS-833 name.
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` - sensitivity gates belong in central service readers; W2 cannot weaken MCP/prompt egress while simplifying auth.

No prior solution was found that already ships the full DOS-833 ADR supersession plus code cleanup and DOS-510 proof. Existing code does contain the main replacement seam: `V2ServerHandler::from_local_stdio` and `mcp/main.rs::local_stdio_grants_for_registered_tools`.

---

## Section 8 - L0 Reviewer Dispatch

Required:

- `/codex challenge` or project-approved equivalent: adversarial review for auth-boundary regression and over/under-correction.
- `ce-security-lens-reviewer`: review local trust topology, MCP egress, hostile-input fixtures, audit privacy, and PII-proof discipline.
- `ce-feasibility-reviewer`: verify the code-path replacement and migration plan are buildable against current MCP v2 code.
- `ce-learnings-researcher`: mandatory K-in over `docs/solutions/` and `.docs/decisions/`.

Conditional:

- `ce-data-migrations-reviewer`: required if W2 drops or migrates MCP auth/rate tables.
- `ce-product-lens-reviewer`: required if L1 changes ADR-0128's headless product surface or write-surface posture.

Approval standard:

- Unanimous L0 approval required before L1 implementation.
- Any finding that W2 removes `Actor::McpClient`, broadens MCP writes, leaks `Confidential`/`UserOnly` to MCP, weakens SurfaceClient auth, leaves a default pairing/HMAC/scope-grant requirement, or treats hostile document text as instruction authority is BLOCKING.

### Section 8.1 - L0 Cycle 3 Verdicts

Cycle 3 is the passing L0 cycle after the packet chose the rmcp-visible `_dailyos.conversationHandle` carrier and made read-audit persistence a response precondition.

| Reviewer | Verdict | Notes |
|---|---|---|
| `/codex challenge` | APPROVE | No P1 findings. P2s: migration-slot cleanup, host-facing docs/fixtures for the JSON text-content envelope, retained legacy seam risk, audit key rotation/retention proof. |
| `ce-security-lens-reviewer` | APPROVE | No P1/P2 security findings after `_dailyos` carrier validation, read-audit response withholding, server-owned client identity, MCP egress, SurfaceClient preservation, and hostile-input proof were made explicit. |
| `ce-feasibility-reviewer` | APPROVE | No P1 findings. `_dailyos` carrier plus JSON text response envelope is buildable against current `rmcp`; migration risk remains conditional and already acceptance-gated. |
| `ce-learnings-researcher` | APPROVE | No P1 substrate miss. Relevant hits: ADR-0102, ADR-0128, ADR-0027, ADR-0111, ADR-0135, ADR-0094, ADR-0092, ADR-0068, ADR-0093, ADR-0108, ADR-0101, ADR-0071, ADR-0112, and matching `docs/solutions/` entries. |

Conditional reviewers still apply during L1/L2 if triggered:

- `ce-data-migrations-reviewer` if W2 drops or makes inert MCP auth/rate/continuity tables.
- `ce-product-lens-reviewer` plus API/security/product approval if L1 expands ADR-0128's write surface instead of hiding/rejecting `dailyos.write.place_document`.

### Section 8.2 - 2026-06-05 L0 Re-review Verdicts

The 2026-06-05 re-review reran the required W2 L0 lanes after the project owner clarified that blocked L0 packets continue through reviewer cycles until unanimous pass. The re-review found and folded in AC9a host-schema, ADR-0111 wrapper, final-envelope provenance, and canonical v2 detail-tool proof gaps. The current packet text is the passing text.

| Reviewer | Verdict | Notes |
|---|---|---|
| `/codex challenge` | APPROVE | Prior blockers closed: `_dailyos.conversationHandle` must be advertised in public schemas or proven by real host fixtures; ADR-0111 wrapper authority is explicit; final envelopes must preserve typed provenance/trust/sensitivity; `provenanceHandle` detail paths must be exposed and invoked through canonical v2 local stdio or inline provenance/trust fields are required. |
| `ce-security-lens-reviewer` | APPROVE | No remaining L0 security blocker. `_dailyos` is metadata-only, stripped before handler/ability validation, barred from authority fields, absent from typed inputs/results, and covered by sensitivity/provenance proof. |
| `ce-feasibility-reviewer` | APPROVE | Current AC9a is implementable. Inline provenance/trust is compatible with the current account-status v2 shape; a detail-tool path is buildable only if L1 adds a v2 catalog entry, handler, registration, listing, and actor-filtered lookup proof. |
| `ce-learnings-researcher` | APPROVE | No remaining ADR/solution contradiction. ADR-0111 amendment, ADR-0105/0108 output preservation, v2-exposed detail-tool proof, centralized sensitivity gates, and transport-metadata-not-authority lessons are applied. |
