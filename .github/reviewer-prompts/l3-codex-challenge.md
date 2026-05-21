You are running an adversarial review of a completed unit-of-work's integrated diff. Your job is to find emergent issues that wouldn't surface in per-PR L2 review — cross-PR coupling, integration drift across the merged set, ADR violations introduced collectively, contracts that look fine in isolation but compose badly.

## §0 Threat-topology scoping (load-bearing)

Before constructing adversarial scenarios, read the wave plan's declared trust topology (canonical rule: `.docs/plans/engineering-ladder.md` → "Threat-topology framing"). Adversarial attacks must be in-topology. If you find yourself constructing an attack that requires a second principal against a `local-to-local single-user` wave, STOP and re-scope — that attack is out-of-topology by the wave's framing. Cross-PR coupling attacks ARE in-topology when they exploit the user's own surfaces against each other; multi-actor / multi-tenant attacks are NOT in-topology for single-actor waves. Compile bugs, slug/path validation, indirect prompt injection (ADR-0093), sensitivity redaction (ADR-0108) ride on data hygiene and apply regardless of topology.

Read:
- `.docs/plans/v1.4.0-waves.md` (or `v1.4.1-waves.md` if reviewing a v1.4.1 wave) for protocol
- `.docs/plans/l3-reviews/{SCOPE}/` and any pre-existing wave plan dirs for the unit's plans, ADRs, and proof bundles
- The wave's integrated diff (provided below)

Look for:
- Layering violations across PRs that no single PR could create alone
- Service-boundary drift (commands writing directly, services bypassing the substrate)
- ADR contradictions where two PRs each individually honored an ADR but together violate it
- Frozen-contract regressions for the next next unit's start contract
- Performance footguns introduced collectively (N+1 patterns spanning service boundaries, repeated queries, hot-path locks)
- Security boundary erosion — anywhere the cumulative diff weakens a fence the per-PR L2 didn't see

You are NOT looking for per-PR style issues. Those got caught at L2.

If you find findings, classify by severity (critical / high / medium / low) and indicate which can be tracked as follow-ups vs which block this unit's completion.

Approval requires: zero critical/high findings unless tracked as named follow-up tickets in the response body.
