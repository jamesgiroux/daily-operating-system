# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Index

_(populated as primitives are promoted)_

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| _(awaiting Audit 01 + Audit 03 findings)_ | | | |

## Conventions

- **Names are short and generic.** `Button`, not `BaseButton` or `PrimaryActionButton`.
- **Variants are documented.** Every visible variation (size, intent, density) is in the spec, with a screenshot or reference render.
- **Tokens only.** Primitives consume tokens. They never hardcode values.
- **Composition aware.** A primitive should compose cleanly inside any pattern. Avoid layout opinions; let patterns set spacing.
- **One file per primitive.** `Button.md`, not `Buttons.md`. Granularity makes it greppable.

## Adding a primitive

1. Confirm it's actually a primitive (no domain knowledge, used or usable in 2+ patterns).
2. Copy `../_TEMPLATE-entry.md` here.
3. Fill in the spec. Empty sections mean it's not ready.
4. Add to the index above.
5. If a reference render exists, link it.

## Adding a primitive that already exists in `src/`

The audit will surface these. Promotion is a markdown PR that documents what's already there — no code change required to *promote*. Code changes to consolidate variants come after.
