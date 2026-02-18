# ADR-0025: App-native governance, not ported CLI tools

**Date:** 2026-02
**Status:** Accepted

## Context

The CLI era solved workspace governance (classification, routing, validation, account registry) with Python scripts in `_tools/`. The Tauri app is a different runtime targeting a broader audience where Python is not guaranteed and subprocess calls are the wrong primitive.

## Decision

Governance logic lives in compiled Rust inside the Tauri app. The app reads `~/.dailyos/` registries and workspace structure directly. No Python subprocess calls for core operations. The app is the authority on first install; CLI and MCP are secondary interfaces that share the same registries.

**What this means:**
- Classification and routing become Rust functions (same rules as Python originals, but compiled)
- Account/project registry read from `accounts-mapping.json` natively
- Validation runs automatically via file watcher, not on-demand scripts
- Interface parity: App, CLI, and Claude Code (via MCP) all read the same state

**Key principle:** The app doesn't call tools — the app *is* the tool.

## Consequences

- No Python runtime dependency for core app operations
- Governance is continuous (file watcher) rather than invoked (scripts)
- Existing `_tools/` Python scripts inform the design but are not ported 1:1
- CLI becomes optional — a user who never opens a terminal gets full governance
- The symlink/eject pattern from the CLI era does not map to native app distribution
