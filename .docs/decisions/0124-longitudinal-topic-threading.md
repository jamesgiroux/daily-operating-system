# ADR-0124: Longitudinal Topic Threading — Substrate Allowance and Deferred Decision

**Status:** Accepted (substrate allowance for v1.4.0 spine; design decision deferred to v1.4.2)
**Date:** 2026-04-24
**Target:** v1.4.0 substrate (schema + envelope allowance) / v1.4.2 (decision + implementation)
**Extends:** [ADR-0105](0105-provenance-as-first-class-output.md) (SubjectAttribution amendment), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (claims)
**Consumed by:** v1.4.3 briefing prep, v1.5.x recommendations layer

## Context

DailyOS today produces strong cross-thread synthesis on **current** signals — recognizing that two concurrent renewal cycles + cross-portfolio role + recent pipeline-risk signal compose into a leverage-relevant moment, for example. It does not retrieve **the longitudinal narrative thread** the user has been building over many prior sessions on the same theme — months of 1:1 reasoning about consolidation strategy that gives the moment its weight.

The substrate as drafted has:

- Per-`(entity_id, claim_type, field_path)` claim history ([ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) R1.6).
- Multi-subject claims spanning Account / Project / Person / Meeting ([ADR-0105](0105-provenance-as-first-class-output.md) SubjectAttribution amendment).
- Temporal primitives `EngagementCurve` and `RoleProgression` ([ADR-0109](0109-temporal-primitives-in-the-entity-graph.md), DOS-215).

None of these models a **longitudinal topic / theme / initiative** — a thread the user has been actively working over time that spans entities and accumulates reasoning across many sessions. Without one, surfaces miss the historical weight that makes a current signal meaningful.

Two solution shapes are plausible (detailed in §3 below). The decision between them needs design work and pilot data that is not v1.4.0 spine scope. But choosing later is expensive if the substrate has no place to put a thread reference. This ADR resolves that mismatch: ship the substrate allowance now, defer the meaning to v1.4.2.

## Decision

### 1. v1.4.0 substrate allowance (spine)

Three additive changes, all forward-compatible, no behavioral impact:

```rust
pub struct ThreadId(pub Uuid);          // Opaque newtype; v1.4.2 owns assignment semantics
```

```sql
ALTER TABLE intelligence_claims ADD COLUMN thread_id TEXT NULL;
```

```rust
pub struct Provenance {
    // ...existing fields per ADR-0105 + amendment...
    pub thread_ids: Vec<ThreadId>,      // NEW — defaults to empty Vec
}
```

Constraints in spine:

- `dedup_key` formula does NOT include `thread_id`. Same-content claims that participate in different threads remain the same claim; thread membership is orthogonal to claim identity.
- No retrieval logic. No UI. No thread creation. The substrate ships the field; v1.4.2 ships the meaning.
- Provenance envelope `provenance_schema_version` stays at `1` — the new field is additive, optional, default-empty (forward-compatible per ADR-0105 §1).

### 2. v1.4.2 design decision (full spec, not deferred indefinitely)

Choose between three options based on early v1.4.0 pilot data:

**Option A — `Theme` entity type.** First-class entity in the Account/Project/Person/Meeting/Theme set. Themes have name, description, owner, lifecycle state (active / dormant / closed). `SubjectRef` gains a `Theme { id }` variant. UI: theme detail page; "join thread" affordance on claims.

- Pro: discoverable, user-creatable, shareable, namable.
- Con: requires entity-creation UX; entities are heavier than thread groupings.

**Option B — `thread_id` retrieval pattern.** Threads are derived: Transform abilities identify longitudinal patterns across claims and assign the same `thread_id` to them. No theme entity; threads are claim-graph affordances surfaced in entity sidebars.

- Pro: lighter; threads emerge from data rather than user creation.
- Con: less discoverable; threads can fragment if the assignment heuristic drifts.

**Option C — hybrid.** Auto-generated `thread_id`s (Option B) AND user-named themes (Option A) populate the same field; themes are an optional layer on top of the thread substrate.

- Pro: maximum flexibility; user can promote auto-discovered threads to named themes.
- Con: most expensive; two creation paths to keep coherent.

**Decision criteria** for v1.4.2:

- If users want to create / name / share threads explicitly → Option A.
- If threads are useful primarily as system-discovered context → Option B.
- If both behaviors emerge in the v1.4.0 pilot → Option C.

### 3. Many-to-many extension path

The v1.4.0 single-`thread_id` column is sufficient for the substrate allowance. If v1.4.2 chooses Option C or discovers that claims legitimately participate in multiple threads (which the Provenance envelope's `thread_ids: Vec<ThreadId>` already supports), v1.4.2 adds:

```sql
CREATE TABLE claim_threads (
  claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
  thread_id TEXT NOT NULL,
  joined_at TIMESTAMP NOT NULL,
  PRIMARY KEY (claim_id, thread_id)
);
```

The v1.4.0 column becomes the "primary thread" denormalization for query speed; the child table carries the full set. Migration is additive; no rewriting of v1.4.0 rows is needed.

### 4. v1.4.3 surface consumption (separate spec)

Whatever v1.4.2 decides, surfaces consume threads via:

- `prepare_meeting` retrieves open threads on the meeting's subjects and includes them in the brief.
- Account / project / person detail pages show "Active threads" sidebar listing thread participation.
- Briefing surfaces longitudinal callouts: "you've been working this for N months; here's the latest signal."

These are scoped in the v1.4.3 surface-contract issue, not this ADR.

### 5. v1.5.x recommendations layer integration

Per project memory `recommendations_layer_vision.md`, post-v1.4.x recommendation claims attach to threads when applicable. A recommendation on the consolidation thread can reference 8 months of prior reasoning rather than the latest email alone. No further substrate ADR needed; the allowance from §1 is the foundation.

## Why ship the allowance in spine

- Adding the column + envelope field in v1.4.0 is ~half a day of work.
- Adding it in v1.4.2 requires a migration on `intelligence_claims` (a load-bearing table by then), backfill considerations, and version-skewed Provenance envelopes for any output already serialized between v1.4.0 spine and v1.4.2.
- The `ThreadId` type + nullable column have **zero behavioral impact on the spine** — every claim ships with `thread_id = NULL` until v1.4.2 decides assignment logic.
- Cost of doing it now: trivial. Cost of doing it later: real.

## Non-goals for v1.4.0 spine

- Thread creation, naming, or assignment logic.
- Retrieval of "all claims in this thread."
- UI surfaces consuming threads.
- Decision between Options A / B / C above.
- Migration to many-to-many `claim_threads` child table.
- Thread merge / split semantics.

## Consequences

### Positive

- v1.4.2 longitudinal topic work doesn't require a schema migration on `intelligence_claims`.
- Provenance envelopes shipped in v1.4.0 carry an empty `thread_ids` field; future consumers see zero-len rather than missing-field.
- The decision between Theme entity and thread_id retrieval pattern can be made with v1.4.0 pilot data on how often legitimate longitudinal patterns emerge.
- `RecommendationClaim` (v1.5.x) can reference threads on the same substrate without further allowance.

### Negative

- One nullable column ships unused in v1.4.0. Acceptable.
- Two opaque-id types (`SubjectRef` and `ThreadId`) increase substrate cognitive load slightly. Mitigated by clear naming and the fact that abilities ignore `thread_id` until v1.4.2.

### Neutral

- No changes to ability authoring; abilities ignore `thread_id` until v1.4.2.
- No changes to existing dedup, propose/commit, or trust scoring behavior.

## References

- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `SubjectRef::Multi` is what enables a single claim to span the subjects a thread typically does.
- [ADR-0113: Human and Agent Analysis as First-Class Claim Sources](0113-human-and-agent-analysis-as-first-class-claim-sources.md) — `intelligence_claims` row schema; this ADR adds one nullable column.
- [ADR-0123: Typed Claim Feedback Semantics](0123-typed-claim-feedback-semantics.md) — `WrongSubject` feedback on a thread-attached claim corrects the subject without affecting thread membership.
- Memory: `project_longitudinal_topic_gap.md` — sourcing of the gap (user observation 2026-04-24).
- Memory: `project_recommendations_layer_vision.md` — downstream consumer in v1.5.x.
