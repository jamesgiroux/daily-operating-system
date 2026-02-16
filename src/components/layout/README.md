# Magazine-Layout Editorial Design System

## Overview

This directory contains the core layout components for DailyOS's magazine-layout editorial redesign (ADR-0077). The system provides a complete, reusable shell for pages using the new editorial aesthetic.

**Status:** Sprint 25 complete — Shell + Account Detail editorial redesign shipped. Live data integrated.

---

## Components

### 1. `FolioBar.tsx`
**Purpose:** Editorial masthead — fixed top bar with brand identity, publication label, date, readiness stats, and search trigger.

**Props:**
```typescript
interface FolioBarProps {
  publicationLabel?: string;      // "Daily Briefing", "Account", etc.
  dateText?: string;               // "Thu, Feb 14, 2026 · Briefed 6:00a"
  readinessStats?: ReadinessStat[]; // [{ label: '4/6 prepped', color: 'sage' }]
  statusText?: string;             // ">_ ready"
  onSearchClick?: () => void;       // Search button callback
}
```

**Features:**
- Fixed position, 40px height with frosted glass (macOS native blur)
- Draggable title bar (via `-webkit-app-region: drag`)
- All interactive elements have `no-drag` to remain clickable
- Responsive hover states on search button
- Semantic `<header>` tag

**Styling:**
- Uses CSS Modules (FolioBar.module.css)
- All colors/spacing from design tokens (src/styles/design-tokens.css)
- Montserrat 800 for brand mark, DM Sans for labels, JetBrains Mono for date

---

### 2. `FloatingNavIsland.tsx`
**Purpose:** Right-margin floating toolbar with icon-based navigation, tooltips, and page-specific active state.

**Props:**
```typescript
interface FloatingNavIslandProps {
  mode?: 'app' | 'chapters';        // 'app' = page nav, 'chapters' = scroll nav
  activePage?: 'today' | 'week' | 'inbox' | 'actions' | 'people' | 'accounts' | 'settings';
  activeColor?: 'turmeric' | 'terracotta' | 'larkspur';
  onNavigate?: (page: string) => void;
  onHome?: () => void;
  chapters?: ChapterItem[];          // Chapter definitions for chapter mode
  activeChapterId?: string;          // Currently active chapter (chapter mode)
}
```

**Features:**
- Fixed position: 28px from right, centered vertically
- **App mode:** 3 nav groups separated by dividers (main, entity, admin)
- **Chapter mode:** Icon-based scroll navigation with smooth 800ms easeInOutCubic scroll
- Hover tooltips using CSS `::after` pseudo-element with `data-label`
- Color-coded active states (turmeric/terracotta/larkspur)
- Lucide React icons (18px, 1.8px stroke)
- Smooth transitions (0.15s)
- Semantic `<nav>` tag

**Icons:**
- Today: Grid3x3
- This Week: Calendar
- Inbox: Inbox
- Actions: CheckSquare2
- People: Users
- Accounts: Building2
- Settings: Settings

---

### 3. `AtmosphereLayer.tsx`
**Purpose:** Fixed-position atmospheric background with page-specific radial gradients and breathing animation.

**Props:**
```typescript
interface AtmosphereLayerProps {
  color?: 'turmeric' | 'terracotta' | 'larkspur'; // Gradient color scheme
  className?: string;
}
```

**Features:**
- Fixed position, covers full viewport (100vh)
- Z-index: 0 (behind all content)
- Pointer-events: none (non-interactive)
- Page-specific gradients (warm, urgent, forward-looking)
- Subtle watermark asterisk (rotated 12deg, 420px)
- Breathing animation (12s cycle, opacity 0.8–1.0)

**Gradient Variants:**
- **Turmeric:** Warm glow (customer-heavy days)
- **Terracotta:** Urgent glow (action-focused days)
- **Larkspur:** Expansive glow (people/forward-looking days)

---

### 4. `MagazinePageLayout.tsx`
**Purpose:** Wrapper component combining all four shell elements + page container. Complete magazine-layout editorial shell.

**Props:**
```typescript
interface MagazinePageLayoutProps {
  heroSection: React.ReactNode;
  children: React.ReactNode;
  atmosphereColor?: 'turmeric' | 'terracotta' | 'larkspur';
  activePage?: 'today' | 'week' | 'inbox' | 'actions' | 'people' | 'accounts' | 'settings';
  folioLabel?: string;
  folioDate?: string;
  readinessStats?: ReadinessStat[];
  statusText?: string;
  onFolioSearch?: () => void;
  onNavigate?: (page: string) => void;
  onNavHome?: () => void;
}
```

**Usage:**
```tsx
<MagazinePageLayout
  folioLabel="Daily Briefing"
  folioDate="Thu, Feb 14, 2026 · Briefed 6:00a"
  readinessStats={[{ label: '4/6 prepped', color: 'sage' }]}
  activePage="today"
  atmosphereColor="turmeric"
  onNavigate={(page) => navigate(page)}
  heroSection={<Hero headline="..." narrative="..." />}
>
  {/* Page content sections */}
  <FocusSection ... />
  <FeaturedMeeting ... />
  <Schedule ... />
</MagazinePageLayout>
```

**Layout Structure:**
```
<div class="magazine-page">
  <AtmosphereLayer />           <!-- z-index: 0, behind all -->
  <FolioBar />                  <!-- z-index: 100, fixed top -->
  <FloatingNavIsland />         <!-- z-index: 100, fixed right -->
  <main class="page-container"> <!-- z-index: 1, above atmosphere -->
    <section class="hero-section"> {heroSection} </section>
    {children}
  </main>
</div>
```

---

## Design Tokens

All styling uses CSS custom properties defined in `src/styles/design-tokens.css`. Key tokens:

### Colors (ADR-0076)
```css
--color-paper-cream: #f5f2ef;
--color-spice-turmeric: #c9a227;
--color-spice-terracotta: #c4654a;
--color-garden-larkspur: #8fa3c4;
--color-garden-sage: #7eaa7b;
--color-text-primary: #1e2530;
--color-text-secondary: #5a6370;
--color-text-tertiary: #8a919a;
```

### Typography (ADR-0073)
```css
--font-serif: 'Newsreader', Georgia, serif;    /* Headlines, narrative */
--font-sans: 'DM Sans', -apple-system, serif;  /* Body, UI text */
--font-mono: 'JetBrains Mono', monospace;      /* Dates, times, labels */
--font-mark: 'Montserrat', sans-serif;         /* Brand mark (800 weight) */
```

### Spacing (ADR-0073)
```css
--space-xs: 4px;
--space-sm: 8px;
--space-md: 16px;
--space-lg: 24px;
--space-xl: 32px;
--space-2xl: 48px;
--space-3xl: 56px;
--space-4xl: 72px;
--space-5xl: 80px;
```

### Layout
```css
--folio-height: 40px;
--page-padding-horizontal: 120px;
--page-max-width: 1100px;
--nav-island-right: 28px;
```

---

## CSS Organization

Each component uses **CSS Modules** for scoped styling:

```
src/components/layout/
├── FolioBar.tsx
├── FolioBar.module.css       ← Scoped FolioBar styles
├── FloatingNavIsland.tsx
├── FloatingNavIsland.module.css
├── AtmosphereLayer.tsx
├── AtmosphereLayer.module.css
├── MagazinePageLayout.tsx
├── MagazinePageLayout.module.css
└── README.md
```

Each `.module.css` file:
1. Imports `../../styles/design-tokens.css` for access to CSS variables
2. Uses CSS custom properties instead of hardcoded values
3. Includes brief comments explaining each section
4. Follows 3-tier naming: component (e.g., `.folio`), sub-component (e.g., `.folioLeft`), variant (e.g., `.folioMark`)

---

## Font Setup

Fonts are imported via Google Fonts in `index.html`:

```html
<link href="https://fonts.googleapis.com/css2?family=Newsreader:...&family=DM+Sans:...&family=JetBrains+Mono:...&family=Montserrat:wght@800&display=swap" rel="stylesheet">
```

**Already configured in the project.** No additional setup needed.

---

## Usage Examples

### Basic Daily Briefing Page
```tsx
import MagazinePageLayout from '@/components/layout/MagazinePageLayout';

export function BriefingPage() {
  return (
    <MagazinePageLayout
      folioLabel="Daily Briefing"
      folioDate={new Date().toLocaleDateString()}
      readinessStats={[{ label: '4/6 prepped', color: 'sage' }]}
      statusText=">_ ready"
      activePage="today"
      atmosphereColor="turmeric"
      heroSection={
        <div>
          <h1>Your day is customer-heavy</h1>
          <p>4 external meetings, 2 with renewal implications.</p>
        </div>
      }
    >
      <FocusSection />
      <FeaturedMeeting />
      <Schedule />
    </MagazinePageLayout>
  );
}
```

### Account Detail Page
```tsx
<MagazinePageLayout
  folioLabel={account.name}
  atmosphereColor="turmeric"
  activePage="accounts"
  heroSection={<AccountHero account={account} />}
>
  <AccountMetrics />
  <AccountContacts />
  <AccountHistory />
</MagazinePageLayout>
```

### Actions List Page
```tsx
<MagazinePageLayout
  folioLabel="Actions"
  readinessStats={[{ label: '12 overdue', color: 'terracotta' }]}
  activePage="actions"
  atmosphereColor="terracotta"
  heroSection={<h1>Your action queue</h1>}
>
  <ActionsList actions={actions} />
</MagazinePageLayout>
```

---

## Testing & Validation

### Visual Testing
1. Navigate to any account detail page to see all components in context
2. Verify folio bar is exactly 40px tall
3. Verify nav island is 28px from right edge, centered vertically
4. Verify atmosphere gradient is visible but subtle (should see it, not overwhelming)
5. Verify watermark asterisk is visible behind hero text
6. Hover over nav items to see tooltips appear

### Browser DevTools
- Inspect folio bar: should have `backdrop-filter: blur(12px)` + `-webkit-backdrop-filter`
- Inspect nav items: active state should have matching background color (e.g., `rgba(201, 162, 39, 0.1)`)
- Inspect atmosphere: should have radial gradients with proper opacity values

### Accessibility
- Tab through nav items — all should be reachable
- Folio search button should be focusable with blue outline on focus
- No console errors or warnings

---

## Responsive Behavior

**Current design targets 1440px width** (macOS secondary display standard).

Responsive breakpoints included in `MagazinePageLayout.module.css`:

| Breakpoint | Padding | Notes |
|-----------|---------|-------|
| 1200px+ | 120px (horizontal) | Default, desktop |
| 768px–1199px | 80px (horizontal) | Tablet landscape |
| 480px–767px | 48px (horizontal) | Tablet portrait |
| <480px | 24px (horizontal) | Mobile (Phase 7) |

---

## macOS-Specific Features

These components use Tauri-specific CSS for native macOS feel:

```css
-webkit-app-region: drag;      /* Draggable title bar */
-webkit-app-region: no-drag;   /* Clickable inside draggable region */
-webkit-backdrop-filter: blur(12px); /* Frosted glass */
```

**Do not remove these.** They're necessary for native app feel on macOS.

---

## Known Limitations

- Responsive design partially implemented (focus on 1440px desktop)
- Mobile view not prioritized (future enhancement)
- No dark mode (future enhancement)

---

## References

- **ADR-0077:** Magazine-Layout Editorial Redesign (`daybreak/docs/decisions/0077-magazine-layout-editorial-redesign.md`)
- **ADR-0076:** Brand Identity (`daybreak/docs/decisions/0076-brand-identity.md`)
- **ADR-0073:** Typography & Spacing (`daybreak/docs/decisions/0073-typography-spacing.md`)
- **CLAUDE.md:** Project instructions (`CLAUDE.md`, section: Code Discipline)

---

## Support

For questions about component usage, props, or styling, refer to:
1. Component JSDoc comments (each `.tsx` file)
2. CSS comments in `.module.css` files
3. Design token definitions in `src/styles/design-tokens.css`
4. ADR-0077 and ADR-0076 for design philosophy
