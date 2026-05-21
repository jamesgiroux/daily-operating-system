//! `list_projects` producer — see `list_accounts::producer` for the
//! cursor + filter discipline; this module mirrors that structure
//! against the `ProjectListReadHandle` seam.

use serde_json::json;

use crate::abilities::get_entity_intelligence::contracts::{CursorState, Paginated};
use crate::abilities::list_pagination::{
    decode_cursor, encode_cursor, finalize_pagination, watermark_from_request,
};
use crate::abilities::list_projects::contracts::{
    ProjectListFilter, ProjectListInput, ProjectSummary, ProjectTrajectory,
};
use crate::abilities::list_projects::{ABILITY_NAME, ABILITY_SCHEMA_VERSION};
use crate::abilities::provenance::SubjectRef;
use crate::abilities::{AbilityContext, AbilityError, AbilityErrorKind, AbilityResult};
use crate::services::context::{
    ProjectListQuery, ProjectListReadError, ProjectListSnapshot, ProjectListSummary,
};

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 200;

pub async fn list_projects(
    ctx: &AbilityContext<'_>,
    input: ProjectListInput,
) -> AbilityResult<Paginated<ProjectSummary>> {
    validate_schema_version(input.schema_version)?;
    let normalized_filter = normalize_filter(input.filter.as_ref())?;
    let page_size = validate_page_size(input.page_size)?;

    let watermark = watermark_from_request(&request_fingerprint(&normalized_filter, page_size));

    let offset = if let Some(cursor) = input.cursor.as_ref() {
        let Some(payload) = decode_cursor(cursor) else {
            return finalize_pagination(
                ctx,
                ABILITY_NAME,
                ABILITY_SCHEMA_VERSION,
                SubjectRef::Global,
                Paginated {
                    items: Vec::new(),
                    next_cursor: None,
                    total_hint: None,
                    cursor_state: CursorState::Invalidated {
                        reason: "cursor token malformed".to_string(),
                        restart_required: true,
                    },
                },
            );
        };
        if payload.watermark != watermark {
            return finalize_pagination(
                ctx,
                ABILITY_NAME,
                ABILITY_SCHEMA_VERSION,
                SubjectRef::Global,
                Paginated {
                    items: Vec::new(),
                    next_cursor: None,
                    total_hint: None,
                    cursor_state: CursorState::Invalidated {
                        reason: "filter or page_size changed since cursor was issued"
                            .to_string(),
                        restart_required: true,
                    },
                },
            );
        }
        payload.offset
    } else {
        0
    };

    let snapshot = ctx
        .services()
        .read_list_projects(ProjectListQuery {
            status: normalized_filter.status.clone(),
            trajectory: normalized_filter.trajectory,
            parent_account_id: normalized_filter.parent_account_id.clone(),
            name_contains: normalized_filter.name_contains.clone(),
            offset,
            page_size,
        })
        .await
        .map_err(read_error)?;

    let body = shape_response(snapshot, offset, &watermark);
    finalize_pagination(
        ctx,
        ABILITY_NAME,
        ABILITY_SCHEMA_VERSION,
        SubjectRef::Global,
        body,
    )
}

fn shape_response(
    snapshot: ProjectListSnapshot,
    offset: u64,
    watermark: &str,
) -> Paginated<ProjectSummary> {
    let ProjectListSnapshot {
        items,
        total_after_filter,
        data_shifted_advisory,
    } = snapshot;

    let items: Vec<ProjectSummary> = items.into_iter().map(project_summary_from).collect();
    let consumed = offset.saturating_add(items.len() as u64);
    let next_cursor = if consumed < total_after_filter {
        Some(encode_cursor(consumed, watermark))
    } else {
        None
    };
    let cursor_state = match data_shifted_advisory {
        Some(advisory) => CursorState::DataShifted { advisory },
        None => CursorState::Stable,
    };

    Paginated {
        items,
        next_cursor,
        total_hint: Some(total_after_filter),
        cursor_state,
    }
}

fn project_summary_from(summary: ProjectListSummary) -> ProjectSummary {
    ProjectSummary {
        project_id: summary.project_id,
        name: summary.name,
        parent_account_id: summary.parent_account_id,
        status: summary.status,
        trajectory: summary.trajectory,
        last_touchpoint_at: summary.last_touchpoint_at,
    }
}

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == ABILITY_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn validate_page_size(page_size: u32) -> Result<u32, AbilityError> {
    if page_size == 0 {
        Ok(DEFAULT_PAGE_SIZE)
    } else if page_size > MAX_PAGE_SIZE {
        Err(validation_error(format!(
            "page_size `{page_size}` exceeds MAX_PAGE_SIZE `{MAX_PAGE_SIZE}`"
        )))
    } else {
        Ok(page_size)
    }
}

fn normalize_filter(filter: Option<&ProjectListFilter>) -> Result<NormalizedFilter, AbilityError> {
    let Some(filter) = filter else {
        return Ok(NormalizedFilter::default());
    };
    Ok(NormalizedFilter {
        status: trimmed_optional(filter.status.as_deref(), "filter.status")?,
        trajectory: filter.trajectory,
        parent_account_id: trimmed_optional(
            filter.parent_account_id.as_deref(),
            "filter.parent_account_id",
        )?,
        name_contains: trimmed_optional(filter.name_contains.as_deref(), "filter.name_contains")?,
    })
}

fn trimmed_optional(value: Option<&str>, label: &str) -> Result<Option<String>, AbilityError> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                Err(validation_error(format!(
                    "{label} must be non-empty when provided"
                )))
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
    }
}

#[derive(Default)]
struct NormalizedFilter {
    status: Option<String>,
    trajectory: Option<ProjectTrajectory>,
    parent_account_id: Option<String>,
    name_contains: Option<String>,
}

fn request_fingerprint(filter: &NormalizedFilter, page_size: u32) -> String {
    json!({
        "status": filter.status,
        "trajectory": filter.trajectory,
        "parent_account_id": filter.parent_account_id,
        "name_contains": filter.name_contains,
        "page_size": page_size,
        "ability": ABILITY_NAME,
    })
    .to_string()
}

fn read_error(error: ProjectListReadError) -> AbilityError {
    match error {
        ProjectListReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError("list_projects_read_failed".to_string()),
            message,
        },
    }
}

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}
