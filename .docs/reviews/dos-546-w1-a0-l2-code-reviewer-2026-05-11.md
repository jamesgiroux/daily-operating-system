# L2 (Diff) — code-reviewer verdict: APPROVE

Commit: `fada3fd9` — DOS-546 W1-A0 audit-log schema + `emit_surface_audit`.
Scope: `src-tauri/src/audit_log.rs` only, +424/-4.
AC: `.docs/plans/dos-546/v1.4.2-project/02-issues.md` lines 283–294.

## AC-bound assessment

1. **Compat.** All four new fields are `Option<_>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Legacy lines parse (test `legacy_records_without_actor_fields_still_parse` line 879 pins it). Non-attributed `append()` writes omit the keys entirely — log stays slim. ✅
2. **Helper signature.** `emit_surface_audit(logger, event_kind, actor, fields) -> Result<(), AuditError>` is idiomatic; `AuditFields::new(...).with_wp_user_id(n)` is a clean builder. `SurfaceClientMissingWpUserId` is precisely named and carries a clear `Display` message. ✅
3. **Drift-proof invariant.** `actor_instance` and `actor_scopes` are derived from the `Actor::SurfaceClient { instance, scopes }` variant inside `append_with_actor` (lines 269–278); the builder does NOT accept them from callers (doc-commented at lines 124–126). Caller cannot shadow them. ✅
4. **Stray `wp_user_id` drop.** Non-SurfaceClient branch (line 284) returns `(None, None, None)`; comment at 280–283 documents the deliberate drop per AC line 293. Drop is silent — spec calls for silent drop, so this matches. Test `emit_surface_audit_drops_wp_user_id_for_non_surface_client` (line 814) pins it. ✅
5. **Test quality.** Negative test (line 778) asserts both `Err(SurfaceClientMissingWpUserId)` AND `records.is_empty()` — invariant pin, not coverage padding. Round-trip test (line 831) reads raw bytes back through `serde_json::from_str` AND re-verifies the hash chain. Legacy parse, sorted-scope ordering, and stray-drop all genuine pins. ✅
6. **Concurrency.** `AuditLogger` lives in `Arc<Mutex<...>>` in `state.rs:405`; `append_with_actor` takes `&mut self` like `append`, sharing the same `write_record` primitive (line 303). No new lock surface, no new race.
7. **AC line 289 index.** Storage is JSONL not SQL; rendered as forensic-grep on the `wp_user_id` field, documented at lines 17–20 and deferred to W6-A — matches commit body and AC's W6-A scope split.

## Path-α (file to maintenance)

- `Actor::User` maps to kind `"user"` but the `Display` doc on `AuditRecord::actor_kind` (line 60) lists `"agent"`, `"user"`, `"admin"`, `"system"`, `"surface_client"`. Matches code. None to file.
- `AuditError` is not `From<std::io::Error>`; callers do string-format. Acceptable for this surface; no action.

## Verdict

**APPROVE.** Schema, helper, invariant, and tests all satisfy AC 287–294 bounded scope. Ready for PR.
