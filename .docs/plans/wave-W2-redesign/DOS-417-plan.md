# DOS-417 - Full calendar lift - L0 plan

**Wave:** W2 redesign  
**Status:** L0 draft  
**Scope:** Finish the schedule slice that W2a already wired to real dashboard calendar data.

## 1. Acceptance Criteria

- [ ] `compose_schedule(state: &AppState) -> ScheduleViewModel` no longer defaults every non-current meeting to `upcoming`.
- [ ] Meetings classify as `past`, `in-progress`, `upcoming`, or `cancelled` from calendar time data.
- [ ] `ScheduleMeeting.state` and `stateTags` match the temporal classification.
- [ ] `MeetingTimeViewModel` has start ISO, best-effort end ISO, formatted start label, and factual duration label.
- [ ] `DayChartViewModel` has hour ticks, legend, one bar per chartable meeting, and `nowLine` when today is in range.
- [ ] The equivalent +/-7 day week shape currently derived in `WeekPage.tsx` moves into the Rust schedule service layer.
- [ ] Trust source is declared: meetings come from `get_dashboard_data`, which comes from Google Calendar through the existing pipeline.
- [ ] Trust band defaults to `Unscored` until DOS-427 W4 wire-in.
- [ ] No inline CSS is added.
- [ ] No ephemeral `DOS-` references are added to code comments; while touching `schedule.rs`, replace current ticket-ID comments with durable wording.

## 2. Current Repo Reality

`src-tauri/src/services/briefing/schedule.rs` already calls `get_dashboard_data(state).await`, maps `DashboardData.meetings` into `ScheduleMeeting`, degrades to the empty schedule branch on upstream empty/error, sets meeting trust to `TrustBandWire::Unscored`, leaves the day chart empty, and marks only `m.is_current == true` as `InProgress`.

Frontend duplication to retire or stop expanding:

- `DailyBriefing.tsx` finds up-next and unprepped meetings with local temporal logic.
- `BriefingMeetingCard.tsx` exports `getTemporalState` based on display strings and optional `currentMeeting`.
- `WeekPage.tsx` derives +/-7 day timeline buckets, readiness stats, future meeting filters, labels, and shape rows locally.
- `weekPageViewModel.ts` derives Mon-Fri `DayShape[]` in TypeScript.

## 3. Trust-Source Declaration

Source chain:

```text
Google Calendar -> existing calendar ingestion/upsert -> dashboard service -> get_dashboard_data -> compose_schedule
```

The schedule composer is not the source of calendar truth. It reshapes the existing dashboard meeting list into the locked `ScheduleViewModel`.

Trust policy:

- `ScheduleMeeting` remains fact-bearing and keeps `TrustMixin`.
- `trustBand` is `TrustBandWire::Unscored` for every meeting in this ticket.
- `trustFieldPath`, `trustSourceDate`, and `renderedProvenance` remain unset unless W4 trust work lands first.
- The implementation PR must repeat this declaration and must not describe the meeting list as AI-derived.

## 4. Service Shape

Keep the public composer:

```rust
pub async fn compose_schedule(state: &AppState) -> ScheduleViewModel
```

Add pure helpers so tests can use a fixed clock:

```rust
fn compose_schedule_from_meetings(
    meetings: Vec<Meeting>,
    now: DateTime<Utc>,
) -> ScheduleViewModel
```

The async wrapper should only fetch dashboard data, read the clock, and call the pure helper. Prefer `state.live_service_context().clock.now()` over adding another direct wall-clock read inside the schedule service.

Lift the WeekPage-only shape into the same service ownership area:

```rust
pub async fn compose_week_schedule_shape(
    state: &AppState,
    days_before: i64,
    days_after: i64,
    now: DateTime<Utc>,
) -> Result<WeekScheduleShape, String>
```

If `schedule.rs` gets crowded, split pure logic into `src-tauri/src/services/briefing/schedule_service.rs` and keep `schedule.rs` as the composer entrypoint. Do not create a frontend-only replacement service.

## 5. Temporal Grouping Rule

Inputs:

- Primary start instant: `Meeting.start_iso`.
- Best-effort end instant: derive from `Meeting.end_time` plus parsed start date unless a reliable ISO end is added before implementation.
- Cancellation: `Meeting.overlay_status == Cancelled` wins over time math.
- Clock: injected into pure helpers as `DateTime<Utc>`.

Rules:

- Cancelled: `ScheduleMeeting.state = Cancelled`, state tag includes `cancelled`, chart bar state/kind are `cancelled`.
- Otherwise parse `start_iso` as RFC3339.
- End derivation: prefer reliable ISO end; otherwise parse display `end_time` on the local date of `start_iso`; if parsed end is not after start, add one day only for a valid overnight edge.
- Missing end: use a 45 minute estimate for state and chart layout only.
- `in-progress`: `start <= now < end`.
- `past`: `end <= now`.
- `upcoming`: `now < start`.
- Missing/invalid `start_iso`: keep the meeting renderable, use `upcoming` unless cancelled, omit chart bar, and keep `startsAtIso` empty/default.

Do not add grouping fields to `ScheduleViewModel`. The locked shape already carries grouping via `ScheduleMeeting.state` and `stateTags`.

Ordering:

- Valid-time meetings sort chronologically.
- Cancelled meetings stay in chronological position.
- Invalid-time meetings sort after valid ones, preserving source order.

## 6. Time Labels And Duration

`MeetingTimeViewModel`:

- `startsAtIso`: source `start_iso`, or empty string.
- `endsAtIso`: derived end ISO, or empty string.
- `startLabel`: formatted from parsed `start_iso`, falling back to `Meeting.time`.
- `durationLabel`: factual `45m`, `1h`, or `1h 30m` only when a real end is derived.

Do not show the 45 minute fallback as a factual duration. It is only for state estimation and chart width when end data is missing.

## 7. Day Chart Algorithm

Default range stays `8` to `20`, with total minutes `(rangeEndHour - rangeStartHour) * 60`.

Hour ticks:

- emit every two hours, including endpoints: `8 AM`, `10 AM`, `12 PM`, `2 PM`, `4 PM`, `6 PM`, `8 PM`
- under the default range, ticks can be unmuted

Bar layout:

- Convert start/end into local minutes since midnight.
- `start_offset = start_minutes - rangeStartHour * 60`.
- `end_offset = end_minutes - rangeStartHour * 60`.
- Clip both offsets to `[0, total_minutes]`.
- Omit a bar only when no valid start exists or the meeting is completely outside the visible range.
- `leftPct = visible_start / total_minutes * 100`.
- `widthPct = (visible_end - visible_start) / total_minutes * 100`.
- Apply a small minimum visual width after calculating the true value so short meetings remain visible.

Bar content:

- `kind`: mapped from `MeetingType` to `DayChartBarKind`; unknown mappings fall back to `internal`.
- `state`: `past`, `now`, `upcoming`, or `cancelled`.
- `title`: meeting title.
- `timeLabel`: start/end label when available.
- `tooltip`: title plus time range/duration when available.

Legend:

- Include only kinds present in bars.
- Canonical order: `customer`, `partner`, `internal`, `oneOnOne`, `personal`, `project`, `cancelled`.
- Do not add a wire enum here.

Now-line:

- Render only when the schedule date is the local date containing `now`.
- Render only inside `[rangeStartHour, rangeEndHour]`.
- `leftPct` uses the same percent formula as bars.
- `label = "Now"`.
- `isoTime = now.to_rfc3339()`.
- Outside range, `nowLine = None`.

## 8. Week Shape Lift

Match current `/week` behavior before W6 deletes it:

- default range: `daysBefore = 7`, `daysAfter = 7`
- preserve current `get_meeting_timeline` source behavior, including the always-live future Google Calendar fetch
- group by local date boundaries, not `toISOString().slice(0, 10)`
- preserve labels: `Today`, `Yesterday`, `Tomorrow`, `N days ago - Weekday, Mon D`, and `Weekday, Mon D`
- preserve buckets: earlier past, recent past, today, future
- preserve Mon-Fri density thresholds: `packed` 5+ meetings, `heavy` 4, `moderate` 2-3, `light` 0-1
- compute meeting minutes from start/end when available; use 45 minutes only as density fallback
- preserve `computeWeekMeta` and `computeShapeEpigraph` semantics

The service does not need to remain public after W6. It needs to create a clean removal target so W6 removes a route/service call rather than scattered frontend schedule math.

## 9. Files

Primary implementation:

- `src-tauri/src/services/briefing/schedule.rs`: temporal parsing/classification, labels/durations, chart ticks/legend/bars/now-line, week-shape extraction or call into sibling helper, durable comments.

Optional extraction:

- `src-tauri/src/services/briefing/schedule_service.rs`: pure temporal, chart, and week-shape helpers if `schedule.rs` gets too large.

Removal targets:

- `src/components/dashboard/DailyBriefing.tsx`: stop expanding local up-next/unprepped temporal logic; remove duplicated classification after redesigned schedule cutover.
- `src/components/dashboard/BriefingMeetingCard.tsx`: keep `getTemporalState` only for legacy daily briefing until no callers remain.
- `src/pages/WeekPage.tsx`: replace local timeline/shape/readiness derivation with service-owned shape if `/week` still exists after this lands; add no inline CSS.
- `src/pages/weekPageViewModel.ts`: delete or reduce after Rust owns the shape.
- `src/types/briefing.ts`: no expected contract change; use existing `ScheduleViewModel` and `DayChartViewModel`.

## 10. Coordination With DOS-435

DOS-435 can delete `/week` only after the +/-7 day shape is no longer owned exclusively by `WeekPage.tsx`.

This ticket must leave one clear W6 removal target:

- schedule-service week-shape entrypoint
- any temporary Tauri command or frontend call that consumes it
- parity tests proving it matches current WeekPage grouping

W6 then removes the `/week` route, `WeekPage.tsx`, unused week CSS, and any temporary week-shape endpoint if no other consumer remains.

## 11. Out Of Scope

- live calendar wiring; the MVP already shipped through `get_dashboard_data`
- new Google Calendar fetch behavior
- trust scoring/provenance beyond `Unscored`
- W5 redesigned schedule component rendering
- `/week` deletion
- new mutations
- new `ScheduleViewModel` fields unless L2 explicitly requires a contract amendment
- inline style workarounds for chart positioning

### File-coordination boundary with DOS-427 (W4)

**Both this ticket (W2a) and DOS-427 (W4) edit `src-tauri/src/services/briefing/schedule.rs`.** The boundary contract:

- **DOS-417 (this ticket) owns:** the function signatures of `compose_schedule`, `map_meeting`, `map_meeting_type`, `build_state_tags`, `compute_meeting_mix`, `format_count_label`, `format_summary`, `extract_entity_name`, plus any new helpers added for temporal grouping / day-chart bars / now-line / week shape. DOS-417 owns the full set of tests it lands.
- **DOS-427 (W4) owns:** replacing the `TrustBandWire::Unscored` literal in `map_meeting`'s `TrustMixin` with a real lookup against `meeting_readiness` claims. DOS-427 must preserve every helper signature DOS-417 establishes and must not break any DOS-417 test.

The seam: DOS-427 will likely add a new helper (`load_trust_band_for_meeting(&meeting) -> TrustBandWire`) and call it from inside `map_meeting` where the `Unscored` literal lives today. That's the only edit point DOS-427 needs in this file. If DOS-427's L0 plan grows beyond that, escalate.

**At L1 of DOS-427's PR:** re-run DOS-417's full test suite (`cargo test --lib services::briefing::schedule`) and confirm zero regressions. DOS-427's PR description must cite this coordination boundary explicitly.

## 12. L1 Gates

Rust:

- `cargo check --lib`
- `cargo clippy --lib -- -D warnings`
- `cargo test --lib services::briefing::schedule`

Required schedule tests:

- cancelled wins over time math
- fixed-clock past/current/upcoming classification from RFC3339 start/end
- invalid `start_iso` stays renderable and omits chart bar
- duration labels format minutes/hours and skip estimated fallback
- chart ticks, legend, bars, and now-line percentages are stable
- now-line omitted outside range
- serialization remains camelCase and contract-compatible

Required week-shape tests:

- local-date grouping avoids UTC boundary drift
- earlier/recent/today/future buckets match current `WeekPage.tsx`
- Mon-Fri density thresholds match `deriveShapeFromTimeline`
- +/-7 day boundaries match current defaults

Frontend/type gates if frontend consumers are edited:

- `pnpm tsc --noEmit`
- targeted tests for edited daily/week files
- grep touched files for `style={{` and remove new inline CSS

Comment hygiene:

- `rg -n "DOS-[0-9]+" src-tauri/src/services/briefing/schedule.rs` returns no code-comment references after implementation.

## 13. L2 Gates

- **code-reviewer:** temporal edge cases, chart math, no contract drift, fixed-clock coverage.
- **architect-reviewer:** trust-source declaration, `Unscored` default, service boundary, W6 `/week` removal target.
- **design-reviewer if frontend is touched:** no inline CSS, no duplicate chart semantics, chart data supports the intended visual treatment.

## 14. Implementation Order

1. Add pure parsing, duration, and temporal classification helpers with tests.
2. Replace `map_meeting` state logic with derived temporal context.
3. Populate day chart ticks, bars, legend, and now-line from the same context.
4. Add week-shape extraction and parity tests.
5. Wire `compose_schedule` to the pure helper after `get_dashboard_data`.
6. Touch frontend only if needed to consume the lifted week shape before W6.
7. Run L1 gates and attach the trust-source note to the PR.
