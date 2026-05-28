# Substrate handoff — DB corruption lookback, 2026-05-28

**Author:** Claude (this session) → next session
**Branch:** `docs/substrate-handoff-2026-05-28` (off `public/dev`)
**Status:** notes for a planning conversation, not a plan
**Audience:** a fresh session, with or without James present, that can hold the architectural question separately from the immediate corruption recovery.

---

## TL;DR

Today we lost the local DailyOS database to corruption that we eventually traced to **two unrelated bugs that had been stacking on top of each other for weeks**. Both are now fixed on `dev` via [PR #416](https://github.com/jamesgiroux/daily-operating-system/pull/416). The fixes prevent new damage. They do not repair existing damage. Every `~/.dailyos/dailyos.db*` snapshot we own is malformed in some page somewhere. The `.recover` path OOM'd at 405 MB. The user is open to nuking and rebuilding from filesystem.

Before nuking, the user wants to use this moment as a strategic inflection. The question they want a fresh session to hold is **whether SQLite is supposed to be the primary store or a cache**, because today's pain is the substrate telling us we've outgrown the file-format-as-source-of-truth model.

This document captures the lived evidence so the conversation doesn't have to re-derive it. It is **not** a plan or a recommendation. It is the raw material a planning session would draw on.

---

## What we lived today, briefly

Tauri dev was running normally. The user killed it via `pkill` to apply a fix, which nailed an in-flight WAL writer and produced btree corruption (the documented hot-restart class — see `CLAUDE.md` Gotchas + `.docs/plans/db-throughput-architecture.html` §8 Risks). All of this is expected behavior given the kill mechanism.

The recovery sequence is where the interesting evidence is.

1. **Restored from `dailyos.db.bak` (in-app manual backup, today 09:50 ET, 406 MB).** First relaunch produced 27+ `database disk image is malformed` signals within seconds. The backup was already broken when it was written.

2. **Restored from `dailyos.db.pre-migration.20260527-133655.bak` (yesterday's pre-migration snapshot, 405 MB).** Opened clean. `IntelProcessor: started`. Foreground commands worked. DB grew 405.4 MB → 405.8 MB over ~20 min of normal use.

3. **Then the bypass-race fired.** A worker (`Claim recompute worker`) hit `SQLCipher key verification failed: disk I/O error` on a fresh `ActionDb::open()` racing the pool writer mid-commit. This is the exact failure mode `src-tauri/src/db_service.rs:5-9` documents and ADR-0133 §1 rejects.

4. **The recovery code wedged the worker supervisor.** `state.rs::db_access_error_requires_manual_recovery` matched the transient bypass-race error as fatal corruption, set a process-wide flag, `IntelProcessor` read the flag and broke, `task_supervisor` restarted it every 2 s, the flag never cleared, the supervisor wedged. 16+ restart cycles observed in minutes.

5. **Then real corruption surfaced on writes**: `pty::write_json_kv` for `background_ai_guard` and `ai_usage_recent` started getting `database disk image is malformed`. Whether the May 27 backup had latent corruption that just took 20 min of writes to surface, or whether the bypass-race interactions produced new corruption during this session, is unresolved. The pattern of dozens of `.corrupt-*` / `.preserve-*` / `.unopenable-*` files in `~/.dailyos/` spanning May 5 → May 28 strongly suggests this isn't a one-session phenomenon.

6. **The user marked one action complete from the daily briefing.** The action's `emit_and_propagate` write hit the malformed DB and `services::actions` dropped it. The user saw the UI update (React state) but the persisted record didn't land. **Silent data loss on the most basic user operation.**

7. **Restored from `dailyos.db.pre-entity-linking-audit-reset.20260523-131834` (May 23, 381 MB, the cleanest known-good).** The schema migration tried to take a pre-migration backup of this file via the chunked path and hit `database disk image is malformed` reading the source. **The May 23 file already had embedded corruption.** This is the moment we realized the corruption isn't from a single session; it's been embedded in the backup chain for weeks.

8. **`sqlcipher .recover` on the 405 MB malformed file OOM'd.** Got 87 bytes of dump before `out of memory (7)`. The CLI can't fit the encrypted DB's per-page recovery metadata in available RAM at this size.

We stopped at step 8 to hold this conversation.

---

## What got fixed today, on `dev` already

PR #416 (`d881dd0a`) ships two patches that close two of the load-bearing bugs surfaced by the above. The fresh session inherits these as substrate.

### `db_backup.rs::backup_database` was using `step(-1)`

`migrations.rs:2637-2642` explicitly documents that `step(-1)` on encrypted DBs in the hundreds of megabytes returns `Ok(StepResult::Done)` while leaving the destination silently inconsistent. The pre-migration backup path (`create_backup_via_api`) has used chunked `PAGES_PER_STEP=1024` with Busy/Locked retry since that comment was written. **The user-facing manual backup never got the fix.** Every `dailyos.db.bak` produced this way against the now-400 MB live DB has been silently malformed on restore. PR #416 replicates the chunked pattern in `db_backup.rs::run_chunked_backup`.

### `state.rs::db_access_error_requires_manual_recovery` matched too aggressively

It treated `"disk i/o error"` and `"sqlcipher key verification failed"` as fatal corruption alongside `"database disk image is malformed"`. The first two messages also fire from transient bypass-race errors. Misclassifying them wedged `task_supervisor` in a 2 s restart loop. PR #416 narrows the predicate to only the actual btree-malformed signal.

Both fixes are tactical. **Neither preempts W1-C bypass closure**, which is the structural fix for the bypass-race class and remains earned per DOS-808.

---

## The recurring shape

The thing that stands out across today's evidence — and across the dozens of `.corrupt-*` files predating today — is the same pattern surfacing in multiple unrelated places.

**Two paths through the same problem domain, one of them silently wrong.**

- The migration backup path used chunked stepping. The user-facing manual backup used `step(-1)`. Same SQLite backup API. Different chunks of code in the same `src-tauri/src/` tree. The broken path "succeeded" silently for months.

- The pool's writer connection uses `with_transaction` and the gate. The fresh `ActionDb::open()` bypass sites at `executor.rs:630/1051`, `meeting_prep_queue.rs:254/518`, `intel_queue.rs:1301`, `glean.rs:1026`, `capture.rs:287`, plus today's surfaced `Claim recompute worker`, `EmbeddingProcessor::sweep_enqueue`, and `DbService::reinit_db_service` recovery path — all open their own OS handles and write through them. Same encrypted database. Different code paths bypass the substrate's serialization. The W1-C ticket exists; the closure hasn't happened.

- The recovery predicate at `state.rs:1631` treated three classes of error identically. Transient WAL race vs. real corruption. The recovery flag had no concept of which it was responding to. PR #416 narrows it, but the underlying recovery design still has only one bit of state for an arbitrarily wide error space.

The same shape shows up in non-DB places too if you start looking. The `useTauriEvent` race we patched yesterday in PR #411 was the same thing in the frontend — two cleanup paths, one of them threw on double-call, the throw broke the surrounding effect chain.

The pattern itself is the substrate signal. It suggests **the boundary between "code that's allowed to talk to storage" and "code that has to go through a service" is in the wrong place**, and people writing code in either category don't have a way to discover the other category exists.

---

## What the corruption pattern is telling us about the substrate

DailyOS treats SQLCipher as both the primary store and the live foreground read path. Linear, Notion, Obsidian, and Slack's `libchat` all moved away from this model at scales DailyOS is now reaching. The throughput plan names this verbatim in §3 and §4 (Alternative B, in-memory claim graph). The plan was right; the moment for it is closer than the plan suggested.

What I find interesting is that **the workspace JSON files at `~/Documents/DailyOS/Accounts/*/dashboard.json`, `Projects/*/dashboard.json`, `People/*/person.json` already exist as a partial source of truth**. The `db_backup::rebuild_from_filesystem` function already takes ~15 minutes to rebuild the SQLite from these files. The accidental architecture is already "workspace as source of truth, SQLite as projection cache" — the code just doesn't quite own that framing yet.

If the framing were intentional:

- Every claim, signal, action mutation would land in workspace JSON first.
- SQLite would be rebuildable from workspace at any time.
- Corruption would stop being catastrophic. `rm -rf ~/.dailyos && pnpm tauri dev` would be the recovery sequence.
- Backups would be `git commit` on the workspace, not custom backup code that silently produces malformed files.
- The me-shape question (per `project_dailyos_me_shape_moat`) becomes more grounded: the user's daily corrections are first-class file-system artifacts they can read, version, and verify. The DB is a query optimization.

This isn't a new direction. It's the direction `db-throughput-architecture.html` §3 already names. Today's evidence is that the load-bearing case for it is here now, not at 50 / 100 / 200 entities (per DOS-808).

---

## Plan substrate the fresh session inherits

Read these before the session. They are the ground truth.

- **`.docs/plans/db-throughput-architecture.html`** — the four-alternative plan (A hygiene / A* partitioned writer queue / B in-memory claim graph / C separate cache service). W0 has shipped. W1-C is earned. B and C are conditional.
- **ADR-0133** (`writer-queue-responsibility`) — names the bypass-site anti-pattern and the invariants that close it. The "what does the writer queue do and what does it explicitly not do" contract.
- **ADR-0134** (`reader-pool-sizing`) — the N+1 latency-tier rule.
- **`docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`** — the K-out doc capturing the 8-instance recurring class. Future L0 packets touching the write path should grep this.
- **DOS-808** — the W1-C earn-signal Linear ticket. Has a long comment from today documenting the bypass-race firing at 20-entity scale under normal use.
- **PR #416** — today's tactical fixes.

---

## What I'd want the fresh session to take seriously

These are not recommendations. They are the options I'd want a planning conversation to hold. Ranked roughly by how much they take advantage of what's already in the codebase.

### 1. Workspace as source of truth, SQLite as projection

Lowest novelty. Highest reuse of existing code (`rebuild_from_filesystem` already does this for accounts/projects/people). Loses the "low-friction action capture" property in the short term — every action write becomes a workspace file write too — but turns corruption from catastrophic to "rebuild in 15 minutes." Naturally splits substrate vs. surface. The conversation should pressure-test: what's currently in SQLite that isn't reflected in workspace? Email enrichment state, meeting history, signal history. Each one becomes a design question.

### 2. Single owning process for SQLite (plan's Alternative C)

One Rust process owns the encrypted store. Tauri renderer, MCP service, future WordPress surface all talk to it over IPC. Kills the bypass-race substrate because there's only ever one process opening the file. Heavier lift. Pays for itself if WordPress + MCP are real near-term surfaces (per `project_v142_wordpress_spike` and `feature_v142_wordpress_spike` memories).

### 3. In-memory claim graph as the foreground read path (plan's Alternative B)

The plan has the design. Today's evidence — `get_account_detail` at 1.5-8 s during enrichment — is the earn signal. Doesn't solve the multi-process surface question. Doesn't solve corruption directly but moves the surface area where corruption matters away from the read path.

### 4. Event-sourced primary, projections everywhere

JSONL log as source of truth. SQLite and the in-memory graph are derived projections, rebuildable at any time. Backups are `cp the log`. This is what Linear actually does under the hood. Highest novelty for the codebase. Likely too big a swing for the next sprint, but worth naming so the planning conversation knows it exists.

### What I would not do

- **Swap SQLCipher for a different embedded DB.** The corruption isn't really SQLCipher's fault. Different substrate, same shape of bug, because the application patterns are what fail. Don't swap substrates without fixing the patterns.
- **Wait for DOS-808's 50/100/200-entity re-measure to "prove" the substrate is wrong.** Today's evidence at 20 entities under normal use is enough. The re-measure was framed when we thought W0 was the close gate. It isn't anymore.

---

## Open questions for the planning session

Concrete things worth chewing on, not in priority order.

1. If workspace JSON becomes the source of truth, what's the write contract? Synchronous? Append-only JSONL per entity? Single rolled-up JSON per entity? How do we handle the case where the workspace and the SQLite projection diverge?

2. Today, multiple background workers each open their own `ActionDb`. Are those workers actually doing things that need a DB connection, or do most just need to read entity state? The bypass-race is bad code, but how many of the sites would just go away if we moved the boundary?

3. The user's positioning is "DailyOS makes intelligence personal" and trust is a product feature. Where does silent corruption rank against feature work in product priority right now? If a single user action can be silently dropped (as it was today), is that a higher-priority fix than W1-C bypass closure?

4. What's the smallest experiment that would tell us whether workspace-as-truth is actually viable? Picking one entity type (accounts? actions?) and routing all writes through workspace + SQLite-projection for a sprint, then measuring whether the user notices?

5. If the plan's Alternative C (separate cache service process) is in play, what's the IPC layer? Unix sockets? Shared memory? gRPC? Tauri already does this for renderer-to-Rust, but extending it to MCP + WordPress is a real design question.

6. The bypass-race wedge today was breaking background enrichment. The foreground UI stayed functional. Is "background enrichment can be degraded but foreground is sacred" a useful design invariant, or are they meant to be the same surface?

7. DailyOS already has a versioned schema migration system. Does the architecture conversation also touch how schema evolution works under the new model, or is that a separate ticket?

---

## Out of scope for the handoff session

The fresh session is for the architectural conversation. The immediate corruption recovery is a separate thread.

- **Recovering the corrupted DB.** The user is open to nuking + `rebuild_from_filesystem`. That decision doesn't need to wait for the architectural conversation.
- **Implementing whatever architectural direction comes out of the session.** This is a conversation, not a plan. A plan comes after.
- **Reviewing PR #416.** It's merged. It works. It's a tactical fix that's already paying off (PR #416's recovery predicate narrowing meant `IntelProcessor` didn't wedge during today's continued malformation — it just kept retrying, which is the correct behavior under degraded substrate).

---

## Specific things to grep / read before the session

- `src-tauri/src/db_backup.rs` — for the chunked-backup pattern + `rebuild_from_filesystem`. The accidental workspace-as-truth architecture is already here.
- `src-tauri/src/db_service.rs:5-21` (file header) — documents the WAL-race failure mode in the project's own words.
- `src-tauri/src/db/core.rs:305-394` `prepare_encrypted_connection` — read this to understand what every fresh `ActionDb::open()` actually does (it runs migrations + four backfills, all writing to the DB).
- `src-tauri/src/state.rs:1454-1491` `recover_db_service_after_access_error` — the post-PR-416 recovery logic.
- `src-tauri/src/intel_queue.rs:748-770` — IntelProcessor's main loop, including the `is_database_recovery_required` break point.
- `~/.dailyos/dailyos.db.corrupt-*` files (don't open them, just `ls`) — the timeline of failed recoveries since May 5.
- `.docs/plans/db-throughput-architecture.html` §1-4 — the throughput problem statement and four-alternative analysis. Section 3 has the Linear/Notion/Slack precedent.

---

## Files referenced by absolute path

- Plan: `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/db-throughput-architecture.html`
- ADR-0133: `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0133-writer-queue-responsibility.md`
- ADR-0134: `/Users/jamesgiroux/Documents/dailyos-repo/.docs/decisions/0134-reader-pool-sizing.md`
- K-out: `/Users/jamesgiroux/Documents/dailyos-repo/docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`
- This doc: `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/substrate-handoff-2026-05-28.md`
- DOS-808 comment with W1-C earn-signal evidence: https://linear.app/a8c/issue/DOS-808
- PR #416 with both tactical fixes: https://github.com/jamesgiroux/daily-operating-system/pull/416
