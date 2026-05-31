# v1.4.3 WordPress Foundation Stabilization Investigation

Date: 2026-05-17

Investigated tickets:

- DOS-671, WP block content disappears within about 30s after render.
- DOS-672, reload-from-runtime fires on every window switch; second reload returns verification banner.
- DOS-673, keychain lookup error vs miss causes false revocation.
- DOS-674, revoked or expired sessions leak signing secrets in macOS keychain.
- DOS-675, shutdown cleanup runs after listener abort.

Checkout note:

- Prompt grounding said branch `dev` at `9a33d347`.
- This checkout reported branch `docs/v143-carry-forward` at `e7a2b2ea54a58f5fc16c1996406f44c0bc9c4860`.
- All citations below are against the files as present in this checkout.

## Threat Model Constraint

The v1.4.2 project description is explicit:

- The Tauri runtime and WordPress Studio are both local processes on the user's machine, using loopback HTTP, with no network or multi-tenant boundary (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md:56`).
- Reads must not write to `surface_client_sessions`, audit tables, rate-limit buckets, or other DB state (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md:60`).
- Pairing is a one-time write, then read-only lookup; session lifetime is bounded by user intent and machine reinstall, not per-request TTLs sized for adversarial remote clients (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md:62`).
- Local defaults should avoid remote-shaped defenses like 5-minute session TTLs, session evaporation on restart, render-path rate-limit consumption, and silent DB-hiccup failures (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md:63`).

Implication for this plan:

- Do not fix these tickets by shortening sessions, evicting on read, increasing read throttles, or forcing re-pair on routine restarts.
- Do not treat the WordPress block as an untrusted remote browser.
- Do remove render-path writes and render-path rate-limit gating where they create local instability.
- Do keep local recovery threats in scope: reinstall, DB restore, site switch, and local secret exfiltration.

## Executive Summary

The observed WP block instability is not primarily a session TTL problem.

The runtime already sets a 365-day session TTL for local-to-local sessions (`src-tauri/src/services/surface_pairing.rs:20` to `src-tauri/src/services/surface_pairing.rs:31`), and the read-only validator ignores `inactive_expires_at` while checking only `absolute_expires_at` for validity (`src-tauri/src/services/surface_pairing.rs:856` to `src-tauri/src/services/surface_pairing.rs:868`, `src-tauri/src/services/surface_pairing.rs:885` to `src-tauri/src/services/surface_pairing.rs:888`).

The stronger root cause for DOS-671 and DOS-672 is a refresh loop across the WP editor, PHP preview route, and runtime project-composition route:

- The editor stores rendered HTML only in transient React state, then renders only when `preview.projection` exists (`wp/dailyos/blocks/account-overview/edit.js:38`, `wp/dailyos/blocks/account-overview/edit.js:153` to `wp/dailyos/blocks/account-overview/edit.js:157`).
- A successful reload updates block attributes that are dependencies of the `reload` callback (`wp/dailyos/blocks/account-overview/edit.js:62` to `wp/dailyos/blocks/account-overview/edit.js:68`, `wp/dailyos/blocks/account-overview/edit.js:82`).
- The auto-reload effect depends on that callback, so the success path schedules another reload when those attributes change (`wp/dailyos/blocks/account-overview/edit.js:84` to `wp/dailyos/blocks/account-overview/edit.js:86`).
- One preview request calls the runtime once in `account_overview_preview`, then calls the block renderer, which calls the runtime again (`wp/dailyos/includes/class-dailyos-plugin.php:587` to `wp/dailyos/includes/class-dailyos-plugin.php:612`, `wp/dailyos/blocks/account-overview/render-functions.php:55` to `wp/dailyos/blocks/account-overview/render-functions.php:59`).
- On a cache miss, the runtime intentionally invokes the producer and advances the composition version (`src-tauri/src/surface_runtime/mod.rs:2348` to `src-tauri/src/surface_runtime/mod.rs:2377`, `src-tauri/src/services/context.rs:108` to `src-tauri/src/services/context.rs:136`).
- The cache is keyed by the caller's incoming `composition_version`, not the projected composition's actual version (`src-tauri/src/services/composition_render_orchestrator.rs:51` to `src-tauri/src/services/composition_render_orchestrator.rs:56`, `src-tauri/src/surface_runtime/mod.rs:2317` to `src-tauri/src/surface_runtime/mod.rs:2321`, `src-tauri/src/surface_runtime/mod.rs:2425` to `src-tauri/src/surface_runtime/mod.rs:2430`).
- The runtime authorizes and consumes surface-client read budget before cache lookup (`src-tauri/src/surface_runtime/mod.rs:2288` to `src-tauri/src/surface_runtime/mod.rs:2311`, `src-tauri/src/surface_runtime/mod.rs:2313` to `src-tauri/src/surface_runtime/mod.rs:2321`).
- The default standard composition-read burst is only five requests per second (`src-tauri/src/bridges/surface_client.rs:211` to `src-tauri/src/bridges/surface_client.rs:214`), and each preview can consume two units before the editor loop adds more.

The visible 30s disappearance is explained by this local loop plus the 30s signed-post timeout: a follow-up reload can be started by the attribute change, wait up to 30 seconds, then replace the preview with an error-shaped response or verification HTML (`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:279` to `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:288`, `wp/dailyos/blocks/account-overview/edit.js:59` to `wp/dailyos/blocks/account-overview/edit.js:60`).

The lifecycle tickets are also confirmed and should land with the same stabilization PR: DOS-673 is lookup classification, DOS-674 is missing key cleanup on lifecycle transitions, and DOS-675 is unreachable shutdown cleanup.

## DOS-671, Block Content Disappears Within About 30s

### Confirmed Root Cause

The confirmed code-level root cause is the editor/runtime refresh loop, not a short runtime session.

Evidence:

- The editor preview body is held only in React state: `const [ preview, setPreview ] = useState( null )` (`wp/dailyos/blocks/account-overview/edit.js:38`).
- The visible block HTML is rendered only from that transient `preview` object and only when `preview.projection` exists (`wp/dailyos/blocks/account-overview/edit.js:153` to `wp/dailyos/blocks/account-overview/edit.js:157`).
- The reload success path replaces `preview` with whatever the preview endpoint returned (`wp/dailyos/blocks/account-overview/edit.js:59` to `wp/dailyos/blocks/account-overview/edit.js:60`).
- The same success path writes `composition_version`, `watermarks`, and `cache_hint_token` back to block attributes (`wp/dailyos/blocks/account-overview/edit.js:62` to `wp/dailyos/blocks/account-overview/edit.js:68`).
- Those attributes are dependencies of the `reload` callback (`wp/dailyos/blocks/account-overview/edit.js:82`).
- The auto-reload effect depends on the callback, so a successful attribute update changes the callback identity and re-runs reload (`wp/dailyos/blocks/account-overview/edit.js:84` to `wp/dailyos/blocks/account-overview/edit.js:86`).
- The preview endpoint calls the runtime once to get the response (`wp/dailyos/includes/class-dailyos-plugin.php:587` to `wp/dailyos/includes/class-dailyos-plugin.php:591`).
- The preview endpoint then renders via `render_block_with_filter` (`wp/dailyos/includes/class-dailyos-plugin.php:596` to `wp/dailyos/includes/class-dailyos-plugin.php:612`).
- `render_block_with_filter` calls `dailyos_account_overview_render` (`wp/dailyos/includes/class-dailyos-plugin.php:682` to `wp/dailyos/includes/class-dailyos-plugin.php:690`).
- `dailyos_account_overview_render` calls `project_composition_for_surface` again (`wp/dailyos/blocks/account-overview/render-functions.php:55` to `wp/dailyos/blocks/account-overview/render-functions.php:59`).
- The signed runtime client allows a project-composition call to sit for 30 seconds before timing out (`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:279` to `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:288`).

This gives a concrete disappearance path:

1. First reload succeeds and sets `preview`.
2. Success writes attributes.
3. Attribute writes retrigger the auto-reload effect.
4. The next preview request can block for up to 30s or return an error-shaped response.
5. `setPreview(response)` replaces the previously visible projection.
6. If the response lacks `projection`, the editor renders nothing because of the `preview.projection` guard.
7. If the first PHP runtime call succeeded but the nested render runtime call failed, the editor can render verification HTML instead of the block body.

The exact outside trigger for "about 30s" after render is not directly visible in DailyOS source. Hypothesis: Gutenberg editor remounts, autosave, or the attribute-triggered follow-up request causes the timing. The 30s signed-post timeout is a confirmed DailyOS timing boundary.

### Incorrect Remote-Shaped Framing

Reject "surface session TTL is too tight" for this ticket.

- The runtime sets `SESSION_ABSOLUTE_TTL_SECONDS` to 365 days (`src-tauri/src/services/surface_pairing.rs:20` to `src-tauri/src/services/surface_pairing.rs:25`).
- `SESSION_INACTIVE_TTL_SECONDS` is equal to the absolute TTL, and comments say inactive expiry is deprecated for validity (`src-tauri/src/services/surface_pairing.rs:26` to `src-tauri/src/services/surface_pairing.rs:31`).
- The read-only validator checks `absolute_expires_at` and explicitly does not consult `inactive_expires_at` (`src-tauri/src/services/surface_pairing.rs:856` to `src-tauri/src/services/surface_pairing.rs:868`, `src-tauri/src/services/surface_pairing.rs:885` to `src-tauri/src/services/surface_pairing.rs:888`).

Do not fix DOS-671 by shortening cache TTL, expiring sessions on read, consuming stricter read budgets, or requiring re-pair after focus changes.

### Fix Shape

Use a local-shaped stabilization fix:

1. Split the block renderer into a pure projection-to-HTML helper.
   - Keep `dailyos_account_overview_render(array $attributes)` as the dynamic front-end entry.
   - Add an internal helper that renders an already-fetched `$projection` without calling the runtime again.
   - Have `account_overview_preview` call the runtime exactly once, then render HTML from that response.

2. Stop auto-reload churn in `edit.js`.
   - The effect should auto-load only when `composition_id` changes from empty to non-empty or when the selected account changes.
   - Do not include `cache_hint_token` in the callback dependency that drives the automatic effect.
   - The button remains explicit user action.
   - Preserve the last good preview on error, and show the error notice without replacing content with an error envelope.

3. Fix project-composition cache/version semantics.
   - If the route continues to cache projections, the cache key must align with the actual projected composition version, not only the incoming stale surface watermark.
   - The current route looks up by `request.composition_version` (`src-tauri/src/surface_runtime/mod.rs:2317` to `src-tauri/src/surface_runtime/mod.rs:2321`) and stores by the same request value (`src-tauri/src/surface_runtime/mod.rs:2425` to `src-tauri/src/surface_runtime/mod.rs:2430`).
   - The route separately fetches current DB version and forwards that to the producer (`src-tauri/src/surface_runtime/mod.rs:2355` to `src-tauri/src/surface_runtime/mod.rs:2377`), so the request version and produced version can diverge.

4. Remove render-path writes as the v1.4.3 target.
   - The current route comments say the producer advances `composition_version` monotonically on render (`src-tauri/src/surface_runtime/mod.rs:2348` to `src-tauri/src/surface_runtime/mod.rs:2353`).
   - Live ability context includes `LiveCompositionCommitter`, which opens the DB and commits compositions (`src-tauri/src/services/context.rs:108` to `src-tauri/src/services/context.rs:136`).
   - That violates the local-to-local read contract for steady-state block rendering.
   - The local shape is: render reads projected state; producer writes happen on explicit refresh, invalidation, or upstream signal propagation, not every render.

### Order of Operations

1. Land PHP single-fetch preview and editor reload guard together.
2. Land runtime cache-key correction, render-read decharging, and typed preview errors in the same PR.
3. Run L4 hands-on with real local runtime and WordPress editor: initial render, wait 45s, switch windows twice, manual reload twice.

### Acceptance Test Sketch

- PHP unit: preview calls runtime once and renders the supplied projection without a nested runtime call.
- JS unit: cache-token/version changes do not trigger an automatic reload, and failed reload preserves last-good preview.
- Rust unit: cache key uses the effective projection/current DB version, and local render cache hits remain scope-checked without read-budget failure.
- Hands-on: content remains visible after 45 seconds and after focus changes, without pressing reload.

### Risk and Blast Radius

Files: `wp/dailyos/includes/class-dailyos-plugin.php`, `wp/dailyos/blocks/account-overview/edit.js`, `wp/dailyos/blocks/account-overview/render-functions.php`, `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`, `src-tauri/src/surface_runtime/mod.rs`, `src-tauri/src/services/composition_render_orchestrator.rs`, `src-tauri/src/bridges/surface_client.rs`.

Risk is medium because render-path producer behavior and read decharging touch W4-A/W4-D assumptions. Keep authorization before cache lookup and separate authorization from budget consumption.

## DOS-672, Reload on Window Switch and Second Reload Verification Banner

### Confirmed Root Cause

DOS-672 shares the DOS-671 root cause, with one extra confirmed failure shape: PHP preview can return a projection from the first runtime call plus verification-banner HTML from the second runtime call.

Evidence:

- The editor's reload starts a REST preview request (`wp/dailyos/blocks/account-overview/edit.js:50` to `wp/dailyos/blocks/account-overview/edit.js:58`).
- The preview handler first calls `project_composition_for_surface` and stores `$response` (`wp/dailyos/includes/class-dailyos-plugin.php:587` to `wp/dailyos/includes/class-dailyos-plugin.php:594`).
- It then builds `$attributes` from `$response['projection']` (`wp/dailyos/includes/class-dailyos-plugin.php:598` to `wp/dailyos/includes/class-dailyos-plugin.php:608`).
- It calls the block render path to generate HTML (`wp/dailyos/includes/class-dailyos-plugin.php:610` to `wp/dailyos/includes/class-dailyos-plugin.php:618`).
- That render path calls the runtime again (`wp/dailyos/blocks/account-overview/render-functions.php:55` to `wp/dailyos/blocks/account-overview/render-functions.php:59`).
- If the nested runtime call returns a `WP_Error`, the render function emits the verification banner (`wp/dailyos/blocks/account-overview/render-functions.php:61` to `wp/dailyos/blocks/account-overview/render-functions.php:63`).
- If the nested runtime response lacks `projection`, the same banner is emitted (`wp/dailyos/blocks/account-overview/render-functions.php:65` to `wp/dailyos/blocks/account-overview/render-functions.php:70`).
- The verification banner text is `Something about this account doesn't line up. Verify before acting.` (`wp/dailyos/blocks/account-overview/render-functions.php:227` to `wp/dailyos/blocks/account-overview/render-functions.php:230`).
- The preview endpoint merges the original `$response` with the later `$html` (`wp/dailyos/includes/class-dailyos-plugin.php:613` to `wp/dailyos/includes/class-dailyos-plugin.php:618`).
- The editor renders `preview.html` when `preview.projection` exists (`wp/dailyos/blocks/account-overview/edit.js:153` to `wp/dailyos/blocks/account-overview/edit.js:157`).

That explains the observed "verification banner with no rendered content underneath": the response can still contain the first projection, so the editor renders the HTML string, but that HTML string came from the failed second call.

The reload-on-window-switch behavior is also explained by the editor's fragile lifecycle:

- Preview state is local React state (`wp/dailyos/blocks/account-overview/edit.js:38`).
- The effect auto-reloads on callback identity changes (`wp/dailyos/blocks/account-overview/edit.js:84` to `wp/dailyos/blocks/account-overview/edit.js:86`).
- The callback identity changes when successful reload writes attribute dependencies (`wp/dailyos/blocks/account-overview/edit.js:62` to `wp/dailyos/blocks/account-overview/edit.js:68`, `wp/dailyos/blocks/account-overview/edit.js:82`).

### Incorrect Remote-Shaped Framing

Reject "session evaporates between reloads" as the primary fix target.

- Session absolute TTL is local-sized at 365 days (`src-tauri/src/services/surface_pairing.rs:20` to `src-tauri/src/services/surface_pairing.rs:25`).
- Read validation does not refresh or consume inactive TTL on success (`src-tauri/src/services/surface_pairing.rs:856` to `src-tauri/src/services/surface_pairing.rs:868`, `src-tauri/src/services/surface_pairing.rs:959` to `src-tauri/src/services/surface_pairing.rs:963`).
- The code comments explicitly say the read validator has no writes on the OK path (`src-tauri/src/services/surface_pairing.rs:959` to `src-tauri/src/services/surface_pairing.rs:963`).

Do not add re-pair-on-focus, shorter TTLs, or read-time session mutation.

### Fix Shape

Use the same shared fix as DOS-671, plus typed error mapping:

1. Single-fetch preview.
   - The preview REST route should not call runtime once for data and once again for HTML.
   - Render HTML from the already-fetched projection.

2. Manual reload should be stable.
   - User-clicked reload should fire exactly one preview request.
   - Window focus should not fire reload unless WordPress remounts the component and no last-good preview exists.
   - Attribute writes from a successful reload should not schedule another reload.

3. Preserve last-good content.
   - A failed reload should not replace `preview` with an error envelope that fails the `preview.projection` guard.
   - Keep last-good HTML visible with a warning notice.

4. Use typed transport/session messages.
   - `rate_limited`, `session_requires_repair`, `session_not_found`, and `runtime_request_failed` should not all map to the consistency banner.
   - Consistency banner should remain for actual projection consistency failures.

5. Decharge local render reads.
   - The bridge currently consumes rate-limit buckets during authorization (`src-tauri/src/bridges/surface_client.rs:387` to `src-tauri/src/bridges/surface_client.rs:410`).
   - Ability budget is one of the consumed buckets (`src-tauri/src/bridges/surface_client.rs:811` to `src-tauri/src/bridges/surface_client.rs:821`).
   - Local block render reads should remain authorized but not be gated by the standard-read-composition burst budget.

### Order of Operations

1. Add failing tests for double runtime calls and attribute-triggered reload.
2. Refactor PHP single-fetch preview and editor last-good-preview behavior.
3. Update runtime project-composition cache/rate behavior, then re-run L4 focus/reload sequence.

### Acceptance Test Sketch

- PHP unit: fake runtime succeeds first call and fails second; after fix only one call happens.
- JS unit: one automatic request per selected `composition_id`; manual reloads are one request each and preserve content on failure.
- Rust unit: tight standard-read-composition budget does not fail local render cache hits.
- Hands-on: switch window focus twice and inspect network, no reload storm.

### Risk and Blast Radius

Files overlap DOS-671. User-visible risk is low if last-good-preview behavior stays editor-only; runtime risk is medium because rate-budget changes touch SurfaceClient authorization. Scope authorization must remain mandatory before cache hits.

## DOS-673, Keychain Lookup Error vs Miss

### Confirmed Root Cause

The root cause is confirmed exactly in code.

Evidence:

- `load_session_master_key` returns `Option<[u8; KEY_BYTES]>`, so it cannot distinguish found, not found, CLI error, and malformed payload (`src-tauri/src/services/surface_session_keychain.rs:107` to `src-tauri/src/services/surface_session_keychain.rs:110`).
- `run_security_cmd(...).ok()?` maps command-spawn errors into `None` (`src-tauri/src/services/surface_session_keychain.rs:112` to `src-tauri/src/services/surface_session_keychain.rs:120`).
- Any non-success `security find-generic-password` status returns `None` (`src-tauri/src/services/surface_session_keychain.rs:121` to `src-tauri/src/services/surface_session_keychain.rs:123`).
- UTF-8, base64, and key-length failures also return `None` (`src-tauri/src/services/surface_session_keychain.rs:124` to `src-tauri/src/services/surface_session_keychain.rs:130`).
- Startup rehydration treats `Some(master_key)` as active and every `None` as missing (`src-tauri/src/surface_runtime/mod.rs:667` to `src-tauri/src/surface_runtime/mod.rs:686`).
- Missing rows are written as revoked with `revoked_reason = 'keychain_entry_missing'` (`src-tauri/src/surface_runtime/mod.rs:705` to `src-tauri/src/surface_runtime/mod.rs:713`).
- The audit event also records `reason: keychain_entry_missing` and `remediation: session_requires_repair` (`src-tauri/src/surface_runtime/mod.rs:726` to `src-tauri/src/surface_runtime/mod.rs:740`).

### Validation of Specified Fix Shape

The ticket's specified three-state shape is correct and local-shaped:

- `Found(payload)`: rehydrate.
- `NotFound`: revoke as `keychain_entry_missing`.
- `Error(reason)`: log and keep DB row for later reconciliation.

One implementation detail should be added:

- Existing-key malformed payload should not collapse into `NotFound`.
- Lines `124` to `130` show decode and length failures currently return `None`.
- Treat malformed payload as `Error` or `Corrupt`, then decide separately whether to surface repair. Do not label it `keychain_entry_missing`.

This is not a remote-shaped fix. It preserves long-lived local sessions through transient macOS keychain failures.

### Fix Shape

1. Replace `Option<[u8; KEY_BYTES]>` with a classified enum, for example:
   - `SessionKeyLookup::Found([u8; KEY_BYTES])`
   - `SessionKeyLookup::NotFound`
   - `SessionKeyLookup::Unavailable { reason: String }`
   - Optional: `SessionKeyLookup::Corrupt { reason: String }`

2. Classify `security` exit results.
   - Not-found should be recognized from known macOS keychain missing-item output and exit status.
   - Locked keychain, permission denied, command spawn failure, and undecodable output should be non-revoking errors.

3. Update `rehydrate_sessions_from_keychain`.
   - `Found`: register session.
   - `NotFound`: add to `missing`.
   - `Unavailable` or `Corrupt`: emit warning/audit and leave DB row active.

4. Keep the read path unchanged.
   - This is startup reconciliation, not a render read.
   - Do not introduce read-time keychain probing or read-time revocation.

### Order of Operations

1. Add lookup enum and keychain tests.
2. Update rehydrate match arms and startup reconciliation tests.
3. Confirm `SessionRequiresRepair` remains only for true missing-key revocations.

### Acceptance Test Sketch

- Unit: mocked `Found`, `NotFound`, and `Unavailable` take register, revoke, and keep-active paths respectively.
- Unit: malformed base64 does not become `keychain_entry_missing`.
- macOS integration: real deleted keychain item produces `NotFound`.

### Risk and Blast Radius

Files: `src-tauri/src/services/surface_session_keychain.rs`, `src-tauri/src/surface_runtime/mod.rs`. Risk is classification accuracy: too narrow leaves missing sessions active until later, too broad recreates false revocation. Blast radius is startup rehydration only.

## DOS-674, Revoked or Expired Sessions Leak Keychain Secrets

### Confirmed Root Cause

The root cause is confirmed.

Evidence:

- Successful pairing persists the session master key in macOS keychain (`src-tauri/src/services/surface_pairing.rs:623` to `src-tauri/src/services/surface_pairing.rs:636`).
- A delete helper exists and treats missing items as success (`src-tauri/src/services/surface_session_keychain.rs:137` to `src-tauri/src/services/surface_session_keychain.rs:156`).
- Explicit revoke loads the pairing target and calls `revoke_pairing_row` inside a DB transaction (`src-tauri/src/services/surface_pairing.rs:1225` to `src-tauri/src/services/surface_pairing.rs:1272`).
- `revoke_pairing_row` updates `surface_client_pairings`, updates `surface_client_sessions`, and inserts `surface_client_revocations`, but does not delete keychain material (`src-tauri/src/services/surface_pairing.rs:1993` to `src-tauri/src/services/surface_pairing.rs:2039`).
- `mark_session_revoked` updates only `surface_client_sessions` (`src-tauri/src/services/surface_pairing.rs:2042` to `src-tauri/src/services/surface_pairing.rs:2057`).
- `mark_pairing_expired` updates only `surface_client_pairings` (`src-tauri/src/services/surface_pairing.rs:2059` to `src-tauri/src/services/surface_pairing.rs:2074`).
- Re-pair revokes the previous pairing in `revoke_existing_pairing_for_site` (`src-tauri/src/services/surface_pairing.rs:1803` to `src-tauri/src/services/surface_pairing.rs:1856`) and then persists the new key (`src-tauri/src/services/surface_pairing.rs:623` to `src-tauri/src/services/surface_pairing.rs:636`), with no keychain delete for the old session.

### Validation of Specified Fix Shape

The ticket's direction is correct: revocation, expiry, and replacement should clean up keychain secrets.

One detail needs revision:

- Do not perform macOS keychain IO inside the SQLite transaction if it can be avoided.
- `revoke_pairing_row` currently runs in transaction contexts (`src-tauri/src/services/surface_pairing.rs:1269` to `src-tauri/src/services/surface_pairing.rs:1272`, `src-tauri/src/services/surface_pairing.rs:1855`).
- Holding the DB writer while invoking the `security` CLI would add local contention to exactly the surface lifecycle path v1.4.2 tried to stabilize.

Local-shaped alternative:

- Commit the DB lifecycle transition first.
- Return the affected `(surface_client_id, session_id)` cleanup targets from the service.
- Delete keychain entries after the DB transaction, best-effort and idempotently.
- Emit cleanup diagnostics and retry on later lifecycle reconciliation if delete fails.

### Fix Shape

1. Add a small cleanup target type.
   - Include `surface_client_id`.
   - Include every active session id affected by revoke, expiry, or replacement.

2. Update lifecycle service functions to return cleanup targets.
   - `revoke_pairing` should return audit event plus cleanup targets, or perform post-commit cleanup before returning.
   - `apply_signed_session_write_action` should clean up keys for `MarkSessionRevoked`.
   - `mark_pairing_expired` should either revoke affected session rows or return affected session ids for cleanup, otherwise expired pairing keys remain live.
   - `revoke_existing_pairing_for_site` should collect old session ids before/while revoking the old pairing.

3. Use `delete_session_master_key` after the DB transition.
   - The delete helper is already idempotent for missing entries (`src-tauri/src/services/surface_session_keychain.rs:148` to `src-tauri/src/services/surface_session_keychain.rs:155`).
   - Log but do not undo the DB lifecycle transition if keychain cleanup fails.

4. Do not use short TTL eviction to clean secrets.
   - Cleanup follows user intent and lifecycle state, not remote-style read expiry.

### Order of Operations

1. Add keychain cleanup target plumbing in `surface_pairing.rs`.
2. Wire explicit revoke, re-pair replacement, and expiry cleanup.
3. Add transition tests and a macOS keychain integration test behind `#[cfg(target_os = "macos")]`.

### Acceptance Test Sketch

- Unit with fake key store: explicit revoke, re-pair, and expiry delete the expected old session keys exactly once.
- Unit: keychain delete failure logs and does not roll back DB revocation.
- macOS integration: after revoke, `security find-generic-password` for the old service/account returns not found.

### Risk and Blast Radius

Files: `src-tauri/src/services/surface_pairing.rs`, `src-tauri/src/services/surface_session_keychain.rs`, possibly `src-tauri/src/surface_runtime/mod.rs`. Risk is partial cleanup and overbroad deletion; mitigate with captured old session ids and idempotent retry. Blast radius is pairing lifecycle and macOS keychain.

## DOS-675, Shutdown Cleanup Runs After Listener Abort

### Confirmed Root Cause

The root cause is confirmed.

Evidence:

- Listener task runs `run_listener`, then removes runtime sentinel, flushes session activity, and marks stopped (`src-tauri/src/surface_runtime/mod.rs:327` to `src-tauri/src/surface_runtime/mod.rs:340`).
- `run_listener` exits when shutdown changes to true (`src-tauri/src/surface_runtime/mod.rs:897` to `src-tauri/src/surface_runtime/mod.rs:908`).
- `stop` sends shutdown and then immediately aborts the task (`src-tauri/src/surface_runtime/mod.rs:415` to `src-tauri/src/surface_runtime/mod.rs:419`).
- `Drop` sends shutdown and then immediately aborts the task (`src-tauri/src/surface_runtime/mod.rs:468` to `src-tauri/src/surface_runtime/mod.rs:475`).
- Sentinel removal lives in the post-listener cleanup block (`src-tauri/src/surface_runtime/mod.rs:329` to `src-tauri/src/surface_runtime/mod.rs:331`), while the remover itself is best-effort (`src-tauri/src/surface_runtime/mod.rs:883` to `src-tauri/src/surface_runtime/mod.rs:894`).
- Session activity flush also lives in the post-listener cleanup block (`src-tauri/src/surface_runtime/mod.rs:332` to `src-tauri/src/surface_runtime/mod.rs:338`) and performs the DB updates (`src-tauri/src/surface_runtime/mod.rs:753` to `src-tauri/src/surface_runtime/mod.rs:782`).

Because `abort()` cancels the spawned task, the cleanup block after `run_listener` is not reliable on normal stop/drop.

### Validation of Specified Fix Shape

The ticket's diagnosis is correct. The smaller "run cleanup synchronously on stop/drop" option needs refinement:

- Sentinel cleanup is synchronous and can run in `stop` and `Drop`.
- Session activity flush is async and requires `AppState` (`src-tauri/src/surface_runtime/mod.rs:753` to `src-tauri/src/surface_runtime/mod.rs:782`).
- `RunningEndpoint` currently stores shutdown sender and abort handle, but not the join handle or app state (`src-tauri/src/surface_runtime/mod.rs:182` to `src-tauri/src/surface_runtime/mod.rs:188`).
- A sync `Drop` cannot await the DB flush.

Preferred local-shaped fix:

- Make sentinel cleanup idempotent and call it before abort in `stop` and `Drop`.
- Store enough state to gracefully join the listener task in normal async shutdown paths.
- Use a bounded timeout, then abort only as fallback.
- Keep session activity flush as shutdown-only best-effort; do not reintroduce read-path `last_seen_at` writes.

### Fix Shape

1. Move sentinel cleanup to an explicit helper called by:
   - the graceful listener task post-loop,
   - `SurfaceEndpointState::stop`,
   - `Drop`.

2. Add a graceful async stop path.
   - Store `JoinHandle<()>` or a cleanup task handle, not only `AbortHandle`.
   - Send shutdown.
   - Await join with a small timeout.
   - If timeout fires, run sentinel cleanup and abort.

3. Keep DB activity flush out of read path.
   - It can stay in graceful post-loop cleanup.
   - If the normal Tauri shutdown path can call async stop, flush there.
   - `Drop` should not block on DB writer.

4. Make cleanup idempotent.
   - `remove_runtime_sentinel` already treats missing sentinel as success (`src-tauri/src/surface_runtime/mod.rs:883` to `src-tauri/src/surface_runtime/mod.rs:894`).
   - Reuse that behavior.

### Order of Operations

1. Add failing tests for sentinel cleanup after `stop`.
2. Store listener join handle or shutdown task owner and implement graceful async stop.
3. Keep `Drop` as best-effort sentinel cleanup plus abort fallback, and verify flush remains shutdown-only.

### Acceptance Test Sketch

- Unit: after `stop`, sentinel file is removed even if listener task is aborted.
- Unit: graceful stop invokes `flush_session_activity_on_shutdown`; forced abort still removes sentinel.
- Integration: start runtime, read sentinel, stop runtime, verify sentinel gone and active session `last_seen_at` advances only on shutdown.

### Risk and Blast Radius

File: `src-tauri/src/surface_runtime/mod.rs`. Risk is shutdown hang if join is unbounded, so use a timeout. `Drop` cannot perform async DB work. Blast radius is runtime endpoint lifecycle only.

## Coordination

### Shared Fix Surfaces

DOS-671 and DOS-672 share these files:

- `wp/dailyos/blocks/account-overview/edit.js`
- `wp/dailyos/blocks/account-overview/render-functions.php`
- `wp/dailyos/includes/class-dailyos-plugin.php`
- `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`
- `src-tauri/src/surface_runtime/mod.rs`
- `src-tauri/src/services/composition_render_orchestrator.rs`
- `src-tauri/src/bridges/surface_client.rs`

They should land together because:

- Single-fetch preview changes must align with editor reload behavior.
- Runtime cache/rate behavior affects the same reload path.
- Testing one without the other can produce false confidence, because a fixed editor can still be destabilized by the double-fetch PHP route or render-path rate budget.

DOS-673 and DOS-674 share these files:

- `src-tauri/src/services/surface_session_keychain.rs`
- `src-tauri/src/services/surface_pairing.rs`
- `src-tauri/src/surface_runtime/mod.rs`

They should land together because:

- DOS-673 changes keychain lookup classification.
- DOS-674 changes keychain deletion on lifecycle transitions.
- Both need a common test seam for keychain operations, otherwise tests will duplicate brittle macOS `security` command setup.

DOS-675 shares lifecycle behavior with DOS-673 and DOS-674 through:

- `src-tauri/src/surface_runtime/mod.rs`
- startup rehydration,
- shutdown session activity flush,
- runtime sentinel discovery used by WP session refresh.

It should land in the same stabilization PR if practical because:

- A stale sentinel can make WP refresh hit the wrong port.
- A keychain lookup false error can make refresh fail after restart.
- A leaked key can hide lifecycle bugs because old local secrets remain available.

### Recommended Landing Shape

Land as one v1.4.3 stabilization PR with three commit groups:

1. WP preview/reload stabilization for DOS-671 and DOS-672.
2. Surface session keychain lifecycle hardening for DOS-673 and DOS-674.
3. Runtime shutdown cleanup for DOS-675.

If the PR needs to split for review size, split only after the shared test seams exist:

1. PR A: DOS-673, DOS-674, DOS-675 lifecycle hardening.
2. PR B: DOS-671, DOS-672 WP preview/runtime render stabilization.

Do not land DOS-671 without DOS-672, and do not land DOS-673 without at least the test seam needed for DOS-674.

### Tickets Whose Specified Fix Shape Needs Revision

- DOS-671: the ticket's candidate TTL/cache framing should be revised. TTL is already local-sized. The fix should target editor reload lifecycle, PHP double fetch, render-path writes, cache key versioning, and render-read rate-budget gating.
- DOS-672: same revision as DOS-671. Do not frame the second reload as session evaporation until logs prove it. The confirmed code path is double fetch plus auto reload plus generic verification-banner mapping.
- DOS-674: direction is correct, but "delete in the same transaction" should be revised to "DB transition first, then idempotent keychain cleanup outside the SQLite transaction."
- DOS-675: diagnosis is correct, but "synchronous stop/drop cleanup" should be split into sync sentinel cleanup and async graceful session-activity flush.

DOS-673 does not need conceptual revision. It needs one implementation expansion: classify malformed existing keychain payload separately from true not-found.
