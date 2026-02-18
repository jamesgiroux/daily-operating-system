# Sprint 25 — UI Surface Inventory

Reference: [ADR-0073](decisions/0073-editorial-design-language.md) (Editorial Design Language)

Complete inventory of every UI surface in the app, website, and onboarding flow. Organized by area, with route, file path, and ADR-0073 gap for each.

---

## App Pages (routed)

| # | Surface | Route | File | ADR-0073 Gap |
|---|---------|-------|------|--------------|
| 1 | **Dashboard (daily briefing)** | `/` | `pages/Dashboard.tsx` → `components/dashboard/Dashboard.tsx` | Needs: editorial headline, breathing room, card restraint, narrative voice |
| 2 | **Focus** | `/focus` | `pages/FocusPage.tsx` | Needs: editorial headline, focus callout, breathing room, priority cards |
| 3 | **Week** | `/week` | `pages/WeekPage.tsx` | Needs: full redesign — narrative headline, week shape, priority cards, readiness table |
| 4 | **Accounts** | `/accounts` | `pages/AccountsPage.tsx` | Needs: editorial list rows, generous spacing, typography hierarchy |
| 5 | **Account Detail** | `/accounts/:accountId` | `pages/AccountDetailPage.tsx` | Needs: entity name at scale, assessment prose, timeline, sidebar metadata |
| 6 | **Projects** | `/projects` | `pages/ProjectsPage.tsx` | Needs: same treatment as Accounts list |
| 7 | **Project Detail** | `/projects/:projectId` | `pages/ProjectDetailPage.tsx` | Needs: same treatment as Account Detail |
| 8 | **People** | `/people` | `pages/PeoplePage.tsx` | Needs: same treatment as Accounts list |
| 9 | **Person Detail** | `/people/:personId` | `pages/PersonDetailPage.tsx` | Needs: same treatment as Account Detail |
| 10 | **Actions** | `/actions` | `pages/ActionsPage.tsx` | Needs: text rows, generous spacing, priority as subtle indicator |
| 11 | **Action Detail** | `/actions/:actionId` | `pages/ActionDetailPage.tsx` | Needs: editorial layout, metadata sidebar |
| 12 | **Emails** | `/emails` | `pages/EmailsPage.tsx` | Needs: text rows, breathing room |
| 13 | **Meeting Detail** | `/meeting/:meetingId` | `pages/MeetingDetailPage.tsx` | Needs: editorial layout — entity name, prep as prose, stakeholder rows |
| 14 | **Meeting History Detail** | `/meeting/history/:meetingId` | `pages/MeetingHistoryDetailPage.tsx` | Needs: editorial layout with outcomes sections |
| 15 | **Inbox** | `/inbox` | `pages/InboxPage.tsx` | Needs: breathing room, editorial empty state |
| 16 | **History** | `/history` | `pages/HistoryPage.tsx` | Needs: editorial list treatment |
| 17 | **Settings** | `/settings` | `pages/SettingsPage.tsx` | Needs: breathing room, reduced card density, editorial section titles |

---

## Settings Tabs (sub-routes of `/settings`)

| # | Surface | Route | File | ADR-0073 Gap |
|---|---------|-------|------|--------------|
| 18 | **Settings → Profile** | `/settings?tab=profile` | `pages/SettingsPage.tsx` (inline) | Needs: form fields with more space, section headers not card titles |
| 19 | **Settings → Integrations** | `/settings?tab=integrations` | `pages/SettingsPage.tsx` (inline) | Needs: minimal — single card |
| 20 | **Settings → Workflows** | `/settings?tab=workflows` | `pages/SettingsPage.tsx` (inline) | Needs: spacing, editorial section titles |
| 21 | **Settings → Intelligence** | `/settings?tab=intelligence` | `pages/SettingsPage.tsx` (inline) | Needs: toggle list cleanup, spacing |
| 22 | **Settings → Intelligence Hygiene** | `/settings?tab=hygiene` | `pages/SettingsPage.tsx` (inline) | Needs: signal cards for gaps, cleaner layout |
| 23 | **Settings → Diagnostics** | `/settings?tab=diagnostics` | `pages/SettingsPage.tsx` (inline) | Low priority — dev-only surface |

---

## Dashboard Components (embedded in `/`)

| # | Surface | File | ADR-0073 Gap |
|---|---------|------|--------------|
| 24 | **Header** | `components/dashboard/Header.tsx` | Ensure cream background, no competing elements |
| 25 | **MeetingTimeline** | `components/dashboard/MeetingTimeline.tsx` | Needs: spacing between cards (16-20px gap), time column with divider |
| 26 | **MeetingCard** | `components/dashboard/MeetingCard.tsx` | Needs: accent bar instead of badge, Newsreader title, inline prep, generous padding |
| 27 | **IntelligenceCard** | `components/dashboard/IntelligenceCard.tsx` | Needs: lighter treatment, signal pills not colored blocks |
| 28 | **ActionList + ActionItem** | `components/dashboard/ActionList.tsx`, `ActionItem.tsx` | Needs: text rows not mini-cards, overdue border accent |
| 29 | **EmailList** | `components/dashboard/EmailList.tsx` | Needs: text rows, editorial restraint |
| 30 | **MeetingOutcomes** | `components/dashboard/MeetingOutcomes.tsx` | Needs: signal cards (win/risk), action rows |
| 31 | **RunNowButton** | `components/dashboard/RunNowButton.tsx` | Needs: warm styling, pill shape |
| 32 | **StatusIndicator** | `components/dashboard/StatusIndicator.tsx` | Needs: subtle treatment |
| 33 | **DashboardEmpty** | `components/dashboard/DashboardEmpty.tsx` | Needs: editorial empty state with personality |
| 34 | **DashboardError** | `components/dashboard/DashboardError.tsx` | Needs: warm error treatment |
| 35 | **DashboardSkeleton** | `components/dashboard/DashboardSkeleton.tsx` | Needs: match new layout structure |

---

## App Shell & Layout

| # | Surface | File | ADR-0073 Gap |
|---|---------|------|--------------|
| 36 | **AppSidebar** | `components/layout/AppSidebar.tsx` | Needs: charcoal background, gold active state, muted text, refined spacing |
| 37 | **CommandMenu** | `components/layout/CommandMenu.tsx` | Needs: cream/charcoal theming, warm feel |
| 38 | **PostMeetingPrompt** | `components/PostMeetingPrompt.tsx` | Needs: warm styling, pill inputs, soft radius (transient — low priority) |
| 39 | **PageState** | `components/PageState.tsx` | Needs: editorial empty states with personality |
| 40 | **ProfileSelector** | `components/ProfileSelector.tsx` | Needs: warm styling consistent with sidebar |
| 41 | **DevToolsPanel** | `components/devtools/DevToolsPanel.tsx` | Low priority — dev surface |

---

## Onboarding Flow (modal overlay)

| # | Surface | File | ADR-0073 Gap |
|---|---------|------|--------------|
| 42 | **OnboardingFlow** (orchestrator) | `components/onboarding/OnboardingFlow.tsx` | Needs: progress dots, cream background, editorial feel |
| 43 | **Welcome** | `chapters/Welcome.tsx` | Needs: editorial headline, warm typography, personality |
| 44 | **Entity Mode** | `chapters/EntityMode.tsx` | Needs: mode selection cards with breathing room |
| 45 | **Workspace** | `chapters/Workspace.tsx` | Needs: directory picker with warm styling |
| 46 | **Google Connect** | `chapters/GoogleConnect.tsx` | Needs: OAuth flow card, minimal |
| 47 | **Claude Code** | `chapters/ClaudeCode.tsx` | Needs: CLI install card, minimal |
| 48 | **About You** | `chapters/AboutYou.tsx` | Needs: form fields with generous spacing |
| 49 | **Internal Team Setup** | `chapters/InternalTeamSetup.tsx` | Needs: bulk create form, warm styling |
| 50 | **Populate Workspace** | `chapters/PopulateWorkspace.tsx` | Needs: entity list with breathing room |
| 51 | **Inbox Training** | `chapters/InboxTraining.tsx` | Needs: classification demo with editorial feel |
| 52 | **Dashboard Tour** | `chapters/DashboardTour.tsx` | Needs: interactive walkthrough, editorial overlay |
| 53 | **Meeting Deep Dive** | `chapters/MeetingDeepDive.tsx` | Needs: prep explainer with breathing room |
| 54 | **Prime Briefing** | `chapters/PrimeBriefing.tsx` | Needs: first briefing trigger, warm CTA |
| 55 | **Ready** | `chapters/Ready.tsx` | Needs: completion screen with personality |

---

## Shared UI Components (cross-cutting)

Update these first — changes cascade across all surfaces.

| # | Component | File | ADR-0073 Change |
|---|-----------|------|-----------------|
| 56 | **Card** | `ui/card.tsx` | Soften: 16px radius, subtle shadow, reserve for featured content only |
| 57 | **Badge** | `ui/badge.tsx` | Replace with pill pattern (rounded-full, dot + text) |
| 58 | **Button** | `ui/button.tsx` | Soften: pill radius on small buttons, warm hover states |
| 59 | **Input** | `ui/input.tsx` | Soften: 12px radius, warm border color |
| 60 | **Label** | `ui/label.tsx` | Typography: DM Sans, proper sizing |
| 61 | **SearchInput** | `ui/search-input.tsx` | Gentle restyle |
| 62 | **TabFilter** | `ui/tab-filter.tsx` | Pill-style tabs, warm active state |
| 63 | **StatusBadge** | `ui/status-badge.tsx` | Migrate to pill pattern |
| 64 | **ListRow** | `ui/list-row.tsx` | Increase padding, typography refinement |
| 65 | **EntityPicker** | `ui/entity-picker.tsx` | Warm combobox styling |
| 66 | **PriorityPicker** | `ui/priority-picker.tsx` | Warm combobox styling |
| 67 | **InlineCreateForm** | `ui/inline-create-form.tsx` | Soften, breathing room |
| 68 | **BulkCreateForm** | `ui/bulk-create-form.tsx` | Soften, breathing room |
| 69 | **Separator** | `ui/separator.tsx` | Ensure subtle (6% opacity) |
| 70 | **Skeleton** | `ui/skeleton.tsx` | Match cream/warm palette |
| 71 | **Dialog** | `ui/dialog.tsx` | Warm styling, 16px radius |
| 72 | **AlertDialog** | `ui/alert-dialog.tsx` | Warm styling |
| 73 | **DropdownMenu** | `ui/dropdown-menu.tsx` | Warm styling, cream background |
| 74 | **Popover** | `ui/popover.tsx` | Warm styling |
| 75 | **Select** | `ui/select.tsx` | Warm styling, 12px radius |
| 76 | **Sheet** | `ui/sheet.tsx` | Warm styling |
| 77 | **Tooltip** | `ui/tooltip.tsx` | Warm styling, cream/charcoal |
| 78 | **ScrollArea** | `ui/scroll-area.tsx` | Subtle scrollbar styling |
| 79 | **Collapsible** | `ui/collapsible.tsx` | Minimal — animation only |
| 80 | **Sidebar** | `ui/sidebar.tsx` | Charcoal theme, gold accents |
| 81 | **Sonner (toasts)** | `ui/sonner.tsx` | Warm toast styling |
| 82 | **Calendar** | `ui/calendar.tsx` | Warm date picker |
| 83 | **CopyButton** | `ui/copy-button.tsx` | Minimal — icon only |
| 84 | **Command** | `ui/command.tsx` | Warm palette (used by CommandMenu) |

---

## Public Website (daily-os.com — GitHub Pages)

| # | Surface | Route | File | ADR-0073 Gap |
|---|---------|-------|------|--------------|
| 85 | **Homepage** | `daily-os.com/` | `docs/index.html` | Needs: editorial typography, Newsreader headlines, warm palette, breathing room |
| 86 | **Tour** | `daily-os.com/tour` | `docs/tour.html` | Needs: screenshot refresh, editorial layout, feature sections with breathing room |
| 87 | **Philosophy** | `daily-os.com/philosophy` | `docs/philosophy.html` | Needs: editorial prose styling, Newsreader for principle titles |
| 88 | **Setup Guide** | `daily-os.com/setup` | `docs/setup.html` | Needs: step-by-step editorial layout, warm code blocks |
| 89 | **Site Stylesheet** | — | `docs/site.css` | Needs: Newsreader + DM Sans font stack, cream/charcoal palette, spacing tokens |

---

## Counts

| Area | Surfaces | Sprint 25 Priority |
|------|----------|-------------------|
| App pages (routed) | 17 | Must ship |
| Settings tabs | 6 | Should ship |
| Dashboard components | 12 | Must ship |
| App shell & layout | 6 | Must ship |
| Onboarding flow | 14 | Defer (self-contained) |
| Shared UI components | 29 | Must ship (Phase 1 — cascades everywhere) |
| Public website | 5 | Should ship (brand consistency) |
| **Total** | **89** | |

---

## Sprint 25 Sequencing

**Phase 1: Foundation (shared components + fonts)**
Install Newsreader font. Update CSS custom properties (cream, charcoal, gold, sage, peach tokens). Update Card, Badge→Pill, Button, Input, Separator, Skeleton, ListRow, TabFilter, StatusBadge→Pill, Dialog, Popover, Select, Tooltip. This cascades visual changes across all surfaces immediately.

**Phase 2: App shell**
AppSidebar (charcoal treatment), Header (cream), CommandMenu (warm palette), PageState (editorial empty states).

**Phase 3: Daily-use pages**
Dashboard layout refinement, FocusPage redesign, WeekPage redesign. These are the highest-impact surfaces.

**Phase 4: Entity pages**
Entity list pages (Accounts, Projects, People — shared pattern). Entity detail pages (Account, Project, Person — shared pattern). Redesign one of each, replicate to siblings.

**Phase 5: Secondary pages**
Meeting detail pages, Actions, Emails, Inbox, History, Settings tabs.

**Phase 6: Website**
Update `docs/site.css` with new font stack + tokens. Restyle all 4 HTML pages for brand consistency.

**Phase 7 (optional — Sprint 28): Onboarding**
Self-contained pass across all 14 onboarding screens. Can defer if Sprint 25 is full.
