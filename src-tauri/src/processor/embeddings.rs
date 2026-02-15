//! Background embedding generation processor (Sprint 26).
//!
//! Trigger model:
//! - watcher enqueue for account/project content changes
//! - startup enqueue for entities with content
//! - periodic sweep every 5 minutes for missed updates

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use tauri::AppHandle;

use crate::db::{ActionDb, DbContentEmbedding, DbContentFile};
use crate::state::AppState;

const STARTUP_DELAY_SECS: u64 = 20;
const IDLE_POLL_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub entity_id: String,
    pub entity_type: String,
    pub requested_at: Instant,
}

pub struct EmbeddingQueue {
    queue: Mutex<VecDeque<EmbeddingRequest>>,
    last_enqueued: Mutex<HashMap<String, Instant>>,
}

impl EmbeddingQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            last_enqueued: Mutex::new(HashMap::new()),
        }
    }

    pub fn enqueue(&self, request: EmbeddingRequest) {
        let mut queue = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return,
        };

        if queue.iter().any(|r| r.entity_id == request.entity_id) {
            return;
        }

        queue.push_back(request.clone());
        if let Ok(mut guard) = self.last_enqueued.lock() {
            guard.insert(request.entity_id, Instant::now());
        }
    }

    pub fn dequeue(&self) -> Option<EmbeddingRequest> {
        self.queue.lock().ok()?.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}

/// Chunk text by approximate token count using whitespace token estimation.
///
/// Token estimation intentionally avoids external tokenizer dependencies.
pub fn chunk_text_by_token_estimate(
    text: &str,
    chunk_tokens: usize,
    overlap_tokens: usize,
) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return Vec::new();
    }

    if chunk_tokens == 0 {
        return vec![tokens.join(" ")];
    }

    let step = chunk_tokens.saturating_sub(overlap_tokens).max(1);
    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < tokens.len() {
        let end = (start + chunk_tokens).min(tokens.len());
        let chunk = tokens[start..end].join(" ");
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        if end == tokens.len() {
            break;
        }
        start += step;
    }

    chunks
}

pub async fn run_embedding_processor(state: Arc<AppState>, _app: AppHandle) {
    tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;
    log::info!("EmbeddingProcessor: started");

    let mut last_sweep_at = Instant::now()
        .checked_sub(Duration::from_secs(60 * 60))
        .unwrap_or_else(Instant::now);

    loop {
        let config = match state.config.read().ok().and_then(|g| g.clone()) {
            Some(c) => c,
            None => {
                tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
                continue;
            }
        };

        if !config.embeddings.enabled {
            tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
            continue;
        }

        let sweep_interval = Duration::from_secs(config.embeddings.sweep_interval_secs.max(30));
        if last_sweep_at.elapsed() >= sweep_interval {
            if let Err(e) = enqueue_sweep_candidates(&state, config.embeddings.max_files_per_sweep)
            {
                log::warn!("EmbeddingProcessor: sweep enqueue failed: {}", e);
            }
            last_sweep_at = Instant::now();
        }

        let request = state.embedding_queue.dequeue();
        if let Some(request) = request {
            match process_request(&state, &request) {
                Ok(updated) => {
                    if updated > 0 {
                        log::info!(
                            "EmbeddingProcessor: updated {} files for {}",
                            updated,
                            request.entity_id
                        );
                    }
                }
                Err(e) => {
                    log::debug!("EmbeddingProcessor: skipped {} ({})", request.entity_id, e);
                }
            }
            continue;
        }

        tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
    }
}

fn enqueue_sweep_candidates(state: &AppState, max_files: usize) -> Result<(), String> {
    let db = ActionDb::open().map_err(|e| format!("open db failed: {e}"))?;
    let files = db
        .get_files_needing_embeddings(max_files)
        .map_err(|e| format!("query files needing embeddings failed: {e}"))?;

    let mut entities = std::collections::HashSet::new();
    for file in files {
        entities.insert((file.entity_id, file.entity_type));
    }

    for (entity_id, entity_type) in entities {
        state.embedding_queue.enqueue(EmbeddingRequest {
            entity_id,
            entity_type,
            requested_at: Instant::now(),
        });
    }

    Ok(())
}

fn process_request(state: &AppState, request: &EmbeddingRequest) -> Result<usize, String> {
    if !state.embedding_model.is_ready() {
        return Err("embedding model unavailable".to_string());
    }

    let config = state
        .config
        .read()
        .map_err(|_| "config lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "config unavailable".to_string())?;

    let db = ActionDb::open().map_err(|e| format!("open db failed: {e}"))?;

    let files = db
        .get_entity_files(&request.entity_id)
        .map_err(|e| format!("query entity files failed: {e}"))?;

    let pending: Vec<DbContentFile> = files
        .into_iter()
        .filter(|f| {
            f.embeddings_generated_at
                .as_deref()
                .map(|ts| ts < f.modified_at.as_str())
                .unwrap_or(true)
        })
        .collect();

    let mut updated_count = 0usize;
    for file in pending {
        if embed_file(&db, state, &config, &file).is_ok() {
            updated_count += 1;
        }
    }

    Ok(updated_count)
}

fn embed_file(
    db: &ActionDb,
    state: &AppState,
    config: &crate::types::Config,
    file: &DbContentFile,
) -> Result<(), String> {
    let path = std::path::Path::new(&file.absolute_path);
    if !path.exists() {
        return Ok(());
    }

    let text = crate::processor::extract::extract_text(path)
        .map_err(|e| format!("extract failed for {}: {}", file.filename, e))?;
    if text.trim().is_empty() {
        db.set_embeddings_generated_at(&file.id, Some(&Utc::now().to_rfc3339()))
            .map_err(|e| format!("watermark update failed: {e}"))?;
        return Ok(());
    }

    let chunks = chunk_text_by_token_estimate(
        &text,
        config.embeddings.chunk_tokens,
        config.embeddings.chunk_overlap_tokens,
    );
    if chunks.is_empty() {
        db.set_embeddings_generated_at(&file.id, Some(&Utc::now().to_rfc3339()))
            .map_err(|e| format!("watermark update failed: {e}"))?;
        return Ok(());
    }

    // nomic-embed-text-v1.5 asymmetric retrieval: documents get "search_document: " prefix,
    // queries get "search_query: " prefix at search time (ADR-0074, I265).
    let prefixed_chunks: Vec<String> = chunks
        .iter()
        .map(|c| format!("{}{}", crate::embeddings::DOCUMENT_PREFIX, c))
        .collect();
    let embeddings = state
        .embedding_model
        .embed_batch(&prefixed_chunks)
        .map_err(|e| format!("embedding batch failed: {e}"))?;

    let now = Utc::now().to_rfc3339();
    let rows: Vec<DbContentEmbedding> = chunks
        .iter()
        .enumerate()
        .map(|(idx, chunk_text)| DbContentEmbedding {
            id: crate::util::slugify(&format!("{}-chunk-{}", file.id, idx)),
            content_file_id: file.id.clone(),
            chunk_index: idx as i32,
            chunk_text: chunk_text.clone(),
            embedding: crate::embeddings::f32_vec_to_blob(&embeddings[idx]),
            created_at: now.clone(),
        })
        .collect();

    db.replace_content_embeddings_for_file(&file.id, &rows)
        .map_err(|e| format!("replace content embeddings failed: {e}"))?;
    db.set_embeddings_generated_at(&file.id, Some(&now))
        .map_err(|e| format!("watermark update failed: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_with_overlap() {
        let text = (0..30)
            .map(|i| format!("t{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let chunks = chunk_text_by_token_estimate(&text, 10, 2);
        assert!(chunks.len() >= 3);

        let first = chunks.first().unwrap();
        let second = chunks.get(1).unwrap();
        assert!(first.contains("t8"));
        assert!(second.contains("t8"));
    }

    #[test]
    fn test_queue_dedup_by_entity() {
        let queue = EmbeddingQueue::new();
        queue.enqueue(EmbeddingRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            requested_at: Instant::now(),
        });
        queue.enqueue(EmbeddingRequest {
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            requested_at: Instant::now(),
        });

        assert_eq!(queue.len(), 1);
    }
}
