# Adversarial L0 Challenge - Packet B render stabilization

## Verdict

BLOCK

Packet B is pointed at the right failure class, but V1.0 is not implementation-ready.
The block is not because render-path commit removal violates DOS-670 by itself; the
block is because the packet does not yet specify a safe read-only projection source
or cache freshness protocol once producer commits move to explicit triggers.

The local-to-local same-UID framing from Packet A still applies, but Packet B touches
a different trust principal boundary: the WordPress plugin is a SurfaceClient, not
the substrate (`.docs/plans/dos-546/v1.4.2-project/01-project-description.md:50`,
`:58-60`). That difference matters for wording and bounded error classes, but it
does not turn local render error codes into a HIGH/CRITICAL fingerprinting issue.

## Findings

- [HIGH] Section 5.4 removes commit-on-render without defining the replacement
  materialization path. Packet B says steady-state render reads existing projected
  state from cache or DB and commits only on explicit refresh, signal invalidation,
  or initial creation (`.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:147-154`).
  Current source has no persisted projected composition body to read from DB:
  composition scope lookup only verifies a `composition_versions` row exists
  (`src-tauri/src/bridges/correction_payload.rs:86-96`), while the current route
  materializes by invoking the producer on cache miss (`src-tauri/src/surface_runtime/mod.rs:2378-2392`).
  The producer itself commits unconditionally through `ctx.services().commit_composition(...)`
  (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-116`), so the packet must
  choose between a read-only producer/projector seam, persisted projection storage,
  or a guarded commit trigger path.

- [HIGH] The editor reload dependency trim creates a stale-closure bug. Packet B
  requires removing `attributes.composition_version` and `attributes.cache_hint_token`
  from the `reload` callback deps while claiming reload can read them at call time
  (`.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:115-118`).
  In the current editor, `reload` reads those values inside the callback payload
  (`wp/dailyos/blocks/account-overview/edit.js:43-58`) and success writes the new
  values through `setAttributes` (`wp/dailyos/blocks/account-overview/edit.js:61-68`).
  If the callback deps become exactly the packet's invariant (`.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:287-289`),
  manual reload after a successful render can keep sending the old version/token
  because the callback closure is not recreated.

- [MEDIUM] Cache option (a) has a real read-then-lookup race unless Packet B adds
  a second freshness check. The current code reads `current_db_version` before
  producer invocation (`src-tauri/src/surface_runtime/mod.rs:2355-2371`), while
  current cache lookup/store key off the caller's version (`src-tauri/src/surface_runtime/mod.rs:2317-2321`,
  `:2425-2431`; `src-tauri/src/services/composition_render_orchestrator.rs:51-56`).
  Concrete race: render R reads current version 7, explicit refresh or signal
  trigger commits version 8, then R looks up and returns cached projection 7;
  the surface now sees a projection that does not match substrate current state.
  That is a bounded concurrent-read stale result, not a DOS-670 OCC failure, but
  Packet B needs an explicit "recheck current version after cache hit" rule if it
  chooses option (a).

- [MEDIUM] Cache option (b) is correct only if invalidation is guaranteed at every
  composition commit boundary, and the packet does not show that propagation path.
  Packet B proposes version-agnostic cache plus explicit invalidation (`.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:131-135`),
  but the commit path lives behind `LiveCompositionCommitter` and the service
  `commit_composition` (`src-tauri/src/services/context.rs:108-136`;
  `src-tauri/src/services/compositions.rs:67-73`). If any future commit path misses
  invalidation, a version-agnostic cache hides the mismatch until TTL expiry.
  For V1.4.3 local stabilization, that is a worse failure mode than a version-keyed
  cache with a recheck.

- [LOW] Render-read decharge does not weaken an audit invariant if it is scoped to
  ability/scope budget consumption, not authorization. The current route authorizes
  before cache lookup and emits bridge audit events returned by authorization
  (`src-tauri/src/surface_runtime/mod.rs:2261-2265`, `:2288-2311`), while normal
  successful rate-limit checks produce no audit row unless they are early retries
  (`src-tauri/src/bridges/surface_client.rs:742-752`). The safe implementation is
  therefore "keep authorize and identity budgets; skip only the `standard_read_composition`
  ability/scope candidate" (`src-tauri/src/bridges/surface_client.rs:766-793`,
  `:795-822`). A full bypass of `check_and_consume` would be broader than Packet B
  needs.

- [LOW] Single-fetch PHP preview does not appear to break a downstream dependency
  on two telemetry events. The double fetch is visible in code: preview calls
  `project_composition_for_surface` first (`wp/dailyos/includes/class-dailyos-plugin.php:587-594`),
  then calls `render_block_with_filter`, which calls `dailyos_account_overview_render`
  and invokes the runtime again (`wp/dailyos/includes/class-dailyos-plugin.php:610-618`,
  `:682-690`; `wp/dailyos/blocks/account-overview/render-functions.php:54-59`).
  The only durable substrate side effect currently tied to producer invocation is
  composition version/outbox mutation (`src-tauri/src/services/compositions.rs:345-366`),
  and the dispatcher consumes each `composition.updated` row independently rather
  than as preview/render pairs (`src-tauri/src/services/version_dispatcher.rs:390-399`,
  `:403-438`). A consumer depending on duplicate rows would be depending on the
  reload-loop bug, not a contract.

- [LOW] Typed error mapping is a fingerprinting channel in the abstract, but not
  a local-to-local blocker if the exposed classes stay bounded. The runtime already
  returns structured error codes in its JSON envelope (`src-tauri/src/surface_runtime/mod.rs:3273-3281`,
  `:3514-3523`), and the PHP client preserves runtime error envelopes on non-2xx
  responses (`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:393-404`).
  The WordPress plugin is a separate SurfaceClient principal, but a hostile
  co-resident plugin with PHP execution can read pairing marker state and hook the
  session-key filter path (`wp/dailyos/includes/transport/class-dailyos-credential-store.php:53-99`,
  `:156-192`). Packet A's Path-alpha precedent supports deferring stronger
  multi-principal hardening under same-UID local scope (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:53-58`,
  `.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-cso.md:11-23`).

## Federation-deferred bucket

- Version-agnostic invalidation with durable fanout is federation-scale unless
  V1.4.3 introduces multiple composition writers or cross-process caches. Local
  single-runtime can use the DB version as the cache epoch and avoid a new
  invalidation bus.

- Typed error fingerprinting becomes material for hosted or multi-tenant WordPress,
  where "another plugin" may be a different administrative principal rather than
  same-user local code execution. File this as bounded error taxonomy and
  co-resident plugin threat modeling for the federation/multi-tenant surface plan.

- Rate-read decharge observability becomes material if DailyOS later needs render
  volume accounting as a security signal. Today normal allowed reads are not audit
  events, so decharging `standard_read_composition` does not remove a durable
  allowed-read trail.

- Audit/telemetry cardinality for duplicate preview/render invocations is a remote
  analytics concern only. The local fix should not preserve duplicate
  `composition.updated` rows to satisfy a hypothetical pair-count consumer.

## Recommendation per concern

1. Render-path producer-commit removal vs DOS-670: do not approve Section 5.4 as
   written. Removing commit-on-render does not break the OCC contract if every
   actual producer commit still forwards the current DB version, matching the
   W4-F comment's narrow purpose (`src-tauri/src/surface_runtime/mod.rs:2348-2353`).
   It does break freshness/materialization unless V1.1 defines a read-only
   projection source and guards the commit-trigger/cache race. Add an explicit
   trigger bit or route mode for "manual refresh commits" vs "render reads".

2. Cache key shape: choose option (a), keyed by `(actor, composition_id,
   current_db_version)`, but add post-hit version revalidation before responding.
   Option (b) is more elegant only after there is a guaranteed invalidation hook
   on every `commit_composition` path; V1.4.3 should not build that bus to fix a
   local render loop.

3. Editor reload guard: keep the auto-reload trigger narrow, but do not make the
   `reload` callback itself stale. Either keep `reload` dependent on
   `composition_version`/`cache_hint_token` and make the effect depend on a separate
   trigger key, or store the latest version/token in refs that are updated every
   render. The packet's exact dep-array invariant should be replaced.

4. Render-read decharge: approve decharging only the ability/scope bucket for
   paired-loopback render reads. Authorization, scope checks, actor allowlist,
   identity buckets, rejection audit, and early-retry audit must remain intact.

5. PHP single-fetch collapse: approve the single-fetch refactor. Add the packet's
   fixture that preview calls runtime exactly once, and add an assertion that the
   returned HTML is rendered from the first response rather than a second
   `dailyos_account_overview_render` runtime call.

6. Typed error mapping: approve bounded typed mapping for local render operability.
   Use coarse classes (`rate_limited`, `session_requires_repair`,
   `session_not_found`, `runtime_request_failed`, `consistency_failure`) and do not
   expose lower-level signing, keychain, or transport internals in editor copy.
   Treat finer fingerprinting analysis as federation-deferred, not a Packet B L0
   blocker.
