# ADR-0045: Profile-Agnostic Entity Abstraction

**Date:** 2026-02-07
**Status:** accepted

## Context

DailyOS core is hardcoded to CS "accounts" as the only tracked entity. ADR-0043 established that meeting intelligence (prep, captures, action association) is core, not extension-specific. But the DB schema, hooks, and types all assume `account_id` -- a PM profile would need "projects", a manager profile would need "people".

Core behaviors that need to work across profiles:
- Last-contact tracking (when did we last interact with this entity?)
- Capture association (wins/risks linked to an entity)
- Action linking (actions belong to an entity)

The CS `accounts` table has domain-specific fields (ring, ARR, health, CSM, champion) that don't belong in a universal abstraction.

## Decision

Introduce an `entities` table as a profile-agnostic layer alongside the existing `accounts` table:

- **`EntityType` enum:** Account (CS), Project (PM), Person (Manager), Other.
- **`entities` table:** id, name, entity_type, tracker_path, updated_at. No domain-specific fields.
- **Bridge pattern:** `upsert_account()` automatically mirrors to `entities`. A backfill migration populates entities from existing accounts on DB open.
- **Generalized hook:** `entity_intelligence()` replaces `cs_account_intelligence()`. Runs for all profiles (core behavior per ADR-0043). For Account entities, also touches the CS-specific `accounts` table.
- **No renames:** Existing `account_id` columns on actions, meetings_history, captures stay as-is. Full foreign key migration is a separate effort (I27).

## Consequences

**Easier:**
- Adding PM or Manager profiles: just use `EntityType::Project` / `EntityType::Person`, core behaviors work.
- Post-meeting captures and last-contact tracking work for any profile out of the box.

**Harder:**
- Two tables to keep in sync for CS accounts (mitigated by automatic bridge in `upsert_account`).
- Foreign keys still reference `account_id` -- full migration deferred to I27.

**Trade-offs:**
- Chose bridge pattern over rename to avoid touching every query, every frontend reference, and every Python script in one commit.
- `entity_type` defaults to `EntityType::default_for_profile()` rather than storing profile on the entity, since an entity's type is inherent, not session-dependent.
