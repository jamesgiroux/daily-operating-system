# ADR-0088: People Relationship Network Intelligence — Graph Model, Two-Layer Intelligence, Network Propagation

**Date:** 2026-02-21
**Status:** Accepted
**Complements:** ADR-0087 (Entity Hierarchy Intelligence) — ADR-0087 explicitly excludes people from the hierarchy/rollup model because people's structural pattern is a relationship graph, not a tree. This ADR defines the people model.
**Extends:** ADR-0057 (Entity Intelligence Architecture) — two-file pattern for people already exists; this ADR adds the `network` section to that pattern.
**Uses:** ADR-0079 (Role Presets) — `entityModeDefault` and vocabulary fields shape how network intelligence is framed per role.

---

## Context

People are the most under-represented entities in DailyOS today. A person is a row in a table: name, title, account links, interaction history. Their `intelligence.json` contains individual-level assessment — meeting cadence, email sentiment, commitment tracking — but nothing about how that person relates to other people. Two people at the same account, each with strong individual profiles, may be in direct conflict with each other. The DailyOS user has no visibility into this.

The structural problem is that people don't form trees. Unlike accounts (where Cox Enterprises → Cox B2B → Cox B2B Southeast is a meaningful ownership hierarchy) or projects (where a campaign rolls up into a program), people exist in relationship graphs. A buying committee at Acme Corp is a network of individuals with influence flows: the CFO approves but defers to the champion, the champion won't buy without the technical evaluator's sign-off, and the blocker has informal veto power the org chart doesn't show. The DailyOS user currently sees five independent Person entities with no structural relationship to each other.

The same pattern appears across all roles:
- CS user: champion → economic buyer → legal approver → implementation lead (relationship chain, not org hierarchy)
- Product user: PM → engineering lead → design lead → data analyst (collaboration graph, not reporting structure)
- Marketing user: content lead → SEO contributor → demand gen → brand approver (workflow dependency graph)

ADR-0087 explicitly deferred people from the hierarchy model: "People are explicitly excluded from this model — people don't have a tree hierarchy in DailyOS, they have a relationship network, which requires a different architectural approach not covered by this ADR." This ADR covers that approach.

---

## Decisions

### 1. Person-to-person relationships as a typed, directional graph

A `person_relationships` table stores typed edges between Person entities:

- **Edge types:** `champion`, `executive_sponsor`, `decision_maker`, `technical_evaluator`, `blocker`, `peer`, `ally`, `detractor`, `collaborator`, `dependency`
- **Directionality:** Edges are directional. "Person A champions Person B's approval path" is distinct from "Person B champions Person A's." Many relationships are symmetric in practice (peer, ally, collaborator) but the model doesn't enforce symmetry — it emerges from evidence.
- **Confidence:** Each edge carries a `confidence` score (0.0–1.0) derived from signal evidence or explicit user confirmation. An edge extracted from a transcript mention starts low confidence (~0.4). An edge confirmed by the user is set high (~0.9). Edges decay over time without reinforcement, following the same temporal decay model as signal confidence (ADR-0080).
- **Context scope:** Edges are optionally scoped to an entity context — `context_entity_id` + `context_entity_type`. Person A is a champion in the context of Account X's renewal, but a peer in the context of Project Y. A person can have different relationship roles in different contexts.

Schema sketch:
```sql
CREATE TABLE person_relationships (
    id TEXT PRIMARY KEY,
    from_person_id TEXT NOT NULL,
    to_person_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,     -- champion, blocker, peer, etc.
    direction TEXT DEFAULT 'directed',   -- 'directed' | 'symmetric'
    confidence REAL NOT NULL DEFAULT 0.5,
    context_entity_id TEXT,              -- optional: scoped to an account or project
    context_entity_type TEXT,
    source TEXT NOT NULL,                -- transcript, email, user_confirmed, inferred
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_reinforced_at DATETIME,
    FOREIGN KEY (from_person_id) REFERENCES people(id),
    FOREIGN KEY (to_person_id) REFERENCES people(id)
);
```

### 2. Two-layer person intelligence

Person intelligence already has a layer today (own signals). This ADR adds a second:

**Layer 1 — Own signals (existing):** Individual-level assessment produced from meetings the person attended, emails from/to them, commitments they made, and sentiment signals. This layer is unchanged.

**Layer 2 — Network signals (new):** Derived from the person's relationship graph — who are they connected to, what's happening to those connected people, what does the person's position in the graph suggest. Network signals are extracted from:
- Transcript mentions of relationship patterns ("Sarah has to approve anything Jim brings forward")
- Email thread topology (who is CC'd with whom, who escalates to whom)
- Meeting attendance overlap patterns (two people who always attend together form a peer edge; one person who is always missing when another is present may be a conflict signal)
- User-confirmed relationships (explicit "this person is my champion" input)

The AI enrichment prompt for a person includes: their own signals (Layer 1 context), the current state of all their relationship edges, and brief summaries of connected persons' recent signals. The enrichment produces both updated Layer 1 fields AND the `network` section.

### 3. Network section in `intelligence.json`

Every person's `intelligence.json` gains a `network` top-level field:

```json
{
  "network": {
    "health": "strong | at_risk | weakened | unknown",
    "key_relationships": [
      {
        "person_id": "...",
        "name": "Sarah Chen",
        "relationship_type": "executive_sponsor",
        "confidence": 0.82,
        "signal_summary": "Consistently advocates in leadership reviews. Reaffirmed Q1 support last week."
      }
    ],
    "risks": [
      "Primary champion (Jim Park) changed roles — influence path unclear",
      "Legal contact has not responded in 3 weeks — cadence risk"
    ],
    "opportunities": [
      "Sarah Chen → CFO relationship creates path to executive expansion"
    ],
    "influence_radius": 4,
    "cluster_summary": "AI-synthesized paragraph describing this person's position in their relationship network — who they're connected to, what their influence looks like, and what's changing."
  }
}
```

Leaf-node persons (no relationship edges) return `network.health = "unknown"` with empty arrays. The `network` section is always present in the schema but may be sparse. Sparse = informative: an unknown network for a contact you've met 8 times is a gap to address.

### 4. Network signal propagation

The existing signal bus (ADR-0080) handles cross-entity propagation. Person→account propagation already exists. This ADR adds person→person propagation:

**Trigger:** When a person emits a signal above a confidence threshold (default: 0.65), derived signals are generated for their relationship-graph neighbors.

**Propagation weight:** `neighbor_confidence = original_confidence * relationship_confidence * 0.4`

The 0.4 base multiplier (vs. 0.6 for hierarchy upward, 0.5 for hierarchy downward) reflects that relationship influence is more attenuated than organizational membership. A person changing roles at an account affects the account more directly than it affects a peer they've worked with.

**Edge-type sensitivity:** Not all relationship types propagate equally:

| Source relationship type | Propagation to target | Rationale |
|--------------------------|----------------------|-----------|
| `executive_sponsor` | Yes (full multiplier) | High influence role; changes here affect the whole engagement |
| `champion` | Yes (full multiplier) | Direct deal/project influence |
| `decision_maker` | Yes (full multiplier) | Approval path changes are consequential |
| `blocker` | Yes (1.2x multiplier) | Blocker removal/addition is high signal |
| `peer` | Partial (0.7x multiplier) | Peer context is valuable but less direct |
| `ally` | Partial (0.7x multiplier) | Same as peer |
| `detractor` | Yes (1.2x multiplier) | Detractor signals are high-priority |
| `collaborator` | No (unless high confidence) | Too broad; creates noise at low confidence |
| `dependency` | Partial (project context only) | Only propagate within shared project context |

**Propagation scope:** Person→person propagation does not cascade beyond 1 hop per cycle. A signal on Person A propagates to Person B (A's neighbor). It does NOT automatically propagate from B to C (B's neighbor) in the same cycle. This matches the ADR-0087 "direct children only" rule and uses the same loop-prevention mechanism: derived signals (source containing `propagation:network`) are not re-propagated.

**Person→account propagation:** Unchanged. Person signals still propagate to their linked account(s) via the existing rules. A person departure signal now propagates both to connected people AND to their linked account — two parallel propagation chains from the same source signal.

### 5. Relationship cluster view on person detail page

The `PersonDetailEditorial` page gains a **Network** chapter:

- **Relationship cluster visualization** — a compact list/graph view of the person's typed relationships, grouped by context (Account X relationships, Project Y relationships). Each relationship shows: person name, type badge, confidence indicator, and last-reinforced date.
- **Cluster summary** — the AI-generated `network.cluster_summary` paragraph.
- **Network risks and opportunities** — surfaced from `network.risks` and `network.opportunities` as callout-style components using the existing signal prose component.

The chapter does not render if the person has no relationship edges. No empty state — absence of the section communicates "no known network relationships" cleanly.

Clicking any connected person navigates to their detail page.

### 6. Vocabulary adapts to role preset

The `network` section's AI-generated prose adapts to the active role preset's vocabulary field (ADR-0079). The relationship type names in the UI also adapt:

| Relationship type (internal) | CS / Sales label | Product / Engineering label | Marketing label |
|-------------------------------|------------------|-----------------------------|-----------------|
| `champion` | Champion | Advocate | Sponsor |
| `executive_sponsor` | Economic Buyer | Executive Sponsor | Brand Owner |
| `decision_maker` | Decision Maker | DACI Driver | Campaign Lead |
| `blocker` | Blocker | Dependency Risk | Blocker |
| `technical_evaluator` | Technical Evaluator | Tech Lead | Platform Owner |

The internal type is persisted in the DB. The label is resolved at render time using the active preset's vocabulary map. No schema changes needed to switch roles.

### 7. Edge detection is probabilistic; user confirmation is the source of truth

Edges extracted from transcripts and emails are probabilistic. The system should surface edge suggestions to the user ("Looks like Sarah is your champion at Acme — confirm?") rather than asserting them silently. Unconfirmed edges contribute to network intelligence at reduced confidence. Confirmed edges carry full confidence and are exempt from temporal decay until the user revokes them.

Edge suggestion generation is part of the enrichment pipeline — not a separate interaction. When enrichment produces a network section that includes a new inferred edge, the edge is flagged as `suggested` and surfaces as a low-friction confirmation UI (single tap to confirm or dismiss) in the person detail's Network chapter.

---

## Consequences

- New `person_relationships` table requires a DB migration.
- Person enrichment prompts expand to include relationship context. Prompt cost increases for persons with large networks; this is bounded — a well-connected contact might have 8–12 relationships, which is manageable in context.
- The enrichment scheduler needs to re-enrich a person when their relationship graph changes (edge added, confidence updated, neighbor signals received). This triggers the existing intel_queue mechanism — no special handling needed.
- The Network chapter on PersonDetailEditorial is a new frontend component. It follows the existing chapter/signal prose component patterns per the design system.
- Relationship type vocabulary adaptation requires a vocabulary map per preset. This is new data but follows the existing preset structure (ADR-0079) — add a `relationship_vocabulary` field to the preset type.
- Edge detection quality depends on transcript and email content quality. In low-data situations (new contacts, sparse meeting history), the network section will be sparse. This is the correct behavior — sparse = transparent, not confabulated.
- Person→person propagation combined with the existing person→account propagation means a high-confidence signal on a person can now fan out to: their relationship graph neighbors AND their linked accounts. Confidence decay rates ensure this doesn't produce noise — a peer edge at 0.4 confidence propagating a 0.65 signal produces a 0.65 × 0.4 × 0.4 = ~0.10 derived signal, well below any enrichment threshold.
