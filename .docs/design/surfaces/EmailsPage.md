# EmailsPage

**Tier:** surface
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `EmailsPage`
**`data-ds-spec`:** `surfaces/EmailsPage.md`
**Source files:**
- `src/pages/EmailsPage.tsx`
- `src/pages/EmailsPage.module.css`
- `src/styles/editorial-briefing.module.css`
**Routes:** `/emails`

## Job

EmailsPage is the email-intelligence surface for triage, commitments, reply signals, account/project updates, and correspondence score bands. It turns the inbox into an editorial briefing rather than a raw message list.

## Layout Regions

1. Folio chrome with refresh action.
2. Editorial hero with narrative headline, priority stats, sync status, and enrichment failure controls.
3. Empty states for disconnected Gmail and connected-but-empty Gmail.
4. Priority correspondence section.
5. Commitments and open questions extracted from entity-linked messages.
6. Gone-quiet and update sections for entity-level changes.
7. Inbox score bands for priority, monitoring, and other correspondence.
8. Finis marker.

## Patterns And Primitives

Consumes the editorial briefing grid, email intelligence rows, triage actions, entity chips, commitment tracking controls, source linkage badges, and empty/loading/error editorial states.

## States

Supports loading, error retry, disconnected empty, connected empty, ready with priority messages, enrichment failure notice with retry/skip/details, tracked commitments, untracked commitments, meeting-linked email, pinned email, archived optimistic state, gone-quiet account, score-banded inbox, and cached-data alert.

