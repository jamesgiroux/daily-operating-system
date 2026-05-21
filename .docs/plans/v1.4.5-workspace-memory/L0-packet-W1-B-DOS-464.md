# L0 Packet — v1.4.5 W1-B — DOS-464 Workspace Source Registry + Path Validation

**Current revision:** V1.1 (cycle 1 fold, 2026-05-20). See §2 Changelog.

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
    // Step 1: per-component rejection of `..` BEFORE canonicalize (defense in depth against
    // canonicalize bugs + per-component validation closes URL-encoded variants like %2e%2e and
    // NFC/NFD Unicode equivalents — the unicode-normalization crate is applied at this step).
    // Step 2: canonicalize path; assert it is a strict child of WORKSPACE_ROOT (no equality with
    // root; no escape). Equality with root → OutsideWorkspace (V1.1 fold #13 fixture).
    // Step 3: lstat the canonical path; record (lstat.dev, lstat.ino).
    //         Assert lstat.dev == workspace_root_dev (cross-device escape rejection; covers
    //         hardlinks-into-workspace from another mount AND bind-mounts over a subdir).
    // Step 4: open the canonical path with O_NOFOLLOW (Unix:
    //         std::os::unix::fs::OpenOptionsExt::custom_flags(libc::O_NOFOLLOW)). If the final
    //         component is a symlink, open fails → return SymlinkRefused. If a directory
    //         component is a symlink, the open follows it; the post-open fstat check below
    //         catches that case.
    // Step 5: fstat the open File handle; assert (fstat.dev, fstat.ino) == (lstat.dev, lstat.ino)
    //         from step 3. If different → return SymlinkRaced (an attacker swapped between
    //         canonicalize/lstat and open; race window closed by the post-open inode check).
    //         Also assert fstat.dev == workspace_root_dev (V1.1 fold #7 hardlink + #8 bind-mount).
    // Step 6: return (File, FileIdentity { canonical_path, device: fstat.dev, inode: fstat.ino }).
    // Windows: returns RejectionReason::OutsideWorkspace with a "platform not supported" log
    // until follow-up ticket (V1.1 fold #19).
}
```

### Negative fixture suite (V1.1 — 13 fixtures, all return typed `RejectionReason` with zero bytes read)

1. **TOCTOU race**: validate → atomically swap symlink target between lstat and open → `SymlinkRaced`.
2. **URL-encoded path traversal**: `%2e%2e/escape` → `PathTraversalAttempt`.
3. **NFC/NFD Unicode normalization**: `\u{30CF}\u{309A}/../escape` equivalents → `PathTraversalAttempt`.
4. **Absolute path**: `/etc/passwd` → `OutsideWorkspace`.
5. **Symlink final-component pointing outside workspace**: → `SymlinkRefused` (`O_NOFOLLOW` refuses).
6. **Bare `..` component**: `notes/../escape` → `PathTraversalAttempt`.
7. **V1.1 #7 — hardlink to `/etc/passwd`**: in-workspace path that is a hardlink to outside → `SymlinkRefused` (`fstat.dev != workspace_root_dev`).
8. **V1.1 #8 — bind-mount tmpfs over workspace subdir**: workspace-relative path inside the bind mount → `SymlinkRefused` (cross-dev).
9. **V1.1 #9 — case-insensitive lookup**: `Accounts/Acme/Presentations/x` vs `…/presentations/x` on case-insensitive FS → both succeed via lowercase normalization at the registry-key layer; explicitly tested.
10. **V1.1 #12 — NUL byte in path**: `foo\0bar` → kernel-level rejection mapped to `PathTraversalAttempt`.
11. **V1.1 #12 — PATH_MAX overflow**: path >4096 bytes → `PathTraversalAttempt`.
12. **V1.1 #13 — strict-child root equality**: `canonical_path == WORKSPACE_ROOT` → `OutsideWorkspace`.
13. **V1.1 #13 — symlink chain >1 hop**: A → B → outside → `SymlinkRefused` at A (`O_NOFOLLOW`).
14. **V1.1 — concurrency positive**: N=8 parallel `open_validated` calls on same valid path return identical `FileIdentity`.
15. **Positive**: valid workspace-relative path returns `(File, FileIdentity)` with `fstat`-derived device+inode.

### CI lint gate (V1.1 broadened per security SEC-005)

`src-tauri/scripts/check_workspace_path_validation.sh` enforces a positive allowlist:
- Forbidden: `std::fs::File::open`, `std::fs::OpenOptions::*::open`, `std::fs::read*`, `std::fs::metadata`, `std::fs::symlink_metadata`, `std::fs::read_link`, `std::fs::read_dir`, `tokio::fs::*`, `tokio::fs::OpenOptions::*`, `memmap2::Mmap::map`, any `BufReader::new(File::open(...))`/`BufReader::new(OpenOptions::*...)`.
- Allowed entry point: `services::workspace_ingestion::registry::open_validated` only.
- Allowlist marker (V1.1 fold #10 — replaces stale `dos-146-allowed`): `// workspace-path-allowed: <one-line rationale>`.

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
