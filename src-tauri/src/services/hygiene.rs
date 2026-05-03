use crate::db::ActionDb;

pub fn update_person_relationship(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    person_id: &str,
    relationship: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        tx.update_person_relationship(person_id, relationship)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            ctx,
            tx,
            "person",
            person_id,
            "relationship_reclassified",
            "hygiene",
            Some(&format!("{{\"relationship\":\"{}\"}}", relationship)),
            0.75,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn mark_content_index_summary(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    file_id: &str,
    extracted_at: &str,
    summary: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.conn_ref()
        .execute(
            "UPDATE content_index SET extracted_at = ?1, summary = ?2 WHERE id = ?3",
            rusqlite::params![extracted_at, summary, file_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn recompute_person_meeting_count(db: &ActionDb, person_id: &str) -> Result<(), String> {
    db.recompute_person_meeting_count(person_id)
        .map_err(|e| e.to_string())
}

pub fn rollover_account_renewal(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    account_id: &str,
    account_name: &str,
    renewal_date: &str,
    arr: Option<f64>,
    next_contract_end: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        tx.record_account_event(
            account_id,
            "renewal",
            renewal_date,
            arr,
            Some("Auto-renewed (no churn recorded)"),
        )
        .map_err(|e| e.to_string())?;

        tx.conn_ref()
            .execute(
                "UPDATE accounts SET contract_end = ?1 WHERE id = ?2",
                rusqlite::params![next_contract_end, account_id],
            )
            .map_err(|e| e.to_string())?;

        crate::services::signals::emit(
            ctx,
            tx,
            "account",
            account_id,
            "renewal_rolled_over",
            "hygiene",
            Some(&format!(
                "{{\"account\":\"{}\",\"from\":\"{}\",\"to\":\"{}\"}}",
                account_name.replace('"', "\\\""),
                renewal_date,
                next_contract_end,
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;

        Ok(())
    })
}

pub fn reset_quill_sync_for_retry(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    sync_id: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.reset_quill_sync_for_retry(sync_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_person_name(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    person_id: &str,
    display_name: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        tx.update_person_name(person_id, display_name)
            .map_err(|e| e.to_string())?;
        crate::services::signals::emit(
            ctx,
            tx,
            "person",
            person_id,
            "person_name_updated",
            "hygiene",
            Some(&format!(
                "{{\"name\":\"{}\"}}",
                display_name.replace('"', "\\\"")
            )),
            0.8,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn merge_people(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    keep_id: &str,
    remove_id: &str,
    source: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        crate::services::people::merge_people_with_stakeholder_cache_rebuild(
            ctx, tx, keep_id, remove_id,
        )?;
        crate::services::signals::emit(
            ctx,
            tx,
            "person",
            keep_id,
            "people_merged",
            source,
            Some(&format!("{{\"removed_person_id\":\"{}\"}}", remove_id)),
            0.9,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn link_person_to_entity(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    person_id: &str,
    entity_id: &str,
    relationship_type: &str,
    signal_confidence: f64,
    signal_value: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        crate::services::people::link_person_to_entity_with_stakeholder_cache_rebuild(
            ctx,
            tx,
            person_id,
            entity_id,
            relationship_type,
        )?;
        crate::services::signals::emit(
            ctx,
            tx,
            "person",
            person_id,
            "account_linked",
            "hygiene",
            Some(signal_value),
            signal_confidence,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;
        Ok(())
    })
}

pub fn emit_low_confidence_match(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    value: &str,
    confidence: f64,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    crate::services::signals::emit(
        ctx,
        db,
        entity_type,
        entity_id,
        "low_confidence_match",
        "heuristic",
        Some(value),
        confidence,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;
    Ok(())
}
