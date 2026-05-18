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

#[cfg(test)]
use std::sync::Arc;

use base64::Engine as _;

const SERVICE_PREFIX: &str = "com.dailyos.desktop.surface-session";
const KEY_BYTES: usize = 32;

fn service_name(surface_client_id: &str) -> String {
    format!("{SERVICE_PREFIX}.{surface_client_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionKeyLookup {
    Found([u8; KEY_BYTES]),
    NotFound,
    Unavailable { reason: String },
}

pub trait KeychainBackend: Send + Sync {
    fn find(&self, service: &str, account: &str) -> SessionKeyLookup;
    fn persist(&self, service: &str, account: &str, payload: &[u8]) -> Result<(), String>;
    fn delete(&self, service: &str, account: &str) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct RealKeychain;

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

fn is_keychain_item_not_found(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("could not be found")
        || stderr.contains("item not found")
        || stderr.contains("-25300")
}

fn unavailable(reason: impl Into<String>) -> SessionKeyLookup {
    SessionKeyLookup::Unavailable {
        reason: reason.into(),
    }
}

fn classify_find_output(output: std::process::Output) -> SessionKeyLookup {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if is_keychain_item_not_found(&stderr) {
            return SessionKeyLookup::NotFound;
        }
        return unavailable(format!("keychain_unavailable: {}", stderr.trim()));
    }

    let encoded = match String::from_utf8(output.stdout) {
        Ok(encoded) => encoded,
        Err(_) => return unavailable("corrupt_payload"),
    };
    let trimmed = encoded.trim();
    let bytes = match base64::engine::general_purpose::STANDARD.decode(trimmed) {
        Ok(bytes) => bytes,
        Err(_) => return unavailable("corrupt_payload"),
    };
    if bytes.len() != KEY_BYTES {
        return unavailable("length_mismatch");
    }
    let mut key = [0u8; KEY_BYTES];
    key.copy_from_slice(&bytes);
    SessionKeyLookup::Found(key)
}

impl KeychainBackend for RealKeychain {
    fn find(&self, service: &str, account: &str) -> SessionKeyLookup {
        match run_security_cmd(&["find-generic-password", "-a", account, "-s", service, "-w"]) {
            Ok(output) => classify_find_output(output),
            Err(_) => unavailable("spawn_failure"),
        }
    }

    fn persist(&self, service: &str, account: &str, payload: &[u8]) -> Result<(), String> {
        let output = run_security_cmd(&[
            "add-generic-password",
            "-a",
            account,
            "-s",
            service,
            "-w",
            std::str::from_utf8(payload)
                .map_err(|err| format!("keychain payload was not UTF-8: {err}"))?,
            "-U", // upsert (replace existing)
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("keychain persist failed: {stderr}"));
        }
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        let output = run_security_cmd(&["delete-generic-password", "-a", account, "-s", service])?;
        // `delete-generic-password` returns non-zero if the entry didn't exist;
        // treat that as success (idempotent delete).
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !is_keychain_item_not_found(&stderr) && !stderr.contains("SecKeychainSearchCopyNext")
            {
                return Err(format!("keychain delete failed: {stderr}"));
            }
        }
        Ok(())
    }
}

#[cfg(not(test))]
fn with_keychain_backend<R>(f: impl FnOnce(&dyn KeychainBackend) -> R) -> R {
    f(&RealKeychain)
}

#[cfg(test)]
thread_local! {
    static TEST_KEYCHAIN_BACKEND: std::cell::RefCell<Option<Arc<dyn KeychainBackend>>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
pub struct KeychainOverrideGuard {
    previous: Option<Arc<dyn KeychainBackend>>,
}

#[cfg(test)]
impl Drop for KeychainOverrideGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        TEST_KEYCHAIN_BACKEND.with(|cell| {
            cell.replace(previous);
        });
    }
}

#[cfg(test)]
fn with_keychain_backend<R>(f: impl FnOnce(&dyn KeychainBackend) -> R) -> R {
    TEST_KEYCHAIN_BACKEND.with(|cell| {
        if let Some(keychain) = cell.borrow().as_ref().cloned() {
            f(keychain.as_ref())
        } else {
            f(&RealKeychain)
        }
    })
}

#[cfg(test)]
pub fn with_keychain_for_tests<R>(keychain: Arc<dyn KeychainBackend>, f: impl FnOnce() -> R) -> R {
    let _guard = set_keychain_for_tests(keychain);
    f()
}

#[cfg(test)]
pub fn set_keychain_for_tests(keychain: Arc<dyn KeychainBackend>) -> KeychainOverrideGuard {
    let previous = TEST_KEYCHAIN_BACKEND.with(|cell| cell.replace(Some(keychain)));
    KeychainOverrideGuard { previous }
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
    with_keychain_backend(|keychain| keychain.persist(&service, session_id, encoded.as_bytes()))
}

/// Retrieve a previously-persisted session master key.
pub fn load_session_master_key(surface_client_id: &str, session_id: &str) -> SessionKeyLookup {
    let service = service_name(surface_client_id);
    with_keychain_backend(|keychain| keychain.find(&service, session_id))
}

/// Remove a session master key from keychain — called when a session is
/// revoked or when reconciliation discovers a stale DB row.
pub fn delete_session_master_key(surface_client_id: &str, session_id: &str) -> Result<(), String> {
    let service = service_name(surface_client_id);
    with_keychain_backend(|keychain| keychain.delete(&service, session_id))
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct MockKeychain {
    entries: std::sync::Mutex<std::collections::BTreeMap<(String, String), Vec<u8>>>,
    find_results_by_key:
        std::sync::Mutex<std::collections::BTreeMap<(String, String), SessionKeyLookup>>,
    find_results: std::sync::Mutex<std::collections::VecDeque<SessionKeyLookup>>,
    delete_results: std::sync::Mutex<std::collections::VecDeque<Result<(), String>>>,
    persist_results: std::sync::Mutex<std::collections::VecDeque<Result<(), String>>>,
    delete_delay: std::sync::Mutex<Option<std::time::Duration>>,
}

#[cfg(test)]
impl MockKeychain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_find_result(&self, result: SessionKeyLookup) {
        self.find_results.lock().unwrap().push_back(result);
    }

    pub fn set_session_lookup(
        &self,
        surface_client_id: &str,
        session_id: &str,
        result: SessionKeyLookup,
    ) {
        self.find_results_by_key.lock().unwrap().insert(
            (service_name(surface_client_id), session_id.to_string()),
            result,
        );
    }

    pub fn push_delete_result(&self, result: Result<(), String>) {
        self.delete_results.lock().unwrap().push_back(result);
    }

    pub fn push_persist_result(&self, result: Result<(), String>) {
        self.persist_results.lock().unwrap().push_back(result);
    }

    pub fn set_delete_delay(&self, delay: std::time::Duration) {
        *self.delete_delay.lock().unwrap() = Some(delay);
    }
}

#[cfg(test)]
impl KeychainBackend for MockKeychain {
    fn find(&self, service: &str, account: &str) -> SessionKeyLookup {
        let key = (service.to_string(), account.to_string());
        if let Some(result) = self.find_results_by_key.lock().unwrap().get(&key).cloned() {
            return result;
        }
        if let Some(result) = self.find_results.lock().unwrap().pop_front() {
            return result;
        }
        self.entries
            .lock()
            .unwrap()
            .get(&key)
            .and_then(|payload| {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(payload)
                    .ok()?;
                let key: [u8; KEY_BYTES] = bytes.try_into().ok()?;
                Some(SessionKeyLookup::Found(key))
            })
            .unwrap_or(SessionKeyLookup::NotFound)
    }

    fn persist(&self, service: &str, account: &str, payload: &[u8]) -> Result<(), String> {
        if let Some(result) = self.persist_results.lock().unwrap().pop_front() {
            result?;
        }
        self.entries
            .lock()
            .unwrap()
            .insert((service.to_string(), account.to_string()), payload.to_vec());
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        if let Some(delay) = *self.delete_delay.lock().unwrap() {
            std::thread::sleep(delay);
        }
        if let Some(result) = self.delete_results.lock().unwrap().pop_front() {
            result?;
        }
        self.entries
            .lock()
            .unwrap()
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    // Note: these tests require macOS Keychain access and will be skipped on
    // CI / non-darwin platforms. Full integration testing for code-signing
    // isolation lives in tests/.

    fn output(
        status: i32,
        stdout: impl Into<Vec<u8>>,
        stderr: impl Into<Vec<u8>>,
    ) -> std::process::Output {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(status << 8),
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn roundtrip_persists_and_loads_master_key() {
        let surface_client_id = format!("dos655-test-{}", std::process::id());
        let session_id = format!("session-{}", std::process::id());
        let master_key = [42u8; KEY_BYTES];

        persist_session_master_key(&surface_client_id, &session_id, &master_key)
            .expect("persist should succeed");
        let SessionKeyLookup::Found(loaded) =
            load_session_master_key(&surface_client_id, &session_id)
        else {
            panic!("load should return the persisted key");
        };
        assert_eq!(loaded, master_key);

        delete_session_master_key(&surface_client_id, &session_id).expect("delete should succeed");
        assert!(
            matches!(
                load_session_master_key(&surface_client_id, &session_id),
                SessionKeyLookup::NotFound
            ),
            "deleted key should not be loadable"
        );
    }

    #[test]
    fn dos673_lookup_classifies_found() {
        let key = [42u8; KEY_BYTES];
        let encoded = base64::engine::general_purpose::STANDARD.encode(key);

        assert_eq!(
            classify_find_output(output(0, encoded, Vec::new())),
            SessionKeyLookup::Found(key)
        );
    }

    #[test]
    fn dos673_lookup_classifies_not_found() {
        for stderr in [
            "The specified item could not be found in the keychain.",
            "security: SecKeychainSearchCopyNext: item not found.",
            "security: SecKeychainSearchCopyNext: -25300",
        ] {
            assert_eq!(
                classify_find_output(output(44, Vec::new(), stderr)),
                SessionKeyLookup::NotFound
            );
        }
    }

    #[test]
    fn dos673_lookup_classifies_unavailable_spawn_failure() {
        let keychain = Arc::new(MockKeychain::new());
        keychain.push_find_result(SessionKeyLookup::Unavailable {
            reason: "spawn_failure".to_string(),
        });

        with_keychain_for_tests(keychain, || {
            assert_eq!(
                load_session_master_key("surface_test", "session_test"),
                SessionKeyLookup::Unavailable {
                    reason: "spawn_failure".to_string()
                }
            );
        });
    }

    #[test]
    fn dos673_lookup_classifies_unavailable_locked() {
        let result = classify_find_output(output(
            1,
            Vec::new(),
            "security: SecKeychainSearchCopyNext: User interaction is not allowed.",
        ));
        assert!(matches!(result, SessionKeyLookup::Unavailable { .. }));
    }

    #[test]
    fn dos673_lookup_classifies_unavailable_corrupt_base64() {
        assert_eq!(
            classify_find_output(output(0, "not-base64!?", Vec::new())),
            SessionKeyLookup::Unavailable {
                reason: "corrupt_payload".to_string()
            }
        );
    }

    #[test]
    fn dos673_lookup_classifies_unavailable_wrong_length() {
        let encoded = base64::engine::general_purpose::STANDARD.encode([7u8; KEY_BYTES - 1]);
        assert_eq!(
            classify_find_output(output(0, encoded, Vec::new())),
            SessionKeyLookup::Unavailable {
                reason: "length_mismatch".to_string()
            }
        );
    }

    #[test]
    fn dos673_keychain_backend_trait_seam() {
        let keychain = Arc::new(MockKeychain::new());
        let key = [9u8; KEY_BYTES];
        with_keychain_for_tests(keychain, || {
            persist_session_master_key("surface_test", "session_test", &key)
                .expect("mock persist should succeed");
            assert_eq!(
                load_session_master_key("surface_test", "session_test"),
                SessionKeyLookup::Found(key)
            );
            delete_session_master_key("surface_test", "session_test")
                .expect("mock delete should succeed");
            assert_eq!(
                load_session_master_key("surface_test", "session_test"),
                SessionKeyLookup::NotFound
            );
        });
    }
}
