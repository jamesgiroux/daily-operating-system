# Wave W3 Proof Bundle (W3-A + W3-B substrate co-land)

**Wave:** W3 (substrate primitives — DOS-210 ability registry + DOS-211 provenance envelope)
**Status:** W3-A + W3-B complete (the rest of W3 — DOS-7, DOS-294, DOS-296, DOS-299, DOS-300, DOS-301 — remain ahead)
**Date:** 2026-05-01

---

## DOS-211 (W3-B) — Provenance envelope + builder

### Final commit chain (local-only, never pushed at proof-bundle time)

| Commit | Cycle | What |
|---|---|---|
| `17947c00` | docs | Plan v2 — L0 cycle-1 revision (5 findings closed: thread_ids, children wire, finalize timing, deferred trust variants, AbilityOutput ownership) |
| `d70af6cc` | initial | Provenance substrate — 8 module files (envelope/builder/trust/source/field/subject + mod) + 23 unit tests + 3 integration tests + lib.rs wire-up + schemars non-optional |

### Acceptance criteria validation

Per DOS-211 ticket + ADR-0105 + ADR-0124:

- [x] `Provenance` envelope per ADR-0105 §24-58 — verified `envelope.rs`
- [x] `TrustAssessment` shape per ADR-0105 §106-158 — verified `trust.rs`
- [x] `SourceAttribution` per ADR-0105 §165-195 — verified `source.rs`
- [x] `FieldAttribution` per ADR-0105 §206-241 — verified `field.rs`
- [x] Composition merge per ADR-0105 §257-266 — `ComposedProvenance` newtype with transparent serde, JSON wire matches `Vec<Provenance>` verbatim
- [x] `ProvenanceBuilder::finalize()` fails for missing field attribution — verified by `dos211_finalize_unattributed_field_fails_test`
- [x] Trust computed not author-set — `pub fn` constructors block manual override
- [x] `contains_stored_synthesis` flag set via builder — verified by `compute_trust`
- [x] A→B→C composition preserves child/grandchild tree — verified by `composition_a_b_c_preserves_child_grandchild_tree`
- [x] JSON roundtrip preserves equality — verified by `json_roundtrip_preserves_equality`
- [x] JSON Schema generator works (non-optional schemars 0.8.22) — verified by `schemars_schema_for_provenance_is_valid_shape`
- [x] `thread_ids: Vec<ThreadId>` on Provenance NOW per ADR-0124 §37+§48 with `provenance_schema_version=1`, serde-default — verified by 3 thread_ids tests
- [x] LLM synthesis fields force Untrusted (closed by L2 fix) — verified by `dos211_llm_synthesis_trust_test`
- [x] AbilityOutput<T> cannot be constructed outside provenance module (closed by L2+L3 fix) — verified by `dos211_envelope_constructor_gate_test` + sibling-module trybuild fixture

### Deliberate scope boundaries

**Out of scope per plan §131:**
- `FeedbackEvent` and `SubjectFitGate` trust contribution variants — ADR-0105 §129 only authorizes direct source, composed child, and stored synthesis field. Any expansion requires an ADR amendment first.

**Deferred to follow-up tickets after L2/L3 review:**
- **DOS-350** (W3 follow-up) — Validate composition_id at finalize/registry-time, not deserialize-time. Current `ComposedProvenance` deserialize fabricates `composition_id` from `provenance.ability_name`, which can collide on roundtrip. Registry-time validation is the structural fix.
- **DOS-351** (W3 follow-up) — ProvenanceBuilder size-budget tombstone replacement + no-progress detection. Current `elide_child_at_path` clears sources/children/field_attributions/warnings but leaves `inputs_snapshot`, `subject`, etc. intact, allowing re-elision of same node without progress.

---

## DOS-210 (W3-A) — Ability registry + #[ability] proc macro

### Final commit chain (local-only, never pushed at proof-bundle time)

| Commit | Cycle | What |
|---|---|---|
| `17947c00` | docs | Plan v2 — L0 cycle-1 revision (7 findings closed: drift test, ADR-0120 observability, Amendment A errors, DOS-304 trybuild, workspace CI, schemars, AbilityOutput ownership) |
| `be51a31f` | initial | Three-part W3-A landing: workspace conversion + new abilities-macro crate + AST visitor + allowlist generation; AbilityRegistry + InvocationRecord + AbilityContext + cycle DFS; macro emission with tracing instrument + JSON wrapper + 5 trybuild fixtures + 3 integration tests + lint script |
| `1432ef13` | L2 cycle-1 fix | 4 findings closed: AbilityOutput field privacy + finalize() gate; populate inventory descriptors via slice-typed AbilityDescriptor; erased wrapper serializes full envelope; trust merge inspects field_attributions for LlmSynthesis |
| `3faae549` | L3 cycle-1 fix | 5 findings closed: tighten AbilityOutput visibility to `pub(in crate::abilities::provenance)` + sibling-module trybuild; expand DOS-304 lint regex; experimental cargo feature + cfg-gate macro emission + registry guard; async erased invocation refactor; AST visitor module-alias resolution |
| `2aa70ac5` | L3 cycle-2 partial | Finding 2 closed: experimental cfg now also gates inner_fn (was only gating descriptor + wrapper) |

### Acceptance criteria validation

Per DOS-210 ticket + ADR-0102 + ADR-0120 + DOS-304:

- [x] `#[ability]` proc macro compiles and generates correct metadata — verified by trybuild compile-pass fixtures
- [x] AbilityDescriptor + type-erased wrapper + JSON input/output schemas — verified by `dos210_macro_descriptor_completeness_test`
- [x] `AbilityRegistry` with typed `invoke_read/transform/publish/maintenance` — verified by 12 registry unit tests
- [x] Erased invocation via `invoke_by_name_json` — verified by `dos210_erased_invocation_envelope_test` + `dos210_erased_invocation_async_test`
- [x] `inventory::submit!` + registry self-check — verified by `registry_collects_inventory_descriptors`
- [x] Category classification via proc-macro AST — verified by trybuild compile-fail fixtures
- [x] Read/Transform with detected mutator fails to compile — verified by `read_ability_direct_mutation_fails`, `transform_ability_imported_mutation_alias_fails`, `read_ability_module_alias_fails`
- [x] Module-level alias resolution in MutationVisitor — verified by 2 new scoring unit tests + trybuild fixture (CAVEAT: only function-body and use-statements within same fn captured; module-scope siblings remain a residual; see DOS-349)
- [x] Registry rejects duplicate names at build time + startup — verified by `registry_rejects_duplicate_names_with_clear_error`
- [x] Registry rejects composition cycles via 3-color DFS — verified by `composition_graph_rejects_random_cycle`
- [x] Actor-filtered enumeration excludes maintenance/admin/experimental from Agent — verified by `registry_iter_for_agent_hides_maintenance_admin_and_experimental`
- [x] AbilityContext wraps ServiceContext with actor + tracer + confirmation — verified by registry tests + DOS-304 trybuild boundary fixtures
- [x] `experimental = true` waives category enforcement and gates registration under `#[cfg(feature = "experimental")]` — verified by `dos210_experimental_feature_gate_test`
- [x] Documentation generator produces deterministic markdown — verified by `documentation_generator_renders_stable_markdown`
- [x] Amendment A error semantics (hard error / soft degradation / hard success) — verified by `dos210_amendment_a_test` (2 tests)
- [x] Span instrumentation per ADR-0120 §34 with redaction — verified by `dos210_observability_span_fields_test`

### Deliberate scope boundaries

**Out of scope (deferred):**
- `experimental_*` trybuild fixtures (3 of the original 11 fixture list) — registration-related compile failures; the live `experimental` cargo feature gate covers the runtime contract today
- `duplicate_ability_name_link_fails`, `ability_input/output_must_deserialize_and_schema`, `ability_signature_must_return_ability_result`, `ability_signature_rejects_extra_handle` — 4 additional trybuild fixtures (the macro currently rejects these via `compile_error!`; the trybuild fixture set is shorter than originally listed)
- Property tests with random DAG/cycle generation — current cycle-DFS unit tests cover the algorithm; property-based fuzzing is a follow-up

**Deferred to follow-up tickets after L2/L3 review:**
- **DOS-349** (HIGH priority, scheduled for v1.5.x or future wave dedicated to ability runtime hardening) — **Move ability runtime into separate crate.** This is the structural fix for the residual porousness in module-scope alias detection (L3 cycle-2 finding 1) and the DOS-304 lint regex (L3 cycle-2 finding 3). Hard precondition: must complete BEFORE first DOS-218+ capability migration ships. Current single-crate proc-macro AST + grep enforcement is best-effort defense-in-depth; full enforcement requires the crate split.
- **DOS-352** (W3 follow-up) — Fixture-trace runtime drift gate for ability metadata. Today's `dos210_metadata_drift_test` is `#[ignore]`-marked because the runtime mechanism doesn't exist; AST visitor catches direct calls but not transitive helper-call drift.

### L2 + L3 review trail

**L2 cycle-1 (BLOCK, 8 findings):**
- 4 closed in commit `1432ef13`: AbilityOutput bypass, hollow descriptors, erased strips envelope, LLM trust
- 4 deferred to follow-up tickets (DOS-349, DOS-350, DOS-351, DOS-352)

**L3 cycle-1 (BLOCK, 5 findings):**
- All 5 closed in commit `3faae549`: visibility tightening + sibling trybuild, lint regex expansion, experimental feature gate, async erased invocation, module-alias AST resolution

**L3 cycle-2 (BLOCK, 3 findings):**
- 1 closed in commit `2aa70ac5`: experimental cfg gates inner_fn (cycle-1 missed this)
- 2 escalated to L6 (Option A ruling): module-scope alias bypass + DOS-304 lint alias miss are inherent single-crate limitations; structural fix is DOS-349 retargeted to v1.5.x or future wave

---

## Final L1 validation (HEAD = `2aa70ac5`)

```
$ cargo build --workspace --all-features          → clean
$ cargo build --workspace --features experimental → clean
$ cargo build --workspace                         → clean (no features)
$ cargo clippy --workspace --all-features --lib --bins -- -D warnings → clean
$ cargo test --lib                                → 1797 passed; 0 failed; 7 ignored
$ cargo test -p dailyos-abilities-macro --tests   → 6 trybuild fixtures + 12 scoring unit tests pass
$ cargo test --test dos210_*                      → 7 integration tests pass (1 ignored = drift fixture-trace deferred)
$ cargo test --test dos211_*                      → 6 integration tests pass
$ pnpm tsc --noEmit                               → clean
$ bash scripts/check_no_db_state_imports_in_abilities.sh   → pass
$ bash scripts/check_no_direct_clock_rng_in_abilities.sh   → pass
```

W3 substrate test counts:
- Library tests: +38 vs W2 close (1759 → 1797)
- Proc-macro crate: 12 scoring unit tests + 6 trybuild fixtures
- W3-A integration tests: 7 (metadata drift / observability span / Amendment A x2 / erased envelope / erased async / experimental feature gate / macro descriptor completeness)
- W3-B integration tests: 6 (thread_ids x3 / composition wire / finalize unattributed / envelope constructor gate / LLM synthesis trust)

## L6 rulings during this wave

- **2026-05-01 (L3 cycle-2):** Option A ruling — accept porous best-effort enforcement of module-scope alias detection + DOS-304 lint, file structural fix as DOS-349 retargeted to v1.5.x or future wave. Hard precondition: DOS-349 must complete before first DOS-218+ capability migration ships. W3 substrate ships with residuals documented; no production exposure today because no real ability code lives under `crate::abilities` yet.

## Status (W3-A + W3-B)

- [x] L0 cycle-1 review on plan v1 (REVISE; 7 + 5 findings closed in plan v2)
- [x] L1 self-validation
- [x] L2 cycle-1 codex adversarial review (BLOCK; 4 closed, 4 deferred to follow-ups)
- [x] L3 cycle-1 Wave-scoped adversarial review (BLOCK; 5 closed in cycle-1 fix)
- [x] L3 cycle-2 confirmation review (BLOCK; 1 closed, 2 escalated to L6 with Option A ruling)
- [x] DOS-349 retargeted to v1.5.x / future wave with hard precondition documented
- [x] DOS-350, DOS-351, DOS-352 filed as W3 follow-up tickets
- [x] Proof bundle (this)
- [ ] Retro (next)
- [ ] Tag `v1.4.0-w3-substrate-complete` (after retro)

## Remaining W3 work (NOT in this proof bundle)

The W3 wave includes 6 more tickets beyond the W3-A + W3-B substrate that this bundle covers:

- DOS-7 (claims commit + 9-mechanism backfill)
- DOS-294 (FeedbackEvent + claim_feedback)
- DOS-296 (longitudinal topic threading — populates `thread_ids` field this substrate defines)
- DOS-299 (source_asof population + freshness)
- DOS-300, DOS-301 (TBD — read live ticket bodies before drafting)

Their plan documents exist at `.docs/plans/wave-W3/DOS-{7,294,296,299,300,301}-plan.md`. They build on the W3-A + W3-B substrate and will produce their own per-ticket proof bundles when they ship.
