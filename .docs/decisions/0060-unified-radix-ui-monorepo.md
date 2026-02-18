# ADR-0060: Unified Radix UI Monorepo — No Standalone Packages

**Status:** Accepted
**Date:** 2026-02-09
**Deciders:** James, Claude

## Context

DailyOS uses shadcn/ui as its component foundation. shadcn/ui components are built on Radix UI primitives.

Radix UI migrated from standalone packages (`@radix-ui/react-dropdown-menu`, `@radix-ui/react-popover`, etc.) to a unified monorepo package (`radix-ui`). Having both installed creates duplicate React contexts — a component's Trigger registers with one context while its Portal/Content registers with the other, causing interactive elements (dropdowns, popovers, dialogs) to silently fail.

This exact failure shipped in v0.7.0: the theme toggle dropdown was completely non-functional because `dropdown-menu.tsx` imported from the old standalone package while every other component imported from the monorepo.

## Decision

1. **All Radix UI imports MUST use the `radix-ui` monorepo package.** No standalone `@radix-ui/*` packages in `package.json`.
2. **All UI primitives MUST come from shadcn/ui.** When adding a new shadcn component via `npx shadcn@latest add`, verify the generated code imports from `radix-ui` (not `@radix-ui/*`). If it doesn't, update the import before committing.
3. **Audit on every dependency change.** Any `pnpm add` that introduces a `@radix-ui/*` standalone package must be caught and corrected.

## Consequences

- Single Radix UI context tree — no silent portal/trigger disconnects
- Smaller `node_modules` (one package instead of many)
- Must verify shadcn CLI output matches this convention (older shadcn versions may still scaffold standalone imports)
