---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 12
related_adrs: [0102, 0105, 0111, 0128, 0129, 0130]
open_questions: none — fixture catalog is closed; downstream consumers (W5-C, W2-A) add fixtures as they implement
---

# 12 — Negative fixture catalog for Phase 1

## Summary

Phase 1 needs explicit negative fixtures because the acceptance criteria name failure modes that can otherwise be missed by happy-path renderer tests. The WordPress Studio spike validates a trust boundary, not only a render path: the WP plugin is a `SurfaceClient`, the runtime owns `Composition`, provenance is canonical output, and WordPress is a projection target rather than the source of truth. These fixtures turn that contract into testable failure cases.

This catalog assumes the Wave 2 contracts are split as follows:

- Artifact 08: projection freshness, Composition schema compatibility, and partial projection behavior.
- Artifact 09: rate-limit axes and response envelope.
- Artifact 10: WordPress Abilities API, MCP Adapter discovery, and DailyOS ability allowlist.
- Artifact 11: SurfaceClient identity, WP user mapping, and user-presence nonce handling.
- Artifact 13: Gutenberg renderer, custom block fallback, and no-raw-payload rendering.
- Artifact 14: markdown projection persistence, snapshot integrity, and quarantine.
- Artifact 15: audit, diagnostics, observability, and Phase 1 verification harness.

The ADR anchors are ADR-0102 for ability policy, schema versioning, and soft degradation; ADR-0105 for provenance and revocation-sensitive rendering; ADR-0111 for `SurfaceClient` actor and scope enforcement; ADR-0128 for headless MCP posture and feedback-only writes; ADR-0129 for WordPress Abilities API plus MCP Adapter exposure policy; and ADR-0130 for substrate-owned `Composition`.

## Fixture execution contract

These fixtures should be implemented as negative tests against the boundary that fails, not only as end-to-end browser checks. Each fixture should have the narrowest test that proves the guard and at least one integration assertion that proves the user- or client-visible behavior.

Common fixture rules:

- Use deterministic clocks for version, nonce, rate-limit, and retry-window assertions.
- Use stable IDs for `composition_id`, `projection_id`, `surface_client_id`, `wp_user_id`, `site_id`, and `ability_name`.
- Seed one positive control when the fixture depends on a policy distinction.
- Assert both denial and non-mutation for write-adjacent failures.
- Assert response bodies do not leak hidden ability names, raw payloads, source excerpts, prompt text, local paths, or internal provenance trees.
- Assert the ability body was not invoked when the failure is expected at auth, discovery, allowlist, schema, or rate-limit boundaries.
- Assert audit or diagnostic events include enough context for operators to identify the failed boundary.
- Do not assert exact human-facing copy unless this artifact gives a recommended string; otherwise assert stable code/state plus absence of forbidden disclosure.

Common response-code expectations:

- Use `401` when the caller is not authenticated.
- Use `403` when the caller is authenticated but lacks WP capability, DailyOS scope, trusted client context, or matching nonce principal.
- Use `409` when the caller presents stale projection state or unsupported schema state that requires refresh/upgrade.
- Use `429` for deterministic rate-limit overflow and include the exhausted axis.
- Use `503` when the local adapter/runtime dependency is unavailable.

Common audit fields:

- `event_name`
- `surface_client_id`
- `site_id`
- `wp_user_id` when available
- `dailyos_principal_id` when available
- `ability_name` when the request reaches ability routing
- `composition_id` or `projection_id` when projection state is involved
- `axis` for rate-limit failures
- `reason_code`
- `request_id` or trace/span ID

Common non-leakage sentinels:

- `SECRET_CLAIM_NOTE_DO_NOT_RENDER`
- `RAW_SOURCE_EXCERPT_DO_NOT_RENDER`
- `PROVENANCE_CHILD_TREE_DO_NOT_RENDER`
- `PRIVATE_ABILITY_SCHEMA_DO_NOT_RENDER`
- `LOCAL_RUNTIME_PATH_DO_NOT_RENDER`

Every fixture should fail if any sentinel appears in DOM text, serialized Gutenberg attributes, REST JSON, MCP tool output, markdown snapshot output, browser console logs, or test-captured structured responses.

## Fixture table

| ID | Category | Trigger | Expected behavior | Verification approach | Related artifact |
|---|---|---|---|---|---|
| F-01 | data integrity | WP renders cached `Composition` v4 while runtime reports current composition watermark v5. | Renderer marks projection stale, blocks trusted writeback, requests refresh, and logs `projection.stale_detected`. | Assert UI stale state, HTTP refresh call, no feedback write accepted with stale watermark, audit row includes expected/current versions. | 08, 15, ADR-0130 |
| F-02 | trust boundary | Cached projection contains source attribution whose provenance mask was revoked after snapshot. | Revoked attribution is not rendered; block shows masked provenance state and refreshes projection. | Assert source text absent from DOM/REST/MCP response, warning `provenance_mask_revoked`, and new render uses masked provenance. | 08, 15, ADR-0105 |
| F-03 | data integrity | Markdown snapshot write for a Composition fails after partial file bytes are written. | Partial file is never promoted; temp file is cleaned or quarantined; existing good snapshot remains active. | Inject partial write, assert atomic-write error, active manifest still points to prior snapshot, quarantine entry exists. | 14, 15, ADR-0130 |
| F-04 | rendering | WP plugin receives `Composition.schema_version` newer than supported renderer schema. | Renderer refuses trusted render, shows upgrade-required fallback, and does not expose unknown payload fields. | Feed schema v3 to v2 plugin, assert 409/unsupported schema envelope and DOM contains no raw JSON. | 08, 13, ADR-0102, ADR-0130 |
| F-05 | rendering | Runtime returns a Composition with resolved blocks and failed optional blocks. | Resolved blocks render; failed blocks render bounded error placeholders; provenance warnings identify failures. | Assert successful blocks visible, failed block placeholders visible, no whole-page hard failure, warnings include block IDs. | 08, 13, 15, ADR-0102 |
| F-06 | discovery | WP plugin cannot reach local MCP Adapter or adapter returns discovery error. | Plugin degrades to local WP-only unavailable state; it does not enumerate stale DailyOS abilities as live. | Simulate connection refusal/500, assert UI status, no ability list cached as current, audit `adapter.discovery_failed`. | 10, 15, ADR-0129 |
| F-07 | discovery | Default WP MCP server attempts to list `dailyos/*` abilities for a remote MCP client. | Denied at DailyOS ability allowlist boundary before schemas or names leak. | Call default MCP `list_tools`, assert no `dailyos/*`, deny log reason `mcp_exposure_denied`, no ability body invoked. | 10, 15, ADR-0102, ADR-0129 |
| F-08a | discovery | Unauthenticated browser/REST request attempts to enumerate DailyOS abilities. | 401 response, empty ability list, no schema metadata. | Assert status 401, response has no names/descriptions/schemas, audit actor `anonymous`. | 10, 11, 15, ADR-0111 |
| F-08b | discovery | Authenticated WP user with Subscriber role attempts ability enumeration. | 403 response, empty ability list, no schema metadata. | Assert status 403, response has no ability metadata, audit reason `wp_capability_denied`. | 10, 11, 15, ADR-0111 |
| F-08c | discovery | Editor-role WP user enumerates abilities beyond their DailyOS scope grants. | Only allowed scoped abilities appear; out-of-scope ability names and schemas are absent. | Seed mixed allowlist, assert response equals allowed subset and deny log for omitted scope. | 10, 11, 15, ADR-0102, ADR-0111 |
| F-08d | discovery | Admin-side JS not on the DailyOS script allowlist calls ability enumeration. | Request is denied even though it runs in wp-admin; admin JS context is not trusted by location. | Execute from non-allowlisted script handle, assert 403 and no ability metadata; audit script handle. | 10, 11, 15, ADR-0111 |
| F-09 | rendering | Composition references unknown `BlockType::Custom("vendor/private")`. | Renderer uses generic privacy-safe fallback; raw payload, claim secrets, and provenance internals are not disclosed. | Assert fallback UI exists, raw payload keys/values absent, provenance rendered only through approved mask. | 13, 15, ADR-0130 |
| F-10 | trust boundary | User B submits user A's single-use presence nonce with a feedback write. | Write is rejected; nonce remains bound to user A and is not consumed for B. | Assert 403/invalid nonce, no feedback event, nonce ledger still rejects replay by A if previously used only when appropriate. | 11, 15, ADR-0111, ADR-0128 |
| F-11a | rate limiting | One SurfaceClient instance exceeds request budget. | 429 with `axis=surface_client`; other SurfaceClients for same site are not blocked unless site budget is also exhausted. | Burst from one instance, assert 429 envelope, retry-after, and unaffected second instance. | 09, 15, ADR-0111 |
| F-11b | rate limiting | One WP user exceeds per-user budget across SurfaceClients. | 429 with `axis=wp_user`; same SurfaceClient can serve a different user if other axes permit. | Burst as one WP user, assert axis and unaffected second user. | 09, 15, ADR-0111 |
| F-11c | rate limiting | Aggregate site traffic exceeds per-site budget. | 429 with `axis=site`; all users/SurfaceClients on that WP site are constrained until reset. | Generate distributed traffic, assert site-axis 429 and shared reset window. | 09, 15, ADR-0111 |
| F-11d | rate limiting | Calls to one ability exceed per-ability budget. | 429 with `axis=ability`; unrelated abilities remain callable if other axes allow. | Burst `dailyos/render_composition`, assert ability-axis 429 and success on different ability. | 09, 15, ADR-0102 |
| F-11e | rate limiting | Requests under one scope exceed per-scope budget. | 429 with `axis=scope`; other scopes granted to the same client remain available if budgets allow. | Burst `read.account_overview`, assert scope-axis 429 and success on `read.briefing`. | 09, 15, ADR-0102, ADR-0111 |
| F-12 | tamper | Markdown snapshot is edited directly on disk outside runtime mutation path. | Tamper is detected on next read; snapshot is quarantined; canonical runtime state is not mutated. | Modify bytes under DailyOS sentinel, assert verification failure, quarantine record, and projection not promoted. | 14, 15, ADR-0105, ADR-0130 |

### F-01 — Stale projection

- **Category:** data integrity
- **Trigger:** Runtime stores `Composition{id=comp_daily_001, composition_version=5}` with current watermark `{composition_version: 5, produced_at: 2026-05-10T14:00:00Z}`. The WP plugin has cached projection metadata for the same composition with `{composition_version: 4, produced_at: 2026-05-10T13:55:00Z}`. User opens the corresponding Gutenberg page or attempts feedback against a block from v4.
- **Expected behavior:** The renderer must render the cached projection only as stale, never as current trusted state. Any write or feedback attempt carrying v4 watermarks fails with a conflict-style response, recommended code `409 StaleProjection`, including `expected_version=4` and `current_version=5`. The UI message should be: `This DailyOS block is out of date. Refresh before sending feedback.` Audit logs include `projection.stale_detected` with `surface_client_id`, `composition_id`, `cached_version`, and `current_version`.
- **Verification approach:** Seed runtime composition ledger with v5 and WP cache with v4. Load the WP page and assert the block has stale styling/state, no normal trusted provenance affordance, and a refresh request is issued. Submit feedback from the stale block and assert HTTP 409, no feedback claim/event row, and a log line or audit row with the version mismatch.
- **Related Wave 2 artifact:** 08 projection freshness contract; 15 diagnostics; ADR-0130 Composition ownership.
- **Setup notes:** Fixture data needs two serialized `Composition` envelopes with same `composition_id` and different `composition_version`; runtime should expose a deterministic current-watermark endpoint.
- **Minimum assertions:**
  - Cached v4 body may be visible only with stale treatment.
  - Any write carrying v4 watermark is rejected before feedback handling.
  - Refresh response returns or points to v5.
- **Negative control:** If cached version equals current version, normal render and feedback flow may proceed.
- **Regression risk:** A renderer that checks only `composition_id` and ignores version will pass happy-path render tests but fail this fixture.

### F-02 — Revoked provenance mask

- **Category:** trust boundary
- **Trigger:** A cached WP projection rendered source attribution for `source_ref=src_redacted_001`. After the projection was cached, the runtime updates the provenance render policy so that source is masked or revoked for the WordPress SurfaceClient. User reloads the WP page from cache before refresh completes.
- **Expected behavior:** The renderer must not display the previously rendered source attribution once revocation is known. If the cache contains rendered text, it is replaced with a masked provenance affordance and warning state. Recommended warning code: `provenance_mask_revoked`. The renderer may show `Source attribution is no longer available for this surface.` It must not show source title, URL, author, excerpt, internal identifiers beyond an opaque reference, or field attribution explanation.
- **Verification approach:** Persist a projection that includes visible attribution text, then seed runtime policy with `MaskReason::Revoked` for that source. Load projection through the renderer path that performs provenance policy check. Assert the old attribution text is absent from DOM, REST response, MCP Adapter response, and any block attributes returned to browser JS. Assert warning exists in rendered provenance and audit records the revocation.
- **Related Wave 2 artifact:** 08 projection invalidation; 15 observability; ADR-0105 provenance mask semantics.
- **Setup notes:** Include one source that remains renderable and one revoked source, to assert the renderer masks selectively rather than hiding the whole block.
- **Minimum assertions:**
  - Revoked source attribution string is absent from all serialized surfaces.
  - Non-revoked attribution remains visible when policy allows.
  - Rendered warning includes a mask reason but no private source details.
- **Negative control:** A provenance mask changed for a different source must not hide this block's allowed attribution.
- **Regression risk:** Storing rendered attribution as inert HTML in WP can bypass later mask revocation unless the renderer revalidates before display.

### F-03 — Markdown write failure

- **Category:** data integrity
- **Trigger:** The runtime renders a `Composition` to a markdown snapshot and attempts to persist it. The write succeeds for the temp file header and part of the block payload, then the filesystem returns `ENOSPC`, `EIO`, or permission denied before fsync/rename.
- **Expected behavior:** The partial file must not become the active snapshot. If an older valid snapshot exists, the manifest keeps pointing to it. If no older valid snapshot exists, the snapshot state becomes unavailable with `snapshot_write_failed`; it must not point at partial bytes. The partial temp file is deleted if safe or moved to quarantine with reason `partial_write`. The runtime emits a structured error and does not update projection ledger signatures for the failed write.
- **Verification approach:** Use a filesystem adapter fixture that fails after N bytes. Assert the active snapshot manifest is unchanged, no new successful projection ledger row exists, and a quarantine or cleanup record is present. Read the markdown projection afterward and assert it returns the previous good snapshot or a clear unavailable response, never the partial content.
- **Related Wave 2 artifact:** 14 markdown persistence; 15 diagnostics; ADR-0130 Composition projection.
- **Setup notes:** Run two variants: existing good snapshot present and first snapshot attempt with no prior snapshot.
- **Minimum assertions:**
  - Manifest update happens only after full write, fsync, signature, and atomic rename succeed.
  - Partial bytes cannot be read through the normal snapshot reader.
  - Error telemetry preserves enough context to diagnose disk failure without logging full payload.
- **Negative control:** A successful write with the same fixture data must advance manifest and ledger exactly once.
- **Regression risk:** Updating the manifest before file commit can make partial markdown appear authoritative after restart.

### F-04 — Schema bump

- **Category:** rendering
- **Trigger:** WP plugin supports `composition_schema_version=2`. Runtime returns `composition_schema_version=3` for `Composition{id=comp_account_002}` with new block payload fields unknown to the plugin. The plugin receives the response through the local transport or reads a stored snapshot produced by a newer runtime.
- **Expected behavior:** The plugin must not attempt a trusted normal render. It returns or displays an upgrade-required state, recommended code `409 UnsupportedCompositionSchema`, with supported and received schema versions. It may render a minimal safe shell using known top-level metadata (`kind`, `generated_at`) but must not display raw unknown payload fields or provenance internals.
- **Verification approach:** Feed a schema v3 composition containing sentinel secret-like unknown fields such as `internal_claim_notes` and `provenance.children[0].sources`. Assert the DOM and REST output contain the upgrade message and supported/received numbers, and do not contain sentinel values. Assert no feedback controls are enabled for unknown blocks.
- **Related Wave 2 artifact:** 08 schema compatibility; 13 renderer fallback; ADR-0102 schema versioning; ADR-0130 Composition model.
- **Setup notes:** Fixture should include both additive unknown fields and one unknown block type so implementers verify schema handling occurs before block fallback.
- **Minimum assertions:**
  - Unsupported schema is detected before block payload interpretation.
  - Response names supported and received schema versions.
  - Unknown fields are neither rendered nor copied into client-visible block attributes.
- **Negative control:** A supported schema with an unknown custom block should use F-09 fallback, not schema-level refusal.
- **Regression risk:** Treating newer schema as "best effort JSON" can leak fields the older renderer does not know are sensitive.

### F-05 — Partial projection failure

- **Category:** rendering
- **Trigger:** Runtime returns `Ok(AbilityOutput<Composition>)` with three blocks: block A resolved, block B failed as optional with warning `ChildAbilityTimeout`, and block C resolved. The Composition includes per-block diagnostics naming failed block B and the child ability that failed.
- **Expected behavior:** The renderer must show blocks A and C normally, show a bounded placeholder for block B, and surface the warning without turning the whole Composition into a hard error. Recommended UI text for B: `DailyOS could not render this block right now.` The placeholder must not include child ability raw error stacks, local paths, prompts, source excerpts, or unmasked provenance.
- **Verification approach:** Load fixture Composition and assert resolved block content appears in correct order, failed block placeholder appears at block B position, page status is degraded not failed, and provenance/warnings include block ID and error code. Assert no raw error stack or source internals are present.
- **Related Wave 2 artifact:** 08 partial projection behavior; 13 rendering; 15 observability; ADR-0102 soft degradation.
- **Setup notes:** Include a second variant where a required block fails and the ability returns hard error, to distinguish optional soft degradation from full failure.
- **Minimum assertions:**
  - Resolved blocks preserve original section and block ordering.
  - Failed optional block is represented as a bounded placeholder.
  - Provenance warnings are visible through the approved warning surface.
- **Negative control:** If all child blocks resolve, the degraded page status and placeholder must be absent.
- **Regression risk:** Failing the whole page on optional block failure makes the renderer brittle; hiding the failed block silently violates the soft-degradation contract.

### F-06 — MCP Adapter discovery failure

- **Category:** discovery
- **Trigger:** The WP plugin attempts to discover DailyOS-backed abilities through the WP Abilities API/MCP Adapter integration, but the local adapter endpoint is unreachable, times out, or returns malformed discovery JSON. This happens during plugin activation and again during normal admin page load.
- **Expected behavior:** The plugin reports DailyOS availability as degraded and does not present stale cached abilities as currently invokable. It may keep the last known list for diagnostics only if marked `stale`. Recommended status code for plugin REST endpoint: `503 DailyOSAdapterUnavailable`. Audit logs include `adapter.discovery_failed` with transport, timeout/error class, and SurfaceClient instance identity.
- **Verification approach:** Stub the adapter to connection-refused, timeout, 500, and invalid JSON. Assert admin UI shows unavailable status, ability execution buttons are disabled, enumeration endpoints return 503 or empty stale-marked results, and no ability invocation reaches the runtime.
- **Related Wave 2 artifact:** 10 MCP Adapter discovery; 15 diagnostics; ADR-0129 WP path.
- **Setup notes:** Include a cached prior list containing `dailyos/briefing` to prove the plugin does not silently reuse it as live.
- **Minimum assertions:**
  - Discovery failure is visible to the admin/status surface.
  - Cached ability lists are marked stale and not executable as live.
  - Adapter errors do not trigger direct PHP-to-runtime ability bypasses.
- **Negative control:** When adapter discovery succeeds, the same cached list can be replaced and marked current.
- **Regression risk:** "Last known good" ability lists are useful for diagnostics but dangerous if the plugin treats them as active capability grants.

### F-07 — Default WP MCP server listing `dailyos/*` attempt

- **Category:** discovery
- **Trigger:** A remote MCP client connects to the default WordPress MCP server and calls tool discovery. DailyOS-backed WP abilities are registered in WordPress but are intended only for the custom DailyOS MCP server configuration with explicit allowlist and low-capability WP user.
- **Expected behavior:** Default MCP discovery must not list `dailyos/*` tools. The denial happens at the abilities allowlist/`mcp_exposure` boundary before tool names, descriptions, input schemas, or annotations are returned. Recommended audit reason: `mcp_exposure_denied_default_server`.
- **Verification approach:** Register test abilities `dailyos/briefing`, `dailyos/feedback`, and ordinary WP ability `wp/list_posts`. Call default MCP `list_tools`. Assert `wp/list_posts` can appear but no `dailyos/*` names or schemas appear. Attempt direct invocation by guessed `dailyos/briefing` name and assert 403/404 without ability body invocation.
- **Related Wave 2 artifact:** 10 allowlist; 15 audit; ADR-0102 `mcp_exposure`; ADR-0129 MCP Adapter exposure policy.
- **Setup notes:** Also test the custom DailyOS MCP server path to prove allowlisted DailyOS abilities appear only there.
- **Minimum assertions:**
  - Default server discovery response contains zero `dailyos/*` tools.
  - Guessed direct invocation is denied before registry body execution.
  - Denial does not reveal whether the guessed ability exists.
- **Negative control:** Custom DailyOS MCP server with explicit allowlist may list the same ability when policy permits it.
- **Regression risk:** Relying only on WordPress capability callbacks can expose model-facing DailyOS schemas through the default MCP Adapter path.

### F-08a — Unauthenticated ability discovery leakage

- **Category:** discovery
- **Trigger:** A browser, REST client, or MCP-like caller without WP authentication requests the DailyOS ability enumeration endpoint.
- **Expected behavior:** Response is `401 Unauthorized` with no DailyOS ability names, descriptions, categories, scopes, schemas, or count of hidden abilities. Audit logs actor as `anonymous` with reason `authentication_required`.
- **Verification approach:** Call each enumeration path without cookies, application password, nonce, or pairing token. Assert body contains only generic error fields and no sentinel ability names seeded in the registry. Assert no substrate registry enumeration call is made if the WP auth layer fails first.
- **Related Wave 2 artifact:** 10 discovery; 11 identity; 15 audit; ADR-0111 SurfaceClient actor.
- **Setup notes:** Seed one ability with a sensitive name like `dailyos/private_claim_repair` to make leakage obvious.
- **Minimum assertions:**
  - HTTP status is 401, not a 200 with empty list.
  - Response body has no total count of hidden abilities.
  - Logs identify anonymous caller without logging credentials or cookies.
- **Negative control:** The same route with valid authenticated and scoped context should proceed to scope filtering.
- **Regression risk:** Returning "empty list" to unauthenticated callers can mask a missing auth check while still leaking endpoint existence and timing.

### F-08b — Subscriber-role WP user enumerates abilities

- **Category:** discovery
- **Trigger:** Authenticated WP user with role `subscriber` and no DailyOS management capability calls the ability enumeration endpoint from wp-admin or REST.
- **Expected behavior:** Response is `403 Forbidden`, empty ability metadata, and no automatic trust from WP login alone. Audit reason is `wp_capability_denied`; the log includes WP user ID and SurfaceClient site ID.
- **Verification approach:** Create subscriber user, attach valid WP nonce, and call enumeration. Assert 403 and absence of ability names/schemas. Assert no DailyOS scope lookup is enough to override missing WP capability.
- **Related Wave 2 artifact:** 10 discovery; 11 WP user mapping; 15 audit; ADR-0111 two-layer enforcement.
- **Setup notes:** Include a paired SurfaceClient with scopes to prove WP role and DailyOS scope must both pass.
- **Minimum assertions:**
  - WP capability failure wins even if the SurfaceClient has DailyOS scopes.
  - Response has no ability names, descriptions, schemas, or omitted-scope hints.
  - Audit connects denial to WP role/capability, not only DailyOS scope.
- **Negative control:** Promoting the same user to the minimum allowed WP capability should move the request to DailyOS scope filtering.
- **Regression risk:** Treating pairing as sufficient authorization lets low-privilege WP accounts enumerate substrate capabilities.

### F-08c — Editor-role WP user enumerates beyond allowed scope

- **Category:** discovery
- **Trigger:** WP user with Editor role and DailyOS grants `read.briefing` calls enumeration when registry includes `dailyos/briefing` requiring `read.briefing`, `dailyos/account_overview` requiring `read.account_overview`, and `dailyos/submit_feedback` requiring `submit.feedback`.
- **Expected behavior:** Response contains only abilities whose required scopes are fully granted and MCP exposure rules match the path. Out-of-scope ability names, descriptions, schemas, and annotations are absent. Audit may record omitted scopes at debug level but must not return them to caller.
- **Verification approach:** Assert response set exactly equals allowed ability IDs. Search serialized response for sentinel names/descriptions of out-of-scope abilities. Attempt guessed invocation of each omitted ability and assert 403 with no body invocation.
- **Related Wave 2 artifact:** 10 scoped discovery; 11 scope grants; 15 audit; ADR-0102 `required_scopes`.
- **Setup notes:** Use at least one ability requiring multiple scopes to assert all required scopes must be present.
- **Minimum assertions:**
  - Allowed subset is exact, not a superset with disabled entries.
  - Out-of-scope schemas are absent, not redacted in place.
  - Direct invocation of omitted abilities is denied consistently with discovery.
- **Negative control:** Adding the missing scope grant should cause the ability to appear without code changes.
- **Regression risk:** Showing disabled out-of-scope abilities still leaks product capability names and input schemas.

### F-08d — Non-allowlisted admin-side JS enumerates abilities

- **Category:** discovery
- **Trigger:** A plugin or injected script running in wp-admin calls the DailyOS client-side ability discovery API. The WP user is an administrator, but the script handle/origin is not registered as a DailyOS admin script allowed to call discovery.
- **Expected behavior:** Request is denied with `403 AdminScriptNotTrusted`. Admin page context, administrator role, and valid WP nonce are not sufficient. No ability names or schemas are returned. Audit includes `script_handle`, `wp_user_id`, and reason `client_script_not_allowlisted`.
- **Verification approach:** Enqueue a test admin script with a valid nonce but without DailyOS script registration and call discovery. Assert 403 and no metadata. Repeat with the official DailyOS script handle and assert normal scoped behavior to prove the guard is script-specific.
- **Related Wave 2 artifact:** 10 client-side discovery; 11 identity and nonce context; 15 audit; ADR-0111 SurfaceClient is not ambient wp-admin.
- **Setup notes:** Fixture should cover both REST and `executeAbility()`-style client paths if both exist in Phase 1.
- **Minimum assertions:**
  - Admin URL, admin role, and valid nonce do not bypass script allowlist.
  - Denial records the untrusted script handle or equivalent client identifier.
  - Official DailyOS script path still applies normal actor/scope filtering.
- **Negative control:** Same WP user and scope from allowlisted DailyOS script should not be denied by this fixture.
- **Regression risk:** Assuming "wp-admin means trusted" lets unrelated admin plugins enumerate DailyOS abilities from browser context.

### F-09 — Custom block fallback projection

- **Category:** rendering
- **Trigger:** Runtime returns a `Composition` containing `BlockType::Custom("vendor/private_forecast")` with payload keys including `summary`, `claim_secret`, `raw_source_excerpt`, and `provenance_internal`. The WP renderer does not know this custom block type.
- **Expected behavior:** Renderer projects the block through a generic privacy-safe fallback, for example a claim summary shell with `rendered_as=fallback`. It must not disclose raw payload JSON, claim secrets, unmasked source excerpts, prompt text, field attribution internals, or provenance child tree. It may render safe fields explicitly allowed by the fallback sanitizer, such as a sanitized title or high-level unavailable message.
- **Verification approach:** Render the composition and assert fallback block appears with generic type and safe label. Search DOM, serialized block attributes, REST response, and MCP response for sentinel values from forbidden fields. Assert provenance is rendered only through the approved provenance renderer/mask and not as raw `ProvenanceEnvelope`.
- **Related Wave 2 artifact:** 13 block fallback; 15 verification; ADR-0130 custom block policy.
- **Setup notes:** Include one known block before and after the custom block to assert fallback does not drop neighboring content.
- **Minimum assertions:**
  - Fallback block is visible enough that the user knows content was not fully rendered.
  - Raw custom payload is absent from every client-visible serialization.
  - Claim refs are retained internally for feedback only if sanitizer allows safe mapping.
- **Negative control:** A known block type with safe payload should render normally and not use fallback.
- **Regression risk:** Dumping unknown payload JSON is the easiest implementation path and the primary disclosure failure this fixture prevents.

### F-10 — Multi-user nonce reuse

- **Category:** trust boundary
- **Trigger:** User A receives a single-use presence nonce for a feedback action on claim `claim_001`. User B, authenticated as a different WP user and mapped to a different DailyOS user or principal, submits that nonce for the same claim feedback endpoint.
- **Expected behavior:** The write is rejected with `403 PresenceNoncePrincipalMismatch` or equivalent. No feedback event, correction, corroboration, dismissal, or claim mutation is recorded. The audit log includes nonce ID, issuing user, presenting user, SurfaceClient ID, and claim ID. The nonce is not consumed as a successful B write.
- **Verification approach:** Mint nonce for A, submit as B, assert 403 and no domain writes. Then submit as A within validity window if the nonce was not marked compromised, or assert a deliberate compromised state if the implementation burns mismatched nonces. In either policy, B must not succeed and logs must identify mismatch.
- **Related Wave 2 artifact:** 11 presence proof; 15 audit; ADR-0111 user-presence requirement; ADR-0128 feedback-only writes.
- **Setup notes:** Include variants for same WP site/different WP users and different SurfaceClient instances to ensure the binding includes user and instance.
- **Minimum assertions:**
  - Nonce validation binds nonce to WP user, DailyOS principal, SurfaceClient instance, action, and expiry.
  - Rejected B submission creates no feedback event and no claim mutation.
  - Audit is specific enough for forensic trace without exposing nonce secret.
- **Negative control:** User A using the nonce for the bound action within expiry should succeed once.
- **Regression risk:** A nonce bound only to site or session becomes a bearer token for any user who can read it.

### F-11a — Rate-limit overflow per SurfaceClient

- **Category:** rate limiting
- **Trigger:** SurfaceClient instance `wp_site_001/install_a` exceeds its request budget for ability invocation or discovery within the configured window while `wp_site_001/install_b` remains under budget.
- **Expected behavior:** Overflow request returns `429 Too Many Requests` with a structured envelope containing `axis=surface_client`, `surface_client_id`, `limit`, `window`, and `retry_after`. Other SurfaceClient instances are not blocked by this axis alone.
- **Verification approach:** Configure low limit, send requests from install A until overflow, assert 429 axis and retry metadata. Immediately send request from install B and assert success unless another axis is exhausted. Verify metric increments `rate_limit.blocked.surface_client`.
- **Related Wave 2 artifact:** 09 rate limits; 15 metrics; ADR-0111 SurfaceClient identity.
- **Setup notes:** Use deterministic clock so reset-window assertions are stable.
- **Minimum assertions:**
  - Response envelope names `axis=surface_client`.
  - Retry window is deterministic and matches limiter state.
  - Ability body is not invoked for the overflow request.
- **Negative control:** Different SurfaceClient under the same site should remain allowed until its own or shared site budget is exhausted.
- **Regression risk:** Collapsing SurfaceClient and site identity makes one noisy install block every install.

### F-11b — Rate-limit overflow per WP user

- **Category:** rate limiting
- **Trigger:** WP user `editor_1` exceeds per-user request budget across one or more SurfaceClient sessions, while `editor_2` is below budget.
- **Expected behavior:** Overflow returns `429 Too Many Requests` with `axis=wp_user` and the WP user identifier or opaque user ref. The same SurfaceClient can still serve `editor_2` if SurfaceClient/site/ability/scope budgets permit.
- **Verification approach:** Send requests as `editor_1` until overflow, assert 429 body reports `wp_user`. Then send equivalent request as `editor_2` and assert success. Verify audit includes WP user and SurfaceClient mapping.
- **Related Wave 2 artifact:** 09 rate limits; 11 user mapping; 15 metrics.
- **Setup notes:** Include a cross-tab variant where the same WP user consumes budget from two browser sessions.
- **Minimum assertions:**
  - Response envelope names `axis=wp_user`.
  - Same user across tabs/sessions shares budget.
  - Different user on same SurfaceClient is not blocked by this axis alone.
- **Negative control:** After reset window, `editor_1` can call again without changing identity grants.
- **Regression risk:** Keying only by browser session lets users bypass the per-user budget by opening new sessions.

### F-11c — Rate-limit overflow per site

- **Category:** rate limiting
- **Trigger:** Aggregate traffic from multiple WP users and SurfaceClient sessions under site `wp_site_001` exceeds the per-site budget.
- **Expected behavior:** Overflow returns `429 Too Many Requests` with `axis=site`, `site_id`, `limit`, `window`, and `retry_after`. All callers under that site receive the site-axis limit until reset, even if their individual budgets are not exhausted.
- **Verification approach:** Generate distributed requests below individual budgets but above aggregate site budget. Assert 429 for final requests across users and SurfaceClients, and assert a different site remains unaffected. Verify metric `rate_limit.blocked.site`.
- **Related Wave 2 artifact:** 09 rate limits; 15 metrics; ADR-0111 instance attribution.
- **Setup notes:** Site ID should be the DailyOS SurfaceClient site identity, not only WordPress `home_url`, so restores/clones can be distinguished.
- **Minimum assertions:**
  - Response envelope names `axis=site`.
  - Multiple users contribute to the same site bucket.
  - A distinct site identity remains available under the same runtime.
- **Negative control:** Distributed traffic below aggregate site budget should not produce site-axis 429s.
- **Regression risk:** Missing site-level limiting leaves the local runtime vulnerable to aggregate load even when user/client budgets are respected.

### F-11d — Rate-limit overflow per ability

- **Category:** rate limiting
- **Trigger:** Calls to `dailyos/render_composition` exceed its per-ability budget, while `dailyos/list_allowed_abilities` and `dailyos/submit_feedback` remain below their per-ability budgets.
- **Expected behavior:** Overflow returns `429 Too Many Requests` with `axis=ability` and `ability_name=dailyos/render_composition`. The response must not imply the user or site is globally blocked. Other abilities remain callable if other axes permit.
- **Verification approach:** Burst one ability until overflow, assert ability-axis envelope. Call another allowlisted ability with same user/client and assert success. Verify ability body is not invoked after the rate limiter denies.
- **Related Wave 2 artifact:** 09 rate limits; 15 metrics; ADR-0102 registry boundary.
- **Setup notes:** Ability name in logs should use canonical registry name, not route path alias.
- **Minimum assertions:**
  - Response envelope names `axis=ability`.
  - Limiter key uses canonical ability name.
  - Unrelated ability remains callable under same caller context.
- **Negative control:** Same traffic spread across different abilities should not exhaust the per-ability bucket unless another axis triggers.
- **Regression risk:** Without per-ability limiting, one expensive composition ability can starve cheap discovery or feedback paths.

### F-11e — Rate-limit overflow per scope

- **Category:** rate limiting
- **Trigger:** Requests under scope `read.account_overview` exceed the per-scope budget for a SurfaceClient, while scope `read.briefing` is below budget for the same client and user.
- **Expected behavior:** Overflow returns `429 Too Many Requests` with `axis=scope` and `scope=read.account_overview`. The response includes retry timing and does not expose hidden scopes. Other scopes remain available if their budgets and higher axes permit.
- **Verification approach:** Configure low scope budget, call abilities mapped to `read.account_overview` until overflow, assert scope-axis 429. Call an ability mapped to `read.briefing` and assert success. Verify logs record scope but response only names the exhausted scope relevant to caller.
- **Related Wave 2 artifact:** 09 rate limits; 11 scope grants; 15 metrics; ADR-0102 `required_scopes`.
- **Setup notes:** Include one ability requiring two scopes and define expected accounting before implementation; Phase 1 should charge all required scopes unless artifact 09 says otherwise.
- **Minimum assertions:**
  - Response envelope names `axis=scope`.
  - Exhausted scope is one the caller is allowed to know about.
  - Other scopes for same user/client remain callable if higher axes permit.
- **Negative control:** Ability under a non-exhausted scope succeeds with same caller and site.
- **Regression risk:** Scope limits are the only way to dampen a narrow class of reads without disabling the whole SurfaceClient.

### F-12 — Tamper detection for out-of-band markdown edit

- **Category:** tamper
- **Trigger:** Runtime writes a signed markdown snapshot for `Composition{id=comp_report_001}` under DailyOS projection sentinels. A user, editor, sync tool, or attacker edits bytes inside the DailyOS-owned region directly on disk, bypassing runtime mutation path. The next read comes from WP sync, CLI render, or runtime reconciliation.
- **Expected behavior:** Signature or manifest verification fails with `ProjectionTampered` or more specific `SignatureMismatch`. The snapshot is quarantined with original path, observed hash, expected hash/signature ID, and detection source. The runtime does not promote the edited markdown into canonical claims, provenance, or Composition state. UI shows a tamper banner if rendered: `This DailyOS projection was changed outside the runtime. DailyOS is reconciling it.`
- **Verification approach:** Write valid signed snapshot, alter one byte inside sentinel, then invoke next-read path. Assert verification failure, quarantine record, projection divergence/audit event, and no canonical state mutation. Assert subsequent render uses last known good snapshot or safe unavailable state according to artifact 14 policy.
- **Related Wave 2 artifact:** 14 markdown quarantine; 15 audit; ADR-0105 provenance integrity; ADR-0130 Composition projection.
- **Setup notes:** Include variants for edited payload, removed signature comment, copied signature from another block, and edit outside DailyOS sentinel; the last variant should not be treated as DailyOS projection tamper.
- **Minimum assertions:**
  - Verification compares observed bytes to signed/ledgered canonical projection bytes.
  - Quarantine preserves enough metadata for diff and repair.
  - Canonical claims and Composition state remain unchanged after detection.
- **Negative control:** User-authored markdown outside DailyOS sentinels is allowed and must not create a tamper event.
- **Regression risk:** Treating markdown as canonical input lets out-of-band edits bypass the runtime's provenance and trust boundary.

## Coverage matrix

| Artifact / ADR | Fixtures |
|---|---|
| Wave 2 artifact 08 - projection freshness, schema, partial projection | F-01, F-02, F-04, F-05 |
| Wave 2 artifact 09 - rate-limit axes | F-11a, F-11b, F-11c, F-11d, F-11e |
| Wave 2 artifact 10 - WP Abilities API, MCP Adapter, allowlist discovery | F-06, F-07, F-08a, F-08b, F-08c, F-08d |
| Wave 2 artifact 11 - identity, scope grants, presence nonces | F-08a, F-08b, F-08c, F-08d, F-10, F-11b, F-11e |
| Wave 2 artifact 13 - renderer and fallback projection | F-04, F-05, F-09 |
| Wave 2 artifact 14 - markdown snapshot persistence and quarantine | F-03, F-12 |
| Wave 2 artifact 15 - audit, metrics, diagnostics, test harness | F-01, F-02, F-03, F-05, F-06, F-07, F-08a, F-08b, F-08c, F-08d, F-09, F-10, F-11a, F-11b, F-11c, F-11d, F-11e, F-12 |
| ADR-0102 - abilities as runtime contract | F-04, F-05, F-07, F-08c, F-11d, F-11e |
| ADR-0105 - provenance as first-class output | F-02, F-12 |
| ADR-0111 - SurfaceClient invocation and scopes | F-07, F-08a, F-08b, F-08c, F-08d, F-10, F-11a, F-11b, F-11c, F-11e |
| ADR-0128 - headless MCP as product surface | F-10 |
| ADR-0129 - WordPress Studio as primary surface | F-06, F-07 |
| ADR-0130 - surface-independent Composition contract | F-01, F-03, F-04, F-09, F-12 |

## Out of scope

- Kernel-level compromise, root filesystem compromise, or malicious hypervisor behavior. These belong in threat model and operational hardening work; Phase 1 accepts them as residual for the local spike.
- A malicious WordPress core build or compromised PHP runtime that lies about request identity before DailyOS code runs. This belongs in supply-chain and distribution threat modeling, not renderer fixture tests.
- Full cryptographic key compromise and rotation workflows beyond detection of revoked masks or invalid signatures. Key compromise re-sign and recovery belongs in projection signing/key-management artifacts.
- Cross-site request forgery coverage for ordinary WordPress endpoints except where it intersects DailyOS discovery and presence nonces. General WP CSRF testing belongs in plugin security tests.
- Host-model prompt injection through MCP tool descriptions. This catalog covers discovery and exposure denial; prompt-injection behavior belongs in ability eval and MCP tool-description review.
- Performance load testing beyond deterministic rate-limit overflow. Capacity, latency, and soak testing belong in benchmark/canary artifacts.
- Data-loss recovery after disk failure. This catalog covers partial write safety and quarantine; backup/restore policy belongs in persistence operations work.
- Multi-tenant hosted DailyOS behavior. DOS-546 Phase 1 validates local WordPress Studio SurfaceClient behavior; hosted-team isolation is future paid-tier architecture.
