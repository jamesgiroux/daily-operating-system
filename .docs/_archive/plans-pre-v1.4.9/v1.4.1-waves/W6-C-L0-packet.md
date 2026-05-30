# DOS-290 W6-C L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 6 - Validation suite.
- **Agent:** W6-C.
- **Linear issue:** DOS-290 - "Validation - cross-surface consistency for daily, meeting, and entity detail views" (DOS-290 content supplied verbatim in the authoring prompt for this packet).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. The only file created by this turn is `.docs/plans/v1.4.1-waves/W6-C-L0-packet.md`.
- **W6-C assignment:** the wave plan names W6-C as "DOS-290 cross-surface consistency validation" and assigns it "validation bundle + assertions for daily/meeting/entity surfaces agreeing on shared claims." Source: `.docs/plans/v1.4.1-waves.md:632-635`.
- **W6 merge gate:** W6 requires L0 plan approvals, L2 approvals, L3 Suite E final with bundles 1-13 and 14-18 mandatory green, L4 `/qa`, L5 drift check, retro, and proof bundle. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Reviewer contract:** W6 L0 requires `qa-expert` for all six W6 agents, with `security-auditor` only for DOS-292. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Validation-suite numbering contract:** the W6 gate names bundles 14, 15, 16, 17, and 18 as mandatory additions, and W6-C is the second validation agent after W6-B while W6-E is explicitly bundle 17; W6-C therefore owns bundle 15. Sources: `.docs/plans/v1.4.1-waves.md:632-646`, `.docs/plans/v1.4.1-waves.md:659-663`.
- **Fixture catalogue:** the committed fixture corpus is documented at `src-tauri/tests/fixtures/bundle-README.md`; it currently describes `bundle-1` through `bundle-13`, requires hyphenated `bundle-N` directories, and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
- **Bundle-15 presence check:** `rg -n "bundle-15|\"bundle\"\\s*:\\s*15|bundle15" src-tauri/tests/fixtures src-tauri/tests` returned no matches in this authoring pass; the current loader test also expects only bundles 1 through 13. Sources: `src-tauri/tests/harness.rs:42-65`, `src-tauri/tests/fixtures/bundle-README.md:33-45`.
- **Runtime contract:** synthesized user-facing and agent-facing context must go through abilities, surfaces invoke abilities through the registry/typed imports, every ability output carries provenance once, and MCP exposure is governed by ability policy. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:29-37`, `.docs/decisions/0102-abilities-as-runtime-contract.md:269-290`, `.docs/decisions/0102-abilities-as-runtime-contract.md:341-359`.

## 2. Load-Bearing User Outcome

DOS-290 frames the user failure this bundle must prevent:

> "DailyOS can be internally inconsistent even when every individual ability returns a valid response. The user experiences this as different pages telling different stories about the same day, meeting, account, project, person, or open loop."

The user harm is also explicit:

> "User cannot tell which surface to trust. May walk into a meeting using the daily briefing while the meeting detail page or account page has fresher or different context."

The load-bearing outcome for W6-C is therefore not "each ability response is valid." It is: **DailyOS must prove that daily readiness, meeting briefing, entity detail, and MCP output cannot tell contradictory current-state stories about the same fixture without a visible timestamped or degraded state.** If divergence is possible, it must be surfaced through shared timestamp/provenance, trust band, render policy, or blocking/lint posture. Silent disagreement is the failure.

Required behavior from DOS-290:

> "Golden Loop validation compares surfaces not just abilities; each rendered surface exposes source timestamp/generated_at where state could diverge; disagreements must be either impossible by shared read model or visible as timestamped/degraded state; lint/blocking state influences render policy across surfaces."

This user outcome depends on the existing Intelligence Loop substrate, not a display-only comparator:

- **Claim model:** W6-C tests whether the same account, meeting, project, person, and open-loop facts share claim-backed identity and subject routing across daily, meeting, entity, and MCP surfaces instead of drifting through per-page projections. `get_entity_context` is the claim-backed Read ability, `prepare_meeting` composes it, and `get_daily_readiness` composes both `prepare_meeting` and `get_entity_context`. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`.
- **Provenance and trust:** ability outputs carry one provenance envelope with temporal context, source attribution, field attribution, warnings, and trust classification; `source_asof` is first-class, and `produced_at`/`generated_at` are the timestamps surfaces use when output state may diverge. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:19-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:160-169`, `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`.
- **Signals and invalidation:** DOS-290 does not add a new invalidation mechanism, but its fixture must prove stale or blocked source state is visible across every consumer path that renders the same claim. The release gate already treats mandatory bundle and invariant failures as blocking. Sources: `src-tauri/src/release_gate.rs:715-794`, `src-tauri/src/release_gate.rs:1268-1292`.
- **Runtime and surfaces:** required consumers are `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, the dashboard render data path, and the MCP bridge. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/src/services/dashboard.rs:603-652`, `src-tauri/src/bridges/mcp.rs:385-426`.
- **Feedback loop:** lint-blocked, dismissed, stale, or degraded claims cannot render confidently on one surface while another surface blocks or qualifies them. MCP and Tauri surfaces already carry separate bridge surfaces and dismissal/render-policy handling, so W6-C should assert parity through those bridges rather than bypass them. Sources: `src-tauri/src/bridges/types.rs:533-534`, `src-tauri/src/bridges/types.rs:803-919`, `src-tauri/src/bridges/mcp.rs:1430-1462`.

The W6-C proof must cover these concrete DOS-290 edge cases as fixture rows or explicit assertions:

- Daily page shows 5 meetings while readiness says 4/4.
- Meeting detail links Account A while the daily briefing summary describes Account B.
- Account page says healthy while meeting briefing says at risk.
- Project page status differs from the project section on the account page.
- Person page shows no actions or meetings while meeting prep uses that person as context.
- Lint flags stale or bleed issue, but the same claim still renders confidently elsewhere.
- Activity log says refresh completed while visible surface still shows old data.
- MCP ability response differs from Tauri app for the same entity or meeting.

## 3. Pre-Work

- **Read W6 source of truth.** W6 is the validation suite, six agents fan out, and W6-C owns cross-surface consistency validation. Source: `.docs/plans/v1.4.1-waves.md:617-635`.
- **Read W6 merge gate.** W6 cannot land partially: L3 Suite E final requires bundles 1-13 plus bundles 14-18 mandatory pass, all 18 green, no partial-pass cut. Source: `.docs/plans/v1.4.1-waves.md:653-663`.
- **Acknowledged Amendment 1.** Amendment 1 recategorizes W3 stage-3b as `instrumentation-complete, data-sufficiency-pending`, relaxes W6's hard precondition to stage-3b instrumentation-complete, and says W6 starts against the partial baseline. This does not reduce W6-C scope; it only narrows what "stage-3b closure" means for unblocking W6. Sources: `.docs/plans/v1.4.1-waves-amendments.md:15-23`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`, `.docs/plans/v1.4.1-waves-amendments.md:71-75`.
- **Mapped bundle number.** The fixture catalogue documents committed `bundle-1` through `bundle-13`, the harness currently expects bundles 1 through 13, and the `bundle-15` grep returned no existing fixture or test. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/tests/harness.rs:42-65`.
- **Assigned W6-C to bundle 15.** W6-C is the second new validation bundle in the DOS-289 through DOS-293 sequence, and W6-E is explicitly bundle 17. Bundle 15 is therefore the W6-C slot. Sources: `.docs/plans/v1.4.1-waves.md:627-646`, `.docs/plans/v1.4.1-waves.md:659-663`.
- **Found the fixture catalogue.** The catalogue defines `metadata.json` manifest fields including `bundle`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, source lifecycle refs, anonymization cert, trust factors, and pass/fail definition. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
- **Read sibling bundle shape.** `bundle2_provider_hallucination_substrate_test.rs` loads a numbered fixture through `bundle_fixture_path`, asserts fixture metadata, runs `run_with_synthetic_enrich_stub`, compares expected and actual post-action state, and asserts warning/trust fields. Source: `src-tauri/tests/bundle2_provider_hallucination_substrate_test.rs:1-60`.
- **Read loader shape.** Fixtures require `inputs.json`, `expected_output.json`, `expected_provenance.json`, and `metadata.json`; `expected_state.json` is optional; the loader discovers only `bundle-` directories with `metadata.json`. Sources: `src-tauri/src/harness/loader.rs:15-23`, `src-tauri/src/harness/loader.rs:150-180`, `src-tauri/src/harness/loader.rs:186-238`.
- **Read ADRs.** Binding ADRs for this packet are ADR-0102 abilities runtime and ADR-0105 provenance plus `source_asof`. Abilities are named, typed, versioned product capabilities invoked through runtime contracts; provenance is first-class output and every field must be attributable. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:29-37`, `.docs/decisions/0102-abilities-as-runtime-contract.md:143-179`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:199-237`.
- **Read daily readiness surface.** `get_daily_readiness` is registered as a Read ability and composes non-optional `prepare_meeting` and `get_entity_context` children. The synthesis path builds child provenance for both. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:506-552`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:610-625`.
- **Read meeting briefing surface.** `prepare_meeting` is registered as a Transform ability and composes `get_entity_context`; its synthesis path reads a public meeting context snapshot through the service reader. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:468-471`.
- **Read entity detail surface.** `get_entity_context` is registered as a Read ability; Tauri operation dispatch also exposes `read_get_entity_context_executor`. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/src/operations/mod.rs:135-139`, `src-tauri/src/operations/mod.rs:332-339`.
- **Read MCP bridge.** MCP ability calls route through `McpAbilityBridge::invoke_ability`, render `BridgeSurface::McpTool` data and `McpToolDetail` provenance, and `src-tauri/src/mcp/main.rs` routes ability tool calls into that bridge. Sources: `src-tauri/src/bridges/mcp.rs:385-426`, `src-tauri/src/mcp/main.rs:840-851`, `src-tauri/src/mcp/main.rs:1009-1027`.
- **Read dashboard render layer.** `services/dashboard.rs` owns dashboard data loading, filters active meetings, merges DB meetings with live calendar events, calculates meeting stats, and exposes manifest `generated_at` or fallback freshness timestamps. Sources: `src-tauri/src/services/dashboard.rs:603-652`, `src-tauri/src/services/dashboard.rs:667-809`, `src-tauri/src/services/dashboard.rs:1147-1189`, `src-tauri/src/services/dashboard.rs:1209-1235`.
- **No fork-point diff in this packet authoring pass.** The authoring request explicitly prohibited commits and source edits. This L0 packet is grounded by direct file reads and command checks, and L2 remains the pre-merge diff review stage after implementation.

## 4. Architecture

### 4.1 Bundle Assignment

W6-C owns **bundle 15**.

- **New fixture directory:** `src-tauri/tests/fixtures/bundle-15/`.
- **New substrate test file:** `src-tauri/tests/bundle15_cross_surface_consistency_substrate_test.rs`.
- **Naming rationale:** existing substrate tests use the `bundleN_TOPIC_substrate_test.rs` shape, including `bundle2_provider_hallucination_substrate_test.rs`. Source: `src-tauri/tests/bundle2_provider_hallucination_substrate_test.rs:1-18`.
- **Discovery rationale:** fixture directories must be hyphenated `bundle-N`, and the loader only recognizes `bundle-` plus digits with `metadata.json`. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/src/harness/loader.rs:205-238`.
- **Release-gate rationale:** W6 requires bundles 14-18 mandatory green. Current release-gate defaults list only bundles 1/5/13 as mandatory. **The W6-C PR itself must include the edit promoting bundle 15 to mandatory** (`src-tauri/src/release_gate.rs:26-38`); coordination-only with W7 is not acceptable, because it creates a coordination-slip path where the W6-C bundle lands non-mandatory and W7 forgets to promote. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`, `src-tauri/src/release_gate.rs:715-794`.

### 4.2 Fixture Invariant

**Invariant:** Daily readiness, meeting briefing, entity detail, dashboard render data, and MCP output must agree on shared current-state claims for the same fixture subject, or must all expose a timestamped/degraded disagreement. It is a release-gate failure for one surface to render a claim confidently while another surface blocks, degrades, or contradicts that same claim without visible timestamp/provenance.

The invariant is not satisfied by comparing JSON snapshots of one ability at a time. The bundle must compare the same seeded account, meeting, project, person, and open loop across all required surfaces:

- Shared identity must flow through claim subject attribution and ability composition, not by string matching display labels. Sources: `src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs:750-751`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`.
- `source_asof`, `produced_at`, and dashboard `generated_at` must be present where state could diverge, so the user can see whether a discrepancy is stale/degraded rather than silently contradictory. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:35-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:160-169`, `src-tauri/src/services/dashboard.rs:1209-1235`.
- MCP and Tauri app surfaces must invoke the same ability contract and bridge/render policy rather than handwritten parallel logic. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:269-290`, `src-tauri/src/bridges/types.rs:803-919`, `src-tauri/src/bridges/mcp.rs:385-426`.
- Mandatory bundle failure must block the release gate, including missing, skipped, or failed bundle status. Sources: `src-tauri/src/release_gate.rs:715-794`, `src-tauri/src/release_gate.rs:1268-1292`.

### 4.3 Fixture Shape

Bundle 15 should seed one synthetic workspace where the same account appears in daily briefing, meeting detail, account detail, project detail, person detail, and MCP response. Use generic domains and people only. No real customer names, domains, email addresses, or account details belong in the fixture.

Required fixture files follow the loader contract:

- `clock.txt` fixes the test clock. Use a May 2026 timestamp so dashboard `generated_at`, ability `produced_at`, and source `source_asof` assertions are deterministic.
- `seed.txt` fixes randomization.
- `state.sql` seeds the shared account, meeting, linked project, linked person, open loop/action, source rows, claim rows, lint/degraded state rows, and any surface-specific read-model rows needed to prove cross-surface consistency.
- `inputs.json` drives `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, and an MCP bridge invocation for the same fixture subject set.
- `provider_replay.json` pins any Transform completion used by `prepare_meeting` or `get_daily_readiness` so the cross-surface assertions do not depend on live provider variance.
- `external_replay.json` pins live-calendar, source freshness, and MCP-visible external state where the same entity could otherwise drift.
- `expected_output.json` asserts the cross-surface comparison matrix, not just one ability output.
- `expected_provenance.json` asserts source attribution, `source_asof`, `produced_at`, child composition, bridge surface, trust/degraded posture, and timestamp visibility.
- `expected_state.json` asserts post-run lint/blocking/degraded state and ensures no surface can render a blocked claim confidently.
- `metadata.json` includes `bundle: 15`, a stable scenario id such as `cross-surface-consistency`, `surfaces_exercised` covering `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, `dashboard`, `mcp`, dominant trust factors including source freshness, subject ownership, render policy, and bridge parity, plus a pass/fail definition that fails on silent disagreement. Sources: `src-tauri/src/harness/loader.rs:15-23`, `src-tauri/src/harness/loader.rs:150-180`, `src-tauri/src/harness/types.rs:63-65`, `src-tauri/tests/fixtures/bundle-README.md:8-27`.

### 4.4 Seeded Scenario

The bundle should model the DOS-290 fixture requirement:

> "Seed one workspace where same account appears in daily briefing, meeting detail, account detail, project detail, person detail, and MCP response."

Minimum rows and assertions:

- **Shared account and meeting:** seed one account such as `account-b15-example`, one customer meeting, and one linked meeting entity so dashboard and meeting prep use the same eligibility set. Daily page meeting counts and readiness counts must derive from that same set. Sources: `src-tauri/src/services/dashboard.rs:667-809`, `src-tauri/src/services/dashboard.rs:1147-1189`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`.
- **Meeting briefing:** `prepare_meeting` must render the same primary account, person, project, open loop, risk/health posture, and timestamps as the entity surfaces or visibly degrade. It composes `get_entity_context`, so divergence here should be impossible unless the child output or source state is timestamped/degraded. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`.
- **Account detail/entity context:** `get_entity_context` must render the current account claim set used by meeting and daily outputs. It must fail the bundle if the account surface says healthy while meeting or daily surfaces say at risk without timestamped disagreement. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/src/operations/mod.rs:332-339`.
- **Project detail:** the seeded project status must match the project section exposed through the account/meeting context, or both surfaces must show the stale/degraded state. This can be represented through `get_entity_context` for a `project` subject and linked account/meeting claims. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`.
- **Person detail:** the seeded person must have the same action/open-loop and meeting association in person context as meeting prep uses for attendee context. It is a failure if person detail shows no actions/meetings while meeting prep uses that person as active context. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1271-1345`.
- **Daily readiness:** `get_daily_readiness` must include child references for the same meeting and entity context, and its meeting counts must agree with dashboard eligibility. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:506-552`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:610-625`, `src-tauri/src/services/dashboard.rs:1147-1189`.
- **Dashboard render data:** `services/dashboard.rs` is the dashboard-facing render layer. Bundle 15 should assert the same meeting set, linked entity chips, lifecycle/health data, and generated freshness posture as the ability outputs. Sources: `src-tauri/src/services/dashboard.rs:603-652`, `src-tauri/src/services/dashboard.rs:854-891`, `src-tauri/src/services/dashboard.rs:972-974`, `src-tauri/src/services/dashboard.rs:1209-1235`.
- **MCP response:** MCP invocation for the same entity/meeting must match the Tauri/ability output after MCP render policy is applied. It is acceptable for MCP to redact or drop private fields per policy; it is not acceptable for MCP to report a different primary entity, health/status, source timestamp, or confident stale claim. Sources: `src-tauri/src/mcp/main.rs:840-851`, `src-tauri/src/bridges/mcp.rs:385-426`, `src-tauri/src/bridges/types.rs:904-919`.

### 4.5 Cross-Surface Comparison Oracle (Normalized Field-Level Diff)

Bundle 15 is the general cross-surface consistency bundle. **The oracle is a normalized field-level diff matrix across the five surfaces, not a string-containment rule.** String containment can mask divergence in unasserted fields; the matrix must be explicit about every compared field.

| Field | get_entity_context | prepare_meeting | get_daily_readiness | dashboard render | MCP bridge response | Allowed divergence |
| --- | --- | --- | --- | --- | --- | --- |
| primary account id | `subject.entity_id` | `attendee_context[].entity_id` (account) | `surfaces.account[].entity_id` | `account_chips[].entity_id` | `entity_id` | None |
| eligible meeting count | (N/A — entity scope) | (N/A — per-meeting scope) | `readiness_checks.eligible_meeting_count` | `meeting_count` | (matches Tauri) | None |
| account health/risk | `entries[].current_state.health` | `topics[].health_posture` | `overnight_changes[].health` | `account_chips[].health` | (matches Tauri, redaction-policy may drop) | MCP redaction explicitly allowed iff redaction-policy table marks the field private |
| project status | `entries[].current_state.project_status` (for project subject) OR `linked_projects[].status` (for account subject) | `topics[].project_status` | (composed from above) | `project_chips[].status` | (matches Tauri) | None |
| current-state claim id | `entries[].claim_id` | `topics[].source_claim_ids` | `overnight_changes[].source_claim_ids` | `claim_refs[]` | `provenance.source_claim_ids` | Set equality, ordering may differ |
| `source_asof` of current-state claim | `entries[].source_asof` | `topics[].source_asof` | `overnight_changes[].source_asof` | `freshness.source_asof` | `provenance.source_asof` | Format only — Unix vs ISO |
| `produced_at` / `generated_at` | (envelope `produced_at`) | (envelope `produced_at`) | (envelope `produced_at`) | `dashboard.generated_at` | `produced_at` | Format only |
| trust band | `entries[].trust_band` | `topics[].trust_band` | `overnight_changes[].trust_band` | `account_chips[].trust_band` | `provenance.trust_band` | None |

**MCP redaction carve-outs:** for fields where the redaction-policy table at `src-tauri/src/bridges/types.rs:803-919` marks the field private for the requesting actor, MCP may return `null` or a redaction marker instead of the Tauri value — but the redaction must match the policy table, not be silently divergent. Bundle 15's expected output asserts MCP returns either the same value or the expected redaction marker per the policy table. Note: actor-specific divergence in *which* fields get redacted is W6-E scope, not W6-C; bundle 15 uses one fixed actor (the seeded Tauri user) and asserts redaction matches that one actor's policy.

**Activity-log refresh-completion case:** DOS-290 explicitly names "activity log says refresh completed, but visible surface still shows old data." Bundle 15 asserts: after a `refresh_completed` event is written to the activity log for the seeded subject, the next `get_entity_context` / `prepare_meeting` / dashboard read returns the post-refresh state. If activity log says complete and a render still shows pre-refresh data, the assertion fires. Source: `src-tauri/src/services/dashboard.rs:603-652` (dashboard read after refresh), `src-tauri/src/bridges/types.rs:533-534` (activity log surfacing).

**Project page surface pinned:** DOS-290 names a "project page" surface. Bundle 15 maps project page to `get_entity_context` invoked with subject of `project` type AND to the project chips rendered through `services/dashboard.rs:603-652`. The "project section on account page" is `get_entity_context` for the account subject's `linked_projects[]` field. The diff is between those two concrete render paths; if they disagree on project status, bundle 15 fails.

This rule is grounded in the ability composition model and bridge contract: `prepare_meeting` composes `get_entity_context`, `get_daily_readiness` composes both, MCP routes through the bridge. Sources: `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`, `src-tauri/src/bridges/mcp.rs:385-426`, `.docs/decisions/0102-abilities-as-runtime-contract.md:269-290`.

### 4.6 Trust And Lint Assertions

Bundle 15 must include two classes of assertions:

- **Trust/render assertions:** the same lint-blocked, stale, or degraded claim cannot be `likely_current` on one surface and blocked/degraded on another. If the claim remains renderable, all surfaces must carry matching trust band or timestamped disagreement. Provenance warnings and `source_asof` must be visible where needed. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:55-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:160-169`, `src-tauri/src/bridges/types.rs:803-919`.
- **Lint/release-gate assertions:** release-gate or harness lint must fail when daily, meeting, entity, dashboard, and MCP outputs disagree on primary entity, meeting count, health/risk state, project status, action/open-loop presence, or confident rendering of a blocked claim. The implementation must add bundle-15 status/invariant coverage so this participates in the W6/W7 mandatory gate. Sources: `src-tauri/src/release_gate.rs:715-794`, `src-tauri/src/release_gate.rs:1101-1111`, `src-tauri/src/release_gate.rs:1268-1292`.

### 4.7 Intelligence Loop Check

- **Claim model:** no display-only cross-surface string comparator is sufficient. Bundle 15 must assert shared subject attribution, field attribution, claim ids/source refs, temporal scope where relevant, lifecycle/render state, and primary-entity selection across daily, meeting, entity, dashboard, and MCP surfaces. Sources: `src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs:750-751`, `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`.
- **Provenance and trust:** every output under test must expose source attribution, `source_asof`, produced/generated timestamp, field attribution, trust posture, and warnings through the ability provenance wrapper or render-layer freshness contract. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:19-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:199-237`, `src-tauri/src/services/dashboard.rs:1209-1235`.
- **Signals and invalidation:** lint/blocking/degraded state must influence render policy everywhere the same claim appears; expected state should assert the blocked/degraded status and the absence of confident rendering on every surface. Sources: `src-tauri/src/bridges/types.rs:803-919`, `src-tauri/src/release_gate.rs:715-794`.
- **Runtime and surfaces:** required consumers are `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, dashboard render data, and MCP bridge response. Tauri and MCP parity follows ability registry invocation and bridge rendering, not per-surface custom logic. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:269-290`, `.docs/decisions/0102-abilities-as-runtime-contract.md:350-359`, `src-tauri/src/mcp/main.rs:840-851`.
- **Feedback loop:** user corrections, dismissals, stale-source lint, or bleed-blocking outcomes must carry forward into render policy; no surface may ignore the blocked/degraded state and render the same claim confidently. Sources: `src-tauri/src/bridges/types.rs:533-534`, `src-tauri/src/bridges/mcp.rs:1430-1462`, `src-tauri/src/release_gate.rs:1268-1292`.

## 5. Acceptance Criteria

DOS-290 Acceptance, quoted verbatim:

> "validation test compares daily readiness + meeting briefing + entity detail + MCP output for same fixture; meeting counts and readiness counts derive from same eligibility set; primary entity disagreement across surfaces fails release gate; stale/degraded source state is visible instead of silently divergent; lint-blocked claims cannot render confidently on another surface."

Testable decomposition:

1. **Cross-surface fixture comparison.** Bundle 15 invokes or captures `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, dashboard render data, and MCP output for the same seeded workspace/meeting/account/project/person/open-loop fixture. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:15-32`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:15-27`, `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:53-65`, `src-tauri/src/bridges/mcp.rs:385-426`.
2. **Daily readiness plus meeting briefing plus entity detail plus MCP output.** The assertion matrix includes at least one daily readiness output, one meeting briefing output, one account entity context output, one project or person entity context output, and one MCP response for the same fixture. Sources: `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:506-552`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`, `src-tauri/src/mcp/main.rs:840-851`.
3. **Same eligibility set for meeting counts.** Dashboard meeting count and daily readiness/readiness counts derive from the same active, non-personal, non-archived eligibility set. It is a failure if daily page shows 5 meetings while readiness says 4/4 unless the discrepancy is timestamped/degraded. Sources: `src-tauri/src/services/dashboard.rs:183-187`, `src-tauri/src/services/dashboard.rs:667-809`, `src-tauri/src/services/dashboard.rs:1147-1189`.
4. **Primary entity disagreement fails release gate.** If daily, meeting, entity, dashboard, or MCP surfaces disagree about Account A versus Account B for the same meeting/entity fixture, bundle 15 fails and the release gate treats the mandatory bundle as blocking. Sources: `src-tauri/src/release_gate.rs:715-794`, `src-tauri/src/release_gate.rs:1268-1292`.
5. **Health/risk state agreement.** If account detail says healthy while meeting briefing says at risk, both surfaces must expose timestamped disagreement or degraded state. Silent confident disagreement fails. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:35-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:160-169`, `src-tauri/src/services/dashboard.rs:187-207`.
6. **Project status agreement.** Project detail and the project section on account/meeting context must agree on status or both expose stale/degraded source state. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:238-299`.
7. **Person action/meeting agreement.** Person detail must not show no actions/meetings when meeting prep uses that person as active context; if one surface lacks state, it must show a generated/source timestamp or degraded state that explains the gap. Sources: `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:65-134`, `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1271-1345`.
8. **Stale/degraded source state is visible.** Each rendered surface exposes `source_asof`, `produced_at`, dashboard `generated_at`, or an equivalent warning/degraded state when source state could diverge. Sources: `.docs/decisions/0105-provenance-as-first-class-output.md:35-63`, `.docs/decisions/0105-provenance-as-first-class-output.md:379-437`, `src-tauri/src/services/dashboard.rs:1209-1235`.
9. **Lint-blocked claims cannot render confidently elsewhere.** If a claim is lint-blocked, stale-blocked, bleed-blocked, dismissed, or degraded in the fixture, every surface must suppress, degrade, or warn. No other surface may render it as confident/likely-current. Sources: `src-tauri/src/bridges/types.rs:803-919`, `src-tauri/src/bridges/mcp.rs:1430-1462`, `src-tauri/src/release_gate.rs:715-794`.
10. **MCP/Tauri parity after render policy.** MCP ability response may apply MCP redaction policy, but it must not disagree with Tauri on primary entity, meeting count, risk/health, project status, person action/meeting presence, timestamps, or confident blocked-claim posture. Sources: `src-tauri/src/bridges/mcp.rs:385-426`, `src-tauri/src/bridges/types.rs:904-919`, `src-tauri/tests/dos412_mcp_ability_data_redaction_test.rs:181-200`.
11. **Bundle 15 is mandatory by W6/W7 release gate.** The implementation cannot leave bundle 15 as tracked/non-blocking if W6 is claiming L3/L5 readiness. Sources: `.docs/plans/v1.4.1-waves.md:653-663`, `src-tauri/src/release_gate.rs:26-38`.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-290 content is supplied verbatim in the authoring prompt for this packet. No Linear connector lookup was required to draft V1.
- **Upstream unblock:** W6 starts after the W3 stage-3b precondition, as amended to instrumentation-complete rather than full data-sufficiency closure. Sources: `.docs/plans/v1.4.1-waves.md:653-655`, `.docs/plans/v1.4.1-waves-amendments.md:37-47`.
- **Adjacent W6 coordination:** W6-B owns bundle 14, W6-C owns bundle 15, W6-D owns bundle 16, W6-E owns bundle 17, and W6-F owns bundle 18 by wave-plan sequencing; DOS-292 explicitly owns bundle 17. Sources: `.docs/plans/v1.4.1-waves.md:627-651`, `.docs/plans/v1.4.1-waves.md:659-663`.
- **Release-gate coordination:** W6-C must produce bundle metadata and invariant names early enough for W7/release-gate wiring because current release-gate defaults still only mark bundles 1, 5, and 13 mandatory. Sources: `src-tauri/src/release_gate.rs:26-38`, `.docs/plans/v1.4.1-waves.md:653-663`.
- **Not a DOS-289 takeover:** Stale-current contradiction is tested only when it produces cross-surface divergence because DOS-290 Acceptance requires stale/degraded source state to be visible instead of silently divergent. General stale-current supersession remains W6-B. Source: `.docs/plans/v1.4.1-waves.md:627-630`.
- **Not a DOS-291 takeover:** Ambiguous identity and primary-context selection remains W6-D. W6-C should assert that a primary entity disagreement fails, but it does not own the ambiguous-identity fixture class. Source: `.docs/plans/v1.4.1-waves.md:637-640`.
- **Not a DOS-292 takeover:** Source lifecycle/privacy/actor-specific provenance remains W6-E, and W6-E is the security-auditor lane. W6-C checks stale/degraded render consistency, not the full privacy/source lifecycle matrix. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.
- **Not a DOS-293 takeover:** Sync, refresh, concurrency, and partial-failure recovery remains W6-F. W6-C may assert visible stale/generated timestamps but does not own refresh idempotency or partial-failure recovery. Source: `.docs/plans/v1.4.1-waves.md:648-651`.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** W6 merge gate requires L0 plan approvals with `qa-expert` for all six W6 agents. Source: `.docs/plans/v1.4.1-waves.md:655-659`.
- **Security reviewer:** not required for W6-C. The wave gate names `security-auditor` only for DOS-292, and DOS-292 is W6-E. Sources: `.docs/plans/v1.4.1-waves.md:642-646`, `.docs/plans/v1.4.1-waves.md:655-659`.
- **Review focus for `qa-expert`:**
  - Bundle 15 assignment and naming are unambiguous.
  - Fixture catalogue/harness shape is followed.
  - DOS-290 edge cases are represented by fixture state, replay, expected output, expected provenance, and expected state.
  - `get_daily_readiness`, `prepare_meeting`, `get_entity_context`, dashboard render data, and MCP output are all asserted.
  - Meeting counts and readiness counts derive from the same eligibility set.
  - Primary entity disagreement fails the bundle and release gate.
  - Stale/degraded source state and lint-blocked claims influence render policy across all surfaces.
  - Bundle 15 can become mandatory in the W6/W7 release gate with no quarantine.

## 8. L0 Acceptance Gate

L0 passes only if the reviewer accepts all of the following:

1. **Problem fit:** the plan tests cross-surface consistency, not one-ability validity.
2. **Bundle lock:** W6-C is locked to bundle 15 and implementation path `src-tauri/tests/bundle15_cross_surface_consistency_substrate_test.rs`.
3. **Fixture lock:** bundle directory is `src-tauri/tests/fixtures/bundle-15/`, using the loader-required files and `metadata.json` fields. Sources: `src-tauri/src/harness/loader.rs:15-23`, `src-tauri/tests/fixtures/bundle-README.md:8-27`.
4. **Amendment acknowledgement:** Amendment 1 is acknowledged, and the packet does not treat stage-3b residual work as a W6-C blocker beyond the amended instrumentation-complete baseline. Source: `.docs/plans/v1.4.1-waves-amendments.md:37-75`.
5. **Acceptance coverage:** every clause of DOS-290 Acceptance is decomposed into a testable bundle assertion in Section 5.
6. **Runtime parity:** required assertions exercise ability outputs, dashboard render data, bridge rendering, and provenance, not frontend-only display fixtures. Sources: `.docs/decisions/0102-abilities-as-runtime-contract.md:269-290`, `.docs/decisions/0105-provenance-as-first-class-output.md:19-63`.
7. **Implementation surfaces cited:** every implementation surface named in the packet has a file:line citation: daily readiness, meeting briefing, entity detail, MCP bridge, and `services/dashboard.rs`.
8. **Reviewer panel:** `qa-expert` is the only required L0 reviewer; no `security-auditor` is listed for W6-C.
9. **No PII:** all fixture examples are synthetic and generic.

## 9. Out-Of-Scope

- Writing implementation files in this packet authoring turn.
- Committing changes.
- Creating schema migrations unless implementation proves no existing claim/lifecycle/trust/provenance/render field can encode the required state. If a schema/table/column is proposed later, the full Intelligence Loop check is mandatory before implementation.
- Building a new user-facing cross-surface disagreement UI.
- DOS-289 stale-current contradiction beyond cross-surface visibility and render-policy consistency.
- DOS-291 ambiguous identity and primary-context selection.
- DOS-292 source lifecycle, privacy, actor-specific provenance, and the security-auditor lane.
- DOS-293 sync, refresh, concurrency, and partial-failure recovery.
- Treating W6-C as a dashboard rewrite. The dashboard path is included only as a render surface that must agree with ability/MCP outputs or show timestamped/degraded state.
- Adding customer-specific names, domains, emails, or account details to fixtures.

## 10. Changelog

- **V1 - 2026-05-15:** Initial W6-C L0 packet. Assigned DOS-290 to bundle 15, cited the W6 plan and Amendment 1, grounded fixture architecture in the bundle catalogue/harness, mapped daily readiness, meeting briefing, entity detail, dashboard, and MCP implementation anchors, quoted DOS-290 problem/user-harm/required-behavior/acceptance, decomposed acceptance criteria, and listed `qa-expert` as the only required L0 reviewer.
