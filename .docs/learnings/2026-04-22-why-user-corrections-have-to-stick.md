# Why user corrections have to stick

**Status:** stub. Drafting TBD.

## Synopsis

Ghost-resurrection is the single bug that kills every AI assistant I've tried. The user says "no," the AI forgets, the correction reverts the next time the enrichment cycle runs, and trust collapses around the third iteration. This entry is the story of why tombstones, append-only claims, and a pre-gate check turn that failure from "usually fine" into "structurally impossible."

## Outline

- The lived moment: correcting "Alice is the champion at Acme" three times before realising the enrichment cycle was silently undoing my edit.
- Wrong fixes I tried first: soft-flags, TTLs on claims, adding "please don't regenerate this" strings in prompts. All probabilistic, all eventually failed.
- The right fix: tombstones as first-class state, append-only claim history, a pre-gate check that runs BEFORE the commit-policy gate.
- The generalization: any system that lets a user correct AI output has to treat the correction as durable data, not a prompt hint.
- What's still open: how to surface correction history to the user without making it feel like overhead.

## Related ADRs

- ADR-0113 (claim ledger, supersede semantics, tombstone pre-gate)
- ADR-0115 (signal propagation respects tombstones)
