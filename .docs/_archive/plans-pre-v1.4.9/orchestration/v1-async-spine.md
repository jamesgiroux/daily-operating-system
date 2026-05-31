# Orchestration v1 — Superseded

**Status:** Superseded by [`v1-lite.md`](./v1-lite.md) on 2026-05-07.

This document went through several rewrites that drifted into over-engineering. Lessons preserved in memory:

- `feedback_slack_signing_authenticates_transport_not_principal` — Slack signed payloads authenticate transport, not which human clicked
- `feedback_protocol_amendments_belong_in_protocol_doc` — wave-protocol changes belong in amendments to `v1.4.0-waves.md`, not consumer plans
- `feedback_l0_partial_convergence_when_class_recurs` — when a reviewer's same-finding-class fires across cycles about a downstream plan's implementation, accept partial convergence and transfer residuals to that plan
- `feedback_dont_treat_examples_as_directives` — illustrative examples in user briefs are not literal feature lists; the L0 panel is for converting vague intent into reasonably defined plans, not a defense gate against attackers

The current architecture lives in [`v1-lite.md`](./v1-lite.md). Previous drafts of this file are in git history if the lessons need to be revisited.
