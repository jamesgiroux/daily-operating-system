# CSO review — L0 Packet B cycle 2 (WP render stabilization V1.1)

**Verdict: APPROVE**

Reviewer: CSO mode (Claude)
Date: 2026-05-17
Packet: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md` V1.1
Prior review: `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cso.md` (cycle 1 CONDITIONAL APPROVE)
Source SHAs re-read for cycle 2:
- `src-tauri/src/bridges/surface_client.rs:280–435, 740–826` (authorization ordering + candidate construction)
- V1.1 packet §5.1–§5.6, §9, §10, §11, §14 in full

All cycle-1 amendments folded or correctly deferred. The §5.4 reframe — producer-commit
preserved on cache miss — eliminates my cycle-1 MEDIUM by retaining the de-facto
invalidation channel I was worried about. The remaining LOWs land cleanly in either
V1.1 (folded) or maintenance (per CLAUDE.md path-α policy + federation-exclusion).

## Summary table

| # | Validation | Severity | Disposition |
|---|---|---|---|
| 1 | §5.4 reframe: always-current-on-read | LOW residual | APPROVE — single-process serialization closes the trace |
| 2 | §5.5 `charge_ability_scope=false` preserves mandatory checks + identity buckets | OK | APPROVE — verified at `surface_client.rs:301–355, 795–822` |
| 3 | §5.6 typed error mapping under same-UID | OK | APPROVE — no escalation vector |
| 4 | §5.2 `reloadTrigger` derived-string state leak | OK | APPROVE — non-secret, safe |
| 5 | §5.1 wrapper preservation CSRF/auth-bypass | OK | APPROVE — block.json render inherits page auth |
| 6 | Cross-packet interlock with Packet A | OK | APPROVE — disjoint regions, either order |
| 7 | V1.1 deferral classifications | OK | APPROVE — all 6 deferrals correctly remote/federation/operational |

## 1 — §5.4 reframe: always-current-on-read property

Cycle-1 concern: removing producer commit broke the only invalidation channel because
no signal subscriber is wired. V1.1 keeps producer-commit-on-cache-miss, so the
invalidation channel persists.

Trace I walked:
1. State moves upstream (claim dismissal / lifecycle / source freshness) → existing
   producer pathways commit a new `composition_versions` row → `current_db_version`
   advances to N+1.
2. Next render arrives → §5.3 moves the `current_composition_version_for_composition_id`
   read BEFORE cache lookup → key becomes `(actor, composition_id, N+1, scopes)`.
3. No entry exists at N+1 (prior entries were stored at ≤N) → cache miss → producer
   invokes → `commit_composition` succeeds → cache store at
   `projection.composition_version.unwrap_or(current_db_version)` = N+1.
4. Subsequent renders at current_db_version=N+1 hit the cache.

**Read-after-stale window:** bounded by the time between commit and next render's
DB-version read. In single-process local-to-local, that's bounded by mutation
latency, not 60s TTL. Stale entries at version ≤N stay in the cache but are never
hit again — they age out via TTL. No security impact (no privilege leak; they're
just dead memory until TTL collects them).

**Residual concern (LOW, not blocking):** the cycle-1 race between the new
`current_db_version` read and the cache lookup. Packet §5.3 acknowledges this
("for local single-runtime, producer invocations are serialized by the SurfaceClient
bridge's writer mutex"). I verified this is true for the producer path; the read-path
race window exists in theory but the worst case is one extra cache miss + producer
re-run, which simply repopulates at the correct version. No stale serve, no privilege
issue. APPROVE.

## 2 — §5.5 `charge_ability_scope=false` preserves mandatory checks

Verified against `surface_client.rs:292–421`:

- `authorize_for_path` performs in this order before `check_and_consume`:
  1. Descriptor lookup (`:301–317`) — ability must be registered.
  2. Allowed actor + mode + experimental gate (`:319–341`).
  3. `ensure_required_scopes(session, descriptor)` (`:343–354`) — scope check is BEFORE
     rate-limit consumption.
  4. Browser-direct-executable guard (`:355–370`).
- Then `check_and_consume` runs. Inside `candidates()` at `:755–826`:
  - Identity candidates (SurfaceClient `:766`, WpSite `:776`, WpUser `:787`) are
    UNCONDITIONALLY pushed.
  - Ability and Scope candidates are inside `if request.charge_ability_scope` at
    `:795–822`.
- Setting `charge_ability_scope=false` (V1.1 §5.5) bypasses ONLY the ability
  (`standard_read_composition`) and scope (`scope.read`) candidates. Descriptor,
  actor, mode, experimental, scope CHECK (not bucket charge), and browser-direct
  guard run unchanged. Identity buckets still consume.

The packet's claim that "Authorization (descriptor, actor, mode, scope check,
browser-direct-executable guard) remains mandatory" matches the code. The
acceptance criterion #15 + #16 are correctly worded. APPROVE.

## 3 — §5.6 typed error mapping under same-UID local model

11 codes mapped: `rate_limited`, `session_requires_repair`, `session_not_found`,
`runtime_unavailable`, `runtime_request_failed`, `runtime_invalid_json`,
`runtime_http_error`, `projection_tampered`, `projection_version_rollback`,
`stale_composition_watermark`, `consistency_failure` (+ `mid_flight_mutation`,
`missing_expected_claim_version` in the verification-banner arm).

Under same-UID local: a hostile co-resident WP plugin already has ambient access to
keychain (security CLI), DB file (filesystem), loopback HTTP (port). Differentiated
error codes on the wire give it nothing it doesn't already have via direct DB or
process inspection. No escalation vector. The federation/multi-tenant case is
correctly deferred to maintenance ("Bounded error taxonomy for multi-tenant /
multi-plugin WordPress deployments"). APPROVE.

## 4 — §5.2 `reloadTrigger` derived-string state leak

`reloadTrigger = \`${account_id || ''}|${composition_id ? '1' : '0'}\``.

- `account_id` is a substrate-internal identifier, not a credential. Already present
  in editor attributes, visible in DOM.
- `composition_id ? '1' : '0'` reduces composition_id to a presence bit.

Both values are already in the editor's attribute store and any inspector can see
them. No new exfiltration channel. Safe. APPROVE.

## 5 — §5.1 wrapper preservation CSRF/auth-bypass

`dailyos_account_overview_render` stays callable from `render.php` (block.json
front-end render). Per WordPress block render lifecycle, `render_callback` is
invoked by core during page rendering — same auth context as the surrounding
WP_Query (current user's nonce, current user's caps). No separate HTTP entry.
The preview REST route uses its own nonce + capability check (unchanged by this
packet). No new CSRF surface; no auth-bypass path. APPROVE.

## 6 — Cross-packet interlock with Packet A (§10)

Cycle 1 verified disjoint code regions: Packet A at `surface_runtime/mod.rs:415–419,
471–475, 818, 887` (lifecycle / sentinel); Packet B at `:2280–2450` (handler).
V1.1 §10 cross-packet section accurately reflects this. The packet's note that
Packet A landing first adds one error code (`session_requires_repair` from a
transient lookup `Unavailable`) to Packet B's switch table is correctly absorbed
in §5.6's mapping — the code is ALREADY in §5.6's switch arm. Either order works.
APPROVE.

## 7 — V1.1 deferral classifications

Per CLAUDE.md "Path-α L2 findings go to maintenance, not cycle-N+1" + v1.4.2
federation-exclusion threat-model commitment:

| Deferral | Classification | CSO assessment |
|---|---|---|
| Signal-propagation cache invalidation bus | federation (multi-writer) | CORRECT — V1.1 §5.4 reframe makes this moot for local-single-runtime; only required when commits originate from non-local writers |
| Hostile co-resident plugin fingerprinting | federation (multi-tenant) | CORRECT — same-UID trust model precludes this defense from being meaningful locally |
| Render-volume audit signal | operational (not security) | CORRECT — operator observability is not a trust boundary; identity-bucket events still fire for DoS detection |
| ESLint rule authoring | tooling (not security) | CORRECT — grep gates at §9 cover L0 closure |
| Persistent projection storage | federation | CORRECT — only required to enable producer-commit removal, which V1.1 withdraws |
| Hot-path performance budgeting | operational | CORRECT — cache hit rate ≥95% invariant is L0 gate; latency tuning is post-L0 |

Per packet §14 cycle-2 guidance, overturning any of these requires L6. I am
not requesting any overturns. APPROVE.

## L0 closure recommendation

APPROVE cycle 2. All cycle-1 amendments folded or correctly deferred. The §5.4
reframe is the right architectural move — the cycle-1 invalidation contract I
asked for is unnecessary when producer-commit stays in place. Substrate's
always-current-on-read property holds via the §5.3 + preserved-commit chain.

No new findings. No cycle 3 requested. No L6 trigger.

Implementation can begin once the other three cycle-2 reviewers converge.
