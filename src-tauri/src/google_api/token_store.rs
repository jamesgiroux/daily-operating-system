//! OAuth token storage abstraction.
//!
//! - macOS: Keychain is canonical, with one-time migration from legacy token.json.
//! - non-macOS: token.json file backend is canonical.

use super::{GoogleApiError, GoogleToken};

/// Load the current Google OAuth token.
pub fn load_token() -> Result<GoogleToken, GoogleApiError> {
    #[cfg(target_os = "macos")]
    {
        load_token_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        load_token_file()
    }
}

/// Persist a Google OAuth token.
pub fn save_token(token: &GoogleToken) -> Result<(), GoogleApiError> {
    #[cfg(target_os = "macos")]
    {
        save_token_macos(token)
    }

    #[cfg(not(target_os = "macos"))]
    {
        save_token_file(token)
    }
}

/// Remove Google OAuth credentials from local storage.
pub fn delete_token() -> Result<(), GoogleApiError> {
    #[cfg(target_os = "macos")]
    {
        delete_token_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        delete_token_file()
    }
}

/// Probe for an authenticated account email without propagating errors.
pub fn peek_account_email() -> Option<String> {
    match load_token() {
        Ok(token) => Some(
            token
                .account
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "connected".to_string()),
        ),
        Err(_) => None,
    }
}

fn load_token_file() -> Result<GoogleToken, GoogleApiError> {
    let path = super::token_path();
    if !path.exists() {
        return Err(GoogleApiError::TokenNotFound(path));
    }
    let content = std::fs::read_to_string(&path)?;
    let token: GoogleToken = serde_json::from_str(&content)?;
    Ok(token)
}

fn save_token_file(token: &GoogleToken) -> Result<(), GoogleApiError> {
    let path = super::token_path();

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
            }
        }
    }

    let content = serde_json::to_string_pretty(token)?;
    crate::util::atomic_write_str(&path, &content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

fn delete_token_file() -> Result<(), GoogleApiError> {
    let path = super::token_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.google-auth";
#[cfg(target_os = "macos")]
const KEYCHAIN_ACCOUNT: &str = "oauth-token-v1";

#[cfg(target_os = "macos")]
fn load_token_from_keychain() -> Result<GoogleToken, GoogleApiError> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
            "-w",
        ])
        .output()
        .map_err(|err| GoogleApiError::Keychain(err.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        if stderr.contains("could not be found") || stderr.contains("item not found") {
            return Err(GoogleApiError::TokenNotFound(super::token_path()));
        }
        return Err(GoogleApiError::Keychain(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let token = serde_json::from_str::<GoogleToken>(&raw)?;
    Ok(token)
}

#[cfg(target_os = "macos")]
fn save_token_to_keychain(token: &GoogleToken) -> Result<(), GoogleApiError> {
    let payload = serde_json::to_string(token)?;
    let output = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
            "-w",
            &payload,
            "-U",
        ])
        .output()
        .map_err(|err| GoogleApiError::Keychain(err.to_string()))?;
    if !output.status.success() {
        return Err(GoogleApiError::Keychain(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn delete_token_from_keychain() -> Result<(), GoogleApiError> {
    let output = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-a",
            KEYCHAIN_ACCOUNT,
            "-s",
            KEYCHAIN_SERVICE,
        ])
        .output()
        .map_err(|err| GoogleApiError::Keychain(err.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("could not be found") || stderr.contains("item not found") {
        return Ok(());
    }
    Err(GoogleApiError::Keychain(
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn load_token_macos() -> Result<GoogleToken, GoogleApiError> {
    match load_token_from_keychain() {
        Ok(token) => return Ok(token),
        Err(GoogleApiError::TokenNotFound(_)) => {}
        Err(err) => return Err(err),
    }

    // One-time migration from legacy plaintext token file.
    let token = load_token_file()?;
    save_token_to_keychain(&token)?;
    let _ = delete_token_file();
    Ok(token)
}

#[cfg(target_os = "macos")]
fn save_token_macos(token: &GoogleToken) -> Result<(), GoogleApiError> {
    save_token_to_keychain(token)?;
    let _ = delete_token_file();
    Ok(())
}

#[cfg(target_os = "macos")]
fn delete_token_macos() -> Result<(), GoogleApiError> {
    delete_token_from_keychain()?;
    let _ = delete_token_file();
    Ok(())
}
