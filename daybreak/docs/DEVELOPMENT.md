# Development Guide

> How to build and run Daybreak locally.

---

## Prerequisites

### Required

| Tool | Version | Purpose | Install |
|------|---------|---------|---------|
| **Rust** | 1.70+ | Tauri backend | [rustup.rs](https://rustup.rs) |
| **Node.js** | 18+ | Frontend build | [nodejs.org](https://nodejs.org) |
| **pnpm** | 8+ | Package manager | `npm install -g pnpm` |
| **Tauri CLI** | 2.x | Build tooling | `cargo install tauri-cli` |

### macOS Additional

```bash
# Xcode command line tools
xcode-select --install
```

### Recommended

| Tool | Purpose |
|------|---------|
| **Claude Code** | AI enrichment (required for full functionality) |
| **DailyOS workspace** | Test data and scripts |

---

## Project Setup

### 1. Clone and Navigate

```bash
cd /path/to/daily-operating-system-daybreak
cd daybreak
```

### 2. Initialize Tauri Project

```bash
# Create Tauri app structure (first time only)
pnpm create tauri-app --template react-ts .

# Or if directories exist:
pnpm init
pnpm add -D @tauri-apps/cli @tauri-apps/api
pnpm add react react-dom
pnpm add -D @types/react @types/react-dom typescript vite @vitejs/plugin-react
```

### 3. Install Dependencies

```bash
# Frontend dependencies
pnpm install

# Rust dependencies (in src-tauri/)
cd src-tauri
cargo build
cd ..
```

### 4. Configure Workspace

Create `~/.daybreak/config.json`:

```json
{
  "version": 1,
  "workspace": {
    "path": "/path/to/your/dailyos/workspace",
    "validated": false
  },
  "schedules": {
    "today": {
      "enabled": true,
      "cron": "0 8 * * 1-5",
      "timezone": "America/New_York"
    }
  },
  "notifications": {
    "enabled": true
  }
}
```

---

## Development Workflow

### Run in Development Mode

```bash
# Start Tauri dev server (hot reload)
pnpm tauri dev
```

This launches:
- Vite dev server for frontend (hot reload)
- Rust backend with debug symbols
- App window with DevTools

### Build for Production

```bash
# Build optimized binary
pnpm tauri build

# Output in src-tauri/target/release/bundle/
```

### Run Tests

```bash
# Frontend tests
pnpm test

# Rust tests
cd src-tauri
cargo test
```

---

## Project Structure

```
daybreak/
├── docs/                    # Documentation
│   ├── PHILOSOPHY.md
│   ├── PRINCIPLES.md
│   ├── VISION.md
│   ├── JTBD.md
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── SKILLS.md
│   ├── MVP.md
│   ├── ROADMAP.md
│   └── DEVELOPMENT.md       # (this file)
│
├── src/                     # Frontend (React + TypeScript)
│   ├── App.tsx              # Root component
│   ├── main.tsx             # Entry point
│   ├── components/          # UI components
│   │   ├── Dashboard.tsx
│   │   ├── MeetingCard.tsx
│   │   ├── ActionList.tsx
│   │   └── TrayMenu.tsx
│   ├── hooks/               # Custom hooks
│   │   ├── useTauri.ts      # IPC bridge
│   │   └── useWorkflow.ts   # Workflow status
│   ├── lib/                 # Utilities
│   └── styles/              # CSS/Tailwind
│
├── src-tauri/               # Backend (Rust)
│   ├── src/
│   │   ├── main.rs          # Entry point
│   │   ├── commands.rs      # Tauri IPC commands
│   │   ├── scheduler.rs     # Job scheduling
│   │   ├── executor.rs      # Workflow execution
│   │   ├── pty.rs           # PTY management
│   │   ├── state.rs         # State persistence
│   │   └── workflow/        # Workflow implementations
│   │       ├── mod.rs
│   │       └── today.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json      # Tauri configuration
│   └── icons/               # App icons
│
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
└── README.md
```

---

## Key Files to Create (Phase 1)

### Frontend

| File | Purpose |
|------|---------|
| `src/App.tsx` | Root component, routing |
| `src/components/Dashboard.tsx` | Main dashboard layout |
| `src/components/MeetingCard.tsx` | Meeting prep card component |
| `src/components/ActionList.tsx` | Action items panel |
| `src/hooks/useTauri.ts` | IPC command wrappers |

### Backend

| File | Purpose |
|------|---------|
| `src-tauri/src/main.rs` | App entry, window/tray setup |
| `src-tauri/src/commands.rs` | IPC command handlers |
| `src-tauri/src/scheduler.rs` | Cron-like job scheduling |
| `src-tauri/src/executor.rs` | Three-phase workflow runner |
| `src-tauri/src/pty.rs` | Claude Code subprocess |
| `src-tauri/src/state.rs` | Config and status persistence |

---

## Tauri IPC Commands

Commands the frontend can call:

```typescript
// src/hooks/useTauri.ts
import { invoke } from '@tauri-apps/api/core';

// Get today's overview
const overview = await invoke<TodayOverview>('get_today_overview');

// Get workflow status
const status = await invoke<WorkflowStatus>('get_workflow_status', { workflow: 'today' });

// Manually trigger workflow
const executionId = await invoke<string>('run_workflow', { workflow: 'today' });

// Get execution history
const history = await invoke<ExecutionRecord[]>('get_execution_history', { limit: 10 });
```

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `DAYBREAK_CONFIG` | Config file path | `~/.daybreak/config.json` |
| `DAYBREAK_LOG_LEVEL` | Log verbosity | `info` |
| `DAYBREAK_DEV` | Development mode flag | `false` |

---

## Debugging

### Frontend (React)

- DevTools: `Cmd+Option+I` in app window
- React DevTools extension works in Tauri

### Backend (Rust)

```bash
# Run with debug logging
RUST_LOG=debug pnpm tauri dev

# Attach debugger (VS Code)
# Use "Tauri Development Debug" launch config
```

### Claude Code Execution

```bash
# Test Claude Code manually
claude --workspace /path/to/workspace --print "echo 'test'"

# Check PTY spawn logs
tail -f ~/.daybreak/logs/pty.log
```

---

## Common Issues

### "Claude Code not found"

```bash
# Verify Claude Code is installed
which claude

# Add to PATH if needed
export PATH="$PATH:/path/to/claude"
```

### "Workspace not configured"

```bash
# Create config file
mkdir -p ~/.daybreak
cat > ~/.daybreak/config.json << 'EOF'
{
  "version": 1,
  "workspace": {
    "path": "/path/to/workspace"
  }
}
EOF
```

### "Google API error"

```bash
# Re-run Google setup in workspace
cd /path/to/workspace
dailyos google-setup
```

### Tauri build fails on macOS

```bash
# Ensure Xcode tools installed
xcode-select --install

# Clear Rust cache if needed
cargo clean
```

---

## Code Style

### TypeScript/React

- Functional components with hooks
- TypeScript strict mode
- Tailwind for styling
- No `any` types

### Rust

- Follow Rust API guidelines
- Use `thiserror` for error types
- Async with Tokio
- Document public APIs

### Commits

```
type(scope): description

feat(dashboard): add meeting card component
fix(scheduler): handle timezone edge case
docs(readme): update setup instructions
```

---

## Next Steps

1. **Scaffold Tauri project** — Run `pnpm create tauri-app`
2. **Create basic window** — Verify Tauri runs
3. **Add system tray** — Basic tray icon and menu
4. **Implement scheduler** — Time-based job execution
5. **Build dashboard shell** — Empty dashboard that renders

See [ROADMAP.md](ROADMAP.md) for full milestone breakdown.

---

*This guide will evolve as the project develops. Update when setup changes.*
