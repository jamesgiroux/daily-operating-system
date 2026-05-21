# v1.4.7 Wave W1-A — DOS-168 proof bundle

**Date:** 2026-05-20
**Branch:** `v1.4.7-w1-foundation`
**Lane spec:** [DOS-168](https://linear.app/a8c/issue/DOS-168) — MCP v2 ability/service gateway + actor policy + auth + audit
**L0 packet:** `.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md`

## Branch state

```
9 commits ahead of dev, ready for PR open (user-validation gated).

d1fa4fae fix(mcp-v2): L2 cycle-5 — failure-path Zeroize for keychain UTF-8 decode
71f96d06 fix(mcp-v2): L2 cycle-4 class-sweep — audit.rs Zeroize intermediates
e5b98771 fix(mcp-v2): L2 cycle-4 — 3 AC-bound fixes
5231df67 fix(mcp-v2): L2 cycle-3 — 5 follow-up fixes
9ed5c828 fix(mcp-v2): L2 cycle-2 — 6 AC-bound fixes
305fef72 test(mcp-v2): migration smoke test for v241-v244
228a17f3 feat(mcp-v2): W1-A substrate part 2 — gateway + auth bodies
62d97537 feat(mcp-v2): W1-A substrate part 1 — audit, actor_policy, migrations, signals, CI lint
45b3f53d feat(mcp-v2): W0 amendment substrate + L0 packet
```

Diff stat vs `dev`: 16 files changed, ~3100 insertions.

## L0 history (7 cycles)

| Cycle | Verdict trajectory | Resolution |
|---|---|---|
| 1 | challenge BLOCK / architect NEEDS-CHANGES / CSO NEEDS-CHANGES / devex BLOCK | 15 finding-classes across 6 areas; substrate (W0 amendment ADR-0102 §C/§D/§E) not yet landed |
| 2 | challenge BLOCK / architect NEEDS-CHANGES / CSO NEEDS-CHANGES / devex BLOCK | Convergent: ExposureForbidden sentinel-scope abuse; nonce ledger schema gaps; transactional audit + JSONL contradiction |
| 3 | challenge BLOCK / architect NEEDS-CHANGES / CSO NEEDS-CHANGES / devex BLOCK | Convergent: per-response nonce refresh needs W0 amendment; handler docs not landed (taxonomy.rs stub); pairing handshake response shape underspecified |
| 4 | architect NEEDS-CHANGES + 3 verifying | Convergent AC-2 narrative regression; nonce ledger lifecycle still incomplete |
| 5 | architect NEEDS-CHANGES + 3 verifying | AC-2 §1 #1 narrative still contradicting AC-2 body (cycle-3 of class) |
| 6 | architect NEEDS-CHANGES + 3 verifying | L6 escalation point: nonce ledger scope, audit-reject path, handler docs strategy |
| 7 | architect NEEDS-CHANGES + 3 unanimous closer | L6 verdict folded: nonce ledger (server-side) + drop reject_reason from audit + rustdoc on TaxonomyCatalog. Substrate landed before dispatch |

**L0 closure**: substrate-truth grep verified before cycle-7 dispatch; reviewers' "shape-only is a trap" finding from cycles 3-4 resolved by actually landing Deliverable 0 (contracts.rs ExposureForbidden + OpaqueNonce + envelope amendments + parity tests; taxonomy.rs trait + ~130 lines handler rustdoc; ADR-0102 cycle-7/8/9 amendments).

## L2 history (6 cycles)

| Cycle | Verdict counts | Resolution |
|---|---|---|
| 1 | 0 APPROVE / 1 BLOCK / 3 NEEDS-CHANGES | 6 AC-bound fixes (signal emission stderr-only; recovery_nonce leak; fail-closed split; rate-limit BEGIN IMMEDIATE; Gate 0 attribution; cursor caps) |
| 2 | 0 APPROVE / 1 BLOCK / 3 NEEDS-CHANGES | 5 AC-bound fixes (attribution regression; warning-as-rejection pollution; internal error detail leak; AC-5 COMMIT no rollback; silent truncation) |
| 3 | 1 APPROVE (CSO) / 0 BLOCK / 3 NEEDS-CHANGES | 3 AC-bound fixes (AC-5 fail-open `unwrap_or(0)`; AC-11 Zeroize intermediates; emit_warning correlation) |
| 4 | 3 APPROVE / 1 NEEDS-CHANGES (codex review class-sweep) | audit.rs AC-11 class-sweep (same pattern as auth.rs) |
| 5 | NEEDS-CHANGES (1 reviewer, codex review) | stdout.to_vec() clone drops unzeroized on UTF-8 failure |
| 6 | **APPROVE** (codex review final verification) | Borrow-not-clone fix applied to both files |

**L2 unanimous APPROVE achieved:**
- codex review: APPROVE (cycle 6)
- code-reviewer: APPROVE (cycle 4)
- CSO: APPROVE (cycle 3 + cycle 4 re-confirm)
- plan-devex-review: APPROVE (cycle 4)

## Acceptance criteria coverage

| AC | Status | Evidence |
|---|---|---|
| AC-1 Gateway dispatch | ✅ | gateway.rs handle_tool_call; unknown-tool → BadParams |
| AC-2 Scope + exposure authorization | ✅ | gateway.rs dispatch decision order: PairingRevoked → ExposureForbidden → Unauthorized → BadParams |
| AC-3a Handle lifecycle | ✅ | auth.rs resolve_or_mint_handle; 24h sliding; revocation |
| AC-3b Cross-session + cross-client | ✅ | auth.rs composite (handle, client_id); cross-client → ConversationRevoked |
| AC-4 Pairing revocation | ✅ | auth.rs load_client_record; per-dispatch ≤ 1 call propagation |
| AC-5 Rate limit | ✅ | gateway.rs reserve_rate_limit: BEGIN IMMEDIATE → prune → match COUNT → ROLLBACK on err/over → INSERT → COMMIT |
| AC-6 Audit append | ✅ | audit.rs write; keyed HMAC-SHA256; emit-or-log outbox; mutation_cursor caps + truncation warning; internal_reject_reason dropped per L6-2 |
| AC-7 Signal emission | ✅ | SignalEmitter trait + StderrSignalEmitter default; 5 sites per ADR-0115; emit_invoked + emit_rejected + emit_warning |
| AC-8 CI gate | ✅ | scripts/check_mcp_tool_handler_allowlist.sh (path-α: preflight wiring) |
| AC-9 Required checks | ✅ | cargo check --lib clean; cargo clippy --lib -- -D warnings clean; 34/34 mcp_v2 unit tests pass |
| AC-10 Legacy coexistence | ✅ | Legacy src/bridges/mcp.rs + src/mcp/main.rs untouched; v2 paired path is parallel ingress |
| AC-11 Key custody | ✅ | Zeroizing<[u8;32]> + Zeroizing<String/Vec> intermediates in auth.rs + audit.rs; borrow-not-clone UTF-8 decode |
| AC-12 Timing oracle | ✅ | tokio::sleep_until(start + 10ms) on all auth-state rejections; uniform error shape |

## Test results

```
cargo check --lib                                                clean
cargo clippy --lib -- -D warnings                                clean
cargo test --lib services::mcp_v2                                34/34 passed
cargo test --test dos168_mcp_v2_migration_smoke_test             1/1 passed
```

Migration smoke test (DOS-168 NEW) validates v241-v244 schema:
- mcp_client_manifest + mcp_tool_grant + idx_mcp_tool_grant_client_tool
- mcp_conversation_handle composite (handle, client_id) per ADR-0102 §D.bis
- mcp_transport_nonce_ledger with issued_at + expires_at + consumed_at + 3 indexes per §C.bis.schema
- UNIQUE(nonce, client_id) enforced (negative test)
- mcp_tool_call_ledger + mcp_audit_outbox (drained_at for sweep cadence)

## Path-α (filed as Maintenance Linear tickets; not blocking PR)

Per `feedback_l2_path_alpha_to_maintenance_project` + L0 packet §9:

1. **Production BusSignalEmitter wrapping signals::bus** — requires ActionDb + PropagationEngine wiring; W2+ scope. SignalEmitter trait + Gateway::with_emitter seam ARE landed; StderrSignalEmitter is the operator-log fallback. CSO L2 cycle-3+ APPROVE accepted this framing.

2. **AC-3b/AC-11/AC-12 behavior integration tests against live SQLite** — gateway unit tests cover pure helpers + cursor caps; full per-dispatch integration tests with stub McpToolHandler land in W2+ when the first real handler ships.

3. **CI lint script wiring into preflight/workflows** — scripts/check_mcp_tool_handler_allowlist.sh exists + works; preflight wiring is a separate Linear ticket.

4. **PreissueFailed operator runbook** — opaque trace_id format spec'd in code; runbook authoring is W2+ DX work.

5. **truncated_oversize golden integration test** — cap_mutation_cursor unit tests pass (3 cases: small-passes, oversize-bytes-truncates, deep-nesting-truncates); end-to-end golden test with stub emitter capturing warning pairs with audit truncation file ticket.

6. **MCP v2 namespace collision CI** — drops per cycle-5 L0 fix; filed as W1.5 ticket; requires W1-B YAML.

## L6 decisions (cycle-7 user verdicts)

User picked all 3 recommended L6 options:
- **L6-1**: server-side nonce ledger (per `services::surface_nonce` pattern). Landed via ADR-0102 §C.bis.replay + §C.bis.refresh + v243 migration + auth.rs verify_and_consume_and_preissue.
- **L6-2**: drop internal_reject_reason from audit — rejections via McpInvocationRejected signal (NonPiiMetadata). Landed in gateway dispatch + signal registration.
- **L6-3**: rustdoc on TaxonomyCatalog trait (not separate SDK docs). Landed in taxonomy.rs (~130 lines).

## Open questions resolved at L1

The 3 cycle-7 L0 questions ("nonce/HMAC binding"; "lost-response recovery"; "pairing response shape") were resolved during L1 implementation + verified by L2 reviewers:
- HMAC signs WHOLE envelope including request_nonce — verified by CSO + codex review L2
- Lost-response recovery: per `feedback_dont_swing_past_center_when_correcting`, fail-closed wins; consume = consumed; client uses last successful next_nonce or re-pairs — verified
- Pairing handshake response shape (PairingResponse) returns `{client_id, seed_nonce, transport_key, transport_key_ref}` — verified by devex L2

## Next steps (waiting on user)

1. **PR open** — `gh pr create --base dev` after user validation
2. **L3 wave gate** — runs after W1-B (DOS-478) also merges; codex-challenge + architect-reviewer + Suite S/P/E on integrated W1 state
3. **W1-B (DOS-478)** — tool taxonomy YAML lane; L0 packet draft in progress in parallel

Per `feedback_no_auto_tag_without_user_validation`: PR open + push to dev + tag = user-validated only.
