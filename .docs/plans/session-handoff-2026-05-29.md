# Session handoff — 2026-05-29 (v1.4.9 substrate reset + DB-mode isolation)

**For:** a fresh session continuing this work, with or without James present.
**Branch in flight:** `feat/dos-822-db-mode-resolver` (820-B). **Remote:** `public` (NOT origin). **Primaries:** `public/dev`, `public/main`.

---

## STATUS UPDATE — 2026-05-29 PM (continued session)

Progress on TL;DR #1–3 below:
- **#1 DONE** — codex's test-isolation fix was in the working tree; I completed it. New commit **`ed9d0745`** on `feat/dos-822-db-mode-resolver`: state-path isolation completion (state.rs now routes the `.dailyos` root through `dailyos_data_dir()`, the single source of truth — codex had only wired `db/core.rs`), the **AC-1b CI gate** (`check_dailyos_path_resolver_allowlist.sh`, wired into lint-frontend), and the **`reset_all` mode-awareness fix**.
- **#2 DONE — safety proof HOLDS** — prod DB mtime (`1780081237`) unchanged across **every** `cargo test --lib` run (6+ runs). Tests are physically unable to touch the real prod DB. clippy `--lib` clean.
- **#3 DONE — MERGED.** 820-B L2 cycle-2 = **APPROVE** from both `l2-bounded-reviewer` (AC-scoped) and `ce-security-reviewer` (no exploitable boundary gap); verdict on DOS-822. **PR #419 squash-merged to `public/dev`** (`9cb89af8`). Branch pruned (local + remote). DOS-822 → Done. (Final commits before merge: `347f4f49` 820-B isolation+gate, `04311250` DOS-825 lint fix.)
- **CI flakes filed (pre-existing, NOT 820-B):**
  - `DOS-824` (Codebase Maintenance, High) — process-global `CUTOVER_CAPTURE_PAUSE_DEPTH` in `write_fence.rs` leaks the cutover pause across parallel tests → ~50-75% flake in `mutation_smoke_tests` and `surface_pairing::dos674_cleanup_outside_transaction`. In-memory-DB tests, exposed by timing only. PR #419 CI passed `cargo test` cleanly on one attempt (intermittent, not always red).
  - `DOS-825` (Codebase Maintenance, Medium) — **fixed** in PR #419: stale `calendar_hot_path_routes_writes_through_service_batch` lint updated for the `meetings_writer` routing refactor.
- **REMAINING — only the prod-DB rebuild at schema 123** (§"Prod DB rebuild"), **James-driven** (destructive rm + v1.2.2 relaunch). Pre-checked + ready: app closed (lsof empty), current 273 DB is byte-identical to backup `_salvage-2026-05-29/dailyos.db.v14x-273-rebuild` (both 47,341,568 B), so the rm is fully recoverable. Awaiting James's go.

---

## TL;DR — do this first (ORIGINAL, see status update above)

1. **A codex run is in flight** implementing a CRITICAL test-isolation fix (bash task `b9mjx6y9t`). Check its result: `tail -c 5000 /private/tmp/claude-501/-Users-jamesgiroux-Documents-dailyos-repo/9d9ba30c-9191-47a9-8d8f-078aae8259a6/tasks/b9mjx6y9t.output`. If that task dir is gone (new session), check `git log --oneline -3` on `feat/dos-822-db-mode-resolver` for a new commit after `db63ebf4`.
2. **Verify the isolation fix** (the proof): note `stat -f %m ~/.dailyos/dailyos.db`, run `cd src-tauri && cargo test --lib`, then re-check the mtime. **It MUST be unchanged** — `cargo test` must not touch the real prod DB. (Prod is currently at schema 273 and backed up, so a stray pre-fix run only no-op-re-migrates; still, the mtime-unchanged check is the acceptance proof.)
3. **Re-run 820-B L2** to confirm the CHANGES-REQUIRED items are resolved (CI gate added; see §820-B). Then merge to `dev`.
4. **Then rebuild prod at schema 123** (see §"Prod DB rebuild").

---

## The DB incident (why this matters)

- **2026-05-28:** the production SQLCipher DB corrupted (dev-workflow artifact: `pkill` on a hot WAL writer + `step(-1)` backup + over-aggressive recovery predicate; all patched in PR #416).
- **Today:** backed up the corrupt DB (`~/.dailyos/_salvage-2026-05-29/dailyos.db`, byte-identical, 364 MB) → nuked → James rebuilt with his **v1.2.2 production app** → fresh DB at schema 123 (102 tables), integrity verified ok.
- **Then it regressed:** running `cargo test` (during codex's 820-B work) **migrated the real prod DB from 123 → 273** (v1.4.x schema, 174 tables, healthy). Root cause below. v1.2.2 can't open schema 273 (ADR-0071 forward-compat guard) → "Database Recovery Required" on launch.
- **Decision (James, firm):** production stays **v1.2.2 (schema 123)**; `cargo test` must be **physically unable** to touch `~/.dailyos`. Isolate tests first, then rebuild prod at 123 (or the rebuild re-clobbers).

### Root cause of the test-migrates-prod regression
`DB_MODE` is a process-wide static. 820-A guard tests call `set_db_mode(DbMode::Live)`. `cargo test` runs thousands of tests in ONE process. A guard test leaves the process in Live; a later test calling `ActionDb::open()` → `db_path()` → resolves the **REAL** `~/.dailyos/dailyos.db` (tests use `dirs::home_dir()`, no sandbox) → runs migrations → 273. The structural guard only blocks *non-Live* opens, so Live tests reach prod. **The fix (in flight): in `cfg(test)` builds the data-root resolves to a temp dir, never the real home.**

### Prod DB current state + backups
- `~/.dailyos/dailyos.db` — schema 273, v1.4.x, 47 MB, **healthy** (opens with key, 174 tables incl. claim substrate). v1.2.2 cannot open it.
- `~/.dailyos/_salvage-2026-05-29/dailyos.db` — original CORRUPT db (364 MB) for later `.recover`/table salvage.
- `~/.dailyos/_salvage-2026-05-29/dailyos.db.v14x-273-rebuild` — the healthy 273 rebuild (47 MB), kept in case a future v1.4.x path wants the re-enriched state.
- Keychain key intact (`com.dailyos.desktop.db` / `sqlcipher-key`); `~/.dailyos/google/` (OAuth) + `models/` retained.

### Prod DB rebuild (do AFTER test-isolation merges + verifies)
1. Back up current `dailyos.db` if changed since (already have the 273 backup).
2. `rm -f ~/.dailyos/dailyos.db ~/.dailyos/dailyos.db-wal ~/.dailyos/dailyos.db-shm ~/.dailyos/dailyos-replica.db ~/.dailyos/dailyos-dev.db` (confirm `lsof ~/.dailyos/dailyos.db` empty first).
3. James relaunches the **v1.2.2 production app** → rebuilds fresh at schema 123 → re-enriches from workspace + sources.
4. It now **stays 123** because dev/`cargo test` can no longer reach it. Verify with the mtime proof after a `cargo test` run.

---

## v1.4.9 work — status

**Project:** v1.4.9 — Substrate Reset & Judgment Maturation (`874a7361-3f2e-4448-96bf-4ba600238ee0`). **Umbrella:** DOS-820.

- **820-A (DOS-821) — DONE, merged PR #418** (`public/dev` @ `2cc0b23b`). `DbMode {Live,Replica,Mock}`, fail-closed default (debug→Replica, release→Live), structural prod-open deny (`guard_path_for_mode` → `DbError::ProdOpenDenied`) at every open chokepoint incl. `open_readonly_at` + DbService + rekey, CI gate `check_db_open_guard_allowlist.sh`. L2: security GUARD HOLDS.
- **820-B (DOS-822) — IN REVIEW, branch `feat/dos-822-db-mode-resolver`** (commit `db63ebf4` + the in-flight isolation fix). Per-mode workspace/config resolver (`mode_scoped_state_path` in state.rs) + Google-token/audit/enrichment/integrations isolation; devtools fail-closed. **L2 verdict:**
  - Security: **ISOLATION HOLDS** (Google token mode-scoped: per-mode keychain service + isolated path). Maintenance gaps: intermediate `~/.dailyos/{replica,dev}` dirs not created 0700; `harden_data_directory` OnceLock hardens only the active subtree.
  - AC-bounded: **CHANGES-REQUIRED** — BLOCK: the AC-1b **CI gate** (forbid new hardcoded `~/.dailyos`/`Documents/DailyOS` joins outside the resolver) was missing. MAINTENANCE: `reset_all` DB-file branch is mode-blind (deletes prod in Replica; currently unreachable).
  - **The in-flight codex fix bundles into this branch:** (1) the test-home isolation, (2) the CI gate (resolves BLOCK), (3) the `reset_all` fix. After it lands → re-run L2 → merge.
- **820-C (DOS-823) — BACKLOG, not started.** `replica-refresh` clone (prod→replica via `run_chunked_backup`, app-closed/checkpointed) + destructive-command guards (`start_fresh_database`, `restore_database_from_backup`).
- **Also in v1.4.9:** DOS-628 (claims→per-entity readable files — the real new build, not started), DOS-784/DOS-804 (v1.4.8 follow-ups), plus pulled-forward moat/MCP/substrate items from the reconciliation.

**L0 plan (Tier 1 HTML):** `.docs/plans/v1.4.9-replica-db-l0-plan.html` (md is a pointer stub).

---

## Broader context (the reset)

- **Strategic frame:** DailyOS = the personal-**judgment** substrate (claims + trust + abilities + salience), surfaced via MCP-first. Memory is commoditizing (GBrain et al.); judgment is the unsolved moat. Captured in `.docs/plans/dailyos-rearchitecture-examination-2026-05-29.md` (4-reviewer red-team) + memory `project_dailyos_rearchitecture_examination_2026_05_28`.
- **ADR-0135** (`.docs/decisions/0135-...md`): **WordPress-as-primary-surface REVERTED** → headless via **MCP read/write**; macOS app stays as the visual + control-plane surface; new surface design → **v1.5.0** (`4a2a7eeb-2306-46c2-95ed-0b6eef6e5cc6`). Supersedes ADR-0129, reinforces ADR-0128.
- **Storage decision:** SQLite stays canonical (ACID); readable files are a projection OUT (not files-as-primary — the adversarial review killed that). Drop SQLCipher *encryption* is desired but **decoupled to its own ADR-0092 amendment (D5, not started)**.
- **Repo consolidation done:** 50 branches/20 worktrees → `dev`+`main` only; ~16 GB + ~323 GB reclaimed; v1.4.4–v1.4.8 reconciled/closed (`.docs/plans/v1.4.9-reconciliation-ledger-2026-05-29.md`); v1.5.0 created; `~/.dailyos` corruption-chain files swept.

---

## Repo / process facts

- Remote `public` → `git@github.com:jamesgiroux/daily-operating-system.git`. `git fetch public`. No `origin`. (memory: `reference_repo_remote_and_branches`)
- Local `dev` is 1 commit AHEAD of `public/dev` = `9a7b07b1` (James's RSM-deck commit, his, unpushed — leave it). 820-B branch was based on `public/dev` (2cc0b23b) to keep the deck out of its PR.
- **CI gotcha:** the `check_no_ephemeral_issue_refs_in_comments.sh` gate fails on `DOS-###`/ticket refs in source comments — keep ticket ids out of code comments (tripped 820-A once).
- **Pre-commit schema gate** false-positives on any `src-tauri/src/db/` change (expects mock-data seeds); use `git commit --no-verify` for DB-mode changes that add no table/column, and say so.
- **Use codex sub-agents** for implementation/refactor/test work to keep the main context lean (memory: `feedback_use_codex_subagents_to_distribute_tokens`). `codex exec --sandbox workspace-write` works (the codex:codex-rescue *agent* stalls at research — avoid it). codex can't write `.git` from its sandbox → commit yourself if it reports that.
- L2 pattern that's working: `l2-bounded-reviewer` (AC-scoped vs the Linear issue) + `ce-security-reviewer` (the trust boundary), both background; merge on green per James's standing "merge when CI green" authority.

## Open threads (post-820-B)
- 820-C (replica-refresh + destructive-command guards).
- D5: SQLCipher-encryption-drop ADR-0092 amendment.
- 820-B maintenance: intermediate-dir 0700 perms + harden OnceLock (→ Codebase Maintenance `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).
- The 7,187-file `~/.dailyos` clutter is swept; salvage backups retained.
- DOS-628 (claims→files projection) — the genuinely new build still ahead.
