## 1. Verdict
APPROVE-WITH-FOLLOWUPS — observed diffs reviewed for `ef6c3ce2`, `0edda6ff`,
`df8a668a`, `3429022a`, `97a02070`; follow-ups are CI-hook strength, not
AC #16/blocking.

## 2. AC-16 fix validation
Observed: AC #16 is satisfied in current code.
`mark_pairing_expired` opens one `db.with_transaction(|tx| { ... })` boundary at
`src-tauri/src/services/surface_pairing.rs:2257`.
Observed: DB op 1, expired active `pairing_epoch` lookup, runs through
`tx.conn_ref().query_row(...)` inside that closure at
`src-tauri/src/services/surface_pairing.rs:2258-2272`.
Observed: DB op 2, cleanup `session_id` snapshot with `revoked_at IS NULL`,
runs inside the same closure before the lifecycle UPDATE at
`src-tauri/src/services/surface_pairing.rs:2273-2291`.
Observed: DB op 3, pairing expiry UPDATE, runs through `tx.conn_ref().execute(...)`
before closure return at `src-tauri/src/services/surface_pairing.rs:2299-2309`.
Observed: the transaction closes after `Ok(cleanup_target)` and maps errors once
at `src-tauri/src/services/surface_pairing.rs:2309-2311`.
Observed: this matches the revoke precedent: collect target, call
`revoke_pairing_row`, return target from one transaction at
`src-tauri/src/services/surface_pairing.rs:1320-1326`.
Observed: no keychain IO appears in `mark_pairing_expired`; keychain deletion
remains in `cleanup_session_keychain_entries` at
`src-tauri/src/services/surface_pairing.rs:1492-1501`.

## 3. Plumbing coverage
| Dispatch path | CleanupTarget plumbing | Validation |
|---|---|---|
| `SessionExpired` -> `MarkSessionRevoked` | Carries `surface_client_id` + `session_id` at `src-tauri/src/services/surface_pairing.rs:908-912`; `write_action()` preserves both at `src-tauri/src/services/surface_pairing.rs:1085-1092`. | PASS: apply returns `mark_session_revoked(...)` target at `src-tauri/src/services/surface_pairing.rs:1157-1161`; runtime cleans after `.db_write(...).await` at `src-tauri/src/surface_runtime/mod.rs:1174-1195`. |
| `PairingExpired` -> `MarkPairingExpired` | Carries `surface_client_id` at `src-tauri/src/services/surface_pairing.rs:924-927`; `write_action()` preserves it at `src-tauri/src/services/surface_pairing.rs:1093-1097`. | PASS: apply returns `mark_pairing_expired(...)` target at `src-tauri/src/services/surface_pairing.rs:1162-1164`; runtime cleans after await at `src-tauri/src/surface_runtime/mod.rs:1188-1195`. |
| `SiteNonceMismatch` -> `SuspendPairing` | Dispatches with `surface_client_id` + reason `site_nonce_mismatch` at `src-tauri/src/services/surface_pairing.rs:1098-1102`. | PASS: no cleanup expected; `cleanup_reason()` returns `None` for `SuspendPairing` at `src-tauri/src/services/surface_pairing.rs:1068-1075`, and apply returns `Ok(None)` at `src-tauri/src/services/surface_pairing.rs:1165-1170`. |
| `SiteBindingDigestMismatch` -> `SuspendPairing` | Dispatches with `surface_client_id` + reason `site_binding_mismatch` at `src-tauri/src/services/surface_pairing.rs:1104-1108`. | PASS: no keychain target expected; same `SuspendPairing` apply path returns `Ok(None)` at `src-tauri/src/services/surface_pairing.rs:1165-1170`. |
| `WpUserHashMismatch` -> `SuspendPairing` | Dispatches with `surface_client_id` + reason `wp_user_mismatch` at `src-tauri/src/services/surface_pairing.rs:1110-1114`. | PASS: no cleanup expected; existing suspend regression still calls `apply_signed_session_write_action` at `src-tauri/src/services/surface_pairing.rs:3963-3975`. |
| Runtime caller | `cleanup_reason` captured before moving the action into the writer closure at `src-tauri/src/surface_runtime/mod.rs:1174-1179`. | PASS: cleanup runs only after `db_write(...).await`, outside the writer closure, at `src-tauri/src/surface_runtime/mod.rs:1188-1195`. |

Observed: all `apply_signed_session_write_action` call sites are accounted for:
production runtime at `src-tauri/src/surface_runtime/mod.rs:1180`, direct test
coverage for session revoke at `src-tauri/src/services/surface_pairing.rs:3453`,
and suspend regression coverage at `src-tauri/src/services/surface_pairing.rs:3969`.
Observed: cleanup-returning paths have a post-await cleanup carrier; suspend-only
paths deliberately return `Ok(None)` and do not need `KeychainCleanupTarget`.

## 4. Invariant test coverage
| §9 invariant | Coverage in `services/tests/dos673_674_675_invariants.rs` | Result |
|---|---|---|
| #1 `load_session_master_key` returns `SessionKeyLookup` not `Option` | Extracts `pub fn load_session_master_key`, asserts `-> SessionKeyLookup`, rejects `-> Option<`, and checks the 3 enum variants at `src-tauri/src/services/tests/dos673_674_675_invariants.rs:38-54`. | PASS |
| #2 no `KeychainBackend`/keychain IO inside writer closures | Asserts one direct `delete_session_master_key(` in `surface_pairing`, confirms it is in `cleanup_session_keychain_entries`, and line-scans keychain calls for `db_write` / `with_transaction` on the same line at `src-tauri/src/services/tests/dos673_674_675_invariants.rs:56-87`. | GAP: executable grep hook exists, but it is not the AST/body closure scan described by L0 §9. |
| #3 `keychain_entry_missing` revocation gated on `NotFound` only | Extracts `rehydrate_sessions_from_keychain`, checks `SessionKeyLookup::NotFound` queues `missing.push(row)`, and checks the `Unavailable` tail does not at `src-tauri/src/services/tests/dos673_674_675_invariants.rs:89-110`. | PASS |
| #4 `stop` and `Drop` call `explicit_sentinel_cleanup` before abort | Extracts `pub fn stop(&self)` and `fn drop(&mut self)`, then asserts cleanup text precedes `endpoint.abort.abort()` at `src-tauri/src/services/tests/dos673_674_675_invariants.rs:112-130`. | PASS |
| #5 `Drop` does not call async functions | Extracts `Drop::drop` and rejects `.await`, `db_write`, `flush_session_activity_on_shutdown`, and `stop_async` at `src-tauri/src/services/tests/dos673_674_675_invariants.rs:132-140`. | PASS |

Observed verification command:
`CARGO_TARGET_DIR=/private/tmp/dailyos-pa-cargo-target cargo test --manifest-path src-tauri/Cargo.toml --lib dos673_674_675_invariants -- --nocapture`.
Observed result: 5 passed, 0 failed, 0 ignored, 2493 filtered out.
Observed limitation: invariant #2 passes today, but does not implement the exact
AST/body scanner described in L0 §9.

## 5. Test seam reachability
Observed: `KeychainBackend` is public with `find`, `persist`, and `delete` at
`src-tauri/src/services/surface_session_keychain.rs:54-58`.
Observed: `RealKeychain` is the production implementation at
`src-tauri/src/services/surface_session_keychain.rs:134-174`.
Observed: production dispatch uses `#[cfg(not(test))] fn with_keychain_backend`
to call `RealKeychain` at `src-tauri/src/services/surface_session_keychain.rs:176-179`.
Observed: test dispatch has a `thread_local!` override plus
`with_keychain_for_tests` / `set_keychain_for_tests` at
`src-tauri/src/services/surface_session_keychain.rs:181-223`.
Observed: public keychain functions delegate through the seam at
`src-tauri/src/services/surface_session_keychain.rs:228-248`, and
`MockKeychain` implements the trait at
`src-tauri/src/services/surface_session_keychain.rs:251-346`.
Observed: existing tests reach the seam in keychain tests at
`src-tauri/src/services/surface_session_keychain.rs:469-485`, surface pairing
tests at `src-tauri/src/services/surface_pairing.rs:3308-3312`, and runtime
rehydration tests at `src-tauri/src/surface_runtime/mod.rs:4896-4915`.

## 6. CI hook feasibility
Observed: the invariant tests are compiled into lib tests because `services::tests`
is included under `#[cfg(test)]` at `src-tauri/src/services/mod.rs:53-54`.
Observed: existing local gates run `cargo test --lib` for Rust changes in
pre-commit and pre-push at `.githooks/pre-commit:82-85` and `.githooks/pre-push:110-112`.
Observed: existing GitHub Rust CI can run them via `cargo test --workspace` at
`.github/workflows/rust.yml:76-78`, but that workflow is push-to-main only at
`.github/workflows/rust.yml:4-11` and `.github/workflows/rust.yml:17-27`.
Observed: PR CI in `lint-frontend.yml` installs `ripgrep` and runs guard scripts,
but it does not currently run cargo tests at `.github/workflows/lint-frontend.yml:76-139`.
Observed: §9 hooks are feasible through existing cargo test plumbing; invariant #2
needs a stronger body/AST scanner if the packet requires the exact §9 enforcement text.

## 7. AC-adjacent findings
1. MEDIUM — `src-tauri/src/services/tests/dos673_674_675_invariants.rs:56-87`:
L0 §9 invariant #2 calls for an AST/body gate over writer closures, but the
implemented test is a line/occurrence grep.
Observed gap: it only rejects keychain calls when the same line also contains
`db_write` or `with_transaction` at
`src-tauri/src/services/tests/dos673_674_675_invariants.rs:73-83`.
Hypothesis: a future regression that calls `cleanup_session_keychain_entries(...)`
or a trait method from a multi-line writer closure would not be caught by this test.
2. LOW — `.github/workflows/rust.yml:4-11`: section 9 invariants are executable
by existing Rust CI on push to `main`, but not by PR CI.
Observed evidence: Rust workflow runs `cargo test --workspace` at
`.github/workflows/rust.yml:76-78`; PR `lint-frontend.yml` runs guard scripts and
TypeScript checks at `.github/workflows/lint-frontend.yml:89-139` but no cargo test.
If the desired gate is pre-merge GitHub CI rather than local hooks/L2 review, wire
the invariant lib test or a grep script into PR CI.

## 8. Path-alpha findings
Linear maintenance: none observed in AC-adjacent scope; no new filing against
`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.
