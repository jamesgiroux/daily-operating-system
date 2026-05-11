# v1.4.2 — Personal Intelligence Engine: WordPress Foundation — Wave Plan

**Date:** 2026-05-10
**Target outcome:** Ship the WordPress foundation: substrate-to-surface contract, paired SurfaceClient transport, custom WP MCP server, first Gutenberg block, three-view consistency, and clean-machine validation. The release that makes ADR-0129 real.
**Source of truth:** Linear project [v1.4.2 — Personal Intelligence Engine: WordPress Foundation](TBD — created after this draft is approved) + project description at `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` + issue list at `.docs/plans/dos-546/v1.4.2-project/02-issues.md`.
**Predecessor:** v1.4.1 (Abilities Runtime Completion). Wave plan: [v1.4.1-waves.md](../../v1.4.1-waves.md).
**Spike basis:** [DOS-546](https://linear.app/a8c/issue/DOS-546) — six L0 cycles + /cso APPROVE on 2026-05-10. Phase 0 design artifacts at `.docs/plans/dos-546/phase-0/`.
**Success metric:** Clean-machine install on a previously-untouched macOS test box reaches a rendered daily briefing in WordPress Studio in ≤15 minutes of user time, with the four Phase 0 contract surfaces (concurrency, tamper, pairing, rate-limits) all proven by negative fixtures + manual exercise.

## Release thesis

DailyOS is a runtime. The runtime renders into surfaces. Per ADR-0129, the primary composable surface is WordPress Studio. v1.4.2 is the foundation release: it does NOT ship rich entity intelligence (that's v1.4.3 on this foundation); it ships the load-bearing contract — `SurfaceClient` actor class, canonical `AbilityPolicy`, `Composition` substrate types, loopback transport, pairing model, custom MCP server, first block, three-view consistency, negative fixtures, clean-machine validation. Every v1.4.3+ wave consumes what this release commits.

The release is foundation, not surface depth. One block proves the producer/renderer split. Multiple blocks are v1.4.3+. The discipline this release enforces is that the substrate stays the authority; WordPress is a SurfaceClient; markdown is a projection; nothing in the WP DB acts as canonical.

## What v1.4.1 leaves behind

v1.4.1 shipped:
* Capability migrations (`get_daily_readiness`, `list_open_loops`, `detect_risk_shift`).
* Signal infrastructure: durable invalidation jobs, policy registry, coalescing, load gate.
* Scoring + trust depth: factor library, freshness decay, shadow-mode rollout, tuned thresholds.
* Substrate completions: source taxonomy migration, prompt fingerprinting, provenance rendering, temporal primitives, DbKeyProvider seam, contract-first operations, declarative claim-field → edge-type map, fail-improve loop, CommitmentClaim, canonicalize.
* Validation suite: bundles 1-18 mandatory; 5 new validation axes covered.
* Ability-runtime crate boundary compile-time-enforced.
* Eval depth: BrainBench-style harness, Suite P Criterion harness, comparison packet.

v1.4.2 wires the surface foundation onto this completed substrate. **v1.4.1 must merge before v1.4.2 wave kickoff** — partial substrate produces partial blocks.

What v1.4.1 does NOT ship that v1.4.2 needs:
* `SurfaceClient` as a fourth actor class — v1.4.2 W1.
* `AbilityPolicy.required_scopes` + `.mcp_exposure` as canonical fields — v1.4.2 W1.
* `Composition` substrate types + `ProvenanceRef` — v1.4.2 W1.
* Any WP-side rendering — v1.4.2 W3-W5.
* Loopback HTTP transport — v1.4.2 W2.
* Pairing model — v1.4.2 W2.
* Three-view consistency contracts — v1.4.2 W4.
* Magazine theme on WP — v1.4.2 W5.

## Product thesis alignment

Mission Gate (unchanged from v1.4.x):

> Does this make DailyOS better at maintaining and updating the user's working understanding of their professional world?

For v1.4.2, the answer is: yes, because it makes the substrate composable, editable, and shareable on a surface the user already lives in — turning latent runtime capability into rendered, correctable intelligence.

Every wave preserves the v1.4.0/v1.4.1 product invariants and adds three more:

1. **The substrate is the only authority.** WordPress is a paired SurfaceClient, not a runtime. Markdown is a projection, not source of truth. The runtime owns the mutation boundary.
2. **Surfaces are pluggable; the contract is fixed.** WordPress is the first SurfaceClient transport beyond Tauri's local IPC. The contract (Composition, ProvenanceRef, AbilityPolicy with required_scopes + mcp_exposure, SurfaceClient actor, signed projections) is the load-bearing primitive every future surface consumes.
3. **User corrections stick across all three views.** Edit in WP → substrate apply → markdown re-project. v1.4.2 ships **2-view-write + 3-view-read** consistency: WP edits write through substrate; markdown is a durable read-side archive (substrate emits the file when claims change). Out-of-band edits in either WP DB or markdown are detected, quarantined, and never silently promoted to canonical. Bidirectional markdown-as-input reconciliation (edit a markdown file → substrate ingests + re-renders) is **v1.4.6 (Workspace Memory Refactor)** scope, NOT a v1.4.2 DoD criterion. Tamper detection (W4-C Ed25519 signatures) is the load-bearing detection path for v1.4.2.

## Reading order

1. This doc — wave assignments and merge gates.
2. v1.4.2 project description (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md`) — full scope + Definition of Done.
3. v1.4.2 issue list (`.docs/plans/dos-546/v1.4.2-project/02-issues.md`) — per-issue contract + acceptance criteria.
4. ADRs: 0102 (§7.1, §7.6), 0111 (§8), 0128, 0129, 0130. Plus 0105 (provenance envelope) and 0108 (sensitivity rendering).
5. Phase 0 artifacts: `.docs/plans/dos-546/phase-0/01-...` through `15-...` (all 15 landed; W0-C INDEX issue promotes them to `spec:ready`).
6. /cso review verdict: `.docs/reviews/dos-546-l0-cso-2026-05-10.md`.
7. v1.4.1 proof bundles for the substrate this release renders against.

## Reusing the review system

The L0–L6 review ladder, plan-review template, reviewer matrix, test suites S/P/E, proof-bundle template, escalation packet format, and pilot/retro protocol are defined in [v1.4.0-waves.md](../../v1.4.0-waves.md) + [v1.4.1-waves.md](../../v1.4.1-waves.md). **v1.4.2 reuses them unchanged unless explicitly amended below.**

### Review ladder — which L levels apply

This is a **surface project**, not a pure substrate project. L0/L1/L2/L4 apply to every wave. L3 applies to waves that integrate multiple parallel lanes (W1, W4). L5 applies at W6 release. L6 only if escalated.

| Level | Applies | Notes |
|---|---|---|
| **L0 Prep (Plan Hardening)** | Every issue | `/plan-eng-review` is the default. Add `/plan-design-review` for the magazine theme issue (W5) and the first-block issue (W4). Add `/plan-devex-review` for the WP plugin skeleton (W3) — DX matters for a plugin. |
| **L0 (Plan)** | Every issue | Codex challenge + domain reviewer + Codex consult. `/cso` added for every issue touching trust boundary (pairing, HMAC, scopes, MCP exposure, nonce, fallback projection, tamper, audit). |
| **L1 (Self)** | Every issue | Standard. |
| **L2 (Diff)** | Every issue | Codex review + code-reviewer + domain reviewer. `/cso` added for trust-boundary diffs. **L2 runs locally before push, per the protocol.** |
| **L3 (Wave)** | W1, W2, W4 (integration-heavy or load-bearing trust-boundary waves) | Codex challenge against integrated diff + ADR alignment + Suites S/P/E. W2 is sequential within the wave but composes transport+HMAC+pairing+rate-limit into one integrated trust surface — L3 is mandatory per architect cycle-1 finding E1 to align the matrix with the W2 merge-gate text. W3/W5/W6 may run L3 if the wave coordinator decides scope warrants it. |
| **L4 (Surface)** | W4, W5, W6 (user-facing changes) | `/qa-only` first. `/qa` if remediation needed. WP-specific QA includes browser checks of the first block in editor + published views; clean-machine flow validation in W6. |
| **L5 (Drift)** | W6 release | `/plan-eng-review` + architect-reviewer comparing integrated state to v1.4.2 DoD. |
| **L6 (Human)** | If escalated | Same triggers as v1.4.x. |

Amendments for v1.4.2:

* **No pilot wave.** v1.4.0 W0 + v1.4.1 startup proved the system. v1.4.2 starts at full fan-out from W0.
* **/cso is the default L0 panelist for every trust-boundary issue.** The surface foundation IS a trust-boundary release; substrate-side and WP-side both touch the boundary. /cso runs at L0 plan (and again at L2 for trust-boundary diffs).
* **WP-specific lint discipline.** PHP_CodeSniffer with WordPress Coding Standards on plugin code. WP Theme Check on the magazine theme. Block.json schema validation on every block. Add to CI before W3 lands.
* **Negative-fixture catalog from artifact 12 is mandatory release-gate scope, not nice-to-have.** **Six** new fixture bundles (19-24) join bundles 1-18 from v1.4.1. The sixth bundle (24, audit attribution) was promoted to mandatory after L0 cycle-2 (audit schema moved to W1-A0 stage-1 issue).
* **Phase 0 artifact 11 retained.** Cycle-2 had described artifact 11 (`editable-composition-overlay.md`) as retired, but the artifact landed (450 lines, `status: proposed`) and is substantive — it specifies the substrate-vs-overlay block taxonomy, save-handler routing rules, paste/nesting/reorder semantics across the boundary. L0 cycle-3 reversed the retirement. Artifact 11 is the design source for W4-D's edit-routing acceptance bullet and for W5-A's save-time feedback router; the W0-C INDEX issue records artifact 11 as `spec:ready` alongside the others.

## Scope cleanup before kickoff

Linear hygiene items resolved before plan freeze:

1. **Original v1.4.2 ("Entity Intelligence") superseded.** Per the W0 supersession issue: every open issue routed to a new project `v1.4.3 — Entity Intelligence on WP` (or equivalent name James decides); old v1.4.2 moved to Canceled/Superseded with a final comment linking to ADR-0129 + the new project. **James executes this** based on the W0 issue's plan. No issues from the old v1.4.2 are deleted.
2. **DOS-546 closure.** DOS-546 ships its proof bundle from Phase 0 and is marked Done. Its commenting trail is the predecessor record for v1.4.2.
3. **Wave 8 carry-forward from v1.4.1.** If v1.4.1 W8 (eval/benchmark consolidation) has follow-up work that lands during v1.4.2's window, those follow-ups stay in v1.4.1's project — they do NOT migrate into v1.4.2 unless they unblock v1.4.2 work specifically.
4. **Artifact 11 retained** — W0-C INDEX issue promotes it to `spec:ready` alongside the other 14 artifacts (cycle-3 reversed the cycle-2 retirement).
5. **v1.4.2 project created** from the description in `.docs/plans/dos-546/v1.4.2-project/01-project-description.md`. **James creates the project**; the issues land afterward.

## Wave shape — seven wave units (W0 – W6)

Mapping the 28 v1.4.2 issues onto codex-agent waves. Default ordering rule: each wave gates on the prior wave's merge. Named exceptions called out per wave.

| Wave | Issues | Parallel agents | Wall-clock target |
|---|---|---|---|
| **W0** — Contract lock + supersession | 4 (ADR-0130 amendments, supersession, Phase 0 INDEX, ADR-0102 §7.1 amendment) | 4 | 1-2 days |
| **W1** — Substrate-side contract | 6 (audit-log schema, SurfaceClient actor, AbilityPolicy schema, inventory format, description CI gate, Composition types) | 6 | 1 week |
| **W2** — Runtime transport + pairing | 4 (loopback HTTP endpoint, HMAC signing, pairing + 4 defenses, rate-limit matrix) | 4 | 1 week |
| **W3** — WP plugin + MCP server | 3 (plugin skeleton, runtime client + HMAC + pairing UI, custom MCP server + low-cap user) | 3 | 1 week |
| **W4** — Composition + first block + three-view consistency | 6 (account-overview ability, account-overview block, concurrency contract impl, tamper detection + projection signing, custom block fallback, user-presence nonce) | 6 | 1.5 weeks |
| **W5** — Feedback + theme + negative fixtures | 3 (save-time feedback router, magazine theme, negative fixture catalog) | 3 | 1 week |
| **W6** — Audit + clean-machine validation + release gate | 2 (audit attribution forensic + CI lint, clean-machine validation + launcher) | 2 | 4-5 days |

Total = 28 issues across 7 waves. Estimated wall-clock ~6 weeks across the wave ladder with the default merge-gate-on-prior-wave rule (W0-D / W1-A0 / W4-A0 land within existing wave windows because they run in parallel with their wave-mates).

## What "parallel" means here

Same shape as v1.4.0/v1.4.1:

1. **A frozen contract.** The Linear ticket text is the contract. Agents do not invent shapes.
2. **An exclusive file/table allowlist.** Listed below per agent.
3. **A deny list.** Files no other agent in the wave will touch.
4. **A merge gate.** Concrete artifact (test, lint, command output) proving done before the next wave starts.

Named ordering rules for v1.4.2:

* **W1 internal staging.** ADR-0130 amendments (W0-A) must land before W1-E (Composition contract substrate types) starts. ADR-0102 §7.1 amendment (W0-D) must land before W1-B (AbilityPolicy canonical schema) starts. Inside W1, **stage-1a** = W1-A (SurfaceClient actor) + W1-B (AbilityPolicy) run in parallel (file-disjoint at `actor.rs` vs `policy.rs`). **Stage-1b** = W1-A0 (audit-log schema + `emit_surface_audit` helper) starts once W1-A merges — its helper signature pattern-matches `Actor::SurfaceClient { instance, .. }` and cannot compile without W1-A. W1-A0 can land in parallel with W1-B once W1-A has merged. All three (W1-A, W1-A0, W1-B) must merge before stage-2 starts. **Stage-2** = W1-C (inventory) + W1-D (description CI gate) + W1-E (Composition types) running in parallel after stage-1. Inventory + Composition reference SurfaceClient + AbilityPolicy + audit schema. (L0 cycle-3 finding #2 corrected stage-1 ordering: cycle-2 had described W1-A0 as W1-A-independent, but the helper's `Actor::SurfaceClient` pattern-match makes the type dependency hard.)
* **W2 strict sequencing.** Loopback HTTP endpoint (W2-A) before HMAC signing (W2-B); HMAC before Pairing handshake (W2-C); all three before Rate-limit matrix (W2-D) — rate limits enforce at the SurfaceClientBridge which exists only after pairing.
* **W3 internal staging.** Plugin skeleton (W3-A) before Runtime client + HMAC + pairing UI (W3-B); custom MCP server (W3-C) runs in parallel with W3-B once W3-A merges. **W3-A gates both W3-B and W3-C. W3-B additionally depends on W2 full merge. W3-C additionally depends on W2 full merge + W1-B + W1-C merges** (W1-B + W1-C precede W2 in the default merge-gate-on-prior-wave rule, but stated explicitly per architect cycle-1 finding B1 because the dependency is load-bearing).
* **W4 internal staging.** Concurrency contract (W4-B) and Composition types (W1-E) gate the producer + renderer issues. Within W4: **stage-1** = W4-B (concurrency contract impl) — assigns `composition_version`. **Stage-2** = W4-A0 (account-overview ability — producer) + W4-C (tamper detection — Ed25519 signatures) + W4-D (custom block fallback) + W4-E (presence nonce) running in parallel once W4-B merges. **Stage-3** = W4-A (account-overview Gutenberg block — renderer) runs once **W4-A0 + W4-C + W4-D + W3-B** all merge. Per L0 cycle-3 finding #5: the W4-A renderer consumes the W4-D substrate-side fallback projection rules and the W4-C Ed25519 verification path for cached projections — it cannot ship before those substrate rules + signatures exist or it renders unverified bytes / silently demotes unknown blocks. (W4-E is not a W4-A blocker; the renderer does not issue nonces, the feedback router W5-A does.)
* **W5 free fan-out.** Feedback router (W5-A) depends on W4-A + W4-E. Theme (W5-B) is independent. Negative fixture catalog (W5-C) runs from W5 start but **closes last** — W5-C is the last issue to merge in W5; consolidation runs after W5-A and W5-B merge, and bundle 19-24 release-gate integration is the closing act (per architect cycle-1 finding B3).
* **W6 free fan-out.** Audit attribution (W6-A) is independent of clean-machine validation (W6-B); both close in parallel.

Outside these named rules, the default merge-gate-on-prior-wave rule holds.

## CI-enforced architecture invariants — additions for v1.4.2

v1.4.0/v1.4.1 invariants stay active. v1.4.2 adds:

| Invariant | Mechanism | Activated by |
|---|---|---|
| `Composition` is constructed ONLY by ability code | grep CI test on `Composition::new` / `Composition {` outside `abilities-runtime/` | W1-E merge |
| `Block.provenance` is `ProvenanceRef`, never `ProvenanceEnvelope` | trybuild on type shape; grep CI on direct envelope embedding | W0 ADR amendment merge |
| Every `#[ability]`-annotated function has an inventory entry | build-time check enumerates ability fns and inventory | W1-C merge |
| Ability descriptions pass PII blocklist + internal-vocabulary scan | pre-commit gate + CI lint on `tools/dailyos-abilities.json` | W1-D merge |
| No raw DB / FS / Tauri-IPC writes from WP plugin source tree | grep CI test against `dailyos/` plugin directory | W3-A merge |
| No DailyOS substrate-backed ability exposed by the default WP MCP server | negative fixture run as part of release gate | W3-C merge |
| `claim_version` is server-assigned; never generated client-side | grep CI test on `claim_version =` outside the substrate single-writer service | W4-B merge |
| Loopback transport rejects non-`127.0.0.1` Host headers | unit test + Suite S integration | W2-A merge |
| HMAC verification runs before ability dispatch | function-call-order test | W2-B merge |
| Per-projection signature verification runs on every projection read | integration test | W4-C merge |
| Every WP-originated audit log entry carries `wp_user_id` + `actor_instance` via the W1-A0 `emit_surface_audit` helper | grep CI lint on log emission for `Actor::SurfaceClient`; helper-call enforcement | W1-A0 merge (lint live); W6-A merge (lint + forensic exercise) |
| `AbilityPolicy.mcp_exposure` is the canonical tri-state `McpExposure { None | MetadataOnly | Invocable }` AND `AbilityPolicy.client_side_executable: bool` is a separate field governing SurfaceClient invocation (per artifact 05 lines 389-412); the two fields are independent | type test pins `McpExposure` enum shape; type test pins `AbilityPolicy` struct fields including `client_side_executable: bool` | W0-D merge (doc); W1-B merge (code) |
| Bundles 1-24 mandatory in v1.4.2 release gate | release-gate config | W6 close |

## Proof-bundle + retro template

Same as v1.4.0/v1.4.1. Each wave produces a proof bundle and (for designated waves) a retro doc. Save to `.docs/plans/wave-W{N}-v142/proof-bundle.md` and `.../retro.md`. Use `-v142` suffix to avoid collision with v1.4.0/v1.4.1 wave dirs.

Mandatory retros for v1.4.2:

| Wave | Retro |
|---|---|
| W1 | **Mandatory** — first foundational substrate-contract wave for the surface program |
| W2 | **Mandatory** — first transport + pairing work; security learnings critical |
| W4 | **Mandatory** — first user-facing block wave; integration learnings critical |
| W6 | **Mandatory** — release gate; clean-machine validation learnings |
| Others | Optional unless material observations |

---

# Wave 0 — Contract lock + supersession

Four issues. Three are documentation (W0-A ADR-0130 amendments, W0-C Phase 0 INDEX, W0-D ADR-0102 §7.1 amendment). One is project hygiene (W0-B old v1.4.2 supersession). Sets the contract source-of-truth before any substrate code lands.

### Agent W0-A — ADR-0130 amendments: ProvenanceRef + custom block fallback

* **Spec:** Issue `Adopt ADR-0130 amendments: ProvenanceRef + custom block fallback projection`.
* **Files owned:** `.docs/decisions/0130-...md` (existing). Read-only on Phase 0 artifacts 06 + 07.
* **Don't touch:** any Rust types, ability code, migration scripts.
* **Done when:** ADR-0130 §2 carries `provenance: ProvenanceRef`; §3 ships field-pointer-granularity projection rules per artifact 07; cross-references to ADRs 0102/0105/0108 updated.

### Agent W0-B — Supersede original v1.4.2 + park entity-intelligence scope

* **Spec:** Issue `Supersede original v1.4.2 and park entity-intelligence scope for v1.4.3`.
* **Files owned:** Linear topology only. No code, no schema.
* **Don't touch:** any Linear issue NOT in old v1.4.2.
* **Done when:** new project `v1.4.3 — Entity Intelligence on WP` exists with the parked scope; old v1.4.2 in Canceled/Superseded; every old-v1.4.2 issue re-projected with audit comment.
* **Owner:** James for the actual Linear mutations (per "Don't create the Linear project yourself" rule in the brief). The W0-B agent prepares the issue list + mapping; James executes.

### Agent W0-C — Phase 0 INDEX + spec:ready promotion

* **Spec:** Issue `Promote Phase 0 artifacts to spec:ready and lift acceptance criteria into project-level fixture index`.
* **Files owned:** `.docs/plans/dos-546/phase-0/INDEX.md` (new), frontmatter `status` field on each phase-0 artifact.
* **Don't touch:** artifact body content.
* **Done when:** INDEX.md round-trips reference cleanly to every v1.4.2 issue; artifact 11 status explicit; downstream consumers gated explicitly if artifact 11 missing.

### Agent W0-D — ADR-0102 §7.1 amendment: promote `mcp_exposure` to tri-state + retain `client_side_executable`

* **Spec:** Issue `ADR-0102 §7.1 amendment: promote mcp_exposure to tri-state enum and retain client_side_executable`.
* **Files owned:** `.docs/decisions/0102-abilities-as-runtime-contract.md` (§7.1, §7.6, default-policy bullet). Status-note addendum on `.docs/plans/dos-546/phase-0/05-ability-surface-inventory.md` referencing the canonical ADR shape.
* **Don't touch:** any Rust types, ability code, inventory format Rust struct (W1-B + W1-C own those).
* **Depends on:** none (independent W0 issue; can land in parallel with W0-A, W0-B, W0-C).
* **Done when:** ADR-0102 §7.1 schema field `mcp_exposure: McpExposure` (tri-state enum) AND `client_side_executable: bool` (retained per artifact 05 lines 389-412; cycle-3 reversed the cycle-2 retire decision); default-policy bullet clarifies the SurfaceClient compile-error gate AND the bridge-level `client_side_executable` gate; cross-ADR references updated.

### W0 merge gate

L0 → L2 cleared per the Review Ladder for the doc agents (W0-A, W0-C, W0-D); W0-B's "L2" is the Linear topology being correct (verified by `list_projects` + a spot-check). Required artifacts:
* L0 plan approvals (architect-reviewer for W0-A; project-management for W0-B; spec-writer for W0-C).
* L2 diff approvals on W0-A and W0-C PRs.
* W0-B: James-executed Linear migration with a confirmation comment on the new v1.4.2 project linking to the supersession trail.
* CI invariants now active: `Composition` constructed-only-by-ability (live but vacuous until W1-E); `Block.provenance: ProvenanceRef` enforced.
* Proof bundle written. Retro optional.

---

# Wave 1 — Substrate-side contract

Six agents. Strict file ownership at function-level for shared files. **Two-stage internal ordering with a stage-1a → stage-1b sub-ordering:**

**Stage 1a — Contract foundation (parallel, file-disjoint):**
* **W1-A (SurfaceClient actor class)** lands the fourth actor variant in `actor.rs`.
* **W1-B (AbilityPolicy canonical schema)** lands `required_scopes` + `mcp_exposure: McpExposure` + `client_side_executable: bool` per W0-D's amended ADR-0102 §7.1 in `policy.rs`. Two independent fields per artifact 05 lines 389-412.

**Stage 1b — Audit helper consumes the new actor variant:**
* **W1-A0 (audit-log schema for SurfaceClient attribution)** lands the schema migration + `emit_surface_audit` helper + round-trip plumbing skeleton. **W1-A0 starts after W1-A merges** because its helper signature pattern-matches `Actor::SurfaceClient { instance, .. }` and will not compile without the variant. W1-A0 may run in parallel with W1-B (file-disjoint).

All three (W1-A, W1-B, W1-A0) must merge before stage 2 starts. Stage 1 = 2-3 day window (1 day W1-A solo, then 1-2 days W1-A0 + W1-B parallel). Updated in L0 cycle-3 from cycle-2's "all three parallel from t=0" framing — codex-consult R3 + codex-challenge #2 both flagged the implicit W1-A0 → W1-A type edge.

**Stage 2 — Contract consumers (after stage 1):**
* **W1-C (Ability-surface inventory format + CI gate)** consumes SurfaceClient + AbilityPolicy.
* **W1-D (Ability-description CI gate)** runs in parallel with W1-C (independent of inventory schema — just file scanning).
* **W1-E (Composition contract types)** consumes the ADR-0130 amendments (from W0) + AbilityPolicy + SurfaceClient.

### Agent W1-A0 — Audit-log schema for SurfaceClient attribution

* **Spec:** Issue `Audit-log schema for SurfaceClient attribution (additional columns + emission contract)`.
* **Files owned:** `audit_log` migration; canonical `emit_surface_audit` helper module; round-trip plumbing skeleton (endpoint → bridge → service handoff for `wp_user_id` + `actor_instance`).
* **Don't touch:** `Actor` enum (W1-A owns); `AbilityPolicy` (W1-B owns); inventory format (W1-C); description CI gate (W1-D); Composition types (W1-E); the forensic-exercise + CI lint (W6-A owns).
* **Depends on:** **W1-A merged** (the helper signature pattern-matches `Actor::SurfaceClient { instance, .. }`; updated in L0 cycle-3 from cycle-2's "no W1 dep" framing). File-disjoint with W1-B (parallel with W1-B once W1-A lands). Architect-reviewer cycle-1 finding + codex-consult + codex-challenge cycle-1 all converged on this moving earlier than W6; cycle-3 codex-consult R3 + codex-challenge #2 corrected the staging.
* **Done when:** acceptance criteria from the issue all green; canonical emission helper available for W2-W5 to bind against; every W2+ trust-boundary issue's `emit ... audit` acceptance lines reference the helper.

### Agent W1-A — SurfaceClient as fourth actor class

* **Spec:** Issue `Implement SurfaceClient as the fourth actor class`.
* **Files owned:** `abilities-runtime/src/actor.rs` (`Actor::SurfaceClient` variant + `SurfaceClientId` + `ScopeSet` types). Audit log emission sites for actor-aware events. Eval fixture for SurfaceClient negative case.
* **Don't touch:** `AbilityPolicy` struct (W1-B owns the schema extension); inventory format (W1-C); description CI gate (W1-D); Composition types (W1-E).
* **Done when:** acceptance criteria from the issue all green; CI invariant for SurfaceClient-aware audit emission live.

### Agent W1-B — AbilityPolicy canonical schema: `required_scopes` + `mcp_exposure`

* **Spec:** Issue `Promote AbilityPolicy to canonical schema: required_scopes + mcp_exposure`.
* **Files owned:** `abilities-runtime/src/policy.rs` (the `AbilityPolicy` struct + new fields). `#[ability]` macro extension to accept `required_scopes` + `mcp_exposure` attributes. Default values for non-annotated abilities. Negative tests.
* **Don't touch:** actor types (W1-A owns); inventory format (W1-C); Composition types (W1-E).
* **Done when:** policy gate runs at `SurfaceClientBridge` (skeleton — full bridge lands in W2); MCP introspection filters by `mcp_exposure`; v1.4.0/v1.4.1 abilities compile clean with default field values.

### Agent W1-C — Ability-surface inventory format + CI gate

* **Spec:** Issue `Ability-surface inventory format + CI gate`.
* **Depends on:** W1-A + W1-B merged.
* **Files owned:** `abilities-runtime/src/inventory.rs` (Rust struct), `web/types/ability-surface.ts` (TS interface), `tools/dailyos-abilities.json` (generated artifact), build-time CI check.
* **Don't touch:** `AbilityPolicy` (W1-B); actor types (W1-A); description scan rules (W1-D).
* **Done when:** every `#[ability]`-annotated function has an inventory entry or build fails; `tools/dailyos-abilities.json` consumable by Wave 3 plugin and MCP server.

### Agent W1-D — Ability-description CI gate

* **Spec:** Issue `Ability-description CI gate: PII blocklist + internal-vocabulary scan`.
* **Depends on:** can run in parallel with W1-C — the gate scans `description` fields wherever they appear, both in `#[ability]` macro and in `tools/dailyos-abilities.json` when it exists.
* **Files owned:** `.claude/hooks/pre-commit-gate.sh` extension. CI workflow extension. Fixture test for deliberately-violating description.
* **Don't touch:** blocklist contents (`.claude/pii-blocklist.txt`); vocabulary rules.
* **Done when:** violation = commit refused + CI failure; clean descriptions pass; CLAUDE.md notes ability descriptions as a scanned surface.

### Agent W1-E — Composition contract substrate types + ProvenanceRef shape

* **Spec:** Issue `Composition contract substrate types + ProvenanceRef shape`.
* **Depends on:** ADR-0130 amendments merged (W0-A); SurfaceClient (W1-A) + AbilityPolicy (W1-B) merged.
* **Files owned:** `abilities-runtime/src/composition.rs` (new) — `Composition`, `Block`, `BlockType`, `ProvenanceRef`. Fallback projection logic per artifact 07. Unit tests + CI lint for substrate-owned authorship.
* **Don't touch:** WP-side code; loopback transport (W2); first block (W4).
* **Done when:** types compile, serialize round-trip, unknown-block fallback works per artifact 07, CI lint asserts only ability code constructs `Composition`.

### W1 merge gate

L0 → L2 → L3 → L5 cleared per the Review Ladder. Required artifacts:
* L0 plan approvals (architect-reviewer for W1-A0/W1-A/B/E; spec-writer for W1-C; code-reviewer for W1-D). **/cso added for W1-A0, W1-A, W1-B, W1-E** (trust-boundary primitives — W1-A0 is the audit-attribution gate).
* L2 diff approvals on all 6 PRs.
* L3 wave adversarial: codex-challenge + architect-reviewer + **Suite S** (SurfaceClient flows through every actor-aware site without leaking; `mcp_exposure: None` abilities not enumerated anywhere) + **Suite P** baseline (substrate primitives don't regress v1.4.1 baseline).
* CI invariants now active: `Composition` constructed-only-by-ability (live and tested); `Block.provenance: ProvenanceRef`; ability inventory enforced; description scan active.
* L5 drift check: integrated state matches v1.4.2 project DoD §Substrate-to-surface contract.
* **Mandatory retro** — first foundational substrate-contract wave for the surface program.
* Proof bundle written.

---

# Wave 2 — Runtime transport + pairing

Four agents, strictly sequenced (no parallel within W2 — each agent depends on the prior one). Wall-clock is 1 week sequential because each piece must be in place before the next can be tested end-to-end.

### Agent W2-A — Loopback HTTP runtime endpoint

* **Spec:** Issue `Loopback HTTP runtime endpoint`.
* **Files owned:** `src-tauri/src/surface_endpoint/` (new module) — bind, lifetime, route surface, Host/Origin guards, error envelopes. Tauri `lib.rs` integration to start the endpoint as a supervised task.
* **Don't touch:** HMAC verification (W2-B owns); pairing logic (W2-C); rate-limit matrix (W2-D).
* **Done when:** endpoint binds to random loopback port; teardown kills listener; Host/Origin guards reject malformed Hosts; skeleton handlers return 401 (auth lands in W2-B/C).

### Agent W2-B — HMAC-SHA256 request signing

* **Spec:** Issue `HMAC-SHA256 request signing for the loopback transport`.
* **Depends on:** W2-A merged.
* **Files owned:** `src-tauri/src/surface_endpoint/hmac.rs` (verifier), canonicalization logic per artifact 08, freshness window + nonce-replay tables.
* **Don't touch:** transport bind/lifetime (W2-A); pairing logic (W2-C); rate-limit matrix (W2-D).
* **Done when:** every request through `/v1/surface/*` requires a valid HMAC before reaching ability dispatch; negative tests from artifact 08 §"Negative cases" + artifact 12 all green.

### Agent W2-C — Pairing handshake + four-path token recovery defenses

* **Spec:** Issue `Pairing handshake + four-path token recovery defenses`.
* **Depends on:** W2-A + W2-B merged.
* **Files owned:** `src-tauri/src/surface_endpoint/pairing.rs` (pairing service), `surface_client_pairings` + `surface_client_revocations` DB tables + migration, runtime UI pairing-code display surface (Tauri-side).
* **Don't touch:** transport (W2-A); HMAC (W2-B); rate-limit matrix (W2-D); WP-side pairing UI (W3-B).
* **Done when:** all four named defenses (Reinstall, DB-Restore, Site-Switch, Exfiltration) implemented with negative tests from artifact 12; `POST /v1/pairing/handshake` accepts pairing code → returns session material; revocation flow live.

### Agent W2-D — Rate-limit matrix in `SurfaceClientBridge`

* **Spec:** Issue `Rate-limit matrix in SurfaceClientBridge`.
* **Depends on:** W2-A + W2-B + W2-C merged.
* **Files owned:** `abilities-runtime/src/surface_bridge.rs` (bridge enforcement), token-bucket implementation, axis configs, audit log integration.
* **Don't touch:** transport (W2-A); HMAC (W2-B); pairing (W2-C); WP-side anything (W3).
* **Done when:** all five axes enforced per artifact 09; 429 response carries exhausted-axis header; ability body not invoked when rate-limit denial fires; config-driven (no float literals).

### W2 merge gate

L0 → L2 → L3 → L5 cleared per the Review Ladder. **/cso runs at L0 plan AND L2 diff for all four agents** — this is the load-bearing trust boundary for the release. Required artifacts:
* L0 plan approvals (architect-reviewer + /cso for all four; performance-engineer for W2-D rate-limit matrix sizing).
* L2 diff approvals on 4 PRs. /cso re-review on each.
* L3 wave adversarial: codex-challenge + /cso + **Suite S full re-run** (transport + auth + pairing + rate-limits as one integrated boundary; no signal leakage; no Host-header bypass; no token forgery; no nonce replay; no pairing recovery silently-resurrects path) + **Suite P** (transport latency budget; rate-limit overhead measured against ability invocation cost).
* CI invariants now active: loopback transport rejects non-127.0.0.1 Host; HMAC verification before dispatch.
* L5 drift check: integrated state matches v1.4.2 project DoD §Pairing model + §Rate-limit matrix.
* **Mandatory retro** — first transport + pairing work; security learnings critical.
* Proof bundle written.

---

# Wave 3 — WP plugin + MCP server

Three agents. **Internal staging:** W3-A (plugin skeleton) must merge before W3-B (runtime client) and W3-C (custom MCP server) start. W3-B + W3-C run in parallel after W3-A.

### Agent W3-A — DailyOS WordPress plugin skeleton

* **Spec:** Issue `DailyOS WordPress plugin skeleton + WP Abilities API registration`.
* **Files owned:** `wp/dailyos/` plugin directory (entire layout per artifact 13). PHP autoloader. `class-dailyos-plugin.php`. `class-dailyos-ability-registry.php` reading the W1-C v1.4.2 ability-surface inventory artifact. Admin page scaffolding (forms only; no handshake wiring).
* **Don't touch:** runtime-client implementation (W3-B); custom MCP server (W3-C); block code (W4); feedback router (W5).
* **Done when:** plugin activates clean on WP 6.9; admin pages render the scaffolds; abilities register via WP Abilities API per `tools/dailyos-abilities.json`; PHP_CodeSniffer + WP Coding Standards clean.

### Agent W3-B — WP-side runtime client + HMAC signer + pairing UI

* **Spec:** Issue `WP-side runtime client + HMAC signer + pairing UI`.
* **Depends on:** W3-A merged. W2 fully merged.
* **Files owned:** `wp/dailyos/includes/class-dailyos-runtime-client.php`, `class-dailyos-hmac-signer.php`. Pairing admin page wiring. Settings page read-side. The `manage_options`-gated WP filter for HMAC key retrieval.
* **Don't touch:** plugin skeleton (W3-A owns); MCP server (W3-C); blocks (W4); feedback router (W5).
* **Done when:** pairing flow works end-to-end against W2 runtime; signed requests verified by runtime HMAC; revoke control works; PHP_CodeSniffer clean.

### Agent W3-C — Custom MCP server with allowlist + dedicated low-cap WP user

* **Spec:** Issue `Custom MCP server with DailyOS allowlist + dedicated low-cap WP user`.
* **Depends on:** W3-A merged. W2 fully merged. W1-C ability-surface inventory artifact landed (v1.4.2 artifact, not v1.4.1).
* **Files owned:** `wp/dailyos/includes/class-dailyos-mcp-server.php`. WP user creation at activation (`dailyos_substrate` role + permissions). Permission callbacks. WP MCP Adapter plugin install check.
* **Don't touch:** plugin skeleton (W3-A); runtime client (W3-B); blocks (W4).
* **Done when:** custom MCP server registers under the DailyOS namespace; default WP MCP server does NOT expose DailyOS abilities (negative fixture green); allowlist enforced; permission callbacks check both WP capability + DailyOS scope.

### W3 merge gate

L0 → L2 → L3 → L4 cleared per the Review Ladder. **/cso runs at L0 + L2 for W3-B (HMAC + pairing UI handling) and W3-C (MCP allowlist + low-cap user — explicit /cso refinement P2 territory).** Required artifacts:
* L0 plan approvals (architect-reviewer for W3-A; /cso for W3-B/C; `/plan-devex-review` for W3-A).
* L2 diff approvals on 3 PRs. /cso re-review on W3-B and W3-C.
* L3 wave adversarial: codex-challenge + /cso + **Suite S** (no substrate ability exposed by default WP MCP server; dedicated low-cap user cannot reach abilities outside allowlist; pairing flow handles all four recovery paths end-to-end with real WP user + plugin reinstall scenarios).
* L4 surface QA: manual exercise of plugin install → pairing → settings page in WP Studio.
* CI invariants now active: no raw DB/FS/IPC writes from WP plugin source; no substrate ability in default WP MCP server.
* Proof bundle written.

---

# Wave 4 — Composition + first block + three-view consistency

Six agents. **Internal staging:** W4-B (concurrency contract impl) gates the producer + substrate-rule wave. W4-A0 / W4-C / W4-D / W4-E run in parallel once W4-B merges; W4-A renderer waits on W4-A0 + W4-C + W4-D + W3-B (per L0 cycle-3 codex-challenge #4).

**Stage 1 — Watermark contract (gates the producer + substrate rules):**
* **W4-B (concurrency contract impl)** — server-assigned `claim_version` and `composition_version` must exist before W4-A0 / W4-A can author + render compositions with watermarks.

**Stage 2 — Producer + parallel substrate rules (all run once W4-B merges):**
* **W4-A0 (account-overview ability — producer)** — minimal viable Composition-producing ability so W4-A has a real producer to render. Grep at L0 cycle-2 confirmed the ability does not pre-exist.
* **W4-C (tamper detection — Ed25519 signatures per Phase 0 artifact 03)** runs in parallel.
* **W4-D (custom block fallback)** runs in parallel.
* **W4-E (user-presence nonce)** runs in parallel.

**Stage 3 — Renderer (once W4-A0 producer + W4-C tamper verification + W4-D fallback rules + W3-B WP runtime client are all merged):**
* **W4-A (account-overview Gutenberg block — renderer)** consumes the W4-A0 producer + the W4-C Ed25519 verification path (for cached projections) + the W4-D substrate-side fallback projection rules (for unknown-block degradation) + the W3-B WP-side runtime client. Shipping the renderer before W4-C/D exist either renders unverified bytes or silently demotes unknown blocks.

### Agent W4-A0 — `dailyos/account-overview` ability (producer)

* **Spec:** Issue `dailyos/account-overview ability — Composition-producing substrate ability`.
* **Files owned:** `abilities-runtime/src/abilities/account_overview.rs` (new) — ability declaration via `#[ability]` macro, input/output types, internal context wiring, unit tests + eval fixture. Eval-harness fixture seed.
* **Don't touch:** Composition types (W1-E owns); concurrency (W4-B owns); WP-side rendering (W4-A consumes); HMAC/transport (W2); plugin code (W3).
* **Depends on:** W1-A (SurfaceClient actor), W1-B (AbilityPolicy schema), W1-E (Composition types), W4-B (composition_version source).
* **Done when:** ability declared with the canonical macro attrs (allowed_actors: [User, SurfaceClient], required_scopes: [read.account_overview], mcp_exposure: Invocable); deterministic composition output against the seeded account fixture; unit + eval tests green; the W4-A renderer can invoke the ability via `SurfaceClientBridge`.

### Agent W4-A — `dailyos/account-overview` Gutenberg block (renderer)

* **Spec:** Issue `dailyos/account-overview Gutenberg block (producer/renderer split)`.
* **Depends on:** W4-A0 (producer ability) + W1-E (Composition types) + W3-B (runtime client) + W4-B (concurrency contract) + **W4-C (tamper detection / Ed25519 verification for cached projections)** + **W4-D (substrate-side fallback projection rules)** merged. Added W4-C + W4-D in L0 cycle-3 per codex-challenge #4 — the renderer's "cached projection fallback" + "unknown block fallback" acceptance criteria consume those substrate primitives, and shipping the renderer before they exist either renders unverified bytes or silently demotes unknown blocks.
* **Files owned:** `wp/dailyos/blocks/account-overview/` directory — `block.json`, `render.php`, `edit.js`, `save.js`, `editor.scss`, `style.scss`. Block category registration. Cached projection support.
* **Don't touch:** concurrency contract (W4-B owns); tamper detection (W4-C); fallback projection rules (W4-D); nonce lifecycle (W4-E); feedback router (W5).
* **Done when:** block inserts, renders against real account fixture, trust bands inline, provenance click-through works, fallback per W4-D when block type unknown, watermark contract honored.

### Agent W4-B — Three-view consistency: concurrency contract implementation

* **Spec:** Issue `Three-view consistency: concurrency contract implementation`.
* **Files owned:** `abilities-runtime/src/concurrency.rs` (new), `intelligence_claims` schema migration adding `claim_version`, watermark assignment + stale-write rejection at the substrate write boundary, hybrid logical clock recording. Composition type extension to carry watermarks.
* **Don't touch:** tamper detection (W4-C owns); fallback (W4-D); nonce (W4-E); block code (W4-A).
* **Done when:** server-assigned `claim_version` works end-to-end; stale writes get `409`; concurrent writes resolve to one success + one `409`; negative fixtures from artifact 02 green.

### Agent W4-C — Tamper detection contract: projection signing + verification

* **Spec:** Issue `Tamper detection contract: projection signing + verification`.
* **Depends on:** W4-B merged (signatures include `composition_version`).
* **Files owned:** `abilities-runtime/src/tamper.rs` (signing + verification), `projection_ledger` DB table + migration (per artifact 03 §"runtime ledger"; the `projection_signatures` working name is retired), quarantine state on projection rows, tamper-event audit emission, `GET /v1/surface/keyring` route handler for SurfaceClient public-key distribution.
* **Don't touch:** concurrency (W4-B owns); fallback (W4-D); nonce (W4-E); block code (W4-A).
* **Done when:** Ed25519 signatures issued on every projection write per artifact 03 (no HMAC, no algorithm negotiation); offline verification on read detects out-of-band edits; key lifecycle (unknown-key refresh, revocation, replacement-key provisioning, queued re-sign, retired-key historical verification) all implemented per artifact 03 §"Fixture C" + §"Out-of-Band Detection"; quarantine preserves tampered state; banner on render when tamper detected; negative fixtures from artifact 03 green.

### Agent W4-D — Custom block fallback projection rules (substrate-side enforcement)

* **Spec:** Issue `Custom block fallback projection rules (substrate-side enforcement)`.
* **Files owned:** `abilities-runtime/src/fallback.rs` (projection rules), per-BlockType admitted-field sets, nearest-known-type intersection logic, audit emission for fallback events.
* **Don't touch:** Composition types (W1-E owns the type shape); WP-side rendering (W4-A consumes the rules).
* **Done when:** substrate publishes rules consumed by W4-A and any future surface; unknown-block payload fields never rendered raw; banner + claim_refs preserved; cap on unknown-block count enforced.

### Agent W4-E — User-presence nonce lifecycle

* **Spec:** Issue `User-presence nonce lifecycle for feedback writes`.
* **Depends on:** W4-B merged (nonce includes `composition_version`).
* **Files owned:** `src-tauri/src/surface_endpoint/nonce.rs` (issue + verify), in-memory nonce table with atomic consume, `/v1/surface/nonce/issue` route handler, audit emission. WP-side JS for nonce request from block editor.
* **Don't touch:** feedback router itself (W5-A owns); block render code (W4-A).
* **Done when:** nonces issued bound to `(session, wp_user_id, claim_id, field_path, action, composition_version)`; verification atomic-consumes; mismatched action / wrong field / expired / replayed all rejected with 403; audit emission live.

### W4 merge gate

L0 → L2 → L3 → L4 → L5 cleared per the Review Ladder. /cso runs at L0 + L2 for W4-C (tamper) and W4-E (nonce). `/plan-design-review` runs at L0 for W4-A (first block — design fidelity matters). Required artifacts:
* L0 plan approvals (architect-reviewer for W4-A0/B/C/D; /cso for W4-C/E; design-reviewer for W4-A; performance-engineer for W4-B watermark hot path).
* L2 diff approvals on 6 PRs. /cso re-review on W4-C and W4-E.
* L3 wave adversarial: codex-challenge + /cso + architect-reviewer + **Suite S** (three-view consistency from artifact 02 + 03 — concurrency AND tamper both green; fallback rules don't leak; nonce binding defends multi-user WP) + **Suite P** (block render latency under realistic claim-volume; watermark write path no regression).
* L4 surface QA: `/qa-only` against the first block in editor + published views. Provenance click-through + trust band rendering verified manually.
* CI invariants now active: `claim_version` server-only; projection signature verification on read.
* L5 drift check: integrated state matches v1.4.2 project DoD §Three-view consistency + §First Gutenberg block.
* **Mandatory retro** — first user-facing block wave; integration learnings critical.
* Proof bundle written.

---

# Wave 5 — Feedback + theme + negative fixtures

Three agents in parallel. W5-A (feedback router) depends on W4-A + W4-E. W5-B (theme) is independent of W5-A. W5-C (fixtures) starts at W5 kickoff and closes when all prior-wave fixtures are integrated.

### Agent W5-A — Save-time feedback router + presence-nonce-bound corrections

* **Spec:** Issue `Save-time feedback router + presence-nonce-bound corrections`.
* **Files owned:** `wp/dailyos/includes/class-dailyos-feedback-router.php`. Gutenberg save-lifecycle hooks. Diff logic for attribute changes. Feedback event translation. POST to `/v1/surface/feedback`. WP-side error handling on `409` / `403`.
* **Don't touch:** block code itself (W4-A owns); nonce issue/verify (W4-E owns); theme (W5-B); fixture catalog (W5-C).
* **Done when:** user-edited block triggers feedback event; runtime applies via existing claim/feedback service; block re-renders with corrected state; negative cases from artifact 12 §Feedback all green; CI lint asserts no direct claim-table write from plugin source.

### Agent W5-B — Magazine theme: editorial shell + tokens port + block styling

* **Spec:** Issue `Magazine theme: editorial shell + tokens port + block styling`.
* **Files owned:** `wp/dailyos-magazine/` block theme directory. `theme.json` consuming DailyOS tokens. Magazine shell parts (FolioBar, FloatingNavIsland, AtmosphereLayer, MagazinePageLayout, FinisMarker). Block style for `dailyos/account-overview`.
* **Don't touch:** plugin code (W5-A owns the feedback router; W3 owns the plugin); design-system source files (`src/styles/design-tokens.css` is the substrate token source — theme reads from it via theme.json).
* **Done when:** theme activates clean on WP 6.9; magazine shell renders; tokens drive every color/font/spacing/shadow/radius/trust-band value; light + dark modes work; Theme Check clean.

### Agent W5-C — Negative fixture catalog implementation

* **Spec:** Issue `Negative fixture catalog implementation: every named failure case from artifact 12`.
* **Start:** at W5 kickoff. **Close:** after all prior-wave fixtures are landed; this agent consolidates + integrates into the release gate.
* **Files owned:** `tests/v142_fixtures/` (or equivalent), `.docs/plans/dos-546/v1.4.2-project/fixture-catalog.md`, release-gate config extension for bundles 19-24.
* **Don't touch:** the implementations the fixtures test (those are owned by prior-wave agents).
* **Done when:** every fixture from artifact 12 has a concrete test at the boundary that fails; release gate includes bundles 19-24 mandatory; quarantine policy documented per v1.4.1 W6 gate.

### W5 merge gate

L0 → L2 → L3 → L4 cleared per the Review Ladder. `/plan-design-review` at L0 for W5-B. /cso re-runs at L2 for W5-A (feedback path crosses the trust boundary). Required artifacts:
* L0 plan approvals (architect-reviewer for W5-A; design-reviewer for W5-B; qa-expert for W5-C).
* L2 diff approvals on 3 PRs. /cso re-review on W5-A.
* L3 wave adversarial: codex-challenge + **Suite S full re-run** (feedback writes through SurfaceClient never bypass nonce; theme doesn't leak via CSS injection / SVG payload paths; fixture catalog covers all artifact-12 named cases) + **Suite E** (bundles 1-18 from v1.4.1 still green + bundles 19-24 from v1.4.2 mandatory).
* L4 surface QA: `/qa` (not `/qa-only`) — feedback flow demonstrated end-to-end with real user gesture; theme rendering verified at multiple breakpoints.
* CI invariants now active: feedback router posts only to `/v1/surface/feedback`; no direct claim-table write from plugin source.
* Proof bundle written. Retro recommended (not mandatory unless material observations).

---

# Wave 6 — Audit + clean-machine validation + release gate

Two agents in parallel. Closeout wave.

### Agent W6-A — Audit attribution: forensic exercise + CI lint + production hardening

* **Spec:** Issue `Audit attribution: forensic exercise + CI lint + production hardening`.
* **Files owned:** CI lint extension; forensic-query template doc; ADR amendment to 0111 (or small new ADR for audit attribution); migration sweep for any v1.4.0/v1.4.1 emission site discovered incomplete during W2-W5. The audit-log schema migration + `emit_surface_audit` helper are W1-A0's scope and are already merged.
* **Don't touch:** anything in W6-B's surface; the W1-A0 helper module (use it, don't rewrite it).
* **Done when:** CI lint enforces `emit_surface_audit` use on every `Actor::SurfaceClient` site; forensic exercise reproduces originator end-to-end; ADR amendment landed; bundle 24 fixtures pass; existing v1.4.0/v1.4.1 non-SurfaceClient emission preserved.

### Agent W6-B — Clean-machine validation + dev-mode runtime launcher

* **Spec:** Issue `Clean-machine validation + dev-mode runtime launcher`.
* **Files owned:** Tauri-side runtime launcher UI for pairing-code display. Bundle layout artifact (`.zip` packaging for plugin + theme; bootstrap script). `INSTALL.md`. `dailyos doctor` diagnostic command. README + onboarding copy reframe.
* **Don't touch:** anything in W6-A's surface.
* **Done when:** clean macOS test box runs the bootstrap → reaches rendered briefing ≤15 min user time; recording / stopwatch evidence in the PR; `dailyos doctor` produces the documented status report; brand reframe captured in README.

### W6 merge gate = release gate

Same shape as v1.4.0 W6 / v1.4.1 W7 release gate, expanded for v1.4.2's WP foundation scope. Required artifacts:
* L0 → L2 → L3 → L4 → L5 cleared per the Review Ladder (L5 explicit on the release: integrated state matches v1.4.2 project DoD verbatim).
* Wave 1-5 merge gates already cleared (no skipping back).
* All four threat paths from Phase 0 artifact 01 demonstrated by negative fixture + manual exercise.
* Three-view consistency demonstrated end-to-end (concurrency + tamper).
* `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green.
* WordPress plugin lints clean (PHP_CodeSniffer + WordPress Coding Standards).
* Magazine theme passes WP Theme Check.
* **`pnpm release-gate -- --mode hermetic` exits zero against bundles 1-24** (1-18 from v1.4.1, 19-24 from v1.4.2).
* **WP MCP Adapter exposure policy verified by negative fixture** (no substrate ability in default server).
* **Two-level enforcement verified by negative fixture** (insufficient scope rejected; mcp_exposure: None not enumerated).
* **Clean-machine validation captured** as screen recording or stopwatch evidence ≤15 min.
* L4 `/qa` full pass against v1.4.2 build in WordPress Studio.
* Manual dogfood evidence captured against ≥10 real-dev briefings rendered in WP.
* Brand/positioning reframe live in README + onboarding copy.
* Proof bundle written.
* **Mandatory retro** — release gate; clean-machine validation learnings.
* Tag `v1.4.2` on `trunk` after `dev` merge **gated on user release-checklist + UI validation per `feedback_no_auto_tag_without_user_validation.md`**. James walks the release checklist + validates UI hands-on before tag.

---

# Agent lane template

For each agent in W1 / W2 (the lanes where parallelism is highest), the lane spec follows this template:

```
### Agent W{N}-{X} — {Issue title}

* **Spec:** Linear issue `{title}`.
* **Linear issue body:** the canonical contract; the lane prompt IS the issue text, not a paraphrase.
* **Files owned (exclusive):** {explicit allowlist; function-level on shared files}.
* **Don't touch (deny list):** {files claimed by other agents in this wave}.
* **Depends on:** {prior-wave merges; same-wave gates if any}.
* **Done when:** {acceptance criteria from the issue body, summarized}.
* **Proof artifact:** {test name, command output, or screenshot proving done}.
```

---

# Merge gate template

For each wave, the merge gate produces these artifacts before the next wave starts:

```
## W{N} merge gate

### L-levels cleared
- [ ] L0 plan approvals (named reviewers per matrix)
- [ ] L1 self-validation per agent
- [ ] L2 diff approvals on every PR (unanimous; /cso re-review where required)
- [ ] L3 wave adversarial (codex challenge + suite S/P/E reports)
- [ ] L4 surface QA (if user-facing changes)
- [ ] L5 drift check (if release-adjacent)

### CI invariants now live
- [ ] {invariant 1}
- [ ] {invariant 2}

### Proof artifacts in `.docs/plans/wave-W{N}-v142/`
- [ ] proof-bundle.md
- [ ] retro.md (if mandatory)
- [ ] negative-fixture log per artifact 12 cases addressed in this wave

### Open items routed to next wave or maintenance
- [ ] {item} → {destination}
```

---

# Proof bundle template

Same shape as v1.4.0/v1.4.1. Each wave's `proof-bundle.md` contains:

* **Wave summary** — issues landed, agents who shipped, wall-clock.
* **L0 plan packets** — links to each issue's L0 review trail.
* **L2 diff approvals** — links to PR review summaries.
* **L3 adversarial output** — codex challenge transcript + suite report tables.
* **Negative fixtures landed** — list of artifact-12 cases now green in this wave.
* **CI invariants newly enforced** — invariant + activation commit.
* **Open items for next wave** — explicit transfer list (no silent deferrals).

---

# L0 questions — open at plan time

These are the open questions the project carries into wave kickoff. /cso refinements 1-10 from `.docs/reviews/dos-546-l0-cso-2026-05-10.md` are folded into Phase 0 artifacts and become per-issue acceptance criteria — they are NOT carried forward as open. The remainder:

1. **Artifact 11 retained (closed in L0 cycle-3).** Cycle-2 had described artifact 11 (`editable-composition-overlay.md`) as retired; cycle-3 reversed that after observing the artifact had landed at 450 lines with substantive substrate-vs-overlay taxonomy + save-handler routing rules. Artifact 11 is the design source consumed by W4-D's edit-routing acceptance bullet and by W5-A's feedback router; it remains `status: proposed` until promoted to `spec:ready` by W0-C. No L0 question outstanding.

2. **WP 7.0 client-side `executeAbility()` path (closed in L0 cycle-3).** Per cycle-3 decision (reversing cycle-2's field-collapse): `mcp_exposure: McpExposure { None | MetadataOnly | Invocable }` is the canonical tri-state for MCP enumeration, AND `client_side_executable: bool` is retained as a separate field governing SurfaceClient invocation (per Phase 0 artifact 05 lines 389-412 — the two fields govern different trust boundaries, and either may be true with the other false). The macro compile-error gate fires on `allowed_actors: [SurfaceClient]` + empty `required_scopes`; the SurfaceClientBridge runtime gate adds `client_side_executable == true`. The WP-side JS path flows through the WP plugin's PHP runtime client (W3-B PHP→loopback), not direct browser-to-runtime — the W2-A Origin guard makes absent-Origin the primary allow path (PHP curl) with `site_url`-match as defense-in-depth backup. No L0 question outstanding.

3. **Tauri UI long-term fate.** Per ADR-0129 §7, the Tauri UI's role after WP stabilizes (deprecate / power-user / thin admin) is explicitly deferred to empirical evaluation. v1.4.2 ships the Tauri app as runtime host + dev surfaces; the fate decision is NOT in this release's scope.

4. **WordPress MCP Adapter dependency.** The custom MCP server (W3-C) depends on the WP MCP Adapter plugin. Resolution: plugin activation check installs it if missing. Risk: WP MCP Adapter is young; behavior under load is unverified. Phase 1 of W3-C verifies install + read-mostly defaults work; production hardening of the adapter is upstream (Automattic / wordpress.org).

5. **`tools/dailyos-abilities.json` generation cadence.** Inventory artifact regenerates at build time. Risk: stale artifact if build skipped. Resolution: CI gate from W1-C asserts the artifact matches the source `#[ability]` annotations on every push.

6. **DB-restore defense interaction with v1.4.1 W2-D key rotation.** The pairing recovery defenses (W2-C) include a runtime-side revocation table; v1.4.1 W2-D introduces `DbKeyProvider` with key rotation. Resolution: pairing service reads through the v1.4.1 key provider; key rotation invalidates active pairings (acceptable per artifact 01 "Reinstall" defense — same anchor-rotation rules apply).

7. **WordPress Studio version pinning.** WP Studio is younger than mature OS apps (per ADR-0129 §Risks). Resolution: clean-machine validation in W6-B captures the exact WP Studio version that works; INSTALL.md documents the minimum.

These are not blockers; they are L0 talking points for the consuming wave.

---

# Final release gate

Same shape as v1.4.0 / v1.4.1 release gates; v1.4.2-specific items:

```
## v1.4.2 Release Gate

### Substrate / runtime
- [ ] `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green
- [ ] Loopback transport binds only to 127.0.0.1, Host/Origin guards enforced
- [ ] HMAC verification runs before ability dispatch; freshness window honored; nonces non-replayable
- [ ] Pairing service: all four named defenses (Reinstall, DB-Restore, Site-Switch, Exfiltration) with negative fixtures
- [ ] Rate-limit matrix enforced at SurfaceClientBridge across five axes with negative fixtures
- [ ] `claim_version` server-assigned; stale writes rejected; concurrent writes resolve correctly
- [ ] Projection signatures issued on write; verification on read; tamper events quarantined + audited
- [ ] User-presence nonces bound to (session, wp_user_id, claim_id, field_path, action, composition_version)
- [ ] Audit log carries SurfaceClient instance + wp_user_id round-trip; forensic exercise reproduces originator

### Surface / WordPress
- [ ] WP plugin activates clean on WP 6.9 Studio
- [ ] Pairing flow completes admin-side end-to-end
- [ ] Custom MCP server registers; default WP MCP server does NOT expose substrate abilities (negative fixture green)
- [ ] Dedicated `dailyos_substrate` low-cap WP user created at activation; permission callbacks check both layers
- [ ] `dailyos/account-overview` block: insert, render, edit, save, re-render with corrections, fallback projection on unknown types
- [ ] Magazine theme: shell + tokens + block styling; Theme Check clean
- [ ] Save-time feedback router: corrections route through runtime feedback path; block re-renders with corrected state
- [ ] PHP_CodeSniffer + WordPress Coding Standards clean

### Release gate
- [ ] `pnpm release-gate -- --mode hermetic` exits zero against bundles 1-24
- [ ] Every fixture from Phase 0 artifact 12 has a concrete green test
- [ ] L4 /qa full pass against v1.4.2 build in WP Studio
- [ ] Manual dogfood: ≥10 real-dev briefings rendered

### Clean-machine validation (per ADR-0129 §10)
- [ ] Clean macOS test box: install bundle → pairing → rendered briefing ≤15 min user time
- [ ] Recording / stopwatch evidence attached
- [ ] `dailyos doctor` returns documented status report
- [ ] INSTALL.md describes the install + recovery + troubleshooting flow
- [ ] Brand/positioning reframe live: README + onboarding copy describe "personal intelligence runtime"

### Linear hygiene
- [ ] Old v1.4.2 superseded; entity-intelligence scope re-projected to v1.4.3
- [ ] DOS-546 closed with proof bundle
- [ ] All v1.4.2 issues closed Done
- [ ] Per-wave proof bundles linked from this gate document
- [ ] Mandatory retros (W1, W2, W4, W6) merged

### Tag
- [ ] User release-checklist walked
- [ ] UI hands-on validation completed by James
- [ ] Tag `v1.4.2` on `trunk` after `dev` merge
```

---

# Summary

| | |
|---|---|
| Total issues | 28 (post L0 cycle-2: +1 W0-D, +1 W1-A0, +1 W4-A0 against cycle-1 baseline of 25) |
| Wave units | 7 (W0, W1, W2, W3, W4, W5, W6) |
| Total agents | 28 — W0=4 + W1=6 + W2=4 + W3=3 + W4=6 + W5=3 + W6=2 |
| Mandatory retros | W1, W2, W4, W6 |
| Estimated wall-clock | ~6 weeks across the wave ladder (cycle-2 additions land within existing wave windows; W0-D parallel to W0-A/B/C, W1-A0 in stage-1b after W1-A per cycle-3 dependency edge, W4-A0 parallel to W4-C/D/E in W4 stage-2) |
| Foundation target | Substrate is the authority; WP is the primary composable surface; three-view consistency proven (2-view-write + 3-view-read; bidirectional markdown deferred to v1.4.6); clean-machine validation ≤15 min user time including Studio install; bundles 1-24 mandatory green; brand reframe captured |
| Definition of Done | matches v1.4.2 project DoD verbatim |
