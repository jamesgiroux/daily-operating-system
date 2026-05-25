# L0 Packet - v1.4.5 W5-B - DOS-476 End-to-End Workspace Memory Validation

**Issue:** [DOS-476](https://linear.app/a8c/issue/DOS-476) - End-to-end validation with real workspace data  
**Wave:** W5-B - Backfill & Validation release gate  
**Branch/worktree:** `codex/v1.4.5-w5-e2e-validation`  
**Status:** Draft for L0 review  
**Prepared:** 2026-05-25  

---

## 1. Scope Decision

W5-B is a validation and evidence lane, not a new implementation lane.

The canonical v1.4.5 wave plan says W5-B has two tracks:

1. Validate that W5-A historical backfill registration remains conservative and pending-review by default.
2. Validate that already-reviewed ingestion paths still perform file drop -> ingestion -> claim proposals -> trust-band rendering -> signal propagation -> context inclusion -> user correction -> claim update -> re-rendering.

W5-B must not make W5-A historical backfill an implicit claim producer. Claim-producing validation uses explicit ingestion fixtures through already-reviewed paths: `_inbox`, entity intake, and placement fixtures where available.

---

## 2. Revision History

- **V1.0** - Initial W5-B validation packet.
- **V1.1** - L0 cycle-1 blocker fold. Removes false-green release-gate path, makes entity-intake and `_inbox` mandatory ingestion paths, treats MCP/headless parity and source lifecycle suppression as fail-closed gates, moves hermetic graph assertions into Rust integration tests, adds W5-B release-gate wiring, adds manual-evidence redaction contract, and expands K-in references.
- **V1.2** - L0 cycle-2 feasibility fix. Splits POSIX-impossible NUL filename coverage into an invalid path/input rejection test and platform-gates non-UTF8 filename plus hardlink fixtures where the host filesystem cannot represent them.
- **V1.3** - Rebased-base readiness refresh. Records that W5-A is folded into the active W5 validation PR, explicit ingestion and graph projection now have partial automated evidence, filesystem negative fixtures have partial automated evidence, and MCP placement/lifecycle/signal-middle-hop gaps remain blocked.

---

## 3. Dependency Gate

Implementation is blocked until these are true on the working base:

- **DOS-475 / W5-A merged:** PR #388 or equivalent is merged/rebased into the base used by W5-B.
- **W4 stack available:** W4 source management, markdown preview, and placement contract code is merged/rebased into the base used by W5-B.
- **Graph projection available:** W3-C / DOS-489 workspace graph projection is available as a service-callable projection on hermetic test DBs.
- **Claim producer available:** W3-A / DOS-470 real workspace extractor claim production is available through `IngestPipeline`.
- **Signal wiring available:** W3-B / DOS-471 workspace lifecycle signal policy and invalidation wiring is available.
- **MCP placement path available:** actual MCP/gateway or registered-handler invocation for workspace placement is available before MCP/headless parity can pass.
- **Lifecycle action substrate available:** source-management actions for relink/correction, quarantine, ignore or scratchpad, archive/delete, and promotion/reingest exist before their named DOS-476 ACs can pass.

Planning and L0 review may proceed before dependencies merge. W5-B implementation may create the validation harness and report shell, but the W5 release gate cannot close while any mandatory axis is blocked. A blocked axis must remain a release-blocking predecessor or split issue, not a green W5-B result.

---

## 4. Evidence Status Taxonomy

Every validation axis reports exactly one status:

- **green:** all automated assertions and required manual evidence for that axis passed.
- **fail:** implementation exists, but evidence failed.
- **blocked:** required substrate, bridge, action, or surface path is unavailable.

Blocked axes can appear in an interim validation report, but they do not satisfy W5-B Done and cannot close the W5 release gate or proof bundle. Only all-green mandatory axes close W5-B.

---

## 5. Owned Files

Primary owned surfaces:

- `tests/v146_validation/README.md`
- `tests/v146_validation/run.sh`
- `tests/v146_validation/redaction_lint.sh`
- `tests/v146_validation/fixtures/`
- `tests/v146_validation/expected/`
- `scripts/release-gate/run-v146-validation.sh`
- `src-tauri/tests/v146_validation.rs`
- `src-tauri/tests/v146_validation/`
- `src-tauri/tests/fixtures/v146_validation/`
- `src-tauri/src/release_gate.rs` only for registering W5-B validation evidence as a mandatory release-gate invariant.
- `.docs/plans/wave-W5-v146/validation-report.md`
- `.docs/plans/wave-W5-v146/proof-bundle.md` if W5-B closes the wave gate in the same PR.

The top-level `tests/v146_validation/` directory is the orchestrator/proof harness named by the wave plan. Hermetic DB assertions live in Rust integration tests under `src-tauri/tests/` because that is the repository's existing Rust convention. Release-gate changes are harness wiring only; W5-B does not change product behavior.

---

## 6. Non-Goals

W5-B does not:

- Add product behavior.
- Add schema migrations by default.
- Change `services/claims.rs`, `commit_claim`, trust recompute, source purge, or lifecycle semantics.
- Patch W4 source-management or MCP placement readiness inside the validation PR unless the owner explicitly moves that work into W5-B with a packet amendment.
- Edit W4 WordPress block implementation files except to capture manual evidence after W4 has merged.
- Relax privacy/sensitivity rules to make validation easier.
- Hand-set lifecycle, trust scores, or claim rows to simulate green behavior when a service action is missing.
- Store raw real workspace paths, file contents, claim text, prompt bodies, output bodies, account names, person names, company/customer data, raw file IDs, or raw content hashes in committed fixtures, reports, PR bodies, generated logs, proof bundles, or Linear comments.

---

## 7. Knowledge-Store Pass

Relevant prior solutions and ADRs:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`
  - W5-B must validate runtime-wide trust behavior when claim producers are real. Do not prove only "a claim exists"; prove provenance, freshness, trust inputs, recompute path, and surface behavior.
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`
  - W5-B context assertions must enumerate prompt-bearing channels and go through the centralized `services::claims` sensitivity gates. Do not add a one-off prompt/context reader.
- `docs/solutions/test-failures/repeated-full-migration-test-fixtures-2026-05-24.md`
  - Use migrated DB template helpers for current-schema tests. Do not replay full migrations in every W5-B fixture helper.
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md`
  - If L0 tries to add missing substrate to this validation ticket, split that work into separate issues. W5-B validates existing contracts and files blockers; it does not absorb substrate rewrites.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`
  - Discovery must grep for existing substrate types and behaviors, not just proposed W5-B names.
- ADR-0098 source-aware lifecycle
  - Source removal, stale source posture, revocation/purge behavior, and source-attributed lifecycle effects must be validated through existing lifecycle services or blocked.
- ADR-0101 service boundary enforcement
  - All mutations observed or driven by W5-B go through service APIs.
- ADR-0102 abilities as runtime contract
  - Ability metadata, actor/scope gates, and declared runtime contracts are part of MCP/WP parity evidence.
- ADR-0105 provenance as first-class output
  - Validation must assert source attribution and field attribution are present where surfaces claim provenance.
- ADR-0107 source taxonomy alignment
  - Workspace file claims must carry `DataSource::WorkspaceFile { kind }` and compatible `source_ref`.
- ADR-0108 provenance rendering and privacy
  - Reports, fixtures, and outputs must be redacted; output-side sensitivity must be preserved across Tauri, WP, and MCP.
- ADR-0110 evaluation harness for abilities
  - W5-B evidence should be machine-readable where practical and commit only anonymized/generic fixtures.
- ADR-0111 surface-independent ability invocation
  - WP and MCP evidence must validate registry/bridge invocation, not parallel schema readers.
- ADR-0112 migration strategy parallel-run and cutover
  - W5-B is the cutover validation lane; it compares old/new source paths without mutating rollback policy.
- ADR-0115 signal granularity audit
  - Signal assertions must validate the named signal chain and invalidation impact, not just row creation.
- ADR-0123 typed claim feedback semantics
  - Correction evidence must exercise typed, append-only feedback rows consumed by trust/context/render paths.
- ADR-0126 memory substrate invariants
  - W5-B must assert immutable claim columns are not rewritten, feedback/trust events are append-only, engagement is not treated as trust, and claim-state mutations consume canonical services.
- ADR-0128 headless DailyOS MCP as product surface
  - MCP/headless parity requires real MCP/gateway or registered-handler execution with actor/scope and privacy behavior, not first-party local DB reads.
- ADR-0130 surface-independent composition contract
  - WP rendering evidence must prove renderers consume substrate output and do not author their own intelligence.

---

## 8. Substrate Discovery Evidence

Before implementation, W5-B records these discovery greps in the validation report:

- Claim producer and extractor: `rg "commit_claim|WorkspaceExtractor|WorkspaceClaimProposal" src-tauri/src src-tauri/tests`
- Feedback actions: `rg "record_claim_feedback|ClaimFeedback|FeedbackAction|SourceManagementAction" src-tauri/src src-tauri/abilities-runtime/src`
- Lifecycle suppression/removal: `rg "scratchpad|ignored|archive|delete|quarantine|reingest|relink" src-tauri/src src-tauri/abilities-runtime/src`
- MCP placement: `rg "workspace_place_document|dailyos.write.place_document|tool_placement|write.workspace_place_document" src-tauri`
- Release-gate hooks: `rg "release-gate|run-v146|mandatory" package.json src-tauri/src/release_gate.rs scripts tests`
- Prompt channels: `rg "load_claims_active|load_claims_active_by_source_ref|prepare_meeting|get_entity_context|linked-meeting|composed" src-tauri/src src-tauri/abilities-runtime/src`

If discovery shows a named AC has no service/action/bridge substrate, W5-B marks that axis blocked and creates or links the owning issue instead of writing a local workaround.

---

## 9. Current Readiness Findings

Read-only prep on 2026-05-25, refreshed after rebasing on merged W4/W5-A substrate, found:

- W5-A backfill service and CLI are testable and cover dry-run, apply, pending-review state, graph exclusion, privacy-safe signal behavior, duplicate grouping, source-time handling, and resume safety.
- Entity intake is wired enough to test ability -> `WorkspaceIntakeService` invocation and block rendering from claim rows.
- `_inbox` flow is testable for lifecycle registration and pending entity assignment.
- Markdown preview and source-management read surfaces exist on the rebased base.
- Full live WP-to-Tauri loopback evidence is not currently present.
- Source-management action round trip must be validated through a real action path, not render-only ledger reads. Current action substrate exposes `reingest`, `quarantine`, and `relink`; ignore/scratchpad plus archive/delete remain missing and block Axis 6.
- MCP placement has contract/catalog pieces, but the actual MCP v2 handler path is not fully wired. MCP/headless parity remains blocked until a real MCP/gateway or registered-handler path is exercised.
- Scratchpad/ignored and archive/delete lifecycle effects are named DOS-476 ACs; missing actions block Axis 6 and W5 release close.
- Automated explicit-ingestion evidence proves a direct pipeline fixture creates lifecycle, run, link, and `commit_claim` rows with privacy-safe provenance, but Axis 2 remains release-blocked until entity-intake, `_inbox`, and MCP placement path coverage is complete.
- Automated partial signal evidence proves workspace ingestion emits privacy-safe `WorkspaceFileIngested` and queues prep invalidation, but static code inspection still does not confirm a literal `WorkspaceFileIngested -> EntityIntelligenceUpdated` derivation. Axis 4 remains release-blocked unless the implemented base proves the literal wave-plan chain or the owning substrate issue lands that hop.
- Automated partial filesystem evidence covers traversal, encoded traversal, outside absolute paths, workspace root equality, symlink escape, NUL input, and hardlink rejection when supported. Axis 7 remains release-blocked until oversized, non-UTF8, managed/hidden, and unsupported-file cases are automated in this W5-B axis.
- `WorkspaceExtractor` is intentionally narrow: note-like, linked Account/Project/Person content becomes `UserNote` claim proposals. W5-B fixtures must use that shape for claim-producing automated tests.
- Stale-source trust-band validation likely requires explicit trust recompute/job execution. W5-B must include the existing recompute step; hand-setting trust scores is not valid evidence.

---

## 10. Validation Matrix

### Axis 1 - Backfill Registration Safety

**Purpose:** prove W5-A remains conservative.

Fixture setup:

- Create a temporary workspace with generic account/project/person-like directories, `_inbox`, and unsupported/managed files.
- Run W5-A dry-run and apply against the fixture workspace.

Assertions:

- Eligible files get pending-review lifecycle/source records or explicit skip/divergence reason codes.
- Backfilled sources do not create claims.
- Backfilled pending-review sources do not enter workspace graph results.
- Backfilled pending-review sources do not enter default runtime contexts.
- Dry-run and report artifacts expose only counts, reason codes, source classes, and opaque handles.
- No user file is rewritten.

Primary commands/tests:

- `cargo test --test v146_validation backfill_registration_safety`
- `cargo test workspace_backfill --lib --bins`
- `bash tests/v146_validation/run.sh backfill`

### Axis 2 - Explicit File to Claim Provenance Chain

**Purpose:** prove every reviewed ingestion path creates claim-backed intelligence with workspace provenance.

Mandatory path submatrix:

| Path | Required status |
| --- | --- |
| Entity intake | Mandatory green when W2-C/W2-A dependencies are present; blocked otherwise. |
| `_inbox` assignment/processing | Mandatory green when W2-D dependencies are present; blocked otherwise. |
| MCP/work-product placement | Mandatory green when W4-C plus MCP handler/registered path is present; blocked otherwise. |

Fixture setup:

- Use explicit ingestion fixtures, not historical backfill.
- Use linked note-like Account/Project/Person source content that `WorkspaceExtractor` is designed to turn into `UserNote` proposals.
- Use generic entity IDs and synthetic text fixtures only.

Assertions for each mandatory path:

- Ingestion writes a lifecycle row and ingestion run.
- Extractor proposals are committed through `services::claims::commit_claim`.
- Committed claims include `source_ref = workspace_file:<opaque file id>` or the equivalent reviewed source ref.
- Committed claims include `DataSource::WorkspaceFile { kind }`.
- Hermetic graph service assertions report zero attribution gaps for explicitly ingested fixture sources.
- Surface/client context reads use existing claim readers, not direct schema shortcuts.
- Immutable claim fields are not rewritten after commit; feedback/trust changes append new evidence or state events per ADR-0126/0123.

Primary commands/tests:

- `cargo test --test v146_validation explicit_ingestion_to_claim_provenance`
- `cargo test --test v146_validation graph_audit_zero_gaps_on_hermetic_fixture_db`
- `bash tests/v146_validation/run.sh graph-audit`

`workspace_graph_audit` CLI is manual/local evidence only until it can target a hermetic DB. Automated zero-gap evidence must call the graph/audit service inside Rust integration tests against the test DB.

### Axis 3 - Trust-Band Discipline

**Purpose:** prove source lifecycle/freshness affects trust in the conservative direction.

Fixture setup:

- One explicitly ingested recent file with confirmed source time.
- One explicitly ingested old file beyond the relevant freshness threshold.
- One W5-A pending-review backfilled file.
- One promoted or reingested W5-A backfilled source only if a real service action exists and defines freshness semantics.

Assertions:

- Recent confirmed explicit-ingestion claims may be `likely_current` only when source time is confirmed.
- Old explicit-ingestion claims are `use_with_caution` or `needs_verification` after the existing trust recompute path runs.
- Pending-review backfilled sources produce zero claims.
- Promoted/reingested backfilled material cannot become `likely_current` from filesystem mtime alone unless a separate freshness attestation service explicitly records that fact.
- Trust-band rendering surfaces expose uncertainty instead of hiding it.
- Engagement signals are not treated as trust evidence.

Primary commands/tests:

- `cargo test --test v146_validation trust_band_discipline`
- Surface smoke after W4 merge for source-management/markdown-preview/entity-intake blocks.

If no real promotion/reingest action exists, the promoted-without-freshness case is blocked and release-blocking. W5-B must not simulate this by editing lifecycle rows or trust scores directly.

### Axis 4 - Signal Propagation and Prep Invalidation

**Purpose:** prove workspace ingestion changes propagate through the literal release-gate signal chain.

Fixture setup:

- Fixture entity with an upcoming meeting/prep row.
- Explicitly ingest a workspace file linked to that entity through entity intake and `_inbox` paths.

Mandatory assertions:

- `WorkspaceFileIngested` is emitted for explicit ingestion, not W5-A lifecycle-only registration.
- `EntityIntelligenceUpdated` is emitted or queued as the wave-plan middle hop.
- Affected meeting prep is invalidated or marked stale through the existing propagation engine / invalidation queue.
- W5-A lifecycle-only registration emits no `WorkspaceFileIngested`.
- W5-A Backfill link creation emits only `WorkspaceFileEntityLinkChanged`.

Primary commands/tests:

- `cargo test --test v146_validation signal_propagation_invalidates_prep`
- Focused signal policy tests for workspace lifecycle names.

If the base lacks the `EntityIntelligenceUpdated` middle hop, Axis 4 is blocked and a W3-B/W4 owner issue is required. Alternate downstream effects do not make this axis green.

### Axis 5 - Context Inclusion and MCP/Privacy Parity

**Purpose:** prove Tauri and MCP/headless observe the same lifecycle, sensitivity, actor, and provenance rules.

Fixture setup:

- Explicit ingestion claims with public/internal sensitivity.
- A `user_only` fixture claim that must not enter prompt/context channels.
- A pending-review backfilled source.
- Real MCP/gateway or registered-handler request path for workspace graph/context and placement.

Assertions:

- Tauri/entity context includes eligible explicit-ingestion claims with provenance.
- Actual MCP/headless invocation includes the same eligible source-backed claims under the same actor/scope/sensitivity rules.
- MCP/headless scope denial is tested.
- `include_entity_names` denial or actor-filtering is tested where privacy profile requires it.
- `user_only` and disallowed sensitivity claims do not enter prompt-bearing contexts.
- Pending-review backfilled sources are absent from default Tauri and MCP contexts.
- Output artifacts redact content and carry only opaque source handles where privacy profile requires it.
- First-party `workspace_graph_audit` is not used as a substitute for MCP/SurfaceClient behavior.

Prompt-channel sweep:

- `get_entity_context` MCP/Agent.
- prepare/meeting context.
- source-ref claim reads.
- composed child contexts.
- linked-meeting-derived claims.

For every channel, filtering happens before page/limit caps and no direct schema shortcut is introduced.

Primary commands/tests:

- `cargo test --test v146_validation context_inclusion_privacy_parity`
- Existing DOS-412 MCP redaction tests remain green.

If no actual MCP/gateway or registered-handler path is available, Axis 5 is blocked. Ability/service-contract-only tests can be recorded as partial evidence, but they do not prove MCP/headless parity.

### Axis 6 - Lifecycle Actions and User Correction Round Trip

**Purpose:** prove source lifecycle and feedback actions change downstream state and surfaces through real services.

Mandatory lifecycle-action submatrix:

| DOS-476 behavior | Required service evidence | Green condition |
| --- | --- | --- |
| Correction / typed feedback | Existing typed claim feedback action via `services::claims` / feedback service | Append-only feedback row, trust recompute consumes it, context/render changes. |
| Relink | Existing source-management relink action | Link state changes through service API, provenance remains traceable, affected context/render changes. |
| Quarantine | Existing source-management quarantine action | Lifecycle changes through service API, downstream contexts/prep/search exclude source according to policy. |
| Ignore / scratchpad | Existing service action | Source stops influencing search weighting, claim extraction, prep, reports, and MCP context. |
| Archive / delete | Existing service action | Existing claims preserve or tombstone provenance according to lifecycle policy; contexts respect the new lifecycle state. |
| Promotion / reingest | Existing service action with freshness semantics | Ingestion path runs through service, trust freshness rules remain conservative. |

Assertions:

- Correction routes through typed, append-only feedback semantics; no generic untyped yes/no shortcut.
- Claim state or source link state changes according to lifecycle policy.
- Affected context/render surface changes on reread.
- Provenance is preserved or tombstoned according to lifecycle policy.
- No direct DB write bypass is introduced by validation helpers.
- Engagement is not counted as trust.

Primary commands/tests:

- `cargo test --test v146_validation lifecycle_actions_and_user_correction_round_trip`
- Manual WP Studio evidence after W4 source-management is merged.

If any named service action is unavailable, Axis 6 is blocked and release-blocking. W5-B must file/link the owning issue rather than collapsing the missing action into a generic correction test.

### Axis 7 - Filesystem Validation Negative Fixtures

**Purpose:** prove backfill and explicit ingestion still route through `WorkspaceSourceRegistry::open_validated` and fail safely.

Fixtures:

- `..` traversal path.
- Encoded traversal path.
- Symlink outside workspace root.
- Hardlink or multi-link alias where the platform exposes a meaningful device/inode collision; otherwise record the platform skip as blocked/unsupported evidence for that subcase only.
- Oversized file.
- Non-UTF8 filename where the host filesystem/test harness can create one; otherwise platform-gate the filename fixture and keep byte-level parser/input validation in Rust.
- NUL-containing path/input rejection injected before filesystem creation. NUL is not a creatable POSIX/macOS filename, so this is an input validation test, not a filesystem fixture.
- Managed/hidden/unsupported file.

Assertions:

- Backfill and explicit ingestion reject unsafe fixtures before content enters extraction or claim commit.
- Rejections use safe reason codes only.
- No raw paths, content, filenames, or raw file IDs appear in logs, reports, summaries, or Linear-ready text.
- No lifecycle, run, link, claim, embedding, or cache row is written for rejected fixtures except explicitly documented safe rejection audit state.

Primary commands/tests:

- `cargo test --test v146_validation filesystem_validation_negative_fixtures`
- `bash tests/v146_validation/run.sh filesystem`

---

## 11. Top-Level Harness Shape

`tests/v146_validation/run.sh` is an orchestrator, not a replacement for Rust integration tests. It must:

- Accept axis names (`backfill`, `graph-audit`, `trust`, `signals`, `contexts`, `lifecycle`, `filesystem`) plus `all`.
- Run focused Rust integration tests for hermetic assertions.
- Run W5-A CLI dry-run/apply against a synthetic fixture workspace.
- Treat `workspace_graph_audit` CLI as manual/local-only evidence until it can target a hermetic DB.
- Write generated local artifacts under ignored target/output directories, not under committed fixture directories.
- Run `tests/v146_validation/redaction_lint.sh` over validation report drafts, proof bundle drafts, generated logs, and Linear-ready text.
- Exit non-zero if any mandatory automated axis fails or remains blocked in final mode.

`scripts/release-gate/run-v146-validation.sh` wraps `tests/v146_validation/run.sh all` and writes mandatory evidence into `src-tauri/target/release-gate/v146-validation.json`. `src-tauri/src/release_gate.rs` treats that evidence as a mandatory invariant in hermetic mode. Existing `pnpm release-gate -- --mode hermetic` remains required but is not considered W5-B evidence unless the v146 validation invariant is present and green.

Committed top-level fixtures must be generic and PII-free. Real workspace evidence is summarized only in `.docs/plans/wave-W5-v146/validation-report.md` with redacted counts/handles.

---

## 12. Manual Evidence and Redaction Contract

Manual real-workspace evidence may contain only:

- Entity ordinals such as `entity_1`, `entity_2`.
- Counts.
- Booleans.
- Trust-band distributions.
- Reason-code distributions.
- Opaque HMAC handles generated with a local-only key and no reversible raw input.
- Command names and pass/fail statuses.

Manual evidence must not contain:

- Raw paths or filenames.
- Entity names, person names, emails, domains, or customer/company names.
- Claim text, file content, prompt text, output bodies, markdown snippets, or provenance blobs.
- Raw file IDs, raw source handles, raw content hashes, or hash prefixes.

`tests/v146_validation/redaction_lint.sh` must scan `validation-report.md`, `proof-bundle.md`, generated logs, and Linear-ready text for forbidden shapes before any artifact is committed, pasted into Linear, or attached to a PR.

---

## 13. Report Format

`validation-report.md` will include:

- Dependency gate status with commit SHAs for W4, W5-A, W3-A/B/C, MCP placement, and base branch.
- Evidence status table for every axis: green/fail/blocked.
- Commands run and pass/fail status.
- Validation matrix table: axis, fixture, assertion, automated evidence, manual evidence, status.
- Hermetic graph-audit summary with zero attribution-gap requirement for explicitly ingested sources.
- Trust-band distribution summary using counts only.
- Signal propagation summary using signal names and opaque IDs only.
- Lifecycle action submatrix status.
- Manual dogfood summary against at least five real entities using only the allowed manual evidence shape.
- Release-gate evidence path and invariant IDs.
- Follow-up issue links for every failed or blocked axis.

The report must not include raw paths, file names, real entity names, emails, domains, claim text, prompt text, output bodies, provenance blobs, raw file IDs, or raw file contents.

---

## 14. Review and Test Plan

L0 reviewers:

- `codex challenge`
- `ce-feasibility-reviewer` or equivalent validation feasibility reviewer
- `ce-security-lens-reviewer` because W5-B touches privacy, MCP/headless parity, sensitivity, claims, and filesystem source evidence
- K-in: `ce-learnings-researcher` citing `docs/solutions/` and `.docs/decisions/`

L1 verification:

- `cargo clippy -- -D warnings`
- `cargo test`
- `pnpm exec tsc --noEmit`
- `pnpm test` only if W5-B changes frontend or WP validation code
- `bash tests/v146_validation/run.sh all`
- `bash scripts/release-gate/run-v146-validation.sh`
- `pnpm release-gate -- --mode hermetic`
- `tests/v146_validation/redaction_lint.sh` over report/proof/log/Linear-ready artifacts

L2 reviewers:

- Bounded default diff review
- Security reviewer for privacy, sensitivity, MCP, filesystem, claims, and service mutation paths
- Testing reviewer for validation strength
- Data migration reviewer only if W5-B unexpectedly adds a migration, which requires a packet amendment

---

## 15. Remaining L0 Questions

1. Should the v146 release-gate invariant be embedded directly in `src-tauri/src/release_gate.rs`, or should it be registered through an existing script-evidence pattern if one is preferred by the release-gate owner?
2. If the MCP placement handler remains unavailable after W4/W5-A merge, which owner issue should block W5-B: W4-C, DOS-168, or a new DOS-476 predecessor?
3. If source-management lacks ignore/scratchpad or archive/delete actions, should those become one split issue or one issue per lifecycle behavior?

These are routing questions only. They do not permit a blocked axis to count as green.

---

## 16. Done Criteria

W5-B is done only when:

- All seven mandatory validation axes are green.
- No axis remains blocked or failed.
- `validation-report.md` is written and linked from the wave plan.
- v146 validation is wired into `pnpm release-gate -- --mode hermetic` as a mandatory invariant and passes.
- Hermetic graph service assertions report zero attribution gaps for explicitly ingested sources.
- W5-A backfilled pending-review sources produce zero default claims and stay out of default graph/context reads.
- Entity intake and `_inbox` each prove ingestion -> `commit_claim` -> provenance -> graph/context behavior.
- MCP/work-product placement proves actual MCP/gateway or registered-handler behavior, or W5-B remains blocked.
- Trust-band discipline is proven for recent confirmed, old confirmed, pending-review backfilled, and promotion/reingest-without-freshness-attestation cases.
- The literal release-gate signal chain `WorkspaceFileIngested -> EntityIntelligenceUpdated -> prep invalidation` is green.
- Lifecycle actions for correction/typed feedback, relink, quarantine, ignore/scratchpad, archive/delete, and promotion/reingest are green through real service APIs.
- Prompt-channel sensitivity sweep is green for `user_only` and other disallowed sensitivity classes before page/limit caps.
- Filesystem negative fixtures and invalid path/input rejection cases are green and privacy-safe, with platform-gated subcases explicitly recorded.
- User correction round trip changes claim/source state and re-renders through existing services.
- `cargo clippy -- -D warnings`, `cargo test`, `pnpm exec tsc --noEmit`, `bash tests/v146_validation/run.sh all`, `bash scripts/release-gate/run-v146-validation.sh`, and `pnpm release-gate -- --mode hermetic` pass.
- Redaction lint passes on validation report, proof bundle, generated logs, and Linear-ready text.
- Manual real-workspace evidence is captured in the allowed redacted form only.
