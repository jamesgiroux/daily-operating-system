use async_trait::async_trait;
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::abilities::registry::AbilityContext;

#[async_trait]
pub trait WorkspaceIntakeService: Send + Sync {
    async fn ingest(
        &self,
        ctx: &AbilityContext<'_>,
        request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError>;

    async fn place_document(
        &self,
        _ctx: &AbilityContext<'_>,
        _invocation: PlacementInvocationContext,
        _request: WorkspacePlaceDocumentRequest,
    ) -> Result<WorkspacePlaceDocumentReceipt, PlacementError> {
        Err(PlacementError::internal(
            "workspace placement service unavailable",
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceIntakeRequest {
    pub file_ref: String,
    pub source_type_slug: String,
    pub entity: Option<EntityRefDto>,
    pub mode_slug: String,
    pub category_slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRefDto {
    pub entity_type_slug: String,
    pub entity_id: String,
    pub entity_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceIntakeReceipt {
    pub run_id: String,
    pub file_id: String,
    pub content_sha256: String,
    pub lifecycle_state_after_slug: String,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceIntakeError {
    InvalidSourceTypeSlug(String),
    InvalidModeSlug(String),
    InvalidCategorySlug(String),
    CategoryNotAllowed { allowed: Vec<String> },
    InvalidEntityTypeSlug(String),
    InvalidEntityId,
    InvalidEntityName(String),
    EntityNotFound,
    FileNotFound,
    PathTraversalAttempt,
    OutsideWorkspace,
    SymlinkRaced,
    FileTooLarge,
    UnsupportedFormat,
    AlreadyProcessed { existing_run_id: String },
    Io(String),
    DbError(String),
}

impl std::fmt::Display for WorkspaceIntakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for WorkspaceIntakeError {}

pub const WORKSPACE_PLACE_DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const WORKSPACE_PLACE_DOCUMENT_TOOL_NAME: &str = "dailyos.write.place_document";
pub const WORKSPACE_PLACE_DOCUMENT_SCOPE: &str = "write.workspace_place_document";
pub const WORKSPACE_PLACE_DOCUMENT_CONTENT_B64_MAX_BYTES: usize = 13_981_016;
pub const WORKSPACE_PLACE_DOCUMENT_DECODED_MAX_BYTES: usize = 10_485_760;
pub const WORKSPACE_PLACE_DOCUMENT_SERIALIZED_ARGUMENTS_MAX_BYTES: usize = 14_100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePlaceDocumentInput {
    pub(crate) raw: serde_json::Value,
}

impl WorkspacePlaceDocumentInput {
    pub(crate) fn into_raw(self) -> serde_json::Value {
        self.raw
    }
}

impl<'de> Deserialize<'de> for WorkspacePlaceDocumentInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_json::Value::deserialize(deserializer).map(|raw| Self { raw })
    }
}

impl JsonSchema for WorkspacePlaceDocumentInput {
    fn schema_name() -> String {
        WorkspacePlaceDocumentRequest::schema_name()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        WorkspacePlaceDocumentRequest::json_schema(gen)
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePlaceDocumentRequest {
    pub schema_version: u32,
    pub entity: WorkspacePlacementEntity,
    pub content_b64: String,
    pub content_type: String,
    #[serde(default, deserialize_with = "deserialize_optional_string_no_null")]
    pub filename_hint: Option<String>,
    pub category: String,
    #[serde(default, deserialize_with = "deserialize_optional_string_no_null")]
    pub client_dedup_key: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspacePlacementEntity {
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementInvocationContext {
    pub actor_id: String,
    pub tool_name: String,
    pub can_read_entity_names: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct WorkspacePlaceDocumentReceipt {
    pub schema_version: u32,
    pub document_handle: Option<String>,
    pub source_handle: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,
    pub workspace_file_kind: String,
    pub source_asof: Option<String>,
    pub lifecycle_state: String,
    pub claim_count_produced: u64,
    pub idempotent_replay: bool,
    pub dry_run: bool,
    pub resolved_path: Option<String>,
    pub mutation_cursor: WorkspacePlacementMutationCursor,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspacePlacementMutationCursor {
    WorkspacePlacement {
        document_handle: String,
        source_handle: String,
        idempotency_id: String,
    },
    WorkspacePlacementPreview {
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PlacementError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl PlacementError {
    pub fn new(code: PlacementErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            allowed: None,
            retry_after_seconds: None,
            trace_id: None,
        }
    }

    pub fn with_allowed(mut self, allowed: Vec<String>) -> Self {
        self.allowed = Some(allowed);
        self
    }

    pub fn with_retry_after(mut self, retry_after_seconds: u64) -> Self {
        self.retry_after_seconds = Some(retry_after_seconds);
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        drop(message.into());
        Self::new(
            PlacementErrorCode::PlacementInternal,
            "workspace placement failed internally",
        )
        .with_trace_id(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for PlacementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PlacementError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementErrorCode {
    InvalidRequestShape,
    InvalidEntityType,
    InvalidEntityId,
    TargetNotFoundOrUnauthorized,
    EntityNotRoutable,
    InvalidCategory,
    CategoryNotAllowed,
    InvalidContentEncoding,
    InvalidContentType,
    ContentTooLarge,
    InvalidFilenameHint,
    InvalidClientDedupKey,
    UnsupportedSchemaVersion,
    IdempotencyInProgress,
    PreviousAttemptFailed,
    PlacementPathRejected,
    RateLimited,
    IngestionFailed,
    PlacementInternal,
}

impl PlacementErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequestShape => "invalid_request_shape",
            Self::InvalidEntityType => "invalid_entity_type",
            Self::InvalidEntityId => "invalid_entity_id",
            Self::TargetNotFoundOrUnauthorized => "target_not_found_or_unauthorized",
            Self::EntityNotRoutable => "entity_not_routable",
            Self::InvalidCategory => "invalid_category",
            Self::CategoryNotAllowed => "category_not_allowed",
            Self::InvalidContentEncoding => "invalid_content_encoding",
            Self::InvalidContentType => "invalid_content_type",
            Self::ContentTooLarge => "content_too_large",
            Self::InvalidFilenameHint => "invalid_filename_hint",
            Self::InvalidClientDedupKey => "invalid_client_dedup_key",
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::IdempotencyInProgress => "idempotency_in_progress",
            Self::PreviousAttemptFailed => "previous_attempt_failed",
            Self::PlacementPathRejected => "placement_path_rejected",
            Self::RateLimited => "rate_limited",
            Self::IngestionFailed => "ingestion_failed",
            Self::PlacementInternal => "placement_internal",
        }
    }
}

fn deserialize_optional_string_no_null<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(value) => Ok(Some(value)),
        serde_json::Value::Null => Err(serde::de::Error::custom("null is not allowed")),
        _ => Err(serde::de::Error::custom("expected string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn place_document_request_rejects_null_optional_strings() {
        let request = json!({
            "schema_version": 1,
            "entity": {
                "entity_type": "account",
                "entity_id": "acct_123"
            },
            "content_b64": "aGVsbG8=",
            "content_type": "text/markdown",
            "filename_hint": null,
            "category": "notes"
        });

        let error = serde_json::from_value::<WorkspacePlaceDocumentRequest>(request)
            .expect_err("null optional strings are not accepted at the MCP schema boundary");
        assert!(
            error.to_string().contains("null is not allowed"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn place_document_request_allows_omitted_optional_strings() {
        let request = json!({
            "schema_version": 1,
            "entity": {
                "entity_type": "account",
                "entity_id": "acct_123"
            },
            "content_b64": "aGVsbG8=",
            "content_type": "text/markdown",
            "category": "notes"
        });

        let parsed: WorkspacePlaceDocumentRequest =
            serde_json::from_value(request).expect("omitted optional strings deserialize");
        assert_eq!(parsed.filename_hint, None);
        assert_eq!(parsed.client_dedup_key, None);
        assert!(!parsed.dry_run);
    }

    #[test]
    fn placement_internal_error_uses_safe_public_message() {
        let error = PlacementError::internal("debug detail: /tmp/workspace/path");

        assert_eq!(error.code, PlacementErrorCode::PlacementInternal.as_str());
        assert_eq!(error.message, "workspace placement failed internally");
        assert!(error.trace_id.is_some());
    }
}
