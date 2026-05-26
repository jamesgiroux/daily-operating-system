//! MCP briefing tool handlers.
//!
//! `dailyos.read.daily_briefing` wraps the claim-backed `get_daily_briefing`
//! ability from `abilities-runtime`. The handler only synthesizes local
//! invocation defaults that the Tauri app already supplies (workspace scope +
//! user's local date); it does not read legacy briefing JSON or generate prose
//! directly.
//!
//! `dailyos.read.meeting_briefing` uses the existing claim-backed meeting prep
//! context reader. It intentionally does not invoke the `prepare_meeting`
//! Transform ability because that ability performs provider-backed synthesis.

use std::collections::BTreeMap;
use std::sync::Arc;

use abilities_runtime::abilities::registry::{AbilityRegistry, McpExposure};
use abilities_runtime::abilities::tracer::NOOP_ABILITY_TRACER;
use chrono::Utc;
use serde_json::{json, Value};

use crate::bridges::types::{
    invoke_registry_json_for_actor, AbilityInvokeError, RequestScopedInvocation,
    BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
};
use crate::bridges::{BridgeActor, BridgeSurface};
use crate::db::{ActionDb, LocalKeychain};
use crate::services::context::{
    attach_live_workspace_readers, ClaimDismissalSurface, ExternalClients,
    PrepareMeetingAttendeeSnapshot, PrepareMeetingContextSnapshot,
    PrepareMeetingLinearIssueChangeSnapshot, PrepareMeetingSubjectSnapshot, ServiceContext,
    SystemClock, SystemRng,
};
use crate::services::mcp_v2::actor_policy::{project_actor, ToolGrant, ToolRateLimit};
use crate::services::mcp_v2::contracts::{McpActor, McpToolHandler, ToolDescription, ToolError};
use crate::services::mcp_v2::runtime_projection::{compact_text, humanize_token, string_at};

const ACTOR_LABEL: &str = concat!("agent:dailyos-mcp-v2:", env!("CARGO_PKG_VERSION"));
const DAILY_BRIEFING_ABILITY_NAME: &str = "get_daily_briefing";
const DAILY_BRIEFING_TOOL_NAME: &str = "dailyos.read.daily_briefing";
const MEETING_BRIEFING_PRODUCER: &str = "prepare_meeting_context_snapshot";
const MEETING_BRIEFING_TOOL_NAME: &str = "dailyos.read.meeting_briefing";
const DAILY_BRIEFING_SCHEMA_VERSION: u32 = 1;
const DAILY_BRIEFING_RESPONSE_SCHEMA_VERSION: u32 = 1;
const MEETING_BRIEFING_RESPONSE_SCHEMA_VERSION: u32 = 1;
const MAX_UPCOMING_MEETINGS_IN_ANSWER: usize = 5;
const MAX_MEETING_EVIDENCE_ITEMS: usize = 8;
const MAX_LINEAR_CHANGES: usize = 8;

pub struct DailyBriefingHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
}

impl DailyBriefingHandler {
    pub fn new(
        description: ToolDescription,
        registry: &'static AbilityRegistry,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            description,
            registry,
            runtime,
        }
    }

    pub fn from_runtime(
        description: ToolDescription,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, &'static str> {
        let registry = AbilityRegistry::global_checked()
            .map_err(|_| "ability registry violations present at startup")?;
        Ok(Self::new(description, registry, runtime))
    }
}

impl McpToolHandler for DailyBriefingHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, actor: &McpActor, _params: Value) -> Result<Value, ToolError> {
        let McpActor::Client {
            client_id,
            conversation_handle,
            tool_name,
            granted_scopes,
        } = actor;

        let synthetic_grant = ToolGrant {
            tool_name: tool_name.clone(),
            scopes_granted: granted_scopes.clone(),
            exposure: McpExposure::Invocable,
            rate_limit: ToolRateLimit {
                max_calls: 0,
                window_seconds: 0,
            },
        };
        let runtime_actor =
            project_actor(client_id, &synthetic_grant, conversation_handle.as_ref());

        let input = match daily_briefing_input_from_config() {
            Ok(input) => input,
            Err(reason) => return Ok(daily_briefing_unavailable_response(&reason)),
        };

        self.runtime.block_on(async {
            let clock = SystemClock;
            let rng = SystemRng;
            let external = ExternalClients::default();
            let services = attach_live_workspace_readers(
                ServiceContext::new_live(&clock, &rng, &external).with_actor(ACTOR_LABEL),
            );

            let invocation = RequestScopedInvocation {
                registry_actor: runtime_actor,
                response_actor: BridgeActor::McpClient,
                surface: BridgeSurface::McpTool,
                claim_dismissal_surface: ClaimDismissalSurface::McpTool,
                dry_run: false,
                confirmation: None,
                confirmation_store: None,
            };
            let response = invoke_registry_json_for_actor(
                self.registry,
                &services,
                &BRIDGE_NOOP_INTELLIGENCE_PROVIDER,
                &NOOP_ABILITY_TRACER,
                invocation,
                DAILY_BRIEFING_ABILITY_NAME,
                input,
            )
            .await
            .map_err(map_invoke_error)?;
            let invocation_id = response.invocation_id.0.to_string();
            Ok(present_daily_briefing_response(
                response.data,
                response.rendered_provenance.value,
                Some(&invocation_id),
            ))
        })
    }
}

pub struct MeetingBriefingHandler {
    description: ToolDescription,
}

impl MeetingBriefingHandler {
    pub fn new(description: ToolDescription) -> Self {
        Self { description }
    }
}

impl McpToolHandler for MeetingBriefingHandler {
    fn description(&self) -> &ToolDescription {
        &self.description
    }

    fn invoke(&self, _actor: &McpActor, params: Value) -> Result<Value, ToolError> {
        let meeting_id = extract_meeting_id(&params)?;
        let db = ActionDb::open_readonly(Arc::new(LocalKeychain::new())).map_err(|error| {
            eprintln!("mcp_v2 dailyos.read.meeting_briefing db open failed: {error:?}");
            ToolError::Internal {
                trace_id: "meeting_briefing_db_open".to_string(),
            }
        })?;

        match crate::services::meetings::load_prepare_meeting_context_snapshot(&db, &meeting_id) {
            Ok(snapshot) => Ok(present_meeting_briefing_response(snapshot)),
            Err(message) if message.contains("not found") => {
                Ok(meeting_briefing_not_found_response(&meeting_id))
            }
            Err(message) => {
                eprintln!("mcp_v2 dailyos.read.meeting_briefing context read failed: {message:?}");
                Ok(meeting_briefing_unavailable_response(&meeting_id))
            }
        }
    }
}

fn extract_meeting_id(params: &Value) -> Result<String, ToolError> {
    let meeting_id = params
        .get("meeting_id")
        .or_else(|| params.get("meetingId"))
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::BadParams {
            detail: "missing 'meeting_id' parameter (expected non-empty string)".into(),
        })?
        .trim();
    if meeting_id.is_empty() {
        return Err(ToolError::BadParams {
            detail: "'meeting_id' must be a non-empty string".into(),
        });
    }
    Ok(meeting_id.to_string())
}

fn daily_briefing_input_from_config() -> Result<Value, String> {
    let config = crate::state::load_config()
        .map_err(|error| format!("DailyOS workspace configuration is unavailable: {error}"))?;
    let workspace_id = config.workspace_path.trim();
    if workspace_id.is_empty() {
        return Err(
            "DailyOS workspace configuration does not include a workspace path.".to_string(),
        );
    }

    let timezone = config
        .schedules
        .today
        .timezone
        .parse::<chrono_tz::Tz>()
        .unwrap_or(chrono_tz::America::New_York);
    let date = Utc::now()
        .with_timezone(&timezone)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    Ok(json!({
        "schemaVersion": DAILY_BRIEFING_SCHEMA_VERSION,
        "date": date,
        "workspaceId": workspace_id,
    }))
}

pub fn present_daily_briefing_response(
    briefing: Value,
    rendered_provenance: Value,
    invocation_id: Option<&str>,
) -> Value {
    let answer = build_daily_briefing_answer(&briefing);
    let provenance = project_daily_briefing_provenance(&briefing, rendered_provenance);
    let status = daily_briefing_status(&briefing);
    let provenance_handle = invocation_id.map(|id| {
        json!({
            "invocationId": id,
            "invocation_id": id,
            "detailAvailable": false,
            "detailTool": Value::Null,
            "detailParams": {
                "invocation_id": id
            }
        })
    });

    json!({
        "schemaVersion": DAILY_BRIEFING_RESPONSE_SCHEMA_VERSION,
        "toolName": DAILY_BRIEFING_TOOL_NAME,
        "surface": DAILY_BRIEFING_TOOL_NAME,
        "producer": DAILY_BRIEFING_ABILITY_NAME,
        "status": status,
        "invocationId": invocation_id,
        "provenanceHandle": provenance_handle,
        "date": briefing.get("date").cloned().unwrap_or(Value::Null),
        "answer": answer,
        "schedule": {
            "currentMeeting": briefing.get("currentMeeting").cloned().unwrap_or(Value::Null),
            "nextMeeting": briefing.get("nextMeeting").cloned().unwrap_or(Value::Null),
            "upcomingMeetings": briefing.get("upcomingMeetings").cloned().unwrap_or_else(|| json!({ "items": [] })),
        },
        "state": briefing.get("state").cloned().unwrap_or(Value::Null),
        "watchProposals": briefing.get("watchProposals").cloned().unwrap_or_else(|| json!([])),
        "trust": briefing.get("trustSummary").cloned().unwrap_or(Value::Null),
        "sensitivity": briefing.get("sensitivity").cloned().unwrap_or(Value::Null),
        "sourceAsofInputs": briefing.get("sourceAsofInputs").cloned().unwrap_or_else(|| json!([])),
        "provenance": provenance,
        "briefing": briefing,
        "sourceEnvelope": {
            "schemaVersion": DAILY_BRIEFING_SCHEMA_VERSION,
            "rawEnvelopeIncluded": false,
        },
    })
}

fn daily_briefing_unavailable_response(reason: &str) -> Value {
    json!({
        "schemaVersion": DAILY_BRIEFING_RESPONSE_SCHEMA_VERSION,
        "toolName": DAILY_BRIEFING_TOOL_NAME,
        "surface": DAILY_BRIEFING_TOOL_NAME,
        "producer": DAILY_BRIEFING_ABILITY_NAME,
        "status": "unavailable",
        "invocationId": Value::Null,
        "provenanceHandle": Value::Null,
        "date": Value::Null,
        "answer": format!("DailyOS daily briefing context is unavailable: {reason}"),
        "schedule": {
            "currentMeeting": Value::Null,
            "nextMeeting": Value::Null,
            "upcomingMeetings": { "items": [] },
        },
        "state": {
            "availability": { "kind": "empty", "reason": "workspace_unknown" },
            "freshness": { "kind": "stale", "reason": "source_asof_older_than_threshold" },
            "integrity": { "kind": "clean" },
            "advisories": [
                { "kind": "partial_read_failure", "advisory": reason }
            ],
        },
        "watchProposals": [],
        "trust": {
            "aggregateBand": "unscored",
            "likelyCurrentCount": 0,
            "useWithCautionCount": 0,
            "needsVerificationCount": 0,
        },
        "sensitivity": Value::Null,
        "sourceAsofInputs": [],
        "provenance": {
            "sources": [],
            "redactionApplied": false,
            "rawClaimIdsIncluded": false,
        },
        "briefing": Value::Null,
        "sourceEnvelope": {
            "schemaVersion": Value::Null,
            "rawEnvelopeIncluded": false,
        },
    })
}

fn daily_briefing_status(briefing: &Value) -> &'static str {
    match string_at(briefing, "/state/availability/kind") {
        Some("available") => "ready",
        Some("empty") => "empty",
        Some("auth_locked") => "unavailable",
        _ => "unknown",
    }
}

fn build_daily_briefing_answer(briefing: &Value) -> String {
    let date = string_at(briefing, "/date").unwrap_or("today");
    match string_at(briefing, "/state/availability/kind") {
        Some("empty") => {
            let reason = string_at(briefing, "/state/availability/reason")
                .map(humanize_token)
                .unwrap_or_else(|| "No briefing content".to_string());
            return format!("DailyOS daily briefing for {date}: {reason}.");
        }
        Some("auth_locked") => {
            return format!(
                "DailyOS daily briefing for {date} is unavailable because the workspace is locked."
            );
        }
        _ => {}
    }

    let mut lines = vec![format!("DailyOS daily briefing for {date}.")];

    if let Some(line) = meeting_line("Current meeting", briefing.get("currentMeeting")) {
        lines.push(String::new());
        lines.push(line);
    }

    if let Some(line) = meeting_line("Next meeting", briefing.get("nextMeeting")) {
        lines.push(String::new());
        lines.push(line);
    }

    let upcoming = briefing
        .pointer("/upcomingMeetings/items")
        .and_then(Value::as_array)
        .map(|items| items.as_slice())
        .unwrap_or(&[]);
    if !upcoming.is_empty() {
        lines.push(String::new());
        lines.push("Upcoming:".to_string());
        for meeting in upcoming.iter().take(MAX_UPCOMING_MEETINGS_IN_ANSWER) {
            if let Some(line) = meeting_bullet(meeting) {
                lines.push(format!("- {line}"));
            }
        }
    }

    let freshness = string_at(briefing, "/state/freshness/kind")
        .map(humanize_token)
        .unwrap_or_else(|| "unknown".to_string());
    let trust = string_at(briefing, "/trustSummary/aggregateBand")
        .map(humanize_token)
        .unwrap_or_else(|| "unscored".to_string());
    let source_count = briefing
        .pointer("/provenance/sources")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    lines.push(String::new());
    lines.push(format!(
        "Briefing posture: {freshness}; trust {trust}; {source_count} source(s) represented."
    ));

    if let Some(advisories) = briefing
        .pointer("/state/advisories")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
    {
        lines.push(String::new());
        lines.push("Advisories:".to_string());
        for advisory in advisories.iter().take(3) {
            if let Some(text) = advisory_text(advisory) {
                lines.push(format!("- {text}"));
            }
        }
    }

    lines.join("\n")
}

fn meeting_line(label: &str, meeting: Option<&Value>) -> Option<String> {
    meeting
        .filter(|value| !value.is_null())
        .and_then(meeting_bullet)
        .map(|line| format!("{label}: {line}"))
}

fn meeting_bullet(meeting: &Value) -> Option<String> {
    let title = string_at(meeting, "/title")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;
    let starts_at = string_at(meeting, "/startsAt")
        .map(|value| format!(" at {value}"))
        .unwrap_or_default();
    let prep_status = string_at(meeting, "/prepStatus")
        .map(humanize_token)
        .unwrap_or_else(|| "unknown prep status".to_string());
    let linked = match (
        string_at(meeting, "/linkedEntityType"),
        string_at(meeting, "/linkedEntityId"),
    ) {
        (Some(kind), Some(id)) => format!("; linked {kind}:{id}"),
        _ => String::new(),
    };
    Some(format!("{title}{starts_at}; prep {prep_status}{linked}"))
}

fn advisory_text(advisory: &Value) -> Option<String> {
    string_at(advisory, "/summary")
        .or_else(|| string_at(advisory, "/advisory"))
        .or_else(|| string_at(advisory, "/message"))
        .map(compact_text)
        .filter(|value| !value.is_empty())
}

fn project_daily_briefing_provenance(briefing: &Value, rendered_provenance: Value) -> Value {
    let sources = briefing
        .pointer("/provenance/sources")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .map(|(index, source)| {
                    json!({
                        "id": format!("source_{}", index + 1),
                        "label": source.get("label").cloned().unwrap_or(Value::Null),
                        "sourceType": source.get("sourceType").cloned().unwrap_or(Value::Null),
                        "asOf": source.get("asOf").cloned().unwrap_or(Value::Null),
                        "redacted": source.get("redacted").cloned().unwrap_or(Value::Bool(false)),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let redaction_applied = rendered_provenance
        .get("redactionApplied")
        .or_else(|| rendered_provenance.get("redaction_applied"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    json!({
        "sources": sources,
        "redactionApplied": redaction_applied,
        "rawClaimIdsIncluded": false,
    })
}

pub fn present_meeting_briefing_response(snapshot: PrepareMeetingContextSnapshot) -> Value {
    let attendees = snapshot
        .attendees
        .iter()
        .map(project_meeting_attendee)
        .collect::<Vec<_>>();
    let subjects = snapshot
        .subjects
        .iter()
        .map(project_meeting_subject)
        .collect::<Vec<_>>();
    let evidence = snapshot
        .claims
        .iter()
        .take(MAX_MEETING_EVIDENCE_ITEMS)
        .map(project_meeting_claim)
        .collect::<Vec<_>>();
    let linear_issue_changes = snapshot
        .linear_issue_changes
        .iter()
        .take(MAX_LINEAR_CHANGES)
        .map(project_linear_issue_change)
        .collect::<Vec<_>>();
    let trust = meeting_trust_summary(&snapshot);
    let provenance = meeting_provenance_summary(&snapshot);
    let answer = build_meeting_briefing_answer(
        &snapshot,
        attendees.len(),
        evidence.len(),
        linear_issue_changes.len(),
        provenance
            .get("sources")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    );

    json!({
        "schemaVersion": MEETING_BRIEFING_RESPONSE_SCHEMA_VERSION,
        "toolName": MEETING_BRIEFING_TOOL_NAME,
        "surface": MEETING_BRIEFING_TOOL_NAME,
        "producer": MEETING_BRIEFING_PRODUCER,
        "status": "ready",
        "meeting": {
            "meetingId": snapshot.meeting.id,
            "title": snapshot.meeting.title,
            "startsAt": snapshot.meeting.starts_at,
            "endsAt": snapshot.meeting.ends_at,
        },
        "answer": answer,
        "attendees": attendees,
        "subjects": subjects,
        "evidence": evidence,
        "linearIssueChanges": linear_issue_changes,
        "sectionStates": {
            "attendees": section_state_for_count(snapshot.attendees.len()),
            "subjects": section_state_for_count(snapshot.subjects.len()),
            "evidence": section_state_for_count(snapshot.claims.len()),
            "linearIssueChanges": section_state_for_count(snapshot.linear_issue_changes.len()),
        },
        "trust": trust,
        "provenance": provenance,
        "sourceEnvelope": {
            "schemaVersion": Value::Null,
            "rawEnvelopeIncluded": false,
        },
    })
}

fn meeting_briefing_not_found_response(meeting_id: &str) -> Value {
    meeting_briefing_empty_response(
        meeting_id,
        "not_found",
        "DailyOS could not find a renderable meeting briefing for the requested meeting.",
    )
}

fn meeting_briefing_unavailable_response(meeting_id: &str) -> Value {
    meeting_briefing_empty_response(
        meeting_id,
        "unavailable",
        "DailyOS meeting briefing context is temporarily unavailable.",
    )
}

fn meeting_briefing_empty_response(meeting_id: &str, status: &str, answer: &str) -> Value {
    json!({
        "schemaVersion": MEETING_BRIEFING_RESPONSE_SCHEMA_VERSION,
        "toolName": MEETING_BRIEFING_TOOL_NAME,
        "surface": MEETING_BRIEFING_TOOL_NAME,
        "producer": MEETING_BRIEFING_PRODUCER,
        "status": status,
        "meeting": {
            "meetingId": meeting_id,
            "title": Value::Null,
            "startsAt": Value::Null,
            "endsAt": Value::Null,
        },
        "answer": answer,
        "attendees": [],
        "subjects": [],
        "evidence": [],
        "linearIssueChanges": [],
        "sectionStates": {
            "attendees": { "kind": "empty", "reason": status },
            "subjects": { "kind": "empty", "reason": status },
            "evidence": { "kind": "empty", "reason": status },
            "linearIssueChanges": { "kind": "empty", "reason": status },
        },
        "trust": {
            "aggregateBand": "unscored",
            "likelyCurrentCount": 0,
            "useWithCautionCount": 0,
            "needsVerificationCount": 0,
            "unscoredCount": 0,
        },
        "provenance": {
            "sources": [],
            "rawClaimIdsIncluded": false,
            "rawProvenanceIncluded": false,
        },
        "sourceEnvelope": {
            "schemaVersion": Value::Null,
            "rawEnvelopeIncluded": false,
        },
    })
}

fn build_meeting_briefing_answer(
    snapshot: &PrepareMeetingContextSnapshot,
    attendee_count: usize,
    evidence_count: usize,
    linear_change_count: usize,
    source_count: usize,
) -> String {
    let mut lines = vec![format!(
        "DailyOS meeting briefing for {}.",
        compact_text(&snapshot.meeting.title)
    )];
    if let Some(starts_at) = snapshot.meeting.starts_at.as_deref() {
        lines.push(format!("Scheduled start: {starts_at}."));
    }
    lines.push(format!(
        "Context: {attendee_count} attendee(s), {} linked subject(s), {evidence_count} evidence item(s), {linear_change_count} issue change(s).",
        snapshot.subjects.len()
    ));

    if !snapshot.claims.is_empty() {
        lines.push(String::new());
        lines.push("Evidence:".to_string());
        for claim in snapshot.claims.iter().take(5) {
            lines.push(format!("- {}", compact_text(&claim.text)));
        }
    }

    if !snapshot.linear_issue_changes.is_empty() {
        lines.push(String::new());
        lines.push("Recent issue changes:".to_string());
        for change in snapshot.linear_issue_changes.iter().take(3) {
            let label = change
                .identifier
                .as_deref()
                .unwrap_or(change.issue_id.as_str());
            let title = change
                .title
                .as_deref()
                .map(compact_text)
                .filter(|value| !value.is_empty())
                .map(|value| format!(" — {value}"))
                .unwrap_or_default();
            let state = change
                .to_state
                .as_deref()
                .or(change.current_state_name.as_deref())
                .map(|value| format!(" ({value})"))
                .unwrap_or_default();
            lines.push(format!("- {label}{title}{state}"));
        }
    }

    lines.push(String::new());
    lines.push(format!(
        "Provenance: {source_count} source(s) represented; raw claim ids and raw provenance are not included."
    ));
    lines.join("\n")
}

fn project_meeting_attendee(attendee: &PrepareMeetingAttendeeSnapshot) -> Value {
    json!({
        "name": attendee.name.as_str(),
        "personId": attendee.person_id.as_deref(),
        "accountId": attendee.account_id.as_deref(),
        "domain": attendee.domain.as_deref(),
    })
}

fn project_meeting_subject(subject: &PrepareMeetingSubjectSnapshot) -> Value {
    json!({
        "kind": subject.kind.as_str(),
        "id": subject.id.as_str(),
        "displayName": subject.display_name.as_str(),
    })
}

fn project_meeting_claim(claim: &crate::db::claims::IntelligenceClaim) -> Value {
    json!({
        "text": compact_text(&claim.text),
        "subject": claim_subject_summary(claim),
        "claimType": claim.claim_type.as_str(),
        "fieldPath": claim.field_path.as_deref(),
        "sourceAsOf": claim.source_asof.as_deref(),
        "observedAt": claim.observed_at.as_str(),
        "trustBand": crate::abilities::provenance::trust::claim_trust_band_from_score(claim.trust_score),
        "sensitivity": claim.sensitivity.clone(),
        "verificationState": claim.verification_state.clone(),
        "source": {
            "sourceType": claim.data_source.as_str(),
            "asOf": claim.source_asof.as_deref(),
        },
    })
}

fn claim_subject_summary(claim: &crate::db::claims::IntelligenceClaim) -> Value {
    let Ok(subject) = serde_json::from_str::<Value>(&claim.subject_ref) else {
        return Value::Null;
    };
    let kind = string_at(&subject, "/kind")
        .or_else(|| string_at(&subject, "/type"))
        .or_else(|| string_at(&subject, "/entity_type"));
    let id = string_at(&subject, "/id").or_else(|| string_at(&subject, "/entity_id"));
    json!({
        "kind": kind,
        "id": id,
    })
}

fn project_linear_issue_change(change: &PrepareMeetingLinearIssueChangeSnapshot) -> Value {
    json!({
        "signalId": change.signal_id.as_str(),
        "issueId": change.issue_id.as_str(),
        "identifier": change.identifier.as_deref(),
        "title": change.title.as_deref().map(compact_text),
        "subject": project_meeting_subject(&change.subject),
        "signalType": change.signal_type.as_str(),
        "fromState": change.from_state.as_deref(),
        "toState": change.to_state.as_deref(),
        "currentStateType": change.current_state_type.as_deref(),
        "currentStateName": change.current_state_name.as_deref(),
        "sourceAsOf": change.source_asof.as_str(),
    })
}

fn section_state_for_count(count: usize) -> Value {
    if count == 0 {
        json!({ "kind": "empty", "reason": "no_renderable_content" })
    } else {
        json!({ "kind": "available", "count": count })
    }
}

fn meeting_trust_summary(snapshot: &PrepareMeetingContextSnapshot) -> Value {
    let mut likely_current = 0u32;
    let mut use_with_caution = 0u32;
    let mut needs_verification = 0u32;
    let mut unscored = 0u32;
    for claim in &snapshot.claims {
        match crate::abilities::provenance::trust::claim_trust_band_from_score(claim.trust_score) {
            crate::abilities::trust::TrustBand::LikelyCurrent => likely_current += 1,
            crate::abilities::trust::TrustBand::UseWithCaution => use_with_caution += 1,
            crate::abilities::trust::TrustBand::NeedsVerification => needs_verification += 1,
            crate::abilities::trust::TrustBand::Unscored => unscored += 1,
        }
    }
    let aggregate_band = if needs_verification > 0 {
        "needs_verification"
    } else if use_with_caution > 0 {
        "use_with_caution"
    } else if likely_current > 0 {
        "likely_current"
    } else {
        "unscored"
    };
    json!({
        "aggregateBand": aggregate_band,
        "likelyCurrentCount": likely_current,
        "useWithCautionCount": use_with_caution,
        "needsVerificationCount": needs_verification,
        "unscoredCount": unscored,
    })
}

fn meeting_provenance_summary(snapshot: &PrepareMeetingContextSnapshot) -> Value {
    let mut source_ids = BTreeMap::new();
    let mut sources = Vec::new();
    for claim in &snapshot.claims {
        let source_key = format!(
            "{}|{}",
            claim.data_source,
            claim.source_asof.as_deref().unwrap_or("")
        );
        if source_ids.contains_key(&source_key) {
            continue;
        }
        let display_id = format!("source_{}", source_ids.len() + 1);
        source_ids.insert(source_key, display_id.clone());
        sources.push(json!({
            "id": display_id,
            "label": humanize_token(&claim.data_source),
            "sourceType": claim.data_source.as_str(),
            "asOf": claim.source_asof.as_deref(),
            "redacted": false,
        }));
    }
    json!({
        "sources": sources,
        "rawClaimIdsIncluded": false,
        "rawProvenanceIncluded": false,
    })
}

fn map_invoke_error(err: AbilityInvokeError) -> ToolError {
    eprintln!("mcp_v2 dailyos.read.daily_briefing invoke failed: {err:?}");

    let trace_id = match &err {
        AbilityInvokeError::Surface(_) => "surface",
        AbilityInvokeError::Ability(_) => "ability",
        AbilityInvokeError::InvalidEnvelope => "invalid_envelope",
        AbilityInvokeError::ProvenanceTooLarge => "provenance_too_large",
        AbilityInvokeError::ProvenanceSerialize(_) => "provenance_serialize",
    };
    ToolError::Internal {
        trace_id: trace_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::claims::{
        ClaimSensitivity, ClaimState, ClaimVerificationState, IntelligenceClaim, SurfacingState,
        TemporalScope,
    };

    #[test]
    fn presenter_builds_readable_daily_briefing_answer() {
        let briefing = json!({
            "schemaVersion": 1,
            "date": "2026-05-26",
            "state": {
                "availability": { "kind": "available" },
                "freshness": { "kind": "needs_preparation", "meetingIds": ["meeting-2"] },
                "integrity": { "kind": "clean" },
                "advisories": []
            },
            "currentMeeting": {
                "meetingId": "meeting-1",
                "title": "Morning planning",
                "startsAt": "2026-05-26T09:00:00-04:00",
                "prepStatus": "ready",
                "linkedEntityType": "account",
                "linkedEntityId": "account-1"
            },
            "nextMeeting": Value::Null,
            "upcomingMeetings": {
                "items": [
                    {
                        "meetingId": "meeting-2",
                        "title": "Customer sync",
                        "startsAt": "2026-05-26T13:00:00-04:00",
                        "prepStatus": "prep_needed"
                    }
                ],
                "nextCursor": Value::Null,
                "totalHint": 1,
                "cursorState": "complete"
            },
            "watchProposals": [],
            "trustSummary": {
                "aggregateBand": "likely_current",
                "likelyCurrentCount": 3,
                "useWithCautionCount": 0,
                "needsVerificationCount": 0
            },
            "provenance": {
                "sources": [
                    { "id": "source-raw", "label": "Calendar", "sourceType": "calendar", "asOf": "2026-05-26T12:00:00Z", "redacted": false }
                ]
            },
            "sensitivity": "Internal",
            "sourceAsofInputs": []
        });

        let payload = present_daily_briefing_response(
            briefing,
            json!({ "redactionApplied": false }),
            Some("invocation-1"),
        );

        assert_eq!(payload["toolName"], DAILY_BRIEFING_TOOL_NAME);
        assert_eq!(payload["status"], "ready");
        assert!(payload["answer"]
            .as_str()
            .expect("answer")
            .contains("Current meeting: Morning planning"));
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);
        assert_eq!(payload["sourceEnvelope"]["rawEnvelopeIncluded"], false);
    }

    #[test]
    fn unavailable_response_is_success_shaped() {
        let payload = daily_briefing_unavailable_response("config missing");

        assert_eq!(payload["status"], "unavailable");
        assert!(payload["answer"]
            .as_str()
            .expect("answer")
            .contains("config missing"));
        assert_eq!(payload["schedule"]["upcomingMeetings"]["items"], json!([]));
    }

    #[test]
    fn extract_meeting_id_accepts_snake_and_camel_case() {
        assert_eq!(
            extract_meeting_id(&json!({ "meeting_id": "  meeting-1 " })).unwrap(),
            "meeting-1"
        );
        assert_eq!(
            extract_meeting_id(&json!({ "meetingId": "meeting-2" })).unwrap(),
            "meeting-2"
        );
    }

    #[test]
    fn extract_meeting_id_rejects_missing_empty_and_non_string() {
        assert!(matches!(
            extract_meeting_id(&json!({})),
            Err(ToolError::BadParams { .. })
        ));
        assert!(matches!(
            extract_meeting_id(&json!({ "meeting_id": " " })),
            Err(ToolError::BadParams { .. })
        ));
        assert!(matches!(
            extract_meeting_id(&json!({ "meeting_id": 15 })),
            Err(ToolError::BadParams { .. })
        ));
    }

    #[test]
    fn meeting_presenter_omits_attendee_email_and_raw_claim_ids() {
        let snapshot = PrepareMeetingContextSnapshot {
            meeting: crate::services::context::PrepareMeetingSnapshot {
                id: "meeting-1".to_string(),
                title: "Customer planning".to_string(),
                starts_at: Some("2026-05-26T13:00:00Z".to_string()),
                ends_at: Some("2026-05-26T13:30:00Z".to_string()),
                attendees_raw: Some("[\"taylor@example.com\"]".to_string()),
            },
            attendees: vec![PrepareMeetingAttendeeSnapshot {
                name: "Taylor Example".to_string(),
                email: Some("taylor@example.com".to_string()),
                person_id: Some("person-1".to_string()),
                account_id: Some("account-1".to_string()),
                domain: Some("example.com".to_string()),
            }],
            subjects: vec![PrepareMeetingSubjectSnapshot {
                kind: "account".to_string(),
                id: "account-1".to_string(),
                display_name: "Example Account".to_string(),
            }],
            claims: vec![test_claim(
                "Example Account is planning a Q3 rollout.",
                Some(0.92),
            )],
            linear_issue_changes: vec![PrepareMeetingLinearIssueChangeSnapshot {
                signal_id: "signal-1".to_string(),
                issue_id: "issue-1".to_string(),
                identifier: Some("DOS-1".to_string()),
                title: Some("Resolve rollout blocker".to_string()),
                url: Some("https://linear.example/issue/DOS-1".to_string()),
                subject: PrepareMeetingSubjectSnapshot {
                    kind: "account".to_string(),
                    id: "account-1".to_string(),
                    display_name: "Example Account".to_string(),
                },
                signal_type: "linear_issue_state_changed".to_string(),
                from_state: Some("Todo".to_string()),
                to_state: Some("In Progress".to_string()),
                current_state_type: Some("started".to_string()),
                current_state_name: Some("In Progress".to_string()),
                source_asof: "2026-05-26T12:00:00Z".to_string(),
            }],
        };

        let payload = present_meeting_briefing_response(snapshot);
        let serialized = serde_json::to_string(&payload).expect("payload serializes");

        assert_eq!(payload["toolName"], MEETING_BRIEFING_TOOL_NAME);
        assert_eq!(payload["status"], "ready");
        assert!(payload["answer"]
            .as_str()
            .expect("answer")
            .contains("Customer planning"));
        assert!(!serialized.contains("taylor@example.com"));
        assert!(!serialized.contains("claim-1"));
        assert!(!serialized.contains("linear.example"));
        assert_eq!(payload["provenance"]["rawClaimIdsIncluded"], false);
        assert_eq!(payload["sourceEnvelope"]["rawEnvelopeIncluded"], false);
    }

    fn test_claim(text: &str, trust_score: Option<f64>) -> IntelligenceClaim {
        IntelligenceClaim {
            id: "claim-1".to_string(),
            claim_version: 1,
            subject_ref: json!({ "kind": "account", "id": "account-1" }).to_string(),
            claim_type: "account_status".to_string(),
            field_path: Some("commercial.status".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: "dedup-1".to_string(),
            item_hash: None,
            actor: "system:test".to_string(),
            data_source: "Salesforce".to_string(),
            source_ref: Some("meeting-1".to_string()),
            source_asof: Some("2026-05-26T12:00:00Z".to_string()),
            observed_at: "2026-05-26T12:00:00Z".to_string(),
            created_at: "2026-05-26T12:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score,
            trust_computed_at: Some("2026-05-26T12:00:00Z".to_string()),
            trust_version: Some(1),
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }
}
