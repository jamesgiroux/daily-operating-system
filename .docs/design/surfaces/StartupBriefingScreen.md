# StartupBriefingScreen

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `StartupBriefingScreen`
**`data-ds-spec`:** `surfaces/StartupBriefingScreen.md`
**Source files:**
- `src/components/startup/StartupBriefingScreen.tsx`
- `src/components/startup/StartupBriefingScreen.module.css`

## Job

StartupBriefingScreen holds the cold-start moment while DailyOS prepares context. It reassures the user that the app is working without exposing implementation mechanics.

## Layout regions

1. Splash mode: centered brand mark, DailyOS wordmark, short preparation title, and editorial rules.
2. Progress mode: elapsed time rail, current work title, phase list, rotating quote, and navigation hint.

## Patterns and primitives

This is a full-bleed startup surface, not an in-app page. It consumes only token-level styling and the DailyOS brand mark; do not wrap it in app chrome, cards, or folio controls.

## States

Supports splash, fading splash, progress, complete phase, current phase, pending phase, injected elapsed time, custom phase lists, custom quote lists, and hidden navigation hint.
