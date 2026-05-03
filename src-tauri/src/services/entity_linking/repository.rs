//! Raw migration/backfill API. NOT for runtime callers.
//! Production code importing this module fails the pre-commit grep hook.

/// Purge inferred account domains before the account-domain rebuild cutover.
///
/// The old entity resolver accumulated domain → account mappings from meeting
/// attendee emails. Many of these are wrong (shared-domain consultants, joint
/// meetings with multiple accounts). This function removes all rows where
/// `source = 'inferred'` so the new deterministic linking engine starts from
/// a clean, trusted-only domain table.
///
/// Preserved rows:
///   source = 'user'        — user explicitly entered on the account page
///   source = 'enrichment'  — Clay/Glean enrichment providers
///
/// Run this function once, then perform the dry-run diff
/// before flipping the entity_linking_v2 feature flag.
pub fn raw_rebuild_account_domains(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let deleted = db
        .conn_ref()
        .execute(
            "DELETE FROM account_domains WHERE source = 'inferred'",
            [],
        )
        .map_err(|e| format!("raw_rebuild_account_domains: delete inferred domains: {e}"))?;

    log::info!(
        "raw_rebuild_account_domains: removed {} inferred domain(s). \
         User-entered (source='user') and enrichment-sourced (source='enrichment') \
         domains were kept.",
        deleted
    );

    Ok(())
}
