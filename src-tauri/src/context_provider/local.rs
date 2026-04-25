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

            // DOS-15 IL-check-3: Surface prior Glean leading signals so the next
            // enrichment pass can detect drift and update stale signals.
            // Only inject the high-signal fields (champion_risk, channel_sentiment,
            // commercial_signals) — not the full blob — to stay within token budget.
            if let Ok(Some(signals_json)) = db.conn_ref().query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                [entity_id],
                |row| row.get::<_, Option<String>>(0),
            ) {
                if let Ok(signals) = serde_json::from_str::<
                    crate::intelligence::glean_leading_signals::HealthOutlookSignals,
                >(&signals_json)
                {
                    let mut block_parts: Vec<String> =
                        vec!["## Prior Glean Leading Signals (drift detection)".to_string()];

                    if let Some(cr) = &signals.champion_risk {
                        if cr.at_risk {
                            block_parts.push(format!(
                                "Champion risk: {} — level={}, evidence_count={}",
                                cr.champion_name.as_deref().unwrap_or("unknown"),
                                cr.risk_level.as_deref().unwrap_or("unknown"),
                                cr.risk_evidence.len()
                            ));
                        }
                    }

                    if let Some(cs) = &signals.channel_sentiment {
                        if cs.divergence_detected {
                            block_parts.push(format!(
                                "Channel divergence: {}",
                                cs.divergence_summary
                                    .as_deref()
                                    .unwrap_or("divergence detected")
                            ));
                        }
                    }

                    if let Some(comm) = &signals.commercial_signals {
                        if let Some(dir) = &comm.arr_direction {
                            block_parts.push(format!("ARR direction: {}", dir));
                        }
                        if let Some(pay) = &comm.payment_behavior {
                            if pay != "on-time" && pay != "unknown" {
                                block_parts.push(format!("Payment behavior: {}", pay));
                            }
                        }
                    }

                    if block_parts.len() > 1 {
                        ctx.extra_blocks.push(block_parts.join("\n"));
                    }
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
