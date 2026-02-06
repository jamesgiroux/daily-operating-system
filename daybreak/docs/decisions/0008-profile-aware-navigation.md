# ADR-0008: Profile-aware navigation with entity pattern

**Date:** 2026-02
**Status:** Accepted

## Context

Two user profiles exist: Customer Success (CSM) and General. Each has a primary entity: CSM works with Accounts, General works with Projects. The navigation should serve both without making either feel stripped-down.

## Decision

Each profile has a primary entity: CS = Accounts (`2-areas/accounts/`), GA = Projects (`1-projects/`). Same sidebar structure, same portfolio page component, different data shape. Neither profile is "stripped down."

## Consequences

- Both profiles feel complete and intentional
- The entity page component must be polymorphic (accounts or projects)
- Adding a new profile means defining its entity type and data shape
