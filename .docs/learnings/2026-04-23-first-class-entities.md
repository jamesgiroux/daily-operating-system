# First-class entities

**Status:** stub. Drafting TBD.

## Synopsis

DailyOS got more useful when it stopped treating work as a pile of documents and started treating people, accounts, projects, meetings, actions, and eventually the user as persistent things the system could reason about. This entry is about that shift from file-shaped context to entity-shaped intelligence, and why it changed nearly every downstream decision.

## Outline

- The lived moment: noticing that "today" was only as good as the system's understanding of who and what the day was actually about.
- The shift: from notes and folders to people, accounts, projects, meetings, actions, and the user as first-class nodes.
- What entities make possible: cross-source synthesis, longitudinal memory, account dossiers, meeting prep, and action tracking that survives context switches.
- Why it mattered architecturally: schemas, IDs, relationship edges, enrichment boundaries, and entity-scoped retrieval.
- The deeper product effect: DailyOS stopped being a daily command and became a system that could build durable operational intelligence around work.
- What's still open: how far the entity model should go before it becomes taxonomy theater.

## Related ADRs

- ADR-0045 (meeting preparation architecture)
- ADR-0046 (contact intelligence and account dossier shape)
- ADR-0057 (JSON structure for entity intelligence)
- ADR-0087 (current user as an entity)
- ADR-0088 (entity-centric information architecture)
- ADR-0089 (entity relationship graph foundations)
