---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 13
related_adrs: [0102, 0111, 0128, 0129, 0130]
open_questions: see ./INDEX.md (routed to W3-A L0 Prep)
---

# 13 — DailyOS WordPress plugin skeleton

## Summary

The DailyOS WordPress plugin is the WordPress-side `SurfaceClient::WordPress` for the DailyOS runtime.

- Plugin name: `DailyOS`
- Slug: `dailyos`
- Shape: single plugin, multi-feature
- Runtime dependency: paired local DailyOS Rust runtime
- WordPress dependency: WP 6.9+ Abilities API, with exact hooks verified in Phase 1

It provides pairing, ability proxy registration, server-rendered DailyOS Gutenberg blocks, a DailyOS-controlled MCP endpoint, save-time feedback routing, HMAC-signed runtime transport, presence-nonce enforcement, and admin diagnostics.

It does not provide a second substrate, a PHP implementation of abilities, a PHP MCP client to the runtime, direct claim creation, arbitrary writes, or JavaScript access to raw runtime credentials. WordPress renders and mediates interactions; the DailyOS runtime remains the substrate and composition authority.

## Plugin header

The plugin entry file is `dailyos/dailyos.php`.

```php
/*
Plugin Name: DailyOS
Plugin URI: <TBD>
Description: DailyOS SurfaceClient for WordPress Studio and local WordPress installs.
Version: 0.1.0
Requires at least: 6.9
Requires PHP: 8.1
Author: <TBD>
Text Domain: dailyos
*/
```

`Requires at least` assumes standard WP 6.9 Abilities API support. Verify exact minimum and field names against current WP Abilities API docs in Phase 1.

## Directory structure

Expected plugin layout:

```text
dailyos/
├── dailyos.php                          # Plugin entry — header + bootstrap
├── readme.txt                           # WP repo readme
├── composer.json                        # PSR-4 autoloader + deps
├── includes/
│   ├── class-dailyos-plugin.php         # Main singleton
│   ├── class-dailyos-pairing.php        # SurfaceClient handshake
│   ├── class-dailyos-runtime-client.php # PHP-side HTTP client to Rust runtime
│   ├── class-dailyos-hmac-signer.php    # HMAC per artifact 08
│   ├── class-dailyos-presence-nonce.php # Per artifact 10
│   ├── class-dailyos-feedback-router.php# Save-handler routing per artifact 11
│   ├── class-dailyos-ability-registry.php
│   ├── class-dailyos-mcp-server.php     # Custom MCP server with allowlist
│   └── traits/
├── blocks/
│   ├── account-overview/                # First block per artifact 14
│   │   ├── block.json
│   │   ├── render.php
│   │   ├── edit.js
│   │   ├── save.js
│   │   ├── editor.scss
│   │   └── style.scss
│   └── shared/                          # Shared block utilities
├── abilities/
│   └── (per-ability PHP shims if any; abilities live in Rust runtime)
├── admin/
│   ├── pages/
│   │   ├── pairing.php                  # Initial pairing UI
│   │   └── settings.php
│   └── assets/
├── languages/
└── vendor/                              # composer install target
```

Responsibilities:

- `dailyos.php` defines plugin constants, loads Composer, registers lifecycle hooks, and boots `DailyOS_Plugin`.
- `readme.txt` is WordPress repository/package metadata and user-facing installation text.
- `composer.json` declares PHP constraints, dependencies, and PSR-4 autoloading.
- `includes/` owns pairing, runtime transport, HMAC signing, nonce storage, feedback routing, ability registration, MCP configuration, and plugin orchestration.
- `includes/traits/` is reserved for small shared traits if implementation duplication justifies them.
- `blocks/` contains block packages. Each block has metadata, editor assets, front-end assets, and optional server render code.
- `blocks/shared/` contains reusable block helpers for composition normalization, preview loading, and shared rendering utilities.
- `abilities/` contains optional PHP shim classes only when an ability registration needs more structure than the registry class. Canonical ability logic stays Rust-side.
- `admin/pages/` contains thin templates for pairing and settings pages.
- `admin/assets/` contains admin-only CSS/JS.
- `languages/` contains translation catalogs for the `dailyos` text domain.
- `vendor/` is Composer output and is never hand-edited.

## composer.json

Expected package shape:

```json
{
  "name": "dailyos/dailyos-wordpress",
  "description": "DailyOS SurfaceClient plugin for WordPress.",
  "type": "wordpress-plugin",
  "license": "<TBD>",
  "require": {
    "php": ">=8.1",
    "guzzlehttp/guzzle": "^7.9"
  },
  "require-dev": {
    "squizlabs/php_codesniffer": "^3.10",
    "wp-coding-standards/wpcs": "^3.1",
    "phpunit/phpunit": "^10.5"
  },
  "autoload": {
    "psr-4": {
      "DailyOS\\": "includes/"
    }
  }
}
```

- Use `guzzlehttp/guzzle` for loopback HTTP unless Phase 1 chooses WordPress HTTP APIs to reduce dependencies.
- Do not add `paragonie/random_compat` while PHP 8.1 is the minimum.
- Do not add a PHP MCP client dependency. WordPress registers abilities and exposes a DailyOS-controlled MCP endpoint; the plugin does not consume MCP from PHP.
- Keep dependencies small because the plugin runs inside arbitrary local WordPress installs.

## Class autoloader

Autoloading is Composer PSR-4:

```text
DailyOS\ => includes/
```

Files use WordPress-style names such as `class-dailyos-plugin.php`; classes are namespaced under `DailyOS`. The main class is `DailyOS\DailyOS_Plugin`, referred to in this artifact as `DailyOS_Plugin`.

Bootstrap sequence:

1. `dailyos.php` defines `DAILYOS_PLUGIN_FILE`, `DAILYOS_PLUGIN_DIR`, `DAILYOS_PLUGIN_URL`, and `DAILYOS_VERSION`.
2. `dailyos.php` requires `vendor/autoload.php`.
3. `dailyos.php` registers activation, deactivation, and uninstall callbacks.
4. `dailyos.php` calls `DailyOS_Plugin::instance()->init()`.

## Main plugin class — `DailyOS_Plugin`

`DailyOS_Plugin` is the WordPress composition root. It is a singleton with a private constructor, lazy service construction, and explicit lifecycle methods.

Lifecycle:

- `init()` registers hooks and delegates feature setup.
- `activate()` initializes pairing state when missing and schedules stale-nonce cleanup.
- `deactivate()` stops cron, flushes local snapshots/caches, and optionally revokes pairing.
- `uninstall()` deletes plugin-owned data.

Public method signatures:

```php
DailyOS_Plugin::instance(): DailyOS_Plugin
DailyOS_Plugin::init(): void
DailyOS_Plugin::activate(): void
DailyOS_Plugin::deactivate(): void
DailyOS_Plugin::uninstall(): void
DailyOS_Plugin::register_abilities(): void
DailyOS_Plugin::register_blocks(): void
DailyOS_Plugin::register_admin_pages(): void
DailyOS_Plugin::register_mcp_server_config(): void
DailyOS_Plugin::register_save_hooks(): void
DailyOS_Plugin::register_rest_routes(): void
```

- `register_abilities()` calls into the WP Abilities API to register DailyOS abilities as proxies into the Rust runtime.
- `register_blocks()` registers Gutenberg blocks, starting with `dailyos/account-overview` from artifact 14.
- `register_admin_pages()` registers pairing UI and settings.
- `register_mcp_server_config()` registers the custom MCP server and denies DailyOS tools on the default WP MCP server per ADR-0128.
- `register_save_hooks()` wires artifact 11's feedback router to `save_post`.
- `register_rest_routes()` registers admin REST endpoints for pairing status, pairing completion, preview, diagnostics, and MCP.

## Integration points

### WP Abilities API integration

DailyOS registers WordPress abilities as PHP proxies for Rust runtime abilities. This assumes standard WP 6.9 Abilities API hooks; exact names must be verified against current docs in Phase 1.

Discovery:

- On successful pairing, the plugin calls `GET /v1/abilities` on the runtime.
- The runtime returns ADR-0102 ability descriptors, schema metadata, and `AbilityPolicy`.
- The plugin stores a short-lived local descriptor snapshot for registration and diagnostics.
- On WordPress `init`, the plugin registers only descriptors currently allowed for this paired SurfaceClient.

Runtime descriptor shape:

```json
{
  "name": "account_overview",
  "description": "Returns the current account overview composition.",
  "category": "Read",
  "input_schema": {},
  "output_schema": {},
  "policy": {
    "allowed_actors": ["SurfaceClient"],
    "allowed_modes": ["Live"],
    "required_scopes": ["read.account_overview"],
    "mcp_exposure": true,
    "requires_confirmation": false,
    "may_publish": false,
    "idempotent": true
  },
  "client_side_executable": false
}
```

Each registered WP ability uses:

- `name`: `dailyos/<ability-name>`
- `description`: runtime descriptor description
- `scope`: derived from `AbilityPolicy.required_scopes`
- `executor`: PHP shim that calls `DailyOS_Runtime_Client`
- `client_side_executable`: explicit per-ability boolean
- `mcp_exposure`: explicit per-ability boolean

Executor responsibilities:

1. Verify the WordPress capability required for the local operation.
2. Load and validate the pairing record.
3. Confirm granted DailyOS scopes cover all `required_scopes`.
4. For write/feedback abilities, verify a fresh artifact 10 presence nonce.
5. Invoke `DailyOS_Runtime_Client::invoke_ability()`.
6. Return the runtime response in WP Abilities API shape.

- `SurfaceClient` exposure is opt-in through `allowed_actors`.
- DailyOS scopes and WordPress capabilities both must pass.
- MCP-mediated discovery and invocation require `mcp_exposure: true`.
- Unauthorized abilities do not appear in WordPress ability discovery.
- Descriptions are model-facing and browser-facing API text and should follow ADR-0128 copy discipline.

### Gutenberg block registration

Each block in `blocks/` is registered from metadata:

```php
register_block_type_from_metadata( DAILYOS_PLUGIN_DIR . 'blocks/account-overview' )
```

The first block is `dailyos/account-overview` from artifact 14.

Block flow:

1. `block.json` declares name, attributes, scripts, styles, and render callback.
2. `render.php` invokes the corresponding WP ability proxy server-side.
3. The WP ability executor calls the Rust runtime through `DailyOS_Runtime_Client`.
4. The runtime returns an ADR-0130 `Composition`.
5. The renderer maps `Composition` blocks to Gutenberg markup and preserves `claim_refs` needed for feedback.

Editor flow:

- `edit.js` uses a scope-limited admin REST or `admin-ajax.php` endpoint for previews.
- The preview endpoint proxies through the PHP runtime client.
- JavaScript never receives raw bearer, HMAC key, or master key material.
- If WP 7.0 client-side `executeAbility()` is available, Phase 1 may use it only for abilities marked `client_side_executable: true`.

### Custom MCP server registration

DailyOS abilities are MCP-exposed only via the DailyOS-controlled MCP head, not through WordPress's default MCP server.

The plugin registers:

```text
/wp-json/dailyos/mcp/v1
```

- Requires runtime-issued bearer.
- Requires a valid paired `SurfaceClient::WordPress` instance.
- Lists only abilities with `mcp_exposure: true`.
- Applies the same `required_scopes` filtering as WP Abilities API registration.
- Invokes abilities through `DailyOS_Runtime_Client`.
- Logs SurfaceClient instance identity for every invocation.

Default WP MCP denial:

- Add a deny filter for `dailyos/*` tools on the default WP MCP server.
- Candidate hook: `pre_get_mcp_server_tools`.
- Verify exact hook/filter name against current WP MCP Adapter and WP Abilities API docs in Phase 1.

- Positive-only. No runtime descriptor means no MCP exposure.
- `mcp_exposure: false` denies MCP listing and invocation even when `SurfaceClient` is otherwise allowed.
- Local WP administrator permissions do not override DailyOS scope grants.

### Save-handler hookup

The plugin wires save events into artifact 11's feedback router:

```php
add_action( 'save_post', [ DailyOS_Feedback_Router::class, 'route' ], 20, 3 )
```

`DailyOS_Feedback_Router` inspects saved Gutenberg blocks, compares them with `_dailyos_composition_snapshot`, converts allowed deltas into typed feedback events, rejects raw editor diffs as direct writes, requires artifact 10 presence nonces, dispatches through `DailyOS_Runtime_Client`, and updates snapshots only after runtime acknowledgement.

### Pairing UI hookup

The plugin adds a WordPress admin page:

```php
add_menu_page( 'DailyOS', 'DailyOS', 'manage_options', 'dailyos-pairing', ... )
```

The page shows runtime handshake state, displays a one-time pairing code, lets an administrator set or confirm the loopback endpoint, completes the pairing exchange, stores the resulting bearer and derived key server-side, shows granted scopes, and provides a revoke/reset action.

Security requirements:

- Pairing requires `manage_options`.
- The master key is fetched through a `manage_options`-gated WP filter per artifact 08.
- Bearer and derived keys are never printed into admin HTML.
- Diagnostics redact secrets by default.

## Activation hooks

Activation behavior:

- Check for an existing pairing record.
- If missing, set pairing status to `needs_pairing`.
- Initialize the runtime endpoint option to the local loopback candidate.
- Register cron for the nonce-store sweep.
- Create a consumed-nonce store if artifact 10 chooses a custom table.

Activation must not auto-pair, fetch broad runtime state, expose abilities before pairing, or make non-loopback network calls by default.

## Deactivation hooks

Deactivation behavior:

- Stop nonce cleanup cron.
- Flush short-lived projection caches.
- Flush local ability descriptor snapshots.
- Optionally revoke runtime-side pairing if configured.
- Preserve durable pairing/settings options unless revoke-on-deactivate or uninstall is requested.

Deactivation must not delete durable admin configuration by default or mutate DailyOS substrate claims.

## Hooks reference (summary)

| Action/filter | Callback | Priority | Purpose |
|---|---:|---:|---|
| `plugins_loaded` | `DailyOS_Plugin::instance()->init` | default | Bootstrap after Composer loads. |
| `init` | `DailyOS_Plugin::register_abilities` | 10 | Register DailyOS proxy abilities. Verify exact hook in Phase 1. |
| `init` | `DailyOS_Plugin::register_blocks` | 11 | Register Gutenberg blocks from metadata. |
| `admin_menu` | `DailyOS_Plugin::register_admin_pages` | 10 | Add pairing and settings pages. |
| `rest_api_init` | `DailyOS_Plugin::register_rest_routes` | 10 | Register pairing, preview, diagnostics, and MCP routes. |
| `save_post` | `DailyOS_Feedback_Router::route` | 20 | Route block save deltas to feedback events. |
| `dailyos_nonce_sweep` | `DailyOS_Presence_Nonce::sweep` | default | Clean stale consumed nonces. |
| `pre_get_mcp_server_tools` | `DailyOS_MCP_Server::deny_default_dailyos_tools` | 10 | Deny `dailyos/*` on default WP MCP server. Verify in Phase 1. |
| `dailyos_runtime_master_key` | `DailyOS_HMAC_Signer::resolve_master_key` | 10 | Fetch master key through artifact 08 filter. |
| `register_activation_hook` | `DailyOS_Plugin::activate` | n/a | Initialize pairing state and scheduler. |
| `register_deactivation_hook` | `DailyOS_Plugin::deactivate` | n/a | Stop cron and flush local caches. |
| `register_uninstall_hook` | `DailyOS_Plugin::uninstall` | n/a | Full plugin data cleanup. |

## Data storage in WP

Options:

- `dailyos_pairing_status`: `needs_pairing`, `paired`, `revoked`, or `error`.
- `dailyos_pairing_record`: encrypted instance ID, bearer reference, derived key reference, granted scopes, issued time, and expiry.
- `dailyos_runtime_endpoint`: local runtime endpoint URL.
- `dailyos_runtime_capabilities_snapshot`: short-lived ability descriptor snapshot.
- `dailyos_plugin_settings`: per-site plugin config.

Post meta:

- `_dailyos_composition_snapshot`: last acknowledged composition projection for artifact 11.
- `_dailyos_claim_refs`: optional normalized index of claim refs rendered into a post.
- `_dailyos_rendered_at`: projection freshness diagnostics.

Transients:

- `dailyos_projection_cache_*`: short-lived render/projection caches.
- `dailyos_pairing_challenge_*`: temporary pairing challenge state.
- `dailyos_runtime_health_*`: short-lived runtime health checks.

Custom table:

- Artifact 10 may require `{$wpdb->prefix}dailyos_consumed_nonces`.
- Required fields if used: nonce hash, SurfaceClient instance ID, action, created time, expires time.
- If artifact 10 proves transients are sufficient for Phase 0, defer the table.

Cleanup:

- Uninstall deletes plugin-owned `dailyos_*` options, transients, custom tables, and `_dailyos_*` post meta.
- Deactivation keeps durable options unless revoke-on-deactivate is enabled.
- Cache expiration never substitutes for runtime truth.

## Security boundaries

- All ability invocations go through `DailyOS_Runtime_Client`.
- Runtime requests are HMAC-signed per artifact 08.
- All save-time feedback events carry a presence nonce per artifact 10.
- Presence nonce checks happen before feedback dispatch and again at the runtime boundary.
- No raw bearer, derived key, or master key is shipped to JavaScript.
- Block `render.php` calls are server-side.
- `edit.js` uses a separate, scope-limited admin endpoint that proxies through PHP.
- WordPress capabilities and DailyOS scopes both must pass.
- SurfaceClient instance identity is logged for every read and write.
- `mcp_exposure: false` denies MCP listing and invocation regardless of WP permissions.
- Default WP MCP listing for `dailyos/*` is explicitly denied.
- Runtime errors must not leak secrets, bearer fragments, filesystem paths, or HMAC material.

## Test fixtures (skeleton-level)

Activation:

- No pairing options creates `dailyos_pairing_status = needs_pairing`.
- Missing cleanup event schedules `dailyos_nonce_sweep`.
- Existing pairing record is not overwritten.

Deactivation:

- Cron event is removed.
- Projection transients are cleared.
- Pairing options persist unless revoke-on-deactivate is enabled.

Pairing:

- Pairing page requires `manage_options`.
- Pairing completion stores encrypted pairing state.
- Diagnostics redact bearer and key material.

Ability registration:

- `GET /v1/abilities` descriptors register `dailyos/<ability-name>` entries after pairing.
- Abilities lacking `SurfaceClient` in `allowed_actors` are not registered.
- Abilities outside granted scopes are not registered.
- Unpaired runtime exposes no runtime-backed abilities.

Gutenberg block:

- `dailyos/account-overview` registers from metadata.
- Server render calls the WP ability proxy.
- Rendered markup preserves claim refs for artifact 11.

Custom MCP server:

- `/wp-json/dailyos/mcp/v1` requires runtime-issued bearer.
- Tool list includes only `mcp_exposure: true`.
- Tool list excludes abilities outside granted scopes.
- Invocation of unlisted `dailyos/*` fails authorization.

Default WP MCP denial:

- Default WP MCP server does not list `dailyos/*`.
- Direct default-server invocation of `dailyos/*` is rejected.
- Deny hook gets a test after exact WP MCP Adapter hook verification.

Save handler:

- `save_post` routes to `DailyOS_Feedback_Router::route` at priority 20 with three args.
- Unchanged DailyOS blocks emit no feedback event.
- Allowed correction emits typed feedback with presence nonce.
- Raw content diff without valid nonce is rejected.

Nonce store:

- Consumed nonce cannot be reused.
- Expired nonces are swept.
- Nonce records are scoped to SurfaceClient instance and action.

## Interaction with other Wave 2 artifacts

- Artifact 08: HMAC signing and master-key handling through `DailyOS_HMAC_Signer`.
- Artifact 09: runtime transport contract implemented by `DailyOS_Runtime_Client`.
- Artifact 10: user-presence nonce implemented by `DailyOS_Presence_Nonce`.
- Artifact 11: save-handler routing wired through `DailyOS_Feedback_Router`.
- Artifact 14: first Gutenberg block at `blocks/account-overview`.
- Artifact 15: packaging/distribution expectations for Composer, readme, translations, and dependency boundaries.

## Open questions

- What are the exact WP 6.9 Abilities API registration functions, hook names, and metadata fields?
- What is the exact WP MCP Adapter hook/filter for denying tool listings and invocations?
- Should runtime calls use Guzzle or WordPress HTTP APIs?
- What protects `dailyos_pairing_record` in local WordPress installs without a managed secrets service?
- Should consumed nonces use a custom table in Phase 1, or are transients sufficient for the spike?
- What is artifact 09's final loopback transport: HTTP port, Unix domain socket bridge, or fallback stdio?
- How does multisite map to SurfaceClient identity: per site, per network, or configurable?
- Which abilities may be `client_side_executable` once client-side `executeAbility()` is available?
- Is WP 6.9 enough, or does editor ergonomics require WP 7.0 client-side Abilities APIs?
- Does uninstall revoke runtime pairing automatically or only delete local credentials?
- What diagnostics are safe for administrators without leaking substrate or filesystem details?
- How should Composer dependencies be packaged for WordPress.org distribution?
