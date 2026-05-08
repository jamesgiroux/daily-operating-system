use http::HeaderMap;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

/// Non-empty tenant/auth boundary used when deriving replay fixture keys.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AuthScopeId(String);

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum AuthScopeIdError {
    #[error("auth_scope_id must not be empty")]
    Empty,
    #[error("auth_scope_id must not contain whitespace")]
    ContainsWhitespace,
    #[error("auth_scope_id value is reserved")]
    Reserved,
}

/// Deterministic SHA-256 key for an external replay fixture request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RequestKey([u8; 32]);

#[derive(Debug, Error)]
pub enum RequestKeyHexError {
    #[error("request key hex must decode to 32 bytes")]
    InvalidLength,
    #[error("invalid request key hex: {source}")]
    InvalidHex {
        #[from]
        source: hex::FromHexError,
    },
}

impl AuthScopeId {
    pub fn try_new(value: &str) -> Result<Self, AuthScopeIdError> {
        if value.is_empty() {
            return Err(AuthScopeIdError::Empty);
        }

        if value.chars().any(char::is_whitespace) {
            return Err(AuthScopeIdError::ContainsWhitespace);
        }

        let lowercase = value.to_ascii_lowercase();
        if matches!(
            lowercase.as_str(),
            "default"
                | "evaluate"
                | "live"
                | "none"
                | "null"
                | "prod"
                | "production"
                | "replay"
                | "simulate"
                | "undefined"
        ) {
            return Err(AuthScopeIdError::Reserved);
        }

        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AuthScopeId {
    type Error = AuthScopeIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl TryFrom<String> for AuthScopeId {
    type Error = AuthScopeIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(&value)
    }
}

impl AsRef<str> for AuthScopeId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl RequestKey {
    pub fn canonicalize(
        method: &str,
        url: &str,
        headers: &HeaderMap,
        body: &[u8],
        auth_scope_id: &AuthScopeId,
    ) -> Self {
        let method = method.to_ascii_uppercase();
        let url = canonicalize_url(url);
        let headers = canonicalize_headers(headers);
        let auth_scope = format!("auth:{}:", auth_scope_id.as_str());

        let mut hasher = Sha256::new();
        update_component(&mut hasher, b"method", method.as_bytes());
        update_component(&mut hasher, b"url", url.as_bytes());
        update_component(&mut hasher, b"headers", &headers);
        update_component(&mut hasher, b"body", body);
        update_component(&mut hasher, b"auth", auth_scope.as_bytes());

        let digest = hasher.finalize();
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&digest);
        Self(bytes)
    }

    pub fn try_canonicalize(
        method: &str,
        url: &str,
        headers: &HeaderMap,
        body: &[u8],
        auth_scope_id: &str,
    ) -> Result<Self, AuthScopeIdError> {
        let auth_scope_id = AuthScopeId::try_new(auth_scope_id)?;
        Ok(Self::canonicalize(
            method,
            url,
            headers,
            body,
            &auth_scope_id,
        ))
    }

    pub fn from_hex(value: &str) -> Result<Self, RequestKeyHexError> {
        let decoded = hex::decode(value)?;
        let bytes: [u8; 32] = decoded
            .try_into()
            .map_err(|_| RequestKeyHexError::InvalidLength)?;
        Ok(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

pub(crate) fn redact_url(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return "<redacted>".to_string();
    };

    let Some(host) = parsed.host_str() else {
        return "<redacted>".to_string();
    };

    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let port = parsed
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();

    format!("{}://{}{}{}", parsed.scheme(), host, port, "/<redacted>")
}

fn update_component(hasher: &mut Sha256, label: &[u8], value: &[u8]) {
    hasher.update([0]);
    hasher.update(label);
    hasher.update([1]);
    hasher.update(value.len().to_be_bytes());
    hasher.update([1]);
    hasher.update(value);
}

fn canonicalize_url(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else {
        return canonicalize_unparsed_url(url);
    };

    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = parsed.set_username("");
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = parsed.set_password(None);
    parsed.set_fragment(None);

    if parsed.query().is_some() {
        let mut pairs = parsed
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        pairs.sort_by(|(left_key, left_value), (right_key, right_value)| {
            left_key
                .cmp(right_key)
                .then_with(|| left_value.cmp(right_value))
        });

        parsed.set_query(None);
        if !pairs.is_empty() {
            let mut query_pairs = parsed.query_pairs_mut();
            for (key, value) in pairs {
                query_pairs.append_pair(&key, &value);
            }
        }
    }

    parsed.to_string()
}

fn canonicalize_unparsed_url(url: &str) -> String {
    let without_fragment = url.split_once('#').map_or(url, |(prefix, _)| prefix);
    let Some((base, query)) = without_fragment.split_once('?') else {
        return without_fragment.to_string();
    };

    let mut pairs = query
        .split('&')
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key, value)
        })
        .collect::<Vec<_>>();
    pairs.sort_by(|(left_key, left_value), (right_key, right_value)| {
        left_key
            .cmp(right_key)
            .then_with(|| left_value.cmp(right_value))
    });

    if pairs.is_empty() {
        return base.to_string();
    }

    let query = pairs
        .into_iter()
        .map(|(key, value)| {
            if value.is_empty() {
                key.to_string()
            } else {
                format!("{key}={value}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");

    format!("{base}?{query}")
}

fn canonicalize_headers(headers: &HeaderMap) -> Vec<u8> {
    let mut canonical = headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.as_str().to_ascii_lowercase();
            matches!(
                name.as_str(),
                "accept" | "content-encoding" | "content-type" | "user-agent"
            )
            .then(|| (name, value.as_bytes().to_vec()))
        })
        .collect::<Vec<_>>();

    canonical.sort_by(|(left_name, left_value), (right_name, right_value)| {
        left_name
            .cmp(right_name)
            .then_with(|| left_value.cmp(right_value))
    });

    let mut bytes = Vec::new();
    for (name, value) in canonical {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&value);
        bytes.push(1);
    }
    bytes
}
