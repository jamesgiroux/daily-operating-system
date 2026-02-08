# ADR-0052: Week page redesign — consumption-first weekly briefing

**Date:** 2026-02-08
**Status:** Accepted

## Context

The Week page currently displays a 5-column calendar grid, two summary numbers (overdue count, due-this-week count), hygiene alerts, and a focus areas list. This is a regression from the CLI markdown archive (`week-00-overview.md`), which included actual action items with accounts and due dates, suggested calendar blocks tied to specific tasks, and account hygiene alerts by severity.

The page feels empty, with little visual hierarchy or value. It duplicates what the user can see in Google Calendar (a grid of meetings) without adding the intelligence layer that DailyOS exists to provide.

Meanwhile, the "Plan your week" wizard (`useWeekPlanning.ts`) prompts users to manually submit priorities and select focus blocks — a production activity that contradicts Principle 7 (Consumption Over Production) and Principle 6 (AI-Native, Not AI-Assisted). The wizard pops on Monday mornings but has no visible effect on what the Week page displays, making it feel disconnected.

### What users actually need from weekly prep

Weekly prep answers different questions than the daily briefing:

- "What's the one thing I need to get done this week?"
- "Where do I have space for deep work?"
- "Which days are crammed and which are light?"
- "Am I prepared, or are there gaps I should close now?"

These are consumption questions, not production tasks. The system should answer them proactively.

## Decision

### 1. Replace the calendar grid with a weekly briefing

The Week page becomes a consumption-first briefing, not a calendar view. Six sections in priority order:

| Section | Purpose | Data source |
|---------|---------|-------------|
| **Week Narrative** | AI-generated 2-3 sentence summary of the week's shape | AI enrichment |
| **Top Priority** | Single highest-impact item for the week | AI enrichment (from actions + meetings + hygiene) |
| **Readiness Checks** | Proactive intelligence — prep gaps, agenda gaps, stale contacts | Mechanical (computed from schedule + preps + entities) |
| **Week Shape** | Day-by-day density visualization (bars, not grid) | Mechanical (meeting count + duration) |
| **Actions** | Actual action items grouped as overdue + due this week | Mechanical (from SQLite) |
| **Account Health** | Hygiene alerts (unchanged) | Mechanical (from enrichment) |

### 2. Retire the "Plan your week" wizard

The manual wizard (priorities submission + focus block selection) violates the prime directive: "The system operates. You leverage." Users shouldn't manually plan their week — the system should propose a plan.

Remove:
- `useWeekPlanning.ts` hook
- `WeekPlanningState` type
- `FocusBlock` type
- `show-week-wizard` event
- "Plan this week" button
- `get_week_planning_state`, `submit_week_priorities`, `submit_focus_blocks`, `skip_week_planning` Tauri commands

### 3. Replace with AI-driven time blocking as a setting

Time blocking (matching tasks to available calendar gaps) becomes an AI enrichment step, not a manual wizard. A user setting controls the level of proactivity:

| Setting | Behavior |
|---------|----------|
| **Suggestions only** (default) | Week briefing shows available time blocks with suggested tasks. No calendar writes. |
| **Draft blocks** | AI proposes specific time blocks with tasks. User reviews in Week page. One-click accept/dismiss. |
| **Auto-block** (future) | AI writes focus blocks directly to Google Calendar. Requires write scope. |

Only "Suggestions only" ships initially. The setting exists in config but higher levels are gated by feature toggles (ADR-0039).

### 4. Expand WeekOverview data model

New fields on `WeekOverview`:

```typescript
weekNarrative?: string;
topPriority?: {
  title: string;
  reason: string;
  meetingId?: string;
  actionId?: string;
};
readinessChecks?: ReadinessCheck[];
dayShapes?: DayShape[];
```

New types:

```typescript
interface ReadinessCheck {
  type: "no_agenda" | "no_prep" | "stale_contact" | "overdue_action" | "missing_context";
  message: string;
  severity: "action_needed" | "heads_up";
  meetingId?: string;
  accountId?: string;
  actionable?: { label: string; command: string };
}

interface DayShape {
  dayName: string;
  date: string;
  meetingCount: number;
  meetingMinutes: number;
  density: "light" | "moderate" | "busy" | "packed";
  meetings: WeekMeeting[];
  availableBlocks: TimeBlock[];
}

interface WeekAction {
  id: string;
  title: string;
  account?: string;
  dueDate?: string;
  priority: Priority;
  daysOverdue?: number;
  source?: string;
}
```

The `actionSummary` field changes from counts-only to actual items:

```typescript
actionSummary?: {
  overdue: WeekAction[];
  dueThisWeek: WeekAction[];
  criticalItems: string[];  // kept for backward compat during transition
};
```

### 5. Implementation phases

**Phase 1 (mechanical):** Readiness checks, day shapes, expanded actions. All computed from existing data in SQLite + schedule. No AI required. Ships first.

**Phase 2 (AI enrichment):** Week narrative, top priority. Added to `/week` enrichment prompt. Requires Claude Code.

**Phase 3 (new capabilities):** Proactive suggestions (draft agenda messages, pre-fill preps), time block proposals tied to specific actions.

## Consequences

**Easier:**
- Week page provides immediate value from mechanical data alone (Phase 1)
- Information hierarchy matches how users actually think about their week
- Readiness checks surface problems before they become surprises
- Actual action items eliminate the "what are these 3 overdue items?" question
- Removing the wizard eliminates dead UI and simplifies the codebase

**Harder:**
- More data to compute in the `/week` pipeline (readiness checks require cross-referencing schedule + preps + entities)
- Day shape density classification needs calendar event duration data (already available from `CalendarEvent.end - CalendarEvent.start`)
- Week narrative quality depends on AI enrichment — mechanical fallback shows no narrative (acceptable per ADR-0042 fault-tolerance pattern)

**Trade-offs:**
- Calendar grid view is lost — users who wanted a visual calendar now see density bars instead. Acceptable because Google Calendar already serves this need; DailyOS should differentiate on intelligence, not duplication.
- The wizard removal means no manual priority input. If users want to set their own priority, they'll need to use the actions system directly. The AI-generated top priority may disagree with the user — but per Principle 4, it's an opinionated default that's overridable.
