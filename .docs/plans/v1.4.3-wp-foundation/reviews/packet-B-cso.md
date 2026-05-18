# CSO review — L0 Packet B (WP preview/runtime render stabilization)

**Verdict: CONDITIONAL APPROVE**

Reviewer: CSO mode (Claude)
Date: 2026-05-17
Packet: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md` V1.0
Source SHAs read (working tree):
- `src-tauri/src/surface_runtime/mod.rs:2238–2451` (project-composition handler — verified)
- `src-tauri/src/services/composition_render_orchestrator.rs:1–120` (verified: 60s TTL, in-memory DashMap, no signal-listener wired)
- `src-tauri/src/services/compositions.rs:67–175` (verified: `commit_composition` is the single mutation entry; OCC token = DB current_version not surface watermark)
- `src-tauri/src/bridges/surface_client.rs:200–435, 1057–1107` (verified: `standard_read_composition` = 60/min + burst 5; rate-limit rejection emits `surface_client.rate_limit` audit event; authorization runs BEFORE rate-budget consumption inside `authorize()` — they are coupled, not orderable separately)
Threat-model anchor: v1.4.2 project description "Threat model: local-to-local" + Packet A cycle-2 closed verdict (same-UID local trust boundary, federation-scale concerns to v1.x-federation maintenance).

Three concerns force conditional approval. None blocks the packet. Two require packet text amendments (§5.4 cache-invalidation contract; §5.5 audit-emission preservation). One nudges the §12 Q3 default toward shape (a) bypass-with-explicit-audit, not shape (b) raise-the-number.

## Summary table

| # | Concern | Severity | Disposition |
|---|---|---|---|
| 1 | §5.4 producer-commit removal creates a read-after-stale window with no signal-driven invalidation today | **MEDIUM** | Approve §5.4 with §5.3 shape (b) version-agnostic cache; add explicit invalidation contract — §7 AC #16 |
| 2 | §5.5 render-read decharge — audit-event preservation | **LOW** | Approve shape (a) bypass; require `surface_client.render_read` audit event still fires at authorization; §9 invariant #8 |
| 3 | §5.6 typed error mapping — fingerprinting channel | **LOW** | Approve as drafted; the codes are already in the wire format. No new channel created. |
| 4 | Authorization-vs-cache-vs-rate-budget ordering | **LOW** | Verified safe as drafted. Authorization and rate-budget are coupled inside `authorize()`; cache check happens AFTER. §5.5 must keep authorization at the front. §9 invariant #9. |
| 5 | WP transport trust model under local-to-local | **N/A** | Confirmed. Signed-loopback is the boundary; once signed, WP is a same-UID trusted surface. No sandbox. |
| 6 | Sentinel-redirection vector (Packet A interlock) | **LOW** | Verified disjoint code regions (Packet A at `:415–419`, `:471–475`, `:818`, `:887`; Packet B at `:2280–2450`). No overlap. No ordering hazard. |

## Concern 1 — Render-path producer-commit removal (§5.4)

**Verdict: MEDIUM — approve the architectural change; require an explicit invalidation contract because there is no signal listener wired today.**

I read `composition_render_orchestrator.rs` end-to-end. **Observation:** the orchestrator has NO subscription to `account_subject.claim_changed`, `claim.lifecycle`, `claim.dismissal`, `source.freshness`, or `source.revocation`. The cache's sole invalidation today is the 60s TTL. The only mechanism that fills it with a fresh projection is the producer-commit-on-render at `mod.rs:2348–2377`. **That commit-on-render IS the de-facto invalidation channel today, even though it was rationalized as an OCC-token fix.**

This makes §5.4 substantively MORE than a "stop writing on read" refactor. It removes the only mechanism keeping the cache live to substrate changes inside a 60s window. The packet's "signal-propagation invalidation" trigger (§5.4 bullet 2) does not exist as wired code; it is a future contract the packet assumes.

**Local-to-local impact analysis.** Same-UID local-to-local does not change the staleness picture — claims advance from the SAME process via the SAME ServiceContext. A user dismissing a claim in Tauri and immediately reloading the WP preview is a realistic single-user, single-process scenario. With §5.4 as drafted:
- T=0: user dismisses claim → `commit_composition` advances DB version to N+1.
- T=1s: user reloads WP preview → cache lookup hits the entry from version N (within TTL); shows stale.
- T=60s: TTL expires; next read repopulates from N+1.

The packet's §6.5 says "Authorization-without-consumption is a coherent local-shape." Agreed. But "cache-without-invalidation" is NOT coherent — it just defers the read-after-stale window from "until the producer happens to run again" (the current shape) to "until the 60s TTL expires" (the proposed shape). Same severity, different bound.

**The DOS-670 substrate-side OCC contract.** The packet's §12 Q2 asks whether removing render-path commit breaks the v1.4.2 W4-F producer OCC fix. I read `compositions.rs:67–147`. `commit_composition` reads `current_db_version` via the per-mutation guard, NOT via the request's `expected_composition_version`. The `expected_composition_version` parameter in the producer ability invocation is used by the producer ability itself, not by `commit_composition`. **The DOS-670 fix does NOT require commit-on-render** — it requires that when the producer DOES commit, it commits against current DB version. That contract is still satisfied if commits only fire on explicit refresh, signal-propagation invalidation, or initial creation.

So §5.4 is architecturally sound — but it must be paired with a wired invalidation path, not a paper one.

**Required amendments (V1.1):**

> **§5.4 — add invalidation contract:** When `commit_composition` succeeds for a `composition_id` (in any path: explicit refresh, signal-driven, initial creation, OR any future mutation entry), the orchestrator's cache entries for that `composition_id` (across ALL scopes_canonical_ids) SHALL be invalidated synchronously in the same transaction-commit hook. Pick §5.3 shape (b) version-agnostic key — it makes this invalidation tractable (drop all entries for `composition_id`) without enumerating versions.

> **§7 add AC #16:** Cache invalidation on commit: a unit test asserts that after `commit_composition(composition_id=X)` succeeds, the next `cache_lookup(actor, X, *)` returns `None`, regardless of TTL. This is the contract that makes "no commit on render" safe. The hook lives in the commit success path inside `commit_composition` or a `LiveCompositionCommitter` wrapper, not in the route handler — so future write paths (feedback, edit affordances) inherit it.

> **§9 add CI invariant #8:** Grep gate — every `commit_composition` call site is followed (statically) by an orchestrator `invalidate(composition_id)` call OR is itself inside a `LiveCompositionCommitter` shim that handles invalidation. This is the structural gate that keeps the contract alive as new write paths are added (v1.4.3 W3/W4 feedback infrastructure named in §11).

> **§12 Q2 answer (CSO):** The DOS-670 OCC fix does NOT require commit-on-render. It requires `commit_composition` to compute its own current-version reference. That requirement holds under commit-on-trigger. APPROVED to remove §5.4's commit-on-render branch, conditional on the invalidation contract above.

> **§12 Q1 (cache key shape) — CSO recommendation:** Pick shape (b) version-agnostic. Shape (a) (key on current_db_version) has the lookup race the packet itself flags, and it makes the new invalidation contract harder (the orchestrator must enumerate active versions). Shape (b) is the natural fit for the invalidation contract above.

## Concern 2 — Render-read decharge from `standard_read_composition` budget (§5.5)

**Verdict: LOW — approve shape (a) bypass. Require that the AUTHORIZATION audit event still fires (the rate-limit audit event is the one that goes away under bypass; the auth-success audit event must remain).**

I traced the audit emission split in `surface_client_bridge.authorize` (`bridges/surface_client.rs:380–435`). Two events are involved:
- **`surface_client.rate_limit`** with `decision: "rejected"` — fires ONLY on rejection, via `RateLimitOutcome::Rejected`. Consumed by operational diagnostics only (no security-policy downstream consumer found in my walk).
- **`surface_client.rate_limit`** with `decision: "allowed"` — fires only in the `early_retry` tightening path; ordinary allow does NOT emit. Authorization success is observable via the upstream pairing/HMAC validation audit events plus the request-id propagation, not via a per-allow rate-limit event.

**Bypass impact on audit signal:** under shape (a), local-render reads through paired-loopback skip the rate-budget gate entirely. The downstream consumer of `surface_client.rate_limit (rejected)` is operational, not a security control. Suppressing rejection events for local-render reads removes only the noise the editor's auto-reload loop currently generates (the packet's exact motivation). No security-policy downstream loses input.

**Required amendments (V1.1):**

> **§5.5 — preserve a per-render audit signal:** Bypassing `standard_read_composition` MUST NOT bypass observability. Emit a `surface_client.render_read` audit event at the authorization-success point for the bypassed path, with: `actor`, `composition_id_hash`, `request_id`, `served_from_cache` (filled after cache lookup), `request_class`, `decision: "allowed_local_render"`. Keep this hashed per existing precedent (`stable_hash_for_audit` for any session-attributable id; raw `composition_id` is acceptable since it's a substrate-internal identifier not a credential).

> **§7 add AC #17:** Every local-render bypass emits exactly one `surface_client.render_read` audit event per request. Fixture asserts 1:1 mapping between render route invocation and the new event.

> **§9 add CI invariant #9:** The render route handler MUST call `authorize(...)` BEFORE `cache_lookup(...)` (preserving existing ordering). Grep gate on `project_composition` route body; CI fails if `cache_lookup` appears before `authorize` in source order. (This belongs here, not in concern 4, because §5.5's path change is the realistic refactor that could re-order it.)

> **§12 Q3 answer (CSO):** Shape (a) bypass with new `surface_client.render_read` audit event. Shape (b) "raise the number" is a remote-shaped defense — it keeps a knob that has no policy reason to fire under local-to-local, and it doesn't address the root cause (`standard_read_composition` was sized for an ability class that this read path no longer logically belongs to under local-shape).

## Concern 3 — Typed transport/session error mapping (§5.6) — fingerprinting risk

**Verdict: LOW — approve as drafted.**

I walked the existing error envelope. **The codes `rate_limited`, `session_requires_repair`, `session_not_found`, `runtime_request_failed` are ALREADY in the wire format** (`surface_runtime/mod.rs:737`, `:1474`, `:1488`, `SurfaceHttpError::rate_limited`, `SurfaceHttpError::runtime_unavailable`). §5.6 changes how the WP PHP renderer maps them to user-facing strings; it does NOT add any new signal to the wire. **No fingerprinting channel is created.**

The packet's §12 Q5 framing ("hostile WP plugin co-resident on the same machine") deserves a direct answer under the same-UID local-to-local model:

> A hostile co-resident WP plugin running under the same UID already has ambient access to the keychain (via `security` CLI), the SQLite DB file (via filesystem read), and the runtime's HTTP listener (via loopback). Distinct error codes on the WP plugin's wire give it nothing it doesn't already have via direct DB inspection. Defending against this attacker via error-code merging would be a remote-shaped defense and is explicitly out of scope per the v1.4.2 threat model.

The escalation path Packet A's CSO review traced for the keychain (same-UID local attacker → ambient access → escalation requires user-action) holds identically here. The WP plugin trust principal is the same-UID user, not a separate sandboxed code identity.

**No amendment required.**

## Concern 4 — Authorization-vs-cache-vs-rate-budget ordering

**Verdict: LOW — verified safe as drafted. Codify with a CI invariant (folded into concern 2's amendment).**

I read the ordering at `surface_runtime/mod.rs:2288–2340`:

1. `surface_client_bridge.authorize(...)` — `:2288–2311`. Authorization scope-check AND rate-budget consumption happen TOGETHER inside `authorize()`. They are not separately orderable. On `RateLimited` rejection, the handler returns before cache lookup.
2. `orchestrator.cache_lookup(...)` — `:2317–2340`. Runs only if `authorize()` returned `Ok`.

**Under §5.5 shape (a):** the bypass changes step 1's rate-budget behavior (no consumption for paired-loopback render-class reads). The authorization scope-check MUST remain. The most natural implementation is a new `authorize_render_read(...)` method on the bridge that performs scope checks + identity validation + emits the new `surface_client.render_read` audit event, but does NOT call `limiter.check_and_consume`. The route handler picks `authorize_render_read` vs `authorize` based on the ability's request class.

The ordering invariant is **authorize-before-cache**. As long as that holds, the §5.5 change cannot accidentally let a cache hit bypass authorization. §9 invariant #9 (folded into concern 2) codifies it as a grep gate.

**No additional amendment beyond concern 2's invariant #9.**

## Concern 5 — WP transport trust model under v1.4.3 local-to-local

**Verdict: N/A — confirmed; no amendment.**

For v1.4.3 the WP plugin is a trusted same-UID surface. Signed-loopback HMAC is the authentication boundary; once a request is signed and validated, the WP plugin is treated as a trusted consumer of the response. There is no second-factor sandboxing of the WP plugin. This matches:
- v1.4.2 project description's "local-to-local" framing.
- The keychain ACL story (same-UID processes already have ambient access).
- The memory note `project_v142_wordpress_spike.md` + `feedback_wp_is_local_surface_not_remote.md` ("loopback HTTP not remote; reads don't mutate surface_client_sessions/audit/rate-limits; only feedback writes").

Any defense that treats the WP plugin as a separate trust principal is federation-shaped and belongs in v1.x-federation maintenance, not v1.4.3.

## Concern 6 — Sentinel-redirection vector (Packet A interlock)

**Verdict: LOW — verified disjoint. No interlock hazard.**

I confirmed the code regions:
- **Packet A** (`explicit_sentinel_cleanup()`): `surface_runtime/mod.rs:415–419` (`stop`), `:471–475` (`Drop::drop`), `:818` (`write_runtime_sentinel`), `:887` (`remove_runtime_sentinel`). Tauri lifecycle path.
- **Packet B**: `surface_runtime/mod.rs:2280–2450` (project-composition handler). HTTP request path.

The two paths share the same file but never touch the same lines or the same state machines (lifecycle state vs per-request handler state). Packet A's sentinel-cleanup ordering fix and Packet B's producer-commit removal are orthogonal. They can land in either order; no rebase coordination beyond standard merge.

**DOS-671's "stale port" symptom** is a downstream effect of the broader stabilization story (port discovery races against runtime restart). Packet A's sentinel-cleanup-before-abort is what stops the stale-sentinel-on-restart class. Packet B's render-path changes are what stop the same-runtime-instance read-after-stale class. They address distinct root causes; both are needed; neither subsumes the other.

**No amendment required.**

## L0 closure recommendation

Approve the packet for implementation **after the following amendments are folded into V1.1**:

- **§5.4** — add invalidation contract (orchestrator cache invalidates synchronously on `commit_composition` success, across all scopes_canonical_ids for the `composition_id`)
- **§5.5** — preserve audit signal: new `surface_client.render_read` event at authorization for the bypass path
- **§7 add AC #16** — cache invalidation on commit (unit test)
- **§7 add AC #17** — render-read audit event 1:1 with request (fixture)
- **§9 add CI invariant #8** — grep gate: every `commit_composition` call paired with `invalidate(composition_id)`
- **§9 add CI invariant #9** — grep gate: `authorize` (or `authorize_render_read`) appears before `cache_lookup` in the route handler
- **§12 Q1 resolved** — shape (b) version-agnostic cache key
- **§12 Q2 resolved** — DOS-670 OCC fix does NOT require commit-on-render; removal approved conditional on invalidation contract
- **§12 Q3 resolved** — shape (a) bypass with new audit event
- **§12 Q5 resolved** — no new fingerprinting channel; same-UID WP plugin already has ambient access

None of these expand scope. Concern 1's amendment (synchronous invalidation in `commit_composition`'s success hook) is the substantive one — it converts the packet from "commit-on-read removed, hope signals show up" to "commit-on-read removed, invalidation wired into the only mutator." That contract makes the read-after-stale window bounded by mutation latency, not by the 60s TTL. The other amendments are calibration.

**Cycle gate:** if the 10 items above land in V1.1, CSO returns APPROVE without a second cycle. If the implementing agent disagrees with concern 1's invalidation-in-commit-hook shape (vs a separate post-commit event-bus subscriber), surface as cycle-2 dissent — I'll re-review that specific architectural choice. The hook-in-commit shape is preferred because (a) the only mutator is `commit_composition`, (b) the cache is in-process, (c) no event-bus subscriber exists today that the orchestrator could plug into, and (d) Concern 1's amendment names the structural gate that protects future write paths (feedback, edit affordances) without rediscovering this lesson.

**§12 Q4 (codex reproduction script) and Q6 (auto-save semantics)** are outside CSO scope and unchanged by this review.

**No L6 trigger.** CSO and codex challenge can disagree only if codex argues that commit-on-render is itself the invalidation mechanism that must be preserved. If that argument lands, escalate per the packet's §14 specific-L6-trigger. My read is that commit-on-render is an accidental invalidation mechanism — the v1.4.2 W4-F authors named it as an OCC-token fix, not an invalidation contract — and the right replacement is an explicit invalidation contract, not a defense of the accidental one.

Closing note: the packet correctly identifies that "reads don't write" is the load-bearing local-to-local invariant and pulls the right thread. The hazard it doesn't quite name is that removing a write-on-read pattern in a substrate without a wired invalidation channel can replace one staleness window with another. The amendments above wire the invalidation channel that should have been there from W4-A's inception.
