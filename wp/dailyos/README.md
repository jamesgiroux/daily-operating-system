# DailyOS WordPress Plugin

DailyOS is the WordPress SurfaceClient shell for the W3 bundle. It is not the PHP substrate: WordPress renders and mediates surface interactions while the paired DailyOS runtime remains authoritative for abilities, scopes, claims, feedback, enrichment, and composition.

## What Shipped

- Plugin bootstrap in `dailyos.php` with the `DailyOS_Plugin` composition root.
- Inventory-backed WP Abilities API registration from `tools/dailyos-abilities.json`.
- Admin pairing and settings pages for SurfaceClient status.
- WP-side runtime client with HMAC-signed loopback requests through the WordPress HTTP API.
- Custom MCP server integration using `wordpress/mcp-adapter` v0.5.0.
- Dedicated `dailyos_substrate` role and user for MCP requests.
- WP-CLI diagnostics under `wp dailyos`.
- PHPCS, PHPUnit, and grep-gate test harnesses.

## Pairing Flow

Pairing starts in the WordPress admin page. An administrator enters the runtime pairing code, WordPress calls the local runtime handshake endpoint, and the plugin stores one non-secret marker in `wp_options.dailyos_pairing_marker`.

The marker contains `marker_version`, `runtime_instance_id`, `site_nonce_hash`, `projection_version`, `instance_id`, `session_id`, granted scopes, endpoint version, and pairing/last-use timestamps. It is a namespace recovery hint only. Runtime state, signatures, and scope checks remain the source of trust.

## Runtime Client

`DailyOS_Runtime_Client` sends JSON as pre-serialized string bodies through `wp_remote_post()`. Signed calls include the W2 HMAC headers and preserve the six transport caveats covered by the tests: exact bytes, string body, no redirects, bounded timeout, no browser-direct runtime calls, and no HTTP argument mutation hooks.

Session material is request-local. The plugin retrieves it through the gated `dailyos_wp_bridge_session_key` filter and never persists HMAC keys, bearer values, derived keys, pairing tokens, or session keys in WordPress storage or browser-visible state.

## MCP Server

The plugin registers a DailyOS-specific MCP server with `wordpress/mcp-adapter` v0.5.0. The server uses an explicit allowlist from the ability inventory, defaults to Read/Transform invocable abilities, and filters DailyOS tools out of non-DailyOS MCP server listings.

MCP requests run as the dedicated `dailyos_substrate` WordPress user. Each invocable tool checks both the `dailyos_invoke_mcp_ability` capability and the resolved DailyOS SurfaceClient scopes before runtime invocation. Metadata-only abilities can be enumerated by DailyOS metadata flows but cannot be invoked.

## WP-CLI

Available commands:

- `wp dailyos status`
- `wp dailyos repair-namespace`
- `wp dailyos repair-projection`

These commands are plugin diagnostics and repair helpers. They do not duplicate Studio host-layer operations.

## Production vs Studio Dev

Production documentation must use standard WordPress plugin install flows, standard WP-CLI, and normal WordPress distribution paths.

Studio tooling is for local development and clean-machine validation only. Do not put `wp-studio`, `studio mcp wp_cli`, Studio import/export, or blueprint re-apply commands in production runbooks or end-user instructions.

## Test Harness

Run the required gates from `wp/dailyos`:

```bash
vendor/bin/phpcs --standard=phpcs.xml.dist
vendor/bin/phpunit --no-coverage
bash scripts/run-grep-gates.sh
```

The grep gates enforce raw database access boundaries, filesystem write boundaries, no transport-secret persistence, string-only `wp_remote_post()` bodies, and no ephemeral issue references in PHP comments.

## References

- W3 L0 packet V4: `../../.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`
- DOS-564, DOS-565, DOS-566 contracts: `../../.docs/plans/dos-546/v1.4.2-project/02-issues.md`
- W3-0 spike artifacts: `../../.docs/plans/dos-546/v1.4.2-project/w3-0-*`
