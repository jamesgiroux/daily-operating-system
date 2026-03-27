//! OAuth2 browser consent flow for Google APIs.
//!
//! Replaces google_auth.py. Opens the user's browser for consent,
//! captures the redirect on a localhost TcpListener, exchanges
//! the auth code for tokens, and fetches the user's email.
//!
//! Shared OAuth primitives (PKCE, callback listener, HTML rendering)
//! live in `crate::oauth` and are reused by the Glean OAuth flow.

use std::net::TcpListener;
use std::path::Path;

use crate::oauth;

use super::{
    load_credentials, save_token, send_with_retry, GoogleApiError, GoogleToken, RetryPolicy, SCOPES,
};

/// Run the full OAuth2 consent flow.
///
/// 1. Load credentials.json
/// 2. Start TcpListener on a random port
/// 3. Open browser with auth URL
/// 4. Wait for redirect with auth code
/// 5. Exchange code for tokens
/// 6. Fetch user email
/// 7. Save token
///
/// Returns the authenticated email address.
pub async fn run_consent_flow(workspace: Option<&Path>) -> Result<String, GoogleApiError> {
    let creds = load_credentials(workspace)?;
    let installed = &creds.installed;
    let pkce_verifier = oauth::pkce_verifier();
    let pkce_challenge = oauth::pkce_challenge(&pkce_verifier);
    let oauth_state = oauth::generate_state();

    // Bind to a random port
    let listener = TcpListener::bind("127.0.0.1:0").map_err(GoogleApiError::Io)?;
    let port = listener.local_addr().map_err(GoogleApiError::Io)?.port();
    let redirect_uri = format!("http://localhost:{}", port);

    // Build authorization URL
    let scope_string = SCOPES.join(" ");
    let auth_url = build_auth_url(
        installed,
        &redirect_uri,
        &scope_string,
        &pkce_challenge,
        &oauth_state,
    );

    // Open browser
    log::info!("Opening browser for Google OAuth consent...");
    if let Err(e) = open::that(&auth_url) {
        log::warn!("Failed to open browser: {}. URL: {}", e, auth_url);
    }

    // Wait for the redirect (120s timeout in listen_for_callback guards
    // against macOS firewall blocking the browser's localhost redirect).
    let oauth::CallbackResult {
        callback,
        mut stream,
    } = oauth::listen_for_callback(&listener).map_err(|e| match e {
        oauth::CallbackError::Io(io_err) => GoogleApiError::Io(io_err),
        oauth::CallbackError::FlowCancelled => GoogleApiError::FlowCancelled,
        oauth::CallbackError::Timeout => GoogleApiError::OAuthTimeout,
    })?;
    if callback.state.as_deref() != Some(oauth_state.as_str()) {
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            "State mismatch detected. Please return to DailyOS and try connecting again.",
        );
        return Err(GoogleApiError::OAuthStateMismatch);
    }

    // Exchange auth code for tokens (browser is waiting — shows loading spinner)
    let include_secret = installed.client_secret.is_some();
    log::info!(
        "OAuth: exchanging auth code for tokens (client_secret={})",
        if include_secret { "present" } else { "absent" },
    );
    let client = reqwest::Client::new();
    let (status, body_text) = match exchange_auth_code(
        &client,
        installed,
        &callback.code,
        &redirect_uri,
        &pkce_verifier,
        include_secret,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("OAuth: token exchange request failed: {}", e);
            oauth::send_error_response(
                &mut stream,
                "Authorization failed",
                "Could not reach Google during token exchange. Please return to DailyOS and try again.",
            );
            return Err(e);
        }
    };
    log::info!("OAuth: token exchange response status={}", status);
    let body: serde_json::Value = if status.is_success() {
        serde_json::from_str(&body_text)?
    } else {
        log::error!(
            "OAuth: token exchange failed: status={} body={}",
            status,
            body_text
        );
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            &format!(
                "Google returned {} during authorization. Please return to DailyOS and try again.",
                status
            ),
        );
        return Err(GoogleApiError::RefreshFailed(format!(
            "Token exchange failed ({}): {}",
            status, body_text
        )));
    };

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| GoogleApiError::RefreshFailed("No access_token in response".into()))?
        .to_string();
    let refresh_token = body["refresh_token"].as_str().map(|s| s.to_string());
    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    // Fetch user email via Gmail API
    let email = fetch_user_email(&access_token).await;
    log::info!("OAuth: authenticated as {}", email);

    let token = GoogleToken {
        token: access_token,
        refresh_token,
        token_uri: installed.token_uri.clone(),
        client_id: installed.client_id.clone(),
        client_secret: installed.client_secret.clone(),
        scopes: SCOPES.iter().map(|s| s.to_string()).collect(),
        expiry: Some(expiry.to_rfc3339()),
        account: Some(email.clone()),
        universe_domain: Some("googleapis.com".to_string()),
    };

    if let Err(e) = save_token(&token) {
        log::error!("OAuth: failed to save token: {}", e);
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            "Credentials could not be saved. Please return to DailyOS and check logs.",
        );
        return Err(e);
    }

    // Token saved — NOW tell the browser it worked
    oauth::send_success_response(
        &mut stream,
        "Google account connected",
        "You can close this tab and return to DailyOS. Settings will update automatically.",
    );
    log::info!("OAuth: flow complete, token saved");

    Ok(email)
}

// Callback listener, HTML rendering, and PKCE helpers are in crate::oauth.

/// Fetch the user's email address from the Gmail API.
///
/// Falls back to "authenticated" if the API call fails.
async fn fetch_user_email(access_token: &str) -> String {
    let client = reqwest::Client::new();

    // Try Gmail users.getProfile first
    match send_with_retry(
        client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
            .bearer_auth(access_token),
        &RetryPolicy::default(),
    )
    .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(email) = body["emailAddress"].as_str() {
                    return email.to_string();
                }
            }
        }
        _ => {}
    }

    // Fallback: OAuth2 userinfo endpoint
    match send_with_retry(
        client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token),
        &RetryPolicy::default(),
    )
    .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(email) = body["email"].as_str() {
                    return email.to_string();
                }
            }
        }
        _ => {}
    }

    "authenticated".to_string()
}

fn build_auth_url(
    installed: &super::InstalledAppCredentials,
    redirect_uri: &str,
    scope_string: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent&code_challenge={}&code_challenge_method=S256&state={}",
        installed.auth_uri,
        oauth::urlencode(&installed.client_id),
        oauth::urlencode(redirect_uri),
        oauth::urlencode(scope_string),
        oauth::urlencode(code_challenge),
        oauth::urlencode(state),
    )
}

// parse_query_params is in crate::oauth

async fn exchange_auth_code(
    client: &reqwest::Client,
    installed: &super::InstalledAppCredentials,
    auth_code: &str,
    redirect_uri: &str,
    code_verifier: &str,
    include_client_secret: bool,
) -> Result<(reqwest::StatusCode, String), GoogleApiError> {
    let mut form = vec![
        ("code", auth_code),
        ("client_id", installed.client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", code_verifier),
    ];
    if include_client_secret {
        if let Some(secret) = installed.client_secret.as_deref() {
            form.push(("client_secret", secret));
        }
    }

    let response = send_with_retry(
        client.post(&installed.token_uri).form(&form),
        &RetryPolicy::default(),
    )
    .await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Ok((status, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_url_includes_pkce_and_state() {
        let creds = super::super::InstalledAppCredentials {
            client_id: "cid".to_string(),
            client_secret: None,
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
        };
        let url = build_auth_url(
            &creds,
            "http://localhost:8080",
            "scope1 scope2",
            "challenge",
            "state123",
        );
        assert!(url.contains("code_challenge=challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state123"));
    }
}
