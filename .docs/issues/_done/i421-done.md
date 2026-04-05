# I421 — Connector rename

**Status:** Open
**Priority:** P2
**Version:** 0.13.9
**Area:** Frontend / UX

## Summary

Pure cosmetic rename throughout the Settings UI: "Connections" → "Connectors". The current UI already says "Connections" (a previous rename from "Integrations" is complete). This final rename aligns UI terminology with the connector signal contract — these are purposeful inbound data sources, not generic connections. Zero logic changes, zero backend impact. Can be merged first as the lowest-risk issue.

## Acceptance Criteria

1. Every user-facing instance of "Connections" is replaced with "Connectors" throughout the app. Verify: `grep -r "Connections" src/ --include="*.tsx" --include="*.ts"` returns only code comments, non-visible strings, or the legacy tab map redirect (not rendered labels, nav items, or page titles).

2. The Settings nav item or section label reads "Connectors." The page title reads "Connectors." The section headers for each connector card read their connector name.

3. `grep -r "connection\|Connection" src/ --include="*.tsx" | grep -v "//\|test\|spec"` — review and replace any remaining user-facing instances. Component file names (`*Connection*.tsx`) may keep their names if the rename is purely cosmetic.

4. Backend code (`src-tauri/src/`) uses "connector" in log messages and command names where appropriate — not a hard requirement, but new code added in this version uses "connector" terminology.

## Dependencies

None. This is a purely cosmetic change with no backend dependencies.

## Notes / Rationale

**Key file:** `src/components/settings/connections/registry.ts` — contains the connector registry and display names.

The rename aligns with the connector signal contract established in v0.13.9. These are not generic "integrations"—they are connectors: purposeful inbound data sources that feed the signal system and entity intelligence. The UI language should match the architecture.
