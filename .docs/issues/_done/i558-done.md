# I558 — Meeting Detail Intelligence Expansion

**Version:** v1.0.0 Phase 4
**Depends on:** I555 (interaction dynamics persistence), I554 (enriched transcript extraction)
**Type:** Enhancement — frontend intelligence surfacing
**Scope:** Frontend + backend (new Tauri command for dynamics data). Meeting detail page only.

---

## Problem

The Meeting Detail page renders a curated editorial briefing but drops significant intelligence on the floor:

### 1. Post-meeting intelligence gap

After a meeting, the transcript extraction produces:
- Discussion topics with decisions/commitments (DISCUSSION block)
- Interaction dynamics (talk balance, speaker sentiment, engagement signals)
- Champion health assessment
- Stakeholder role changes
- Sentiment analysis with 12+ dimensions
- Categorized wins/risks with urgency levels and verbatim quotes

None of this post-meeting intelligence is surfaced on the meeting detail page. The only post-meeting content shown is the Outcomes section (summary + flat wins/risks/decisions/actions from `MeetingOutcomeData`). The richer data from I554/I555 sits in the database unseen.

### 2. Unused FullMeetingPrep fields

The `FullMeetingPrep` type has fields that are loaded but never rendered:

| Field | Content | Why Useful |
|-------|---------|-----------|
| `sinceLast[]` | What changed since last meeting with this account | Critical for meeting prep — "here's what happened since you last spoke" |
| `currentState` | What's working / not working | Context for the conversation |
| `recentWins[]` | Recent account wins | Talking points for the meeting |
| `strategicPrograms[]` | Active strategic programs | Context for discussing priorities |
| `openItems[]` | Open items from previous meetings | Follow-up tracking |

These fields exist in the `hasAnyContent` check but have no rendering section.

### 3. Outcomes section is too flat

The current Outcomes section shows wins/risks/decisions as plain text lists. After I554/I555, these items have sub-types, urgency levels, and verbatim quotes — but the renderer doesn't display them.

---

## Solution

### Act 0 — Post-Meeting Intelligence (new, replaces Outcomes when transcript exists)

When a meeting has a processed transcript, show a rich post-meeting intelligence section instead of the flat Outcomes block:

**Meeting Summary** — existing `outcomes.summary` but in editorial serif treatment

**Discussion Topics** — from `meeting_transcripts.summary` (the DISCUSSION items)
- Each topic as a serif heading with its outcome/decision as body text
- Decisions get a distinct sage highlight

**Engagement Dynamics** — from `meeting_interaction_dynamics`
- Talk balance bar: visual bar showing customer vs internal talk ratio
- Per-speaker sentiment: name + sentiment badge + evidence quote
- Engagement signals: compact indicator strip (question density, decision-maker active, forward-looking, monologue risk)
- Only shown when interaction dynamics exist for this meeting

**Champion Health** — from `meeting_champion_health`
- Champion name + status badge (strong/weak/lost/none)
- Evidence text (what they said/did that indicates their health)
- Risk text when status is weak or lost
- Only shown when champion health data exists

**Categorized Outcomes** — enriched wins/risks/decisions from `captures` with metadata
- Wins grouped by sub-type (ADOPTION, EXPANSION, VALUE_REALIZED, etc.) with impact badges
- Risks sorted by urgency (RED first, then YELLOW, then GREEN_WATCH) with urgency badges and evidence quotes
- Decisions with commitment type and owner attribution
- Actions (existing ActionRow rendering)

**Role Changes** — from `meeting_role_changes`
- Person name → old status → new status, with evidence quote
- Only shown when role changes detected

**Sentiment Deep Dive** — from `captures` WHERE `capture_type = 'sentiment'`
- Visual indicator strip: overall sentiment, customer sentiment, engagement level
- Expanded indicators: ownership language, forward-looking, competitor mentions, internal advocacy, data export interest
- Collapsed by default; expandable for detail

### Act I enrichment — "Ground Me" additions

**Since Last Meeting** — from `data.sinceLast[]`
- Rendered as a compact timeline between the headline and the key insight blockquote
- Shows what happened since the previous meeting with this account/entity
- Only shown when non-empty

**Account Pulse** — from `data.currentState`
- Two-column compact display (working / not working) below the entity chips
- Lighter treatment than the full State of Play on Account Detail — just the headlines
- Only shown when currentState exists in prep data

### Act II enrichment — after risks

**Recent Wins** — from `data.recentWins[]`
- Sage-accented items after the risks section
- Provides positive context to balance the risk focus

**Open Items** — from `data.openItems[]`
- Compact list of unresolved items from previous meetings
- Important for continuity — "don't forget to follow up on..."

### Backend: New Tauri command

```rust
#[tauri::command]
pub async fn get_meeting_post_intelligence(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingPostIntelligence, String>
```

Returns:
```rust
pub struct MeetingPostIntelligence {
    pub interaction_dynamics: Option<InteractionDynamics>,
    pub champion_health: Option<ChampionHealthAssessment>,
    pub role_changes: Vec<RoleChange>,
    pub enriched_captures: Vec<EnrichedCapture>,  // captures with sub_type, urgency, evidence_quote
    pub sentiment_detail: Option<TranscriptSentiment>,
}

pub struct EnrichedCapture {
    pub id: String,
    pub capture_type: String,
    pub content: String,
    pub sub_type: Option<String>,
    pub urgency: Option<String>,
    pub impact: Option<String>,
    pub evidence_quote: Option<String>,
    pub speaker: Option<String>,
    pub owner: Option<String>,
}
```

This command is called when the meeting detail page loads and a transcript has been processed (`intel.outcomes` exists or `meeting_transcripts.transcript_processed_at` is set).

---

## Files

| File | Changes |
|------|---------|
| `src/pages/MeetingDetailPage.tsx` | Add post-meeting intelligence section (Act 0). Render sinceLast, currentState, recentWins, openItems in appropriate acts. Call `get_meeting_post_intelligence`. |
| `src/pages/MeetingDetailPage.module.css` | Styles for talk balance bar, speaker sentiment cards, engagement indicator strip, champion health badge, urgency-sorted outcomes. |
| `src/components/meeting/PostMeetingIntelligence.tsx` | New component: engagement dynamics + champion health + categorized outcomes + role changes + sentiment deep dive |
| `src/components/meeting/PostMeetingIntelligence.module.css` | Styles |
| `src/components/meeting/TalkBalanceBar.tsx` | New component: visual bar showing customer/internal talk ratio |
| `src/components/meeting/EngagementStrip.tsx` | New component: compact engagement signal indicators |
| `src-tauri/src/commands/meetings.rs` | Add `get_meeting_post_intelligence` command |
| `src-tauri/src/db/meetings.rs` | Add queries for interaction_dynamics, champion_health, role_changes per meeting. Add enriched captures query. |
| `src/types/index.ts` | Add `MeetingPostIntelligence`, `InteractionDynamics`, `ChampionHealthAssessment`, `RoleChange`, `EnrichedCapture` types |

---

## Design

**Pre-meeting vs post-meeting state**: The page should feel different depending on whether the meeting has happened yet. Before the meeting, Acts I-III (Ground Me, Brief Me, the Room, Your Plan) dominate. After the meeting, Act 0 (Post-Meeting Intelligence) takes prominence — it slides above the briefing content as the new lead section.

**Engagement Dynamics visual treatment**:
- Talk balance: horizontal bar, customer% in larkspur, internal% in sage, divider line at 50%
- Speaker sentiment: compact cards with name, colored sentiment dot (green/amber/red/gray), evidence quote in italic serif
- Engagement signals: icon + label strip (similar to VitalsStrip compact treatment)

**Champion Health visual treatment**:
- Strong: sage accent, shield icon
- Weak: turmeric accent, warning icon
- Lost: terracotta accent, alert icon
- None: neutral gray, question icon

**Urgency-sorted outcomes**:
- RED risks: terracotta left border, bold urgency label
- YELLOW risks: turmeric left border
- GREEN_WATCH: sage left border (it's an early warning, not a crisis)
- Evidence quotes in italic serif, indented

---

## Out of Scope

- Changes to the transcript extraction prompt (I554)
- Schema changes (I555)
- Report pipeline changes (I556)
- Account detail page changes (I557)
- Changes to the pre-meeting briefing assembly (MeetingPrepQueue)

---

## Acceptance Criteria

1. Meeting with processed transcript shows "Post-Meeting Intelligence" section above the pre-meeting briefing content.
2. Engagement dynamics section shows talk balance bar and per-speaker sentiment cards when `meeting_interaction_dynamics` data exists.
3. Champion health badge shows with status (strong/weak/lost/none), name, and evidence when `meeting_champion_health` data exists.
4. Wins are grouped by sub-type with impact badges. Risks are sorted by urgency (RED first) with urgency badges and evidence quotes.
5. Role changes section appears when `meeting_role_changes` has entries for this meeting.
6. Sentiment deep dive is collapsed by default, expandable on click.
7. `data.sinceLast[]` renders as a compact timeline in Act I when non-empty.
8. `data.currentState` renders as a compact two-column pulse display when present.
9. `data.recentWins[]` renders after risks in Act II when non-empty.
10. `data.openItems[]` renders as a follow-up list when non-empty.
11. `get_meeting_post_intelligence` Tauri command returns correct data. Returns empty/null fields gracefully when no transcript has been processed.
12. Meetings without transcripts show the existing page layout — no post-meeting sections, no empty containers.
13. All new components use CSS modules. Zero inline styles. Zero ADR-0083 violations.
14. Talk balance bar, speaker sentiment cards, and engagement strip are reusable components (not page-specific).
15. Page load time is not degraded — `get_meeting_post_intelligence` is a lightweight DB query, not a PTY call.
16. After applying `full` mock scenario (with I555 mock data), meeting detail pages with seeded interaction dynamics and champion health render the post-meeting intelligence section correctly — engagement bar, champion badge, categorized outcomes. No console errors.
