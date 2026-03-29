//! Shared OAuth2 primitives used by Google and Glean consent flows.
//!
//! Extracted from `google_api/auth.rs` to avoid duplication. Provides:
//! - PKCE code verifier / challenge generation
//! - State nonce generation
//! - Localhost TCP listener for OAuth callbacks
//! - Callback HTML rendering (editorial design system)
//! - Query parameter parsing

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use base64::Engine;
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

/// Generate a PKCE code verifier (random 64-char hex string).
pub fn pkce_verifier() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Derive a PKCE code challenge from a verifier (SHA-256 + base64url, no padding).
pub fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

/// Generate a random state nonce for CSRF protection.
pub fn generate_state() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

// ---------------------------------------------------------------------------
// URL encoding
// ---------------------------------------------------------------------------

/// Percent-encode a string for use in URL query parameters.
pub fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ---------------------------------------------------------------------------
// Query parsing
// ---------------------------------------------------------------------------

/// Parse URL query parameters into a HashMap.
pub fn parse_query_params(query: &str) -> std::collections::HashMap<String, String> {
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

// ---------------------------------------------------------------------------
// Callback types
// ---------------------------------------------------------------------------

/// Result from waiting for the OAuth callback — includes the TCP stream so the
/// caller can send the final response after the token exchange completes.
pub struct CallbackResult {
    pub callback: AuthCallback,
    pub stream: TcpStream,
}

/// Parsed OAuth callback parameters.
#[derive(Debug, Clone)]
pub struct AuthCallback {
    pub code: String,
    pub state: Option<String>,
}

/// Tone for callback HTML response (maps to editorial design system colors).
pub enum CallbackTone {
    Success,
    Error,
    Info,
}

// ---------------------------------------------------------------------------
// Localhost callback listener
// ---------------------------------------------------------------------------

/// Wait for an OAuth redirect on the given TCP listener.
///
/// Extracts the auth code and state from the URL query parameters.
/// Sends error/info responses for terminal failures (denied, missing code).
/// Does NOT send a success response — the caller must do that after the
/// token exchange succeeds.
///
/// Times out after 120 seconds to prevent thread leaks when the macOS
/// Application Firewall blocks the browser's redirect to localhost.
pub fn listen_for_callback(listener: &TcpListener) -> Result<CallbackResult, CallbackError> {
    // Poll with a 120-second deadline so the backend doesn't hang forever if
    // the browser redirect is blocked (e.g., macOS firewall denying incoming
    // connections to DailyOS).
    listener.set_nonblocking(true).map_err(CallbackError::Io)?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    let (mut stream, _) = loop {
        match listener.accept() {
            Ok(conn) => break conn,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    log::warn!("OAuth callback timed out after 120s — browser redirect may have been blocked by firewall");
                    return Err(CallbackError::Timeout);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(CallbackError::Io(e)),
        }
    };

    // Restore blocking mode for the accepted stream's read
    stream.set_nonblocking(false).map_err(CallbackError::Io)?;

    let mut buffer = [0u8; 4096];
    let n = stream.read(&mut buffer).map_err(CallbackError::Io)?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    let query = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| path.split('?').nth(1))
        .ok_or(CallbackError::FlowCancelled)?;
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
            return Err(CallbackError::FlowCancelled);
        }
        send_error_response(
            &mut stream,
            "Authorization failed",
            "The authorization server returned an error. You can close this tab and return to DailyOS.",
        );
        return Err(CallbackError::FlowCancelled);
    }

    let code = code.ok_or(CallbackError::FlowCancelled)?;
    if code.is_empty() {
        send_error_response(
            &mut stream,
            "Authorization failed",
            "No authorization code was returned. Please close this tab and try again from DailyOS.",
        );
        return Err(CallbackError::FlowCancelled);
    }

    log::info!("OAuth: received auth code from browser callback");

    Ok(CallbackResult {
        callback: AuthCallback { code, state },
        stream,
    })
}

// ---------------------------------------------------------------------------
// Callback HTML rendering
// ---------------------------------------------------------------------------

/// Render an editorial-styled HTML page for the OAuth callback response.
pub fn render_callback_html(title: &str, message: &str, tone: CallbackTone) -> String {
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

/// Send an HTML response to the browser callback stream.
pub fn send_response(stream: &mut impl Write, title: &str, message: &str, tone: CallbackTone) {
    let body = render_callback_html(title, message, tone);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Send a success response (sage green).
pub fn send_success_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Success);
}

/// Send an error response (terracotta).
pub fn send_error_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Error);
}

/// Send an info response (turmeric).
pub fn send_info_response(stream: &mut impl Write, title: &str, message: &str) {
    send_response(stream, title, message, CallbackTone::Info);
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during the OAuth callback listener phase.
#[derive(Debug)]
pub enum CallbackError {
    Io(std::io::Error),
    FlowCancelled,
    Timeout,
}

impl std::fmt::Display for CallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "OAuth callback IO error: {}", e),
            Self::FlowCancelled => write!(f, "OAuth flow cancelled"),
            Self::Timeout => write!(f, "OAuth callback timed out — if your firewall blocked the connection, allow DailyOS in System Settings → Network → Firewall"),
        }
    }
}

impl std::error::Error for CallbackError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_challenge_shape() {
        let verifier = pkce_verifier();
        let challenge = pkce_challenge(&verifier);
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
    fn test_state_is_unique() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
    }
}
