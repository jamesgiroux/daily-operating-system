---
module: db
tags: [sqlite, wal, dev-workflow, recovery, replica]
problem_type: runtime-failure
date: 2026-06-11
---

# Deleting -wal/-shm under a live app wedges every DB session — graceful SIGINT recovers the orphaned WAL

## Symptom

Frontend stuck in permanent loading state on every surface; no errors visible. Even fresh `sqlite3 "file:...?mode=ro"` opens fail with `unable to open database file (14)` — WAL-mode read-only connections require the `-shm` file.

## Root cause

The replica DB's `-wal` (36 MB) and `-shm` files were deleted from disk while the dev app held them open. `lsof -p <pid> | grep .dailyos` showed open FDs on the unlinked inodes; `ls` showed no files. SQLite sessions whose WAL vanishes underneath them error or wedge on every operation; every IPC command hangs; the frontend renders loading forever. The known recovery recipe for the duplicate-instance class (`kill all + rm -wal/-shm + relaunch`) causes exactly this when the `rm` runs **before** the kill.

What it is NOT: not the `WRITE_TRANSACTION_GATE`, not the DOS-866 starvation class, not two writers on one DB (verify with `lsof` per process — dev on replica, installed app on live were cleanly separated), not the `rebuild.lock` (flock-based, held shared in normal operation; the lingering 0-byte file is cosmetic).

## Diagnosis procedure

1. `pgrep -fl dailyos` — rule out duplicate instances on ONE db via `lsof -p <pid> | grep "\.dailyos"` (check which DB files each process actually holds).
2. Compare `lsof` FDs against `ls` — open-but-unlinked `-wal`/`-shm` is the smoking gun.
3. `ls -i` the main DB file vs the inode in `lsof` — same inode means a graceful close can still save the WAL contents.
4. `sqlite3 "file:...?immutable=1" "PRAGMA quick_check;"` verifies main-file integrity without needing `-shm`.

## Recovery (preserves the orphaned WAL's committed writes)

1. **Graceful quit, never SIGKILL**: `kill -INT -<pgid of the dev server>` (Ctrl+C semantics). On clean close SQLite checkpoints the orphaned WAL into the still-linked main file via the process's own handles. Observed: main file grew 381→397 MB on close; 1,050 claims recovered; `quick_check` ok.
2. Relaunch (`DAILYOS_DB_MODE=replica pnpm tauri dev`) — fresh `-wal`/`-shm` created.
3. Only if the main file fails `quick_check`: rebuild the replica from the live workspace per the documented procedure.

## Prevention

- The `rm -wal/-shm` step of any recovery recipe runs ONLY after `pgrep` confirms zero app processes.
- Candidate hardening (ticket-worthy, not yet built): app health check detects `st_nlink == 0` on its WAL and fails loud ("database files modified externally — restart me") instead of infinite loading.
