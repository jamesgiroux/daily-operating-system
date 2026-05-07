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

Per the architect's W2a merge gate, Watch must name the trust source for every claim-bearing row variant. Format mirrors DOS-414's table (architect's N1 review note).

| Watch row variant | Source | Upstream | Today's state | W2a default | Unblocked at |
|---|---|---|---|---|---|
| `WatchSuggestedActionRow` | `intelligence_claims.claim_type = "suggested_outcome"` (per `crate::abilities::claims::ClaimType::SuggestedOutcome`). Ability output: `crate::abilities::prepare_meeting::synthesis::SuggestedOutcome`. Publish path: `draft_claims_for_publish()`. Canonical subject: meeting subjects only. | Claim type + ability output exist at fork point. Watch-specific loader for active surfaced claims relevant today does NOT yet exist. | DOS-415 lands the Watch loader + action-id materialization. If no active surfaced `suggested_outcome` claims exist today, emit zero `suggestedAction` rows (do not emit unscored placeholder suggestions). | `trustBand` from claim trust via existing claim-trust-band helper; fall back to `Unscored` if claim has no trust score. `renderedProvenance` parsed from `provenance_json` when valid; `None` when invalid/empty. `correctionState` from `ClaimVerificationState`: Active → `"none"`; preserve `corrected`/`contested` if already on the claim. | Today (DOS-415 itself). |
| `WatchOpenActionRow` | Action lifecycle status. Actions have no claim trust today. | MVP shipped at commit `3d5d5b3c` with `Unscored`. | Continue `Unscored`. | `Unscored`. | Pending action-as-claim modeling (post-v1.4.x track). |
| `WatchParkedRow` | Snooze record (DOS-415 introduces; backed by an `action_snoozes` table or schema extension). | Not yet wired. | DOS-415 lands the snooze persistence + reads. | `Unscored` (snooze is metadata, not a claim). | DOS-415 itself. |
| `WatchAgingRow` | Action lifecycle (status=Backlog, age threshold). Actions have no claim trust today. | MVP shipped with `Unscored` for Backlog → Aging. | Continue `Unscored`. | `Unscored`. | Pending action-as-claim modeling (post-v1.4.x track). |

**This ticket owns:** the Watch loader/adapter, action-id materialization or lookup (because `WatchSuggestedActionRow.actionId` must be a real action id accepted by `actions::*` mutations), and snooze record persistence.

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
- `src-tauri/src/migrations/{NNN}_watch_action_triage.sql` - new migration. **Migration number is assigned at implementation time** (`ls src-tauri/src/migrations/` to find the next available; do not assume `143` because the parent v1.4.0 track may have landed migrations since fork point). PR description must cite the chosen number.
- `src-tauri/src/migrations.rs` - register the migration.
- `src-tauri/src/services/briefing_view_model.rs` - only if Rust wire helpers/tests need fixture updates; do not change the locked wire shape without ADR follow-up.
- `src/types/briefing.ts` - no expected change; the union already contains all four variants and required mutation ids for suggested/aging rows.

No TS component files should be touched in DOS-415. WatchRow rendering belongs to the pattern ticket, and this service ticket must not introduce inline CSS.

### File-deny-list (W2a parallel-agent allowlist enforcement)

Per the wave plan's parallel-agent allowlist discipline (`waves.md:39-44`), W2a tickets running concurrently must declare which files are off-limits to other agents. **DOS-415 owns these files exclusively during W2a:**

- `src-tauri/src/services/briefing/watch.rs`
- `src-tauri/src/services/actions.rs` (any new mutation wrappers)
- `src-tauri/src/db/actions.rs` (watch candidate queries, snooze persistence)
- `src-tauri/src/commands/actions_calendar.rs` (new Tauri command registrations)
- `src-tauri/src/migrations/{NNN}_watch_action_triage.sql` (new file)
- `src-tauri/src/migrations.rs` (registration line)

Other concurrent W2a agents (DOS-414, 416, 417, 418) must NOT edit any of the above. `lib.rs` is shared (every W2a ticket may register Tauri commands there) — coordinate via small, distinct edits at known anchor lines. Conflicts at L1 escalate to the wave orchestrator (Claude/me).

### Scope split decision

L0 review (architect) flagged that DOS-415 is roughly 2x the work of other W2a plans because it bundles the Watch composer with new action-system mutations + a schema migration. **Decision: keep as one ticket.** Rationale:

- The composer cannot ship without the mutations it emits (per W0 plan rev 3.1 mutation-existence gate).
- The mutations cannot ship without the schema migration that backs `actions::snooze` and `actions::add_to_meeting`.
- Splitting into separate tickets would require the composer ticket to declare its mutation-existence verification as "done in DOS-NNN," creating a hard sequencing dependency without saving total work.
- The parallel-agent file-deny-list above prevents merge contention with other W2a tickets despite the larger scope.

If the impl agent finds the scope unmanageable in one cycle, escalate at L1 to split off the migration + mutation surface as DOS-415b. Do not attempt a partial DOS-415 ship that leaves Watch variants without their mutations.

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
