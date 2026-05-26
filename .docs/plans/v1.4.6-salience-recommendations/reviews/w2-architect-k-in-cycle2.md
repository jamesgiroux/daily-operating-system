# W2 L0 architecture / K-in review — cycle 2

Verdict: BLOCK

Finding:

- `SurfacingDecisionMade` and `SalienceCandidateRefreshTriggered` are registered as invalidation signals, but the cycle 2 packet emitted both with non-propagating `emit_once_for_key`. The service facade's `emit_once_for_key` only inserts the signal row; it does not run propagation. Current registry policy places both variants under `coalesced_invalidation_policy`, which is `SignalRole::Invalidation` / async propagation. That violates ADR-0115's "policies are declared, not chosen per call site" contract.

Required fix:

- Align the signal role and emission path before implementation. Cycle 3 keeps the predeclared invalidation role and switches W2 to `services::signals::emit_once_for_key_and_propagate`, with `PropagationEngine` passed through the W2 service APIs.
