# L2 (Diff) Review — DOS-546 W1-A cycle-2 — SurfaceClient + W1-A.1 ScopeSet promotion

- **Reviewer:** architect-reviewer
- **Date:** 2026-05-11
- **Cumulative commits:** `7fba6a22` (W1-A) + `0e98377c` (W1-A.1 correction) on `dos-546-wp-studio-spike`
- **Scope of authority:** L2 diff re-review bounded by W1-A AC at `.docs/plans/dos-546/v1.4.2-project/02-issues.md` lines 357, 365–367, 372, 400 plus W1-A0 line 290 and W1-B line 450; cross-checked against ADR-0102 §7.1/§7.6, ADR-0105, ADR-0108, ADR-0111 §8 line 214.

## Verdict

**APPROVE.**

F1 (cycle-1 blocking) is closed. F2 path-α (`todo!()` arms) remains appropriate as path-α per cycle-1 disposition and the wave-staging contract; file it to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` if not yet logged.

## F1 closure — evidence

1. **Struct variant.** `registry.rs:280-298` defines `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }` exactly. Matches AC line 365 and ADR-0111 §8 line 214 verbatim.
2. **`ScopeSet` invariants (registry.rs:103-265).**
   - `ScopeSetError::Empty` returned by `ScopeSet::new` when input iterator is empty (line 180-182).
   - `ScopeSetError::UnknownScopes` returned when `SCOPE_ALLOWLIST` initialized and any scope falls outside (line 183-190).
   - Deserialize routes through `ScopeSet::new` (line 257-265), so both empty and unknown-scope rejection fire at the wire boundary — same code path, no drift.
   - `SCOPE_ALLOWLIST: OnceLock<BTreeSet<SurfaceScope>>` with lenient bootstrap (accept-all until `initialize_allowlist()` called); documented at line 140-145.
   - Public API surface: `contains`, `iter`, `len`, `is_empty`, `new`, `initialize_allowlist`, `set_allowlist_for_tests` — all present (line 178-243).
   - `BTreeSet<SurfaceScope>` backing confirmed at line 166 — deterministic iteration order for AC line 372 audit emission.
3. **Tests.** Four new `ScopeSet` tests at registry.rs:2110-2146: empty-rejection at `new()`, valid-construction + membership, empty-rejection at deserialization, non-empty deserialization round-trip. Plus three updated `Actor::SurfaceClient` tests (round-trip, distinct-instance inequality, allowed-actors negative). Note: no explicit deser-rejects-unknown-scope test was added at this layer; the code path is exercised by `ScopeSet::new` which is tested for the empty branch, and the unknown-scope branch is identical structure — defensible but worth a one-liner test in W1-B's allowlist-initialization PR.
4. **Match-sites migrated.** Grep confirms all six match arms moved to `Actor::SurfaceClient { .. }` struct pattern: `get_entity_context.rs:435,473`, `prepare_meeting/synthesis.rs:1574`, `bridges/mcp.rs:734`, `bridges/tauri.rs:635`, `bridges/types.rs:62`. No stale tuple patterns remain.

## Adjudication defensibility (kept `SurfaceScope(String)` newtype)

**Defensible.** AC line 367's "outside the defined enum" reads naturally as "outside the registered allowlist" given W1-B's `#[ability]` macro `required_scopes` mechanism — the allowlist *is* the closed vocabulary, populated at registry boot from ability declarations. A compile-time enum would force a substrate rebuild for every new ability scope, which contradicts the runtime-contract intent of ADR-0102. Serde round-trip works cleanly (newtype is `serde(transparent)` over `String`); `ScopeSet.contains(&SurfaceScope)` is the natural W1-B bridge check; BTreeSet iteration order keeps `actor_scopes` audit emission deterministic per AC line 372. All three downstream consumers (W1-A0 audit emission, W1-B bridge enforcement, W3 construction site) are satisfied by the shipped shape.

## Cross-ADR consistency

- ADR-0111 §8 line 214: matches verbatim. PASS.
- ADR-0102 §7.1/§7.6 (registry-recognized actor): exhaustive matching preserved. PASS.
- ADR-0105 (trust scoring through provenance): unchanged at this layer. PASS.
- ADR-0108 (per-scope rendering): substrate ready; renderer wiring is downstream. PASS at substrate level.

## F2 path-α — unchanged

The six `todo!()` arms remain unreachable in production (no construction site outside registry tests). Disposition from cycle-1 still applies: swap to `unreachable!()` + add CI gate, filed to maintenance — does not block this PR.

## L1 evidence

Trusted per L2 protocol: cargo clippy `-D warnings` exit 0 + `cargo test --lib` 2162 passed / 0 failed / 11 ignored, as declared in commit `0e98377c`. Architect did not re-run.

## Final disposition

Architecture-reviewer cycle-2 verdict: **APPROVE**. Substrate is ready for W1-A0 (audit emission), W1-B (bridge enforcement + allowlist initialization), and W3 (construction site). Recommend reviewer panel close W1-A.

---

*Reviewer: architect-reviewer (Claude Opus 4.7 1M-context). L2 cycle-2 verdict 2026-05-11.*
