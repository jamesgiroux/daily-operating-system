# DOS-546 W1-A.2 L2 Architect Review — Commit 7705e6fd

**Verdict: APPROVE**

## Assessment

1. **Invariants preserved.** The `None`→`Some(set)` state machine maps cleanly onto the W1-A.1 lenient→strict transition. `compare_exchange(false→true, AcqRel/Acquire)` keeps the one-winner production semantics; the W1-B `from_descriptors_checked` idempotent no-op path still receives `Err(set)` on re-entry. ADR-0102 §7.6 substrate-enforcement guarantees are intact.

2. **Primitive choice is correct.** `RwLock<Option<>>` + `AtomicBool` is the right shape: the atomic is the *authority* for production single-init; the RwLock is just the *container*. `LazyLock` cannot revoke. `Mutex<Option<>>` alone forces serialized reads on every `ScopeSet::new`/deserialize call — a hot path. `arc_swap` would work but is heavier and not idiomatic for std-only code. Reads dominate; RwLock is appropriate.

3. **Escape hatch is sound but underdefended.** `set_allowlist_for_tests` is `#[doc(hidden)]` and named `_for_tests`; production code calling it would be obvious in review. However, there is no `#[cfg(test)]` gate or `debug_assertions` guard. Recommend a follow-up (path-α, file in maintenance project): `debug_assert!(cfg!(test) || cfg!(debug_assertions))` inside the test helpers. Not a merge blocker.

4. **No deadlock risk.** `initialize_allowlist` takes the atomic first, then the write lock. `new()` and serde paths take only the read lock and `drop(guard)` before returning `Ok`. No read-then-write upgrade pattern exists; `std::sync::RwLock` is not re-entrant but the code never re-enters.

5. **ADR-0111 §8 coherence intact.** Class-of-actors semantics are unaffected — the allowlist gates *scope vocabulary*, not actor class or per-instance grants. The two-level filter (actor class + per-instance scopes) lives in `SurfaceClientBridge`, orthogonal to this primitive.

**Recommendation:** Merge. File the `debug_assert!` hardening as a maintenance ticket.
