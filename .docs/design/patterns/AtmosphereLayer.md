# AtmosphereLayer

**Tier:** pattern
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `AtmosphereLayer`
**`data-ds-spec`:** `patterns/AtmosphereLayer.md`
**Variants:** `tint="turmeric" | "larkspur" | "terracotta" | "eucalyptus" | "olive" | "sage"`
**Design system version introduced:** 0.1.0

## Job

The page-tinted radial-gradient background layer that sits behind every editorial surface. Slow opacity-breath animation (~14s cycle). Provides the warm "morning light" feel that distinguishes DailyOS from a typical app.

## When to use it

- Every full-page editorial surface — auto-injected by `chrome.js` based on `<body data-tint>`
- Per-surface tint follows the surface's primary entity / theme (briefing → turmeric, MePage → eucalyptus, ProjectDetail → olive, etc.)

## When NOT to use it

- Modal dialogs, popovers, onboarding screens
- Print contexts (atmosphere is screen-only)

## Composition

Two overlapping radial gradients in a fixed full-viewport layer. Per-tint variables (`--atm-color-1`, `--atm-color-2`) set the gradient colors; `data-tint` selects the variant.

```
radial-gradient(ellipse 900px 700px at 20% 15%, atm-color-1, transparent 60%)
+ radial-gradient(ellipse 700px 500px at 85% 70%, atm-color-2, transparent 55%)
```

Animated via `atmosphere-breathe` keyframe (opacity 0.9 ↔ 1, 14s ease-in-out, infinite).

## Variants

| Tint | atm-color-1 | atm-color-2 | Used on |
|---|---|---|---|
| `turmeric` | `rgba(201,162,39,0.09)` | `rgba(201,162,39,0.06)` | DailyBriefing, AccountDetail (default) |
| `larkspur` | `rgba(143,163,196,0.10)` | `rgba(143,163,196,0.06)` | PeoplePage, PersonDetail |
| `terracotta` | `rgba(196,101,74,0.09)` | `rgba(196,101,74,0.05)` | Surfaces signaling urgency |
| `eucalyptus` | `rgba(107,168,164,0.10)` | `rgba(107,168,164,0.06)` | MePage |
| `olive` | `rgba(107,124,82,0.10)` | `rgba(107,124,82,0.06)` | ProjectsPage, ProjectDetail |
| `sage` | `rgba(126,170,123,0.10)` | `rgba(126,170,123,0.06)` | success-themed surfaces |

## Tokens consumed

- `--z-atmosphere` (z-index 0)
- Per-tint underlying paint tokens (turmeric, larkspur, etc.) — but rendered as `rgba()` opacity values directly because the gradient needs alpha control

## API sketch

Mockup form (auto-injected via chrome.js):

```html
<body data-tint="larkspur">
  <!-- chrome.js prepends: <div class="atmosphere" data-tint="larkspur"></div> -->
</body>
```

Production form:

```tsx
<AtmosphereLayer tint="eucalyptus" />
```

## Source

- **Code:** `src/components/layout/AtmosphereLayer.tsx` + `AtmosphereLayer.module.css`
- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/_shared/chrome.css` (`.atmosphere`)

## Surfaces that consume it

Every editorial surface. Tint per surface:
- DailyBriefing — `turmeric`
- AccountDetail — `turmeric` (or per account preference)
- ProjectDetail — `olive`
- PersonDetail — `larkspur`
- MePage — `eucalyptus`
- MeetingDetail — `turmeric` (default; may inherit from primary entity)
- Settings — `turmeric`

## Naming notes

`AtmosphereLayer` — describes its role (atmosphere, behind everything else). Mockup class is `.atmosphere`. Don't rename to `Background` (too generic).

## History

- 2026-05-02 — Promoted to canonical (existing production component).
