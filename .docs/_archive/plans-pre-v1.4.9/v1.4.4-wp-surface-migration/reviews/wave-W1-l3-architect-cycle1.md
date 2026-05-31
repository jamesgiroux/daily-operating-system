# L3 Wave Review — v1.4.4 W1 Substrate (architect-reviewer, cycle 1)

**Reviewer:** architect-reviewer (L3 wave-architectural pass per `.docs/plans/engineering-ladder.md` line 19)
**Branch:** `wave/v1.4.4-w1-stage1a`
**Diff:** `0f8533e1..HEAD` (30 commits — 10 sub-tickets + 3 L2 cycle patches)
**Date:** 2026-05-20

## VERDICT: APPROVE

L3 evaluates the integrated wave state as a whole. All six focus areas hold up architecturally. Two L2 cycle-3 P2 findings (DOS-8 idempotency race + lifecycle_changed accuracy) are correctly routed to path-α per James 2026-05-20 — they are not wave-architectural and do not block the L3 gate. No new wave-level findings.

---

## Area-by-area assessment

### 1. Producer / projection / renderer coherence (ADR-0130) — APPROVE

The three named producers compose cleanly without contract conflict:

- `get_entity_intelligence` (DOS-459) — Read ability, ENVELOPE_SCHEMA_VERSION = 1, additive-only minor bump policy declared. Producer at `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/producer.rs:55` validates input.schema_version on entry and stamps output envelope at `:146`. `EntityFact.provenance: ProvenanceRef` shape conforms to ADR-0130 §2.
- `get_daily_briefing` (DOS-507) — Read/User-only ability with `BriefingState { availability, freshness, integrity, advisories }` composed shape per cycle-1 correctness F3 fold. NotExposed to MCP per L0 decision.
- `build_receipt_for_audience` (DOS-341) — audience-as-input construction at `services/claim_receipt/privacy.rs` with per-audience allowlist; redaction at emit boundary, not consumer boundary (matches invariant #4 of W1 §10).

All three are Read producers; W2–W5 own the projection (`wp/dailyos/blocks/*/render-functions.php`) per ADR-0130 §4. No outer/inner block contract violation in W1 because W1 ships substrate; W2 owns the `dailyos/account-detail` outer block.

### 2. Signal propagation correctness — APPROVE

Both new and reused signals register cleanly in `src-tauri/src/signals/policy_registry.rs`:

- `ClaimVerificationStateChanged` at `:26` / `:160` / `:278` — existing, consumed by DOS-339 fan-out.
- `MeetingPrepStatusChanged` at `:74` / `:201` / `:319` — pre-declared at W1 kickoff per cycle-1 arch F3 fold; policy fn at `:663` (`meeting_prep_status_changed_policy`).

Ordering under realistic scenarios:
1. User authors meeting note → `services::meeting_prep_status::write::record_user_authored` → updates status row → emits `MeetingPrepStatusChanged`.
2. Claim verification state mutates upstream → `ClaimVerificationStateChanged` fires → DOS-339 `event_bridge.rs:68` emits `claim_receipt:invalidated` Tauri event with 250ms trailing-edge debounce per `(target.claim_id, surface)` (AC-339.6).
3. TS hook `useClaimReceiptSubscription` at `src/services/claim-receipt/useClaimReceiptSubscription.ts:107` listens, re-fetches.

The two signal paths are independent — `MeetingPrepStatusChanged` does not transit through `claim_receipt:invalidated`. No fan-out collision risk. The wave-internal ordering is correct.

### 3. Migration slot coordination — APPROVE

Wave claimed v240–v249. Landed migrations:

| Slot | File | Sub-ticket |
|---|---|---|
| v240 | `240_claim_review_deferrals.sql` | DOS-701 (prior) |
| v241 | `241_meeting_prep_status_indexed_view.sql` | DOS-335 |
| v242 | `242_meeting_prep_status_dismissals.sql` | DOS-335 |
| v243 | `243_meeting_prep_status_indexed_view_deterministic.sql` | DOS-335 (cycle-2 view determinism patch) |

All in `src-tauri/src/migrations.rs:928-948`. Six slots remain free in the block (v244–v249).

Cross-wave check:
- v1.4.5 holds v200–v219 — disjoint.
- v1.4.6 holds v260–v279 — disjoint; `.docs/plans/v1.4.6-waves.md:327` cross-version coordination table confirms v1.4.4 = v240–v249.
- v250–v259 explicitly held unclaimed as buffer per v1.4.6 wave plan.

No collision. Slot discipline holds.

### 4. Read-ability non-mutation contract (ADR-0102 §3) — APPROVE

Both call-graph lints implemented and enforce in-tree:

- **AC-335.12** at `src-tauri/src/services/meeting_prep_status/read.rs:206-263` — `mod call_graph_lint` denies mutation identifiers (`enqueue_refresh` at `:233`, `emit_signal` at `:236`) in the read path. The `compute_status` function at `read.rs:54` reads only.
- **AC-507.7** at `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:791-834` — mirrors the AC-335.12 pattern; denies mutation-suggestive identifiers in the briefing producer call graph.

The read/write split is structural: `services::meeting_prep_status::{read, write}` are separate files; `write::enqueue_refresh` at `write.rs:57` is the only mutation path. Transitive mutation through `compute_status` is statically refuted by the lint.

### 5. Forward-coupling risk — APPROVE

DOS-459 envelope schema_version policy is documented in W1 L0 §2 changelog as **semver-style additive-only minor bumps**. Implementation matches: `ENVELOPE_SCHEMA_VERSION` const at producer.rs:146, validator at `:179-184` returns typed error for unsupported versions. v1.4.5 (workspace memory source/ingestion claims) and v1.4.6 (RecommendationProposal claims) can land as minor bumps without breaking existing envelope consumers.

Risk surface: v1.4.5/v1.4.6 must not introduce envelope FIELD removal or renames. This is encoded in the policy but not yet CI-enforced. **Not a blocker** — v1.4.5 W1 L0 panel is the right place to require an additive-only contract test once that wave authors its envelope extension.

### 6. W1 → W2 transition readiness — APPROVE

AC-W1.9 CI gate (`src-tauri/scripts/check_w1_consumer_skeleton.sh`) PASSES — every W1 producer has at least one downstream `wp/dailyos/blocks/*/render-functions.php` consumer skeleton. Skeletons present for:

- `account-detail/render-functions.php` (DOS-459 / DOS-460 / DOS-477 consumer)
- `project-detail/render-functions.php` (DOS-459 / DOS-460 consumer)
- `person-detail/render-functions.php` (DOS-459 / DOS-460 consumer)
- `daily-briefing/render-functions.php` (DOS-507 / DOS-335 / DOS-339 consumer)

W2 inherits a documented wiring path. No empty composer risk per CLAUDE.md "Definition of Done" — the four block stubs encode the producer → block contract that W2 fills.

---

## Suite S / P / E confirmation

- **Suite S (Security) — GREEN.** Three CI lint scripts pass: `check_w1_consumer_skeleton.sh`, `check_sensitivity_gate_composition.sh` ("All claim-sensitivity branching routes through the canonical gate"), `check_audit_disclosure_allowlist.sh` ("DOS-340: receipt-vs-audit disclosure allowlist clean"). `/cso` cycle-2 + cycle-3 APPROVE recorded at `wave-W1-l2-cso-cycle2.md`. DOS-749 / DOS-750 path-α tickets filed for cycle-3 P2 findings.
- **Suite P (Performance) — N/A at this stage.** Substrate is Read abilities + write paths, not hot-path. Cargo test elapsed 917s for 2637 tests is baseline. No performance vector exposed by W1 work itself. v1.4.6 Salience + W3 briefing surfaces are the right perf-suite gate.
- **Suite E (Edge cases) — GREEN.** DOS-461 entity_fixture_harness ships 53 tests across Account / Project / Person matrix; cargo test --lib 2637/0; property tests landed for DOS-335 (write commutativity per AC-335.13), DOS-477 (composes set per AC-477.13), DOS-507 (BriefingState multi-dimensional per AC-507.4).

---

## Wave-level findings

**None.** L3 surfaces no wave-architectural finding that re-opens W1.

The two L2 cycle-3 P2 findings (idempotency race + lifecycle_changed accuracy in DOS-8 feedback path) are correctly path-α; both are local correctness gaps in `services/claim_receipt/feedback.rs`, not contract violations, signal-propagation breakages, or boundary failures. The class-pattern sweep recommendation in the cycle-3 verdict (feedback-path edge cases) is the right structural response and belongs in maintenance, not in W1 cycle-N+1.

---

## L3 close

W1 substrate wave is architecturally ready. Producer / projection / renderer split is clean; signals are pre-declared with correct policies; migrations sit in the claimed block without cross-wave collision; Read-ability non-mutation is statically enforced; envelope schema_version policy is forward-compatible with v1.4.5 / v1.4.6 extensions; W2 wiring path is documented and gated.

**Proceed to PR open + L4 (Surface) when W2 begins consuming.** L5 (Drift) checkpoint is not required at W1 close — drift risk is W2+ when surfaces start consuming the envelope.
