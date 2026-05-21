//! `list_accounts` producer — validates input, delegates to the
//! `ListAccountsReadHandle` service seam, applies opaque-cursor
//! pagination, and shapes the response into `Paginated<AccountSummary>`.

use serde_json::json;

use crate::abilities::get_entity_intelligence::contracts::{CursorState, Paginated};
use crate::abilities::list_accounts::contracts::{
    AccountListFilter, AccountListInput, AccountSummary,
};
use crate::abilities::list_accounts::{ABILITY_NAME, ABILITY_SCHEMA_VERSION};
use crate::abilities::list_pagination::{
    decode_cursor, encode_cursor, finalize_pagination, watermark_from_request,
};
use crate::abilities::provenance::SubjectRef;
use crate::abilities::{AbilityContext, AbilityError, AbilityErrorKind, AbilityResult};
use crate::services::context::{
    AccountListQuery, AccountListReadError, AccountListSnapshot, AccountListSummary,
};

const DEFAULT_PAGE_SIZE: u32 = 25;
const MAX_PAGE_SIZE: u32 = 200;

pub async fn list_accounts(
    ctx: &AbilityContext<'_>,
    input: AccountListInput,
) -> AbilityResult<Paginated<AccountSummary>> {
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
        .read_list_accounts(AccountListQuery {
            status: normalized_filter.status.clone(),
            health_band: normalized_filter.health_band,
            name_contains: normalized_filter.name_contains.clone(),
            offset,
            page_size,
        })
        .await
        .map_err(read_error)?;

    let body = shape_response(snapshot, offset, page_size, &watermark);
    finalize_pagination(
        ctx,
        ABILITY_NAME,
        ABILITY_SCHEMA_VERSION,
        SubjectRef::Global,
        body,
    )
}

fn shape_response(
    snapshot: AccountListSnapshot,
    offset: u64,
    page_size: u32,
    watermark: &str,
) -> Paginated<AccountSummary> {
    let AccountListSnapshot {
        items,
        total_after_filter,
        data_shifted_advisory,
    } = snapshot;

    let items: Vec<AccountSummary> = items.into_iter().map(account_summary_from).collect();
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

    let _ = page_size; // page_size is part of the watermark, not the response shape
    Paginated {
        items,
        next_cursor,
        total_hint: Some(total_after_filter),
        cursor_state,
    }
}

fn account_summary_from(summary: AccountListSummary) -> AccountSummary {
    AccountSummary {
        account_id: summary.account_id,
        name: summary.name,
        status: summary.status,
        health_band: summary.health_band,
        last_touchpoint_at: summary.last_touchpoint_at,
        open_loops_count: summary.open_loops_count,
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

fn normalize_filter(
    filter: Option<&AccountListFilter>,
) -> Result<NormalizedFilter, AbilityError> {
    let Some(filter) = filter else {
        return Ok(NormalizedFilter::default());
    };
    let status = match filter.status.as_deref() {
        None => None,
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(validation_error(
                    "filter.status must be non-empty when provided",
                ));
            }
            Some(trimmed.to_string())
        }
    };
    let name_contains = match filter.name_contains.as_deref() {
        None => None,
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(validation_error(
                    "filter.name_contains must be non-empty when provided",
                ));
            }
            Some(trimmed.to_string())
        }
    };
    Ok(NormalizedFilter {
        status,
        health_band: filter.health_band,
        name_contains,
    })
}

#[derive(Default)]
struct NormalizedFilter {
    status: Option<String>,
    health_band: Option<crate::abilities::trust::types::TrustBand>,
    name_contains: Option<String>,
}

fn request_fingerprint(filter: &NormalizedFilter, page_size: u32) -> String {
    json!({
        "status": filter.status,
        "health_band": filter.health_band,
        "name_contains": filter.name_contains,
        "page_size": page_size,
        "ability": ABILITY_NAME,
    })
    .to_string()
}

fn read_error(error: AccountListReadError) -> AbilityError {
    match error {
        AccountListReadError::ReadFailed(message) => AbilityError {
            kind: AbilityErrorKind::HardError("list_accounts_read_failed".to_string()),
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
