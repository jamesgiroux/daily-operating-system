# Reference

Rendered HTML/CSS/JS that makes the design system *visible*, not just readable. Markdown specs say what something is; reference renders show it working.

## What lives here

- **`_shared/`** — the canonical CSS+JS substrate consumed by every reference render and by exploration mockups
  - `tokens.css` — runtime token declarations (mirrors `src/styles/design-tokens.css`)
  - `primitives.css` — primitive class library (`.pill`, `.type-badge`, `.vitals`, etc.)
  - `chrome.css` + `chrome.js` — auto-injected page chrome (`FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`)
  - `fonts.css` — webfont declarations
  - `inspector.js` + `inspector.css` — opt-in hover inspector (`?` to toggle); reads `data-ds-*` attributes
- **`surfaces/`** — one HTML reference render per app surface (briefing, account detail, settings, etc.). Each renders with mock data and links to peer surfaces via `chrome.js` nav (set `data-nav-base` on body).
- **`system/`** — system showcase pages: `tokens.html`, `primitives.html`, `patterns.html`. Each gallery loads `_shared/` + `inspector.js` and renders every entry with its `data-ds-*` attributes.

## Why this matters

- **Self-contained visual reference.** Open any `surfaces/<name>.html` and see the surface without running the app.
- **Figma / Claude Design export source.** When designing a new surface in an external tool, point at these renders.
- **Playwright target for visual regression.** A change to a token reflows the reference renders; visual diffs catch unintended drift.
- **Onboarding and review aid.** New contributors and PR reviewers can see the system in browser, with no setup.

## Conventions

- **Markdown specs are the contract.** Reference renders are *derivative* — they implement the spec. If they disagree, the spec is right and the render gets updated.
- **No domain data in renders.** Use placeholder text and the canonical mock data palette (Acme Corp, Globex Inc, Northwind Traders, Meridian Harbor; people: Jen Park, Dan Mitchell, Priya Raman, Marco Devine, Aoife Murphy, Liu Kang). Never real customer data.
- **One render per file.** Don't bundle everything into one mega-file.
- **Inspector loaded everywhere.** Every reference page loads `_shared/inspector.{js,css}`; press `?` to inspect any element with `data-ds-*` attributes.

## Navigation between reference renders

Reference render HTML files set `<body data-nav-base="../surfaces">` (or whatever the relative base is) so `chrome.js` renders the `FloatingNavIsland` items as anchor tags linking to peer surface files. Mockups (which don't set `data-nav-base`) keep rendering as buttons (no navigation, just visual chrome).

## How `_shared/` got here

Promoted from `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/` per **D8** (synthesis decision). The mockup-side files now `<link>` and `<script src=>` directly at this canonical location.
