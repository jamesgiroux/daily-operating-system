# Archive: DailyOS

This directory contains the original **DailyOS** CLI proof-of-concept.

**Status:** Archived for reference. Active development has moved to `/daybreak/`.

---

## What Was DailyOS?

DailyOS was a CLI-based productivity system built on Claude Code. It validated the core concepts that became Daybreak:

- Three-phase workflow pattern (Prepare → Enrich → Deliver)
- `/today`, `/wrap`, `/inbox`, `/week` commands
- PARA file organization
- Google API integration
- Skills and agents architecture

---

## Why Archived?

Daybreak is the native app destination. DailyOS served its purpose:

1. **Validated the workflow** — Three-phase pattern works
2. **Tested integrations** — Google Calendar, Gmail, Sheets
3. **Refined the primitives** — Commands, skills, agents
4. **Identified the gap** — Users need a GUI, not a CLI

The working scripts (`prepare_today.py`, `deliver_today.py`, etc.) will be called by Daybreak's scheduler. The CLI wrapper is no longer needed.

---

## Contents

```
dailyos/
├── src/                # Python setup wizard and CLI
├── templates/          # Commands, skills, agents, scripts
├── server/             # Web dashboard (Express)
├── docs/               # Original documentation site
├── tests/              # Python tests
└── config/             # Default configurations
```

---

## Using Archive Content

If you need to reference DailyOS code:

```bash
# Python scripts (still used by Daybreak)
cat _archive/dailyos/templates/scripts/prepare_today.py

# Command templates
cat _archive/dailyos/templates/commands/today.md

# Agent definitions
cat _archive/dailyos/templates/agents/
```

---

*This archive will be removed after Daybreak reaches stable release.*
