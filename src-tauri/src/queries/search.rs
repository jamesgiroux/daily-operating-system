use std::collections::HashMap;

use serde::Serialize;

use crate::db::{ActionDb, DbContentEmbedding};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentMatch {
    pub content_file_id: String,
    pub filename: String,
    pub relative_path: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub vector_score: f32,
    pub text_score: f32,
    pub combined_score: f32,
    pub modified_at: String,
    pub content_type: String,
}

pub fn search_entity_content(
    db: &ActionDb,
    model_opt: Option<&crate::embeddings::EmbeddingModel>,
    entity_id: &str,
    query: &str,
    top_k: usize,
    vector_weight: f32,
    text_weight: f32,
) -> Result<Vec<ContentMatch>, String> {
    let files = db
        .get_entity_files(entity_id)
        .map_err(|e| format!("get_entity_files failed: {e}"))?;
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let file_map: HashMap<String, crate::db::DbContentFile> =
        files.into_iter().map(|f| (f.id.clone(), f)).collect();

    let mut chunks = db
        .get_entity_embedding_chunks(entity_id)
        .map_err(|e| format!("get_entity_embedding_chunks failed: {e}"))?;

    // Fallback corpus when embeddings are not present yet.
    if chunks.is_empty() {
        for file in file_map.values() {
            if let Some(summary) = file.summary.as_ref() {
                chunks.push(DbContentEmbedding {
                    id: format!("{}-summary", file.id),
                    content_file_id: file.id.clone(),
                    chunk_index: -1,
                    chunk_text: summary.clone(),
                    embedding: Vec::new(),
                    created_at: file.modified_at.clone(),
                });
            }
        }
    }

    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let text_scores = compute_text_scores(&chunks, query);

    let query_embedding = model_opt
        .filter(|m| m.is_ready())
        .and_then(|m| m.embed(query).ok());

    let mut out: Vec<ContentMatch> = Vec::new();
    for chunk in chunks {
        let Some(file) = file_map.get(&chunk.content_file_id) else {
            continue;
        };

        let vector_score = query_embedding
            .as_ref()
            .and_then(|q| {
                crate::embeddings::blob_to_f32_vec(&chunk.embedding)
                    .ok()
                    .map(|v| (q, v))
            })
            .map(|(q, v)| crate::embeddings::cosine_similarity(q, &v))
            .unwrap_or(0.0)
            .max(0.0);

        let text_score = *text_scores.get(&chunk.id).unwrap_or(&0.0);
        let combined_score = vector_weight * vector_score + text_weight * text_score;

        out.push(ContentMatch {
            content_file_id: chunk.content_file_id,
            filename: file.filename.clone(),
            relative_path: file.relative_path.clone(),
            chunk_index: chunk.chunk_index,
            chunk_text: chunk.chunk_text,
            vector_score,
            text_score,
            combined_score,
            modified_at: file.modified_at.clone(),
            content_type: file.content_type.clone(),
        });
    }

    out.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out.truncate(top_k);
    Ok(out)
}

fn compute_text_scores(chunks: &[DbContentEmbedding], query: &str) -> HashMap<String, f32> {
    let q_terms = tokenize(query);
    if q_terms.is_empty() {
        return HashMap::new();
    }

    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let mut tokenized_docs: HashMap<String, Vec<String>> = HashMap::new();

    for chunk in chunks {
        let tokens = tokenize(&chunk.chunk_text);
        let uniq: std::collections::HashSet<String> = tokens.iter().cloned().collect();
        for term in uniq {
            *doc_freq.entry(term).or_insert(0) += 1;
        }
        tokenized_docs.insert(chunk.id.clone(), tokens);
    }

    let n_docs = chunks.len() as f32;
    let mut raw_scores: HashMap<String, f32> = HashMap::new();
    let mut max_score = 0.0_f32;

    for chunk in chunks {
        let tokens = tokenized_docs.get(&chunk.id).cloned().unwrap_or_default();
        if tokens.is_empty() {
            raw_scores.insert(chunk.id.clone(), 0.0);
            continue;
        }

        let doc_len = tokens.len() as f32;
        let mut tf_map: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *tf_map.entry(token).or_insert(0) += 1;
        }

        let mut score = 0.0_f32;
        for term in &q_terms {
            let tf = *tf_map.get(term).unwrap_or(&0) as f32;
            if tf == 0.0 {
                continue;
            }
            let df = *doc_freq.get(term).unwrap_or(&0) as f32;
            let idf = ((n_docs + 1.0) / (df + 1.0)).ln() + 1.0;
            // BM25-like normalization without external dependency.
            let tf_norm = tf / (tf + 1.2 * (0.25 + 0.75 * (doc_len / 200.0)));
            score += idf * tf_norm;
        }

        if score > max_score {
            max_score = score;
        }
        raw_scores.insert(chunk.id.clone(), score);
    }

    if max_score <= 0.0 {
        return raw_scores;
    }

    raw_scores
        .into_iter()
        .map(|(id, score)| (id, (score / max_score).clamp(0.0, 1.0)))
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Risk: renewal blocked!");
        assert_eq!(tokens, vec!["risk", "renewal", "blocked"]);
    }
}
