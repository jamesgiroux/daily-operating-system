# DOS-831 L0 Packet (Draft) - Drop SQLCipher Encryption to Plain SQLite + FileVault

**Version:** v1.4.9 - W1b (storage reset)
**Issue:** [DOS-831](https://linear.app/a8c/issue/DOS-831). Parent umbrella [DOS-820](https://linear.app/a8c/issue/DOS-820).
**Author date:** 2026-06-03
**Tier:** Tier 3 (markdown-only). **Scope tier:** High-risk substrate/security change.
**Status:** Draft for L0 cycle 1 review.

---

## §0 Origination + Scope

DOS-831 replaces app-layer SQLCipher encryption for the DailyOS SQLite store with plain SQLite protected by the macOS disk boundary (FileVault / managed-device controls). This is a **real supersession of ADR-0092**, not a quiet code cleanup.

The work is debug-driven and decision-driven:

- The 2026-05-28 production DB-loss incident came from the dev loop opening the production SQLCipher DB and killing a hot WAL writer. DOS-820/821/822 isolated Live/Replica/Mock paths; DOS-823 adds replica refresh and destructive-command guards.
- DOS-848 records a second production btree corruption incident found 2026-06-01: 10 tables and 21 indexes damaged; standard repair tools OOM/abort; logical extraction into a fresh schema-123 DB was the working repair. Root cause remains under investigation, but the failure class is adjacent to hot WAL writer termination, stale MCP binaries, and SQLCipher open/backup complexity.
- ADR-0134 and `db-lock-storm-class` both name SQLCipher per-page decryption as a real reader CPU cost. Removing SQLCipher simplifies the open/read/backup path and removes that cost class, but does not change SQLite's single-writer WAL contract.
- The v1.4.9 wave plan records the 2026-05-30 L6 direction: FedRAMP/at-rest-encryption is retired for this product posture; cipher-drop proceeds through a real ADR-0092 amendment.

### Scope

In scope:

1. Author a superseding ADR-0092 amendment section that explicitly retires SQLCipher for the local same-OS-user product posture and names FileVault / OS disk controls as the at-rest boundary.
   - Author the missing DB-mode-isolation ADR/addendum for DOS-820-A in the same docs pass, because the active wave plan assigns that storage-boundary record to DOS-831.
2. Switch `rusqlite` from `bundled-sqlcipher` to the non-SQLCipher bundled SQLite feature set while preserving `backup`.
3. Replace encrypted DB open chokepoints with plain SQLite opens:
   - `ActionDb::open`, `open_for_inspection`, `open_at`, `open_readonly`, and test helpers.
   - `DbService` pooled writer/reader startup, `DbService::open_fresh_serialized`, and any migration/backup call that currently accepts an `EncryptionKey`.
   - MCP/read-only, release-gate, maintenance-bin, and recovery/status surfaces that currently depend on SQLCipher key state.
4. Retire SQLCipher-specific code paths:
   - `PRAGMA key`, `sqlcipher_export`, key verification via schema touch after keying, `EncryptionKey`, `SqlCipherPragma`, SQLCipher key generation/rotation/rekey code.
   - Plaintext-to-encrypted migration detection and migration.
5. Preserve storage safety primitives:
   - DB-mode guard (`Live`/`Replica`/`Mock`) before any file open.
   - Single-writer discipline and existing DB open guard lint.
   - WAL pragmas, busy timeout, migration backup, startup recovery, restore validation, and file permission hardening.
6. Define the DB encryption key retirement behavior:
   - Do not delete Keychain material automatically in the same release.
   - Stop requiring it for DB opens.
   - Add an operator/user-visible retirement note in docs and a follow-up if active cleanup is desired after rebuild proves stable.

Out of scope:

- No in-place cipher-to-plain migration. DOS-832 owns rebuild-from-canonical-JSON and depends on W3's correction-preserving projection.
- No workspace file encryption. Workspace files are already intentionally readable by local tools and remain governed by the workspace/plaintext contract.
- No change to Google OAuth, surface session keys, projection signing keys, Gravatar/Glean credentials, or any non-DB Keychain surface.
- No rollback of ADR-0092's non-cipher hardening by accident: restrictive permissions, Time Machine exclusion posture, iCloud workspace warning, app-lock posture, and PII log hygiene remain in force unless a later ADR amends them directly.
- No change to MCP egress sensitivity policy or claim actor semantics.
- No change to SQLite single-writer topology, reader-pool ownership, or background worker admission.
- No root-cause closure for DOS-848. DOS-831 should record how the failure surface changes and what follow-up checks would prove it, not pretend the root cause is known.
- No automatic deletion or rotation of unrelated runtime anchors. `db/key_provider.rs` contains non-DB material and must not be deleted wholesale.

### Migration Slots

The current dev schema head in this worktree is `v276`; next free is `v277`. DOS-831 is expected to require **no schema migration**: it changes the storage engine/open path, not the logical schema. If L1 discovers a durable marker table/column is required, claim `v277` before implementation and update this packet plus `.docs/plans/v1.4.9-waves.md`.

---

## §1 Ground Truth

### §1.1 Current SQLCipher Contract

ADR-0092 is Accepted and currently requires:

- `rusqlite = { version = "0.31", features = ["bundled-sqlcipher", "backup"] }`.
- DB key material stored in macOS Keychain.
- `PRAGMA key` before any other pragma or schema access.
- `open_readonly()` using the same key for MCP/read-only surfaces.
- Backup and migration paths that can apply destination key material.
- Recovery UI when the key is missing and an encrypted DB already exists.

Current implementation mirrors that:

- `src-tauri/Cargo.toml` uses `bundled-sqlcipher`.
- `src-tauri/src/db/encryption.rs` owns `key_to_pragma`, plaintext detection, and plaintext-to-encrypted `sqlcipher_export`.
- `src-tauri/src/db/key_provider.rs` owns `LocalKeychain`, `DbKeyProvider`, `EncryptionKey`, `SqlCipherPragma`, rotation accounts, cached key material, and rekey coordination.
- `src-tauri/src/db/core.rs` calls `guard_path_for_mode(path)` before key fetch/file open, applies `PRAGMA key`, validates schema readability, then sets WAL/busy/synchronous pragmas and runs `run_migrations_with_key`.
- `src-tauri/src/db_service.rs` applies DB key material to pooled writer/readers and fresh serialized opens.
- `src-tauri/src/db_backup.rs` applies an encryption key to backup destinations and tries encrypted validation first.
- `src-tauri/src/migrations.rs` accepts an optional `EncryptionKey`, applies keyed backup destinations, and has a transaction-wrapped `sqlcipher_export` fallback for encrypted backup failures.
- MCP read-only startup, release-gate manual readers, recovery/status commands, and frontend recovery routing still talk about DB key state or encrypted DB status.
- ADR-0092 also includes non-cipher hardening decisions: restrictive permissions, Time Machine exclusion, iCloud workspace warning, app-lock posture, and PII log hygiene. Those survive DOS-831 unless explicitly amended.

### §1.2 Existing Safety That Must Remain

- **DB-mode isolation:** DOS-820/821/822 route non-Live modes to isolated DB/config/workspace paths and structurally deny production DB opens from non-Live modes.
- **Replica refresh:** DOS-823 uses the backup API and destructive-command guards to clone Live to Replica without unsafe file copies.
- **Migration framework:** ADR-0071 owns schema versioning, forward-compatibility guard, and pre-migration backup behavior. DOS-831 changes storage mechanics only; logical schema migration stays under ADR-0071.
- **Single-writer discipline:** ADR-0133 says SQLite WAL has one file-level writer, fresh mutating connections do not add concurrency, and mutations must route through the writer lane.
- **Reader pool sizing:** ADR-0134 separates reader slot sizing from SQLCipher decryption cost. Dropping SQLCipher may remove one CPU cost, but reader ownership/sizing remains separate substrate.
- **Open guard lint:** `src-tauri/scripts/check_db_open_guard_allowlist.sh` ensures new file-backed `Connection::open` sites stay behind known guarded chokepoints.
- **Backup hardening:** chunked `rusqlite::backup::Backup` paths replaced unsafe `step(-1)` assumptions for large DBs.
- **Execution mode:** ADR-0104's `ExecutionMode` and service mutation checks are orthogonal to DB-mode `Live`/`Replica`/`Mock`. DOS-831 must not blur those boundaries.
- **Other Keychain contracts:** ADR-0068 and ADR-0116 keep OAuth/session/control-plane key material separate from the DB encryption key. DOS-831 retires the DB key path only.
- **Audit semantics:** ADR-0094 audit/event semantics may mention DB key or backup events; L1 must either remove stale DB-key audit semantics or rename them without leaking content.

### §1.3 What Is Genuinely Unbuilt

- A superseding ADR-0092 amendment for the new at-rest security posture.
- Plain SQLite open/read/backup/migration path with no SQLCipher key dependency.
- Keychain DB-key retirement behavior.
- Tests proving missing SQLCipher key material no longer blocks opening a valid plain SQLite DB.
- Tests proving encrypted SQLCipher DBs are not silently accepted as plain SQLite or mistaken for empty/fresh DBs.

---

## §2 Decisions

### D1 - At-rest Boundary

DailyOS v1.4.9 treats the OS disk/user boundary as the at-rest control for the local DB. In the current local single-user product posture, FileVault and the logged-in macOS user boundary are the relevant controls. SQLCipher no longer carries product value proportional to its operational cost.

The superseding ADR text must be explicit about what is given up:

- A copied `dailyos.db` file is readable as plain SQLite by the same OS user or any actor with filesystem access.
- SQLCipher no longer protects a DB copied out of the machine after the user unlocks the account.
- Workspace JSON/markdown was already plaintext, and Keychain-backed secrets remain separate.

### D2 - No In-place Cipher-to-plain Migration

Do **not** implement SQLCipher export/import as the migration path for DOS-831. The v1.4.9 storage reset already makes rebuild-from-canonical-JSON the recovery/migration story (DOS-832), and the 2026-06-01 corruption incident shows standard repair/export tools are unreliable on damaged encrypted DBs.

L1 should make plain SQLite the runtime target and keep direct conversion out of scope. If a developer needs to salvage an encrypted DB during the transition, use the documented logical-extraction/rebuild path, not automatic startup conversion.

DOS-831 is not releaseable to any environment with an existing encrypted active DB unless one of these is true:

- DOS-832's rebuild path is available and validated for that environment.
- The release is explicitly coordinated as a storage-reset cutover with operator confirmation that no encrypted active DB is being opened by the plain build.

This gate prevents a compile-clean plain SQLite build from stranding users before the rebuild path exists.

### D3 - Fail Loud on Encrypted Input

After DOS-831, an existing SQLCipher-encrypted DB at the active path must not be treated as a new empty DB. Startup/open should fail with a clear storage-health error that names the unsupported encrypted store and routes to rebuild/restore guidance.

The test should create a non-SQLite-header fixture or encrypted-looking file at the DB path and assert the open path does not silently run migrations into it.

Recovery/status surfaces should no longer present normal startup as "Database key missing" or key recovery. Old encrypted input is a storage-health/rebuild condition, not a runtime key-entry condition.

### D4 - Keychain Retirement

Stop using the DB encryption key at runtime. Do not delete the existing `com.dailyos.desktop.db/sqlcipher-key` automatically in DOS-831 because:

- It may still be needed for forensic/salvage reads of preserved encrypted backups.
- Removing it before DOS-832 proves rebuild/correction preservation would lower recovery optionality.

Document it as retired/unused and file cleanup as an explicit follow-up if desired after the rebuild path is proven.

This decision applies only to the SQLCipher DB key. OAuth tokens, surface session keys, projection signing keys, provider credentials, and other Keychain-backed secrets remain canonical and are outside the cleanup blast radius.

### D5 - Backups and Exports Become Plain SQLite Copies

Manual, pre-migration, restore-point, and user-selected export copies become plain SQLite files. Keep chunked backup API behavior, backup validation, restore snapshots, permissions, and pruning.

The PR body and ADR amendment must not claim backup/export confidentiality from app-layer encryption. Backup confidentiality is now the same OS disk boundary as the active DB. Exported copies are different because the user chooses the destination: Settings/recovery copy must state that destination disk/cloud controls determine confidentiality once the DB is exported.

---

## §3 Implementation Shape

### U1 - ADR + Docs

- Add an ADR-0092 amendment dated 2026-06-03 (or implementation date) that supersedes the SQLCipher decision for v1.4.9.
- Add the missing DB-mode-isolation ADR/addendum for DOS-820-A, or amend the wave plan if that deliverable moves elsewhere before L1 starts.
- Update operation/release docs that still describe DB files/backups as encrypted.
- Keep the distinction between DB at-rest posture and other Keychain-backed secrets.
- Enumerate ADR-0092 sub-decisions explicitly: SQLCipher/key/recovery decisions are retired; file permissions, Time Machine/iCloud posture, app lock, and PII log hygiene remain unless separately amended.
- Keep `.docs/plans/v1.4.9-waves.md` aligned with this per-ticket L0: ADR-0136 is already occupied by a different decision, and the prior split-build decrypt/sentinel plan is superseded by fail-loud + rebuild guidance. DOS-831 amends ADR-0092 instead of minting or reusing ADR-0136.

### U2 - Cargo / Build Feature

- Change `rusqlite` features from `bundled-sqlcipher, backup` to `bundled, backup`.
- Update `src-tauri/Cargo.lock` as part of the feature switch and remove SQLCipher-only build assumptions from tests/workflows if any fail after the feature change.
- Keep Tauri MCP sidecar build coverage in scope because stale sidecar binaries were part of the v1.4.9 failure surface.

### U3 - DB Open Path

- Collapse `ActionDb` open helpers to plain SQLite:
  - `guard_path_for_mode` remains before file open.
  - Parent dir creation and file permission hardening remain.
  - `Connection::open` / readonly flags remain behind guarded chokepoints.
  - WAL, busy timeout, synchronous NORMAL, migrations, foreign keys, and startup healing remain.
- Collapse `DbService` writer/reader pool initialization and fresh serialized opens to the same plain-open chokepoints.
- Replace `run_migrations_with_key(conn, Some(key))` with plain `run_migrations(conn)` or remove the key parameter entirely if no longer useful.
- Preserve test-only unencrypted helpers as either aliases or remove redundant split once all opens are plain.
- Update MCP read-only startup and release-gate manual reader paths so they open plain SQLite while preserving `query_only=ON` and DB-mode path guards.

### U4 - Key Provider Retirement

- Remove or reduce SQLCipher DB key provider types and rotation code.
- Do not delete `db/key_provider.rs` wholesale: it also contains non-DB/runtime-anchor material.
- Keep unrelated keychain modules untouched:
  - `services/surface_session_keychain.rs`
  - `services/projection_signing.rs`
  - Google/Glean/Gravatar keychain/token stores
- If DB-key code is left temporarily for salvage docs, it must be unreachable from normal DB open and named as legacy/salvage-only.
- Re-source helpers that currently derive from DB key material (`local_db_keyed_audit_tag`, `local_db_workspace_graph_diagnostic_key_bytes`) to a stable non-DB secret or deterministic non-secret scheme before implementation; do not leave them dangling or silently tied to a retired DB key.
- Update or remove DB-key-specific audit events so audit logs no longer imply runtime DB-key access after DOS-831.

### U5 - Backup / Restore / Migration Safety

- Update `db_backup.rs` so backup destinations are plain SQLite and validation does not try to create/fetch DB encryption keys.
- Update `export_database_copy` and its Settings/recovery entry points so exported copies are treated as plaintext egress: restrictive file permissions where possible, no app-layer confidentiality claim, and user/operator copy that names destination disk/cloud controls as the confidentiality boundary.
- Update `migrations.rs` backup helpers to remove destination keying and SQLCipher fallback.
- Keep hollow-backup detection, chunked stepping, Busy/Locked retry, restore snapshot, WAL/SHM cleanup, permissions, and pruning.
- Review restore shutdown ordering for both `state.db_service` and any installed global DB service before replacing/restoring files.
- Add explicit test coverage for schema-version reads and backup validation without Keychain.

### U6 - Guard / Regression Tests

Focused tests should cover:

- Valid plain SQLite DB opens without a Keychain DB key for `ActionDb`, `DbService`, MCP read-only, release-gate manual readers, and maintenance bins.
- Missing legacy SQLCipher Keychain entry does not block a valid plain DB open.
- Encrypted-looking/non-SQLite active DB fails loudly and does not get migrated/overwritten silently.
- SQLCipher-era `NotADatabase` recovery handling is updated for plain SQLite semantics instead of retrying as a key/WAL race.
- Manual backup produces a plain SQLite backup whose schema version can be read without key material.
- Export copy produces a plain SQLite file, sets restrictive permissions where possible, and the Settings/recovery UI copy warns that destination disk/cloud controls determine confidentiality.
- `check_db_open_guard_allowlist.sh` remains clean.
- Service/writer boundary gates remain clean: no new mutating fresh-connection bypass is introduced, and the SQLCipher `PRAGMA key` exception disappears rather than widening.
- No `PRAGMA key`, `sqlcipher_export`, `bundled-sqlcipher`, or DB encryption-key runtime dependency remains outside explicit historical docs/tests.
- Frontend startup/recovery UI no longer gates valid plain DB startup on `get_encryption_key_status` or shows key recovery as the normal error path.

---

## §4 Acceptance Criteria

- **AC1 - ADR supersession:** ADR-0092 has a committed amendment that explicitly retires SQLCipher for the v1.4.9 local same-OS-user posture, states the FileVault/OS disk boundary, and names what protection is no longer provided.
- **AC1a - Non-cipher hardening preserved:** The ADR amendment explicitly says ADR-0092's file permissions, Time Machine/iCloud posture, app lock, and PII log hygiene remain active unless separately amended.
- **AC1b - DB-mode record closed:** The missing DOS-820-A DB-mode-isolation ADR/addendum is committed in this PR, or the active wave plan is amended before L1 to assign that deliverable elsewhere.
- **AC2 - Plain runtime DB:** The app, MCP read-only path, maintenance bins, and backup/restore code open valid DailyOS DB files as plain SQLite with no `PRAGMA key` or DB encryption-key dependency.
- **AC3 - Fail-loud encrypted input:** An encrypted-looking existing DB at the active path is not silently overwritten, migrated, or treated as empty. The user/operator gets a clear storage-health/rebuild/restore error.
- **AC3a - Release/cutover gate:** DOS-831 is not releaseable to an environment with an existing encrypted active DB until DOS-832 rebuild is available and validated for that environment, or until a coordinated storage-reset cutover confirms the plain build will not strand encrypted stores.
- **AC4 - Backups/exports stay safe:** Manual/pre-migration/restore backups and user-selected export copies use the proven copy path, validate integrity where applicable, keep restrictive file permissions where possible, and are plain SQLite under the destination's disk/cloud boundary.
- **AC5 - Guards preserved:** DB-mode structural deny, open guard lint, single-writer routing, WAL pragmas, and migration backup behavior still pass.
- **AC6 - Keychain retirement:** Normal DB open no longer touches the SQLCipher Keychain key. Existing DB key material is documented as retired/unused, not automatically deleted in this PR.
- **AC6a - Diagnostic keys re-sourced:** `local_db_keyed_audit_tag`, `local_db_workspace_graph_diagnostic_key_bytes`, and their consumers use a non-DB-key scheme and do not touch the retired SQLCipher key.
- **AC7 - No accidental collateral:** Non-DB Keychain-backed secrets, runtime anchors, and surface/session/projection signing keys are untouched.
- **AC7a - Audit semantics corrected:** DB-key-specific audit/status copy is removed or renamed so runtime reports no longer imply SQLCipher key access.
- **AC8 - Gates green:** `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit && pnpm test` pass.

---

## §5 Test Plan

Focused:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib db::
cargo test --manifest-path src-tauri/Cargo.toml --lib db_backup
cargo test --manifest-path src-tauri/Cargo.toml --lib migrations::
bash src-tauri/scripts/check_db_open_guard_allowlist.sh
bash scripts/check_service_layer_boundary.sh
bash scripts/check_db_mutator_must_use.sh
pnpm test -- src/routerStartupGate.test.ts
rg -n "bundled-sqlcipher|PRAGMA key|sqlcipher_export|SqlCipherPragma|EncryptionKey|sqlcipher-key" src-tauri/src src-tauri/Cargo.toml src-tauri/Cargo.lock
```

Full gates:

```bash
cargo clippy -- -D warnings
cargo test
pnpm tsc --noEmit
pnpm test
```

If the implementation touches MCP sidecar packaging or release-gate readers, also run the Tauri external binary setup path:

```bash
bash src-tauri/scripts/build-mcp.sh
```

The `rg` command is a review aid, not a blanket delete instruction. Legitimate historical references may remain in the ADR amendment or archived docs; runtime code should not depend on them after DOS-831.

---

## §6 Intelligence Loop Integration Check

1. **Claim model:** No new claim/table/field/user-visible intelligence datum. The change affects storage/open mechanics only.
2. **Provenance and trust:** Claim provenance, source attribution, trust bands, and sensitivity labels are unchanged. The ADR amendment must not imply that trust scoring changes because the DB storage engine changes.
3. **Signals and invalidation:** No signal or invalidation topology change. Derived-state regeneration remains handled by existing services and future DOS-832/W3 work.
4. **Runtime and surfaces:** Tauri app, MCP read-only sidecar, doctor/maintenance bins, backup/restore, and migration startup are the consuming surfaces. Behavior must be consistent across Live/Replica/Mock.
5. **Feedback loop:** User corrections/corroborations/dismissals are unchanged. Correction preservation during rebuild belongs to W3 + DOS-832, not DOS-831.

---

## §7 K-In Findings

- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` - Consume the SQLite WAL contract: one file-level writer; dropping SQLCipher does not justify new writer connections or bypassing the pool. Also consume the documented SQLCipher reader-CPU cost.
- `.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md` - Currently authoritative for SQLCipher, Keychain key, `PRAGMA key` first, encrypted backups, and recovery screen. DOS-831 must amend/supersede this explicitly.
- `.docs/decisions/0071-schema-migration-framework.md` - Preserve schema versioning, forward-compatibility, and pre-migration backup behavior. Avoid adding a schema migration unless a durable marker is truly required.
- `.docs/decisions/0068-oauth-pkce-keychain-hardening.md` - Keychain remains canonical for OAuth/token storage. DB-key retirement is not a general Keychain rollback.
- `.docs/decisions/0094-audit-log-and-enterprise-observability.md` - Audit and diagnostic output must retain PII hygiene and stop implying DB-key access if that path disappears.
- `.docs/decisions/0104-execution-mode-and-mode-aware-services.md` - `ExecutionMode` is separate from DB-mode isolation. Cipher-drop must not re-open Live/Replica/Mock semantics.
- `.docs/decisions/0116-tenant-control-plane-boundary.md` - Future key-provider/control-plane seams separate DB key material from other secrets. DOS-831 retires only the local DB SQLCipher key path.
- `.docs/decisions/0133-writer-queue-responsibility.md` - Preserve single mutating connection/process, writer queue semantics, and open/write-path lints. The SQLCipher pragma exception should disappear rather than become a broader exception.
- `.docs/decisions/0134-reader-pool-sizing.md` - SQLCipher decryption cost is independent of reader pool sizing. Removing SQLCipher changes one residual cost but does not settle reader ownership policy.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` - K-in must search the substrate primitives, not only "SQLCipher drop." This packet cites DB open guard, DB-mode resolver, backup path, key provider, and writer substrate.
- `.docs/plans/v1.4.9-replica-db-l0-plan.md` / `.html` - D5 was intentionally decoupled from DB-mode isolation and requires its own ADR-0092 amendment. DOS-831 must not re-open DOS-820's Live/Replica/Mock decisions.
- `.docs/plans/v1.4.9-waves.md` - Records the FileVault/local trust direction and ADR sub-decision requirement. The active W1b text now points DOS-831 at an ADR-0092 amendment and fail-loud + rebuild instead of ADR-0136 and split-build decrypt/sentinel.
- Linear DOS-848 - The logical extraction repair is evidence that automatic conversion/repair of damaged encrypted DBs is not trustworthy. DOS-831 should fail loud and point to rebuild/restore rather than adding another fragile startup repair.
- L0 security-lens review - Export copies are plaintext egress outside the active DB path; DOS-831 must cover `export_database_copy` and Settings/recovery copy, not only internal backups.
- L0 adversarial document review - DOS-831 must close release/cutover gating, migration-slot freshness, DB-mode ADR ownership, frontend recovery gates, writer-bypass gates, and diagnostic-key re-sourcing before L1.

---

## §8 Reviewer Dispatch

Required L0 panel:

- `/codex challenge` - adversarial plan review because this supersedes an accepted security ADR and touches storage startup.
- `ce-feasibility-reviewer` - implementation reality across DB open/backup/migration paths.
- `ce-security-lens-reviewer` - explicit security/trust posture review for dropping app-layer encryption.
- `ce-learnings-researcher` - mandatory K-in confirmation over `docs/solutions/` and `.docs/decisions/`.

Because this touches storage security, filesystem paths, Keychain behavior, and DB open chokepoints, add plan-level CSO review if the L0 panel does not already cover the same threat-model questions in enough detail.

Review questions:

1. Does the ADR amendment state the security trade-off honestly?
2. Does the plan avoid a hidden in-place migration or fragile encrypted-DB salvage path?
3. Are DB-mode and single-writer guarantees preserved?
4. Are non-DB Keychain surfaces protected from collateral deletion?
5. Are backup/restore and migration safety still real after removing destination keying?
6. Does the release/cutover gate prevent a plain build from stranding encrypted active DBs before DOS-832 is available?
7. Are exported DB copies handled as plaintext egress with destination-boundary copy?

---

## §9 Definition of Done

- L0 packet approved unanimously and mirrored to Linear.
- ADR-0092 amendment committed.
- DB-mode-isolation ADR/addendum committed, or active wave plan amended before L1 to move that deliverable.
- Runtime opens, read-only opens, migration backups, manual backups, and restore validation run as plain SQLite with no DB encryption key dependency.
- User-selected export copies are plain SQLite, permission-hardened where possible, and surfaced with destination-boundary copy.
- Encrypted-looking active DB fails loud with restore/rebuild guidance.
- Release/cutover gate prevents DOS-831 from shipping into an environment with an encrypted active DB before DOS-832 rebuild is available, unless an explicit storage-reset cutover confirms safety.
- Keychain DB key is retired from runtime but not deleted automatically.
- DB-key-derived audit/workspace diagnostic helpers are re-sourced without touching the retired SQLCipher key.
- Focused tests and full gates pass.
- PR targets `dev`, links DOS-831, includes `security_auditor_invoked: true`, and carries the correct `L2-status` line.
