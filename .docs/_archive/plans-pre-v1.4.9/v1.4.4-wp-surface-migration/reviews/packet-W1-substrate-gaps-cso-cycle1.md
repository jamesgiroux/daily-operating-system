# CSO Review — W1 Substrate Gaps L0 Packet (Cycle 1)

**Date:** 2026-05-20
**Reviewer:** `/cso` (Chief Security Officer mode)
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` V1.0
**Scope (per James 2026-05-20 opt-in):** DOS-477 (§5.4), DOS-8 (§5.7), DOS-341 (§5.9). Read-path sub-tickets DOS-459 / DOS-460 / DOS-461 / DOS-335 / DOS-339-wiring / DOS-340 / DOS-507 are OUT of CSO scope this cycle.
**Authority anchors:** ADR-0108 (provenance rendering + privacy), ADR-0123 (typed claim feedback semantics), ADR-0125 (claim anatomy / sensitivity / TypeRegistry), `.docs/plans/engineering-ladder.md` Amendment 3, memory `project_engineering_ladder`.
**Substrate ground truth verified:** `src-tauri/abilities-runtime/src/sensitivity.rs` exports `RenderSurface`, `RenderActor`, `ClaimSensitivity`, `RenderPolicy`, `RenderDecision`, `RenderableClaimText`, `ClaimVerificationState`, `render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)`. `src-tauri/src/services/claim_receipt/contracts.rs` exports `ReceiptTarget`, `SurfaceContext` (ActionsWork | EntityDetail | DailyBriefing | MeetingDetail | Mcp), `Freshness`, `RedactionLevel`, `ReceiptTrust`, `ReceiptLifecycle`, `ProvenanceSource`, `ClaimReceipt`. `claim_receipt/auth.rs` already at 319 LOC with `AuthError` + `can_surface_for` (referenced in §5.4 + §5.7).

---

## Verdicts (one per CSO sub-ticket)

- **DOS-477 (§5.4) — entity-detail trust-boundary hardening:** **CONDITIONAL APPROVE.** Substrate composition is correct; sensitivity gate name in packet (`render_policy_for`) does not match the shipped symbol (`render_policy_for_surface`) and four high-severity gaps need to be closed before L1 dispatch.
- **DOS-8 (§5.7) — semantic claim feedback actions:** **CONDITIONAL APPROVE.** Server-issued idempotency, per-action JSON schema validation, and authorization matrix are well-modelled. Three high-severity gaps must be folded into AC: (a) FeedbackAction variant set divergence vs ADR-0123, (b) `WrongSource` index integrity check, (c) `NeedsNuance` text sanitization parity with ADR-0108 §3.
- **DOS-341 (§5.9) — receipt privacy / redaction rules:** **CONDITIONAL APPROVE.** Audience matrix is well-grounded in ADR-0108 §2. Two high-severity gaps: (a) redaction is allowlist-based at the field level but the packet's `apply_privacy_for_audience` signature implies a transform-after-render path that lets non-allowlisted fields ride through if `ClaimReceipt` grows new fields; (b) the `OperationalAuditStorage` audience is correct as a non-disclosure assertion class but the packet does not state where the assertion lives (CI fixture vs runtime panic vs both).

None are BLOCKED at L0 — all three are accept-with-folded-AC. Cycle-2 dispatch only if a reviewer disputes the fix surface.

---

## Findings

Severity scale: Critical / High / Medium / Low / Informational. Active-verification hooks named per memory `feedback_enumerate_channels_before_patching`: every boundary finding gets a structural gate, not just a test.

### Finding 1 — High — DOS-477 §5.4

**Concern:** Symbol name mismatch — packet references `abilities_runtime::sensitivity::render_policy_for` (and "ADR-0108 primitives"). The shipped symbol at `src-tauri/abilities-runtime/src/sensitivity.rs:257` is `render_policy_for_surface(claim: &IntelligenceClaim, surface: RenderSurface, actor: &RenderActor) -> RenderDecision`. The `renderable_claim_text_with_value` helper (`:270`) is the public composition primitive most W2 callers should consume. Per memory `feedback_check_substrate_before_authoring_primitives` + `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`, this is exactly the class of drift that gets caught in L0 K-in but the packet labels still carry the proposed-name shape.

**Recommended fix:** Update §5.4 contract sketch + §5.6 / §5.9 cross-references to use `render_policy_for_surface` + `renderable_claim_text_with_value`. Add an explicit AC-477.11: `services::entity_intelligence::auth::redact_provenance_for_surface` MUST compose `render_policy_for_surface` (not re-implement). CI lint forbids any `match claim.sensitivity { … }` outside `abilities_runtime::sensitivity` module.

**Active-verification hook:** Grep gate in `src-tauri/scripts/check_sensitivity_gate_composition.sh` modelled on `check_claim_writer_allowlist.sh` — fails CI if any file outside `abilities-runtime/src/sensitivity*` contains `match … sensitivity` against `ClaimSensitivity` variants. Pairs with the existing `prompt-channel-sensitivity-class-sweep` precedent.

---

### Finding 2 — High — DOS-477 §5.4

**Concern:** `RECEIPT_ALLOWED_FIELDS` / `AUDIT_ONLY_DENYLIST` (declared in §5.8 DOS-340 but co-binding to §5.4 boundary work) is a **denylist + allowlist hybrid**. The packet's `filter_for_receipt` applies the receipt allowlist; the `AUDIT_ONLY_DENYLIST` is informational rather than primary. This is the structural rule from the L0 Addendum, but the packet's open architectural question §13.2 still flags it as undecided. ADR-0108 §2 (Agents never see…) is allowlist-shaped (positive list of what crosses). Denylist + allowlist hybrid leaks the moment a new field name lands.

**Recommended fix:** Lock the contract as **allowlist-primary** at the boundary. The denylist becomes a redundant CI lint that fails the build if any field outside `RECEIPT_ALLOWED_FIELDS` appears in a `ClaimReceipt`-JSON-serialized snapshot fixture. Resolve §13.2 in V1.1 fold with allowlist-primary recommendation. Add AC-477.12: `filter_for_receipt` panics on encountering unknown field names (fail-loud, per Rule 11).

**Active-verification hook:** Snapshot fixture set under `src-tauri/tests/claim_receipt_boundary/` — one serialized ClaimReceipt per (SurfaceContext × ClaimSensitivity) cell. CI compares against allowlist by parsing the JSON and asserting every key ∈ `RECEIPT_ALLOWED_FIELDS`. New fields added to `ClaimReceipt` struct FORCE either snapshot update + allowlist amendment, or boundary review.

---

### Finding 3 — High — DOS-477 §5.4

**Concern:** §5.4 `validate_envelope_target` asserts target belongs to the rendered envelope. But the packet's §5.10 DOS-507 daily briefing envelope (BriefingState::Full + meeting refs) composes `get_entity_intelligence` per referenced subject. If a user clicks `MarkFalse` on a claim shown in the briefing, the feedback flows via §5.7 — but `validate_envelope_target` only sees the *briefing* envelope, which may not directly contain the claim_id (the claim is nested inside one of the sub-envelopes). Either the briefing must explicitly enumerate every nested claim_id, or `validate_envelope_target` must accept transitively-composed envelopes. Packet does not say.

**Recommended fix:** Decide explicitly: AC-477.13 — `validate_envelope_target` accepts an envelope-set (the produced envelope + its transitively composed child envelopes via the abilities-runtime `composes` declaration). Implementation: each ability's `composes = [...]` metadata defines the transitive set; the validator walks the composition graph at invocation-time and accepts targets from any node. This composes cleanly with the existing abilities-runtime `composes` metadata seen in §5.1 and §5.10.

**Active-verification hook:** Property test in `services::entity_intelligence::auth::tests` — for every (parent_ability, child_ability) in `composes`, assert that a target valid for the child envelope is valid for the parent envelope. Failing pair = drift between renderer and validator.

---

### Finding 4 — Medium — DOS-477 §5.4

**Concern:** AC-477.5 says "Correction/feedback payloads are sensitivity-classified; user-authored text does not leak into MCP / reports / logs / cite chips / generic provenance drawers." But §5.7 `ClaimFeedbackRequest.metadata: Option<serde_json::Value>` (≤4KB, ≤4 nesting, ≤24 keys, deny unknown) is the carrier of user-authored corrections (e.g., `NeedsNuance.corrected_text`, `WrongSubject.intended_subject_ref`, `CannotVerify.note`). The packet does not state how user-authored free-text is classified for sensitivity at the moment it's written. Default behavior would be `ClaimSensitivity::Internal` (ADR-0125 §2 default) — but a `NeedsNuance` correction might quote a confidential customer detail.

**Recommended fix:** AC-8.9 (new): user-authored free-text fields in `ClaimFeedbackRequest.metadata` are persisted with `ClaimSensitivity::Confidential` by default (not `Internal`), unless the originating claim's sensitivity is higher (in which case inherit). Render policy for surface stays unchanged — `Confidential` is hidden from MCP / log structured by default, click-to-reveal in Tauri. Document this default in ADR-0123 amendment (or note for v1.4.4 amendment-bundle).

**Active-verification hook:** Test fixture: write a `NeedsNuance` feedback with `corrected_text` containing a synthetic confidential token (e.g., `CONFIDENTIAL-{uuid}`); query MCP surface; assert token does not appear in rendered output.

---

### Finding 5 — High — DOS-8 §5.7

**Concern:** ADR-0123 §1 enumerates 9 variants with specific shapes:
- `WrongSubject { corrected_to: Option<SubjectRef> }`
- `WrongSource { source_index: usize }`
- `NeedsNuance { corrected_text: String }`
- `SurfaceInappropriate { surface: SurfaceId }`
- `NotRelevantHere { invocation_id: InvocationId }`

§5.7 taxonomy table says `WrongSource` requires `source_ref` (not `source_index`). The packet's `apply_feedback_to_state` at `feedback.rs:222-372` is the canonical writer; mismatched parameter shape between the public `ClaimFeedbackRequest.metadata` schema and the substrate variant payload guarantees a wire-format bug. Memory `feedback_ground_first_drafts_in_real_codebase` + `feedback_naming_md_caveats` apply.

**Recommended fix:** AC-8.10 (new): per-action metadata JSON schema MUST match the ADR-0123 §1 variant payload field names verbatim. `WrongSubject.intended_subject_ref` → `corrected_to` per ADR; `WrongSource.source_ref` → `source_index` per ADR. Schema test asserts JSON-schema-emitted field set == ADR-0123 §1 variant field set (golden parity test, similar to TS mirror parity).

**Active-verification hook:** New crate-level test in `claim_receipt::feedback::tests` — for each FeedbackAction variant, serialize a minimal valid metadata payload + deserialize through the substrate writer. Mismatch = compile or runtime panic. Pairs with the existing `record_claim_feedback_persists_a_row_per_action_for_each_of_9_variants` test at `claims.rs:13283`.

---

### Finding 6 — High — DOS-8 §5.7

**Concern:** `WrongSource { source_index: usize }` — the index references the position in the **rendered** receipt's `provenance.sources` array. If a claim's source set changes between render and feedback submission (source revoked, source added, source order changed via dedup), the index aliases to a different source. ADR-0107 source revocation flow + ADR-0108 `ProvenanceMasked` make this concrete: a user clicks `WrongSource` on source[2]=`glean_doc_X`; revocation runs; receipt re-renders with source[2]=`linear_issue_Y`; feedback applies to the wrong source. Source-reliability decay then punishes the innocent source.

**Recommended fix:** AC-8.11 (new): `WrongSource` metadata MUST carry a stable source content hash or canonical source identity (per ADR-0131 structured embedding canonicalization), not an index. Server validates the hash against the *current* claim's source set at feedback-apply time. If the source no longer exists in the set, the feedback is rejected with `BadRequest::SourceNoLongerInClaim` (caller is asked to re-render + resubmit). Optionally retain `source_index` as a presentation hint but never as the authoritative identifier.

**Active-verification hook:** Adversarial fixture in `feedback::tests::wrong_source_index_aliasing_race` — race a source revocation against a `WrongSource` submission; assert no source-reliability decay applied to innocent successor source. Pairs with memory `feedback_enumerate_channels_before_patching` — every source-identity channel into the feedback writer audited in one sweep.

---

### Finding 7 — High — DOS-8 §5.7

**Concern:** `NeedsNuance { corrected_text }` (≤500 chars per ADR-0123) is free-text user input that gets persisted, surfaced in receipts, and presumably consumed by the trust compiler (ADR-0123 §4 — text-overlap heuristic ≥ 0.5 determines refinement vs contradiction). ADR-0108 §3 specifies a sanitizer for LLM-generated `FieldAttribution.explanation` text (HTML entity encoding, URL stripping, banned-token list, no executable content). User-authored text gets the same threat model: prompt injection, PII smuggling, attacker-controlled URLs. Packet does not specify whether ADR-0108 §3 sanitizer applies.

**Recommended fix:** AC-8.12 (new): user-authored `corrected_text` (NeedsNuance), `note` (CannotVerify), and any other free-text field in `ClaimFeedbackRequest.metadata` passes through the ADR-0108 §3 sanitizer pipeline BEFORE persistence. Sanitizer instance is shared with the existing `FieldAttribution.explanation` path. Sanitization warnings surface in the response (e.g., `ProvenanceWarning::ExplanationFiltered`) so the user knows their text was modified.

**Active-verification hook:** Test fixture matrix: each banned-token injection pattern from ADR-0108 §3 attempted via NeedsNuance free-text. Assert sanitizer applied + warning surfaced + persisted text is sanitized. CI lint: forbid any persistence path for user-authored claim-adjacent text that bypasses `abilities_runtime::sanitizer::*` (centralize per memory `feedback_enumerate_channels_before_patching`).

---

### Finding 8 — Medium — DOS-8 §5.7

**Concern:** §5.7 authorization matrix says `ActionsWork` + `EntityDetail` allow Agent (SurfaceClient) up to `Internal` sensitivity. But ADR-0125 §2 makes `Internal` the **default** for claims sourced from internal systems — meaning the bulk of claims an agent encounters are `Internal`-classified. The matrix as-stated lets the SurfaceClient agent submit feedback on ANY `Internal`-or-lower claim. Per ADR-0123 §8 ("Feedback from non-User actors. … v1.4.0 commits user-only"), agent-side feedback is **out of scope** at the ADR level today. Packet's matrix opens an authorization door the ADR closes.

**Recommended fix:** AC-8.13 (new): in v1.4.4 W1, `actor: Actor::Agent` is **denied** at all surfaces (not allowed-up-to-Internal). The matrix row collapses to `deny` for Agent everywhere until an explicit ADR-0123 amendment + v1.4.7 MCP scope manifest expands it. Tighten in §5.7 substrate; defer broadening to v1.4.7 path.

**Active-verification hook:** Authorization matrix table-test in `claim_receipt::feedback::tests::authorization_matrix` — for every (surface × actor × sensitivity) cell, assert the matrix outcome. Agent-Allow-Internal would fail today.

---

### Finding 9 — Medium — DOS-8 §5.7

**Concern:** Server-issued idempotency key (AC-8.2) — packet says caller-supplied keys rejected with `BadRequest::CallerSuppliedIdempotencyKey`. This is correct (good defense against replay), but the packet does not state the key's TTL / scope. A long-lived idempotency key cache lets a stale retry overwrite a recently-applied feedback in the rare case where the user submits MarkFalse, then ConfirmCurrent on the same claim within the cache window. The cache must be keyed on `(claim_id, action_variant, actor, content_hash_of_metadata)` and expire on a short window (seconds, not minutes).

**Recommended fix:** AC-8.14 (new): server-issued idempotency key is scoped per `(claim_id, action, actor, metadata_hash)`. TTL ≤ 60 seconds. Outside the window, retries are treated as new submissions (since two genuinely-distinct feedbacks at >60s separation reflect distinct user intent).

**Active-verification hook:** Test in `feedback::tests::idempotency_window_boundary` — submit two distinct feedbacks on same claim with same metadata; second succeeds if outside 60s.

---

### Finding 10 — High — DOS-341 §5.9

**Concern:** `apply_privacy_for_audience(receipt, audience, sensitivity) -> ClaimReceipt` — signature implies a transform applied to an already-rendered receipt. The DOS-339 substrate at `render::render_receipt_for` already composes `render_policy_for_surface` for the claim text. Two layers of redaction (render-time text policy + post-render audience filter) is a recipe for drift — a confidential token could survive the post-render filter if it landed in a non-allowlisted field that gets added to `ClaimReceipt` in a future PR. Per memory `feedback_systemic_look_for_recurring_issue_classes`, this is the audit-disclosure-allowlist class.

**Recommended fix:** Restructure §5.9 contract: `apply_privacy_for_audience` is a **render-time** primitive, not a post-render transform. It runs INSIDE `render_receipt_for` and constructs only the allowlisted fields per audience. The function signature becomes `build_receipt_for_audience(target, audience, conn) -> ClaimReceipt` — i.e., the audience is an input to construction, not a filter on output. Allowlist source: per-audience field list in `privacy.rs` const. AC-341.10 (new): no post-render mutation; receipts are immutable; same target + audience → byte-identical receipt.

**Active-verification hook:** Snapshot fixture per (audience × sensitivity × claim_type) — golden JSON files. CI lint asserts JSON key set per audience is a subset of the audience-specific allowlist. New field on `ClaimReceipt` forces explicit audience-allowlist amendment.

---

### Finding 11 — High — DOS-341 §5.9

**Concern:** Privacy matrix `OperationalAuditStorage` row says "no surface — storage only; non-disclosure assertion class with positive/negative fixtures". Packet does not specify where the non-disclosure assertion is *enforced*. ADR-0108 §1 separates the security audit (`.jsonl`, ADR-0094) from operational audit (`maintenance_audit` SQLite). The risk is: a future surface (e.g., a debug viewer for power users, an admin tool) accidentally queries `maintenance_audit` and exposes raw model i/o through a code path that bypasses `apply_privacy_for_audience`. Memory `project_engineering_ladder` "trust-boundary fence" CI gate is the structural pattern.

**Recommended fix:** AC-341.11 (new): `services::claim_receipt::privacy::OperationalAuditStorage` audience is a non-disclosure tag, not a render target. ANY call site that touches `maintenance_audit` rows must be either (a) the audit-management commands allowlisted in DOS-340 §5.8 lint, OR (b) routed through `apply_privacy_for_audience` with a non-OperationalAuditStorage audience. Cross-bind with `check_audit_disclosure_allowlist.sh` (DOS-340) — same lint, just expanded to cover the privacy-side as well.

**Active-verification hook:** Extend `check_audit_disclosure_allowlist.sh` to also forbid any direct read of `maintenance_audit` table rows from `services::claim_receipt::*` (the privacy module is allowed to *write* assertions but never to *surface* raw rows). Pair with negative-fixture test: a deliberately-attempted disclosure path that the lint catches.

---

### Finding 12 — Medium — DOS-341 §5.9

**Concern:** Matrix row "Product receipt (Tauri / WP block render)" treats Tauri and WP block render as a single audience. Per memory `feedback_wp_is_local_surface_not_remote` + ADR-0129 (WP Studio as primary surface), WP block render IS local-to-local — same trust boundary as Tauri. Correct framing. But MCP rendered outputs (mentioned at §5.9 as v1.4.7 consumer) are a distinct audience: agent-side, must collapse internal IDs per ADR-0108 §2. The matrix as-drafted lists `AgentMcp` separately from `UserTauri`, but the field-allowlist enumeration only spans User-Tauri-WP-Activity-Lint-Audit — Agent/MCP audience is named in the enum but not given an explicit field-allowlist row.

**Recommended fix:** AC-341.12 (new): add explicit `AgentMcp` row to the privacy matrix. Field allowlist for AgentMcp: trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary (sanitizer per ADR-0108 §3), `subject_type` (not `subject_id`). Forbid: source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak).

**Active-verification hook:** AgentMcp snapshot fixtures in `claim_receipt/privacy/tests/agent_mcp_audience` — golden files per sensitivity. CI lint catches any field bleed.

---

### Finding 13 — Informational — Cross-cutting (DOS-477 + DOS-8 + DOS-341)

**Concern:** All three sub-tickets cite L2 CSO approval as an AC. Memory `feedback_l2_then_pr_not_intermediate_pushes` + `feedback_l2_is_not_optional` apply — L2 CSO must run on the wave-integrated diff before PR submission, not as separate PRs. Per memory `feedback_l2_wave_scope_not_per_pr`, L2 is wave-scoped; the three CSO-mandatory sub-tickets ride the same L2 CSO pass.

**Recommended fix:** No new AC needed; clarify in §11 reviewer matrix that the L2 `/cso` pass for the W1 wave covers all three sub-tickets at the integrated-diff level, not per-PR. Surface in proof bundle (§7 AC-W1.5 already lists the five sub-tickets — make explicit that L2 CSO is one pass over the integrated diff).

**Active-verification hook:** Wave-integrated L2 `/cso` script invocation against the merged-on-dev W1 SHA range. Verdict logged as Linear comment per memory `feedback_linear_is_the_audit_trail`.

---

## Cross-cutting recommendations

1. **Allowlist-primary across all three sub-tickets.** Replace denylist + allowlist hybrids with allowlist-primary boundaries (Findings 2, 10, 12). Denylist is CI lint redundancy, not the contract. Aligns with ADR-0108 §2 "Agents never see…" allowlist shape.

2. **Substrate symbol names are authoritative.** Packet must use `render_policy_for_surface` / `renderable_claim_text_with_value` (not proposed `render_policy_for`). Sweep in V1.1 fold (Finding 1).

3. **Sanitize user-authored free-text at the same gate as LLM-authored text.** ADR-0108 §3 sanitizer is the single shared primitive (Finding 7). New AC across DOS-8 + ADR amendment captures the parity.

4. **One CI lint script covers the receipt-boundary class.** DOS-340 `check_audit_disclosure_allowlist.sh` + the new `check_sensitivity_gate_composition.sh` (Finding 1) + the privacy-side disclosure extension (Finding 11) all sit in `src-tauri/scripts/`. Memory `feedback_zoom_out_for_class_pattern_in_l2_loop` — this is the structural-gate class.

5. **Authorization matrix collapses Agent rows to deny in v1.4.4.** Tighten in §5.7 substrate; broadening waits for v1.4.7 MCP scope manifest + ADR-0123 amendment (Finding 8).

---

## Pass-rule per packet §11

Unanimous APPROVE required across the 4-panel + `/cso` on the three sub-tickets named. This verdict is `CONDITIONAL APPROVE` on each, with 12 specific findings to fold into V1.1. Recommend the packet author folds the findings + reposts V1.1; cycle-2 dispatch only if a reviewer disputes a fix surface. Per memory `feedback_l0_review_loop_diminishing_returns_means_scope_is_wrong`: if cycle-2 surfaces 5+ net-new findings, scope is wrong; this verdict reflects scope-correct, contract-incomplete.

**End of CSO Cycle 1 verdict.**
