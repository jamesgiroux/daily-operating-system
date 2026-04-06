# Plan Migration Inventory — Pass C

_Date: 2026-04-05_

This inventory covers `.docs/plans/**` and classifies active version brief docs into shipped, future, archive, and Linear-migration status.


## Summary

- Latest git tag: **v1.1.1**
- Active plan docs: **13**
- Archived plan docs: **28**
- Linear projects currently found with version names: **1**

## Migration Policy

- Linear is the canonical home for active project/version briefs.
- Shipped versions should be represented primarily by git tags, `CHANGELOG.md`, and optional release notes.
- Markdown version briefs should not remain the primary execution tracker once the content is migrated to Linear.
- Historical briefs may be archived if they still provide useful rationale; otherwise they should be retired.

## Active Plan Docs

| Plan Doc | Heading | Classified As | Linear Project | Recommended Action | Notes |
|---|---|---|---|---|---|
| `.docs/plans/v1.0.1.md` | v1.0.1 — The Correspondent: Email Intelligence for Customer Success | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.0.2.md` | v1.0.2 — Fix & Reconnect | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.0.3.md` | v1.0.3 — The Meeting Record | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.0.4.md` | v1.0.4 — Test & Trust | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.1.0.md` | v1.1.0 — Lifecycle Intelligence + Briefing Depth | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.1.1.md` | v1.1.1 — Entity Linking as a Service + Security Hardening | shipped-or-past | — | Archive or retire after confirming changelog/release history is sufficient | — |
| `.docs/plans/v1.1.2.md` | v1.1.2 — Transcript Routing Fix | future | — | Create/map Linear project, then migrate brief out of markdown | — |
| `.docs/plans/v1.1.3.md` | v1.1.3 — Design Hardening | future | — | Create/map Linear project, then migrate brief out of markdown | — |
| `.docs/plans/v1.2.0.md` | v1.2.0 — Actions & Success Plans: Closing the Loop | future | v1.2.0 — Actions & Success Plans: Closing the Loop | Migrate fully to Linear project description, then shrink or retire markdown brief | — |
| `.docs/plans/v1.3.0.md` | v1.3.0 — Report Engine Rebuild: Intelligence-First, Display-Only Reports | future | — | Create/map Linear project, then migrate brief out of markdown | — |
| `.docs/plans/v1.4.0.md` | v1.4.0 — Publication + Portfolio + Intelligence Quality | future | — | Create/map Linear project, then migrate brief out of markdown | — |
| `.docs/plans/v2.1.0.md` | v1.1.0 Version Brief | future | — | Create/map Linear project, then migrate brief out of markdown | heading/version mismatch |
| `.docs/plans/v2.2.0.md` | v1.2.0 Version Brief | future | — | Create/map Linear project, then migrate brief out of markdown | heading/version mismatch |

## Archived Plan Docs

| Plan Doc | Notes |
|---|---|
| `.docs/plans/_archive/ga-ux-audit.md` | Historical archive |
| `.docs/plans/_archive/phase-2-execution-tracker.md` | Historical execution tracker |
| `.docs/plans/_archive/phase-3-execution-tracker.md` | Historical execution tracker |
| `.docs/plans/_archive/v0.13.0-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.1-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.2-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.3-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.4-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.5-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.6-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.7-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.8-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.13.9-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.14.1-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.14.2-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.14.3-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.15.0-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.15.1-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.15.2-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.16.0-shipped.md` | Historical shipped plan |
| `.docs/plans/_archive/v0.16.1-dissolved.md` | Historical dissolved plan |
| `.docs/plans/_archive/v0.16.2-dissolved.md` | Historical dissolved plan |
| `.docs/plans/_archive/v1.0.0-review-fixes.md` | Historical archive |
| `.docs/plans/_archive/v1.0.0.md` | Historical archive |
| `.docs/plans/_archive/v1.0.1-dissolved.md` | Historical dissolved plan |
| `.docs/plans/_archive/v1.1.0-dissolved.md` | Historical dissolved plan |
| `.docs/plans/_archive/v1.1.1-dissolved.md` | Historical dissolved plan |
| `.docs/plans/_archive/v1.1.2-dissolved.md` | Historical dissolved plan |

## Notable Drift / Inconsistencies

- `.docs/plans/v2.1.0.md` has heading `v1.1.0 Version Brief` — clear mismatch.
- `.docs/plans/v2.2.0.md` has heading `v1.2.0 Version Brief` — clear mismatch.
- `v1.0.1` through `v1.1.1` are at-or-before the latest shipped tag and should not remain active planning docs forever.
- Only `v1.2.0` is currently visible as a Linear project in the current project list output.

## Immediate Recommendations

1. Migrate `v1.2.0` completely to Linear and reduce the markdown brief to an index/reference or retire it.
2. Treat `v1.1.2`, `v1.1.3`, `v1.3.0`, and `v1.4.0` as next migration candidates into Linear projects if they are still strategically live.
3. Reclassify `v1.0.1`–`v1.1.1` as historical shipped/past briefs and move them out of the active planning surface.
4. Fix or retire `v2.1.0.md` and `v2.2.0.md` — they currently create confusion rather than clarity.
5. After migration, replace `.docs/plans/` with either an archive + small index, or retire the directory entirely as an active planning surface.

## Suggested Next Cleanup Batch

- write a tiny README for `.docs/plans/` explaining that active planning now lives in Linear
- move shipped/past briefs into `_archive/` in a single low-risk batch
- isolate or remove mismatched speculative briefs (`v2.1.0`, `v2.2.0`) after human signoff
- avoid mixing migration with architectural doc refresh in the same commit
