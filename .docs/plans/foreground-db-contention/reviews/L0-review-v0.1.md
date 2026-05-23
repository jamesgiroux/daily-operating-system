# L0 Review Ledger — Foreground DB Contention V0.1

**Date:** 2026-05-22

**Packet reviewed:** `.docs/plans/foreground-db-contention/L0-packet.md` V0.1

## Verdict

V0.1 returned **request changes** from all four reviewers. The reviewers agreed the cure is pointed at the real failure class, but the packet needed stricter scope boundaries before L1.

## Review Panel

- **Adversarial challenge:** request changes.
- **Performance/architecture:** request changes.
- **Security/CSO lens:** request changes.
- **Scope guardian:** request changes.

## Blocking Findings

1. Foreground `get_*` routes cannot cause hidden post-return writes. ADR-0101 requires reads to be pure; scheduled or spawned mutations from a read route still violate that contract.
2. `get_meeting_intelligence` has a pre-return live-calendar auto-persist branch that V0.1 missed.
3. The background poller claim was broader than the implementation scope. V0.1 said no background poller bypasses services, but only planned the sampled calendar attendee path.
4. Calendar batching needs a lock-scope rule: compute outside the transaction, keep the writer closure DB-only, and move filesystem/artifact side effects after commit.
5. Settings frontend fan-out alone is insufficient while first-screen status commands still use fresh writable DB opens.
6. Phase 1 left room for speculative read models or caches. Reviewers requested a negative acceptance criterion forbidding persistent projections and cross-render caches in Phase 1.
7. Phase 2 projection privacy needed audience/principal binding, not only actor class.
8. Measurement needed reproducible sample counts, cold/warm procedure, build mode, active background-worker evidence, and DB queue/open timing breakdowns.
9. PII-free diagnostics needed a concrete check for touched poller/batch paths.

## V0.2 Resolutions

- Foreground read purity now forbids direct, awaited, spawned, scheduled, and post-return mutations caused by `get_account_detail` or `get_meeting_intelligence`.
- The live-calendar-only meeting branch is now explicitly in AC2.
- PR1 is backend contention and measurement; PR2 is Settings UI mount fan-out if still needed.
- AC3 now requires proof-run background-worker inventory and narrows the implementation claim to the sampled calendar hot path.
- AC3 adds the DB-only writer closure and after-commit filesystem/artifact rule.
- AC4 adds backend cleanup for `get_context_mode`, `get_gravatar_status`, and `get_linear_status`.
- AC5 forbids Phase 1 read-model tables, materialized projections, and cross-render caches.
- Phase 2 rules now require actor/surface/policy/sensitivity and surface-client or scope-grant audience binding.
- AC1 defines sample counts and required proof metadata.
- AC6 adds a PII-safe diagnostics check for touched poller/batch logging.

## Follow-Up Review

V0.2 follow-up review returned three approvals and one remaining scope request-change: AC4 made Settings frontend fan-out conditional, but AC6 still required PR2 frontend tests unconditionally.

V0.3 split the criteria cleanly:

- PR1 requires Settings status-command DB cleanup, latency rollups, and backend proof.
- PR2 requires Settings lazy/deferred sections and connector concurrency tests only if PR2 is triggered.

Final V0.3 L0 verdict:

- **Adversarial challenge:** approve.
- **Performance/architecture:** approve.
- **Security/CSO lens:** approve.
- **Scope guardian:** approve.

L0 is approved for L1 implementation against the PR1 boundary in the packet.

## K-In References

- `.docs/decisions/0062-briefing-artifacts-vs-live-queries.md`
- `.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md`
- `.docs/decisions/0101-service-boundary-enforcement.md`
- `.docs/decisions/0104-execution-mode-and-mode-aware-services.md`
- `.docs/decisions/0111-surface-independent-ability-invocation.md`
- `.docs/decisions/0130-surface-independent-composition-contract.md`
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md`
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md`
