# I426 — Google Drive Connector — Doc Import and Watch

**Status:** Open
**Priority:** P1
**Version:** 0.14.2
**Area:** Backend / Connectors + Frontend / Inbox

## Summary

A Google Drive connector that brings collaborative documents into DailyOS's intelligence context. Two distinct modes: **Import** (one-time pull) and **Watch** (continuous change detection). Both modes convert Google Docs to markdown with entity-routing YAML frontmatter and route through the existing file processor pipeline. Import drops the file into `_inbox/` immediately. Watch tracks the document via the Drive Changes API and keeps the file current.

The design has two UI surfaces: the **Inbox page** is where new imports/watches are initiated (consistent with how files already enter the system); the **Connectors settings page** shows what is currently being monitored and its sync health. These are not the same surface doing the same job.

Scale: the Drive Changes API (`drive.changes.list` with `startPageToken`) returns all changes since the last poll token in one API call, regardless of how many documents are being watched. Polling is O(1) API calls per cycle, not O(N). This makes watching 50 documents as cheap as watching 5.

## Acceptance Criteria

### Authentication

1. A separate "Connect Google Drive" flow in the Google Drive connector card requests the `https://www.googleapis.com/auth/drive.readonly` scope. This is a **separate OAuth token** from the existing Calendar/Gmail token — users with existing Google connections are NOT prompted to re-authenticate. The Drive token is stored separately in the keychain alongside the Calendar token.

2. Disconnecting Google Drive revokes only the Drive token. Calendar and Gmail continue to function. Verify: disconnect Drive in Settings — calendar continues to sync and emails continue to flow.

### Inbox page — initiation UI

3. The Inbox page has a "Google Drive" import button (or equivalent affordance — consistent with the existing editorial design, not a new pattern). Clicking it opens the **Google Picker** (Drive file picker UI) when Drive is connected.

4. After selecting a doc or folder from the Picker, a configuration sheet appears:
   - **Mode**: "Import once" (one-time) or "Watch for changes" (continuous)
   - **Entity link**: Auto-detected from filename or folder context if possible. If unclear, show a picker for account, project, person, or user entity ("My Context"). Entity link is **required** — can't proceed without it.
   - **Confirm** button

5. On confirm, the import/watch is created immediately — the doc is processed and written to the entity's Documents folder (`Accounts/{entity_id}/Documents/` etc.). The file appears in `Inbox` briefly during processing, then moves to its final location. No further UI navigation required.

### YAML frontmatter

6. Every Google Drive document is written with YAML frontmatter:

   ```yaml
   ---
   title: "Agentforce GTM Strategy"
   source: google-drive
   google-doc-id: 1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OhifoEwI
   google-doc-url: https://docs.google.com/document/d/...
   entity-id: agentforce
   entity-type: project
   watch: true
   imported-at: 2026-02-22T12:00:00Z
   last-synced-at: 2026-02-22T12:00:00Z
   conversion-format: markdown
   images-stored: true
   ---
   ```

   `entity-id` and `entity-type` are always present (entity identification is required). `watch: true` for watched docs, omitted for one-time imports. Frontmatter is read-only after creation; the watcher does not re-parse it.

   Verify: import a Google Doc linked to the Agentforce project. File appears at `Projects/agentforce/Documents/{date}-{title}.md` with frontmatter containing all fields.

### Import mode (one-time)

7. In import mode, the file is fetched once from the Drive API, converted to markdown, written to the entity's Documents folder with frontmatter. No further polling occurs.

   Verify: import a Google Doc in one-time mode to an account. File appears at `Accounts/{account_id}/Documents/`. After the watcher detects the file: `SELECT * FROM content_files WHERE source LIKE '%drive%' AND entity_id = '<account_id>'` returns a row. Modify the Google Doc in Drive — DailyOS does NOT re-import it.

### Watch mode (continuous)

8. In watch mode, the document is registered in `drive_watched_sources` table with fields: `id`, `google_id`, `name`, `type`, `google_doc_url`, `entity_id`, `entity_type`, `last_synced_at`, `changes_token`, `created_at`. The `changes_token` stores the Drive Changes API page token at registration time.

9. The Drive poller (`run_drive_poller`) runs every 60 minutes (configurable). **It uses the Changes API:** `drive.changes.list(pageToken=changes_token)` returns all changes since the last token. The poller filters to registered watch sources. One API call per poll cycle, regardless of count.

   Verify: register a watch. Modify the Google Doc in Drive. Wait one poll cycle. File at `{entity_type}s/{entity_id}/Documents/` is updated. `SELECT last_synced_at FROM drive_watched_sources WHERE name = '<doc-name>'` shows recent timestamp.

10. **Content change detection:** When a watched doc is fetched, compare new content against stored content via embedding cosine similarity. If similarity > 0.95 (typos/formatting), update the file silently. If similarity < 0.95 (meaningful change), update the file. **In both cases, the watcher detects the file change and enqueues intel_queue automatically.**

    Verify: fix a typo in the Google Doc — file updates within one poll cycle. Add a new paragraph — file updates and intel_queue is enqueued (confirm via `SELECT * FROM intel_queue` or watch entity intelligence refresh in Settings).

11. **Folder mode:** When a Drive folder is watched, the poller monitors for new Google Docs added to that folder. Only Google Docs are imported (not images, PDFs, Sheets — those require explicit individual import). New docs discovered in a watched folder get the same entity link and frontmatter as directly-watched docs.

### Scale guard

12. A configurable cap (default: 30 watched sources, including folders) prevents unbounded polling growth. If the user tries to add a 31st watch, the UI shows: "You have 30/30 watched sources. Remove one to add another." The **import-once mode is NOT subject to this cap** — it processes and forgets.

### Connectors settings — monitoring view

13. The Google Drive connector card in Settings shows: connection status, total watched sources count, last sync timestamp, and a list of watched sources with name, entity link, last-synced-at, and "Remove watch." This is the **management surface** — you remove watches here. You add watches from the Inbox page.

### Intelligence integration

14. Drive documents are automatically embedded and indexed. The watcher detects the file, intel_queue is enqueued, and the document content is embedded under that entity's collection in `content_embeddings` with `source = 'google_drive'`. Verify: `SELECT * FROM content_files WHERE source LIKE '%drive%' AND entity_id = '<agentforce-id>'` returns a row after sync and embedding completes.

15. **The Agentforce end-to-end test:** Watch the Agentforce GTM Google Doc, linked to the Agentforce project entity. After one poll cycle: the markdown file exists at `Projects/agentforce/Documents/{date}-GTM.md`. After embeddings process: `SELECT * FROM content_embeddings WHERE source = 'google_drive' AND entity_id = 'agentforce'` returns rows. When Agentforce entity intelligence is refreshed: the enrichment prompt (DEBUG log) includes a passage from the GTM doc. A meeting detail page for an Agentforce meeting includes GTM doc context in prep.

## Implementation Details

### File Storage & Entity Identification

**Storage location:** Drive documents are stored directly in the entity's Documents/ folder, not in a separate drive_imports directory:
- `Accounts/{entity_id}/Documents/{date}-{doc-title}.md`
- `Projects/{entity_id}/Documents/{date}-{doc-title}.md`
- `People/{entity_id}/Documents/{date}-{doc-title}.md` (if person entity type is supported)

Entity identification happens at import/update time:
1. **Explicit link:** User selected entity in the Inbox modal → use that entity_id/entity_type
2. **Filename heuristics:** If doc title matches an account/project name → resolve to entity
3. **Google Drive folder context:** If doc is from a watched folder, use the folder's linked entity

**Low-confidence resolution:** Deferred (I426 Phase 2). For v0.14.3, assume entity is provided or heuristics succeed. If both fail, log a warning and skip the import.

### Google Docs Conversion to Markdown

**Library:** `google-drive-api-client` (existing Google API infra) + `pandoc` or `comrak` (Markdown generation). Images/tables/comments:
- **Images:** Download and store locally alongside the markdown as `{doc-id}-images/`, embed as `![alt]({doc-id}-images/filename.png)`. Images provide little value for entity enrichment but context is preserved.
- **Tables:** Convert to Markdown table format (pipes). `comrak` handles this natively.
- **Comments:** Append as footnotes or inline `[comment: ...]` markers. Optional; can be skipped in v0.14.3.
- **Non-native formats:** Google Sheets → CSV export (easier than PDF). Google Slides → PDF export, then OCR optional (likely overkill).
- **Fallback:** If conversion fails, download as PDF, add a note in frontmatter `conversion_format: pdf`, and store for later processing.

**Pattern reuse:** The app already has `build_companion_md()` for non-markdown files in processor/mod.rs (lines 114-124). Drive conversion follows the same pattern.

### YAML Frontmatter Format

Parse using the same pattern as `parser.rs` lines 32-45 (state machine toggle on `---`). Frontmatter format:

```yaml
---
title: "Agentforce GTM Strategy"
source: google-drive
google-doc-id: 1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OhifoEwI
google-doc-url: https://docs.google.com/document/d/...
entity-id: agentforce
entity-type: project
watch: true
imported-at: 2026-02-22T12:00:00Z
last-synced-at: 2026-02-22T12:00:00Z
conversion-format: markdown
images-stored: true
---
```

All fields are optional except `title`, `source`, `google-doc-id`, `google-doc-url`, `imported-at`. The processor's `router.rs` and `hooks.rs` already read entity-id/entity-type from frontmatter (processor/hooks.rs lines 26-27, used for entity_intelligence hook). Drive docs use the same mechanism.

**Creation:** When a doc is imported, generate frontmatter server-side in Rust (no YAML library needed — simple string building). When the processor encounters the file, it parses frontmatter using the existing parser pattern.

### Entity Picker on Inbox Page

**Reuse:** EntityPicker component (src/components/ui/entity-picker.tsx) is already used in meeting-entity-chips and email-entity-chip. For inbox, create an InboxImportModal with:
1. Google Picker (Drive file/folder selector) — Google's native UI, reuses existing OAuth token
2. Mode selector: "Import once" or "Watch for changes"
3. EntityPicker drop-in (lines 39-98 of entity-picker.tsx)
4. Confirm button → invoke `import_google_doc_item` (backend command)

**Modal placement:** "Google Drive" button on InboxPage, next to existing inbox file upload affordances.

### Conflict Resolution (File Versioning)

When the poller detects that the user has edited the local file (stat.mtime > last_synced_at) while the poller also fetched a new version:

1. Rename the incoming version: `filename.md` → `filename-2.md` (append `-{n}`)
2. Write the incoming version to the new name
3. Re-embed both versions
4. Emit a Tauri notification: "Drive update: 'Agentforce GTM' has changes. Saved as '...GTM-2.md' to preserve your edits."

**Mechanism:** Check file mtime before writing in the poller; if mtime > last_synced_at, apply versioning. Stored in `drive_watched_sources.last_synced_at` for comparison.

### Database Schema

New table `drive_watched_sources`:

```sql
CREATE TABLE drive_watched_sources (
  id TEXT PRIMARY KEY,
  google_id TEXT NOT NULL,               -- Google Drive file/folder ID
  name TEXT NOT NULL,                    -- User-facing name (from Drive)
  type TEXT NOT NULL,                    -- 'doc' | 'folder' (enforce via check)
  google_doc_url TEXT NOT NULL,          -- Full Google Drive URL
  entity_id TEXT NOT NULL,               -- Account/project/person ID; required (identified at import)
  entity_type TEXT NOT NULL,             -- 'account' | 'project' | 'person' | 'user_context'
  last_synced_at TEXT NOT NULL,          -- ISO 8601, used to detect user edits
  changes_token TEXT,                    -- Drive Changes API pageToken for this source
  created_at TEXT NOT NULL,              -- ISO 8601

  FOREIGN KEY(entity_id, entity_type) REFERENCES entity_intel(entity_id, entity_type) ON DELETE CASCADE
);
```

**Notes:**
- `entity_id` and `entity_type` are NOT nullable (document must be linked to an entity at import; low-confidence deferred to Phase 2)
- Local file path is implicit: `{entity_type}s/{entity_id}/Documents/{date}-{slugified-title}.md`
- No `workspace_path` column needed; watcher monitors entity directories directly

Migration: `0034-drive-watched-sources.sql` (sequential, post-v0.14.2).

### Drive Changes API Polling

**Setup:** On first successful import, call `drive.changes.list(pageToken='1')` to get the initial token. Store in `changes_token` for this watch.

**Poll cycle (60 minutes, configurable):** Background task `run_drive_poller()` in lib.rs:

```rust
pub async fn run_drive_poller(state: Arc<AppState>) {
  loop {
    tokio::time::sleep(Duration::from_secs(config.drive_poll_interval)).await;

    // Fetch all changes since last token
    let changes = match drive_api.changes_list(&last_page_token).await {
      Ok(c) => c,
      Err(e) => {
        log::warn!("Drive poller: changes_list failed: {}", e);
        continue;  // Skip this cycle, don't advance token
      }
    };

    // Filter to registered watches
    let watched_ids: Vec<&str> = state.db.get_all_watched_google_ids();
    let relevant_changes = changes.iter()
      .filter(|ch| watched_ids.contains(&ch.file_id.as_str()))
      .collect::<Vec<_>>();

    // Process each change
    for change in relevant_changes {
      process_drive_change(&state, &change).await;
    }

    // Advance token for next poll
    if let Some(next_token) = changes.next_page_token {
      state.db.update_drive_poller_token(&next_token).await;
    }
  }
}
```

**Error handling:** If `pageToken` is invalid (user revokes/re-grants), Fall back to `pageToken='1'` and re-download all docs. Log as a "Reset watch due to token expiry."

### Content Change Detection & Automatic Intelligence Enqueue

After fetching a new version of a watched doc:

1. Extract text from new PDF/markdown
2. Embed new content via existing embeddings pipeline
3. Query previous embedding(s) for the same `google_doc_id`
4. Compute cosine similarity between new and previous
5. **If similarity > 0.95:** Mechanical update (typos, formatting). Update file, mark `last_synced_at`. **Watcher detects the file change and enqueues intel_queue automatically.**
6. **If similarity < 0.95:** Meaningful change. Update file, mark `last_synced_at`. **Watcher detects and enqueues intel_queue (no special logic needed).**

**First-time import:** When a doc is imported with an identified entity, it's written directly to `Accounts/{entity_id}/Documents/` or `Projects/{entity_id}/Documents/`. **The file watcher (`watcher.rs` line 141-150) detects the new file in the AccountContent or ProjectContent directories and enqueues intel_queue automatically.** No manual intel_queue enqueue needed.

**No signals emitted:** Drive document import/update does not emit signals. The file is routed to its entity's directory, and the existing watcher/intel_queue mechanism handles enrichment triggering. This aligns with how other entity content files (meeting notes, reports) are handled.

**Implementation:** Add `compare_embeddings()` helper in `processor/embeddings.rs` for similarity comparison. File write operations are the same as any other inbox file (write with frontmatter, let watcher do the rest). Reuse existing `EmbeddingModel` already loaded in AppState.

### Connector Status Indicator (Settings UI)

**GoogleDriveCard** in Settings shows:
- Connection toggle ("Connected" / "Disconnected")
- Last sync timestamp ("Last synced: 2 min ago")
- Document count ("3 documents imported")
- Watched count ("2 active watches")
- "Sync Now" button (manual trigger)
- List of watched sources with:
  - Name
  - Entity link (if any)
  - Last-synced-at
  - Remove button (→ deletes from `drive_watched_sources`)

**Component:** Reuse existing connector card pattern (see IntegrationsSection, SettingsPage.tsx). Add a new ConnectorCard variant for Google Drive.

### Intelligence Integration (Automatic Context Injection)

Drive documents are automatically available as context during entity enrichment. **No special code needed** — the existing `inject_entity_intelligence()` in `intelligence/prompts.rs` already handles this:

**How it works (automatic):**
1. Entity intelligence is triggered to regenerate (via intel_queue enqueue from watcher)
2. PTY enrichment process calls `inject_entity_intelligence(entity_id, entity_type)`
3. Function queries: `SELECT * FROM content_embeddings WHERE entity_id = $1 AND entity_type = $2`
4. This returns embeddings for ALL documents associated with the entity (accounts' Documents/, projects' Documents/, plus Drive docs with `source = 'google_drive'`)
5. Top-K relevant passages are extracted (semantic similarity to entity context)
6. Passages are formatted and injected into the enrichment prompt as context
7. PTY includes document context in the enriched intelligence output
8. intelligence.json is updated

**Drive docs are treated identically to manually-added entity documents** — they land in Documents/, get embedded, and become available for injection. No routing, classification, or special handling needed. The system already knows to use content_embeddings as context sources.

### Watcher & Processor Integration

Drive documents bypass the processor/classifier pipeline entirely. The processor is for `_inbox/` files that need classification, routing, action extraction, and signal emission. Drive docs don't need any of that — they're already routed by entity.

**Flow after file write:**

1. **Import/update:** Doc imported → entity identified → written to `Accounts/{entity_id}/Documents/filename.md`
2. **Watcher detection:** File system watcher (`watcher.rs` line 141-150) detects change in `Accounts/*/Documents/` (non-dashboard files)
3. **Intel queue enqueue:** Watcher directly enqueues the entity in `intel_queue` with `ContentChange` priority
4. **Background embedding:** Background embeddings processor reads the file and generates embeddings, storing in `content_embeddings` with `source = "google_drive"`, `entity_id = {account_id}`, `entity_type = "account"`
5. **PTY enrichment triggered:** intel_queue processor spawns PTY task to regenerate entity intelligence
6. **Enrichment context injection:** During PTY enrichment, `intelligence/prompts.rs::inject_entity_intelligence()` queries:
   ```sql
   SELECT * FROM content_embeddings WHERE entity_id = $1 AND entity_type = $2
   ```
   This retrieves ALL documents for the entity (including Drive docs). Top-K relevant passages are extracted and injected into the enrichment prompt.
7. **intelligence.json updated:** PTY returns enriched intelligence, stored in DB and read by frontend

**Result:** intelligence.json now includes context from Drive documents, without needing explicit signals or processor classification. Drive docs become passive context, automatically available during enrichment.

**No processor modification needed.** Drive documents are indistinguishable from manually-added entity Documents (reports, meeting notes, playbooks) after landing in the workspace. The watcher/embedding/enrichment mechanism handles everything.

## Dependencies

- Requires Google OAuth infrastructure already in place. Drive token uses the same keychain pattern with a new key.
- Benefits from I417 (context entries) having established the `user_context` collection label — Drive docs linked to the user entity use the same collection.
- Build after I424 (Granola) to confirm the new background task poller pattern.
- Existing embeddings pipeline handles similarity comparison; no new ML infrastructure needed.
- EntityPicker component already exists; no new UI components needed beyond GoogleDriveCard.

## Notes / Rationale

**Two UI surfaces, one job each.** The Connectors settings page shows health and lets you remove watches. The Inbox page is where files enter the system — a "Google Drive" button there is natural. Initiating imports from Settings would bury the feature behind a config-first mental model that doesn't fit how the app works.

**Why the Changes API?** Polling each watched file individually is O(N) API calls per cycle — rate-limited at ~30 docs. `drive.changes.list` with a stored `pageToken` returns all changes in one call. The app stores the token after each poll; the next poll starts from it. Scales to hundreds of watched docs at O(1) cost.

**Import vs. Watch is a real distinction.** Historical docs (old PRDs, playbooks) → import once. Live collaborative docs (active GTM doc, running project spec) → watch. Conflating them creates confusion in both directions.

**Automatic file placement.** Drive documents are identified to an entity at import time and written directly to the entity's Documents folder. The existing watcher infrastructure then handles everything: file monitoring, intel_queue enqueue, embedding generation, enrichment context injection. No special routing or processor code needed. This aligns Drive with how manually-added entity documents work today.

**No signals for content infrastructure.** Signals are user-facing events (emails, transcripts, user edits). File additions/updates are infrastructure and should enqueue intel_queue directly (via watcher) without the overhead of signal creation, propagation rules, and Bayesian fusion. Clean separation of concerns.

**The 30-source cap is a design nudge, not a technical limit.** Monitoring 100 docs means using DailyOS as a file sync tool. The cap prompts intentionality and is configurable for users who genuinely need more.

**Related improvement (separate issue):** Transcripts currently emit `transcript_outcomes` signals but nothing listens to them. A propagation rule should be added (as a follow-up) to trigger `transcript_outcomes` → intel_queue enqueue, so raw transcript content (quotes, discussion notes, trends) can inform entity enrichment beyond the AI-extracted wins/risks.
