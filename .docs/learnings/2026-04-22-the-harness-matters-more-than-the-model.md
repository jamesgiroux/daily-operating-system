# The harness matters more than the model

**Status:** stub. Drafting TBD.

## Synopsis

Every few months, LLM capability takes a step forward, and a lot of AI product work gets obsoleted overnight. Almost none of the work that gets obsoleted is harness work: the scaffolding of memory, trust, provenance, abilities, evaluation around the model. As models improve, the harness gets more valuable, not less. This entry is about the rule we use to decide what scaffolding is earning its keep and what should be deleted the next time the model improves.

## Outline

- The lived moment: watching a prompt-engineering hack become obsolete when the model improved, and having to decide what to throw away and what to keep.
- Harness-stripping fixtures: a periodic test that runs abilities with specific pieces of scaffolding removed, to see whether the scaffolding is still earning its keep against a current-generation model.
- What always survives: the deterministic layer (memory, trust, provenance, corrections, policy). What comes and goes: prompt engineering, context composition tactics, fine-tuning scaffolds.
- Why this is counter-intuitive: most AI-product teams are investing in exactly the layer that will be obsolete next quarter.
- What's still open: where the boundary sits for ambiguous cases. Is prompt-template versioning "the harness" or "the prompt"? Is a context-composition pipeline a harness piece or a prompt piece?

## Related ADRs

- ADR-0118 (AI harness principles)
- ADR-0110 §9 (harness-stripping fixtures amendment)
- ADR-0119 (runtime evaluator)
