# Avatar

**Tier:** primitive
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `Avatar`
**`data-ds-spec`:** `primitives/Avatar.md`
**Variants:** photo URL; cached person avatar; initials fallback; variable size
**Design system version introduced:** 0.5.0

## Job

Show a person visually with the best available image, falling back to a single initial in the DailyOS editorial style.

## When to use it

- People list rows.
- Person hero areas.
- Stakeholder galleries and relationship views where a face or initial helps scanning.

## When NOT to use it

- Account or project identity marks; use entity dots, type badges, or dedicated entity heroes.
- Decorative people imagery.

## Source

- **Code:** `src/components/ui/Avatar.tsx`
- **Extraction note:** source currently uses inline styles for the rendered size/color shape; keep this spec as the canonical contract while a CSS module extraction is pending.

## Surfaces that consume it

PeoplePage, PersonHero, and StakeholderGallery.

