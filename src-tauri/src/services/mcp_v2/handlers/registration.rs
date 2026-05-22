//! Wave-scoped handler registration helper.
//!
//! Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)`
//! and `gateway.seal()`. Each wave (W2-A, W2-B, ...) adds its handlers here.

use std::sync::Arc;

use crate::services::mcp_v2::contracts::ScopedName;
use crate::services::mcp_v2::gateway::Gateway;
use crate::services::mcp_v2::taxonomy::TaxonomyCatalog;

use super::tool_account_status::AccountStatusHandler;

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

/// Register all v1.4.7 W2-A Phase-A handlers on the gateway.
///
/// Cycle-1 scope: `dailyos.read.account_status` only. Phase-B (daily_briefing)
/// adds its handler via this same helper once its sub-ticket lands.
pub fn register_v147_handlers(
    gateway: &mut Gateway,
    catalog: &Arc<dyn TaxonomyCatalog>,
    runtime: tokio::runtime::Handle,
) -> Result<(), RegistrationError> {
    let account_status_name = ScopedName::new("dailyos.read.account_status");
    let description = catalog
        .description_for(&account_status_name)
        .ok_or_else(|| RegistrationError::CatalogEntryMissing(account_status_name.clone()))?
        .clone();

    let handler = AccountStatusHandler::from_runtime(description, runtime)
        .map_err(RegistrationError::AbilityRegistry)?;
    gateway.register(Arc::new(handler));

    Ok(())
}
