//! Keychain storage for Glean OAuth tokens (macOS).
//!
//! Mirrors `google_api/token_store.rs` but for Glean credentials.
//! Stores the full `GleanToken` as JSON in the macOS Keychain.

use serde::{Deserialize, Serialize};

use super::GleanAuthError;

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.glean-auth";
const KEYCHAIN_ACCOUNT: &str = "glean-oauth-v1";

/// Glean OAuth token stored in the Keychain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GleanToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// The OIDC token endpoint (needed for refresh).
    pub token_endpoint: String,
    /// The OAuth client ID (populated by DCR, not user-provided).
    pub client_id: String,
    /// Optional client secret returned by DCR (confidential client).
    #[serde(default)]
    pub client_secret: Option<String>,
    /// RFC 3339 expiry timestamp.
    pub expiry: Option<String>,
    /// Authenticated user's email (from OIDC userinfo or id_token).
    pub email: Option<String>,
    /// Authenticated user's display name.
    pub name: Option<String>,
}

/// Run a `security` CLI command with retry + backoff for transient Keychain contention.
fn run_security_cmd(args: &[&str]) -> Result<std::process::Output, GleanAuthError> {
    const MAX_RETRIES: u32 = 4;
    const BASE_MS: u64 = 150;

    for attempt in 0..=MAX_RETRIES {
        let output = std::process::Command::new("security")
            .args(args)
            .output()
            .map_err(|e| GleanAuthError::Keychain(e.to_string()))?;

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
            "Glean keychain busy (attempt {}/{}), retrying in {}ms",
            attempt + 1,
            MAX_RETRIES,
            delay.as_millis()
        );
        std::thread::sleep(delay);
    }

    unreachable!()
}

/// Load the Glean OAuth token from the Keychain.
pub fn load_token() -> Result<GleanToken, GleanAuthError> {
    let output = run_security_cmd(&[
        "find-generic-password",
        "-a",
        KEYCHAIN_ACCOUNT,
        "-s",
        KEYCHAIN_SERVICE,
        "-w",
    ])?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        if stderr.contains("could not be found") || stderr.contains("item not found") {
            return Err(GleanAuthError::TokenNotFound);
        }
        return Err(GleanAuthError::Keychain(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let token: GleanToken =
        serde_json::from_str(&raw).map_err(|e| GleanAuthError::Other(e.to_string()))?;
    Ok(token)
}

/// Save a Glean OAuth token to the Keychain.
pub fn save_token(token: &GleanToken) -> Result<(), GleanAuthError> {
    let payload =
        serde_json::to_string(token).map_err(|e| GleanAuthError::Other(e.to_string()))?;
    let output = run_security_cmd(&[
        "add-generic-password",
        "-a",
        KEYCHAIN_ACCOUNT,
        "-s",
        KEYCHAIN_SERVICE,
        "-w",
        &payload,
        "-U",
    ])?;
    if !output.status.success() {
        return Err(GleanAuthError::Keychain(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(())
}

/// Delete the Glean OAuth token from the Keychain.
pub fn delete_token() -> Result<(), GleanAuthError> {
    let output = run_security_cmd(&[
        "delete-generic-password",
        "-a",
        KEYCHAIN_ACCOUNT,
        "-s",
        KEYCHAIN_SERVICE,
    ])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    if stderr.contains("could not be found") || stderr.contains("item not found") {
        return Ok(());
    }
    Err(GleanAuthError::Keychain(
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Non-panicking probe for the authenticated account email.
pub fn peek_account_email() -> Option<String> {
    match load_token() {
        Ok(token) => token
            .email
            .filter(|e| !e.trim().is_empty())
            .or(Some("connected".to_string())),
        Err(_) => None,
    }
}

/// Non-panicking probe for the authenticated account name.
pub fn peek_account_name() -> Option<String> {
    match load_token() {
        Ok(token) => token.name.filter(|n| !n.trim().is_empty()),
        Err(_) => None,
    }
}
