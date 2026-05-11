# L2 Code Reviewer — DOS-546 W1-A.2 (7705e6fd)

**Verdict: APPROVE.**

## Assessment

1. **Architecture (RwLock<Option<>> + AtomicBool).** Clean separation. `None`/`Some` carries allowlist state; atomic carries the one-time-init bit. Avoids the `OnceLock` cannot-reset limitation without conflating "is initialized" with "what is the content." Reads remain cheap (RwLock read path; brief drop before `Ok(Self(set))`).

2. **compare_exchange correctness.** `AcqRel` success / `Acquire` failure is the right pairing: the success path's Release synchronizes-with subsequent Acquire-reads of the atomic, and the write-lock on `SCOPE_ALLOWLIST` is itself a happens-before edge for the contents. Concurrent first-callers collapse to exactly one winner; losers get `Err(set)` (preserves W1-B `from_descriptors_checked` no-op idempotence).

3. **set_allowlist_for_tests bypass.** Acceptable. Marked `#[doc(hidden)]`, doc explicitly states it does not touch the atomic, so prod single-init invariant is preserved even if a test helper were misused — prod's `initialize_allowlist` still rejects a second call. Risk is bounded.

4. **seed_test_allowlist baseline.** Appropriate: centralizes the fixture vocab (`read.account_overview`, `submit.feedback`) used pervasively in this module's tests, eliminates per-test boilerplate, and prevents future tests from re-introducing the ordering bug.

5. **clear_allowlist_for_tests.** Useful, not scope creep — symmetric with seed, supports tests that need to exercise the unseeded lenient path.

6. **No new races.** Fix is structural, not papering: the symptom was test-ordering coupling via OnceLock immutability; the new primitive removes that coupling at the type level.

7. **Doc comments.** Updated thoroughly; state semantics, prod-vs-test split, and atomic role all documented inline.

Minor (non-blocking, file as maintenance if desired): the `drop(guard)` in `new_inner` is redundant — the guard would drop at scope end anyway — but it's defensible as a readability signal that the lock is released before construction. No action required.

285/0 pass + clippy clean. Ship.
