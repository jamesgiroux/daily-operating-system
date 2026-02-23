//! Keychain helpers for Smithery API key storage (macOS).
//!
//! Follows the same `security` CLI pattern as `google_api/token_store.rs`.

const SERVICE: &str = "com.dailyos.desktop.smithery";
const ACCOUNT: &str = "smithery-api-key-v1";

// Legacy constants for cleanup
const LEGACY_SERVICE: &str = "com.dailyos.desktop.clay-auth";
const LEGACY_ACCOUNT: &str = "clay-oauth-token-v1";

/// Run a `security` CLI command with retry + backoff for transient Keychain
/// contention (macOS errno 35 / EAGAIN).
fn run_security_cmd(args: &[&str]) -> Result<std::process::Output, String> {
    const MAX_RETRIES: u32 = 4;
    const BASE_MS: u64 = 150;

    for attempt in 0..=MAX_RETRIES {
        let output = std::process::Command::new("security")
            .args(args)
            .output()
            .map_err(|err| format!("security command failed: {}", err))?;

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
        log::warn!(
            "Keychain busy (attempt {}/{}), retrying in {}ms",
            attempt + 1,
            MAX_RETRIES,
            delay.as_millis()
        );
        std::thread::sleep(delay);
    }

    unreachable!()
}

/// Retrieve Smithery API key from keychain.
pub fn get_smithery_api_key() -> Option<String> {
    let output = run_security_cmd(&[
        "find-generic-password",
        "-a",
        ACCOUNT,
        "-s",
        SERVICE,
        "-w",
    ])
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// Save Smithery API key to keychain.
/// Uses `-U` flag to update if entry already exists.
pub fn save_smithery_api_key(key: &str) -> Result<(), String> {
    let output = run_security_cmd(&[
        "add-generic-password",
        "-a",
        ACCOUNT,
        "-s",
        SERVICE,
        "-w",
        key,
        "-U",
    ])?;

    if !output.status.success() {
        return Err(format!(
            "Failed to save Smithery API key: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Delete Smithery API key from keychain.
pub fn delete_smithery_api_key() -> Result<(), String> {
    let output = run_security_cmd(&[
        "delete-generic-password",
        "-a",
        ACCOUNT,
        "-s",
        SERVICE,
    ])?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("could not be found") || stderr.contains("item not found") {
        return Ok(());
    }

    Err(format!(
        "Failed to delete Smithery API key: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Clean up the legacy Clay OAuth keychain entry if it exists.
/// Called once during migration to Smithery transport.
pub fn cleanup_legacy_clay_token() {
    let _ = run_security_cmd(&[
        "delete-generic-password",
        "-a",
        LEGACY_ACCOUNT,
        "-s",
        LEGACY_SERVICE,
    ]);
}
