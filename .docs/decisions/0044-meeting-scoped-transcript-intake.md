# ADR-0044: Meeting-scoped transcript intake

**Date:** 2026-02-07
**Status:** Accepted

## Context

Post-meeting capture (ADR-0037) and dashboard-as-product (ADR-0007) create a workflow gap. The dashboard surfaces meeting prep (before), a current-meeting indicator (during), and manual outcome capture via `PostMeetingPrompt` and the "Outcomes" button (after). But the highest-value post-meeting artifact — the call transcript — can only enter the system through the generic Inbox page, which:

1. Requires navigating away from the dashboard and the meeting context
2. Runs generic file classification when the meeting context is already known
3. Produces weaker results because the enrichment pipeline lacks the meeting's identity, account, type, and attendees

The `PostMeetingPrompt` fallback variant acknowledges this gap — it says "we'll process the transcript if one arrives" — but offers no affordance to actually attach one. The capture state machine (`capture.rs`) already detects transcript providers (Otter, Fireflies, Fathom, Read.ai), confirming the system expects transcripts but has no intake path from the meeting context.

ADR-0043 established meeting intelligence as core. Transcript intake is the missing link between post-meeting capture and the intelligence loop that feeds future preps (I33).

## Decision

Transcript intake is **meeting-scoped**: users attach transcripts directly from the meeting context on the dashboard, not through the generic Inbox.

### Surfaces

Two UI surfaces offer transcript attachment:

- **PostMeetingPrompt** — a file-picker button alongside Win/Risk/Action. Covers the "transcript ready immediately" case. The prompt auto-dismisses at 60s, so this is a fast-path, not the primary intake.
- **MeetingCard** (past meetings) — an attach button or compact drop zone next to the existing "Outcomes" button. This is the primary intake path, since transcript services typically deliver 5-10 minutes after a meeting ends.

### Processing

- The file receives frontmatter/metadata: meeting ID, title, account, type, date, attendees.
- The file is routed to its account or project location in the workspace (following existing PARA workspace patterns), not to `_inbox/`. The system already knows where it belongs.
- The full enrichment pipeline runs with the pre-attached meeting context: AI extracts summary, actions, wins, risks, and outcomes. This produces richer extraction than the generic inbox path because the meeting's identity and account context are injected into the prompt.
- Index files are updated and cross-links established (meeting → transcript, account → transcript).

### Immutability

- A transcript is a point-in-time record of a conversation. Once processed, it is **not re-processed** on briefing re-runs.
- The link between meeting and transcript is maintained (frontmatter + index), but the transcript content is immutable.
- Briefing analysis may evolve with new inputs; the transcript does not.

### Outcome lifecycle

- When a transcript is processed, extracted outcomes **supersede** manual capture on the meeting card.
- The meeting card transitions from showing the "Outcomes" manual-capture button to displaying the AI-extracted summary, wins, risks, and actions.
- The meeting card becomes a lifecycle view: **prep → current → outcomes**.
- Manual Outcomes capture remains as a fallback when no transcript is available.

### Relationship to generic Inbox

The generic Inbox (`/inbox`) retains its role for **uncontextualized files** — documents where the system doesn't know what they are or where they belong. Meeting-scoped intake is for when the context is already known. Same enrichment pipeline, different entry point, richer output.

## Consequences

- The dashboard becomes the primary surface for the full meeting lifecycle (ADR-0007), not just the pre-meeting and current-meeting phases
- Post-meeting capture (ADR-0037) gains its most valuable input channel without requiring page navigation
- Manual Outcomes capture becomes a lightweight fallback rather than the primary post-meeting interaction
- The enrichment pipeline needs a "meeting-contextualized" mode that accepts pre-attached metadata rather than classifying from scratch
- I31 (generic inbox transcript summarization) remains a separate concern — transcripts dropped in `_inbox/` without meeting context still need classification
- New backlog items: I44 (implementation), I45 (post-transcript outcome interaction UI)
- Risk: scope creep toward general-purpose "attach files to meetings." The boundary is clear: **one meeting, one transcript/notes artifact, targeted extraction.** Not a file manager.
