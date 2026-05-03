# Surfaces

Full screens. The user-visible products built on top of patterns and primitives. DailyBriefing, AccountDetail, MeetingDetail, ProjectDetail, PersonDetail, Settings.

## Index

| Canonical name | Current src name | Status | Spec |
|---|---|---|---|
| [`DailyBriefing`](./DailyBriefing.md) | `Dashboard.tsx` | Redesigning (v1.4.3) | ✓ Wave 1 (0.1.0) |
| [`Settings`](./Settings.md) | `src/features/settings-ui/` | Redesigning (separate project) | ✓ Wave 3 (0.3.0) |
| [`MeetingDetail`](./MeetingDetail.md) | _verify_ | Redesigning | ✓ Wave 4 (0.4.0) |
| `AccountDetail` | `AccountDetailPage.tsx` | Canonical (recent v1.4.2 redesign) | _Wave 5 / surface pass_ |
| `ProjectDetail` | `ProjectDetailEditorial.tsx` | In v1.4.2 scope | _Wave 5 / surface pass_ |
| `PersonDetail` | `PersonDetailEditorial.tsx` | In v1.4.2 scope | _Wave 5 / surface pass_ |

## What a surface spec captures

Each surface gets one `.md` file with:

- **Job** — what the user accomplishes here
- **Canonical name** vs current `src/` name (rename status if mismatched — see `../NAMING.md`)
- **Source files** — every file under `src/` that implements this surface
- **Layout regions** — header, spine, sidebar, dock, etc.
- **Local nav approach** — chapter inventory provided to `FloatingNavIsland` (per D2)
- **Patterns consumed** — in reading order, with links
- **Primitives consumed**
- **Notable interactions**
- **Empty / loading / error states**
- **Naming notes** — rename history, candidate renames, decisions deferred

## Conventions

- **Surface specs are the contract for what the surface is.** Implementation in `src/` should match. If they disagree, the spec wins (or the spec gets updated, deliberately).
- **A surface re-implementing a pattern is a smell.** Either the pattern is missing a variant, or the surface is wrong, or the pattern is wrong. Resolve, don't paper over.
- **Surfaces provide chapters to `FloatingNavIsland`.** Per D2, surfaces do not invent local nav patterns. Their chapter inventory lives in their surface spec.
- **Don't duplicate the figma/mockup here.** Link to it. The spec is the contract; the mockup is a reference.

## Surface-internal components

Components that are genuinely unique to one surface (and have no plausible reuse) live in `src/` next to the surface, not in `primitives/` or `patterns/`. They don't get a markdown spec here. If two surfaces start needing it, *that's* the trigger to promote.
