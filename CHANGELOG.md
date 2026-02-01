# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Changes in development that will be included in the next release.

---

## [0.8.0] - 2026-02-01

### Added

- **Prep status lifecycle** across `/week`, `/today`, and `/wrap` commands
  - Progressive status tracking: `📋 Prep needed` → `✅ Prep ready` → `✅ Done`
  - Four prep types: Customer (📋/📅), Project (🔄), Internal (👥)
  - Agenda ownership logic based on relationship stage
  - Auto-update of week overview as prep files are generated
  - Resilience: `/today` creates minimal week overview if `/week` wasn't run

- **Enhanced internal meeting prep** in `/today`
  - 1:1 meetings get relationship-focused prep (shared work, context)
  - Team syncs get "Your Updates to Share" format

- **Prep completion reconciliation** in `/wrap`
  - Auto-marks completed meetings as `✅ Done`
  - Smart prompting only for pending agenda items that can't be auto-resolved

### Changed

- Week overview table now includes `Prep Status` column with status icons
- Daily overview shows prep status progress for each meeting
- Wrap summary includes agenda task status

---

## [0.7.0] - 2026-01-30

### Added

- Web dashboard with visual navigation and markdown rendering
- Role-based configuration system (8 roles)
- Dashboard health indicators and ring badges for Customer Success roles

### Fixed

- Spinner context manager error handling
- Made error messages more beginner-friendly
- Removed email support option from error handling

---

## [0.6.0] - 2026-01-29

### Added

- Initial public release
- Setup wizard with 10-step guided installation
- 8 slash commands (`/today`, `/wrap`, `/week`, `/month`, `/quarter`, `/email-scan`, `/git-commit`, `/setup`)
- 3 skill packages (inbox, strategy-consulting, editorial)
- 16 specialized agents
- Google API integration (Calendar, Gmail, Sheets, Docs)
- Role-based directory structure templates
- PARA folder organization

---

## Version Numbering

This project uses [Semantic Versioning](https://semver.org/):

- **0.x.y**: Pre-release development (current)
- **1.0.0**: First stable release (target)

During pre-release (0.x.y):
- Minor version bumps (0.**x**.0) for new features
- Patch version bumps (0.0.**y**) for bug fixes

[Unreleased]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/jamesgiroux/daily-operating-system/releases/tag/v0.6.0
