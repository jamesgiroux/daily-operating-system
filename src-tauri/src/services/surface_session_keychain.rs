//! Keychain persistence for signed-surface-session master keys.
//!
//! Stores the 32-byte HMAC master key derived during pairing so the runtime
//! can rehydrate session state across Tauri restarts. Without this, every
//! restart invalidates every paired WP session and forces re-pairing — the
//! observed failure mode that motivated this module.
//!
//! ## Storage shape
//!
//! - Service: `com.dailyos.desktop.surface-session.<surface_client_id>`
//! - Account: `<session_id>`
//! - Password (raw bytes, base64-encoded for keychain transport): 32-byte master key
//!
//! ## Defense
//!
//! Code-signing-bound app isolation: the keychain entry is owned by the
//! DailyOS-signed binary. A separate binary signed by a different team ID
//! cannot `SecItemCopyMatching` the entry. Negative fixture
//! `signing_team_keychain_isolation` is authoritative.
//!
//! Implementation uses the `security` CLI (same pattern as
//! `gravatar::keychain` — see `services/keychain.rs` in this crate for
//! prior art). The CLI defaults to current-app-only ACL when `-T` flag is
//! omitted on add-generic-password.
//!
//! ## Reconciliation
//!
//! If a session row exists in `surface_client_sessions` but the keychain
//! entry is missing (user-deleted, machine-migrated), the runtime should
//! mark the session `revoked_at = now()` with reason `keychain_entry_missing`
//! and surface `session_requires_repair` to the WP plugin. Reconciliation
//! is the consumer's responsibility; this module just provides the lookup
//! primitive.

use base64::Engine as _;

const SERVICE_PREFIX: &str = "com.dailyos.desktop.surface-session";
const KEY_BYTES: usize = 32;

fn service_name(surface_client_id: &str) -> String {
    format!("{SERVICE_PREFIX}.{surface_client_id}")
}

/// Run a `security` CLI command with retry for transient Keychain contention.
/// Mirrors `gravatar::keychain::run_security_cmd` pattern.
fn run_security_cmd(args: &[&str]) -> Result<std::process::Output, String> {
    const MAX_RETRIES: u32 = 4;
    const BASE_MS: u64 = 150;

    for attempt in 0..=MAX_RETRIES {
        let output = std::process::Command::new("security")
            .args(args)
            .output()
            .map_err(|err| format!("security command failed: {err}"))?;

        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let is_transient = stderr.contains("temporarily unavailable")
            || stderr.contains("os error 35")
            || stderr.contains("EAGAIN");

        if !is_transient || attempt == MAX_RETRIES {
            return Ok(output);
        }

        let delay = std::time::Duration::from_millis(BASE_MS * 2u64.pow(attempt));
        std::thread::sleep(delay);
    }

    unreachable!()
}

/// Persist a 32-byte session master key for the given surface_client_id +
/// session_id pair. Idempotent: an existing entry is replaced via
/// `-U` (upsert) on add-generic-password.
pub fn persist_session_master_key(
    surface_client_id: &str,
    session_id: &str,
    master_key: &[u8; KEY_BYTES],
) -> Result<(), String> {
    let service = service_name(surface_client_id);
    let encoded = base64::engine::general_purpose::STANDARD.encode(master_key);
    let output = run_security_cmd(&[
        "add-generic-password",
        "-a",
        session_id,
        "-s",
        &service,
        "-w",
        &encoded,
        "-U", // upsert (replace existing)
    ])?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("keychain persist failed: {stderr}"));
    }
    Ok(())
}

/// Retrieve a previously-persisted session master key. Returns `None` if
/// no entry exists for this surface_client_id + session_id pair (which
/// signals reconciliation: mark the DB row revoked with reason
/// `keychain_entry_missing`).
pub fn load_session_master_key(
    surface_client_id: &str,
    session_id: &str,
) -> Option<[u8; KEY_BYTES]> {
    let service = service_name(surface_client_id);
    let output = run_security_cmd(&[
        "find-generic-password",
        "-a",
        session_id,
        "-s",
        &service,
        "-w",
    ])
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let encoded = String::from_utf8(output.stdout).ok()?;
    let trimmed = encoded.trim();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .ok()?;
    if bytes.len() != KEY_BYTES {
        return None;
    }
    let mut key = [0u8; KEY_BYTES];
    key.copy_from_slice(&bytes);
    Some(key)
}

/// Remove a session master key from keychain — called when a session is
/// revoked or when reconciliation discovers a stale DB row.
pub fn delete_session_master_key(surface_client_id: &str, session_id: &str) -> Result<(), String> {
    let service = service_name(surface_client_id);
    let output = run_security_cmd(&[
        "delete-generic-password",
        "-a",
        session_id,
        "-s",
        &service,
    ])?;
    // `delete-generic-password` returns non-zero if the entry didn't exist;
    // treat that as success (idempotent delete).
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("could not be found") && !stderr.contains("SecKeychainSearchCopyNext") {
            return Err(format!("keychain delete failed: {stderr}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: these tests require macOS Keychain access and will be skipped on
    // CI / non-darwin platforms. Full integration testing for code-signing
    // isolation lives in tests/.

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn roundtrip_persists_and_loads_master_key() {
        let surface_client_id = format!("dos655-test-{}", std::process::id());
        let session_id = format!("session-{}", std::process::id());
        let master_key = [42u8; KEY_BYTES];

        persist_session_master_key(&surface_client_id, &session_id, &master_key)
            .expect("persist should succeed");
        let loaded = load_session_master_key(&surface_client_id, &session_id)
            .expect("load should return the persisted key");
        assert_eq!(loaded, master_key);

        delete_session_master_key(&surface_client_id, &session_id)
            .expect("delete should succeed");
        assert!(
            load_session_master_key(&surface_client_id, &session_id).is_none(),
            "deleted key should not be loadable"
        );
    }
}
