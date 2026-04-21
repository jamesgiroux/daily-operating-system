# v1.4.0 Execution Readiness

**Date:** 2026-04-20
**Status:** Draft — decisions and artifact list pending founder approval
**Companion to:** [strategy doc](2026-04-20-v1.4.0-architectural-strategy.md), [persona reviews](2026-04-20-persona-reviews.md)
**Purpose:** Answer the question "what does it take to move from decisions-made to AI-sessions-executing-reliably." Ground every claim in code + doc + issue evidence. Name the decisions pending and the artifacts required.

---

## How we got here

Four independent reviews informed this document:

1. **Red-team pass** against the strategy doc itself — findings addressed in strategy R1.
2. **Senior engineer persona review** — 10 findings on day-1 shipping reality. S1, S2, S3, S6, S9, S10 flagged High urgency.
3. **Systems architect persona review** — 10 findings on structural shape. A2 (data plan), A6 (failure domains), A7 (observability contract) flagged High urgency.
4. **Three code-level reality checks** — architecture docs audit, Linear issue spec audit, code-reality-vs-ADR-assumptions check.

All four agree on one thing: the ADR cluster is good. What's missing is the **bridge between decision and execution** — the reference artifacts an AI session reaches for when implementing, the code-level preconditions the ADRs assume, and a few decisions that were deferred past the point they can stay deferred.

---

## Current state, honest

### ADRs: solid

- ADR-0100–0119: 20 decisions, all in writing. R1 revisions addressed codex adversarial findings + reference-pass reality checks.
- ADR-0118 harness framing is committed. Gap resolution table points each residual gap at its resolving ADR.
- ADR-0119 runtime evaluator closes Gap A. ADR-0110 §9 closes Gap B. ADR-0113 §5 + R1.3 closes Gap D.
- Cross-link audit passes.

### Linear issues: 21 of 24 are rubric-ready

From the issue spec-completeness audit:

- **GREEN (21):** DOS-5, DOS-6, DOS-7, DOS-10, DOS-209, DOS-210, DOS-211, DOS-212, DOS-213, DOS-214, DOS-215, DOS-216, DOS-217, DOS-218, DOS-219, DOS-220, DOS-221, DOS-222, DOS-235, DOS-236, DOS-237, DOS-238. All have Problem / Why now / Scope / Acceptance / Dependencies structure in sufficient depth. Could be labeled `spec:needs-review` as-is.
- **YELLOW (1):** DOS-234 (DbKeyProvider trait seam) — missing Build-ready checklist items; otherwise solid.
- **RED (1):** DOS-241 (enrichment refactor research spike) — research-shaped, needs Shape F (research spike) rubric applied explicitly.

Every issue depending on recently-revised ADRs carries enough provenance to stay compatible with R1 changes — no issue needs rewrite due to ADR drift.

### Architecture docs: fully pre-date v1.4.0

From the docs audit:

| Doc | Grade | Reality |
|---|---|---|
| DATA-MODEL.md | Outdated | Current to migration 108; zero references to v1.4.0 tables |
| MODULE-MAP.md | Outdated | Auto-gen from HEAD; no `abilities/`, no `scoring::factors`, no `DbKeyProvider` |
| DATA-FLOWS.md | Outdated | 6 flows from March; no propose/commit, no trust compiler, no publish flow |
| LIFECYCLES.md | Outdated | No claim lifecycle, no agent trust ledger |
| PIPELINES.md | Partial | ~60% still relevant; missing invalidation jobs, coalescing, rate limits |
| SELF-HEALING.md | Partial | ~75% still relevant; missing durable jobs, cycle detection, dead-letter |
| README.md | Partial | Index is current but no pointers to ADRs 0100-0119 |
| `ARCHITECTURE.md` | **Missing** | Only archived (pre-v0.16) version exists |

The docs are accurate to what's in the codebase today. They become actively misleading the moment v1.4.0 substrate lands.

### Code reality: two hard blockers, one velocity tax

From the code-reality-vs-ADR-assumptions check:

**Hard blockers** (missing prerequisites the ADRs assume exist):

- **No `ServiceContext` struct.** ADR-0104 (ExecutionMode + Mode-Aware Services) assumes it exists. It does not. This blocks every ADR that takes `&AbilityContext` / `&ServiceContext` — effectively all of v1.4.0.
- **No `IntelligenceProvider` trait.** ADR-0091 asserts it; ADR-0106 extends it. Neither exists in code today. Intelligence generation is scattered across `services/intelligence.rs` (orchestration), `intelligence/glean_provider.rs` (one impl), and inline PTY orchestration. This blocks [ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) and the prompt-fingerprinting discipline.

**Velocity tax** (things that work but will need refactoring):

- **35% of SQL mutations live outside `services/`.** 44 files contain raw INSERT/UPDATE/DELETE. Notable offenders: `signals/` (5 files), `intelligence/glean_provider.rs`, `self_healing/`, `workflow/`. ADR-0104's `check_mutation_allowed()` discipline requires consolidation first.
- **Signal bus is sync and blocks async runtimes.** 236 `.await` calls in codebase, but `emit_signal_and_propagate` synchronously calls the engine. ADR-0115's policy registry + durable jobs will need async-friendly variants.
- **Dimension prompts have no `prompt_template_id` / `prompt_template_version`.** ADR-0106 fingerprinting requires them.
- **Process-wide singletons** (`notification.rs`, `util.rs` path caching, Glean discovery cache) need to be wrapped in `ServiceContext` or equivalent for per-ability isolation under Evaluate mode.

**Aligned and ready:**

- Services layer is ~85% consolidated.
- Migration infrastructure is solid (108 migrations, framework scales).
- `consistency.rs` and `validation.rs` exist with clean API shape.
- `intelligence_feedback` table exists (DOS-8 lineage).
- Signal bus structure matches ADR-0115 assumptions (4 emit functions, string-typed, inline policy, 318 call sites).

---

## What needs updating in the ADRs

From the persona reviews plus code reality. Six amendments, two new ADRs. Sized in hours at AI velocity.

### Amendments

1. **[ADR-0102](../decisions/0102-abilities-as-runtime-contract.md) — warning vs error vs soft-degradation contract (S2).** The ability return contract is implicit. Define three paths: hard error, soft degradation with `warnings[]`, explicit `AbilityError`. No ability silently logs-and-succeeds.

2. **[ADR-0102](../decisions/0102-abilities-as-runtime-contract.md) — add `experimental = true` registry flag (A5).** Enables prototyping new abilities without full provenance + fixtures + category enforcement. One-cycle lifespan; promotion requires ADR compliance.

3. **[ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md) — enforce 64 KB provenance hard cap (S3).** New `AbilityError::ProvenanceTooLarge`. Forces summarization or shallower composition.

4. **[ADR-0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md) — pessimistic row-lock in `commit_claim` (S6).** Prevents concurrent writers both seeing "no existing claim" and both inserting. SQLite serializable isolation alone is not sufficient for actor-based routing.

5. **[ADR-0115](../decisions/0115-signal-granularity-audit.md) — `PropagateSync { await_completion: bool }` variant (A3).** Allows callers to wait on synchronous propagation completion when they need to render with fresh state (user correction → next briefing).

6. **[ADR-0104](../decisions/0104-execution-mode-and-mode-aware-services.md) — acknowledge code preconditions.** The ADR assumes `ServiceContext` exists; it doesn't. Add a Phase 0 section specifying the struct shape that lands first, with explicit DI pattern for injecting into existing call paths.

### New ADRs

7. **ADR-0120: Observability Contract.** Addresses A7, S1, S9. Foundational — every ability and service function emits structured log entry with `invocation_id`, `ability_name`, `ability_version`, `duration_ms`, `outcome`. `tracing` spans thread `invocation_id` through every log line. Single doc, consumed by [DOS-250](https://linear.app/a8c/issue/DOS-250) debug trace surface and future ops tooling.

8. **ADR-0121: Data Plan for Scale.** Addresses A2. The 100-user data plan — per-user SQLite file, per-tenant Postgres, shared-with-tenant-id, etc. Every table added under v1.4.0 entrenches the default. Requires founder decision before more substrate lands. Could be an amendment to [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md) rather than a new ADR; call either way.

---

## Decisions pending

Three decisions cannot be deferred past the v1.4.0 substrate landing. Each requires founder judgment.

### D1 — The 100-user data plan (A2)

**The question:** At 100 users across 10 tenants, does DailyOS run as per-user SQLite files, per-tenant Postgres, or shared DB with tenant_id filtering?

**Why it can't wait:** Every table added in v1.4.0 entrenches the default assumption (per-user local SQLite). If the eventual answer is per-tenant Postgres, we're building 10 tables of debt right now.

**Options:**
- **(a) Per-user SQLite, indefinitely.** Control plane coordinates identity; each user's data never leaves their device. Aligns with [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md). Caps at single-user utility — no team intelligence, no shared analyst workflows.
- **(b) Per-tenant Postgres at multi-user threshold.** Migrate each tenant's data from N SQLite files to 1 Postgres schema. Enables team intelligence. Breaks the "content never leaves the device" framing of [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md). Requires a big migration at ~10-user mark.
- **(c) Hybrid: per-user local-first, per-tenant shared cache.** Claims live per-user locally; team-visible aggregates live per-tenant. Complex; two consistency models.

**Recommendation:** decide in writing before the next substrate issue ships. Default to (a) if no strong signal; migration to (b) is painful but possible. The point is choosing, not picking the "right" answer — any of these is survivable if committed to.

### D2 — ADR-0116 commercial commitment

**The question:** Does the founder commit in writing that softening the "metadata only, content never" boundary requires founder approval + named compensating control?

**Why it can't wait:** First enterprise conversation that asks for team activity dashboards will test the boundary. If the boundary softens once, it rots. If the answer is "we won't commit to that," better to know before [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md) is treated as load-bearing for the strategy.

### D3 — Config strategy (S10)

**The question:** Where do `config/scoring.toml`, `config/trust_compiler.toml`, eval harness `quality.toml` live at runtime?

**Recommendation:**
- Bundled compiled defaults in the binary.
- User config at `$APP_SUPPORT/dailyos/config/` overrides (macOS path).
- Boot-time validation fails fast on malformed config; falls back to defaults with WARN log.
- No hot-reload in v1.4.0 (restart to apply).

This is a mechanical decision, not a strategic one — but it needs to land before any config-consuming code ships.

---

## Reference architecture artifacts — the bridge from decision to execution

For AI sessions to execute v1.4.0 substrate work reliably, they need reference artifacts they can reach in one click from the repo root. Current state: the architecture docs exist but pre-date v1.4.0 by a month.

### Top-priority artifacts (land before v1.4.0 substrate ships)

Priority is set by "how much does an AI session need this to not misimplement."

1. **`.docs/ARCHITECTURE.md`** — top-level onboarding doc. Distill the strategy into ~1500 words aimed at a contributor (human or AI) picking up work. The dependency graph. The thematic groupings. The bets. The data plan decision (D1). One-click reach from repo root. Replaces `_archive/ARCHITECTURE.md`.

2. **Refresh of [`.docs/architecture/DATA-MODEL.md`](../architecture/DATA-MODEL.md)** — add v1.4.0 tables (`intelligence_claims`, `agent_trust_ledger`, `claim_corroborations`, `claim_contradictions`, `evaluation_traces`, `invalidation_jobs`, `publish_drafts`, `publish_outbox`, `confirmation_tokens`, `db_key_metadata`) with their relationships. Document the append-only + supersede pattern with a worked example.

3. **Refresh of [`.docs/architecture/MODULE-MAP.md`](../architecture/MODULE-MAP.md)** — add `src-tauri/src/abilities/{read,transform,publish,maintenance}/`, `scoring::factors`, signal policy registry location, `DbKeyProvider` trait location, `ServiceContext` definition.

4. **`.docs/architecture/CONSISTENCY.md`** — new. Addresses A4. Names the consistency model per operation class (claim reads, claim writes, signal propagation, trust computation, publish, evaluator retry). One page.

5. **`.docs/architecture/FAILURE-MODES.md`** — new. Addresses A6. One-page matrix: Glean down / PTY unavailable / consistency check fails / SQLite locked / invalidation queue full → what works, what user sees, recovery path.

### Second-priority artifacts (land before first ability migration)

6. **Refresh of [`.docs/architecture/DATA-FLOWS.md`](../architecture/DATA-FLOWS.md)** — add propose/commit flow, trust compiler pass, runtime evaluator loop, Pencil/Pen publish flow. Mermaid diagrams for each.

7. **Refresh of [`.docs/architecture/LIFECYCLES.md`](../architecture/LIFECYCLES.md)** — add claim state machine (proposed → committed → superseded / tombstoned / withdrawn), actor taxonomy, agent trust ledger updates.

8. **`.docs/architecture/VERSIONING.md`** — new. Addresses A9. When to bump `ability_version` vs `ability_schema_version` vs `prompt_template_version` vs `provenance_schema_version` vs `trust_version`. A half-page.

### Third-priority artifacts (land as observability contract materializes)

9. **Refresh of [`.docs/architecture/PIPELINES.md`](../architecture/PIPELINES.md)** — add invalidation job model details, coalescing, rate limits, dead-letter handling.

10. **Refresh of [`.docs/architecture/SELF-HEALING.md`](../architecture/SELF-HEALING.md)** — add durable jobs, cycle detection, dead-letter queue, healing rate limits.

---

## The hard blockers, addressed

The code-reality check identified two hard blockers ADRs assume but don't exist. These need explicit issues filed in v1.4.0 **before** any ADR implementation begins.

### Blocker 1: ServiceContext struct does not exist

**Affects:** ADR-0104, and transitively every ADR that threads `&ServiceContext` through mutations (0101, 0102, 0103, 0113, 0115, 0116, 0117, 0119).

**Action:** DOS-209 (ExecutionMode and Mode-Aware Services, currently GREEN) is the natural home. Amend the issue to specifically call out the ServiceContext struct as the Phase 0 deliverable. Land it first; nothing else compiles without it.

### Blocker 2: IntelligenceProvider trait does not exist

**Affects:** [ADR-0091](../decisions/0091-intelligence-provider-abstraction.md) (existing), [ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) (amendment). PTY + Glean intelligence paths run inline today without a shared trait.

**Action:** File a new issue to extract `IntelligenceProvider` trait, migrate PTY orchestration + `glean_provider.rs` to implement it. Precondition for [ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md) amendment. Sized in hours.

---

## Senior engineer + systems architect findings, summarized

Full detail in [persona-reviews.md](2026-04-20-persona-reviews.md). High-urgency list (must happen before v1.4.0 substrate ships):

- **S1 Day-1 observability** — ADR-0120.
- **S2 Error handling contract** — ADR-0102 amendment.
- **S3 Provenance size cap** — ADR-0108 amendment.
- **S6 Concurrency on claim commits** — ADR-0113 amendment.
- **S9 Structured logging** — ADR-0120 (same as S1).
- **S10 Config loading strategy** — decision D3.
- **A2 Data gravity** — decision D1.
- **A6 Failure modes** — FAILURE-MODES.md artifact.
- **A7 Observability contract** — ADR-0120 (same as S1/S9).

Nine items collapse to: two new ADRs (0120, possibly 0121), four amendments, one reference doc, two decisions. All sized in hours.

---

## Execution order

Priority sorted. Each step sized in hours at AI velocity unless gated on external input (decisions).

1. **Make decisions D1, D2, D3.** Founder-gated. Cannot proceed without.
2. **Write ADR-0120 Observability Contract.** Foundational for S1, S9, A7. Enables every other day-1 observability concern.
3. **File / amend blockers.** DOS-209 amendment (ServiceContext explicit Phase 0); new issue for IntelligenceProvider trait extraction.
4. **Write the six ADR amendments.** ADR-0102 (×2: S2 + A5), ADR-0104 (Phase 0), ADR-0108 (S3), ADR-0113 (S6), ADR-0115 (A3).
5. **Produce the top-priority artifacts** (ARCHITECTURE.md, DATA-MODEL refresh, MODULE-MAP refresh, CONSISTENCY.md, FAILURE-MODES.md).
6. **Run /plan-eng-review against the aggregate** (strategy action 5). This is the second review pass specifically on the aggregate shape, informed by everything above.
7. **Ship ServiceContext + IntelligenceProvider trait** (blockers). Enables ADR implementation to begin.
8. **Ship end-to-end slice** (strategy action 1). First ability end-to-end through every substrate surface.
9. **Establish baselines.** Metrics instrumented per strategy doc before targets become meaningful.
10. **Continue v1.4.0 ADR implementation.** Now unblocked.

---

## What's NOT in this document

By design:

- **Implementation specifics** — this is readiness, not implementation. The ADRs hold the designs.
- **Revised timelines** — at AI velocity, the meaningful timeline question is "what order" not "how long." Steps above are ordered; not date-scheduled.
- **Opportunity cost accounting** — strategy doc's explicit-non-goals section covers this.
- **Risk register** — strategy doc's four-bets section covers this.
- **Any claim of "we will definitely ship X by date Y."** Commitments come with founder sign-off, not in draft synthesis.

---

## Summary for the reader

We have good ADRs, good spec template, good rubric, and good Linear issue hygiene (21/24 already rubric-ready). We have two hard code-level blockers (ServiceContext, IntelligenceProvider) that must land before implementation. We have three decisions pending that are founder-gated (data plan, ADR-0116 commitment, config strategy). We have six ADR amendments and two new ADRs (0120, 0121) to address persona-review findings. We have seven architecture docs that are accurate to pre-v1.4.0 and need refresh as substrate lands.

None of this is unexpected. All of it is addressable in short order at AI velocity. The bridge from decision to execution is constructable; this document is the inventory.
