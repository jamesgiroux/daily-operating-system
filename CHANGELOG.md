# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Changes in development that will be included in the next release.

---

## [0.6.0] - 2026-02-03

### Added

- **Secure credential storage** (`~/.dailyos/google/`)
  - Credentials now stored in user home directory with restricted permissions (0o600)
  - Keeps credentials out of workspaces for safer sharing and git operations
  - Automatic migration from legacy workspace location

- **Guided Google API setup** (`dailyos google-setup`)
  - Interactive 6-step CLI wizard walks through Google Cloud Console setup
  - `--verify` flag to check current Google API status
  - `--reset` flag to clear credentials and start over
  - Opens browser automatically for each setup step

- **Web wizard improvements** (`docs/setup.html`)
  - File upload dropzone with drag-and-drop support for credentials.json
  - Paste option for JSON content
  - Real-time validation feedback
  - Test connection button before completing setup

- **API endpoints for Google setup** (`server/routes/setup.js`)
  - `POST /api/setup/google/upload-credentials` - Upload and validate credentials
  - `POST /api/setup/google/test-auth` - Test authentication
  - `GET /api/setup/google/status` - Get current setup status

- **Enhanced error handling** (`templates/scripts/google/google_api.py`)
  - Error classification with actionable remediation messages
  - Persistent error logging to `~/.dailyos/google/error.log`
  - Automatic retry with exponential backoff for transient errors (429, 500, 503)

- **New test suite** (`tests/test_google_api_setup.py`)
  - 52 new tests covering all Google API setup functionality
  - Tests for credential validation, secure storage, error handling, retry logic

### Changed

- Google credentials path changed from `workspace/.config/google/` to `~/.dailyos/google/`
- Improved setup instructions to reference new `dailyos google-setup` command

---

## [0.5.3] - 2026-02-03

### Fixed

- **Email summary overwritten by delivery script**
  - `deliver_today.py` was unconditionally overwriting `83-email-summary.md` during Phase 3
  - This destroyed AI-enriched content created during Phase 2 (classifications, conversation arcs, recommendations)
  - Now checks if file was modified after directive creation (timestamp) or has actual classifications (content markers)
  - Only writes template if file doesn't exist or still contains placeholder text

---

## [0.5.2] - 2026-02-03

### Added

- **Smart workspace detection** for `dailyos start`
  - Works from any directory without `-w` flag
  - Priority cascade: explicit flag → current directory → stored default → auto-scan
  - Scans ~/Documents, ~/workspace, ~/projects, ~/dev for `.dailyos-version` marker
  - Interactive workspace picker when multiple workspaces found
  - Offers to save auto-detected workspace as default

- **Configuration management** (`dailyos config`)
  - `dailyos config` — Show current configuration
  - `dailyos config workspace` — Show default workspace
  - `dailyos config workspace ~/path` — Set default workspace
  - `dailyos config scan` — Rescan for workspaces
  - `dailyos config reset` — Reset to defaults

- **New config file** (`~/.dailyos/config.json`)
  - Stores default workspace path
  - Tracks known workspaces with last-used timestamps
  - Configurable scan locations and depth
  - User preferences (auto-save, prompt behavior)

- **New module** (`src/workspace.py`)
  - `WorkspaceConfig` class for config management
  - `WorkspaceScanner` class for workspace discovery
  - `WorkspaceResolver` class for smart resolution

### Changed

- `dailyos start` now auto-detects workspace when run from any directory
- `dailyos start --set-default` flag to save workspace as default
- Improved error messages with setup instructions when no workspace found

---

## [0.5.1] - 2026-02-02

### Changed

- **Automatic CLI installation** in `easy-start.command`
  - `dailyos` CLI now installs automatically without prompting
  - Advanced users via `advanced-start.py` still get the optional prompt

- **Smart port detection** in `easy-start.command`
  - Detects if port 5050 is already in use
  - If DailyOS is running: offers to use existing server, stop it, or use different port
  - If other app is running: notifies user and finds alternative port (5051-5060)
  - Graceful error if all ports are in use

---

## [0.5.0] - 2026-02-02

### Added

- **Server management commands** for the `dailyos` CLI
  - `dailyos start` — Start the web UI server (auto-detects workspace, auto-installs dependencies)
  - `dailyos stop` — Stop the web UI server
  - `dailyos ui` — Show web UI status (running/stopped, URL, PID)
  - Options: `--port/-p` for custom port, `--no-browser` to skip browser open
  - Graceful handling: "already running" detection, zombie process cleanup
  - Works from any directory via workspace auto-detection

- **CLI documentation** (`docs/cli-reference.md`)
  - Complete reference for all `dailyos` commands
  - Examples for common workflows

### Fixed

- **Symlink resolution in `dailyos` bash wrapper**
  - Fixed bug where SCRIPT_DIR resolved to `/usr/local/bin/` when invoked via symlink
  - Now correctly resolves to actual script location

---

## [0.4.1] - 2026-02-02

### Fixed

- **Setup wizard opening wrong page** — `easy-start.command` was opening `docs/index.html` instead of `docs/setup.html` because Express static middleware runs before explicit routes. Fixed by moving the root route handler before the static middleware.

---

## [0.4.0] - 2026-02-02

### Added

- **Waiting On (Delegated) Tracking**
  - New `extract_waiting_on()` function parses "Waiting On (Delegated)" table from master task list
  - `/today` now extracts delegated items (Who, What, Asked, Days, Context)
  - Overview shows Waiting On count in stats bar and sidebar card
  - Actions file includes full Waiting On table with follow-up tip
  - Web UI already supports display via existing `buildWaitingCard()` transform

- **Version Management System** (WordPress-inspired auto-update architecture)
  - Symlink-based installation: workspaces link to `~/.dailyos` core, enabling automatic updates
  - Daily update check: `/today` checks for updates once per day, prompts with options
  - Eject/reset pattern: power users can customize skills while tracking changes
  - `dailyos` CLI tool with commands: `version`, `status`, `update`, `doctor`, `repair`, `eject`, `reset`
  - Self-healing: `dailyos doctor` detects broken symlinks, missing files; `repair` fixes them

- **Core Initialization**
  - `easy-start.command` now initializes `~/.dailyos` core before web wizard
  - Optional CLI installation to `/usr/local/bin/dailyos`
  - Setup wizard uses symlinks instead of file copies when core exists

- **Version Tracking**
  - `.dailyos-version` file tracks installed version per workspace
  - `.dailyos-ejected` file tracks customized components
  - `.dailyos-last-check` prevents redundant daily update checks

### Changed

- Setup wizard (`wizard.py`, `steps/skills.py`) now uses symlink installation by default
- Commands, skills, agents link to core rather than being copied

---

## [0.3.4] - 2026-02-02

### Added

- **Bidirectional task sync in /wrap**
  - When tasks are marked complete in master task list, completion syncs back to source account files
  - Matches by task ID (e.g., `2026-01-20-002`) or by title (first 4 words)
  - Keeps account action files clean - no more zombie unchecked items
  - Especially useful for accounts with monthly cadence where items could otherwise accumulate

---

## [0.3.3] - 2026-02-02

### Fixed

- **JSON parsing resilience for Google API output**
  - Google API scripts can print Python warnings (NotOpenSSLWarning, FutureWarning, etc.) to stdout before JSON data
  - This caused `json.loads()` to fail with "Expecting value: line 1 column 1" errors
  - Added `extract_json_from_output()` helper that strips warning text before parsing
  - Fixed in `calendar_utils.py`, `meeting_utils.py`, and `prepare_today.py`

- **Web dashboard action item count showing inflated numbers**
  - `querySelectorAll('li')` was counting ALL list items including nested metadata bullets
  - Changed to `querySelectorAll(':scope > li')` to count only top-level action items
  - Fixed in `templates/ui/public/js/markdown.js`

- **Week overview showing personal events and broken table formatting**
  - Personal events (Home, Daily Prep, Post-Meeting Catch-Up) now filtered out
  - Meeting titles now display properly with account context
  - Fixed time parsing when `start_display` is missing
  - Pipe characters in meeting titles now escaped to prevent markdown table breaks
  - Fixed in `templates/scripts/daily/deliver_week.py`

---

## [0.3.2] - 2026-02-01

### Added

- **Dashboard auto-start**: Web dashboard can now auto-start when `/today` runs
  - New `dashboard_utils.py` module in `templates/scripts/lib/`
  - Feature flag `web_dashboard_autostart` in workspace config
  - `--skip-dashboard` CLI flag for `prepare_today.py`
  - Idempotent behavior - detects if dashboard already running on port 5050
  - Silent, non-blocking startup before other preparation steps

### Changed

- Updated `prepare_today.py` to integrate dashboard auto-start
- Updated `workspace-schema.json` with `web_dashboard_autostart` property

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

[Unreleased]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.5.3...HEAD
[0.5.3]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/jamesgiroux/daily-operating-system/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgiroux/daily-operating-system/releases/tag/v0.1.0
