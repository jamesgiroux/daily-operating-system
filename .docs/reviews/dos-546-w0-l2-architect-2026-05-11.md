---
reviewer: architect-reviewer
lane: L2 (Diff)
ticket: DOS-546
scope: W0-A / W0-C / W0-D (substrate-contract amendments, doc-only)
date: 2026-05-11
verdict: APPROVE
---

# L2 architect review — W0 substrate-contract amendments

## Scope under review

Doc-only diff:

- `.docs/decisions/0102-abilities-as-runtime-contract.md` — W0-D §7.1 tri-state + orthogonal `client_side_executable`; §7.4 + §7.6 introspection ripple; header amendment line.
- `.docs/decisions/0111-surface-independent-ability-invocation.md` — W0-D §8 cross-reference to the tri-state and to `client_side_executable`.
- `.docs/decisions/0130-surface-independent-composition-contract.md` — W0-A §2 `ProvenanceRef` + §3.1 nine-step JSON-pointer fallback + §1 consumed-primitives subsection.
- `.docs/plans/dos-546/phase-0/INDEX.md` — W0-C 15-row contract map.
- 15 phase-0 artifacts — W0-C `status: spec:ready` promotion.

## Acceptance verification

### W0-A — ADR-0130 amendments

- §2 `Block.provenance: ProvenanceRef` confirmed (lines 70-83). Shape is `{ invocation_id: InvocationId, field_path: FieldPath }` per artifact 06. The doc-comment correctly cites ADR-0102 §6 and ADR-0105 §8 for the lives-once invariant.
- §2 Resolution paragraph (line 97) routes through `render_provenance_for(prov, actor, surface)` per ADR-0108 §2 — actor/surface filtering is not bypassed.
- §2 explicit `Why ProvenanceRef, not ProvenanceEnvelope` rationale (line 95) cites both ADR-0102 §6 + §9 Rule 5 and ADR-0108's 64KB cap with a quantitative example (288KB envelope-copy vs 13KB ref form).
- §2 Composition-size guard (line 101) preserves 48KB headroom under the 64KB cap.
- §3.1 fallback ships exactly nine numbered steps (verified by grep) matching artifact 07: deterministic candidate selection with lexicographic tie-break, JSON-pointer intersection with safe-widening rules, drop-non-intersected, preserve `claim_refs` + `provenance`, `needs_verification` cap, non-dismissible product-vocabulary banner. Generic `dailyos/text` no-candidate fallback covered.
- §1 "Consumed substrate primitives" subsection (line 33) cites ADR-0074 (vector), 0078 (embed model), 0080 (signals), 0102 (abilities runtime), 0111 (invocation) — substrate-reuse fence per the consumed-primitives discipline.
- Status remains "Proposed" per W0-A scope limit.

### W0-D — ADR-0102 + ADR-0111 amendments

- §7.1 `mcp_exposure: McpExposure` tri-state confirmed (lines 222-235). `McpExposure { None | MetadataOnly | Invocable }` enum declared inline. Default is `McpExposure::None`.
- `client_side_executable: bool` is a separate field (line 228), default `false`, per artifact 05 lines 383-412. Acceptance criterion to "retain as separate field" is honored — not collapsed.
- §7.4 introspection rule rewritten to filter `mcp_exposure::None` and to enumerate `MetadataOnly` as schema-without-handler. First-party Tauri bridge explicitly carved out from both new fields.
- §7.6 multi-level enforcement enumerates four ordered checks: `allowed_actors` membership, `required_scopes` coverage, MCP-mediated `mcp_exposure` tier check, client-side-JS `client_side_executable` check. Each check rejects from the substrate boundary (registry or SurfaceClientBridge), not from prose. Default policy bullet matches W0-D acceptance: `[User]` + `vec![]` + `McpExposure::None` + `false`.
- Macro compile-error gate (artifact 05 §"opt-in") preserved: `SurfaceClient` actor membership triggers the gate regardless of `client_side_executable` value — the trigger is reachability, not client-side path.
- ADR-0111 §8 cross-reference updated (line 222 of 0111) — bool form replaced with tri-state + the orthogonal `client_side_executable` field; §8 actor-attribution semantics unchanged.

### W0-C — Phase-0 INDEX + status promotion

- INDEX.md has 15 rows (01-15) plus the index frontmatter row, in artifact order, with columns: file, owning sub-contract, consumed by (wave letter), status, open questions. All 15 artifacts are `spec:ready`.
- All 15 artifacts carry `status: spec:ready` frontmatter (verified via grep across the directory).
- All 15 artifacts cited at least once in `02-issues.md` (per-artifact citation counts: 01=7, 02=8, 03=16, 04=5, 05=22, 06=5, 07=15, 08=10, 09=9, 10=5, 11=6, 12=24, 13=8, 14=11, 15=9). No orphans.
- INDEX.md "How to use" section gives L2 reviewers and `/plan-eng-review` clear consumption rules; "Coverage cross-check" footnote provides the audit grep.

### Substrate-coherence + acyclic amendment chain

- ADR-0102 ↔ 0130: ADR-0130 §2 cites 0102 §6 + §9 Rule 5 for lives-once. ADR-0102 amendments do not touch §6 / §9 / §10 / §11 — composition contract preserved.
- ADR-0105 ↔ 0130: §2 explicitly mirrors 0105 §8's planned-mutation reference pattern. No amendment to 0105.
- ADR-0108 ↔ 0130: 64KB cap is referenced as a constraint, not modified. Renderer applies `render_provenance_for(...)` per 0108 §2.
- ADR-0111 ↔ 0102: 0111 §8 forward-references 0102 §7.6's tri-state and `client_side_executable`; 0102 §7.6 in turn back-references 0111 §8 for `SurfaceClient` actor class. The cycle is by-design (both ADRs co-evolved 2026-05-10) and not a violation — both ADRs are in the same wave, landing together with consistent text.
- ADR-0125 sensitivity tiers referenced from 0130 §3.1 (unknown-payload privacy rationale) without modifying 0125.
- Wave-protocol authority: amendments land in the contract ADRs they amend (not in consumer plans). Matches the protocol-amendments-in-protocol-doc discipline.

### Divergence from L0 spec

None material. Minor cosmetic observations (not blockers, not regressions):

- ADR-0102 line 508 (pre-existing) labels ADR-0108 as "Surface-Independent Ability Invocation"; that title belongs to ADR-0111. Pre-existing in the file, not introduced by W0-D. File as Path-α to the maintenance project.
- ADR-0130 §2 prose includes a probabilistic `field_path` fallback ("the surface MAY fall back to invocation-level provenance, but MUST label the fallback as less specific"). Artifact 06 covers this; the prose is consistent. No action.

## Verdict

APPROVE. The diff implements the W0-A / W0-C / W0-D acceptance criteria literally and preserves the invariants of ADRs 0102/0105/0108/0111/0125/0130 that it does not touch. Downstream Wave 1 issues (W1-B canonical AbilityPolicy schema, W1-D ability-surface inventory generator, W1-F Composition types) can consume these contracts as authoritative.

Recommended follow-up (non-blocking, file in the Codebase Maintenance & Production Quality project, id `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):

- Fix ADR-0102 line 508's mis-labeled forward reference to ADR-0108. One-line correction.
