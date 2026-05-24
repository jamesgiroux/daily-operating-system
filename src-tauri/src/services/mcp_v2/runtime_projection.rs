//! Shared MCP projection over abilities-runtime entity intelligence envelopes.
//!
//! This is a surface adapter, not an authority source: it only projects
//! rendered, policy-filtered values already present in the runtime envelope.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

const MAX_ASSESSMENT_ITEMS: usize = 8;

pub(crate) struct RuntimeEvidenceProjection {
    pub(crate) provenance: Value,
    pub(crate) facts: Vec<Value>,
    pub(crate) open_loops: Vec<Value>,
    pub(crate) relationships: Vec<Value>,
    pub(crate) touchpoints: Vec<Value>,
    pub(crate) record_entries: Vec<Value>,
    pub(crate) priorities: Vec<Value>,
    pub(crate) caveats: Vec<Value>,
    pub(crate) section_states: Value,
    pub(crate) truncation: Value,
    pub(crate) status: &'static str,
}

pub(crate) fn project_runtime_evidence(envelope: &Value) -> RuntimeEvidenceProjection {
    let (provenance, source_id_map) = build_provenance_summary(envelope);
    let facts = collect_fact_summaries(envelope, &source_id_map);
    let open_loops = collect_open_loop_summaries(envelope, &source_id_map);
    let relationships = collect_relationship_summaries(envelope, &source_id_map);
    let touchpoints = collect_touchpoint_summaries(envelope, &source_id_map);
    let record_entries = collect_record_entry_summaries(envelope, &source_id_map);
    let priorities = collect_priority_summaries(&facts, &open_loops, &record_entries);
    let mut caveats = collect_caveats(envelope);
    if priorities.is_empty() {
        caveats.push(json!({
            "section": "priorities",
            "text": "No explicit priority claims were present; use facts, open loops, touchpoints, and record entries as evidence candidates."
        }));
    }
    let section_states = envelope
        .get("sections")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let truncation = build_truncation_summary(
        envelope,
        &facts,
        &open_loops,
        &relationships,
        &touchpoints,
        &record_entries,
    );
    let status = response_status(
        &facts,
        &open_loops,
        &relationships,
        &touchpoints,
        &record_entries,
        envelope,
    );

    RuntimeEvidenceProjection {
        provenance,
        facts,
        open_loops,
        relationships,
        touchpoints,
        record_entries,
        priorities,
        caveats,
        section_states,
        truncation,
        status,
    }
}

fn build_provenance_summary(envelope: &Value) -> (Value, BTreeMap<String, String>) {
    let mut source_id_map = BTreeMap::new();
    let mut sources = Vec::new();

    if let Some(raw_sources) = envelope
        .pointer("/provenance/sources")
        .and_then(Value::as_array)
    {
        for (index, source) in raw_sources.iter().enumerate() {
            let display_id = format!("source_{}", index + 1);
            if let Some(raw_id) = source.get("id").and_then(Value::as_str) {
                source_id_map.insert(raw_id.to_string(), display_id.clone());
            }

            let mut projected = Map::new();
            projected.insert("id".to_string(), Value::String(display_id));
            insert_string_or_clone(&mut projected, "label", source, "/label");
            insert_string_or_clone(&mut projected, "sourceType", source, "/sourceType");
            insert_string_or_clone(&mut projected, "asOf", source, "/asOf");
            if let Some(redacted) = source.get("redacted").and_then(Value::as_bool) {
                projected.insert("redacted".to_string(), Value::Bool(redacted));
            }
            sources.push(Value::Object(projected));
        }
    }

    let redaction_applied = envelope
        .pointer("/provenance/redactionApplied")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    (
        json!({
            "sources": sources,
            "redactionApplied": redaction_applied,
            "rawClaimIdsIncluded": false,
        }),
        source_id_map,
    )
}

fn collect_fact_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/facts/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| fact_summary(item, source_id_map))
                .take(MAX_ASSESSMENT_ITEMS)
                .collect()
        })
        .unwrap_or_default()
}

fn fact_summary(item: &Value, source_id_map: &BTreeMap<String, String>) -> Option<Value> {
    let text = string_at(item, "/renderedText/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("text".to_string(), Value::String(text));
    insert_string_or_clone(&mut summary, "fieldPath", item, "/fieldPath");
    insert_string_or_clone(&mut summary, "claimType", item, "/claimType");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_string_or_clone(&mut summary, "sourceAsOf", item, "/sourceAsof");
    insert_string_or_clone(&mut summary, "sensitivity", item, "/sensitivity");
    insert_string_or_clone(&mut summary, "lifecycleState", item, "/lifecycleState");
    insert_string_or_clone(
        &mut summary,
        "verificationState",
        item,
        "/verificationState",
    );
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn collect_open_loop_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/openLoops/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| open_loop_summary(item, source_id_map))
                .take(MAX_ASSESSMENT_ITEMS)
                .collect()
        })
        .unwrap_or_default()
}

fn open_loop_summary(item: &Value, source_id_map: &BTreeMap<String, String>) -> Option<Value> {
    let open_loop = item.get("openLoop").unwrap_or(item);
    let description = string_at(open_loop, "/description")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("description".to_string(), Value::String(description));
    insert_string_or_clone(&mut summary, "loopKind", open_loop, "/loop_kind");
    insert_string_or_clone(&mut summary, "owner", open_loop, "/owner");
    insert_string_or_clone(&mut summary, "dueDate", open_loop, "/due_date");
    insert_string_or_clone(&mut summary, "status", open_loop, "/status");
    insert_string_or_clone(&mut summary, "sourceAsOf", open_loop, "/source_asof");
    insert_string_or_clone(&mut summary, "claimType", open_loop, "/claim_type");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn collect_relationship_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/relationships/items")
        .and_then(Value::as_array)
        .map(|bundles| {
            let mut summaries = Vec::new();
            for bundle in bundles {
                if let Some(participants) = bundle
                    .pointer("/participants/items")
                    .and_then(Value::as_array)
                {
                    summaries.extend(
                        participants.iter().filter_map(|item| {
                            relationship_participant_summary(item, source_id_map)
                        }),
                    );
                }
                if let Some(edges) = bundle.pointer("/edges/items").and_then(Value::as_array) {
                    summaries.extend(
                        edges
                            .iter()
                            .filter_map(|item| relationship_edge_summary(item, source_id_map)),
                    );
                }
            }
            summaries.truncate(MAX_ASSESSMENT_ITEMS);
            summaries
        })
        .unwrap_or_default()
}

fn relationship_participant_summary(
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Option<Value> {
    let display_label = string_at(item, "/displayLabel/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("kind".to_string(), Value::String("participant".to_string()));
    summary.insert("displayLabel".to_string(), Value::String(display_label));
    if let Some(role) = string_at(item, "/role/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("role".to_string(), Value::String(role));
    }
    if let Some(relationship) = string_at(item, "/relationship/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("relationship".to_string(), Value::String(relationship));
    }
    if let Some(count) = item
        .get("normalizedTouchpointCount")
        .and_then(Value::as_u64)
    {
        summary.insert(
            "normalizedTouchpointCount".to_string(),
            Value::Number(count.into()),
        );
    }
    insert_string_or_clone(&mut summary, "lastSeenAt", item, "/lastSeenAt");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn relationship_edge_summary(
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Option<Value> {
    let relationship = string_at(item, "/edgeType")
        .map(compact_text)
        .filter(|value| !value.is_empty())
        .map(|value| relationship_label_for_edge_type(&value))
        .unwrap_or_else(|| "Relationship".to_string());
    let mut summary = Map::new();
    summary.insert("kind".to_string(), Value::String("edge".to_string()));
    summary.insert("relationship".to_string(), Value::String(relationship));
    if let Some(display_label) = string_at(item, "/relatedDisplayLabel/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())
    {
        summary.insert("displayLabel".to_string(), Value::String(display_label));
    }
    if let Some(related_entity_type) = relationship_subject_type(item.get("relatedSubjectRef")) {
        summary.insert(
            "relatedEntityType".to_string(),
            Value::String(related_entity_type.to_string()),
        );
    }
    insert_string_or_clone(&mut summary, "inclusionReason", item, "/inclusionReason");
    insert_string_or_clone(&mut summary, "observedAt", item, "/observedAt");
    insert_string_or_clone(&mut summary, "sourceAsOf", item, "/sourceAsof");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn relationship_label_for_edge_type(edge_type: &str) -> String {
    match edge_type {
        "hierarchy_parent" => "Parent relationship",
        "hierarchy_child" => "Child relationship",
        "stakeholder" => "Stakeholder",
        "member" => "Member",
        "meeting_subject" => "Meeting subject",
        "meeting_attendance" => "Meeting attendance",
        "meeting_link" => "Meeting link",
        "person_relationship" => "Person relationship",
        _ => "Relationship",
    }
    .to_string()
}

fn relationship_subject_type(subject_ref: Option<&Value>) -> Option<&'static str> {
    subject_ref.and_then(|value| {
        if let Some(value) = value.as_str() {
            return value
                .split_once(':')
                .map(|(entity_type, _)| entity_type)
                .or(Some(value))
                .and_then(known_entity_type);
        }
        value
            .as_object()
            .and_then(|object| object.keys().find_map(|key| known_entity_type(key)))
    })
}

fn known_entity_type(entity_type: &str) -> Option<&'static str> {
    match entity_type {
        "account" => Some("account"),
        "project" => Some("project"),
        "person" => Some("person"),
        "meeting" => Some("meeting"),
        _ => None,
    }
}

fn collect_touchpoint_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    let mut summaries = Vec::new();
    if let Some(bundles) = envelope
        .pointer("/touchpoints/items")
        .and_then(Value::as_array)
    {
        for bundle in bundles {
            summaries.extend(collect_touchpoint_side(
                bundle,
                "upcoming",
                "/upcoming/items",
                source_id_map,
            ));
            summaries.extend(collect_touchpoint_side(
                bundle,
                "recent",
                "/recent/items",
                source_id_map,
            ));
        }
    }
    summaries.truncate(MAX_ASSESSMENT_ITEMS);
    summaries
}

fn collect_touchpoint_side(
    bundle: &Value,
    timing: &str,
    pointer: &str,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    bundle
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| touchpoint_summary(item, timing, source_id_map))
                .collect()
        })
        .unwrap_or_default()
}

fn touchpoint_summary(
    item: &Value,
    timing: &str,
    source_id_map: &BTreeMap<String, String>,
) -> Option<Value> {
    let when = string_at(item, "/when")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;
    let mut summary = Map::new();
    summary.insert("timing".to_string(), Value::String(timing.to_string()));
    summary.insert("when".to_string(), Value::String(when));
    insert_string_or_clone(&mut summary, "kind", item, "/kind");
    insert_string_or_clone(&mut summary, "inclusionReason", item, "/inclusionReason");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn collect_record_entry_summaries(
    envelope: &Value,
    source_id_map: &BTreeMap<String, String>,
) -> Vec<Value> {
    envelope
        .pointer("/recordEntries/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| record_entry_summary(item, source_id_map))
                .take(MAX_ASSESSMENT_ITEMS)
                .collect()
        })
        .unwrap_or_default()
}

fn record_entry_summary(item: &Value, source_id_map: &BTreeMap<String, String>) -> Option<Value> {
    let text = string_at(item, "/renderedText/text")
        .map(compact_text)
        .filter(|value| !value.is_empty())?;

    let mut summary = Map::new();
    summary.insert("text".to_string(), Value::String(text));
    insert_string_or_clone(&mut summary, "claimType", item, "/claimType");
    insert_string_or_clone(&mut summary, "recordedAt", item, "/recordedAt");
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "sensitivity", item, "/sensitivity");
    insert_source_refs(&mut summary, item, source_id_map);
    Some(Value::Object(summary))
}

fn collect_priority_summaries(
    facts: &[Value],
    open_loops: &[Value],
    record_entries: &[Value],
) -> Vec<Value> {
    let mut priorities = facts
        .iter()
        .filter(|item| is_priority_like(item))
        .filter_map(|item| priority_from_text_item(item, "/text", "claim"))
        .chain(
            record_entries
                .iter()
                .filter(|item| is_priority_like(item))
                .filter_map(|item| priority_from_text_item(item, "/text", "record_entry")),
        )
        .collect::<Vec<_>>();

    if priorities.is_empty() {
        priorities.extend(
            open_loops
                .iter()
                .filter_map(|item| priority_from_text_item(item, "/description", "open_loop"))
                .take(3),
        );
    }

    priorities.truncate(3);
    priorities
}

fn is_priority_like(item: &Value) -> bool {
    ["/claimType", "/fieldPath", "/text"]
        .into_iter()
        .filter_map(|pointer| string_at(item, pointer))
        .any(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized.contains("priority")
                || normalized.contains("next step")
                || normalized.contains("focus")
                || normalized.contains("recommend")
        })
}

fn priority_from_text_item(item: &Value, text_pointer: &str, basis: &str) -> Option<Value> {
    let text = string_at(item, text_pointer)
        .map(compact_text)
        .filter(|value| !value.is_empty())?;
    let mut summary = Map::new();
    summary.insert("text".to_string(), Value::String(text));
    summary.insert("basis".to_string(), Value::String(basis.to_string()));
    insert_string_or_clone(&mut summary, "trustBand", item, "/trustBand");
    insert_string_or_clone(&mut summary, "freshness", item, "/freshness");
    insert_string_or_clone(&mut summary, "sourceAsOf", item, "/sourceAsOf");
    if let Some(source_refs) = item.get("sourceRefs").filter(|value| !value.is_null()) {
        summary.insert("sourceRefs".to_string(), source_refs.clone());
    }
    Some(Value::Object(summary))
}

fn collect_caveats(envelope: &Value) -> Vec<Value> {
    let mut caveats = Vec::new();
    if let Some(section_caveats) = envelope
        .pointer("/trust/sectionCaveats")
        .and_then(Value::as_object)
    {
        for (section, value) in section_caveats {
            if let Some(text) = value
                .as_str()
                .map(compact_text)
                .filter(|text| !text.is_empty())
            {
                push_unique_caveat(&mut caveats, section, text);
            }
        }
    }

    if let Some(sections) = envelope.get("sections").and_then(Value::as_object) {
        for (section, state) in sections {
            if let Some(advisory) = partial_failure_advisory(state) {
                push_unique_caveat(&mut caveats, section, advisory);
            }
        }
    }

    push_bundle_partial_failure_caveats(
        &mut caveats,
        envelope,
        "relationships",
        "/relationships/items",
    );
    push_bundle_partial_failure_caveats(
        &mut caveats,
        envelope,
        "touchpoints",
        "/touchpoints/items",
    );

    if envelope
        .pointer("/provenance/redactionApplied")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        push_unique_caveat(
            &mut caveats,
            "provenance",
            "Some source details were redacted for this surface.".to_string(),
        );
    }
    caveats
}

fn push_bundle_partial_failure_caveats(
    caveats: &mut Vec<Value>,
    envelope: &Value,
    section: &str,
    pointer: &str,
) {
    if let Some(items) = envelope.pointer(pointer).and_then(Value::as_array) {
        for item in items {
            if let Some(advisory) = item.get("emptyReason").and_then(partial_failure_advisory) {
                push_unique_caveat(caveats, section, advisory);
            }
        }
    }
}

fn push_unique_caveat(caveats: &mut Vec<Value>, section: &str, text: String) {
    let exists = caveats.iter().any(|caveat| {
        string_at(caveat, "/section") == Some(section) && string_at(caveat, "/text") == Some(&text)
    });
    if !exists {
        caveats.push(json!({
            "section": section,
            "text": text,
        }));
    }
}

fn partial_failure_advisory(state: &Value) -> Option<String> {
    state
        .pointer("/partial_failure/advisory")
        .or_else(|| state.pointer("/partialFailure/advisory"))
        .or_else(|| state.pointer("/reason/partial_failure/advisory"))
        .or_else(|| state.pointer("/empty/partial_failure/advisory"))
        .or_else(|| state.pointer("/reason/partialFailure/advisory"))
        .and_then(Value::as_str)
        .map(compact_text)
        .filter(|value| !value.is_empty())
}

fn build_truncation_summary(
    envelope: &Value,
    facts: &[Value],
    open_loops: &[Value],
    relationships: &[Value],
    touchpoints: &[Value],
    record_entries: &[Value],
) -> Value {
    json!({
        "facts": paginated_summary(envelope.pointer("/facts"), facts.len()),
        "openLoops": paginated_summary(envelope.pointer("/openLoops"), open_loops.len()),
        "relationships": relationships_truncation_summary(envelope, relationships.len()),
        "touchpoints": touchpoints_truncation_summary(envelope, touchpoints.len()),
        "recordEntries": paginated_summary(envelope.pointer("/recordEntries"), record_entries.len()),
    })
}

fn paginated_summary(value: Option<&Value>, rendered_count: usize) -> Value {
    let raw_count = value
        .and_then(|value| value.pointer("/items"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let omitted_count = raw_count.saturating_sub(rendered_count);
    json!({
        "renderedCount": rendered_count,
        "rawCount": raw_count,
        "omittedCount": omitted_count,
        "projectionTruncated": omitted_count > 0,
        "presenterTruncated": raw_count > rendered_count,
        "sourcePage": source_page_summary(value),
    })
}

fn source_page_summary(value: Option<&Value>) -> Value {
    json!({
        "totalHint": value
            .and_then(|value| value.get("totalHint"))
            .cloned()
            .unwrap_or(Value::Null),
        "nextCursorPresent": value
            .and_then(|value| value.get("nextCursor"))
            .is_some_and(|cursor| !cursor.is_null()),
        "cursorState": value
            .and_then(|value| value.get("cursorState"))
            .cloned()
            .unwrap_or(Value::Null),
    })
}

fn relationships_truncation_summary(envelope: &Value, rendered_count: usize) -> Value {
    let bundles = envelope
        .pointer("/relationships/items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edge_count = bundles
        .iter()
        .filter_map(|bundle| bundle.pointer("/edges/items").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let participant_count = bundles
        .iter()
        .filter_map(|bundle| {
            bundle
                .pointer("/participants/items")
                .and_then(Value::as_array)
        })
        .map(Vec::len)
        .sum::<usize>();
    let edges_truncated = bundles.iter().any(|bundle| {
        bundle
            .pointer("/truncation/edgesTruncated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let participants_truncated = bundles.iter().any(|bundle| {
        bundle
            .pointer("/truncation/participantsTruncated")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let raw_summary_count = edge_count + participant_count;
    let omitted_count = raw_summary_count.saturating_sub(rendered_count);

    json!({
        "renderedBundleCount": bundles.len(),
        "renderedCount": rendered_count,
        "edgeCount": edge_count,
        "participantCount": participant_count,
        "rawSummaryCount": raw_summary_count,
        "omittedCount": omitted_count,
        "projectionTruncated": omitted_count > 0,
        "presenterTruncated": raw_summary_count > rendered_count,
        "edgesTruncated": edges_truncated,
        "participantsTruncated": participants_truncated,
        "page": paginated_summary(envelope.pointer("/relationships"), bundles.len()),
    })
}

fn touchpoints_truncation_summary(envelope: &Value, rendered_count: usize) -> Value {
    let bundles = envelope
        .pointer("/touchpoints/items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let upcoming_count = bundles
        .iter()
        .filter_map(|bundle| bundle.pointer("/upcoming/items").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let recent_count = bundles
        .iter()
        .filter_map(|bundle| bundle.pointer("/recent/items").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let raw_summary_count = upcoming_count + recent_count;
    let omitted_count = raw_summary_count.saturating_sub(rendered_count);
    json!({
        "renderedBundleCount": bundles.len(),
        "renderedCount": rendered_count,
        "upcomingCount": upcoming_count,
        "recentCount": recent_count,
        "rawSummaryCount": raw_summary_count,
        "omittedCount": omitted_count,
        "projectionTruncated": omitted_count > 0,
        "presenterTruncated": raw_summary_count > rendered_count,
        "page": paginated_summary(envelope.pointer("/touchpoints"), bundles.len()),
    })
}

fn response_status(
    facts: &[Value],
    open_loops: &[Value],
    relationships: &[Value],
    touchpoints: &[Value],
    record_entries: &[Value],
    envelope: &Value,
) -> &'static str {
    let has_content = !(facts.is_empty()
        && open_loops.is_empty()
        && relationships.is_empty()
        && touchpoints.is_empty()
        && record_entries.is_empty());
    if contains_partial_failure(envelope) {
        return if has_content { "ok" } else { "unavailable" };
    }
    if has_content {
        "ok"
    } else {
        "not_found"
    }
}

fn contains_partial_failure(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object.contains_key("partial_failure")
                || object.contains_key("partialFailure")
                || object.values().any(contains_partial_failure)
        }
        Value::Array(items) => items.iter().any(contains_partial_failure),
        _ => false,
    }
}

fn insert_string_or_clone(
    target: &mut Map<String, Value>,
    key: &str,
    source: &Value,
    pointer: &str,
) {
    if let Some(value) = source.pointer(pointer).filter(|value| !value.is_null()) {
        target.insert(key.to_string(), value.clone());
    }
}

fn insert_source_refs(
    target: &mut Map<String, Value>,
    item: &Value,
    source_id_map: &BTreeMap<String, String>,
) {
    let refs = item
        .pointer("/provenance/sourceIds")
        .and_then(Value::as_array)
        .map(|source_ids| {
            source_ids
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|raw_id| source_id_map.get(raw_id))
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !refs.is_empty() {
        target.insert("sourceRefs".to_string(), Value::Array(refs));
    }
}

pub(crate) fn evidence_suffix(item: &Value) -> String {
    let mut parts = Vec::new();
    push_humanized_part(&mut parts, "trust", item, "/trustBand");
    push_humanized_part(&mut parts, "freshness", item, "/freshness");
    if let Some(source_as_of) = string_at(item, "/sourceAsOf") {
        parts.push(format!("as of {source_as_of}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

pub(crate) fn open_loop_suffix(item: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(status) = string_at(item, "/status") {
        parts.push(format!("status: {}", humanize_token(status)));
    }
    if let Some(due_date) = string_at(item, "/dueDate") {
        parts.push(format!("due {due_date}"));
    }
    push_humanized_part(&mut parts, "trust", item, "/trustBand");
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join("; "))
    }
}

fn push_humanized_part(parts: &mut Vec<String>, label: &str, item: &Value, pointer: &str) {
    if let Some(value) = string_at(item, pointer) {
        parts.push(format!("{label}: {}", humanize_token(value)));
    }
}

pub(crate) fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

pub(crate) fn compact_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn humanize_token(value: &str) -> String {
    value.replace('_', " ")
}
