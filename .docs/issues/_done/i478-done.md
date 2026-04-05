# I478 — Remove feature toggle section from Advanced Settings

**Priority:** P1
**Area:** Frontend / Settings + Backend / Config
**Version:** v0.16.0

## Problem

The Advanced Settings panel exposes a FeaturesSection with 10 enable/disable toggles for core app capabilities (emailTriage, meetingPrep, weeklyPlanning, inboxProcessing, accountTracking, projectTracking, impactRollup, postMeetingCapture, autoArchiveEnabled, emailBodyAccess). These were useful during development but are wrong for end users:

- Disabling core features (emailTriage, meetingPrep, weeklyPlanning) cripples the app with no user benefit.
- accountTracking / projectTracking duplicate what entity_mode already controls.
- impactRollup is downstream of account tracking — not independently meaningful.
- emailBodyAccess is never shown in the UI at all.
- The toggles leak internal architecture ("inbox processing", "impact rollup") into a user-facing surface, violating ADR-0083 product vocabulary.

With onboarding (v0.16.0) bringing new users into the app for the first time, the settings surface must not expose internal development knobs.

## Solution

1. **Remove `FeaturesSection`** from `SystemStatus.tsx` Advanced panel entirely.
2. **Remove `get_features` and `set_feature_enabled` Tauri commands** from `commands.rs`. These are the only callers.
3. **Keep `is_feature_enabled()` in the backend** — it still drives behavior gating internally. Features remain controlled by entity_mode + role preset defaults (`default_features_for_mode`), just not user-toggleable.
4. **Remove `config.features` HashMap** from the persisted Config if no features have user overrides. If a user has previously set overrides, they are harmless (ignored on next read) — no migration needed.
5. **Keep all other Advanced sections** (AI Models, Hygiene, Capture, Data Management) unchanged.

## Acceptance Criteria

1. The Advanced Settings panel no longer shows any feature enable/disable toggles.
2. `get_features` and `set_feature_enabled` commands are removed from the Tauri command list.
3. `is_feature_enabled()` still works correctly — features gate based on entity_mode + role preset defaults.
4. Existing user config files with `features: {}` overrides do not cause errors on load.
5. All other Advanced sections (AI Models, Hygiene, Capture, Data Management, Lock Timeout) render unchanged.
