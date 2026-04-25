# Provenance as a first-class output

**Status:** stub. Drafting TBD.

## Synopsis

Every claim the AI makes carries its sources as structured data, attached to the output, one click away from display. Not a ChatGPT-style footnote list. A provenance envelope with field-level attribution, prompt fingerprint, and a hard size cap. This entry is about why the envelope has to be a first-class output type of every AI call, and what happens when it isn't.

## Outline

- The lived moment: a briefing claim that was confidently wrong, and having no way to ask the AI "where did you get that?"
- The envelope shape: identity, temporal context, source attribution, composition tree, field-level attribution, prompt fingerprint, trust assessment.
- The 64 KB size cap and why it had to be hard (and the debugging that made the cap necessary).
- Field-level attribution: which sentence in a generated paragraph came from which source claim.
- Click-to-source as the affordance that makes the envelope pay off for the user.
- What's still open: how to show provenance for LLM-synthesized content without overwhelming the reader, especially when the source is a probabilistic composition of twenty claims.

## Related ADRs

- ADR-0105 (provenance as first-class output)
- ADR-0106 (prompt fingerprinting)
- ADR-0108 (rendering and privacy, 64 KB size cap)
