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
    let pkce_verifier = generate_code_verifier();
    let pkce_challenge = derive_code_challenge(&pkce_verifier);
    let oauth_state = generate_state();

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

    // Wait for the redirect with a timeout
    listener
        .set_nonblocking(false)
        .map_err(GoogleApiError::Io)?;

    let CallbackResult {
        callback,
        mut stream,
    } = wait_for_auth_callback(&listener)?;
    if callback.state.as_deref() != Some(oauth_state.as_str()) {
        send_error_response(
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
            send_error_response(
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
        send_error_response(
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
        send_error_response(
            &mut stream,
            "Authorization failed",
            "Credentials could not be saved. Please return to DailyOS and check logs.",
        );
        return Err(e);
    }

    // Token saved — NOW tell the browser it worked
    send_success_response(
        &mut stream,
        "Google account connected",
        "You can close this tab and return to DailyOS. Settings will update automatically.",
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
    let n = stream.read(&mut buffer).map_err(GoogleApiError::Io)?;
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
            send_info_response(
                &mut stream,
                "Authorization cancelled",
                "No changes were made. You can close this tab and return to DailyOS.",
            );
            return Err(GoogleApiError::FlowCancelled);
        }
        send_error_response(
            &mut stream,
            "Authorization failed",
            "Google returned an error response. You can close this tab and return to DailyOS.",
        );
        return Err(GoogleApiError::FlowCancelled);
    }

    let code = code.ok_or(GoogleApiError::FlowCancelled)?;
    if code.is_empty() {
        send_error_response(
            &mut stream,
            "Authorization failed",
            "No authorization code was returned. Please close this tab and try again from DailyOS.",
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
enum CallbackTone {
    Success,
    Error,
    Info,
}

fn render_callback_html(title: &str, message: &str, tone: CallbackTone) -> String {
    // Editorial design system colors (ADR-0076)
    let (accent, rule_color) = match tone {
        CallbackTone::Success => ("#7eaa7b", "#7eaa7b"), // Sage
        CallbackTone::Error => ("#c4654a", "#c4654a"),   // Terracotta
        CallbackTone::Info => ("#c9a227", "#c9a227"),    // Turmeric
    };

    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>DailyOS</title>
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500&family=JetBrains+Mono:wght@500&display=swap" rel="stylesheet" />
</head>
<body style="margin:0;background:#f5f2ef;color:#1e2530;font-family:'DM Sans',sans-serif;">
<main style="min-height:100vh;display:flex;align-items:center;justify-content:center;padding:48px 24px;">
<section style="max-width:480px;width:100%;text-align:center;">
<div style="font-family:'Montserrat','DM Sans',sans-serif;font-size:28px;font-weight:800;color:#c9a227;margin-bottom:48px;">*</div>
<div style="font-family:'JetBrains Mono',monospace;font-size:10px;font-weight:500;text-transform:uppercase;letter-spacing:0.1em;color:{accent};margin-bottom:16px;">DailyOS</div>
<h1 style="margin:0 0 16px;font-family:'DM Sans',sans-serif;font-size:24px;font-weight:500;line-height:1.35;color:#1e2530;">{title}</h1>
<p style="margin:0 0 40px;font-family:'DM Sans',sans-serif;font-size:15px;line-height:1.65;color:#6b7280;max-width:360px;margin-left:auto;margin-right:auto;">{message}</p>
<div style="border-top:1px solid {rule_color};padding-top:20px;max-width:320px;margin:0 auto;">
<p style="margin:0;font-family:'JetBrains Mono',monospace;font-size:11px;letter-spacing:0.04em;color:#6b7280;">Return to DailyOS. This window can be closed.</p>
</div>
</section>
</main>
</body>
</html>"#
    )
}

fn send_response(stream: &mut impl Write, title: &str, message: &str, tone: CallbackTone) {
    let body = render_callback_html(title, message, tone);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn send_success_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Success);
}

fn send_error_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Error);
}

fn send_info_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Info);
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
