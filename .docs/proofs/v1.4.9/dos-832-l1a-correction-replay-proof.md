# DOS-832 Proof Bundle -- Correction Replay + Plain-SQLite Storage Addendum

**Issue:** DOS-832
**Branch:** `codex/v1.4.9-l3-storage-rebuild`
**Date:** 2026-06-06; L3 addendum updated 2026-06-07
**Scope:** L1a correction-replay proof plus L3 storage-reset addendum. The original L1a slice proves replay of W3 claim-file sidecar feedback into freshly regenerated claim IDs. The 2026-06-07 addendum proves the same replay path on a service-backed plain-SQLite DB and proves the Live cutover exclusivity substrate blocks new/shared DB access through typed `RebuildCutoverInProgress` errors.

## Verdict

**L1a pass, bounded; L3 storage-reset addendum pass, bounded.**

The branch proves the correction-replay core and the plain-SQLite storage boundary that DOS-832 can honestly validate at this stage:

- W3 sidecar feedback rows now carry a stable `feedbackId` replay key.
- Legacy v1 sidecars without `feedbackId` replay through a deterministic derived key so old sidecars remain readable without pretending they had durable source IDs.
- DOS-832 resolves sidecar entries by rebuild-stable semantic identity, not old runtime claim UUID.
- v2 replay requires the sidecar source identity hash before mutation. This hash binds `data_source`, `source_ref`, and `item_hash`; it is not proof of the full projection-file contents.
- Replay now authenticates the sidecar against a committed projection-run ledger before mutation, including checksum binding for the replayed sidecar payload.
- Replay claims the stable sidecar event in a durable journal before calling the claim feedback service.
- Feedback replay goes through `services::claims`, not raw `claim_feedback` inserts.
- Feedback signal rows are persisted inside the feedback transaction, so crash-after-write retry paths do not silently lose feedback signals.
- Missing feedback signals are repaired on idempotent replay and claim-file apply paths before a replay is considered recovered.
- Re-running the replay does not duplicate feedback rows.
- Replay preserves the original sidecar feedback `submitted_at` rather than rewriting it to rebuild time.
- Sidecar feedback older than an existing claim feedback watermark is rejected as stale before mutation; same-sidecar stale exclusions only apply when durable replay feedback matches the sidecar event action and content hash.
- Same event ID with changed feedback content or `submitted_at` is rejected.
- Same event ID collision against a different claim, action, or content is failed without attaching the wrong durable feedback row.
- Missing stable replay IDs are rejected before mutation.
- Duplicate stable replay IDs are rejected before mutation.
- Unsupported feedback actions are rejected during sidecar preflight, before any earlier row can mutate.
- Unsupported sidecar schema versions are rejected before mutation.
- Missing, provenance-mismatched, or ambiguous semantic matches are represented as orphan outcomes, not guessed.
- Recoverable orphan rows can be reclaimed only by the same sidecar/event once a later rebuild resolves the semantic claim match. Migrated legacy orphan placeholders keep checksum-neutral behavior because the original checksum is unknowable, but terminal rows still bind canonical feedback content.
- Deterministic service-invalid feedback rows are marked `failed` in the replay journal and do not abort later independent rows.
- Retryable service failures remain `claimed` so the same sidecar/event can be retried instead of being permanently terminal.
- Stable replay event IDs cannot be retargeted after applied, failed, or orphaned terminal journal outcomes.
- Partial v282 journal rows from an interrupted old-shape migration are repaired before replay continues.
- A follow-up v283 repair migration covers databases that already recorded the draft v282 migration before the journal shape was completed.
- Service-backed replay now runs against a file-backed plain-SQLite DB and reports `plainSqliteRecoveryProven: true`.
- Service-backed replay now proves the process/OS cutover gate blocks exclusive Live cutover while the `DbService` pool holds shared DB access, and reports `liveCutoverProven: true`.
- Direct `ActionDb` opens and raw `DbService` opens fail with typed `RebuildCutoverInProgress` while an exclusive cutover token is active.

## Explicit Non-Claims

These remain unproven by this bounded proof:

- Full destructive replacement file swap, restore-point creation, queue pause/drain/resume, phase manifest, rollback/roll-forward, and crash-consistency recovery.
- Entity JSON seed trust-boundary replay.
- Full source registration, ingestion, re-enrichment, salience, embeddings, and derived-context rebuild.
- Supersession and contradiction edge replay beyond sidecar serialization of semantic endpoint identities.
- Operator recovery docs and ADR-0048 amendment.
- Real-workspace end-to-end rebuild.

The runtime report keeps the boundary machine-visible: in-memory/direct replay remains false for both proof flags, while service-backed file replay sets `plainSqliteRecoveryProven: true` and `liveCutoverProven: true`.

## Implementation Evidence

### Schema

- Added migration `v282_dos_832_rebuild_replay_journal`.
- Added migration `v283_dos_832_rebuild_replay_journal_repair`.
- Added `claim_feedback.replay_event_id` plus a unique partial index for replay idempotency.
- Added `rebuild_correction_replay_events` with status-coded replay outcomes:
  - `claimed`
  - `applied`
  - `already_applied`
  - `orphan_missing`
  - `orphan_ambiguous`
  - `failed`
- Journal rows persist a canonical feedback-content hash, including `submitted_at`, so replay can reject event-ID reuse with changed feedback content or historical feedback time.
- Journal rows persist a sidecar checksum for new replay events, so replay cannot bind an event from one sidecar payload to another sidecar payload after authorization. Rows migrated with `legacy-v283-unset` keep checksum-neutral semantics because the original checksum was not recorded; terminal content hash still binds the feedback payload.
- Registered v282 through the idempotent multi-statement migration helper so schema-version record gaps can retry without duplicate-column failure.
- v282 includes an explicit idempotent repair for partial old-shape journal tables missing `feedback_content_hash`.
- v283 reruns the replay-journal repair path for databases that already recorded v282 before `feedback_content_hash` and `sidecar_checksum` existed.
- Bumped claim-file sidecar schema version to `2` because stable `feedbackId` is now part of the replay contract.

### Services

- Added `services::rebuild::replay_claim_file_sidecar_corrections`.
- Added `services::claims::record_claim_feedback_replay`.
- Existing `record_claim_feedback` now delegates through a shared internal writer path; replay adds only stable-event idempotency and does not bypass actor or feedback validation.
- `claim_files` sidecar serialization now includes stable feedback row IDs and semantic identities for supersession/contradiction endpoints.
- Replay validates sidecars against the committed projection run before claim matching or feedback mutation.
- Existing feedback replay events now repair missing feedback signals through the service layer before continuing.
- Added `services::rebuild` DB-path-scoped cutover gate plus OS-level shared/exclusive lock file. Normal DB access acquires a shared token; Live cutover acquires an exclusive token and fails typed when same-DB shared access is still open.
- `DbService` now holds the shared rebuild access token for the pool lifetime. Direct `ActionDb` open paths acquire fail-fast shared tokens during open/preflight.
- Replay reports plain-SQLite recovery proof from the actual file-backed DB header and cutover proof only when the service-held shared access token blocks exclusive cutover for that DB path.

## Intelligence Loop Check

1. **Claim model:** Replay does not create display-only data. It targets regenerated claim rows and writes typed claim feedback through the claim service.
2. **Provenance + trust:** Sidecar semantic identity retains subject, claim type, field path, source ref, source-asof, observed-at, and source identity hash for matching. Replay requires the regenerated claim to match those provenance fields before applying feedback. Trust movement remains owned by the claim feedback writer and repair queue.
3. **Signals + invalidation:** Replay uses the same `record_claim_feedback` path, so existing feedback signals, version events, invalidation bumps, and repair-job coalescing remain active.
4. **Runtime + surfaces:** The L1a helper is service-owned and callable by future rebuild orchestration. It does not expose a user-visible surface yet.
5. **Feedback loop:** User feedback rows replay as typed `FeedbackAction` events with existing claim-service behavior; duplicates are suppressed by stable event ID.

## Validation

Final commands run from the DOS-832 worktree after the Cycle 16 remediation:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib dos832_replay_rejects_unsupported_later_feedback_action_before_writes
cargo test --manifest-path src-tauri/Cargo.toml --lib dos832_replay_mismatched_sidecar_event_does_not_mask_stale_later_row
cargo test --manifest-path src-tauri/Cargo.toml --lib migrated_orphan
cargo test --manifest-path src-tauri/Cargo.toml --lib dos832
cargo test --manifest-path src-tauri/Cargo.toml --lib apply_claim_file_corrections_repairs_signal_for_already_applied_feedback
cargo test --manifest-path src-tauri/Cargo.toml --lib record_claim_feedback_replay_event
cargo test --manifest-path src-tauri/Cargo.toml --test dos7_d4_lint_test
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
git diff --check
```

L3 addendum commands run from `codex/v1.4.9-l3-storage-rebuild` after the plain-SQLite storage reset remediation:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml live_cutover_gate_blocks_new_shared_access_until_released
cargo test --manifest-path src-tauri/Cargo.toml shared_access_guard_blocks_live_cutover_until_released
cargo test --manifest-path src-tauri/Cargo.toml action_db_open_at_fails_while_live_cutover_active
cargo test --manifest-path src-tauri/Cargo.toml live_cutover_fails_while_db_service_pool_is_open
cargo test --manifest-path src-tauri/Cargo.toml dos832_replay_report_proves_plain_sqlite_and_cutover_when_service_backed
```

Results:

- Unsupported later feedback action preflight regression: 1 passed.
- Mismatched sidecar event stale-watermark regression: 1 passed.
- Migrated orphan placeholder regressions: 2 passed.
- Focused DOS-832 replay suite: 43 passed.
- Claim-file already-applied signal repair regression: 1 passed.
- Claim feedback replay idempotency suite: 6 passed.
- DOS7/D4 lint suite: 28 passed.
- Clippy: passed with `-D warnings`.
- Diff hygiene: passed.
- L3 storage addendum: check passed; four cutover-gate focused tests passed; service-backed plain-SQLite replay proof passed.

## L2 Review Cycles

The L2 remediation ran adversarial review cycles across five subagents: data migrations, reliability, testing, security, and adversarial review. All five passed on Cycle 16.

- Cycle 11 found missing mismatch proof, missing claimed-legacy signal repair, and terminal checksum rebinding. Fixed with content/action/claim collision tests, recovery signal repair, and legacy checksum-neutral terminal handling.
- Cycle 12 found content-hash adoption and signal-repair failure ordering gaps. Fixed by requiring content-hash equality for fallback recovery, preserving legacy checksums, and proving retryability after transient signal repair failures.
- Cycle 13 found pre-terminal content hash mutation and migrated orphan placeholder validation gaps. Fixed by moving content-hash writes into terminal updates and allowing recoverable migrated orphans through validation.
- Cycle 14 found orphan reclaim pre-binding. Fixed by making orphan reclaim set only `claimed`/`resolved_claim_id`; terminalization owns content binding while legacy checksum placeholders remain neutral by policy.
- Cycle 15 found sidecar-wide stale exclusions and malformed later feedback action partial writes. Fixed with content-aware stale exclusions and action preflight.
- Cycle 16 verdict: data migrations PASS; reliability PASS; testing PASS; security PASS; adversarial PASS.
