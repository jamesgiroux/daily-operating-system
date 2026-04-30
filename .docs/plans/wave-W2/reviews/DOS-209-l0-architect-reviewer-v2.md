# L0 review cycle 2 — DOS-209 plan v2 — architect-reviewer

**Reviewer:** architect-reviewer (substrate / schema profile, cycle 2)
**Plan revision under review:** v2 (2026-04-28)
**Verdict:** APPROVE

## Cycle 1 finding closures verified

### F1 (IntelligenceProvider placement) — closed: yes
Closure location in v2: §2 paragraph 1 ("`IntelligenceProvider` deliberately does not land on `ServiceContext`; W2-B owns provider extraction, and ADR-0104 places provider selection on `AbilityContext`.") and §3 paragraph "`TxCtx` exposes ... no intelligence provider."
Verification: v2 enumerates the eight carriers (`db`, `signals`, `intel_queue`, `mode`, `clock`, `rng`, `external`, `tx`) and explicitly excludes `IntelligenceProvider`, citing ADR-0104. The exclusion is restated on `TxCtx` (no LLM/external in transactions), which is the right architectural propagation — the seam consumers in W3-A (registry) and W4-C (invoke_ability bridge) now have a frozen carrier list to compose `AbilityContext` over. Closure is at the architectural level, not just text.

### F2 (Plan/Apply split scope) — closed: yes
Closure location in v2: §3 paragraph 5 ("`PlannedMutationSet`, `PlannedMutation`, `ProvenanceRef`, and `plan_*` naming do not land in W2-A. They are deferred to W3-A/DOS-210 for the ability planning surface, with `ProvenanceRef` supplied by W3-B/DOS-211 before W4-C consumes the bridge.").
Verification: This is a wave assignment, not a defer-without-owner. W3-A owns `PlannedMutationSet`/`PlannedMutation`/`plan_*`, W3-B owns `ProvenanceRef`, and W4-C consumes the composed surface. That ordering makes the bridge type available before the consumer. Rationale (no Phase-0 DOS-209 caller consumes planned mutations) is sound — `check_mutation_allowed` on `ServiceContext` is defense-in-depth, the Plan/Apply substrate is the ability-facing surface, and they don't share a hard prerequisite. Closure complete.

### F3 (Migration strategy concreteness) — closed: yes
Closure location in v2: §2 paragraph 3 (single PR, no feature flag, no backward-compat default; PR includes `dos209_mutation_catalog.rs` generated from the audit script; mixed final state forbidden, mid-PR working-tree may be temporarily compile-broken) and §3's full mutation catalogue table (24 service modules, ~210 mutator rows enumerated with kinds D/SQL/TX/SIG/FS/BG/EXT/C and coverage column).
Verification: All three sub-asks are answered. (a) Enumeration mechanism: audit script + checked-in `dos209_mutation_catalog.rs` structural lint, with the catalogue table in §3 as the human-readable artifact. (b) Mid-PR state: explicitly local-only compile-broken windows allowed, but the reviewable PR forbids mixed signatures. (c) `test_live()` scope: `#[cfg(test)]`-only, "not a production migration shim" — guard in CI. The catalogue-as-table is materially stronger than cycle 1's "grep audit" prose; it is now reviewable and mechanically checkable.

### F4 (W2-A/W2-B rebase order) — closed: yes
Closure location in v2: §7 paragraph 2 ("W2-B opens/lands first per coordination guidance. W2-B owns provider extraction... W2-A then rebases and touches only service mutation boundaries in `services/intelligence.rs`...") with the explicit residual mutator list (`enrich_entity`, `persist_entity_keywords`, `upsert_assessment_from_enrichment`, `upsert_assessment_snapshot`, `upsert_health_outlook_signals`, `upsert_inferred_relationships_from_enrichment`, `update_intelligence_field`, `update_stakeholders`, `dismiss_intelligence_item`, `recompute_entity_health*`, `bulk_recompute_health`, `track_recommendation`, `dismiss_recommendation`, `mark_commitment_done`) and the protocol "If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2."
Verification: Explicit merge order chosen (W2-B first). The protocol covers the cross-file guard-preservation concern from cycle 1: any function W2-B extracts must retain the guard at L2 review time, with W2-A as the catalog owner. Closed.

### F5 (ExternalClients shape) — closed: yes
Closure location in v2: §3 paragraph 2 ("`ExternalClients` is a concrete struct with named wrapper fields `glean`, `slack`, `gmail`, and `salesforce`; wrapper internals hold live clients only in Live and replay/fixture wrappers in Simulate/Evaluate.") and §4 ("external replay wrappers must not contain live secrets in non-Live modes.").
Verification: Promoted from Open Question to Key Decision. Concrete struct with named fields matches ADR-0104 §2; replay/fixture wrappers in Simulate/Evaluate match ADR-0104 §"Known Limitations" #4 contract-only stance until ADR-0107. W3-A and W4-A now have a real type to compile against (`ctx.services.external.glean.*`). Closed.

### F6 (Suite-P-relevant budget) — closed: yes
Closure location in v2: §5 ("Hot paths touched: `accounts::update_account_field_inner`, `meetings::capture_meeting_outcome`, `meetings::refresh_meeting_preps`, `intelligence::persist_entity_keywords`, `intelligence::upsert_health_outlook_signals`, and transaction wrappers. Budget: p99 deviation under 5% versus the W1 Suite P baseline, and empty guard overhead below measurement noise for single mutators.") with measurement plan citing W1 proof bundle and W2-end re-measurement artifact requirement.
Verification: Named mutator set, named budget (5% p99 deviation), named comparand (W1 Suite P baseline), named artifact (W2 proof bundle). This converts "no regression" into a falsifiable claim. Closed.

### F7 (DB CURRENT_TIMESTAMP audit) — closed: yes
Closure location in v2: §3 final paragraph ("`CURRENT_TIMESTAMP` audit from `rg CURRENT_TIMESTAMP src-tauri/src/migrations/`: `081_init_tasks.sql:6 completed_at`; `068_success_plans.sql:112,113,131,132,143,160`; `044_user_entity.sql:23,24,33,34`; `050_reports.sql:10,11`; `051_entity_context_entries.sql:8,9`; `069_account_events_expand.sql:53,54,77,78`. W2-A documents and files the follow-on if not converted in this PR; conversion is not required for DOS-209 completion.").
Verification: Audit list is concrete (file + line numbers), follow-on is committed. The remaining gap — that W2-A says "files the follow-on" without a Linear ticket id — is acceptable because the issue is filed at PR time, not plan time, and W3-G/W4-A can reference the audit list directly from this plan. Closed.

### F8 (Mutation-coverage contract test) — closed: yes
Closure location in v2: §9 test list, specifically:
- `src-tauri/tests/dos209_mutation_catalog.rs::catalog_every_mutator_guarded_first_line` (structural lint over the catalogue)
- `src-tauri/tests/dos209_mode_boundary.rs::evaluate_catalog_public_mutators_return_write_blocked_by_mode` (runtime, iterates public mutators under Evaluate)
- `src-tauri/tests/dos209_surface_constructors.rs::all_live_surfaces_construct_new_live`
Verification: Cycle 1 asked for a contract test that enumerates every public mutator and asserts `WriteBlockedByMode` under Evaluate. v2 splits into two layers (structural lint over the full catalogue, runtime over the public subset), which is stronger than the single-test ask. The proptest is retained for the 4-line match but no longer load-bearing. Closed.

### F9 (5-question CI enforcement activation) — partial
Closure location in v2: §6 ("Services-only mutations: this PR activates the W2 CI enforcement mechanism from the wave invariants table. The guard catalogue prevents unguarded service mutators; lint rejects raw clock/RNG calls in `services/` and `abilities/` even though `src-tauri/src/abilities/` does not exist yet.") and Intelligence Loop 5-answers paragraph.
Verification: §6 activates the *services-only-mutations* CI invariant from the wave invariants table — that is the architecturally load-bearing one for DOS-209. The 5-question PR-template/CI-bot mechanism specifically (the cycle-1 ask) is not named here, but the wave invariants table also lists "services-only mutations" as the W2-activated invariant; DOS-209 is the natural home for that one specifically. The 5-question schema-PR enforcement is more naturally activated by a schema-touching ticket in W2/W3 (DOS-209 lands no schema). Calling this partial rather than no-closure: v2 activates the W2 invariant DOS-209 actually owns, and defers the 5-question PR-template to whichever W2/W3 ticket first introduces a schema change. That is a defensible re-scoping of the cycle-1 finding rather than a miss, and does not block APPROVE on its own.

## Fresh findings introduced in v2

None at High or Critical severity.

Two minor observations (informational, not blocking):

**O1 — Catalog table coverage column reads "catalog+runtime" uniformly without distinguishing "public mutator" from "internal helper".** The runtime test (`evaluate_catalog_public_mutators_return_write_blocked_by_mode`) iterates *public* mutators only, but several catalogue rows (e.g. `update_account_field_inner`, `update_stakeholder_engagement_inner`, `add_stakeholder_role_inner`, `persist_and_invalidate_entity_links_sync`) are inner helpers reachable only via their public wrappers. These will not be hit by the runtime test directly, only via the structural lint. v2 already says "Evaluate runtime coverage where public" in §3, so this is documented; the catalogue column just doesn't visually distinguish. Not load-bearing — the structural lint covers them. No action needed.

**O2 — `dashboard.rs` row says "no production mutation; `get_dashboard_data_inner` is clock-only and must use `ctx.clock`" with coverage "lint".** This is fine but worth flagging: if any future change adds a mutation to `dashboard.rs`, the catalogue must be updated. The structural lint only checks listed entries; un-listed mutators in un-listed files would slip. This is a known property of allow-list lints, not a v2 defect.

## End-state alignment assessment (cycle 2)

v2 freezes a seam W3 and W4 consumers can adopt without breaking changes:

- **W3-A (ability registry, DOS-210):** Consumes `AbilityContext` which composes over `ServiceContext`. The eight-carrier list is now explicit, `IntelligenceProvider` exclusion is named, `ExternalClients` is a concrete struct with named fields. Registry can compile against this surface. Plan/Apply types (`PlannedMutationSet`, `PlannedMutation`) are W3-A's own deliverable per v2's wave assignment, so W3-A is self-consistent.
- **W3-B (claims/provenance, DOS-211):** Owns `ProvenanceRef`. v2 confirms it ships before W4-C consumes the bridge. The `services/claims.rs::commit_claim` path will receive `&ServiceContext` with the frozen shape; `ctx.clock`, `ctx.rng`, `ctx.external.*` are all stable surfaces.
- **W3-G (source_asof / freshness):** Reads `ctx.clock` for deterministic freshness math. The `CURRENT_TIMESTAMP` audit list is shipped in v2 §3, so W3-G has a known-deferred-risk register. Evaluate-mode flakiness on DB-defaulted timestamps is a documented limitation, not a surprise.
- **W4-A (Trust Compiler):** Reads `ctx.clock` and `ctx.services.external.*` indirectly. Both are frozen. Suite-P regression budget (5% p99 deviation) gives W4-A a measurable comparand for its own performance work.
- **W4-C (invoke_ability bridge):** Consumes `PlannedMutationSet` from W3-A and `ProvenanceRef` from W3-B. v2's wave assignment ensures these exist before W4-C compiles. `TxCtx` shape (no external, no provider) enforces ADR-0104's transaction ban architecturally, so W4-C cannot accidentally compose a transaction that calls an LLM.
- **W5 (pilots):** Consumes Live-only construction. `new_live` is the single path; `new_evaluate` is fixture-DB-only with panic-guard; `test_live` is `#[cfg(test)]`-only. Pilot deployments cannot accidentally pick up a mode-aware shim.

No new architectural drift introduced by cycle-1 closures. The changes between v1 and v2 are additive specifications and a comprehensive mutation catalogue; no carrier was removed, no signature changed, no consumer-facing type was renamed. The freeze is reviewable, mechanically enforceable (catalog + runtime + lint + trybuild), and well-bounded by §10's narrow remaining-uncertainty list (DOS-304 interpretation + HRTB fallback).

## Verdict rationale

APPROVE. All nine cycle-1 findings are closed at the architectural level — eight fully, one (F9) acceptably re-scoped to the W2 invariant DOS-209 actually owns. The mutation catalogue in §3 is materially stronger than v1's prose and is the right artifact for a 24-module sweep. The seam shape (eight carriers, IntelligenceProvider excluded, concrete `ExternalClients`, `TxCtx` with no external/provider) is consumable by W3-A, W3-B, W3-G, W4-A, W4-C, and W5 without breaking-change risk. Migration discipline (one PR, no feature flag, no backward-compat default, structural lint + runtime + trybuild + direct-clock/RNG regex lint) addresses the highest-blast-radius operation in the project with the right combination of mechanical and runtime enforcement. Failure modes (HRTB slippage, capability leakage, missed guard) all have named fallbacks. No High or Critical fresh finding. L6 escalation is not warranted.

## If APPROVE

What gives confidence the architectural foundation is sound for downstream consumption:

1. **Carrier list is frozen and complete.** The eight-carrier `ServiceContext` (`db`, `signals`, `intel_queue`, `mode`, `clock`, `rng`, `external`, `tx`) plus the explicit non-carrier (`IntelligenceProvider` on `AbilityContext`) leaves no ambiguity for W3-A's registry composition or W4-C's bridge.
2. **Capability boundary is enforced by trybuild, not by convention.** `dos209_capability_trybuild.rs` compile-fail fixtures for raw `ActionDb`, raw SQL, live external client, and production `test_live()` make capability leakage a compile error, not a review-time observation. This is the DOS-304 enforcement boundary the plan claims, and it is mechanically realized.
3. **`TxCtx` shape architecturally enforces the ADR-0104 transaction ban.** No external clients, no provider, no LLM in transactions — by type, not by lint. W4-C cannot accidentally violate this.
4. **Mutation catalogue is checked-in and mechanically verified.** `dos209_mutation_catalog.rs` structural lint plus `evaluate_catalog_public_mutators_return_write_blocked_by_mode` runtime coverage means a missed migration is a CI failure, not a runtime surprise. The ~210-row catalogue in §3 is the human-readable artifact reviewers can scan.
5. **Performance has a falsifiable budget.** 5% p99 deviation against the W1 Suite P baseline on the named hot-path mutators is the W2-end re-measurement gate. "No regression" is now measurable.
6. **Fallback paths are named and bounded.** HRTB fallback is sync-within-async with no `.await` in the transaction body; capability-leakage fallback is the trybuild compile-fail; missed-guard fallback is the structural lint plus the Evaluate runtime test. Failure modes are not open-ended.
7. **Wave coordination is explicit.** W2-B-first merge order with the cross-file guard-preservation protocol prevents the most likely integration surprise. W3 wave assignments for `PlannedMutationSet` / `ProvenanceRef` / `plan_*` prevent type duplication or compile-time discovery.
8. **Known limitations are documented, not silent.** `CURRENT_TIMESTAMP` audit list is shipped, follow-on is committed, W3-G and W4-A can reference. Evaluate-mode determinism partial-coverage is a known-deferred risk, not an unknown one.

The architectural foundation is sound for downstream consumption. Plan is frozen for coding.
