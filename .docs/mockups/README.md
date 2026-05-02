# Mockups — Exploration

This directory holds **exploration and research**. Mockups, iterations, drops from Claude Design, hand-drawn variations. Nothing here is canonical.

The canonical design system lives in `../design/`. See `../design/SYSTEM-MAP.md` for the relationship.

## Layout

- `current/` — active iteration projects, one folder per surface or initiative. Drop new mockups here.
- `_archive/` — variations and projects that didn't make it. Kept for reference.

The top-level `.html` files and the `v2/` and `claude-design-project/` directories will be sorted into `current/` or `_archive/` during the audit synthesis.

## Naming convention for new drops

When dropping in mockups from Claude Design (or any external tool):

```
current/<surface>-<date>-<source>/
```

Examples:

- `current/briefing-2026-04-claude-design/`
- `current/settings-2026-04-claude-design/`
- `current/account-detail-2026-03-figma/`

Include a `README.md` in each project explaining:

- What was being explored
- What variations are in here
- Status (active, decided, abandoned)
- Decisions that came out of it

## Lifecycle

```
new drop      →    audit / review     →    decision         →    promotion or archive
current/...        _audits/...             surface spec          design/{tier}/Name.md
                                           or rejection          + mockups/_archive/
```

When a mockup's design becomes canonical:

1. Per-entry markdown specs land in `.docs/design/`.
2. The CSS/JS substrate (if any) gets reviewed for promotion to `.docs/design/reference/_shared/`.
3. The mockup project folder either stays in `current/` (if iteration is ongoing) or moves to `_archive/` with a note pointing at the canonical entries.

## What does NOT go here

- Production code → `src/`
- Canonical specs → `../design/`
- Version-specific plans → `../plans/`
- Audit reports → `claude-design-project/_audits/` for now (will reorganize)
