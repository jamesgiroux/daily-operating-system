//! Canonical stakeholder graph write helpers.
//!
//! Any service-layer mutation that changes `account_stakeholders` or
//! `entity_members` membership must route through these helpers so the
//! stakeholder cache invalidation signal is co-committed with the source write.

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

pub(crate) fn write_with_stakeholders_changed<T>(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    mutation_source: &str,
    write: impl FnOnce(&ActionDb) -> Result<T, String>,
) -> Result<T, String> {
    let result = write(tx)?;
    emit_stakeholders_changed(ctx, tx, entity_type, entity_id, mutation_source)?;
    Ok(result)
}

pub(crate) fn emit_stakeholders_changed(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    mutation_source: &str,
) -> Result<String, String> {
    crate::services::signals::emit_in_transaction(
        ctx,
        tx,
        entity_type,
        entity_id,
        crate::services::signals::STAKEHOLDERS_CHANGED_SIGNAL,
        mutation_source,
        serde_json::json!({
            "entity_id": entity_id,
            "entity_type": entity_type,
            "mutation_source": mutation_source,
        }),
    )
}

pub(crate) fn emit_stakeholders_changed_for_entities(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    affected_entities: impl IntoIterator<Item = (String, String)>,
    mutation_source: &str,
) -> Result<(), String> {
    for (entity_id, entity_type) in affected_entities {
        emit_stakeholders_changed(ctx, tx, &entity_type, &entity_id, mutation_source)?;
    }
    Ok(())
}

pub(crate) fn write_with_stakeholders_changed_for_entities<T>(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    mutation_source: &str,
    write: impl FnOnce(&ActionDb) -> Result<(T, Vec<(String, String)>), String>,
) -> Result<T, String> {
    let (result, affected_entities) = write(tx)?;
    emit_stakeholders_changed_for_entities(ctx, tx, affected_entities, mutation_source)?;
    Ok(result)
}
