# HealthBadge

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `HealthBadge`
**`data-ds-spec`:** `primitives/HealthBadge.md`
**Variants:** `size="compact" | "standard" | "hero"`; `band="green" | "yellow" | "red"`; insufficient-data state
**Design system version introduced:** 0.5.0

## Job

Show a health score with a compact visual band, trend direction, and optional evidence qualifiers. This is the shipped score primitive used when a row, hero, or meeting recap needs to communicate account or relationship health quickly.

## When to use it

- In list rows where the score must scan without a card.
- In account or meeting hero areas where the score anchors the health read.
- When the UI already has a computed score, band, trend, and sufficient-data signal.

## When NOT to use it

- For generic connection status; use `StatusDot`.
- For intelligence completeness; use `IntelligenceQualityBadge`.
- For unscored risk labels; use `Pill`.

## Source

- **Code:** `src/components/shared/HealthBadge.tsx`
- **Styles:** `src/components/shared/HealthBadge.module.css`

## Surfaces that consume it

AccountsPage, AccountHero, MeetingDetailPage, DailyBriefing, and WeekPage.

