# Safe MCP Binary Setup

DailyOS Claude Desktop integration must launch the guarded MCP runtime, not a raw build artifact.

## Build

Use the project script:

```bash
pnpm build:mcp
```

This builds both sidecars with `--features mcp`:

- `src-tauri/binaries/dailyos-mcp-$TARGET_TRIPLE`
- `src-tauri/binaries/dailyos-mcp-launcher-$TARGET_TRIPLE`
- `src-tauri/binaries/dailyos-mcp-bundle-$TARGET_TRIPLE.provenance.json`

If running Cargo directly for diagnostics, include the MCP feature:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release --features mcp --bin dailyos-mcp --bin dailyos-mcp-launcher-bin
```

Do not configure Claude Desktop to launch anything under `target/debug`, `target/release`, or `.cargo/bin`.

## Configure Claude Desktop

Configure through DailyOS Settings or the integration service, not by hand-editing Claude's config to point at a raw binary. The service:

- verifies bundled or dev build provenance;
- rejects stub provenance and zero-byte sidecars;
- copies the verified launcher to `~/.dailyos/mcp/dailyos-mcp-launcher`;
- writes `~/.dailyos/mcp/dailyos-mcp-manifest.json`;
- sets Claude's `mcpServers.dailyos.command` to the app-managed launcher with `--manifest`.

The launcher revalidates the manifest, bundled provenance, file paths, executable bits, SHA-256 hashes, and MCP runtime self-check before it starts `dailyos-mcp`.

## Verify

After `pnpm build:mcp`, the self-check must succeed without opening a DB:

```bash
TARGET_TRIPLE=$(rustc -vV | awk '/^host:/ { print $2 }')
src-tauri/binaries/dailyos-mcp-$TARGET_TRIPLE --self-check-json
```

Expected shape:

- `guardEpoch` is `dailyos-mcp-runtime-guard:v1`
- `defaultDbMode` is `replica`
- `runtimeContainsDbModeGuard` is `true`
- `dbOpened` is `false`

After configuring Claude Desktop through DailyOS, the managed launcher check should succeed:

```bash
~/.dailyos/mcp/dailyos-mcp-launcher --manifest ~/.dailyos/mcp/dailyos-mcp-manifest.json --check
```

If status reports an unsafe legacy config, rerun the DailyOS Claude Desktop configuration action and restart Claude Desktop.

## Packaging Checks

The Tauri app bundle must include:

- `DailyOS.app/Contents/MacOS/dailyos-mcp`
- `DailyOS.app/Contents/MacOS/dailyos-mcp-launcher`
- `DailyOS.app/Contents/Resources/binaries/dailyos-mcp-bundle-$TARGET_TRIPLE.provenance.json`

Release validation must reject `stub: true` provenance. Stubs exist only to satisfy local dependency validation before real sidecars are built.

When pasting proof output into Linear, PRs, or docs, redact local home-directory paths as `~`.
