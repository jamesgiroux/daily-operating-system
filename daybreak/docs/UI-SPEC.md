# DailyOS UI Specification

> Design system, component library, and interaction patterns for the native Tauri app.

---

## Design Philosophy

**Zero-guilt aesthetic.** Calm, warm, professional. No urgency cues, no guilt-inducing metrics, no gamification.

**Consumption-first.** 80% reading, 20% interaction. Optimize for scannability and readability.

**Native feel.** Respects OS conventions—system tray, native notifications, dark mode, keyboard shortcuts.

---

## Color Palette

### Core Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `cream` | `#f5f2ef` | Primary background |
| `cream-dark` | `#ebe5e0` | Secondary background, hover states |
| `charcoal` | `#1a1f24` | Primary text, dark UI elements |
| `charcoal-light` | `#2d343c` | Secondary dark, code blocks |
| `white` | `#ffffff` | Cards, elevated surfaces |

### Accent Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `gold` | `#c9a227` | Primary accent, links, highlights, customer items |
| `gold-muted` | `#d4b85c` | Hover state for gold |
| `peach` | `#e8967a` | Errors, warnings, strings in code |
| `sage` | `#7fb685` | Success, personal items, positive states |

### Semantic Colors

| Token | Hex | Usage |
|-------|-----|-------|
| `text-primary` | `#1a1f24` | Body text |
| `text-secondary` | `#5a6370` | Secondary text, descriptions |
| `text-muted` | `#8a929d` | Tertiary text, timestamps |
| `border` | `rgba(26,31,36,0.1)` | Default borders |
| `border-dark` | `rgba(26,31,36,0.15)` | Emphasized borders |

### Status Colors

| Status | Text | Background |
|--------|------|------------|
| Success | `#2e7d32` | `#e8f5e9` |
| Warning | `#f57c00` | `#fff8e1` |
| Error | `#c62828` | `#ffebee` |
| Info | `#1565c0` | `#e3f2fd` |

### Tailwind Configuration

```js
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      colors: {
        cream: { DEFAULT: '#f5f2ef', dark: '#ebe5e0' },
        charcoal: { DEFAULT: '#1a1f24', light: '#2d343c' },
        gold: { DEFAULT: '#c9a227', muted: '#d4b85c' },
        peach: '#e8967a',
        sage: '#7fb685',
      },
      textColor: {
        primary: '#1a1f24',
        secondary: '#5a6370',
        muted: '#8a929d',
      },
    },
  },
};
```

---

## Typography

### Font Families

| Token | Stack | Usage |
|-------|-------|-------|
| `font-sans` | DM Sans, system-ui | Body text, UI |
| `font-mono` | JetBrains Mono, monospace | Code, times, technical |

### Font Sizes (Fluid)

| Token | Size | Usage |
|-------|------|-------|
| `text-xs` | 0.75-0.8rem | Labels, badges, timestamps |
| `text-sm` | 0.85-0.9rem | Secondary text, descriptions |
| `text-base` | 0.95-1rem | Body text |
| `text-lg` | 1.1-1.25rem | Subheadings |
| `text-xl` | 1.25-1.5rem | Card titles |
| `text-2xl` | 1.5-2rem | Section headers |
| `text-3xl` | 2-3rem | Page titles |

### Font Weights

| Token | Weight | Usage |
|-------|--------|-------|
| `font-light` | 300 | Large display text |
| `font-normal` | 400 | Body text |
| `font-medium` | 500 | Emphasis, active states |
| `font-semibold` | 600 | Headings, labels |
| `font-bold` | 700 | Strong emphasis |

---

## Spacing Scale

| Token | Value | Usage |
|-------|-------|-------|
| `space-1` | 4px | Tight gaps |
| `space-2` | 8px | Icon gaps, inline spacing |
| `space-3` | 12px | List items |
| `space-4` | 16px | Component padding |
| `space-5` | 20px | Card padding |
| `space-6` | 24px | Section gaps |
| `space-8` | 32px | Large gaps |
| `space-10` | 40px | Section separation |
| `space-12` | 48px | Major sections |

---

## Border Radius

| Token | Value | Usage |
|-------|-------|-------|
| `radius-sm` | 6px | Badges, small elements |
| `radius-md` | 8px | Inputs, buttons |
| `radius-lg` | 12px | Cards |
| `radius-xl` | 16px | Modals, large surfaces |
| `radius-full` | 9999px | Pills, avatars |

---

## Shadows

| Token | Value | Usage |
|-------|-------|-------|
| `shadow-sm` | `0 1px 2px rgba(0,0,0,0.04)` | Subtle elevation |
| `shadow-md` | `0 4px 12px rgba(0,0,0,0.06)` | Cards, dropdowns |
| `shadow-lg` | `0 4px 24px rgba(0,0,0,0.08)` | Hover states |
| `shadow-xl` | `0 8px 32px rgba(0,0,0,0.1)` | Modals, popovers |

---

## Animation System

### Timing

| Token | Duration | Usage |
|-------|----------|-------|
| `duration-fast` | 150ms | Hover states, micro-interactions |
| `duration-normal` | 250ms | State changes |
| `duration-slow` | 400ms | Page transitions |

### Easing

| Token | Value | Usage |
|-------|-------|-------|
| `ease-out-expo` | `cubic-bezier(0.16, 1, 0.3, 1)` | Entrance animations |
| `ease-out-quart` | `cubic-bezier(0.25, 1, 0.5, 1)` | Smooth deceleration |
| `ease-in-out` | `cubic-bezier(0.4, 0, 0.2, 1)` | Hover transitions |

### Entrance Animations

```css
/* Fade up (default entrance) */
@keyframes fadeInUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Fade in with scale (modals, empty states) */
@keyframes fadeInScale {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}

/* Skeleton pulse (loading) */
@keyframes skeletonPulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.7; }
}

/* Timeline dot pulse (current item) */
@keyframes pulse {
  0%, 100% { box-shadow: 0 0 0 2px var(--gold); }
  50% { box-shadow: 0 0 0 6px rgba(201, 162, 39, 0.3); }
}
```

### Staggered Entrance

Elements enter with staggered delays for visual flow:

```
Element 1: 0.1s delay
Element 2: 0.15s delay
Element 3: 0.2s delay
...
```

---

## Component Library (shadcn/ui)

### Core Components to Install

```bash
npx shadcn@latest add sidebar
npx shadcn@latest add command
npx shadcn@latest add button
npx shadcn@latest add card
npx shadcn@latest add badge
npx shadcn@latest add alert
npx shadcn@latest add dropdown-menu
npx shadcn@latest add separator
npx shadcn@latest add skeleton
npx shadcn@latest add scroll-area
```

### Sidebar

```tsx
<SidebarProvider>
  <Sidebar collapsible="icon">
    <SidebarHeader>
      <SidebarMenuButton>
        <Zap className="text-gold" />
        <span>DailyOS</span>
      </SidebarMenuButton>
    </SidebarHeader>
    <SidebarContent>
      <SidebarGroup>
        <SidebarGroupLabel>Today</SidebarGroupLabel>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton asChild>
              <a href="/"><Calendar /> Overview</a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
    </SidebarContent>
  </Sidebar>
</SidebarProvider>
```

### Cards

Use shadcn `Card` with DailyOS styling:

```tsx
<Card className="hover:-translate-y-0.5 hover:shadow-lg transition-all">
  <CardHeader>
    <CardTitle>Meeting Prep</CardTitle>
    <CardDescription>Acme Corp Sync</CardDescription>
  </CardHeader>
  <CardContent>...</CardContent>
</Card>
```

**Status variants** via className:
- Success: `bg-emerald-50 border-emerald-200 dark:bg-emerald-950`
- Warning: `bg-amber-50 border-amber-200 dark:bg-amber-950`
- Error: `bg-red-50 border-red-200 dark:bg-red-950`

### Buttons

| Variant | DailyOS Style |
|---------|---------------|
| `default` | Charcoal bg, cream text |
| `secondary` | Cream-dark bg, charcoal text |
| `ghost` | Transparent, hover cream-dark |
| `destructive` | Red bg |
| `outline` | Border only |
| `link` | Gold text, no background |

Custom gold variant:
```tsx
<Button className="bg-gold hover:bg-gold-muted text-charcoal">
  Primary Action
</Button>
```

### Badges

```tsx
<Badge variant="default">Internal</Badge>
<Badge className="bg-gold/15 text-gold">Customer</Badge>
<Badge variant="destructive">P1</Badge>
<Badge className="bg-amber-100 text-amber-700">P2</Badge>
```

### Timeline (Custom Component)

Build on shadcn primitives:

```tsx
<div className="relative">
  {/* Vertical line */}
  <div className="absolute left-6 top-0 bottom-0 w-0.5 bg-border" />

  {items.map((item) => (
    <div className="relative flex gap-4 pb-6">
      {/* Dot */}
      <div className={cn(
        "w-3 h-3 rounded-full mt-1.5 z-10",
        item.type === "customer" && "bg-gold shadow-[0_0_0_2px] shadow-gold",
        item.type === "internal" && "bg-charcoal-light",
        item.type === "personal" && "bg-sage",
        item.current && "animate-pulse"
      )} />

      {/* Content */}
      <Card className={cn(
        "flex-1",
        item.type === "customer" && "border-l-4 border-l-gold"
      )}>
        ...
      </Card>
    </div>
  ))}
</div>
```

### Empty States

```tsx
<div className="flex flex-col items-center justify-center py-12 animate-in fade-in zoom-in-95">
  <Inbox className="h-12 w-12 text-muted-foreground mb-4" />
  <h3 className="text-lg font-semibold">No briefing yet</h3>
  <p className="text-muted-foreground text-sm mb-4">
    Add files to your inbox to get started
  </p>
  <Button>Open Inbox Folder</Button>
</div>
```

### Loading States (Skeleton)

```tsx
<Card>
  <CardHeader>
    <Skeleton className="h-5 w-32" />
    <Skeleton className="h-4 w-24" />
  </CardHeader>
  <CardContent>
    <Skeleton className="h-20 w-full" />
  </CardContent>
</Card>
```

---

## Layout Patterns

### App Shell (Native App)

Uses shadcn's `Sidebar` component with `collapsible="icon"` mode for maximum content space.

```
┌──────────────────────────────────────────────────┐
│ [≡] DailyOS                    [⌘K] [◐] [⚙]     │ ← Header (48px)
├────┬─────────────────────────────────────────────┤
│ 📅 │                                             │
│ 📥 │              Content Area                   │
│ 📊 │           (fluid, centered)                 │
│ ⚙️ │                                             │
│    │                                             │
└────┴─────────────────────────────────────────────┘
  ↑
 Icon mode (collapsed)     Expands on hover/click
```

**Sidebar modes:**
- `icon` (default) — Collapsed to icons, expands on hover
- `offcanvas` — Slides in from left on mobile
- `none` — No collapse, always visible (optional for large screens)

**Header elements:**
- Toggle button (hamburger) — Expand/collapse sidebar
- App title — "DailyOS"
- Command trigger — Opens `Cmd+K` search
- Theme toggle — Light/dark/system
- Settings — Opens preferences

### Search (Command Palette)

Replace header search input with shadcn `CommandDialog`:

```tsx
// Triggered by Cmd+K / Ctrl+K
<CommandDialog open={open} onOpenChange={setOpen}>
  <CommandInput placeholder="Search files, actions, meetings..." />
  <CommandList>
    <CommandEmpty>No results found.</CommandEmpty>
    <CommandGroup heading="Today">
      <CommandItem>Overview</CommandItem>
      <CommandItem>Actions Due</CommandItem>
    </CommandGroup>
    <CommandGroup heading="Quick Actions">
      <CommandItem>Run Briefing Now</CommandItem>
      <CommandItem>Open Inbox</CommandItem>
    </CommandGroup>
  </CommandList>
</CommandDialog>
```

### Navigation (Simplified)

**Remove:** Breadcrumbs (web pattern, not native)

**Add:** Back button in header when drilling into content

```
┌──────────────────────────────────────────────────┐
│ [←] Meeting Prep: Acme Sync          [⌘K] [◐]   │
├──────────────────────────────────────────────────┤
│                                                  │
│              [Content]                           │
│                                                  │
└──────────────────────────────────────────────────┘
```

### Content Panels (Not Pages)

Native apps favor panels over full page navigation. Consider:

```
┌────┬───────────────────┬─────────────────────────┐
│    │                   │                         │
│ Nav│   List Panel      │    Detail Panel         │
│    │   (meetings)      │    (selected meeting)   │
│    │                   │                         │
└────┴───────────────────┴─────────────────────────┘
```

For MVP, single-panel is acceptable. Plan for master-detail in future.

### Dashboard Grid

```
┌────────────────────┬──────────────┐
│                    │              │
│    Main Content    │   Sidebar    │
│       (2fr)        │    (1fr)     │
│                    │              │
└────────────────────┴──────────────┘
```

Collapses to single column on smaller windows.

---

## Screen Inventory

### 1. Dashboard (Today Overview)

**Purpose:** Morning briefing consumption

**States:**
- **Loading:** Skeleton cards
- **Success:** Overview card, meeting timeline, actions panel
- **Empty:** "Run /today to generate" prompt
- **Error:** "Failed to load" with retry

**Key Components:**
- Stats row (meetings count, actions due)
- Meeting timeline with type indicators
- Action items panel with priority badges

### 2. System Tray Menu

**Purpose:** Quick access, status at a glance

**Items:**
- DailyOS label + status indicator
- "Open Dashboard" (primary action)
- Last run timestamp
- "Run Now" submenu (Today, Wrap, Inbox)
- Separator
- "Preferences..."
- "Quit DailyOS"

### 3. Notifications

**Types:**
- Briefing complete → "Your day is ready"
- Processing complete → "Inbox processed: X items"
- Error → "Briefing failed" with action

**Style:** Native OS notifications, not in-app toasts.

### 4. Onboarding Flow

**Purpose:** First-run setup

**Steps:**
1. Welcome → Explain value prop
2. Claude Code check → Verify installed + authenticated
3. Workspace selection → Choose or create folder
4. Role selection → Quick vs custom setup
5. Schedule preferences → When to run briefings
6. Confirmation → Ready to go

**Empty state after onboarding:** "Add files to your inbox to get started"

### 5. Preferences

**Sections:**
- General (workspace path, launch at login)
- Schedule (briefing times, enable/disable)
- Notifications (enable, sound)
- Advanced (timeouts, debug logging)

---

## State Inventory Template

For each screen, define:

```markdown
### [Screen Name]

**Empty State**
- Visual: [Icon + message]
- Action: [CTA if applicable]

**Loading State**
- Indicator: [Skeleton / spinner / progress]
- Interruptible: [Yes/No]

**Success State**
- Content: [What displays]
- Actions: [Available CTAs]

**Error State**
- Message: [User-friendly text]
- Recovery: [What user can do]

**Edge States**
- Partial data: [How handled]
- Stale data: [Indicator if any]
```

---

## Accessibility Requirements

- [ ] Keyboard navigation works (Tab, Enter, Escape)
- [ ] Focus states visible (2px gold outline)
- [ ] Color contrast WCAG AA (4.5:1 text, 3:1 UI)
- [ ] Touch targets 44x44px minimum
- [ ] Motion can be reduced (prefers-reduced-motion)
- [ ] Screen reader announces state changes

---

## Dark Mode

Uses shadcn's built-in theme provider with system preference detection.

### Theme Provider Setup

```tsx
// components/theme-provider.tsx
import { ThemeProvider } from "@/components/theme-provider"

<ThemeProvider defaultTheme="system" storageKey="dailyos-theme">
  <App />
</ThemeProvider>
```

### Mode Toggle

```tsx
// In header
<DropdownMenu>
  <DropdownMenuTrigger asChild>
    <Button variant="ghost" size="icon">
      <Sun className="h-4 w-4 scale-100 rotate-0 dark:scale-0" />
      <Moon className="absolute h-4 w-4 scale-0 dark:scale-100" />
    </Button>
  </DropdownMenuTrigger>
  <DropdownMenuContent>
    <DropdownMenuItem onClick={() => setTheme("light")}>Light</DropdownMenuItem>
    <DropdownMenuItem onClick={() => setTheme("dark")}>Dark</DropdownMenuItem>
    <DropdownMenuItem onClick={() => setTheme("system")}>System</DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>
```

### Color Mappings

| Token | Light | Dark |
|-------|-------|------|
| `background` | `cream` (#f5f2ef) | `charcoal` (#1a1f24) |
| `card` | `white` (#ffffff) | `charcoal-light` (#2d343c) |
| `foreground` | `charcoal` (#1a1f24) | `cream` (#f5f2ef) |
| `muted` | `cream-dark` (#ebe5e0) | `#3d444d` |
| `border` | `rgba(26,31,36,0.1)` | `rgba(245,242,239,0.1)` |
| `primary` | `gold` (#c9a227) | `gold` (#c9a227) |

Gold accent remains consistent across themes.

### CSS Variables

```css
:root {
  --background: 30 23% 95%;  /* cream */
  --foreground: 210 14% 12%; /* charcoal */
  --card: 0 0% 100%;
  --primary: 45 71% 47%;     /* gold */
  /* ... */
}

.dark {
  --background: 210 14% 12%; /* charcoal */
  --foreground: 30 23% 95%;  /* cream */
  --card: 213 14% 20%;       /* charcoal-light */
  --primary: 45 71% 47%;     /* gold - unchanged */
}
```

---

## Native App Patterns (Tauri)

### Window Behavior
- Remember position and size
- Minimize to tray (don't quit)
- Focus on tray click

### Keyboard Shortcuts
- `Cmd+,` → Preferences
- `Cmd+W` → Hide window
- `Cmd+Q` → Quit app
- `Cmd+R` → Refresh dashboard

### Menu Bar
- Standard macOS app menu
- DailyOS menu with About, Preferences, Quit

---

## Migration Notes

### From Existing CSS

The archived `design-system.css` and `components.css` define the visual language. Key elements preserved:

| Element | How Preserved |
|---------|---------------|
| Color palette (cream/charcoal/gold) | Tailwind config + CSS variables |
| DM Sans + JetBrains Mono | Tailwind fontFamily config |
| Staggered animations | Tailwind animate utilities |
| Card hover lift | `hover:-translate-y-0.5 hover:shadow-lg` |
| Timeline pulse | Custom `animate-pulse` on dots |
| Gold accent | `--primary` CSS variable |

### shadcn/ui Component Mapping

| Archived Component | shadcn Component | Notes |
|--------------------|------------------|-------|
| `.sidebar` | `Sidebar` | Use collapsible="icon" |
| `.card` | `Card` | Add hover animation class |
| `.btn` | `Button` | Define gold variant |
| `.tag` | `Badge` | Map semantic colors |
| `.alert` | `Alert` | Map status variants |
| `.callout-box` | `Alert` + custom | Gold left border style |
| `.terminal` | Custom | Keep existing styles |
| `.timeline` | Custom | Build on Card primitive |
| `.meeting-row` | Custom | Within Card |
| `.search-modal` | `CommandDialog` | Built-in Cmd+K support |

### Tailwind Config

```js
// tailwind.config.js
module.exports = {
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        cream: { DEFAULT: '#f5f2ef', dark: '#ebe5e0' },
        charcoal: { DEFAULT: '#1a1f24', light: '#2d343c' },
        gold: { DEFAULT: '#c9a227', muted: '#d4b85c' },
        peach: '#e8967a',
        sage: '#7fb685',
      },
      fontFamily: {
        sans: ['DM Sans', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      animation: {
        'fade-in-up': 'fadeInUp 0.5s ease-out',
        'pulse-gold': 'pulseGold 2s ease-in-out infinite',
      },
      keyframes: {
        fadeInUp: {
          '0%': { opacity: '0', transform: 'translateY(20px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        pulseGold: {
          '0%, 100%': { boxShadow: '0 0 0 2px #c9a227' },
          '50%': { boxShadow: '0 0 0 6px rgba(201, 162, 39, 0.3)' },
        },
      },
    },
  },
}
```

### CSS Variables for shadcn

```css
/* globals.css */
@layer base {
  :root {
    --background: 30 23% 95%;      /* cream */
    --foreground: 210 14% 12%;     /* charcoal */
    --card: 0 0% 100%;             /* white */
    --card-foreground: 210 14% 12%;
    --popover: 0 0% 100%;
    --popover-foreground: 210 14% 12%;
    --primary: 45 71% 47%;         /* gold */
    --primary-foreground: 210 14% 12%;
    --secondary: 30 18% 91%;       /* cream-dark */
    --secondary-foreground: 210 14% 12%;
    --muted: 30 18% 91%;
    --muted-foreground: 215 14% 45%;
    --accent: 30 18% 91%;
    --accent-foreground: 210 14% 12%;
    --destructive: 0 72% 51%;
    --destructive-foreground: 0 0% 100%;
    --border: 210 14% 12% / 0.1;
    --input: 210 14% 12% / 0.1;
    --ring: 45 71% 47%;            /* gold */
    --radius: 0.75rem;
  }

  .dark {
    --background: 210 14% 12%;     /* charcoal */
    --foreground: 30 23% 95%;      /* cream */
    --card: 213 14% 20%;           /* charcoal-light */
    --card-foreground: 30 23% 95%;
    --popover: 213 14% 20%;
    --popover-foreground: 30 23% 95%;
    --primary: 45 71% 47%;         /* gold - same */
    --primary-foreground: 210 14% 12%;
    --secondary: 213 14% 25%;
    --secondary-foreground: 30 23% 95%;
    --muted: 213 14% 25%;
    --muted-foreground: 215 14% 65%;
    --accent: 213 14% 25%;
    --accent-foreground: 30 23% 95%;
    --border: 30 23% 95% / 0.1;
    --input: 30 23% 95% / 0.1;
  }
}
```

---

## Implementation Priority

### Phase 1 (MVP)
1. Tailwind + shadcn setup with DailyOS theme
2. Sidebar (collapsible icon mode)
3. Dashboard layout with Card components
4. Command palette (Cmd+K search)
5. Theme toggle (light/dark/system)
6. Basic skeleton loading states

### Phase 2
1. Timeline component for schedule
2. Custom Badge variants
3. Alert/Callout components
4. Empty states with animations

### Phase 3
1. Master-detail panel layout
2. Advanced animations (stagger)
3. Keyboard navigation polish

---

*Document Version: 1.1*
*Last Updated: 2026-02-04*
