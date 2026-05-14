# W3 wave-bundle session handoff

Date: 2026-05-13
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Wave: W3 (DOS-563 W3-0 spike, DOS-564 W3-A skeleton, DOS-565 W3-B transport, DOS-566 W3-C MCP)

## Where we are

W3 wave-bundle is implementation-complete + L3-folded but **not yet L3-re-reviewed**, not yet pushed, not yet PR-opened. W2 substrate-side is merged and on `dev`.

### Branches + commits

| Ref | Commit | State |
|---|---|---|
| `dev` (local + `public/dev`) | `dd003ee2` | W2 merged here via PR #270 (2026-05-13). 4 ahead of pre-W2 `75eda588`. |
| `dos-546-w3-wordpress-foundation` (local only) | `68e1ed9a` | Rebased on top of `dd003ee2`. 9 commits + 1 W3 L3 fold polish. Not pushed. |
| `codex/dos-546-w2` | `722618ad` (orig) / `338b64a4` (post fold via PR #270) | W2 PR merged to dev. Branch still exists. |

### W3 branch commit log (newest first)

```
68e1ed9a DOS-564/565/566 W3 L3 fold: complete 5-fix implementation polish
c1fdf2b4 DOS-566 W3-C: align McpExposureNoneTest with actor_instance audit field
870b5a3e DOS-564/566 W3 L3 fold (partial): nonce_sweep callback + drop mcp_server_name
8ed7676e DOS-566 W3-C: fail closed when substrate user cannot be created at activation
20be2830 DOS-564/565/566 W3 L2 Cycle 1 fold: 8 AC findings closed
c88bccc4 DOS-546 W3 L0: track W3-L0-packet.md in the wave-bundle branch
9a21427b DOS-565 W3-B: wire revoke-pairing admin action on settings page
f8e4c0a0 DOS-566 W3-C: add audit-emission coverage to McpExposureNoneTest
2ef5fa9b DOS-565 W3-B + DOS-566 W3-C: transport client + MCP server (parallel agents)
77463329 DOS-564 W3-A: docblock + PHPCS/PHPUnit config cleanup
3e9c5c91 DOS-564 W3-A: WordPress plugin skeleton + activation guards + grep gates
63d3b6c4 DOS-563 W3-0: spike artifacts — Blueprint v0, rsm-second-brain mapping, byte-exactness test
dd003ee2 Merge pull request #270 from jamesgiroux/codex/dos-546-w2  (W2 merged here)
```

### Current gates (W3 worktree)

- **PHPCS:** 28 files / 0 errors / 0 warnings
- **PHPUnit:** **45 tests / 161 assertions**, all green
- **grep-gates:** exit 0
- **Cargo (substrate):** untouched in this branch; W2 substrate is at `dd003ee2`

## What's done

### Ladder closure state

| Layer | State | Notes |
|---|---|---|
| L0 | **CLOSED** | 4 cycles, unanimous APPROVE on V4 packet 2026-05-13. Packet at `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md`. Linear doc: https://linear.app/a8c/document/w3-l0-packet-host-boundary-open-decisions-f26d89aa7cf5 |
| L1 (self-validation) | **PASSING** | All gates green |
| L2 | **CLOSED Cycle 2** | 3-of-4 reviewer APPROVE + codex CONDITIONAL re-classified per AC-bound discipline. Findings filed as maintenance: DOS-585/586/587/588. |
| L3 (initial run) | **BLOCKED** | Architect-reviewer P0 (W2 missing from dev) + codex 3 HIGH (missing fixes). Both ROOT CAUSES are now resolved. |
| L3 (fold) | **DONE** | 5 fixes folded in 68e1ed9a + 870b5a3e + c1fdf2b4. See "Per-fix status" below. |
| L3 (re-run) | **NEXT** | Has not been re-run after W2 merge + W3 fold. |
| L4 | **PAUSED** | dailyos-dev Studio site live at `~/Studio/dailyos-dev` port 8884; plugin symlinked at `~/Studio/dailyos-dev/wp-content/plugins/dailyos`. Pairing flow not exercised live yet. |

### 5 W3 L3 fixes — all implemented

1. **Hardcoded runtime URL → pairing marker** (architect-reviewer P0). `DEFAULT_RUNTIME_URL` and `127.0.0.1:8765` purged from `wp/dailyos/`. Marker carries `runtime_url`; client returns `dailyos_not_paired` `WP_Error` when missing. `dailyos_wp_bridge_runtime_url` filter validates loopback-only via `normalize_loopback_runtime_url`.
2. **`dailyos_nonce_sweep` callback** (architect-reviewer P1). No-op `sweep_presence_nonces` registered in `DailyOS_Plugin::init`. Full lifecycle in W4-E.
3. **`mcp_server_name` → `actor_instance`** (architect-reviewer P1). Replaced everywhere. `actor_instance` is `plugin_instance_uuid` from marker; generated at activation via `wp_generate_uuid4`, persisted in `wp_options.dailyos_plugin_instance_uuid`.
4. **HMAC 15-field canonicalization** (W2 L2 CSO + codex L3). `canonical_bytes()` matches `src-tauri/src/surface_runtime/hmac.rs canonical_request_bytes` byte-for-byte. New marker fields: `site_binding_digest`, `site_nonce_full`, `wp_install_uuid`, `plugin_instance_uuid`. Test vectors at `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` drive `@dataProvider canonical_vector_provider` in `HmacSignerTest`. Asserts both pre-HMAC canonical bytes AND final signature.
5. **MCP scope check at `tools/list` time** (codex L3 MEDIUM). `mcp_adapter_tools_list` filter runs per-tool `DailyOS_Mcp_Permission::check` against the resolved session.

## What's next (next-session pickup)

### Option A — L3 re-run (recommended)

Spawn 5 background reviewers against the rebased+folded W3 state. They should each see:
- W2 substrate is now on `dev` (`dd003ee2`)
- W3 fold lands 5 named fixes (68e1ed9a + earlier)
- Gates green: PHPCS 0, PHPUnit 45/161, grep-gates 0

Reviewer roster (mirrors initial L3 run):
1. **codex challenge** — wave-integrated adversarial, full W3 context
2. **architect-reviewer** subagent — integrated state architectural integrity
3. **security-auditor** (Suite S) — integrated trust boundary attack chains
4. **performance-engineer** (Suite P) — likely no regressions; already approved
5. **chaos-engineer** (Suite E) — already approved, optional re-run

If all approve, L3 closes and W3 advances to:
- L4 surface QA (interactive, in `~/Studio/dailyos-dev`)
- Push branch + open wave-scoped PR to `public/dev`

### Option B — push + open draft PR first

Push `dos-546-w3-wordpress-foundation` to `public`, open a draft PR, let L3 reviewers operate against the PR diff in GitHub. Cleaner audit trail; slightly more setup overhead.

### Option C — L4 first, then L3 re-run

Drive manual L4 surface QA in `~/Studio/dailyos-dev`:
1. Activate the plugin (currently symlinked; not yet activated)
2. Pair with the Tauri runtime (`pnpm dev` already running per James 2026-05-13)
3. Exercise pairing/revoke/repair flow
4. Verify MCP tool enumeration through Claude Desktop or `studio mcp` peer
5. Test `wp dailyos status`, `wp dailyos repair-namespace` CLI commands

This validates the integration empirically before L3 re-runs against it.

## Open / accumulated maintenance items

### Filed as Linear issues (Codebase Maintenance & Production Quality project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`)

From L0:
- **DOS-585** — DX "this was my edit" attribution for Studio wp_cli mutations (Low)
- **DOS-586** — Forged-marker + clean-runtime → quarantine negative fixture (Medium)
- **DOS-587** — Unauthenticated REST schema introspection negative fixture (Medium)
- **DOS-588** — Bearer-token credential retrieval evaluation gate (Low)

### To be filed (from L2 + L3 reviewer notes)

L2 maintenance (multiple reviewers):
- Multisite role registration semantics
- `sslverify => false` conditional on loopback
- `runtime_base_url` filter context invariant comment
- Marker non-instance-id field hardening
- `ScopeSetDriftTest` inline-resolver vs production-resolver
- PSR-4 declaration cleanup
- Dead `$scope_resolver` field
- `runtime_url` loopback validation (now done in FIX 1)
- `$_SERVER` access without `phpcs:ignore` rationale
- `public_log_invocation` test helper on production class
- `abilities.json` sync mechanism
- `ensure_user` concurrency window / partial-state observability
- `wp_set_current_user` call-site centralization
- Marker tautology fix (cross-source binding)
- Identity-switch ordering hardening
- README version-pin centralization
- U+2026 ellipsis ASCII consideration

L3 maintenance (multiple reviewers):
- Credential bridge provider integration (Tauri-side, post-W3 wave)
- Pairing marker runtime-state authority cross-check at first signed request
- Session-key filter origin attestation (closure-bound provider)
- Substrate user collision detection on adoption
- Suite P: pairing marker double-read in `update_last_use`, inventory loaded twice per MCP request, `find_ability` linear scan
- Suite E: revoke-vs-`update_last_use` TOCTOU resurrection, concurrent admin pairing last-writer-wins, options-table-read-only quiet success, permission callback side-effect ordering
- W2 follow-up: artifact 08 doc update (15-field canonical shape, was 6-field)
- W2 follow-up: shared HMAC test vectors consumed by both Rust verifier and PHP signer for cross-lang drift defense (`hmac_canonical_vectors.json` is the PHP side; Rust consumer pending)

## Environment

### Local repo

- **Path:** `/Users/jamesgiroux/Documents/dailyos-repo`
- **Remote `public`:** `git@github.com:jamesgiroux/daily-operating-system.git`
- **Active branch:** `dev` at `dd003ee2`

### Worktrees in active use

| Path | Branch | Purpose |
|---|---|---|
| `/Users/jamesgiroux/Documents/dailyos-repo` | `dev` | Main checkout |
| `/private/tmp/dailyos-w3-0` | `dos-546-w3-wordpress-foundation` | W3 work (this branch) |
| `/Users/jamesgiroux/Documents/dailyos-repo/worktrees/dos-546-w2` | `codex/dos-546-w2` | W2 (merged, retained) |

### Studio dev site

- **Path:** `~/Studio/dailyos-dev`
- **URL:** `http://localhost:8884`
- **WP / PHP:** 6.9.4 / 8.4
- **Plugin symlinked at:** `~/Studio/dailyos-dev/wp-content/plugins/dailyos` → `/private/tmp/dailyos-w3-0/wp/dailyos`
- **Plugin activated?** No — pending L4 manual exercise

### Tauri runtime

- James confirmed `pnpm dev` running per 2026-05-13. Pair flow not yet exercised. Runtime bound port unknown; will come through pairing handshake response.

### Stash state (main repo)

`stash@{0}` from session start preserves James's uncommitted local work that conflicted on `migrations.rs` (v168 slot collision with W4-B.1.1) and `tasks/lessons.md`. The W4-* + W3 L0 packet untracked files in `.docs/plans/dos-546/v1.4.2-project/` are James's local-only work — preserved on disk, never staged.

### GitHub auth state

- `github.com` (public): logged in as `jamesgiroux`, working.
- `github.a8c.com` (internal): timeout (VPN required). A8C-internal prior-art sweep (DOS-563 W3-0 scope) is deferred until auth resolves.

## Key file references

- `.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md` — V4 L0 packet (authoritative acceptance contract). Linear: https://linear.app/a8c/document/w3-l0-packet-host-boundary-open-decisions-f26d89aa7cf5
- `.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md` — wave plan
- `.docs/plans/dos-546/v1.4.2-project/02-issues.md` — issue contracts
- `.docs/plans/dos-546/v1.4.2-project/w3-0-blueprint-v0.json` — Studio Blueprint v0 (consumed by W6-B)
- `.docs/plans/dos-546/v1.4.2-project/w3-0-rsm-second-brain-mapping.md` — architecture borrow patterns
- `.docs/plans/dos-546/v1.4.2-project/w3-0-transport-test/` — WP HTTP API byte-exactness test
- `wp/dailyos/` — plugin source
- `wp/dailyos/tests/fixtures/hmac_canonical_vectors.json` — shared HMAC test vectors
- `src-tauri/src/surface_runtime/hmac.rs` — substrate HMAC verifier (canonical bytes source of truth)
- `src-tauri/src/services/surface_pairing.rs` — substrate pairing authority

## Resume protocol

When resuming, the first three things to verify:

1. `cd /private/tmp/dailyos-w3-0 && git status --short` — should be clean (or only show James's untracked W4 L0 packets)
2. `cd /private/tmp/dailyos-w3-0/wp/dailyos && ./vendor/bin/phpunit --no-coverage` — should report `OK (45 tests, 161 assertions)`
3. `cd /Users/jamesgiroux/Documents/dailyos-repo && git log dev --oneline -3` — should start with `dd003ee2 Merge pull request #270 from jamesgiroux/codex/dos-546-w2`

If any of those drift, reconcile before continuing. After confirmation, pick A/B/C from "What's next" with James.
