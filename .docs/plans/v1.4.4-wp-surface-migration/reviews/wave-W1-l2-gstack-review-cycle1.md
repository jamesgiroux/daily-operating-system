# L2 (Diff) gstack `/review` — v1.4.4 W1 Wave (Cycle 1)

**Date:** 2026-05-20
**Reviewer:** gstack `/review` skill (Claude adversarial subagent + structured pass)
**Branch:** `wave/v1.4.4-w1-stage1a`
**Base ref:** `0f8533e1`
**Diff range:** `0f8533e1..9544d930` — 115 files, 19,144 insertions, 0 deletions
**Sub-tickets covered:** DOS-335, DOS-459, DOS-339, DOS-477, DOS-340, DOS-341, DOS-460, DOS-8, DOS-461, DOS-507
**Bounding:** per `feedback_l2_must_review_against_acceptance_criteria` + `.docs/plans/engineering-ladder.md` path-α gate; findings cross-checked against L0-packet-W1-substrate-gaps.md §5.1–§5.10 AC anchors AND AC-W1.1–AC-W1.11.
**Active verification:** `cargo clippy -- -D warnings` (green); read paths exercised: `services/{meeting_prep_status,claim_receipt,entity_intelligence}/*`, `abilities-runtime/src/abilities/get_{entity_intelligence,daily_briefing}/*`, `commands/claim_feedback.rs`, `signals/policy_registry.rs`, migrations v241+v242, all four new lint scripts; grep audit of `wp/dailyos/blocks/**/render-functions.php` consumer references.

---

## VERDICT

**CONDITIONAL APPROVE.** Substrate body is structurally sound; ten sub-tickets carry their named ACs at the code level. Two literal AC violations gate the wave-level criteria (AC-W1.2 + AC-W1.9 + AC-340.2 + AC-340.7 + AC-477.11) — these are wiring obligations explicit in the L0 packet, not theoretical hardening. Recommend a small remediation pass before merge to `dev`:

1. Wire the four new CI lint scripts into `.github/workflows/lint-frontend.yml`.
2. Ship `scripts/check_w1_consumer_skeleton.sh` + at least one wp block render PHP skeleton invoking each W1 producer (or downgrade AC-W1.2/W1.9 via a packet amendment).

Other findings are MEDIUM/LOW path-α — file to Codebase Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per `feedback_l2_path_alpha_to_maintenance_project`.

---

## Findings

Severity scale: CRITICAL / HIGH / MEDIUM / LOW / Informational. Confidence per finding.

### Finding 1 — HIGH — AC-W1.9 literal violation (consumer-skeleton CI gate missing)

**Sub-ticket:** AC-W1.9 (wave-rolled-up)
**AC cite:** L0 packet §7 AC-W1.9 — "`scripts/check_w1_consumer_skeleton.sh` (modeled on `check_claim_writer_allowlist.sh`)… Substrate-only PRs without at least one downstream consumer reference fail CI. The script IS the mechanical enforcement for AC-W1.2's 'wiring IS the work' obligation."
**File:line:** N/A — script does not exist (verified via `find . -name "check_w1_consumer_skeleton.sh"` returns empty).
**Confidence:** 10/10.
**Recommended fix:** Add `src-tauri/scripts/check_w1_consumer_skeleton.sh` per the packet pattern (modeled on `check_claim_writer_allowlist.sh`); wire into `.github/workflows/lint-frontend.yml`. Alternatively: file a V1.2 packet amendment narrowing AC-W1.9 to a follow-up (cycle-1 path-α convergence under `feedback_l0_partial_convergence_when_class_recurs`).

### Finding 2 — HIGH — AC-W1.2 literal violation (no downstream consumer skeletons)

**Sub-ticket:** AC-W1.2 (wave-rolled-up)
**AC cite:** "No W1 producer ships without at least one downstream consumer skeleton in W2 (or W3/W4)… Each W1 PR must point to the W2/W3/W4 PR that will consume it… 'Skeleton' = a stubbed block render PHP file that calls the producer and renders a minimal projection — proves the contract carries weight."
**File:line:** Verified — `grep -rln "get_entity_intelligence\|get_daily_briefing\|meeting_prep_status\|entity_intelligence::touchpoints\|claim_receipt::{boundary,feedback,privacy}" wp/` returns zero matches.
**Confidence:** 10/10.
**Recommended fix:** Ship minimal `wp/dailyos/blocks/*/render-functions.php` stubs invoking each producer (account/project/person detail blocks for envelope + touchpoints; meeting-briefing block for prep status; daily-briefing block for `get_daily_briefing`; actions-work block for claim_feedback + boundary; activity-log/lint blocks for privacy projection). Alternatively per `feedback_pick_more_complete_option_over_simpler`: defer to a W1 stage 1d follow-up PR explicitly amending the packet — but the V1.1 fold explicitly elevated this to AC status, so deferral re-opens the L0 deliberation.

### Finding 3 — HIGH — AC-340.2 / AC-340.7 / AC-477.11 — CI lint scripts not wired

**Sub-ticket:** DOS-340 §5.8 + DOS-341 §5.9 + DOS-477 §5.4
**AC cite:** AC-340.2 — "CI lint script `check_audit_disclosure_allowlist.sh` committed + green"; AC-340.7 — "CI lint `check_audit_denylist_completeness.sh` enforces"; AC-477.11 — references `check_sensitivity_gate_composition.sh` "by path."
**File:line:** Scripts exist at `src-tauri/scripts/check_audit_disclosure_allowlist.sh`, `:check_audit_denylist_completeness.sh`, `:check_sensitivity_gate_composition.sh`. Verified zero references from `.github/workflows/` or `.githooks/`. Precedent `check_ability_surface_drift.sh` IS wired at `lint-frontend.yml:114`.
**Confidence:** 10/10.
**Recommended fix:** Append to the "Enforce ability surface drift guard" step at `.github/workflows/lint-frontend.yml:112-119`:
```yaml
bash src-tauri/scripts/check_audit_disclosure_allowlist.sh
bash src-tauri/scripts/check_audit_denylist_completeness.sh
bash src-tauri/scripts/check_sensitivity_gate_composition.sh
```
Without CI wiring, the lints exist but cannot enforce — "committed + green" is undefined when nothing invokes them.

### Finding 4 — MEDIUM — DOS-8 / AC-477.2 / path-α — Tauri command synthesizes envelope from request target

**Sub-ticket:** DOS-8 §5.7 + DOS-477 §5.4 cross-coupling
**AC cite:** AC-477.2 — "Mutations… MUST call [validate_envelope_target] BEFORE routing through services::claims::*"; AC-477.13 — "envelope-set (parent + transitively composed child envelopes)."
**File:line:** `src-tauri/src/commands/claim_feedback.rs:89-98` constructs `SingleClaimEnvelope` containing only the request's claim_id, then validates the request's target against it. The binding check is trivially satisfied — defeats the substrate-level guarantee at this surface.
**Confidence:** 9/10.
**Recommended fix:** Path-α (W2 entity-detail block will pass the real `EntityIntelligenceEnvelope` from `get_entity_intelligence`). Code comment at `:83-88` acknowledges this as a v1.4.4 W1 stub. Mitigating controls (sensitivity gate, agent-deny, metadata schema, source-content-hash) DO still apply. Worth filing as a maintenance ticket so the placeholder doesn't outlive its caveat; not a wave-1 blocker because the substrate helper at `entity_intelligence/auth.rs:170` IS correctly composed when called with a real envelope.

### Finding 5 — MEDIUM — DOS-340 — script bash structure (two `set -euo pipefail`)

**Sub-ticket:** DOS-340 §5.8
**AC cite:** path-α — script quality, not AC violation.
**File:line:** `src-tauri/scripts/check_audit_disclosure_allowlist.sh:28` AND `:113`. Two scripts concatenated; second `set` line is harmless but suggests merge artifact. Also `:115` uses `REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"` which is brittle vs `git rev-parse --show-toplevel`.
**Confidence:** 8/10.
**Recommended fix:** Path-α maintenance. Collapse to a single setup block; resolve REPO_ROOT via `git rev-parse --show-toplevel`. Lint behaviour is correct today.

### Finding 6 — LOW — DOS-340 — `audit_tables` regex drift between two scripts

**Sub-ticket:** DOS-340 §5.8 + AC-340.7
**AC cite:** path-α.
**File:line:** `check_audit_disclosure_allowlist.sh:40` lists 5 tables including `legacy_user_note_migration_audit`; `check_audit_denylist_completeness.sh:37` lists 4. Two scripts that should share a source-of-truth diverge silently.
**Confidence:** 7/10.
**Recommended fix:** Path-α maintenance. Extract `audit_tables` to a shared sourced file (e.g., `src-tauri/scripts/_audit_tables.sh`) and `source` it from both lints.

### Finding 7 — LOW — DOS-335 / AC-335.5 — `enqueue_refresh` does not enqueue

**Sub-ticket:** DOS-335 §5.5
**AC cite:** AC-335.5 — "Manual entity linking… causes prep status to recompute or enqueue refresh through services/abilities (not UI-local state)"; AC-335.15 — signal emit within one invalidation cycle. Both met partially.
**File:line:** `src-tauri/src/services/meeting_prep_status/write.rs:57-64`. `enqueue_refresh` only validates the `from → Queued` transition and emits the signal; no actual queue push (no `INSERT INTO meeting_prep_queue` or similar). Code comment at `:46-50` explicitly defers the queue plumbing to "Stage 1c when the meeting prep queue read API stabilizes." The signal emit DOES satisfy AC-335.15 (literal text: "dismissal write emits signal within 1 invalidation cycle").
**Confidence:** 8/10.
**Recommended fix:** Path-α convergence — the substrate is shaped correctly; the queue-write follow-through is the obvious downstream wiring. File a maintenance ticket so callers don't assume `enqueue_refresh` is end-to-end. NOT a wave-1 blocker because the L0 packet §5.5 read/write split and AC-335.15 signal pre-declaration ARE met.

### Finding 8 — LOW — DOS-477 — duplicate `RECEIPT_ALLOWED_FIELDS` naming clash

**Sub-ticket:** DOS-477 §5.4 + DOS-340 §5.8
**AC cite:** path-α naming clarity.
**File:line:** `src-tauri/src/services/entity_intelligence/auth.rs:455` AND `src-tauri/src/services/claim_receipt/boundary.rs:52`. Both consts named `RECEIPT_ALLOWED_FIELDS` but operate on DIFFERENT field-name spaces (top-level `ClaimReceipt` JSON keys: `target/surfaceContext/renderedText/...` vs audit-row column names: `source_label/source_type/source_asof/...`). auth.rs comment at `:451-454` acknowledges the duplication but the consequence is two unrelated lists with the same name — invites future confusion.
**Confidence:** 7/10.
**Recommended fix:** Path-α maintenance. Rename one — e.g., `auth.rs::RECEIPT_TOP_LEVEL_FIELDS` vs `boundary.rs::RECEIPT_AUDIT_PROJECTION_FIELDS` — and update the cross-script lint to compare them per dimension.

### Finding 9 — Informational — DOS-507 / AC-507.1 — `mcp_exposure` literal

**Sub-ticket:** DOS-507 §5.10
**AC cite:** AC-507.1 — "Ability registered at registry with… `NotExposed` MCP exposure."
**File:line:** `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:37` says `mcp_exposure = None`. `registry.rs:521` defines `McpExposure::None` as "Hidden from every MCP enumeration surface" with `#[default]`. Functionally equivalent — packet "NotExposed" was prose, not the literal enum variant name.
**Confidence:** 10/10. **Not a finding.** Noted for the next packet authoring pass to align terminology.

### Finding 10 — Informational — AgentMcp scrub leaves empty JSON string

**Sub-ticket:** DOS-341 §5.9 / AC-341.12
**File:line:** `src-tauri/src/services/claim_receipt/privacy.rs:478-508`. `scrub_target_for_agent_mcp` replaces `claim_id` and `subject_id` with `String::new()`. Serializes as `claim_id: ""` and `subject: {"account": ""}` — the JSON key is present even though the value carries no info.
**Confidence:** 8/10.
**Recommended fix:** Path-α — consider `Option<String>` + `skip_serializing_if = "Option::is_none"` for AgentMcp shape so the key is absent entirely. The leak risk is low (empty strings carry no identifier bits) but it surfaces the scrubbed-field set to consumers, which could confuse downstream MCP renderers about whether the field is "unknown" vs "redacted."

---

## ACs verified met

- AC-335.12 / AC-507.7 — read-module + briefing-producer call-graph fences are `#[test]` grep guards over the file source. Banned patterns include `enqueue_*`, `INSERT INTO`, `signals::bus::emit`. Passing.
- AC-335.13 Part A — disjoint-column commutativity property test at `mod.rs:417` exercises the user_authored × refresh ordering.
- AC-335.14 — exhaustive `legal_transitions` table in `mod.rs:108`, with `every_variant_has_legal_transitions_entry` test.
- AC-335.15 — `MeetingPrepStatusChanged` signal pre-declared in `policy_registry.rs` with coalesced-async policy.
- AC-340.1 — `RECEIPT_ALLOWED_FIELDS` + `AUDIT_ONLY_DENYLIST` encoded in `boundary.rs:52,70`; fail-loud panic on unknown fields at `:177`.
- AC-340.3 — fixture matrix at `tests/dos340_receipt_boundary_snapshots.rs` (124 LOC) + JSON snapshots per (surface × sensitivity).
- AC-341.10 — `build_receipt_for_audience` is construction-time, per-audience builders zero forbidden fields rather than post-render filtering.
- AC-341.11 — `OperationalAuditStorage` returns `PrivacyError::NonDisclosureAudience`; lint script enforces no `maintenance_audit` reads from `services::claim_receipt::*`.
- AC-341.12 — `build_agent_mcp` at `privacy.rs:307` zeroes `sources: Vec::new()`, `source_asof: None`, scrubs claim_id + subject_id (to type), preserves subject_type. Coarsened freshness is computed.
- AC-477.2 / AC-477.13 — `validate_envelope_target` + `composes_set_for` at `entity_intelligence/auth.rs:170,228` walk the transitive composes graph.
- AC-477.11 — `redact_provenance_for_surface` composes the shipped `render_policy_for_surface`; `check_sensitivity_gate_composition.sh` exists.
- AC-477.12 — `filter_for_receipt` at `entity_intelligence/auth.rs:469` is fail-loud allowlist.
- AC-461.6a / 6b — `tests/entity_fixture_harness/assertions.rs:31` bypass denylist + binding check + stale-vs-bypass counter, with the legacy producer identifiers from §5.3.
- AC-8.2 / AC-8.10 / AC-8.11 / AC-8.13 / AC-8.14 — caller-supplied idempotency key rejected at `feedback.rs:241`; per-action allowed-key validation at `:639`; source_content_hash constant-time-compared at `:286-305`; Agent denied at `:266`; 60s TTL idempotency cache at `:153-220`.
- AC-507.1 / AC-507.2 — Read category, User-only allowed_actors, `NotExposed` MCP, no provider synthesis in producer.
- AC-507.4 — composed `BriefingState` struct (NOT flat enum) at `contracts.rs` + state-matrix fixtures at `producer.rs:842`.
- AC-W1.4 — `cargo clippy -- -D warnings` clean.

---

## Recommendation

CONDITIONAL APPROVE: address Findings 1–3 (literal AC violations) before merging to `dev`. Findings 4–10 are path-α and route to the maintenance project — they do not block the substrate PR per `feedback_l2_path_alpha_to_maintenance_project`.

If the orchestrator opts to amend AC-W1.2 + AC-W1.9 (defer consumer-skeleton wiring to a stage-1d follow-up PR) AND wires the three lint scripts in a small precursor commit, the wave clears L2 fully.
