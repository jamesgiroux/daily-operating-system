# ADR-0018: Hybrid storage: Markdown + SQLite

**Date:** 2026-02
**Status:** Accepted

## Context

User content (meeting preps, notes, account info) should be portable and human-readable. System state (action tracking, processing history, settings cache) needs fast queries.

## Decision

User content stays in markdown files (portable, human-readable, user-owned). System state lives in SQLite (performant, queryable). SQLite is a disposable cache — it can be rebuilt from files. Markdown is the source of truth.

## Consequences

- Users own their data as plain files (Principle 5: Local-First)
- App gets fast queries without parsing markdown every time
- If SQLite is deleted, system state is lost but can be regenerated
- Two data stores means keeping them in sync — the briefing workflow is the synchronization point
