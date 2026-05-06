# Wave W5 Proof Bundle

**Wave:** W5 (Read + Transform pilots on the abilities runtime — DOS-218 `get_entity_context` Read pilot, DOS-219 `prepare_meeting` Transform pilot)
**Status:** Cycle-8 L2 APPROVE — ship-ready, no material findings
**Date:** 2026-05-06

---

## Initial wave landing

W5-A (DOS-218) and W5-B (DOS-219) landed together at `6a191be3` — the Read pilot ability, the Transform pilot ability composing it, six bundle fixtures (1, 5, 9-13), MCP wiring, and ServiceContext support for the new readers. Eight cycles of L2 closure followed before the wave hit APPROVE.

## L2 review closure — final commit chain

| Commit | Cycle | What |
|---|---|---|
| `6a191be3` | landing | W5-A + W5-B initial implementation, 6 bundle fixtures, MCP wiring |
| `3cc3a3d3` | 1 | Track A (claim-backed `get_entity_context` + depth + Agent role) + Track D (ADR-0106 canonical replay key + Transform scorer thresholds) |
| `52c376ee` | 1 | Track B+C — live claim-backed `MeetingBriefContext` builder + meeting-scope source gate + 6 fixtures rebuilt without private context |
| `145e306c` | 2 | Track E (Tauri command read cutover) + F+G (scope-first meeting gate + ADR-0106 prompt fingerprint + bundle-13 direct-bleed adversarial replay) |
| `bf22c68f` | 3 | Track H (Tauri read rollback — write cutover deferred to follow-up projection ticket) + I (replay/fingerprint metadata parity) + J (linked-meeting subjects added to scope allowlist) |
| `3fb0345c` | 4 | Track K (`#[serde(skip_deserializing)]` on private context — close MCP/Agent injection path) + L (`test-harness` Cargo feature gating Track H test helpers out of release builds) |
| `92703cb4` | 5 | Track M (fail-closed `ServiceContext` reads in non-Live mode) + N (filter source_ref-matched claims by subject before prompt input) |
| `bce54342` | 6 | Track O (`entity_contexts` channel allowlist + sensitivity) + P (sensitivity filter on `get_entity_context` Agent path) |
| `055516ea` | 7 | Track Q — comprehensive prompt-channel sensitivity sweep across all 5 claim-bearing channels |

**Cycle 8: APPROVE — no material findings.**

---

## Architectural decisions and pattern shifts

### Two-pilot composition (DOS-218 Read + DOS-219 Transform)

- `get_entity_context` (Read) is the single claim-backed read path for entity claim fan-in; depth bounded; registered for User + System + Agent (per DOS-218 acceptance for MCP discovery).
- `prepare_meeting` (Transform) composes `get_entity_context` per resolved attendee/linked subject, then synthesizes the brief through a versioned prompt template with byte-equal provider replay (ADR-0106).

### Test/private surface area gating (cycles 4-5)

- `PrepareMeetingInput.context` carries `#[serde(skip_deserializing, skip_serializing)]` so an MCP/Agent caller cannot inject fabricated context JSON to bypass the live builder.
- `test-harness` Cargo feature gates the Track H Tauri-command test helpers (`DbService::open_at_unencrypted_for_tests`, `command_test_api`, `substrate_test_api`, `AppState::test_with_db_service`) out of release builds. A `compile_error!` guard hard-fails when the feature is enabled in a release profile.
- `ServiceContext` non-Live read methods (`read_prepare_meeting_context`, `read_entity_context_claims`, sibling legacy `read_entity_context_entries`) return `ServiceError::FixtureReaderRequired` instead of falling through to `ActionDb::open()` on the user's real workspace DB.

### ADR-0106 canonical replay parity (cycles 1-2)

- `ReplayProvider` lookup and `PromptFingerprint` provenance now compute `canonical_prompt_hash` from the same canonicalization helper (template id + version + canonical template hash + canonical JSON inputs + provider/model/sampling fields).
- `ReplayProvider::complete` stamps the returned `Completion` with the same `FingerprintMetadata` it used for the lookup key, so non-default provider/model/temperature/top_p/seed cannot diverge replay key from provenance hash.
- Fixtures missing the canonical key hard-fail rather than silent-fallback.

### Subject-scope and sensitivity gating (cycles 2-7)

This was the wave's longest-running closure class. Six cycles of partial-fix patching one channel at a time before the cycle-7 sweep stopped the pattern:

- **Cycle 2:** `source_subject_allowed` rejects sources whose subject is not the meeting or a resolved attendee/account/person. Bundle-13 adversarial replay where the provider mislabels an adjacent-account source as the meeting.
- **Cycle 3:** `meeting_scope_source_subjects` extended to include linked entities from `meeting_entities`/`linked_entities`. Direct adjacent-account bleed regression added.
- **Cycle 5:** `load_prepare_meeting_claims` filters source_ref-matched claims against the resolved subject allowlist before adding to evidence.
- **Cycle 6:** Composed `get_entity_context` children filtered against the same allowlist + Public/Internal sensitivity gate before `PromptContext::from_context` runs. `get_entity_context` itself filters claims by sensitivity for the Agent actor.
- **Cycle 7 (the sweep):** Centralized Public/Internal-only gate in `services/claims.rs`. Applied uniformly across all five claim-bearing prompt channels:
  1. Subject-ref claims (`load_claims_active`)
  2. Source-ref claims (`load_claims_active_by_source_ref`)
  3. `snapshot.claims → EvidenceSource` mapping
  4. Composed `EntityContextEntry` children
  5. Prebuilt `PrepareMeetingInput.context.evidence` test seam

  Channels 6 (rendered prompt + canonical JSON) and 7 (template variables) inherit the filter through their upstream. Unknown sensitivity values fail closed. End-to-end sweep regression seeds Public/Internal/Confidential/UserOnly claims and asserts restricted-sensitivity text and source IDs are absent from evidence, entity contexts, canonical JSON, and rendered prompt.

### Tauri read/write split (cycle 3)

The cycle-2 Tauri command cutover migrated `get_entity_context_entries` reads to claims while leaving create/update/delete on the legacy `entity_context_entries` table. Cycle 3 caught the create→read divergence: a user could save a note and the refreshed read would return nothing. Fix path B (rollback the read cutover) was chosen over Fix path A (write through claims) because user-created generic context notes need a proper claim type plus projection/backfill/lifecycle, which is out of scope for the W5 read pilots. The Agent/MCP claim-backed ability path remains intact (DOS-218 acceptance preserved); the Tauri UI path stays on legacy until a follow-up projection migration. A live create-then-read regression guards against re-introducing the divergence silently.

---

## Class themes from the cycle progression

**Channel-by-channel partial fixes get expensive.** Cycles 2/3/5/6/7 each found a new claim-text channel into the prompt — the cycle-7 sweep broke the pattern by enumerating all 9 prompt-related channels (5 claim-bearing) and applying the gate uniformly. This is the same lesson logged in the existing systemic-look memory: 2+ similar findings → pause and audit the class, don't keep patching.

**Cycle-7 channel enumeration (preserved here for the next wave):**

1. Subject-ref claims via `load_claims_active`
2. Source-ref claims via `load_claims_active_by_source_ref`
3. `snapshot.claims → EvidenceSource` mapping (cycle-7 finding)
4. Composed `get_entity_context` children → `entity_contexts`
5. Prebuilt `PrepareMeetingInput.context.evidence` (private/eval seam)
6. Rendered prompt + canonical JSON inputs (downstream of 1-5)
7. Template variables in `prepare_meeting_prep.v1.txt` (downstream of 6)
8. Output-only provenance fields (not sent to provider)
9. Non-claim prompt data (meeting metadata, no sensitivity needed)

**Trust-boundary findings dominated.** Severity trended from cycle-1's 1 critical + 3 high + 2 medium down to cycle-7's 1 high. Every BLOCK after cycle 4 was a trust-boundary or surface-leak finding (eval seam fall-through, prompt-input subject filter, entity_contexts channel, Agent sensitivity, MCP-injectable private field, `cargo build --release` exposure of test helpers). The wave landed on a comprehensive "private claim text never reaches a public surface or the provider" invariant.

---

## Acceptance state at cycle 8

- `cargo clippy --no-default-features -- -D warnings` clean
- `cargo clippy --no-default-features --features test-harness -- -D warnings` clean
- `cargo test --no-default-features --tests` passes end-to-end including the DOS-311 ordering test (no flake observed in cycle-8 verification run)
- `cargo build --no-default-features` does not expose the `test-harness` symbols (release-profile guard fires when `test-harness` is enabled outside debug)
- All cycle-1..7 regressions present and passing — every channel sealed by an earlier cycle has at least one regression that exercises a hostile input on that channel
- Pre-existing `dos311_substrate_migration_sequence_end_to_end` ordering flake from W4 still passes solo as documented in the W4 proof bundle; cycle-8 rerun did not reproduce it

## Out-of-scope follow-ups

These were explicitly identified during the cycle progression and deferred to follow-up tickets rather than expanded into W5 scope:

- **Tauri entity-context write cutover** — needs a proper user-created-note claim type plus projection migration. Tracked in the cycle-3 commit `bf22c68f`.
- **ADR-0108 sensitivity rendering for callouts/published surfaces** — current W5 enforcement is on the prompt-input boundary and the `get_entity_context` Agent read; published rendering layers were inspected and don't carry private claim text but were not exhaustively audited.
- **Bundle-coverage expansion** — W5 ships with bundles 1, 5, 9-13. Other scenarios (multi-thread, stale Glean propagation across multiple meetings, etc.) belong in a follow-up bundle-catalog ticket.
