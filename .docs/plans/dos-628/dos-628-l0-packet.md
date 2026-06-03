# DOS-628 L0 Packet -- Claims To Editable Readable Files

> Status: L0-approved locally after K-in, feasibility, security, adversarial document review, and Codex challenge.
> Issue: DOS-628.
> Wave: v1.4.9 W3 -- Claims -> editable readable files.
> Branch: `codex/v1.4.9-w3-dos628`.
> Base: `public/dev` at `da1df394`.
> Scope tier: Wave/critical-path substrate surface. W3 is the first visible correction-loop slice and is a prerequisite for DOS-832 rebuild correction preservation.

## 0. Origination

Extension, not incident recovery. DOS-628 predates v1.4.9 and originally asked for "bidirectional markdown <-> substrate ingestion" as a missing leg from older three-view consistency plans. The v1.4.9 wave plan deliberately narrows and corrects that framing:

- SQLite/claim substrate is authority.
- Readable files are projection out, not files-as-truth.
- A file edit is a correction input routed through typed claim feedback, not an in-place claim update.
- The structured corrections sidecar is the bridge that makes user corrections survive DOS-832 rebuild.

The user outcome is the first leg of the v1.4.9 north star: a user can correct a claim by editing a readable entity file, and the next projection reflects that correction through the same substrate that app and MCP feedback use.

## 1. Trust Topology

Topology is `local-to-local single-user` for the app/file path. The user edits files in their own workspace on the same machine. Do not add multi-actor gates, app/file scope grants, or principal differentiation to this path.

The safety requirements that still apply:

- Workspace-root confinement for all file paths.
- Symlink/`..`/canonicalization checks before reading or writing.
- File-read content is untrusted evidence. If it reaches prompt construction, it is ADR-0093 Tier-2-classified and Tier-3-handled via `wrap_user_data`.
- File-provided correction text and provenance explanations are sanitized/redacted per ADR-0108.
- MCP remains a separate carve-out: `Confidential`/`UserOnly` claims never cross MCP even if the same file is readable locally.

## 2. Source Reconciliation

### Linear Issue

DOS-628 currently says markdown should be generated from substrate and re-ingested when edited out of band. That text is stale relative to v1.4.9's accepted wave-plan decisions. The usable part is the need for a durable, user-readable file surface that can accept out-of-band edits.

### Version Plan

`.docs/plans/v1.4.9-waves.md` is the authority:

- W3 is `Claims -> editable readable files`.
- DOS-628 projects per-entity claims to markdown and a structured corrections sidecar.
- File-to-claim write-back is `Actor::User`, workspace-root-confined, sensitivity re-validated, and routed through `services/claims.rs` as typed `FeedbackAction`.
- File-read content is ADR-0093 Tier-2-classified and Tier-3-handled.
- W3 must run the full `claim-producers-require-runtime-wide-trust-audit` inventory on write-back.
- DOS-832 depends on the W3 sidecar for correction-preserving rebuild.

### Current Code Evidence

- `services/claims.rs::commit_claim` is the only writer for `intelligence_claims`.
- `services/claims.rs::record_claim_feedback` inserts `claim_feedback`, updates verification/lifecycle through the service, bumps claim invalidation, and emits `claim_feedback_recorded`.
- `record_claim_feedback` rejects non-user feedback actors.
- `abilities-runtime/src/abilities/feedback.rs::FeedbackAction` is the 10-variant typed feedback contract.
- `services/claim_receipt/feedback.rs::submit_claim_feedback` already validates envelope target binding, sensitivity/surface access, per-action metadata, sanitizer warnings, idempotency, and routes into `record_claim_feedback`.
- `services/claims.rs::compute_dedup_key(item_hash, subject_ref_compact, claim_type, field_path)` is the content-derived semantic identity W3/DOS-832 must use for rebuild-stable correction keys.
- `services/derived_state.rs` and `claim_projection_status` show the existing pattern: canonical claim writes can drive projection status without making projections authoritative.
- `services/workspace_ingestion/registry.rs` already has a canonicalize-and-child-of-workspace-root pattern to reuse for path validation.

## 3. Non-Negotiable Decisions

### D1 -- Files Are Projection, Not Authority

The per-entity markdown file is a readable projection of canonical claim state. It must not become a source-of-truth file that can overwrite `intelligence_claims`.

Implementation must not:

- Parse arbitrary prose into fresh claims.
- Update immutable claim columns (`text`, `subject_ref`, `claim_type`, `source_asof`, `created_at`) in place.
- Bypass `services/claims.rs`.
- Teach runtime prompt/context builders to treat projected markdown as trusted fact.

Implementation must:

- Select claims from canonical services/readers.
- Render claim identity, trust band, provenance summary, sensitivity-safe labels, and editable correction affordances.
- Re-render projection after accepted corrections.
- Treat unparseable or ambiguous edits as review/quarantine, not silent claim writes.

### D2 -- File Edits Become Typed Corrections

An edit to a projected claim block is a user correction event. The write path routes through the receipt-level feedback validator (`submit_claim_feedback`) with `Actor::User`; the receipt path then calls `record_claim_feedback`. W3 does not add a direct call path that bypasses receipt validation.

Supported edit mappings for L1:

| File edit shape | FeedbackAction |
| --- | --- |
| User confirms a claim remains true/current | `ConfirmCurrent` |
| User marks a claim false | `MarkFalse` |
| User marks a claim outdated or no longer current | `MarkOutdated` |
| User corrects claim text in an editable text block | `NeedsNuance` with `corrected_text` |
| User marks claim as attached to the wrong subject | `WrongSubject` with `corrected_subject_ref` when the user supplies the correct subject |
| User says the cited evidence does not support the claim | `WrongSource` with source-content hash metadata |
| User cannot verify from surfaced evidence | `CannotVerify` |

If a file edit cannot be deterministically mapped to one of those actions, L1 must not invent a new action. It records an explicit parse/reconciliation failure artifact and leaves canonical claim state unchanged.

`WrongSource` always uses the receipt-level source-content-hash contract. The sidecar stores the `source_content_hash` derived from the current claim source tuple. Apply builds a file-surface feedback envelope from the sidecar, calls `submit_claim_feedback`, and lets the existing validator reject stale or mismatched hashes as `SourceNoLongerInClaim`. L1 must not route `WrongSource` directly to `record_claim_feedback` with only `source_ref`.

`WrongSubject` uses `corrected_subject_ref` as the canonical metadata field: a JSON `SubjectRef` object or encoded string accepted by `subject_ref_from_json`. Current receipt validation only allowlists legacy `corrected_to`, while targeted repair consumes `corrected_subject` / `corrected_subject_ref`; L1 must close that mismatch before file apply ships. Required behavior: receipt validation accepts `corrected_subject_ref`, sanitizes/rejects malformed subject refs, forwards the field unchanged to `record_claim_feedback`, and may bridge legacy `corrected_to` into `corrected_subject_ref` for compatibility. Tests must prove the corrected subject reaches targeted repair/replay rather than being silently dropped.

### D3 -- Structured Corrections Sidecar Is Required

Markdown alone is too lossy for DOS-832. W3 must write a structured corrections sidecar next to the readable projection under the managed `_dailyos_claims` root.

The sidecar is the authoritative rebuild input for correction replay. It must include, at minimum:

- `schema_version`.
- `projection_version`.
- Entity subject reference.
- Stable claim semantic identity: `item_hash`, compact subject identity, `claim_type`, and `field_path`, matching the `compute_dedup_key` shape.
- Runtime claim id and claim version as debugging aids only, never as the replay key.
- `source_ref`, `observed_at`, and `source_asof` where available for non-unique tie-breaks.
- Feedback rows as typed `FeedbackAction` plus actor, actor id when available, payload JSON, submitted/applied timestamps.
- Lifecycle effects needed for rebuild: tombstone/withdrawn/superseded/dormant state, supersession link, render/surfacing suppression, contradiction edge references.
- Contradiction endpoints by semantic identity plus `reconciliation_kind`, not by UUID alone.
- Orphan/replay status fields for DOS-832 to record ambiguous or missing endpoints without dropping data.

The sidecar must not include raw customer PII beyond what is already present in the user's local projected files. Logs and PR fixtures must stay generic. Sidecar retention is tied to the projection/rebuild lifecycle: it is durable enough for DOS-832 replay and repair, but not a separate archival store for raw correction prose.

### D4 -- Rebuild Identity Is Content-Derived And Tombstone-Aware

W3 owns the exact semantic identity that DOS-832 replays. The key is the canonical `compute_dedup_key` shape, not the raw `dedup_key` column as a universal truth.

The sidecar identity is `ClaimSemanticIdentityV1`:

- `identity_version`: `1`.
- `identity_kind`: `claim_dedup_v1` for non-`UserNote` claims, `user_note_v1` for `ClaimType::UserNote`.
- `item_hash`: the canonical `intelligence_claims.item_hash` captured when the projection is rendered. It is anchored to the original canonical claim text and is not recomputed from user-edited correction prose.
- `subject_ref_compact`: the same compact subject string used by `commit_claim` before it calls `compute_dedup_key`.
- `subject_ref`: the structured `SubjectRef` serialized canonically for validation and diagnostics.
- `claim_type`: exact canonical claim type string.
- `field_path`: exact canonical field path string; `null` in the sidecar maps to the empty fourth component in `compute_dedup_key`.
- `dedup_key_components_hash`: SHA-256 over the four canonical components for content-redacted logs.
- `source_ref`, `data_source`, `observed_at`, `source_asof`, and `source_content_hash` where present.
- `runtime_claim_id` and `runtime_claim_version`: diagnostics only, never the replay key.
- `claim_state`: active/dormant/withdrawn/tombstoned/superseded plus supersession target identity when present.

For non-`UserNote` claims, DOS-832 recomputes `compute_dedup_key(item_hash, subject_ref_compact, claim_type, field_path)` from the fresh claim row and matches against the sidecar identity. For `UserNote`, DOS-832 uses the existing `compute_user_note_dedup_key(subject_ref_compact, actor, observed_at)` identity kind and never falls back to runtime UUID.

Replay matching algorithm:

1. Build candidate fresh claims by `identity_kind` and exact semantic identity.
2. If no candidate exists, record `orphan_missing` and leave canonical state unchanged.
3. If one candidate exists, validate `source_content_hash` when the correction action requires source binding, then replay the typed `FeedbackAction` through the service path.
4. If multiple candidates exist, filter by exact `source_ref` and `observed_at`; if that yields one candidate, replay to that candidate.
5. If still ambiguous, filter by `source_content_hash`; if that yields one candidate, replay to that candidate.
6. If still ambiguous, record `orphan_ambiguous` and leave canonical state unchanged.
7. Resolve contradiction endpoints independently with the same algorithm. If either endpoint is missing or ambiguous, record an orphaned edge and do not attach it to a guessed UUID.
8. Apply terminal lifecycle state before surfacing projection results. A sidecar tombstone/withdrawal/supersession replays as typed feedback/state through services; a freshly regenerated matching claim is demoted before it can appear as active. No replay path creates a fresh active claim to satisfy an orphan.

Ambiguous replay is an orphan, never a guessed correction.

### D5 -- Projection Write State Is Durable And Idempotent

W3 adds a dedicated readable-file projection ledger instead of extending `claim_projection_status`. The existing table is per `(claim_id, projection_target)` and constrained to legacy targets; readable files need entity/path/version/checksum state. Use W3 migration slot `v280` for `claim_file_projection_runs` and `v281` for its claim-membership table/indexes if needed.

Minimum ledger schema:

- `claim_file_projection_runs`: `id`, `entity_subject_ref_json`, `entity_subject_compact`, `projection_root`, `markdown_rel_path`, `sidecar_rel_path`, `projection_version`, `sidecar_schema_version`, `entity_claim_invalidation_version`, `claim_watermark`, `markdown_checksum`, `sidecar_checksum`, `status`, `error_class`, `error_detail_hash`, `attempted_at`, `succeeded_at`, `repaired_from_run_id`, `created_at`, `updated_at`.
- `claim_file_projection_run_claims`: `run_id`, `claim_id`, `claim_version`, `semantic_identity_json`, `trust_band`, `sensitivity`, with primary key `(run_id, claim_id)`.
- `status` is constrained to `committed`, `failed`, or `repaired`.
- `projection_root` is constrained to the managed projection root named in D8.
- `claim_watermark` is a SHA-256 over sorted `claim_id:claim_version` entries plus the per-entity invalidation version. It is the repair/re-render trigger, not a trust input.
- `sidecar_checksum` is SHA-256 over canonical JSON serialization of the sidecar. The markdown block points to this checksum; apply refuses mismatched markdown/sidecar pairs and records `sidecar_mismatch`.
- Repair worklist query is `status = 'failed' OR current_claim_watermark != claim_watermark`, ordered by `attempted_at`, bounded by service-owned batch size.

Projection repair must be idempotent. A failed projection must not abort the authoritative claim feedback write that caused it.

### D6 -- Sensitivity And Prompt-Injection Boundaries Are Explicit

Local file output can include content the user can already access, but the projection must still preserve sensitivity metadata so downstream consumers do not flatten policy.

Requirements:

- Each projected claim block carries sensitivity and trust band metadata in machine-readable form.
- MCP readers do not read these files as a way around the ADR-0125 MCP sensitivity gate.
- File-read correction content is never interpolated into prompts without `wrap_user_data`.
- Free-text correction fields use the ADR-0108 sanitizer before rendering/logging.
- Logs include claim ids, action names, paths relative to workspace, and error classes; they do not include raw correction prose or claim text.

### D7 -- W3 Emits W4-Consumable Signals

W3 is not complete if it only writes files. A successful file edit must produce the same substrate effects as app feedback:

- `claim_feedback` row inserted.
- `claim_feedback_recorded` signal emitted.
- `claim_verification_state_changed` emitted when applicable.
- Per-entity claim invalidation version bumped.
- Targeted repair job enqueued where the `FeedbackAction` semantics require one.
- Projection re-render scheduled or completed.

This is how W4/DOS-834 consumes W3. Do not add a parallel "file correction" signal family unless it is a thin source annotation on the existing feedback signal.

### D8 -- Projection Files Live In A Managed Scanner-Excluded Root

Readable projections are written under `<workspace>/_dailyos_claims/`, never under the ordinary per-entity document folders that workspace ingestion scans as source evidence. Existing backfill skips roots beginning with `_` except `_inbox`, but explicit workspace intake can still open any in-workspace path through `WorkspaceSourceRegistry::open_validated`; D8 therefore applies to every source-ingestion path, not only backfill.

Required shape:

- Markdown: `_dailyos_claims/<entity-kind>/<entity-slug>/claims.md`.
- Sidecar: `_dailyos_claims/<entity-kind>/<entity-slug>/claims.corrections.json`.
- Relative paths only. No user-supplied absolute output root in the first slice.
- Apply commands canonicalize paths and require them to remain under `_dailyos_claims`.
- Projection files may be user-edited locally, but source ingestion treats them as managed output, not evidence.

Required source-ingestion fence:

- Add a shared managed-root predicate for `_dailyos_claims`.
- Backfill must skip the root before classification.
- Explicit workspace intake (`entity_intake`, MCP/tool placement paths that call `WorkspaceSourceRegistry::open_validated`, and any command that turns a workspace file into source evidence) must reject `_dailyos_claims` paths with a managed-output rejection before opening/running the ingestion pipeline.
- The rejection must be content-redacted and must not read the projected file before deciding it is managed output.

## 4. Implementation Shape

### Service Boundary

Add the main implementation under `src-tauri/src/services/claim_files/` or equivalent service-owned module. Commands and any doctor/debug entrypoints are thin option validation and dispatch.

Proposed service responsibilities:

- `render_entity_claim_file(subject)`: reads canonical claims and writes readable markdown plus sidecar.
- `detect_entity_claim_file_changes(subject/path)`: compares file/sidecar/projection ledger.
- `parse_claim_file_corrections(path, sidecar)`: maps structured editable blocks to typed correction candidates.
- `apply_claim_file_corrections(corrections)`: builds file-surface feedback envelopes and calls `submit_claim_feedback` with `Actor::User`.
- `repair_claim_file_projection(subject)`: retries failed projection/sidecar writes idempotently.

### File Format

The markdown should be human-readable but machine-bounded. Use claim blocks with stable IDs and a narrow editable field shape rather than prose scraping.

Required block metadata:

- Claim semantic identity.
- Runtime claim id/version for diagnostics.
- Claim type.
- Subject ref.
- Trust band and sensitivity.
- Provenance/source summary.
- Editable fields allowed for the claim type.
- Sidecar pointer/checksum.

The sidecar is the machine contract. Markdown is the user surface.

### Apply Boundary

The first L1 slice uses an explicit apply command only. The user edits a file, then the service scans `_dailyos_claims` and applies structured corrections. No file watcher ships in DOS-628.

The explicit command still must avoid applying while a file is mid-write. It uses an atomic read safeguard: read metadata, read file, re-read metadata, and reject/retry if size or mtime changed during the read.

### Migration Slot

W3 has reserved migration slots `v280-v283` in the version plan. DOS-628 uses `v280` for `claim_file_projection_runs` and `v281` for `claim_file_projection_run_claims` or supporting indexes. Slots `v282-v283` remain W3 reserve only; they are not used unless L1 needs a follow-on migration for the same projection/sidecar substrate.

### Runtime-Wide Trust Audit Inventory

Authoritative audit source: `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`. There is no separate L0 script; the required check is this inventory against the solution note's five dimensions.

Audit result:

- Claim type metadata: DOS-628 adds no new `ClaimType`. It consumes existing claim metadata and `FeedbackAction` semantics. Allowed actor for file write-back is `Actor::User`; MCP actor labels remain separate and are not collapsed.
- Producer path: projection out is read-only and does not commit claims. File apply is a correction producer, not a claim producer: `_dailyos_claims` -> `claim_files` service -> `submit_claim_feedback` -> `record_claim_feedback` -> `claim_feedback`.
- Trust inputs: W3 carries existing trust inputs (`source_asof`, `observed_at`, `data_source`, `source_ref`, source reliability, lifecycle state, corrections, contradictions, and sensitivity) into the sidecar/projection. It does not seed trust scores or invent a file-specific trust rule.
- Recompute trigger: accepted corrections use the existing feedback signal/invalidation path. Projection repair is a consumer of invalidation state, not an alternate trust compiler.
- Surface behavior: projected markdown renders centralized trust band and sensitivity metadata. Missing or stale provenance/source hashes degrade to `needs_verification` or an orphan/needs-review status; they never render as fully trusted by default.

Runtime path inventory:

- App feedback: existing receipt/service path remains the model path.
- File apply: must use the same receipt/service path and action metadata validation, with a file-surface source annotation only.
- MCP feedback, if present: continues through canonical services and render policy; it cannot read `_dailyos_claims` to bypass sensitivity.
- DOS-832 rebuild replay: replays typed corrections through services after semantic-identity matching; it does not raw-insert `claim_feedback` rows or mutate `intelligence_claims`.
- Existing claim producers: unchanged by W3. If L1 touches any `commit_claim` producer while implementing projection, it must update this inventory before PR.

## 5. Acceptance Criteria

### AC1 -- Projection Out

For an account, project, and person with active public/internal claims, L1 writes a readable markdown file and structured sidecar containing claim identity, trust band, sensitivity, provenance summary, and editable correction metadata.

### AC2 -- No Files-As-Truth

Tests prove arbitrary markdown prose cannot create or update canonical claims. Only recognized structured edit blocks can produce typed correction candidates, and all candidates route through the receipt/service path.

### AC3 -- Typed Correction Write-Back

Editing a projected claim block to mark false/outdated/needs nuance/wrong subject/wrong source/cannot verify produces the expected `FeedbackAction` row through `submit_claim_feedback` and the underlying `record_claim_feedback` writer.

### AC4 -- Immutable Core Preserved

File corrections do not update immutable claim columns directly. Lifecycle/verification changes are service-owned and versioned.

### AC5 -- Signals And Invalidation

Each accepted file correction emits/bounces through the existing feedback signal path and bumps per-entity invalidation. W4 can observe the same effect as app feedback.

### AC6 -- Sidecar Replay Contract

The sidecar contains rebuild-stable semantic identities and typed correction rows sufficient for DOS-832 to replay corrections after fresh UUID generation. Runtime UUIDs are diagnostic only.

### AC7 -- Ambiguity Is Safe

If semantic identity matches multiple fresh claims and `source_ref` + `observed_at` + `source_content_hash` does not disambiguate, replay/apply records an orphan/needs-review status and leaves canonical state unchanged.

### AC8 -- Tombstones Stay Tombstoned

A correction that tombstones/withdraws/supersedes a claim is represented in the sidecar and prevents DOS-832 replay from resurrecting the claim.

### AC9 -- Path Boundary

Projection and apply paths reject files outside the configured workspace root, symlink escapes, `..` traversal, malformed entity slugs, and any path outside `_dailyos_claims`.

### AC10 -- Sensitivity And MCP Carve-Out

Local projection preserves sensitivity metadata. MCP cannot consume the projected files to bypass existing public/internal-only claim gates.

### AC11 -- Prompt Injection Handling

Hostile file-edited content containing fake instructions, XML/HTML delimiters, fake JSON markers, or invisible Unicode remains data. Any prompt-bound path wraps it with `wrap_user_data`; logs and rendered explanations are sanitized.

### AC12 -- Projection Ledger

Projection status is durable, idempotent, repairable, and content-redacted. Failed projection writes are visible to repair tooling without aborting canonical claim writes.

### AC13 -- Producer Trust Audit

This L0 packet includes the runtime-wide claim-producer/trust audit for the write-back path. The W3 L1 PR must verify the implementation still matches that inventory and update it if any producer path changes.

### AC14 -- Real-Data Proof

Dogfood proof uses real local workspace data and redacted output evidence: edit one projected claim file, apply it, observe claim feedback/lifecycle/signal/projection effects, re-render the file, and confirm the correction remains visible.

### AC15 -- Managed Projection Root Exclusion

Workspace ingestion does not ingest `_dailyos_claims/**/*.md` or sidecar files as source evidence. Tests prove projected markdown is excluded from backfill `EntityDoc`/`Notes` classification and rejected by explicit workspace intake / MCP placement paths before the ingestion pipeline opens the file.

## 6. Test Plan

Unit tests:

- Semantic identity generation matches `compute_dedup_key` components.
- `UserNote` identity uses the existing user-note identity kind and never falls back to runtime UUID.
- Subject-ref compacting is deterministic across JSON key order.
- Markdown parser ignores arbitrary prose and only reads bounded claim edit blocks.
- Each supported edit maps to the correct `FeedbackAction`.
- Per-action metadata validation failures leave DB unchanged.
- `WrongSource` goes through receipt validation and rejects mismatched `source_content_hash`.
- `WrongSubject` uses `corrected_subject_ref`; receipt validation forwards it to targeted repair/replay and rejects malformed subject refs.
- Sidecar serialization/deserialization round-trips all required fields.
- Path validator rejects traversal and symlink escapes.
- Workspace backfill and explicit workspace intake reject `_dailyos_claims` projected markdown and sidecars as managed output.
- Redacted logging tests assert raw claim/correction text is absent.

Integration tests:

- Render entity claim file and sidecar from seeded account/project/person claims.
- Apply `NeedsNuance` from a file and verify `claim_feedback`, lifecycle/version event, invalidation bump, signal, and re-render.
- Apply `WrongSubject` with `corrected_subject_ref` and verify the corrected subject reaches targeted repair/replay; malformed subject metadata leaves DB state unchanged.
- Apply `WrongSource` using source-content hash and reject stale/mismatched hashes.
- Apply `CannotVerify` and verify targeted repair enqueue behavior.
- Simulate ambiguous rebuild replay and assert orphan status.
- Simulate tombstone replay and assert no resurrection.
- Run hostile-content fixture through file apply and any prompt-bound path.
- Projection failure records durable status without rolling back canonical feedback.

Required gates for L1 PR:

- `cargo clippy -- -D warnings`
- `cargo test`
- `pnpm tsc --noEmit`

Doc-only L0 packet validation:

- `git diff --check`
- PII/stub marker scan for the packet.
- Local L0 review loop: K-in, feasibility, security, adversarial document review, and `/codex challenge`.

## 7. Intelligence Loop Integration Check

1. Claim model: Files do not create a parallel fact model. They project claims and route edits to typed `FeedbackAction` against canonical claims.
2. Provenance and trust: Projection carries provenance/trust/sensitivity metadata. Write-back preserves provenance by adding user correction rows and relying on existing trust compiler semantics.
3. Signals and invalidation: Accepted corrections emit existing claim feedback signals and bump per-entity invalidation. Projection repair consumes the same invalidation state.
4. Runtime and surfaces: Tauri/app file path is first-party local. MCP parity consumes canonical claims through existing MCP render policy, not projected markdown.
5. Feedback loop: Corrections flow into `claim_feedback`, lifecycle/verification, repair jobs, source/linker reliability effects, and DOS-834 propagation.

## 8. Review Routing

Required local L0 reviewers:

- K-in: `ce-learnings-researcher`.
- Codex challenge: `/codex challenge`.
- Planning reviewer: `ce-feasibility-reviewer`.
- Add security lens because W3 touches filesystem paths, prompt-injection boundaries, sensitivity policy, and claim write paths.
- Add adversarial document review because this packet resolves stale issue text into a safer contract and gates DOS-832.

Codex challenge cycle 1 returned `CHANGES_REQUIRED` on two blockers: `_dailyos_claims` exclusion covered backfill but not explicit workspace intake, and `WrongSubject` metadata did not line up between receipt validation and targeted repair. Both were folded into D2, D8, AC15, and the test plan. Codex challenge cycle 2 returned `APPROVE` with no blocking findings.

## 9. Resolved L0 Decisions

Resolved by this packet:

1. W3 uses a dedicated `claim_file_projection_runs` ledger in migration slot `v280`, plus `v281` for claim membership/index support.
2. The first L1 slice uses an explicit apply command only; no watcher ships in DOS-628.
3. `WrongSource` routes through the receipt-level source-content-hash validator.
4. The authoritative W3 trust audit is the `claim-producers-require-runtime-wide-trust-audit` solution-note checklist, included above as the L0 inventory.

## 10. Done For This Packet

This packet is ready for L0 only when:

- K-in cites prior solution/ADR hits.
- Feasibility confirms the service/file/sidecar shape is buildable against current code.
- Security confirms the narrowed local topology while preserving path, prompt-injection, sensitivity, and log-hygiene boundaries.
- Adversarial review confirms the packet cannot be read as files-as-truth or direct markdown-to-claim mutation.
- `/codex challenge` approves after the cycle-1 blocker folds.
- Linear DOS-628 receives the packet path, verdict summary, and PR link.
