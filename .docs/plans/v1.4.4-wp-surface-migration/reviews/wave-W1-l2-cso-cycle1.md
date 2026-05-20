# L2 (Diff) CSO Review — v1.4.4 W1 Wave (Cycle 1)

**Date:** 2026-05-20
**Reviewer:** `/cso` (Chief Security Officer mode, daily / 8-of-10 confidence gate)
**Branch:** `wave/v1.4.4-w1-stage1a`
**Diff range:** `0f8533e1..9544d930` (HEAD)
**Scope (per Amendment 3):** DOS-477 (entity-detail trust-boundary), DOS-8 (semantic feedback write path), DOS-341 (privacy/redaction), DOS-340 (receipt-vs-audit boundary)
**Authority anchors:** ADR-0108 (provenance rendering + privacy), ADR-0123 (typed feedback semantics), ADR-0125 (claim anatomy / sensitivity / TypeRegistry), ADR-0131 (canonical source_content_hash), ADR-0129 (Tauri+WP local-to-local), `.docs/plans/engineering-ladder.md` Amendment 3, memory `feedback_l2_path_alpha_to_maintenance_project`.
**Active verification performed:** ran `src-tauri/scripts/check_sensitivity_gate_composition.sh` (clean), `src-tauri/scripts/check_audit_disclosure_allowlist.sh` (clean), `src-tauri/scripts/check_audit_disclosure_allowlist.sh.test` (PASS — lint catches disclosure attempt). Read paths exercised: `entity_intelligence/auth.rs`, `claim_receipt/{feedback,privacy,boundary,auth,render}.rs`, `commands/claim_feedback.rs`, plus the integration-test fixture matrices.

---

## VERDICT

**APPROVE.** No findings rise to L2-blocker per the path-α gate. Cycle-1 L0 CSO findings (1-12) were correctly folded into V1.1 of the L0 packet AND landed in the code:

- F1 → sensitivity gate composition is canonical (`render_policy_for_surface` invoked, no parallel `match … sensitivity`), CI lint `check_sensitivity_gate_composition.sh` shipped and passes.
- F2 → `RECEIPT_ALLOWED_FIELDS` is allowlist-primary; fail-loud panic on unknown field (`entity_intelligence/auth.rs:485`, `claim_receipt/boundary.rs:177`); fixture matrix at `tests/dos340_receipt_boundary_snapshots.rs` enforces.
- F3 → `validate_envelope_target` accepts envelope-sets (`entity_intelligence/auth.rs:170`), `composes_set_for` walks transitive composes (`:228`).
- F4/F5 → user-authored free-text sensitivity floor (`feedback.rs::inherit_sensitivity_floor:766`) + ADR-0108 §3 sanitizer wiring (`feedback.rs::sanitize_freetext:653`).
- F6 → 9-variant `FeedbackAction` matches ADR-0123; `allowed_keys_for` (`feedback.rs:639`) is verbatim.
- F7 → `WrongSource` requires `source_content_hash`, constant-time-compared against `current_source_content_hash` (SHA-256 over canonical `data_source ‖ source_ref ‖ item_hash`); index-style identifiers explicitly rejected by `is_opaque_hash`.
- F8 → server-issued idempotency, 60s TTL, scope = `(claim_id, action, actor, metadata_hash)` (`feedback.rs:322,757`); caller-supplied keys rejected at line 241.
- F9 → Agent actor denied at line 266 (`!actor.is_user()` → `AgentActorDenied`).
- F10 → `build_receipt_for_audience` is construction-time allowlist (per-audience builder functions never write disallowed fields), not post-render filter; `OperationalAuditStorage` returns `NonDisclosureAudience` error at line 175.
- F11 → audit disclosure CI lint shipped (`check_audit_disclosure_allowlist.sh` + negative-test fixture `audit_disclosure_negative/` proves it catches violations).
- F12 → `AgentMcp` audience scrubs `claim_id`, `subject_id` (to type), `source_asof`, removes `source` labels — `build_agent_mcp:307`.

Findings below are MEDIUM/observation, **all routed to Codebase Maintenance & Production Quality** (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per `feedback_l2_path_alpha_to_maintenance_project`. None block the merge.

---

## Findings

Severity scale: Critical / High / Medium / Low / Informational. All entries scored against `feedback_enumerate_channels_before_patching` (every boundary concern must terminate in a structural gate, not just a finding).

### Finding 1 — MEDIUM — DOS-477 / path-α

**Sub-ticket:** DOS-477 §5.4
**File:line:** `src-tauri/src/services/entity_intelligence/auth.rs:351-376` + `:469-493`
**Status:** UNVERIFIED (substrate exists + tested; not wired into production render path)
**ADR cite:** AC-477.4, AC-477.11, AC-477.12 (named these functions explicitly)
**Path-α reason:** Theoretical hardening — contract is met by composition through `build_receipt_for_audience`, not regression in PR-touched code.

**Concern.** `redact_provenance_for_surface` and `filter_for_receipt` are shipped, unit-tested, and named in the AC, but **no production code path calls them**. The receipt construction in `claim_receipt/privacy.rs::build_receipt_for_audience` composes the canonical sensitivity gate directly (line 197) and uses per-audience builder functions (`build_user_tauri`, `build_agent_mcp`, etc.) that never write disallowed fields. The contract is satisfied structurally; the named helpers are dead-ish (W2 will consume them when entity-detail surfaces wire through the boundary).

**Exploit scenario.** None directly — the receipt construction path enforces the same contract. The risk is **maintenance drift**: a future engineer adds a new field to `ClaimReceipt`, updates the per-audience builders, but forgets that `filter_for_receipt` exists as a separate allowlist (named identically to the one in `boundary.rs` but covering camelCase DTO keys, not snake_case audit columns). Two `RECEIPT_ALLOWED_FIELDS` constants with the same name in different modules — one in `entity_intelligence/auth.rs:455` and one in `claim_receipt/boundary.rs:52` — is a confusion hazard.

**Recommended fix.** Two options for the maintenance ticket:
1. Wire `redact_provenance_for_surface` + `filter_for_receipt` (entity_intelligence) into the production path as a final pre-serialization checkpoint, OR
2. Rename one of the two `RECEIPT_ALLOWED_FIELDS` constants to disambiguate (e.g., `RECEIPT_DTO_ALLOWED_KEYS` vs `RECEIPT_AUDIT_ALLOWED_FIELDS`) AND mark the entity_intelligence helpers `#[allow(dead_code)]` with a `// W2: wired in DOS-XXX` comment OR remove them with a substrate-deletion ADR amendment.

**Active-verification hook.** Add a CI grep gate: any new field on `ClaimReceipt` must appear (or be explicitly zeroed) in every `build_<audience>` function in `claim_receipt/privacy.rs`. A regex against the struct definition + a check that each audience builder sets every field would catch the drift class. Pair with the existing snapshot fixture matrix.

---

### Finding 2 — MEDIUM — DOS-8 / path-α

**Sub-ticket:** DOS-8 §5.7
**File:line:** `src-tauri/abilities-runtime/src/sensitivity.rs:182-186` (definition) + `src-tauri/src/services/claim_receipt/feedback.rs:266` (call site)
**Status:** UNVERIFIED — not reachable from production Tauri command surface
**ADR cite:** ADR-0125 §2 (actor classification) — not a literal AC violation
**Path-α reason:** Substrate hardening; the only production caller (`commands/claim_feedback.rs:102`) hardcodes `RenderActor::user("user", None)`, so the bypass surface is server-trusted.

**Concern.** `RenderActor::is_user()` returns true whenever `actor` starts with `"user:"` (case-insensitive). This is broad — any string `"user:agent-bypass-attempt"` would satisfy the check. In `submit_claim_feedback`, the Agent denial (line 266) relies on `is_user()` returning false for non-human actors. If a future caller exposes `RenderActor` construction to untrusted input (e.g., an MCP write-tool that synthesizes an actor from the tool-call payload), the prefix loophole becomes exploitable.

**Exploit scenario (hypothetical — requires future wiring).** A v1.4.7 MCP write tool routes `claim_feedback` through the substrate with `actor` derived from the MCP session principal. If the session principal carries a colon-style identifier (`"user:remote-agent"`), the actor passes `is_user()` and the Agent denial doesn't fire. The remote agent then writes feedback as if it were the user.

**Recommended fix.** Tighten `is_user()` to require exact match `"user"` (or a typed enum, e.g., `RenderActor::Kind::User { id: String }` instead of a stringly-typed `actor` field). Today the kind is encoded in the constructor (`RenderActor::user(...)` vs `RenderActor::agent(...)`); make the kind a typed field, not a string prefix. Until then, document the constraint at the call site in `feedback.rs:266` so future MCP wiring can't accidentally hand the helper an attacker-controlled string.

**Active-verification hook.** Add a property test: `for all s in ALPHANUMERIC_PREFIX_STRINGS, RenderActor { actor: format!("user:{s}"), user_id: None }.is_user() == false`. Hook into the maintenance ticket alongside a structural refactor that makes the actor kind a discriminated enum.

---

### Finding 3 — MEDIUM — DOS-477 / path-α

**Sub-ticket:** DOS-477 §5.4
**File:line:** `src-tauri/src/commands/claim_feedback.rs:83-98`
**Status:** VERIFIED (documented W2 deferral)
**ADR cite:** AC-477.2, AC-477.13 (envelope-set binding)
**Path-α reason:** Documented deferred wiring; not a regression in PR-touched code.

**Concern.** The Tauri command builds a `SingleClaimEnvelope` containing exactly the targeted claim id, so `validate_envelope_target` is a tautology in W1. The doc comment at line 83-88 acknowledges this: real envelope-set binding lands in W2 when the command receives a real `EntityIntelligenceEnvelope`. The other authorization layers (sensitivity gate, Agent denial, per-action metadata schema, source content hash) ARE enforced and are the actual security boundary in v1.4.4.

**Exploit scenario.** A user (already authenticated through the Tauri trust boundary) submits feedback against ANY claim id — there is no check that the targeted claim was actually rendered in a context the user was looking at. Mitigated by: (a) the sensitivity gate still gates per-claim read access (`can_surface_for` at line 270 hits the canonical gate), (b) DailyOS is single-user-tier today so there's no privilege escalation across users. The envelope-binding contract becomes load-bearing only when multi-user or MCP-write surfaces consume the same path.

**Recommended fix.** Already tracked — DOS-XXX (W2 entity-detail wiring) consumes the real `EntityIntelligenceEnvelope`. Add an explicit `// W2: replace with parent envelope from get_entity_intelligence response` comment + a TODO in the Tauri command, plus a property test asserting that as soon as `EnvelopeView` returns a non-trivial set, only targets within that set succeed.

**Active-verification hook.** When W2 lands, add an integration test asserting that a `submit_claim_feedback_command` call with a claim_id NOT in the source envelope returns `TargetBinding(ClaimNotInEnvelope)`. Today the unit tests in `feedback.rs::tests` cover this at the substrate level (`refuses_target_outside_envelope`), so the wave-level substrate is sound — the gap is at the command boundary only.

---

### Finding 4 — MEDIUM — DOS-340 / path-α

**Sub-ticket:** DOS-340 §5.8
**File:line:** `src-tauri/src/services/claim_receipt/boundary.rs:148-194`
**Status:** UNVERIFIED — backstop with no production caller
**ADR cite:** None — this is auxiliary substrate, not a literal AC.
**Path-α reason:** Maintenance / naming clarity.

**Concern.** `boundary::filter_for_receipt(&OperationalAuditRow)` and `OperationalAuditRow` are unused outside tests. The DOS-340 boundary is enforced by the CI lint (`check_audit_disclosure_allowlist.sh`) which forbids `SELECT … FROM <audit_table>` in `services::claim_receipt::*`. The runtime filter is a backstop — useful if someone ever does need to project an audit row into a receipt-shaped view, but currently dead. Pairs with Finding 1: there are now TWO `filter_for_receipt` functions (this one + `entity_intelligence/auth.rs:469`) operating on different field-name vocabularies (snake_case audit columns vs camelCase DTO keys).

**Exploit scenario.** None — dead code is not exploitable. But the naming collision is a future-confusion hazard: a reviewer who sees `filter_for_receipt(...)` in a diff cannot tell from the call site alone which contract is being enforced.

**Recommended fix.** Either (a) rename one of the two functions (`filter_audit_row_for_receipt` vs `filter_receipt_snapshot_keys`) OR (b) consolidate into a single namespaced module. Lowest-cost: rename the boundary.rs one to `project_audit_row_to_receipt` since it's a different operation (row projection, not field filtering).

**Active-verification hook.** CI lint that fails if two functions in `services::claim_receipt::*` share a name. Trivial grep gate; pair with the existing CI lint suite.

---

### Finding 5 — INFORMATIONAL — DOS-8

**Sub-ticket:** DOS-8 §5.7
**File:line:** `src-tauri/src/services/claim_receipt/feedback.rs:645` (`allowed_keys_for(WrongSource)`)
**Status:** VERIFIED — documented bridge, not a contract violation
**ADR cite:** ADR-0123 §1 (variant field names verbatim), ADR-0131 (canonical source_content_hash)
**Path-α reason:** Already documented as W2 follow-up; persisted JSON carries both keys.

**Concern.** `allowed_keys_for(WrongSource)` permits both `"source_content_hash"` (canonical per ADR-0131) AND `"source_index"`. Reading the validation code: `source_index` is accepted as a key but the WrongSource branch explicitly requires `source_content_hash` to be present and opaque (line 510-529); a request that supplies only `source_index` is rejected at line 518 with the correct ADR-0131 error message. So `source_index` is on the allowlist but is effectively a no-op pass-through.

**Exploit scenario.** None — the canonical hash IS required and IS validated. The risk is that `source_index` is accepted on the wire and silently ignored, which is benign but could give a caller a false sense of which key matters.

**Recommended fix.** Remove `"source_index"` from `allowed_keys_for(WrongSource)`. The bridge in `bridge_wrong_source_for_writer` mirrors the hash into the legacy `source_ref` field internally; that mapping happens server-side and doesn't need a wire-level passthrough for `source_index`. If the writer tightening to consume `source_content_hash` directly happens in W2 as documented at line 726-733, drop `source_index` at the same time.

**Active-verification hook.** Add a test asserting `submit_claim_feedback` with `metadata: { source_index: 0 }` and NO `source_content_hash` returns `BadRequest("wrong_source.source_content_hash is required (ADR-0131 …)")`. Today's test suite covers this implicitly through the required-field check but not the surplus-key case.

---

## Summary

```
                    Findings:  0 Critical, 0 High, 4 Medium, 1 Informational
                    Verified:  3
                  Unverified:  2 (substrate exists, not yet wired into prod)
                       Path-α: 5 / 5 (all routed to Codebase Maintenance)
                  L2 blockers: 0
```

All five findings are routed to the Codebase Maintenance & Production Quality project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`). The wave merge is approved.

**Trend vs L0 cycle-1:** L0 surfaced 12 findings (4 High, 5 Medium, 3 Informational). All 12 were folded into V1.1 ACs and shipped in the wave diff. L2 surfaces 0 net-new High findings — the contract closure is clean. The 5 Medium findings here are either deferred-wiring observations (Findings 1, 3, 4) or substrate hardening that's better suited to a structural pass than a fold-cycle (Findings 2, 5).

**Reviewer note.** Per `feedback_zoom_out_for_class_pattern_in_l2_loop`, I checked for the class pattern across Findings 1 + 4: both surface the same shape (two functions/constants named `filter_for_receipt` / `RECEIPT_ALLOWED_FIELDS` operating on different vocabularies). Consolidating those naming collisions into a single sweep in the maintenance ticket would be one PR, not two.

---

## Disclaimer

This is an AI-assisted L2 (Diff) review against named acceptance criteria. It catches common substrate patterns and known ADR contracts; it is not a substitute for professional security audit on cutover (multi-user, MCP write surfaces, remote-agent flows in v1.4.7+). Re-run `/cso --comprehensive` after MCP write surfaces land — the threat model changes when actors stop being server-trusted.
