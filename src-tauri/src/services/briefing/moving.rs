//! Moving composer — produces the `MovingViewModel` slice.
//!
//! DOS-414 wires the existing meeting/action sources into the W2 briefing
//! contract. Email and lifecycle helpers keep their source-shaped signatures
//! but return empty lists until DOS-416/DOS-419 land. All trust bands are
//! `Unscored` for this ticket, matching the schedule MVP pattern.

use std::collections::HashMap;

use chrono::{DateTime, Local, NaiveDate, TimeZone};

use crate::services::actions::{get_all_actions, ActionsResult};
use crate::services::briefing_view_model::{
    LifecycleMixin, LinkRole, LinkedEntityType, LinkedEntityWire, MovingEntityKind,
    MovingEntityViewModel, MovingSignalViewModel, MovingViewModel, PillTone, PillView,
    ProvenanceStatView, ProvenanceTrend, SignalDotKind, SignalUrgency, ThreadAction, TrustBandWire,
    TrustMixin, WhatSegment,
};
use crate::services::dashboard::{get_dashboard_data, DashboardResult};
use crate::state::AppState;
use crate::types::{Action, DashboardLifecycleUpdate, LinkedEntity, Meeting};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EntityId(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClaimId(i64);

type SignalWithClaim = (EntityId, MovingSignalViewModel, Option<ClaimId>);
type EntitySignal = (MovingSignalViewModel, Option<ClaimId>);

#[derive(Debug, Clone)]
struct EntityRecord {
    id: EntityId,
    name: String,
    entity_type: LinkedEntityType,
    is_internal: bool,
    confidence: Option<f64>,
    is_primary: Option<bool>,
    suggested: Option<bool>,
    role: Option<LinkRole>,
    applied_rule: Option<Option<String>>,
    health: Option<String>,
    stage: Option<String>,
    status: Option<String>,
    owner: Option<String>,
    updated_at: Option<String>,
}

pub async fn compose_moving(state: &AppState) -> MovingViewModel {
    let dashboard = get_dashboard_data(state).await;
    let (meetings, lifecycle_updates) = match dashboard {
        DashboardResult::Success { data, .. } => (data.meetings, data.lifecycle_updates),
        _ => (Vec::new(), None),
    };

    let mut entity_index = collect_entity_index(&meetings);
    let grouped = group_signals_by_entity(
        collect_meeting_signals(&meetings)
            .into_iter()
            .chain(collect_action_signals(state).await)
            .chain(collect_email_signals(state).await)
            .chain(collect_lifecycle_signals(lifecycle_updates.as_deref())),
    );

    let ids: Vec<EntityId> = grouped.keys().cloned().collect();
    enrich_entity_index(state, &ids, &mut entity_index).await;
    let entities = build_ranked_entities(grouped, &entity_index);

    MovingViewModel {
        label: "Moving".into(),
        heading: "What's moving".into(),
        count_label: format_count_label(entities.len()),
        summary: format_summary(&entities),
        entities,
    }
}

fn collect_meeting_signals(meetings: &[Meeting]) -> Vec<SignalWithClaim> {
    meetings
        .iter()
        .filter_map(|meeting| {
            let entity = primary_linked_entity(meeting)?;
            Some((
                EntityId(entity.id.clone()),
                MovingSignalViewModel {
                    trust: unscored(Some(format!("meeting.{}.intelligence", meeting.id))),
                    lifecycle: no_lifecycle(),
                    kind: SignalDotKind::Meeting,
                    when: format_meeting_when(meeting),
                    what_segments: meeting_segments(meeting),
                    urgency: SignalUrgency::Normal,
                    thread_action: meeting.has_prep.then(|| ThreadAction {
                        label: "Open briefing".into(),
                        href: format!("/meetings/{}", meeting.id),
                    }),
                },
                None,
            ))
        })
        .collect()
}

async fn collect_action_signals(state: &AppState) -> Vec<SignalWithClaim> {
    match get_all_actions(state).await {
        ActionsResult::Success { data } => map_actions_to_signals(&data),
        _ => Vec::new(),
    }
}

fn map_actions_to_signals(actions: &[Action]) -> Vec<SignalWithClaim> {
    actions
        .iter()
        .filter_map(|action| {
            let account_id = action.account.as_ref()?;
            Some((
                EntityId(account_id.clone()),
                MovingSignalViewModel {
                    trust: unscored(Some(format!("action.{}", action.id))),
                    lifecycle: no_lifecycle(),
                    kind: SignalDotKind::Action,
                    when: action.due_date.clone().unwrap_or_else(|| "Open".into()),
                    what_segments: action_segments(action),
                    urgency: if action.is_overdue.unwrap_or(false) {
                        SignalUrgency::Overdue
                    } else {
                        SignalUrgency::Normal
                    },
                    thread_action: Some(ThreadAction {
                        label: "Open action".into(),
                        href: format!("/actions/{}", action.id),
                    }),
                },
                None,
            ))
        })
        .collect()
}

async fn collect_email_signals(_state: &AppState) -> Vec<SignalWithClaim> {
    Vec::new()
}

fn collect_lifecycle_signals(
    _updates: Option<&[DashboardLifecycleUpdate]>,
) -> Vec<SignalWithClaim> {
    Vec::new()
}

fn group_signals_by_entity<I>(signals: I) -> HashMap<EntityId, Vec<EntitySignal>>
where
    I: IntoIterator<Item = SignalWithClaim>,
{
    let mut grouped: HashMap<EntityId, Vec<EntitySignal>> = HashMap::new();
    for (entity_id, signal, claim_id) in signals {
        grouped
            .entry(entity_id)
            .or_default()
            .push((signal, claim_id));
    }
    grouped
}

fn build_ranked_entities(
    grouped: HashMap<EntityId, Vec<EntitySignal>>,
    entity_index: &HashMap<EntityId, EntityRecord>,
) -> Vec<MovingEntityViewModel> {
    let mut entities: Vec<_> = grouped
        .into_iter()
        .filter_map(|(id, entries)| {
            (!entries.is_empty()).then(|| build_entity_view(&id, entries, entity_index.get(&id)))
        })
        .collect();
    entities.sort_by(|a, b| {
        change_magnitude(b)
            .total_cmp(&change_magnitude(a))
            .then_with(|| latest_signal_sort_key(b).cmp(&latest_signal_sort_key(a)))
            .then_with(|| a.entity.name.cmp(&b.entity.name))
    });
    entities.truncate(3);
    entities
}

fn build_entity_view(
    entity_id: &EntityId,
    mut entries: Vec<EntitySignal>,
    record: Option<&EntityRecord>,
) -> MovingEntityViewModel {
    let _last_claim_id = entries.iter().filter_map(|(_, id)| id.map(|id| id.0)).max();
    entries.sort_by(|(a, _), (b, _)| {
        signal_weight(b)
            .total_cmp(&signal_weight(a))
            .then_with(|| signal_sort_key(b).cmp(&signal_sort_key(a)))
    });
    let signals: Vec<_> = entries
        .into_iter()
        .map(|(signal, _)| signal)
        .take(5)
        .collect();
    let fallback = fallback_record(entity_id);
    let record = record.unwrap_or(&fallback);

    MovingEntityViewModel {
        kind: classify_kind(record, &signals),
        entity: to_wire_entity(record),
        href: entity_href(record),
        state_pill: state_pill(record, &signals),
        lede: format_lede(record, &signals),
        signals,
        provenance_stats: provenance_stats(record),
    }
}

fn change_magnitude(entity: &MovingEntityViewModel) -> f64 {
    entity.signals.iter().map(signal_weight).sum()
}

fn signal_weight(signal: &MovingSignalViewModel) -> f64 {
    match signal.kind {
        SignalDotKind::Lifecycle => 5.0,
        SignalDotKind::Meeting if signal.thread_action.is_some() => 3.0,
        SignalDotKind::Meeting if signal.when.starts_with("Today") => 2.5,
        SignalDotKind::Meeting => 0.5,
        SignalDotKind::Action if signal.urgency == SignalUrgency::Overdue => 2.0,
        SignalDotKind::Action => 0.5,
        SignalDotKind::Email => 1.5,
        SignalDotKind::GongCall
        | SignalDotKind::ZendeskTicket
        | SignalDotKind::SlackThread
        | SignalDotKind::LinearIssue => 1.0,
    }
}

fn latest_signal_sort_key(entity: &MovingEntityViewModel) -> Option<i64> {
    entity.signals.iter().filter_map(signal_sort_key).max()
}

fn signal_sort_key(signal: &MovingSignalViewModel) -> Option<i64> {
    parse_when_sort_key(&signal.when)
}

fn classify_kind(record: &EntityRecord, signals: &[MovingSignalViewModel]) -> MovingEntityKind {
    if lifecycle_dominated(signals) {
        return MovingEntityKind::Lifecycle;
    }
    match &record.entity_type {
        LinkedEntityType::Account if record.is_internal => MovingEntityKind::Internal,
        LinkedEntityType::Account => MovingEntityKind::Customer,
        LinkedEntityType::Project => MovingEntityKind::Project,
        LinkedEntityType::Person => MovingEntityKind::Person,
    }
}

fn lifecycle_dominated(signals: &[MovingSignalViewModel]) -> bool {
    !signals.is_empty()
        && signals
            .iter()
            .filter(|signal| signal.kind == SignalDotKind::Lifecycle)
            .count()
            * 2
            >= signals.len()
}

fn collect_entity_index(meetings: &[Meeting]) -> HashMap<EntityId, EntityRecord> {
    let mut index = HashMap::new();
    for meeting in meetings {
        if let Some(record) = primary_linked_entity(meeting).and_then(entity_record_from_linked) {
            index.entry(record.id.clone()).or_insert(record);
        }
    }
    index
}

async fn enrich_entity_index(
    state: &AppState,
    ids: &[EntityId],
    entity_index: &mut HashMap<EntityId, EntityRecord>,
) {
    let ids: Vec<String> = ids.iter().map(|id| id.0.clone()).collect();
    let records = state
        .db_read(move |db| {
            let records = ids
                .into_iter()
                .filter_map(|id| db.get_account(&id).ok().flatten())
                .map(|account| {
                    let id = EntityId(account.id);
                    EntityRecord {
                        id,
                        name: account.name,
                        entity_type: LinkedEntityType::Account,
                        is_internal: account.account_type.is_internal(),
                        confidence: None,
                        is_primary: None,
                        suggested: None,
                        role: None,
                        applied_rule: None,
                        health: account.health,
                        stage: account.commercial_stage.or(account.lifecycle),
                        status: None,
                        owner: None,
                        updated_at: Some(account.updated_at),
                    }
                })
                .collect::<Vec<_>>();
            Ok(records)
        })
        .await
        .unwrap_or_default();

    for mut record in records {
        if let Some(existing) = entity_index.get(&record.id) {
            record.confidence = existing.confidence;
            record.is_primary = existing.is_primary;
            record.suggested = existing.suggested;
            record.role = existing.role.clone();
            record.applied_rule = existing.applied_rule.clone();
        }
        entity_index.insert(record.id.clone(), record);
    }
}

fn entity_record_from_linked(linked: &LinkedEntity) -> Option<EntityRecord> {
    Some(EntityRecord {
        id: EntityId(linked.id.clone()),
        name: linked.name.clone(),
        entity_type: linked_entity_type(&linked.entity_type)?,
        is_internal: false,
        confidence: Some(linked.confidence),
        is_primary: Some(linked.is_primary),
        suggested: Some(linked.suggested),
        role: linked.role.as_deref().and_then(link_role),
        applied_rule: linked.applied_rule.clone().map(Some),
        health: None,
        stage: None,
        status: None,
        owner: None,
        updated_at: None,
    })
}

fn fallback_record(entity_id: &EntityId) -> EntityRecord {
    EntityRecord {
        id: entity_id.clone(),
        name: entity_id.0.clone(),
        entity_type: LinkedEntityType::Account,
        is_internal: false,
        confidence: None,
        is_primary: None,
        suggested: None,
        role: None,
        applied_rule: None,
        health: None,
        stage: None,
        status: None,
        owner: None,
        updated_at: None,
    }
}

fn primary_linked_entity(meeting: &Meeting) -> Option<&LinkedEntity> {
    let linked = meeting.linked_entities.as_ref()?;
    linked
        .iter()
        .find(|entity| entity.is_primary)
        .or_else(|| linked.first())
}

fn linked_entity_type(value: &str) -> Option<LinkedEntityType> {
    match value {
        "account" => Some(LinkedEntityType::Account),
        "project" => Some(LinkedEntityType::Project),
        "person" => Some(LinkedEntityType::Person),
        _ => None,
    }
}

fn link_role(value: &str) -> Option<LinkRole> {
    match value {
        "primary" => Some(LinkRole::Primary),
        "related" => Some(LinkRole::Related),
        "auto_suggested" => Some(LinkRole::AutoSuggested),
        "user_dismissed" => Some(LinkRole::UserDismissed),
        _ => None,
    }
}

fn to_wire_entity(record: &EntityRecord) -> LinkedEntityWire {
    LinkedEntityWire {
        id: record.id.0.clone(),
        name: record.name.clone(),
        entity_type: record.entity_type.clone(),
        confidence: record.confidence,
        is_primary: record.is_primary,
        suggested: record.suggested,
        role: record.role.clone(),
        applied_rule: record.applied_rule.clone(),
    }
}

fn entity_href(record: &EntityRecord) -> String {
    match &record.entity_type {
        LinkedEntityType::Account => format!("/accounts/{}", record.id.0),
        LinkedEntityType::Project => format!("/projects/{}", record.id.0),
        LinkedEntityType::Person => format!("/people/{}", record.id.0),
    }
}

fn state_pill(record: &EntityRecord, signals: &[MovingSignalViewModel]) -> PillView {
    if lifecycle_dominated(signals) {
        pill("Lifecycle", PillTone::Olive)
    } else if signals.iter().any(|s| s.urgency == SignalUrgency::Overdue) {
        pill("Overdue", PillTone::Terracotta)
    } else if let Some(stage) = record.stage.as_ref().or(record.status.as_ref()) {
        pill(&titleize(stage), PillTone::Sage)
    } else if signals.iter().any(|s| s.kind == SignalDotKind::Meeting) {
        pill("Meeting", PillTone::Larkspur)
    } else {
        pill("Active", PillTone::Neutral)
    }
}

fn pill(label: &str, tone: PillTone) -> PillView {
    PillView {
        label: label.into(),
        tone,
    }
}

fn provenance_stats(record: &EntityRecord) -> Vec<ProvenanceStatView> {
    let mut stats = Vec::new();
    if let Some(health) = &record.health {
        stats.push(stat(&record.id, "Health", health, None));
    }
    if let Some(stage) = &record.stage {
        stats.push(stat(&record.id, "Stage", &titleize(stage), None));
    }
    if let Some(owner) = &record.owner {
        stats.push(stat(&record.id, "Owner", owner, None));
    }
    if let Some(updated) = &record.updated_at {
        stats.push(stat(
            &record.id,
            "Last touch",
            updated,
            Some(ProvenanceTrend::Flat),
        ));
    }
    if stats.is_empty() {
        stats.push(stat(
            &record.id,
            "Type",
            entity_type_label(&record.entity_type),
            None,
        ));
    }
    stats.truncate(4);
    stats
}

fn stat(
    entity_id: &EntityId,
    label: &str,
    value: &str,
    trend: Option<ProvenanceTrend>,
) -> ProvenanceStatView {
    ProvenanceStatView {
        trust: unscored(Some(format!(
            "entity.{}.{}",
            entity_id.0,
            label.to_lowercase().replace(' ', "_")
        ))),
        label: label.into(),
        value: value.into(),
        trend,
    }
}

fn entity_type_label(entity_type: &LinkedEntityType) -> &'static str {
    match entity_type {
        LinkedEntityType::Account => "Account",
        LinkedEntityType::Project => "Project",
        LinkedEntityType::Person => "Person",
    }
}

fn format_lede(record: &EntityRecord, signals: &[MovingSignalViewModel]) -> String {
    let parts: Vec<String> = signals.iter().take(2).map(signal_text).collect();
    match parts.as_slice() {
        [] => format!("{} has fresh movement.", record.name),
        [one] => format!("{}: {}.", record.name, trim_sentence(one)),
        [one, two, ..] => format!(
            "{}: {}; {}.",
            record.name,
            trim_sentence(one),
            trim_sentence(two)
        ),
    }
}

fn signal_text(signal: &MovingSignalViewModel) -> String {
    signal
        .what_segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join("")
}

fn trim_sentence(value: &str) -> String {
    value.trim().trim_end_matches(['.', ';']).to_string()
}

fn meeting_segments(meeting: &Meeting) -> Vec<WhatSegment> {
    vec![
        segment(format!("{} ", meeting.title), true),
        segment(
            if meeting.has_prep || meeting.prep.is_some() {
                "has prep ready"
            } else {
                "has no briefing yet"
            },
            false,
        ),
    ]
}

fn action_segments(action: &Action) -> Vec<WhatSegment> {
    vec![
        segment(
            if action.is_overdue.unwrap_or(false) {
                "Overdue action: "
            } else {
                "Open action: "
            },
            false,
        ),
        segment(action.title.clone(), true),
    ]
}

fn segment(text: impl Into<String>, emphasized: bool) -> WhatSegment {
    WhatSegment {
        text: text.into(),
        emphasized: emphasized.then_some(true),
    }
}

fn format_meeting_when(meeting: &Meeting) -> String {
    if meeting.start_iso.as_deref().is_some_and(is_today) {
        return format!("Today {}", meeting.time);
    }
    meeting
        .start_iso
        .as_deref()
        .and_then(|start| DateTime::parse_from_rfc3339(start).ok())
        .map(|dt| format!("{} {}", dt.date_naive(), meeting.time))
        .unwrap_or_else(|| meeting.time.clone())
}

fn is_today(value: &str) -> bool {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Local).date_naive() == Local::now().date_naive())
        .unwrap_or(false)
}

fn parse_when_sort_key(value: &str) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp());
    }
    if let Some(rest) = value.strip_prefix("Today ") {
        let time = chrono::NaiveTime::parse_from_str(rest.trim(), "%H:%M").ok()?;
        return Local
            .from_local_datetime(&Local::now().date_naive().and_time(time))
            .single()
            .map(|dt| dt.timestamp());
    }
    NaiveDate::parse_from_str(value.get(..10)?, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .and_then(|dt| Local.from_local_datetime(&dt).single())
        .map(|dt| dt.timestamp())
}

fn format_count_label(count: usize) -> String {
    if count == 1 {
        "1 entity".into()
    } else {
        format!("{count} entities")
    }
}

fn format_summary(entities: &[MovingEntityViewModel]) -> String {
    if entities.is_empty() {
        "Quiet.".into()
    } else if entities.len() == 1 {
        format!("{} is moving.", entities[0].entity.name)
    } else {
        format!("{} entities are moving.", entities.len())
    }
}

fn unscored(path: Option<String>) -> TrustMixin {
    TrustMixin {
        trust_band: TrustBandWire::Unscored,
        trust_field_path: path,
        trust_source_date: None,
        rendered_provenance: None,
    }
}

fn no_lifecycle() -> LifecycleMixin {
    LifecycleMixin {
        correction_state: None,
    }
}

fn titleize(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ActionStatus, MeetingType};
    use serde_json::Value;

    fn link(id: &str, name: &str, ty: &str) -> LinkedEntity {
        LinkedEntity {
            id: id.into(),
            name: name.into(),
            entity_type: ty.into(),
            confidence: 0.95,
            is_primary: true,
            suggested: false,
            role: Some("primary".into()),
            applied_rule: Some("P5".into()),
        }
    }

    fn meeting(id: &str, account: &str, prep: bool) -> Meeting {
        Meeting {
            id: id.into(),
            calendar_event_id: None,
            time: "10:00".into(),
            end_time: None,
            start_iso: Some(Local::now().to_rfc3339()),
            title: format!("Meeting {id}"),
            meeting_type: MeetingType::Customer,
            prep: prep.then(Default::default),
            is_current: None,
            prep_file: None,
            has_prep: prep,
            overlay_status: None,
            prep_reviewed: None,
            linked_entities: Some(vec![link(account, "Globex", "account")]),
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
            calendar_attendees: None,
            calendar_description: None,
        }
    }

    fn action(id: &str, account: &str, overdue: bool) -> Action {
        Action {
            id: id.into(),
            title: format!("Action {id}"),
            account: Some(account.into()),
            due_date: Some("2026-05-07".into()),
            priority: crate::types::Priority::Medium,
            status: ActionStatus::Unstarted,
            is_overdue: Some(overdue),
            context: None,
            source: None,
            days_overdue: overdue.then_some(2),
        }
    }

    fn record(id: &str, name: &str, ty: LinkedEntityType) -> EntityRecord {
        EntityRecord {
            id: EntityId(id.into()),
            name: name.into(),
            entity_type: ty,
            is_internal: false,
            confidence: None,
            is_primary: None,
            suggested: None,
            role: None,
            applied_rule: None,
            health: None,
            stage: None,
            status: None,
            owner: None,
            updated_at: None,
        }
    }

    fn sig(
        kind: SignalDotKind,
        urgency: SignalUrgency,
        when: &str,
        action: bool,
    ) -> MovingSignalViewModel {
        MovingSignalViewModel {
            trust: unscored(Some("test.signal".into())),
            lifecycle: no_lifecycle(),
            kind,
            when: when.into(),
            what_segments: vec![segment("movement", false)],
            urgency,
            thread_action: action.then(|| ThreadAction {
                label: "Open".into(),
                href: "/open".into(),
            }),
        }
    }

    fn build(
        signals: Vec<SignalWithClaim>,
        records: Vec<EntityRecord>,
    ) -> Vec<MovingEntityViewModel> {
        let index = records
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect();
        build_ranked_entities(group_signals_by_entity(signals), &index)
    }

    #[test]
    fn moving_empty_branch_renders_editorial_copy() {
        let entities = build_ranked_entities(HashMap::new(), &HashMap::new());
        assert!(entities.is_empty());
        assert_eq!(format_count_label(0), "0 entities");
        assert_eq!(format_summary(&entities), "Quiet.");
    }

    #[test]
    fn single_meeting_source_builds_one_entity() {
        let meetings = vec![meeting("m1", "acc-1", true)];
        let signals = collect_meeting_signals(&meetings);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].2, None);
        assert_eq!(signals[0].1.kind, SignalDotKind::Meeting);
        assert_eq!(signals[0].1.trust.trust_band, TrustBandWire::Unscored);
        let entities = build_ranked_entities(
            group_signals_by_entity(signals),
            &collect_entity_index(&meetings),
        );
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].kind, MovingEntityKind::Customer);
        assert_eq!(entities[0].entity.id, "acc-1");
    }

    #[test]
    fn multiple_sources_group_and_preserve_claim_ids() {
        let grouped = group_signals_by_entity(
            collect_meeting_signals(&[meeting("m1", "acc-1", true)])
                .into_iter()
                .chain(map_actions_to_signals(&[action("a1", "acc-1", true)]))
                .chain([(
                    EntityId("acc-1".into()),
                    sig(
                        SignalDotKind::Lifecycle,
                        SignalUrgency::Normal,
                        "2026-05-07T12:00:00Z",
                        false,
                    ),
                    Some(ClaimId(42)),
                )]),
        );
        let entries = grouped.get(&EntityId("acc-1".into())).expect("entity");
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|(_, claim)| *claim == Some(ClaimId(42))));
        assert_eq!(
            entries.iter().filter(|(_, claim)| claim.is_none()).count(),
            2
        );
        assert!(entries.iter().any(|(signal, claim)| {
            signal.kind == SignalDotKind::Action
                && signal.urgency == SignalUrgency::Overdue
                && claim.is_none()
        }));
    }

    #[test]
    fn ranking_respects_weights_and_three_cap() {
        let entities = build(
            vec![
                (
                    EntityId("normal".into()),
                    sig(
                        SignalDotKind::Action,
                        SignalUrgency::Normal,
                        "2026-05-07T09:00:00Z",
                        false,
                    ),
                    None,
                ),
                (
                    EntityId("meeting".into()),
                    sig(
                        SignalDotKind::Meeting,
                        SignalUrgency::Normal,
                        "Today 10:00",
                        true,
                    ),
                    None,
                ),
                (
                    EntityId("overdue".into()),
                    sig(
                        SignalDotKind::Action,
                        SignalUrgency::Overdue,
                        "2026-05-07T11:00:00Z",
                        false,
                    ),
                    None,
                ),
                (
                    EntityId("lifecycle".into()),
                    sig(
                        SignalDotKind::Lifecycle,
                        SignalUrgency::Normal,
                        "2026-05-07T08:00:00Z",
                        false,
                    ),
                    None,
                ),
                (
                    EntityId("email".into()),
                    sig(
                        SignalDotKind::Email,
                        SignalUrgency::Normal,
                        "2026-05-07T12:00:00Z",
                        false,
                    ),
                    None,
                ),
            ],
            ["normal", "meeting", "overdue", "lifecycle", "email"]
                .into_iter()
                .map(|id| record(id, id, LinkedEntityType::Account))
                .collect(),
        );
        assert_eq!(entities.len(), 3);
        assert_eq!(
            entities
                .iter()
                .map(|e| e.entity.id.as_str())
                .collect::<Vec<_>>(),
            ["lifecycle", "meeting", "overdue"]
        );
    }

    #[test]
    fn lifecycle_dominated_kind_override_and_internal_kind_work() {
        let mut internal = record("acc-1", "DailyOS", LinkedEntityType::Account);
        internal.is_internal = true;
        let entities = build(
            vec![
                (
                    EntityId("acc-1".into()),
                    sig(
                        SignalDotKind::Lifecycle,
                        SignalUrgency::Normal,
                        "2026-05-07T12:00:00Z",
                        false,
                    ),
                    None,
                ),
                (
                    EntityId("acc-1".into()),
                    sig(
                        SignalDotKind::Action,
                        SignalUrgency::Normal,
                        "2026-05-07T11:00:00Z",
                        false,
                    ),
                    None,
                ),
            ],
            vec![internal],
        );
        assert_eq!(entities[0].kind, MovingEntityKind::Lifecycle);

        let mut internal = record("acc-2", "DailyOS", LinkedEntityType::Account);
        internal.is_internal = true;
        let entities = build(
            vec![(
                EntityId("acc-2".into()),
                sig(
                    SignalDotKind::Action,
                    SignalUrgency::Normal,
                    "2026-05-07",
                    false,
                ),
                None,
            )],
            vec![internal],
        );
        assert_eq!(entities[0].kind, MovingEntityKind::Internal);
    }

    #[test]
    fn moving_serializes_to_camel_case_wire_shape() {
        let entities = build(
            vec![(
                EntityId("acc-1".into()),
                sig(
                    SignalDotKind::Action,
                    SignalUrgency::Overdue,
                    "2026-05-07T11:00:00Z",
                    false,
                ),
                None,
            )],
            vec![record("acc-1", "Globex", LinkedEntityType::Account)],
        );
        let vm = MovingViewModel {
            label: "Moving".into(),
            heading: "What's moving".into(),
            count_label: format_count_label(entities.len()),
            summary: format_summary(&entities),
            entities,
        };
        let parsed: Value =
            serde_json::from_str(&serde_json::to_string(&vm).expect("serialize")).expect("parse");
        assert_eq!(parsed["countLabel"], "1 entity");
        assert_eq!(parsed["entities"][0]["statePill"]["label"], "Overdue");
        assert_eq!(
            parsed["entities"][0]["provenanceStats"][0]["trustBand"],
            "unscored"
        );
        assert_eq!(
            parsed["entities"][0]["signals"][0]["whatSegments"][0]["text"],
            "movement"
        );
        assert_eq!(
            parsed["entities"][0]["signals"][0]["trustFieldPath"],
            "test.signal"
        );
        assert!(parsed["entities"][0]["signals"][0].get("claimId").is_none());
    }
}
