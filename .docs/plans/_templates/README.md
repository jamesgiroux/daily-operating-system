# `.docs/plans/_templates/` — Starter scaffolds for Tier 1 HTML-first plan docs

Each file here is a copyable starter for a doc type. Clone, rename, fill in.

## Current templates

| Template | File | When to use |
|---|---|---|
| Generic plan | `generic-plan.html` | Any Tier 1 plan doc — version plans, roadmaps, internal references. Includes cover, sections, callouts, finis. |
| Wave plan | `wave-plan.html` | New wave plan (e.g., `v1.4.5-waves.html`). Includes wave-overview cards, gate states, reviewer matrix, suite cards. |

## Workflow

1. Copy the template to your target path:
   ```
   cp .docs/plans/_templates/generic-plan.html .docs/plans/my-new-doc.html
   ```
2. Update `<title>`, the cover section, and the topnav links to fit the new doc.
3. Compose patterns from `.docs/plans/_patterns/` — copy snippets, paste in place, customize.
4. Annotate any new inline style or one-off layout with `<!-- TODO(ds): needs pattern -->`.
5. Source content stays markdown when appropriate (per the AUTHORING.md three-tier ruleset). Long-form prose sections inside an HTML doc are fine; full markdown source files are not duplicated.

## What templates DON'T do

- They don't auto-link the topnav — update it manually so navigation works as the planning surface grows.
- They don't generate IDs, dates, or other metadata — fill those in.
- They don't replace authoring judgment — the template is structure; the content is yours.

## Adding new templates

When 2+ docs of the same type have shipped and have a common shape, extract a new template:

1. Look at the structural commonalities across the docs.
2. Copy the cleanest version to `_templates/<doctype>.html`.
3. Replace concrete content with placeholder text (using `&lt;angle brackets&gt;` for clarity).
4. Add to the table above.

Don't pre-emptively create templates for doc types that don't exist yet — wait for the second instance.
