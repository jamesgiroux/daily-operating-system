---
title: Claim producers require runtime-wide trust recomputation audit
problem_type: architecture_pattern
track: knowledge
module: abilities-runtime claim substrate
tags: [abilities-runtime, claims, producers, trust, provenance, substrate]
date: 2026-05-22
related_adr: ADR-0102
---

## Context

When a new surface starts reading claim-backed intelligence, the first missing producer can make the gap look claim-specific. That is misleading. A producer landing one claim type means the substrate is now ingesting live facts from services, so trust recomputation, provenance inputs, freshness decay, corroboration, contradiction handling, and surface trust-band rendering become runtime-wide contracts.

In the account fact producer work, the immediate gap was that structured account fields existed in schema tables and source-reference tables but did not exist as claims. The narrow fix was to promote sourced account facts into the claim substrate. The broader finding is that newly real producers must be audited against the whole abilities runtime trust model, not just against the first affected claim type.

## Guidance

Before shipping or reviewing a new claim producer, inventory the full runtime contract:

1. Claim type metadata: freshness class, commit policy, allowed subjects, allowed actors, and surface placement.
2. Producer path: where the service commits claims, what source/provenance fields it passes, and whether updates supersede prior active claims.
3. Trust inputs: `source_asof`, `observed_at`, `data_source`, source reliability, corroborations, contradictions, lifecycle state, corrections, and sensitivity.
4. Recompute trigger: whether trust is scored at commit time, by a worker, by startup/backfill, by invalidation, or not at all.
5. Surface behavior: how unscored claims render, whether missing trust is visible as `needs_verification`, and whether prose producers hide or expose uncertainty appropriately.

Do not add a one-off scoring branch for the first new claim type until the inventory shows that the runtime has a deliberate per-type recomputation plan. If the runtime already has generic trust primitives but no service-owned recomputation trigger, treat that as the work.

## Why This Matters

The value proposition depends on trust being a product feature, not a display label. If some producers emit scored claims and others emit unscored claims, users see uneven `needs_verification` behavior and the system cannot use corroborating source-backed facts to improve trust bands over time.

Claim-backed surfaces also multiply quickly. Tauri, MCP, and WordPress should not each invent their own schema readers, trust shortcuts, or freshness rules. Producers should write into the common substrate, and trust recomputation should run consistently over that substrate.

## When to Apply

Apply this whenever work touches:

- A new `ClaimType`
- A new service-owned claim producer
- A backfill that converts existing schema/cache data into claims
- A new surface that reads `get_entity_intelligence` or claim-backed contexts
- A change to `source_asof`, provenance, trust scores, trust bands, or verification state

## Examples

- Correct: add a producer for sourced account facts, then audit trust recomputation across all claim types before adding type-specific scoring.
- Correct: backfill only source-backed facts and leave `source_asof` empty when no true upstream source date exists.
- Correct: supersede old active claims for the same subject and field when a current sourced fact changes.
- Incorrect: teach MCP or WordPress to read account schema fields directly because claims are missing.
- Incorrect: seed arbitrary trust scores in a producer because a surface looks too cautious.
