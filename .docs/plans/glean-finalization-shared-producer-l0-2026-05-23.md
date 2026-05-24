# Shared Glean Finalization Producer L0 Packet — 2026-05-23

Status: L0 approved; implementation in progress
Branch: `account-fact-claim-producer`
Origination class: debug-driven extension
Scope tier: Standard, with security/trust review required because this touches claim provenance, signal propagation, trust inputs, and write-path ownership. This is a focused implementation slice under `.docs/plans/abilities-runtime-producer-remediation-waves.html`, not the full wave.
Threat topology: local-to-local, single-user, encrypted local DB

## Origination and Symptom Trace

User-visible symptom, sanitized per the no-customer-data rule: Claude Desktop could reach the DailyOS MCP runtime for an account briefing prompt, but the DailyOS answer was thinner than the generated export artifacts and missed recent meeting / commercial context that should have been available through Glean-backed DailyOS intelligence.

Trace from surface to suspected failure point:

1. Claude Desktop calls DailyOS MCP `query_entity` for an account in `src-tauri/src/mcp/main.rs:244`.
2. Account results call `query_account_intelligence()` in `src-tauri/src/mcp/main.rs:394`.
3. `query_account_intelligence()` invokes the `get_entity_intelligence` ability in `src-tauri/src/mcp/main.rs:406`.
4. `get_entity_intelligence` reads claim substrate through `read_claims()` in `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/producer.rs:303`.
5. Glean enrichment finalization currently branches by trigger in `src-tauri/src/intel_queue.rs:2745`.
6. Queue-worker Glean uses `emit_queue_worker_glean_signals()` in `src-tauri/src/intel_queue.rs:2891`, which delegates to `glean_provider::emit_glean_signals()` in `src-tauri/src/intelligence/glean_provider.rs:1106`.
7. That queue path emits Glean signals, promotes selected account facts, and recomputes health in `src-tauri/src/intelligence/glean_provider.rs:1127`, `src-tauri/src/intelligence/glean_provider.rs:1418`, and `src-tauri/src/intelligence/glean_provider.rs:1440`.
8. Manual Glean refresh instead runs `promote_manual_refresh_glean_account_facts()` in `src-tauri/src/intel_queue.rs:2835`, which promotes selected account facts and recomputes health but skips the broader Glean signal producer.

Suspected failure point: `src-tauri/src/intel_queue.rs:2745` splits one Glean producer into two trigger-specific finalization paths. That makes runtime substrate depend on how enrichment was triggered instead of what evidence Glean returned.

Rejected hypotheses:

- MCP was not reading the abilities runtime. Rejected: account lookup invokes `get_entity_intelligence`.
- MCP output shape alone was the root issue. Rejected: shaping thin substrate into a briefing only hides missing producer evidence.
- Generated JSON/markdown should be read back into MCP. Rejected: generated artifacts are write-only projections, not source-of-truth inputs.

## Problem

DailyOS has a real producer asymmetry in the Glean enrichment path.

Queue-worker Glean enrichment currently runs `emit_queue_worker_glean_signals()`, which emits Glean-derived signals and, through the current implementation, also promotes selected account facts and recomputes account health. Manual Glean refresh currently runs a separate manual-only account fact promotion helper, but skips the broader Glean signal producer.

That means the same parsed Glean `IntelligenceJson` can produce different substrate depending on trigger. The runtime may see different signals, source refs, account facts, recompute jobs, and health side effects depending on whether the user clicked refresh or the queue performed enrichment.

The fix is not to shape MCP output. The fix is a shared services-owned Glean finalization producer invoked by both queue and manual refresh.

This is a runtime integrity issue, not just DRY cleanup. If Glean finalization is split across two services/helpers, the same source evidence can:

- enter the signal bus in one path but not another
- affect weighted Bayesian signal fusion in one path but not another
- enqueue trust recompute in one path but not another
- refresh account health in one path but not another
- bypass claim lifecycle gates that protect tombstones, dismissals, user corrections, and overrides
- double-count evidence if both old and new paths run for the same finalization

The target state is one service-owned Glean producer. Queue refresh and manual refresh are trigger modes, not separate producers.

## K-in

Relevant prior guidance reviewed before authoring:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`
- `.docs/decisions/0096-glean-mode-local-footprint.md`
- `.docs/decisions/0100-glean-first-intelligence-architecture.md`
- `.docs/decisions/0101-service-boundary-enforcement.md`
- `.docs/decisions/0102-abilities-as-runtime-contract.md`
- `.docs/decisions/0105-provenance-as-first-class-output.md`
- `.docs/decisions/0107-source-taxonomy-alignment.md`
- `.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md`
- `.docs/decisions/0115-signal-granularity-audit.md`
- `.docs/decisions/0120-observability-contract.md`
- `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md`

Key constraints:

- MCP, WordPress, and Tauri consume the same abilities substrate.
- Mutations go through `services/`.
- Glean is an input to DailyOS intelligence, not a parallel authority.
- Prompt and surface channels should consume claim/runtime state, not generated JSON/markdown exports.
- Producer writes that affect intelligence need provenance, signal/claim semantics, and trust behavior.

## Current Evidence

Relevant code paths:

- `src-tauri/src/intel_queue.rs::run_enrichment_finalize_post_commit()`
- `src-tauri/src/intel_queue.rs::emit_queue_worker_glean_signals()`
- `src-tauri/src/intel_queue.rs::promote_manual_refresh_glean_account_facts()`
- `src-tauri/src/intelligence/glean_provider.rs::emit_glean_signals()`
- `src-tauri/src/services/account_fact_claims.rs::promote_glean_facts_from_intelligence()`
- `src-tauri/src/signals/bus.rs`
- `src-tauri/src/services/signals.rs`
- `src-tauri/src/signals/fusion.rs`
- `src-tauri/src/signals/policy_registry.rs`
- `src-tauri/src/db/data_lifecycle.rs`
- `src-tauri/src/services/claims_backfill.rs`
- `src-tauri/src/services/intelligence.rs` tests around `finalize_post_commit_*`

Existing tests intentionally encode the old asymmetry:

- `finalize_post_commit_manual_refresh_skips_queue_only_effects`
- `manual_refresh_promotes_account_facts_only_for_glean_producer`
- `finalize_post_commit_queue_worker_runs_full_chain`

Those tests should be changed to distinguish:

- shared Glean side effects that both queue and manual refresh must run
- queue-only lifecycle side effects that should remain queue-only

Runtime behaviors affected by the asymmetry:

- Signal bus: queue Glean can emit propagation signals while manual Glean currently does not.
- Bayesian scoring: missing or duplicated signals change weighted log-odds fusion inputs, so trust can diverge based on trigger rather than evidence.
- Trust recompute: selected account facts enqueue recompute work, but broader Glean signal evidence and health recompute are not consistently produced.
- Tombstones and user feedback: existing tombstone/user-correction paths protect dismissed or corrected intelligence only when producer writes go through the same claim lifecycle and policy surfaces. A separate finalization path can accidentally reintroduce stale or dismissed content.
- Overrides: source-attributed corrections and feedback should remain stronger than refreshed provider evidence unless the lifecycle policy explicitly supersedes them.

## Design

Add a shared Glean finalization service.

Proposed module:

- `src-tauri/src/services/glean_finalization.rs`

Proposed API shape:

```rust
pub struct GleanFinalizationInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub intel: &'a IntelligenceJson,
    pub preset: Option<&'a RolePreset>,
}

pub struct GleanFinalizationReport {
    pub run_key: String,
    pub signals_attempted: usize,
    pub signals_emitted: usize,
    pub schema_promoted: usize,
    pub claims_committed: usize,
    pub recompute_jobs_enqueued: usize,
    pub health_recomputed: bool,
    pub degraded_classes: Vec<GleanFinalizationSideEffect>,
    pub warnings: Vec<GleanFinalizationWarning>,
}

pub struct GleanFinalizationWarning {
    pub code: &'static str,
    pub side_effect: GleanFinalizationSideEffect,
    pub signal_type: Option<&'static str>,
    pub field: Option<&'static str>,
    pub source: Option<&'static str>,
    pub entity_type: String,
    pub entity_id: String,
    pub count: Option<usize>,
    pub pii_safe_detail: Option<&'static str>,
}

pub fn finalize_glean_enrichment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: GleanFinalizationInput<'_>,
) -> Result<GleanFinalizationReport, GleanFinalizationError>
```

The service owns the substrate side effects that are currently split across `intelligence::glean_provider` and `intel_queue`:

- emit Glean-derived signals from `org_health`, `support_health`, technical footprint, competitive context, org changes, Gong summaries, Slack context, and champion-health evidence
- promote selected sourced account facts through `services::account_fact_claims`
- enqueue trust recompute jobs through the existing account fact producer behavior
- recompute account health after Glean signals/facts when the subject is an account
- preserve existing claim lifecycle behavior for tombstones, dismissals, user corrections, and overrides by delegating claim writes to service-owned claim commit/promotion APIs
- preserve source lifecycle semantics for Glean and downstream sources (`glean_crm`, `glean_zendesk`, `glean_gong`, `glean_chat`, `glean_slack`, and any promoted source refs)
- avoid duplicate signal/fact/recompute emission across queue/manual retries for the same Glean output
- return a structured report so partial failures are visible in logs/tests without logging raw Glean payloads, claim text, source excerpts, names, email addresses, or prompt/response content

`run_enrichment_finalize_post_commit()` then calls the shared service whenever `producer.is_glean()` for both `QueueWorker` and `ManualRefresh`.

Write-only export behavior remains projection-only. The authoritative `entity_intelligence` DB upsert still happens before `run_enrichment_finalize_post_commit()`. Inside post-commit finalization, generated JSON/markdown exports must be written consistently for queue and manual refresh and must not become runtime inputs. If this PR changes export ordering, shared Glean finalization should run before the disk fence so exported artifacts do not claim success for side effects that failed or were degraded.

Queue-only behavior remains queue-only:

- supplemental leading-signals / peer benchmark spawn
- `intelligence-updated` Tauri event
- self-healing scheduler hook
- `claude_code` sync-success recording

Manual refresh should still skip those queue-only effects, but should no longer skip Glean substrate side effects.

## Trigger Truth

Use `FinalizeMode::{QueueWorker, ManualRefresh}.producer` as the source of truth for whether the result is Glean-derived.

Do not gate shared finalization on the current context provider mode. The context mode can race after the enrichment result is produced. If `producer == Glean`, the parsed output was Glean-derived and should run Glean finalization even if settings changed afterward.

## Idempotency and Evidence Identity

One Glean output should produce one shared finalization result, regardless of whether it is reached by queue worker, manual refresh, retry, or a queue/manual overlap.

Implementation requirements:

- Compute a stable `run_key` from entity type, entity id, producer, and a canonicalized hash of the finalized `IntelligenceJson` payload used by the service.
- Add a production signal facade/bus mechanism for idempotent Glean finalization emissions, such as `emit_once()` and `emit_once_and_propagate()`, that accepts a deterministic side-effect id or idempotency key and uses insert-or-replace/insert-or-ignore semantics intentionally. Do not use fixture-only signal insertion helpers in production code.
- Derive deterministic side-effect identities from `run_key + side_effect + signal_type/field/source`. Do not rely on fresh UUID-only signal ids for Glean finalization evidence.
- Signal tests must compare evidence identity, not just counts: signal type, payload hash, data source, confidence, source context/source ref where present, and recompute subject.
- Account fact promotion remains deduped through the existing claim commit path, but tests must prove queue and manual produce the same account-fact claim identity/source refs for the same fixture.
- Re-running finalization with the same `run_key` must not double-promote account facts, double-enqueue trust recomputes, or double-count Bayesian evidence.
- If a prior attempt partially failed, retry may complete missing side-effect classes, but it must not duplicate classes that already succeeded for the same `run_key`.

This slice does not change Bayesian scoring math. It controls the inputs so scoring receives evidence once, with the same identity, from both triggers.

## Failure Semantics

Failures are not allowed to be free-form log-and-proceed behavior.

Finalization outcomes are classified by side-effect class:

| Class | Minimum behavior |
| --- | --- |
| Lifecycle gate unavailable | hard error; do not commit claim/fact writes through a bypass |
| Account fact/source-ref write failed | degraded report plus typed warning; do not report fact parity as successful |
| Required signal emission/propagation failed | degraded report plus typed warning; do not report signal parity as successful |
| Trust recompute enqueue failed | degraded report plus typed warning; do not report recompute parity as successful |
| Health recompute failed | degraded report plus typed warning; account health may remain stale |
| Non-critical report/log warning | structured warning only; no raw content |

`run_enrichment_finalize_post_commit()` must propagate hard errors before recording enrichment success. Recoverable degradation must be durable enough for the broader wave to render caveats; a typed `GleanFinalizationReport` returned only to logs is insufficient. If no existing durable degraded-state surface exists, emitting a PII-safe `glean_finalization_degraded` signal or equivalent service-owned marker is acceptable for this slice.

Full user-facing caveat rendering is not implemented in this slice, but L2 should block if failures are only hidden in logs.

## Source Lifecycle Matrix

The implementation must inventory every persistent artifact written by shared Glean finalization and prove its source lifecycle behavior.

| Artifact | Source fields to verify | Lifecycle expectation |
| --- | --- | --- |
| `signal_events` | `data_source`, `signal_type`, payload hash | `purge_source(DataSource::Glean)` covers every Glean data source emitted by this service, including `glean_slack` |
| `account_source_refs` | source system, source kind, source record ref, observed/source-as-of values | retained or purged/masked according to the downstream source authority recorded on the row |
| `intelligence_claims` account facts | `source_ref`, `source_asof`, claim lifecycle state | committed only through lifecycle-aware claim services; tombstone/correction gates apply |
| `account_technical_footprint` | source/source kind fields | purged/masked or explicitly retained according to the recorded Glean/downstream source |
| recompute jobs | job kind, subject type/id, origin signal/claim where present | deduped for one `run_key`; no duplicate trust recompute from queue/manual overlap |
| generated entity-intelligence JSON/markdown exports touched by this path | file write only | never read by MCP/Tauri runtime paths as authority; unrelated legacy JSON readers remain broader remediation work |

If a table cannot currently express the needed source lifecycle authority, the implementation must either add the minimum source marker required for this service or remove that write from this slice and file a maintenance issue. It cannot silently write lifecycle-opaque Glean-derived state.

## Non-goals

This slice does not:

- change MCP projection shape
- implement entity neighborhood or participation summaries
- fix all `source_asof` / `source_ref` gaps in projection claims
- rewrite generated JSON runtime reads
- change trust scoring math
- backfill production rows
- make report producers claim writers
- implement full user-facing caveat rendering for all runtime surfaces

Those remain in the broader producer remediation plan.

## Implementation Units

### U1 — Add services-owned Glean finalization module

Files:

- `src-tauri/src/services/glean_finalization.rs`
- `src-tauri/src/services/mod.rs` or equivalent module registration
- `src-tauri/src/intelligence/glean_provider.rs`
- `src-tauri/src/signals/bus.rs`
- `src-tauri/src/services/signals.rs`
- `src-tauri/src/db/data_lifecycle.rs`

Work:

- Move the mutating Glean signal/fact/health logic used by `emit_glean_signals()` behind `services::glean_finalization`.
- Add a production signal idempotency helper/facade for deterministic finalization signals. It must support propagation, not just raw row insertion.
- Keep provider code in the finalization path focused on retrieval/parsing. If a compatibility wrapper remains in `glean_provider.rs`, it must delegate to the service and contain no direct DB writes.
- Inventory other `glean_provider.rs` DB mutations found during L0. Mutations outside the finalization path may be excluded from this slice only if they are named in L2 proof notes and tracked separately; they must not be used as an alternate finalization path.
- Preserve current best-effort behavior only for non-critical signal failures, and record structured PII-safe warnings plus degraded side-effect classes in the report.
- Route claim writes through existing lifecycle-aware services rather than direct insertion so tombstones, dismissals, corrections, and overrides keep their authority.
- Ensure one Glean output cannot run both the old queue signal path and the old manual account-fact path.
- Add source-lifecycle coverage for Glean finalization artifacts, especially `glean_slack` signal events and source-bearing account fact / technical footprint writes. `purge_source(DataSource::Glean)` must include every Glean data source emitted by the shared finalizer.

Verification:

- No new DB mutation is introduced outside `services/`.
- Existing signal/fact promotion behavior is preserved for queue-worker Glean output.
- No duplicate signal/fact/recompute side effects are produced by queue/manual/retry finalization of the same Glean output.
- Warnings and logs are typed and PII-safe per ADR-0120.

### U2 — Wire shared finalization into queue and manual refresh

Files:

- `src-tauri/src/intel_queue.rs`

Work:

- Replace `emit_queue_worker_glean_signals()` and `promote_manual_refresh_glean_account_facts()` with a shared call for any `producer.is_glean()`.
- Keep supplemental queue-only Glean finalize in the queue branch.
- Keep PTY fallback behavior unchanged: `producer == Pty` must not emit Glean signals or promote Glean account facts.
- Update comments that still describe `intelligence.json` as the reason meeting prep consumes enrichment output.

Verification:

- Queue and manual Glean finalization produce equivalent shared side effects.
- Manual refresh still skips queue-only sync/event/scheduler side effects.
- PTY fallback still skips Glean-only side effects.

### U3 — Update tests to encode the new contract

Files:

- `src-tauri/src/services/intelligence.rs` existing finalize tests
- Add helper tests wherever the service module test convention fits best

Required tests:

- `glean_queue_and_manual_refresh_share_finalization_side_effects`
- `manual_refresh_glean_emits_signals_and_promotes_facts`
- `pty_manual_refresh_skips_glean_finalization`
- `queue_only_finalize_effects_remain_queue_only`
- `glean_finalization_does_not_double_count_shared_side_effects`
- `glean_finalization_same_run_key_is_idempotent_across_queue_and_manual`
- `glean_finalization_uses_production_signal_idempotency`
- `glean_finalization_preserves_tombstones_feedback_and_overrides`
- `glean_finalization_source_lifecycle_covers_written_artifacts`
- `glean_finalization_reports_pii_safe_degraded_side_effects`

Assertions:

- both queue and manual Glean finalization emit `renewal_data_updated`
- both queue and manual Glean finalization feed the signal bus / propagation engine for the same fixture evidence
- idempotent signal emission uses the production signal facade/bus helper, not fixture-only insertion
- both write/update technical footprint when technical-footprint evidence exists
- both promote account fact claims/source refs when account facts exist
- both enqueue account trust recompute when facts are promoted
- signal/fact/recompute assertions compare evidence identity, not only counts
- repeated shared-finalization wiring does not double-promote facts, double-enqueue recompute, or double-count signal evidence for the same `run_key`
- manual does not record `claude_code` sync success
- manual does not run queue-only self-healing scheduler hook
- PTY fallback commits no Glean facts/signals
- tombstoned, dismissed, user-corrected, and overridden intelligence is not resurrected by queue or manual Glean finalization
- non-claim side effects that affect health/signals cannot reintroduce dismissed content without passing the same lifecycle policy
- source lifecycle tests prove Glean finalization artifacts are purged/masked or explicitly retained with valid downstream-source authority
- `purge_source(DataSource::Glean)` removes or masks `glean_slack` signal events and any other Glean data source emitted by the finalizer
- warnings/logs contain typed codes and metadata only, not raw Glean content or customer/user text

### U4 — L2 proof notes

Files:

- PR description or proof bundle if requested

Work:

- Document changed side-effect matrix:

| Effect | Queue Glean | Manual Glean | PTY fallback |
| --- | --- | --- | --- |
| Projection claims | yes | yes | yes |
| Glean signals | yes | yes | no |
| Account fact promotion | yes | yes | no |
| Trust recompute enqueue | yes, when facts promoted | yes, when facts promoted | no |
| Health recompute | yes for accounts | yes for accounts | no Glean health recompute |
| Write-only exports | yes, same order | yes, same order | existing PTY behavior |
| Durable degraded marker/caveat input | yes, on degraded finalize | yes, on degraded finalize | no Glean marker |
| Supplemental Glean pass | yes, queue only | no | no |
| `intelligence-updated` event | yes, queue only | no | no |
| `claude_code` sync success | yes, queue only | no | no |

Verification:

- Include targeted test output and note full L2 status if run.

## Acceptance Criteria

This implementation is acceptable when:

1. Queue and manual Glean enrichment invoke the same services-owned Glean finalization path.
2. Manual Glean refresh no longer skips Glean-derived signals.
3. Queue and manual Glean refresh both promote the same selected account facts for the same fixture output.
4. Queue-only lifecycle side effects remain queue-only.
5. PTY fallback remains clean: no Glean signals, no Glean account fact claims, no Glean source refs.
6. Side-effect failures are reported as structured, PII-safe, durable degraded-state metadata; they are not silently hidden or logged only as free-form strings.
7. No generated JSON/markdown file becomes a source of truth in this change.
8. Tests encode both parity and non-regression boundaries.
9. Queue and manual Glean refresh feed equivalent signal bus / Bayesian-fusion inputs for the same fixture evidence.
10. The implementation does not bypass lifecycle protections for tombstones, dismissals, user corrections, or overrides.
11. The implementation does not duplicate evidence by running two Glean producers for one finalization.
12. Queue/manual/retry finalization of the same Glean output is idempotent by evidence identity, not just by count.
13. Source lifecycle behavior is explicit for every persistent Glean-derived artifact written by this service.
14. Degraded side-effect classes are reported through typed, PII-safe metadata and are durable enough for downstream surfaces to render caveats in the broader wave.
15. Generated entity-intelligence JSON/markdown exports touched by this path remain write-only and are written consistently across queue/manual triggers.

## L2 Review Checklist

L2 should block if any of these are true:

- Glean DB mutations remain implemented primarily in provider code instead of `services/`.
- Manual and queue paths still have separate Glean fact/signal producers.
- The implementation double-promotes account facts by calling both the shared signal path and the old manual helper.
- The implementation can double-count Glean evidence across queue/manual/retry finalization of the same output.
- The implementation uses fixture-only signal insertion helpers for production idempotency.
- `producer.is_glean()` is ignored in favor of current context mode only.
- PTY fallback emits Glean signals or writes Glean account facts.
- Queue-only effects leak into manual refresh.
- Queue and manual Glean produce different signal bus / Bayesian-fusion inputs for the same fixture evidence.
- The patch bypasses claim lifecycle or policy gates that protect tombstones, dismissals, user corrections, or overrides.
- The patch can double-count one Glean output by emitting duplicate signals, duplicate account facts, or duplicate trust recompute jobs.
- The patch records free-form warning strings or raw source content in logs/reports.
- The patch writes lifecycle-opaque Glean-derived state to `signal_events`, `account_source_refs`, `intelligence_claims`, `account_technical_footprint`, or generated exports.
- `purge_source(DataSource::Glean)` omits a Glean data source emitted by shared finalization, including `glean_slack`.
- The patch only proves parity by row counts instead of evidence identity.
- Tests assert only counts without proving the side-effect class.
- The patch adds customer-specific fixtures or source labels.

## L0 Decisions

1. Partial failures are split into hard errors and degraded side-effect classes. Hard errors propagate and prevent enrichment success recording. Recoverable failures produce typed warnings, a degraded report, and a durable caveat input/marker.
2. Account health recompute stays inside Glean finalization for accounts in this slice. Generic health recompute belongs to W4.
3. Remove `glean_provider::emit_glean_signals()` if call sites are fully migrated. A temporary wrapper is acceptable only if it delegates to `services::glean_finalization` and contains no direct DB writes.
4. Tombstone, dismissal, user-correction, and override preservation fixtures are mandatory for this slice.
5. Source lifecycle coverage is mandatory for persistent artifacts written by shared Glean finalization.
6. Export parity is in scope only as trigger/order consistency and write-only discipline. Changing MCP/Tauri to read generated exports remains out of scope.

## Rollback

Rollback is straightforward: revert the shared finalization commit. No migration is expected. Existing rows written by the new shared finalization are normal signal/account-fact/claim rows and do not require cleanup.

## L0 Review Verdict

Status: approved after revision.

Reviewer panel:

- Feasibility reviewer: approve.
- Coherence reviewer: approve after export scope, L0 decisions, failure semantics, and technical-footprint assertions were tightened.
- Security/trust reviewer: approve after source lifecycle, tombstone/user-feedback fixtures, and PII-safe reporting were made mandatory.
- Adversarial reviewer: approve after run-key idempotency, durable degraded-state reporting, lifecycle fixtures, provider mutation inventory, and evidence-identity tests were added.

Key L0 changes made before approval:

- Added formal debug-driven origination and symptom-to-failure trace.
- Added production signal idempotency requirements (`emit_once` / `emit_once_and_propagate` or equivalent).
- Added source lifecycle coverage, including `purge_source(DataSource::Glean)` coverage for every Glean data source emitted by shared finalization.
- Replaced free-form warning strings with structured PII-safe degraded-state reporting.
- Made tombstone, dismissal, user-correction, and override preservation fixtures mandatory.
- Required evidence-identity assertions instead of count-only parity tests.
- Scoped export parity to trigger/order consistency and write-only discipline for entity-intelligence exports touched by this path.
