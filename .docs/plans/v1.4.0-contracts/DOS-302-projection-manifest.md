# DOS-302 — Claim projection manifest, backfill parity, legacy-cache semantics

**Status:** verified satisfied at v1.4.0 wave tip (`658dbd07`).
**Acceptance walk last refreshed:** 2026-05-07.

## Contract

`intelligence_claims` is canonical. Legacy readers continue working from derived state populated by DOS-301 derived-state writers. Every claim-shaped field that v1.4.0 writes or reads is covered by an explicit projection rule with merge / ordering / tombstone / parity semantics. SQLite-resident projections are transactional with `commit_claim`; filesystem cache (`intelligence.json`) is post-commit + repair/rebuild on drift.

## Projection manifest

| (claim_type, field_path) | Canonical claim row shape | Subject ownership | Legacy target | Merge behavior | Ordering / replacement | Tombstone | Provenance fields | Transaction | Parity assertion | Owner |
|---|---|---|---|---|---|---|---|---|---|---|
| `entity_assessment.health_score` | `IntelligenceClaim` with `claim_type=entity_health_score`, `subject_ref=Account(id)`, `value=numeric` | account.id | `entity_intelligence.health_score` (column) | latest-state filter (`superseded_at IS NULL`) | `claim_sequence` desc, latest active wins | dismissal → suppress in projection; mark_false → exclude | `source_refs`, `source_asof`, `extractor`, `linker` | DB transaction within `commit_claim` | `entity_intelligence.health_score == latest active claim.value` | DOS-301 |
| `entity_assessment.health_reason` | `claim_type=entity_health_reason`, subject Account, `value=text + structured` | account.id | `entity_intelligence.health_reason` | latest-state | `claim_sequence` desc | as above | as above | DB tx | parity equality | DOS-301 |
| `entity_assessment.value_delivered` | `claim_type=entity_value_delivered`, subject Account, list of typed items | account.id | `entity_intelligence.value_delivered` (JSON column) | append-collapse via canonicalization (DOS-280) | newer claim wins on collision; canonicalization reduces duplicates | wrong_source → re-attribute; mark_outdated → drop from current | as above | DB tx | parity: rendered list == canonicalized active claim list | DOS-301 |
| `entity_assessment.company_overview` | `claim_type=entity_company_overview`, subject Account, text | account.id | `accounts.company_overview` | latest-state | `claim_sequence` desc | as above | as above | DB tx | parity equality | DOS-301 |
| `entity_assessment.strategic_programs` | `claim_type=entity_strategic_program`, subject Account, structured | account.id | `accounts.strategic_programs` (JSON column) | append + canonicalization | per-program latest active wins | as above | as above | DB tx | parity: program list matches active claims | DOS-301 |
| `entity_assessment.notes` | `claim_type=user_note` (DOS-411) | account.id / project.id / person.id (matches owner) | `accounts.notes` / `entity_context_entries` (legacy, frozen) | latest-state | supersession on edit | withdrawal on delete | user actor, manual data source | DB tx | post-DOS-411: `entity_context_entries` is frozen; reads come through `get_entity_context` claim path | DOS-411 |
| `success_plans.*` | `claim_type=success_plan_field`, subject Account, structured per ADR | account.id | `success_plans.*` columns | latest-state per field | per-field latest active wins | as above | as above | DB tx | parity per field | DOS-301 |
| `meeting.attendee_context` | `claim_type=meeting_attendee_context`, subject `Multi(meeting + attendee_account + attendee_person)`, structured | meeting + attendees | rendered through `prepare_meeting`; not a separate legacy column | composed at read time from `get_entity_context` | n/a — composition, not projection | n/a | as above | n/a | n/a — composition contract, not projection | DOS-219 |
| `meeting.topics` | `claim_type=meeting_topic`, subject Meeting, structured | meeting.id | not projected (live composition) | latest-state | `claim_sequence` desc | as above | as above | DB tx | n/a (composed at read) | DOS-219 |
| `meeting.suggested_outcomes` | `claim_type=meeting_suggested_outcome`, subject Meeting, structured | meeting.id | not projected | latest-state | `claim_sequence` desc | as above | as above | DB tx | n/a | DOS-219 |
| `intelligence.json` (filesystem cache) | per-account JSON snapshot | account.id | `<dataDir>/intelligence/<account>.json` | post-commit cache write | overwrite on each commit | drift-recoverable via `reconcile_post_migration --repair` | derived from DB projections | **post-commit**, NOT in `commit_claim` tx | repair/rebuild on demand; not parity-asserted with DB | DOS-301 + reconcile binary |

### Projection / cache distinction (DB-transactional vs filesystem-cache)

- **DB legacy projections** (every row except `intelligence.json`) run inside the `commit_claim` SQLite transaction. Rolled back atomically on commit failure.
- **`intelligence.json` is a post-commit filesystem cache.** Cannot have rollback-equivalent semantics with the DB. Repair/rebuild via `cargo run --bin reconcile_post_migration --repair`.

## Acceptance criteria — verification

### Manifest exists in repo and is linked from DOS-7 / DOS-301

This document. Linked from DOS-7 ticket (claim schema) and DOS-301 (derived-state writers).

### Manifest covers entity_assessment / entity_intelligence, account narrative columns, project narrative columns, success-plan-shaped state, intelligence.json fields

Covered above. Project narrative columns: same shape as account narrative columns; subject_ref points to project. (Project surfaces are W4 read-only; not yet writing claims for project-only fields; included for forward compatibility.)

### Each manifest row has owner, target, merge semantics, tombstone semantics, parity assertion

Columns above.

### Backfill dry-run reports row counts, malformed JSON quarantine, skipped fields, projection deltas

`src-tauri/src/bin/repair_entity_linking.rs:45-97` runs `--dry-run` mode that reports row counts, malformed/quarantined entries, and skipped fields. `src-tauri/src/bin/reconcile_post_migration.rs:8-9` runs `--repair`-less reconcile (read-only) reporting projection deltas without writes.

DOS-216 / DOS-301 backfill harness: harness fixtures exercise the dry-run path under `tests/fixtures/bundle-*`. See `tests/harness.rs` + `src-tauri/src/harness/runner.rs:122` for evidence-bound report writes.

### Entity-centric latest-claim indexes are specified for current reader patterns

Migration set (W3-C / W4-A) includes indexes on `intelligence_claims`:
- `(subject_ref, claim_type, superseded_at)` — latest active per (subject, type)
- `(subject_ref, claim_sequence DESC)` — claim_sequence ordering for resurrection
- `(superseded_by)` — supersession traversal
- Partial index on `superseded_at IS NULL` for active-only fast path

Enforced by migration test suite. See `services/claims.rs::latest_active_claim_for` reader path.

### Timestamp parsing tests replace string timestamp comparison for suppression / resurrection

`commit_claim` and supersession lookup parse `source_asof` and `created_at` via `chrono::DateTime<Utc>` typed comparison, not string lex compare. The bundle-3 fixture (stale source resurrection) and `dos283_bundle5_double_refresh_resurrection_test.rs` exercise the typed parse path. Malformed / future-dated timestamps emit `ProvenanceWarning::SourceTimestampUnknown` and downgrade through trust scoring rather than silent string-compare bugs.

### DB projection rollback test exists

`commit_claim`'s SQLite transaction wraps both the claim write AND every DOS-301 derived-state writer registered for the claim's `claim_type`. If any projection write fails, the transaction rolls back; the claim is not persisted. Exercised by integration tests in `src-tauri/tests/dos301_*` that inject failure on a projection writer and assert no claim row appears.

### Filesystem cache projection has post-commit repair / rebuild semantics and is not claimed to roll back with SQLite

`intelligence.json` writes are explicitly post-`commit_claim`. The reconcile binary (`reconcile_post_migration`) handles drift detection + repair. Documented in `services/claims.rs:16-17` ("D5 owns reconcile_post_migration").

Drift is treated as expected (not a correctness failure): the cache is rebuildable from the DB at any time. Daily app start runs reconcile in read-only mode; user can invoke `--repair` if anything is flagged.

### DOS-218 cannot require claims-only reads until backfill / parity evidence exists

DOS-218 shipped at `6a191be3` with bundle-1 fixture asserting parity (read-pilot exact equality). Cycle-2/3 of W5 boundary L2 explicitly held the read cutover until parity evidence was written; cycle-3 rolled back the Tauri read, cycle-7 sealed the prompt-input gate, cycle-8 APPROVE confirmed parity. Gate cleared.

### DOS-219 cannot require claims-only reads until meeting-brief inputs have equivalent claim/parity evidence

DOS-219 shipped at `6a191be3` with bundles 5+13 covering correction resurrection and direct subject bleed. Bundle-1 covers cross-entity ambiguity for the composed `get_entity_context` path. Cycle-8 of W5 boundary L2 APPROVE; W6 cycles re-validated against the ClaimTextRenderer carrier wrapping. Gate cleared.

## Outstanding

None. Manifest committed; verification gates passed.

## References

- ADR-0098 — Source revocation policy
- ADR-0105 — Provenance Envelope + amendments
- ADR-0124 — thread_id allowance
- ADR-0125 — temporal_scope + sensitivity + claim type registry
- DOS-7 (canonical claim schema), DOS-301 (derived-state writers), DOS-218/219 (read pilots), DOS-411 (Tauri claim-backed lifecycle), DOS-280 (canonical duplicate collapse)
- W5 proof bundle (commit `b5bb3bbd`), W6 proof bundle (commit `17afb9e7`)
