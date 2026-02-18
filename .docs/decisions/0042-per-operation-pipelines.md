# ADR-0042: Per-operation pipelines with progressive delivery

**Date:** 2026-02-06
**Status:** Accepted

## Context

The monolithic three-phase pattern (Prepare → Enrich → Deliver) was designed for the CLI, where one command produced the full daily briefing. The app has scheduling, background execution, and progressive UI rendering — it doesn't need everything in one shot.

In practice, the monolithic pipeline is fragile. If any step fails partway through — a Google API timeout, a Phase 2 enrichment error, a malformed markdown parse — the entire delivery fails. The app renders partial or malformed data because `deliver_today.py` is all-or-nothing.

ADR-0030 decomposed Phase 1 into atomic operations (`ops/`). Phase 2 (one big Claude Code invocation) and Phase 3 (one big deliver script) remain monolithic bottlenecks.

Key insight: some operations don't need AI at all. Schedule, actions, and focus data are fully structured after Phase 1. Gating them behind AI enrichment adds latency and fragility for zero benefit.

## Decision

Each atomic operation runs its own pipeline: **prepare → deliver (mechanical)** or **prepare → enrich → deliver (AI-dependent)**. Operations execute independently, fail independently, and deliver progressively.

**Mechanical operations (no AI, deliver immediately after Phase 1):**
- `calendar:fetch` → `schedule.json`
- `action:sync` → `actions.json`
- `gap:analyze` → `focus.json`

**AI-enriched operations (deliver after per-operation Claude Code call):**
- `meeting:prep` → per-meeting Claude call → `preps/{id}.json`
- `email:enrich` → Claude call → `emails.json` with summaries
- `briefing:generate` → Claude call (reads delivered JSON files) → briefing narrative

**Execution model:**
1. Orchestrator kicks off all Phase 1 operations (parallel where possible)
2. Mechanical operations deliver immediately — app has a working dashboard
3. AI-enriched operations run concurrently as separate Claude Code invocations
4. Each AI call is smaller, focused, faster, and can fail independently
5. Briefing narrative runs last — it's the capstone that synthesizes across all delivered data
6. Frontend renders progressively as each operation's JSON arrives

**ADR-0006 is preserved, not superseded.** The determinism boundary still holds: deterministic phases wrap each AI call. The change is granularity — N small deterministic-AI-deterministic pipelines instead of one large one.

**Feature toggles (ADR-0039) gate operations at the orchestrator level.** If Gmail isn't connected, `email:fetch` is skipped entirely — but calendar and actions still run and deliver. This prevents incomplete runs at the source.

## Consequences

**What becomes easier:**
- Resilience: one operation's failure doesn't cascade to others
- Latency: mechanical data appears instantly; AI enrichment layers on progressively
- Debugging: each operation is independently testable and observable
- Reactive updates: calendar poll → single meeting:prep pipeline (I41) is just another operation
- Feature toggles: naturally per-operation instead of all-or-nothing
- Cost: smaller, focused AI prompts may be cheaper than one mega-prompt

**What becomes harder:**
- More Claude Code invocations per briefing cycle (cost/concurrency management)
- Briefing narrative needs to handle partial data (some operations may not have completed)
- Per-operation enrichment prompts need to be maintained separately
- Orchestrator complexity increases (parallel execution, progressive delivery, operation dependencies)

**What this retires:**
- Monolithic `deliver_today.py` / `deliver_week.py` — replaced by per-operation delivery
- Single mega-prompt for Phase 2 — replaced by focused per-operation prompts
- All-or-nothing execution — replaced by progressive, fault-tolerant delivery
