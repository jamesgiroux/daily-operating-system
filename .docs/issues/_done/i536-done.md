# I536 — Dev Tools Mock Data Migration for v1.0.0 Schema

**Priority:** P1
**Area:** Backend / Dev Tools
**Version:** v1.0.0 (Phase 2a — after schema changes land, before frontend surfaces)
**Depends on:** I511 (schema decomposition), I508a (intelligence schema types), I508b (enrichment schema), I499 (health scoring), I503 (health schema), I512 (ServiceLayer)

## Problem

The dev tools mock data system (`devtools/mod.rs`, ~3700 lines) seeds 11 phases of data into the old schema: `entity_intelligence`, `entity_people` + `account_team`, `meetings_history`. v1.0.0 Phase 1 (I511) decomposes these tables. Phase 2 (I508a/I508b, I499, I503) redesigns intelligence with 6 dimensions and algorithmic health scoring. After those changes land, the mock data is broken — it seeds tables that no longer exist in the same shape.

Beyond the schema mismatch, the mock data has accumulated cruft:

1. **Workspace file writes are obsolete.** `write_workspace_markdown()` writes `intelligence.json`, `dashboard.md`, prep JSONs to `_today/data/`. Post-I513, the app reads from DB only. These writes create false test conditions — the app appears to work because mock files exist, hiding bugs where the DB read path is broken.

2. **Scenarios overlap confusingly.** `mock_full` vs `mock_enriched` vs `simulate_briefing` — what's the difference? An engineer new to the project can't tell. `mock_enriched` is `mock_full` plus intelligence. `simulate_briefing` is `mock_full` plus directives. The naming doesn't communicate intent.

3. **Intelligence data is flat.** The seeded `entity_intelligence` rows use the old format: a single `executive_assessment` string, flat `risks_json`, no dimensions, no structured health scoring, no competitive context, no engagement cadence, no commercial context. Post-I508, this is a hollow shell.

4. **No signal variety.** 6 email signals, zero intelligence feedback rows, zero `signal_events` variety. The Bayesian scoring system (I530) and surfacing threshold model (I532) can't be tested with this data.

5. **Prep data uses old format.** Fixture templates (`prep-acme.json.tmpl`, etc.) use the pre-I511 `FullMeetingPrep` format with `quickContext`, `strategicPrograms`, etc. Post-I511, prep lives in `meeting_prep` table with a different structure.

6. **Purge system uses hardcoded ID patterns.** `purge_mock_data()` deletes by LIKE patterns (`acme-corp%`, `act-sow-%`). If mock IDs change, purge breaks silently.

## Design

### 1. Schema-aligned seed phases

Replace the 11-phase `seed_database()` with phases that match the v1.0.0 schema:

| Phase | Old Table(s) | New Table(s) | What Changes |
|-------|-------------|-------------|--------------|
| 0: Accounts | `accounts` | `accounts` | Same structure, add `lifecycle` values from I499 set |
| 1: Entities + Projects | `entities`, `projects` | Same | No change |
| 2: Stakeholders | `entity_people` + `account_team` | `account_stakeholders` | Merge into single table. Add `data_source` per ADR-0098. |
| 3: People | `people` | `people` | Add `enrichment_sources` JSON. |
| 4: Meetings | `meetings_history` | `meetings` + `meeting_prep` + `meeting_transcripts` | Decompose. Prep content goes to `meeting_prep`. Transcript paths go to `meeting_transcripts`. |
| 5: Actions | `actions` | `actions` | No structural change. |
| 6: Intelligence | `entity_intelligence` | `entity_assessment` + `entity_quality` | Full 6-dimension intelligence per I508. Health scores per I499. |
| 7: Relationships | `person_relationships` | Same | No change. |
| 8: Signals | `signal_events` + `email_signals` | Same + `intelligence_feedback` + `signal_weights` | Add signal variety. Seed intelligence feedback (I529). Seed source weights (I530). |
| 9: User Entity | `user_entity` + `user_context_entries` | Same | No change. |
| 10: Captures | `captures` | Same | No change. |
| 11: Emails | `emails` | Same | No change. |

### 2. Rich intelligence data (replaces `seed_intelligence_data()`)

Every seeded account gets full 6-dimension intelligence:

**Acme Corp** (healthy, growth candidate):
- Strategic Assessment: executive_assessment, 2 risks (minor), 3 wins, competitive_context (1 competitor), strategic_priorities (2)
- Relationship Health: 4 stakeholder insights, relationship_depth (strong champion, exec access), coverage_assessment (80% fill rate), 1 org change (new VP hire)
- Engagement Cadence: meeting_cadence (4/month, stable), email_responsiveness (responsive)
- Value & Outcomes: 3 value_delivered items, 2 success_metrics, 1 open_commitment, 0 blockers
- Commercial Context: contract_context (annual, $1.2M, auto-renew), 1 expansion_signal (platform team), renewal_outlook (high confidence)
- External Health: support_health (3 open tickets, improving trend), product_adoption (85% adoption rate)

**Globex Industries** (at-risk, saveable):
- Strategic Assessment: 3 risks (1 critical: Team B failure), 1 win, 1 competitive_context (competitor eval)
- Relationship Health: champion engaged but executive sponsor departing (org change), coverage_assessment (60% fill rate, gap: no technical lead)
- Engagement Cadence: meeting_cadence (declining, was 3/month now 1/month), email_responsiveness (slowing)
- Value & Outcomes: 1 value_delivered, 0 success_metrics, 2 open_commitments (overdue), 1 blocker (Team B integration stalled)
- Commercial Context: contract_context (annual, $800K, renewal in 45 days), renewal_outlook (moderate confidence, risk: Team B)
- External Health: support_health (8 open tickets, 2 critical, degrading trend)

**Initech** (onboarding, autopilot):
- Strategic Assessment: 0 risks, 1 win (signed), 0 competitive_context
- Relationship Health: sparse (2 contacts, no champion assigned), coverage_assessment (40% fill rate)
- Engagement Cadence: meeting_cadence (2/month, new baseline), email_responsiveness (normal)
- Value & Outcomes: 0 value_delivered (too new), 0 success_metrics, 1 open_commitment (onboarding milestones)
- Commercial Context: contract_context (annual, $350K, first term), renewal_outlook (not assessed — too early)
- External Health: empty (no support data yet — correct for onboarding)

**Health scores computed algorithmically** — seed the dimension scores that `compute_account_health()` (I499) would produce, plus the bucket classification:
- Acme: score 78, trend stable, bucket GrowthFocus
- Globex: score 42, trend declining, bucket AtRiskSaveable
- Initech: score 55, trend stable, bucket Autopilot (sparse but no red flags)

### 3. Signal and feedback data

Seed realistic signal variety:

**`signal_events`** (20+ rows):
- `meeting_completed` signals for historical meetings
- `email_received` signals for email interactions
- `entity_updated` signals from enrichment
- `intelligence_curated` signals (I530 — user deleted items, no penalty)
- `user_correction` signals (I530 — user edited items, source penalized)
- `intelligence_confirmed` / `intelligence_rejected` signals (I529 — thumbs up/down)
- `person_profile_updated` signals from hygiene
- `enrichment_stale` signals for old enrichments

**`intelligence_feedback`** (6 rows, I529):
- 2 positive (thumbs up on Acme risks, Globex stakeholder insight)
- 2 negative (thumbs down on Initech executive_assessment, Globex renewal_outlook)
- 2 neutral/replaced (user changed vote)

**`signal_weights`** (4 rows, I530):
- `glean` source: alpha 8, beta 2 (mostly reliable)
- `ai_enrichment` source: alpha 12, beta 5 (good but sometimes wrong)
- `email` source: alpha 6, beta 1 (highly reliable)
- `user_correction` source: alpha 3, beta 0 (always trusted)

### 4. Eliminate workspace file writes

Remove `write_workspace_markdown()` entirely. Post-I513, the app reads from DB. Mock data should exercise the DB read path, not bypass it with files.

**Keep only:** Entity directory creation (empty `Accounts/Acme Corp/` dirs) so user-contributed file processing can be tested if needed. No `intelligence.json`, no `dashboard.md`, no `_today/data/*.json`.

**Fixture templates:** Keep `schedule.json.tmpl` and directive templates ONLY for the `simulate_briefing` scenario (which tests the pipeline, not the app). All other scenarios seed DB only.

### 5. Scenario consolidation

Replace 6 overlapping scenarios with 4 clear ones:

| Old Scenario | New Scenario | What It Tests |
|-------------|-------------|---------------|
| `reset` | **`reset`** | First-run state. Wipes everything. Shows onboarding. |
| `mock_full` + `mock_enriched` | **`full`** | Complete app state. 6 accounts, 12 people, 25 meetings, full 6-dimension intelligence, health scores, signals, feedback. Everything the app can render. This is the "open the app and see everything working" scenario. |
| `mock_no_auth` + `mock_empty` | **`no_connectors`** | Full mock data in DB but no Google auth, no Glean. Tests offline/degraded mode (I428) and "what does the app look like without connectors" state. |
| `simulate_briefing` | **`pipeline`** | Full mock + directive fixtures written to `_today/data/`. Tests the Rust delivery pipeline processing. Only scenario that writes workspace files. |

**Onboarding scenarios unchanged** — the 7 auth override permutations are clean and well-designed.

### 6. Prep data migration

Replace fixture templates with DB-seeded `meeting_prep` rows:

```rust
// Old: write prep-acme.json.tmpl to _today/data/preps/
// New: INSERT INTO meeting_prep (meeting_id, prep_content, prep_quality, generated_at, ...)
db.execute("INSERT INTO meeting_prep ...", params![
    "mtg-acme-weekly",
    serde_json::to_string(&acme_prep_content)?,  // MeetingPrepContent struct
    "ready",  // quality
    &now,
]);
```

The `MeetingPrepContent` struct matches whatever I511 defines for the `meeting_prep` table.

### 7. Purge system cleanup

Replace hardcoded ID pattern matching with a dev-mode marker:

```rust
// Old: DELETE FROM accounts WHERE id LIKE 'acme-corp%' OR id LIKE 'globex%' ...
// New: DELETE FROM accounts WHERE id LIKE 'mock-%'
```

All mock entity IDs get a `mock-` prefix: `mock-acme-corp`, `mock-globex-industries`, `mock-act-sow-acme`, `mock-mh-acme-weekly`, etc. One `DELETE WHERE id LIKE 'mock-%'` per table cleans everything.

### 8. Panel UI cleanup

Simplify the DevToolsPanel to match the new scenarios:

```
Scenarios
  [Reset to First Run]        — wipe everything, show onboarding
  [Full Mock Data]             — complete app state with intelligence
  [No Connectors]              — full data, no auth (offline testing)
  [Pipeline Test]              — full data + directive fixtures

Onboarding
  (7 auth override buttons — unchanged)

Daily Briefing
  [Run Mechanical]             — schedule + actions + preps, no AI
  [Run Full + AI]              — same + Claude enrichment

Cleanup
  [Reset Dev Environment]      — wipe dev DB + workspace
```

Remove: "Weekly Prep" section (the mechanical/full distinction was removed — both do the same thing). Remove: latency rollups from the main panel (move to a separate "Performance" tab if needed — it clutters the scenario testing flow).

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/devtools/mod.rs` | Rewrite `seed_database()` for new schema (I511 tables). Rewrite `seed_intelligence_data()` for 6-dimension intelligence (I508). Add signal/feedback seeding (I529/I530). Remove `write_workspace_markdown()`. Consolidate scenarios (6 → 4). Update `purge_mock_data()` for `mock-` prefix IDs. |
| `src-tauri/src/devtools/fixtures/` | Update fixture templates for new prep format (I511). Remove templates that are no longer written (intelligence.json, dashboard.md). Keep directive templates for `pipeline` scenario only. |
| `src/components/devtools/DevToolsPanel.tsx` | Update scenario buttons (6 → 4). Remove Weekly Prep section. Simplify layout. Update `DevState` interface if needed. |
| `src-tauri/src/commands.rs` | Update `dev_apply_scenario` match arms for new scenario names. Remove `dev_run_week_mechanical` / `dev_run_week_full` if weekly section removed. |
| `src-tauri/src/lib.rs` | Update command registration if commands change. |

## Acceptance Criteria

1. `full` scenario seeds data into v1.0.0 schema tables: `meetings` + `meeting_prep` + `meeting_transcripts` (not `meetings_history`), `entity_assessment` + `entity_quality` (not `entity_intelligence`), `account_stakeholders` (not `entity_people` + `account_team`)
2. All 3 accounts have full 6-dimension intelligence: strategic assessment (with competitive_context, strategic_priorities), relationship health (with coverage_assessment, org_changes), engagement cadence, value & outcomes (with blockers), commercial context (with expansion_signals, renewal_outlook), external health signals
3. Health scores are seeded with algorithmic dimension scores: Acme 78/stable/GrowthFocus, Globex 42/declining/AtRiskSaveable, Initech 55/stable/Autopilot
4. `signal_events` has 20+ rows spanning 8+ signal types. `intelligence_feedback` has 6 rows (positive + negative + replaced). `signal_weights` has 4 source rows with realistic alpha/beta values.
5. `full` scenario does NOT write `intelligence.json`, `schedule.json`, or any `_today/data/` files. App renders all data from DB.
6. `pipeline` scenario writes directive fixtures to `_today/data/` for pipeline testing — this is the ONLY scenario that writes workspace files.
7. `no_connectors` scenario seeds full data but with no Google auth and no Glean. App shows cached/offline state without errors.
8. All mock entity IDs use `mock-` prefix. `purge_mock_data()` cleans with a single `WHERE id LIKE 'mock-%'` pattern per table.
9. Portfolio page in `full` scenario shows 3 accounts with bucket classification: Acme in Growth Focus, Globex in At Risk Saveable, Initech in Autopilot.
10. Meeting detail page in `full` scenario shows prep content from `meeting_prep` table — not from filesystem.
11. DevToolsPanel shows 4 scenario buttons (not 6). No "Weekly Prep" section.
12. Onboarding scenarios unchanged — all 7 auth override permutations still work.
13. After applying `full` scenario and navigating every major page (briefing, accounts, account detail, meeting detail, portfolio, actions, week, emails, settings), zero console errors and zero blank sections.

## Sequencing

This issue sits between Phase 1 (schema changes) and Phase 3 (frontend surfaces). The mock data must be updated AFTER the schema tables exist but BEFORE frontend work begins — otherwise frontend developers can't test against realistic data.

```
I511 (schema decomposition) ──→ I536 (mock data migration) ──→ I521 (frontend cleanup)
I508 (intelligence schema)  ──↗                                  I502 (health surfaces)
I499 (health scoring)       ──↗                                  I493 (account detail)
```

## Out of Scope

- Adding new mock accounts beyond the existing 6 (the current set covers the scenarios well)
- Mock data for reports (reports are generated, not seeded)
- Mock data for Glean responses (Glean is external — mock at the API boundary, not in the seed data)
- Automated mock data validation (checking that seeded data matches schema) — trust the migration
- Performance benchmarks in dev tools (latency rollups are a separate concern)
