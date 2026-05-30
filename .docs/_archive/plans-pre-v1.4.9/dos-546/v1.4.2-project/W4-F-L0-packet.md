# W4-F L0 packet — Local-to-local read path

**Current revision: V3.2 (post-cycle-3 trim — restore local-to-local elegance). See §2 Changelog.**

## 1. Header

Date: 2026-05-16
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: W4-F, stage-4 (closing Wave 4 threat-model gap)
Issue: DOS-655
Surface: Loopback HTTP runtime + session validation path
Primary path: `POST /v1/surface/project-composition` (and every other signed read route)
Primary code: `src-tauri/src/services/surface_pairing.rs::validate_signed_session`
Primary anchor: `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` §"Threat model: local-to-local"
Downstream dependencies: W5 feedback writes; W6 release-gate L4 proof

This packet captures the corrective work needed to make the W4 read path
honor the local-to-local threat model that v1.4.2 commits to. The block
render path (`POST /v1/surface/project-composition`) currently performs
two per-request UPDATEs on the SQLite writer mutex inside
`validate_signed_session`, contending with background workers and
producing silent `pairing_authority_unavailable` 503s under realistic
load. This blocked the W4-A L4 render proof across two sessions.

The fix is not a substrate-writer rearchitecture (that was W4-Sub V2 —
BLOCKED twice at L0 with 30+ findings). The fix is to stop applying
remote-model defenses to a local-to-local deployment: reads do not
write. That is the load-bearing change.

This packet is a closing W4 packet, not a new wave. It must merge
before W5 begins, because W5 introduces feedback writes whose write
semantics only make sense once read semantics are clean.

## 2. Changelog

- **V3.2 (2026-05-16):** Path-2 trim per James's pre-implementation check. Compared V3.1 against the v1.4.2 project description §"Threat model: local-to-local" and the original spike framing. Cycles 2-3 had re-added remote-model defense thinking piece-by-piece — each individual reviewer finding was justifiable but together they violated the threat-model contract that v1.4.2 explicitly states ("does NOT preemptively defend against threats outside its deployment shape"). V3.2 demotes the following back out of W4-F scope, filing as v1.4.3-federation maintenance tickets:
  - **Acceptance #21 (indistinguishable Err responses) — REMOVED.** Defended against credential-dimension fingerprinting by a same-UID local probe. But such a probe cannot construct valid signed requests (HMAC key in keychain, per-app ACL). Fingerprinting is informationally useless without ability to forge requests. The defense addresses a federation-shape threat (remote attacker who may eventually learn the HMAC key via lateral movement) that v1.4.2 does not commit to defending. File as v1.4.3-federation hardening.
  - **Fixture #25 (`dos655_err_responses_indistinguishable`) — REMOVED** alongside AC #21.
  - **Acceptance #22 (startup_id HMAC-validated rotation) — REMOVED.** The defense is belt-and-suspenders on top of existing HMAC validation. A same-UID attacker who replaces the sentinel and redirects WP to a fake port cannot produce valid HMAC responses (key in keychain). WP's existing HMAC-on-every-response validation correctly rejects the fake. The startup_id challenge adds nothing the HMAC doesn't already provide. File as v1.4.3-federation hardening if a remote-replay attack model becomes relevant.
  - **Fixture #26 (`dos655_startup_id_rotation_requires_hmac`) — REMOVED** alongside AC #22.
  - **Sentinel payload simplified:** the sentinel now carries `{port, runtime_version}` only. `startup_id` removed — was used only for the AC #22 rotation challenge that V3.2 removes.
  - **§9 CI gate — Layer 2 demoted to advisory.** Layer 1 runtime counter via `db_write_observer` is the mandatory gate; it directly measures writes on the Ok-return path and catches every drift case that matters (including off-thread writes via tokio::spawn). Layer 2 AST walker with N=3 transitive callees + route detection mechanism + closure traversal is demoted from MUST to SHOULD: a nice defense-in-depth but not load-bearing. Simplified to a focused AST check on `validate_signed_session_readonly` only (no transitive walk).
  - **Acceptance #11 (Keychain ACL) generalized.** Replaced "`kSecAttrAccessibleWhenUnlockedThisDeviceOnly` + `kSecAttrAccessControl` per-app ACL bound to DailyOS team ID" with "code-signing-bound app isolation; implementation picks the specific macOS Keychain mechanism. Negative fixture `dos655_keychain_isolation` is authoritative: a separate test binary signed by a different team ID cannot read the entry."
  - **§9.10 symbol-existence guard — KEPT** (cheap, no implementation cost).
  - **§9.11 `requires_write` exhaustive-match enforcement — KEPT** (free Rust hygiene, no implementation cost).
  - **§6.8b v179 rollback semantic note — KEPT** (documentation only, no implementation cost).
  - **§11 expanded** to list V3.2 trims as v1.4.3-federation maintenance candidates.
  - **§15 maintenance ticket count updated** to reflect V3.2 trims (was 6, now 6 + 2 v1.4.3-federation candidates = 8 maintenance tickets to file before L0 closes).

  V3.2 acceptance criteria count: 20 (down from 22). Fixtures: 25 (down from 27). The packet is back inside the local-to-local threat model it committed to. The CORE remains unchanged: dispatch refactor, failure-path writes preserved, sentinel TOCTOU hardening (minus startup_id), keychain persistence, migration v180 data-only, cache-miss latency budget, L4 proof. That CORE is the elegant answer to the spike question.

- **V3.1 (2026-05-16):** Cycle 3 packet-hygiene fix. Cycle 3 returned cso APPROVE (9/10), eng CONDITIONAL APPROVE (8/10), codex consult CONDITIONAL APPROVE (8/10), codex challenge BLOCK (7/10). All non-APPROVE reviewers converged on the same finding: V3 changelog updated correctly but §7 Acceptance #8 and §8 Fixture #22 carried stale V1/V2 prose contradicting V3's resolution. V3.1 surgical fixes:
  - **Acceptance #8 corrected:** removed `inactive_expires_at = NULL` backfill line (contradicted V3 NOT NULL preservation policy); changed `paired_at` → `issued_at` (actual v169 `surface_client_sessions` column name at `migrations/169_dos_559_surface_client_pairings.sql:75`; `created_at` is on `surface_client_pairings`, not sessions).
  - **Fixture #22 corrected:** same `paired_at` → `issued_at` rename + explicit "never NULL" assertion.
  - **Acceptance #21 + Fixture #25 clarified:** the 5 write-needing failure paths in current code (v179) return only 3 distinct enum variants (`SessionExpired`, `PairingExpired`, `SiteBindingMismatch` for both site_nonce and site_binding, `WpUserMismatch`). The W4-F refactor splits these into 5 explicit `SignedSessionFailure` variants (`SessionExpired`, `PairingExpired`, `SiteNonceMismatch`, `SiteBindingDigestMismatch`, `WpUserHashMismatch`). Fixture asserts the post-refactor 5 variants. Implementation note added to §5 and Acceptance #21.
  - **§12b Q11 corrected:** removed stale NULL-backfill reference; aligned with V3.1 policy.
  - **§9 Layer 2 AST gate hardened:** sub-clause added — walker traverses `tokio::spawn` / `task::spawn` closure bodies as if inline calls (catches off-thread write escape per codex challenge cycle 3 finding M2).
  - **§9 invariant #11 NEW** — `requires_write` enforcement: dispatch handler must use exhaustive match on `SignedSessionFailure` with `#[deny(non_exhaustive_omitted_patterns)]`, OR the enum exposes `fn writer(&self) -> Option<WriterFn>` so the compiler forces every variant to declare its write footprint. Prevents future variants from silently skipping security-load-bearing writes.
  - **§6 V179 rollback note NEW:** documented that rows whose `inactive_expires_at` is in the past at v180 apply-time will be rejected by v179 code on rollback (since v179 still consults the column). Mitigation: v180 rollback requires re-pair. Documented as known v179-rollback footgun, not a v180-forward blocker.
  - **§16 vs #22 contradiction (codex consult cycle 3):** Fixture #16 (sentinel substitution detected) and Acceptance #22 (startup_id rotation requires HMAC) align on error code: `session_requires_repair`. Fixture #16 wording adjusted to match.
  - **§15 maintenance ticket count:** updated to 6 (V3 demoted `is_safe_ability_name` adds back the original W4-Sub finding #4 ticket). §11 §15 references reconciled.

- **V3 (2026-05-16):** Cycle 2 L0 fold. 4× CONDITIONAL APPROVE (eng 8/10, codex challenge 7/10, codex consult 8/10, cso 8/10). One CRITICAL (codex) + 7 HIGH + 6 MEDIUM. Material changes:
  - **CRITICAL fix (codex):** v180 backfill of `inactive_expires_at = NULL` violates v169 `NOT NULL` constraint at `migrations/169_dos_559_surface_client_pairings.sql:77`. V3 changes strategy: existing `inactive_expires_at` values are LEFT UNCHANGED (forensic preservation). New session inserts continue to write `inactive_expires_at` (NOT NULL satisfied) but with the same value as `absolute_expires_at` — semantically "no longer authoritative for validity, retained for forensics." SQL comment in v180 + migrations.rs marks the column deprecated for validity-checking. The constraint is preserved; the semantics is deprecated.
  - **HIGH fix (eng):** V2 §5 backfill references `paired_at` column. The actual column is `created_at` per `migrations/169_dos_559_surface_client_pairings.sql:19`. V3 corrects all references (§2 V2 entry, §5, §6 V2 entry, Acceptance #8, fixture #22).
  - **HIGH fix (eng):** Readonly-split scope-recovery shape underspecified. Current dispatch at `mod.rs:759-779` invokes `validate_signed_session` + `load_session_scope_set_for_audit` inside the same `db_write` closure. V3 §5 specifies: `SignedSessionFailure` enum payload carries `granted_scopes: Option<ScopeSet>` populated by the readonly variant. The dispatch handler's Err arm constructs audit attribution from the enum payload without a second `db_read` round-trip. Alternative (also acceptable): a separate `db_read` for `load_session_scope_set_for_audit` before the Err-arm `db_write`. Implementation picks one.
  - **HIGH fix (codex):** `SignedSessionFailure` enum scope underspecified. The current `validate_signed_session` has ~10 Err return paths (UnknownRuntimeAnchor :724, RestoredStalePairing :733, PairingRevoked :736/:741, PairingSuspended :740, PairingExpired :742, SessionInvalid :743/:746, SessionThrottled :753, plus the 5 enumerated). V3 §5 specifies: the enum has variants for ALL ~10 returns, with a `requires_write: bool` discriminant or trait method that the dispatch handler matches on. Only 5 variants route through Err-arm `db_write`; the rest return directly.
  - **HIGH fix (cso):** Err response disclosure. The 5 failure variants currently can return different error codes, allowing attackers to fingerprint which credential dimension tripped. V3 adds Acceptance #21: ALL 5 Err variants return identical opaque `session_invalid` error code + identical HTTP status (403) to the WP client. Differentiated reasons appear ONLY in local audit log and `pairing.session.*` events, never in the HTTP response. Fixture `dos655_err_responses_indistinguishable` asserts byte-equal response bodies across all 5 variants.
  - **HIGH fix (cso):** startup_id refresh policy ambiguous. V3 adds Acceptance #22: startup_id refresh is gated on a successful HMAC-validated response from the *previously-known* endpoint. If WP cannot HMAC-validate against old startup_id, it triggers `session_requires_repair` (re-pair flow). An attacker who replaces the sentinel cannot rotate WP's expected startup_id without also producing a valid HMAC response from the old endpoint — which requires the keychain key.
  - **HIGH fix (cso):** CI gate coverage gaps. V3 §9 amendments:
    - Layer 2 AST walker walks transitive callees to depth N=3 (catches helper-write drift)
    - Layer 1 fixture adds a coverage assertion that every helper invoked on the Ok-path is exercised
    - §9.3 specifies the new-route detection mechanism: cargo test `gettrshape_routes_listed_in_get_shape_handlers` scans `surface_runtime/mod.rs` for `(Method::POST|GET, "/v1/surface/...")` patterns and asserts every match is in the `GET_SHAPE_HANDLERS` allowlist
    - New invariant #10: `validate_signed_session_readonly` symbol MUST exist at the dispatch site; grep fails CI if rename without binding update
  - **HIGH fix (consult):** Stale §5 "Migration story" subsection (V1-era prose contradicting V2's data-only framing) DELETED in V3.
  - **HIGH fix (consult):** Cache budget wording inconsistency. V2 changelog said "warm ≤ 500ms"; Acceptance #15 said "warm ≤ 200ms". V3 reconciles to **warm p95 ≤ 200ms** throughout.
  - **HIGH fix (codex):** `is_safe_ability_name` location wrong in packet. Actual location is `surface_runtime/mod.rs:2294`, only caller is `/v1/surface/invoke` at `mod.rs:1602`. V3 corrects the location reference.
  - **HIGH fix (eng + verified):** `is_safe_ability_name` DEMOTED back out of W4-F scope. Verified via wave plan §"Agent W5-A": W5-A feedback router posts to `/v1/surface/feedback`, NOT `/v1/surface/invoke`. The validator is not load-bearing for W5 kickoff. V3 removes Acceptance #16, fixture #24, the §5 validator block, and the §11 "promoted" note. Files as a fresh maintenance ticket in the `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` Maintenance project. W4-Sub finding #4 returns to its original maintenance status.
  - **MEDIUM filed to maintenance (cso M1):** Keychain reconciliation policy: V2's "revoke on missing keychain entry" is a local DoS vector. Replace with `pending_keychain_repair` lifecycle state requiring user confirmation. NOT folded in V3; filed as maintenance ticket. V3 keeps V2's revoke-on-missing as the v1.4.2 behavior with the trade-off documented.
  - **MEDIUM filed to maintenance (cso M2):** Quarantine-write latency timing channel. Same-UID attackers can already read the DB directly; local-to-local doesn't model side-channel attackers. V3 §6.9 documents this explicitly and files as maintenance.
  - **MEDIUM filed to maintenance (cso M4):** W5 silent-drop framing. V3 §10 already says "measure foreground-write latency"; the additional "feedback writes must surface loud errors on timeout, never silent drop" is W5's acceptance concern, not W4-F's. Filed for W5.
  - **MEDIUM filed to maintenance (eng M):** Acceptance #15 cache-miss test harness path. Implementation detail; harness lives at `tests/integration/dos655_cache_miss_latency.rs`. Filed.
  - **MEDIUM filed to maintenance (consult):** "GET-shape" → "read-shape" rename in §9 + §11. Mechanical cleanup; not blocking. Filed.
  - **MEDIUM closed:** Dot-traversal regression test (cso M3). Mooted by demoting `is_safe_ability_name` out of scope; the existing v1.4.x byte allow-list maintains current behavior.

- **V2 (2026-05-16):** Cycle 1 L0 fold. 1 BLOCK (codex challenge) + 3 CONDITIONAL APPROVE (eng, consult, cso). Material changes:
  - **CRITICAL fix (codex C1):** Audit missed the actual contention source. `validate_signed_session` is called inside `app_state.db_write(...)` at `surface_runtime/mod.rs:760` — the dispatch site itself takes the writer lane, regardless of SQL inside the callee. V2 relocates the dispatch to `db_read` and refactors `validate_signed_session` to take `&ActionDb` for read-only operations, escalating to `db_write` only on the Err-path inside the dispatch handler.
  - **CRITICAL fix (codex C2, eng C1, cso H1):** Audit missed 5+ conditional writes inside `validate_signed_session`. `mark_session_revoked` at `surface_pairing.rs:729`, `mark_pairing_expired` at `:756`, and `suspend_pairing` at `:760/:764/:771`. The three `suspend_pairing` writes are security-load-bearing (credential-mismatch defense). V2 explicitly preserves all five as failure-path writes routed via the dispatch handler's Err-arm `db_write`, not the success path. CI invariant #1 phrasing tightened to "zero writes on the Ok-return path" so failure-path writes pass.
  - **CRITICAL fix (codex C3):** `absolute_expires_at` already exists in v169 (`migrations/169_dos_559_surface_client_pairings.sql:77-78`). V1's proposed v180 `ADD COLUMN` is a no-op at best. V2's v180 is DATA-ONLY: validity-check policy change (remove `inactive_expires_at` consultation), backfill rule for existing rows where `absolute_expires_at` is in the past, deprecation comment on `inactive_expires_at`.
  - **CRITICAL fix (codex C4, consult R5, cso M4):** Sentinel file TOCTOU + auth-token disclosure threats unhandled. V2: parent dir `0700`; sentinel itself `0600`; `O_NOFOLLOW|O_EXCL` on write; ownership+mode check on every read; sentinel carries only `port` + `startup_id`, NEVER the HMAC session key; WP verifies `startup_id` matches a pairing-bound expected value before issuing a signed request.
  - **HIGH fix (eng H2, consult R3, cso H2):** Absolute-lifetime defense framing. V2 §6.3 explicitly states: the Exfiltration defense (Phase 0 artifact 01) is site-binding digest + nonce replay + user-initiated unpair, NOT absolute lifetime. Absolute lifetime remains at 365 days as user-configurable session hygiene, not as load-bearing security.
  - **HIGH fix (cso H3):** Keychain ACL underspecified. V2 §5 + Acceptance #8 require `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` + per-app ACL bound to DailyOS team ID. Negative fixture verifies a separate test binary cannot `SecItemCopyMatching` the entry.
  - **HIGH fix (codex H5, consult R4):** Cache-miss is NOT rare. In-memory DashMap is cold on every Tauri restart; first N requests after restart hit `commit_composition` writer-mutex path. V2 adds Acceptance #15: fresh render against James's prod DB on first-request-after-restart completes within p95 ≤ 1.0s (cold) and ≤ 500ms (warm). If this fails L4, file W4-Sub #14 (composition version-churn restructure) as a v1.4.x follow-up — but the latency MUST be measured.
  - **HIGH fix (codex H4):** `is_safe_ability_name` missing `/` is NOT latent for W5. WP transport sends `dailyos/account-overview`; if W5 feedback writes route through `/v1/surface/invoke` they fail at validation. V2 §5 adds the structural validator fix (split on `/`, reject empty segments, accept the one-slash form) inside W4-F scope, since it's load-bearing for W5 kickoff.
  - **HIGH fix (consult R6):** v180 schema default `'9999-12-31T23:59:59Z'` is unsafe (silent-permanent-session footgun on missed inserts). V2 keeps `absolute_expires_at` populated at pairing time (computed `created_at + configured_lifetime`); does NOT introduce a "never expires" sentinel default.
  - **HIGH fix (consult R2):** W5 feedback-write contention interlock. V2 §10 explicitly notes: feedback writes WILL contend with background workers when W5 starts. The mitigation isn't this packet's scope but W5's kickoff acceptance must include foreground-write latency measurement against James's prod DB; if it fails, ADR-0067 Stage 3 escalates to v1.4.3 critical path.
  - **HIGH fix (eng H1):** Acceptance §7.4 testability. V2 specifies the writer-mutex monitor mechanism: instrument `ActionDb::with_transaction` via the existing `db_write_observer` test harness (already used in W4-B's concurrency tests).
  - **MEDIUM fold (cso M5):** Migration v180 backfills `inactive_expires_at = NULL` on existing rows to signal "no longer maintained."
  - **MEDIUM fold (eng M2):** Sentinel-missing fallback: 3x retry at 100ms before falling back to stored pairing-marker option, per cycle 1 eng resolution of §12 Q5.
  - **§12 Open Questions:** All 8 resolved with cycle-1 decisions; see §12 for resolutions inline.
  - **CI mechanism:** Resolved to (d) runtime-counter fixture for Ok-path zero-writes assertion PLUS (c) narrow AST gate on the enumerated GET-shape handlers, per consult R7 + cso M1.

- **V1 (2026-05-16):** Initial L0 plan packet. Anchored to project
  description "Threat model: local-to-local" section added in the same
  session. Built from the W4-A read-path audit performed before
  drafting: every DB write on the GET path enumerated, classified as
  local-load-bearing or remote-model-defense, and the latter targeted
  for removal. Folds operational warts DOS-636 (stale pairing-marker
  port) and DOS-646 (HMAC session key not persisted across restart);
  these bite any render path and are scoped narrowly here. Explicitly
  excludes the 14 architectural findings surfaced by W4-Sub V2 — they
  remain real but become latent once the synchronous render path stops
  contending with the writer mutex; they are filed as substrate-quality
  backlog rather than blocking W4-F.

## 3. Status Snapshot

### What exists (V2 — corrected per cycle 1 audit)

- **Dispatch site (the actual contention source):**
  `surface_runtime/mod.rs:760` calls `app_state.db_write(move |db| validate_signed_session(...))`. The dispatch takes the SQLite writer lane regardless of what SQL the callee runs. This is the load-bearing contention point — V1 missed it; V2 fixes it.
- **`validate_signed_session` Ok-path writes** (success path, lines 774-798):
  - `UPDATE surface_client_sessions SET last_seen_at, inactive_expires_at` (:777)
  - `UPDATE surface_client_pairings SET last_used_at` (:788)
  Both inside `with_transaction`. V2 removes both; relocates dispatch to `db_read`.
- **`validate_signed_session` Err-path writes** (security-load-bearing, must be preserved):
  - `mark_session_revoked` at `:729` (session expiry detected)
  - `mark_pairing_expired` at `:756` (pairing expiry detected)
  - `suspend_pairing` at `:760` (site_nonce mismatch — Site-Switch defense)
  - `suspend_pairing` at `:764` (site_binding mismatch — Site-Switch defense)
  - `suspend_pairing` at `:771` (wp_user_hash mismatch — Exfiltration defense)
  Each enters `with_transaction`. These are auto-quarantine writes on credential-mismatch attacks and are required by Phase 0 artifact 01 defenses. V2 preserves them but routes them through the dispatch handler's Err-arm `db_write`, not the success path.
- **`absolute_expires_at` column already exists** in `surface_client_sessions` and `surface_client_pairings` per `migrations/169_dos_559_surface_client_pairings.sql:77-78`. Read by `surface_pairing.rs:92,482,581,591,617,727,1281,1606,1632`. v180 must NOT introduce this column; the delta is data-only.
- **`surface_client_bridge.authorize`** at `bridges/surface_client.rs:260`:
  in-memory rate-limit checks. NO DB writes. Good shape.
- **`emit_pairing_audit_event`** at `surface_runtime/mod.rs:2868`:
  JSONL append via in-process Mutex. NO DB writes. Good shape.
- **`CompositionRenderOrchestrator::cache_lookup` / `cache_store`** at
  `services/composition_render_orchestrator.rs:102`: in-memory DashMap. NO DB writes. **Cold on Tauri restart** — first N requests after restart take `commit_composition` writer-mutex path. See V2 Acceptance #15 (cache-miss latency measurement).
- **`commit_composition`** at `services/compositions.rs:67`: writes via composition transaction (writer mutex). Reached on cache miss only. Out of scope as a restructure, but cache-miss latency MUST be measured at L4 — see §11 for the deferral boundary.
- **`db_read` infrastructure** already exists and is used widely (mod.rs:1095, 1327, 1453, 1829, 2080, 2550). V2's dispatch-site refactor uses the existing primitive — no new infrastructure required.

### What's missing
- A read path that does not enter the writer-mutex-gated transaction.
- A session lifecycle that does not depend on per-request
  `last_seen_at` writes to stay valid.
- Persistence for the HMAC session key bytes across Tauri restarts
  (DOS-646). Currently in-memory only; every restart invalidates every
  WP session.
- A reliable mechanism for the WP plugin to discover the current Tauri
  loopback port across Tauri restarts (DOS-636). Currently stored in
  the `dailyos_pairing_marker` option which goes stale.
- A CI gate that prevents future read paths from regressing into
  writer-mutex contention.

### What's broken right now
- W4-A L4 render proof (DoD §2): block render reaches the runtime,
  signed session validates, writer mutex blocks behind background
  workers, the 5s pairing handshake timeout fires before the read
  completes, and the surface returns
  `pairing_authority_unavailable` 503.
- This is reproducible on James's production DB (327MB, active
  background workers). It is not reproducible on a clean dev DB —
  which is exactly why it slipped past initial development.

## 4. Pre-work — substrate reuse audit

### Existing primitives this packet consumes
- `surface_client_sessions` table — keep the row, keep `revoked_at`,
  drop the per-request `last_seen_at` write semantics.
- `surface_client_pairings` table — keep the row, drop the per-request
  `last_used_at` write semantics.
- `validate_signed_session` function — keep the read-validation path,
  remove the write block.
- The OS keychain wrapper in `services/keychain.rs` (existing) — reuse
  for persisting HMAC session key bytes per DOS-646.
- The existing pairing-marker mechanism in the WP plugin — extend with
  a sentinel-file lookup so the WP side can find the live runtime port
  after a Tauri restart (DOS-636).
- The composition cache in `composition_render_orchestrator.rs` —
  already in-memory, already correct shape; no changes needed.

### No new primitives
This packet does not introduce a new table, a new ability, a new
service, a new transport, or a new CI lane. Every change is a delete,
a relocate, or a tightening of an existing surface.

### Linear dependency edges read at L0 time
- Depends on: W4-A merged (block exists), W4-A0 merged (producer
  ability exists), W4-C merged (signing exists), W4-D merged (fallback
  projection exists), W4-E merged (presence nonce exists), W4-B
  merged (concurrency contract exists), DOS-589 merged (signal
  channel exists). All true on `dev` as of 2026-05-16.
- Blocks: W5 feedback writes (writes against a clean read path),
  W6 release gate (L4 proof depends on this).
- Surfaces residuals to: substrate-quality backlog (W4-Sub V2 findings
  remain as separate tickets; see §11).

## 5. What W4-F authors net-new

### Code changes (V2)

- **`surface_runtime/mod.rs` dispatch-site refactor** (V2 CRITICAL fix):
  - Line 760: change `app_state.db_write(move |db| validate_signed_session(...))` to `app_state.db_read(move |db| validate_signed_session(...))`.
  - Wrap the Err arm in a separate `app_state.db_write(...)` block that handles the failure-path writes: session-revoke / pairing-expire / suspend-pairing. The Ok arm stays purely on the read lane.
  - Implementation pattern:
    ```rust
    let validation = app_state.db_read(|db| validate_signed_session_readonly(db, input)).await;
    match validation {
        Ok(validated) => /* continue with no writer-mutex acquisition */,
        Err(SignedSessionFailure::SessionExpired) => {
            app_state.db_write(|db| mark_session_revoked(db, ...)).await;
            return error_response(...);
        }
        Err(SignedSessionFailure::SiteNonceMismatch) => {
            app_state.db_write(|db| suspend_pairing(db, ..., "site_nonce_mismatch")).await;
            return error_response(...);
        }
        // ... and so on for the five failure types
    }
    ```
  - The failure-path writer-mutex acquisitions only occur on rejected requests; valid traffic never touches `db_write` on the validation path. Rate-limit gates at `surface_client_bridge.authorize` catch flood-rejection attacks before they saturate the writer.

- **`services/surface_pairing.rs`**:
  - Split `validate_signed_session` into a pure-read `validate_signed_session_readonly(db: &ActionDb) -> Result<ValidatedSurfaceSession, SignedSessionFailure>` that performs ONLY SELECTs.
  - **`SignedSessionFailure` enum scope (V3 per codex H):** the enum has variants for ALL ~10 Err returns from the current `validate_signed_session` function (UnknownRuntimeAnchor :724, RestoredStalePairing :733, PairingRevoked :736/:741, PairingSuspended :740, PairingExpired :742, SessionInvalid :743/:746, SessionThrottled :753, SessionExpired (new), SiteNonceMismatch :760, SiteBindingMismatch :764, WpUserHashMismatch :771). Each variant carries an `requires_write: bool` discriminant (or trait method). Only the 5 write-needing variants (SessionExpired, PairingExpired, SiteNonceMismatch, SiteBindingMismatch, WpUserHashMismatch) route through Err-arm `db_write`; the rest return directly from `db_read` without escalation.
  - **Scope-recovery for audit attribution (V3 per eng H):** the `SignedSessionFailure` payload includes `granted_scopes: Option<ScopeSet>` populated by the readonly variant via the existing `load_session_scope_set_for_audit` lookup (which is itself a read, safe inside `db_read`). The dispatch handler's Err arm constructs `Actor::SurfaceClient` attribution from the enum payload without a second `db_read` round-trip and without needing to call `load_session_scope_set_for_audit` again.
  - Move the 5 write-needing failure-path writes (`mark_session_revoked`, `mark_pairing_expired`, three `suspend_pairing`) into separate exported functions called from the dispatch handler's Err arm via `db_write`.
  - Remove the Ok-path writes entirely (V1 lines 774-798):
    - `last_seen_at` updates: in-memory tracking map keyed by `session_id`. Lazy-flushed to DB on Tauri graceful shutdown only.
    - `last_used_at` updates: in-memory `AtomicI64` per pairing, surfaced via existing admin-diagnostics command path.
  - Session-validity check (in `validate_signed_session_readonly`): `revoked_at IS NULL AND absolute_expires_at > ?now AND lifecycle_state = 'active'`. `inactive_expires_at` is no longer consulted.

- **`migrations.rs` migration `v180`** (V3 corrected — DATA-ONLY, NOT NULL preserved):
  - **DO NOT** `ADD COLUMN absolute_expires_at` — column already exists in v169 at `migrations/169_dos_559_surface_client_pairings.sql:77-78`.
  - **DO NOT** backfill `inactive_expires_at = NULL` — v169 declares the column `NOT NULL`. V3 corrects this V2 mistake.
  - Policy change: for existing `surface_client_sessions` rows where `absolute_expires_at <= now()`, set `absolute_expires_at = COALESCE(created_at, now()) + 365 days`. (`created_at` is the actual v169 column name; v180 does not introduce `paired_at`.)
  - Existing `inactive_expires_at` values are LEFT UNCHANGED (forensic preservation).
  - Add SQL comment: `-- DEPRECATED v180: inactive_expires_at not consulted for validity. Retained for forensics. See absolute_expires_at.`
  - Add equivalent doc comment in `migrations.rs` adjacent to the v180 entry.
  - **New row insert policy** (changed in `services/surface_pairing.rs` pairing flow): new inserts into `surface_client_sessions` set `inactive_expires_at = absolute_expires_at` (same value), so the NOT NULL constraint is satisfied without ambiguous semantics. Semantically: "both columns share the same future timestamp; only `absolute_expires_at` is consulted for validity."
  - Schema does NOT change. v180 is a data + policy migration only.

- **`services/keychain.rs`** (DOS-646) — V2 tightened per cso H3:
  - Add `persist_surface_session_key` / `load_surface_session_key` storing the HMAC session key bytes under `com.dailyos.desktop.surface-session.<surface_client_id>` with:
    - `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
    - `kSecAttrAccessControl` configured per-app (DailyOS team ID ACL)
    - Application-bound: another binary running as the same user but signed by a different team ID cannot `SecItemCopyMatching`
  - Reconciliation on Tauri startup: if a session row exists in `surface_client_sessions` but the keychain entry is missing (user-deleted, machine-migrated), mark the session as `revoked_at = now()` with reason `keychain_entry_missing` and emit `pairing.session.key_missing` audit event. WP plugin gets a distinct `session_requires_repair` error code on next request.

- **`services/surface_runtime/mod.rs` sentinel** (DOS-636) — V2 hardened per codex C4 + cso M4 + consult R5:
  - Parent dir `~/.dailyos/` ensured at `0700` on startup.
  - Sentinel file at `~/.dailyos/runtime-endpoint.json` with:
    - Atomic write via `tempfile.persist()` (write to tmp then rename).
    - `0600` perms on creation.
    - `O_NOFOLLOW|O_EXCL` on open.
    - Payload contains ONLY: `port`, `startup_id` (random 32 bytes, generated each Tauri start), `runtime_version`. NEVER the HMAC session key or any auth material.
  - On every WP read: stat the file, verify mode & owner; if anything fails, treat as missing.
  - WP plugin verifies `startup_id` against a pairing-bound expected value (stored at pairing time, refreshed lazily on session-refresh roundtrip). A substituted sentinel with a fabricated `startup_id` will trigger session-refresh, which validates against the real Tauri via signed HMAC. The sentinel is port-discovery only; defense is the HMAC.
  - On bind failure or graceful shutdown: atomically remove the sentinel.

- **`wp/dailyos/includes/class-dailyos-plugin.php`** (DOS-636) — V2 retry policy per eng M2:
  - Read sentinel on each signed request, cache for 5s in-process.
  - On `ENOENT` (sentinel missing): retry up to 3× at 100ms before falling back to stored `dailyos_pairing_marker` option.
  - On `ECONNREFUSED` after a successful sentinel read: re-read sentinel once (Tauri may have restarted with new port), retry the request once.

- **CI gate** (new) — V2 mechanism per consult R7 + cso M1:
  - **Layer 1 (runtime counter, mandatory):** test fixture `validate_signed_session_ok_path_zero_writes` instruments `ActionDb::with_transaction` via the existing `db_write_observer` test harness (used by W4-B's concurrency tests at `services/compositions.rs:411-470`). The fixture asserts that for a successful signed-session validation, the observer records ZERO `with_transaction` entries.
  - **Layer 2 (AST gate, narrow):** Rust integration test that walks the AST of `validate_signed_session_readonly` and asserts no calls to write-shape APIs (`with_transaction`, `db_write`, `conn.execute` with INSERT/UPDATE/DELETE). Scope restricted to the readonly function; failure-path writers are NOT scoped.
  - **Phrasing of invariant:** "ZERO writes on the Ok-return path of any GET-shape route handler. Err-return-path writes are permitted." Failure-path writes (suspend_pairing, mark_session_revoked, mark_pairing_expired) explicitly pass.

### Documentation
- Update CLAUDE.md "Critical Rules" with a one-line entry pointing at
  the project description §"Threat model: local-to-local" — so future
  agents read the constraint, not just the audit.
- Add a docs/decisions entry only if the L0 reviewer panel asks for
  one (ADR territory if defense semantics change; this packet's
  position is they don't — only the *mechanism* changes).

## 6. Directional decisions resolved at L0

### 6.1. Why eliminate the writes instead of batching them
Batching (every N seconds, or coalescing within a transaction window)
moves the contention from "every request" to "every N seconds" but
keeps the wrong shape: the read path still writes. Tauri React's
`invoke` does not write on read. The WP equivalent must not either.
Eliminating the writes is the correct shape; batching is a
remote-model accommodation.

### 6.2. Why keep `last_seen_at` / `last_used_at` columns
Two reasons:
1. Admin diagnostics depend on them ("when did this session last
   make a request"). Removing the column would break that surface.
2. Lazy-flush on shutdown gives us a useful approximation without
   per-request cost. The exact value is not load-bearing — it's a
   debugging field, not an authority.

If a future packet decides admin diagnostics should come from the
audit log rather than the session row, the columns can drop entirely
then. Not in scope here.

### 6.3. Why absolute lifetime instead of "until unpaired" (V2 reframed per cso H2 + consult R3)

Absolute lifetime is **session hygiene/rotation, NOT a load-bearing security defense.** Default 365 days, user-configurable.

The Exfiltration defense (Phase 0 artifact 01 §Exfiltration) is:
- **Site-binding digest** — a stolen session key used from another machine fails site-binding validation at `surface_pairing.rs:763`, triggering `suspend_pairing` ("site_binding_mismatch") which auto-quarantines the pairing.
- **Nonce replay rejection** — captured signed envelopes are rejected on second use.
- **User-initiated unpair** — the user revokes the session via Tauri admin UI.

These three are what catch Exfiltration, not the inactivity-expiry the V1 packet was trying to preserve. Absolute lifetime sets an outer hygiene bound (re-pair every 365 days) but is not the security primitive.

### 6.4. Why the sentinel file for DOS-636 (V2 hardened per codex C4)

A fixed port (e.g., always 60101) collides with other services and removes the random-port defense for browser-origin-confused-deputy attacks. The sentinel file lets the port stay random while giving the WP plugin a deterministic read-side.

**Sentinel is a port-discovery convenience, not a defense.** Defense is the HMAC session key in keychain. An attacker who reads the sentinel without the HMAC key cannot make any signed request.

Hardening (V2):
- `0700` parent dir, `0600` sentinel, `O_NOFOLLOW|O_EXCL`, ownership+mode check on every read.
- Payload is `{port, startup_id, runtime_version}` only. NEVER auth material.
- WP verifies `startup_id` against a pairing-bound expected value; substituted sentinels trigger session-refresh, which re-validates against the real Tauri via HMAC.

A same-UID malicious process that replaces the sentinel can redirect WP to a fake port, but cannot impersonate Tauri because the fake endpoint cannot produce valid HMAC responses. WP detects the impersonation on the first signed request.

### 6.5. Why keychain for DOS-646 instead of DB persistence
The HMAC session key is a credential. The keychain is the correct
durability surface for credentials on macOS. Putting it in the
SQLCipher DB would couple session restoration to DB liveness — but a
corrupt or migrating DB is exactly when sessions most need to survive.
The keychain gives orthogonal durability.

### 6.6. What this packet does NOT change
- The HMAC signing algorithm.
- The presence-nonce TTL on write paths (60s is correct for writes;
  reads no longer need a nonce because reads aren't writes).
- The signed envelope structure (W4-C).
- The composition projection logic (W4-D).
- The producer ability invocation logic (W4-A0).
- The `commit_composition` write path (W4-B). Cache-miss versioning
  remains as-is; restructuring is a separate concern.
- The audit-log JSONL emission. Stays as-is — filesystem, not DB.

### 6.7. What about cache invalidation on writes
DOS-589 already builds the substrate→surface signal channel. When a
feedback write changes a claim, the signal already propagates and the
cache entry is invalidated. This packet does not need a new
invalidation mechanism; it just needs the read path to not write.

### 6.8. Dispatch-site relocation (V2 NEW per codex C1)

The actual contention source was missed by V1's audit. `validate_signed_session` is called inside `app_state.db_write(...)` at `surface_runtime/mod.rs:760`. The writer-lane acquisition happens at the *dispatch site*, not just from the SQL inside the callee. Removing the SQL inside the function does NOT relieve contention if the dispatch wrapper is still `db_write`.

V2 relocates dispatch to `db_read` for the Ok-path validation. Failure paths still use `db_write` but only when a request is being *rejected*, so:
1. Valid traffic never touches the writer mutex on validation.
2. Rejected traffic acquires the writer mutex for the auto-quarantine write — this is acceptable because the request is being rejected anyway.
3. Flood of rejection attempts (e.g., credential-stuffing) is bounded by `surface_client_bridge.authorize`'s in-memory rate-limit gate, which fires before the writer-mutex write.

### 6.8b. V179 rollback semantic note (V3.1 NEW per codex challenge cycle 3)

After v180 is applied, the read path no longer consults `inactive_expires_at`. Existing values remain (forensic preservation). However, **rollback to v179 binary**:
- v179 code at `surface_pairing.rs:726` consults `ts_before_or_equal(&row.inactive_expires_at, &now)` for validity.
- For sessions paired before v180 where `inactive_expires_at` was already in the past at v180 apply-time, rollback to v179 will reject those sessions as expired, forcing re-pair.
- For sessions paired after v180 (where the v180-corrected insert policy writes `inactive_expires_at = absolute_expires_at`), rollback to v179 works correctly because both columns carry the same future timestamp.

**Conclusion:** v180 is forward-safe but introduces an asymmetric-rollback footgun for sessions whose `inactive_expires_at` was already past at apply-time. Mitigation: documented as known v179-rollback behavior; if rollback is needed in production, users may need to re-pair WP sessions. This is acceptable for v1.4.2 since rollback is a manual-recovery operation, not a routine flow.

### 6.9. Failure-path write treatment (V2 NEW per codex C2 + cso H1)

The 5 conditional writes inside `validate_signed_session` (mark_session_revoked, mark_pairing_expired, three suspend_pairing) are **security-load-bearing** — they implement the auto-quarantine response to Site-Switch (site_nonce/site_binding mismatch) and Exfiltration (wp_user_hash mismatch) attacks named in Phase 0 artifact 01.

V2 cannot remove them. V2 preserves them by:
- Splitting validation into `validate_signed_session_readonly` (pure SELECT) returning a `SignedSessionFailure` enum.
- Dispatch handler matches on the enum and invokes the corresponding writer function from its Err arm via `db_write`.
- The writer-function exports (`mark_session_revoked`, `mark_pairing_expired`, `suspend_pairing`) remain in `services/surface_pairing.rs` with their existing SQL semantics.

This treatment preserves every defense while satisfying "Ok-path zero writes" — the central invariant of the local-to-local threat model.

## 7. Acceptance criteria (V2 — scoped per cycle 1)

Acceptance is satisfied when ALL of the following are true. Criteria are scoped to the **Ok-return path** unless explicitly noted otherwise.

1. **Dispatch-site refactor:** `surface_runtime/mod.rs:760` calls `app_state.db_read(...)` for `validate_signed_session_readonly`, NOT `app_state.db_write`. Verified by integration test that asserts the dispatch handler's Ok branch never enters the writer lane.
2. **Ok-path zero writes (sessions):** A successful `POST /v1/surface/project-composition` request performs ZERO writes to `surface_client_sessions` and `surface_client_pairings`. Verified by `db_write_observer` test fixture that records zero `with_transaction` entries on the Ok return.
3. **Failure-path writes PRESERVED:** Site-Switch (site_nonce_mismatch, site_binding_mismatch) and Exfiltration (wp_user_hash_mismatch) failure paths still call `suspend_pairing`, transitioning `surface_client_pairings.lifecycle_state` to `suspended` per Phase 0 artifact 01. Verified by negative fixtures (`dos655_site_switch_suspends_pairing`, `dos655_exfiltration_suspends_pairing`).
4. **Cache-hit zero writes:** The cache-hit path of `POST /v1/surface/project-composition` performs ZERO SQLite writes (any table). Verified by the `db_write_observer` fixture with cache pre-populated.
5. **Cache-miss writes scoped to commit_composition only:** The cache-miss path is allowed exactly one writer-mutex acquisition, in `commit_composition` (W4-B contract). No other writes occur. Verified by the same fixture.
6. **1000 sequential reads under background writer hold:** A signed session remains valid across 1000 sequential signed requests with a concurrent background task holding the writer mutex via `db_write` for 100ms each. All 1000 reads complete with NO timeout. Verified using `db_write_observer` + a synthetic writer-mutex-hog test harness.
7. **Session validity rule:** Determined by `revoked_at IS NULL AND absolute_expires_at > ?now AND lifecycle_state = 'active'`. `inactive_expires_at` is no longer consulted. Verified by negative fixture: a session row with `inactive_expires_at` in the past but `absolute_expires_at` in the future still validates.
8. **Migration v180 (data-only, idempotent; V3.1 corrected):**
   - Does NOT add `absolute_expires_at` column (already in v169).
   - **Does NOT** backfill `inactive_expires_at = NULL` — v169 declares the column NOT NULL. Existing values are LEFT UNCHANGED for forensic preservation.
   - For any `surface_client_sessions` row where `absolute_expires_at <= now()`, set `absolute_expires_at = COALESCE(issued_at, now()) + 365 days`. (`issued_at` is the actual v169 column at `migrations/169_dos_559_surface_client_pairings.sql:75`, not `created_at` or `paired_at`.)
   - SQL comment: `-- DEPRECATED v180: inactive_expires_at not consulted for validity; retained for forensics.`
   - Idempotent: re-running the migration is a no-op.
   - Verified by the existing migration-test harness on a copy of James's prod DB.
9. **Lazy-flush on graceful shutdown:** `last_seen_at` and `last_used_at` columns populated on Tauri graceful shutdown via a single coalesced UPDATE. Crash-stop tolerated; columns may be stale; admin diagnostics divergence up to absolute-lifetime-window is acceptable per §6.2.
10. **Keychain HMAC session key persistence (DOS-646):** Session key bytes persist across Tauri restart for any session row in `surface_client_sessions`. Verified by integration test: pair WP → restart Tauri → signed request from WP succeeds without re-pairing.
11. **Keychain ACL (V3.2 generalized):** Code-signing-bound app isolation — implementation picks the specific macOS Keychain mechanism (e.g. `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` + access-control flags, or equivalent). Negative fixture `dos655_keychain_isolation` is authoritative: a separate test binary signed by a different team ID cannot `SecItemCopyMatching` the entry.
12. **Keychain reconciliation:** On Tauri startup, if a `surface_client_sessions` row exists but the keychain entry is missing, the session is marked `revoked_at = now()` with reason `keychain_entry_missing`, audit `pairing.session.key_missing` is emitted, and the WP plugin receives a distinct `session_requires_repair` error code on next request.
13. **Sentinel file discovery (DOS-636):** WP plugin discovers the current Tauri runtime port via `~/.dailyos/runtime-endpoint.json` within 5 seconds of Tauri restart. Verified by manual L4 test.
14. **Sentinel TOCTOU hardening (V3.2 simplified — startup_id challenge removed):**
    - Parent dir `~/.dailyos/` is `0700`; sentinel `0600`.
    - Written via `tempfile.persist()` atomic rename.
    - Opened with `O_NOFOLLOW|O_EXCL`.
    - Read verifies ownership + mode before parsing.
    - Payload contains ONLY `port` and `runtime_version`. NEVER auth material.
    - Defense against a substituted sentinel: WP's existing HMAC validation on every response — a fake endpoint at the substituted port cannot produce valid HMAC responses (HMAC key lives in keychain, ACL-bound), so WP's first signed request to the fake endpoint fails HMAC validation and surfaces `session_requires_repair`. No startup_id challenge needed; HMAC IS the defense.
    - Verified by unit tests + negative fixture `dos655_sentinel_substitution_detected_via_hmac_fail`.
15. **Cache-miss latency on cold cache (cycle 1 codex H5 + consult R4, V3 reconciled to warm 200ms):** First request after Tauri restart (cold cache) renders `dailyos/account-overview` against James's production DB with p95 ≤ 1.0s. Warm-cache p95 ≤ 200ms. Measured via 20 consecutive cold-then-warm render sequences. If cold-cache p95 exceeds the budget, W4-Sub finding #14 (composition version churn restructure) escalates to v1.4.3 critical path.
16. **CI gate active (two-layer, V3 hardened):** Layer 1 (runtime counter, mandatory) and Layer 2 (AST gate with N=3 transitive callee walk) per §9. Plus route-detection mechanism and symbol-existence guard.
17. **L4 proof:** A `dailyos/account-overview` block renders end-to-end in WP Studio against James's production DB with screenshot evidence captured. Satisfies the L4 acceptance from W4-A (§59 + §60).
18. **Linting and type checks:** `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green. WordPress plugin lints green.
19. **Operational warts retested:**
    - DOS-636: WP plugin auto-discovers runtime port after Tauri restart, no manual option-patching required.
    - DOS-646: WP→Tauri signed requests survive a Tauri restart with no re-pairing.
    - DOS-647: any DB error on the read path surfaces as a loud `log::warn!` with `rusqlite::Error::code()`, not a silent `pairing_authority_unavailable`. (Mostly moot once dispatch relocates to `db_read`, but the diagnostic is preserved as defensive change.)
20. **Cache-miss latency budget V3 reconciliation:** cold-cache p95 ≤ 1.0s, warm-cache p95 ≤ 200ms (consistent throughout packet; supersedes any V2 reference to 500ms warm).
21–22. **REMOVED in V3.2** — see §2 V3.2 changelog. Both criteria (indistinguishable Err responses, startup_id HMAC rotation) defended against threats outside the local-to-local model that v1.4.2 explicitly commits to. Filed as v1.4.3-federation maintenance candidates.

## 8. Negative fixtures (V2 — expanded per cycle 1)

1. `dos655_validate_readonly_does_not_write` — calls `validate_signed_session_readonly` against a test DB with `db_write_observer`; observer records zero `with_transaction` entries.
2. `dos655_dispatch_ok_path_uses_db_read` — exercises the dispatch handler at `mod.rs:760` with a valid session; asserts only `db_read` is called, never `db_write`.
3. `dos655_dispatch_err_path_writes_via_db_write` — exercises each of the 5 failure paths (session-expired, pairing-expired, site_nonce_mismatch, site_binding_mismatch, wp_user_hash_mismatch) and asserts the corresponding writer function is called via `db_write`.
4. `dos655_site_switch_suspends_pairing` — submits a request with mismatched `site_binding_digest`; asserts `surface_client_pairings.lifecycle_state` transitions to `suspended` and `pairing.site_binding_mismatch` audit event is emitted.
5. `dos655_exfiltration_suspends_pairing` — submits a request with mismatched `wp_user_hash`; asserts pairing suspended + `pairing.wp_user_mismatch` audit.
6. `dos655_project_composition_cache_hit_zero_writes` — cache hit path end-to-end; observer asserts zero writes.
7. `dos655_project_composition_cache_miss_only_commit_writes` — cache miss path; observer asserts the only writes are from `commit_composition`.
8. `dos655_session_validates_with_stale_inactive_expires_at` — session row has `inactive_expires_at` in the past but `absolute_expires_at` in the future; validate returns Ok.
9. `dos655_session_rejects_past_absolute_expires_at` — session row has `absolute_expires_at` in the past; validate returns `SessionExpired`.
10. `dos655_session_key_survives_restart` — persist a session key via keychain, simulate restart by dropping + recreating the keychain client, load the key; bytes match.
11. `dos655_keychain_isolation` (cycle 1 cso H3) — separate test binary signed by a different team ID attempts `SecItemCopyMatching`; fails with `errSecAuthFailed` or equivalent.
12. `dos655_keychain_missing_reconciles_session` (cycle 1 codex H3) — session row exists but keychain entry deleted; Tauri startup marks session `revoked_at = now()` with reason `keychain_entry_missing`; audit emitted.
13. `dos655_sentinel_atomicity` — write sentinel, kill -9 simulated mid-write via tempfile-based test harness; observer never sees a partial file.
14. `dos655_sentinel_perms` — write sentinel; assert parent dir is 0700, sentinel is 0600, ownership is current user.
15. `dos655_sentinel_no_auth_token` — write sentinel; assert payload contains only `port` and `runtime_version` (V3.2 — startup_id removed alongside AC #22); no key material.
16. `dos655_sentinel_substitution_detected_via_hmac_fail` (V3.2 simplified) — replace sentinel to point WP at a fake endpoint; WP signs a request; fake endpoint cannot produce valid HMAC response; WP rejects + surfaces `session_requires_repair` to user. Defense is HMAC validation; no startup_id challenge.
17. `dos655_sentinel_o_nofollow` — replace sentinel with a symlink; read fails with `ELOOP` per `O_NOFOLLOW` open flag.
18. `dos655_wp_plugin_discovers_new_port` — start runtime on port A, write sentinel; WP signed request succeeds. Stop runtime, start on port B, rewrite sentinel; WP signed request succeeds without re-pairing.
19. `dos655_wp_plugin_3x_retry_on_missing_sentinel` — remove sentinel; WP signed request retries 3× at 100ms; if sentinel reappears, request succeeds.
20. `dos655_migration_v180_idempotent` — apply v180 twice; second application is a no-op.
21. `dos655_migration_v180_does_not_add_column` (cycle 1 codex C3) — apply v180 to a v179 DB; assert `PRAGMA table_info(surface_client_sessions)` shows the same columns before and after (data-only migration).
22. `dos655_migration_v180_backfills_absolute_expires_at` — pre-migration `surface_client_sessions` rows with `absolute_expires_at <= now()` get backfilled to `COALESCE(issued_at, now()) + 365d`; pre-migration rows with future `absolute_expires_at` are left unchanged. Existing `inactive_expires_at` values are NEVER set to NULL (v169 NOT NULL constraint preserved by acceptance #27).
23. `dos655_concurrent_reads_no_writer_contention` (cycle 1 eng H1) — background task holds writer mutex for 100ms; issue 1000 sequential signed reads; all complete with NO timeout, observer records zero writer-mutex acquisitions on the read path.
24. `dos655_cache_miss_latency_under_budget` (cycle 1 codex H5) — cold cache + James's prod DB; first render cold p95 ≤ 1.0s, warm p95 ≤ 200ms across 20 consecutive cold-then-warm sequences. Asserts on the histogram, not a single point.
25. `dos655_v180_inactive_expires_at_not_null_preserved` (V3 cycle 2 codex CRITICAL; renumbered in V3.2) — apply v180 to a v179 DB; assert all `surface_client_sessions.inactive_expires_at` values remain populated (never NULL); v180 SQL contains no `UPDATE surface_client_sessions SET inactive_expires_at = NULL` statement.

*(Fixtures 26-27 from V3.1 removed in V3.2 alongside their parent ACs.)*

## 9. CI invariants (V2 — two-layer per cycle 1)

1. **Layer 1 — Runtime counter (mandatory):** test fixture `validate_signed_session_ok_path_zero_writes` instruments `ActionDb::with_transaction` via `db_write_observer` (existing harness from W4-B's concurrency tests). For an Ok-return-path validation, observer records ZERO `with_transaction` entries. Failure-path Err returns are explicitly OUT OF SCOPE — they MAY write via the dispatch handler's Err arm.

2. **Layer 2 — AST gate (V3.2 simplified, advisory):** Rust integration test that walks the syntax tree of `validate_signed_session_readonly` ONLY (no transitive walk). Asserts no calls to write-shape APIs (`with_transaction`, `db_write`, `conn.execute` with INSERT/UPDATE/DELETE) directly in its body. Layer 1 runtime counter is the mandatory gate; Layer 2 is defense-in-depth for the symbol-bound surface. Off-thread writes via `tokio::spawn` etc. are caught by Layer 1 (the observer records writes regardless of which thread fires them).

3. **Allowlist discipline (V3.2 simplified).** `GET_SHAPE_HANDLERS` const enumerates every route handler that must pass Layer 1. New GET-shape handlers added without an entry should be added by the implementing PR. CI enforcement of "new route detected without entry" is a SHOULD-have, not a MUST-have for v1.4.2; demoted from V3's mandatory cargo-test mechanism.

4. **Sentinel file path discipline.** Grep fails CI if any code other than `surface_runtime/mod.rs` writes to `~/.dailyos/runtime-endpoint.json`. WP plugin and admin commands are read-only consumers.

5. **Sentinel payload discipline.** Grep fails CI if the sentinel-write code path emits any field other than `port`, `startup_id`, `runtime_version`. Specifically forbids `auth_token`, `session_key`, `hmac_key`, `secret`.

6. **Keychain naming discipline.** Grep fails CI if any code stores session-key-shaped bytes under a keychain service name other than `com.dailyos.desktop.surface-session.<id>`.

7. **Keychain ACL discipline.** Grep fails CI if any keychain insert for the surface-session prefix omits `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` or `kSecAttrAccessControl`.

8. **Migration v180 immutability.** Grep fails CI if `migrations/180_*.sql` contains `ALTER TABLE ... ADD COLUMN absolute_expires_at` (the column already exists in v169; v180 is data-only).

9. **Existing CI invariants preserved.** W4-A's invariants on block-attribute discipline, W4-B's concurrency contract, W4-C's signing contract, W4-D's projection contract, W4-E's nonce contract — all remain green.

10. **Symbol-existence guard (V3 cycle 2 cso H).** Grep fails CI if the symbol `validate_signed_session_readonly` is missing from `services/surface_pairing.rs`, OR if `surface_runtime/mod.rs` dispatch site does not call `validate_signed_session_readonly` inside an `app_state.db_read(...)` block. Binds the refactor to the gates so a future rename cannot silently bypass them. **Evaluated against the PR's final tree state, not intermediate commits** (so the migration→rename→dispatch sequence within a PR is not blocked mid-implementation per cycle 3 eng HIGH).

11. **`requires_write` exhaustive-match enforcement (V3.1 NEW per codex challenge cycle 3).** The dispatch handler that matches on `SignedSessionFailure` MUST be an exhaustive match without a wildcard arm, OR the enum exposes a trait method `fn writer(&self) -> Option<WriterFn>` that the dispatch handler invokes. Either approach forces every new variant to explicitly declare whether it requires a writer-mutex acquisition. Verified by integration test `dos655_signed_session_failure_exhaustive_dispatch`: adding a new variant without an explicit write-policy declaration fails compilation.

## 10. Interlocks

### With W5 (Feedback + theme + negative fixtures)
- W5 introduces feedback writes. Feedback writes are the genuine
  write path; they keep the presence nonce, the signed envelope, the
  claim version check, the scope check. W4-F does not change W5's
  contract. W5 simply executes against a clean read path.
- The CI invariant #1 (Ok-path zero writes) does NOT apply to feedback
  writes — those are write-shape routes (POST `/v1/surface/feedback`),
  explicitly excluded from the invariant scope.
- **W5 contention interlock (V2 NEW per cycle 1 consult R2):** feedback
  writes WILL contend with background workers on the SQLite writer
  mutex. W4-F does not solve this — solving it requires ADR-0067 Stage
  3 (split-pool / priority lane), which is explicitly out of scope.
  W5's kickoff acceptance MUST include a foreground-write latency
  measurement against James's production DB. If foreground writes p95
  exceeds an agreed budget (recommend 1.5s warm, 3s cold), ADR-0067
  Stage 3 escalates from "substrate-quality backlog" to "v1.4.3
  critical path." This interlock is documented in W4-F's L0 packet so
  W5 cannot start without acknowledging it; the actual decision is
  W5's, not W4-F's.

### With W6 (Audit + clean-machine validation + release gate)
- L4 proof (DoD §2: first block renders end-to-end) becomes
  achievable. W6's release gate depends on L4 being captured.
- Audit emission (DoD §8: audit log carries SurfaceClient instance
  identity + WP user_id) continues unchanged. W4-F does not change
  audit semantics — it just doesn't gate audit on a DB write.

### With W4-Sub V2 (BLOCKED)
- W4-Sub V2's 14 architectural findings remain real. They are
  classified at §11 below into "becomes latent under W4-F" vs
  "remains regardless." The latter file as substrate-quality backlog
  tickets in the Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`)
  per CLAUDE.md feedback rules.

## 11. What W4-F explicitly does NOT own

### Out of scope, becomes latent under local-to-local
- **W4-Sub finding #8** (writer-mutex FIFO contention on foreground
  writes): the foreground read path no longer takes the writer
  mutex. Foreground writes (feedback) are rare and tolerate latency.
  ADR-0067 Stage 3 (split-lock) is not required for v1.4.2.
- **W4-Sub finding #9** (background worker churn): contention
  exists but not on the render hot path. Cadence overhaul is
  beneficial but not load-bearing for L4.
- **W4-Sub finding #10** (`BackgroundScheduler` abstraction): same
  reasoning. File as substrate-quality backlog.
- **W4-Sub finding #11** (`abilities-runtime` crate-boundary): only
  load-bearing if recurring source pollers migrate to abilities.
  Defer until that migration is actually planned.
- **W4-Sub finding #12** (`Actor::RecurringSource` authorization):
  same. Defer.
- **W4-Sub finding #13** (Actor enum split): existed before W4-F;
  unchanged by W4-F; file as substrate-quality backlog.

### Out of scope, still real
- **W4-Sub finding #4** (`is_safe_ability_name` allow-list missing `/`):
  Located at `surface_runtime/mod.rs:2294`. V2 PROMOTED this into W4-F scope
  based on a faulty W5 dependency claim. V3 cycle 2 verified via the wave plan
  (§"Agent W5-A"): **W5-A feedback router posts to `/v1/surface/feedback`,
  NOT `/v1/surface/invoke`.** The validator's only caller is `/v1/surface/invoke`
  at `mod.rs:1602`. W5 does NOT need this fix to kick off. V3 DEMOTES the
  finding back to its original maintenance status; the structural validator
  fix files as a separate Maintenance project ticket.
- **W4-Sub finding #5** (composition route handler architecture):
  the route exists now (W4-A merged); the deeper concerns about
  cache-key scope-keying are W4-A's territory, not W4-F's.
- **W4-Sub finding #6** (schema drift on `surface_client_sessions.
  throttled_until_at`): apply the proper Rust-gated migration in a
  separate small ticket. Already applied manually to James's prod
  DB.
- **W4-Sub finding #7** (silent error swallowing in surface_runtime
  Err arms): valuable defensive work. W4-F adds the diagnostic
  `log::warn!` already drafted in the integration worktree; the
  broader sweep files as substrate-quality backlog.
- **W4-Sub finding #14** (`commit_composition` version churn on
  cache miss): real; not load-bearing for L4. Cache hits dominate
  steady-state; cache miss costs an extra write per (composition,
  version) tuple. File as substrate-quality backlog for v1.4.x.
- **W4-Sub finding #15** (migration slot reservation discipline):
  W4-F claims `v180`. File the broader discipline ticket separately.
- **W4-Sub finding #16** (CI lint enforcement mechanism): W4-F's
  CI invariant #1 needs the same robust mechanism. §12 Open Q1
  asks reviewers which mechanism to use.

### V3.2 trims filed as v1.4.3-federation candidates

When v1.4.3+ extends DailyOS to remote shapes (federation, hosted substrate, multi-machine MCP), the following defenses become load-bearing and re-enter scope:

- **Indistinguishable Err responses** (V3 AC #21 / V3 fixture #25). Defense against credential-dimension fingerprinting becomes relevant when an attacker can observe responses without holding the HMAC key (e.g., a remote attacker who has compromised the WP plugin's network path). Local-to-local: not relevant. File as v1.4.3-federation maintenance ticket.

- **startup_id HMAC-validated rotation** (V3 AC #22 / V3 fixture #26). Defense against substituted-endpoint impersonation becomes relevant under federation transport where HMAC validation may not happen on every message. Local-to-local: HMAC already covers this. File as v1.4.3-federation maintenance ticket.

- **Layer 2 AST gate with N=3 transitive callee walk + tokio::spawn closure traversal + route-detection cargo test.** Defense-in-depth against drift becomes more valuable as the surface footprint grows. v1.4.2 has 6 GET-shape routes; the runtime counter catches every drift case that matters. File as substrate-quality maintenance ticket for v1.4.x.

### Out of scope, project-level
- The Tauri UI's long-term role (ADR-0129 §7). v1.4.2 does not
  decide.
- Multi-tenant / hosted substrate / federation. Outside the
  local-to-local threat model.
- Markdown ingestion (v1.4.6).
- BYOM agent backend.
- Production install signing.

## 12. Open questions for L0 reviewers — all resolved in cycle 1

1. **CI invariant mechanism — RESOLVED:** Two-layer per consult R7 + cso M1. Layer 1 mandatory runtime-counter fixture (d). Layer 2 narrow AST gate on enumerated GET-shape handlers (c). Codified in §9.

2. **Absolute lifetime default — RESOLVED:** 365 days, user-configurable. **Framing corrected:** absolute lifetime is session hygiene/rotation, NOT load-bearing security. The Exfiltration defense is site-binding + nonce + unpair (per Phase 0 artifact 01). Codified in §6.3.

3. **last_seen_at lazy-flush trigger — RESOLVED:** Shutdown-only for v1.4.2. Admin diagnostics divergence up to absolute-lifetime-window is documented as acceptable in §6.2 + Acceptance #9.

4. **Migration v180 deprecation comment — RESOLVED:** Both. `-- DEPRECATED v180: inactive_expires_at not consulted for validity. See absolute_expires_at.` in the migration SQL, AND a one-line note in `migrations.rs` adjacent to v180.

5. **DOS-636 sentinel race — RESOLVED:** Sentinel-missing → 3× retry at 100ms before falling back to stored `dailyos_pairing_marker` option. ECONNREFUSED after a successful sentinel read → re-read sentinel once + retry the request once. Codified in §5.

6. **DOS-646 keychain scoping — RESOLVED:** Per `surface_client_id` as drafted. ACL tightening added in V2 per cso H3: `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` + per-app ACL bound to DailyOS team ID.

7. **W4-F merge order vs W5 kickoff — RESOLVED:** Full L0 panel, not expedited. The scope is narrow but the constraint is project-level. W5 kickoff acceptance must include the foreground-write latency interlock per §10.

8. **W4-Sub V2 substrate-quality findings filing — RESOLVED:** File before L0 closes. §15 makes this a closure precondition. V3 NOTE: V2's promotion of W4-Sub finding #4 (`is_safe_ability_name`) into W4-F scope was reverted per cycle 2 verification — W5 does not need it. All 6 W4-Sub findings (originally 5, now 6 with #4 returned to maintenance) file as separate Maintenance project tickets.

## 12b. New open questions raised in cycle 1 — RESOLVED

9. **Dispatch-site refactor scope (codex C1) — RESOLVED:** Relocate `mod.rs:760` from `db_write` to `db_read`. Split `validate_signed_session` into a pure-read `validate_signed_session_readonly` returning `SignedSessionFailure` enum. Failure-path writers exported separately and invoked from the dispatch handler's Err arm. Codified in §5 + §6.8.

10. **Failure-path write treatment (codex C2 + cso H1) — RESOLVED:** Preserve all 5 failure-path writes (mark_session_revoked, mark_pairing_expired, three suspend_pairing). Routed via dispatch Err arm `db_write`. CI invariant phrasing scoped to Ok-return path only. Codified in §6.9 + §9.1.

11. **Migration v180 scope (codex C3) — RESOLVED in V3, corrected in V3.1:** Data-only, NOT schema-changing. `absolute_expires_at` already exists in v169. v180 LEAVES `inactive_expires_at` UNCHANGED (v169 NOT NULL preserved) + repairs past `absolute_expires_at` to `COALESCE(issued_at, now()) + 365d`. CI invariant #8 forbids future `ADD COLUMN absolute_expires_at`. The V3 entry initially had a NULL-backfill error caught in cycle 3; corrected in V3.1.

12. **Sentinel TOCTOU hardening (codex C4 + cso M4 + consult R5) — RESOLVED:** Parent dir 0700, sentinel 0600, `O_NOFOLLOW|O_EXCL`, ownership+mode check, payload contains only port/startup_id/runtime_version (NO auth material), WP verifies startup_id against pairing-bound expected value. Codified in §5 + §6.4 + Acceptance #14.

13. **Cache-miss latency budget (codex H5 + consult R4) — RESOLVED:** Acceptance #15. Cold-cache p95 ≤ 1.0s, warm-cache p95 ≤ 200ms, measured on James's prod DB. If cold exceeds budget, W4-Sub finding #14 (composition version churn restructure) escalates to v1.4.3 critical path.

14. **W5 contention interlock (consult R2) — RESOLVED:** §10 W5 interlock. W5 kickoff acceptance must measure foreground-write latency; failure escalates ADR-0067 Stage 3 to v1.4.3.

## 13. Linear dependency edges

- Parent: DOS-546 (v1.4.2 program).
- Wave: 4-F (closing Wave 4 threat-model gap).
- Linear ticket: DOS-655 — filed 2026-05-16, status Backlog, priority Urgent.
- Blocks: W5 kickoff (feedback writes), W6 release gate (L4 proof).
- Blocked by: Nothing on `dev` as of 2026-05-16 (W4 stage-3 fully
  merged after rebase).
- Subsumes operationally: DOS-636 (folded as Acceptance #9, #10, §12 Q5), DOS-646
  (folded as Acceptance #8, §12 Q6), DOS-647 (becomes mostly moot;
  diagnostic log is folded as Acceptance #14 sub-bullet 3).
- Files-during-implementation: 6 substrate-quality tickets in
  Maintenance project per §11.

## 14. L0 reviewer panel — required runners

Per CLAUDE.md ladder + the W4-A precedent:

- `/plan-eng-review` — primary engineering review.
- `/codex challenge` — adversarial pass.
- `/codex consult` — design consult.
- `/cso` — REQUIRED: this packet changes session-lifecycle defense
  semantics (absolute lifetime replaces inactivity expiry). Trust-
  boundary territory.
- `/plan-design-review` — NOT required (no user-facing UI change).
- `/plan-devex-review` — NOT required (no MCP / API surface change).

Unanimous required, bounded by the acceptance criteria in §7. Per
the feedback memory `l2_must_review_against_acceptance_criteria` and
`l2_path_alpha_to_maintenance_project`: theoretical hardening
findings outside §7 file to maintenance, not back into the packet.

## 15. Acceptance for L0 closure

L0 closes when ALL of the following are true:

1. All four required reviewers return APPROVE.
2. No reviewer flags a CRITICAL or HIGH finding inside the §7
   acceptance scope. (Reviewer findings outside §7 file as
   maintenance tickets; they do not block.)
3. §12 open questions all have resolved recommendations recorded
   in the changelog (V2 entry).
4. Maintenance tickets filed in Linear under the Maintenance project before L0 closes (V3.2 count = 8):
   - 5 W4-Sub V2 findings (#9, #10, #13, #14, #15, #16 — the originally-deferred substrate-quality items)
   - 1 W4-Sub finding #4 (`is_safe_ability_name` structural validator) returned to maintenance via V3 demotion
   - 2 V3.2 trims as v1.4.3-federation candidates:
     - Indistinguishable Err responses (V3 AC #21)
     - startup_id HMAC-validated rotation (V3 AC #22)
5. A Linear ticket is created for W4-F itself with the title
   "W4-F: Local-to-local read path" and this packet linked.

When L0 closes, the implementation lane:
1. Picks up the Linear ticket.
2. Implements §5 in the order listed.
3. Validates §7 acceptance.
4. Runs L2 (codex review + code-reviewer + cso) per the wave plan.
5. Opens PR.
6. Captures L4 proof per Acceptance #12.
7. Files the wave-4 retro doc that was held pending L4 closure.
