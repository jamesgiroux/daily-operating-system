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
- Packet version: V1 doc-only.
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

## 5. Read-path shape

- Ability name: `list_open_loops`.
- Category: `Read`.
- Trust: `Trusted`.
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
- Operations array: declare read operations only.
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
- [ ] Claim-type allowlist is explicit and closed.
- [ ] Current V1 allowlist is `open_loop` plus `commitment`.
- [ ] The packet records that `follow_up`, `open_question`, and `blocker` are not current registry claim types.
- [ ] Implementation fails closed if a non-registered claim type is requested internally.
- [ ] Output shape is `OpenLoopsResult { loops, schema_version }`.
- [ ] Each returned loop has provenance field attribution.
- [ ] Each returned loop has source attribution with `source_asof` when knowable.
- [ ] Empty result has explicit provenance for `/loops`.
- [ ] No provider call exists in code or fixtures.
- [ ] Replay fixture is empty because the ability is pure Read.
- [ ] ADR-0106 parity is explicitly N/A and is not required by reviewers.
- [ ] Fixture 1: account with many open loops.
- [ ] Fixture 1 includes adjacent-subject rows that must not return.
- [ ] Fixture 2: person with a single loop.
- [ ] Fixture 3: entity with no loops returns empty result.
- [ ] Fixture set uses generic entities only.
- [ ] Fixture equality is exact for data.
- [ ] Fixture provenance diff is exact except for approved deterministic timestamps.
- [ ] Subject-fit is verified by the account fixture's adjacent-subject rows.
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

## 8. L0 reviewer panel

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

## 9. L0 acceptance gate

- All required reviewer panels approve.
- Required approval: `/plan-eng-review`.
- Required approval: `/codex challenge`.
- Approval must be unanimous.
- DOS-221 Linear ticket links to this packet.
- Target link path: `.docs/plans/v1.4.1-waves/W5-B-L0-packet.md`.
- Packet status remains V1 doc-only.
- Reviewers accept the wave-owned `abilities-runtime` path.
- Reviewers accept ADR-0106 N/A for this ability.
- Reviewers accept `open_loop` plus `commitment` as the V1 registered claim-type allowlist.
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
- Bundle-10 parity is an explicit gate.
- Parallel-run divergence threshold is <=1%.
- Parallel-run sampling is 20%.
- Tauri registry consumption is explicit.
- MCP registry consumption is explicit.
- Non-Live reader fail-closed behavior is explicit.
- No LLM, prompt, replay, or fingerprint proof is required.
- No mutation path is added.
- No ranking beyond recency is added.
- L0 can close only after reviewers accept the claim-type registry gap.

## 10. Out-of-scope

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

## 11. Why DOS-221 is the simplest of W5

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

## 12. Changelog

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
