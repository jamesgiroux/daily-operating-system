# I537 — Gate Role Presets Behind Feature Flag (CS-Only for v1.0.0)

**Priority:** P1
**Area:** Frontend / Onboarding / Settings
**Version:** v1.0.0 (Phase 3)

## Problem

DailyOS has 9 role presets (Customer Success, Sales, Marketing, Partnerships, Agency, Consulting, Product, Leadership, The Desk) that control vocabulary, vitals, report availability, nav ordering, briefing emphasis, and email priority keywords. The preset system is well-built but premature — v1.0.0 is laser-focused on the CS use case. Showing 9 role options during onboarding and in Settings creates confusion ("which one should I pick?"), dilutes the CS-first message, and surfaces configuration that isn't tested or polished for non-CS roles.

The preset infrastructure should stay — vocabulary injection, vitals rendering, prompt shaping all work correctly and will be needed post-v1.0.0 when DailyOS expands to other roles. But the **selection UI** should be hidden, and the app should hard-default to Customer Success.

## Design

### 1. Feature flag

Add a `feature_flags` section to `Config` (or a standalone constant):

```rust
// In src-tauri/src/types.rs or a new feature_flags module
pub const ROLE_PRESETS_ENABLED: bool = false;  // flip to true post-v1.0.0
```

The frontend needs access to this flag. Options:
- **Option A (recommended):** New Tauri command `get_feature_flags()` returning a `FeatureFlags` struct. Simple, extensible for future flags.
- **Option B:** Include in the existing `get_config()` response. Simpler but mixes config with feature gating.

### 2. Onboarding: skip EntityMode chapter

`OnboardingFlow.tsx` currently includes an `EntityMode` chapter that shows the 9-preset grid. When `ROLE_PRESETS_ENABLED = false`:

- **Skip the EntityMode chapter entirely.** The onboarding flow goes from the prior chapter directly to PopulateWorkspace (or whichever chapter follows).
- **Auto-set role to `"customer-success"`.** Call `invoke("set_role", { role: "customer-success" })` during onboarding initialization, before the wizard renders.
- **Override entity mode to `"both"`.** The CS preset defaults to `entityMode = "account"`, but users should have access to both accounts and projects. After setting the role, call `invoke("set_entity_mode", { mode: "both" })` to ensure both are available.

The `PopulateWorkspace` chapter uses `entityMode` to show/hide account vs project input fields — with `"both"`, users see inputs for both accounts and projects. `FolderTree` also gates folder previews on entity mode.

### 3. Settings: hide RoleSection + EntityModeSelector

`YouCard.tsx` contains a `RoleSection` component (line ~190) that renders the 9-preset grid. `DiagnosticsSection.tsx` contains an `EntityModeSelector` (line ~701) — a "Work Mode" picker that lets users switch between account/project/both. When `ROLE_PRESETS_ENABLED = false`:

- **Hide the entire `RoleSection` component.** Don't render it at all — no "Role Presets" heading, no grid, no selection.
- **Hide the `EntityModeSelector` component** in DiagnosticsSection. Entity mode is locked to `"both"`.
- The rest of YouCard (name, email, identity fields) and DiagnosticsSection (schedules, manual runs, backfill, archived accounts) stay visible.

### 4. Backend: no changes

All backend preset infrastructure stays exactly as-is:

- `set_role()` / `get_active_preset()` / `get_available_presets()` commands remain
- Vocabulary injection into prompts continues (using CS vocabulary)
- Email priority keywords continue
- Report availability per preset continues
- Vitals/metadata rendering continues

The preset is still loaded from `config.role` at startup. It just defaults to `"customer-success"` and the user can't change it through the UI.

### 5. Entity mode: `"both"` with CS vocabulary

With entity mode set to `"both"`:
- `FloatingNavIsland` shows Accounts before Projects (the `"both"` path uses account-first ordering, same as `"account"`)
- `PopulateWorkspace` shows both account and project input fields during onboarding
- `FolderTree` shows both account and project folders in workspace preview
- Users get full access to both entity types while CS vocabulary shapes prompts and vitals

### 6. What stays visible

Even with presets hidden, the app still uses CS preset data:

| Surface | Behavior | Source |
|---------|----------|--------|
| Vitals strip on account detail | ARR, Health, Lifecycle, NPS, Renewal Date | `preset.vitals.account` |
| Report menu | Account Health, EBR/QBR, SWOT, Risk Briefing | `report-config.ts` |
| Nav ordering | Accounts before Projects (both visible) | `config.entity_mode = "both"` |
| Intelligence prompts | "accounts", "ARR", "Health Score", "Churn Risk" | `preset.vocabulary` |
| Email classification | 15 CS priority keywords boosted | `preset.email_priority_keywords` |
| Briefing emphasis | Retention, renewals, health | `preset.briefing_emphasis` |
| MePage playbooks | At-Risk, Renewal, EBR/QBR | Hardcoded CS branch |

All of this continues working — the user just can't switch away from it.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/types.rs` (or new `feature_flags.rs`) | Add `ROLE_PRESETS_ENABLED` constant. Optionally add `FeatureFlags` struct + `get_feature_flags()` command. |
| `src-tauri/src/commands.rs` | Add `get_feature_flags` command if using Option A. |
| `src-tauri/src/lib.rs` | Register `get_feature_flags` command if added. |
| `src/components/onboarding/OnboardingFlow.tsx` | Skip `EntityMode` chapter when presets disabled. Auto-set CS role. |
| `src/components/onboarding/chapters/EntityMode.tsx` | No deletion — just not rendered when flag is off. |
| `src/components/settings/YouCard.tsx` | Hide `RoleSection` when presets disabled. |
| `src/components/settings/DiagnosticsSection.tsx` | Hide `EntityModeSelector` when presets disabled. |
| `src/types/` | Add `FeatureFlags` TypeScript type if using Option A. |

## Acceptance Criteria

1. Fresh install: onboarding flow does not show the role preset selection grid. User goes straight from identity setup to workspace population (or whatever chapter follows EntityMode).
2. After onboarding, `config.role` is `"customer-success"` and `config.entity_mode` is `"both"` without the user having to select anything.
3. Settings page does not show the "Role Presets" section, the 9-preset grid, or the "Work Mode" entity mode selector.
4. All CS preset behavior works: vocabulary in prompts, vitals on account detail, report menu shows Account Health + EBR/QBR + SWOT + Risk Briefing, email priority keywords active.
5. Nav shows both Accounts and Projects (Accounts first). Both entity types fully accessible.
6. `get_active_preset()` still returns the full CS `RolePreset` struct — no data loss.
7. `get_available_presets()` still returns all 9 presets (backend not gated — only UI hidden).
8. Setting `ROLE_PRESETS_ENABLED = true` restores the EntityMode chapter in onboarding and the RoleSection in Settings. No code deletion — just conditional rendering.
9. Existing users who already selected a non-CS preset: their selection is preserved in config but they can't change it via UI. App continues using whatever preset they had. (Edge case — acceptable for v1.0.0 since all current users are CS.)

## Out of Scope

- Removing preset JSON files or backend preset loading infrastructure
- Removing preset-driven vitals, vocabulary, or report filtering
- Removing the `set_role` / `get_active_preset` / `get_available_presets` commands
- Any changes to how presets affect backend prompts or intelligence
- Redesigning the preset system for post-v1.0.0 multi-role support
