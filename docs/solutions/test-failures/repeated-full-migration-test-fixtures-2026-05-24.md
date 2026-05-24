---
title: "Repeated full-migration test fixtures make cargo test --lib look hung"
problem_type: test_failure
track: bug
module: src-tauri/src/migrations.rs, src-tauri/src/db/core.rs test utilities
tags: [cargo-test, test-performance, migrations, sqlite, fixtures, pre-push]
date: 2026-05-24
---

## Problem

Rust unit tests that need a current SQLite schema were opening a fresh in-memory or temporary database and replaying every registered migration inside each test fixture helper. As the migration list grew, this made `cargo test --lib` spend minutes rebuilding identical empty schemas.

## Symptoms

- `cargo test --lib` appears stuck while test binaries consume several CPU cores.
- Stack samples show tests inside `migrations::run_migrations` and SQLite schema parsing.
- Module filters with many DB fixtures take tens of seconds or time out even though individual assertions are simple.
- Pre-commit and pre-push feel disproportionately expensive for small Rust diffs.

## What Didn't Work

- Retrying the hook. That repeats the same migration replay work.
- Treating one sampled test as the only culprit. The sampled test may pass in isolation; the class issue is the repeated fixture setup.
- Running multiple full suites in parallel worktrees. That amplifies CPU and target-dir contention.

## Solution

Use a test-only migrated in-memory template and clone it for isolated DB fixtures:

```rust
#[cfg(test)]
pub(crate) fn migrated_in_memory_for_tests() -> Connection {
    static TEMPLATE: std::sync::OnceLock<std::sync::Mutex<Connection>> =
        std::sync::OnceLock::new();

    let template = TEMPLATE.get_or_init(|| {
        let conn = Connection::open_in_memory().expect("open migrated test template");
        run_migrations(&conn).expect("migrate test template");
        std::sync::Mutex::new(conn)
    });

    let template = template
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut conn = Connection::open_in_memory().expect("open test db clone");
    {
        let backup =
            rusqlite::backup::Backup::new(&template, &mut conn).expect("start test db clone");
        backup.step(-1).expect("clone migrated test db");
    }
    conn
}
```

Then have schema-current fixture helpers call `crate::migrations::migrated_in_memory_for_tests()` instead of replaying `run_migrations(&conn)` themselves.

## Why This Works

The template still runs the real migration stack once per test binary process, so tests exercise the current schema. SQLite backup creates a fresh isolated database per test, so mutations do not leak across tests. The expensive migration replay is no longer multiplied by every fixture.

Observed during the fix:

- `db::core` filter: timed out after 120s before finishing; after template cloning, 185 tests finished in 1.80s.
- `db::intelligence_feedback` filter: 27.65s to 0.65s.
- `services::surface_pairing` filter: 30.22s to 3.23s.
- `services::trust_extraction` filter: 10.23s to 0.54s.
- `services::claims_backfill` filter: 15.63s to 3.17s.

## Prevention

When adding unit-test helpers that need the current schema:

1. Use `migrated_in_memory_for_tests()` for an empty current-schema DB.
2. Replay `run_migrations` directly only when the test is about migration behavior itself.
3. Avoid file-backed DBs unless the test asserts path, WAL, backup, permissions, or reopen behavior.
4. Before adding another full-schema fixture helper, time the module filter and compare against the template helper.
