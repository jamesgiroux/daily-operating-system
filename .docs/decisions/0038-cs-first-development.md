# ADR-0038: CS-first development focus

**Date:** 2026-02-06
**Status:** Accepted
**Supersedes:** [ADR-0020](0020-profile-dependent-accounts.md) (profile system as co-equal CS + General)

## Context

ADR-0020 described CS and General as co-equal profiles. In practice, "General" is a fiction — every real professional role has specific workflows (account tracking for CS, sprint management for engineering, pipeline management for sales). Building a generic baseline that satisfies no one wastes effort and creates ambiguity about what to optimize for.

Customer Success is the only profile with real usage, real data flows, and real feedback. The right strategy is to build CS-first, prove the extension/feature model, and let other roles follow the same pattern.

## Decision

Customer Success is the first and only implemented extension. All current development targets CS workflows: account tracking, customer meetings, renewal health, stakeholder management.

1. **Config defaults to `customer-success` profile.** First-run profile selector defaults to CS.
2. **"General" profile is deferred** — it remains in code as a fallback but receives no active development or testing.
3. **Future roles are new extensions** — Marketing, Executive, Sales, Engineering each get their own extension bundle. They add features, they don't subtract from CS.
4. **Third-party extensions** (ADR-0039) are the preferred long-term path for non-CS roles.
5. **The CS extension proves the pattern** that all future extensions follow: feature bundles, profile activation, post-enrichment hooks, data schemas.

## Consequences

- Removes decision paralysis about "will this work for General too?" — if it works for CS, ship it
- General profile users get core features (briefing, calendar, inbox, actions) but no CS-specific enrichment
- ADR-0020's profile switching mechanism remains intact (ADR-0009) — profiles are still non-destructive
- When a second role is needed, the extension architecture (ADR-0026, ADR-0039) is already proven
