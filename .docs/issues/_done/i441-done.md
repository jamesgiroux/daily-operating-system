# I441 — Personality Coverage + useActivePreset Cache

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Frontend

## Summary

Two separate gaps in the personality and preset systems are addressed together because they share the same root fix pattern — both need a shared React context initialized once at app root. Three empty state keys are defined in `personality.ts` but never called (`accounts-empty`, `projects-empty`, `actions-waiting-empty`), so personality-differentiated copy never renders in those states. And `useActivePreset` fires three separate IPC calls per entity detail page navigation with no shared cache and no reactivity — switching presets in Settings has no visible effect until the user navigates. This issue fixes both: wires the missing empty state keys and refactors `useActivePreset` to a shared context modelled on the existing `PersonalityProvider` pattern.

## Acceptance Criteria

1. The three defined-but-never-called empty state keys are wired to components: `accounts-empty` → AccountsPage empty state (when no accounts exist), `projects-empty` → ProjectsPage empty state, `actions-waiting-empty` → ActionsPage waiting-on tab empty state. Verify by clearing the relevant data and navigating — personality-differentiated copy renders.
2. `useActivePreset` is refactored to a shared React context (similar to `PersonalityProvider`). The context is initialized once at app root, not three times per entity detail page navigation. Verify: `grep -rn "useActivePreset\|get_active_preset" src/ --include="*.tsx"` — no direct `invoke("get_active_preset")` calls remain in page components; all consumption goes through the context.
3. The active preset context is reactive: switching presets in Settings updates the preset context within 1 second without requiring navigation. Verify: switch from Customer Success to Sales in Settings; the AccountDetailEditorial.tsx vitals panel updates to show Sales-appropriate fields without navigating away and back.
4. All 9 presets render without error when activated. Verify: activate each of the 9 presets in turn in the running app — no console errors, vitals panel renders the correct fields for each.

## Dependencies

- I446 (user entity page × preset) is blocked by this issue — the reactive preset context is required for the `/me` page to update on preset switch.
- No other issues block this one. Build first in the v0.14.1 sequence.

## Notes / Rationale

The three missing empty state wires are straightforward: `accounts-empty` in AccountsPage, `projects-empty` in ProjectsPage, and `actions-waiting-empty` in ActionsPage's waiting tab. The `getPersonalityCopy()` function in `src/lib/personality.ts` already defines these keys with personality-differentiated strings; they just are not called anywhere.

The `useActivePreset` refactor should mirror the `PersonalityProvider` pattern in `usePersonality.tsx` — a React context and provider initialized at app root, with a `useActivePreset()` hook that reads from context rather than firing `invoke("get_active_preset")`. The provider should listen for a Settings change event (or use a polling interval as a fallback) to refresh the preset when it changes, giving the reactivity required by acceptance criterion 3.

The reactivity mechanism should be consistent with how `PersonalityProvider` handles its own updates. If personality already refreshes on a settings-change event, use the same event for preset. If it polls, extend that polling to include the preset. Do not introduce a second polling loop.

Once the context is in place, all existing direct `invoke("get_active_preset")` calls in page components are replaced with `useActivePreset()` from context. The blast radius is bounded: search for all `invoke("get_active_preset")` call sites and replace them.
