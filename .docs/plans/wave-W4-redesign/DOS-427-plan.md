# DOS-427 — Trust band wire-in for briefing meetings

**Wave:** W4 redesign
**Status:** L0 draft
**Parent track:** DOS-320 trust-band frontend infrastructure already shipped at the fork point; this ticket wires the schedule composer output into the redesign meeting rows.

## 1. Acceptance Criteria

- `ScheduleMeeting` items emitted by `src-tauri/src/services/briefing/schedule.rs` carry a real `trustBand` when a meeting-level trust source exists.
- The schedule composer keeps emitting `trustBand: "unscored"` when the upstream trust claim is missing, unreadable, or lacks a finite trust score.
- `src/pages/DailyBriefingRedesign.tsx` renders `TrustBandBadge` inline on briefing schedule meeting rows for the three scored bands:
  - `likely_current`
  - `use_with_caution`
  - `needs_verification`
- The redesign omits the badge for `unscored`, matching the established `PredictionsSection` behavior.
- The UI handles the full four-variant `TrustBandWire` union without casts that can pass `unscored` into `TrustBandBadge`.
- No inline CSS is introduced. Any layout adjustment uses the existing CSS module for the surface or a colocated module if a meeting-row sub-component is extracted.
- No new code comments contain ephemeral ticket references.
- Claim lifecycle correction visualization remains out of scope; this ticket only displays the current trust band.

## 2. Current State

`src/types/briefing.ts` already defines `ScheduleMeeting extends TrustMixin`, and the Rust mirror flattens `TrustMixin` into `ScheduleMeeting` via serde. The wire contract is already capable of carrying trust.

`src-tauri/src/services/briefing/schedule.rs` currently hard-codes:

```rust
trust_band: TrustBandWire::Unscored
```

`src/pages/DailyBriefingRedesign.tsx` renders schedule meetings as a minimal row with time and title only. It does not consume `meeting.trustBand`.

`src/components/ui/TrustBandBadge.tsx` is the canonical primitive, but its prop type deliberately excludes `unscored`. `src/components/dashboard/PredictionsSection.tsx` already owns the adapter pattern: convert `unscored` to `null`; render the badge only for the three scored bands.

## 3. Trust Source Declaration

**Source name:** `meeting_readiness` claim.

For schedule meeting cards, the visible trust band is sourced from the active meeting-level readiness/intelligence-quality claim:

- claim registry variant: `ClaimType::MeetingReadiness`
- persisted string: `claim_type = "meeting_readiness"`
- subject: `SubjectRef::Meeting { id: meeting.id }`
- score input: `intelligence_claims.trust_score`
- band mapping: existing trust threshold mapping from the trust/provenance layer

This source represents the quality and actionability of the meeting briefing as a fact-bearing claim. It is distinct from the legacy computed `Meeting.intelligence_quality: Option<IntelligenceQuality>`, which gives completeness/readiness labels but does not itself carry a trust score.

Fallback source, only when no active `meeting_readiness` claim exists: the most cautious scored band across related active meeting-prep claim types attached to the same meeting subject:

- `meeting_topic`
- `meeting_change_marker`
- `suggested_outcome`
- meeting-scoped `open_loop`

`unscored` does not override a scored sibling. If every candidate is missing or unscored, the result remains `Unscored`.

## 4. Backend Plan

Primary file:

- `src-tauri/src/services/briefing/schedule.rs`

No contract type changes are expected in `src-tauri/src/services/briefing_view_model.rs` or `src/types/briefing.ts`.

Implementation steps:

1. Add a private schedule-local helper that loads trust bands for the dashboard meetings before `map_meeting` runs.
2. Collect meeting IDs from the successful dashboard result.
3. In one DB read, query active claims for each meeting subject.
4. Prefer `meeting_readiness` over related fallback claim types.
5. Convert each selected claim's `trust_score` into `TrustBandWire`.
6. Populate `TrustMixin.trust_band` from the helper result.
7. Preserve `TrustBandWire::Unscored` as the default when the helper cannot find a scored source.

Selection policy:

- Preferred candidate: active `meeting_readiness` claim for the meeting subject.
- Fallback candidates: active related meeting-prep claims listed above.
- Band choice for multiple fallback candidates: most cautious scored band, with `needs_verification` more cautious than `use_with_caution`, and `use_with_caution` more cautious than `likely_current`.
- Missing `trust_score`, non-finite `trust_score`, query error, malformed subject, or no claim: `Unscored`.

Optional metadata: `trust_source_date` may use `source_asof`, then `observed_at`, then `created_at`; `trust_field_path` may use the selected claim's `field_path`; `rendered_provenance` can remain `None` unless a faithful source summary is already available.

## 5. Frontend Plan

Primary file:

- `src/pages/DailyBriefingRedesign.tsx`

Likely supporting file:

- `src/pages/DailyBriefingRedesign.module.css`

Implementation steps:

1. Import `TrustBandBadge` and its `TrustBand` type from `@/components/ui/TrustBandBadge`.
2. Import or reuse `TrustBandWire` from `@/types/briefing`.
3. Add a local adapter equivalent to the PredictionsSection adapter: `unscored` returns `null`; scored bands return the badge-safe type.
4. Extract a small `ScheduleMeetingRow` function inside `DailyBriefingRedesign.tsx`, or keep the logic inline if the row stays tiny.
5. Render the badge beside the meeting title or in the existing row meta area, compact if needed to preserve row density.
6. Keep the row accessible: the badge text already supplies non-color meaning; do not hide it with `aria-hidden`.
7. Use CSS module classes for any spacing/alignment change.

The UI must never pass `unscored` to `TrustBandBadge`, because the primitive only supports the three visible trust states. The meeting title and time remain visible for all bands, including `needs_verification`; this ticket is disclosure, not evidence suppression.

## 6. Backward Compatibility

- Existing serialized `ScheduleMeeting` rows with `trustBand: "unscored"` continue to render without a badge.
- Missing upstream claims keep the backend output stable as `Unscored`.
- The frontend handles the entire `TrustBandWire` union and does not assume that W4 data is fully scored.
- No existing predictions trust behavior changes.
- No route, contract, or Tauri command name changes.

## 7. Out of Scope

- Claim-lifecycle correction visualization is DOS-428.
- New trust-band primitive work; `TrustBandBadge` already exists.
- About-this/provenance detail panels.
- Evidence hiding/demotion policy for schedule rows.
- Changing `IntelligenceQualityView` labels or readiness computation.
- Reworking the schedule row into the full future meeting-card anatomy beyond the badge wire-in.

## 8. Test Plan

Backend tests in `src-tauri/src/services/briefing/schedule.rs`:

- `meeting_trust_defaults_to_unscored_without_claim`
- `meeting_trust_uses_meeting_readiness_claim_score`
- `meeting_trust_falls_back_to_related_meeting_claims`
- `meeting_trust_ignores_unscored_when_scored_related_claim_exists`
- `schedule_serializes_scored_trust_band_to_camel_case_wire_shape`

Frontend tests in `src/pages/DailyBriefingRedesign.test.tsx`:

- scored schedule meeting renders one `TrustBandBadge`
- `data-band` matches the meeting's scored trust band
- unscored schedule meeting omits `TrustBandBadge`
- all three scored variants render without passing `unscored` to the primitive

## 9. L1 Gates

- `cargo check --lib`
- `cargo clippy --lib -- -D warnings`
- `cargo test --lib services::briefing::schedule`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx src/components/dashboard/PredictionsSection.test.tsx`
- `pnpm lint`
- Manual code scan confirms:
  - no inline CSS
  - no new ephemeral ticket references in code comments
  - no `TrustBandBadge` call site receives `unscored`

## 10. L2 Gates

- Backend review: confirm `meeting_readiness` is the correct primary trust source and that fallback related claim selection does not mask missing readiness claims.
- Frontend review: confirm the badge placement is visible but does not dominate the compact schedule row.
- Contract review: confirm no `BriefingViewModel` or `ScheduleMeeting` shape change slipped in.
- Test review: confirm both non-`unscored` happy path and `unscored` backward-compatible path are covered.
