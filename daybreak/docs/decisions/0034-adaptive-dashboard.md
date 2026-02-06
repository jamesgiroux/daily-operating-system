# ADR-0034: Adaptive dashboard: density-aware layout

**Date:** 2026-02
**Status:** Proposed

## Context

The dashboard layout is identical for 0 meetings and 9 meetings. Busy days need a "what's next" view. Light days waste space and could offer focus/coaching content.

## Decision

Near-term (recommended): Density hint in overview text only. Keep current layout but make the AI-generated overview density-aware. Busy day: "Packed day — your 9 AM Acme call is the priority." Light day: "Open afternoon — good day to tackle that overdue Globex proposal."

Long-term: Time-aware adaptive layout (collapsed future meetings on busy days, expanded focus on light days, "between meetings" HUD). The HUD could be a tray popover rather than a dashboard change.

## Consequences

- Near-term: Low implementation cost, leverages existing AI enrichment, no layout changes
- Doesn't solve the "between meetings" UX on busy days — that's a separate feature
- Not blocking MVP — the static layout works
- Revisit when post-meeting capture (ADR-0016) ships, since that's when "between meetings" becomes critical
