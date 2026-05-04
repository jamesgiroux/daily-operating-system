# I417 — Context Entries — Professional Knowledge as Intelligence Input

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Intelligence

## Summary

Context entries are user-authored professional knowledge that gets embedded and retrieved in entity enrichment contexts. They are NOT entity notes — notes document interactions with a specific entity and belong on that entity's page. Context entries document the user's own professional expertise, judgment, and mental models: how they think about a class of problem, what they have learned about a domain, what they believe works in recurring situations. "How I think about migration risk conversations with technical evaluators" belongs as a context entry, not on any specific account, because it applies wherever migration risk and technical evaluation intersect — which is every relevant account, not just one.

The embedding is the access mechanism. When the enrichment pipeline processes an entity, it queries the context entries by semantic similarity. The top-matching entries above a threshold are included as prose in the user context block of the enrichment prompt. The AI reads them as user-supplied framing context, not as instructions.

## Acceptance Criteria

### 1. Table and embedding pipeline

The `user_context_entries` table exists per I411 criterion 2. The embedding pipeline processes new entries without blocking the caller:

When `create_user_context_entry(title, content)` is called, the entry is saved to DB immediately (synchronous) and the content is enqueued for the background embedding processor (task #9) using the collection label `user_context` (asynchronous). The command returns the created entry with `embedding_id = null`. The caller (frontend) does not wait for the embedding.

Verify: call `create_user_context_entry`. Wait for the embedding processor to run its next cycle (up to 2 minutes in dev). Query: `SELECT embedding_id FROM user_context_entries WHERE id = '<id>'` — returns a non-null value. The `content_embeddings` table has a row with the matching ID and `collection = 'user_context'`.

### 2. search_user_context function

A function `search_user_context(query: &str, limit: usize) -> Vec<UserContextEntry>` exists in the backend (in the intelligence module or a dedicated user context module). It:

- Embeds the query string using the same nomic-embed-text model used for all content embeddings
- Queries `content_embeddings` by cosine similarity, filtering to `collection = 'user_context'`
- Returns the top-N entries (by similarity) where the cosine similarity exceeds a threshold (0.5 is a reasonable starting value; make this configurable via a constant)
- Joins results back to `user_context_entries` to return full entry objects (id, title, content, embedding_id, created_at)

This function uses the same cosine similarity infrastructure as `search_entity_content`. It is not a new search mechanism — it is a new call site with a collection filter.

Verify: create two context entries with distinct topics. Call `search_user_context` with a query matching the first entry's topic. The first entry ranks higher than the second in the results. Call it with a query matching neither — results are empty or below threshold.

### 3. Enrichment prompt builder integration

The enrichment prompt builder (from I412) calls `search_user_context` with the entity's name and domain as the query. The returned entries (top-2, above threshold) are included as a sub-section in the user context block of the enrichment prompt. Format:

```
## User Context
[...existing declared context fields: value_proposition, success_definition, current_priorities...]

### Professional Knowledge (retrieved)
The following entries from your professional knowledge base are relevant to this account:

**<entry title>**
<entry content>

**<entry title>**
<entry content>
```

The "Professional Knowledge (retrieved)" sub-section only appears when at least one entry meets the similarity threshold. If no entries exceed the threshold, the sub-section is omitted entirely — no empty header, no placeholder text.

Verify: write a context entry: title "Technical evaluator conversations", content "When facing a technical evaluator who has concerns about migration complexity, I lead with implementation risk segmentation — separating infrastructure migration (our team's responsibility) from adoption and workflow migration (theirs). This reframes the risk conversation away from 'will it work' toward 'how will we work together on this.'" Enrich an account whose meeting history includes the word "migration" and "technical." The assembled enrichment prompt (logged at DEBUG level during enrichment) includes the context entry title and content in the user context block.

### 4. Graceful absence

If `search_user_context` returns no entries above the similarity threshold for an entity, the enrichment prompt's user context block does not include the "Professional Knowledge (retrieved)" sub-section. The block assembles only from the declared context fields (value_proposition, success_definition, current_priorities). No empty headers. No "No relevant context entries found" placeholder.

Verify: create a context entry about a specific niche topic with no relationship to any account in the test workspace. Enrich any account. The enrichment prompt (logged at DEBUG) does NOT contain "Professional Knowledge (retrieved)."

### 5. Context entries are not surfaced on entity pages

Context entries do not appear on account detail pages, person detail pages, project detail pages, or any entity surface. They are visible only on the user entity page (`/me`, I415 § Context Entries). They are consumed by the AI in enrichment prompts — they are never surfaced as visible signals, callouts, or intelligence sections on entity pages.

Verify: navigate to any account detail page — no reference to context entries appears. Navigate to `/me` — context entries are listed in the Context Entries section.

### 6. Embedding failure handling

If the embedding model is unavailable when a context entry is created (model not loaded, nomic service not running), the entry is saved to the DB with `embedding_id = null`. No error is thrown to the caller. The entry is added to a retry queue and embedding is attempted on the embedding processor's next cycle.

An entry with `embedding_id = null` does not prevent `search_user_context` from functioning for other entries. The function simply has no vector to match against for the un-embedded entry — it is excluded from search results until its embedding is ready.

`cargo test` passes. A test for this failure path exists: simulate embedding failure, verify entry is saved with null embedding_id, verify no panic or propagated error.

## Dependencies

- Blocked by I411 (table `user_context_entries` and commands must exist).
- Blocked by I412 (enrichment prompt builder must exist to inject retrieved context entries — criterion 3 requires the prompt builder to call `search_user_context`).
- See ADR-0090 Decision 4 for the full rationale on the context entries concept and its distinction from entity notes.

## Notes / Rationale

The context entries concept exists because there is a class of professional knowledge that is inherently cross-entity. An account note captures what happened at a specific account. A context entry captures how the user thinks about a class of situation. The first depreciates as circumstances change. The second appreciates — it becomes more refined as the user adds to it and revises it.

The embedding-first retrieval model means the quality of the entries determines the quality of retrieval. Vague entries ("I think renewals are important") will not surface in relevant contexts. Specific, scenario-grounded entries ("When a champion goes dark 60 days before renewal, my first move is always a peer introduction from another champion at a similar company — I keep a list of willing references segmented by industry") will retrieve precisely when relevant.

This is different from playbooks (§ My Playbooks) in one key way: playbooks are structured by role and named by situation type. Context entries are unstructured and named by the user. Playbooks are how you approach your standard situations. Context entries are what you know — professional knowledge that doesn't fit a named playbook but matters nonetheless.
