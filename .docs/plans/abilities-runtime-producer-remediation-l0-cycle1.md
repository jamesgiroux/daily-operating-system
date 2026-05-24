# Abilities Runtime Producer Remediation L0 — Cycle 1

Date: 2026-05-23
Plan: `.docs/plans/abilities-runtime-producer-remediation-waves.html`
Scope tier: Wave
Origination: Debug-driven extension
Threat topology: Local-to-local single-user
PR: https://github.com/jamesgiroux/daily-operating-system/pull/366

## Verdict

Cycle 1 blocked.

The reviewers agreed with the direction of a generic producer remediation plan, but blocked implementation until W2 and the cross-wave contracts were concrete enough to prevent another account-shaped MCP patch.

## Review Lanes

- Adversarial challenge
- Architecture/domain review
- Security/trust review
- MCP/DX review

## Blockers Found

1. W2 did not concretely prove the Claude Desktop comparison outcome. The plan needed a source-safe fixture prompt, expected tool call, required output sections, comparison rubric, and failure examples.
2. The MCP projection contract was still too account-shaped. The plan needed either a generic MCP tool or a shared generic projection layer with account/project/person/meeting contract coverage.
3. Participation and stakeholder evidence were not frozen as a runtime contract. The plan needed a generic DTO, count/recency semantics, source refs, and caveats for incomplete normalization.
4. Trust and caveat semantics landed too late. W2 could not render richer derived output before section trust, freshness, and caveat rules existed.
5. Render-policy enforcement was underspecified for MCP. Every text, identity, source label, and provenance ref needed an explicit render path or allowlist.
6. Subject resolution was underspecified. Exact id, slug, name, ambiguous, missing, inactive, and blocked-subject cases needed host-visible result shapes.
7. Production-data handling was underspecified. The plan needed synthetic fixtures only, approved local production runs only, sanitized diagnostics, and no host-model output persistence outside the claim pipeline.
8. Source-label backfill safety was too thin. The cleanup needed dry-run, allowlist, mutation gate, chunked transactions, audit/failure records, and rollback/remediation behavior.

## Remediation Applied

- Added plan metadata for origination class, scope tier, threat topology, and failure trace.
- Added an L0 hardening section that freezes the runtime relationship section, neighborhood snapshot seam, participation DTO, section trust/caveats, MCP render policy, and production-data protocol.
- Added a producer matrix that classifies each evidence class as durable claim writer, read-model snapshot, or mixed producer, with owner, source, projection, and trust rules.
- Tightened W1 around a bounded services-owned `EntityNeighborhoodSnapshot` and generic participation evidence.
- Tightened W2 into a source-safe Claude Desktop eval contract with prompt, subject fixture, tool metadata requirements, named tests, negative render fixtures, comparison rubric, failure examples, and canonical MCP response fields.
- Added subject resolution behavior for id, slug, name, ambiguous, no-match, inactive, hidden, and render-policy-blocked cases.
- Moved minimum trust/caveat and render-policy gates before W2.
- Reframed W4 as trust/backfill audit and refinement, not the first trust gate.
- Added source-label cleanup safety requirements for encrypted production rows.

## Next Gate

Run L0 cycle 2 against the updated plan. Implementation cannot start until the plan gets unanimous L0 approval.

## Cycle 2 Result

Cycle 2 returned `REQUEST_CHANGES` from the active L0 lanes and no `BLOCK` findings.

The reviewers agreed that the plan is no longer account-only, does not bypass the abilities runtime, and does not reinvent documented substrate. Remaining gaps were contract-hardening issues:

1. Subject-resolution outcomes needed canonical response fields for clarification, not-found, unavailable, and blocked cases.
2. MCP tool metadata needed ADR-0128 host-selection guidance, including when not to call DailyOS.
3. The response contract needed top-level MCP metadata: schema version, status, tool name, invocation/provenance handle, truncation, and section states.
4. The comparison rubric needed pass/fail thresholds rather than a content checklist.
5. Trust recomputation sequencing needed per-producer triggers, invalidation behavior, failure behavior, and proof tests.
6. `EntityNeighborhoodSnapshot` needed to be explicitly defined as a projection over existing substrate, not new canonical relationship storage.
7. Participation rules needed duplicate-attendee and internal-participant handling so the generic shape does not hide account-specific stakeholder logic.
8. MCP prompt-injection fixtures needed explicit adversarial external text coverage.
9. Dynamic read-model text needed a renderable evidence wrapper; static allowlists must be compile-time literals only.
10. Source-label backfill needed explicit mutation authorization and typed-source-ref derivation rules.

## Cycle 2 Remediation Applied

- Expanded K-in to include ADR-0093, ADR-0108, ADR-0110, and ADR-0111.
- Added prompt-injection gates and W2/Suite S fixtures for hostile external text.
- Narrowed MCP static allowlists to compile-time literals and required a renderable evidence wrapper for dynamic non-claim text.
- Stated that `EntityNeighborhoodSnapshot` is a read-model projection over existing substrate tables and edges.
- Added duplicate-attendee collapse and internal-participant inclusion rules.
- Added a trust sequencing table with recompute trigger, invalidation/failure behavior, and proof tests for durable/mixed producers.
- Added host-selection positive and negative fixtures for the DailyOS-vs-broad-corpus comparison.
- Added canonical response metadata and resolver variants for MCP output.
- Converted the comparison rubric into pass/fail criteria.
- Tightened source-label cleanup with known-placeholder allowlist, explicit user-approved mutation mode, typed-source-ref derivation, pre/post counts, and partial-failure remediation.

## Current Gate

Run L0 cycle 3 against the remediated plan before implementation starts, unless the team decides these cycle-2 request changes are sufficient for a plan-only PR.
