# Local-first is strategy

**Status:** stub. Drafting TBD.

## Synopsis

DailyOS is not local-first because local is trendy or because privacy copy sounds good on a website. It is local-first because the product only makes sense if the personal context layer actually belongs to the person using it. This entry is about why the remote-first path was wrong for DailyOS, what local-first ruled out, and why the constraint sharpened the product rather than shrinking it.

## Outline

- The lived moment: realising that the usual AI SaaS architecture would force DailyOS to become a company system before it had earned the right to be a personal one.
- The rule: personal context lives on-device; publication is explicit; remote systems are connectors or control planes, not the canonical home of the user's working memory.
- What the rule forces: BYO-key model access, local storage, metadata-only telemetry, and much stricter boundaries around sync and debugging.
- What the rule blocks: easy aggregation, central AI features, and a lot of familiar product shortcuts. Real constraints, accepted deliberately.
- What the rule gives back: a credible ownership story, cleaner trust boundaries, and a product shape that fits the TAM / Customer Success use case instead of fighting it.
- What's still open: team intelligence, publish boundaries, and which parts of the system should ever be allowed to become shared infrastructure.

## Related ADRs

- ADR-0092 (data security at rest)
- ADR-0100 (individual context layer architecture)
- ADR-0116 (tenant control plane boundary)
- ADR-0121 (team intelligence, Open)
