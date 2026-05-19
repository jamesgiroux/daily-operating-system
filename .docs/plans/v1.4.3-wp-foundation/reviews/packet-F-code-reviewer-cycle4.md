# Packet F — code-reviewer cycle 4 (V1.3 lock verification, feasibility lens)

**Verdict:** APPROVE — contributes to lock.

**Summary:** V1.3 grounds the §5.4 pseudocode against the real `verify_and_consume` API at `surface_nonce.rs:640` returning `VerifiedNonce` at `:583`; the proposed extension (carrying `claim_id`, `action`, `payload_json`, `wp_user_id`, `session_id` into `VerifiedNonce`) is mechanically buildable because all four (claim_id, action, wp_user_id, session_id) already live on `PresenceNonceBindingFields` at `:482-494` and are accessible inside `verify_and_consume` before `binding.try_mark_consumed(now)` is called at `:725`. The cycle-3 FYIs (noun chain, `[Self; 9]` size, crate boundary) are captured verbatim in §16; the codex challenge HIGH #1 dissent is resolved (V1.3 no longer references a non-existent `try_mark_consumed(&request)` signature); the LOW #4 + #5 citation fixes (`NonceAuditContext` at `:1158`, `submit_feedback()` at `:149`) verify against the tree.

---

## 1. §16 L1 implementation notes — verification

All 8 items map to real L1 concerns; my 3 cycle-3 FYIs are captured.

| §16 # | Maps to | Verified? |
|---|---|---|
| 1 — noun chain (`SurfaceNonceStore` vs `SurfaceNonceService`) | My cycle-3 FYI #1 | YES — `surface_nonce.rs:248` confirms public `verify_nonce` delegates to `self.inner.store.verify_and_consume`; private store ownership is real. |
| 2 — `[Self; 9]` → `[Self; 10]` | My cycle-3 FYI #2 | YES — captures the fixed-size array adjustment without dictating refactor; preserves `len()` const-eval. |
| 3 — crate boundary `From<PresenceNonceAction> for FeedbackAction` | My cycle-3 FYI #3 | YES — captures the three options without committing; L1 picks minimum visibility change. |
| 4 — deployment ordering window for orphan retirement | /cso FYI | Real concern; correctly scoped as ops note for any future remote deploy. |
| 5 — phase-3 failure should charge `failure_budget_per_minute` | /cso FYI | Verified against `surface_nonce.rs:30-52, 273` — `charge_failure_best_effort` exists; phase-3 (record_claim_feedback failure post-consume) is genuinely not on a charge path today. |
| 6 — HKDF `info` distinct from `PRESENCE_NONCE_KEY_INFO` | security-auditor FYI | Real cryptographic-hygiene concern; correctly named as a single-constant decision. |
| 7 — `/nonce/verify` permission callback = literal `can_issue_presence_nonce` | security-auditor FYI | Real drift-prevention concern; aligned with §5.6 step 1. |
| 8 — WP `payload_json` reject non-plain-object | security-auditor FYI | Defensive measure; correctly scoped as L1. |

**No conflicts** between §16 notes and main packet text. Notes consistently use the FYI register: "L1 picks the smallest change", "L1 task", "L1 notes for ops" — not contradicting locked decisions.

## 2. V1.3 substrate changes implementability (`VerifiedNonce` extension)

**Question:** Is the V1.3 §5.4 step 1 extension (adding `claim_id`, `action`, `payload_json`, `wp_user_id`, `session_id` to `VerifiedNonce` at `surface_nonce.rs:583`) mechanically buildable inside `verify_and_consume` BEFORE `binding.try_mark_consumed(now)` at `:725`?

**Answer:** YES.

- `PresenceNonceBindingFields` at `:482-494` carries: `surface_client_id`, `session_id`, `wp_user_id`, `claim_id`, `field_path`, `action`, `claim_version`, `composition_id`, `composition_version`, `generated_at`, `expires_at`. Four of the five V1.3 additions (claim_id, action, wp_user_id, session_id) are direct field reads.
- `payload_json` is the §5.2 W4 addition to `PresenceNonceBindingFields`; once added (commit-2 scope per §10), it lives on the same struct and follows the same capture path.
- `verify_and_consume` body at `:649-734` already reads `binding.fields.claim_id` (`:687`) and `binding.fields.composition_id` (`:705`) for the version checks. Capturing into a return value is additive; no new lock acquisition, no new borrow shape.
- The Mutex is held for the full body via `inner.blocking_lock()` at `:649`; `binding` is a `&mut` reference to the entry under the lock. Cloning the binding's `String` / `Option<String>` / `u64` fields into a `VerifiedNonce` struct between the validity checks and the `try_mark_consumed` call at `:725` is straightforward — these are owned-clone operations, not lifetime-borrowing exotica.

**Caller impact:**

- `verify_and_consume` has exactly ONE caller in the tree: `SurfaceNonceService::verify_nonce` at `:248`. That call site at `:256-270` destructures `VerifiedNonce` into `SurfaceNonceVerify { consumed_at, request_id, expected_claim_version, expected_composition_version, audit_events }` at `:264`. **Growing `VerifiedNonce` is additive — the existing destructure keeps compiling as long as it uses named-field syntax** (which it does at `:265-268`). No existing caller breaks.
- The W4 W-side caller (the handler at `surface_runtime/mod.rs:2570/2577`) currently invokes `SurfaceNonceService::verify_nonce` returning `SurfaceNonceVerify`. The V1.3 §5.4 pseudocode at packet line 310 sketches it as `app_state.surface_nonce_store.verify_and_consume(...)` directly — which is a different noun chain than the existing handler. §16 note #1 acknowledges this: "L1 reaches it through `app_state.surface_nonce_store` or a service-level wrapper."
- **L1 implementer's decision tree** (captured by §16 #1 + the §5.4 pseudocode): either (a) handler bypasses `SurfaceNonceService::verify_nonce` and calls `SurfaceNonceStore::verify_and_consume` directly (requires `SurfaceNonceStore` to be reachable from `AppState` — currently it's `pub(crate)`-ish behind `SurfaceNonceService::inner.store`; L1 promotion needed), OR (b) `SurfaceNonceService::verify_nonce` is extended to thread the W4 fields out, which means `SurfaceNonceVerify` at `:1008` also grows. Both paths are L1-mechanical; neither needs a new architectural decision the plan should have made. The plan's §16 #1 note correctly identifies this as an L1 implementation choice, not a substrate gap.

## 3. Cycle-3 dissent verification (codex challenge HIGH #1)

V1.2 named `try_mark_consumed(&request)` — that signature does not exist; actual `try_mark_consumed` at `:518` takes `&mut self, now: DateTime<Utc>`. V1.3 grounds against `SurfaceNonceStore::verify_and_consume` at `:640` with signature `fn verify_and_consume(&self, ctx: &ServiceContext<'_>, db: &ActionDb, digest: NonceDigest, request: &VerifyNonceRequest, now: DateTime<Utc>, audit: NonceAuditContext) -> Result<VerifiedNonce, SurfaceNonceError>`. V1.3 pseudocode at packet line 310-312 matches this signature.

**End-to-end buildability check:**
1. `let verified = app_state.surface_nonce_store.verify_and_consume(ctx, db, digest, &request, now, audit)?;` — signature match, returns extended `VerifiedNonce`. ✓
2. `let action: FeedbackAction = verified.action.into();` — requires `From<PresenceNonceAction> for FeedbackAction`; §5.1 + §16 #3 own the impl + crate boundary. ✓
3. `actor: format!("user:wp:{}", verified.wp_user_id)` — V1.3 verified at `claims.rs:5121-5131`: `actor_class_for_actor` splits on `:`/`/`/`@`; head `"user"` is in the user allowlist. ✓
4. `record_claim_feedback(ctx, db, input)` — takes `&ActionDb`, opens own `with_claim_transaction:6717`. ✓ (db must be a write conn — §5.4 explicitly upgrades `db_read → db_write` at `surface_runtime/mod.rs:2570` in commit 3.)

No phantom APIs remain in V1.3 §5.4.

## 4. New file:line citation spot-checks

| Citation | Verified |
|---|---|
| `NonceAuditContext` at `surface_nonce.rs:1158` | YES — read confirms `struct NonceAuditContext` declared at `:1158`. `:1235` is `audit_event` (the V1.2 mis-cite). |
| `submit_feedback()` at `runtime-client.php:149` | YES — `public function submit_feedback(...): array|\WP_Error {` at `:149`. `:163` is the body line `return $this->signed_post( '/v1/surface/feedback', $body_bytes );`. V1.3 fix avoids L1 partial-deletion footgun. |

Both citations correct.

## 5. New feasibility concerns in V1.3 (not present in V1.2)

**One advisory observation, no findings:**

- **Advisory (confidence 50):** The §5.4 step-1 `VerifiedNonce` extension (5 new owned fields, mostly `String`) increases per-verify clone cost from ~32 bytes (current `VerifiedNonce`) to ~200+ bytes amortized. At the v1.4.3 personal-tier scale (single WP user, manual feedback affordance clicks) this is unmeasurable. §16 doesn't call it out and shouldn't — flagging it as a current-scale-irrelevant theoretical concern would route to the false-positive catalog. Surfacing as a 50-band observation only because the same fields also live on `PresenceNonceBindingFields` and the L1 implementer may want to consider returning a reference / `Arc` if profile shows hot-path pressure — but no baseline measurement suggests this matters. **Not a blocker; not a CONDITIONAL.**

No other feasibility concerns. V1.3 closes the V1.2 phantom-API gap; the substrate-grounding is tighter than V1.2.

---

**Lock contribution:** code-reviewer APPROVES V1.3 for lock.
