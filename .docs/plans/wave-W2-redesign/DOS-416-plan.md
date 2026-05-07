# DOS-416 - Email lift into briefing services - L0 plan

**Wave:** W2a - Services  
**Ticket:** DOS-416 - Move email ranking + selection out of view  
**Status:** L0 draft for review  
**Dependency:** DOS-413 contract; feeder for DOS-414 Moving

## 1. Acceptance criteria

- [ ] `DailyBriefing.tsx` no longer imports or calls `compareEmailRank`.
- [ ] Legacy email-selection rules move to Rust service code with parity tests:
  pinned first, relevance score descending, entity-linked only, score threshold
  `>= 0.15`, enriched-summary fill, and no raw recency fallback.
- [ ] No `BriefingViewModel.emails` slice is added.
- [ ] Email items fold into `BriefingViewModel.moving.entities[].signals[]` as
  `MovingSignalViewModel` rows with `kind: "email"`.
- [ ] `DailyBriefingRedesign.tsx` consumes email signals only through
  `model.moving.entities`; no email-specific branching or shaping.
- [ ] Every email Moving signal declares a trust source.
- [ ] No `/emails` surface changes land here; DOS-432 owns that.

## 2. Current state

- `src/components/dashboard/DailyBriefing.tsx` imports
  `@/lib/email-ranking`, builds `briefingEmails` inline, and passes those
  emails to the legacy Attention section.
- `src/lib/email-ranking.ts` contains the TS comparator.
- `src-tauri/src/services/emails.rs` already has Rust `compare_email_rank` and
  DB-backed enriched email assembly.
- `src/types/briefing.ts` has no email slice. The contract target is
  `MovingSignalViewModel`.
- `src-tauri/src/services/briefing/moving.rs` is currently an empty Moving
  branch; DOS-414 owns real aggregation.
- `src/pages/DailyBriefingRedesign.tsx` already renders `model.moving.entities`
  through `MovingRow`, so it should not gain email-specific logic.

## 3. Trust-source declaration

Email claims in DOS-416 are **not Glean-sourced**.

| Claim | Source of truth | Source date | Notes |
|---|---|---|---|
| Sender, subject, received timestamp, unread/thread metadata | Gmail metadata persisted in `emails` | `emails.received_at` or sync timestamp | Raw metadata. |
| Summary/context | Email enrichment over Gmail message/snippet, persisted as `emails.contextual_summary` | `emails.enriched_at` or `last_enrichment_at` | Subject-only fallback if absent. |
| Relevance score/reason | Local `signals::email_scoring` over Gmail content, entity link, meeting context, urgency, keywords, recency | `emails.updated_at` after `set_relevance_score` | Selection input. |
| Signal text/type/sentiment/urgency | `email_signals`, default `source = "email_enrichment"` | `email_signals.detected_at` | Preferred Moving signal text. |
| Entity ID/type | Entity-linking result persisted on `emails` and cascaded to `email_signals` | row `updated_at` or signal `detected_at` | User overrides are already reflected in DB. |
| Entity display name | Local `accounts`, `people`, `projects` tables | entity source if available | Row stats remain DOS-414-owned. |

Trust policy:

- W2 emits `trustBand: "unscored"` until DOS-427/DOS-320 provide scored trust.
- Each signal still sets `trustFieldPath`, `trustSourceDate` when known, and
  `renderedProvenance` naming `gmail`, `email_enrichment`, or `email_scoring`
  plus source email/signal IDs.
- Glean may only appear later if a future producer explicitly imports a
  Glean-sourced claim and records it.

## 4. Moving signal feed shape

DOS-416 produces email candidates. DOS-414 owns final Moving rows: entity
ranking, top 2-3 cap, ledes, state pills, provenance stats, and cross-source
ordering.

Internal feeder:

```rust
pub struct EmailMovingSignalCandidate {
    pub entity_id: String,
    pub entity_type: String,
    pub entity_name: Option<String>,
    pub movement_score: f64,
    pub occurred_at_iso: Option<String>,
    pub source_email_id: String,
    pub source_thread_id: Option<String>,
    pub signal: MovingSignalViewModel,
}
```

Signal row emitted to Moving:

```ts
{
  kind: "email",
  when: string,
  whatSegments: [{ text: string }, { text: string, emphasized: true }],
  urgency: "normal" | "overdue",
  threadAction?: { label: "Open email", href: "/emails" },
  trustBand: "unscored",
  trustFieldPath: "moving.entities[].signals[].whatSegments",
  trustSourceDate?: string,
  renderedProvenance?: RenderedProvenanceSummary
}
```

Selection rules:

- Only active, non-noise, entity-linked emails can become candidates.
- Sort pinned first, then descending `relevance_score`, null scores last.
- Primary set requires `relevance_score >= 0.15`.
- Fill remaining budget with enriched emails that have non-empty summaries.
- No raw recent-email fallback.
- The feeder may return more than the legacy 5-item UI cap so DOS-414 can rank
  email against other signal kinds. The selector helper should still have a
  5-cap parity test.

## 5. Files this lands

Rust:

- `src-tauri/src/services/briefing/email_signals.rs` - new feeder returning
  `EmailMovingSignalCandidate`.
- `src-tauri/src/services/briefing/mod.rs` - register the module.
- `src-tauri/src/services/emails.rs` - extract the briefing-email selector
  helper; keep `/emails` response shape unchanged.
- `src-tauri/src/services/briefing/moving.rs` - integration seam only. If
  DOS-414 is not merged, export the feeder and tests without changing the empty
  Moving branch.
- Unit tests in touched Rust modules.

TypeScript:

- `src/components/dashboard/DailyBriefing.tsx` - remove `compareEmailRank`
  import and inline email ranking/selection.
- `src/lib/email-ranking.ts` - delete if `rg compareEmailRank src` has no
  remaining callers.
- `src/components/dashboard/DailyBriefing.test.tsx` - update legacy assertions
  that assumed the view sorted emails.
- `src/pages/DailyBriefingRedesign.tsx` - expected no-op; verify consumption via
  `MovingRow` only.

No CSS files are touched. No inline CSS is allowed.

## 6. Coordination with DOS-414

- DOS-416 is a feeder; DOS-414 is the Moving aggregator.
- Handoff key: `entity_id` plus `entity_type`.
- Handoff payload: `EmailMovingSignalCandidate`.
- DOS-414 owns `MovingViewModel.entities`, service caps, row ledes, state pills,
  provenance stats, and final ranking.
- DOS-416 should add one narrow import/call once the DOS-414 shell exists, not
  rewrite ranking logic.
- DOS-419 lifecycle signals stay independent.

Preferred order: DOS-414 aggregation shell first, DOS-416 email feeder second.
If DOS-416 lands first, DOS-414 wires the exported feeder during integration.

## 7. Display-layer purity

- `DailyBriefingRedesign.tsx` has no email-specific branch.
- `DailyBriefingRedesign.tsx` has no `.filter`, `.sort`, or `.reduce` over
  email data.
- `MovingRow` remains generic over `MovingEntityViewModel`.
- Legacy `DailyBriefing.tsx` no longer owns email ranking after this ticket.
- Allowed view logic remains load-state branching, generic mapping over
  `model.moving.entities`, and navigation to supplied hrefs.

## 8. Mutation verification

No new mutation command is introduced. `update_email_entity` stays owned by
`/emails`; action and lifecycle mutations are unrelated. Email `threadAction`
is a route affordance only and points to `/emails` until DOS-432 defines deeper
email routing.

## 9. Out of scope

- No `/emails` surface uplift or raw inbox redesign.
- No `BriefingViewModel.emails` slice.
- No Glean email producer.
- No new trust scoring model; W2 stays `unscored` with provenance.
- No lifecycle-change mapping; DOS-419 owns that.
- No SignalDot, MovingRow, CSS, routed `/` cutover, or per-thread route changes.

## 10. L1 gates

Rust:

- `cargo test --lib services::briefing::email_signals`
- `cargo test --lib services::emails`
- `cargo check --lib`
- `cargo clippy --lib -- -D warnings`

TypeScript:

- `pnpm tsc --noEmit`
- `pnpm test src/components/dashboard/DailyBriefing.test.tsx`
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx`

Search/audit:

- `rg "compareEmailRank|@/lib/email-ranking" src` returns no callers.
- `rg "briefingEmails|emailSectionLabel" src/components/dashboard/DailyBriefing.tsx`
  returns no lifted selection path.
- `rg "style=|<style" src/components/dashboard/DailyBriefing.tsx src/pages/DailyBriefingRedesign.tsx`
  returns no new inline CSS.
- Rust fixture proves an enriched, entity-linked email serializes to
  `kind: "email"` with `trustBand: "unscored"`, source date, and provenance.

## 11. L2 gates

Reviewers:

- `/codex review`
- `code-reviewer` subagent for selection parity and legacy regression risk
- Backend/architect reviewer for trust-source correctness and DOS-414 boundary

Must explicitly approve: no email slice, correct Gmail/email-enrichment/email-
scoring provenance, DOS-414-owned final Moving ranking, pass-through
`DailyBriefingRedesign.tsx`, and unchanged `/emails` behavior.

## 12. Risk and rollback

Risks: over-filtering makes Moving quiet; under-filtering leaks inbox noise;
AI-enriched text lacks clear source; `moving.rs` conflicts with DOS-414.

Rollback: remove feeder registration and any `moving.rs` call site; leave the
pure Rust selector if unused and tested; do not restore email ranking to
`DailyBriefingRedesign.tsx`.

## 13. Implementation notes

- Added `services::briefing::email_signals` as the Moving email feeder. It reads
  active DB emails, applies the lifted briefing selector, prefers
  `email_signals` text when present, and emits `MovingSignalViewModel` rows with
  `kind = email`, `trustBand = unscored`, source dates, and rendered provenance.
- Replaced the `collect_email_signals` Moving stub with the feeder mapping only;
  email signals remain claimless today.
- Moved legacy briefing email selection into `services::emails` with parity
  coverage for pinned-first ordering, relevance threshold, entity-linked-only
  filtering, enriched-summary fill, five-item cap, and no raw fallback.
- Updated live dashboard email assembly to hand `DailyBriefing.tsx` the
  service-selected email set. `DailyBriefing.tsx` no longer imports or calls
  `compareEmailRank`, and the email Attention section is now pass-through.
- Left `/emails` comparator usage unchanged to preserve that surface boundary.
