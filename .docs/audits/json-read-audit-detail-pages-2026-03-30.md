# JSON Read Audit — Detail Pages + Daily Briefing

**Date:** 2026-03-30
**Requested scope:** account detail, daily briefing, meeting detail, person detail, project detail
**Methodology:** traced each page entrypoint from React hook/page into the invoked Tauri command/service, then checked whether the reachable path reads `intelligence.json` or any other JSON file from disk.

---

## ADR Context

- `.docs/issues/i652.md:30` states the intended direction clearly: frontend reads should come from DB and `stakeholder_insights_json` should be write-only context, not a display source.
- `src-tauri/src/intelligence/io.rs:1205-1209` also marks `read_intelligence_json()` as deprecated for Tauri app call sites and says new callers should use `db.get_entity_intelligence()` instead.

## Executive Summary

The five audited surfaces are not all equally problematic:

| Surface | Initial page load | `intelligence.json` content read found? | Other JSON file read found? | Notes |
|---|---|---:|---:|---|
| Account detail | `useAccountDetail()` → `get_account_detail` | No on happy path | Yes: `dashboard.json` | Also has `intelligence.json` fallback on intelligence edit path |
| Daily briefing | `useDashboardData()` → `get_dashboard_data` | No | No confirmed page-path file read | Clean on current route path |
| Meeting detail | `MeetingDetailPage` → `get_meeting_intelligence` | No | No confirmed page-path file read | Clean on load and secondary reads |
| Person detail | `usePersonDetail()` → `get_person_detail` | No | No | Has `intelligence.json` fallback on intelligence edit path |
| Project detail | `useProjectDetail()` → `get_project_detail` | No on happy path | Yes: `dashboard.json` | Also has `intelligence.json` fallback on intelligence edit path |

## Root Cause Hypothesis

The app is mid-migration to DB-first reads. Most render-time detail/dashboard paths are already DB-backed, but two legacy patterns remain:

1. Narrative fields on some detail pages still come from workspace `dashboard.json`.
2. Intelligence edit commands still keep a legacy fallback to workspace `intelligence.json` when the DB cache is missing.

---

## Findings

### 1. Account detail initial load still reads `dashboard.json`

**Status:** Confirmed

- The page loads through `useAccountDetail()`:
  - `src/hooks/useAccountDetail.ts:90-99`
- That hook invokes `get_account_detail`, whose service still resolves the account directory and reads `dashboard.json`:
  - `src-tauri/src/services/accounts.rs:1231-1247`
- The actual file read happens through `read_account_json()`:
  - `src-tauri/src/accounts.rs:518-532`
- Intelligence itself is then loaded from DB via `db.get_entity_intelligence()`:
  - `src-tauri/src/services/accounts.rs:1248-1260`

**Assessment:** This path does **not** currently read `intelligence.json` on the happy path, but it **does** read `dashboard.json` from disk during account detail render. If the intended policy is "all detail-page reads come from DB and JSON is write-only", this is still a policy violation.

### 2. Project detail initial load still reads `dashboard.json`

**Status:** Confirmed

- The page loads through `useProjectDetail()`:
  - `src/hooks/useProjectDetail.ts:47-53`
- That hook invokes `get_project_detail`, whose service reads `dashboard.json`:
  - `src-tauri/src/services/projects.rs:133-149`
- The actual file read happens through `read_project_json()`:
  - `src-tauri/src/projects.rs:320-328`
- Intelligence is loaded from DB, not `intelligence.json`:
  - `src-tauri/src/services/projects.rs:150-151`

**Assessment:** Same pattern as account detail. No confirmed `intelligence.json` read on initial load, but `dashboard.json` is still a live render dependency.

### 3. Account, person, and project detail pages still have a reachable `intelligence.json` fallback on intelligence edit flows

**Status:** Confirmed

- All three editorial pages wire `useIntelligenceFieldUpdate()`:
  - `src/pages/AccountDetailEditorial.tsx:255-260`
  - `src/pages/PersonDetailEditorial.tsx:136-141`
  - `src/pages/ProjectDetailEditorial.tsx:155-160`
- The shared hook invokes `update_intelligence_field`:
  - `src/hooks/useIntelligenceFieldUpdate.ts:29-41`
- The backend service is DB-first, but if the DB cache is missing it falls back to filesystem intelligence mutation:
  - `src-tauri/src/services/intelligence.rs:614-625`
- That fallback ultimately uses deprecated intelligence file I/O:
  - `src-tauri/src/intelligence/io.rs:1205-1213`
- The command-level docs still describe a read/modify/write file flow:
  - `src-tauri/src/commands/integrations.rs:163-167`

**Assessment:** This is the clearest remaining `intelligence.json` read path that is reachable from the audited detail pages. It is interaction-time, not initial render-time, but it is still user-reachable from those pages.

### 4. A second `intelligence.json` fallback exists in dismissal logic adjacent to the same page family

**Status:** Confirmed, but not currently proven wired from the five audited page entrypoints

- `dismiss_intelligence_item()` is DB-first but falls back to `read_intelligence_json()` when the DB row is absent:
  - `src-tauri/src/services/intelligence.rs:877-883`

**Assessment:** I did not find a direct call from the five audited page entrypoints during this pass, so I am listing this as adjacent follow-up rather than a page-path finding. It is still a relevant legacy read path in the same intelligence editing subsystem.

---

## Verified Clean Paths

### Daily briefing page

- The route uses `useDashboardData()`:
  - `src/router.tsx:481-492`
  - `src/hooks/useDashboardData.ts:59-60`
- That calls `get_dashboard_data`, which currently delegates to the dashboard service:
  - `src-tauri/src/commands/core.rs:17-22`
- The current dashboard service implementation builds from SQLite + live calendar state:
  - `src-tauri/src/services/dashboard.rs:491-1091`

**Result:** I did **not** find a confirmed `intelligence.json`, `briefing.json`, or other workspace JSON content read on the current daily briefing page load path.

### Meeting detail page

- The page loads through `get_meeting_intelligence`:
  - `src/pages/MeetingDetailPage.tsx:206-219`
- That service hydrates prep from DB fields `prep_frozen_json` and `prep_context_json`, with an explicit comment saying there is no disk prep fallback:
  - `src-tauri/src/services/meetings.rs:257-325`
  - `src-tauri/src/services/meetings.rs:1473-1501`
- Secondary meeting detail reads are DB-backed:
  - `get_meeting_post_intelligence`: `src-tauri/src/commands/actions_calendar.rs:719-731`
  - `get_meeting_continuity_thread`: `src-tauri/src/commands/actions_calendar.rs:833-893`
  - `get_prediction_scorecard`: `src-tauri/src/commands/actions_calendar.rs:895-930`

**Result:** I did **not** find a confirmed workspace `intelligence.json` or other JSON file content read on current meeting detail page load or its secondary fetches.

### Person detail page

- The page loads through `usePersonDetail()`:
  - `src/hooks/usePersonDetail.ts:55-61`
- The service reads person, meetings, signals, captures, and intelligence from DB:
  - `src-tauri/src/services/people.rs:97-145`

**Result:** Initial person detail render appears DB-only.

---

## Adjacent Paths Worth Noting, But Not Counted As Page-Load Violations

### `_today/data/intelligence.json` is still read elsewhere, but not on the current daily briefing route

- `services/entities::load_skip_today()` reads `_today/data/intelligence.json`:
  - `src-tauri/src/services/entities.rs:403-430`
- I did not find that service wired into `useDashboardData()` / `get_dashboard_data()` for the audited daily briefing route.

### Meeting-context assembly still probes the filesystem, but I did not find a content read of `intelligence.json` on the detail-page path

- Project meeting-context assembly records a `dashboard.json` path reference:
  - `src-tauri/src/prepare/meeting_context.rs:414-417`
- Account disambiguation scores candidate paths by whether `intelligence.json` exists:
  - `src-tauri/src/prepare/meeting_context.rs:1167-1169`

These are filesystem probes / path references, not confirmed JSON content reads.

---

## Documentation Drift

These comments are now misleading relative to the code:

- `src-tauri/src/services/accounts.rs:1209-1211` says account detail "reads dashboard.json + intelligence.json", but the implementation reads `dashboard.json` and DB intelligence.
- `src-tauri/src/services/projects.rs:115-118` says project detail "reads dashboard.json + intelligence.json", but the implementation reads `dashboard.json` and DB intelligence.
- `src-tauri/src/commands/core.rs:17` still says dashboard data comes from `_today/data/` JSON files, but the current route goes through `services::dashboard::get_dashboard_data()` and builds from DB/live state.

This is not a runtime violation by itself, but it makes audits harder and masks the real remaining legacy paths.

---

## Recommended Follow-Up

1. Move account detail narrative fields (`overview`, `programs`, `notes`) off `dashboard.json` and onto DB-backed fields so `get_account_detail` becomes fully DB-read-only.
2. Move project detail narrative fields (`description`, `milestones`, `notes`) off `dashboard.json` and onto DB-backed fields so `get_project_detail` becomes fully DB-read-only.
3. Remove the legacy file fallback from `update_intelligence_field()` and sibling intelligence-edit commands so page interactions cannot read `intelligence.json` when DB cache is absent.
4. Clean up stale comments after the runtime changes so future audits reflect real behavior.

## Audit Status

**DONE**

Confirmed read violations in scope:
- Account detail: `dashboard.json`
- Project detail: `dashboard.json`
- Account/person/project edit flows: legacy fallback to `intelligence.json`

Confirmed clean in scope:
- Daily briefing initial route path
- Meeting detail initial load + secondary reads
- Person detail initial load
