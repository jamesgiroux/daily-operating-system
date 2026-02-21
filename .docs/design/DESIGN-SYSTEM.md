# DailyOS Design System

**Source of truth for all visual decisions.** This document consolidates ADRs 0073, 0076, 0077, 0083, and 0084 into a single actionable reference.

---

## 1. Typography

Type does the structural work. Borders, containers, and dividers are secondary.

### Font Stack

| Role | Family | Token | Usage |
|------|--------|-------|-------|
| Headlines, narrative | Newsreader | `var(--font-serif)` | Page titles, section headings, hero narratives, pull quotes |
| Body, UI | DM Sans | `var(--font-sans)` | Body text, labels, buttons, navigation, form inputs |
| Data, timestamps | JetBrains Mono | `var(--font-mono)` | Times, dates, metrics, mono labels, code |
| Brand mark only | Montserrat 800 | `var(--font-mark)` | The asterisk `*` mark. Nothing else. |

### Type Scale

| Level | Size | Family | Weight | Letter-spacing | Usage |
|-------|------|--------|--------|---------------|-------|
| Page headline | 76px | Newsreader | 400 | -0.025em | Page hero titles (daily briefing, entity name) |
| Section title | 22-28px | Newsreader | 400 | normal | ChapterHeading, section headers |
| Card/item title | 19-20px | Newsreader | 400 | normal | Meeting card titles, featured items |
| Hero narrative | 21px | Newsreader italic | 300 | normal | AI-synthesized opening statement |
| Body text | 15-16px | DM Sans | 300-400 | normal | Prose, descriptions, UI text |
| Mono label | 10-11px | JetBrains Mono | 500 | 0.08em, uppercase | Section margin labels, timestamps, metadata |
| Meta/secondary | 11-13px | DM Sans | 400 | normal | Tertiary text, hints |

### Rules

- The size jump between levels creates hierarchy without chrome
- If you need a border to tell sections apart, the type scale isn't working
- Pages open with a narrative voice — the headline is a conclusion, not a greeting
- Line-height: 1.06 for headlines, 1.55 for body, 1.65 for long-form prose

---

## 2. Color System

Four families named after materials. Every color earns its pixel.

### Paper (Grounds & Backgrounds) — 80%+ of viewport

| Name | Token | Hex | Role |
|------|-------|-----|------|
| Cream | `--color-paper-cream` | `#f5f2ef` | Primary background |
| Linen | `--color-paper-linen` | `#e8e2d9` | Secondary surface, alternating rows |
| Warm White | `--color-paper-warm-white` | `#faf8f6` | Elevated surfaces (cards, modals) |

### Desk (Frame & Structure) — App chrome, primary text

| Name | Token | Hex | Role |
|------|-------|-----|------|
| Charcoal | `--color-desk-charcoal` | `#1e2530` | Primary text, app frame |
| Ink | `--color-desk-ink` | `#2a2b3d` | Deep code backgrounds |
| Espresso | `--color-desk-espresso` | `#3d2e27` | Tertiary dark, hover states |

### Spice (Warm Accents) — No more than 10-15% of viewport

| Name | Token | Hex | Role |
|------|-------|-----|------|
| Turmeric | `--color-spice-turmeric` | `#c9a227` | Primary accent, active states, accounts |
| Saffron | `--color-spice-saffron` | `#deb841` | Secondary warm, hover highlights |
| Terracotta | `--color-spice-terracotta` | `#c4654a` | Attention, overdue, urgency, actions |
| Chili | `--color-spice-chili` | `#9b3a2a` | Critical/destructive, rarely used |

### Garden (Cool Accents) — Calm, success, completion

| Name | Token | Hex | Role |
|------|-------|-----|------|
| Sage | `--color-garden-sage` | `#7eaa7b` | Success, healthy, complete |
| Olive | `--color-garden-olive` | `#6b7c52` | Projects, muted secondary |
| Rosemary | `--color-garden-rosemary` | `#4a6741` | Deep green, pressed/hover |
| Larkspur | `--color-garden-larkspur` | `#8fa3c4` | People, informational, atmospheric |

### Entity Color Mapping

| Entity | Color | Token |
|--------|-------|-------|
| Accounts | Turmeric | `--color-entity-account` |
| Projects | Olive | `--color-entity-project` |
| People | Larkspur | `--color-entity-person` |
| Actions | Terracotta | `--color-entity-action` |

Entity color = identity (accent bars, icon fills, page headers). State color = status (pills, badges, progress). They coexist.

### Semantic Text Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `--color-text-primary` | Charcoal | Headlines, primary text |
| `--color-text-secondary` | `#5a6370` | Body text, secondary |
| `--color-text-tertiary` | `#6b7280` | Labels, hints (WCAG AA compliant) |

### Color Rules

1. **Paper fills the page.** 80%+ of any viewport is Paper family.
2. **Spice draws attention.** Max 10-15% of viewport. If everything is spice, nothing communicates.
3. **Every color earns its pixel.** If you remove the color and meaning is unchanged, remove it.
4. **No cross-family mixing.** Spice on Paper (yes). Garden on Paper (yes). Spice on Garden (never).
5. **Briefing surfaces use constrained subset:** Cream, Warm White, Charcoal, Turmeric, Terracotta, Sage, Larkspur only.

### Opacity Variants (for backgrounds, tints)

When using a palette color at reduced opacity (for backgrounds, tints, hover states), use the base color's rgba value. **Always reference the base hex in a comment:**

```css
/* Turmeric tint — from --color-spice-turmeric #c9a227 */
background: rgba(201, 162, 39, 0.08);
```

**TODO:** Create formal opacity tokens (e.g., `--color-spice-turmeric-8`, `--color-spice-turmeric-12`) to eliminate scattered rgba values.

---

## 3. Spacing

Base 4px grid. Use the token scale.

| Token | Value | Common uses |
|-------|-------|-------------|
| `--space-xs` | 4px | Tight gaps, icon margins |
| `--space-sm` | 8px | Inline spacing, small gaps |
| `--space-md` | 16px | Standard padding, card gaps |
| `--space-lg` | 24px | Section padding, generous gaps |
| `--space-xl` | 32px | Large gaps |
| `--space-2xl` | 48px | Between sections |
| `--space-3xl` | 56px | Major section breaks |
| `--space-4xl` | 72px | Page-level spacing |
| `--space-5xl` | 80px | Hero padding |

### Rules

- Never use arbitrary pixel values in inline styles. Map to the scale.
- `gap: 20` is not on the scale. Use `--space-md` (16) or `--space-lg` (24).
- Between sections: 48-64px (`--space-2xl` to `--space-3xl`)
- Inside cards: 28-32px padding (`--space-lg` to `--space-xl`)
- Between action items: 18px is close enough to `--space-md` — round to the scale.

---

## 4. Layout

### Page Container

```css
max-width: var(--page-max-width);  /* 1100px */
padding: 0 var(--page-padding-horizontal);  /* 120px sides */
margin-top: var(--page-margin-top);  /* 40px, clears folio */
padding-bottom: var(--page-padding-bottom);  /* 160px */
```

### Margin Grid Pattern

The editorial margin grid places labels in a left column and content in a right column:

```
┌─────────────┬────┬──────────────────────────────┐
│ MONO LABEL  │gap │ Content flows here...         │
│ (100px)     │32px│                               │
└─────────────┴────┴──────────────────────────────┘
```

**Implementation:** CSS module classes `s.marginGrid`, `s.marginLabel`, `s.marginContent` in `editorial-briefing.module.css`. Use these — don't reinvent.

### Section Rules

- Thin horizontal dividers: `1px solid var(--color-rule-heavy)`
- No cards for most content. Dividers + spacing = structure.
- Entity accent borders: 3px left border in entity color

### Cards (Featured Content Only)

Cards with background + shadow are reserved for:
- Meeting cards (the primary content unit)
- Priority items in weekly briefing
- Signal cards (wins/risks)
- Focus callout (turmeric-bordered pull quote)

**Card style:**
- Background: Warm White on Cream
- Border-radius: `var(--radius-editorial-xl)` (16px)
- Shadow: `var(--shadow-md)`
- Hover: slightly deeper shadow for interactive cards

Everything else (action rows, stakeholder rows, metadata, timeline items) = styled text rows with thin dividers.

### Border Radius Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--radius-editorial-sm` | 4px | Small elements, search button |
| `--radius-editorial-md` | 10px | Nav island items |
| `--radius-editorial-lg` | 12px | Featured actions box |
| `--radius-editorial-xl` | 16px | Nav island container, cards |

### Z-Index Stack

| Token | Value | Usage |
|-------|-------|-------|
| `--z-atmosphere` | 0 | Background gradient layer |
| `--z-page-content` | 1 | Main content |
| `--z-app-shell` | 100 | Folio bar, nav island |

---

## 5. App Chrome

### Folio Bar (`FolioBar.tsx`)

- Fixed top, 40px height
- Frosted glass: `rgba(cream, 0.85)` + `blur(12px)`
- Left: Brand mark `*` (Montserrat 800, turmeric, 18px) + page label
- Center: Date/time (JetBrains Mono)
- Right: Context-specific actions + status indicator

### Floating Nav Island (`FloatingNavIsland.tsx`)

- Fixed right margin (28px from edge), vertically centered
- Frosted glass background
- Icon-based with tooltips on hover
- Active state color varies by page (turmeric default, terracotta for actions, larkspur for weekly)

### Atmosphere Layer (`AtmosphereLayer.tsx`)

- Fixed-position radial gradients at low opacity (4-11%)
- Page-specific color (turmeric for briefing, terracotta for actions, larkspur for weekly)
- Breathing animation (12-16s depending on page tempo)

### Asterisk Watermark

- 420px Montserrat 800, rotated 12deg
- Behind hero content at z: -1
- Opacity ~7%, page-specific color

---

## 6. Editorial Patterns

### FinisMarker

Every editorial page ends with `<FinisMarker />`. This renders the `* * *` section break with a closing message. The user knows they've read everything. Non-negotiable.

### ChapterHeading

Section headers use `<ChapterHeading>` which renders Newsreader 28px with a thin rule above. Every major section on a page uses this component.

### PullQuote

The focus callout: turmeric-bordered left bar, italic serif text, cream/turmeric tint background. Used for the "one thing to know" insight on briefing pages.

### EditorialEmpty

Empty state for sections with no data. Newsreader italic title + DM Sans description. Gentle, not broken.

### StateBlock

Used in editorial pages for structured state display (working/struggling on accounts, momentum/headwinds on projects).

### Scroll-linked Reveals

Content fades in as you scroll using `.editorial-reveal` (600ms) and `.editorial-reveal-slow` (800ms). Applied via IntersectionObserver. The slower variant signals "deeper content."

---

## 7. Product Vocabulary (ADR-0083)

**If a user can see it, it uses product vocabulary. No exceptions.**

### Translation Table

| System term | User-facing term |
|-------------|-----------------|
| Entity | Use the specific type: Account, Project, Person |
| Intelligence (on a meeting) | **Briefing** |
| Intelligence (on an account/project) | **Insights** |
| Intelligence (general) | **Context** |
| Enrichment | Invisible, or "Updating" |
| Signal | **Update** or **Change** |
| Prep / prep file | **Briefing** |
| Proposed (action status) | **Suggested** |
| Archived (action status) | **Dismissed** |
| Run Briefing | **Refresh** (existing) / **Prepare my day** (cold start) |
| Refresh Intelligence | **Check for updates** |
| Reject (proposed action) | **Dismiss** |
| Entity resolution | Invisible |
| Signal bus, Bayesian fusion, Thompson Sampling | Never user-facing |

### Quality Labels (meeting intelligence)

| System level | User label | Badge style |
|-------------|-----------|-------------|
| Sparse | **New** | Grey, understated |
| Developing | **Building** | Turmeric |
| Ready | **Ready** | Sage |
| Fresh | **Updated** | Sage + dot |
| Stale | **Ready** + refresh icon | Sage, muted |

### Voice

- **Confident, not apologetic.** "Ready" not "We think we have enough."
- **Specific, not abstract.** "2 new updates about Acme" not "New signals detected."
- **Warm, not clinical.** "Building context" not "Enrichment in progress."
- **Invisible when working.** Don't narrate internal processes.

---

## 8. Interaction Patterns

### Pills Over Badges (ADR-0073)

State indicators use pill-shaped tags (border-radius: 100px) with colored dots:
- `pill-sage` + dot: Ready / Complete / Healthy
- `pill-terracotta` + dot: Needs attention / Overdue / At Risk
- `pill-turmeric` + dot: Active / Renewing / Partial
- `pill-neutral`: Informational (no dot)

Pills: 12px text, 5px 14px padding. Small, contained, purposeful.

### Inline Editing (ADR-0084, I343)

Entity data is edited inline where it's displayed. No field-editing drawers. Click to edit, same position. The StakeholderGallery inline editing pattern is the model.

**Exception:** TeamManagementDrawer (involves search + create workflows).

### Tapering Density

Content deepens early, then tapers:
- Featured/urgent items: 500 weight, full context, accent color
- Secondary items: 400 weight, standard context
- Upcoming/future items: 300-400 weight, minimal context, tertiary text

### Finite Documents

Every page has an explicit end. No infinite scroll. When you've read it, you know.

---

## 9. CSS Architecture

### Preferred approach (in order of preference)

1. **CSS modules** — `*.module.css` files co-located with components. Best for complex component styles.
2. **Tailwind utility classes** — For simple, one-off styling. Use semantic tokens (`bg-primary`, `text-muted-foreground`).
3. **Design token CSS custom properties** — `var(--color-spice-turmeric)` for values Tailwind doesn't cover.

### Forbidden

- **Inline `style={{}}` props** — Untraceable, unaudiable, unoverridable. Use CSS modules or Tailwind.
- **Hardcoded hex colors** — Always use tokens. `#c9a227` in a TSX file is a violation.
- **Hardcoded px values** that bypass the spacing scale — Map to `--space-*` tokens.
- **New font declarations** — The four fonts in design-tokens.css are the complete stack.

### File organization

```
src/styles/
  design-tokens.css          ← Single source of truth for all tokens
  editorial-briefing.module.css  ← Shared editorial layout classes

src/components/
  layout/
    *.module.css              ← Chrome component styles
  editorial/
    (no CSS modules — uses Tailwind + tokens)
  ui/
    (no CSS modules — uses Tailwind via CVA variants)
  account/
    AccountHero.module.css    ← Entity-specific hero styles
  project/
    ProjectHero.module.css
  person/
    PersonHero.module.css
```
