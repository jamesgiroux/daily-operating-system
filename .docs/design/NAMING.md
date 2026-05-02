# Naming Discipline

Names are part of the design system. A surface called `Dashboard.tsx` that shows a Daily Briefing is a small lie that compounds — over time the team uses both names interchangeably, the docs talk about "the briefing", the code talks about "the dashboard", and new contributors have to learn the mapping.

This document captures the policy and the active rename track.

## Policy

**Components are named after the user-visible job, not implementation details.**

- ✅ `BriefingSpine` (the spine of a briefing layout)
- ❌ `MainContent`, `PageBody`, `Container`

**Surfaces are named after what the user calls them.**

- ✅ `DailyBriefing` (this is what we and the user say out loud)
- ❌ `Dashboard` (legacy; nobody calls it this)

**Patterns are PascalCase, named for the pattern, not the surface.**

- ✅ `TrustBand`, `ClaimRow`, `LocalNavIsland`
- ❌ `BriefingTrustBand` (it's not unique to briefing)

**Primitives are generic and short.**

- ✅ `Button`, `Pill`, `Chip`, `Card`
- ❌ `PrimaryActionButton`, `BaseButton`

**No "Generic", "Base", "Common", or numeric suffixes.** If you need a suffix to disambiguate, the name is wrong.

## Rename track — candidates

These are renames the audits and reviews have surfaced (or will surface). Each is a rename PR that updates code, routes, tests, docs, and surface specs together.

| Current (in `src/`) | Canonical | Status | Notes |
|---|---|---|---|
| `Dashboard.tsx` (route, file, component) | `DailyBriefing` | Proposed | Top of the rename queue. User-visible name everywhere. |
| _(awaiting audit findings)_ | | | Audit 01 will surface more |

## Rename procedure

A rename is a small PR. Don't bundle with feature work.

1. Open a PR titled `rename(<old>): <old> → <canonical>`
2. Update: file name, component name, all imports, route name, test names, fixture names, surface spec, any docs that reference the old name
3. Add a one-line note to the surface's `.md` history section
4. Land

Renames in flight should not block other work — if your branch is mid-rename, downstream work targets the new name and we resolve at merge.

## When *not* to rename

Sometimes the current name is fine and the urge to rename is just polish. Skip if:

- The current name already reflects the user-visible job
- The rename would touch >50 files for cosmetic reasons
- A bigger redesign is imminent that will obviate the question

## Open questions

- Do we rename **routes** (`/dashboard` → `/briefing`)? Argument for: external consistency. Argument against: breaks bookmarks. Default position: yes, with redirect, but defer until a major version bump.
- Do we rename **DB tables / API endpoints** that use legacy names? Default: no, unless they're user-facing or already breaking.
