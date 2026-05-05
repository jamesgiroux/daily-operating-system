//! Fixture-backed replay for external HTTP clients.
//!
//! JSON fixtures are keyed by the canonical request hash and contain owned
//! responses:
//!
//! ```json
//! {
//!   "version": 1,
//!   "fixtures": [
//!     {
//!       "request_key_hex": "abc123...",
//!       "method": "GET",
//!       "url": "https://example.com/api/foo",
//!       "auth_scope_id": "test-tenant-1",
//!       "response": {
//!         "status": 200,
//!         "headers": [["Content-Type", "application/json"]],
//!         "body_base64": "..."
//!       }
//!     }
//!   ]
//! }
//! ```

mod key;
mod fixture;

pub use fixture::{FixtureLoadError, JsonExternalReplayFixture};
pub use key::{AuthScopeId, AuthScopeIdError, RequestKey, RequestKeyHexError};

use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("external replay fixture missing for request: {request_key_hex}")]
pub struct ExternalReplayFixtureMissing {
    pub request_key_hex: String,
    pub method: String,
    pub url_redacted: String,
}

impl ExternalReplayFixtureMissing {
    pub fn new(key: &RequestKey, method: &str, url: &str) -> Self {
        Self {
            request_key_hex: key.to_hex(),
            method: method.to_ascii_uppercase(),
            url_redacted: key::redact_url(url),
        }
    }

}

pub trait ExternalReplayFixture: Send + Sync {
    fn lookup(
        &self,
        key: &RequestKey,
        method: &str,
        url: &str,
    ) -> Result<ReplayResponse, ExternalReplayFixtureMissing>;
}

#[cfg(test)]
mod tests {
    use std::fs;

    use base64::Engine;
    use http::{HeaderMap, HeaderName, HeaderValue};
    use serde_json::json;

    use super::{
        AuthScopeId, AuthScopeIdError, ExternalReplayFixture, ExternalReplayFixtureMissing, FixtureLoadError,
        JsonExternalReplayFixture, RequestKey, ReplayResponse,
    };

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    fn request_key(body: &[u8], auth_scope_id: &str) -> RequestKey {
        let auth_scope_id = AuthScopeId::try_new(auth_scope_id).unwrap();
        RequestKey::canonicalize(
            "GET",
            "https://example.com/api/foo?a=1&b=2",
            &headers(&[
                ("User-Agent", "dailyos-test"),
                ("Content-Type", "application/json"),
                ("Accept", "application/json"),
            ]),
            body,
            &auth_scope_id,
        )
    }

    fn response_body_base64(body: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(body)
    }

    #[test]
    fn request_key_canonicalization_is_deterministic() {
        let expected = request_key(br#"{"query":"alpha"}"#, "test-tenant-1");

        for _ in 0..100 {
            assert_eq!(
                expected,
                request_key(br#"{"query":"alpha"}"#, "test-tenant-1")
            );
        }
    }

    #[test]
    fn request_key_strips_volatile_headers() {
        let stable_headers = headers(&[
            ("User-Agent", "dailyos-test"),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
        ]);
        let volatile_headers = headers(&[
            ("User-Agent", "dailyos-test"),
            ("Content-Type", "application/json"),
            ("Accept", "application/json"),
            ("Date", "Mon, 04 May 2026 12:00:00 GMT"),
            ("Authorization", "Bearer secret"),
            ("X-Request-Id", "request-123"),
        ]);

        assert_eq!(
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo",
                &stable_headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            ),
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo",
                &volatile_headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            )
        );
    }

    #[test]
    fn request_key_includes_content_encoding_header() {
        let auth_scope_id = AuthScopeId::try_new("test-tenant-1").unwrap();

        assert_ne!(
            RequestKey::canonicalize(
                "POST",
                "https://example.com/api/foo",
                &headers(&[("Content-Encoding", "gzip")]),
                b"compressed",
                &auth_scope_id,
            ),
            RequestKey::canonicalize(
                "POST",
                "https://example.com/api/foo",
                &headers(&[("Content-Encoding", "identity")]),
                b"compressed",
                &auth_scope_id,
            )
        );
    }

    #[test]
    fn request_key_sorts_query_params() {
        let headers = headers(&[]);

        assert_eq!(
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo?b=2&a=1",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            ),
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo?a=1&b=2",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            )
        );
    }

    #[test]
    fn request_key_includes_auth_scope_for_tenant_isolation() {
        assert_ne!(
            request_key(b"", "test-tenant-1"),
            request_key(b"", "test-tenant-2")
        );
    }

    #[test]
    fn request_key_method_is_uppercase() {
        let headers = headers(&[]);

        assert_eq!(
            RequestKey::canonicalize(
                "get",
                "https://example.com/api/foo",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            ),
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            )
        );
    }

    #[test]
    fn request_key_includes_request_body() {
        assert_ne!(
            request_key(br#"{"query":"alpha"}"#, "test-tenant-1"),
            request_key(br#"{"query":"beta"}"#, "test-tenant-1")
        );
    }

    #[test]
    fn request_key_strips_url_fragment() {
        let headers = headers(&[]);

        assert_eq!(
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo#bar",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            ),
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo",
                &headers,
                b"",
                &AuthScopeId::try_new("test-tenant-1").unwrap(),
            )
        );
    }

    #[test]
    fn request_key_strips_url_userinfo() {
        let headers = headers(&[]);
        let auth_scope_id = AuthScopeId::try_new("test-tenant-1").unwrap();

        assert_eq!(
            RequestKey::canonicalize(
                "GET",
                "https://user:password@example.com/api/foo",
                &headers,
                b"",
                &auth_scope_id,
            ),
            RequestKey::canonicalize(
                "GET",
                "https://example.com/api/foo",
                &headers,
                b"",
                &auth_scope_id,
            )
        );
    }

    #[test]
    fn request_key_rejects_empty_auth_scope_id() {
        let err = RequestKey::try_canonicalize(
            "GET",
            "https://example.com/api/foo",
            &headers(&[]),
            b"",
            "",
        )
        .unwrap_err();

        assert_eq!(err, AuthScopeIdError::Empty);
    }

    #[test]
    fn auth_scope_id_rejects_whitespace_and_reserved_values() {
        assert_eq!(
            AuthScopeId::try_new("test tenant").unwrap_err(),
            AuthScopeIdError::ContainsWhitespace
        );
        assert_eq!(
            AuthScopeId::try_new("production").unwrap_err(),
            AuthScopeIdError::Reserved
        );
    }

    #[test]
    fn external_replay_missing_returns_typed_error_with_request_key_hex() {
        let fixture = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": []
            }),
            "inline",
        )
        .unwrap();
        let key = request_key(b"", "test-tenant-1");

        let error = ExternalReplayFixture::lookup(
            &fixture,
            &key,
            "GET",
            "https://example.com/api/foo?secret=redacted",
        )
        .unwrap_err();

        assert_eq!(error.request_key_hex, key.to_hex());
        assert_eq!(
            error.to_string(),
            format!(
                "external replay fixture missing for request: {}",
                key.to_hex()
            )
        );
    }

    #[test]
    fn external_replay_missing_url_redacted_strips_query_params_and_fragment() {
        let key = request_key(b"", "test-tenant-1");

        let error = ExternalReplayFixtureMissing::new(
            &key,
            "get",
            "https://example.com/api/foo?token=secret#fragment",
        );

        assert_eq!(error.method, "GET");
        assert_eq!(error.url_redacted, "https://example.com/<redacted>");
    }

    #[test]
    fn json_fixture_loads_from_file_round_trip() {
        let key = request_key(b"", "test-tenant-1");
        let body = br#"{"ok":true}"#;
        let fixture_json = json!({
            "version": 1,
            "fixtures": [
                {
                    "request_key_hex": key.to_hex(),
                    "method": "GET",
                    "url": "https://example.com/api/foo?a=1&b=2",
                    "auth_scope_id": "test-tenant-1",
                    "response": {
                        "status": 200,
                        "headers": [["Content-Type", "application/json"]],
                        "body_base64": response_body_base64(body)
                    }
                }
            ]
        });
        let tempdir = tempfile::tempdir().unwrap();
        let fixture_path = tempdir.path().join("external_replay.json");
        fs::write(
            &fixture_path,
            serde_json::to_string_pretty(&fixture_json).unwrap(),
        )
        .unwrap();

        let fixture = JsonExternalReplayFixture::from_json_file(&fixture_path).unwrap();

        assert_eq!(fixture.fixture_path(), fixture_path.display().to_string());
        assert_eq!(fixture.len(), 1);
        assert_eq!(
            ExternalReplayFixture::lookup(&fixture, &key, "GET", "https://example.com/api/foo")
                .unwrap(),
            ReplayResponse {
                status: 200,
                headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                body: body.to_vec(),
            }
        );
    }

    #[test]
    fn json_fixture_loads_from_value_round_trip() {
        let key = request_key(b"{}", "test-tenant-1");
        let body = br#"{"ok":true}"#;
        let fixture_json = json!({
            "version": 1,
            "fixtures": [
                {
                    "request_key_hex": key.to_hex(),
                    "method": "GET",
                    "url": "https://example.com/api/foo?a=1&b=2",
                    "auth_scope_id": "test-tenant-1",
                    "response": {
                        "status": 202,
                        "headers": [["Accept", "application/json"]],
                        "body_base64": response_body_base64(body)
                    }
                }
            ]
        });

        let fixture =
            JsonExternalReplayFixture::from_json_value(&fixture_json, "inline-fixture").unwrap();

        assert_eq!(fixture.fixture_path(), "inline-fixture");
        assert_eq!(
            ExternalReplayFixture::lookup(&fixture, &key, "GET", "https://example.com/api/foo")
                .unwrap(),
            ReplayResponse {
                status: 202,
                headers: vec![("Accept".to_string(), "application/json".to_string())],
                body: body.to_vec(),
            }
        );
    }

    #[test]
    fn json_fixture_load_fails_on_invalid_version() {
        let error = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 2,
                "fixtures": []
            }),
            "inline",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FixtureLoadError::InvalidVersion {
                version: 2,
                fixture_path
            } if fixture_path == "inline"
        ));
    }

    #[test]
    fn json_fixture_load_fails_on_invalid_request_key_hex() {
        let error = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [
                    {
                        "request_key_hex": "not-a-valid-request-key",
                        "method": "GET",
                        "url": "https://example.com/api/foo",
                        "auth_scope_id": "test-tenant-1",
                        "response": {
                            "status": 200,
                            "headers": [],
                            "body_base64": ""
                        }
                    }
                ]
            }),
            "inline",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FixtureLoadError::InvalidRequestKeyHex {
                request_key_hex,
                fixture_path,
                ..
            } if request_key_hex == "not-a-valid-request-key" && fixture_path == "inline"
        ));
    }

    #[test]
    fn json_fixture_load_fails_when_file_exceeds_size_cap() {
        let tempdir = tempfile::tempdir().unwrap();
        let fixture_path = tempdir.path().join("oversized_external_replay.json");
        fs::write(&fixture_path, vec![b'{'; 10 * 1024 * 1024 + 1]).unwrap();

        let error = JsonExternalReplayFixture::from_json_file(&fixture_path).unwrap_err();

        assert!(matches!(
            error,
            FixtureLoadError::FixtureTooLarge {
                byte_count,
                cap,
                fixture_path: _
            } if byte_count == 10 * 1024 * 1024 + 1 && cap == 10 * 1024 * 1024
        ));
    }

    #[test]
    fn json_fixture_load_fails_when_response_body_exceeds_size_cap() {
        let key = request_key(b"", "test-tenant-1");
        let oversized_body = vec![b'a'; 1024 * 1024 + 1];
        let error = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [
                    {
                        "request_key_hex": key.to_hex(),
                        "method": "GET",
                        "url": "https://example.com/api/foo",
                        "auth_scope_id": "test-tenant-1",
                        "response": {
                            "status": 200,
                            "headers": [],
                            "body_base64": response_body_base64(&oversized_body)
                        }
                    }
                ]
            }),
            "inline",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FixtureLoadError::FixtureTooLarge {
                byte_count,
                cap,
                fixture_path
            } if byte_count == 1024 * 1024 + 1 && cap == 1024 * 1024 && fixture_path == "inline"
        ));
    }

    #[test]
    fn json_fixture_load_fails_on_empty_auth_scope_id() {
        let key = request_key(b"", "test-tenant-1");
        let error = JsonExternalReplayFixture::from_json_value(
            &json!({
                "version": 1,
                "fixtures": [
                    {
                        "request_key_hex": key.to_hex(),
                        "method": "GET",
                        "url": "https://example.com/api/foo",
                        "auth_scope_id": "",
                        "response": {
                            "status": 200,
                            "headers": [],
                            "body_base64": ""
                        }
                    }
                ]
            }),
            "inline",
        )
        .unwrap_err();

        assert!(matches!(
            error,
            FixtureLoadError::InvalidAuthScopeId {
                request_key_hex,
                fixture_path,
                source: AuthScopeIdError::Empty
            } if request_key_hex == key.to_hex() && fixture_path == "inline"
        ));
    }
}
