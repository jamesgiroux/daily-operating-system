# FolioBar

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `FolioBar`
**`data-ds-spec`:** `patterns/FolioBar.md`
**Variants:** action sets driven by `data-folio-actions` (refresh, search, reports, tools, status-dot); `mark="pulsing"` for live status
**Design system version introduced:** 0.1.0

## Job

The fixed top frosted bar of every editorial surface. Carries surface label, breadcrumbs, status text, and an action set on the right. Always present (along with `FloatingNavIsland`) on stable surfaces.

## When to use it

- Every full-page editorial surface that needs an app shell (briefing, account detail, project detail, person detail, meeting detail, settings)
- Auto-injected by `chrome.js` based on `<body data-*>` attributes (mockup substrate convention)

## When NOT to use it

- Modal dialogs and popovers (use their own chrome)
- Onboarding flows (different chrome stack — see OnboardingFlow)
- Per-section toolbars (use `FolioActions` Wave 4 pattern instead)

## Composition

Three regions in a horizontal flex:

- **Left:** brand mark (asterisk, optionally pulsing for "live" status) + surface label (`data-folio-label`) + optional breadcrumbs (`data-folio-crumbs`, `>`-separated)
- **Center:** optional status text (`data-folio-status`) — italic mono, e.g., "Auto-saved · just now" or center timestamp like "THU · APR 23 · LIVE"
- **Right:** action set per `data-folio-actions` csv — combinations of `refresh`, `search` (⌘K), `reports`, `tools`, `status-dot`

Frosted glass background via `--frosted-glass-background` + `backdrop-filter: blur(12px)`.

## Variants

- **Default** — surface label + crumbs + standard action set (refresh + search)
- **Live status** — `data-folio-mark="pulsing"` adds slow opacity pulse to brand mark
- **Reports/Tools surfaces** (e.g., AccountDetail) — adds `reports` and `tools` actions
- **Settings** — minimal action set (`search` only)

## Tokens consumed

- `--frosted-glass-background`
- `--backdrop-blur` (`blur(12px)`)
- `--folio-height`, `--folio-padding-*`
- `--font-mark` (asterisk), `--font-mono` (label, status, action labels)
- `--color-spice-turmeric` (brand mark, accent action border)
- `--color-text-tertiary` (label, status, default action color)
- `--color-rule-light` (bottom border)
- `--color-desk-charcoal-4` (action hover)
- `--z-app-shell`

## API sketch

Mockup form (auto-injected via chrome.js):

```html
<body
  data-folio-label="Account"
  data-folio-crumbs="Accounts > Acme Corp"
  data-folio-status="Live"
  data-folio-actions="refresh,reports,tools,search"
  data-folio-mark="pulsing"
>
```

Production form (when implemented as React pattern in `src/components/layout/`):

```tsx
<FolioBar
  label="Account"
  crumbs={["Accounts", "Acme Corp"]}
  status="Live"
  actions={["refresh", "reports", "tools", "search"]}
  markPulsing
/>
```

## Tokens-only changes accepted

Tinting per surface tint (`data-tint="larkspur" | "terracotta" | ...`) — the brand mark color follows surface tint.

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/chrome.css` (`.folio`) + `chrome.js` `buildFolio()`
- **Code:** to be implemented as React pattern in `src/components/layout/FolioBar.tsx` (currently rendered by per-page React + a magazine layout shell — see `src/components/layout/MagazinePageLayout.tsx`)

## Surfaces that consume it

Every editorial surface: DailyBriefing, AccountDetail, ProjectDetail, PersonDetail, MeetingDetail, Settings, AccountsPage, ProjectsPage, PeoplePage, MePage, etc.

## Naming notes

`FolioBar` — kept the editorial / publishing metaphor (folio = page header in a magazine). Don't rename to "TopBar", "AppBar", etc.

## History

- 2026-05-02 — Promoted to canonical from `_shared/.folio` + production magazine layout shell.
