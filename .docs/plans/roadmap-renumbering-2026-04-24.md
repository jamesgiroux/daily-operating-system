# Roadmap Renumbering — Post-v1.2.2

**Date:** 2026-04-24
**Source of truth:** Linear projects for team `DOS`

## Decision

`v1.4.0 — Abilities Runtime` is now the direct successor to
`v1.2.2 — Role-Aware Intelligence`.

The later entity, briefing, MCP, and hardening work all assume the abilities
substrate has shipped. Keeping `v1.2.3`, `v1.2.4`, `v1.3.0`, and `v1.3.1`
ahead of that substrate would imply DailyOS can safely build new user-facing
intelligence features on top of the older flaky loop.

## Mapping

| Old label | New label | Treatment |
| --- | --- | --- |
| `v1.2.2 — Role-Aware Intelligence` | unchanged | Current in-progress release. |
| `v1.4.0 — Abilities Runtime` | unchanged, moved directly after v1.2.2 | Substrate plus Golden Daily Loop release gate. |
| `v1.2.3 — Project Entity` | `v1.4.1 — Entity Intelligence (Accounts, Projects, People)` | Combined with person entity work. |
| `v1.2.4 — Person Entity` | `v1.4.1 — Entity Intelligence (Accounts, Projects, People)` | Combined with project/account entity work. |
| `v1.3.0 — Daily Briefing Redesign` | `v1.4.2 — Briefing Experience (Daily + Meeting)` | Combined with meeting briefing work. |
| `v1.3.1 — Meeting Briefing` | `v1.4.2 — Briefing Experience (Daily + Meeting)` | Combined with daily briefing work. |
| `v1.5.0 — MCP Server v2 (Abilities-First)` | unchanged label, tightened dependency | Comes after the app-facing loop is stable. |
| `v1.6.0 — Hardening (Post-v1.4.0)` | `v1.5.1 — Production Hardening (Post-Abilities)` | Only residual hardening that is not a v1.4.x release gate. |

## Guardrail

Do not park known user-visible correctness failures in `v1.5.1`. Bugs that
affect meeting readiness, entity links, briefing truthfulness, stale claims,
duplicate commitments, correction stickiness, provenance, or account/person/
project detail trust belong in the v1.4.x release gates.

## Update — 2026-04-24 PM — v1.4.x split for Tuesday-validatable spine

Founder direction (2026-04-24, after re-scope discussion): v1.4.0 is too
large to validate in the first-pass push due Tuesday 2026-04-28. The spine
that proves the substrate works ships as v1.4.0; the rest of the substrate
work composes into a new v1.4.1 (Abilities Runtime Completion). The
previously-numbered v1.4.1 (Entity Intelligence) and v1.4.2 (Briefing
Experience) shift one slot.

| Old label | New label | Treatment |
| --- | --- | --- |
| `v1.4.0 — Abilities Runtime` (full scope, 47 issues) | `v1.4.0 — Abilities Runtime Spine` (14 issues) | Cut to spine: ServiceContext, IntelligenceProvider, registry + Provenance with `SubjectAttribution` (ADR-0105 amendment), `intelligence_claims` + tombstone PRE-GATE, Trust Compiler with named factors, typed feedback enum (ADR-0123), Tauri/MCP bridges, eval harness scaffolding, two end-to-end pilots (`get_entity_context` Read + `prepare_meeting` Transform), subject-ownership enforcement (DOS-288), Golden Daily Loop gate against adversarial bundles 1 + 5. |
| n/a | `v1.4.1 — Abilities Runtime Completion` (33 issues) | New project. Substrate completion: remaining capability migrations, signal infrastructure, scoring factor library refactor, full validation suite, full adversarial bundles 2-8, eval harness depth, surface polish. |
| `v1.4.1 — Entity Intelligence (Accounts, Projects, People)` | `v1.4.2 — Entity Intelligence (Accounts, Projects, People)` | Renumbered; scope unchanged. |
| `v1.4.2 — Briefing Experience (Daily + Meeting)` | `v1.4.3 — Briefing Experience (Daily + Meeting)` | Renumbered; scope unchanged. |

## Spec changes that landed alongside the split

- **ADR-0105 amendment (2026-04-24)** — `SubjectAttribution` becomes a typed
  substrate primitive on every `FieldAttribution` and on the `Provenance`
  envelope itself. ProvenanceBuilder enforces structural subject coherence.
  Closes the substrate-level gap behind the addendum's #1 invariant
  (cross-entity content bleed).
- **ADR-0123 (new, 2026-04-24)** — Typed Claim Feedback Semantics. Closes
  the 7 open decisions from the v1.4.0 Claim Feedback and Prompt Granularity
  Review. Defines the closed-form `FeedbackAction` enum (9 variants), the
  `ClaimFeedback` row, and per-action trust deltas. Blocks Trust Compiler
  shipping with feedback-as-undefined.

## Validation gate (replaces wave-completion gates)

The Tuesday gate for v1.4.0 spine is: boot the seeded Golden Daily Loop
mock workspace, run the spine end-to-end against adversarial bundles 1
(same-domain ambiguity) and 5 (correction resurrection), screenshot the
briefing, verify it does not lie. Pass = substrate is real. Fail = substrate
is wrong, fix before fanning out into v1.4.1.
