# Lane A — L0 Packet: Surface-Relevant Meetings Projection

**Date:** 2026-05-26 (revised after L0 cycle-1)
**Status:** Cycle-2 draft (cycle-1 verdicts: K-in CITES_REQUIRED, Feasibility REQUEST_CHANGES, codex challenge no-verdict)
**Branch:** TBD (`fix/v1.4.8-lane-a-meetings-projection` proposed)
**Ticket:** [DOS-774](https://linear.app/a8c/issue/DOS-774) — closes [DOS-771](https://linear.app/a8c/issue/DOS-771)
**Wave:** v1.4.8 Meetings Substrate v2 ([DOS-773](https://linear.app/a8c/issue/DOS-773) parent)

**Cycle-1 → cycle-2 changes:**
- §3.4 consumer table corrected — cycle-1 feasibility caught that only 2 of 4 entries actually call the readiness handle. Adopted **path (a) — narrow scope**: Lane A migrates only the 2 real readiness-handle callers. The dashboard `compute_focus_capacity` path and the entities calendar-merge path are file as follow-up work (see §6 Follow-up scope). Keeps Lane A at Standard tier; closes DOS-771 cleanly; deliberately leaves a known policy-drift gap that a follow-up ticket addresses.
- §2 K-in ADR citations added — 0102 boundary heuristic, 0101 Rule 5, 0111 below-bridge note.
- AC #3 simplified to match the 2-consumer scope.

---

## §0 — Origination, scope, threat topology

- **Origination class:** Debug-driven. DOS-771 is the forcing function; the architectural reshape is built around the root-cause trace, not retrofitted to it.
- **Scope tier:** Standard (multi-file, single domain — meetings substrate read path. Trait signature changes ripple to ~6 call sites and test fixtures; no ADR-named contract change; no cross-domain reshape).
- **Threat topology:** local-to-local single-user. Tauri runtime reading workspace-private meeting data, scoped to the configured workspace. No multi-actor gates, no remote surface, no MCP scope expansion in this lane.

### Symptom-to-failure trace (mandatory for debug-driven)

**1. User-visible symptom verbatim** (chat 2026-05-25):

> "it's saying 4 meetings need prep but there are no meetings today. i'm making an assumption this is for later in the week? however we're showing this on the 'today' page so it should be focused on the day, not the future"

Followed by (2026-05-25, after PR #392 round 4 fix attempts):

> "i also have no meetings today. i have one that was cancelled and that's shown up sometimes but not always. currently it is not showing up at all."

Rendered state on the today page (pre-PR #392):
- `state.freshness.NeedsPreparation { meeting_ids: [4 ids] }` → "4 BRIEFINGS NEED PREP"
- `state.advisories[0] = UnlinkedMeetings { meeting_ids: [4 ids] }` → "Link 4 meetings for fuller context"
- Focus capacity panel correctly says "0 meetings today" — two views of the same day disagree.

**2. Call path (surface → substrate):**

| Hop | File:line |
|---|---|
| React component renders strip | `src/components/dashboard/DailyBriefing.tsx:485` (pre-PR #392) |
| Hook fetches via Tauri invoke | `src/hooks/useDailyBriefingAbility.ts:86` |
| Tauri ability bridge | `src-tauri/src/commands/abilities.rs:26 invoke_ability` |
| Briefing ability producer | `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:69` |
| Service context wrapper | `src-tauri/abilities-runtime/src/services/context.rs:2308 read_daily_readiness_context` |
| Live reader (TZ resolution) | `src-tauri/src/services/context.rs:792 LiveDailyReadinessContextReader::read_daily_readiness_context` |
| SQL projection (failure point) | `src-tauri/src/services/context.rs:856-866 project_daily_readiness_context_snapshot` |

**3. Suspected failure point** (verified via Codex independent trace + cross-reference, not architectural intuition):

`project_daily_readiness_context_snapshot` SQL at `services/context.rs:856-866` filters by TZ-aware UTC range and excludes `meeting_transcripts.intelligence_state = 'archived'`, but applies **no `meeting_type` filter and no all-day filter**.

The canonical "what counts as a surfaceable meeting" filter lives at `src-tauri/src/focus_capacity.rs:169-179 should_exclude_meeting`:

```rust
fn should_exclude_meeting(meeting: &Meeting) -> bool {
    if meeting.meeting_type == MeetingType::Personal { return true; }
    if meeting.overlay_status == Some(OverlayStatus::Cancelled) { return true; }
    let is_all_day = meeting.time.len() == 10 && meeting.time.chars().nth(4) == Some('-');
    is_all_day
}
```

The 4 phantom advisories are James's personal calendar blocks (focus time, lunch, OOO — anything `MeetingType::Personal` per `google_api/classify.rs` rule 2: "0-1 attendees"). They're real Google Calendar events that pass every existing readiness filter but the focus-capacity filter (and the frontend filter at `src/components/dashboard/DailyBriefing.tsx:186 scheduleMeetings`) correctly excludes them. The readiness substrate is the only consumer that doesn't.

**4. Hypotheses explored and rejected** (so reviewers don't re-walk them):

| Hypothesis | Rejected because |
|---|---|
| H1 (from DOS-771): readiness returns rows with `intelligence_state IS NULL` and the cancellation filter passes them through | Refuted — personal blocks pass the filter even when transcripts exist with non-archived state |
| H2 (from DOS-771): producer branches past `empty_no_meetings` | Refuted — codex independent trace confirmed no alt constructor of `NeedsPreparation`/`UnlinkedMeetings` exists outside the producer's non-empty path |
| H3 (from DOS-771): stale rows with `calendar_event_id IS NULL` leak through | Refuted — personal blocks HAVE `calendar_event_id` set (they're real GCal events) |
| Cycle 3 fix (TZ-aware UTC range): the leak is yesterday-evening meetings due to UTC-naive date compare | Partial — fix is correct and lands, but didn't move the count because personal blocks ARE in today's range |
| Cycle 4 fix (`LEFT JOIN meeting_transcripts` excluding archived): cancelled meetings persisting | Partial — fix is correct and lands, but personal blocks pass the `IS NULL OR != 'archived'` clause |

**Confidence: 9/10.** Remaining uncertainty: whether the 4 are pure personal blocks or mixed personal-plus-all-day. Pinned at 10/10 once a `tracing::info!` diagnostic line confirms (planned as first move at L1, not a packet blocker).

---

## §1 — Mission

Promote the "what counts as a surfaceable meeting" policy from scattered surface-side filters into a single substrate-level projection that the abilities-runtime narrow read handle consumes via declared intent. Closes DOS-771 as part of the same PR; substrate change and visible payoff (strip revival) ship together.

---

## §2 — K-in sources (substrate-grep + diagnostic-grep)

### Substrate-grep targets (mandatory L0 reviewer pass)

`ce-learnings-researcher` runs parallel-grep across:

- `docs/solutions/architecture-patterns/` — surface any prior fix for "scattered filter policy" / "consumer-derived projection" class.
- `docs/solutions/workflow-issues/` — surface any prior diagnostic entry for "phantom advisory counts" / "filter drift between consumers."
- `.docs/decisions/` — ADRs likely relevant:
  - ADR-0101 (services as the only mutation surface)
  - ADR-0102 (abilities as runtime contract)
  - ADR-0105 (provenance as first-class output)
  - ADR-0125 (claim anatomy, temporal/sensitivity registries)
  - ADR-0129 (composable surfaces / WordPress Studio as primary surface) — relevant because the substrate change must not silently shift contract for WP block consumers.

If any of the above cite a prior projection layer or read-handle intent pattern, BLOCK and cite the path — Lane A is reinventing it.

### Diagnostic-grep targets (debug-driven packet requirement)

- `docs/solutions/workflow-issues/` for the symptom "phantom meetings on briefing." If prior diagnostic exists and Lane A's §3 contradicts it: `wrong cure` verdict.

### ADR citations (from cycle-1 K-in)

The K-in reviewer surfaced three ADRs that need explicit citation in this packet, with the specific rule each touches:

- **[ADR-0102](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0102-abilities-as-runtime-contract.md) — Abilities as runtime contract.** Boundary heuristic: "if the output is a single read or a trivial projection, it is not an ability." `read_surface_meetings` is exactly that — a substrate helper *below* the abilities layer, consumed by ability producers. Cite to justify why this is a service-layer projection and not a new ability.
- **[ADR-0101](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0101-service-boundary-enforcement.md) — Service boundary enforcement.** Rule 5: "functions named `get_*`, `list_*`, `load_*`, `build_*`, `read_*` must not write." `read_surface_meetings` is `read_*`-shaped and read-only by construction.
- **[ADR-0111](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation.** The `DailyReadinessContextReadHandle` trait sits *below* the ADR-0111 bridge contract (it's the producer's narrow read handle, not the bridge). Adding an `intent` param does not violate ADR-0111; the change is internal to producer/handle interaction, not surface↔ability dispatch.

### Prior-work background

- [DOS-258](https://linear.app/a8c/issue/DOS-258) — entity linking deterministic-engine rewrite. Same architectural family (substrate-owned policy, deterministic contract, adapter pattern). Lane A is the read-side equivalent of what DOS-258 did for entity linkage; Lane B mirrors its write-side pattern more directly.
- [DOS-771](https://linear.app/a8c/issue/DOS-771) — root cause writeup with ranked hypotheses + diagnostic step. Lane A's §3 implementation directly intersects the failure point at `services/context.rs:856-866`.

### Scattered-filter-policy gap (K-in flagged for K-out follow-up)

ADR-0033, ADR-0061, ADR-0081 define meeting identity and lifecycle but **do not name a single canonical "what counts as surfaceable" policy** — the K-in reviewer confirmed this is genuinely undocumented in ADR space, not reinvention. Lane A's success closes that gap; the L3 retro K-out should file a `docs/solutions/architecture-patterns/` entry naming the "consumer-derived projection → substrate-owned narrow read handle with declared intent" pattern so future similar fan-out drift has documented precedent.

---

## §3 — Implementation / design direction

### 3.1 New substrate type — `MeetingsViewIntent`

```rust
// src-tauri/src/services/meetings_view.rs (new file)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingsViewIntent {
    /// Meetings worth surfacing on the briefing — excludes personal blocks,
    /// cancelled meetings, all-day events. Matches focus_capacity policy.
    Briefing,
    /// Schedule rendering — same exclusions as Briefing today; reserved
    /// as a distinct variant in case schedule wants to differ later.
    Schedule,
    /// Raw rows for analytics / diagnostic surfaces that need full visibility.
    /// No filters beyond TZ range + archived-transcript exclusion.
    AllRows,
}
```

**Why not collapse `Briefing` and `Schedule`:** they have identical filter sets today but their consumers diverge in intent. Keeping them distinct prevents a future "briefing wants to surface OOO blocks as 'unavailable' affordances but schedule doesn't" change from rippling through every callsite. Memory of the renumber pattern — pay the small enum cost now, avoid the migration later.

### 3.2 New substrate function — `read_surface_meetings`

```rust
// src-tauri/src/services/meetings_view.rs
pub fn read_surface_meetings(
    db: &ActionDb,
    workspace_scope: &str,
    date: NaiveDate,
    tz: &chrono_tz::Tz,
    intent: MeetingsViewIntent,
) -> Result<Vec<SurfaceMeeting>, String>
```

- Owns the TZ-aware UTC range computation (today's `project_daily_readiness_context_snapshot`).
- Owns the per-intent filter set (today's `should_exclude_meeting` policy, lifted into the substrate).
- Returns a typed `SurfaceMeeting` projection (lift of `DailyReadinessMeetingSnapshot`).
- File location: **new file `src-tauri/src/services/meetings_view.rs`**, per codex grounding. `context.rs` is the abilities-runtime adapter glue; `services/entity_linking/` precedent (DOS-258) is for write-side adapters not read-side projections, so a separate `meetings_view.rs` matches the project's existing categorization better than nesting inside either.

### 3.3 Trait migration — `DailyReadinessContextReadHandle`

```rust
// src-tauri/abilities-runtime/src/services/context.rs
pub trait DailyReadinessContextReadHandle: Send + Sync {
    fn read_daily_readiness_context<'a>(
        &'a self,
        workspace_scope: String,
        date: String,
        intent: MeetingsViewIntent,  // ← new param
    ) -> DailyReadinessContextReadFuture<'a>;
}
```

Live impl threads intent into `read_surface_meetings`. Test fixtures accept and ignore.

### 3.4 Consumer migrations — corrected after cycle-1 feasibility finding

Cycle-1 feasibility caught that my original §3.4 table conflated 4 different code paths into one. Only **2 production callers** actually invoke `read_daily_readiness_context` today. Cycle-2 adopts **path (a) — narrow scope**: Lane A migrates only the 2 real readiness-handle callers; the dashboard and entities paths file as follow-up work (§6). Rationale: keep Lane A at Standard tier, close DOS-771 cleanly, accept the residual policy-drift gap as a known issue with a tracked follow-up. The wave's "policy lives in exactly one place" mission completes when the follow-up lands, not when Lane A merges.

**Consumer migrations (2 real readiness-handle callers):**

| Caller | File:line | Declared intent | Justification |
|---|---|---|---|
| `get_daily_briefing` ability | `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:69` | `Briefing` | Surfacing meetings worth prepping on the today page — excludes personal blocks (the DOS-771 fix). |
| `get_daily_readiness` ability | `src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:1025` | `Briefing` | Per cycle-1 feasibility audit: synthesis maps every returned meeting into `prepare_meeting_children` at `synthesis.rs:1047-1050`. Personal blocks shouldn't generate prep children. Same policy as briefing. |

The trait signature change (§3.3 — adding `intent: MeetingsViewIntent`) ripples to: live impl at `src-tauri/src/services/context.rs:791`, 2 test fixtures (`tests/w5_a_get_daily_readiness_test.rs:95`, `producer.rs:967`), wrapper at `abilities-runtime/src/services/context.rs:2301`. Confirmed clean and tractable by cycle-1 feasibility.

**Out of scope (filed as follow-up, see §6):**

- `compute_focus_capacity` callers at `dashboard.rs:532, :1121` (bespoke SQL → `compute_focus_capacity(meetings, …)` with inline cancelled/personal filtering at `:502-509`). Today's filter logic stays in `should_exclude_meeting` at `focus_capacity.rs:169-179`.
- Calendar-merge consumer at `entities.rs:319-396` (bespoke SQL → `calendar_merge::merge_meetings`). Today's path stays.

After Lane A merges, `should_exclude_meeting` continues to exist at `focus_capacity.rs:169-179` for the dashboard/entities path, AND its semantic twin lives in `services/meetings_view.rs` (the per-intent filter set inside `read_surface_meetings`) for the readiness path. Two implementations of one policy; the follow-up ticket consolidates.

**`record_cancelled_calendar_meetings`** (`src-tauri/src/services/meetings.rs:99-103`) stays as-is — write-path reconciliation function, not a read-view consumer. Out of scope.

### 3.5 Frontend — strip revival + defensive filter resolution

- Restore `DailyBriefingAbilityStrip` and its helpers (`briefingFreshnessLabel`, `briefingAdvisoryLabel`, `renderedProvenanceSourceCount`, `useDailyBriefingAbility` import) in `src/components/dashboard/DailyBriefing.tsx`. The component logic is preserved in PR #392's parent commit; restore from there rather than re-author.
- Restore the test for strip rendering in `DailyBriefing.test.tsx` (the "renders ability-backed briefing trust and provenance state" test removed in PR #392).
- Resolve the frontend defensive filter at `src/components/dashboard/DailyBriefing.tsx:186 scheduleMeetings`:
  - **Keep it (defensive belt-and-braces)** — DRY violation, but cheap safety net.
  - **Remove it (substrate owns the policy)** — cleaner, but a future buggy substrate change would surface immediately in the UI with no frontend safety net.
  - **Decision:** keep, with a comment pointing to `MeetingsViewIntent::Briefing` as the authoritative policy. Removing it earns nothing material; keeping it costs one filter call per render.

---

## §4 — Acceptance criteria

1. `MeetingsViewIntent` enum + `read_surface_meetings` function shipped in `src-tauri/src/services/meetings_view.rs`.
2. `DailyReadinessContextReadHandle::read_daily_readiness_context` signature carries `intent`. Live impl + 2 test fixtures + abilities-runtime wrapper updated.
3. The 2 readiness-handle callers (briefing + readiness abilities) consume the new projection with declared intent.
4. `DailyBriefingAbilityStrip` restored in `DailyBriefing.tsx`; strip test restored.
5. **DOS-771 closes** — verified live: today page on a 0-meeting calendar day renders **without** "BRIEFINGS NEED PREP" / "Link N meetings" advisories, and the strip shows the correct "Current" freshness label.
6. Regression test: a fixture covering a day with `MeetingType::Personal` rows verifies they're excluded under `MeetingsViewIntent::Briefing` and included under `MeetingsViewIntent::AllRows`.
7. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` clean.
8. **L4 before L2** (per ladder): user-facing change is the strip revival; L4 hands-on validation on the today page before L2 dispatch.
9. Follow-up ticket filed for the dashboard/entities migration (§6) so the wave's "policy lives in one place" mission has a tracked path to completion.

---

## §5 — Risks + open issues for reviewers

### Risks

- **Residual policy-drift gap after Lane A.** §3.4 narrow scope leaves `should_exclude_meeting` in `focus_capacity.rs` AND its semantic twin in `services/meetings_view.rs`. Two implementations of one policy until the §6 follow-up lands. Risk: a future filter change to one and not the other reintroduces the DOS-771 class of bug for the dashboard/entities surfaces. Mitigation: the follow-up ticket is the closing move; until it lands, any change to `should_exclude_meeting` requires touching both sites.
- **Trait signature ripple to test fixtures.** Cycle-1 feasibility confirmed 3 impls (live + 2 fixtures) + 1 wrapper. Tractable but requires care.
- **`get_daily_readiness` intent mis-classification.** §3.4 assigns `Briefing`. Cycle-1 feasibility verified at `synthesis.rs:1047-1050` that personal blocks shouldn't generate prep children — assignment is correct. If a future readiness consumer wants personal-block visibility, it requires a new intent variant.
- **Strip design refinements.** Strip revival as-is restores PR #392's parent state. If James wants visual/copy refinements (e.g., suppress "Trust: unscored" on no-meeting days), flag for `ce-design-lens-reviewer` before L1 starts.

### Open questions for reviewers

1. **Frontend defensive filter** — keep at `DailyBriefing.tsx:186` or remove? §3.5 recommends keep. Reviewers may push back.
2. **`Briefing` vs `Schedule` distinction** — collapse to one variant (today they have identical filters)? §3.1 recommends keep distinct for future divergence; reviewers may prefer the YAGNI cut.
3. **`AllRows` consumer audit** — `services/entities.rs:396` may want `Briefing` not `AllRows`. L0 reviewer with feasibility lens can validate by reading that call site.

---

## §6 — Sequencing + migration + follow-up scope

**Follow-up ticket (file at L1 commit time):** "Migrate `compute_focus_capacity` + `executive_intelligence` paths to `read_surface_meetings`." Targets `dashboard.rs:430-470, :502-509, :532, :1121` and `entities.rs:319-396`. Adds `Meeting ↔ SurfaceMeeting` conversion (helper / re-export / unification — L1 of that ticket picks). Closes the wave's "policy lives in one place" mission; until then, `focus_capacity.rs:169-179` `should_exclude_meeting` and `services/meetings_view.rs` per-intent filter set are semantic twins that any policy change must touch together.

- **Single PR.** Substrate + consumer migrations + strip revival in one PR. Vertical-slice principle (per the wave plan's pacing callout): substrate without a consumer risks being designed wrong.
- **Pre-L1 step:** add `tracing::info!` diagnostic line at producer.rs:88 logging `(meeting.id, meeting.title, meeting.starts_at, meeting.meeting_type)`. Run with current James calendar (0 meetings, 4 phantoms). Confirms the personal-blocks-leak theory at 10/10 before the substrate change ships. Remove before merge.
- **L4 before L2.** Strip revival is user-facing; per ladder, L4 hands-on validation on today page lands before L2 dispatch.
- **L2 router-selected reviewers** (preview, finalized at L2): `pr-review-toolkit:type-design-analyzer` (new enum + new function signature + trait migration) + `ce-api-contract-reviewer` (trait change is a substrate API contract). Advisory parallel as usual; findings to maintenance.
- **No migrations or schema changes** in this lane. No `intelligence_state` lifecycle redesign. No `calendar_merge` change.
- **Closes DOS-771** at merge. Linear comment on DOS-771 with the verified-live evidence.

---

## §7 — L0 review composition

Per the engineering ladder, Standard tier L0:

- **Required (default 2):**
  - `/codex challenge` — adversarial.
  - One planning reviewer routed by what the plan touches: **`ce-feasibility-reviewer`** — does the trait migration survive contact with all 4 consumer sites? Does the intent enum carve cleanly?
- **K-in (mandatory, parallel-grep):** `ce-learnings-researcher` — cite hits from §2 substrate-grep + diagnostic-grep targets.
- **Symptom-fit prompt (mandatory because debug-driven):** at least one reviewer must answer: *Does §3 directly move the user-visible symptom in §0?* — citation required, otherwise `wrong cure` verdict.

Conditional second planning reviewer (`ce-scope-guardian-reviewer`) is NOT added by default at Standard tier. If the L0 cycle surfaces scope-creep risk (e.g., a reviewer pushes to expand the enum to N variants), tier-up to Wave and add scope guardian.

No design lens needed: strip revival restores a removed component, no new design.
No security lens needed: local-to-local single-user, no new trust boundary.

---

## §8 — What "done" looks like (L4 hands-on, before L2)

On the today page in a 0-meeting calendar day (James's current state):
- **Pre-Lane-A baseline (post PR #392):** No strip rendered. Hero says "A clear day. Nothing needs you." No phantom advisories — but no self-assessment metadata either.
- **Post-Lane-A target:** Strip rendered. Freshness label says "Current". Trust band reads correctly (likely "unscored" with 0 source claims). No advisory CTA (because no unlinked meetings). Source count matches actual provenance.

On a day WITH meetings (any other day):
- Strip rendered with accurate freshness, trust, source count, and (if applicable) advisory CTA pointing at real unlinked meetings.

If the strip still shows phantom counts on a 0-meeting day: Lane A failed; do not L2-dispatch; trace deeper.
