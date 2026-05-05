# Archive

Superseded entries. Kept for context — we don't delete history, we date it.

## When something lands here

- A primitive or pattern is replaced by a better version → old `.md` moves here with a `> Superseded by [link]` note added at the top.
- A top-level documentation file (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, etc.) is replaced by per-entry specs → moves here with a redirect note pointing at the new home.
- A surface is removed from the product → spec moves here.

## What's here today

The original monolithic top-level design docs, now superseded by per-entry specs in `tokens/`, `primitives/`, `patterns/`, and `surfaces/` plus canonical reference renders in `reference/surfaces/`:

- `DESIGN-SYSTEM.md`, `COMPONENT-INVENTORY.md`, `PAGE-ARCHITECTURE.md`, `STATE-PATTERNS.md`, `NAVIGATION-ARCHITECTURE.md`, `INTERACTION-PATTERNS.md`, `DATA-PRESENTATION-GUIDELINES.md` — each carries a redirect note pointing at its successor location.

Surface- and feature-scoped historicals:

- `account-detail-content-design.md` — superseded by `reference/surfaces/account.html`
- `I644-FIELD-MATRIX.md` — issue-scoped historical
- `PLUGIN-MARKETPLACE-DESIGN.md` — feature was descoped

Backend architecture references that previously sat under `design/` have been relocated to `../architecture/` rather than archived (they're current, just in the wrong place).

## What does NOT go here

- Mockup variations that didn't get picked → those live in `.docs/_archive/mockups/_archive/`
- Code branches / experiments → git history handles that
- Anything that's still being decided → keep it in active dirs until the decision lands
