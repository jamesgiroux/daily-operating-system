//! Embedding-based signal relevance scoring (I308 — ADR-0080 Phase 4).
//!
//! Ranks signals by cosine similarity to a meeting context string using
//! the local embedding model.

use crate::embeddings::EmbeddingModel;

use super::bus::SignalEvent;

/// Prefix for query embeddings (nomic-embed-text-v1.5).
const QUERY_PREFIX: &str = "search_query: ";
/// Prefix for document embeddings.
const DOCUMENT_PREFIX: &str = "search_document: ";

/// Rank signals by embedding similarity to meeting context.
///
/// Returns signals paired with their relevance score, sorted descending.
/// Signals without value text get a baseline score of 0.0.
pub fn rank_signals_by_relevance(
    model: &EmbeddingModel,
    meeting_context: &str,
    signals: &[SignalEvent],
) -> Vec<(SignalEvent, f64)> {
    if signals.is_empty() || meeting_context.is_empty() {
        return signals.iter().map(|s| (s.clone(), 0.0)).collect();
    }

    // Embed the meeting context as a query
    let query_text = format!("{}{}", QUERY_PREFIX, meeting_context);
    let query_vec = match model.embed(&query_text) {
        Ok(v) => v,
        Err(_) => return signals.iter().map(|s| (s.clone(), 0.0)).collect(),
    };

    // Embed each signal's value text as a document
    let mut scored: Vec<(SignalEvent, f64)> = signals
        .iter()
        .map(|signal| {
            let score = signal
                .value
                .as_deref()
                .and_then(|v| {
                    let doc_text = format!("{}{} {}", DOCUMENT_PREFIX, signal.signal_type, v);
                    model.embed(&doc_text).ok()
                })
                .map(|doc_vec| cosine_similarity(&query_vec, &doc_vec))
                .unwrap_or(0.0);
            (signal.clone(), score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_rank_empty_signals() {
        let model = EmbeddingModel::new();
        let result = rank_signals_by_relevance(&model, "test context", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_rank_empty_context() {
        let model = EmbeddingModel::new();
        let signal = SignalEvent {
            id: "sig-1".to_string(),
            entity_type: "account".to_string(),
            entity_id: "a1".to_string(),
            signal_type: "stakeholder_change".to_string(),
            source: "propagation".to_string(),
            value: Some("Alice promoted to CRO".to_string()),
            confidence: 0.85,
            decay_half_life_days: 30,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            superseded_by: None,
            source_context: None,
        };
        let result = rank_signals_by_relevance(&model, "", &[signal]);
        assert_eq!(result.len(), 1);
        assert!((result[0].1 - 0.0).abs() < 0.001);
    }
}
