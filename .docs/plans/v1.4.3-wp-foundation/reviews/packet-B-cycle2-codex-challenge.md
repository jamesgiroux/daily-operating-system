# Cycle-2 adversarial L0 review - Packet B render stabilization V1.1

## 1. Verdict

CONDITIONAL APPROVE

## 2. Per-question validation

1. **PASS** - §5.4 resolves cycle-1 HIGH #1 by withdrawing producer-commit removal.
   Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:377-385` says "V1.1 withdraws that proposal" and "The producer commit IS the materialization step".
   Cycle-1 blocked because V1.0 removed the materialization path at `.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-codex-challenge.md:20-31`.
   Loop check: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:387-397` says cache hits dominate after the first current-version miss, so the producer remains on the miss path without firing on every steady-state render.

2. **CONCERN** - Option (a) invalidates correctly after `composition_versions` advances, but the packet should make that boundary explicit.
   Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:342-356` moves the DB-version read before cache lookup and stores by `projection.composition_version.unwrap_or(current_db_version)`.
   Concern: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:394-397` says "Producer commits naturally fire when account state actually moves"; that is correct only once account-state movement has reached a producer commit.
   Source cross-check: `src-tauri/src/services/compositions.rs:149-175` reads current composition version from `composition_versions`, and `src-tauri/src/services/compositions.rs:270-372` advances that row through `commit_composition`.

3. **PASS** - §5.2 fixes the stale-closure bug without reintroducing the auto-reload loop.
   Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:293-301` keeps the full `reload` dep list and moves the effect to `[reloadTrigger]`; `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:304-310` says the trigger ignores version/token writes while manual reload uses the live closure.
   HYPOTHESIS (React): primitive string deps compare by value, so a freshly rebuilt but equal `reloadTrigger` does not schedule the effect.
   Manual-button cross-check: the current editor wires `onClick: reload` at `wp/dailyos/blocks/account-overview/edit.js:133-139`, so V1.1's full dep list gives the button the latest callback on each render.

4. **PASS** - `charge_ability_scope=false` preserves identity-bucket consumption and only removes ability/scope budget charging.
   Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:418-423` says identity buckets continue while ability/scope buckets are bypassed; `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:425-429` preserves descriptor, actor/mode, scope, and browser-direct checks.
   Source cross-check: `src-tauri/src/bridges/surface_client.rs:795-823` gates only ability/scope candidates on `charge_ability_scope`, while `src-tauri/src/bridges/surface_client.rs:766-793` always builds surface/site/user candidates.
   DoS check: this preserves the surface/site/user volume gate while removing the ability/scope tighten-event channel described at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:439-449`.

5. **PASS** - The V1.1 deferrals are correctly classified as remote/federation or maintenance, subject to the LOW clarification below.
   Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:149-171` lists the first four V1.1 deferrals; `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:717-741` lists the §11 not-owned deferrals.
   I am not overturning any federation-aligned deferral; `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:782-788` would require L6 for that.
   Scope check: `.docs/plans/dos-546/v1.4.2-project/01-project-description.md:43-50` frames WordPress as a local SurfaceClient, with runtime authority retained by DailyOS.

## 3. New findings

- **LOW** `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:394-397`
  Evidence quote: "Producer commits naturally fire when account state actually moves (signal propagation triggers a new producer invocation through other paths, that invocation commits a new version, which invalidates the cache via the cache-key-by-current-db-version pattern)."
  Why it matters: the cache proof is exact for composition-version movement, but the phrase "account state actually moves" can be read as claim/source movement before recomposition. The W4-A0 packet separately says "Cache-bust marks the cached projection stale before any renderer fetch can reuse it as current" and "Recomposition re-invokes W4-A0" at `.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md:453-455`.
  Recommended action: add one sentence to §5.3/§5.4: "For Packet B, state movement means `composition_versions` has advanced through `commit_composition`; upstream claim/source movement reaches this cache through the existing DOS-589/W4-A0 recomposition contract."
  Recommended action: add one fixture beside §8 #6-#8 proving an external composition-version advance invalidates the old cache key before a render response is served.

## 4. Deferral classification check

- **Signal-propagation cache invalidation bus** - justified under federation exclusion.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:717-721` says "producer-commit-on-cache-miss" is the local invalidation channel and explicit orchestrator invalidation is for "Federation / multi-writer deployments".
  Classification: justified if the small clarification above pins "state movement" to committed composition-version movement.

- **Hostile co-resident WordPress plugin fingerprinting** - justified under same-UID local model.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:722-727` says a co-resident plugin with PHP execution already has "ambient keychain/DB/loopback access".
  Classification: remote/multi-tenant defense, not local-shipping.

- **Render-volume audit signal** - justified as operator observability, not a local DoS gate.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:728-731` says local-render-decharge volume observability "isn't security-load-bearing locally".
  Classification: maintenance is fine because identity buckets remain consumed per §5.5.

- **ESLint rule authoring** - justified as tooling hardening, not L0 correctness.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:732-734` says "grep gates cover L0 closure; a proper ESLint rule is maintenance."
  Classification: justified because §9 invariants #2-#3 at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:657-658` gate the exact shapes.

- **Persistent projection storage** - justified under federation/multi-writer exclusion.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:735-738` says it would enable producer-commit removal and is "substantial new infrastructure".
  Classification: justified because V1.1 withdraws producer-commit removal at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:405-408`.

- **Hot-path performance budgeting** - justified as operational tuning.
  Packet quote: `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:739-741` says cache hit rate >=95% is the L0 target and latency p95 budgeting is "operational tuning".
  Classification: justified because §9 invariant #5 at `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:660` already gates the local hot-path behavior.

## 5. CONDITIONAL APPROVE edits

1. Add the §5.3/§5.4 clarification that Packet B's cache invalidation proof starts from committed `composition_versions` movement, while claim/source movement reaches that state through the existing W4-A0/DOS-589 recomposition contract.

2. Add one §8 Rust fixture: seed cache at version N, advance current DB composition version outside the request watermark, render with stale `request.composition_version=N`, assert lookup misses old key, producer path runs, cache is stored at the emitted version, and the following render hits.

## 6. BLOCK findings

None. Cycle-1 HIGH #1 is closed because §5.4 no longer removes producer commit. Cycle-1 HIGH #2 is closed because §5.2 no longer trims the `reload` callback deps. Cycle-1 MEDIUM option (b) is moot because V1.1 picks option (a). Cycle-1 MEDIUM option (a) is reduced to the LOW clarification/test above.

## 7. L6 escalation

None. I am not overturning a federation-aligned deferral under `.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:782-788`.
