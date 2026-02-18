# ADR-0072: Auto-Updater Architecture

**Status:** Accepted
**Date:** 2026-02-13
**Deciders:** James, Claude

## Context

Every hotfix required manually building a DMG, uploading to GitHub Releases, and telling users to download it and run `xattr -cr` to bypass Gatekeeper. This was tolerable for 4 alpha testers but unworkable for beta (20-50 users). Users had no way to know an update existed, and the unsigned binary triggered macOS security warnings on every install.

Three problems needed solving simultaneously:
1. **Update discovery** — the app needs to know when a new version exists
2. **Code signing** — macOS requires Developer ID Application certificates for distribution outside the App Store, and Gatekeeper requires notarization
3. **Update delivery** — download, verify, install, and restart without user friction

## Decision

### Update Signing (Ed25519)

Tauri's updater uses **Ed25519 signatures** (via `tauri-plugin-updater`) to verify update integrity. This is separate from Apple code signing — it's Tauri's own mechanism to ensure the update came from us.

- Signing keypair generated via `pnpm tauri signer generate`
- Private key stored as GitHub Secret (`TAURI_SIGNING_PRIVATE_KEY`)
- Public key embedded in `tauri.conf.json` (`plugins.updater.pubkey`)
- CI signs `.tar.gz` update bundles, producing `.tar.gz.sig` files

### Update Discovery

The app checks a `latest.json` manifest hosted as a GitHub Release asset:

```json
{
  "version": "0.7.3",
  "notes": "DailyOS v0.7.3",
  "pub_date": "2026-02-14T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<Ed25519 signature>",
      "url": "https://github.com/.../releases/download/v0.7.3/DailyOS.app.tar.gz"
    }
  }
}
```

Endpoint configured in `tauri.conf.json`:
```
https://github.com/jamesgiroux/daily-operating-system/releases/latest/download/latest.json
```

The `/latest/download/` path always resolves to the most recent release, so no URL updates are needed between versions.

### Apple Code Signing + Notarization

- **Certificate type:** Developer ID Application (not Apple Development — that's for dev-only)
- **CI integration:** .p12 certificate base64-encoded as GitHub Secret, imported into a temporary CI keychain during builds
- **Notarization:** Handled automatically by Tauri's build process when `APPLE_ID`, `APPLE_PASSWORD` (app-specific), and `APPLE_TEAM_ID` environment variables are set
- **Result:** Signed + notarized DMG that passes Gatekeeper without `xattr -cr`

### Frontend Update UI

`UpdateCard` component on SettingsPage (first card, before Google integration). Uses `@tauri-apps/plugin-updater` JavaScript API directly — no Rust wrapper commands needed.

States: idle → checking → available (version + release notes) → installing → restarted.

The JS plugin API was chosen over custom Rust commands because:
- Tauri's plugin already handles the full lifecycle (check, download, install)
- Adding Rust commands would duplicate what the plugin provides
- The JS API gives direct access to download progress events

### CI Workflow

`.github/workflows/release.yml` triggers on version tags (`v*`):
1. Version consistency check (git tag vs `tauri.conf.json`)
2. Certificate import into temporary keychain
3. `pnpm tauri build` with signing + notarization env vars
4. Collect artifacts: DMG (new installs) + `.tar.gz` + `.tar.gz.sig` (updates)
5. Generate `latest.json` from sig file content
6. Upload all artifacts to GitHub Release
7. Keychain cleanup

### macOS Configuration

- `minimumSystemVersion: "13.0"` (Ventura) — matches Tauri v2 requirements
- `createUpdaterArtifacts: true` — tells Tauri to produce the `.tar.gz` bundle alongside the DMG
- `bundle.macOS.signingIdentity: "Developer ID Application"` — Tauri finds the matching cert in keychain

## Consequences

- Users receive updates automatically — no manual DMG downloads for hotfixes
- Code signing eliminates Gatekeeper warnings and `xattr -cr` workaround
- Ed25519 update signing (separate from Apple signing) ensures update integrity
- GitHub Releases serves as both the update server and the distribution channel — no additional infrastructure needed
- `latest.json` must be regenerated and uploaded with every release — the CI workflow handles this automatically
- Apple Developer Program ($99/year) is now a recurring cost
- Certificate rotation (~5 year expiry) must be planned for
- Tauri updater only supports replacing the full app bundle — no delta updates. Acceptable at current binary size (~30MB compressed)
- The updater cannot downgrade — if a bad version ships, the fix must be a newer version number
