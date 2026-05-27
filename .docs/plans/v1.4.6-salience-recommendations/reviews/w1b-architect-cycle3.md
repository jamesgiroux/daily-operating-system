PASS

Implementation can proceed. V0.3 resolves the prior blockers: the read ability is non-mutating, persistent recompute is split into an internal service path with `ctx.check_mutation_allowed()`, v270 DDL is idempotent/retry-safe, and MCP/SurfaceClient exposure is explicitly disabled.

Feasibility evidence:
- W1-A contracts and salience placeholder exist, including typed `SalienceScore`, ten `SalienceFactorKind` values, and typed `FactorRationale`.
- The runtime crate boundary is handled by mirrored DTOs plus app-side adapter parity tests.
- The proposed read-handle shape matches existing `claim_receipt`, `workspace_graph`, and `list_open_loops` patterns.
- v270 is the next executable migration slot after current v269.
- Factor mapping consumes existing claim, feedback, corroboration, contradiction, canonicalization, trust/freshness, action/open-loop substrate instead of creating parallel primitives.

Non-blocking risks:
- DTO mirror drift between app and `abilities-runtime`; golden JSON parity tests are required.
- Novelty must not read or store `claim_semantic_evidence.original_text`.
- Actor policy must stay derived from `AbilityContext`, never from request fields.
- v271-v272 are safe only as same-lane repair slots before any higher migration lands.