# I551 — Success Plan: Data Model + Backend

**Version:** v1.0.0 Phase 4
**Depends on:** I499 (health scoring), I503 (health schema), I508a (intelligence schema), I512 (ServiceLayer), I554 (prompt fidelity — produces COMMITMENTS + successPlanSignals), I555 (persistence — creates captured_commitments table + success_plan_signals_json column)
**Consumers:** I552 (frontend), I553 (templates), I497 (Success Plan report)
**Type:** Feature — new data model + service layer + suggestion extraction
**Scope:** Backend only (Rust + migrations). **PTY prompt changes superseded by I554; schema for captured_commitments + success_plan_signals_json superseded by I555.**

---

## Context

The account detail page's "The Work" chapter currently shows grouped actions and upcoming meetings. CSMs need a structured success plan surface — objectives with measurable milestones, tied to account lifecycle — to track customer outcomes beyond ad-hoc action lists. The existing actions system stays; objectives provide the strategic layer above individual tasks.

Gainsight's CTA/Success Plan model validates the concept but its UX is form-heavy. DailyOS differentiates by making success plans **living documents maintained by intelligence**: AI suggests objectives from enrichment output, milestones auto-detect completion from lifecycle events, and progress feeds the health scoring engine via `signal_momentum`.

---

## Data Model

### New Tables

**`account_objectives`** — Strategic goals scoped to an account.

```sql
CREATE TABLE account_objectives (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK(status IN ('draft', 'active', 'completed', 'abandoned')) DEFAULT 'draft',
    target_date TEXT,                    -- Optional target completion
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    source TEXT NOT NULL DEFAULT 'user', -- 'user', 'ai_suggested', 'template'
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_objectives_account ON account_objectives(account_id);
CREATE INDEX idx_objectives_status ON account_objectives(status);
```

**`account_milestones`** — Measurable checkpoints within an objective.

```sql
CREATE TABLE account_milestones (
    id TEXT PRIMARY KEY,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'skipped')) DEFAULT 'pending',
    target_date TEXT,
    completed_at TEXT,
    auto_detect_signal TEXT,             -- Optional: lifecycle event type that auto-completes this milestone
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_milestones_objective ON account_milestones(objective_id);
CREATE INDEX idx_milestones_account ON account_milestones(account_id);
```

**`action_objective_links`** — Junction table linking existing actions to objectives.

```sql
CREATE TABLE action_objective_links (
    action_id TEXT NOT NULL REFERENCES actions(id) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES account_objectives(id) ON DELETE CASCADE,
    linked_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (action_id, objective_id)
);
```

### Modified Tables

**`account_events`** — Expand lifecycle event types. Remove the 4-value CHECK constraint and replace with a broader taxonomy:

```sql
-- Migration: ALTER TABLE account_events DROP CONSTRAINT (via table rebuild)
-- New CHECK:
CHECK(event_type IN (
    'renewal', 'expansion', 'churn', 'downgrade',
    'go_live', 'onboarding_complete', 'kickoff',
    'ebr_completed', 'qbr_completed',
    'escalation', 'escalation_resolved',
    'champion_change', 'executive_sponsor_change',
    'contract_signed', 'pilot_start',
    'health_review'
))
```

This expands from 4 to 16 event types. The TypeScript `AccountEventType` union already declares 10 of these — the backend catches up and adds 6 more.

### Not Changed

- **`actions` table** — Actions stay exactly as they are. Linked to objectives via `action_objective_links`, not modified.
- **`entity_context_entries` table** — Context entries stay in Appendix for now (I552 moves them to The Record on the frontend, but no schema change).

---

## Rust Types

### New Types (`src-tauri/src/db/accounts.rs` or new `src-tauri/src/db/success_plans.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountObjective {
    pub id: String,
    pub account_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,           // draft | active | completed | abandoned
    pub target_date: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,           // user | ai_suggested | template
    pub sort_order: i32,
    pub milestones: Vec<AccountMilestone>,
    pub linked_action_count: i32,
    pub completed_milestone_count: i32,
    pub total_milestone_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountMilestone {
    pub id: String,
    pub objective_id: String,
    pub account_id: String,
    pub title: String,
    pub status: String,           // pending | completed | skipped
    pub target_date: Option<String>,
    pub completed_at: Option<String>,
    pub auto_detect_signal: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveSuggestion {
    pub title: String,
    pub description: String,
    pub milestones: Vec<SuggestedMilestone>,
    pub rationale: String,              // Why AI suggests this, citing source evidence
    pub confidence: String,             // high | medium | low
    pub source_ids: Vec<String>,        // IDs of captured_commitments that contributed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedMilestone {
    pub title: String,
    pub target_date: Option<String>,
    pub auto_detect_event: Option<String>,  // Lifecycle event type if applicable
}

/// Raw commitment captured from transcript/file enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedCommitment {
    pub id: String,
    pub account_id: Option<String>,
    pub content: String,
    pub owned_by: Option<String>,       // us | them | joint
    pub success_criteria: Option<String>,
    pub target_date: Option<String>,
    pub source_type: String,
    pub source_id: Option<String>,
    pub source_label: Option<String>,
    pub consumed: bool,
    pub created_at: String,
}

/// Synthesized success plan signals from entity intelligence enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessPlanSignals {
    pub stated_objectives: Vec<StatedObjective>,
    pub mutual_success_criteria: Vec<MutualSuccessCriterion>,
    pub milestone_candidates: Vec<MilestoneCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatedObjective {
    pub objective: String,
    pub source: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub confidence: String,             // high | medium | low
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualSuccessCriterion {
    pub criterion: String,
    pub owned_by: Option<String>,       // us | them | joint
    pub status: String,                 // not_started | in_progress | achieved | at_risk
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneCandidate {
    pub milestone: String,
    pub expected_by: Option<String>,
    pub detected_from: Option<String>,
    pub auto_detect_event: Option<String>,
}
```

### TypeScript Mirror (`src/types/accounts.ts` or new `src/types/success-plans.ts`)

```typescript
export interface AccountObjective {
  id: string;
  accountId: string;
  title: string;
  description?: string;
  status: 'draft' | 'active' | 'completed' | 'abandoned';
  targetDate?: string;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
  source: 'user' | 'ai_suggested' | 'template';
  sortOrder: number;
  milestones: AccountMilestone[];
  linkedActionCount: number;
  completedMilestoneCount: number;
  totalMilestoneCount: number;
}

export interface AccountMilestone {
  id: string;
  objectiveId: string;
  accountId: string;
  title: string;
  status: 'pending' | 'completed' | 'skipped';
  targetDate?: string;
  completedAt?: string;
  autoDetectSignal?: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export type ExpandedAccountEventType =
  | 'renewal' | 'expansion' | 'churn' | 'downgrade'
  | 'go_live' | 'onboarding_complete' | 'kickoff'
  | 'ebr_completed' | 'qbr_completed'
  | 'escalation' | 'escalation_resolved'
  | 'champion_change' | 'executive_sponsor_change'
  | 'contract_signed' | 'pilot_start'
  | 'health_review';

export interface ObjectiveSuggestion {
  title: string;
  description: string;
  milestones: { title: string; targetDate?: string; autoDetectEvent?: string }[];
  rationale: string;
  confidence: 'high' | 'medium' | 'low';
  sourceIds: string[];
}
```

---

## Service Layer

All mutations go through `ServiceLayer` per I512.

### New Service Functions (`src-tauri/src/services/accounts.rs` or new `services/success_plans.rs`)

| Function | Signal | Notes |
|----------|--------|-------|
| `create_objective(account_id, title, description?, target_date?, source)` | `objective_created` | Returns new `AccountObjective` |
| `update_objective(id, fields)` | `objective_updated` | Partial update (title, description, status, target_date, sort_order) |
| `complete_objective(id)` | `objective_completed` | Sets status=completed, completed_at=now |
| `abandon_objective(id)` | `objective_updated` | Sets status=abandoned |
| `delete_objective(id)` | `objective_deleted` | CASCADE deletes milestones + links |
| `create_milestone(objective_id, title, target_date?, auto_detect_signal?)` | `milestone_created` | Returns new `AccountMilestone` |
| `update_milestone(id, fields)` | `milestone_updated` | Partial update |
| `complete_milestone(id)` | `milestone_completed` | Sets status=completed, completed_at=now. Checks if all milestones complete → auto-complete objective |
| `skip_milestone(id)` | `milestone_updated` | Sets status=skipped |
| `delete_milestone(id)` | `milestone_deleted` | |
| `link_action_to_objective(action_id, objective_id)` | — | Operational, no signal |
| `unlink_action_from_objective(action_id, objective_id)` | — | Operational, no signal |
| `reorder_objectives(account_id, ordered_ids)` | — | Bulk sort_order update |
| `reorder_milestones(objective_id, ordered_ids)` | — | Bulk sort_order update |

### Signal Integration

- `objective_completed` and `milestone_completed` signals feed the `signal_momentum` health dimension (ADR-0097). Progress on success plan milestones is positive momentum; stalled objectives (active with all milestones pending past target_date) are negative.
- Use `emit_signal_and_propagate()` for completion signals — they should trigger prep invalidation for upcoming meetings with this account.

### Milestone Auto-Detection

When `record_account_event()` fires (existing service function), check for milestones with matching `auto_detect_signal`:

```rust
// In record_account_event() — after inserting the event:
let auto_milestones = db.query(
    "SELECT id FROM account_milestones
     WHERE account_id = ? AND auto_detect_signal = ? AND status = 'pending'",
    [account_id, event_type]
);
for milestone in auto_milestones {
    complete_milestone(milestone.id);  // Cascades objective completion check
}
```

This enables milestones like "Customer goes live" to auto-complete when a `go_live` lifecycle event is recorded.

---

## AI Signal Enrichment for Success Plans

### Philosophy: Consume Existing Enrichment Output, No New PTY Calls

**Note:** The PTY prompt changes (COMMITMENTS extraction block and `successPlanSignals` entity intelligence schema) and the persistence infrastructure (`captured_commitments` table, `success_plan_signals_json` column) have been **superseded by I554 and I555** respectively. Those issues handle the prompt engineering and schema creation. This section describes how I551 **consumes** that data for objective suggestions.

I554 adds:
- COMMITMENTS extraction block to transcript/file enrichment prompts
- `successPlanSignals` section to entity intelligence enrichment JSON schema

I555 adds:
- `captured_commitments` table (staging table for raw commitment capture)
- `success_plan_signals_json` column on `entity_assessment`
- Parser for COMMITMENTS block in transcript output

### Suggestion Extraction (from enriched signals)

`get_objective_suggestions` now reads from **two sources** and merges:

1. **`entity_assessment.success_plan_signals_json`** — AI-synthesized objectives from the full entity context. These are the highest-quality suggestions because the AI had the complete picture (90 days of meetings, emails, captures, prior intelligence).

2. **`captured_commitments` WHERE `account_id = ? AND consumed = 0`** — Raw commitments captured from individual transcripts/files that haven't been incorporated into objectives yet. These supplement the synthesized view with fresh, granular signals.

**Extraction logic (`extract_objective_suggestions`):**

1. Parse `success_plan_signals_json.statedObjectives` — each becomes a candidate objective with its milestones from `milestoneCandidates`
2. Parse unconsumed `captured_commitments` — group by similarity, each cluster becomes a candidate objective
3. Fall back to existing fields if `successPlanSignals` is empty (backward compatibility): parse `success_metrics`, `open_commitments`, `risks_json` as before
4. Score candidates: explicit > inferred > extrapolated. Prefer high-confidence signals.
5. De-duplicate across sources
6. Return top 3-5 suggestions with rationale citing the source evidence

When a user accepts a suggestion (creates an objective from it), mark the corresponding `captured_commitments` rows as `consumed = 1`.

### Tauri Command

```rust
#[tauri::command]
pub async fn get_objective_suggestions(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<Vec<ObjectiveSuggestion>, String>
```

### ObjectiveSuggestion (updated type)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveSuggestion {
    pub title: String,
    pub description: String,
    pub milestones: Vec<SuggestedMilestone>,
    pub rationale: String,              // Why AI suggests this, citing source evidence
    pub confidence: String,             // high | medium | low
    pub source_ids: Vec<String>,        // IDs of captured_commitments that contributed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedMilestone {
    pub title: String,
    pub target_date: Option<String>,
    pub auto_detect_event: Option<String>,  // Lifecycle event type if applicable
}
```

```typescript
export interface ObjectiveSuggestion {
  title: string;
  description: string;
  milestones: { title: string; targetDate?: string; autoDetectEvent?: string }[];
  rationale: string;
  confidence: 'high' | 'medium' | 'low';
  sourceIds: string[];
}
```

---

## Query Layer

### Account Detail Extension

Extend the existing `get_account_detail()` query to include objectives:

```rust
pub fn get_account_objectives(db: &Connection, account_id: &str) -> Result<Vec<AccountObjective>> {
    // Query account_objectives + LEFT JOIN milestones + COUNT linked actions
    // Order by sort_order, then created_at
    // Include milestone/action counts for progress display
}
```

This is returned as part of `AccountDetail` (new field: `objectives: Vec<AccountObjective>`).

### Lifecycle Events Query

Update `get_account_events()` to handle the expanded event type taxonomy. No query changes needed — the existing query returns all event_type values. Frontend TypeScript type needs the expanded union (already specified above).

---

## Migration

Single migration file: `XXX_success_plan_tables.sql`

1. Create `account_objectives` table
2. Create `account_milestones` table
3. Create `action_objective_links` table
4. Create `captured_commitments` table
5. Add `success_plan_signals_json TEXT` column to `entity_assessment`
6. Rebuild `account_events` table with expanded CHECK constraint (SQLite requires table rebuild for CHECK changes)

Migration is additive — no data migration needed. Existing account_events rows all have valid types under the expanded constraint. Existing `entity_assessment` rows get `success_plan_signals_json = NULL` (populated on next enrichment cycle).

---

## Files

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/migrations/XXX_success_plan_tables.sql` | Migration: 4 new tables + entity_assessment column + account_events CHECK expansion |
| `src-tauri/src/db/success_plans.rs` | Query functions for objectives, milestones, action links, captured commitments |
| `src-tauri/src/services/success_plans.rs` | Service layer mutations with signal emission |
| `src-tauri/src/commands/success_plans.rs` | Tauri IPC command handlers |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/db/mod.rs` | Add `pub mod success_plans;` |
| `src-tauri/src/services/mod.rs` | Add `pub mod success_plans;` |
| `src-tauri/src/commands/mod.rs` | Add `pub mod success_plans;`, register commands |
| `src-tauri/src/db/accounts.rs` | Extend `get_account_detail()` to include objectives |
| `src-tauri/src/services/accounts.rs` | Add auto-detect milestone check in `record_account_event()` |
| `src-tauri/src/migrations.rs` | Register new migration |
| ~~`src-tauri/src/processor/enrich.rs`~~ | ~~COMMITMENTS block~~ — **superseded by I554** |
| ~~`src-tauri/src/processor/transcript.rs`~~ | ~~COMMITMENTS block~~ — **superseded by I554** |
| ~~`src-tauri/src/intelligence/prompts.rs`~~ | ~~successPlanSignals schema~~ — **superseded by I554** |
| ~~`src-tauri/src/intelligence/io.rs`~~ | ~~SuccessPlanSignals types~~ — **superseded by I554** |
| `src/types/accounts.ts` | Add `AccountObjective`, `AccountMilestone`, `ObjectiveSuggestion`, expanded `AccountEventType` |

---

## What This Is NOT

- **Not replacing actions.** Actions remain the tactical layer. Objectives are the strategic layer above them. Actions can optionally link to objectives.
- **Not a Gainsight CTA system.** No workflow automation, no assigned CSM queues, no CTA types. Objectives are simple goal trackers.
- **Not customer-visible.** Sharing/presenting success plans externally is post-1.0 scope.
- **Not adding new PTY calls.** Two existing PTY call sites are augmented with additional extraction blocks/schema fields. The enrichment pipeline runs at the same frequency — it just captures richer signal.

---

## Acceptance Criteria

1. `account_objectives`, `account_milestones`, `action_objective_links`, and `captured_commitments` tables exist after migration. `entity_assessment` has `success_plan_signals_json` column.
2. `account_events` CHECK constraint accepts all 16 event types. Existing events (renewal, expansion, churn, downgrade) preserved.
3. `create_objective` → objective appears in `get_account_detail()` response. All CRUD operations work.
4. `create_milestone` → milestone appears nested under its objective. Completing all milestones auto-completes the objective.
5. `link_action_to_objective` creates junction row. Action still appears in normal action queries. Objective shows `linkedActionCount`.
6. Recording a `go_live` lifecycle event auto-completes any pending milestone with `auto_detect_signal = 'go_live'` on that account.
7. `objective_completed` signal emitted via `emit_signal_and_propagate()`. Signal visible in `signal_events` table.
8. Transcript enrichment extracts COMMITMENTS block. Commitments appear in `captured_commitments` table with account_id, owned_by, success_criteria, and source attribution.
9. Entity intelligence enrichment produces `successPlanSignals` JSON with `statedObjectives`, `mutualSuccessCriteria`, and `milestoneCandidates`. Stored in `entity_assessment.success_plan_signals_json`.
10. `get_objective_suggestions` merges signals from `success_plan_signals_json` (synthesized) and `captured_commitments` (raw). Returns 1-5 suggestions with confidence levels and source evidence. No new PTY call triggered.
11. Accepting a suggestion marks contributing `captured_commitments` rows as `consumed = 1`. Same commitment is not re-suggested.
12. Fallback: if `successPlanSignals` is empty (account not yet re-enriched), suggestions fall back to parsing `success_metrics`, `open_commitments`, and `risks_json`.
13. Objectives ordered by `sort_order`. `reorder_objectives` and `reorder_milestones` update sort_order correctly.
14. Deleting an objective CASCADE deletes its milestones and action links. Actions themselves are NOT deleted.
15. All mutations go through ServiceLayer. No direct DB writes from command handlers.
16. `cargo test` passes. `cargo clippy -- -D warnings` clean.
17. Mock data (`seed_database` / `full` scenario) seeds objectives, milestones, and action-objective links for mock accounts. Acme: 2 active objectives with milestones (one partially complete). Globex: 1 at-risk objective with overdue milestones. Initech: 1 completed objective. At least 3 actions linked to objectives via `action_objective_links`.
18. Mock data seeds expanded lifecycle events beyond the original 4 types (at least `go_live`, `ebr_completed`, `champion_change` in addition to existing types).
19. After applying `full` mock scenario, account detail page shows objectives in The Work chapter with correct progress, milestone states, and linked action counts — no console errors, no blank sections.
