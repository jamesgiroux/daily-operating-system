# W1 wave L2 — code-reviewer verdict (cycle 1)

**Branch:** `wave/v1.4.4-w1-stage1a`
**Diff range:** `0f8533e1..HEAD` — 116 files, ~19,144 insertions across 10 sub-tickets
**Reviewer:** correctness / code-reviewer subagent (per engineering-ladder.md L2 matrix)
**Date:** 2026-05-20

---

## VERDICT: BLOCKED

Two HIGH findings violate explicit wave acceptance criteria. One is an architectural gap (privacy module not in the production receipt-render call chain) and one is a CI gate that was promised in the L0 cycle-1 fold and never landed. A third HIGH is a placebo envelope-set construction at the Tauri command boundary that defeats AC-477.2 from the only consumer that actually runs in v1.4.4 W1.

The V1.1 fold items from L0 cycle-1 (DOS-335 read/write split + AC-335.12 lint; BriefingState composed-struct; WrongSource content-hash; transitive composes walker; allowlist-primary boundary) all landed structurally — but the privacy gate is wired test-only, not in the production path.

---

## Findings

### F1 — HIGH (DOS-341) — `build_receipt_for_audience` is not invoked from the production receipt-render call path

**File:** `src-tauri/src/services/claim_receipt/render.rs:27-99` (the canonical `render_receipt_for`) vs. `src-tauri/src/services/claim_receipt/privacy.rs:170` (`build_receipt_for_audience`)

**AC violated:**
- **AC-341.1** — "Receipt rendering uses explicit field allowlist by audience/surface (encoded in privacy.rs)." Render path hard-codes fields directly into the `ClaimReceipt` struct without consulting `USER_TAURI_ALLOWED_FIELDS` / `AGENT_MCP_ALLOWED_FIELDS`.
- **AC-341.4** — "Tauri AND WP block render AND MCP render paths produce policy-compliant output." Production path never invokes audience selection. `SurfaceContext::Mcp` is mapped to `RenderSurface::McpTool` but `Audience::AgentMcp` is never reached.
- **AC-341.10** + **cycle-1 CSO F10 fold** — "`build_receipt_for_audience` is a render-time primitive (NOT a post-render transform); audience is an input to construction." Implementation is the latter — privacy lives in its own module called only by unit tests (`privacy.rs:770, 793, 829, 838, 858, 867, 876, 885, 908, 918`). `render_receipt_for` is not called with audience.
- **AC-341.12** — AgentMcp denylist (no source labels, no source_asof, no claim_id) cannot bind because the Mcp surface path through `render_receipt_for` populates `provenance.sources[0].label = "primary source"`, `provenance.sources[0].source_type = Some(claim.data_source)`, `provenance.sources[0].as_of = source_asof` (`render.rs:85-95`).

**Trace:** `submit_claim_feedback` → `maybe_render_receipt` → `render_receipt_for(state, target, surface)` → returns `ClaimReceipt` constructed from raw claim fields without `Audience` argument. The privacy module's allowlist constants are never consulted in production.

**Recommended fix:** Route `render_receipt_for` through `build_receipt_for_audience` with an `Audience` derived from `surface + actor`. Add an integration test that calls `render_receipt_for(..., SurfaceContext::Mcp)` and asserts the response has neither `source_type` populated nor a non-redacted source label.

---

### F2 — HIGH (AC-W1.9) — `scripts/check_w1_consumer_skeleton.sh` does not exist; no WP block render PHP consumes any W1 producer

**File:** `src-tauri/scripts/` (missing); `wp/dailyos/blocks/*/render-functions.php` (no consumer references found)

**AC violated:**
- **AC-W1.9** (verbatim): "`scripts/check_w1_consumer_skeleton.sh` (modeled on `check_claim_writer_allowlist.sh`): for each W1-shipped producer ... assert at least one block render PHP entry point under `wp/dailyos/blocks/**/render-functions.php` invokes it via the abilities runtime handle. Substrate-only PRs without at least one downstream consumer reference fail CI. The script IS the mechanical enforcement for AC-W1.2's 'wiring IS the work' obligation."
- **AC-W1.2** — "No W1 producer ships without at least one downstream consumer skeleton."

**Trace:** `grep -l "get_entity_intelligence\|get_daily_briefing\|meeting_prep_status\|claim_receipt\|touchpoints" wp/dailyos/blocks/*/render-functions.php` returns zero hits. `ls src-tauri/scripts/` shows no `check_w1_consumer_skeleton.sh`. The wave is substrate-only; the CI gate that was the fold of cycle-1 codex-challenge F1 is absent.

**Recommended fix:** Land the script (modeled on `check_claim_writer_allowlist.sh`) and add at least one minimal `wp/dailyos/blocks/<name>/render-functions.php` skeleton per W1 producer (e.g., `claim-receipt-skeleton`, `entity-intelligence-skeleton`) that calls the abilities runtime via `class-dailyos-runtime-client.php`. Wire the script into CI. If consumer skeletons cannot land in W1 alongside producers, file a wave-amendment to defer AC-W1.9 to W2 with explicit dependency declaration.

---

### F3 — HIGH (DOS-477) — Tauri command constructs envelope-set from the target itself, making `validate_envelope_target` a tautology

**File:** `src-tauri/src/commands/claim_feedback.rs:78-113` (`submit_claim_feedback_command`)

**AC violated:**
- **AC-477.2** — "Accept/dismiss/edit/correct actions validate target binding via `validate_envelope_target` BEFORE mutating metadata/trust/suppression/source reliability/repair state/feedback signals."

**Trace:** The command at line 89-98 extracts the claim id from the request target, inserts that single id into a `BTreeSet`, wraps it in a `SingleClaimEnvelope`, then calls `validate_envelope_target(set, request.target)`. The set is constructed to contain exactly the claim being mutated, so the check passes by construction (`feedback.rs:246` walks into `auth.rs:170` which compares the target's claim id against the set built from the target's claim id). The comment at line 83-88 acknowledges this is a placeholder pending W2 wiring.

This is the live v1.4.4 W1 production path — the Tauri command is what `useClaimReceiptSubscription` and downstream UI hooks invoke. Until W2 supplies a real envelope, the AC-477.2 binding contract is unenforced at the actual call site. Per memory `feedback_wire_existing_substrate_not_future_producer`: "wire existing substrate, don't ship empty-branch claiming 'producer is future work.'"

**Mitigating factor (not exonerating):** The sensitivity gate (`can_surface_for` at `feedback.rs:270`), Agent-actor denial (line 266-268), per-action metadata schema (line 273), and source-content-hash validation for WrongSource (line 290-305) all still bite at the Tauri boundary. The miss is specifically the envelope-set binding contract.

**Recommended fix:** Either (a) wire the command to call `get_entity_intelligence` to construct the real envelope-set before invoking `submit_claim_feedback` (the "wiring IS the work" path), or (b) explicitly file AC-477.2 enforcement as a W2 dependency and tighten the W1 AC to acknowledge that the binding check is satisfied at the substrate layer but not at the Tauri command surface in v1.4.4 W1. Option (b) requires a wave-amendment.

---

### F4 — MEDIUM (DOS-8 path-α) — WrongSource writer bridge mirrors content hash into legacy `source_ref` field; persisted JSON carries both keys

**File:** `src-tauri/src/services/claim_receipt/feedback.rs:307-320, 726-747` (`bridge_wrong_source_for_writer`)

**AC affected:** AC-8.11 is technically satisfied (hash is validated, `SourceNoLongerInClaim` on mismatch) but the persisted JSON carries the hash in BOTH `source_content_hash` AND a synthetic `source_ref` field. Future readers that consult `source_ref` see a hash-looking string where they previously expected an index. Documented as a follow-up in the inline comment ("Tighten the writer to consume the canonical field in a follow-up; the persisted JSON carries both").

**Path-α reason:** Not an AC violation, not a regression, not an ADR-named contract violation — the writer-tightening is a documented follow-up in code. File as maintenance.

**Recommended fix (path-α):** File maintenance ticket against `services::claims::record_claim_feedback` to consume `source_content_hash` directly and drop the synthetic `source_ref` mirror. Project ID: `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.

---

### F5 — LOW (DOS-335) — `transition_status` emits signal but does not persist a status row; relies entirely on view-derived state

**File:** `src-tauri/src/services/meeting_prep_status/write.rs:216-228`

**AC affected:** AC-335.14 (state machine) — `transition_status` validates the from→to legality and emits the signal, but does not persist the `to` state anywhere. The v241 view derives status from underlying `meeting_prep` columns + the `meeting_prep_status_dismissals` table. This is intentional per the read/write architecture, but it means `transition_status(meeting_id, Ready, Queued, db)` produces no persistent state change — the next `compute_status` call still derives whatever the view says.

**Path-α reason:** Pre-meditated design choice (the function is a signal-emit primitive, with persistence delegated to the actual queue / dismissal writers). The state-machine guard prevents emitting invalid signals. Not an AC violation; the L0 packet §5.5 read/write split deliberately routes persistence through purpose-built writers.

**Recommended fix (path-α):** Rename `transition_status` to `emit_transition_signal` to remove the implication that calling it transitions status. Or document explicitly in the rustdoc that "this function is a signal-emit primitive; persistence lives in `record_dismissal` / queue producers." Maintenance ticket.

---

### F6 — LOW (AC-W1.4) — `cargo clippy --workspace --all-targets -- -D warnings` is not green

**File:** `src-tauri/tests/{dos210_observability_span_fields_test.rs, dos259_lint_wiring_test.rs, dos567_fixture_scope_leak.rs, w5_c_detect_risk_shift_test.rs}` — 55 clippy errors, primarily `let_underscore_must_use` (48) and 7 others.

**Trace:** `git log 0f8533e1..HEAD --` shows no W1 commits touched these test files; the lint failures pre-date the wave (likely a rust toolchain bump or accumulated drift on dev).

**Path-α reason:** Pre-existing failures on dev branch, not introduced by W1. AC-W1.4 ("clippy green") is technically violated at integration time, but the wave did not regress it.

**Recommended fix (path-α):** File maintenance ticket for clippy cleanup on the named test files. Do not block W1 merge for this. AC-W1.4 should be re-checked at the wave integration tag.

---

## Items checked clean (V1.1 fold confirmation)

- ✓ **DOS-335 read/write module split + AC-335.12 lint.** `src-tauri/src/services/meeting_prep_status/{read.rs, write.rs, mod.rs}` — `read.rs:226-258` includes the `read_module_contains_no_mutations` test that searches for `&mut `, `enqueue_*`, `INSERT INTO`, `UPDATE`, `DELETE FROM`, signal emit. The fence runs on every `cargo test`. Read path uses `&ActionDb`, no writes.

- ✓ **DOS-507 BriefingState as composed struct.** `abilities-runtime/src/abilities/get_daily_briefing/contracts.rs:96-171` — `BriefingState { availability, freshness, integrity, advisories }` with `BriefingAvailability` / `BriefingFreshness` / `BriefingIntegrity` typed enums per the 4-tuple AC-507.4 fixture matrix.

- ✓ **DOS-8 WrongSource by content hash, not index.** `feedback.rs:286-305` validates supplied `source_content_hash` against `current_source_content_hash(&claim)` computed over `(data_source, source_ref, item_hash)` per ADR-0131. Constant-time comparison at `feedback.rs:786-795`. `SourceNoLongerInClaim` raised on mismatch.

- ✓ **DOS-477 `validate_envelope_target` walks composes graph.** `auth.rs:170-215` + `composes_set_for` graph walker at `auth.rs:228+` iterates the `AbilityRegistry`. Property test `ac_477_13_composes_set_property_test_against_real_registry` at `auth.rs:651+` asserts every (parent, child) in composes has the child's claims accepted by the parent's `EnvelopeSet`.

- ✓ **DOS-341 `build_receipt_for_audience` is a construction-time primitive.** `privacy.rs:170-248` — takes `Audience` as input, dispatches to per-audience builders (`build_user_tauri`, `build_agent_mcp`, `build_activity_log`, `build_lint`). `OperationalAuditStorage` rejected with `NonDisclosureAudience`. Per-audience allowlist constants at `privacy.rs:97-150`. **Caveat: see F1 — not wired into the production render path.**

- ✓ **DOS-340 receipt vs audit boundary CI lint.** `src-tauri/scripts/check_audit_disclosure_allowlist.sh` + `.test` companion + `check_audit_denylist_completeness.sh` (AC-340.7) all present.

- ✓ **DOS-460 canonical touchpoints + open-loops.** `src-tauri/src/services/entity_intelligence/touchpoints.rs` (497 LOC).

- ✓ **DOS-461 entity fixture harness.** `src-tauri/tests/entity_fixture_harness/` + 33 JSON fixtures spanning Account/Project/Person per the AC-461.5b matrix.

- ✓ **Sensitivity gate composition lint.** `src-tauri/scripts/check_sensitivity_gate_composition.sh` (AC-477.11).

---

## Bounding rationale

Per memory `feedback_l2_must_review_against_acceptance_criteria` + `feedback_l2_path_alpha_to_maintenance_project`:

- **F1, F2, F3** are explicit AC violations (AC-341.1/4/10/12, AC-W1.9 / AC-W1.2, AC-477.2). Wave-blocking.
- **F4, F5, F6** are path-α (documented follow-ups, pre-existing drift, design clarification). Maintenance project, not L2 blockers.

The wave should not merge until F1, F2, F3 are remediated or wave-amended. F4/F5/F6 should ship as separate maintenance tickets.

---

## Reviewer note

The substrate quality of this wave is high — the V1.1 fold items from L0 cycle-1 all landed structurally, the test harnesses are dense, the typed contracts (BriefingState composed struct, EnvelopeSet, FeedbackAction 9-variant taxonomy, PrepStatus state machine, audience-keyed receipt builders) read cleanly. The three HIGH findings cluster around a single theme: substrate is built, consumers are not wired. F1 fails to wire the privacy module into the renderer; F2 fails to wire any WP block to any producer; F3 fails to wire `get_entity_intelligence` into the Tauri command that depends on it. "Wiring IS the work" per AC-W1.1/W1.2 — and on these three axes the wiring is deferred.

Two viable paths to unblock:
1. **Wire it.** F1: route `render_receipt_for` through `build_receipt_for_audience` (a few hundred LOC). F2: land the script + one minimal WP skeleton per producer. F3: have the Tauri command call `get_entity_intelligence` before invoking `submit_claim_feedback`.
2. **Amend the AC.** File a wave-amendment that explicitly defers AC-341.4 (Mcp policy compliance), AC-W1.9 (consumer skeleton gate), and AC-477.2 (Tauri command binding) to W2 with named tickets. Pure substrate-only W1, with the wiring obligation moved.

Per memory `feedback_dont_swing_past_center_when_correcting`: the substrate isn't overengineered, it's correctly trimmed; the gap is consumer wiring. Don't strip the producers — wire them.
