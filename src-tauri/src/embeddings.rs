use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

pub const DEFAULT_DIMENSION: usize = 768;

/// Query prefix for nomic-embed-text-v1.5 asymmetric retrieval.
pub const QUERY_PREFIX: &str = "search_query: ";

/// Document prefix for nomic-embed-text-v1.5 asymmetric retrieval.
pub const DOCUMENT_PREFIX: &str = "search_document: ";

// ---------------------------------------------------------------------------
// Model state
// ---------------------------------------------------------------------------

enum EmbeddingModelInner {
    /// Real model loaded via fastembed (nomic-embed-text-v1.5).
    Fastembed {
        model: fastembed::TextEmbedding,
        dimension: usize,
    },
    /// Fallback: deterministic hash-based embeddings (dev/test/offline).
    HashFallback { dimension: usize },
    /// Model unavailable — all embed calls return Err.
    Unavailable { reason: String },
}

impl std::fmt::Debug for EmbeddingModelInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fastembed { dimension, .. } => write!(f, "Fastembed(dim={})", dimension),
            Self::HashFallback { dimension } => write!(f, "HashFallback(dim={})", dimension),
            Self::Unavailable { reason } => write!(f, "Unavailable({})", reason),
        }
    }
}

#[derive(Debug, Clone)]
pub enum EmbeddingModelStatus {
    Ready { dimension: usize },
    Unavailable { reason: String },
}

#[derive(Debug)]
pub struct EmbeddingModel {
    state: Mutex<EmbeddingModelInner>,
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingModel {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(EmbeddingModelInner::Unavailable {
                reason: "embedding model not initialized".to_string(),
            }),
        }
    }

    /// Initialize the embedding model via fastembed.
    ///
    /// Downloads the quantized nomic-embed-text-v1.5 model on first run and
    /// caches it in `cache_dir` (typically `~/.dailyos/models/`). Subsequent
    /// runs load from cache without network access.
    ///
    /// If initialization fails (no network on first run, ONNX runtime issue),
    /// the hash-based fallback is activated so the app remains functional with
    /// lower-quality embeddings.
    pub fn initialize(&self, cache_dir: PathBuf) -> Result<(), String> {
        use fastembed::{EmbeddingModel as FE, TextEmbedding};

        let options = fastembed::InitOptions::new(FE::NomicEmbedTextV15Q)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(false);

        match TextEmbedding::try_new(options) {
            Ok(model) => {
                log::info!("Embedding model loaded: nomic-embed-text-v1.5 (quantized)");
                let mut guard = self
                    .state
                    .lock()
                    .map_err(|_| "embedding model lock poisoned".to_string())?;
                *guard = EmbeddingModelInner::Fastembed {
                    model,
                    dimension: DEFAULT_DIMENSION,
                };
                Ok(())
            }
            Err(e) => {
                let reason = format!("fastembed init failed: {e}");
                log::warn!(
                    "Embedding model unavailable, using hash fallback: {}",
                    reason
                );
                let mut guard = self
                    .state
                    .lock()
                    .map_err(|_| "embedding model lock poisoned".to_string())?;
                *guard = EmbeddingModelInner::HashFallback {
                    dimension: DEFAULT_DIMENSION,
                };
                // Return Ok — hash fallback is a valid state, not a fatal error.
                Ok(())
            }
        }
    }

    pub fn set_unavailable(&self, reason: String) {
        match self.state.lock() {
            Ok(mut guard) => {
                *guard = EmbeddingModelInner::Unavailable { reason };
            }
            Err(_) => {
                log::error!("Failed to update embedding model status: lock poisoned");
            }
        }
    }

    pub fn status(&self) -> EmbeddingModelStatus {
        self.state
            .lock()
            .map(|s| match &*s {
                EmbeddingModelInner::Fastembed { dimension, .. }
                | EmbeddingModelInner::HashFallback { dimension } => {
                    EmbeddingModelStatus::Ready {
                        dimension: *dimension,
                    }
                }
                EmbeddingModelInner::Unavailable { reason } => {
                    EmbeddingModelStatus::Unavailable {
                        reason: reason.clone(),
                    }
                }
            })
            .unwrap_or(EmbeddingModelStatus::Unavailable {
                reason: "embedding model lock poisoned".to_string(),
            })
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status(), EmbeddingModelStatus::Ready { .. })
    }

    /// Returns true when the model is running real inference (not hash fallback).
    pub fn is_onnx(&self) -> bool {
        self.state
            .lock()
            .map(|s| matches!(&*s, EmbeddingModelInner::Fastembed { .. }))
            .unwrap_or(false)
    }

    /// Embed a single text. The caller is responsible for adding the appropriate
    /// prefix (`QUERY_PREFIX` or `DOCUMENT_PREFIX`) before calling this method.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "embedding model lock poisoned".to_string())?;

        match &mut *guard {
            EmbeddingModelInner::Fastembed {
                model, dimension, ..
            } => {
                let results = model
                    .embed(vec![text], None)
                    .map_err(|e| format!("fastembed embed failed: {e}"))?;
                let mut vec = results
                    .into_iter()
                    .next()
                    .ok_or_else(|| "fastembed returned empty results".to_string())?;
                vec.truncate(*dimension);
                Ok(vec)
            }
            EmbeddingModelInner::HashFallback { dimension } => Ok(hash_embed(text, *dimension)),
            EmbeddingModelInner::Unavailable { reason } => Err(reason.clone()),
        }
    }

    /// Embed a batch of texts. Each text should already have the appropriate prefix.
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "embedding model lock poisoned".to_string())?;

        match &mut *guard {
            EmbeddingModelInner::Fastembed {
                model, dimension, ..
            } => {
                let results = model
                    .embed(texts.to_vec(), None)
                    .map_err(|e| format!("fastembed batch embed failed: {e}"))?;
                Ok(results
                    .into_iter()
                    .map(|mut v| {
                        v.truncate(*dimension);
                        v
                    })
                    .collect())
            }
            EmbeddingModelInner::HashFallback { dimension } => {
                Ok(texts.iter().map(|t| hash_embed(t, *dimension)).collect())
            }
            EmbeddingModelInner::Unavailable { reason } => Err(reason.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Hash-based fallback (deterministic, for dev/test/offline)
// ---------------------------------------------------------------------------

fn hash_embed(text: &str, dimension: usize) -> Vec<f32> {
    let mut vec = vec![0.0_f32; dimension];
    let mut seen = 0usize;

    for token in text.split_whitespace() {
        let token = token.trim().to_lowercase();
        if token.is_empty() {
            continue;
        }
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        let hash = hasher.finish();
        let idx = (hash as usize) % dimension;
        let sign = if (hash & 1) == 0 { 1.0 } else { -1.0 };
        vec[idx] += sign;
        seen += 1;
    }

    if seen == 0 {
        return vec;
    }

    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vec {
            *value /= norm;
        }
    }

    vec
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for (va, vb) in a.iter().zip(b.iter()) {
        dot += va * vb;
        norm_a += va * va;
        norm_b += vb * vb;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

pub fn f32_vec_to_blob(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

pub fn blob_to_f32_vec(blob: &[u8]) -> Result<Vec<f32>, String> {
    if blob.len() % 4 != 0 {
        return Err("invalid embedding blob length".to_string());
    }

    let mut values = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f32_blob_roundtrip() {
        let original = vec![0.1_f32, -0.5_f32, 1.25_f32, 0.0_f32];
        let blob = f32_vec_to_blob(&original);
        let restored = blob_to_f32_vec(&blob).expect("valid blob");
        assert_eq!(original.len(), restored.len());
        for (a, b) in original.iter().zip(restored.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn test_cosine_similarity_ranking() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.9, 0.1, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b) > cosine_similarity(&a, &c));
    }

    #[test]
    fn test_hash_embed_deterministic() {
        let a = hash_embed("hello world", 768);
        let b = hash_embed("hello world", 768);
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_embed_normalized() {
        let v = hash_embed("some test text here", 768);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "expected unit norm, got {}", norm);
    }

    #[test]
    fn test_hash_embed_dimension() {
        let v = hash_embed("test embedding", DEFAULT_DIMENSION);
        assert_eq!(v.len(), DEFAULT_DIMENSION);
    }
}
