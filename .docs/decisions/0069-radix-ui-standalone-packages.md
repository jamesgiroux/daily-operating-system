# ADR-0069: Radix UI Standalone Packages — Supersedes ADR-0060

**Status:** Accepted
**Date:** 2026-02-12
**Deciders:** James, Claude
**Supersedes:** ADR-0060

## Context

ADR-0060 mandated using the `radix-ui` monorepo package exclusively. During 0.7.1 fast-follow (I157), migrating all 14 shadcn primitives to the monorepo proved unstable — both full-rewrite and const-alias approaches caused white-screen failures.

The root problem ADR-0060 solved was dual-install: having both `radix-ui` (monorepo) and `@radix-ui/react-*` (standalone) creates duplicate React contexts, causing portal/trigger disconnects.

## Decision

1. **All Radix UI imports use explicit standalone `@radix-ui/react-*` packages.** The `radix-ui` monorepo package is removed from `package.json`.
2. **No dual-install.** Only one source of Radix UI primitives may exist. The audit-on-every-dependency-change rule from ADR-0060 still applies — just checking for `radix-ui` monorepo instead of standalone packages.
3. **shadcn CLI output must match.** When adding components via `npx shadcn@latest add`, verify generated code imports from `@radix-ui/react-*` (not `radix-ui`). Update if needed.

## Consequences

- Same single-context guarantee as ADR-0060, achieved from the opposite direction
- More packages in `package.json` but each is independently versionable
- Aligns with the shadcn/ui component scaffolding defaults
