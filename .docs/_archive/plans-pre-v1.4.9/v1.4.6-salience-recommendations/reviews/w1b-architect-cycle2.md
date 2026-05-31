PASS

V0.2 is ready for implementation against the current tree.

Key checks:
- W1-A recommendation contracts and `services::recommendations::salience` placeholder are present.
- Crate boundary issue is handled correctly by mirroring salience DTOs in `abilities-runtime` and converting in the app adapter.
- The `ServiceContext` read-handle pattern matches existing `claim_receipt`, `workspace_graph`, and `list_open_loops` seams.
- Current migrations stop at v269, so v270 is the correct next slot. v271-v272 are safe only as same-lane repair slots before any higher migration lands.
- Factor extraction is feasible using existing durable tables/helpers: `intelligence_claims`, `claim_corroborations`, `claim_contradictions`, `claim_feedback`, `canonicalization_decisions`, `claim_semantic_evidence`, signal weights/events, and trust/freshness helpers.

Implementation notes, not blockers:
- Do not put `mutates = []` in the `#[ability(...)]` macro; the macro derives `descriptor.mutates`. Assert it is empty in tests instead.
- Make v270 DDL idempotent with `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`, matching recent migration style.
- Novelty must read durable score/count fields only, not `claim_semantic_evidence.original_text`.