# StatusDot

**Tier:** primitive
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `StatusDot`
**`data-ds-spec`:** `primitives/StatusDot.md`
**Variants:** `connected`, `disconnected`, `loading`, `error`; `size="sm" | "md"`; optional label
**Design system version introduced:** 0.5.0

## Job

Represent a live connector or system status with a small colored dot. Optional label text can travel with the dot when the status must be readable outside a surrounding label.

## When to use it

- Connector status in Settings.
- Background system status, sync status, and diagnostics rows.
- Small operational states where a full pill would be too heavy.

## When NOT to use it

- For health scores; use `HealthBadge`.
- For entity identity colors; use `EntityChip` or `EntityRow`.
- For meeting temporal state; use `MeetingStatusPill`.

## Source

- **Code:** `src/components/shared/StatusDot.tsx`
- **Styles:** `src/components/shared/StatusDot.module.css`

## Surfaces that consume it

Settings and connector/detail diagnostics.

