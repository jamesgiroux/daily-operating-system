# CSO Review — Local Trust-Model Unification
**Packet:** L0-packet.md — DOS-761 + DOS-762 + DOS-168 amended  
**Reviewer:** CSO (security-lens)  
**Date:** 2026-05-22  
**Verdict:** APPROVE with two conditions (see end)

---

## Evaluation basis

The packet's core premise — that transport-ceremony gates (HMAC signing, nonce ledger, keyed-HMAC audit, timing floor) defend remote-network threats and are inapplicable to local-stdio and kernel-protected loopback transports — is **correct**. The threat-model reframe is sound for personal-tier with SQLCipher + Keychain DB encryption and OS-user file ownership. What follows addresses the five specific questions in §6.5.

---

## Q1 — WP loopback specifics

**Finding 1 — `loopback_origin` is client-supplied with no server-side validation: MED (confidence 75)**

The packet's AC specifies that audit rows carry `loopback_origin: "wp_plugin"` (L0-packet.md §3a, §4 DOS-761 AC). The mechanism as described emits the `loopback_origin` value from the request body into the audit log without validation against a server-side allowlist. Any process running as the same OS user can POST to `/v1/local/invoke` and supply an arbitrary `loopback_origin` string — e.g., `"legitimate_surface"` — causing the audit log to misattribute the caller.

The packet acknowledges this partially in §5.Q6 ("CI lint that rejects new `loopback_origin` variants without a CSO sign-off comment") but the lint gate is for _new variant additions at code time_, not for _runtime spoofing of the existing enum_. The audit value should be produced server-side, not accepted from the body.

**Required fix (condition 1 below):** The `/v1/local/invoke` handler must derive `loopback_origin` from a server-side-only source. Two options: (a) hard-code `loopback_origin = LoopbackOrigin::WpPlugin` at the route handler level with no field accepted from the body, or (b) require a static header `X-DailyOS-Surface-Kind: wp_plugin` and validate it against the server-side enum at parse time, rejecting unknown values with HTTP 400 before reaching any audit emit path. Either is sufficient. The body must not carry a field that flows into `loopback_origin`.

**Finding 2 — No body-size limit on `/v1/local/invoke`: LOW (confidence 75)**

`handle_hyper_request` at `surface_runtime/mod.rs:1025–1033` gates body collection on `is_signed_route_candidate()` and the explicit pairing/session-refresh paths. The new `/v1/local/invoke` route is none of these; it falls through to `body_limit = None` at line 1041, which results in `Bytes::new()` (no body read at all). This is the current implementation behavior — the body limit gating will need to cover the new route explicitly or the route will receive zero bytes and fail to parse the invoke request. File:line: `surface_runtime/mod.rs:1025–1043`.

This is a correctness issue that surfaces as a security issue if not gated: without a body limit, a same-OS-user process could POST arbitrarily large payloads. Functionally it needs to be gated to `runtime.signed_request_max_body_bytes` or a new per-route constant.

**Finding 3 — Confused-deputy: Tauri renderer compromise: LOW (confidence 50)**

If the Tauri renderer process itself is compromised (malicious WebView content bypasses the JS-level boundary), it already has `tauri::invoke` access as `Actor::User`. The new `/v1/local/invoke` route provides no additional attack surface because the adversary already holds the higher-privilege path. This is not a meaningful residual threat the loopback route adds — the packet's conclusion on this is correct.

**Finding 4 — Port discovery and loopback interception: LOW (confidence 50)**

The runtime sentinel file at `~/.dailyos/runtime-endpoint.json` is readable by any same-OS-user process. Any same-OS-user process can learn the port and POST to `/v1/local/invoke`. The packet's position is that "same OS user = trusted" is the correct personal-tier boundary, consistent with `tauri::invoke`. This is sound — the threat model is one user, one machine. The only residual concern is a rogue same-OS-user process (malware running as the user) which is an OS-level threat outside the application's scope.

---

## Q2 — MCP transport-ceremony rip: residual threats

**Keyed-HMAC audit:** The keyed-HMAC audit (services/mcp_v2/audit.rs — currently a stub, 7 LOC, implementation pending) was designed to prevent a compromised audit log from being tampered with post-write. In personal-tier with SQLCipher protecting the DB at rest and OS-user file ownership controlling who can open the file, the threat it defends — an attacker reading and silently modifying audit rows while preserving plausibility — requires the attacker to have already broken either (a) SQLCipher + Keychain or (b) OS-user file access. If either is broken, the HMAC key stored in Keychain is equally compromised. The residual threat does not survive. Plaintext-in-encrypted-DB is the correct posture.

**Nonce ledger (migration 257):** The nonce ledger defended replay of a captured stdio message. Local stdio transport is OS-pipe — attacker requires the same OS user to read the pipe, at which point they have `tauri::invoke` access anyway. In-process hosting (DOS-647-C) further collapses the transport to a Rust function call. The nonce ledger defends a threat that does not exist on local stdio. Dropping migration 257 is correct.

**10ms timing floor (gateway.rs:33, 795–801):** The timing floor defended a side-channel where response latency disclosed whether a scope-check hit or missed the manifest. That side-channel is meaningful against a remote HTTP attacker who can make thousands of probe requests. Against a same-machine process that already shares the OS user boundary, the attacker has direct file access and the side-channel is superfluous. The timing floor is dead weight.

---

## Q3 — Confused-deputy / scope-elevation without HMAC envelope signing

The scope manifest is loaded server-side from the DB by `McpClientId` (auth.rs — stub, references `pair_client`, `load_client_record`, `resolve_tool_grant`). The HMAC envelope signing was authenticating the _transport_ (the pipe bytes), not the _manifest load_. The manifest is a DB-resident, server-authoritative record keyed to `McpClientId`; it does not depend on the HMAC for integrity.

If Claude Desktop is compromised by a hostile MCP server that injects tool calls:
- The injected calls still carry the legitimate `McpClientId` (the pairing-issued stable identifier).
- The gateway resolves the manifest for that `McpClientId` from the server-side DB.
- The scope subset check in gateway.rs (KEEP set, ~820 LOC) enforces which tools are reachable.
- The conversation handle lifecycle (auth.rs:321–356 per the packet's KEEP categorization) is server-minted — the client cannot forge a handle it was not issued.

Removing HMAC signing does not weaken the scope manifest because the manifest was never protected _by_ the HMAC — it was protected by server-side DB custody with `McpClientId` as the key. A compromised Claude Desktop can invoke any tool within the _granted scope_ of the legitimate pairing, which is the intended and bounded behavior.

The conversation-handle binding question raised by the v1.4.7 cross-track reviewer (§6.7c) is the one point requiring verification: if `OpaqueConversationHandle` minting is currently coupled to nonce state in auth.rs:321–356, the handle lifecycle must be decoupled from the nonce before the rip. The packet asserts this as transport-independent; L1 must confirm by grepping the handle mint path in auth.rs for nonce dependencies.

---

## Q4 — PR #347 6-cycle L0 premise-validation

The 6-cycle L0 was internally-validated, not premise-validated. The reviewers evaluated whether the HMAC/nonce/timing-floor architecture was internally coherent (it was) and whether the implementation correctly mechanized the design (it did). The question that was not asked — and that the packet correctly surfaces — is whether the threat model motivating the transport-ceremony layer applied to local stdio at all.

No reviewer in those 6 cycles surfaced the observation that stdio is an OS pipe and not a wire that HMAC defends. The packet's §1 names this precisely: "Six L0 cycles got CSO sign-off on the internal coherence; nobody asked whether transport-ceremony was the right answer for local stdio/loopback at all."

I find no specific threat that PR #347's premise had correct that this packet has wrong. The strongest case for the prior model would be: "HMAC defends against a compromised Claude Desktop that replays a previously captured signed message to re-invoke a tool after its scope is revoked." Under local stdio, capturing a pipe message requires OS-user access; by the time the attacker has OS-user access, they can revoke-and-re-pair directly. The scope revocation path (auth.rs KEEP set: `revoke_*`) is the correct mitigation, not nonce replay detection. The prior model was not wrong about _what_ it built; it was wrong about _whether_ the threat existed locally.

---

## Q5 — Audit masking: `PARAM_PAYLOAD_KEYS` / `RESPONSE_PAYLOAD_KEYS` completeness

The `services/mcp_v2/audit.rs` is currently a 7-line stub (implementation pending). The `sanitize_detail()` function referenced at audit.rs:111–124 in the packet does not yet exist in that file. The packet's §5.Q9 decision (Write/SubmitCorrection-class uses `sanitize_detail()`) is a correct architectural decision but the masking key lists — `PARAM_PAYLOAD_KEYS` and `RESPONSE_PAYLOAD_KEYS` — do not yet exist to evaluate.

**Finding 5 — Mask-list completeness cannot be confirmed pre-implementation: MED (confidence 75)**

The packet asks CSO to confirm the mask-list is correct for write-class at plan time. That confirmation is not possible because the lists do not yet exist in the codebase. What can be confirmed at plan level:

The taxonomy of write-class tools in `services/mcp_v2/handlers/` (contracts.rs `Side::Write` and `Side::SubmitCorrection`) includes: `tool_create_action.rs`, `tool_note.rs`, `tool_update_action_status.rs`, `tool_placement.rs`. The params for these tools will include user-typed content (note body, action description, placement details). The mask list must cover at minimum the JSON keys that carry free-text user input in these handlers.

**Required pre-ship condition (condition 2 below):** Before PR C merges, the `PARAM_PAYLOAD_KEYS` list must be reviewed against every `Side::Write` and `Side::SubmitCorrection` handler in `services/mcp_v2/handlers/` and confirmed to cover all free-text user-typed fields. This review is an L2 gate item for PR C, not a follow-up. The PR C description must include a table: handler → param keys masked.

For read-class (`Side::Read`) abilities: entity IDs, depths, query strings — plaintext-in-encrypted-DB is correct. No user-typed content flows through read params.

---

## Top-3 residual threats if shipped without additional security thinking

1. **`loopback_origin` spoofing in audit log (most likely):** Any same-OS-user process that discovers the port can POST to `/v1/local/invoke` and set a false `loopback_origin` in the body, causing misattribution in the audit log. This is not a privilege escalation (they already have `Actor::User` semantics) but it corrupts the audit trail. Mitigation: server-side derivation of `loopback_origin` only (condition 1).

2. **Write-class mask-list gap (highest impact):** If the `PARAM_PAYLOAD_KEYS` mask list is incomplete when PR C ships, user-authored note text, action descriptions, or correction content lands in the SQLite audit outbox in plaintext. SQLCipher protects it from external readers but the user themselves can query it — the concern is retention and future exposure if the DB is backed up or inspected. Mitigation: enforce mask-list completeness review as an L2 gate on PR C (condition 2).

3. **Body-limit gap on `/v1/local/invoke` (most subtle):** The existing body-collection logic at `surface_runtime/mod.rs:1025–1043` gates unlimited-body collection on known routes. The new `/v1/local/invoke` route falls through to the `None` branch and receives zero bytes rather than the POST body. The implementer must explicitly add the new route to the body-limit gate. Without this, PR B will silently fail to parse any request body — a correctness failure that could be mistaken for an auth issue during L4 and mask the root cause.

---

## Verdict: APPROVE — with two required conditions before PR B and PR C merge

**Condition 1 (PR B — blocks merge):** The `/v1/local/invoke` handler must derive `loopback_origin` server-side only. The request body must not contain a field that flows into `loopback_origin` in the audit record. Add a test that POSTing a body with `"loopback_origin": "anything"` has no effect on the emitted audit row.

**Condition 2 (PR C — blocks merge):** PR C description must include a handler-to-masked-fields table covering every `Side::Write` and `Side::SubmitCorrection` tool in `services/mcp_v2/handlers/`. The L2 reviewer confirms this table is complete before approving.

Neither condition requires re-opening the trust-model question. The packet's decomposition (authorization machinery keep, transport-ceremony drop, first-party loopback as `Actor::User`) is correct and I do not find a threat that invalidates it.
