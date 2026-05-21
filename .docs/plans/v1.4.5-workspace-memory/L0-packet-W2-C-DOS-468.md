# L0 Packet — v1.4.5 W2-C — DOS-468 Entity-Seeded Intake via Gutenberg Block

**Current revision:** V1.3 (2026-05-21 local-to-local security trim + cycle-3 compile-shape fold).

## §1 Header

- **Date:** 2026-05-21
- **Project:** v1.4.5 — Workspace Memory Refactor
- **Wave:** W2 stage 2b
- **Lane:** v1.4.5 W2-C — Entity-seeded intake via Gutenberg block
- **Issue:** DOS-468
- **Branch/worktree:** implementing agent decides from wave coordinator instructions
- **Migration slot:** none
- **§0 shared contract:** `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md`
- **Authority docs:** `.docs/plans/v1.4.5-waves.md`; §0 shared contract; `.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md`; `.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md`; `.docs/decisions/0102-abilities-as-runtime-contract.md`; `.docs/decisions/0108-provenance-rendering-and-privacy.md`; `.docs/decisions/0111-surface-independent-ability-invocation.md`; `.docs/decisions/0130-surface-independent-composition-contract.md`; `.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md`
- **L2 reviewer matrix:** codex-review + code-reviewer + architect-reviewer + /cso + plan-devex-review
- **Extra scrutiny:** this lane is the first new block in v1.4.5; extra DX scrutiny is required for scaffolding, editor insertion, render path, fixture ergonomics, and error-state discoverability.
- **Sandbox posture:** workspace-write intent; write only the owned packet and, during implementation, only files listed in §4.
- **Trust topology:** local-to-local single-user: WP block in James's local browser -> loopback HTTP -> Tauri runtime; James is the only principal.

## §2 Changelog

- **V1.3 — 2026-05-21 — local-to-local security trim + cycle-3 compile-shape fold.**
  - Adds the local-to-local single-user trust topology declaration; per memory `feedback_local_to_local_security_overreach_primary_concern`, drops multi-actor gates that do not apply to this surface.
  - Demotes `entity_intake` from `category = Publish` to `category = Transform`. Transform abilities do not require confirmation in the live runtime, resolving cycle-3 codex F1 without inventing confirmation-token transport. The ability still sets `may_publish = true` because the pipeline may write claim/link rows as a side effect of transforming user-provided intake input.
  - Drops scope-helper requirements (`ctx.has_scope()`, scope-gated `resolved_path` redaction). `abilities-runtime` has no scope-context accessor for this use, and W2-C must not invent one.
  - Drops `EntityIntelligenceUpdated` entity-scoped subscription as a security requirement. A broad local refresh is a UX issue in this topology, not a cross-principal leak; defer entity predicate polish to a future ticket if it surfaces.
  - Drops stored-block tamper detection and confirmation-token transport requirements. The user edits their own local posts, and the ability is no longer `Publish`.
  - Fixes cycle-3 compile-shape issues retained from V1.2: `pub async fn`, `ctx: &AbilityContext<'_>`, `allowed_actors = [SurfaceClient]` only, and no `WordPressRender`.
  - Fixes cycle-3 crate-boundary issues by keeping `abilities-runtime` DTOs raw: no `crate::entity::*`, no `crate::services::workspace_ingestion::*`, no `WorkspaceCategoryRegistry` import, and no DB lookup from the ability. `validate_entity_seed` and caller-provided category validation move to the dailyos_lib bridge implementation in W2-A's `workspace_intake_impl.rs`.
  - Uses bare `mcp_exposure = None`, not `McpExposure::None`, per the live macro.
- **V1.2 — 2026-05-21 — cycle-13 + cycle-2 reviewer fold.** Historical superseded revision that introduced the canonical `pub async fn`, `ctx: &AbilityContext<'_>`, `allowed_actors=[SurfaceClient]`, and bridge-DTO direction now corrected by V1.3's Transform category and bridge-owned validation.
- **V1.1 — 2026-05-21 — cycle-1 reviewer fold.** Historical superseded revision that introduced the §0 `WorkspaceIntakeService` bridge, producer/render split, `LinkRepo::add_link` handoff, ADR-0108 sensitivity handling, and initial block/error-state stubs.
- **V1.0 — 2026-05-21 — initial packet.**

## §3 Goal

Verbatim wave-plan body from `.docs/plans/v1.4.5-waves.md` §Agent W2-C:

> ### Agent W2-C — DOS-468: Entity-seeded intake via Gutenberg block
>
> - **Spec:** [DOS-468](https://linear.app/a8c/issue/DOS-468).
> - **Goal:** Ship a Gutenberg block (`dailyos/entity-intake`) that allows a user to associate a workspace document with an entity, trigger ingestion, and see the resulting claim proposals with trust-band rendering. Producer/renderer split per ADR-0130: the block stores `entity_id` + `file_path` as attributes, re-renders by invoking the intake ability on read.
> - **Files owned (exclusive):** `wp/dailyos/blocks/entity-intake/` (new block directory — `block.json`, `render.php`, `edit.js`, `save.js`, styling). New ability `abilities-runtime/src/abilities/entity_intake.rs` (intake ability declaration via `#[ability]` macro). Does NOT own the ingestion pipeline (`IngestPipeline` — W2-A owns); does not own the block scaffolding pattern (reuses v1.4.2 W4 `block.json` pattern).
> - **Don't touch:** `IngestPipeline` implementation (W2-A owns); source management UI (W4-A DOS-472 owns); markdown preview (W4-B DOS-473 owns); watcher (W2-B owns).
> - **Depends on:** W2-A (DOS-466) merged. **Surface Bridge dependency:** v1.4.2 W4 `block.json` scaffolding pattern + trust-band-render helper confirmed merged in v1.4.2. If not yet merged, this lane is blocked — wave coordinator documents and re-schedules.
> - **Intelligence Loop gate:**
>   1. *Claim model:* The block triggers `IngestPipeline::run` via the `entity_intake` ability. Claim proposals are committed by the pipeline, not the block. Block attributes store `entity_id` + `file_ref` only — no claim data stored in block attributes.
>   2. *Provenance + trust:* Trust-band rendering on resulting claims uses the v1.4.0/v1.4.1 trust-band-render helper. No new trust UI primitives.
>   3. *Signals + invalidation:* Block re-renders when `EntityIntelligenceUpdated` signal fires for the entity (uses existing signal subscription via SurfaceClient bridge).
>   4. *Runtime + surfaces:* Block invokes the `entity_intake` ability with `allowed_actors: [SurfaceClient]`, `required_scopes: [write.entity_intake]`, `mcp_exposure: None`. The ability routes to `IngestPipeline::run`.
>   5. *Feedback loop:* User can correct an ingested claim through the existing claim correction block (not in scope for W2-C); the intake block shows resulting claim proposals with trust bands.
> - **Tests required (cycle 3 amendment — fixture-injected claim rendering):** Block renders entity claim list from a fixture-seeded set of `intelligence_claims` rows for the fixture entity (the W2 demo uses hand-seeded claims because W2-A's pipeline produces zero claims; real ingestion-to-claim-render validation is W5-B's job); trust-band colors match `likely_current`/`use_with_caution`/`needs_verification`; intake ability invocation triggers `IngestPipeline::run` in unit test (verifies the wiring fires; the pipeline records the run but produces zero proposals at W2 time, which is the expected W2-A behavior); negative fixture: intake with path traversal path returns error, block shows error state (not a crash).
> - **Done when:** Block activates, renders fixture-seeded claim proposals for a fixture entity with trust bands inline; intake ability invocation triggers `IngestPipeline::run` and writes an ingestion run record (zero proposal expected at W2 time); intake ability declared with correct scopes and `mcp_exposure: None`; path traversal negative fixture green; `/qa-only` L4 surface QA passes in WP Studio against the fixture-seeded entity. Real ingestion-to-claim-render flow validation deferred to W5-B's E2E validation against real workspace data.

V1.1 correction to the quoted V1.0 goal:

- The write path is editor/user-gesture only. `render.php` never invokes the write ability.
- The render path calls `entity_intake_render` or a confirmed existing read ability only, per §0 §11.
- The ability in `abilities-runtime` calls `ctx.services().workspace_intake().ingest(...)` through §0 §4, not app-crate services directly.
- Attribute wording is canonicalized to `entity_id`, `entity_type`, and `file_ref` only. `file_path` in the wave text means the registry-relative `file_ref` token, not an absolute path.

V1.3 correction to the quoted V1.0/V1.1 goal:

- `entity_intake` is `category=Transform`, `may_publish=true`, `allowed_actors=[SurfaceClient]`, `required_scopes=["write.entity_intake"]`, and `mcp_exposure=None`. The Transform category is the architectural decision for V1.3: user-provided intake input is transformed into workspace-memory state, and downstream claim/link writes are side effects of that transformation.
- No confirmation-token transport is required or owned by this lane.
- Local-to-local single-user topology collapses multi-principal security gates. W2-C still owns data-hygiene validation, render escaping, path traversal rejection, and ADR-0108 log/screenshot/output redaction.
- `IngestionMode::EntitySeeded` is valid only because cycle-13 §13.1 makes the W1-extension PR a W2 L1 precondition.

## §4 Files Owned Exclusive

Verbatim wave-plan ownership:

> `wp/dailyos/blocks/entity-intake/` (new block directory — `block.json`, `render.php`, `edit.js`, `save.js`, styling). New ability `abilities-runtime/src/abilities/entity_intake.rs` (intake ability declaration via `#[ability]` macro). Does NOT own the ingestion pipeline (`IngestPipeline` — W2-A owns); does not own the block scaffolding pattern (reuses v1.4.2 W4 `block.json` pattern).

V1.3 ownership corrections for this checkout:

- `src-tauri/abilities-runtime/src/services/workspace_intake.rs` is **W2-A-owned**, not W2-C-owned, per §0 V1.2 §4 and cycle-13 §13.3.1. W2-C consumes `WorkspaceIntakeService` and its DTOs; W2-C must not alter the trait contract or bridge registration unless W2-A's companion V1.3 packet explicitly assigns the bridge-surface update.
- `src-tauri/abilities-runtime/src/abilities/entity_intake.rs` is W2-C-owned. This is the write ability and must use `category=Transform`, `may_publish=true`, `allowed_actors=[SurfaceClient]`, raw abilities-runtime DTOs, and the §0 V1.2 §13 canonical function shape.
- `src-tauri/abilities-runtime/src/abilities/entity_intake_render.rs` is W2-C-owned **only if** no existing read ability can satisfy §9. The first reuse candidate is the existing claim-read substrate cited in §6 (`EntityContextClaimReadHandle` plus the live claim reader); if reused, W2-C documents that reuse and owns no new read ability file.
- Rust validation helpers move out of W2-C:
  - `validate_entity_seed` belongs in W2-A's dailyos_lib-side `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs`, because it needs typed `EntityType` parsing and DB existence lookup.
  - `WorkspaceCategoryRegistry::validate` also belongs in the bridge implementation, because the ability passes raw `category_slug: Option<String>` and the bridge converts to the internal typed request.
  - W2-C depends on W2-A V1.3 absorbing this bridge-helper section before L1 implementation. In this checkout W2-A is still V1.2, so W2-C L1 is blocked until that companion packet is updated or the coordinator assigns the bridge change.
- `wp/dailyos/blocks/entity-intake/` is W2-C-owned:
  - `block.json`
  - `edit.js`
  - `save.js`
  - `render.php`
  - `view.js` / `view.asset.php` only if the SurfaceClient subscription bridge requires a front-end re-render hook for `EntityIntelligenceUpdated`
  - `style.css`
  - `editor.css`
  - generated `*.asset.php` files matching the repo's block build pattern
- `wp/dailyos/blocks/_shared/dailyos_block_error_panel.php` is W2-C-owned to create if it does not already exist. If the helper already exists, W2-C reuses it and does not fork a parallel error-panel primitive.

Ownership notes:

- The block is dynamic: `save.js` returns `null`; `render.php` owns output.
- Block attributes store only `entity_id`, `entity_type`, and `file_ref`.
- Claim rows, claim payloads, trust scores, provenance payloads, local filesystem paths, customer data, generated receipts, and typed bridge rejection payloads are never serialized into block attributes.
- `LinkRepo::add_link` writes may happen inside the §0 §4 dailyos_lib `WorkspaceIntakeService` implementation. If W2-A leaves link creation to W2-C, W2-C performs it only through the service bridge, not through direct app-crate imports inside `abilities-runtime`.

## §5 Don't Touch

Verbatim wave-plan deny list:

> `IngestPipeline` implementation (W2-A owns); source management UI (W4-A DOS-472 owns); markdown preview (W4-B DOS-473 owns); watcher (W2-B owns).

Additional local guardrails:

- Do not edit `src-tauri/src/services/workspace_ingestion/pipeline.rs`; W2-A owns the implementation and the `WorkspaceIntakeService` adapter.
- Do not edit `src-tauri/abilities-runtime/src/services/workspace_intake.rs`; W2-A owns the trait contract per §0 §4.
- Do not edit source-management, markdown-preview, watcher, or W4 block scaffolding outside the entity-intake peer directory.
- Do not add migrations for this lane.
- Do not add MCP exposure, adapter tools, or external enumeration metadata for either ability.

## §6 K-in Substrate Audit

| Surface | K-in finding | Required implementation consequence |
| --- | --- | --- |
| §0 V1.2 §4 | `WorkspaceIntakeService` is the crate-boundary trait. `abilities-runtime` declares it; dailyos_lib implements it; `AbilityContext.services()` exposes the `ServiceContext` accessor per `src-tauri/abilities-runtime/src/abilities/registry.rs:740-768`. | `entity_intake` calls `ctx.services().workspace_intake().ingest(...)` with raw abilities-runtime DTOs. It must not import `dailyos_lib::services::workspace_ingestion`, app-crate entity types, `WorkspaceCategoryRegistry`, or instantiate `IngestPipeline`. |
| §0 V1.2 §11 | Render-time mutation is prohibited. Write trigger and render read are separate flows. | `edit.js` invokes `entity_intake` after a user gesture. `render.php` invokes only `entity_intake_render` or an existing read ability. |
| §0 V1.2 §13 | Canonical ability shape is pinned to `src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-115`: `pub async fn`, `ctx: &AbilityContext<'_>`, and `AbilityResult<T>` returning `T` directly. | §9 stubs must not use `ServiceContext`, sync functions, or hand-built `AbilityOutput` wrappers. §10 adds a literal-diff guard against this skeleton. |
| §0 §10 | Substrate-reuse table replaces `EntityIntakeClaimBand { trust_band: String }` with canonical `TrustBand`. | Output claims use `abilities-runtime/src/abilities/trust/types.rs:24-29` `TrustBand`; no local trust enum or string-only band DTO. |
| ActorKind variants | `src-tauri/abilities-runtime/src/abilities/registry.rs:468-486` defines only `Agent`, `User`, `Admin`, `System`, `SurfaceClient`, and `McpClient`. `WordPressRender` is not real. | Both write and read ability stubs use `allowed_actors=[SurfaceClient]`; render/write differentiation is by scope and route, not actor. |
| ADR-0102 | Ability category, actor, scope, confirmation, and MCP exposure are runtime contract, not UI hints. `src-tauri/abilities-runtime/src/abilities/registry.rs:1495-1501` gates `Publish` through confirmation. | V1.3 sets the write ability to `category=Transform`, `may_publish=true`, `allowed_actors=[SurfaceClient]`, `required_scopes=["write.entity_intake"]`, and bare `mcp_exposure=None`. Transform avoids the live Publish confirmation gate while still allowing downstream substrate writes as transformation side effects. |
| ADR-0108 | Provenance rendering, privacy, and sensitivity policy apply to all claim-bearing surfaces. | Rust filters claims by sensitivity threshold before returning; PHP renders only the returned projection. |
| ADR-0111 | SurfaceClient bridge and scope allowlist govern surface invocation. | WP editor and render paths use paired runtime client/scope bridge; no bespoke HTTP client. |
| ADR-0130 | Producer/renderer split keeps durable attributes separate from rendered output. | `save.js` returns null; attributes hold only identifiers; render reads a projection. |
| Link writes | `src-tauri/src/services/workspace_ingestion/link.rs:215-224` pins `LinkRepo::add_link(conn, file_id, entity_type, entity_id, LinkAttributionSource, confidence, rationale, actor)`. | Entity-seeded link creation uses `LinkAttributionSource::EntityIntake`, full entity triple, idempotent behavior, and a unit test. |
| Claim read prior art | `src-tauri/abilities-runtime/src/services/context.rs:866-877` defines `EntityContextClaimReadHandle`; `src-tauri/src/services/context.rs:80-96` implements the live reader; `src-tauri/src/services/claims.rs:9165-9198` reads active claims for a surface. | Prefer this read substrate for `entity_intake_render` unless an even more specific read ability already exists. |
| Sensitivity substrate | `src-tauri/abilities-runtime/src/types.rs:37-47` defines `ClaimSensitivity`; `src-tauri/src/services/sensitivity.rs:109-145` maps policy by sensitivity and surface. | `EntityIntakeOutput.claims[]` includes sensitivity after the Rust gate has already filtered disallowed rows. |
| Category parsing vs validation | `src-tauri/src/services/workspace_ingestion/contracts.rs:95-99` says `WorkspaceCategory::from_slug` is lexical only. `src-tauri/src/services/workspace_ingestion/registry.rs:358-362` is the canonical `WorkspaceCategoryRegistry::validate(conn, &category, entity_type)` signature. | V1.3 moves caller-provided category validation into W2-A's dailyos_lib bridge implementation. The ability passes raw `category_slug: Option<String>`; the bridge parses, validates against the typed entity, and returns `InvalidCategorySlug` or `CategoryNotAllowed` before constructing the internal typed request. |
| WP editor permission | `wp/dailyos/includes/class-dailyos-plugin.php:655-665` gates editor REST routes through login, `edit_posts`, and pairing. Existing registered editor routes are at `wp/dailyos/includes/class-dailyos-plugin.php:609-647`. | `edit.js` write-trigger route must use `can_edit_posts_rest` or stricter. No existing entity-intake route fits; §9 names the required route unless a generic SurfaceClient invoke route lands first. |
| WP block category | In this checkout the requested `wp/dailyos/includes/class-dailyos-plugin.php:651-670` range is the editor REST permission callback, not block-category registration. The actual `"dailyos"` block category registration is `wp/dailyos/includes/class-dailyos-plugin.php:162-182`. | `block.json` uses `"category": "dailyos"` and §11 verifies the category exists through the actual registration lines. |
| Entity existence/data-hygiene substrate | `src-tauri/src/services/trust_extraction.rs:148-164` shows entity existence checks by typed entity table. | In this local single-user topology, entity authorization reduces to data hygiene: the bridge parses `entity_type`, validates `entity_id`, and rejects `EntityNotFound` when the row is absent. W2-C does not invent a cross-principal ACL helper. |

Surface Bridge dependency checks:

- Existing block prior art is present in this worktree: `wp/dailyos/blocks/account-overview/`.
- Existing trust-band helper is present in this worktree: `wp/dailyos/blocks/trust-band-badge/render-functions.php`.
- Actual ability path in this checkout is `src-tauri/abilities-runtime/src/abilities/`, not repo-root `abilities-runtime/src/abilities`.
- Required L1 kickoff dev-diff command:

```bash
git diff <fork-sha>..dev -- wp/dailyos/blocks/trust-band-badge/ wp/dailyos/blocks/account-overview/
```

L0 outcome: not run against `dev`. The v1.4.5 W1 stream is blocked on v1.4.4 per `HANDOFF-2026-05-21.md`, so this packet records the dev-diff as a W2-C L1 kickoff precondition: before W2-C L1 starts, run the dev-diff and either confirm scaffolding is on `dev` or document the gap and unblock via wave coordinator.

## §7 Security Gate

This is a local-to-local single-user block invocation surface, not a generic external tool and not a multi-principal authorization boundary.

- The write ability uses `allowed_actors = [SurfaceClient]`, `required_scopes = ["write.entity_intake"]`, `category = Transform`, `may_publish = true`, and `mcp_exposure = None`.
- The read ability uses `allowed_actors = [SurfaceClient]`, `required_scopes = ["read.entity_intelligence"]`, `category = Read`, `may_publish = false`, and `mcp_exposure = None`.
- The render path calls only the read ability (`entity_intake_render`) or a confirmed existing read ability. It must never call `entity_intake`.
- The block calls through the paired SurfaceClient bridge and signed runtime client. PHP and browser JavaScript never read workspace files directly.
- The ability routes raw `file_ref`, `entity_seed`, and `category_slug` values into §0 V1.2 §4 `WorkspaceIntakeService`; W2-A's dailyos_lib bridge validates the registry-relative path, parses entity/category strings, checks entity existence, and calls the W2-A pipeline boundary.
- The ability must not accept absolute customer paths as durable block attributes; store only the registry-relative `file_ref` needed by the pipeline.
- ADR-0093 untrusted-document-to-AI handling is not W2-C-owned because this lane does not perform extraction or AI calls; that boundary lands with W3-A.
- `/cso` reviews at L0 and L2 are mandatory, scoped to the local trust topology.

V1.3 security/data-hygiene gates:

- **F1 ADR-0108 output/log redaction:** `EntityIntakeOutput.claims[]` carries `ClaimSensitivity`; `EntityIntakeClaim` is `{ claim_id, trust_band: TrustBand, sensitivity: ClaimSensitivity }`. Rust filters/redacts per ADR-0108 before returning claim projections, and PHP escapes only the already-filtered projection. This is for logs/screenshots/output hygiene, not a cross-viewer authorization gate.
- **F2 path traversal protection:** W2-A's bridge calls `registry::open_validated`; traversal values such as `../outside.md`, `%2e%2e/outside.md`, absolute paths outside the workspace, or trailing-space/dot components return `PathTraversalAttempt`.
- **F3 string hygiene:** `file_ref` and optional `category_slug` are slug/path-component validated before storage/log use. `render.php` escapes every block attribute and display string to avoid displayable XSS.
- **F4 entity existence:** in the single-user case, entity "authorization" means the typed entity exists in the local DB. The bridge returns `EntityNotFound` when it does not.
- **F5 category validation:** the bridge parses and validates caller-provided category slugs against `WorkspaceCategoryRegistry`; invalid or disallowed slugs return typed `InvalidCategorySlug` / `CategoryNotAllowed` errors before the internal typed request is constructed.
- **F6 block category registration:** `block.json` must register under `"dailyos"`, and §11 verifies the actual category registration in `wp/dailyos/includes/class-dailyos-plugin.php:162-182`. The requested `:651-670` range is separately cited in §6 as the editor REST permission callback in this checkout.

Explicitly dropped for V1.3:

- No confirmation-token transport requirement; `category=Transform` avoids the live `Publish` confirmation gate.
- No `ctx.has_scope()` or `read.entity_names` redaction rule; `resolved_path` passes through from the bridge.
- No cross-entity subscription security gate; broad local refresh is UX polish, not a security boundary.
- No stored-block tamper detection gate; the user edits their own local posts.

Security acceptance checks:

- A traversal value returns `PathTraversalAttempt` through the shared block error panel.
- Unknown entity returns `EntityNotFound`; invalid entity type/id returns `InvalidEntityType` / `InvalidEntityId`; invalid category returns `InvalidCategorySlug` / `CategoryNotAllowed`; missing pairing returns `RuntimeNotPaired`; runtime/pipeline failure returns `IngestionFailed`.
- Error rendering escapes every value and does not include raw local filesystem paths, raw provenance payloads, claim body content, or customer-specific data.
- Success rendering escapes all claim display strings and uses the trust-band helper for band labels.
- No customer-specific data, PII, claim body content, provenance payload, local filesystem path, or generated receipt is stored in block attributes.
- No ephemeral ticket references appear inside code comments, block metadata, CSS, JS, PHP, or Rust stubs.

## §8 Intelligence Loop Gate

V1.3 amended gate:

1. *Claim model:* `edit.js` triggers the write ability after a user gesture. The write ability calls §0 V1.2 §4 `WorkspaceIntakeService::ingest` with raw `source_type_slug="entity_doc"` and `mode_slug="entity_seeded"`; W2-A's bridge maps those to `WorkspaceFileKind::EntityDoc` and `IngestionMode::EntitySeeded` after validation. Claim proposals are committed by the pipeline, not the block. Block attributes store `entity_id`, `entity_type`, and `file_ref` only.
2. *Provenance + trust:* Trust-band rendering uses canonical `TrustBand` and the existing trust-band-render helper. No new trust UI primitive.
3. *Signals + invalidation:* The block may re-render on `EntityIntelligenceUpdated`. V1.3 does not require entity-scoped subscription filtering for security; if broad local refresh becomes noisy, track it as UX polish outside W2-C.
4. *Runtime + surfaces:* `entity_intake` is a Transform ability with `allowed_actors=[SurfaceClient]`, `required_scopes=[write.entity_intake]`, `mcp_exposure=None`, `category=Transform`, and `may_publish=true`. `entity_intake_render` is a read ability with `allowed_actors=[SurfaceClient]`, `required_scopes=[read.entity_intelligence]`, `mcp_exposure=None`, `category=Read`, and `may_publish=false`.
5. *Feedback loop:* User correction of claims is out of W2-C scope unless a confirmed claim-correction block already exists. W2-C must not invent a feedback surface; it only renders resulting claim proposals with trust bands and links to the existing correction path if that path is confirmed.

## §9 Code Stub

These stubs show contract shape only. Implementing agents adapt module paths to W2-A's final §0 V1.2 §4 trait files without changing actor, scope, MCP exposure, Transform category, bridge-owned validation, path validation, ADR-0108 redaction, or render-time mutation posture.

V1.3 hard pins:

- Function skeleton mirrors `src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-115` per cycle-13 §13.3.4: `pub async fn`, `ctx: &AbilityContext<'_>`, and returns the inner output type directly.
- `AbilityContext` is canonical per `src-tauri/abilities-runtime/src/abilities/registry.rs:740-768`; `ServiceContext` is not an ability function parameter.
- `allowed_actors=[SurfaceClient]` only; `WordPressRender` is not in `ActorKind` at `src-tauri/abilities-runtime/src/abilities/registry.rs:468-486`.
- `category=Transform`, not `Publish`. Transform does not require confirmation in the live runtime.
- `mcp_exposure=None` is bare macro syntax.
- The ability does not import app-crate types (`crate::entity::*`, `crate::services::workspace_ingestion::*`) and does not call `WorkspaceCategoryRegistry::validate`, `ctx.services().conn()`, or `ctx.has_scope()`.
- Entity/category parsing, category allowedness, entity existence check, and internal typed request construction live in W2-A's dailyos_lib bridge implementation.

### Rust write ability — `entity_intake`

```rust
use crate::abilities::registry::{AbilityContext, AbilityResult};
use crate::services::workspace_intake::{
    EntityRefDto, WorkspaceIntakeRequest, WorkspaceIntakeService,
};
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::types::ClaimSensitivity;
use dailyos_abilities_macro::ability;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EntityIntakeInput {
    pub file_ref: String,
    pub entity_seed: Option<EntityRefDto>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityIntakeOutput {
    pub run_id: String,
    pub file_id: String,
    pub resolved_path: Option<String>,
    pub claims: Vec<EntityIntakeClaim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityIntakeClaim {
    pub claim_id: String,
    pub trust_band: TrustBand,
    pub sensitivity: ClaimSensitivity,
}

#[derive(Debug, Clone, Serialize)]
pub enum EntityIntakeError {
    InvalidEntityType(String),
    InvalidEntityId(String),
    EntityNotFound,
    InvalidCategorySlug(String),
    CategoryNotAllowed { allowed: Vec<String> },
    FileNotFound,
    PathTraversalAttempt,
    IngestionFailed(String),
}

#[ability(
    name = "entity_intake",
    category = Transform,  // V1.3: was Publish; Transform doesn't require confirmation
    may_publish = true,    // still emits claim/link rows downstream
    allowed_actors = [SurfaceClient],
    required_scopes = ["write.entity_intake"],
    mcp_exposure = None,   // V1.3: bare, not McpExposure::None
)]
pub async fn entity_intake(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeInput,
) -> AbilityResult<EntityIntakeOutput> {
    // V1.3: validation lives in the bridge impl, NOT in the ability.
    // The ability passes the raw input through to the workspace_intake service;
    // the dailyos_lib impl owns the type parsing + category validation +
    // entity existence check.
    let receipt = ctx.services().workspace_intake().ingest(ctx, WorkspaceIntakeRequest {
        file_ref: input.file_ref,
        source_type_slug: "entity_doc".to_string(),  // bridge maps to WorkspaceFileKind
        entity: input.entity_seed,  // Option<EntityRefDto>
        mode_slug: "entity_seeded".to_string(),
        category_slug: input.category,  // Option<String>
    }).await?;
    let claims = read_claims_for_block_render(ctx, &receipt.file_id).await?;

    Ok(EntityIntakeOutput {
        run_id: receipt.run_id,
        file_id: receipt.file_id,
        resolved_path: receipt.resolved_path,  // V1.3: no scope gating; pass through
        claims,
    })
}
```

Bridge-owned validation contract:

- Trim and parse `entity_seed.entity_type` to typed `EntityType` inside dailyos_lib.
- Reject unknown slugs with `EntityIntakeError::InvalidEntityType(String)`; do not coerce unknown values.
- Validate `entity_seed.entity_id` is non-empty and matches the accepted UUID/slug shape before wrapping as the internal `EntityId`.
- Check the typed entity exists in the local DB; return `EntityNotFound` if absent.
- Parse `category_slug` inside the bridge, then call `WorkspaceCategoryRegistry::validate(conn, &category, entity_type)` before constructing the internal typed request. Return `InvalidCategorySlug(String)` or `CategoryNotAllowed { allowed }` on failure.

### Required crate-boundary DTO shape

V1.3 pins the raw shape W2-C consumes from W2-A's `src-tauri/abilities-runtime/src/services/workspace_intake.rs` bridge. These types live in `abilities-runtime` and contain no app-crate imports. If W2-A names fields differently at L1, W2-C updates the imports only after §0 is updated; it must not invent parallel DTOs.

```rust
#[derive(Debug, Clone)]
pub struct WorkspaceIntakeRequest {
    pub file_ref: String,
    pub source_type_slug: String,
    pub entity: Option<EntityRefDto>,
    pub mode_slug: String,
    pub category_slug: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntityRefDto {
    pub entity_type: String,
    pub entity_id: String,
    pub entity_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceIntakeReceipt {
    pub run_id: String,
    pub file_id: String,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WorkspaceIntakeError {
    InvalidEntityType(String),
    InvalidEntityId(String),
    EntityNotFound,
    InvalidCategorySlug(String),
    CategoryNotAllowed { allowed: Vec<String> },
    FileNotFound,
    PathTraversalAttempt,
    IngestionFailed(String),
}
```

Link write contract:

- `WorkspaceIntakeService` impl or explicit post-ingest hook writes `LinkRepo::add_link` with `LinkAttributionSource::EntityIntake`.
- Confidence/rationale values are deterministic and documented by W2-A/W2-C at L1.
- Link write is idempotent for repeated editor clicks and never runs from `render.php`.
- W2-C does not perform the link write inside `abilities-runtime`; the bridge/pipeline side owns it.

### Rust read ability — `entity_intake_render`

```rust
use crate::abilities::registry::{AbilityContext, AbilityResult};
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::types::ClaimSensitivity;
use dailyos_abilities_macro::ability;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct EntityIntakeRenderInput {
    pub entity_type: String,
    pub entity_id: String,
    pub file_ref: String,
}

#[ability(
    name = "entity_intake_render",
    category = Read,
    may_publish = false,
    allowed_actors = [SurfaceClient],
    required_scopes = ["read.entity_intelligence"],
    mcp_exposure = None,
)]
pub async fn entity_intake_render(
    ctx: &AbilityContext<'_>,
    input: EntityIntakeRenderInput,
) -> AbilityResult<EntityIntakeOutput> {
    let receipt = read_public_receipt_for_file_ref(ctx, &input).await?;
    let claims = read_claims_for_block_render(ctx, &receipt.file_id).await?;

    Ok(EntityIntakeOutput {
        run_id: receipt.run_id,
        file_id: receipt.file_id,
        resolved_path: receipt.resolved_path,
        claims,
    })
}
```

If an existing read ability is reused, it must satisfy the same output shape, ADR-0108 output/log redaction, entity-exists data hygiene, and MCP-hidden constraints. Reuse must be recorded in §11 at L1.

### `block.json`

```json
{
  "$schema": "https://schemas.wp.org/trunk/block.json",
  "apiVersion": 3,
  "name": "dailyos/entity-intake",
  "title": "Entity Intake",
  "category": "dailyos",
  "description": "Associates a workspace document with an entity and renders resulting claim trust bands.",
  "supports": { "html": false, "reusable": false, "inserter": true },
  "attributes": {
    "entity_id": { "type": "string", "default": "" },
    "entity_type": { "type": "string", "default": "" },
    "file_ref": { "type": "string", "default": "" }
  },
  "render": "file:./render.php",
  "editorScript": "file:./edit.js",
  "style": "file:./style.css",
  "editorStyle": "file:./editor.css"
}
```

### `edit.js`

```js
import apiFetch from '@wordpress/api-fetch';
import { useState } from '@wordpress/element';

export default function Edit({ attributes, setAttributes }) {
	const [state, setState] = useState({ kind: 'idle' });
	const ingest = async () => {
		setState({ kind: 'pending' });
		try {
			await apiFetch({
				path: '/dailyos/v1/entity-intake/ingest',
				method: 'POST',
				data: {
					entity_seed: {
						entity_id: attributes.entity_id,
						entity_type: attributes.entity_type,
					},
					file_ref: attributes.file_ref,
				},
			});
			setState({ kind: 'done' });
		} catch (error) {
			setState({ kind: 'error', error });
		}
	};

	return (
		<div className="dailyos-entity-intake-editor">
			{/* entity_type, entity_id, and file_ref controls update block attributes. */}
			<button type="button" onClick={ingest} disabled={state.kind === 'pending'}>Ingest</button>
			{state.kind === 'error' && <div role="alert">{state.error.message}</div>}
		</div>
	);
}
```

Editor route contract: existing routes in `wp/dailyos/includes/class-dailyos-plugin.php:609-647` cover nonce and account-overview surfaces only. No existing route fits entity-intake. The required route, if no generic SurfaceClient invoke route lands first, is `POST /wp-json/dailyos/v1/entity-intake/ingest` in `wp/dailyos/includes/class-dailyos-plugin.php` using `can_edit_posts_rest` from `:655-665`. Because that PHP file is not in §4 ownership, W2-C must get coordinator assignment before editing it; otherwise it must reuse a confirmed generic route. The route invokes `entity_intake` with `["write.entity_intake"]`.

Editor preview strategy: no live preview. The write ability mutates and is user-gesture only. The editor shows a configured-state placeholder and an `Ingest` button that calls the WP REST bridge and invokes `entity_intake`.

### `save.js`

```js
export default function save() {
	return null;
}
```

`save.js` returns `null` per ADR-0130; all durable block state is in attributes and all rendering is server-side.

### `render.php`

```php
<?php
declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

require_once __DIR__ . '/../_shared/dailyos_block_error_panel.php';

$attributes  = isset( $attributes ) && is_array( $attributes ) ? $attributes : [];
$entity_id   = dailyos_entity_intake_attr( $attributes, 'entity_id' );
$entity_type = dailyos_entity_intake_attr( $attributes, 'entity_type' );
$file_ref    = dailyos_entity_intake_attr( $attributes, 'file_ref' );

$validation = dailyos_entity_intake_validate_attrs( $entity_type, $entity_id, $file_ref );
if ( is_wp_error( $validation ) ) {
	echo dailyos_block_error_panel( 'dailyos/entity-intake', $validation->get_error_code() );
	return;
}

$runtime_client = apply_filters( 'dailyos_runtime_client_for_block', null );
if ( ! is_object( $runtime_client ) || ! method_exists( $runtime_client, 'invoke_ability' ) ) {
	echo dailyos_block_error_panel( 'dailyos/entity-intake', 'RuntimeNotPaired' );
	return;
}

$response = $runtime_client->invoke_ability(
	'entity_intake_render',
	[
		'entity_id'   => $entity_id,
		'entity_type' => $entity_type,
		'file_ref'    => $file_ref,
	],
	[ 'read.entity_intelligence' ]
);

if ( is_wp_error( $response ) ) {
	echo dailyos_block_error_panel( 'dailyos/entity-intake', dailyos_entity_intake_error_kind( $response ) );
	return;
}

echo dailyos_entity_intake_render_claims( $response, $attributes );
```

`render.php` error variants are discriminated and render distinct block states via shared `dailyos_block_error_panel`: `InvalidEntityType`, `InvalidEntityId`, `EntityNotFound`, `InvalidCategorySlug`, `CategoryNotAllowed`, `FileNotFound`, `PathTraversalAttempt`, `RuntimeNotPaired`, `IngestionFailed`.

## §10 Tests Required

Verbatim wave-plan test paragraph:

> **Tests required (cycle 3 amendment — fixture-injected claim rendering):** Block renders entity claim list from a fixture-seeded set of `intelligence_claims` rows for the fixture entity (the W2 demo uses hand-seeded claims because W2-A's pipeline produces zero claims; real ingestion-to-claim-render validation is W5-B's job); trust-band colors match `likely_current`/`use_with_caution`/`needs_verification`; intake ability invocation triggers `IngestPipeline::run` in unit test (verifies the wiring fires; the pipeline records the run but produces zero proposals at W2 time, which is the expected W2-A behavior); negative fixture: intake with path traversal path returns error, block shows error state (not a crash).

V1.3 additional required tests:

- §9 literal-diff test: assert the `entity_intake` function skeleton matches `src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-115` character-by-character for `pub async fn`, `ctx: &AbilityContext<'_>`, input parameter, and `AbilityResult<EntityIntakeOutput>` return shape, adjusted only for function/input/output names. This is the cycle-13 §13.6 K-out prevention guard.
- Write ability bridge test: `entity_intake` uses injected `WorkspaceIntakeService`; it imports no app-crate entity, registry, DB, or pipeline modules.
- Ability metadata test: `entity_intake` is `category=Transform`, `may_publish=true`, `allowed_actors=[SurfaceClient]`, `required_scopes=["write.entity_intake"]`, and `mcp_exposure=None`; it passes the live runtime check without confirmation.
- Render split test: `render.php` invokes only `entity_intake_render`; repeated renders do not mutate ingestion runs or links.
- Fixture claim render test: `entity_intake_render` reads fixture-seeded `intelligence_claims` rows and returns renderable `claims[]` with canonical `TrustBand` values.
- ADR-0108 output/log redaction test: sensitive claim projections are filtered/redacted before output crosses the ability boundary; PHP never sees raw confidential claim body/provenance text.
- `validate_entity_seed` bridge test: unknown `entity_type` returns `InvalidEntityType`, invalid/empty `entity_id` returns `InvalidEntityId`, and absent entity rows return `EntityNotFound`.
- Invalid category slug bridge test: malformed slugs return `InvalidCategorySlug`; registry-disallowed categories return `CategoryNotAllowed`; the internal typed request is not constructed on either failure.
- Path traversal test: traversal/absolute/encoded traversal `file_ref` values return `PathTraversalAttempt`.
- MCP non-enumeration test: `entity_intake` and `entity_intake_render` are not enumerated and cannot be invoked through MCP.
- Typed error coverage test: every `EntityIntakeError` variant is covered by Rust mapping and PHP block-state rendering.
- Discriminated error-state test: each of `InvalidEntityType`, `InvalidEntityId`, `EntityNotFound`, `InvalidCategorySlug`, `CategoryNotAllowed`, `FileNotFound`, `PathTraversalAttempt`, `RuntimeNotPaired`, and `IngestionFailed` renders a unique block state through `dailyos_block_error_panel`.
- Link write test: entity-seeded intake creates or reuses `document_entity_links` through `LinkRepo::add_link(..., EntityIntake, ...)` idempotently when the W2-A service impl owns that hook.
- Repeat-render idempotency test: repeated renders do not create ingestion runs, links, or claim rows.
- Block category registration test: `block.json` uses `"category": "dailyos"` and the plugin registers that category via `wp/dailyos/includes/class-dailyos-plugin.php:162-182`.
- Durable-attribute privacy test: serialized post content contains only `entity_id`, `entity_type`, and `file_ref`.
- `/qa-only` WP Studio pass against the fixture entity.

Command gates for L2: `cargo test -p abilities-runtime entity_intake`; `cargo test -p dailyos_lib workspace_ingestion`; `pnpm --dir wp/dailyos build`; `pnpm --dir wp/dailyos lint:js`; `composer --working-dir=wp/dailyos test`; `composer --working-dir=wp/dailyos lint`; `composer --working-dir=wp/dailyos grep-gates`; and W2-A's workspace mutation allowlist gate once its script lands.

## §11 Done When

Verbatim wave-plan done criteria:

> Block activates, renders fixture-seeded claim proposals for a fixture entity with trust bands inline; intake ability invocation triggers `IngestPipeline::run` and writes an ingestion run record (zero proposal expected at W2 time); intake ability declared with correct scopes and `mcp_exposure: None`; path traversal negative fixture green; `/qa-only` L4 surface QA passes in WP Studio against the fixture-seeded entity. Real ingestion-to-claim-render flow validation deferred to W5-B's E2E validation against real workspace data.

V1.3 additional done criteria:

- Surface Bridge dev-diff confirmed, or explicit gap-on-`dev` escalation recorded before W2-C L1 starts.
- `WorkspaceIntakeService` trait and dailyos_lib bootstrap impl are wired by W2-A. W2-A V1.3 also absorbs bridge-side entity/category validation in `workspace_intake_impl.rs`; if not, W2-C L1 is blocked.
- `entity_intake` uses the §0 V1.2 §4 trait bridge and never imports app-crate ingestion services directly.
- `entity_intake` matches the §0 V1.2 §13 canonical ability shape: `pub async fn`, `ctx: &AbilityContext<'_>`, and returns `EntityIntakeOutput` directly.
- `entity_intake_render` exists or existing read-ability reuse is confirmed and documented.
- `entity_intake_render` returns a claim list filtered/redacted per ADR-0108 before output crosses the ability boundary.
- `render.php` invokes only the read ability and passes only `read.entity_intelligence`.
- `edit.js` invokes the write ability only through a user gesture and the scoped REST bridge.
- Write ability is `category=Transform`, `may_publish=true`, `allowed_actors=[SurfaceClient]`, `required_scopes=["write.entity_intake"]`, `mcp_exposure=None`, and passes the runtime check without a confirmation requirement.
- Read ability is `category=Read`, `may_publish=false`, `allowed_actors=[SurfaceClient]`, `required_scopes=["read.entity_intelligence"]`, `mcp_exposure=None`.
- Canonical `TrustBand` and `ClaimSensitivity` are used; no local `EntityIntakeClaimBand` enum or stringly trust DTO remains.
- `resolved_path` passes through from the W2-A bridge; there is no `read.entity_names` scope gate in W2-C.
- `WorkspaceCategoryRegistry::validate` runs inside W2-A's dailyos_lib bridge before internal typed request construction; invalid category never reaches the internal `IngestRequest`.
- Link write with `LinkAttributionSource::EntityIntake` is implemented in the service layer or explicitly confirmed as W2-A service behavior.
- Entity existence, ADR-0108 output/log redaction, render-time idempotency, MCP non-enumeration, typed error coverage, and path/category hygiene tests are green.
- `save.js` returns `null`.
- `block.json` registers under `"dailyos"` and the plugin category registration is verified against `wp/dailyos/includes/class-dailyos-plugin.php:162-182`.
- Shared helper `wp/dailyos/blocks/_shared/dailyos_block_error_panel.php` exists and is used by `render.php`.
- `/qa-only` WP Studio pass is recorded against the fixture-seeded entity.

## §12 Reviewer Panel

- codex challenge
- codex consult
- architect-reviewer
- `/cso`
- `/plan-devex-review`

**Pass rule:** unanimous APPROVE.

## PATH-α Appendix

These are non-blocking improvements unless a reviewer upgrades one to acceptance criteria:

- Add block scaffolding generator script `pnpm gen:block` for future WP block lanes.
- Promote shared `dailyos_block_error_panel` as a documented `wp/dailyos/blocks/_shared/` primitive.
