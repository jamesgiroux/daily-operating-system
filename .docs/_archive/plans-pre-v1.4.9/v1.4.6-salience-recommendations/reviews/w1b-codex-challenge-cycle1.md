BLOCK

Findings ordered by severity:

- CRITICAL — `score_salience` is declared as a Read ability but persists rows. Packet §6 writes `salience_factors` in a transaction; §8 registers `category = Read`. ADR-0102 defines Read as no service mutation. Persistent factor rows are product audit state, not ephemeral telemetry. Split persistence into an internal/Maintenance recompute path, or reclassify the ability and update the contract.

- HIGH — MCP/SurfaceClient exposure is over-broad. Packet §8 sets `McpClient`, `mcp_exposure = Invocable`, and `read.salience`; v1.4.6 wave §369 says MCP exposure is post-v1.4.6 with `dailyos.*` tool scopes, and ADR-0125 keeps v1.4.6 recommendation exposure at MetadataOnly until v1.4.7 wrapping. Direct `score_salience(claim_id)` also bypasses W2 surfacing/render gates for quiet or suppressed items.

- HIGH — `for_actor` has no authorization contract. Packet §8 accepts arbitrary `SubjectRef`; §7 UserFit reads feedback history. With Agent/SurfaceClient/McpClient allowed, numeric UserFit/rationale values can leak private feedback or preference history unless `for_actor` is bound to the authenticated caller or disabled for external actors.

- MEDIUM — K-in evidence is not recorded. The packet declares a mandatory K-in learning check but does not cite `docs/solutions/` hits or substrate-pattern grep evidence. This touches known patterns around substrate discovery and capability boundaries, so acceptance should name the exact reused scoring/trust/corroboration APIs or explicitly document any gaps.