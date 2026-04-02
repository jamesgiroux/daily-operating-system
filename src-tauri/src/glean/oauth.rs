//! Glean OAuth2 consent flow via MCP OAuth discovery + Dynamic Client Registration.
//!
//! 1. Probe the MCP endpoint → 401 → read `WWW-Authenticate` for resource_metadata URL
//! 2. Fetch Protected Resource Metadata → get `authorization_servers[0]`
//! 3. Fetch AS metadata from `/.well-known/oauth-authorization-server`
//!    (fall back to `/.well-known/openid-configuration`)
//! 4. If `registration_endpoint` exists, POST DCR to get `client_id`
//! 5. Open browser for authorization (with PKCE + state + `resource` param)
//! 6. Listen for localhost callback
//! 7. Exchange code for tokens (with `resource` param)
//! 8. Fetch user info (if userinfo_endpoint available)
//! 9. Save tokens to Keychain

use std::net::TcpListener;

use serde::{Deserialize, Serialize};

use crate::oauth;

use super::token_store::{self, GleanToken};
use super::GleanAuthError;

/// MCP OAuth endpoints discovered via the two-step MCP discovery flow.
#[derive(Debug, Clone)]
pub struct McpOAuthEndpoints {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: Option<String>,
    pub userinfo_endpoint: Option<String>,
    /// The MCP server's canonical URI (for RFC 8707 `resource` parameter).
    pub resource: String,
}

/// Dynamic Client Registration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DcrResponse {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
}

/// Result of a successful Glean auth flow.
#[derive(Debug, Clone)]
pub struct GleanAuthResult {
    pub email: Option<String>,
    pub name: Option<String>,
}

/// Derive the base URL from a Glean MCP endpoint.
///
/// Strips trailing path components like `/mcp/default` to get the instance root.
/// E.g. `https://company-be.glean.com/mcp/default` → `https://company-be.glean.com`
fn derive_base_url(instance_url: &str) -> Result<String, GleanAuthError> {
    let url = url::Url::parse(instance_url)
        .map_err(|e| GleanAuthError::Other(format!("Invalid Glean URL: {}", e)))?;

    Ok(format!(
        "{}://{}",
        url.scheme(),
        url.host_str()
            .ok_or_else(|| GleanAuthError::Other("Glean URL has no host".into()))?
    ))
}

/// Canonicalize a URL for use as the RFC 8707 `resource` parameter.
///
/// Strips query string and fragment, ensures no trailing slash on path.
fn canonicalize_resource(url: &str) -> Result<String, GleanAuthError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| GleanAuthError::Other(format!("Invalid resource URL: {}", e)))?;
    let path = parsed.path().trim_end_matches('/');
    Ok(format!(
        "{}://{}{}",
        parsed.scheme(),
        parsed
            .host_str()
            .ok_or_else(|| GleanAuthError::Other("Resource URL has no host".into()))?,
        if path.is_empty() { "" } else { path }
    ))
}

// -------------------------------------------------------------------------
// Discovery structs
// -------------------------------------------------------------------------

/// Protected Resource Metadata (RFC 9728 / MCP spec).
#[derive(Debug, Deserialize)]
struct ProtectedResourceMetadata {
    #[serde(default)]
    authorization_servers: Vec<String>,
    #[serde(default)]
    resource: Option<String>,
}

/// OAuth Authorization Server metadata (RFC 8414).
#[derive(Debug, Deserialize)]
struct AsMetadata {
    authorization_endpoint: Option<String>,
    token_endpoint: Option<String>,
    registration_endpoint: Option<String>,
    userinfo_endpoint: Option<String>,
}

/// Truncate a string for log output (first 200 chars).
fn truncate_log(s: &str) -> &str {
    if s.len() > 200 {
        &s[..200]
    } else {
        s
    }
}

// -------------------------------------------------------------------------
// MCP OAuth discovery
// -------------------------------------------------------------------------

/// Discover MCP OAuth endpoints using the MCP spec two-step flow.
///
/// 1. Probe the MCP endpoint → expect 401 → read `WWW-Authenticate` for `resource_metadata`
/// 2. Fetch Protected Resource Metadata → extract `authorization_servers[0]` and `resource`
/// 3. Fetch AS metadata from `{as}/.well-known/oauth-authorization-server`
///    or fall back to `{as}/.well-known/openid-configuration`
pub async fn discover_mcp_oauth(mcp_endpoint: &str) -> Result<McpOAuthEndpoints, GleanAuthError> {
    // No-redirect client for the probe (we need to see the 401 + WWW-Authenticate)
    let probe_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| GleanAuthError::Other(e.to_string()))?;

    // Normal client that follows redirects for metadata fetches
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| GleanAuthError::Other(e.to_string()))?;

    let base = derive_base_url(mcp_endpoint)?;
    let resource = canonicalize_resource(mcp_endpoint)?;

    // --- Step 1: Probe MCP endpoint for resource_metadata hint ---
    let resource_metadata_url =
        probe_for_resource_metadata(&probe_client, mcp_endpoint, &base).await?;

    // --- Step 2: Fetch Protected Resource Metadata ---
    log::info!(
        "Glean MCP: fetching resource metadata from {}",
        resource_metadata_url
    );
    let prm = fetch_protected_resource_metadata(&client, &resource_metadata_url).await;

    let (as_url, effective_resource) = match prm {
        Ok(ref meta) => {
            let res = meta.resource.clone().unwrap_or_else(|| resource.clone());
            let as_server = meta.authorization_servers.first().cloned();
            log::info!(
                "Glean MCP: resource metadata returned authorization_servers={:?}, resource={:?}",
                meta.authorization_servers,
                meta.resource,
            );
            (as_server, res)
        }
        Err(e) => {
            log::warn!(
                "Glean MCP: resource metadata failed ({}), will try OIDC on base URL",
                e
            );
            (None, resource.clone())
        }
    };

    // --- Step 3: Fetch AS metadata ---
    // Try AS URL from resource metadata first, then fall back to base URL
    let as_meta = if let Some(ref discovered_as) = as_url {
        log::info!(
            "Glean MCP: trying AS metadata on discovered server: {}",
            discovered_as
        );
        match fetch_as_metadata(&client, discovered_as).await {
            Ok(meta) => meta,
            Err(e) if *discovered_as != base => {
                log::warn!(
                    "Glean MCP: AS metadata failed on {} ({}), retrying on base {}",
                    discovered_as,
                    e,
                    base
                );
                fetch_as_metadata(&client, &base).await?
            }
            Err(e) => return Err(e),
        }
    } else {
        log::info!(
            "Glean MCP: no authorization_servers discovered, trying OIDC on base URL: {}",
            base
        );
        fetch_as_metadata(&client, &base).await?
    };

    let authorization_endpoint = as_meta.authorization_endpoint.ok_or_else(|| {
        GleanAuthError::Discovery("No authorization_endpoint in AS metadata".into())
    })?;
    let token_endpoint = as_meta
        .token_endpoint
        .ok_or_else(|| GleanAuthError::Discovery("No token_endpoint in AS metadata".into()))?;

    Ok(McpOAuthEndpoints {
        authorization_endpoint,
        token_endpoint,
        registration_endpoint: as_meta.registration_endpoint,
        userinfo_endpoint: as_meta.userinfo_endpoint,
        resource: effective_resource,
    })
}

/// Probe the MCP endpoint to discover the `resource_metadata` URL.
///
/// Sends a GET to the MCP endpoint. If we get a 401, parse the `WWW-Authenticate`
/// header for a `resource_metadata` URL. Falls back to the well-known path.
async fn probe_for_resource_metadata(
    client: &reqwest::Client,
    mcp_endpoint: &str,
    base_url: &str,
) -> Result<String, GleanAuthError> {
    log::info!("Glean MCP: probing {} for OAuth metadata", mcp_endpoint);

    match client.get(mcp_endpoint).send().await {
        Ok(resp) => {
            let status = resp.status();
            log::info!("Glean MCP probe: status {}", status);

            if status == reqwest::StatusCode::UNAUTHORIZED {
                // Parse WWW-Authenticate header for resource_metadata URL
                if let Some(www_auth) = resp.headers().get("www-authenticate") {
                    if let Ok(header_str) = www_auth.to_str() {
                        if let Some(url) = parse_resource_metadata_url(header_str) {
                            log::info!(
                                "Glean MCP: found resource_metadata in WWW-Authenticate: {}",
                                url
                            );
                            return Ok(url);
                        }
                    }
                }
            }
            // Fall through to well-known path
        }
        Err(e) => {
            log::warn!(
                "Glean MCP: probe request failed ({}), falling back to well-known",
                e
            );
        }
    }

    // Fallback: well-known path
    Ok(format!("{}/.well-known/oauth-protected-resource", base_url))
}

/// Parse `resource_metadata` URL from a WWW-Authenticate header.
///
/// Looks for: `Bearer resource_metadata="https://..."` or unquoted form.
fn parse_resource_metadata_url(header: &str) -> Option<String> {
    // Look for resource_metadata="..." or resource_metadata=...
    let lower = header.to_lowercase();
    let prefix = "resource_metadata=";
    let idx = lower.find(prefix)?;
    let start = idx + prefix.len();
    let rest = &header[start..];

    if let Some(stripped) = rest.strip_prefix('"') {
        // Quoted value
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        // Unquoted — take until whitespace or comma
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',')
            .unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

/// Check if a response content-type looks like JSON.
fn is_json_content_type(resp: &reqwest::Response) -> bool {
    resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("json"))
        .unwrap_or(false)
}

/// Fetch Protected Resource Metadata from the given URL.
async fn fetch_protected_resource_metadata(
    client: &reqwest::Client,
    url: &str,
) -> Result<ProtectedResourceMetadata, GleanAuthError> {
    let resp = client.get(url).send().await.map_err(|e| {
        GleanAuthError::Discovery(format!("Resource metadata request failed: {}", e))
    })?;

    let status = resp.status();
    let content_is_json = is_json_content_type(&resp);
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        log::error!(
            "Resource metadata returned {} body={}",
            status,
            truncate_log(&body_text)
        );
        return Err(GleanAuthError::Discovery(format!(
            "Resource metadata returned {}",
            status
        )));
    }

    if !content_is_json && body_text.trim_start().starts_with('<') {
        return Err(GleanAuthError::Discovery(
            "Resource metadata returned HTML instead of JSON — endpoint likely doesn't exist"
                .into(),
        ));
    }

    log::debug!("Resource metadata body: {}", truncate_log(&body_text));
    serde_json::from_str(&body_text).map_err(|e| {
        GleanAuthError::Discovery(format!(
            "Failed to parse resource metadata: {} (body starts with: {})",
            e,
            truncate_log(&body_text),
        ))
    })
}

/// Fetch OAuth Authorization Server metadata.
///
/// Tries `/.well-known/oauth-authorization-server` first,
/// falls back to `/.well-known/openid-configuration`.
async fn fetch_as_metadata(
    client: &reqwest::Client,
    as_url: &str,
) -> Result<AsMetadata, GleanAuthError> {
    let as_base = as_url.trim_end_matches('/');

    // Try OAuth AS metadata first
    let oauth_as_url = format!("{}/.well-known/oauth-authorization-server", as_base);
    log::info!("Glean MCP: trying AS metadata at {}", oauth_as_url);

    if let Ok(resp) = client.get(&oauth_as_url).send().await {
        let status = resp.status();
        let content_is_json = is_json_content_type(&resp);
        if status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            if content_is_json || !body_text.trim_start().starts_with('<') {
                log::debug!(
                    "AS metadata (oauth-authorization-server) body: {}",
                    truncate_log(&body_text)
                );
                match serde_json::from_str::<AsMetadata>(&body_text) {
                    Ok(meta)
                        if meta.authorization_endpoint.is_some()
                            && meta.token_endpoint.is_some() =>
                    {
                        log::info!("Glean MCP: found AS metadata via oauth-authorization-server");
                        return Ok(meta);
                    }
                    Ok(_) => {
                        log::warn!("Glean MCP: oauth-authorization-server response missing required endpoints");
                    }
                    Err(e) => {
                        log::warn!(
                            "Glean MCP: oauth-authorization-server parse failed: {} (body starts with: {})",
                            e,
                            truncate_log(&body_text),
                        );
                    }
                }
            } else {
                log::info!(
                    "Glean MCP: oauth-authorization-server returned HTML (SPA catch-all), skipping"
                );
            }
        } else {
            log::info!("Glean MCP: oauth-authorization-server returned {}", status);
        }
    }

    // Fallback: OIDC discovery
    let oidc_url = format!("{}/.well-known/openid-configuration", as_base);
    log::info!("Glean MCP: falling back to OIDC discovery at {}", oidc_url);

    let resp =
        client.get(&oidc_url).send().await.map_err(|e| {
            GleanAuthError::Discovery(format!("OIDC discovery request failed: {}", e))
        })?;

    let status = resp.status();
    let content_is_json = is_json_content_type(&resp);
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        log::error!(
            "OIDC discovery returned {} body={}",
            status,
            truncate_log(&body_text)
        );
        return Err(GleanAuthError::Discovery(format!(
            "AS metadata returned {} (tried both oauth-authorization-server and openid-configuration on {})",
            status, as_base,
        )));
    }

    if !content_is_json && body_text.trim_start().starts_with('<') {
        log::error!(
            "OIDC discovery on {} returned HTML instead of JSON",
            as_base
        );
        return Err(GleanAuthError::Discovery(format!(
            "No OAuth metadata found on {} — both well-known endpoints returned HTML. \
             The authorization server may be a different host (check resource metadata).",
            as_base,
        )));
    }

    log::debug!(
        "AS metadata (openid-configuration) body: {}",
        truncate_log(&body_text)
    );
    serde_json::from_str(&body_text).map_err(|e| {
        GleanAuthError::Discovery(format!(
            "Failed to parse AS metadata: {} (body starts with: {})",
            e,
            truncate_log(&body_text),
        ))
    })
}

// -------------------------------------------------------------------------
// Dynamic Client Registration
// -------------------------------------------------------------------------

/// Register a client dynamically with the authorization server (RFC 7591).
pub async fn register_client(
    registration_endpoint: &str,
    redirect_uri: &str,
) -> Result<DcrResponse, GleanAuthError> {
    log::info!(
        "Glean MCP: registering client via DCR at {}",
        registration_endpoint
    );

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "client_name": "DailyOS",
        "redirect_uris": [redirect_uri],
        "grant_types": ["authorization_code"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none"
    });

    let resp = client
        .post(registration_endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| GleanAuthError::Discovery(format!("DCR request failed: {}", e)))?;

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        log::error!("Glean DCR failed: status={} body={}", status, body_text);
        return Err(GleanAuthError::Discovery(format!(
            "Dynamic client registration failed ({}): {}",
            status, body_text
        )));
    }

    serde_json::from_str::<DcrResponse>(&body_text)
        .map_err(|e| GleanAuthError::Discovery(format!("Failed to parse DCR response: {}", e)))
}

// -------------------------------------------------------------------------
// Consent flow
// -------------------------------------------------------------------------

/// Run the full Glean OAuth consent flow.
///
/// Uses MCP OAuth discovery + DCR. Opens the user's browser for Glean SSO,
/// captures the redirect, exchanges the code for tokens, and saves to Keychain.
pub async fn run_glean_consent_flow(instance_url: &str) -> Result<GleanAuthResult, GleanAuthError> {
    // 1. MCP OAuth discovery
    let endpoints = discover_mcp_oauth(instance_url).await?;

    // 2. PKCE + state
    let pkce_verifier = oauth::pkce_verifier();
    let pkce_challenge = oauth::pkce_challenge(&pkce_verifier);
    let oauth_state = oauth::generate_state();

    // 3. Bind localhost listener
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| GleanAuthError::Other(format!("Failed to bind listener: {}", e)))?;
    let port = listener
        .local_addr()
        .map_err(|e| GleanAuthError::Other(format!("Failed to get listener port: {}", e)))?
        .port();
    let redirect_uri = format!("http://localhost:{}", port);

    // 4. DCR — register client dynamically
    let dcr = if let Some(ref reg_url) = endpoints.registration_endpoint {
        register_client(reg_url, &redirect_uri).await?
    } else {
        return Err(GleanAuthError::Discovery(
            "No registration_endpoint in AS metadata — cannot register client dynamically".into(),
        ));
    };

    log::info!("Glean DCR: obtained client_id={}", dcr.client_id);

    // 5. Build auth URL (includes resource param per RFC 8707)
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&code_challenge={}&code_challenge_method=S256&state={}&resource={}",
        endpoints.authorization_endpoint,
        oauth::urlencode(&dcr.client_id),
        oauth::urlencode(&redirect_uri),
        // Only request scopes we actually use. Requesting scopes a user doesn't
        // have (e.g. agents, entities) can cause consent failures for non-admin users.
        oauth::urlencode("openid profile email mcp search chat offline_access"),
        oauth::urlencode(&pkce_challenge),
        oauth::urlencode(&oauth_state),
        oauth::urlencode(&endpoints.resource),
    );

    // 6. Open browser
    log::info!("Opening browser for Glean OAuth consent...");
    if let Err(e) = open::that(&auth_url) {
        log::warn!("Failed to open browser: {}. URL: {}", e, auth_url);
    }

    // 7. Wait for callback (120s timeout in listen_for_callback guards
    // against macOS firewall blocking the browser's localhost redirect).
    let oauth::CallbackResult {
        callback,
        mut stream,
    } = oauth::listen_for_callback(&listener).map_err(|e| match e {
        oauth::CallbackError::Io(io_err) => {
            GleanAuthError::Other(format!("Callback IO error: {}", io_err))
        }
        oauth::CallbackError::FlowCancelled => GleanAuthError::FlowCancelled,
        oauth::CallbackError::Timeout => GleanAuthError::Other(
            "Authorization timed out. If your firewall blocked the connection, allow DailyOS in System Settings → Network → Firewall, then try again.".into(),
        ),
    })?;

    // Validate state
    if callback.state.as_deref() != Some(oauth_state.as_str()) {
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            "State mismatch detected. Please return to DailyOS and try connecting again.",
        );
        return Err(GleanAuthError::StateMismatch);
    }

    // 8. Exchange code for tokens (includes resource param)
    log::info!("Glean OAuth: exchanging auth code for tokens");
    let client = reqwest::Client::new();

    let mut form_params = vec![
        ("code", callback.code.as_str()),
        ("client_id", dcr.client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
        ("code_verifier", pkce_verifier.as_str()),
        ("resource", endpoints.resource.as_str()),
    ];
    // If DCR returned a client_secret, include it
    let secret_ref;
    if let Some(ref secret) = dcr.client_secret {
        secret_ref = secret.clone();
        form_params.push(("client_secret", &secret_ref));
    }

    let token_resp = client
        .post(&endpoints.token_endpoint)
        .form(&form_params)
        .send()
        .await
        .map_err(|e| {
            oauth::send_error_response(
                &mut stream,
                "Authorization failed",
                "Could not reach Glean during token exchange. Please return to DailyOS and try again.",
            );
            GleanAuthError::TokenExchange(format!("Token exchange request failed: {}", e))
        })?;

    let status = token_resp.status();
    let body_text = token_resp.text().await.unwrap_or_default();

    if !status.is_success() {
        log::error!(
            "Glean OAuth: token exchange failed: status={} body={}",
            status,
            body_text
        );
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            &format!(
                "Glean returned {} during authorization. Please return to DailyOS and try again.",
                status
            ),
        );
        return Err(GleanAuthError::TokenExchange(format!(
            "Token exchange failed ({}): {}",
            status, body_text
        )));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
        GleanAuthError::TokenExchange(format!("Failed to parse token response: {}", e))
    })?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or_else(|| GleanAuthError::TokenExchange("No access_token in response".into()))?
        .to_string();
    let refresh_token = body["refresh_token"].as_str().map(|s| s.to_string());
    let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
    let expiry = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);

    // 9. Fetch user info (if available)
    let (email, name) = if let Some(ref userinfo_url) = endpoints.userinfo_endpoint {
        fetch_userinfo(&client, userinfo_url, &access_token).await
    } else {
        // Fallback: extract display-only claims from id_token (unverified).
        // SAFETY: email/name are used only for UI labels, not authorization.
        // See extract_from_id_token doc comment for full trust boundary rationale.
        extract_from_id_token(body.get("id_token"))
    };

    log::info!(
        "Glean OAuth: authenticated as {}",
        email.as_deref().unwrap_or("unknown")
    );

    // 10. Save to Keychain
    let token = GleanToken {
        access_token,
        refresh_token,
        token_endpoint: endpoints.token_endpoint,
        client_id: dcr.client_id,
        client_secret: dcr.client_secret,
        expiry: Some(expiry.to_rfc3339()),
        email: email.clone(),
        name: name.clone(),
    };

    if let Err(e) = token_store::save_token(&token) {
        log::error!("Glean OAuth: failed to save token: {}", e);
        oauth::send_error_response(
            &mut stream,
            "Authorization failed",
            "Credentials could not be saved. Please return to DailyOS and check logs.",
        );
        return Err(e);
    }

    // Success!
    oauth::send_success_response(
        &mut stream,
        "Glean account connected",
        "You can close this tab and return to DailyOS. Settings will update automatically.",
    );
    log::info!("Glean OAuth: flow complete, token saved");

    Ok(GleanAuthResult { email, name })
}

/// Fetch user information from the OIDC userinfo endpoint.
async fn fetch_userinfo(
    client: &reqwest::Client,
    userinfo_url: &str,
    access_token: &str,
) -> (Option<String>, Option<String>) {
    match client
        .get(userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let email = body["email"].as_str().map(|s| s.to_string());
                let name = body["name"].as_str().map(|s| s.to_string());
                return (email, name);
            }
        }
        Ok(resp) => {
            log::warn!("Glean userinfo returned {}", resp.status());
        }
        Err(e) => {
            log::warn!("Glean userinfo fetch failed: {}", e);
        }
    }
    (None, None)
}

/// Extract email and name from a JWT id_token **without cryptographic signature
/// verification**. The returned values are used exclusively for display purposes
/// (UI status labels and log messages) — never for authorization decisions.
///
/// # Trust boundary
///
/// This function decodes the JWT payload via base64 without verifying the
/// signature against the issuer's JWKS. This is acceptable here because:
///
/// 1. **The token was received over a direct TLS connection** from the Glean
///    authorization server's token endpoint, authenticated via PKCE + state
///    parameter. A network attacker who could tamper with this response could
///    also substitute the access_token itself, so signature verification on
///    the id_token alone would not raise the security bar.
///
/// 2. **The extracted claims are display-only.** The email and name are stored
///    in the Keychain token blob and surfaced in `GleanAuthStatus::Authenticated`
///    for the Settings UI. They are not used for access control, entitlement
///    checks, or any authorization decision. The actual Glean API authorization
///    is performed server-side using the access_token.
///
/// 3. **`fetch_userinfo` is preferred when available.** This function is only
///    the fallback path when the authorization server metadata does not advertise
///    a `userinfo_endpoint`.
///
/// If the email is ever needed for authorization (e.g., entitlement gating),
/// this function MUST be replaced with full OIDC id_token validation:
/// fetch the issuer's JWKS via the `jwks_uri` from AS metadata and verify the
/// JWT signature, issuer, audience, and expiry before trusting any claims.
fn extract_from_id_token(
    id_token_value: Option<&serde_json::Value>,
) -> (Option<String>, Option<String>) {
    let token_str = id_token_value.and_then(|v| v.as_str());
    let Some(token_str) = token_str else {
        return (None, None);
    };

    // JWT format: header.payload.signature — decode the payload (unverified).
    let parts: Vec<&str> = token_str.split('.').collect();
    if parts.len() != 3 {
        return (None, None);
    }

    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| {
            // Some JWTs use standard base64 with padding
            base64::engine::general_purpose::STANDARD.decode(parts[1])
        })
        .ok();

    let Some(payload) = payload else {
        return (None, None);
    };

    let Ok(claims) = serde_json::from_slice::<serde_json::Value>(&payload) else {
        return (None, None);
    };

    let email = claims["email"].as_str().map(|s| s.to_string());
    let name = claims["name"].as_str().map(|s| s.to_string());
    (email, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_base_url() {
        assert_eq!(
            derive_base_url("https://company-be.glean.com/mcp/default").unwrap(),
            "https://company-be.glean.com"
        );
        assert_eq!(
            derive_base_url("https://example.glean.com").unwrap(),
            "https://example.glean.com"
        );
        assert_eq!(
            derive_base_url("https://foo.glean.com/some/path").unwrap(),
            "https://foo.glean.com"
        );
    }

    #[test]
    fn test_canonicalize_resource() {
        assert_eq!(
            canonicalize_resource("https://company-be.glean.com/mcp/default").unwrap(),
            "https://company-be.glean.com/mcp/default"
        );
        assert_eq!(
            canonicalize_resource("https://company-be.glean.com/mcp/default/").unwrap(),
            "https://company-be.glean.com/mcp/default"
        );
        assert_eq!(
            canonicalize_resource("https://company-be.glean.com/").unwrap(),
            "https://company-be.glean.com"
        );
    }

    #[test]
    fn test_parse_resource_metadata_url_quoted() {
        let header = r#"Bearer resource_metadata="https://example.com/.well-known/oauth-protected-resource""#;
        assert_eq!(
            parse_resource_metadata_url(header),
            Some("https://example.com/.well-known/oauth-protected-resource".into())
        );
    }

    #[test]
    fn test_parse_resource_metadata_url_unquoted() {
        let header =
            "Bearer resource_metadata=https://example.com/.well-known/oauth-protected-resource";
        assert_eq!(
            parse_resource_metadata_url(header),
            Some("https://example.com/.well-known/oauth-protected-resource".into())
        );
    }

    #[test]
    fn test_parse_resource_metadata_url_with_other_params() {
        let header = r#"Bearer realm="example", resource_metadata="https://example.com/rm", error="invalid_token""#;
        assert_eq!(
            parse_resource_metadata_url(header),
            Some("https://example.com/rm".into())
        );
    }

    #[test]
    fn test_parse_resource_metadata_url_missing() {
        let header = "Bearer realm=\"example\"";
        assert_eq!(parse_resource_metadata_url(header), None);
    }

    #[test]
    fn test_extract_from_id_token_none() {
        let (email, name) = extract_from_id_token(None);
        assert!(email.is_none());
        assert!(name.is_none());
    }

    #[test]
    fn test_dcr_response_deserialize() {
        let json = r#"{"client_id": "abc123"}"#;
        let dcr: DcrResponse = serde_json::from_str(json).unwrap();
        assert_eq!(dcr.client_id, "abc123");
        assert!(dcr.client_secret.is_none());

        let json_with_secret = r#"{"client_id": "abc123", "client_secret": "sec456"}"#;
        let dcr: DcrResponse = serde_json::from_str(json_with_secret).unwrap();
        assert_eq!(dcr.client_id, "abc123");
        assert_eq!(dcr.client_secret.as_deref(), Some("sec456"));
    }
}
