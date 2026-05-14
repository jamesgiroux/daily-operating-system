# DOS-220 W5-A L0 Packet V1

## 1. Header

- Date: 2026-05-13.
- Project: v1.4.1 - Abilities Runtime Completion.
- Wave: 5 - Capability migrations.
- Issue: DOS-220 - Capability: Migrate `get_daily_readiness` to Abilities.
- Linear URL: https://linear.app/a8c/issue/DOS-220.
- Linear source: `mcp__linear__.get_issue(id="DOS-220", includeRelations=true, includeReleases=true)`.
- Working branch: `dos-280-w5-a-l0-prep`.
- Worktree path: `/Users/jamesgiroux/Documents/dailyos-repo/worktrees/dos-280-w5-a-l0-prep`.
- Packet status: V1 re-do; prior worktree was wiped before commit.
- Boundary: documentation-only. No Rust, TypeScript, SQL, fixture, generated asset, or runtime code changes.
- Only file intended for this turn: `.docs/plans/v1.4.1-waves/W5-A-L0-packet.md`.
- Commit boundary: do not commit; the user commits manually.
- Wave source: `.docs/plans/v1.4.1-waves.md:582-604` defines W5 as capability migrations and assigns W5-A to DOS-220.
- W5-A done condition source: `.docs/plans/v1.4.1-waves.md:588-592` requires full Provenance + Trust band, bundle-9 parity, and operations array declaration.
- W5 merge gate source: `.docs/plans/v1.4.1-waves.md:606-613` requires L0, L2, L3, L5, Suite P/E, MCP-bridge retest, and proof bundle.
- ADR source: `.docs/decisions/0102-abilities-as-runtime-contract.md:50-77` names `get_daily_readiness` in the Read ability module catalog.
- ADR category source: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97` defines category classification by call-graph effect.
- Provenance source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58` defines the envelope fields, including `sources`, `children`, and `field_attributions`.
- Prompt fingerprint source: requested path `.docs/decisions/0106-prompt-fingerprinting.md` is absent in this worktree.
- Prompt fingerprint checked path: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:1-8`.
- ADR-0105 companion link confirms the checked filename: `.docs/decisions/0105-provenance-as-first-class-output.md:9`.
- Migration strategy source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:21-49`.
- Post-W5 pilot source: `.docs/plans/wave-W5/proof-bundle.md:1-11`.
- Runtime pattern source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:25-65`.
- Legacy readiness exact-symbol search: `rg -n "get_daily_readiness" src-tauri` returned no matches in this worktree.
- Legacy daily briefing command anchor: `src-tauri/src/commands/core.rs:23-32`.
- Legacy daily briefing registration anchor: `src-tauri/src/lib.rs:664-683`.
- Legacy dashboard service anchor: `src-tauri/src/services/dashboard.rs:599-649`.
- Legacy daily briefing orchestration anchors: `src-tauri/src/prepare/orchestrate.rs:902-919`, `:968-991`, `:2974-2978`, `:3102-3118`, `:3346-3386`.

## 2. Load-Bearing User Outcome

- Daily readiness becomes ability-composed rather than command-local or file-local orchestration.
- User outcome: the daily briefing can explain why every material item appeared.
- User outcome: "About this" can show field-level attribution for every brief field.
- User outcome: every meeting brief, risk shift, open loop, and trajectory-derived line has provenance.
- User outcome: depth compounds through children instead of bespoke one-off integrations.
- If `get_entity_context` improves trajectory depth, readiness improves without a daily-readiness-specific rewrite.
- If `prepare_meeting` improves attendee context, readiness inherits that detail for today's meetings.
- If `detect_risk_shift` improves trajectory interpretation, readiness inherits sharper overnight-change/risk language.
- If `list_open_loops` improves filtering, readiness inherits better workspace-level follow-through.
- ADR-0102 names this exact product effect: depth compounds automatically when entity enrichment improves composed abilities.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:472-477`.
- ADR-0102 also says app, MCP, and future surfaces get identical outputs through abilities.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:474-477`.
- ADR-0105 makes field-level attribution non-optional for all fields.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:201-241`.
- ADR-0105 makes child provenance visible through `children[]`, not hidden in call stacks.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:245-266`.
- The W5-A done condition requires full Provenance + Trust band.
- Source: `.docs/plans/v1.4.1-waves.md:588-592`.
- The proof bundle says the W5 pilots established claim-backed `get_entity_context` and `prepare_meeting` composition.
- Source: `.docs/plans/wave-W5/proof-bundle.md:33-47`.
- This packet treats "daily readiness" as the user-facing daily briefing/readiness surface.
- Reason: the exact symbol `get_daily_readiness` is not present in `src-tauri`, while `get_dashboard_data` and `prepare/orchestrate` are the current daily briefing/readiness implementation anchors.
- Source: `src-tauri/src/commands/core.rs:23-32`.
- Source: `src-tauri/src/services/dashboard.rs:599-649`.
- Source: `src-tauri/src/prepare/orchestrate.rs:2974-2978`.
- Source: `src-tauri/src/types.rs:1452-1482` for `DashboardData` and briefing callouts.
- Source: `src-tauri/src/types.rs:1504-1527` for `WeekOverview.readiness_checks`.
- Field-level user affordance contract: provenance lives once on `AbilityOutput<T>`, not inline on the domain data.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:141-182`.
- About-this rendering therefore reads the provenance tree attached to the ability output.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:56-58`.
- Trust outcome: the user sees `likely_current`, `use_with_caution`, or `needs_verification` from the trust/provenance layer rather than raw confidence prose.
- Source: W5-A done condition in `.docs/plans/v1.4.1-waves.md:588-592`.
- Source: ADR-0105 trust computation in `.docs/decisions/0105-provenance-as-first-class-output.md:103-158`.

## 3. Pre-Work

- Upstream artifact read: `.docs/plans/v1.4.1-waves.md`.
- Relevant lines: `.docs/plans/v1.4.1-waves.md:582-613`.
- Finding: W5 contains three capability migrations: DOS-220, DOS-221, DOS-222.
- Source: `.docs/plans/v1.4.1-waves.md:582-604`.
- Finding: W5-A owns `abilities/get_daily_readiness/` inside `abilities-runtime`.
- Source: `.docs/plans/v1.4.1-waves.md:588-592`.
- Finding: W5-B owns `abilities/list_open_loops/`.
- Source: `.docs/plans/v1.4.1-waves.md:594-598`.
- Finding: W5-C owns `abilities/detect_risk_shift/`.
- Source: `.docs/plans/v1.4.1-waves.md:600-604`.
- Upstream artifact read: `.docs/plans/wave-W5/proof-bundle.md`.
- Relevant lines: `.docs/plans/wave-W5/proof-bundle.md:1-11`.
- Finding: DOS-218 and DOS-219 landed together as W5 Read + Transform pilots.
- Source: `.docs/plans/wave-W5/proof-bundle.md:3-11`.
- Finding: proof bundle status is Cycle-8 L2 APPROVE, ship-ready, no material findings.
- Source: `.docs/plans/wave-W5/proof-bundle.md:3-4` and `:27`.
- Date note: the proof bundle date is 2026-05-06, not 2026-05-07.
- Source: `.docs/plans/wave-W5/proof-bundle.md:5`.
- Date note: `.docs/plans/v1.4.0-waves-amendments.md` is dated 2026-05-07 but is a protocol amendment, not the W5 proof bundle.
- Source: `.docs/plans/v1.4.0-waves-amendments.md:1-7`.
- Packet wording therefore records the checked evidence rather than fabricating a missing "Done 2026-05-07" line.
- Upstream artifact read: `.docs/plans/wave-W5/DOS-218-plan.md`.
- Finding: DOS-218 was the Read pilot and established entity-context ability parity.
- Source: `.docs/plans/wave-W5/DOS-218-plan.md:18-38`.
- Finding: DOS-218 required subject attribution and `source_asof` when knowable.
- Source: `.docs/plans/wave-W5/DOS-218-plan.md:24-36`.
- Finding: DOS-218's implementation plan kept UI write/read cutover out of scope.
- Source: `.docs/plans/wave-W5/DOS-218-plan.md:36`, `:88-96`.
- Upstream artifact read: `.docs/plans/wave-W5/DOS-219-plan.md`.
- Finding: DOS-219 was the Transform pilot and composes `get_entity_context`.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:21-33`, `:37-45`.
- Finding: DOS-219 separates pure Transform output from publish/cache/maintenance behavior.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:29-31`, `:48-52`.
- Finding: DOS-219 about-this shape uses top-level provenance field attribution.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:56-58`.
- Upstream artifact read: `.docs/decisions/0102-abilities-as-runtime-contract.md`.
- Finding: `get_daily_readiness` is listed as a Read ability.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:50-60`, `:83-86`.
- Upstream artifact read: `.docs/decisions/0105-provenance-as-first-class-output.md`.
- Finding: provenance requires sources, children, prompt fingerprint, field attribution, trust, and warnings.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`.
- Upstream artifact requested: `.docs/decisions/0106-prompt-fingerprinting.md`.
- Finding: that exact file is missing in this worktree.
- Substitute checked artifact: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md`.
- Finding: ADR-0106 defines `PromptFingerprint` and `canonical_prompt_hash`.
- Source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:18-48`.
- Upstream artifact read: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md`.
- Finding: Stage 0-5 migration flow, parallel-run, cutover, fallback, and old-path removal are specified.
- Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:21-49`.
- Finding: `get_daily_readiness` is third in the capability migration order.
- Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:112-126`.
- Upstream source read: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs`.
- Finding: post-DOS-349 ability lives in `abilities-runtime`, not the older ADR path.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:25-65`.
- Finding: it accepts `ContextDepth::Deep`.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:28-42`, `:147-162`.
- Finding: it reads claims through `ctx.services().read_entity_context_claims`.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:72-82`.
- Finding: it reads trajectory when depth permits.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:88-89`, `:165-179`.
- Upstream exact-symbol search: `rg -n "get_daily_readiness" src-tauri`.
- Finding: no exact local symbol was found.
- Broader legacy anchors checked: `src-tauri/src/commands/core.rs:23-32`, `src-tauri/src/services/dashboard.rs:599-649`, and `src-tauri/src/prepare/orchestrate.rs:2974-2978`.
- Upstream status: DOS-221 and DOS-222 are sibling W5 migrations.
- Source: `.docs/plans/v1.4.1-waves.md:594-604`.
- Upstream status: DOS-215 temporal primitives are a W2-C artifact in this v1.4.1 plan.
- Source: `.docs/plans/v1.4.1-waves.md:95-98`, `:330-332`.
- Additional git evidence: `46a32e8b DOS-215 W2-C: temporal primitives EngagementCurve + RoleProgression (#244)`.
- Upstream status: DOS-280 W4-B canonicalization is in the current branch head.
- Source: `.docs/plans/v1.4.1-waves.md:479-481`.
- Additional git evidence: `40aafe2a DOS-280 W4-B.2: ADR-0131 Phase B+C - v2 canonicalization cutover (#271)`.
- This packet is grounded on the branch head that includes PR #271.
- Source: `git log --oneline --decorate --max-count=30` at `40aafe2a`.

## 4. Inherited Contracts From v1.4.0 W5 Pilots

- Contract 1: ADR-0106 canonical replay key parity is inherited.
- Source: `.docs/plans/wave-W5/proof-bundle.md:44-48`.
- The replay lookup and provenance fingerprint must use the same `canonical_prompt_hash`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:46-47`.
- ADR-0106 says the hash input includes template id, template version, canonical template bytes, canonical JSON inputs, provider kind, model, temperature, top_p, and seed.
- Source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:40-48`.
- ADR-0106 says replay is keyed by `canonical_prompt_hash`.
- Source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:82-90`.
- DOS-220 must assert parity between ReplayProvider lookup and PromptFingerprint provenance.
- Review target: a fixture should fail if the replay provider and provenance builder compute different hashes.
- Contract 2: Cycle-7 sensitivity sweep is inherited as a class-pattern control.
- Source: `.docs/plans/wave-W5/proof-bundle.md:50-65`.
- Cycle-7 channel enumeration is preserved for the next wave.
- Source: `.docs/plans/wave-W5/proof-bundle.md:75-87`.
- Channel 1: subject-ref claims via `load_claims_active`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:79`.
- Channel 2: source-ref claims via `load_claims_active_by_source_ref`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:80`.
- Channel 3: `snapshot.claims -> EvidenceSource` mapping.
- Source: `.docs/plans/wave-W5/proof-bundle.md:81`.
- Channel 4: composed `get_entity_context` children -> `entity_contexts`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:82`.
- Channel 5: prebuilt `PrepareMeetingInput.context.evidence` private/eval seam.
- Source: `.docs/plans/wave-W5/proof-bundle.md:83`.
- Channel 6: rendered prompt + canonical JSON inputs.
- Source: `.docs/plans/wave-W5/proof-bundle.md:84`.
- Channel 7: template variables in `prepare_meeting_prep.v1.txt`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:85`.
- Channel 8: output-only provenance fields.
- Source: `.docs/plans/wave-W5/proof-bundle.md:86`.
- Channel 9: non-claim prompt data.
- Source: `.docs/plans/wave-W5/proof-bundle.md:87`.
- DOS-220 adds a second prompt boundary: the daily-readiness narrative intro.
- Therefore the same nine-channel audit applies to daily-readiness prompt inputs and output provenance.
- Contract 3: `serde(skip_deserializing)` on private context fields is inherited.
- Source: `.docs/plans/wave-W5/proof-bundle.md:38-42`.
- The W5 pilot used it to prevent MCP/Agent callers from injecting fabricated context JSON.
- Source: `.docs/plans/wave-W5/proof-bundle.md:40`.
- DOS-220 public input must not accept preassembled daily context, child evidence, provenance, prompt context, or private fixture seams from callers.
- Public input should contain only caller-safe selectors and schema version.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:141-182`.
- Contract 4: test-harness Cargo feature gating is inherited.
- Source: `.docs/plans/wave-W5/proof-bundle.md:38-42`.
- The prior wave gated command test helpers out of release builds and used `compile_error!` to hard-fail release exposure.
- Source: `.docs/plans/wave-W5/proof-bundle.md:41`.
- DOS-220 fixtures and shims must not expose fixture readers, replay capture helpers, or private context builders in release builds.
- Contract 5: ServiceContext non-Live reads fail closed when a fixture reader is missing.
- Source: `.docs/plans/wave-W5/proof-bundle.md:42`.
- Current ServiceContext keeps narrow readers for entity context claims and prepare-meeting context.
- Source: `src-tauri/abilities-runtime/src/services/context.rs:837-840`, `:860-873`, `:912-920`.
- Current read methods return `missing_reader_error(...)` if no injected reader exists.
- Source: `src-tauri/abilities-runtime/src/services/context.rs:1090-1119`, `:1180-1191`.
- The missing-reader error is `ServiceError::FixtureReaderRequired`.
- Source: `src-tauri/abilities-runtime/src/services/context.rs:1224-1229`.
- DOS-220 Evaluate/Simulate mode must never fall through to `ActionDb::open()` on the user's real workspace.
- Contract 6: centralized sensitivity gate in `services/claims.rs` is inherited.
- Source: `.docs/plans/wave-W5/proof-bundle.md:58-65`.
- Current claim service allows only Public/Internal at LLM prompt-input boundaries.
- Source: `src-tauri/src/services/claims.rs:3461-3478`.
- Current abilities-runtime mirror has the same prompt-input helper.
- Source: `src-tauri/abilities-runtime/src/types.rs:186-202`.
- DOS-220 must not add per-ability ad hoc sensitivity filters that drift from this helper.
- Contract 7: subject-fit hard error on field/output subject mismatch is inherited.
- Source: `.docs/plans/wave-W5/DOS-218-plan.md:34`.
- DOS-219 plan repeats that subject-fit is a hard gate for claim-bearing output.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:70-72`.
- Current `get_entity_context` converts and validates subject refs before attribution.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:356-390`.
- Current `get_entity_context` rejects unsupported claim subjects.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:492-508`.
- DOS-220 must hard-error on deliberate field/output subject mismatch fixtures.
- Contract 8: `source_asof` propagation through composition is inherited.
- Source: `.docs/plans/wave-W5/DOS-218-plan.md:24`, `.docs/plans/wave-W5/DOS-219-plan.md:21`.
- ADR-0105 says `source_asof` is must-populate-when-knowable.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:391-420`.
- ADR-0105 says the builder should lift timestamps and emit `SourceTimestampUnknown` on fallback.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:435-446`.
- Current `get_entity_context` passes `source_asof` from trajectory direct source refs.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:329-347`.
- Current `get_entity_context` builds source attribution with parsed claim timestamp and `source_asof`.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:522-537`, `:588-617`.
- DOS-220 provenance flattening must preserve source timestamps from direct sources and children.
- Contract 9: no silent anomaly handling.
- ADR-0102's error-handling amendment forbids log-and-proceed for material anomalies.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:519-545`.
- Default composition failure is hard error; optional soft degradation must be declared explicitly.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:537-544`.
- DOS-220 must define which children are required and which, if any, are optional.
- Contract 10: Tauri read/write split lesson is inherited.
- Source: `.docs/plans/wave-W5/proof-bundle.md:67-69`.
- Prior Tauri read cutover broke because write path remained legacy.
- Source: `.docs/plans/wave-W5/proof-bundle.md:69`.
- DOS-220 should not silently cut over daily briefing writes/projections without a claim/projection migration plan.
- Contract 11: output-boundary audit remains follow-up unless explicitly pulled in.
- Source: `.docs/plans/wave-W5/proof-bundle.md:102-108`.
- W5 enforcement was prompt-input boundary plus `get_entity_context` Agent read.
- Source: `.docs/plans/wave-W5/proof-bundle.md:107`.
- DOS-220 should not claim all published surfaces are audited unless the implementation performs that separate audit.

## 5. Composition Shape

- Linear DOS-220 direct-child contract: compose `prepare_meeting`, `detect_risk_shift`, and `list_open_loops`.
- Source: Linear DOS-220 description, section "Scope (per ADR-0112 §1)", Stage 1.
- Local wave plan confirms DOS-220 is W5-A and siblings are DOS-221 and DOS-222.
- Source: `.docs/plans/v1.4.1-waves.md:588-604`.
- ADR-0102 gives the trace example for `get_daily_readiness`: it invokes `prepare_meeting` three times, `detect_risk_shift` twice, and `list_open_loops` once.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:414-418`.
- Packet resolution: DOS-220 has three W5 migration children plus a support Read child family for `get_entity_context`.
- Why not hide the mismatch: the user-requested packet requires `get_entity_context Deep` for tracked account/attendee trajectory data.
- Direct composition family A: `prepare_meeting` per today's meetings.
- Category: Transform.
- Source: DOS-219 plan says `prepare_meeting` composes `get_entity_context` once per deduped subject.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:37-45`.
- Source: current ability exists at `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs`, found by source search.
- Direct composition family B: `detect_risk_shift` per tracked account.
- Category: Transform.
- Source: W5-C assignment in `.docs/plans/v1.4.1-waves.md:600-604`.
- Source: ADR-0112 migration order names `detect_risk_shift` as Transform + trajectory consumption.
- Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:116-120`.
- Direct composition family C: `list_open_loops` once at workspace scope with no entity filter.
- Category: Read.
- Source: W5-B assignment in `.docs/plans/v1.4.1-waves.md:594-598`.
- Source: ADR-0112 migration order names `list_open_loops` as a simple Read ability.
- Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:118-120`.
- Support composition family D: `get_entity_context` at `ContextDepth::Deep` for tracked accounts and relevant attendees.
- Category: Read.
- Source: current `ContextDepth` supports `Deep`.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:28-42`, `:147-162`.
- Source: current `get_entity_context` returns entries plus optional trajectory.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:44-50`, `:130-134`.
- Source: current trajectory loading is gated by depth.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:165-179`.
- Call order pin:
- Step 1: build the day seed from the legacy daily briefing inputs: today's meetings, tracked accounts, actions, callouts, and workspace context.
- Legacy anchors: `src-tauri/src/services/dashboard.rs:667-748`, `src-tauri/src/prepare/orchestrate.rs:902-919`, `:968-991`.
- Step 2: resolve tracked account subjects and meeting attendee subjects.
- Source shape: `prepare_meeting` plan resolves meeting and related attendee/account/person `SubjectRef`s.
- Source: `.docs/plans/wave-W5/DOS-219-plan.md:37-45`.
- Step 3: invoke `get_entity_context` Deep for each tracked account needing trajectory context.
- Step 4: invoke `prepare_meeting` for each meeting today.
- Step 5: invoke `detect_risk_shift` for each tracked account.
- Step 6: invoke `list_open_loops` once with workspace scope and no entity filter.
- Step 7: synthesize one narrative intro via `ctx.intelligence.complete(...)`.
- Narrative synthesis is a single LLM call, not a sub-ability.
- ADR-0102 says abilities receive the provider through `AbilityContext.intelligence`.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:121-137`.
- ADR-0102 says direct PTY/provider construction inside abilities is forbidden.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:424-426`.
- Category note: Linear pins category=Read even with a narrative intro.
- Source: Linear DOS-220 acceptance criteria.
- ADR tension: ADR-0102 table says Transform may invoke provider, while Read usually does not.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-93`.
- Plan resolution: classify by effective call graph as Linear states, but L0 reviewers must explicitly approve this because the parent does invoke one narrative LLM call.
- Fan-out semantics:
- `prepare_meeting`: fan out per today's meetings.
- `detect_risk_shift`: fan out per tracked account.
- `get_entity_context`: fan out per tracked account and per attendee only after deduping subjects.
- `list_open_loops`: no fan-out; one workspace-scope invocation.
- Fan-out dedupe key must include subject, depth, schema version, mode, actor, and relevant claim invalidation/watermark inputs.
- Source: ADR-0102 caching warning for Read abilities.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:90-93`.
- Provenance flatten/merge contract:
- Parent provenance keeps its own direct sources in `sources[]`.
- Child provenance remains under `children[]`.
- Field attribution references child fields using stable composition IDs.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:245-266`.
- Trust merges bottom-up.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:257-264`.
- Prompt fingerprints do not merge; parent narrative gets its own fingerprint.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:259-262`.
- Source refs from child outputs should not be copied into parent direct sources.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:257-263`.
- Render-time "About this" may dedupe sources across children for display.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:263`.
- Schema-version coordination:
- Parent input requires `schema_version`; no implicit latest for external callers.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:141-182`.
- Child inputs must pass each child ability's current schema version explicitly.
- Current `get_entity_context` schema version is 2.
- Source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:25-27`, `:52-57`.
- Parent output must include `schema_version` in `DailyReadiness`.
- Source: Linear DOS-220 output shape.
- Parent provenance must include the ability schema version.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:24-37`, `:70-72`.
- Parallel-run comparator must understand that child schema changes can cause structured divergence even when the parent schema is unchanged.
- Source: ADR-0106 regression classification for different inputs/fingerprints.
- Source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:106-114`.

## 6. Acceptance Criteria

- Source: Linear DOS-220 `mcp__linear__.get_issue` response, section "Acceptance criteria".
- Verbatim DOS-220 acceptance criteria:
- [ ] Registered with category=Read (composed; does not itself invoke LLM apart from narrative intro - classified based on effective call graph).
- [ ] Eval fixtures cover composition scenarios.
- [ ] Parallel-run divergence ≤1% over 7 days.
- [ ] Tauri app daily briefing view renders from this ability; provenance tree visible.
- Added L0 acceptance criteria:
- [ ] 9-channel sensitivity audit complete at L0 with per-channel gate citation.
- Source for the 9-channel baseline: `.docs/plans/wave-W5/proof-bundle.md:75-87`.
- Source for centralized prompt-input gate: `src-tauri/src/services/claims.rs:3461-3478`.
- Source for abilities-runtime prompt-input gate mirror: `src-tauri/abilities-runtime/src/types.rs:186-202`.
- [ ] ADR-0106 replay-key parity assertion: `canonical_prompt_hash` is identical between ReplayProvider lookup and PromptFingerprint provenance.
- Source for W5 parity lesson: `.docs/plans/wave-W5/proof-bundle.md:44-48`.
- Source for hash canonicalization fields: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:40-48`.
- Source for replay key: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:82-90`.
- [ ] Subject-fit hard-error verified on fixture with deliberate field/output subject mismatch.
- Source for inherited subject-fit rule: `.docs/plans/wave-W5/DOS-218-plan.md:34`.
- Source for Transform subject-fit hard gate: `.docs/plans/wave-W5/DOS-219-plan.md:70-72`.
- Source for current subject validation shape: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:356-390`, `:492-508`.
- [ ] Fixtures are mandatory for all four named scenarios: typical day, no meetings, day with risk shifts, day with stale Glean.
- Source for Stage 2 fixture count and scenarios: Linear DOS-220 description, section "Scope (per ADR-0112 §1)".
- Source for fixture stage requirement: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:27-31`.
- [ ] Bundle-9 parity green per wave plan §588.
- Source: `.docs/plans/v1.4.1-waves.md:588-592`.
- [ ] Parallel-run divergence ≤1% over a 7-day rolling window at 10% sampling.
- Source for 10% sampling: Linear DOS-220 Stage 3.
- Source for rolling 7-day criterion: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:128-136`.
- [ ] Tauri daily briefing view renders from this ability; provenance tree visible.
- Source for existing Tauri command: `src-tauri/src/commands/core.rs:23-32`.
- Source for command registration: `src-tauri/src/lib.rs:664-683`.
- Source for current dashboard data shape: `src-tauri/src/types.rs:1452-1482`.
- [ ] Child provenance tree remains visible instead of flattened away.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:245-266`.
- [ ] Narrative intro has PromptFingerprint provenance and no child fingerprint merge.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:48-52`, `:259-262`.
- [ ] Non-Live fixture reader absence returns FixtureReaderRequired.
- Source: `src-tauri/abilities-runtime/src/services/context.rs:1090-1119`, `:1180-1191`, `:1224-1229`.
- [ ] Private context/evidence cannot be caller-injected from Tauri, MCP, Agent, eval JSON, or replay JSON.
- Source for prior W5 lesson: `.docs/plans/wave-W5/proof-bundle.md:38-42`.
- [ ] No direct DB/file/signal writes from ability implementation; mutations, if any, remain in services and are not part of this Read migration.
- Source: ADR-0102 Read category definition in `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`.
- Source: DailyOS command rule in user-provided AGENTS instructions: "All mutations go through services/."
- [ ] Output shape matches Linear DOS-220:
- `DailyReadiness.narrative`.
- `DailyReadiness.meetings_today`.
- `DailyReadiness.overnight_changes`.
- `DailyReadiness.risk_shifts`.
- `DailyReadiness.open_loops`.
- `DailyReadiness.schema_version`.
- Source: Linear DOS-220 description, section "Output shape".
- [ ] Every leaf field in the output has field attribution.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:201-241`.
- [ ] Every material LLM-synthesized field has non-empty source refs.
- Source: `.docs/decisions/0105-provenance-as-first-class-output.md:237-240`.

## 7. Linear Dependency Edges

- Linear source: `mcp__linear__.get_issue(id="DOS-220", includeRelations=true, includeReleases=true)`.
- DOS-220 has no Linear-recorded blockers.
- Linear relation source: DOS-220 response `relations.blockedBy=[]`.
- DOS-220 blocks no issue in Linear.
- Linear relation source: DOS-220 response `relations.blocks=[]`.
- DOS-220 related issues include DOS-218 and DOS-219.
- Linear relation source: DOS-220 response `relations.relatedTo`.
- Stage 0 requires DOS-218 and DOS-219 past Stage 3.
- Source: Linear DOS-220 description, section "Scope (per ADR-0112 §1)".
- Evidence checked: W5 proof bundle says DOS-218 and DOS-219 landed together and reached Cycle-8 APPROVE.
- Source: `.docs/plans/wave-W5/proof-bundle.md:3-11`, `:27`.
- Evidence checked: v1.4.0 proof bundle date is 2026-05-06, while the requested 2026-05-07 Done date is not present in the checked W5 proof file.
- Source: `.docs/plans/wave-W5/proof-bundle.md:5`.
- Stage 0 therefore appears met from the available W5 proof evidence, subject to Linear status updates outside this packet.
- DOS-220 composes sibling W5-B DOS-221 `list_open_loops`.
- Source: `.docs/plans/v1.4.1-waves.md:594-598`.
- DOS-220 composes sibling W5-C DOS-222 `detect_risk_shift`.
- Source: `.docs/plans/v1.4.1-waves.md:600-604`.
- DOS-220 may ship in parallel with W5-B and W5-C because the v1.4.1 wave plan says W5 migrations run in parallel.
- Source: `.docs/plans/v1.4.1-waves.md:582-584`.
- Conflict to manage: ADR-0112 originally says migration order is serial within the capability group.
- Source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:112-126`.
- Resolution for v1.4.1: follow the newer wave plan for parallel implementation, but isolate parallel-run telemetry and use shims until sibling cutovers are complete.
- Source for wave override: `.docs/plans/v1.4.1-waves.md:582-584`.
- Shim/fallback rule:
- If DOS-221 is not cut over, `get_daily_readiness` may call a compatibility reader for open loops.
- If DOS-222 is not cut over, `get_daily_readiness` may call a compatibility reader for risk shifts.
- The compatibility reader must be declared in provenance as legacy/fallback input, not silently presented as child ability output.
- Source for legacy coexistence: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:41-49`, `:138-144`.
- The shim must not bypass sensitivity, subject-fit, source lifecycle, or prompt fingerprint gates.
- Source: W5 channel sweep `.docs/plans/wave-W5/proof-bundle.md:75-89`.
- The shim must be removed or demoted after the sibling ability reaches cutover.
- Source: ADR-0112 Stage 5 old-path removal in `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:49`.
- DOS-220 related issues also include DOS-175, DOS-281, DOS-282, DOS-283, and DOS-44 in Linear.
- Source: Linear DOS-220 response `relations.relatedTo`.
- These are related context, not Linear blockers, based on the fetched relation shape.

## 8. L0 Reviewer Panel

- Required panel 1: `/plan-eng-review`.
- Scope: architecture, composition shape, child fan-out, schema version coordination, ServiceContext fixture seams, provenance merge, and parallel-run cutover plan.
- Source for W5 L0 requirement: `.docs/plans/v1.4.1-waves.md:606-613`.
- Source for composition semantics: `.docs/decisions/0102-abilities-as-runtime-contract.md:394-418`.
- Source for provenance merge: `.docs/decisions/0105-provenance-as-first-class-output.md:245-266`.
- Source for parallel-run flow: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:21-49`.
- Required panel 2: `/codex challenge`.
- Scope: adversarial channel leak, replay parity, fan-out explosion, subject bleed, stale Glean, hidden Tauri read/write split, and provenance flattening errors.
- Source for W5 cycle leak pattern: `.docs/plans/wave-W5/proof-bundle.md:50-65`, `:75-89`.
- Source for prompt replay parity: `.docs/plans/wave-W5/proof-bundle.md:44-48`.
- Source for ADR-0106 canonicalization: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:40-48`, `:82-90`.
- Not required for this L0 packet: `/cso`.
- Rationale: this packet is documentation-only and the proposed implementation is a pure migration with no new trust boundary beyond inherited ability/sensitivity gates.
- Caveat: if implementation changes MCP exposure, render-policy code, sensitivity gates, services, migrations, or privileged action paths, `/cso` is triggered by the protocol amendment.
- Source: `.docs/plans/v1.4.0-waves-amendments.md:38-52`.
- Not required for this L0 packet: `/plan-design-review`.
- Rationale: no new visual/UI design surface is proposed beyond rendering the existing provenance affordance.
- Caveat: if daily briefing visual treatment, "About this" interaction, or trust-band presentation changes, design review should be added.
- Source for current UI data shape: `src-tauri/src/types.rs:1452-1482`, `:1504-1527`.
- Source for provenance rendering dependency: `.docs/decisions/0105-provenance-as-first-class-output.md:315-325`.
- Optional consult: `/codex consult` is not listed as required by this packet because the user asked for eng + codex only.
- If reviewers apply the broader Review Ladder literally, codex consult may be added by the Linear-ticket L0 process.
- Source for W5 merge gate only says "L0 plan approvals per matrix" without enumerating the matrix.
- Source: `.docs/plans/v1.4.1-waves.md:606-613`.

## 9. L0 Acceptance Gate

- Gate 1: all required reviewer panels approve.
- Required approvals: `/plan-eng-review` and `/codex challenge`.
- Minimum reviewer count: 2.
- Source: this packet §8 and `.docs/plans/v1.4.1-waves.md:606-613`.
- Gate 2: DOS-220 Linear description links to this packet.
- Target link path: `.docs/plans/v1.4.1-waves/W5-A-L0-packet.md`.
- Source for Linear as canonical planning surface: user-provided AGENTS instructions, "Linear is canonical for issues, backlog, and project execution."
- Gate 3: 9-channel audit is complete with per-channel gate citation.
- Source for channels: `.docs/plans/wave-W5/proof-bundle.md:75-87`.
- Source for gate: `src-tauri/src/services/claims.rs:3461-3478`.
- Source for current render surface policies: `src-tauri/abilities-runtime/src/sensitivity.rs:1-22`, `:257-363`.
- Gate 4: ADR-0106 parity proof is shown in the packet and implementation plan.
- Required proof: ReplayProvider lookup key equals PromptFingerprint provenance `canonical_prompt_hash`.
- Source: `.docs/plans/wave-W5/proof-bundle.md:44-48`.
- Source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:40-48`, `:82-90`.
- Gate 5: Cycle-7 class-pattern memo is explicitly carried forward.
- Memo: this packet pre-enumerates the channels rather than discovering them across L2 cycles.
- Source: `.docs/plans/wave-W5/proof-bundle.md:75-89`.
- Gate 6: sibling W5-B/W5-C status is noted.
- Source: `.docs/plans/v1.4.1-waves.md:594-604`.
- Required status: can run in parallel, but DOS-220 uses a shim/fallback until sibling cutovers complete.
- Source for parallel W5: `.docs/plans/v1.4.1-waves.md:582-584`.
- Source for fallback/old path coexistence: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:41-49`, `:138-144`.
- Gate 7: direct-vs-support child ambiguity is resolved before coding.
- Decision: direct migration children are `prepare_meeting`, `detect_risk_shift`, and `list_open_loops`; support Read child family is `get_entity_context` Deep.
- Source for three direct children: Linear DOS-220 Stage 1.
- Source for `get_entity_context` child example: `.docs/decisions/0102-abilities-as-runtime-contract.md:414-418`.
- Source for current Deep support: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:147-179`.
- Gate 8: no caller-injected private context.
- Source: `.docs/plans/wave-W5/proof-bundle.md:38-42`.
- Gate 9: non-Live readers fail closed.
- Source: `src-tauri/abilities-runtime/src/services/context.rs:1090-1119`, `:1180-1191`, `:1224-1229`.
- Gate 10: exact-symbol legacy gap is documented.
- Required note: `get_daily_readiness` is absent as an exact local symbol, so implementation must define the precise legacy comparator path before Stage 3.
- Current anchors: `src-tauri/src/commands/core.rs:23-32`, `src-tauri/src/services/dashboard.rs:599-649`, `src-tauri/src/prepare/orchestrate.rs:2974-2978`.

## 10. Out-Of-Scope

- Out of scope: Tauri write cutover for daily briefing surfaces.
- Reason: v1.4.0 W5 cycle 3 proved read cutover can diverge from legacy writes.
- Source: `.docs/plans/wave-W5/proof-bundle.md:67-69`.
- Out of scope: silently moving daily briefing create/update/delete or projection writes to the new ability.
- Reason: this ticket is a Read migration and ADR-0102 Read abilities cannot mutate domain state.
- Source: `.docs/decisions/0102-abilities-as-runtime-contract.md:81-97`.
- Out of scope: claim type and projection migration for daily briefing persisted surfaces.
- Reason: the prior W5 proof bundle says Tauri entity-context write cutover needed a proper claim type plus projection migration.
- Source: `.docs/plans/wave-W5/proof-bundle.md:106-107`.
- Out of scope: surface rendering audit beyond the prompt-input boundary.
- Reason: W5 prompt-input enforcement does not prove every published/callout surface.
- Source: `.docs/plans/wave-W5/proof-bundle.md:107`.
- Out of scope: ADR-0108 sensitivity rendering for callouts/published surfaces audit.
- Reason: proof bundle marks it as a follow-up.
- Source: `.docs/plans/wave-W5/proof-bundle.md:107`.
- Out of scope: bundle expansion beyond the four named DOS-220 fixtures plus bundle-9 parity.
- Reason: W5 proof bundle explicitly deferred broader bundle expansion.
- Source: `.docs/plans/wave-W5/proof-bundle.md:108`.
- Out of scope: adding new claim schema columns or tables.
- Reason: no new schema is required to author this L0 packet; implementation should first answer the Intelligence Loop check if schema changes are proposed.
- Source for existing claim schema/lifecycle readers: `src-tauri/src/services/claims.rs:8081-8124`, `:8171-8215`.
- Out of scope: hardcoding customer data, domains, company names, email addresses, or account details in fixtures.
- Source: user-provided AGENTS instructions, "No customer-specific data in source code."
- Out of scope: direct DB writes from command handlers or ability modules.
- Source: user-provided AGENTS instructions, "All mutations go through services/."
- Out of scope: replacing DOS-221 or DOS-222 implementation work inside this packet.
- Source: W5-B/W5-C file ownership in `.docs/plans/v1.4.1-waves.md:594-604`.
- Out of scope: using daily-readiness narrative synthesis to authorize mutations.
- Source: Transform output trust boundary in `.docs/decisions/0102-abilities-as-runtime-contract.md:365-388`.
- Out of scope: claiming the exact prior "Done 2026-05-07" date for DOS-218/DOS-219 without a checked source line.
- Checked source says W5 proof bundle date 2026-05-06 and status Cycle-8 APPROVE.
- Source: `.docs/plans/wave-W5/proof-bundle.md:3-5`, `:27`.

## V2 Cycle-1 Fold (2026-05-14)

Cycle-1 L0 panel raised 12 findings — 7 architect APPROVE-WITH-COMMENTS + 5 codex BLOCK. This section folds them. Forward references back to V1 sections remain valid; this section adds tightening + missing enforcement gates.

### Ability registration — folds Architect F1

Explicit registration metadata:
- allowed_actors = [User, Agent, System]
- mcp_exposure = Invocable
- required_scopes = ["read.daily_readiness"]
- operations = [IntelligenceComplete]

Grounded source: src-tauri/abilities-runtime/src/registry.rs for the registration macro shape — search for similar Read+LLM abilities to confirm exact form.

Per ADR-0102 §141-182 + wave plan §588-592.

### Narrative-prompt channel table — folds Architect F2 + Codex F1

Cycle-7 pilot lesson: pre-enumerate, don't discover. Each composed child ability contributes output fields to the narrative-synthesis prompt:

| Child ability | Output field | Channel | Gate point | Reaches prompt? |
|---|---|---|---|---|
| prepare_meeting | MeetingBrief.topics[] | 1 — subject-ref claims | services/claims.rs centralized gate | YES |
| prepare_meeting | MeetingBrief.attendee_context[] | 4 — composed children | applied at child invocation | YES |
| prepare_meeting | MeetingBrief.open_loops[] | 1+4 | gated at child | YES |
| prepare_meeting | MeetingBrief.suggested_outcomes[] | 4 | gated at child | YES |
| detect_risk_shift | RiskShiftResult.direction | 4 | Computed; no claim text | metadata only |
| detect_risk_shift | RiskShiftResult.indicators[].source_refs | 1 — subject-ref | gated at child | metadata only — refs, not text |
| detect_risk_shift | RiskShiftResult.evidence_summary | 6 — rendered prompt | parent boundary gate fires | YES |
| list_open_loops | OpenLoopsResult.loops[].text | 1+2 | gated at child | YES |

At DOS-220's boundary, after all children return, the centralized sensitivity gate fires again on every claim-bearing field before serialization into canonical JSON for the narrative LLM call.

### ADR-0106 replay parity fixture — folds Architect F3

Fixture: replay_parity_narrative_synthesis.json
- Seeds: deterministic composition input — today + N meetings + N tracked accounts
- Captures: ReplayProvider lookup hash for the narrative completion
- Asserts: PromptFingerprint.canonical_prompt_hash == ReplayProvider lookup hash, byte-for-byte
- Includes: non-default temperature/top_p/seed assertion — pair with W5-C's same fixture pattern

### Composition handoff — folds Architect F4

When child output fields are used as narrative prompt inputs, they re-enter channels 6+7 at DOS-220's boundary. The centralized sensitivity gate in services/claims.rs fires a second time at this boundary — children may have applied their own gate at their boundary, but the parent re-applies because composition aggregates can surface combinations a child gate doesn't see.

### Per-child subject-fit + parent workspace scope — folds Architect F5 + Codex F3

Subject-fit hard-error applies per child invocation. Each composed child's input carries an explicit subject_ref; child returns AbilityError::SubjectNotOwned — or AbilityErrorKind::HardError with "subject_not_owned" — on cross-tenant/cross-workspace inputs.

Parent DailyReadiness declares workspace scope explicitly — no subject attribution at parent level. Bundle-9 fixture additions:
- bundle-9-subject-partition: Account A attendee in today's meetings + Account B in tracked-accounts; assert Account A meeting/attendee claim text does NOT enter Account B risk fields/narrative.

### Workspace scope per child — folds Codex F4

Every child invocation carries the calling workspace in its input + provenance scope + cache dedupe key. Cross-workspace meeting fixture: meeting includes attendees from a foreign workspace; child invocation for that attendee returns SubjectNotOwned hard-error.

### Source lifecycle across children — folds Codex F5

Each composed child applies the source revocation check during its load path — per W5-B V2 pattern: ProvenanceWarning::Masked with SourceRevoked shape + source_revoked envelope counter. DOS-220 surfaces aggregated counter at the parent DailyReadiness.coverage_warnings field so users see degradation across composed children.

### Judge-scored divergence metric — folds Codex F6

Stage-3 parallel-run divergence metric:
- Window: 7-day rolling
- Sampling: 10% of get_daily_readiness invocations
- Comparator: legacy prepare::orchestrate + dashboard::get_dashboard_data aggregate — grounded at orchestrate.rs:2974-2978, :3102-3118, :3346-3386
- Sample unit: one daily-readiness output, judge-scored across relevance, faithfulness, attribution-completeness
- Judge model: TODO — packet pins judge config before Stage 3; same judge model as DOS-219; grep wave-W5/DOS-219-plan.md for the judge harness config; bind same model + prompt template here
- Rubric: relevance ≥0.85, faithfulness ≥0.90, attribution-completeness ≥0.95 — matches DOS-219 thresholds
- Drift threshold: ≤1% divergence over 7-day window
- Drift-failure rule: alert + investigate; do NOT auto-cutover until divergence trends down for 7 days post-investigation

### Legacy comparator — folds Architect F6

Stage-3 parallel-run legacy comparator: prepare::orchestrate + dashboard::get_dashboard_data aggregate — per grounded source citations above. Stage-1 fixtures authored against this comparator pair.

### Category=Read-with-LLM-call reviewer sign-off — folds Architect F7

L0 reviewer sign-off line in §9 Gates: "Reviewer panel accepts DOS-220 as category=Read despite the narrative-synthesis LLM call. ADR-0102 §141-182 allows Read abilities to call LLMs for synthesis when the output is composed from Trusted children + a single narrative wrap; the wrap does not introduce new claim text, only narrative framing of existing children's claims."

### Changelog V2 — 2026-05-14

- V2 folds 12 cycle-1 panel findings — 7 architect + 5 codex.
- Architect F1 → §Ability registration
- Architect F2 + Codex F1 → §Narrative-prompt channel table
- Architect F3 → §ADR-0106 replay parity fixture
- Architect F4 → §Composition handoff
- Architect F5 + Codex F3 → §Per-child subject-fit + parent workspace scope
- Architect F6 → §Legacy comparator
- Architect F7 → §Category=Read reviewer sign-off
- Codex F4 → §Workspace scope per child
- Codex F5 → §Source lifecycle across children
- Codex F6 → §Judge-scored divergence metric

## 11. Changelog

- V1 2026-05-13: initial L0 packet.
- Grounded on v1.4.0 W5 pilot lessons.
- Grounded source: `.docs/plans/wave-W5/proof-bundle.md:1-108`.
- Grounded on v1.4.1 W5 capability migration plan.
- Grounded source: `.docs/plans/v1.4.1-waves.md:582-613`.
- Grounded on ADR-0102 ability runtime contract.
- Grounded source: `.docs/decisions/0102-abilities-as-runtime-contract.md:50-182`, `:394-418`, `:447-480`, `:519-545`.
- Grounded on ADR-0105 provenance envelope, trust, field attribution, composition, and `source_asof`.
- Grounded source: `.docs/decisions/0105-provenance-as-first-class-output.md:19-58`, `:103-158`, `:166-173`, `:201-266`, `:391-446`.
- Grounded on ADR-0106 prompt fingerprinting.
- Grounded source: `.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:18-48`, `:82-90`, `:106-114`.
- Grounded on ADR-0112 parallel-run and cutover.
- Grounded source: `.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:21-49`, `:112-144`.
- Grounded on current `get_entity_context` post-DOS-349 ability runtime shape.
- Grounded source: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:25-65`, `:72-134`, `:147-179`, `:356-390`, `:402-415`, `:522-537`.
- Grounded on current daily briefing/readiness legacy anchors where exact `get_daily_readiness` symbol was absent.
- Grounded source: `src-tauri/src/commands/core.rs:23-32`, `src-tauri/src/services/dashboard.rs:599-649`, `src-tauri/src/prepare/orchestrate.rs:2974-2978`, `:3102-3118`, `:3346-3386`.
- No code changes.
- No commit.
