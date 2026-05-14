# W5-B L0 Packet - DOS-221 migrate list_open_loops

## 1. Header

- Date: 2026-05-13.
- Project: v1.4.1 Abilities Runtime Completion.
- Wave: Wave 5 - Capability migrations.
- Issue: DOS-221 - Capability: Migrate `list_open_loops` to Abilities.
- Linear URL: https://linear.app/a8c/issue/DOS-221.
- Linear source: `mcp__linear__.get_issue(id="DOS-221", includeRelations=true, includeReleases=true)`.
- Working branch: `dos-280-w5-b-l0-prep`.
- Worktree path: `/Users/jamesgiroux/Documents/dailyos-repo/worktrees/dos-280-w5-b-l0-prep`.
- Packet version: V2 doc-only.
- Packet status: L0 authoring packet for review.
- Boundary: documentation-only.
- Boundary: no Rust edits.
- Boundary: no TypeScript edits.
- Boundary: no SQL migrations.
- Boundary: no fixture files.
- Boundary: no generated assets.
- Boundary: no runtime config changes.
- Boundary: no commit.
- Boundary: no push.
- Target implementation owner path from the wave plan: `src-tauri/abilities-runtime/src/abilities/list_open_loops/`.
- Linear issue path note: DOS-221 currently says Stage 1 is `src-tauri/src/abilities/read/list_open_loops.rs`.
- Wave authority note: `.docs/plans/v1.4.1-waves.md` assigns the new ability inside `abilities-runtime`, so this packet uses the wave-owned runtime path.
- Wave source: `.docs/plans/v1.4.1-waves.md`, Wave 5 section.
- W5-B done condition source: `.docs/plans/v1.4.1-waves.md` says parity vs legacy on bundle-10 fixture green and operations array declared.
- W5 merge gate source: `.docs/plans/v1.4.1-waves.md` says L0, L2, L3, L5, Suite P/E, MCP-bridge retest, and proof bundle.
- W5 pilot source: `.docs/plans/wave-W5/proof-bundle.md`.
- Read pilot source: `.docs/plans/wave-W5/DOS-218-plan.md`.
- ADR source: `.docs/decisions/0102-abilities-as-runtime-contract.md`.
- Provenance source: `.docs/decisions/0105-provenance-as-first-class-output.md`.
- Migration strategy source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md`.
- Prompt fingerprint checked file: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md`.
- ADR-0106 packet relevance: N/A for DOS-221 because this ability makes no provider call.
- Claim registry source: `src-tauri/abilities-runtime/src/abilities/claims.rs`.
- Current registry claim types relevant to loops: `commitment` and `open_loop`.
- Current registry gap: `follow_up`, `open_question`, and `blocker` are not registered `ClaimType` variants.
- Runtime pattern source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs`.
- Runtime composition pattern source: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/`.
- ServiceContext narrow-reader pattern source: `src-tauri/abilities-runtime/src/services/context.rs`.
- Existing bundle target: `src-tauri/tests/fixtures/bundle-10/`.
- Existing bundle-10 files verified in this worktree: `state.sql`, `inputs.json`, `expected_output.json`, `expected_provenance.json`, `metadata.json`, `clock.txt`, `seed.txt`, `provider_replay.json`, and `external_replay.json`.
- Only file intended for this turn: `.docs/plans/v1.4.1-waves/W5-B-L0-packet.md`.

## 2. Load-bearing user outcome

- `list_open_loops` gives the user a trustworthy list of unfinished work.
- The list is not a synthesized task plan.
- The list is a claim-backed read over active open-loop facts.
- The user outcome is follow-through without asking the model to remember.
- Daily readiness can ask one ability for open loops instead of duplicating query logic.
- Meeting prep can consume the same ability instead of keeping a local open-loop path.
- Tauri and MCP callers get the same list once registry cutover happens.
- Every loop is attributable to a claim row or legacy parity source.
- Every loop carries enough provenance for "why is this still open".
- Every loop is subject-scoped to the requested entity when an entity filter is supplied.
- Workspace-scope daily readiness can call the ability without an entity filter.
- Entity-scoped surfaces can call the ability with `entity_id`.
- The ability distinguishes no loops from unreadable state.
- Empty output is a valid result when the entity has no active loops.
- Missing fixture readers in Evaluate are not a valid empty result.
- Trust is product-facing because open loops drive user attention.
- Trust is `Trusted` because this is a pure Read ability with no LLM synthesis.
- Trusted does not mean the underlying claim is always true.
- Trusted means the ability output is an authenticated read of stored state with provenance.
- `source_asof` lets the user see whether a loop came from recent or stale evidence.
- Source attribution lets a user trace a loop to the meeting, claim, or action source.
- Sensitivity-aware rendering prevents restricted claim text from leaking through MCP or Agent reads.
- Subject-fit prevents adjacent accounts, people, projects, and meetings from sharing loops accidentally.
- Daily readiness uses this output as one chapter in the morning composition.
- That keeps daily readiness focused on composition instead of reinventing open-loop semantics.
- DOS-220 can render this output as the readiness "open loops" section.
- DOS-220 can preserve this ability's provenance as child provenance.
- DOS-220 should not invent a second ranking, synthesis, or filtering model for open loops.
- The simplest successful W5-B outcome is exact, explainable, registry-invoked open-loop parity.

## 3. Pre-work checklist

- [x] Read sibling packet W5-A: `worktrees/dos-280-w5-a-l0-prep/.docs/plans/v1.4.1-waves/W5-A-L0-packet.md`.
- [x] W5-A style note: numbered sections, short factual bullets, explicit source grounding, review gates at the end.
- [x] W5-A content note: DOS-220 composes `list_open_loops` once at workspace scope.
- [x] Read sibling packet W5-C: `worktrees/dos-280-w5-c-l0-prep/.docs/plans/v1.4.1-waves/W5-C-L0-packet.md`.
- [x] W5-C style note: records grounding gaps instead of fabricating missing symbols.
- [x] W5-C content note: DOS-221 is called out as the simplest W5 migration.
- [x] Read `.docs/plans/wave-W5/proof-bundle.md`.
- [x] Proof bundle note: DOS-218 and DOS-219 landed as the Read and Transform pilots.
- [x] Proof bundle note: cycle-7 preserved nine sensitivity channels for future waves.
- [x] Proof bundle note: private context fields and test-harness feature gates were load-bearing.
- [x] Proof bundle note: non-Live ServiceContext readers must fail closed.
- [x] Read DOS-218 plan at `.docs/plans/wave-W5/DOS-218-plan.md`.
- [x] DOS-218 note: Read abilities must prove subject attribution and `source_asof` when knowable.
- [x] DOS-218 note: exact parity is the bar for a Read migration unless Linear explicitly expands scope.
- [x] DOS-218 note: Tauri read/write divergence is a known risk when reads cut over without write migration.
- [x] Read `.docs/plans/v1.4.1-waves.md` Wave 5 section.
- [x] Wave note: W5-B owns `abilities/list_open_loops/` inside `abilities-runtime`.
- [x] Wave note: W5-B is done when bundle-10 parity is green and operations array is declared.
- [x] Fetch Linear DOS-221 with relations and releases through Linear MCP.
- [x] Linear note: DOS-221 has no recorded blockers.
- [x] Linear note: DOS-221 acceptance requires category Read, no LLM, trust Trusted.
- [x] Linear note: Stage 2 requires at least three fixtures.
- [x] Linear note: Stage 3 requires 20% sampling.
- [x] Linear note: Tauri and MCP consume via registry.
- [x] Inspect claim registry in `src-tauri/abilities-runtime/src/abilities/claims.rs`.
- [x] Registry note: `ClaimType::Commitment` persists as `commitment`.
- [x] Registry note: `ClaimType::OpenLoop` persists as `open_loop`.
- [x] Registry note: no `ClaimType` variants exist for `follow_up`, `open_question`, or `blocker`.
- [x] Inspect prepare-meeting open-loop output shape in `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs`.
- [x] Inspect ServiceContext narrow-reader pattern in `src-tauri/abilities-runtime/src/services/context.rs`.
- [x] Inspect bundle-10 metadata under `src-tauri/tests/fixtures/bundle-10/`.

## 4. Inherited contracts from v1.4.0 W5 pilots

| Contract | Applies to DOS-221 | DOS-221 handling |
|---|---:|---|
| ADR-0102 ability metadata | Yes | Register `list_open_loops` as category `Read`, no mutations, no provider call, registry-invoked. |
| ADR-0105 provenance wrapper | Yes | Provenance lives on `AbilityOutput<OpenLoopsResult>`, with field attribution per loop row. |
| ADR-0106 replay-key parity | N/A | No LLM invocation, no ReplayProvider lookup, no prompt fingerprint, and no `canonical_prompt_hash` to assert. |
| ADR-0112 Stage 0-5 migration | Yes | Implement ability, fixtures, parallel run, cutover, and old-path removal after evidence. |
| Cycle-7 nine-channel audit | Partly | Only channel 1, channel 2, and output rendering apply; channels 3-7 are N/A because there is no prompt. |
| `serde(skip_deserializing)` private fields | Mostly N/A | Public input has no private context field; if a fixture seam is added, it must be skipped and schemars-hidden. |
| Test-harness feature gating | Yes | Fixture readers and test-only helpers must not be exposed in release builds. |
| ServiceContext non-Live fail-closed reads | Yes | Evaluate must require injected open-loop claim readers and must not fall through to the live workspace DB. |
| Centralized sensitivity gate | Yes | Use shared sensitivity/render policy; do not invent per-ability sensitivity strings. |
| Subject-fit hard gate | Yes | Every returned loop must match the requested subject when `entity_id` is supplied. |
| `source_asof` propagation | Yes | Lift claim/source timestamps into `SourceAttribution` when knowable; warn when unknown. |
| No silent anomaly handling | Yes | Missing readers, unsupported subjects, and invalid claim types are hard errors or explicit diagnostics. |
| Tauri read/write split lesson | Yes | This ticket migrates reads only and must not cut over action/commitment writes. |
| Output-boundary audit follow-up | Limited | This ability audits its own output rendering; it does not claim every published surface is exhaustively audited. |
| No customer data in fixtures | Yes | Fixtures use generic names, domains, account ids, people, and projects only. |
| All mutations through services | Yes | The ability performs no mutation at all; old-path removal must still respect service boundaries. |

### Composition handoff gate

- DOS-221 itself has no prompt boundary.
- DOS-221's direct parent in W5 is DOS-220 `get_daily_readiness`.
- DOS-220 composes DOS-221 output and may feed selected loop facts into a rendered prompt.
- Composition handoff gate: any parent ability that feeds DOS-221 output into a prompt must audit channel 6 rendered prompt plus canonical JSON at the parent's boundary.
- Composition handoff gate: any parent ability that feeds DOS-221 output into a prompt must audit channel 7 template variables at the parent's boundary.
- Source cross-reference: W5-A packet section 4, "Inherited Contracts From v1.4.0 W5 Pilots", already inherits the nine-channel sweep for DOS-220.
- Source cross-reference: W5-A packet section 4, Contract 6, inherits the centralized `services/claims.rs` prompt-input sensitivity gate.
- Central gate name in code: `prompt_input_sensitivity_allowed`.
- Central gate source: `src-tauri/src/services/claims.rs:3461-3478`.
- DOS-221 fulfills its side of the handoff by stamping every `OpenLoop` with sensitivity and `subject_ref`.
- DOS-221 fulfills its side of the handoff by loading claim text only through the gated claim-read path.
- DOS-221 must not leak raw claim text through output-only diagnostics, provenance warnings, or debug fields.
- DOS-221 consumers must apply the centralized sensitivity gate before prompt rendering.
- DOS-221 consumers must not invent a per-parent sensitivity allowlist.
- Packet resolution: channel 6 and channel 7 are N/A inside DOS-221, but covered at the DOS-220 boundary when DOS-220 composes DOS-221.

## 5. Read-path shape

- Ability name: `list_open_loops`.
- Category: `Read`.
- Trust: `Trusted`.
- Metadata pattern source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:52-64`.
- DOS-218 pilot actor pattern: `allowed_actors = [User, Agent, System]`.
- Allowed actors packet decision: `allowed_actors = [User, Agent, System]`.
- Reviewer shorthand `[User, System, Agent]` is the same actor set, but implementation should preserve the DOS-218 pilot order for local consistency.
- MCP exposure enum source: `src-tauri/abilities-runtime/src/abilities/registry.rs:422-429`.
- MCP exposure packet decision: `mcp_exposure = Invocable`.
- Reason: `MetadataOnly` is discovery-only and does not register an invocation handler; DOS-221 acceptance requires MCP consumption through the registry.
- Scope registry source: `src-tauri/abilities-runtime/src/abilities/registry.rs:771-786`.
- Scope registry behavior: the runtime allowlist is seeded from each ability descriptor's `required_scopes`.
- Required scope packet decision: `required_scopes = ["read.open_loops"]`.
- Reason: no existing runtime scope named `read.open_loops` is present yet; the list-open-loops ability descriptor must introduce it through the registry union.
- Contract-first operations array source: `src-tauri/src/operations/mod.rs:126-147`.
- Operations packet decision: `operations = [{ name: "list-open-loops", category: Read, remote: true, requires_scope: Some("read.open_loops"), executor: read_list_open_loops_executor }]`.
- Operation input schema: `operations/schemas/list-open-loops.input.schema.json`.
- Operation output schema: `operations/schemas/list-open-loops.output.schema.json`.
- Provider use: none.
- LLM use: none.
- Prompt template: none.
- Prompt fingerprint: none.
- Replay fixture: empty.
- Public input: `ListOpenLoopsInput`.
- Required input field: `schema_version`.
- Optional input field: `entity_id`.
- Optional input field: `entity_type`.
- Supported entity types: `account`, `person`, `project`, and `meeting`.
- Entity filter shape should match the existing `get_entity_context` public pattern: explicit `entity_type` plus `entity_id`.
- Reason: raw Rust enum shape should not leak into Tauri or MCP JSON.
- Workspace-scope call: omit `entity_id` and `entity_type`.
- Entity-scope call: provide both `entity_type` and `entity_id`.
- Unsupported entity types are hard errors.
- Blank `entity_id` is a schema error.
- Blank `entity_type` with an `entity_id` is a schema error.
- Entity filter converts into `SubjectRef` internally.
- Subject kinds must match the registry's canonical subject set.
- Ownership gate order: when `entity_id` is supplied, verify subject ownership before applying claim-type or `subject_ref` filters.
- Ownership gate behavior: an entity from another workspace is a hard error, not an empty result.
- Ownership gate code name requested by reviewers: `AbilityError::SubjectNotOwned`.
- Existing code convention: `AbilityErrorKind::HardError(String)` exists; no enum variant named `SubjectNotOwned` exists in this worktree.
- Packet pin: implement as `AbilityError::SubjectNotOwned` if the typed variant is added, otherwise as `AbilityErrorKind::HardError("subject_not_owned")`.
- The error must be fail-closed and must not return `OpenLoopsResult { loops: [] }`.
- Claim source: `intelligence_claims` active rows.
- Active filter: `claim_state = 'active' AND surfacing_state = 'active'`.
- Surface filter: respect surface-specific dismissal when the read is for a named surface.
- Claim-type allowlist confirmed from codebase: `open_loop` and `commitment`.
- `open_loop` is registered as `ClaimType::OpenLoop`.
- `commitment` is registered as `ClaimType::Commitment`.
- `follow_up` is not a registered claim type in the current runtime registry.
- `open_question` is not a registered claim type in the current runtime registry.
- `blocker` is not a registered claim type in the current runtime registry.
- DOS-221 must not query unregistered claim type strings.
- If Linear expects `follow_up`, `open_question`, or `blocker` as first-class claim types, that is a separate registry change before implementation.
- V1 filter resolution: include `open_loop` and `commitment`; treat legacy follow-up/question/blocker wording as source semantics, not claim_type strings.
- Sort order: deterministic recency first.
- Recency key: prefer `source_asof`, then `observed_at`, then `created_at`.
- Ties use stable `claim_id` ordering.
- Ranking beyond recency is out of scope.
- Revoked-source gate order: load active claims, apply the centralized sensitivity/render gate, then resolve every loop `source_ref`.
- Revoked-source behavior: omit any loop whose primary source is revoked.
- Revoked-source behavior: do not surface the loop as masked text in `Vec<OpenLoop>`.
- Revoked-source behavior: count the omission in the `AbilityOutput` envelope so callers can detect coverage degradation.
- Reviewer-requested warning name: `ProvenanceWarning::SourceRevoked`.
- Existing provenance convention: `ProvenanceWarning::Masked { reason: MaskReason::SourceRevoked }` and `ProvenanceMaskReason::SourceRevoked { data_source }` already exist.
- Packet pin: implementation may add `ProvenanceWarning::SourceRevoked`; if it keeps the current enum shape, the envelope counter key must still be `source_revoked`.
- Revoked Glean behavior: a loop backed primarily by a revoked Glean source is omitted and increments the `source_revoked` provenance warning count.
- Output envelope: `AbilityOutput<OpenLoopsResult>`.
- Linear output shape: `OpenLoopsResult { loops: Vec<OpenLoop>, schema_version: SchemaVersion }`.
- User-requested shorthand: `Vec<OpenLoop>` with provenance per item.
- Packet resolution: data contains `loops: Vec<OpenLoop>`; provenance per item lives in the wrapper, not inline on each loop.
- Each `OpenLoop` should include a stable id.
- Each `OpenLoop` should include subject.
- Each `OpenLoop` should include loop kind.
- Each `OpenLoop` should include title or description.
- Each `OpenLoop` should include owner when present.
- Each `OpenLoop` should include due date or target date when present.
- Each `OpenLoop` should include status or lifecycle state when present.
- Each `OpenLoop` should include `source_asof` when knowable.
- Each `OpenLoop` should include `claim_type` for display/debug parity.
- Every returned row has field attribution.
- Every returned row has a direct source attribution to its claim row.
- Empty result attributes `/loops` as a constant empty list for the requested subject.
- Direct DB handles must not be passed into ability code.
- Ability code reads through a narrow ServiceContext handle.
- If no narrow reader exists yet, implementation adds one following `EntityContextClaimReadHandle`.
- Non-Live without an injected reader returns `FixtureReaderRequired` or equivalent hard error.
- No DB writes.
- No claim writes.
- No action writes.
- No signal emissions.
- No Tauri event emissions.
- No file writes.
- No external service calls.
- Operations array: declare the single read operation `list-open-loops` only.
- Composition: none.
- Downstream composition: DOS-220 calls this ability for daily readiness.

## 6. Acceptance criteria

- Source: Linear DOS-221 `mcp__linear__.get_issue` response.
- Verbatim Linear acceptance criteria:
- [ ] Registered with category=Read; no LLM invocation; trust=Trusted.
- [ ] Eval fixtures green (exact equality).
- [ ] Parallel-run divergence <=1%.
- [ ] Tauri and MCP consume via registry.
- Added L0 acceptance criteria:
- [ ] Ability lives under `src-tauri/abilities-runtime/src/abilities/list_open_loops/`.
- [ ] Wave-owned path supersedes Linear's older `src-tauri/src/abilities/read/list_open_loops.rs` path.
- [ ] Input supports workspace scope and optional entity scope.
- [ ] Entity-scope input supports `account`, `person`, `project`, and `meeting`.
- [ ] Ability metadata explicitly declares `allowed_actors = [User, Agent, System]`.
- [ ] Ability metadata explicitly declares `mcp_exposure = Invocable`.
- [ ] Ability metadata explicitly declares `required_scopes = ["read.open_loops"]`.
- [ ] Ability metadata declares a contract-first operations array entry for `list-open-loops`.
- [ ] Claim-type allowlist is explicit and closed.
- [ ] Current V2 allowlist is `open_loop` plus `commitment`.
- [ ] The packet records that `follow_up`, `open_question`, and `blocker` are not current registry claim types.
- [ ] Implementation fails closed if a non-registered claim type is requested internally.
- [ ] Output shape is `OpenLoopsResult { loops, schema_version }`.
- [ ] Each returned loop has provenance field attribution.
- [ ] Each returned loop has source attribution with `source_asof` when knowable.
- [ ] Each returned loop carries sensitivity and `subject_ref` for downstream prompt-boundary filtering.
- [ ] Empty result has explicit provenance for `/loops`.
- [ ] Cross-tenant entity input fails before claim-type or `subject_ref` filtering.
- [ ] Cross-tenant entity input returns `AbilityError::SubjectNotOwned` or `AbilityErrorKind::HardError("subject_not_owned")`, not an empty loop list.
- [ ] Revoked-source loops are omitted from `Vec<OpenLoop>`.
- [ ] Revoked-source omissions increment `ProvenanceWarning::SourceRevoked` coverage degradation, using the existing `Masked { SourceRevoked }` convention if the explicit variant is not added.
- [ ] No provider call exists in code or fixtures.
- [ ] Replay fixture is empty because the ability is pure Read.
- [ ] ADR-0106 parity is explicitly N/A and is not required by reviewers.
- [ ] Fixture 1: account with many open loops.
- [ ] Fixture 1 includes adjacent-subject rows that must not return.
- [ ] Fixture 2: person with a single loop.
- [ ] Fixture 3: entity with no loops returns empty result.
- [ ] Fixture 4: `bundle-10-cross-tenant`.
- [ ] Fixture 4 uses a well-formed `entity_id` for a subject owned by another workspace.
- [ ] Fixture 4 seeds at least one active `open_loop` for that other-workspace subject.
- [ ] Fixture 4 expects a typed hard error, not `Vec<OpenLoop>` emptiness.
- [ ] Revoked-Glean fixture variant seeds an active loop whose primary Glean source is later revoked.
- [ ] Revoked-Glean fixture variant expects the loop omitted and the `source_revoked` warning counter incremented.
- [ ] Fixture set uses generic entities only.
- [ ] Fixture equality is exact for data.
- [ ] Fixture provenance diff is exact except for approved deterministic timestamps.
- [ ] Subject-fit is verified by the account fixture's adjacent-subject rows.
- [ ] Bundle-10 fixture presence is a Stage-0 gate.
- [ ] Bundle-10 parity green per W5-B done condition.
- [ ] Parallel-run sampling is 20% per Linear DOS-221 Stage 3.
- [ ] Parallel-run divergence threshold is <=1%.
- [ ] Parallel-run comparison uses exact structured equality because this is Read.
- [ ] Cutover does not remove old path until Stage 4 evidence exists.
- [ ] Tauri invocation goes through the ability registry.
- [ ] MCP invocation goes through the ability registry.
- [ ] MCP-bridge retest confirms subject-ownership and sensitivity policy.
- [ ] No direct DB writes from ability code.
- [ ] No command-handler mutation path changes in this ticket.
- [ ] No ranking heuristic beyond recency.
- [ ] No cross-entity aggregation.

### Channel audit

| Channel | Applies? | DOS-221 judgment |
|---|---:|---|
| 1. Subject-ref claims via `load_claims_active` | Yes | This is the main read path; active lifecycle, sensitivity-band, and render policy must gate returned claim text. |
| 2. Source-ref claims via `load_claims_active_by_source_ref` | Yes, narrow | Applies only as subject-fit/provenance validation if source refs are expanded; it must not broaden the requested subject. |
| 3. `snapshot.claims -> EvidenceSource` mapping | N/A | No meeting snapshot and no prompt evidence source mapping. |
| 4. Composed `get_entity_context` children | N/A | `list_open_loops` composes no child ability. |
| 5. Prebuilt private/eval seam | N/A | Public input should not accept prebuilt loop context; test readers are injected through ServiceContext. |
| 6. Rendered prompt plus canonical JSON inputs | N/A | No prompt and no canonical prompt JSON. |
| 7. Template variables | N/A | No prompt template. |
| 8. Output-only provenance fields | Yes | Source refs, field paths, trust bands, and "About this" rendering must be safe and subject-scoped. |
| 9. Non-claim prompt data | N/A | No prompt-input data exists; deterministic filters are not sent to a provider. |

### Composition handoff gate

- DOS-221 channel audit closes only this ability's own boundary.
- Channels 6 and 7 become live when a parent composes DOS-221 output into a prompt.
- DOS-220 W5-A section 4 already inherits the nine-channel sweep and centralized sensitivity gate.
- DOS-221 handoff requirement: every `OpenLoop` includes sensitivity and `subject_ref`.
- DOS-221 handoff requirement: consumers must call the centralized `services/claims.rs` prompt-input sensitivity gate before rendering prompts.
- DOS-221 handoff requirement: raw claim text must enter parent prompts only through the gated load path.
- Acceptance proof lives at the parent boundary, starting with DOS-220.

## 7. Linear dependency edges

- Linear source: `mcp__linear__.get_issue(id="DOS-221", includeRelations=true, includeReleases=true)`.
- DOS-221 has no Linear-recorded blockers.
- Linear relation source: DOS-221 response `relations.blockedBy=[]`.
- DOS-221 blocks no issue in Linear.
- Linear relation source: DOS-221 response `relations.blocks=[]`.
- DOS-221 has no Linear-recorded related issues.
- Linear relation source: DOS-221 response `relations.relatedTo=[]`.
- Project: `v1.4.1 - Abilities Runtime Completion`.
- Status at read time: Backlog.
- Priority at read time: High.
- Stage 0 substrate readiness is inherited from v1.4.1 W5 preconditions.
- Evidence checked: W5 proof bundle says DOS-218 and DOS-219 reached Cycle-8 L2 APPROVE.
- Evidence checked: W5 proof bundle says the Read pilot and Transform pilot are ship-ready.
- Local wave plan says W5 migrations run in parallel.
- Local wave plan assigns DOS-221 to W5-B.
- Local wave plan assigns DOS-220 to W5-A.
- Local wave plan assigns DOS-222 to W5-C.
- Downstream dependency: DOS-220 daily readiness composes `list_open_loops`.
- Source: W5-A sibling packet says `list_open_loops` is invoked once at workspace scope.
- Source: ADR-0102 daily-readiness trace includes one `list_open_loops` child.
- Execution note: DOS-221 can land before DOS-220 and provide the clean child ability.
- Execution note: if DOS-220 lands first, it should use a declared fallback until DOS-221 cutover.
- Execution note: DOS-221 does not depend on DOS-222.
- Execution note: DOS-221 should not import risk-shift trajectory logic.
- Recommended W5 order: DOS-221 first, then DOS-220, then DOS-222.
- Reason: DOS-221 is pure Read, has no LLM, and gives DOS-220 a stable child.

## 8. Stage-0 and fixture gates

- Stage-0 gate: `src-tauri/tests/fixtures/bundle-10/` must exist before implementation work starts.
- Current worktree status: the directory exists.
- Stage-0 gate: bundle-10 must contain the standard harness files `state.sql`, `inputs.json`, `expected_output.json`, `expected_provenance.json`, `metadata.json`, `clock.txt`, `seed.txt`, `provider_replay.json`, and `external_replay.json`.
- Current worktree status: all standard files are present.
- Stage-0 deliverable: bundle-10 exists at `src-tauri/tests/fixtures/bundle-10/` with N seeded rows covering four fixture scenarios plus the revoked-source variant plus the cross-tenant variant.
- Stage-0 deliverable: bundle-10 documents the fixture scenarios in `metadata.json`.
- Stage-1 deliverable: if bundle-10 is missing in a rebased implementation branch, W5-B authors it before coding the ability.
- Stage-1 deliverable: W5-B extends bundle-10 with N seeded rows covering the four fixture scenarios below plus revoked-source and cross-tenant variants.
- Packet decision: N is at least 6 seeded loop-bearing or guard-bearing rows, with more rows allowed for adjacent-subject contamination proof.
- Scenario 1: account with many open loops plus adjacent-subject rows.
- Scenario 2: person with a single open loop.
- Scenario 3: entity with no loops and explicit empty-result provenance.
- Scenario 4: `bundle-10-cross-tenant`, a well-formed other-workspace subject that must hard-error as `subject_not_owned`.
- Variant 5: revoked-Glean source, omitted from `Vec<OpenLoop>` with `source_revoked` warning count.
- Variant 6: cross-tenant other-workspace subject has at least one active `open_loop`, proving the hard error is not a no-rows coincidence.
- Stage-0 gate failure: missing bundle-10 blocks W5-B implementation until fixture presence is restored.
- Stage-1 gate failure: missing scenario coverage blocks L1.

## 9. L0 reviewer panel

- Required reviewer: `/plan-eng-review`.
- `/plan-eng-review` focus: ability registration metadata.
- `/plan-eng-review` focus: path conflict between Linear and wave plan.
- `/plan-eng-review` focus: claim-type allowlist resolution.
- `/plan-eng-review` focus: optional entity filter schema.
- `/plan-eng-review` focus: ServiceContext reader seam.
- `/plan-eng-review` focus: exact fixture equality.
- `/plan-eng-review` focus: bundle-10 parity.
- `/plan-eng-review` focus: 20% sampling and <=1% divergence.
- `/plan-eng-review` focus: Tauri/MCP registry consumption.
- Required reviewer: `/codex challenge`.
- `/codex challenge` focus: unregistered claim-type drift.
- `/codex challenge` focus: wrong-subject open loops.
- `/codex challenge` focus: confidential or user-only claim text leaking into MCP output.
- `/codex challenge` focus: empty result masking a missing reader.
- `/codex challenge` focus: recency sorting hiding exact parity divergence.
- `/codex challenge` focus: old Tauri path continuing after registry cutover.
- `/codex challenge` focus: "simple Read" scope creep into ranking or synthesis.
- Not required: `/cso`.
- `/cso` rationale: DOS-221 is pure Read, no LLM boundary, no write path, no external call, and no new trust-boundary mutation.
- `/cso` caveat: add `/cso` if implementation changes MCP auth policy, sensitivity render policy, claim writer code, migrations, filesystem access, or any mutation path.
- Optional reviewer: `/plan-devex-review`.
- `/plan-devex-review` trigger: only if MCP schema/discovery ergonomics change beyond registry consumption.
- Optional reviewer: `/plan-design-review`.
- `/plan-design-review` trigger: only if a new visual "open loops" surface is designed.

## 10. L0 acceptance gate

- All required reviewer panels approve.
- Required approval: `/plan-eng-review`.
- Required approval: `/codex challenge`.
- Approval must be unanimous.
- DOS-221 Linear ticket links to this packet.
- Target link path: `.docs/plans/v1.4.1-waves/W5-B-L0-packet.md`.
- Packet status remains V2 doc-only.
- Reviewers accept the wave-owned `abilities-runtime` path.
- Reviewers accept the explicit ability metadata tuple: `allowed_actors`, `mcp_exposure`, `required_scopes`, and operations array.
- Reviewers accept ADR-0106 N/A for this ability.
- Reviewers accept `open_loop` plus `commitment` as the V2 registered claim-type allowlist.
- Reviewers explicitly acknowledge that `follow_up`, `open_question`, and `blocker` are not current registry claim types.
- Reviewers decide whether missing claim types require a separate Linear issue.
- Channel audit is complete.
- Channel audit confirms channels 3-7 are N/A.
- Channel audit confirms output rendering remains in scope.
- Fixtures are enumerated.
- Fixture 1: account with many loops and adjacent-subject contamination.
- Fixture 2: person with a single loop.
- Fixture 3: entity with no loops.
- Subject-fit proof is included in fixture 1 or a dedicated fourth fixture.
- Cross-tenant ownership proof is included in `bundle-10-cross-tenant`.
- Revoked-source omission proof is included in the revoked-Glean bundle-10 variant.
- Bundle-10 fixture presence is a Stage-0 gate.
- Bundle-10 parity is an explicit Stage-1/L1 gate.
- Parallel-run divergence threshold is <=1%.
- Parallel-run sampling is 20%.
- Tauri registry consumption is explicit.
- MCP registry consumption is explicit.
- Non-Live reader fail-closed behavior is explicit.
- No LLM, prompt, replay, or fingerprint proof is required.
- No mutation path is added.
- No ranking beyond recency is added.
- L0 can close only after reviewers accept the claim-type registry gap.

## 11. Out-of-scope

- LLM synthesis.
- Prompt templates.
- Prompt replay fixtures.
- ADR-0106 replay-key parity proof.
- Risk-shift detection.
- Daily-readiness composition code.
- Meeting-prep synthesis changes.
- New claim types.
- Claim registry expansion for `follow_up`.
- Claim registry expansion for `open_question`.
- Claim registry expansion for `blocker`.
- Commitment extraction.
- Commitment acceptance or rejection flows.
- Action creation.
- Action completion.
- Action dismissal.
- Claim mutation.
- Claim feedback mutation.
- Tauri write cutover.
- Old-path removal before Stage 4 evidence.
- Surface redesign.
- New UI copy.
- Ranking heuristics beyond deterministic recency.
- Priority scoring.
- Cross-entity aggregation.
- Workspace-level rollups that merge unrelated subjects.
- Provider calls.
- External service calls.
- File writes.
- Signal emission.
- Customer-specific fixture data.
- Commit or push.

## 12. Why DOS-221 is the simplest of W5

- DOS-221 is pure Read.
- DOS-221 has no LLM call.
- DOS-221 has no prompt.
- DOS-221 has no replay fixture.
- DOS-221 has no prompt fingerprint.
- DOS-221 has no Transform output.
- DOS-221 has no synthesis variance.
- DOS-221 has no judge scoring.
- DOS-221 has no composed child ability.
- DOS-221 has no trajectory dependency.
- DOS-221 has no deterministic risk algorithm.
- DOS-221 has no daily narrative.
- DOS-221 has no new write path.
- DOS-221 has no new migration requirement.
- DOS-221 has the smallest output schema.
- DOS-221 returns one list and one schema version.
- DOS-221's comparison is exact equality.
- DOS-221's parallel-run threshold is the standard Read threshold.
- DOS-221's main edge case is subject fit.
- DOS-221's second edge case is sensitivity-safe rendering.
- DOS-221's third edge case is missing-reader failure.
- DOS-220 is more complex because it composes three W5 abilities.
- DOS-220 may synthesize a daily narrative intro.
- DOS-220 must preserve child provenance.
- DOS-222 is more complex because it is Transform.
- DOS-222 consumes trajectory.
- DOS-222 invokes the provider.
- DOS-222 needs judge-scored output.
- DOS-222 has stale-source and revoked-source risk.
- Natural W5 order remains DOS-221, then DOS-220, then DOS-222.
- Shipping DOS-221 first gives W5 a small registry proof before composed readiness.

## 13. Changelog

- V1 2026-05-13 initial L0 packet.
- V1 grounded on Linear DOS-221 MCP response.
- V1 grounded on `.docs/plans/v1.4.1-waves.md`.
- V1 grounded on `.docs/plans/wave-W5/proof-bundle.md`.
- V1 grounded on `.docs/plans/wave-W5/DOS-218-plan.md`.
- V1 grounded on sibling W5-A packet.
- V1 grounded on sibling W5-C packet.
- V1 grounded on ADR-0102.
- V1 grounded on ADR-0105.
- V1 grounded on ADR-0112.
- V1 explicitly marks ADR-0106 replay-key parity N/A.
- V1 confirms `commitment` and `open_loop` are registered claim types.
- V1 confirms `follow_up`, `open_question`, and `blocker` are not registered claim types.
- V1 documents bundle-10 parity and 20% sampling.
- V1 documents no code changes.
- V1 documents no commit.
- V2 2026-05-14 cycle-1 fold: 6 findings from L0 panel cycle-1.
- V2 2026-05-14 Architect F1 -> ability metadata enum in section 5.
- V2 2026-05-14 Architect F2 -> bundle-10 fixture presence gate in section 8 and Stage-0.
- V2 2026-05-14 Architect F3 plus Codex F2 -> cross-tenant hard-error plus Fixture 4 in section 5, section 6, and section 8.
- V2 2026-05-14 Codex F1 -> Composition handoff gate audit subsection in section 4 and channel audit.
- V2 2026-05-14 Codex F3 -> revoked-Glean masking step plus `ProvenanceWarning::SourceRevoked` in section 5 read-path.
