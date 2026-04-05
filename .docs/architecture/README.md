# DailyOS Architecture Reference

**Last updated:** 2026-03-21
**Scope:** Rust backend (`src-tauri/src/`) + frontend data flow (`src/`)

This directory contains human-readable architecture docs for DailyOS. Four docs are **auto-generated** by scripts in `.docs/generators/` and kept up to date by a PostToolUse hook — they regenerate automatically when relevant source files change.

---

## Auto-Generated Docs

These stay current automatically. Run `.docs/generators/gen-*.sh` manually to force a refresh.

| Document | Generator | Triggers On |
|----------|-----------|-------------|
| [COMMAND-REFERENCE.md](COMMAND-REFERENCE.md) | `gen-command-reference.sh` | Edits to `commands.rs` or `commands/*.rs` |
| [MODULE-MAP.md](MODULE-MAP.md) | `gen-module-map.sh` | Edits to any `.rs` file in `src-tauri/src/` |
| [FRONTEND-HOOKS.md](FRONTEND-HOOKS.md) | `gen-frontend-hooks.sh` | Edits to `src/hooks/*.ts` or `.tsx` |
| [DATA-MODEL.md](DATA-MODEL.md) | `gen-data-model.sh` | Edits to `migrations/*.sql` |

## Manually Maintained Docs

These require human/AI judgment to update. Update when the noted triggers occur.

| Document | What It Covers | Update When |
|----------|---------------|-------------|
| [DATA-FLOWS.md](DATA-FLOWS.md) | 6 end-to-end data flows with Mermaid diagrams | Changing cross-cutting data flows |
| [LIFECYCLES.md](LIFECYCLES.md) | State machines for 7 domain objects | Changing state transitions |
| [PIPELINES.md](PIPELINES.md) | 7 async pipelines traced end-to-end | Modifying async pipeline logic |
| [SELF-HEALING.md](SELF-HEALING.md) | Automatic data repair, quality maintenance | Changing hygiene rules or permissions |

## Frontend Audit Docs (needs refresh)

Generated 2026-03-02. May have drifted — verify before relying on specifics.

| Document | What It Covers |
|----------|---------------|
| [FRONTEND-COMPONENTS.md](FRONTEND-COMPONENTS.md) | Component registry, ghost components, business logic violations |
| [FRONTEND-STYLES.md](FRONTEND-STYLES.md) | CSS audit against design tokens, compliance score |
| [FRONTEND-TYPES.md](FRONTEND-TYPES.md) | TS↔Rust type alignment matrix |

## Related Docs (outside this directory)

| Document | Location |
|----------|----------|
| [SIGNAL-SCORING-REFERENCE.md](../design/SIGNAL-SCORING-REFERENCE.md) | Signal scoring math (weights, decay, fusion) |
| [INTELLIGENCE-CONSISTENCY-REFERENCE.md](../design/INTELLIGENCE-CONSISTENCY-REFERENCE.md) | Contradiction rules, repair policy, triage SQL |
| [SERVICE-CONTRACTS.md](../design/SERVICE-CONTRACTS.md) | Service layer contracts |
| [ARCHITECTURE-MAP.md](../design/ARCHITECTURE-MAP.md) | Module boundaries, data flow overview |

## How to Use

- **Before modifying a module:** Read MODULE-MAP.md for dependencies and boundaries
- **Before adding a Tauri command:** Read COMMAND-REFERENCE.md for the pattern in your domain
- **Before touching signals:** Read PIPELINES.md + SIGNAL-SCORING-REFERENCE.md
- **Before adding/modifying a DB table:** Read DATA-MODEL.md for schema and duplication risks
- **Before changing a lifecycle:** Read LIFECYCLES.md for the current state machine
- **Before frontend work:** Read FRONTEND-HOOKS.md for existing hook patterns, FRONTEND-TYPES.md for type alignment
