# ADR-0030: Composable workflow operations

**Date:** 2026-02
**Status:** Accepted

*Rewrites the original "Weekly prep generation with daily refresh" proposal. Supersedes [ADR-0035](0035-incremental-prep-generation.md).*

## Context

The current workflow system has two problems:

**1. Monolithic Python scripts.** `prepare_today.py` (1400 lines) and `prepare_week.py` (750 lines) each perform calendar fetch, meeting classification, gap analysis, and config resolution independently. They duplicate Google API auth, workspace resolution, and classification logic. Adding a new capability (e.g., refreshing just emails) requires running the entire pipeline.

**2. Coupled concerns.** Meeting prep generation, email triage, action sync, and daily overview synthesis all run as a single atomic operation in `/today`. This means:
- Meeting preps can't be generated ahead of time (e.g., for the full week)
- A new meeting detected by calendar polling can't get prep without re-running the entire briefing
- Feature toggles (ADR-0039) can't disable individual capabilities — it's all or nothing
- Each workflow reinvents data fetching instead of sharing it

Meanwhile, the Rust side already demonstrates the right pattern: inbox processing (`processor/`), calendar polling (`calendar_merge.rs`), and archive (`archive.rs` + `reconcile.rs`) are each isolated, independently callable operations.

## Decision

Decompose workflows into **atomic operations** that orchestrators compose.

### Atomic Operations

| Operation | Responsibility | Trigger |
|-----------|---------------|---------|
| `calendar:fetch` | Fetch events from Google Calendar for a date range, classify by meeting type | Called by orchestrators |
| `meeting:prep` | Generate prep for a single meeting — attendee context, account history, past notes, file refs | Called per-meeting |
| `email:fetch` | Fetch unread emails from Gmail, classify by priority tier (ADR-0029) | Called by `/today` |
| `action:sync` | Parse actions from workspace markdown + merge SQLite-sourced actions (post-meeting, inbox) | Called by `/today` |
| `inbox:process` | Classify, route, and enrich files in `_inbox/` | Timer (already isolated) |
| `calendar:poll` | Detect new/changed/cancelled meetings against briefing baseline | Timer (already isolated) |
| `archive:reconcile` | Transcript status check, action stats, day summary, morning flags, file moves | Nightly (already isolated) |

Each operation is independently callable. Operations don't know about each other.

### Orchestrators

Orchestrators are thin — they compose operations and write output JSON.

**`/week`** — runs weekly (Monday AM) + on-demand if no preps exist:
1. `calendar:fetch` (Monday–Friday)
2. `meeting:prep` × N (for each classified meeting)
3. `action:sync` (week-scope summary)
4. Gap analysis + focus blocks
5. Write `week-overview.json` + `preps/*.json`

**`/today`** — runs daily (6 AM) + on-demand:
1. Check: do preps exist for today's meetings? If not, call `meeting:prep` for missing ones
2. `email:fetch`
3. `action:sync`
4. Overview synthesis (the AI enrichment step — Phase 2)
5. Write `schedule.json`, `actions.json`, `emails.json`, `overview.json`

**`calendar:poll` → `meeting:prep`** — reactive, continuous:
- When polling detects a new meeting, call `meeting:prep` for that single meeting
- This replaces the need for a separate "incremental prep" workflow (supersedes ADR-0035)

**`inbox:process`** and **`archive:reconcile`** — already independent, no changes needed.

### What `/today` No Longer Does

- Generate meeting preps from scratch (owned by `/week` or reactive `meeting:prep`)
- Fetch calendar events for classification (preps already exist; calendar polling handles changes)
- Gather meeting contexts and file references (that's `meeting:prep`'s job)

### What `/today` Still Owns

- Email fetch and classification (inherently daily — yesterday's emails aren't today's)
- Action sync (status changes overnight, new inbox actions, completed items)
- Daily overview synthesis (the "your day is ready" output — the AI enrichment step)
- Freshness check: if a prep is missing for a today meeting, trigger `meeting:prep`

### Three-Phase Pattern

ADR-0006's determinism boundary still applies **per-operation** where AI enrichment is needed. `meeting:prep` and `/today`'s overview synthesis are the two operations that involve Phase 2 (Claude). The others are purely deterministic.

## Consequences

- `prepare_today.py` decomposes from 1400 lines into focused, single-responsibility modules
- `prepare_week.py` and `prepare_today.py` stop duplicating calendar fetch, classification, and config resolution
- Meeting preps are ready days in advance — the Week page shows "prep ready" instead of perpetual "prep needed"
- `/today` runs faster — it assembles from pre-computed preps + fresh email/action signals
- Feature toggles (ADR-0039) can enable/disable individual operations (e.g., email triage off, meeting prep on)
- New meetings get prep within minutes via calendar polling, not hours via next briefing
- Shared Google API coordination (I18) becomes natural — operations share a fetch layer instead of each hitting APIs independently
- Migration is incremental: extract one operation at a time from the monoliths, validate, repeat

## 2026-02-18 Alignment Note

This ADR's decomposition was never fully implemented. The per-meeting prep operation exists inside `prepare_today()` but is not independently callable. `prepare_week()` generates only `week-overview.json` (summary) — no individual meeting preps. Calendar polling detects changes but doesn't trigger `meeting:prep`.

ADR-0081 (Event-Driven Meeting Intelligence, 0.13.0) implements this ADR's vision end-to-end:
- **I326** extracts `meeting:prep` into a truly independent operation with lifecycle management
- **I327** wires it into the weekly orchestrator (individual intelligence for all meetings in forecast window) and calendar polling (reactive generation for new meetings)
- **I331** implements the "today assembles from pre-computed" model — `/today` no longer generates preps from scratch

The atomic operations table and orchestrator composition described in this ADR remain the architectural target. ADR-0081 adds: intelligence quality tracking, signal-triggered refresh, and classification expansion (all meetings, not just external).
