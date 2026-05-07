# DOS-415 - Watch full triage - L0 plan

**Wave:** W2a - Daily Briefing service composers
**Status:** L0 draft, plan-only deliverable
**Scope:** Finish the Watch service triage rules behind `WatchViewModel`.

## 1. Acceptance Criteria Checklist

- [ ] `compose_watch(state: &AppState) -> WatchViewModel` emits all four locked row variants: `suggestedAction`, `openAction`, `parked`, and `aging`.
- [ ] `suggestedAction` rows are claim-bearing rows backed by the abilities runtime, not unproven free-text suggestions.
- [ ] `openAction` rows represent real action records pressing today and can be marked complete by action id.
- [ ] `parked` rows represent real action records currently snoozed with a human-readable reason.
- [ ] `aging` rows represent stale backlog actions that need restore/archive triage.
- [ ] The TODAY-relevance filter is explicit, tested, and service-owned.
- [ ] The aging threshold rule is explicit, tested, and service-owned.
- [ ] Every emitted action affordance has a real mutation path or this ticket adds one.
- [ ] All fact-bearing rows carry `TrustMixin`; suggested-action rows also carry `LifecycleMixin`.
- [ ] Missing upstream data degrades to an empty Watch section, never fabricated rows.
- [ ] No display-layer filtering moves into TS components.
- [ ] No inline CSS is introduced.
- [ ] No ephemeral ticket/cycle references are added to code comments.
- [ ] W2a merge gate remains intact: no top-level `briefing_view_model::get_briefing_view_model` orchestration edits.

## 2. Trust-Source Declaration

Per the architect's W2a merge gate, Watch must name the trust source for claim-bearing rows.

- **Suggested-action upstream claim type:** `crate::abilities::claims::ClaimType::SuggestedOutcome`, persisted as `intelligence_claims.claim_type = "suggested_outcome"`.
- **Ability output type:** `crate::abilities::prepare_meeting::synthesis::SuggestedOutcome`.
- **Publish path:** `draft_claims_for_publish()` converts `MeetingBrief.suggested_outcomes` into `ClaimDraft { claim_type: "suggested_outcome", ... }`.
- **Canonical subject:** `SuggestedOutcome` is registered for meeting subjects only, so Watch joins it to today's meetings.
- **Today's state:** claim type and ability output exist; the missing piece is a Watch-specific loader for active surfaced `suggested_outcome` claims relevant today.
- **This ticket owns:** the Watch loader/adapter plus action-id materialization or lookup, because `WatchSuggestedActionRow.actionId` must be a real action id accepted by `actions::*` mutations.
- **Fallback:** if no active surfaced `suggested_outcome` claims exist for today, emit zero `suggestedAction` rows. Do not emit unscored placeholder suggestions.
- **Trust band mapping:** use claim trust via the existing claim-trust-band helper; fall back to `trustBand: "unscored"`.
- **Rendered provenance:** parse claim `provenance_json` into `RenderedProvenanceSummary` when valid; invalid/empty provenance becomes `None`.
- **Lifecycle default:** map `ClaimVerificationState::Active` to `correctionState: "none"` and preserve contested/corrected state if already present.

If L2 rejects action-id materialization inside this ticket, the plan must be revised before implementation. A suggested-action row without a durable action id is not acceptable under the locked TS contract.

## 3. Mutation-Existence Verification

W0 rev 3.1 requires every W2 service plan to verify the mutations it emits.

| Watch semantic | Variant / affordance | Current state | DOS-415 requirement |
|---|---|---|---|
| `actions::snooze` | `suggestedAction` selector option | Missing. `snooze_triage_item` exists for Health triage cards, but it is entity/triage-key based and has no action-level reason contract. | Add action-level snooze with `action_id`, `snoozed_until`, `reason`, and `source = "daily_briefing"`. Persist enough data for `WatchParkedRow`. |
| `actions::dismiss` | `suggestedAction` selector option | Partial. `dismiss_suggested_action` service/command exists and `daily_briefing` is an allowed source. | Use existing behavior for backlog suggested actions. Add a thin wrapper only if the frontend command surface needs the exact semantic name. |
| `actions::mark_complete` | `openAction` check button | Partial. Existing command/service is `complete_action`; no exact `mark_complete` command. | Reuse `complete_action` or add a naming wrapper. Do not duplicate DB logic. |
| `actions::restore` | `aging` restore option | Partial. `accept_suggested_action` covers backlog -> unstarted; `reopen_action` covers completed -> unstarted. No exact restore mutation. | Add `restore_action` service/command for Watch aging rows; backlog rows use accept semantics and emit restore-specific source/signal. |
| `actions::archive` | `aging` archive option | Partial. `ActionDb::archive_action` exists; service wrapper and Tauri command are missing. | Add `services::actions::archive_action` and register a command in `lib.rs`. |
| `actions::add_to_meeting` | `suggestedAction` selector option | Missing. No action-to-meeting mutation exists. | Add a mutation that links an action to a meeting without overwriting action source provenance. Prefer `action_meeting_links` over repurposing `source_id`. |

Mutation tests must assert missing/terminal/invalid ids fail deterministically. Watch must not emit an option whose mutation is absent.

## 4. Existing MVP vs This Ticket

Existing MVP in `src-tauri/src/services/briefing/watch.rs` already covers:

- Produces `WatchViewModel` with label, heading, count label, summary, and rows.
- Calls `services::actions::get_all_actions(state)` and falls back to an empty section on upstream empty/error.
- Maps `ActionStatus::Started` and `ActionStatus::Unstarted` to `WatchOpenActionRow`.
- Maps `ActionStatus::Backlog` to `WatchAgingRow`.
- Filters `Completed`, `Cancelled`, and `Archived`.
- Emits `TrustMixin { trustBand: Unscored }` for current rows.
- Tests empty branch, serialization, open rows, backlog aging rows, terminal filtering, missing-account placeholder, and summary pluralization.

This ticket adds/fixes:

- Live candidate query that preserves real statuses; current `get_all_actions` maps DB rows to `Unstarted`, so backlog/aging is mostly test-only today.
- `WatchSuggestedActionRow` from active surfaced `suggested_outcome` claims.
- `WatchParkedRow` from action-level snoozes with reasons.
- Full TODAY-relevance filtering.
- Concrete aging threshold instead of "every backlog action is aging."
- Mutation verification and create-mutation work for every Watch affordance.
- Claim provenance/trust mapping for suggested actions.

## 5. TODAY-Relevance Filter

The Watch composer should start from a broad candidate set, then filter down to items pressing today. The filter is service-owned and tested against a fixed clock.

An item is pressing today when one of these is true:

1. `status in (unstarted, started)` and `due_date <= today`.
2. `status = started`, even without a due date.
3. `status = unstarted`, `priority in (0, 1, 2)`, and `created_at` or `updated_at` is today.
4. Active surfaced `suggested_outcome` claim whose meeting subject is scheduled today.
5. Underlying candidate would be pressing by rules 1-4, but an active action snooze exists with a non-empty reason; render as `parked`.
6. Backlog action satisfies the aging threshold below.

Always exclude:

- `completed`, `cancelled`, and `archived` action statuses.
- Suggested-outcome claims that are not active/surfaced.
- Suggested-outcome claims not attached to today's meeting subject.
- Active snoozes without a reason; suppress those rows instead of rendering vague parked items.
- Duplicate candidates for the same `action_id`.

Row precedence for the same action id: `parked`, then `aging`, then `suggestedAction`, then `openAction`.

Date handling:

- Derive `today` from the app/service clock, not direct `Utc::now()` calls in triage helpers.
- Compare parsed ISO dates, not raw strings.
- Pin the service clock in tests and cover local-midnight boundaries.

## 6. Aging Threshold Rule

Backlog actions graduate to `WatchAgingRow` when the user should restore them to active work or archive them deliberately.

- Candidate must have `status = backlog`.
- Candidate must not have an active snooze.
- Candidate must be at least 14 days old.
- Age basis is `due_date` when present and before/equal to today; otherwise `created_at`.
- `ageLabel` is compact: `14d`, `21d`, or `30d+`.
- `since` is the ISO date used as the age basis.
- Rows at 30+ days still render if present, but tests should note the existing stale-action archive job may remove them before Watch sees them.

Non-goals:

- Do not alter the scheduler's auto-archive policy.
- Do not age completed, cancelled, or archived actions.
- Do not turn fresh backlog suggestions into aging rows only because they lack claim provenance.

## 7. Files This Lands

Plan file already landed by this task:

- `.docs/plans/wave-W2-redesign/DOS-415-plan.md`

Implementation target paths for DOS-415:

- `src-tauri/src/services/briefing/watch.rs` - full triage rules, claim-backed suggested rows, parked rows, TODAY filter, tests.
- `src-tauri/src/services/actions.rs` - action-level mutation wrappers for missing Watch semantics.
- `src-tauri/src/db/actions.rs` - watch candidate queries, action snooze persistence, archive/restore helpers where needed.
- `src-tauri/src/commands/actions_calendar.rs` - Tauri commands for new action mutations.
- `src-tauri/src/lib.rs` - command registration for any new Tauri commands.
- `src-tauri/src/migrations/143_watch_action_triage.sql` - proposed next migration for action snoozes and action-meeting links, subject to migration-number availability at implementation time.
- `src-tauri/src/migrations.rs` - register the migration.
- `src-tauri/src/services/briefing_view_model.rs` - only if Rust wire helpers/tests need fixture updates; do not change the locked wire shape without ADR follow-up.
- `src/types/briefing.ts` - no expected change; the union already contains all four variants and required mutation ids for suggested/aging rows.

No TS component files should be touched in DOS-415. WatchRow rendering belongs to the pattern ticket, and this service ticket must not introduce inline CSS.

## 8. Out Of Scope

- Building or changing `src/components/dashboard/WatchRow.tsx`.
- Styling, CSS modules, or reference HTML updates.
- Top-level briefing orchestrator changes.
- Moving, Schedule, Predictions, or Lead service behavior.
- Full W4 trust/lifecycle visual treatment beyond preserving existing claim state fields.
- Actions page or Meeting Detail UI updates to consume new mutations.
- Stale-action auto-archive scheduler changes.
- Linear push/sync semantics.
- Global action-status vocabulary refactors.

## 9. L1 Self-Validation Gates

- `cargo test --lib services::briefing::watch`
- `cargo test --lib services::actions`
- `cargo test --lib db::actions`
- `cargo check --lib`
- `cargo clippy --lib -- -D warnings`
- `pnpm tsc --noEmit` only if any TypeScript contract or command consumer file is touched.

Required targeted test coverage:

- Empty Watch section still serializes to the same camelCase shape.
- Today's active/due action maps to `openAction`.
- Future non-started action is filtered out.
- Today's `suggested_outcome` claim maps to `suggestedAction` with trust and provenance.
- Non-today `suggested_outcome` claim is filtered out.
- 13-day backlog action is not aging; 14-day backlog action maps to `aging`.
- Active action snooze with reason maps to `parked`; active snooze without reason suppresses the row.
- Terminal statuses never emit rows.
- Every selector option emitted by `suggestedAction` has verified mutation backing.
- Row precedence prevents duplicate rows for the same action id.

## 10. L2 Reviewers

- `/codex review` - diff-level adversarial review for mutation safety and triage edge cases.
- `code-reviewer` subagent - Rust service/query/migration review.
- `architect-reviewer` - confirm trust-source declaration, TODAY relevance, aging threshold, and W2a merge-gate compliance.

L2 should explicitly check that no new code comment contains ephemeral ticket/cycle references and that the implementation does not move filtering logic into display components.
