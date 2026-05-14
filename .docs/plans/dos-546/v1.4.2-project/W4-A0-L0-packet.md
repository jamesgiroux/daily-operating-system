# W4-A0 L0 packet — `dailyos/account-overview` Composition-producing ability

## Header

Issue: DOS-568

Project issue family: DOS-546

Wave: 4 stage-2 producer

Branch: `dos-546-w4-a0-l0-prep`

Working tree: `/private/tmp/dailyos-w4-a0-l0-prep`

Packet output: `.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md`

Prepared: 2026-05-13

Primary spec: `.docs/plans/dos-546/phase-0/14-gutenberg-block-account-overview.md`

Structural model: `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md`

Upstream contract: W4-B `commit_composition` and watermark substrate.

Downstream consumer: W4-A renderer through `SurfaceClientBridge`.

Ability name: `dailyos/account-overview`

Ability category: `Read`

Output: `AbilityOutput<Composition>`

Mutation stance: no claim mutation; `MutatingProposal` does not apply.

Composition finalization: consumes W4-B `commit_composition` once W4-B lands.

Core acceptance: declare policy, compose from account claims, produce deterministic Composition, prove bridge invocation.

Linear fetch note: `gh issue view https://linear.app/a8c/issue/DOS-568` is not a valid `gh issue` target in this environment; scope below is inferred from the supplied DOS-568 acceptance criteria plus the W4 wave plan.

## Changelog

- **V5 (2026-05-13):** Codex Cycle 4 caught two leftover "four" references in the prose around §2 after V4 patched the bullet list. V5 substitutes "three" and "three policy macro keys above" in both spots. No structural change.
- **V4 (2026-05-13):** Codex Cycle 3 CONDITIONAL fold — one item: `mutates` is NOT a `#[ability]` macro key (the macro synthesizes it from detected mutation paths; declaring `mutates = []` errors as unknown attribute). §2 macro-key list updated to remove `mutates` and add explanatory paragraph; registry-projection test asserts empty mutates on the materialized descriptor. Items 2-4 (BlockType mapping, trust-band table, audit `wp_user_id`) already ADDRESSED in V3 per codex re-verification.
- **V3 (2026-05-13):** Cycle 2 reviewer fold — 4 V3-fixable items closed. (a) **Macro fields are real** (eng + codex verified at `abilities-macro/src/lib.rs:795-798`); §2 §"Policy fields set through registry or policy-builder" section rewritten as §"Macro keys for `AbilityPolicy` fields" — `required_scopes`, `mcp_exposure`, `client_side_executable`, `mutates` declared directly via `#[ability]`, not separated. (b) **Claim-type → BlockType mapping pinned** to canonical enum at `composition.rs:298-309` via explicit table; `Custom` reserved for downstream abilities; CI exhaustiveness gate on new claim types. (c) **Trust-band fallback table pinned** by `(claim_state, source_asof freshness, trust_score present)` triple with explicit `likely_current` / `use_with_caution` / `needs_verification` / "excluded entirely" mapping; freshness_class per claim-type lives in `trust.rs`. (d) **Audit emission `wp_user_id` plumbing** (cso H1) — ac §55 expanded to require `actor.session.wp_user_id` plumbing into `AuditFields.wp_user_id`, with explicit CI assertion that `emit_surface_audit` returns `Ok(_)` (not `SurfaceClientMissingWpUserId`).
- **V2 (2026-05-13):** Cycle 1 reviewer fold — eng + cso + devex CONDITIONAL APPROVE items and codex BLOCK items folded into closure requirements.
- Adds hard `ClaimRef::with_field` / `Block.field_bindings` enforcement for rendered Source and FeedbackTarget fields, with ComputedFrom and DisplayOnly roles pinned.
- Splits determinism into pure builder determinism and committed-output determinism so W4-A0 no longer contradicts W4-B's every-accepted-commit-advances-version rule.
- Pins actual SurfaceClient scope enforcement as a closure condition in `src-tauri/src/bridges/surface_client.rs`; missing `read.account_overview` must reject before ability internals.
- Pins ADR-0105 trust derivation, freshness caps, unknown `source_asof` warnings, and most-cautious block trust across Source and ComputedFrom bindings.
- Adds W4-A0 invalidation behavior for DOS-589 claim/source/dismissal signals and re-composition through `commit_composition`.
- Closes `commit_composition` ownership: W4-A0 builds `CompositionProposal`, calls the W4-B finalizer seam directly, and returns committed `AbilityOutput<Composition>`.
- Closes `client_side_executable`: `false` is correct because the Gutenberg renderer invokes server-side through the PHP runtime SurfaceClient.
- Keeps direct claim-reader use as required v1 path; `get_entity_context` composition is explicitly out until its SurfaceClient TODOs close.
- Adds cso fixtures and gates for surface-scoped claim loading, sensitivity exclusion, and `emit_surface_audit(event_kind = ability_invoked)`.
- Splits macro attribute keys from `AbilityPolicy` fields per actual `get_entity_context.rs:52` grammar.
- Inherits W4-B V8: `bridges/surface_client.rs` canonical route module, wp_user_id session binding at bridge layer, and W4-D `project_composition_for_surface` interlock.
- **V1 (2026-05-13):** Initial L0 Prep packet for DOS-568 / W4-A0.
- Mirrors the W4-B packet section order requested for this work.
- Carries W4-B contracts forward only where W4-A0 consumes them.
- Explicitly marks `SurfaceClientBridge` concrete path as an open substrate-location question because this branch exposes generic bridge primitives but no concrete `SurfaceClientBridge` type.
- Keeps net-new W4-A0 scope minimal: ability declaration, composition assembly, bridge contract proof, deterministic fixture.
- Confirms ADR-0080, ADR-0102, ADR-0105, and ADR-0130 exist before citing them.
- Treats the ability as Read despite consuming `commit_composition`: the domain read produces a composition from existing claims; W4-B owns watermark finalization and outbox semantics.
- Pins `MutatingProposal` / `ClaimMutationTarget` as non-applicable for W4-A0 because this ability does not call `commit_claim`.
- Adds negative fixtures for missing account id, unauthorized scope, empty claim set, and absent trust band.
- Adds CI invariants for registry declaration, scope gate, seeded determinism, and bridge invocation contract.

## Status snapshot

- **L0 Prep state:** V2 packet folded from Cycle 1 reviewer findings; no implementation edits made.
- **Branch confirmed:** `/private/tmp/dailyos-w4-a0-l0-prep` is on `dos-546-w4-a0-l0-prep`.
- **Output-only safety:** this packet is the only intended write.
- **Linear issue body:** not fetched through `gh`; `gh` rejected the Linear URL as an invalid GitHub issue target.
- **Scope source:** DOS-568 acceptance criteria supplied in the task plus W4 wave plan lines 397-403.
- **W4 stage status:** W4-A0 is stage-2 and starts after W4-B merges.
- **W4-B dependency:** W4-B provides `commit_composition`, `CompositionVersionEvent`, `BridgeSurfaceError` precedence, and the outbox table.
- **W4-B V8 dependency:** W4-B also provides `bridges/surface_client.rs`, `validate_session_bound_wp_user_id`, and the canonical W4-D projection interlock.
- **Current branch reality:** `commit_composition` is not present in this W4-A0 prep worktree; W4-A0 implementation must either rebase after W4-B or target the merged W4-B API.
- **Current branch reality:** `ClaimRef::with_field`, `FieldBinding`, and `BindingRole` are not present in this W4-A0 prep worktree; those are W4-B-owned surfaces.
- **W4-B V8 inheritance:** `src-tauri/src/bridges/surface_client.rs` is the canonical SurfaceClient route module; Open Q1 is closed.
- **Ability location decision:** new module under `src-tauri/abilities-runtime/src/abilities/account_overview.rs`.
- **Renderer status:** W4-A renderer is downstream; W4-A0 should not implement block PHP/JS, cached projections, signatures, or fallback renderer rules.
- **Trust status:** backend trust bands exist under `src-tauri/abilities-runtime/src/abilities/trust/` and provenance trust helpers exist under `src-tauri/abilities-runtime/src/abilities/provenance/trust.rs`.
- **Frontend trust render status:** `src/lib/trust-band.ts`, `src/components/ui/TrustBandBadge.tsx`, and `src/components/intelligence/TrustBand.tsx` already normalize/render the canonical trust-band strings.
- **Account-context status:** existing builders include `build_intelligence_context()` and `gather_account_context()`, but W4-A0 must use the direct claim-reader service path for v1 and must not compose `get_entity_context` until SurfaceClient TODOs close.
- **Risk focus:** accidental renderer work, accidental new claim model, accidental direct DB read/write, bridge leakage on unauthorized scopes, missing field bindings, and over-including sensitive claims.

## Pre-work confirmed substrate reuse audit

**Headline finding:** W4-A0 should be a thin producer over already-landed substrate. It should not create a parallel account model, a parallel trust-band model, a renderer, a new transport, or a claim write path.

### ADR existence check

- Verified: `.docs/decisions/0080-signal-intelligence-architecture.md`
- Verified: `.docs/decisions/0102-abilities-as-runtime-contract.md`
- Verified: `.docs/decisions/0105-provenance-as-first-class-output.md`
- Verified: `.docs/decisions/0130-surface-independent-composition-contract.md`
- No unverified ADR number is used as an implementation authority in this packet.

### W4-B contracts carried forward

- `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md` §3 defines `commit_composition`.
- §3 signature carried forward: `commit_composition(ctx, db, proposal: CompositionProposal) -> Result<CommittedComposition, CompositionError>`.
- §3 states `CompositionProposal` carries `composition_id`, `expected_composition_version`, and `composition`.
- §3 states `CommittedComposition` returns server-assigned `composition_version`.
- §3 authority rule: `composition_version` is assigned inside `commit_composition`, not trusted from the ability.
- §3 idempotency rule: every accepted `commit_composition` advances the version.
- §3 outbox rule: `CompositionVersionEvent` is inserted in the same transaction as the version mutation.
- §3 concurrent producer rule: two producers for the same `composition_id` race through CAS; one succeeds, one gets `StaleVersion`.
- `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md` §5 defines the signal payload schema.
- §5 carries `CompositionVersionEvent { event_kind, composition_id, previous_version, current_version, cursor, reason }`.
- §5 assigns delivery and subscriber replay to DOS-589, not W4-A0.
- `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md` §6.5 defines bridge error precedence.
- §6.5 precedence is inherited; W4-A0 does not add a new precedence ladder.
- `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md` §13 defines `MutatingProposal` and `ClaimMutationTarget`.
- §13 is explicitly non-applicable to W4-A0 because W4-A0 is a Read ability and does not call `commit_claim`.
- `/private/tmp/dailyos-w4-l0-prep/.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md` §15 defines the `version_events` outbox table.
- §15 outbox pattern is inherited only through `commit_composition`; W4-A0 does not write `version_events`.
- W4-B acceptance criteria 3-5 apply to W4-A0 only as consumer obligations.
- W4-B acceptance criterion 3: composition versions are assigned exclusively inside `commit_composition`.
- W4-B acceptance criterion 4: `commit_composition` uses `BEGIN IMMEDIATE` transactional CAS.
- W4-B acceptance criterion 5: `ClaimRef.field_path` and `Block.field_bindings` are W4-B substrate surfaces W4-A0 consumes.
- W4-B acceptance criterion 10 is claim-mutation-only; W4-A0 must not implement or satisfy `MutatingProposal`.
- W4-B acceptance criterion 11 is mutation-proposal-only; W4-A0 must not emit mutation proposal bindings.
- W4-B acceptance criterion 12 is lock/CAS substrate; W4-A0 relies on it through W4-B.
- W4-B acceptance criterion 13 is the three-Tx outbox protocol; W4-A0 consumes it by calling `commit_composition`.
- W4-B V8 §37 is inherited: `src-tauri/src/bridges/surface_client.rs` is the canonical module for `/v1/surface/*` route ownership and SurfaceClient bridge tests.
- W4-B V8 §17 is inherited at bridge layer: any request payload containing `wp_user_id` must match the W2-C session-bound value before dispatch; W4-A0 has no direct domain obligation because this Read ability input does not accept `wp_user_id`.
- W4-B §16 scope-filter shape is inherited for W4-A0 claim loads: the SurfaceClient actor's scopes must drive surface projection before any claim body reaches composition assembly.
- W4-B acceptance criterion 17 is consumer-critical: every feedback-eligible Source or FeedbackTarget reference must use `ClaimRef::with_field(claim_id, version, field_path)`.
- W4-B acceptance criterion 5 is consumer-critical: every `Block` must populate `field_bindings`; `Vec::new()` is only a back-compat constructor default, not acceptable W4-A0 output for rendered intelligence blocks.

### Ability runtime substrate

- `src-tauri/abilities-runtime/src/abilities/registry.rs` has `AbilityCategory::Read`.
- ADR-0102 describes Read abilities as no domain mutation in the call graph.
- `registry.rs` carries `Actor::SurfaceClient { instance, scopes }`.
- `registry.rs` carries `ActorKind::SurfaceClient`.
- `registry.rs` carries `SurfaceScope` and `ScopeSet`.
- `registry.rs` lines 432-480 define `AbilityPolicy` fields.
- `AbilityPolicy` includes `allowed_actors`, `required_scopes`, `mcp_exposure`, and `client_side_executable`.
- `AbilityPolicy::default()` keeps `mcp_exposure: None` and `client_side_executable: false`.
- `AbilityPolicy::required_scopes_typed()` materializes `required_scopes` into typed scopes.
- Registry build seeds the global scope allowlist from registered descriptors.
- Registry invocation policy checks actor kind before dispatch.
- Current bridge gap to close: `src-tauri/src/bridges/types.rs::resolve_pre_dispatch` checks `allowed_actors` and schema closure, but does not yet compare `Actor::SurfaceClient.scopes` against `descriptor.policy.required_scopes`.
- W4-A0 closure condition: the actual `SurfaceClientBridge` path must enforce `read.account_overview` before ability internals, claim reads, or audit success emission run.
- Existing macro examples: `get_entity_context` uses an ability declaration under `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs`.
- W4-A0 should follow the same module/import style instead of a custom registry path.

### Composition substrate

- `src-tauri/abilities-runtime/src/abilities/composition.rs` is the authoritative model.
- Lines 1-21 document substrate-owned composition authorship and provenance-lives-once.
- Lines 52-70 define `CompositionDocId`.
- Lines 107-125 define `CompositionVersion`; current branch still uses `saturating_add`, but W4-B §4 replaces this with checked overflow behavior.
- Lines 131-154 define `ClaimRef` with `claim_id` and `claim_version`.
- W4-B adds `ClaimRef.field_path` / `ClaimRef::with_field()`; not present in this worktree.
- Lines 252-287 define `ProvenanceRef`.
- Lines 294-310 define `BlockType`, including `AccountOverview`.
- Lines 367-378 define `CompositionMetadata`.
- Lines 390-417 define `Section`.
- Lines 419-520 define `Block`.
- Lines 522-585 define `Composition`.
- Lines 555-570 make `Composition::new` `pub(crate)`, matching ADR-0130 substrate-owned authorship.
- Lines 667-671 define fallback trust cap `NeedsVerification`.
- W4-A0 should construct `Composition` inside the abilities-runtime crate, not in Tauri commands or WP code.

### Ability output and provenance substrate

- `src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs` lines 302-335 define `AbilityOutput<T>`.
- `AbilityOutput<T>` carries `data`, `provenance`, `ability_version`, and `diagnostics`.
- ADR-0102 Rule 5 says provenance lives exactly once on `AbilityOutput<T>`.
- ADR-0130 §2 repeats that `Composition` is the domain output and blocks carry `ProvenanceRef`.
- `src-tauri/abilities-runtime/src/abilities/provenance/builder.rs` applies field trust bands into `field_attributions`.
- `src-tauri/abilities-runtime/src/abilities/provenance/trust.rs` exposes `claim_trust_band_from_score` and `most_cautious_trust_band`.
- W4-A0 should use the existing provenance builder/envelope path, not embed provenance inside block attributes.

### Account context and claim-reader substrate

- `src-tauri/src/intelligence/prompts.rs` defines `IntelligenceContext` and `build_intelligence_context()`.
- `build_intelligence_context()` lines 267-276 gather account/project context from DB and prior intelligence.
- `build_intelligence_context()` lines 285-360 build account facts from account DB fields, products, team, and source refs.
- `src-tauri/src/context_provider/local.rs` lines 1-4 explicitly wrap `build_intelligence_context()`.
- `src-tauri/src/context_provider/local.rs` lines 59-68 delegate to `build_intelligence_context()`.
- `src-tauri/src/prepare/meeting_context.rs` lines 274-283 define `gather_account_context()`.
- `gather_account_context()` lines 319-423 reads recent captures, actions, meetings, products, account team, sourced facts, and technical footprint.
- `gather_account_context()` is meeting-prep oriented and private; W4-A0 should not depend on its JSON shape unless it is deliberately promoted.
- `src-tauri/abilities-runtime/src/services/context.rs` lines 830-842 define narrow read seams on `ServiceContext`.
- `src-tauri/abilities-runtime/src/services/context.rs` lines 857-873 define `EntityContextClaimReadHandle`.
- `src-tauri/abilities-runtime/src/services/context.rs` lines 1101-1119 expose `read_entity_context_claims()`.
- `src-tauri/src/services/context.rs` wires that trait to `services::claims::load_entity_context_claims_active_for_surface`.
- `src-tauri/src/services/claims.rs` lines 7904-7933 provide `load_claims_active` and `load_claims_active_for_surface`.
- `src-tauri/src/services/claims.rs` lines 8058-8088 provide `load_entity_context_claims_active_for_surface`.
- Preferred W4-A0 read path: use the narrow claim-reader seam in `ServiceContext` when inside an ability; do not pass raw DB handles into ability internals.

### Claim model substrate

- `src-tauri/abilities-runtime/src/types.rs` lines 40-74 define `IntelligenceClaim`.
- `IntelligenceClaim` includes `subject_ref`, `claim_type`, `field_path`, `text`, `data_source`, `source_ref`, `source_asof`, `observed_at`, `provenance_json`, lifecycle states, `trust_score`, temporal scope, sensitivity, and verification fields.
- `src-tauri/src/services/claims.rs` lines 3744-3785 read those columns from `intelligence_claims`.
- `src-tauri/abilities-runtime/src/abilities/claims.rs` defines the claim taxonomy.
- Claim metadata includes default temporal scope, sensitivity, freshness decay class, commit policy class, canonical subject types, and actor classes.
- Account-relevant claim types already include `risk`, `win`, `entity_risk`, `entity_win`, `stakeholder_engagement`, `stakeholder_assessment`, `value_delivered`, `commitment`, and `company_context`.
- W4-A0 should select and order from this claim substrate; it should not add new claim columns or new claim types for the first account overview.
- If a claim type needed by the fixture is missing, that is an input-fixture adjustment or upstream claim-registry question, not W4-A0 scope.

### Trust-band rendering substrate

- Backend trust enum lives under `src-tauri/abilities-runtime/src/abilities/trust/types.rs`.
- Backend trust compiler helpers live under `src-tauri/abilities-runtime/src/abilities/trust/mod.rs`.
- Provenance trust helper `claim_trust_band_from_score()` maps score to `LikelyCurrent`, `UseWithCaution`, `NeedsVerification`, or `Unscored`.
- `src/lib/trust-band.ts` lines 10-15 defines wire bands `likely_current`, `use_with_caution`, `needs_verification`, and `unscored`.
- `src/lib/trust-band.ts` lines 41-55 normalizes/extracts trust bands from rendered provenance.
- `src/lib/trust-band.ts` lines 66-104 partitions evidence by trust band.
- `src/components/ui/TrustBandBadge.tsx` lines 6-27 defines visual labels for the three user-facing bands.
- `src/components/intelligence/TrustBand.tsx` lines 59-96 composes badge, provenance tag, and freshness indicator.
- W4-A0 output should carry canonical trust-band strings and field attribution; W4-A decides visual rendering.

### SurfaceClientBridge substrate

- W4-B V8 promotes `src-tauri/src/bridges/surface_client.rs` as the canonical route module for `/v1/surface/*` endpoints.
- Open Q1 is closed: W4-A0 bridge contract tests target `bridges/surface_client.rs`, not an invented parallel module.
- Existing generic bridge primitives still live under `src-tauri/src/bridges/types.rs`, `tauri.rs`, `mcp.rs`, `worker.rs`, and `eval.rs`.
- `src-tauri/src/bridges/types.rs` lines 421-455 provide pre-dispatch actor/mode/schema checks.
- The known gap is scope enforcement: `resolve_pre_dispatch` does not yet compare `Actor::SurfaceClient.scopes` with `descriptor.policy.required_scopes`.
- W4-A0 does not paper over that gap inside the ability; L0 closure requires the real SurfaceClient bridge path to reject missing `read.account_overview` before `account_overview` code runs.
- The bridge proof must spy on the claim-reader seam and assert it is not called on missing scope.
- W4-B §17 `wp_user_id` session binding is inherited at request-entry layer; W4-A0 remains read-only and does not accept a `wp_user_id` domain field.
- Successful SurfaceClient invocation must emit a surface audit event after commit, not before, because the audit row carries the assigned `composition_version`.

### Intelligence Loop integration check

1. Claim model: W4-A0 does not create new claim fields; it renders existing account claims through field-aware `ClaimRef` and `FieldBinding` roles.
2. Provenance + trust: provenance remains on `AbilityOutput<Composition>`; blocks carry `ProvenanceRef`; trust bands come from existing provenance/trust helpers plus the V2 freshness cap algorithm.
3. Signals + invalidation: W4-A0 consumes W4-B `CompositionVersionEvent` emission through `commit_composition` and declares the DOS-589 triggers that must re-run the producer.
4. Runtime + surfaces: ability is invoked through registry/SurfaceClientBridge; Tauri/MCP consume the same `AbilityOutput<Composition>` without renderer-specific HTML.
5. Feedback loop: W4-A0 preserves `(claim_id, claim_version, field_path)` for Source and FeedbackTarget fields so W5-A feedback can route corrections/dismissals to the claim substrate.

## What W4-A0 authors net-new table

| Surface | Net-new? | W4-A0 ownership |
|---|---:|---|
| `src-tauri/abilities-runtime/src/abilities/account_overview.rs` | Yes | New Read ability module with input/output types, composition assembly, tests. |
| Ability declaration for `dailyos/account-overview` | Yes | Macro keys plus policy-builder fields: `allowed_actors: [User, SurfaceClient]`, `required_scopes: ["read.account_overview"]`, `mcp_exposure: Invocable`, `client_side_executable: false`. |
| Composition block assembly | Yes | Create `CompositionKind::EntityPage`, subject account ref, mapped block types for risk/win/value/commitment/current-state claims, and field-aware bindings from existing claims. |
| Provenance field attribution for blocks | Yes | Build/attach canonical provenance once on `AbilityOutput<Composition>` and point blocks at field paths. |
| Field binding topology | Yes | Every block gets `Block.field_bindings`; Source and FeedbackTarget require `ClaimRef::with_field`, summaries use ComputedFrom, decorative/empty fields use DisplayOnly. |
| Use of W4-B `commit_composition` | Yes, as consumer | Call the W4-B chokepoint or finalizer exactly once; never assign `composition_version` locally. |
| Seeded deterministic fixture | Yes | Generic account fixture with stable clock, sorted claims, known trust bands, expected JSON snapshot. |
| SurfaceClientBridge contract test | Yes | Prove renderer-style SurfaceClient actor with `read.account_overview` invokes and receives `AbilityOutput<Composition>`. |
| Surface audit | Yes | Successful Invocable SurfaceClient calls emit `emit_surface_audit(event_kind = ability_invoked)` after commit with actor, account, composition, and claim-ref counts. |
| New account claim type | No | Reuse existing claim registry. |
| New trust-band enum | No | Reuse ADR-0105/W4 trust helpers and wire labels. |
| New renderer | No | W4-A owns Gutenberg render code. |
| New transport/HMAC/pairing | No | W2 owns transport and pairing. |
| New fallback projection rules | No | W4-D owns unknown-block fallback. |
| New signature/tamper logic | No | W4-C owns Ed25519 projection signing/verification. |
| New feedback router | No | W5-A owns correction/dismissal routing. |
| `MutatingProposal` implementation | No | W4-A0 is not a claim mutation producer. |
| Direct DB writes | No | All writes, if any, go through W4-B `commit_composition`; ability reads through service seams. |
| Signal dispatcher implementation | No | DOS-589 owns delivery; W4-A0 only declares invalidation triggers and re-invokes through its normal ability/finalizer path. |

## Directional decisions

### §1. Ability lives in `abilities-runtime`

- W4-A0 lives at `src-tauri/abilities-runtime/src/abilities/account_overview.rs`.
- Add `pub mod account_overview;` in `src-tauri/abilities-runtime/src/abilities/mod.rs`.
- Register through the existing ability macro/inventory path, not a hand-written bridge map.
- Use ability name `dailyos/account-overview` because the Gutenberg block spec names `dailyos/account-overview`.
- If current macro only admits snake-case names, implementation must either use the canonical external name in descriptor metadata or update the macro deliberately with a registry test.
- Do not place W4-A0 under `src-tauri/src/services/`; services provide data, abilities produce product output.
- Do not place W4-A0 under `src-tauri/src/bridges/`; bridges invoke abilities, they do not author composition.

### §2. Policy declaration is part of acceptance

**Macro attribute keys, per `get_entity_context.rs:52`:**

- `name = "dailyos/account-overview"` or a canonical external-name metadata mapping if the macro cannot accept slash names.
- `category = Read`.
- `version` and `schema_version` pinned in the ability declaration.
- `allowed_actors = [User, SurfaceClient]` only if the macro supports `SurfaceClient`; otherwise the descriptor/policy builder must supply SurfaceClient and the registry test must prove the projection.
- `allowed_modes = [Live]` unless the implementation deliberately adds Evaluate with fixture-backed behavior.
- `requires_confirmation = false`.
- `may_publish = false`.
- `composes = []`; v1 uses direct claim-reader services and must not compose `get_entity_context` until that ability's SurfaceClient TODOs close.
- `experimental = false`.
- `signal_policy = { emits_on_output_change = [], coalesce = false }` unless the registry requires a composition-output-change signal hook.

**Macro keys for `AbilityPolicy` fields** (V3 correction per eng + codex Cycle 2: these ARE macro keys at `abilities-macro/src/lib.rs:795-798`, NOT registry/policy-builder fields):

- `required_scopes = ["read.account_overview"]` — declared directly on `#[ability]`.
- `mcp_exposure = Invocable` — declared directly on `#[ability]`.
- `client_side_executable = false` — declared directly on `#[ability]`.

**`mutates` is NOT a macro key (V4 correction per codex Cycle 3):** the macro synthesizes `mutates` from detected mutation paths in the ability body (`detected.iter()` → `mutates_exprs` per `abilities-macro/src/lib.rs`). Declaring `mutates = []` as a `#[ability]` attribute fails compilation (unknown attribute). For W4-A0 (Read-only), the expectation is that the macro detector finds no mutation paths and synthesizes an empty `mutates` list automatically. The registry-projection test asserts `descriptor.policy.mutates.is_empty()` on the materialized descriptor — not on macro-key presence.

V2's "policy-builder fields" framing was over-defensive — the macro grammar admits the three policy macro keys above (`required_scopes`, `mcp_exposure`, `client_side_executable`). V3 places them in the macro invocation alongside the other declared keys. The macro expansion materializes them into `AbilityPolicy` automatically; a registry-projection test asserts the values surface correctly on the `AbilityDescriptor`.

**Behavioral notes:**

- SurfaceClient scope enforcement is not satisfied by declaration alone; the actual bridge path must compare `Actor::SurfaceClient.scopes` with `descriptor.policy.required_scopes` before ability internals (this gap is in the substrate, not W4-A0's authoring obligation).
- Phase 0 artifact 14 is compatible with `client_side_executable = false`: Gutenberg invokes server-side through the PHP runtime client, which authenticates as a SurfaceClient and calls the server ability.
- The registry declaration test must fail if any of the three policy macro keys above drifts from its declared value.

### §3. Composition shape for renderer

- Return committed `AbilityOutput<Composition>`, not renderer HTML.
- `Composition.kind`: `CompositionKind::EntityPage`.
- `Composition.subject`: account entity ref for the requested account id.
- `Composition.generated_by`: `dailyos/account-overview`.
- `Composition.metadata.generated_by`: same ability name.
- `Composition.metadata.composition_version`: `0` placeholder on proposal input; W4-B overwrites on commit.
- `Composition.sections`: deterministic sections from claim groups.
- Minimum viable section set:
- `overview` section with an `AccountOverview` block.
- `signals` section with mapped claim summary/evidence blocks when visible claims exist.
- `empty` state section only when the account exists but no visible claims resolve.
- **Claim-type to `BlockType` mapping (V3 pinned to canonical enum at `composition.rs:298-309`):**

| Claim type | `BlockType` variant | Notes |
|---|---|---|
| `risk`, `entity_risk` | `BlockType::RiskCallout` | One block per active risk claim |
| `win`, `entity_win` | `BlockType::ClaimSummary` | `attributes.intent = "win"` discriminator |
| `value_delivered` | `BlockType::ClaimSummary` | `attributes.intent = "value"` discriminator |
| `commitment` | `BlockType::ActionList` | Items aggregated by claim_id |
| `stakeholder_engagement`, `stakeholder_assessment` | `BlockType::RelationshipMap` | One block per stakeholder identity |
| `current_state`, `health_signal` | `BlockType::HealthSnapshot` | One block per account state snapshot |
| `company_context`, other generic account claims | Folded into `BlockType::AccountOverview` (the account block itself) — NOT a new block |
| Evidence text from claim sources | `BlockType::EvidenceList` | Only when block needs separate evidence list; otherwise inline |

The mapping is implemented as a closed `match` over the substrate's claim_type taxonomy returning a known `BlockType` variant. CI invariant: exhaustiveness gate fails the build if a new claim type added to the substrate has no `BlockType` mapping declared. Unknown claim types FAIL the build, not silently drop to `BlockType::Custom { type_id }` — `Custom` is reserved for downstream-ability blocks not in W4-A0's taxonomy.
- Every rendered claim reference for a Source or FeedbackTarget field uses `ClaimRef::with_field(claim_id, claim_version, field_path)`.
- Every `Block` populates `Block.field_bindings`; no W4-A0 output block may rely on constructor default `Vec::new()`.
- Source bindings identify directly rendered claim fields.
- FeedbackTarget bindings identify fields eligible for correction/dismissal routing.
- Computed summaries use `BindingRole::ComputedFrom` with all contributing field-aware refs.
- Decorative, empty-state, separator, and copy-only fields use `BindingRole::DisplayOnly` and carry no feedback target.
- Source and FeedbackTarget bindings reject `claim_ref.field_path = None` at build time or fixture assertion time.
- Blocks carry `ProvenanceRef` into the top-level envelope.
- Do not duplicate full provenance or raw source payloads in `Block.attributes`.
- Do not emit substrate-authored HTML.
- Do not include customer-specific strings in fixtures or snapshots.

### §4. Existing trust-band helper feeds the Composition

- Input eligibility runs before trust derivation: non-active, non-surfaced, tombstoned, superseded, withdrawn, dismissed, or sensitivity-excluded claims are not scored, counted, or rendered.
- Use `src-tauri/abilities-runtime/src/abilities/provenance/trust.rs::claim_trust_band_from_score` for the initial per-claim band from `IntelligenceClaim.trust_score`.
- For the user-facing three bands, output canonical strings: `likely_current`, `use_with_caution`, `needs_verification`.
- `unscored` may exist only as internal/wire fallback; renderer-facing account-overview blocks must cap it to a visible caution band.
- **Trust-band fallback table (V3 pinned per codex C2 V3-item-3):**

| `claim_state` | `source_asof` freshness | `trust_score` present | Resolved band |
|---|---|---|---|
| `active` | < 7 days | yes | `likely_current` (use `claim_trust_band_from_score` directly) |
| `active` | 7-30 days | yes | `use_with_caution` (cap on score-derived band) |
| `active` | > 30 days OR unknown | any | `needs_verification` (cap on score-derived band) |
| `active` | any | absent | `needs_verification` (never `likely_current` without a score) |
| `tombstoned`, `superseded`, `withdrawn`, `dismissed`, or sensitivity > `read.account_overview` | — | — | Claim excluded entirely — no block, no ClaimRef, no count (per /cso H2) |

Freshness-class durability is per-claim-type: fast-decay (e.g. `engagement_state`, `stakeholder_sentiment`) ages on the 7d/30d boundaries above; durable account facts (e.g. `company_context.industry`, `account.country`) skip the freshness cap entirely (treated as `likely_current` when active + scored). The exact per-claim-type freshness class lives in a `freshness_class` table inside `trust.rs` (W4-A0's first commit pins values for the current claim taxonomy; new claim types must declare a class at registration time via the same exhaustiveness gate as the BlockType mapping).
- If a source is revoked or marked unreliable before composition, exclude the claim unless the claim substrate still exposes an active surfaced replacement.
- Block trust is `most_cautious_trust_band` across every contributing Source and ComputedFrom binding.
- DisplayOnly bindings do not strengthen trust.
- FeedbackTarget bindings inherit the trust of the Source or ComputedFrom binding they point at; they do not independently raise the band.
- Renderer mode (`full`, `compact`, `icon`) remains W4-A/UI scope; W4-A0 output is mode-independent.

### §5. Determinism strategy

- Split determinism into two fixtures: pure builder determinism and committed-output determinism.
- Pure builder determinism does not call `commit_composition`.
- Pure builder fixture compares the proposed `Composition` plus provenance after normalizing expected runtime-only envelope fields.
- Committed-output determinism calls `commit_composition`, but each comparison run uses a cloned fixture DB or rolls back to the same pre-commit state.
- Do not assert byte-stable output after two accepted commits to the same live DB; W4-B explicitly advances `composition_version` on every accepted commit.
- Seed fixture with generic account id such as `acct-fixture-1`.
- Use injected service clock, not `Utc::now()`.
- Inject deterministic composition id, block ids, and stable invocation ids for fixture output.
- Pass identical `expected_composition_version` into each committed-output comparison; bootstrap uses `0`.
- Sort claims before composition assembly.
- Primary sort: salience/trust policy bucket.
- Secondary sort: claim type stable order.
- Tertiary sort: `source_asof` descending with unknown last.
- Final sort: `claim_id` lexicographic.
- Use `BTreeMap` / `BTreeSet` where map iteration reaches output.
- Derive deterministic block ids from `composition_id`, section key, claim id, and field path.
- Normalize JSON snapshots before comparing.
- Pin expected committed output after W4-B overwrites `composition_version` from the same cloned/rolled-back starting state.

### §6. Error precedence inherits W4-B

- Pre-dispatch unknown ability, actor denial, mode denial, schema closure, and reserved input fields continue to map to `AbilityUnavailable`.
- SurfaceClient missing `read.account_overview` must be rejected by the actual `src-tauri/src/bridges/surface_client.rs` path before ability internals run.
- The known `resolve_pre_dispatch` gap must be closed or bypassed only by the canonical SurfaceClient bridge adding equivalent required-scope enforcement.
- Missing `account_id` is input validation after actor/scope admission; it should not reveal account existence.
- Account not visible to the actor should map to a safe unavailable or validation surface consistent with the existing bridge policy.
- Sensitivity tighter than `read.account_overview` is not an error; those claims are excluded from the composition entirely.
- If `commit_composition` returns `Overflow`, W4-B §6.5 precedence maps composition overflow before stale composition.
- If `commit_composition` returns `StaleVersion`, W4-B §6.5 maps `StaleComposition` as the composition-level 409.
- W4-A0 must not introduce an alternate error envelope for stale composition.
- W4-A0 must not catch W4-B errors and downgrade them to generic success-with-warning.

### §7. Read ability plus `commit_composition`

- W4-A0 remains `Read` because it reads account claims and returns composed intelligence.
- The ability must not mutate claims, source rows, feedback rows, accounts, or external systems.
- `MutatingProposal` is not implemented.
- `ClaimMutationTarget` is not constructed.
- `commit_claim` is not called.
- The only write-adjacent behavior is W4-B composition watermark finalization.
- W4-A0 builds a `CompositionProposal` with deterministic `composition_id`, `expected_composition_version`, and proposal `composition`.
- Bootstrap call ergonomics are pinned: `expected_composition_version: 0` means first-ever commit under that `composition_id`.
- Proposal input ergonomics are pinned: `Composition.metadata.composition_version: 0` is a placeholder and is not trusted.
- W4-A0 calls the W4-B finalizer through a narrow service/finalizer seam directly from the ability path.
- W4-A0 returns committed `AbilityOutput<Composition>` after W4-B assigns `composition_version`.
- Treat `commit_composition` as substrate-owned Composition contract finalization, not a domain claim mutation.
- L0 reviewers have closed Open Q8 with this ownership model; bridge wrapping a raw uncommitted Composition is not the v1 path.

### §8. Claim selection is conservative

- Use active, surfaced account claims only.
- Use the surface-aware claim-reader seam with `Actor::SurfaceClient { scopes }` projection when invoked by the Gutenberg path.
- Do not call non-surface claim loaders for SurfaceClient invocations.
- Do not include dormant, tombstoned, withdrawn, superseded, contradicted, or surface-dismissed claims.
- Respect sensitivity before composition assembly.
- Claims requiring scopes tighter than `read.account_overview` are excluded entirely: no block, no `ClaimRef`, no `field_bindings`, no provenance field attribution, no count.
- Do not expose raw source snippets in attributes.
- Empty claim set returns a valid empty-state Composition, not a hard error.
- Missing account id returns validation/unavailable, not empty Composition.
- Missing trust band degrades visibly.

### §9. SurfaceClientBridge proof is contract-level

- W4-A0 does not implement the W4-A renderer.
- W4-A0 must include a test that invokes the ability as a `SurfaceClient` actor with `read.account_overview` through `src-tauri/src/bridges/surface_client.rs`.
- The proof must fail if dispatch goes through a helper that does not enforce `descriptor.policy.required_scopes`.
- The missing-scope test asserts the claim-reader seam is not called.
- The successful test asserts the bridge returns committed `AbilityOutput<Composition>`, not raw `Composition` data.
- The successful test asserts `emit_surface_audit` runs after commit with `event_kind = "ability_invoked"`.
- The successful audit detail carries actor, account_id, composition_id, composition_version, and claim_ref_count.
- Direct claim-reader use is required for v1. Do not compose `get_entity_context` until its SurfaceClient TODOs close and a later packet deliberately promotes that dependency.
- W4-B §17 `wp_user_id` precondition is verified at bridge layer only; W4-A0 does not add a domain-level `wp_user_id` field.

### §10. Signal invalidation and cache-bust behavior

- W4-A0 does not implement DOS-589 delivery, but its packet declares the signals that invalidate the account-overview composition.
- Included claim version events invalidate any composition whose field bindings include the changed `(claim_id, claim_version, field_path)` lineage.
- Account-subject claim changes invalidate account-overview composition for that account even when the changed claim was not previously rendered.
- Tombstone, supersede, withdrawal, contradiction, and correction events invalidate affected account-overview compositions.
- Surface dismissal changes invalidate only projections for the affected surface/account pair.
- Source freshness changes, source revocation, and source reliability downgrades invalidate any block whose Source or ComputedFrom binding depends on that source.
- Cache-bust marks the cached projection stale before any renderer fetch can reuse it as current.
- Recomposition re-invokes W4-A0 through the same SurfaceClient-authorized path, rebuilds the proposal, and calls `commit_composition`.
- Successful recomposition publishes a new composition version through W4-B's `commit_composition` outbox row.
- DOS-589 owns fan-out, cursor replay, backpressure, and subscriber delivery; W4-A0 owns correct dependency declaration and deterministic rebuild behavior.

## Acceptance criteria

1. Ability module exists at `src-tauri/abilities-runtime/src/abilities/account_overview.rs`.
2. Ability is registered in the abilities runtime registry/inventory path.
3. Ability name is externally `dailyos/account-overview`.
4. Ability category is `Read`.
5. Ability returns committed `AbilityOutput<Composition>`.
6. Macro declaration pins name, category, version, schema_version, allowed_actors, allowed_modes, confirmation, publish, composes, experimental, and signal_policy.
7. Policy builder or registry pins `allowed_actors: [User, SurfaceClient]`.
8. Policy builder or registry pins `required_scopes: ["read.account_overview"]`.
9. Policy builder or registry pins `mcp_exposure: Invocable`.
10. Policy builder or registry pins `client_side_executable: false`.
11. Ability does not implement `MutatingProposal`.
12. Ability does not construct `ClaimMutationTarget`.
13. Ability does not call `commit_claim`.
14. Ability does not write account, claim, feedback, source, or renderer state directly.
15. Ability reads existing account claims through service/context reader seams.
16. SurfaceClient claim loads route through `Actor::SurfaceClient { scopes }` projection per W4-B §16.
17. CI grep gate rejects account-overview claim loads using non-surface variants in the SurfaceClient path.
18. Ability composes from account claims already in the claim substrate.
19. Ability uses existing trust-band helper(s) plus V2 freshness caps for claim/block trust.
20. Ability uses existing provenance builder/envelope patterns.
21. Composition blocks carry `ClaimRef` entries for rendered claims.
22. Every rendered Source or FeedbackTarget claim ref uses `ClaimRef::with_field`.
23. Source and FeedbackTarget bindings reject `claim_ref.field_path = None`.
24. Every block populates `Block.field_bindings`.
25. Summaries use `BindingRole::ComputedFrom`.
26. Decorative and empty-state fields use `BindingRole::DisplayOnly`.
27. Composition blocks carry `ProvenanceRef` entries into the canonical output envelope.
28. Composition does not duplicate full provenance envelopes in block attributes.
29. Composition does not include raw source snippets in block attributes.
30. Composition uses canonical trust-band strings.
31. Unknown `source_asof` emits a provenance warning and caps trust.
32. Stale `source_asof` caps trust by freshness class.
33. Block trust is most-cautious across Source and ComputedFrom bindings.
34. Claims tighter than `read.account_overview` are excluded entirely: no block, no ClaimRef, no count.
35. Empty account claim set returns a valid empty-state Composition.
36. Missing account id is rejected.
37. Unauthorized scope is rejected by the actual SurfaceClient bridge before ability internals run.
38. Trust band absent fixture degrades visibly and deterministically.
39. Pure builder fixture produces deterministic proposal output without commit.
40. Committed-output fixture uses cloned or rolled-back fixture DB state.
41. Committed-output fixture passes identical `expected_composition_version` and expects identical assigned versions from identical starting state.
42. Fixture controls clock, composition id, block ids, and invocation ids.
43. Fixture uses generic account/customer data only.
44. W4-A0 consumes W4-B `commit_composition` after W4-B merges.
45. W4-A0 never assigns final `composition_version` locally.
46. Bootstrap passes `expected_composition_version: 0`.
47. Proposal input uses `Composition.metadata.composition_version: 0` placeholder.
48. W4-A0 surfaces W4-B `StaleComposition` without changing the error contract.
49. W4-A0 surfaces W4-B composition overflow without changing the error contract.
50. W4-A renderer can invoke W4-A0 through SurfaceClientBridge and receive committed `AbilityOutput<Composition>`.
51. Bridge contract test covers `Actor::SurfaceClient` with the right scope.
52. Bridge contract test covers `Actor::SurfaceClient` without the right scope.
53. Bridge test covers the `resolve_pre_dispatch` required-scope gap through the canonical SurfaceClient route.
54. Every successful Invocable SurfaceClient invocation emits `emit_surface_audit` with `event_kind = ability_invoked`.
55. **Audit detail carries `wp_user_id` from `actor.session.wp_user_id` (sourced from the W2-C-bound SurfaceClient session, NOT from any request body), plus `actor_instance` (SurfaceClient id), `account_id`, `composition_id`, `composition_version`, and `claim_ref_count`.** This satisfies `emit_surface_audit`'s mandatory `AuditFields.wp_user_id = Some(_)` requirement for `Actor::SurfaceClient` per `audit_log.rs:96` — without it the helper returns `SurfaceClientMissingWpUserId` and no audit row writes. The successful-call CI assertion (per ac §41) MUST assert `emit_surface_audit` returns `Ok(_)`, not just that it was called.
56. Registry declaration test pins policy fields.
57. Inventory test pins MCP exposure and SurfaceClient actor projection.
58. Output schema is closed and generated from the typed output.
59. Input schema is closed and rejects reserved actor/bridge fields.
60. W4-A0 declares DOS-589 invalidation triggers for claim, account-subject, lifecycle, dismissal, source freshness, and revocation events.
61. Cache-bust marks cached projection stale before renderer reuse.
62. Recomposition publishes a new composition version through `commit_composition`.
63. W4-A0 does not compose `get_entity_context` in v1.
64. `cargo clippy -- -D warnings` passes.
65. `cargo test` passes.
66. `pnpm tsc --noEmit` passes if TypeScript surface typing is touched by contract fixtures.

## Negative fixtures

1. **`dos568_fixture_missing_account_id.rs`**
2. Input omits or blanks `account_id`.
3. Actor otherwise valid; scope grant includes `read.account_overview`.
4. Expected: rejected before claim read, no `commit_composition`, no account existence leakage.

5. **`dos568_fixture_unauthorized_scope.rs`**
6. Actor is `SurfaceClient`; scope grant lacks `read.account_overview`.
7. Ability name is known.
8. Expected: bridge rejects before ability internals and before claim reader spy is called.
9. Expected: no Composition, no provenance, no audit success row, no composition version event.

10. **`dos568_fixture_surface_scoped_claim_reader.rs`**
11. Actor is `SurfaceClient` with only `read.account_overview`.
12. Fixture includes one visible account claim and one tighter-scope account claim.
13. Expected: service call uses surface projection with actor scopes, not a raw/non-surface loader.
14. Expected: only the visible claim reaches composition assembly.

15. **`dos568_fixture_sensitivity_excluded.rs`**
16. Account has a claim whose sensitivity requires a scope tighter than `read.account_overview`.
17. Expected: excluded entirely with no block, no `ClaimRef`, no `field_bindings`, no provenance field attribution, no count.
18. Expected: output still succeeds if other visible claims exist.

19. **`dos568_fixture_empty_account_claim_set.rs`**
20. Account exists and is visible; claim reader returns no active surfaced claims.
21. Expected: successful `AbilityOutput<Composition>` with explicit empty-state section/block.
22. Expected: empty-state block uses DisplayOnly binding and no feedback target.
23. Expected: no raw internal diagnostic copy in user-visible attributes.

24. **`dos568_fixture_trust_band_absent.rs`**
25. Account has at least one visible claim with `trust_score: None`.
26. Expected: claim appears only with degraded or unscored-safe treatment.
27. Expected: block trust is not stronger than the missing claim's trust.
28. Expected: renderer-facing attributes do not imply `likely_current`.
29. Expected: provenance field attribution records the absent/unknown trust condition.

30. **`dos568_fixture_unknown_or_stale_source_asof.rs`**
31. Fixture includes one visible claim with unknown `source_asof` and one stale claim by freshness class.
32. Expected: provenance warning for unknown `source_asof`.
33. Expected: both claims cap to caution/needs-verification according to freshness class.
34. Expected: block trust is the most cautious contributing band.

35. **`dos568_fixture_field_bindings_required.rs`**
36. Fixture renders risk, win, value, and commitment claims.
37. Expected: Source and FeedbackTarget refs use `ClaimRef::with_field`.
38. Expected: `claim_ref.field_path = None` is rejected for Source and FeedbackTarget roles.
39. Expected: summaries use ComputedFrom and empty/decorative fields use DisplayOnly.

40. **`dos568_fixture_surface_bridge_receives_composition.rs`**
41. SurfaceClient actor has valid instance id and `read.account_overview`.
42. Bridge invokes `dailyos/account-overview` through `bridges/surface_client.rs`.
43. Expected: response is committed `AbilityOutput<Composition>`.
44. Expected: Composition has server-assigned version after W4-B finalization.
45. Expected: claim refs, field bindings, provenance refs, and audit success row are present.

46. **`dos568_fixture_builder_determinism.rs`**
47. Same seeded claims, clock, input, composition id, block id strategy, and invocation id.
48. Does not call `commit_composition`.
49. Expected: normalized proposal output and provenance match exactly.
50. Expected: no customer-specific fixture strings.

51. **`dos568_fixture_committed_output_determinism.rs`**
52. Run committed-output comparison against cloned or rolled-back fixture DB state.
53. Both runs pass identical `expected_composition_version` and injected ids.
54. Expected: committed outputs match, including assigned `composition_version` from identical starting state.
55. Expected: test does not assert byte stability after two accepted commits to the same live DB.

56. **`dos568_fixture_invalidation_recomposes.rs`**
57. Seed cached projection from visible claims.
58. Emit included claim version, source freshness, dismissal, and tombstone events through DOS-589-style fixtures.
59. Expected: cache marks stale, W4-A0 re-runs, `commit_composition` publishes a new composition version.

## CI invariants

1. **Ability registry declaration check**
2. Walk the live registry/inventory.
3. Assert `dailyos/account-overview` exists.
4. Assert category is `Read`.
5. Assert `allowed_actors` projects to `User` and `SurfaceClient`.
6. Assert `required_scopes` exactly `["read.account_overview"]`.
7. Assert `mcp_exposure` exactly `Invocable`.
8. Assert `client_side_executable` exactly `false`.
9. Assert `mutates` is empty.
10. Assert `composes` is empty for v1.

11. **Scope-gate test**
12. Build SurfaceClient actor with only an unrelated read scope.
13. Invoke through `src-tauri/src/bridges/surface_client.rs`.
14. Assert ability internals are not called.
15. Assert claim-reader seam is not called.
16. Assert no ability metadata leaks beyond existing bridge policy.
17. Assert same rejection class as unknown/unauthorized where bridge policy requires byte equality.

18. **Surface claim-loader grep gate**
19. Scan `account_overview.rs` and its tests for non-surface claim loader calls in the SurfaceClient path.
20. Allow only the service/context reader seam that projects with `Actor::SurfaceClient { scopes }`.
21. Fail on raw DB reads or `load_claims_active` variants that bypass surface projection.

22. **Field-binding lint**
23. Build representative output containing risk, win, value, and commitment blocks.
24. Assert every block has non-empty `field_bindings` except explicit DisplayOnly-only empty state.
25. Assert Source and FeedbackTarget refs have `field_path: Some(_)`.
26. Assert ComputedFrom summaries list every contributing field-aware ref.
27. Assert DisplayOnly fields cannot be routed as feedback targets.

28. **Fixture-determinism tests**
29. Pure builder fixture freezes clock and ids, skips commit, and compares normalized proposal output.
30. Committed-output fixture runs against cloned or rolled-back DB state.
31. Both committed runs pass identical `expected_composition_version`.
32. Fail on non-deterministic block ordering, section ordering, trust-band drift, or unstable invocation ids.
33. Do not compare two accepted commits to the same live DB as byte-identical.

34. **SurfaceClientBridge contract test**
35. Invoke as SurfaceClient with `read.account_overview`.
36. Assert bridge response envelope contains ability name, version, schema version, domain data, and provenance.
37. Assert domain data deserializes to `Composition`.
38. Assert `composition_id`, committed `composition_version`, `claim_refs`, `field_bindings`, and `ProvenanceRef` are present.
39. Assert W4-B stale/overflow errors are preserved when forced through a test double.

40. **Audit event test**
41. Successful Invocable invocation emits `emit_surface_audit` with `event_kind = ability_invoked`.
42. Assert detail carries actor, account_id, composition_id, composition_version, and claim_ref_count.
43. Assert unauthorized scope does not emit the success event.

44. **Sensitivity exclusion test**
45. Seed a tighter-than-`read.account_overview` claim.
46. Assert no block, no `ClaimRef`, no count, and no provenance field attribution references it.
47. Assert visible sibling claims still render.

48. **Composition authorship lint**
49. Existing `scripts/check_composition_authorship.sh` remains green.
50. W4-A0 construction is allowed only because it lives in `abilities-runtime`.
51. No surface, command handler, or WP code constructs `Composition`.

52. **Invalidation contract test**
53. Simulate included claim version, account-subject claim, lifecycle, dismissal, freshness, and revocation events.
54. Assert cached projection is marked stale.
55. Assert recomposition calls W4-A0 and publishes a new version through `commit_composition`.
56. Assert DOS-589 delivery is mocked or faked, not reimplemented in W4-A0.

57. **No PII fixture lint**
58. Use generic names such as `Fixture Account`, `subsidiary.com`, and `user@example.com`.
59. No real customer domains, names, emails, or account details.

60. **Standard test gate**
61. `cargo clippy -- -D warnings`
62. `cargo test`
63. `pnpm tsc --noEmit`

## Interlocks

| Producer / consumer | What W4-A0 needs | What W4-A0 provides |
|---|---|---|
| **W4-B (DOS-567)** | `commit_composition`, `CompositionProposal`, `CommittedComposition`, `StaleComposition`, `version_events`, `ClaimRef::with_field`, `field_bindings`, `bridges/surface_client.rs`, `validate_session_bound_wp_user_id` | First real producer proving committed account Composition output with field-aware bindings |
| **W4-A renderer (DOS-572)** | Ability invocation path and stable Composition shape; W4-D `project_composition_for_surface(composition, ctx)` for renderer projection | Real `dailyos/account-overview` Composition for Gutenberg renderer |
| **W4-C tamper detection** | Nothing for ability authoring | Claim/composition ids and versions for signing cached projections |
| **W4-D fallback projection** | No dependency for W4-A0 authoring; renderer consumes W4-D's composition-level `project_composition_for_surface` | Known block payloads and preserved refs so fallback can degrade unknowns later |
| **W4-E presence nonce** | No direct dependency | Stable claim refs/field refs for later feedback nonce routing |
| **DOS-589 signal dispatcher** | W4-B outbox row schema and replay; dispatcher emits invalidation triggers named in §10 | Correct dependency graph so cache-bust re-runs W4-A0 and commits a new version |
| **W5-A feedback router** | Nothing at W4-A0 time | `(claim_id, claim_version, field_path)` triples and BindingRole semantics needed to route corrections/dismissals |
| **Claims substrate** | Active surfaced account claims, provenance JSON, trust score, lifecycle/sensitivity state | No new claim shape; only read consumption through surface projection |
| **ADR-0105 trust/provenance** | Field attribution, source attribution, freshness classes, trust bands | Composition envelope and refs that preserve lives-once provenance |
| **ADR-0130 composition** | Typed Composition substrate | First account overview producer using the contract |

## Open questions

1. **Exact ability external name shape.**
   Spec and task say `dailyos/account-overview`; existing ability examples use names like `get_entity_context`. Confirm macro/registry permits slash names or define canonical mapping.

2. **`ClaimRef::with_field` availability.**
   W4-B packet says W4-A0 needs it, but current W4-A0 branch lacks it. W4-A0 implementation starts only after W4-B merge or rebases onto the W4-B API branch.

3. **Empty account semantics.**
   Confirm whether an account with no visible claims should show a generic empty Composition or a more specific "not enough intelligence yet" block. Product copy must follow ADR-0083 vocabulary.

4. **Trust-band absent default.**
   V2 pins no `likely_current`; implementation still needs the exact fallback split between `use_with_caution` and `needs_verification` by freshness class.

### Closed in V2

- **Concrete SurfaceClientBridge path:** closed by W4-B V8 §37; use `src-tauri/src/bridges/surface_client.rs`.
- **Read category plus `commit_composition`:** closed; W4-A0 remains `Read`, and `commit_composition` is substrate finalization.
- **`client_side_executable` value:** closed; set `false` because Gutenberg invokes server-side through PHP runtime SurfaceClient.
- **Composition persistence ownership:** closed; W4-A0 calls W4-B finalizer directly and returns committed output.
- **`get_entity_context` composition:** closed for v1; direct claim-reader is required until SurfaceClient TODOs close.

## Linear dependency edges

- **DOS-567 (W4-B) blocks DOS-568 (W4-A0).**
- Reason: W4-A0 needs `commit_composition`, composition version assignment, outbox schema, `ClaimRef::with_field`, and `field_bindings`.
- This edge is already named in W4-B packet Linear dependency edges.
- **DOS-568 (W4-A0) blocks DOS-572 (W4-A renderer).**
- Reason: the W4-A renderer needs a real Composition-producing ability to invoke.
- **DOS-568 (W4-A0) is parallel with DOS-569 (W4-C), DOS-570 (W4-D), and DOS-571 (W4-E) after W4-B merges.**
- Reason: W4 wave plan lines 383-395 define W4-A0/W4-C/W4-D/W4-E as stage-2 parallel work.
- **DOS-568 indirectly feeds DOS-573 (W5-A feedback router).**
- Reason: feedback routing needs claim refs and field refs preserved by the first Composition producer and renderer.
- **DOS-589 depends on W4-B for substrate rows and interlocks with W4-A0 for invalidation semantics.**
- Reason: dispatcher consumes `version_events`; W4-A0 declares which account-overview dependencies make a cached projection stale and re-commits through W4-B finalization.

## L0 reviewer panel runners

- `/plan-eng-review` — required.
- `/plan-devex-review` — required because W4-A0 exposes an ability contract to SurfaceClient/WP/MCP consumers.
- `/plan-design-review` — required because Composition shape controls the account-overview reading order and trust-band surface.
- `/codex challenge` — required.
- `/codex consult` — required.
- `/cso` — required if the implementation touches trust boundary, scope filtering, provenance masking, SurfaceClient invocation, or commit_composition error handling.
- Security reviewer focus:
- Scope leakage on unauthorized SurfaceClient invocation.
- Ability metadata leakage through MCP exposure.
- Raw source/provenance leakage in block attributes.
- Trust-band absent default.
- Read ability classification with composition finalization.
- Design reviewer focus:
- Composition is magazine/editorial, not dashboard-shaped.
- Empty-state copy is product vocabulary, not pipeline vocabulary.
- Trust affordances are preserved as first-class meaning.
- Devex reviewer focus:
- Registry declaration discoverability.
- Fixture ergonomics.
- Bridge contract test path.
- Error precedence inherited cleanly from W4-B.

## Acceptance for L0 closure

1. All reviewer panel runners complete with approve or explicitly folded conditional findings.
2. W4-B dependency edge remains explicit and blocks implementation start until W4-B API is available.
3. Concrete `SurfaceClientBridge` path is closed to `src-tauri/src/bridges/surface_client.rs`.
4. Read category plus `commit_composition` is closed: W4-A0 is Read; finalization is substrate-owned.
5. Ability policy fields are accepted: `allowed_actors`, `required_scopes`, `mcp_exposure`, and `client_side_executable = false`.
6. Fixture plan is accepted with generic data, deterministic clock/id strategy, and split builder/committed determinism.
7. Negative fixtures are accepted as sufficient for first producer risk.
8. CI invariants are accepted as merge-gating, not optional review notes.
9. Field-binding and field-path enforcement is accepted as a hard contract.
10. Surface-scoped claim loading and sensitivity exclusion are accepted as hard contracts.
11. Audit emission for successful Invocable calls is accepted as a hard contract.
12. DOS-589 invalidation triggers are accepted as the cache-bust contract.
13. W4-A renderer interlock is recorded: renderer waits for W4-A0 producer plus W4-C/W4-D/W3-B per wave plan.
14. W4-D interlock is recorded: renderer consumes `project_composition_for_surface`, not W4-A0.
15. Packet is posted to Linear DOS-568 as the L0 plan packet or linked from the issue.
16. No implementation commit is made from this L0 Prep branch.
17. This file exists at `.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md`.
