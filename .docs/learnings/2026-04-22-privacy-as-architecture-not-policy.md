# Privacy as an architectural constraint, not a policy

**Status:** stub. Drafting TBD.

## Synopsis

"Content never leaves your laptop" is not a feature you toggle or a promise you make in a privacy page. It's an architectural choice that rules out entire classes of implementation. This entry is about what choosing that constraint on day one did to every other decision downstream, and why it made the system better rather than worse.

## Outline

- The lived moment: realising that every AI productivity tool I'd looked at was piping customer content to a vendor's server and calling it "enterprise-ready."
- The rule: content stays on-device; any server component (if one ever exists) sees metadata, not content.
- What the rule forces: BYO-key LLM, local SQLite encrypted at rest, metadata-only telemetry, no central index of individual user content.
- What the rule blocks: cross-user aggregation, server-side AI features, easy remote debugging, a lot of common product patterns. Real costs, accepted deliberately.
- What the rule gives back: a trust story nobody else in the category can credibly tell, and design pressure that forced better answers downstream.
- What's still open: team intelligence (ADR-0121). How to share structured state across users without breaking the core boundary.

## Related ADRs

- ADR-0092 (data security at rest)
- ADR-0116 (tenant control plane boundary)
- ADR-0121 (team intelligence, Open)
