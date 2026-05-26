//! Wave-scoped handler registration helper.
//!
//! Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)`
//! and `gateway.seal()`. Each wave (W2-A, W2-B, ...) adds its handlers here.

use std::sync::Arc;

use crate::services::mcp_v2::contracts::ScopedName;
use crate::services::mcp_v2::gateway::Gateway;
use crate::services::mcp_v2::taxonomy::TaxonomyCatalog;
use crate::signals::propagation::PropagationEngine;

use super::tool_account_status::AccountStatusHandler;
use super::tool_briefing::{DailyBriefingHandler, MeetingBriefingHandler};
use super::tool_create_action::CreateActionHandler;
use super::tool_note::NoteHandler;
use super::tool_placement::PlacementHandler;
use super::tool_portfolio::PortfolioAttentionHandler;
use super::tool_update_action_status::UpdateActionStatusHandler;
use super::tool_workspace_search::WorkspaceSearchHandler;
use super::tool_workspace_source_provenance::WorkspaceSourceProvenanceHandler;

/// Errors registering wave-scoped handlers at boot.
#[derive(Debug)]
pub enum RegistrationError {
    /// The catalog has no entry for a scoped name the wave expects.
    CatalogEntryMissing(ScopedName),
    /// The abilities-runtime registry refused to expose itself at startup
    /// (validation violations or `global_checked` panic guard).
    AbilityRegistry(&'static str),
}

impl std::fmt::Display for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CatalogEntryMissing(name) => {
                write!(f, "taxonomy catalog missing entry for {name}")
            }
            Self::AbilityRegistry(detail) => {
                write!(f, "ability registry unavailable: {detail}")
            }
        }
    }
}

impl std::error::Error for RegistrationError {}

/// Register all v1.4.7 W2 handlers on the gateway.
///
/// Current registered scope includes the account status read tool plus the
/// v1.4.5 workspace placement write tool once its placement substrate is
/// present on the rebased base.
pub fn register_v147_handlers(
    gateway: &mut Gateway,
    catalog: &Arc<dyn TaxonomyCatalog>,
    runtime: tokio::runtime::Handle,
    signal_engine: Arc<PropagationEngine>,
) -> Result<(), RegistrationError> {
    let account_status_name = ScopedName::new("dailyos.read.account_status");
    let description = catalog
        .description_for(&account_status_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(account_status_name.clone()))?
        .clone();

    let handler = AccountStatusHandler::from_runtime(description, runtime.clone())
        .map_err(RegistrationError::AbilityRegistry)?;
    gateway.register(Arc::new(handler));

    let daily_briefing_name = ScopedName::new("dailyos.read.daily_briefing");
    let description = catalog
        .description_for(&daily_briefing_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(daily_briefing_name.clone()))?
        .clone();

    let handler = DailyBriefingHandler::from_runtime(description, runtime.clone())
        .map_err(RegistrationError::AbilityRegistry)?;
    gateway.register(Arc::new(handler));

    let meeting_briefing_name = ScopedName::new("dailyos.read.meeting_briefing");
    let description = catalog
        .description_for(&meeting_briefing_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(meeting_briefing_name.clone()))?
        .clone();

    gateway.register(Arc::new(MeetingBriefingHandler::new(description)));

    let portfolio_attention_name = ScopedName::new("dailyos.read.portfolio_attention");
    let description = catalog
        .description_for(&portfolio_attention_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(portfolio_attention_name.clone()))?
        .clone();

    let handler = PortfolioAttentionHandler::from_runtime(description, runtime.clone())
        .map_err(RegistrationError::AbilityRegistry)?;
    gateway.register(Arc::new(handler));

    let workspace_search_name = ScopedName::new("dailyos.search.workspace_memory");
    let description = catalog
        .description_for(&workspace_search_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(workspace_search_name.clone()))?
        .clone();
    gateway.register(Arc::new(WorkspaceSearchHandler::new(description)));

    let workspace_provenance_name = ScopedName::new("dailyos.read.workspace_source_provenance");
    let description = catalog
        .description_for(&workspace_provenance_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(workspace_provenance_name.clone()))?
        .clone();
    gateway.register(Arc::new(WorkspaceSourceProvenanceHandler::new(description)));

    let placement_name = ScopedName::new("dailyos.write.place_document");
    let description = catalog
        .description_for(&placement_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(placement_name.clone()))?
        .clone();

    let handler = PlacementHandler::from_runtime(description, runtime, Arc::clone(&signal_engine))
        .map_err(RegistrationError::AbilityRegistry)?;
    gateway.register(Arc::new(handler));

    let create_action_name = ScopedName::new("dailyos.submit.action");
    let description = catalog
        .description_for(&create_action_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(create_action_name.clone()))?
        .clone();
    gateway.register(Arc::new(CreateActionHandler::new(
        description,
        Arc::clone(&signal_engine),
    )));

    let update_action_status_name = ScopedName::new("dailyos.submit.action_status");
    let description = catalog
        .description_for(&update_action_status_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(update_action_status_name.clone()))?
        .clone();
    gateway.register(Arc::new(UpdateActionStatusHandler::new(
        description,
        Arc::clone(&signal_engine),
    )));

    let note_name = ScopedName::new("dailyos.submit.note");
    let description = catalog
        .description_for(&note_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(note_name.clone()))?
        .clone();
    gateway.register(Arc::new(NoteHandler::new(
        description,
        Arc::clone(&signal_engine),
    )));

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::services::mcp_v2::contracts::ScopedName;
    use crate::services::mcp_v2::gateway::Gateway;
    use crate::services::mcp_v2::taxonomy::{TaxonomyCatalog, YamlTaxonomyCatalog};
    use crate::signals::propagation::default_engine;

    use super::*;

    #[test]
    fn registers_workspace_placement_handler_from_catalog() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let catalog: Arc<dyn TaxonomyCatalog> =
            Arc::new(YamlTaxonomyCatalog::load_embedded().expect("catalog"));
        let mut gateway = Gateway::new();
        gateway.set_taxonomy(Arc::clone(&catalog));

        register_v147_handlers(
            &mut gateway,
            &catalog,
            runtime.handle().clone(),
            Arc::new(default_engine()),
        )
        .expect("register handlers");

        let registered = gateway.registered_tools().cloned().collect::<Vec<_>>();
        assert!(registered.contains(&ScopedName::new("dailyos.read.account_status")));
        assert!(registered.contains(&ScopedName::new("dailyos.read.daily_briefing")));
        assert!(registered.contains(&ScopedName::new("dailyos.read.meeting_briefing")));
        assert!(registered.contains(&ScopedName::new("dailyos.read.portfolio_attention")));
        assert!(registered.contains(&ScopedName::new("dailyos.search.workspace_memory")));
        assert!(registered.contains(&ScopedName::new("dailyos.read.workspace_source_provenance")));
        assert!(registered.contains(&ScopedName::new("dailyos.write.place_document")));
        assert!(registered.contains(&ScopedName::new("dailyos.submit.note")));
        assert!(registered.contains(&ScopedName::new("dailyos.submit.action")));
        assert!(registered.contains(&ScopedName::new("dailyos.submit.action_status")));
        let pending = gateway.seal().expect("registered handlers match catalog");
        assert!(
            !pending.contains(&ScopedName::new("dailyos.read.daily_briefing")),
            "daily briefing handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.read.meeting_briefing")),
            "meeting briefing handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.read.portfolio_attention")),
            "portfolio attention handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.search.workspace_memory")),
            "workspace search handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.read.workspace_source_provenance")),
            "workspace source provenance handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.write.place_document")),
            "placement handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.submit.note")),
            "note handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.submit.action")),
            "create action handler should no longer be a catalog-only placeholder"
        );
        assert!(
            !pending.contains(&ScopedName::new("dailyos.submit.action_status")),
            "update action status handler should no longer be a catalog-only placeholder"
        );
    }
}
