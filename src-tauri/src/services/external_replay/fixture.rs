use std::{collections::HashMap, fs, path::Path};

use base64::Engine;
use serde::Deserialize;
use thiserror::Error;

use super::key::{AuthScopeId, AuthScopeIdError, RequestKey, RequestKeyHexError};
use super::{ExternalReplayFixture, ExternalReplayFixtureMissing, ReplayResponse};

const MAX_FIXTURE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_RESPONSE_BODY_BYTES: u64 = 1024 * 1024;

#[derive(Debug)]
pub struct JsonExternalReplayFixture {
    map: HashMap<RequestKey, ReplayResponse>,
    fixture_path: String,
}

#[derive(Debug, Error)]
pub enum FixtureLoadError {
    #[error("failed to read external replay fixture {fixture_path}: {source}")]
    Read {
        fixture_path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "external replay fixture {fixture_path} is too large: {byte_count} bytes exceeds {cap} bytes"
    )]
    FixtureTooLarge {
        fixture_path: String,
        byte_count: u64,
        cap: u64,
    },
    #[error("failed to parse external replay fixture {fixture_path}: {source}")]
    Parse {
        fixture_path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "external replay fixture {fixture_path} has unsupported version {version}; expected 1"
    )]
    InvalidVersion { fixture_path: String, version: u64 },
    #[error(
        "external replay fixture {fixture_path} has invalid request_key_hex `{request_key_hex}`: {source}"
    )]
    InvalidRequestKeyHex {
        fixture_path: String,
        request_key_hex: String,
        #[source]
        source: RequestKeyHexError,
    },
    #[error(
        "external replay fixture {fixture_path} has invalid auth_scope_id for request `{request_key_hex}`: {source}"
    )]
    InvalidAuthScopeId {
        fixture_path: String,
        request_key_hex: String,
        #[source]
        source: AuthScopeIdError,
    },
    #[error(
        "external replay fixture {fixture_path} has invalid body_base64 for request `{request_key_hex}`: {source}"
    )]
    InvalidBodyBase64 {
        fixture_path: String,
        request_key_hex: String,
        #[source]
        source: base64::DecodeError,
    },
    #[error("external replay fixture {fixture_path} contains duplicate request_key_hex `{request_key_hex}`")]
    DuplicateRequestKey {
        fixture_path: String,
        request_key_hex: String,
    },
}

impl JsonExternalReplayFixture {
    pub fn from_json_file(path: &Path) -> Result<Self, FixtureLoadError> {
        let fixture_path = path.display().to_string();
        let metadata = fs::metadata(path).map_err(|source| FixtureLoadError::Read {
            fixture_path: fixture_path.clone(),
            source,
        })?;
        let byte_count = metadata.len();
        if byte_count > MAX_FIXTURE_BYTES {
            return Err(FixtureLoadError::FixtureTooLarge {
                fixture_path,
                byte_count,
                cap: MAX_FIXTURE_BYTES,
            });
        }

        let content = fs::read_to_string(path).map_err(|source| FixtureLoadError::Read {
            fixture_path: fixture_path.clone(),
            source,
        })?;
        let byte_count = content.len() as u64;
        if byte_count > MAX_FIXTURE_BYTES {
            return Err(FixtureLoadError::FixtureTooLarge {
                fixture_path,
                byte_count,
                cap: MAX_FIXTURE_BYTES,
            });
        }

        let value = serde_json::from_str::<serde_json::Value>(&content).map_err(|source| {
            FixtureLoadError::Parse {
                fixture_path: fixture_path.clone(),
                source,
            }
        })?;
        Self::from_json_value(&value, &fixture_path)
    }

    pub fn from_json_value(
        value: &serde_json::Value,
        fixture_path: &str,
    ) -> Result<Self, FixtureLoadError> {
        let raw_fixture =
            serde_json::from_value::<RawFixtureFile>(value.clone()).map_err(|source| {
                FixtureLoadError::Parse {
                    fixture_path: fixture_path.to_string(),
                    source,
                }
            })?;

        if raw_fixture.version != 1 {
            return Err(FixtureLoadError::InvalidVersion {
                fixture_path: fixture_path.to_string(),
                version: raw_fixture.version,
            });
        }

        let mut map = HashMap::with_capacity(raw_fixture.fixtures.len());
        for fixture in raw_fixture.fixtures {
            let key = RequestKey::from_hex(&fixture.request_key_hex).map_err(|source| {
                FixtureLoadError::InvalidRequestKeyHex {
                    fixture_path: fixture_path.to_string(),
                    request_key_hex: fixture.request_key_hex.clone(),
                    source,
                }
            })?;
            AuthScopeId::try_new(&fixture.auth_scope_id).map_err(|source| {
                FixtureLoadError::InvalidAuthScopeId {
                    fixture_path: fixture_path.to_string(),
                    request_key_hex: fixture.request_key_hex.clone(),
                    source,
                }
            })?;
            let body = base64::engine::general_purpose::STANDARD
                .decode(&fixture.response.body_base64)
                .map_err(|source| FixtureLoadError::InvalidBodyBase64 {
                    fixture_path: fixture_path.to_string(),
                    request_key_hex: fixture.request_key_hex.clone(),
                    source,
                })?;
            let byte_count = body.len() as u64;
            if byte_count > MAX_RESPONSE_BODY_BYTES {
                return Err(FixtureLoadError::FixtureTooLarge {
                    fixture_path: fixture_path.to_string(),
                    byte_count,
                    cap: MAX_RESPONSE_BODY_BYTES,
                });
            }
            let response = ReplayResponse {
                status: fixture.response.status,
                headers: fixture.response.headers,
                body,
            };

            if map.insert(key, response).is_some() {
                return Err(FixtureLoadError::DuplicateRequestKey {
                    fixture_path: fixture_path.to_string(),
                    request_key_hex: fixture.request_key_hex,
                });
            }
        }

        Ok(Self {
            map,
            fixture_path: fixture_path.to_string(),
        })
    }

    pub fn fixture_path(&self) -> &str {
        &self.fixture_path
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl ExternalReplayFixture for JsonExternalReplayFixture {
    fn lookup(
        &self,
        key: &RequestKey,
        method: &str,
        url: &str,
    ) -> Result<ReplayResponse, ExternalReplayFixtureMissing> {
        self.map
            .get(key)
            .cloned()
            .ok_or_else(|| ExternalReplayFixtureMissing::new(key, method, url))
    }
}

#[derive(Debug, Deserialize)]
struct RawFixtureFile {
    version: u64,
    fixtures: Vec<RawFixture>,
}

#[derive(Debug, Deserialize)]
struct RawFixture {
    request_key_hex: String,
    #[serde(default)]
    auth_scope_id: String,
    response: RawFixtureResponse,
}

#[derive(Debug, Deserialize)]
struct RawFixtureResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body_base64: String,
}
