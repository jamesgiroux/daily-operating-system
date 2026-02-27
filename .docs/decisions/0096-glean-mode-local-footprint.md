# ADR-0096: Glean Mode Local Footprint

**Date:** 2026-02-27
**Status:** Accepted
**Extends:** ADR-0095 (Dual-Mode Context Architecture)

## Context

When DailyOS operates in Glean mode (ADR-0095), Glean replaces several local data sources. This ADR documents what's stored locally in Glean mode, what's no longer stored, and how workspace file behavior changes.

## Decision

### Still Local (Both Glean Strategies)

| Data | Table/Path | Why |
|------|-----------|-----|
| Meeting history | `meetings_history` | Calendar is always local |
| Captures | `captures` | Post-meeting capture is always local |
| Actions | `actions` | User-created, always local |
| Signal events | `signal_events` | DailyOS's own signal bus |
| Entity intelligence cache | `entity_intelligence` | DailyOS synthesis output |
| intelligence.json on disk | `_workspace/entity/intelligence.json` | DailyOS synthesis artifact |
| People table | `people` | Populated from Glean instead of Clay |
| Meeting prep | `prep_frozen_json` | Synthesis output, always local |
| User entity | `user_entity` | User's own professional context |
| Glean document cache | `glean_document_cache` | Cached Glean responses (TTL-based) |
| Context mode config | `context_mode_config` | Which mode is active |
| Transcripts | Filesystem (Granola/Quill) | Always local, always active |

### No Longer Stored (Glean Mode)

| Data | Previous Source | Why |
|------|---------------|-----|
| Raw email bodies | Gmail API | Gmail poller disabled (Governed) or emails sourced from Glean |
| Clay enrichment metadata | Clay MCP | Clay disabled in both strategies |
| Gravatar profiles | Gravatar API | Gravatar disabled in both strategies |

### New Storage (Glean Mode)

| Data | Location | Details |
|------|----------|---------|
| `glean_document_cache` | SQLite table | TTL-based: docs 1h, profiles 24h, org graph 4h |
| `context_mode_config` | SQLite table | Single-row config: mode JSON or NULL (local) |

### intelligence.json Retained

`intelligence.json` is DailyOS's synthesis artifact — not Glean's. It:
- Survives Glean outages (cached on disk)
- Serves meeting prep (MeetingPrepQueue reads it)
- Stores user edits (field-level corrections persist across re-enrichment)
- Contains DailyOS-specific structure (executive_assessment, stakeholder_reconciliation, etc.)

Glean data is an *input* to the intelligence pipeline. The *output* (`intelligence.json`) is always DailyOS-owned and locally stored.

### Workspace Files in Glean Mode

**Additive:** Workspace files are read normally by `LocalContextProvider` AND Glean documents are appended. File-based content and Glean documents coexist in `IntelligenceContext.file_contents`.

**Governed:** Workspace files in `file_contents` are *replaced* by Glean documents. The workspace directory still exists (for `intelligence.json`, transcripts, etc.), but file-based entity context is not read. This is the enterprise-clean mode where all knowledge comes from Glean.

In both modes, transcript files from Granola/Quill are always read locally (they're meeting recordings, not knowledge documents).

## Consequences

- Glean mode has a smaller local footprint (no email bodies, no Clay/Gravatar data)
- `intelligence.json` persistence means Glean outages don't blank out intelligence
- The `glean_document_cache` table adds ~100KB-1MB of cached data (auto-pruned by TTL)
- Switching modes doesn't migrate or delete data — old data from the previous mode remains in the DB but is not actively used
