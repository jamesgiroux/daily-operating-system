# Plan 3 — Superseded

**Status:** Superseded by [`v1-lite.md`](./v1-lite.md) on 2026-05-07.

The L6 authority infrastructure described here was over-engineered for the actual threat model (single developer, his Slack/GitHub/Linear/laptop). The lean replacement uses claudebot DM with Slack interactive blocks for L6 approvals and a simple append-only `.claude/state/decisions.jsonl` for audit. See [`v1-lite.md`](./v1-lite.md) §6 (claudebot DM) and §7 (audit + visibility).

The codex acceptance-criteria captured here from Plan 0 L0 cycles 4–6 turned out to be the over-engineered class — they fired because the doc was trying to fully constrain Plan 3's implementation rigor in protocol text, which isn't what protocol text is for. See `feedback_l0_partial_convergence_when_class_recurs` memory.

Previous content in git history.
