use async_trait::async_trait;

use crate::abilities::registry::AbilityContext;

#[async_trait]
pub trait WorkspaceIntakeService: Send + Sync {
    async fn ingest(
        &self,
        ctx: &AbilityContext<'_>,
        request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError>;
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
