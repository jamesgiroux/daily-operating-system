---
title: "Parallel test flake from shared process-singleton state — mark tests #[serial] when touching FenceCycle-class singletons"
problem_type: test_failure
track: bug
module: src-tauri/src/intelligence/write_fence/tests.rs and any test touching process-singleton registries
tags: [test-flake, parallel-tests, singleton, fencecycle, serial, w3]
date: 2026-05-18
---

## Problem

`intelligence::write_fence::tests::dos311_substrate_migration_sequence_end_to_end` passes in isolation but flakes intermittently in the full `cargo test --lib` run. The failure mode looks like a regression introduced by the most recent commit, masking diagnosis during W3-C/H Phase 3 and Phase 5 work (~5 min misdiagnosis cost each time).

## Symptoms

- Test passes alone (`cargo test --lib dos311_substrate_migration_sequence_end_to_end`).
- Test fails when run as part of the full suite, with unrelated assertion errors that change run-to-run.
- Failure correlates with multiple tests touching `FenceCycle` running in parallel threads.

## What Didn't Work

- Treating it as a regression and reverting recent changes (the symptom returned).
- Re-running the suite (intermittent — sometimes green, sometimes red).

## Solution

Mark the test (and any other test touching the same singleton) with `#[serial]` from the `serial_test` crate so they don't run concurrently:

```rust
#[test]
#[serial]
fn dos311_substrate_migration_sequence_end_to_end() {
    // ...
}
```

Or fix the singleton itself to be thread-isolated (per-test instance, scoped Drop, etc.) — the right answer depends on whether the singleton is genuinely process-global by design or just lazy.

## Why This Works

`FenceCycle` is a process-singleton registry shared across the test binary. Parallel tests stomp on each other's registry state, producing nondeterministic failures. `#[serial]` forces serial execution for the annotated tests; the rest of the suite still runs in parallel.

## Prevention

When authoring a test that touches:
- Process-singleton registries (`FenceCycle`, ability registry, signal bus globals)
- Static `Mutex`/`RwLock` state
- Global telemetry/observability state that tests assert on
- `std::env` (`std::env::set_var` is process-wide)
- `Drop` order-dependent test fixtures

Either:
1. Mark the test `#[serial]`.
2. Isolate the singleton per-test (preferred but often more work).

When debugging a "regression introduced by recent commit" that disappears under `--test-threads=1`, suspect singleton state, not the recent commit.

## Related

- W3 retro `.docs/plans/wave-W3/retro.md` (W3-C/H section, "Sub-issues" #3)
- `serial_test` crate documentation
