//! Coherence detection for entity assessment (I407).
//!
//! Validates that entity assessment is semantically coherent with
//! the entity's actual meeting history using embedding similarity.

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;

/// Result of a coherence check.
#[derive(Debug)]
pub struct CoherenceResult {
    pub entity_id: String,
    pub score: f64,
    pub passed: bool,
}

/// Minimum cosine similarity for intelligence to be considered coherent.
const COHERENCE_THRESHOLD: f64 = 0.30;

/// Compute raw coherence score between an entity's executive assessment
/// and its recent meeting corpus.
///
/// Returns Ok(1.0) if insufficient data (< 2 meetings or no assessment) — not a failure.
pub fn coherence_check(
    db: &ActionDb,
    embedding_model: &EmbeddingModel,
    entity_id: &str,
) -> Result<f64, String> {
    // Get executive assessment from entity_assessment
    let assessment: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT executive_assessment FROM entity_assessment WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| row.get(0),
        )
        .ok()
        .flatten();

    let assessment = match assessment {
        Some(a) if !a.trim().is_empty() => a,
        _ => return Ok(1.0), // No assessment — skip, not a failure
    };

    // Get linked meetings from the last 90 days
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT mh.title, mt.summary FROM meetings mh
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = mh.id
             INNER JOIN meeting_entities me ON me.meeting_id = mh.id
             WHERE me.entity_id = ?1
               AND mh.start_time > datetime('now', '-90 days')
             ORDER BY mh.start_time DESC
             LIMIT 20",
        )
        .map_err(|e| format!("Failed to query meetings: {}", e))?;

    let meetings: Vec<String> = stmt
        .query_map(rusqlite::params![entity_id], |row| {
            let title: String = row.get(0)?;
            let summary: Option<String> = row.get(1)?;
            Ok(format!("{} {}", title, summary.unwrap_or_default()))
        })
        .map_err(|e| format!("Failed to read meetings: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if meetings.len() < 2 {
        return Ok(1.0); // Insufficient meeting data — skip
    }

    // Embed assessment and meeting corpus
    let assessment_text = format!("{}{}", crate::embeddings::DOCUMENT_PREFIX, assessment);
    let corpus = meetings.join(" ");
    let corpus_text = format!("{}{}", crate::embeddings::DOCUMENT_PREFIX, corpus);

    let assessment_vec = embedding_model.embed(&assessment_text)?;
    let corpus_vec = embedding_model.embed(&corpus_text)?;

    let similarity = crate::embeddings::cosine_similarity(&assessment_vec, &corpus_vec) as f64;
    Ok(similarity)
}

/// Run a full coherence check: compute score, update DB, return result.
///
/// Gracefully returns passed with score 1.0 if embedding model unavailable.
pub fn run_coherence_check(
    db: &ActionDb,
    embedding_model: Option<&EmbeddingModel>,
    entity_id: &str,
) -> Result<CoherenceResult, String> {
    let model = match embedding_model {
        Some(m) if m.is_ready() => m,
        _ => {
            return Ok(CoherenceResult {
                entity_id: entity_id.to_string(),
                score: 1.0,
                passed: true,
            });
        }
    };

    let score = coherence_check(db, model, entity_id)?;
    let passed = score >= COHERENCE_THRESHOLD;

    // Update entity_quality with coherence results
    let _ = db.conn_ref().execute(
        "INSERT OR IGNORE INTO entity_quality (entity_id, entity_type)
         SELECT entity_id, entity_type FROM entity_assessment WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    );
    let _ = db.conn_ref().execute(
        "UPDATE entity_quality SET coherence_score = ?1, coherence_flagged = ?2
         WHERE entity_id = ?3",
        rusqlite::params![score, if passed { 0 } else { 1 }, entity_id],
    );

    Ok(CoherenceResult {
        entity_id: entity_id.to_string(),
        score,
        passed,
    })
}
