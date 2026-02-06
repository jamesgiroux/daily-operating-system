# User Journeys: Where We Are, Where We're Not

> Assessment of end-to-end user flows against stated philosophy.
> This document maps what exists, what's missing, and asks the hard questions.
>
> Written after Phase 1 MVP + Phase 1.5 Nav/UI. The app works. Now we need to
> understand how people actually move through it.

---

## How to Read This Document

Each journey follows this structure:

1. **The scenario** — Who is this person, what's happening
2. **What happens today** — What the app actually does right now
3. **What's missing** — Gaps between intent and reality
4. **Hard questions** — Things we haven't answered yet

At the end: cross-cutting concerns that affect multiple journeys.

---

## Journey 1: First Launch

### The Scenario

Someone installs DailyOS for the first time. They've heard about it, they're curious, they downloaded the `.dmg`. They double-click the app icon.

They are a knowledge worker. They probably use Google Calendar and Gmail. They've never used a terminal. They have no idea what a "workspace" is.

### What Happens Today

```
Launch app
  → Profile selector modal (Customer Success or General)
  → Select profile
  → Dashboard loads
  → Empty state: "No briefing yet" + "Generate Briefing" button
  → Click "Generate Briefing"
  → Workflow starts...
  → ...but Google isn't connected
  → prepare_today.py fails (no calendar, no email)
  → Error state
```

The profile selector is clean and works. Everything after it is a cliff.

### What's Missing

**There is no onboarding flow.** The user goes from profile selection to a dead end. The app needs Google Calendar and Gmail to produce a briefing. Without those, the core value proposition — "Open the app. Your day is ready." — is impossible. But we never guide the user to connect Google.

The sequence should be:

```
Install → Profile → Connect Google → Set workspace path → First briefing → Value
```

But only step 2 exists.

**Specific gaps:**

- No workspace creation or selection (relies on `~/.dailyos/config.json` existing)
- No Google connection prompt during setup
- "Generate Briefing" fires but fails silently if prerequisites aren't met
- Settings page has Google connection, but a first-time user doesn't know to go there
- Error messages after failed briefing don't explain what to fix

### Hard Questions

1. **What's the minimum viable first experience?** Can we show anything useful without Google connected? (Calendar data from `.ics` file import? Manual meeting entry? Or is Google connection truly required for first value?)

2. **How many steps before first value?** The ideal is zero (app works out of the box). The reality is at least: profile + Google auth + workspace path. That's three steps minimum. How do we make them feel effortless rather than like setup?

3. **What if they don't have Google Workspace?** Personal Gmail? Outlook? iCloud Calendar? We've assumed Google but the user base might not be 100% Google. Is this MVP-acceptable ("Google required") or a design flaw?

4. **Do we need a workspace path at all for first launch?** Could we create a default workspace at `~/Documents/DailyOS/` and let them change it later? Principle 4: Opinionated defaults, escapable constraints.

5. **What happens if the first briefing takes 3 minutes?** The user clicked "Generate Briefing," the spinner is spinning... do they wait? Leave? What does the wait state communicate? Is there enough visual feedback to keep them engaged?

---

## Journey 2: The Morning Briefing

### The Scenario

It's Tuesday. The user's briefing ran at 6:00 AM automatically. They open the app at 8:15 AM with coffee. They have 6 meetings today, including 2 customer calls.

### What Happens Today

```
Open app
  → Dashboard loads instantly (JSON already generated)
  → Overview: date, greeting, summary, focus
  → Stats: 6 meetings, 2 customer, 4 actions due, 0 inbox
  → Meeting timeline: chronological list with type badges
  → Emails: high-priority flagged
  → Actions: due/overdue items
```

User scrolls through, gets a sense of the day. They see their 9:00 AM customer call has a prep file. They... can't click it from the dashboard yet (meeting card links aren't wired to meeting detail page).

### What's Missing

**The meeting card → prep detail flow is broken.** Meeting cards in the timeline don't link to the meeting detail page. The detail page exists (`/meeting/$prepFile`) but the entry point from the dashboard isn't connected. This is the most important user action on the dashboard ("I see my next meeting, let me review the prep") and it doesn't work.

**The "what do I do first?" question is unanswered.** The dashboard shows everything simultaneously. There's no prioritization of attention. On a 6-meeting day, what should the user focus on RIGHT NOW? The meeting that's in 45 minutes? The overdue action? The high-priority email?

**The summary is generic.** "6 meetings today with 2 customer calls" is factual but not insightful. A good EA would say: "Big day. Your 9:00 Acme call is the priority — they're up for renewal next month and ARR is $180K. You also have an overdue action for Globex that's been sitting for 3 days."

### Hard Questions

1. **What is the dashboard's job?** Is it a morning newspaper (read once, reference later) or a live command center (always current)? Right now it's a snapshot from 6 AM. If a meeting gets cancelled at 7 AM, the dashboard still shows it. The calendar polling system has live data but the dashboard schedule is static.

2. **Should the overview summary be AI-generated?** The current summary is computed by deliver_today.py with a template. Phase 2 (Claude enrichment) could write a genuinely insightful summary. But that means the summary quality depends on AI enrichment succeeding. What's the fallback?

3. **How does the user get from "I've read my briefing" to "I'm doing work"?** The app has no concept of "I'm done reviewing." Do they minimize and come back later? Is there a natural exit point? Or does the app try to remain relevant throughout the day?

4. **What's the relationship between the briefing and the live calendar?** The briefing is generated once. The calendar polls every 5 minutes. These are two separate data sources that can disagree. Should the dashboard merge them? Show live calendar with briefing annotations?

---

## Journey 3: The Busy Day (8+ Meetings)

### The Scenario

It's Wednesday. The user has 9 meetings, back-to-back from 9 AM to 5 PM with 15-minute gaps. This is a real day for CSMs and sales reps. They have maybe 2 minutes between meetings to glance at their phone or laptop.

### What Happens Today

```
Open app at 8:50 AM
  → See 9 meetings on timeline
  → Scroll to find the 9:00 AM meeting
  → Read prep (if they can find the link... they can't)
  → Close app
  → Meeting happens
  → 9:55 AM: post-meeting capture prompt appears
  → Type "Win: Acme agreed to expand" + "Action: Send proposal by Friday"
  → Auto-dismiss after 60 seconds
  → 10:00 AM: next meeting starts
  → No time to review prep for this one
  → Repeat 7 more times
```

### What's Missing

**There is no "what's next" view.** On a busy day, the user doesn't need the full timeline. They need: "Your next meeting is in 12 minutes. Here's the one thing you need to know." A focused, time-aware view that surfaces only what matters RIGHT NOW.

**Post-meeting capture works but is a race.** 60-second auto-dismiss is tight when you're also joining the next meeting. The capture flow is well-designed (quick wins/risks/actions) but the timing is brutal on a packed day. The 2-minute / 5-minute / 10-minute delay options help, but what if the next meeting has already started?

**There's no "between meetings" mode.** The dashboard is the same whether you have 2 minutes or 2 hours. On a busy day, the ideal interface is essentially a heads-up display: next meeting, prep status, any urgent actions. Everything else is noise.

### Hard Questions

1. **Should the app be time-aware?** Should it know "the user has 12 minutes before their next meeting" and adjust what it shows? Or is that over-engineering?

2. **What's the minimum useful interaction between meetings?** If you have 2 minutes: review next meeting prep, check one urgent action, capture one outcome. Can we design for that 2-minute interaction?

3. **Should post-meeting capture queue if the user is busy?** Instead of prompting immediately after Meeting A ends (when Meeting B just started), could it queue captures and present them during the first gap? "You had 3 meetings since we last checked in. Quick outcomes?"

4. **Is the meeting timeline the right metaphor for a busy day?** A chronological list of 9 meetings is overwhelming. Would a "now → next → later" view serve better? Current meeting expanded, next meeting visible, everything else collapsed?

---

## Journey 4: The Light Day (1-2 Meetings)

### The Scenario

It's Friday. The user has one team standup at 10 AM and nothing else. They have 6 hours of open time.

### What Happens Today

```
Open app
  → Dashboard shows 1 meeting, few actions
  → Meeting timeline is sparse (one card)
  → Actions section has some items
  → Emails section has some items
  → User reads everything in 30 seconds
  → ...now what?
```

### What's Missing

**The app has nothing to say about open time.** The Focus page exists but it's a separate page, not integrated into the dashboard. On a light day, the dashboard's most valuable output would be: "You have 6 hours of open time today. Here's what you could accomplish." Instead it shows a mostly-empty meeting timeline.

**There's no coaching.** The EA metaphor breaks down here. A good EA on a light day would say: "You've got breathing room today. That Globex proposal has been sitting for 3 days — good day to knock it out. Also, your inbox has 2 files from yesterday that need processing." The app shows data but doesn't connect the dots.

**Action triage doesn't adapt.** On a busy day, you might only look at P1 actions. On a light day, you could work through P2 and P3 items. The app shows the same action list regardless.

### Hard Questions

1. **Should the dashboard adapt to calendar density?** Different layouts for heavy vs light days, or a single layout that works for both?

2. **How much should the AI "coach"?** Saying "good day to tackle that proposal" is helpful. Saying "you should also review your Q3 numbers and clean up your filing system" is nagging. Where's the line?

3. **Should focus/priority data live on the dashboard?** Currently Focus is a separate page. On a light day, focus recommendations ARE the dashboard's main value. Should they be integrated?

4. **What's the relationship between Actions (page) and Actions (dashboard widget)?** The dashboard shows a few actions. The Actions page shows all of them with filtering. Are these the same data presented differently, or should they have different purposes?

---

## Journey 5: Post-Meeting Capture

### The Scenario

The user just finished a 45-minute customer call. There were two wins, one risk, and three action items. The post-meeting prompt appears.

### What Happens Today

```
Meeting ends
  → 5-minute delay (configurable)
  → Prompt slides in: "Any quick outcomes from [Meeting Title]?"
  → Three buttons: Win / Risk / Action
  → User types wins, risks, actions (one at a time)
  → Items saved to SQLite + impact log
  → Auto-dismiss after 60s of inactivity
```

This flow is actually well-designed. The capture UI is fast, keyboard-friendly, and respects the user's time.

### What's Missing

**Captured data doesn't flow back into the system visibly.** Wins go to SQLite and the impact log markdown file. Risks go to SQLite. Actions go to SQLite. But none of this surfaces in the dashboard or the next day's briefing in an obvious way. The user captures data and it... disappears into the system. Trust erodes when you don't see the effect of your input.

**There's no "what happened today" view.** At end of day, the user can't see: "Today you captured 4 wins, 2 risks, 8 actions across 6 meetings." The data exists but there's no aggregated view.

**Capture doesn't connect to meeting prep for next time.** If the user captures "Risk: Acme considering competitor" — that should show up in the prep for the next Acme meeting. Does it? The meeting prep is generated fresh each briefing from the directive. Does the directive incorporate captured outcomes from previous meetings?

### Hard Questions

1. **How does captured data resurface?** If I capture a risk today, when do I see it again? In tomorrow's briefing? In the next meeting with that account? In a weekly summary? The data lifecycle after capture is undefined.

2. **Should capture be more than wins/risks/actions?** What about "follow up with [person]"? "Schedule [thing]"? "Note: [observation]"? Or does keeping it to three types maintain simplicity?

3. **Is the 60-second auto-dismiss right?** The user might be in the middle of typing an action item when it dismisses. Should it only auto-dismiss if there's been NO interaction, not if they're mid-capture?

---

## Journey 6: Weekly Planning

### The Scenario

It's Monday morning. The user wants to plan their week. They navigate to the Week page.

### What Happens Today

```
Navigate to Week page
  → No week data yet
  → Empty state with "Run /week" button
  → Click "Run /week"
  → Workflow executes (prepare → enrich → deliver)
  → Week grid appears: Mon-Fri with meetings
  → "Plan This Week" button → wizard opens
  → Step 1: Pick 3-5 priorities
  → Step 2: Review week overview
  → Step 3: Select focus blocks
  → Done
```

### What's Missing

**Daily and weekly meeting data are completely independent.** The week view generates its own meeting data from a separate workflow run. If you look at Tuesday's meetings in the week view and then navigate to Today (which is Tuesday), you're looking at data from two different workflow runs that may disagree. A meeting that was cancelled between the weekly run and the daily run will show in one but not the other.

**Prep status on weekly meetings is decorative.** The week view shows prep status badges (Prep Needed, Prep Ready, etc.) but these don't connect to actual prep files. You can't click a weekly meeting to see its prep. The prep files are generated by the daily briefing, not the weekly one.

**The wizard flow is disconnected from the rest of the app.** You pick priorities in the wizard, but those priorities don't influence what the daily briefing shows. You select focus blocks, but those don't appear on the daily dashboard. The wizard captures intent but doesn't feed it back into the system.

### Hard Questions

1. **Why are daily and weekly separate workflows?** Both need calendar data. Both produce meeting cards. The weekly overview is essentially 5 daily briefings stitched together with a meta-layer. Could the weekly view be built FROM daily data rather than independently?

2. **Should meeting cards be a unified data model?** A meeting is a meeting regardless of whether you're viewing it on the daily dashboard or the weekly grid. If you mark prep as "done" on the daily view, the weekly view should reflect that. Right now there's no shared state.

3. **What happens to weekly priorities?** The user picks "Focus on Acme renewal" as a Monday priority. Does that influence what the daily briefing emphasizes on Tuesday? On Wednesday? Or do priorities live only in the wizard and never feed forward?

4. **When does weekly planning happen?** We assume Monday morning. What if the user wants to plan Sunday night? What if they never plan and just use daily? Is weekly planning required or optional? If optional, what's the cost of skipping it?

5. **Is the 3-step wizard the right model?** Step 1 (priorities) makes sense. Step 2 (review overview) is passive — the user is just reading. Step 3 (focus blocks) assumes the user knows their energy patterns. Is this actually useful or is it ceremony?

---

## Journey 7: Returning After Absence

### The Scenario

The user was on vacation for a week. They open the app Monday morning.

### What Happens Today

```
Open app
  → IF archive ran during absence: empty state, "Generate Briefing" button
  → IF archive didn't run: stale data from last Thursday
    → Dashboard shows Thursday's briefing
    → "Last updated Thursday at 6:02 AM" (freshness indicator we just added)
    → "Generate Briefing" button available in header
```

This is actually one of the better flows thanks to the freshness work we just did. No guilt, no "you missed 5 days!" — just quiet context about when data was last updated.

### What's Missing

**No "welcome back" context.** A good EA would say: "Welcome back. While you were out: 23 emails flagged, 3 actions went overdue, Acme sent a renewal notice." Instead the user gets either empty state or stale data with no bridge.

**The archive may have run but data/ survived until our fix.** Before this session's archive cleanup fix, `data/` persisted after archiving. So a returning user might see an empty meeting timeline (markdown archived) but stale stats and emails (JSON persisted). This is now fixed but worth noting.

**There's no "catch up" mode.** The briefing generates for today. But the user missed a week. What about the 15 emails and 8 actions that accumulated? The daily briefing only surfaces today's context, not the backlog.

### Hard Questions

1. **Should the first briefing after absence be different?** Instead of a normal daily briefing, should it include a "while you were out" summary? This would require knowing when the user was last active.

2. **What happens to actions that went overdue during absence?** They're in SQLite but the daily briefing may not surface all of them. Should the briefing be smarter about surfacing stale actions after a gap?

3. **How does the system know you were absent vs. just not using the app?** If the user doesn't open the app for 3 days but is still working, that's different from vacation. Do we track last-active? Or do we not care and just show today's data?

---

## Journey 8: Inbox Processing

### The Scenario

The user drags a meeting transcript (PDF) into the app's inbox. They want it processed, summarized, and filed.

### What Happens Today

```
Drag file into inbox drop zone (or into _inbox/ folder)
  → File appears in inbox list
  → Click "Process" or "Process All"
  → Deterministic classifier runs (quick, ~1 second)
  → Classified as: meeting notes, action items, account update, etc.
  → Routed to appropriate folder
  → If classification is uncertain → AI enrichment via Claude Code
  → Actions extracted and saved to SQLite
  → Banner: "1 file routed successfully"
```

This flow is solid. Drag-drop works, processing is fast for most files, AI enrichment handles edge cases.

### What's Missing

**No automatic processing.** JTBD says: "Files processed automatically, routed correctly, actions extracted." But the user has to click "Process" manually. The architecture doc mentions file watching but it's explicitly out of MVP scope. This means the inbox can accumulate files that the user has to remember to process.

**Processing results are invisible after routing.** A file gets classified and routed to `Accounts/Acme/`. Great. But the user can't see where it went from the inbox page. The banner says "1 file routed" but not where. Trust requires visibility.

**No connection between inbox items and daily briefing.** If a transcript gets processed and actions extracted, those actions live in SQLite. But the next day's briefing reads actions from the directive (generated by prepare_today.py), not from SQLite. Are SQLite actions and directive actions the same thing? Different things? This is a data model question.

### Hard Questions

1. **When does file watching ship?** Without it, the inbox is a manual step that violates "the system operates." This is a philosophical breach, not just a missing feature.

2. **Should processing happen on drop or on schedule?** Real-time processing when a file appears vs. batch processing on a schedule. Real-time is more magical but harder to build. Batch is simpler but less "EA-like."

3. **How does the inbox connect to the briefing pipeline?** If I drop a transcript at 3 PM, should tomorrow's briefing know about it? Should the context from that transcript influence tomorrow's meeting prep for the same account?

4. **What's the inbox's relationship to email?** Emails are fetched by prepare_today.py from Gmail API. Inbox files are dropped manually. But an email with an attachment is both. Should forwarding an email to a special address drop it in the inbox? Or is that scope creep?

---

## Journey 9: Settings & Configuration

### The Scenario

The user wants to change when their briefing runs, connect their Google account, or switch from Customer Success to General profile.

### What Happens Today

Settings page has:
- Google account connection (connect/disconnect)
- Post-meeting capture toggle + delay
- Workspace path display (read-only, edit via config.json)
- Theme toggle
- Schedule display (read-only cron expressions)
- Manual workflow triggers (Run /today, Run /week, Run Archive)

### What's Missing

**Profile switching.** The profile selector at onboarding says "You can change this later in Settings." But Settings doesn't have profile switching. This is a broken promise.

**Schedule editing.** Schedules are displayed as cron expressions but can't be edited in the UI. Users must edit `~/.dailyos/config.json` directly. This violates Principle 3 (Buttons, Not Commands).

**Workspace path editing.** Same problem — displayed but not editable in UI.

**No explanation of what settings DO.** The post-meeting capture toggle exists but doesn't explain what it is or why you'd want it. The briefing schedule shows "0 6 * * *" which means nothing to non-technical users.

### Hard Questions

1. **What settings actually need to exist?** The current settings feel like a developer's config panel. What does a non-technical user need to configure? Probably: when briefing runs (time picker, not cron), Google connection, and maybe notification preferences. Everything else should be opinionated defaults.

2. **Should settings be progressive?** Show simple options by default, advanced options behind an expand. "Briefing time: 6:00 AM [change]" is better than "Cron: 0 6 * * * (Timezone: America/Toronto)."

3. **Where does onboarding end and settings begin?** Google connection could be in onboarding OR settings. Profile selection could be in onboarding OR settings. Should settings be the canonical place for all configuration, with onboarding being a guided first pass through the important ones?

---

## Cross-Journey Concerns

These issues affect multiple journeys and need architectural decisions.

### The Two-Source Calendar Problem

The app has two sources of calendar truth:

1. **Briefing data** (`schedule.json`) — Generated once at briefing time. Static snapshot. Has AI enrichment (prep summaries, meeting classification, context).

2. **Live calendar poll** (`calendar_events` in AppState) — Polls Google Calendar every 5 minutes. Always current. No enrichment.

These can disagree. A meeting cancelled after the briefing ran shows in the briefing but not the live poll. A meeting added after the briefing shows in the live poll but not the briefing.

**The question:** Should the dashboard show briefing data (richer but stale) or live data (current but bare)? Or merge them (complex but correct)?

**Possible approach:** Use live calendar as the source of truth for what meetings exist. Overlay briefing enrichment (prep, classification, context) onto live events by matching on calendar event ID. If a meeting exists in briefing but not live → it was cancelled, hide it. If a meeting exists in live but not briefing → it's new, show it bare.

This is an architectural decision that affects Journey 2, 3, 4, and 6.

### The Meeting Card Unification Problem

Today, meetings exist in three independent forms:

1. **Daily dashboard meeting card** — From `schedule.json`, has prep summary, type badge, account
2. **Weekly grid meeting cell** — From `week-overview.json`, has prep status badge, type
3. **Meeting detail page** — From `preps/*.json`, full context

These are generated by different workflows and have no shared state. Prep done on the daily view doesn't reflect on the weekly view. A meeting that's "current" on the daily view has no equivalent concept on the weekly view.

**The question:** Should there be one `Meeting` entity that's the same regardless of where you view it? A meeting record with an ID, a state (prep status, notes, outcomes), and a lifecycle that persists across daily and weekly views?

**What this would require:**
- A stable meeting ID scheme (we have this — calendar event ID)
- A meeting state store (SQLite? Already used for actions)
- Views that read from the same source and render differently
- Prep status that updates in one place and reflects everywhere

This is a significant architectural change but it solves problems across Journey 2, 3, 5, 6.

### The Action Lifecycle Problem

Actions come from four sources:
1. **Daily briefing** (directive → actions.json) — Overdue, due today, due this week
2. **Post-meeting capture** — Saved to SQLite
3. **Inbox processing** — Extracted by classifier, saved to SQLite
4. **Weekly planning** — Priorities selected in wizard

These live in two stores:
- **JSON** (`actions.json`) — Ephemeral, regenerated each briefing
- **SQLite** — Persistent, cross-day

The Actions page reads from SQLite. The dashboard reads from JSON. These can disagree.

**The question:** What is the canonical source of truth for actions? If I complete an action on the Actions page (SQLite update), does the dashboard still show it as pending (JSON unchanged)?

**Possible approach:** SQLite is the source of truth. The daily briefing writes new actions to SQLite (upsert by ID). The dashboard reads from SQLite, not JSON. `actions.json` becomes a staging area, not a display source.

### The AI Enrichment Timing Problem

AI enrichment (Phase 2 — Claude Code) is expensive: 2-5 minutes per run, requires Claude subscription, can fail.

**When does it run?**
- Daily briefing: 6 AM scheduled, or manual trigger
- Weekly overview: Manual trigger only
- Inbox processing: On-demand per file

**What if it fails?**
- Briefing falls back to Phase 1 data (calendar + actions, no AI summary)
- Weekly overview shows no data
- Inbox file stays in inbox

**The question:** How much of the app's value depends on AI enrichment succeeding? If Phase 2 fails every day for a week, is the app still useful?

**Current state:** The app degrades but remains functional. Phase 1 (deterministic) produces meeting cards, action lists, and email triage without AI. The AI adds summaries, prep context, focus suggestions, and coaching text. So the app works without AI but is significantly less valuable.

**Is this acceptable?** Probably yes for MVP. But the user might not understand why their briefing feels "thin" some days (AI failed) and "rich" others (AI succeeded). Should we communicate enrichment status? "Your briefing was enriched with AI context" vs. "Quick briefing — AI enrichment will run on next refresh."

### The Google API Efficiency Problem

Every briefing run calls:
- Google Calendar API (fetch today's events)
- Gmail API (fetch recent emails)
- Google Sheets API (fetch account data, CS profile only)

**When does this happen?**
- 6 AM scheduled briefing
- Manual "Generate Briefing" click
- Calendar poller (every 5 minutes, Calendar API only)

**What's inefficient?**
- Manual re-run fetches everything fresh even if nothing changed
- Calendar poller and briefing both call Calendar API independently
- No caching layer between API calls and consumers

**The question:** Should there be a shared API cache? Calendar data fetched by the poller could be reused by the briefing. Email data could be cached with a TTL. This reduces API calls and makes manual re-runs faster.

---

## What This Assessment Reveals

### The app is strong at:
- **Static briefing generation** — The three-phase pattern works well
- **Meeting prep depth** — When prep exists, it's genuinely useful
- **Capture UX** — Post-meeting capture is fast and respectful
- **Inbox processing** — Drag-drop + classify + route is solid
- **Zero-guilt aesthetics** — No streak counters, no shame, stale data is handled gracefully

### The app is weak at:
- **First-time experience** — No onboarding beyond profile selection
- **Flow between views** — Dashboard → meeting detail isn't connected
- **Live vs static data** — Two calendar sources, two action sources, no reconciliation
- **Adaptiveness** — Same dashboard whether 0 meetings or 9
- **Data lifecycle** — Captured outcomes don't visibly resurface
- **Settings UX** — Developer-facing, not user-facing

### The hardest decisions ahead:
1. **Meeting card unification** — One entity or stay independent?
2. **Calendar source of truth** — Briefing snapshot or live poll?
3. **Action source of truth** — JSON or SQLite?
4. **Onboarding scope** — How much setup before first value?
5. **Adaptive dashboard** — Time-aware and density-aware, or keep it simple?

---

## Recommended Next Steps

These are ordered by what unblocks the most downstream work:

### 1. Design the first-time experience
Without this, nobody can use the app. Doesn't need to be fancy — Profile → Google → Workspace → First briefing. But it needs to exist.

### 2. Connect meeting cards to meeting detail
The dashboard → prep flow is the app's killer moment (from PHILOSOPHY.md: "You open DailyOS. Your day is already there. For that mystery meeting, you have..."). The meeting detail page exists but has no entry point. Wire it.

### 3. Decide on calendar source of truth
This is an architectural decision that affects everything else. Pick one: briefing snapshot with live overlay, or live calendar with enrichment cache. Document in RAIDD.

### 4. Decide on action source of truth
Same: pick SQLite or JSON as canonical. The other becomes a write-through layer. Document in RAIDD.

### 5. Design the adaptive dashboard (or decide not to)
This is the biggest open design question. Could be "not MVP" — a single dashboard that works adequately for both busy and light days is acceptable. But if we want to pursue it, the design needs to happen before more frontend work.

---

*This document should be revisited after each of these decisions is made. The journeys will change as the architecture solidifies.*
