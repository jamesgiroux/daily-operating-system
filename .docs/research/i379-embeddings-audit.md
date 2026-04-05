# I379: Embeddings Pipeline Audit

**Date**: 2026-02-21
**Status**: Complete

## 1. Infrastructure

### Embedding Model
- **File**: `src-tauri/src/embeddings.rs`
- **Model**: nomic-embed-text-v1.5 (quantized, ~137MB), loaded via fastembed
- **Dimension**: 768 (DEFAULT_DIMENSION)
- **Fallback**: Deterministic hash-based embeddings when ONNX model fails to load
- **Initialization**: Async on app startup (`src-tauri/src/lib.rs:84-95`), cached in `~/.dailyos/models/`
- **Prefixes**: `search_query: ` for queries, `search_document: ` for documents (nomic asymmetric retrieval)

### content_embeddings Table
- **Schema**: `src-tauri/src/migrations/006_content_embeddings.sql`
- **Columns**: id, content_file_id (FK to content_index), chunk_index, chunk_text, embedding (BLOB), created_at
- **Index**: idx_embeddings_file on content_file_id

---

## 2. Write Paths (content_embeddings table)

### W1: Background Embedding Processor
- **File**: `src-tauri/src/processor/embeddings.rs:275`
- **Function**: `embed_file()` → calls `db.replace_content_embeddings_for_file()`
- **Content**: Entity content files (account/project/person tracker files from workspace)
- **Trigger**: Background async loop (`run_embedding_processor`, spawned at `lib.rs:178`)
  - Periodic sweep every N seconds (configurable `sweep_interval_secs`)
  - On-demand via `EmbeddingQueue` enqueue
- **Enqueue sources**:
  - `commands.rs:4897` — after `sync_content_index` command
  - `watcher.rs:376,409` — filesystem watcher detects content changes
  - `state.rs:421` — on startup, enqueues entities with content
  - `processor/embeddings.rs:172` — sweep finds files needing embeddings

### W2: Test-only insert
- **File**: `src-tauri/src/migrations.rs:383` — migration test verifies table accepts inserts
- **File**: `src-tauri/src/db/mod.rs:3017` — unit test for `replace_content_embeddings_for_file`
- Not a production write path.

### W3: Devtools cleanup
- **File**: `src-tauri/src/devtools/mod.rs:301` — DELETE from content_embeddings (cleanup, not a write)

---

## 3. Read Paths (content_embeddings table)

### R1: search_entity_content (hybrid search)
- **File**: `src-tauri/src/queries/search.rs:42`
- **Function**: `search_entity_content()` → calls `db.get_entity_embedding_chunks(entity_id)`
- **Performs**: BM25 text scoring + vector cosine similarity, returns ranked `ContentMatch` results
- **Consumers** (4 live call sites):

#### R1a: chat_search_content command
- **File**: `src-tauri/src/commands.rs:5157`
- **User-facing**: Yes — Tauri IPC command for entity content search in chat UI
- **What user sees**: Ranked content matches for an entity

#### R1b: chat_query_entity command
- **File**: `src-tauri/src/commands.rs:5250`
- **User-facing**: Yes — Tauri IPC command for entity Q&A
- **What user sees**: Semantic search results included in entity query response

#### R1c: MCP search_content tool
- **File**: `src-tauri/src/mcp/main.rs:425`
- **User-facing**: Yes — MCP sidecar tool for Claude Code workspace search
- **What user sees**: Formatted search results with scores

#### R1d: Intelligence prompt builder
- **File**: `src-tauri/src/intelligence/prompts.rs:402`
- **User-facing**: Indirect — feeds ranked files into AI intelligence generation prompts
- **What user sees**: Better-quality entity intelligence (AI gets most relevant files first)

---

## 4. Embedding Model Usage (NOT content_embeddings table)

These paths use the embedding model directly for real-time similarity computation, but do NOT read/write the content_embeddings table:

### M1: Entity resolver — embedding similarity signal
- **File**: `src-tauri/src/prepare/entity_resolver.rs:500-587`
- **Function**: `signal_embedding_similarity()` — compares meeting title embedding against entity name embeddings
- **Purpose**: Meeting-to-entity resolution (Signal 5 of 5)
- **User-facing**: Yes — determines which accounts/projects/people are linked to meetings

### M2: Signal scoring — embedding similarity component
- **File**: `src-tauri/src/signals/scoring.rs:161-176`
- **Function**: `compute_embedding_similarity()` — inline text-to-text cosine similarity
- **Purpose**: Signal priority scoring component (meeting relevance 0.0-0.25)

### M3: Signal relevance ranking
- **File**: `src-tauri/src/signals/relevance.rs:19-54`
- **Function**: `rank_signals_by_relevance()` — ranks signals by similarity to meeting context
- **Purpose**: Prioritizes which signals surface in meeting prep

---

## 5. Cross-Reference: Writes vs Reads

| Write Path | Downstream Consumer(s) | Status |
|-----------|------------------------|--------|
| W1: Background embedding processor | R1a, R1b, R1c, R1d (via search_entity_content) | **LIVE** — all 4 consumers are on active user-facing code paths |

**No orphaned write paths.** The single production write path (W1) feeds all 4 read consumers through the `search_entity_content` function.

---

## 6. Embedding Model Usage Summary

The embedding model serves two distinct purposes:

1. **Content embeddings** (content_embeddings table): Background chunking + embedding of entity files, queried via hybrid search. All write/read paths are wired and live.

2. **Real-time similarity** (no table): Entity resolution, signal scoring, and signal relevance ranking use the model for on-the-fly comparisons. These are independent of the content_embeddings table.

---

## 7. Assessment

**The embedding pipeline is fully wired and serving live features.** No orphaned paths exist. The background processor generates embeddings that are consumed by 4 distinct user-facing features (chat search, entity Q&A, MCP search, intelligence prompts). The embedding model is additionally used for 3 real-time similarity features.

**No remediation needed.** The processor should remain enabled.

### Duplicated cosine_similarity implementations

There are 3 separate `cosine_similarity` functions:
- `src-tauri/src/embeddings.rs:255` — canonical, returns f32
- `src-tauri/src/signals/scoring.rs:179` — duplicate, returns f64
- `src-tauri/src/signals/relevance.rs:57` — duplicate, returns f64

The scoring/relevance versions could call through to `embeddings::cosine_similarity` and cast, but this is a minor cleanup item, not a correctness issue.
