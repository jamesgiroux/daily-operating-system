# MonthlyWrappedPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `MonthlyWrappedPage`
**`data-ds-spec`:** `surfaces/MonthlyWrappedPage.md`
**Source files:**
- `src/pages/monthly-wrapped/MonthlyWrappedPage.tsx`
- `src/pages/monthly-wrapped/monthly-wrapped.module.css`
- `src/pages/monthly-wrapped/animations.css`

## Job

MonthlyWrappedPage renders a celebratory monthly retrospective. It turns the user's meetings, account movement, wins, patterns, priorities, and carry-forward work into a high-energy slide sequence.

## Layout regions

1. Full-viewport wrapped slide container.
2. Splash/month introduction.
3. Volume, top accounts, moments, hidden pattern, and personality slides.
4. Priority, top win, carry-forward, and closing slides.
5. Animated backgrounds, count-ups, and scroll-snap navigation.

## Patterns and primitives

Consumes monthly-wrapped slide components, animation utilities, `GeneratingProgress`, buttons, skeletons, and the magazine shell registration hooks.

## States

Supports loading, generating, cached report, empty month, parse error, save-in-progress, saved, regeneration, keyboard slide navigation, and completed report states.
