//! OAuth2 browser consent flow for Google APIs.
//!
//! Replaces google_auth.py. Opens the user's browser for consent,
//! captures the redirect on a localhost TcpListener, exchanges
//! the auth code for tokens, and fetches the user's email.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;

use super::{
    load_credentials, save_token, GoogleApiError, GoogleToken, SCOPES,
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

    // Bind to a random port
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| GoogleApiError::Io(e))?;
    let port = listener
        .local_addr()
        .map_err(|e| GoogleApiError::Io(e))?
        .port();
    let redirect_uri = format!("http://localhost:{}", port);

    // Build authorization URL
    let scope_string = SCOPES.join(" ");
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
        installed.auth_uri,
        urlencoding(&installed.client_id),
        urlencoding(&redirect_uri),
        urlencoding(&scope_string),
    );

    // Open browser
    log::info!("Opening browser for Google OAuth consent...");
    if let Err(e) = open::that(&auth_url) {
        log::warn!("Failed to open browser: {}. URL: {}", e, auth_url);
    }

    // Wait for the redirect with a timeout
    listener
        .set_nonblocking(false)
        .map_err(|e| GoogleApiError::Io(e))?;

    let auth_code = wait_for_auth_code(&listener)?;

    // Exchange auth code for tokens
    let client = reqwest::Client::new();
    let resp = client
        .post(&installed.token_uri)
        .form(&[
            ("code", auth_code.as_str()),
            ("client_id", installed.client_id.as_str()),
            ("client_secret", installed.client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(GoogleApiError::RefreshFailed(format!(
            "Token exchange failed: {}",
            body
        )));
    }

    let body: serde_json::Value = resp.json().await?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| GoogleApiError::RefreshFailed("No access_token in response".into()))?
        .to_string();
    let refresh_token = body["refresh_token"].as_str().map(|s| s.to_string());
    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    // Fetch user email via Gmail API
    let email = fetch_user_email(&access_token).await;

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

    save_token(&token)?;

    Ok(email)
}

/// Wait for the OAuth redirect and extract the auth code from the URL.
fn wait_for_auth_code(listener: &TcpListener) -> Result<String, GoogleApiError> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| GoogleApiError::Io(e))?;

    let mut buffer = [0u8; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| GoogleApiError::Io(e))?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    // Extract the code parameter from GET /?code=xxx&scope=... HTTP/1.1
    let code = request
        .lines()
        .next()
        .and_then(|line| {
            let path = line.split_whitespace().nth(1)?;
            let query = path.split('?').nth(1)?;
            query
                .split('&')
                .find(|p| p.starts_with("code="))
                .map(|p| p.strip_prefix("code=").unwrap_or("").to_string())
        })
        .ok_or(GoogleApiError::FlowCancelled)?;

    if code.is_empty() {
        // Check if user denied access
        let has_error = request.contains("error=");
        if has_error {
            send_response(&mut stream, "Authorization denied. You can close this tab.");
            return Err(GoogleApiError::FlowCancelled);
        }
        send_response(&mut stream, "No authorization code received. You can close this tab.");
        return Err(GoogleApiError::FlowCancelled);
    }

    // URL-decode the auth code (it may contain %2F etc.)
    let code = url_decode(&code);

    send_response(
        &mut stream,
        "Authorization successful! You can close this tab and return to DailyOS.",
    );

    Ok(code)
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
    match client
        .get("https://gmail.googleapis.com/gmail/v1/users/me/profile")
        .bearer_auth(access_token)
        .send()
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
    match client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(access_token)
        .send()
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

/// Simple URL decoding.
fn url_decode(s: &str) -> String {
    url::form_urlencoded::parse(s.as_bytes())
        .map(|(key, val)| {
            if val.is_empty() {
                key.to_string()
            } else {
                format!("{}={}", key, val)
            }
        })
        .collect::<Vec<_>>()
        .join("&")
        // If it was a single value (no =), just return the decoded key
        .split('=')
        .next()
        .unwrap_or(s)
        .to_string()
}
