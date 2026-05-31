# DOS-285 W7-A L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 7 - Release gate hardening + close.
- **Agent:** W7-A.
- **Linear issue:** DOS-285 - "v1.4.0 review — Linear issues on entity pages (deferred from DOS-75)" (verbatim in §2 + §5).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. Only file created: `.docs/plans/v1.4.1-waves/W7-A-L0-packet.md`.
- **W7-A assignment:** entity page component for Linear issue display. Source: `.docs/plans/v1.4.1-waves.md:671-675`.
- **W7 merge gate:** v1.4.1 release-gate close. Source: `.docs/plans/v1.4.1-waves.md:698-712`.
- **Reviewer contract:** qa-expert reviewer + design-reviewer (UI) on L0 panel. The wave plan doesn't explicitly name design-reviewer for W7-A, but DOS-285 §1.3 lists Phase 2 signal emission + Bayesian source weights — a substantial intelligence-loop change beyond UI display, so plan-design-review on the surface is appropriate.
- **Runtime contract:** backend support already exists (`inject_linear_issues()` in `src-tauri/src/prepare/meeting_context.rs`). W7-A wires the frontend chapter + signal emission rules + health scoring impact.

## 2. Load-Bearing User Outcome

DOS-285 was the deferred Phase 2 of DOS-75 (Linear issues display). The original DOS-75 recommended Option B (read-only display) but the 5-question Intelligence Loop check rejected it for v1.2.2 because display-only data surfaces violate CLAUDE.md's critical rule. DOS-285 is the v1.4.0/v1.4.1 re-evaluation:

> "1. Is this still relevant? v1.4.0 is the Abilities Runtime release. Revisit whether Linear issues-on-entity-pages serves the abilities model, or whether it's been superseded by broader context/provenance work."

> "2. If still relevant, slot it in properly: don't ship Option B (Phase 1 display-only) alone. Bundle with Phase 2 (signal emission on state change, health scoring rules, briefing callouts, Bayesian source weights). Scope jumps from ~2 hours to ~6–8 hours but the result is CLAUDE.md-compliant."

The load-bearing outcome: **Linear issues for an account/project become claim-substrate input — they emit signals on state change, contribute to health scoring, surface in briefing callouts, and respect ADR-0108 sensitivity (internal-only project issues don't leak to MCP).**

Required from wave plan §675:

> "Entity page renders Linear issue chapter; respects ADR-0108 sensitivity (issues from internal-only projects don't leak to MCP responses)."

Required from DOS-285 §1.4 decision points:

- Keep separate from pushed actions (no dedup)
- Scope: account detail + project detail (decision needed)
- Signal emission rules for state changes
- Health scoring impact (e.g., blocked issue → technical debt signal)
- Briefing callout trigger conditions

## 3. Pre-Work

- **Read W7 source of truth.** `.docs/plans/v1.4.1-waves.md:671-675` assigns W7-A entity page component for Linear issue display + ADR-0108 sensitivity respect.
- **DOS-285 frames acceptance as 5-question Intelligence Loop check.** Display alone is rejected; signal + health + callout + sensitivity required.
- **Backend support exists.** `src-tauri/src/prepare/meeting_context.rs::inject_linear_issues()` already pulls Linear issues for meeting context. Reused at entity-page scope.
- **ADR-0108 sensitivity reference.** Internal-only project issues must not surface in MCP responses. Render policy + actor filtering applies.
- **Bayesian source weights.** Linear-issue-state changes feed into the trust factor library (ADR-0114). Trust factor: a state change on a known issue is a stronger signal than a new uncategorized issue.
- **Briefing callout precedent.** `prepare_meeting` topic generation already consumes account/project context. Adding a "what changed in Linear this week" callout fits the existing pattern.

## 4. Architecture

### 4.1 Files Owned

- **Frontend:**
  - `src/components/entity/LinearIssuesChapter.tsx` (new component).
  - `src/pages/AccountDetail.tsx` (integrate chapter on account-scope entity pages).
  - `src/pages/ProjectDetail.tsx` (integrate chapter on project-scope entity pages — if scoping decision includes project pages).
- **Backend:**
  - `src-tauri/src/services/linear.rs` extension for entity-scoped issue queries.
  - `src-tauri/src/services/signals/linear_issues.rs` (new): signal emission on issue-state changes.
  - `src-tauri/abilities-runtime/src/abilities/trust/factors/linear_issue_state.rs` (new): Bayesian source weight contribution.
- **Briefing callout:**
  - `src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs` extension to surface Linear-issue-change callouts.

### 4.2 Frontend Component

`LinearIssuesChapter.tsx`:
- Props: `entityRef: EntityRef`, `actorScope: ActorScope` (from context).
- Calls a service to fetch claim-backed Linear issues for the entity.
- Renders read-only list grouped by status (Open / In Progress / Blocked / Done).
- Respects ADR-0108: if any issue's `source_lifecycle_state == "restricted"` for the actor, the issue is hidden or shown as a redacted summary.
- No interactive update (Phase 3 deferred per DOS-285 §1.3).
- Uses existing `FinisMarker` convention if the chapter is the page's final block.

### 4.3 Signal Emission Rules

`signals/linear_issues.rs` extends the existing signal infrastructure (W1) to emit signals on these state changes for any tracked Linear issue:

- `state_changed_to_in_progress`
- `state_changed_to_blocked` (high-priority signal — feeds health scoring)
- `state_changed_to_done` (resolution signal — feeds positive trend)
- `assignee_changed` (lighter signal — context only)
- `priority_changed_to_urgent` (escalation signal)

Each signal carries: `source_ref` (Linear issue ID), `subject_ref` (parent account/project), `claim_type`, `source_asof` (Linear's update timestamp).

### 4.4 Health Scoring Impact

Linear-issue state signals feed the existing health/risk scoring per ADR-0114:

- Blocked issues count contributes to a "technical debt" or "delivery risk" health factor.
- Long-duration "In Progress" without resolution flags stalled work.
- Issue resolution rate (Done / Total per period) contributes to positive trend.

The exact factor weights are tuned through the same scoring unification path as the rest of the trust/health system — not hardcoded in this packet.

### 4.5 Briefing Callout Conditions

`prepare_meeting/synthesis.rs` surfaces a Linear-issue callout in topics when:

- A tracked issue changed state since the last briefing AND the subject of the issue is the meeting's primary entity.
- An issue moved to `Blocked` and is not yet resolved.
- Multiple issues moved to `Done` for the meeting's subject (positive change worth surfacing).

The callout includes provenance: issue link, source_asof, state-change summary.

### 4.6 ADR-0108 Sensitivity Sweep

The 9-channel sweep from W6-E bundle 17 applies here. Linear issues from internal-only projects must:

- NOT leak through MCP responses to `Actor::Agent`.
- NOT appear in customer-facing prep suggestions (callouts).
- NOT appear in eval-fixture content (synthetic only).

This is asserted via the `RenderPolicyChannel::all()` matrix established by W6-E.

### 4.7 Intelligence Loop Check (5-question)

This is the gate that originally rejected DOS-75 display-only:

1. **Claim model:** YES. Linear-issue-state becomes a claim with source attribution (Linear), `source_asof`, subject_ref (account/project), and trust band reflecting source freshness + state transition signal strength.
2. **Provenance + trust:** YES. Each rendered issue carries the ADR-0105 envelope; trust factor reflects Linear-issue-state-change as a Bayesian source weight contribution per ADR-0114.
3. **Signals + invalidation:** YES. State changes emit signals (§4.3) that invalidate dependent claims (health scores, prep callouts).
4. **Runtime + surfaces:** YES. Account detail + project detail consume the chapter; `prepare_meeting` consumes the callout; MCP respects sensitivity policy.
5. **Feedback loop:** PARTIAL. Phase 3 (interactive updates) is explicitly out of scope; user feedback through DailyOS into Linear is deferred. User feedback path through `MarkOutdated` on a Linear-sourced claim still applies (W6-B substrate).

Phase 2 satisfies questions 1-4 and partial 5. Phase 3 would satisfy 5 fully; DOS-285 §1.3 says Phase 3 is out of scope unless a strong user need emerges.

## 5. Acceptance Criteria

DOS-285 §1.4 decision points become acceptance criteria:

1. **Scope locked:** account detail page renders Linear chapter. Project detail page renders Linear chapter. Both surface the same data with subject-appropriate filtering.
2. **No dedup with pushed actions:** Linear-issue chapter and pushed-action rows coexist on the same entity page without merging. The chapter is a separate visual section.
3. **Signal emission live:** the 5 signal types in §4.3 are emitted by `signals/linear_issues.rs` on the corresponding state transitions.
4. **Health scoring contribution:** blocked-issue count + stalled-progress duration + resolution rate feed the existing health factor library; the exact weighting follows the trust/scoring unification path.
5. **Briefing callout trigger:** `prepare_meeting` surfaces Linear-issue callouts under the three conditions in §4.5.
6. **ADR-0108 sensitivity sweep:** internal-only project issues do not leak across the 9 channels per W6-E's RenderPolicyChannel matrix. This is asserted via a substrate test mirroring bundle-17's shape, scoped to Linear-issue source class.
7. **Provenance envelope:** each rendered issue carries an ADR-0105 provenance envelope with `source_class: "linear"`, `source_asof: <linear-update-time>`, `subject_ref: <account|project|person>`, trust band reflecting freshness.
8. **No raw pipeline vocabulary in copy:** user-facing strings use product vocabulary (per ADR-0083, see src/CLAUDE.md), not raw "AI enrichment" or "intelligence pipeline" terms.
9. **Frontend CSS modules + design tokens:** the chapter component uses CSS modules per `src/CLAUDE.md` editorial conventions; no inline styles; uses existing design tokens.
10. **No PII in fixtures:** test fixtures use generic synthetic identifiers.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-285 supplied verbatim in §2 + §5.
- **Upstream:** DOS-75 (parent; closed, this is the carry-forward). Linear backend support already present.
- **Adjacent:** W7-E (DOS-260 telemetry) is independent. W7-B/C/D (release_gate.rs / build.rs) are independent.
- **Cross-wave:** W6-E (bundle 17 sensitivity sweep) provides the `RenderPolicyChannel::all()` enum that this packet's §4.6 consumes. Already merged via PR #290.

## 7. L0 Reviewer Panel

- **Required reviewers:** `qa-expert` + `plan-design-review` (UI).
- **Panel reason:** W7-A is the only W7 agent with a substantial user-facing UI surface; design-reviewer ensures the chapter component fits the magazine layout + editorial conventions per `src/CLAUDE.md`.
- **Security reviewer:** the wave plan doesn't require it, but the ADR-0108 sensitivity sweep on internal-only projects is a trust-boundary concern. `qa-expert` review on §4.6 + §5.6 should suffice; if reviewer flags the trust-boundary surface as needing deeper scrutiny, escalate to security-auditor.

## 8. L0 Acceptance Gate

L0 passes only if reviewers accept all of the following:

1. **Intelligence Loop 5-question check:** passes for questions 1-4; question 5 is partial per Phase-3-deferred and is documented as such.
2. **Scope locked:** account + project detail both render the chapter.
3. **Signal types enumerated:** 5 named state-change signals.
4. **Health scoring path:** integrates with existing factor library, not a parallel system.
5. **Callout conditions:** named and bounded (3 conditions in §4.5).
6. **Sensitivity sweep:** asserted via the W6-E matrix, not a parallel surface check.
7. **Provenance envelope:** ADR-0105 shape on every rendered issue.
8. **Frontend conventions:** CSS modules + design tokens + product vocabulary.
9. **No Phase 3:** interactive update is explicitly deferred.

## 9. Out-Of-Scope

- Phase 3 (interactive updates from DailyOS into Linear).
- Dedup with pushed actions (kept separate per DOS-75 recommendation).
- Adding new Linear issue types not already supported by `inject_linear_issues`.
- Bayesian source-weight tuning (lives in the scoring unification path, not W7-A).
- A new MCP tool for Linear issues (the existing MCP bridge surface is what gets sensitivity-filtered).
- DOS-260 telemetry on Linear-issue rendering (that's W7-E if relevant at all).

## 10. Changelog

- **V1 - 2026-05-15:** Initial W7-A L0 packet. Mapped DOS-75 Phase 2 to v1.4.1; locked Intelligence Loop 5-question check; defined signal types + health scoring integration + callout conditions + ADR-0108 sensitivity sweep; named qa-expert + design-reviewer for L0 panel.
