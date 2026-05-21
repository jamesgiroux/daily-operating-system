# L3 Codex Challenge — v1.4.4 W1 Wave (cycle 2)

**Reviewer:** `/codex challenge` via `codex exec` direct (focused 5-fix verification)
**Date:** 2026-05-21
**Patches verified:** aa48ffce (F1+F3), 3e58bc13 (F2+F4), 00f38b3b (F5)

## Verdict: ALL 5 FIXES PATCHED CORRECTLY

### F1 — Signal coalesce — PATCHED
- `MeetingPrepStatusChanged` included in `uses_emit_path_coalescing()` at `policy_registry.rs:380`/`395`
- Policy `COALESCE_ENTITY_500MS` at `policy_registry.rs:873`
- Bus applies predicate+channel coalescing with `CoalescingKey(type, entity_id)` at `bus.rs:485`

### F2 — Touchpoint audience filter — PATCHED
- MCP maps to non-user `RenderActor` at `producer.rs:268`
- Touchpoint projection threads actor through `compose_touchpoints`/bundle/item at `producer.rs:468`, `507`, `563`
- Non-user titles redact at `producer.rs:576`
- Serialized absence asserted at `producer.rs:1523`

### F3 — v244 transactional migration — PATCHED
- v244 registered after v243 at `migrations.rs:946`/`958`
- `DROP VIEW` + `CREATE VIEW` wrapped by `BEGIN IMMEDIATE`/`COMMIT` at `244_meeting_prep_status_view_transactional.sql:26` and `:69`

### F4 — Envelope cache principal binding — PATCHED
- Cache key `(envelope_render_id, actor_principal_id, surface)` at `envelope_cache.rs:50`
- Lookup builds same key at `:213`
- Miss returns `EnvelopeRequired` at `:130`
- Command requires render id/principal at `claim_feedback.rs:77` and `:91`
- Uses cached envelope ids at `:120` — no longer synthesizes from target id

### F5 — WP block real consumer skeletons — PATCHED
- 3-arg `invoke_ability` calls confirmed in all 4 blocks:
  - account-detail `render-functions.php:64`, `:139`, `:148`
  - daily-briefing `render-functions.php:56`, `:118`
  - person-detail `render-functions.php:61`
  - project-detail `render-functions.php:61`
- Signature checker enforces producer literal + exactly 3 args at `check_w1_consumer_skeleton.sh:138`

## Targeted checks passed

- `check_w1_consumer_skeleton.sh` ✅
- F1 bus tests ✅
- F2 touchpoint serialization tests ✅
- F4 envelope cache tests ✅

## L3 Verdict: UNANIMOUS

- Architect-reviewer cycle-1: APPROVE ✅
- Codex challenge cycle-2: 5/5 PATCHED CORRECTLY ✅
- Suite S (Security): GREEN — 4 CI lint scripts all exit 0; /cso cycle-2 APPROVE on cycle-2 substrate
- Suite P (Performance): N/A at W1 substrate stage
- Suite E (Edge cases): GREEN — cargo test 2628/0 on cycle-2 wave (2637 on cycle-2 + F1-F5 patches still running for verification)

L3 closes. Proceed to PR open + K-out + next wave L0.
