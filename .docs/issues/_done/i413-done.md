# I413 — User Entity Document Attachment

**Status:** Open
**Priority:** P2
**Version:** 0.15.0
**Area:** Backend / File Processing

## Summary

Users can attach professional documents to the user entity — product decks, playbooks, case studies, internal training notes. Attached documents are ingested via the existing file processor pipeline (processor/) and embedded via nomic-embed-text, making them searchable context for intelligence enrichment. This is the same pipeline already used for inbox files, applied to the `_user/attachments/` directory.

## Acceptance Criteria

1. A `_user/attachments/` directory in the workspace is created and monitored by the file watcher (watcher.rs) or included in the hygiene file scan loop. Files added to this directory trigger the file processor.

2. Files placed in `_user/attachments/` are processed by `processor/mod.rs` using the standard pipeline: classified (document type), summarised (main points), and embedded (vector representation). The classification result includes a label indicating the document is user context ("playbook", "product_deck", "case_study", "methodology", etc.). Verify: place a Markdown document in `_user/attachments/`, wait for processing, confirm `SELECT * FROM content_files WHERE path LIKE '%attachments%'` returns a row with a summary and classification.

3. Processed attachments are indexed in `content_files` and `content_embeddings` tables under a `user_context` collection label (distinct from `inbox_file` or other collection labels). This allows filtering and retrieval logic to treat user documents separately.

4. A `search_user_context(query: &str, db: &SqlitePool, limit: usize) -> Vec<ContentMatch>` function retrieves relevant passages from user documents given a semantic query. Verify: attach a product deck containing the phrase "cost reduction"; search for "cost efficiency"; the relevant passage appears in results with a cosine similarity score > 0.75.

5. The enrichment prompt builder (I412) optionally includes a top-K semantic match from user documents when the entity being enriched has context relevant to user document content. The relevance check compares the entity's context (signals, business domain) to user document embeddings. If the top user document has cosine similarity > threshold (0.70), include a ~150-token excerpt in the enrichment prompt. Verify: entity discussing cost pressure + user documents containing cost-reduction content → a relevant passage from the documents appears in the assembled enrichment prompt.

## Design Decisions

1. **Semantic relevance implementation** — Uses a real-time vector similarity query at enrichment time. When enriching an entity, query `content_embeddings` WHERE `collection = 'user_context'` with the entity's context vector (entity name + recent meeting titles + key terms). Top result above cosine similarity 0.70 threshold gets a ~150-token excerpt included in the enrichment prompt. This is the same embedding query pattern used by `signals/relevance.rs` for signal ranking.

2. **Prompt inclusion trigger** — Fires on every entity intelligence enrichment (intel_queue processor). The user context retrieval is a fast DB query (< 10ms), not a model inference call. Only the top-1 result is included if above threshold. No inclusion for mechanical meeting prep — only PTY-based intelligence enrichment.

---

## Dependencies

- Blocked by I411 (user entity must exist), I412 (enrichment prompts must be wirable).
- Uses existing processor/ and embedding infrastructure (no new embedding model required).

Unblocks future analysis and report generation that benefits from user document context.

## Notes / Rationale

From ADR-0089 Decision 2: attached documents are Layer 2 user context (dynamic, ingested). This issue reuses the processor/ pipeline rather than building a new ingestion path. The `_user/attachments/` directory follows the workspace directory convention. Separate collection labeling (`user_context`) allows future queries to focus on user documents without mixing them with inbox files. The optional inclusion in enrichment prompts (criterion 5) uses semantic relevance rather than keyword matching — a cost-reduction document is relevant to an entity with cost signals even if the exact keyword "cost" doesn't appear in the entity's data.
