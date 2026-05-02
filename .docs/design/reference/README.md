# Reference

Rendered HTML/CSS/JS that makes the design system *visible*, not just readable. Markdown specs say what something is; reference renders show it working.

## What lives here

- `_shared/` — the canonical CSS substrate (tokens, primitives, chrome). Seeded from `.docs/mockups/claude-design-project/mockups/surfaces/_shared/` (pending Audit 03 review).
- `tokens.html` — palette, type ramps, spacing scale, motion examples
- `primitives.html` — gallery of every primitive in every state
- `patterns.html` — each pattern rendered with its variants, in isolation
- _(future)_ Per-surface preview pages that compose patterns end-to-end

## Why this matters

- **Self-contained visual reference.** Open `tokens.html` and see the system without running the app.
- **Figma / Claude Design export source.** When designing a new surface in an external tool, point at these renders.
- **Playwright target for visual regression.** A change to a token reflows the reference renders; visual diffs catch unintended drift.
- **Onboarding and review aid.** New contributors and PR reviewers can see the system in browser, with no setup.

## Conventions

- **Markdown specs are the contract.** Reference renders are *derivative* — they implement the spec. If they disagree, the spec is right and the render gets updated.
- **No domain data in renders.** Use placeholder text (`subsidiary.com`, `user@example.com`). Never real customer data.
- **One render per file.** Don't bundle everything into one mega-file.

## Source seed

`.docs/mockups/claude-design-project/mockups/surfaces/_shared/` already contains:

- `tokens.css` — candidate token substrate
- `primitives.css` — candidate primitive styles
- `chrome.css` + `chrome.js` — page chrome (header, local nav, etc.)
- `fonts.css` — webfont declarations

These are reviewed in Audit 03. Anything that survives gets pulled into `_shared/` here as the canonical substrate.
