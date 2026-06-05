# DOS-832 L1a Proof Bundle -- Current-Encrypted Replica Correction Replay

**Issue:** DOS-832
**Branch:** `codex/v1.4.9-w1-dos832-l1`
**Date:** 2026-06-05
**Scope:** L1a substrate proof only. This proves replay of W3 claim-file sidecar feedback into freshly regenerated claim IDs in the current encrypted/Replica-compatible schema. It does not claim final plain-SQLite rebuild, Live cutover, or full DOS-832 release completion.

## Verdict

**L1a pass, bounded.**

The branch proves the correction-replay core that DOS-832 can honestly validate before DOS-831 lands:

- W3 sidecar feedback rows now carry a stable `feedbackId` replay key.
- Legacy v1 sidecars without `feedbackId` replay through a deterministic derived key so old sidecars remain readable without pretending they had durable source IDs.
- DOS-832 resolves sidecar entries by rebuild-stable semantic identity, not old runtime claim UUID.
- v2 replay requires source-content hash provenance before mutation.
- Replay claims the stable sidecar event in a durable journal before calling the claim feedback service.
- Feedback replay goes through `services::claims`, not raw `claim_feedback` inserts.
- Re-running the replay does not duplicate feedback rows.
- Same event ID with changed feedback content is rejected.
- Missing stable replay IDs are rejected before mutation.
- Duplicate stable replay IDs are rejected before mutation.
- Unsupported sidecar schema versions are rejected before mutation.
- Missing, provenance-mismatched, or ambiguous semantic matches are represented as orphan outcomes, not guessed.
- Service-invalid feedback rows are marked `failed` in the replay journal and do not abort later independent rows.
- Stable replay event IDs cannot be retargeted after applied, failed, or orphaned terminal journal outcomes.
- Partial v282 journal rows from an interrupted old-shape migration are repaired before replay continues.

## Explicit Non-Claims

These remain unproven by this L1a slice:

- Plain-SQLite fresh recovery after DOS-831.
- Live destructive replacement and cutover exclusivity.
- Entity JSON seed trust-boundary replay.
- Full source registration, ingestion, re-enrichment, salience, embeddings, and derived-context rebuild.
- Supersession and contradiction edge replay beyond sidecar serialization of semantic endpoint identities.
- Operator recovery docs and ADR-0048 amendment.
- Real-workspace end-to-end rebuild.

The runtime report returns `plainSqliteRecoveryProven: false` and `liveCutoverProven: false` to keep these boundaries machine-visible.

## Implementation Evidence

### Schema

- Added migration `v282_dos_832_rebuild_replay_journal`.
- Added `claim_feedback.replay_event_id` plus a unique partial index for replay idempotency.
- Added `rebuild_correction_replay_events` with status-coded replay outcomes:
  - `claimed`
  - `applied`
  - `already_applied`
  - `orphan_missing`
  - `orphan_ambiguous`
  - `failed`
- Journal rows persist a canonical feedback-content hash so replay can reject event-ID reuse with changed feedback content.
- Registered v282 through the idempotent multi-statement migration helper so schema-version record gaps can retry without duplicate-column failure.
- v282 includes an explicit idempotent repair for partial old-shape journal tables missing `feedback_content_hash`.
- Bumped claim-file sidecar schema version to `2` because stable `feedbackId` is now part of the replay contract.

### Services

- Added `services::rebuild::replay_claim_file_sidecar_corrections`.
- Added `services::claims::record_claim_feedback_replay`.
- Existing `record_claim_feedback` now delegates through a shared internal writer path; replay adds only stable-event idempotency and does not bypass actor or feedback validation.
- `claim_files` sidecar serialization now includes stable feedback row IDs and semantic identities for supersession/contradiction endpoints.

## Intelligence Loop Check

1. **Claim model:** Replay does not create display-only data. It targets regenerated claim rows and writes typed claim feedback through the claim service.
2. **Provenance + trust:** Sidecar semantic identity retains subject, claim type, field path, source ref, source-asof, observed-at, and source-content hash for matching. Replay requires the regenerated claim to match those provenance fields before applying feedback. Trust movement remains owned by the claim feedback writer and repair queue.
3. **Signals + invalidation:** Replay uses the same `record_claim_feedback` path, so existing feedback signals, version events, invalidation bumps, and repair-job coalescing remain active.
4. **Runtime + surfaces:** The L1a helper is service-owned and callable by future rebuild orchestration. It does not expose a user-visible surface yet.
5. **Feedback loop:** User feedback rows replay as typed `FeedbackAction` events with existing claim-service behavior; duplicates are suppressed by stable event ID.

## Validation

Commands run from the DOS-832 worktree:

```bash
src-tauri/scripts/check_migrations_transactional.sh
cargo test --manifest-path src-tauri/Cargo.toml services::rebuild --lib -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml services::claim_files --lib -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml record_claim_feedback --lib -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml migration_282 --lib -- --nocapture
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
git diff --check
```

Results:

- Migration wrapper check: passed.
- Focused DOS-832 replay tests: 18 passed.
- Claim-file tests: 18 passed.
- Claim feedback tests: 23 passed.
- v282 migration regressions: 2 passed.
- Clippy: passed with `-D warnings`.
- Full Rust lib suite: 3181 passed, 0 failed, 11 ignored.
- Full Cargo suite: passed, including integration tests and doc tests.
- TypeScript: passed.
- Diff hygiene: passed.
- Note: full Cargo emitted two pre-existing unused-import warnings in `tests/dos567_fixture_backfill_and_composition_versions.rs`; `cargo clippy -- -D warnings` passed.

## L2 Inputs

The next L2 reviewer should focus on these risk areas:

- Whether replay-event claiming is sufficiently atomic for the current single-writer/local topology.
- Whether the sidecar stable `feedbackId` contract belongs in W3/DOS-628 before DOS-832 merges, or whether the stacked DOS-832 branch may extend the W3 sidecar contract.
- Whether the L1a proof boundary is acceptable for PR scope, given the full DOS-832 packet still requires DOS-831 and broader rebuild orchestration.
- Whether supersession/contradiction endpoint semantic identities are enough as contract plumbing while actual edge replay remains out of this slice.
