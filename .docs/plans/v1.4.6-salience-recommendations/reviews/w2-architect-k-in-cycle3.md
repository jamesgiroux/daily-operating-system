# W2 L0 architecture / K-in review — cycle 3

Verdict: PASS

No pre-code architecture/K-in blockers.

- Both predeclared W2 signals are registered as invalidation signals and the packet now uses `services::signals::emit_once_for_key_and_propagate` with `PropagationEngine`.
- Durable salience recompute ownership is singular: W2-A calls W1-B `recompute_salience_for_claim`; W2-B calls W2-A and copies returned evaluation/decision IDs.
- No parallel claim/feedback/signal substrate: recommendations stay claim-backed, feedback/dismissal reads reuse `claim_feedback` and `claim_surface_dismissals`, and policy state is scoped to recommendation surfacing audit rather than a generic replacement substrate.

Note: ADR files are named `0125-claim-anatomy-temporal-sensitivity-typeregistry.md` and `0126-memory-substrate-invariants.md` in this worktree.
