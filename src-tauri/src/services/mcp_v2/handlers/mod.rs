//! Per-tool MCP handler modules. Each handler implements McpToolHandler.
//! Module names match the canonical scoped names declared in taxonomy.

pub mod registration;
pub mod tool_account_status;
pub mod tool_briefing;
pub mod tool_create_action;
pub mod tool_note;
pub mod tool_pagination;
pub mod tool_placement;
pub mod tool_portfolio;
pub mod tool_resources;
pub mod tool_update_action_status;
pub mod tool_workspace_search;
pub mod tool_workspace_source_provenance;
