# L2 (Diff) Review — DOS-546 W1-A: SurfaceClient as the Fourth Actor Class

- **Reviewer:** architect-reviewer
- **Date:** 2026-05-11
- **Commit:** `7fba6a22` on `dos-546-wp-studio-spike`
- **Scope of authority:** L2 diff review, bounded by the W1-A acceptance criteria in `.docs/plans/dos-546/v1.4.2-project/02-issues.md` lines 363-373 + wave plan `.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md` lines 222, 243-248.
- **Cross-reviewed against:** ADR-0102 §7.1 + §7.6, ADR-0111 §8, ADR-0130 §6, W1-B issue body (lines 426-455).

## Verdict

**RECONSIDER.**

The diff is small, clean, well-tested at the variant identity level, and clearly intends to be the stage-1a substrate landing the rest of W1 + W2 + W3 compile against. The implementer's care is evident (`Copy` removed at every contagion site, `serde(transparent)` for stable wire shape, ten unit tests, `todo!()` arms flagged at every non-production match site with a W1-B+ pointer).

But the shipped shape diverges from the W1-A acceptance criteria in a way that is not within the implementer's authority to decide unilaterally, and the divergence has a downstream contractual consequence for W1-B. The right resolution is either (a) ship the AC-as-written shape, or (b) amend the W1-A and W1-B ticket bodies together to formalize the architectural relocation, then ship. Pick one before merging; do not silently merge an undocumented re-architecture.

This is not a fix-and-merge round. It is a contract reconciliation round.

---

## What's strong

1. **Substrate-contract fidelity at the identity-only layer.** `SurfaceClientId(String)` is correctly per-instance opaque (matches ADR-0111 §8 "Per-instance properties" / "Instance identity") rather than class-level. `#[serde(transparent)]` gives the audit emitter a stable wire shape, and `Display` is non-PII-aware-by-discipline (the type does no scrubbing, but documentation explicitly states the contract). Equality/Hash semantics tested at both newtype and `Actor` round-trip levels.
2. **`Copy` contagion correctly traced.** Removing `Copy` from `Actor` correctly cascaded into `TauriInvokeContext` and `OperationBridgeContext`, with explicit comments at both sites. Every clone-or-move site picked up either an `.clone()` or a `&` borrow. No phantom `Copy` derives remain.
3. **`todo!()` arms are all in dead paths today.** Verified by grep — every site that constructs an `Actor::SurfaceClient` value is inside `registry.rs` tests. No production handler can reach the `todo!()` arms in `get_entity_context.rs`, `prepare_meeting/synthesis.rs`, `bridges/mcp.rs`, `bridges/tauri.rs`, or `bridges/types.rs` until W3 ships the construction site. The `todo!()` choice is defensible *given* the W1-A → W1-B → W3 staging plan.
4. **Unit-test discipline.** Ten new tests cover variant equality, distinct-instance inequality, serde round-trip, Hash semantics, and the negative `.contains` semantics the registry will use. The negative test in particular pins the W1-A AC line 369 ("ability marked `allowed_actors: [User, Agent]` rejects a `SurfaceClient` invocation") at the `Vec<Actor>::contains` shape registries depend on. That's exactly the kind of test that prevents Wave 2 from inheriting a silent allow-list bug.
5. **Documentation discipline.** `Actor::SurfaceClient` doc comment is explicit about the amendment date, the `Copy` removal, the per-instance vs class-level identity distinction, and the W1-B handoff. This is the right tier of inline doc for a load-bearing substrate type.

---

## Findings

### F1 — Architecture / AC-shape (BLOCKING — requires Reconsider)

**Finding:** The W1-A acceptance criteria require `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }` (struct variant, two fields) plus a `ScopeSet` typed-set primitive that rejects empty construction and rejects unknown scope values at deserialization. The shipped diff ships `Actor::SurfaceClient(SurfaceClientId)` (tuple variant, identity-only) plus a `SurfaceScope(String)` newtype. There is no `ScopeSet` type, no `scopes` field on the actor variant, no construction-time empty rejection, no deserialization-time scope vocabulary validation.

**Citations:**
- AC line 365: ``Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet } lands in the abilities-runtime crate.``
- AC line 367: ``ScopeSet is a typed set of SurfaceScope values; the registry rejects scopes outside the defined enum at deserialization.``
- AC line 400: ``Empty ScopeSet — rejected at construction (a SurfaceClient with no scopes is not a paired surface; it is a misconfiguration).``
- Wave plan line 246: W1-A "Files owned: `abilities-runtime/src/actor.rs` (`Actor::SurfaceClient` variant + `SurfaceClientId` + `ScopeSet` types)."
- W1-B issue line 450: ``SurfaceClientBridge enforces required_scopes against Actor::SurfaceClient { scopes } at the bridge boundary before registry lookup.``

**Why this is BLOCKING and not path-α:** This is a literal acceptance-criterion deviation. Per CLAUDE.md "L2 path-α findings → maintenance, not cycle-N+1," only theoretical hardening files to maintenance; literal AC violations block. Also: W1-B's issue body explicitly destructures `Actor::SurfaceClient { scopes }` field-access at the bridge boundary. With the shipped tuple variant, W1-B as written does not compile — the next-wave consumer's contract is broken by the substrate landing.

**Implementer rationale (from the diff's doc comment):** "Per-instance scope grants ride alongside the actor through `SurfaceClientBridge` request context in W1-B; this stage-1a landing ships the identity-only shape so downstream wave plans can compile against the variant."

This rationale is architecturally plausible — embedding capability sets in per-request bridge context instead of in the actor identity is a defensible runtime pattern. But it is a re-architecture relative to the AC, not a stage-1a / stage-1b split that the wave plan authorizes. The wave plan never delegates W1-A's `ScopeSet` and `scopes` field to W1-B; the cycle-3 revision summary (lines 35-47) only moved `W1-A0` later, not `ScopeSet` off W1-A.

**Resolution options (pick one before merge):**

- **R1 — Land the AC-as-written shape (preferred default).** Promote the variant to struct form `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }`. Add a `ScopeSet` newtype around `BTreeSet<SurfaceScope>` (or `Vec<SurfaceScope>` with a dedup invariant) that rejects empty construction via `ScopeSet::new(impl IntoIterator<Item = SurfaceScope>)` returning `Result<Self, ScopeSetError>`. Add `#[serde(transparent)]`-equivalent serde plus a deserialization guard that rejects unknown scope values (the AC says "defined enum"; today there's no enum, so either introduce a finite `SurfaceScope` enum here or document the W1-B promotion timing). This is the smaller-blast-radius option because it preserves W1-B's contract as written.

- **R2 — Formalize the re-architecture (heavier).** Amend the W1-A AC and the W1-B AC together on the Linear tickets to relocate `scopes` from `Actor::SurfaceClient` into the bridge request context. Update wave plan §"Named ordering rules / W1 internal staging" and the W1-A files-owned line. Add a `BridgeRequestContext` type (or extend `AbilityContext`) to carry the `ScopeSet` alongside the actor. Re-circulate L0 on the W1-A and W1-B issues for the architecture change. Then ship.

R1 is preferred because (a) the architectural advantage of relocating scopes is not articulated anywhere in the cycle-3 plan packet and was not what the L0 reviewers approved, and (b) the AC-as-written shape is well-precedented for capability-bearing actor variants in audit-attribution substrates.

---

### F2 — `todo!()` arms vs AC "no panic" rule (PATH-α MAINTENANCE)

**Finding:** AC line 371 reads "No `unwrap()` or `panic!()` on actor-class branching; pattern matches are exhaustive." Six `todo!()` arms ship in the diff:

- `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:435` (`provenance_actor`)
- `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:473` (`render_actor_for_context`)
- `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1574` (`provenance_actor`)
- `src-tauri/src/bridges/mcp.rs:734` (test fixture `provenance_actor_for_test`)
- `src-tauri/src/bridges/tauri.rs:634` (test fixture `provenance_actor_for_test`)
- `src-tauri/src/bridges/types.rs:62` (`BridgeActor::from(Actor)` — the production type conversion)

`todo!()` is a panic. Per the AC's literal text, this is a violation. Per the operational reality, every site is unreachable today because no production construction site for `Actor::SurfaceClient` exists yet (verified by grep — only the registry's own tests construct the variant).

**Why path-α rather than blocking:** The unreachability gives the AC's intent (no surprise panics on actor branching in production) the same protection that `unreachable!()` would give, *given* the wave plan's W1-A → W3 staging. The variant has no public constructor today; W1-A0 will plumb construction at the audit-emission helper boundary; W3-B will plumb construction at the loopback endpoint. Both downstream issues are where the matching arms get filled in. Reverting `todo!()` to fully-implemented arms here would force W1-A to land W1-A0 + W1-B + W3-B's work, collapsing the wave staging.

**Recommended maintenance ticket scope (file to project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):**

1. Swap `todo!("W1-B+ wiring for Actor::SurfaceClient")` to `unreachable!("Actor::SurfaceClient construction is gated until W3-B; if you hit this, the wave-staging invariant was violated — see ADR-0111 §8")` at all six sites. `unreachable!()` is semantically what's meant; `todo!()` is a literal AC trip-wire. The shift is one line per site, zero behavior change.
2. Add a CI grep gate that fails the build if a `todo!()` arm matches `Actor::SurfaceClient` after W3-B merges (forces the downstream waves to fill the arms in rather than leaving permanent dead-code panics).
3. Track each `todo!()` site's downstream-issue ownership in a co-located comment (e.g., `// owned by: W1-A0`, `// owned by: W3-B`) so the L3 wave review at the end of W1 can verify every arm has a planned filler.

This is path-α because: (a) no AC-mandated functional behavior is broken; (b) the fix is mechanical and class-wide (the "two similar findings → sweep" rule from CLAUDE.md applies — there are six sites of the same shape, do them in one pass); (c) it does not block downstream substrate work.

---

### F3 — `actor.rs` vs `registry.rs` placement (INFORMATIONAL — wave plan inaccuracy)

The wave plan line 246 names `abilities-runtime/src/actor.rs` as W1-A's owned file. That file does not exist; the existing `Actor` enum lives in `abilities-runtime/src/abilities/registry.rs`. The implementer correctly landed the new variant + identity/scope newtypes adjacent to the existing `Actor` enum where they must compile.

No action on the implementer. Optional path-α: rename the W1-A "Files owned" line in the wave plan to read `abilities-runtime/src/abilities/registry.rs` (or, more cleanly, lift the `Actor` enum into a new `actor.rs` module in a separate maintenance ticket — but that is its own refactor and out of scope for W1-A).

---

### F4 — Cross-ADR consistency (PASS)

Checked against the named ADRs:

- **ADR-0102 §7.1 + §7.6** (Accepted, W0-D amendments): `Actor` is one of the registry-recognized actor classes; adding a fourth variant is supported by the existing pattern-match contract. The diff preserves exhaustive matching at every site (modulo F2's `todo!()` placeholder discussion). PASS.
- **ADR-0105** (Trust scoring): Trust scoring consumes `Actor` indirectly through provenance; the diff updates provenance attribution helpers (`provenance_actor` in get_entity_context + prepare_meeting + temporal) so trust derivation continues to read the actor cleanly. PASS at this layer (W1-A0's emission helper is where the actual `actor_instance` field gets populated).
- **ADR-0108** (Provenance rendering + privacy): ADR-0108's sensitivity rules are render-time and consume `Actor` to decide what to redact for whom. The `render_actor_for_context` site is one of the `todo!()` sites; the renderer cannot yet render a `SurfaceClient` actor. This is consistent with the W1-A scope-limits paragraph (line 361) and ADR-0108 §"new actor class fourth tier" semantics. The full per-scope rendering work is W1-A0 + ADR-0108 amendments per the AC's "Privacy rendering" surface-touched line. PASS at the substrate level; the rendering hookup is downstream-issue work.
- **ADR-0111 §8**: This is the binding contract for W1-A. The diff matches ADR-0111 §8 on instance identity (per-instance opaque), per-instance bridge plumbing (deferred to W2-A/W2-B), and audit attribution (deferred to W1-A0). The diff DEVIATES from ADR-0111 §8 line 214 ("Constructs `AbilityContext` with `Live` mode, `Actor::SurfaceClient { instance: <id>, scopes: <grants> }`") on the `scopes` field — see F1. ADR-0111 §8 is the canonical source for "scopes ride on the actor variant"; the implementer's relocation to bridge-context contradicts this.
- **ADR-0130 §6**: Composition contract consumption of `SurfaceClient` is W1-E work; this diff does not touch composition. PASS by non-interference.

---

### F5 — Scope discipline within stage-1a (PASS)

The diff does not bleed into W1-B (`AbilityPolicy.required_scopes`), W1-A0 (`audit_log.actor_instance` schema + `emit_surface_audit` helper), W1-C (inventory), W1-D (description CI gate), or W1-E (Composition types). File boundaries respected: `policy.rs` untouched, `audit_log` migrations untouched, no `inventory.rs` introduced. PASS.

---

### F6 — L1 evidence (TRUSTED, NOT RE-VERIFIED)

Per L2 review discipline, L1 (the implementer's `cargo clippy -- -D warnings` exit 0 + `cargo test --lib` 255 passed) is trusted. The architect did not re-run `cargo clippy` or `cargo test` for this review. If the reviewer panel includes a `code-reviewer` agent, that agent should re-run locally per its own L1-replay protocol. The architect reviews structure, not test-pass state.

---

## What "Reconsider" requires before re-review

1. **Pick R1 or R2 from F1.** If R1: re-shape `Actor::SurfaceClient` to the struct variant + introduce `ScopeSet` + add empty/unknown rejection + add tests for both rejection paths. If R2: amend the W1-A and W1-B Linear ticket bodies + the wave plan §"Named ordering rules" + ADR-0111 §8 (or carry an ADR-0111 amendment note in the W0-D ADR-0102 amendment) before re-submitting.
2. **File the F2 maintenance ticket against project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`** with the swap-`todo!()`-to-`unreachable!()` + CI gate scope spelled out. Reference the ticket from a comment at one of the `todo!()` sites so the W1 L3 wave review picks it up.
3. **No other changes required for the architect's L2 path.** F3 (informational), F4 (PASS), F5 (PASS), F6 (trusted) do not block.

If the implementer chooses R1, expected diff size is +60 lines (ScopeSet type + constructor + serde guard + 3 new tests). The variant promotion to struct form is mechanical at the six match sites.

---

## Final disposition

- **Architecture-reviewer verdict: RECONSIDER.**
- **Path forward:** F1 must be reconciled; F2 files to maintenance.
- **Cycle budget:** This is L2 cycle-1 for W1-A; per the review-loop policy, cycle-2 closes if F1 is resolved by R1 (in-cycle) or R2 (with documented ticket amendments). If R2 is chosen, the W1-B issue body also needs the matching edit before W1-B's L0 fires, so the reviewer recommends R1 unless the implementer + James jointly want the re-architecture.

---

*Reviewer: architect-reviewer (claude opus 4.7 1M-context). L2-status track: cycle-1 verdict written 2026-05-11. Co-reviewers per W1-A reviewer matrix: codex-review (challenge channel) + code-reviewer (implementation channel) + /cso (trust-boundary channel — substrate primitive flag triggers this).*
