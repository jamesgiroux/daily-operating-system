//! Keychain helpers for Gravatar API key storage (macOS).
//!
//! Follows the same `security` CLI pattern as `clay/oauth.rs`.

const SERVICE: &str = "com.dailyos.desktop.gravatar";
const ACCOUNT: &str = "gravatar-api-key-v1";

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

/// Retrieve Gravatar API key from keychain.
pub fn get_gravatar_api_key() -> Option<String> {
    let output =
        run_security_cmd(&["find-generic-password", "-a", ACCOUNT, "-s", SERVICE, "-w"]).ok()?;

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

/// Save Gravatar API key to keychain.
/// Uses `-U` flag to update if entry already exists.
pub fn save_gravatar_api_key(key: &str) -> Result<(), String> {
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
            "Failed to save Gravatar API key: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Delete Gravatar API key from keychain.
pub fn delete_gravatar_api_key() -> Result<(), String> {
    let output = run_security_cmd(&["delete-generic-password", "-a", ACCOUNT, "-s", SERVICE])?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("could not be found") || stderr.contains("item not found") {
        return Ok(());
    }

    Err(format!(
        "Failed to delete Gravatar API key: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// Migrate Gravatar API key from config.json to Keychain.
///
/// Called on startup. If the key exists in the legacy config field but not
/// in the Keychain, copies it over and clears the config field.
pub fn migrate_from_config(state: &crate::state::AppState) {
    let legacy_key = {
        let config = state.config.read().ok();
        config
            .as_ref()
            .and_then(|g| g.as_ref())
            .and_then(|c| c.gravatar.api_key.clone())
    };

    let Some(key) = legacy_key else {
        return;
    };

    // Only migrate if Keychain doesn't already have the key
    if get_gravatar_api_key().is_some() {
        log::info!("Gravatar API key already in Keychain, skipping migration");
        return;
    }

    match save_gravatar_api_key(&key) {
        Ok(()) => {
            log::info!("Migrated Gravatar API key from config.json to Keychain");
            // Clear the legacy field — skip_serializing prevents it from being
            // written back, but we clear it in memory for consistency.
            if let Ok(mut guard) = state.config.write() {
                if let Some(ref mut config) = *guard {
                    config.gravatar.api_key = None;
                }
            }
            // Force a config save so the plaintext key is removed from disk
            let _ = crate::state::create_or_update_config(state, |_| {});
        }
        Err(e) => {
            log::error!("Failed to migrate Gravatar API key to Keychain: {}", e);
        }
    }
}
