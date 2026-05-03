-- Migration 044: Add embedding column to user_context_entries for semantic retrieval
--
-- Stores the embedding vector directly on the context entry row,
-- avoiding the content_embeddings FK constraint to content_index.
-- Uses the same nomic-embed-text-v1.5 model (768 dimensions, f32 BLOB).

ALTER TABLE user_context_entries ADD COLUMN embedding BLOB;
