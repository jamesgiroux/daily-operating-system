# W3-0 Studio Blueprint v0

This directory contains `w3-0-blueprint-v0.json`, the W3-0 output from the W3 L0 packet V4. W6-B consumes it when assembling the clean-machine DailyOS bundle and validating the `Studio + plugin + theme + runtime` path.

## Blueprint Contract

The blueprint uses the public WordPress Playground / Studio schema at `https://playground.wordpress.net/blueprint-schema.json`. Studio Blueprints follow the Playground format, including `preferredVersions`, install/activate steps, `setSiteOptions`, and `wp-cli` steps.

WordPress is pinned to `6.9.4`, which WordPress.org listed as the latest 6.9 branch release on 2026-05-13. E agent must re-confirm before V5 and verify Studio accepts exact point pins in `preferredVersions.wp`; if Studio only accepts major minors, amend to `6.9` and record the resolved core version in W6-B evidence.

PHP is pinned to `8.4`, matching the Studio default for new sites.

## Step Order

| Order | Step | Purpose |
| --- | --- | --- |
| 1 | `installPlugin` | Installs `resources/dailyos.zip` from VFS into `wp-content/plugins/dailyos`. |
| 2 | `installTheme` | Installs `resources/dailyos-magazine.zip` from VFS into `wp-content/themes/dailyos-magazine`. |
| 3 | `activatePlugin` | Activates `dailyos/dailyos.php`, matching Phase 0 artifact 13. |
| 4 | `activateTheme` | Activates the `dailyos-magazine` block theme. |
| 5 | `setSiteOptions` | Seeds DailyOS defaults: placeholder pairing endpoint, unpaired status, admin-notice flag, and blueprint provenance. |
| 6 | `wp-cli` | Runs `wp dailyos status --format=json` to fail fast if plugin activation is unhealthy. |
| 7 | `wp-cli` | Adds a one-time admin notice telling the user to pair the local runtime. |

## Required Artifacts

Place these zip files in the blueprint bundle VFS before applying it:

| Path | Source | Packaging requirement |
| --- | --- | --- |
| `resources/dailyos.zip` | W3 plugin package from `wp/dailyos/` after W3-A/W3-B/W3-C merge gates. | Zip must expand to top-level `dailyos/` and include `dailyos/dailyos.php`, Composer vendor output, built assets, and `wp dailyos status`. |
| `resources/dailyos-magazine.zip` | W5-B magazine theme package from `wp/dailyos-magazine/`. | Zip must expand to top-level `dailyos-magazine/`, include `theme.json`, built theme assets, and pass Theme Check. |

The Tauri runtime app is not installed by this blueprint. W6-B owns the runtime launcher, pairing-code display, and final bootstrap wrapper.

## Studio CLI Use

Run the command from the bundle root that contains the blueprint and `resources/` directory:

```sh
studio site create --name "DailyOS W3-0" --blueprint .docs/plans/dos-546/v1.4.2-project/w3-0-blueprint-v0.json --path ~/Studio/dailyos-w3-0
```

After creation, W6-B should start the runtime, complete the WP admin pairing flow, then verify:

```sh
studio wp dailyos status --path ~/Studio/dailyos-w3-0
```

## V5 Amendments To Confirm

- Confirm `preferredVersions.wp: "6.9.4"` works in the installed Studio build; otherwise use `6.9` and document the resolved patch version.
- Confirm `studio site create --blueprint` is the final CLI flag shape for the W6-B target Studio version.
- Confirm VFS resource packaging. If Studio expects self-contained Blueprint bundles instead, switch `resource: "vfs"` to `resource: "bundled"` while keeping the same placeholder paths.
- Replace `http://127.0.0.1:PORT` if the bootstrap can inject the actual runtime pairing endpoint; otherwise leave the site unpaired and rely on the one-time notice.
- Align option names and the one-time notice option with the W3-B plugin implementation.
- Confirm `wp dailyos status --format=json` exists, returns non-zero on unhealthy activation, and does not require a paired runtime for the activation-health check.
- Add checksum metadata for both zip artifacts once W6-B packaging produces deterministic archives.
