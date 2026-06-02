# v1.5.0 List-Surfaces PR A Proof Bundle - DOS-826 Shared Entity Selection

**Date:** 2026-06-01  
**Branch:** `codex/v1.5.0-list-surfaces`  
**Base:** `public/dev` at `f4caeb05`  
**Scope:** frontend-only shared selection affordance for active Accounts, Projects, and People list rows.

## Acceptance Evidence

- Active Accounts, Projects, and People rows now receive semantic checkbox selection props.
- Archived rows do not receive selection props.
- Shared `useEntityListSelection` handles individual toggle, select visible, clear, stale-id pruning, and visible-order shift ranges.
- `EntityRow` renders checkbox, navigation link/anchor, and row controls as sibling focus targets.
- Accounts/Projects expand controls moved out of link-wrapped row content.
- Selection bars clear in archived mode.
- People relationship tabs now filter from the active people list client-side so hidden-but-active selections can persist across tab changes.
- No backend commands, services, migrations, claims, source attribution, filesystem behavior, or signal policy changed.

## Verification

### L2 diff review

Artifact:

```text
.docs/reviews/v1.5.0-list-surfaces-pr-a-l2-2026-06-01.md
```

Result: APPROVE, no findings.

### Focused frontend tests

Command:

```bash
pnpm test EntityRow EntityListShell useEntityListSelection ProjectsPage.selection PeoplePage.selection
```

Result:

```text
Test Files  5 passed (5)
Tests       10 passed (10)
```

### TypeScript

Command:

```bash
pnpm tsc --noEmit
```

Result: pass.

### Full frontend tests

Command:

```bash
pnpm test
```

Result:

```text
Test Files  53 passed (53)
Tests       284 passed (284)
```

### Production frontend build

Command:

```bash
pnpm build
```

Result: pass. Vite emitted its existing large-chunk warning.

### Targeted lint

Commands:

```bash
pnpm exec eslint src/components/entity/EntityListShell.tsx src/components/entity/EntityRow.tsx src/components/entity/useEntityListSelection.ts src/pages/AccountsPage.tsx src/pages/ProjectsPage.tsx src/pages/PeoplePage.tsx src/components/entity/EntityListShell.test.tsx src/components/entity/EntityRow.test.tsx src/components/entity/useEntityListSelection.test.tsx src/pages/ProjectsPage.selection.test.tsx src/pages/PeoplePage.selection.test.tsx --max-warnings 9999
pnpm exec stylelint "src/components/entity/EntityListShell.module.css" "src/components/entity/EntityRow.module.css"
```

Result: pass. ESLint reported one existing `AccountsPage` warning for the unrelated `discoveryEnabled` folio-action memo dependency; the PR-introduced archived-mode selection dependency warnings were fixed before final verification.

### Diff check

Command:

```bash
git diff --check public/dev
```

Result: pass.

## Rust Gates

Not run for PR A. The diff is frontend-only and does not touch Tauri, services, migrations, commands, or Rust tests.

## Remaining Gates

- Browser/L4 surface proof before PR if local Tauri/browser environment is available without entering the replica/production database path currently under separate investigation.

## L4 Attempt - 2026-06-02

Result: blocked by environment, not by a PR A code finding.

- `DAILYOS_DB_MODE=replica RUST_LOG=warn pnpm tauri dev` launched Vite and compiled the PR A backend, but startup entered database recovery because the shared replica DB is already at schema version 274 from the later PR B stack while PR A supports schema version 273.
- No recovery, downgrade, or production/live DB action was attempted.
- `DAILYOS_DB_MODE=mock RUST_LOG=warn pnpm tauri dev` launched against the fixture DB path and completed startup migrations for `dailyos-dev.db`; only expected mock/dev warnings were observed.
- Native surface inspection was blocked because the macOS session was at the lock screen, and the agent must not unlock the machine or enter credentials.

Draft status remains appropriate until native Tauri L4 proof can be captured from an unlocked session or after PR A is evaluated against a compatible branch/database state.
