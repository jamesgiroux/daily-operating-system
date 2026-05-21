# L0 Packet — v1.4.4 W1 Substrate Gaps

**Current revision: V1.1 (cycle-1 fold, 2026-05-20). V1.0 was BLOCKED on architecture; V1.1 lifts the blockers. See §2 Changelog.**

## 1. Header

Date: 2026-05-20
Project: v1.4.4 — WordPress Surface Migration ([Linear](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177))
Wave: W1 — Substrate gaps (substrate-only wave; lands the contracts W2–W5 surfaces consume).
Sub-tickets (10):
- [DOS-459](https://linear.app/a8c/issue/DOS-459) — `get_entity_intelligence` envelope (from dissolved v1.4.10 Entity Intelligence)
- [DOS-460](https://linear.app/a8c/issue/DOS-460) — Canonical entity touchpoints + open-loops contract
- [DOS-461](https://linear.app/a8c/issue/DOS-461) — Entity fixture harness + no-bypass checks
- [DOS-477](https://linear.app/a8c/issue/DOS-477) — Entity-detail trust-boundary hardening
- [DOS-335](https://linear.app/a8c/issue/DOS-335) — Meeting prep/readiness DTO
- [DOS-339](https://linear.app/a8c/issue/DOS-339) — Shared claim receipt contract (from dissolved v1.4.4 Claim Experience; **W1-RECEIPT carve-out shipped under [DOS-701](https://linear.app/a8c/issue/DOS-701), PR #323** — this packet covers wire-up + downstream filler files, not the DTO redesign)
- [DOS-8](https://linear.app/a8c/issue/DOS-8) — Semantic claim feedback actions (typed `claim_feedback` substrate)
- [DOS-340](https://linear.app/a8c/issue/DOS-340) — Provenance receipt vs operational audit boundary
- [DOS-341](https://linear.app/a8c/issue/DOS-341) — Claim receipt privacy + redaction rules
- [DOS-507](https://linear.app/a8c/issue/DOS-507) — `get_daily_briefing` Read/User-only ability contract + proof gate

Reference (Done, prior shipped substrate W1 consumes/extends):
- [DOS-701](https://linear.app/a8c/issue/DOS-701) — `services::claim_receipt` + `services::claim_review_queue` substrate carve-out (PR #323, merged 2026-05-19). Shipped: `mod.rs`, `contracts.rs`, `render.rs`, `auth.rs` (319 LOC), TS mirror, migration `240_claim_review_deferrals.sql`. Empty zero-byte placeholders: `feedback.rs`, `boundary.rs`, `privacy.rs`, `contradiction.rs`, `render_rules.rs`. **W1 in THIS packet fills those placeholders + adds the 9 remaining substrate pieces around them.**

Surfaces that consume W1 (cite the wave L0 packet for full surface mapping when authored):
- **W2 — Entity surfaces:** Account Detail (DOS-462), Project Detail (DOS-483), Person Detail (DOS-484), Meeting Detail, entity list shells. Consume DOS-459 envelope + DOS-460 touchpoints/open-loops + DOS-339 receipts + DOS-341 privacy + DOS-477 trust-boundary gates.
- **W3 — Briefing surfaces:** Daily Briefing block, Meeting Briefing block, FolioBar + FloatingNavIsland (already substrate-rendered by W3 chrome lane). Consume DOS-507 `get_daily_briefing` envelope + DOS-335 meeting prep/readiness DTO + DOS-339 receipts.
- **W4 — Action surfaces:** Actions/Work block (DOS-514), Activity Log block (DOS-444), Action Detail, Lint Mode (DOS-445), review queue (DOS-443). Consume DOS-339 receipts + DOS-8 semantic feedback + DOS-340 audit boundary + DOS-341 privacy.
- **W5 — System/history surfaces:** History block, optionally email/settings. Consume DOS-339 + DOS-340/341 redaction matrix.

Primary code areas touched:
- `src-tauri/src/services/claim_receipt/{feedback,boundary,privacy,contradiction,render_rules}.rs` (fill the 5 zero-byte placeholders left by DOS-701)
- `src-tauri/src/services/entity_intelligence/` (new module — envelope service producer)
- `src-tauri/src/services/meeting_prep_status.rs` (new — DOS-335 service-owned DTO)
- `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/` (new Read ability — DOS-459)
- `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/` (new Read/User-only ability — DOS-507)
- `src-tauri/src/migrations/v250_*…v269_*` (W1 migration slot block; see §9)
- TypeScript mirrors at `src/services/entity-intelligence/contracts.ts`, `src/services/meeting-prep-status/contracts.ts`, `src/abilities/get-daily-briefing/contracts.ts`
- Renderer integration at `wp/dailyos/blocks/` is W2–W5 work, NOT W1. W1 produces; W2+ projects via ADR-0130 producer→projection→renderer contract.

This packet is the sub-L0 for the substrate-only W1 wave. **Wiring IS the work** (per CLAUDE.md "Definition of Done"): each producer ships with at least one downstream consumer skeleton in W2 to prove the contract carries weight. No empty composers, no "Phase 2" deferrals.

## 2. Changelog

- **V1.0 (2026-05-20, initial L0 draft):** First L0 cycle. 10 sub-tickets covered. Pre-dispatch to 5-reviewer panel (codex challenge + codex consult + ce-architecture-strategist + `/cso` mandatory + ce-correctness-reviewer). Substrate ground truth verified: claim_receipt substrate from DOS-701 already shipped (contracts.rs / render.rs / auth.rs at 319+290+175 LOC); 5 placeholder files remain zero-byte; `get_entity_context` ability exists; `prepare_meeting` ability exists; `list_open_loops` ability + `services::context::ListOpenLoopsQuery` exist; `services::claims::record_claim_feedback` + `services::claims::reconcile_contradiction` confirmed at `services/claims.rs:6700` + `:8906`; no existing `get_entity_intelligence` envelope OR `get_daily_briefing` ability; no existing `meeting_prep_status` service. Slot block v250–v269 claimed (v240 taken by DOS-701 W1-RECEIPT migration).

- **V1.1 (2026-05-20, cycle-1 fold — lifts V1.0 BLOCKED verdict):** Folds 23 findings from the 5-panel cycle-1 review. Verdict artifacts at `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-{architecture,correctness,cso,codex-challenge,codex-consult}-cycle1.md`. James's locked decisions 2026-05-20 encoded inline.

  **Folds by reviewer + finding:**
  - **Architecture (BLOCKED → APPROVED on V1.1):**
    - F1 CRITICAL (migration slot collision v250–v269 vs v1.4.6 v260–v279) → §9 reclaimed to **v240–v249**; coordination table on v1.4.6-waves.md §327 + §414 honored verbatim (it already states "v1.4.4 holds v240–v249"). DOS-701 took v240; W1 needs ~5 more slots (DOS-335 takes 2). Cross-reference added.
    - F2 CRITICAL (Read-ability call-graph violation in §5.5) → §5.5 split into `services::meeting_prep_status::read` (pure read; no `&mut`; no signal emit) + `services::meeting_prep_status::write` (`enqueue_refresh` + `record_user_authored`); AC-335.12 added (trybuild test or call-graph lint asserts `compute_status` graph contains zero mutations).
    - F3 HIGH (DOS-335 signals gap) → new `MeetingPrepStatusChanged` signal pre-declared in `signals/policy_registry.rs` at W1 kickoff, mirroring v1.4.6 W0 cycle-4 fix C pattern; §5.5 Intelligence Loop check item 3 rewritten.
    - F4 HIGH (receipt fan-out coalesce) → §5.6 names existing `ClaimVerificationStateChanged` signal (verify at L1) + 250ms trailing-edge debounce per `(target.claim_id, surface)`; AC-339.6 added.
    - F5 MEDIUM (cursor in DTO sketches) → §5.1 envelope DTO updated: `touchpoints: Paginated<TouchpointBundle>`, same for `open_loops`, `metadata_proposals`, `record_entries`, `threads`; AC-459.9 + AC-507.10 added; CursorState policy folded into §13 + §5.1 / §5.2.
    - F6 MEDIUM (sub-lane sequencing fix — §5.4 dependency edge) → §13 Q10 staging rewritten; §5.4 moves from Stage 1b to Stage 1a; new staging: Stage 1a (§5.6 + §5.5 + §5.1 + §5.4); Stage 1b (§5.7 + §5.8 + §5.9 + §5.2 + §5.3); Stage 1c (§5.10).
    - F9 LOW (ProvenanceRef per-fact) → §5.1 retypes `EntityFact.provenance: Vec<ProvenanceSource>` → `provenance: ProvenanceRef` per ADR-0130 §2 amendment; top-level `EnvelopeProvenance` stays on `AbilityOutput` wrapper.
    - F7 + F8 path-α → filed to maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per memory `feedback_l2_path_alpha_to_maintenance_project`.
  - **Correctness (CONDITIONAL → APPROVED on V1.1):**
    - F1 CRITICAL (§13 Q7 write-commutativity wrong) → two-part contract: plain user-authored fields (agenda/notes/preparation_text/hidden_attendees) commute with `enqueue_refresh` (property-tested); decision/commitment claim emission goes through claim store with eventual consistency via existing signal substrate.
    - F2 HIGH (DOS-461 no-bypass edge cases) → AC-461.6 split into 6a (text-extraction harness — every visible claim-substantive string traces back to `[data-claim-id]` ancestor) + 6b (unresolved `data-claim-id` renders as stale, not bypass; distinguished failure mode with `account_claim_retracted_mid_render.json` fixture).
    - F3 HIGH (DOS-507 BriefingState taxonomy) → §5.10 retyped to composed struct: `BriefingState { availability, freshness, integrity, advisories }`; AC-507.4 rewritten.
    - F4 HIGH (cursor invalidation policy) → §13 Q11 resolved: `CursorState: Stable | DataShifted { advisory } | Invalidated { reason, restart_required: true }`; concurrent insert/retract tested.
    - F5 HIGH (privacy redaction matrix derived/composed) → AC-341.10 expanded: mixed-sensitivity = max(inputs); cross-claim references render as `redacted: true, label: "<source type> (redacted)"`; derived claims carry `derived_from: Vec<ClaimId>` with max-across-chain check.
    - F6–F10 (envelope error propagation, fixture matrix, idempotency window, audit denylist extensibility, prep state machine) → folded into AC additions (AC-459.10, AC-461.5b, AC-8.9, AC-340.7, AC-335.13).
  - **CSO (CONDITIONAL APPROVE × 3 → APPROVED on V1.1):** 12 findings folded:
    - F1 (symbol drift) → all references replaced: `render_policy_for` → `render_policy_for_surface(claim, surface, actor)`; `Actor`/`SurfaceContext` → `RenderActor`/`RenderSurface`; `can_surface_for(actor, surface, claim_id)` → `can_surface_for(&AppState, actor, surface, claim_id)`; `render_receipt_for(target, surface)` → `render_receipt_for(&AppState, target, surface)`. `renderable_claim_text_with_value` added to §5.4 cite list.
    - F2 (DOS-477 allowlist-primary) → contract locked: receipt allowlist is primary; denylist becomes redundant CI lint. New `scripts/check_sensitivity_gate_composition.sh` grep gate forbidding `match … sensitivity` outside `abilities-runtime/src/sensitivity*`. AC-477.11 + AC-477.12 added.
    - F3 (validate_envelope_target composition) → AC-477.13: accepts envelope-set (parent + transitively composed children via abilities-runtime `composes` declaration).
    - F4 (free-text sensitivity classification) → AC-8.9: user-authored free-text in `ClaimFeedbackRequest.metadata` persisted as `Confidential` by default.
    - F5 (FeedbackAction variant divergence vs ADR-0123) → AC-8.10: per-action metadata JSON schema MUST match ADR-0123 §1 variant payload field names verbatim.
    - F6 (WrongSource hash not index) → AC-8.11: `WrongSource` metadata carries stable source content hash per ADR-0131 canonicalization, not `source_index`.
    - F7 (NeedsNuance sanitizer) → AC-8.12: `NeedsNuance.corrected_text`, `CannotVerify.note` pass through ADR-0108 §3 sanitizer.
    - F8 (Agent actor denial) → AC-8.13: in v1.4.4 W1, `Actor::Agent` is denied at all surfaces; matrix row collapses to deny everywhere; broadening waits for v1.4.7 MCP scope manifest.
    - F9 (idempotency window) → AC-8.14: server-issued key scoped per `(claim_id, action, actor, metadata_hash)`; TTL ≤ 60 seconds.
    - F10 (DOS-341 allowlist-primary) → §5.9 contract restructured: `build_receipt_for_audience(target, audience, conn) -> ClaimReceipt` (audience as input to construction, not filter on output); per-audience field allowlist in `privacy.rs` const; AC-341.10.
    - F11 (OperationalAuditStorage enforcement) → AC-341.11: `check_audit_disclosure_allowlist.sh` extended to forbid any direct read of `maintenance_audit` from `services::claim_receipt::*`.
    - F12 (AgentMcp audience) → AC-341.12: explicit AgentMcp row with field allowlist (trust band, freshness coarsened, redaction level, lifecycle, sanitized evidence_summary, subject_type); forbid source labels, source_asof timestamps, claim_id.
  - **Codex Challenge (APPROVE WITH CHANGES → APPROVED on V1.1):**
    - F1 HIGH (AC-W1.1/W1.2 unenforced) → AC-W1.9 added: `scripts/check_w1_consumer_skeleton.sh` CI gate asserts each W1 producer has at least one `wp/dailyos/blocks/**/render-functions.php` consumer; substrate-only PRs without reference fail CI.
    - F2 LOW (DOS-701 LOC drift) → §6 + §5.6 LOC counts re-verified against `wc -l`: auth.rs=319, contracts.rs=175, render.rs=290 (already accurate in changelog tuple; §5.6 line reordered to match disk order).
    - F3 LOW (§5.8 IL check Q5 thin) → §5.8 Q5 re-authored: "Boundary violations detected by §5.8 CI lint or by user reports of leaked audit-only fields feed back as DOS-8 `WrongSource`/`SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract."
    - F4 LOW (AC-W1.1 inconsistency) → AC-W1.1 updated: `+ DOS-461 harness green` added to Account + Person rows to match Project row.
  - **Codex Consult (APPROVE WITH FINDINGS → APPROVED on V1.1):**
    - F1 + F2 (signature drift on `can_surface_for` + `render_receipt_for`) → folded into the CSO F1 symbol sweep above.
    - F3 (Proposal-receipt deferral) → §5.6 explicit deferral note: `render_receipt_for` shipped only handles `Claim` targets; `Proposal` and `WorkItem` return `TargetNotFound`. W4 budget does NOT include proposal receipt rendering.
    - F4 + F5 info-only — no fold needed.

  **Cycle-2 dispatch:** Not required. All 23 findings either (a) folded into V1.1 inline, (b) routed to maintenance project as path-α, or (c) info-only. Re-dispatch only if any cycle-1 reviewer disputes a fix surface. Per memory `feedback_l0_partial_convergence_when_class_recurs`: V1.1 reflects scope-correct, contract-complete.

## 3. K-in record (substrate-grep audit)

Per CLAUDE.md "Knowledge store discovery" + engineering-ladder.md L0 K-in obligation. Both directories greped 2026-05-20.

### `docs/solutions/` — 16 .md files at scan time; 4 cross-references relevant; no documented prior substrate reinvented

Full inventory at scan:
```
docs/solutions/architecture-patterns/append-only-jsonl-schema-change-preserves-hash-chain-2026-05-20.md
docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md
docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md
docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md
docs/solutions/tooling-decisions/codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md
docs/solutions/tooling-decisions/gh-pr-merge-delete-branch-multi-worktree-incompatibility-2026-05-19.md
docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md
docs/solutions/tooling-decisions/pre-push-hook-duration-vs-ssh-idle-timeout-2026-05-19.md
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md
docs/solutions/workflow-issues/node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md
docs/solutions/workflow-issues/parallel-wave-synced-from-conflicts-2026-05-19.md
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
docs/solutions/workflow-issues/worktree-setup-needs-pnpm-install-2026-05-19.md
```

**Greps run:** `entity_intelligence`, `claim_receipt`, `meeting_prep`, `briefing`, `trust_boundary`, `feedback_action`, `provenance`, `redaction`, `open_loops`, `touchpoints`, `sensitivity`, `subject_isolation`, `audit_boundary`.

**Relevant cross-references (cited in the body of this packet):**
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` — sets the precedent for class-sweep treatment of sensitivity boundaries. Applied in §5.7 (DOS-8) + §5.8 (DOS-340) + §5.9 (DOS-341): each surface allowlist sweep covers ALL channels, not patch-by-patch.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — lines 14-20: grep for ACTUAL substrate names, not proposed type vocabulary. Applied throughout §5; every DTO sketch confirms the existing substrate type names (`ClaimVerificationState`, `SubjectRef`, `RenderableClaimText`, `ClaimSensitivity`) match the ones shipped in `abilities-runtime`.
- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — sets precedent: substrate-only waves need an L0 amendment recording the wiring obligation. This packet IS that amendment for W1.
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — when reviewers surface 5+ net-new findings per cycle, scope is wrong. Applied in §4: 10 sub-tickets stay scoped to substrate ONLY; surface-side concerns route to W2–W5 packets.

**Verdict: net new substrate territory for entity intelligence envelope (DOS-459), meeting prep status DTO (DOS-335), and `get_daily_briefing` Read ability (DOS-507). Claim receipt substrate (DOS-339) extends DOS-701-shipped foundation. No documented prior solution reinvented.**

### `.docs/decisions/` — ADRs consumed (none overridden)

| ADR | Title | How W1 consumes |
|---|---|---|
| 0057 | Entity intelligence architecture | Anchors DOS-459 envelope shape; envelope is the typed manifestation of "entity intelligence" the ADR named. |
| 0083 | Product vocabulary | All user-facing strings emitted from W1 substrate (empty-state reasons, freshness caveats, feedback action labels) use the product vocabulary registry. |
| 0102 | Abilities runtime | `get_entity_intelligence` + `get_daily_briefing` are Read abilities per this contract. Allowed-actor sets respect the ADR's safety model. |
| 0105 | Trust scoring | `ReceiptTrust.band` + `ReceiptTrust.freshness` consume the trust-band + decay primitives this ADR established. |
| 0108 | Provenance rendering + privacy | DOS-341 privacy matrix is a per-surface application of this ADR's render-policy model; W1 does NOT introduce a second redaction system. |
| 0113 | Human + agent analysis as first-class claim sources | DOS-339 receipt carries the analysis claim source-type without special-casing. |
| 0123 | Typed claim feedback semantics | DOS-8 (W1) implements the consumer wiring for the typed enum this ADR pinned (9 variants at `abilities-runtime/src/abilities/feedback.rs:31`). |
| 0125 | Claim anatomy + temporal + sensitivity + TypeRegistry | Receipt + envelope DTOs carry the anatomy fields from this ADR; no new `claim_kind` introduced. |
| 0129 | Composable surfaces — WordPress Studio as primary surface | W1 substrate is surface-agnostic; W2–W5 WP block consumers prove the ADR's "render anywhere" promise. |
| 0130 | Surface-independent composition contract | Producer→projection→renderer split: W1 owns producers; W2–W5 own renderers; projection layer is the contract. |
| 0131 | Structured embedding + claim canonicalization | Provenance source dedup in receipts respects canonical claim identity, not surface-string equality. |

**Verdict: K-in complete. 11 ADRs consumed verbatim; no overrides; no reinvention. The substrate that is genuinely net-new (envelope, prep status DTO, daily briefing ability) is net-new because the ADR landscape already named these as gaps to be filled in v1.4.x — this packet picks the spec up where it was left.**

## 4. Scope summary

10 sub-tickets organized into 4 sub-lanes (W1-A entity intelligence; W1-B meeting prep/readiness; W1-C briefing ability; W1-D claim receipt finishers). DOS-701 (claim_receipt substrate carve-out) is **Done** and is the precondition for W1-D.

| Sub-ticket | Producer (service or ability) | Consumer surfaces (W2+) | Migration needed? | Reviewer panel |
|---|---|---|---|---|
| DOS-459 | `abilities::get_entity_intelligence` (Read ability) + `services::entity_intelligence::*` | W2: Account/Project/Person Detail blocks; W3: briefing references | N — read-side composition over existing claim/entity tables | `/cso` + architect + codex challenge |
| DOS-460 | `services::entity_intelligence::touchpoints` + `services::context::list_open_loops` extension | W2 entity detail open-loop/touchpoint sections; W3 briefing | N — read-side; extends `list_open_loops` ability hook only | architect + codex consult |
| DOS-461 | `tests/entity_intelligence_no_bypass/` red-first harness + fixtures | All W2 entity surfaces (gate, not consume) | N — test harness only | codex challenge + correctness reviewer |
| DOS-477 | `abilities-runtime::sensitivity` + `services::entity_intelligence::auth` (delegates to `services::claim_receipt::auth::can_surface_for`) | W2 entity detail every claim/proposal row | N — boundary types only | **`/cso` MANDATORY** + architect + codex challenge |
| DOS-335 | `services::meeting_prep_status::{read,write}` (new — split per cycle-1 architecture F2) | W3 briefing chrome + meeting detail + FolioBar readiness signal | Y — v241 `meeting_prep_status_signals` index + v242 `meeting_prep_status_dismissals` | architect + codex challenge + `/cso` (touches sensitivity on user-authored notes) |
| DOS-339 | `services::claim_receipt::*` (DOS-701 shipped; W1 wires consumers) + `claim_receipt::feedback` finisher | All W2/W3/W4 surfaces that render claims | N — DOS-701 already shipped contracts + migration v240 | `/plan-devex-review` + architect + codex consult |
| DOS-8 | `services::claim_receipt::feedback` (fills DOS-701 placeholder) + reuses `services::claims::record_claim_feedback` | W4 Actions/Work + W2 entity detail correct-claim affordances | N — extends typed enum substrate; no new tables | `/cso` + `/plan-devex-review` + codex challenge |
| DOS-340 | `services::claim_receipt::boundary` (fills DOS-701 placeholder) + `scripts/check_audit_disclosure_allowlist.sh` CI lint | All W2–W5 surfaces (boundary, not consume) | N — render-side filter + CI gate | **`/cso` MANDATORY** + correctness + codex challenge |
| DOS-341 | `services::claim_receipt::privacy` (fills DOS-701 placeholder) | All W2–W5 surfaces (render policy gate) | N — render policy extension | **`/cso` MANDATORY** + architect + codex challenge |
| DOS-507 | `abilities::get_daily_briefing` (new Read/User-only ability) | W3 Daily Briefing block | N — composes existing `prepare_meeting` + entity intelligence outputs | `/cso` + `/plan-devex-review` + architect + codex challenge |

Migration slot block claimed: **v240–v249** (10 slots; DOS-701 took v240; W1 needs ~5 more in practice — see §9). Per v1.4.6-waves.md §327 + §414 cross-version coordination table: "v1.4.4 holds v240–v249."

## 5. Detailed sections (one per sub-ticket)

### §5.1 — DOS-459: `get_entity_intelligence` envelope

**Contract shape (Rust trait + ability signature):**

```rust
// New ability at: src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/mod.rs

#[ability(
  name = "get_entity_intelligence",
  category = Read,
  version = "0.1.0",
  schema_version = 1,
  allowed_actors = [User, Agent, System],
  allowed_modes = [Live, Simulate, Evaluate],
  requires_confirmation = false,
  may_publish = false,
  required_scopes = ["read.entity_intelligence"],
  mcp_exposure = Invocable,  // v1.4.7 will gate; here we declare the eventual posture
  composes = [
    { id = "get_entity_context", ability = "get_entity_context", optional = false },
    { id = "list_open_loops", ability = "list_open_loops", optional = false }
  ],
  experimental = false,
  signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_entity_intelligence(
  ctx: &AbilityContext<'_>,
  input: EntityIntelligenceInput,
) -> AbilityResult<EntityIntelligenceEnvelope> { … }

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntelligenceInput {
  pub schema_version: u32,                  // = 1
  pub entity_type: EntityKind,              // Account | Project | Person
  pub entity_id: String,
  pub depth: ContextDepth,                  // Shallow | Standard | Deep (existing enum in get_entity_context)
  pub sections: Option<Vec<EnvelopeSection>>, // None = all
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityIntelligenceEnvelope {
  pub schema_version: u32,
  pub subject: NormalizedSubject,                          // { kind, id, subject_ref, display_label }
  pub sections: BTreeMap<EnvelopeSection, SectionState>,   // typed presence/empty-reason map
  // All list-shape fields paginated per cycle-1 architecture F5 + correctness F4
  pub facts: Paginated<EntityFact>,                        // claim-backed; carries field_path, trust, source_asof; provenance: ProvenanceRef per ADR-0130 §2
  pub health_story: Option<HealthStory>,                   // interpretive rows; evidence hooks
  pub metadata_proposals: Paginated<MetadataProposal>,     // typed claim/proposal records per DOS-328
  pub open_loops: Paginated<OpenLoopWithReceipt>,          // from §5.2 DOS-460
  pub touchpoints: Paginated<TouchpointBundle>,            // from §5.2 DOS-460 (next_cursor on the bundle)
  pub threads: Paginated<ThreadSummary>,                   // from DOS-297 when available; else empty + reason
  pub record_entries: Paginated<RecordEntry>,              // claim-backed timeline rows
  pub trust: EnvelopeTrustSummary,                         // aggregate trust posture + per-section caveats
  pub provenance: EnvelopeProvenance,                      // display-safe; respects DOS-477 redaction; top-level only — per-fact uses ProvenanceRef
  pub sensitivity: ClaimSensitivity,                       // existing ADR-0125 enum; top-level minimum visibility
}

/// Cursor envelope wrapper — server-signed opaque cursor per §13 Q11.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Paginated<T> {
  pub items: Vec<T>,
  pub next_cursor: Option<Cursor>,
  pub cursor_state: CursorState,                           // per correctness F4
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CursorState {
  Stable,
  DataShifted { advisory: String },                        // continue is safe; some rows may be skipped/duplicated
  Invalidated { reason: String, restart_required: bool },  // caller must restart from page 1
}

/// Per-fact provenance reference — points into the envelope's top-level `EnvelopeProvenance`
/// per ADR-0130 §2 amendment; prevents 64KB serialized-provenance-cap blowup ADR-0108 names.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRef {
  pub source_ids: Vec<String>,                             // index into EnvelopeProvenance.sources
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeSection {
  Facts, Health, MetadataProposals, OpenLoops, Touchpoints, Threads, Record,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SectionState {
  Present { item_count: usize },
  Empty { reason: EmptyReason }, // NotConnected | NotProcessedYet | FilteredOutBySubject | NoRelevantTouchpoints | Stale | NoEvidenceBackedProposal | UnsupportedForSubject
}
```

**Producer placement:**
- Ability: `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/{mod,prompts?,synthesis}.rs` — Read category, no provider synthesis; pure composition over `get_entity_context` (entries) + `list_open_loops` + existing `services::accounts`/`projects`/`people` read paths.
- Service helper: `src-tauri/src/services/entity_intelligence/mod.rs` — pure read composition with explicit empty-reason logic. The ability calls the service; the service composes; the service does NOT write.
- TypeScript mirror: `src/services/entity-intelligence/contracts.ts` with golden parity test against the Rust schema.

**Consumer surfaces and projection:**
- W2 Account Detail block (DOS-462): consumes envelope via WP block render PHP that calls the local-to-local runtime through `class-dailyos-runtime-client.php`. Block-render PHP projects envelope → editorial HTML with TrustBandBadge + EntityChip + ProvenanceTag primitives (already shipped in v1.4.3 W1).
- W2 Project Detail (DOS-483), Person Detail (DOS-484): identical envelope; block-render switches projection rules per entity type.
- W3 Daily Briefing references entities via `subject_ref` only; the briefing ability (§5.10 DOS-507) calls `get_entity_intelligence` with `depth: Shallow + sections: [Facts, OpenLoops]` for each referenced subject.
- Per ADR-0130: producer (ability) → projection (block-render PHP) → renderer (Gutenberg block + tokens). W1 owns producer; W2 owns projection + renderer.

**Intelligence Loop integration check:**
1. *Claim model:* Envelope is a read-side projection of existing claim/proposal records; no new claim_kind. `facts[].claim_id` + `metadata_proposals[].proposal_id` + `record_entries[].claim_id` preserve subject/field binding per ADR-0125.
2. *Provenance + trust:* Every `EntityFact` carries `source_asof`, `trust_band`, `freshness`, and `provenance: ProvenanceRef` (per cycle-1 architecture F9 / ADR-0130 §2 amendment — points into envelope-level `EnvelopeProvenance.sources`; display-safe with DOS-477 redaction applied at the service boundary, not the renderer). Missing `source_asof` → `Freshness::Unknown` + currentness caveat (never fabricated).
3. *Signals + invalidation:* Envelope re-renders on existing claim-lifecycle signals + entity-link signals (DOS-258 path); no new signals from W1-A. `signal_policy = { emits_on_output_change = [], coalesce = false }` matches `get_daily_readiness` pattern.
4. *Runtime + surfaces:* Consumed by W2 entity detail blocks (Tauri WP runtime), v1.4.7 MCP (declared `Invocable`, gated at exposure-time). The same producer is invocable from MCP for headless agents per ADR-0130 anchored decision #6.
5. *Feedback loop:* Receipt-shaped actions on envelope items (e.g., `WrongSubject` on a metadata_proposal) route through DOS-8 typed feedback (§5.7); corrections feed claim lifecycle via existing `services::claims::record_claim_feedback`. Envelope re-fetches reflect the new state through standard lifecycle signals.

**Migration slots:** None. Read-side composition over existing tables.

**Acceptance criteria:**
- **AC-459.1** — Ability registered at `abilities-runtime` registry with schema_version=1 + Read category + User/Agent/System actors.
- **AC-459.2** — Envelope returns same shape for Account / Project / Person subjects; empty sections carry typed reasons (no `null` without reason); `sections` map enumerates all 7 EnvelopeSection variants in every response.
- **AC-459.3** — `get_entity_context` (existing) continues to work unchanged; no in-place breaking change.
- **AC-459.4** — Output never exposes legacy AI JSON as source of truth; `evidence_summary` strings are claim-backed.
- **AC-459.5** — Every rendered claim/proposal item carries DOS-477 target binding: `claim_id` OR `proposal_id`, `subject_ref`, `field_path`, `sensitivity`, `trust_band`, lifecycle state, display-safe provenance.
- **AC-459.6** — Tests cover: account / project / person / empty / stale-fact / corrected-superseded / ambiguous-association / project-account-overlap / wrong-subject correction / subject isolation. Fixture suite cross-referenced from DOS-461 harness (§5.3).
- **AC-459.7** — Reuse audit: at L0 plan, identify which existing producers are reused (`get_entity_context`, `list_open_loops`, `services::accounts`, etc.) and justify any new reader/composer.
- **AC-459.8** — TypeScript mirror exists + parity-tested (golden file pattern; template: `src/services/claim-receipt/contracts.ts`).
- **AC-459.9** — (cycle-1 architecture F5 + correctness F4) Every list-shape field in the envelope carries a typed `Paginated<T>` with `next_cursor` + `CursorState`. Consumer pagination tested via re-invocation; cursor is opaque server-signed (§13 Q11) and survives schema changes. Concurrent insert/retract during pagination is tested: harness inserts a claim between page fetches and asserts behavior matches the declared `CursorState`.
- **AC-459.10** — (cycle-1 correctness F6 + residual R3) Partial-failure + signal-driven refresh semantics specified. Per-section composition is independent (one section failing emits `Empty { reason: PartialFailure }`, others continue). Errors affecting subject identity OR auth fail the entire envelope. Signal-driven envelope refresh tested: every claim-lifecycle transition (`Active → Contested` etc.) triggers re-render within signal propagation budget. Fixture: `account_partial_section_failure.json`.

---

### §5.2 — DOS-460: Canonical entity touchpoints + open-loops contract

**Contract shape:**

```rust
// Extension at: src-tauri/src/services/entity_intelligence/touchpoints.rs (new)
// Reuses existing services::context::list_open_loops + services::accounts touchpoint helpers.

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TouchpointBundle {
  pub upcoming: Paginated<Touchpoint>,       // per cycle-1 architecture F5
  pub recent: Paginated<Touchpoint>,
  pub candidate_set: CandidateSetRef,        // typed basis: meetings query window + filter
  pub empty_reason: Option<EmptyReason>,     // typed; see DOS-459 §5.1
  pub subject_scope: SubjectScope,           // explicit; prevents bleed
  pub next_cursor: Option<Cursor>,           // bundle-level cursor when wrapped by envelope's Paginated<TouchpointBundle>
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Touchpoint {
  pub meeting_id: Option<String>,            // optional — interaction may be non-meeting
  pub kind: TouchpointKind,                  // Meeting | EmailThread | Document | Salesforce | Linear
  pub when: DateTime<Utc>,
  pub subject_ref: SubjectRef,
  pub inclusion_reason: InclusionReason,     // SubjectMatch | EntityLink | AttendeeMatch | DomainMatch
  pub exclusion_reason: Option<ExclusionReason>, // populated only on debug + empty-state diagnostics
  pub trust: TouchpointTrust,                // band + freshness; reuses claim trust primitives
  pub provenance: ProvenanceRef,             // per cycle-1 architecture F9; points into envelope-level EnvelopeProvenance
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenLoopWithReceipt {
  pub open_loop: abilities_runtime::abilities::list_open_loops::OpenLoop, // existing type
  pub receipt_target: services::claim_receipt::contracts::ReceiptTarget,  // links to W1-RECEIPT DTO
  pub trust: ReceiptTrust,                                                // existing type
  pub provenance: ProvenanceRef,                                          // per cycle-1 architecture F9
}
```

**Producer placement:**
- `src-tauri/src/services/entity_intelligence/touchpoints.rs` — composes existing `services::accounts::list_recent_meetings_for_account` + sibling `projects` + `people` helpers + `services::context::list_open_loops` query (already exists per `abilities-runtime/src/abilities/list_open_loops` consumer chain).
- NO new ability surface; this is a service helper consumed by the DOS-459 envelope ability (§5.1). Reusing the existing `list_open_loops` ability is correct — DOS-460 is the *contract* for what gets returned in the envelope, not a parallel ability.

**Consumer surfaces and projection:**
- W2 Account/Project/Person Detail blocks consume `TouchpointBundle` + `Vec<OpenLoopWithReceipt>` directly off the envelope (§5.1). No page-local touchpoint composition.
- W3 Daily Briefing references upcoming touchpoints via the same envelope (briefing ability calls `get_entity_intelligence(sections=[Touchpoints])` per referenced subject).

**Intelligence Loop integration check:**
1. *Claim model:* Open loops are existing claim-backed records (`ClaimType::OpenLoop` + `ClaimType::Commitment` per the existing `list_open_loops` ability). Touchpoints are meeting/interaction records — not claims themselves, but they carry `subject_ref` + provenance for the inclusion/exclusion decision.
2. *Provenance + trust:* Each touchpoint carries trust band + freshness; inclusion/exclusion reasons are part of the provenance ("why this belongs"). Subject isolation tests prevent bleed across ambiguous-account / multi-account-person / project-account-overlap cases.
3. *Signals + invalidation:* Re-renders on entity-link signals (DOS-258), calendar-sync signals, claim-lifecycle signals. No new signals.
4. *Runtime + surfaces:* W2 entity blocks + W3 briefing block consume.
5. *Feedback loop:* `WrongSubject` on a misattributed touchpoint routes through DOS-8 typed feedback (§5.7); subject reattribution updates the candidate-set basis.

**Migration slots:** None. Service-side composition.

**Acceptance criteria:**
- **AC-460.1** — Account / Project / Person contexts expose open loops + touchpoints through the same envelope shape (per §5.1).
- **AC-460.2** — Person context no longer depends on a special legacy path to answer upcoming meetings/actions; tests assert no `gather_account_context`-shaped fallback is called for Person subjects.
- **AC-460.3** — `inclusion_reason` + `exclusion_reason` available to downstream consumers + tests.
- **AC-460.4** — Account / Project / Person Detail blocks (W2) use these fields for visible open-loop/touchpoint/work/record rows; page-local derivations are tested-absent (no raw `meetings` or `actions` DB queries from block render PHP).
- **AC-460.5** — Subject isolation tests cover: ambiguous people, multi-account meetings, project/account overlap, parent/child accounts, direct vs propagated email signals, entity links. Cross-referenced in DOS-461 harness (§5.3).
- **AC-460.6** — Outputs include trust / provenance / freshness metadata for each touchpoint + open-loop item.
- **AC-460.7** — Daily briefing readiness/count divergence class (DOS-278) has a reusable candidate-set primitive — `CandidateSetRef` is the explicit primitive.

---

### §5.3 — DOS-461: Entity fixture harness + no-bypass checks

**Contract shape (test harness):**

```rust
// New harness: src-tauri/tests/entity_intelligence_no_bypass/
//   ├── mod.rs                    (harness driver — invokes via abilities runtime only)
//   ├── account_proof.rs          (Account Detail proof path)
//   ├── project_proof.rs          (Project Detail proof path)
//   ├── person_proof.rs           (Person Detail proof path)
//   ├── no_bypass_assertions.rs   (red-first: fails if any proof path uses bypass)
//   └── fixtures/
//       ├── account_stale.json
//       ├── account_corrected_superseded.json
//       ├── account_low_trust.json
//       ├── account_metadata_proposal.json
//       ├── account_open_loop.json
//       ├── account_upcoming_touchpoint.json
//       ├── account_recent_touchpoint.json
//       ├── account_thread_summary.json
//       ├── account_confidential_user_only_claim.json
//       ├── account_glean_citation.json
//       ├── account_wrong_subject_correction.json
//       ├── project_*.json   (full set per project subject)
//       └── person_*.json    (full set per person subject)
```

**Producer placement:** Tests only — no new substrate. The harness invokes the abilities runtime (`get_entity_intelligence`) ONLY; any proof path that uses legacy `get_entity_context_entries`, raw SQL, `AppState`, `ActionDb`, page-local intelligence composers, or legacy direct detail commands as the source of rendered claim-backed intelligence FAILS the no-bypass check.

**Consumer surfaces:** W2 entity detail blocks gate on this harness passing. Releasing v1.4.4 W2 without harness-green is blocked.

**Intelligence Loop integration check:**
1. *Claim model:* Fixtures use generic PII-safe synthetic data only (per CLAUDE.md "No customer-specific data in source code").
2. *Provenance + trust:* Each fixture asserts trust/provenance/sensitivity metadata is present + usable for rendering.
3. *Signals + invalidation:* Fixture cases include stale-fact + corrected-superseded → harness verifies envelope reflects current state.
4. *Runtime + surfaces:* Harness is the gate for W2 entity surface ships.
5. *Feedback loop:* `wrong_subject_correction` fixture verifies the feedback action correctly retracts subject binding via the envelope re-fetch.

**Migration slots:** None.

**Acceptance criteria:**
- **AC-461.1** — Add downstream-style fixtures for Account / Project / Person detail routes that consume the entity-intelligence envelope ONLY through abilities/runtime handles.
- **AC-461.2** — Account fixture remains richest primary proof (covers metadata proposals + correction loops in depth).
- **AC-461.3** — Project fixture covers trajectory / horizon / stakeholder / team context / record entries / work / open loops / touchpoints / trust / provenance / empty states.
- **AC-461.4** — Person fixture covers profile / dynamic / rhythm / network / relationships / record entries / work / open loops / touchpoints / ambiguous-association / trust / provenance / empty states.
- **AC-461.5** — Fixtures include stale fact, corrected-superseded claim, low-trust claim, metadata proposal (per subject-matrix below), open loop, upcoming touchpoint, recent touchpoint, thread summary, confidential/user-only claim, Glean citation, wrong-subject claim.
- **AC-461.5b** — (cycle-1 correctness F7) Per-subject expected-fixture matrix is explicit. Absent expected fixture = harness fail:

  ```
                          Account  Project  Person
  metadata_proposal         ✓        ✓        -
  upcoming_touchpoint       ✓        ✓        ✓
  recent_touchpoint         ✓        ✓        ✓
  thread_summary            ✓        ✓        ✓
  glean_citation            ✓        ✓        ✓
  wrong_subject             ✓        ✓        ✓
  ambiguous_association     -        -        ✓   (Person-only)
  project_account_overlap   ✓        ✓        -
  parent_child              ✓        -        -   (Account-only)
  ```
- **AC-461.6a** — (cycle-1 correctness F2 — binding check) Text-extraction harness pass: every visible claim-substantive string in the rendered DOM (text originating from `IntelligenceClaim.text` / `RenderableClaimText.body` per the envelope) MUST trace back to a `[data-claim-id]` ancestor. Text-without-binding FAILS. Red-first no-bypass test also FAILS if any proof path uses legacy AI JSON / `get_entity_context_entries` fallback / raw SQL / `AppState` / `ActionDb` / page-local intelligence composers / legacy direct detail commands as the source of rendered claim-backed intelligence.
- **AC-461.6b** — (cycle-1 correctness F2 — stale vs bypass) Unresolved `data-claim-id` (claim no longer in envelope but DOM was cached from a prior render) renders as **stale**, NOT **bypass**. Distinguished failure mode with its own assertion + fixture (`account_claim_retracted_mid_render.json`). Harness produces separate counters for `stale_render_count` and `bypass_count`.
- **AC-461.7** — Grep/check targets include `AccountDetailPage`, `useAccountDetailPage`, `useAccountDetail`, `ProjectDetailEditorial`, `useProjectDetail`, `PersonDetailEditorial`, `usePersonDetail`, the new envelope hook, AND the WP block render PHP entry points (`wp/dailyos/blocks/account-detail/render-functions.php` etc.).
- **AC-461.8** — Reuse audit artifact in proof bundle lists existing abilities / services / DTOs / tests / UI primitives reused (per CLAUDE.md DoD #7 style).

**Note on substrate-only-with-no-W2-consumer scope:** DOS-461 is genuinely substrate-only (a test harness has no rendering consumer). It STAYS in W1 because it underpins the no-bypass rule that gates W2 entity surfaces from shipping. Without this harness landing in W1, W2 has no mechanical proof that block render PHP doesn't bypass the envelope. Per memory `feedback_check_substrate_before_authoring_primitives`, this is the gate that prevents PR #224-style "visually polished surface that recreates its own composers" regressions.

---

### §5.4 — DOS-477: Entity-detail trust-boundary hardening

**Contract shape (boundary types + auth helper):**

```rust
// Service-side at: src-tauri/src/services/entity_intelligence/auth.rs (new)
// Delegates to shipped: src-tauri/src/services/claim_receipt/auth.rs (DOS-701)

/// Accepts envelope-set (parent + transitively composed children via abilities-runtime
/// `composes` declaration) per cycle-1 CSO F3. Walker traverses the composition graph
/// at invocation time; any target valid for a child envelope is valid for the parent.
pub fn validate_envelope_target(
  envelope_set: &EnvelopeSet,        // parent + transitively composed children
  target: &ReceiptTarget,            // existing W1-RECEIPT type
) -> Result<(), TargetBindingError> {
  // Asserts target.{claim_id|proposal_id} belongs to any envelope in the set:
  // .{facts|metadata_proposals|record_entries|open_loops}.
  // Mutations (accept/dismiss/edit/correct) call this BEFORE routing through services::claims::*.
}

pub fn validate_metadata_proposal_field(
  field_path: &str,
  entity_type: EntityKind,
) -> Result<(), FieldAllowlistError> {
  // Metadata proposals are field-allowlisted per entity_type.
  // Allowlist sourced from existing services::claims field schemas (no arbitrary JSON).
}

/// Composes (does NOT re-implement) the shipped sensitivity gate per cycle-1 CSO F1.
/// CI lint `check_sensitivity_gate_composition.sh` forbids any `match … sensitivity`
/// outside `abilities-runtime/src/sensitivity*`.
pub fn redact_provenance_for_surface(
  source: ProvenanceSource,
  actor: RenderActor,                // not the packet-introduced `Actor` — per cycle-1 CSO F1
  surface: RenderSurface,            // not `SurfaceContext` — per cycle-1 CSO F1
  sensitivity: ClaimSensitivity,
) -> ProvenanceSource {
  // Strips raw source_ref / Glean document IDs / chunk IDs / meeting IDs / URLs / emails /
  // filesystem paths / private snippets / raw prompt/audit identifiers unless render policy allows reveal.
  // Composes abilities_runtime::sensitivity::render_policy_for_surface(claim, surface, actor)
  // + renderable_claim_text_with_value(...) — never re-implements the gate.
}
```

**Producer placement:**
- `src-tauri/src/services/entity_intelligence/auth.rs` — boundary helpers; called by the envelope service before emitting + by every mutation entry point (accept/dismiss/edit/correct).
- Delegates to shipped `services::claim_receipt::auth::can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>` (DOS-701; signature corrected per cycle-1 codex-consult F1).
- Composes shipped `abilities_runtime::sensitivity::render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)` (per cycle-1 CSO F1). NO parallel sensitivity gate.

**Consumer surfaces:** Every W2 entity-detail mutation. Every claim/proposal row rendered. All accept/dismiss/edit/correct actions.

**Intelligence Loop integration check:**
1. *Claim model:* Target binding ensures mutations only touch claim/proposal records that belong to the rendered envelope; no orphan mutations.
2. *Provenance + trust:* Redaction layer preserves trust signal integrity — cite chips show source-type + currentness + trust band without leaking raw source IDs.
3. *Signals + invalidation:* No new signals.
4. *Runtime + surfaces:* Consumed by W2 entity blocks AND by future v1.4.7 MCP exposure (MCP surface validates target binding before exposing).
5. *Feedback loop:* `WrongSource` / `WrongSubject` actions still go through DOS-8 typed feedback (§5.7); auth helper validates before routing.

**Migration slots:** None — boundary types + validation.

**Acceptance criteria (verbatim from ticket + Intelligence Loop additions):**
- **AC-477.1** — Every rendered claim/proposal row in Account/Project/Person detail proof paths carries `claim_id` OR `proposal_id`, `subject_ref`, `field_path`, `sensitivity`, `trust_band`, lifecycle state, display-safe provenance.
- **AC-477.2** — Accept/dismiss/edit/correct actions validate target binding via `validate_envelope_target` BEFORE mutating metadata/trust/suppression/source reliability/repair state/feedback signals.
- **AC-477.3** — Metadata proposals are typed + field-allowlisted via `validate_metadata_proposal_field`. No proposal flow writes arbitrary full metadata JSON.
- **AC-477.4** — Cite chips + evidence drawers use sanitized display labels via `redact_provenance_for_surface`. No raw source_ref / Glean doc IDs / chunk IDs / meeting IDs / URLs / emails / filesystem paths / private snippets / raw prompt/audit identifiers unless render policy explicitly permits.
- **AC-477.5** — Correction/feedback payloads are sensitivity-classified; user-authored text does not leak into MCP / reports / logs / cite chips / generic provenance drawers.
- **AC-477.6** — Fixtures (cross-ref DOS-461 §5.3) include confidential/user-only claims, Glean citations, stale claims, corrected/superseded claims, wrong-subject claims, PII-safe fake data only.
- **AC-477.7** — Permanent person deletion removed from Person/Account detail proof path OR blocked behind hardened destructive-action flow (impact preview + typed confirmation + service-layer mutation + no raw names in logs/externally visible payloads).
- **AC-477.8** — Project archive/delete-like flows + Account stakeholder/person mutation flows checked for same service-layer + sensitivity guarantees when reachable from converted detail surfaces.
- **AC-477.9** — All mutation paths route through `services/`; no direct DB writes from commands / UI / block render PHP.
- **AC-477.10** — `/cso` L0 + L2 approval recorded.
- **AC-477.11** — (cycle-1 CSO F1) `services::entity_intelligence::auth::redact_provenance_for_surface` MUST compose shipped `render_policy_for_surface` + `renderable_claim_text_with_value` — never re-implement. CI lint `src-tauri/scripts/check_sensitivity_gate_composition.sh` (modeled on `check_claim_writer_allowlist.sh`) forbids any `match … sensitivity` against `ClaimSensitivity` variants outside `abilities-runtime/src/sensitivity*`. Pairs with the existing `prompt-channel-sensitivity-class-sweep` precedent (`docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`).
- **AC-477.12** — (cycle-1 CSO F2 — allowlist-primary boundary) Receipt allowlist (`RECEIPT_ALLOWED_FIELDS` from §5.8) is the **primary** boundary contract. Denylist (`AUDIT_ONLY_DENYLIST`) is a redundant CI lint that fails the build if any field outside the allowlist appears in a `ClaimReceipt`-JSON-serialized snapshot. `filter_for_receipt` panics on encountering unknown field names (fail-loud, per Rule 11). Snapshot fixture set under `src-tauri/tests/claim_receipt_boundary/` — one serialized ClaimReceipt per (`SurfaceContext` × `ClaimSensitivity`) cell. New fields added to `ClaimReceipt` FORCE snapshot update + allowlist amendment.
- **AC-477.13** — (cycle-1 CSO F3 — transitive composes) `validate_envelope_target` accepts an envelope-set (parent + transitively composed child envelopes via abilities-runtime `composes` declaration). Each ability's `composes = [...]` metadata defines the transitive set; the validator walks the composition graph at invocation-time. Property test: for every `(parent_ability, child_ability)` in `composes`, assert that a target valid for the child is valid for the parent. Failing pair = drift between renderer and validator. Concrete consumer: §5.10 DOS-507 daily briefing composes `get_entity_intelligence` per referenced subject; a `MarkFalse` on a claim shown in the briefing must validate against the transitively-composed envelope.

---

### §5.5 — DOS-335: Meeting prep / readiness DTO

**Contract shape:**

```rust
// New service: src-tauri/src/services/meeting_prep_status.rs

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPrepStatusDto {
  pub meeting_id: String,
  pub event_id: Option<String>,                 // calendar event id when distinct
  pub linked_entity: Option<EntityBinding>,     // None when blocked/no entity
  pub status: PrepStatus,
  pub blocking_reason: Option<BlockingReason>,
  pub stale_reason: Option<StaleReason>,
  pub last_prepared_at: Option<DateTime<Utc>>,
  pub source_asof_inputs: Vec<SourceAsofRef>,   // freshness inputs that fed prep
  pub trust_summary: Option<TrustSummary>,      // aggregate of underlying claims
  pub provenance: Option<EnvelopeProvenance>,   // display-safe
  pub next_allowed_transition: Vec<PrepStatus>, // ["queued", "user_suppressed"] etc.
  pub user_authored: UserAuthoredFields,        // agenda, notes, preparation text, hidden attendees, decisions
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepStatus {
  BlockedNoEntity,
  PrepNeeded,
  Queued,
  Running,
  Ready,
  Limited,           // ready but incomplete; carries `stale_reason`
  Stale,
  Failed,
  UserSuppressed,
  UserDismissed,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockingReason {
  NoLinkedEntity,
  AmbiguousAttendeeMatch,
  SourceRevoked,
  PolicyForbidden,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
  EntityContextStale,
  RecentCorrection,
  SourceAsofOlderThanThreshold,
  ContradictedClaimUpstream,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserAuthoredFields {
  pub agenda: Option<String>,
  pub notes: Option<String>,
  pub preparation_text: Option<String>,
  pub hidden_attendees: Vec<String>,           // display labels only; not raw emails
  pub decisions: Vec<DecisionRef>,             // claim-backed
}

// Per cycle-1 architecture F2 — read/write split enforces ADR-0102 §3 Read-ability call-graph.
// services::meeting_prep_status::read — pure read; no &mut; no signal emit
//   safe for Read-ability call graph (consumed by get_daily_briefing §5.10)
pub mod read {
  pub fn compute_status(meeting_id: &str, conn: &SqlConn) -> Result<MeetingPrepStatusDto, …>
  // No write paths. No lazy enqueue-on-stale. compute_status returns Stale state; caller decides.
}

// services::meeting_prep_status::write — mutating; called only from non-Read paths
pub mod write {
  pub fn enqueue_refresh(meeting_id: &str, reason: RefreshReason, conn: &SqlConn) -> Result<(), …>
  pub fn record_user_authored(meeting_id: &str, fields: UserAuthoredFields, actor: RenderActor, conn: &SqlConn) -> Result<(), …>
}
```

**State transition table** (per cycle-1 correctness F10; `next_allowed_transition` field references this):

| From → To | PrepNeeded | Queued | Running | Ready | Limited | Stale | Failed | UserSuppressed | UserDismissed |
|---|---|---|---|---|---|---|---|---|---|
| BlockedNoEntity | ✓ (entity linked) | - | - | - | - | - | - | - | - |
| PrepNeeded | - | ✓ | - | - | - | - | - | ✓ | ✓ |
| Queued | - | - | ✓ | - | - | - | ✓ (timeout) | ✓ | ✓ |
| Running | - | - | - | ✓ | ✓ | - | ✓ | - | - |
| Ready | - | ✓ (refresh) | - | - | - | ✓ | - | ✓ | ✓ |
| Limited | - | ✓ (refresh) | - | ✓ (re-run) | - | ✓ | - | ✓ | ✓ |
| Stale | - | ✓ (refresh) | - | - | - | - | - | ✓ | ✓ |
| Failed | - | ✓ (retry) | - | - | - | - | - | ✓ | ✓ |
| UserSuppressed | ✓ (un-suppress) | - | - | - | - | - | - | - | - |
| UserDismissed | ✓ (un-dismiss) | - | - | - | - | - | - | - | - |

Encoded as `pub fn legal_transitions(from: PrepStatus) -> &'static [PrepStatus]`. Illegal transitions return `TransitionError`.

**Producer placement:**
- `src-tauri/src/services/meeting_prep_status.rs` — service-owned status contract. All writers (calendar sync, manual entity linking, prep generation, user-authored writes) route through this service.
- DOES NOT introduce a parallel salience model (per ticket non-goal).
- Reuses DOS-258 entity-linking signals — manual link/relink/unlink emits a signal that `services::meeting_prep_status` subscribes to and recomputes.

**Consumer surfaces:**
- W3 FolioBar readiness chrome (FloatingNavIsland chapter status — already shipped W3 chrome lane, will consume DTO at W3 surface ship).
- W3 Meeting Briefing block.
- W3 Daily Briefing block (per-meeting status rollup).
- W2 Meeting Detail block (when authored — currently in W0 audit list).

**Intelligence Loop integration check:**
1. *Claim model:* DTO is read-side projection; user-authored writes (notes/agenda/decisions) go through `record_user_authored` which classifies sensitivity per ADR-0125 + ADR-0108 and emits claims for `decisions` (using existing `ClaimType::Decision` if present; else routes to commitment claim per DOS-129).
2. *Provenance + trust:* `source_asof_inputs` + `trust_summary` + `provenance` carry freshness/trust posture aggregate of underlying claims used to generate prep. Stale prep surfaces `StaleReason::SourceAsofOlderThanThreshold` (threshold is config-driven; default 24h).
3. *Signals + invalidation:* New invalidation path: entity-link signal (DOS-258) → `write::enqueue_refresh(RefreshReason::EntityRelinked)`. Recent-correction signal → `StaleReason::RecentCorrection`. **NEW signal emitted** (per cycle-1 architecture F3): `MeetingPrepStatusChanged { meeting_id, transition: PrepStatus → PrepStatus }`. Pre-declared in `signals/policy_registry.rs` at W1 kickoff, mirroring v1.4.6 W0 cycle-4 fix C pattern (`.docs/plans/v1.4.6-waves.md` §348). Consumers (FolioBar readiness chrome, Meeting Briefing block, Daily Briefing rollup) subscribe to this signal — no poll-on-render drift.
4. *Runtime + surfaces:* FolioBar (W3 chrome — already substrate-rendered), Meeting Detail (W2/W3 — TBD per W0 audit), Daily Briefing (W3), Meeting Briefing (W3). Same DTO across all.
5. *Feedback loop:* User suppression / dismissal writes update `status` to `UserSuppressed` / `UserDismissed`. These do not feed claim lifecycle (per ticket: "silent-by-default preparation; no proactive notifications, no insight queue, no interruption policy") — they only gate prep refresh re-enqueue.

**Migration slots:**
- v241 — `meeting_prep_status_signals` (indexed view materializing the join across `meetings` + `meeting_entity_links` + `meeting_prep_outputs` + `claim_invalidation_queue` rows for fast status compute). Use existing `services::invalidation_jobs` substrate; no new tables. (Renumbered from V1.0's v250 per cycle-1 architecture F1.)
- v242 — `meeting_prep_status_dismissals` (UserSuppressed/UserDismissed persistence; small table; index on `(meeting_id, actor, created_at)`). (Renumbered from V1.0's v251.)

**Acceptance criteria (verbatim from ticket + intelligence loop additions):**
- **AC-335.1** — `DOS-335` assigned to v1.4.4 W1; explicitly linked to DOS-258 (entity linking) + DOS-278 (readiness/count consistency).
- **AC-335.2** — Implementation audit names every current prep/readiness writer + reader + fields used.
- **AC-335.3** — Service-owned status contract defines (at minimum): meeting/event id, linked entity binding, status, blocking reason, stale reason, last_prepared_at, source-asof/freshness inputs, trust/provenance summary, next_allowed_transition.
- **AC-335.4** — Status vocabulary distinguishes: blocked/no_entity, prep_needed, queued, running, ready, limited, stale, failed, user_suppressed, user_dismissed.
- **AC-335.5** — Manual entity linking (DOS-258 path) causes prep status to recompute or enqueue refresh through services/abilities (not UI-local state).
- **AC-335.6** — Repro: calendar event starts unlinked + not ready → user manually links correct entity → FolioBar / event row/card / daily briefing / meeting detail converge on same status without app restart or unrelated sync.
- **AC-335.7** — If event is linked but no current prep output exists, surfaces show correct transitional state (not incorrectly count as ready or permanently not ready).
- **AC-335.8** — User-authored agenda / notes / preparation text / hidden attendees / decisions preserved across status recompute + re-enrichment.
- **AC-335.9** — Refresh respects correction / tombstone / lifecycle / trust / sensitivity / provenance invariants.
- **AC-335.10** — No direct DB writes from command handlers; all mutations through services/abilities.
- **AC-335.11** — Tests cover stale entity context / recent correction / ambiguous attendee match / no-op refresh / source-revoked evidence / manual link/relink/unlink / status convergence across FolioBar + event components / user-authored note preservation. Additional case: `decision_authored_during_refresh_eventually_visible_in_next_status_recompute_without_lost_update` (per cycle-1 correctness F1).
- **AC-335.12** — (cycle-1 architecture F2) `services::meeting_prep_status::read::compute_status` call graph contains zero mutations; verified by a trybuild test OR call-graph lint at registration time per ADR-0102 §3. Static check lands as part of W1, not deferred. Pairs with §5.10 AC-507.7.
- **AC-335.13** — (cycle-1 correctness F1 — write-commutativity contract, two-part):
  - **Part A:** Plain user-authored fields (agenda, notes, preparation_text, hidden_attendees) commute with `write::enqueue_refresh` writes — disjoint columns, no derivation. Property-tested at L1.
  - **Part B:** Decision/commitment claim emission via `write::record_user_authored` is a claim-store write, NOT a disjoint-column write. Status recompute observes the new claim through the standard claim-lifecycle-signal → invalidation path. Contract: "eventual consistency via existing signal substrate," not "commutes." The §13 Q7 V1.0 recommendation is corrected accordingly.
- **AC-335.14** — (cycle-1 correctness F10) State transition table is exhaustive (every variant has an entry — see table above). Illegal transitions return `TransitionError`; legal transitions succeed.
- **AC-335.15** — (cycle-1 architecture F3) `MeetingPrepStatusChanged` signal pre-declared in `signals/policy_registry.rs` at W1 kickoff. Consumer subscription test: dismissal write emits signal within 1 invalidation cycle; FolioBar readiness chrome re-renders without poll.

---

### §5.6 — DOS-339: Shared claim receipt contract (W1-RECEIPT extends DOS-701)

**Status:** DOS-701 (W1-RECEIPT carve-out) shipped the substrate at PR #323 (merged 2026-05-19). Confirmed shipped at scan time (2026-05-20):
- `src-tauri/src/services/claim_receipt/contracts.rs` (175 LOC) — full `ReceiptTarget`, `SurfaceContext`, `Freshness`, `RedactionLevel`, `ReceiptTrust`, `ReceiptLifecycle`, `ReceiptProvenance`, `ProvenanceSource`, `ClaimReceipt` per the L0 Addendum.
- `src-tauri/src/services/claim_receipt/render.rs` (290 LOC) — `render_receipt_for(state: &AppState, target: ReceiptTarget, surface: SurfaceContext) -> Result<ClaimReceipt, RenderError>` (signature corrected per cycle-1 codex-consult F2).
- `src-tauri/src/services/claim_receipt/auth.rs` (319 LOC) — `can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>` (signature corrected per cycle-1 codex-consult F1) + positive/negative fixtures per sensitivity class.
- Migration v240 — `claim_review_deferrals` table.
- TS mirror at `src/services/claim-receipt/contracts.ts` with parity test.

**Proposal-receipt deferral note** (per cycle-1 codex-consult F3): `render_receipt_for` shipped at PR #323 only handles the `Claim` arm of `ReceiptTarget`. The `Proposal { .. }` and `WorkItem { .. }` arms return `RenderError::TargetNotFound`. DTO ships all three variants but render only implements Claim. **W4 (Actions/Work) does NOT budget against proposal-receipt rendering end-to-end** — `submit_claim_feedback` (§5.7) on a Proposal target returns a typed "no receipt yet, target-only" response. Either DOS-701-deferred follow-on or W4-specific extension; surface flag in W4 L0 to avoid downstream surprise.

**Remaining W1 work (this packet):** wire the receipt to downstream consumers + fill the 5 placeholder files (feedback, boundary, privacy, contradiction, render_rules). Those placeholders are owned individually by sub-tickets:
- `feedback.rs` → §5.7 (DOS-8)
- `boundary.rs` → §5.8 (DOS-340)
- `privacy.rs` → §5.9 (DOS-341)
- `contradiction.rs` → deferred to v1.4.4 W4 (Action surfaces) — substrate ships here as part of DOS-339 wiring, NOT in W1
- `render_rules.rs` → deferred to v1.4.4 W4 (per-claim_type render rules — DOS-447 in dissolved archive) — substrate stub only in W1

**Producer placement (W1 sub-tasks under DOS-339):**
- Confirm shipped DOS-701 substrate matches L0 Addendum verbatim — re-verify at L0 plan via fresh `git diff` vs L0 Addendum DTO.
- Write the `useClaimReceiptSubscription` hook in TS that W2/W3/W4 blocks consume.
- Codify the producer→projection→renderer contract for the receipt in `wp/dailyos/docs/CLAIM-RECEIPT-INTEGRATION.md` (Tier-3 markdown-only).

**Consumer surfaces:** all W2 entity-detail surfaces, W3 briefing surfaces, W4 actions/work + activity log + lint surfaces. THE shared receipt — no page-local receipts.

**Intelligence Loop integration check:**
1. *Claim model:* Receipt references claim/proposal records + preserves subject/field binding. No new claims.
2. *Provenance + trust:* Receipt fields come from rendered provenance, `source_asof`, field attribution, trust bands per ADR-0108/0105.
3. *Signals + invalidation:* Receipt state updates on claim lifecycle / feedback / contradiction / source state changes.
4. *Runtime + surfaces:* WP Gutenberg block renderers (W2–W5) are the v1.4.4 consumers; Tauri React equivalents stay in stasis (Tauri freeze) but the receipt is surface-agnostic per ADR-0130.
5. *Feedback loop:* Receipt exposes semantic actions via DOS-8 (§5.7) + reconciliation via the contradiction substrate (W4 deferred).

**Migration slots:** None for W1 (v240 already taken by DOS-701).

**Acceptance criteria (additive to shipped DOS-701):**
- **AC-339.1** — TypeScript hook `useClaimReceiptSubscription(target, surface)` exists; subscribes to claim-lifecycle signals; re-renders on signal. The Tauri command behind it threads `&AppState` through to the shipped `render_receipt_for(&AppState, target, surface)`.
- **AC-339.2** — At least one W2 block consumer wires the receipt end-to-end as the W1 proof (chosen at W2 L0; recommend Account Detail per DOS-462 pattern).
- **AC-339.3** — Producer→projection→renderer contract documented at `wp/dailyos/docs/CLAIM-RECEIPT-INTEGRATION.md`.
- **AC-339.4** — No component writes claim / feedback / lifecycle state directly; all mutations route through `services/`.
- **AC-339.5** — Reuse audit names owner for each behavior: service / ability / DTO / hook / primitive (per CLAUDE.md DoD #7).
- **AC-339.6** — (cycle-1 architecture F4 — multi-surface fan-out) Receipt re-emission signal named explicitly: `ClaimVerificationStateChanged` (existing per ADR-0080 substrate; verify at L1). Coalesce policy on `useClaimReceiptSubscription`: **250ms trailing-edge debounce per `(target.claim_id, surface)` pair**. Test: feedback action on claim C from Actions/Work re-emits receipt to all subscribed Entity Detail / Briefing / Activity Log surfaces within 1 invalidation cycle; coalesce window prevents > N re-emits per (claim, surface) per second under burst feedback (e.g., user marks 5 stale claims in rapid succession).

---

### §5.7 — DOS-8: Semantic claim feedback actions (typed `claim_feedback`)

**Contract shape:**

```rust
// Fills DOS-701 placeholder: src-tauri/src/services/claim_receipt/feedback.rs

pub use abilities_runtime::abilities::feedback::FeedbackAction; // 9 variants per ADR-0123

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFeedbackRequest {
  pub target: FeedbackTarget,                  // Claim { id } | Proposal { id }
  pub action: FeedbackAction,
  pub surface: SurfaceContext,                 // existing W1-RECEIPT enum
  pub metadata: Option<serde_json::Value>,     // per-action JSON Schema validated; ≤4KB; ≤4 nesting; ≤24 keys; deny unknown
  // idempotency_key is SERVER-ISSUED — not caller-controlled
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFeedbackResponse {
  pub idempotency_key: ServerIssuedKey,
  pub receipt: ClaimReceipt,                   // updated receipt for re-render
  pub lifecycle_changed: bool,
  pub repair_queued: bool,
  pub queue_changed: bool,                     // review queue membership delta
}

pub async fn submit_claim_feedback(
  state: &AppState,                            // per cycle-1 codex-consult F1 — shipped can_surface_for needs &AppState
  req: ClaimFeedbackRequest,
  actor: &RenderActor,                         // per cycle-1 CSO F1 — not the proposed `Actor`
  conn: &SqlConn,
) -> Result<ClaimFeedbackResponse, FeedbackError> {
  // 1. validate target via services::entity_intelligence::auth::validate_envelope_target(envelope_set, &req.target)
  //    (envelope_set = parent + transitively composed children per AC-477.13)
  // 2. authorize via services::claim_receipt::auth::can_surface_for(state, actor, render_surface, claim_id)
  //    (signature: &AppState, &RenderActor, RenderSurface, &str — per cycle-1 codex-consult F1)
  //    Actor::Agent collapses to deny everywhere in v1.4.4 (per AC-8.13).
  // 3. sanitize user-authored free-text fields (NeedsNuance.corrected_text, CannotVerify.note)
  //    through ADR-0108 §3 sanitizer pipeline (per AC-8.12); surface ProvenanceWarning::ExplanationFiltered.
  // 4. validate metadata per-action JSON schema (field names MATCH ADR-0123 §1 variants verbatim per AC-8.10)
  //    WrongSource uses stable source content hash per ADR-0131, NOT source_index (per AC-8.11)
  //    WrongSubject uses field `corrected_to` per ADR-0123 §1 (not `intended_subject_ref`)
  // 5. mint server-issued idempotency_key scoped per (claim_id, action, actor, metadata_hash); TTL ≤ 60s (per AC-8.14)
  // 6. route to services::claims::record_claim_feedback (existing at claims.rs:6700)
  // 7. fetch updated receipt via services::claim_receipt::render::render_receipt_for(state, target, surface)
  //    For Proposal/WorkItem targets, returns typed "no receipt yet" (Proposal-receipt deferred per §5.6).
  // 8. emit ClaimVerificationStateChanged signal (per AC-339.6 fan-out coalesce)
  // 9. return response (idempotency_key is response-only; caller-supplied keys rejected per AC-8.2)
}
```

**Symbol corrections from V1.0** (per cycle-1 CSO F1 + codex-consult F1/F2):
- `Actor` → `RenderActor` (from `abilities_runtime::sensitivity`)
- `SurfaceContext` (when crossing into sensitivity) → `RenderSurface` (sensitivity gate uses `RenderSurface`; `SurfaceContext` remains the receipt-side type for non-sensitivity-gate calls)
- `render_policy_for` → `render_policy_for_surface(claim, surface, actor)`
- Add `renderable_claim_text_with_value(...)` as the public composition primitive most W2 callers should consume.

**Reuses existing substrate:**
- `services::claims::record_claim_feedback` at `src-tauri/src/services/claims.rs:6700` — already 9-variant aware per `record_claim_feedback_persists_a_row_per_action_for_each_of_9_variants` test at `:13283`. NO change to the writer; W1 is a caller-wrapper.
- 9-variant enum at `abilities-runtime/src/abilities/feedback.rs:31`: `ConfirmCurrent | MarkOutdated | MarkFalse | WrongSubject | WrongSource | CannotVerify | NeedsNuance | SurfaceInappropriate | NotRelevantHere`.

**Consumer-semantics taxonomy table (sourced from substrate `apply_feedback_to_state` at `feedback.rs:222-372`):**

| Variant | User label (UI; ADR-0083) | Substrate `ClaimVerificationState` transition | Other substrate effect | Queue effect | Required metadata |
|---|---|---|---|---|---|
| ConfirmCurrent | "Still true" | → `Active` | `claim_feedback` row | Removes from queue if present | none |
| MarkOutdated | "Out of date" | → `Contested` | row + schedule refresh | (no change) | optional `last_known_true_at` |
| MarkFalse | "Incorrect" | → `Contested` | row + tombstone candidate | Adds if conflicting | optional `corrected_value` |
| WrongSubject | "About wrong account/person" | → `Contested` | row + subject reattribution candidate | Adds | required `corrected_to: Option<SubjectRef>` (per ADR-0123 §1; per cycle-1 CSO F5) |
| WrongSource | "Bad source" | → `Contested` | row + source-reliability decay | (no change) | required **stable source content hash** per ADR-0131 (NOT `source_index` or `source_ref`; per cycle-1 CSO F6 + AC-8.11) |
| CannotVerify | "Can't tell" | (no change unless terminal) | row only | (no change) | optional `note` ≤ 500 chars |
| NeedsNuance | "True but needs context" | → `Contested` | row + nuance attached as evidence | Adds | required `nuance_text` ≤ 500 chars |
| SurfaceInappropriate | "Don't show here (private)" | (no change) | row + render-policy denylist add for surface | (no change) | required `surface` |
| NotRelevantHere | "Wrong place" | (no change) | row + render-policy denylist add for surface | (no change) | required `surface` |

**Authorization matrix (per CSO requirements in dissolved archive class C5; cycle-1 CSO F8 tightened Agent rows to deny everywhere — see AC-8.13):**

| Surface | Actor: User | Actor: Agent (SurfaceClient) | Actor: McpClient (v1.4.7+) | Actor: System |
|---|---|---|---|---|
| ActionsWork | allow ≤ Confidential | **deny** (v1.4.4) | (v1.4.7 scope) | read-only |
| EntityDetail | allow ≤ Confidential | **deny** (v1.4.4) | (v1.4.7 scope) | read-only |
| DailyBriefing | allow ≤ Confidential | deny | (v1.4.7 scope) | read-only |
| MeetingDetail | allow ≤ Confidential | deny | (v1.4.7 scope) | read-only |
| Mcp | (v1.4.7 scope) | (v1.4.7 scope) | per v1.4.7 scope manifest | deny |

Agent broadening waits for v1.4.7 MCP scope manifest + explicit ADR-0123 amendment. Per ADR-0123 §8: "Feedback from non-User actors. … v1.4.0 commits user-only."

**Intelligence Loop integration check:**
1. *Claim model:* Feedback routes through `services::claims::record_claim_feedback` (existing); no direct DB.
2. *Provenance + trust:* Feedback updates lifecycle + verification state which feeds trust-band recompute via existing substrate.
3. *Signals + invalidation:* Successful feedback emits existing claim-lifecycle signals → consumers re-render via DOS-339 receipt subscription.
4. *Runtime + surfaces:* Consumed by W4 Actions/Work + W2 entity detail correct-claim affordances.
5. *Feedback loop:* THIS IS the feedback loop primitive. The Intelligence Loop's "feedback closes" guarantee is the contract this ticket lands.

**Migration slots:** None — uses existing typed `claim_feedback` table.

**Acceptance criteria:**
- **AC-8.1** — All 9 substrate `FeedbackAction` variants supported (no parallel enum).
- **AC-8.2** — Server-issued `idempotency_key` round-trip tested; caller-supplied keys rejected with `BadRequest::CallerSuppliedIdempotencyKey`.
- **AC-8.3** — Authorization matrix tested per row (positive + negative fixture per surface × actor × sensitivity).
- **AC-8.4** — Per-action metadata JSON schemas validated; oversized/unstructured metadata rejected.
- **AC-8.5** — Cannot-verify + preference-dismissal (SurfaceInappropriate/NotRelevantHere) semantics tested as distinct from wrong/outdated.
- **AC-8.6** — UI copy uses product vocabulary (ADR-0083); avoids pipeline vocabulary. UI may collapse the 9 substrate variants into a smaller user-visible primary set + overflow menu (e.g., "Wrong" maps to MarkFalse / WrongSubject / WrongSource based on context), but the **persisted action remains semantic** per ADR-0123. (§13 Q4 resolved.)
- **AC-8.7** — At least one end-to-end Actions/Work proof flow records feedback + updates rendered state (W4 W1-handoff acceptance).
- **AC-8.8** — `/cso` L0 + L2 approval recorded; `/plan-devex-review` L0 approval recorded.
- **AC-8.9** — (cycle-1 CSO F4 + correctness F8) User-authored free-text fields in `ClaimFeedbackRequest.metadata` are persisted with `ClaimSensitivity::Confidential` by default (NOT `Internal`), unless the originating claim's sensitivity is higher (inherit). Render policy for surface unchanged — `Confidential` is hidden from MCP / log structured by default, click-to-reveal in Tauri. Document the default in ADR-0123 amendment (or v1.4.4 amendment-bundle). Retry-without-prior-key semantics tested: dropped-response retry produces at most one persisted feedback row per (claim, action, actor, timestamp window) — per `services::claims::record_claim_feedback` existing idempotency contract over `(claim_id, action, actor)` per `claims.rs:13283` test.
- **AC-8.10** — (cycle-1 CSO F5) Per-action metadata JSON schema MUST match ADR-0123 §1 variant payload field names **verbatim**. Specifically: `WrongSubject` carries `corrected_to: Option<SubjectRef>` (NOT `intended_subject_ref`); `WrongSource` carries the source-content-hash field per AC-8.11 (NOT `source_index`); `NeedsNuance` carries `corrected_text: String`; `SurfaceInappropriate` carries `surface: SurfaceId`; `NotRelevantHere` carries `invocation_id: InvocationId`. Schema test asserts JSON-schema-emitted field set == ADR-0123 §1 variant field set (golden parity test, similar to TS mirror parity).
- **AC-8.11** — (cycle-1 CSO F6) `WrongSource` metadata MUST carry a stable source content hash (per ADR-0131 structured embedding canonicalization), NOT an index. Server validates the hash against the **current** claim's source set at feedback-apply time. If the source no longer exists in the set, the feedback is rejected with `BadRequest::SourceNoLongerInClaim` (caller is asked to re-render + resubmit). Optionally retain `source_index` as a presentation hint but never as the authoritative identifier. Adversarial fixture: `feedback::tests::wrong_source_index_aliasing_race` — race a source revocation against a `WrongSource` submission; assert no source-reliability decay applied to innocent successor source.
- **AC-8.12** — (cycle-1 CSO F7) User-authored `corrected_text` (NeedsNuance), `note` (CannotVerify), and any other free-text field in `ClaimFeedbackRequest.metadata` passes through the ADR-0108 §3 sanitizer pipeline BEFORE persistence. Sanitizer instance is shared with the existing `FieldAttribution.explanation` path. Sanitization warnings surface in the response (e.g., `ProvenanceWarning::ExplanationFiltered`) so the user knows their text was modified. CI lint forbids any persistence path for user-authored claim-adjacent text that bypasses `abilities_runtime::sanitizer::*`.
- **AC-8.13** — (cycle-1 CSO F8) In v1.4.4 W1, `actor: Actor::Agent` is **denied** at all surfaces (NOT allowed-up-to-Internal as V1.0 stated). The matrix row collapses to deny everywhere until an explicit ADR-0123 amendment + v1.4.7 MCP scope manifest expands it. Tighten in §5.7 substrate; broadening waits for v1.4.7 path. Authorization matrix table-test in `claim_receipt::feedback::tests::authorization_matrix` — for every `(surface × actor × sensitivity)` cell, assert the matrix outcome.
- **AC-8.14** — (cycle-1 CSO F9) Server-issued idempotency key scoped per `(claim_id, action, actor, metadata_hash)`. TTL ≤ 60 seconds. Outside the window, retries are treated as new submissions (since two genuinely-distinct feedbacks at >60s separation reflect distinct user intent). Test: `feedback::tests::idempotency_window_boundary` — submit two distinct feedbacks on same claim with same metadata; second succeeds if outside 60s.

---

### §5.8 — DOS-340: Provenance receipt vs operational audit boundary

**Contract shape:**

```rust
// Fills DOS-701 placeholder: src-tauri/src/services/claim_receipt/boundary.rs

pub const RECEIPT_ALLOWED_FIELDS: &[&str] = &[
  "source_label",
  "source_type",
  "source_asof",
  "trust_band",
  "freshness",
  "field_path",
  "evidence_summary",     // redacted per surface
  "lifecycle_state",
  "verification_state",
];

pub const AUDIT_ONLY_DENYLIST: &[&str] = &[
  "raw_source_id",
  "prompt_hash",
  "internal_audit_id",
  "local_file_path",
  "private_url",
  "private_email",
  "private_snippet",
  "inaccessible_doc_title",
  "raw_command_metadata",
  "raw_invocation_metadata",
  "debug_payload",
  "raw_model_input",
  "raw_model_output",
  "raw_tool_input",
  "raw_tool_output",
  "raw_document_body",
  "raw_message_body",
  "correlation_id",       // when surfaced outside operational audit viewer
];

pub fn filter_for_receipt(raw: serde_json::Value) -> serde_json::Value { … }
pub fn filter_for_activity_log(raw: serde_json::Value) -> serde_json::Value { … }
pub fn filter_for_lint(raw: serde_json::Value) -> serde_json::Value { … }

pub enum SurfaceKind { Receipt, ActivityLog, Lint, OperationalAudit }
```

**CI lint script:** `src-tauri/scripts/check_audit_disclosure_allowlist.sh` modeled on existing `check_claim_writer_allowlist.sh` — forbids `audit_log` table reads outside an allowlisted set of audit-management commands. Scans `services/` AND `commands/`:
- Allowed readers: `commands/audit_management*` ONLY.
- Forbidden readers: `services/activity_log_projection.rs`, `services/lint_projection.rs`, `commands/activity*`, `commands/lint*`, `services/claim_receipt/*`, `services/entity_intelligence/*`, all W2–W5 block render PHP entry points.

**Producer placement:** `services::claim_receipt::boundary` (Rust runtime filter); `scripts/check_audit_disclosure_allowlist.sh` (CI gate).

**Consumer surfaces:** All W2–W5 surfaces consume the filter. The Activity Log + Lint surfaces (W4) consume the four product buckets (`fixed_automatically` / `watching` / `needs_evidence` / `needs_user_decision`) instead of raw audit rows.

**Intelligence Loop integration check:**
1. *Claim model:* No claims written; boundary enforcement only.
2. *Provenance + trust:* Boundary preserves provenance trust by preventing receipts leaking audit-only fields that would let users mistake audit identifiers for evidence.
3. *Signals + invalidation:* No new signals.
4. *Runtime + surfaces:* Consumed by every receipt-rendering path (DOS-339 §5.6) + Activity Log / Lint (W4).
5. *Feedback loop:* (per cycle-1 codex-challenge F3) Boundary violations detected by the §5.8 CI lint OR by user reports of leaked audit-only fields feed back as DOS-8 `WrongSource` / `SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract. A surface that accidentally exposes audit-only fields will surface user feedback that loops back into source-reliability decay through the existing DOS-8 path.

**Migration slots:** None.

**Acceptance criteria:**
- **AC-340.1** — Receipt allowlist + audit-only denylist encoded in `boundary.rs` constants + enforced in `filter_for_receipt`/`filter_for_activity_log`/`filter_for_lint`.
- **AC-340.2** — CI lint script `check_audit_disclosure_allowlist.sh` committed + green; rejects `audit_log` table reads outside allowlist.
- **AC-340.3** — Suite S test for every audit-only field per L0 Addendum — MUST NOT appear in any receipt rendering snapshot.
- **AC-340.4** — Activity/Lint product buckets land as service-side typed projections (precedence rules from claim lifecycle/verification/contradiction/repair states); raw audit reads forbidden.
- **AC-340.5** — Receipt-vs-Activity-vs-Audit taxonomy documented in proof bundle with example payloads using generic data.
- **AC-340.6** — `/cso` L0 + L2 approval recorded.
- **AC-340.7** — (cycle-1 correctness F9) CI lint `check_audit_denylist_completeness.sh` enforces that every column added to the operational audit storage schema (any migration touching `audit_log` or `provenance_audit_storage`) explicitly adds an entry to `AUDIT_ONLY_DENYLIST` OR includes a comment justifying receipt-safety (`// receipt-safe: <reason>`). Pattern modeled on existing `check_claim_writer_allowlist.sh`. Pairs with AC-477.12 allowlist-primary boundary.

---

### §5.9 — DOS-341: Claim receipt privacy + redaction rules

**Contract shape (allowlist-primary build, NOT post-render transform — per cycle-1 CSO F10):**

```rust
// Fills DOS-701 placeholder: src-tauri/src/services/claim_receipt/privacy.rs

#[derive(Debug, Clone)]
pub enum PrivacyAudience { UserTauri, AgentMcp, ActivityLog, Lint, OperationalAuditStorage }

/// Per-audience field allowlist consts (one per audience).
/// New field on ClaimReceipt forces explicit audience-allowlist amendment.
pub const USER_TAURI_ALLOWED_FIELDS: &[&str] = &[ /* trust band, freshness, source label, ... */ ];
pub const AGENT_MCP_ALLOWED_FIELDS: &[&str] = &[
  "trust_band",
  "freshness",            // coarsened — Current/Aging/Stale only
  "redaction_level",
  "lifecycle_state",
  "evidence_summary",     // sanitized per ADR-0108 §3
  "subject_type",         // NOT subject_id
  // FORBIDDEN: source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak)
];
pub const ACTIVITY_LOG_ALLOWED_FIELDS: &[&str] = &[ /* … */ ];
pub const LINT_ALLOWED_FIELDS: &[&str] = &[ /* … */ ];
// OperationalAuditStorage = non-disclosure tag; not a render target.

/// Render-time primitive. Constructs ONLY the allowlisted fields per audience.
/// Audience is INPUT to construction, NOT a filter on output.
/// Same target + audience → byte-identical receipt.
pub fn build_receipt_for_audience(
  target: ReceiptTarget,
  audience: PrivacyAudience,
  conn: &SqlConn,
) -> Result<ClaimReceipt, BuildError> {
  // Composes existing abilities_runtime::sensitivity::render_policy_for_surface + renderable_claim_text_with_value (ADR-0108).
  // Does NOT define a second redaction system.
  // Runs INSIDE render_receipt_for (per cycle-1 CSO F10).
}
```

**Privacy matrix (per L0 Addendum from dissolved archive):**

| Audience | May show | Must not show |
|---|---|---|
| Product receipt — UserTauri (Tauri / WP block render — both are local-to-local, same trust boundary per ADR-0129) | generic source label, source type, source-as-of, trust band, field/topic path, redacted evidence summary, lifecycle state | raw source id, prompt hash, internal audit id, local file path, private URL, full private email, full private snippet, inaccessible doc title, raw command/invocation metadata |
| **AgentMcp (v1.4.7 consumer; explicit row per cycle-1 CSO F12)** | **trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary, subject_type** | **source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak), subject_id, raw source ids, prompt hashes, audit ids, file paths, private URLs/emails/snippets, command/invocation metadata** |
| Activity (W4 Activity Log) | user-readable event, subject label, bucket, timestamp/source-as-of, surface, link to receipt/queue item | raw audit rows, command invocation metadata, prompt hashes, debug payloads |
| Lint (W4 Lint Mode) | finding type, subject label, bucket, severity, link to receipt/queue item | raw audit rows, runtime evaluator output, prompt hashes, debug payloads |
| Operational audit (storage) | correlation ids, raw source ids, prompt/invocation metadata, tamper-evident record ids, debug payloads, raw model/tool i/o, raw document/message bodies (per retention policy) | (no surface — storage only; non-disclosure tag enforced via AC-341.11 lint extension) |

**Producer placement:** `services::claim_receipt::privacy` — render-time helper invoked by `render_receipt_for` (DOS-339 substrate). Reuses `abilities_runtime::sensitivity` ADR-0108 primitives — NOT a parallel redaction system.

**Consumer surfaces:** All receipt-rendering paths (every W2–W5 surface). MCP rendered outputs (v1.4.7 path) also consume.

**Intelligence Loop integration check:**
1. *Claim model:* Sensitivity belongs to claim/provenance state + travels with the receipt.
2. *Provenance + trust:* Redaction preserves trust + currentness while withholding sensitive source details. Users can still see "this claim is `likely_current`, source-asof 2 hours ago" even when source identity is redacted.
3. *Signals + invalidation:* Revoked / inaccessible sources update rendered receipt state via existing source-state signals; privacy policy re-applies.
4. *Runtime + surfaces:* Render policy varies by Tauri / WP block / MCP / Activity / Lint / audit storage.
5. *Feedback loop:* Users can correct + contest redacted claims without seeing private source internals; corrections feed source reliability via DOS-8 (§5.7).

**Migration slots:** None.

**Acceptance criteria:**
- **AC-341.1** — Receipt rendering uses explicit field allowlist by audience/surface (encoded in `privacy.rs`).
- **AC-341.2** — Redaction covers confidential, user-only, revoked source, inaccessible source, raw prompt metadata, internal ids, private URLs, email addresses, file paths, private snippets.
- **AC-341.3** — Redacted receipts remain useful — show source-type / currentness / trust band where allowed without leaking private details.
- **AC-341.4** — Tauri AND WP block render AND MCP render paths produce policy-compliant output (parity-tested via snapshot fixtures).
- **AC-341.5** — Activity / Lint surfaces (W4) do not expose operational audit-only fields (cross-referenced with DOS-340 boundary CI lint §5.8).
- **AC-341.6** — Tests use generic / fake data only.
- **AC-341.7** — A redacted Actions/Work proof state included in W4 evidence (cross-ref DOS-514).
- **AC-341.8** — No raw audit-only field appears in shared receipt DTO snapshots (snapshot test).
- **AC-341.9** — `/cso` L0 + L2 approval recorded.
- **AC-341.10** — (cycle-1 CSO F10 + correctness F5 — allowlist-primary + derived-claim handling) `build_receipt_for_audience` is a render-time primitive (NOT a post-render transform); audience is an input to construction. Same target + audience → byte-identical receipt. No post-render mutation; receipts are immutable. CI lint asserts JSON key set per audience is a subset of the audience-specific allowlist. Plus derived/composed-claim handling:
  - **Mixed-sensitivity composed claims:** sensitivity = max(inputs); if max exceeds surface, **drop the entire composed claim** (no partial render — prevents inference leaks).
  - **Cross-claim references:** if referenced claim's sensitivity exceeds surface, render as `redacted: true, label: "<source type> (redacted)"`, NEVER as opaque ID.
  - **Derived claims:** carry explicit `derived_from: Vec<ClaimId>`; surface check is **max sensitivity across the derivation chain**.
  - Fixtures: `account_health_story_derived_from_confidential.json` + per-(audience × sensitivity × claim_type) golden JSON files.
- **AC-341.11** — (cycle-1 CSO F11 — OperationalAuditStorage enforcement) `services::claim_receipt::privacy::OperationalAuditStorage` audience is a **non-disclosure tag, not a render target**. Extend `check_audit_disclosure_allowlist.sh` (DOS-340 §5.8) to forbid any direct read of `maintenance_audit` table rows from `services::claim_receipt::*` (the privacy module is allowed to **write** assertions but never to **surface** raw rows). ANY call site that touches `maintenance_audit` rows must be either (a) audit-management commands allowlisted in DOS-340 §5.8 lint, OR (b) routed through `build_receipt_for_audience` with a non-OperationalAuditStorage audience. Negative-fixture test: a deliberately-attempted disclosure path that the lint catches.
- **AC-341.12** — (cycle-1 CSO F12 — AgentMcp audience explicit row) Privacy matrix explicit `AgentMcp` row added. Field allowlist for AgentMcp: trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary (sanitizer per ADR-0108 §3), `subject_type` (NOT `subject_id`). Forbidden: source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak). AgentMcp snapshot fixtures in `claim_receipt/privacy/tests/agent_mcp_audience` — golden files per sensitivity. CI lint catches any field bleed.

---

### §5.10 — DOS-507: `get_daily_briefing` Read/User-only ability contract + proof gate

**Contract shape:**

```rust
// New ability at: src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs

#[ability(
  name = "get_daily_briefing",
  category = Read,
  version = "0.1.0",
  schema_version = 1,
  allowed_actors = [User],            // User-only per ticket L0 decision
  allowed_modes = [Live, Simulate, Evaluate],
  requires_confirmation = false,
  may_publish = false,
  required_scopes = ["read.daily_briefing"],
  mcp_exposure = NotExposed,          // per ticket: cannot become Agent/MCP-visible without CSO re-approval
  composes = [
    { id = "get_entity_intelligence", ability = "get_entity_intelligence", optional = false },  // §5.1
    { id = "get_daily_readiness", ability = "get_daily_readiness", optional = false }           // existing
  ],
  experimental = false,
  signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_daily_briefing(
  ctx: &AbilityContext<'_>,
  input: DailyBriefingInput,
) -> AbilityResult<DailyBriefingOutput> { … }

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DailyBriefingInput {
  pub schema_version: u32,                       // = 1
  pub date: chrono::NaiveDate,                   // briefing date (default: today in user's tz)
  pub sections: Option<Vec<BriefingSection>>,    // None = all
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DailyBriefingOutput {
  pub schema_version: u32,
  pub date: chrono::NaiveDate,
  pub state: BriefingState,                      // Full | Empty | AuthLocked | Stale | NeedsPreparation | NeedsVerification | Corrected | Ambiguous
  pub current_meeting: Option<MeetingBriefRef>,
  pub next_meeting: Option<MeetingBriefRef>,
  pub upcoming_meetings: Paginated<MeetingBriefRef>,    // per cycle-1 architecture F5 + AC-507.10
  pub candidate_set: CandidateSetRef,            // typed basis (DOS-460 primitive)
  pub watch_proposals: Vec<WatchProposal>,       // proposal-only — no synthesis
  pub trust_summary: BriefingTrustSummary,
  pub provenance: EnvelopeProvenance,            // display-safe
  pub sensitivity: ClaimSensitivity,
  pub source_asof_inputs: Vec<SourceAsofRef>,    // for freshness display
}

/// Composed struct (NOT flat enum) per cycle-1 correctness F3 — real briefings have
/// multi-dimensional state ("full" + "has-stale-references" + "needs-prep for one of N meetings"
/// is common). Flat enum forces a precedence that doesn't exist.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BriefingState {
  pub availability: BriefingAvailability,         // overall presence
  pub freshness: BriefingFreshness,               // freshness posture (may be Fresh even if integrity has corrections)
  pub integrity: BriefingIntegrity,               // claim-store integrity (corrections, ambiguity)
  pub advisories: Vec<BriefingAdvisory>,          // typed non-blocking notices
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BriefingAvailability {
  Available,
  Empty { reason: EmptyReason },
  AuthLocked,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BriefingFreshness {
  Fresh,
  Stale { reason: StaleReason },
  NeedsPreparation { meeting_ids: Vec<String> },
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BriefingIntegrity {
  Clean,
  HasCorrections { superseded_claim_ids: Vec<String> },
  HasAmbiguity { ambiguous_pairs: Vec<AmbiguityPair> },
}
```

**Producer placement:**
- `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/{mod,synthesis}.rs` — Read ability, no provider synthesis (per ticket "no provider-backed synthesis during read composition"; "if fresh prep is missing, return a typed empty/needs-preparation state rather than triggering synthesis").
- Composes existing `prepare_meeting` (already shipped at `abilities-runtime/src/abilities/prepare_meeting/mod.rs`) + new `get_entity_intelligence` (§5.1) + existing `get_daily_readiness` (already shipped).
- Consumes meeting prep status DTO from §5.5 (DOS-335) to determine BriefingState transitions.

**Consumer surfaces:**
- W3 Daily Briefing Gutenberg block (the v1.4.4 W3 surface — explicit single block).
- NOT exposed to MCP or Agent actors in v1.4.4; Agent/MCP exposure requires `/cso` re-approval per ticket.

**Intelligence Loop integration check:**
1. *Claim model:* No claim production; pure read composition over existing claim/proposal/prep records.
2. *Provenance + trust:* `BriefingTrustSummary` aggregates per-meeting prep trust + per-claim trust; `source_asof_inputs` carries freshness inputs.
3. *Signals + invalidation:* Re-renders on prep-status signals (DOS-335 §5.5) + claim-lifecycle signals + candidate-set invalidation (DOS-278 path).
4. *Runtime + surfaces:* W3 Daily Briefing block only; User actor only.
5. *Feedback loop:* User feedback on briefing-displayed claims routes through DOS-8 (§5.7); briefing re-fetches reflect new state.

**Migration slots:** None — pure read composition.

**Acceptance criteria:**
- **AC-507.1** — Ability registered at registry with schema_version=1 + Read category + User-only actors + `NotExposed` MCP exposure.
- **AC-507.2** — No provider-backed synthesis during read composition; no calls that create fresh meeting prep or invoke provider-backed `prepare_meeting`.
- **AC-507.3** — If fresh prep is missing, returns `BriefingState { freshness: NeedsPreparation { meeting_ids: [...] }, ... }` (NOT triggers synthesis). Read-ability contract: briefing does NOT auto-enqueue prep; the W3 block renderer (or a sibling write-ability `enqueue_briefing_prep`) handles the enqueue separately. (§13 Q9 resolved.)
- **AC-507.4** — (cycle-1 correctness F3) Output fixture covers the **composed-state matrix**, NOT a single flat-enum value per fixture. Each fixture asserts the 4-tuple `(availability, freshness, integrity, advisories)` independently:
  - `Available + Fresh + Clean + []` (full happy path)
  - `Empty { reason: NoMeetings } + Fresh + Clean + []`
  - `AuthLocked + _ + _ + []`
  - `Available + NeedsPreparation { meeting_ids: [m2] } + Clean + []` (the common partial state — was unrepresentable in V1.0 flat enum)
  - `Available + Stale { reason: SourceAsofOlderThanThreshold } + Clean + []`
  - `Available + Fresh + HasCorrections { superseded_claim_ids: [...] } + []`
  - `Available + Fresh + HasAmbiguity { ambiguous_pairs: [...] } + []`
  - `Available + Stale + HasCorrections + [advisory: WatchProposal]` (multi-dimensional realistic state)
- **AC-507.5** — Cannot become Agent/MCP-visible in v1.4.4 without `/cso` re-approval (ability metadata enforces).
- **AC-507.6** — Redaction policy for sensitive fields applied per DOS-341 §5.9 privacy matrix.
- **AC-507.7** — `no-write` guarantees tested — `cargo test` asserts no writes through the ability call path.
- **AC-507.8** — Registry/schema test from the contract.
- **AC-507.9** — `/cso` L0 + L2 approval recorded; `/plan-devex-review` L0 approval recorded.
- **AC-507.10** — (cycle-1 architecture F5) `upcoming_meetings: Paginated<MeetingBriefRef>` (NOT plain `Vec`). Consumer pagination tested via re-invocation; cursor is opaque server-signed; CursorState policy (`Stable | DataShifted | Invalidated`) per AC-459.9.

---

## 6. Substrate consumed (existing primitives W1 builds on, not reinvents)

| Substrate | Shipped in | Where W1 consumes |
|---|---|---|
| `services::claim_receipt::{contracts, render, auth}` (319+290+175 LOC) + migration v240 | DOS-701 (PR #323, 2026-05-19) | §5.6 DOS-339 wiring; §5.7 DOS-8 feedback; §5.8 DOS-340 boundary; §5.9 DOS-341 privacy |
| `services::claims::record_claim_feedback` at `claims.rs:6700` (9-variant aware) | v1.4.0/1.4.1 substrate | §5.7 DOS-8 caller-wrapper |
| `services::claims::reconcile_contradiction` at `claims.rs:8906` | v1.4.0/1.4.1 substrate | §5.6 DOS-339 future contradiction substrate (deferred to v1.4.4 W4) |
| `abilities-runtime::abilities::feedback::FeedbackAction` enum (9 variants) | ADR-0123 substrate | §5.7 DOS-8 re-exports verbatim |
| `abilities-runtime::abilities::get_entity_context` | v1.4.1 substrate | §5.1 DOS-459 composes |
| `abilities-runtime::abilities::list_open_loops` + `services::context::ListOpenLoopsQuery` | v1.4.1 substrate | §5.2 DOS-460 extends consumer contract |
| `abilities-runtime::abilities::prepare_meeting` | v1.4.x substrate | §5.10 DOS-507 composes |
| `abilities-runtime::abilities::get_daily_readiness` | v1.4.x substrate | §5.10 DOS-507 composes |
| `abilities-runtime::sensitivity::{renderable_claim_text_with_value, RenderableClaimText, ClaimVerificationState}` | ADR-0108 + v1.4.1 substrate | §5.6 DOS-339; §5.9 DOS-341 redaction primitives |
| `abilities-runtime::types::{ClaimState, SurfacingState, ClaimSensitivity}` | ADR-0125 + v1.4.0 substrate | §5.1 envelope, §5.5 prep status, §5.6 receipt, §5.9 privacy |
| `abilities-runtime::abilities::trust::types::TrustBand` | ADR-0105 substrate | §5.1, §5.5, §5.6, §5.10 |
| `abilities-runtime::abilities::provenance::subject::SubjectRef` | ADR-0125 substrate | §5.1, §5.2, §5.5, §5.6 |
| v1.4.3 W1 chrome lane primitives: TrustBandBadge, EntityChip, ProvenanceTag, FreshnessIndicator, Pill | v1.4.3 W1 + W3 chrome lane (PR #315 + DOS-721/729/730/731) | W2 block renderers will project envelope/receipt fields through these primitives (per ADR-0130) |
| v1.4.3 W3 magazine theme + FolioBar runtime-injected chrome | v1.4.3 W3 (DOS-698 + chrome lane) | §5.5 DOS-335 prep status DTO surfaces in FolioBar at W3 surface ship |
| ADR-0130 producer→projection→renderer composition contract | ADR-0130 | Every sub-ticket §5.1–§5.10 respects the split |
| ADR-0129 composable surfaces + WP Studio as primary surface | ADR-0129 | W1 producers are surface-agnostic; W2–W5 WP block consumers prove the ADR |

**Reinvention rule:** if a primitive in this table covers a need, W1 consumes it. New substrate is introduced ONLY in §5.1 (envelope ability), §5.5 (prep status service), §5.10 (daily briefing ability) — and each of those is justified at the L0 reuse audit by the absence of an existing producer.

## 7. Acceptance criteria — wave-rolled-up

Vertical-slice requirement (CLAUDE.md "wiring IS the work"):

- **AC-W1.1** — No W2 surface ships without its W1 producer landed on `dev`. The dependency map is (per cycle-1 codex-challenge F4 — DOS-461 harness gates Account / Project / Person equally):
  - W2 Account Detail block → DOS-459 + DOS-460 + DOS-477 + DOS-339 + DOS-340 + DOS-341 + DOS-461 harness green
  - W2 Project Detail block → same set + DOS-461 harness green
  - W2 Person Detail block → same set + DOS-461 harness green
  - W3 Daily Briefing block → DOS-507 + DOS-335 + DOS-339 + DOS-340 + DOS-341
  - W3 Meeting Briefing block → DOS-335 + DOS-339 + DOS-341
  - W4 Actions/Work block → DOS-8 + DOS-339 + DOS-340 + DOS-341
  - W4 Activity Log block → DOS-340 + DOS-341 (boundary + privacy)
  - W4 Lint Mode → DOS-340 + DOS-341
- **AC-W1.2** — No W1 producer ships without at least one downstream consumer skeleton in W2 (or W3/W4 if W2 is not the natural consumer). Each W1 PR must point to the W2/W3/W4 PR that will consume it at the W2+ wave. "Skeleton" = a stubbed block render PHP file that calls the producer and renders a minimal projection — proves the contract carries weight.
- **AC-W1.3** — Intelligence Loop integration check verified per sub-ticket — every §5.x section answers all 5 questions (claim model / provenance + trust / signals + invalidation / runtime + surfaces / feedback loop). Substrate that fails any question is incomplete per CLAUDE.md Critical Rule.
- **AC-W1.4** — `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit && pnpm test` green.
- **AC-W1.5** — `/cso` L2 approval recorded for DOS-477, DOS-340, DOS-341, DOS-8, DOS-335 (sensitivity on user-authored notes), DOS-507 (User-only actor enforcement).
- **AC-W1.6** — `/plan-devex-review` L0 approval recorded for DOS-339 (shared DTO is the consumer-facing API), DOS-8 (feedback action taxonomy is consumer-facing API), DOS-507 (briefing ability is consumer-facing API).
- **AC-W1.7** — Mandatory retro at W1 close; `/ce-compound` runs on K-out candidates surfaced during the wave.
- **AC-W1.8** — Proof bundle at `.docs/plans/v1.4.4-wp-surface-migration/proof-bundles/W1-substrate-gaps.md` with reuse audit + Intelligence Loop check transcripts per sub-ticket.
- **AC-W1.9** — (cycle-1 codex-challenge F1 — consumer-skeleton CI gate) `scripts/check_w1_consumer_skeleton.sh` (modeled on `check_claim_writer_allowlist.sh`): for each W1-shipped producer (`abilities-runtime/src/abilities/get_entity_intelligence/`, `get_daily_briefing/`, `services/meeting_prep_status/{read,write}.rs`, `services/entity_intelligence/touchpoints.rs`, filled placeholders in `services/claim_receipt/{boundary,contradiction,feedback,privacy,render_rules}.rs`), assert at least one block render PHP entry point under `wp/dailyos/blocks/**/render-functions.php` invokes it via the abilities runtime handle. Substrate-only PRs without at least one downstream consumer reference fail CI. The script IS the mechanical enforcement for AC-W1.2's "wiring IS the work" obligation.
- **AC-W1.10** — (cycle-1 correctness G2) TS mirror parity test template: `src/services/claim-receipt/contracts.ts` is the canonical template; each sub-ticket's TS mirror cites this pattern rather than inventing its own.
- **AC-W1.11** — (cycle-1 correctness G3) Trust-band recompute path under feedback (DOS-8) explicitly cited at L1: name the substrate path (likely `services::trust::*` or signals on `verification_state` change) before wiring `record_claim_feedback`, so receipts don't go stale-by-one-feedback.

## 8. Out of scope (explicit deferrals)

| Out of scope | Why deferred | Where it goes |
|---|---|---|
| W2 entity-detail block rendering (Account/Project/Person Detail Gutenberg block render PHP + interaction JS) | Surface concern, not substrate | v1.4.4 W2 L0 packet |
| W3 briefing block rendering (Daily Briefing + Meeting Briefing Gutenberg block render PHP) | Surface concern | v1.4.4 W3 L0 packet |
| W4 Actions/Work + Activity Log + Lint Mode + Action Detail block rendering | Surface concern | v1.4.4 W4 L0 packet |
| W5 history + email + settings surfaces | Surface concern | v1.4.4 W5 L0 packet |
| Salience scoring + RecommendationClaim variant | Different version's mission | v1.4.6 — Salience & Recommendations |
| Workspace memory ingestion (local files, AI work product, inbox sources auto-feeding the graph) | Different version's mission | v1.4.5 — Workspace Memory Refactor |
| Report substrate (typed report ability outputs, source contract, generation lifecycle) | Different version's mission | v1.4.8 — Reports as Shareable Intelligence |
| `claim_receipt::contradiction.rs` + `claim_receipt::render_rules.rs` filler implementation | W4 Action surfaces own; W1 leaves placeholders intact | v1.4.4 W4 (contradiction UX = DOS-318 path; render rules = DOS-447 path from dissolved archive) |
| MCP exposure of `get_entity_intelligence` and/or `get_daily_briefing` to Agent/MCP actors | Trust-boundary gated | v1.4.7 — MCP Server v2 (Abilities-First); `/cso` re-approval required |
| Causal lineage between claims ("X caused Y" claim-to-claim references) | Recommendations foundation | v1.5.x (per memory `project_causal_lineage_deferred`) |
| Tauri React re-skin of Actions/Work + entity detail to consume the new envelope/receipt directly | Tauri UI freeze (per memory `feedback_tauri_ui_freeze`) | Out of scope until Tauri-freeze-lift release |

## 9. Migration slots

**Slot block claimed: v240–v249** (10 slots; DOS-701 took v240; W1 needs ~5 more in practice).

Per CLAUDE.md "Parallel-wave migration slot reservations" + memory `feedback_l0_reconcile_against_dev`: W1 declares its block upfront. **Cross-version coordination table on `.docs/plans/v1.4.6-waves.md` §327 + §414** explicitly states "v1.4.4 holds v240–v249" — V1.0's v250–v269 claim collided with v1.4.6's v260–v279 block (cycle-1 architecture F1). V1.1 reclaims v240–v249 per the coordination table; v1.4.6 §327 stays canonical.

Last shipped slot at scan time (2026-05-20): v240 (DOS-701 `claim_review_deferrals.sql`).

| Slot | Sub-ticket | Migration purpose |
|---|---|---|
| v240 | DOS-701 (Done) | `claim_review_deferrals` (shipped at PR #323, 2026-05-19) |
| v241 | DOS-335 §5.5 | `meeting_prep_status_signals` indexed view materializing meetings + meeting_entity_links + meeting_prep_outputs + claim_invalidation_queue rows for fast status compute |
| v242 | DOS-335 §5.5 | `meeting_prep_status_dismissals` (UserSuppressed/UserDismissed persistence; index on `(meeting_id, actor, created_at)`) |
| v243–v249 | (buffer) | Reserved for W1 L1 discovery — any sub-ticket that lands a migration during implementation claims the next slot in this block via the wave coordinator (single-issue claim, no overlap). 7 buffer slots — sufficient for W1 needs. |

**Coordination rule:** any sub-ticket that needs a new slot during L1 implementation claims via a PR comment on this L0 packet's GitHub thread (or via the v1.4.4 wave-coordinator Linear comment) BEFORE writing the migration filename. First-come, first-serve within the v243–v249 buffer. If 7 buffer slots is insufficient, file a coordinated amendment to BOTH this packet AND `.docs/plans/v1.4.6-waves.md` §327 + §414 reserving a non-overlapping block (e.g., v280–v289, the next clean window past v1.4.6's v279) before claiming.

**Other in-flight wave plans:** v1.4.3 W4-F shipped at v180 (`local_to_local_read_path.sql`); v1.4.5 holds v200–v219; v1.4.7 holds v220–v239; v1.4.4 holds v240–v249; v1.4.6 holds v260–v279 (per v1.4.6-waves.md §327 cross-version coordination table). No v1.4.4 W0/W2/W3/W4/W5/W6 plans authored yet — W1 claims first within v240–v249.

## 10. Architecture invariants

1. **All mutations route through `services/`** (CLAUDE.md Critical Rule). No direct DB writes from command handlers, abilities, block render PHP, or UI. `services::claims::record_claim_feedback` + `services::claims::reconcile_contradiction` are the only writers for claim_feedback + contradiction reconciliation. `services::meeting_prep_status::*` writers (§5.5) are the only writers for meeting prep status state.

2. **Read abilities follow the abilities-runtime contract** (ADR-0102). `get_entity_intelligence` (§5.1) + `get_daily_briefing` (§5.10) declare allowed actors, modes, scopes, MCP exposure, signal policy. `get_daily_briefing` is User-only + `NotExposed` per DOS-507 L0 decision; future MCP exposure requires `/cso` re-approval.

3. **Claim receipts emit signals that propagate via existing invalidation paths** (ADR-0125 + signals substrate). W1 consumes existing `ClaimVerificationStateChanged` (per AC-339.6), `entity-link-changed`, `source-asof-updated`, `contradiction-detected`, `correction-recorded` signals. **One new signal** introduced by W1 (per cycle-1 architecture F3 fold): `MeetingPrepStatusChanged` for DOS-335 dismissals + status transitions, pre-declared in `signals/policy_registry.rs` at W1 kickoff (AC-335.15). `signal_policy = { emits_on_output_change = [], coalesce = false }` on new Read abilities matches `get_daily_readiness` pattern.

4. **Privacy / redaction at receipt-emit boundary, not consumer boundary** (ADR-0108 + DOS-341 matrix). `apply_privacy_for_audience` runs inside `render_receipt_for` (DOS-339 substrate). Consumers receive already-redacted receipts; they cannot accidentally over-share by mishandling raw fields they never see.

5. **Audit trail lives in Linear** (memory `feedback_linear_is_the_audit_trail`). Reviewer verdicts, L6 decisions, K-out runs, retro outcomes all land as Linear comments. Git keeps wave-scoped artifacts (proof bundles, retros, ADRs); operational logs live in the local audit storage and are NEVER surfaced as a product surface.

6. **Producer→projection→renderer split** (ADR-0130). W1 owns producers (abilities + services). W2–W5 own projections (WP block render PHP) + renderers (Gutenberg blocks + tokens). Surface-side concerns route to those wave L0 packets.

7. **Chrome runtime-injection scope unchanged** (v1.4.4 W3 chrome lane invariant). W1 substrate is consumed by Gutenberg block renderers AND by the existing runtime-injected chrome (FolioBar prep readiness signal — DOS-335). No new runtime-injected chrome introduced in W1.

8. **No new `claim_kind` variants without L0 plan amendment** (CLAUDE.md substrate-grep obligation). W1 consumes existing variants from `CLAIM_TYPE_REGISTRY`. If sub-tickets discover a missing variant during L1 (e.g., `ClaimType::Touchpoint` to model touchpoints as claims), the L0 plan amendment is filed BEFORE the migration claim.

9. **Tauri UI freeze respected** (memory `feedback_tauri_ui_freeze`). W1 substrate is surface-agnostic and ships in `src-tauri/` (runtime + abilities); WP block render is the v1.4.4 consumer. Existing Tauri React surfaces stay in stasis pending WP equivalents.

10. **No PII in commit messages, titles, PR bodies, OR comments** (CLAUDE.md Critical Rule). Fixtures use generic synthetic data only (`subsidiary.com`, `parent.com`, `user@example.com`). PII-blocklist gate enforced per `.githooks/pre-commit`.

## 11. Reviewer matrix (L0 panel)

Per `.docs/plans/engineering-ladder.md` Amendment 3: `/cso` applies when the packet surface actually touches **trust-boundary enforcement, write-path validation, or privacy/redaction logic** — not when a read-path consumes existing claim/provenance substrate. James's scoping call 2026-05-20: the bulk of W1 is Read abilities (DOS-459/460/461/335/339-wiring/340/507) that consume substrate already CSO-reviewed at its producer. The 5-panel below is the **default** for the wave; `/cso` opts in **per-sub-ticket** for the three items that genuinely cross security boundaries.

**Default 4-panel (applies to every sub-ticket):**

| Reviewer | Why | Verdict artifact path |
|---|---|---|
| `/codex challenge` | Adversarial review — catches L0-class blockers before L1 burn | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md` |
| `/codex consult` | Substrate-grep verification + ADR consumption sanity check | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md` |
| `ce-architecture-strategist` | Producer/projection/renderer split + signals propagation correctness | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md` |
| `ce-correctness-reviewer` | Test plans + fixture coverage + no-bypass harness shape (DOS-461) | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md` |

**`/cso` add-on — REQUIRED for these sub-tickets only:**

| Sub-ticket | Why CSO is required |
|---|---|
| **DOS-477** entity-detail trust-boundary hardening | Defines the boundary itself — Tauri channel allowlist + per-ability sensitivity gate enforcement. Trust-boundary is the explicit deliverable. |
| **DOS-8** semantic feedback actions | User-input write path — `claim_feedback` typed enum flows from UI → service → claim store. Input validation + write-path discipline is CSO territory. |
| **DOS-341** receipt privacy / redaction rules | Privacy/redaction at receipt-emit boundary. Defines what user-presentable receipts redact vs reveal. |

**Read-path sub-tickets — `/cso` NOT required** (consume already-CSO-reviewed producer substrate; no new boundary, no new write path):

DOS-459 (`get_entity_intelligence` Read ability), DOS-460 (touchpoints/open-loops Read contract), DOS-461 (fixture harness — test infrastructure), DOS-335 (meeting prep DTO — status writes are service-owned, not user-input), DOS-339 (shared receipt — DOS-701 producer already CSO-reviewed; W1 work is consumer wiring), DOS-340 (receipt vs operational classification — read-side denylist, no boundary crossing), DOS-507 (`get_daily_briefing` Read ability).

**Per-sub-ticket reviewer add-ons (cumulative on top of the 4-panel):**

- DOS-339 + DOS-8 + DOS-507 — `/plan-devex-review` (consumer-facing API surface)
- DOS-461 — additional codex challenge pass on harness no-bypass assertions
- DOS-477 + DOS-8 + DOS-341 — `/cso` per the table above

**Pass rule:** Unanimous APPROVE. Cycle-2 dispatch only if any reviewer returns BLOCKED or substantive CONDITIONAL. Per memory `feedback_review_loop_l6_policy`: continue looping on architectural/critical/high; expand scope on edge-case classes; L6 at 15 cycles or genuine decision. Per memory `feedback_l0_partial_convergence_when_class_recurs`: same-class recurrence across cycles + downstream-plan-impl-not-substrate-spec → accept path-α + transfer residuals to downstream plan AC.

**Cycle-1 outcome (2026-05-20):**

| Reviewer | V1.0 Verdict | V1.1 Status |
|---|---|---|
| `/codex challenge` | APPROVE WITH CHANGES (1 HIGH + 3 LOW) | All 4 findings folded into V1.1 (AC-W1.9 + LOC re-check + §5.8 Q5 rewrite + AC-W1.1 fix); awaiting cycle-1 reviewer confirmation that fold lifts approval to clean APPROVE. |
| `/codex consult` | APPROVE WITH FINDINGS (2 minor sketch drifts) | Both signature drifts folded via CSO F1 symbol sweep; F3 Proposal-receipt deferral noted in §5.6. |
| `ce-architecture-strategist` | **BLOCKED** (3 CRITICAL/HIGH + 4 MEDIUM/LOW + 2 path-α) | All 7 in-scope findings folded; F1 (slot collision) → v240–v249; F2 (Read-ability call-graph) → §5.5 read/write split + AC-335.12; F3 (signals gap) → new MeetingPrepStatusChanged signal; F4/F5/F6/F9 folded; F7/F8 → maintenance project. **V1.1 lifts BLOCKED.** |
| `ce-correctness-reviewer` | CONDITIONAL APPROVE (1 CRITICAL + 4 HIGH + 5 MEDIUM/LOW) | All 10 findings folded inline (AC-335.13 commutativity correction, AC-461.6a/b split, BriefingState recompose, CursorState, derived-claim handling, AC-459.10 partial-failure, fixture matrix, AC-8.9 retry, AC-340.7 denylist extensibility, AC-335.14 state transitions). |
| `/cso` (DOS-477 + DOS-8 + DOS-341) | CONDITIONAL APPROVE × 3 (12 findings) | All 12 findings folded: 1 (symbol sweep), 2 (allowlist-primary + CI lint), 3 (transitive composes), 4 (free-text Confidential default), 5 (variant field names), 6 (source hash), 7 (sanitizer), 8 (Agent deny), 9 (idempotency TTL), 10 (build_receipt_for_audience), 11 (OperationalAuditStorage lint), 12 (AgentMcp row). |

**Cycle-2 dispatch:** **Not required.** All 23 findings folded into V1.1. Re-dispatch only if any cycle-1 reviewer disputes a specific fix surface.

## 12. References

**ADRs (consumed verbatim):**
- ADR-0057 — Entity intelligence architecture (anchors DOS-459)
- ADR-0083 — Product vocabulary (all user-facing strings)
- ADR-0102 — Abilities runtime contract
- ADR-0105 — Trust scoring + trust bands
- ADR-0108 — Provenance rendering + privacy (anchors DOS-341)
- ADR-0113 — Human + agent analysis as first-class claim sources
- ADR-0123 — Typed claim feedback semantics (anchors DOS-8)
- ADR-0125 — Claim anatomy + temporal + sensitivity + TypeRegistry
- ADR-0129 — Composable surfaces; WordPress Studio as primary surface
- ADR-0130 — Surface-independent composition contract
- ADR-0131 — Structured embedding + claim canonicalization

**Plan docs:**
- `.docs/plans/wp-foundation-roadmap-reorientation.md` §v1.4.4 — Wave structure + W1 substrate gaps list
- `.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md` — L0-converged contracts for Shared Receipt DTO, semantic feedback action surface, claim_receipt module skeleton (verbatim spec source for §5.6, §5.7, §5.8, §5.9)
- `.docs/plans/engineering-ladder.md` — L0 reviewer matrix + K-in obligation + Amendment 3 `/cso`-mandatory triggers
- `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` — shape template; chrome lane (W3) substrate already shipped + consumes some W1 substrate at surface-ship (DOS-335 → FolioBar)

**Existing substrate files (verified at scan time 2026-05-20):**
- `src-tauri/src/services/claim_receipt/{mod,contracts,render,auth}.rs` — DOS-701 shipped (PR #323)
- `src-tauri/src/services/claim_receipt/{feedback,boundary,privacy,contradiction,render_rules}.rs` — zero-byte placeholders awaiting W1 fill
- `src-tauri/src/services/claims.rs:6700` — `record_claim_feedback` (9-variant aware)
- `src-tauri/src/services/claims.rs:8906` — `reconcile_contradiction`
- `src-tauri/abilities-runtime/src/abilities/feedback.rs:31` — `FeedbackAction` enum (9 variants)
- `src-tauri/abilities-runtime/src/abilities/get_entity_context.rs` — composed by §5.1
- `src-tauri/abilities-runtime/src/abilities/list_open_loops/` — extended by §5.2
- `src-tauri/abilities-runtime/src/abilities/prepare_meeting/` — composed by §5.10
- `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs` — composed by §5.10
- `src-tauri/abilities-runtime/src/sensitivity.rs:226-266` — `ClaimSensitivity` + render policy
- `src-tauri/scripts/check_claim_writer_allowlist.sh` — pattern source for §5.8 audit-disclosure CI lint

**Memories consumed:**
- `feedback_check_substrate_before_authoring_primitives` — applied throughout §6 substrate-consumed audit
- `feedback_wire_existing_substrate_not_future_producer` — applied in §7 vertical-slice AC (no empty-branch shipping)
- `feedback_dont_swing_past_center_when_correcting` — applied to scope sizing (10 substrate sub-tickets, not 20)
- `feedback_pick_more_complete_option_over_simpler` — applied to envelope shape (DOS-459 ships full envelope, not "shallow MVP")
- `feedback_think_in_dod_and_outcomes_not_ticket_scope` — applied to wiring obligation (AC-W1.1 + AC-W1.2)
- `project_engineering_ladder` — applied to §11 reviewer matrix (`/cso` mandatory)
- `feedback_l0_reconcile_against_dev` — applied to §9 slot block reservation
- `feedback_no_deferrals_period` — applied to §8 scope (defer only across version boundaries, not within v1.4.4)
- `feedback_l0_partial_convergence_when_class_recurs` — applied to §11 pass rule

## 13. Open architectural questions

**James's wave-level decisions 2026-05-20 that constrain this packet:**

- **Pagination is server-side via cursor.** Every list-shape envelope (DOS-459 entity list variant, DOS-460 touchpoints/open-loops list) MUST return `next_cursor` from day one. Consumer blocks paginate by re-invoking the ability with the cursor. No client-side hide-extras pattern. (Folded into DTO sketches in V1.1: `Paginated<T>` wrapper + `CursorState` enum per AC-459.9 / AC-507.10.)
- **DOS-336 candidate extension hook is NOT in this wave.** W1 leaves the claim review queue alone. v1.4.6 Salience designs and lands the candidate hook itself. Strike from §5.6 and §5.7 scope; reference in §8 (out of scope).
- **Tauri shell deprecation is flag-flip at W6.** Doesn't directly constrain W1 substrate but affects how DOS-461 no-bypass harness asserts: harness runs against the WP surface set even while Tauri UI is still live.
- **Entity-detail composites are 1 outer + N inner blocks.** DOS-459 envelope shape stays a single composed structure (not N independent envelopes); the outer block consumes the composed envelope, inner blocks render their slice.

**V1.1 resolutions (cycle-1 fold):**

1. **Envelope composition with `build_intelligence_context()`.** **[RESOLVED V1.1 — coexist + path-α maintenance ticket.]** Per cycle-1 architecture F7 (path-α candidate). V1.0 recommendation accepted: coexist in v1.4.4; v1.5.x consolidation ticket FILED in DailyOS Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) at W1 retro, not deferred. No drift-detection AC needed for V1.0 lifetime; future merge of the two paths handled via the maintenance ticket.

2. **What makes a claim_receipt "user-presentable" vs "operational" — DOS-340 §5.8 boundary.** **[RESOLVED V1.1 — allowlist-primary + CI lint extensibility gate.]** Per cycle-1 CSO F2: receipt allowlist (`RECEIPT_ALLOWED_FIELDS`) is **primary**; denylist (`AUDIT_ONLY_DENYLIST`) becomes redundant CI lint. AC-477.12 codifies; AC-340.7 adds `check_audit_denylist_completeness.sh` extensibility check (cycle-1 correctness F9). New columns on operational audit schema MUST update the denylist OR carry `// receipt-safe: <reason>` comment.

3. **`get_daily_briefing` consumes existing prep DTO or extends it — DOS-335/DOS-507 coupling.** **[RESOLVED V1.1 — Read-only; §5.5 read/write split statically enforces.]** Per cycle-1 architecture F2: `services::meeting_prep_status::read` is pure read (call-graph-safe for `get_daily_briefing`); writers live in `::write`. AC-335.12 adds trybuild/call-graph lint asserting `compute_status` graph contains zero mutations. Viewed-tracking, if needed, lives in a sibling W2/W3 service mutation path — not in the briefing ability.

4. **Semantic feedback action API shape — DOS-8 §5.7 typed `claim_feedback`.** The 9-variant enum is fixed at substrate level. Is the consumer surface (UI / WP block) allowed to combine variants into a smaller user-visible set (e.g., "Wrong" UI button maps to MarkFalse OR WrongSubject OR WrongSource based on context)? **Recommendation:** YES — UI may collapse to a primary set + overflow menu, but the persisted action must remain semantic per ADR-0123. Ticket DOS-8 explicitly allows this. Reviewer call: codify as packet AC-8.x? **Yes — folded into AC-8.6.**

5. **Trust-boundary hardening — DOS-477 §5.4: Tauri channel allowlist vs per-ability sensitivity gate.** **[RESOLVED V1.1 — BOTH; symbol names corrected.]** Sensitivity gate is the substrate (shipped at `render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)` — symbols corrected per cycle-1 CSO F1; V1.0's `render_policy_for` was wrong). IPC channel allowlist is W2+ surface concern (block render PHP enumerates allowed ability calls per surface). W1 lands the substrate composition (AC-477.11 + AC-477.12); W2 enumerates per surface.

6. **`get_entity_intelligence` envelope vs `get_entity_context` entry-list compatibility — DOS-459 §5.1.** **[RESOLVED V1.1 — coexist + v1.5.x consolidation maintenance ticket filed at W1 retro.]** Same path-α resolution as Q1; see cycle-1 architecture F7. Leave clean (no deprecation noise in v1.4.4).

7. **Meeting prep status DTO — DOS-335 §5.5 — concurrent writer ordering.** **[RESOLVED V1.1 — per cycle-1 correctness F1.]** V1.0's "writes commute on disjoint columns" recommendation was partially wrong: `record_user_authored` emits Decision/Commitment **claims**, which feed the v241 indexed view through the claim store and break the disjoint-column premise. **V1.1 two-part contract** (AC-335.13):
   - **Part A:** Plain user-authored fields (agenda, notes, preparation_text, hidden_attendees) commute with `write::enqueue_refresh` — disjoint columns. Property-tested.
   - **Part B:** Decision/commitment claim emission is a claim-store write, NOT disjoint-column. Status recompute observes the new claim via the standard claim-lifecycle-signal → invalidation path. Contract is **eventual consistency via existing signal substrate**, not commutativity.

8. **DOS-461 no-bypass harness — what counts as "the source of rendered claim-backed intelligence."** **[RESOLVED V1.1 — three-mode assertion per cycle-1 correctness F2.]** AC-461.6a (text-extraction binding check): every claim-substantive string traces to `[data-claim-id]` ancestor. AC-461.6b (stale vs bypass distinction): unresolved `data-claim-id` renders as **stale**, NOT bypass; distinguished failure mode with `account_claim_retracted_mid_render.json` fixture. AC-461.5b adds the per-subject expected-fixture matrix (Account/Project/Person × fixture-class) so "comprehensive coverage" is mechanically provable.

9. **DOS-507 `BriefingState::NeedsPreparation` — does it auto-enqueue prep, or strictly return the state?** **[RESOLVED V1.1 — Read-only.]** Per ADR-0102 §3 Read-ability call-graph (cycle-1 architecture F2). `get_daily_briefing` is pure read. The W3 block renderer (or a sibling write-ability `enqueue_briefing_prep`) handles the enqueue separately. AC-507.3 codifies. The §5.5 read/write split (AC-335.12) statically enforces no-mutation in the briefing call graph.

11. **List-envelope cursor design.** **[RESOLVED V1.1 — opaque server-signed cursor + typed `CursorState`.]** Per cycle-1 correctness F4: cursor is opaque server-signed (survives schema changes, prevents tampering). Pagination behavior under concurrent writes is **named explicitly**: every list-shape envelope returns `Paginated<T> { items, next_cursor, cursor_state: CursorState }` where `CursorState = Stable | DataShifted { advisory } | Invalidated { reason, restart_required: true }`. Concurrent insert/retract tested per AC-459.9. **CSO question:** since cursor is pagination state (not sensitivity state), DOS-459 does NOT need `/cso`. Residual key-management concern (where the cursor-signing key lives, rotation policy) is correctness R1 — flag to v1.4.6/v1.4.7 if cursor signing key becomes a security-token concern, not a substrate change for W1.

10. **W1 sub-lane sequencing within W1.** **[RESOLVED V1.1 — per cycle-1 architecture F6.]** §5.4 (DOS-477) moves from Stage 1b → Stage 1a because §5.7 (DOS-8) calls `validate_envelope_target` as the first step of `submit_claim_feedback`. Boundary primitive must exist before consumers wire it.

    **V1.1 staging:**
    - **Stage 1a (parallel — boundary + envelope + prep + receipt wiring):** §5.6 wiring (DOS-339 finishers); §5.5 (DOS-335 read+write split); §5.1 (DOS-459 envelope); **§5.4 (DOS-477 boundary helpers — moved from 1b)**
    - **Stage 1b (parallel after 1a — placeholder fills + consumers):** §5.7 (DOS-8); §5.8 (DOS-340); §5.9 (DOS-341); §5.2 (DOS-460 — composes envelope from §5.1); §5.3 (DOS-461 — harness needs envelope + boundary)
    - **Stage 1c (after 1b):** §5.10 (DOS-507 only — depends on §5.1 + §5.5)

    Migration vs contract-freeze boundary: §5.5 ships migrations v241+v242 in Stage 1a; DTO contract is frozen at Stage 1a close. §5.10 (Stage 1c) consumes the contract, not the migrations directly.

---

**End of L0 Packet — v1.4.4 W1 Substrate Gaps V1.1.** Cycle-1 fold complete. V1.0 BLOCKED → V1.1 lifts blockers. 23 cycle-1 findings folded inline; 2 path-α findings filed to maintenance project. No cycle-2 dispatch required.
