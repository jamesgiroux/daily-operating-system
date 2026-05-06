# InferredActionSelector

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `InferredActionSelector`
**`data-ds-spec`:** `patterns/InferredActionSelector.md`
**Variants:** selected; parked; compact mobile
**Design system version introduced:** 0.1.0

## Job

Offer one recommended action with a small dropdown of alternatives. The control
is for low-stakes watch items where DailyOS can suggest a next move without
turning the row into a heavy CTA.

## When to use it

- Watch rows in DailyBriefing Daily Briefing redesign.
- Future lightweight recommendation rows where the user should be able to choose
  an alternate handling.

## When NOT to use it

- Destructive actions.
- Primary task execution. Use a normal button when there is one clear command.
- Status selection. This selector is an action picker, not a state picker.

## Composition

- Trigger: label + CSS-drawn chevron icon
- Menu: plain text alternatives; no confidence dots in the list
- Optional selected row treatment

## Source

- **Mockup substrate:** `/Users/jamesgiroux/Downloads/dailyos-design-system 2/project/mockups/briefing/variations/Daily Briefing redesign.html`
- **Reference styles:** `.docs/design/reference/_shared/styles/InferredActionSelector.module.css`

## Surfaces that consume it

- `DailyBriefingRedesign` proposed reference surface (`.docs/design/reference/surfaces/briefing-redesign.html`)
