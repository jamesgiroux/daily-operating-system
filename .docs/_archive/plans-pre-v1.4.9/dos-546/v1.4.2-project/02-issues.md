# v1.4.2 — Personal Intelligence Engine: WordPress Foundation — Issue List

**Status:** Draft for James review. **Not yet in Linear.** Create after the project description and wave plan are approved.

**Convention:** Each issue carries the Core block per `.docs/SPEC-TEMPLATE.md` plus the appropriate Shape-specific block. Acceptance criteria reference Phase 0 design artifacts as the spec input; the issue body does not duplicate the artifact, it cites it.

**Phase 0 artifact paths (cited throughout):** `.docs/plans/dos-546/phase-0/01-pairing-token-recovery.md` … `15-runtime-php-to-rust-endpoint.md`.

**Issue count:** 28 issues across Wave 0 → Wave 6 (W0=4 + W1=6 + W2=4 + W3=3 + W4=6 + W5=3 + W6=2). Cycle-2 additions: +1 W0-D ADR amendment, +1 W1-A0 audit schema, +1 W4-A0 account-overview producer (against a cycle-1 baseline of 25).

**All issues start with:**
* **Labels:** `spec:draft` (start state) — promoted to `spec:ready` after L0 Prep.
* **Project:** v1.4.2 — Personal Intelligence Engine: WordPress Foundation (created after this draft is approved).
* **Team:** DailyOS.

---

## Wave 0 — Contract lock + supersession (4 issues)

---

### Adopt ADR-0130 amendments: ProvenanceRef + custom block fallback projection

**Suggested milestone:** Wave 0 — Contract Lock
**Suggested labels:** `spec:draft`, `Improvement`, `architecture`
**Estimate:** S (1-2 days)

#### Problem

ADR-0130 ships in v1.4.2-foundation form, but two Phase 0 artifacts amend it: composition provenance reference shape (artifact 06) and custom block fallback projection rules (artifact 07). The amendments need to land in the ADR text before downstream issues consume the contract.

#### Why now

Every downstream Wave 1 substrate issue (SurfaceClient, Composition types, AbilityPolicy) reads ADR-0130 as authoritative. Patching the ADR mid-wave produces a moving target. /cso refinements 9 and 10 explicitly named these as L0 commitments.

#### Scope limits

This issue is a documentation change only. It does not introduce Rust types, ability code, or migration scripts. Those land in their owning issues.

#### Acceptance criteria

* ADR-0130 §2 `Block` shape carries `provenance: ProvenanceRef`, not `ProvenanceEnvelope`. The ProvenanceRef shape is defined in the ADR per phase-0/06 §"Decision".
* ADR-0130 §3 fallback section ships the field-pointer-granularity projection rules from phase-0/07: nearest-known-type intersection, explicit "rendered as nearest known type — payload may be incomplete" banner, `claim_refs` preserved, no raw-payload field rendering.
* Each amendment cites the Phase 0 artifact it folded in.
* ADR-0130 status remains "Proposed" until the foundation work in this project lands it as "Accepted."
* ADRs 0102, 0105, 0108 cross-references updated where the amendments change the cross-ADR contract.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A. Doc change.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** N-A. The amendments preserve the existing feedback path; they do not introduce a new path.

#### Architectural surfaces touched

* [ ] Services layer — Not touched.
* [ ] Abilities contract — Not touched (the contract is amended; the implementation lands in later issues).
* [x] Provenance — Touched. ProvenanceRef shape is the load-bearing change.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [ ] Claims layer — Not touched.
* [ ] Signal granularity — Not touched.
* [ ] Migration — Not touched.
* [ ] Evaluation harness — Not touched.
* [x] Surface parity — Touched indirectly. The amendments are the substrate contract every surface (WP, MCP, future) renders against.
* [ ] Privacy rendering — Not touched.

#### Edge cases

* Empty or missing input state — N-A, doc change.
* Stale data — N-A.
* Race — N-A.
* User intent persistence — N-A.
* Cross-ADR reference drift — Handled: every amendment cites the Phase 0 artifact and updates ADRs 0102/0105/0108 cross-references in the same PR.

#### Dependencies

* Phase 0 artifacts 06 and 07 (in repo).
* ADR-0130 first revision must already exist or land in the same PR.

#### Shape A — New capability or ability

N-A. This is a doc-only change.

#### Shape — ADR amendment

* **Target ADR:** 0130.
* **Amends:** §2 Block shape (provenance field type), §3 Fallback (projection rules).
* **Source artifacts:** `.docs/plans/dos-546/phase-0/06-composition-provenance-ref.md`, `.docs/plans/dos-546/phase-0/07-custom-block-fallback-projection.md`.
* **Consequences updated:** Yes — Positive / Negative / Neutral lists revised to reflect ProvenanceRef + projection-fallback semantics.

---

### Supersede original v1.4.2 and park entity-intelligence scope for v1.4.3

**Suggested milestone:** Wave 0 — Contract Lock
**Suggested labels:** `spec:draft`, `Maintenance`, `project-hygiene`
**Estimate:** S (4 hours)

#### Problem

The original v1.4.2 Linear project ("Entity Intelligence — Accounts, Projects, People," ID `33411e87-987a-4bd0-8c88-1e9cc2a920d2`) is scoped against a Tauri React surface that ADR-0129 supersedes. The new v1.4.2 (WordPress Foundation) needs the old project's issues routed elsewhere so they aren't double-counted and don't drift.

#### Why now

This project's Wave 0 is the only point where the old v1.4.2 issues can be moved without producing ambiguity about which project owns what. Once new v1.4.2 work starts, every parallel issue in the old project becomes a "did we ship this?" question.

#### Scope limits

This issue does not delete or close any old issues. It re-projects them. The actual implementation of entity-intelligence work on the WP foundation is the v1.4.3 reframed project's scope.

#### Acceptance criteria

* New Linear project `v1.4.3 — Entity Intelligence on WP` (or equivalent) created as the parking-lot destination. Description notes this is the reframed entity scope.
* Every issue in the old v1.4.2 project re-projected to `v1.4.3 — Entity Intelligence on WP` with a comment that records the supersession, the date, and the ADR-0129 reference.
* Old v1.4.2 project moved to `Canceled` or `Superseded` state (whichever Linear permits) with a final comment linking to the new v1.4.2 and to ADR-0129.
* New v1.4.2 (this project) created with the description from `.docs/plans/dos-546/v1.4.2-project/01-project-description.md`.
* Audit comment on each re-projected issue: "Reframed onto the WordPress foundation per ADR-0129; original Tauri-React scope is parked. Re-scope at L0 in v1.4.3."

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A. Project hygiene.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** N-A.

#### Architectural surfaces touched

All N-A. This is Linear hygiene work.

#### Edge cases

* Issues in the old v1.4.2 that already started implementation — they receive an explicit comment recording the WIP state and a routing decision (continue in the old shape, or re-plan).
* Issues that are duplicate-of v1.4.1 work — confirmed closed in v1.4.1, not re-projected.

#### Dependencies

* This project's description must already be drafted (`01-project-description.md`) so the new project can be created.
* ADR-0129 accepted (already true as of 2026-05-10).

#### Shape — Maintenance / project-hygiene

* **What changes:** Linear project topology only. No code, no schema, no tests.
* **How we prove it:** Linear search for "project:v1.4.2 Entity Intelligence" returns zero open issues; new v1.4.2 (foundation) exists; v1.4.3 (entity intelligence on WP) holds the re-projected backlog.
* **Why now:** First write must come from the new v1.4.2; can't fan out issues until the topology is right.

---

### Promote Phase 0 artifacts to spec:ready and lift acceptance criteria into project-level fixture index

**Suggested milestone:** Wave 0 — Contract Lock
**Suggested labels:** `spec:draft`, `Maintenance`, `documentation`
**Estimate:** S (1 day)

#### Problem

Phase 0 produced 14 design artifacts that name acceptance criteria informally. Wave 1+ implementation issues need a citable index of which artifact provides the contract for which acceptance criterion so reviewers can verify alignment without re-reading 7,500 lines of design doc per L0 cycle.

#### Why now

Every downstream issue in this project's body cites Phase 0 artifacts by file path. A project-level index lets `/plan-eng-review` and L2 reviewers find the contract source in seconds, not minutes.

#### Scope limits

This issue does not modify Phase 0 artifact content. It produces a single index doc that maps acceptance areas → artifact source. All 15 artifacts (01-15) have landed and are `status: proposed`; this issue promotes them to `spec:ready`.

#### Acceptance criteria

* `.docs/plans/dos-546/phase-0/INDEX.md` created with one row per Phase 0 artifact: file path, owning sub-contract, downstream issue that consumes it, status.
* Each Phase 0 artifact frontmatter `status` updated from `draft`/`proposed` to `ready` if no open question remains, or annotated with the open question.
* Artifact 11 (`11-editable-composition-overlay.md`) is `spec:ready` at 450 lines and carries substantive content (substrate-vs-overlay block taxonomy, edit-routing rules, paste/nesting/reorder semantics across the boundary). INDEX.md records artifact 11 alongside the other 14 artifacts. W4-D + W5-A acceptance criteria cite artifact 11 as the design source for editable-composition-overlay semantics; artifacts 07 (substrate-side fallback projection rules) + 14 (block consume path) + 10 (presence-nonce-bound feedback) remain the orthogonal load-bearing artifacts they always were.
* PR description includes a grep-verified list of which `.docs/plans/dos-546/phase-0/*.md` files are referenced from which issue body.

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A. Documentation-only.

#### Architectural surfaces touched

All N-A.

#### Edge cases

* Artifact 11 is part of the spec set — INDEX.md records artifact 11 alongside artifacts 07/10/14 and promotes it to `spec:ready`; W4-D and W5-A cite it as the design source for editable-composition-overlay semantics.
* Phase 0 artifact ships an open question that downstream relies on — flagged in INDEX.md and routed to L0 Prep for the consuming issue.

#### Dependencies

* All 15 Phase 0 artifacts in repo (01-15).

#### Shape — Maintenance

* **What changes:** new doc + frontmatter status fields.
* **How we prove it:** `rg "status: draft" .docs/plans/dos-546/phase-0/` returns only artifacts with open questions; INDEX.md round-trips reference cleanly to every issue in this project.
* **Why now:** prevents L0 cycle-2 reviewers from "can't find the contract source" findings.

---

### ADR-0102 §7.1 amendment: promote `mcp_exposure` to tri-state enum and retain `client_side_executable`

**Suggested milestone:** Wave 0 — Contract Lock
**Suggested labels:** `spec:draft`, `Improvement`, `architecture`, `trust-boundary`
**Estimate:** S (1 day)

#### Problem

ADR-0102's 2026-05-10 amendment line declares `mcp_exposure: bool`. ADR-0111 §8 echoes the bool form. The v1.4.2 W1-B canonical schema work + Phase 0 artifact 05 ship a tri-state `McpExposure { None | MetadataOnly | Invocable }` to capture the WP MCP server's "describe-but-don't-invoke" tier. The tri-state is the correct design (architect A1 + challenge #1 + /cso refinement P2 + consult §"do not defer client_side_exposure" all converge), but the ADR text is stale. Artifact 05 lines 383-412 explicitly resolves the EoP P2 naming concern by **keeping `mcp_exposure` and `client_side_executable` separate**: they govern different trust boundaries. `mcp_exposure` controls the network-facing MCP tool surface (where a host agent discovers tools and sends JSON input); `client_side_executable` controls whether a trusted in-product SurfaceClient (e.g., a WP block hydrating a composition) may invoke the ability after WordPress capability + runtime scope + actor checks pass. The two fields can disagree: a WP block may need to hydrate an ability that should never be MCP-listed, and an MCP tool may be invocable by an agent while the WP client only renders returned data and does not call the ability directly.

#### Why now

W1-B implements the canonical AbilityPolicy schema and will land code that contradicts the ADR text if the ADR is not amended first. Two-source schemas drift; pinning the canonical source before W1-B starts closes the loop. L0 cycle-3 reversed the cycle-2 collapse of `client_side_executable` into `mcp_exposure` after re-reading artifact 05's explicit warning — the cycle-2 collapse would have pushed WP renderability toward MCP exposure, which is the exact rigidity Phase 0 warned against.

#### Scope limits

This issue is a documentation change only. It amends ADR-0102 §7.1 and aligns artifact 05's status note with the amended ADR. No Rust types change in this issue (those land in W1-B).

#### Acceptance criteria

* ADR-0102 §7.1 amended: `mcp_exposure: bool` → `mcp_exposure: McpExposure` where `McpExposure = None | MetadataOnly | Invocable`. The amendment note records the cycle-3 reversal of the cycle-2 collapse after re-reading artifact 05.
* ADR-0102 §7.1 retains `client_side_executable: bool` as a separate field with default `false`. Per artifact 05 lines 389-412: `mcp_exposure` governs MCP tool-surface enumeration; `client_side_executable` governs SurfaceClient invocation after policy + scope + actor checks. The fields are independent; either can be true while the other is false.
* ADR-0102 §7.6 + default-policy bullet rewritten to consume the tri-state for MCP: default is `McpExposure::None`; `MetadataOnly` enumerates name + description; `Invocable` enumerates full schema. Default `client_side_executable: false` preserves v1.4.0/v1.4.1 behavior.
* ADR-0102 default-policy bullet clarified: abilities exposed to SurfaceClient (i.e., `allowed_actors` includes `SurfaceClient`) MUST declare non-empty `required_scopes`, OR explicitly opt out with `#[ability(..., no_scope_required)]`. The macro must compile-error if both are absent. The compile-error gate applies regardless of `client_side_executable` value — actor membership in `allowed_actors` is the trigger, because that is what makes the ability reachable from a SurfaceClient at all.
* Phase 0 artifact 05 status note updated: the inventory schema continues to carry both `mcp_exposure` (tri-state) and `client_side_executable` (bool) per the amended ADR-0102. The field-name `client_side_exposure` remains intentionally not used per artifact 05 line 408.
* ADR-0111 §8 cross-reference updated where the bool `mcp_exposure` form appears; the §8 actor-attribution semantics for SurfaceClient are unchanged.
* ADR-0102 status remains "Accepted" (the cycle-2/cycle-3 amendments are refinements, not a re-decision).

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A. Documentation change.

#### Architectural surfaces touched

* [x] Abilities contract — Touched. Canonical schema field type changed.
* [x] Provenance — Touched indirectly (policy gate records exposure tier).
* [x] Surface parity — Touched. One canonical schema across all surfaces.
* [x] Privacy rendering — Touched indirectly. `MetadataOnly` is the privacy-rendering tier for MCP introspection.
* All others — Not touched.

#### Edge cases

* Stale ADR-0102 readers consuming the bool form — Handled: amendment line + git history + W0-D PR description explicitly bridges.
* Artifact 05 schema drift — Handled: artifact 05 status note pins canonical source to ADR-0102.
* Default-policy regression — Handled: default is `McpExposure::None` (closed by default, same as bool false).

#### Dependencies

* ADR-0102 (in repo).
* Phase 0 artifact 05 (in repo).
* W0-A (ADR-0130 amendments) — independent; can land in parallel.

#### Shape — ADR amendment

* **Target ADR:** 0102.
* **Amends:** §7.1 (schema field type), §7.6 (default policy + introspection semantics).
* **Source artifacts:** L0 cycle-1 architect-reviewer finding A1, codex-challenge finding #1, /cso refinement P2, codex-consult §"do not defer client_side_exposure to W3-B", L0 cycle-3 codex-challenge CRITICAL #1 (reversed the cycle-2 collapse after re-reading artifact 05 lines 389-412).
* **Consequences updated:** Yes — Positive (one canonical source across runtime, WP MCP, SurfaceClient; tri-state captures MetadataOnly tier; field separation matches Phase 0 design intent), Negative (one more enum variant), Neutral (artifact 05 schema unchanged in shape).

---

## Wave 1 — Substrate-side contract (6 issues)

---

### Audit-log schema for SurfaceClient attribution (additional columns + emission contract)

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`, `audit`, `abilities-runtime`

**Estimate:** M (2-3 days)

#### Problem

Per /cso refinement 4 + ADR-0111 §8, every substrate operation log entry must carry SurfaceClient instance identity AND WP `user_id` (or `None`/`null` for non-SurfaceClient calls). If the audit-log schema lands in W6 (closeout), every W2-W5 audit emission has to be retrofitted — and the codex-challenge + codex-consult cycle-1 panels both flagged this as the structural gap: W2-C pairing events, W2-D rate-limit denials, W3-C MCP invocations, W4-E nonce events, W5-A feedback applications all emit audit events.

#### Why now

The audit fields must be on the schema before the first W2 audit emission ships. Otherwise the L2 CI lint enforced in W6-A has nothing to enforce against in W2-W5, and the path-α "honestly emit attribution" claim is partial.

#### Scope limits

This issue lands the schema migration + the canonical emission helper + the round-trip plumbing skeleton. It does NOT land the forensic-query exercise, the CI lint against `Actor::SurfaceClient` log sites, or the migration of every existing v1.4.0/v1.4.1 emission site — those are W6-A scope (forensic + CI lint + production hardening). W1-A0 establishes the rails; W6-A drives them home.

#### Acceptance criteria

* `audit_log` schema gains two columns: `actor_instance: TEXT NULL` (serialized `SurfaceClientId`), `wp_user_id: INTEGER NULL`. Index on `(wp_user_id, created_at)` for forensic query throughput. Migration is additive; existing rows default to `NULL`.
* Canonical helper `emit_surface_audit(event_kind: &str, actor: &Actor, fields: AuditFields) -> Result<()>` lands in the audit module. When `actor` is `Actor::SurfaceClient { instance, .. }`, the helper MUST populate `actor_instance` AND require `fields.wp_user_id: Some(_)`; calling the helper for a SurfaceClient actor without `wp_user_id` is a compile-time or runtime contract error per the `AuditFields` builder.
* Round-trip plumbing skeleton: WP plugin includes `wp_user_id` in every request (Wave 3 ships the WP side); endpoint extracts the field from request headers/body; `SurfaceClientBridge` propagates into the actor; service-layer emission calls `emit_surface_audit` with `wp_user_id` from the actor's request context.
* The W2-C pairing event, W2-D rate-limit denial, W3-C MCP invocation, W4-E nonce event, and W5-A feedback application acceptance criteria all bind to `emit_surface_audit` (not to direct `audit_log` row writes).
* Non-SurfaceClient emission sites (existing v1.4.0/v1.4.1 user/agent/system code paths) continue to call the existing emission paths; `actor_instance` and `wp_user_id` default to `NULL` for those rows.
* Forensic-query tooling skeleton: a `select` template documented showing how to derive `wp_user_id` from a sample feedback event row. Full forensic exercise lands in W6-A.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Indirect — audit emission is downstream of signal application.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Touched indirectly — feedback application audit row now carries both fields once W5-A wires through.

#### Architectural surfaces touched

* [x] Services layer — Touched (emission helper).
* [x] Provenance — Touched directly.
* [x] Trust-boundary — Touched directly.
* [x] Migration — Touched (new columns + index).
* [x] Surface parity — Touched.
* All others — Not touched.

#### Edge cases

* Empty `wp_user_id` for `Actor::SurfaceClient` — runtime contract error in `AuditFields` builder; W2 endpoint rejects malformed WP requests that omit `wp_user_id` per artifact 12.
* Stale `wp_user_id` (user deleted between request and emission) — recorded as observed; downstream tools resolve `NULL` if user no longer exists.
* Null `actor_instance` for `Actor::SurfaceClient` — runtime contract error.
* Race: emission helper called concurrently for different actors — Handled per existing audit-log concurrency rules.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class` (W1-A) — **must merge first.** The `emit_surface_audit` helper pattern-matches `Actor::SurfaceClient { instance, .. }`, and W1-A is the issue that introduces that variant; without it, the helper signature does not compile. Updated in L0 cycle-3 from "same wave, stage-1, independent" — the file boundary is preserved (W1-A0 owns `audit_log` + helper module; W1-A owns `actor.rs`) but the type boundary is W1-A → W1-A0 strict.
* Phase 0 artifact 04 (host inventory).
* /cso refinement 4 + ADR-0111 §8.

#### Shape B — Schema or data-model change

* **New tables or columns:** `audit_log.actor_instance: TEXT NULL`, `audit_log.wp_user_id: INTEGER NULL`. Index on `(wp_user_id, created_at)`.
* **Append-only vs mutate-in-place:** append-only (existing audit-log semantics preserved).
* **Negative knowledge / tombstones:** N-A.
* **Pruning policy:** existing audit-log retention preserved.
* **Read path:** new fields surfaced in audit queries.
* **Write path:** `emit_surface_audit` helper is the canonical writer for SurfaceClient-actor emissions; existing emission paths preserved for non-SurfaceClient actors.

#### Shape C — Migration

* **Parallel-run plan:** N-A — additive columns with NULL default.
* **Divergence monitor:** N-A.
* **Cutover criteria:** N-A.
* **Rollback path:** drop the new columns + index; old read/write paths unaffected.
* **Backfill strategy:** none required; `NULL` for existing rows is correct (pre-SurfaceClient events have no SurfaceClient identity).

---

### Implement `SurfaceClient` as the fourth actor class

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `trust-boundary`
**Estimate:** M (3-4 days)

#### Problem

ADR-0111 §8 names `SurfaceClient` as a new actor class for paired surface bridges, but the substrate today has three actor classes (user, agent, system) and no per-instance identity, scope, or audit attribution for surface-paired clients. Wave 2 transport, Wave 3 plugin, and Wave 4 blocks all need `SurfaceClient` to exist before their L0 plans can be written.

#### Why now

This is the load-bearing substrate type for the WordPress foundation. Every Wave 2+ issue depends on `Actor::SurfaceClient { instance, scopes }` being a real, registry-recognized actor.

#### Scope limits

This issue lands the actor class and its identity/scope shape. It does not land the WP-side bridge (Wave 2), the pairing handshake (Wave 2 issue), the rate-limit matrix (Wave 2 issue), or any block rendering (Wave 4). Out of scope: changes to `Actor::Agent` or `Actor::System` semantics.

#### Acceptance criteria

* `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }` lands in the abilities-runtime crate.
* `SurfaceClientId` is a typed wrapper that survives serialization across the audit boundary; debug-printing it produces a stable, non-PII representation.
* `ScopeSet` is a typed set of `SurfaceScope` values; the registry rejects scopes outside the defined enum at deserialization.
* The actor class flows through every existing actor-aware site without compile-error fallout: ability invocation, registry lookup, provenance attribution, audit log emission, signal subject-ownership checks.
* Negative test: an ability marked `allowed_actors: [User, Agent]` rejects a `SurfaceClient` invocation at the registry boundary before the ability body runs.
* Negative test: a `SurfaceClient` invocation outside `required_scopes` is rejected at the bridge boundary before registry lookup.
* No `unwrap()` or `panic!()` on actor-class branching; pattern matches are exhaustive.
* Audit log emission records `actor_kind = "surface_client"`, `actor_instance = <SurfaceClientId>`, `actor_scopes = <serialized ScopeSet>` for every operation invoked by `Actor::SurfaceClient`.
* L0 Prep packet cites `.docs/decisions/0111-surface-independent-ability-invocation.md` §8 as the binding contract and Phase 0 artifact 05 as the inventory of which abilities can target `SurfaceClient`.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched indirectly — signal subject-ownership checks need to accept `SurfaceClient` correctly. The signal-bus contract from v1.4.1 W1-B/E remains the authoritative routing layer.
* **Health scoring:** Not touched.
* **Intel context:** `build_intelligence_context()` and `gather_account_context()` continue to consume the existing actor class; SurfaceClient calls them through the runtime, not directly.
* **Briefing callouts:** N-A in this issue. Surfaces consume callouts in Wave 4.
* **Feedback hook:** Feedback writes from a `SurfaceClient` route through the existing claim/feedback path with `actor = Actor::SurfaceClient`; no new feedback path.

#### Architectural surfaces touched

* [x] Services layer — Touched. Service functions that branch on `Actor` get a new arm.
* [x] Abilities contract — Touched. Registry pattern-matches actor.
* [x] Provenance — Touched. Provenance envelope records `actor_kind = "surface_client"` and `actor_instance`.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [ ] Claims layer — Not touched (claim write path is the same; the actor branch is upstream).
* [x] Signal granularity — Touched. Signal subject-ownership checks must accept `SurfaceClient` correctly; v1.4.1 W1-B policy registry contract preserved.
* [ ] Migration — Not touched.
* [x] Evaluation harness — Touched. Eval fixtures gain a `SurfaceClient` actor case for negative tests.
* [x] Surface parity — Touched directly. This is the surface parity primitive.
* [x] Privacy rendering — Touched. ADR-0108 sensitivity rules now have a fourth actor class to render against; the rendering is per-scope, not just per-actor-kind.

#### Edge cases

* Empty `ScopeSet` — rejected at construction (a SurfaceClient with no scopes is not a paired surface; it is a misconfiguration).
* Stale `SurfaceClientId` (paired then revoked) — Handled: revocation check at bridge boundary; covered by negative fixture from artifact 12.
* Null `actor_instance` in audit log — rejected; audit emission is non-optional for `SurfaceClient`.
* Race: concurrent invocations from the same `SurfaceClientId` with different scope grants — Handled: pairing handshake (Wave 2) defines the canonical scope grant; bridge reads the current grant per request.
* User intent persistence — N-A.
* Value instability — N-A (actor class is deterministic).
* Revoked source — overlaps with stale `SurfaceClientId`; same handling.

#### Dependencies

* Phase 0 artifact 04 (runtime-host inventory) — explicit at L0 plan time.
* ADR-0111 §8 (already accepted).
* v1.4.0 `Actor` enum — extension point.
* This issue's PR merges before any Wave 2 issue starts implementation.

#### Shape A — New capability or ability

* **Category:** Substrate primitive, not an ability per se. The new actor class is a registry-consumed type.
* **Call-graph effect:** N-A — the actor class itself has no call-graph; it is matched against by every ability.
* **Input type:** N-A.
* **Output type:** N-A.
* **Composition:** N-A.
* **Consumers:** Wave 2 SurfaceClientBridge, Wave 3 WP plugin, Wave 4 Gutenberg blocks, Wave 5 feedback router, audit log emission, eval harness.

---

### Promote `AbilityPolicy` to canonical schema: `required_scopes` + `mcp_exposure`

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `trust-boundary`
**Estimate:** M (3-4 days)

#### Problem

ADR-0102 §7.1 names `AbilityPolicy` with `allowed_actors` only; §7.6 names `mcp_exposure`; the cycle-2 amendments add `required_scopes` as the two-level enforcement field. Today the substrate enforces `allowed_actors` only. Without `required_scopes` and `mcp_exposure` as canonical schema fields, every paired surface is one-bit allow/deny — no per-ability scope discipline, no metadata-only exposure tier, no WP MCP allowlist enforcement.

#### Why now

WP MCP Adapter exposure policy (ADR-0129 §4) requires `mcp_exposure` to be the gate. WP `SurfaceClient` scope enforcement requires `required_scopes`. Both are blockers for Wave 3 (WP plugin) and Wave 4 (block invocation).

#### Scope limits

This issue lands the schema and the enforcement; it does not populate every existing ability's `required_scopes`. Population is an ability-by-ability change with named defaults — those land in the abilities' own issues. Default for unspecified `required_scopes`: empty set (no scope required, equivalent to today's behavior). Default for unspecified `mcp_exposure`: `none`.

#### Acceptance criteria

* `AbilityPolicy` struct gains `required_scopes: Vec<SurfaceClientScope>`, `mcp_exposure: McpExposure`, and `client_side_executable: bool` fields per ADR-0102 §7.1 + §7.6 (as amended in W0-D).
* `McpExposure` enum: `None | MetadataOnly | Invocable` — names match the tri-state landed by W0-D in ADR-0102 §7.1. `client_side_executable: bool` is a separate field governing SurfaceClient invocation after policy/scope/actor checks pass, per artifact 05 lines 389-412 — the two fields govern different trust boundaries and either may be true with the other false.
* The `#[ability]` macro accepts `required_scopes = [...]`, `mcp_exposure = ...`, `client_side_executable = bool`, and `no_scope_required` attributes; missing attributes default to empty `Vec<SurfaceClientScope>::new()` / `McpExposure::None` / `false` respectively.
* **Macro compile-error invariant:** the macro fails to compile any ability whose `allowed_actors` includes `SurfaceClient` AND `required_scopes` is empty AND `no_scope_required` is not explicitly set. This codifies ADR-0102 §7.6 "macro attribute must declare required_scopes explicitly" as a compile-time gate, not a code-review trust. Default `required_scopes = vec![]` remains valid for abilities whose `allowed_actors` does NOT include `SurfaceClient` (preserves v1.4.0/v1.4.1 compat).
* `SurfaceClientBridge` enforces `required_scopes` against `Actor::SurfaceClient { scopes }` at the bridge boundary before registry lookup. Mismatch returns a typed `PolicyError::InsufficientScope` with the missing scope set named (for audit only — error surface does not leak the requirement to unauthorized callers per ADR-0102 §7.4).
* `SurfaceClientBridge` also enforces `client_side_executable == true` for `Actor::SurfaceClient` invocations; an ability with `client_side_executable: false` (the default) is rejected at the bridge even if `allowed_actors` includes `SurfaceClient` and `required_scopes` are satisfied. Returns `PolicyError::ClientInvocationDisabled`. This honors the artifact 05 field separation: an ability may declare `allowed_actors: [User, SurfaceClient]` for audit/identity purposes while reserving SurfaceClient invocation behind `client_side_executable`.
* MCP introspection (the `list_tools` / `list_abilities` surface) filters by `mcp_exposure`: `None` abilities are not enumerated, `MetadataOnly` enumerates name + description but not invoke schema, `Invocable` enumerates full schema. WP MCP Adapter exposure policy consumes the same field for the WP-mediated MCP server.
* Negative test: ability `mcp_exposure: None` does not appear in any MCP-bridge response, including the WP MCP server's `list_tools`.
* Negative test: ability `required_scopes: [WriteClaims]` invoked by `SurfaceClient` with `scopes: [ReadClaims]` returns `403 PolicyError::InsufficientScope`; the ability body is not invoked.
* Negative test (compile-time): an ability declaration with `allowed_actors: [User, SurfaceClient]`, empty `required_scopes`, and no `no_scope_required` attribute fails to compile with a named macro error.
* Audit log emission records `policy_check_result` per request: `accepted`, `rejected_actor_kind`, `rejected_scope`, `rejected_mcp_exposure`.
* Backward compatibility: every v1.4.0/v1.4.1 ability continues to compile and pass tests with default field values.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Not directly. Policy is upstream of signal emission.
* **Health scoring:** Not touched.
* **Intel context:** Not touched.
* **Briefing callouts:** Not touched.
* **Feedback hook:** Indirect — `WriteFeedback` scope becomes a named `SurfaceScope` value that gates corrections from `SurfaceClient`.

#### Architectural surfaces touched

* [x] Services layer — Touched. Services that invoke abilities via the registry now flow through the two-level gate.
* [x] Abilities contract — Touched. Canonical schema extension.
* [x] Provenance — Touched. Provenance records the policy gate that accepted the call.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [ ] Claims layer — Not touched directly; the gate is upstream of claim writes.
* [ ] Signal granularity — Not touched.
* [ ] Migration — Not touched (default values preserve v1.4.0/v1.4.1 behavior).
* [ ] Evaluation harness — Touched. Eval fixtures gain policy-rejection cases.
* [x] Surface parity — Touched. This is the two-level enforcement substrate every surface consumes.
* [x] Privacy rendering — Touched. ADR-0108 sensitivity gating now composes with `mcp_exposure`.

#### Edge cases

* Empty `required_scopes` — allowed; means "no scope required."
* Stale scope grant (SurfaceClient paired, scope later revoked) — Handled: bridge reads current grant per request.
* Null `mcp_exposure` — defaults to `None`; the default is the safest position.
* Race: scope grant revoked mid-request — Handled: the bridge re-checks at request boundary, not at pairing boundary; revocation takes effect on the next request.
* User intent persistence — N-A.
* Value instability — policy decisions must be deterministic given the same actor and policy; covered by replay determinism tests.
* Revoked source — N-A; this is policy-layer, not source-layer.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class` (must merge first).
* Issue: W0-D `ADR-0102 §7.1 amendment: harmonize mcp_exposure to tri-state` (must merge first — W1-B implements the amended schema).
* Phase 0 artifact 05 (ability-surface inventory format; consumed in the W0-D-amended form).
* ADR-0102 §7.1 + §7.6 (as amended by W0-D).

#### Shape A — New capability or ability

* **Category:** Substrate primitive. Schema change on an existing type.
* **Call-graph effect:** N-A (policy is upstream of call-graph).
* **Input type:** N-A.
* **Output type:** N-A.
* **Composition:** N-A.
* **Consumers:** SurfaceClientBridge (Wave 2), MCP bridges (existing + new WP-mediated), ability registry introspection, eval harness, audit log.

#### Shape B — Schema or data-model change

* **New tables or columns:** None. The schema change is on the in-memory `AbilityPolicy` struct that ships in the abilities-runtime crate. No DB migration.
* **Append-only vs mutate-in-place:** N-A — schema is in-code, not in-data.
* **Negative knowledge / tombstones:** N-A.
* **Pruning policy:** N-A.
* **Read path:** every registry lookup now reads `required_scopes` and `mcp_exposure`; default values preserve v1.4.0 behavior.
* **Write path:** policy is `#[ability]` macro authorship; the macro is the only writer.

---

### Ability-surface inventory format + CI gate

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `dx`
**Estimate:** M (3 days)

#### Problem

WordPress Abilities API registration, MCP tool registration, and SurfaceClient introspection all need to describe each ability in different shapes derived from the same source. Today there is no canonical inventory format; each surface invents its own.

#### Why now

Wave 3 (WP plugin) reads the inventory to register abilities; Wave 3 (custom MCP server) reads the inventory to set allowlist; Wave 4 (block code) reads the inventory to find renderable abilities. Without a single canonical format, three surfaces drift and the ability-description PII/vocabulary CI gate has nothing to scan.

#### Scope limits

This issue lands the inventory format (one TypeScript interface + the matching Rust struct) and the CI gate that asserts every `#[ability]`-annotated function ships an inventory entry. It does not populate the inventory for every existing ability — that lands per-ability in the abilities' own issues, with sensible defaults for v1.4.0/v1.4.1 abilities.

#### Acceptance criteria

* Phase 0 artifact 05's `AbilitySurface` TypeScript interface lands as a frontend type and matching Rust struct in the abilities-runtime crate.
* Each ability declares its surface entry inline (via the `#[ability]` macro) or in a sibling `inventory.toml` per the format in artifact 05.
* CI gate: a build-time check enumerates every `#[ability]`-annotated function and asserts each has an inventory entry. Missing entry = build failure.
* Inventory entries serialize to a `tools/dailyos-abilities.json` artifact consumed by:
  * WP plugin's `class-dailyos-ability-registry.php` at install/activation
  * Custom MCP server's allowlist
  * SurfaceClient introspection (`list_tools` filtered by `mcp_exposure`)
* Inventory schema fields per artifact 05 §"Canonical TypeScript Interface": `name`, `category`, `actor`, `mcp_exposure`, `client_side_executable`, `idempotency_class`, `composition`, `required_scopes`, `description`, `display`.
* The `description` field is the model-facing + user-facing copy and is scanned by the CI gate in the next issue.
* **Inventory schema is additive-only across consuming releases** (per architect-reviewer cycle-1 D4). Consuming releases (Skillify `DOS-540`, v1.4.7 MCP v2) extend the inventory schema with new optional fields; they do NOT break the v1.4.2 contract. The CI gate asserts this by versioning the JSON Schema and refusing PRs that remove or rename existing fields.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Indirect — the inventory names which abilities expose feedback paths so the WP save handler can route corrections.

#### Architectural surfaces touched

* [x] Abilities contract — Touched. Inventory is a contract-adjacent schema.
* [x] Provenance — Touched indirectly. Ability descriptions are part of provenance rendering.
* [x] Surface parity — Touched. One inventory, three surfaces.
* [x] Privacy rendering — Touched. The description field enters the PII regime.
* All others — Not touched.

#### Edge cases

* Empty inventory entry — rejected at CI gate.
* Description field with PII — rejected at the description-CI-gate issue (next).
* Race: two abilities with the same `name` — rejected at registry construction (already true).
* Inventory entry references a `composition` block type not in the `Composition` contract — CI gate flags it.

#### Dependencies

* Phase 0 artifact 05.
* Issue: `AbilityPolicy required_scopes + mcp_exposure` (must merge first; inventory entries reference both fields).

#### Shape A — New capability or ability

* **Category:** Maintenance / dx primitive.
* **Consumers:** WP plugin, custom MCP server, SurfaceClient introspection, eval harness, every L0 review.

---

### Ability-description CI gate: PII blocklist + internal-vocabulary scan

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `trust-boundary`, `dx`
**Estimate:** S (1-2 days)

#### Problem

Ability descriptions are model-facing copy, user-facing copy, and generator-facing copy. They are committed source, but the existing PII blocklist + internal-vocabulary scan (per `.claude/hooks/pre-commit-gate.sh`) does not currently scan them as a separate category. Phase 0 /cso refinement 6 named this gap.

#### Why now

Wave 1 lands `AbilityPolicy` and the inventory format. The descriptions land in Wave 2+ ability authorship. If the gate is not active before Wave 2, descriptions drift and the gate becomes retroactive cleanup.

#### Scope limits

This issue extends the existing `.claude/hooks/pre-commit-gate.sh` regime to ability descriptions. It does not introduce new blocklist terms or vocabulary rules — those are owned by the existing gate config files.

#### Acceptance criteria

* The pre-commit gate (`.claude/hooks/pre-commit-gate.sh`) and any sibling CI lint scan every `#[ability]`-annotated `description` field and every `inventory.toml` `description` entry for the existing PII blocklist (`.claude/pii-blocklist.txt`) and the internal-vocabulary lint (per `feedback_no_pii_in_commit_messages` and product-vocabulary ADR-0083).
* Violation = commit refused (locally) and CI failure (CI).
* The gate runs against the serialized `tools/dailyos-abilities.json` artifact so generated descriptions are also scanned.
* Test fixture: a deliberately-violating description (e.g., contains a blocklist term) is rejected; a clean description passes.
* Documentation in `CLAUDE.md` updated to name ability descriptions as a scanned surface.

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A — this is a discipline gate, not a substrate change.

#### Architectural surfaces touched

* [x] Abilities contract — Touched (descriptions live on the contract).
* [x] Privacy rendering — Touched (PII discipline).
* All others — Not touched.

#### Edge cases

* Description with a legitimate term that happens to match the blocklist — escape hatch is the same as for any false positive: explicit allowlist annotation per existing protocol. No silent bypass.
* Description that lints clean but is misleading — out of scope; this gate enforces PII + vocabulary, not accuracy.

#### Dependencies

* Issue: `Ability-surface inventory format + CI gate`.
* Existing `.claude/hooks/pre-commit-gate.sh` infrastructure.

#### Shape — Maintenance / discipline gate

* **What changes:** the gate scans a new file category.
* **How we prove it:** fixture test rejects deliberately-violating description; existing v1.4.0/v1.4.1 ability descriptions still pass.
* **Why now:** before Wave 2 authorship lands new descriptions.

---

### `Composition` contract substrate types + `ProvenanceRef` shape

**Suggested milestone:** Wave 1 — Substrate Contract
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `composition`
**Estimate:** L (5-7 days)

#### Problem

ADR-0130 defines `Composition` as the surface-independent block tree the substrate ships. Today there is no Rust type for `Composition`, no `Block` type, no `ProvenanceRef`. Every Wave 2-5 issue downstream needs these types.

#### Why now

This is the load-bearing producer/renderer split that lets WordPress (and any future surface) render substrate content without re-implementing the substrate. Without `Composition` types, blocks store frozen HTML and the surface becomes a second authority.

#### Scope limits

This issue lands `Composition`, `Block`, `BlockType`, `ProvenanceRef`, the projection-fallback rules per Phase 0 artifact 07, and the `claim_refs` shape per artifact 06. It does not land the loopback HTTP endpoint, the WP-side render code, or any concrete ability that produces a `Composition` — those land in Waves 2-4.

#### Acceptance criteria

* `Composition { id: CompositionId, version: CompositionVersion, blocks: Vec<Block> }` in the abilities-runtime crate.
* `Block { type: BlockType, attributes: BTreeMap<String, Value>, claim_refs: Vec<ClaimRef>, provenance: ProvenanceRef }` per ADR-0130 §2 amended.
* `BlockType` enum covers the values in Phase 0 artifact 05's `CompositionBlockType`: `account_overview`, `claim_summary`, `evidence_list`, `health_snapshot`, `relationship_map`, `risk_callout`, `action_list`, `markdown_document`, `custom { type_id: String }`.
* `ProvenanceRef { invocation_id: InvocationId, field_path: FieldPath }` resolves against the ability output's top-level `Provenance` envelope. The full envelope lives once on `AbilityOutput<T>`; `ProvenanceRef` is the load-bearing shape that prevents the 64KB ADR-0108 cap from being violated by composition output.
* Fallback projection per artifact 07: unknown `BlockType::Custom { type_id }` renders via the nearest-known-type projection at JSON-pointer granularity, with the explicit banner, `claim_refs` preserved, no raw-payload field rendering.
* `Composition` serializes/deserializes round-trip stably; `composition_version` is a server-assigned watermark per Phase 0 artifact 02.
* Unit tests for: nominal block tree, fallback projection on unknown block type, `ProvenanceRef` resolution, watermark monotonicity, claim_refs preservation under fallback.
* `Composition` is produced ONLY by abilities (ADR-0130 §1, substrate-owned authorship); a CI lint asserts no non-ability code constructs `Composition` directly.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Indirect — `Composition` re-renders on signal-driven invalidation. The signal/job/claim chain from v1.4.1 W1 is the upstream invalidator.
* **Health scoring:** Not touched in this issue.
* **Intel context:** `build_intelligence_context()` is the existing intel context surface; `Composition` is the substrate-side output of abilities that consume that context. No edit to context building in this issue.
* **Briefing callouts:** `risk_callout` BlockType is the callout surface for compositions; concrete callout abilities land in v1.4.3+.
* **Feedback hook:** `Composition` carries the claim_refs the WP save handler routes corrections against; the feedback path itself lands in Wave 5.

#### Architectural surfaces touched

* [x] Services layer — Touched. Service functions that produce compositions invoke abilities and wrap in `Composition`.
* [x] Abilities contract — Touched. New typed output category.
* [x] Provenance — Touched directly. `ProvenanceRef` is the load-bearing change.
* [x] Execution mode — Touched. `Composition` carries the execution-mode-determined version watermark.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [x] Claims layer — Touched. `Block.claim_refs` is the substrate-side reference shape.
* [x] Signal granularity — Touched. Compositions re-render on claim-invalidation signal.
* [ ] Migration — Not touched (new types).
* [x] Evaluation harness — Touched. Eval fixtures gain composition-shape cases.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. Fallback projection rules per artifact 07 are privacy-load-bearing.

#### Edge cases

* Empty `Composition.blocks` — allowed; renders as an empty composition surface. The fallback marker block (per artifact 11) appears here.
* Stale `composition_version` — Handled per artifact 02 concurrency contract: surfaces refresh on stale-version rejection.
* Null `provenance` in a Block — rejected at construction. Every block carries a `ProvenanceRef`.
* Race: concurrent ability invocations producing the same `composition_id` — Handled per artifact 02: server-assigned `composition_version` increments monotonically.
* User intent persistence — Handled at the Block-attribute level; user-edited block attributes are projection-side and route through the feedback path, not the composition path.
* Value instability — same input → same output is asserted via composition-fingerprint test.
* Revoked source — provenance ref resolves to a Provenance envelope whose source attribution carries lifecycle state; ADR-0108 rendering rules apply on the surface side.

#### Dependencies

* ADR-0130 amendments (must land first — Wave 0 issue).
* ADR-0102 / ADR-0105 / ADR-0108 (already in place).
* Phase 0 artifacts 06, 07, 02 (cited by acceptance).

#### Shape A — New capability or ability

* **Category:** Substrate primitive. `Composition` is a new typed output category alongside `AbilityOutput<T>`.
* **Call-graph effect:** Compositions are produced by abilities; they have the same call-graph effects as the underlying ability category (Read, Transform, Publish, Maintenance).
* **Input type:** N-A — `Composition` is an output shape.
* **Output type:** `AbilityOutput<Composition>` for abilities whose `composition.produces_blocks = true`.
* **Composition:** N-A (this IS composition).
* **Consumers:** Wave 4 Gutenberg blocks, future render surfaces, eval harness, audit log.

---

## Wave 2 — Runtime transport + pairing (4 issues)

---

### Loopback HTTP runtime endpoint (`/v1/surface/invoke`, `/v1/surface/feedback`, `/v1/pairing/handshake`)

**Suggested milestone:** Wave 2 — Transport + Pairing
**Suggested labels:** `spec:draft`, `Feature`, `tauri-runtime`, `trust-boundary`
**Estimate:** L (5-7 days)

#### Problem

The WP plugin needs a runtime transport to invoke abilities and submit feedback. The runtime today does not expose HTTP at all; everything runs in-process via Tauri IPC or stdio MCP. Per Phase 0 artifact 15, the transport is a loopback-bound HTTP endpoint that exists only for paired SurfaceClients.

#### Why now

Wave 3 (WP plugin) cannot start without the transport. Wave 4 (block invocation) consumes it. This is the load-bearing infrastructure for every WP-side path.

#### Scope limits

This issue lands the endpoint shape, bind, lifetime, and route surface. It does not land HMAC signing (next issue), pairing handshake logic (its own issue), rate limits (its own issue), or the WP-side client (Wave 3). It lands skeleton handlers that 401 until the auth issues land.

#### Acceptance criteria

* Bind: only `127.0.0.1:<random_free_port>`. Per-startup random port. No fixed port, no `localhost`, no `0.0.0.0`, no `::1`, no IPv6 dual-bind.
* Listener lifetime tied to runtime process; teardown kills the listener; pairing re-handshake required after restart.
* `Host` header guard: reject any request whose `Host` does not normalize to `127.0.0.1:<bound_port>` exactly.
* `Origin` header guard: **positive allowlist, PHP-curl-primary** — the canonical caller in v1.4.2 is the WP plugin's PHP runtime client (`class-dailyos-runtime-client.php` per W3-B), which makes server-side cURL requests with no `Origin` header. The primary allow path is therefore **empty/absent Origin**. As defense-in-depth backup (NOT as an invitation to browser-direct calls), `Origin` whose origin (scheme+host+port) exactly matches the paired site's `site_url` (captured at first-pair per W2-C) is also accepted; this backup exists to keep the door open for narrow PHP-side proxies that some hosting stacks inject, not to admit browser-originated requests. Reject all other `Origin` values. Reject `Origin: null`. Host + Origin checks run before auth, rate limits, ability lookup, feedback handling. Per L0 cycle-3 codex-consult R1: the canonical transport model in v1.4.2 is PHP-only (artifact 15) — W3-A, W3-C, W4-E all describe browser→runtime as out of scope, and the W2-A guard is now phrased to match. Browser-originated requests reach the runtime only via the PHP plugin's REST endpoint → `class-dailyos-runtime-client.php` → loopback cURL path (see W4-E "transport model"). The positive-allowlist phrasing also prevents implementation drift toward inverted-allow logic (per /cso path-α #2).
* **Port advertisement to the WP plugin:** the pairing code displayed by the runtime is structured as `dailyos://pair?port=<bound_port>&code=<single_use_token>` (URL-shape, human-readable). The user copies this single string from the runtime UI into the WP admin pairing form; the WP plugin parses `port` and `code` from the string. The pairing code is short-lived (≤5 min) and single-use per W2-C; the port within is the bound loopback port for this runtime startup. There is no persistent file readable at arbitrary times by the WP-side process and no port-discovery side channel. Alternative: the runtime UI may also display the port and code as separate fields, both required at the WP admin form; consuming wave L0 picks one UX.
* Routes (skeleton, 401 until auth lands):
  * `POST /v1/pairing/handshake` — pairing code → session material.
  * `POST /v1/surface/invoke` — ability invocation via `SurfaceClientBridge`.
  * `POST /v1/surface/feedback` — feedback event submission.
  * `GET /v1/surface/health` — liveness only; no DailyOS state exposure.
  * `GET /v1/surface/keyring` — Ed25519 public-key distribution for projection verification (paired + HMAC-gated; added per W4-C key lifecycle).
* Each route returns a typed error envelope per Phase 0 artifact 12: `401` unauthenticated, `403` policy/scope/nonce failure, `409` stale version, `429` rate-limit, `503` runtime unavailable.
* Port advertisement to the WP plugin happens through the pairing handshake only — never via a persistent file readable at arbitrary times by the WP-side process.
* Negative tests from artifact 12 §"Common response-code expectations": each error code is produced by a concrete trigger; no error response leaks ability names, raw payloads, source excerpts, prompt text, local paths, or internal provenance trees.
* The endpoint binary is part of the Tauri runtime process, not a separate binary; teardown is governed by the existing `task_supervisor`.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A. Transport, not substrate.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** `/v1/surface/feedback` is the entry point for the WP-originated feedback path; the actual feedback application is the existing claim/feedback service.

#### Architectural surfaces touched

* [x] Services layer — Touched. The runtime endpoint module is a new service-adjacent surface.
* [x] Abilities contract — Touched. The invoke route routes to the registry through `SurfaceClientBridge`.
* [x] Provenance — Touched indirectly. Every invocation through this endpoint gets `actor_kind = surface_client` provenance.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [ ] Claims layer — Touched indirectly via the feedback route.
* [ ] Signal granularity — Not touched directly.
* [ ] Migration — Not touched.
* [x] Evaluation harness — Touched. Eval fixtures gain transport-shape cases.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. Error envelopes must not leak per artifact 12.

#### Edge cases

* Empty / missing `Host` header — rejected.
* Stale port (process restarted, WP plugin still has old port) — Handled: WP plugin receives a connection error and re-runs pairing handshake.
* Null `Origin` — rejected.
* Race: two startups racing to bind a port — Handled: random port selection per startup; if bind fails the runtime retries on a fresh random port up to N attempts.
* User intent persistence — N-A.
* Value instability — port changes per startup is intentional.
* Revoked source — N-A at the transport layer; revocation is upstream.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class` (must merge first).
* Phase 0 artifact 15 (binding contract).
* Phase 0 artifact 04 (the runtime continues to host this — Tauri stays).

#### Shape A — New capability or ability

* **Category:** Substrate primitive (transport surface, not an ability).
* **Consumers:** WP plugin (Wave 3), eval harness, audit log.

---

### HMAC-SHA256 request signing for the loopback transport

**Suggested milestone:** Wave 2 — Transport + Pairing
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`
**Estimate:** M (3-4 days)

#### Problem

The loopback endpoint is reachable from any local process. Bearer tokens alone do not prove that a specific HTTP request was produced by the paired plugin — a malicious local process, a malicious co-resident WP plugin, a browser extension resolving localhost, or a stolen token are all in scope.

#### Why now

Without HMAC signing, the bearer token is the only credential and the threat model from Phase 0 artifact 08 is unmet. Wave 2's transport endpoint must reject forged, replayed, or tampered ability invocations.

#### Scope limits

This issue lands HMAC-SHA256 signing on the runtime side (verifier) and the signing contract documented for the WP side (signer). WP-side PHP implementation lands in Wave 3 with the runtime client class.

#### Acceptance criteria

* Canonical signing per artifact 08 §"Canonicalization": `method`, `path_query`, `content_type`, `body`, `nonce`, `timestamp`, with the literal `DAILYOS-WP-BRIDGE-HMAC-V1` prefix and length-prefixed field encoding.
* Per-session signing key derived during pairing handshake. Key rotates per pairing. Key is not stored WP-side in `wp_options` plaintext; per Phase 0 artifact 08 the WP side retrieves the key only via a `manage_options`-gated WP filter at request-time.
* `X-DailyOS-Signature`, `X-DailyOS-Nonce`, `X-DailyOS-Timestamp` headers carry the signature, nonce, and RFC3339 UTC timestamp.
* Freshness window per Phase 0 artifact 08: timestamps more than **30 seconds older** than runtime time are rejected as `timestamp_stale` (401); timestamps more than **5 seconds newer** than runtime time are rejected as `timestamp_future` (401). Both thresholds are **config-driven, centrally configurable** (per /cso path-α #3), not float-literal'd in handler code. The runtime-time source is the monotonic clock for ordering and the wall clock for absolute comparisons; the artifact 08 ordering of "timestamp before nonce before HMAC" is preserved.
* Nonce replay window: nonces consumed within the freshness window are recorded; duplicate nonces are rejected with `403`.
* Verification runs before ability dispatch, after Host/Origin guards, before rate-limit matrix.
* Negative tests from artifact 08 §"Negative cases" + artifact 12: tampered method, tampered path, tampered body, expired timestamp, replayed nonce, mismatched session key — each rejected with a typed error.
* Error envelope on signing failure does not leak the failure reason to the caller; the audit log records the exact reason for operator triage.
* Constant-time MAC comparison per `ring::constant_time::verify_slices_are_equal` or equivalent.

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A — this is a transport-layer authenticity gate.

#### Architectural surfaces touched

* [x] Provenance — Touched. Audit log records signing-check outcome.
* [x] Surface parity — Touched. The signing contract is the load-bearing primitive that makes the surface trustworthy.
* [x] Privacy rendering — Touched. Error envelopes must not leak.
* All others — Not touched.

#### Edge cases

* Empty body — sign zero bytes.
* Clock skew — Handled: 30s stale / 5s future freshness window per artifact 08.
* Replay across sessions — Handled: per-session key changes the signature space.
* Replay within session — Handled: nonce table.
* Race: two requests with the same nonce racing — Handled: nonce-table mutex with atomic insert-or-reject.
* Tampered header value with same length — Handled: length-prefixed canonicalization.

#### Dependencies

* Issue: `Loopback HTTP runtime endpoint` (must merge first).
* Phase 0 artifact 08 (binding contract).
* OS keychain access via existing `LocalKeychain` for per-session key storage.

#### Shape A — New capability or ability

* **Category:** Substrate primitive (transport authenticity).
* **Consumers:** Wave 2 endpoint, Wave 3 WP plugin client, audit log.

---

### Pairing handshake + four-path token recovery defenses

**Suggested milestone:** Wave 2 — Transport + Pairing
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`, `tauri-runtime`
**Estimate:** L (5-6 days)

#### Problem

A WP `SurfaceClient` becomes paired through a user-mediated handshake. Per Phase 0 /cso refinement 1 and artifact 01, four threat paths — Reinstall, DB-Restore, Site-Switch, Exfiltration — need individually-named defenses, not one generic "re-pair on failure" control.

#### Why now

The handshake is the only authenticated path that creates a paired session. Every Wave 3+ issue assumes a paired session exists; if the handshake is weak, every downstream path is compromised.

#### Scope limits

This issue lands the four named defenses and the handshake protocol. It does not land the WP-side admin UI for entering the pairing code (Wave 3) or the rate-limit matrix that gates the handshake (next issue).

#### Acceptance criteria

* Pairing code displayed once by the runtime, short-lived (≤5 minutes), single-use, bound to the runtime process instance, invalid after runtime restart, invalid after N failed attempts.
* Successful handshake creates or refreshes: `surface_client_id`, short-lived bearer token (≤8h), per-session HMAC key, endpoint version, granted scopes, WP-allowlisted ability listing.
* **Reinstall defense (Anchor Rotation Handshake, artifact 01).** Runtime signing key is the anchor; stored OS-keychain-side. Reinstall creates a new anchor; stale WP proof is rejected by `runtime_anchor_id` mismatch. Re-pairing requires explicit user-visible handshake.
* **DB-Restore defense.** Authoritative pairing + revocation table lives runtime-side; tokens expire by clock + are checked against the revocation table on every request. Restoring an old `wp_options` row does not resurrect authority.
* **Site-Switch defense.** Pairing binds to `site_url` + runtime-issued `site_nonce` captured at first-pair. Migration to a different domain/path/site URL fails verification.
* **Exfiltration defense.** Bearer tokens are short-lived (≤8h refreshable); HMAC key + bearer-token combination is required; write actions also require user-presence nonce (Wave 4).
* `POST /v1/pairing/handshake` per artifact 15: accepts pairing code + WP context (`wp_user_id`, `wp_site_id`, `request_id`) + client metadata; returns session material.
* Negative tests from artifact 12: each of the four threat paths has a regression fixture that asserts the named defense rejects the path.
* Audit log records every pairing event: `pairing_created`, `pairing_refreshed`, `pairing_revoked`, with `surface_client_id`, `wp_user_id`, `wp_site_id`, `reason` for revocations.
* User-visible runtime UI shows: current pairings, pairing creation timestamp, last-use timestamp, "revoke pairing" control.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Not directly. Revocation flows through the runtime revocation table; downstream signal emission for "pairing revoked" is the existing audit-log path.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Indirect — a revoked pairing rejects feedback submissions with `403`.

#### Architectural surfaces touched

* [x] Services layer — Touched. New pairing service.
* [x] Abilities contract — Touched. Scope grants live on the pairing.
* [x] Provenance — Touched. `actor_instance` derives from the pairing.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [ ] Claims layer — Not touched.
* [ ] Signal granularity — Not touched.
* [x] Migration — Touched. A new pairing/revocation DB table is introduced.
* [ ] Evaluation harness — Touched.
* [x] Surface parity — Touched.
* [x] Privacy rendering — Touched.

#### Edge cases

* Empty pairing code — rejected.
* Stale pairing code — rejected per the 5-minute window.
* Null `wp_site_id` — rejected.
* Race: two parallel handshakes with the same pairing code — Handled: code is single-use; second attempt rejected.
* User intent persistence — Handled: user can explicitly revoke a pairing.
* Value instability — pairing material is deterministic per session.
* Revoked source — explicit revocation table + clock-based expiry.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class`.
* Issue: `Loopback HTTP runtime endpoint`.
* Issue: `HMAC-SHA256 request signing`.
* Phase 0 artifact 01 (binding contract).

#### Shape B — Schema or data-model change

* **New tables or columns:** `surface_client_pairings` (runtime-side DB): `surface_client_id`, `runtime_anchor_id`, `site_url`, `site_nonce`, `created_at`, `expires_at`, `revoked_at`, `scopes`, `audit_id`. `surface_client_revocations` (runtime-side DB): `surface_client_id`, `revoked_at`, `reason`.
* **Append-only vs mutate-in-place:** `surface_client_pairings` is append-only on creation; `revoked_at` is set in-place on revocation (the row is not deleted, keeping audit history).
* **Negative knowledge / tombstones:** revocation IS the tombstone shape; expired/revoked pairings remain queryable for audit.
* **Pruning policy:** revoked pairings retained 90 days, then pruned with audit-log preservation.
* **Read path:** every request reads the pairing table to validate the session.
* **Write path:** pairing service is the only writer.

---

### Rate-limit matrix in `SurfaceClientBridge`

**Suggested milestone:** Wave 2 — Transport + Pairing
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`, `performance`
**Estimate:** M (3-4 days)

#### Problem

A compromised WP plugin, a buggy Gutenberg block, a runaway agent loop through the WP MCP Adapter, or a malicious local process can repeatedly invoke abilities through the same loopback transport. Without per-axis rate limits, the runtime has no defense before ability dispatch.

#### Why now

Wave 2 transport landing without rate limits creates a window where Wave 3 plugin development can exhaust runtime capacity by accident. Wave 4 block invocation fans out to multiple abilities per page load; the limits need to be calibrated before blocks ship.

#### Scope limits

This issue lands the rate-limit matrix at `SurfaceClientBridge` per Phase 0 artifact 09 §"Axes and concrete numbers." It does not change ability cost models or introduce per-ability metering — those are scoped axes, not per-ability budgets.

#### Acceptance criteria

* Enforcement at `SurfaceClientBridge`, after loopback transport authentication + parsing, after pairing/HMAC/scope validation, before registry dispatch.
* Token-bucket implementation with monotonic clock. Per-axis budgets per artifact 09:
  * **Axis 1 — Per SurfaceClient instance:** Read 300 req/min, burst 20/s. Write 30 req/min, burst 2/s.
  * **Axis 2 — Per WP user:** Read 120 req/min, burst 8/s. Write 12 req/min, burst 1/s.
  * **Axis 3 — Per WP site:** Read 600 req/min, burst 40/s. Write 60 req/min, burst 4/s.
  * **Axis 4 — Per ability (across all callers):** category-specific limits per artifact 09.
  * **Axis 5 — Per scope class:** per-scope-class limits per artifact 09.
* All axes checked; rejection on first exceeded axis with `429` and `X-RateLimit-Exhausted-Axis: <axis>` header.
* Rate-limit state is in-memory per process; restart clears state (acceptable per artifact 09 because pairing re-handshake is also required).
* Negative tests from artifact 12 + artifact 09: each axis has a fixture that exhausts the bucket and asserts `429` with the correct exhausted-axis header; no ability body invoked when rate-limit denial fires.
* Rate-limit denial events emit audit-log entries with `surface_client_id`, `wp_user_id`, `wp_site_id`, `ability_name`, `exhausted_axis`.
* `429` envelope does not leak which other axes had budget remaining.
* Config-driven thresholds (no `f64` literals outside config files, per v1.4.1 invariant).
* **Post-hoc calibration acceptance.** Per architect-reviewer cycle-1 finding B2: per-ability (axis 4) and per-scope-class (axis 5) budgets are sensitive to actual ability dispatch cost, which becomes empirical only after W4-A's first block invokes through `SurfaceClientBridge`. W4's L3 wave adversarial includes a calibration pass that MAY amend axis-4 and axis-5 thresholds. Any amendments land via Linear tickets against the Codebase Maintenance & Production Quality project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`), not via re-opening this issue.

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A — DoS defense layer.

#### Architectural surfaces touched

* [x] Surface parity — Touched.
* [x] Privacy rendering — Touched (error envelope).
* [x] Evaluation harness — Touched.
* All others — Not touched.

#### Edge cases

* Empty bucket on first request — Handled: bucket starts full per artifact 09.
* Stale bucket after long idle — Handled: refill cap = initial full bucket.
* Null `wp_user_id` — Handled: routed to a per-instance bucket only (axes 2 + 5 not applicable).
* Race: concurrent requests racing to deplete the bucket — Handled: token-bucket access is atomic via existing concurrent-primitive crate.
* User intent persistence — N-A.
* Value instability — limits are deterministic given the same clock.
* Revoked source — Handled: revoked pairing rejects at pairing layer before rate-limit check.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class`.
* Issue: `Loopback HTTP runtime endpoint`.
* Issue: `Pairing handshake + four-path token recovery defenses`.
* Phase 0 artifact 09 (binding contract).

#### Shape A — New capability or ability

* **Category:** Substrate primitive (DoS-defense layer at the SurfaceClientBridge boundary).
* **Consumers:** Wave 3 WP plugin (consumes the rejection envelope), audit log, eval harness.

---

## Wave 3 — WP plugin + MCP server (3 issues)

---

### DailyOS WordPress plugin skeleton + WP Abilities API registration

**Suggested milestone:** Wave 3 — WP Plugin + MCP Server
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `dx`
**Estimate:** L (5-7 days)

#### Problem

WordPress needs a plugin that registers DailyOS abilities into the WP Abilities API, sets up the admin pages, scaffolds the block library directory, and routes ability invocation through the WP-side runtime client. Without it there is no WP-side path to the runtime.

#### Why now

Wave 4 blocks read from this plugin's ability proxies. Wave 5 feedback router lives in this plugin. Without the skeleton, Wave 4 cannot start.

#### Scope limits

This issue lands the plugin skeleton per Phase 0 artifact 13: directory layout, plugin header, autoloader, ability proxy registration class, admin page scaffolding, block library directory. It does not land the runtime client implementation (next issue) or any concrete Gutenberg block (Wave 4).

#### Acceptance criteria

* Plugin name `DailyOS`, slug `dailyos`, entry file `dailyos/dailyos.php` per artifact 13 §"Plugin header."
* Directory layout per artifact 13 §"Directory structure": `includes/`, `blocks/`, `abilities/`, `admin/`, plus the named class files.
* `Requires at least: 6.9`, `Requires PHP: 8.1`.
* Composer autoloader (PSR-4) configured under `vendor/`.
* `class-dailyos-plugin.php` is the main singleton; instantiated on `plugins_loaded`.
* `class-dailyos-ability-registry.php` reads the `tools/dailyos-abilities.json` inventory artifact and registers each entry into the WP Abilities API per WP 6.9 hook conventions. Abilities registered with the `mcp_exposure` value from the inventory.
* Admin page `admin/pages/pairing.php` exists with the form skeleton (input field for pairing code, submit, success/error message slots). The actual pairing handshake call lands in the next issue.
* Admin page `admin/pages/settings.php` exists with the settings shell (pairing status, scopes, last-use timestamp). Read-only in this issue.
* Plugin lints clean against PHP_CodeSniffer + WordPress Coding Standards.
* Activation hook scaffolds initial `wp_options` rows for plugin state with safe defaults.
* Deactivation hook does NOT delete pairing state — only revokes the WP-side bearer.
* WP 7.0 client-side `executeAbility()` path: governed by `client_side_executable` ∧ `allowed_actors: [SurfaceClient]` ∧ `required_scopes` per W0-D ADR-0102 amendment. `mcp_exposure` does NOT govern SurfaceClient invocation (the two fields are independent per artifact 05 lines 389-412). The WP client-side JS invocation flows through the WP plugin's runtime client per W3-B (PHP→loopback) rather than direct browser-to-runtime calls. Direct browser-to-runtime JS is explicitly out of scope for v1.4.2: per W4-E, browser scripts call a WP REST endpoint and the PHP runtime client makes the loopback request — the W2-A Origin guard treats absent Origin as the primary allow path (PHP curl), with `site_url`-matching as defense-in-depth backup, not as an invitation to browser-direct calls.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A — the plugin is a thin client.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Plugin scaffolds the feedback router class but does not wire it; that lands in Wave 5.

#### Architectural surfaces touched

* [x] Abilities contract — Touched. Plugin consumes the inventory artifact.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. WP admin pages must not leak runtime internals.
* All others — Not touched.

#### Edge cases

* Empty inventory artifact at activation — plugin activates with no registered abilities; admin page shows "no abilities registered yet — check pairing."
* WP version below 6.9 — plugin refuses to activate with a clear message.
* PHP version below 8.1 — plugin refuses to activate.
* Race: activation hook running concurrently with another plugin's activation — Handled: WP's option-update mutex.
* User intent persistence — Handled: deactivation preserves pairing state.

#### Dependencies

* Issue: `Ability-surface inventory format + CI gate` (provides `tools/dailyos-abilities.json`).
* Phase 0 artifact 13 (binding contract).
* WP 6.9 Abilities API documentation (cite at L0 plan time; Phase 1 verifies the exact hook names).

#### Shape A — New capability or ability

* **Category:** Surface implementation (WP-side).
* **Consumers:** WP users, future block library, future MCP Adapter integration.

---

### WP-side runtime client + HMAC signer + pairing UI

**Suggested milestone:** Wave 3 — WP Plugin + MCP Server
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `trust-boundary`
**Estimate:** L (5-6 days)

#### Problem

The plugin skeleton scaffolds the runtime-client and HMAC-signer class files; this issue implements them. Without the client, WP abilities cannot reach the runtime; without the signer, requests cannot pass the runtime's HMAC verifier.

#### Why now

This is the WP-side counterpart to Wave 2 transport + HMAC. They must both exist before Wave 4 block invocation works.

#### Scope limits

This issue lands `class-dailyos-runtime-client.php`, `class-dailyos-hmac-signer.php`, the pairing admin page's actual handshake wiring, and the `manage_options`-gated WP filter that retrieves the per-session HMAC key WP-side. It does not land block invocation (Wave 4) or feedback routing (Wave 5).

#### Acceptance criteria

* `class-dailyos-runtime-client.php` exposes:
  * `invoke_ability(string $ability_name, array $payload): array` — performs the HMAC-signed `POST /v1/surface/invoke` and returns the typed response.
  * `submit_feedback(array $event): array` — performs the HMAC-signed `POST /v1/surface/feedback`.
  * `handshake(string $pairing_code, array $wp_context): array` — performs `POST /v1/pairing/handshake`.
  * Each method honors the error envelope from Wave 2 endpoint per artifact 12.
* `class-dailyos-hmac-signer.php` implements the canonicalization from Phase 0 artifact 08 byte-for-byte.
* **Per-session HMAC key retrieved WP-side ONLY via the `dailyos_wp_bridge_session_key` WP filter at request-time, per Phase 0 artifact 08 §"WordPress-side retrieval."** Both filter gates MUST pass: (1) current WP user has `manage_options`; (2) the runtime-issued handshake gate `dailyos_pairing_handshake_complete:<pairing_id>:<session_id>` is present (single-use; consumed on first successful retrieval). The plugin MUST NOT persist the derived key in `wp_options`, transients, post meta, browser local storage, or block attributes after the active session expires. The derived key is process-local secret material per artifact 08 line 114. Public WP code cannot read the key.
* **WP admin form CSRF + nonces.** Pairing form submit, revoke-pairing button, and any settings-page mutation use WordPress's standard `wp_nonce_field` + `check_admin_referer` per WP coding standards (this is separate from DailyOS user-presence nonces in W4-E). Error surfaces in the admin UI clearly distinguish "WP nonce failure" from "runtime auth failure" without leaking runtime internals.
* Pairing admin page: form submit triggers `handshake()`; success message displays scopes granted; failure message displays the typed error reason from the runtime (without leaking internals).
* Settings admin page: shows current pairing (instance ID, site nonce hash, scopes, last-use timestamp), "revoke pairing" button.
* Negative tests: tampered request body produces `403`; replayed request produces `403`; calling `dailyos_wp_bridge_session_key` filter without `manage_options` returns no key and emits a plugin-side typed error; persisted-key detection — a grep CI check asserts the plugin source tree contains no path that writes the derived key to `wp_options` / transients / postmeta / block attributes after retrieval.
* Plugin lints clean.
* Documentation in plugin readme describes the pairing flow + recovery (what to do if the runtime restarted, what to do if the plugin is reinstalled).

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A — this is plumbing.

#### Architectural surfaces touched

* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. Error surfacing in WP admin must not leak runtime internals.
* All others — Not touched.

#### Edge cases

* Empty pairing code in form — rejected client-side before submit.
* Stale pairing code — runtime rejects; UI shows "code expired, generate a new one."
* Null `wp_user_id` in WP context — rejected at runtime.
* Race: two admins pairing concurrently — Handled: single-use code at runtime.
* User intent persistence — Handled: explicit revoke control.
* Value instability — N-A.
* Revoked source — runtime rejects with `403`; UI shows "pairing revoked, re-pair."

#### Dependencies

* Issue: `Loopback HTTP runtime endpoint`.
* Issue: `HMAC-SHA256 request signing`.
* Issue: `Pairing handshake + four-path token recovery defenses`.
* Issue: `DailyOS WordPress plugin skeleton`.
* Phase 0 artifacts 08, 13, 15.

#### Shape A — New capability or ability

* **Category:** Surface implementation (WP-side).
* **Consumers:** Wave 4 blocks, Wave 5 feedback router, WP admins.

---

### Custom MCP server with DailyOS allowlist + dedicated low-cap WP user

**Suggested milestone:** Wave 3 — WP Plugin + MCP Server
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `trust-boundary`, `mcp`
**Estimate:** M (3-5 days)

#### Problem

Per ADR-0129 §4, substrate-backed abilities must NOT be exposed by the default WP MCP server. DailyOS ships a custom MCP server with explicit ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. The default WP MCP Adapter behavior is wrong for DailyOS.

#### Why now

Without this, the WordPress MCP Adapter exposes all registered abilities to any connected MCP client (Claude Desktop, Cursor, others), bypassing the `mcp_exposure` field and the SurfaceClient scope grants.

#### Scope limits

This issue lands the custom MCP server configuration, the dedicated WP user account creation at activation, the allowlist construction from the inventory, and permission callbacks that check both WP capabilities and DailyOS `SurfaceClient(WordPress)` scopes. It does not land the WP MCP Adapter plugin itself — that is an external dependency.

#### Acceptance criteria

* `class-dailyos-mcp-server.php` registers a custom MCP server with the WP MCP Adapter under a DailyOS-specific server name (per artifact 13 §"Custom MCP server").
* The custom server enumerates abilities with `mcp_exposure: Invocable | MetadataOnly` only; `mcp_exposure: None` abilities are not enumerated. Verified by negative fixture from artifact 12.
* A dedicated WP user `dailyos_substrate` is created at plugin activation with a low-capability role (`subscriber`-derived custom role: no posting, no commenting, no admin). The custom MCP server runs requests as this user.
* Permission callbacks check both: (a) WP capabilities of the requesting user (typically `dailyos_substrate`), AND (b) DailyOS `SurfaceClient(WordPress)` scopes per ADR-0129 §4 amendment. Failure on either rejects the request.
* The default WP MCP server (the one registered automatically by the MCP Adapter for built-in abilities) is verified to NOT expose any DailyOS-namespaced ability; a negative test asserts this.
* Connection auth uses the MCP Adapter's standard authentication mechanism; DailyOS does not implement transport auth at this layer (the inner `SurfaceClient` HMAC remains the runtime-side gate).
* Audit log: every MCP server invocation records `mcp_server_name`, `wp_user_id` (the `dailyos_substrate` user), `ability_name`, `dailyos_scope_check_result`.
* Read-mostly default: the allowlist defaults to ability `mcp_exposure: Invocable` ∩ ability `category: Read | Transform`. Publish/Maintenance abilities are not in the default allowlist; promotion to allowlist is per-ability with explicit ADR review.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A.
* **Health scoring:** N-A.
* **Intel context:** N-A in this issue.
* **Briefing callouts:** N-A.
* **Feedback hook:** N-A.

#### Architectural surfaces touched

* [x] Abilities contract — Touched. MCP allowlist is the gate.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. MCP server output goes to external clients (Claude Desktop, Cursor).
* All others — Not touched.

#### Edge cases

* Empty inventory — server registers with no abilities; not an error.
* Stale `dailyos_substrate` user (deactivated by WP admin) — Handled: plugin re-creates on next activation with audit log entry.
* Null `wp_user_id` in MCP request — Handled: server rejects with `401`.
* Race: two plugin activations racing to create the user — Handled: WP's user-create returns existing user gracefully.
* User intent persistence — Handled: WP admin deactivating `dailyos_substrate` is treated as an explicit revocation.
* Value instability — N-A.
* Revoked source — Handled: scope check at request time rejects revoked pairings.

#### Dependencies

* Issue: `Ability-surface inventory format + CI gate`.
* Issue: `Promote AbilityPolicy to canonical schema`.
* Issue: `WP-side runtime client + HMAC signer + pairing UI`.
* WordPress MCP Adapter plugin (external dependency, install at plugin activation if not present).

#### Shape A — New capability or ability

* **Category:** Surface implementation (WP-side MCP server).
* **Consumers:** External MCP clients connecting to WP (Claude Desktop, Cursor, others).

---

## Wave 4 — Composition + first block + three-view consistency (6 issues)

---

### `dailyos/account-overview` ability — Composition-producing substrate ability

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `abilities-runtime`, `composition`

**Estimate:** M (3-4 days)

#### Problem

The W4-A Gutenberg block (renderer) consumes a `Composition`-producing ability named `dailyos/account-overview`. A grep across `src-tauri/`, `src/`, and `abilities-runtime/` (run at L0 cycle-2) returns no matches for `account.overview`, `account_overview`, `AccountOverview`, `prepare_account_overview`, or `get_account_overview`. The producer does not exist, so without this issue W4-A renderer ships a UI with no backing ability — the producer/renderer split is the architectural claim of v1.4.2 and the producer side must be wired explicitly (per `feedback_wire_existing_substrate_not_future_producer`).

#### Why now

W4-A renderer cannot start without the producer in scope. The producer is a small ability (input: `account_id`; output: `AbilityOutput<Composition>` with the typed block tree from artifact 14) — most of its substance is wiring existing v1.4.1 context primitives (`gather_account_context()`, claim retrieval, trust-band data) into the `Composition` shape that W1-E defines.

#### Scope limits

This issue lands the minimal viable producer: one ability that returns `AbilityOutput<Composition>` against a real `account_id` fixture. It does NOT land the full entity-intelligence detail page (v1.4.3 reframed), additional account-related abilities, or cross-account aggregation. The composition includes only the block types the W4-A renderer needs to demonstrate end-to-end: `account_overview`, `claim_summary`, `evidence_list`. `health_snapshot` and `risk_callout` are optional inclusions if their content is already produced by existing v1.4.1 services; otherwise they are deferred to v1.4.3.

#### Acceptance criteria

* `dailyos/account-overview` ability declared via `#[ability]` macro with `category: Read`, `allowed_actors: [User, SurfaceClient]`, `required_scopes: [read.account_overview]` (per the W1-B compile-error gate), `mcp_exposure: McpExposure::Invocable`, `client_side_executable: true` (required for WP block hydration per W1-B SurfaceClientBridge gate; the field is independent of `mcp_exposure` per artifact 05 lines 389-412), `composition.produces_blocks: true`.
* Input type: `AccountOverviewInput { account_id: AccountId }`.
* Output type: `AbilityOutput<Composition>`. The `Composition` carries `composition_id`, server-assigned `composition_version` (per W4-B), and a `blocks: Vec<Block>` populated per artifact 14 §"Composition shape":
  * One `Block { type: BlockType::AccountOverview, attributes: {...account header data...}, claim_refs: [...], provenance: ProvenanceRef }`.
  * Zero or more `Block { type: BlockType::ClaimSummary, ... }` for top open claims.
  * Zero or more `Block { type: BlockType::EvidenceList, ... }` for the evidence linked to the surfaced claims.
* Trust band data populated per block: each `Block.claim_refs` resolves to a claim whose substrate-side trust scoring (per v1.4.1) feeds the rendered band (`likely_current` / `use_with_caution` / `needs_verification`); the ability does NOT compute trust bands, it surfaces them.
* `ProvenanceRef` shape per W1-E: `{ invocation_id, field_path }`. The ability output's top-level `Provenance` envelope is the resolution target.
* Source attribution per ADR-0105: every claim_ref resolves to the source attribution captured at claim creation.
* Unit tests: nominal account, account with no open claims, account with one trust-band-failing claim, account with stale source.
* Eval-harness fixture: a deterministic seed account in the dev DB produces a stable composition fingerprint (per the W1-E composition-fingerprint test contract).
* The ability code lives under `abilities-runtime/src/abilities/` (or the appropriate existing abilities directory).

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched. Ability output invalidates on claim-version changes for the claims its `claim_refs` list (v1.4.1 W1 signal infrastructure).
* **Health scoring:** Consumes existing trust-scoring outputs; no scoring change.
* **Intel context:** Internally uses `gather_account_context()` and existing claim retrieval; no changes to the context-build path.
* **Briefing callouts:** Indirect — if a `risk_callout` block is included, it pulls from existing callout-emitting services.
* **Feedback hook:** Indirect — the produced composition carries `claim_refs` against which W5-A feedback router applies corrections.

#### Architectural surfaces touched

* [x] Services layer — Touched. Ability is a new service consumer.
* [x] Abilities contract — Touched directly. New Composition-producing ability.
* [x] Provenance — Touched. Resolves `ProvenanceRef` against ability output envelope.
* [x] Claims layer — Touched. Reads claims + claim_refs.
* [x] Signal granularity — Touched (subscribes to claim invalidation).
* [x] Evaluation harness — Touched.
* [x] Surface parity — Touched (the producer half of the producer/renderer split).
* [x] Privacy rendering — Touched (ADR-0108 sensitivity gating per claim).
* All others — Not touched.

#### Edge cases

* Empty `account_id` — rejected at input validation.
* Stale account (no claims for >freshness threshold) — Handled: ability returns a composition with `account_overview` block showing trust bands appropriate to the stale state; W4-A renderer presents the bands.
* Null account (doesn't exist) — rejected with a typed `AccountNotFound` error.
* Race: concurrent invocations on the same `account_id` — Handled: ability is deterministic given the same input + substrate state at invocation time; `composition_version` is server-assigned per W4-B.
* User intent persistence — Handled: dismissed/corrected claims (existing tombstone semantics) are respected by the underlying retrieval path; the ability surfaces what the substrate says is current.
* Value instability — Handled: same input + same substrate state → same composition fingerprint.
* Revoked source — Handled per ADR-0108 (claim with revoked source surfaces with `needs_verification` band).

#### Dependencies

* Issue: `Composition contract substrate types + ProvenanceRef shape` (W1-E) — provides the `Composition` and `Block` types.
* Issue: `Promote AbilityPolicy to canonical schema` (W1-B) — provides `required_scopes` + `mcp_exposure`.
* Issue: W4-B `Three-view consistency: concurrency contract implementation` — provides server-assigned `composition_version`.
* v1.4.1 existing context primitives (`gather_account_context()`, claim retrieval, trust scoring).
* Phase 0 artifact 14 §"Composition shape" (binding contract).

#### Shape A — New capability or ability

* **Category:** Read ability producing `Composition`.
* **Call-graph effect:** Read (no mutation).
* **Input type:** `AccountOverviewInput { account_id }`.
* **Output type:** `AbilityOutput<Composition>`.
* **Composition:** Yes — this IS a Composition-producing ability.
* **Consumers:** W4-A Gutenberg block, future MCP renderer, future entity-page surfaces.

---

### `dailyos/account-overview` Gutenberg block (producer/renderer split)

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `gutenberg`, `composition`
**Estimate:** L (5-7 days)

#### Problem

The first concrete proof that the foundation works is a Gutenberg block that renders substrate intelligence end-to-end: attributes stored in WP, content re-rendered on read by invoking the ability, trust bands inline, provenance refs resolved, claim refs preserved. Per Phase 0 artifact 14, this is the `dailyos/account-overview` block.

#### Why now

Every architectural decision in this project converges on this block. If the block doesn't render the foundation is incomplete. This is the load-bearing user-visible artifact for the release.

#### Scope limits

This issue lands the one block — `dailyos/account-overview` — per artifact 14. It does not land additional blocks. The editable-composition-overlay semantics (artifact 11) are consumed at the W4-D substrate rule layer and the W5-A feedback router layer; this issue's render path consumes the W4-D output, it does not re-implement overlay routing. It does not land click-bound feedback routing (Wave 5).

#### Acceptance criteria

* Block descriptor `block.json` per artifact 14 §"block.json schema" — `apiVersion: 3`, attributes for `account_id`, `composition_id`, `composition_version`, `claim_refs`, `trust_band_render_mode`.
* `render.php` invokes the `dailyos/account-overview` ability via the runtime client, receives a `Composition`, renders each `Block` according to its `BlockType`, resolves `ProvenanceRef` against the ability output's top-level `Provenance` envelope.
* `edit.js` and `save.js` per artifact 14: edit UI shows account selector + render-mode toggle; save preserves only attributes, not rendered HTML.
* Trust bands render inline per ADR-0108: `likely_current`, `use_with_caution`, `needs_verification`, with the existing trust-band CSS treatment from `.docs/design/`.
* Provenance click target: clicking a trust band opens a provenance panel showing source attribution, source `_asof`, claim_refs, ability invocation ID.
* Cached projection support per artifact 14 §"Performance + fallback": last-known-good projection rendered if the ability invocation fails; fallback banner per artifact 07. The cached-projection path verifies the W4-C Ed25519 signature embedded in the cached block attribute before rendering — unverified cached bytes render with the tamper banner per W4-C semantics, not as trusted content. W4-C must be merged before W4-A starts so the verification primitive exists.
* Unknown block type from the ability output renders per the W4-D substrate-side fallback projection rules (which W4-D publishes per artifact 07 + artifact 11 edit-routing semantics), NOT raw payload. W4-D must be merged before W4-A starts so the substrate rule source exists; per W4-D acceptance "WP may not deviate," the renderer consumes the published rules rather than re-deriving them.
* Block can be inserted from the block inserter under the "DailyOS" category.
* End-to-end test: insert block on a draft page, select a real account fixture (`account_id` from dev DB), publish — page renders with substrate data, trust bands, provenance click-through.
* Lints clean (block.json schema, `@wordpress/scripts` build), PHP_CodeSniffer clean on `render.php`.
* WP Theme Check and Block Validation pass.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched. Block re-renders on claim-invalidation signal that affects its `claim_refs`. The signal/job/claim chain from v1.4.1 W1 is the upstream invalidator.
* **Health scoring:** Trust bands consume the substrate-side trust scoring; no scoring change in this issue.
* **Intel context:** `dailyos/account-overview` ability internally uses `gather_account_context()`; no changes in this issue.
* **Briefing callouts:** A `risk_callout` block type in the composition output renders inline; no callout-content change in this issue.
* **Feedback hook:** Block save preserves attribute-level state; substrate-data feedback (corrections, dismissals) lands in Wave 5.

#### Architectural surfaces touched

* [x] Services layer — Touched indirectly. The block invokes through the WP runtime client into the runtime; service-layer behavior is upstream.
* [x] Abilities contract — Touched. The block consumes a `Composition`-producing ability.
* [x] Provenance — Touched. Resolves `ProvenanceRef` against the ability output's envelope.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [x] Claims layer — Touched. Renders claim_refs.
* [x] Signal granularity — Touched. Re-renders on signal-driven invalidation.
* [ ] Migration — Not touched.
* [x] Evaluation harness — Touched. Eval fixtures gain block-render cases.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched. ADR-0108 sensitivity rules render against WP surface.

#### Edge cases

* Empty `account_id` attribute — block renders a "select an account" placeholder in edit mode; published view shows a configured-by-author message.
* Stale data: substrate ability returns claims with `source_asof` past the trust freshness threshold — trust band renders `needs_verification` per substrate decision; block does not over-render confidence.
* Null `composition_version` (first render before save) — Handled: block renders the current ability output and stores the returned `composition_version`.
* Malformed `claim_refs` on save — rejected at save handler with a recoverable error in the editor.
* Race: two editor sessions editing the same block — Handled per Wave 4 concurrency issue; this block consumes the watermark contract.
* User intent persistence — Handled: user dismissals route through Wave 5 feedback path; block re-renders without dismissed claim on next refresh.
* Value instability — Handled: block invokes ability deterministically; stability-as-confidence is upstream.
* Revoked source — Handled per ADR-0108: revoked source claims render with `needs_verification` band + revoked-source provenance indicator.

#### Dependencies

* Issue: `Composition contract substrate types + ProvenanceRef shape` (W1-E).
* Issue: `DailyOS WordPress plugin skeleton` (W3-A).
* Issue: `WP-side runtime client + HMAC signer + pairing UI` (W3-B).
* Issue: `dailyos/account-overview ability — Composition-producing substrate ability` (W4-A0) — producer. Must merge before this issue starts; grep confirmed at L0 cycle-2 that the producer does not pre-exist.
* Issue: `Tamper detection contract: projection signing + verification` (W4-C) — **must merge first.** The renderer consumes the offline Ed25519 verification path for cached projections; without W4-C the cached-projection fallback either renders unverified bytes (trust-boundary violation) or has no signature to verify (silent demotion). Added in L0 cycle-3.
* Issue: `Custom block fallback projection rules (substrate-side enforcement)` (W4-D) — **must merge first.** Acceptance criteria reference the substrate-side fallback projection rules ("renders per artifact 07's nearest-known-type projection rules, NOT raw payload") and W4-D is the substrate authority for those rules per its own acceptance ("WP may not deviate"). Added in L0 cycle-3.
* Phase 0 artifact 14 (binding contract).

#### Shape A — New capability or ability

* **Category:** Surface implementation (Gutenberg block consuming a Read+Transform ability).
* **Call-graph effect:** Read (the block invokes a Read ability; no mutation).
* **Input type:** Block attributes (`account_id`, etc.).
* **Output type:** Rendered HTML (server-side via `render.php`).
* **Composition:** Consumes `Composition` output.
* **Consumers:** WP editors and end users; future entity-page templates.

---

### Three-view consistency: concurrency contract implementation

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `composition`, `trust-boundary`, `abilities-runtime`
**Estimate:** L (5-6 days)

#### Problem

Multiple surfaces (Tauri runtime, MCP, WP editor, markdown filesystem) can observe and attempt to mutate projected claims. Without a concurrency contract, stale writes overwrite fresh state, concurrent edits race, and the substrate loses its role as concurrency authority. Per Phase 0 artifact 02, the contract is server-assigned monotonic `claim_version: u64`.

#### Why now

Wave 4 ships the first block that consumes and re-renders composition state. Wave 5 ships feedback writes. Both depend on the watermark contract being in place before the block can write back safely.

#### Scope limits

This issue lands the concurrency model only — the watermark, the stale-write rejection, the version increment rules. Tamper detection (out-of-band edits, projection signing) is the next issue. Conflict-free replicated data types are explicitly out of scope per artifact 02 §"Non-goals."

#### Acceptance criteria

* `claim_version: u64` per claim, server-assigned, incremented exactly once per accepted mutation to authoritative state. Stable per `claim_id`. Never generated WP-side, MCP-side, browser-side, or agent-side.
* Hybrid logical clock recorded for ordering/replay/diagnostics — NOT used as conflict authority.
* `composition_version` per composition, server-assigned, monotonic per `composition_id`.
* Every projected `Block` carries `composition_version` AND per-block `claim_version`s for the claims it references.
* Write path: every mutation request from a `SurfaceClient` includes its observed `claim_version`. Server rejects mutations whose observed version is below the current version with `409 STALE_VERSION` + the current version in the response.
* Read path: surfaces refresh on `409 STALE_VERSION` by re-invoking the producing ability and receiving the current composition.
* Concurrent edits from two surfaces racing the same claim — exactly one mutation succeeds; the other receives `409` and refreshes.
* Tab-switch test: user opens two WP editor tabs on the same page, edits in tab A, then attempts to edit in tab B — tab B's save returns `409` and prompts refresh.
* No polling fallback — surfaces consume signal-driven refresh per v1.4.1 W1 signal infrastructure.
* Negative fixtures from artifact 02 §"Negative cases": stale-write, concurrent-write, watermark-overflow (large `claim_version` rollover defense), missing-watermark-on-write — each pass.
* Watermark fields documented in the `Composition` and `Block` types from Wave 1.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched directly. Watermark change → signal emission → downstream invalidation. Consumes v1.4.1 W1-B/E policy registry contract.
* **Health scoring:** Not touched.
* **Intel context:** Not touched.
* **Briefing callouts:** Not touched.
* **Feedback hook:** Touched. Feedback writes consume the watermark contract; stale feedback rejected.

#### Architectural surfaces touched

* [x] Services layer — Touched directly.
* [x] Abilities contract — Touched. Compositions carry watermarks.
* [x] Provenance — Touched indirectly. Watermark events recorded in audit log.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched directly (hybrid logical clock is recorded but not authority).
* [x] Claims layer — Touched directly. `claim_version` lives on the claim.
* [x] Signal granularity — Touched. Version-change events emit signals.
* [x] Migration — Touched. `intelligence_claims` table gains `claim_version` column.
* [x] Evaluation harness — Touched.
* [x] Surface parity — Touched directly.
* [ ] Privacy rendering — Not touched.

#### Edge cases

* Empty watermark on first save — Handled: server assigns initial version `1`.
* Stale watermark — Handled: `409 STALE_VERSION`.
* Null `claim_version` on write — rejected at bridge.
* Race: two concurrent writes with the same observed version — Handled: one succeeds, one gets `409`.
* User intent persistence — Handled: dismissed claims preserve their tombstone state through version increments.
* Value instability — N-A; versions are deterministic per server.
* Revoked source — Handled per existing claim-lifecycle rules.

#### Dependencies

* Issue: `Composition contract substrate types + ProvenanceRef shape`.
* Issue: `SurfaceClient as fourth actor class`.
* v1.4.1 W1 signal infrastructure (durable invalidation jobs).
* Phase 0 artifact 02 (binding contract).

#### Shape B — Schema or data-model change

* **New tables or columns:** `intelligence_claims.claim_version: INTEGER NOT NULL DEFAULT 1`; new index `claim_id, claim_version`. `compositions.composition_version: INTEGER NOT NULL DEFAULT 1` (if compositions are persisted; otherwise version is computed per request).
* **Append-only vs mutate-in-place:** versions mutate in-place on the claim row; old versions are recoverable from the existing append-only claim history (v1.4.0).
* **Negative knowledge / tombstones:** existing tombstone semantics preserved; tombstone events also increment `claim_version`.
* **Pruning policy:** N-A (versions are a small integer column).
* **Read path:** every claim read returns `claim_version`; surfaces store and replay it.
* **Write path:** version increment is atomic within the claim-mutation transaction (the existing single-writer service path).

#### Shape C — Migration replacing existing capability or table

* **Parallel-run plan:** N-A — the migration is additive. Existing claims default to `claim_version = 1`.
* **Divergence monitor:** N-A.
* **Cutover criteria:** N-A.
* **Rollback path:** drop the new column + index; old read/write paths still work.
* **Backfill strategy:** none required; `DEFAULT 1` handles the backfill at column-add time.

---

### Tamper detection contract: projection signing + verification

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `composition`, `trust-boundary`, `abilities-runtime`
**Estimate:** L (5-7 days)

#### Problem

WP DB rows and markdown files are projection targets; users (and admins, and migration tools, and DB-restore workflows) can edit them directly, bypassing the runtime. Without tamper detection, an out-of-band edit silently becomes "current state" and the substrate is no longer the source of truth.

#### Why now

Wave 4 ships the first projected block; Wave 5 ships the markdown projection. Both need tamper detection in place from the start; retrofitting is harder than landing it now.

#### Scope limits

This issue lands projection signing on write and verification on read for both WP DB projections and markdown filesystem projections. It includes substrate→markdown emission with the signature envelope embedded in an HTML comment per artifact 03 §"Storage." It does NOT land bidirectional markdown↔substrate edit propagation (markdown-as-input edit ingestion) — that is v1.4.6 (Workspace Memory Refactor) scope per the project description. It does not land quarantine UI, automated repair, or reconciliation tooling — those are scoped follow-ups.

#### Acceptance criteria

* Every projection write (WP DB block save, markdown file write) carries a server-issued projection signature per Phase 0 artifact 03 §"Algorithm" + §"Storage."
* **Signing algorithm: Ed25519, and no other algorithm.** No HMAC, no RSA, no ECDSA, no algorithm negotiation per artifact 03 line 161. The signed bytes are the RFC 8785 canonical JSON serialization of the `SignedProjectionPayload`. The signature envelope carries `alg: "Ed25519"` and `canonicalization: "RFC8785-JSON"`. An explicit domain separator string `dailyos.wp_studio.projection.v1` is included in the signed payload (artifact 03 line 167).
* `SignedProjectionPayload` fields per artifact 03: `composition_id`, `composition_version`, `claim_refs[]`, `claim_versions[]`, `projection_target` (`wordpress_db` or `markdown_file`), `runtime_anchor_id`, `key_id`, `issued_at`, plus the domain separator.
* **Key custody:** the Ed25519 private signing key lives in the runtime keychain (macOS keychain entry owned by the Rust runtime, or platform equivalent). The **public verification key** is copied to WordPress (block attribute) and markdown (HTML-comment-embedded envelope), keyed by `key_id`. The private key is never written to WP, markdown, options, postmeta, transients, browser storage, or exported block JSON. `key_id` is a runtime-generated opaque id (not a path or keychain label).
* **Key lifecycle** (per Phase 0 artifact 03 §"Out-of-Band Detection" lines 380-441 + §"Fixture C: Signature Key Compromise Simulation" lines 763-794, lifted into W4-C acceptance per L0 cycle-3 codex-challenge finding #5):
  * **Unknown key refresh:** SurfaceClient encountering an envelope with an unrecognized `key_id` renders the projection as unverified and asks the runtime for a `ProjectionKeyRing` refresh; if the key is still unknown after refresh, the projection enqueues for reconciliation rather than rendering as trusted.
  * **Key ring distribution:** runtime exposes a `GET /v1/surface/keyring` route (paired-actor only, HMAC-gated) that returns the current set of `(key_id, public_key, status: active|retired|revoked)` tuples. WP plugin caches the keyring per session; cache invalidates on `key_id`-unknown verification failure.
  * **Revocation:** runtime marks a `key_id` revoked in the keyring; SurfaceClients seeing a signature under a revoked `key_id` treat it as a verification failure with `KeyRevoked` and render the tamper banner with revoked-key state (per artifact 03 line 386).
  * **Replacement key provisioning:** on revocation, the runtime provisions a fresh Ed25519 keypair in the runtime keychain, assigns a new `key_id`, distributes the public key via the next keyring fetch, and (per artifact 03 line 776) re-signs all live projections reachable from `projection_ledger`.
  * **Re-signing:** the runtime walks `projection_ledger`, regenerates `SignedProjectionPayload` (canonical bytes unchanged unless claim state changed), and writes the new signature into the projection's storage (WP block attribute update / markdown comment rewrite). `projection_ledger` records the old `signature_id` as revoked and the new one as active.
  * **Retired-key verification:** public keys for retired `key_id`s remain published in the keyring with `status: retired` for historical-projection verification (audit replays, ledger comparisons). Retired keys still verify the bytes they signed; they do NOT sign new projections. Revoked keys fail verification.
  * **Re-sign trigger queue:** scope-limit per Phase 0 artifact 03 open question (line 829): in v1.4.2, re-sign is queued (not synchronous) with the tamper banner showing until each projection is refreshed. Synchronous re-sign for visible projections is a v1.4.3 follow-up filed against the Codebase Maintenance & Production Quality project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).
* **Offline verification:** SurfaceClients (WP, future markdown consumer) verify projection signatures using the embedded public key — no runtime round-trip required. The runtime ledger (`projection_ledger`) stores the canonical signed payload for reconciliation comparisons only.
* Verification on read: every projection read parses the embedded signature envelope, locates the public key for `key_id`, verifies the Ed25519 signature against the RFC 8785 canonical JSON. Mismatch = tamper-detected.
* Tamper response: the projection is NOT silently promoted to canonical. Surface renders the substrate's authoritative state, displays a banner ("This page was edited outside DailyOS — substrate state shown"), and writes a `tamper_detected` audit event (via the W1-A0 `emit_surface_audit` helper) with `projection_target`, `projection_id`, `expected_signature_id`, `observed_signature`.
* Quarantine: the tampered projection is preserved on disk / in DB with a `.quarantined` suffix or status field; nothing is deleted. Manual reconciliation tooling is a follow-up issue, not this one.
* Out-of-band edit categories detected per artifact 03 §"Detection cases": direct WP DB row edit, direct markdown file edit, WP DB restore from backup, SQL import, markdown commit from a different machine.
* Signature reference lives outside the `ProvenanceEnvelope` per ADR-0108 64KB cap — the provenance envelope carries only `{ projection_authenticity: { projection_id, signature_id, key_id, alg, canonicalization, signed_at } }` per artifact 03 lines 193-205. Full canonical signed payload lives in `projection_ledger`.
* Negative fixtures from artifact 03: each detection case has a regression test that produces the named audit event. Plus: bad-signature, wrong-`key_id`, wrong-`runtime_anchor_id`, mismatched-domain-separator (cross-domain replay attempt) — each rejected and audited.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched. Tamper-detection events emit a signal that downstream surfaces consume to refresh.
* **Health scoring:** Touched indirectly — repeated tamper detection on the same projection feeds source-reliability state (over time, after v1.4.2).
* **Intel context:** Not touched.
* **Briefing callouts:** Tamper-detected projections may surface a callout in v1.4.3+; not in this issue.
* **Feedback hook:** Tamper detection is NOT a feedback event — it is an integrity event. Distinct from corrections/dismissals.

#### Architectural surfaces touched

* [x] Services layer — Touched. New projection-signing service.
* [x] Abilities contract — Touched indirectly.
* [x] Provenance — Touched. Signatures live alongside provenance, not inside it.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [x] Claims layer — Touched. Claim reads on a projection re-check signature.
* [x] Signal granularity — Touched. Tamper signal.
* [x] Migration — Touched. New `projection_ledger` table + quarantine state on existing projection rows.
* [x] Evaluation harness — Touched.
* [x] Surface parity — Touched.
* [x] Privacy rendering — Touched. Tamper banners are user-facing.

#### Edge cases

* Empty projection (no claims rendered) — Handled: Ed25519 signature over the canonical-JSON empty-blocks payload still issued.
* Stale signature (signed under a previous runtime anchor or key rotation) — Handled per artifact 03: signature carries `runtime_anchor_id` + `key_id`; mismatched anchor → re-verify under the anchor-rotation handshake from Wave 2; if still mismatched, quarantine. Public keys for retired `key_id`s remain published for historical verification.
* Null signature on a projection — treated as tampered.
* Race: signature write and projection write must be atomic — Handled via DB transaction.
* User intent persistence — Handled: user-dismissed claims preserve their tombstone signature.
* Value instability — Handled: Ed25519 is deterministic (artifact 03 line 159); same canonical payload → same signature given same private key.
* Revoked source — Handled per existing claim lifecycle.
* Masked provenance (ADR-0108 sensitivity masking) — Handled per artifact 03 line 217: signature remains valid for the historic projection bytes; rendered provenance details may be unavailable; masked provenance renders as masked, NOT as a signature failure.

#### Dependencies

* Issue: `Composition contract substrate types + ProvenanceRef shape`.
* Issue: `Three-view consistency: concurrency contract implementation` (provides watermarks).
* Issue: `Pairing handshake + four-path token recovery defenses` (provides `runtime_anchor_id`).
* Phase 0 artifact 03 (binding contract).

#### Shape B — Schema or data-model change

* **New tables or columns:** `projection_ledger` runtime-side table per artifact 03 §"runtime ledger" line 180: `projection_id`, `surface`, `surface_locator`, `block_id`, `claim_id`, `claim_version`, `signature_id`, `key_id`, `composition_id`, `composition_version`, `canonical_signed_payload_bytes`, `signature_bytes`, `runtime_anchor_id`, `alg`, `canonicalization`, `issued_at`, `quarantined_at`.
* **Append-only vs mutate-in-place:** append-only on issue; `quarantined_at` set in-place on tamper detection.
* **Negative knowledge / tombstones:** quarantined rows preserved.
* **Pruning policy:** quarantined rows retained until manual reconciliation; ledger rows for active projections retained for the lifetime of the projection.
* **Read path:** every projection read parses the embedded signature envelope (offline); the runtime queries `projection_ledger` only for reconciliation comparisons.
* **Write path:** projection-signing service is the only writer of `projection_ledger`; the projected block attribute or markdown comment is the SurfaceClient's offline-verifiable copy.

---

### Custom block fallback projection rules (substrate-side enforcement)

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `composition`, `trust-boundary`
**Estimate:** M (3-4 days)

#### Problem

A renderer that encounters an unknown `BlockType::Custom { type_id }` could naively render the raw payload. Per Phase 0 /cso refinement 9 + artifact 07, this is an information-disclosure vector — internal identifiers, sensitive fields, prompt fragments, debug carrier fields can all leak. The substrate must enforce projection rules at the boundary, not just the surface.

#### Why now

Wave 4 ships the first surface that consumes `Composition`. If fallback rules live only at the WP renderer level, every future surface (MCP, future browser, future mobile) re-implements them and drifts. Substrate enforcement is the load-bearing position.

#### Scope limits

This issue lands the substrate-side projection ruleset. The WP-side renderer in the first-block issue consumes it; no WP-side fallback logic is allowed to deviate. Future surfaces consume the same ruleset.

#### Acceptance criteria

* Substrate publishes a `projection_rule` per known `BlockType` describing which fields are admitted at which sensitivity tier per ADR-0125 (`Public | Internal | Confidential | UserOnly`).
* Unknown `BlockType::Custom { type_id }` resolves via nearest-known-type intersection at JSON-pointer granularity (per artifact 07 §"Projection algorithm"):
  * Find candidate known types whose schema overlaps with the unknown payload.
  * Intersect the unknown payload with the chosen known type's admitted-fields set.
  * Render ONLY the intersection — fields not admitted by the known type are dropped.
  * `claim_refs` always preserved.
  * Banner: "Rendered as nearest known type — payload may be incomplete." Banner text is product-vocabulary-disciplined.
* `ProvenanceRef` always preserved.
* Sensitivity gate: even an admitted field with sensitivity tier above the requesting actor's permission is masked per ADR-0108 rendering rules.
* Substrate emits a `custom_block_fallback_applied` audit event per fallback render: `composition_id`, `block_index`, `unknown_type_id`, `chosen_known_type`, `fields_dropped[]`.
* Cap: unknown-block count per composition is capped at N (configurable, default 5 per artifact 09 §Axis-specific reasoning). Excess unknown blocks are dropped + audit event emitted.
* Negative fixtures from artifact 12: unknown block with raw-payload fields verifies they are NOT rendered; banner is present; audit event emitted.
* The same ruleset is the source consumed by Wave 4 block renderer AND by any future surface.
* **Editable composition overlay semantics (per Phase 0 artifact 11).** Substrate-side projection rules also govern edit-routing per artifact 11's substrate-vs-overlay block taxonomy: an edit to a `Block.attribute` whose JSON-pointer maps to a `claim_ref` field is a substrate-bound feedback event (consumed by W5-A feedback router). An edit to a non-claim block attribute (e.g., display toggles, layout switches, surface-local annotations) stays WP-local and does NOT generate substrate writes. The ruleset publishes which attributes per `BlockType` are substrate-bound vs surface-local, so W5-A can route correctly without WP-side guessing. Artifact 11 also defines the save-handler behavior for paste / nesting / reorder across the substrate-overlay boundary; W4-D publishes the rules, W5-A consumes them at the save handler.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A. Fallback rendering is read-side.
* **Health scoring:** N-A in this issue.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** N-A. Fallback events do not generate feedback.

#### Architectural surfaces touched

* [x] Abilities contract — Touched. Projection rules per known BlockType ship alongside the ability registry.
* [x] Composition — Touched directly.
* [x] Provenance — Touched. ProvenanceRef preservation under fallback.
* [x] Privacy rendering — Touched directly. This IS privacy rendering for unknown block types.
* [x] Surface parity — Touched. One ruleset, every surface.
* [x] Evaluation harness — Touched.
* All others — Not touched.

#### Edge cases

* Empty payload on unknown block — Handled: banner + `claim_refs` only (which may be empty too).
* Stale `type_id` (renderer is newer than the producer; the unknown type IS now known) — Handled: renderer reads its known-type registry at request time, not at build time.
* Null fields in payload — Handled: dropped.
* Race: known-type registry update mid-render — Handled: registry reads are atomic snapshots.
* User intent persistence — Handled: fallback rendering does not affect user-corrected state.
* Value instability — Handled: same payload + same registry → same fallback projection.
* Revoked source — Handled per ADR-0108 sensitivity rules.

#### Dependencies

* Issue: `Adopt ADR-0130 amendments` (must merge first).
* Issue: `Composition contract substrate types + ProvenanceRef shape`.
* Phase 0 artifact 07 (binding contract).

#### Shape A — New capability or ability

* **Category:** Substrate primitive (rendering policy at the substrate boundary).
* **Consumers:** Wave 4 WP block renderer, future MCP renderer, future surfaces, audit log.

---

### User-presence nonce lifecycle for feedback writes

**Suggested milestone:** Wave 4 — Composition + First Block
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`, `wordpress-plugin`
**Estimate:** M (3-4 days)

#### Problem

HMAC proves the channel. Bearer proves the paired session. Rate limits prevent flooding. None of these prove that a human in the current editor session is deliberately acting on this specific claim field right now. Per Phase 0 artifact 10, feedback writes (corrections, dismissals, corroborations, contradictions) require a user-presence nonce.

#### Why now

Wave 4 ships the block; Wave 5 wires the feedback router. The nonce must be issued + verified by the time feedback writes flow.

#### Scope limits

This issue lands the nonce issue + verify path (runtime-side) and the JS-side nonce request from the block editor (WP-side). It does not land the full click-bound feedback router — that is Wave 5 (W5-A), which consumes this nonce.

#### Acceptance criteria

* Runtime endpoint `POST /v1/surface/nonce/issue` issues a single-use nonce per artifact 10 §"Binding payload": `nonce` (32B base64url), `session_id`, `wp_user_id`, `claim_id`, `field_path`, `action` (`correct | dismiss | corroborate | contradict`), `composition_version`, `generated_at`, `expires_at` (≤60s).
* Per /cso refinement P3: nonce binding includes `wp_user_id` (defends multi-user WP installs against cross-user nonce reuse).
* Nonce is server-bound; the runtime stores `(session_id, wp_user_id, claim_id, field_path, action, composition_version)` → nonce, with atomic consume-on-verify.
* Feedback write path verifies the nonce: extract claim_id + field_path + action from the request, find the bound nonce, mark consumed atomically. Mismatched action / wrong field / expired / already-consumed → `403`.
* WP block JS requests a fresh nonce when the user initiates a feedback action (clicks a "correct" or "dismiss" affordance). The nonce is bound to the click, not to the page load. **Transport model:** the WP block JS calls a WP REST endpoint (e.g., `/wp-json/dailyos/v1/nonce`); the WP PHP layer issues the HMAC-signed runtime request via `class-dailyos-runtime-client.php` (W3-B) to `POST /v1/surface/nonce/issue`. The browser does NOT call the runtime directly — the W2-A positive Origin allowlist would reject browser-originated requests with a WP-served Origin. This keeps browser-to-runtime trust mediated through PHP per artifact 15 §"transport."
* **Click-bound nonce vs save-bound model.** Per consult cycle-1 §"save-time nonce model": a user may edit a field, wait 90 seconds, then save — the issued 60s nonce would expire. Resolution: each discrete feedback gesture (correct, dismiss, corroborate, contradict) is its own click that issues + immediately consumes a nonce, with the runtime applying the feedback inline. Save-time persistence is for non-feedback attribute changes only (display/layout) — those do NOT consume nonces because they are surface-local per W4-D. This separates "immediate explicit feedback actions" (nonce-gated) from "block attribute autosave" (no nonce, no substrate write).
* Negative fixtures from artifact 12 §"Nonce cases": expired nonce, replayed nonce, cross-user nonce on multi-user install, mismatched-action nonce, mismatched-field nonce, missing-composition-version nonce — each rejected with `403`.
* Audit log: every nonce issue + consume + reject recorded with `wp_user_id`, `claim_id`, `field_path`, `action`, `result`.
* Nonces are NOT authorization tokens — verification runs alongside pairing/bearer/HMAC/scope/rate-limit checks.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A. Nonce is a freshness/presence gate.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Touched directly. Nonces gate the feedback path.

#### Architectural surfaces touched

* [x] Services layer — Touched.
* [x] Claims layer — Touched indirectly (feedback gate).
* [x] Trust-boundary — Touched directly.
* [x] Surface parity — Touched.
* [x] Evaluation harness — Touched.
* All others — Not touched.

#### Edge cases

* Empty nonce — rejected.
* Stale / expired nonce — rejected (`403 NONCE_EXPIRED`).
* Null `wp_user_id` in nonce binding — rejected at issue.
* Race: two clicks on the same field racing to consume the same nonce — Handled: atomic consume.
* User intent persistence — Handled: each click gets a fresh nonce.
* Value instability — Handled: nonce content is RNG-derived.
* Revoked source — Handled: nonces issued against a revoked pairing rejected at issue time.

#### Dependencies

* Issue: `Loopback HTTP runtime endpoint`.
* Issue: `HMAC-SHA256 request signing`.
* Issue: `Pairing handshake + four-path token recovery defenses`.
* Issue: `Three-view consistency: concurrency contract implementation` (provides `composition_version`).
* Phase 0 artifact 10 (binding contract).

#### Shape A — New capability or ability

* **Category:** Substrate primitive (presence-gate at the feedback boundary).
* **Consumers:** Wave 5 feedback router, audit log, eval harness.

---

## Wave 5 — Feedback + theme + negative fixtures (3 issues)

---

### Feedback router (click-bound) + presence-nonce-bound corrections

**Suggested milestone:** Wave 5 — Feedback + Theme
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `composition`, `feedback`
**Estimate:** L (5-6 days)

#### Problem

When a user dismisses a claim, corrects a value, corroborates, or contradicts via the affordance buttons on a `dailyos/account-overview` block, that explicit feedback gesture must travel back through the surface bridge as a typed feedback event, land on the substrate via the existing claim/feedback path, and re-render the block inline. Today there is no click-bound router that translates feedback-affordance events to substrate feedback events. (Per L0 cycle-3 codex-challenge #2 reconciliation: feedback is click-bound, not save-bound — Gutenberg save lifecycle handles only display/layout autosave per W4-D surface-local rules.)

#### Why now

The block ships in Wave 4 but is read-only without the feedback path. Without this issue, the foundation cannot demonstrate the load-bearing user outcome ("corrections stick").

#### Scope limits

This issue lands the click-bound feedback router for the `dailyos/account-overview` block per Phase 0 artifact 13 §"Feedback routing" (reconciled with W4-E click-bound nonce model at L0 cycle-3). It does not extend to other blocks (there is only one in v1.4.2) or to read-time feedback paths. Bulk feedback routing is a follow-up.

#### Acceptance criteria

* `class-dailyos-feedback-router.php` handles **explicit feedback-gesture clicks** emitted by the `dailyos/account-overview` block JS — `correct`, `dismiss`, `corroborate`, `contradict` affordance buttons on claim renderings. Per L0 cycle-3 codex-challenge #2 reconciliation: feedback is **click-bound, not save-bound**, to align with W4-E's click-bound presence-nonce lifecycle. Block-attribute autosave covers ONLY display/layout (surface-local) per W4-D substrate-published projection rules; surface-local autosaves do NOT consume nonces and do NOT emit feedback events.
* On each feedback-gesture click, the WP block JS calls `/wp-json/dailyos/v1/feedback` with the action-specific payload; the PHP router (this issue) receives the payload, requests the W4-E nonce via `class-dailyos-runtime-client.php`, attaches the (already-bound) nonce, and POSTs to `/v1/surface/feedback`. The runtime applies the feedback inline per the existing claim/feedback service contract.
* Each feedback event carries: `claim_id`, `field_path`, `action`, `value` (for corrections), `composition_version`, `wp_user_id`, `nonce`.
* Nonce per the Wave 4 nonce-lifecycle issue.
* Feedback events POSTed to `/v1/surface/feedback`; runtime applies them through the existing claim/feedback service (NOT a new substrate path — must use the v1.4.0/v1.4.1 path).
* On successful application, the block re-renders with the corrected state; trust band updates if applicable.
* Negative cases from artifact 12 §"Feedback cases": stale `composition_version` → `409`, missing nonce → `403`, wrong-actor nonce → `403`, mismatched-field nonce → `403`. Each verified.
* Audit log entry per feedback application: `feedback_applied` emitted via the W1-A0 `emit_surface_audit` helper with `wp_user_id`, `surface_client_id` (`actor_instance`), `claim_id`, `field_path`, `action`, `outcome`.
* **Editable composition overlay routing.** The router consults the W4-D substrate-published projection rules to determine whether the clicked affordance's `field_path` is substrate-bound (claim-ref-mapped) or surface-local (display/layout). Only substrate-bound clicks emit feedback events; affordances on surface-local fields are inert (no event, no nonce consumed). This codifies the artifact-11 semantics (per Phase 0 artifact `11-editable-composition-overlay.md`, promoted to spec:ready by W0-C) into the W4-D + W5-A boundary.
* The router does NOT write to `wp_options` or any substrate-side state directly. All mutations flow through the runtime feedback endpoint.
* CI lint: no direct claim-table write from the WP plugin source tree.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched. Feedback application emits the existing `ClaimCorrected`/`ClaimDismissed`/`ClaimCorroborated`/`ClaimContradicted` signals via the existing service path.
* **Health scoring:** Touched. Feedback feeds the existing trust-scoring inputs.
* **Intel context:** Not touched (context-build is upstream).
* **Briefing callouts:** A dismissed claim may suppress a related callout via the existing dedup rule.
* **Feedback hook:** Touched directly. This IS the WP-side feedback hook.

#### Architectural surfaces touched

* [x] Services layer — Touched. Routes through existing claim/feedback service.
* [x] Abilities contract — Touched. Feedback events use the typed feedback shape.
* [x] Provenance — Touched. Feedback application records actor + nonce.
* [ ] Execution mode — Not touched.
* [ ] Source taxonomy — Not touched.
* [ ] Temporal primitives — Not touched.
* [x] Claims layer — Touched directly.
* [x] Signal granularity — Touched.
* [ ] Migration — Not touched.
* [x] Evaluation harness — Touched.
* [x] Surface parity — Touched.
* [x] Privacy rendering — Touched.

#### Edge cases

* Click on a non-claim-bound affordance (e.g., display-only attribute) — Handled: router emits no event; only claim-ref-mapped attributes trigger feedback per W4-D rules.
* Stale `composition_version` — Handled: `409`, surface refreshes.
* Null nonce — rejected.
* Race: two rapid feedback clicks on the same field — Handled via atomic nonce consume (W4-E) + composition_version watermark.
* User intent persistence — Handled directly: dismissals stick across refreshes (existing tombstone semantics).
* Value instability — Handled: feedback writes are idempotent on the same nonce.
* Revoked source — Handled: feedback events on revoked-source claims still apply (user intent overrides source state per existing rules).

#### Dependencies

* Issue: `dailyos/account-overview Gutenberg block`.
* Issue: `User-presence nonce lifecycle for feedback writes`.
* Issue: `Three-view consistency: concurrency contract implementation`.
* Existing v1.4.0 claim/feedback service path.
* Phase 0 artifact 13 (binding contract).

#### Shape A — New capability or ability

* **Category:** Surface implementation (WP-side feedback router).
* **Call-graph effect:** Indirect mutation — the router does not mutate substrate directly; it submits typed feedback events that the existing service path applies.
* **Consumers:** End users (correction flow), v1.4.1 trust scoring, future audit/inspection surfaces.

---

### Magazine theme: editorial shell + tokens port + block styling

**Suggested milestone:** Wave 5 — Feedback + Theme
**Suggested labels:** `spec:draft`, `Feature`, `wordpress-plugin`, `design-system`
**Estimate:** L (5-7 days)

#### Problem

WordPress's default themes do not match DailyOS's editorial reading surface (FolioBar, FloatingNavIsland, AtmosphereLayer, MagazinePageLayout, FinisMarker, typography, color palette, spacing scale). Without a DailyOS theme, the foundation renders with the wrong visual hierarchy and "WordPress aesthetic baggage" (per ADR-0129 §Negative) shows through.

#### Why now

The clean-machine validation in the DoD requires a published-quality render. Without the theme, the demo lands but the visual fidelity gap masks the architectural win.

#### Scope limits

This issue lands a block theme implementing the magazine shell + tokens + styling for the `dailyos/account-overview` block. It does not land patterns library, custom post types, or templates for entity/briefing/report types (those are v1.4.3+).

#### Acceptance criteria

* WordPress block theme `dailyos-magazine` per WP 6.9+ block-theme conventions.
* `theme.json` ships DailyOS design tokens ported from `src/styles/design-tokens.css` and `.docs/design/reference/_shared/styles/design-tokens.css` — keeping the three sources in sync per CLAUDE.md.
* Magazine shell components: FolioBar (header), FloatingNavIsland (nav), AtmosphereLayer (background), MagazinePageLayout (page shell), FinisMarker (end marker). Implemented as block-theme parts and patterns per ADR-0083 product-vocabulary discipline.
* Typography stack matches `.docs/design/typography/` specs.
* Color palette matches `.docs/design/color/` specs; trust band colors from existing trust-band tokens.
* Spacing scale matches existing tokens.
* Block style for `dailyos/account-overview` consumes theme tokens via `theme.json` and the block's `editor.scss` + `style.scss`.
* Editorial rules per CLAUDE.md "DailyOS visual rules": magazine not dashboard, cards for featured content only, color communicates state.
* No hardcoded colors / fonts / spacing scales / shadows / radii / trust bands / entity colors in theme code; every value reads from `theme.json` per the v1.4.x token discipline.
* WP Theme Check passes.
* Light + dark mode supported (matches existing token sets).
* No raw pipeline vocabulary in any theme-supplied copy per CLAUDE.md.

#### Intelligence Loop fit — CLAUDE.md critical rule

All N-A — visual layer only; consumes the substrate-side trust/provenance rendering.

#### Architectural surfaces touched

* [x] Privacy rendering — Touched indirectly. Trust band rendering must match existing token-driven treatment.
* [x] Surface parity — Touched. WP theme is the visual surface contract.
* All others — Not touched.

#### Edge cases

* Empty page (no DailyOS blocks) — theme still renders the magazine shell; FinisMarker shows at the end.
* Stale token (token added to design system mid-flight) — Handled: theme reads from `theme.json` which mirrors `design-tokens.css`; sync is enforced.
* Null block attributes — block renders empty placeholder per WP conventions.
* User intent persistence — N-A in theme layer.
* Race / value instability — N-A.
* Revoked source — N-A at the theme layer.

#### Dependencies

* `.docs/design/` design system specs (cite at L0 plan time).
* Existing `src/styles/design-tokens.css` and `.docs/design/reference/_shared/styles/design-tokens.css`.
* Issue: `dailyos/account-overview Gutenberg block`.

#### Shape A — New capability or ability

* **Category:** Surface implementation (WP block theme).
* **Consumers:** End users viewing rendered DailyOS pages, WP editors composing pages.

---

### Negative fixture catalog implementation: every named failure case from artifact 12

**Suggested milestone:** Wave 5 — Feedback + Theme
**Suggested labels:** `spec:draft`, `Feature`, `evaluation-harness`, `trust-boundary`
**Estimate:** L (5-7 days)

#### Problem

Phase 0 artifact 12 names ~30 negative fixtures across pairing, projection freshness, rate-limits, ability discovery, allowlist, SurfaceClient identity, nonce, Gutenberg renderer, custom block fallback, markdown projection, audit, diagnostics. Without these as actual tests, the foundation's failure modes are unverified.

#### Why now

The DoD requires "all named failure cases from Phase 0 artifact 12 pass at the boundary that fails, not only as end-to-end checks." This is the verification harness for the release.

#### Scope limits

This issue lands every fixture named in artifact 12. Individual fixtures are owned by their producing issue (e.g., HMAC negative fixtures live in the HMAC issue), but the consolidation, the per-fixture catalog file, and the release-gate integration live here. New fixtures discovered during implementation are scoped here.

#### Acceptance criteria

* Every fixture from artifact 12 has a concrete test in the codebase at the boundary that fails per artifact 12 §"Fixture execution contract":
  * Use deterministic clocks for version, nonce, rate-limit, retry windows.
  * Use stable IDs for `composition_id`, `projection_id`, `surface_client_id`, `wp_user_id`, `site_id`, `ability_name`.
  * Seed one positive control per fixture.
  * Assert denial AND non-mutation for write-adjacent failures.
  * Assert response bodies do not leak hidden ability names, raw payloads, source excerpts, prompt text, local paths, internal provenance trees.
  * Assert ability body NOT invoked when failure expected at auth/discovery/allowlist/schema/rate-limit boundary.
  * Assert audit/diagnostic events include enough context.
* Fixture catalog file `.docs/plans/dos-546/v1.4.2-project/fixture-catalog.md` enumerates every fixture, its source (artifact 12 + which §), its target test file, its current status.
* Fixtures integrated into `pnpm release-gate -- --mode hermetic` so a release cannot ship with any fixture failing.
* Bundle naming: new v1.4.2 fixture bundles `19` (pairing+HMAC), `20` (composition+watermark+tamper), `21` (rate-limits+nonce), `22` (custom block fallback), `23` (MCP allowlist + ability discovery), `24` (audit attribution — schema lands W1-A0, fixtures consolidated here).
* Six bundles landed (19-24); each bundle's fixtures are produced by their owning wave issues (the consolidation + release-gate integration is this issue's load-bearing work).
* Each bundle has the documented quarantine policy per v1.4.1 W6 gate (flake → single-bundle quarantine + ticket; no release with a quarantined bundle).

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Indirect — some fixtures cover signal-routing under SurfaceClient failure paths.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Indirect — feedback-path negative fixtures live here.

#### Architectural surfaces touched

* [x] Evaluation harness — Touched directly.
* [x] Surface parity — Touched via the release gate.
* All others — Not touched (fixtures consume existing implementations).

#### Edge cases

All fixture-internal; documented per fixture in artifact 12.

#### Dependencies

* All Wave 1-4 implementation issues (provides the surfaces the fixtures test).
* Phase 0 artifact 12 (binding contract).

#### Shape A — New capability or ability

* **Category:** Evaluation harness extension.
* **Consumers:** Release gate, every L3 wave review, every L4 surface QA.

---

## Wave 6 — Audit, runtime launcher, clean-machine validation (2 issues)

---

### Audit attribution: forensic exercise + CI lint + production hardening

**Suggested milestone:** Wave 6 — Release Gate
**Suggested labels:** `spec:draft`, `Feature`, `trust-boundary`, `audit`
**Estimate:** M (2-3 days)

#### Problem

Per /cso refinement 4 + ADR-0111 §8, every substrate operation log entry must carry SurfaceClient instance identity AND WP `user_id`. **The audit-log schema + emission contract + `emit_surface_audit` helper land in W1-A0** so every W2-W5 emission site uses the canonical shape from inception. This issue closes the audit-attribution work by exercising the forensic round-trip end-to-end, landing the CI lint that enforces the invariant on new emission sites going forward, and hardening any production gaps observed across W2-W5.

#### Why now

Before release validation, the audit attribution behavior must be verified end-to-end (not just at the schema level). The CI lint codifies the invariant for v1.4.3+ work; without it, future PRs can introduce new SurfaceClient emission sites that omit the fields.

#### Scope limits

This issue does NOT land the schema migration, the emission helper, or the round-trip plumbing — those are W1-A0 scope. It does land: the forensic exercise documentation + reproduction, the CI lint enforcing both fields on every `Actor::SurfaceClient` log site, the migration of any v1.4.0/v1.4.1 emission sites discovered to be incomplete during W2-W5, and an ADR amendment formalizing the audit-attribution contract.

#### Acceptance criteria

* CI lint: every audit-log emission site that runs for a `SurfaceClient` actor calls the W1-A0 `emit_surface_audit` helper (not direct `audit_log` row writes). Missing helper call = build failure. The lint runs on every PR via the existing CI workflow.
* Forensic exercise (documented + reproduced): given a sample correction event in the audit log, the originator's `wp_user_id` and `actor_instance` are derivable in a single SQL query; given a `wp_user_id`, all corrections that user submitted in a date range are listed via the documented forensic-query template.
* End-to-end verification: a paired SurfaceClient (W3-B WP plugin) submits a correction; the audit row is written with `actor_instance` AND `wp_user_id` populated; a second negative test submits a malformed request without `wp_user_id` and confirms endpoint rejection per artifact 12.
* Migration sweep: any v1.4.0/v1.4.1 emission site that was NOT updated to the new shape during W1-A0 (because it was discovered only during W2-W5 implementation) is updated in this issue. Default values for non-SurfaceClient calls preserve existing log volume + content.
* Negative fixtures: SurfaceClient request without `wp_user_id` — rejected at endpoint; emission for `Actor::SurfaceClient` without `actor_instance` — runtime contract error.
* Audit-log contract documented in `.docs/decisions/` (either an ADR amendment to 0111, or a small new ADR for audit attribution if 0111 is too crowded).

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** N-A.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** N-A.
* **Feedback hook:** Touched indirectly — feedback application now logs with full attribution.

#### Architectural surfaces touched

* [x] Provenance — Touched.
* [x] Services layer — Touched (log emission).
* [x] Trust-boundary — Touched directly.
* [x] Surface parity — Touched.
* [x] Migration — Touched (log schema).
* [x] Evaluation harness — Touched.
* All others — Not touched.

#### Edge cases

* Empty `wp_user_id` (anonymous WP request) — rejected at endpoint per artifact 12.
* Stale `wp_user_id` (WP user deleted between request and log emission) — Handled: log records the value as observed; downstream tools resolve `null` if user no longer exists.
* Null `actor_instance` for SurfaceClient — rejected.
* Race: log emission and log read racing — Handled via existing log infrastructure.

#### Dependencies

* Issue: `SurfaceClient as fourth actor class`.
* Issue: `Loopback HTTP runtime endpoint`.
* Issue: `DailyOS WordPress plugin skeleton`.

#### Shape — Maintenance + CI enforcement

* **What changes:** CI lint added; forensic-query template documented; ADR amendment landed. Schema columns + helper are W1-A0's scope and are already merged before this issue starts.
* **How we prove it:** new CI run on a PR that adds a SurfaceClient emission site without the helper fails; forensic-query template reproduces the originator from a real audit row; the negative fixture from artifact 12 runs in the release gate.
* **Why now:** locks the contract before v1.4.2 ships so v1.4.3+ surface work cannot regress attribution.

---

### Clean-machine validation + dev-mode runtime launcher

**Suggested milestone:** Wave 6 — Release Gate
**Suggested labels:** `spec:draft`, `Feature`, `tauri-runtime`, `release`
**Estimate:** L (5-7 days)

#### Problem

The release DoD requires a clean macOS test box to install the DailyOS bundle and reach a rendered briefing in WordPress Studio in under 15 minutes of user time. Today no such flow exists end-to-end; the Tauri app is the only install path; the WP plugin is unbundled; the pairing flow is manual.

#### Why now

This is the final integration step. Without clean-machine validation, the foundation ships but the user outcome is unverified empirically (per ADR-0129 §10's explicit gate: "instant-launch feel" is empirical).

#### Scope limits

This issue lands a dev-mode runtime launcher + a bundle layout that can be installed on a clean machine. Production-grade install signing is explicitly out of scope (companion ticket); dev-mode signing is sufficient for the release validation.

#### Acceptance criteria

* Bundle layout: Tauri app + DailyOS WP plugin (`.zip` ready for WP install) + DailyOS magazine theme (`.zip` ready for WP install) + bootstrap script that:
  * Verifies WordPress Studio (or alternative local WP install at a documented path) is reachable.
  * Installs the plugin + theme via WP-CLI or via Studio's plugin/theme upload.
  * Activates the plugin + theme.
  * Prints the runtime pairing code.
  * Provides the WP admin URL to complete pairing.
* Runtime launcher (Tauri app) prints the pairing code in a dedicated UI surface (per Phase 0 artifact 04 §"Tauri continues" — visible controls remain at the Tauri host).
* `dailyos doctor` (or equivalent diagnostic command) reports: runtime status, listening port, paired surface clients, current pairing scopes, signal job queue depth, last successful briefing render.
* End-to-end test: a clean macOS test box (no prior DailyOS state **and no prior WordPress Studio install**) runs the bootstrap, completes pairing, navigates to a daily briefing page in WP, sees the `dailyos/account-overview` block render against a seeded account fixture, trust bands visible, provenance click-through works.
* **Total user time from "double-click installer" to "rendered briefing" ≤ 15 minutes (recorded as evidence in the PR — screen recording or stopwatch log).** Time budget covers: WordPress Studio download + install (bootstrap walks the user through this if Studio is absent), DailyOS bundle install (Tauri app + plugin `.zip` + theme `.zip`), runtime launch, pairing handshake, first briefing render. The clean-install validation MUST exercise the Studio-acquisition path on at least one validation run; an alternate run from an existing Studio install is also recorded but is not the primary evidence.
* Brand/positioning: README and onboarding copy describe DailyOS as "personal intelligence runtime" rather than "Mac app." Per ADR-0129 §1 brand reframe.
* Documentation: a `INSTALL.md` describes the install flow, the recovery path (re-pair if the runtime restarted), the troubleshooting matrix (runtime not reachable, pairing code expired, plugin activation failed, MCP server not connecting).
* The full Tauri-UI-fate decision is NOT made here. Per ADR-0129 §7, that decision is deferred to empirical evaluation after WP stabilizes; this release ships the Tauri app as runtime host + dev surfaces.

#### Intelligence Loop fit — CLAUDE.md critical rule

* **Signals:** Touched indirectly — the launcher confirms signal infrastructure is live.
* **Health scoring:** N-A.
* **Intel context:** N-A.
* **Briefing callouts:** Indirect — first rendered briefing exercises the callout path.
* **Feedback hook:** Indirect — the launcher confirms the feedback path is reachable.

#### Architectural surfaces touched

* [x] Services layer — Touched indirectly.
* [x] Surface parity — Touched directly.
* [x] Privacy rendering — Touched (first-run flow).
* [x] Evaluation harness — Touched.
* All others — Not touched.

#### Edge cases

* Empty: clean machine with NO local WP install — bootstrap walks the user through Studio acquisition (documented step with download URL + verified-checksum); Studio install is part of the validated ≤15min path, not an out-of-scope branch.
* Stale: an old DailyOS install exists on the test box — bootstrap detects and prompts user (do not silently merge state).
* Null pairing code (runtime not started) — bootstrap waits with a clear status; does not silently fail.
* Race: bootstrap running while WP-CLI is also active — Handled by mutex or serial step ordering.
* User intent persistence — Handled: pairing state preserved on re-run unless user explicitly resets.
* Value instability — Handled: bootstrap is deterministic given the same machine state.
* Revoked source — N-A at install time.

#### Dependencies

* All Wave 1-5 implementation issues.
* Phase 0 artifact 04 (runtime-host inventory).
* `.docs/decisions/0129-...` brand-reframe language.

#### Shape A — New capability or ability

* **Category:** Surface implementation (install + bootstrap flow).
* **Consumers:** First-run users, release validation, future onboarding documentation.

---

## Summary

| | |
|---|---|
| Total issues | 28 |
| Waves | 7 (W0, W1, W2, W3, W4, W5, W6) |
| Phase 0 artifacts cited | 15 (01-15) |
| Estimated wave-by-wave wall-clock | W0: 1-2d, W1: 1w, W2: 1w, W3: 1w, W4: 1.5w, W5: 1w, W6: 4-5d. Total ~6 weeks across the wave ladder (W0-D/W1-A0/W4-A0 land within existing wave windows). |
| Definition of Done | Matches v1.4.2 project DoD verbatim. |

**Issue-to-artifact map (for L0 dispatch):**

| Wave | Issue | Phase 0 artifact |
|---|---|---|
| W0 | ADR-0130 amendments | 06, 07 |
| W0 | Supersession + parking lot | (project hygiene) |
| W0 | Phase 0 INDEX + spec:ready | (cross-cutting) |
| W0 | **ADR-0102 §7.1 amendment (mcp_exposure tri-state)** | (ADR-0102 §7.1 + §7.6; artifact 05) |
| W1 | **Audit-log schema for SurfaceClient attribution** | 04 (host inventory); /cso ref 4; ADR-0111 §8 |
| W1 | SurfaceClient actor class | 04 (host inventory), 05 (surface mapping) |
| W1 | AbilityPolicy canonical schema | (ADR-0102 §7.1+§7.6) |
| W1 | Ability-surface inventory format | 05 |
| W1 | Ability-description CI gate | (CLAUDE.md + /cso ref 6) |
| W1 | Composition contract + ProvenanceRef | 06, 07 (consumes the amendments) |
| W2 | Loopback HTTP endpoint | 15 |
| W2 | HMAC-SHA256 signing | 08 |
| W2 | Pairing handshake + 4 defenses | 01 |
| W2 | Rate-limit matrix | 09 |
| W3 | WP plugin skeleton | 13 |
| W3 | WP runtime client + HMAC + pairing UI | 08, 13 |
| W3 | Custom MCP server + low-cap user | 13 (§Custom MCP server), ADR-0129 §4 |
| W4 | **account-overview ability (producer)** | 14, ADR-0130 §1 |
| W4 | account-overview block (renderer) | 14 |
| W4 | Concurrency contract impl | 02 |
| W4 | Tamper detection + projection signing | 03 |
| W4 | Custom block fallback projection | 07, 11 |
| W4 | User-presence nonce lifecycle | 10 |
| W5 | Save-time feedback router | 13 (§Feedback routing), 10, 11 |
| W5 | Magazine theme | (.docs/design/ + ADR-0083) |
| W5 | Negative fixture catalog | 12 |
| W6 | Audit attribution round-trip | (/cso ref 4) |
| W6 | Clean-machine validation + launcher | 04, ADR-0129 §10 |
