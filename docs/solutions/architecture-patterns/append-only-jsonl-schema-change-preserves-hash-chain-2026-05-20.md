---
title: "Adding an Optional<T> field with skip_serializing_if to an append-only JSONL audit record preserves hash chain continuity across the schema cutover — legacy rows serialize identically pre/post"
problem_type: architecture_pattern
track: knowledge
module: src-tauri/src/audit_log.rs (AuditRecord schema + AuditLogger::write_record)
tags: [audit-log, append-only, jsonl, hash-chain, schema-evolution, serde, skip-serializing, dos-741]
date: 2026-05-20
related_linear: DOS-741
related_memories: []
---

## Context

DOS-741 added a new `request_id: Option<String>` field to `AuditRecord` (serialized to a single line of append-only JSONL in `~/.dailyos/audit.log`). The audit log uses a hash chain: every row's `prev_hash` is SHA-256 of the previous line's full serialization. Tamper-evidence depends on chain continuity.

The L0 cycle-2 code reviewer flagged this as a HIGH finding (V12-2):

> "If hashing is over the full serialized record (typical for tamper-evident chains), every legacy row's hash recomputes differently the moment the schema changes — chain breaks."

This was the concern: did adding a new field to `AuditRecord` mean every existing audit row's hash was now wrong?

## The pattern that preserves continuity

The Rust struct + serde annotation:

```rust
pub struct AuditRecord {
    pub ts: String,
    pub v: u8,
    // ... existing fields ...
    /// End-to-end correlation identifier (DOS-741).
    /// SurfaceClient emissions carry the validated X-DailyOS-Request-Id header.
    /// User/Agent emissions carry server-generated UUIDv7 via new_request_id().
    /// Legacy append() emissions leave this None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}
```

Three properties combine to preserve hash chain continuity:

1. **`Option<String>` with `None` default** — existing rows that don't carry the field deserialize cleanly (deserialize as None) and serialize back identically (no key emitted because of #3).
2. **`skip_serializing_if = "Option::is_none"`** — when value is None, the key + value are omitted from JSONL output entirely. The serialized line shape is bytewise-identical to pre-DOS-741.
3. **Append-only JSONL is never re-serialized in place.** Once a line is written, it's never rewritten. New lines append; the file grows but doesn't mutate.

## Why this works

The hash chain is computed line-by-line at write time:

```rust
fn write_record(&mut self, ...) -> Result<(), AuditError> {
    let record = AuditRecord { /* ... */ prev_hash: self.last_hash.clone(), /* ... */ };
    let line = serde_json::to_string(&record)?;
    // line gets written to file; hash of this line becomes prev_hash for next record
    self.last_hash = Some(sha256(line));
    writeln!(file, "{}", line)?;
    Ok(())
}
```

Pre-cutover:
- Row N serialized with the legacy schema. `prev_hash` field references SHA-256 of row N-1's serialized bytes. Both rows on disk forever.

Post-cutover:
- Row N+1 is written with the new schema. If `request_id = None`, the JSON output omits the key (per `skip_serializing_if`), so the serialized bytes are bytewise-identical to what a legacy writer would have produced. Row N+1's `prev_hash` references SHA-256 of row N's bytes — same as before.
- If `request_id = Some("...")`, the JSON output includes the new key. Row N+1's bytes are different, but its `prev_hash` still references row N's unchanged bytes.

**Chain unbroken in both cases.** Verification tools that walk the chain see the same hashes pre and post.

## The non-pattern that would break it

If the new field were:

- **Required (non-Option)** — every existing row would fail to deserialize without a default.
- **Without `skip_serializing_if`** — Option::None would serialize as `"request_id": null`, changing the bytes of every newly-written row even when the value is absent. Legacy readers comparing prev_hash against legacy-formatted previous lines would mismatch.
- **In a non-append-only store** (e.g., a SQL table with re-serialization on read) — re-serialization would change hashes of pre-existing rows.

## How to apply when adding a new field

1. **Make it `Option<T>`** with default None.
2. **Annotate with `#[serde(default, skip_serializing_if = "Option::is_none")]`** so absent values produce zero serialization footprint.
3. **Document the schema-version cutover SHA** in the forensic runbook so operators can identify when the new field starts appearing.
4. **Never re-serialize existing rows in place.** The JSONL append-only invariant is what makes the chain stable.

## Cross-references

- Implementation: `src-tauri/src/audit_log.rs` (AuditRecord + write_record + tests `emit_surface_audit_threads_request_id_into_top_level_field` + `emit_surface_audit_without_request_id_omits_field_from_serialization`)
- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md`
