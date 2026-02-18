# ADR-0003: Config in JSON file, no UI for MVP

**Date:** 2024-02
**Status:** Accepted

## Context

Config (workspace path, profile, schedule) needs to be editable. Building a settings UI takes time.

## Decision

Store config in `~/.dailyos/config.json`. Power users edit directly. Settings page displays but doesn't edit (MVP). Full settings UI is post-MVP.

## Consequences

- Reduces MVP scope
- Non-technical users can't configure without help
- Acceptable trade-off: MVP targets the builder (James), not end users
