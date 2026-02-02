# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Changes in development that will be included in the next release.

---

## [0.3.1] - 2026-02-01

### Fixed

- **Restored docs landing page structure**
  - `index.html` is now the 14-slide Setup Guide (was accidentally replaced by wizard)
  - `intro.html` remains the 41-slide Overview presentation
  - Web wizard moved to `setup.html` (accessible but not the landing page)

- **Wizard server updated** to serve `setup.html` instead of `index.html`

### Changed

- Removed auto-open browser behavior from CLI wizard (`advanced-start.py`)
  - `easy-start.command` is now the preferred entry point for visual setup

---

## [0.3.0] - 2026-02-01

### Added

- **Web-based setup wizard** (`easy-start.command`)
  - Visual step-by-step setup with real-time progress indicators
  - Node.js backend with Express server at localhost:5050
  - Same capabilities as CLI wizard in a friendlier interface
  - Auto-launches in default browser on macOS

- **Centralized workspace configuration** (`_config/workspace.json`)
  - Single source of truth for workspace settings
  - JSON schema validation (`workspace-schema.json`)
  - Organization name and internal email domains stored here
  - Feature flags for Google API, web dashboard, Python tools

- **Organization config collection** during setup
  - Workspace name (human-readable)
  - Organization/company name
  - Internal email domains (for meeting classification)
  - Both CLI and Web UI collect identical configuration

- **First-run detection** in `/today`, `/wrap`, `/week`
  - Detects fresh workspaces and provides friendly onboarding
  - Explains manual mode vs full mode clearly
  - Guides users to `/setup --google` for API configuration

- **Claude local installation step** in setup wizard
  - Checks for Claude Code CLI availability
  - Creates `.claude/` directory structure
  - IDE selection for post-setup instructions

- **Quick install mode** in CLI wizard (`--quick`)
  - Skips optional features for faster setup
  - Uses sensible defaults

### Changed

- **Renamed entry points** for clarity
  - `setup.py` → `advanced-start.py` (CLI wizard)
  - `start.command` → `easy-start.command` (Web wizard)

- **Relative paths in command templates**
  - All commands now use relative paths (e.g., `_tools/prepare_today.py`)
  - Templates portable across different workspace locations
  - No more hardcoded `/Users/*/Documents/VIP/` paths

- **Internal domains loaded from config** instead of hardcoded
  - `meeting_utils.py` reads from `_config/workspace.json`
  - `prepare_today.py` uses config-based domain classification
  - Empty set fallback for unconfigured workspaces

### Fixed

- Session persistence errors in web wizard
- Missing `file_ops` arguments in Git, Google API, and Skills steps
- Visual indicators and error handling improvements
- CLAUDE.md generation edge cases

---

## [0.2.1] - 2026-02-01

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

## [0.2.0] - 2026-01-30

### Added

- Web dashboard with visual navigation and markdown rendering
- Role-based configuration system (8 roles)
- Dashboard health indicators and ring badges for Customer Success roles

### Fixed

- Spinner context manager error handling
- Made error messages more beginner-friendly
- Removed email support option from error handling

---

## [0.1.0] - 2026-01-29

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

[Unreleased]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgiroux/daily-operating-system/releases/tag/v0.1.0
