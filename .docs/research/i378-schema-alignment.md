# I378: Intelligence Schema Alignment

Field-level alignment between the three representations of entity intelligence data.

## Sources

1. **AI Prompt** — `IntelligenceJson` (Rust struct in `src-tauri/src/intelligence/io.rs`) + JSON schema in prompt (`src-tauri/src/intelligence/prompts.rs`)
2. **DB Table** — `entity_intelligence` (defined in `src-tauri/src/migrations/001_baseline.sql:203`)
3. **TypeScript** — `EntityIntelligence` (defined in `src/types/index.ts:1178`)

---

## entity_intelligence DB Schema (001_baseline.sql)

| Column | Type | Notes |
|---|---|---|
| entity_id | TEXT PK | |
| entity_type | TEXT | default 'account' |
| enriched_at | TEXT | |
| source_file_count | INTEGER | default 0 |
| executive_assessment | TEXT | |
| risks_json | TEXT | JSON array |
| recent_wins_json | TEXT | JSON array |
| current_state_json | TEXT | JSON object |
| stakeholder_insights_json | TEXT | JSON array |
| next_meeting_readiness_json | TEXT | JSON object |
| company_context_json | TEXT | JSON object |

---

## Field-by-Field Alignment

| Field | AI Produces | DB Stores | TS Type Defines | Frontend Renders | Classification |
|---|---|---|---|---|---|
| **version** | Yes (`IntelligenceJson.version`) | No | Yes (`EntityIntelligence.version`) | No | **write-only** (AI→file only, never in DB, TS declares it but no .tsx reads it) |
| **entityId** | Yes | Yes (`entity_id` PK) | Yes | No direct render | **live** (structural) |
| **entityType** | Yes | Yes | Yes | No direct render | **live** (structural) |
| **enrichedAt** | Yes | Yes | Yes | Yes (staleness display) | **live** |
| **sourceFileCount** | Yes | Yes | Yes | No render found | **write-only** (stored but never surfaced to user) |
| **sourceManifest** | Yes (file only) | **No** (comment: "Not cached in DB") | Yes (TS type exists) | **No** (.tsx never reads it) | **write-only** (AI→file, TS declares, never rendered) |
| **executiveAssessment** | Yes | Yes | Yes | Yes (AccountHero, ProjectHero, PersonHero) | **live** |
| **risks** | Yes | Yes (`risks_json`) | Yes (`IntelRisk[]`) | Yes (WatchList component) | **live** |
| **recentWins** | Yes | Yes (`recent_wins_json`) | Yes (`IntelWin[]`) | Yes (various components) | **live** |
| **currentState** | Yes | Yes (`current_state_json`) | Yes (`IntelCurrentState`) | Yes (StateOfPlay component) | **live** |
| **stakeholderInsights** | Yes | Yes (`stakeholder_insights_json`) | Yes | Yes (StakeholderGallery) | **live** |
| **valueDelivered** | Yes | **No** (comment: "Not cached in DB") | Yes (`ValueItem[]`) | **No** (.tsx never reads it) | **dead** (AI produces, stored in file, TS declares, but never rendered) |
| **nextMeetingReadiness** | Yes | Yes (`next_meeting_readiness_json`) | Yes | Yes (meeting prep context) | **live** |
| **companyContext** | Yes (initial accounts only) | Yes (`company_context_json`) | Yes (`IntelCompanyContext`) | Yes (AccountHero company overview) | **live** |
| **userEdits** | Yes (file only) | **No** (comment: "Not cached in DB") | Yes (`UserEdit[]`) | **No** (.tsx never reads it) | **write-only** (operational — protects user edits from AI overwrite, correct to keep in file only) |
| **keywords** | Yes (prompt asks for them) | **No** (stored on `accounts.keywords` / `projects.keywords`, not on entity_intelligence) | **No** (not part of EntityIntelligence) | Via account/project keyword display | **live** (but stored separately, which is correct by design) |

---

## Structural Misalignments Found

### 1. `valueDelivered` — DEAD field

- **AI produces it** in every enrichment response (both JSON and pipe-delimited formats)
- **File stores it** in intelligence.json on disk
- **DB does NOT cache it** (explicitly skipped in `get_entity_intelligence` with comment "Not cached in DB (stored in file only)")
- **TS declares it** as `ValueItem[]` in `EntityIntelligence`
- **No .tsx component renders it** — zero references to `valueDelivered` in any page or component

**Recommendation**: Remove from AI prompt to save tokens. The field was never wired to a frontend consumer. If we want to surface value delivered in the future, we can re-add it then.

### 2. `sourceManifest` — Write-only (intentional)

- AI produces it, file stores it, DB does not cache it, TS declares it, frontend never reads it
- This is operationally useful for debugging/auditing enrichment provenance
- **Recommendation**: Keep in file + TS type. Remove from prompt output (it's populated mechanically from the file list, not by AI). Already correct — the prompt does NOT ask AI to produce this; it's assembled mechanically in `parse_intelligence_response`.

### 3. `sourceFileCount` — Write-only

- Stored in both file and DB but never surfaced to the user
- **Recommendation**: Keep. Low cost, useful for diagnostics. No action needed.

### 4. `version` — Write-only

- Always hardcoded to 1, never incremented, never read by frontend
- **Recommendation**: Keep as structural placeholder. No action needed.

### 5. `userEdits` — Write-only (intentional, correct design)

- Stored in file only, not in DB — this is correct. User edits protect fields from AI overwrite during enrichment cycles. The file is the source of truth.
- TS declares it but frontend doesn't display the edit history — correct, it's an operational concern
- **Recommendation**: No action needed. Design is sound.

---

## DB → Frontend Delivery Verification

The backend delivers `EntityIntelligence` to the frontend via the `get_entity_intelligence` DB method, which reads from the `entity_intelligence` table. The frontend receives it through Tauri IPC commands like `get_account_detail`, `get_project_detail`, and `get_person_detail`.

When the DB cache is populated (via `upsert_entity_intelligence`), it stores these fields:
- entity_id, entity_type, enriched_at, source_file_count
- executive_assessment, risks_json, recent_wins_json
- current_state_json, stakeholder_insights_json
- next_meeting_readiness_json, company_context_json

When the DB cache is read (via `get_entity_intelligence`), it reconstructs `IntelligenceJson` with:
- `source_manifest: Vec::new()` — intentionally empty
- `value_delivered: Vec::new()` — intentionally empty
- `user_edits: Vec::new()` — intentionally empty

The frontend `EntityIntelligence` type declares all three fields (`sourceManifest`, `valueDelivered`, `userEdits`) but they always arrive as empty arrays from the DB path. Only the file-read path (which is used in some contexts like field updates) has the full data.

---

## Remediation Plan

| # | Action | Scope | Risk |
|---|---|---|---|
| 1 | Remove `valueDelivered` from AI prompt JSON schema | `prompts.rs` | None — field is never consumed |
| 2 | Remove `ValueItem` and `valueDelivered` from TS `EntityIntelligence` | `types/index.ts` | None — never referenced in .tsx |
| 3 | Keep `valueDelivered` in Rust `IntelligenceJson` struct for backwards compatibility with existing intelligence.json files on disk | `io.rs` | None — serde will deserialize old files fine |
| 4 | Keep `sourceManifest`, `userEdits`, `sourceFileCount`, `version` as-is | All | None — intentional design |
