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
| I2 | Compact `meetings.md` format for dashboard dropdowns | Low | — | Explore |

### I2 Notes
The archive from 2026-02-04 contains a compact `meetings.md` format with structured prep summaries:
```markdown
## 1:00 PM - Meeting Title
type: customer
account: Account Name
end: 1:45 PM

### Prep
**Context**: Brief context with key metrics (ARR, renewal date, etc.)
**Wins**: Bullet list of recent wins
**Risks**: Bullet list of current risks
**Actions**: Bullet list of discussion items
```
This format could be useful for:
- Dashboard meeting card dropdowns (quick glance without full prep)
- Role-specific templates (CSM/Sales may need this more than others)
- Generating consolidated daily meeting summary

Consider adding as a Claude Code template output for `/today` command post-MVP.

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
| DEC4 | Hybrid JSON + Markdown architecture | 2026-02 | JSON for machine consumption, markdown for humans. Eliminates fragile regex parsing. | Markdown-only (fragile), JSON-only (not human-readable) |
| DEC5 | Archives remain markdown-only | 2026-02 | Historical data is for human reference. JSON generation happens at runtime for active `_today/` only. | Full JSON archives (unnecessary complexity) |
| DEC6 | Phase 3 generates JSON (not Claude) | 2026-02 | Maintains determinism boundary. Claude outputs markdown (its strength), Python converts to validated JSON. | Claude outputs JSON directly (less reliable) |

---

*Last updated: 2026-02-05*
