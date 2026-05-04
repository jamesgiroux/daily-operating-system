# DailyOS Design System — Map

This directory is the **canonical source of truth** for the DailyOS design system. If something is here, the app code under `src/` should match it. If `src/` and this directory disagree, this directory is right and `src/` is the bug.

## Taxonomy

DailyOS uses four tiers (not atomic design):

| Tier | What it is | Examples | Where it lives |
|---|---|---|---|
| **Tokens** | Lowest-level decisions — color, type, spacing, motion | `--color-trust-likely-current`, `--space-4`, `--font-display` | `tokens/` |
| **Primitives** | Smallest reusable units. Generic, unopinionated | Button, Input, Card, Pill, Chip | `primitives/` |
| **Patterns** | Composed, opinionated, named after their job | TrustBand, ClaimRow, BriefingSpine, LocalNavIsland | `patterns/` |
| **Surfaces** | Full screens | DailyBriefing, AccountDetail, MeetingDetail, Settings | `surfaces/` |

**The pattern layer is where drift happens.** When two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

## Lifecycle

Every entry passes through three states:

```
exploration         →    promotion       →    canonical          →    superseded
.docs/_archive/mockups/           audit + decision      .docs/design/           _archive/
```

- **Exploration** — Mockups, research, "what if" experiments. Lives in `.docs/_archive/mockups/`. Not load-bearing.
- **Promotion** — A decision (audit, PR review) elevates an entry from exploration to canonical. Adds an `.md` file under the right tier.
- **Canonical** — The contract. Every primitive/pattern/surface in this dir has a `.md` spec, a code reference (`src/...`), and where applicable a reference render.
- **Superseded** — When something is replaced, it moves to `_archive/` with a note pointing at the replacement. We don't delete history.

## How to add a new entry

1. Copy `_TEMPLATE-entry.md` into the correct tier directory.
2. Name it after the job (PascalCase): `BriefingSpine.md`, not `Container2.md`.
3. Fill in every section. Empty sections == not promoted yet.
4. Update the tier `README.md` index.
5. Link the source file paths in `src/`.
6. If a reference render exists, link it under `reference/`.

## How to use this directory

- **Looking for a component?** Path is predictable: `primitives/[Name].md` or `patterns/[Name].md`.
- **Building a new surface?** Start with `surfaces/[Name].md` — list the patterns and primitives it consumes. Anything missing is a gap. File it.
- **Reviewing a PR?** Check that any UI it adds either consumes an existing entry or proposes a new one with an `.md` spec. No "we'll spec it later."
- **Exporting to Figma / Playwright / Claude Design?** Tokens live in `tokens/` as markdown + the rendered CSS in `reference/_shared/tokens.css`. That's the export surface.

## Naming discipline

See [NAMING.md](./NAMING.md). Surfaces and components should be named after the user-visible job, not implementation details. The current `Dashboard.tsx` → `DailyBriefing` is the canonical example of the rename track.

## Inspectability — the `data-ds-*` convention

Every rendered design system element — in `reference/*.html`, in `src/` components, anywhere — should carry data attributes that name what it is. This makes the system inspectable in browser devtools, greppable from code, readable by AI tools, and pointable-at in conversation.

```html
<section
  data-ds-tier="pattern"
  data-ds-name="AccountHero"
  data-ds-spec="patterns/AccountHero.md">
  <div
    data-ds-tier="primitive"
    data-ds-name="VitalsStrip"
    data-ds-spec="primitives/VitalsStrip.md"
    data-ds-variant="editable">
    …
  </div>
</section>
```

**Required attributes:**

- `data-ds-tier` — one of `token`, `primitive`, `pattern`, `surface`
- `data-ds-name` — the canonical PascalCase name. Matches the markdown filename in `<tier>/<Name>.md`.
- `data-ds-spec` — relative path from the design system root to the markdown spec.

**Optional attributes:**

- `data-ds-variant` — variant name when one is selected (e.g., `default`, `compact`, `editable`)
- `data-ds-state` — runtime state where meaningful (`loading`, `error`, `empty`)

**Why this matters:**

- **Hover inspect** — `reference/_shared/inspector.js` reads these attributes and renders a hover label with tier/name/spec. Toggle with `?`. Click a label to open the spec.
- **Greppable** — `grep -r 'data-ds-name="VitalsStrip"' src/` finds every consumer instantly.
- **AI-readable** — when I'm asked "what's that thing in the hero?" you can say "the VitalsStrip" and I can grep to it.
- **Lives in `src/` too** — adopted gradually, not just in reference renders. Every promoted primitive/pattern should pass the attributes through to its rendered output.

Templates and entries declare their `data-ds-name` value as part of the spec — see `_TEMPLATE-entry.md`.

## What lives outside this directory

- `.docs/_archive/mockups/` — exploration only. Iteration projects from Claude Design / hand-drawn / variations. Never canonical.
- `src/` — implementation. Consumes from this directory.
- `.docs/plans/` — version briefs and acceptance criteria. References this directory, doesn't duplicate it.

## Status (2026-05-02)

The structure exists. Population is in progress, driven by four audits (see `.docs/_archive/mockups/claude-design-project/_audits/`). Until those land, the existing top-level `.md` files in this directory (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, etc.) remain the working reference. Post-audit, those move to `_archive/` and per-entry `.md` files become canonical.
