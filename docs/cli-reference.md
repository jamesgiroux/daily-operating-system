# DailyOS CLI Reference

The `dailyos` command-line tool provides workspace management, server control, and health checking capabilities.

## Installation

**Automatic (recommended):** The CLI is installed automatically when you run `easy-start.command`. No extra steps needed.

**Manual installation** (if you used `advanced-start.py` or skipped the prompt):

```bash
# Create symlink to /usr/local/bin
sudo ln -sf ~/.dailyos/dailyos /usr/local/bin/dailyos

# Or add to PATH in your shell profile
export PATH="$HOME/.dailyos:$PATH"
```

## Commands Overview

| Command | Purpose |
|---------|---------|
| `dailyos start` | Start the web UI server (auto-detects workspace) |
| `dailyos stop` | Stop the web UI server |
| `dailyos ui` | Show web UI status |
| `dailyos config` | Show/manage configuration |
| `dailyos version` | Show version info |
| `dailyos status` | Check for updates |
| `dailyos update` | Update to latest version |
| `dailyos doctor` | Check workspace health |
| `dailyos repair` | Fix broken installation |
| `dailyos eject <name>` | Customize a skill/command |
| `dailyos reset <name>` | Restore to core version |

---

## Server Management

### `dailyos start`

Start the web UI dashboard server.

```bash
dailyos start                    # Auto-detects workspace, opens browser
dailyos start --no-browser       # Start without opening browser
dailyos start -p 8080            # Use a different port
dailyos start --set-default      # Start and save as default workspace
dailyos -w ~/Work start          # Specify workspace explicitly
```

**Options:**
- `-p, --port PORT` — Port to run on (default: 5050)
- `--no-browser` — Don't open browser automatically
- `--set-default` — Save the workspace as your default

**Smart Workspace Detection:**

The CLI finds your workspace using this priority:
1. **Explicit flag**: `-w ~/path/to/workspace`
2. **Current directory**: If it has `.dailyos-version`
3. **Saved default**: From `~/.dailyos/config.json`
4. **Auto-scan**: Searches `~/Documents`, `~/workspace`, `~/projects`, `~/dev`

When auto-detected, you'll be prompted to save as default.

**Behavior:**
- Works from any directory (no need to `cd` first)
- Auto-installs npm dependencies if `node_modules/` is missing
- If server is already running, opens browser to existing instance
- Kills zombie Node processes if port is stuck
- Interactive picker if multiple workspaces found

### `dailyos stop`

Stop the web UI server.

```bash
dailyos stop                     # Stop server on default port
dailyos stop -p 8080             # Stop server on specific port
```

**Options:**
- `-p, --port PORT` — Port to stop (default: 5050)

### `dailyos ui`

Show the current status of the web UI server.

```bash
dailyos ui                       # Check default port
dailyos ui -p 8080               # Check specific port
```

**Output includes:**
- Running status (Running / Not running)
- URL if running
- Process ID (PID)

---

## Version Management

### `dailyos version`

Show version information for core and workspace.

```bash
dailyos version
```

**Output:**
```
DailyOS Version Information

  Core version:      v0.4.1
  Workspace version: v0.4.1
  Core location:     /Users/you/.dailyos
  Workspace:         /Users/you/Documents/productivity
```

### `dailyos status`

Check if updates are available and show workspace health summary.

```bash
dailyos status
```

**Output shows:**
- Current vs available version
- Changelog highlights
- Any detected issues

### `dailyos update`

Update the core installation and sync workspace.

```bash
dailyos update                   # Update with confirmation prompt
dailyos update -y                # Update without confirmation
```

**What happens:**
1. Pulls latest from core repository
2. Shows changelog of what's new
3. Updates workspace version marker
4. Verifies symlinks are correct

**Note:** Ejected (customized) skills are not updated automatically.

---

## Workspace Health

### `dailyos doctor`

Check workspace health and identify issues.

```bash
dailyos doctor
```

**Checks:**
- Core git repository status
- `_tools` and `_ui` symlinks
- Command symlinks (today, week, wrap, etc.)
- Skill symlinks
- Ejected components

If issues are found, offers to run `repair` automatically.

### `dailyos repair`

Fix broken symlinks and missing files.

```bash
dailyos repair
```

**Fixes:**
- Broken or missing `_tools` symlink
- Broken or missing `_ui` symlink
- Missing command symlinks
- Updates version marker

---

## Customization

### `dailyos eject <name>`

Eject a skill or command for customization.

```bash
dailyos eject today              # Customize the /today command
dailyos eject inbox   # Customize the inbox skill
```

**What happens:**
1. Copies the file from core to your workspace
2. Removes the symlink
3. Tracks it as "ejected" so updates skip it

**To see what can be ejected:**
- Commands: `today`, `week`, `wrap`, `month`, `quarter`, `email-scan`
- Skills: `inbox`, `editorial`, `strategy-consulting`

### `dailyos reset <name>`

Reset an ejected skill back to the core version.

```bash
dailyos reset today              # Restore /today to core version
```

**Warning:** This deletes your customizations! A backup is created with `.backup` extension.

---

## Configuration

### `dailyos config`

Show current configuration.

```bash
dailyos config
```

**Output:**
```
DailyOS Configuration

  Default workspace: ~/Documents/VIP

  Scan locations:
    - ~/Documents

  Scan depth: 2

  Known workspaces:
    - ~/Documents/VIP (today)

  Preferences:
    Auto-save default: yes
    Prompt on multiple: yes
```

### `dailyos config workspace [path]`

Get or set the default workspace.

```bash
dailyos config workspace                # Show current default
dailyos config workspace ~/Documents/VIP  # Set new default
```

When set, `dailyos start` will use this workspace automatically from any directory.

### `dailyos config scan`

Rescan for workspaces in configured locations.

```bash
dailyos config scan
```

Searches `~/Documents`, `~/workspace`, `~/projects`, and `~/dev` for directories containing `.dailyos-version`.

### `dailyos config reset`

Reset configuration to defaults.

```bash
dailyos config reset
```

**Warning:** This clears your default workspace and known workspaces list.

---

## Global Options

These options work with any command:

| Option | Description |
|--------|-------------|
| `-w, --workspace PATH` | Specify workspace path (default: current directory) |
| `-h, --help` | Show help for command |

**Examples:**
```bash
dailyos -w ~/Work doctor         # Check health of specific workspace
dailyos --workspace ~/Work start # Start UI for specific workspace
```

---

## Common Workflows

### Daily Use

```bash
# Morning: Start your dashboard
dailyos start

# In Claude Code
/today

# Evening: Close out
/wrap

# Stop dashboard when done
dailyos stop
```

### After Updating

```bash
# Check for updates
dailyos status

# Update if available
dailyos update

# Verify everything is healthy
dailyos doctor
```

### Troubleshooting

```bash
# Dashboard won't start?
dailyos stop                     # Clear any stuck processes
dailyos start                    # Try again

# Symlinks broken?
dailyos doctor                   # Diagnose
dailyos repair                   # Fix

# Something weird going on?
dailyos version                  # Check versions match
dailyos doctor                   # Full health check
```

---

## Environment

The CLI expects:
- **Python 3.8+** — For running the CLI itself
- **Node.js 18+** — For the web UI server (optional)
- **Core installation** — `~/.dailyos/` must exist

### Workspace Detection Priority

The CLI automatically finds your workspace by:
1. Using `-w/--workspace` if specified
2. Checking current working directory for `.dailyos-version`
3. Using saved default from `~/.dailyos/config.json`
4. Auto-scanning `~/Documents`, `~/workspace`, `~/projects`, `~/dev`

### Configuration File

User preferences are stored in `~/.dailyos/config.json`:
- Default workspace path
- Known workspaces with last-used timestamps
- Scan locations and depth
- Auto-save and prompt preferences

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (see output for details) |
| 130 | Interrupted by user (Ctrl+C) |
