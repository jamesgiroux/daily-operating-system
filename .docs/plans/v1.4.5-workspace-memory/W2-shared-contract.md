# v1.4.5 W2 Shared Surface Contract

**Revision:** V1.2 (2026-05-21 — cycle-13 wave-amendment fold)
**Status:** Frozen surface for W2-A V1.2 onwards; W2-B/C/D V1.2 packets cite this file.
**Authority:** This addendum is the L0 reconciliation artifact for cycle 1+2 substrate-gap findings. Any conflict between a per-lane V1.x packet and this file resolves in favor of this file. A change to this file is a W2 wave-amendment, not a per-lane edit.

**V1.2 changelog (cycle 13):**
1. `file_id_from_identity` switched from `blake3` to `sha256` (sha2 ships in Cargo.toml; blake3 doesn't). See §2.4.
2. `IngestError::Rejected(RejectionReason)` consumes W1 unit variants (NOT payloaded). See §2.3.
3. `WorkspaceIntakeService::ingest` is `async fn`. See §4.
4. `AbilityContext` (not `ServiceContext`) is the ability fn parameter type. See §4 + new §13.
5. `allowed_actors` differentiate by scope only — no fictional `WordPressRender` actor. See §13.
6. `resolved_path` scope-gated by `read.entity_names` with enforcement test required. See §2.2.
7. Cycle-13 also adds: W1-extension PR for `IngestionMode::EntitySeeded`/`Realtime` (W2-A precondition); W2-A bridge-ownership expansion to abilities-runtime lib.rs + context.rs; Drive staging + confirmation-token transport de-scoped to v1.4.6.

**V1.0 (2026-05-21 — cycle-1 fold):** original — see git history.

---

## 1. Why this exists

Cycle 1 found that all four W2 lane packets (W2-A/B/C/D V1.0) invented their own `IngestRequest`/`IngestReceipt` shapes. None compose. The codex challenge panel BLOCKED all four lanes on the same recurring class. This addendum freezes the canonical surface so the four lane V1.1 packets reconcile to a single source of truth.

## 2. Canonical types — W2-A defines, W2-B/C/D/W5-A consume

### 2.1 `IngestRequest`

```rust
// src-tauri/src/services/workspace_ingestion/pipeline.rs (W2-A owns)
use std::fs::File;
use chrono::{DateTime, Utc};
use abilities_runtime::abilities::provenance::source::{DataSource, WorkspaceFileKind};
use crate::entity::EntityType;
use super::contracts::{FileIdentity, WorkspaceCategory};
use super::runs::IngestionMode;

pub struct IngestRequest {
    /// Validated file handle returned by `WorkspaceSourceRegistry::open_validated`.
    /// W2-A pipeline never re-opens by path; the request carries the handle.
    pub file: File,

    /// Canonical identity returned alongside `file` by the registry.
    pub identity: FileIdentity,

    /// W1 lifecycle row primary key. Deterministic derivation from canonical path —
    /// see §2.4 for the exact formula. Callers MUST derive via
    /// `pipeline::file_id_from_identity(&identity, workspace_root)` — pipeline
    /// validates the value matches before doing work.
    pub file_id: String,

    /// File mtime captured at the time `registry::open_validated` returned. Caller
    /// reads via `identity.canonical_path.metadata()?.modified()?.into()` or via
    /// the pre-validated mtime cached by W1-B's registry.
    pub source_asof: DateTime<Utc>,

    /// Canonical `WorkspaceFileKind` (Inbox | EntityDoc | DriveSync | UserAttachment |
    /// GranolaTranscript | QuillTranscript | McpPlacement). Maps 1:1 to
    /// `DataSource::WorkspaceFile { kind }`. Pipeline serde-encodes this for the
    /// `source_type` lifecycle column. NOT a separate enum — consume canonical.
    pub source_type: WorkspaceFileKind,

    /// Pre-resolved entity binding when caller knows the entity at request time
    /// (W2-C entity-intake, W4-A user-override, W2-D after assignment).
    /// When `None`, pipeline transitions to `pending_entity_assignment` lifecycle.
    pub entity: Option<EntityRef>,

    pub mode: IngestionMode,

    /// Pre-validated category from caller (DOS-474 MCP placement, W2-C block
    /// attribute, W2-D drop). When `Some`, MUST already be valid per
    /// `WorkspaceCategoryRegistry::validate(conn, &category, entity.as_ref().map(|e| e.entity_type))`.
    /// Pipeline trusts the type — caller is responsible for upstream validation.
    /// When `None`, pipeline runs `auto_detect_category` as fallback.
    pub category_hint: Option<WorkspaceCategory>,
}

/// Pre-resolved entity binding. Combines `EntityType` and `EntityId` so the
/// pipeline never has to look up entity_type from entity_id.
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: abilities_runtime::abilities::provenance::source::EntityId,
    /// Display name carried for lifecycle-row population. Not used for routing.
    pub entity_name: Option<String>,
}
```

### 2.2 `IngestReceipt`

```rust
pub struct IngestReceipt {
    /// W1-C ingestion-run row ID. Caller uses for correlation / retry / audit.
    pub ingestion_run_id: super::runs::IngestionRunId,

    /// Echoed back from request for caller convenience. Stable per file.
    pub file_id: String,

    /// SHA-256 of the file content as ingested. Used by W1-C idempotency fence.
    pub content_sha256: String,

    /// Lifecycle state after pipeline completes (`ingested` |
    /// `pending_entity_assignment` | `quarantined`). Caller uses to branch on
    /// downstream UI ("show pending badge" vs "show ingested confirmation").
    pub lifecycle_state_after: super::lifecycle::LifecycleState,

    /// At W2 time this is always 0 (zero-claim pipeline shell). Field exists
    /// so W3-A can ship without changing receipt shape.
    pub claim_proposals: Vec<super::contracts::WorkspaceClaimProposal>,

    /// `auto_detect_category` result OR echo of `category_hint`. `None` means
    /// the file lands at entity root (no sub-directory).
    pub resolved_category: Option<WorkspaceCategory>,

    /// Workspace-relative path of the file after ingestion. Per cycle 9 CSO
    /// amendment, populated ONLY when caller has `read.entity_names` scope.
    /// Otherwise `None`; caller uses `file_id` for follow-up operations.
    /// (W2-A pipeline always populates; the redaction is the caller's job at
    /// scope-bridge time — DOS-474 server / W2-C ability output.)
    pub resolved_path: Option<String>,
}
```

### 2.3 `IngestError` — consumes `RejectionReason`, no duplicates

V1.2 fix: `RejectionReason` is W1's shipped enum with UNIT variants (see `src-tauri/src/services/workspace_ingestion/contracts.rs:125-132`). Earlier V1.1 text mistakenly described payloaded variants. The pipeline carries metadata (limit_bytes, found_bytes, detected format string) in a separate `RejectionMetadata` log/event payload, NOT inside the `RejectionReason` enum.

```rust
pub enum IngestError {
    /// All file-content rejection paths route through canonical RejectionReason
    /// from contracts.rs (FileTooLarge | UnsupportedFormat | PathTraversalAttempt
    /// | OutsideWorkspace | SymlinkRaced | ... — UNIT variants per W1).
    /// W2-A pipeline does NOT define parallel size/format variants.
    Rejected(super::contracts::RejectionReason),

    /// W1-C idempotency conflict.
    AlreadyProcessed {
        existing_run_id: super::runs::IngestionRunId,
    },

    /// I/O failure that is NOT a rejection (e.g., disk full during read).
    /// Display impl reduces to `io::ErrorKind` — must NOT serialize the path
    /// (cycle 1 /cso F4).
    Io(std::io::Error),

    /// DB write failure during lifecycle/run record. Stringly-typed for now
    /// (substrate-wide pattern; not W2-A's job to fix per cycle 1 path-α).
    DbError(String),
}
```

### 2.4 `file_id` derivation

V1.2: formula is `sha256` (Cargo.toml ships `sha2 = "0.10"` + `hex = "0.4"`; no `blake3` dep). Truncated to 16 hex chars — same collision resistance for workspace file count ≤2^32.

```rust
// src-tauri/src/services/workspace_ingestion/pipeline.rs
use sha2::{Sha256, Digest};

/// Canonical file_id formula. ONE derivation; all lanes use this helper.
///
/// Formula: `sha256_hex_lower(workspace_relative_path_bytes)[..16]`.
///
/// Why path, not (device, inode)? Workspace files cross filesystems
/// (iCloud sync, external drives). Path identity survives the file moving
/// between filesystems; (device, inode) does not. Path identity also
/// survives lineage across mtime changes (edits keep file_id).
///
/// Cost: file rename invalidates file_id. Acceptable — renames are explicit
/// user actions tracked by W4-A; the old file_id transitions to `superseded`,
/// the new file_id is a fresh row.
pub fn file_id_from_identity(
    identity: &FileIdentity,
    workspace_root: &Path,
) -> Result<String, FileIdError> {
    let relative = identity
        .canonical_path
        .strip_prefix(workspace_root)
        .map_err(|_| FileIdError::OutsideWorkspace)?;
    let mut hasher = Sha256::new();
    hasher.update(relative.as_os_str().as_encoded_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(&hash)[..16].to_string())
}

pub enum FileIdError {
    OutsideWorkspace,
}
```

## 3. Lifecycle write helper — W2-A owns

**Cycle-1 finding:** W1-A's `lifecycle.rs` ships types + `escalate_to_pending` stub only. There is no `LifecycleRepo` write API. W2-A's pipeline cannot "transition lifecycle" against W1's surface as shipped.

**Resolution:** W2-A V1.1 adds `lifecycle.rs` to its owned-files list and ships the write helpers:

```rust
// src-tauri/src/services/workspace_ingestion/lifecycle.rs (W1-A types kept;
// W2-A adds the write helpers below).

pub struct LifecycleRepo;

impl LifecycleRepo {
    /// Inserts a new lifecycle row at `lifecycle_state = pending`. Caller
    /// (pipeline) calls this BEFORE start_run.
    pub fn insert_pending(
        conn: &Connection,
        file_id: &str,
        identity: &FileIdentity,
        source_type: &WorkspaceFileKind,
        source_asof: DateTime<Utc>,
        entity: Option<&EntityRef>,
    ) -> Result<(), LifecycleError> { /* W2-A L1 fills */ }

    /// Transitions a row from one state to another with validation.
    /// Returns `LifecycleError::InvalidStateTransition` if the from/to pair
    /// is not in the legal transition matrix (§5 of W1-A's lifecycle.rs
    /// docstring).
    pub fn transition(
        conn: &Connection,
        file_id: &str,
        from: LifecycleState,
        to: LifecycleState,
    ) -> Result<(), LifecycleError> { /* W2-A L1 fills */ }

    /// Records the user override for quarantine + entity-assignment paths.
    pub fn record_user_override(
        conn: &Connection,
        file_id: &str,
        actor: &str,
    ) -> Result<(), LifecycleError> { /* W2-A L1 fills */ }

    /// Updates the `category` column after `auto_detect_category` resolution.
    pub fn update_category(
        conn: &Connection,
        file_id: &str,
        category: Option<&WorkspaceCategory>,
    ) -> Result<(), LifecycleError> { /* W2-A L1 fills */ }
}
```

W2-A's V1.1 §5 "Don't touch" REMOVES `lifecycle.rs` from the forbidden list. W2-B/C/D do NOT touch `lifecycle.rs` — they call `LifecycleRepo` from W2-A.

The W1-A `escalate_to_pending` stub stays as-is (it's a sketch; W2-A's `LifecycleRepo::transition(file_id, Ingested, PendingEntityAssignment)` is the real implementation per the new contract).

## 4. Ability/runtime crate bridge — `WorkspaceIntakeService` trait

**Cycle-1 finding (Class E):** `abilities-runtime` is a separate crate with no `dailyos_lib` dependency. W2-C's `entity_intake` ability cannot call `services::workspace_ingestion::IngestPipeline::run` directly.

**Resolution:** mirror the same DI pattern W1-A used for `Extractor`/`SignalEmitter`, but at the crate boundary.

```rust
// src-tauri/abilities-runtime/src/services/workspace_intake.rs (NEW, W2-A owns)
//
// Crate-boundary trait. abilities-runtime declares the trait; dailyos_lib
// implements it; ServiceContext extension wires the impl into the ability
// invocation flow.

#[async_trait::async_trait]
pub trait WorkspaceIntakeService: Send + Sync {
    /// V1.2: async. Ability fn is `pub async fn`; trait must compose. The
    /// dailyos_lib impl calls IngestPipeline::run inside spawn_blocking so the
    /// sync rusqlite work doesn't stall the ability runtime.
    /// Validates the file_ref against the workspace root, opens, and invokes
    /// the pipeline. Returns the receipt OR a typed error matching the
    /// IngestError variants.
    async fn ingest(
        &self,
        ctx: &AbilityContext<'_>,
        request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError>;
}

/// Mirror of IngestRequest but flattened for crate-boundary stability. The
/// dailyos_lib impl converts WorkspaceIntakeRequest into IngestRequest using
/// internal helpers (registry::open_validated, file_id_from_identity).
pub struct WorkspaceIntakeRequest {
    pub file_ref: String,              // workspace-relative path
    pub source_type: WorkspaceFileKind,
    pub entity: Option<EntityRefDto>,
    pub mode: IngestionMode,
    pub category_hint: Option<WorkspaceCategory>,
}
```

`ServiceContext` gets a new accessor: `ctx.services().workspace_intake() -> &dyn WorkspaceIntakeService`. The dailyos_lib `Bootstrap` registers an `IngestPipelineWorkspaceIntake` impl during app startup.

**V1.2 ownership expansion:** W2-A owns these crate-bridge edits, not just the trait file:

| File | W2-A V1.2 edit |
|---|---|
| `src-tauri/abilities-runtime/src/lib.rs` | Add `pub mod services::workspace_intake;` declaration |
| `src-tauri/abilities-runtime/src/services/mod.rs` | Add `pub mod workspace_intake;` |
| `src-tauri/abilities-runtime/src/services/workspace_intake.rs` | NEW — trait definition (this section) |
| `src-tauri/abilities-runtime/src/services/context.rs` | Add `workspace_intake: &dyn WorkspaceIntakeService` field to `ServiceContext`; add `services()` accessor; add constructor params |
| `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs` | NEW — `IngestPipelineWorkspaceIntake` impl of the trait, threading through `IngestPipeline::run` via `spawn_blocking` |
| `src-tauri/src/main.rs` or `Bootstrap` location | Register the impl into the runtime's `ServiceContext` construction |

**W2-C V1.2 §9 stub uses this trait, not `services::workspace_ingestion::*`.** The ability stays in abilities-runtime; the crate-boundary impl lives in dailyos_lib.

Alternative considered (and rejected): moving the `entity_intake` ability into dailyos_lib's abilities namespace. Rejected because ADR-0102 puts the canonical ability registry in `abilities-runtime` and migrating a single ability across the boundary breaks the registry-discovery pattern.

## 5. CI script ownership

`check_workspace_mutation_allowlist.sh` is owned by **W2-A** (creation). Lives at `src-tauri/scripts/check_workspace_mutation_allowlist.sh`, modeled on `src-tauri/scripts/check_claim_writer_allowlist.sh`.

- W2-A V1.1 §4 lists the script in owned files.
- W2-A V1.1 §10 includes the script in the test/CI gate list.
- W2-B/C/D §11 reference the script as "must remain green after this lane's edits" — they do NOT create it.

## 6. `DataSource` canonicalization

The wave plan body contains stale references to `DataSource::WorkspaceInbox`. The canonical type from ADR-0107 + the W0 amendment is:

```rust
DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }
```

There is NO `DataSource::WorkspaceInbox` variant. All four V1.1 packets MUST use the canonical form everywhere (including IL gate Q2 text). The wave plan itself gets a cycle-13 amendment to sweep the body text (filed by W2-A V1.1 changelog).

## 7. `WorkspaceCategoryRegistry::validate` signature

W1-B ships `WorkspaceCategoryRegistry::validate(conn: &Connection, category: &WorkspaceCategory, entity_type: EntityType)`. The function requires a DB connection AND a typed `EntityType` — not stringly. Two consequences:

1. **`auto_detect_category` MUST be split into two functions.** The sniff is pure (filename + content_head → candidate category); the validate is impure (needs conn + entity_type). Pipeline calls `auto_detect_category_pure(filename, content_head) -> Option<WorkspaceCategory>` first, then `WorkspaceCategoryRegistry::validate(conn, &candidate, entity_type)` second, falling back to `None` on validation failure.
2. **Callers (W2-C, W2-D, DOS-474) MUST validate `category_hint` upstream** before constructing `IngestRequest`. Pipeline trusts the type — request-time `category_hint` is contracted to be already-valid.

## 8. CI grep gate hardening

W2-A V1.1 §10 CI grep gate covers ALL path-opening APIs, not just the two strings in V1.0:

```text
Forbidden in pipeline.rs source:
  - std::fs::File::open
  - std::fs::File::options
  - std::fs::OpenOptions
  - tokio::fs::File::open
  - tokio::fs::File::options
  - tokio::fs::OpenOptions
  - std::fs::read
  - std::fs::read_to_string
  - std::fs::metadata
  - memmap
  - std::process::Command
```

Implementation is a line-anchored grep that ignores comment lines and `use ` imports (W1's registry.rs legitimately imports `File::open`). The test loads `pipeline.rs`, strips `//`-comments and `use ` lines, then asserts zero matches.

## 9. `quarantine_source` actor constraint

Cycle-1 /cso F2: actor parameter must be capability-gated. V1.1 contract:

```rust
pub enum QuarantineActor {
    /// W4-A source-management UI (human user action via SurfaceClient).
    User { user_id: String },
}

pub fn quarantine_source(
    conn: &Connection,
    file_id: &str,
    reason: &str,
    actor: QuarantineActor,
) -> Result<(), IngestError>;
```

Free-form `&str` actor rejected. Future callers (e.g., automated quarantine-by-policy) get a new variant via a wave-plan amendment, not by passing arbitrary strings.

## 10. Substrate reuse policy (Class D fix)

V1.1 lanes MUST consume canonical primitives. The cycle-1-recurring reinventions:

| Reinvention found at cycle 1 | Canonical replacement |
|---|---|
| `IngestError::FileTooLarge { limit_bytes }` | `IngestError::Rejected(RejectionReason::FileTooLarge { … })` |
| `IngestError::UnsupportedFormat { detected }` | `IngestError::Rejected(RejectionReason::UnsupportedFormat { … })` |
| `EntityIntakeClaimBand { trust_band: String }` | `abilities_runtime::abilities::trust::types::TrustBand` |
| `IngestRequest.entity_id: Option<String>` | `IngestRequest.entity: Option<EntityRef>` (typed) |
| `entity_type: String` everywhere | canonical `crate::entity::EntityType` |
| `IngestRequest.source_path: PathBuf` | `IngestRequest.file: File + identity: FileIdentity` |

Reviewers at cycle 2 verify zero new reinventions against this table.

## 11. Render-time mutation prohibition (Class F fix)

W2-C V1.1 §9 separates two flows:

- **Editor-side write trigger.** User inserts the block, configures `entity_id` + `file_ref`, clicks "Ingest." The editor invokes the `entity_intake` ability (write). This is a deliberate user gesture with `requires_confirmation = true` and a confirmation token surface.
- **Render-side read.** `render.php` reads the existing lifecycle row + claim proposals via a SEPARATE read-only ability (`entity_intake_render` or reuse of an existing claim-list ability). NO ingestion happens at render time.

If the existing read-only ability doesn't exist, W2-C V1.1 OWNS its creation. Either way, the read and write are not the same ability and the render path never triggers a write.

## 12. Done — V1.2 is frozen for cycle 3

Any required change to §§1–13 is a wave-plan cycle-14+ amendment to v1.4.5-waves.md, NOT a packet-local edit. V1.2 packets that conflict with this file are L0-BLOCKED at cycle 3 review.

## 13. V1.2 canonical ability shape (cycle-13 fold)

W2-C V1.2 §9 stubs MUST mirror `src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-115` exactly:

```rust
#[ability]
pub async fn entity_intake(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeInput,
) -> AbilityResult<EntityIntakeOutput> {
    // Body returns `EntityIntakeOutput` directly. NOT `Ok(AbilityOutput { data: ... })`.
    // The #[ability] macro/runtime wraps the inner value into `AbilityOutput<T>`.
    Ok(EntityIntakeOutput { /* fields */ })
}
```

Pins for cycle-3 reviewers:
- Parameter type: `&AbilityContext<'_>` — NOT `ServiceContext`. AbilityContext at `src-tauri/abilities-runtime/src/abilities/registry.rs:740-768`.
- `pub async fn` — NOT sync.
- Return value: `Ok(T)` where T is the inner output type. NOT `Ok(AbilityOutput { data: T })`.
- `allowed_actors` MUST use only existing `ActorKind` variants at `src-tauri/abilities-runtime/src/abilities/registry.rs:468-486`: `Agent | User | Admin | System | SurfaceClient | McpClient`. No fictional `WordPressRender` actor — write/read differentiation is by scope, not actor variant.
- `mcp_exposure: McpExposure::None` for v1.4.5 W2-C (per wave plan).

W2-C V1.2 §10 includes a literal-diff test against `account_overview.rs:106-115` skeleton to prevent V1.1's stub-drift class pattern from recurring.
