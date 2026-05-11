# v1.4.2 Wave 0 — Proof Bundle

**Wave:** W0 — Contract lock + supersession
**Date sealed:** 2026-05-11 (impl + L1 + L2 phases; PR + CI pending Linear topology)
**Branch:** `dos-546-wp-studio-spike` off `dev` @ 4ebec482
**Predecessor:** v1.4.1 (DOS-213 W2-A landed; W2-H DOS-295 merged; spike DOS-546 spec:ready)

---

## Issue map

| Issue | Title | Linear ID | Owner |
|---|---|---|---|
| W0-A | Adopt ADR-0130 amendments: ProvenanceRef + custom block fallback projection | _pending_ | Claude (impl complete) |
| W0-B | Supersede original v1.4.2 and park entity-intelligence scope for v1.4.3 | _pending_ | James (Linear-side) |
| W0-C | Promote Phase 0 artifacts to spec:ready and lift acceptance criteria into project-level fixture index | _pending_ | Claude (impl complete) |
| W0-D | ADR-0102 §7.1 amendment: promote `mcp_exposure` to tri-state enum and retain `client_side_executable` | _pending_ | Claude (impl complete) |

---

## Acceptance criteria — pass/fail per issue

### W0-A — ADR-0130 amendments

- ✅ `Block.provenance: ProvenanceRef` (not `ProvenanceEnvelope`), shape `{invocation_id, field_path}` per Phase 0 artifact 06 `.docs/decisions/0130-...md` §2
- ✅ Custom block fallback projection rules from Phase 0 artifact 07 — JSON-pointer granularity, 9-step deterministic algorithm, explicit non-dismissible banner, `claim_refs` preserved, no raw payload field rendering, trust cap at `needs_verification`
- ✅ Each amendment cites the Phase 0 artifact it folded in
- ✅ ADR-0130 status remains `Proposed` (foundation work will promote to `Accepted` at v1.4.2 close)
- ✅ ADRs 0102/0105/0108 reciprocal cross-references added — each names ADR-0130 §2 + the invariant it preserves
- File delta: `.docs/decisions/0130-surface-independent-composition-contract.md` 188→253 lines (+65)

### W0-C — Phase 0 INDEX + status promotion

- ✅ `.docs/plans/dos-546/phase-0/INDEX.md` created with 15 rows (file, owning sub-contract, consumed-by wave letter, status, open questions)
- ✅ All 15 Phase 0 artifacts (01-15) frontmatter promoted to `status: spec:ready`
- ✅ Open questions captured in INDEX.md and routed to consuming wave-letter L0 Prep
- ✅ Every artifact 01-15 cited at least once in `02-issues.md` (no orphans)
- ✅ Inline fix: Phase 0 artifact 15 `:45-50` Origin guard rewritten as positive allowlist (cycle-3.1 reconciliation with W2-A)
- ✅ Inline fix: Phase 0 artifact 05 `:655` stale "ADR-0102 should be amended" open question resolved + cross-referenced to W0-D

### W0-D — ADR-0102 §7.1 amendment

- ✅ `mcp_exposure: bool` → `mcp_exposure: McpExposure { None | MetadataOnly | Invocable }` enum
- ✅ `client_side_executable: bool` retained as separate orthogonal field per Phase 0 artifact 05 lines 383-412
- ✅ `MetadataOnly` enumerates name + description only (no input/output schema); full schema reserved for `Invocable` — verified across 4 sites in ADR-0102 + 1 site in ADR-0111 §8
- ✅ Default policy: `mcp_exposure = McpExposure::None`, `client_side_executable = false`, opt-in per ability
- ✅ Macro compile-error gate explicitly pinned as substrate-enforceable (§7.6 dedicated paragraph): `#[ability]` emits compile-time error if `allowed_actors` includes `SurfaceClient` AND `required_scopes` is empty AND `no_scope_required` opt-out is absent
- ✅ §7.4 + §7.6 introspection paths split between MCP-mediated and client-side-JS
- ✅ ADR-0102 status: `Accepted` (cycle-2/cycle-3 are refinements, not re-decision)
- ✅ ADR-0111 §8 actor-filtered discovery cross-reference updated to match
- File deltas: ADR-0102 ~28 lines net; ADR-0111 +1 line; ADR-0105/0108 +1 line each (reciprocal cross-refs)

---

## L1 self-validation evidence

| Check | Command | Exit | Date |
|---|---|---|---|
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | 0 | 2026-05-11 |
| TS type-check | `pnpm tsc --noEmit` | 0 | 2026-05-11 |
| Rust tests | `cargo test` | _not run_ | doc-only wave — no source touched |
| Frontend tests | `pnpm test` | _not run_ | doc-only wave — no source touched |

W0 is a doc-only wave. cargo clippy + pnpm tsc were run to confirm the worktree has no regressions in untouched source. Skipping `cargo test` + `pnpm test` is consistent with the n-a-doc-only L2-status discipline.

---

## L2 review evidence

Three reviewer files per the wave-plan reviewer matrix (trust-boundary issues require /cso; substrate contract requires architect; codex is the adversarial baseline):

| Reviewer | Cycles | Verdict | File |
|---|---|---|---|
| /cso (security-auditor) | 1 | APPROVE (2 path-α observations, non-blocking) | `.docs/reviews/dos-546-w0-l2-cso-2026-05-11.md` |
| architect-reviewer | 1 | APPROVE (1 path-α: pre-existing ADR-0108 mis-label) | `.docs/reviews/dos-546-w0-l2-architect-2026-05-11.md` |
| codex review (adversarial) | 2 | cycle-1 REVISE (5 AC violations) → all fixed inline → cycle-2 APPROVE | `.docs/reviews/dos-546-w0-l2-codex-2026-05-11.md`, `.docs/reviews/dos-546-w0-l2-codex-cycle-2-2026-05-11.md` |

**Cycle-1 codex findings (5 literal AC violations) all closed in one pass:**
1. ADR-0102/0105/0108 reciprocal cross-refs to ADR-0130 → added with section pointers
2. ADR-0102 status `Proposed` → `Accepted`
3. MetadataOnly schema exposure: ADR-0102 + ADR-0111 prose tightened to "name + description only"
4. `no_scope_required` macro compile-error → pinned as substrate gate in ADR-0102 §7.6
5. Phase 0 artifact 05 line 655 stale open question → resolved + cross-referenced to W0-D

---

## Trust boundary check

W0 amendments touch the trust-boundary contract (ADR-0102 §7.6 multi-level enforcement). Per /cso L2 review:

- Effective gate order (W1 onwards must implement): Host → Origin → HMAC → rate-limit → auth → registry lookup → `allowed_actors` ∩ caller → `required_scopes` ⊆ caller scopes → `client_side_executable` (client-side-JS path) → `mcp_exposure` filter (MCP path) → dispatch
- Least-privilege defaults preserved across diff: `mcp_exposure = McpExposure::None` + `client_side_executable = false`
- ADR-0105 lives-once invariant preserved via `ProvenanceRef` (refs not envelope copies)
- No new EoP / info-disclosure / spoofing class introduced

---

## Path-α observations (file as maintenance, non-blocking)

Filed to project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` ("Codebase Maintenance & Production Quality"):

1. **/cso #1:** ADR-0102 §7.6 step 3 vs introspection bullet readability — `MetadataOnly` over MCP enumerates-only-not-invokes only stated in step 3, not restated in §7.6 introspection paragraph. Minor copy clarification.
2. **/cso #2:** ADR-0102 stacks three `Amended: 2026-05-10` lines for the same day. Awkward but accurate; load-bearing for git-archaeology readers.
3. **architect:** ADR-0102 line 508 pre-existing mis-label of ADR-0108 as "Surface-Independent Ability Invocation" (should be "Provenance Rendering and Privacy"). Not introduced by W0.

---

## Pending (post-Linear)

- [ ] W0-B Linear topology executed by James (supersede v1.4.2, create v1.4.3 parking, create new v1.4.2, create 28 issues)
- [ ] Linear IDs filled into the issue map table above
- [ ] Local commits with `L2-status: passed` trailer per `.githooks/commit-msg`
- [ ] Push + open PR
- [ ] CI green
- [ ] W0 wave merge gate signed
- [ ] W0 retro (mandatory per wave plan retro rules at end of W1, W2, W4, W6; W0 retro is OPTIONAL — single-day doc-only wave)

---

## Related L0 ladder evidence

Cycle-by-cycle L0 reviewer artifacts under `.docs/reviews/`:
- Cycle 0: `dos-546-l0-prep-codex-2026-05-10.md`, `dos-546-l0-cso-2026-05-10.md`
- Cycle 1 (plan-level L0): codex-challenge, codex-consult, architect, /cso
- Cycle 2: same panel
- Cycle 3 + 3.1 + 3.2: same panel + sweep verifications
- Plus 3 revision summaries documenting the 14 → 7 → 4 → 0 findings convergence

Wave-protocol invariant maintained: L0 unanimous APPROVE achieved before W0 implementation began.
