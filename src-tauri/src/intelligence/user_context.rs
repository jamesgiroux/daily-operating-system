//! Semantic retrieval of user context entries for enrichment prompts (I417).
//!
//! Searches `user_context_entries` by cosine similarity against embedded content,
//! returning top-K matches above a threshold for injection into intelligence prompts.

use crate::db::ActionDb;
use crate::embeddings::{blob_to_f32_vec, cosine_similarity, EmbeddingModel, QUERY_PREFIX};

/// A matched user context entry with its similarity score.
pub struct UserContextMatch {
    pub title: String,
    pub content: String,
    pub score: f32,
}

/// Search user context entries by semantic similarity to a query string.
///
/// Returns up to `limit` entries with cosine similarity >= `threshold`.
/// If the embedding model is unavailable or no entries have embeddings, returns empty.
pub fn search_user_context(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    query: &str,
    limit: usize,
    threshold: f32,
) -> Vec<UserContextMatch> {
    let model = match model.filter(|m| m.is_ready()) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let prefixed = format!("{}{}", QUERY_PREFIX, query);
    let query_vec = match model.embed(&prefixed) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    // Read all context entries with embeddings
    let rows: Vec<(String, String, Vec<u8>)> = match db.conn_ref().prepare(
        "SELECT title, content, embedding FROM user_context_entries WHERE embedding IS NOT NULL",
    ) {
        Ok(mut stmt) => stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .ok()
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default(),
        Err(_) => return Vec::new(),
    };

    let mut matches: Vec<UserContextMatch> = rows
        .into_iter()
        .filter_map(|(title, content, blob)| {
            let vec = blob_to_f32_vec(&blob).ok()?;
            let score = cosine_similarity(&query_vec, &vec);
            if score >= threshold {
                Some(UserContextMatch {
                    title,
                    content,
                    score,
                })
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches.truncate(limit);
    matches
}
