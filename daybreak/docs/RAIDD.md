# RAIDD Log

> Risks, Assumptions, Issues, Dependencies, Decisions

---

## Risks

| ID | Risk | Impact | Likelihood | Mitigation | Status |
|----|------|--------|------------|------------|--------|
| R1 | Claude Code PTY issues on different machines | High | Medium | Retry logic, test matrix | Open |
| R2 | Google API token expiry mid-workflow | Medium | High | Detect early, prompt re-auth | Open |
| R3 | File watcher unreliability on macOS | Medium | Low | Periodic polling backup | Open |
| R4 | Scheduler drift after sleep/wake | Medium | Medium | Re-sync on wake events | Open |

---

## Assumptions

| ID | Assumption | Validated | Notes |
|----|------------|-----------|-------|
| A1 | Users have Claude Code CLI installed and authenticated | No | Need onboarding check |
| A2 | Workspace follows PARA structure | No | Should gracefully handle variations |
| A3 | `_today/` files use expected markdown format | Partial | Parser handles basic cases |

---

## Issues

| ID | Issue | Priority | Owner | Status |
|----|-------|----------|-------|--------|
| I1 | Config directory named `.daybreak` should be `.dailyos` for brand consistency | Low | — | Open |

---

## Dependencies

| ID | Dependency | Type | Status | Notes |
|----|------------|------|--------|-------|
| D1 | Claude Code CLI | Runtime | Available | Requires user subscription |
| D2 | Tauri 2.x | Build | Stable | Using latest stable |
| D3 | Google Calendar API | Runtime | Optional | For calendar features (Phase 3) |

---

## Decisions

| ID | Decision | Date | Rationale | Alternatives Considered |
|----|----------|------|-----------|------------------------|
| DEC1 | Use Tauri over Electron | 2024-01 | Smaller binary, Rust backend, native performance | Electron (too heavy), native Swift (platform lock-in) |
| DEC2 | Frontend-first implementation | 2024-01 | Reveals data shapes before backend investment | Backend-first (speculative) |
| DEC3 | Config in JSON file, no UI for MVP | 2024-02 | Reduces scope, power users can edit | Settings UI (adds complexity) |

---

*Last updated: 2026-02-04*
