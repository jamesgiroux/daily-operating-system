---
status: spec:ready
date: 2026-05-10
spike: DOS-546
phase: 0
wave: 2
artifact: 09
related_adrs: [0102, 0111, 0128, 0129, 0130]
open_questions: see ./INDEX.md (routed to W2-D L0 Prep)
---

# 09 — Rate-limit budget matrix

## Summary

The WordPress SurfaceClient path introduces a new abuse surface: a compromised WP plugin, a buggy Gutenberg block, a runaway agent loop through the WP MCP Adapter, or a malicious local process can repeatedly invoke DailyOS abilities through the same loopback transport that legitimate blocks use. The answer is a multi-axis rate-limit matrix enforced in the Rust runtime's `SurfaceClientBridge`, where every request must have budget across the SurfaceClient instance, WP user, WP site, ability name, and scope class before ability dispatch begins.

## Enforcement point

The enforcement point is `SurfaceClientBridge` in the Rust runtime, after loopback transport authentication and request parsing, but before registry lookup completes ability dispatch.

The path is:

1. Loopback endpoint receives the WP plugin request.
2. Endpoint verifies transport shape and signature envelope per artifact 08.
3. Endpoint hands the request to `SurfaceClientBridge`.
4. `SurfaceClientBridge` validates pairing token, site identity, WP user claim, granted scopes, user-presence nonce for writes, and this rate-limit matrix.
5. Only then does the bridge construct `Actor::SurfaceClient { instance, scopes }` and invoke the ability registry.

This layer is the right enforcement point because it can see all dimensions at once:

- The loopback endpoint sees HTTP transport, but should not know ability cost, scope semantics, or per-client policy.
- Individual abilities see their own invocation, but do not know whether the same SurfaceClient, WP user, or WP site is exhausting shared runtime capacity through other abilities.
- The registry enforces `AbilityPolicy.allowed_actors`, `required_scopes`, and `mcp_exposure` per ADR-0102, but it is not the quota ledger for runtime abuse.

The bridge is also the last substrate-controlled boundary before work is allocated. Rate limiting there prevents expensive input validation, provider calls, provenance construction, and service reads from happening when the caller is already outside budget.

## Axes and concrete numbers

All limits use token buckets maintained by the Rust runtime with a monotonic clock. "Burst" is expressed as maximum accepted requests per second when the bucket is full. "Refill-rate" is expressed as sustained requests per minute. Write budgets are intentionally about one tenth of read budgets because writes require stronger trust checks, user-presence proof, audit capture, and feedback-path mutation.

The initial numbers assume a local WordPress Studio site, a small number of active editor sessions, Gutenberg block refreshes that may fan out to several read abilities on page load, and ability costs where most read abilities are cacheable but transform/composition abilities may invoke provider-backed synthesis.

### Axis 1: Per SurfaceClient instance

One SurfaceClient instance represents one paired WP plugin installation. This is the primary containment boundary for plugin compromise and runaway loops.

| Operation class | Sustained limit | Burst | Refill-rate | Reasoning |
|---|---:|---:|---:|---|
| Read | 300 req/min | 20 req/sec | 300 tokens/min | A single WP page can legitimately load 10-20 DailyOS blocks in one second, especially in the editor. Sustained 5 req/sec is enough for active editing and background refresh without letting a loop dominate runtime CPU. |
| Write | 30 req/min | 2 req/sec | 30 tokens/min | Feedback writes are user-gesture bound and should be rare. Thirty per minute covers bulk correction flows while making forged or repeated write attempts visible quickly. |
| Admin | 6 req/min | 1 req/sec | 6 tokens/min | Pairing, token refresh, scope inspection, and recovery actions are operational, not page-render traffic. A lower cap reduces blast radius if an admin endpoint is accidentally exposed through WP code. |

The SurfaceClient bucket is charged for every signed request that reaches the bridge, including rejected ability invocations after signature verification. HMAC failures count against this axis as specified in artifact 08 because a compromised plugin or local attacker can otherwise probe indefinitely for valid envelopes.

### Axis 2: Per WP user

The WP plugin presents a `wp_user_id` claim. The runtime accepts it only when the artifact 08 signature is valid and includes the user id. The rate-limit key stores a keyed hash of `(site_id, wp_user_id)` rather than the raw WP user id.

| Operation class | Sustained limit | Burst | Refill-rate | Reasoning |
|---|---:|---:|---:|---|
| Read | 120 req/min | 8 req/sec | 120 tokens/min | One human editor can trigger a burst of block previews, sidebar refreshes, and MCP-mediated reads. Sustained 2 req/sec is more than enough for a page lifecycle while catching auto-refresh bugs. |
| Write | 12 req/min | 1 req/sec | 12 tokens/min | Real correction, dismissal, corroboration, or contradiction gestures do not occur faster than this for long. This cap catches repeated nonce replay attempts without blocking normal use. |
| Admin | 3 req/min | 1 req/sec | 3 tokens/min | User-scoped admin actions are pairing and scope-management flows. A human might retry a failed pairing, but not dozens of times per minute. |

Unauthenticated requests do not get a WP user bucket because the runtime cannot safely bind the user claim. They are charged to the SurfaceClient instance when the client id is known, or to the loopback endpoint's unauthenticated transport bucket in artifact 15 when it is not.

### Axis 3: Per WP site

One local WordPress runtime may host multiple sites, and a multisite install may register several DailyOS-aware sites against one Rust runtime. The site axis prevents many plugin instances or WP users on one site from aggregating enough traffic to bypass individual limits.

| Operation class | Sustained limit | Burst | Refill-rate | Reasoning |
|---|---:|---:|---:|---|
| Read | 600 req/min | 40 req/sec | 600 tokens/min | Covers several active editors, a front-end preview, and the WP MCP Adapter listing/invoking abilities during development. Sustained 10 req/sec is high for local-first use but bounded enough to protect the runtime. |
| Write | 60 req/min | 4 req/sec | 60 tokens/min | Allows multiple users to submit feedback concurrently while preserving the 1:10 write/read posture. More than one write per second sustained from one site is suspicious for feedback-only writes. |
| Admin | 12 req/min | 2 req/sec | 12 tokens/min | Multisite setup can legitimately perform a short run of registration or pairing checks. Sustained admin traffic beyond this points to a broken sync loop. |

The WP site key is the DailyOS-issued site registration id, not the mutable WP blog id alone. In multisite, each registered site gets a distinct key, and the runtime may also maintain an aggregate install key in Phase 1 if multisite sharing proves noisy.

### Axis 4: Per ability name

The per-ability axis prevents one heavy ability from consuming the full SurfaceClient or site budget. Defaults apply unless `AbilityPolicy.rate_limit` sets a lower value.

| Ability class | Sustained read limit | Sustained write limit | Burst | Refill-rate | Reasoning |
|---|---:|---:|---:|---:|---|
| Cheap read ability | 120 req/min | N/A | 10 req/sec | 120 tokens/min | Lightweight projections and cached claim reads are needed by block-heavy pages. The cap is high enough for page hydration but still catches tight client loops. |
| Standard read or composition ability | 60 req/min | N/A | 5 req/sec | 60 tokens/min | Most block-producing abilities should be cacheable, but may traverse claims, provenance, salience, and render hints. One per second sustained is enough for interactive use. |
| Heavy transform ability | 12 req/min | N/A | 2 req/sec | 12 tokens/min | Synthesis-heavy abilities may call an intelligence provider or assemble large context. Sustained use should be deliberate or backgrounded, not driven by every render. |
| Feedback write ability | N/A | 6 req/min | 1 req/sec | 6 tokens/min | Feedback writes are mutation-like even when product-framed as correction. Six per minute covers deliberate user edits without allowing replay storms. |
| Admin ability | N/A | 3 req/min | 1 req/sec | 3 tokens/min | Pairing and scope-management abilities should never be hot paths. Low limits make abuse and retry loops obvious. |

Ability class is derived from the ability descriptor:

- `Read` category with no provider call and cacheable output: cheap read.
- `Read` or `Transform` category producing `Composition`: standard read or composition unless overridden.
- `Transform` category with provider invocation, long context windows, or external retrieval fanout: heavy transform.
- Feedback-only mutation path per ADR-0128: feedback write.
- Pairing, token, registration, or scope-management ability: admin.

The descriptor must not let an ability classify itself upward to gain budget. It may only use `AbilityPolicy.rate_limit` to lower the default.

### Axis 5: Per scope

Scopes are the ADR-0102 `SurfaceClientScope` grants attached to the paired instance. The scope axis prevents one broad grant such as `read.briefing` or `submit.feedback` from being used as an unbounded channel across many abilities.

| Scope class | Example scopes | Sustained limit | Burst | Refill-rate | Reasoning |
|---|---|---:|---:|---:|---|
| Read scope | `read.account_overview`, `read.briefing`, `read.composition` | 240 req/min | 16 req/sec | 240 tokens/min | Read scopes may back several blocks on one page. Four req/sec sustained is enough for live editing, block refresh, and MCP discovery/invocation bursts. |
| Write scope | `submit.feedback`, `submit.correction`, `submit.dismissal` | 24 req/min | 2 req/sec | 24 tokens/min | Write scopes are user-presence bound. This preserves the 1:10 write/read ratio while allowing short correction sessions. |
| Admin scope | `manage.pairing`, `manage.scopes`, `manage.site_registration` | 6 req/min | 1 req/sec | 6 tokens/min | Admin scopes change trust posture or reveal sensitive registration state. They must not be usable as polling endpoints. |

If an ability requires multiple scopes, every required scope bucket is charged. Example: an ability requiring both `read.account_overview` and `read.briefing` must have budget in both scope buckets. This makes broad composition honest about the grants it consumes.

## Composition rule

A request is allowed only if all applicable buckets have at least one token. The bridge computes the candidate bucket set, checks all buckets without mutation, and consumes one token from every applicable bucket only after all checks pass.

Applicable axes by request type:

| Request type | SurfaceClient axis | WP user axis | WP site axis | Ability axis | Scope axis |
|---|---|---|---|---|---|
| Authenticated read ability | Yes | Yes, if `wp_user_id` claim present | Yes | Yes | Yes, for all `required_scopes` |
| Authenticated transform/composition ability | Yes | Yes, if `wp_user_id` claim present | Yes | Yes | Yes, for all `required_scopes` |
| Feedback write ability | Yes | Yes, required | Yes | Yes | Yes, plus user-presence nonce check from artifact 10 |
| Admin or pairing ability | Yes | Yes, if user-bound | Yes | Yes | Yes, admin scope required |
| Signature-authenticated request with policy rejection | Yes | Yes, if validated user claim present | Yes, if validated site claim present | No, unless ability resolved | No, unless scopes resolved |
| HMAC failure after client id parse | Yes | No | No, unless site id was authenticated earlier in the envelope | No | No |
| User-presence nonce failure | Yes | Yes | Yes | Yes | Separately counted by artifact 10 nonce-failure bucket |

The 429 response reports one axis even if several are exhausted. Precedence is deterministic so clients and tests can rely on it:

1. `surface_client`
2. `wp_site`
3. `wp_user`
4. `scope`
5. `ability`

This precedence reports the broadest containment failure first. For example, if both the SurfaceClient and a heavy ability are exhausted, returning `surface_client` tells the WP plugin that the whole integration should cool down instead of retrying a different ability immediately.

When multiple scope buckets are exhausted, the response uses `axis: "scope"` and includes the first exhausted scope in structured telemetry, not in the public response body. The public body avoids leaking unauthorized scope names to browser-side code.

## Rejection behavior

Rejected requests return HTTP 429:

```json
{
  "error": "rate_limited",
  "axis": "<axis>",
  "retry_after_ms": 1500,
  "request_id": "<id>"
}
```

The response also includes:

```http
Retry-After: 2
```

`retry_after_ms` is computed from the exhausted bucket that won precedence. The `Retry-After` header is rounded up to whole seconds, with a minimum of `1` when the retry interval is positive.

The WP plugin must handle 429 as a non-fatal integration state:

- Surface a non-blocking admin/editor notice tied to the affected block or panel.
- Reuse cached projection output when available.
- Avoid live re-invocation until `Retry-After` has elapsed.
- Keep editor save and page rendering functional even when DailyOS content is stale.
- Do not escalate to repeated admin or pairing checks as a recovery strategy.

For front-end visitor requests, the plugin should prefer silent degradation to cached projection output. For authenticated editor requests, it may show a concise notice with the `request_id` for support.

## Telemetry

Every rate-limit decision emits a structured runtime audit event. For allowed requests, the event is sampled by default at 10% unless debug mode is enabled. For rejected requests, the event is always recorded.

Fields:

| Field | Value |
|---|---|
| `event_type` | `surface_client.rate_limit` |
| `decision` | `allowed` or `rejected` |
| `surface_client_id` | DailyOS-issued SurfaceClient instance id |
| `wp_site_id` | DailyOS-issued site registration id |
| `wp_user_hash` | Keyed hash of `(site_id, wp_user_id)` when authenticated |
| `ability_name` | Resolved ability name, if available |
| `scope_class` | `read`, `write`, or `admin` |
| `required_scopes_hashes` | Keyed hashes of required scope identifiers |
| `axis_exhausted` | `surface_client`, `wp_site`, `wp_user`, `scope`, or `ability` |
| `retry_after_ms` | Retry interval for rejected request |
| `timestamp` | Runtime monotonic-derived wall timestamp for audit display |
| `request_id` | Correlates endpoint, bridge, and ability logs |

The log goes to the runtime audit log described by artifact 15. The loopback endpoint attaches the same `request_id` to transport logs so a 429 can be traced from HTTP receipt through bridge rejection.

Upstream emission:

- A low-trust claim signal may be emitted when a SurfaceClient repeatedly exhausts write or admin buckets because that pattern is evidence about integration health, not user intent.
- No domain claim is emitted for ordinary read throttling.
- If a rejected request would have produced a claim-shaped output, no ADR-0105 provenance envelope is produced because the ability did not run.
- If a write request is accepted and later commits feedback, the normal ADR-0105 provenance path applies to the feedback event's resulting claim lifecycle.

## Per-ability override table

Some abilities need lower limits than their class default. The override mechanism is a new optional field on `AbilityPolicy`:

```rust
pub struct AbilityPolicy {
    // Existing ADR-0102 fields omitted.
    pub rate_limit: Option<AbilityRateLimit>,
}

pub struct AbilityRateLimit {
    pub rpm: u32,
    pub burst: u32,
}
```

`rpm` is sustained requests per minute. `burst` is accepted requests per second when the bucket is full. The override composes with the per-ability axis by minimum wins:

```text
effective_ability_rpm = min(default_ability_class_rpm, policy.rate_limit.rpm)
effective_ability_burst = min(default_ability_class_burst, policy.rate_limit.burst)
```

The override cannot increase budget above the default. It also cannot bypass SurfaceClient, WP user, WP site, or scope buckets.

Initial overrides:

| Ability name | Class default | Override | Effective limit | Reasoning |
|---|---:|---:|---:|---|
| `dailyos/prep-account-briefing` | 12 rpm, 2 req/sec | 6 rpm, 1 req/sec | 6 rpm, 1 req/sec | Long-running briefing assembly may traverse many claims, sources, and salience paths. It should be cached and deliberate. |
| `dailyos/prepare-meeting` | 12 rpm, 2 req/sec | 6 rpm, 1 req/sec | 6 rpm, 1 req/sec | Meeting prep is high-value but provider/context heavy. Page load should use cached projection unless explicitly refreshed. |
| `dailyos/get-account-overview` | 60 rpm, 5 req/sec | 60 rpm, 5 req/sec | 60 rpm, 5 req/sec | Standard composition read. No lower override needed for Phase 0. |
| `dailyos/list-open-loops` | 120 rpm, 10 req/sec | 120 rpm, 10 req/sec | 120 rpm, 10 req/sec | Cheap read used by multiple block surfaces; default cheap read budget is acceptable. |
| `dailyos/submit-claim-feedback` | 6 rpm, 1 req/sec | 6 rpm, 1 req/sec | 6 rpm, 1 req/sec | Feedback write default already matches expected human gesture rate. |
| `dailyos/manage-pairing` | 3 rpm, 1 req/sec | 2 rpm, 1 req/sec | 2 rpm, 1 req/sec | Pairing changes trust state. Retrying faster usually means broken UI or abuse. |

Phase 1 should move these names into the ability-surface inventory once the inventory has canonical ability ids.

## Failure modes

### Clock skew causes token bucket inconsistency

Mitigation: token buckets use the Rust runtime's monotonic clock for refill math. Wall-clock timestamps are used only for audit display. The WP plugin's clock is never trusted for quota refill or retry eligibility.

### SurfaceClient id collision

Mitigation: SurfaceClient ids are issued during ADR-0111 client registration and are bound to pairing material. A caller cannot choose an arbitrary id and gain another client's budget. If duplicate ids are detected during registration recovery, the newer registration is quarantined until the pairing recovery path resolves ownership.

### WP user id forgery

Mitigation: the artifact 08 HMAC signature includes the `wp_user_id`, `wp_site_id`, request body hash, timestamp, and nonce. The runtime does not trust `wp_user_id` unless the signature is valid for the paired SurfaceClient. Invalid or absent user claims cannot obtain a WP user bucket.

### Distributed plugin attempts to multi-home against one runtime

Mitigation: the per-site axis prevents one WP install from multiplying throughput by creating many SurfaceClient instances or using several WP users. In Phase 1, the runtime may add a per-runtime aggregate bucket if local team/multisite use shows that site-level limits are too easy to shard.

### Cache stampede from block hydration

Mitigation: the WP plugin should deduplicate identical in-flight ability requests by cache key, and the bridge should expose `Retry-After` precisely enough for the plugin to back off. Phase 1 should consider a bridge-level singleflight cache for read/composition abilities.

### Retry storm after 429

Mitigation: clients must honor `Retry-After`. The runtime logs clients that retry before the interval as a separate integration-health signal. Repeated early retries may temporarily reduce the SurfaceClient instance bucket by half for a rolling five-minute window.

## Interaction with Wave 2 artifacts

- Artifact 08: HMAC failures count against the per-SurfaceClient axis when the client id can be parsed. This prevents signature probing from being free. A fully unauthenticated malformed request is handled by the loopback endpoint bucket in artifact 15.
- Artifact 10: User-presence nonce failures count against artifact 10's nonce-failure budget and against the normal write request axes when the request is otherwise authenticated. Nonce replay should therefore cool both the write path and the nonce path.
- Artifact 15: The loopback endpoint applies this matrix by delegating authenticated requests to `SurfaceClientBridge` before ability dispatch. Artifact 15 owns transport-level unauthenticated throttles; this artifact owns actor, site, user, ability, and scope throttles.

## Test fixtures

### Fixture 1: Per SurfaceClient exhaustion

- SurfaceClient `sc_wp_1`, site `site_a`, user `u_editor`.
- Send 301 authenticated read requests within 60 seconds to a cheap read ability while keeping WP user, site, ability, and scope below their limits by rotating users and abilities where needed.
- Expected: request 301 returns 429 with `axis: "surface_client"` and positive `retry_after_ms`.

### Fixture 2: Per WP user exhaustion

- SurfaceClient `sc_wp_1`, site `site_a`, user `u_editor`.
- Send 121 authenticated read requests within 60 seconds for the same user, using several cheap read abilities and scopes to avoid ability/scope exhaustion.
- Expected: request 121 returns 429 with `axis: "wp_user"`.

### Fixture 3: Per WP site exhaustion

- Site `site_a` with SurfaceClients `sc_wp_1` and `sc_wp_2`, users `u_1` through `u_10`.
- Send 601 read requests within 60 seconds distributed across clients, users, scopes, and cheap abilities.
- Expected: request 601 returns 429 with `axis: "wp_site"`.

### Fixture 4: Per ability exhaustion

- Ability `dailyos/prep-account-briefing` has override `6 rpm, 1 req/sec`.
- Send 7 authenticated invocations over 60 seconds while all broader buckets have remaining budget.
- Expected: request 7 returns 429 with `axis: "ability"`.

### Fixture 5: Per scope exhaustion

- SurfaceClient has `read.account_overview` and `read.briefing`.
- Send 241 requests requiring `read.account_overview` within 60 seconds, distributed across cheap abilities so no ability bucket exhausts.
- Expected: request 241 returns 429 with `axis: "scope"`.

### Fixture 6: Simultaneous exhaustion precedence

- Arrange SurfaceClient and ability buckets to both be empty before a request.
- Invoke `dailyos/prep-account-briefing`.
- Expected: response reports `axis: "surface_client"` because broad containment precedes ability-specific exhaustion.

### Fixture 7: Burst versus sustained behavior

- For WP user read bucket, send 8 requests in one second.
- Expected: all 8 are allowed when the bucket is full.
- Send a 9th request in the same second.
- Expected: 429 with `axis: "wp_user"`.
- Wait 30 seconds.
- Expected: approximately 60 read tokens refill for that user, allowing the next read burst up to the burst cap.

### Fixture 8: Override application

- Register a heavy transform ability with class default `12 rpm, 2 req/sec` and policy override `{ rpm: 4, burst: 1 }`.
- Send two concurrent requests in one second.
- Expected: first request allowed, second request 429 with `axis: "ability"` and `Retry-After: 1`.

### Fixture 9: Retry-after honored

- Exhaust `submit.feedback` write bucket.
- Capture `retry_after_ms` and `Retry-After`.
- Retry immediately.
- Expected: second request remains 429 and logs early retry.
- Retry after the specified interval.
- Expected: request is allowed if all other buckets have budget.

### Fixture 10: WP plugin graceful degradation

- Simulate a Gutenberg block invoking `dailyos/get-account-overview`.
- Exhaust the scope bucket and return 429.
- Expected plugin behavior: block renders cached projection, editor remains usable, notice is non-blocking, no immediate live re-invoke occurs before `Retry-After`.

### Fixture 11: HMAC failure charged to SurfaceClient

- Send signed-envelope requests with parseable `surface_client_id` but invalid HMAC.
- Repeat until the SurfaceClient read-equivalent abuse bucket is exhausted.
- Expected: subsequent requests return 429 with `axis: "surface_client"` before signature work repeats.

### Fixture 12: User-presence nonce failure composition

- Send authenticated write request with valid HMAC and stale user-presence nonce.
- Expected: request counts against SurfaceClient, WP user, WP site, ability, write scope, and artifact 10 nonce-failure buckets. The response should identify the nonce failure if nonce validation occurs before quota exhaustion; once quota is exhausted, 429 follows this matrix.

## Open questions

1. Should Phase 1 add a per-runtime aggregate bucket above WP site to protect single-user local runtimes from many registered local surfaces?
2. Do front-end anonymous visitor renders need a separate bucket from authenticated editor renders, or should Phase 0 require all DailyOS-backed block reads to come from authenticated WP contexts?
3. Should ability cost be one token per request, or should heavy abilities consume weighted tokens proportional to expected provider/context cost?
4. Should `AbilityPolicy.rate_limit` include separate read/write/admin classes, or is `{ rpm, burst }` sufficient because each ability has one effective operation class?
5. Should repeated 429 early retries become a revocation signal for the SurfaceClient pairing token, or remain an integration-health warning?
6. How should multisite installs with many legitimate users tune site-level limits without weakening the local single-user default?
7. Should runtime audit logs retain hashed scope identifiers only, or may local developer mode store raw scope names for debugging?
8. Should cached projection reads served entirely by the WP plugin count against runtime limits when no loopback request is made? Phase 0 says no, but Phase 1 should validate whether cache freshness checks create an equivalent polling channel.
