# Packet F — codex challenge — cycle 2

**Verdict:** CONDITIONAL APPROVE (3 surgical conditions; 0 BLOCK)

**Summary:** V1.1 lands the major folds — substrate reframe is real, the 7 cycle-1 findings are all named in the changelog and reflected in §3/§4/§5/§6/§9/§14. But two of those folds carry semantic drift between packet prose and the actual substrate (atomicity is misdescribed as a SQL-tx contract over what is in fact an in-memory Mutex; WP REST "extend" is in reality "register a new verify route"), and one prose claim in §5.3 was confirmed wrong on the previous cycle but is now restated as if confirmed. None of these is a class-pattern recurrence or a BLOCK — they are surgical and editable in V1.2 without changing the design center.

## Part 1 — cycle-1 findings, fold verification

| # | Cycle-1 finding | V1.1 fold | Verified | Notes |
|---|---|---|---|---|
| 1 | `surface_nonce.rs` already exists with 2-phase lifecycle + HKDF + rate limits + routes; V1.0 proposed a parallel substrate | §2 changelog calls it out explicitly; §3 substrate table enumerates the existing impl with file:line; §5 reframes the entire authoring scope as "extend existing" | YES | Real fold. Substrate-grep evidence in §4 is correct. |
| 2 | 4 of 9 ADR-0123 variants need `payload_json` (`WrongSource`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`) | §5.2 lists exactly those four as mandatory + `WrongSubject` as optional; §8.6 e2e uses `NeedsNuance` + `WrongSubject` with payload | YES | Variant set matches `validate_feedback_payload` at `claims.rs:5150`. |
| 3 | Replay atomicity unspecified | §5.4 specifies "UPDATE WHERE consumed_at IS NULL + rows_affected" + `with_transaction` block | PARTIAL — see Part 2 finding C1 | Prose is wrong about what the substrate is. |
| 4 | `wp_user_id` missing from schema | §5.3 says "already present per cycle-1 K-in"; §3 substrate table shows `wp_user_id: u64` in `PresenceNonceBindingFields:485` | YES — field exists at line 485 verified | But §5.3 line 173 cites `surface_pairing.rs:189` derivation, which I did not verify here. Plausible. |
| 5 | Presence-nonce precedent overclaimed (existing pattern derives identity, doesn't trust JS) | §5.3 explicitly states "never trust JS-supplied `wp_user_id`; always derive from the WP session and cross-check against the binding" | YES | Correctly walked back. |
| 6 | `FeedbackAction` enum lives in `abilities-runtime/src/abilities/feedback.rs:31`, not `claims.rs` | §3 substrate table line 73 cites the correct file:line; §6 decision #2 narrows the CI gate to both `claims.rs:6700+` (the function) AND `abilities-runtime/.../feedback.rs:31` (the enum) | YES | Correctly relocated. |
| 7 | Channel-list baseline 9 not 10 | §4 + §12 open question #1 — explicitly flags "cycle-2 verify by reading registry" | YES (now verified — see Part 2 finding C3) | I verified directly. |

**All 7 cycle-1 findings are folded.** None renamed-only.

## Part 2 — attempt to break V1.1

### C1 (HIGH) — §5.4 atomicity claim conflates DB-tx with in-memory-Mutex; pseudocode is wrong about the substrate

**Evidence:**

`SurfaceNonceStore` (`surface_nonce.rs:565-575`):

```rust
pub struct SurfaceNonceStore {
    inner: Mutex<SurfaceNonceStoreInner>,   // tokio::sync::Mutex
}

struct SurfaceNonceStoreInner {
    by_digest: HashMap<NonceDigest, PresenceNonceBinding>,
    by_composition: HashMap<String, HashSet<NonceDigest>>,
    lru: VecDeque<NonceDigest>,
}
```

`verify_and_consume` at `surface_nonce.rs:640-742` acquires `inner.blocking_lock()` and holds it through `try_mark_consumed`. The store is **not table-backed**; there is no `UPDATE ... WHERE consumed_at IS NULL` SQL semantics; there is no `with_transaction(db, ...)` over the nonce row. Existing atomicity is provided by the **exclusive Mutex critical section** — `try_mark_consumed` returns `Err(LifecycleRace::AlreadyConsumed)` when called twice (`surface_nonce.rs:519-520`), which is exactly the replay-rejection contract.

**The problem with V1.1's §5.4 pseudocode:**

1. It writes `with_transaction(db, |tx| { mark_consumed_if_unclaimed(tx, &nonce.digest)? ...; record_claim_feedback(ctx, tx, input) })` as if mark-consumed-and-record-feedback are co-transactional rows in the SAME DB tx. They are not — the consume mark is an in-memory state mutation under the `SurfaceNonceStore` Mutex; `record_claim_feedback` does its own DB writes (with its own `MutationGuard::reserve`). These are two different consistency domains.

2. The actual atomicity question that needs answering: **what happens if `record_claim_feedback` fails AFTER `try_mark_consumed` has succeeded?** The Mutex is still released, the in-memory state says "consumed", but no `claim_feedback` row was written. The nonce cannot be retried (replay-rejected); the user click is lost without a feedback row. The packet's §5.4 prose claims this case is covered by the tx; it isn't, because the consume isn't IN the tx.

3. The reverse case: **what happens if the tokio task is cancelled or the process crashes after `record_claim_feedback` commits but before the Mutex releases the consume mark?** Mutex is in-memory only — on restart, the in-memory store is empty, so the nonce digest is gone entirely (TTL is moot; the store is wiped). The `claim_feedback` row exists but the nonce is gone. This is benign (the row is durable, the nonce window already closed) but the packet doesn't say so.

4. The deeper question: **`record_claim_feedback` should run INSIDE the Mutex-protected critical section** so that a concurrent verify on the same digest is forced to wait — AND the consume-mark should be deferred until after `record_claim_feedback` commits successfully, so that a record-feedback failure leaves the nonce live for retry. This is "consume-after-commit" ordering, the opposite of the current `try_mark_consumed → return Ok` order at lines 725-741. V1.1's pseudocode is too sketchy to tell which order is intended.

**Condition:** V1.2 §5.4 must (a) acknowledge the store is `Mutex<HashMap>`, not DB-tx-backed; (b) specify the consume-vs-record-feedback ordering (consume-after-commit recommended for fail-closed semantics, with the proviso that a second concurrent verify must wait on the Mutex, not race); (c) explicitly cite `try_mark_consumed` at `surface_nonce.rs:518` as the existing primitive and explain how the new `record_claim_feedback` call interleaves with it; (d) replace the misleading `with_transaction(db, |tx| ...)` pseudocode with one that shows the Mutex critical section explicitly.

**Severity rationale (HIGH, anchor 75):** Packet text presents a contract — atomic single-transaction — that is technically false for the substrate as-shipped. Implementer following the pseudocode literally would either (i) write a non-existent `with_transaction` API call or (ii) hold the nonce-store Mutex across an arbitrarily long claims DB write. Both are real bugs. Folding this is one paragraph + one pseudocode-block rewrite.

### C2 (HIGH) — WP REST "extend existing" is in fact "register new `/verify` route"; §3 substrate table line 70 is overclaimed

**Evidence:**

`wp/dailyos/includes/class-dailyos-plugin.php:573-601`:

```php
register_rest_route( 'dailyos/v1', '/nonce', [
    'methods' => 'POST',
    'callback' => [ $this, 'issue_presence_nonce' ],
    'permission_callback' => [ $this, 'can_issue_presence_nonce' ],
] );
register_rest_route( 'dailyos/v1', '/account-overview/preview', [...] );
register_rest_route( 'dailyos/v1', '/account-overview/accounts', [...] );
```

**Only `/dailyos/v1/nonce` (issue) is registered on the WP REST surface.** No `/verify` route exists.

**The runtime-client transport** (`class-dailyos-runtime-client.php:188`) does call `/v1/surface/nonce/verify` against the **runtime sentinel**, but that is the runtime side, not the WP REST side. The packet §5.6 describes a JS-to-WP-to-runtime path: "JS POSTs `{ nonce_digest }` to `/verify`" — that POST has no WP REST landing zone today. The runtime-side `/v1/surface/nonce/verify` is signed-internal; JS cannot call it directly.

**Why this matters:** the packet treats §5.6 as "extend the request body". The actual work is:

- Register a brand-new `/dailyos/v1/nonce/verify` WP REST route with its own permission callback (presumably `can_issue_presence_nonce`, but that needs an audit — is the issue permission the right callback for verify?).
- Wire its handler to `DailyOS_Runtime_Client::verify_nonce()` (which exists).
- Decide the request body shape: is it `{ nonce_digest, payload_json? }` per §5.6, or is the payload-echo path actually unnecessary because the canonical payload lives in the binding (§5.6 says so explicitly)? If unnecessary, the `payload_json` parameter in §5.6 phase-2 should be deleted from the contract.

This is not a class-pattern recurrence with the V1.0 parallel-substrate problem — the runtime-side `/v1/surface/nonce/verify` exists, the transport-side `verify_nonce()` exists, the only gap is the **WP REST landing route**. But the packet's prose makes it sound like a field addition to a route that already accepts JS POSTs. It isn't.

**Condition:** V1.2 §3 substrate table line 70 to be revised — the WP REST scaffold provides issue only; verify needs a new route registration. §5.6 prose to be revised — phase-2 is a new route, not a body-shape extension. §6 decision #1 (services-only writes) is unaffected; this is REST-surface plumbing not write-path drift. §10 PR shape commit 4 ("WP REST endpoint extension") to be relabeled to include both "register new `/verify` route" and "extend `/nonce` body for `payload_json`".

**Severity rationale (HIGH, anchor 75):** Same shape as C1 — packet text describes a smaller surface than the actual work. Implementer who reads §5.6 literally will look for an existing `/verify` route and not find one, then either (i) ad-lib a new route registration without going through the decision sieve, or (ii) be blocked at L1 needing scope clarification. Folding is 2-3 sentence revisions across §3/§5.6/§10.

### C3 (LOW) — channel registry confirmed at 9; §12 open question #1 can be closed

**Evidence:**

`src-tauri/src/bridges/types.rs:84-126`:

```rust
#[non_exhaustive]
pub enum RenderPolicyChannel {
    Callouts, PrepOutputs, McpResponses, TauriRenders,
    SignalPayloads, Telemetry, EvalFixtures, Replay, ErrorLogs,
}
impl RenderPolicyChannel {
    const ALL: [Self; 9] = [...];
    pub const fn all() -> &'static [Self] { &Self::ALL }
}
```

**Exactly 9 channels.** Variants match the W6-E packet's canonical list (callouts, prep outputs, MCP responses, Tauri renders, signal payloads, telemetry, eval fixtures, replay, error logs) — `.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:252` and `:269`.

**WP block-render is NOT a distinct channel** — there is no `WpBlockRenders` variant, and v1.4.3 substrate work has not added one. WP block-render conceptually parallels `TauriRenders` (both are user-facing claim presentations), but the policy registry treats them under the existing `TauriRenders` semantics OR they leak through the projection layer that feeds both surfaces, which is a separate question.

**Recommendation for V1.2:** Close §12 open question #1 with the verified answer "9 channels confirmed; feedback-render is not a separate channel". Then make a separate decision: does W4's feedback render path need a 10th `WpBlockRenders` channel (or a renamed `RenderedClaims` that covers both Tauri and WP)? If yes, that is a substrate decision that probably belongs to v1.4.4 surface migration, not W4. If no, document why.

**Severity rationale (LOW, anchor 50):** This is a known open question that V1.1 already flagged. My verification just confirms which way it resolves. Doesn't block; just needs language adjustment.

### Out-of-scope probes I ran

**§5.1 reviewer call (extend `PresenceNonceAction` 4 → 9 vs mapping shim):** Reviewed for any case where mapping is strictly better. The extend path is correct. Mapping would require defining a 4-of-9 → 9 translation, but 4 of the existing 4 variants (Correct, Dismiss, Corroborate, Contradict) have NO clean semantic mapping to the 9 ADR-0123 variants (Correct → MarkOutdated? or NeedsNuance? Dismiss → CannotVerify? or NotRelevantHere?) — the mapping table would itself become a contested decision. Extend-to-9 is the right call. **Confirm.**

**§6 decision #2 (CI gate buildable as grep):** The gate is "PR must not edit the lines spanning `pub fn record_claim_feedback` through its closing brace at `claims.rs:6700+`" AND "must not edit `pub enum FeedbackAction` block at `abilities-runtime/.../feedback.rs:31`". A grep gate on the function signature is straightforward (`git diff --unified=0 dev..HEAD -- src-tauri/src/services/claims.rs | grep -E '^[+-].*record_claim_feedback'`), but a grep gate on the function **body** requires knowing the closing brace, which needs a balanced-brace scan — that is borderline-AST territory. For the enum, the same problem applies (the block boundary needs balanced-brace parsing). A pragmatic implementation: use `git diff` to extract changed line ranges in those files, then check whether any changed line falls within the function/enum line range (the line ranges are fixed in the gate's reference). That works with a `git diff` + line-range arithmetic, no AST tool needed. **Recommend V1.2 §6 #2 say so explicitly so the implementer doesn't reach for `tree-sitter` unnecessarily.** Severity LOW — already covered by §9.2 invariant; just an implementation hint.

**§5.6 contract-change for callers using nonce REST for presence-only:** The existing `/dailyos/v1/nonce` (issue) is called by JS for presence-only purposes today (W2 + W3 paths). V1.1 §5.6 adds optional `payload_json` to the issue request body — additive, no break. The new verify route is net-new — no break. Existing `submit_feedback()` transport at `runtime-client.php:149` posts a `presence_nonce` field as part of a feedback call to `/v1/feedback` (which is... a separate runtime endpoint? Or is `submit_feedback` legacy?) — packet should clarify but I see no contract-break risk for any existing JS caller from the changes in §5.6.

## Part 3 — verdict

**CONDITIONAL APPROVE.** Three surgical conditions, all editorial / scope-clarification:

1. **C1:** V1.2 §5.4 rewrites the atomicity contract to acknowledge `Mutex<HashMap>`, specify consume-vs-record-feedback ordering, and remove the misleading `with_transaction(db, |tx| ...)` pseudocode.
2. **C2:** V1.2 §3 substrate table line 70 + §5.6 + §10 commit 4 acknowledge that the WP REST `/verify` route is net-new (registration), not a body-shape extension.
3. **C3:** V1.2 §12 open question #1 closes with "9 channels confirmed; feedback-render is not a separate channel"; spawn a separate decision (likely v1.4.4) on whether a `WpBlockRenders` channel variant is needed.

No BLOCK. No class-pattern recurrence with cycle-1's K-in-miss. Cycle-1's seven findings all folded with substantive design changes, not renames. The two new findings (C1, C2) are about prose-vs-substrate drift in the SAME §5 sections that were rewritten — that drift is a natural artifact of large substrate-reframe rewrites, not a new class pattern.

Per `feedback_l0_partial_convergence_when_class_recurs.md`: no class recurrence here, so the standard CONDITIONAL APPROVE path applies — fold C1/C2/C3 into V1.2 and re-lock. If any reviewer in cycle 2 dissents on the unanimous-equivalent calculus, that is a signal worth investigating per `feedback_reviewer_dissent_is_signal.md`, but I do not have other cycle-2 verdicts in hand to compare.

**Top 3 findings**

1. **C1 (HIGH):** §5.4 atomicity pseudocode misdescribes the in-memory `Mutex<HashMap>` substrate as a SQL transaction; consume-vs-record-feedback ordering unspecified.
2. **C2 (HIGH):** §5.6 + §3 line 70 claim "extend existing WP REST `/verify` route" but no such route is registered today; this is net-new endpoint registration, not a body-shape extension.
3. **C3 (LOW):** §12 open question #1 — channel registry verified at 9 (`RenderPolicyChannel::all()`); close the question and spawn the WpBlockRenders decision separately.
