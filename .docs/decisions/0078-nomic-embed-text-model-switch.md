# ADR-0078: Switch Embedding Model to nomic-embed-text-v1.5 via fastembed

**Status:** Accepted
**Date:** 2026-02-15
**Deciders:** James, Claude
**Supersedes:** ADR-0074 section "Embedding Model" and "Implementation" only. Storage schema, index granularity, hybrid scoring, query API, and integration sections of ADR-0074 remain binding.

## Context

ADR-0074 selected `snowflake-arctic-embed-s` (384 dims, ~34MB INT8) as the local embedding model for entity content search. During Sprint 26 implementation (I264), we re-evaluated this choice:

1. **Age.** snowflake-arctic-embed-s was last updated ~2 years ago. The embedding model landscape has moved significantly since then.
2. **Benchmark quality.** At the 200MB budget (see below), substantially better models exist — up to +6.83 NDCG@10 on BEIR benchmarks.
3. **Size budget was too conservative.** ADR-0074 assumed ~34MB was the ceiling. In reality, competitors ship Electron apps starting at 150MB. A 200MB model budget is justified — the total app remains smaller than competing products.

## Decision

### Model: nomic-embed-text-v1.5

Replace `snowflake-arctic-embed-s` (384d, 51.98 NDCG@10) with **`nomic-embed-text-v1.5`** (768d, 58.81 NDCG@10).

Key properties:
- **768 dimensions** (vs. 384) — richer representations, still efficient for per-entity scale
- **8192 token context** — handles full documents without truncation (arctic-embed-s was 512 tokens)
- **Matryoshka representation** — can truncate to 256/384/512 dims if needed in future for storage optimization
- **Apache 2.0 license** — GPLv3-compatible, no restrictions
- **~137MB INT8 quantized** — fits comfortably in the 200MB budget
- **BEIR 58.81 NDCG@10** — +6.83 points over arctic-embed-s (51.98), a significant retrieval quality improvement
- **Asymmetric prefixes** — `"search_query: "` for queries, `"search_document: "` for documents (different from arctic-embed-s's longer prefix)

### Runtime: fastembed crate

Replace raw `ort` + `tokenizers` + `ndarray` with **`fastembed = "5"`**.

Rationale:
- **Single dependency** replaces three (`ort`, `tokenizers`, `ndarray`) — dramatically simpler build chain
- **Handles everything** — model download, caching, tokenization, ONNX inference, pooling, L2 normalization
- **Auto-download** — model downloads on first app launch to `~/.dailyos/models/`, cached for subsequent runs. No bundled model file in the app binary.
- **Native nomic support** — `NomicEmbedTextV15Q` is a first-class enum variant in fastembed
- **Graceful fallback** — if download fails (no internet on first run), the app falls back to hash-based deterministic embeddings (BM25-only search still works)

### Fallback: Hash-based embeddings

Three-state model in `EmbeddingModel`:
1. **Fastembed** — real ONNX inference (production quality)
2. **HashFallback** — deterministic hash-based vectors (dev/test/offline, lower quality but functional)
3. **Unavailable** — model failed to initialize, all embed calls return Err

The hash fallback ensures the app is never blocked by model availability. Search degrades to BM25-only (text_weight=1.0 effectively) when vector scores are zero.

## Consequences

### Changed from ADR-0074
- **Binary size:** No bundled model file. App binary shrinks by ~34MB (removed placeholder). Model downloads separately (~137MB) on first run.
- **First-run experience:** First launch requires internet to download the model. Subsequent launches work offline. If offline on first launch, hash fallback activates — search works, just lower quality.
- **Dimension:** 768 → storage doubles to ~3KB per chunk (768 * 4 bytes). 1000 chunks = ~3MB. Still negligible for SQLite.
- **Build complexity:** Simplified — fastembed handles native deps internally vs. manually wiring ort + tokenizers + ndarray.
- **Query prefix:** Changed from `"Represent this sentence for searching relevant passages: "` to `"search_query: "`. Document prefix changed from none to `"search_document: "`.

### Unchanged from ADR-0074
- Storage schema (content_embeddings table)
- Index granularity (~500 tokens, 80-token overlap)
- Hybrid scoring (70/30 vector/BM25)
- Query API signature
- Auto-embed on file change
- Integration with build_intelligence_context()
- Zero ongoing cost / P5 alignment

### Rejected Alternatives (new)
- *Keep snowflake-arctic-embed-s* — 6.83 NDCG@10 points worse. The size increase (34MB → 137MB) is justified by the quality gain.
- *Raw ort + tokenizers* — Built this first, then scrapped. Three crates with version-sensitive interactions (ndarray 0.16 vs 0.17, Session requiring &mut self). fastembed wraps all three in a tested, maintained package.
- *Bundle model in app binary* — Inflates every app update by 137MB. Download-on-first-run is better UX — smaller download, model cached separately from app updates.
- *Jina embeddings v3* — CC BY-NC 4.0 license, not compatible with GPLv3 distribution.
- *GTE-base* — MIT license, but 51.14 NDCG@10 — worse than both arctic-embed-s and nomic.
