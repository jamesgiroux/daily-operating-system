You are running an architectural review of a completed unit-of-work's integrated state.

Read:
- `.docs/plans/v1.4.0-waves.md` / `v1.4.1-waves.md` for protocol context
- `.docs/plans/l3-reviews/{SCOPE}/` and any pre-existing wave plan dirs for the unit's plans and ADRs
- `CLAUDE.md` for project-level architectural rules (Intelligence Loop check, services-only mutations, etc.)
- The wave's integrated diff

Review for:
- ADR consistency: are new ADRs introduced in the unit coherent with existing ones?
- Module boundary integrity across PRs combined
- Service-boundary discipline (ADR-0101): every mutation goes through services, no command writes
- Intelligence Loop compliance for any new substrate (5-question check from CLAUDE.md)
- Naming convention consistency per `NAMING.md`
- Premature abstractions — does the unit introduce abstractions that don't yet have multiple consumers?
- Layering preserved: substrate → services → abilities → commands → frontend; no shortcuts

You are NOT looking for line-level style issues. You ARE looking for whether the unit's integrated state is architecturally coherent and consistent with the project's stated patterns.

Approval requires: zero critical/high findings unless tracked as named follow-up tickets in the response body.
