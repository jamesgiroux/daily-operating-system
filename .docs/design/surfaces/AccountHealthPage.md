# AccountHealthPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `AccountHealthPage`
**`data-ds-spec`:** `surfaces/AccountHealthPage.md`
**Source files:**
- `src/pages/AccountHealthPage.tsx`
- `src/pages/report-slides.module.css`

## Job

AccountHealthPage renders an account health review deck. It turns relationship signals, engagement cadence, value delivered, risks, and renewal context into a concise customer-facing account review.

## Layout regions

1. Report slide shell with account title and review controls.
2. Cover and partnership framing.
3. Current health and where-we-stand analysis.
4. Value-delivered evidence and expansion signals.
5. What's-ahead actions, renewal context, and recommendations.

## Patterns and primitives

Consumes the shared report-slide module, editable report text, account-health slide components, feedback controls, `GeneratingProgress`, and `FinisMarker`.

## States

Supports loading, generating, cached report, stale report, parse error, save-in-progress, saved, regeneration, empty inputs, and completed report states.
