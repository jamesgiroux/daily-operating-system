# [Name]

> Copy this template into the correct tier directory and rename the file. Delete this blockquote when done.

**Tier:** primitive | pattern | surface
**Status:** canonical | proposed | superseded
**Owner:** [name]
**Last updated:** YYYY-MM-DD
**`data-ds-name`:** `[Name]` <!-- value to use on rendered elements; see SYSTEM-MAP.md → Inspectability -->
**`data-ds-spec`:** `[tier]/[Name].md` <!-- relative path from design system root -->
**Variants:** `default` <!-- comma-separated list, used as data-ds-variant values -->
**Design system version introduced:** `0.0.0` <!-- bump per VERSION.md when this lands or changes -->

## Job

One paragraph. What problem does this solve for the user? Avoid implementation details.

## When to use it

- Bullet list of the situations this is the right answer for.

## When NOT to use it

- Bullet list of look-alike situations where a different entry fits better. Link to the better fit.

## States / variants

For primitives and patterns. Document every state visible to the user (default, hover, active, disabled, error, loading, empty, etc.) and every variant. Be specific about what differs visually and why.

For surfaces, replace with **Local nav approach** and **Layout regions**.

## Composition

For patterns: which primitives this is composed of, with brief notes on how. For primitives: omit. For surfaces: list patterns consumed, in reading order.

## Tokens consumed

- `--token-name` — what it controls

## API sketch

For primitives and patterns: the props / inputs / customization surface. Keep it minimal — if a prop only exists for one consumer, don't add it.

For surfaces: omit.

## Source

- **Code:** `src/path/to/file.tsx` (and related files)
- **Reference render:** `reference/[name].html` (if applicable)
- **Mockup origin:** `.docs/_archive/mockups/...` (if recently promoted)

## Surfaces that consume it

For primitives and patterns. List every surface using this and link to its `.md` spec. For surfaces: omit.

## Naming notes

Canonical name, current code name (if different), rename status. See `NAMING.md`.

## History

- YYYY-MM-DD — Promoted from `.docs/_archive/mockups/[origin]`
- YYYY-MM-DD — [change]
- YYYY-MM-DD — Superseded by `[name]` (only if status is `superseded`)
