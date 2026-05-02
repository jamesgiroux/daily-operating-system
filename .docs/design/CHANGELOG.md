# Design System Changelog

All notable changes to the DailyOS design system. See `VERSION.md` for bump rules.

Format:

```
## [version] — YYYY-MM-DD

### Added
- ...

### Changed
- ...

### Removed
- ...

### Notes
- Migration notes, deprecations, things consumers should know
```

---

## [0.0.0] — 2026-05-02

### Added

- Initial scaffolding: directory structure for `tokens/`, `primitives/`, `patterns/`, `surfaces/`, `reference/`, `_archive/`
- `SYSTEM-MAP.md` — taxonomy, lifecycle, conventions
- `NAMING.md` — naming policy + first rename candidate (`Dashboard` → `DailyBriefing`)
- `_TEMPLATE-entry.md` — entry template
- `VERSION.md`, `CHANGELOG.md` — versioning ground truth
- `reference/_shared/inspector.js` + `inspector.css` — opt-in hover inspector for reference renders
- `data-ds-*` convention documented in `SYSTEM-MAP.md`
- `.docs/mockups/` demoted to exploration-only with `current/` and `_archive/` subdirs

### Notes

- No canonical entries yet. The four foundational audits are running in parallel; their findings will populate the first canonical entries and trigger a bump to `0.1.0`.
- Existing `.docs/design/*.md` files (DESIGN-SYSTEM.md, COMPONENT-INVENTORY.md, etc.) remain in place pending audit synthesis. They will move to `_archive/` and per-entry specs will become canonical.
