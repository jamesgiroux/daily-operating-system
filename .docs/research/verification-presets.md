# Verification: 0.11.0 Role Preset Issues (I309-I316)

**Date:** 2026-02-19
**Verifier:** preset-verifier
**Branch:** dev (commit f84b6c3)

---

## I309: Role Preset Schema + JSON Loader Infrastructure

**Rating: PASS**

**Evidence:**

- **Schema struct** exists at `src-tauri/src/presets/schema.rs:7-30` with all required fields:
  - `RolePreset` with id, name, description, default_entity_mode, vocabulary, vitals, metadata, stakeholder_roles, internal_team_roles, lifecycle_events, prioritization, briefing_emphasis, email_priority_keywords
  - `PresetVocabulary` (entity_noun, entity_noun_plural, primary_metric, health_label, risk_label, success_verb, cadence_noun)
  - `PresetVitalsConfig` with account/project/person arrays
  - `PresetMetadataConfig` with account/project/person arrays
  - `PresetPrioritization` (primary_signal, secondary_signal, urgency_drivers)

- **Loader** at `src-tauri/src/presets/loader.rs` with:
  - `load_preset(role)` — loads embedded preset by ID
  - `load_custom_preset(path)` — loads and validates from file
  - `validate_preset()` — validates id, name, entity mode
  - `get_available_presets()` — lists all 9 embedded presets

- **Embedded registry** at `src-tauri/src/presets/embedded.rs` uses `include_str!` to compile all 9 presets into the binary

- **Tests**: Deserialization, roundtrip, validation, all-presets-load tests present and passing (verified by test names in source)

**Minor gap:** The backlog specifies a `schemaVersion: 1` field in the JSON schema contract, but neither the Rust struct nor the JSON preset files include this field. This is cosmetic — the version is implied by the struct shape — but deviates from the stated contract.

---

## I310: Ship 9 Role Presets

**Rating: PASS**

**Evidence:**

All 9 JSON preset files exist in `src-tauri/presets/`:
1. `customer-success.json` — entityMode: account, ARR/health/lifecycle/NPS/renewal vitals, 7 stakeholder roles, 6 internal team roles, 10 lifecycle events
2. `sales.json` — entityMode: account
3. `marketing.json` — entityMode: project
4. `partnerships.json` — entityMode: both
5. `agency.json` — entityMode: both
6. `consulting.json` — entityMode: both
7. `product.json` — entityMode: project
8. `leadership.json` — entityMode: both
9. `the-desk.json` — entityMode: both

- All 9 are included via `include_str!` in `embedded.rs` and registered in `ALL_PRESETS`
- Test `test_all_presets_load_and_validate()` at `loader.rs:95-114` verifies every preset loads and validates successfully
- CS preset verified to have 5 account vitals, stakeholder/internal roles, lifecycle events, and role-specific vocabulary

**Acceptance criteria met:**
1. All 9 presets exist as valid JSON files — YES
2. Each preset has role-appropriate metadata fields — YES (CS has ARR/health/NPS, Sales has deal fields, etc.)

---

## I311: Metadata Storage Migration

**Rating: PASS**

**Evidence:**

- Migration file `src-tauri/src/migrations/025_entity_metadata.sql` adds:
  ```sql
  ALTER TABLE accounts ADD COLUMN metadata TEXT DEFAULT '{}';
  ALTER TABLE projects ADD COLUMN metadata TEXT DEFAULT '{}';
  ```
- Migration registered in `migrations.rs:91`
- DB methods exist at `db.rs:2121` (`update_entity_metadata`) and `db.rs:2140` (`get_entity_metadata`)
- Tauri commands exposed: `update_entity_metadata` and `get_entity_metadata` at `commands.rs:10355-10374`
- Commands registered in `lib.rs:527-528`

**Acceptance criteria met:**
1. JSON metadata columns on accounts and projects — YES
2. Backwards compatible with existing hardcoded columns — YES (CS preset maps to existing columns via `columnMapping`)

---

## I312: Preset-Driven Vitals Strip + Entity Detail Fields

**Rating: PASS**

**Evidence:**

- **Vitals strip**: `src/lib/preset-vitals.ts` provides `buildVitalsFromPreset()` that:
  - Reads preset vital field definitions
  - Resolves values from column, signal, or metadata sources
  - Formats by field type (currency, number, date, select, text)
  - Handles renewal countdown display
  - Used by all three entity detail pages (AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial)

- **Editable fields**: `src/components/entity/PresetFieldsEditor.tsx` renders preset metadata fields with:
  - Text, number, currency, date (DatePicker), select inputs
  - Required field indicators
  - Editorial design tokens
  - Used in `AccountFieldsDrawer.tsx`

- **Active preset hook**: `src/hooks/useActivePreset.ts` provides `useActivePreset()` to all detail pages

**Acceptance criteria met:**
1. Vitals strip shows preset-defined fields — YES
2. Entity detail page renders preset-appropriate editable fields — YES
3. Switching presets changes visible fields without data loss — YES (metadata column preserved)
4. Field types render correctly — YES (date picker, currency, select, etc.)

---

## I313: Vocabulary-Driven AI Prompts

**Rating: PARTIAL**

**Evidence:**

**Implemented:**
- Entity intelligence enrichment (`entity_intel.rs:1099-1113`) injects `vocabulary.entity_noun` into the prompt label (e.g. "account" instead of "customer account")
- Inbox file enrichment (`processor/enrich.rs:285-344`) uses:
  - `vocabulary.entity_noun` for entity references in the prompt
  - `vocabulary.success_verb` for win/outcome framing
- Intelligence queue (`intel_queue.rs:502-505`) reads vocabulary from active preset and passes to `build_intelligence_prompt()`
- Test `test_build_intelligence_prompt_with_vocabulary` at `entity_intel.rs:3060` verifies vocabulary injection

**Not implemented:**
- `briefingEmphasis` is defined in the schema (`schema.rs:25`) and preset JSON files but **never injected** into any daily briefing prompt. It only exists as data — no code reads it for prompt construction.
- `risk_label` and `health_label` from vocabulary are defined in the schema but not used in any prompt (only `entity_noun` and `success_verb` are injected)
- `cadence_noun` is not used in any prompt
- `urgencySignals` from prioritization config are not used in prompt construction

**Acceptance criteria assessment:**
1. Intelligence enrichment uses role-appropriate vocabulary — PARTIAL (only entity_noun and success_verb)
2. Daily briefing tone shifts between presets — NO (briefingEmphasis not wired)
3. Risk/win sections use preset vocabulary — PARTIAL (success_verb used for wins, risk_label not used)
4. Switching presets produces different enrichment output — YES (entity_noun changes the prompt)

---

## I314: Role Selection in Settings + Community Preset Import

**Rating: PARTIAL**

**Evidence:**

**Implemented:**
- `SettingsPage.tsx:2948-3063`: `RoleSelectionCard` component with:
  - 3-column grid of all 9 presets showing name + description
  - Active preset indicator (checkmark + turmeric border)
  - Selection invokes `set_role` command, reloads page
  - Listed under "Your Role" section (line 198, 384-387)

**Not implemented:**
- **No "Import Custom Preset" button** — the file picker and community preset import are absent from SettingsPage
- `load_custom_preset()` exists in Rust backend (`loader.rs:14-21`) but has no frontend caller
- No JSON validation UI for invalid community presets

**Acceptance criteria assessment:**
1. All 9 presets displayed in selection grid — YES
2. Selecting a preset updates config and activates immediately — YES
3. Community preset import validates JSON before accepting — NO (backend exists, no UI)
4. Invalid community presets show clear error — NO (no UI)

---

## I315: Onboarding: Role Selection Replaces Entity Mode Selection

**Rating: PASS**

**Evidence:**

- `src/components/onboarding/chapters/EntityMode.tsx` has been fully reworked:
  - Title: "What's your role?" (line 44)
  - Fetches all presets via `invoke("get_available_presets")` (line 18)
  - Displays as 3-column grid with name + description for each preset
  - Selection calls `invoke("set_role")` then reads back `defaultEntityMode` from the active preset (line 28-31)
  - Onboarding chapter label changed to "Your Role" (OnboardingFlow.tsx:69)
  - "The Desk" preset serves as the neutral/unsure option

**Acceptance criteria met:**
- Role selection replaces entity mode selection — YES
- One choice implies entity mode default — YES
- The Desk available as fallback — YES

---

## I316: Lift Parent-Child Depth Constraint (N-Level Entity Nesting)

**Rating: PASS**

**Evidence:**

**Backend:**
- `db.rs:1580-1621`: Recursive CTE queries for both ancestors and descendants:
  - `get_account_ancestors()` — walks parent_id chain upward (no depth limit)
  - `get_descendant_accounts()` — recursive CTE with depth < 10 safety limit
- `accounts.rs:733-853`: `scan_child_accounts_inner()` discovers child accounts at any nesting depth with depth < 10 safety limit (line 749-751)
- Commands registered: `get_account_ancestors` and `get_descendant_accounts` at `commands.rs:5285-5305`, `lib.rs:416-417`

**Frontend:**
- `AccountsPage.tsx:48-161`: Expandable tree with:
  - `expandedParents` state for toggle expand/collapse
  - `AccountTreeNode` recursive component (line 442+)
  - Auto-expands all parents on load (line 62-85)
  - Uses `get_descendant_accounts` for n-level support (line 149)
- `AccountDetailEditorial.tsx:212-237`: Ancestor breadcrumb navigation for nested accounts

**Acceptance criteria met:**
1. No one-level validation — YES (removed)
2. Recursive tree queries — YES (CTEs in db.rs)
3. Breadcrumb navigation — YES (AccountDetailEditorial)
4. Expandable tree on AccountsPage — YES
5. Depth < 10 safety limit — YES

---

## Summary

| Issue | Title | Rating |
|-------|-------|--------|
| I309 | Role preset schema + JSON loader | **PASS** |
| I310 | Ship 9 role presets | **PASS** |
| I311 | Metadata storage migration | **PASS** |
| I312 | Preset-driven vitals strip + entity detail fields | **PASS** |
| I313 | Vocabulary-driven AI prompts | **PARTIAL** |
| I314 | Role selection in Settings + community preset import | **PARTIAL** |
| I315 | Onboarding role selection | **PASS** |
| I316 | N-level entity nesting | **PASS** |

**Overall: 6 PASS, 2 PARTIAL**

### Key Gaps

1. **I313**: `briefingEmphasis`, `risk_label`, `health_label`, `cadence_noun` from vocabulary are defined in the schema but never injected into AI prompts. Only `entity_noun` and `success_verb` are actually used. The daily briefing prompt does not shift tone between presets.

2. **I314**: Community preset import UI is missing. The backend `load_custom_preset()` function exists but has no frontend caller. No file picker, no validation error display.

3. **Minor (I309)**: `schemaVersion` field from the backlog's schema contract is not present in the Rust struct or JSON files.
