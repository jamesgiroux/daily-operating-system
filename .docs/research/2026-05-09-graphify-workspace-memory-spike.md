# Graphify Workspace Memory Spike

**Date:** 2026-05-09
**Status:** Research note - input for v1.4.9 Workspace Memory Refactor and v1.4.10 MCP Server v2
**Linear context:** DOS-476, DOS-479, DOS-481, DOS-482
**External source:** https://github.com/safishamsi/graphify and https://graphify.net/

---

## Executive Summary

Graphify is useful reference architecture, but it should not become a DailyOS runtime dependency or fork target for product code.

The valuable idea is not "store DailyOS memory in Graphify." The valuable idea is a derived, queryable graph projection plus an audit report that helps an AI agent understand what source material exists, how things connect, which edges are inferred, and what needs review.

DailyOS already has the harder substrate: entities, claims, source lifecycle, provenance, trust bands, source-as-of, signals, invalidation, ability runtime, and MCP. Graphify can sharpen how v1.4.9 exposes workspace memory as an explainable graph and how v1.4.10 lets host models query it headlessly.

Recommendation:

1. Use Graphify as a reference for graph projection, graph audit reports, MCP graph traversal tools, and security hardening.
2. Do not adopt Graphify as the authoritative graph store.
3. Do not ship NetworkX/Python as the core workspace memory engine.
4. Add a DailyOS-native derived source graph projection and audit report if it is not already explicit in v1.4.9.
5. Expand DOS-479 so v1.4.10 exposes graph-shaped workspace memory tools, not just search-shaped tools.

---

## What Graphify Actually Is

Graphify is an MIT-licensed Python tool/package (`graphifyy`, CLI command `graphify`) that turns a folder of code, docs, PDFs, images, and videos into:

- `graphify-out/graph.json`
- `graphify-out/GRAPH_REPORT.md`
- `graphify-out/graph.html`

Its pipeline is cleanly staged:

```text
detect -> extract -> build -> cluster -> analyze -> report -> export
```

The code path uses Tree-sitter for local code extraction, NetworkX for graph construction, Leiden-style community detection, SHA256 caching, and optional MCP serving over stdio.

Its relationship confidence model is simple:

- `EXTRACTED`: explicitly found in source
- `INFERRED`: model or heuristic inferred the edge
- `AMBIGUOUS`: uncertain and should be reviewed

Graphify's MCP server exposes graph traversal patterns such as:

- query graph
- get node
- get neighbors
- shortest path
- graph stats
- god nodes
- report/resources

---

## What DailyOS Should Copy

### 1. Derived Graph Projection

Graphify's best product insight is that an AI agent benefits from a compact map before it reads raw material.

For DailyOS, the projection should be derived from our authoritative substrate:

- workspace sources
- source versions or hashes
- ingestion runs
- entity links
- claim proposals
- committed claims
- provenance envelopes
- trust compiler output
- lifecycle state
- sensitivity state

It should not be created by independently rescanning the filesystem as an authority.

### 2. Graph Audit Report

Graphify's `GRAPH_REPORT.md` is valuable because it summarizes:

- core nodes
- surprising cross-file or cross-domain links
- confidence breakdown
- ambiguous edges
- suggested questions
- knowledge gaps

DailyOS should produce an equivalent source graph audit for v1.4.9 validation and v1.4.10 MCP resources. The DailyOS report should be more intelligence-aware:

- sources by lifecycle state
- sources excluded from intelligence and why
- claims without usable provenance
- claims sourced only from stale material
- inferred edges requiring review
- contradictions or competing claims
- hidden/deleted/scratchpad exclusion proof
- source-as-of coverage
- trust-band coverage
- sensitivity/redaction coverage

### 3. Confidence Separate From Trust

Graphify's `EXTRACTED | INFERRED | AMBIGUOUS` taxonomy is useful, but it is not the same as DailyOS trust.

DailyOS should keep two separate concepts:

- `extraction_certainty`: how sure we are that a relationship/fact was extracted correctly
- `trust_band`: whether the resulting claim should be used now

Example:

```text
An explicitly extracted renewal date from a two-year-old note can have high extraction certainty
and low current trust.
```

### 4. MCP Graph Traversal Shape

Graphify's MCP tools are a good shape for host-model ergonomics. DailyOS should expose graph traversal in v1.4.10, but with claim/provenance/trust semantics.

Candidate tools/resources:

- `query_workspace_graph`
- `get_workspace_graph_node`
- `get_entity_source_neighbors`
- `shortest_workspace_source_path`
- `get_workspace_graph_audit`
- `dailyos://workspace-graph/report`
- `dailyos://workspace-graph/audit`
- `dailyos://workspace-graph/gaps`

These should be read-only, privacy-rendered, lifecycle-aware, and service/ability-backed.

### 5. Security Guardrails

Graphify has useful security patterns:

- block private/link-local/metadata URLs during ingest
- cap fetch sizes
- revalidate redirects
- do not follow symlinks during tree walk
- validate output path containment
- sanitize labels before rendering or returning to MCP
- handle corrupt graph JSON safely
- avoid network listeners by using stdio MCP

DailyOS should adopt the spirit of these controls, especially for URL/file intake, preview rendering, and MCP text output.

---

## What DailyOS Should Not Copy

### 1. Graphify As Authority

DailyOS should not make `graph.json` the source of truth. The source of truth stays in encrypted SQLite and service-owned domain tables.

DailyOS graph output should be disposable and rebuildable.

### 2. Runtime Python/NetworkX Dependency

Graphify is Python-first. That is fine for an AI coding assistant skill, but not ideal for the native DailyOS runtime. A Python graph engine would complicate packaging, update behavior, privacy expectations, and error handling.

Prefer one of:

- SQL-backed traversal for the first version
- Rust in-memory projection using a crate such as `petgraph` if traversal logic grows
- JSON node-link export only as an artifact, not as the authoritative store

### 3. Generic File Manager Scope

Graphify treats folders as corpora. DailyOS is not a generic corpus explorer. DailyOS should expose source memory through entities, claims, provenance, lifecycle, and headless AI workflows.

### 4. Blind Assistant Hooking

Graphify installs assistant rules that tell agents to read the graph before grepping/searching. That makes sense for a codebase map.

DailyOS should be more specific. Host models should use DailyOS when the question is about private personal intelligence, source provenance, workspace memory, or entity context. They should not treat DailyOS as a generic filesystem search layer.

### 5. Unqualified Privacy Claims

Graphify's local code extraction story is strong, but semantic extraction for docs/images/PDFs can use configured model APIs. DailyOS should keep privacy language precise: local state is private; model inference privacy depends on the configured provider and egress policy.

---

## Empirical Check Against A Representative DailyOS Workspace

I installed Graphify into an isolated Python 3.12 virtualenv and ran local-only checks against a representative DailyOS workspace. I did not run Graphify's full semantic extraction path against private workspace documents because the headless `graphify extract` command requires an LLM backend and would send document content to the configured provider.

The local detection pass found:

- 2,944 supported files
- about 4.19M words
- 2 code files
- 2,938 document files
- 4 PDF/paper files
- 1 skipped sensitive file

Graphify correctly warned that full semantic extraction would be expensive and should be run on a subfolder or in AST/local-only mode.

I then ran a local structural extraction over the files Graphify can process without an LLM:

- 1,519 Markdown/code files selected
- 1,425 detected text/HTML/PDF files left out of the local structural graph
- 16,792 nodes
- 15,376 edges
- 1,590 communities
- 1,442 connected components
- 100% `EXTRACTED` edges
- 100% `contains` edges

This is the most important finding from the spike:

> A filesystem-derived Markdown graph creates lots of source structure, but not DailyOS intelligence.

The local graph mostly became per-file heading trees. It did not create meaningful cross-source links, entity links, claim support edges, contradiction edges, trust-band edges, source-as-of edges, lifecycle exclusions, or provenance paths. Those are exactly the things DailyOS already knows how to model.

So Graphify is a strong reference for the shape of graph outputs and MCP ergonomics, but DailyOS must compile its graph from the workspace memory substrate, not from filesystem structure alone.

---

## Architecture Recommendation

### Authoritative Substrate

```text
workspace_sources
workspace_source_versions
workspace_source_entity_links
workspace_ingestion_runs
claim_proposals
claims
claim_provenance
trust_compiler_outputs
source_lifecycle_events
```

### Derived Projection

```text
source graph projection = deterministic read model compiled from authoritative substrate
```

This projection can be a materialized table, view, or in-memory service output. The first version should optimize for correctness and explainability over graph-database power.

### Node Types

- `source`
- `source_version`
- `source_section`
- `entity`
- `claim`
- `claim_proposal`
- `ingestion_run`
- `workspace_location`
- `topic` only if there is a closed taxonomy or confidence policy

### Edge Types

- `source_links_entity`
- `source_supports_claim`
- `source_contradicts_claim`
- `claim_about_entity`
- `claim_supersedes_claim`
- `claim_derives_from_claim`
- `source_derived_from_source`
- `source_generated_by_agent`
- `source_located_in_workspace`
- `run_processed_source`
- `run_emitted_claim_proposal`

### Edge Fields

- `extraction_certainty`
- `extraction_confidence_score`
- `trust_band`
- `source_asof`
- `observed_at`
- `lifecycle_state`
- `sensitivity`
- `provenance_ref`
- `ingestion_run_id`
- `stale_reason`
- `redaction_policy`

### Compile Flow

```text
register source
  -> classify source
  -> link source to entity/workspace location
  -> extract facts
  -> propose claims
  -> compile source graph projection
  -> produce source graph audit
  -> serve projection through UI and MCP
```

The graph projection should be invalidated by the same lifecycle/signals work in DOS-471.

---

## v1.4.9 Prep

v1.4.9 should explicitly own the derived source graph projection or at least the audit/report shape that v1.4.10 will consume.

Recommended hardening:

1. DOS-466 should treat graph projection compilation as a named stage or follow-on service output.
2. DOS-470 should emit extraction certainty separately from claim trust.
3. DOS-471 should invalidate graph projection/audit output when source lifecycle, entity links, claim proposals, claims, or user feedback change.
4. DOS-472 should show "why this source is used" through source/entity/claim edges rather than a file-manager hierarchy.
5. DOS-476 should require a source graph audit proof against real workspace data.

Recommended new issue if this is not already covered:

**Add derived workspace source graph projection and audit report**

This should be a derived read model, not a new authoritative graph database.

---

## v1.4.10 Prep

v1.4.10 should use the v1.4.9 projection as a headless product surface.

Recommended hardening:

1. DOS-479 should explicitly include graph traversal tools, not only source search.
2. DOS-481 should test whether host models select DailyOS for provenance/trust/source-coverage questions instead of generic file search.
3. DOS-482 should include one validation scenario where the host model answers a workspace-memory question using graph traversal, then drills into provenance and trust.
4. MCP outputs should cite source graph edges in claim-shaped language:

```text
This answer is supported by 3 source edges:
- source A supports claim X, likely_current
- source B contradicts older claim Y, needs_verification
- source C links this claim to project Z, use_with_caution
```

5. MCP should not expose arbitrary source file reading. Raw preview should remain a separate policy-controlled tool, aligned with DOS-473.

---

## Open Architecture Choices

### Materialized Table vs Computed Read Model

Recommendation: start with computed read model plus cached artifact for validation. Add materialized tables only if traversal performance requires it.

Reason: v1.4.9 already has schema surface area. Premature graph tables risk making a second source of truth.

### SQL Traversal vs Rust Graph Library

Recommendation: start with SQL queries and small in-memory traversal. Move to a Rust graph library only when v1.4.10 tools need deeper pathfinding/ranking.

Reason: initial user value is source explanation, not arbitrary graph analytics.

### Graph HTML Preview

Recommendation: do not ship a Graphify-style `graph.html` as a product feature in v1.4.9. It can be a developer proof artifact.

Reason: DailyOS's UI should show source usage, provenance, and lifecycle, not become a graph visualization tool.

### Obsidian/PARA Export

Recommendation: defer. v1.4.9 should make the local workspace disciplined and ingestible. Export/sync to Obsidian-style vaults is a separate product question.

---

## L0 Spike Verdict

Proceed with Graphify as reference architecture.

Do not fork Graphify for DailyOS product runtime.

Add or harden v1.4.9 work so source graph projection/audit is explicit. Expand v1.4.10 so MCP can query that projection directly with provenance and trust semantics.
