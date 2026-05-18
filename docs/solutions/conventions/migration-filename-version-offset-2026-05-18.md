---
title: Migration filename = registered_version - 1 (off-by-one convention)
problem_type: convention
track: knowledge
module: src-tauri/src/migrations.rs, src-tauri/src/migrations/*.sql
tags: [migrations, naming-convention, schema-version, w3]
date: 2026-05-18
---

## Context

DailyOS migrations follow an off-by-one convention: the migration **filename** is `(registered_version - 1)`. A migration file named `134_<slug>.sql` is registered as version `135` in `migrations.rs`.

This bit several test assertions during W3-C/H Phase 3 + Phase 5 work where hard-coded `expected_schema_version` values assumed the filename and registered version matched.

## Guidance

When adding a new migration:
1. Determine the next registered version (read `migrations.rs` for the current max).
2. Name the file `<registered_version - 1>_<slug>.sql`.
3. Register in `migrations.rs` with the registered version, not the filename number.
4. If a test assertion needs to bind to the version, use the registered version (from `migrations.rs`), not the filename number.

When reviewing a migration PR:
- Verify the filename number is `registered_version - 1`.
- Verify test assertions reference the registered version.
- Verify migration ordering by registered version, not filename sort.

## Why This Matters

Tests that compare `schema_version == filename_int` pass in isolation but break when the actual registered version is one higher. The convention is consistent throughout the codebase — but undocumented inline. Future agents (and reviewers) will keep hitting this until either the convention is documented or refactored to match (filename == registered version).

A one-line comment in `migrations.rs` near the version registration would prevent re-discovery:

```rust
// Convention: migration file `<N>_<slug>.sql` is registered as version N+1.
// Test assertions must use the registered version, not the filename number.
```

## When to Apply

- Authoring any new migration.
- Reviewing a migration PR.
- Debugging `schema_version` test failures or `expected_version` mismatches.
- Reading migration ordering output (sort by registered version, not filename).

## Examples

W3 Phase 3 (DOS-294 schema reconciliation): three test assertions failed initially because they bound to filename-int. Fixed by referencing the registered version. The retro flagged this as worth a one-line comment in `migrations.rs`.

## Related

- W3 retro `.docs/plans/wave-W3/retro.md` (W3-C/H section, "Sub-issues" #2)
