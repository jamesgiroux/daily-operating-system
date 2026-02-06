# ADR-0001: Use Tauri over Electron

**Date:** 2024-01
**Status:** Accepted

## Context

DailyOS needed a native desktop app framework. The app requires system tray, file system access, background scheduling, and a small footprint. The backend needs to spawn subprocesses (Claude Code via PTY).

## Decision

Use Tauri v2 with a Rust backend and React/TypeScript frontend.

## Consequences

- Smaller binary (~10MB vs ~150MB Electron)
- Rust backend enables direct file system ops, SQLite, PTY management without Node.js overhead
- Harder to find developers familiar with Tauri + Rust compared to Electron
- No native Swift — platform-specific features require Rust bindings or Tauri plugins
- Alternatives rejected: Electron (too heavy), native Swift (platform lock-in)
