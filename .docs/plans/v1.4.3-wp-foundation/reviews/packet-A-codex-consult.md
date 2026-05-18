Verdict: CONDITIONAL APPROVE

## Q1 - `load_session_master_key` call sites

Observed grep scope: `rg -n "load_session_master_key" src-tauri/src .docs -S`.

Call site: `src-tauri/src/surface_runtime/mod.rs:667`.
- Observed behavior: `rehydrate_sessions_from_keychain` iterates active DB sessions and calls `load_session_master_key(&row.surface_client_id, &row.session_id)` at `src-tauri/src/surface_runtime/mod.rs:663-670`.
- Current `Some(master_key)` semantics register the session into signed transport at `src-tauri/src/surface_runtime/mod.rs:671-683`.
- New `SessionKeyLookup::Found(master_key)` satisfies this caller: it should keep the current registration path.
- Current `None` semantics push the row into `missing` at `src-tauri/src/surface_runtime/mod.rs:685-687`, then later revoke DB rows as `keychain_entry_missing` at `src-tauri/src/surface_runtime/mod.rs:696-722` and emit `pairing.session.key_missing` audit events at `src-tauri/src/surface_runtime/mod.rs:723-740`.
- New `SessionKeyLookup::NotFound` satisfies the current reconciliation semantics and must be the only variant that enters the existing `missing` branch.
- New `SessionKeyLookup::Unavailable { reason }` does not satisfy the current `None` branch, because the current branch revokes. The packet's proposed behavior is correct: emit diagnostic audit and leave the DB row active.
- New `SessionKeyLookup::Corrupt { reason }` also does not satisfy the current `None` branch. The packet's proposed behavior is correct for avoiding false revocation, but it should be explicit that corrupt payload means "do not register session" and "do not revoke row" in this startup pass.

Call site: `src-tauri/src/services/surface_session_keychain.rs:176`.
- Observed behavior: macOS-only roundtrip test persists a master key at `src-tauri/src/services/surface_session_keychain.rs:174-175`, then expects `load_session_master_key(...).expect(...)` at `src-tauri/src/services/surface_session_keychain.rs:176-177`.
- New `SessionKeyLookup::Found(master_key)` satisfies this caller; the test must change from `Option::expect` to an explicit `Found` match and assert the loaded key at `src-tauri/src/services/surface_session_keychain.rs:178`.
- `NotFound`, `Unavailable`, and `Corrupt` should all fail this test after a successful persist.

Call site: `src-tauri/src/services/surface_session_keychain.rs:183`.
- Observed behavior: the same test deletes the key at `src-tauri/src/services/surface_session_keychain.rs:180-181`, then asserts `load_session_master_key(...).is_none()` at `src-tauri/src/services/surface_session_keychain.rs:182-184`.
- New `SessionKeyLookup::NotFound` satisfies this caller; the test must change from `is_none()` to an explicit `NotFound` match.
- `Unavailable` would indicate keychain failure, not a successful delete; `Corrupt` would indicate an unexpected residual entry.

Function definition: `src-tauri/src/services/surface_session_keychain.rs:107`.
- Observed current return type is `Option<[u8; KEY_BYTES]>` at `src-tauri/src/services/surface_session_keychain.rs:107-110`.
- Observed current implementation collapses CLI spawn failure, non-zero `security` exit, UTF-8 failure, base64 failure, and length mismatch into `None` at `src-tauri/src/services/surface_session_keychain.rs:112-130`.
- The proposed enum is semantically necessary for the production caller because only true not-found should revoke rows.

Could not locate any additional source call sites outside the function's own tests and `rehydrate_sessions_from_keychain`.

## Q2 - Lifecycle revoke/expire call sites and cleanup targets

Observed grep scope: `rg -n "revoke_pairing_row|mark_session_revoked|mark_pairing_expired|revoke_existing_pairing_for_site" src-tauri/src -S`.

Call site: `revoke_existing_pairing_for_site` from pairing handshake at `src-tauri/src/services/surface_pairing.rs:532`.
- Observed behavior: `perform_pairing_handshake` calls `revoke_existing_pairing_for_site(...)` inside `db.with_transaction` at `src-tauri/src/services/surface_pairing.rs:528-538`.
- Observed behavior: the transaction returns `(next_epoch, previous_pairing)` at `src-tauri/src/services/surface_pairing.rs:616-618`.
- Observed behavior: the new session key is persisted after the transaction at `src-tauri/src/services/surface_pairing.rs:623-636`.
- Required cleanup wiring: return the old pairing cleanup target from the transaction alongside `previous_pairing`, then call `delete_session_master_key` after the transaction commits and before persisting the new session key at `src-tauri/src/services/surface_pairing.rs:628-632`.
- This is the only way to satisfy packet AC #11 without keychain IO inside the SQLite transaction.

Call site: `revoke_pairing_row` from explicit `revoke_pairing` at `src-tauri/src/services/surface_pairing.rs:1270`.
- Observed behavior: explicit revoke loads a `RevocationTarget` at `src-tauri/src/services/surface_pairing.rs:1233-1268`, then calls `revoke_pairing_row` inside `db.with_transaction` at `src-tauri/src/services/surface_pairing.rs:1269-1272`.
- Required cleanup wiring: collect affected session ids in or before `revoke_pairing_row`, return a `KeychainCleanupTarget` from the transaction, then delete after `db.with_transaction` returns and before `revoke_pairing` returns the audit event at `src-tauri/src/services/surface_pairing.rs:1273-1286`.
- Existing Tauri command `revoke_surface_client_pairing` calls `revoke_pairing` inside `state.db_write` at `src-tauri/src/commands/surface_runtime.rs:67-75`, forgets in-memory sessions at `src-tauri/src/commands/surface_runtime.rs:76-78`, and emits audit at `src-tauri/src/commands/surface_runtime.rs:79-82`.
- To preserve service-boundary discipline, `revoke_pairing` should perform post-commit keychain cleanup itself; otherwise the command return type must change and the command becomes another lifecycle cleanup caller.

Call site: `revoke_pairing_row` from suspicious replay at `src-tauri/src/services/surface_pairing.rs:1383`.
- Observed behavior: `record_signed_transport_failure` opens one transaction at `src-tauri/src/services/surface_pairing.rs:1297-1404`; when replay count reaches threshold, it calls `revoke_pairing_row(tx, &row, &now, "suspicious_replay")` at `src-tauri/src/services/surface_pairing.rs:1381-1384`.
- Required cleanup wiring: return cleanup target(s) from the transaction alongside the existing audit events, then delete after commit.
- This caller is easy to miss because it is not named in packet section 5.3, but it is a real revoke path and must invoke `delete_session_master_key` after the change.
- Runtime caller `surface_runtime::record_signed_transport_failure` invokes this service inside `app_state.db_write` at `src-tauri/src/surface_runtime/mod.rs:1526-1536`, then evicts in-memory sessions when a `pairing_revoked` event is returned at `src-tauri/src/surface_runtime/mod.rs:1554-1565`.
- Cleanup must not occur inside the `db_write` closure at `src-tauri/src/surface_runtime/mod.rs:1526-1536`; either the service wrapper performs cleanup after its own transaction before returning, or the runtime closure must return cleanup targets to be processed after `.await`.

Call site: `revoke_pairing_row` from `revoke_existing_pairing_for_site` at `src-tauri/src/services/surface_pairing.rs:1855`.
- Observed behavior: the function queries the previous active/suspended/issued pairing at `src-tauri/src/services/surface_pairing.rs:1810-1846`, captures fields for `RevokedPairingRef` at `src-tauri/src/services/surface_pairing.rs:1847-1854`, then revokes via `revoke_pairing_row` at `src-tauri/src/services/surface_pairing.rs:1855`.
- Required cleanup wiring: collect old session ids for the previous `surface_client_id`/pairing epoch before or during this function, return the target to the handshake caller, and delete outside the transaction.

Call site: `mark_session_revoked` from legacy writer validation at `src-tauri/src/services/surface_pairing.rs:763`.
- Observed behavior: `validate_signed_session` marks a session revoked on expiry at `src-tauri/src/services/surface_pairing.rs:760-764`.
- Required cleanup wiring: this function has `row.surface_client_id` from `load_session_pairing` at `src-tauri/src/services/surface_pairing.rs:754-755`, so it can delete `input.session_id` for that surface client after `mark_session_revoked` returns.
- Observed source grep found only tests calling `validate_signed_session`; no production caller was located.

Call site: `mark_session_revoked` from signed read-path write action at `src-tauri/src/services/surface_pairing.rs:1112-1114`.
- Observed behavior: `apply_signed_session_write_action` dispatches `SignedSessionWriteAction::MarkSessionRevoked` to `mark_session_revoked` at `src-tauri/src/services/surface_pairing.rs:1112-1114`.
- Required cleanup wiring: `mark_session_revoked` must return enough data to delete the session key, or `apply_signed_session_write_action` must look up `surface_client_id` before/while revoking and return a cleanup target.
- The caller of `apply_signed_session_write_action` is in `surface_runtime` and currently runs inside `app_state.db_write` at `src-tauri/src/surface_runtime/mod.rs:1126-1135`; keychain delete must happen after that `.await`.

Call site: `mark_pairing_expired` from legacy writer validation at `src-tauri/src/services/surface_pairing.rs:790`.
- Observed behavior: `validate_signed_session` marks the pairing expired when `row.pairing_expires_at <= now` at `src-tauri/src/services/surface_pairing.rs:789-791`.
- Required cleanup wiring: collect all affected session ids for `row.surface_client_id` and delete after the DB update. The current `mark_pairing_expired` updates only the pairing lifecycle at `src-tauri/src/services/surface_pairing.rs:2064-2074`, so the cleanup target must add a session query.

Call site: `mark_pairing_expired` from signed read-path write action at `src-tauri/src/services/surface_pairing.rs:1115-1117`.
- Observed behavior: `apply_signed_session_write_action` dispatches `SignedSessionWriteAction::MarkPairingExpired` to `mark_pairing_expired`.
- Required cleanup wiring: same boundary as `MarkSessionRevoked`; return cleanup target out of the `db_write` closure and delete after commit.

Return-type compatibility verdict:
- Low-level return-type changes are feasible but not drop-in. `revoke_pairing_row`, `mark_session_revoked`, and `mark_pairing_expired` currently return `Result<(), SurfacePairingError>` at `src-tauri/src/services/surface_pairing.rs:1993-1998`, `src-tauri/src/services/surface_pairing.rs:2042-2047`, and `src-tauri/src/services/surface_pairing.rs:2059-2063`.
- `revoke_existing_pairing_for_site` currently returns `Result<Option<RevokedPairingRef>, SurfacePairingError>` at `src-tauri/src/services/surface_pairing.rs:1803-1809`.
- Every caller above must either handle the new cleanup target directly after transaction commit or call a higher-level service wrapper that does so before returning.

## Q3 - `SignedSessionWriteAction::MarkSessionRevoked` dispatch path

Observed path:
- `validate_signed_session_readonly` returns `SignedSessionFailure::SessionExpired { session_id }` when `absolute_expires_at <= now` at `src-tauri/src/services/surface_pairing.rs:888-891`.
- `SignedSessionFailure::write_action()` maps that to `SignedSessionWriteAction::MarkSessionRevoked { session_id, reason: "session_expired" }` at `src-tauri/src/services/surface_pairing.rs:1040-1047`.
- `signed_transport_response` runs readonly validation inside `app_state.db_read` at `src-tauri/src/surface_runtime/mod.rs:1095-1110`.
- On failure, `signed_transport_response` calls `failure.write_action()` at `src-tauri/src/surface_runtime/mod.rs:1119`, then opens a fresh writer closure at `src-tauri/src/surface_runtime/mod.rs:1126-1135`.
- Inside that closure, `apply_signed_session_write_action` dispatches `MarkSessionRevoked` to `mark_session_revoked` at `src-tauri/src/services/surface_pairing.rs:1105-1114`.

Closest cleanup wiring point:
- The closest safe place is immediately after the `app_state.db_write(...).await` at `src-tauri/src/surface_runtime/mod.rs:1126-1135` and before validation rejection events are emitted at `src-tauri/src/surface_runtime/mod.rs:1141-1145`.
- Observed transaction boundary: `db_write` serializes a mutating closure on the writer at `src-tauri/src/state.rs:1493-1524`; keychain CLI IO must not run inside that closure.
- Required shape: change `apply_signed_session_write_action` to return `Option<KeychainCleanupTarget>`; the runtime closure returns it; the async caller deletes keys after `.await`.
- The current `SignedSessionWriteAction::MarkSessionRevoked` carries only `session_id` at `src-tauri/src/services/surface_pairing.rs:1021-1025`, while `delete_session_master_key` needs both `surface_client_id` and `session_id` at `src-tauri/src/services/surface_session_keychain.rs:139`.
- Therefore `mark_session_revoked` or `apply_signed_session_write_action` must query `surface_client_id` for the session before/while updating the row.

Transaction boundary issue:
- There is no blocker if the cleanup target is returned out of the `db_write` closure.
- There is a boundary violation if cleanup is placed inside `apply_signed_session_write_action` as currently called, because that function executes inside the writer closure at `src-tauri/src/surface_runtime/mod.rs:1126-1135`.

## Q4 - `RunningEndpoint` join field feasibility

Observed structure:
- `RunningEndpoint` currently stores `startup_id`, `bound_port`, `runtime_anchor_id`, `shutdown`, and `abort` at `src-tauri/src/surface_runtime/mod.rs:182-188`.
- `JoinHandle` and `AbortHandle` are already imported at `src-tauri/src/surface_runtime/mod.rs:29`.
- The listener task is spawned with `tokio::spawn` at `src-tauri/src/surface_runtime/mod.rs:327-341`; the abort handle is derived from that join at `src-tauri/src/surface_runtime/mod.rs:342`.
- Current state stores only the `AbortHandle` at `src-tauri/src/surface_runtime/mod.rs:348-354`.

Feasibility:
- Adding `join: Option<JoinHandle<()>>` is feasible from a type-storage perspective: `RunningEndpoint` is inside `EndpointInner`, which is inside `parking_lot::Mutex` at `src-tauri/src/surface_runtime/mod.rs:167-178`; `tokio::spawn` returns a `JoinHandle<()>` for a `Send + 'static` task already accepted by the compiler at `src-tauri/src/surface_runtime/mod.rs:327`.
- Drop semantics are acceptable only if `Drop` still sends shutdown and aborts before dropping the join handle. Current `Drop` already sends shutdown and aborts at `src-tauri/src/surface_runtime/mod.rs:468-476`.
- `stop` likewise currently sends shutdown and aborts at `src-tauri/src/surface_runtime/mod.rs:404-420`; adding a join field does not change sync-stop semantics if `stop` continues not to await.

Integration caveat:
- The join handle is currently returned from `start_listener` as part of `Result<(SurfaceEndpointSnapshot, JoinHandle<()>), ...>` at `src-tauri/src/surface_runtime/mod.rs:239-243`.
- `run_until_stopped` awaits that returned handle at `src-tauri/src/surface_runtime/mod.rs:385-399`.
- A single `JoinHandle<()>` cannot be both stored in `RunningEndpoint` for `stop_async` and returned to `run_until_stopped` for awaiting.
- Packet implementation must explicitly refactor this ownership model. Options include storing the join in state and giving `run_until_stopped` another completion signal, or keeping the join owned by the supervisor and implementing `stop_async` against a separate completion primitive.

## Q5 - Tauri shutdown flow and `stop_async`

Observed current flow:
- App setup creates and manages `Arc<AppState>` at `src-tauri/src/lib.rs:167-169` and `src-tauri/src/lib.rs:324-326`.
- The surface runtime endpoint is started by the task supervisor at `src-tauri/src/lib.rs:328-332`.
- The supervisor restarts futures that exit normally at `src-tauri/src/task_supervisor.rs:11-17`.
- The tray quit menu calls `app.exit(0)` at `src-tauri/src/lib.rs:584-585`.
- Main window close is not app shutdown; it prevents close and hides the window at `src-tauri/src/lib.rs:611-616`.
- The app currently calls `.run(tauri::generate_context!())` without a run-event callback at `src-tauri/src/lib.rs:1164`.

Closest shutdown hook:
- The single correct shutdown entry point is a Tauri process-level run-event handler around `src-tauri/src/lib.rs:1164`, not the tray menu at `src-tauri/src/lib.rs:584-585`.
- Wiring only the tray quit menu misses OS/app quit paths; wiring the window close path is wrong because it intentionally hides rather than exits at `src-tauri/src/lib.rs:611-616`.
- The shutdown handler must reach managed `AppState`, then call `state.surface_runtime_endpoint.stop_async(Duration::from_secs(2)).await`.

Integration caveat:
- Because `SurfaceRuntimeEndpoint` is supervised and normal exit restarts at `src-tauri/src/task_supervisor.rs:11-17`, the shutdown path should either run late enough that process teardown wins or add a shutdown-aware guard so `stop_async` does not cause a restart during quit.

## Q6 - `flush_session_activity_on_shutdown` from `stop_async`

Observed current reachability:
- The listener task captures `runtime_for_shutdown` at `src-tauri/src/surface_runtime/mod.rs:326`.
- After `run_listener` returns, the task removes the sentinel at `src-tauri/src/surface_runtime/mod.rs:327-331`.
- The same post-loop block calls `flush_session_activity_on_shutdown(app_state).await` when `runtime_for_shutdown.app_state` exists at `src-tauri/src/surface_runtime/mod.rs:332-339`.
- `flush_session_activity_on_shutdown` is async and takes `&Arc<AppState>` at `src-tauri/src/surface_runtime/mod.rs:753`.
- It performs a best-effort `app_state.db_write(...).await` at `src-tauri/src/surface_runtime/mod.rs:758-782`.
- `run_listener` exits on the shutdown watch signal at `src-tauri/src/surface_runtime/mod.rs:903-908`, then aborts/drains connection tasks at `src-tauri/src/surface_runtime/mod.rs:939-940`.

Verdict:
- Yes, `flush_session_activity_on_shutdown` can run from a graceful `stop_async` context if `stop_async` sends the existing shutdown signal and awaits the listener task instead of aborting it.
- It runs on the same Tokio/Tauri async runtime as the listener because the listener is spawned with `tokio::spawn` at `src-tauri/src/surface_runtime/mod.rs:327`.
- `AppState` is reachable in the supervised Tauri path because `run_supervised_http_endpoint` passes `state.clone()` into `run_until_stopped` at `src-tauri/src/surface_runtime/mod.rs:491-495`, and `start_listener` stores that `app_state` in `EndpointRuntime` at `src-tauri/src/surface_runtime/mod.rs:308-323`.
- It will not run for test starts that pass `None` app state at `src-tauri/src/surface_runtime/mod.rs:230-236` and `src-tauri/src/surface_runtime/mod.rs:239-243`; that is expected.
- It will not run on timeout/abort paths unless the listener reaches the post-loop block before abort; packet AC #19 should remain scoped to graceful async stop.

## Q7 - Acceptance criteria vs fixtures gap

Observed packet counts:
- Packet section 7 lists 22 acceptance criteria at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:210-241`.
- Packet section 8 lists 14 negative fixtures at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:243-260`.
- Packet section 15 already acknowledges the mapping gap at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:360-364`.

Missing or partial fixture coverage:
- AC #1: no fixture or static test proves `load_session_master_key` returns `SessionKeyLookup` rather than `Option`; CI invariant #1 covers it at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:262-267`, but section 8 does not.
- AC #2: no section 8 fixture covers successful exit-0 decode to `Found`; add `dos673_lookup_classifies_found`.
- AC #4: section 8 fixture #2 covers mocked spawn failure only at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:248`; locked-keychain and permission-denied exit classifications are not explicitly covered.
- AC #5: section 8 fixture #3 covers malformed base64 only at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:249`; UTF-8 failure and wrong-length decoded payload are not explicitly covered.
- AC #7: fixtures #2 and #3 assert "DB row not revoked" at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:248-249`, but neither explicitly asserts audit diagnostic emission for `Unavailable` and `Corrupt`.
- AC #8: no fixture or CI invariant proves read-path code does not call the new enum; add a grep/AST invariant for signed route/read-path modules.
- AC #9: no fixture covers the public shape of `KeychainCleanupTarget` fields from `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:225`.
- AC #12: section 8 fixture #7 covers `mark_pairing_expired` cleanup at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:253`, but there is no fixture for the `SignedSessionWriteAction::MarkSessionRevoked` runtime dispatch path.
- AC #13: section 8 fixture #9 covers DB rollback and audit at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:255`, but does not prove logging on cleanup failure.
- AC #15: no fixture covers repeated keychain cleanup/idempotent `delete_session_master_key`; add `dos674_cleanup_target_idempotent`.
- AC #16: section 8 fixture #10 covers `stop` sentinel cleanup at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:256`, but no fixture proves helper call from listener post-loop or `Drop`.
- AC #18: no fixture covers normal Tauri shutdown calling `stop_async`; add an integration test or harness assertion around the Tauri run-event hook.
- AC #20: section 8 fixture #14 proves Drop does not async-flush at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:260`, but does not prove Drop performs sync sentinel cleanup plus abort.
- AC #21: section 8 fixture #10 covers sync `stop`; fixture #12 covers timeout cleanup at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:256-258`; no fixture explicitly covers successful `stop_async` sentinel cleanup.
- AC #22: section 8 fixture #13 covers repeated `stop` only at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:259`; no fixture covers repeated `stop_async` or mixed `stop`/`stop_async`.

## Findings

MEDIUM - Suspicious replay revoke path must be named in DOS-674 cleanup wiring.
- `record_signed_transport_failure` calls `revoke_pairing_row` on replay threshold at `src-tauri/src/services/surface_pairing.rs:1381-1384`.
- The packet names `revoke_pairing_row` generally, but section 5.3 examples do not explicitly include this caller at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:113-126`.
- Without explicit wiring, suspicious-replay revocation can still leave keychain material behind.

MEDIUM - `JoinHandle` ownership requires an explicit runtime refactor.
- The listener join is currently returned from `start_listener` at `src-tauri/src/surface_runtime/mod.rs:239-243` and awaited by `run_until_stopped` at `src-tauri/src/surface_runtime/mod.rs:385-399`.
- The packet proposes storing the same class of handle in `RunningEndpoint` at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:144-151`.
- The packet must specify how `run_until_stopped` observes listener completion after `stop_async` owns or takes the join.

MEDIUM - Tauri shutdown hook is underspecified.
- The current app has a tray `app.exit(0)` path at `src-tauri/src/lib.rs:584-585`, a window-close hide path at `src-tauri/src/lib.rs:611-616`, and no run-event shutdown callback at `src-tauri/src/lib.rs:1164`.
- The packet should require a central Tauri run-event shutdown hook, not menu-specific cleanup.
- The packet should also address supervisor restart behavior from `src-tauri/src/task_supervisor.rs:11-17` during intentional app shutdown.

LOW - section 8 fixture list does not cover all section 7 acceptance criteria.
- The highest-value missing fixtures are: `dos673_lookup_classifies_found`, locked/permission denied unavailable classification, corrupt UTF-8 and wrong-length payloads, unavailable/corrupt audit emission, no read-path enum usage, `SignedSessionWriteAction::MarkSessionRevoked` cleanup, idempotent keychain cleanup, Tauri shutdown `stop_async`, Drop sentinel cleanup, successful `stop_async` sentinel cleanup, and repeated `stop_async`.
