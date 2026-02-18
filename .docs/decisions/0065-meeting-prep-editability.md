# ADR-0065: Meeting prep editability

**Date:** 2026-02-12
**Status:** Accepted

## Context

Meeting preps are automatically generated, but users still need to inject their own agenda items and notes—especially for customer meetings where context shifts daily. Previously those edits either lived only on the prep page or required manual markdown tweaks, making it hard to keep prep content synched with the CLI and the rest of the product.

## Decision

- Persist two new fields (`userAgenda` and `userNotes`) inside each prep JSON file so user edits augment rather than overwrite the AI-generated content.
- Add two Tauri commands (`update_meeting_user_agenda` and `update_meeting_user_notes`) that rewrite the prep file atomically, remove empty arrays/strings, and handle guardrails (no edits on filled future meetings, tab-based additions to avoid losing context).
- Surface inline editors on `MeetingDetailPage` (list of agenda items with add/remove controls, textarea for notes) with client-side saving states to give the appearance of a native editable document.
- Keep the server-side enrichment flow idempotent: human edits are treated as additive overrides rather than wholesale replacements, so AI reruns can still append new material without clobbering user prose.

## Consequences

- Prep JSON files now mix machine-generated and user-authored content, so downstream consumers must merge `userAgenda`/`userNotes` with the canonical `proposedAgenda`/`talkingPoints`.
- The UI needs to guard edits (e.g., disable removals of agenda items created by AI) or at least warn users before resetting to the latest enrichment pass.
- Saving is synchronous with the filesystem, so we need to throttle repeated writes from rapid keystrokes and surface failure states gracefully.

## Related issues

- [I189](../BACKLOG.md#i189) Meeting prep editability (ADR-0065)
- [I194](../CHANGELOG.md#i194) User agenda + notes editability shipping summary
