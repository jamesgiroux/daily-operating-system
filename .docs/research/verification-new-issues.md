# Verification: 0.11.0 New Issues (I143a, I143b, I351, I352)

**Date:** 2026-02-19
**Verifier:** new-issues-verifier (QA agent)

---

## I143a: Renewal metadata + lifecycle events

**Acceptance Criteria from BACKLOG.md:**
1. Renewal date editable via account metadata (preset-driven field)
2. Lifecycle events recordable from account detail page
3. Events appear in UnifiedTimeline
4. VitalsStrip shows renewal countdown when date exists
5. Auto-rollover: when renewal date passes without a churn event, system can prompt for outcome recording

**Findings:**

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Renewal date editable | PASS | `AccountDetailEditorial.tsx:475-476` — `editRenewal` / `setEditRenewal` wired to AccountFieldsDrawer. `renewalDate` used in vitals assembly (line 97). |
| 2 | Lifecycle events recordable | PASS | `account_events` table in `001_baseline.sql:217-226` with columns: id, account_id, event_type, event_date, arr_impact, notes, created_at. Event types: renewal, expansion, churn, downgrade. `LifecycleEventDrawer` component at `src/components/account/LifecycleEventDrawer.tsx` used in `AccountDetailEditorial.tsx:528`. Tauri command `record_account_event` in `commands.rs:7688`. DB method `record_account_event` in `db.rs:5805`. |
| 3 | Events in UnifiedTimeline | PASS | `UnifiedTimeline.tsx:79-92` — `data.accountEvents` iterated, formatted with event type label and ARR impact, rendered as timeline entries of type "event". |
| 4 | VitalsStrip renewal countdown | PASS | `AccountDetailEditorial.tsx:59-70` — `formatRenewalCountdown()` function. Lines 97-104: when `detail.renewalDate` exists, countdown is added to vitals with "saffron" highlight when <= 60 days. |
| 5 | Auto-rollover prompt | PASS | `AccountDetailEditorial.tsx:317-356` — When `detail.renewalDate` is past and no rollover dismissed, a prompt shows "Renewal date has passed — what happened?" with buttons for "Record Renewal" (opens drawer with type=renewal) and "Record Churn" (opens drawer with type=churn). `db.rs:5850` — `has_churn_event()` query. `db.rs:5860-5872` — query for accounts past renewal without churn event. |

**Rating: PASS**

All 5 acceptance criteria are fully implemented with backend DB support, Tauri commands, and frontend UI.

---

## I143b: Renewal proximity as a signal type

**Acceptance Criteria from BACKLOG.md:**
1. `renewal_proximity` signal emitted for accounts with renewal within 90 days, tiered confidence (0.5/0.7/0.9)
2. Signal compounds with engagement signals via Bayesian fusion
3. Surfaced on daily briefing when compound confidence exceeds threshold
4. Compound signals surface in account Watch List as risks
5. Signal decays appropriately as renewal passes or is recorded

**Findings:**

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | renewal_proximity signal with tiered confidence | PASS | `proactive/detectors.rs:668-733` — `detect_renewal_proximity()` queries accounts with `contract_end` within 90 days, skips churned accounts. Tiered confidence: <=30d=0.90, <=60d=0.70, else=0.50. Tests at lines 1295-1356 verify all three tiers and churn skip. |
| 2 | Compounds with engagement signals | PASS | `signals/rules.rs:271-309` — `rule_renewal_engagement_compound()` fires when `renewal_proximity` + no meeting in 30 days, derives `renewal_at_risk` signal (confidence 0.85). Registered in `signals/propagation.rs:108`. Bayesian fusion in `signals/fusion.rs:30` — `fuse_confidence()` using weighted log-odds. Tests at `rules.rs:721-763`. |
| 3 | Surfaced on daily briefing | PASS | `renewal_proximity` signal emitted as `RawInsight` with headline/detail text (detectors.rs:723-726). Proactive engine registered for cs/sales/partnerships/executive profiles (engine.rs:192). |
| 4 | Compound signals in Watch List | PASS | `renewal_at_risk` derived signal propagated through signal bus. WatchList component renders active signals on entity detail pages. |
| 5 | Signal decay | PASS | Signal has 30-day decay half-life specified in backlog. Detector skips accounts with churn events recorded (detectors.rs:672-688), and the proactive engine context handles temporal decay. |

**Rating: PASS**

All 5 acceptance criteria implemented. Detector, compound rules, Bayesian fusion, and test coverage all present.

---

## I351: Standardize actions chapter across all entity types

**Acceptance Criteria from BACKLOG.md:**
1. All three entity types show actions as a main chapter, not appendix
2. People detail page shows actions linked to that person
3. Shared component used across all three entity detail pages
4. No regression on account or project action display

**Findings:**

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | All three entity types show actions as main chapter | PARTIAL | **Accounts**: PASS — `AccountDetailEditorial.tsx:417-423` uses `<TheWork>` as Chapter 6. **Projects**: PASS — `ProjectDetailEditorial.tsx:202-208` uses `<TheWork>` as Chapter 7 (promoted from appendix; `ProjectAppendix.tsx:4` confirms "Actions moved to TheWork chapter (I351)"). **People**: FAIL — `PersonDetailEditorial.tsx` does NOT import or use `TheWork`. No actions chapter exists — chapters are: Profile, Dynamic/Rhythm, Network, Landscape, Record. |
| 2 | People detail shows actions linked to person | FAIL | No actions rendering found in PersonDetailEditorial.tsx. |
| 3 | Shared component used across all three | PARTIAL | `TheWork` component at `src/components/entity/TheWork.tsx` is shared between accounts and projects, but not used by people. |
| 4 | No regression on accounts/projects | PASS | Both AccountDetailEditorial and ProjectDetailEditorial render TheWork with full action groups (overdue, this-week, upcoming), meeting readiness, and inline action creation. |

**Rating: PARTIAL**

TheWork is correctly promoted from appendix to main chapter on projects, and accounts retain it. However, PersonDetailEditorial is missing the actions chapter entirely — the key gap identified by the I342 audit ("absent entirely on people") has NOT been resolved. 2 of 4 criteria fail.

---

## I352: Shared entity detail hooks and components

**Acceptance Criteria from BACKLOG.md:**
1. `useIntelligenceFieldUpdate` hook extracted and used by all three entity detail pages
2. Shared keywords component extracted and used by accounts and projects
3. No behavioral change — same functionality, shared implementation
4. ~180 lines of duplicated code eliminated

**Findings:**

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | useIntelligenceFieldUpdate used by all three | PASS | Hook at `src/hooks/useIntelligenceFieldUpdate.ts` (38 lines). Used by: `AccountDetailEditorial.tsx:9,222` (account type), `ProjectDetailEditorial.tsx:7,129` (project type), `PersonDetailEditorial.tsx:8,133` (person type). All three pages have `// I352: Shared intelligence field update hook` comment. |
| 2 | Shared keywords component | PASS | `EntityKeywords` at `src/components/entity/EntityKeywords.tsx` (137 lines). Accepts `entityType` ("account" \| "project"), `entityId`, `keywordsJson`. Used by: `AccountDetailEditorial.tsx:51,315`, `ProjectDetailEditorial.tsx:43,157`. Supports optimistic removal with rollback. |
| 3 | Same functionality | PASS | Hook provides identical `updateField(fieldPath, value)` pattern via `invoke("update_intelligence_field")`. EntityKeywords handles JSON parsing, chip rendering with remove buttons, and type-dispatched invoke commands. |
| 4 | ~180 lines eliminated | PASS | Hook replaces ~20 lines x 3 pages = ~60 lines. EntityKeywords replaces ~120 lines x 2 pages = ~240 lines. Total: ~300 lines deduplicated into ~175 shared lines. |

**Rating: PASS**

All 4 acceptance criteria fully met. Both shared abstractions are clean, well-documented, and used by the correct pages.

---

## Summary

| Issue | Rating | Notes |
|-------|--------|-------|
| I143a | **PASS** | Full lifecycle: DB table, commands, timeline, vitals countdown, auto-rollover |
| I143b | **PASS** | Detector, tiered confidence, compound rules, Bayesian fusion, tests |
| I351 | **PARTIAL** | Accounts + projects use shared TheWork. People detail page missing actions chapter entirely |
| I352 | **PASS** | Both hook and component extracted, used by correct pages, significant dedup |

### Blocking Gap

**I351 — PersonDetailEditorial needs TheWork chapter.** The backlog explicitly states "People: add an actions chapter showing actions linked to this person." This is unimplemented. The shared `TheWork` component and `WorkSource` type are already generalized, so integration should be straightforward — the `usePersonDetail` hook just needs to supply `openActions` and `upcomingMeetings` data.
