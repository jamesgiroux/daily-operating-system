Verdict: CONDITIONAL APPROVE

## 1. PHP Single-Fetch Refactor

Answer: feasible, with contract made explicit.

Evidence:
- Block registration enumerates every `blocks/*/block.json` and registers each metadata package, so `account-overview/block.json` continues to own the render entrypoint. `wp/dailyos/includes/class-dailyos-plugin.php:154`; `wp/dailyos/includes/class-dailyos-plugin.php:161`
- The block declares `"render": "file:./render.php"`, and `render.php` still delegates to `dailyos_account_overview_render($attributes)`. `wp/dailyos/blocks/account-overview/block.json:22`; `wp/dailyos/blocks/account-overview/render.php:20`; `wp/dailyos/blocks/account-overview/render.php:25`
- `dailyos_account_overview_render` currently validates `composition_id`, fetches the runtime client by filter, calls `project_composition_for_surface`, then renders the projection. `wp/dailyos/blocks/account-overview/render-functions.php:32`; `wp/dailyos/blocks/account-overview/render-functions.php:43`; `wp/dailyos/blocks/account-overview/render-functions.php:55`; `wp/dailyos/blocks/account-overview/render-functions.php:65`
- `account_overview_preview` already performs the first runtime call, then builds attributes and re-enters the render path through `render_block_with_filter`. `wp/dailyos/includes/class-dailyos-plugin.php:587`; `wp/dailyos/includes/class-dailyos-plugin.php:598`; `wp/dailyos/includes/class-dailyos-plugin.php:612`
- `render_block_with_filter` injects the same client and calls `dailyos_account_overview_render`, causing the second runtime call. `wp/dailyos/includes/class-dailyos-plugin.php:682`; `wp/dailyos/includes/class-dailyos-plugin.php:686`; `wp/dailyos/includes/class-dailyos-plugin.php:690`
- The pure helper can consume the existing success envelope: top-level `projection`, optional `cache_hint_token`, optional `served_from_cache`. `src-tauri/src/surface_runtime/mod.rs:2330`; `src-tauri/src/surface_runtime/mod.rs:2335`; `src-tauri/src/surface_runtime/mod.rs:2336`; `src-tauri/src/surface_runtime/mod.rs:2337`
- The `projection` contract is `composition_id`, optional `composition_version`, `fallback_policy_version`, `blocks`, diagnostics/count fields. `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:29`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:30`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:31`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:32`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:33`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:34`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:35`
- Each block renderer expects `selected_known_type_id` or `original_type_id`, `payload`, `trust_band`, and text-like payload fields. `wp/dailyos/blocks/account-overview/render-functions.php:129`; `wp/dailyos/blocks/account-overview/render-functions.php:134`; `wp/dailyos/blocks/account-overview/render-functions.php:136`; `wp/dailyos/blocks/account-overview/render-functions.php:144`; `wp/dailyos/blocks/account-overview/render-functions.php:153`; `wp/dailyos/blocks/account-overview/render-functions.php:155`; `wp/dailyos/blocks/account-overview/render-functions.php:163`; `wp/dailyos/blocks/account-overview/render-functions.php:171`

Risks:
- Preserve `dailyos_account_overview_render` as the fetch wrapper for `render.php`; make `dailyos_account_overview_render_from_projection($response, $attributes)` pure and callable by preview only. `wp/dailyos/blocks/account-overview/render.php:25`
- The helper must accept both runtime envelopes and local transport error envelopes, because PHP transport returns `ok:false,error:{code,message}` for transport failures. `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:503`

## 2. Editor Reload Guard

Answer: feasible with changes; not safe as stated.

Evidence:
- `reload` currently reads `attributes.composition_id`, `attributes.composition_version`, and `attributes.cache_hint_token` when building the POST body. `wp/dailyos/blocks/account-overview/edit.js:44`; `wp/dailyos/blocks/account-overview/edit.js:54`; `wp/dailyos/blocks/account-overview/edit.js:55`; `wp/dailyos/blocks/account-overview/edit.js:56`
- The success path writes `composition_version`, `watermarks`, and `cache_hint_token` back to attributes. `wp/dailyos/blocks/account-overview/edit.js:62`; `wp/dailyos/blocks/account-overview/edit.js:63`; `wp/dailyos/blocks/account-overview/edit.js:66`; `wp/dailyos/blocks/account-overview/edit.js:67`
- The callback dependency list includes `composition_version` and `cache_hint_token`, and the effect runs whenever `reload` identity changes. `wp/dailyos/blocks/account-overview/edit.js:82`; `wp/dailyos/blocks/account-overview/edit.js:84`
- The manual button also uses the same `reload` closure. `wp/dailyos/blocks/account-overview/edit.js:136`

Risks:
- Removing `composition_version` and `cache_hint_token` from deps while still reading them creates a stale-closure risk for manual reloads. `wp/dailyos/blocks/account-overview/edit.js:55`; `wp/dailyos/blocks/account-overview/edit.js:56`; `wp/dailyos/blocks/account-overview/edit.js:136`
- Safe implementation choices: either stop reading those attributes in `reload`, or keep latest values in refs updated outside the callback, and key the automatic effect only on `composition_id`/`account_id`. `wp/dailyos/blocks/account-overview/edit.js:43`; `wp/dailyos/blocks/account-overview/edit.js:84`; `wp/dailyos/blocks/account-overview/edit.js:103`

## 3. Cache Key Correction

Answer: feasible; pick option (a), key by current DB/effective projection version.

Evidence:
- Cache key currently includes `composition_id`, `composition_version`, and `scopes_canonical_id`. `src-tauri/src/services/composition_render_orchestrator.rs:51`; `src-tauri/src/services/composition_render_orchestrator.rs:53`; `src-tauri/src/services/composition_render_orchestrator.rs:54`; `src-tauri/src/services/composition_render_orchestrator.rs:55`
- Route cache lookup passes `request.composition_version`, which is the surface watermark. `src-tauri/src/surface_runtime/mod.rs:2317`; `src-tauri/src/surface_runtime/mod.rs:2320`
- Route cache store also passes `request.composition_version`. `src-tauri/src/surface_runtime/mod.rs:2425`; `src-tauri/src/surface_runtime/mod.rs:2429`
- The route already knows how to fetch the current DB version, but only after the current cache miss. `src-tauri/src/surface_runtime/mod.rs:2355`; `src-tauri/src/surface_runtime/mod.rs:2361`; `src-tauri/src/surface_runtime/mod.rs:2370`
- `ProjectedComposition` exposes the actual projected composition version. `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:29`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:31`; `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:373`

Risks:
- Option (a) is smaller: move current-version lookup before cache lookup, lookup by `current_db_version`, and store by `projection.composition_version.unwrap_or(current_db_version)`. `src-tauri/src/surface_runtime/mod.rs:2317`; `src-tauri/src/surface_runtime/mod.rs:2355`; `src-tauri/src/surface_runtime/mod.rs:2425`
- Option (b) needs an invalidation API; the orchestrator currently has lookup, store, test size, and test clear only. `src-tauri/src/services/composition_render_orchestrator.rs:102`; `src-tauri/src/services/composition_render_orchestrator.rs:121`; `src-tauri/src/services/composition_render_orchestrator.rs:141`; `src-tauri/src/services/composition_render_orchestrator.rs:147`

## 4. Render-Path Producer-Commit Removal

Answer: feasible only with a meaningful refactor; not a clean conditional inside the current handler.

Evidence:
- In the `project-composition` route, cache miss builds a live service context and invokes the producer ability. `src-tauri/src/surface_runtime/mod.rs:2342`; `src-tauri/src/surface_runtime/mod.rs:2344`; `src-tauri/src/surface_runtime/mod.rs:2378`
- The route forwards current DB version as `expected_composition_version` on every miss. `src-tauri/src/surface_runtime/mod.rs:2348`; `src-tauri/src/surface_runtime/mod.rs:2355`; `src-tauri/src/surface_runtime/mod.rs:2371`; `src-tauri/src/surface_runtime/mod.rs:2376`
- `dailyos/account-overview` is a read-category ability but commits the prepared composition before returning output. `src-tauri/abilities-runtime/src/abilities/account_overview.rs:83`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:85`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:112`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:114`
- The live service context attaches `LiveCompositionCommitter`, and that adapter calls `services::compositions::commit_composition`. `src-tauri/src/services/context.rs:52`; `src-tauri/src/services/context.rs:58`; `src-tauri/src/services/context.rs:108`; `src-tauri/src/services/context.rs:135`
- `commit_composition` updates or inserts only `composition_versions`, emits a version event, and returns the proposal composition; it does not persist a reusable composition payload. `src-tauri/src/services/compositions.rs:271`; `src-tauri/src/services/compositions.rs:277`; `src-tauri/src/services/compositions.rs:312`; `src-tauri/src/services/compositions.rs:345`; `src-tauri/src/services/compositions.rs:368`
- Current read support exposes only current composition version lookup. `src-tauri/src/services/compositions.rs:149`; `src-tauri/src/services/compositions.rs:161`; `src-tauri/src/services/compositions.rs:174`
- Manual editor refresh is not distinguished from automatic preview: the button calls `reload`, the REST preview payload has only composition/version/cache hint, and the runtime request struct has no trigger field. `wp/dailyos/blocks/account-overview/edit.js:136`; `wp/dailyos/blocks/account-overview/edit.js:53`; `wp/dailyos/includes/class-dailyos-plugin.php:570`; `src-tauri/src/surface_runtime/mod.rs:2225`
- Signal policy metadata exists on the producer, but the route has no signal/invalidation trigger input. `src-tauri/abilities-runtime/src/abilities/account_overview.rs:97`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:99`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:103`; `src-tauri/src/surface_runtime/mod.rs:2225`

Risks:
- Every render-path commit site is indirect: `project-composition` invokes the producer, the producer calls `ctx.services().commit_composition`, and the live adapter writes through `services::compositions::commit_composition`. `src-tauri/src/surface_runtime/mod.rs:2378`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:112`; `src-tauri/abilities-runtime/src/services/context.rs:1333`; `src-tauri/src/services/context.rs:135`
- Trigger surfaces need new shape: manual refresh flag from editor/PHP, initial-creation branch when current version is `0`, and a substrate-side invalidation/refresh path for signal-driven recomposition. `wp/dailyos/blocks/account-overview/edit.js:136`; `src-tauri/src/services/compositions.rs:174`; `src-tauri/abilities-runtime/src/abilities/account_overview.rs:97`
- A simple "skip commit when current version > 0" would leave no DB-backed composition payload to project on cache miss. `src-tauri/src/services/compositions.rs:368`; `src-tauri/src/services/compositions.rs:371`

## 5. Render-Read Decharge

Answer: choose option (a), with a narrow authorization variant.

Evidence:
- Default `standard_read_composition` budget is 60/min, burst 5. `src-tauri/src/bridges/surface_client.rs:211`; `src-tauri/src/bridges/surface_client.rs:213`
- `authorize` delegates to `authorize_for_path` and currently always consumes the rate limiter. `src-tauri/src/bridges/surface_client.rs:260`; `src-tauri/src/bridges/surface_client.rs:267`; `src-tauri/src/bridges/surface_client.rs:387`; `src-tauri/src/bridges/surface_client.rs:389`
- Mandatory checks are separate before consumption: descriptor exists, actor/mode/experimental gates, required scopes, and browser-direct executable guard. `src-tauri/src/bridges/surface_client.rs:301`; `src-tauri/src/bridges/surface_client.rs:319`; `src-tauri/src/bridges/surface_client.rs:343`; `src-tauri/src/bridges/surface_client.rs:355`
- The ability/scope bucket candidates are already gated by `charge_ability_scope`; `authorize` hardcodes that flag to `true`. `src-tauri/src/bridges/surface_client.rs:409`; `src-tauri/src/bridges/surface_client.rs:795`; `src-tauri/src/bridges/surface_client.rs:811`

Risks:
- Cleanest split is `authorize_local_render`/`authorize_decharged` that reuses descriptor, actor, mode, and scope validation, then calls limiter with `charge_ability_scope=false` or bypasses limiter entirely if the packet intends zero bucket writes. `src-tauri/src/bridges/surface_client.rs:292`; `src-tauri/src/bridges/surface_client.rs:319`; `src-tauri/src/bridges/surface_client.rs:343`; `src-tauri/src/bridges/surface_client.rs:409`
- Setting `charge_ability_scope=false` bypasses `standard_read_composition` and scope candidates, but still consumes surface/site/user buckets. `src-tauri/src/bridges/surface_client.rs:766`; `src-tauri/src/bridges/surface_client.rs:776`; `src-tauri/src/bridges/surface_client.rs:787`; `src-tauri/src/bridges/surface_client.rs:795`

## 6. Typed Error Mapping

Answer: feasible, but packet shape is slightly wrong for WP.

Evidence:
- Runtime error envelopes carry typed code strings under `error.code`, plus message, request_id, remediation, optional retry/axis. `src-tauri/src/surface_runtime/mod.rs:3514`; `src-tauri/src/surface_runtime/mod.rs:3517`; `src-tauri/src/surface_runtime/mod.rs:3518`; `src-tauri/src/surface_runtime/mod.rs:3525`; `src-tauri/src/surface_runtime/mod.rs:3528`
- Runtime does not put `code` at top level in `error_response`; WP transport adds `ok=false` for non-2xx and preserves nested `error`. `src-tauri/src/surface_runtime/mod.rs:3517`; `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:393`; `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:401`
- Local transport failures produce `ok:false,error:{code,message}` with codes such as `runtime_request_failed`, `runtime_invalid_json`, and `runtime_http_error`. `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:379`; `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:388`; `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:396`; `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:503`
- Current renderer maps any `WP_Error` or missing projection to the verification banner. `wp/dailyos/blocks/account-overview/render-functions.php:61`; `wp/dailyos/blocks/account-overview/render-functions.php:68`; `wp/dailyos/blocks/account-overview/render-functions.php:227`
- Current project-composition route emits `project_composition_invalid`, `project_composition_unknown_producer`, `project_composition_invalid_id`, `rate_limited`, and `runtime_unavailable`. `src-tauri/src/surface_runtime/mod.rs:2247`; `src-tauri/src/surface_runtime/mod.rs:2268`; `src-tauri/src/surface_runtime/mod.rs:2274`; `src-tauri/src/surface_runtime/mod.rs:2304`; `src-tauri/src/surface_runtime/mod.rs:2326`
- Existing runtime code groups include transport codes `signature_invalid`, `canonicalization_mismatch`, `timestamp_stale`, `timestamp_future`, `key_not_found`, `key_rotated`, `token_invalid`, `nonce_replay`, `transport_abuse_limited`. `src-tauri/src/surface_runtime/hmac.rs:792`
- Existing pairing/session codes include `pairing_code_invalid`, `pairing_code_expired`, `pairing_code_consumed`, `pairing_code_limited`, `unknown_runtime_anchor`, `restored_stale_pairing`, `site_binding_mismatch`, `pairing_suspended`, `pairing_revoked`, `pairing_expired`, `session_invalid`, `session_expired`, `session_throttled`, `session_requires_repair`, `wp_user_mismatch`, `scope_denied`, `pairing_authority_unavailable`. `src-tauri/src/services/surface_pairing.rs:261`
- Existing HTTP helper/bridge codes include `host_invalid`, `browser_origin_forbidden`, `auth_missing`, `session_not_found`, `identity_mismatch`, `wrong_user`, `route_not_found`, `request_body_too_large`, `rate_limited`, `runtime_unavailable`, `projection_tampered`, `projection_version_rollback`, `missing_expected_claim_version`, `mid_flight_mutation`, `claim_version_overflow`, `stale_watermark`, `stale_composition_watermark`, `composition_version_overflow`. `src-tauri/src/surface_runtime/mod.rs:3285`; `src-tauri/src/surface_runtime/mod.rs:3294`; `src-tauri/src/surface_runtime/mod.rs:3303`; `src-tauri/src/surface_runtime/mod.rs:3361`; `src-tauri/src/surface_runtime/mod.rs:3370`; `src-tauri/src/surface_runtime/mod.rs:3379`; `src-tauri/src/surface_runtime/mod.rs:3388`; `src-tauri/src/surface_runtime/mod.rs:3406`; `src-tauri/src/surface_runtime/mod.rs:3426`; `src-tauri/src/surface_runtime/mod.rs:3447`; `src-tauri/src/surface_runtime/mod.rs:3010`; `src-tauri/src/surface_runtime/mod.rs:3016`; `src-tauri/src/surface_runtime/mod.rs:3022`; `src-tauri/src/surface_runtime/mod.rs:3025`; `src-tauri/src/surface_runtime/mod.rs:3031`; `src-tauri/src/surface_runtime/mod.rs:3037`; `src-tauri/src/surface_runtime/mod.rs:3043`; `src-tauri/src/surface_runtime/mod.rs:3049`

Risks:
- WP renderer should gate verification-banner emission on consistency/projection codes only; transport/session/rate codes should render typed operational notices. `wp/dailyos/blocks/account-overview/render-functions.php:61`; `wp/dailyos/blocks/account-overview/render-functions.php:68`; `src-tauri/src/surface_runtime/mod.rs:3518`

## Findings

- HIGH `src-tauri/src/surface_runtime/mod.rs:2378` — Render-path commit removal is not a local conditional; add explicit refresh/invalidation/initial-create trigger plumbing before removing producer invocation from ordinary reads.
- HIGH `src-tauri/src/services/compositions.rs:368` — Current composition persistence returns the proposal but does not store a reusable composition payload; add a read model or read-only producer path before claiming "read existing projected state from DB."
- MEDIUM `wp/dailyos/blocks/account-overview/edit.js:55` — Removing version/cache deps while reading them creates stale manual reload payloads; use refs or stop sending those fields from `reload`.
- MEDIUM `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:393` — Packet says runtime returns top-level `code`, but WP receives nested `error.code`; map `error.code` and local transport envelopes.
- MEDIUM `src-tauri/src/bridges/surface_client.rs:409` — `authorize` hardcodes `charge_ability_scope=true`; add a render-specific authorization variant instead of raising global budgets.
- LOW `src-tauri/src/services/composition_render_orchestrator.rs:147` — Version-agnostic cache needs invalidation API that does not exist; choose current-version keying for this packet.
