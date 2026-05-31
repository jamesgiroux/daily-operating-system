# Packet B — L2 CSO security review

**Branch:** `dos-671-672-render-stabilization` (3 commits since `a594cd4d`)
**Anchor:** L0 Packet B V1.1.1 (L0-closed; cycle-2 CSO unanimous APPROVE)
**Threat model:** local-to-local, same-UID; v1.4.3 WP surface is loopback to Tauri runtime on the same machine.
**Scope:** AC-adjacent. Path-α → Linear maintenance `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. Not a BLOCK.

## Verdict: APPROVE

All trust-boundary invariants from L0 §5.5 / §11 are preserved. The §5.3 cache-key correction does not introduce a new authorization-bypass surface: authorization runs strictly before cache lookup, and the lookup key now refers to substrate-owned state (`current_db_version`) rather than client-supplied watermark. The §5.5 `authorize_local_render` variant is a surgical decharge of the *rate-limit buckets only* for ability + scope axes — every mandatory check (descriptor lookup, actor/mode/experimental gate, required-scope CHECK, browser-direct guard) is preserved. The §5.6 typed-error switch maps codes already on the wire and is renderer-side only; no new attack surface under same-UID local model. No remote-shape defenses reintroduced.

---

## Trust-boundary checks against L0 plan

| # | Invariant | Status |
|---|---|---|
| 1 | §5.4 reframe preserved: producer commit on cache miss is KEPT | OK — `surface_runtime/mod.rs:2389-2410` invokes the W4-F producer on cache miss; `commit_composition` semantics in the producer are untouched. No removal of producer state-advance. |
| 2 | Cache lookup keyed by `current_db_version`, not `request.composition_version` | OK — `mod.rs:2336-2352` reads `current_composition_version_for_composition_id` from substrate; `:2361` uses `current_db_version`. Grep-gate test `dos671_project_composition_cache_lookup_current_db_version_grep_gate` enforces both positive and negative assertion. |
| 3 | Authorization runs BEFORE cache lookup | OK — `authorize_local_render` is invoked at `mod.rs:2304-2327` (rejects on `RateLimited` / `ScopeDenied` / `AbilityUnavailable` with audit emission); cache lookup at `:2360-2381` is only reachable past the early-returns. A cache hit cannot bypass auth. |
| 4 | `authorize_local_render` preserves scope CHECK; only scope/ability BUCKETS bypassed | OK — `bridges/surface_client.rs:277-291` delegates to `authorize_for_path` with `charge_ability_scope=false`. `authorize_for_path:363-371` runs `ensure_required_scopes` unconditionally and short-circuits on failure with `ScopeDenied`. Descriptor lookup (`:321`), allowed_actors / allowed_modes / experimental (`:338`), `BrowserDirectJs` policy.client_side_executable guard (`:374`) all unconditional. Test `authorize_local_render_enforces_required_scope` proves rejection on insufficient scope. |
| 5 | Identity buckets still consumed | OK — rate limiter `candidates()` at `surface_client.rs:780-813` adds SurfaceClient / WpSite / WpUser candidates unconditionally; only the `Scope` and `Ability` axis candidates at `:815-843` are gated by `charge_ability_scope`. Test `authorize_local_render_still_charges_identity_buckets` proves SurfaceClient throttle still trips with budget=1. |
| 6 | §5.6 typed error switch introduces no new attack vector | OK — all 30 error codes mapped (`render-functions.php:85-141`) are already emitted by `SurfaceHttpError` and observable to any client of the loopback runtime. The switch is renderer mapping only: it does not branch on payload contents, does not leak server internals beyond the existing taxonomy, and does not change which codes are wire-visible. Hostile co-resident plugin fingerprinting is the V1.1 deferred maintenance item, which is correct for local-to-local. |
| 7 | Editor `reloadTrigger` contains no secrets | OK — `edit.js:96` derives `reloadTrigger = "${account_id || ''}|${composition_id ? '1' : '0'}"`. Account ID is a user-typed identifier already visible in the editor UI, sidebar inspector, and DOM attributes; composition presence is a single bit. No tokens, session IDs, claim IDs, scopes, or watermarks appear. |
| 8 | `render.php` front-end path inherits page auth context; no new CSRF surface | OK — `wp/dailyos/blocks/account-overview/render.php` calls the wrapper `dailyos_account_overview_render($attributes)` exactly as before. The wrapper now delegates internally to `fetch_projection` → `render_from_projection`, but the entrypoint and the trust context (page render under normal WP capability checks) are unchanged. The REST preview route at `class-dailyos-plugin.php:587-621` already enforces `current_user_can('edit_posts')` upstream; this commit changes how it renders the projection, not who can call it. |

## Deferred items (V1.1) — still DEFERRED in implementation

Verified by diff inspection: none of these were silently reintroduced.

- **Signal-propagation cache invalidation bus** — not present; invalidation continues to flow through producer commits advancing `current_db_version`.
- **Hostile co-resident plugin error-code fingerprinting** — error taxonomy unchanged from `SurfaceHttpError`; no redaction layer added.
- **Render-volume audit signal** — no new audit-event types emitted from the decharge path; existing identity-bucket audits are preserved through `audit_events` returned from the limiter.
- **ESLint authoring** — `edit.js:99-101` uses `eslint-disable-next-line react-hooks/exhaustive-deps` with rationale comment; grep gates enforce the pattern.
- **Persistent projection storage** — in-memory `CompositionRenderOrchestrator` unchanged.
- **Perf budgeting** — no new perf budget assertions; behavioral fixtures cover the warm-path producer-skip claim.

## Cross-packet interlock (Packet A ↔ Packet B)

The L0 packet description called the regions disjoint. Empirically there is one **textual** overlap inside `surface_project_composition_response` (the cache_lookup call site):

- **Packet A** reformats lines 2317-2322 as a rustfmt-only cosmetic change (compresses multi-line call onto a single line; same arguments).
- **Packet B** rewrites the same call to use `current_db_version` and adds the upstream `db_read` for that value.

Either landing order works. Whichever lands second produces a trivial textual conflict that resolves to Packet B's semantics (Packet A's hunk becomes a no-op because the new shape no longer matches rustfmt's "long form" trigger). No security risk — both edits preserve the same authorization-precedes-cache invariant.

## Path-α findings (file to maintenance, not BLOCK)

1. **§9 invariant #6 grep gate is one-sided.** The test `dos672_project_composition_route_uses_local_render_authorization_grep_gate` asserts the route uses `authorize_local_render` and does NOT call `authorize`. It does not assert that `authorize_local_render` itself passes `charge_ability_scope=false` to `authorize_for_path` (an attacker editing `surface_client.rs:285` could silently flip the bool back to `true` and still pass the route-level grep gate). Behavioral test `authorize_local_render_does_not_charge_ability_or_scope_buckets` covers the regression, but a literal grep gate inside `bridges/surface_client.rs` would harden the boundary.
   → Linear maintenance: "Add grep gate for authorize_local_render charge_ability_scope=false delegation in surface_client.rs"

2. **`render_from_projection` fallback branches.** The `default:` arm and `consistency_failure` arm both fall through to `dailyos_account_overview_render_verification_banner()` (`render-functions.php:139-141`). This is correct fail-safe behavior, but the third call site at `:143` (`is_array($projection)` defensive guard for malformed success envelopes) is also reachable. The verification banner is reserved language with claim-trust semantics; using it for "the runtime returned a non-array projection" muddles the trust-band signal. Either route this case to `RuntimeUnavailableNotice` or clarify §9 #7 to include the shape-mismatch fail-safe.
   → Linear maintenance: "Disambiguate verification banner vs runtime-unavailable notice for malformed projection envelope"

## Out-of-scope changes (rebase artifacts, not security-relevant)

Three files in the diff are enabling changes for validation gates on this worktree, not part of the render-stabilization work:

- `src-tauri/scripts/check_claim_writer_allowlist.sh` — adds `W6-A-meta-N/state.sql` to the allowlist regex (extension of the existing pattern; same authorization model).
- `src-tauri/src/migrations/v178_dos_285_linear_issue_state.rs` — adds an `if !table_exists` guard so v178 is idempotent on DBs without `linear_issues`. Defensive; can only narrow what the migration does.
- `src-tauri/src/services/linear_issue_signals.rs` — routes signal emission through `services::signals::emit_and_propagate` instead of `signals::bus::emit_signal_and_propagate`. Service-facade compliance fix, not a trust-boundary change.

None of these affect the surface trust boundary, claim authorization, or runtime auth path. Noted for completeness; not flagged.

## Files referenced

- `/private/tmp/dailyos-pb/src-tauri/src/surface_runtime/mod.rs:2275-2473` (project-composition route)
- `/private/tmp/dailyos-pb/src-tauri/src/bridges/surface_client.rs:270-440,815-843` (authorize_local_render + bucket gating)
- `/private/tmp/dailyos-pb/wp/dailyos/blocks/account-overview/render-functions.php:32-141` (split + typed error switch)
- `/private/tmp/dailyos-pb/wp/dailyos/blocks/account-overview/render.php` (front-end entrypoint, unchanged shape)
- `/private/tmp/dailyos-pb/wp/dailyos/blocks/account-overview/edit.js:81-102` (reloadTrigger)
- `/private/tmp/dailyos-pb/wp/dailyos/includes/class-dailyos-plugin.php:587-625` (preview route uses render_from_projection)
- `/private/tmp/dailyos-pb/.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md` V1.1.1
