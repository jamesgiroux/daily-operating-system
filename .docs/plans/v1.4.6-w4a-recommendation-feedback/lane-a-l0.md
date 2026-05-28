---
ticket: DOS-332
title: "v1.4.6 W4-A — Recommendation feedback loop"
parent_plan: ../v1.4.6-waves.md
fork_sha: 2a6ec716
branch: codex/v1.4.6-w4a-recommendation-feedback
status: L0 — plan hardening, cycle 0 draft
authors: James Giroux, Claude
related:
  - ADR-0123 (Typed Claim Feedback Semantics — 10 variants, V1.1)
  - ADR-0102 (Abilities as Runtime Contract)
  - ADR-0105 (Provenance as First-Class Output)
  - ADR-0125 (Claim Anatomy / Sensitivity / TypeRegistry)
  - DOS-332 (this ticket)
  - DOS-298 (W3-A — Suggested Next Steps block contract, merged)
  - DOS-331 (W2-A — Surfacing policy, merged; owns cooldown state)
  - DOS-329 (W1-A — RecommendationClaim contract, merged; owns FeedbackState shape)
---

# v1.4.6 W4-A — Recommendation feedback loop (DOS-332)

## Cycle 1 amendments (2026-05-27) — 4-reviewer panel resolutions

Cycle 0 panel: adversarial BLOCKED (3 BLOCKED + 5 NEEDS_REVISION + 2 NIT); feasibility CYCLE_2 (1 BLOCKED + 5 NEEDS_REVISION); scope-guardian CONDITIONAL APPROVE (1 BLOCKED + 3 TRIMs); K-in NO REINVENTION (but with substrate-side answers that resolve most BLOCKED items by pointing at what exists).

**The class pattern across reviewers:** the cycle-0 packet was authored without enough grounding in the actual substrate. It invented APIs (`bump_cooldown`), missed ADR-0123 variants (`NotRelevantHere`, `NeedsNuance`, `SurfaceInappropriate`), and proposed a new signal where one already exists. Cycle 1 reframes the packet around primitives that actually exist — verified against code at:

- `src-tauri/src/services/recommendations/surfacing.rs:33` (`SURFACING_DECISION_SIGNAL` is a string const, not an enum variant)
- `src-tauri/src/services/recommendations/surfacing.rs:120,136` (`feedback_suppression_days: i64` default 14; cooldown is read-derived)
- `src-tauri/src/services/recommendations/surfacing.rs:724` (`latest_feedback_suppression` reads `claim_feedback` rows)
- `src-tauri/src/services/claims.rs:5488-5527` (`validate_feedback_actor` admits ONLY `User`; SurfaceClient/Agent/System all rejected by `actor_class_for_actor`)
- `src-tauri/src/services/claims.rs:9127` (`claim_feedback_recorded` signal already emits from `record_claim_feedback`)
- `src-tauri/abilities-runtime/src/abilities/feedback.rs:39` (`FeedbackAction` enum has all 10 ADR-0123 variants including `NotRelevantHere`, `NeedsNuance`, `SurfaceInappropriate`)
- `src-tauri/src/migrations/269_recommendation_claim_metadata_indexes.sql:9-18` (`feedback_state` lives in `intelligence_claims.metadata_json` as JSON — indexed via `json_extract`, NOT a column)

### B1. The mapping table reframed — every variant writes via `record_claim_feedback` (supersedes §1)

The cycle-0 mapping was wrong on three rows and confused on three more. Cycle 1 uses ADR-0123's existing variant catalog faithfully. **Every `RecommendationFeedbackDecision` variant writes to `claim_feedback` with the appropriate `FeedbackAction`.** Cooldown emerges automatically from the read side (`latest_feedback_suppression`); there is no separate cooldown table, no separate cooldown writer.

| `RecommendationFeedbackDecision` | `FeedbackAction` | Truth/Trust effect | Surfacing effect | Rationale |
|---|---|---|---|---|
| `Accept { at }` | `ConfirmCurrent` | +α on source / agent | None | Standard reinforcement (ADR-0123 §149 row #1) |
| `Dismiss { reason: NotRelevant }` | `NotRelevantHere` | No trust delta | 14-day cooldown via `feedback_suppression_days` | "True but not relevant here" — ADR-0123 §149 explicitly for this case. Resolves adv F2. |
| `Dismiss { reason: AlreadyKnew }` | `NotRelevantHere` | No trust delta | 14-day cooldown | Not disputing truth, just not useful surfacing. Same as NotRelevant. |
| `Dismiss { reason: WrongSubject }` | `WrongSubject` | -0.3 subject_evidence on linker; source untouched | Tombstones the claim on the asserted subject (per-subject) | Direct ADR-0123 mapping |
| `Dismiss { reason: Other(BoundedNote) }` | `NotRelevantHere` with `note` field populated | No trust delta; note captured | 14-day cooldown | Conservative default — user provided free text but not a structured judgment; treat as "not relevant" + log the note via ADR-0123's typed `note: Option<String>` column (NOT `payload_json`). Resolves adv F3. |
| `NotUseful { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown + flags for review | ADR-0123 §149 overflow set — "this surface placement is wrong"; ranking-only |
| `TooNoisy { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown (same as NotUseful at substrate level; intensity is a UX framing, not a substrate distinction) | Per K-in: K-in could not find a §149 variant that distinguishes "too noisy" from "surface inappropriate." Both map to the same FeedbackAction. Cooldown bump is uniform; future cross-claim aggregation (ADR-0123 §207 deferred to v1.5.0+) handles "user often marks X as noisy" learning. |
| `Convert { at, into: Action(action_id) }` | `ConfirmCurrent` | +α on source / agent | None | Reinforcement + open-loop attachment. Open-loop attachment is a SEPARATE service call (`services::actions::attach_from_recommendation` or equivalent); not transactional with the claim_feedback write because attachment may invoke external side effects (calendar / message scheduling). **Cycle 1 explicit:** the ability returns success after `claim_feedback` write; open-loop attachment is at-least-once via a follow-up service call within the same ability body. If attachment fails, the feedback row stays (truth-feedback is authoritative); the attachment is retried via existing action retry infrastructure. Resolves adv F4. |
| `Convert { at, into: ClaimCorrection(claim_id) }` | `NeedsNuance` with `corrected_text` payload | Refinement path per ADR-0123 §149 row #7 | Cooldown on original subject | ADR-0123 `NeedsNuance` is **specifically named** for user-authored corrections via text-overlap heuristic. K-in cited ADR-0123 §154 structured-only rules as routing structured corrections elsewhere — but §154 covers structured corrections IN the feedback API. Our `ClaimCorrection(claim_id)` references an EXISTING claim that already went through propose/commit. So the original recommendation's correctness is refined; `NeedsNuance` with the corrected claim_id as evidence is the cleanest mapping. Resolves adv F8. |
| `Convert { at, into: ReviewQueue(queue_item_id) }` | None (no claim_feedback row) | No trust delta | No cooldown | DOS-336 review queue routes independently; this variant emits the queue item id only. W4-A doesn't write claim_feedback here. |

**Q1/Q2/Q3 from cycle-0 §12 are RESOLVED by this table** (per scope-guardian TRIM):
- Q1: `Dismiss { Other }` → `NotRelevantHere` with typed `note`
- Q2: `NotUseful` vs `TooNoisy` cooldown magnitude → same at substrate; UX distinction is surface-side
- Q3: `Convert { ClaimCorrection }` → `NeedsNuance` (substrate keeps the original claim refining-path semantic)

### B2. Cooldown is read-derived — drop the `bump_cooldown` API (supersedes §1, §3.4, §11)

Cooldown is NOT a separate write API. `services::recommendations::surfacing::latest_feedback_suppression` reads `claim_feedback` rows within the policy's `feedback_suppression_days` window (default 14) and surfaces them through `feedback_suppressed_recently: bool` in the surfacing state. **A feedback write IS the cooldown bump.**

This deletes the `bump_cooldown` references throughout cycle-0 §1 / §3.4 / §11. The W4-A scope shrinks: no cross-lane coordination with W2-A, no scope creep into surfacing.rs, no parallel cooldown table. Resolves adv F1 + feas #2.

### B3. Signal: reuse `claim_feedback_recorded`, do not declare new (supersedes §3.1, §3.4 step 5, §4 AC #8)

`record_claim_feedback` already emits the `claim_feedback_recorded` signal (string const at `services::claims:9127`). All recommendation feedback paths flow through `record_claim_feedback`, so this signal fires automatically for W4-A writes.

The cycle-0 packet's new `RecommendationFeedbackRecorded` SignalType variant is **dropped**. Resolves scope F4 (W0 ownership concern — by dropping the new variant, W0 ownership of `signals/policy_registry.rs` is preserved) and feas #6.

If W4-B (deviation) or W4-C (engagement) need finer-grained discrimination, they consume `claim_feedback_recorded` with payload filtering — the existing signal carries `feedback_id`, `claim_id`, `feedback_type` (which is the `FeedbackAction`). For the `NeedsNuance` and overflow-set variants, downstream lanes can filter by `feedback_type`. No new signal needed.

### B4. Category is `Maintenance`, not `Write` (supersedes §3.1)

Per ADR-0102 §82, ability categories are call-graph-derived (Read / Transform / Publish / Maintenance), not author-declared. `submit_recommendation_feedback` mutates internal state through `services::claims::record_claim_feedback` (which itself is the canonical maintenance path for `intelligence_claims` mutations). The ability is **Maintenance**.

ADR-0103 maintenance-ability constraints apply: `may_publish = false`, `client_side_executable = false`, no MCP exposure for maintenance abilities by default (`mcp_exposure = None`). Updated:

- **`category`:** `Maintenance` (not `Write`)
- **`required_scopes`:** `["maintenance.recommendations.feedback"]` (replaces the cycle-0 `write.recommendations` since maintenance scopes follow a different naming convention; verify at L1 grep against existing maintenance scope strings)
- **`mutates`:** `["intelligence_claims", "claim_feedback"]` declared at the macro

This is the **first Maintenance ability under `recommendations/`** (cycle-0 was framed as the first Write; that framing was wrong). Inventory pattern: existing Maintenance abilities elsewhere in the codebase establish the pattern; W4-A follows.

### B5. Actor normalization — SurfaceClient → "user" at the ability boundary (supersedes §3.1, §3.4)

`validate_feedback_actor` (`services::claims:5507`) admits only `User` (`"user"` or `"human"` strings via `actor_class_for_actor`). SurfaceClient string `"surface_client"` is unmapped and would hard-fail with `InvalidFeedback`.

**Resolution:** the ability normalizes `ctx.actor()` to `"user"` before calling `record_claim_feedback`. The actor-class check inside `record_claim_feedback` is for the CONTENT of the feedback (it IS a user-class feedback regardless of which surface posted it). The original Actor identity (`SurfaceClient { instance, scopes }`) is preserved in `RecommendationFeedbackContext` (existing field) and in the ability's provenance envelope, so the audit trail records the surface that posted.

This is a 2-line normalization at the ability body, not an extension of `actor_class_for_actor`. Resolves feas #1.

### B6. `feedback_state` mutation is JSON-patch, not column UPDATE (supersedes §3.4 step 4)

`RecommendationClaim.feedback_state` lives in `intelligence_claims.metadata_json` as a JSON field, indexed via `json_extract(metadata_json, '$.recommendation.feedbackState…')` per migration `269_recommendation_claim_metadata_indexes.sql`. The mutation path uses SQLite's `json_set`:

```sql
UPDATE intelligence_claims
SET metadata_json = json_set(metadata_json, '$.recommendation.feedbackState', ?, '$.recommendation.conversionState', ?)
WHERE id = ?
  AND json_extract(metadata_json, '$.recommendation.feedbackState') LIKE '%"pending"%'
```

The `WHERE … LIKE '%"pending"%'` clause is the **atomic compare-and-set** for idempotency (resolves adv F6): if the row's feedback_state is already non-Pending, the UPDATE affects 0 rows, the ability returns `EffectKind::NoMutation`. SQLite's single-writer guarantee (per feas #4) prevents the read-then-write race. The packet cites SQLite single-writer rather than implying row-level locking.

### B7. Output type trim — `EffectKind` replaces `DownstreamEffect` (supersedes §3.3)

Per scope-guardian TRIM: the cycle-0 `DownstreamEffect` enum carried inner fields (`feedback_id`, `new_cooldown_until`, etc.) for a confirmation-chip use case in a surface that's parked. Trim to a leaner discriminator:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    ClaimFeedbackRecorded,
    NoMutation,
}
```

`NoMutation` is the idempotent-reject case (already-decided per B6). When a surface ships and wants confirmation-chip metadata, that ability evolution is additive (add `feedback_id` field; bump schema_version).

The cycle-0 `BothApplied` and `SurfacingCooldownBumped` variants are gone because B2 eliminated the separate cooldown path. Every successful write produces `ClaimFeedbackRecorded`; cooldown is read-side.

`Convert { Action }` still attaches the open-loop; its success/failure does NOT show up in `EffectKind` — instead, the response carries an `Option<ActionAttachment>` field for the surface to render. Failure of the open-loop attach does not roll back the feedback (per B1 Convert row + adv F4).

### B8. Output wrapped in `AbilityOutput<T>` for provenance (supersedes §3.3)

Per K-in (ADR-0105 §8 "lives-once" invariant), the response shape is:

```rust
pub type SubmitRecommendationFeedbackOutput = AbilityOutput<SubmitRecommendationFeedbackResponse>;
```

Provenance envelope lives once on the `AbilityOutput`; the response struct itself carries no envelope copy. Standard `ProvenanceBuilder` pattern used by all v1.4.6 abilities.

### B9. WP REST Write-path verified (resolves feas #8 + adv F5)

Per feas grep: `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85` exposes `invoke_ability(name, payload, scope_set)` category-agnostic. No category-specific branching in the transport. This is the **first Maintenance ability** to traverse it (cycle-0 mis-framed as Write), but the transport admits all categories. No new transport work.

L1 verification step: confirm `tools/dailyos-abilities.json` regen handles `Maintenance` category as passthrough (per feas #9 — existing inventory has Read/Transform/Maintenance categories already, so this is exercised).

### B10. Cycle 0 §12 open questions — all resolved

- Q1 (`Dismiss { Other }` mapping) → B1 row: `NotRelevantHere` with typed `note`
- Q2 (NotUseful vs TooNoisy) → B1 rows: same FeedbackAction (`SurfaceInappropriate`); UX distinction is surface-side
- Q3 (Convert { ClaimCorrection } target) → B1 row: `NeedsNuance` with corrected_text payload
- Q4 (Signal payload shape) → B3: dropped new signal; consume existing `claim_feedback_recorded`
- Q5 (WP REST endpoint shape) → B9: existing transport admits all categories

### B11. CI gate inputs add per-Actor dry-run row (supersedes §7)

Per scope-guardian TRIM: per-Actor dry-run test enumerated in §5 channel #6 and §6 unit tests but missing from §7 CI gate table. Added:

| Artifact | Notes |
|---|---|
| Per-Actor dry-run test | `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error. Lives in `abilities-runtime/src/abilities/recommendations/mod.rs::tests`. CI-enforced gate, not advisory. |

### B12. DoD additions (supersedes §14)

Three additions per adv F9 + F10:

- **L4 filter-flip gate.** The W3-A WP block's `dailyos_suggested_next_steps_feedback_enabled` filter flip from `false` to `true` is **NOT** an automatic post-merge step. It is an L4-gated change requiring hands-on QA of the full WP block → REST → ability → `record_claim_feedback` → claim_feedback row → surfacing.rs read pickup vertical. The flip ships separately from W4-A merge, gated by L4 evidence + DOS-298 / DOS-333 readiness.
- **Privacy non-leak documented.** `EffectKind::ClaimFeedbackRecorded` carries no subject identity; surface that submitted the feedback already knows its own claim's subject. No new privacy surface introduced. Documented in §3.3.
- **Auth overhaul note.** ADR-0111 §193 nonce requirement for write events is currently being stripped for local same-user contexts per the 2026-05-21 auth overhaul (per memory). W4-A does NOT add nonce machinery; it relies on the post-overhaul actor-allowlist model. If the overhaul lands differently than expected, W4-A's transport gate at L1 grep adds nonce verification.

### B13. Scope shrinks net

After cycle 1: NO new tables, NO new SignalType variants, NO new cooldown API, NO new scope (just a maintenance-scope rename), NO new column (json_set on metadata_json). The ability + service-function fill (`feedback.rs`) is the entire scope.

This is appropriately smaller than W3-A. Per scope-guardian: "If W4-A's packet bloats toward W3-A scale, that's a signal." Cycle 1 trims it back to substrate-extension.

### Cycle 1 status

- B1–B12 amendments apply; B13 trim confirmed
- Original §1, §3.1, §3.3, §3.4, §4, §7, §12, §14 sections are SUPERSEDED inline by cycle 1 (the inline-sweep pattern from W3-A's cycle 3); a future cycle-2 reviewer reading any single section should see consistent state without needing to cross-reference cycle 1 amendments

---

## Cycle 2 amendments (2026-05-27 late) — substrate grounding pass

Cycle-2 panel verdict: adversarial BLOCKED (5 new findings); feasibility CYCLE_3 (3 defects); scope-guardian CYCLE_3 (1 scope decision + stale text cleanup).

**The class pattern recurred:** cycle 1 dropped the cycle-0 `bump_cooldown` hallucination but introduced new hallucinations of the same class (typed `note` column that doesn't exist; `LIKE '%"pending"%'` predicate that doesn't actually match; `attach_from_recommendation` retry infra that doesn't exist). Per adversarial reviewer's own recommendation: "the packet author needs to do a single grounding pass against the live schema, the FeedbackState serde shape, and the actions service module, then re-emit a packet whose every 'X already exists' claim is line-cited."

This amendment block does that grounding pass. **Every claim below is file:line-cited and verified against the current `dev` HEAD (`2a6ec716`).**

### B14. Compare-and-set predicate fixed (supersedes B6 + §3.4 step 4)

`LIKE '%"pending"%'` was empirically broken in `sqlite3 :memory:` testing. Reason: `json_extract` strips JSON quotes from primitive string values; the LIKE pattern searches for embedded quotes that never appear in the extracted text.

**Verified empirically:**

```
sqlite> SELECT json_extract(json('{"recommendation":{"feedbackState":"pending"}}'),
                            '$.recommendation.feedbackState');
pending                              -- bare text, no quotes
sqlite> SELECT json_extract(...) LIKE '%"pending"%';
0                                    -- DOES NOT MATCH
sqlite> SELECT json_extract(...) = 'pending';
1                                    -- correct match
```

**Corrected predicate** (replaces B6's UPDATE):

```sql
UPDATE intelligence_claims
SET metadata_json = json_set(metadata_json,
    '$.recommendation.feedbackState', json(?),
    '$.recommendation.conversionState', json(?))
WHERE id = ?
  AND json_extract(metadata_json, '$.recommendation.feedbackState') = 'pending'
```

Note: `json_set` requires explicit `json(?)` wrapping when the bound parameter is itself a JSON value (e.g., the Decided object). For the Pending → Decided transition, the new feedbackState is `{"decided": {...}}` (object), so `json(?)` parses the parameter as JSON. SQLite `json_extract` continues to return the unquoted primitive string `"pending"` for the prior state, allowing the `= 'pending'` compare-and-set to succeed only when the row is in Pending state.

**FeedbackState serde shape verified** at `src-tauri/src/services/recommendations/contracts.rs:103-106`:

```rust
#[derive(...)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackState {
    Pending,                                          // → bare JSON string "pending"
    Decided(RecommendationFeedbackDecision),          // → {"decided": {...}}
}
```

No `#[serde(tag = ...)]` → external tagging default. Pending serializes to the JSON string `"pending"`; Decided serializes to an object with the `decided` key.

The packet's atomic compare-and-set is structurally sound; only the predicate was broken. Cycle 3 fixes it.

### B15. `note` storage — encode in payload_json with validator extension (supersedes B1 row #5 + §4 AC#7 + §12 Q1)

**Grounded against schema:** `claim_feedback` table at `src-tauri/src/migrations/245_dos_484_feedback_merge_intent.sql` has columns:

```
id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at, applied_at
```

**There is no `note` column.** `ClaimFeedbackInput` at `src-tauri/src/services/claims.rs:230-236` has fields:

```rust
pub struct ClaimFeedbackInput {
    pub claim_id: String,
    pub action: FeedbackAction,
    pub actor: String,
    pub actor_id: Option<String>,
    pub payload_json: Option<String>,
}
```

**There is no `note: Option<String>` field.** ADR-0123 §2 describes an intended typed shape, but it is not present in the live substrate. The cycle-1 packet's "typed `note: Option<String>` column" claim is wrong.

**Correction.** The note encodes into `payload_json` as a JSON key alongside the action-required key:

```json
{ "invocation_id": "<surface_invocation_id>", "note": "<up to 200 chars>" }
```

**Validator extension required.** `validate_feedback_action_metadata` at `claims.rs:5560-5570` is action-keyed:

```rust
match action {
    FeedbackAction::WrongSource       => require_payload_string(action, payload, "source_ref"),
    FeedbackAction::NeedsNuance       => require_payload_string(action, payload, "corrected_text"),
    FeedbackAction::SurfaceInappropriate => require_payload_string(action, payload, "surface"),
    FeedbackAction::NotRelevantHere   => require_payload_string(action, payload, "invocation_id"),
    FeedbackAction::MergeIntent       => require_payload_object(action, payload, "merge_target"),
    _ => Ok(()),
}
```

L1 extends this for `NotRelevantHere` to admit an OPTIONAL `note` key (size-validated to 200 chars at the writer, matching `BoundedNote::MAX_CHARS` from `contracts.rs:148`). The validator change is a 3-line addition: keep the required `invocation_id` check, add an optional `note` length check.

This is a **net-new validator-allowlist line, not a schema change.** §4 AC#5 "no new migrations" holds; §4 AC#11 "first Maintenance ability under recommendations/" is unaffected. New CI gate input row in §7: "Validator extension: `NotRelevantHere` admits optional `note` key with 200-char cap."

§4 AC#7 cycle-1 text superseded: "The note is encoded in `payload_json.note` per B15. The 200-char cap is enforced at the API boundary by `BoundedNote::try_from`; the validator extension at `claims.rs:5560` enforces it server-side after JSON deserialization."

### B16. Convert{Action} simplifies — pure state update, no external side effect (supersedes B1 Convert{Action} row + §3.3 ActionAttachment / AttachStatus + §3.4 step 6)

**Grounded against actions service.** Greppy survey of `src-tauri/src/services/`:

- `sync_action_open_loop_claim` exists at `src-tauri/src/services/action_claims.rs` — takes an existing `DbAction`, syncs its open-loop claim shape. Does not link an arbitrary action to a recommendation.
- `commitment_bridge.rs` handles commitments → action linkage. Not recommendation-specific.
- `attach_from_recommendation` does NOT exist. The cycle-1 packet's "existing action retry infrastructure" claim is a hallucination.

**Key insight from `ConversionState` enum** (`contracts.rs`):

```rust
pub enum ConversionState {
    NotConverted,
    ConvertedToAction { action_id: String },          // existing action_id
    ConvertedToClaimCorrection { claim_id: ClaimId }, // existing claim_id
    ConvertedToReviewQueue { queue_item_id: String }, // existing queue_item_id
}
```

Every Convert variant carries an ID of an **already-existing** entity. The user has the action / claim / queue item in hand BEFORE submitting feedback. The recommendation just records "I was converted to that thing." This is pure state mutation on the recommendation's `metadata_json`. **No external service call. No attachment. No retry infrastructure.**

**Convert{Action(action_id)} flow simplifies:**

1. `record_claim_feedback(... ConfirmCurrent ...)` — reinforce source/agent
2. `json_set` on `metadata_json` sets `recommendation.conversionState = {kind: "convertedToAction", actionId: <action_id>}`
3. Done. No `ActionAttachment` type, no `AttachStatus::Queued`, no retry.

**Output simplifies (supersedes §3.3):**

```rust
pub struct SubmitRecommendationFeedbackResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub feedback_state: FeedbackState,
    pub conversion_state: ConversionState,
    pub effect_kind: EffectKind,
    pub recorded_at: DateTime<Utc>,
}

pub enum EffectKind {
    ClaimFeedbackRecorded,
    NoMutation,
}
```

`ActionAttachment` and `AttachStatus` types from cycle 1 are **dropped**. The conversion_state field on the response already carries the action_id when relevant; no separate envelope needed.

**Resolves the cycle-1 atomicity-honesty concern** (adv F4): there is no external side effect, so there is no honesty problem to disclose. The cycle-1 framing was solving a non-problem.

### B17. `actor_id` populated with SurfaceClient instance (supersedes B5 audit-trail gap)

Per adversarial cycle 2 new attack surface: when normalized to `"user"`, the `claim_feedback.actor` column loses SurfaceClient identity. **Resolution: populate `actor_id`.**

Verified at `claims.rs:230-236`: `ClaimFeedbackInput` has `actor_id: Option<String>`. Setting this to the SurfaceClient instance string (e.g., `"surface:wp-block-suggested-next-steps-{uuid}"`) preserves audit-trail identity in the DB row alongside the normalized `actor: "user"`.

For `Actor::User` direct invocations, `actor_id` is `None`. For `Actor::SurfaceClient { instance, .. }`, `actor_id = Some(instance.to_string())`.

**Verified against actor_class_for_actor** at `claims.rs:5488`: the function splits on `[:, /, @]` so `actor: "user"` admits regardless of `actor_id` value — the audit identity is captured without breaking validation.

### B18. Signal payload corrected (supersedes B3 description in cycle 1)

**Grounded against signal emission** at `claims.rs:9112-9136`:

```rust
let payload = serde_json::json!({
    "action": write.outcome.action.as_str(),
    "claim_id": &write.outcome.claim_id,
    "verification_state_before": &write.verification_state_before,
    "verification_state_after": &write.verification_state_after,
}).to_string();
```

Cycle 1's claim that the signal carries `feedback_id, claim_id, feedback_type` is wrong. The actual payload is `{action, claim_id, verification_state_before, verification_state_after}`.

**Discriminator surface for downstream W4-B/W4-C:**

- `action` is the FeedbackAction string (e.g., `"not_relevant_here"`, `"surface_inappropriate"`, `"needs_nuance"`, `"confirm_current"`, `"wrong_subject"`)
- `claim_id` is the recommendation's claim_id

W4-B / W4-C consumers filter recommendation-feedback events by joining on `claim_id` against `intelligence_claims` to confirm `claim_type = 'recommendation'`. They cannot filter from signal payload alone without the join. **This is a known cost, not a defect** — the signal is a notification, not a self-contained record.

§4 AC#8 superseded: "Signal payload is `{action, claim_id, verification_state_before, verification_state_after}` per the existing `claim_feedback_recorded` emission at `claims.rs:9114`. Downstream W4-B/C consumers filter via JOIN against `intelligence_claims` on `claim_type = 'recommendation'`."

### B19. Scope name follows existing convention (supersedes B4 scope naming)

Per feasibility cycle 2: existing scope strings in `registry.rs:2759, 2845` follow `verb.noun` (e.g., `read.account_overview`, `submit.feedback`). The cycle-1 `maintenance.recommendations.feedback` has no precedent — grep returned zero `maintenance.*` scopes.

**Corrected:** `submit.recommendations.feedback` (matches existing `submit.feedback` convention; verb prefix; noun-namespace).

§3.1 `required_scopes` superseded: `["submit.recommendations.feedback"]`.

ADR-0103 maintenance-ability constraints still apply via category, not scope namespace.

### B20. Stale-text cleanup (sweep of cycle-1 in-document references to dropped primitives)

Cycle 2 scope-guardian flagged stale references to dropped concepts (B3 SignalType, B7 ActionAttachment, B16 retry infra). Cleanup applied inline:

- **§5 channel #5** — was: "`signals/policy_registry.rs` `RecommendationFeedbackRecorded` variant." Updated: "No new SignalType variant; consume existing `claim_feedback_recorded` per B3 + B18."
- **§6 integration test bullets** — references to `MarkOutdated` (was cycle-0 mapping for Convert{ClaimCorrection}) replaced with `NeedsNuance` per B1; references to declaring a new signal dropped.
- **§9 cross-wave coordination** — sentence "W4-B's L0 reads from `RecommendationFeedbackRecorded` signal declared here" replaced with "W4-B's L0 reads from the existing `claim_feedback_recorded` signal (filtering via JOIN on `claim_type = 'recommendation'`)."
- **§13 risk register** — last row "Signal type declared but unused" dropped entirely (no signal declared).
- **§7 CI gate inputs** — L2-status row dropped (per scope cycle 2: commit hygiene, not artifact deliverable).

### B21. Verified-against-codebase appendix (new §15)

Per adversarial cycle 2: "the packet must include a 'verified-against-codebase' appendix with file:line citations for every 'X already exists' claim." Added as §15.

### Cycle 3 status

- B14–B21 amendments apply; cycle 2 hallucinations resolved with citations
- B16 simplifies the Convert{Action} path — drops `ActionAttachment` / `AttachStatus` types entirely
- B17 closes the SurfaceClient audit-trail gap via `actor_id` field (no schema change)
- B19 aligns scope naming with codebase convention
- B20 sweeps stale references

**Ready for cycle 3 panel re-dispatch.** Recommended subset: adversarial only (verify the LIKE-fix + note-storage + Convert simplification all close their cycle-2 findings). Feasibility + scope-guardian skipped — their cycle-2 findings are addressed by B14–B19 with citations they can verify by spot-check.

---

## §0 Frame

W4-A is **substrate-only**. Per the UI Surface Deferral Amendment (2026-05-27), v1.4.6 continues with backend wiring while WP and Tauri surfaces wait for production readiness + redesign respectively. W4-A delivers the feedback ability + service path so that *when* a surface ships its affordance, the producer is ready.

Scope per `v1.4.6-waves.md:1005-1022`:

1. Fill the placeholder at `src-tauri/src/services/recommendations/feedback.rs` (today: 7-line docblock — see read at L0).
2. Author the recommendation-feedback wrapper API: `record_recommendation_feedback(claim_id, decision, context) -> FeedbackState`.
3. Map each `RecommendationFeedbackDecision` variant (Accept / Dismiss{reason} / NotUseful / TooNoisy / Convert) onto:
   - the existing `FeedbackAction` enum + `services::claims::record_claim_feedback` write path (for truth-feedback semantics) OR
   - the surfacing cooldown state (for ranking-only feedback, never trust)
4. Ship the typed `FeedbackState::Pending → FeedbackState::Decided(...)` transition on the `RecommendationClaim`.
5. Tests verify each variant produces the documented ADR-0123 action and the right downstream effect (trust vs surfacing vs ranking).

**No new tables.** `claim_feedback` is the existing storage. ADR-0123's 10-variant enum is fixed; W4-A consumes it.

**No new ADR.** This is a wrapper around existing primitives.

**No UI dependency.** W3-A's WP block has affordance markup but disabled handler (Option A). When this lane ships, the block's `dailyos_suggested_next_steps_feedback_enabled` filter flips to true. DOS-802's Tauri surface follows the same pattern when it eventually lands.

**Reviewer routing (per CLAUDE.md L0 default + W4-A wave-plan W4-A-specific):**

- `/codex challenge` — adversarial planning reviewer (parallel)
- `ce-feasibility-reviewer` — substrate alignment (claim_feedback shape, FeedbackAction surface, cooldown state)
- `ce-scope-guardian-reviewer` — confirm no parallel tables, no new ADR, no UI scope creep
- `ce-learnings-researcher` — K-in mandatory parallel (look for prior feedback-mapping work, ADR-0123 amendments, prior feedback solution memories)

No `ce-design-lens-reviewer` — substrate-only. No `ce-security-lens-reviewer` — W4-A doesn't touch actor exposure or telemetry (W4-C does; this lane doesn't).

---

## §1 The mapping (locked at cycle 1; see B1)

**Every `RecommendationFeedbackDecision` variant writes to `claim_feedback` via `record_claim_feedback` with the appropriate ADR-0123 `FeedbackAction`.** Cooldown emerges automatically from the read side (`services::recommendations::surfacing::latest_feedback_suppression`); there is no separate cooldown table or writer.

| `RecommendationFeedbackDecision` | `FeedbackAction` | Truth/Trust effect | Surfacing effect (read-derived) |
|---|---|---|---|
| `Accept { at }` | `ConfirmCurrent` | +α on source / agent | None |
| `Dismiss { reason: NotRelevant }` | `NotRelevantHere` | No trust delta | 14-day cooldown via `feedback_suppression_days` |
| `Dismiss { reason: AlreadyKnew }` | `NotRelevantHere` | No trust delta | 14-day cooldown |
| `Dismiss { reason: WrongSubject }` | `WrongSubject` | -0.3 subject_evidence on linker; source untouched | Tombstones the claim on the asserted subject (per-subject) |
| `Dismiss { reason: Other(BoundedNote) }` | `NotRelevantHere` with `note` encoded in `payload_json` | No trust delta; note encoded as `{"invocation_id": ..., "note": "..."}` in `payload_json` (per B15 — `claim_feedback` has no typed `note` column; validator extended to admit optional `note` key) | 14-day cooldown |
| `NotUseful { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown |
| `TooNoisy { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown (same as NotUseful at substrate level; intensity is UX framing, not substrate distinction) |
| `Convert { at, into: Action(action_id) }` | `ConfirmCurrent` | +α on source / agent | None. **Pure state update** (per B16): `conversion_state` set to `ConvertedToAction { action_id }` via `json_set` on `metadata_json`. The `action_id` references an already-existing action the caller created/holds; W4-A does NOT create the action and does NOT attach. No external service call. If the caller submits a bogus `action_id`, the conversion_state points at a non-existent action (acceptable — see AC#14). |
| `Convert { at, into: ClaimCorrection(claim_id) }` | `NeedsNuance` with `corrected_text` payload referencing the corrected claim_id | Refinement path per ADR-0123 §149 row #7 | Cooldown on original subject |
| `Convert { at, into: ReviewQueue(queue_item_id) }` | None (no claim_feedback row) | No trust delta | None — DOS-336 review queue routes independently |

**Rationale citations:**

- `NotRelevantHere` (ADR-0123 §149) is the explicit variant for "true but not relevant here — no trust delta." Used for `Dismiss{NotRelevant|AlreadyKnew|Other}`.
- `SurfaceInappropriate` (ADR-0123 §149 overflow set) covers "this surface placement is wrong — ranking-only." Used for `NotUseful` and `TooNoisy`.
- `WrongSubject` (ADR-0123 §1) tombstones at the asserted subject; linker takes the -0.3 hit; source untouched. Used only for explicit `Dismiss{WrongSubject}`.
- `NeedsNuance` (ADR-0123 §149 row #7) is specifically named for user-authored corrections via text-overlap heuristic. Used for `Convert{ClaimCorrection}`.
- `ConfirmCurrent` (ADR-0123 §149 row #1) is standard reinforcement. Used for `Accept` and `Convert{Action}`.

**Cooldown:** `surfacing.rs::latest_feedback_suppression` reads `claim_feedback` rows within `feedback_suppression_days` (default 14, per `SurfacingPolicy` at `surfacing.rs:120,136`). Every feedback write IS the cooldown bump; no separate writer needed.

**FeedbackState transition:** every successful variant transitions `Pending → Decided(<the variant>)` on the `RecommendationClaim`'s `metadata_json` JSON field via `json_set` (NOT a column UPDATE — the field is indexed via `json_extract` per migration `269_recommendation_claim_metadata_indexes.sql`). The mutation is wrapped in an atomic compare-and-set: `UPDATE … WHERE json_extract(metadata_json, '$.recommendation.feedbackState') LIKE '%"pending"%'`. SQLite single-writer guarantees the race-free path (see B6).

---

## §2 Scope + producer authority

### In scope

| File / surface | Owner |
|---|---|
| `src-tauri/src/services/recommendations/feedback.rs` (fill 7-line placeholder) | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/` — new `submit_recommendation_feedback` ability | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/contracts.rs` — additive input/output types for the new ability | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/mod.rs` — register ability | W4-A |
| `src-tauri/src/services/context.rs` — wire feedback handle | W4-A |
| `tools/dailyos-abilities.json` — regen | W4-A |
| Rust unit tests covering all 10 mapping rows above (parametric) | W4-A |
| Integration test: full Pending → Decided cycle with seeded `RecommendationClaim` | W4-A |

### Out of scope

- WP block UI changes (block already has affordance markup gated by `dailyos_suggested_next_steps_feedback_enabled`; flip after W4-A merges OR in a follow-up depending on WP surface readiness — see UI Surface Deferral)
- Tauri React UI (DOS-802 parked)
- New tables / migrations (use `claim_feedback` + W2-A cooldown state)
- New ADR-0123 variants (consume the 10 existing)
- Engagement telemetry path (W4-C lane)
- Deviation detection (W4-B lane)
- Salience re-compute timing changes (W1-B owns; W4-A emits the feedback that re-compute consumes on next cycle)

### Producer authority restatement

Wave plan §line 1015-1020:
- Claim model: feedback writes through service path (existing `record_claim_feedback`)
- Provenance: user feedback affects future salience factor weights (UserFit factor — W1-B re-compute consumes)
- Signals: emit existing claim-feedback signals; no new signal types
- Runtime: wired from W3-A action handlers (when UI flips on)
- Feedback loop: this IS the feedback loop primitive

---

## §3 Ability contract

### §3.1 Identity

- **Name:** `submit_recommendation_feedback`
- **Category:** `Maintenance` — per ADR-0102 §82 the category is call-graph-derived. Mutates internal state through `services::claims::record_claim_feedback` (canonical maintenance path for `intelligence_claims` mutations). ADR-0103 maintenance-ability constraints apply.
- **Allowed actors:** `[User, SurfaceClient]` — the user OR a surface acting on the user's explicit click. NOT `System` (no automatic feedback), NOT `Agent` / `Admin`, NOT `McpClient`. Per B5 + B17, the ability **normalizes `ClaimFeedbackInput.actor = "user"`** (so `validate_feedback_actor` at `claims.rs:5507` admits) AND **populates `actor_id` with the SurfaceClient instance string** (when caller is `Actor::SurfaceClient { instance, .. }`) so the audit trail in `claim_feedback.actor_id` (per migration 245 schema) preserves identity. For `Actor::User` direct invocations, `actor_id` is `None`.
- **`required_scopes`:** `["submit.recommendations.feedback"]` (per B19 — matches existing `submit.feedback` convention at `registry.rs:2759, 2845`; `maintenance.*` prefix from cycle 1 has no codebase precedent)
- **`may_publish`:** false (per ADR-0103 maintenance default)
- **`mcp_exposure`:** `None` (per ADR-0103 maintenance default)
- **`client_side_executable`:** false (per ADR-0103)
- **`mutates`:** `["intelligence_claims", "claim_feedback"]` declared at the `#[ability(...)]` macro
- **`composes`:** `[]` (does NOT compose `claim_receipt` — idempotent compare-and-set on `feedback_state` happens in the UPDATE itself, no separate read needed)
- **`signal_policy.emits_on_output_change`:** `[]` — the existing `claim_feedback_recorded` signal fires automatically from `record_claim_feedback` (`claims.rs:9127`); W4-A does not declare a new signal (B3).
- **Schema version:** 1

### §3.2 Input

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRecommendationFeedbackInput {
    pub schema_version: u32, // pinned to 1 per A14 schema-version-cliff policy
    pub claim_id: ClaimId,
    pub decision: RecommendationFeedbackDecision, // existing enum from contracts.rs
    pub context: RecommendationFeedbackContext,   // existing struct (surface + invocation_id)
}
```

`RecommendationFeedbackDecision` and `RecommendationFeedbackContext` already exist in `src-tauri/src/services/recommendations/contracts.rs` (W1-A landed them). No new types required.

### §3.3 Output

The ability returns `AbilityOutput<SubmitRecommendationFeedbackResponse>` per ADR-0105 §8 "lives-once" invariant. The provenance envelope rides on the wrapper; the inner response carries no envelope copy.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRecommendationFeedbackResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub feedback_state: FeedbackState,           // always Decided(<variant>) after success; Pending on NoMutation
    pub conversion_state: ConversionState,        // updated if Convert variant; carries the action_id / claim_id / queue_item_id inline
    pub effect_kind: EffectKind,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    ClaimFeedbackRecorded,
    NoMutation, // idempotent reject — claim's feedback_state was already non-Pending at compare-and-set time
}
```

Per B16: cycle 1's `ActionAttachment` + `AttachStatus` types are **dropped**. The existing `ConversionState` enum already carries `ConvertedToAction { action_id }` / `ConvertedToClaimCorrection { claim_id }` / `ConvertedToReviewQueue { queue_item_id }` inline — no separate attachment envelope needed. No external service call, no retry infrastructure required; the Convert variants record the user-supplied IDs of already-existing entities.

**Privacy note:** `EffectKind` carries no subject identity. The caller (the surface that posted the feedback) already knows its own claim's subject and decision. No new privacy surface introduced (resolves adv F10).

**Why `EffectKind` not the full `DownstreamEffect` from cycle 0:** per scope-guardian TRIM (B7) — the cycle-0 enum carried fields (`feedback_id`, `new_cooldown_until`, `action_signature`, etc.) for a confirmation-chip surface that's parked. When a surface lights up and needs confirmation metadata, schema-version bump adds fields additively.

### §3.4 Service-side flow

`services::recommendations::feedback::record_recommendation_feedback`:

1. **Resolve claim.** Load the `RecommendationClaim` by `claim_id` via `load_claim_by_id`. Reject with `ClaimError::UnknownClaimId` if not found, or `ClaimError::UnsupportedClaimType` if not a recommendation claim.
2. **Normalize actor.** `ctx.actor()` resolves to either `Actor::User` or `Actor::SurfaceClient { ... }`. The downstream call to `record_claim_feedback` requires the `actor` string to map to `ClaimActorClass::User` via `actor_class_for_actor` (`claims.rs:5488-5504`) — which admits only `"user"` or `"human"`. The ability normalizes by passing `"user"` regardless of which actor variant invoked. The original Actor identity (with `SurfaceClient.instance` / `SurfaceClient.scopes` if applicable) is captured in the provenance envelope at the ability boundary, so audit trail is complete.
3. **Map `decision` variant per §1 table.** Constructs a `ClaimFeedbackInput` with the mapped `FeedbackAction` and an appropriate `note` (for `Dismiss{Other(BoundedNote)}`) or `payload_json` (for `Convert{ClaimCorrection}` carrying `corrected_text` per ADR-0123 §149 row #7).
4. **Idempotent compare-and-set on `feedback_state`.** SQLite atomic UPDATE within `with_claim_transaction`:
   ```sql
   UPDATE intelligence_claims
   SET metadata_json = json_set(metadata_json,
       '$.recommendation.feedbackState', ?,
       '$.recommendation.conversionState', json(?))
   WHERE id = ?
     AND json_extract(metadata_json, '$.recommendation.feedbackState') = 'pending'
   ```
   Predicate is `= 'pending'` (NOT `LIKE '%"pending"%'` — per B14, `json_extract` strips JSON quotes from primitive strings; LIKE-with-quotes would never match). If `rows_affected == 0`, return `Ok(EffectKind::NoMutation)` with the existing state — the claim was already decided. SQLite single-writer prevents the read-then-write race (per feas #4); the WHERE-clause `=` check is the atomic guard. **Resolves adv F6 + cycle-2 dissent.**
5. **Call `record_claim_feedback`.** With the constructed `ClaimFeedbackInput`:
   - `actor = "user"` (normalized per B5)
   - `actor_id = Some(<surface_instance_string>)` for `Actor::SurfaceClient { instance, .. }` invocations; `None` for direct `Actor::User`. Per B17 — preserves audit identity in `claim_feedback.actor_id` column (verified in migration 245 schema).
   - `payload_json` per the action's validator requirement at `claims.rs:5560-5570`:
     - `NotRelevantHere` → `{"invocation_id": "<context.invocation_id>", "note": "<optional BoundedNote, ≤200 chars>"}`. The `note` key is OPTIONAL; L1 extends `validate_feedback_action_metadata` for `NotRelevantHere` to admit it (per B15).
     - `NeedsNuance` → `{"corrected_text": "<reference to corrected claim_id>"}` (uses the corrected claim_id reference; L1 confirms text-vs-id semantics).
     - `SurfaceInappropriate` → `{"surface": "<context.surface>"}`.
     - `WrongSubject`, `ConfirmCurrent` → no required payload keys (per validator).
   This inserts the `claim_feedback` row (UUID generated per `claims.rs:7156`), transitions verification_state per ADR-0123 dispatch, and emits the `claim_feedback_recorded` signal automatically.
6. **Convert variants — pure state update on conversion_state.** Per B16: no external service call. The `ConversionState` enum variants (`ConvertedToAction { action_id }`, `ConvertedToClaimCorrection { claim_id }`, `ConvertedToReviewQueue { queue_item_id }`) carry user-supplied IDs of already-existing entities. The step-4 UPDATE already sets `recommendation.conversionState` via `json_set`; no follow-up call needed.
7. **Convert{ReviewQueue} skips record_claim_feedback.** Per §1 row 10: ReviewQueue conversion routes through DOS-336's review queue lifecycle independently; W4-A's ability skips step 5 (no `record_claim_feedback` call), executes only step 4 (the `json_set` UPDATE setting `conversionState`), then returns.
8. **Return `AbilityOutput<SubmitRecommendationFeedbackResponse>`.** Wrap in provenance via `ProvenanceBuilder` per ADR-0105.

All steps 4–7 are within `with_claim_transaction` (SQLite serializable isolation). **There are no external side effects to roll back** — per B16, Convert variants record IDs of pre-existing entities, not creation calls.

---

## §4 Acceptance criteria

1. **Ability ships and is registered.** `submit_recommendation_feedback` in `tools/dailyos-abilities.json` (regen); `AbilityRegistry` exposes to `[User, SurfaceClient]` with `category = Maintenance`. Required scope `submit.recommendations.feedback` (per B19; matches existing `submit.feedback` convention at `registry.rs:2759, 2845`). `cargo test recommendations::submit_recommendation_feedback` passes.
2. **Mapping is deterministic.** Parametric test enumerates all 10 mapping rows in §1; asserts the documented `(FeedbackAction, EffectKind)` pair for each. Each row's `claim_feedback` row inspection confirms the `payload_json` keys match the validator requirement per `claims.rs:5560-5570`.
3. **Idempotent compare-and-set.** Re-submitting feedback for an already-`Decided` claim returns `EffectKind::NoMutation` and does NOT mutate `claim_feedback`. Per B14: predicate is `json_extract(...) = 'pending'` (verified empirically). Test asserts: pre-Decided claim → UPDATE affects 0 rows → service returns NoMutation without calling `record_claim_feedback`.
4. **Atomicity via SQLite single-writer.** The `metadata_json` json_set UPDATE + the `record_claim_feedback` INSERT are within `with_claim_transaction`. SQLite's serializable isolation prevents partial observation. **No external side effects** — per B16, Convert variants record IDs of pre-existing entities; no creation calls, no retry infra needed.
5. **No new tables.** Migrations diff is empty. Uses existing `claim_feedback` rows; cooldown is read-derived from `latest_feedback_suppression` at `surfacing.rs:724` (per B2).
6. **No new ADR-0123 variants.** Maps to the existing 10 `FeedbackAction` variants at `abilities-runtime/src/abilities/feedback.rs:39`.
7. **Note encoded in `payload_json`, not a typed column.** Per B15: `claim_feedback` table has no `note` column (verified migration 245); `ClaimFeedbackInput` has no `note` field (verified `claims.rs:230-236`). `Dismiss { reason: Other(BoundedNote) }` encodes the note in `payload_json` as `{"invocation_id": "<...>", "note": "<≤200 chars>"}`. L1 extends `validate_feedback_action_metadata` for `NotRelevantHere` to admit the optional `note` key (3-line addition at `claims.rs:5560`).
8. **Signal: reuse `claim_feedback_recorded`.** The existing signal fires automatically from `record_claim_feedback` (`claims.rs:9112-9136`). W4-A does NOT declare a new SignalType. Actual signal payload per B18: `{action, claim_id, verification_state_before, verification_state_after}`. Downstream W4-B/C consume this signal and JOIN against `intelligence_claims` to filter by `claim_type = 'recommendation'`.
9. **Actor normalization + `actor_id` populated.** Per B5 + B17: ability passes `actor = "user"` (so `validate_feedback_actor` at `claims.rs:5507` admits) AND `actor_id = Some(<surface_instance>)` for SurfaceClient invocations. Preserves SurfaceClient audit identity in `claim_feedback.actor_id` column (verified migration 245 schema).
10. **`feedback_state` is JSON, not a column.** Mutation via `json_set` on `metadata_json` (per migration 269). Predicate per B14: `json_extract(...) = 'pending'`.
11. **W3-A filter-flip is L4-gated, not auto-merged.** Per B12: the `dailyos_suggested_next_steps_feedback_enabled` filter flip from `false` to `true` is a separate L4-gated change requiring hands-on QA of the full WP block → REST → ability → record_claim_feedback → surfacing read pickup vertical. The flip does NOT ship with W4-A merge.
12. **First Maintenance ability under `recommendations/`.** Per B4 + feas #8: existing `dailyos-abilities.json` inventory has Read/Transform/Maintenance categories; transport admits Maintenance without changes. L1 confirms regen handles it as passthrough.
13. **Cycle hygiene.** `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean.
14. **Caller responsibility for Convert{into:*} ID references** — per B16 + cycle-3 adversarial NEEDS_REVISION on phantom-id: W4-A does NOT validate that `action_id` / `claim_id` / `queue_item_id` references exist before writing `conversion_state` via json_set. The caller (a surface or test seeder) is responsible for supplying IDs of real entities. If the conversion target is later deleted or never existed, the recommendation's `conversion_state` points at a phantom — acceptable today since: (a) the substrate doesn't enforce referential integrity on JSON-encoded refs anywhere else, (b) surfaces render conversion confirmation eagerly so phantom refs are caller-bug not substrate-bug, (c) downstream consumers (W4-B/C analytics) can detect dangling refs at read time if needed. If future surface evolution requires substrate-side validation, it's an additive ability change.

---

## §5 Registry channel enumeration

Per W3-A's A8 pattern (channels enumerated before merge):

1. **Ability registry** — auto via `#[ability(...)]` macro; tested via `tools/dailyos-abilities.json` diff.
2. **Surface scope registry** — new `submit.recommendations.feedback` scope per B19 (matches existing `submit.feedback` convention). File path at L1 grep is likely `abilities-runtime/src/abilities/registry.rs` (existing scope strings at `registry.rs:2759, 2845`).
3. **WP REST endpoint allowlist** — `wp/dailyos/includes/transport/class-dailyos-runtime-client.php` transport is category-agnostic per feas #8 cycle 0. No allowlist change needed for the new Maintenance category.
4. **`tools/dailyos-abilities.json`** — regen + commit.
5. **No new `signals/policy_registry.rs` row.** Per B3 + B20: W4-A consumes the existing `claim_feedback_recorded` signal at `claims.rs:9112`; no new SignalType variant. W0 ownership preserved.
6. **Per-Actor dry-run test** — assert `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error.
7. **`validate_feedback_action_metadata` extension** — L1 adds optional `note` key admission for `NotRelevantHere` at `claims.rs:5560-5570`. 3-line addition; preserves required `invocation_id` check.

---

## §6 Test plan

### Rust unit

- Mapping table parametric: each `RecommendationFeedbackDecision` variant produces the documented `(FeedbackAction, EffectKind)` pair (10 rows)
- Idempotent compare-and-set: pre-Decided claim returns `NoMutation`; predicate `json_extract(...) = 'pending'` verified to match Pending shape via fixture (per B14)
- `Dismiss { Other }` with note >200 chars rejected at contract level (already enforced by `BoundedNote::try_from`)
- Unknown `claim_id` returns `ClaimError::UnknownClaimId`
- Non-recommendation claim_id returns `ClaimError::UnsupportedClaimType`
- Per-Actor allowlist test ([User, SurfaceClient] admit; rest deny)
- `actor_id` populated correctly: User direct → None; SurfaceClient → Some(instance) (per B17)

### Integration (`cargo test --test` recommendations)

- Full Pending → Decided cycle: seed claim, submit Accept, verify `claim_feedback` row + claim's `metadata_json.recommendation.feedbackState` updated atomically to `{"decided": {"kind": "accept", "at": "..."}}`
- NotUseful path: submit `NotUseful`, verify `claim_feedback` row with `feedback_type = 'surface_inappropriate'` AND `latest_feedback_suppression` returns the row on subsequent surfacing.rs read (cooldown emergent — per B2)
- Convert{ClaimCorrection} path: submit decision, verify `claim_feedback` row with `feedback_type = 'needs_nuance'` + `payload_json` containing `corrected_text` (per B1 row + ADR-0123 §149 row #7); verify `conversion_state` updated to `convertedToClaimCorrection` via json_set
- Convert{Action} path: submit decision with action_id, verify `claim_feedback` row with `feedback_type = 'confirm_current'` + `conversion_state` updated to `convertedToAction { action_id }` via json_set; **NO** external service call (per B16)
- Signal emission: assert `claim_feedback_recorded` signal payload matches `{action, claim_id, verification_state_before, verification_state_after}` per `claims.rs:9112-9136` (per B18)

### Substrate-side smoke (deferred to W3-A integration when UI flips)

- WP block view.js → REST endpoint → ability → service path → claim_feedback → surfacing.rs read pickup — full vertical test. Deferred per UI Surface Deferral; gated by L4 per AC#11.

---

## §7 CI gate inputs (L1 deliverables per memory rule)

| Artifact | Notes |
|---|---|
| `tools/dailyos-abilities.json` regen | Deterministic per W3-A precedent; first Maintenance ability under `recommendations/` |
| `services::recommendations::feedback` doc comments | Document the §1 mapping table inline so future readers don't need to find this packet |
| Per-Actor dry-run test | Asserts `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error. Lives in `abilities-runtime/src/abilities/recommendations/mod.rs::tests`. CI-enforced gate. |
| Parametric mapping test | All 10 mapping rows from §1; asserts `(FeedbackAction, EffectKind)` pair per row + `payload_json` shape per `validate_feedback_action_metadata` requirements at `claims.rs:5560-5570`. |
| Idempotent compare-and-set test | Re-submit feedback for an already-Decided claim → `EffectKind::NoMutation`; zero new `claim_feedback` rows. Tests both predicate halves: pre-Pending claim succeeds, pre-Decided claim short-circuits. |
| `validate_feedback_action_metadata` extension | L1 adds optional `note` key admission for `NotRelevantHere` at `claims.rs:5560-5570` (3-line addition; preserves required `invocation_id` check). |
| `actor_id` populated correctly | Test: `Actor::User` direct → `actor_id = None`; `Actor::SurfaceClient { instance, .. }` → `actor_id = Some(instance_string)`. Per B17. |

**No new `signals/policy_registry.rs` row** — per B3 + B18, W4-A consumes the existing `claim_feedback_recorded` signal at `claims.rs:9112-9136`; no new SignalType variant declared. W0 ownership preserved.

**No `L2-status` row** — per cycle 2 scope-guardian TRIM: L2-status in commit messages is process hygiene enforced by the `.githooks/commit-msg` hook, not a W4-A deliverable artifact. Covered by CLAUDE.md, not by this packet.

---

## §8 Migration disposition

**No new migrations.** Empty migrations diff. v273 stays available for W4-B/W4-C if they need storage.

---

## §9 Cross-wave coordination

- **W3-A (merged):** W3-A's WP block has affordance markup gated by `dailyos_suggested_next_steps_feedback_enabled` filter (default false). Post W4-A merge, a one-line follow-up flips this filter. That follow-up is NOT W4-A scope per the UI Surface Deferral — it depends on WP surface readiness.
- **DOS-802 (parked):** Tauri carve-out follows the same pattern when it unblocks.
- **W4-B (DOS-316 deviation):** consumes feedback signal as Novelty factor input. W4-B's L0 reads from the existing `claim_feedback_recorded` signal (`claims.rs:9112-9136`), filtering recommendation-specific events via JOIN against `intelligence_claims` on `claim_type = 'recommendation'`.
- **W4-C (DOS-317 engagement):** distinct from feedback (engagement is implicit observation; feedback is explicit click). W4-C imports the same `EngagementSignal` types from W1-A contracts but does not call into W4-A's service.
- **W5 eval (DOS-338):** the eval harness tests "feedback updates ranking on next cycle" — depends on W4-A's signal + W1-B's salience consumer.

---

## §10 Out of scope (restatement)

- WP block UI changes
- Tauri React UI (DOS-802 parked)
- New tables, migrations, or ADR-0123 variants
- Engagement telemetry (W4-C)
- Deviation detection (W4-B)
- Salience re-compute timing (W1-B owns)
- Cross-surface dedup of recommendations (W3-B parked)

---

## §11 K-in citations (preliminary; `ce-learnings-researcher` confirms in parallel)

- ADR-0123 (Typed Claim Feedback Semantics) — the 10-variant enum W4-A maps onto
- ADR-0102 (Abilities as Runtime Contract) — ability registration pattern
- ADR-0105 (Provenance as First-Class Output) — feedback writes carry provenance
- `services::claims::record_claim_feedback` (claims.rs:7128) — existing write path
- `services::recommendations::contracts` — `RecommendationFeedbackDecision`, `RecommendationFeedbackContext`, `BoundedNote` already landed in W1-A
- `abilities-runtime::abilities::feedback` — existing `FeedbackAction` enum + claim-feedback ability (for the bridge keys)
- `services::recommendations::surfacing` — W2-A's cooldown state API (consume from this lane)
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` — class-sweep test pattern (less critical here than W3-A but applicable)

K-in needs to confirm: no prior `submit_recommendation_feedback` solution exists; no parallel feedback-mapping memory documents alternative variant→action mappings.

---

## §12 Open questions

**All cycle-0/1/2 open questions resolved by B10 + B14-B21.** No remaining L0-blocking questions.

Cycle 3 confirmed:
- `validate_feedback_action_metadata` for `NotRelevantHere` requires `invocation_id` only (per §15 row); L1 adds optional `note` key admission (3-line addition at `claims.rs:5560`).
- `attach_from_recommendation` does NOT exist and is NOT needed — per B16, Convert{Action} is pure state update on `conversion_state` via json_set; no external service call.

**Resolved at cycle 1:**
- Q1 (cycle 0) `Dismiss { Other }` → `NotRelevantHere` with `note` encoded in `payload_json` (B1 + B15)
- Q2 (cycle 0) `NotUseful` vs `TooNoisy` → same `SurfaceInappropriate` (B1)
- Q3 (cycle 0) `Convert { ClaimCorrection }` → `NeedsNuance` (B1)
- Q4 (cycle 0) Signal payload → consume existing `claim_feedback_recorded` (B3 + B18)
- Q5 (cycle 0) WP REST endpoint → existing transport admits all categories (B9)

---

## §13 Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Mapping table is wrong (a variant punishes source when it shouldn't) | HIGH | Parametric test asserts each variant's `(FeedbackAction, EffectKind)` pair matches §1 + ADR-0123 citations |
| Atomicity bug — claim_feedback row written but recommendation_claim feedback_state not updated | HIGH | Single transaction wrap via `with_claim_transaction`; integration test asserts both observe same state |
| Idempotent re-submission silently fails to short-circuit (predicate bug) | HIGH | Cycle 3 B14 fixed the `LIKE '%"pending"%'` to `= 'pending'` (empirically verified). Test asserts predicate matches Pending state shape from `contracts.rs:103-106`. |
| `BoundedNote` 200-char enforcement bypassed | LOW | Already enforced at `BoundedNote::try_from` construction (`contracts.rs:148`); validator extension imposes server-side cap on `note` value (B15) |
| `submit.recommendations.feedback` scope not registered in WP allowlist → block can't call ability | LOW | Transport at `class-dailyos-runtime-client.php:85` is category-agnostic (verified B9); scope is allowlist-runtime-registered per `SurfaceScope::new` (verified §15) |
| Convert{Action(action_id)} accepts phantom action_id | LOW | Explicit per AC#14 — caller responsibility; W4-A does not validate action existence. If validation needed later, additive ability change |
| Convert{ClaimCorrection(claim_id)} accepts phantom claim_id | LOW | Same as above — `corrected_text` payload references claim_id; W4-A trusts the caller-supplied reference |

---

## §14 Definition of Done

Per CLAUDE.md DoD section:

1. All 5 `RecommendationFeedbackDecision` variants (10 mapping rows including Dismiss reasons) implemented per §1 table
2. End-to-end flow tested: ability invocation → `record_claim_feedback` → `claim_feedback` row written → `claim_feedback_recorded` signal fires automatically → `latest_feedback_suppression` returns the row on next read
3. No stubs or TODOs
4. `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean
5. **L2-status declared in commit messages** — enforced by `.githooks/commit-msg` per CLAUDE.md; this is process hygiene every PR observes, not a W4-A-specific artifact (per cycle 2 scope TRIM moved out of §7 CI gate table). Still binding as a project rule.
6. **No L4 evidence required for the W4-A merge itself** (no UI surface introduced by this lane). However: the `dailyos_suggested_next_steps_feedback_enabled` filter flip in the W3-A WP block is an **L4-gated change**, shipped separately from W4-A merge, gated by hands-on QA of the WP block → REST → ability → record_claim_feedback → claim_feedback row → surfacing read pickup vertical (B12).
7. **Privacy non-leak documented.** `EffectKind` carries no subject identity beyond what the caller already knows about its own claim (B12).
8. **Auth overhaul note.** Per the 2026-05-21 auth overhaul memory, ADR-0111 §193 nonce requirement for write events is being stripped for local same-user contexts. W4-A does NOT add nonce machinery; relies on the post-overhaul actor-allowlist model. L1 verifies the current state via a transport-layer grep (B12).
9. K-out: any class-pattern findings from L2 filed via `/ce-compound mode:headless` at retro close.

---

## §15 Verified-against-codebase appendix (cycle 3 grounding pass)

Per adversarial cycle-2 recommendation. Every "X already exists" claim in this packet has been grep-verified against `public/dev@2a6ec716`. Citations below.

| Claim | File:Line | Verified shape |
|---|---|---|
| `claim_feedback` table schema | `src-tauri/src/migrations/245_dos_484_feedback_merge_intent.sql:25-45` | Columns: `id, claim_id, feedback_type (CHECK IN 10 ADR-0123 variants), actor, actor_id, payload_json, submitted_at, applied_at`. **No `note` column.** |
| `ClaimFeedbackInput` struct | `src-tauri/src/services/claims.rs:230-236` | Fields: `claim_id, action, actor, actor_id, payload_json`. **No `note` field.** |
| `validate_feedback_action_metadata` | `src-tauri/src/services/claims.rs:5560-5570` | Per-action required keys: WrongSource→`source_ref`; NeedsNuance→`corrected_text`; SurfaceInappropriate→`surface`; NotRelevantHere→`invocation_id`; MergeIntent→`merge_target`; rest unrestricted. L1 extends NotRelevantHere to admit optional `note` (B15). |
| `validate_feedback_actor` | `src-tauri/src/services/claims.rs:5507-5527` | Admits only `ClaimActorClass::User`; rejects all others. Normalization required (B5). |
| `actor_class_for_actor` | `src-tauri/src/services/claims.rs:5488-5504` | Maps `"user"\|"human"` → User. Splits on `[:/@]` so `"user:surface-id"` also admits (allowing audit-rich actor strings if desired). |
| `record_claim_feedback` | `src-tauri/src/services/claims.rs:7128-7170` | Wraps in `with_claim_transaction` + `MutationGuard`; generates UUID feedback_id at line 7156; returns `ClaimFeedbackOutcome`. |
| `claim_feedback_recorded` signal emission | `src-tauri/src/services/claims.rs:9112-9136` | Payload: `{action, claim_id, verification_state_before, verification_state_after}`. **No `feedback_id` in emitted payload.** |
| `FeedbackAction` enum (10 ADR-0123 variants) | `src-tauri/abilities-runtime/src/abilities/feedback.rs:39` | All 10 variants present: `ConfirmCurrent, MarkOutdated, MarkFalse, WrongSubject, WrongSource, CannotVerify, NeedsNuance, SurfaceInappropriate, NotRelevantHere, MergeIntent`. |
| `FeedbackState` serde shape | `src-tauri/src/services/recommendations/contracts.rs:103-106` | External tagging default (no `#[serde(tag = ...)]`): `Pending` → bare JSON string `"pending"`; `Decided(x)` → `{"decided": x}`. |
| `feedback_state` storage in `metadata_json` | `src-tauri/src/migrations/269_recommendation_claim_metadata_indexes.sql:9-18` | Indexed via `json_extract(metadata_json, '$.recommendation.feedbackState')`. Not a column. |
| `json_extract` behavior on primitive strings | SQLite empirical test (`sqlite3 :memory:`) | Returns the UNQUOTED text value. `json_extract(...) = 'pending'` matches; `json_extract(...) LIKE '%"pending"%'` does NOT match. |
| `surfacing.rs` cooldown read | `src-tauri/src/services/recommendations/surfacing.rs:724` (`latest_feedback_suppression`); policy at `:120, 136` (`feedback_suppression_days: 14`) | Cooldown derived from `claim_feedback` rows within `feedback_suppression_days` window. No bump_cooldown writer needed (B2). |
| `SURFACING_DECISION_SIGNAL` declaration pattern | `src-tauri/src/services/recommendations/surfacing.rs:33` | `pub const X: &str = "..."` — stringly-typed module constant. Not a centralized SignalType enum. |
| `ConversionState` enum | `src-tauri/src/services/recommendations/contracts.rs` (existing W1-A type) | Variants carry IDs of pre-existing entities: `ConvertedToAction { action_id }`, `ConvertedToClaimCorrection { claim_id }`, `ConvertedToReviewQueue { queue_item_id }`. **No external attachment call required** (B16). |
| `sync_action_open_loop_claim` | `src-tauri/src/services/action_claims.rs` | Takes `&DbAction` (existing); syncs open-loop claim shape. Not a recommendation attachment function. **`attach_from_recommendation` does NOT exist** — Convert{Action} doesn't need it (B16). |
| Existing scope-name convention | `src-tauri/abilities-runtime/src/abilities/registry.rs:2759, 2845` | `verb.noun` (e.g., `read.account_overview`, `submit.feedback`). No `maintenance.*` prefix exists. W4-A uses `submit.recommendations.feedback` (B19). |
| WP REST transport | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85` | `invoke_ability(name, payload, scope_set)` is category-agnostic. Maintenance category traverses without changes (B9). |

**Hallucinations dropped at cycle 3** (each replaced by its grounded primitive in the table above):
- `bump_cooldown` API — read-derived per surfacing.rs:724 instead
- Typed `note: Option<String>` column on claim_feedback — encoded in `payload_json` instead
- `attach_from_recommendation` retry infrastructure — `ConversionState` already carries the action_id; no attachment needed
- New `RecommendationFeedbackRecorded` SignalType variant — reuse existing `claim_feedback_recorded` per claims.rs:9112
- `LIKE '%"pending"%'` predicate — replaced with `= 'pending'` (empirically verified)
- `maintenance.recommendations.feedback` scope name — replaced with `submit.recommendations.feedback` matching codebase convention
