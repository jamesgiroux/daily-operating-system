# W3 L3 re-run — Suite P (Performance)

Date: 2026-05-13
Reviewer: performance-engineer (subagent)
Worktree: `/private/tmp/dailyos-w3-0`
Branch: `dos-546-w3-wordpress-foundation` @ `eae1c267`
Parent: `dev@dd003ee2` (W2 merged via PR #270)
Diff: `git diff dd003ee2..HEAD`
Acceptance contract: `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`
Maintenance project: `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`

---

## 1. VERDICT

**APPROVE**

W3 fold did not introduce a new performance regression. The 5 fold fixes are AC-bound corrections; each has bounded, justified, sub-millisecond per-request cost relative to the network round-trip (~5s timeout) they accompany. No hot-path delta warrants blocking. Prior path-α items remain present and remain maintenance-only.

---

## 2. Hot-path delta analysis

Scope of W3 wave: additive-only WordPress plugin under `wp/dailyos/`. Zero Rust substrate, frontend, or schema changes (`git diff dd003ee2..HEAD --name-only` outside `wp/dailyos/` and `.docs/` returns empty). Substrate-side hot paths are therefore unchanged by W3.

Per-signed-request cost on the WP side (signed_post in `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:143-227`):

| Operation | Cost class | Fold delta |
|---|---|---|
| `get_marker()` (signed_post:144) | 1 wp_options read | unchanged |
| `runtime_base_url_for_signed_request($marker)` | in-memory string ops | new in fold (FIX 1) — negligible |
| `canonical_identity($marker)` | in-memory array assembly | new in fold (FIX 4) — negligible |
| `retrieve_session_key()` / `retrieve_hmac_key()` | filter calls | unchanged |
| `signer->sign_request()` → `canonical_bytes()` 15-field | 15× `strlen + concat` + `preg_match //u` per field + 1 `hmac_sha256` | expanded from 6→15 fields in fold (FIX 4) — see below |
| `wp_remote_post()` | network round-trip (5s timeout budget) | unchanged |
| `update_last_use()` (on success) | get_marker + update_option | unchanged (already filed path-α) |

### FIX 4 — HMAC 15-field canonicalization expansion

The canonical buffer added 9 identity fields (3 base + 12 = 15 total length-prefixed entries). Per signed request the added work is:

- 9 additional `preg_match( '//u', $value )` UTF-8 assertions on short ASCII strings (digests, UUIDs, integer IDs, URLs)
- 9 additional `strlen($value)` + 4 string concatenations per field via `canonical_field()`
- 1 additional `hmac_sha256` call over ~600-1200 bytes vs ~200-400 bytes (constant-factor hash work)

Realistic per-request overhead on PHP 8.x: order of 5–30 microseconds. The signed request itself targets a 5-second loopback HTTP timeout. Overhead is < 0.001% of the request budget. Not an AC violation; not a regression worth blocking.

The expansion is mandated by W2 L2 CSO + L0 architect-reviewer to reach byte-exact parity with `src-tauri/src/surface_runtime/hmac.rs::canonical_request_bytes`. Equivalent work happens on the verifier side. Cross-lang drift defense (test vectors at `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json`) is the right shape.

### FIX 5 — MCP scope check at `tools/list` time

`filter_tools_list()` in `wp/dailyos/includes/mcp/class-dailyos-mcp-server.php:217-248` runs per-tool `permission->check()` against resolved scopes. Cost per `tools/list`:

- 1× `switch_to_substrate_user()` (`wp_set_current_user`) — fixed cost, called once per request
- N tools × `permission->check()` where `check()` internally calls `find_ability()` linear scan over inventory (M abilities)

Worst-case `tools/list` cost: O(N × M). With current inventory size (`tools/dailyos-abilities.json` — 37 lines, ≤ ~10 abilities in v1.4.2), N ≈ M ≤ 10, so ≤ 100 simple comparisons per `tools/list` call. Permission `check()` does scope intersection on small string sets. No DB hits inside the per-tool loop. No marker read inside the per-tool loop. Acceptable.

Note: `find_ability()` linear scan is already filed path-α maintenance and was filed before the fold. The fold-fix raises its call-count from "per invocation" to "per invocation + per tool at list time," but with N ≤ 10 this remains sub-millisecond. The path-α prescription (build a name→ability lookup map cached against inventory mtime) would mitigate the new path naturally.

### FIX 1 — Pairing marker runtime URL read

`runtime_base_url_for_signed_request($marker)` is a pure in-memory read of `$marker['runtime_url']` plus a `dailyos_wp_bridge_runtime_url` filter dispatch and `normalize_loopback_runtime_url` validation. The marker was already loaded at `signed_post:144`; no extra `wp_options` read. Negligible cost.

### FIX 2 — `dailyos_nonce_sweep` callback

No-op callback registered at `init`. Zero per-request cost. The callback fires only on the (W4-E) scheduled sweep event. Not on hot path.

### FIX 3 — `mcp_server_name` → `actor_instance`

Field renames only. `actor_instance()` is invoked from audit emission inside `authorize_ability_invocation()` (per ability INVOCATION, not per `tools/list` entry). One marker read per invocation, same as before the rename. Field semantic change does not move the call out of its prior position on the call graph.

### Pairing marker read paths post-fold

`get_marker()` callers across the plugin (`grep -rn "get_marker()" wp/dailyos/includes/`):

1. `class-dailyos-credential-store.php:45` — `is_paired()` helper
2. `class-dailyos-credential-store.php:102` — `update_last_use()` (path-α)
3. `class-dailyos-runtime-client.php:144` — signed_post entry (single per-request read)
4. `class-dailyos-mcp-server.php:518` — `actor_instance()` (audit-emission only; not on list path)
5. `class-dailyos-admin/class-dailyos-settings-page.php:76` — admin UI render only; not on transport hot path

Per signed runtime request: 1 marker read in `signed_post()` + 1 in `update_last_use()` on success = 2 reads. Matches the pre-fold baseline (the prior path-α finding). The fold did NOT add a third read.

---

## 3. New findings

**None.** No new path-α or path-β items surfaced by this re-run. No AC violation. No regression vs the initial L3 cycle that already issued APPROVE.

---

## 4. Confirmation of prior path-α items

All three Suite P path-α items remain present in the diff, structurally unchanged by fold, and remain maintenance-only (already filed against project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per `W3-handoff.md` §"Open / accumulated maintenance items"):

| Item | File:line in W3 head | Status |
|---|---|---|
| Pairing marker double-read in `update_last_use` | `wp/dailyos/includes/transport/class-dailyos-credential-store.php:101-111` | maintenance-only, unchanged by fold |
| Inventory loaded twice per MCP request | `wp/dailyos/includes/class-dailyos-ability-registry.php:43-66` consumed at `class-dailyos-mcp-server.php:407-419` + `class-dailyos-mcp-permission.php:104` | maintenance-only, FIX 5 increases pressure but does not change shape |
| `find_ability` linear scan | `wp/dailyos/includes/mcp/class-dailyos-mcp-permission.php:103-119` | maintenance-only, unchanged by fold |

The FIX 5 expansion of the per-tool filter compounds the "inventory loaded twice" and "find_ability linear scan" pressure quantitatively (now also fires N times per `tools/list`), but does not change the qualitative shape of either finding. Current inventory cardinality keeps the cost negligible; the same fix (inventory cache + name→ability map) addresses both the original baseline and the new list-time path. The compounding does NOT promote either finding from path-α (maintenance) to path-β (AC violation) at v1.4.2 scale.

---

## Summary

Re-run question: "did W3 fold introduce a NEW perf regression?" — **No.**
Prior APPROVE stands. W3 advances per Suite P.
