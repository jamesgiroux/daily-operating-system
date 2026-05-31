# DOS-303 — Evidence-age trust + feedback semantics + tombstone gates contract

**Status:** verified satisfied at v1.4.0 wave tip (`658dbd07`).
**Acceptance walk last refreshed:** 2026-05-07.

## Contract

Trust scoring consumes evidence age (not row creation age). Source/extractor/linker reliability are separable scoring inputs. Source lifecycle hard-excludes revoked sources. `ScoringClass` prevents reference/context/chat sources from scoring like authoritative systems. Feedback is a **typed escalation mechanism** with deterministic state transitions, not free-form notes. Tombstones are PRE-GATE rules with explicit override semantics, not normal scored claims.

## Acceptance criteria — verification

### DOS-299 marked as blocker for DOS-5 / DOS-10 implementation

DOS-299 (`source_asof` population semantics + freshness fallbacks) shipped at v1.4.0; DOS-5 (Trust Compiler) consumes `source_asof` directly via `freshness_weight`. DOS-299 is closed (Done state) and was a hard prerequisite captured in the W3-B substrate plan.

### DOS-5 formula consumes evidence age, not claim row creation age

`src-tauri/src/abilities/trust/factors.rs::freshness_weight` reads `IntelligenceClaim.source_asof` (Option<DateTime<Utc>>), not `created_at`. When `source_asof` is unknown, the factor falls through to per-DataSource half-lives (DOS-10) and emits `ConfidenceCaveat::UnknownTimestamp`.

Test fixtures cover:
- `source_asof = Some(...)` → claim ages from source timestamp
- `source_asof = None` → fallback per-DataSource half-life + `UnknownTimestamp` caveat
- `source_asof` in the future → clamped to now, caveat emitted

### SourceTimestampUnknown changes scoring AND render policy explicitly

Trust scoring: `ConfidenceCaveat::UnknownTimestamp` emitted when `source_asof` is None (`src-tauri/src/abilities/trust/types.rs::ConfidenceCaveat:65`). Caveats appear in `ConfidenceEvidence` and downgrade band visibility per the trust-band rendering rule (DOS-320).

Render policy: claims with `UnknownTimestamp` caveat get downweighted via `freshness_weight` clamp + the band threshold check. They render in the `use_with_caution` band by default (Background section, collapsed) rather than `likely_current` (main body).

Provenance side: `ProvenanceWarning::SourceTimestampUnknown` is set on field attribution when the source timestamp could not be lifted from the LLM output / Glean document. Downstream `prepare_meeting` reads this and renders the field as background context, not load-bearing current state.

### Source lifecycle hard-excludes revoked / restricted sources

Trust gates: `TrustGateKind::SourceWithdrawn` (`src-tauri/src/abilities/trust/types.rs:82`) caps the score below `use_with_caution_min` regardless of how strong the other factors are. The recompute function (`abilities/trust/mod.rs:108-113`) forces `TrustBand::NeedsVerification` when the gate triggers.

`SourceLifecycle` enumeration (DOS-298 substrate work) covers `Active`, `Withdrawn`, `Restricted`. Withdrawn/Restricted gate at `commit_claim` time and again at recompute. Verified by the bundle-1 trust reproduction at `dos287_substrate_bundle1_reproduction.rs:247-322` and the retired-contamination-module regression at `dos287_retired_contamination_module_test.rs`.

### ScoringClass prevents reference/context/chat sources from scoring like authoritative systems

`ScoringClass` enum on `DataSource` distinguishes `Authoritative` (Salesforce, Zendesk records) from `Reference` (Glean snippets, transcripts) from `Chat` (Slack messages). Reference/Chat sources can corroborate but cannot anchor a `likely_current` band on their own — scoring caps in `factors.rs::source_reliability` ensure this.

### Feedback semantics matrix

| Feedback action | Claim state | Trust factors | Source reliability | Extractor reliability | Linker reliability | Render policy | Repair job | Pre-gate state |
|---|---|---|---|---|---|---|---|---|
| `confirmed` | active+surfaced | `user_feedback_weight` ↑ (boost capped at ceiling) | unchanged | unchanged | unchanged | shown | none | unchanged |
| `dismissed` | dormant | `user_feedback_weight` ↓ | unchanged | unchanged | unchanged | hidden behind "Show all evidence" | none | dismissal counted at next supersession |
| `mark_false` | dormant + falsity flag | `user_feedback_weight` ↓↓ | unchanged | unchanged | unchanged | hidden | reconcile at next refresh | tombstone-class pre-gate |
| `mark_outdated` | dormant + temporal_scope downgrade | `freshness_weight` floored | unchanged | unchanged | unchanged | hidden behind "Show all evidence" | next supersession event closes | tombstone-class pre-gate |
| `wrong_subject` | withdrawn (different subject) | unchanged on this claim | unchanged | unchanged | linker_reliability ↓ | hidden | linker re-runs with `subject_evidence_floor` raised | tombstone-class pre-gate (do-not-re-link) |
| `wrong_source` | annotated, claim text intact | unchanged | unchanged | extractor_reliability ↓ | unchanged | shown with "source corrected" | re-attribute via supersession | none |
| `surface_inappropriate` | active, render-suppressed for surface | unchanged | unchanged | unchanged | unchanged | suppressed only on offending surface | none | surface-scoped block |
| `corrected` | superseded by new claim | new claim takes user_feedback boost | unchanged | unchanged | unchanged | new claim shown, old hidden | linker honors correction across reconcile | tombstone-class pre-gate on the old claim |

Implementation references: typed feedback enum in `src-tauri/src/abilities/feedback/` (DOS-294); `services/claims.rs::commit_feedback` writes the feedback row + emits the appropriate signal (`CorroborationStrengthened`, `ClaimSuperseded`, etc.). Pre-gate enforcement at `commit_claim`: tombstone-class feedback hits cause `ClaimResurrectionBlocked` errors unless override conditions below are met.

### Tombstone override rules are PRE-GATE rules, not normal trust scoring

A user dismissal / mark-false / mark-outdated / wrong-subject / corrected event creates a tombstone-class precedence record. `commit_claim` runs the pre-gate before scoring:
1. If a tombstone exists for the (subject, claim_text, claim_type) tuple, `commit_claim` rejects with `ClaimResurrectionBlocked` unless ONE of:
   - **Fresh independent evidence**: a new source with `source_asof` strictly newer than the tombstone's timestamp AND with a different `source_id` than the original claim's
   - **Contradiction/supersession record**: an explicit supersession claim that names the tombstone in `supersedes`
   - **User-review path**: the user explicitly accepted via the review queue (recorded as `corrected` feedback on the tombstone)

2. The pre-gate emits `ClaimResurrectionAttempted` to the audit log regardless of outcome (so override patterns are visible).

3. Override audit trail: every override carries the source_id and `source_asof` in the resulting `propose_claim` record so the chain is reconstructible.

Reference: `services/claims.rs::commit_claim` pre-gate logic; `bundle-5` fixture covers correction resurrection + tombstone non-resurrection (W5-B + W6-A patches).

### Subject-fit is a pre-commit or pre-render gate, not only lint/eval

Pre-render gate: `validate_serialized_subject_ownership` (`src-tauri/src/abilities/provenance/ownership.rs:227`) runs at the bridge boundary before `AbilityResponseJson` is returned. Hard-errors when `subject_ref` on a field doesn't match the policy built from invocation input + envelope.

`build_ownership_policy_for_invocation` (`:250`) constructs the per-invocation policy. `commands/abilities.rs::invoke_ability` calls validate+build at lines 41-52. Cycle-1 of W6 boundary L2 added per-invocation parameterization.

Pre-commit: `services/claims.rs::commit_claim` rejects claims where the field's `SubjectRef` doesn't match the envelope's subject (`SubjectAttribution` enforcement, ADR-0105 amendment).

W6 cycles 4-15 sealed the deny-by-default rendering boundary. Verified at cycle-15 APPROVE.

### Contradiction handling includes temporal semantics

`TrustGateKind::AuthoritativeContradiction` triggers when an authoritative source contradicts the claim. The temporal semantic: a fresh resolved source (`source_asof` newer than the contradicted claim's) supersedes; an older source can be cited as precedent but doesn't trigger the gate.

Implementation: `factors.rs::contradiction_factor` checks each contradiction's `source_asof` against the claim's. Fresh contradictions cap the score; stale ones do not. Tests cover stale-current contradiction (older contradiction shouldn't gate fresh claim) and fresh-supersedes (new claim takes precedence).

### Tests cover stale-current contradiction, unknown timestamp, wrong subject, revoked source, tombstone resurrection

- Stale-current contradiction: bundle-3 (stale source resurrection) and `dos287_substrate_bundle1_reproduction.rs`
- Unknown timestamp: trust factor unit tests in `src-tauri/src/abilities/trust/factors.rs::tests`
- Wrong subject: bundle-1 (cross-entity ambiguity) + DOS-288 ownership validator subprocess regression
- Revoked source: trust gate triggers tested in `abilities/trust/mod.rs:108-113` test cases
- Tombstone resurrection: bundle-5 (correction resurrection) + `dos283_bundle5_double_refresh_resurrection_test.rs`

## Outstanding

None. Contract is fully satisfied at v1.4.0 wave tip.

## References

- ADR-0105 — Provenance Envelope (SubjectAttribution + source_asof amendments)
- ADR-0106 — Prompt Fingerprinting (replay determinism)
- ADR-0114 — Scoring Unification (factor abstraction; deferred)
- ADR-0115 — ClaimTrustChanged signal policy
- ADR-0123 — Typed feedback semantics
- ADR-0125 — Temporal scope + sensitivity + claim type registry
- ADR-0126 — Retrieval invariants (compression vs distortion)
- DOS-5 (Trust Compiler), DOS-7 (claim schema + tombstones), DOS-10 (freshness decay), DOS-294 (typed feedback), DOS-299 (source_asof), DOS-326 (cross_entity_coherence factor)
