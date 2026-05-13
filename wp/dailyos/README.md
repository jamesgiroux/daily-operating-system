# DailyOS WordPress Plugin

This is the W3-A scaffold for the DailyOS WordPress SurfaceClient. It provides the plugin entrypoint, lifecycle hooks, inventory-backed ability registration shell, admin page shells, WP-CLI command namespace, projection envelope contract, and safety grep gates.

DailyOS for WordPress is a shell, not a substrate. WordPress renders and mediates interactions, while the paired Tauri runtime remains authoritative for abilities, claims, feedback, enrichment, and composition.

Pairing depends on the local DailyOS Tauri runtime and is intentionally not implemented in this scaffold. The pairing handshake, runtime client, and HMAC transport land in the next wave.

Reference links:

- [W3 packet](../../.docs/plans/dos-546/v1.4.2-project/w3-0-rsm-second-brain-mapping.md)
- [DOS-564](../../.docs/plans/dos-546/v1.4.2-project/02-issues.md#dailyos-wordpress-plugin-skeleton--wp-abilities-api-registration)
- [DOS-563](../../.docs/plans/dos-546/v1.4.2-project/02-issues.md#wp-side-runtime-client--hmac-signer--pairing-ui)
