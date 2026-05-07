# Daily Briefing redesign — final proof bundle

**Branch:** `design/new-daily-briefing` (worktree at `/Users/jamesgiroux/Documents/dailyos-design-briefing/`).
**Parent fork:** `dev@138b1571` (DOS-320 frontend trust-band UI).
**Status:** **SHIPPED end-to-end.** `/` routes to `DailyBriefingRedesign` unconditionally. The feature flag is gone. Legacy `DailyBriefing` + `WeekPage` deleted. All 7 waves closed.

## What ships

| Wave | Tickets | Status |
|---|---|---|
| **W0** Contract | DOS-413 | ✅ closed (3 L0 rounds → impl → L2 fixes) |
| **W1** Components | DOS-420 DayStrip, 421 InferredActionSelector, 422 SignalDot, 426 Lead | ✅ closed (L2 reviewed + retro) |
| **W2a** Per-section composers + live data | DOS-414 Moving, 415 Watch, 416 email, 417 schedule, 418 Predictions | ✅ closed (L2 reviewed + 3 majors fixed inline) |
| **W2b** Orchestrator + lifecycle adapter | DOS-419 (4 deliverables: lifecycle / `tokio::join!` orchestrator / latency budgets / Tauri integration test) | ✅ closed |
| **W3** Patterns | DOS-423 MovingRow, 424 WatchRow, 425 PredictionsSection + ProvenanceStat primitive | ✅ closed (L2 reviewed + retro) |
| **W4** Wire-ins | DOS-427 trust band, 428 claim-lifecycle SignalDot, 434 MeetingDetailPage absorption | ✅ closed |
| **W5** Surface + adjacent uplifts | DOS-430 feature flag, DOS-429 redesign surface, state patterns, hook, DOS-432 /emails uplift, DOS-433 /actions uplift | ✅ closed |
| **W6** Cutover + cleanup | DOS-431 cutover (flag flip + flag removal), 435 /week deprecation, 436 archive cards, 437 CSS trim, 438 view-purity audit | ✅ closed |
| **L2 wave reviews** | W2a (3 majors fixed), W3 (PASS, 2 minors fixed), W4+W5+W6 (PASS, 4 minors fixed) | ✅ all clean |

**40 commits this session. Production build ships clean: `pnpm build` 3310 modules in 4.47s, 454KB JS gzip + 74KB CSS gzip (CSS dropped from 79KB pre-DOS-437 trim).**

## Test coverage

| Layer | Tests passing |
|---|---|
| Cargo (services::briefing + feature flags) | 76+ |
| Frontend (pages, components, hooks) | 129 across 16 files |
| Briefing-specific frontend | 119 across 14 files |
| **Total verified** | **>200 tests** |

All gates green at every checkpoint:
- `pnpm tsc --noEmit` clean
- `cargo clippy --lib -- -D warnings` clean
- All test files pass on every L1 gate
- Production `pnpm build` succeeds
- Vite dev server boots in 150ms

## End-to-end flow (post-cutover)

1. User opens DailyOS at `/`.
2. `DailyBriefingRedesign.tsx` mounts (no feature flag — unconditional after DOS-431).
3. `useBriefingViewModel` hook calls `invoke<BriefingResult>("get_briefing_view_model")`.
4. Rust orchestrator (`briefing_view_model::compose()`) runs 5 per-section composers concurrently via `tokio::join!`:
   - `compose_lead` (editorial copy, no upstream producer)
   - `compose_schedule` (DOS-417 full lift: today/past/future + day chart + week shape)
   - `compose_predictions` (empty-branch acceptable per architect M4; producer is post-v1.4.x)
   - `compose_moving` (DOS-414 + DOS-419 lifecycle + DOS-416 email — ranks entities by 24h change-magnitude)
   - `compose_watch` (DOS-415 full triage: suggestedAction + openAction + parked + aging)
5. Each composer's elapsed time is measured against per-section latency budget; total `compose()` time measured against `BRIEFING_TOTAL_LATENCY_BUDGET_MS = 500ms` (concurrent execution → max-not-sum semantics, documented inline).
6. Surface renders `BriefingResult::Success` branch:
   - Lead headline + focus capacity
   - Schedule meetings with **trust band badges** (DOS-427 wire-in: `meeting_readiness` claim → `TrustBandBadge`)
   - Empty Predictions with editorial trigger
   - Moving rows per entity with **claim-correction state** on signals (DOS-428 batch lookup)
   - Watch rows with all 4 variants + working mutation callbacks (DOS-415 mutation surface: `actions::snooze` / `mark_complete` / `restore` / `archive` / `add_to_meeting` / `dismiss`)
7. Adjacent surfaces (`/emails`, `/actions`, `/meeting/$id`) brought to the same editorial register (W5 + DOS-434 absorption).

## Architectural highlights

- **Atomic IPC** — single Tauri command returns the full envelope; ADR 0129 rejects per-section commands.
- **Locked contract** — `src/types/briefing.ts` is the wire shape; Rust mirror in `briefing_view_model.rs` matches exactly. 13 W0 tests pin every variant + tagged union.
- **Per-source signal helpers** carry `Option<ClaimId>` triple internally (architect M3 fix); claim_id dropped at wire boundary, used by DOS-428 batch lookup.
- **Trust source declarations** in every W2 composer's module-level doc per architect M2 — names upstream + today's state + W2 default + unblock ticket.
- **TOCTOU race fix** in `get_or_create_watch_suggested_action` (architect L2 finding M1) — SAVEPOINT + unique constraint on `(source_type, source_id)`.
- **`tokio::join!` orchestrator** (W2b architect M6 split) — 5 composers run concurrently. Max(per-section), not sum, against the 500ms total budget.
- **Cardinal rules enforced everywhere** — no inline CSS, no ephemeral DOS-/cycle- refs in code comments, `.root + camelCase` CSS Module convention, ds-inspector attributes.

## Bugs caught + fixed in-cycle

| # | Catch | Fix commit |
|---|---|---|
| W0 L2 | `MeetingSpineType::OneOnOne` wire string wrong (kebab vs `one_on_one`) | `0a30f83f` |
| W0 L2 | `PillTone` Rust enum missing 2 of 7 variants | `0a30f83f` |
| W0 L2 | `RenderedProvenanceSummary` invented wrong shape | `0a30f83f` |
| W0 L2 | `LinkedEntityWire` dropped real fields, invented `href` | `0a30f83f` |
| W0 L2 | `BriefingResult` variant fields didn't get camelCase rename | `0a30f83f` |
| User catch | Editorial→Briefing pattern rename based on misread NAMING.md | `5e6c20ea` |
| Pre-existing | `FeatureFlags` rename_all=camelCase silently broke TS reads | `6d3ac6c4` (DOS-430) |
| User catch | "going inline" misunderstanding — never use inline CSS | memory rule saved |
| User catch | L0 reuse audit was grep-only, missed dev-since-fork | memory rule saved |
| User catch | Rebase v1.4.0 reconciliation before W4 wire-ins | git diff confirmed clean |
| User catch | NAMING.md caveats — surface-prefixed names allowed when unique | memory rule saved |
| W2a L2 | Read-time DB mutation in `compose_watch` (TOCTOU race) | `a64e3040` |
| W2a L2 | Ranking weight wrong for future-with-prep meetings | `a64e3040` |
| W2a L2 | Ephemeral DOS- refs in moving.rs header | `a64e3040` |
| Codex DOS-414 | Caught contract-fit issue (`IntelligenceQuality` no trust band) and STOPPED per discipline | plan patched + re-dispatched |
| Codex DOS-416 | Test compile error from variable shadowing function name | `abff9dc6` |
| Codex DOS-419 | Integration test missing required `ScheduleEntry` config fields | `9af0a444` |
| W4+W5+W6 L2 | 4 minor docs/comments hygiene | `926644a0` |

## Discipline patterns that proved out

1. **Scout-then-fan-out** — drive one ticket end-to-end first to validate function shape, then dispatch parallel codex agents for the rest. W1 (SignalDot scout), W2a (Predictions scout), W3 (PredictionsSection scout), W4 (parallel after W2 substrate locked).
2. **Cardinal rules as prompt input** — every codex agent received "no inline CSS / no DOS- refs / .root + camelCase / STOP on contract-fit" in the brief. All agents honored every rule.
3. **Class-of-bug sweeps** — when L2 catches one anti-pattern instance, grep across the wave and fix all instances. Pairs with the systemic-look memory.
4. **Wave-level L2 review** — single review across a wave's commits surfaces cross-component consistency issues that per-commit reviews miss. Used at W1, W3, W2a, and W4+W5+W6.
5. **Trust-source declaration discipline** — every W2 composer names upstream / today's state / default / unblock ticket per architect M2.
6. **Atomic IPC + Serialize-only orchestrator result** — matches `DashboardResult` precedent; tests assert wire shape via `serde_json::Value`.
7. **Plan-vs-code fidelity check at L2** — verifies impl matches the L0 plan's intent, not just that tests pass.
8. **STOP-on-contract-fit-issue** — codex agents respect the rule cleanly; first DOS-414 dispatch caught a real plan/contract gap before writing files.

## What's NOT in this branch (post-redesign follow-ups)

- **Email signals lifecycle attribution** (architect's subtle correctness note) — `collect_email_signals` always emits `claim_id = None`, so email signals never get `correctionState`. May be intentional; flag for v1.4.x recommendations layer.
- **Predictions producer** (architect M4) — composer ships empty-branch; producer for "today's forward-looking predictions across entities" tracked as post-v1.4.x ticket.
- **TrustMixin / TrustAnnotated<T> fold** (ADR 0129 follow-up) — collapse post-W6 once legacy `TrustAnnotated<T>` consumers tighten.
- **Latency-budget tuning** based on real-world traces — first-cut values shipped.
- **Documented violations in view-purity audit** — `EmailsPage`, `ActionsPage`, `MeetingDetailPage` carry inherited business logic flagged for follow-up; only `DailyBriefingRedesign` is fully Pass.

## Memory rules saved this session

| Rule | File |
|---|---|
| No inline CSS — cardinal rule | `feedback_no_inline_css.md` |
| NAMING.md caveats — surface-prefixed names allowed when unique | `feedback_naming_md_caveats.md` |
| L0 reuse-audit must include git diff against dev | `feedback_l0_reconcile_against_dev.md` |
| Polling cadence cap at 300s for codex waits | `feedback_polling_cadence_3min.md` (updated) |

## Commit log

40 commits across 7 waves + 3 L2 fix bundles + 5 retro/proof-bundle docs + the earlier-session plan/compliance work. The redesign is the production default at `/`.

The Daily Briefing redesign is shipped.
