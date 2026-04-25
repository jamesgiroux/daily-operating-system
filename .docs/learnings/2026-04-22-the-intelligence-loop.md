# The intelligence loop

**Status:** stub. Drafting TBD.

## Synopsis

DailyOS only becomes interesting if it gets smarter through use without turning into a black box. This entry is about the loop underneath that ambition: signals arrive from different systems, get weighted, decay over time, attach to entities, feed retrieval, shape synthesis, and then get updated again through fresh activity and user feedback. The goal is not "the model remembers." The goal is a system that notices, weighs, and refreshes its understanding of the world.

## Outline

- The lived moment: realising that "entity intelligence" was not one pipeline run but a continuous loop of observation, scoring, enrichment, and revision.
- The inputs: email, calendar, notes, account systems, user-authored fields, MCP-connected tools, and event triggers.
- The mechanics: Bayesian-style weighting, source reliability, temporal decay, embeddings, hybrid retrieval, and selective AI calls.
- The feedback path: user corrections, explicit feedback, changed source material, tombstones, and invalidation. Corrections matter here because they are one kind of signal, not because they deserve pillar status on their own.
- The product implication: trust is downstream of the loop. If the loop is weak, trust bands and provenance are just presentation.
- What's still open: where the scoring logic should stay explicit, where learned weights might earn their keep, and how to keep the system understandable as the loop gets richer.

## Related ADRs

- ADR-0078 (entity intelligence as confidence-bearing synthesis)
- ADR-0080 (signal-based freshness and health scoring)
- ADR-0081 (feedback, decay, and learning loop)
- ADR-0097 (event-driven intelligence service)
- ADR-0113 (claim ledger, supersede semantics, tombstone pre-gate)
- ADR-0115 (signal propagation respects tombstones)
