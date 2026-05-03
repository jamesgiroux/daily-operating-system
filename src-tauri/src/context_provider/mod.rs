//! Context provider abstraction for dual-mode operation (ADR-0095).
//!
//! Two orthogonal abstractions:
//! - **ContextProvider** (this module) — where entity context is gathered
//!   (local DB/files vs. Glean search)
//! - **IntelligenceProvider** (ADR-0091) — how assembled context is synthesized
//!   into intelligence (Claude Code vs. Ollama vs. OpenAI)

pub mod cache;
pub mod glean;
pub mod local;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::db::ActionDb;
use crate::intelligence::prompts::IntelligenceContext;
use crate::intelligence::IntelligenceJson;

// ---------------------------------------------------------------------------
// Context mode configuration
// ---------------------------------------------------------------------------

/// Operating mode for context gathering.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode")]
pub enum ContextMode {
    /// Local-first: all context gathered from local DB + workspace files.
    /// This is today's behavior.
    #[default]
    Local,
    /// Enterprise: Glean as primary context source.
    /// DCR handles client registration — no user-provided client_id needed.
    /// Always additive: Glean primary + local signals merged (Gmail/Linear/Calendar still active).
    Glean { endpoint: String },
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from context gathering operations.
#[derive(Debug)]
pub enum ContextError {
    /// Database error during context assembly.
    Db(String),
    /// Network timeout reaching external service (e.g., Glean).
    Timeout(String),
    /// Authentication failure (expired token, missing keychain entry).
    Auth(String),
    /// Generic provider error.
    Other(String),
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(msg) => write!(f, "context db error: {}", msg),
            Self::Timeout(msg) => write!(f, "context timeout: {}", msg),
            Self::Auth(msg) => write!(f, "context auth error: {}", msg),
            Self::Other(msg) => write!(f, "context error: {}", msg),
        }
    }
}

impl std::error::Error for ContextError {}

// ---------------------------------------------------------------------------
// ContextProvider trait
// ---------------------------------------------------------------------------

/// Trait for gathering entity context from different sources.
///
/// Implementations:
/// - `LocalContextProvider` — gathers from local SQLite DB + workspace files (default)
/// - `GleanContextProvider` — gathers from Glean search API (enterprise)
pub trait ContextProvider: Send + Sync {
    /// Gather all context signals for an entity.
    ///
    /// Returns an `IntelligenceContext` ready for prompt construction.
    /// This is the only method called by `intel_queue.rs` during enrichment.
    fn gather_entity_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        prior: Option<&IntelligenceJson>,
    ) -> Result<IntelligenceContext, ContextError>;

    /// Human-readable name for logging.
    fn provider_name(&self) -> &str;

    /// Whether this provider makes network calls (affects error handling strategy).
    fn is_remote(&self) -> bool;

    /// MCP endpoint URL for remote providers. Returns None for local providers.
    fn remote_endpoint(&self) -> Option<&str> {
        None
    }
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

/// Read the current context mode from DB. Returns `ContextMode::Local` if not set.
pub fn read_context_mode(db: &ActionDb) -> ContextMode {
    db.conn_ref()
        .query_row(
            "SELECT mode_json FROM context_mode_config WHERE id = 1",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<ContextMode>(&json).ok())
        .unwrap_or_default()
}

/// Save context mode to DB. Requires app restart to take effect.
pub fn save_context_mode(db: &ActionDb, mode: &ContextMode) -> Result<(), String> {
    let json = match mode {
        ContextMode::Local => None,
        other => Some(
            serde_json::to_string(other)
                .map_err(|e| format!("Failed to serialize context mode: {}", e))?,
        ),
    };

    db.conn_ref()
        .execute(
            "INSERT OR REPLACE INTO context_mode_config (id, mode_json, updated_at)
             VALUES (1, ?1, datetime('now'))",
            rusqlite::params![json],
        )
        .map_err(|e| format!("Failed to save context mode: {}", e))?;

    Ok(())
}
