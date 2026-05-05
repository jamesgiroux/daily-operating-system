# FolioRefreshButton

**Tier:** primitive
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `FolioRefreshButton`
**`data-ds-spec`:** `primitives/FolioRefreshButton.md`
**Variants:** idle label; loading label; loading progress; disabled
**Design system version introduced:** 0.5.0

## Job

Provide the quiet mono editorial refresh/run action used in folio bars and hero action rows. It is intentionally smaller and calmer than product-primary buttons.

## When to use it

- Refresh, regenerate, or rerun actions attached to a surface-level briefing.
- FolioBar-adjacent actions that should not look like navigation.
- Hero actions on entity or project pages where the action is operational, not a CTA.

## When NOT to use it

- Form submissions or destructive actions.
- Dense toolbar icon actions.
- Primary product CTAs that need visual weight.

## Source

- **Code:** `src/components/ui/folio-refresh-button.tsx`
- **Extraction note:** source currently carries inline button styles; this spec preserves the shipped contract and flags the CSS-module extraction target.

## Surfaces that consume it

DailyBriefing, MeetingDetailPage, ProjectHero, and folio-adjacent entity surfaces.

