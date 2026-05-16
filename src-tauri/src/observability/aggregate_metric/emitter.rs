use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

use super::AggregateMetric;

const HTTPS_PREFIX: &[u8] = b"https://";
pub const MAX_RETRY_ATTEMPTS: usize = 5;
pub const MAX_BACKOFF: Duration = Duration::from_secs(60 * 60);
pub const PRODUCTION_COLLECTOR_URL: HttpsUrl =
    HttpsUrl::from_static_https("https://telemetry.dailyos.dev/v1/aggregate");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsUrl {
    value: HttpsUrlValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HttpsUrlValue {
    Static(&'static str),
    Runtime(String),
}

impl HttpsUrl {
    pub const fn from_static_https(value: &'static str) -> Self {
        if !starts_with_https(value.as_bytes()) {
            panic!("aggregate telemetry collector URL must use https://");
        }
        Self {
            value: HttpsUrlValue::Static(value),
        }
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, HttpsUrlError> {
        let value = value.into();
        let parsed = url::Url::parse(&value).map_err(HttpsUrlError::Invalid)?;
        if parsed.scheme() != "https" {
            return Err(HttpsUrlError::NonHttps);
        }
        if parsed.host_str().is_none() {
            return Err(HttpsUrlError::MissingHost);
        }
        Ok(Self {
            value: HttpsUrlValue::Runtime(value),
        })
    }

    pub fn as_str(&self) -> &str {
        match &self.value {
            HttpsUrlValue::Static(value) => value,
            HttpsUrlValue::Runtime(value) => value.as_str(),
        }
    }
}

impl fmt::Display for HttpsUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Error)]
pub enum HttpsUrlError {
    #[error("collector URL is invalid: {0}")]
    Invalid(url::ParseError),
    #[error("collector URL must use https")]
    NonHttps,
    #[error("collector URL must include a host")]
    MissingHost,
}

#[derive(Debug, Error)]
pub enum EmitError {
    #[error("aggregate telemetry request failed after {attempts} attempts")]
    RetryExhausted { attempts: usize },
    #[error("aggregate telemetry request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitStatus {
    Accepted,
    Retryable,
}

pub fn collector_url() -> Result<HttpsUrl, HttpsUrlError> {
    #[cfg(feature = "debug-telemetry-override")]
    if let Ok(value) = std::env::var("DAILYOS_TELEMETRY_COLLECTOR_URL") {
        if !value.trim().is_empty() {
            return HttpsUrl::parse(value);
        }
    }

    Ok(PRODUCTION_COLLECTOR_URL.clone())
}

pub async fn emit(metrics: &[AggregateMetric], collector_url: &HttpsUrl) -> Result<(), EmitError> {
    if metrics.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::new();
    for attempt_index in 0..MAX_RETRY_ATTEMPTS {
        let status = post_once(&client, collector_url, metrics).await?;
        if status == EmitStatus::Accepted {
            return Ok(());
        }
        if attempt_index + 1 == MAX_RETRY_ATTEMPTS {
            break;
        }
        tokio::time::sleep(jittered_backoff(attempt_index, jitter_seed())).await;
    }

    Err(EmitError::RetryExhausted {
        attempts: MAX_RETRY_ATTEMPTS,
    })
}

async fn post_once(
    client: &reqwest::Client,
    collector_url: &HttpsUrl,
    metrics: &[AggregateMetric],
) -> Result<EmitStatus, reqwest::Error> {
    let response = client
        .post(collector_url.as_str())
        .json(&AggregateMetricBatch { metrics })
        .send()
        .await?;
    Ok(status_to_emit_status(response.status()))
}

#[derive(Serialize)]
struct AggregateMetricBatch<'a> {
    metrics: &'a [AggregateMetric],
}

pub fn status_to_emit_status(status: StatusCode) -> EmitStatus {
    if status == StatusCode::OK {
        EmitStatus::Accepted
    } else {
        EmitStatus::Retryable
    }
}

pub fn jittered_backoff(attempt_index: usize, seed: u64) -> Duration {
    let base_secs = match attempt_index {
        0 => 5 * 60,
        1 => 15 * 60,
        2 => 45 * 60,
        _ => MAX_BACKOFF.as_secs(),
    };
    let jitter_window = (base_secs / 5).max(1);
    let jitter = seed.wrapping_add(attempt_index as u64 * 37) % jitter_window;
    Duration::from_secs((base_secs + jitter).min(MAX_BACKOFF.as_secs()))
}

fn jitter_seed() -> u64 {
    let uuid = uuid::Uuid::new_v4();
    (uuid.as_u128() & u128::from(u64::MAX)) as u64
}

const fn starts_with_https(value: &[u8]) -> bool {
    if value.len() < HTTPS_PREFIX.len() {
        return false;
    }
    let mut i = 0;
    while i < HTTPS_PREFIX.len() {
        if value[i] != HTTPS_PREFIX[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_rejects_plaintext_http() {
        assert!(matches!(
            HttpsUrl::parse("http://telemetry.example.test"),
            Err(HttpsUrlError::NonHttps)
        ));
        assert!(HttpsUrl::parse("https://telemetry.example.test").is_ok());
    }

    #[test]
    fn retry_backoff_is_capped_at_one_hour() {
        for attempt in 0..10 {
            assert!(jittered_backoff(attempt, 42) <= MAX_BACKOFF);
        }
    }

    #[test]
    fn emitter_status_contract_ignores_response_body() {
        assert_eq!(status_to_emit_status(StatusCode::OK), EmitStatus::Accepted);
        assert_eq!(
            status_to_emit_status(StatusCode::INTERNAL_SERVER_ERROR),
            EmitStatus::Retryable
        );
    }
}
