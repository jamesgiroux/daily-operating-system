# DOS-461 Reuse Audit

AC-461.8 — list existing abilities/services/DTOs/tests/UI primitives reused
by the entity fixture harness, per CLAUDE.md DoD #7 style and memory
`feedback_check_substrate_before_authoring_primitives`.

## Reused — substrate that already exists

| Surface | Path | What we reuse |
|---|---|---|
| `EntityIntelligenceEnvelope` DTO | `abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs` | Full envelope shape (subject, sections, facts, open_loops, touchpoints, threads, record_entries, metadata_proposals, trust, provenance). Fixtures serialize/deserialize through this type — the harness does NOT define a parallel shape. |
| `get_entity_intelligence` ability | `abilities-runtime/src/abilities/get_entity_intelligence/mod.rs` | The harness names this ability as the ONLY allowed source of rendered claim-backed intelligence in `BYPASS_DENYLIST` (the renderer source MUST mention `get_entity_intelligence`; if it mentions any legacy reader it fails). |
| `OpenLoop` / `OpenLoopSubject` | `abilities-runtime/src/abilities/list_open_loops/mod.rs` | Open-loop fixtures embed real `OpenLoop` instances — no shadow struct. |
| `SubjectRef` | `abilities-runtime/src/abilities/provenance/subject.rs` | Subject identity in all fixtures. |
| `TrustBand` | `abilities-runtime/src/abilities/trust/types.rs` | Trust band markers on facts / open loops / touchpoints. |
| `ClaimSensitivity` / `ClaimState` / `SurfacingState` | `abilities-runtime/src/types.rs` | Lifecycle + sensitivity enums on facts. |
| `RenderableClaimText` / `RenderPolicy` / `RedactionAffordance` | `abilities-runtime/src/sensitivity.rs` | Rendered-text shape on facts and record entries; confidential/user-only fixture uses the `Redacted` policy + `ConfidentialHidden` affordance directly. |
| `ClaimVerificationState` | `abilities-runtime/src/sensitivity.rs` | Verification state on facts. |
| `RenderSurface::TauriEntityDetail` | `abilities-runtime/src/sensitivity.rs` | The render surface every fixture targets (W1 substrate; W2 block renderers will assert against the same harness using their own surface variants). |

## Reused — test infrastructure

| Pattern | Where it came from | Use |
|---|---|---|
| Top-level `tests/{name}.rs` + `#[path]` subdir-module pattern | `src-tauri/tests/bundle16_ambiguous_identity_substrate_test.rs` + `src-tauri/tests/harness/mod.rs` | Same shape: integration test entry file pulls in a `mod.rs` from a sibling directory. |
| Fixture JSON-on-disk pattern with deterministic regen | `src-tauri/tests/fixtures/bundle-*` (each bundle ships `inputs.json`, `expected_output.json`, etc.) | Adopted (one envelope JSON per fixture file). |
| Synthetic anonymized fixture content | `src-tauri/tests/dos216_fixture_anonymization_lint_test.rs` ("synthetic" anonymization-cert pattern) | All fixture content is generic synthetic — domains like `acct-zero`, `project-zero`, `person-zero`; no real customer references. |

## NOT reused — net-new code in this ticket

| Surface | Why net-new |
|---|---|
| `dom.rs` minimal HTML walker | No existing HTML walker in `src-tauri/`. The repo has no `scraper` / `html5ever` dependency; pulling one in for a 250-LOC tokenizer would be heavier than warranted. The walker is scoped to two operations (text-with-ancestor-claim-id, list of claim-id attribute values). |
| `assertions.rs` no-bypass + binding + stale-vs-bypass library | This is the gate primitive DOS-461 was filed to add — by definition net-new. |
| `matrix.rs` per-subject expected-fixture table | New per AC-461.5b (correctness F7). |
| `fixture_builder.rs` programmatic builders | New. Builders compose against the substrate types listed above; they do not introduce parallel DTOs. |

## Grep confirmation that no existing harness covers this

```
$ rg -l "data-claim-id" src-tauri/tests
# (empty)

$ rg -l "bypass" src-tauri/tests | head -3
# bundle3_stale_source_resurrection_substrate_test.rs (different bypass class — stale-source)
# bundle7_temporal_scope_violation_substrate_test.rs (different bypass class — temporal scope)
```

No existing test covers the **render-source no-bypass** class (renderer text-source mentions legacy non-envelope reader). The closest prior art is the stale-source-resurrection bundle, which is a substrate-side test of claim lifecycle and does not exercise rendering surfaces. DOS-461 fills the rendering-side gate.
