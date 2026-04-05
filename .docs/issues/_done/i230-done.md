# I230 — Claude Cowork Integration — Project/Task Sync

**Status:** Open (Integrations Queue)
**Priority:** P2
**Version:** Unscheduled (API TBD)
**Area:** Integrations

## Summary

Claude Cowork (Anthropic's project/task collaboration product) is a natural companion to DailyOS — Cowork tracks what needs to be done, DailyOS tracks the relationships and meetings that generate that work. A sync between the two would let tasks created in Cowork appear as actions in DailyOS, and let DailyOS actions be surfaced in the Cowork project context. This avoids the user maintaining two separate task lists.

There is already a Claude Code plugin infrastructure in DailyOS (the Cowork plugin shipped in Sprint 28). This issue covers the data sync layer — bidirectional task/action sync, not just prompt integration.

## Acceptance Criteria

Not yet specified. At minimum: connection to Claude Cowork API, pull of tasks from linked Cowork projects to DailyOS actions, and push of DailyOS actions back to Cowork on completion.

## Dependencies

- Requires Claude Cowork API (TBD — API details not finalized).
- The Cowork plugin (`cowork-plugin/`) provides the prompt integration layer; this issue covers the data sync layer.
- Not version-locked.

## Notes / Rationale

The Claude Code plugin (I274, I275) provided the workspace-level integration. This issue is about bidirectional data sync — a more complete integration that requires a stable Cowork API. Blocked on API availability.
