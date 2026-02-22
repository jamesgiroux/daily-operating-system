//! General-purpose item relevance scorer (I395).
//!
//! Composes existing signal infrastructure — entity linkage counts, embedding
//! cosine similarity, AI urgency mapping, keyword matching, and time decay —
//! to compute a normalized 0.0–1.0 relevance score for any item.

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::signals::decay;

/// Context needed to score a single item.
pub struct ScoringContext<'a> {
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub content_text: &'a str,
    pub urgency: Option<&'a str>,
    pub sentiment: Option<&'a str>,
    pub created_at: &'a str,
}

/// Result of scoring a single item.
pub struct ScoredItem {
    pub total: f64,
    pub entity_score: f64,
    pub relevance_score: f64,
    pub urgency_score: f64,
    pub keyword_score: f64,
    pub recency_score: f64,
    pub reason: String,
}

/// Business keywords and their relevance weights.
const KEYWORD_WEIGHTS: &[(&str, f64)] = &[
    ("renewal", 0.15),
    ("contract", 0.12),
    ("expansion", 0.12),
    ("escalation", 0.12),
    ("churn", 0.12),
    ("qbr", 0.10),
    ("order form", 0.10),
    ("deadline", 0.08),
    ("budget", 0.08),
    ("executive", 0.06),
];

/// Score a single item against today's context.
///
/// Scoring dimensions:
/// - Entity linkage (0.0–0.30): signal_events count for entity
/// - Meeting relevance (0.0–0.25): embedding cosine similarity to meeting context
/// - AI urgency (0.0–0.20): mapped from enrichment urgency field
/// - Keyword relevance (0.0–0.15): business term matching
/// - Recency (0.0–0.10): exponential decay with 14-day half-life
pub fn score_item(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    ctx: &ScoringContext,
    todays_meeting_context: &str,
) -> ScoredItem {
    let mut reasons = Vec::new();

    // 1. Entity linkage (0.0–0.30)
    let entity_score = if let (Some(eid), Some(etype)) = (ctx.entity_id, ctx.entity_type) {
        let signal_count = count_entity_signals(db, eid, etype);
        let score = match signal_count {
            0 => 0.10,  // Entity exists but no updates yet
            1..=3 => 0.20,
            _ => 0.30,
        };
        // Resolve entity name for product-vocabulary reason string (ADR-0083)
        let entity_label = resolve_entity_name(db, eid, etype);
        reasons.push(entity_label);
        score
    } else {
        0.0
    };

    // 2. Meeting relevance (0.0–0.25) — embedding similarity
    let relevance_score = if !todays_meeting_context.is_empty() && !ctx.content_text.is_empty() {
        if let Some(m) = model {
            let sim = compute_embedding_similarity(m, ctx.content_text, todays_meeting_context);
            let score = (sim.max(0.0) * 0.25).min(0.25);
            if score > 0.05 {
                reasons.push("relates to today's meetings".to_string());
            }
            score
        } else {
            0.0
        }
    } else {
        0.0
    };

    // 3. AI urgency (0.0–0.20)
    let urgency_score = match ctx.urgency {
        Some("high") => {
            reasons.push("urgent".to_string());
            0.20
        }
        Some("medium") => 0.08,
        Some("low") => 0.02,
        _ => 0.0,
    };

    // 4. Keyword relevance (0.0–0.15)
    let keyword_score = if !ctx.content_text.is_empty() {
        let lower = ctx.content_text.to_lowercase();
        let mut best_score = 0.0_f64;
        let mut best_keyword = "";
        for &(keyword, weight) in KEYWORD_WEIGHTS {
            if lower.contains(keyword) && weight > best_score {
                best_score = weight;
                best_keyword = keyword;
            }
        }
        if best_score > 0.0 {
            // Product vocabulary (ADR-0083): just the topic, not "X keyword"
            reasons.push(best_keyword.to_string());
        }
        best_score
    } else {
        0.0
    };

    // 5. Recency (0.0–0.10) — 14-day half-life decay
    let age = decay::age_days_from_now(ctx.created_at);
    let recency_score = decay::decayed_weight(0.10, age, 14.0);

    // Total clamped to [0.0, 1.0]
    let total = (entity_score + relevance_score + urgency_score + keyword_score + recency_score)
        .clamp(0.0, 1.0);

    let reason = if reasons.is_empty() {
        String::new()
    } else {
        reasons.join(" · ")
    };

    ScoredItem {
        total,
        entity_score,
        relevance_score,
        urgency_score,
        keyword_score,
        recency_score,
        reason,
    }
}

/// Count signal_events for a given entity.
fn count_entity_signals(db: &ActionDb, entity_id: &str, entity_type: &str) -> i64 {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND entity_type = ?2",
            rusqlite::params![entity_id, entity_type],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

/// Compute cosine similarity between two texts using the embedding model.
fn compute_embedding_similarity(model: &EmbeddingModel, text_a: &str, text_b: &str) -> f64 {
    let prefix_d = "search_document: ";
    let prefix_q = "search_query: ";

    let vec_a = match model.embed(&format!("{}{}", prefix_d, text_a)) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };
    let vec_b = match model.embed(&format!("{}{}", prefix_q, text_b)) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };

    cosine_similarity(&vec_a, &vec_b)
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

/// Resolve an entity ID to a human-readable name (ADR-0083: product vocabulary).
/// Falls back to the entity type label if name lookup fails.
fn resolve_entity_name(db: &ActionDb, entity_id: &str, entity_type: &str) -> String {
    match entity_type {
        "account" => db
            .get_account(entity_id)
            .ok()
            .flatten()
            .map(|a| a.name)
            .unwrap_or_else(|| "known account".to_string()),
        "person" => db
            .get_person(entity_id)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_else(|| "known contact".to_string()),
        "project" => db
            .get_project(entity_id)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_else(|| "known project".to_string()),
        _ => "known contact".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_matching() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "Discussing the renewal timeline for Q3",
            urgency: None,
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let result = score_item(&db, None, &ctx, "");
        assert!(result.keyword_score > 0.0, "renewal keyword should score > 0");
        assert!(result.reason.contains("renewal"));
    }

    #[test]
    fn test_urgency_scoring() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "",
            urgency: Some("high"),
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let result = score_item(&db, None, &ctx, "");
        assert!((result.urgency_score - 0.20).abs() < 0.001);
    }

    #[test]
    fn test_total_clamped() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "renewal contract expansion escalation",
            urgency: Some("high"),
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let result = score_item(&db, None, &ctx, "");
        assert!(result.total <= 1.0, "total should be clamped to 1.0");
        assert!(result.total >= 0.0);
    }
}
