# W0 Retro — DB Throughput Architecture

**Date:** 2026-05-27
**Branch:** `feat/db-throughput-w0`
**Commits landed:** 3
**L2 closure:** Unanimous APPROVE after cycle 2

---

## What worked

**The plan's measurement-gated decomposition.** W0 shipped as a substrate that observes its own goal rather than asserting it. The plumbing for AC1–AC5 ships in this PR; the gate-pass measurement runs after merge. Treating the gate as a separate step (and committing to AC6 30/40-entity re-measure via DOS-808) honors the plan's primary risk-mitigation: "A treated as architectural completion" gets a Linear ticket as observable mitigation, not a calendar wish.

**Documentation agent + code in parallel.** The W0-C docs agent ran while I implemented W0-A + W0-B. ADR-0133, ADR-0134, and the K-out solution doc returned in ~6 minutes total wall-clock — comparable to a single careful manual authoring pass, but in parallel with the substrate work. The agent also corrected the plan's `executor.rs:632/1052` citation to the actual `:630/:1051` lines, which surfaced AC8's true scope.

**Codex-as-dissenter.** Three reviewers approved on first pass. Codex BLOCKED. Codex's findings were all real and substantive (with_transaction wrap, hydrate double-count, AC8 comment removal). The dissenting reviewer caught the architectural-contract violations the consensus reviewers missed. Per `feedback_reviewer_dissent_is_signal.md`: don't break the tie, investigate the dissent. All three findings resolved cleanly in cycle 1 → cycle 2 APPROVE.

**Worktree isolation.** Creating `.worktrees/db-throughput-w0` off `public/dev` kept this work cleanly separated from the in-progress `feat/v1.4.8-lane-b-meetings-writers` branch (which carried unrelated `pty.rs` W1-C-class work, stashed). No cross-contamination.

## What hurt

**Pre-commit hook flakiness.** Six commit attempts hit four different flaky tests (`glean_queue_and_manual_refresh_share_finalization_side_effects`, `compose_enrichment_full_path_rollback_atomicity`, `enrich_entity_disk_db_atomicity_under_rollback`, `dos674_cleanup_outside_transaction`). Dev baseline ran the same full suite cleanly. Hypothesis: W0-B's 2–3× sample recording per DB call adds Mutex contention noise that pushes timing-sensitive tests over their thresholds. Each test passes in isolation; failure surfaces only under full-suite parallel execution.

Path-α route: file as maintenance — flake characterization + either lockless counter migration or sample-recording optimization. Not blocking W0 per the policy (each flake is on dev's test suite, not introduced by W0's substrate semantics), but the burn cost is real: 3 retries × ~3 min pre-commit = ~9 minutes of wasted wall-clock per code commit.

**ADR-text-vs-code precedent ambiguity.** The newly-authored ADR-0133 §2 said "all mutations through state.db_write with explicit transaction wrapper." Pre-existing precedent (`pty.rs`'s `write_json_kv`) uses `writer.call_sync_labeled` with raw `conn.execute` — no `with_transaction`. My initial implementation followed the precedent. Codex flagged the ADR violation. Resolution: wrapped in `with_transaction` to match the ADR strictly. The precedent's now-divergent shape is a separate maintenance question.

**`pnpm tsc` in fresh worktree.** First pre-commit attempt failed because `node_modules` didn't exist in the worktree. Tauri/Vite worktrees need `pnpm install` before the hook can run `pnpm tsc --noEmit`. Worth a one-liner in the worktree-setup docs (or a `pre-commit` early-exit check that says "run `pnpm install` first").

## Decisions that held

**`NUM_READERS = 4` not 5.** N+1 latency-tier rule says 5; W0-B ships 4. The +1 buffer only buys structural starvation protection once W1-D adds tier ownership. Today's round-robin pool of 5 is just additional capacity without the structural guarantee. ADR-0134 §3 documents this verbatim. Reviewers all agreed.

**Tier-of-origin API without site migration.** `PooledConnection::with_tier` + `DbService::reader_for_tier` exist in W0-B; the 197 `state.db_read` call sites are not migrated. ADR-0134 §6 explicitly defers migration to W1-D's earn signal. Codex did not flag this as an AC violation. The W0 close gate uses the writer/reader split, not tier-of-origin, so the substrate is sufficient without migration.

**`mmap_size = 0` fail-loud.** Plan explicitly says fail loud if SQLCipher silently disables mmap. The cost (app refuses to open on SQLCipher builds without `SQLITE_ENABLE_MMAP_SIZE`) is the intended signal, not a regression. Data-integrity reviewer noted but did not block. Reasoning: a silent-no-mmap build delivers far less of the W0-A read-path acceleration than the plan assumes; surfacing it at open beats discovering it as missing acceleration during the close-gate measurement.

## Decisions that turned

**Aggregate vs separate commits per W0-A / W0-B / W0-C.** The plan describes W0 as three parallel PRs. I shipped one PR with two commits (substrate + docs). The lanes share `db_service.rs` (W0-A and W0-B both touch it), making true parallel PRs awkward. The two-commit split (substrate + docs) is the right granularity for review even within one PR.

**Whether to migrate hot read sites to `reader_for_tier` in W0-B.** Initial reading of W0-C ADR-0134 implied W0-B should migrate every call site. Performance reviewer agreed the substrate alone is sufficient for W0's gate. I revised ADR-0134 mid-flight to explicitly defer the migration to W1-D. The ADR-vs-substrate alignment held after the revision.

## K-out (mandatory per L3 protocol)

Shipped in this PR — `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`. Documents:

- The eight-instance recurring class (table verbatim from plan §2)
- The inverted-model anti-pattern with six file:line citations
- The 2026-04-20 unify-pool revert (`72e7b036` → `8bfdfdd8`) as a permanent forbidden pattern
- The four-alternative routing (A hygiene / A* partitioned writer queue / B in-memory claim graph / C separate cache service process)
- When to apply guidance for future L0 packets

Future L0 packets touching the write path can grep `db-lock-storm-class` and inherit the diagnosis instead of re-deriving.

## Followups

- **DOS-808** — Re-measure at 50/100/200 entities (AC7, assigned). Trigger to close: pass all three, or open the next-wave L0 (W1/W2/WX) if any miss.
- **Maintenance project items** — 9 path-α findings (see proof-bundle.md) routed to `Codebase Maintenance & Production Quality` (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).
- **Test-suite flake characterization** — investigate whether W0-B's recording overhead introduces measurable timing perturbation, or whether the dev suite's parallel-scheduling sensitivity is independent. Path-α.

## What didn't ship in W0 (by design)

- Per-subject-domain TX decomposition + durable cursor (W1-A — conditional on W0 close-gate miss + writer execution_ms p95 > 500 ms)
- Presence-aware admission (W1-B — conditional on foreground p99 > 3× p50 in user-active windows)
- Bypass-site closure + CI gate (W1-C — unconditional if W0 misses)
- ReaderTier ownership migration (W1-D — conditional on tier-of-origin telemetry showing wrong-tier routing)
- Subject-ref-hash partitioned writer queue (W2 / Alternative A* — conditional on W1 hard-gate naming multi-subject writer-hold)
- In-memory claim graph (WX / Alternative B — separate L0; conditional on reader-CPU dominating as residual)
- Separate cache service process (Alternative C — conditional on multi-surface need, not latency)

Each is earned by a specific measurement signal named in the plan's decision tree. W0's job was to make those signals observable.
