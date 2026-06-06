# ADR-0137: Local MCP same-user trust boundary

**Status:** Accepted
**Date:** 2026-06-05
**Supersedes:** local-stdio portions of [ADR-0102](0102-abilities-as-runtime-contract.md) §C and [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §F
**Amends:** [ADR-0111](0111-surface-independent-ability-invocation.md) MCP tool-registration schema publishing for local stdio v2 transport metadata
**Preserves:** [ADR-0102](0102-abilities-as-runtime-contract.md) §A/B/D/E/G, [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §C/D, [ADR-0105](0105-provenance-as-first-class-output.md) and [ADR-0108](0108-provenance-rendering-and-privacy.md) output/provenance obligations

## Context

ADR-0102's 2026-05-19 MCP amendment modeled every MCP client like an external surface: pairing handshake, transport HMAC, server-side scope manifests, per-tool rate limits, revocation, and keyed audit hashes. v1.4.9 W2 re-scopes the local desktop MCP server to its actual topology: a same-OS-user stdio process launched on the same Mac as the DailyOS Tauri app.

In that topology, HMAC/pairing/presence-nonce/scope-grant state duplicates the operating-system user boundary without adding meaningful isolation. The useful security properties are different: the caller must not assert identity or grants, writes must remain closed unless explicitly exposed, audit must not store raw payloads, and MCP-originated activity must still carry `Actor::McpClient` attribution rather than collapsing into `Actor::User`.

## Decision

For local stdio MCP only, DailyOS retires the ADR-0102 pairing/HMAC/presence-nonce/scope-manifest/rate-limit requirement and replaces it with a server-owned local runtime:

- **Identity is server-owned.** The local MCP server loads or creates a stable opaque `McpClientId` in the macOS Keychain. `DAILYOS_MCP_CLIENT_ID` and caller-provided client ids are not accepted.
- **Caller assertions are rejected.** Tool params may not carry `clientId`, `actor`, `scope(s)`, `grantedScopes`, `conversationId`, `side`, or `sensitivity`. The only caller-echoed DailyOS transport metadata is `arguments._dailyos.conversationHandle`, advertised in public v2 `tools/list` schemas so schema-following hosts can preserve continuity. `_dailyos` is stripped before handler/ability validation, is not part of `AbilityDescriptor.input_schema`, and cannot carry actor, client, scope, side, sensitivity, raw conversation id, or arbitrary authority.
- **MCP schemas are gateway-owned wrappers.** For local stdio v2, the public MCP input schema is the handler/ability schema plus the optional reserved `_dailyos.conversationHandle` wrapper. Handler and ability validation still see only typed handler params.
- **Response envelopes preserve typed output.** Successful local stdio v2 responses are JSON text envelopes with `dailyos.conversationHandle` and `result`. This is the v1.4.9 W2 local-stdio wire migration boundary from the earlier raw top-level JSON text result: compatibility is preserved by keeping the typed tool value intact under `result`, not by retaining the old raw top-level shape. The `result` remains the actor-filtered ability/tool output with provenance, trust rendering, attribution, and sensitivity filtering intact. If a future `provenanceHandle` detail path is used instead of inline provenance/trust fields, that detail tool must be exposed and callable through canonical v2 local stdio `tools/list`; legacy v1-only detail tools do not satisfy this ADR.
- **Exposure is local-stdio-specific.** Read tools may be invocable. `Side::Write` tools are non-invocable over local stdio. `Side::SubmitCorrection` is invocable only for the ADR-0128 submit trio when those handlers are actually registered; W2 does not create placeholder submit handlers.
- **Conversation continuity remains ADR-0102 §D.** The local store uses a server-minted `OpaqueConversationHandle`, 24-hour sliding expiry, transparent remint after expiry or missing non-revoked state, and dedicated failure for revoked or cross-client handles.
- **Audit privacy remains mandatory.** Read audit rows store keyed HMAC-SHA256 digests over canonical JSON for params and responses, never raw payloads. Write/submit audit continues to sanitize payload keys. Digest-key failure fails closed.
- **Local persistence is outside legacy auth tables.** Conversation handles live in a mode-scoped JSON state file guarded by an interprocess lock. The old MCP auth/rate/scope tables are inert for local stdio and may be removed by a later schema cleanup.
- **Legacy MCP v1 is quarantined.** `DAILYOS_MCP_LEGACY_V1=1` cannot start legacy v1 in production local stdio; the only escape hatch is debug-only and explicitly unsafe.

This ADR does not weaken non-local MCP transports. Any future loopback HTTP, remote, or multi-user MCP surface stays under ADR-0102 §C unless another ADR explicitly scopes it differently.

## Consequences

DailyOS keeps the security controls that matter for the single-user desktop trust boundary while removing ceremony that made the local MCP path harder to run and reason about. The local stdio gateway remains fail-closed for caller-asserted authority and read-audit digest failures, but no longer depends on grant rows or HMAC state that the same user could only meaningfully attack by already controlling the local process.

The distinction between `Actor::McpClient` and `Actor::User` remains product-significant. MCP activity is still attributable to the headless surface for provenance, audit, and later feedback-loop analysis even though the same human owns both processes.
