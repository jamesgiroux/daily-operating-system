//! OAuth2 browser consent flow for Google APIs.
//!
//! Replaces google_auth.py. Opens the user's browser for consent,
//! captures the redirect on a localhost TcpListener, exchanges
//! the auth code for tokens, and fetches the user's email.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use base64::Engine;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{
    load_credentials, save_token, send_with_retry, GoogleApiError, GoogleToken, RetryPolicy,
    SCOPES,
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
    let pkce_verifier = generate_code_verifier();
    let pkce_challenge = derive_code_challenge(&pkce_verifier);
    let oauth_state = generate_state();

    // Bind to a random port
    let listener = TcpListener::bind("127.0.0.1:0").map_err(GoogleApiError::Io)?;
    let port = listener
        .local_addr()
        .map_err(GoogleApiError::Io)?
        .port();
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

    // Wait for the redirect with a timeout
    listener
        .set_nonblocking(false)
        .map_err(GoogleApiError::Io)?;

    let CallbackResult { callback, mut stream } = wait_for_auth_callback(&listener)?;
    if callback.state.as_deref() != Some(oauth_state.as_str()) {
        send_response(&mut stream, "Authorization failed: state mismatch. Please try again.");
        return Err(GoogleApiError::OAuthStateMismatch);
    }

    // Exchange auth code for tokens (browser is waiting — shows loading spinner)
    log::info!("OAuth: exchanging auth code for tokens...");
    let client = reqwest::Client::new();
    let (status, body_text) = match exchange_auth_code(
        &client,
        installed,
        &callback.code,
        &redirect_uri,
        &pkce_verifier,
        false,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            log::error!("OAuth: token exchange request failed: {}", e);
            send_response(&mut stream, "Authorization failed: could not reach Google. Please try again.");
            return Err(e);
        }
    };
    log::info!("OAuth: token exchange response status={}", status);
    let body: serde_json::Value = if status.is_success() {
        serde_json::from_str(&body_text)?
    } else if status.as_u16() == 400
        && body_text.contains("invalid_client")
        && installed.client_secret.is_some()
    {
        log::info!("OAuth: retrying token exchange with client_secret...");
        let (retry_status, retry_body) = exchange_auth_code(
            &client,
            installed,
            &callback.code,
            &redirect_uri,
            &pkce_verifier,
            true,
        )
        .await?;
        if !retry_status.is_success() {
            log::error!("OAuth: retry token exchange failed: status={} body={}", retry_status, retry_body);
            send_response(&mut stream, "Authorization failed: token exchange rejected by Google. Please try again.");
            return Err(GoogleApiError::RefreshFailed(format!(
                "Token exchange failed: {}",
                retry_body
            )));
        }
        serde_json::from_str(&retry_body)?
    } else {
        log::error!("OAuth: token exchange failed: status={} body={}", status, body_text);
        send_response(&mut stream, &format!(
            "Authorization failed: Google returned {}. Please try again or check the app logs.",
            status,
        ));
        return Err(GoogleApiError::RefreshFailed(format!(
            "Token exchange failed: {}",
            body_text
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
        client_secret: None,
        scopes: SCOPES.iter().map(|s| s.to_string()).collect(),
        expiry: Some(expiry.to_rfc3339()),
        account: Some(email.clone()),
        universe_domain: Some("googleapis.com".to_string()),
    };

    if let Err(e) = save_token(&token) {
        log::error!("OAuth: failed to save token: {}", e);
        send_response(&mut stream, "Authorization failed: could not save credentials. Check app logs.");
        return Err(e);
    }

    // Token saved — NOW tell the browser it worked
    send_response(
        &mut stream,
        "Authorization successful! You can close this tab and return to DailyOS.",
    );
    log::info!("OAuth: flow complete, token saved");

    Ok(email)
}

/// Result from waiting for the OAuth callback — includes the TCP stream so the
/// caller can send the final response after the token exchange completes.
struct CallbackResult {
    callback: AuthCallback,
    stream: TcpStream,
}

/// Wait for the OAuth redirect and extract the auth code from the URL.
///
/// Does NOT send a success response — the caller must do that after the token
/// exchange succeeds. Error responses (denied, missing code) are sent immediately
/// since those are terminal.
fn wait_for_auth_callback(listener: &TcpListener) -> Result<CallbackResult, GoogleApiError> {
    let (mut stream, _) = listener.accept().map_err(GoogleApiError::Io)?;

    let mut buffer = [0u8; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(GoogleApiError::Io)?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    let query = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| path.split('?').nth(1))
        .ok_or(GoogleApiError::FlowCancelled)?;
    let params = parse_query_params(query);
    let code = params.get("code").cloned();
    let state = params.get("state").cloned();
    let error = params.get("error").cloned();

    if let Some(error_code) = error {
        if error_code == "access_denied" {
            send_response(&mut stream, "Authorization denied. You can close this tab.");
            return Err(GoogleApiError::FlowCancelled);
        }
        send_response(&mut stream, "Authorization failed. You can close this tab.");
        return Err(GoogleApiError::FlowCancelled);
    }

    let code = code.ok_or(GoogleApiError::FlowCancelled)?;
    if code.is_empty() {
        send_response(
            &mut stream,
            "No authorization code received. You can close this tab.",
        );
        return Err(GoogleApiError::FlowCancelled);
    }

    log::info!("OAuth: received auth code from browser callback");

    // Don't send success yet — caller will respond after token exchange
    Ok(CallbackResult {
        callback: AuthCallback { code, state },
        stream,
    })
}

/// Send an HTTP response to the browser.
fn send_response(stream: &mut impl Write, message: &str) {
    let body = format!(
        "<html><body style=\"font-family: system-ui; text-align: center; padding: 40px;\">\
         <h2>{}</h2></body></html>",
        message
    );
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

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

/// Simple percent-encoding for URL parameters.
fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[derive(Debug, Clone)]
struct AuthCallback {
    code: String,
    state: Option<String>,
}

fn generate_code_verifier() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn derive_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn generate_state() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
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
        urlencoding(&installed.client_id),
        urlencoding(redirect_uri),
        urlencoding(scope_string),
        urlencoding(code_challenge),
        urlencoding(state),
    )
}

fn parse_query_params(query: &str) -> std::collections::HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

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
    fn test_pkce_challenge_shape() {
        let verifier = generate_code_verifier();
        let challenge = derive_code_challenge(&verifier);
        assert!(!challenge.is_empty());
        assert!(!challenge.contains('='));
        assert!(challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_parse_query_params_decodes_values() {
        let params = parse_query_params("code=a%2Fb&state=x-y_z&scope=abc");
        assert_eq!(params.get("code").map(String::as_str), Some("a/b"));
        assert_eq!(params.get("state").map(String::as_str), Some("x-y_z"));
    }

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
