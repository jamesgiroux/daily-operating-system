# DOS-624 — CSO L0 amendment for MCP exposure of daily/meeting briefing intelligence

**Type:** L0 amendment (security-annotated)
**Wave:** v1.4.7 W2-A predecessor (gates DOS-175 implementation)
**Lane spec:** [DOS-624](https://linear.app/a8c/issue/DOS-624)
**Origin:** v1.4.3 carve-out per Linear project description — `get_daily_briefing` ability shipped as Read-only with **User actor only**. **No Agent/MCP exposure without a separate CSO-approved L0 amendment.** This is that amendment.
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Reorientation" + §"Agent W2-A — DOS-175".

## 1. What this amendment unblocks

v1.4.7 W2-A (DOS-175) ships two MCP tools that wrap the v1.4.3 `get_daily_briefing` ability:
- `dailyos.read.daily_briefing` — host-model invocation returning the user's current-day briefing
- `dailyos.read.meeting_briefing(meeting_id)` — host-model invocation returning the prep briefing for a specific meeting

Both surface the same intelligence the Tauri app's briefing surface consumes. The v1.4.3 carve-out blocks them pending this amendment.

## 2. Why MCP exposure is appropriate now

The v1.4.3 carve-out predates the v1.4.7 W1-A MCP trust contract. The carve-out was correct at the time: pre-W1-A, MCP exposure meant unauthenticated, unsigned, unbounded ability invocation. v1.4.7 W1-A changes that. Per the now-merged ADR-0102 §C/§D/§E + §C.bis.replay/refresh/schema/fail-closed (cycle-7/8/9 amendments) every MCP-originated invocation passes through:

1. **Pairing handshake** (`auth::pair_client`) — operator-set per-client manifest of scopes + per-tool exposure tier + per-tool rate limits. Server-issued `McpClientId` is non-spoofable.
2. **Transport HMAC-SHA256 signing** over the canonical-JSON bytes of the WHOLE envelope including `request_nonce`. Key material is `Zeroizing`-wrapped end-to-end (W1-A AC-11).
3. **Server-side nonce ledger** (`mcp_transport_nonce_ledger`) per ADR-0102 §C.bis.replay — atomic consume-once + fail-closed; nonce stays consumed even on downstream failure (TCP-reset replay defense).
4. **Server-side scope manifest authorization** — caller-asserted scopes/conversation_id REJECTED; gateway enforces against `mcp_tool_grant` rows loaded per-dispatch (no cache; revocation propagates within 1 call).
5. **Per-tool exposure tier** (`McpExposure::None | MetadataOnly | Invocable`) — operator-set at pairing; non-Invocable → `ExposureForbidden` dedicated variant. Default tier is `None` (deny).
6. **Per `(McpClientId, ScopedName)` rate limit** with atomic `BEGIN IMMEDIATE` reservation.
7. **Audit attribution** (`mcp.tool_invoked` event in `audit_log`) with keyed HMAC-SHA256 hashes of params + response (no raw payloads). Audit double-failure surfaces `ToolError::Internal` + Suite-S alert.
8. **Signal emission** (`SignalType::McpToolInvoked` + `McpInvocationRejected` registered at 5 sites per ADR-0115; NonPiiMetadata payloads).

The pre-v1.4.7 carve-out's threat model (unauthenticated MCP exposure of personal briefing intelligence) no longer applies. The post-v1.4.7 MCP path is more strictly gated than any other DailyOS substrate consumer.

## 3. Specific protections for daily/meeting briefing

Beyond the gateway's universal gates, this amendment names the briefing-specific protections W2-A (DOS-175) MUST honor:

- **Sensitivity rendering at handler boundary.** The briefing payload includes claims with `ClaimSensitivity` per ADR-0125. Handler MUST filter to the caller's `read.entity_names` scope grant; PII-tier claims (file paths, raw entity names, raw claim text in aggregates) must be redacted when the caller's manifest does not grant the corresponding read scope. Same redaction rules as v1.4.5 `WorkspaceGraphProjection v1`.
- **Per-pairing tool grant default = denied.** New pairings do NOT auto-receive briefing access. The operator (pairing manifest editor) must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
- **Per-invocation rate limit budget = conservative.** Operator-tunable, but the recommended default for briefing tools is `60 calls / hour` per `(McpClientId, ScopedName)` — substantially lower than read-tool defaults — to reduce inference-attack surface.
- **Audit detail includes briefing fingerprint.** The audit row's `params_hash` covers the briefing date / meeting_id; response_hash covers the rendered output. Operators can detect anomalous briefing-replay patterns by grepping audit log for `event="mcp.tool_invoked"` + `detail.tool_name="dailyos.read.daily_briefing"` + abnormal cadence.

## 4. Coordination with v1.4.2 W3-C WordPress MCP adapter

Per wave plan reorientation (2026-05-15), v1.4.2 W3-C already ships a WordPress-mediated MCP server via the WP MCP Adapter with a DailyOS ability allowlist + dedicated low-cap WP user + read-mostly defaults. v1.4.7 is the **second MCP path** — direct headless MCP from runtime to Claude Desktop / Cursor / other agents.

This amendment specifies the coexistence:

- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 already supports surface-agnostic producer abilities.
- **Per-tool exposure tier is per-pairing** — a single operator might grant briefing access to the WP user (W3-C path) but deny it to a Claude Desktop pairing (W1-A direct path), or vice versa. The manifest schema supports this naturally; no new substrate.
- **No duplicate audit rows.** Each invocation through either path writes ONE audit row attributed to its respective `Actor::McpClient` variant; the audit log tags the invocation source via the existing `actor_kind` audit field (see audit_log.rs:38 + W1-A audit.rs).
- **Operator runbook** (path-α maintenance ticket, NOT blocking W2-A): document the two-paths-one-ability model for downstream operator dashboards. Cross-link to ADR-0129 (composable surfaces) + ADR-0130 (composition contract).

## 5. Audit + signal payload changes (NONE required)

This amendment adds no new audit event types, no new signal types, no new claim types. The W1-A substrate (audit.rs `event="mcp.tool_invoked"` + SignalType::McpToolInvoked) handles briefing invocations identically to any other tool. The amendment is a scope decision, not a substrate change.

## 6. Acceptance

- **CSO L0 sign-off** on this amendment (mandatory; this whole document is the artifact).
- **No code changes** — this is a scope/policy amendment that unblocks W2-A. Implementation lands in DOS-175 (W2-A).
- **W2-A DOS-175 L0 packet** (separate) MUST cite this amendment as predecessor + enumerate the §3 briefing-specific protections in its AC.

## 7. Path-α (file as Maintenance, not blocking this amendment)

- Operator runbook for two-paths-one-ability briefing exposure (§4 final bullet) — DX docs work, ship with W3-C/W2-A operator-facing guidance.
- Per-tool rate-limit policy registry (recommended defaults across all v1.4.7 tools, not just briefing) — substrate hardening for v1.4.7+.

## 8. Open questions

| # | Question | Default | Resolution path |
|---|---|---|---|
| Q1 | Should briefing-tool exposure be opt-in-per-pairing OR opt-in-per-tool-globally-then-per-pairing? | per-pairing only (operator decides at pairing handshake; no global toggle) | CSO at L0 sign-off |
| Q2 | Should `meeting_briefing(meeting_id)` validate the caller's user-context implicit access to the meeting OR rely solely on the manifest scope grant? | manifest scope grant ONLY (W1-A trust model is gateway-mediated; ability-internal user-context checks would re-introduce the layered-auth pattern the trust contract was designed to remove) | CSO at L0 sign-off |
| Q3 | Should the v1.4.2 W3-C WP MCP adapter need a parallel amendment for its briefing exposure? | Yes, separate amendment specific to W3-C path; not blocking this one | filed as DOS-625 (suggested) |

## 9. Reviewer dispatch

This amendment IS a CSO L0 review artifact. Dispatch:
- **CSO** (mandatory primary) — security threat-model review per ADR-0102 §C four-gate trust contract; verify briefing-specific protections in §3 are sufficient; sign off on §8 Q1+Q2 defaults.
- **/codex challenge** (adversarial) — try to break the amendment: missing threat-model angles, attribution gaps, replay/enumeration vectors specific to briefing intelligence.
- **architect-reviewer** (substrate) — verify §4 coexistence story holds against ADR-0129/ADR-0130 + audit_log.rs actor_kind tagging.
- **/plan-devex-review** (DX) — verify operator pairing experience (Q1 default).
