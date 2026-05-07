# Design System Documentation

**Owner:** Product Design
**Status:** Living documentation. Engineers MUST reference this before building UI.

---

## Layout

The design system has migrated from monolithic top-level docs to per-entry specs plus canonical reference renders. Read in this order:

| Layer | Where | What it is |
|---|---|---|
| **Tokens** | [`tokens/`](./tokens/) | Color, typography, spacing, motion, radius, shadows, glass, layout, and z-index — the primitive values everything else composes from |
| **Primitives** | [`primitives/`](./primitives/) | 23 atomic UI elements (Pill, EntityChip, HealthBadge, EditableText, etc.) — one spec per primitive |
| **Patterns** | [`patterns/`](./patterns/) | Composed UI patterns (FolioBar, ChapterHeading, EntityListShell, PostMeetingIntelligence, etc.) — one spec per pattern |
| **Surfaces** | [`surfaces/`](./surfaces/) + [`reference/surfaces/`](./reference/surfaces/) | Per-screen specs + manifest-backed fidelity checks for covered routed pages, reports, onboarding chapters, and splash modes; see [`INVENTORY.md`](./INVENTORY.md) for acknowledged gaps |
| **Journeys** | [`reference/journeys/`](./reference/journeys/) | JTBD flows mapping which surfaces a CSM touches to accomplish each job |
| **Inventory** | [`INVENTORY.md`](./INVENTORY.md) | Full surface roster (referenced + spec'd + gap) |
| **Audits** | [`_audits/`](./_audits/) | Drift audits, fidelity reports, consolidation analyses |

## The Design Philosophy (30-second version)

DailyOS is a **magazine, not a dashboard**. Every surface is a document the user reads top-to-bottom, not a database they query. The aesthetic is editorial calm — a beautifully typeset briefing laid on a warm desk.

- **Typography does the structural work.** If you need a border to tell sections apart, the type scale isn't working.
- **Cards are for featured content only.** Most content is styled text rows separated by spacing and thin dividers.
- **Color communicates state, not decoration.** If removing the color doesn't change the meaning, the color was decorative.
- **Every page ends.** Finite documents, not infinite feeds. FinisMarker at the bottom, always.
- **Conclusions before evidence.** The hero tells you the synthesis. The page provides the proof.

## Quick Reference: "Should I..."

| Question | Answer |
|----------|--------|
| Wrap this in a Card? | Probably not. Cards are for meeting cards, priority items, signal cards, and the focus callout. Everything else is text rows. |
| Use a hardcoded hex color? | Never. Use design tokens from `design-tokens.css` or Tailwind semantic classes — see [`tokens/color.md`](./tokens/color.md). |
| Add a new font? | No. Newsreader, DM Sans, JetBrains Mono, Montserrat (mark only). See [`tokens/typography.md`](./tokens/typography.md). |
| Use inline `style={{}}` props? | Avoid. Use CSS modules or Tailwind classes. The reference-fidelity audit at [`_audits/audit-reference.py`](./_audits/audit-reference.py) flags inline-style invention against canonical TSX. |
| Build a new component for this? | Check [`primitives/`](./primitives/) and [`patterns/`](./patterns/) first. There are 72 documented primitive/pattern specs, and status labels use `proposed`, `integrated`, or `production`. |
| Use "intelligence" or "enrichment" in user-facing text? | Never. System terms stay in code. See vocabulary guidance in `src/CLAUDE.md`. |
| Skip the FinisMarker? | No. Every editorial page ends with one. See [`patterns/FinisMarker.md`](./patterns/FinisMarker.md). |

## Substrate references

Domain-specific reference docs that don't fit the token/primitive/pattern/surface taxonomy:

- [`INTELLIGENCE-CONSISTENCY-REFERENCE.md`](./INTELLIGENCE-CONSISTENCY-REFERENCE.md) — contradiction guardrails and SQL diagnostics for briefing trust issues
- [`SIGNAL-SCORING-REFERENCE.md`](./SIGNAL-SCORING-REFERENCE.md) — signal scoring algebra
- [`FIELD-PROMOTION-MATRIX.md`](./FIELD-PROMOTION-MATRIX.md) — field promotion rules
- [`_audits/shipped-component-inventory.md`](./_audits/shipped-component-inventory.md) — shipped-source-first component inventory and status corrections
- [`NAMING.md`](./NAMING.md) — naming conventions
- [`POSITIONING.md`](./POSITIONING.md), [`PRODUCT-PRINCIPLES.md`](./PRODUCT-PRINCIPLES.md), [`SYSTEM-MAP.md`](./SYSTEM-MAP.md) — strategy
- [`VERSION.md`](./VERSION.md), [`VIOLATIONS.md`](./VIOLATIONS.md), [`CHANGELOG.md`](./CHANGELOG.md) — process
- Backend architecture docs (data flow, service contracts, etc.) live under [`../architecture/`](../architecture/), not here.

## What got moved

Earlier monolithic top-level docs (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, PAGE-ARCHITECTURE.md, etc.) have been superseded by the per-entry layout above and live under [`_archive/`](./_archive/) for historical context.
