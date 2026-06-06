//! Local MCP runtime anchors that must not be caller-asserted over stdio.
//!
//! W2 keeps local stdio auth right-sized: the server owns the local client id,
//! read audit digests use a server-held key, and conversation continuity is
//! persisted outside the SQLCipher auth tables.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use hmac::{Hmac, Mac};
use parking_lot::Mutex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::contracts::{McpClientId, OpaqueConversationHandle};

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.mcp-local";
const CLIENT_ID_ACCOUNT: &str = "local-stdio-client-id-v1";
const AUDIT_DIGEST_KEY_ACCOUNT: &str = "read-audit-digest-key-v1";
const DIGEST_KEY_BYTES: usize = 32;
const CONVERSATION_STORE_RELATIVE_PATH: &str = "mcp/conversation-handles.json";
const DEFAULT_CONVERSATION_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const SECURITY_CMD_TIMEOUT: Duration = Duration::from_secs(5);
const SECURITY_CMD_PATH: &str = "/usr/bin/security";
const CONVERSATION_FILE_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const CONVERSATION_STALE_LOCK_AFTER: Duration = Duration::from_secs(30);
pub const AUDIT_DIGEST_ALGORITHM: &str = "hmac_sha256_canonical_json_v1";

#[derive(Debug, thiserror::Error)]
pub enum LocalRuntimeError {
    #[error("keychain item unavailable: {0}")]
    Keychain(String),
    #[error("keychain item payload is corrupt: {0}")]
    CorruptPayload(String),
    #[error("conversation store I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("conversation store serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("conversation handle has been revoked or expired")]
    ConversationRevoked,
    #[error("conversation handle is malformed")]
    BadConversationHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeychainLookup {
    Found(String),
    NotFound,
    Unavailable(String),
}

trait KeychainBackend: Send + Sync {
    fn find(&self, service: &str, account: &str) -> KeychainLookup;
    fn create(&self, service: &str, account: &str, payload: &str) -> Result<(), String>;
    fn upsert(&self, service: &str, account: &str, payload: &str) -> Result<(), String>;
}

#[derive(Debug, Default)]
struct SecurityCliKeychain;

impl KeychainBackend for SecurityCliKeychain {
    fn find(&self, service: &str, account: &str) -> KeychainLookup {
        match run_security_cmd(&["find-generic-password", "-a", account, "-s", service, "-w"]) {
            Ok(output) if output.status.success() => match String::from_utf8(output.stdout) {
                Ok(payload) => KeychainLookup::Found(payload.trim().to_string()),
                Err(error) => KeychainLookup::Unavailable(format!("non_utf8_payload: {error}")),
            },
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if is_keychain_item_not_found(&stderr) {
                    KeychainLookup::NotFound
                } else {
                    KeychainLookup::Unavailable(stderr.trim().to_string())
                }
            }
            Err(error) => KeychainLookup::Unavailable(error),
        }
    }

    fn create(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
        let output = run_security_cmd(&[
            "add-generic-password",
            "-a",
            account,
            "-s",
            service,
            "-w",
            payload,
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(stderr.trim().to_string());
        }
        Ok(())
    }

    fn upsert(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
        let output = run_security_cmd(&[
            "add-generic-password",
            "-a",
            account,
            "-s",
            service,
            "-w",
            payload,
            "-U",
        ])?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(stderr.trim().to_string());
        }
        Ok(())
    }
}

fn run_security_cmd(args: &[&str]) -> Result<Output, String> {
    const MAX_RETRIES: u32 = 4;
    const BASE_MS: u64 = 150;

    for attempt in 0..=MAX_RETRIES {
        let output = run_command_with_timeout(SECURITY_CMD_PATH, args, SECURITY_CMD_TIMEOUT)?;

        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let transient = stderr.contains("temporarily unavailable")
            || stderr.contains("os error 35")
            || stderr.contains("EAGAIN");
        if !transient || attempt == MAX_RETRIES {
            return Ok(output);
        }
        std::thread::sleep(Duration::from_millis(BASE_MS * 2u64.pow(attempt)));
    }

    unreachable!()
}

fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("{program} command failed: {err}"))?;
    let deadline = Instant::now() + timeout;

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                return child
                    .wait_with_output()
                    .map_err(|err| format!("{program} command output failed: {err}"));
            }
            Ok(None) => {}
            Err(error) => {
                terminate_child(&mut child);
                return Err(format!("{program} command wait failed: {error}"));
            }
        }

        if Instant::now() >= deadline {
            terminate_child(&mut child);
            return Err(format!(
                "{program} command timed out after {}ms",
                timeout.as_millis()
            ));
        }

        std::thread::sleep(Duration::from_millis(25));
    }
}

fn terminate_child(child: &mut std::process::Child) {
    drop(child.kill());
    drop(child.wait());
}

fn is_keychain_item_not_found(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("could not be found")
        || stderr.contains("item not found")
        || stderr.contains("-25300")
}

fn is_keychain_duplicate_item(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("already exists") || stderr.contains("duplicate") || stderr.contains("-25299")
}

#[cfg(not(test))]
fn with_keychain_backend<R>(f: impl FnOnce(&dyn KeychainBackend) -> R) -> R {
    f(&SecurityCliKeychain)
}

#[cfg(test)]
thread_local! {
    static TEST_KEYCHAIN_BACKEND: std::cell::RefCell<Option<Arc<dyn KeychainBackend>>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
fn with_keychain_backend<R>(f: impl FnOnce(&dyn KeychainBackend) -> R) -> R {
    TEST_KEYCHAIN_BACKEND.with(|cell| {
        if let Some(keychain) = cell.borrow().as_ref().cloned() {
            f(keychain.as_ref())
        } else {
            f(&SecurityCliKeychain)
        }
    })
}

#[cfg(test)]
#[derive(Default)]
struct TestKeychain {
    entries: std::sync::Mutex<BTreeMap<(String, String), String>>,
}

#[cfg(test)]
impl KeychainBackend for TestKeychain {
    fn find(&self, service: &str, account: &str) -> KeychainLookup {
        self.entries
            .lock()
            .expect("entries lock")
            .get(&(service.to_string(), account.to_string()))
            .cloned()
            .map(KeychainLookup::Found)
            .unwrap_or(KeychainLookup::NotFound)
    }

    fn create(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
        let mut entries = self.entries.lock().expect("entries lock");
        let key = (service.to_string(), account.to_string());
        if entries.contains_key(&key) {
            return Err("-25299 duplicate item".to_string());
        }
        entries.insert(key, payload.to_string());
        Ok(())
    }

    fn upsert(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
        self.entries.lock().expect("entries lock").insert(
            (service.to_string(), account.to_string()),
            payload.to_string(),
        );
        Ok(())
    }
}

#[cfg(test)]
fn with_test_keychain<R>(keychain: Arc<dyn KeychainBackend>, f: impl FnOnce() -> R) -> R {
    TEST_KEYCHAIN_BACKEND.with(|cell| {
        let previous = cell.replace(Some(keychain));
        let output = f();
        cell.replace(previous);
        output
    })
}

#[cfg(test)]
pub(crate) fn with_audit_digest_key_for_tests<R>(
    key: [u8; DIGEST_KEY_BYTES],
    f: impl FnOnce() -> R,
) -> R {
    with_audit_digest_key_payload_for_tests(
        &base64::engine::general_purpose::STANDARD.encode(key),
        f,
    )
}

#[cfg(test)]
pub(crate) fn with_audit_digest_key_payload_for_tests<R>(
    payload: &str,
    f: impl FnOnce() -> R,
) -> R {
    let keychain = Arc::new(TestKeychain::default());
    keychain
        .upsert(KEYCHAIN_SERVICE, AUDIT_DIGEST_KEY_ACCOUNT, payload)
        .expect("seed audit digest key");
    with_test_keychain(keychain, f)
}

/// Return the process-local MCP client id. The caller cannot set or override
/// this over stdio; the first launch creates and stores the opaque id.
pub fn get_or_create_local_client_id() -> Result<McpClientId, LocalRuntimeError> {
    let id = load_or_seed_keychain_string(CLIENT_ID_ACCOUNT, generate_local_client_id)?;
    Ok(McpClientId::new(id))
}

/// HMAC-SHA256 over a deterministic JSON encoding. The HMAC key lives in
/// Keychain so read-audit rows can prove stable equality without storing raw
/// read params or responses.
pub fn digest_json_value_hex(value: &serde_json::Value) -> Result<String, LocalRuntimeError> {
    let encoded = load_or_seed_keychain_string(AUDIT_DIGEST_KEY_ACCOUNT, generate_digest_key)?;
    let key = decode_digest_key(&encoded)?;
    hmac_sha256_json_hex_for_key(&key, value).map_err(LocalRuntimeError::Serialization)
}

pub fn hmac_sha256_json_hex_for_key(
    key: &[u8; DIGEST_KEY_BYTES],
    value: &serde_json::Value,
) -> serde_json::Result<String> {
    let canonical = canonical_json_bytes(value)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts 32-byte keys");
    mac.update(&canonical);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn load_or_seed_keychain_string(
    account: &str,
    generate: impl FnOnce() -> String,
) -> Result<String, LocalRuntimeError> {
    with_keychain_backend(|keychain| load_or_seed_keychain_string_from(keychain, account, generate))
}

fn load_or_seed_keychain_string_from(
    keychain: &dyn KeychainBackend,
    account: &str,
    generate: impl FnOnce() -> String,
) -> Result<String, LocalRuntimeError> {
    match keychain.find(KEYCHAIN_SERVICE, account) {
        KeychainLookup::Found(payload) if !payload.trim().is_empty() => Ok(payload),
        KeychainLookup::Found(_) => {
            let generated = generate();
            keychain
                .upsert(KEYCHAIN_SERVICE, account, &generated)
                .map_err(LocalRuntimeError::Keychain)?;
            Ok(generated)
        }
        KeychainLookup::NotFound => {
            let generated = generate();
            match keychain.create(KEYCHAIN_SERVICE, account, &generated) {
                Ok(()) => Ok(generated),
                Err(error) if is_keychain_duplicate_item(&error) => {
                    read_seeded_keychain_value(keychain, account)
                }
                Err(error) => Err(LocalRuntimeError::Keychain(error)),
            }
        }
        KeychainLookup::Unavailable(reason) => Err(LocalRuntimeError::Keychain(reason)),
    }
}

fn read_seeded_keychain_value(
    keychain: &dyn KeychainBackend,
    account: &str,
) -> Result<String, LocalRuntimeError> {
    match keychain.find(KEYCHAIN_SERVICE, account) {
        KeychainLookup::Found(payload) if !payload.trim().is_empty() => Ok(payload),
        KeychainLookup::Found(_) | KeychainLookup::NotFound => Err(LocalRuntimeError::Keychain(
            "keychain item raced but no seeded value was readable".to_string(),
        )),
        KeychainLookup::Unavailable(reason) => Err(LocalRuntimeError::Keychain(reason)),
    }
}

fn generate_local_client_id() -> String {
    let mut bytes = [0_u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    format!("dailyos-local-{}", hex::encode(bytes))
}

fn generate_digest_key() -> String {
    let mut key = [0_u8; DIGEST_KEY_BYTES];
    rand::rng().fill_bytes(&mut key);
    base64::engine::general_purpose::STANDARD.encode(key)
}

fn decode_digest_key(encoded: &str) -> Result<[u8; DIGEST_KEY_BYTES], LocalRuntimeError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|error| LocalRuntimeError::CorruptPayload(error.to_string()))?;
    if bytes.len() != DIGEST_KEY_BYTES {
        return Err(LocalRuntimeError::CorruptPayload(format!(
            "expected {DIGEST_KEY_BYTES} bytes, got {}",
            bytes.len()
        )));
    }
    let mut key = [0_u8; DIGEST_KEY_BYTES];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn canonical_json_bytes(value: &serde_json::Value) -> serde_json::Result<Vec<u8>> {
    serde_json::to_vec(&canonicalize_json(value))
}

fn canonicalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted = map
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize_json(value)))
                .collect::<BTreeMap<_, _>>();
            serde_json::to_value(sorted).expect("BTreeMap JSON serialization is infallible")
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonicalize_json).collect())
        }
        other => other.clone(),
    }
}

#[derive(Clone, Debug)]
pub struct LocalConversationStore {
    path: Arc<PathBuf>,
    lock: Arc<Mutex<()>>,
    ttl: Duration,
}

impl Default for LocalConversationStore {
    fn default() -> Self {
        Self::new(crate::state::mode_scoped_state_path(
            CONVERSATION_STORE_RELATIVE_PATH,
        ))
    }
}

impl LocalConversationStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Arc::new(path.into()),
            lock: Arc::new(Mutex::new(())),
            ttl: DEFAULT_CONVERSATION_TTL,
        }
    }

    pub fn resolve_or_mint(
        &self,
        client_id: &McpClientId,
        presented: Option<&OpaqueConversationHandle>,
    ) -> Result<OpaqueConversationHandle, LocalRuntimeError> {
        let _guard = self.lock.lock();
        let _file_guard = ConversationFileLock::acquire(&self.path)?;
        let now = unix_millis();
        let mut state = read_conversation_state(&self.path)?;

        let (resolved, mut write_required) = if let Some(handle) = presented {
            validate_handle(handle)?;
            match state.handles.get_mut(handle.as_str()) {
                Some(record)
                    if record.revoked_at_ms.is_some() || record.client_id != client_id.as_str() =>
                {
                    return Err(LocalRuntimeError::ConversationRevoked);
                }
                Some(record) if record.expires_at_ms > now => {
                    record.last_seen_at_ms = now;
                    record.expires_at_ms = now + self.ttl.as_millis() as i64;
                    (handle.clone(), true)
                }
                Some(_) | None => (
                    insert_conversation_handle(&mut state, client_id, now, self.ttl),
                    true,
                ),
            }
        } else {
            (
                insert_conversation_handle(&mut state, client_id, now, self.ttl),
                true,
            )
        };

        if prune_expired(&mut state, now) {
            write_required = true;
        }
        if write_required {
            write_conversation_state(&self.path, &state)?;
        }

        Ok(resolved)
    }
}

fn insert_conversation_handle(
    state: &mut ConversationState,
    client_id: &McpClientId,
    now: i64,
    ttl: Duration,
) -> OpaqueConversationHandle {
    let handle = mint_handle();
    state.handles.insert(
        handle.as_str().to_string(),
        ConversationRecord {
            client_id: client_id.as_str().to_string(),
            issued_at_ms: now,
            last_seen_at_ms: now,
            expires_at_ms: now + ttl.as_millis() as i64,
            revoked_at_ms: None,
        },
    );
    handle
}

struct ConversationFileLock {
    path: PathBuf,
    _file: File,
}

impl ConversationFileLock {
    fn acquire(target_path: &Path) -> std::io::Result<Self> {
        let lock_path = target_path.with_extension("lock");
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let deadline = Instant::now() + CONVERSATION_FILE_LOCK_TIMEOUT;

        loop {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    writeln!(
                        &mut file,
                        "pid={} acquired_at_ms={}",
                        std::process::id(),
                        unix_millis()
                    )?;
                    return Ok(Self {
                        path: lock_path,
                        _file: file,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if stale_lock_file(&lock_path) {
                        match std::fs::remove_file(&lock_path) {
                            Ok(()) => continue,
                            Err(remove_error)
                                if remove_error.kind() == std::io::ErrorKind::NotFound =>
                            {
                                continue;
                            }
                            Err(remove_error) => return Err(remove_error),
                        }
                    }
                    if Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "timed out waiting for conversation store lock",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for ConversationFileLock {
    fn drop(&mut self) {
        drop(std::fs::remove_file(&self.path));
    }
}

fn stale_lock_file(path: &Path) -> bool {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > CONVERSATION_STALE_LOCK_AFTER)
}

#[cfg(test)]
fn write_conversation_record_for_tests(
    path: &Path,
    handle: &OpaqueConversationHandle,
    record: ConversationRecord,
) {
    let mut state = read_conversation_state(path).expect("read conversation state");
    state.handles.insert(handle.as_str().to_string(), record);
    write_conversation_state(path, &state).expect("write conversation state");
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationState {
    version: u32,
    handles: BTreeMap<String, ConversationRecord>,
}

impl Default for ConversationState {
    fn default() -> Self {
        Self {
            version: 1,
            handles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationRecord {
    client_id: String,
    issued_at_ms: i64,
    last_seen_at_ms: i64,
    expires_at_ms: i64,
    revoked_at_ms: Option<i64>,
}

fn read_conversation_state(path: &Path) -> Result<ConversationState, LocalRuntimeError> {
    if !path.exists() {
        return Ok(ConversationState::default());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.trim().is_empty() {
        return Ok(ConversationState::default());
    }
    serde_json::from_str(&raw).map_err(LocalRuntimeError::Serialization)
}

fn write_conversation_state(
    path: &Path,
    state: &ConversationState,
) -> Result<(), LocalRuntimeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(state)?;
    crate::util::atomic_write_str(path, &raw)?;
    Ok(())
}

fn prune_expired(state: &mut ConversationState, now: i64) -> bool {
    let before = state.handles.len();
    state
        .handles
        .retain(|_, record| record.revoked_at_ms.is_some() || record.expires_at_ms > now);
    state.handles.len() != before
}

fn validate_handle(handle: &OpaqueConversationHandle) -> Result<(), LocalRuntimeError> {
    let value = handle.as_str();
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(LocalRuntimeError::BadConversationHandle);
    }
    Ok(())
}

fn mint_handle() -> OpaqueConversationHandle {
    let mut bytes = [0_u8; 18];
    rand::rng().fill_bytes(&mut bytes);
    OpaqueConversationHandle::new(format!("local-stdio-{}", hex::encode(bytes)))
}

fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RacingCreateKeychain {
        entries: std::sync::Mutex<BTreeMap<(String, String), String>>,
        first_missing_barrier: std::sync::Barrier,
    }

    impl RacingCreateKeychain {
        fn new(participants: usize) -> Self {
            Self {
                entries: std::sync::Mutex::new(BTreeMap::new()),
                first_missing_barrier: std::sync::Barrier::new(participants),
            }
        }
    }

    impl KeychainBackend for RacingCreateKeychain {
        fn find(&self, service: &str, account: &str) -> KeychainLookup {
            if let Some(value) = self
                .entries
                .lock()
                .expect("entries lock")
                .get(&(service.to_string(), account.to_string()))
                .cloned()
            {
                return KeychainLookup::Found(value);
            }
            self.first_missing_barrier.wait();
            KeychainLookup::NotFound
        }

        fn create(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
            let mut entries = self.entries.lock().expect("entries lock");
            let key = (service.to_string(), account.to_string());
            if entries.contains_key(&key) {
                return Err("-25299 duplicate item".to_string());
            }
            entries.insert(key, payload.to_string());
            Ok(())
        }

        fn upsert(&self, service: &str, account: &str, payload: &str) -> Result<(), String> {
            self.entries.lock().expect("entries lock").insert(
                (service.to_string(), account.to_string()),
                payload.to_string(),
            );
            Ok(())
        }
    }

    #[test]
    fn security_cli_keychain_uses_absolute_system_binary() {
        let security_path = Path::new(SECURITY_CMD_PATH);

        assert_eq!(security_path, Path::new("/usr/bin/security"));
        assert!(security_path.is_absolute());
    }

    #[test]
    fn local_client_id_is_server_generated_and_persisted() {
        with_test_keychain(Arc::new(TestKeychain::default()), || {
            let first = get_or_create_local_client_id().expect("first id");
            let second = get_or_create_local_client_id().expect("second id");

            assert!(first.as_str().starts_with("dailyos-local-"));
            assert_eq!(first, second);
        });
    }

    #[test]
    fn concurrent_first_seed_returns_single_keychain_value() {
        const THREADS: usize = 8;

        let keychain = Arc::new(RacingCreateKeychain::new(THREADS));
        let mut workers = Vec::new();
        for idx in 0..THREADS {
            let keychain = Arc::clone(&keychain);
            workers.push(std::thread::spawn(move || {
                load_or_seed_keychain_string_from(keychain.as_ref(), CLIENT_ID_ACCOUNT, || {
                    format!("client-{idx}")
                })
                .expect("seeded keychain value")
            }));
        }

        let values = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker"))
            .collect::<Vec<_>>();
        let first = values.first().expect("at least one value");

        assert!(values.iter().all(|value| value == first));
    }

    #[test]
    fn command_timeout_kills_hung_child() {
        let err = run_command_with_timeout("sh", &["-c", "sleep 1"], Duration::from_millis(50))
            .expect_err("sleeping child should time out");

        assert!(err.contains("timed out"));
    }

    #[test]
    fn command_timeout_helper_returns_output() {
        let output =
            run_command_with_timeout("sh", &["-c", "printf dailyos"], Duration::from_secs(1))
                .expect("command output");

        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "dailyos");
    }

    #[test]
    fn digest_is_stable_across_object_key_order() {
        let key = [7_u8; DIGEST_KEY_BYTES];
        let left = serde_json::json!({ "b": 2, "a": { "z": true, "c": 3 } });
        let right = serde_json::json!({ "a": { "c": 3, "z": true }, "b": 2 });

        let left_digest = hmac_sha256_json_hex_for_key(&key, &left).expect("left digest");
        let right_digest = hmac_sha256_json_hex_for_key(&key, &right).expect("right digest");

        assert_eq!(left_digest, right_digest);
        assert_eq!(left_digest.len(), 64);
    }

    #[test]
    fn conversation_store_mints_and_reuses_presented_handle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LocalConversationStore::new(dir.path().join("handles.json"));
        let client = McpClientId::new("client-a");

        let first = store.resolve_or_mint(&client, None).expect("minted handle");
        let reused = store
            .resolve_or_mint(&client, Some(&first))
            .expect("reused handle");

        assert_eq!(first, reused);
        assert!(dir.path().join("handles.json").exists());
    }

    #[test]
    fn conversation_store_expired_handle_transparently_mints_replacement() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("handles.json");
        let store = LocalConversationStore::new(&path);
        let client = McpClientId::new("client-a");
        let old_handle = store.resolve_or_mint(&client, None).expect("minted handle");
        let now = unix_millis();
        write_conversation_record_for_tests(
            &path,
            &old_handle,
            ConversationRecord {
                client_id: client.as_str().to_string(),
                issued_at_ms: now - 100_000,
                last_seen_at_ms: now - 100_000,
                expires_at_ms: now - 1,
                revoked_at_ms: None,
            },
        );

        let replacement = store
            .resolve_or_mint(&client, Some(&old_handle))
            .expect("expired handle remints");
        let state = read_conversation_state(&path).expect("state");

        assert_ne!(replacement, old_handle);
        assert!(!state.handles.contains_key(old_handle.as_str()));
        assert!(state.handles.contains_key(replacement.as_str()));
    }

    #[test]
    fn conversation_store_revoked_handle_stays_revoked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("handles.json");
        let store = LocalConversationStore::new(&path);
        let client = McpClientId::new("client-a");
        let handle = store.resolve_or_mint(&client, None).expect("minted handle");
        let now = unix_millis();
        write_conversation_record_for_tests(
            &path,
            &handle,
            ConversationRecord {
                client_id: client.as_str().to_string(),
                issued_at_ms: now - 100_000,
                last_seen_at_ms: now - 100_000,
                expires_at_ms: now - 1,
                revoked_at_ms: Some(now - 10),
            },
        );

        let err = store
            .resolve_or_mint(&client, Some(&handle))
            .expect_err("revoked handle fails");
        let state = read_conversation_state(&path).expect("state");

        assert!(matches!(err, LocalRuntimeError::ConversationRevoked));
        assert!(state.handles.contains_key(handle.as_str()));
    }

    #[test]
    fn conversation_store_preserves_concurrent_independent_mints() {
        const THREADS: usize = 8;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = Arc::new(dir.path().join("handles.json"));
        let barrier = Arc::new(std::sync::Barrier::new(THREADS));
        let mut workers = Vec::new();

        for idx in 0..THREADS {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                let store = LocalConversationStore::new(path.as_ref().clone());
                let client = McpClientId::new(format!("client-{idx}"));
                barrier.wait();
                store.resolve_or_mint(&client, None).expect("mint handle")
            }));
        }

        let handles = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker"))
            .collect::<Vec<_>>();
        let state = read_conversation_state(&path).expect("state");

        assert_eq!(state.handles.len(), THREADS);
        for handle in handles {
            assert!(state.handles.contains_key(handle.as_str()));
        }
    }

    #[test]
    fn conversation_store_rejects_cross_client_reuse() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LocalConversationStore::new(dir.path().join("handles.json"));
        let handle = store
            .resolve_or_mint(&McpClientId::new("client-a"), None)
            .expect("minted handle");

        let err = store
            .resolve_or_mint(&McpClientId::new("client-b"), Some(&handle))
            .expect_err("cross-client reuse rejected");

        assert!(matches!(err, LocalRuntimeError::ConversationRevoked));
    }

    #[test]
    fn conversation_store_rejects_malformed_handle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = LocalConversationStore::new(dir.path().join("handles.json"));
        let err = store
            .resolve_or_mint(
                &McpClientId::new("client-a"),
                Some(&OpaqueConversationHandle::new("../bad")),
            )
            .expect_err("bad handle rejected");

        assert!(matches!(err, LocalRuntimeError::BadConversationHandle));
    }
}
