# L0 Packet — v1.4.5 W1-B — DOS-464 Workspace Source Registry + Path Validation

**Current revision:** V1.2 (cycle 2 fold, 2026-05-20). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-20
- **Project:** v1.4.5 — Workspace Memory Refactor ([Linear](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102))
- **Wave:** W1 stage 1b (gates on W1-A merge; runs parallel with W1-C)
- **Issue:** [DOS-464 — Add workspace document registry migration](https://linear.app/a8c/issue/DOS-464)
- **Branch (proposed):** `feat/dos-464-workspace-registry` from `wave/v1.4.5-w1-stage1a` after W1-A merges
- **Migration slot claimed:** **v252** (from v1.4.5 W1 block v250–v254 per wave-plan §Cycle 11)
- **L0 reviewer matrix:** architect-reviewer + codex challenge + codex consult + **`/cso` (security-auditor) — mandatory**
- **L2 reviewer matrix:** codex review + code-reviewer + architect-reviewer + **`/cso` re-review**

## 2. Changelog

- **V1.2 (2026-05-20 — cycle 2 fold):** Cycle 2 returned architect APPROVE (clean) + `/cso` CONDITIONAL APPROVE (3 new LOW/MEDIUM nits → path-α) + codex challenge BLOCK (4 substantive: hardlink defense FALSE, symlink fixture conflicts algorithm, CI lint unscoped, reserved-name regex inconsistent) + codex consult BLOCK (5 substantive, 4 overlap). The class is genuine security architecture — codex reviewers correctly identified that V1.1's hardlink defense was overclaim, and the symlink algorithm + fixture had a logical contradiction. Per memory `feedback_reviewer_dissent_is_signal`, dissent wins. Folds:
  1. **Hardlink defense corrected via `st_nlink > 1 → SymlinkRefused`** (challenge #1 + consult #1): cycle-1 fold #7 incorrectly claimed `fstat.dev == workspace_root_dev` caught same-device hardlinks-into-workspace. Codex correctly noted: an in-workspace path that is a hardlink to outside same-device content has in-workspace `canonical_path`, identical lstat/fstat dev+ino (the inode is shared), and passes ALL V1.1 checks while serving outside content. The actual defense is to refuse files with `st_nlink > 1` in the workspace at validation time — any hardlinked file in the workspace is suspect. Trade-off: legitimate hardlinks (rare in modern document workflows) are rejected. §7 step 5 adds: `if fstat.nlink > 1 → SymlinkRefused` (named broadly to capture the path-aliasing class; future variant rename to `HardLinkRefused` filed as Codebase Maintenance per architect F5/V1.1).
  2. **Symlink algorithm + fixture reconciliation** (challenge #2 + consult #2): cycle-1 V1.1 algorithm (canonicalize FIRST then `O_NOFOLLOW`) makes `O_NOFOLLOW` vacuous because `canonicalize`/`realpath` already resolves all symlinks. After canonicalize, no symlinks remain to refuse. V1.2 keeps the canonicalize-first approach (simpler, matches POSIX best practice) and **removes `O_NOFOLLOW` from step 4** (no longer load-bearing). Fixture expectations corrected: outside-symlink (fixture #5) → `OutsideWorkspace` (canonicalize catches; not `SymlinkRefused`); symlink chain >1 hop (fixture #13) → `OutsideWorkspace` at canonicalize (chain resolves to outside target). The `SymlinkRefused` variant is repurposed for the V1.2 fold #1 hardlink rejection only.
  3. **CI lint gate scoped to ingestion-call-site files** (challenge #3 + consult #5): cycle-1 V1.1 made the gate repo-wide which would trip existing non-ingestion callsites. V1.2 scopes the gate to a positive allowlist:
     - **Forbidden in:** `src-tauri/src/services/workspace_ingestion/{pipeline,extract,registry}.rs`, `src-tauri/src/processor/**`, `src-tauri/src/watcher.rs`, `src-tauri/src/google_drive/poller.rs` + `sync.rs`, `src-tauri/src/granola/poller.rs`, `src-tauri/src/quill/poller.rs` — these are the ingestion code paths the wave plan §Architecture invariants identifies.
     - **Allowed entry point:** `services::workspace_ingestion::registry::open_validated` only.
     - Existing pre-v1.4.5 `fs::File::open` callsites elsewhere in the repo are grandfathered (not in scope).
     - Allowlist marker: `// workspace-path-allowed: <one-line rationale>` (V1.1 fold #10 retained).
  4. **Reserved-name regex inconsistency reconciled** (challenge #4 + consult #3): the slug regex `^[a-z][a-z0-9_-]{0,31}$` accepts lowercase `"con"` which is a Windows reserved name. Since Windows path validation is deferred to follow-up ticket (V1.1 fold #19), the Windows-reserved-name slug check is also deferred. V1.2 §8 explicitly notes: the test asserting `register_other(Account, "CON") → MalformedSlug` rejects on the uppercase regex constraint, NOT on Windows-reserved-name semantics. The case-INSENSITIVE Windows-reserved check lands in the Windows follow-up ticket together with the path validation work.
  5. **EntityType::Other explicit handling** (consult #4): V1.2 §4 + §6 + §8 document that `crate::entity::EntityType::Other` is intentionally non-canonical for workspace category routing. `validate(category, EntityType::Other) → Err(CategoryNotAllowed { allowed: vec![] })` — Other-typed entities cannot bind workspace files in v1.4.5 (deferred to follow-up). `resolve_path(EntityType::Other, …)` returns Err via the validate gate. V1.2 §8 adds `entity_type_other_rejects_category_validation` test.
  6. **§7 fixtures #10-13 (NAME_MAX, trailing-dot, ADS) explicitly enumerated** (consult #3): V1.1 changelog claimed them but the fixture list collapsed. V1.2 §7 enumerates each with expected `RejectionReason` variant per platform.
  7. **NFKC normalization for registry-key/slug** (security SEC-W1B-010 from /cso cycle 2): V1.2 §7 specifies NFKC for path-component registry-key normalization (collapses compatibility-equivalent forms like fullwidth → ASCII); NFC retained for file-content paths. The `unicode-normalization` crate supports both.
  8. **Adversarial concurrency fixture** (security SEC-W1B-011): V1.2 §7 adds fixture #14b — 1 attacker thread renaming target symlink in tight loop while N=8 readers call `open_validated`; every result is `Ok(identical FileIdentity)` or `Err(SymlinkRaced)`, never `Ok` with swapped inode.
  9. **Btrfs subvolume / APFS firmlink residual** (security SEC-W1B-009): filed as Codebase Maintenance follow-up (V1.2 doesn't change the algorithm; just documents the cross-device check is load-bearing, not the hardlink-impossibility claim).
  10. **Changelog overstatement** (consult #3): V1.1 fold #12 claimed fixtures that V1.2 now actually enumerates in §7 (fold #6 above).

- **V1.1 (2026-05-20 — cycle 1 fold):** Cycle 1 returned architect CONDITIONAL APPROVE (4 findings), `/cso` BLOCK (2 HIGH + 4 MEDIUM + 2 LOW = 8 findings), codex challenge BLOCK (5 findings), codex consult BLOCK (6 findings). After dedup, ~20 unique findings across 4 classes. Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop` the security-architecture class fires across 3 reviewers — V1.1 rewrites §7 from first principles rather than patching. Folds:
  1. **`data_source_json` seed serde shape corrected** (challenge #1 + consult #1 + bg-grep): canonical externally-tagged enum produces `{"workspace_file":{"kind":"inbox"}}`, NOT `{"kind":"workspace_file","name":{"kind":"inbox"}}`. §6 seed INSERT statements rewritten to canonical shape; §8 adds explicit serde-round-trip test against the live `DataSource` enum for every seeded row.
  2. **`EntityType` uses canonical `crate::entity::EntityType` from `src-tauri/src/entity.rs:13`** (architect F1 + challenge #3 + consult #2): pre-grepped; the canonical type has variants `Account | Project | Person | Other` with `#[serde(rename_all = "snake_case")]`. W1-B does NOT define a parallel `EntityType`. `validate(category: &WorkspaceCategory, entity_type: crate::entity::EntityType) -> Result<(), CategoryNotAllowed>`.
  3. **`resolve_path` takes `entity_type` + handles `_inbox` sentinel** (architect F1 + consult #3 + challenge #2): signature is `resolve_path(entity_type: crate::entity::EntityType, entity_name: &str, category: Option<&WorkspaceCategory>, filename: &str, source_type: contracts::WorkspaceFileKind) -> PathBuf`. Path rules: `Inbox` source → `_inbox/{filename}` regardless of entity; otherwise routes by `EntityType` to `Accounts/{entity_name}/{category}/{filename}` | `People/{entity_name}/{category}/{filename}` | `Projects/{entity_name}/{category}/{filename}` (category omitted → entity root). Per cycle 8 + cycle 12 wave-plan amendments.
  4. **`CategoryNotAllowed` error type declared in §4 + §6** (architect F3 + challenge #3): defined in `registry.rs` as `pub struct CategoryNotAllowed { category: String, entity_type: crate::entity::EntityType, allowed: Vec<String> }`. Distinct from `PlacementError::CategoryNotAllowed` (W4-C) which carries the same name but different envelope.
  5. **`register_other` added to §4 API surface + regex pinned** (architect F4 + security SEC-008): API in §4: `register_other(entity_type: crate::entity::EntityType, slug: &str) -> Result<(), RegisterError>`. Lex regex pinned: `^[a-z][a-z0-9_-]{0,31}$` (max 32 chars; starts with lowercase letter; ASCII `[a-z0-9_-]+` thereafter). RegisterError variants: `MalformedSlug { slug: String }`, `EntityTypeUnknown`, `DbError(String)`. SQL injection vector explicitly closed via parameterized rusqlite `?` binding (called out in security gate).
  6. **TOCTOU step 4 rewritten** (security SEC-001 HIGH + architect F2 + challenge #4): the original "compare pre-canonicalize identity vs post-open identity" was unimplementable because canonicalize returns a `PathBuf`, not a stat. V1.1 replaces with: **(a)** canonicalize → check strict-child-of-workspace-root + per-component `..` rejection; **(b)** record `lstat`-result device+inode of canonical path AFTER canonicalize (sat call against the path); **(c)** open with `O_NOFOLLOW` (Unix); **(d)** `fstat` the open `File` handle; **(e)** assert `(fstat.dev, fstat.ino) == (lstat.dev, lstat.ino)` — if not, return `SymlinkRaced`. Closes the actual TOCTOU window (race between path validation and open).
  7. **Hardlink vector handled** (security SEC-002 HIGH): post-open contract adds **(f)** `fstat.dev == workspace_root_dev` check — if not, return `SymlinkRefused` (cross-device hard links are impossible by Unix design; same-device hard links from outside the workspace into the workspace are caught by canonicalize being strict-child + lstat dev/ino check matching post-open fstat). The fixture suite adds an explicit hardlink-to-`/etc/passwd` fixture asserting `SymlinkRefused`.
  8. **Bind-mount / cross-device escape handled** (security SEC-003): covered by step (f) above. Negative fixture: bind-mount tmpfs over a workspace subdir → workspace_root_dev mismatch → `SymlinkRefused` (renaming the variant or adding `OutsideWorkspaceDevice` is path-α maintenance; for V1.1 we reuse `SymlinkRefused` since the user-visible failure mode is identical).
  9. **Case-insensitive FS ambiguity** (security SEC-004): V1.1 adds fixture for `Accounts/Acme/presentations/x` vs `Accounts/Acme/Presentations/x` on case-insensitive FS (macOS APFS default, Windows NTFS). Decision: registry lookup key is always the lowercase canonical slug per `WorkspaceCategory::as_slug()`; canonicalize result on case-insensitive FS may return mixed-case but the lookup normalizes via `to_lowercase()` BEFORE registry query. NFC normalization handled via `unicode-normalization` crate on the filename component before registry validation (added as a dev-dep).
  10. **CI lint gate broadened** (security SEC-005): `check_workspace_path_validation.sh` updated to a positive-allowlist approach. Forbidden patterns: `std::fs::File::open`, `std::fs::OpenOptions::*::open`, `std::fs::read*`, `std::fs::metadata`, `std::fs::symlink_metadata`, `std::fs::read_link`, `std::fs::read_dir`, `tokio::fs::*`, `tokio::fs::OpenOptions`, `memmap2::Mmap::map`. Allowed entry points enumerated: `registry::open_validated` ONLY. Allowlist exception marker changed from stale `dos-146-allowed` to neutral `// workspace-path-allowed: <rationale>` (challenge #5).
  11. **Quill + Granola transcripts default_sensitivity bumped to `'confidential'`** (security SEC-006): both source types contain raw meeting transcription with PII / health / comp details. Per ADR-0125 + CLAUDE.md "No customer-specific data in source code", the safe default is `confidential`. The other 5 source types remain `'internal'`.
  12. **Special-filename + path-length fixtures added** (security SEC-007): NUL byte in path → typed rejection; Windows reserved names (`CON`/`PRN`/`AUX`/`NUL`/`COM1-9`/`LPT1-9`) as `register_other` slug → `MalformedSlug`; PATH_MAX overflow (>4096) → `PathTraversalAttempt` (treating as malformed input); NAME_MAX overflow (>255 per component) → `PathTraversalAttempt`; trailing dot / trailing space on Windows → `MalformedSlug` via the regex; NTFS ADS (`file.txt:hidden`) → fixture but Unix-only test gated.
  13. **Strict-child root-equality fixture added** (challenge #4): canonical_path == workspace_root → `OutsideWorkspace` (root is not a valid file target). Symlink chain >1 hop fixture (security gap): symlink A → symlink B → outside-workspace target → `SymlinkRefused` at the first hop's `O_NOFOLLOW`.
  14. **Symlink ordering documented** (consult #4): the canonicalize step happens FIRST; if a symlink resolves out of workspace, `OutsideWorkspace` is returned (NOT `SymlinkRefused`). `SymlinkRefused` is returned only when a symlink is encountered DURING `O_NOFOLLOW` open of a path that previously canonicalized within workspace (i.e., the directory component containing the file IS a symlink, and `O_NOFOLLOW` refuses the open). The fixture suite documents which variant fires per attack vector.
  15. **DataSource variant mapping round-trip test added** (consult #5): per wave-plan §W1-B done-when "DataSource variant mapping round-trip across all WorkspaceFileKind values", the test asserts that for each of the 7 `WorkspaceFileKind` variants, the `data_source_json` seed value parses back to the canonical `DataSource::WorkspaceFile { kind }` via serde.
  16. **W1-A `no-substrate-reinvention` gate listed in §8** (consult #5): explicit dependency declared.
  17. **Concurrency positive test added** (security gap): N=8 parallel `open_validated` calls on the same valid path return identical `FileIdentity` triples.
  18. **Person entity category default-set corrected** (challenge #2): now seeds all 6 categories for Person (was 4); matches wave-plan default contract.
  19. **Windows platform decision** (architect F2 + security SEC-001 residual): V1.1 explicitly platform-gates the security boundary as **Unix-only at W1-A merge**, with Windows path validation deferred to a follow-up `feat/dos-464b-windows-path-validation` ticket. The Windows code-path returns `RejectionReason::OutsideWorkspace` with a documented "Windows path validation not implemented; W1-B Unix-only" message until that follow-up ships. Justification: DailyOS macOS-first per CLAUDE.md; Windows support is post-v1.4.x. Filed for Codebase Maintenance follow-up.
  20. **`workspace_source_registry` extension column for `default_sensitivity`** (security SEC-006 follow-on): added explicit per-row column for transcript-specific sensitivity bump per the fold above.

- **V1.0 (2026-05-20):** Initial packet. Drafted pre-W1-A-cycles substrate-grep but inherited the 6-cycle blowup patterns the cycle-1 fold corrects.

## 3. Goal (verbatim from wave plan §Agent W1-B)

Migrate the registry of known workspace source types (inbox, Drive, entity-doc, user-attachment, Granola transcript, Quill transcript) to the v1.4.1 `data_source` taxonomy. Establishes the allowlist of file origins the ingestion pipeline accepts and validates paths against. **Cycle 8 amendment (Option B-prime):** also ships `WorkspaceCategoryRegistry` — defines per-entity-type allowed `WorkspaceCategory` values and the path resolution rules `Accounts/{entity_name}/{category}/{filename}` when category present; `Accounts/{entity_name}/{filename}` when absent. Files in `_inbox/` resolve to `_inbox/{filename}` until entity assignment promotes them.

## 4. Files owned (exclusive)

### New migration
- `src-tauri/src/migrations.rs` — slot **v252** registration.
- `src-tauri/src/migrations/252_workspace_source_registry.sql` — source-allowlist + entity-category-registry tables (see §6 for shape).

### Module content
- `src-tauri/src/services/workspace_ingestion/registry.rs` — fills the W1-A-pre-created placeholder. Substantive content:
  - `WorkspaceSourceRegistry::open_validated(path: &Path) -> Result<(File, contracts::FileIdentity), contracts::RejectionReason>` per §7 security gate.
  - `WorkspaceCategoryRegistry::validate(category: &contracts::WorkspaceCategory, entity_type: crate::entity::EntityType) -> Result<(), CategoryNotAllowed>`.
  - `WorkspaceCategoryRegistry::resolve_path(entity_type: crate::entity::EntityType, entity_name: &str, category: Option<&contracts::WorkspaceCategory>, filename: &str, source_type: contracts::WorkspaceFileKind) -> PathBuf` per cycle 8 + V1.1 fold #3.
  - `WorkspaceCategoryRegistry::register_other(entity_type: crate::entity::EntityType, slug: &str) -> Result<(), RegisterError>` per V1.1 fold #5.
  - Error types: `pub struct CategoryNotAllowed { category: String, entity_type: crate::entity::EntityType, allowed: Vec<String> }`; `pub enum RegisterError { MalformedSlug { slug: String }, EntityTypeUnknown, DbError(String) }`.
  - Constant `SLUG_REGEX: &str = r"^[a-z][a-z0-9_-]{0,31}$"`.

### Tests
- `src-tauri/tests/workspace_registry_open_validated.rs` — security negative-fixture suite (see §8).
- `src-tauri/tests/workspace_registry_category.rs` — `WorkspaceCategoryRegistry::{validate, resolve_path, register_other}` tests.
- `src-tauri/tests/workspace_registry_data_source_roundtrip.rs` — V1.1 fold #15: every seeded `data_source_json` parses back to canonical `DataSource::WorkspaceFile { kind }` for all 7 `WorkspaceFileKind` variants.

### NOT touched (deny list)
- `src-tauri/src/services/workspace_ingestion/{mod,contracts,lifecycle,runs,link,pipeline,extract,signals,graph,wiring}.rs` — W1-A or other-lane territory.
- `src-tauri/src/services/claims.rs`, `src-tauri/src/signals/**`, processor/**, watcher.rs, google_drive/**, granola/**, quill/**.
- `abilities-runtime/**` — consume canonical types; never modify substrate.
- `src-tauri/src/entity.rs` — consume `EntityType`; never modify (substrate elsewhere).

## 5. Contracts referenced + K-in citations

### Substrate prerequisites verified live on `wave/v1.4.5-w1-stage1a` (V1.1 re-grep 2026-05-20)

| Contract | Location | W1-B relationship |
|---|---|---|
| `contracts::FileIdentity` | `src/services/workspace_ingestion/contracts.rs:42` (W1-A) | **consumes** (return type of `open_validated`) |
| `contracts::RejectionReason` | `…/contracts.rs:113` (W1-A; 6 variants — all consumed) | **consumes**; W1-B emits `PathTraversalAttempt`, `SymlinkRefused`, `SymlinkRaced`, `OutsideWorkspace` |
| `contracts::WorkspaceCategory` | `…/contracts.rs:54` (W1-A) | **consumes** (`validate` argument) |
| `contracts::WorkspaceFileKind` | `…/contracts.rs` `pub use` re-export | **consumes** (`resolve_path` argument + registry-row enumeration) |
| **`crate::entity::EntityType`** | `src-tauri/src/entity.rs:13` (variants `Account | Project | Person | Other`, snake_case serde) | **consumes** (V1.1 fold #2; NOT reinvented) |
| `DataSource::WorkspaceFile { kind }` (externally-tagged) | `abilities-runtime/src/abilities/provenance/source.rs:81` | **consumes** (V1.1 fold #1: canonical serde shape `{"workspace_file":{"kind":"inbox"}}`) |
| Highest registered migration on dev | `migrations.rs:927` (v240) + W1-A v250 + v251 | W1-B claims **v252** per cycle 11 |

### Anti-reinvention pre-grep (V1.1 re-grep after cycle 1 substrate-class findings)

Re-verified zero collisions on all V1.1-introduced names. `WorkspaceSourceRegistry`, `WorkspaceCategoryRegistry`, `CategoryNotAllowed`, `RegisterError` are net-new W1-B primitives. Note: `PlacementError::CategoryNotAllowed` exists at wave-plan W4-C (DOS-474) — these are deliberately distinct types in different layers (registry-level vs placement-API-level), tracked as future maintenance if convergence is wanted post-W4-C.

W1-A's `tests/workspace_ingestion_no_substrate_reinvention.rs` CI gate will catch any future drift in `services/workspace_ingestion/registry.rs`. V1.1 §8 explicitly lists this gate as a required passing test.

### K-in `docs/solutions/` + `.docs/decisions/` grep

Re-run V1.1: `WorkspaceSourceRegistry`, `WorkspaceCategoryRegistry`, `open_validated`, `path traversal`, `O_NOFOLLOW`, `TOCTOU`, `symlink validation`, `hardlink`, `bind-mount`. **Zero hits.** ADR anchors: ADR-0098 (source-aware lifecycle), ADR-0107 (source taxonomy + W0 amendment). No K-in BLOCKED finding.

## 6. v252 migration column shape

```sql
-- v1.4.5 W1-B — workspace source registry: source-type allowlist + per-entity-type category registry.

CREATE TABLE IF NOT EXISTS workspace_source_registry (
    source_type             TEXT PRIMARY KEY,           -- canonical WorkspaceFileKind serde-tag (snake_case)
    data_source_json        TEXT NOT NULL,              -- JSON-serialized DataSource::WorkspaceFile{kind} per externally-tagged enum
    default_sensitivity     TEXT NOT NULL,              -- 'internal' | 'confidential' | …
    allowed                 INTEGER NOT NULL DEFAULT 1,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- V1.1 fold #1: canonical serde-produced shape for externally-tagged DataSource::WorkspaceFile{kind}.
-- V1.1 fold #11: Granola + Quill transcripts default to 'confidential' (PII-bearing).
INSERT OR IGNORE INTO workspace_source_registry (source_type, data_source_json, default_sensitivity) VALUES
    ('inbox',              '{"workspace_file":{"kind":"inbox"}}',              'internal'),
    ('entity_doc',         '{"workspace_file":{"kind":"entity_doc"}}',         'internal'),
    ('drive_sync',         '{"workspace_file":{"kind":"drive_sync"}}',         'internal'),
    ('user_attachment',    '{"workspace_file":{"kind":"user_attachment"}}',    'internal'),
    ('granola_transcript', '{"workspace_file":{"kind":"granola_transcript"}}', 'confidential'),
    ('quill_transcript',   '{"workspace_file":{"kind":"quill_transcript"}}',   'confidential'),
    ('mcp_placement',      '{"workspace_file":{"kind":"mcp_placement"}}',      'internal');

CREATE TABLE IF NOT EXISTS workspace_category_registry (
    entity_type             TEXT NOT NULL,              -- 'account' | 'person' | 'project' (snake_case per crate::entity::EntityType)
    category_slug           TEXT NOT NULL,              -- canonical WorkspaceCategory::as_slug() output
    allowed                 INTEGER NOT NULL DEFAULT 1,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (entity_type, category_slug)
);

-- V1.1 fold #18: all 6 categories for each entity type (was 4 for person; corrected).
INSERT OR IGNORE INTO workspace_category_registry (entity_type, category_slug) VALUES
    ('account', 'presentations'), ('account', 'transcripts'), ('account', 'meetings'),
    ('account', 'notes'),         ('account', 'contracts'),   ('account', 'attachments'),
    ('person',  'presentations'), ('person',  'transcripts'), ('person',  'meetings'),
    ('person',  'notes'),         ('person',  'contracts'),   ('person',  'attachments'),
    ('project', 'presentations'), ('project', 'transcripts'), ('project', 'meetings'),
    ('project', 'notes'),         ('project', 'contracts'),   ('project', 'attachments');

CREATE INDEX IF NOT EXISTS idx_wcr_entity_type ON workspace_category_registry (entity_type);
```

`Other(slug)` categories runtime-registered via `WorkspaceCategoryRegistry::register_other` after lex-shape validation (regex `^[a-z][a-z0-9_-]{0,31}$`). SQL injection vector closed via parameterized rusqlite `?` binding.

## 7. Security gate (mandatory `/cso` review) — V1.1 rewrite

**Trust boundary:** every workspace path entering the ingestion pipeline passes through `WorkspaceSourceRegistry::open_validated`. Failures route through typed `contracts::RejectionReason` variants and never proceed to read.

### V1.1 `open_validated` contract (Unix-only at v1.4.5; Windows deferred — fold #19)

```rust
pub fn open_validated(path: &Path) -> Result<(File, contracts::FileIdentity), contracts::RejectionReason> {
    // V1.2 algorithm — canonicalize-first; O_NOFOLLOW removed (vacuous post-canonicalize);
    // hardlink defense via st_nlink>1 refusal at fstat time.
    //
    // Step 1: per-component rejection of `..` BEFORE canonicalize (defense in depth against
    //         canonicalize bugs + per-component validation closes URL-encoded variants like
    //         %2e%2e and Unicode equivalents). For path components used as registry keys,
    //         apply NFKC normalization via `unicode-normalization` crate (V1.2 fold #7 per
    //         /cso SEC-W1B-010). For file-content path bytes, NFC only.
    // Step 2: canonicalize path; assert it is a strict child of WORKSPACE_ROOT (no equality
    //         with root; no escape). Equality with root → OutsideWorkspace. If canonicalize
    //         resolves through a symlink chain to outside workspace → OutsideWorkspace.
    //         (canonicalize/realpath resolves all symlinks; after this step no symlinks
    //         remain in the path. V1.2 fold #2 removes the V1.1 O_NOFOLLOW open since it
    //         is vacuous after canonicalize.)
    // Step 3: lstat the canonical path; record (lstat.dev, lstat.ino).
    //         Assert lstat.dev == workspace_root_dev (cross-device escape: bind-mount over
    //         subdir, or hardlink-into-workspace from another mount, both rejected here).
    //         Per /cso SEC-W1B-009: the cross-device check is the load-bearing primitive;
    //         hardlink-impossibility is a corollary, not the gate.
    // Step 4: open the canonical path (plain open(2); no O_NOFOLLOW since canonicalize has
    //         already resolved all symlinks).
    // Step 5: fstat the open File handle.
    //         (a) Assert (fstat.dev, fstat.ino) == (lstat.dev, lstat.ino) from step 3.
    //             Mismatch → SymlinkRaced (TOCTOU race: attacker swapped target between
    //             lstat and open; the post-open inode check closes the window).
    //         (b) Assert fstat.dev == workspace_root_dev (defense in depth against step 3
    //             lstat race; cross-device fail-safe).
    //         (c) V1.2 fold #1: assert fstat.nlink == 1.
    //             nlink > 1 means the inode has another directory entry somewhere — that
    //             entry might be outside the workspace with arbitrary content. Refuse all
    //             multi-link files in workspace as SymlinkRefused (variant repurposed for
    //             the path-aliasing class; future rename to HardLinkRefused is path-α
    //             Maintenance). Trade-off: legitimate hardlinks rejected; DailyOS workspace
    //             documents don't use hardlinks in normal workflows.
    // Step 6: return (File, FileIdentity { canonical_path, device: fstat.dev, inode: fstat.ino }).
    // Windows: returns RejectionReason::OutsideWorkspace with a "platform not supported" log
    // until follow-up ticket (V1.1 fold #19). The follow-up will also cover the Windows
    // case-INSENSITIVE reserved-name slug check (V1.2 fold #4).
}
```

### Negative fixture suite (V1.2 — 17 fixtures, all return typed `RejectionReason` with zero bytes read)

1. **TOCTOU race**: validate → atomically swap symlink target between lstat and open → `SymlinkRaced`.
2. **URL-encoded path traversal**: `%2e%2e/escape` → `PathTraversalAttempt`.
3. **NFC/NFD Unicode normalization**: `\u{30CF}\u{309A}/../escape` equivalents → `PathTraversalAttempt`.
4. **Absolute path**: `/etc/passwd` → `OutsideWorkspace`.
5. **V1.2 — Symlink final-component pointing outside workspace**: → `OutsideWorkspace` (canonicalize resolves and catches; V1.2 corrects V1.1's `SymlinkRefused` claim).
6. **Bare `..` component**: `notes/../escape` → `PathTraversalAttempt`.
7. **V1.2 — hardlink to `/etc/passwd`**: in-workspace path that is a hardlink to outside (or to any other file regardless of device) → `SymlinkRefused` (`fstat.nlink > 1` per V1.2 fold #1).
8. **V1.2 — bind-mount tmpfs over workspace subdir**: workspace-relative path inside the bind mount → `OutsideWorkspace` (canonicalize on the bind-mount path returns the bind-mount root which fails strict-child OR lstat.dev mismatch fires).
9. **Case-insensitive lookup**: `Accounts/Acme/Presentations/x` vs `…/presentations/x` on case-insensitive FS → both succeed via NFKC + lowercase normalization at the registry-key layer.
10. **NUL byte in path**: `foo\0bar` → kernel-level rejection mapped to `PathTraversalAttempt`.
11. **V1.2 — NAME_MAX per-component overflow**: any single component >255 bytes → `PathTraversalAttempt`.
12. **PATH_MAX overflow**: total path >4096 bytes → `PathTraversalAttempt`.
13. **V1.2 — trailing-dot / trailing-space component**: `foo./bar` / `foo /bar` → `PathTraversalAttempt` (Windows-context attack; Unix-test gated as defense-in-depth even on Unix).
14. **V1.2 — NTFS ADS (`file.txt:hidden`)**: → `PathTraversalAttempt` (Unix-test asserts the `:` character is rejected at lex stage; Windows-specific handling deferred to follow-up).
15. **Strict-child root equality**: `canonical_path == WORKSPACE_ROOT` → `OutsideWorkspace`.
16. **V1.2 — symlink chain >1 hop**: A → B → outside → `OutsideWorkspace` at canonicalize (chain resolves to outside target; V1.2 corrects V1.1's `SymlinkRefused` claim).
17. **V1.2 — concurrency adversarial (security SEC-W1B-011)**: 1 attacker thread renaming target symlink in a tight loop while N=8 readers call `open_validated`; every result is either `Ok(identical FileIdentity)` or `Err(SymlinkRaced)`, never `Ok` with a swapped inode.
18. **Concurrency positive**: N=8 parallel `open_validated` calls on same valid path return identical `FileIdentity`.
19. **Positive**: valid workspace-relative path returns `(File, FileIdentity)` with `fstat`-derived device+inode + `fstat.nlink == 1`.

### CI lint gate (V1.2 fold #3 — scoped to ingestion-call-site files only)

`src-tauri/scripts/check_workspace_path_validation.sh` enforces a positive allowlist within an explicit file-path scope (V1.1's repo-wide scope would trip pre-existing non-ingestion callsites):

- **Scope (forbidden patterns checked only in these files):** `src-tauri/src/services/workspace_ingestion/{pipeline,extract,registry}.rs`, `src-tauri/src/processor/**`, `src-tauri/src/watcher.rs`, `src-tauri/src/google_drive/{poller,sync}.rs`, `src-tauri/src/granola/poller.rs`, `src-tauri/src/quill/poller.rs` (the wave-plan §Architecture-invariants enumerated ingestion code paths).
- **Forbidden patterns:** `std::fs::File::open`, `std::fs::OpenOptions::*::open`, `std::fs::read*`, `std::fs::metadata`, `std::fs::symlink_metadata`, `std::fs::read_link`, `std::fs::read_dir`, `tokio::fs::*`, `tokio::fs::OpenOptions::*`, `memmap2::Mmap::map`, any `BufReader::new(File::open(...))` / `BufReader::new(OpenOptions::*...)`.
- **Allowed entry point:** `services::workspace_ingestion::registry::open_validated` only.
- **Allowlist marker:** `// workspace-path-allowed: <one-line rationale>`.
- Existing pre-v1.4.5 `fs::*::open` callsites in non-ingestion code (e.g., backup utilities, signing key reads, MCP transport) are out of scope; they are explicitly grandfathered. New code in those areas is unconstrained by this gate.

### Sensitivity

V1.1 fold #11: Granola + Quill transcripts default to `'confidential'`; others `'internal'`. Per ADR-0125 + memory `feedback_no_pii_in_commit_messages`.

### Windows path validation

V1.1 fold #19: deferred. Unix-only at v1.4.5. Windows path returns `OutsideWorkspace` with log. Follow-up ticket `feat/dos-464b-windows-path-validation` filed to Codebase Maintenance.

## 8. Tests required

### Security suite (Suite S contributors; mandatory `/cso` re-review at L2)

- `tests/workspace_registry_open_validated.rs` — all 15 fixtures in §7 (13 negative + 2 positive). Each negative asserts zero bytes read.
- `check_workspace_path_validation.sh` CI lint (initially vacuous; activates W2-A merge).
- `tests/workspace_ingestion_no_substrate_reinvention.rs` — W1-A's CI grep gate (V1.1 fold #16: explicit dependency).

### Registry / category tests

- `tests/workspace_registry_category.rs`:
  - `validate(category, entity_type)` returns `Ok(())` for every default `(EntityType, WorkspaceCategory)` pair seeded in v252 (18 pairs after V1.1 fold #18).
  - `validate(Other("new_slug"), Account)` returns `Err(CategoryNotAllowed { allowed: [...] })` until `register_other` is called; allowed list populated from current registry.
  - `register_other(Account, "new_slug")` → INSERT succeeds; second `validate` call passes.
  - `register_other(Account, "WithUpper")` → `Err(MalformedSlug)` (regex rejects uppercase).
  - `register_other(Account, "with space")` → `Err(MalformedSlug)`.
  - `register_other(Account, "x".repeat(33))` → `Err(MalformedSlug)` (length limit).
  - `register_other(Account, "CON")` → `Err(MalformedSlug)` (regex `^[a-z]...` accepts; but CON test is Unix-context; Windows reserved-name check is in path validation, not slug regex).
  - `resolve_path(Account, "Acme", Some(&Presentations), "q1.pdf", EntityDoc)` → `"Accounts/Acme/presentations/q1.pdf"`.
  - `resolve_path(Person, "Bob", Some(&Notes), "1on1.md", EntityDoc)` → `"People/Bob/notes/1on1.md"`.
  - `resolve_path(Project, "Apollo", None, "design.pdf", EntityDoc)` → `"Projects/Apollo/design.pdf"` (no category sub-dir).
  - `resolve_path(Account, "_unused_", None, "drop.md", Inbox)` → `"_inbox/drop.md"` (source_type=Inbox overrides entity routing per cycle-8 + V1.1 fold #3).
- `tests/workspace_registry_data_source_roundtrip.rs` (V1.1 fold #15): each of 7 `WorkspaceFileKind` variants — parse `data_source_json` from registry → assert equals canonical `DataSource::WorkspaceFile { kind }` via serde.
- Migration round-trip: v252 applies cleanly after v251 (W1-A); pre-seeded rows present after migration; PRAGMA `table_info` matches §6.
- `cargo clippy --lib -- -D warnings` clean.

## 9. Done when

- Migration slot **v252** used; `workspace_source_registry` + `workspace_category_registry` tables exist; 7 source rows + 18 category rows pre-seeded.
- `services/workspace_ingestion/registry.rs` substantively filled with the 4 APIs (`open_validated`, `validate`, `resolve_path`, `register_other`) + 2 error types (`CategoryNotAllowed`, `RegisterError`) + `SLUG_REGEX` constant.
- All 15 §7 security fixtures pass with typed `RejectionReason` variants (Unix; Windows skipped).
- All registry/category tests in §8 pass.
- `tests/workspace_ingestion_no_substrate_reinvention.rs` (W1-A's CI gate) still passes — `registry.rs` defines no `pub struct/enum/trait` matching the canonical-primitives blocklist.
- `data_source_json` serde round-trip green for all 7 variants.
- `cargo clippy --lib -- -D warnings && cargo test` green.
- IL gate items 1–5 answered in commit message.
- `L2-status: passed` declared.
- `/cso` L0 plan AND L2 diff approval recorded.
- L0 verdict posted as Linear comment on DOS-464.

## 10. Intelligence Loop gate

1. **Claim model.** Registry rows are metadata; W1-B commits no claims.
2. **Provenance + trust.** Each `workspace_source_registry` row carries `data_source_json` (canonical externally-tagged `DataSource::WorkspaceFile { kind }`) + `default_sensitivity` (consumed by claim commit per ADR-0125; V1.1 fold #11 bumps PII-bearing transcripts).
3. **Signals + invalidation.** W1-B emits no signals; `RejectionReason` typed errors route to W2-A pipeline boundary (per W1-A cycle 4 wave-timing fix).
4. **Runtime + surfaces.** Registry consumed by W2 ingestion (`pipeline::run` calls `open_validated`) + W4-A source-management block (category picker).
5. **Feedback loop.** `register_other` writes through service-only mutation path; canonical 7-source allowlist is read-only.

## 11. Handoff notes

- **W2-A (DOS-466)** consumes `open_validated` exclusively (CI lint gate enforces); consumes `resolve_path` for category-resolution fallback in `auto_detect_category`.
- **W4-C (DOS-474)** consumes `validate` for caller-provided category validation per cycle 10 contract reconciliation; its `PlacementError::CategoryNotAllowed` carries the same name as W1-B's `CategoryNotAllowed` but is a different envelope (deliberately distinct types in different layers).
- **W4-A (DOS-472)** consumes `WorkspaceCategoryRegistry` for the category-picker UI.

## 12. Path-α follow-ups (filed at L2 close, not blocking)

- **Windows path validation** (V1.1 fold #19): `feat/dos-464b-windows-path-validation`. Implement `lstat`-style pre-check + `FILE_FLAG_OPEN_REPARSE_POINT` semantic decision. Codebase Maintenance.
- **`CategoryNotAllowed` vs `PlacementError::CategoryNotAllowed` convergence** (challenge #3 residual): post-W4-C land, decide whether to unify or keep distinct envelopes. Codebase Maintenance.
- **Cross-device escape variant naming**: V1.1 reuses `SymlinkRefused` for hardlink + bind-mount cases; rename to `OutsideWorkspaceDevice` is a future refinement. Codebase Maintenance.
