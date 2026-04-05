# I379 — Vector DB Audit — Map Embedding Writes vs. Queries, Disable Orphaned Paths

**Status:** Open (0.13.2)
**Priority:** P2
**Version:** 0.13.2
**Area:** Code Quality / Architecture

## Summary

The vector embedding pipeline (ADR-0074/0078, Sprint 26) generates embeddings via a background processor using nomic-embed-text-v1.5 (~137MB INT8 model). Embeddings are written to `content_embeddings` and queries use hybrid scoring (70% vector similarity + 30% BM25). However, it's unknown whether all embedding write paths have downstream consumers, or whether some embeddings are generated but never queried. This issue maps every write and every read of `content_embeddings` and disables any generation paths that don't serve a live consumer.

## Acceptance Criteria

From the v0.13.2 brief, verified in the codebase:

1. A written inventory exists at `.docs/research/i379-embeddings-audit.md` listing: every write to `content_embeddings` (what is embedded, what trigger, what collection/label), and every query that reads from `content_embeddings` (function name, what the results feed into, whether it's on a live user-facing path or dead code).
2. Every embedding write path is mapped to at least one downstream consumer. Embedding paths with no downstream consumer are identified as orphaned.
3. Orphaned embedding paths are either (a) wired to a consumer that meaningfully uses them, or (b) the background generation for that path is disabled. No embedding generation runs in the background unless it serves a live query.
4. The embedding processor (background task #9, nomic-embed-text-v1.5, ~137MB) is confirmed useful. If all embedding paths are orphaned after step 3, the processor is disabled until a consumer is built and this is documented.
5. If any semantic search is live (actively queried in a user-facing path): test it with at least 3 real queries using real indexed content and confirm results are semantically relevant — not just that the query function executes without error.

## Dependencies

- No code dependencies; may result in disabling background task #9.
- Independent of other v0.13.2 issues.

## Notes / Rationale

The vector embedding processor is the most expensive background task in the system — it downloads a 137MB model and runs continuously. If embeddings are being generated but never queried for anything the user actually sees, this is pure wasted compute. The audit may confirm the embeddings are live and valuable (the semantic search in `build_intelligence_context()` uses them), or it may find orphaned paths from earlier development iterations.
