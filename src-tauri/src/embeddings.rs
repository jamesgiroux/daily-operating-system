use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const DEFAULT_DIMENSION: usize = 384;

#[derive(Debug, Clone)]
pub enum EmbeddingModelStatus {
    Ready {
        model_path: PathBuf,
        dimension: usize,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug)]
pub struct EmbeddingModel {
    state: RwLock<EmbeddingModelStatus>,
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingModel {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(EmbeddingModelStatus::Unavailable {
                reason: "embedding model not initialized".to_string(),
            }),
        }
    }

    /// Initialize the model from a bundled ONNX path.
    ///
    /// Current implementation validates model presence and marks status ready.
    /// Inference uses deterministic local hashing so the app remains fully local
    /// and testable even when ONNX runtime is unavailable at build time.
    pub fn initialize_from_path(&self, model_path: &Path) -> Result<(), String> {
        if !model_path.exists() {
            let reason = format!("embedding model not found: {}", model_path.display());
            self.set_unavailable(reason.clone());
            return Err(reason);
        }

        let size = std::fs::metadata(model_path)
            .map(|m| m.len())
            .unwrap_or_default();
        if size == 0 {
            let reason = format!("embedding model is empty: {}", model_path.display());
            self.set_unavailable(reason.clone());
            return Err(reason);
        }

        if let Ok(mut guard) = self.state.write() {
            *guard = EmbeddingModelStatus::Ready {
                model_path: model_path.to_path_buf(),
                dimension: DEFAULT_DIMENSION,
            };
        }

        Ok(())
    }

    pub fn set_unavailable(&self, reason: String) {
        if let Ok(mut guard) = self.state.write() {
            *guard = EmbeddingModelStatus::Unavailable { reason };
        }
    }

    pub fn status(&self) -> EmbeddingModelStatus {
        self.state
            .read()
            .map(|s| s.clone())
            .unwrap_or(EmbeddingModelStatus::Unavailable {
                reason: "embedding model lock poisoned".to_string(),
            })
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status(), EmbeddingModelStatus::Ready { .. })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        match self.status() {
            EmbeddingModelStatus::Ready { dimension, .. } => Ok(hash_embed(text, dimension)),
            EmbeddingModelStatus::Unavailable { reason } => Err(reason),
        }
    }

    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

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
}
