# DOS-432 — Emails surface uplift

**Status:** L0 plan, awaiting reviewer signoff before implementation.
**Wave:** W5 redesign
**Route:** `/emails`

## 1. Acceptance criteria

- [ ] `/emails` adopts the Daily Briefing redesign chrome: `FolioBar`, `FloatingNavIsland`, and `AtmosphereLayer`.
- [ ] Surface root has ds-inspector attrs: `data-ds-name="EmailsPage"`, `data-ds-tier="surface"`, `data-ds-spec="surfaces/EmailsPage.md"`.
- [ ] `EmailsPage.module.css` uses `.root` as the surface wrapper and camelCase child selectors only.
- [ ] Styling uses design-system 0.6.0 token aliases from `src/styles/design-tokens.css`; no new raw paint literals.
- [ ] `EmailsPage.tsx` has no inline CSS (`style={{ ... }}`).
- [ ] Existing behavior is unchanged: loading, retry, Gmail disconnected empty, connected empty, priority filtering, extracted commitments/questions, gone-quiet alerts, update dismissal, inbox bands, archive/undo, pin, Gmail open, meeting navigation, and commitment promotion all still work.
- [ ] Visual register matches `DailyBriefingRedesign`: paper ground, fixed folio, right floating nav, editorial section rhythm, mono labels, restrained rules, no decorative card-heavy rewrite.
- [ ] Reuses W1/W3 components or existing UI primitives; no reinvented buttons, pills, badges, nav, or state blocks.
- [ ] No `DOS-` ticket references remain in comments in touched source files.

## 2. What is changing

This is a visual/style alignment only. The current Correspondent surface keeps its information architecture, state derivation, and action handlers, but shifts from the older magazine register to the W5 Daily Briefing redesign register.

Implementation shape:

- Render an explicit `EmailsPage` surface wrapper, following the routed `DailyBriefingRedesign.tsx` structure.
- Add `AtmosphereLayer color="turmeric"`.
- Render `FolioBar` with `publicationLabel="The Correspondent"` and the existing `EmailRefreshButton` in the actions slot.
- Render `FloatingNavIsland` with `activePage="emails"` and `activeColor="turmeric"`.
- Move page content into module-owned `main`/content classes.
- Keep all section conditions and event handlers intact.

## 3. Files

Primary implementation files:

```
src/pages/EmailsPage.tsx
src/pages/EmailsPage.module.css
```

Plan file:

```
.docs/plans/wave-W5-redesign/DOS-432-plan.md
```

Optional only if implementation exposes spec drift:

```
.docs/design/surfaces/EmailsPage.md
```

## 4. Reuse targets

- Chrome: `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`.
- Refresh action: existing `EmailRefreshButton` and `FolioRefreshButton`.
- State handling: keep current `EditorialLoading`, `EditorialError`, and `EmptyState` unless swapping to W3 briefing states is behavior-neutral.
- Email primitives: keep `EmailEntityChip`, `EntityPicker`, `DatePicker`, and existing lucide icon actions.
- Layout vocabulary: mirror `DailyBriefingRedesign.module.css` where it fits: `root`, `chrome`, `main`, `stateMain`, `content`, `section`, `sectionLabel`, `sectionBody`, `sectionHeader`.

## 5. Cardinal rules

- **No inline CSS:** move the current sentiment-dot runtime paint to classes or data attrs, e.g. `.sentimentDot[data-sentiment="positive"]`.
- **CSS convention:** `.root` wrapper; camelCase children. Rename/remove selectors like `ageBadge_1_2_days`.
- **ds-inspector:** surface wrapper carries surface-tier attrs; reused components keep their own pattern/primitive attrs.
- **No DOS refs in code comments:** delete or rewrite ticket-number comments as product intent comments.
- **No new primitives:** local markup is composition only.
- **No behavior rewrite:** visual variants use classes/data attrs, not new data logic.

## 6. Backward compatibility

Do not change:

- Tauri command names or payloads.
- `EmailBriefingData`, `EmailSyncStats`, `EnrichedEmail`, `FailedEmailPreview`, or `TrackedEmailCommitment` shapes.
- Score thresholds: Your Move `>= 0.15`; inbox priority `> 0.40`, monitoring `0.15..0.40`, other `< 0.15`.
- Dismissal keys (`commitment:${text}`, `question:${text}`).
- Optimistic archive/undo behavior.
- Pin, Gmail open, meeting navigation, and commitment promotion flows.
- Gmail disconnected vs connected-empty branching.
- Retry, Skip, View details, and Dismiss semantics for enrichment failures.

## 7. Out of scope

- Email service backend changes.
- Email data-shape or view-model changes.
- Enrichment, sync, retry, skip, archive, pin, or dismissal command changes.
- Re-ranking or reclassifying emails.
- New design-system primitives.
- Global `MagazinePageLayout` redesign.
- Changing the Correspondent information architecture.

## 8. Implementation notes

1. First verify whether route-level `MagazinePageLayout` still wraps `EmailsPage`. Avoid double-rendering folio/nav; if direct chrome conflicts, use the smallest shell-contract adjustment and document it before L1.
2. Replace `useRegisterMagazineShell` only if direct chrome composition is the correct routed pattern for this surface.
3. Keep `useMemo`, `useCallback`, `invoke`, Tauri event subscriptions, and local optimistic state intact.
4. Refactor CSS after markup structure is stable; do not mix behavior edits into the CSS convention pass.
5. Remove dead reply-debt CSS if still unreachable; otherwise camelCase it.
6. Clean touched comments with ticket refs during the same pass.

## 9. L1 self-validation gates

- `pnpm lint` clean, or the repo's narrower lint equivalent.
- Run an EmailsPage test if one exists; otherwise run the nearest page smoke test plus `pnpm test src/pages/DailyBriefingRedesign.test.tsx`.
- `rg -n "style=\\{\\{" src/pages/EmailsPage.tsx` returns no matches.
- `rg -n "DOS-" src/pages/EmailsPage.tsx src/pages/EmailsPage.module.css` returns no matches.
- `rg -n "\\.[A-Za-z0-9]+_[A-Za-z0-9_]+" src/pages/EmailsPage.module.css` returns no matches.
- `rg -n "#[0-9a-fA-F]{3,8}|rgba?\\(|hsla?\\(" src/pages/EmailsPage.module.css` shows no new local paint literals.
- Browser QA desktop + mobile: folio, atmosphere, floating nav, loading, error, empty, refresh, archive, pin, Gmail open, commitment tracking, and gone-quiet dismissal all work.

## 10. L2 gates

- **Design review:** confirms `/emails` reads as the redesigned Daily Briefing register, not old styling with new chrome pasted on.
- **Code review:** confirms behavior neutrality, no command/data-shape changes, and no invented local primitives.
- **CSS review:** confirms `.root` + camelCase, token-only styling, responsive text fit, and no inline CSS escape hatches.
- **Inspector review:** confirms `EmailsPage`, `FolioBar`, `FloatingNavIsland`, and `AtmosphereLayer` expose correct ds-inspector attrs.

## 11. Risk notes

- Highest-risk miss: current sentiment dot uses inline style. Convert early.
- The current page mixes module CSS with `editorial-briefing.module.css`; reduce that coupling without rewriting every email row.
- Existing source comments include ticket refs. Clean touched blocks without turning this into a broad history rewrite.
