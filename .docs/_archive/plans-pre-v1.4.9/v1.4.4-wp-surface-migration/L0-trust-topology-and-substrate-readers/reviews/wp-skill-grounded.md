# WP-skill-grounded reviewer — L0 trust-topology + substrate-readers

**Reviewer:** Claude Opus 4.7, with `wp-plugin-development` + `wp-block-themes` + `wp-wpcli-and-ops` skills loaded.
**Scope:** §3a `signed_post()` → `local_post()` cutover for first-party WP loopback (DOS-761); §3b/3c are out of scope for this panel.
**Verdict:** **APPROVE with two MEDIUM findings folded into PR B AC.** No BLOCKers. Pairing handshake, activation, WP-CLI, multisite, and Studio/WP-Now sandbox concerns all hold under the proposed cutover, with the small caveats below.

---

## Q1 — Pairing handshake preservation

**Finding: PASS.** The packet's plan to "retain `signed_post()` for the pairing handshake only" is more nuanced than §3a names, but the nuance lines up. There are **two distinct flows that must not collapse into each other**:

| Flow | Method on client | Route | Identity | Decision under §3a |
|------|------------------|-------|----------|--------------------|
| Pairing handshake | `handshake()` → `plain_post()` | `/v1/pairing/handshake` | wp_context body only; **no HMAC, no session, no marker** | UNCHANGED — already not signed |
| Per-invoke ability call | `invoke_ability()` → `signed_post()` | `/v1/surface/invoke` | full 16-header HMAC | CUT OVER to `local_post()` → `/v1/local/invoke` |
| Nonce issue / verify | `issue_nonce()` / `verify_nonce()` → `signed_post()` | `/v1/surface/nonce/*` | full 16-header HMAC | **AMBIGUOUS — see HIGH below** |
| Scope refresh | `refresh_pairing_scopes()` → `signed_post()` | `/v1/surface/pairing/refresh-scopes` | full 16-header HMAC | **AMBIGUOUS — see HIGH below** |
| `project_composition_for_surface()` | `signed_post()` | `/v1/surface/project-composition` | full 16-header HMAC | **AMBIGUOUS — see HIGH below** |

`wp/dailyos/includes/transport/class-dailyos-runtime-client.php` actually has **five** `signed_post()` call sites (`:103, :142, :158, :174, :203`), not one. The packet's §3a "WP `local_post()` method + cut over all invoke call sites" reads ambiguously against this.

### HIGH-1 — Specify per-call-site routing in PR B AC
**Where:** packet §3a + §4 DOS-761 AC.
**What:** AC line 4 says "routes all ability-invoke call sites through it" — but `signed_post()` is also called by `issue_nonce`, `verify_nonce`, `refresh_pairing_scopes`, and `project_composition_for_surface`. Each needs an explicit routing decision:

- `invoke_ability()` (`:103`) → `local_post()` → `/v1/local/invoke`. Clear.
- `project_composition_for_surface()` (`:142`) → also `local_post()`. Same trust column; same Actor::User shape; same scope-filter happens runtime-side regardless. Used by trust-band-badge, status-dot, avatar, health-badge, freshness-indicator, provenance-tag, score-band (7 render-functions call sites). **Recommend: add a sibling local-route `/v1/local/project-composition`.**
- `issue_nonce()` / `verify_nonce()` (`:158, :174`) — runtime-issued user-presence nonce used by confirmation-attestation for destructive ops. This is the confirmation-token path, NOT pairing. Under `Actor::User` materialization, the runtime can mint and verify these without HMAC because the caller is already trusted-as-User. **Recommend: route to local channel.** Alternatively, keep on signed channel if the nonce ledger gate has any structural dependency on session-bound identity — flag for L1 author audit.
- `refresh_pairing_scopes()` (`:203`) — this IS pairing-adjacent (it re-grants scopes against DEFAULT_GRANTED_SCOPES). DOS-746 added this 2026-05-19. Under the new model, "scope refresh" loses most of its meaning because Actor::User has all scopes by definition. **Recommend: noop or remove from invoke-path; if kept for the WP MCP path's SurfaceClient view, keep on signed channel.**
- `handshake()` → `plain_post()` (`:71`) — already unsigned; no change needed.

Add an explicit per-call-site decision table to §3a or to PR B's PR description.

### HIGH-2 — Pairing handshake STILL needs port discovery
The packet correctly says "WP marker/sentinel flow unchanged (Tauri port discovery still works)" (§4 AC line 5), but the sentinel/marker dance is not transport-ceremony — it's discovery, and it's load-bearing for `/v1/local/invoke` too. `local_post()` will need `discover_runtime_base_url()` exactly the way `signed_post()` does (`runtime_client.php:537`). Confirm in PR B description that `local_post()` reuses the existing sentinel discovery path (recommend factoring `runtime_base_url_for_signed_request()` body out to a shared helper, since the auth-strip is what differs, not the URL resolution).

---

## Q2 — Activation / deactivation / uninstall hooks

**Finding: PASS.**

`wp/dailyos/dailyos.php:61–63` registers `activate`/`deactivate`/`uninstall` against `DailyOS_Activation` (`includes/class-dailyos-activation.php`). I read the full file. **None of the three lifecycle handlers call the runtime client.**

- `activate()` (`:30`) — runs `assert_environment()` (PHP 8.1 / WP 6.9), namespace-vacancy check on `dailyos_*` options, `complete_activation()` which ensures the `dailyos_substrate` WP user, schedules `dailyos_nonce_sweep` cron, calls `DailyOS_Plugin::instance()->register_post_types()`, then `flush_rewrite_rules()`. **Zero HTTP to runtime.**
- `deactivate()` (`:62`) — clears the `dailyos_nonce_sweep` cron, deletes DailyOS transients, flushes rewrite rules. **Zero HTTP.**
- `uninstall()` (`:78`) — deletes the `dailyos_substrate` user + namespace reset + revokes MCP role. **Zero HTTP.**

**"Plugin activates without having paired yet" path:** `complete_activation()` sets `dailyos_pairing_status = 'needs_pairing'` and exits. The runtime client is only constructed at render time via the `dailyos_runtime_client_for_block` filter (`includes/class-dailyos-plugin.php:109`). `signed_post()` returns `not_paired_error()` immediately when the credential store has no marker (`runtime_client.php:217–219`). Block renders see `WP_Error` → empty chip path. Same behavior under `/v1/local/invoke` — `local_post()` should also short-circuit when no marker exists (sentinel discovery returns null → return same `not_paired_error`). **AC suggestion: PR B's `local_post()` must preserve the not-paired-returns-WP_Error contract** so render-functions' `is_wp_error()` checks (e.g. account-detail/render-functions.php:77, project-detail/render-functions.php:84, all 24+ inner blocks) keep working unchanged.

### LOW-1 — Activation runs `register_post_types()` before pairing
Not a new issue under this packet; flagging only because the packet touches lifecycle thinking. `DailyOS_Plugin::instance()->register_post_types()` is called from `complete_activation()` before any pairing exists. CPTs (`dailyos_account`, `dailyos_briefing`, etc.) register fine — they're metadata, not runtime-backed — but any block in the CPT default-template will render an empty chip on first activation until pairing completes. Already true today; cutover doesn't make it worse. No action.

---

## Q3 — WP-CLI surfaces

**Finding: PASS.**

`wp/dailyos/includes/cli/class-dailyos-cli.php` registers `wp dailyos` with three subcommands: `status`, `repair_namespace`, `repair_projection`. **None invoke the runtime client.**

- `status` reads `dailyos_pairing_status` + `dailyos_pairing_marker` options. Local DB only.
- `repair_namespace` reads namespace report; with `--execute` writes `dailyos_pairing_status = 'needs_pairing'`. Local options only.
- `repair_projection` is a stub ("No projection rows exist in this scaffold").

Bootstrap at `class-dailyos-plugin.php:121–123` registers CLI only when `defined('WP_CLI') && WP_CLI`. The CLI surface is **observation + local-state repair**, not runtime invocation. Cutover does not affect WP-CLI.

### LOW-2 — Consider adding `wp dailyos invoke <ability>` for L4
Out of packet scope, but useful for L4 verification: a thin WP-CLI command that calls `local_post()` directly with `--ability=X --input='{}'` would let `wp dailyos invoke get_daily_briefing` reproduce a chip-render envelope without needing to drive the editor. Files as a maintenance-bucket ticket, not a blocker. The §6.4 reviewer (plan-design-review) already covers chip rendering separately.

---

## Q4 — Multisite consideration

**Finding: PASS with a note.**

The current signed path sends `X-DailyOS-Multisite-Blog-Id` conditionally (`runtime_client.php:276–278`). The runtime side reads it into `RequestIdentity.multisite_blog_id: Option<String>` (`surface_runtime/hmac.rs:557, 695, 722`) and threads it into `SurfacePairingValidationParts` (`services/surface_pairing.rs:1802, 1842–1844`) and the HMAC canonical block (`surface_runtime/hmac.rs:921–922`). It is also used inside `surface_pairing.rs:1862` to gate `if let Some(blog_id) = &self.multisite_blog_id { ... }`.

I read every consumer. **The runtime never infers blog context from the header for substrate routing.** It uses `multisite_blog_id` purely for (a) HMAC canonical signing surface and (b) pairing-marker validation (the paired blog must match the requesting blog). Both concerns dissolve under `/v1/local/invoke`:

- No HMAC → not in canonical block.
- No SurfaceClient session → no paired-blog-binding to validate.

DailyOS is a single-tenant personal-tier app. The substrate DB at `~/.dailyos/dailyos.db` is per-OS-user, not per-blog. Dropping the multisite header on the loopback path is **safe and correct** — there is no code path that would mis-route on blog context.

### MED-1 — Document the multisite invariant
Add to §3a or §8: *"Under `/v1/local/invoke`, the substrate operates on the single per-OS-user DailyOS DB regardless of which WP blog the call originated from. Multisite blog identity does not factor into substrate routing under the first-party trust column. WP-mediated MCP path (v1.4.2 W3-C) retains the signed channel where blog identity may matter for cross-tenant safety."*

This is also worth a `docs/solutions/` entry — the next reviewer to touch this code may not know the multisite header is decorative-only on the substrate side.

---

## Q5 — WP-Now / Studio sandbox port discovery

**Finding: PASS — sandbox is already handled, no gap.**

The handoff §1.3 reference to `getenv('HOME')` returning `/home/web_user` in WP-Now describes the **wasm-PHP sandbox** scenario. Verified against actual code:

- `class-dailyos-plugin.php:755` — `runtime_endpoint_sentinel_path()` reads `getenv('HOME')` and returns `null` if HOME is unavailable or blank.
- `:680` — `discover_runtime_endpoint()` calls `posix_geteuid()` **conditionally** via `function_exists()` — if `posix_*` is unavailable (as in WP-Now wasm), the ownership check is skipped, not erroring.
- `:674–678` — mode check uses `stat()` (universally available).
- `:686` — `file_get_contents()` (universally available).

For Studio (native macOS PHP), `HOME=/Users/jamesgiroux`, sentinel at `/Users/jamesgiroux/.dailyos/runtime-endpoint.json` — works today.

For WP-Now wasm with `HOME=/home/web_user`, sentinel would resolve to `/home/web_user/.dailyos/runtime-endpoint.json` which does not exist in the sandbox. `discover_runtime_endpoint()` returns `null`. Pairing-by-URL still works (the URL carries `port=` in its query, parsed by `runtime_base_url_for_pairing()`:574–580, no sentinel needed). **Per-invoke calls fail closed with `not_paired_error`** because both `signed_post()` and any future `local_post()` resolve their base URL via `runtime_base_url_for_signed_request()` which prefers the sentinel.

**No new gap introduced by this cutover.** The constraint is pre-existing: in WP-Now, the user pairs via URL (works), but subsequent invokes fail because the sentinel is unreadable from inside the wasm sandbox. This is orthogonal to trust-topology and lives wherever WP-Now → Tauri loopback gets first-class support.

### LOW-3 — Surface the WP-Now constraint in §8
Add to §8 NOT-doing list: *"WP-Now wasm sandbox loopback discovery is unchanged. WP-Now installs that paired via URL will still hit `not_paired_error` on invoke if the sentinel is unreachable from `/home/web_user/.dailyos/`. Studio (native PHP) is unaffected. Cross-sandbox sentinel access is its own work item."*

This protects against a future L4 round assuming the cutover should have unblocked WP-Now.

---

## Summary verdict

**APPROVE.** The packet's first-party trust column is correct under WordPress's plugin lifecycle, WP-CLI surface, multisite header semantics, and WP-Now sandbox constraints. The cutover preserves the pairing handshake correctly because pairing was already on a separate unsigned channel (`/v1/pairing/handshake` via `plain_post()`).

**Fold into PR B AC before opening PR:**
- **HIGH-1** — explicit per-call-site routing decision for the 5 `signed_post()` callers (invoke / project-composition / issue_nonce / verify_nonce / refresh_pairing_scopes); avoid the ambiguous phrasing "all invoke call sites."
- **HIGH-2** — confirm `local_post()` reuses sentinel discovery (factor `runtime_base_url_for_signed_request()` body into a shared helper); preserve the `not_paired_error` WP_Error contract so block renders short-circuit correctly.
- **MED-1** — document the multisite invariant in §3a or §8 + capture as a `docs/solutions/` entry.

**LOW** items are nice-to-have, file as maintenance tickets:
- LOW-1: activation-before-pairing chip behavior already correct; no action.
- LOW-2: `wp dailyos invoke` CLI helper for L4 reproducibility.
- LOW-3: surface WP-Now sandbox constraint in §8 NOT-doing.

No BLOCKers. No CSO-overlap territory (CSO panel covers the threat-model side; this panel grounded in WP-plugin-lifecycle / WP-CLI / multisite / sandbox mechanics).
