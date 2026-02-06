# Product Assessment & Roadmap Proposal

> Honest audit of what's been built, what's been promised, and what comes next.
>
> Date: 2026-02-05

---

## Executive Summary

DailyOS has significantly overbuilt beyond the original MVP scope. The MVP (F1+F7+F6+F3) is functionally complete and most of Phases 2 and 3 backend work has shipped. But the docs haven't caught up, acceptance criteria are unchecked, and the gap between "code exists" and "feature works end-to-end" hasn't been honestly measured.

**The core value proposition — "Open the app. Your day is ready" — is validated.** The briefing runs, the dashboard renders, the tray works. That's real.

**What's at risk:** The project has expanded into Phase 2/3 work without completing Phase 1's stability validation or closing Phase 2/3 acceptance criteria. Code exists for calendar polling, post-meeting capture, weekly planning, and inbox processing — but much of it is untested end-to-end with real user workflows.

---

## Part 1: JTBD Coverage Audit

### Job 1: Prepare My Day

> "When I start my workday, I want everything I need already gathered and synthesized."

| Promised Outcome | Built? | Working E2E? | Notes |
|-----------------|--------|-------------|-------|
| Morning briefing runs before I open the app | Yes | Yes | Scheduler + executor + PTY working |
| Calendar context in briefing | Yes | Partial | Requires Google OAuth — auth flow built but untested in daily use |
| Meeting prep surfaced per meeting | Yes | Yes | MeetingCard with expandable prep |
| Actions due today visible | Yes | Yes | ActionList on dashboard |
| Emails triaged | Yes | Partial | Email summary section exists but depends on Python script output |
| Notification: "Your day is ready" | Yes | Yes | Native macOS notification |

**Verdict: Job 1 is substantially delivered.** The "partial" items depend on Google API integration stability, which is Phase 3 work that got built early. The core loop (scheduler → three-phase → dashboard) works.

---

### Job 2: Capture & Route My Inputs

> "When I get content from anywhere, I want it processed and filed automatically."

| Promised Outcome | Built? | Working E2E? | Notes |
|-----------------|--------|-------------|-------|
| File watcher detects new files in `_inbox/` | Yes | Yes | watcher.rs with debouncing |
| Classify file type automatically | Yes | Partial | classifier.rs exists; AI enrichment for unknowns works |
| Route to correct PARA location | Yes | Partial | router.rs handles known types; unknowns queue for review |
| AI enrichment (summaries, actions) | Yes | Partial | enrich.rs works but requires Claude Code running |
| Actions extracted to SQLite | Yes | Untested | hooks.rs post-enrichment engine built but never run in production |
| Bidirectional action sync (SQLite ↔ markdown) | Yes | Untested | sync_completion_to_markdown in hooks.rs, not exercised |
| Processing queue shows status | Yes | Yes | InboxPage shows files and status |
| Zero manual `/inbox` runs needed | Partial | No | Batch scheduler exists but relies on Claude Code availability |

**Verdict: Job 2 is architecturally complete but operationally unproven.** The file watcher, classifier, router, and enrichment pipeline all exist as code. The post-enrichment engine (hooks.rs) was just built. None of this has been tested with real-world transcript drops. The happy path (drop file → auto-process → route → extract actions) needs end-to-end validation.

**This is the most critical job** per JTBD.md, and it's the least validated.

---

### Job 3: Close My Day

> "When I finish working, I want my accomplishments captured and loose ends identified."

| Promised Outcome | Built? | Working E2E? | Notes |
|-----------------|--------|-------------|-------|
| Archive runs silently at midnight | Yes | Yes | archive.rs, pure Rust, no AI |
| Post-meeting capture after customer meetings | Yes | Untested | capture.rs state machine built, PostMeetingPrompt component exists |
| Transcript auto-detection | Yes | Untested | check_for_transcript() in capture.rs, never exercised |
| Fallback prompt when no transcript | Yes | Untested | FallbackReady state + post-meeting-prompt-fallback event |
| Quick note capture in <10 seconds | Yes | Untested | PostMeetingPrompt fallback variant built |
| Actions carry forward to next day | Yes | Partial | SQLite persists across days; carryover logic is in briefing prep |
| Impact logged (wins, risks) | Partial | No | PostMeetingPrompt captures wins/risks but no impact log exists |

**Verdict: Job 3 is split.** Archive (the mechanical part) works perfectly. Post-meeting capture (the intelligent part) is fully coded but completely untested. Impact logging has no persistent store beyond individual captures.

---

### JTBD Measurable Goals Scorecard

| Goal (from JTBD.md) | Target | Current Status |
|---------------------|--------|---------------|
| Meeting prep time: 30 min → 5 min | 5 min | Likely achieved — briefing generates prep automatically. Not formally measured. |
| Action drop rate: -80% | 80% reduction | Unknown — actions extract from briefing, but post-enrichment pipeline untested. No baseline measurement. |
| Inbox processing: Automatic | 0 manual runs | Not achieved — file watcher works but full auto-processing requires Claude Code always available. |
| System maintenance time: Zero | 0 min/day | Mostly achieved — user doesn't run commands. Config changes still require JSON editing. |

---

## Part 2: PRD Feature Completeness

### F1: Morning Briefing

| Acceptance Criteria | Status |
|--------------------|--------|
| Briefing runs at configured time without user intervention | **Done** |
| Dashboard shows complete day context when opened | **Done** |
| Meeting cards display prep inline (expandable) | **Done** |
| Action items appear in dedicated panel with due dates | **Done** |
| System tray notification indicates completion | **Done** |
| Clicking notification opens dashboard | **Done** |

**Score: 6/6.** F1 is complete.

### F2: Post-Meeting Capture

| Acceptance Criteria | Status |
|--------------------|--------|
| Prompt appears 5 minutes after meeting end (configurable) | **Built, untested** |
| Only prompts for customer/external meetings | **Built** — `should_prompt()` checks meeting type |
| Capture completes in under 10 seconds | **Built, untested** |
| "Skip" is prominent and guilt-free | **Built** |
| Captured data persists to appropriate files | **Partial** — SQLite capture exists, file persistence unclear |
| Prompt doesn't appear if user is in another meeting | **Built** — `current_in_progress.is_empty()` check |

**Score: 1/6 verified, 5/6 built.** F2 needs end-to-end testing with real calendar data.

### F3: Background Automation

| Acceptance Criteria | Status |
|--------------------|--------|
| Archive runs automatically at configured time | **Done** |
| No user prompt or confirmation needed | **Done** |
| Process is silent (no notification unless error) | **Done** |
| Files remain accessible in archive structure | **Done** |

**Score: 4/4.** F3 is complete.

### F4: Processing Queue (Active Inbox)

| Acceptance Criteria | Status |
|--------------------|--------|
| File watcher detects new files within 30 seconds | **Done** |
| Quick processing completes within 1 minute | **Built, untested** |
| Full processing batches run on schedule | **Built** — InboxBatch in scheduler |
| Queue panel shows real-time status | **Done** — InboxPage |
| Review items have clear resolution flow | **Partial** — manual process/enrich buttons, no guided flow |
| Completed items clear from queue after 24 hours | **Not built** |

**Score: 2/6 verified, 3/6 built, 1/6 missing.** The "processing history" / auto-clear feature from I6 in RAIDD would close the gap.

### F5: Weekly Planning

| Acceptance Criteria | Status |
|--------------------|--------|
| Data prepared in background before user arrives | **Built** — prepare_week.py + Rust workflow |
| Prompt appears when user opens app on Monday | **Not built** — wizard exists but no auto-prompt |
| Priority selection uses visual cards, not text input | **Built** — PriorityPicker.tsx |
| Week overview shows calendar + alerts visually | **Built** — WeekOverviewStep.tsx |
| Focus block creation is opt-in with clear toggles | **Built** — FocusBlocksStep.tsx |
| Entire flow completes in under 2 minutes | **Untested** |
| Skipping any step is easy and guilt-free | **Built** — skip_week_planning command exists |

**Score: 0/7 verified, 6/7 built, 1/7 missing.** The weekly planning wizard components are all built but the auto-prompt trigger is missing. The WeekPage route was removed in 1.5a and needs to be re-added.

### F6: System Tray Presence

| Acceptance Criteria | Status |
|--------------------|--------|
| App runs in system tray (menubar on macOS) | **Done** |
| Left-click opens dashboard window | **Done** |
| Right-click shows quick actions menu | **Done** |
| Notifications appear for completed workflows | **Done** |
| Badge shows count requiring attention | **Not built** |

**Score: 4/5 verified, 1/5 missing.** Tray badge (unread/attention count) was never implemented.

### F7: Dashboard

| Acceptance Criteria | Status |
|--------------------|--------|
| Dashboard loads in under 1 second | **Done** |
| Real-time updates when backing files change | **Partial** — updates on workflow-completed events, not file changes |
| Meeting cards expand/collapse smoothly | **Done** |
| Actions can be checked off directly | **Done** — via ActionsPage, not directly on dashboard |
| Processing queue shows live status | **Partial** — InboxPage separate, not embedded queue panel |
| Click-through to files opens in default editor | **Not built** |
| Responsive to window resize | **Done** |

**Score: 3/7 verified, 2/7 partial, 2/7 missing.** The PRD envisioned an embedded processing queue panel on the dashboard — what exists is a separate Inbox page. The "click-through to files" feature was never built.

---

## Part 3: What's Actually Built vs. What Docs Claim

### IMPLEMENTATION.md Checkbox Audit

The implementation doc has aggressive checkbox marking. Here's an honest reassessment:

| Phase | Checkboxes Marked Done | Honestly Done | Honestly "Built, Untested" |
|-------|----------------------|---------------|---------------------------|
| Phase 0 | 6/6 | 6/6 | 0 |
| Phase 1A | 6/6 | 6/6 | 0 |
| Phase 1B | 5/5 | 5/5 | 0 |
| Phase 1C | 8/8 | 8/8 | 0 |
| Phase 1D | 4/4 | 4/4 | 0 |
| MVP Complete | 5/6 | 5/6 | 0 |
| Phase 1.5a-d | 13/13 | 13/13 | 0 |
| Phase 2 Pre-work | 6/6 | 4/6 | 2 (2.0a JSON migration, 2.0f unknown research) |
| Phase 2A | 4/4 | 4/4 | 0 |
| Phase 2B | 3/3 | 2/3 | 1 (queue real-time status) |
| Phase 2C | 4/4 | 2/4 | 2 (batch processing, review flow) |
| Phase 3A-C | 0/all | 0/all | Most items built as code, none verified |

**Key finding:** Phase 1 through 1.5 is genuinely done. Phase 2 Pre-work through 2C is mostly done but some checkboxes were marked without end-to-end verification. Phase 3 checkboxes are correctly unchecked even though most backend code exists.

### ROADMAP.md Acceptance Criteria

| Phase | Criteria Listed | Verified | Built But Unverified | Not Built |
|-------|---------------|----------|---------------------|-----------|
| Phase 1 | 8 | 7 | 0 | 1 (7-day crash-free) |
| Phase 2 | 6 | 2 | 3 | 1 (auto queue clear) |
| Phase 3 | 6 | 0 | 5 | 1 (weekly planning auto-prompt) |
| Phase 4 | 7 | 0 | 0 | 7 |

---

## Part 4: RAIDD Open Issues Assessment

| Issue | Priority | Assessment | Recommendation |
|-------|----------|-----------|----------------|
| **I1**: Config dir `.daybreak` → `.dailyos` | Low | Already resolved per MEMORY.md | **Close** — mark as done |
| **I2**: Compact meetings.md format | Low | Nice-to-have for dashboard dropdowns | **Defer** to post-v1 polish |
| **I3**: Browser extension for web capture | Low | Interesting but premature | **Defer** to Phase 5+ |
| **I4**: Motivational quotes | Low | Personality layer, not functional | **Defer** to post-v1 polish |
| **I5**: Orphaned pages | Medium | Resolved — closed | **Already closed** |
| **I6**: Processing history page | Low | `processing_log` table exists; just needs UI | **Include** in Phase 2 wrap-up |
| **I7**: Settings can't change workspace path | Medium | Missing Principle 3 (Buttons, Not Commands) | **Include** in Phase 2 wrap-up |
| **I8**: No update/distribution mechanism | Medium | Blocks any public distribution | **Decide** before any external release |

### RAIDD Risks

| Risk | Status | Assessment |
|------|--------|-----------|
| **R1**: Claude Code PTY issues | Open | Not yet tested across machines. Real risk for distribution. |
| **R2**: Google API token expiry | Open | Auth flow built including re-auth detection (get_google_auth_status re-checks disk). Needs real-world validation. |
| **R3**: File watcher unreliability | Open | No evidence of issues yet. Periodic backup polling not implemented. |
| **R4**: Scheduler drift on sleep/wake | Open | Time-jump detection built. Needs validation after laptop lid close/open. |

### RAIDD Assumptions

| Assumption | Validated? | Assessment |
|-----------|-----------|-----------|
| **A1**: Users have Claude Code installed | No | **Critical gap.** No detection or graceful fallback in app. User sees nothing if Claude Code isn't installed. |
| **A2**: Workspace follows PARA structure | No | Parser handles basic cases but no validation or setup. |
| **A3**: `_today/` files use expected format | Partial | JSON loader works for JSON format. Markdown fallback removed per DEC4. |

---

## Part 5: Honest Gap Analysis

### What's genuinely working (daily-driveable)

1. Morning briefing runs on schedule
2. Dashboard renders overview, meetings, actions, emails
3. System tray with open/run-now/quit
4. Archive at midnight
5. File watcher detects inbox changes
6. Actions page with filters, mark-complete
7. Inbox page shows files, manual process/enrich
8. Settings with Google auth, capture toggle, schedules
9. Profile-aware sidebar (CS vs General)

### What's built but never used in anger

1. **Post-enrichment hooks** — actions to SQLite, markdown checkbox sync. Just committed. Zero production runs.
2. **Transcript detection** — capture.rs state machine. Just committed. Never tested with a real transcript drop.
3. **Post-meeting capture** — UI exists, backend exists, events wired. Never triggered by a real meeting ending.
4. **Calendar polling** — google.rs poller runs. Never validated with real OAuth tokens in daily use.
5. **Weekly planning wizard** — 4 components, backend hooks, Python scripts. Never completed a real planning flow.
6. **Batch inbox processing** — InboxBatch scheduled but unclear if full pipeline works unattended.

### What's genuinely missing

1. **Tray badge** — Attention count on tray icon (F6 acceptance criteria)
2. **Dashboard processing queue** — PRD shows embedded queue panel; actual implementation is separate Inbox page
3. **Click-through to files** — Open file in default editor from dashboard (F7 acceptance criteria)
4. **Auto-prompt on Monday** — Weekly planning wizard exists but nothing triggers it automatically
5. **Processing history UI** — SQLite has the data (I6), no frontend
6. **Workspace path picker** — Settings can't change workspace (I7)
7. **Completed items auto-clear** — Processed inbox items persist indefinitely
8. **A1 validation** — No Claude Code installation detection or onboarding
9. **Week page route** — Removed in 1.5a, needs re-addition for Phase 3C

---

## Part 6: What Comes Next — Proposal

### Option A: "Validate Before Expanding" (Recommended)

**Philosophy:** Stop building new features. Validate what exists. Close the gap between "code committed" and "feature working."

**Duration:** 2-3 weeks

#### Sprint 1: Stability & E2E Validation (1 week)

1. **Complete 7-day crash-free validation** — MVP acceptance criterion
2. **Google OAuth E2E test** — Connect real account, validate calendar polling, token refresh
3. **Inbox processing E2E test** — Drop 5 real transcripts, verify: classify → enrich → route → extract actions → post-enrichment hooks
4. **Post-meeting capture E2E test** — Have a real meeting, verify: calendar detects end → transcript detection → fallback prompt
5. **Fix any bugs found** — Real usage will surface real issues

#### Sprint 2: Close Acceptance Gaps (1 week)

1. **Re-add Week page route** — Wire WeekPage.tsx back to router, link from sidebar
2. **Weekly planning auto-prompt** — Trigger wizard when app opens on Monday
3. **Tray badge** — Show count of inbox items needing attention
4. **Processing history UI** — Wire existing `get_processing_log` to frontend (I6)
5. **Workspace path picker** — Tauri file dialog in Settings (I7)

#### Sprint 3: Doc Accuracy & Polish (0.5 week)

1. **Update IMPLEMENTATION.md** — Honest checkbox status based on E2E validation
2. **Update ROADMAP.md** — Reflect what's actually complete
3. **Update MVP.md** — Close out validation period
4. **Close RAIDD items** — I1 (done), I5 (done), update I6/I7 status

**What this achieves:**
- Honest "Phase 2 Complete" and "Phase 3 nearly complete" status
- Every existing feature validated with real-world usage
- Documentation matches reality
- Clear foundation for Phase 4 extension work

---

### Option B: "Push to Phase 4"

**Philosophy:** Phase 2/3 code is written and compiles. Start extension architecture.

**Why I don't recommend this:** Phase 4 (extensions, MCP) depends on the post-enrichment engine working reliably. That engine was just committed and has zero production runs. Building extension hooks on top of unvalidated infrastructure compounds risk. The extension architecture (DEC26) is beautifully designed on paper — but the foundation it sits on hasn't been load-tested.

---

### Option C: "Ship What Works, Defer What Doesn't"

**Philosophy:** Declare MVP+Phase 2 complete for the parts that work. Park Phase 3 features as "experimental."

**Concretely:**
- Ship: Briefing, Dashboard, Archive, File Watcher, Quick Processing, Actions, Inbox, Settings
- Experimental: Calendar, Post-Meeting Capture, Weekly Planning
- Deferred: Everything else

**Why this is tempting but wrong:** The "experimental" features are actually the differentiators. Without calendar awareness and post-meeting capture, DailyOS is a prettier version of the CLI. The JTBD is "EA for everyone" — an EA that doesn't know when your meetings end isn't much of an EA.

---

## Part 7: Prioritization Rationale

**Why validation first?**

The JTBD framework says: "What makes someone hire DailyOS?" The hiring criteria are:

1. **Zero friction** — Runs automatically ← Achieved for briefing, unproven for inbox
2. **Zero maintenance** — Updates itself ← Achieved (app-native), but A1 assumption is unvalidated
3. **Zero guilt** — Works even when forgotten ← Achieved by design
4. **Active processing** — Inbox doesn't pile up ← **Unproven.** This is the biggest gap.
5. **Full ownership** — Local files ← Achieved
6. **Polished consumption** — Nice UI ← Achieved

Hiring criterion #4 is the one that matters most (JTBD.md says Job 2 is "the most critical job") and it's the one with the least validation. Every line of code for it exists. It just hasn't been exercised.

The cheapest way to close this gap is running the existing code against real data and fixing what breaks — not writing more code.

---

## Part 8: Decision Needed

After reviewing everything, there's one question that shapes what comes next:

**Are we building for ourselves (dogfooding), or are we building toward external release?**

- **If dogfooding:** Option A is clearly right. Validate everything, fix bugs, use it daily for 2 weeks. The features exist — make them work.
- **If external release:** Option A is still right, plus I8 (distribution mechanism) becomes blocking. Need code signing, DMG packaging, and update mechanism before anyone else can use this.

**My recommendation:** Option A, focused on dogfooding. Use DailyOS with real Google account, real transcripts, real meetings for 2 weeks. Fix everything that breaks. Then decide whether to push toward Phase 4 (extensions) or toward distribution (I8).

---

## Appendix: Feature Inventory

### Rust Backend (25 modules)

| Module | Phase | Status |
|--------|-------|--------|
| main.rs, lib.rs | 0 | Stable |
| state.rs, types.rs, error.rs | 0 | Stable |
| commands.rs | 0-3 | Stable, growing |
| json_loader.rs, parser.rs | 1B | Stable |
| scheduler.rs | 1C | Stable |
| executor.rs | 1C | Stable |
| pty.rs | 1C | Stable, R1 risk |
| notification.rs | 1C | Stable |
| workflow/today.rs | 1C | Stable |
| workflow/archive.rs | 1D | Stable |
| workflow/week.rs | 3C | Built, untested |
| watcher.rs | 2A | Stable |
| processor/mod.rs | 2B | Stable |
| processor/classifier.rs | 2B | Stable |
| processor/router.rs | 2B | Stable |
| processor/enrich.rs | 2C | Built, partially tested |
| processor/hooks.rs | 2C+ | Built, untested |
| db.rs | 2 Pre | Stable |
| google.rs | 3.0/3A | Built, needs E2E validation |
| capture.rs | 3B | Built, untested |
| workflow/mod.rs | 1C | Stable |

### Frontend (9 hooks, 9 pages, 40+ components)

| Area | Count | Status |
|------|-------|--------|
| Dashboard components | 12 | Stable |
| WeeklyPlanning components | 4 | Built, untested |
| PostMeetingPrompt | 1 | Built, untested |
| Layout (sidebar, command menu) | 2 | Stable |
| UI primitives (shadcn) | 15+ | Stable |
| Hooks | 9 | Mostly stable; useWeekPlanning, usePostMeetingCapture, useCalendar untested |
| Pages | 9 | Mostly stable; WeekPage/FocusPage/EmailsPage are orphaned or drill-down |

---

*This assessment is a snapshot. Update when validation sprints complete.*
