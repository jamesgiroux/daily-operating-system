# Wave W4 Proof Bundle

**Wave:** W4 (substrate spine for the abilities runtime — DOS-5 Trust Compiler, DOS-216 Eval Harness, DOS-217 Bridges, DOS-383 External Replay Framework, DOS-384 Bundle Catalog)
**Status:** Complete through cycle-20 L2 review closure (APPROVE/SHIP)
**Date:** 2026-05-05

---

## Initial wave landing (pre-L2 review)

W4 substrate primitives landed across W4-A, W4-B, W4-C, DOS-383, DOS-384 prior to the cycle-3 L2 review opening. The cycle-1 + cycle-2 review loops closed 11 BLOCK/REVISE classes (A-K) before this proof bundle's commit range begins; that scope is documented in the wave-W4 plan files for each ticket.

This proof bundle covers the **L2 review closure** that ran cycles 3 through 20, plus the wave-decision pulls (signals/bus.rs atomicity, lifecycle review state machine, bundled `let _ =` sweep) that the user pulled into the wave at cycle 16.

## L2 review closure — final commit chain (local-only)

| Commit | Cycle | What |
|---|---|---|
| `af257f96` | recovery | Stable `briefing_callouts.id` so dedup actually fires (1.6M dup rows on prod) |
| `935a3c5b` | recovery | Chunked `Backup::step` + source-size log (encrypted-DB > 300MB backup safety) |
| `6863edcb` | recovery | Re-apply migration 140 registration (TemporalScope::Closed) |
| `f9a4363e` | 3 | Hard-policy gates (Sensitivity / SourceWithdrawn / AuthoritativeContradiction) before geometric mean |
| `a18695e9` | 3 | Wire corroborators + internal_consistency into production trust recompute |
| `c01ae1d1` | 3 | Server-side `TauriConfirmationStore` + consume-once + forged/replay regression tests |
| `158ff5fc` | 3 | Gate MCP `request_confirmation` off until W5/W6 prompt UI lands |
| `fd2cec03` | 4 | Close silent-error swallow class in cycle-3 recovery paths (corroborator/internal_consistency/callout) |
| `e3e3285b` | 4 | Migration backup deadline + IndeterminateReadState band forcing + MCP list_tools gating |
| `0324cc6c` | 5 | `IndeterminateReadState` gate replaces ad-hoc fail-closed signals |
| `6abb24a1` | 6 | Widen indeterminate-read sweep to `trust_feedback_signal` + `signal_weights` |
| `82d919ba` | 7 | Finiteness guard + parse-failure read_ok + GenerateCalloutsOutcome shape |
| `92ff52b0` | 8 | Negative components + present-but-invalid metadata + callout decode flag |
| `7b9ee507` | 9 | `TrustInput<T>` type refactor + reasons + `trust_read_state_indeterminate` pipeline metric + `emit_signal` sweep |
| `0bf37646` | 10 | `services/feedback.rs` + `services/actions.rs` Result-drop closures + corroboration validation |
| `f7d25de5` | 11 | `services/accounts.rs` correction-flow drops + telemetry-of-telemetry |
| `c9c9001f` | 12 | Account correction transactionality + product ownership scoped update |
| `c7588804` | 13 | Source-weight authorization gap + idempotency + lifecycle outer txn |
| `cbb91b6c` | 14 | Pending-row claim ordered BEFORE destructive mutation |
| `2f5e66c9` | 15 | Drift detection on `correct_lifecycle_change` |
| `1fba3672` | 16 | Symmetric drift guard on `confirm_lifecycle_change` + test fixture alignment |
| `5a4d76a2` | wave | bus.rs atomic emit_and_propagate + lifecycle review state machine |
| `9fa8df99` | wave | Bundled `let _ =` sweep — 40+ sites → `emit_or_log` / `emit_and_propagate_or_log` |
| `ac681b47` | 17 | Correct anchor on `change.new_*` + propagate `LifecycleReviewOutcome` to commands |
| `cc3cfd9a` | 18 | Stale-review rollback via sentinel + contract_end restoration on correction |
| `ae104c38` | 19 | Scope contract_end restoration to lifecycle-rejection + reorder before health recompute |

Cycle 20: APPROVE / SHIP — no material findings.

---

## Major architectural pulls into the wave (cycle-13-onward decisions)

### `TrustInput<T>` type system (cycle 9)
- All 8 trust-recompute helpers (`source_reliability`, `source_lifecycle`, `freshness`, `corroboration_strength`, `corroborators`, `contradiction_count`, `feedback`, `internal_consistency`) return `TrustInput<T>` with `&'static` reason strings.
- `build_trust_context_for_claim` aggregates reasons via `collect_trust_input` into a `Vec<&'static str>`.
- Aggregation surfaces as `pipeline_failures(kind="trust_read_state_indeterminate", payload="claim_id=… reasons=…,…")` so observability sees both the trigger and the specific cause.
- New `TrustGateKind::IndeterminateReadState` short-circuits `compile_trust` to `NeedsVerification` regardless of factor weights.

### `emit_signal_and_propagate` atomic (cycle 11 → wave)
- Single `db.with_transaction` boundary across source-signal insert, every derived-signal insert, every derivation-link insert, and the meeting-fanout writes.
- Meeting fanout stays best-effort (warn-log) by design — it's a denormalized join-table write.
- Closes the partial-state-on-Err class flagged in cycle-11 review.

### `LifecycleReviewOutcome` state machine (cycles 13-19 → wave)
- Single source of truth `review_lifecycle_change(action: LifecycleReviewAction)` for both Confirm and Correct.
- Three explicit invariants enforced inside one outer transaction:
  1. **Pending claim** — `set_lifecycle_change_response_if_pending` returns `usize`; rows=0 → `AlreadyReviewed`.
  2. **Drift guard** — current account state vs `change.new_*`; mismatch → `STALE_DRIFT_SENTINEL` Err that rolls the txn back, outer `.or_else` unwraps to `LifecycleReviewOutcome::StaleDrift` so retries stay stale.
  3. **Side effects only on `Applied`** — source-weight upsert, signal emission (Confirm) or `apply_lifecycle_transition` + contract_end restoration + source-weight penalty (Correct).
- contract_end restoration scoped to lifecycle-rejection (`corrected_lifecycle != change.new_lifecycle`) + ordered BEFORE `apply_lifecycle_transition` so health recompute observes the corrected date.
- Tauri command boundary maps `Applied | AlreadyReviewed → Ok(())`, `StaleDrift → Err(human-readable message)`.

### Bundled `let _ =` sweep (cycle 12 recommendation → wave)
- `services/signals.rs` gains `emit_or_log` and `emit_and_propagate_or_log` wrappers that warn-log on failure.
- 40+ call sites across `services/`, `abilities/`, `clay/`, `context_provider/`, `executor.rs`, `google.rs` converted from `let _ = ... emit*(...)` to the wrapper.
- Two `emit_propagate_and_evaluate` sites got inline `if let Err` (single caller variant; not worth a wrapper).
- Sites with pre-existing `.map_err(...)?` were intentionally NOT swept — they were already error-propagating.

---

## DB recovery operation (cycle-3 day)

Live development DB hit a SQLCipher `btreeInitPage` corruption on page 47 (rooting `app_state_kv`), blocking pre-migration backup with the canonical "not an error" wrapper string. Recovery executed during the cycle-3 → cycle-4 transition:

- 113 tables dumped individually (per-table fresh-connection workaround for the cascading "out of memory" downstream of page 47) — 113 ok / 0 fail.
- Identified `briefing_callouts` table at 1,614,301 rows (~600 MB) caused by `id: format!("bc-{}", Uuid::new_v4())` defeating `INSERT OR IGNORE` dedup.
- Rebuilt from per-table dumps + dedup query keyed on `signal_id`: **748 MB → 201 MB** (74% reduction), `PRAGMA integrity_check = ok`, all critical tables row-for-row identical.
- Original corrupt DB preserved at `dailyos.db.corrupt-20260505-pre-recovery`. May 3 backup safety-copied to `~/Documents/dailyos-recovery/`.

---

## Tests added (this wave's L2 closure)

| File | Tests added | Coverage |
|---|---|---|
| `migrations.rs` | `migration_140_relaxes_temporal_scope_to_accept_closed` | v139→v140 transition with row preservation |
| `signals/callouts.rs` | `callout_id_is_stable_so_repeated_runs_do_not_duplicate` | 5 reruns → 1 row via INSERT OR IGNORE |
| `abilities/trust/mod.rs` | 4: `confidential_on_public_caps_at_needs_verification_under_default_weights`, `withdrawn_source_caps_at_needs_verification_under_default_weights`, `single_strong_contradiction_caps_at_needs_verification_under_default_weights`, `indeterminate_read_state_caps_at_needs_verification_even_with_strong_confirming_evidence` | Hard-policy gate band forcing |
| `intel_queue.rs` | 4: `source_reliability_corroborators_reads_corroborations_and_unreconciled_contradictions`, `internal_consistency_honors_metadata_hint_otherwise_defaults_to_one`, `trust_feedback_signal_reports_read_error_when_table_missing`, `source_reliability_reports_read_error_when_signal_weights_missing` | Production trust recompute wiring + read-failure surfacing |
| `bridges/tauri.rs` | 2: `forged_confirmation_token_not_issued_by_bridge_is_rejected`, `server_issued_confirmation_token_passes_lookup_then_consumes_on_first_use` | Server-side token store + consume-once |
| `bridges/mcp.rs` | `mcp_request_confirmation_disabled_by_default_until_prompt_ui_ships` | MCP gate off-by-default |
| `mcp/main.rs` | `mcp_list_tools_omits_request_confirmation_when_gate_disabled` | list_tools omits the gated tool |

Plus 2 fixture corrections in existing tests for the lifecycle drift guard semantics (account state must match `change.new_*` post-auto-transition).

**Final test status:** 2235 passed / 0 failed / 10 ignored. Clippy clean (`-D warnings -A non-snake-case`).

---

## Suite reports

### Suite S — Security (W4 gate)
- Cycle 3 finding `c01ae1d1`: closed Tauri ConfirmationToken forgeability via server-side store + consume-once + forged-token regression test.
- Cycle 12 finding `c9c9001f`: closed product-correction cross-account write via `update_account_product_scoped` (id+account_id WHERE clause + rows-affected check).
- Cycle 13 finding `c7588804`: closed source-weight authorization gap — caller-supplied `source_to_penalize` no longer trusted; row's actual source loaded inside txn.

### Suite P — Performance (informal)
- DB rebuild reduced live DB 748MB → 201MB.
- Trust compiler p99 < 5ms threshold maintained (`trust_compiler_p99_under_5ms_claim_volume`).
- Backup chunked-step path tested for 10MB+ encrypted source via `test_pre_migration_backup_created`.

### Suite E — Edge cases
- Bundle 1-8 substrate fixture tests pass via DOS-216 harness loader.
- Cycles 13-19 progressively hardened the lifecycle review surface against: stale rows, idempotent retry, drift between auto-detection and review, contract_end rollback, transaction rollback semantics, side-effect ordering.

---

## CI invariants now structurally enforced (this wave)

- TrustFactorInputs cannot be constructed without `read_state_indeterminate` field (Rust struct field is non-optional + serde defaults to false).
- `LifecycleReviewOutcome` is the only return type from `review_lifecycle_change`; `confirm_lifecycle_change` and `correct_lifecycle_change` cannot bypass the state machine.
- `update_account_product` retained for backward compatibility but ALL correction paths route through `update_account_product_scoped`.
- `emit_or_log` / `emit_and_propagate_or_log` are the only intentional Result-drop sites in service layer (manual audit; not yet a clippy lint).

---

## Known gaps / follow-ups

| Item | Source | Disposition |
|---|---|---|
| `intelligence::write_fence::tests::dos311_substrate_migration_sequence_end_to_end` order-dependent flake | Pre-existing | Not from this wave; passes solo; out of scope |
| Regression test for lifecycle rejection restoring contract_end before health recompute | Cycle 20 optional | v1.4.1 candidate |
| Regression test for same-lifecycle stage-only correction preserving rolled date | Cycle 20 optional | v1.4.1 candidate |
| Clippy/CI lint forbidding future `let _ = signals::emit*` regressions | Cycle 12 reviewer recommendation | v1.4.1 candidate |
| `correct_lifecycle_change` provenance restore — currently stamps `user_correction` rather than carrying the original auto-detection source | Implicit | v1.4.1 candidate |
| `app_state_kv` corrupt-page recovery — table currently empty post-rebuild; consumers (`briefing_freshness`, daily PTY budget, morning_flags) self-repopulate but a migration-backfill helper would harden the recovery path | Recovery operation | v1.4.1 candidate |

---

## Frozen-contract verification for next wave

- `TrustInput<T>` contract: any new trust input helper MUST return `TrustInput<T>` and OR its reason into `build_trust_context_for_claim`'s collected reasons.
- `emit_signal_and_propagate` atomicity: callers MUST NOT add side-effects between source-signal insert and propagation that should not roll back together.
- `LifecycleReviewOutcome` contract: new lifecycle review actions MUST extend the state machine (not bypass it). New action variants MUST declare their drift anchor (currently `change.new_*` for Confirm and Correct).
- Tauri confirmation-token flow MUST issue server-side via `TauriConfirmationStore::issue` and consume via `consume`. Renderer-supplied token structs are rejected unless their opaque id was server-issued.
