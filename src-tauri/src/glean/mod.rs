//! Glean OAuth module — MCP OAuth discovery + DCR for Glean's MCP server.
//!
//! Provides:
//! - MCP OAuth two-step discovery (Protected Resource Metadata → AS metadata)
//! - Dynamic Client Registration (RFC 7591) — no user-provided client ID needed
//! - Browser-based OAuth consent flow (reuses `crate::oauth` primitives)
//! - Keychain token storage with automatic refresh

pub mod oauth;
pub mod token_store;

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use token_store::GleanToken;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from Glean authentication operations.
#[derive(Debug)]
pub enum GleanAuthError {
    /// Token not found in Keychain.
    TokenNotFound,
    /// Keychain operation failed.
    Keychain(String),
    /// OIDC discovery failed.
    Discovery(String),
    /// OAuth state mismatch (CSRF protection).
    StateMismatch,
    /// Token exchange failed.
    TokenExchange(String),
    /// Token refresh failed.
    RefreshFailed(String),
    /// User cancelled the OAuth flow.
    FlowCancelled,
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for GleanAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenNotFound => write!(f, "Glean token not found"),
            Self::Keychain(msg) => write!(f, "Glean keychain error: {}", msg),
            Self::Discovery(msg) => write!(f, "Glean OIDC discovery error: {}", msg),
            Self::StateMismatch => write!(f, "Glean OAuth state mismatch"),
            Self::TokenExchange(msg) => write!(f, "Glean token exchange error: {}", msg),
            Self::RefreshFailed(msg) => write!(f, "Glean token refresh failed: {}", msg),
            Self::FlowCancelled => write!(f, "Glean OAuth flow cancelled"),
            Self::Other(msg) => write!(f, "Glean auth error: {}", msg),
        }
    }
}

impl std::error::Error for GleanAuthError {}

// ---------------------------------------------------------------------------
// Auth status (mirrors GoogleAuthStatus)
// ---------------------------------------------------------------------------

/// Glean authentication status for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum GleanAuthStatus {
    /// No Glean credentials configured.
    NotConfigured,
    /// Successfully authenticated with Glean.
    Authenticated { email: String, name: Option<String> },
}

// ---------------------------------------------------------------------------
// Token refresh
// ---------------------------------------------------------------------------

/// Mutex to serialize token refresh attempts (same pattern as Google OAuth).
static REFRESH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn refresh_mutex() -> &'static Mutex<()> {
    REFRESH_LOCK.get_or_init(|| Mutex::new(()))
}

/// Get a valid Glean access token, refreshing if expired.
///
/// Returns the access token string ready for use in Authorization headers.
/// Handles automatic refresh with mutex serialization to prevent thundering herd.
pub async fn get_valid_access_token() -> Result<String, GleanAuthError> {
    let token = token_store::load_token()?;

    // Check if token is expired (with 60s buffer)
    if let Some(ref expiry_str) = token.expiry {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
            let now = chrono::Utc::now();
            let buffer = chrono::Duration::seconds(60);
            if now + buffer < expiry {
                // Token is still valid
                return Ok(token.access_token);
            }
        }
    }

    // Token expired or no expiry info — try to refresh
    let _guard = refresh_mutex().lock().await;

    // Re-check after acquiring lock (another task may have already refreshed)
    let token = token_store::load_token()?;
    if let Some(ref expiry_str) = token.expiry {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
            let now = chrono::Utc::now();
            let buffer = chrono::Duration::seconds(60);
            if now + buffer < expiry {
                return Ok(token.access_token);
            }
        }
    }

    // Actually refresh
    match refresh_token(&token).await {
        Ok(access_token) => {
            // I572: Structured log for audit trail
            log::info!("[audit:security] oauth_token_refreshed provider=glean");
            Ok(access_token)
        }
        Err(e) => {
            log::warn!("[audit:security] oauth_token_refresh_failed provider=glean error={}", e);
            Err(e)
        }
    }
}

/// Refresh the Glean access token using the refresh token.
async fn refresh_token(token: &GleanToken) -> Result<String, GleanAuthError> {
    let refresh_token = token.refresh_token.as_deref().ok_or_else(|| {
        GleanAuthError::RefreshFailed("No refresh token available — re-authenticate".into())
    })?;

    log::info!("Glean: refreshing access token");

    let client = reqwest::Client::new();
    let mut form_params = vec![
        ("grant_type".to_string(), "refresh_token".to_string()),
        ("refresh_token".to_string(), refresh_token.to_string()),
        ("client_id".to_string(), token.client_id.clone()),
    ];
    // Include client_secret if DCR returned one (confidential client)
    if let Some(ref secret) = token.client_secret {
        form_params.push(("client_secret".to_string(), secret.clone()));
    }
    let resp = client
        .post(&token.token_endpoint)
        .form(&form_params)
        .send()
        .await
        .map_err(|e| GleanAuthError::RefreshFailed(format!("Refresh request failed: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        log::error!(
            "Glean token refresh failed: status={} body={}",
            status,
            body_text
        );
        return Err(GleanAuthError::RefreshFailed(format!(
            "Refresh failed ({}): {}",
            status, body_text
        )));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
        GleanAuthError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
    })?;

    let new_access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| GleanAuthError::RefreshFailed("No access_token in refresh response".into()))?
        .to_string();

    let new_refresh_token = body["refresh_token"]
        .as_str()
        .map(|s| s.to_string())
        .or_else(|| token.refresh_token.clone());

    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    // Update stored token
    let updated = GleanToken {
        access_token: new_access_token.clone(),
        refresh_token: new_refresh_token,
        token_endpoint: token.token_endpoint.clone(),
        client_id: token.client_id.clone(),
        client_secret: token.client_secret.clone(),
        expiry: Some(expiry.to_rfc3339()),
        email: token.email.clone(),
        name: token.name.clone(),
    };

    token_store::save_token(&updated)?;
    log::info!("Glean: token refreshed, new expiry={}", expiry.to_rfc3339());

    Ok(new_access_token)
}

/// Detect current Glean auth status from Keychain.
pub fn detect_glean_auth() -> GleanAuthStatus {
    match token_store::load_token() {
        Ok(token) => {
            let email = token
                .email
                .filter(|e| !e.trim().is_empty())
                .unwrap_or_else(|| "connected".to_string());
            GleanAuthStatus::Authenticated {
                email,
                name: token.name,
            }
        }
        Err(_) => GleanAuthStatus::NotConfigured,
    }
}
