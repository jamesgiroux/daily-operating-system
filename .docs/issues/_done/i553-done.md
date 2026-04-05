# I553 — Success Plan Templates + Starter Lifecycle Collection

**Version:** v1.0.0 Phase 4
**Depends on:** I551 (data model + backend)
**Type:** Feature — template system + built-in seeds
**Scope:** Backend + minimal frontend (template picker UI)

---

## Context

New accounts start with a blank success plan. CSMs repeatedly create the same objectives and milestones for accounts at similar lifecycle stages (onboarding, growth, renewal prep, at-risk recovery). Templates solve the cold-start problem by pre-populating objectives and milestones based on the account's lifecycle stage.

DailyOS differentiates from Gainsight by making templates **lifecycle-stage-triggered** rather than manually selected. When an account's lifecycle changes (e.g., from "onboarding" to "active"), the system can suggest the appropriate template. Templates are not auto-applied — the user chooses to apply them.

---

## Template Data Model

Templates are stored as JSON in a Rust constant (not in the database). They're built-in seeds, not user-created. User-created templates are a post-1.0 feature.

### Template Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessPlanTemplate {
    pub id: &'static str,                      // e.g., "onboarding-standard"
    pub name: &'static str,                     // "Onboarding Success Plan"
    pub description: &'static str,
    pub lifecycle_trigger: &'static str,        // lifecycle stage that triggers suggestion
    pub objectives: &'static [TemplateObjective],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateObjective {
    pub title: &'static str,
    pub description: &'static str,
    pub milestones: &'static [TemplateMilestone],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMilestone {
    pub title: &'static str,
    pub offset_days: i32,               // Days from template application to target_date
    pub auto_detect_signal: Option<&'static str>,  // Lifecycle event that auto-completes
}
```

### TypeScript Mirror

```typescript
export interface SuccessPlanTemplate {
  id: string;
  name: string;
  description: string;
  lifecycleTrigger: string;
  objectives: {
    title: string;
    description: string;
    milestones: {
      title: string;
      offsetDays: number;
      autoDetectSignal?: string;
    }[];
  }[];
}
```

---

## Built-In Template Seeds

### 1. Onboarding Success Plan

**Trigger:** `lifecycle = 'onboarding'`

| Objective | Milestones | Auto-Detect |
|-----------|-----------|-------------|
| **Technical Setup & Integration** | Kickoff completed (0d), Integration configured (14d), Go-live achieved (30d) | `kickoff`, `go_live` |
| **Initial Value Delivery** | First use case live (45d), Initial value report shared (60d), Customer confirms value (90d) | `onboarding_complete` (on last) |
| **Relationship Foundation** | Key stakeholders identified (7d), Executive sponsor confirmed (14d), Regular cadence established (30d) | `executive_sponsor_change` |

### 2. Growth & Expansion Plan

**Trigger:** `lifecycle = 'active'` (or 'growing')

| Objective | Milestones | Auto-Detect |
|-----------|-----------|-------------|
| **Deepen Product Adoption** | Usage review completed (30d), Expansion opportunities identified (45d), New use case proposed (60d) | — |
| **Expand Stakeholder Footprint** | Map additional teams (14d), Executive business review scheduled (30d), Cross-functional champions identified (60d) | `ebr_completed` |
| **Drive Measurable Outcomes** | Baseline metrics documented (14d), QBR with ROI data (90d), Case study candidate identified (120d) | `qbr_completed` |

### 3. Renewal Preparation Plan

**Trigger:** `lifecycle = 'renewing'` (or account with `contract_end` within 120 days)

| Objective | Milestones | Auto-Detect |
|-----------|-----------|-------------|
| **Secure Renewal Decision** | Renewal timeline confirmed (0d), Decision-maker engaged (14d), Proposal delivered (60d), Contract signed (90d) | `renewal`, `contract_signed` |
| **Demonstrate Value** | ROI summary prepared (14d), Customer success stories compiled (30d), Executive review completed (45d) | `ebr_completed` |
| **Mitigate Risks** | Risk assessment completed (7d), Competitive threats addressed (30d), Open issues resolved (60d) | — |

### 4. At-Risk Recovery Plan

**Trigger:** `lifecycle = 'at_risk'` (or health_band = 'red')

| Objective | Milestones | Auto-Detect |
|-----------|-----------|-------------|
| **Stabilize the Relationship** | Escalation acknowledged (0d), Recovery meeting scheduled (3d), Executive sponsor engaged (7d), Recovery plan agreed (14d) | `escalation`, `escalation_resolved` |
| **Address Root Causes** | Issues catalogued (3d), Technical blockers resolved (30d), Process gaps addressed (45d) | — |
| **Rebuild Confidence** | Quick win delivered (14d), Health review completed (30d), Regular check-in cadence restored (45d) | `health_review` |

---

## Auto-Detection: Template Suggestion

When an account's lifecycle changes (via `update_account_field` with field='lifecycle'), check if a template matches the new stage:

```rust
pub fn get_suggested_templates(lifecycle: &str, health_band: Option<&str>) -> Vec<&'static SuccessPlanTemplate> {
    TEMPLATES.iter()
        .filter(|t| {
            t.lifecycle_trigger == lifecycle
            || (t.id == "at-risk-recovery" && health_band == Some("red"))
        })
        .collect()
}
```

This is a pure function — no DB query, no AI call. The frontend can call it to show "Would you like to apply the Onboarding Success Plan?" when the lifecycle changes.

### Suggestion UI Flow

1. User changes account lifecycle (e.g., sets to "onboarding")
2. Frontend calls `get_suggested_templates` with new lifecycle
3. If templates match, show a subtle banner below the lifecycle field: "A success plan template is available for onboarding accounts. [View] [Dismiss]"
4. "View" opens a preview showing the template's objectives and milestones
5. "Apply" creates all objectives + milestones with `source = 'template'`, computing `target_date` from `offset_days + today()`
6. "Dismiss" hides the banner (no persistence — it will appear again on next lifecycle change)

### Renewal Proximity Detection

For the Renewal Preparation template, also check `contract_end` proximity:

```rust
// In get_account_detail or a dedicated check:
if account.contract_end is within 120 days && no active objectives with source='template' {
    suggest "renewal-preparation" template
}
```

This runs on account detail load, not as a background task.

---

## Template Application

### Tauri Command

```rust
#[tauri::command]
pub async fn apply_success_plan_template(
    state: State<'_, AppState>,
    account_id: String,
    template_id: String,
) -> Result<Vec<AccountObjective>, String>
```

**Behavior:**
1. Look up template by ID from the built-in constant
2. For each template objective: call `create_objective` with `source = 'template'`
3. For each milestone: call `create_milestone` with `target_date = today + offset_days` and `auto_detect_signal` from template
4. Return the created objectives (for frontend to render immediately)

Does NOT check for duplicate objectives. If the user applies the same template twice, they get duplicate objectives. This is intentional — the user can delete duplicates, and preventing duplicates requires fuzzy matching that would be unreliable.

### Tauri Command — List Templates

```rust
#[tauri::command]
pub fn list_success_plan_templates() -> Vec<SuccessPlanTemplate>
```

Returns all built-in templates. Frontend uses this to render the template picker.

---

## Files

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/services/success_plan_templates.rs` | Template constants + suggestion logic + application function |
| `src/components/entity/TemplateSuggestionBanner.tsx` | Banner UI for template suggestion |
| `src/components/entity/TemplateSuggestionBanner.module.css` | Banner styles |
| `src/components/entity/TemplatePreview.tsx` | Template preview popover/modal showing objectives + milestones |
| `src/components/entity/TemplatePreview.module.css` | Preview styles |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/services/mod.rs` | Add `pub mod success_plan_templates;` |
| `src-tauri/src/commands/success_plans.rs` | Add `apply_success_plan_template` and `list_success_plan_templates` commands |
| `src/pages/AccountDetailEditorial.tsx` | Show TemplateSuggestionBanner when lifecycle matches a template |
| `src/components/entity/TheWork.tsx` | Add "From template" option alongside "+ Objective" (opens template picker) |

---

## Vocabulary (ADR-0083)

| System Term | User-Facing Label |
|-------------|------------------|
| template | "Template" or "Success plan template" |
| lifecycle_trigger | never shown |
| offset_days | shown as computed dates, not offsets |
| auto_detect_signal | "Auto-completes when [event] is recorded" |
| source='template' | "From template" badge (subtle, mono) |

---

## Out of Scope

- User-created custom templates (post-1.0)
- Template editing/customization UI
- Template versioning
- Preset-specific template variants (all presets use the same templates for now)
- Template sharing between users
- Scheduled/automatic template application (always user-initiated)

---

## Acceptance Criteria

1. `list_success_plan_templates` returns 4 built-in templates: Onboarding, Growth & Expansion, Renewal Preparation, At-Risk Recovery.
2. Each template has 3 objectives with 3 milestones each. Milestones have `offset_days` and optional `auto_detect_signal`.
3. `apply_success_plan_template("onboarding-standard")` creates 3 objectives + 9 milestones with `source = 'template'`. Target dates computed from today + offset_days.
4. Milestones with `auto_detect_signal` have the signal value stored. When the corresponding lifecycle event fires (tested via `record_account_event`), the milestone auto-completes (tested end-to-end with I551 auto-detection).
5. Changing account lifecycle to "onboarding" shows a template suggestion banner. "View" shows preview. "Apply" creates objectives + milestones.
6. Renewal proximity: account with `contract_end` within 120 days and no template-sourced objectives shows renewal preparation template suggestion on account detail load.
7. Template suggestion banner is dismissible. Dismissed state is per-session only (not persisted).
8. The Work chapter shows a "From template" option alongside "+ Objective" that opens a template picker.
9. Applying the same template twice creates duplicate objectives (intentional). User can delete duplicates.
10. Zero ADR-0083 vocabulary violations. Template UI uses human-readable labels.
11. All new CSS in module files. Zero inline styles. Design tokens only.
12. Empty state: template preview shows all objectives + milestones clearly. No truncation.
13. Mock data includes at least one account with template-sourced objectives (`source = 'template'`) so the template badge and template-applied state are visible in mock scenarios.
