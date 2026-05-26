# L0 Packet - v1.4.6 W1-A - DOS-329 RecommendationClaim Contract

**Current revision:** V0.4 (cycle-3 L0 fixes, 2026-05-26)

## 1. Header

- **Date:** 2026-05-26
- **Project:** v1.4.6 - Salience & Recommendations
- **Wave:** W1 stage 1a (gates W1-B and every downstream recommendation lane)
- **Issue:** [DOS-329 - Define RecommendationClaim contract and lifecycle](https://linear.app/a8c/issue/DOS-329)
- **Branch:** `codex/v1.4.6-w1-a-dos-329`
- **Worktree:** `/Users/jamesgiroux/Documents/dailyos-repo/.worktrees/codex/v1.4.6-w1-a-dos-329`
- **Base:** `public/dev` @ `064df158` (PR #399 merge)
- **Migration slots claimed:** **v269** for W1-A. `.docs/plans/v1.4.6-waves.md` has been amended to v269-v288 because current `dev` already registers migrations through v268.
- **Authority docs:** `.docs/plans/v1.4.6-waves.md`; ADR-0123; ADR-0125 §5 RecommendationClaim amendment; ADR-0126; ADR-0130; ADR-0131.
- **Required L0 gates:** `/plan-devex-review` + codex challenge + K-in learnings check.

## 2. Live-Dev Reconciliation

W0 is materially complete on `dev`, and `.docs/plans/v1.4.6-waves.md` now records the live state:

| W0 gate | Live `dev` status |
|---|---|
| v1.4.4 `claim_receipt` + `claim_review_queue` substrate | Satisfied by PR #323 / merge `c1703966`; paths exist and `ClaimReceipt` resolves. |
| `SignalType::SurfacingDecisionMade` + `SignalType::SalienceCandidateRefreshTriggered` predeclared by W0 | Satisfied in `src-tauri/src/signals/policy_registry.rs`; W1-A does not touch the signal registry. |
| ADR-0125 RecommendationClaim amendment | Satisfied in `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md` §5 and mirrored in `ClaimType::Recommendation`. |
| ADR-0102 amendment | Not needed for W1-A. ADR-0125 §5 records that existing `McpExposure::MetadataOnly` covers v1.4.6. |

Migration coordination was the live-dev blocker. The original v1.4.6 plan reserved v260-v279 and named v260-v261 for W1-A; those slots are no longer executable. Current `dev` registers:

- v260 `email_summary_context_evidence`
- v261 `drop_mcp_transport_nonce_ledger`
- v262 `workspace_placement_idempotency`
- v263 `claim_subject_lookup_index`
- v264 `effective_meeting_entities_view`
- v265 `runtime_evidence_backfill_request`
- v266 `runtime_entity_action_backfill_request`
- v267 `v178` repair
- v268 `workspace_backfill_state`

Because the migration runner applies only migrations with `version > current`, W1-A must not add any executable migration below v269. This packet claims v269 and the wave plan now reserves **v269-v288**.

## 3. Goal

Ship the route-neutral `RecommendationClaim` contract every v1.4.6 lane consumes:

- `services::recommendations::contracts` owns the Rust DTOs and every cross-lane enum.
- `src/services/recommendations/contracts.ts` mirrors the wire contract.
- Recommendation writes remain normal claims: `claim_type = "recommendation"` through `services::claims::commit_claim`.
- Recommendation-specific fields live in the claim row's `metadata_json` under a typed `recommendation` envelope. W1-A adds a partial index migration at v269 so downstream surfacing and feedback lanes can query action/state paths without a parallel lifecycle table.

W1-A does not build the salience algorithm, surfacing policy, trigger policy, render surface, feedback loop, engagement telemetry, or eval harness. It freezes the shared API those lanes import.

## 4. Files Owned

### W1-A fills

- `src-tauri/src/services/recommendations/contracts.rs`
- `src-tauri/src/services/recommendations/recommendation.rs`
- `src/services/recommendations/contracts.ts`
- `src/services/recommendations/__tests__/contracts.golden.test.ts`
- `src-tauri/src/migrations/269_recommendation_claim_metadata_indexes.sql` (registered as version `269`; current repo SQL migrations use filename == registered version)
- `src-tauri/src/migrations.rs` registration + migration test

### W1-A preserves

The `services/recommendations` module skeleton already exists on `dev`, including placeholders for `salience.rs`, `surfacing.rs`, `triggers.rs`, `feedback.rs`, `deviation.rs`, `engagement.rs`, `render.rs`, `eval.rs`, and `why_this_now.rs`. `mod.rs` already declares `pub mod why_this_now;`; W1-A preserves the placeholder and does not rewrite unrelated placeholder contents.

### W1-A must not touch

- `services::claims::commit_claim` behavior.
- `signals/policy_registry.rs`.
- `services::claim_receipt::*` or `services::claim_review_queue::*`.
- W1-B+ placeholder file bodies except where imports require compile-only references.
- MCP bridge code.

## 5. Contract Shape

`contracts.rs` implements the frozen wave-plan block with current live imports:

- `SubjectRef` from `abilities_runtime::abilities::provenance::subject::SubjectRef`.
- `Provenance` from `abilities_runtime::abilities::provenance::envelope::Provenance`.
- `TrustBand` from `abilities_runtime::abilities::trust::types::TrustBand`.
- `ClaimState`, `SurfacingState` from `abilities_runtime::types`.
- `ClaimVerificationState`, `RenderSurface` from `abilities_runtime::sensitivity`.
- `DateTime<Utc>` from `chrono`.

New recommendation-local types:

- `ClaimId`
- `EvidenceRef`
- `RecommendationDraft`
- `RecommendationClaim`
- `RecommendedAction`
- `FeedbackState`
- `RecommendationFeedbackDecision`
- `DismissReason`
- `BoundedNote`
- `RecommendationFeedbackContext`
- `ConversionTarget`
- `ConversionState`
- `SalienceScore`
- `SalienceFactor`
- `SalienceFactorKind`
- `FactorRationale`
- `WhyThisNow`
- `SurfacingDecision`
- `SurfacingTier`
- `DeferReason`
- `SuppressReason`
- `TriggerRef`
- `TriggerKind`
- `EngagementSignal`
- `RecommendationMetadataEnvelope`
- `RecommendationMetadataPayload`

Serde contract:

- Field structs use `#[serde(rename_all = "camelCase")]`.
- Struct-variant enums use `#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]`.
- Tuple-variant enums remain externally tagged with `#[serde(rename_all = "camelCase")]`; golden fixtures must cover the exact external shapes (`"pending"`, `{ "decided": { "kind": "accept", ... } }`, `{ "other": "..." }`, `{ "claimCorrection": "claim-id" }`).
- `RecommendedAction::Custom` uses `action_kind`, not `kind`, so it cannot collide with the serde discriminator.
- `BoundedNote` has a private `String`, `TryFrom<String>`, and custom `Deserialize` enforcing a 200-character cap.
- The TypeScript mirror must encode the exact Rust JSON shape, including externally tagged tuple variants.

Pinned TypeScript shapes:

```ts
export type FeedbackState =
  | "pending"
  | { decided: RecommendationFeedbackDecision };

export type DismissReason =
  | "notRelevant"
  | "alreadyKnew"
  | "wrongSubject"
  | { other: string };

export interface RecommendationFeedbackContext {
  surface: RenderSurface;
  invocationId: string;
}

export type ConversionTarget =
  | { action: string }
  | { claimCorrection: ClaimId }
  | { reviewQueue: string };

export type ConversionState =
  | { kind: "notConverted" }
  | { kind: "convertedToAction"; actionId: string }
  | { kind: "convertedToClaimCorrection"; claimId: ClaimId }
  | { kind: "convertedToReviewQueue"; queueItemId: string };
```

## 6. Metadata Envelope

The claim substrate already has `metadata_json`, `superseded_by`, provenance, trust, lifecycle, sensitivity, and typed feedback. W1-A should not add a parallel recommendation lifecycle table.

W1-A stores the recommendation payload as:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationMetadataEnvelope {
    pub recommendation: RecommendationMetadataPayload,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationMetadataPayload {
    pub schema_version: u16,
    pub recommended_action: RecommendedAction,
    pub evidence: Vec<EvidenceRef>,
    pub salience: SalienceScore,
    pub feedback_state: FeedbackState,
    pub conversion_state: ConversionState,
}
```

The stored JSON shape is therefore:

```json
{
  "recommendation": {
    "schemaVersion": 1,
    "recommendedAction": {
      "kind": "scheduleMeeting",
      "entityId": "entity-1",
      "whenWindow": "next_week",
      "rationale": "recent support trend needs follow-up"
    },
    "evidence": [],
    "salience": { "total": 0.0, "factors": [] },
    "feedbackState": "pending",
    "conversionState": { "kind": "notConverted" }
  }
}
```

`services::recommendations::recommendation` owns helpers that:

- build this metadata envelope from a `RecommendationDraft`;
- convert a `RecommendationDraft` into a `services::claims::ClaimProposal`;
- set `claim_type` from `ClaimType::Recommendation.as_str()`;
- set `field_path = Some(format!("recommendation.{action_key}"))` and `topic_key = Some(action_key)` so ADR-0125 replace semantics dedupe by `(subject, recommended_action_kind)` through the existing `commit_claim` lock/dedup path;
- derive `action_key` as:
  - `scheduleMeeting`
  - `sendMessage`
  - `reviewClaim`
  - `updateRecord`
  - `investigateChange`
  - `custom.<action_kind>` for `RecommendedAction::Custom`, where `action_kind` is a non-empty opaque ASCII token capped at 64 characters and rejects whitespace, `/`, `\`, and control characters;
- leave `ClaimProposal.id = None` for fresh runtime writes unless the caller explicitly chooses the deterministic insert wrapper with an `InsertWithId` target;
- leave `temporal_scope` and `sensitivity` unset unless the caller has a specific override, so `commit_claim` applies ADR-0125 defaults;
- never write directly to `intelligence_claims`.

## 7. Migration v269

`269_recommendation_claim_metadata_indexes.sql` is registered as migration version `269` and adds query support only. The K-in solution note `migration-filename-version-offset-2026-05-18.md` was checked, but current `dev` SQL migrations v240+ use filename == registered version (`268_workspace_backfill_state.sql` is registered as v268), so W1-A follows the live repo convention.

- partial index on recommendation action kind:
  `json_extract(metadata_json, '$.recommendation.recommendedAction.kind')`
- partial index on normalized feedback state/decision kind:
  `COALESCE(json_extract(metadata_json, '$.recommendation.feedbackState.decided.kind'), json_extract(metadata_json, '$.recommendation.feedbackState'))`
- partial index on conversion state:
  `json_extract(metadata_json, '$.recommendation.conversionState.kind')`

Each index is scoped with:

```sql
WHERE claim_type = 'recommendation'
  AND metadata_json IS NOT NULL
  AND json_valid(metadata_json) = 1
```

This keeps `intelligence_claims` authoritative while avoiding table-level reinvention. W4-A derives current feedback state from the append-only `claim_feedback` log and may update the denormalized envelope only through a service-owned claim/recommendation API approved in W4-A L0. W1-A only defines the shape and read indexes.

## 7a. Feedback Bridge

W1-A does not extend `abilities_runtime::abilities::feedback::FeedbackAction`, which ADR-0123 owns as a closed substrate enum. The recommendation-local decision type is named `RecommendationFeedbackDecision` to avoid collision with the substrate enum.

W4-A records recommendation feedback through a recommendation service wrapper:

```rust
record_recommendation_feedback(
    claim_id: ClaimId,
    decision: RecommendationFeedbackDecision,
    context: RecommendationFeedbackContext,
) -> FeedbackState
```

The wrapper updates the recommendation metadata envelope's `feedback_state` / `conversion_state` through the approved recommendation service path, then calls `services::claims::record_claim_feedback` with **only ADR-0123-allowed metadata keys**. It does not put recommendation-specific marker keys into `claim_feedback.payload_json`, because `services::claim_receipt::feedback::validate_and_sanitize_metadata` rejects unknown keys for the public feedback surface.

Exact ADR-0123 bridge:

| Recommendation decision | ADR-0123 action | `payload_json` sent to `record_claim_feedback` |
|---|---|---|
| `Accept { .. }` | `ConfirmCurrent` | `None` |
| `Convert { into, .. }` | `ConfirmCurrent` | `None`; conversion target is stored in the recommendation metadata envelope's `conversion_state`, not in ADR-0123 feedback metadata. |
| `Dismiss { reason: WrongSubject, .. }` | `WrongSubject` | `None`; adding a corrected-subject field requires a W4-A L0 contract amendment because the W1-A decision shape does not carry `corrected_to`. |
| `TooNoisy { .. }` | `SurfaceInappropriate` | `{ "surface": ClaimDismissalSurface::from(context.surface).as_str() }`; `RenderSurface` is converted through the existing `From<RenderSurface> for ClaimDismissalSurface` implementation before writing the payload. |
| `NotUseful { .. }` | `NotRelevantHere` | `{ "invocation_id": context.invocation_id }` |
| `Dismiss { reason: NotRelevant \| AlreadyKnew \| Other(_), .. }` | `NotRelevantHere` | `{ "invocation_id": context.invocation_id }`; the dismissal reason stays in the recommendation metadata envelope, not ADR-0123 feedback metadata. |

If W4-A finds this bridge creates incorrect trust effects for recommendation claims, W4-A must amend ADR-0123 before implementation rather than adding an ad hoc feedback table or bypassing `record_claim_feedback`.

## 8. Intelligence Loop Check

1. **Claim model:** `RecommendationClaim` is a first-class `ClaimType::Recommendation`; `commit_claim` remains the writer.
2. **Provenance + trust:** The base claim row carries provenance, source attribution, `source_asof`, trust band, sensitivity, and lifecycle. Salience is stored separately in the recommendation metadata envelope and is not trust.
3. **Signals + invalidation:** W0 predeclared `SalienceCandidateRefreshTriggered` and `SurfacingDecisionMade`. W1-A does not emit them.
4. **Runtime + surfaces:** W1-B, W2, W3, W4, W5, and v1.4.7 W2-C import `services::recommendations::contracts`; W3 render flows through `ClaimReceipt`.
5. **Feedback loop:** `FeedbackState` and `RecommendationFeedbackDecision` are recommendation-local contract types only in W1-A. W4-A wires them to ADR-0123 `record_claim_feedback` and ranking updates through the bridge above.

## 9. K-In Notes

Relevant prior knowledge:

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`: new real claim producers require runtime-wide trust/provenance audit, not a narrow first-claim patch.
- `docs/solutions/workflow-issues/parallel-wave-synced-from-conflicts-2026-05-19.md`: parallel work needs explicit shared-coordinate reservations. The migration slot drift here is the same class of issue.
- `docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md`: checked during K-in. Current live migrations v240+ follow filename == registered version, so W1-A uses `269_...sql` registered as v269 and documents the convention choice.
- ADR-0123: substrate `FeedbackAction` stays closed; recommendation decisions bridge through existing `record_claim_feedback` actions with only ADR-0123-allowed metadata keys unless W4-A amends ADR-0123.
- ADR-0126: no new claim mutation APIs bypassing `services/claims.rs`.
- ADR-0130: salience consumes substrate retrieval primitives; W1-A must not reinvent retrieval.
- ADR-0131: claim-type-specific salience and canonicalization inputs belong in typed registries/contracts, not loose strings.

Regression prevention:

- Add a test that `ClaimType::Recommendation.as_str()` returns `"recommendation"` and the registry metadata matches ADR-0125 defaults.
- Add a migration test proving v269 registers above current max and the indexes exist.
- Add Rust JSON golden coverage for every cross-lane type.
- Add TS golden coverage matching the Rust fixture, including externally tagged tuple variants and `conversionState.kind`.

## 10. Tests Required

- `cargo test --manifest-path src-tauri/Cargo.toml recommendations::`
- `cargo test --manifest-path src-tauri/Cargo.toml claim_type_registry`
- `cargo test --manifest-path src-tauri/Cargo.toml migration_269`
- `pnpm tsc --noEmit`
- `pnpm test -- src/services/recommendations/__tests__/contracts.golden.test.ts`
- Before PR: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, full `cargo test --manifest-path src-tauri/Cargo.toml`, and full `pnpm tsc --noEmit`.

## 11. L0 Questions

| Question | Proposed resolution |
|---|---|
| Does W1-A need a separate recommendation table? | No. Store the typed payload in `intelligence_claims.metadata_json` and add partial JSON-path indexes. Claim row remains authoritative. |
| What migration slot should W1-A use? | v269, the next executable slot after current `dev` max v268. Amend v1.4.6 reservation to v269-v288. |
| Does W1-A change `commit_claim`? | No. It builds `ClaimProposal` input and lets `commit_claim` apply registry defaults, locking, supersession, provenance, trust, and version events. |
| Does W1-A emit recommendation signals? | No. W0 already declared variants; W2 emits. |
| Does W1-A expose MCP behavior? | No. v1.4.7 consumes the contract later; v1.4.6 keeps MCP exposure metadata-only. |
| How does TS mirror substrate types? | Import `SubjectRef`, `TrustBand`, `ClaimState`, `SurfacingState`, `ClaimVerificationState`, and `RenderSurface` from `src/services/claim-receipt/contracts.ts`. Define local `JsonValue` in `src/services/recommendations/contracts.ts` and type `RecommendationClaim.provenance` as `JsonValue` until a shared generated TS provenance contract lands. The metadata envelope itself does not carry full provenance. |

## 12. Done When

- W0 gate checklist is reconciled in `.docs/plans/v1.4.6-waves.md`.
- The migration slot amendment is committed before any executable W1-A migration lands.
- Rust contracts compile and serialize the frozen wire shape.
- TypeScript mirror exists and `tsc` accepts it.
- `RecommendationDraft` to `ClaimProposal` conversion uses `ClaimType::Recommendation.as_str()`, leaves fresh-write `id` unset, and routes through `commit_claim`.
- v269 migration and tests are green.
- Local L0 records `/plan-devex-review` approval before implementation begins.
