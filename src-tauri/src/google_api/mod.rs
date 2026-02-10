//! Native Google API client (ADR-0049: Eliminate Python runtime)
//!
//! Replaces Python google-api-python-client + google-auth-oauthlib with
//! direct HTTP via reqwest. Token format is compatible with the existing
//! ~/.dailyos/google/token.json written by the Python OAuth library.
//!
//! Modules:
//! - auth: OAuth2 browser consent flow
//! - calendar: Google Calendar API v3
//! - classify: 10-rule meeting classification (MEETING-TYPES.md)
//! - gmail: Gmail API v1

pub mod auth;
pub mod calendar;
pub mod classify;
pub mod gmail;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Google OAuth2 scopes used by DailyOS.
pub const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/calendar",
    "https://www.googleapis.com/auth/gmail.modify",
    "https://www.googleapis.com/auth/gmail.compose",
    "https://www.googleapis.com/auth/spreadsheets",
    "https://www.googleapis.com/auth/documents",
    "https://www.googleapis.com/auth/drive",
];

// ============================================================================
// Token types — must be compatible with Python's google-auth token format
// ============================================================================

/// OAuth2 token persisted to ~/.dailyos/google/token.json.
///
/// Field names match what Python's `google.oauth2.credentials.Credentials.to_json()`
/// produces. Both `token` and `access_token` are accepted on read for compat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleToken {
    /// The access token (Python writes this as "token")
    #[serde(alias = "access_token")]
    pub token: String,
    /// The refresh token (long-lived, used to get new access tokens)
    pub refresh_token: Option<String>,
    /// Token endpoint URL
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
    /// OAuth2 client ID
    pub client_id: String,
    /// OAuth2 client secret
    pub client_secret: String,
    /// Authorized scopes
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Token expiry time (ISO 8601)
    #[serde(default)]
    pub expiry: Option<String>,
    /// Authenticated user email (Python stores in "account" field)
    #[serde(default, alias = "email")]
    pub account: Option<String>,
    /// Universe domain (Python includes this)
    #[serde(default)]
    pub universe_domain: Option<String>,
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

/// OAuth2 client credentials from credentials.json (Desktop App type).
#[derive(Debug, Clone, Deserialize)]
pub struct ClientCredentials {
    pub installed: InstalledAppCredentials,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstalledAppCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub auth_uri: String,
    pub token_uri: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

// ============================================================================
// Error type
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum GoogleApiError {
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Token expired or revoked")]
    AuthExpired,
    #[error("Credentials not found at {0}")]
    CredentialsNotFound(PathBuf),
    #[error("Token not found at {0}")]
    TokenNotFound(PathBuf),
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),
    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("OAuth flow cancelled")]
    FlowCancelled,
    #[error("Invalid credentials format: {0}")]
    InvalidCredentials(String),
}

// ============================================================================
// Token I/O
// ============================================================================

/// Canonical path to the Google token file.
pub fn token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("google")
        .join("token.json")
}

/// Canonical path to the Google credentials file (primary location).
pub fn credentials_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("google")
        .join("credentials.json")
}

/// Load token from disk. Returns None if file doesn't exist.
pub fn load_token() -> Result<GoogleToken, GoogleApiError> {
    let path = token_path();
    if !path.exists() {
        return Err(GoogleApiError::TokenNotFound(path));
    }
    let content = std::fs::read_to_string(&path)?;
    let token: GoogleToken = serde_json::from_str(&content)?;
    Ok(token)
}

/// Save token to disk atomically with 0o600 permissions.
pub fn save_token(token: &GoogleToken) -> Result<(), GoogleApiError> {
    let path = token_path();

    // Ensure directory exists with 0o700 permissions
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

    // Set 0o600 on the token file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Load client credentials.
///
/// Resolution order:
/// 1. ~/.dailyos/google/credentials.json (dev override)
/// 2. <workspace>/.config/google/credentials.json (CLI-era fallback)
/// 3. Embedded defaults (production — I123)
pub fn load_credentials(workspace: Option<&Path>) -> Result<ClientCredentials, GoogleApiError> {
    // Dev override: file on disk takes priority
    let primary = credentials_path();
    if primary.exists() {
        let content = std::fs::read_to_string(&primary)?;
        let creds: ClientCredentials = serde_json::from_str(&content)
            .map_err(|e| GoogleApiError::InvalidCredentials(format!("{}: {}", primary.display(), e)))?;
        return Ok(creds);
    }

    if let Some(ws) = workspace {
        let fallback = ws.join(".config").join("google").join("credentials.json");
        if fallback.exists() {
            let content = std::fs::read_to_string(&fallback)?;
            let creds: ClientCredentials = serde_json::from_str(&content)
                .map_err(|e| GoogleApiError::InvalidCredentials(format!("{}: {}", fallback.display(), e)))?;
            return Ok(creds);
        }
    }

    // Production defaults — no credentials.json needed
    Ok(embedded_credentials())
}

/// Built-in OAuth client credentials (I123).
///
/// These are the production DailyOS Desktop App credentials registered in
/// Google Cloud. Users don't need to supply their own credentials.json.
/// A file on disk still overrides these for local development.
fn embedded_credentials() -> ClientCredentials {
    ClientCredentials {
        installed: InstalledAppCredentials {
            client_id: "245504828099-06i3l5339nkhr5ffq08qn3h9omci4efn.apps.googleusercontent.com".to_string(),
            client_secret: "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".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
        },
    }
}

// ============================================================================
// Token refresh
// ============================================================================

/// Global mutex to serialize concurrent token refreshes.
static TOKEN_REFRESH_MUTEX: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

fn refresh_mutex() -> &'static Mutex<()> {
    TOKEN_REFRESH_MUTEX.get_or_init(|| Mutex::new(()))
}

/// Check if a token is expired based on its expiry field.
pub fn is_token_expired(token: &GoogleToken) -> bool {
    match &token.expiry {
        None => true, // No expiry = assume expired, try refresh
        Some(expiry_str) => {
            // Python stores expiry as "2026-02-08T12:00:00.000000Z" or similar
            match chrono::DateTime::parse_from_rfc3339(
                &expiry_str.replace('Z', "+00:00"),
            )
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(expiry_str))
            {
                Ok(expiry) => {
                    // Consider expired if within 60 seconds of expiry
                    let now = chrono::Utc::now();
                    expiry <= now + chrono::Duration::seconds(60)
                }
                Err(_) => true, // Can't parse = assume expired
            }
        }
    }
}

/// Refresh an access token using the refresh token.
///
/// Returns an updated GoogleToken with new access token and expiry.
/// Serializes concurrent refreshes via a tokio Mutex.
pub async fn refresh_access_token(token: &GoogleToken) -> Result<GoogleToken, GoogleApiError> {
    let _guard = refresh_mutex().lock().await;

    let refresh_token = token
        .refresh_token
        .as_ref()
        .ok_or(GoogleApiError::AuthExpired)?;

    let client = reqwest::Client::new();
    let resp = client
        .post(&token.token_uri)
        .form(&[
            ("client_id", token.client_id.as_str()),
            ("client_secret", token.client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status == 400 || status == 401 {
            return Err(GoogleApiError::AuthExpired);
        }
        return Err(GoogleApiError::RefreshFailed(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    let body: serde_json::Value = resp.json().await?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| GoogleApiError::RefreshFailed("No access_token in response".into()))?;

    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    let mut new_token = token.clone();
    new_token.token = access_token.to_string();
    new_token.expiry = Some(expiry.to_rfc3339());

    // Persist the refreshed token
    save_token(&new_token)?;

    Ok(new_token)
}

/// Get a valid access token, refreshing if expired.
///
/// This is the main entry point for all API calls.
pub async fn get_valid_access_token() -> Result<String, GoogleApiError> {
    let token = load_token()?;

    if is_token_expired(&token) {
        let refreshed = refresh_access_token(&token).await?;
        Ok(refreshed.token)
    } else {
        Ok(token.token)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_token_roundtrip() {
        let token = GoogleToken {
            token: "ya29.test-access-token".to_string(),
            refresh_token: Some("1//test-refresh-token".to_string()),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            client_id: "12345.apps.googleusercontent.com".to_string(),
            client_secret: "test-secret".to_string(),
            scopes: vec!["https://www.googleapis.com/auth/calendar".to_string()],
            expiry: Some("2026-02-08T12:00:00Z".to_string()),
            account: Some("user@example.com".to_string()),
            universe_domain: Some("googleapis.com".to_string()),
        };

        let json = serde_json::to_string_pretty(&token).unwrap();
        let parsed: GoogleToken = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.token, "ya29.test-access-token");
        assert_eq!(parsed.refresh_token.as_deref(), Some("1//test-refresh-token"));
        assert_eq!(parsed.client_id, "12345.apps.googleusercontent.com");
        assert_eq!(parsed.account.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn test_google_token_python_compat() {
        // Simulates the JSON format Python's google-auth writes
        let python_json = r#"{
            "token": "ya29.python-token",
            "refresh_token": "1//python-refresh",
            "token_uri": "https://oauth2.googleapis.com/token",
            "client_id": "client.apps.googleusercontent.com",
            "client_secret": "secret",
            "scopes": [
                "https://www.googleapis.com/auth/calendar",
                "https://www.googleapis.com/auth/gmail.modify"
            ],
            "expiry": "2026-02-08T12:00:00.000000Z",
            "account": "user@company.com",
            "universe_domain": "googleapis.com"
        }"#;

        let token: GoogleToken = serde_json::from_str(python_json).unwrap();
        assert_eq!(token.token, "ya29.python-token");
        assert_eq!(token.account.as_deref(), Some("user@company.com"));
        assert_eq!(token.scopes.len(), 2);
    }

    #[test]
    fn test_google_token_access_token_alias() {
        // Some implementations use "access_token" instead of "token"
        let json = r#"{
            "access_token": "ya29.alias-token",
            "refresh_token": "1//refresh",
            "client_id": "client",
            "client_secret": "secret"
        }"#;

        let token: GoogleToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.token, "ya29.alias-token");
    }

    #[test]
    fn test_is_token_expired_no_expiry() {
        let token = GoogleToken {
            token: "test".to_string(),
            refresh_token: None,
            token_uri: default_token_uri(),
            client_id: "c".to_string(),
            client_secret: "s".to_string(),
            scopes: vec![],
            expiry: None,
            account: None,
            universe_domain: None,
        };
        assert!(is_token_expired(&token));
    }

    #[test]
    fn test_is_token_expired_future() {
        let future = chrono::Utc::now() + chrono::Duration::hours(1);
        let token = GoogleToken {
            token: "test".to_string(),
            refresh_token: None,
            token_uri: default_token_uri(),
            client_id: "c".to_string(),
            client_secret: "s".to_string(),
            scopes: vec![],
            expiry: Some(future.to_rfc3339()),
            account: None,
            universe_domain: None,
        };
        assert!(!is_token_expired(&token));
    }

    #[test]
    fn test_is_token_expired_past() {
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        let token = GoogleToken {
            token: "test".to_string(),
            refresh_token: None,
            token_uri: default_token_uri(),
            client_id: "c".to_string(),
            client_secret: "s".to_string(),
            scopes: vec![],
            expiry: Some(past.to_rfc3339()),
            account: None,
            universe_domain: None,
        };
        assert!(is_token_expired(&token));
    }

    #[test]
    fn test_credentials_json_parsing() {
        let json = r#"{
            "installed": {
                "client_id": "12345.apps.googleusercontent.com",
                "client_secret": "secret",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "redirect_uris": ["http://localhost"]
            }
        }"#;

        let creds: ClientCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.installed.client_id, "12345.apps.googleusercontent.com");
        assert_eq!(creds.installed.redirect_uris, vec!["http://localhost"]);
    }
}
