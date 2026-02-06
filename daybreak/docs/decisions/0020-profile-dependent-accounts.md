# ADR-0020: Profile-dependent accounts (CSM plugin)

**Date:** 2026-02
**Status:** Accepted

## Context

Accounts (customer tracking, dashboards, health monitoring) are core to the CSM workflow but irrelevant for General users. Making accounts universal adds noise for non-CSM users.

## Decision

Accounts are a "plugin" activated by the CSM profile. Profile selection configures workspace structure, meeting classification rules, and prep templates. General profile has Projects as its primary entity instead.

## Consequences

- Each profile feels purpose-built, not stripped-down
- The "entity pattern" (ADR-0008) makes this extensible to future profiles
- Classification rules must be profile-aware (CSM: attendee domain → account mapping; General: no account mapping)
