# The Engineering Ladder (L0–L6)

**Status:** Canonical reference (replaces "Review Ladder" naming)
**Adopted:** 2026-05-18
**Reviewer-composition revision:** 2026-05-23 (compose installed `ce-*` + `pr-review-toolkit:*` agents instead of always-on custom panels; default sizes reduced)
**See also:** `.docs/plans/v1.4.0-waves.md` § Engineering Ladder, `CLAUDE.md` § The Engineering Ladder, `docs/solutions/README.md`

The Engineering Ladder defines how work moves from intake to merge across L0–L6, with each rung naming the **Plan**, **Implement**, **Review**, and **Capture** skills that apply at that phase. Numbering is preserved (L0–L6) — load-bearing in the `commit-msg` hook, agent prompts, ADRs, Linear comments, and memory entries.

Used in chat: "L2 looks good." Used in docs/headings: "L2 (Diff) review verdict: approve."

## Reviewer-composition principle

Default reviewer counts are deliberately small. The cost of a 4-reviewer L0 × 5–6 convergence cycles is the cost that drove this revision (W1 v1.4.5 burned ~27 review cycles across 3 lanes). The library of installed agents — `ce-*` and `pr-review-toolkit:*` — is large; the ladder **composes** specific agents per concern rather than running every reviewer on every change.

Two design rules:

1. **One default reviewer + router-selected specialists.** Don't run `ce-maintainability-reviewer` AND `ce-correctness-reviewer` AND `ce-code-simplicity-reviewer` on every PR; the router (below) picks 0–2 based on what the diff actually touches.
2. **Advisory ≠ blocking.** Code-quality concerns (maintainability, simplicity, pattern duplication) run in parallel as **advisory** — findings go to the maintenance project, not back to the cycle-N+1 reviewer loop.

## Skill matrix

| Rung | Phase | Plan | Implement | Review | Capture |
|---|---|---|---|---|---|
| **L0 Prep** | Plan hardening | `/plan-eng-review` (always) + `/plan-ceo-review` (product/domain) + `/plan-design-review` (UI/workflow) + `/plan-devex-review` (MCP/API/DX) + `/ce-plan` (structure + open-question surfacing) + `/ce-strategy` (only when no STRATEGY exists) | n/a | n/a | `/ce-sessions` — pull prior-session findings into the plan |
| **L0 Plan** | Plan review (gate) | n/a | n/a | **Default (2):** `/codex challenge` + ONE planning reviewer from the matrix below. **K-in (mandatory):** `ce-learnings-researcher` runs in parallel — not a panel slot, a consumed signal. **Conditional (+1):** add second planning reviewer when scope tier is Wave or Amendment 3 paths are touched. | **K-in capture**: `ce-learnings-researcher` cites `docs/solutions/` + `.docs/decisions/` hits in verdict. Reinvented substrate = **BLOCKED**. |
| **L1 Implementation** | Implementation + self-validation | n/a | `/ce-work` (executes against plan) + `/ce-debug` (root-cause-first when stuck) + `/ce-simplify-code` (pre-PR polish) + manual editing | self-validation: tests pass, proof artifacts captured, demo evidence captured before opening the PR | `/ce-sessions` when picking up mid-task across sessions |
| **L2 Diff** | PR review (gate) | n/a | n/a | **Default (1):** `/codex review` orchestrated via the `l2-bounded-reviewer` agent (AC-scoped, path-α routing). **Router-selected (0–2):** see "L2 reviewer router" below. **Advisory parallel (always-on, non-blocking):** `ce-maintainability-reviewer` + `ce-code-simplicity-reviewer` file findings to the maintenance project. `/ce-resolve-pr-feedback` for resolving review threads after L2 verdict. | **K-in**: router reviewer cites matching `docs/solutions/` entries when finding repeats a known class. |
| **L3 Wave** | Integrated review (gate) | n/a | n/a | `/codex challenge` against integrated wave + ADRs + `ce-architecture-strategist` on integrated state + Suites S/P/E (gate-by-need — see "Suite firing rules"). For non-major versions (v1.x.y), L5 drift folds into L3 — `ce-architecture-strategist` does both passes. | **K-out (mandatory, Tier 1 autonomous)**: at retro close, Claude runs `/ce-compound mode:headless` for each class-pattern + substrate-already-existed finding. Output committed to `docs/solutions/<category>/`. |
| **L4 Surface** | User-facing QA | n/a | n/a | `/qa-only` first, `/qa` if remediation needed, `accessibility-tester` for user-facing, `/ce-test-browser` if no `/qa` config | **K-out**: capture surface bugs that recur across waves via `/ce-compound` |
| **L5 Drift** | Architecture drift (major versions only) | n/a | n/a | For v1.x.0 (major) only: `/plan-eng-review` + `ce-architecture-strategist` comparing integrated state to planned end-state. **For v1.x.y (minor/patch), L5 folds into L3** — separate rung does not fire. | **K-out**: `/ce-compound-refresh` on stale `.docs/decisions/` and `docs/solutions/` entries the drift sweep surfaces |
| **L6 Human** | Escalation | n/a | n/a | James — decision posted as Linear comment on affected ticket | **K-out**: L6 decisions captured as Linear comments (existing) + ADR if decision is architecturally durable |

## L0 reviewer router

Default L0 panel is **`/codex challenge` + ONE planning reviewer** chosen by what the plan touches. Add a second reviewer only when scope tier is Wave or Amendment 3 paths are touched.

| Plan touches | Planning reviewer |
|---|---|
| Internal consistency, terminology drift, ambiguity | `ce-coherence-reviewer` |
| Scope creep, premature abstraction, unjustified complexity | `ce-scope-guardian-reviewer` |
| Architectural feasibility, dependency gaps, migration risk | `ce-feasibility-reviewer` |
| Product strategy, premise claims, trajectory | `ce-product-lens-reviewer` |
| UI/workflow, interaction states, design slop risk | `ce-design-lens-reviewer` |
| Security gaps at plan level (auth, data exposure, API surface) | `ce-security-lens-reviewer` |

`ce-learnings-researcher` runs in **parallel** on every L0 (mandatory K-in) — not a panel slot, a consumed signal in the verdict.

## L2 reviewer router

Default L2 panel is **`/codex review` via `l2-bounded-reviewer` (1 reviewer)**. Router adds 0–2 specialists based on what the diff actually touches:

| Diff touches | Reviewer to add |
|---|---|
| New types, structs, enums, type signatures | `pr-review-toolkit:type-design-analyzer` |
| Error handling, fallbacks, catch blocks, suppress paths | `pr-review-toolkit:silent-failure-hunter` |
| Hot paths, queries, loops over collections, I/O | `ce-performance-reviewer` |
| Database migrations, schema changes, backfills | `ce-data-migrations-reviewer` |
| API routes, request/response types, serialization, versioning | `ce-api-contract-reviewer` |
| Auth middleware, public endpoints, user input, permission checks | `ce-security-reviewer` |
| Error handling at production scale (retries, circuit breakers, timeouts) | `ce-reliability-reviewer` |
| TypeScript code (frontend) | `pr-review-toolkit:type-design-analyzer` + (when high-risk) `ce-kieran-typescript-reviewer` |
| Migration safety + data integrity together | `ce-data-integrity-guardian` |

**Advisory parallel (always-on, non-blocking):**

| Concern | Agent |
|---|---|
| DRY / duplication / anti-patterns | `ce-pattern-recognition-specialist` |
| Premature abstraction, dead code, coupling | `ce-maintainability-reviewer` |
| YAGNI / simplification | `ce-code-simplicity-reviewer` |
| CLAUDE.md / AGENTS.md compliance | `ce-project-standards-reviewer` |
| Test coverage gaps | `ce-testing-reviewer` |
| Comment rot, stale docstrings | `pr-review-toolkit:comment-analyzer` |
| Correctness sanity check | `ce-correctness-reviewer` |

Advisory reviewers file findings to the **Codebase Maintenance & Production Quality** Linear project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`), not back to the L2 cycle. They DO NOT block merge.

**Why advisory-not-blocking:** the old `code-reviewer` slot blocked merges on theoretical findings, which is the cycle-N+1 problem path-α was supposed to solve. Moving these to advisory closes the loop — quality concerns are tracked, not skipped, but they don't bottleneck the substrate PR.

## L3 reviewer composition

| Concern | Agent |
|---|---|
| Integrated diff vs intended wave outcome | `/codex challenge` |
| Architectural fit on integrated state | `ce-architecture-strategist` |
| Suite S (security) when wave AC names security | `ce-security-reviewer` + `ce-security-sentinel` |
| Suite P (performance) when wave AC names perf | `ce-performance-oracle` |
| Suite E (edge cases) — always | `ce-testing-reviewer` + harness |
| Schema drift across wave (always for waves with migrations) | `ce-schema-drift-detector` |
| Deployment safety for production-touching waves | `ce-deployment-verification-agent` |

## Suite firing rules

- **Suite S (security)** runs at L3 when wave AC includes any auth/permission/data-exposure claim, OR Amendment 3 paths were touched in any wave PR.
- **Suite P (performance)** runs at L3 when wave AC includes a perf budget OR touches migrations / hot paths / projections.
- **Suite E (edge cases)** runs at L3 on every wave (cheap; catches the long tail).

This replaces "all three always at L3" — most waves only need Suite E plus one other.

## Pass rules

- **L0 Plan**: unanimous approval from the panel sized to the scope tier
- **L2 Diff**: `/codex review` (via `l2-bounded-reviewer`) approves AC compliance + all router-selected reviewers approve. Advisory parallel reviewers DO NOT block.
- **L3 Wave**: `/codex challenge` + `ce-architecture-strategist` approve; firing suites green; schema drift = none.
- **L4 Surface**: zero blockers
- **L5 Drift** (major versions only): no drift
- **L6 Human**: James's call

**Pacing rule.** No wave starts until prior wave clears L3 *and* L5 (where applicable). No agent codes before L0 clears unanimously. 2 revision cycles on the same plan or PR without convergence ⇒ L6 escalation.

**Symptom-progress gate (debug-driven work).** If a user-visible symptom hasn't moved after a PR landing that claimed to address it, no new packet in the same surface area until a fresh trace lands. This prevents stacking adjacent substrate work on a misdiagnosed root cause. Memory `feedback_parallel_codex_for_diagnosis_when_vortexing`.

**L4 before L2 for user-facing fixes.** Any work touching a user-visible surface (block render, chip content, UI affordance, page chrome) runs L4 hands-on validation BEFORE L2 reviewers dispatch. The L2 dispatch with no L4 evidence is structurally invalid — chips passing tests but rendering empty in browser is the failure mode this prevents. Memory `feedback_l4_before_l2_for_user_facing`.

**Bounding.** L2 reviews are bounded by acceptance criteria, enforced by the `l2-bounded-reviewer` orchestrator agent (memory `feedback_l2_must_review_against_acceptance_criteria`).

**Path-α** — what it is, what it isn't.

The path-α maintenance channel exists for one purpose: prevent L2-cycle token loss on findings that surface during review but aren't part of the PR's acceptance contract. A finding qualifies as path-α when **all** of these are true:

1. Surfaced by an L2 reviewer during a review cycle (codex, router-selected specialist, advisory parallel).
2. **Not** a literal acceptance-criterion violation from the ticket.
3. **Not** an ADR-named contract violation introduced by this PR.
4. **Not** a regression in PR-touched code (touching code makes its existing behavior fair game).
5. Theoretical hardening, dormant edge case, latent risk, or generic codebase improvement.

A qualifying finding files to the Codebase Maintenance & Production Quality project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) with priority reflecting impact + likelihood, and the substrate PR unblocks. The mechanism is a stop-loss on cycle-N+1 review attention, not a parking lot.

**Advisory parallel reviewers feed directly into path-α** — every finding from `ce-maintainability-reviewer` / `ce-pattern-recognition-specialist` / `ce-code-simplicity-reviewer` / `ce-project-standards-reviewer` / `ce-testing-reviewer` is path-α by construction. The `l2-bounded-reviewer` orchestrator drafts the Linear tickets; James/orchestrator confirms before file.

**What path-α is NOT for:**

- **Original-spec scope.** If a deliverable is named in the ticket, wave plan, or L0 packet, finishing it is part of L1, not a path-α candidate. "We didn't get to X" → reopen the cycle or expand the wave, never park as maintenance.
- **Scope reduction that isn't acknowledged.** Cutting scope is a deliberate decision posted to Linear as a scope amendment, not a quiet "file as path-α and move on."
- **Punting hard problems.** A finding that's hard to fix is still in-scope if it meets the criteria above. Path-α status reflects relevance to *this PR's acceptance contract*, not difficulty.
- **Tracking debt accumulation.** Memory `feedback_no_deferrals_period` applies: once scope is agreed, finish it. Path-α exists to keep the L2 loop bounded, not to enable structural deferrals.

When in doubt: if removing the finding from this PR would change whether the PR delivers what was agreed, it isn't path-α.

**Threat-topology framing (L0 reviewer scoping).** Every L0 packet declares its trust topology in §1 header: `local-to-local single-user | local-to-local multi-user | remote-to-local | remote-to-remote`. `ce-security-lens-reviewer`, `ce-security-reviewer`, and `/codex challenge` scope their threat model to that topology — they do **not** enforce multi-actor gates on single-actor surfaces. DailyOS's WP block → loopback Tauri runtime is local-to-local single-user; most multi-actor gates (confirmation tokens, principal differentiation, cross-actor poisoning, scope-gated redaction of data the user already has filesystem access to) collapse to non-issues. What still applies regardless of topology: compile bugs, crate-boundary rules, slug/path validation for data hygiene, indirect prompt injection from untrusted document content (ADR-0093), and sensitivity redaction in logs/screenshots (ADR-0108). Reviewers who flag multi-actor gates on a single-actor surface waste a cycle; reviewer prompts must cite the topology and constrain accordingly. See memory entry on local-to-local security overreach for the full pattern.

## Origination check (debug-driven vs greenfield vs extension)

Every L0 packet declares an origination class in §0 before §1 header:

- **Greenfield** — net-new substrate, no prior surface
- **Extension** — extends or refines existing substrate
- **Debug-driven** — work originates from a user-visible failure or stuck symptom

**For debug-driven origination, §0 MUST include a symptom-to-failure trace.** The trace structure:

1. **User-visible symptom** verbatim (the words the user said, not paraphrased)
2. **Call path** from surface entry to substrate, with file:line at each hop
3. **Suspected failure point** with file:line, found via instrumentation OR direct trace OR clean-context codex dispatch (NOT inferred from architectural intuition)
4. **Hypotheses already explored and rejected** (so reviewers don't re-walk them)

**The packet's implementation §3 must intersect the trace.** If §3 reshapes substrate that the trace doesn't implicate, reviewers cite the mismatch and the packet rejects with a **`wrong cure`** verdict (new disposition, alongside APPROVE / REQUEST_CHANGES / BLOCK). `wrong cure` is harsher than REQUEST_CHANGES because it means the packet's premise is broken — patching the implementation won't fix it; the packet must restart from a corrected trace.

Trace can come from instrumentation, codex dispatch with a clean brief, or live debugging — but it must be evidence, not intuition. Memory `feedback_parallel_codex_for_diagnosis_when_vortexing`: **2h+ on the same user-visible symptom without resolution = dispatch codex with "trace, don't patch" brief BEFORE drafting any packet.**

## Scope-based ladder sizing

Not every packet warrants the full reviewer panel. Tier by scope:

| Scope | What it looks like | L0 panel | L2 panel |
|---|---|---|---|
| **Trivial** | Single-file fix <50 LOC, no contract change, no user-facing impact (typo, comment, lint-fix, test-only) | Skip L0 — code straight | `/codex review` only (no router additions) |
| **Small** | Single substrate change OR single UI fix, no architectural reshape, single domain | `/codex challenge` (1 reviewer) | `/codex review` + 0–1 router reviewer |
| **Standard** | Multi-file scope, single domain, no cross-cutting contract changes | `/codex challenge` + 1 planning reviewer (2 reviewers) | `/codex review` + 1–2 router reviewers |
| **Wave** | Architecture, substrate reshape, multiple domains, ADR-touching | `/codex challenge` + 2 planning reviewers (3 reviewers) + `ce-learnings-researcher` parallel | `/codex review` + 1–2 router reviewers + Suite S/P/E per firing rules at L3 |

**Scope tier declared in §0** alongside origination class. Reviewers can escalate the tier if scope is mis-declared (e.g., a "small" fix that touches an ADR-named contract → reviewer requests tier-up to standard before reviewing). Reviewers can also de-escalate if a wave-tier packet is actually doing standard-tier work.

Memory `feedback_review_loop_diminishing_returns_means_scope_is_wrong` applies: if a tier-N L0 produces 5+ findings per cycle, the tier is likely wrong; reset to tier-N+1 rather than absorbing findings cycle by cycle. Tier mis-declaration is a packet-author error, not a reviewer-thoroughness error.

## The Knowledge Channel (K)

`K` is a continuous feedback channel parallel to L0–L6, not a rung.

```
                     Knowledge channel (K)
        ┌──────────────────────────────────────────┐
        │  docs/solutions/    .docs/decisions/     │
        │  (CE writes)        (ADRs, manual)       │
        └──────────────────────────────────────────┘
              ▲ (K-out)              │ (K-in)
              │                       ▼
   L3 retro ──┘              ┌─── L0 reviewer must grep
   L5 drift ──┘              │    before approving
   L4 surface bugs ──┘       └─── L1 author may grep
                                  before drafting
```

### K-in (consume) — substrate-grep + diagnostic-grep obligations

**At L0 (mandatory).** `ce-learnings-researcher` runs in parallel with the planning reviewer and surfaces both obligations:

> 1. **Substrate-grep**: grep `docs/solutions/` and `.docs/decisions/` for substrate this plan claims to be net new. Cite any hits. Reinvented documented substrate = **BLOCKED**, cite the path.
> 2. **Diagnostic-grep** (debug-driven packets): grep `docs/solutions/workflow-issues/` for the symptom pattern named in §0's trace. If a prior diagnostic entry exists, cite it. Ignored prior diagnostic pattern = **REQUEST_CHANGES** with the workflow-issue path cited; if the §0 trace contradicts a prior diagnostic, flag as **`wrong cure`** verdict.

**Symptom-fit prompt (debug-driven packets, mandatory for at least one reviewer).** L0 reviewer prompts include:

> Does this packet's implementation §3 directly move the user-visible symptom named in §0? Cite the specific change in §3 that intersects the failure point in §0's trace. If you can't cite a direct intersection, the packet is **`wrong cure`** — return that verdict, not REQUEST_CHANGES.

**At L1 (advisory).** Implementing agents should grep these directories for the entity / module / pattern being touched before authoring. Memory `feedback_check_substrate_before_authoring_primitives` codifies this.

**At L2 (advisory).** Router-selected reviewers cite matching `docs/solutions/` entries when a finding repeats a documented class.

### K-out (capture) — Tier 1 autonomous only

K-out runs `/ce-compound mode:headless` to write findings to `docs/solutions/<category>/<slug>-<date>.md`.

**Tier 1 — Autonomous (default and only tier).** Claude runs `/ce-compound mode:headless` for each qualifying finding as part of the L3 retro work. Same set-and-forget authorization as the wave protocol covers for impl→L1→commit→L2→fix→retro→tag. The retro is already the parked context at wave-end; K-out is the closing step.

The retro template names a K-out captures section, populated by the autonomous run — visibility, not enforcement. (Previous Tier 2 Stop hook and Tier 3 template-gate enforcement removed 2026-05-23 as duplicate layers on already-defended work.)

### K-out cadence

**Default: every L3 retro.** At wave end, `/ce-compound` runs for:

- Class-pattern findings (memory `feedback_zoom_out_for_class_pattern_in_l2_loop`)
- Substrate-already-existed findings (memory `feedback_check_substrate_before_authoring_primitives`)
- Cross-wave drift findings surfaced by L5

**Also fires at:** L5 drift sweep close for major versions (via `/ce-compound-refresh`), L4 surface-bug recurrence (2+ waves), manual invocation by James or any agent.

## L6 escalation policy (carries forward from v1-lite.md §6)

**MUST escalate**: cycle cap exceeded, reviewer flags "needs human judgment" with a specific structural question, L5 drift without remediation path, suite gate failure, reviewer infrastructure failure, net-new scope expansion, contract amendment.

**MUST NOT escalate**: routine review iteration, single flaky codex output, lint/type/test failures, reviewer dissent (investigate, don't break ties), same finding class repeats (sweep instead), scope discovery within DoD, equivalent-implementation choice (pick more complete), "I don't know" without a specific question, deferral requests, half-finished implementations, asking for context already in codebase/memory/plan, tool choice when convention exists.

## How this doc relates to others

- **`CLAUDE.md`** keeps a ~5-line stub section pointing here for the matrix.
- **`.docs/plans/v1.4.0-waves.md`** keeps the wave protocol and reviewer matrix; references this doc for the skill assignments per rung.
- **`docs/solutions/README.md`** describes the knowledge store structure and K-in / K-out flow from the consumer side.
- **`.github/reviewer-prompts/`** holds custom DailyOS reviewer prompts (architect, security-auditor, code-reviewer, accessibility, performance, l3-architect, l3-codex-challenge). These remain in use; the routers above invoke them under their custom names where DailyOS-specific topology/origination context is required, and invoke ce-* / pr-review-toolkit:* agents directly otherwise.
- **`.docs/plans/orchestration/v1-lite.md`** describes the async orchestration design (cloud routines, claudebot DMs, daily digest) — **none of which has shipped**. Treat as design archive until parts ship. Local workflow (this doc) is the canonical operating model.
