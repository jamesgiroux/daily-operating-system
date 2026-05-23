# Abilities Runtime Producer Audit — 2026-05-23

## Scope

Audit whether the current `get_entity_intelligence` runtime is receiving the entity evidence DailyOS already stores, and whether MCP v2 can use that evidence to answer a headless executive-briefing prompt with claim-backed, human-readable output.

This audit is intentionally entity-general. Accounts are the visible failing surface today, but the design target is a runtime that works for accounts, people, projects, meetings, and future entity kinds without adding one surface-specific producer per entity type.

Wave plan: `.docs/plans/abilities-runtime-producer-remediation-waves.html`.
L0 review artifact: `.docs/plans/abilities-runtime-producer-remediation-l0-cycle1.md`.

## K-in

Reviewed existing architectural guidance before recommending changes:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`
- `.docs/decisions/0102-abilities-as-runtime-contract.md`
- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md`
- `.docs/decisions/0057-entity-intelligence-architecture.md`
- `.docs/decisions/0081-event-driven-meeting-intelligence.md`
- `.docs/decisions/0088-people-relationship-network-intelligence.md`
- `.docs/decisions/0097-account-health-scoring-architecture.md`
- `.docs/decisions/0098-data-governance-source-aware-lifecycle.md`

The strongest existing constraint is that MCP, WordPress, Tauri, and future heads should invoke abilities over the same substrate. They should not each invent schema-specific readers to patch missing producer coverage.

## Verdict

`get_entity_intelligence` is the right envelope shape, but it is underfed.

The runtime contract is mostly generic: it supports accounts, people, projects, and meetings, and it already has section shapes for facts, health, metadata proposals, open loops, touchpoints, threads, and record entries. The gap is not primarily the envelope. The gap is producer/read-model coverage:

- Structured entity facts can exist in schema/source-reference tables without being claim-backed.
- Relationship and attendance evidence exists in SQLite but is not promoted or summarized for the runtime.
- Entity context claim expansion only walks parent/child hierarchies, so related people, meetings, actions, and communications do not reliably reach a root entity briefing.
- MCP v2 currently requests a richer envelope than it returns to the host model.

Status: `DONE_WITH_CONCERNS`. The architecture is salvageable and pointed in the right direction, but the next work should be a generic entity-neighborhood and producer pass, not another account-only patch.

## Current Runtime Inventory

### Envelope

`get_entity_intelligence` declares generic subject kinds and sections in `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs`:

- Subject kinds: `account`, `project`, `person`, `meeting`
- Sections: `facts`, `health`, `metadata_proposals`, `open_loops`, `touchpoints`, `threads`, `record`
- Output shapes: `EntityFact`, `HealthStory`, `MetadataProposal`, `OpenLoopWithReceipt`, `TouchpointBundle`, `ThreadSummary`, `RecordEntry`

This is a good shape for a headless DailyOS runtime because it is not account-only.

### Producers and Readers

`src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/producer.rs` currently composes:

- `facts` from active surfaced claims
- `record_entries` from the same claims
- `open_loops` from `list_open_loops`
- `touchpoints` from `read_entity_touchpoints`
- `health_story` only for meeting subjects via meeting prep status

The following sections remain empty or thin:

- `metadata_proposals` is always empty.
- `threads` is always empty.
- `health_story` is absent for account, project, and person subjects.
- `touchpoints` are present in the envelope but unscored and currently not projected by the MCP account-status handler.

### Claim Reader Scope

`load_entity_context_claims_active_for_surface` loads active surfaced claims for the root subject, then expands only through immediate parent/child hierarchy:

- Account -> parent/child accounts
- Project -> parent/child projects
- Person -> no related subjects
- Meeting -> no related subjects

That means a root entity briefing does not automatically see evidence attached to related people, meetings, actions, emails, documents, or projects unless those signals were also committed directly to the root subject as claims.

### MCP Projection

`dailyos.read.account_status` invokes `get_entity_intelligence` with:

- `facts`
- `open_loops`
- `touchpoints`
- `record`

But the handler response only projects facts, open loops, trust, sensitivity, provenance, and section states. It does not project touchpoint details, record entries, attendance/participation summaries, stakeholder candidates, or threads. This limits what Claude Desktop can use even when the runtime envelope has more evidence.

The handler also treats the incoming `subject` parameter as the account id directly. Natural-language account resolution is still a follow-on gap for Claude Desktop prompts.

## Evidence Inventory

Read-only production DB inspection shows the substrate already has useful evidence classes:

- Structured entity records: accounts, people, projects, meetings
- Source references for structured account facts
- Relationship records and roles between accounts and people
- Meeting-to-entity links and meeting attendee data
- Actions and open-loop-shaped work records
- Captures, transcripts, email threads, indexed content, and linked entities
- Claim lifecycle tables, trust inputs, corroborations, contradictions, and feedback primitives

Important shape findings:

- Existing claim coverage is strongest for synthesized summaries, risks, wins, current state, and value delivered.
- Structured account facts were populated in schema/source-reference tables but were absent from `account_fact` claims until the current producer/backfill branch.
- Stakeholder/relationship insight claim types exist, but production evidence is not consistently present as active stakeholder claims.
- Meeting participation exists, but the runtime has no generic participation summary producer.
- Older meeting evidence is split between normalized attendee joins and raw attendee JSON, so a complete producer needs a normalization/read strategy that can use both safely.
- Actions are heavily populated, but only a small fraction appears as claim-backed commitments.
- Email and content links exist, but thread summaries are not currently produced into the `threads` section.
- Historical source-reference rows may still contain legacy placeholder source labels; the code cleanup does not rewrite existing encrypted DB rows.

## Gap Findings

### F1 — Account facts were schema-backed but not substrate-backed

The current branch addresses the first visible gap by adding a service-owned account fact producer and backfill. That is necessary, but it should be treated as the first instance of a broader producer class: sourced structured entity facts should reach the claim substrate with source attribution and trust recomputation.

### F2 — Entity context expansion is too narrow

The current claim reader expands only through hierarchy. For a real briefing, “related” must include an entity neighborhood:

- linked meetings
- attendees and people
- account/person/project relationships
- linked actions
- linked email threads and content
- parent/child hierarchy where applicable

This should be implemented as a generic neighborhood reader, not as account-specific joins inside MCP.

### F3 — Stakeholder evidence is split from account evidence

Stakeholder claim types are person-shaped, which is correct, but account briefings do not currently pull related person claims into the account envelope. This blocks prompts that ask for top stakeholders, influence, and engagement unless the host model independently reconstructs the graph from raw data.

The better fix is not to attach all stakeholder claims to accounts. The better fix is to let the entity-neighborhood reader surface related person evidence with explicit subject refs and inclusion reasons.

### F4 — Participation and attendance are evidence, not currently a runtime section

Meeting attendance can support stakeholder influence, relationship strength, engagement trends, and account health. The records exist, but the runtime does not expose a participation summary that the host model can use directly.

This should be a generic producer, for example `entity_participation_summary`, with rows shaped around:

- participant subject
- related entity subject
- touchpoint count and recency
- role when known
- inclusion reason
- source refs
- caveats for incomplete normalization

The producer should distinguish deterministic evidence from interpretive claims. A count can be evidence. “High influence” is a claim or synthesized assessment that should carry trust/provenance.

### F5 — MCP discards runtime evidence it already asks for

The account-status MCP handler asks for touchpoints and record entries, then omits them from the returned `assessment`. This makes Claude Desktop see a flatter DailyOS output than the runtime can provide.

This is a projection bug, not a substrate limitation.

### F6 — `threads` and `metadata_proposals` are envelope promises without producers

The envelope has sections for communication threads and metadata proposals, but both are empty today. The DB has email threads, content links, source refs, and structured fact discrepancies that can feed these sections.

The producer model should fill these sections generically:

- `entity_thread_summary` from linked email/content/thread evidence
- `entity_metadata_proposals` from sourced structured facts, corrections, and conflicts

### F7 — Health is meeting-only

`HealthStory` only exists for meeting prep status. Account/project/person health should not be hardcoded as account health. It should be a generic entity-health story derived from risks, wins, current state, participation, relationship coverage, work/open-loop state, freshness, and trust caveats.

### F8 — Trust aggregation is too fact-only

The envelope trust summary currently aggregates facts. Open loops and touchpoints are projected as unscored, and section-level caveats do not materially influence the top-level trust summary.

Trust recomputation work in the current branch is generic enough to build on, but the runtime still needs a section-aware trust/caveat model so derived sections do not appear more certain than their evidence supports.

### F9 — Actions and communications are under-promoted

Actions, captures, transcripts, email threads, linked entities, and content index rows are populated. Much of that is not claim-backed or summarized into runtime sections. This prevents DailyOS from showing its core advantage over retrieval-only systems: it has the user’s working state, not just documents.

## Recommended Next Work

### P0 — Add an entity-neighborhood reader

Create a generic read model that returns related subjects and evidence by relationship type:

- hierarchy
- explicit entity links
- meeting participation
- attendee/person relationship
- action/work relationship
- communication/content relationship

Each edge should carry source, recency, confidence, and inclusion reason. `get_entity_intelligence` should consume this reader before composing claims and sections.

### P1 — Add a participation summary producer

Produce a generic participation/attendance section or record-entry subtype that can support stakeholder and relationship questions across entity kinds. It should work for accounts, people, projects, and meetings without account-specific handler logic.

### P2 — Widen MCP account-status projection

Project the evidence the handler already requests:

- touchpoints
- record entries
- participation/stakeholder candidates when available
- section caveats
- provenance refs

For the immediate Glean-vs-DailyOS executive briefing test, this is likely the highest-leverage MCP change after the backfill.

### P3 — Add a generic entity-intelligence MCP tool

Keep `dailyos.read.account_status` as a question-shaped product tool, but add or prepare `dailyos.read.entity_intelligence` as the generic headless inspection tool over the same ability envelope. This gives future people/project/meeting prompts a first-class path without copying account-status logic.

### P4 — Fill thread and metadata proposal producers

Implement:

- `entity_thread_summary` from linked email/content/thread evidence
- `entity_metadata_proposals` from structured fact source refs, corrections, conflicts, and missing high-value fields

These should produce provenance-rich read-model sections first. Promote to durable claims only when the output is an assertion that should enter lifecycle/trust feedback.

### P5 — Generalize health story

Build entity-health story as an ability-owned synthesis over existing sections:

- facts
- risks and wins
- work/open loops
- participation and relationship coverage
- source freshness
- trust bands and contradictions

Avoid account-specific health math in the first pass. Let account-specific labels be surface copy, not the substrate model.

### P6 — Add production source-label cleanup/backfill

The source-label code cleanup does not rewrite old encrypted DB values. Add an idempotent service-owned maintenance/backfill path if those legacy labels need to be normalized for display or trust evaluation.

## Acceptance Criteria

For the executive-briefing prompt, DailyOS MCP should be able to return enough structured, human-readable material for a host model to produce:

- executive summary/current state from claim-backed intelligence
- commercial status from sourced structured facts
- current risks and wins with trust/provenance
- top stakeholder candidates from relationship/participation evidence
- a short influence assessment with caveats when evidence is thin
- two to three priorities from recommendations, open loops, risks, and work state
- explicit source/trust/freshness language that distinguishes DailyOS from flat retrieval

Generic runtime criteria:

- The same neighborhood reader works for account, project, person, and meeting subjects.
- Producers do not bypass `services/`.
- Durable assertions enter the claim substrate with provenance, temporal scope, sensitivity, and trust recomputation.
- Deterministic evidence can remain a read-model section when it is not itself a claim.
- MCP and WordPress consume abilities/runtime output, not parallel schema-specific adapters.
- Fixture coverage includes at least one non-account entity so the producer model cannot regress into account-only logic.

## Implementation Slice

Recommended first slice:

1. Add the entity-neighborhood reader with relationship edges and read-only tests.
2. Add participation summary composition to `get_entity_intelligence`.
3. Project touchpoints, record entries, and participation summaries in `dailyos.read.account_status`.
4. Add a fixture/eval that exercises the executive-briefing prompt shape without customer-specific test data.
5. Keep trust recomputation investigation on the todo list for all claim types, with section-aware caveats as a follow-on.

This slice should make the Glean-vs-DailyOS comparison materially fairer without creating an account-only producer architecture.
