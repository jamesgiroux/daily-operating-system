# DOS-168 — MCP v2 ability/service gateway + actor policy + auth + audit — L0 plan packet

**Wave:** v1.4.7 W1-A
**Lane spec:** [DOS-168](https://linear.app/a8c/issue/DOS-168)
**Wave plan:** `.docs/plans/v1.4.7-waves.md` §"Agent W1-A — DOS-168"
**Authoring discipline:** scoped tight; cycle-2 same-shape findings driven by my substrate-grep errors → cycle-3 grounds every claim with the verified path. Cycle-2 architectural questions resolved via small W0 amendment (~30 LOC + ADR-0102 cycle-7 text) folded into this packet as a precondition deliverable.

## Cycle-7 changelog (2026-05-20) — close cycle-6 NEW HIGHs via substrate landing

Cycle-6 verdicts: challenge **BLOCK**; architect / CSO / devex **NEEDS-CHANGES**. 3 convergent NEW HIGH: (a) per-response nonce refresh + transport envelope shape (CSO + challenge + architect MED); (b) nonce ledger lifecycle gaps (issued_at/expires_at, fail-closed) (architect MED + challenge HIGH + CSO MED); (c) L6-3 handler docs NOT actually landed (devex HIGH + challenge) — taxonomy.rs was still a stub despite my packet claim.

**Substrate landed BEFORE cycle-7 dispatch (per "stop the shape-only is a trap" pattern):**

1. **`services/mcp_v2/contracts.rs`** — `OpaqueNonce` newtype + `request_nonce: OpaqueNonce` on `McpToolRequestEnvelope` + `next_request_nonce: OpaqueNonce` on `McpToolResponseEnvelope` + parity tests (`opaque_nonce_wire_is_transparent_string` + updated 4 envelope parity tests). 27/27 contracts tests pass.
2. **ADR-0102 cycle-9 amendment** — §C.bis.refresh (per-response nonce refresh; seed at pairing, fresh nonce per response, asymmetric flow), §C.bis.schema (full ledger DDL with `issued_at`/`expires_at`/`consumed_at` + 3 indexes + atomic consume SQL), §C.bis.fail-closed (consumed nonce stays consumed even on downstream failure — fail-closed against TCP-reset replay attack), §C.bis.sweep (background sweep cadence + grace window).
3. **`services/mcp_v2/taxonomy.rs`** — actual `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` (architect cycle-6 LOW: catalog-neutral naming, NOT `HandlerYamlMismatch`) + ~130 lines of rustdoc with concrete Side::Read no-cursor example + concrete Side::Write required-cursor example + composite-mutation example + cursor-shape-per-service table (claim_id → CommittedClaim.new_claim_id at services/claims.rs:209; signal_id → emit* String return at services/signals.rs:46-106; action_id → services::actions::*). cargo check clean.

Cycle-7 packet changes:

1. **§1 #1 gateway flow narrative** updated to use the new request envelope shape: Gate 0 verifies HMAC over (payload + `envelope.request_nonce`); calls `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh. On success, gateway invokes handler; on response, calls `auth::issue_next_nonce(client_id)` → returns `next_request_nonce` in response envelope.
2. **AC-2** unchanged structurally (decision order preserved); Gate 0 references new `verify_and_consume_nonce`.
3. **AC-6** mutation_cursor convention: SHOULD-with-warning (NOT MUST-fail per devex cycle-6 MED). Side::Write without cursor → Suite-S `mcp_write_handler_missing_cursor` signal + audit row written without cursor field. Forced MUST-fail (dev-mode validator that rejects test fixtures) filed as separate W1.5 ticket.
4. **AC-7 reject signal durability** (challenge cycle-6 MED): `emit_signal` for `McpInvocationRejected` uses the emit-or-log pattern; failure → fallback warning log + Suite-S alert `mcp_reject_signal_emit_failed`. No forensic blind spot.
5. **AC-11 + AC-12 + AC-5** unchanged (already correct per cycle 6).
6. **Migration v243 schema reflects ADR-0102 §C.bis.schema**: full DDL with `nonce, client_id, issued_at, expires_at, consumed_at` + 3 indexes + atomic consume predicate + background sweep + 1h grace window.
7. **L6-3 substrate-truth verified**: taxonomy.rs now contains `TaxonomyCatalog` trait + rustdoc; not a stub.

Cycle-7 net delta: ~25 lines packet + 60 LOC code (contracts.rs OpaqueNonce + envelope amendments + tests) + 130 LOC code (taxonomy.rs trait + rustdoc) + 100 lines ADR cycle-9 amendment. Cumulative cycles 1→7: 151 → 296 LOC packet (+145 over 7 cycles, well within the K-in `<80/cycle` discipline target).

---

## Cycle-6 changelog (2026-05-20) — L6 decisions folded in

User L6 verdict (3 structural decisions) folded in this cycle:

**L6-1 (replay guard) = nonce ledger.** Per ADR-0102 §C cycle-8 amendment (this cycle): every MCP message MUST carry a server-issued `request_nonce` signed in the HMAC; server stores recently-seen nonces in `mcp_transport_nonce_ledger` (v243-shifted-up to v244 — see slot reallocation below); replay attempts within window → `ToolError::BadParams { detail: "nonce_replayed" }`. Mirrors `services::surface_nonce::verify_and_consume` pattern verbatim. Closes cycle-5 CSO HIGH.

**L6-2 (reject audit) = drop `internal_reject_reason`.** AC-6 narrows: audit append happens ONLY on handler success. Auth-state rejections (`PairingRevoked` / `ExposureForbidden` / `Unauthorized` / `ConversationRevoked`) emit Suite-S signal `mcp_invocation_rejected` (per ADR-0115 NonPiiMetadata) carrying `client_id` (if manifest resolved) or `<unresolved>` plus the reject reason. No audit row for rejects. Closes cycle-5 challenge HIGH + architect MED. Removes the AC-6 vs handler-success-precondition contradiction.

**L6-3 (handler docs) = rustdoc on TaxonomyCatalog trait.** Cursor convention documented as rustdoc on `TaxonomyCatalog` trait in `services/mcp_v2/taxonomy.rs`. ~30 lines: trait-level overview + per-method examples (`Ok(json!({..., "mutation_cursor": {"claim_id": 42}}))`); naming conventions; allowed cursor ID shapes (UUID, integer, opaque hash; NEVER PII per AC-6). `cargo doc` surfaces it for handler authors. Closes cycle-5 devex HIGH.

Cycle-6 packet edits:

1. **AC-2 + §1 #1 + AC-12** add nonce check as Gate 0 (BEFORE PairingRevoked): `auth::verify_transport_hmac(payload, sig, presented_nonce)` verifies HMAC AND checks/consumes nonce; failure → `ToolError::BadParams { detail: "invalid_signature" }` OR `{ detail: "nonce_replayed" }`. Identical wall-clock floor via `sleep_until(start + 10ms)` to prevent timing oracle.
2. **AC-6 reworded** (L6-2): "Handler success precedes audit append. On rejection, NO audit row is written; instead a `SignalType::McpInvocationRejected` is emitted (NonPiiMetadata; payload: `client_id_or_unresolved`, `reject_reason`). Audit detail JSON: `{client_id, conversation_handle, tool_name, params_hash, response_hash, mutation_cursor?}` — `internal_reject_reason` field REMOVED (per L6-2; rejections handled via SignalType not audit). Outbox path unchanged for JSONL append failures on success-rows."
3. **AC-7 extended**: registers TWO new SignalTypes — `McpToolInvoked` (existing) + `McpInvocationRejected` (new). Both NonPiiMetadata per ADR-0115. Same 5-touchpoint wiring.
4. **AC-11 + AC-12** unchanged structurally (Zeroizing wrapper, 10ms floor preserved); applies to nonce-check path too.
5. **§14 + taxonomy.rs**: trait gets rustdoc per L6-3. Concrete examples for handler authors.
6. **Migration slot reallocation**: insert v243 for nonce ledger; bump original v243 (rate-limit ledger + audit outbox) → v244.
   - **v241** `mcp_client_manifest` (unchanged)
   - **v242** `mcp_conversation_handle` (unchanged: `(handle, client_id, mint_at, last_touched_at, revoked_at)` per cycle-3 binding)
   - **v243** `mcp_transport_nonce_ledger` (`nonce, client_id, consumed_at`; UNIQUE composite on `(nonce, client_id)`; index for cleanup; window = 5 minutes)
   - **v244** `mcp_tool_call_ledger` + `mcp_audit_outbox` (was v243)
7. **ADR-0102 §C cycle-8 amendment** (this cycle) documents the nonce ledger requirement: 5-minute window, `services::surface_nonce` precedent, BadParams variants. Lands in Deliverable 0 alongside cycle-7.

Cycle-6 net delta: +30 lines packet + 1 substrate landing (nonce ledger spec) + 1 ADR amendment.

---

## Cycle-5 changelog (2026-05-20)

Cycle-4 verdicts: codex challenge **BLOCK**; architect / CSO / devex **NEEDS-CHANGES**. **Unanimous (4/4)** on two issues: (i) §1 #1 still contradicted AC-2 on the absent-grant branch (3rd cycle hitting the same propagation gap); (ii) `ServiceMutationCursor` was claimed as landed but `rg` finds no such type in `contracts.rs` (and the substrate citations for `CommittedClaim.claim_id` and `services::signals::publish -> SignalId` were wrong — actual shapes are `CommittedClaim { claim, new_claim_id }` and `services::signals::emit*` returning `String`). 3/4 on namespace collision lint not credible (script absent, paths wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`, W1-B YAML doesn't exist yet). 2/3 on AC-12 ±5ms ceiling not portable on macOS/Linux CI.

Cycle-5 changes (all surgical, all substrate-verified via `rg` before writing):

1. **§1 #1 absent-grant branch fixed** (4/4 convergent). Narrative now reads: "absent grant OR non-Invocable exposure → `ExposureForbidden { tool_name }`; grant present AND invocable, but `scopes_required.is_subset(scopes_granted)` false → `Unauthorized { missing_scope }`." Matches AC-2 verbatim. Grep-validated: `rg 'absent grant' dos-168-l0-plan.md` returns only the matching new wording.
2. **ServiceMutationCursor REMOVED from Deliverable 0 / W0 amendment scope** (4/4 convergent). The `McpToolHandler::invoke` W0-frozen trait signature does NOT change. Cursor enforcement happens at the gateway level via JSON inspection: gateway extracts `result_value.get("mutation_cursor")` after handler success for `Side::Write` handlers. If absent for `Side::Write`, gateway logs a Suite-S warning `mcp_write_handler_missing_cursor` (forensic alert; handler effects preserved) and proceeds to audit append with `mutation_cursor` field absent in detail JSON. The cursor convention is documented in W1-A `taxonomy.rs` `TaxonomyCatalog` trait docs (Side::Write handlers SHOULD populate cursor). No new W0 type added.
3. **Substrate citations corrected** (architect HIGH + devex HIGH + CSO MED). `CommittedClaim` is at `services/claims.rs:209` with `new_claim_id` field (not `claim_id`); `services::signals::emit*` returns `String` (at `services/signals.rs:46-106`), not `publish -> SignalId`. AC-6 cursor format: `mutation_cursor: serde_json::Value` (typed-less, IDs only — no payloads per CSO MED). Convention documented in taxonomy.rs trait docs.
4. **Namespace collision CI REMOVED from W1-A scope** (3/4 convergent). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" (Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`). The CI lint requires W1-B YAML which doesn't exist yet AND clean reading of the legacy tool list at `src-tauri/src/mcp/main.rs:715-730` (challenge cycle-4 cited path is correct). Cycle-4 §7 + AC-10 reference + §11 entry all dropped.
5. **AC-12 timing floor reworked** (2/3 — challenge + CSO). Drop ±5ms ceiling (not portable). Gateway uses `tokio::time::sleep_until(start + Duration::from_millis(10))` before returning ANY `Unauthorized` / `ExposureForbidden` / `PairingRevoked` / `ConversationRevoked` error. Test asserts both missing-client and invalid-HMAC branches return after `>= 10ms` wall-clock; no upper bound. Identical error shape constraint preserved (uniform JSON keys).
6. **AC-11 key custody spelled out** (CSO not-closed). `auth::verify_transport_hmac` wraps loaded key in `Zeroizing<[u8; 32]>` from the `zeroize` crate; per `Zeroize` Drop impl the buffer is wiped on function exit. (NOT the line 623 derivation; correct precedent is `SecretBytes32` at `src/surface_runtime/hmac.rs:643`.) Test asserts `verify_transport_hmac` does not retain key reference past function return.
7. **AC-6 internal reject reason** (architect MED). Audit detail JSON includes `internal_reject_reason: Option<String>` carrying machine-readable cause (`absent_grant`, `non_invocable`, `missing_scope:<scope>`, `invalid_hmac`, `pairing_revoked`, `conversation_revoked`) for forensic ordering. Wire shape stays merged — caller sees only the typed `ToolError`. No leak.
8. **AC-5 ledger pruning order** (challenge tightening). Explicit transaction order: `BEGIN IMMEDIATE` → `DELETE FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2 AND called_at < (?3 - window_seconds_ms)` → `SELECT COUNT(*) FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2` → if `count >= max_calls` ROLLBACK + return `RateLimited`; else `INSERT INTO mcp_tool_call_ledger VALUES (?1, ?2, ?3)` → `COMMIT`.
9. **`mcp_audit_outbox` access control** (CSO MED). Same access pattern as JSONL audit log per ADR-0094: operator-only via existing audit inspection paths (`get_audit_log_records` / `export_audit_log` Tauri commands). No separate CLI / no MCP exposure of outbox contents. Outbox is forensic only.
10. **AC-6 mutation_cursor cap** (devex MED). `mutation_cursor` JSON field max depth 4, max byte size 2 KiB (uniform with existing `MCP_INVOCATION_CACHE_ENTRY_BYTE_CAP / 5` budget at `bridges/mcp.rs`). Test asserts oversize → gateway logs warning + truncates to placeholder `"truncated_oversize"` in audit detail; handler effects preserved.
11. **DX note added** (devex MED). 10ms response floor on auth-state errors adds latency vs sub-ms paths. Acceptable security tradeoff (prevents client_id enumeration). Hosts amortize via successful-result caching.

Cycle-5 packet net delta: +35 lines. Cumulative cycle 1→5: 151 → ~253. Cycle-5 is the convergence cycle; no new scope, only AC + narrative tightening + 1 ticket spinoff.

---

## Cycle-4 changelog (2026-05-20)

Cycle-3 verdicts: codex challenge **BLOCK** (1 NEW HIGH AC-2 + 2 MED; cycle-2 "STILL-OPEN" findings were all about Deliverable 0 not having landed yet); architect **NEEDS-CHANGES** (7/7 closed; 1 NEW HIGH AC-2; 1 MED wave-plan-stale); CSO **NEEDS-CHANGES** (2 STILL-OPEN due to Deliverable 0 unlanded + 1 unverified-floor; 3 NEW MED + 1 LOW); devex **NEEDS-CHANGES** (4/4 packet-level closed; 1 NEW HIGH "Deliverable 0 must actually land" + 1 NEW HIGH mutation_cursor + 1 MED fixture realism).

Convergent NEW HIGH (3/4): AC-2 internal inconsistency between §1 #1 and the AC body on `Unauthorized` vs `ExposureForbidden` branching. **All cycle-3 "STILL-OPEN" findings about Deliverable 0 are now closed by substrate landing** — `contracts.rs` carries `ExposureForbidden` variant + golden parity test; ADR-0102 carries cycle-7 amendment (§C.bis + §D.bis); wave plan stale text fixed at lines 85, 453, 467.

Cycle-4 changes:

1. **Deliverable 0 has landed in this same branch** (closes the 4/4 "shape-only is a trap" finding). Substrate-truth, not packet promise. Cycle-4 reviewers can grep `ExposureForbidden` in `contracts.rs` + read ADR-0102 cycle-7 amendment + verify wave plan lines 85/453/467 carry the new text.
2. **AC-2 reworded to architect's recommended decision order** (closes convergent NEW HIGH). Decision order: (i) `revoked_at.is_some()` → `PairingRevoked`; (ii) absent grant OR `exposure != Invocable` → `ExposureForbidden { tool_name }`; (iii) grant present + invocable but `scopes_required.is_subset(scopes_granted)` false → `Unauthorized { missing_scope }`; (iv) caller-asserted scope/conversation_id in params → `BadParams`. §1 #1 narrative aligned.
3. **AC-12 timing oracle gains numeric floor** (CSO MED): floor = 10ms minimum response time on `Unauthorized` failure paths (missing-client + invalid-HMAC); tolerance = ± 5ms jitter ceiling. Test asserts measured wall-clock floor is `>= 10ms - 5ms = 5ms` for both branches. Implementation: gateway uses `tokio::time::sleep_until(start + 10ms)` before returning either error.
4. **AC-11 transport key uses `Zeroizing<[u8; 32]>`** (CSO MED): per existing `surface_runtime/hmac.rs:623` precedent — Rust drop is insufficient for transient key material. `auth::verify_transport_hmac` borrows the key inside `Zeroizing` wrapper; on function exit the buffer is zeroized.
5. **mcp_tool_call_ledger pruning + composite index** (challenge MED): v243 migration adds `idx_mcp_tool_call_ledger_client_tool_called ON (client_id, tool_name, called_at)` + same-transaction prune step in the rate-limit reservation: `DELETE FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2 AND called_at < ?3 - window_seconds_ms` before the `BEGIN IMMEDIATE` insert.
6. **mutation_cursor specified per service** (devex HIGH + challenge MED): for `Side::Write` handlers, the gateway requires the handler's `Ok(value)` payload to carry a `mutation_cursor: ServiceMutationCursor` field. `ServiceMutationCursor` is a tagged enum: `Claim { claim_id: ClaimId }`, `Signal { signal_id: SignalId }`, `Action { action_id: ActionId }`, `Multi(Vec<ServiceMutationCursor>)` — handlers route through `services::claims::commit_claim` (returns `CommittedClaim { claim_id: ClaimId, ... }` per `services/claims.rs:204`), `services::signals::publish` (returns `SignalId` per `services/signals.rs:46-106`), or `services::actions::*` (returns `ActionId`). For `Side::Read` handlers, `mutation_cursor` is omitted from the audit detail (no mutation happened). New W0 type added in cycle-4 substrate landing: `ServiceMutationCursor` in `services/mcp_v2/contracts.rs` (small additive type, sibling to `ToolError`).
7. **Legacy coexistence interim guard** (CSO MED): **SUPERSEDED by cycle-5 #4** — the namespace collision CI is moved out of W1-A scope (script doesn't exist; cross-wave dep on W1-B YAML; path was wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" in Maintenance project. AC-10 records the spinoff.
8. **AC-3b fixture realism** (devex MED): cross-client test fixture uses two real paired `McpClientId`s minted via the pairing handshake (not synthetic strings), and a real `OpaqueConversationHandle` minted under client A presented by client B.
9. **Wave-plan stale text fixed and verified in same commit** (architect MED): lines 85, 453, 467 now carry the dedicated-variant language per ADR-0102 cycle-7 amendment.

Cycle-4 packet net delta: +20 lines (198 → ~218). Tight; cycle-4 is convergence cycle, not new scope.

---

## Cycle-3 changelog (2026-05-20)

Cycle-2 verdicts: codex challenge **BLOCK** (1 STILL-OPEN HIGH + 2 NEW HIGH + 2 MED); architect **NEEDS-CHANGES** (2 STILL-OPEN + 3 NEW HIGH + 2 MED); CSO **NEEDS-CHANGES** (0 STILL-OPEN + 3 MED + 1 LOW); devex **BLOCK** (2 STILL-OPEN HIGH + 3 NEW HIGH + 2 MED). Convergent: sentinel-scope-on-Unauthorized (3/4), `bridges/mcp.rs:737` misidentified (2/4 — confirmed factually as test fixture), audit wire envelope mutation (2/4 — confirmed factually as W0 violation), handle client_id binding gap (2/4).

Cycle-3 changes:

1. **W0 amendment commit added as Deliverable 0** (Class N — 3/4 convergent + Class O — 2/4 convergent). Lands BEFORE W1-A body. Two changes to `src-tauri/src/services/mcp_v2/contracts.rs`:
   - New `ToolError::ExposureForbidden { tool_name: ScopedName }` variant. Same shape as `PairingRevoked`/`ConversationRevoked` per ADR-0102 §C — auth-state classes that do not abuse `Scope` newtype. New parity test added.
   - `OpaqueConversationHandle` wire shape unchanged (still opaque string); server-side binding spec amended in ADR-0102 §D — handles bind to `client_id` at mint time, resolve by `(client_id, handle)`, cross-client presentation returns `ConversationRevoked`. Wire layer stays compatible.
   ADR-0102 §C/§D cycle-7 amendment text drafted as part of this commit (~6 lines added to ADR-0102).
2. **§1 #8 bridges/mcp.rs change REMOVED** (Class Q — 2/4, factually confirmed). The `todo!()` at `src/bridges/mcp.rs:737` is inside `provenance_actor_for_test` (`#[cfg(test)] mod tests`), not production routing. There is no production `Actor::McpClient` arm in `bridges/mcp.rs` to wire in W1-A. Test fixture is filled by W2+ when the first `Actor::McpClient`-using handler ships parity tests.
3. **Audit wire envelope mutation REMOVED** (Class R — 2/4, factually confirmed). `audit_outbox_pending` warning no longer surfaces on `McpToolResponseEnvelope` (W0 frozen with only `conversation_handle` + `result`). Outbox state stays behind `audit.rs` — operator-visible only (Suite-S regression alert when outbox row count > 0).
4. **Audit signature corrected** (Class S — 1/4, factually confirmed). `AuditLogger::append_with_actor(event: &str, actor: &Actor, fields: AuditFields)`. `event = "mcp.tool_invoked"` is the SEPARATE arg; `AuditFields { category: "security", detail: {...}, wp_user_id: None, wp_user_hash: None, request_id: Some(...) }` carries the structured payload. `audit_log.rs:325` already documents that McpClient routes through this path with `wp_user_id/hash = None`.
5. **AC-2 reworded** (Class N closure): non-Invocable exposure returns `ToolError::ExposureForbidden { tool_name }` (W0 amendment, item 1 above). No sentinel scope. Missing scope still returns `ToolError::Unauthorized { missing_scope }`.
6. **Handle migration shape updated** (Class O closure): v242 schema is `mcp_conversation_handle { handle, client_id, mint_at, last_touched_at, revoked_at }` with composite unique on `(handle, client_id)`. Resolve takes `(client_id, handle)`; mismatch → `ConversationRevoked`. AC-3b adds explicit cross-client rejection test.
7. **Migration filename clarified** (Class T — architect HIGH #3): registered `version` field is authoritative. Filenames mirror version with no offset: `241_mcp_client_manifest.sql`, `242_mcp_conversation_handle.sql`, `243_mcp_rate_limit_and_audit_outbox.sql`. The `migration-filename-version-offset` solution doc applies to a different convention; cited here only as a discipline reminder, not as offset guidance.
8. **W1-A/W1-B taxonomy seam restructured** (Class V — architect MED): W1-A defines the CATALOG INTERFACE in `taxonomy.rs` (validation function signature + binding contract). W1-B owns YAML LOADING + boot validation invocation. W1-A no longer dispatches off W1-B-owned YAML at runtime; the binding is purely a registry-time validation seam.
9. **Manifest lookup is indexed** (Class U — architect MED): per-dispatch DB query is `SELECT * FROM mcp_tool_grant WHERE client_id = ? AND tool_name = ?` (indexed composite key), NOT `tool_grants: Vec<ToolGrant>` full-load. Manifest struct holds one resolved `ToolGrant` per call, not the full vector.
10. **AC-6 audit-required emit-or-log caveat addressed** (Class W — challenge MED): if JSONL append fails AND outbox insert also fails, gateway returns `ToolError::Internal { trace_id }` to caller (handler effects preserved but caller is warned via Internal error), emits Suite-S alert `mcp_audit_double_failure` for operator action. Documented as a documented degradation path per the cited emit-or-log caveat.
11. **Mid-call revocation audit state** (Class X — challenge MED): if revocation lands between manifest load and handler return, audit row's `detail` includes `revocation_state: "revoked_during_invocation"` per ADR-0102 §C.
12. **Key custody AC added** (Class Y — CSO MED): transport key loaded from keychain for ONE HMAC verification per dispatch and dropped after; gateway state never caches the raw key. Test asserts `Manifest` struct contains `KeychainRef`, not raw bytes.
13. **Timing oracle AC added** (Class Z — CSO LOW): missing-client and invalid-HMAC return the same `ToolError::Unauthorized` shape with the same response timing (bounded by a uniform sleep floor). Test asserts both paths return the same wall-clock-floor.
14. **AC-6 temporal ordering note added** (Class AA — devex MED): audit is post-commit attribution, not source-of-order truth. AC-6 detail carries mutation cursor IDs (e.g., `claim_id`, `signal_seq`) so audit ordering can be reconstructed from substrate even when audit timestamp != mutation timestamp.
15. **Taxonomy mismatch returns structured error** (Class BB — devex MED): boot-time mismatch returns `TaxonomyError::HandlerYamlMismatch { handler, yaml }` to the registry init path; operator-friendly error message + operator-init suggested fix. No `panic!()`.
16. **Wave plan stale text fixed** (Class G — devex HIGH): doc-only edit to `.docs/plans/v1.4.7-waves.md:451+467` removing `Unauthorized { missing_scope: "valid_conversation" }`, lands in same commit as W0 amendment.

Cycle-3 packet net delta from cycle-2: +60 lines (151 → 184 → ~245). Most of the delta is the changelog above + AC tightening + the W0 amendment specification. Not in cycle-2's "5+ net-new findings per cycle / 80+ line revision" diminishing-returns range.

---

## Deliverable 0 — W0 amendment commit (precondition)

Lands BEFORE W1-A body (own commit, own L2 cycle). Files (all already landed in this branch as of cycle-3/cycle-6 substrate work):

- `src-tauri/src/services/mcp_v2/contracts.rs` — `ToolError::ExposureForbidden { tool_name: ScopedName }` variant + golden parity test (`tool_error_exposure_forbidden_wire_shape`). LANDED.
- `.docs/decisions/0102-abilities-as-runtime-contract.md` — cycle-7 amendment (§C.bis sentinel-scope ExposureForbidden + §D.bis handle client_id binding) + cycle-8 amendment (§C.bis.replay nonce ledger per L6-1 user verdict). LANDED.
- `.docs/plans/v1.4.7-waves.md` — stale `Unauthorized { missing_scope: "valid_conversation" }` text fixed at lines 85 + 451 + 467. LANDED.

Total: ~45 LOC code + ~50 lines doc. All grep-verified against substrate before cycle-6 dispatch.

---

## 1. What this lane ships (W1-A body, lands AFTER Deliverable 0)

1. **`src-tauri/src/services/mcp_v2/gateway.rs`** — `Gateway::handle_tool_call(envelope: McpToolRequestEnvelope) -> McpToolResponseEnvelope`. Per-dispatch flow (matches AC-2 verbatim): **Gate 0a** `auth::verify_transport_hmac(payload, sig)` — HMAC failure → `ToolError::BadParams { detail: "invalid_signature" }`. **Gate 0b** `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh (cycle-9 amendment) — atomic SQL `UPDATE mcp_transport_nonce_ledger SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`; on `rowsAffected = 0` reject with `{ detail: "nonce_replayed" }` (row consumed) or `{ detail: "invalid_signature" }` (row absent/expired — uniform shape). Fail-closed: consumed nonce stays consumed even on downstream failure. Then: `auth::load_client_record(client_id)` → reject if `revoked_at.is_some()` (→ `PairingRevoked`); `auth::resolve_tool_grant(client_id, tool_name)` (indexed lookup, single row) → **absent grant OR `grant.exposure != Invocable` → `ExposureForbidden { tool_name }`**; **grant present AND invocable but `tool.description().scopes_required.is_subset(grant.scopes_granted)` false → `Unauthorized { missing_scope }`**; `auth::resolve_or_mint_handle(client_id, envelope.conversation_handle)` (mismatch → `ConversationRevoked`); enforce rate limit BEFORE dispatch via atomic ledger insert (AC-5 explicit tx order); construct `McpActor::Client`; dispatch via registered `McpToolHandler`; on success → extract `mutation_cursor` from `result_value.get("mutation_cursor")` for `Side::Write` (Suite-S warning if absent for Write — SHOULD not MUST per AC-6); → `audit::write(event, fields, actor)` via emit-or-log path (handler effects preserved on audit failure); emit `SignalType::McpToolInvoked`; **call `auth::issue_next_nonce(client_id)` to mint fresh nonce** → return `next_request_nonce` in response envelope. On any auth-state rejection (BadParams from Gate 0a/0b, PairingRevoked, ExposureForbidden, Unauthorized, ConversationRevoked): NO audit row written; emit `SignalType::McpInvocationRejected` via emit-or-log helper (failure → fallback log + Suite-S alert `mcp_reject_signal_emit_failed` per AC-7); gateway calls `tokio::time::sleep_until(start + Duration::from_millis(10))` for AC-12 timing parity; mint fresh nonce on response anyway (so client can recover).
2. **`src-tauri/src/services/mcp_v2/auth.rs`** — `pair_client(handshake) -> McpClientId` (mirrors `services::surface_pairing` + `services::surface_session_keychain` per ADR-0102 §C); `load_client_record(client_id) -> ClientRecord`; `resolve_tool_grant(client_id, tool_name) -> Option<ToolGrant>` (indexed query); `verify_transport_hmac(payload, sig)` (loads key for one verify, drops); `resolve_or_mint_handle(client_id, presented) -> OpaqueConversationHandle` (mint on None; resolve by `(client_id, handle)`; mismatch → `ConversationRevoked`); `revoke_handle / revoke_client` admin paths.
3. **`src-tauri/src/services/mcp_v2/audit.rs`** — `write(event, actor, params, response_or_error, request_id)`. Computes keyed HMAC-SHA256 hashes per ADR-0102 §C. Calls `AuditLogger::append_with_actor(event = "mcp.tool_invoked", actor = &Actor::McpClient { .. }, fields = AuditFields { category: "security", detail: serde_json!({client_id, conversation_handle, tool_name, params_hash, response_hash, revocation_state?, mutation_cursor?}), wp_user_id: None, wp_user_hash: None, request_id: Some(...) })`. On JSONL append failure → insert into `mcp_audit_outbox` table. On outbox failure → return `ToolError::Internal { trace_id }` to caller + emit Suite-S `mcp_audit_double_failure` alert.
4. **`src-tauri/src/services/mcp_v2/actor_policy.rs`** — `ClientRecord -> AbilityPolicy` projection consumed by gateway. Per ADR-0102 §B asymmetry, `Actor::McpClient` carries no scopes; this module owns the projection.
5. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — W1-A defines the CATALOG INTERFACE only: `pub trait TaxonomyCatalog { fn validate_against_handlers(&self, handlers: &[&dyn McpToolHandler]) -> Result<(), TaxonomyError> }` + `TaxonomyError::HandlerYamlMismatch { handler: ScopedName, yaml: Option<ScopedName> }`. W1-B fills the YAML loader and invokes `validate_against_handlers` at boot.
6. **`src-tauri/src/signals/policy_registry.rs`** — adds `SignalType::McpToolInvoked` at 5 sites per ADR-0115: enum variant; `from_name`; `canonical_name`; known-names const; `policy_for` returning `SignalPolicy::local_observation + NonPiiMetadata`. Payload: `tool_name`, `client_id`, `conversation_handle`. NO raw params/response. Suite S invariant.
7. **`src-tauri/src/migrations.rs`** — FOUR migrations in v241–v244 (cycle-6 added v243 nonce ledger per L6-1; original v243 rate-limit + audit-outbox shifted to v244). Latest registered is v240 at `migrations.rs:927`; filename mirrors version with no offset:
   - **v241** `241_mcp_client_manifest.sql` — `mcp_client_manifest (client_id, paired_at, revoked_at, transport_key_ref)` + `mcp_tool_grant (client_id, tool_name, scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs)` with index `idx_mcp_tool_grant_client_tool ON (client_id, tool_name)`.
   - **v242** `242_mcp_conversation_handle.sql` — `mcp_conversation_handle (handle, client_id, mint_at, last_touched_at, revoked_at)` with `UNIQUE(handle, client_id)` + `idx_mcp_conversation_handle_lookup ON (client_id, handle)`.
   - **v243** (cycle-6 NEW per L6-1; cycle-7 schema completed per ADR-0102 §C.bis.schema) `243_mcp_transport_nonce_ledger.sql` — `mcp_transport_nonce_ledger (nonce TEXT, client_id TEXT, issued_at INTEGER, expires_at INTEGER, consumed_at INTEGER NULL)` with `UNIQUE(nonce, client_id)` + `idx_mcp_nonce_lookup ON (client_id, nonce)` + `idx_mcp_nonce_consumed ON (consumed_at)` + `idx_mcp_nonce_expires ON (expires_at)`. Window: 5 minutes (`expires_at = issued_at + 5min`) per ADR-0102 §C.bis.refresh. Atomic consume per §C.bis.schema: `UPDATE ... SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`. Background sweep every 5 min deletes rows where `expires_at < now() - 1 hour` (1h grace per §C.bis.sweep). Fail-closed per §C.bis.fail-closed.
   - **v244** (was v243) `244_mcp_rate_limit_and_audit_outbox.sql` — `mcp_tool_call_ledger (client_id, tool_name, called_at)` for sliding-window rate accounting (`BEGIN IMMEDIATE` for atomic reservation) + `mcp_audit_outbox (id, event, detail_json, actor_kind, request_id, created_at, drained_at)` forensic store.
8. **`src-tauri/scripts/check_mcp_tool_handler_allowlist.sh`** — CI lint forbidding direct DB writes from `services/mcp_v2/handlers/*.rs`. Acknowledged porous per `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md`; structural enforcement filed as a separate maintenance ticket if a real bypass surfaces in W2+.

## 2. Coexistence with legacy MCP

The wave plan §W1-A explicitly admits coexistence: v2 introduces a v2-paired ingress for clients that opt into the ADR-0102 §C contract; legacy `src/mcp/main.rs` + `src/bridges/mcp.rs:402` continue serving `Actor::Agent` (pre-pairing substrate). The two paths route by `Actor` variant at the abilities-runtime layer (per ADR-0102 §F `AbilityPolicy.allowed_actors`).

The challenge HIGH "every MCP-originated invocation must pass §C four gates" applies forward — to v1.4.7-introduced ingress. Legacy is grandfathered; retirement is a separate Linear ticket (filed at L0 close).

**W1-A does NOT touch legacy code paths.** The `Actor::McpClient` test fixture in `src/bridges/mcp.rs:737` is filled by W2+ when first handler ships its parity test; production legacy code (`mcp/main.rs:1379` rmcp stdio serving) stays unchanged.

**W1-A does NOT ship:** per-tool handler bodies (handlers/*.rs placeholders untouched); tool taxonomy YAML (W1-B); MCP write allowlist extension (W4); new `Actor::McpClient` enum variant or `McpClientId` / `OpaqueConversationHandle` newtypes (W0 already shipped in `abilities-runtime/src/abilities/registry.rs:124,153,441`); new pairing transport design (mirrors surface_pairing).

## 3. Frozen substrate citations (W0 + W0-amendment + ADRs)

- **W0 contracts (`src-tauri/src/services/mcp_v2/contracts.rs`)** — `ToolDescription`, `Side`, `ScopedName`, `Scope`, `ParamSpec`, `ReturnSpec`, `ToolExample`, `Page<T>`, `OpaqueConversationHandle` (wire envelope, opaque string), `McpClientId`, `McpActor::Client`, `McpToolRequestEnvelope` (no granted_scopes — gateway hydrates), `McpToolResponseEnvelope` (frozen: `conversation_handle` + `result`; no audit warnings), `McpToolResult`, `McpToolHandler` trait, `ToolError` (Deliverable 0 adds `ExposureForbidden`).
- **`abilities-runtime/src/abilities/registry.rs:106-153,441-480`** — `Actor::McpClient`, `McpClientId`, `OpaqueConversationHandle` runtime types.
- **`abilities-runtime/src/inventory.rs:63,412-433,611,679`** — `ActorKind::McpClient` discriminator + `AbilityActor::project()` Agent+Invocable → McpClient.
- **`src/audit_log.rs:47,149,325`** — `AuditRecord`, `AuditFields`, `AuditLogger::append_with_actor(event, actor, fields)` (event SEPARATE arg). McpClient explicitly noted at line 350-354.
- **ADR-0102 §C/§D/§E + cycle-7 amendment (Deliverable 0)** — four-gate trust contract, handle lifecycle with client_id binding, scope namespace.
- **ADR-0094** — JSONL append-only audit substrate.
- **ADR-0111 §8** — SurfaceClient pairing precedent.
- **ADR-0115** — signal registration discipline.
- **ADR-0128 §6 + §C** — continuity affordance; wire shape defers to ADR-0102 §D.

## 4. K-in citations (present-on-dev)

- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — applied; cycle-3 grounded every claim with a verified path.
- `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md` — audit-required caveat at line 50 addressed via §1 #3 double-failure escalation.
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — acknowledged porous; structural enforcement filed if W2+ surfaces a bypass.
- `docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md` — relevant only as filename discipline reminder; v241–v243 do NOT use an offset; `registered version` is authoritative.
- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — Deliverable 0 W0 amendment satisfies the precedent.

## 5. Per-client substrate format (frozen at L0 cycle 3)

```rust
// src-tauri/src/services/mcp_v2/auth.rs
pub struct ClientRecord {
    pub client_id: McpClientId,
    pub paired_at: SystemTime,
    pub revoked_at: Option<SystemTime>,
    pub transport_key_ref: KeychainRef,        // raw key never in ClientRecord/state
}

pub struct ToolGrant {                          // resolved per-call via indexed query
    pub tool_name: ScopedName,
    pub scopes_granted: Vec<Scope>,
    pub exposure: McpExposure,
    pub rate_limit: ToolRateLimit,
}

pub struct ToolRateLimit { pub max_calls: u32, pub window_seconds: u32 }
```

Manifest is composed at call time from `ClientRecord` + `Option<ToolGrant>` — never bulk-loaded.

## 6. Acceptance criteria

- **AC-1 Gateway dispatch.** Unknown tool → `ToolError::BadParams { detail: "unknown tool name" }`. Handler dispatch ONLY after AC-2..AC-5 pass.
- **AC-2 Scope + exposure authorization (cycle-4 decision-order locked).** Per-dispatch, in this exact order: (i) `ClientRecord.revoked_at.is_some()` → `ToolError::PairingRevoked`. (ii) `auth::resolve_tool_grant(client_id, tool_name)` indexed query: absent grant OR `grant.exposure != Invocable` → `ToolError::ExposureForbidden { tool_name }`. (iii) grant present AND invocable, but `tool.description().scopes_required.is_subset(grant.scopes_granted)` is false → `ToolError::Unauthorized { missing_scope }`. (iv) caller-asserted `granted_scopes` or `conversation_id` in `params` → `ToolError::BadParams`. `McpActor::Client` constructed ONLY after manifest resolution (post step iii pass).
- **AC-3a Handle lifecycle.** First call no handle → mint(client_id), dispatch, return new handle. Subsequent call with handle → resolve `(client_id, handle)`, refresh last_touched_at, dispatch. Expired handle (24h sliding) → silent mint of replacement (transparent). Revoked handle → `ConversationRevoked`. Caller-asserted `conversation_id` in params → `BadParams`.
- **AC-3b Cross-session + cross-client tests.** (a) New MCP session with same client_id + non-revoked handle → resolves successfully, handler sees same handle. (b) Different client_id presenting another client's handle → `ConversationRevoked` (binding enforcement). Both integration tests required.
- **AC-4 Pairing revocation.** Per-dispatch `ClientRecord` load → revoked → `PairingRevoked`. Propagates within ≤ 1 call per ADR-0102 §C.
- **AC-5 Rate limit (cycle-5 tx order).** Atomic reservation via explicit transaction sequence: `BEGIN IMMEDIATE` → `DELETE FROM mcp_tool_call_ledger WHERE client_id=?1 AND tool_name=?2 AND called_at < (?3 - window_seconds_ms)` (prune) → `SELECT COUNT(*) FROM mcp_tool_call_ledger WHERE client_id=?1 AND tool_name=?2` → if `count >= max_calls` ROLLBACK + return `RateLimited { retry_after_seconds }`; else `INSERT INTO mcp_tool_call_ledger VALUES (?1, ?2, ?3)` → `COMMIT`. Key tuple: `(ActorKind::McpClient, McpClientId, ScopedName)` (ledger is MCP-only so ActorKind is implicit; audit + signal carry the full tuple). Concurrent burst test required.
- **AC-6 Audit append (cycle-6 L6-2: success-only).** Handler success precedes audit append. Gateway calls `AuditLogger::append_with_actor(event="mcp.tool_invoked", actor=&actor, fields=AuditFields::new("security", detail).with_request_id(req_id))`. Detail JSON: `{client_id, conversation_handle, tool_name, params_hash, response_hash, mutation_cursor?, revocation_state?}` — `mutation_cursor` is IDs-only `serde_json::Value` (max depth 4, max 2 KiB; truncate → `"truncated_oversize"`), required for `Side::Write` (handler-filled per `taxonomy.rs` rustdoc), omitted for `Side::Read`; `revocation_state` only present if revocation lands mid-call per ADR-0102 §C. Auth-state rejections do NOT write an audit row — handled via `McpInvocationRejected` signal (AC-7) per L6-2. JSONL append fails → insert into `mcp_audit_outbox` (operator-only access via existing `get_audit_log_records` / `export_audit_log` paths; no MCP exposure, no separate CLI). Outbox insert also fails → `ToolError::Internal { trace_id }` + Suite-S alert `mcp_audit_double_failure`. Raw params/responses NEVER stored. Audit is post-commit attribution, not source-of-order truth.
- **AC-7 Signal emission (cycle-7 with reject-signal durability).** TWO new `SignalType` variants, both registered at 5 sites per ADR-0115 (variant + `from_name` + `canonical_name` + known-names const + `policy_for`); both `SignalPolicy::local_observation + NonPiiMetadata`: (a) `SignalType::McpToolInvoked` on dispatch success — payload `{tool_name, client_id, conversation_handle}`. (b) `SignalType::McpInvocationRejected` on auth-state rejection — payload `{client_id_or_unresolved, reject_reason, tool_name_or_unresolved}`. Reject-signal emission uses the emit-or-log pattern: on emission failure, fallback to operator warning log + Suite-S alert `mcp_reject_signal_emit_failed` (closes cycle-6 challenge MED — replaces audit forensics for rejects so emission failure cannot become a forensic blind spot). `client_id_or_unresolved` is either the resolved `McpClientId` or the literal `"unresolved"` (Gate 0a/0b failures, missing-client). `tool_name_or_unresolved` always present (even Gate 0 failures echo the envelope's `tool_name`). NO raw params/response in either event.
- **AC-8 CI gate.** `scripts/check_mcp_tool_handler_allowlist.sh` rejects direct DB writes from `services/mcp_v2/handlers/*.rs`. Wired in CI.
- **AC-9 Required checks.** `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green.
- **AC-10 Legacy coexistence.** Legacy `src-tauri/src/bridges/mcp.rs:402` + `src-tauri/src/mcp/main.rs:1379` continue with `Actor::Agent`; W1-A does NOT touch them. Test fixture at `src-tauri/src/bridges/mcp.rs:737` left `todo!()` (W2+ fills when first McpClient handler ships). Namespace collision CI (was cycle-4 #7) is OUT of W1-A scope per cycle-5 spinoff Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" (Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).
- **AC-11 Key custody (cycle-5 spelled out).** `auth::verify_transport_hmac` wraps loaded key bytes in `Zeroizing<[u8; 32]>` from the `zeroize` crate; per `Zeroize` Drop impl the buffer is wiped on function exit. Pattern follows `SecretBytes32` precedent at `src-tauri/src/surface_runtime/hmac.rs:643` (NOT the line 623 key derivation, which is a separate concern). `ClientRecord` never holds raw key bytes — only `KeychainRef`. Tests: (a) `ClientRecord` struct field shape is `KeychainRef`, not `[u8; 32]`; (b) post-call snapshot of stack/heap shows no zeroed-pattern lingering bytes for the loaded key.
- **AC-12 Timing oracle (cycle-6 expanded for Gate 0).** Gateway calls `tokio::time::sleep_until(start + Duration::from_millis(10))` before returning ANY of: `BadParams { detail: "invalid_signature" }`, `BadParams { detail: "nonce_replayed" }` (Gate 0 failures per L6-1), `PairingRevoked`, `ExposureForbidden`, `Unauthorized`, `ConversationRevoked`. Test asserts measured wall-clock from request entry to error response is `>= 10ms` for ALL branches (missing-client, invalid-HMAC, replayed-nonce, missing-grant, non-invocable, missing-scope, expired-handle, cross-client-handle). NO upper bound assertion (per cycle-4 challenge/CSO portability finding). Identical error-shape constraint preserved: same JSON keys, same JSON-canonical byte length within 16 bytes (BadParams variants distinguishable only by `detail` field content — same shape).

## 7. Test plan

- **Unit**: dispatch, unknown-tool, missing-scope, ExposureForbidden, revoked-pairing, revoked-handle, first-write-mint, expired-handle-replacement, raw-conversation-id-rejection, raw-granted-scopes-rejection, cross-client-handle-rejection.
- **Integration**: stub `McpToolHandler` proving (a) audit row iff handler success, (b) JSONL fail → outbox path, (c) JSONL+outbox fail → Internal + Suite-S alert, (d) signal payload carries only `tool_name + client_id + conversation_handle`, (e) rate-limit ledger consumes per full tuple atomically, (f) cross-session same-client (AC-3b.a), (g) cross-client rejection (AC-3b.b), (h) revocation within 1 call.
- **Property**: HMAC canonical-JSON parity for `params_hash`.
- **Concurrency**: `BEGIN IMMEDIATE` rate-limit burst test.
- **Timing**: AC-12 oracle test — missing-client vs invalid-HMAC indistinguishable in error shape + bounded floor.
- **CI lint**: planted direct-conn fixture rejected.

## 8. Security gates (CSO + plan-devex-review at L0 AND L2)

Per ADR-0102 §C four gates + W0 cycle-7 amendment + cycle-3 ACs:
1. Caller scopes ignored at dispatch (AC-2).
2. Caller conversation IDs rejected (AC-3a).
3. Audit hashes keyed HMAC-SHA256, raw data forbidden (AC-6).
4. Signal no-leak (AC-7).
5. Atomic pre-dispatch rate limit (AC-5).
6. Per-dispatch revocation, ≤ 1-call propagation (AC-2 + AC-3a + AC-4).
7. Per-tool exposure tier enforced via `ExposureForbidden` variant (AC-2 + Deliverable 0).
8. Handle client_id binding (AC-3b + Deliverable 0).
9. Key custody — single-verify, no caching (AC-11).
10. Timing oracle — uniform failure shape (AC-12).
11. Audit double-failure → Internal + Suite-S alert (AC-6).
12. Mid-call revocation captured in audit `revocation_state` (AC-6 + ADR-0102 §C).

## 9. Out-of-scope findings → DailyOS Maintenance

L0/L2 findings not bound by AC-1..AC-12 or by named ADR sections file as separate Linear tickets to `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. Already-identified followups to file at L0 close:
- "v1.4.7 W1.5 — legacy MCP retirement plan" (ADR-0102 §C forward-application clarification + plan to flip clients to v2 or deprecate the legacy path).
- "MCP gateway handler crate split" (structural replacement for `check_mcp_tool_handler_allowlist.sh` grep lint when W2+ surfaces a real bypass).

## 10. Open questions

| # | Question | Default | Resolution path |
|---|---|---|---|
| Q1 | Rate-limit default budgets | 60/min per (client_id, tool_name); per-client global 600/min | CSO L0 sign-off on this default; tunable in manifest at pairing |
| Q2 | Forensic outbox drain cadence | On next gateway invocation; if N>100 backlog, drain immediately | architect L0 sign-off |

## 11. Files (W1-A exclusive ownership)

| File | State | Owner |
|---|---|---|
| `src-tauri/src/services/mcp_v2/contracts.rs` | Deliverable 0 (W0 amendment: +`ExposureForbidden` variant + parity test) | shared (additive only) |
| `src-tauri/src/services/mcp_v2/gateway.rs` | W0 placeholder → W1-A body | exclusive |
| `src-tauri/src/services/mcp_v2/auth.rs` | W0 placeholder → W1-A body | exclusive |
| `src-tauri/src/services/mcp_v2/audit.rs` | W0 placeholder → W1-A body | exclusive |
| `src-tauri/src/services/mcp_v2/actor_policy.rs` | W0 placeholder → W1-A body | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | W0 placeholder → W1-A trait + error type only | exclusive (interface only; W1-B fills loader) |
| `src-tauri/src/services/mcp_v2/handlers/*.rs` | W0 placeholders | **untouched** |
| `src-tauri/src/signals/policy_registry.rs` | function-level addition (5 sites) | shared (additive) |
| `src-tauri/src/migrations.rs` | new v241–v243 entries | shared (additive) |
| `src-tauri/src/audit_log.rs` + `audit.rs` | **read-only** (reused via `append_with_actor`) | shared |
| `src-tauri/src/bridges/mcp.rs` | **untouched** (production); test fixture at line 737 untouched | n/a |
| `src-tauri/scripts/check_mcp_tool_handler_allowlist.sh` | NEW | exclusive |
| `.docs/decisions/0102-abilities-as-runtime-contract.md` | Deliverable 0 (cycle-7 amendment +6 lines) | shared (additive) |
| `.docs/plans/v1.4.7-waves.md` | Deliverable 0 (lines 451 + 467 stale-text fix) | shared |

## 12. Depends-on

- W0 (PR #322) — merged ✅.
- Deliverable 0 — lands as part of this lane's first commit.
- v1.4.5 frozen contracts — NOT a hard dep for W1-A.

## 13. Definition of Done

§6 AC-1..AC-12 met; L0 unanimous APPROVE on cycle 3; Deliverable 0 commit + W1-A body commit each pass L2; commit-msg `L2-status: passed`; PR opens against `dev` after L2 clears.
