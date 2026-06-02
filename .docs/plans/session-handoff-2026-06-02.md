# Session handoff — 2026-06-02 — v1.4.9 ready to run

**For:** a fresh session picking up v1.4.9 (Substrate Reset & Judgment Maturation), with or without James present.
**Repo:** remote is `public` (NOT origin). Primaries: `public/dev`, `public/main`. `git fetch public`. PRs → `dev`.

---

## TL;DR — start here

1. **The v1.4.9 plan is L0-converged and on `dev`.** Canonical: `.docs/plans/v1.4.9-waves.md` (+ `.html` twin). Read it first — it has the full W1–W6 wave structure, the 8 cross-wave decisions, per-wave proof obligations, and the complete L0 cycle-1→4 verdict trail in its Amendments section. **Do not re-plan it.**
2. **Linear is the source of truth for status.** Project: `v1.4.9 — Substrate Reset & Judgment Maturation` (`874a7361-3f2e-4448-96bf-4ba600238ee0`). Milestones W1–W6. Pull live status; this doc's snapshot will drift.
3. **Critical-path the headline:** single-writer (W1) → W3 (claims→editable files) → W4 (correction loop = the headline) → W5 (MCP parity). W1's storage-hardening (cipher/rebuild), W2 (security), W6 (transcript) run *parallel* and gate nothing on the loop.
4. **There is a live parallel track: v1.5.0 composable surfaces** is actively moving on `public/dev` (W0/W1, e.g. #428 "Account composition surface"). v1.4.9 substrate and v1.5.0 surfaces are separate programs sharing `dev`. Don't conflate them; don't assume `dev` HEAD == v1.4.9 work.

---

## v1.4.9 status snapshot (2026-06-02 — verify against Linear)

**Done + merged:**
- **DOS-821 (820-A)** — DB-mode bootstrap contract, fail-closed default, structural prod-open deny. Merged #418. *This is the safety core; it's why this session's incidents were survivable.*
- **DOS-822 (820-B)** — per-mode workspace/config resolver + isolation completion + AC-1b CI gate. Merged #419.

**In flight / blocked:**
- **DOS-758** (W1, McpToolHandler request-scoped DB refactor) — Linear says "In Review" but **PR #355 is CLOSED, draft, ~186 commits behind dev, CONFLICTING, and gated on the W2 auth overhaul (DOS-833) which hasn't started.** Do NOT just reopen/merge it. Its own ticket marks it PAUSED pending the auth-model decision. Options when you get here: (a) do W2 first then re-implement 758 fresh against current `dev`; (b) re-scope 758 to its auth-independent core (the `McpHandlerContext` trait + `db_service` re-entrancy guard + the CI gate — the parts the ticket says "survive the overhaul") and fold the HMAC-coupled handler work into W2. **James's call — not yet made.**

**Everything else: Backlog/Todo — no wave has started implementation.** W1 remaining (831 cipher-drop, 832 rebuild-from-JSON, 823 replica-refresh/820-C, 846 MCP-binary-guard, 647 umbrella, + ingestion carryovers 767/768/770/812), W2 (833/510), W3 (628), W4 (12 issues incl. 834 propagation keystone + 8/318/443/447/316/811/338/446/278/277/317), W5 (9 issues), W6 (343/327/511).

**New tickets filed this session (Codebase Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):**
- **DOS-846** (W1) — Claude Desktop MCP must run a guarded/versioned `dailyos-mcp`, not a stale `target/debug/` build. *Root cause of a prod clobber this session — see below.*
- **DOS-847** — migration v68 (`068_success_plans.sql`) assumes `dimensions_json` in source `entity_assessment`; hard-fails the forward migration. **On the 123→273 path a future v1.4.x prod push will run — fix before that push.**
- **DOS-848** — prod btree corruption root-cause (10 tables/21 indexes); standard SQLite repair tools OOM on it.

---

## OPERATIONAL KNOWLEDGE — read before touching any DB or dev build

This is the hard-won part that is NOT in any ticket. This session lost hours to it.

### The DB-mode isolation model (820-A/B, shipped)
- `DbMode { Live, Replica, Mock }`, resolved at process start (`db/core.rs` `resolve_and_set_db_mode_from_process`): `--live`/`--replica` arg or `DAILYOS_DB_MODE=live|replica|mock` env; else default = **Replica in debug builds, Live in release** (`cfg!(debug_assertions)`).
- Live → `~/.dailyos/dailyos.db` (prod). Replica → `~/.dailyos/dailyos-replica.db`. Mock → `dailyos-dev.db`.
- `guard_path_for_mode` / `ProdOpenDenied` structurally refuse opening the prod path in non-Live mode, at every open chokepoint.
- **The guard protects the DB-OPEN PATH. It does NOT control which binary runs.** That gap is the whole problem below.

### Prod-clobber vectors (how prod kept getting migrated off schema 123)
Production runs **v1.2.2 (schema 123)**. Any newer/dev binary that opens prod and runs migrations takes it to 273, after which v1.2.2 can't open it (ADR-0071 forward-compat guard → "Database Recovery Required").
- **Vector 1 (fixed):** Claude Desktop's `claude_desktop_config.json` ran a **stale `dailyos-mcp` built before the guard existed** (May-17 binary, zero guard symbols) → opened prod, migrated it. Fixed by rebuilding the guarded binary (`cd src-tauri && cargo build --features mcp --bin dailyos-mcp` — note `--features mcp` is REQUIRED) to the same `target/debug/dailyos-mcp` path the config points at. DOS-846 is the durable fix (versioned binary + guard-presence check).
- **Vector 2 (observed):** a `pnpm tauri` dev app from a `/private/tmp` worktree opened prod and tried to migrate it; it failed at v68 (DOS-847) — but note it was migrating the **replica** (v67), not prod. The guard worked there.
- **Rule:** before running ANY dev build (app or MCP), confirm it's current (`strings <binary> | grep -E "ProdOpenDenied|dailyos-replica.db"` must be non-empty) and prefer `DAILYOS_DB_MODE=replica`. Memory: `feedback_stale_mcp_binary_clobbers_prod_db`.

### Reading SQLCipher DBs for diagnosis (you WILL need this)
- Key: `security find-generic-password -s com.dailyos.desktop.db -a sqlcipher-key -w` (hex).
- Open read-only: `sqlcipher "file:<db>?mode=ro"` then `PRAGMA key="x'<hex>'";`
- **Schema version lives in the `schema_version` table** (`SELECT MAX(version) FROM schema_version`), NOT `PRAGMA user_version` and NOT table-count. (I wasted a cycle judging schema by table count — don't.)
- **NEVER run `integrity_check` against the live prod file while the app holds it** — a 2MB+ WAL gives phantom "malformed" errors. Copy main+wal+shm to scratch, `PRAGMA wal_checkpoint(TRUNCATE)` the copy, THEN check. Or just `SELECT count(*)` real tables through the running app's view.

### Repairing a corrupt SQLCipher DB (method captured, DOS-848)
Standard tools (`.recover`, `VACUUM`, `VACUUM INTO`, `DROP TABLE`, backup API) **all OOM/abort** because they traverse corrupt pages. What works: **logical extraction** — apply the full schema to a fresh keyed DB, then `INSERT INTO new.X SELECT * FROM src.X` **one table per sqlcipher process** (a single session OOMs partway), skipping unreadable tables (recreate empty if derived). Then verify `integrity_check` = clean before swapping in. Always move the corrupt original aside (don't delete).

### Current DB state (2026-06-02)
- **prod `~/.dailyos/dailyos.db`: schema 123, repaired + clean** (integrity 0 errors; 58 acct/756 mtg/238 ppl/229 actions). v1.2.2 opens it. Repaired this session via logical extraction. Corrupt original + backups in `~/.dailyos/_salvage-2026-06-01/`.
- **replica `dailyos-replica.db`: schema 274**, reseeded from the clean 273-rebuild salvage and migrated forward by a dev app — i.e. the replica path works as designed.
- 3 derived/cache tables (`signal_events`, `entity_linking_evaluations`, `linked_entities_raw`) are empty in repaired prod; the runtime regenerates them.

---

## Repo / process facts
- Working tree may have **uncommitted v1.5.0 changes** that are NOT v1.4.9 work (app_support.rs, db/core.rs, db_backup.rs, etc. seen 2026-06-02). Leave them; don't bundle into v1.4.9 commits.
- **CI gotchas (cost real cycles this session):**
  - `L2 / validate-pr-template` reads the PR body **frozen at trigger time** + counts `previous_filename` for renames. Fixing the body needs a **fresh push** (not `gh pr edit` + `gh run rerun`). `security_auditor_invoked: false` FAILS if any changed path (incl. a rename's old path, e.g. archiving `.docs/plans/orchestration/**`) matches a `matrix.yml` security trigger → set `true` with rationale. Memory: `reference_pr_body_validator_needs_fresh_push`.
  - Pre-commit/pre-push run `cargo test --lib` which intermittently flakes (DOS-824 cutover-pause). `--no-verify` is justified when clippy+tsc are clean and the flake is the only red.
  - DB-mode/`db/` changes false-positive the pre-commit schema-seed gate → `--no-verify` with a note.
- **L2-status line** mandatory in code commit messages (`L2-status: passed|not-run-acknowledged|n-a-doc-only`).
- **Never spawn concurrent/background git commits** — they race `.git/index.lock` and corrupt the terminal (memory: `feedback_never_spawn_concurrent_background_git_commits`). One git mutation at a time, foreground.
- Plan HTML uses `plans.css` + the `_templates/wave-plan.html` shape — never hand-roll inline CSS.

## Engineering ladder reminder (for running the waves)
Per `.docs/plans/engineering-ladder.md`: each wave's lead issues get an L0 packet (declare §0 origination + §1 trust topology). L0 panel = `/codex challenge` + ONE routed planning reviewer (+1 at Wave scope), mandatory K-in `ce-learnings-researcher`. The v1.4.9 trust topology is **local-to-local single-user** with an MCP carve-out (Confidential/UserOnly never cross MCP) — see the plan §1. L2 is AC-bounded (`l2-bounded-reviewer`). Reinvented substrate = BLOCKED.

## Suggested first move for the next session
Confirm Linear status, then either (a) get James's decision on the DOS-758 path (the only thing mid-flight, and it's blocked on the W2 auth call), or (b) start W1 L0 packets for the clean-start items (831 cipher-drop, 832 rebuild, 823 replica-refresh, 846 MCP-binary-guard) since W1 single-writer is the only hard gate for the headline loop. W3→W4 is where the product headline ("correct once, stays corrected") actually lands.
