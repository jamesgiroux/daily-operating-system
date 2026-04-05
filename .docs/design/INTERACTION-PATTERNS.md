# Interaction Patterns

**Last audited:** 2026-03-15

Documented interaction patterns in DailyOS. Use existing patterns before inventing new ones.

---

## 1. Inline Editing

**Components:** `EditableText`, `EditableVitalsStrip`, `StakeholderGallery`, `EngagementSelector`
**Where:** Entity detail heroes, stakeholder gallery, action detail page, vitals strips, report slides
**Model implementation:** `StakeholderGallery` in `src/components/entity/StakeholderGallery.tsx`

### Behavior

- Click text to enter edit mode. Text element becomes an `<input>` or `<textarea>` with matched styling.
- On hover (before click), a subtle background hint signals editability (`EditableText.module.css`).
- Blur or Enter (single-line) commits the change. Escape cancels without saving.
- Tab advances to next editable element; Shift+Tab goes to previous. Uses `data-editable-text` attribute to find siblings in document order.
- Emits a Tauri `editable-text:commit` event on save for the persistence layer.

### Visual Treatment

- Edit indicator: bottom border only (`2px solid var(--color-spice-terracotta)`), no input chrome.
- Background remains transparent. Padding and font match the display element exactly.
- Textarea auto-resizes to content height on input and focus.

### EditableText Props

| Prop | Type | Default | Purpose |
|------|------|---------|---------|
| `value` | string | required | Current text |
| `onChange` | (value: string) => void | required | Commit handler |
| `as` | span, p, h1, h2, div | span | Display element tag |
| `multiline` | boolean | true | Textarea (true) or input (false) |
| `placeholder` | string | - | Shown when value is empty |
| `fieldId` | string | - | Identifier for Tauri event emission |

### EditableVitalsStrip

Extends inline editing to structured vital fields with type-specific editors:

- **Currency/number:** Click to reveal input, commit on blur/Enter.
- **Select:** Click to cycle through options (e.g., health status).
- **Date:** Click to open DatePicker popover.
- **Text:** Click to reveal input, commit on blur/Enter.

File: `src/components/entity/EditableVitalsStrip.tsx`

### Rules

- Edit in place -- no drawers, no modals for field edits.
- Always provide Escape to cancel.
- Tab navigation between editable fields must work.
- Debounced save (500ms) for report-level edits (AccountHealthPage, RiskBriefingPage).
- StakeholderGallery uses `update_intelligence_field` Tauri command for individual field saves and `update_stakeholders` for bulk stakeholder array updates.

---

## 2. Slide-Deck Navigation

**Components:** `AccountHealthPage`, `RiskBriefingPage`, `EbrQbrPage`, `SwotPage`
**Where:** Account report pages
**Model implementation:** `AccountHealthPage` in `src/pages/AccountHealthPage.tsx`

### Behavior

- Report content is divided into full-height slides (sections with `id` attributes).
- CSS scroll-snap (`scroll-snap-type: y proximity`) settles on slide boundaries.
- FloatingNavIsland switches to `chapters` mode, showing slide icons instead of page navigation.
- Clicking a chapter icon smooth-scrolls to the corresponding section via `smoothScrollTo()`.

### Keyboard Navigation

| Key | Action |
|-----|--------|
| Arrow Down / Arrow Right | Scroll to next slide |
| Arrow Up / Arrow Left | Scroll to previous slide |
| Number keys (1-N) | Jump directly to slide N |

Keyboard handlers skip input when focus is on INPUT or TEXTAREA elements.

### Shell Registration

Each report page registers its chapter list via `useRegisterMagazineShell()`:

```typescript
const shellConfig = {
  folioLabel: "Account Review",
  atmosphereColor: "turmeric",
  activePage: "accounts",
  chapters: content ? SLIDES : undefined,
  // ...
};
useRegisterMagazineShell(shellConfig);
```

### Slide Definition

```typescript
const SLIDES = [
  { id: "cover", label: "Cover", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "partnership", label: "The Partnership", icon: <Users size={18} strokeWidth={1.5} /> },
  // ...
];
```

### Rules

- Each slide has a unique `id` and a `scrollMarginTop: 60` on the first slide (clears the FolioBar).
- Subsequent slides use `editorial-reveal` class for scroll-linked fade-in.
- Pages display a `FinisMarker` after the last slide.
- Generating state shows `GeneratingProgress` with phased progress and editorial quotes.
- Report content is inline-editable with debounced auto-save.

---

## 3. Expansion / Disclosure

**Components:** `Collapsible` (Radix), `WatchList`, `StakeholderGallery`
**Where:** Watch list items, stakeholder grids, briefing sections

### Collapsible (Radix Primitive)

File: `src/components/ui/collapsible.tsx`

Thin wrapper around `@radix-ui/react-collapsible`. Provides `Collapsible`, `CollapsibleTrigger`, `CollapsibleContent` with `data-slot` attributes.

### Show More / Truncation

StakeholderGallery limits the visible grid to 6 items. A "Show N more" button expands the full grid:

```
STAKEHOLDER_LIMIT = 6
visibleStakeholders = expandedGrid ? stakeholders : stakeholders.slice(0, STAKEHOLDER_LIMIT)
```

WatchList items with long assessment text use `TruncatedAssessment` (150 char limit) with a "Read more" button to expand inline.

### Rules

- Use Radix Collapsible for structured expand/collapse with trigger and content regions.
- Use "Show N more" pattern for grid/list truncation.
- Use inline "Read more" for long text within cards or rows.
- No accordions -- content sections are always independently expandable.

---

## 4. Entity Linking

**Components:** `EntityPicker`, `MeetingEntityChips`
**Where:** Meeting detail pages, action detail pages, inbox items
**Model implementation:** `MeetingEntityChips` in `src/components/ui/meeting-entity-chips.tsx`

### EntityPicker

File: `src/components/ui/entity-picker.tsx`

Popover-based command palette for searching and selecting accounts or projects.

- Uses Radix Popover + cmdk Command (search, groups, items).
- Groups entities by type: Internal Teams, External Accounts (with parent/child nesting), Partners, Projects.
- Icons: `Building2` for accounts, `FolderKanban` for projects.
- Supports `excludeIds` for multi-select mode (picker stays open after selection).
- Selected entity shows as a removable chip (unless `locked`).
- Fires `onChange(id, name, entityType)` on selection.

### MeetingEntityChips

Inline entity assignment for meetings. Shows linked entities as color-coded removable chips + an EntityPicker for adding more.

- **Optimistic updates:** Chips appear/disappear instantly. Rollback on Tauri command failure.
- **Color coding:** Accounts = turmeric, Projects = olive, People = larkspur.
- Each chip links to the entity detail page.
- `compact` mode for smaller chips in briefing expansion panels.

### StakeholderGallery Linking

- Unlinked stakeholders show a "Create contact" button on hover.
- Suggested person links (I420) show a "Link to [name]?" confirmation button.
- Linked stakeholders show a `LinkIcon` indicator and their card becomes a navigation link.

### Rules

- Use EntityPicker for account/project selection. Do not build custom pickers.
- Chips are always removable (unless locked). Show `X` button on each chip.
- Entity color coding is consistent: turmeric for accounts, olive for projects, larkspur for people.
- Use optimistic local state for chip add/remove to avoid UI flicker.

---

## 5. Selection / Filtering

**Components:** `PriorityPicker`, `EngagementSelector`, entity list filters
**Where:** Action detail page, stakeholder gallery, entity list pages

### PriorityPicker

File: `src/components/ui/priority-picker.tsx`

Horizontal button group for P1/P2/P3 selection. Active state uses priority-specific colors:

| Priority | Active Style |
|----------|-------------|
| P1 | `bg-destructive/15 text-destructive border-destructive/30` |
| P2 | `bg-primary/15 text-primary border-primary/30` |
| P3 | `bg-muted text-muted-foreground border-muted-foreground/30` |

Inactive buttons show `border-transparent text-muted-foreground`.

### EngagementSelector

File: `src/components/entity/EngagementSelector.tsx`

Click-to-cycle engagement level selector for stakeholders (champion, supporter, neutral, detractor, unknown).

### Entity List Filtering

People page uses search query params for relationship tabs (`all`, `external`, `internal`, `unknown`) and hygiene filters (`unnamed`, `duplicates`). Actions page uses `?search=` query parameter.

### Rules

- Use horizontal button groups for small option sets (3-5 items).
- Active state must be visually distinct with color + background.
- Filters that affect the URL should use search params for deep linking.

---

## 6. Feedback

**Components:** `Toaster` (Sonner), `IntelligenceFeedback`, `PostMeetingPrompt`
**Where:** Global (toasts), entity detail pages (intelligence feedback), post-meeting overlay

### Toasts (Sonner)

File: `src/components/ui/sonner.tsx`

Global toast system using Sonner. Positioned `bottom-right`. 5-second duration. Icons for success, info, warning, error, loading states. Styled with design system CSS variables.

Usage: `import { toast } from "sonner"` then `toast.success("Message")`.

### IntelligenceFeedback

File: `src/components/ui/IntelligenceFeedback.tsx`

Inline thumbs-up / thumbs-down feedback for AI-generated intelligence content.

- Hidden by default (`opacity: 0`). Revealed on parent `:hover` or when a vote is active.
- `ThumbsUp` in sage green when positive, `ThumbsDown` in terracotta when negative.
- 22px button targets. Focus-visible outline in turmeric.
- Used on entity detail pages via `useIntelligenceFeedback` hook.

### PostMeetingPrompt

File: `src/components/PostMeetingPrompt.tsx`

Global overlay that appears after a meeting ends. Multi-phase capture flow: prompt -> input -> confirm -> processing -> done. Captures wins, risks, and actions. Supports transcript attachment. Auto-dismisses if not interacted with.

### Rules

- Use Sonner toasts for transient feedback (save confirmations, errors, async completions).
- Use IntelligenceFeedback for AI-generated content that benefits from quality signals.
- PostMeetingPrompt is the only modal capture flow. Other captures are inline.
- Feedback controls must not block content reading -- use hover-reveal or subtle placement.

---

## 7. Hover-Reveal Actions

**Where:** StakeholderGallery, IntelligenceFeedback, entity rows
**Model implementation:** `IntelligenceFeedback.module.css`

### Behavior

Elements start at `opacity: 0` and transition to `opacity: 1` on parent `:hover` or `:focus-within`. This keeps the reading experience clean while making actions discoverable.

### IntelligenceFeedback Pattern

```css
.container {
  opacity: 0;
  transition: opacity var(--transition-normal);
}
.visible,
*:hover > .container,
.container:focus-within {
  opacity: 1;
}
```

### StakeholderGallery Pattern

Remove button and "Create contact" action only appear when the card is hovered (`hoveredCard === i` state). Uses React state rather than CSS for conditional rendering.

### Rules

- Use CSS opacity transitions (not display/visibility) for hover reveals. This preserves layout stability.
- Always provide a non-hover path to reach the action (e.g., `:focus-within`, or always-visible when a vote is active).
- Keep transition duration short: `var(--transition-fast)` for instant feedback, `var(--transition-normal)` for smooth reveals.

---

## 8. Scroll-Linked Reveals

**Class:** `.editorial-reveal`, `.editorial-reveal-slow`
**Where:** All editorial pages (report slides, entity detail sections)
**Hook:** `useRevealObserver` in `src/hooks/useRevealObserver.ts`

### Behavior

Elements with the `.editorial-reveal` class start invisible and translate down:

```css
.editorial-reveal {
  opacity: 0;
  transform: translateY(16px);
  transition: opacity 600ms ease, transform 600ms ease;
}
.editorial-reveal.visible {
  opacity: 1;
  transform: translateY(0);
}
```

An `IntersectionObserver` (threshold 0.08, rootMargin `0px 0px -40px 0px`) adds the `.visible` class when elements scroll into view. Once revealed, elements are unobserved (one-shot animation).

### Variants

| Class | Duration | Purpose |
|-------|----------|---------|
| `.editorial-reveal` | 600ms | Standard content sections |
| `.editorial-reveal-slow` | 800ms | Deep-dive sections (pace change signals deeper content) |

### Reduced Motion

Both variants collapse to instant display for users with `prefers-reduced-motion`:

```css
@media (prefers-reduced-motion: reduce) {
  .editorial-reveal,
  .editorial-reveal-slow {
    opacity: 1;
    transform: none;
    transition: none;
  }
}
```

### Rules

- Wrap each section after the hero/cover in `<div className="editorial-reveal">`.
- Call `useRevealObserver(ready)` in the page component once data is loaded.
- Pass a `revision` value to re-observe when data reloads.
- Never use reveal animations on above-the-fold content (heroes, covers).

---

## 9. Command Menu (Global Search)

**Component:** `CommandMenu`
**File:** `src/components/layout/CommandMenu.tsx`
**Trigger:** `Cmd+K` or FolioBar search button

### Behavior

- Opens a `CommandDialog` (cmdk-based) with search input.
- Searches entities globally via `global_search` Tauri command.
- Results grouped by entity type (Accounts, People, Projects, Meetings, Actions, Emails).
- Selection navigates to the entity detail page.
- Also provides quick actions: navigation to pages, run briefing, refresh data.

### Rules

- CommandMenu is the single global search surface. Do not build page-specific search dialogs.
- Entity type icons are consistent with the rest of the app (Building2, Users, FolderKanban, etc.).
- Always dismiss on selection (`onOpenChange(false)`).

---

## 10. Startup Gate

**File:** `src/routerStartupGate.ts`
**Where:** Root layout in router

### Priority Order

The startup gate evaluates conditions in strict priority order. The first matching condition wins:

| Priority | Gate | Component | Condition |
|----------|------|-----------|-----------|
| 1 | `checking` | Blank screen | `checkingConfig === true` |
| 2 | `encryption-recovery` | `EncryptionRecovery` | `encryptionKeyMissing === true` |
| 3 | `database-recovery` | `DatabaseRecovery` | `dbRecoveryRequired === true` |
| 4 | `lock` | `LockOverlay` | `isLocked === true` |
| 5 | `onboarding` | `OnboardingFlow` | `needsOnboarding === true` |
| 6 | `app` | Normal routing | All conditions false |

### Rules

- Gates are mutually exclusive; only one renders at a time.
- The `app` gate is the normal state. All other gates block the application.
- Recovery gates (encryption, database) take priority over lock and onboarding.

---

## 11. Drag-and-Drop

**Where:** Inbox page (file drop zone), Me page (attachment drop zone)

### Inbox Drop Zone

The Inbox page has a full-area drag-and-drop zone for uploading files. Uses the browser's native Drag and Drop API with Tauri file handling:

- Visual feedback: Drop zone highlights on drag-over
- File browse: Button fallback for clicking to select files
- Google Drive import: Modal dialog for Drive file import (`GoogleDriveImportModal`)

### Me Page Attachment Zone

The Me page has a document drop zone in the Attachments section for attaching files to the user's context.

### Rules

- Drop zones must provide visual feedback on drag-over (highlight border/background)
- Always offer a file browse button as a non-drag alternative
- Show processing status after drop (classification, routing indicators)

---

## 12. Modal / Dialog Patterns

**Components:** `dialog`, `alert-dialog`, `sheet`, `agenda-draft-dialog`

### Standard Dialog (`dialog.tsx`)

Radix-based modal dialog. Used for forms and content that requires focus:
- `GoogleDriveImportModal` -- Drive file import
- `WhatsNewModal` -- Release notes after update
- `ICloudWarningModal` -- iCloud sync warning
- `AccountMergeDialog` -- Merge duplicate accounts

### Alert Dialog (`alert-dialog.tsx`)

Radix-based confirmation dialog for destructive actions:
- Delete person confirmation
- Archive entity confirmation
- Merge entity confirmation

### Sheet / Drawer (`sheet.tsx`)

Side drawer. Used for workflows that involve search + create:
- `LifecycleEventDrawer` -- Add lifecycle event to account
- Team management drawer (adding team members involves search)

### Agenda Draft Dialog (`agenda-draft-dialog.tsx`)

AI-generated meeting agenda draft. Custom dialog with:
- Generate button triggers AI draft
- Preview of generated agenda items
- Accept/dismiss actions
- Used exclusively on Meeting Detail page

### Rules

- Use `dialog` for content modals that don't involve destruction
- Use `alert-dialog` for destructive confirmations (delete, merge, archive)
- Use `sheet` only for complex create/search workflows (not for field editing -- use inline editing instead)
- Never use modals for field editing. Inline editing is the standard (ADR-0084, I343)
- Modals should be dismissable via Escape key (Radix handles this)

---

## 13. Dropdown / Popover Patterns

**Components:** `dropdown-menu`, `popover`, `select`

### Dropdown Menu (`dropdown-menu.tsx`)

Radix-based context menu. Used for action menus:
- FolioBar context actions (Regenerate, Archive)
- Entity row actions (right-click or kebab menu)

### Popover (`popover.tsx`)

Radix-based positioned popup. Used for:
- `EntityPicker` -- search and select entities
- `DatePicker` / `EditableDate` -- calendar popover
- `PriorityPicker` -- priority selection

### Select (`select.tsx`)

Radix-based dropdown select for form fields:
- Status selection
- Lifecycle event type
- Filter options

### Rules

- Use `dropdown-menu` for action lists (verbs: Edit, Delete, Archive)
- Use `popover` for pickers and selection UI (nouns: dates, entities)
- Use `select` for form fields with a fixed set of options
- All three auto-close on selection (Radix default behavior)

---

## 14. Keyboard Shortcuts

### Global Shortcuts

| Shortcut | Action | Handler |
|----------|--------|---------|
| `Cmd+K` | Open Command Menu | `useCommandMenu()` in `CommandMenu.tsx` |

### Report Page Shortcuts

| Shortcut | Action | Handler |
|----------|--------|---------|
| `1-9` | Jump to slide N | `keydown` handler in each report page |
| `Arrow Down` / `Arrow Right` | Next slide | Same handler |
| `Arrow Up` / `Arrow Left` | Previous slide | Same handler |

Keyboard handlers skip processing when focus is on `INPUT` or `TEXTAREA` elements to avoid interfering with text editing.

### Inline Editing Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Commit single-line edit |
| `Escape` | Cancel edit, revert to original value |
| `Tab` | Move to next editable field (via `data-editable-text`) |
| `Shift+Tab` | Move to previous editable field |

### Rules

- Global shortcuts must use `Cmd` (macOS) modifier to avoid conflicts
- Page-level shortcuts must not fire when focus is on input elements
- All keyboard shortcuts must have equivalent mouse/touch paths
