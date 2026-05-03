# FloatingNavIsland

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `FloatingNavIsland`
**`data-ds-spec`:** `patterns/FloatingNavIsland.md`
**Variants:** dual-pill (default), chapter-only (OnboardingFlow), per-page tint
**Design system version introduced:** 0.1.0

## Job

The right-margin "Dynamic Island" navigation pattern. Two pills displayed simultaneously:

- **Global pill** (right): always-visible icon-based app navigation
- **Local pill** (left): chapter / section navigation that appears when the surface provides `chapters`

The pills merge visually where they overlap (shared edge loses border-radius). The local pill aligns vertically so its top matches the active icon in the global pill (with overflow protection).

This is the **canonical local-nav pattern for DailyOS** per design system D2 (synthesis). Surfaces that need in-page navigation provide `chapters`; they do not invent new local-nav patterns (no `DayStrip`, no `SectionTabbar`).

## When to use it

- Every stable editorial surface (briefing, account detail, project detail, person detail, settings, etc.) — provides app-level wayfinding
- When a surface has discrete sections worth scroll-spying — provide `chapters` to render the local pill
- OnboardingFlow uses chapter-only mode (no global pill)

## When NOT to use it

- For action toolbars — use `FolioActions` (Wave 4)
- For modals / dialogs — they don't get nav
- For deep-linking external pages — use surface-internal links instead

## Composition

### Global pill (always-present)

Vertical stack of icon buttons in the canonical app order:

| Group | Items |
|---|---|
| Brand | Today (Brand mark — home button) |
| Time | Week |
| Work | Mail, Actions |
| Entities | Me, People, Accounts/Projects (order configurable per role preset) |
| Tools | Inbox, Settings |

Dividers separate groups. Active item gets the surface tint color (turmeric / larkspur / terracotta / olive / eucalyptus). The "Me" item shows a content dot if the user's profile needs filling in.

### Local pill (conditional)

Renders when the surface provides `chapters` prop. Each chapter is `{id, label, icon}`. Click scrolls to the section (smooth-scroll); active chapter gets surface-tint highlight via scroll-spy.

Visual alignment math: local pill top aligns with active global icon when possible, clamps to global pill bounds when alignment would overflow.

### Chapter-only mode

When `onNavigate` is absent (e.g., OnboardingFlow), only the local pill renders — single-pill mode.

## Variants

- **Default dual-pill** — global + (optional) local
- **Chapter-only** — local pill only (no global), used by OnboardingFlow
- **Tint variants** — `activeColor="turmeric" | "terracotta" | "larkspur" | "olive" | "eucalyptus"` per surface tint

## Tokens consumed

- `--frosted-glass-nav` (background)
- `--backdrop-blur`
- `--nav-island-right` (right offset)
- `--radius-editorial-md` (item), `--radius-editorial-xl` (container)
- `--color-rule-light` (border, divider)
- `--color-text-tertiary` → `--color-text-secondary` (default → hover)
- Active state per tint: `--color-spice-turmeric-10` + `--color-spice-turmeric` (and four other tint variants)
- `--space-sm` (container padding), `--space-xs` (item gaps)
- `--shadow-md`
- `--z-app-shell`
- `--transition-fast` (hover, color change)

## API sketch

Production component already exists with this contract:

```tsx
<FloatingNavIsland
  activePage="accounts"
  activeColor="turmeric"
  entityMode="account" // "account" | "project" | "both"
  onNavigate={(page) => router.navigate(page)}
  onHome={() => router.navigate("today")}
  chapters={[
    { id: "today", label: "Today", icon: <Calendar size={18} /> },
    { id: "moving", label: "Moving", icon: <TrendingUp size={18} /> },
    { id: "watch", label: "Watch", icon: <Eye size={18} /> },
    { id: "ask", label: "Ask", icon: <MessageCircle size={18} /> },
  ]}
  activeChapterId="today"
  onChapterClick={(id) => setActiveChapter(id)}
/>
```

Chapter-only mode: pass `chapters` without `onNavigate`.

## Source

- **Code:** `src/components/layout/FloatingNavIsland.tsx` + `FloatingNavIsland.module.css` — production, canonical
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/chrome.css` (`.nav-island`) + `chrome.js` `buildNav()` — single-pill simplified version of the production component (does not implement local pill or chapters); for spec / mockup purposes the production component is canonical

## Surfaces that consume it

Every stable surface. Each surface defines its `chapters` inventory:

- **DailyBriefing** — `Today / Moving / Watch / Ask` (per surface spec)
- **Settings** — `Identity / Connectors / Briefing / Data / Activity / System / Diagnostics`
- **AccountDetail** — provides chapters per the active view (Health / Context / Work)
- **MeetingDetail** — likely no chapters (short single-purpose surface)

## Naming notes

`FloatingNavIsland` — keeps the existing component name and the "Dynamic Island" metaphor. **Do not** rename to `Sidebar`, `Nav`, `LocalNav`, etc. The mockups invented `DayStrip` (D-spine) and `SectionTabbar` (Settings) as alternatives — both rejected per D2; this pattern subsumes both via the chapters contract.

## History

- 2026-05-02 — Promoted to canonical (existing production component).
- D2 (synthesis) — confirmed as canonical local-nav pattern; rejected DayStrip / SectionTabbar alternatives.
- DS-XCUT-05 (DOS-361) — drives surface adoption of chapters contract.
