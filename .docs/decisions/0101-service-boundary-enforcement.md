# ADR-0101: Service Boundary Enforcement

**Status:** Accepted  
**Date:** 2026-04-15  
**Context:** Codebase maintenance audit (DOS-174, DOS-175, DOS-176, DOS-177, DOS-178)

## Problem

A line-by-line adversarial audit of the services layer revealed systemic violations of the service boundary contract:

- **59 mutation functions** write to DB without emitting a signal (DOS-174)
- **34 multi-write operations** lack transaction boundaries (DOS-175)
- **142 error-swallowing instances** hide data loss from callers (DOS-176)
- **Business logic in 6+ non-service layers** bypasses signals and transactions (DOS-177)
- **2 read functions** secretly perform mutations (DOS-178)

The root cause: `ActionDb::open()` is accessible from anywhere in the codebase. Any file can open a database connection, write directly, and skip the service layer entirely. The service boundary is advisory, not enforced. There are 60+ independent `ActionDb::open()` calls outside of `db/` and `services/`.

## Decision

### The Service Boundary Contract

**Rule 1 — Services own all domain mutations.**  
Only functions in `services/*.rs` may call `db.insert`, `db.update`, `db.upsert`, or `db.delete` for domain entities (accounts, people, meetings, actions, emails, objectives, milestones, signals). Other layers (commands, processors, workflow, pollers) call service functions.

**Rule 2 — Every service mutation emits a signal.**  
If a service function writes to the DB, it must call `emit_signal()` or `emit_signal_and_propagate()`. The only exceptions are: internal bookkeeping (processing logs, app state KV, chat sessions) and operations where the signal is emitted by the caller as part of a larger transaction.

**Rule 3 — Related writes are transactional.**  
If a service function performs 2+ DB writes that must succeed together, they must be wrapped in `db.with_transaction()`. No multi-write operation should leave inconsistent state on crash.

**Rule 4 — Errors propagate, not swallow.**  
Service functions must return `Result`. `let _ =` is only acceptable for truly best-effort operations (file exports, cosmetic writes). Any path where user data could be lost must propagate the error to the caller.

**Rule 5 — Reads don't mutate.**  
Functions named `get_*`, `list_*`, `load_*`, or `build_*` must not write to the DB or filesystem. Side-effects belong in explicit mutation functions called separately.

### Enforcement Mechanism — Three Phases

**Phase 1 (immediate): Hook-based prevention of new violations**

- Pre-commit hook: block new `ActionDb::open()` calls outside `db/` and `services/`
- PostToolUse hook: flag service functions that do DB writes without `emit_signal`

**Phase 2 (v1.3.x): ServiceContext refactor**

Introduce a `ServiceContext` struct that encapsulates DB access + signal bus + intel queue. Services receive `ServiceContext` instead of opening their own connections. Commands and other layers cannot construct a `ServiceContext` — they call service functions.

```rust
pub struct ServiceContext<'a> {
    pub db: &'a ActionDb,
    pub signals: &'a PropagationEngine,
    pub intel_queue: &'a IntelligenceQueue,
}
```

**Phase 3 (v1.4.x): Compile-time enforcement**

- `ActionDb::open()` becomes `pub(in crate::db)` — only the DB module can construct connections
- Services receive DB handles through `ServiceContext`, not by opening their own
- Any caller outside `services/` that needs DB access must go through a service function
- Violations become compile errors, not code review findings

## Consequences

**Positive:**
- New violations are blocked at commit time (Phase 1)
- Existing violations can be systematically migrated (Phase 2)
- Once Phase 3 is complete, the service boundary is enforced by the type system
- Signal coverage becomes automatic — services emit, callers don't need to think about it
- Transaction boundaries are visible at the service function level

**Negative:**
- Phase 2-3 is a significant refactor touching 60+ call sites
- Some background processors (transcript, enrichment) will need service function wrappers for operations that currently write directly
- Short-term increase in service file sizes as logic migrates in

**Migration strategy:**
- Don't move everything at once. Start with the highest-traffic command→service paths
- Each Tuesday release train can include 5-10 migration items alongside feature work
- Track progress via the Codebase Maintenance project milestones in Linear

## References

- DOS-174: 59 signalless mutations
- DOS-175: 34 non-atomic multi-writes
- DOS-176: 142 swallowed errors
- DOS-177: Business logic scattered across 6+ layers
- DOS-178: Read functions that mutate
- DOS-166: Command→service boundary violations
