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

### Navigation Architecture (DEC7-DEC13)

**Principle:** The Dashboard is the product. Everything else is a drill-down or supporting view.

**Sidebar structure is profile-aware.** CS adds Accounts; GA omits it. No other structural differences.

#### CS Profile Sidebar

```
┌─────────────────────┐
│ ⚡ DailyOS           │
│ Customer Success     │  ← Profile indicator (clickable Phase 2)
├─────────────────────┤
│ Today               │
│   ◻ Dashboard       │  ← Primary. 80% of time.
│                     │
│ Workspace           │
│   ☑ Actions         │  ← Full action list, filterable by account
│   📥 Inbox          │  ← Incoming items to triage
│   🏢 Accounts       │  ← Portfolio health dashboard (CS only)
├─────────────────────┤
│   ⚙ Settings        │
└─────────────────────┘
```

#### GA Profile Sidebar

```
┌─────────────────────┐
│ ⚡ DailyOS           │
├─────────────────────┤
│ Today               │
│   ◻ Dashboard       │
│                     │
│ Workspace           │
│   ☑ Actions         │  ← Filterable by project
│   📥 Inbox          │
│   📁 Projects       │  ← GA equivalent of Accounts
├─────────────────────┤
│   ⚙ Settings        │
└─────────────────────┘
```

#### Profile Entity Pattern

The third sidebar item under Workspace is profile-dependent but structurally identical:

| Profile | Nav Item | Entity | Source Directory | Key Metrics |
|---------|----------|--------|-----------------|-------------|
| CS | Accounts | Account | `2-areas/accounts/` | ARR, Ring, Health, Renewal |
| GA | Projects | Project | `1-projects/` | Status, Deadline, Progress |

Both render as a portfolio page with entity cards. Same component, different data shape.

#### What Was Removed and Why

| Removed | Reason | Where It Lives Instead |
|---------|--------|----------------------|
| Focus | Dashboard section, not a page | Priority list on dashboard overview |
| Week | Post-MVP (Phase 2) | Add to sidebar when built |
| Emails | Already on dashboard | Email section on dashboard; full view post-MVP |

#### Navigation Flows

```
Dashboard ──click meeting card──→ Meeting Detail ──back──→ Dashboard
Dashboard ──click "see all"────→ Actions Page ──back──→ Dashboard
Dashboard ──click entity name──→ Account/Project Detail ──back──→ Dashboard
Sidebar ───click Inbox─────────→ Inbox Page
Sidebar ───click Accounts/Projects──→ Portfolio List
```

#### Drill-Down Header

When navigating to a detail page, the header shows a back button and contextual title:

```
┌──────────────────────────────────────────────────┐
│ [←] Meeting Prep: Acme Sync          [⌘K] [◐]   │
├──────────────────────────────────────────────────┤
│                                                  │
│              [Content]                           │
│                                                  │
└──────────────────────────────────────────────────┘
```

#### Profile-Aware Content Differences

The dashboard shell is identical across profiles. Content and metadata within components change:

| Component | CS Profile | GA Profile |
|-----------|-----------|------------|
| Meeting cards | Gold border for customer, show account + ARR/ring/health | No account concept, external meetings show attendee names |
| Prep depth | Full for customer (talking points, risks, wins, stakeholders) | Light for external (attendee context, last meeting) |
| Actions panel | Grouped by account, show account health color | Flat list, no account grouping |
| Email section | Linked to accounts ("Acme Corp" next to sender) | Just sender name |
| Stats row | "3 customer meetings" | "2 external meetings" |
| Meeting detail | Quick Context (Ring, ARR, Health), stakeholder influence, strategic programs | Attendee names only, no account metadata |

#### Profile Switcher (Phase 2)

Location: Sidebar header, below app name. Shows current profile as text label. Clickable to switch.

**Non-destructive.** Switching profiles changes:
- Meeting classification rules
- Sidebar nav items (Accounts appears/disappears)
- Card metadata rendering
- Prepare phase data collection

Switching does NOT:
- Delete or move files
- Restructure PARA directories
- Lose any data

### Content Panels (Not Pages)

Native apps favor panels over full page navigation. Consider for post-MVP:

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

**Purpose:** Morning briefing consumption. This is the product.

**States:**
- **Loading:** Skeleton cards
- **Success:** Overview card, meeting timeline, actions panel, email section
- **Empty:** "Briefing not yet generated" (checks for `_today/data/manifest.json`)
- **Error:** "Failed to load" with retry
- **Stale:** Show last-generated timestamp if data is >24h old

**Key Components:**
- Greeting + summary + focus area
- Stats row (meetings count, actions due, emails flagged)
- Meeting timeline with type-specific indicators and inline prep summaries
- Action items panel (today + overdue only, "see all" links to Actions page)
- Email section (high priority, linked to accounts for CS)

**Profile differences:** See Navigation Architecture section.

### 2. Meeting Detail (Drill-Down)

**Purpose:** Full meeting prep before a call

**Access:** Click meeting card on Dashboard. Back button returns to Dashboard.

**States:**
- **Loading:** Skeleton
- **Success:** Full prep rendered from `_today/data/preps/{meeting-id}.json`
- **Empty:** "No prep available for this meeting"

**Key Components (CS):**
- Quick Context badges (Ring, ARR, Health, Renewal)
- Attendees with roles and influence
- Since Last Meeting
- Strategic Programs
- Risks to Monitor
- Talking Points
- Open Items with due dates
- Questions
- Key Principles
- Source References (linkable)

**Key Components (GA):**
- Attendees (names only)
- Last meeting notes summary
- Open items
- Talking points

### 3. Actions Page

**Purpose:** Full action list with filtering and status management

**States:**
- **Loading:** Skeleton list
- **Success:** Filterable action list from SQLite
- **Empty:** "No actions tracked yet"

**Key Components:**
- Filter bar: status (pending/completed/waiting), priority (P1/P2/P3)
- Account filter (CS only)
- Action cards with complete/waiting toggles
- Overdue indicator
- Source attribution ("from: Acme Sync, Feb 3")

### 4. Inbox Page

**Purpose:** View and triage incoming items from `_inbox/` directory

**States:**
- **Loading:** Skeleton
- **Success:** File list with previews
- **Empty:** "Inbox is clear" (positive, not "nothing here")

**Key Components:**
- File list from `_inbox/`
- Preview panel (or inline preview)
- Processing status if file watcher is active (Phase 2A)

### 5. Portfolio Page (Accounts or Projects)

**Purpose:** Entity health dashboard. Accounts for CS, Projects for GA. Same layout, different data.

**States:**
- **Loading:** Skeleton cards
- **Success:** Entity cards with health metrics
- **Empty:** "No [accounts/projects] configured" with setup guidance

**CS (Accounts) Components:**
- Account cards: name, ring, ARR, health indicator, renewal date
- Sort by: health (red first), ARR, alphabetical
- Click to Account Detail (drill-down)
- At-a-glance: total ARR, accounts at risk, upcoming renewals

**GA (Projects) Components:**
- Project cards: name, status, deadline, progress
- Sort by: status (at risk first), deadline, alphabetical
- Click to Project Detail (drill-down)
- At-a-glance: active projects, at risk, upcoming deadlines

### 6. System Tray Menu

**Purpose:** Quick access, status at a glance

**Items:**
- DailyOS label + status indicator
- "Open Dashboard" (primary action)
- Last run timestamp
- "Run Now" submenu (Today, Wrap, Inbox)
- Separator
- "Preferences..."
- "Quit DailyOS"

### 7. Notifications

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

### Phase 1 (MVP) ✅ Mostly Complete
1. ✅ Tailwind + shadcn setup with DailyOS theme
2. ✅ Sidebar (collapsible icon mode, defaults collapsed)
3. ✅ Dashboard layout with Card components
4. ✅ Command palette (Cmd+K search)
5. ✅ Theme toggle (light/dark/system)
6. ✅ Basic skeleton loading states
7. ✅ Meeting detail drill-down page
8. ✅ System tray with status indicator

### Phase 1.5 (Nav Refactor — Current)
1. Simplify sidebar to Dashboard / Actions / Inbox / [Entity] / Settings
2. Remove Focus, Week, Emails from sidebar
3. Add profile-aware nav (Accounts for CS, Projects for GA)
4. Actions page backed by SQLite (interactive status updates)
5. Inbox page (list files from `_inbox/`)

### Phase 2
1. Profile switcher in sidebar header
2. Portfolio page component (renders Accounts or Projects based on profile)
3. Entity detail drill-down (Account Detail / Project Detail)
4. Master-detail panel layout
5. Advanced animations (stagger)
6. Keyboard navigation polish

### Phase 3
1. Week view page (re-add to sidebar)
2. Full email page (if needed)
3. Focus mode (dashboard toggle, not separate page)

---

*Document Version: 1.2*
*Last Updated: 2026-02-05 — Nav architecture, profile-aware screens, screen inventory expansion*
