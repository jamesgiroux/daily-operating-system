# DOS-546 W1-E + W1-E.1 + workflow integration — L2 (Diff) code-reviewer verdict

**Date:** 2026-05-11
**Reviewer lane:** code-reviewer (L2 Diff)
**Commits:** `e9596c49` (W1-E composition types) + `9d09c1c3` (W1-E.1 widen + authorship gate) + `57a57e1f` (workflow integration)
**Scope of file changes:** `src-tauri/abilities-runtime/src/abilities/composition.rs` (1363L), `scripts/check_composition_authorship.sh`, `.github/workflows/test.yml`

## Verdict: APPROVE

L1 reproduced locally: `cargo test -p abilities-runtime --lib composition` → 22/22 pass; `scripts/check_composition_authorship.sh` exit 0; gate wired at `.github/workflows/test.yml` line 94 alongside W1-C (line 78) and W1-D (line 88).

## AC-bounded assessment

1. **Types (Composition / Section / Block / ProvenanceRef / ClaimRef / BlockType / CompositionKind / Salience / RenderHints).** Widened verbatim to ADR-0130 §2. CompositionKind is internally-tagged with `Custom { type_id }` extension; BlockType mirrors that shape; SalienceBand, Density, SectionLayout all snake_case serde, defaults set, `#[serde(default)]` + `skip_serializing_if = "Option::is_none"` on additive fields so forward-compat deserialization holds. Newtypes (CompositionDocId, SectionId, BlockId, EntityRef, AbilityRef, CompositionVersion) all `#[serde(transparent)]` with consistent `new`/`as_str` shape. CompositionVersion uses `saturating_add` on `bump()` — correct.

2. **Authorship gate dual-layer (ADR-0130 §1).** Primary `Composition::new` visibility is `pub(crate)`. Defense-in-depth grep at `scripts/check_composition_authorship.sh` matches both `Composition::new(` and `Composition {` literal shapes across the workspace, excludes `src-tauri/abilities-runtime/**`, `target`, `node_modules`, `_archive`, `.git`, `*.md`, and itself. Both branches emit remediation guidance and exit 1 on any hit. Wired into `.github/workflows/test.yml` as step "Enforce ADR-0130 substrate-owned composition authorship." Gate is necessary because struct fields remain `pub` — `pub(crate) fn new` alone does not block cross-crate struct-literal construction. Correct layering.

3. **ProvenanceRef preserves ADR-0105 §8 lives-once.** Shape is `{ invocation_id: InvocationId, field_path: FieldPath }`. No embedded `Provenance` copy. Renderers resolve refs against canonical `AbilityOutput<Composition>.provenance` envelope per the documented contract. Compactness asserted by `provenance_ref_stays_compact` test at ≤256 bytes.

4. **`project_to_nearest_known` deterministic algorithm.** Scoring weights match Phase 0 artifact 07 (kind 100, required 10/ptr, optional 2/ptr, annotation min(4×count, 20), namespace 5). Sort is total desc → kind desc → required desc → optional desc → annotation desc → lex `type_id` asc. Zero-score winner falls back to `dailyos/text` generic. `intersected` empty → same fallback. `project_pointers` rebuilds containers, never copies siblings wholesale. `claim_refs` and `provenance` cloned from unknown verbatim. `TrustBandCap::NeedsVerification` capped. `FallbackBanner.dismissible = false`. Banner text uses product vocabulary ("payload may be incomplete"). `projection_tie_break_is_lexicographic` verifies determinism on equal scores. Path-α gaps (`~0`/`~1` JSON-Pointer escape, array-index container reconstruction, BlockDescriptor `schema_shape`/`allowed_surfaces`/`actor_reachability`) documented as deferred to maintenance — acceptable per "L2 path-α findings → maintenance project."

5. **Block-builder validation.** `Block::new` rejects nil InvocationId, rejects InvocationId mismatch against supplied envelope, accepts exact path match or parent `covers()` match, returns `UnresolvedFieldPath { field_path }` otherwise. `validate_against` exposed for deferred-validation path. `fixture_envelope_with_attribution` exercises both OK and reject paths against a real `Provenance` rather than `None` — fixes the previously-no-op test flagged in the commit body.

6. **Test coverage.** 22/22 pass. Coverage spans: serde roundtrip, ProvenanceRef compactness, nil-invocation rejection, envelope-validated OK + rejection paths, projection determinism + secret-field non-leak + claim_refs/provenance preservation + trust cap + banner copy, generic-text fallback, fingerprint stability via `canonicalize` BTreeMap traversal, monotonic version, widened-fields roundtrip, section label+salience roundtrip, render_hints default+roundtrip, CompositionKind::Custom roundtrip, lexicographic tie-break.

7. **`attributes: serde_json::Value` vs typed `BlockPayload`.** Documented in module header as a strict superset of AC line 660's `BTreeMap<String, Value>`, with the typed `BlockPayload` enum filed as maintenance. Defensible: per-block-type payload schemas land in W3 ability contracts and renderers, where the substrate's general shape lets each block type evolve without churning composition.rs. Aligns with "don't swing past center" — substrate stays general, surface contracts carry the typing.

## Path-α observations (not blocking; file as maintenance if not already)

- **Duplicate `generated_by`.** Top-level `Composition.generated_by: AbilityRef` (W1-E.1 widening) and `CompositionMetadata.generated_by: String` (W1-E original) carry the same datum in two places. Producers must keep them in sync manually; nothing in the type system enforces it. Either drop the metadata copy, narrow it to `AbilityRef`, or document it as a denormalized cache with a builder that sets both atomically.
- **`#[allow(dead_code)]` on `Composition::new`.** Real producer wiring is W3; the allow should drop the moment the first ability constructs a composition. Track as a follow-up so the dead-code suppression doesn't outlive its rationale.
- **`insert_at_path` silently overwrites leaf-at-intermediate-path.** Comment notes "Conflict: existing leaf at intermediate path. Overwrite with a fresh container — projection is target-shape driven." Behavior is correct for projection semantics, but the diagnostic counts (`projected_pointer_count` / `dropped_pointer_count`) do not record the overwrite. Low-likelihood scenario — required and optional pointer sets shouldn't overlap as leaf-vs-container in a well-formed schema — but worth surfacing in `ProjectionDiagnostic` if it ever fires.
- **Banner copy injects `type_id` verbatim into user-visible string.** `format!("Rendered as {selected_type_id} — payload may be incomplete")`. Internal-style type ids (`account_overview`, `dailyos/text`) leak through to the surface. If banners are user-facing, this needs a `display_name` mapping per ADR-0083 product-vocabulary discipline; if banners are operator-only or wrapped by surface renderers before display, ignore.

## L2 path-α policy compliance

None of the path-α observations are literal AC violations, ADR-named contract violations, or W1-E-introduced regressions. They are class-of-issue maintenance items appropriate for `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` ("Codebase Maintenance & Production Quality"). Substrate PR unblocked.

## Workflow integration check

`.github/workflows/test.yml` carries all three Wave 1 gates in the expected order. The three steps run sequentially after the durable-source-comment lint; any drift in non-substrate paths breaks CI before reaching cargo test. Structural enforcement parity with the AC line 666 lint requirement is satisfied.

---

**code-reviewer verdict: APPROVE.** Path-α items above to `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` if not already filed.
