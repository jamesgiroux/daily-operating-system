use http::HeaderMap;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

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

impl RequestKey {
    pub fn canonicalize(
        method: &str,
        url: &str,
        headers: &HeaderMap,
        body: &[u8],
        auth_scope_id: &str,
    ) -> Self {
        let method = method.to_ascii_uppercase();
        let url = canonicalize_url(url);
        let headers = canonicalize_headers(headers);
        let auth_scope = format!("auth:{auth_scope_id}:");

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
    let without_fragment = url.split_once('#').map_or(url, |(prefix, _)| prefix);
    without_fragment
        .split_once('?')
        .map_or(without_fragment, |(prefix, _)| prefix)
        .to_string()
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
                "accept" | "content-type" | "user-agent"
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
