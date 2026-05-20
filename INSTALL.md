# DailyOS — Clean-Machine Install (macOS)

> Target time-to-first-render: **≤15 minutes** of user time on a fresh macOS user account with no prior DailyOS or Studio state.

This walkthrough installs DailyOS Tauri app + WordPress Studio + DailyOS plugin + magazine theme, pairs them, and renders your first `dailyos/account-overview` block. If you hit a snag, [`dailyos doctor`](#troubleshooting) gives a structured report.

---

## Prerequisites

| Component | Tested version | Where it comes from |
|---|---|---|
| macOS | 14+ (Sonoma) | Apple |
| WordPress Studio | 1.9.0+ | https://developer.wordpress.com/studio/ |
| Node.js | 24.x | nvm or direct install (only needed if building from source) |
| pnpm | 9.x | `corepack enable pnpm` |
| Rust | 1.81+ | rustup (only needed if building from source) |

DailyOS itself ships as a notarized macOS app — no Rust/Node toolchain required for the install path. Source build is only for contributors.

## Path A — Signed installer (default)

1. **Download the DailyOS dmg** from the release artifact URL (TBD: release process).
2. Mount the dmg, drag DailyOS to `/Applications`.
3. Launch DailyOS. First launch prompts for keychain access — accept (DailyOS stores HMAC session keys in macOS keychain).
4. The app creates `~/.dailyos/` at mode `0700` and writes `~/.dailyos/runtime-endpoint.json` once the surface runtime binds a port.

Verify with `dailyos doctor pairing`:

```
$ dailyos doctor pairing
dailyos doctor pairing: ok
runtime_endpoint=present (port=NNNNN, runtime_version=1.4.3)
audit_log=writable
```

If the runtime endpoint is `absent`, the DailyOS app isn't running. Launch it.

## Path B — Source build (contributors only)

```sh
git clone <repo-url> dailyos
cd dailyos
pnpm install
pnpm dev   # Tauri dev + frontend
```

The dev runtime writes the sentinel to the same `~/.dailyos/runtime-endpoint.json` path.

## Studio + plugin setup

1. **Install WordPress Studio** from https://developer.wordpress.com/studio/. Studio is the macOS local-development WordPress environment DailyOS pairs with.
2. **Create a Studio site**. Name it whatever you like (`dailyos-dev` works).
3. **Install the DailyOS plugin**. Either:
   - Drop the plugin zip into `wp-content/plugins/` and activate via WP-CLI:
     ```sh
     cd ~/Studio/<site-name>
     wp plugin activate dailyos
     ```
   - Or via the WP admin UI: Plugins → Add Plugin → Upload → activate.
4. **Optional: install the magazine theme** from `wp/dailyos/theme/`. Same path: symlink or upload, then activate via `wp theme activate dailyos-magazine`.

## Pairing

1. In the DailyOS app, open **Settings → Pairing → Generate Code**. An 8-digit pairing code displays.
2. In the WordPress admin, open **DailyOS → Pair with Runtime**, paste the code, click **Pair**.
3. The WP plugin writes a pairing marker to `wp_options.dailyos_pairing_marker` and reports "Paired".
4. Verify on the Tauri side — the audit log records a `pairing.code_consumed` event.

## First render

1. In Studio's WP admin, create a new **DailyOS Account** custom post (e.g., "Acme Corporation").
2. The default editor inserts a `dailyos/account-overview` block.
3. View the post on the frontend (`/accounts/acme-corporation/`). The block renders the account overview with trust band, provenance tags, and entity-intelligence rows.

If the block renders empty (`is-empty` class with "No account context to show here"), check:

- Is the DailyOS app running? → `dailyos doctor pairing` reports `runtime_endpoint=absent` if not.
- Did pairing complete? → check `wp_options.dailyos_pairing_status = "paired"`.

## Troubleshooting

`dailyos doctor` reports state without leaking secrets. Subcommands:

```
dailyos doctor pairing      # sentinel + audit-log writeability
dailyos doctor watermarks   # claim/composition watermark integrity
dailyos doctor all          # both, with combined exit code
```

Common failures + remediations:

| Symptom | Cause | Fix |
|---|---|---|
| `runtime_endpoint=absent` | DailyOS app not running | Launch DailyOS |
| Block renders silently empty | Pairing marker stale or plugin can't reach runtime | Restart DailyOS; if marker stale, re-pair from Settings |
| `runtime_endpoint=present-but-unparseable` | Sentinel file corrupted | `rm ~/.dailyos/runtime-endpoint.json` and restart DailyOS |
| `audit_log=not-writable` | `~/.dailyos/` permissions or disk full | `ls -la ~/.dailyos/`; parent dir should be `0700` owned by you |
| Pairing code rejected as "expired" | Code TTL elapsed (default 5 min) | Generate a new code in DailyOS Settings |
| Pairing code rejected as "consumed" | Code already paired against another marker | Generate a new code; DailyOS marks consumed codes single-use |

For runtime-side debugging without leaking secrets:

```sh
# Sentinel inspection (safe — port + version only, no auth material)
cat ~/.dailyos/runtime-endpoint.json

# Audit log tail (the runtime emits sensitive material via hashes, not raw values)
tail -f ~/.dailyos/audit.log | jq .
```

The HMAC session key is stored in macOS keychain. **Never copy keychain contents into bug reports.** If a support case requires keychain state, the dev team requests a redacted `dailyos doctor all` output and a fresh audit log line range.

## What `dailyos doctor` does NOT check

- **WordPress-side pairing marker.** That lives in `wp_options` and is only reachable through WP-CLI or the WP admin. Pair `dailyos doctor` output with WP-side:
  ```sh
  wp option get dailyos_pairing_marker
  wp option get dailyos_pairing_status
  ```
- **MCP allowlist behavior.** Default WP MCP server does NOT expose `dailyos/*`; custom DailyOS MCP allowlist is verified by the W6 release-gate fixtures (DOS-575) not by `dailyos doctor`.
- **HMAC session-key integrity.** Keychain entries are owner-only; the doctor doesn't probe them to avoid surfacing presence/absence as a side channel.

## After install

You're paired and rendering. Next surfaces to explore:
- Create more `dailyos_account` posts and watch entity-intelligence accumulate.
- Try the W2 primitive blocks (entity-chip, trust-band-badge, score-band) in the editor.
- Use the v1.4.3 magazine theme for the full editorial layout.

For incident response (audit forensic trace), see `.docs/runbooks/audit-forensic-trace.md` (lands with DOS-576).
