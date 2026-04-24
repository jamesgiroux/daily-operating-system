# DOS-75 Exploration: Linear Issues Section on Account/Project Detail Pages

**Status:** Exploration (decision doc)  
**Date:** 2026-04-23  
**Related:** DOS-52 (push-to-Linear), DOS-19 (detail page IA redesign), `.docs/design/account-detail-content-design.md`

---

## Current State

### What exists today

1. **Backend support is complete**
   - `linear_issues` table stores synced Linear issues (identifier, title, state_name, priority_label, due_date)
   - `linear_entity_links` junction table maps Linear projects to account/project entities
   - `inject_linear_issues()` in `src-tauri/src/prepare/meeting_context.rs` queries and formats the data:
     - Filters by entity_id, excludes completed/cancelled states, limits to 10 issues, orders by priority ASC then due_date ASC
     - Returns: identifier, title, state, priority, due_date
   - Query is production-ready; function only surfaces data during meeting prep

2. **Frontend infrastructure exists**
   - `src/components/shared/ActionRow.tsx` renders Linear badges on action rows (lines 304–331):
     - Shows `linearIdentifier` as a clickable link
     - Opens issue URL in browser on click
     - Styled as inline badge (tertiary color, 10px font, monospace)
   - This is the only current user-facing Linear surface outside meeting prep

3. **Data flow for pushed actions (DOS-52)**
   - When a user pushes an action to Linear, a link is created in `action_linear_links` (migration 085)
   - Actions enriched with `linear_identifier` and `linear_url` fields
   - ActionRow component renders these as inline badges on the action row

### What the user doesn't see

- Linear issues linked to an account/project are **invisible** on the entity detail page
- They only surface in **meeting prep context** (meeting briefing intelligence)
- A CS manager reviewing an account's detail page has no visibility into related engineering work

### Why it matters

From `.docs/design/account-detail-content-design.md` (line 190):
> "Role of Linear issues linked to this account?" — Design questions for next session

The detail page design explicitly flagged this as an open question for The Work tab. The gap is intentional — the answer requires answering the four exploration questions below.

---

## The Four Questions

### 1. Does this belong on entity pages?

**Pro:**
- Active engineering context is valuable to CS managers (what's being built, what's blocked, what's launching)
- Already linked at the DB level — exists but is hidden
- Supports the "forward-looking" aspect of The Work tab (what we plan to do)

**Con:**
- Could be visual noise if the team tracks hundreds of issues per project
- Engineering work rhythm (sprints, cycles) doesn't always align with CS rhythm (renewal, expansion)
- Duplication risk: if actions are pushed to Linear, they appear both as Actions and as Issues

**Data says:**
- Seed data includes 1 link: `lp-1` (Linear project) linked to `a1` (account)
- `inject_linear_issues()` hard-codes LIMIT 10, implying the team anticipated 10+ issues per entity
- Mock data comment in exploration: "10 issues across 3 projects with entity links"
- Real usage data: unknown (no telemetry on entity_links table size or issue frequency)

**Recommendation:** Yes, this belongs — but with a cap and filtering logic.

---

### 2. Read-only or interactive?

**Read-only approach:**
- Display status badge, priority label, identifier, due date
- Click identifier to open issue in Linear
- No mutations from the detail page

**Interactive approach:**
- Update issue state from the detail page
- Add comments to issues
- Manage link confirmations
- Requires two-way sync (read + write)

**Recommendation:** Start with read-only. The 5-question Intelligence Loop check (below) reveals that interactive features have zero impact on signals, health scoring, or intelligence context. Building interactivity without a signal/feedback loop would be incomplete per CLAUDE.md. If The Work tab's "forward-looking" JTBD requires state updates, that belongs to a future phase with proper signal integration.

---

### 3. Parity model: How does this interact with pushed actions?

**The overlap:**
- DOS-52: Users push Actions TO Linear (from action list) → creates issue
- DOS-75: Show Linear issues linked to entity → includes those pushed issues + others

**Merged model (dedup):**
- Single "Engineering work" chapter showing:
  - Pushed actions (our actions that are tracked in Linear)
  - Other linked issues (engineering work we didn't initiate)
  - Visually distinguishes pushed-by-us vs found-linked
- Prevents showing same issue twice
- More coherent: "here's what we're doing in Linear"

**Separate model (don't dedup):**
- Keep Actions page as-is (pushed actions only)
- New Linear Issues section on entity detail (all linked issues)
- Accept duplication: action shows in The Work + issue shows in new section
- Clearer responsibilities: Actions = "what we're tracking" vs Issues = "what's being built"

**Recommendation:** Separate model, but with a design note. Deduping would require:
- A query join against `action_linear_links` for every issue
- Extra state on the issue card (distinguishing "our action" vs "found linked")
- Complicates the Intelligence Loop feedback (which entity should get signal credit?)

Better to keep them separate and acknowledge the duplication in the design doc: "Pushed actions and linked issues may overlap; this is intentional — actions are *our work tracking*, issues are *engineering context*."

---

### 4. Intelligence Loop check: Does this feed signals/health/intelligence context/callouts/Bayesian weights?

**Question 1: Does this data emit signals?**
- No. Linear issues are read-only. No user interactions to emit.
- Could emit signals later (e.g., "issue moved to done" could trigger a celebration signal) but not in read-only first phase.
- **Answer: No, not in read-only mode.**

**Question 2: Does it feed health scoring dimensions?**
- Health dimensions: engagement, usage, champion risk, product health, commercial, expansion
- Linear issues could inform "product health" (blocked issue = technical debt?) or "engagement" (issue moved = sign of activity)
- But this would require periodic sync and scoring rules — not included in read-only display
- **Answer: Could, but not initially. No scoring rules exist.**

**Question 3: Should it appear in intelligence context?**
- `build_intelligence_context()` (intel prompts) or `gather_account_context()` (meeting prep)
- Already appears in meeting prep via `inject_linear_issues()` — called from `gather_account_context()`
- So: Yes, for meeting prep (already done). No, for intel prompts (Glean analysis doesn't need it).
- **Answer: Already covered in meeting prep.**

**Question 4: Should it trigger briefing callouts?**
- `CALLOUT_SIGNAL_TYPES` triggers alerts in briefings
- Could trigger callouts like "blocked issue X blocking your renewal timeline"
- Requires scoring rules + signal routing
- **Answer: Could, but deferred to Phase 2.**

**Question 5: Does user interaction feed Bayesian weights?**
- Users could click "this issue is relevant to renewal" or "not relevant"
- Clicks/ignores could tune source credibility
- Not applicable in read-only mode
- **Answer: No, not in read-only mode.**

**Intelligence Loop verdict:** Linear issues on entity pages, in read-only mode, are **incomplete per CLAUDE.md**: "Data stored only for frontend display — without signal emissions, health scoring input, intel context, or prep context — is incomplete."

They already have intel context (meeting prep). But the display adds zero signal, health, or callout value. The user sees engineering status, but the system doesn't learn from their engagement with it.

**Implication for scope:** Acceptable as Phase 1 (read-only display). But mark as Phase 2 entry point: signal emission, health scoring rules, and Bayesian feedback integration required before this feature is "done" per CLAUDE.md.

---

## Options

### Option A: Don't build it (do nothing)

**Rationale:**
- Engineering work is already visible in meeting prep
- Adds display-only value with zero signal/learning impact
- Design space of The Work tab is still unsettled (per content-design.md line 188–192)
- Risk: showing engineering status without context could mislead (issue in backlog != urgent)

**Cost:** Zero.

**Why we might choose this:** If The Work tab's JTBD ("What are we doing about it?") is answered entirely by pushed actions + watch programs + reports, Linear issues are redundant.

---

### Option B: Read-only chapter on The Work tab

**Approach:**
- New chapter after "Commitments" or before "Recently landed"
- Query: issues linked to entity, state NOT IN (completed, cancelled)
- Show: identifier (link), title, state badge, priority label
- Label: "Engineering work" or "From Linear"
- Density: max 8 issues visible, overflow into "show more"

**Files to touch:**
- Backend: Add `linked_issues` field to `AccountDetailResult` in `src-tauri/src/commands/accounts_content_chat.rs`
- Backend: Add Tauri command or extend `get_account_detail()` to fetch linked issues
- Frontend: New component `LinkedIssuesSection.tsx`
- Frontend: Update `buildWorkChapters()` in `account-detail-utils.ts` to add "Engineering work" chapter
- Design: Sketch in `.docs/mockups/account-detail-work-linear-section.html`

**Scope estimate:** Medium (M)
- Backend query is copy-paste from `inject_linear_issues()`
- Component is straightforward (render issue list, no state mgmt)
- IA integration is minimal (one new chapter pill)
- Testing: parity fixture update + visual regression

**Intelligence Loop impact:** Adds display value but no signal/health/callout value initially.

**Recommendation:** Ship this if The Work tab's JTBD is answered by "pushed actions + our programs + what's blocked externally." Adds 10–15 lines of rendering, low regression risk.

---

### Option C: Merged model (dedup pushed actions + linked issues)

**Approach:**
- Single "What we're building" section combining:
  - Pushed actions from `action_linear_links` (visually marked "Created by us")
  - Other linked issues (visually marked "Found")
- Unified sort: priority, due date, state
- Prevents visual duplication
- Single source of truth for "engineering status for this entity"

**Files to touch:**
- All of Option B, plus:
- Query: Join `linear_issues` + `action_linear_links` to identify "ours"
- Component: Distinguish rows by origin (badge or row color tint)
- Design: Decide visual treatment (separate groups, row markers, or subtle distinction)

**Scope estimate:** Large (L)
- Query is more complex (join, left outer match)
- Component has conditional rendering for "created by us" badge
- Design needs visual differentiation
- Risk: action duplication bug (same issue shown in Actions page AND here)

**Intelligence Loop impact:** Same as Option B.

**Recommendation:** Defer to Phase 2. The design debt isn't settled yet (do we even want to dedup?). Option B gives us the data surface; Option C refines it after we see real usage.

---

## Recommendation

**Build Option B: Read-only chapter on The Work tab.**

**Rationale:**
1. **It's discoverable but not intrusive.** The chapter sits in The Work tab's clear JTBD ("what are we doing"). It's not noise — it's answers.
2. **Matches current design philosophy.** The content-design.md explicitly asks "Role of Linear issues linked to this account?" for The Work tab. This answers it.
3. **Low regression risk.** Query exists, component is straightforward, IA integration is a single pill.
4. **Clear Phase 2 entry point.** Once shipped, the signal/health/callout integration becomes the next story (DOS-76 or DOS-77).
5. **Separates concerns cleanly.** Don't dedup; keep pushed actions and linked issues separate. Users understand the difference.

**Phasing:**
- **Phase 1 (now):** Read-only "Engineering work" chapter, 8-issue cap, on The Work tab
- **Phase 2 (later):** Signal emission (issue state changes), health scoring (blockage = risk), briefing callouts, Bayesian weight updates
- **Phase 3 (later):** Interactive features (state updates, comments) if requested

---

## Scope if we build

### Files to touch

**Backend:**
- `src-tauri/src/commands/accounts_content_chat.rs` — add `linked_issues` field to `AccountDetailResult`
- `src-tauri/src/services/accounts.rs` — query linked issues (reuse `inject_linear_issues` logic)
- Optional: new Tauri command `get_entity_linked_issues` if we want to fetch async

**Frontend:**
- `src/components/account/LinkedIssuesSection.tsx` (new) — render issue list
- `src/components/account/account-detail-utils.ts` — add `linked_issues` to `buildWorkChapters()` return
- `src/pages/AccountDetailPage.tsx` — import and render LinkedIssuesSection in The Work view
- `src/hooks/useAccountDetail.ts` — no change (data already in detail response)

**Design:**
- `.docs/mockups/account-detail-work-linear-section.html` (new) — visual spec
- `.docs/design/account-detail-content-design.md` — update "The Work tab" section with "Engineering work" chapter decision

**Tests:**
- `src/pages/AccountDetailEditorial.test.tsx` — add fixture with linked issues
- `src/parity/fixtures/parity/mock/account_detail.json` — add `linked_issues` to mock response

### Estimated effort

- Backend: 30 minutes (query + struct field)
- Frontend component: 45 minutes (render, no logic)
- Integration: 30 minutes (IA, navigation, tests)
- **Total: ~2 hours engineer time, Medium scope**

### Design direction

**Chapter label:** "Engineering work" (clear, active voice, aligns with "The Work" JTBD)

**Row format:**
```
[state-badge] PROJ-123 Issue title  [priority-label]  [due-date]
```

Example:
```
[In Progress] DAILYOS-45 API rate limit blocking integration [High] [May 12]
[Backlog] API-89 Query optimization [Medium]
```

**Styling:**
- Reuse `ActionRow` component? Or custom render to avoid extra badge noise?
- State badge: use Linear's color coding (shipped = green, in progress = blue, backlog = gray)
- Priority label: High (turmeric), Medium (charcoal), Low (disabled)
- Identifier: clickable, monospace, opens Linear in browser

**Density:** Show 8 issues, "8 more" overflow link. Reasoning: The Work tab is already dense (actions + programs + reports); don't overwhelm.

---

## Open Questions

1. **Should we cap at 8 issues or show all?** The query caps at 10; frontend could show 8 + "more". What's the UX philosophy for density on The Work tab?
   
2. **How do we visually distinguish pushed actions (Actions page) from linked issues (here)?** If Option C (merged model) is chosen later, we need a design system for "created by us" vs "found." Should we reserve that visual distinction now?

3. **Do linked issues appear in The Work tab only, or should they also appear on project detail pages?** Assuming project entities also have `linear_entity_links`; should we support those too?

4. **Signal emission rules for Phase 2:** When a linked issue's state changes, should we emit a signal? Emit to briefing? Update health scoring? Needs a separate design decision.

5. **Design refinement:** Should the chapter be collapsed by default if all issues are in backlog/low-priority? (Following the design philosophy that density = verdict.)

---

## Definition of Done

This exploration is complete when:

- [x] Current state documented (backend support exists, frontend surface doesn't)
- [x] Four questions answered with data and rationale
- [x] Three options sketched with trade-offs
- [x] One recommendation chosen with clear rationale
- [x] Intelligence Loop 5-question check documented
- [x] Scope estimated (files, effort, size)
- [x] Design direction sketched
- [x] Open questions listed
- [ ] **User review and decision** (does this match product vision?)

If the user approves Option B, the next step is **create a Linear issue** with acceptance criteria:
- Display linked issues on The Work tab
- Filter by entity, exclude completed/cancelled states
- Show: identifier (link), title, state badge, priority label, due date
- Cap at 8 visible + overflow link
- Add chapter to buildWorkChapters()
- Tests pass: parity fixtures updated, no regressions

---

## References

- `.docs/design/account-detail-content-design.md` — Design principles and outstanding questions (line 190)
- `src-tauri/src/prepare/meeting_context.rs` — `inject_linear_issues()` query (line 903)
- `src/components/shared/ActionRow.tsx` — Linear badge rendering (lines 304–331)
- `src-tauri/src/migrations/041_linear_entity_links.sql` — Schema
- `src-tauri/src/migrations/085_action_linear_links.sql` — Action-to-issue links (DOS-52)
- CLAUDE.md — Intelligence Loop 5-question check, Definition of Done
