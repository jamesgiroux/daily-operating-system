# ADR-0051: User-Configurable Metadata Settings

**Date:** 2026-02-08
**Status:** Superseded by [ADR-0079](0079-role-presets.md)

## Context

DailyOS displays metadata fields on entity pages (accounts, projects, people). Currently these fields are hardcoded: `health`, `lifecycle`, `ARR`, `NPS`, `CSM`, `champion`, `renewal_date` for accounts. The `ring` field (integer 1-4 CS tier) was removed in favor of `lifecycle` (generic string) because ring was CS-specific decoration that doesn't generalize.

This raises a broader question: should users be able to configure which metadata fields appear on entity pages, and what values those fields accept?

**Forces at play:**

1. Different roles need different fields. A CS team cares about health/ARR/NPS. A PM team cares about status/priority/sprint. A sales team cares about deal stage/pipeline/close date.
2. Entity-mode architecture (ADR-0046) already separates account-mode from project-mode. Kits (CS Kit, PM Kit) are designed to contribute fields and templates.
3. P4 (Opinionated Defaults, Escapable Constraints) says: works out of box, but overridable.
4. P1 (Zero-Guilt by Default) says: if metadata becomes stale, the system shouldn't punish the user.
5. The current hardcoded fields are CS-biased. Non-CS users get fields they don't need.

## Options Considered

### Option A: Hardcoded Kit Schemas

Each Kit (CS, PM, Sales) defines its own field schema. When you select a Kit, you get that Kit's fields. No customization.

**Pros:** Simple, predictable, no settings UI needed, fields are curated for the role.
**Cons:** Can't add fields the Kit doesn't anticipate. Users stuck with Kit author's choices. Mixing fields across Kits requires new Kit.

### Option B: Fully User-Defined Custom Fields

Settings page where users define arbitrary fields: name, type (text/number/select/date), allowed values, display location.

**Pros:** Maximum flexibility. Users get exactly what they need.
**Cons:** High implementation cost. Settings become a mini-schema-builder. Risk of complexity that violates P7 (Consumption Over Production). Field proliferation creates maintenance burden (violates P1). Hard to provide AI context for arbitrary fields.

### Option C: Hybrid — Universal Core + Kit Defaults + User Overrides

Three layers:
1. **Universal core fields** that every entity has: name, health, lifecycle, notes. Always present.
2. **Kit-contributed fields** that activate with entity mode: ARR/NPS/CSM/champion (CS Kit), priority/sprint/status (PM Kit). On by default when Kit is active.
3. **User overrides**: toggle Kit fields on/off, add a small number of custom text fields (max 5) for edge cases.

**Pros:** Opinionated defaults (P4) with escape hatch. Kit fields are curated. Custom fields cover edge cases without schema-builder complexity. AI enrichment can understand Kit fields deeply.
**Cons:** More complex than A, still limited compared to B. Custom fields lack the semantic richness of Kit fields.

## Decision

**Deferred.** This is a post-ship concern. For v1:

- `lifecycle` replaces `ring` as a generic string field (no constraints on values).
- Current hardcoded fields remain as-is.
- Kit architecture (ADR-0046) provides the future hook for field customization.

When we revisit, **Option C is the recommended approach** — it aligns with P4 (opinionated defaults, escapable constraints) and avoids the schema-builder trap of Option B.

## Consequences

### Immediate (v1)
- `lifecycle` field is free-text, not constrained to a fixed set of values.
- Non-CS users see CS-biased fields (ARR, NPS) that may not be relevant. Acceptable for v1 since CS-first is the stated focus (ADR-0038).

### Future (post-ship)
- Kit implementation (I40, I27 umbrella) will need a field schema mechanism.
- Settings UI will need a "field visibility" section per entity type.
- AI enrichment prompts will need to be field-aware (know which fields exist and what they mean).
- Migration path: existing hardcoded fields become CS Kit defaults.
