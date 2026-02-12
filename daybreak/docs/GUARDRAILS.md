# Rust Performance + Security Guardrails

Non-optional guardrails for runtime responsiveness and safety in DailyOS.

Related backlog: I149, I150, I151, I152, I153, I155, I177, I197.

## Core Guardrails

1. Never hold DB lock across expensive work
- Do not hold `state.db.lock()` during AI calls, network calls, filesystem scans, or long loops.
- Use split-lock phases: gather -> compute -> persist.
- Prefer `AppState` DB helpers (`with_db_try_read`, `with_db_read`, `with_db_write`) over direct lock handling in commands.

2. Hot read commands must not block UI on DB contention
- For focus/render paths, prefer `try_lock()` and partial/degraded results over blocking.
- Log when data is degraded because DB is busy.

3. Focus-triggered refreshes must be throttled and deduped
- Add minimum refresh interval for window focus handlers.
- Deduplicate in-flight requests to prevent request storms.

4. Shell-outs in user-visible paths need bounded execution
- Add explicit timeouts to CLI checks.
- Kill timed-out child processes.
- Cache command health checks with TTL when feasible.

5. Enforce command latency budgets
- p95 targets:
  - hot read/status commands: <100ms
  - dashboard load: <300ms
- Emit warning logs when budgets are exceeded.
- Record in-memory rollups (`p50`/`p95`/max + degraded counters) for diagnostics.

6. IPC boundary validation is mandatory
- Validate command args for path traversal, enums, IDs, and size limits.
- Return explicit errors; do not panic.

7. Dependency and credential hygiene
- Run dependency audits regularly.
- Store credentials in secure storage (keychain), never plaintext.

## PR Checklist

- [ ] No long-lived global DB lock in changed code.
- [ ] Focus-resume path has throttle + in-flight dedupe.
- [ ] Any shell-out has timeout and, if hot-path, cache.
- [ ] Command latency impact measured or logged.
- [ ] New IPC inputs validated at boundary.
