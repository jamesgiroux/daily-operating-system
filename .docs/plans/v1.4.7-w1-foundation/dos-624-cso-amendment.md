# DOS-624 — CSO L0 amendment for MCP exposure of daily/meeting briefing intelligence

**Type:** L0 amendment (security-annotated)
**Wave:** v1.4.7 W2-A predecessor (gates DOS-175 implementation)
**Lane spec:** [DOS-624](https://linear.app/a8c/issue/DOS-624)
**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.

## Cycle-2 changelog (2026-05-20)

Cycle-1 verdicts: all 4 NEEDS-CHANGES. **Convergent (2+ reviewers):**
- **meeting_briefing existence oracle / enumeration** (CSO + challenge)
- **W3-C two-paths actor_kind tagging is factually wrong** (architect + challenge + devex)

Cycle-2 fixes:

1. **Architect §4 rewrite — actor_kind correctly distinguishes paths** (architect HIGH + challenge + devex convergent). The cycle-1 amendment claimed "both paths write via Actor::McpClient variants distinguishable by actor_kind." Architect correctly flagged: there's only ONE `Actor::McpClient { client_id, conversation_handle }` variant; `audit_log.rs:215` collapses all McpClient invocations to `actor_kind = "mcp_client"`. The W3-C WordPress MCP Adapter path uses `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 — audit tags `actor_kind = "surface_client"`. So the two paths ARE distinguishable, just by `surface_client` vs `mcp_client`, not by two McpClient variants. §4 corrected.

2. **§4 W3-C policy store correction** (architect HIGH). ADR-0129 §4: WP uses WP capability + WP MCP Adapter allowlist + `SurfaceClient` scopes per ADR-0111 §8. Not `mcp_client_manifest` / `mcp_tool_grant` (which is W1-A direct-path-only). §4 specifies independent policy stores per path; operator grants briefing access in BOTH places independently. No shared manifest schema.

3. **§3 sensitivity gate composition** (architect MED). The cycle-1 §3.1 protection said "Handler MUST filter to caller's read.entity_names scope." Architect correctly flagged this should COMPOSE the existing centralized claim sensitivity gate (ADR-0125 + per-call ClaimSensitivity render in services::claims::render), NOT copy redaction logic. §3.1 rewritten to require DOS-175 handler routes briefing output through the centralized sensitivity render with caller's manifest scopes; no parallel redaction.

4. **meeting_briefing uniform unavailable response in DOS-175 AC** (CSO HIGH + challenge HIGH convergent). Existence-oracle defense: DOS-175 W2-A must include `dailyos.read.meeting_briefing(meeting_id)` returning a uniform `{status: "unavailable"}` typed result for ALL of: nonexistent meeting_id, manifest scope insufficient for entity_names, filtered by sensitivity, stale beyond freshness threshold. This pushes the "object resolution hygiene" into W2-A AC explicitly. CSO Q2 conditional sign-off requires this.

5. **CSO #2 audit coverage for unavailable responses** — DOS-175 makes unavailable responses successful typed results (per #4), so the W1-A success-path audit (`audit::write` with `event="mcp.tool_invoked"`) covers them naturally. `params_hash` over `(meeting_id)` fingerprints probing patterns; operator detects via duplicate `params_hash` across many `conversation_handle`s.

6. **Q1 sign-off folded — per-pairing only** (CSO + devex confirm). Default-denied per-pairing grants + revocation are sufficient for v1.4.7. Global policy registry remains path-α (separate ticket: "DOS-? v1.4.7+ MCP global briefing policy registry").

7. **Q2 sign-off folded — manifest scope authorization + uniform unavailable response** (CSO conditional sign-off). Combined gate.

8. **Singleton substrate hardening filed as separate tickets, NOT folded into this amendment** (per `feedback_review_loop_diminishing_returns_means_scope_is_wrong` + `feedback_l2_path_alpha_to_maintenance_project`):
   - **DOS-? "MCP pairing credential-theft defense"** (challenge #1) — host/client-key binding, one-time pairing codes, short pairing TTL, operator-visible device identity. Real substrate work for v1.4.7+; not blocking this amendment because the post-pairing trust contract already covers replay (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed which CSO independently verified holds).
   - **DOS-? "MCP per-user global briefing rate-limit budget"** (challenge #4) — path-coalesced anomaly detection across W1-A direct + W3-C WP paths. Substrate work for v1.4.7+.
   - **DOS-? "MCP scope grant UI: no glob, explicit briefing scope, confirmation copy"** (challenge #5) — operator UI work, post-v1.4.7.
   - **DOS-? "MCP response_hash side-channel mitigation"** (challenge #2) — per-row salt or security-admin-only fingerprint querying. Substrate work for v1.4.7+.
   - **DOS-? "MCP operator pairing UX: briefing intelligence section"** (devex #1) — operator-facing docs + manifest UI changes. Post-v1.4.7.
   - **DOS-? "MCP rate-limit config keys + retry-after surfacing"** (devex #2) — DOS-175 should make `rate_limit_max=60`, `rate_limit_window_secs=3600` discoverable; gateway's `ToolError::RateLimited { retry_after_seconds }` already exists in W1-A so this is documentation + config wiring, not substrate.
   - **DOS-? "MCP audit dashboard query for briefing replay anomaly detection"** (devex #3) — explicit SQL query schema for operator dashboards.
   - **DOS-? "MCP two-paths-one-ability operator runbook"** (devex #4) — pre-W2-A docs work; how to grant/revoke each path independently; how to audit both sources via `actor_kind` filter.

   These are real, named, scoped — filed in this amendment rather than folded keeps the amendment as a policy decision (its proper shape) rather than turning it into a substrate-hardening initiative.

---

## 1. What this amendment unblocks (unchanged from cycle 1)

v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
- `dailyos.read.daily_briefing` — current-day briefing
- `dailyos.read.meeting_briefing(meeting_id)` — prep briefing for a specific meeting

The v1.4.3 carve-out blocks them pending this amendment.

## 2. Why MCP exposure is appropriate now (unchanged + CSO verified)

The v1.4.3 carve-out predates v1.4.7 W1-A's MCP trust contract. Pre-W1-A, MCP exposure meant unauthenticated unbounded ability invocation. v1.4.7 W1-A landed (commits `45b3f53d`..`3e5d7155` on `v1.4.7-w1-foundation`; L2 unanimous APPROVE; CSO confirmed in this cycle's review: "No new transport replay vector found. ADR-0102 nonce consume + per-response refresh + fail-closed semantics are implemented in `auth.rs`/`gateway.rs` as described.") Every MCP-originated invocation now passes through:

1. Pairing handshake → server-issued `McpClientId` + operator-set manifest
2. Transport HMAC-SHA256 signing over canonical-JSON envelope including `request_nonce`; key wrapped in `Zeroizing`
3. Server-side nonce ledger consume-once + fail-closed (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed)
4. Server-side manifest scope authorization (caller-asserted scopes rejected)
5. Per-tool exposure tier (None / MetadataOnly / Invocable; default None)
6. Per `(McpClientId, ScopedName)` rate limit with atomic BEGIN IMMEDIATE
7. Audit attribution with keyed HMAC-SHA256 hashes (no raw payloads)
8. Signal emission via `McpToolInvoked` / `McpInvocationRejected` (NonPiiMetadata)

Pre-W1-A threat model (unauthenticated MCP exposure of personal briefing intelligence) no longer applies.

## 3. Briefing-specific protections (cycle-2 sensitivity gate composed)

Beyond W1-A's universal gates, DOS-175 (W2-A) MUST honor these briefing-specific ACs:

1. **Sensitivity rendering composes the centralized claim sensitivity gate** (cycle-2 fix #3 per architect MED). DOS-175 handler routes briefing output through `services::claims::render` (or the equivalent centralized sensitivity rendering path per ADR-0125 + ADR-0108) using the caller's manifest-resolved scopes. PII-tier claims (file paths, raw entity names, raw claim text in aggregates) redact when caller's manifest does not grant the corresponding read scope (`read.entity_names`). NO parallel redaction logic in the handler.

2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.

3. **Per-invocation rate limit default = 60 calls/hour** (unchanged from cycle 1). Operator-tunable.

4. **Uniform unavailable response for meeting_briefing(meeting_id)** (cycle-2 fix #4 per CSO HIGH + challenge HIGH). DOS-175 handler returns `{status: "unavailable", reason_class: "unavailable"}` (NOT distinguishing nonexistent / scope-insufficient / sensitivity-filtered / stale-beyond-freshness — uniform shape) for ALL of:
   - meeting_id not in caller's accessible meeting set
   - meeting exists but scope insufficient for entity_names disclosure
   - meeting exists, accessible, but ClaimSensitivity gate filters all surfaceable claims
   - meeting exists but briefing freshness exceeds operator-configured threshold (stale)
   
   **Uniform timing floor for unavailable success responses** (cycle-2 CSO NEEDS-CHANGES fix): the W1-A AC-12 10ms floor applies only to auth-state rejections, NOT to successful typed responses. DOS-175 handler MUST apply an explicit timing floor (recommended: `tokio::time::sleep_until(start + Duration::from_millis(20))`) before returning ANY `{status: "unavailable"}` response, regardless of which internal branch (nonexistent / scope-insufficient / sensitivity-filtered / stale) produced it. Concurrent burst test required to assert all four branches return within a uniform jitter band. Closes CSO cycle-2 enumeration-via-latency finding.

5. **Audit covers unavailable responses** (cycle-2 fix #5 per CSO #2). Because unavailable is a successful typed result, W1-A's success-path audit naturally fingerprints it via `params_hash` over `(meeting_id)`. Operators detect probing via duplicate `params_hash` across many `conversation_handle`s. No new audit substrate needed.

## 4. Coordination with v1.4.2 W3-C WordPress MCP Adapter (cycle-2 corrected)

Per cycle-2 fix #1 + #2 (architect HIGH convergent):

- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
- **Different actor classes, distinguishable in audit**:
  - W1-A direct path (Claude Desktop / Cursor / other agents over loopback HTTP or stdio MCP) → `Actor::McpClient { client_id, conversation_handle }` → audit `actor_kind = "mcp_client"`
  - W3-C WP MCP Adapter path (WordPress block + adapter) → `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 §4 → audit `actor_kind = "surface_client"`
- **Different policy stores per path** (cycle-2 fix #2):
  - W1-A direct: `mcp_client_manifest` + `mcp_tool_grant` rows (v241 migration; loaded by `services::mcp_v2::auth`)
  - W3-C WP: WP capability + WP MCP Adapter allowlist + `SurfaceClient` scope grants per ADR-0111 §8
  - Operator MUST grant briefing access in BOTH places independently — no shared manifest schema, no implicit propagation.
- **No duplicate audit rows** — each invocation through either path writes ONE audit row attributed to its respective Actor variant; the audit log's `actor_kind` field distinguishes the source.
- **Operator runbook for two-paths coordination** filed as separate ticket (cycle-2 fix #8 — "DOS-? MCP two-paths-one-ability operator runbook").

## 5. Audit + signal payload changes (NONE required) (unchanged)

This amendment adds no new audit event types, no new signal types, no new claim types. The W1-A substrate handles briefing invocations identically to any other tool. The amendment is a scope decision, not a substrate change.

## 6. Acceptance

- **CSO L0 sign-off** on this amendment (Q1 + Q2 cycle-1 sign-offs folded per cycle-2 fix #6 + #7).
- **No code changes in this amendment** — scope/policy only. Implementation lands in DOS-175 (W2-A).
- **W2-A DOS-175 L0 packet** (separate) MUST cite this amendment as predecessor + enumerate §3 ACs in its own AC (especially #1 sensitivity-gate composition + #4 uniform unavailable response + #5 audit fingerprinting).

## 7. Spinoff Linear tickets (cycle-2 fix #8 filed list)

Filed to DailyOS Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per path-α discipline:

- "MCP pairing credential-theft defense" (challenge #1; substrate)
- "MCP per-user global briefing rate-limit budget" (challenge #4; substrate)
- "MCP scope grant UI: no glob, explicit briefing scope, confirmation copy" (challenge #5; UX)
- "MCP response_hash side-channel mitigation" (challenge #2; substrate)
- "MCP operator pairing UX: briefing intelligence section" (devex #1; docs + UI)
- "MCP rate-limit config keys + retry-after surfacing" (devex #2; docs + DOS-175 AC reference)
- "MCP audit dashboard query for briefing replay anomaly detection" (devex #3; ops dashboards)
- "MCP two-paths-one-ability operator runbook" (devex #4; pre-W2-A docs)

## 8. Open questions — none remaining (CSO Q1 + Q2 signed off in cycle 1+2; cycle-2 folded)

## 9. Reviewer dispatch (cycle 2)

- **CSO** (mandatory primary) — verify Q1 + Q2 sign-offs folded correctly + new §3 #4 uniform unavailable response is sufficient existence-oracle defense
- **/codex challenge** (adversarial) — verify the 8 spinoff tickets actually capture the cycle-1 findings adequately; nothing dropped that should block
- **architect-reviewer** (substrate) — verify cycle-2 fix #1+#2 actor_kind framing + #3 sensitivity composition match ADR-0111/0125/0129/0130
- **/plan-devex-review** (DX) — verify cycle-2 fix #8 ticket-filing-vs-folding split is the right call vs cycle-1 ask
