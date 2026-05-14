# W3 L0 packet — host boundary + open decisions

Date: 2026-05-13 (V4)
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 3 (DOS-563 W3-0, DOS-564 W3-A, DOS-565 W3-B, DOS-566 W3-C)

This packet captures the cross-cutting W3 decisions resolved at L0 so that W3-0 stays narrow and W3-A/B/C contracts are unambiguous when parallel agents fan out. The Linear issue descriptions remain the canonical execution contracts; this packet supersedes them only where it makes explicit a decision they leave open.

## Changelog

- **V4 (2026-05-13):** Cycle 3 reviewer panel — eng + devex + cso all APPROVE; codex CONDITIONAL with 3 textual findings (transport-neutrality mapping under non-default transports, handshake-per-invoke vs handshake-at-pairing ambiguity, marker-as-heuristic clarification). 2-reviewer class signal on stdio mapping (codex + cso). V4 folds all 3 as textual tightening; no structural changes.
- **V3 (2026-05-13):** Cycle 2 reviewer panel — eng + devex + cso all APPROVE; codex CONDITIONAL with 3 new findings. V3 folds all 3: loosens MCP transport pin to invariants-only (transport class deferred to W3-0 adapter choice per DOS-566 contract), strikes the credential-retrieval OR in favor of the DOS-565 contract (manage_options-gated WP filter, no WP persistence), defines the prior-pairing marker shape at W3-A activation.
- **V2 (2026-05-13):** Folded L0 reviewer findings — eng (3), devex (4), cso (5), codex (5 of which 1 overlaps with cso). After consolidation all four reviewers are CONDITIONAL. Adds: namespace-vacancy + reservation invariant for W3-A; credential state machine + HMAC redaction wrapper for W3-B; MCP transport pinning + invariant-not-assertion for default-MCP enumeration + drift-fixture for W3-C; supply-chain pinning, prod-vs-dev split, WP version fixture matrix, transport-class gating clause for W3-0→W3-B. Renamed `mcp_server_name` audit field to `mcp_exposure_path`. Renderer/detector handoff named explicitly.
- **V1 (2026-05-13):** Initial draft.

## Status snapshot

- W2 closed clean 2026-05-12 — loopback transport, HMAC, pairing+4-path recovery, rate-limit matrix all merged.
- W3 milestone seeded with four issues (DOS-563/564/565/566). Internal staging: W3-0 → W3-A → (W3-B ∥ W3-C).
- v1.4.2 supersedes the parked v1.4.10 entity-intelligence project; first WordPress proof is one substrate-authored composition rendered as one dynamic Gutenberg block, not full entity surfaces.

## Pre-work confirmed

- WP skills installed: `automattic/agent-skills@wp-plugin-development`, `wp-block-development`, `wp-wpcli-and-ops`, `wp-block-themes`. The `ollie` block-theme skill is already present and may be relevant for W3-A WP Abilities tools workflow + W5-B magazine theme.
- Studio CLI (`wp-studio` npm, v1.9.0) installed and working. `studio site create --blueprint <path>` confirmed available.
- Studio app upgraded to 1.9.0 (was 1.6.3); config migration complete.

## Studio site posture

- **Primary W3-W6 implementation + W6-B validation target:** `DailyOS Dev` at `~/Studio/dailyos-dev`, port 8884, PHP 8.4, WP latest. Created clean 2026-05-13.
- **Stock-content control site:** `~/Studio/dailyos` (marketing-site WP install, autoStart on) — used as a stock-content control so we can prove the DailyOS plugin doesn't break a generic Studio site with unrelated content. Not the implementation target.
- **Decision rule:** any negative fixture or W6-B clean-machine evidence must run against `~/Studio/dailyos-dev` or a fresh `studio site create`. Evidence against the marketing site is invalid because pre-existing content contaminates baselines.

## Studio Blueprint posture

Validated 2026-05-13. Studio Blueprints (introduced 1.6.0, local resource fix in 1.7.8) are JSON files conforming to the WordPress Playground blueprint schema, with steps including `installPlugin`, `installTheme`, `wp-cli`, `runPHP`, `setSiteOptions`, `defineWpConfigConsts`. Local zips referenced as `vfs` resources work since 1.7.8. The DailyOS bundle (plugin + theme + pairing setup) maps cleanly onto a single blueprint.

- **W6-B bootstrap mechanism:** `studio site create --blueprint <path>`. No need to script around Studio.
- **Blueprint authorship lives in W6**, not W3. W3 produces the plugin + theme + custom MCP server; W6 packages them into a blueprint and validates clean-machine install.

## Studio native MCP — host vs site layer

Probed 2026-05-13. `studio mcp` exposes 24 hardcoded tools, all host-layer (site lifecycle, WP.com push/pull, `wp_cli` passthrough, scaffold/validate helpers, preview management). No `resources` or `prompts` capability, no extension point, no way to register DailyOS abilities into Studio's MCP.

This resolves a W3-C open question definitively:

- **Studio MCP and DailyOS plugin MCP are at different layers, not competing.** Studio MCP manages local sites; DailyOS plugin MCP exposes DailyOS abilities running against the substrate. Both can coexist on the same machine.
- **DOS-566 plan stands.** DailyOS ships its own MCP server using `wordpress/mcp-adapter` (or W3-0 chooses an equivalent), with explicit allowlist + low-cap WP user + SurfaceClient scope enforcement.
- **Host-boundary invariant:** Studio's `wp_cli` and `site_import/export` tools can mutate the WP DB independently of DailyOS scope enforcement. DailyOS treats those mutations as **out-of-band edits** — they hit the W4-B watermark contract + W4-C tamper detection, and surface in trust-band rendering as `use_with_caution` / `needs_verification`. They are never silently promoted to canonical substrate.
- **Acknowledged DX papercut (V2):** developers exercising the standard Studio dev loop with `studio mcp wp_cli` will sometimes see `use_with_caution` states on DailyOS-rendered content. This is intentional behavior routed through W4-B watermark + W4-C tamper detection — not a W3-C ask. A future surface affordance can offer "this was my edit" attribution; v1.4.2 detects, renders, and does not silently promote.

## Production vs Studio dev (V2)

`wp-studio` CLI, `studio mcp`, and Studio Blueprints are dev-loop and clean-machine-validation tooling. They are **not** production WordPress operations tooling.

Banned from any production runbook, deployment guide, or end-user instruction:

- `studio mcp wp_cli` passthrough — Studio host-layer tool, dev-only.
- `studio site export/import` — Studio site-lifecycle, not a production migration tool.
- Any `wp-studio` CLI command that mutates site state at runtime (`wp_cli`, `runPHP`, blueprint re-apply).

Production WordPress operations reference standard WP-CLI, standard plugin install flows, and standard WordPress.org plugin/theme distribution. The DailyOS plugin readme (W3-B authorship) and W6 install documentation inherit this rule by reference rather than re-deriving it.

## Directional decisions resolved at L0

These are not deferred to W3-0 spike; they are inputs to it.

1. **WPGraphQL — out of v1.4.2 critical path.** Any GraphQL usage is reference/prior-art only, read-only WP-local, and cannot bypass SurfaceClient/HMAC/nonce/scope. W3-0 confirms this in writing; no implementation work.
2. **Remote Data Blocks — reference/borrow patterns only.** Editor UX and server-rendered block patterns are inspiration; DailyOS runtime remains canonical, RDB is not vendored or forked into the plugin.
3. **WordPress MCP Adapter — directional default: depend on it as a library, with explicit fallback.** W3-0 confirms whether to take a direct dependency, pin a fork, or implement adapter-style patterns inline. The custom DailyOS MCP server registers using the adapter's pattern; ability logic does not move to PHP. **(V2)** Pin a specific known-good adapter release in the W6-B blueprint. Exit strategy: if the adapter ships a breaking change OR stalls (no commit ≥6 months), the v1.4.x project owning the next blueprint refresh evaluates vendor/fork. The pin is a v1.4.x contract, not a forever choice.
4. **PHP runtime transport — directional default: WordPress HTTP API** wrapped in `DailyOS_Runtime_Client`, with tests proving exact-byte canonicalization to match the W2 HMAC contract. **(V2)** Gating clause: **if W3-0 finds the WP HTTP API insufficient for HMAC byte-exactness (e.g. body-stream re-read corrupts canonical bytes), W3-B start is gated on the transport-class decision being recorded in this packet (V3) before implementation.** A late-breaking transport switch from WP HTTP API → cURL after W3-A merges is non-trivial and must not happen mid-stream.
5. **WP/PHP minimums — WordPress 6.9+, PHP 8.1+.** **(V2)** "WP latest" is insufficient as a compatibility target. The Abilities API is documented as WP 6.9+ only. W6-B validates on **two pinned WP fixtures**: exact WP 6.9.x minimum AND the current `WP latest` at v1.4.2 release. Both fixtures must pass activation, ability registration, and MCP exposure tests. Compatibility shim work is in scope only if a real consumer needs WP 6.7/6.8.
6. **Studio MCP composition — not pursued.** DailyOS abilities are exposed exclusively through the DailyOS plugin MCP server (DOS-566). Studio's native MCP is treated as a host-layer peer.
7. **Studio CLI ergonomics — adopt `wp-studio` npm CLI for headless flows.** Bundled `studio-cli.sh` in Studio.app is `preview`-only. The npm CLI (`studio`) is the canonical headless interface and is the W6-B clean-machine validation driver. **(V2)** Pin `wp-studio` to a specific known-good version in the W6-B blueprint metadata. Same exit strategy as MCP Adapter (item 3) applies if Studio CLI ships a breaking change between W3 start and v1.4.2 release.

## Host-boundary acceptance criteria (lifted into W3-A/B/C)

These criteria are derived from the W3–W6 Studio-first WordPress prior-art reuse packet (2026-05-12), the workspace-boundary decision (2026-05-12), and the L0 reviewer panel (2026-05-13). They are conditions of L1/L2 passing for the named issues, not aspirations.

### W3-A (DOS-564) plugin skeleton

- MUST NOT introduce: local-site management UI, connector marketplace, OAuth/provider credential storage, generic MCP toolkit browsing, routine builder UI, markdown/block sync engine, Jetpack/Woo admin chrome.
- **(V2)** MUST NOT register `WP_CLI::add_command` entries that shadow or duplicate Studio host-layer surfaces (`site_create`, `site_list`, `site_start`, `site_stop`, `site_delete`, `site_push`, `site_pull`, `preview_*`, `scaffold_theme`, `validate_blocks`, `need_for_speed`, `rank_me_up`, `wp_cli` passthrough). DailyOS WP-CLI commands are scoped to plugin diagnostics + DailyOS-specific operations (e.g. `wp dailyos status`, `wp dailyos repair-projection`).
- MAY register: DailyOS Gutenberg blocks (W4 surface), DailyOS abilities via WP Abilities API, pairing/status diagnostics, settings page (read-side until W3-B).
- **(V2) Reserve DailyOS-owned WP namespaces:** option keys prefixed `dailyos_`, post-type prefixes `dailyos_`, role name `dailyos_substrate`, user-meta prefix `dailyos_`. Namespace reservation is a W3-A activation contract that W4-C will sign + verify.
- **(V2) Namespace-vacancy invariant at first activation:** activation MUST refuse to adopt pre-existing `dailyos_*` option keys, DailyOS-prefixed post-types, the `dailyos_substrate` role, or DailyOS-prefixed user-meta if any pre-exist with no recorded prior pairing. Diagnostic-with-recovery-path UX, never silent inheritance. Negative fixture seeded into phase-0 artifact 12.
- **(V3 + V4) Prior-pairing marker — concrete:** "recorded prior pairing" means a non-secret marker stored in `wp_options.dailyos_pairing_marker` containing `runtime_instance_id`, `site_nonce_hash`, `projection_version`. The marker is non-secret by design: its contents do not authenticate anyone, only attest that *some* prior DailyOS pairing existed on this site. **(V4) The marker is a namespace-vacancy heuristic only — runtime-reported state is the authoritative trust source. Marker-match NEVER bypasses runtime pairing, signature, or scope checks; it only informs the activation-stage decision about whether the WP namespace was previously claimed.** An attacker with WP DB write can forge the marker fields, but cannot forge the runtime's view of `runtime_instance_id` or `projection_version`; the runtime-state comparison is the load-bearing check. Activation logic:
  - Marker absent AND `dailyos_*` namespace clean → legitimate fresh install. Activate.
  - Marker absent AND `dailyos_*` namespace not clean → hostile pre-existing state. Refuse activation with diagnostic; recovery is the `wp dailyos repair-namespace` CLI helper (one of the explicit W3-A WP-CLI commands listed under the `WP_CLI::add_command` MUST NOT exception list).
  - Marker present AND matches the paired runtime's reported state → legitimate re-activation (reinstall, upgrade, re-pair). Proceed.
  - Marker present AND mismatches runtime state → quarantine; require explicit admin repair via the same CLI helper.

  Negative fixtures covering all four paths seeded into phase-0 artifact 12.
- **(V2) Projection envelope shape pinned at W3-A:** every W3-A-authored storage row destined for W4-C signature carries: `dailyos_canonical_id`, `dailyos_signature`, `dailyos_source_runtime` (runtime instance ID), `dailyos_projection_version`. W4-A renderers and W4-C verifiers consume this envelope as the contract.
- **(V2) Plugin source grep gates — concrete:** W3-A authors a regex set in `wp/dailyos/scripts/grep-gates.json` covering:
  - Raw `$wpdb->query`/`$wpdb->insert`/`$wpdb->update`/`$wpdb->delete` outside service wrappers under `wp/dailyos/includes/services/`.
  - Filesystem writes outside `wp-content/uploads/dailyos/` scope.
  - Known secret-persistence patterns: `update_option.*hmac`, `set_transient.*hmac`, `update_post_meta.*hmac`, `update_user_meta.*hmac`, `wp_localize_script.*hmac`, similar variants for `bearer`/`session_key`/`pairing_token`.
  CI job runs against `wp/dailyos/**` on every PR; deliberate-violation negative fixture in phase-0 artifact 12 proves the gate triggers.

### W3-B (DOS-565) runtime client + HMAC + pairing UI

- Browser JS never calls the Rust runtime directly. JS → WP REST/admin → PHP signer → `127.0.0.1:<runtime_port>`.
- Per-session HMAC key is request-time secret material only. Negative grep gates: no key in `wp_options`, transients, post meta, block attributes, JS preload state, page HTML, logs, diagnostic dumps.
- **(V2 + V3 + V4 + V5) Credential state machine:** pairing produces session material (per W2-C contract); session material is held in-process via a `DailyOS_Session_Credential` value object whose lifetime is the request only. **Cross-request retrieval (per DOS-565 contract, V3 + V4 disambiguation):** the W2-C single-use handshake happens **once at pairing time** (or on rotation events below) — NOT per invoke. The handshake issues a per-session credential held by the runtime. Per WP request, the `manage_options`-gated `dailyos_wp_bridge_session_key` WP filter pulls a runtime-resident *reference* to that already-issued credential. The credential lifetime is tied to the W2-C session, not to individual WP requests; admin AJAX, page renders, and block invocations in the same paired session share the same credential. No WP-side persistence of secret material — ever. Rotation: on re-pair, runtime restart, scope change, or revoke — each rotation event invalidates the prior credential and issues a fresh W2-C handshake. Revoke: WP-side flag flips → next request fails closed with typed error. State transitions tested in W3-B unit + integration. **(V5 amendment, 2026-05-14):** bearer_token is REMOVED from the auth contract. HMAC signature alone authenticates signed requests; session_id moves to a non-secret `X-DailyOS-Session-Id` request header. The runtime exposes a new `POST /v1/surface/session/refresh` endpoint that returns the derived per-session hmac_key given a matching `{session_id, site_binding_digest, wp_install_uuid, plugin_instance_uuid}` identity bundle; this endpoint is the concrete implementation of "runtime-resident reference." Auth on the refresh path is loopback + identity-bundle match (no HMAC; chicken-and-egg). Rationale: bearer + HMAC was defense-in-depth that cost W2 substrate complexity (plaintext bearer storage requirements) without adding meaningful security for the local-only port-to-port threat model. HMAC + nonce replay store carries the authentication weight cleanly. Nothing shipped on bearer; W2 wave is L3-re-reviewable for the change. **(V3 superseded):** the prior V3 note ("bearer-token retrieval branch was considered in V2 and struck") referred to a different retrieval-vs-rotation question; V5 supersedes by removing bearer entirely from the signed-request path.
- **(V2) HMAC redaction wrapper:** `DailyOS_Session_Credential` and any `DailyOS_Hmac_Key` value object expose `__debugInfo`, `__toString`, and `jsonSerialize` overrides returning a redacted token (e.g. `***REDACTED***`). Unit tests assert `print_r`, `var_dump`, `json_encode`, and a deliberate `WP_DEBUG_LOG`-line emit of the object yield redacted output, not the secret bytes. Belongs in W3-B; gates on PHP opcode cache / `var_export` debug paths / fatal-error stack dumps capturing in-memory values.
- Pairing/revoke/re-pair handle runtime restart, stale pairing code, concurrent-admin pairing flows with typed user-facing errors that do not leak runtime internals.
- Settings page exposes instance ID, granted scopes, endpoint version, last-use timestamp; never the HMAC key or runtime port. **(V5)** Bearer token no longer exists in the protocol; settings page text and tests updated to drop the bearer reference.
- Pairing UI uses WordPress admin nonces for admin form CSRF only. DailyOS user-presence nonces are separate and are not introduced in W3-B.

### W3-C (DOS-566) custom MCP server

- Default WP MCP server (if any host plugin enables one) MUST NOT enumerate `dailyos/*` tools or schemas.
- **(V2)** "Must not enumerate" is enforced as an invariant, not asserted as prose: DailyOS ability registration sets `mcp_exposure` metadata on every `wp_register_ability()` call; the `mcp_exposure: None` default makes abilities invisible to any generic MCP-adapter enumerator. **Negative fixture:** instantiate a generic MCP adapter against the local WP, enumerate, assert zero `dailyos/*` tools surface unless explicit allowlist is loaded. Belongs in phase-0 artifact 12.
- DailyOS-namespaced MCP server enumerates only allowlisted abilities from `tools/dailyos-abilities.json`: `mcp_exposure: Invocable | MetadataOnly`. `None` is absent.
- `MetadataOnly` exposure never returns invoke schema and never permits invocation.
- Read-mostly default: allowlist defaults to Read/Transform categories. Publish/Maintenance abilities require explicit ADR/issue promotion.
- Dedicated low-capability WP user (`dailyos_substrate` role) created at activation; WP capability alone is never sufficient. Permission callbacks check both WP capability AND DailyOS SurfaceClient scopes before invocation.
- **(V2) Capability + scope drift fixture:** pin the `dailyos_substrate` role's capability map AND the resolved SurfaceClient scope set in `wp/dailyos/tests/fixtures/role-capabilities.json` + `tests/fixtures/surfaceclient-scopes.json`. CI diff-checks both; drift fails the build. Prevents silent capability creep via `add_cap` or scope-set widening.
- Audit emits `mcp_exposure_path` (Invocable vs MetadataOnly enumeration result), `wp_user_id`, `ability_name`, scope-check result via the W1-A0 SurfaceClient audit helper. **(V2)** Renamed from `mcp_server_name` per eng review — the server-name field was constant under the "DailyOS abilities exclusively through DailyOS plugin MCP" decision and carried no forensic value; the exposure-path distinction is the actually-useful signal.
- **(V2 + V3) MCP transport invariants:** the DailyOS MCP server uses whatever transport the W3-0 adapter decision selects (likely REST under `/wp-json/dailyos/v1/mcp/*`, but the adapter may choose SSE, streamable HTTP, or stdio bridge — DOS-566 contract gives W3-0 this choice). Independent of transport choice, the following invariants apply unconditionally:
  - Authentication via the dedicated `dailyos_substrate` WP user (not anonymous, not `manage_options`).
  - No public unauthenticated enumeration — tool listing requires authenticated session.
  - Rate limits inherited from the W2-D `SurfaceClientBridge` matrix (per-instance, per-user, per-site, per-ability, per-scope-class).
  - Bind scope: same-origin only; cross-origin MCP discovery refused.

  **(V3):** the specific transport + adapter compatibility is recorded by W3-0 in a packet V5 amendment if the adapter pattern alters the default REST assumption. V2 over-constrained by pinning REST; V3 corrects to invariants-only.

  **(V4) Transport-neutrality mapping requirement:** if W3-0 selects a non-default transport (SSE, streamable HTTP, stdio bridge), the V5 amendment MUST explicitly map each of the four invariants above to that transport. Reference mappings:
  - **REST (default):** auth via WP REST nonce / cookie / app password for `dailyos_substrate` user; no anon enum = no permission callback for guests; rate-limit identity = `wp_user_id` + IP; bind = same-origin via `Origin`/`Host` header allowlist.
  - **SSE:** auth same as REST at connection open; no anon enum = connection refused without session; rate-limit identity = `wp_user_id` + connection; bind = `Origin` header allowlist of paired runtime URLs.
  - **stdio bridge:** auth = process spawned by the paired DailyOS runtime (process-parentage check + runtime-signed handshake nonce on first frame); no anon enum = no MCP frames before runtime auth; rate-limit identity = runtime-instance-id passed in handshake; bind = process boundary, no network listener.

  Adapters that cannot map all four invariants are rejected at the W3-0 decision stage. The mapping requirement is documentation, not implementation work for W3-C — implementation follows W3-0's chosen adapter.
- **Host MCP coexistence invariant:** Studio's native `wp_cli` MCP tool can mutate the WP DB. DailyOS-side handling: any WP DB row that differs from the runtime's signed projection (per W4-C) renders in degraded/tamper state; this is the same path used for direct WP admin edits and DB restores. **(V2) Renderer/detector handoff named explicitly:** W4-A renderer reads tamper/quarantine state from the W4-C projection ledger and degrades the trust band accordingly. **W3-C does not own UI degradation;** W3-C's audit emission carries the divergence signal, and W4-A/C consume it.

## Workspace-boundary release-gate invariants

From the 2026-05-12 workspace boundary decision. These are additive release-gate items, not new code:

- DailyOS canonical workspace remains `DailyOS/` (per-user, source ledger, claims, provenance). WordPress DB is a projection surface. No code path in W3-A/B/C may promote a WP DB row, a markdown file edit, or a Studio site-import into canonical substrate state.
- WordPress, including local Studio sites, may become a source for future signal ingestion. v1.4.2 does not ship that ingestion path; v1.4.6 owns it.
- DailyOS-authored projections must be distinguishable from human-authored WordPress content at the source-ingestion layer (v1.4.6 contract). v1.4.2's contribution is making this distinction renderable via projection signatures (W4-C) so that v1.4.6 has a substrate to consume.

## Linear dependency edges (formalize)

Currently described only in prose. Add as Linear blocker relations:

- DOS-563 (W3-0) blocks DOS-564 (W3-A).
- DOS-564 (W3-A) blocks DOS-565 (W3-B).
- DOS-564 (W3-A) blocks DOS-566 (W3-C).
- DOS-563 (W3-0) blocks DOS-565 (W3-B) and DOS-566 (W3-C) transitively; explicit edges optional.

W3-B and W3-C run in parallel after W3-A merges. **(V2)** W3-B's start is additionally gated on the W3-0 PHP-transport decision being recorded in this packet (V3) if the WP HTTP API canonicalization concern surfaces — see Directional decision item 4.

## What W3-0 (DOS-563) still owns after this packet

This packet shrinks W3-0 scope to the items still genuinely ambiguous:

1. **MCP Adapter dependency posture:** direct dependency, pinned fork, or pattern-only. Default lean: direct dependency, pinned to a known-good release.
2. **rsm-second-brain mapping:** which exact files/patterns we borrow for bootstrap, domain layout, REST/service split, asset manifest guard, AI-log evidence. The "borrow architecture, not substrate semantics" rule is fixed; the file-level mapping is research output.
3. **A8C-internal prior-art sweep:** if/when GitHub auth resolves, scan WooCommerce MCP/Abilities work, Big Sky plugin, wp-admin-rpg, claude-code-vip for additional reusable shape. Internal-code findings summarize shape only — no proprietary code in Linear.
4. **PHP transport edge cases:** what (if anything) breaks WP HTTP API exact-byte canonicalization at the HMAC-signed boundary; whether body-stream re-reads are safe. **(V2)** If WP HTTP API fails byte-exactness, W3-0 records the transport-class switch decision in a packet V3 amendment; W3-B does not start until this is locked.
5. **Studio Blueprint authoring spike for W6-B:** the actual blueprint JSON shape that bundles plugin zip + theme zip + WP-CLI bootstrap + pairing onboarding admin notice. Output is a v0 blueprint that W6-B can validate, not a finished one.
6. **(V2) WP version pin:** confirm the exact WP 6.9.x minimum target (likely the latest 6.9 point release at v1.4.2 release time) and document the "current latest" target. Both fixtures land in the W6-B blueprint test matrix.

## L0 reviewer panel — results (V2)

Reviewers run 2026-05-13. All four reviewers CONDITIONAL after consolidation. Verdicts posted as comments on DOS-546:

- **Eng:** 3 findings (P1 renderer handoff, P2 audit-field rename, P3 transport gating clause). All folded into V2.
- **DevEx:** 3 P2 + 1 P3 (supply-chain pinning, prod-vs-dev split, missing WP_CLI MUST NOT, DX papercut ack). All folded into V2.
- **CSO:** 2 HIGH + 2 MEDIUM + 1 LOW (concrete grep gates, namespace vacancy, HMAC redaction wrapper, capability+scope drift fixture, pairing nonce non-issue). All folded into V2.
- **Codex (challenge mode):** 5 findings, BLOCK verdict (consolidated to CONDITIONAL — credential state machine overlaps CSO MEDIUM; MCP transport / invariant-vs-assertion / namespace + envelope reservation / WP version matrix all folded into V2).

Two genuinely new W3-A scope items emerged from the panel and are now in the W3-A host-boundary criteria above: (a) reserve DailyOS WP namespaces + pin projection envelope shape, (b) namespace-vacancy check at activation.

### Cycle 2 (2026-05-13, V2 packet under review)

- **Eng:** APPROVE. All 3 V1 findings ADDRESSED; no new L0-blocking findings.
- **DevEx:** APPROVE. All 4 V1 findings ADDRESSED; no new L0-blocking findings.
- **CSO:** APPROVE. All 4 V1 findings ADDRESSED; no new L0-severity findings.
- **Codex (challenge mode, full context — V2 packet + W3-A/B/C issue contracts inline):** CONDITIONAL. All 5 V1 findings ADDRESSED; 3 new findings — [HIGH] MCP transport over-constraint (V2 pinned REST when W3-0 owns adapter choice per DOS-566), [MEDIUM] credential retrieval OR conflicting with DOS-565 single-path contract, [LOW] prior-pairing marker undefined. All 3 folded into V3.

### Cycle 3 (2026-05-13, V3 packet under review)

- **Eng:** APPROVE. NO-REGRESSION on all three V3 changes; marker shape aligns 1:1 with W4-C projection envelope.
- **DevEx:** APPROVE. NO-REGRESSION; high-frequency-invocation latency papercut named as future packet V5 amendment owned by W3-0 with required attributes spelled out.
- **CSO:** APPROVE. NO-REGRESSION; sharp note that stdio "same-origin" means process-parentage (folded into V4 transport-neutrality mapping).
- **Codex (challenge mode, full context):** CONDITIONAL. All 3 V2/V3 findings ADDRESSED; 3 new findings — [MEDIUM] transport-neutrality mapping under non-default transports, [MEDIUM] handshake-per-invoke vs handshake-at-pairing wording ambiguity, [LOW] marker-as-heuristic clarification. All 3 folded into V4 as textual tightening.

## Acceptance for L0 closure

This packet is L0-approved when:

1. Devex + eng + cso unanimous APPROVE — confirmed in Cycle 3 against V3 (with V4 carrying their notes forward). **(V4)** Cycle 4 codex re-run is **required** to confirm V4 textual fold closes the 3 Cycle 3 wording findings. Eng / devex / cso don't need re-confirmation in Cycle 4 since V4 changes are pure textual tightening of surfaces they already approved. L0 closes when codex APPROVEs on V4.
2. W3-A/B/C Linear issue descriptions updated to reference this packet by URL for the lifted criteria.
3. Linear dependency edges added.
4. Any reviewer-surfaced changes folded back into this doc, dated.

W3-0 (DOS-563) may start in parallel with reviewer re-scheduling, scoped to the residual questions above. W3-A/B/C wait for L0 closure.
