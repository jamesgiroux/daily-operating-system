# Surfaces

Full screens. The user-visible products built on top of patterns and primitives. DailyBriefing, AccountDetail, MeetingDetail, ProjectDetail, PersonDetail, Settings.

## Index

_(populated as surfaces are documented)_

| Canonical name | Current src name | Status | Spec |
|---|---|---|---|
| DailyBriefing | `Dashboard.tsx` | Redesigning (v1.4.3) | _to be added_ |
| AccountDetail | `AccountDetail.tsx` (?) | Canonical (recent v1.4.2 redesign) | _to be added_ |
| MeetingDetail | _(verify)_ | Redesigning | _to be added_ |
| ProjectDetail | _(verify)_ | In v1.4.2 scope | _to be added_ |
| PersonDetail | _(verify)_ | In v1.4.2 scope | _to be added_ |
| Settings | _(verify)_ | Redesigning (separate project) | _to be added_ |

## What a surface spec captures

Each surface gets one `.md` file with:

- **Job** — what the user accomplishes here
- **Canonical name** vs current `src/` name (rename status if mismatched — see `../NAMING.md`)
- **Source files** — every file under `src/` that implements this surface
- **Patterns consumed** — in reading order, with links
- **Primitives consumed**
- **Local nav approach** — which local-nav pattern this surface uses (and why, if it differs from a peer)
- **Layout regions** — header, spine, sidebar, dock, etc.
- **Empty / loading / error states**
- **Naming notes** — rename history, candidate renames, decisions deferred

## Conventions

- **Surface specs are the contract for what the surface is.** Implementation in `src/` should match. If they disagree, the spec wins (or the spec gets updated, deliberately).
- **A surface re-implementing a pattern is a smell.** Either the pattern is missing a variant, or the surface is wrong, or the pattern is wrong. Resolve, don't paper over.
- **Don't duplicate the figma/mockup here.** Link to it. The spec is the contract; the mockup is a reference.

## Surface-internal components

Components that are genuinely unique to one surface (and have no plausible reuse) live in `src/` next to the surface, not in `primitives/` or `patterns/`. They don't get a markdown spec here. If two surfaces start needing it, *that's* the trigger to promote.
