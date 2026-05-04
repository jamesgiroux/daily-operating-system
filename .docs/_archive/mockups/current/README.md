# Current mockup projects

Active iteration projects. One folder per surface or initiative.

## Naming convention

```
<surface>-<date>-<source>/
```

Examples:

- `briefing-2026-04-claude-design/`
- `settings-2026-04-claude-design/`
- `meeting-detail-2026-04-claude-design/`

## Each project should have

- `README.md` — what's being explored, status, variations, decisions
- The mockup files (HTML, CSS, JSX, etc.)
- A pointer back to its audit report (when one exists)

## When a project's done

Either:
- **Promoted** — canonical specs land in `../../design/`, and this folder either stays here (if iterating further) or moves to `../_archive/` with a redirect note.
- **Abandoned** — moves to `../_archive/` with a note explaining why and what was learned.
