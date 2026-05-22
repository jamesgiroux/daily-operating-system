# L0 Packet — Local Trust-Model Unification (First-Party + Third-Party)

**Tickets:** DOS-761 (first-party WP loopback as `Actor::User`) + DOS-762 (live readers + observability) + **DOS-168 amended** (MCP v2 substrate at the new shape — transport-ceremony out, authorization machinery in; PR C closes DOS-168).
**Framing:** apply one trust-model reframe consistently to ALL local surfaces. The abilities runtime is a headless substrate. Local consumers (first-party WP plugin via loopback; third-party MCP agents via stdio/in-process) all live on the same kernel-protected machine as the user. The right gates are **authorization** (what scopes does this caller have) and **operational** (rate-limit, audit, sensitivity gating) — NOT **transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor) which was modeled on the wrong premise and propagated forward from v1.4.2 WP pairing → v1.4.7 MCP v2 W1-A.
**Branch base:** `pairing-ux-humanize` (fork SHA `852a0118`); PR #351 open with prerequisite fixes.
**Cross-track action:** **hold PR #347 merge** (v1.4.7 W1-A) — its substrate is reshaped by this packet's §3e. PR #355 (DOS-758 `McpHandlerContext`) is **unaffected** — handler context is transport-agnostic and can land independently. DOS-647-C (in-process MCP hosting) becomes simpler under the new model.
**Author:** Claude Opus 4.7, session 2026-05-22.

---

## §1 — Problem statement (the framed thesis)

The v1.4.1 → v1.4.4 narrative was: build the abilities-runtime substrate so any surface can consume it without re-wiring the world. The premise was correct; the execution **conflated authorization (what scopes a caller has) with transport-ceremony (HMAC signing, nonce ledger, key custody)**. The conflation originated in v1.4.2 WP pairing and **propagated forward** into v1.4.7 MCP v2 W1-A — the waves doc names this explicitly: *"Server-issued `McpClientId` after pairing handshake (mirrors v1.4.2 SurfaceClient pairing pattern); HMAC-SHA256 transport signing for stdio/HTTP MCP transports"*. Six L0 cycles got CSO sign-off on the internal coherence; nobody asked whether transport-ceremony was the right answer for local stdio/loopback at all.

**The threat model — load-bearing, read before scoring this packet.** DailyOS is personal-tier: single user, single machine, no network egress for substrate data, encrypted local DB via SQLCipher with Keychain-held key. The OS user IS the principal. **Same-user processes are inside the trust boundary, not outside.** Any process running as the user already has filesystem access to the user's home, can read SSH keys / browser cookies / clipboard / screen, and could capture keystrokes — so positing "a rogue local process hits `/v1/local/invoke`" as a threat ignores that the same process has more direct attack vectors. The trust boundary that matters is: cross-user (other OS accounts on the machine), cross-machine (network), and physical access to the unencrypted file (defended by SQLCipher + Keychain). Gates that defend "same-user-but-not-the-installed-app" are defending no one — there is no such actor in the model. If a reviewer surfaces a finding of the shape "what if some other process running as the user does X" — that finding is explicitly out of scope; the disposition is reject-as-not-in-threat-model, not fold-in.

**The right decomposition.** Gates aren't one stack; they're two stacks with different threat models:

| Gate class | What it defends | First-party loopback (Tauri React, WP plugin) | Third-party local stdio (Claude Desktop, Cursor) | Third-party remote HTTP (future) |
|-----------|-----------------|-----------------------------------------------|--------------------------------------------------|----------------------------------|
| **Authorization** (scope manifest, per-tool exposure tier, sensitivity gating) | Caller can't do more than user authorized | meaningful (implicit — same code we ship) | **meaningful** | meaningful |
| **Operational** (rate-limit, audit log, confirmation attestation) | Runaway loops, accountability, destructive-op guard | meaningful (audit + confirmation) | **meaningful** | meaningful |
| **Transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor, transport key custody) | In-flight tamper, replay, side-channel on a wire | **n/a** (kernel-protected loopback) | **n/a** (stdio is in-process pipes; in-process hosting per DOS-647-C makes this even more obvious) | meaningful |

The transport-ceremony column applies to **remote network transports**. Locally, the kernel is the trust boundary and the OS user is the principal. We built the transport-ceremony column for local-only consumers because we modeled on a "WP plugin is third-party HTTP API" mental model that was wrong. Then we copied that wrong model into MCP v2.

**Layer 1 — First-party WP loopback (DOS-761).** WP plugin (`class-dailyos-runtime-client.php:256–278`) ships **16 HTTP headers per request** through 8 validation gates for an invoke from a process running as the same OS user as Tauri. WP plugin is code we ship. The Tauri React renderer skips all 8 gates by going through in-process `tauri::invoke` as `Actor::User`. WP plugin belongs in the same column.

**Layer 2 — Substrate readers (DOS-762).** `attach_live_workspace_readers` at `src-tauri/src/services/context.rs:61` registers 9 reader handles. The `ServiceContext` API exposes 14. The 4 missing readers (`with_list_open_loops_reader`, `with_account_list_reader`, `with_person_list_reader`, `with_project_list_reader`) are exactly what W1 producers compose against. Producer error propagates through `bridges/types.rs:1003` and `surface_runtime/mod.rs:3400` to wire code `auth_missing`. **6 distinct rejection paths collapse into `auth_missing`** with no `log::warn!`. The wire code lies about the cause.

**Layer 3 — Third-party MCP transport-ceremony rip (new ticket).** v1.4.7 W1-A's substrate (PR #347, not yet merged to dev) contains ~645 LOC of transport-ceremony layered on top of ~1100 LOC of authorization machinery. For local stdio (today's primary MCP transport) and in-process hosting (DOS-647-C's goal), the transport-ceremony defends threats that don't exist. Rip the transport-ceremony layer; keep the authorization machinery verbatim. Specifics in §2c and §3e.

**Why all three at once.** They're the same correction applied consistently. Splitting them means the reframe lands in one place and is contradicted by adjacent code in another — exactly the conflation that produced this mess. The 6-cycle L0 on v1.4.7 W1-A was internally coherent but premised on a wrong taxonomy; landing this packet IS the correction.

**Explicitly preserved (load-bearing — see §8):** authorization machinery (per-client scope manifest, per-tool exposure tier, rate-limit, plain audit, conversation handle lifecycle), `Actor::McpClient` as a distinct actor class (third-party agents need scope-distinct attribution even if transport-ceremony goes), v1.4.2 W3-C WP-mediated MCP path, the pairing handshake itself, PR #355's `McpHandlerContext` interface (transport-agnostic), connector OAuth (Slack/Gmail external calls).

---

## §2 — Current state, file:line grounded

### 2a — Gate stack (DOS-761)

| # | Gate | File:Line | What it does | Defends against (same-OS-user threat model) |
|---|------|-----------|--------------|---------------------------------------------|
| 1 | `validate_transport_headers` | `src-tauri/src/surface_runtime/mod.rs:3914` | Host = `127.0.0.1:<port>`, origin check | Browser CSRF (loopback only; n/a for PHP curl) |
| 2 | Loopback rate-limit (token bucket) | `surface_runtime/mod.rs:1015` | Per-IP request budget | DoS from local app (negligible) |
| 3 | HMAC signature verify | `surface_runtime/mod.rs:1109` | Verifies `X-DailyOS-Signature` over canonical request | Tamper of in-flight loopback traffic (kernel-protected) |
| 4 | `validate_signed_session_readonly` | `surface_runtime/mod.rs:1154` (impl `services/surface_pairing.rs`) | Session id ↔ wp_user_id ↔ site-nonce binding | Cross-tenant session replay (single-tenant local) |
| 5 | `validate_session_bound_wp_user_id_for_request` | `surface_runtime/mod.rs:1969` | Body/query/header wp_user_id matches session-bound | Inconsistent identity injection (no second identity exists) |
| 6 | `SurfaceClientBridge::authorize` (scope + cap) | `surface_runtime/mod.rs:2123` | Granted scope contains requested ability; allowed_actors includes SurfaceClient | Privilege escalation (we're already Actor::User in React) |
| 7 | Per-axis rate-limit | embedded in #6 | Per-ability budget for the surface client | Runaway local consumer (cheap to local-budget) |
| 8 | Audit-emit contract: `Actor::SurfaceClient` ⇒ `wp_user_id = Some(_)` | `src-tauri/src/audit_log.rs:104,141` | Refuses to emit audit if SurfaceClient actor lacks wp_user_id | Audit accountability (compromised by collapsing to Actor::User — see §3) |

**Materialization.** `registry_actor = validated.actor.clone()` (`mod.rs:2152`) wraps as `Actor::SurfaceClient { surface_client_id }`. The React path constructs `Actor::User` directly via `ServiceContext::new_live().with_actor(...)` in `services/context.rs:139`.

### 2b — Missing readers (DOS-762)

`ServiceContext` `with_*` methods (`abilities-runtime/src/services/context.rs`):

| Reader | Line | Currently attached? | Producer dependency |
|--------|------|---------------------|---------------------|
| `with_entity_context_reader` | 1779 | ✓ | `get_entity_context`, `get_entity_intelligence` |
| `with_entity_context_claim_reader` | 1784 | ✓ | same |
| `with_list_open_loops_reader` | 1792 | **✗** | `list_open_loops`, composed by `get_entity_intelligence` |
| `with_prepare_meeting_context_reader` | 1797 | ✓ | `prepare_meeting` |
| `with_daily_readiness_context_reader` | 1805 | ✗ (orphan) | unreferenced — confirm before removing |
| `with_trajectory_reader` | 1813 | ✓ | temporal abilities |
| `with_temporal_maintenance` | 1818 | ✓ | maintenance loop |
| `with_composition_commit_handle` | 1826 | ✓ | composition commits |
| `with_entity_touchpoints_reader` | 1834 | ✓ | touchpoints feed |
| `with_meeting_prep_status_reader` | 1842 | ✓ | meeting prep status |
| `with_claim_receipt_reader` | 1850 | ✓ | claim receipts |
| `with_account_list_reader` | 1858 | **✗** | `list_accounts` (producer at `abilities-runtime/src/abilities/list_accounts/mod.rs:219`) |
| `with_person_list_reader` | 1863 | **✗** | `list_people` |
| `with_project_list_reader` | 1868 | **✗** | `list_projects` |

### 2c — Error collapse + wire code (DOS-762 observability)

- `bridges/types.rs:541` — `validate_input_json_against_schema(...).map_err(|_| BridgeSurfaceError::AbilityUnavailable)?` — schema rejection → AbilityUnavailable.
- `bridges/types.rs:1003` — `AbilityInvokeError::Ability(_) => ... _ => BridgeSurfaceError::AbilityUnavailable` — any non-stale producer error → AbilityUnavailable.
- `bridges/types.rs:540` — `reject_reserved_input_fields(...)?` — reserved field → BridgeSurfaceError (variant).
- `surface_runtime/mod.rs:3400` — `bridge_surface_error: AbilityUnavailable | Ownership(_) => SurfaceHttpError::auth_missing()`.
- `surface_runtime/mod.rs:3649` — `auth_missing()` returns HTTP 401 with code `"auth_missing"` and message *"The requested DailyOS surface ability is not available."*

No `log::warn!` instrumentation on any of these collapse points. The wire code is the only signal.

### 2c-bis — v1.4.7 W1-A MCP v2 surface (DOS-168) — separability by trust layer

Branch `v1.4.7-w1-foundation` (PR #347, not merged to dev). Total ~3576 LOC across 7 files + 4 migrations. Mapped by the structural survey:

| File | Total LOC | KEEP (authorization + ops) | DROP (transport-ceremony) | Separability |
|------|-----------|-----------------------------|----------------------------|--------------|
| `services/mcp_v2/auth.rs` | 566 | ~252 (pair_client, load_client_record, resolve_tool_grant, revoke_*, exposure helpers) | ~310 (verify_transport_hmac:232–244, verify_and_consume_and_preissue:257–315, transport key persist/load:399–474) | Partial — keychain custody is isolated; nonce consume/preissue is coupled to fail-closed contract; HMAC verify is called from gateway.rs:281 |
| `services/mcp_v2/transport.rs` | 464 | ~370 (list_tools:157–188, build_input_schema, tool_from_description, unwrap_response, tool_error_to_mcp_error, tests) | ~70 (transport_key field:69, next_nonce field:71–72, call_tool nonce state machine:200–209+260, HMAC signing block:218–229) | Partial — nonce state machine interleaved with rmcp dispatch; envelope-shape stays, nonce population goes |
| `services/mcp_v2/gateway.rs` | 883 | ~820 (handler registration, dispatch flow:303–528, scope subset checks, rate-limit reserve:715–786, audit/signal emission) | ~65 (Gate 0a HMAC verify:278–284, Gate 0b nonce consume+preissue:286–297, TIMING_FLOOR_MILLIS:33 + sleep_until_floor:795–801, preissued nonce threading across error paths:291–302+320–353+437–438) | Partial — nonce preissue threading touches ~20 lines across 5 error paths; non-local edit |
| `services/mcp_v2/audit.rs` | 344 | ~144 (write API:50–78, sanitize_detail:111–124, add_actor_attribution:126–144, insert_outbox:181–220) | ~200 (hmac_json:146–150, canonical_value:152–168, audit key keychain custody:239–343, AUDIT_KEY_* constants:22–26) | Partial — keychain bits clean; hashing (lines 91–95) woven through `detail_with_hashes()` and all call sites; rip requires switching to plaintext emission OR field-level masking |
| `services/mcp_v2/actor_policy.rs` | 79 | 79 | 0 | Clean — entirely authorization |
| `services/mcp_v2/taxonomy.rs` | 714 | 714 | 0 | Clean — entirely tool registry |
| `services/mcp_v2/contracts.rs` | 851 | ~800 (ToolDescription, Scope, ScopedName, ParamSpec, ReturnSpec, OpaqueConversationHandle, McpClientId, McpActor, envelope shapes) | ~50 (OpaqueNonce type:223–241 — keep type for safety, drop usage; request_nonce/next_request_nonce fields on envelopes) | Clean (type-level) |
| `migrations/255_mcp_client_manifest.sql` | 19 | 19 | 0 | KEEP entirely (manifest table + transport_key_ref column kept as reference; key material is gone) |
| `migrations/256_mcp_conversation_handle.sql` | 11 | 11 | 0 | KEEP entirely |
| `migrations/257_mcp_transport_nonce_ledger.sql` | 17 | 0 | 17 | **DROP entirely** |
| `migrations/258_mcp_rate_limit_and_audit_outbox.sql` | 18 | 18 | 0 | KEEP (rate-limit ledger + audit outbox tables both load-bearing for ops layer) |

**Net rip estimate:** ~645 LOC removed, ~30 LOC modified (envelope shapes drop nonce fields), ~2900 LOC untouched. Migration 257 dropped before merge (replace with a no-op or renumber the rip-out into a new migration if 257 ships first).

**`services/mcp_v2/handlers/*` — transport-agnostic.** Handlers receive `(actor: &McpActor, params: Value) → Result<Value, ToolError>`. PR #355's `McpHandlerContext` adds `(ctx: &McpHandlerContext, actor: &McpActor, params: Value)`. Neither signature touches transport-ceremony types. Handlers don't change.

**PR #355 / DOS-758 status under this packet.** Unaffected. Lands independently. The `McpHandlerContext` interface + re-entrancy guard are correct under both the old and new trust models. The CI gate (`scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh`) stays.

### 2d — WP transport client

`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:214` — `signed_post()` builds the 16-header POST. `DailyOS_Hmac_Signer::sign_request` (lines 243–253) computes the signature. Marker storage in `wp_options`. Session credential cached server-side.

### 2e — Reuse audit (per `feedback_l0_reconcile_against_dev`)

Fork SHA: `852a0118`. `git diff 852a0118..dev -- src-tauri/src/services/context.rs src-tauri/src/surface_runtime/ src-tauri/src/audit_log.rs src-tauri/src/bridges/types.rs wp/dailyos/includes/transport/` returns **no changes** — dev has not advanced on any surface touched by this packet since the fork. Safe to land against `dev` after PR #351 merges. If `dev` advances before this packet's PR opens, re-run reconcile.

---

## §3 — Proposed direction

### 3a — Trust topology (DOS-761): first-party WP plugin as peer surface

**Premise.** The DailyOS WP plugin is first-party code (we ship it, we control it, we sign its releases). When it runs in the user's WP-Now/Studio environment as the same OS user as Tauri, and connects via `127.0.0.1`, it is functionally identical to the React Tauri renderer — same code provenance, same process owner, same kernel-protected loopback. It belongs in the same trust column as `tauri::invoke`. The third-party agent column (Claude Desktop, Cursor) is where the v1.4.7 W1-A ceremony belongs and stays.

**Mechanism.** Introduce a new HTTP route `POST /v1/local/invoke` on the existing runtime listener, bound to `127.0.0.1` only (already the bind). The handler:

1. Verifies `peer_addr().is_loopback()` (defense in depth — the bind already enforces this).
2. Materializes `Actor::User` directly (same as React `tauri::invoke`).
3. Deserializes `{ ability, input, projection_verification? }` (the canonical `SurfaceInvokeRequest` shape — same as the signed path).
4. Calls `TauriAbilityBridge::invoke` (the React path's entrypoint) with a new `BridgeSurface::LocalLoopback` variant (decision in §5.Q1; recommended new variant for audit clarity).
5. Returns the canonical envelope JSON.
6. Audit emission: emits with `Actor::User`, required `loopback_origin: "wp_plugin"` field (today's only consumer; enum extensible). No `wp_user_id` contract — there is no wp_user.

**Kept.** Connector OAuth (Slack/Gmail/etc external calls). Audit log (every invoke emits). Confirmation attestation for destructive ops (the existing `confirmation_args_hash` + `verify_confirmation_token` pathway — unchanged; runs inside `TauriAbilityBridge::invoke`).

**Removed for `/v1/local/invoke` path only.** HMAC verify, session validate, wp_user_id-per-request, SurfaceClient bridge authorize, per-axis rate-limit, audit-emit `wp_user_id` contract (because actor isn't SurfaceClient).

**Explicitly preserved — DO NOT touch in this packet:**
- `/v1/surface/invoke` (the existing signed-transport route) stays fully wired. v1.4.7's MCP v2 substrate (PR #347) uses it; DOS-647-C in-process MCP hosting will use it; if the WP-mediated MCP path (v1.4.2 W3-C) reaches into it, that stays too.
- `services/mcp_v2/*` — gateway, auth, audit, actor_policy, taxonomy, transport, contracts. None of the v1.4.7 substrate moves in this packet.
- `Actor::McpClient` and the `McpClientId`-bound scope manifest, nonce ledger, keyed-HMAC audit. Third-party agent ceremony is correct and stays.
- The pairing handshake itself (`/v1/pair/...` routes). One-shot, infrequent, signed. Unchanged.

**WP client.** `class-dailyos-runtime-client.php` gains a `local_post()` method targeting `/v1/local/invoke` with two headers: `Content-Type: application/json` + `X-DailyOS-Request-Id` (for traceability only, not auth). Pairing/marker stays for the connect-Tauri-to-WP UX flow (the WP plugin still needs to know which port Tauri is on — that's the existing sentinel/marker dance, unchanged).

**Per-call-site cutover decisions for `signed_post()`** — there are 5 callers, not 1 (WP-domain review finding):

| Call site | Cutover decision |
|-----------|------------------|
| `invoke_ability()` | route to `local_post()` |
| `project_composition_for_surface()` | route to `local_post()` |
| `issue_nonce()` | likely no-op under Actor::User; remove call site if confirmed unused |
| `verify_nonce()` | likely no-op under Actor::User; remove call site if confirmed unused |
| `refresh_pairing_scopes()` | likely no-op under Actor::User; pairing/scope concept stays but auto-refresh is dead code |

The pairing-handshake itself is already on a separate unsigned channel (`plain_post()` at runtime-client.php:71) — that path is unchanged.

**Refactor.** Factor out `discover_runtime_base_url()` (runtime-client.php:537) into a shared helper consumed by both `signed_post()` and `local_post()`. Both must preserve the `not_paired_error` `WP_Error` contract so block renders short-circuit cleanly when no pairing exists.

**Multisite invariant (load-bearing for future readers).** The current signed path sends `X-DailyOS-Multisite-Blog-Id` conditionally. DailyOS is single-tenant per-OS-user; the substrate never infers routing from this header. Dropping it on `local_post()` is safe. A `docs/solutions/` entry captures this invariant so future contributors don't reintroduce blog-id routing logic on the assumption that the substrate cares.

**Body-size limit (robustness, not security).** The `/v1/local/invoke` route must be added to the body-collection size gate at `surface_runtime/mod.rs:1025–1043` — without explicit inclusion it falls through to the `None` branch and parses zero bytes. Cheap correctness; not a defense.

**Out-of-threat-model — explicit rejections.** Per §1's threat model, `/v1/local/invoke` does NOT need a shared-secret header, a peer-pid check, codesign verification, or any other "is the caller really the WP plugin" gate. Same-user processes are inside the trust boundary; same-user-but-not-WP-plugin is not a threat actor in personal-tier. `loopback_origin` is descriptive metadata on the audit row (today's value: `wp_plugin`), not a security claim — server-side derivation is a cleanliness preference, not a defense.

**Cross-track guarantee.** This packet's diff touches zero files under `src-tauri/src/services/mcp_v2/` or `src-tauri/src/mcp_v2/`. CI gate `scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh` (introduced by DOS-758 / PR #355) is unaffected. v1.4.7 W1-A's wire-shape parity tests are unaffected.

### 3b — Substrate readers (DOS-762): wire the 4 missing

In `src-tauri/src/services/context.rs::attach_live_workspace_readers`, add:

- `with_list_open_loops_reader(Arc::new(LiveListOpenLoopsReader))`
- `with_account_list_reader(Arc::new(LiveAccountListReader))`
- `with_person_list_reader(Arc::new(LivePersonListReader))`
- `with_project_list_reader(Arc::new(LiveProjectListReader))`

Implement the 4 new `Live*Reader` structs as siblings to `LiveEntityContextReader` (same pattern: `Arc::new(LocalKeychain)` → `ActionDb::open` → call into existing `services::accounts::list`, `services::people::list`, etc — those service functions exist; the reader is just an adapter).

Verify `with_daily_readiness_context_reader` truly has no live consumer. If unused, leave un-attached and add a `// orphan: no producer composes against this` comment at the trait definition. If grep finds a consumer, attach it too.

### 3c — Observability: stop the `auth_missing` lie

In `bridges/types.rs`:

- `:541` — replace `.map_err(|_| BridgeSurfaceError::AbilityUnavailable)` with `.map_err(|err| { log::warn!(target: "abilities::dispatch", "schema validation failed for {ability_name}: {err}"); BridgeSurfaceError::InputSchemaInvalid })`. Add the new `BridgeSurfaceError::InputSchemaInvalid` variant.
- `:1003` — split the `AbilityInvokeError::Ability` arm: on `AbilityErrorKind::*` that indicate missing context (TBD enum check — likely a new variant `ProducerContextUnavailable`), map to `BridgeSurfaceError::ProducerUnavailable` and `log::warn!`. Keep `AbilityUnavailable` for genuine "ability not registered."
- `:540` — `reject_reserved_input_fields` adds `log::warn!` on rejection.

In `surface_runtime/mod.rs`:

- `:3400` — split `bridge_surface_error`: `AbilityUnavailable` → wire code `ability_not_registered` (HTTP 404), `InputSchemaInvalid` → wire code `input_schema_invalid` (HTTP 422), `ProducerUnavailable` → wire code `producer_unavailable` (HTTP 503), `Ownership(_)` → wire code `ownership_denied` (HTTP 403). Reserve `auth_missing` strictly for actual auth failures on the signed path.
- New wire codes added to the WP-side error renderer in `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php` so chips can render the producer reason (`stale`, `not_processed_yet`, `producer_unavailable`) vs an auth wall.

### 3d — Order of work (wave plan)

Three coordinated PRs against `dev`, sequenced to keep CI green at every step:

**PR A — Substrate readers + observability (DOS-762).** Lowest risk, immediate chip-rendering win.
1. Add 4 live readers + wire in `attach_live_workspace_readers`.
2. Split error wire codes + add `log::warn!` instrumentation.
3. L4: chips render producer reasons honestly even on signed path.

**PR B — First-party WP loopback (DOS-761).** Adds new route, cuts WP over.
4. Add `/v1/local/invoke` route + `Actor::User` materialization for loopback + new `BridgeSurface::LocalLoopback` variant.
5. WP `local_post()` method + cut over all invoke call sites in `class-dailyos-runtime-client.php`.
6. L4: bring-a-trailer chips render real envelopes; Tauri log shows zero `auth_missing` from WP path.

**PR C — MCP v2 substrate at the new shape (DOS-168 amended).** Coordinated with v1.4.7 W1-A.
7. Hold PR #347 merge OR rebase PR #347 over PR C.
8. Apply the §3e rip: delete migration 257, delete ~310 LOC from auth.rs, ~70 LOC from transport.rs, ~65 LOC from gateway.rs, ~200 LOC from audit.rs. Modify ~30 LOC across envelope shapes.
9. PR #355 (DOS-758 handler context) lands independently and unaffected.

**Total estimate:** PR A = 3–4h. PR B = 4–6h. PR C = 8–12h (the rip is mostly mechanical but L0 already passed on the wrong premise, so re-review is structural). Plus L2 + L4 on each.

### 3e — Third-party MCP transport-ceremony rip (new ticket)

**Premise.** v1.4.7 W1-A's transport-ceremony layer defends remote-network threats against a local-only transport. Local stdio (today's MCP transport) has no in-flight surface. In-process hosting (DOS-647-C) makes "transport" a Rust function call. The layer is dead weight.

**What gets ripped (with file:line precision from §2c-bis):**

**Migration drops:**
- `src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql` — entire schema gone. If 257 ships in dev before PR C lands, replace with a `DROP TABLE IF EXISTS mcp_transport_nonce_ledger;` rip-out migration at the next slot.

**Code drops (~645 LOC):**
- `services/mcp_v2/auth.rs:232–244` — `verify_transport_hmac()`.
- `services/mcp_v2/auth.rs:257–315` — `verify_and_consume_and_preissue()` fail-closed nonce ledger.
- `services/mcp_v2/auth.rs:399–474` — transport key Keychain custody (`persist_transport_key`, `load_transport_key`, Zeroize wrappers).
- `services/mcp_v2/auth.rs:32–51` — error variants `InvalidSignature`, `NonceReplayed`, `PreissueFailed`, `Keychain`.
- `services/mcp_v2/transport.rs:69` — `transport_key: Arc<Zeroizing<[u8; 32]>>` field.
- `services/mcp_v2/transport.rs:71–72` — `next_nonce: Arc<Mutex<Option<OpaqueNonce>>>` field.
- `services/mcp_v2/transport.rs:200–209,260` — nonce pull-from-state and store-next-nonce.
- `services/mcp_v2/transport.rs:218–229` — HMAC envelope signing block.
- `services/mcp_v2/gateway.rs:33` — `TIMING_FLOOR_MILLIS` const.
- `services/mcp_v2/gateway.rs:278–284` — Gate 0a HMAC verify call.
- `services/mcp_v2/gateway.rs:286–297` — Gate 0b nonce consume+preissue call.
- `services/mcp_v2/gateway.rs:795–801` — `sleep_until_floor()` timing floor.
- `services/mcp_v2/audit.rs:22–26` — `AUDIT_KEY_*` constants.
- `services/mcp_v2/audit.rs:146–150` — `hmac_json()`.
- `services/mcp_v2/audit.rs:152–168` — `canonical_value()` (canonical JSON for HMAC).
- `services/mcp_v2/audit.rs:239–343` — audit key Keychain custody (`load_or_create_audit_key`, `read_audit_key`, `persist_audit_key`, etc).

**Code modifications (~30 LOC):**
- `services/mcp_v2/contracts.rs:223–241` — keep `OpaqueNonce` type, remove all usage. (Or delete the type and accept the type-safety loss; recommend keep-the-type in case it earns its way back.)
- `services/mcp_v2/contracts.rs` — `McpToolRequestEnvelope.request_nonce` field becomes `Option<OpaqueNonce>` or removed. `McpToolResponseEnvelope.next_request_nonce` similarly.
- `services/mcp_v2/audit.rs:80–100` — `detail_with_hashes()` becomes `detail_with_attribution()`. Emit params/response **plaintext** in the detail (no HMAC hashing) OR switch to field-level masking via the existing `sanitize_detail()` (lines 111–124) — decision in §5.Q9.
- `services/mcp_v2/gateway.rs:291–302,320–353,437–438` — unthread `preissued` from rejection-response building. Response envelope no longer carries `next_request_nonce`.
- `services/mcp_v2/migrations/255_mcp_client_manifest.sql` — keep table; `transport_key_ref` column either nulls out (kept for schema stability) or drops in a sibling migration.

**Kept (load-bearing authorization + ops):**
- Entire authorization machinery: pair_client, manifest load, scope resolution, exposure-tier check, conversation handle lifecycle, scope-subset enforcement, rate-limit reserve, plain audit emission, audit outbox, `actor_policy.rs`, `taxonomy.rs`, `contracts.rs` type ecosystem (minus nonce/HMAC bits).
- `Actor::McpClient` as a distinct actor class — third-party agents need scope-distinct attribution from `Actor::User`, even if they don't sign envelopes.
- DOS-624 sensitivity gating amendment (briefing exposure) — independent of transport, composes downstream.
- PR #355's `McpHandlerContext` interface + CI gate.
- **Opaque resource IDs** (carried forward from v1.4.7 cycle 2 CSO MED #4 — `.docs/plans/v1.4.7-waves.md:54`): `dailyos://account/{id}` `id` is a server-minted opaque handle, not the underlying entity_id. This is identifier-design clarity (don't leak internal entity_ids to client conversation context), not threat-model defense. Stays in PR C scope.

**Out-of-threat-model — explicit rejections (mirror of §3a).** Per §1's threat model, dropping HMAC envelope signing for stdio MCP does NOT require a per-client shared secret or any cryptographic identity binding. `McpClientId` is for SCOPE attribution and AUDIT — "this invoke came from Claude Desktop, scoped to read.account_status" — not for proof-of-identity against same-user impersonation. If a same-user process spoofs a `McpClientId`, the WORST case is they get scopes the user already granted to that client. The user is inside the trust boundary by definition; granting Claude Desktop a scope means that scope is reachable by any same-user process the user has installed. That's the personal-tier model, and it's the correct one for this product.

**New invariant.** Audit log row format becomes: `{ actor: McpClient(client_id), conversation_handle, ability, params: <plaintext or masked>, response: <plaintext or masked>, request_id, timestamp }`. No HMAC hashes. Tamper-evidence is delegated to the OS — the SQLCipher DB the user controls is the audit substrate.

**What CSO must re-pass on.** §6.5 reviewer prompt is rewritten to bound the CSO question precisely: "the personal-tier trust model puts the user in control of their own DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with SQLCipher + Keychain DB encryption, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't?"

---

## §4 — Acceptance criteria

### DOS-762 (readers + observability)
- [ ] `attach_live_workspace_readers` attaches `list_open_loops_reader`, `account_list_reader`, `person_list_reader`, `project_list_reader` with live SQLite adapters.
- [ ] `cargo run --bin emit_ability_inventory` regenerates clean; CI ability-inventory gate green.
- [ ] Direct curl to `list_accounts` returns populated envelope with non-empty `accounts` array.
- [ ] Direct curl to `get_entity_intelligence` for a real entity returns populated envelope with non-empty `sections`.
- [ ] `log::warn!` fires on schema rejection, reserved-field rejection, producer context unavailable — with ability name + reason. Verified by `RUST_LOG=warn` tail during a forced-fail invoke.
- [ ] Wire codes split: HTTP responses for the 4 distinct rejection classes return the new codes (`ability_not_registered`, `input_schema_invalid`, `producer_unavailable`, `ownership_denied`). `auth_missing` returns ONLY on genuine signed-path auth failures.

### DOS-761 (trust topology)
- [ ] `POST /v1/local/invoke` route registered; binds to loopback only; rejects non-loopback peers with HTTP 404 (404, not 401 — pretends not to exist for non-loopback callers).
- [ ] Handler materializes `Actor::User` and calls `TauriAbilityBridge::invoke` with the canonical path. No HMAC, session, scope, or wp_user_id validation in the loopback handler.
- [ ] Audit log emits an entry per loopback invoke with `Actor::User` and optional `loopback_origin` for traceability. No SurfaceClient/wp_user_id contract.
- [ ] WP `class-dailyos-runtime-client.php` adds `local_post()` and routes all ability-invoke call sites through it. `signed_post()` retained for pairing handshake only.
- [ ] WP marker/sentinel flow unchanged (Tauri port discovery still works).
- [ ] L4: bring-a-trailer hard-refresh → 24 chapters render real envelope data; `not_available` and `no_envelope` chips replaced by either real content or honest producer reasons (`stale`, `not_processed_yet`, `no_relevant_touchpoints`).
- [ ] Tauri log tail shows zero `auth_missing` on the loopback path; if a producer fails, the wire code names the actual cause.

### DOS-168 amended (MCP v2 substrate at the new shape — see §3e)
- [ ] Migration 257 dropped (or rip-out migration filed at next slot if 257 ships first).
- [ ] `services/mcp_v2/auth.rs` shrinks by ~310 LOC: HMAC verify, nonce ledger, transport key custody gone. Manifest load + scope resolution + handle lifecycle intact.
- [ ] `services/mcp_v2/transport.rs` shrinks by ~70 LOC: nonce state machine + HMAC signing gone. `list_tools`, `tool_error_to_mcp_error`, dispatch flow intact.
- [ ] `services/mcp_v2/gateway.rs` shrinks by ~65 LOC: Gate 0a, Gate 0b, timing floor, preissue threading gone. Dispatch + scope subset + rate-limit + audit emission intact.
- [ ] `services/mcp_v2/audit.rs` shrinks by ~200 LOC: HMAC hashing + audit key custody gone. Emission + outbox + actor attribution intact. Detail emits plaintext-in-encrypted-DB OR field-masked (per §5.Q9 decision).
- [ ] `Actor::McpClient` retained; scope manifest enforcement intact end-to-end.
- [ ] Local MCP integration smoke test: a stdio MCP client (real or mock) connects, sees `tools/list` filtered by manifest grants, invokes a read tool, receives result. No signing, no nonces.
- [ ] `services/mcp_v2/handlers/*` unchanged; PR #355 lands cleanly.
- [ ] `cargo test --workspace --all-features --lib services::mcp_v2` green; `cargo test --test dos168_mcp_v2_migration_smoke_test` updated for the migration-257 drop.
- [ ] DOS-624 sensitivity gating still composes correctly downstream of `Actor::McpClient` materialization.
- [ ] **PR C description includes a handler-to-masked-fields table** — every `Side::Write` and `Side::SubmitCorrection` tool in `services/mcp_v2/handlers/*`. Each entry names: handler file, declared `Side`, params field list, which fields hit `PARAM_PAYLOAD_KEYS` mask. L2 verifies completeness before merge. (Audit hygiene per CSO finding; not threat-model defense.)
- [ ] Opaque resource IDs preserved: `dailyos://account/{id}` server-minted handle, not raw entity_id. Carried forward from v1.4.7 cycle 2 CSO MED #4.

### Joint
- [ ] `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green across all three PRs.
- [ ] L2 unanimous APPROVE on each PR (codex challenge + codex consult + architect-reviewer + wp-skill-grounded reviewer + CSO + v1.4.7-cross-track reviewer).
- [ ] PR descriptions name relevant tickets; commit messages carry `L2-status: passed`.
- [ ] PR #347 merge held until PR C reshapes the substrate; or PR #347 author rebases over PR C; coordination decision in §5.Q8.

---

## §5 — Open decisions for L0 (the questions that shape the PR)

**Q1.** New `BridgeSurface::LocalLoopback` variant, or reuse `BridgeSurface::TauriApp`?
- *Pro new:* keeps audit/traceability distinct so we can tell "React in-process invoke" from "WP-plugin first-party-loopback invoke" without parsing the actor.
- *Pro reuse:* fewer surface enum cases; the trust model is genuinely identical between the two.
- **Recommend:** new variant. Audit clarity is cheap; the WP plugin is a distinct first-party surface even if it shares the trust model with React.

**Q2.** Touch the signed-transport invoke route at all?
- **Hard no.** v1.4.7 W1-A (PR #347) uses it. DOS-758 / PR #355 builds on it. DOS-647-C will land more handlers into it. WP-mediated MCP (v1.4.2 W3-C) may reach into it. Stripping or deprecating it now would burn 6 cycles of L0 review and block all three open MCP tracks.
- **Recommend:** `/v1/surface/invoke` is fully load-bearing for third-party agents and stays. This packet adds `/v1/local/invoke` as a new sibling route; it does not modify, deprecate, or annotate the existing route.

**Q3.** Wire the 4 new readers all at once, or batched? (DOS-762 split into sub-tickets?)
- All 4 live SQLite adapters are the same shape (10-15 lines each). Splitting buys nothing.
- **Recommend:** all 4 in one commit, one set of tests.

**Q4.** `daily_readiness_context_reader` — attach or leave orphan?
- Need a grep audit to confirm no producer composes against it.
- **Recommend:** the L1 author audits in commit 1 and either attaches (with live adapter) or files a Linear comment confirming orphan + adds explanatory comment at trait definition.

**Q5.** Per-axis rate-limit replacement for loopback?
- The signed-path's per-ability budget defends against runaway third-party agents.
- First-party WP plugin as Actor::User has no such defense. Worth it?
- *Position:* no rate-limit. If WP plugin runs away, that's a WP plugin bug to fix at the source, not paper over with a rate-limit on our own code. Parity with React `tauri::invoke` (also has no per-axis limit) is the point.
- **Decided now:** no rate-limit on `/v1/local/invoke`. To keep runaway loops observable (not deferred), §3e's per-invoke audit emission + the §3c `log::warn!` instrumentation give a clear signal: if the same `(actor, ability)` pair fires N+ times in a window, the audit log shows it immediately. If a viewer surface is needed to surface that signal, it's a viewer ticket, not a rate-limit ticket.

**Q6.** Should `Actor::User` materialization from `/v1/local/invoke` distinguish WP-plugin origin from a (future) in-process MCP host?
- DOS-647-C is in-process MCP hosting — moving handlers from the standalone binary into the signed-route server. It still mints `Actor::McpClient` for the requesting agent, not `Actor::User` — the in-process move is a deployment change, not a trust-model change.
- So `/v1/local/invoke` is exclusively for first-party surfaces (today: WP plugin; tomorrow: anything else we ship that's not a renderer of `tauri::invoke`).
- **Recommend:** `loopback_origin` enum is restrictive — only first-party surface kinds. Add a CI lint that rejects new `loopback_origin` variants without a CSO sign-off comment in the PR body. Today's enum: `{ wp_plugin }`. MCP hosts do NOT use this route.

**Q7.** Coordination with DOS-624 (v1.4.7 W2-A CSO amendment for `dailyos.read.daily_briefing` MCP exposure)?
- DOS-624 reasons about which intelligence the third-party MCP path can expose. It does not constrain the first-party WP path because the WP path already renders the intelligence directly (it's the WP plugin's own UI).
- *Risk:* if DOS-624 amends the briefing intelligence sensitivity gate AFTER this packet lands, the first-party WP path could continue rendering briefings that have been restricted from third-party MCP. That's the intended behavior — first-party gets the un-redacted view; third-party gets the gated view. Worth naming as an architectural invariant.
- **Recommend:** add an invariant note in §3a: *"sensitivity gating composes downstream of actor materialization. First-party `Actor::User` and third-party `Actor::McpClient` may see different projections of the same intelligence by design."*

**Q8.** PR #347 coordination — what happens to the existing open PR?
- *Hold + rebase:* PR #347 author cherry-picks the KEEP set (~2900 LOC of authorization machinery), rebases over PR C's drops. Preserves PR review history; 6 cycles of L0 reviewer context stays attached. But: the 6 cycles validated the wrong premise; preserving them carries assumption-baggage forward.
- *Close + supersede:* close PR #347 as superseded by PR C. New L2 cycle on the simplified substrate. Loses review history but the history was internally-coherent-on-wrong-premise.
- **Decided now:** **close + supersede**. PR C lands the substrate at the new shape directly (authorization machinery + dispatch + handlers + migrations 255/256/258 + taxonomy + actor_policy + contracts) — it IS the substrate, not an amendment to it. PR #347 closes with a comment pointing at PR C. The v1.4.7 wave plan amendment lands as part of PR C's diff (`.docs/plans/v1.4.7-waves.md` updated in same PR). Nothing follows-up; nothing is parked. DOS-168 description + AC update at PR-open time.

**Q9.** Audit detail emission — plaintext or field-masked?
- `sanitize_detail()` already exists at audit.rs:111–124. The mechanism is built; the question is whether to use it.
- *Read-class abilities:* params are entity IDs, depths, query strings. Plaintext-in-encrypted-DB. The user is the only auditor; SQLCipher + Keychain gates the file.
- *Write-class / submit-class abilities:* params may carry user-typed content (note text, claim corrections, document body). Mask via `sanitize_detail()` to drop named payload fields. Preserves audit attribution without re-rendering user content into a separate log substrate.
- **Decided now:** PR C ships both per ability `Side` (`Read` / `Write` / `SubmitCorrection` from contracts.rs:18–23). Read = plaintext detail. Write/SubmitCorrection = `sanitize_detail()`-masked. Wired in `services/mcp_v2/audit.rs::detail_with_attribution` based on the ability's declared Side. CSO confirms in §6.5 that the existing masking set in `PARAM_PAYLOAD_KEYS` / `RESPONSE_PAYLOAD_KEYS` is the right list for write-class.

**Q10.** Where does PR C live as a ticket?
- **Decided now:** amend DOS-168 directly. DOS-168 IS the v1.4.7 W1-A MCP v2 gateway substrate ticket; it has not merged. Updating its description + acceptance criteria to reflect the simplified substrate (drop transport-ceremony, keep authorization) is the honest move. PR C closes DOS-168 at the new shape. No new umbrella. DOS-168 description + AC update lands at PR C open time. PR #347 closes superseded-by-PR-C per §5.Q8.
- Per `feedback_no_deferrals_period` and `feedback_path_alpha_as_sneaky_deferral`: this is not a follow-up, not an umbrella, not a maintenance bucket. PR C is the work and DOS-168 is the spec being amended to match it.

---

## §6 — L0 review verdict (what actually ran)

Per CLAUDE.md L0 minimum for trust-boundary work: codex challenge + CSO + 1 domain reviewer. The §6.original prompt set (7 panels) was over-engineered for the scope; trimmed to 3 actually dispatched. Verdicts on file in `reviews/`:

| Reviewer | Verdict | Dispositions |
|----------|---------|--------------|
| **codex challenge** (`reviews/codex-challenge.md`) | REQUEST_CHANGES | HIGH-A (`/v1/local/invoke` unauthenticated): **rejected** — same-user is inside trust boundary per §1. HIGH-B (MCP scope elevation without HMAC): **rejected** — same. MED-C (compromised WP plugin blast-radius): **rejected** — first-party, same trust as Tauri. MED-D (audit content tamper): **rejected** — SQLCipher protects content. LOW-E (opaque resource IDs carry-forward): **folded** into §3e KEEP set. |
| **CSO** (`reviews/cso.md`, ce-security-lens-reviewer) | APPROVE w/2 conditions | MED-1 (loopback_origin server-side): **rejected as security**, server-derive kept as cleanliness preference (§3a). LOW-2 (body-size limit): **folded** as robustness (§3a). PR C masking-table: **folded** as audit hygiene (§4). Q4 confirmed: PR #347's 6-cycle L0 was internally-validated only, no specific prior-cycle threat the new model breaks. |
| **wp-skill-grounded** (`reviews/wp-skill-grounded.md`, general-purpose with wp-* skills) | APPROVE w/2 MED | HIGH-1 (5 callers of signed_post): **folded** as per-call-site decision table in §3a. HIGH-2 (URL helper factoring): **folded** in §3a. MED-1 (multisite invariant doc): **folded** in §3a + `docs/solutions/` entry. LOW-3 (WP-Now sandbox naming): **folded** in §8. |

**Synthesis.** Codex challenge dissented (REQUEST_CHANGES vs APPROVE-with-conditions). Per `feedback_reviewer_dissent_is_signal`, the dissent was investigated — the codex findings all posit threat actors (same-user-but-not-installed-app) explicitly excluded from the personal-tier threat model. Rejection is principled, not dismissive. The §1 threat-model framing is now load-bearing for future reviewers — if the same finding shape recurs in L2/L4, the disposition is reject-as-out-of-threat-model.

**No second L0 cycle.** The folded findings are tightenings of implementation correctness (per-call-site table, URL helper, body-size, multisite invariant, masking table, opaque resource IDs) — none are premise changes. Move to L1.

## §6.original — Reviewer prompts (preserved for record)

The original §6 contemplated 7 reviewers (codex challenge + codex consult + architect + design-lens + CSO + wp-skill + v1.4.7-cross-track). 4 were skipped at dispatch time as over-engineering for the scope. Preserved here so future similar packets can lift prompts.

Per the wave reviewer matrix + memory (`feedback_wp_skill_grounded_reviewer_for_wp_l0`) + CSO required (trust-boundary work, Amendment 3):

1. **codex challenge** — adversarial: "find a same-OS-user threat that the gate stack defends and the loopback model doesn't. If you find one, propose the cheapest non-gate mitigation (process-level, audit-level, OS-level). Don't argue for keeping the gates abstractly."
2. **codex consult** — design review: "is `/v1/local/invoke` the right shape, or should loopback dispatch happen via an in-process channel (mpsc/dashmap) with no HTTP at all? Trade-off the two for MCP-readiness."
3. **architect-reviewer** — integrated state: "does this packet hold against W3-A.5/A.6 work scheduled later in v1.4.4? Any wiring this packet locks in that the workspace memory or salience waves would need to re-open?"
4. **plan-design-review** — surface integrity: "chip rendering currently fails opaque; this packet ships honest producer-reason chips. Does the empty-chip CSS pattern + Magazine shell language degrade gracefully for `producer_unavailable` / `stale` / `not_processed_yet`?"
5. **CSO** — trust-boundary scrutiny on the full packet: "(a) for first-party WP loopback: any third process running as the same OS user that could intercept the loopback port, confused-deputy risk if Tauri's own renderer is compromised, audit-log integrity if `loopback_origin` is client-supplied; (b) **for MCP transport-ceremony rip:** the personal-tier trust model puts the user in control of their own SQLCipher+Keychain DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with DB encryption + OS-user file ownership, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't? Same for nonce ledger — what local-stdio replay attack is the nonce defending? (c) confused-deputy / scope-elevation: if a third-party MCP client (Claude Desktop) is compromised by a hostile MCP server, does the scope manifest still hold without HMAC? (d) PR #347's 6-cycle L0 sign-off on the keyed-HMAC/nonce/timing-floor model — was it premise-validated or only internally-validated? If premise-validated, surface the threat model that changed."
6. **wp-skill-grounded reviewer** (`/codex consult` with skill context) — "WP plugin transport: does the `signed_post` → `local_post` cutover correctly preserve the pairing handshake path? Any WP-CLI / activation hook / multisite consideration that breaks if the plugin's invoke stops carrying identity headers?"
7. **v1.4.7-cross-track reviewer** (`/codex consult` pointed at `.docs/plans/v1.4.7-waves.md` + PR #347 + PR #355 + DOS-647-C ticket) — "this packet **reshapes** the v1.4.7 W1-A substrate, not just preserves it. Audit: (a) the §3e KEEP/DROP split in §2c-bis — anything mis-categorized? Specifically the conversation handle lifecycle (auth.rs:321–356), is it really transport-independent or does it depend on nonce-binding? (b) DOS-647-C's in-process MCP hosting plan — does dropping HMAC/nonce simplify or complicate the in-process move? (c) the §3d hybrid coordination (PR C ships substrate at new shape; PR #347 becomes docs-only amendment) — is this the right cycle hygiene or should PR #347 be closed-and-rebased? (d) PR #355 `McpHandlerContext` lands independently — confirm no transport-ceremony dependency. (e) DOS-624 sensitivity gating composes downstream of actor materialization — confirm intact under rip."

**K-in obligation per CLAUDE.md.** Before scoring, every reviewer greps `docs/solutions/` and `.docs/decisions/` for prior trust-topology or reader-wiring decisions. ADR-0083 (product vocab) and any ADR in the 01xx range touching surface auth must be read.

---

## §7 — Memories load-first

- `feedback_local_to_local_security_overreach_primary_concern` — primary concern; flagged here in §1.
- `feedback_l4_debugging_can_hide_overengineering` — 4h of `auth_missing` debugging was the trigger.
- `project_auth_overhaul_strip_hmac_for_local_same_user` — current state of this work; this packet IS the overhaul, narrowly scoped to first-party WP plugin loopback.
- `feedback_wire_existing_substrate_not_future_producer` — wiring the readers IS the work; v1.4.3 had this working.
- `feedback_dont_swing_past_center_when_correcting` — preserve v1.4.7 MCP v2 substrate verbatim; only reclassify first-party WP plugin.
- `feedback_ci_gate_inputs_are_L1_deliverables_not_verification_steps` — observability instrumentation ships in this PR, not in a follow-up.
- `feedback_l0_reconcile_against_dev` — reconciled at §2e.
- `feedback_wp_skill_grounded_reviewer_for_wp_l0` — wp-skill panel added at §6.6.
- `project_v142_wordpress_spike` — WordPress as primary surface; first-party trust column is the architectural commitment.

---

## §8 — What this packet does NOT do (hard scope guard)

**Authorization machinery — diff against KEEP set must be zero except for envelope nonce-field removal:**
- `services/mcp_v2/actor_policy.rs` — KEEP entirely.
- `services/mcp_v2/taxonomy.rs` — KEEP entirely.
- `services/mcp_v2/handlers/*` — KEEP entirely (transport-agnostic).
- `services/mcp_v2/auth.rs` KEEP set (~252 LOC of pairing + manifest + grant resolution + revocation + exposure helpers).
- `services/mcp_v2/gateway.rs` KEEP set (~820 LOC of dispatch + scope subset + rate-limit + audit emission + signal emission).
- `services/mcp_v2/transport.rs` KEEP set (`list_tools`, dispatch, error mapping, tests).
- `services/mcp_v2/audit.rs` KEEP set (write API, sanitize_detail, attribution, outbox).
- `services/mcp_v2/contracts.rs` — KEEP all types except remove nonce fields from envelopes.
- Migrations 255, 256, 258 — KEEP entirely.
- `Actor::McpClient`, `McpClientId`, `OpaqueConversationHandle`, `ToolGrant`, `ClientRecord` — KEEP all types.
- `/v1/surface/invoke` signed-transport route handler — KEEP (still serves the pairing handshake at minimum; long-term future-remote consumer; coordinate with §5.Q2).
- `/v1/pair/*` pairing handshake routes — KEEP entirely.
- `scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh` and hookups — KEEP entirely (PR #355's gate is correct under both models).
- v1.4.2 W3-C WP-mediated MCP path — KEEP entirely.
- Connector OAuth (Slack/Gmail external HTTP) — KEEP entirely.
- DOS-624 CSO sensitivity gating — KEEP entirely (composes downstream of actor materialization).

**Explicitly does NOT do:**
- Does NOT implement DOS-647-C in-process MCP hosting (the rip simplifies the path; the move itself stays in v1.4.7).
- Does NOT change handler signatures beyond what PR #355 already does.
- Does NOT touch ability registry or producer code (orthogonal).
- Substrate-stability / writer-mutex / Glean-validation issues from the post-merge handoff §5 are real and orthogonal — they're DB-layer issues, this packet is auth-layer. They need their own L0 packet (corruption recovery cadence, writer-mutex fairness queueing, Glean validation moving outside the writer transaction). If no Linear ticket exists by PR A merge, file one and link from PR A description — don't let "separate" become "deferred."
- Inner-block envelope-shape mapping (Open Thread #2 in handoff) is one L4 verification cycle, not a separate workstream — runs as part of PR A's L4 since the readers + observability are what surface the shape-gap symptoms. If shape gaps exist, fix in PR A.
- Remote-MCP transport: not built. If a remote consumer arrives later, gate-stack-revival is the right move at that point. "Don't pre-build for a threat that hasn't materialized" is different from "defer work that needs doing" — there is no caller, so there is no work.
- WP-Now sandbox limitations are pre-existing and orthogonal. Studio works today; WP-Now's wasm sandbox (where `getenv('HOME')` returns `/home/web_user` and `posix_*` is unavailable) cannot reach `~/.dailyos/runtime-endpoint.json` directly — the existing marker-URL fallback in `wp_options` handles this and stays unchanged. If L4 surfaces an environment-specific issue, it's orthogonal to this packet's trust-model work.

**Coordination commitment.**
- PR A (DOS-762 readers + observability) can ship first independently.
- PR B (DOS-761 WP loopback) ships independently of PR C.
- PR C (DOS-NNN MCP rip) coordinates with v1.4.7 W1-A per §5.Q8 hybrid: PR C ships substrate at the new shape, PR #347 becomes a docs-only amendment to `.docs/plans/v1.4.7-waves.md`.
- DOS-758 (PR #355) is **unaffected** and can land at any time; the `McpHandlerContext` interface is correct under both models.
- DOS-624 CSO sensitivity gating amendment is **unaffected** and can land at any time; it composes downstream of actor materialization.
