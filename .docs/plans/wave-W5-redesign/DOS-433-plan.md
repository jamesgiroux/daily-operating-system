# DOS-433 - Actions surface uplift (W5 redesign)

**Status:** L0 draft
**Scope:** Bring the Daily Briefing redesign's editorial register to `/actions` and, if L1 confirms the same chrome contract, `/actions/$actionId`.

## Acceptance Criteria

- [ ] `/actions` renders as an editorial surface aligned with the Daily Briefing redesign chrome, spacing, typography, and token palette.
- [ ] `/actions/$actionId` renders as the matching action detail surface when included by L1.
- [ ] `ActionsPage` and `ActionDetailPage` emit complete ds-inspector attributes on their surface roots.
- [ ] Surface roots use `.root`; CSS module selectors are camelCase and token-driven.
- [ ] No inline CSS is introduced; existing inline CSS in touched page files is removed or moved into modules.
- [ ] Existing action filters, search, create flow, suggested accept/reject, complete/reopen, save, account/source edits, due date edits, and Linear push behavior are unchanged.
- [ ] Shared row reuse remains intact; the page does not fork local copies of generic action rows.
- [ ] No source comments mention ticket IDs or temporary plan references.
- [ ] Loading, error, empty, suggested, active, completed, grouped, saving, and saved states remain visibly covered.

## Current Code Facts

- `src/pages/ActionsPage.tsx` already registers magazine shell chrome through `useRegisterMagazineShell`.
- `ActionsPage` already composes `EditorialPageHeader`, `EditorialLoading`, `EditorialError`, `EmptyState`, `ChapterHeading`, `FinisMarker`, `ActionRow`, and `SuggestedActionRow`.
- `src/pages/ActionDetailPage.tsx` already registers folio breadcrumbs and save status, but still uses inline `style` props for dynamic button, due-date, Linear icon, and save/status presentation.
- `ActionsPage.module.css` and `ActionDetailPage.module.css` both need root/class cleanup; surface specs already exist for both pages.

## File Plan

### `src/pages/ActionsPage.tsx`

- Rename the outer page class from `s.pageContainer` to `s.root`.
- Add root ds-inspector attrs: `data-ds-name="ActionsPage"`, `data-ds-tier="surface"`, `data-ds-spec="surfaces/ActionsPage.md"`.
- Keep `useRegisterMagazineShell` behavior and folio actions unchanged.
- Keep the tab arrays, status/priority filters, search state, create form state, grouping, and date formatting behavior unchanged.
- Keep `EditorialLoading` and `EditorialError`; do not introduce local replacements.
- Keep `SharedActionRow` for active/completed rows and `SharedSuggestedActionRow` for suggested rows.
- Keep `ChapterHeading` for meeting/time-band grouping.
- Do not move action mutation handlers or change hook contracts.

### `src/pages/ActionsPage.module.css`

- Rename `.pageContainer` to `.root`.
- Keep selector names camelCase; do not use prefixed module names or kebab-case.
- Remove unused legacy selectors if they are no longer referenced after the page cleanup.
- Normalize section spacing to design tokens and the existing editorial page width tokens.
- Keep create form styling local to this surface and token-driven.
- Preserve responsive behavior for narrow widths.

### `src/pages/ActionDetailPage.tsx`

- Rename the outer page class from `s.container` to `s.root`.
- Add root ds-inspector attrs: `data-ds-name="ActionDetailPage"`, `data-ds-tier="surface"`, `data-ds-spec="surfaces/ActionDetailPage.md"`.
- Keep the existing loading/error branches, but align their root attrs if L1 keeps detail in scope.
- Remove inline `style` props from page JSX.
- Convert dynamic visual states to classes or data attributes: completion/open, toggling/wait, due urgency, save status, Linear icon spacing, and disabled/toggling action buttons.
- Keep `PriorityPicker`, `EntityPicker`, `EditableInline`, `EditableTextarea`, `EditableDate`, and `EditableText`.
- Keep all Tauri command names and payload shapes unchanged.
- Keep toast copy and error handling unchanged.

### `src/pages/ActionDetailPage.module.css`

- Rename `.container` to `.root`.
- Add classes or data-attribute selectors for every dynamic state moved out of JSX.
- Keep all selectors camelCase.
- Replace hard-coded spacing with existing tokens where equivalent tokens exist.
- Keep detail-page max width and reading rhythm aligned to the surface spec.
- Keep priority tint classes, mono badges, account chip, reference rows, Linear row, and action bar local to this module.

## Reuse Rules

- Reuse the existing W1/W3 editorial surface vocabulary where applicable: `EditorialPageHeader`, `ChapterHeading`, `EmptyState`, `EditorialLoading`, `EditorialError`, `FinisMarker`, and the magazine shell registration pattern.
- Reuse `ActionRow` as the generic action-row pattern for active/completed action lists.
- Reuse `SuggestedActionRow` for suggested-action review rows.
- `WatchRow` is briefing-specific because it consumes `WatchRowViewModel`; do not reuse it for `/actions`.
- Do not introduce a new row abstraction unless L1 proves `ActionRow` cannot satisfy visual alignment without behavior drift.

## Cardinal Rules

- No inline CSS in edited page files.
- CSS module root selector must be `.root`.
- CSS module class names must be camelCase.
- Emit `data-ds-name`, `data-ds-tier`, and `data-ds-spec` on surface roots.
- Do not add source-code comments containing `DOS-` references.
- Do not change mutation behavior while doing visual work.
- Do not change backend service calls, hook contracts, or route params.

## Backward Compatibility

- `/actions` must keep the same route, filters, tab defaults, search query initialization, and empty-state branching.
- `/actions/$actionId` must keep the same route param, load command, save command, completion/reopen commands, Linear status/team commands, and Linear push command.
- Suggested action accept/reject must still refresh or update exactly as it does today.
- Existing `ActionRow` and `SuggestedActionRow` props stay compatible with all current consumers.
- Visual alignment must not rename exported components, route IDs, Tauri command names, or type imports.

## Out Of Scope

- Action service backend changes.
- Mutation handler redesign.
- New Tauri commands or payload contracts.
- Rewriting `useActions` or `useSuggestedActions`.
- Broad `ActionRow` or `SuggestedActionRow` redesign unless L1 explicitly expands the scope.
- Reusing or generalizing `WatchRow`.
- Navigation or router changes.

## Risks

- Existing shared action rows still contain inline styles; L1 must decide whether the no-inline gate is restricted to edited page files or expanded to shared row internals.
- Detail page dynamic state currently lives in inline style props; moving it to classes must not lose wait/disabled affordances.
- `EditableDate` accepts an `urgencyColor` prop; if that component applies inline styles internally, this ticket should not broaden unless L1 expands the gate.
- Removing apparently unused CSS requires checking TSX references first.

## L1 Gates

- `pnpm tsc --noEmit`
- Run targeted tests if added for the pages.
- `pnpm test src/pages/DailyBriefingRedesign.test.tsx` to guard shared editorial component assumptions.
- `rg "style=\\{\\{" src/pages/ActionsPage.tsx src/pages/ActionDetailPage.tsx`
- `rg "DOS-" src/pages/ActionsPage.tsx src/pages/ActionDetailPage.tsx src/pages/ActionsPage.module.css src/pages/ActionDetailPage.module.css`
- `rg "^\\.[a-z0-9]+[-_]" src/pages/ActionsPage.module.css src/pages/ActionDetailPage.module.css`
- `rg "data-ds-(name|tier|spec)" src/pages/ActionsPage.tsx src/pages/ActionDetailPage.tsx`
- Manual browser check of `/actions`: suggested, active, completed, search, create form, grouped active rows, and empty states.
- Manual browser check of `/actions/$actionId`: loading/error if reachable, open/completed state, save status, due date, account/source edits, and Linear section if enabled.

## L2 Gates

- Design review: actions reads as part of the briefing redesign family without becoming a briefing page clone.
- Code review: visual changes are isolated to page TSX and page CSS modules, with no backend or mutation drift.
- Reuse review: generic `ActionRow` pattern remains the list row foundation; `WatchRow` is not imported.
- Inspector review: surface roots expose correct `data-ds-*` attributes and spec links.
- Regression review: no interactive affordance is lost while removing inline styles.
- Scope review: any expansion into shared row internals is called out separately before implementation.
