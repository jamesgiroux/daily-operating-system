# DOS-546 W1-E L2 Architecture Review (Cumulative)

**Scope:** commits `e9596c49` + `9d09c1c3` + workflow step in `57a57e1f`.
**Surface under review:** `src-tauri/abilities-runtime/src/abilities/composition.rs`, `scripts/check_composition_authorship.sh`, `.github/workflows/test.yml` step "Enforce ADR-0130 substrate-owned composition authorship".

## Verdict: APPROVE

## 1. Type fidelity to ADR-0130 §2

Match. `Composition` carries `id, kind, subject, sections, salience, generated_at, generated_by, metadata`; `Section` carries `id, label, blocks, layout, salience`; `Block` carries `id, block_type, attributes, claim_refs, provenance, salience, render_hints`. `RenderHints` (emphasis + density) and `Density { Compact, Comfortable, Spacious }` are surface-neutral; `CompositionMetadata` exposes `schema_version`, `generated_at`, monotonic `composition_version`, `generated_by`. `BlockType` taxonomy covers artifact 05's canonical eight plus the `Custom { type_id }` extension point. `CompositionKind` is closed-with-extension via `Custom`. Newtype hygiene (`CompositionDocId`, `SectionId`, `BlockId`, `EntityRef`, `AbilityRef`) is appropriate and `#[serde(transparent)]` keeps wire shape ergonomic. `CompositionVersion::bump` uses `saturating_add` — safe even under unrealistic overflow.

Minor: `Composition.generated_at` is duplicated against `metadata.generated_at` and `generated_by` is duplicated against `metadata.generated_by`. This mirrors ADR-0130 §2's prose verbatim but is genuinely redundant on the wire. Not a blocker — ADR-0130 §2 lists both at the same level. File as path-α maintenance follow-up only if it surfaces in renderer code.

## 2. Lives-once invariant (ADR-0102 §6 + ADR-0105 §8)

Preserved. `Block.provenance: ProvenanceRef` is `{ invocation_id, field_path }` — 80-200 byte typical, 256-byte ceiling asserted in `provenance_ref_stays_compact`. No `Provenance` envelope is embedded in any block, section, or composition node. Renderers resolve a `ProvenanceRef` by fetching the canonical envelope (which lives once on the `AbilityOutput<Composition>` wrapper) and reading the `FieldAttribution` at `field_path`. The 64KB ADR-0108 envelope cap cannot be multiplicatively exploded by composition output because each block contributes O(invocation_id + pointer string), not O(envelope).

`Block::new` enforces two hard invariants at construction: (a) `invocation_id != nil`, and (b) `field_path` resolves into the canonical envelope's `field_attributions`, either exact-match or via a parent path that `covers()` it. The `covers()` fallback follows ADR-0130 §2 "Resolution. ... fallback to invocation-level provenance is labeled as less specific" — the construction-time gate still rejects paths that don't resolve at all. Tests `block_construction_rejects_nil_invocation`, `block_construction_validates_field_path_when_envelope_provided`, and `block_construction_rejects_field_path_outside_envelope` cover OK + both reject paths against a real `Provenance` fixture (not just deferred-None).

Deferred validation (`output_provenance: Option<&Provenance>`) is correct for builders that assemble blocks before sealing the envelope; the contract is documented and `validate_against` is exposed for the caller to honor. No silent bypass — the caller is on the hook.

## 3. Substrate-owned authorship (ADR-0130 §1)

Two-layer enforcement is architecturally sound:

- **Primary (Rust):** `Composition::new` is `pub(crate)`. Cross-crate consumers cannot construct via the constructor. Combined with the struct-field privacy that `pub(crate)` *doesn't* give (struct literal `Composition { ... }` is still buildable cross-crate if fields are `pub`), this is necessary but not sufficient.
- **Defense-in-depth (CI):** `scripts/check_composition_authorship.sh` ripgreps `Composition::new(` and `\bComposition\s*\{` across the workspace, excluding `src-tauri/abilities-runtime/**`, generated artifacts, archive, and markdown. Excluding markdown is correct — ADRs legitimately quote the type name. The script is wired into `.github/workflows/test.yml` as a required step.

This matches the canonical pattern (visibility primary, grep fence secondary) used by other v1.4.x trust-boundary gates (`check_ability_descriptions.sh`, `check_ability_inventory.sh`). The struct literal scan is the load-bearing piece because `Composition`'s fields are `pub`; without it, `pub(crate) fn new` would be bypassable. Both shapes are caught.

One observation: the regex `\bComposition\s*\{` may produce false positives on identifiers like `CompositionMetadata { ... }`, `CompositionKind::Custom { type_id: ... }`, `CompositionDocId { ... }` if they ever appear in non-substrate code. In the current tree there are zero non-substrate construction sites of any of those, so the gate passes; if Wave 2-5 surfaces ever need to construct `CompositionMetadata` directly outside the substrate (unlikely — they receive sealed `Composition`), the script can be tightened to require `Composition\s*\{` with a leading non-identifier byte AND a lookahead that the next token isn't an alphanumeric continuation of `Composition`. Path-α maintenance candidate, not a blocker.

## 4. Fallback projection (ADR-0130 §3.1 + artifact 07)

Deterministic and bounded. The 9-step algorithm is implemented as:

1. `select_nearest_known_type` scores candidates on (kind_match=100, required_overlap×10, optional_overlap×2, annotation_similarity capped at 20, namespace_similarity=5). Total used as primary sort key.
2. Tie-break ordering: total desc → kind_match desc → required_overlap desc → optional_overlap desc → annotation_similarity desc → **lexicographic `type_id` asc**. The lex tie-break is the load-bearing determinism gate; `projection_is_deterministic_across_runs` asserts order-independence by running both candidate orderings and checking equality. `projection_tie_break_is_lexicographic` directly asserts the lex semantics.
3. Intersected pointer set = unknown's required ∪ optional, filtered to nearest's required ∪ optional. Empty intersection → `generic_text_fallback` to `dailyos/text`.
4. `project_pointers` reconstructs container objects to hold only allowed leaves; siblings are NEVER copied wholesale. `projection_is_deterministic_across_runs` asserts `secret_email` (a payload field outside the intersected set) does not leak into the projected attributes — the privacy boundary holds.
5. `claim_refs` and `provenance` are preserved exactly (asserted in both projection tests).
6. `TrustBandCap::NeedsVerification` is the only variant; fallback cannot upgrade trust. Correct per ADR-0130 §3.1 step 9.
7. `FallbackBanner { text, dismissible: false }` is non-dismissible. Banner text uses product vocabulary ("payload may be incomplete"). The banner is structurally a non-dismissible flag — the renderer (Wave 4) is responsible for actually rendering it non-dismissibly; the substrate side cannot enforce that. Acceptable given the contract.
8. `ProjectionDiagnostic` carries projected/dropped pointer counts but **not** the dropped values — the operator visibility surface cannot become a privacy leak channel. `reason: &'static str` confines the diagnostic taxonomy to compile-time.

The pointer-escape and array-reconstruction gaps surfaced in the cycle-2 L2 (RFC 6901 escape semantics for `~0`/`~1`, and reconstruction of array-indexed pointer segments such as `/items/0/title`) are deferred to maintenance. This is the correct call under the path-α discipline: the projection contract is privacy-bounded (siblings never leak, trust cap applied, banner shown), the determinism contract holds, and the W1-E acceptance criteria are met. The escape + array-reconstruction gaps are theoretical hardening — they cannot cause incorrect render trust, only sub-optimal reconstruction shape for payloads that haven't been used yet. They belong in Codebase Maintenance & Production Quality (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).

## 5. `attributes: serde_json::Value` vs typed `BlockPayload` enum

Defensible. ADR-0130 §2 names a typed `BlockPayload` enum, but the substrate-side trade-off rationale documented in the module header is correct: the load-bearing typings for per-block-type payload schemas live in the W3 ability contracts and renderers (the producers of each `BlockType`). A substrate-side closed enum would couple this file to every ability's payload shape and force a substrate edit on every new ability, breaking the v1.4.x extensibility story. `serde_json::Value` is the most general shape that:

- works for every canonical `BlockType` and the `Custom { type_id }` extension point uniformly;
- lets the fallback projection operate at JSON-pointer granularity without needing typed-payload knowledge;
- preserves the option to migrate to a typed enum in a future wave when the payload-schema producers stabilize.

The migration path is straightforward (introduce `BlockPayload` as a wrapper, add per-variant `TryFrom<Value>` impls, retain `Value` as an escape hatch for `Custom`). Filing as v1.4.x+ maintenance candidate is correct; it is not a W1-E acceptance violation.

## 6. Cross-ADR coherence

- **ADR-0102 §6 (provenance lives once):** preserved via `ProvenanceRef`. No copy paths.
- **ADR-0102 §7.1 (`mcp_exposure` / `client_side_executable`):** not touched by W1-E.
- **ADR-0105 §8 (lives-once invariant on `FieldAttribution`):** preserved. Blocks never own `FieldAttribution`; they own a `FieldPath` reference into the envelope's `field_attributions` map.
- **ADR-0108 (64KB envelope cap, actor-filtered rendering):** preserved. The cap applies to the envelope once, not once-per-block; the renderer-side actor filter is unaffected.
- **ADR-0125 (claim substrate):** `ClaimRef { claim_id, claim_version }` is the substrate reference shape, with `claim_version` optional. Preserved exactly across fallback projection (artifact 06).
- **ADR-0129 (composition.produces_blocks ability category):** not directly touched by W1-E — it's a W3 ability-side concern. The `Composition` type is the producer's output category. No conflict.
- **ADR-0130 §1 (substrate-owned authorship):** enforced via `pub(crate)` + grep gate (see §3 above).
- **ADR-0130 §2 (Composition / Section / Block primitives):** fields match.
- **ADR-0130 §3.1 (custom block fallback projection):** algorithm implemented; privacy boundary, determinism, trust cap, banner all correct.

No ADR invariant is broken by this substrate landing.

## Path-α maintenance candidates (NOT blocking)

These are theoretical hardening or contract-evolution items, not W1-E acceptance violations. File to the Codebase Maintenance & Production Quality project:

1. **Pointer-escape semantics in `project_pointers`** — RFC 6901 `~0`/`~1` handling for keys containing `/` or `~`. Today's `split('/')` will mis-segment such keys.
2. **Array reconstruction in `insert_at_path`** — pointer segments that index arrays (`/items/0/title`) currently rebuild as object keys `"0"` rather than array index 0. Faithful reconstruction would parse numeric segments and rebuild `Value::Array`.
3. **Typed `BlockPayload` enum migration** — when per-block-type payload schemas stabilize across Wave 3-5 abilities, promote `attributes: Value` to a typed sum type.
4. **`Composition.generated_at` / `generated_by` duplication with `metadata.*`** — ADR-0130 §2 names both, but the on-wire duplication is genuine. Either rationalize the ADR or alias the metadata fields.
5. **Grep gate identifier-prefix false-positive guard** — tighten `\bComposition\s*\{` to disambiguate from `CompositionMetadata { ... }` etc. if such constructions ever appear outside the substrate.

## Summary

The W1-E substrate types match ADR-0130 §2 verbatim, preserve the ADR-0102/105/108 lives-once invariant via `ProvenanceRef`, enforce substrate-owned authorship through the canonical two-layer (Rust visibility + CI grep) pattern, and implement a deterministic, privacy-bounded fallback projection with explicit trust cap and non-dismissible banner. The `serde_json::Value` attribute shape is a defensible substrate-side choice. The cycle-2 path-α gaps in pointer escape and array reconstruction are correctly transferred to maintenance per CLAUDE.md path-α discipline. No cross-ADR invariants are broken.

**APPROVE.** Proceed to PR.
