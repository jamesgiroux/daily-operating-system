//! MCP tool taxonomy + host-selection contract — W1-A handler-author surface.
//!
//! The catalog of tool descriptions — the version-controlled product copy
//! per ADR-0128 §3 — loads from a YAML file at runtime. This module
//! defines two things owned by W1-A:
//!
//! 1. The [`TaxonomyCatalog`] trait — the abstract catalog interface the
//!    gateway consumes at boot. W1-B (DOS-478) ships the concrete YAML
//!    loader that implements this trait.
//! 2. The [`TaxonomyError`] type — used to fail-fast at startup if
//!    handler registrations don't match the loaded catalog.
//!
//! Handler bodies live in subsequent waves (W2 / W3 / W4) but must
//! follow the handler-contract conventions documented on
//! [`TaxonomyCatalog`] below.

use super::contracts::{McpToolHandler, ScopedName, Side};

/// Errors surfaced when the loaded tool catalog disagrees with the
/// registered [`McpToolHandler`] implementations at startup.
///
/// These are operator-facing — they indicate a mismatch between the
/// version-controlled YAML catalog (W1-B) and the compiled handler
/// registry (W2 / W3 / W4). The gateway treats this as fatal at boot
/// (no half-served catalog) and surfaces the error verbatim to the
/// operator log.
#[derive(Debug, Clone, PartialEq)]
pub enum TaxonomyError {
    /// A registered handler's `ScopedName` has no matching entry in
    /// the loaded catalog. Either the handler is renamed without a
    /// catalog update, or the catalog is stale. Fix one or the other.
    HandlerCatalogMismatch {
        handler: ScopedName,
        catalog_entry: Option<ScopedName>,
    },
}

/// The abstract catalog interface the gateway uses at boot. The
/// concrete implementation is a YAML loader shipped by W1-B; W1-A only
/// freezes the validation seam between handlers and catalog.
///
/// # Handler-contract conventions (read before writing a handler in W2 / W3 / W4)
///
/// Every concrete [`McpToolHandler`] in `services::mcp_v2::handlers::*`
/// must obey the following rules. The gateway enforces them at
/// dispatch time; failure to follow them produces operator-visible
/// warnings or rejected calls.
///
/// ## Side::Read handlers
///
/// Read tools may return any JSON shape compatible with the
/// `ToolDescription.returns.schema` advertised in the catalog. There
/// is no `mutation_cursor` requirement — `Side::Read` invocations
/// produce no audit cursor and the gateway omits the cursor field from
/// the audit detail JSON. Example:
///
/// ```ignore
/// // services/mcp_v2/handlers/tool_account_status.rs
/// impl McpToolHandler for AccountStatusHandler {
///     fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
///         let account = lookup_account(params)?;
///         Ok(serde_json::json!({
///             "account_id": account.id,
///             "status": account.status,
///             "as_of": account.fresh_at,
///         }))
///     }
/// }
/// ```
///
/// ## Side::Write and Side::SubmitCorrection handlers — `mutation_cursor` field
///
/// Write and submit-correction tools must include a `mutation_cursor`
/// field in their success payload. The gateway extracts it via JSON
/// inspection (`result_value.get("mutation_cursor")`) and folds it
/// into the audit detail JSON as forensic ordering material. Omitting
/// it produces a `Suite-S` warning signal
/// (`mcp_write_handler_missing_cursor`); the call still succeeds
/// (handler effects are not reverted) but the cursor is absent from
/// the audit row.
///
/// **Cursor shape: IDs only, no payload data.** The cursor value
/// must be a JSON object whose values are stable substrate IDs
/// (UUIDs, integer primary keys, opaque hashes). Never embed user
/// content, email addresses, names, or any PII-shaped string. The
/// gateway caps the cursor at depth 4 and 2 KiB serialized; oversize
/// cursors are replaced with `"truncated_oversize"` in the audit
/// detail and a `Suite-S` warning is emitted.
///
/// ## Cursor shape per service
///
/// | Service the handler calls | Cursor key | Source |
/// |---|---|---|
/// | `services::claims::commit_claim` | `claim_id` | `CommittedClaim.new_claim_id` (per `services/claims.rs:209`) |
/// | `services::signals::emit*` | `signal_id` | the `String` return value of the emit facade (per `services/signals.rs:46-106`) |
/// | `services::actions::*` | `action_id` | the action ID returned by the service |
/// | composite (e.g. claim + signal in one call) | nested cursor | combine sub-cursors |
///
/// ## Concrete `Side::Write` example
///
/// ```ignore
/// // services/mcp_v2/handlers/tool_create_action.rs
/// impl McpToolHandler for CreateActionHandler {
///     fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
///         let action_id = services::actions::create(parse(params)?)?;
///         Ok(serde_json::json!({
///             "action_id": action_id,
///             "mutation_cursor": { "action_id": action_id },
///         }))
///     }
/// }
/// ```
///
/// ## Concrete composite-mutation example (a write that emits both a
/// claim and a downstream signal)
///
/// ```ignore
/// let claim = services::claims::commit_claim(...)?;
/// let signal_id = services::signals::emit_observation(...)?;
/// Ok(serde_json::json!({
///     "claim_id": claim.new_claim_id,
///     "signal_id": signal_id,
///     "mutation_cursor": {
///         "claim_id": claim.new_claim_id,
///         "signal_id": signal_id,
///     },
/// }))
/// ```
///
/// The handler decides which sub-mutations it considers
/// audit-relevant; the gateway does not enforce a composition shape
/// beyond the depth and size caps.
pub trait TaxonomyCatalog: Send + Sync {
    /// Verify that every registered [`McpToolHandler`] in the registry
    /// has a matching catalog entry. Called by the gateway at startup
    /// before serving any MCP traffic.
    ///
    /// Returns `Ok(())` if every handler's
    /// [`McpToolHandler::description`] `.name` appears in the loaded
    /// catalog with a matching [`Side`] tier. Returns
    /// [`TaxonomyError::HandlerCatalogMismatch`] on the first
    /// mismatch.
    fn validate_against_handlers(
        &self,
        handlers: &[&dyn McpToolHandler],
    ) -> Result<(), TaxonomyError>;

    /// Look up the catalog-declared [`Side`] tier for a tool by name,
    /// if present. Returns `None` if the catalog has no entry for the
    /// given name.
    fn side_for(&self, tool_name: &ScopedName) -> Option<Side>;
}
