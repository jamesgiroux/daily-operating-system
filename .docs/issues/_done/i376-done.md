# I376 — AI Enrichment Site Audit — Map Every PTY/AI Call Site, Verify ADR-0086 Compliance

**Status:** Open (0.13.2)
**Priority:** P1
**Version:** 0.13.2
**Area:** Code Quality / Architecture

## Summary

ADR-0086 defines the intended architecture: AI enrichment happens at the entity level via the intel_queue; meeting prep is mechanical assembly. But the codebase grew across many sprints and there may be direct PTY/AI calls from command handlers or other paths that bypass the intel_queue — pre-ADR-0086 patterns that were never cleaned up. This issue creates a complete inventory of every AI call site, classifies each one against ADR-0086, and remediates any non-compliant paths.

A specific concrete verification is also required: entity relinks (changing which account is linked to a meeting) must trigger immediate prep re-assembly without an AI call, reflecting the new entity's intelligence within 2 seconds.

## Acceptance Criteria

From the v0.13.2 brief, verified in the running app and codebase:

1. A written inventory exists at `.docs/research/i376-enrichment-audit.md` listing every `PtyManager` / Claude Code process invocation in the backend — file, line, function name, what it produces, where the output goes.
2. Every call site is classified: "follows ADR-0086 (entity-level, routes through intel_queue)" or "pre-ADR-0086 (meeting-level, inline, or orphaned)."
3. Any pre-ADR-0086 call sites found are either (a) removed, (b) redirected to route through intel_queue, or (c) documented as a deliberate ADR-0086 exception with a one-sentence rationale.
4. After remediation, `grep -rn "PtyManager::new\|PtyManager::for_tier" src-tauri/src/` returns a list — every entry has a corresponding entry in the inventory. The inventory and the grep output agree.
5. No command handler in `commands.rs` makes a direct PTY call for entity intelligence outside of `intel_queue`. The intel_queue is the sole path for entity-level AI enrichment.
6. Entity relinks trigger immediate prep re-assembly via `MeetingPrepQueue` — no AI call, no reload required. Verify: relink a meeting from one account to another in the running app. The meeting card on the daily briefing updates within 2 seconds to reflect the new entity's intelligence, without any user-initiated refresh. If the new entity has no `intelligence.json` yet, the card shows a "sparse" quality state and updates again automatically once background enrichment completes.

## Dependencies

- Informs I380 (service extraction) — audit may surface enrichment paths inside command handlers that need to move to services before extraction.
- Should be done before I380.

## Notes / Rationale

The pre-0.13.0 audit found that the meeting intelligence lifecycle was "marking meetings enriched after mechanical row-count with no AI." This audit is the systematic version of that spot-check — a complete picture of every AI call in the system and whether each one is in the right place. ADR-0086 established the architecture; this audit verifies whether reality matches.
