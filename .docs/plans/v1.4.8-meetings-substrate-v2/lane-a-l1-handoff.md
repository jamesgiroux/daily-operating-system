# Lane A L1 Implementation Handoff

**Paste this into a fresh session as the priming context.** It's self-contained — no need to read the prior session transcript.

---

## Where you are

You're starting L1 implementation on **DOS-774 — Lane A: Surface-relevant meetings projection** in the **v1.4.8 Meetings Substrate v2** wave. Lane A's L0 packet cleared cycle-3 with unanimous APPROVE from all 3 reviewers (K-in, feasibility, codex) on 2026-05-26.

Lane A **closes DOS-771** (phantom briefing advisories — "4 BRIEFINGS NEED PREP" / "Link 4 meetings" on a 0-meeting calendar day). It also re-enables `DailyBriefingAbilityStrip` that was removed in PR #392 as a tactical workaround.

**Working dir:** `/Users/jamesgiroux/Documents/dailyos-repo`
**Branch from:** `dev`
**Proposed branch name:** `fix/v1.4.8-lane-a-meetings-projection`

## Read these first (in order)

1. **`.docs/plans/v1.4.8-meetings-substrate-v2/lane-a-l0.md`** — the approved L0 packet. Has §0 symptom-to-failure trace, §3 implementation, §4 AC, §6 sequencing.
2. **`.docs/plans/v1.4.8-meetings-substrate-v2-waves.html`** — wave doc for outcome framing (user-facing payoff).
3. **`.docs/plans/engineering-ladder.md`** — L1 procedures, especially L4-before-L2 for user-facing changes.
4. **`CLAUDE.md`** — project rules. All mutations go through `services/`. No PII in code or commit messages. L2-status in commit messages.

Skip:
- The Lane B L0 packet (separate ticket, ships after Lane A).
- Prior session transcripts (this doc is the handoff).

## Pre-L1 diagnostic step (do this first)

Before touching the substrate, **add a `tracing::info!` line** at `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:88` logging `(meeting.id, meeting.title, meeting.starts_at, meeting.meeting_type)`. Restart the app, load the briefing page, observe runtime log.

**Goal:** verify the personal-blocks-leak hypothesis at 10/10 before the substrate change ships. The 4 phantom rows on a 0-meeting day should have `meeting_type = "personal"`. If they don't, the diagnosis is wrong and the substrate change won't close DOS-771 — stop and reassess before implementing.

**Remove the diagnostic line before merge.** It's pre-L1 verification, not shipping code.

## What to build (L0 packet §3, summarized)

### 1. New substrate file: `src-tauri/src/services/meetings_view.rs`

```rust
use chrono::NaiveDate;
use chrono_tz::Tz;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingsViewIntent {
    Briefing,    // excludes personal, cancelled, all-day
    Schedule,    // same exclusions today; reserved for future divergence
    AllRows,     // raw range + archived-transcript filter only
}

pub fn read_surface_meetings(
    db: &crate::db::ActionDb,
    workspace_scope: &str,
    date: NaiveDate,
    tz: &Tz,
    intent: MeetingsViewIntent,
) -> Result<Vec<SurfaceMeeting>, String> { /* ... */ }
```

- Owns the TZ-aware UTC range computation (lifted from `services/context.rs:826-866`).
- Owns the per-intent filter (lifted from `focus_capacity.rs:169-179 should_exclude_meeting`).
- Returns a `SurfaceMeeting` type — for Lane A scope you can make this a re-export of `DailyReadinessMeetingSnapshot` or a thin newtype; pick whichever keeps the diff smallest.

ADR cites in the doc comment:
- ADR-0102 boundary heuristic — trivial projection is service-layer, not a new ability
- ADR-0101 Rule 5 — `read_*` functions don't write

### 2. Trait migration: `DailyReadinessContextReadHandle`

`src-tauri/abilities-runtime/src/services/context.rs` (trait def around line 1630):

```rust
pub trait DailyReadinessContextReadHandle: Send + Sync {
    fn read_daily_readiness_context<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
        intent: MeetingsViewIntent,        // ← new
    ) -> DailyReadinessContextReadFuture<'a>;
}
```

Ripples to:
- Live impl: `src-tauri/src/services/context.rs:791-816`
- Wrapper: `src-tauri/abilities-runtime/src/services/context.rs:2301-2313`
- Test fixture 1: `src-tauri/tests/w5_a_get_daily_readiness_test.rs:95`
- Test fixture 2: `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:967`

Live impl threads `intent` into `read_surface_meetings`. Test fixtures accept and ignore it.

### 3. Consumer migrations (2 callers)

| Caller | File:line | Declared intent |
|---|---|---|
| `get_daily_briefing` ability | `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:69` | `MeetingsViewIntent::Briefing` |
| `get_daily_readiness` ability | `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:1025` | `MeetingsViewIntent::Briefing` |

Both pass `intent: MeetingsViewIntent::Briefing` when calling the readiness handle.

**Out of scope (filed as follow-up):**
- `services/dashboard.rs:532, :1121 compute_focus_capacity` callers
- `services/entities.rs:319-396` calendar-merge consumer

Lane A leaves `should_exclude_meeting` at `focus_capacity.rs:169-179` AS-IS — it stays in place for the dashboard/entities path. The semantic twin in `services/meetings_view.rs` covers the readiness path. **Two implementations of one policy until the follow-up lands** — file the follow-up ticket at commit time (see "Follow-up ticket" below).

### 4. Strip revival

Restore `DailyBriefingAbilityStrip` and helpers (`briefingFreshnessLabel`, `briefingAdvisoryLabel`, `renderedProvenanceSourceCount`, `useDailyBriefingAbility` import) in `src/components/dashboard/DailyBriefing.tsx`. Also restore the strip test in `DailyBriefing.test.tsx` (the "renders ability-backed briefing trust and provenance state" test).

**Source commit:** `8467cc7e` — that's the parent of PR #392's strip-removal commit `7cc79aa9`. Confirmed clean by Lane A cycle-2 codex challenge — no sibling drift since then.

```bash
git show 8467cc7e:src/components/dashboard/DailyBriefing.tsx > /tmp/DailyBriefing.tsx
git show 8467cc7e:src/components/dashboard/DailyBriefing.test.tsx > /tmp/DailyBriefing.test.tsx
```

Then merge those back in. PR #392's removal was 188 lines; this puts them back.

### 5. Regression test

Fixture covering a day with `MeetingType::Personal` rows. Verify:
- `MeetingsViewIntent::Briefing` excludes them.
- `MeetingsViewIntent::AllRows` includes them.

The fixture should match the DOS-771 case shape (4 personal blocks on a calendar day, 0 entity-linked customer meetings).

## Acceptance criteria (L0 packet §4)

1. `MeetingsViewIntent` enum + `read_surface_meetings` function shipped.
2. `DailyReadinessContextReadHandle::read_daily_readiness_context` signature carries `intent`. Live impl + 2 test fixtures + abilities-runtime wrapper updated.
3. The 2 readiness-handle callers (briefing + readiness abilities) consume the new projection with declared intent.
4. `DailyBriefingAbilityStrip` restored; strip test restored.
5. **DOS-771 closes** — verified live on a 0-meeting calendar day: no "BRIEFINGS NEED PREP" / "Link N meetings" advisories; strip shows "Current" freshness label.
6. Regression test: `Briefing` excludes personal blocks; `AllRows` includes them.
7. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` clean.
8. **L4 before L2** per ladder — strip revival is user-facing; hands-on validation on today page **before** L2 dispatch.
9. Follow-up ticket filed for dashboard/entities migration.

## L4 verification (do before L2 dispatch)

On a 0-meeting calendar day (today's situation):
- Strip renders. Freshness label says "Current".
- No "X BRIEFINGS NEED PREP" advisory.
- No "Link N meetings for fuller context" advisory.
- Trust band reads correctly (likely "unscored" with 0 source claims — that's fine).

On a day WITH meetings (any other day):
- Strip renders with accurate freshness / trust / source count.
- Advisory CTA fires correctly if there are real unlinked meetings.

If the strip still shows phantom counts on a 0-meeting day → Lane A failed; don't L2-dispatch; trace deeper.

## L2 dispatch (after L4 passes)

Per ladder Standard-tier L2:
- **Required:** `/codex review` via `l2-bounded-reviewer` agent (AC-scoped).
- **Router-selected (likely):** `pr-review-toolkit:type-design-analyzer` (new enum + new function signature + trait migration) + `ce-api-contract-reviewer` (trait change is a substrate API contract).
- **Advisory parallel (always-on, non-blocking):** standard set.

## Follow-up ticket (file at L1 commit time, before merge)

**Title:** "v1.4.8 follow-up — Migrate compute_focus_capacity + executive_intelligence paths to `read_surface_meetings`"

**Why now:** Lane A intentionally leaves the policy in two places (`focus_capacity.rs:169-179` + `services/meetings_view.rs`). The follow-up closes the wave's "policy lives in one place" mission. Targets:
- `src-tauri/src/services/dashboard.rs:430-470, :502-509, :532, :1121`
- `src-tauri/src/services/entities.rs:319-396`

Adds a `Meeting ↔ SurfaceMeeting` conversion (helper / re-export / unification — L1 of that ticket picks).

File as a child of DOS-773 in the v1.4.8 — Meetings Substrate v2 project.

## Commit + PR rules (per CLAUDE.md memory)

- Branch from `dev`, PR to `dev`. `trunk` is tagged releases only.
- Three version files must stay in sync: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json`. Lane A is a fix; bump per the project's versioning convention or leave alone if not version-bump-worthy.
- Commits include `Co-authored-by: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.
- Commit-msg hook requires `L2-status: passed | not-run-acknowledged | n-a-doc-only` on code-touching commits.
- **No PII in commits or PR body.** Pre-commit `.githooks/pre-commit` enforces on file content; commit messages are your responsibility.
- **PR template:** `security_auditor_invoked: true | false` is required. For this PR, code touches `src-tauri/src/services/`, `abilities-runtime/`, `src-tauri/abilities-runtime/src/services/context.rs` — that triggers the `when_changed` rule in `.github/reviewer-prompts/matrix.yml`, so set `true`. Rationale: defensible substrate work (intent enum, projection function, trait migration); no new auth/permission surface; threat topology local-to-local single-user.

## Linear update protocol

At commit time:
- Comment on DOS-774 with implementation start + branch link.
- Comment on DOS-771 with verified-live evidence when L4 passes.
- File the follow-up ticket as a child of DOS-773.

At merge:
- DOS-771 closes (per AC #5).
- DOS-774 moves to Done.
- DOS-773 parent updated noting Lane A merged + handoff predicate now ready for Lane B.

## Background — what L0 surfaced

The L0 review (cycles 1-3) caught issues a fresh L1 implementation would have hit on day 1:
- Original §3.4 consumer table was wrong on 2 of 4 entries (only 2 actually call the readiness handle). Cycle-2 corrected to narrow scope.
- 3 ADRs to cite (0102, 0101 Rule 5, 0111).
- Strip revival source commit identified (`8467cc7e`).

You don't need to re-derive any of that. The L0 packet has it all.

## Key memory items to honor

- **All mutations go through `services/`.** No direct DB writes from command handlers.
- **Read the codebase before committing to a plan.** Grep for things; don't assume from architectural intuition.
- **L4 hands-on BEFORE L2 for user-facing fixes.** Don't dispatch L2 reviewers on a strip that hasn't been visually verified.
- **No `--no-verify` on commits** unless explicitly authorized.
- **No deferrals — period.** Once scope is agreed, finish it. If something's blocking, surface and fix root cause.
- **Stop and reassess if symptom doesn't move after a fix.** Per DOS-771's own history: 4 patches before stepping back to root cause.

## Authority

- Linear is canonical (DOS-774).
- Git keeps the L0 packet + PR + this handoff doc.
- James is L6.
