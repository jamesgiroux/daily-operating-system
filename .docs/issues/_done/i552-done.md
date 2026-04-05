# I552 — Success Plan Frontend: The Work Evolution + The Record

**Version:** v1.0.0 Phase 4
**Depends on:** I551 (data model + backend), I550 (margin label layout)
**Type:** Feature — frontend evolution of existing components
**Scope:** Frontend only (React + CSS modules)

---

## Context

With I551 delivering objectives, milestones, and expanded lifecycle events on the backend, the account detail page needs two frontend evolutions:

1. **The Work** chapter evolves from a flat action list into a success plan surface showing objectives → milestones → linked actions, while preserving the existing action creation and display for unlinked actions.
2. **Context entries** move from the Appendix to a new **The Record** chapter (powered by UnifiedTimeline), giving them chronological visibility alongside meetings, emails, captures, and lifecycle events.

Both changes follow the editorial magazine aesthetic (ADR-0073/0077) and the margin label layout from I550.

---

## The Work — Success Plan Surface

### Current State

`TheWork.tsx` renders:
- Upcoming meetings section
- Actions grouped by due date classification (Overdue / This Week / Upcoming / No Due Date)
- Inline action creation (+ button → title input → Add/Cancel)

### New State

The Work becomes a two-part chapter:

**Part 1: Objectives** (new, top of section)

Each active objective renders as an editorial card-like section (not a boxed card per ADR-0073 — use section rules):

```
─── Objective: Reduce time-to-value by 40% ───────────────
Target: June 2026 · 2 of 4 milestones · 3 linked actions

  ✓  Kickoff completed                         Mar 1
  ✓  Integration live                          Mar 10
  ○  First value report delivered              Apr 15
  ○  Customer confirms ROI                     Jun 1

  Linked actions:
    □ Send integration checklist to Sarah       P2 · Due Mar 20
    □ Schedule value review meeting             P1 · Due Apr 10
```

- Progress bar: thin, color-coded (sage for on-track, turmeric for at-risk/past target, terracotta for stalled)
- Milestone list: checkmark for completed, circle for pending, dash for skipped
- Milestones with `auto_detect_signal` show a small lightning icon (auto-detection enabled)
- Linked actions: compact list below milestones, same style as current action items
- Objective status badge: draft (muted), active (default), completed (sage), abandoned (muted + strikethrough)

**Part 2: Unlinked Actions** (existing, below objectives)

Actions NOT linked to any objective continue to render in the current grouped format (Overdue / This Week / Upcoming / No Due Date). This preserves the existing workflow for ad-hoc actions.

**Part 3: Upcoming Meetings** (existing, bottom)

Unchanged.

### Interaction

| Action | UX | Backend Call |
|--------|-----|-------------|
| Add objective | "+ Objective" button → inline title input → Enter to save | `create_objective` |
| Edit objective title | Click title → inline edit (EditableText pattern) | `update_objective` |
| Add milestone | "+ Milestone" under last milestone → inline title input | `create_milestone` |
| Complete milestone | Click checkbox | `complete_milestone` |
| Skip milestone | Right-click or ... menu → "Skip" | `skip_milestone` (via `update_milestone`) |
| Link action | Drag action to objective, or ... menu → "Link to objective" → picker | `link_action_to_objective` |
| Unlink action | ... menu on linked action → "Unlink" | `unlink_action_from_objective` |
| Complete objective | Auto when all milestones done, or manual via ... menu | `complete_objective` |
| Abandon objective | ... menu → "Abandon" | `abandon_objective` |
| Reorder objectives | Drag handle | `reorder_objectives` |
| Reorder milestones | Drag handle within objective | `reorder_milestones` |
| View AI suggestions | "Suggestions" button in section header → dropdown/popover | `get_objective_suggestions` |
| Accept suggestion | Click suggestion → creates objective + milestones | `create_objective` + `create_milestone` (batch) |

### AI Suggestions UI

A subtle "Suggestions" button (or sparkle icon) in The Work chapter header. On click, shows a popover with 1-5 AI-suggested objectives derived from the account's intelligence (I551 `get_objective_suggestions`).

Each suggestion shows:
- Title (bold)
- Description (1 line)
- Confidence badge: high (solid), medium (outlined), low (muted) — indicates whether the objective was explicitly stated in meetings or inferred
- Suggested milestones (compact list, with auto-detect lightning icon where applicable)
- Rationale (italic, citing source evidence — e.g., "From Sarah Chen sync on Mar 5")
- "Add" button to accept

Suggestions are read-only previews. "Add" creates the objective with source='ai_suggested', pre-populates milestones with computed target dates and auto-detect signals, and marks contributing captured commitments as consumed.

Empty state: "No suggestions available — objectives will be suggested as more context is gathered." (No system jargon per ADR-0083.)

---

## The Record — Context Entries in Timeline

### Current State

Context entries live in `AccountAppendix.tsx` under a "Context" subsection. They're isolated from the chronological flow of meetings, emails, and events.

### New State

Context entries move into the **UnifiedTimeline** component as a new timeline entry type: `context`. They appear chronologically alongside meetings, email signals, captures, and lifecycle events. This transforms UnifiedTimeline from "The Record" of system-observed events into the full account record including user-contributed context.

### UnifiedTimeline Changes

Add `contextEntries` to `TimelineSource`:

```typescript
export interface TimelineSource {
  recentMeetings: { ... }[];
  recentEmailSignals: { ... }[];
  recentCaptures: { ... }[];
  accountEvents: { ... }[];
  contextEntries: {           // NEW
    id: string;
    title: string;
    content: string;
    createdAt: string;
    updatedAt: string;
  }[];
}
```

Context entries render in the timeline with:
- Icon: notebook/pen icon (differentiates from meeting/email/event icons)
- Title: entry title
- Body: truncated content (first 2 lines, expandable)
- Timestamp: `createdAt`
- Attribution: "Added by you" (context entries are always user-created)

### Appendix Changes

Remove the "Context" section from `AccountAppendix.tsx`. The "+ Add context" button moves to The Record chapter header (or stays in Appendix as a quick-add shortcut — implementation choice).

Appendix retains:
- Lifecycle section (event recording + event list)
- Files section
- Merge button

---

## LifecycleEventDrawer — Expanded Event Types

The existing lifecycle event recording UI (in AccountAppendix) needs to support the 16 event types from I551.

### Current State

The LifecycleEventDrawer (or inline form) offers a dropdown with 4 event types: renewal, expansion, churn, downgrade.

### New State

Expand the event type picker to show all 16 types, organized into categories:

| Category | Event Types |
|----------|-------------|
| **Contract** | renewal, expansion, downgrade, churn, contract_signed |
| **Onboarding** | kickoff, go_live, onboarding_complete, pilot_start |
| **Review** | ebr_completed, qbr_completed, health_review |
| **People** | champion_change, executive_sponsor_change |
| **Escalation** | escalation, escalation_resolved |

Each event type has a human-readable label (ADR-0083):
- `go_live` → "Go-Live"
- `champion_change` → "Champion Change"
- `escalation_resolved` → "Escalation Resolved"
- `ebr_completed` → "EBR Completed"
- etc.

The ARR impact field remains optional and contextual (shown for contract/financial events, hidden for people/review events).

---

## Chapter Order Change

The account detail page chapter order updates:

| # | Current | New |
|---|---------|-----|
| 1 | Headline (Hero) | Headline (Hero) |
| 2 | Portfolio (parent only) | Portfolio (parent only) |
| 3 | State of Play | State of Play |
| 4 | Relationship Health | Relationship Health |
| 5 | The Room | The Room |
| 6 | Watch List | Watch List |
| 7 | **The Record** (was timeline only) | **The Record** (timeline + context entries) |
| 8 | **The Work** (was actions only) | **The Work** (objectives + actions + meetings) |
| 9 | Reports | Reports |
| 10 | Appendix | Appendix (lifecycle + files, no context) |

The Record moves above The Work — you read the history before you plan the work.

---

## Files

### Modified Files

| File | Change |
|------|--------|
| `src/components/entity/TheWork.tsx` | Add objectives section above actions. AI suggestions button. Objective/milestone rendering. |
| `src/components/entity/TheWork.module.css` | New: objective card styles, milestone list, progress bar, suggestion popover |
| `src/components/entity/UnifiedTimeline.tsx` | Add `contextEntries` as timeline entry type. Render with notebook icon. |
| `src/components/account/AccountAppendix.tsx` | Remove Context section. Retain Lifecycle + Files + Merge. |
| `src/pages/AccountDetailEditorial.tsx` | Pass objectives to TheWork. Pass contextEntries to UnifiedTimeline. Update chapter order (Record before Work). |
| `src/types/accounts.ts` | Import/use types from I551 (`AccountObjective`, `AccountMilestone`, expanded `AccountEventType`) |

### New Files

| File | Purpose |
|------|---------|
| `src/components/entity/ObjectiveCard.tsx` | Single objective rendering (milestones, progress, linked actions) |
| `src/components/entity/ObjectiveCard.module.css` | Objective card styles |
| `src/components/entity/ObjectiveSuggestions.tsx` | AI suggestion popover/dropdown |
| `src/components/entity/ObjectiveSuggestions.module.css` | Suggestion popover styles |

---

## Design Tokens

All new styles use existing design tokens from `design-tokens.css`:

- Progress bars: `var(--color-sage)`, `var(--color-turmeric)`, `var(--color-terracotta)`
- Milestone text: `var(--font-serif)` at 16px (per I550 editorial pattern)
- Objective title: `var(--font-serif)` at 18px
- Section rules: `var(--color-rule)` with `1px solid`
- Status badges: existing badge pattern from WatchList
- Spacing: `var(--space-*)` tokens, no hardcoded values

---

## Vocabulary (ADR-0083)

| System Term | User-Facing Label |
|-------------|------------------|
| objective | "Objective" (acceptable — CS industry term) |
| milestone | "Milestone" (acceptable — universally understood) |
| auto_detect_signal | never shown — lightning icon only |
| ai_suggested | "Suggested" |
| sort_order | never shown |
| context_entry | "Note" or just the title |
| signal_momentum | never shown — invisible to user |

---

## Out of Scope

- Backend/data model changes (I551)
- Template system (I553)
- Drag-and-drop reordering (can use ... menu → Move Up/Down as simpler v1)
- Customer-visible success plan sharing (post-1.0)
- Gantt chart or timeline visualization of milestones
- Person detail or project detail adaptations

---

## Acceptance Criteria

1. The Work chapter shows objectives above the existing action groups. Each objective displays title, target date, progress bar, milestone list, and linked action count.
2. Milestones render as a checklist within each objective. Clicking checkbox completes the milestone. All milestones completed → objective auto-completes.
3. Draft objectives appear muted. Completed objectives show sage accent. Abandoned objectives are muted + strikethrough.
4. Unlinked actions continue to render in grouped format (Overdue / This Week / Upcoming / No Due Date) below objectives.
5. Inline objective creation works: "+ Objective" → title input → Enter saves, Escape cancels.
6. Inline milestone creation works: "+ Milestone" under last milestone → title input → Enter saves.
7. AI suggestions button shows 1-5 suggestions from `get_objective_suggestions`. "Add" creates objective + milestones. Empty state shows guidance text (no system jargon).
8. Context entries appear in UnifiedTimeline chronologically with notebook icon, title, truncated content, and "Added by you" attribution.
9. Context section removed from AccountAppendix. Appendix retains Lifecycle + Files + Merge.
10. LifecycleEventDrawer supports all 16 event types organized by category. Human-readable labels per ADR-0083.
11. The Record chapter appears before The Work in page order.
12. Zero inline styles in new components. All CSS in module files using design tokens.
13. Zero ADR-0083 vocabulary violations in any user-facing string.
14. Empty state: account with no objectives shows "No objectives yet" with "+ Objective" button. No error, no blank section.
15. Progress bar color reflects status: sage (on track), turmeric (approaching target date), terracotta (past target date with pending milestones).
16. After applying `full` mock scenario (I551 mock data), all new UI renders correctly: objectives with progress bars, milestone checklists, linked action counts, context entries in timeline, expanded lifecycle events in Appendix. No console errors, no blank sections.
