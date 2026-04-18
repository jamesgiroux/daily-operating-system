# ADR-0110: Evaluation Harness for Abilities

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
**Depends on:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0107](0107-source-taxonomy-alignment.md)  
**Consumed by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 (snapshot-diff evaluation), [ADR-0102](0102-abilities-as-runtime-contract.md) §13 (Phase 3 regression gate)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §10 Enforcement Phase 3 requires a regression gate before abilities become authoritative; Risk #5 declares the evaluation harness a non-negotiable co-dependency. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 specifies snapshot-diff evaluation for maintenance abilities. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers in `Evaluate` mode. [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §6 uses fingerprints to classify regression types.

Without a concrete harness, these requirements do not produce action. This ADR specifies fixture structure, scoring methodology, regression classification, CI integration, and fixture governance (retention, anonymization, source-aware lifecycle).

## Decision

### 1. Fixture Shape

An evaluation fixture captures everything needed to deterministically run an ability against a known input state:

```
src-tauri/tests/abilities/{ability_name}/
├── fixtures/
│   ├── fixture_001/
│   │   ├── state.sql              # Pre-populated DB state
│   │   ├── inputs.json            # Ability input values
│   │   ├── provider_replay.json   # Recorded provider completions by canonical_prompt_hash
│   │   ├── external_replay.json   # Recorded external API responses (Glean, Slack, etc.)
│   │   ├── clock.txt              # Deterministic clock value
│   │   ├── seed.txt               # RNG seed
│   │   ├── expected_output.json   # Expected ability output
│   │   └── expected_provenance.json # Expected provenance envelope (with field-level attributions)
│   ├── fixture_002/
│   └── manifest.toml              # Fixture metadata, labels, retention
├── evals/
│   ├── quality.toml               # Per-ability quality criteria
│   └── regression_baseline.json   # Baseline scores for regression detection
```

### 2. Scoring Methodology

Evaluation produces a score for each fixture run. Scoring differs by ability category:

**Read abilities** — exact output equality against `expected_output.json`. Field-level diff surfaces any drift. Confidence thresholds: 100% equality required; any diff fails.

**Transform abilities** — LLM-as-judge scoring via a dedicated judge model. The judge receives the input, the expected output, and the actual output, and produces a rubric score on configurable dimensions (e.g., relevance, faithfulness, attribution completeness). Thresholds are per-ability in `evals/quality.toml`.

**Maintenance abilities** — snapshot-diff per [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9. Compare `planned_mutations` against `expected_output.json.planned_mutations`. Diff on operation set, affected entity IDs, and parameter values. Thresholds: exact match or a configured tolerance (e.g., allow timestamp drift of ≤1s).

**Publish abilities** — the outbox entry is compared, not the external side effect.

### 3. Regression Classification

Per [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §6, fingerprints classify regressions:

```rust
pub enum RegressionClass {
    /// Same inputs, same prompt_template_version, different outputs.
    /// Indicates provider drift or nondeterminism in replay.
    ProviderDrift,
    
    /// Same inputs, different prompt_template_version, different outputs.
    /// Expected when a prompt is edited; reviewer approves or rejects.
    PromptChange,
    
    /// Different inputs, same fingerprint otherwise.
    /// Real input-driven behavior change.
    InputChange,
    
    /// canonical_prompt_hash differs but prompt_template_version matches.
    /// Canonicalization bug or unstamped template edit.
    CanonicalizationBug,
    
    /// Output differs and no fingerprint fields explain why.
    /// Indicates a logic change not captured in fingerprint.
    LogicChange,
}
```

The harness labels each regression with a class. CI treats them differently:

| Class | CI default action | Reviewer workflow |
|-------|-------------------|-------------------|
| ProviderDrift | Warn | Inspect replay fixture; may refresh or declare as nondeterminism |
| PromptChange | Fail-soft (block merge pending explicit approval) | Reviewer approves by rebaselining; CI re-runs against new baseline |
| InputChange | Fail | Reviewer updates expected output for legitimate input-driven change |
| CanonicalizationBug | Fail (hard) | Must fix canonicalization before merge |
| LogicChange | Fail-soft | Reviewer approves rebaseline or rejects as unintended |

### 4. CI Integration

Evaluation runs on every PR that touches `src-tauri/src/abilities/`, prompt template files, or the fixtures themselves. The harness:

1. Builds the binary with the ability under test.
2. For each fixture in the ability's directory:
   - Constructs `ServiceContext::new_evaluate` with the fixture's clock, seed, replay providers.
   - Loads `state.sql` into an in-memory SQLite.
   - Invokes the ability via the registry with `inputs.json`.
   - Compares actual output, provenance, and planned mutations to expected.
   - Classifies any diffs per §3.
3. Aggregates per-ability scores into a regression report.
4. Compares scores to `regression_baseline.json`; flags any ability whose score drops.

Runtime budget: evaluation is hermetic (no network), so full ability suite typically runs in 30–60 seconds. Per-ability isolation allows parallel execution.

### 5. Quality Criteria per Ability

`evals/quality.toml` defines per-ability thresholds:

```toml
[prepare_meeting]
judge_model = "claude-sonnet-4-6"
judge_prompt_id = "eval_meeting_brief_judge.v1"
min_relevance = 0.85
min_faithfulness = 0.9
min_attribution_completeness = 0.95
regression_tolerance = 0.02  # 2% score drop is the alert threshold
```

Judge models invoke a fixture-replayed provider with a dedicated judge prompt that asks for structured rubric scores. Judge outputs are themselves subject to prompt fingerprinting and stored per run for audit.

### 6. Fixture Authoring and Maintenance

Fixtures are created two ways:

- **Hand-authored** for targeted regression cases: author sets up the state, declares the expected output, commits.
- **Captured from Live runs** for coverage: the `--capture-fixture` mode runs an ability in `Live` with a tracing wrapper, records everything the ability saw, and writes a fixture. Captured fixtures must be reviewed before merging.

Fixture refresh cadence: fixtures capturing live provider output are refreshed when `prompt_template_version` changes, when the replay provider returns outdated completions, or on a quarterly schedule.

### 7. Fixture Governance: Anonymization, Retention, Purge

Fixtures can contain PII and source-attributed content. Per [ADR-0098](0098-data-governance-source-aware-lifecycle.md) and [ADR-0107](0107-source-taxonomy-alignment.md), fixtures must honor source lifecycle:

**Anonymization rules (applied at capture time):**

1. Entity names replaced with placeholders (`acct_01`, `person_03`). Mapping preserved in a separate `fixture_identity_map.json` that is NOT committed.
2. Email addresses replaced with `user@example.com`, `person_N@example.com`.
3. Free-text content (email bodies, meeting transcripts, documents) passed through a named-entity redactor; recognizable names, phone numbers, addresses replaced with tokens.
4. Source IDs retained but flagged as "anonymized" so provenance still works; real IDs not present.

**Retention:**

- Fixtures with real-anonymized content live in the repo and are retained as long as the ability exists.
- Fixtures are versioned in git; historical fixtures can be reconstructed from git history.

**Purge on source revocation:**

- When a user revokes a source, the user's `fixture_identity_map.json` (kept out-of-tree) is consulted; fixtures referencing identities mapped to revoked user data are regenerated or removed.
- Repo-stored anonymized fixtures are not affected because the anonymization is the point of keeping them safely in-tree.

**Multi-user note for v1.4.0 (six users, local-first):** fixtures are per-developer. Each user's captured fixtures include their own data, anonymized. No shared fixture corpus crosses users without re-anonymization.

### 8. Regression Gate in Phase 3 Cutover

[ADR-0102](0102-abilities-as-runtime-contract.md) §13 Phase 3 cutover requires the regression gate to be active before the registry becomes authoritative. The gate's entry requirement is:

- Every migrated ability has at least one fixture with an expected output.
- `regression_baseline.json` is populated from an accepted baseline run.
- CI runs the harness on every PR touching ability code.
- Merge is blocked on any hard failure; fail-soft failures require reviewer rebaseline action.

## Consequences

### Positive

1. **Quality becomes measurable.** Per-ability scores in CI; regressions caught at ability granularity.
2. **Regression types distinguished.** Provider drift, prompt change, input change, canonicalization bug, logic change are separately classified.
3. **Hermetic evaluation.** No network dependency; fast feedback loops.
4. **Maintenance evaluation is tractable.** Snapshot-diff works because [ADR-0104](0104-execution-mode-and-mode-aware-services.md) mode gating is structural.
5. **Judge-model evaluation provides nuanced quality signal.** Transform abilities are not limited to exact-match scoring.
6. **Fixture governance aligned with source lifecycle.** Anonymization and purge interoperate with [ADR-0107](0107-source-taxonomy-alignment.md).

### Negative

1. **Fixture authoring is real work.** Every ability needs fixtures before it can be gated.
2. **Judge model is an additional provider invocation.** Doubles cost at evaluation time for Transform abilities.
3. **Capture mode adds infrastructure.** Tracing wrapper, anonymization pipeline, manifest handling.
4. **Fixture drift from production data.** Fixtures capture point-in-time state; real-world drift means fixtures require periodic refresh.

### Risks

1. **Judge model drift.** The judge itself regresses silently. Mitigation: judge fingerprint captured in every eval run; judge model changes trigger rebaseline prompts.
2. **Anonymization bugs leak PII into fixtures.** Mitigation: anonymization is reviewed; pre-commit hook grep for known-PII patterns (phone numbers, email patterns not matching `@example.com`).
3. **Fixture rot.** Abilities evolve; fixtures don't. Mitigation: every ability version bump requires fixture review; fixtures older than quarterly cadence trigger CI advisory.
4. **CI runtime ballooning.** Many fixtures × many abilities = long test time. Mitigation: parallel execution; fixture labels (`@core`, `@regression`, `@edge`) allow selective PR-time subsets with full suite on nightly runs.
5. **Fixture identity map loss.** `fixture_identity_map.json` lost means purge-on-revocation cannot target specific fixtures. Mitigation: identity map is backed up per-developer; reconstruction from git history plus user-side records is possible but manual.

## References

- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Phase 3 regression gate depends on this harness.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — §9 snapshot-diff evaluation consumes this harness.
- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — `Evaluate` mode enables hermetic evaluation; fixtures consumed via `ServiceContext::new_evaluate`.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — Expected provenance is part of every fixture; attribution accuracy gauged via LLM self-eval.
- [ADR-0106: Prompt Fingerprinting and Provider Interface Extension](0106-prompt-fingerprinting-and-provider-interface.md) — Fingerprints drive regression classification in §3.
- [ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md) — Fixture lifecycle honors per-source revocation rules.
- [ADR-0098: Data Governance — Source-Aware Lifecycle](0098-data-governance-source-aware-lifecycle.md) — Anonymization and purge requirements.
