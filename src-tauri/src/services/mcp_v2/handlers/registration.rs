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
use super::tool_claim_feedback::ClaimFeedbackHandler;
use super::tool_create_action::CreateActionHandler;
use super::tool_note::NoteHandler;
use super::tool_update_action_status::UpdateActionStatusHandler;
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

/// Register the W5 MCP parity handler set on the gateway.
///
/// Hidden W5 tools stay absent from this registry. In particular,
/// `dailyos.write.place_document` remains implemented elsewhere but is not
/// advertised or invocable through the W5 MCP parity surface.
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

    for name in [
        "dailyos.read.workspace_source_provenance",
        "dailyos.submit.claim_feedback",
        "dailyos.submit.note",
        "dailyos.submit.action",
        "dailyos.submit.action_status",
    ] {
        let scoped = ScopedName::new(name);
        let description = catalog
            .description_for(&scoped)
            .ok_or_else(|| RegistrationError::CatalogEntryMissing(scoped.clone()))?
            .clone();
        match name {
            "dailyos.read.workspace_source_provenance" => {
                gateway.register(Arc::new(WorkspaceSourceProvenanceHandler::new(description)));
            }
            "dailyos.submit.claim_feedback" => {
                gateway.register(Arc::new(ClaimFeedbackHandler::new(description)));
            }
            "dailyos.submit.note" => {
                gateway.register(Arc::new(NoteHandler::new(
                    description,
                    Arc::clone(&signal_engine),
                )));
            }
            "dailyos.submit.action" => {
                gateway.register(Arc::new(CreateActionHandler::new(
                    description,
                    Arc::clone(&signal_engine),
                )));
            }
            "dailyos.submit.action_status" => {
                gateway.register(Arc::new(UpdateActionStatusHandler::new(
                    description,
                    Arc::clone(&signal_engine),
                )));
            }
            _ => unreachable!("registered W5 tool name is exhaustive"),
        }
    }

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
    fn registers_w5_handlers_from_catalog() {
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
        for name in [
            "dailyos.read.account_status",
            "dailyos.read.workspace_source_provenance",
            "dailyos.submit.claim_feedback",
            "dailyos.submit.note",
            "dailyos.submit.action",
            "dailyos.submit.action_status",
        ] {
            assert!(
                registered.contains(&ScopedName::new(name)),
                "missing registered W5 tool: {name}"
            );
        }
        assert!(!registered.contains(&ScopedName::new("dailyos.write.place_document")));
        let pending = gateway.seal().expect("registered handlers match catalog");
        assert!(
            pending.is_empty(),
            "W5 catalog entries should all have registered handlers: {pending:?}"
        );
    }
}
