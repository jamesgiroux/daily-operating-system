# v1.4.1 Integrated Proof Bundle

**Version:** v1.4.1 (not yet tagged — release-gate close pending)
**Branch:** `dev` @ `93bcbebf` (post DOS-644 fixture sweep) / `58fdca7c` (W7 merge)
**Date sealed:** 2026-05-16
**Predecessor tag:** `v1.4.0-w5-complete`

---

## Wave merge timeline

| Wave | PR | Merge SHA | Title |
|---|---|---|---|
| W1 | — | (foundation; signal infrastructure + load-test gate) | `dos237_load_test`, durable invalidation, signal policy registry |
| W2 | — | (substrate completions: 12 tickets) | DOS-213/214/215/234/262/264/265/295/315/344/347/448 |
| W3 | — | (trust scoring shadow mode) | Stage-3a complete; stage-3b routed to v1.4.2 spike per **Amendment 1** |
| W4 | #281, #282, #287, #289 | various | DOS-568/569/570/571 canonicalization + signing + projection |
| W5 | #275, #276, #277 | `354d5215`, `821f3c0b`, `1109346b` | DOS-220 get_daily_readiness, DOS-221 list_open_loops, DOS-222 detect_risk_shift |
| W6 | #290 | `4ecac9ac` | validation suite — bundles 14-18 + edge-case regression meta |
| W7 | #295 | `58fdca7c` | release-gate hardening + Linear-issues chapter + opt-in telemetry |

W8 (DOS-505 eval harness/benchmark) is an independent workstream — release-blocking subset decision still pending.

---

## Review ladder verdicts (per Amendment 2: L2 wave-scope, not per-PR)

| Wave | L0 | L2 | L3 |
|---|---|---|---|
| W1 | ✅ packet APPROVE | ✅ wave-scope | n/a (single-wave) |
| W2 | ✅ packet APPROVE | ✅ wave-scope | n/a |
| W3 | ✅ packet APPROVE | ✅ wave-scope (stage-3a) | n/a; stage-3b → Amendment 1 → v1.4.2 |
| W4 | ✅ 5 packet APPROVEs | ✅ wave-scope | n/a |
| W5 | ✅ 3 packet APPROVEs | ✅ wave-scope | n/a |
| W6 | ✅ 6 packet APPROVEs | ✅ wave-scope #290 | ✅ integrated (architect APPROVE 2026-05-16) |
| W7 | ✅ 5 packet APPROVEs | ✅ wave-scope #295 | ✅ integrated (architect APPROVE 2026-05-16) |

**L3 (W6+W7 integrated)** — architect-reviewer 2026-05-16: APPROVE. ADR contracts (0102/0105/0108/0114/0120) hold end-to-end. 1 NIT filed as DOS-643 (ADR-0108 RenderPolicyChannel registry centralization).

**L5 (Drift, integrated v1.4.1)** — 2026-05-16: GAPS (remediation in progress; see below).

---

## DoD §700-712 line-by-line status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | L0→L2→L3→L4→L5 cleared | 🟡 Partial | L0/L2/L3 done; L4 + L5 in progress |
| 2 | W5 capability migrations parallel-run validated | ✅ | DOS-220 #276, DOS-221 #275, DOS-222 #277 |
| 3 | W6 adversarial fixture matrix green | ✅ | bundles 14-18 ship in #290; bundles 1-13 fixture drift resolved via DOS-644 at `93bcbebf` |
| 4 | W1 signal load-test gate | ✅ | `tests/dos237_load_test.rs` |
| 5 | Trust shadow ≥50 events × 3 bands | 🟡 Amendment 1 | Real distribution 4489/1/0; routed to v1.4.2 spike per `.docs/plans/v1.4.1-waves-amendments.md` Amendment 1 |
| 6 | clippy + cargo test + tsc green | ✅ | tsc green at 58fdca7c; cargo clippy/test from W7 CI |
| 7 | `pnpm release-gate -- --mode hermetic` exits 0 | ✅ | Resolved via DOS-645 commit `8099b5c9` — 8/8 mandatory bundles Pass, 11/11 invariants Pass |
| 8 | MCP-bridge re-test for 3 W5 capabilities | ✅ | `.docs/proofs/v1.4.1/W5-mcp-bridge-retest.md` captures 46/46 tests green |
| 9 | W8/DOS-505 stop-check recorded | ✅ | §709 release-blocking subset SATISFIED in dev (2026-05-16). Evidence: `pnpm wave8:smoke` exit 0, `pnpm eval:abilities --mode smoke` exit 0, `bash scripts/suite-p.sh --mode smoke` exit 0; all three emit valid Evaluation Evidence Records under `src-tauri/target/evidence/`. Stop-check decision posted on DOS-505 comment `613d568f-1467-421e-a312-a7a45b36893e`. |
| 10 | Dogfood ≥20 real-dev meetings | ❌ GAP | Not captured |
| 11 | Proof bundle written | 🟢 This doc | — |
| 12 | Tag v1.4.1 on trunk after dev merge | ❌ Gated | Version files at 1.2.1; CHANGELOG no v1.4.1 entry; `dev → trunk` not merged; tag pending user release-checklist + UI validation per `feedback_no_auto_tag_without_user_validation` |

---

## Path-α tickets filed (L2/L3/L5 surface findings)

All filed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`:

| Ticket | Source | Priority | Summary |
|---|---|---|---|
| DOS-637 | W7 L2 | Medium | W7-E disable+flush race window |
| DOS-638 | W7 L2 | Medium | W7-B 64KB cap math edge cases |
| DOS-639 | W7 L2 | Medium | W7-A claim-backed deepening audit |
| DOS-640 | W7 L2 | Medium | W7-E default-OFF integration test |
| DOS-641 | W7 maintenance | Medium | BSD-grep portability in pre-commit hook |
| DOS-642 | W7 L2 | Low | W7-E in-memory buffer UX clarification |
| DOS-643 | W6+W7 L3 | Medium | ADR-0108 RenderPolicyChannel registry centralization |
| DOS-644 | L5 | Urgent → **closed via `93bcbebf`** | Bundle 1-13 fixture schema drift (claim_version/canonical_status/non_semantic_mergeable) — sweep complete |
| DOS-645 | L5 | Urgent → **closed via `8099b5c9`** | Release-gate hermetic exits 0; bundles 14-18 invariant evaluators wired; harness pipeline integrated |
| DOS-648 | v1.4.2 dev | High | Linear projects picker typeahead search (blocks linking actions to per-entity Linear projects in large orgs) |

---

## Substrate integrity check (L3 architect verdict)

ADR contracts preserved end-to-end across W3+W5+W6+W7 integrated state:

- **ADR-0102 (abilities runtime / ServiceContext):** W0.5 crate boundary + W5 migrations land inside `abilities-runtime/`. W2-F operations contract consumed.
- **ADR-0105 (provenance first-class):** `source_ref`, `subject_ref`, `claim_type`, `source_asof`, `source_lifecycle_state` propagated through Linear chapter + bundle-17 fixtures.
- **ADR-0108 (sensitivity rendering):** 9-channel sweep parameterized; `RenderPolicyChannel::all()` exhaustive iterator; bundle-17 + W7-A Linear MCP filter assertion both enforce. DOS-643 NIT for registry centralization.
- **ADR-0114 (scoring unification):** No regressions; trust threshold literals config-driven inside `abilities/trust/`. W7-A `linear_issue_state_weight` composed via TrustFactorInputs.
- **ADR-0120 §10 (telemetry contract):** W7-E AggregateMetric struct ships verbatim per §10. `FORBIDDEN_AGGREGATE_FIELDS` lint, `AGGREGATE_METRIC_CATALOG` enumerated, HttpsUrl newtype enforces TLS at compile time.

---

## Amendments honored

- **Amendment 1** (W3 stage-3b → v1.4.2 spike): Documented at `.docs/plans/v1.4.1-waves-amendments.md`. Stage-3b proof bundle at `.docs/plans/v1.4.1-waves/W3-stage-3b-proof-bundle.md` self-reports PARTIAL (2/6 criteria met). DoD §705 reconciliation pending (release-notes language vs amend §705 text).
- **Amendment 2** (L2 wave-scope, not per-PR): Documented at same file. Applied to W6 (#290) + W7 (#295) — one L2 panel per wave against integrated diff. Codified going forward.

---

## Remaining release-tag gates

Before `v1.4.1` tag:
1. ~~DOS-645~~ ✅ resolved (`8099b5c9`)
2. ~~MCP-bridge re-test artifact~~ ✅ captured
3. ~~W8 DOS-505 stop/check decision~~ ✅ recorded on DOS-505 comment `613d568f` — release-blocking subset complete in dev
4. ~~DoD §705 trust-band reconciliation~~ ✅ amended at `0ab1a213`
5. ~~Version files bumped~~ ✅ at `0ab1a213` (`1.4.1` across all three)
6. ~~CHANGELOG entry~~ ✅ at `0ab1a213`
7. **Manual dogfood evidence ≥20 real-dev meetings** — pending user
8. **L4 surface QA on entity Linear chapter + telemetry splash + Privacy panel** — pending user
9. **User release-checklist + hands-on UI validation** (per `feedback_no_auto_tag_without_user_validation`) — pending user
10. **`dev → trunk` merge** — pending user
11. **Tag `v1.4.1` on `trunk`** — pending user

Substrate work complete. Remaining items are real-usage validation + the release-cut lane that requires the user's hands on the running app.

## First substrate consumer on the actions surface

v1.4.1 ships with `ActionRow` rendering `<TrustBandIndicator>` next to action titles when `trust_band` is set on the underlying commitment claim (commit `ca124e2a`). Cascades to TheWork chapter (entity pages), MeetingDetailPage, and ActionsPage. Use-with-caution + needs-verification bands appear as tooltip chips; likely-current stays clean by design. Actions that haven't been substrate-scored show no chip (designed behavior).

`MeetingDetailPage` already consumed `TrustBandIndicator` from v1.4.0 era — ActionRow is the first consumer on the **actions** surface, not the first substrate consumer overall.

### Feedback loop status

User accept/reject/edit on actions does NOT yet refeed trust-factor weights for that commitment lineage. Per Amendment 1, this closure is routed to the v1.4.2 spike outcome and is not in v1.4.1 scope. Substrate plumbing is correct (services → db → query hydration → TS type → UI); the missing piece is the bidirectional weight update on correction.

---

## Authority surface

- **Linear** is canonical for ticket state, reviewer verdicts, L6 decisions
- **Git** carries this proof bundle + wave plan + amendments + ADRs
- **Slack** is visibility for digests + L6 DM interactions

Reviewer verdicts per CLAUDE.md Review Ladder are comments on the relevant Linear tickets; this bundle aggregates the integrated state, not the per-ticket history.
