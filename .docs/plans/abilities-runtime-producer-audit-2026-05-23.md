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

## Prompt Producer Inventory

The source of the thin MCP answer is not only the MCP projection. DailyOS already runs several Glean and local PTY producers that ask for richer analysis, but their outputs are unevenly normalized into substrate.

| Producer path | Provider | Output | Current persistence | Substrate behavior | Gap |
| --- | --- | --- | --- | --- | --- |
| `intelligence/glean_provider.rs::enrich_entity_parallel()` | Glean MCP `chat`, six dimension prompts | `IntelligenceJson` dimension slices with `itemSource` | Progressive `entity_assessment` snapshot and `intelligence.json` projection | Commits a subset through `commit_claim_shaped_intelligence_projection()`, emits Glean signals, promotes selected account facts | Prompt asks for source-rich output, but many fields lose `source_asof`, `source_ref`, and provenance when committed as claims |
| `intelligence/glean_provider.rs::enrich_entity_legacy()` | Glean MCP `chat`, monolithic prompt | `IntelligenceJson` | `entity_assessment` snapshot and export projection | Same projection path after parse/reconcile | Same source/provenance loss; debug response file is diagnostic only |
| `intel_queue.rs::run_parallel_enrichment()` | local Claude PTY, six dimension prompts | `IntelligenceJson` dimension slices | Progressive `entity_assessment` snapshot and export projection | Same projection path, source is `ai_enrichment` | Local transcript/file-derived assertions enter claims, but source refs are mostly opaque and field-level source dates are partial |
| `intel_queue.rs::run_enrichment_legacy()` | local Claude PTY, monolithic synthesis prompt | `IntelligenceJson` plus extracted keywords | `entity_assessment`, export projection, keyword rows | Same projection path | Legacy fallback can still produce broad synthesized claims without enough field-level provenance |
| `prepare/email_enrich.rs` | local Claude PTY extraction | email summary, sentiment, urgency, noise flag | email enrichment columns | No claim or signal producer found in this path | Email sentiment/urgency can affect relationship state but currently does not enter entity runtime directly |
| `risk_briefing.rs` | local Claude PTY section synthesis | `risk-briefing.json` | cached JSON report | Report-only; not a claim producer | This is downstream analysis unless its extracted findings are promoted through a services-owned claim/signal producer |
| `reports/*` and `workflow/deliver.rs` | local Claude PTY synthesis | report/prose artifacts | report or deliverable output | Mostly report-only | These should consume substrate; they should not become hidden producers unless a services-owned extraction path commits durable assertions |
| `reports/book_of_business.rs::prefetch_glean_portfolio_context()` | Glean MCP `chat` | free-text portfolio context | prompt input to report synthesis | No claim/signal producer | Valuable cross-account signals are used transiently and then disappear from substrate |
| `processor/transcript.rs` | local Claude PTY phases | meeting summary, risks, wins, decisions, commitments, dynamics | meeting metadata, captures, actions, key-advocate health, commitment/dynamics rows | Emits transcript outcomes, but no direct claim producer found | Transcript-derived account/person facts can update side tables without entering runtime claims |
| `processor/email_actions.rs` and `prepare/email_enrich.rs` | local Claude PTY extraction | commitments, summary, sentiment, urgency | actions and email enrichment columns | No claim/signal producer found | Email-derived work and relationship signals do not reliably feed entity intelligence |
| `context_provider/glean.rs` | Glean search / people search | `GleanEntityData`, field suggestions, documents | Glean cache, people/account links, org health JSON | Emits Glean document / field-suggestion signals, no claims found | Glean context can shape future prompts without creating durable substrate assertions |

The important distinction is producer intent. Enrichment producers should turn extracted facts and assessments into claims/signals with provenance. Report producers should be downstream consumers unless they explicitly run a services-owned extraction/promotion step. Otherwise DailyOS quietly creates another authority layer outside the abilities runtime.

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

### F10 — Glean/PTY prompts ask for provenance that projection does not fully keep

The Glean dimension prompt explicitly asks for `itemSource.source`, `itemSource.confidence`, `itemSource.sourcedAt`, and `itemSource.reference` for every array item. The current claim projection only maps `source_asof` for selected itemized arrays such as risks, wins, expansion signals, value delivered, open commitments, and stakeholder insights. Summary fields, current-state fields, recommendations, success metrics, contract context, agreement outlook, and company context generally commit with `source_asof: None`, `source_ref: None`, and empty provenance JSON.

This means the model may have seen recent Salesforce, Gong, Zendesk, Slack, P2, Drive, or transcript evidence, but the runtime cannot reliably prove or prioritize that evidence later.

### F11 — Several AI producers are analysis-only by accident

Email enrichment, risk briefing, book-of-business Glean prefetch, SWOT/account-health report synthesis, file enrichment, transcript processing, and workflow deliverable prompts all run local or Glean model calls. Some are correctly downstream reports. Others extract facts, sentiment, urgency, commitments, risks, or stakeholder context that should be signals or claims.

Each producer needs an explicit classification:

- `durable producer`: commits claims/signals through `services/`
- `read-model producer`: writes deterministic evidence or bounded summaries consumed by abilities
- `downstream report`: consumes substrate and writes an artifact, but does not create authority

Anything in the first two categories needs provenance and trust behavior. Anything in the third category must not be read back as runtime authority.

### F12 — Generated JSON/markdown still leaks into prompt-input paths

The workspace `CLAUDE.md` and entity README guidance now need to say generated JSON/markdown files are export projections, not authority. That removes the immediate Claude Desktop failure mode where the host model chooses `dashboard.json` over MCP/runtime.

However, code paths still read generated artifacts as prompt inputs or cached surface state, including meeting context dashboard reads, the legacy MCP daily briefing JSON reader, cached `risk-briefing.json`, and skip-today `_today/data/intelligence.json`. These paths need a follow-up allowlist: migration/backfill/import/export reads are allowed; runtime/MCP/prompt-input reads should use DB/runtime services unless explicitly operating in file-artifact mode.

### F13 — Queue and manual Glean refresh do not run the same producers

The Glean enrichment trigger matters today:

- Queue-worker Glean finalization emits the broader Glean signal set, including org health, support health, technical footprint, competitors, org changes, Gong summaries, Slack context, and champion-health signals.
- Manual Glean refresh promotes selected account facts into schema and `account_fact` claims.

Those paths should converge. The same Glean response should not produce different substrate depending on whether it came from background queue work or a manual app refresh. This is a producer orchestration bug, not a rendering bug.

### F14 — User corrections can leave old projection claims behind

User-facing correction paths update intelligence snapshots and emit correction/curation signals, but do not consistently supersede, tombstone, or recompute the matching projection claims. Recommendation accept/reject paths can remove persisted recommendations while leaving old recommendation claims active.

For MCP/WP surfaces that read claims, that means corrected app state can still render stale claim-backed intelligence. Corrections must participate in the same claim lifecycle as enrichment output.

### F15 — Some provider writes still bypass services-owned claim/signal promotion

Glean product classification and several transcript/email/file processing paths write useful structured data to relational tables, captures, or action rows without an obvious services-owned claim/signal producer. Some of those writes may be correct deterministic state, but the audit needs an explicit rule: if a write changes the intelligence picture, it either emits deterministic evidence for the runtime read model or commits a durable claim/signal with provenance.

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

### P7 — Add producer classification and provenance gates

Inventory every Glean/local PTY call and mark it as durable producer, read-model producer, or downstream report. Add tests or lints that prevent new AI producers from persisting durable assertions without a services-owned claim/signal path, source attribution, temporal handling, and trust recomputation behavior.

For the main enrichment path, normalize item-level source metadata into claim provenance consistently:

- carry `itemSource.sourcedAt` into `source_asof` wherever available
- carry `itemSource.reference` into a renderable provenance/source-ref handle when allowed
- preserve source system labels as typed source data rather than free-text display strings
- add section caveats when a field is synthesized from multiple sources and cannot safely name one source date

### P8 — Replace generated-artifact prompt inputs with runtime context builders

Replace live prompt-input reads of `dashboard.json`, `dashboard.md`, `intelligence.json`, and cached briefing/risk artifacts with DB/runtime context builders. Keep generated files as export projections for portability and explicit user file-inspection workflows.

### P9 — Unify Glean finalization across triggers

Create a single services-owned Glean finalization path used by queue-worker enrichment and manual refresh. It should:

- persist the intelligence snapshot
- commit projection claims with provenance
- promote sourced structured facts where applicable
- emit Glean-specific signals
- enqueue trust/health recompute
- write generated exports only after the runtime state is committed

This should close the current queue-vs-manual asymmetry.

### P10 — Route corrections through claim lifecycle

When a user corrects intelligence, stakeholders, or recommendations, update the runtime snapshot and also supersede, tombstone, or recompute the corresponding claim records. The corrected state must be what MCP/WP see through `get_entity_intelligence`.

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

## 2026-05-24 Subagent Addendum — Legacy Surface Bypass Sweep

The initial audit focused on producer coverage. A follow-up subagent sweep expanded the scope to legacy reads and generated-artifact paths that can still bypass the abilities runtime.

### Method

Four read-only subagents split the audit by responsibility:

- raw SQLite/projection reads outside the runtime
- filesystem and generated prose/artifact paths
- MCP v1/v2 and Tauri surface contracts
- abilities-runtime producer coverage and live reader seams

The shared classification below separates acceptable deterministic inputs from authority bypasses.

### Confirmed Good

- MCP v2 `dailyos.read.account_status` invokes `get_entity_intelligence` through the runtime gateway. It is not reading generated JSON or legacy `entity_assessment` directly.
- `get_entity_intelligence` is now the right central envelope for MCP-safe entity output: facts, open loops, relationships, touchpoints, and record entries are composed through service-context readers.
- Account fact promotion exists for selected sourced account fields and is the right pattern to generalize.
- Relationship and participation evidence is partly available through the generic neighborhood reader. It already covers linked entities, account stakeholders, project members, meeting attendees, and participant counts.
- User feedback and tombstones are live in the claim services and suppress active claim readers; the problem is producer coverage and old projection surfaces, not a missing lifecycle substrate.

### Immediate Mechanical Findings

- MCP v2 local grants can outlive compiled handlers. If a tool existed in a previous binary or catalog state, `tools/list` could advertise a stale invocable grant even when no handler is registered. This produces confusing host behavior such as tool selection followed by an unavailable ability. The current branch now prunes stale local grants at startup and defensively intersects `tools/list` with the registered handler set.
- `dailyos.read.account_status` still describes `subject` as a name or handle while the handler currently treats it as an entity id. Subject resolution remains a W2 acceptance gap.
- MCP v1 remains a bypass surface by design: `get_briefing`, `search_meetings`, `search_content`, and project/person `query_entity` still read files, raw DB rows, or legacy projections. That is acceptable only while v1 is treated as legacy/debug and not the path for headless product validation.
- MCP v1 hides raw abilities from `tools/list`, but exact-name ability calls can still route to the bridge. That makes unadvertised ability names a callable bypass unless the call path also enforces the advertised-tool allowlist.
- MCP v1 also hides some static tools from `tools/list` while their handlers remain callable by exact name. Hidden static tools need the same fail-closed treatment as hidden abilities unless they are explicitly debug-only and unavailable to Claude Desktop.

### Authority Bypasses That Need Producer Work

The largest bypass class is not one table. It is generated analysis/prose that is still treated as input authority by other surfaces.

- Report and briefing composers read legacy projections and direct evidence tables, synthesize prose via PTY calls, then persist `reports.content_json` or briefing JSON. Examples: account health, EBR/QBR, SWOT, risk briefing, Book of Business, weekly impact, monthly wrapped, and workflow deliverables.
- `build_intelligence_context()` is the central legacy prompt bridge. It assembles account source refs, email signals, stakeholders, meetings, actions, entity context rows, and prior intelligence directly for enrichment/report prompts.
- Meeting prep still has a split authority path: `prep_context_json` is the primary UI read source, while `prep_frozen_json` remains an active export/cache and PTY enrichment target.
- Meeting detail post-meeting intelligence reads interaction dynamics, champion health, role changes, and captures directly into a user-visible intelligence panel.
- Transcript processing can use legacy call-summary intelligence as prompt input, then persists meeting outcomes and actions without a direct claim producer for all extracted conclusions.
- Email enrichment writes useful urgency, sentiment, and action-shaped data, but `get_entity_intelligence` has no generic email/signal reader yet.
- Success-plan suggestions can still fall back to `entity_assessment` fields when newer success-plan signal rows are absent.
- Content chat packages legacy entity intelligence beside facts, actions, meetings, and semantic matches.
- Generated artifacts are mostly export projections, but some live paths still read `_today/data/*.json`, `risk-briefing.json`, `dashboard.md`, or `dashboard.json` as prompt input or cached UI state.
- Context providers can still mutate durable intelligence. Glean context gathering writes org health and contact/relationship data that later feeds local context, but those writes do not consistently pass through claim promotion, signal propagation, or trust recompute.
- Progressive dimension enrichment writes snapshots through the legacy assessment path before finalization. It should be classified as an operational partial read model unless it goes through the same durable finalization path as completed enrichment.
- Provider-local Glean product classification and some stakeholder side writes still write useful intelligence outside the shared side-effect/finalization services.

### Acceptable Reads

Not every raw table read is a bypass. These are acceptable when they stay deterministic and feed a runtime producer or a non-authoritative export:

- list/index abilities over accounts, projects, people, meetings
- MCP v2 account-status service-context readers
- meeting metadata, meeting attendees, captures, and actions when used as evidence inputs
- deterministic health scoring inputs
- export, migration, backup, rebuild, and recovery paths
- compatibility legacy payloads ignored by the frontend ability-envelope mapper

### Producer Remediation Implications

The remediation should not teach MCP or Tauri to read report JSON, dashboard files, or account-shaped legacy projections. The service-oriented fix is to classify each generated-output path:

- `durable producer`: extracted assertions become claims/signals with provenance, temporal scope, sensitivity, lifecycle, and trust recomputation
- `read-model producer`: deterministic evidence becomes a bounded runtime section with caveats and source refs
- `downstream artifact`: report/prose output consumes the runtime and remains an artifact, never authority

Priority producer gaps after the subagent sweep:

1. Transcript/capture outcomes into generic claims or runtime evidence.
2. Email and generic signal events into entity touchpoints/threads.
3. Actions/open loops into claim-backed or provenance-safe runtime evidence for MCP.
4. Report/prose conclusions explicitly classified as artifacts unless promoted by a services-owned extraction step.
5. Meeting prep and daily briefing switched to runtime-backed context builders instead of legacy prompt bridges.
6. Project/person MCP rich answers routed through the same `get_entity_intelligence` path as accounts.
7. Glean context/provider-local side effects moved behind shared services-owned finalization and side-effect producers.
8. MCP v1 exact-call paths fail closed for unadvertised abilities and hidden static tools.

### Next Acceptance Criteria

- Claude Desktop sees only registered MCP v2 tools from the current binary.
- `dailyos.read.account_status` resolves subject names, slugs, and ids before runtime invocation.
- The executive-briefing eval uses runtime output, not legacy files or reports, and contains recent touchpoints, relationship participants, open loops, sourced commercial facts, and caveats.
- No new AI producer persists durable user-visible intelligence outside `services/` without a claim/signal/read-model classification.
- Generated JSON/markdown remains export-only except for explicit import/recovery workflows.

## 2026-05-25 Stranded Evidence Addendum

The L4 comparison exposed a second-order audit failure: some durable evidence was already present in legacy service tables, and the runtime reader even touched some of it, but the useful semantics were dropped before `get_entity_intelligence` reached MCP.

This is a broader class than one account field. The inventory now tracks both missing producers and "queried-but-discarded" evidence. A read-only production inventory confirmed the risk: legacy/evidence tables are materially populated, so fixture-level runtime success is not enough. Counts included thousands of action/source rows, email rows, meeting links, captures, source refs, signal events, and existing claims.

| Evidence class | Durable source | Runtime state | Current remediation |
| --- | --- | --- | --- |
| Structured account facts | account/source-reference rows | Promoted to `account_fact` claims by producer/backfill; migration 265 now records a one-time service-owned runtime evidence backfill request so existing local rows run through the same producer path after migrations | Keep producer/backfill; add eval checks that surfaced commercial facts come from claims, not direct schema reads |
| Core entity scalar facts | account, project, and person schema fields | Stable DB fields can disappear from MCP if no claim exists, or can render with weak freshness when `source_asof` is absent | Generalize sourced entity-field producers beyond account facts; runtime freshness falls back to `observed_at` when `source_asof` is missing |
| Entity assessment and health narrative | `entity_assessment`, `entity_quality`, `health_score_history` | Derived app summaries and health rows remain mostly outside `get_entity_intelligence` health, except where separately claim-backed | Add generic health/read-model producer; keep reports and cached prose downstream only |
| Account-team and stakeholder roles | account-stakeholder role rows | Relationship reader queried role but emitted generic stakeholder evidence, losing owner/RM/champion semantics | Preserve safe role categories in relationship edges and account-specific participant roles; prioritize explicit roles before generic associated links |
| Actions and commitments | action/open-loop rows, commitment sources | Tauri could see them, MCP intentionally dropped synthesized action evidence; commitment source provenance/trust is still thinned in open-loop projection | Expose bounded action evidence to MCP as `Internal` open-loop/commitment runtime evidence; follow-on should promote durable commitments into first-class claims or preserve source rows in the read model |
| Meeting/entity links | current linked-entity graph plus legacy meeting junctions | Legacy readers could miss current graph-only links or resurrect dismissed legacy links | Read current graph first and use legacy junctions only as fallback |
| Transcripts, captures, and meeting summaries | transcript/capture/outcome tables | Some paths write side tables and generated summaries without generic claim/runtime production | W3 producer work: commit extracted assertions or expose bounded read-model evidence with source refs and caveats |
| Email and content signals | email enrichment, linked content, indexed artifacts | Useful urgency/sentiment/context often remains outside entity runtime | W3 producer work: thread/touchpoint/read-model producers; reports stay downstream artifacts |
| Metadata proposals and review candidates | stakeholder suggestions, review queues, claim-review deferrals | `metadata_proposals` is still empty even when proposal-like rows exist elsewhere | Add generic metadata proposal producer with suppression/provenance state |
| Thread context | `thread_metadata`, `intelligence_claims.thread_id`, emails | `threads` section is empty; claims are not grouped by conversation context | Add `services::threads` runtime producer over claim-backed and linked communication evidence |
| Feedback and suppression controls | tombstones, feedback events, claim-surface dismissals, linking dismissals | Claim reads honor claim suppression, but non-claim read models apply bespoke filters and often drop suppression provenance | Centralize suppression hooks and carry suppression/exclusion caveats through section state |

Acceptance addition: a stranded-evidence audit is incomplete if it only asks "is there a table?" or "does a query run?" It must also verify the runtime carries the evidence's meaning, provenance, freshness, sensitivity, and suppression semantics through to the surface projection.
