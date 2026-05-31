# v1.4.4 W1 Post-Merge Handoff — 2026-05-21

**PR:** [#346](https://github.com/jamesgiroux/daily-operating-system/pull/346) — **MERGED** at `f9b286c4` (2026-05-21T16:44:19Z, admin override on `L2/config-fence` per maintainer co-sign)
**Branch:** `wave/v1.4.4-w1-stage1a` (now on `dev`)
**Author:** James Giroux + Claude Opus 4.7 (session 2026-05-21)
**Predecessor:** `.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md` (pre-merge handoff; superseded by this doc)

---

## What landed

The v1.4.4 W1 wave merged into `dev` carrying the full W1 substrate AND substantial in-session retrofitting that surfaced during L4 hands-on validation. Net: 30+ commits across 4+ hours.

### Original W1 substrate (10 sub-tickets, all L2/L3-passing)
DOS-335 meeting_prep_status read/write split · DOS-459 EntityIntelligenceEnvelope · DOS-339 claim_receipt fan-out · DOS-477 entity trust-boundary · DOS-340 audit boundary · DOS-341 privacy/redaction · DOS-460 touchpoints + open-loops · DOS-8 semantic feedback (10 variants) · DOS-461 fixture harness · DOS-507 get_daily_briefing composition.

### In-session retrofits (today)

**1. Substrate trim (`df19db78` + later commits)**
- Added `SurfaceClient` to `allowed_actors` on 8 read abilities (`get_entity_intelligence`, `get_entity_context`, `list_accounts`, `list_people`, `list_projects`, `list_open_loops`, `get_daily_briefing`, `get_daily_readiness`)
- Expanded `DEFAULT_GRANTED_SCOPES` from v1.4.2's narrow 2 to 10 (covers W2/W3 reads)
- Added `required_scopes` to `get_entity_context` (proc-macro caught the omission)
- `method_exists` → `is_callable` in account-detail (supports `__call` proxies)
- Tauri command allowlist entries for `render_claim_receipt` + `submit_claim_feedback_command` (dual-path commands; abilities also exist)
- Ability inventory regenerated via `cargo run -p abilities-runtime --bin emit_ability_inventory`

**2. v1.4.2 cleanup (James's morning corrections)**
- Account-overview test-spike fully removed (12 files in `blocks/account-overview/`, pattern, tests, lint script — net -2473 LOC)
- `dailyos_briefing` CPT registered in `register_post_types()` + dataProvider test
- Templates + 2 GH workflow steps scrubbed of account-overview refs

**3. Visual parity — canonical class rewire (24 inner blocks + 1 outer)**
- All 24 v1.4.4 W2 inner blocks rewired from invented BEM (`wp-block-dailyos-X__Y`) to canonical CSS-module class names from `.docs/design/reference/surfaces/account.html` (multi-class side-by-side form)
- account-hero hand-done as reference; codex agent fanned out the remaining 23 in 19 minutes
- `account-detail-default.php` pattern converted from direct `register_block_pattern()` style to header-style (matches project/person/meeting/briefing siblings; the plugin loader expects header style)
- `single-dailyos_account.html` template rewired to `<!-- wp:pattern slug="dailyos/account-detail-default" /-->` (expands to 24-chapter composition at parse time)
- 34 block-kit integration fixtures generated (one per W2 block) + harness registry wired
- Empty-chip CSS pattern added to `wp-overlay-globals.css` (the `[class*="--empty"]` rule forcing display:block + grid-column:1/-1 on grid parents that would otherwise squeeze the chip to a 3px-wide column)
- v1.4.2 `mu-plugins/dailyos-block-showcase.php` renamed `.disabled` (incompatible composite client with v1.4.4 envelope shape)
- Made-up palette tokens reverted (focus-ring, surface-emphasis, text-on-emphasis); blocks use canonical `action` / `desk-charcoal` / `paper-warm-white` already in palette

**4. Dev merge (`f7c27c58`)**
- DOS-746 pairing scope-refresh API (auto-extends granted_scopes on each invoke)
- DOS-745 claim_ref field_path fix
- Chrome polish (FinisMarker footer, colophon, day-strip chrome, MagazinePageLayout wraps, global baseline)
- Engineering-ladder L0 threat-topology framing (PR #348)

**5. L1 test alignment**
- AccountDetailBlockTest filter-stub format fix + pattern header-style assertion + wrapper-args fallback
- EditorialShellPresenceTest drop v1.4.2 end-mark assertions, keep FinisMarker_root
- EntityDetailAutoFillTest accept `wp:pattern` slug references
- PatternRegistrationTest add baseline assertion to dodge risky-test status
- 1785 PHPCBF auto-fixes + manual docblock additions across the 3 test files

**6. Ephemeral-ref strip (`08c241ae`)**
- 119 `DOS-NNN` and `IN+` references stripped from code comments across 54 files (5 batches)
- ADR-#### refs preserved (durable per `feedback_no_ephemeral_issue_refs_in_comments`)

---

## Studio state (carry-forward)

The dev branch now has everything. The wave branch was merged via admin override on `L2/config-fence`. Studio symlinks are pointed at `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos*` (the main repo path, not the `/private/tmp/dailyos-w4-l1/` worktree as before). Re-checkout dev locally and Studio sees the merged state.

```sh
git checkout dev
git pull public dev
```

**Pairing state:** James re-paired today (after Tauri rebuild). The runtime port changes on every restart, so the pairing marker's `runtime_url` was patched a few times via direct SQLite UPDATE. With DOS-746 scope-refresh on dev, future Tauri restarts should pick up the new sentinel-discovered port automatically.

**Tauri runtime** was running on whatever port the sentinel reports (`~/.dailyos/runtime-endpoint.json`). After fresh `pnpm tauri dev` it'll bind a new port.

---

## Open threads (priority-ordered)

### 1. ROOT CAUSE FOUND: surface invoke fails on audit-emit contract (`wp_user_id` is None)

**Status as of 2026-05-21T17:17 (after PR #346 merge + fresh repair):** all 24 inner blocks still render empty (19 `not_available` + 4 `no_envelope`). The outer block has `data-dailyos-envelope-handle="a0d3baf87fa224e78f109fb3f448146b71806a569d5bea8a98c07eddd381f71b"` — that's the **deterministic-hash of an empty/error response** per `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php:122`. So the runtime IS being reached, signing IS verified, scope check IS passing (after the Phase A trim + DOS-746 scope-refresh), but the invoke fails at the **audit-emit gate**.

**Tauri log smoking gun:**
```
WARN dailyos_lib::surface_runtime] surface pairing audit write failed:
emit_surface_audit contract violation:
Actor::SurfaceClient requires AuditFields.wp_user_id to be Some(_)
```

**What this means:** when a `SurfaceClient` actor invokes an ability, the audit-event emitter validates that `AuditFields.wp_user_id` is `Some(_)` (the contract added during DOS-559 W2-C `preserve SurfaceClient attribution on post-HMAC validation rejections`). The WP plugin's signed POST IS sending `X-DailyOS-WP-User-Id` (see `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:244`), so either:
- the runtime side isn't reading it from the request header into `AuditFields`
- the runtime IS reading it but the `Option<String>` becomes `None` because the empty-string `""` isn't being wrapped in `Some()`
- something is stripping wp_user_id from the audit payload between request validation and the invoke audit emission

**Files to start with:**
- `src-tauri/src/surface_runtime/mod.rs` — search for `wp_user_id` + `AuditFields` + `emit_surface_audit`
- `src-tauri/src/services/surface_pairing.rs` — `AuditFields` struct + the audit-emit pathway
- Look at recent commits touching audit-emit: `git log --oneline -- src-tauri/src/surface_runtime/mod.rs | head -10`

**Why empty chips:** the failed audit-emit causes the invoke to return an error response. The outer block catches `is_wp_error || !is_array` → returns empty chip. The inner blocks read the (cached, empty) envelope handle → render their own empty chips. Both behaviors are correct; the substrate fault is at the audit-emit contract.

### 2. Inner-block envelope-shape mapping (only after #1 is fixed)
Once the audit-emit gate passes and real envelopes arrive, the 19 `not_available` chapters may STILL fail if their resolver expects different keys than the runtime ships. That's the secondary investigation. Don't start it until #1 is fixed — empty envelopes will mask shape mismatches.

The 4 `no_envelope` blocks (`recommended-actions`, `touchpoints-feed`, `open-loops-feed`, `unified-timeline`) are reading from block-context (`providesContext` → `usesContext`) but their context isn't propagating. After #1 fixes, check whether the feed-shape blocks have `usesContext` in their block.json.

### 2. Trust Topology Realignment dedicated session
`.docs/plans/trust-topology-realignment/L0-packet.md` is **SUPERSEDED** per James's edit. Phase A trim shipped inline in PR #346. Remaining: one tiny PR auto-deriving `DEFAULT_GRANTED_SCOPES` from registry at pair time (James started this; see commit `25bbec93` which already did half the work). Update the 5 reviewer prompts (already in `25bbec93`) and the ADR amendments stay deferred until shape is known.

### 3. Test-fixture realness
Codex generated 34 minimal stub fixtures. They satisfy `cargo test --no-run` and the per-block harness invariant, but assertions are skeletal (BlockWrapperAssertion + at least one binding). Real fixture coverage (asserting actual projection behavior) is path-α follow-up. File a Linear ticket.

### 4. Two more local-environment artifacts (pre-existing, surfaced this session)
- `src-tauri/src/intelligence/dimension_prompts.rs:947-951` has a stale `<<<<<<< HEAD ... >>>>>>> dc45a795` merge marker from commit `dc45a795 (DOS-249)`. Pre-existing. Worth a one-line cleanup commit.
- `wp/dailyos/tests/transport/RuntimeClientTest.php` tests hardcode `http://127.0.0.1:54321` but read the live `~/.dailyos/runtime-endpoint.json` sentinel in test env. Sentinel-having developers see 4 false failures. CI doesn't run with the sentinel so passes. Worth filing as an isolation issue.

### 5. Substrate health — corruption + write-starvation pattern (file a ticket)

This session surfaced three distinct DB-level issues that probably share one root cause. Worth filing as a maintenance ticket against substrate stability:

- **3 corruption-recovery cycles in 2 weeks** — `~/.dailyos/dailyos.db.corrupt-20260505`, `dailyos.db.corrupt-20260516`, `dailyos.db.corrupt-20260518`, `dailyos.db.corrupt-20260519`. SHM + WAL + DB snapshots preserved. Current DB passes `PRAGMA integrity_check` but the cadence (every ~7 days) suggests a write-path bug, not random hardware/power issues.
- **Background workers starve user actions on writer-mutex** — `Targeted claim repair worker iteration failed: transaction error: Failed to begin transaction: database is locked` fires every 7 seconds even with no Glean writers. The HygieneLoop + claim_repair worker + people_sync watcher + version_mutation_recovery + intel_queue all compete for the same writer mutex on startup AND in steady-state. When a user action arrives (revoke pair), it can't break in.
- **Glean enrichment writes are 30-60 SECONDS each** because Glean is slow AND the responses frequently fail parse validation (`Invalid JSON: trailing characters`, `No INTELLIGENCE block or JSON found`). The runtime holds the writer mutex for the full 30-60s, then discards the result. Two failures per 30-60s × 6 dimensions per entity = devastating writer-mutex contention for any entity flowing through IntelQueue.

Recommend: file an L0 packet for substrate write-path resilience. Topics: SQLite busy_timeout tuning, writer-mutex queueing fairness (so user actions don't starve), Glean response validation moving OUTSIDE the writer transaction, claim_repair / hygiene loop polling cadence reduction.

---

## Memories saved this session (auto-load in future sessions)

These four feedback memories at `/Users/jamesgiroux/.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/`:

1. `feedback_local_to_local_security_overreach_primary_concern.md` — WP loopback should have parity with React `tauri::invoke`; surface FIRST in every cross-surface architectural decision; treat asymmetric gating as a finding to challenge, not a default.

2. `feedback_visual_parity_is_never_path_alpha_for_wp_waves.md` — when an L0 packet routes "per-block style.css" or "theme.json per-block-styles" to path-α maintenance, flag BLOCKING; the wave's whole purpose is parity.

3. `feedback_ci_gate_inputs_are_L1_deliverables_not_verification_steps.md` — every CI gate's required artifact (style.css, integration fixture, allowlist entry, lint pre-condition) is an L1 deliverable; flag at L0 when routed to "L4 verification matrix" / "future fixture coverage." Umbrella class pattern covering visual parity + integration fixtures + security alignment.

4. `feedback_design_system_reference_dir_is_canonical_copy_paste.md` — `.docs/design/reference/` is intentionally copy-paste-ready; READ it before writing visual code; WP block style.css is translation, not design.

The MEMORY.md index has them at the top so they're load-first for the next session.

---

## What to do FIRST in the fresh session

1. **Pull the latest dev:** `git checkout dev && git pull public dev`
2. **Verify Studio renders the chrome:** `pnpm tauri dev`, hard-refresh `http://localhost:8884/accounts/bring-a-trailer/`. Should render the magazine shell with all 24 chapters showing canonical typography. Empty chips are quiet italic tertiary text inside canonical chapter shells. **This part works as of 2026-05-21.**
3. **Tail the Tauri log:** when the page renders, watch for `emit_surface_audit contract violation: Actor::SurfaceClient requires AuditFields.wp_user_id`. That's the root-cause from Open Thread #1.
4. **Fix the audit-emit contract** at `src-tauri/src/surface_runtime/mod.rs` or `services/surface_pairing.rs` — make `wp_user_id` extraction wrap properly into `Some(_)` from the signed POST headers.
5. **Re-validate**: bring-a-trailer should populate real envelope sections after the fix. Inner blocks may STILL be empty if envelope-shape mapping has gaps — that's Open Thread #2.

### Pairing + Studio quick-reference (in case re-pair is needed)

- The active Tauri SQLCipher DB key lives in macOS Keychain: `security find-generic-password -s "com.dailyos.desktop.db" -a "sqlcipher-key" -w`
- SQLite path: `~/.dailyos/dailyos.db` (sqlcipher 4.x; `sqlcipher` binary at `/opt/homebrew/bin/sqlcipher`)
- To revoke a stuck pairing without the Tauri UI (today's escape hatch when writer-mutex deadlocked):
  ```sh
  KEY=$(security find-generic-password -s "com.dailyos.desktop.db" -a "sqlcipher-key" -w)
  # First fully kill: pkill -9 -f "target/debug/dailyos$"
  sqlcipher ~/.dailyos/dailyos.db <<SQL
  PRAGMA key = "x'${KEY}'";
  UPDATE surface_client_pairings SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), revoked_reason = 'manual_revoke' WHERE revoked_at IS NULL;
  UPDATE surface_client_sessions SET revoked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), revoked_reason = 'manual_revoke' WHERE revoked_at IS NULL;
  SQL
  # Then restart Tauri and re-pair from WP admin
  ```
- WP marker DB path: `/Users/jamesgiroux/Studio/dailyos-dev/wp-content/database/.ht.sqlite` (plain SQLite). The `dailyos_pairing_marker` row's `runtime_url` needs to match the current Tauri sentinel port (`~/.dailyos/runtime-endpoint.json`). Re-pairing through the WP admin UI updates this automatically; manual port-patching pattern documented in this session's earlier work if needed.

---

## Codex tooling reminders

Per memory `feedback_codex_exec_direct_for_oneshot_reviews`:
- For one-shot codex review/challenge on docs: `codex exec --skip-git-repo-check -C <dir> '<prompt>' < /dev/null`
- For codex review on git diff: `node ${CLAUDE_PLUGIN_ROOT}/scripts/codex-companion.mjs review --background --base <ref>`
- For codex write-mode tasks: `codex-companion.mjs task --write --background --prompt "..."` — used 4 times today (block rewire, fixture generation, ephemeral-ref strip, PHPCBF). Each ran clean to completion in 5-20 min.
- 5-min heartbeat default for in-flight codex tasks: `ScheduleWakeup delaySeconds: 270`.

---

## Git remote conventions (carried forward from prior handoff)

- Remote: `public` (NOT `origin`) — `git@github.com:jamesgiroux/daily-operating-system.git`
- Public repo default branch: `main` (per memory `reference_public_repo_main_not_trunk`)
- Wave PRs target: `dev` (CLAUDE.md branch convention)
- Branch naming: `wave/v1.4.x-w<n>-stage<a/b/c>`

---

🤖 Generated with [Claude Code](https://claude.com/claude-code) — post-merge handoff for fresh session pickup
