# ADR-0059: Core entity directory template

**Status:** Accepted
**Date:** 2026-02-09
**Supersedes:** None (new)
**Related:** ADR-0047 (entity dashboard architecture), ADR-0046 (entity-mode architecture), ADR-0048 (three-tier data model)

## Context

Entity directories (accounts and projects) had no standardized structure. The CLI era used a 13-folder numbered template (`00-Index.md`, `01-Customer-Information/`, ..., `12-P2s/`) that was CS-specific and profile-dependent. The app created bare directories with only `dashboard.json` and `dashboard.md`.

This caused three problems:
1. **Transcript routing was wrong** — hardcoded to `01-Customer-Information/` instead of a dedicated transcript directory.
2. **New accounts from the app had no scaffold** — external tools (Claude Desktop, ChatGPT, CLI tools) browsing an entity directory saw no organizational structure.
3. **The numbered folders were role-specific** — `12-P2s` is a VIP CS concept. The template should work across entity modes (account-based, project-based, both).

Per ADR-0048, the filesystem is durable and must be meaningful without the app. Per the project philosophy, the archive is the product — any AI tool consuming the workspace should find well-organized, discoverable content.

## Decision

Every entity directory (account or project) uses a **3-subdirectory template** plus README files:

```
{Entity}/
├── README.md               # What this entity is, directory structure, guidance for AI tools
├── dashboard.json           # Canonical structured data (ADR-0047)
├── dashboard.md             # Generated overview (ADR-0047)
├── intelligence.json        # AI-synthesized intelligence (ADR-0057)
├── Call-Transcripts/        # Meeting call transcripts with YAML frontmatter
│   └── README.md
├── Meeting-Notes/           # Meeting summaries, notes, and outcomes
│   └── README.md
└── Documents/               # General documents (inbox-routed, reports, reference)
    └── README.md
```

### Routing rules

| Content type | Destination |
|-------------|-------------|
| Transcripts (via transcript pipeline) | `Call-Transcripts/` |
| Meeting notes (via inbox classifier) | `Meeting-Notes/` |
| Account updates (via inbox classifier) | `Documents/` |
| Everything else (inbox general) | `Documents/` |
| No account/project detected | `_archive/{date}/` |

### README convention

Each directory includes a `README.md` that:
- Describes what belongs in the directory
- Explains the file format (e.g., YAML frontmatter on transcripts)
- Notes that files are automatically indexed for intelligence enrichment
- Helps external AI tools understand the structure

READMEs are written once on bootstrap and never overwritten (idempotent).

### BU directory detection

App-managed directories (`Call-Transcripts`, `Meeting-Notes`, `Documents`) are excluded from BU child account detection via `MANAGED_ENTITY_DIRS` constant. This prevents the workspace sync from creating phantom child accounts.

### What was dropped

The 13-folder CLI template is retired. These folders are not created by the app:
- `00-Index.md` — replaced by `README.md`
- `01-Customer-Information/` — replaced by `Documents/`
- `02-Meetings/` — replaced by `Meeting-Notes/`
- `03-Call-Transcripts/` — replaced by `Call-Transcripts/` (no numeric prefix)
- `04-Action-Items/` — actions live in SQLite
- `05-Projects/` — projects are a separate entity type
- `06-Integrations/` through `12-P2s/` — role-specific, not core

Users can still create any additional directories they want. The content indexer recursively scans all files in entity directories regardless of subdirectory name.

## Consequences

**Easier:**
- New accounts/projects get immediate structure on creation
- Transcripts route to the right place
- External AI tools can discover and understand the workspace
- Template is entity-mode agnostic (works for accounts and projects)

**Harder:**
- Existing workspaces with the old 13-folder structure will have both old and new directories until manually cleaned up
- Users who relied on specific numbered folders need to adjust

**Migration:** The bootstrap is idempotent — running it on existing directories creates the new subdirectories alongside existing ones without disturbing the old structure. No forced migration. Content indexing picks up files in all subdirectories regardless of naming.
