# Implementation Plan: DOS-411

## Revision history
- v1 (2026-05-06) — initial L0 draft. Pulled into W6 from v1.4.1 follow-up backlog after no-deferrals call on the wave; original ticket filed as a W5 cycle-3 follow-up at commit `bf22c68f`.

## 1. Contract restated

DOS-411 is the Tauri entity-context write cutover that finishes what W5 cycle-2 Track E started and cycle-3 Track H rolled back. The MCP/Agent path already reads claims via the `get_entity_context` ability (W5-A acceptance is preserved). The Tauri UI command path still reads and writes the legacy `entity_context_entries` SQL table because cycle-2's read-only cutover caused a create→read divergence that cycle-3 had to revert: a user saved a note, the refreshed read went to claims, and the note appeared lost.

The cycle-3 rollback was the right call at the time (W5's pilots scope did not include a new claim type plus migration). It is not the right end state. v1.4.0 ships when:

- The Tauri UI is on the same claim-backed substrate as MCP/Agent.
- A new claim type covers user-created generic context notes with full Intelligence Loop semantics.
- A backfill migration moves every existing `entity_context_entries` row into `intelligence_claims` in active+surfacing state.
- Update / delete semantics are claim-lifecycle correct (supersession + withdrawal), with UX behavior that does not surprise users.
- The legacy `entity_context_entries` table is dropped or write-frozen.

The live regression at `src-tauri/tests/w5_l2_track_e_tauri_entity_context_test.rs` already guards against silent re-cut-over of the read alone; this plan must keep that regression green and extend it to the full create→edit→delete→read round-trip on the new substrate.

## 2. Approach

### 2.1 New claim type

Define `claim_type = 'user_note'` (working name; final naming is part of the design call) covering free-form user-created context notes attached to an account, person, or project. The five-question Intelligence Loop check (per CLAUDE.md) must be answered before code lands:

1. **Claim model.** `subject_ref` is the account/person/project the note is about. Body is the note text. `actor = user`, `data_source = manual`, default `sensitivity = internal`, default `temporal_scope = state`. The `dedup_key` policy needs a call: notes are not naturally deduped by content (two distinct notes about the same account are common), so `dedup_key = sha256(subject_ref || actor || created_at_ms)` keeps every note distinct while still being deterministic.

2. **Provenance + trust.** `source_asof = created_at` of the user action; source attribution is the authenticated workspace user. The note's claim is unambiguously direct user attribution, so trust factors that don't apply to user-attributed claims must be excluded from compilation for this claim type. Trust band starts at `likely_current` for fresh notes and decays per existing freshness factor.

3. **Signals + invalidation.** Notes do not invalidate other claims by default — they are user observations, not authoritative facts about external state. They DO emit a `user_note_added` signal that the relevant entity surface can use to re-render. They do NOT participate in cross-entity coherence scoring (no foreign domains in user-typed text, by construction).

4. **Runtime + surfaces.** Consumed by `read_entity_context_claim_entries` and the `get_entity_context` ability — both already exist post-W5. Tauri commands and the React UI consume them through the bridge envelope. MCP agents see them subject to the cycle-6 Public/Internal sensitivity gate (default Internal renders for Agent; user-marked Confidential does not).

5. **Feedback loop.** Edit-via-supersession is the natural feedback path: a user typo correction supersedes the original; the trust pipeline observes the supersession event without paging it as a "wrong claim" feedback. Withdrawal (delete) records a user-initiated withdrawal reason that does NOT penalize source reliability for the user actor.

### 2.2 Backfill migration

Add a migration (next available version after the current head; check `src-tauri/src/migrations.rs` at PR time) that:

1. Reads every row from `entity_context_entries`.
2. Constructs an `intelligence_claims` row per entry: stable mapping of entity_context_entry IDs to claim IDs (`uuid_v5(NAMESPACE_USER_NOTE, entity_context_entry.id)` so re-running the migration is idempotent). Fields:
   - `subject_ref` from `entry.entity_type` + `entry.entity_id`
   - `claim_type = 'user_note'`
   - `text = entry.content`
   - `dedup_key` per the 2.1 policy
   - `actor = 'user'`, `data_source = 'manual'`
   - `observed_at = entry.created_at`, `source_asof = entry.created_at`
   - `provenance_json` with the user actor + a synthetic source identifier `("user_note", entry.id)`
   - `claim_state = 'active'`, `surfacing_state = 'active'`, `temporal_scope = 'state'`, `sensitivity = 'internal'`
3. Routes the writes through `services::claims::commit_claim` so the claim_writer_allowlist lint stays green and trust compilation runs at backfill time, not as a deferred recompute.
4. Migration runs inside `db.with_transaction` so partial backfills cannot leave the DB in a mixed state.
5. Records the legacy `entity_context_entries` row IDs in a new `legacy_user_note_migration_audit` table so we can prove every row was accounted for. The audit table also lets us detect drift if anyone reactivates the legacy write path before we drop it.

### 2.3 Update / delete UX through claim lifecycle

Claims are immutable. Update semantics:

- **Edit a note**: `services::claims::commit_claim` with `supersedes = old_claim_id`. The old claim transitions to `superseded`; the new claim is `active`. The `dedup_key` for the new claim is freshly generated (reflects the new content's `created_at_ms`). Display shows only the latest active claim; provenance retains the supersession chain.
- **Delete a note**: `services::claims::withdraw_claim` with `reason = 'user_deleted'`. The claim transitions to `withdrawn`; UI hides it. The withdrawal does NOT emit a contradicting-claim signal.
- **Restore a deleted note within the session**: out of scope for v1.4.0. Treat as v1.4.1 follow-up if users complain.

UX behavior change worth confirming with the user before implementation:
- Editing a note creates a new immutable claim under the hood. From the user's perspective the note "is" the same note; provenance under `/About this` shows the edit trail. Acceptable? If not, the alternative is "user notes are not claims" and we abandon this ticket's substrate-claim approach.

### 2.4 Cut over Tauri commands

Once the migration is in and the new claim type compiles + commits cleanly:

1. `commands/workspace.rs::create_entity_context_note` calls `services::claims::commit_claim` with the new type instead of `services::entity_context::create`.
2. `commands/workspace.rs::update_entity_context_note` calls `services::claims::commit_claim` with `supersedes`.
3. `commands/workspace.rs::delete_entity_context_note` calls `services::claims::withdraw_claim`.
4. `commands/workspace.rs::get_entity_context_entries` cuts over from `services::entity_context::get_entries` to `services::context::read_entity_context_claim_entries` (the cycle-2 cutover that cycle-3 rolled back, now correct because writes match).
5. The existing `tests/w5_l2_track_e_tauri_entity_context_test.rs` integration test is updated to seed the new claim type and exercise create→edit→delete→read; the pre-existing seed-claim-then-read paths from cycle-3 already work and must continue to.

### 2.5 Drop or freeze the legacy table

Once the cutover is in production and the integration test confirms the new path works end-to-end, freeze writes to `entity_context_entries`:

- Add a CHECK constraint on the table that rejects all new INSERTs (`CHECK (FALSE)` or equivalent).
- Add a lint to `src-tauri/scripts/` that fails CI if any code path references `entity_context_entries` outside the migration and the audit table.
- The table itself stays in the schema for the v1.4.0 release so we have a rollback path. A v1.4.1 follow-up drops it.

## 3. Key decisions

**Edit-as-supersession is the explicit UX.** The alternative — treating notes as a separate "user annotations" model outside the claim substrate — was considered and rejected. Rejected because: the whole point of the ability runtime is to put user-attributed and system-generated claims through the same trust + provenance + sensitivity pipeline. Having a parallel annotation model defeats the substrate's promise. If user research finds the supersession-on-edit model surprising, we revisit at v1.4.1, but ship with it.

**Sensitivity defaults to Internal, not UserOnly.** UserOnly is a stronger gate than user notes typically need. Internal allows the agent to see them (which matches user expectation: "if I tell the agent I made this note, the agent can see it"). Users who want stronger gating can mark a specific note as Confidential or UserOnly via the existing sensitivity controls.

**Trust compilation runs at backfill time.** The migration commits each backfilled claim through `commit_claim`, which triggers trust compilation. This costs migration time — for ~10k existing entries that's a few seconds — but means the post-migration DB is in a fully-coherent state with no deferred recompute backlog.

**Backfill is idempotent.** `uuid_v5` mapping from legacy ID to claim ID means re-running the migration is safe. This is load-bearing for the rollback story: if we have to revert the cutover and re-run later, the second run does not duplicate claims.

**Legacy table writes are blocked, not the table dropped.** A dropped table is destructive; a blocked-writes table preserves the rollback path. The drop is a v1.4.1 follow-up after v1.4.0 ships.

## 4. File scope

New / modified files:
- `src-tauri/src/migrations/<NEXT>_user_note_claim_type_backfill.sql` — schema additions for any required indexes plus the audit table
- `src-tauri/src/migrations.rs` — register the new migration
- `src-tauri/src/services/claims.rs` — add `user_note` claim type validation if needed; the existing `commit_claim` should accept it without code change other than the type registry
- `src-tauri/src/services/context.rs` — confirm `read_entity_context_claim_entries` returns user_note claims correctly (already does post-W5; verify)
- `src-tauri/src/commands/workspace.rs` — cut over the four entity-context commands
- `src-tauri/src/db/types.rs` — add `ClaimType::UserNote` variant if claim types are an enum
- `src-tauri/tests/w5_l2_track_e_tauri_entity_context_test.rs` — extend regression to the full round-trip
- `src-tauri/tests/dos411_user_note_migration_test.rs` — new test exercising the backfill migration end-to-end on a seeded legacy DB
- `src-tauri/tests/dos411_user_note_lifecycle_test.rs` — new test exercising commit / supersede / withdraw

Files NOT in scope:
- The frontend UI for entity context notes is consumed via the existing Tauri commands; if those commands keep their public signatures, no frontend change is required. The bridge envelope shape becomes available as a future `useEntityContextEntries` migration target (DOS-320 territory).
- Anything in `prepare_meeting/*` — the Transform pilot already reads via the claims service and is unaffected.

## 5. Acceptance

- `cargo clippy --no-default-features -- -D warnings` clean
- `cargo test --no-default-features --tests` passes including:
  - `tests/w5_l2_track_e_tauri_entity_context_test.rs::workspace_tauri_create_then_read_returns_created_entity_context_note` (extended to round-trip)
  - `tests/dos411_user_note_migration_test.rs` (new — backfill correctness, idempotence, audit table populated)
  - `tests/dos411_user_note_lifecycle_test.rs` (new — commit / supersede / withdraw semantics)
- `bash src-tauri/scripts/check_claim_writer_allowlist.sh` passes
- New lint script blocking `entity_context_entries` references outside migration + audit
- Full Intelligence Loop 5-question check answered explicitly in this plan and reflected in the new claim type's serde + service shape
- Manual smoke: dev workspace with seeded legacy notes survives migration, all notes visible, edit + delete behave correctly

## 6. Open questions

1. Final claim type name: `user_note`, `entity_note`, `user_annotation`, or other? `user_note` is shortest and most direct.
2. Does `entity_context_entries` have rows tied to entity types we don't yet have a `subject_ref` mapping for? Audit before writing the migration.
3. Should the audit table be persistent (production schema) or migration-scoped (dropped after migration confirms)? Persistent is safer for rollback, larger schema.
4. Does the user expect editing a note to preserve the original `created_at`, or to update it? Default behavior with supersession is "claim has its own created_at, but UI can show the chain's earliest as the note's age." Acceptable?
