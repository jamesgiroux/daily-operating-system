# ADR-0122 — Chapter-level Enrichment Strategy

**Status:** Proposed
**Date:** 2026-04-23
**Issue:** DOS-204
**Stakeholders:** James Giroux
**Gates:** DOS-203 (Health & Outlook tab), DOS-15 (Glean leading-signal), DOS-207 (Context tab schema)

---

## Context

### Current Architecture (from static analysis)

DailyOS already runs **per-dimension parallel enrichment** (I574/I576). When `intelligence_context` is available, `run_parallel_enrichment()` fans out 6 separate PTY/Glean calls — one per dimension — and merges results progressively as they complete (I575). The 6 dimensions are:

1. `core_assessment` → executiveAssessment, currentState, risks, recentWins
2. `stakeholder_champion` → stakeholderInsights, championHealth, coverageAssessment, organizationalChanges
3. `commercial_financial` → health, contractContext, renewalOutlook, expansionSignals, blockers
4. `strategic_context` → companyContext, competitiveContext, strategicPriorities
5. `value_success` → valueDelivered, successMetrics, successPlanSignals, openCommitments
6. `engagement_signals` → meetingCadence, emailResponsiveness, productAdoption, supportHealth, gongCallSummaries, npsCsat

The key architectural facts discovered during this exploration:

**Enrichment already fans out.** `run_enrichment()` dispatches one thread per dimension with `DIMENSION_ENRICHMENT_TIMEOUT_SECS = 240s`. The "all-or-nothing" framing in the issue description was accurate for the monolithic v1.0 prompt, but the system is already dimension-decomposed. The open question is whether dimensions can have *independent TTLs and independent triggers*, not whether they can run independently (they already can).

**Single enriched_at timestamp per entity.** `IntelligenceJson.enriched_at` is a single RFC 3339 field on the entity's top-level record. There is no per-dimension timestamp. The TTL check in `check_enrichment_ttl()` reads this single field and either skips or re-enriches the *entire* entity (all 6 dimensions).

**Queue operates at entity granularity.** `IntelRequest` carries `entity_id` + `entity_type` + `priority`. There is no way to express "re-enrich only stakeholder_champion for entity X." Deduplication in `IntelligenceQueue.enqueue()` also operates at entity granularity — if entity X is already queued for any reason, a new request for X is merged, not appended as a dimension-specific request.

**Signal propagation does not target dimensions.** `CALLOUT_SIGNAL_TYPES` and the propagation rules (`rules.rs`) emit signals at entity granularity. For example, `rule_person_job_change` emits `stakeholder_change` on the account — but this does not carry information about which chapter should be refreshed. Similarly, `rule_meeting_frequency_drop` emits `engagement_warning`, but the enrichment scheduler sees only "account X needs enrichment," not "account X's engagement_signals dimension needs enrichment."

**TTL is 2 hours (`ENRICHMENT_TTL_SECS = 7200`).** Background and calendar-triggered requests are gated by this TTL; Manual is not. The TTL is applied entity-wide.

**Trigger points that cause enrichment today:**
- `hygiene/loop_runner.rs` — overnight scan + hygiene sweep (ProactiveHygiene)
- `prepare/orchestrate.rs` — queues person intelligence after meeting prep (CalendarChange)
- `services/accounts.rs` — multiple hooks when account data changes (ContentChange)
- `services/people.rs` — similar (ContentChange)
- `commands/integrations.rs` — Glean/calendar integration callbacks (CalendarChange, Onboarding)
- Manual refresh button (Manual)

**The frontend already derives freshness from a single `enriched_at`.** `ChapterFreshness` was introduced in v1.2.1 as a component that reads `enriched_at`. All chapters show the same staleness indicator today.

### The Problem

The new tab-based dossier exposes a real staleness mismatch: the six dimensions have structurally different volatility:

| Dimension | Signal source | Change frequency |
|---|---|---|
| `core_assessment` | Transcripts, account updates | Weekly / on transcript |
| `stakeholder_champion` | Meeting transcripts, Clay job-change signals | On transcript or job-change signal |
| `commercial_financial` | CRM contract data | Once per renewal cycle (~quarterly) |
| `strategic_context` | Company news, strategic docs | Monthly at fastest |
| `value_success` | Transcript outcomes, user corrections | On transcript |
| `engagement_signals` | Meeting cadence, email signals (algorithmic) | Partially algorithmic already |

Running all 6 in parallel when a transcript lands is 6 PTY calls where 2–3 are necessary and 3–4 are stable. This is the waste to eliminate.

Note: `engagement_signals` is already partially algorithmic (health_scoring.rs computes meeting_cadence, email_engagement etc. at gather time, before any PTY call). Its dimension prompt adds LLM narrative and productAdoption synthesis, but the underlying scores are computed without AI.

---

## Evaluated Approaches

### A: Keep all-or-nothing (no change)

**What it means:** Continue re-running all 6 dimensions on every enrichment trigger. TTL stays at 2h, entity-granular.

**Where it wins:**
- Zero dev cost, zero migration risk.
- Parallel fan-out (I574) means wall-clock time is bounded by the slowest dimension (~240s), not the sum. Six calls in parallel is not 6× the cost of one.
- Coherence: all six dimensions are always from the same point-in-time context, which prevents internal contradictions (core_assessment says "at risk" while commercial_financial says "healthy" from a stale cache).

**Where it loses:**
- A transcript arrival triggers refreshes to `commercial_financial` and `strategic_context` — domains the transcript tells us nothing new about.
- LLM costs scale with enrichment frequency × 6 dimensions, not with actual change volume.
- Correctness: `commercial_financial` is refreshed weekly even though contracts move quarterly. The LLM will hallucinate "stable" based on stale context, not because it saw new data.

**Dev cost:** S. **Runtime savings:** 0%. **Correctness risk:** Low for most dimensions, moderate for commercial (stale context → stale output framed as fresh). **Frontend:** No change. **Migration:** None.

---

### B: Per-chapter TTL

**What it means:** Each dimension gets its own TTL constant. The enrichment queue gains a `dimension_mask` field (bitmask or set of dimension names). `check_enrichment_ttl()` checks per-dimension timestamps and only dispatches the stale subset. The DB gains per-dimension `enriched_at` timestamps (a `dimension_freshness_json` column on `entity_assessments`).

**Where it wins:**
- Clean and predictable. Ops team can reason about when each chapter will be refreshed.
- `commercial_financial` at 14-day TTL → saves ~13 PTY calls per renewal cycle per account.
- `strategic_context` at 30-day TTL → similar savings.

**Where it loses:**
- A stable chapter can get genuinely stale during a crisis (e.g., a customer's CFO is replaced mid-cycle — `commercial_financial` won't refresh until its TTL fires, even though a stakeholder signal arrived).
- TTL fires are time-driven, not signal-driven. If a customer goes dark in month 2 of a 3-month contract, `commercial_financial` won't surface the renewal risk until the TTL hits.
- Schema complexity: `dimension_freshness_json` means frontend needs to parse per-dimension timestamps and `ChapterFreshness` becomes more complex.
- Coherence risk increases: different dimensions at different staleness levels can produce internal contradictions.

**Dev cost:** M (IntelRequest schema, TTL check per-dimension, DB column + migration, merge logic changes, ChapterFreshness update). **Runtime savings:** ~30–40% on stable accounts (2–3 dimensions consistently skipped). **Correctness risk:** Moderate — stale stable dimensions can miss crisis signals. **Frontend:** `ChapterFreshness` reads per-dimension timestamp from a new JSON column. **Migration:** New DB column, new TTL constants, updated TTL check.

---

### C: Signal-driven per-chapter

**What it means:** Signals carry chapter affinity. A `stakeholder_change` signal targets `stakeholder_champion`. A `meeting_frequency` signal targets `engagement_signals`. The enrichment queue gains a `dimensions` field (set of dimension names to refresh). A new dispatcher maps signal types to dimension subsets. TTL is the *only* fallback for chapters that receive no signals.

**Where it wins:**
- Maximally efficient: only chapters that received relevant signals get enriched.
- Semantically correct: a transcript arrival refreshes `core_assessment`, `stakeholder_champion`, `value_success`, and `engagement_signals` — not `commercial_financial` or `strategic_context`.
- Extends the existing signal propagation model (I308/ADR-0080) rather than adding a competing TTL system.

**Where it loses:**
- Requires a signal affinity table (which signal types map to which dimensions) — a new piece of configuration that must stay in sync with CALLOUT_SIGNAL_TYPES and dimension schema as both evolve.
- Coherence risk is highest here: if only `stakeholder_champion` gets refreshed, `core_assessment`'s executive assessment can contradict the updated stakeholder picture until the next full refresh.
- Signals are currently entity-level. Plumbing dimension affinity through the signal bus is a multi-file change touching `bus.rs`, `rules.rs`, `event_trigger.rs`, `IntelRequest`, and `IntelligenceQueue`.
- Some important context (transcripts) affects 3–4 dimensions simultaneously. The affinity map must handle this correctly, and divergence from it silently wastes a re-enrichment.

**Dev cost:** XL (signal affinity table, IntelRequest schema, queue dispatch changes, partial merge logic, coherence reconciliation, TTL fallback path, testing). **Runtime savings:** ~50–60% (only signal-relevant dimensions run). **Correctness risk:** High — coherence between dimensions degrades as they drift. **Frontend:** Same `ChapterFreshness` complexity as Option B. **Migration:** XL — every signal type needs an affinity declaration.

---

### D: Hybrid — signal-driven primary + TTL fallback

**What it means:** Arriving signals target specific dimensions (C's mechanism). A background TTL sweep catches stable chapters that have not received signals in N days. The coherence problem is managed by a "drift check": when any dimension is refreshed, a lightweight consistency pass checks whether the updated dimension contradicts the cached state of other dimensions; if it does, the contradicting dimensions are queued for re-enrichment.

**Where it wins:**
- Best correctness model if implemented fully: signals drive the fast path, TTL catches drift, coherence guard prevents contradictions.
- Architecturally matches how the system *should* scale as Glean data matures.

**Where it loses:**
- Highest dev cost. The coherence guard alone is significant (consistency.rs already exists but does post-hoc contradiction detection, not pre-merge conflict resolution).
- The value over C is uncertain: if signals are well-calibrated, the TTL sweep is mostly idle; if signals are miscalibrated, TTL fires as frequently as the current all-or-nothing schedule.
- This is the right long-term architecture, but the wrong thing to build before Wave 3 tabs exist and before we have telemetry on how often each dimension actually changes.

**Dev cost:** XL+ (superset of C plus coherence guard). **Runtime savings:** ~45–55%. **Correctness risk:** Low if coherence guard is correct, High if it has bugs (could over-refresh). **Frontend:** Same as B and C. **Migration:** XL+.

---

## Decision

**Recommendation: Option A (keep all-or-nothing) for Wave 3, with preparatory instrumentation.**

The key architectural fact this exploration uncovered is that **the system is already dimension-parallel** (I574). The six dimensions already run as independent threads and emit progressive events. The problem is not execution coupling — it's TTL and queue granularity.

Building dimension-level TTL or signal-driven dimension targeting now has two problems:

**1. We have no telemetry.** We cannot measure what fraction of enrichment runs actually result in changed dimension output. Without this number, we cannot calculate savings. The issue estimates "running full enrichment on every signal = expensive + slow + churns stable chapters," but from static analysis, the 2-hour entity TTL already prevents most redundant runs. In practice, ProactiveHygiene is the primary driver of "stable chapter" waste — and that can be addressed by tuning the hygiene loop's account selection criteria rather than building a new dimension routing system.

**2. Coherence risk is real.** `core_assessment` and `commercial_financial` share vocabulary (`health`, `renewalOutlook`, `risks`). If they are enriched at different times from different context snapshots, their narratives will contradict each other. The existing consistency checker (I527, `consistency.rs`) runs post-merge and can flag contradictions, but it cannot resolve them without re-running a dimension. Dimension-level staleness makes this much more likely.

**What changes for Wave 3:** Nothing, and that is the right answer. DOS-203 (Health tab) and DOS-15 (Glean leading-signal) can both build on the current all-or-nothing model without wasted work. The ChapterFreshness component correctly shows a single entity-level enriched_at, which is accurate under the all-or-nothing model.

**What we instrument instead:**

The exploration cannot measure "how many dimensions actually change per run" because no per-dimension change-detection exists in the audit log. Before building any chapter-level system, add audit events for:
1. Per-dimension output hash at the end of each `run_parallel_enrichment()` thread (SHA-256 of dimension JSON output).
2. Compare to stored dimension hash in `entity_assessments`; log "changed" vs "unchanged" per dimension.
3. Run for 2 weeks across production accounts to get a real change matrix.

This telemetry would tell us: (a) which dimensions are stable across 90%+ of runs, (b) how strongly dimension volatility correlates with signal types, and (c) whether the problem is frequent enough to justify the coherence risk.

**If telemetry shows >40% of dimensions unchanged per run**, build Option B (per-chapter TTL) with a 2-week TTL for `commercial_financial` and `strategic_context`. Option D (hybrid) is the right eventual architecture but should not be built before Options B is proven.

---

## Answering the 5 Questions from the Issue

**Q1: Does chapter-level TTL make sense, or is per-signal-type enrichment the right axis?**

Neither is clearly superior without telemetry. TTL is simpler and safer (less coherence risk). Signal-driven is architecturally cleaner but requires a well-maintained affinity table and coherence guard. Start with TTL if savings prove necessary.

**Q2: Where's the boundary between "fresh because signals changed" vs "fresh because time passed"?**

Current system: signals trigger a full entity re-enrichment (ContentChange/CalendarChange priorities). Time-based sweep is ProactiveHygiene. The right boundary is: **signals drive re-enrichment when they carry new information the LLM needs** (transcripts, stakeholder changes, CRM updates); TTL drives re-enrichment for stable domains where signals are sparse. Today the system already makes this distinction implicitly — a CalendarChange triggers enrichment with higher priority than ProactiveHygiene, and the 2h TTL prevents redundant runs.

**Q3: Can we piggyback on existing signal propagation rules, or do we need a new dispatcher?**

We would need a new dispatcher. The signal propagation rules emit `SignalEvent`s at entity granularity; they do not carry dimension affinity. Adding dimension affinity would require changes to `bus.rs`, `rules.rs`, `event_trigger.rs`, and `IntelRequest`. This is the XL cost in Option C.

**Q4: How does the frontend render partial freshness per chapter?**

Under Option A (recommended): `ChapterFreshness` continues reading entity-level `enriched_at`. Per-chapter freshness is derived, not stored. This is accurate: all chapters have the same freshness timestamp because they were enriched together.

If Option B is adopted later: a `dimension_freshness_json` column on `entity_assessments` stores per-dimension timestamps. `ChapterFreshness` reads the appropriate dimension timestamp. The TS DTO for `AccountIntelligence` gains an optional `dimensionFreshnessJson` field.

**Q5: What's the cost of a refactor vs. the savings?**

From static analysis:
- Option B: M-sized refactor. Estimated savings: 30–40% of PTY calls on stable accounts with active enrichment.
- Option C/D: XL-sized refactor. Estimated savings: 50–60%. But coherence debt is real and hard to price.
- Current waste: unknown — needs telemetry. The 2h entity TTL already prevents most waste. ProactiveHygiene may account for most of the "stable chapters being re-enriched" concern.

---

## Consequences

### What changes if this ADR is adopted

1. **DOS-203 (Health & Outlook tab):** Build on current all-or-nothing model. ChapterFreshness shows entity-level enriched_at. No rework needed.
2. **DOS-15 (Glean leading-signal):** Already runs as a supplemental async pass after the main enrichment (per intel_queue.rs lines 773–846). This is already dimension-specific (health_outlook_signals_json only). No rework needed.
3. **DOS-207 (Context tab schema):** New `context_signals_json` column is the primary change in DOS-207. It should emit signals (Intelligence Loop check passed in v1.2.1 codex review). This does not depend on per-dimension enrichment — the new column is populated from existing dimensions.
4. **Telemetry instrumentation:** Add per-dimension output hash comparison in `run_parallel_enrichment()` (3–5 lines, no schema changes). Target: 2 weeks of production data before revisiting Option B.

### What gets harder

- If we later move to Option B, the `dimension_freshness_json` column migration requires a migration file, DTO changes, and ChapterFreshness update. This is not worse for having waited — the telemetry will make the right TTL values knowable.
- ProactiveHygiene waste continues until either TTLs are tuned or instrumentation guides pruning of the account selection criteria.

### What gets easier

- Wave 3 tab implementations proceed on stable foundations. No enrichment model changes to track during tab development.
- The intelligence coherence guarantee (all dimensions from same context) holds.

---

## Intelligence Loop Check (CLAUDE.md 5 questions)

1. **Does this data need to emit signals?** No new data is created by this ADR. Instrumentation (per-dimension hashes) is internal telemetry, not a user-visible signal surface.

2. **Does it feed any of the 6 health scoring dimensions?** No. Health scoring (`compute_account_health()`) is algorithmic and runs at gather time, before PTY. This ADR does not affect it.

3. **Should it appear in `build_intelligence_context()` or `gather_account_context()`?** No. Per-dimension TTL timestamps (if added in Option B later) would be used in the queue logic, not injected into prompts.

4. **Should it trigger briefing callouts via `CALLOUT_SIGNAL_TYPES`?** No. Enrichment scheduling is infrastructure, not a user-facing signal event.

5. **Does user interaction with this data need to feed back into Bayesian source weights?** No. This ADR addresses enrichment scheduling, not source weighting.

**Intelligence Loop verdict:** Enrichment scheduling is pure infrastructure. It passes the loop check by non-applicability.

---

## Gate Implications

**DOS-203 (Health & Outlook tab):**
Wave 3 can proceed on the current model. ChapterFreshness reads entity-level `enriched_at`. Sentiment hero, needs-attention cards, divergences, and renewal outlook all read from existing `IntelligenceJson` fields populated by the all-or-nothing enrichment. **Not blocked.**

**DOS-15 (Glean leading-signal enrichment):**
Already architecture-correct — it runs as a post-enrichment supplemental pass, independently of the 6 main dimensions. The enrichment model change in this ADR (no change = Option A) does not affect DOS-15's implementation path. **Not blocked.**

**DOS-207 (Context tab schema):**
DOS-207 adds new signals and schema columns that will need Intelligence Loop wiring. That work is independent of enrichment scheduling — new columns get enriched on the same all-or-nothing schedule as existing columns. **Not blocked.** If DOS-207 later identifies a dimension that changes so frequently it warrants its own TTL, that is a trigger to revisit Option B post-telemetry.

---

## Measurement Gap

One measurement was not possible from static analysis and requires user-triggered instrumentation:

**"How many dimensions actually change output per enrichment run?"**

To instrument: in `run_parallel_enrichment()`, after `merge_dimension_into()` succeeds, compute a SHA-256 of the merged dimension's JSON representation and compare to the hash of the same dimension from the prior enrichment stored in `entity_assessments`. Log `dimension_changed: true/false` per dimension per entity per run to the audit log. Aggregate after 2 weeks across all production accounts to answer: which dimensions are stable 90%+ of the time? This is the key number that determines whether Option B is worth building.
