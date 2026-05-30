# L0 Correctness Review — v1.4.4 W1 Substrate Gaps (Cycle 1)

**Reviewer:** ce-correctness-reviewer
**Packet:** `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` V1.0
**Date:** 2026-05-20

---

## VERDICT: CONDITIONAL APPROVE

Substrate scope is grounded, K-in is real, and the existing DOS-701 floor (`contracts.rs` 175 LOC + `render.rs` 290 LOC + `auth.rs` 319 LOC verified at scan time) lines up with the placeholders the packet proposes to fill. The shapes are sound. What's missing is **testability precision** in 8 spots where ACs currently underspecify what "tested" means concretely enough to write a red test against. Folding the conditions below (most are single-bullet AC additions) unblocks L1; none are architectural reroutes.

Path: **fold conditions into V1.1, no cycle-2 dispatch**. All conditions are corrections inside the packet's current scope.

---

## Findings

### F1 — CRITICAL — §13 Q7 write-commutativity claim is wrong as stated (DOS-335)

**Claim under review (§13 Q7, line 1089):** "writes commute on disjoint columns — user_authored never touches status-derived columns; refresh never touches user_authored columns. Migration v251 separates dismissal persistence; v250 indexed view auto-refreshes."

**Trace:**
- `compute_status` (read) reads from the indexed view at v250 which joins `meetings + meeting_entity_links + meeting_prep_outputs + claim_invalidation_queue`.
- `enqueue_refresh(reason)` writes to the prep queue / `claim_invalidation_queue` and (eventually) updates `meeting_prep_outputs` + `last_prepared_at`.
- `record_user_authored(fields)` writes `agenda`, `notes`, `preparation_text`, `hidden_attendees`, and **emits claims for `decisions`** (per §5.5 Intelligence Loop check item 1: "emits claims for `decisions` (using existing `ClaimType::Decision` if present; else routes to commitment claim per DOS-129)").

The bug: `record_user_authored` writes **claims** (Decision/Commitment claim_type). Those claims feed back into the v250 indexed view (it joins `claim_invalidation_queue`, and decisions are claim-state-tracked). So user-authored writes DO touch status-derived columns transitively — through the claim store, not directly. A user authoring a decision while a refresh runs can race the refresh's view-recompute and produce a status snapshot where the decision is half-counted (claim row inserted, view materialization predates the insert, status returned says `Ready` but `decisions` count is stale by one).

The simpler interleave: `record_user_authored` writes notes (column A), `enqueue_refresh` updates `last_prepared_at` + `source_asof_inputs` (column B). These DO commute on the row. But the moment `decisions` enters the picture, the disjoint-column premise breaks.

**Recommended fix:** Replace §13 Q7's recommendation with two-part contract:
1. **Plain user-authored fields (agenda/notes/preparation_text/hidden_attendees)** commute with `enqueue_refresh` writes — disjoint columns, no derivation. Property-test at L1.
2. **Decision/commitment claim emission** is a claim-store write, NOT a disjoint-column write. Status recompute observes the new claim through the standard claim-lifecycle-signal → invalidation path. The contract is "eventual consistency via existing signal substrate," not "commutes."

Add to AC-335.11 a test case: `decision_authored_during_refresh_eventually_visible_in_next_status_recompute_without_lost_update`.

**Severity rationale:** This is the single substrate claim the packet makes about concurrent state that doesn't hold under realistic traffic, and §13 Q7 is exactly where reviewers are asked to verify the claim. Flagging now prevents an L1 property test being written against a false premise. Trace confidence ~80 (depends on whether `record_user_authored` actually emits claims at L1; §5.5 Intelligence Loop item 1 says it does).

---

### F2 — HIGH — DOS-461 no-bypass assertion has two enforceable edge cases the packet doesn't enumerate

**Section:** §5.3 AC-461.6/7 + §13 Q8 recommendation (line 1091).

**Recommended assertion in §13 Q8:** "every rendered claim-row in the snapshot DOM carries a `data-claim-id` (or `data-proposal-id`) attribute that resolves to an envelope item; rows without resolution are bypass."

**Enforceability trace:**
- Static check: grep block render PHP for non-envelope intelligence calls — **enforceable, no edge case**.
- Runtime snapshot diff: render the block, parse DOM, for every `[data-claim-id]` look up the envelope. Two edge cases slip through:

**Edge case A — claims rendered as descendant text without a wrapper.** A block render that interpolates `<span>{$claim_text}</span>` without `data-claim-id` on the span is invisible to the harness. The harness would report "0 claim rows, 0 bypass" — false-clean. The envelope text is rendered, but the binding is lost mid-projection.

**Edge case B — claim_id present but stale.** If block render PHP caches a prior envelope's DOM and the cached `data-claim-id` no longer resolves to the live envelope (claim was retracted/superseded between renders), the harness flags "unresolved data-claim-id = bypass" — but the actual problem is stale render, not bypass. The harness should distinguish.

**Recommended fix to AC-461.6:**
- Add to AC-461.6: "Snapshot includes a **text-extraction harness pass** — every visible claim-substantive string in the rendered DOM (text that originates from `IntelligenceClaim.text` / `RenderableClaimText.body` per the envelope) must trace back to a `[data-claim-id]` ancestor. Text-without-binding fails."
- Add a separate AC-461.6b: "Unresolved `data-claim-id` (claim no longer in envelope) renders as **stale, not bypass** — distinguished failure mode with its own assertion + fixture (`account_claim_retracted_mid_render.json`)."

**Severity rationale:** Without these the no-bypass harness has gaps that let two real regression classes through. Trace confidence ~75 (the rendering paths exist as patterns in `wp/dailyos/blocks/account-overview/view.js:503` which already uses `data-claim-id` extraction — the harness must close the gap).

---

### F3 — HIGH — DOS-507 BriefingState taxonomy is incomplete for the empty + stale + partial intersection

**Section:** §5.10 `BriefingState` enum (lines 845-855) + AC-507.4.

**Trace through realistic states:**
- Day has 0 meetings → `Empty { reason: EmptyReason }` ✓
- Day has 2 meetings, both prep-ready → `Full` ✓
- Day has 2 meetings, one prep-ready + one needs-preparation → ??? — does `NeedsPreparation { meeting_ids: [m2] }` apply (the **whole** briefing is in that state) OR is the briefing `Full` with one meeting carrying its own `PrepStatus::PrepNeeded`?
- Day has 2 meetings, one ready + one stale claim referenced → both `Stale` AND `Full`?
- Day has 3 meetings, prep is ready for all 3 but **one references a corrected-superseded claim** → `Corrected { superseded_claim_ids: [...] }` overrides `Full`?

The enum is mutually-exclusive variant-shaped, but the real states are **multi-dimensional** (a briefing can be simultaneously "full" + "has-stale-references" + "has-corrected-references"). The flat enum collapses these and forces a precedence the packet doesn't define.

**Recommended fix:** Either (a) split `BriefingState` into a composed `{ availability: Available|Empty|AuthLocked, freshness: Fresh|Stale|NeedsPreparation, integrity: Clean|HasCorrections|HasAmbiguity, advisories: Vec<BriefingAdvisory> }` so partial states don't collide, OR (b) keep flat but define explicit precedence (`AuthLocked > Empty > NeedsPreparation > Stale > Corrected > Ambiguous > Full`) and add AC-507.4b: "test the matrix of all 2-state intersections; assert correct precedence."

The composed shape (option a) is correctness-preferred; the flat shape (option b) is acceptable if precedence is enforced and tested.

**Severity rationale:** Without resolution, downstream W3 block render has no contract for "briefing is mostly fine but one meeting needs prep" — the most common real state. Trace confidence ~75.

---

### F4 — HIGH — Cursor invalidation under concurrent writes is unspecified (§13 Q11)

**Section:** §13 Q11 (line 1095). "opaque server-signed cursor — survives schema changes, prevents cursor tampering."

**Trace:**
- Page 1 returns 10 facts + `next_cursor: sig(last_id=f10, last_sort_key=t10)`.
- Between page 1 and page 2, a new claim `f5b` is inserted (subject re-classified into this entity's set) AND `f7` is retracted.
- Page 2 requested with the cursor. Server validates signature. The cursor encodes `(last_id, last_sort_key)`. Does page 2 return:
  - (a) facts strictly after the cursor — but `f5b` (inserted before f10, would have appeared on page 1) is now skipped forever, AND `f7` is gone so the result set the cursor was anchored to no longer exists exactly;
  - (b) re-anchor on cursor's `(claim_id=f10)` — but f10 may have been retracted too;
  - (c) return a typed `CursorInvalidated { reason }` and force the caller to restart from page 1.

The packet says "opaque server-signed cursor" but doesn't say which of (a)/(b)/(c) the server actually does.

**Recommended fix:** Add AC under §5.1 (DOS-459) and §5.2 (DOS-460):
- "List-shape envelopes return cursor + typed `CursorState`: `Stable | DataShifted { advisory } | Invalidated { reason, restart_required: true }`."
- Concurrent insert/retract during pagination is tested: harness inserts a claim between page fetches, asserts behavior matches the declared `CursorState`.
- Document the trade-off in §13 Q11 recommendation: pick lost-update-tolerant (return stable cursor + accept skipped inserts as advisory) OR consistent-snapshot (invalidate cursor on any underlying shift, force restart).

**Severity rationale:** Cursor invalidation is a textbook off-by-one-across-pages bug class. Naming the policy at L0 prevents an L1 implementation that picks arbitrarily and a W2 consumer that assumes the other. Trace confidence ~75.

---

### F5 — HIGH — Privacy redaction matrix has an unresolved case for derived/composed claims (DOS-341)

**Section:** §5.9 Privacy matrix (lines 753-760) + AC-341.2.

**Trace:**
- A `Confidential` claim about Account A and an `Internal` claim about Account A are both inputs to an `EnvelopeFact` shown in Account Detail.
- The envelope's `EntityFact` carries a single `sensitivity: ClaimSensitivity` field (§5.1 line 179: "top-level minimum visibility"). For this case, top-level = `Confidential` (the more restrictive).
- A consumer renders the fact at `EntityDetail` surface. Privacy matrix says Product receipt may show "redacted evidence summary." But the *summary* is **derived** from BOTH source claims — one of which is Internal-visible. Does the redaction:
  - (a) drop the entire summary because one input is Confidential?
  - (b) emit only the Internal-derived portion?
  - (c) emit a redacted-but-presence-marked summary ("evidence withheld for confidential source")?

Same problem with **claim references** (`record_entries[].claim_id`, `metadata_proposals[].proposal_id` pointing to other claims): when fact A references claim B and B has higher sensitivity than A's surface allows, does the reference render as opaque ID, broken-reference, or get removed entirely?

**Recommended fix to AC-341.2:** Add explicit handling rules:
- **Mixed-sensitivity composed claims:** sensitivity is the maximum of all inputs; if max exceeds surface policy, the **composed claim is dropped**, not partially rendered (prevents inference leaks).
- **Cross-claim references:** if referenced claim's sensitivity exceeds surface policy, the reference renders as `redacted: true, label: "<source type> (redacted)"`, never as an opaque ID.
- **Derived claims (e.g., `health_story` interpretive rows):** carry an explicit `derived_from: Vec<ClaimId>` and the surface check is max-sensitivity-across-derivation-chain. Test fixture: `account_health_story_derived_from_confidential.json`.

Add AC-341.10: "Mixed-sensitivity + derived-claim + cross-reference redaction explicitly tested per matrix; no rule-by-omission."

**Severity rationale:** Without this, a low-sensitivity render path can leak the existence/inference of higher-sensitivity claims — the privacy matrix's whole purpose. Trace confidence ~80.

---

### F6 — MEDIUM — DOS-459 envelope error/empty/stale propagation is underspecified

**Section:** §5.1 + AC-459.2.

**Trace through realistic states:**
- AC-459.2 says "empty sections carry typed reasons (no `null` without reason)." Good.
- But the envelope has a top-level `EnvelopeTrustSummary` + `EnvelopeProvenance`. What happens when the entire underlying entity context fetch fails partway? Is it:
  - All sections `Empty { reason: PartialFailure }` with a top-level error attached?
  - The ability returns `AbilityResult::Err(...)` and surfaces handle it?
  - Some sections return `Present` (the ones that did fetch) and others `Empty { reason: NotFetched }` (the ones that didn't)?
- What about `facts: Vec<EntityFact>` where ONE fact's claim row is corrupt — partial envelope returned, or error-out?

**Recommended fix:** Add AC-459.9:
- "Partial-failure semantics specified: per-section composition is **independent** (one section failing emits `Empty { reason: PartialFailure }`, others continue). Errors that affect subject identity OR auth fail the entire envelope (`AbilityResult::Err`). Distinguish corrupted-claim-skip (warn + drop the fact + emit `Empty { reason: PartialFailure }` for that section) from total fetch failure."
- Add fixture: `account_partial_section_failure.json`.

**Severity rationale:** Without this, downstream W2 block render has no contract for "envelope returned but is partial" — the most common degraded state. Trace confidence ~70.

---

### F7 — MEDIUM — AC-461.5 fixture list mixes "must include" with "if supported"

**Section:** §5.3 AC-461.5 (line 342).

"Fixtures include stale fact, corrected-superseded claim, low-trust claim, **metadata proposal where supported**, open loop, upcoming touchpoint, recent touchpoint, thread summary, confidential/user-only claim, Glean citation, wrong-subject claim."

**Trace:** "where supported" makes this fixture conditional on subject type (Project doesn't have metadata proposals the same way Account does, for example). But the AC doesn't say which subjects support which fixtures — so the harness driver can't know whether to expect a metadata-proposal fixture for a Project or treat its absence as a missing fixture.

**Recommended fix:** Replace "where supported" with an explicit matrix in the packet:

```
                    Account  Project  Person
metadata_proposal     ✓        ✓        -
upcoming_touchpoint   ✓        ✓        ✓
recent_touchpoint     ✓        ✓        ✓
thread_summary        ✓        ✓        ✓
glean_citation        ✓        ✓        ✓
wrong_subject         ✓        ✓        ✓
ambiguous_association -        -        ✓   (Person-only per AC-461.4)
project_account_overlap ✓     ✓        -
parent_child          ✓        -        -   (Account-only)
```

Encode in the harness as a per-subject expected-fixture-set; absent expected fixture = harness fail.

**Severity rationale:** Without this, "comprehensive fixture coverage" is an unprovable AC. Trace confidence ~70.

---

### F8 — MEDIUM — DOS-8 server-issued idempotency_key contract has retry/window ambiguity

**Section:** §5.7 AC-8.2 + the contract sketch (lines 587-607).

**Claim:** `idempotency_key` is server-issued. Caller-supplied keys rejected.

**Trace:**
- Client submits feedback. Server mints key K, persists feedback row, returns `ClaimFeedbackResponse { idempotency_key: K, ... }`.
- Network error mid-response. Client never sees K.
- Client retries. Server has no caller-stable key to dedupe against — it would mint K' for the retry, producing a duplicate `claim_feedback` row.
- The standard pattern is: caller supplies a **request-id** (different from idempotency-key); server uses request-id to detect retries and returns the same idempotency-key + same response.

**Recommended fix:** Either (a) clarify §5.7 contract — caller supplies `request_id` (a UUID-shaped retry-dedup token, not an idempotency-key), server uses it for retry detection AND mints an `idempotency_key` separately, OR (b) document the retry semantics explicitly: "On retry without prior idempotency-key, server treats as new request; duplicate row possible — acceptable because `services::claims::record_claim_feedback` is idempotent over (claim_id, action, actor) per existing 9-variant test at `claims.rs:13283`."

Option (b) is acceptable if the existing writer's idempotency contract is documented and AC-8.2 is amended to test the retry case.

Add AC-8.9: "Retry-without-prior-key semantics tested: dropped-response retry produces at most one persisted feedback row per (claim, action, actor, timestamp window)."

**Severity rationale:** Without this, retries quietly double-count feedback, which the trust-scoring substrate consumes. Trace confidence ~65.

---

### F9 — LOW — DOS-340 audit-only denylist is a static const list, missing extensibility check

**Section:** §5.8 `AUDIT_ONLY_DENYLIST` constant (lines 678-697).

**Trace:** Hardcoded list. When a new audit field is added in v1.5.x, the denylist won't include it; receipts could expose the new field by default. The packet's recommended structural rule (§13 Q2) was correctly rejected ("enum-based + CI lint enforces; structural rule is a maintenance pattern, not a substrate pattern") — but the **CI lint** for adding-an-audit-field-without-denylist-update isn't specified.

**Recommended fix:** Add AC-340.7: "CI lint `check_audit_denylist_completeness.sh` enforces that every column added to the operational audit storage schema (any migration touching `audit_log` or `provenance_audit_storage`) explicitly adds an entry to `AUDIT_ONLY_DENYLIST` OR includes a comment justifying receipt-safety (`// receipt-safe: <reason>`)." Pattern modeled on existing `check_claim_writer_allowlist.sh`.

**Severity rationale:** Drift risk over future versions, not a current bug. Trace confidence ~60.

---

### F10 — LOW — DOS-335 PrepStatus state machine has unspecified transitions

**Section:** §5.5 `PrepStatus` enum (lines 442-453) + `next_allowed_transition: Vec<PrepStatus>` field.

**Trace:** The DTO carries `next_allowed_transition` but the packet doesn't enumerate the legal transitions. Realistic questions:
- Can `UserDismissed` → `Running` (user un-dismisses to re-trigger)?
- Can `Failed` → `Queued` (retry)?
- Can `BlockedNoEntity` → anything other than `PrepNeeded` (when entity gets linked)?

Without a transition table, the runtime check `if !current.next_allowed_transition.contains(&target) { ... }` is implementation-defined, not contract-defined.

**Recommended fix:** Add to §5.5 a `PrepStatus` state-machine diagram (text-form acceptable) OR a Rust function signature `pub fn legal_transitions(from: PrepStatus) -> &'static [PrepStatus]` referenced from the DTO. Add AC-335.12: "Transition table is exhaustive (every variant has an entry) and tested: illegal transitions return `TransitionError`, legal transitions succeed."

**Severity rationale:** State machine without enumerated transitions is a class of bugs (invalid states reachable) — but this is mitigable at L1 if the implementer notices. Trace confidence ~55.

---

## Residual Risks

- **R1 — Cursor in opaque-signed form requires a signing key.** §13 Q11 + the §5.1/§5.2 cursor decision implies a key-management decision (where the cursor-signing key lives, rotation policy, agent-vs-user split). This is out of W1's substrate scope but is a CSO consideration; flag to v1.4.6/v1.4.7 if cursor goes opaque-signed.
- **R2 — `meeting_prep_status_signals` view (v250) is indexed but the refresh policy isn't named.** If the view is a SQLite materialized-equivalent (trigger-driven) it's fine; if it's an inline computed query the "fast status compute" claim depends on query-plan stability. Verify at L1.
- **R3 — `get_entity_intelligence` envelope re-renders on existing signals (§5.1 IL check item 3) — no new signals.** If the existing signal substrate doesn't fire on `claim_state` transitions that go `Active → Contested` (which it should, but isn't verified in the packet), envelopes will silently go stale. Recommend AC-459.10: "Signal-driven envelope refresh tested: trigger every claim-lifecycle transition + verify envelope re-renders within signal propagation budget."

## Testing Gaps

- **G1 — No explicit property test specified for AC-W1.2's "wiring IS the work" vertical-slice gate.** The wave-rolled AC says every W1 producer must have a W2/W3/W4 consumer skeleton, but no L1 test enforces "producer-without-consumer fails CI." Add a CI gate: substrate-only PRs that don't reference at least one downstream consumer file are flagged.
- **G2 — TS mirror parity test pattern (AC-459.8) needs a referenced golden-file location.** Existing pattern at `src/services/claim-receipt/contracts.ts` should be cited as the template; otherwise each sub-ticket invents its own.
- **G3 — Trust-band recompute path under feedback (DOS-8 §5.7 IL check item 2) — "feedback updates lifecycle + verification state which feeds trust-band recompute via existing substrate" — the existing trust-recompute trigger isn't named.** Cite the substrate path (likely `services::trust::*` or signals on `verification_state` change). Without the path named, an L1 implementer may wire `record_claim_feedback` but miss the trust-band-refresh hookup, producing receipts where trust band is stale-by-one-feedback.

---

**End of Cycle 1 verdict.**
