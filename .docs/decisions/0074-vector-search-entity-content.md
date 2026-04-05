# ADR-0074: Vector Search for Entity Content

**Status:** Accepted
**Date:** 2026-02-14
**Deciders:** James, Claude

## Context

Current entity enrichment retrieves files by priority + recency (`ORDER BY priority DESC, modified_at DESC`), not semantic relevance. When `build_intelligence_context()` assembles context for enrichment, it includes ~50 files' summaries (25K chars) prioritized by manual content-type classification. This approach has two problems:

1. **Misses relevant historical content.** A transcript from 3 months ago mentioning a specific risk won't surface if enough recent files exist to fill the context window.
2. **Includes irrelevant recent noise.** A recently-modified README or boilerplate document gets included over a highly relevant older QBR deck.

OpenClaw's hybrid retrieval pattern (70% semantic + 30% keyword) demonstrates that vector search over entity content dramatically improves enrichment quality for AI-native operational intelligence tools. Their users report that semantic retrieval surfaces "the right context at the right time" — exactly the promise DailyOS makes with entity intelligence.

**Relevant existing decisions:**
- ADR-0048: Three-tier data model (SQLite as working store — embeddings belong here)
- ADR-0057: Entity intelligence architecture (enrichment pipeline this enhances)
- ADR-0058: Proactive intelligence maintenance (gap detection benefits from semantic retrieval)
- ADR-0062: Briefing artifacts vs. live queries (query function pattern for search)

## Decision

### Embedding Model

**Local model via ONNX Runtime** — `snowflake-arctic-embed-s` (384 dimensions, ~34MB INT8 ONNX weight).

Rationale:
- **No new cost center.** DailyOS uses Claude Code (via PTY) for AI enrichment. Adding a separate API billing relationship (OpenAI, Voyage) for embeddings introduces operational complexity and ongoing cost that doesn't fit the product's economics. Local inference has zero marginal cost.
- **P5 alignment (Local-First, Always).** Embeddings generate and query entirely on-device. No network dependency, works offline, data never leaves the machine. This is the strongest possible alignment with the local-first principle.
- **Fast on Apple Silicon.** Local inference is ~1-5ms per chunk on M-series chips. No network latency. Embedding 100 files takes seconds, not minutes.
- **Deterministic.** Same input always produces the same vector. No API variability or version drift.
- **Retrieval-optimized.** Arctic-embed-s scores 51.98 NDCG@10 on MTEB retrieval benchmarks — 10 points higher than general-purpose STS models like all-MiniLM-L6-v2 (41.95) at the same parameter count. Snowflake trained the arctic-embed family specifically for passage retrieval with hard negative mining, which directly maps to our entity content search use case.
- **384 dimensions is sufficient.** Entity content search operates over hundreds of chunks per entity, not millions of documents. At this scale, 384-dim retrieval-optimized vectors outperform 768/1536-dim general-purpose vectors.
- **Asymmetric query encoding.** Queries are prefixed with `"Represent this sentence for searching relevant passages: "` while documents are encoded without prefix. This asymmetry is how retrieval-optimized models achieve better precision than symmetric STS models.

**Implementation:** `ort` crate (ONNX Runtime Rust bindings) with the INT8 ONNX model bundled as a binary asset (~34MB — still less than half a basic Electron app). The model is loaded once at app startup and shared across all embedding operations. Alternatively, `fastembed-rs` wraps `ort` with higher-level tokenization/pooling/normalization and already supports arctic-embed-s out of the box.

**Rejected alternatives:**
- *OpenAI text-embedding-3-small* — Adds a second vendor API dependency (DailyOS is built on Anthropic/Claude). Requires API key management and ongoing cost. Violates P5.
- *Voyage AI embeddings* — Separate billing from Anthropic despite the partnership. Same cost-center concern as OpenAI.
- *Claude via PTY* — Claude is a generative model, not an embedding model. It cannot produce float vectors.
- *all-MiniLM-L6-v2* — General-purpose STS model, not retrieval-optimized. Scores 41.95 NDCG@10 vs. arctic-embed-s's 51.98 — a 10-point deficit on the exact task we need (passage retrieval). Same 384 dims, similar size, strictly worse quality.
- *snowflake-arctic-embed-xs* — 23MB INT8, scores 50.15. Viable fallback if 34MB is too heavy, but the extra 11MB buys 1.8 NDCG@10 points.

### Storage Schema

New `content_embeddings` table with chunk-level vectors, foreign-keyed to `content_index`:

```sql
CREATE TABLE content_embeddings (
    id TEXT PRIMARY KEY,
    content_file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding BLOB NOT NULL,        -- f32 vector, 384 dimensions (1536 bytes)
    created_at TEXT NOT NULL,
    FOREIGN KEY (content_file_id) REFERENCES content_index(id) ON DELETE CASCADE
);

CREATE INDEX idx_embeddings_file ON content_embeddings(content_file_id);
```

Separate table (not a column on `content_index`) because:
- One file produces multiple chunks (1:N relationship)
- Chunk-level granularity enables paragraph-level retrieval
- Cascade delete cleans up embeddings when source files are removed

### Index Granularity

**Per-paragraph chunks (~500 tokens, 80-token overlap).**

- Per-file (coarse) loses intra-document precision — a 20-page QBR has very different sections
- Per-sentence (fine) creates too many chunks and fragments context
- ~500 tokens captures a meaningful paragraph while staying well under model limits
- 80-token overlap prevents relevant content from being split across chunk boundaries

### Embedding Generation

**Auto-embed on file change via background processor**, similar to `EnrichmentProcessor`:

1. `EmbeddingProcessor` watches `content_index` for new/changed files (via `indexed_at` or `modified_at`)
2. Reads file content, chunks into ~500-token segments with 80-token overlap
3. Generates embeddings locally via ONNX Runtime (batched, ~1-5ms per chunk)
4. Upserts to `content_embeddings` table
5. Tracks state via `embeddings_generated_at` column in `content_index`

Re-embedding triggers on file modification (content hash change). Deletion cascades via FK.

No network calls. No rate limits. No API failures to handle. The embedding processor is as reliable as the filesystem watcher.

### Query API

Pure query function following ADR-0062 pattern:

```rust
// src-tauri/src/queries/search.rs
pub fn search_entity_content(
    db: &Database,
    model: &EmbeddingModel,
    entity_id: &str,
    query: &str,
    top_k: usize,
    vector_weight: f32,  // default 0.7
    text_weight: f32,    // default 0.3
) -> Result<Vec<ContentMatch>>
```

Steps:
1. Generate query embedding locally (ONNX Runtime, ~1-5ms)
2. Vector search: cosine similarity over `content_embeddings` scoped to entity
3. Keyword search: BM25 over `content_index` summaries (FTS5 if available, LIKE fallback)
4. Hybrid scoring: `vector_weight * vector_score + text_weight * text_score`
5. Return top_k results sorted by combined score

Exposed as Tauri command: `search_entity_content(entity_id, query, top_k)`.

### Hybrid Scoring

**70% vector similarity + 30% BM25 keyword** as default, configurable per call.

This ratio comes from OpenClaw's production experience and academic research on hybrid retrieval. Vector search captures semantic meaning ("risk" matches "concern", "blocker", "challenge"), while BM25 handles exact terms (account names, product names, acronyms) that embeddings sometimes miss.

### Fallback Strategy

If vector search is unavailable (model failed to load, embeddings not yet generated), fall back to the existing priority + recency retrieval. The system degrades gracefully — enrichment quality decreases but never fails.

### Integration with Entity Enrichment

`build_intelligence_context()` in `entity_intel.rs` gains a semantic search path:

1. Analyze what intelligence is thin (e.g., `risks.len() == 0`)
2. Generate targeted search queries: "risks concerns blockers challenges"
3. Call `search_entity_content(entity_id, query, top_k=20)`
4. Include top results in enrichment context (within 25K char budget)
5. Preserve priority+recency fallback when semantic search unavailable

## Consequences

### Easier
- **Better enrichment quality** — semantically relevant content replaces recency-biased retrieval
- **Historical content surfaces** — 3-month-old transcript mentioning a risk now appears in enrichment context
- **Gap filling** — when intelligence has thin areas (no risks, no wins), targeted search finds relevant content
- **Foundation for chat** — semantic search enables ADR-0075 conversational queries over entity content
- **Zero ongoing cost** — no API billing, no usage metering, no cost surprises
- **Offline-capable** — embeddings work without network connectivity
- **No API key management** — one fewer credential for users to configure

### Harder
- **Binary size** — ~34MB INT8 model weight added to the app bundle (acceptable; still less than half a basic Electron app, and the current DMG is already ~30MB for the Tauri binary)
- **ONNX Runtime dependency** — `ort` crate adds a native dependency to the build chain
- **Model updates** — upgrading the embedding model requires an app update (not a config change). Mitigated by the auto-updater (ADR-0072).
- **Storage** — 384-dim float32 vectors at 1.5KB per chunk; 1000 chunks = ~1.5MB (negligible for SQLite)

### Trade-offs
- Chose local model over API: zero cost and P5 alignment, but adds binary weight and limits model upgrades to app releases
- Chose arctic-embed-s (384 dims, 34MB INT8) over larger models: best-in-class retrieval quality at this size, 10 points above general-purpose STS models, sufficient for per-entity scale
- Chose per-paragraph over per-file: better precision, but more storage and embedding computation
- Chose auto-embed over on-demand: always-fresh indexes, but background processing load
- Chose 70/30 hybrid over pure vector: handles exact terms better, but adds FTS5 dependency
