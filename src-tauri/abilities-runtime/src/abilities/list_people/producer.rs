//! `list_people` producer — see `list_accounts::producer` for the
//! cursor + filter discipline; this module mirrors that structure
//! against the `PersonListReadHandle` seam.

use serde_json::json;

use crate::abilities::get_entity_intelligence::contracts::{CursorState, Paginated};
use crate::abilities::list_pagination::{
    decode_cursor, encode_cursor, finalize_pagination, watermark_from_request,
};
use crate::abilities::list_people::contracts::{PersonListFilter, PersonListInput, PersonSummary};
use crate::abilities::list_people::{ABILITY_NAME, ABILITY_SCHEMA_VERSION};
use crate::abilities::provenance::SubjectRef;
use crate::abilities::{AbilityContext, AbilityError, AbilityErrorKind, AbilityResult};
use crate::services::context::{
    PersonListQuery, PersonListReadError, PersonListSnapshot, PersonListSummary,
};

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 200;

pub async fn list_people(
    ctx: &AbilityContext<'_>,
    input: PersonListInput,
) -> AbilityResult<Paginated<PersonSummary>> {
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
                        reason: "filter or page_size changed since cursor was issued".to_string(),
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
        .read_list_people(PersonListQuery {
            role: normalized_filter.role.clone(),
            primary_account_id: normalized_filter.primary_account_id.clone(),
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
    snapshot: PersonListSnapshot,
    offset: u64,
    watermark: &str,
) -> Paginated<PersonSummary> {
    let PersonListSnapshot {
        items,
        total_after_filter,
        data_shifted_advisory,
    } = snapshot;

    let items: Vec<PersonSummary> = items.into_iter().map(person_summary_from).collect();
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

fn person_summary_from(summary: PersonListSummary) -> PersonSummary {
    PersonSummary {
        person_id: summary.person_id,
        display_name: summary.display_name,
        primary_account_id: summary.primary_account_id,
        role: summary.role,
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

fn normalize_filter(filter: Option<&PersonListFilter>) -> Result<NormalizedFilter, AbilityError> {
    let Some(filter) = filter else {
        return Ok(NormalizedFilter::default());
    };
    Ok(NormalizedFilter {
        role: trimmed_optional(filter.role.as_deref(), "filter.role")?,
        primary_account_id: trimmed_optional(
            filter.primary_account_id.as_deref(),
            "filter.primary_account_id",
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
    role: Option<String>,
    primary_account_id: Option<String>,
    name_contains: Option<String>,
}

fn request_fingerprint(filter: &NormalizedFilter, page_size: u32) -> String {
    json!({
        "role": filter.role,
        "primary_account_id": filter.primary_account_id,
        "name_contains": filter.name_contains,
        "page_size": page_size,
        "ability": ABILITY_NAME,
    })
    .to_string()
}

fn read_error(error: PersonListReadError) -> AbilityError {
    match error {
        PersonListReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError("list_people_read_failed".to_string()),
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
