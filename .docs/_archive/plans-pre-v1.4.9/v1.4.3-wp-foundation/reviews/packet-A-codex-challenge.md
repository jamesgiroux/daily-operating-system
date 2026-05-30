BLOCK

- [CRITICAL] Target 4, cleanup-outside-transaction can strand revoked-session keychain entries forever.
  The packet says lifecycle cleanup runs after SQLite commit and failures are "retried on later reconciliation" (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:123-126`).
  The only named startup reconciliation path reads `surface_client_sessions` with `s.revoked_at IS NULL` and active pairings only (`src-tauri/src/surface_runtime/mod.rs:634-640`).
  That means a session successfully revoked in SQLite, then followed by process death before `delete_session_master_key`, is excluded from the next startup sweep (`src-tauri/src/surface_runtime/mod.rs:696-741`).
  The other reviewers' claim is correct: rehydration does not catch this orphan because revoked rows are filtered out before keychain lookup (`src-tauri/src/surface_runtime/mod.rs:638-640`).
  The current idempotent delete primitive exists, but nothing in the rehydrate path calls it for already-revoked sessions (`src-tauri/src/services/surface_session_keychain.rs:139-156`, `src-tauri/src/surface_runtime/mod.rs:624-747`).
  The packet's cleanup target is volatile process memory, so it disappears exactly in the crash window it is supposed to recover from (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:102-126`).
  This is worse because the local session absolute TTL is 365 days (`src-tauri/src/services/surface_pairing.rs:21-25`).
  V2 must add a durable cleanup backlog, a startup cleanup sweep over revoked/expired sessions, or a DB cleanup marker that is cleared only after keychain delete succeeds.
  Without that, §6.2's external-side-effect-after-commit design is incomplete, not merely best-effort (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:171-179`).
  Acceptance criteria #13 says delete failure logs and emits audit without rollback, but it does not require a retryable durable record (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:229-231`).
  Acceptance criteria #10-12 cover the happy lifecycle transitions, not the post-commit/pre-delete crash recovery path (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:223-230`).

- [HIGH] Target 1, `SessionKeyLookup::Unavailable` collapses transient keychain outages and permanent authorization denials.
  The packet maps CLI spawn failure, locked keychain, permission denied, and daemon-not-running into one `Unavailable { reason }` bucket (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:81-85`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:89-93`).
  Existing keychain code already treats all non-success `security find-generic-password -w` results as `None`, which is the defect this packet is fixing (`src-tauri/src/services/surface_session_keychain.rs:107-123`).
  Existing DB-key keychain code recognizes only not-found signatures as absence: "could not be found", "item not found", and `-25300` (`src-tauri/src/db/key_provider.rs:989-994`).
  The repo already has a negative fixture showing `"User interaction is not allowed"` is not not-found (`src-tauri/src/db/key_provider.rs:1231-1240`).
  Scenario classification: locked keychain mid-call can reasonably be `Unavailable`, because the item may still exist and the keychain is temporarily unreadable (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:92-93`).
  Scenario classification: `securityd` restart can reasonably be `Unavailable`; the current surface keychain runner already retries "temporarily unavailable", `os error 35`, and `EAGAIN` before returning the final output (`src-tauri/src/services/surface_session_keychain.rs:44-74`).
  Scenario classification: ACL refusal is not the same operational state as a locked keychain, because the keychain service is reachable but DailyOS is not authorized to read the item; the packet still puts "permission denied" under `Unavailable` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:92-93`).
  Scenario classification: user-denied prompt is not the same as daemon-not-running; the existing repo treats "User interaction is not allowed" as a hard keychain read failure in the surface runtime anchor path (`src-tauri/src/db/key_provider.rs:1207-1228`).
  The ambiguity matters because §5.2 says `Unavailable` leaves the DB row active and only emits audit (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:95-100`).
  `Corrupt` is only defined for exit-0 payload decoding failures, so it cannot represent "entry exists but DailyOS cannot read it" authorization cases (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:89-93`).
  `NotFound` is explicitly reserved for documented item-not-found status or stderr signatures, so it also cannot represent ACL refusal or user-denied prompt (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:89-93`).
  That leaves ACL refusal and user-denied prompt in `Unavailable`, but the packet gives `Unavailable` only retry/audit semantics, not repair semantics (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:99-100`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:219-221`).
  A permanent ACL denial would therefore stay active, emit repeatedly at startup, and never drive an explicit re-pair or user-facing repair state (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:219-221`).
  V2 should split `Unavailable` into at least transient-unavailable vs access-denied/user-denied, or define a remediation policy that does not leave permanent authorization failures in the same lifecycle state as a temporary locked keychain.

- [HIGH] Target 2, `KeychainCleanupTarget` is safe only if old session-id capture and DB revocation happen in the same transaction, and the packet does not pin that down tightly enough.
  The packet says `revoke_existing_pairing_for_site` "collects old session ids" and returns cleanup for the old pairing before the new one is persisted (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:116-122`).
  The current handshake wraps replacement in a single `db.with_transaction` block, calling `revoke_existing_pairing_for_site` before inserting the new pairing and new session (`src-tauri/src/services/surface_pairing.rs:528-617`).
  The current `revoke_existing_pairing_for_site` first selects the previous pairing, then calls `revoke_pairing_row` (`src-tauri/src/services/surface_pairing.rs:1803-1856`).
  The current returned `RevokedPairingRef` does not include session ids (`src-tauri/src/services/surface_pairing.rs:1607-1616`).
  If V2 collects old session ids outside that transaction, a second lifecycle path can change `revoked_at` between collection and revoke; the service already has separate expiry/revocation paths through `mark_session_revoked`, `mark_pairing_expired`, and suspicious replay revocation (`src-tauri/src/services/surface_pairing.rs:1112-1117`, `src-tauri/src/services/surface_pairing.rs:1368-1384`).
  If V2 collects after `revoke_pairing_row` using `revoked_at IS NULL`, the target can be empty because `revoke_pairing_row` sets every session for the old `surface_client_id` revoked (`src-tauri/src/services/surface_pairing.rs:2010-2019`).
  If V2 collects by `surface_client_id` only, it mirrors the broad existing revoke predicate and can over-collect if historical rows share that client id (`src-tauri/src/services/surface_pairing.rs:2010-2019`).
  The packet says "captured-old-session-ids" but does not specify `pairing_epoch` as part of the capture predicate (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:223-228`).
  V2 should require `SELECT session_id FROM surface_client_sessions WHERE surface_client_id = ? AND pairing_epoch = ?` inside the same transaction and before `revoke_pairing_row`.
  V2 should also require the cleanup target to be carried out of the existing `complete_handshake` transaction along with `previous_pairing`, because the current transaction boundary is the only thing preventing the collect/revoke race (`src-tauri/src/services/surface_pairing.rs:528-617`).

- [HIGH] Target 3, bounded `stop_async` probably does not abort a SQLite transaction mid-statement, but the packet does not define the late-write semantics it creates.
  The packet proposes `stop_async(timeout)` that sends shutdown, awaits the listener join with a 2s-style timeout, then performs sentinel cleanup and aborts (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:144-151`).
  The existing listener shutdown path breaks the accept loop, aborts all connection tasks, then awaits them (`src-tauri/src/surface_runtime/mod.rs:897-941`).
  Surface request handlers do perform `db_write` work from connection tasks, including quarantine writes and pairing-code failure writes (`src-tauri/src/surface_runtime/mod.rs:1112-1135`, `src-tauri/src/surface_runtime/mod.rs:1238-1257`).
  `AppState::db_write` submits the closure to the DbService writer and awaits the pooled call (`src-tauri/src/state.rs:1493-1524`).
  The pooled DB worker runs the submitted closure on a dedicated OS thread and sends the result back over a channel (`src-tauri/src/db_service.rs:123-131`).
  Because the closure is already handed to the writer thread, aborting the async request future should not interrupt a synchronous rusqlite closure mid-transaction (`src-tauri/src/db_service.rs:157-172`).
  The unresolved failure mode is different: `stop_async` can return after timeout while a writer-thread closure is still finishing, so shutdown state can claim "stopped" before the write is actually durable (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:147-151`, `src-tauri/src/db_service.rs:123-131`).
  The post-loop shutdown flush also uses `db_write`, so aborting the listener join while it waits on that flush can abandon the await even if the writer thread continues (`src-tauri/src/surface_runtime/mod.rs:327-340`, `src-tauri/src/surface_runtime/mod.rs:753-782`).
  Existing `stop()` sends the shutdown signal and immediately calls `abort`, so the current bug is real and the proposed async join is directionally necessary (`src-tauri/src/surface_runtime/mod.rs:404-420`).
  Existing `Drop` has the same immediate signal-then-abort shape and has no async DB access, matching the packet's "best-effort sentinel-only" constraint (`src-tauri/src/surface_runtime/mod.rs:468-477`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:151-157`).
  The implementation must store the join handle because current `RunningEndpoint` stores only startup metadata, shutdown sender, and abort handle (`src-tauri/src/surface_runtime/mod.rs:182-188`).
  V2 must state whether timed-out shutdown waits for in-flight DB writer work, tolerates late writer completion after endpoint stop, or intentionally skips flush without implying the writer was cancelled.
  The current packet's fixture `dos675_timeout_aborts_with_sentinel_cleaned` only proves sentinel cleanup and abort, not DB writer drain semantics (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:256-260`).

- [HIGH] Target 5, CI invariant #4 is currently unenforceable because `db_writer_observer` does not exist in the repo.
  The packet requires a test-only `db_writer_observer` that records writer-lock duration (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:262-270`).
  `AppState::db_write` currently has no observer hook; it obtains the writer and calls the pooled connection directly (`src-tauri/src/state.rs:1493-1524`).
  `DbService::writer` only returns the cloned pooled writer connection (`src-tauri/src/db_service.rs:524-527`).
  `PooledConnection::call` sends the closure to the worker and awaits a oneshot; there is no duration callback, counter, or test hook in that path (`src-tauri/src/db_service.rs:157-172`).
  Grep evidence: `rg -n "db_writer_observer|writer_observer" src-tauri/src` returns no matches.
  The current worker message type carries only the task and response channel, so there is no place for an observer callback today (`src-tauri/src/db_service.rs:44-54`).
  The current worker loop measures no start/end timestamps around `run_task` (`src-tauri/src/db_service.rs:123-131`).
  This blocks L0 closure because §15 requires all six CI invariants to have concrete enforcement (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:360-365`).
  V2 must either add the observer primitive to the implementation scope or replace invariant #4 with an enforceable integration fixture based on existing DbService hooks.
  A grep-only fallback will not prove this invariant because the packet itself says static grep is only an approximation and runtime proof is invariant #4 (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:267-270`).
  If V2 keeps invariant #4, the observer belongs in `DbService` or `AppState::db_write`, because all writer work is funneled through that shared path (`src-tauri/src/state.rs:1493-1524`, `src-tauri/src/db_service.rs:524-527`).

- [MEDIUM] Target 6, `Unavailable` audit emission has no blast-radius cap.
  The packet requires `Unavailable` and `Corrupt` to emit an audit diagnostic per occurrence while leaving the DB row active (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:219-221`).
  The packet itself flags sustained keychain outage audit flood as an open question, not a resolved design (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:318-327`).
  The current audit path is append-only JSONL, not a relational table; each record is one line in `~/.dailyos/audit.log` (`src-tauri/src/audit_log.rs:1-5`, `src-tauri/src/audit_log.rs:17-20`).
  Each pairing audit emission locks the audit logger and writes through `emit_pairing_audit` (`src-tauri/src/surface_runtime/mod.rs:3255-3259`, `src-tauri/src/services/surface_pairing.rs:1407-1419`).
  Each append opens the audit file and writes exactly one line (`src-tauri/src/audit_log.rs:384-402`).
  The startup rehydrate loop processes every active session row (`src-tauri/src/surface_runtime/mod.rs:624-668`).
  The existing missing-key path emits one audit event per affected session (`src-tauri/src/surface_runtime/mod.rs:723-741`), and the packet asks the new `Unavailable` arm to do the same (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:219-221`).
  The blast-radius formula is therefore `active surface sessions * startup rehydrate attempts` during the outage.
  Active sessions can stay valid for 365 days, so this is not naturally bounded to a tiny row set (`src-tauri/src/services/surface_pairing.rs:21-25`).
  The rehydrate query does not cap row count; it selects every active, non-expired session matching active pairings (`src-tauri/src/surface_runtime/mod.rs:629-657`).
  The current missing-key implementation then loops all misses and emits one audit line per miss (`src-tauri/src/surface_runtime/mod.rs:698-741`).
  The supervisor restarts normally exited background tasks (`src-tauri/src/task_supervisor.rs:13-17`), and the endpoint task returns after a start error path sleeps five seconds (`src-tauri/src/surface_runtime/mod.rs:491-499`), so repeated start attempts can multiply the per-start event count when startup fails for adjacent reasons.
  V2 should coalesce by `(variant, reason, surface_client_id_hash or startup_id)` over a defined window, or record one summary event with counts for the rehydrate pass.
  The coalescing decision belongs in L0 because §15 requires all open questions to be resolved before implementation begins (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:360-367`).

- [MEDIUM] Deeper class issue: this packet needs a general external-side-effect contract for DB lifecycle transitions.
  Pairing creation already commits the new DB session before persisting the keychain master key (`src-tauri/src/services/surface_pairing.rs:528-617`, `src-tauri/src/services/surface_pairing.rs:623-636`).
  Packet A adds the inverse side effect, deleting old keychain state after DB revocation commits (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:123-126`).
  Both directions create the same class of inconsistency: DB committed, keychain side effect not completed (`src-tauri/src/services/surface_pairing.rs:623-636`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:171-179`).
  Startup rehydration handles one side only: active DB row with missing keychain entry (`src-tauri/src/surface_runtime/mod.rs:614-620`, `src-tauri/src/surface_runtime/mod.rs:696-741`).
  It does not handle revoked/expired DB row with still-present keychain entry (`src-tauri/src/surface_runtime/mod.rs:634-640`).
  V2 should name the invariant for both directions and add one reconciliation mechanism instead of solving only the false-revocation branch.

- [LOW] Target 1 test plan should pin exact macOS `security` stderr/status fixtures, not only mocked spawn failure.
  The negative fixture list includes mocked spawn failure and corrupt payload, but only "documented item-not-found" for real CLI classification (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:243-250`).
  The reviewer panel explicitly asks for locked keychain, access prompt timeout, and daemon restart stress tests (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:347-354`).
  Existing DB-key tests already encode concrete stderr examples for not-found and user-interaction denial (`src-tauri/src/db/key_provider.rs:1231-1240`).
  V2 should add fixture strings for locked keychain, ACL refusal, user-denied prompt, daemon restart/transient failure, and unknown non-zero exit so the enum cannot drift back into `None` semantics.
