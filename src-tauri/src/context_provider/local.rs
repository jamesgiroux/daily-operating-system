//! Local context provider — gathers entity context from SQLite DB + workspace files.
//!
//! This is a thin wrapper around `build_intelligence_context()` from `intelligence/prompts.rs`.
//! It preserves today's exact behavior while implementing the `ContextProvider` trait.

use std::path::PathBuf;
use std::sync::Arc;

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::intelligence::prompts::IntelligenceContext;
use crate::intelligence::IntelligenceJson;

use super::{ContextError, ContextProvider};

/// Local context provider: all data from SQLite + workspace files.
pub struct LocalContextProvider {
    workspace: PathBuf,
    embedding_model: Arc<EmbeddingModel>,
}

impl LocalContextProvider {
    pub fn new(workspace: PathBuf, embedding_model: Arc<EmbeddingModel>) -> Self {
        Self {
            workspace,
            embedding_model,
        }
    }
}

impl ContextProvider for LocalContextProvider {
    fn gather_entity_context(
        &self,
        db: &ActionDb,
        entity_id: &str,
        entity_type: &str,
        prior: Option<&IntelligenceJson>,
    ) -> Result<IntelligenceContext, ContextError> {
        // Look up entity details from DB (same as intel_queue.rs does today)
        let account = if entity_type == "account" {
            db.get_account(entity_id).ok().flatten()
        } else {
            None
        };

        let project = if entity_type == "project" {
            db.get_project(entity_id).ok().flatten()
        } else {
            None
        };

        // Delegate to the existing function (no behavior change)
        let embedding = if self.embedding_model.is_ready() {
            Some(self.embedding_model.as_ref())
        } else {
            None
        };

        let mut ctx = crate::intelligence::build_intelligence_context(
            &self.workspace,
            db,
            entity_id,
            entity_type,
            account.as_ref(),
            project.as_ref(),
            prior,
            embedding,
        );

        // I500: Load org_health from DB if available (may have been stored by a prior Glean enrichment)
        if entity_type == "account" {
            if let Ok(Some(json)) = db.conn_ref().query_row(
                "SELECT org_health_json FROM entity_assessment WHERE entity_id = ?1",
                [entity_id],
                |row| row.get::<_, Option<String>>(0),
            ) {
                if let Ok(org_health) = serde_json::from_str(&json) {
                    ctx.org_health = Some(org_health);
                }
            }
        }

        Ok(ctx)
    }

    fn provider_name(&self) -> &str {
        "local"
    }

    fn is_remote(&self) -> bool {
        false
    }
}
