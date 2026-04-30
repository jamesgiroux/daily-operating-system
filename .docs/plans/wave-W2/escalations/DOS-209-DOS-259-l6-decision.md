# L6 escalation: W2-A + W2-B plans hit cycle-2 cap; both surface a shared seam-ownership question + DOS-209 catalogue gap

## Context

- **Wave / agents:** W2-A (DOS-209 — `ServiceContext` + `ExecutionMode`) and W2-B (DOS-259 — `IntelligenceProvider` trait + PTY/Glean migration). Both blocked at L0 cycle 2.
- **Layer where escalation triggered:** L0 plan-review, cycle 2 of 2.
- **Trigger types:** (a) `revision-cycle` — both plans REVISE on cycle 2 means L6 per the 2-revision-cycle hard cap; (b) `needs-human-judgment` — DOS-209 cycle-2 challenge explicitly returned ESCALATE-TO-L6; (c) `severity-disagreement` — DOS-209 architect APPROVE vs DOS-209 challenge ESCALATE on the same v2 plan.

## Cycle-2 verdict matrix

| Plan | architect-reviewer | /codex consult | /codex challenge |
|---|---|---|---|
| **DOS-209 v2** | APPROVE | APPROVE | ESCALATE-TO-L6 |
| **DOS-259 v2** | APPROVE | REVISE | STALLED (runtime fail, both cycles) |

## The disagreements

### Disagreement 1 — DOS-209 mutation catalogue exhaustiveness

- **architect-reviewer (APPROVE):** Catalogue + ~210-row table is "materially stronger than v1," meets architectural soundness.
- **codex challenge (ESCALATE):** Catalogue claims `rg`-derived exhaustiveness but misses 5+ live mutators. Specifics: `emails::unarchive_email:1124`, `emails::unsuppress_email:1181`, `emails::pin_email:1186`, `accounts::snooze_triage_item:1941`, `entity_linking/rules/p2_thread_inheritance.rs:19`. These are exactly the kinds of side effects (DB write + external + signal + queue) DOS-209 is supposed to gate.

**Independent verifiability:** Trivially. Run `rg -n 'fn (archive|unarchive|pin|unpin|suppress|unsuppress|snooze|enqueue)_' src-tauri/src/services src-tauri/src/db src-tauri/src/entity_linking` and check whether each result appears in the v2 plan's mutation table. Challenge's claim survives this check; architect's "materially stronger" claim does not survive at the "exhaustive" bar the plan claims.

### Disagreement 2 — DOS-209 W2-B-first sequencing

- **codex challenge (ESCALATE):** Plan v2 §7 says "W2-B opens/lands first." DOS-209 Linear ticket Dependencies block says: *"Landing order: this issue first; then IntelligenceProvider trait extraction (separate issue, in parallel); then everything else."* Plan author flipped the frozen contract per wave-plan hint, which is not authority to override the Linear ticket.
- **architect-reviewer (APPROVE) — implicitly:** Did not flag the contract conflict. Likely because the wave-plan hint argument (W2-B-first reduces W2-A's mutation surface) is architecturally sound.

**The actual contract conflict:** the wave plan (`v1.4.0-waves.md`) and the DOS-209 Linear ticket disagree on landing order. One of them must be amended.

### Disagreement 3 — DOS-259 provider seam ownership

- **architect-reviewer (APPROVE):** Trait shape frozen, routing factory named, mode-boundary integrity structural.
- **codex consult (REVISE, fresh F2):** Plan v2 §3 freezes `select_provider(ctx: &ServiceContext, tier)`. DOS-259 ticket text says *"Transform abilities take `&dyn IntelligenceProvider` via `AbilityContext`."* ADR-0104 splits: `ServiceContext` carries mode + clients; provider injected via `AbilityContext`. Cycle-1 F3 fix landed the routing on the wrong context.

**Independent verifiability:** Read DOS-259 ticket §"Architectural surfaces touched" — explicit statement "Transform abilities take `&dyn IntelligenceProvider` via `AbilityContext`." Plan v2 contradicts this surface.

### Tiebreaker note

I did not run an independent fourth-reviewer tiebreaker pass because the disagreements are about plan-vs-frozen-contract conflicts, not reviewer judgment. A tiebreaker on a different model would re-find the same conflicts.

## What's at stake

### Architecture

- **Mutation catalogue:** DOS-209's load-bearing invariant is "every mutator gets `check_mutation_allowed()?`." A non-exhaustive catalogue means W2-A ships with **ungated mutators** — the exact failure mode DOS-209 exists to prevent. This is the kind of substrate gap the W3 claims layer + W4 Trust Compiler + W5 pilots will compound on top of.
- **Provider seam:** Putting `select_provider(ctx: &ServiceContext, ...)` on the wrong context means W3-B (Provenance), W4-B (eval harness), and post-spine DOS-213 (prompt fingerprinting) consumers will need to invent a bridge or the plan will need a v3 architectural pass after v2 is "frozen."
- **Landing order:** Whichever issue lands first determines the merge-conflict surface on `services/intelligence.rs`. W2-A first means W2-B rebases past 60+ `check_mutation_allowed()?` insertions, some of which are inside functions W2-B is restructuring or deleting. W2-B first means W2-A's sweep is on a smaller surface (PTY orchestration block already gone).

### Schedule

- **Wave 2 target wall-clock:** 1–2 days per the wave plan. Currently we've spent ~2 hours on plan + L0 cycles; the underlying work has not started.
- **W3 dependency:** W3-A (registry), W3-B (claims), W3-C (DOS-7) all consume W2-A's `ServiceContext` and W2-B's `IntelligenceProvider`. Every day W2 doesn't freeze pushes W3 right.
- **2026-05-12 release gate:** v1.4.0 spine target is 14 days out. Cycle 3 with tight scope is cheap; a true rewrite would cost 3+ days.

### Cost of being wrong

- **Catalogue gap:** one-way door if it ships. Ungated mutators in W2-A means W3+W4+W5 will compose on a structurally-broken substrate. Recovery requires re-auditing services/ post-hoc, which is exactly the v1.4.0 thesis's failure mode (catching invariant violations weeks later instead of structurally).
- **Provider seam:** moderate one-way door. If `select_provider` ships on `ServiceContext`, downstream consumers can route around it via additional plumbing, but the architecturally-correct seam (`AbilityContext`) becomes the path of last resort, not first. ADR-0104 already specified the right answer; this is a "re-read the ADR and align" fix, not a research project.
- **Landing order:** zero one-way-door cost if amended now (the ticket's Dependencies block can be updated with one edit). If unamended and the plan ships W2-B-first, future readers will trip on the contradiction every time they consult the ticket.

## Recommended options

### Option A — Tightly-scoped cycle 3 (recommended)

Authorize a single, narrowly-scoped cycle-3 revision pass on each plan with the following hard constraints:

1. **DOS-209 plan v3 changes ONLY:**
   - Re-audit the mutation catalogue using a programmatic script (commit it to the repo as `scripts/dos209-mutation-audit.sh` so future agents can re-run). Catalogue must include every match. Plan v3 commits the script, the catalogue output, and a test that re-runs the audit and asserts the catalogue file matches.
   - Restore full `cargo clippy && cargo test && pnpm tsc --noEmit` to the §9 mandatory CI command. Targeted `dos209` tests stay as additional evidence, not replacement.
   - Resolve landing-order conflict per Option-1A, 1B, or 1C below.
2. **DOS-259 plan v3 changes ONLY:**
   - Move `select_provider` signature to take `&AbilityContext` (or whatever the equivalent W3-A registry-derived context is). Acknowledge that for early callers (intel_queue, services/intelligence.rs) that don't yet have an `AbilityContext`, the routing happens via an `AppState`-owned provider Arc per ADR-0091, and the factory only runs in ability-execution contexts.
   - State explicitly that `ServiceContext.execution_mode` is the only thing the factory reads from `ServiceContext`; everything else is `AbilityContext`-owned.
3. **L0 cycle 3 = same triangle, single pass.** No cycle 4. If cycle 3 doesn't unanimously APPROVE, the work pauses until a deeper L6 architecture review.

Sub-decisions for the landing-order conflict:

- **1A: Amend DOS-209 ticket.** Update the Linear Dependencies block: "Landing order amended 2026-04-28 per wave-plan hint and architectural review: W2-B (DOS-259, IntelligenceProvider trait extraction) opens its PR first to reduce the mutation surface; W2-A rebases on top." This is the cleanest fix; aligns ticket with wave plan + plan v2.
- **1B: Restore W2-A first.** Revise both plans to land DOS-209 first, with W2-A handling the full mutation surface including the PTY orchestration block, then W2-B extracts. Honours the original ticket; loses the architecturally-cleaner merge.
- **1C: L6 abstains; document the conflict in v3 §10 and let cycle-3 reviewers pick.** Risk: kicks the can; cycle 3 might re-escalate.

**Founder/architect recommendation:** **1A.** The wave plan was written with merge-conflict surface analysis; the ticket's landing-order line predates that analysis. Amending the ticket aligns truth.

### Option B — Reset to plan v1, bring in domain experts for a fresh L0 (slow but thorough)

Discard plan v2, rebuild plan v1 from scratch with explicit pre-L0 architectural alignment from a domain expert pass on ADR-0104 ServiceContext / AbilityContext split. Adds 2–3 days. Justified only if Option A's tightly-scoped cycle 3 fails or if the architectural questions are deeper than the cycle-2 reviewers caught.

### Option C — Accept v2 as-is, file fixes as v1.4.1 follow-ups

Ship plan v2 as-frozen. File three follow-up issues for v1.4.1: (1) Re-audit DOS-209 mutation catalogue post-W2 ship; (2) Re-route DOS-259 provider seam to `AbilityContext`; (3) Reconcile DOS-209 landing-order with wave plan. **Not recommended** — the catalogue gap is one-way-door; ungated mutators ship and W3+W4+W5 compose on broken substrate.

### Option D — Defer W2 entirely; collapse W2 + W3 into a unified wave (long lever)

Re-plan v1.4.0 to defer the substrate split between W2 and W3. Argument against: this is the fan-out wave plan's whole thesis; merging W2 + W3 puts 10 agents on intersecting files. **Not recommended.**

## Specific decisions L6 must make

To unblock cycle 3 (Option A), L6 must answer:

1. **Landing order:** 1A (amend ticket) / 1B (restore W2-A first) / 1C (defer to cycle-3 reviewers).
2. **Provider seam:** confirm `select_provider` moves to `&AbilityContext` per ADR-0104 / DOS-259 ticket text. Y/N.
3. **DOS-209 catalogue:** confirm the script-based audit approach. Y/N.
4. **CI command:** confirm full `cargo clippy && cargo test && pnpm tsc --noEmit` requirement is restored. Y/N.
5. **Cycle-3 authorization:** confirm a single tightly-scoped revision pass is allowed despite the 2-cycle cap. Y/N. (The 2-cycle cap is "no third unilateral revision"; L6-authorized revision is permitted.)

## Process notes for the W1 retro

1. **codex:rescue runtime fragility.** `/codex challenge` mode for DOS-259 stalled in both cycle 1 and cycle 2. Wrapper forwards to a Codex CLI task, CLI runs substantive work, wrapper exits before file write, codex companion returns "No job found." Reliable failure pattern. Worth either fixing the runtime contract (poll until file appears) or substituting `code-reviewer` subagent for the challenge slot when this pattern triggers.
2. **Cycle-2 over-correction.** Both plans' cycle-1 revisions over-corrected on the wave-plan hint and ended up violating the frozen Linear ticket. This suggests the cycle-1 prompt should require explicit conflict-mapping (where does the ticket disagree with the wave plan?) before resolving. Tightening the cycle-1 revision prompt is in scope for the W1 retro tuning protocol.
3. **Independent reviewer signal.** The cycle-2 reviewers did exactly the job they were designed for. The architect-reviewer caught architectural soundness; the consult caught the wrong-context provider routing; the challenge caught the catalogue gap and the landing-order conflict. The disagreement isn't reviewer noise — it's three independent lenses revealing real issues the v2 author missed. Strong signal.

## Evidence packet

- **Original Linear tickets:** [DOS-209](https://linear.app/a8c/issue/DOS-209) | [DOS-259](https://linear.app/a8c/issue/DOS-259)
- **Plan v2 files:**
  - `.docs/plans/wave-W2/DOS-209-plan.md` (2,648 words)
  - `.docs/plans/wave-W2/DOS-259-plan.md` (1,398 words)
- **Cycle-2 review files:**
  - `.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md` (APPROVE)
  - `.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md` (APPROVE)
  - `.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md` (ESCALATE)
  - `.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer-v2.md` (APPROVE)
  - `.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v2.md` (REVISE)
  - DOS-259 challenge v2 — STALLED, no file
- **Cycle-1 review files:** `.docs/plans/wave-W2/reviews/*-l0-*.md` (no -v2 suffix)
- **ADRs cited:** 0104 (ExecutionMode + ServiceContext), 0091 (IntelligenceProvider), 0106 (Prompt Fingerprinting + Provider Interface)
- **Wave protocol:** `.docs/plans/v1.4.0-waves.md` §"Wave 2" lines 428–456; §"Review system" lines 69–365
