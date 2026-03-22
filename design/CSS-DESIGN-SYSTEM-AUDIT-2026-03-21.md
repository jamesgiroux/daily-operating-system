# CSS Design System Audit

Date: 2026-03-21

Scope: `src/pages`, `src/components`, `src/styles`

## Executive Summary

The app has a real design-system base, but it is no longer acting as the primary source of truth for page composition. The shell, tokens, and editorial vocabulary exist, yet many pages and feature areas are restyling the same patterns locally.

This is no longer just "normal variance." It is design-system drift:

- The same page-header pattern exists under different class names and sometimes as inline styles.
- Entity hero styling has forked into three nearly identical CSS modules.
- Shared editorial elements such as section headings are visually centralized but not implemented as reusable CSS primitives.
- A large amount of styling has escaped back into inline JSX, especially in settings and report surfaces.
- There is already token drift, including at least one invalid token reference.

## What I Audited

- 65 CSS files under `src/`
- 2,440 CSS class selector matches across those files
- 135 source files with inline style blocks
- 1,516 inline style blocks total

Representative shell and editorial sources reviewed:

- [src/styles/design-tokens.css](/Users/jamesgiroux/Documents/daily-operating-system/src/styles/design-tokens.css)
- [src/styles/editorial-briefing.module.css](/Users/jamesgiroux/Documents/daily-operating-system/src/styles/editorial-briefing.module.css)
- [src/styles/entity-detail.module.css](/Users/jamesgiroux/Documents/daily-operating-system/src/styles/entity-detail.module.css)
- [src/components/layout/MagazinePageLayout.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/components/layout/MagazinePageLayout.tsx)
- [src/components/editorial/ChapterHeading.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/components/editorial/ChapterHeading.tsx)

## Findings

### 1. Page headers are not a shared system

Severity: High

The app uses multiple implementations of the same editorial page-header pattern:

- Inbox defines `.heroSection`, `.heroTitle`, `.heroRule`, and `.heroRow` in [src/pages/InboxPage.module.css:62](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/InboxPage.module.css#L62)
- Settings defines `.hero`, `.heroTitle`, and `.heroRule` in [src/pages/SettingsPage.module.css:54](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/SettingsPage.module.css#L54)
- Me defines `.hero`, `.heroTitle`, `.heroSubtitle`, and `.heroRule` in [src/pages/MePage.module.css:22](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/MePage.module.css#L22)
- Actions uses a renamed copy: `.headerSection`, `.pageTitle`, `.itemCount`, `.sectionRule` in [src/pages/ActionsPage.module.css:75](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/ActionsPage.module.css#L75)
- History uses another renamed copy: `.hero`, `.title`, `.entryCount`, `.heroRule` in [src/pages/HistoryPage.module.css:7](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/HistoryPage.module.css#L7)
- People bypasses page-module styling entirely for its top-level header in [src/pages/PeoplePage.tsx:290](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/PeoplePage.tsx#L290)

Visual intent is consistent, implementation is not. This is exactly the kind of drift that causes AI-generated classes to keep multiplying.

### 2. Entity hero CSS has been copy-forked

Severity: High

The account, person, and project hero modules are structurally the same system with small semantic differences:

- [src/components/account/AccountHero.module.css:3](/Users/jamesgiroux/Documents/daily-operating-system/src/components/account/AccountHero.module.css#L3)
- [src/components/person/PersonHero.module.css:3](/Users/jamesgiroux/Documents/daily-operating-system/src/components/person/PersonHero.module.css#L3)
- [src/components/project/ProjectHero.module.css:3](/Users/jamesgiroux/Documents/daily-operating-system/src/components/project/ProjectHero.module.css#L3)

Shared duplicated structure includes:

- `.hero`
- `.heroDate`
- `.name`
- `.lede`
- `.badges`
- `.badge`
- `.archivedBanner`
- `.archivedText`
- `.meta`
- `.metaButton`
- `.metaButtonEnriching`

These should be a base hero system with variants, not three separate files that need to evolve in parallel.

### 3. The app has a visual standard for chapter headers, but not a CSS primitive

Severity: High

`ChapterHeading` is widely adopted, which is good. But the implementation is inline-style based instead of class-based:

- [src/components/editorial/ChapterHeading.tsx:19](/Users/jamesgiroux/Documents/daily-operating-system/src/components/editorial/ChapterHeading.tsx#L19)

That means:

- there is no canonical class name for chapter headers
- the heading cannot be composed or extended cleanly in CSS
- typography and spacing decisions are trapped in JSX instead of the design layer

If the requirement is "headers should be the same everywhere and use the same class names," this component currently fails that standard even though it enforces a shared look.

### 4. Inline styles have become a parallel styling system

Severity: High

The CSS system is being bypassed at scale:

- 135 files contain inline style blocks
- 1,516 inline style blocks total

Top offenders include:

- [src/features/settings-ui/SystemStatus.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/features/settings-ui/SystemStatus.tsx)
- [src/features/settings-ui/DiagnosticsSection.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/features/settings-ui/DiagnosticsSection.tsx)
- [src/pages/PeoplePage.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/PeoplePage.tsx)
- [src/pages/BookOfBusinessPage.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/BookOfBusinessPage.tsx)
- [src/components/reports/EbrQbrReport.tsx](/Users/jamesgiroux/Documents/daily-operating-system/src/components/reports/EbrQbrReport.tsx)

Example:

- `SystemStatus` is effectively using a JS style object design system rather than the CSS system in [src/features/settings-ui/SystemStatus.tsx:117](/Users/jamesgiroux/Documents/daily-operating-system/src/features/settings-ui/SystemStatus.tsx#L117)

This is one of the biggest reasons consistency will keep decaying. CSS modules cannot govern visual reuse if entire surfaces are styled in JSX.

### 5. Container widths and hero scales are repeated instead of tokenized

Severity: Medium

The same layout constants show up repeatedly:

- `max-width: 900px` in list/detail surfaces such as [src/pages/InboxPage.module.css:8](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/InboxPage.module.css#L8), [src/pages/SettingsPage.module.css:3](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/SettingsPage.module.css#L3), [src/pages/AccountsPage.module.css:8](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/AccountsPage.module.css#L8), and [src/pages/ActionsPage.module.css:16](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/ActionsPage.module.css#L16)
- `padding-top: 80px` for heroes in multiple modules, including [src/pages/InboxPage.module.css:62](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/InboxPage.module.css#L62), [src/pages/SettingsPage.module.css:54](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/SettingsPage.module.css#L54), and the entity hero modules
- headline scales at `36px`, `42px`, `52px`, and `76px` across page and entity surfaces

Some of this variation is legitimate. The problem is that the sizes are expressed as local declarations rather than as named editorial scales.

### 6. Folio action styling is duplicated locally

Severity: Medium

There is repeated "small mono utility button" styling under local names:

- shared entity detail folio actions in [src/styles/entity-detail.module.css:46](/Users/jamesgiroux/Documents/daily-operating-system/src/styles/entity-detail.module.css#L46)
- inbox folio buttons in [src/pages/InboxPage.module.css:16](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/InboxPage.module.css#L16)
- account list folio buttons in [src/pages/AccountsPage.module.css:43](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/AccountsPage.module.css#L43)
- actions folio add button in [src/pages/ActionsPage.module.css:112](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/ActionsPage.module.css#L112)
- the shell also has its own folio action area in [src/components/layout/MagazinePageLayout.tsx:126](/Users/jamesgiroux/Documents/daily-operating-system/src/components/layout/MagazinePageLayout.tsx#L126)

The app needs one folio action primitive with variants, not repeated local button recipes.

### 7. Token drift has already produced a broken reference

Severity: Medium

`WeekPage` references a token that does not exist:

- [src/pages/WeekPage.module.css:99](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/WeekPage.module.css#L99) uses `var(--color-surface-linen)`

That token is not defined in [src/styles/design-tokens.css](/Users/jamesgiroux/Documents/daily-operating-system/src/styles/design-tokens.css).

This is a concrete example of why stricter token governance is needed.

### 8. The shell is centralized, but page content primitives are not

Severity: Medium

The app shell itself is centralized and routed correctly through `MagazinePageLayout`:

- [src/router.tsx:427](/Users/jamesgiroux/Documents/daily-operating-system/src/router.tsx#L427)

Pages also register shell state consistently via `useRegisterMagazineShell`, which is good:

- [src/pages/InboxPage.tsx:559](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/InboxPage.tsx#L559)
- [src/pages/SettingsPage.tsx:172](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/SettingsPage.tsx#L172)
- [src/pages/MePage.tsx:197](/Users/jamesgiroux/Documents/daily-operating-system/src/pages/MePage.tsx#L197)

The drift is therefore not at the shell level. It is in page content primitives: headers, sections, entity heroes, utility buttons, and editorial blocks.

## Root Cause

The design system currently has strong foundations but weak composition boundaries.

What exists:

- tokens
- shell
- a few shared editorial primitives

What is missing:

- canonical page-header component/module
- canonical entity-hero base
- canonical folio action/button primitives
- canonical chapter-heading CSS classes
- enforcement against inline-style expansion

The current setup makes it too easy for new work, especially AI-assisted work, to solve styling locally instead of extending the shared vocabulary.

## Recommended Remediation Order

### Phase 1: Stop the bleeding

1. Extract a shared editorial page-header primitive.
   - It should cover container, title row, count/meta line, subtitle, and rule.
   - Pages like Inbox, Settings, Me, Actions, History, and People should consume it.

2. Convert `ChapterHeading` to CSS-module or shared-class styling.
   - Keep the component, but move typography and spacing out of inline JSX.

3. Fix invalid token references.
   - Start with `--color-surface-linen` on WeekPage.

### Phase 2: Re-centralize duplicated systems

1. Create a shared `EntityHeroBase` styling layer.
   - Account, Person, and Project heroes should compose a common hero module and define only semantic variants.

2. Create shared folio action button variants.
   - Text action
   - outlined action
   - danger action
   - disabled/loading state

3. Introduce named editorial scale tokens.
   - content widths
   - page-header sizes
   - hero sizes
   - rule thickness/spacing

### Phase 3: Reduce style escape hatches

1. Migrate the settings feature away from JS style objects.
2. Migrate People page top-level layout/header styles out of JSX.
3. Audit report surfaces and slide surfaces separately, because they are currently a major inline-style island.

## Guardrails I Recommend Adding

- Add a style audit script or test that flags:
  - unknown CSS custom properties
  - repeated page-header declarations
  - repeated entity-hero declarations
  - excessive inline `style={{ ... }}` usage in app surfaces
- Add a short design-system rule to the contributor guidance:
  - new page headers must use the shared header primitive
  - new entity hero work must extend the shared hero base
  - new editorial headings must use `ChapterHeading`
  - inline styles are allowed only for true one-off dynamic values

## Bottom Line

The design system is not broken, but it is no longer strict enough to prevent local reinvention. The next step should not be cosmetic cleanup file by file. It should be a re-centralization pass that creates 3 to 5 canonical primitives and migrates the highest-drift surfaces onto them.

If we do that, the system becomes easier for both humans and AI to extend without multiplying near-duplicate CSS.
