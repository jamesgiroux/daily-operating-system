# Design System Documentation

**Owner:** Product Design
**Last audit:** 2026-02-20
**Status:** Living document. Engineers MUST reference this before building UI.

---

## What This Directory Contains

| Document | Purpose | Read before... |
|----------|---------|----------------|
| [DESIGN-SYSTEM.md](./DESIGN-SYSTEM.md) | The rules. Typography, color, spacing, layout, components. | Writing any CSS or building any component |
| [COMPONENT-INVENTORY.md](./COMPONENT-INVENTORY.md) | Every shared component, its job, its compliance status | Deciding whether to build a new component |
| [PAGE-ARCHITECTURE.md](./PAGE-ARCHITECTURE.md) | How each page is structured, what it renders, what it should look like | Touching any page file |
| [VIOLATIONS.md](./VIOLATIONS.md) | Known violations of the design system with severity and fix instructions | Starting any UI cleanup work |
| [ARCHITECTURE-MAP.md](./ARCHITECTURE-MAP.md) | Backend modules, data flow, async tasks, IPC surface | Any backend structural work |
| [SERVICE-CONTRACTS.md](./SERVICE-CONTRACTS.md) | Target service layer, extraction contracts, migration path | Refactoring commands.rs or db.rs |
| [PLUGIN-MARKETPLACE-DESIGN.md](./PLUGIN-MARKETPLACE-DESIGN.md) | Plugin marketplace design spec (pre-existing) | Working on integrations UI |

## The Problem This Solves

DailyOS has 84 ADRs, a 350KB backlog, and a design language spec split across ADRs 0073, 0076, 0077, 0083, and 0084. No engineer has time to read all of that before adding a `<div>`. The result: every page is slightly different, hardcoded colors creep in, spacing is inconsistent, and the editorial magazine aesthetic we spent weeks perfecting gets eroded by well-intentioned but unguided implementation.

**This directory is the single reference.** If it's not documented here, check the ADRs. If it conflicts with an ADR, the ADR wins and this document needs updating.

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
| Use a hardcoded hex color? | Never. Use design tokens from `design-tokens.css` or Tailwind semantic classes. |
| Add a new font? | No. Newsreader, DM Sans, JetBrains Mono, Montserrat (mark only). That's the stack. |
| Use inline `style={{}}` props? | Avoid. Use CSS modules or Tailwind classes. Inline styles are untraceable. |
| Build a new component for this? | Check [COMPONENT-INVENTORY.md](./COMPONENT-INVENTORY.md) first. There are 90+ components. Yours probably exists. |
| Use "intelligence" or "enrichment" in user-facing text? | Never. See the vocabulary table in DESIGN-SYSTEM.md. System terms stay in code. |
| Skip the FinisMarker? | No. Every editorial page ends with one. |
