# L0 Cycle 1 — W1-A DOS-463 — architect-reviewer

**Date:** 2026-05-20
**Reviewer:** compound-engineering:ce-architecture-strategist (substrate/schema domain per ladder matrix)
**Verdict:** **APPROVE** (5 doc-comment nits, none blocking)

## Substrate verifications

- `provenance/source.rs:81` → `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` — exact match.
- `provenance/source.rs:201` → `WorkspaceFileKind` enum, all 7 variants matching packet's `SourceType` mirror.
- `claims.rs:130` → `ClaimType` enum (cycle 7 correction validated; not `ClaimKind`).
- `provenance/subject.rs:8` → `SubjectRef` variant-tuple shape (`Account(String)`).
- `services/claim_receipt/{mod,contracts}.rs` precedent exists — submodule + `contracts.rs` is a consumption pattern, not novel.
- v178 highest dev migration → v200/v201 has clean 22-slot buffer.

## Findings (non-blocking)

**F1 (low)** — Placeholder file list (8 names) cleanly maps to 6 downstream lanes. Handoff is clean; nobody touches `mod.rs` post-W1-A.

**F2 (low, schema)** — `lifecycle_state TEXT` not CHECK-constrained; relies on the Rust enum at write time + `services::workspace_ingestion` mutation-allowlist CI gate (W2-A merge). Matches `intelligence_claims.data_source` precedent. Acceptable; CHECK constraint optional path-α maintenance for v1.x.y.

**F3 (low, schema)** — `data_source TEXT` JSON-serialized is right for non-flat enum (`Glean { downstream }`, `WorkspaceFile { kind }`). Matches `intelligence_claims.data_source` precedent.

**F4 (medium, SubjectRef duality)** — Duality verified: abilities-runtime has `Account(String)`/…/`User(String)`/`Unknown`; db-layer has struct-variant + `Email` and lacks `User`/`Unknown`. Material refactor outside W1-A scope. Packet mitigation (explicit `use abilities_runtime::abilities::provenance::subject::SubjectRef;` + file maintenance ticket) is sufficient. **Nit:** add a single-line `// Reconciliation tracked in <maintenance-ticket>` comment next to the `use`.

**F5 (low, downstream coupling)** — `SignalEmitter` trait's 5 methods must stay 1:1 with W3-B's 5 `SignalType::WorkspaceFile*` variants. **Nit:** add a one-line comment on the trait in `contracts.rs`: "Each method maps 1:1 to a `SignalType::WorkspaceFile*` variant in W3-B's `signals/policy_registry.rs`."

**F6 (low, downstream-author ambiguity)** — `subject_ref` entity-id namespace not pinned. **Nit:** doc-comment "subject_ref entity ids are the canonical v1.4.0 entity-slug format (consumed by `services::claims::commit_claim`)."

## Architecture compliance

- SOLID/DIP via `Extractor`/`SignalEmitter` traits + Null defaults — clean DI substrate.
- W1-A consumes ADR-0107 + ADR-0125 without amendment; emits no signals; commits no claims. Boundary honest.
- Cycle 4+5 fix (trait + skeleton ownership) load-bearing and correctly executed. `mod.rs` ownership rule testable per §8.

## Verdict

**APPROVE.** Findings F4–F6 are doc-comment nits the implementing agent can fold in without re-cycling. Plan is gate-clean.
