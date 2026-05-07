# Reference HTML — Agent briefing

**Goal**: produce a static HTML file at `.docs/design/reference/surfaces/<surface>.html` that is a faithful 1:1 visual mirror of the corresponding TSX page in `src/pages/`. Use only the building blocks already prepared in `_shared/`.

## How the system works

- **CSS** — every `*.module.css` from `src/components/` and `src/pages/` has been copied verbatim into `.docs/design/reference/_shared/styles/` and run through `scope-modules.py`, which prefixes every class selector with the module name. So `.title` in `ChapterHeading.module.css` becomes `.ChapterHeading_title`. The HTML uses the prefixed names directly. Do NOT copy or modify these files; they're already prepared.
- **HTML** — uses the prefixed camelCase class names from the scoped modules. No inline styles unless the source TSX uses them (some components like `FinisMarker` and `folio-refresh-button` do — mirror their inline styles literally).
- **Chrome** — `chrome.js` injects FolioBar + FloatingNavIsland + AtmosphereLayer based on `<body data-…>` attributes. You don't render these yourself.
- **Fonts** — `_shared/fonts.css` provides @font-face for DM Sans, Newsreader, JetBrains Mono, Montserrat. Link it FIRST in the head.

## Required `<head>` structure

```html
<link rel="stylesheet" href="../_shared/fonts.css">
<link rel="stylesheet" href="../_shared/styles/design-tokens.css">
<link rel="stylesheet" href="../_shared/styles/AtmosphereLayer.module.css">
<link rel="stylesheet" href="../_shared/styles/FolioBar.module.css">
<link rel="stylesheet" href="../_shared/styles/FloatingNavIsland.module.css">
<link rel="stylesheet" href="../_shared/styles/MagazinePageLayout.module.css">
<!-- + each module CSS used by the assigned page (already in _shared/styles/) -->
<link rel="stylesheet" href="../_shared/chrome.css">
<link rel="stylesheet" href="../_shared/inspector.css">
```

## Required `<body>` data attributes

Match the page's `MagazinePageLayout` shellConfig from `useRegisterMagazineShell()`:

```html
<body
  data-folio-label="<publicationLabel>"
  data-folio-crumbs="<crumbA>><crumbB>"        <!-- only if breadcrumbs are set -->
  data-tint="<atmosphereColor>"                <!-- turmeric|larkspur|terracotta|olive|eucalyptus -->
  data-active-page="<activePage>"              <!-- today|emails|actions|me|people|accounts|projects|dropbox|settings -->
  data-nav-base="."
  data-folio-date="<dateText>"                 <!-- only if folioDateText is set -->
  data-folio-readiness="<csv with sage/terracotta colors>"   <!-- only if folioReadinessStats set -->
  data-folio-actions="<csv: refresh,reports,tools>"          <!-- only if folioActions set -->
  data-folio-refresh-title="<exact title from folio-refresh-button props>">
```

## MagazinePageLayout wrapping

Wrap page content in:

```html
<div class="MagazinePageLayout_magazinePage">
<main class="MagazinePageLayout_pageContainer">
  <!-- surface-specific page container -->
</main>
</div>
```

## Mirror the TSX 1:1

For your assigned page:

1. **Read the TSX file** at `src/pages/<Page>.tsx`. Identify every imported component.
2. **Read each component's TSX + module CSS** to understand the DOM it produces and the class names it uses.
3. **Read the matching mock fixture** at `.docs/fixtures/parity/mock/<page>.json` if one exists (e.g. `dashboard_briefing.json`, `account_detail.json`, `meeting_detail.json`, `project_detail.json`, `person_detail.json`, `actions.json`, `inbox_emails.json`, `settings_data.json`).
4. **Use scoped class names** from `_shared/styles/` for every element. Class name format: `{ModuleName}_{classname}`. E.g. `ChapterHeading_title`, `EditableVitalsStrip_strip`.
5. **Mirror inline styles literally** when the source TSX has them (FinisMarker, folio-refresh-button, etc.). Don't paraphrase.

## Critical rules — codex adversarial review found these gotchas

1. **No invented intelligence**: don't add HealthBadges, IntelligenceQualityBadges, random scores, prep dots, or pull quotes unless the TSX explicitly renders them in the data path you're mirroring. If `m.entityHealthMap[id]` would be empty in the real app, omit the badge.
2. **Use real helper output formats**:
   - `formatEntityByline(entities)` → `{Name} · Customer|Project|1:1` (single primary entity, NOT multi-person joined strings). See `src/lib/entity-helpers.ts:64-75`.
   - `formatDurationFromIso(start, end)` → `45m` / `1h` / `1h 30m` (NOT `60m`). See `src/lib/meeting-time.ts:15-30`.
   - Date ranges use abbreviated months and en dashes, e.g. `Apr 20 – Apr 24`.
3. **Preserve TSX-driven class combinations** from the source component. Don't drop state classes just because the static fixture is non-interactive.
4. **No h1 in page header** — the folio bar carries the page identity. Page-level title comes from `data-folio-label`.
5. **Drop `editorial-reveal` classes** — they're for scroll-triggered fade-in animations driven by an observer hook (`useRevealObserver`). Static reference doesn't need them.
6. **Outcome SVG attrs**: `width="12"` `height="12"` `stroke-width="2"`. Match the TSX exactly.
7. **FinisMarker**: three BrandMark SVGs centered, gap `0.4em`, color `var(--color-spice-turmeric)`. Don't add a "Last updated" timestamp unless the page passes `enrichedAt`.

## Mock data style

Use the names already established across other surfaces for consistency:

- **Customers**: Acme Corp, Globex Inc, Northwind Traders, Stark Industries, Meridian Harbor
- **People**: Jen Park, Dan Mitchell, Sara Wu, Marco Devine, Aoife Murphy, Liu Kang, Priya Raman, Kevin Otieno
- **Projects**: Q2 Launch, MSA Renewal, Helpline Rollout
- **User**: Sam Chen, Senior CSM at a generic company
- **Today**: Wed Apr 22, 2026 (week 17)

## Verification

When the file is loaded at `http://localhost:8765/surfaces/<surface>.html`:

- Folio bar should show the right BrandMark + label + (breadcrumbs|date) + readiness/refresh/⌘K
- Right-margin nav island should show BrandMark + correct icons + active tint
- Page typography should match the actual app (Newsreader serif for editorial, DM Sans body, JetBrains Mono for labels/dates)
- No console errors about missing classes (every className in HTML must exist in a loaded scoped module)

## Gold template

Use the closest active surface reference as the template for the page family you
are mirroring. Match its quality bar.
