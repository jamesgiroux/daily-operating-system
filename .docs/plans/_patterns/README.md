# `.docs/plans/_patterns/` — Snippet library for Tier 1 HTML-first plan docs

Each file in this directory is a copy-and-adapt snippet for plan-doc authoring. These are not React components or includes — they're starting points you paste into your HTML file and modify in place.

## When to add a pattern here

A snippet earns a place in `_patterns/` when:

1. It's been used in **2+ HTML-first plan docs**, OR
2. It's a high-confidence reusable shape (matrix tables, status pills, callouts)

Single-use shapes stay inline in the source doc with a `<!-- TODO(ds): needs pattern -->` comment naming the gap. If/when the gap recurs, extract here.

## When to promote a pattern OUT of here

When a `_patterns/` snippet becomes useful across **multiple surface types** (product app + planning + reference), promote it to `.docs/design/reference/_shared/` (or to a new `.module.css` if it's a real component). The plan-doc snippet then becomes a `<link rel="stylesheet">` import + thin HTML.

## Current patterns

| Pattern | File | Purpose |
|---|---|---|
| Cover section | `cover-section.html` | Hero / lede / meta strip used at the top of every plan doc |
| Section header | `section-header.html` | Eyebrow + title + lede block that introduces a section |
| Topnav | `topnav.html` | Page topnav with brand mark + links |
| Pass-rule badge | `pass-rule-badge.html` | Gate-state badge (REVISE / BLOCK / APPROVE / etc.) |
| Reviewer matrix | `reviewer-matrix.html` | Two-column table mapping agent profile → domain reviewer |
| Suite card | `suite-card.html` | Test suite card with large letter, prose, owner, pass rule |
| K-channel diagram | `k-channel-diagram.html` | Two-zone feedback-loop visualization (store + flows) |
| Two-column policy list | `two-column-policy.html` | Side-by-side MUST / MUST NOT lists with pills |
| Callout strip | `callout.html` | Bordered operational-rule callout (pacing / bounding / etc.) |
| Finis marker | `finis.html` | Close-of-document signature |

## Conventions

- Patterns use only design tokens from `.docs/design/reference/_shared/styles/design-tokens.css` (no hardcoded colors / sizes).
- Patterns are presentation-only — no JavaScript, no data binding. Static HTML.
- Class names are kebab-case with a single domain prefix (`ladder-`, `k-`, `suite-`, `l6-`, `plans-`, `callout-`). Avoid generic names like `.card`, `.title`.
- When a snippet uses a token, reference it via `var(--color-…)` / `var(--space-…)` / `var(--font-…)` — never literal values.
