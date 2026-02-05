# Development Guide

> How to build and run DailyOS locally.

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
      "cron": "0 6 * * 1-5",
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
├── docs/                        # See CLAUDE.md for doc index
│
├── src/                         # Frontend (React + TypeScript)
│   ├── App.tsx                  # Root component
│   ├── main.tsx                 # Entry point
│   ├── router.tsx               # TanStack Router ✅
│   ├── components/
│   │   ├── dashboard/           # Dashboard views ✅
│   │   │   ├── Header.tsx
│   │   │   ├── MeetingCard.tsx
│   │   │   ├── RunNowButton.tsx
│   │   │   └── StatusIndicator.tsx
│   │   ├── layout/              # AppSidebar, CommandMenu ✅
│   │   └── ui/                  # shadcn/ui components
│   ├── hooks/
│   │   ├── useDashboardData.ts  # ✅
│   │   └── useWorkflow.ts       # ✅
│   ├── pages/                   # Route pages ✅
│   ├── lib/                     # Utilities, types
│   └── types/
│       └── index.ts             # TypeScript interfaces ✅
│
├── src-tauri/                   # Backend (Rust)
│   ├── src/
│   │   ├── main.rs              # Entry point ✅
│   │   ├── lib.rs               # Library root, Tauri setup ✅
│   │   ├── commands.rs          # Tauri IPC commands ✅
│   │   ├── json_loader.rs       # JSON data loading ✅
│   │   ├── scheduler.rs         # Cron-like scheduling ✅
│   │   ├── pty.rs               # PTY/Claude Code ✅
│   │   ├── state.rs             # Config & status ✅
│   │   ├── notification.rs      # Native notifications ✅
│   │   ├── error.rs             # Error types ✅
│   │   ├── types.rs             # Shared types ✅
│   │   ├── parser.rs            # Markdown parsing ✅
│   │   └── workflow/
│   │       ├── mod.rs           # ✅
│   │       ├── today.rs         # ✅
│   │       ├── archive.rs       # ✅
│   │       ├── inbox.rs         # (Phase 2)
│   │       └── week.rs          # (Phase 3)
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

✅ = Implemented | (Phase N) = Planned

---

## Tauri IPC Commands

Commands the frontend can call:

```typescript
// src/hooks/useTauri.ts
import { invoke } from '@tauri-apps/api/core';

// Get dashboard data (loads _today/data/*.json)
const data = await invoke<DashboardResult>('get_dashboard_data');

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
| `DAILYOS_CONFIG` | Config file path | `~/.daybreak/config.json` |
| `DAILYOS_LOG_LEVEL` | Log verbosity | `info` |
| `DAILYOS_DEV` | Development mode flag | `false` |

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

## Current Status

Phase 1 (MVP) is functionally complete. See `IMPLEMENTATION.md` for phase details and `ROADMAP.md` for milestones.

---

*Last updated: 2026-02-05*
