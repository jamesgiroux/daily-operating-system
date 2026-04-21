# ADR-0119: Runtime Evaluator Pass for Transform Abilities

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.5.0 (implementation) / v1.4.0 enrichment research spike input
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0110](0110-evaluation-harness-for-abilities.md)
**Related:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) (Gap A)
**Resolves:** [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap A — Runtime evaluator pass

## Context

[ADR-0110](0110-evaluation-harness-for-abilities.md) defines an LLM-as-judge rubric scoring model for Transform abilities. It runs **at test time** on fixtures to gate PR merges. This catches prompt regressions and logic bugs, but it cannot see production inputs and does not run on live invocations. When a Transform ability produces a bad output in production — an insight that reads confidently but is wrong, a brief that misses the meeting's actual context, a stakeholder call that mis-classifies intent — the output passes structural verification ([`validation.rs`](../../src-tauri/src/intelligence/validation.rs), [`consistency.rs`](../../src-tauri/src/intelligence/consistency.rs)) and reaches the user unchallenged.

[ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap A names this explicitly: the generator/evaluator separation Anthropic identifies as the single strongest harness lever exists only at test time. Runtime has a generator and deterministic sanity checks, but no evaluator.

This ADR specifies the runtime evaluator pass: an additional LLM invocation after a Transform ability's primary output, scoring the output against a rubric, and triggering re-prompt on low scores. It is intentionally simple — reuses [ADR-0110](0110-evaluation-harness-for-abilities.md)'s rubric definitions, reuses [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)'s provider interface, and adds no new storage beyond a trace row per evaluation.

The v1.4.0 enrichment refactor research spike ([DOS-241](https://linear.app/a8c/issue/DOS-241)) is the natural integration point: enrichment is the highest-volume Transform path and the place where evaluator cost and quality gains are most measurable.

## Decision

### 1. Evaluator hook

Every Transform ability's service-layer wrapper gains an optional evaluator hook invoked after primary output + `TrustAssessment` computation, before the output is returned to the caller:

```rust
pub async fn invoke_transform_with_evaluation<T>(
    ability: &dyn TransformAbility<Output = T>,
    ctx: &AbilityContext,
    input: T::Input,
) -> Result<AbilityOutput<T>, AbilityError> {
    let primary = ability.invoke(ctx, input).await?;
    
    if should_evaluate(ability.name(), ctx) {
        match evaluator::score(&primary, ability.name(), ctx).await {
            Ok(score) if score.composite >= threshold_for(ability.name()) => {
                Ok(primary.with_evaluator_annotation(score))
            }
            Ok(score) => {
                let retry = ability.invoke_with_critique(ctx, input, &score.critique).await?;
                // Re-score once; do not recurse.
                let retry_score = evaluator::score(&retry, ability.name(), ctx).await?;
                Ok(retry.with_evaluator_annotation(retry_score))
            }
            Err(e) => {
                log_evaluator_failure(ability.name(), &e);
                Ok(primary.with_evaluator_skipped(e))
            }
        }
    } else {
        Ok(primary.with_evaluator_skipped(SkipReason::Sampled))
    }
}
```

Properties:

- **Single retry, no recursion.** A failed retry is shipped with a low score recorded, not re-evaluated in a loop. Avoids unbounded cost and infinite re-prompt cycles.
- **Evaluator failure never blocks primary output.** If the evaluator LLM call errors, the primary output ships with a `SkipReason::Error` annotation. Worse-case degenerates to current behavior.
- **Opt-in per ability.** `should_evaluate` consults ability config. Initial rollout enables only on opt-in abilities ([DOS-241](https://linear.app/a8c/issue/DOS-241) flags enrichment as first candidate).

### 2. Rubric source — reuse [ADR-0110](0110-evaluation-harness-for-abilities.md)'s `quality.toml`

Rubric dimensions, thresholds, and judge prompts are defined exactly once — in the `quality.toml` files under `src-tauri/tests/abilities/{ability_name}/evals/` per [ADR-0110](0110-evaluation-harness-for-abilities.md) §5.

The runtime evaluator loads the same file at startup. A `quality.toml` change propagates to both test-time and runtime scoring. No second rubric definition. Reviewers tuning rubrics for test-time regression fixtures see the impact on runtime scoring automatically; tuning them in opposite directions is not possible.

### 3. Judge prompt structure

Per [ADR-0110](0110-evaluation-harness-for-abilities.md) §5, judge prompts produce structured rubric scores. The runtime evaluator uses the same judge prompt:

```
[Judge system prompt — from quality.toml]
You are evaluating a {ability_name} output. Score it 0-3 on each rubric dimension.

[Context the primary ability saw]
{input_summary}
{retrieval_summary}

[The output to evaluate]
{primary_output_rendered}

[Output schema]
{
  "scores": {
    "specificity": {"value": 0..3, "reason": "string"},
    "groundedness": {"value": 0..3, "reason": "string"},
    "actionability": {"value": 0..3, "reason": "string"},
    "non_repetition": {"value": 0..3, "reason": "string"}
  },
  "critique": "string (used to drive retry if composite below threshold)"
}
```

`composite = weighted sum of dimension scores / max possible * 100`. Threshold per ability in `quality.toml`. Judge output is itself fingerprinted ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)) and stored for audit.

### 4. Sampling and cost control

Runtime evaluation doubles LLM call count on Transform paths. Sampling strategy per ability, declared in `quality.toml`:

```toml
[runtime_evaluation]
enabled = true
sample_rate = 0.2                      # Evaluate 20% of invocations
always_evaluate_on_failure_suspected = true  # Overrides sample on signals below
failure_suspicion_triggers = [
  "consistency_check_low_severity",    # Consistency found something short of re-prompt
  "validation_anomaly_detected",       # Anomaly logged but not blocking
  "trust_score_below_band_boundary",   # Trust compiler put score near use_with_caution
]
judge_model_tier = "fast"              # Use cheaper model tier for evaluation
```

Sampling rate starts at 0.2 for opt-in abilities, tuned up over time as variance and cost are measured. `always_evaluate_on_failure_suspected` ensures low-confidence outputs are always evaluated, even off-sample — focusing the evaluator budget where it matters most.

Judge model tier defaults to `fast` ([ADR-0091](0091-intelligence-provider-abstraction.md) tier enum); a smaller, cheaper model is sufficient for rubric scoring and keeps cost in check. Tier override per ability is supported for cases where the judge itself needs capability.

### 5. Critique propagation to retry

When `composite < threshold`, the ability is re-invoked with the evaluator's `critique` field attached as additional context:

```rust
pub trait TransformAbility {
    async fn invoke(&self, ctx: &AbilityContext, input: Self::Input) -> Result<AbilityOutput<Self::Output>, AbilityError>;
    
    async fn invoke_with_critique(
        &self,
        ctx: &AbilityContext,
        input: Self::Input,
        critique: &str,
    ) -> Result<AbilityOutput<Self::Output>, AbilityError>;
}
```

Default implementation of `invoke_with_critique` appends a `previous_attempt_critique` field to the prompt input. Abilities may override for richer critique handling.

The retry's provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) records:

- The initial output's invocation_id.
- The critique text.
- The retry's fingerprint (which differs from the initial; appending critique changes the canonical prompt hash).
- The final score after retry.

This preserves the trail — reviewers can see "primary output scored 42, critiqued as 'too vague on stakeholder status,' retry scored 78."

### 6. Trace storage

Every evaluator invocation produces a row in `evaluation_traces`:

```sql
CREATE TABLE evaluation_traces (
  id                   TEXT PRIMARY KEY,
  ability_name         TEXT NOT NULL,
  ability_version      TEXT NOT NULL,
  primary_invocation_id TEXT NOT NULL,
  primary_output_hash  TEXT NOT NULL,
  judge_model          TEXT NOT NULL,
  judge_prompt_version TEXT NOT NULL,
  scores_json          TEXT NOT NULL,       -- Dimension scores
  composite            REAL NOT NULL,
  critique             TEXT,
  threshold            REAL NOT NULL,
  passed               INTEGER NOT NULL,    -- 0/1
  retry_invocation_id  TEXT,                -- Set if retry happened
  retry_composite      REAL,
  evaluated_at         TIMESTAMP NOT NULL,
  duration_ms          INTEGER NOT NULL
);
CREATE INDEX idx_eval_traces_ability ON evaluation_traces(ability_name, ability_version, evaluated_at DESC);
CREATE INDEX idx_eval_traces_failed ON evaluation_traces(ability_name, passed) WHERE passed = 0;
```

Used for:

- Telemetry — rubric score distributions per ability, retry rates, cost tracking.
- Stability metrics — is score variance drifting over time for a given ability version?
- Debugging — [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap E's debug trace surface consumes this.

Retention: 90 days of trace rows, pruned by a scheduled maintenance ability.

### 7. Mode awareness

Under `ExecutionMode::Evaluate` ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)), the evaluator is **not** invoked. Fixture replay returns deterministic primary outputs; re-running the evaluator against them would produce scores that depend on the current state of the judge model and rubric, breaking replay determinism. Test-time scoring is [ADR-0110](0110-evaluation-harness-for-abilities.md)'s job; runtime scoring is this ADR's job. They do not overlap.

Under `ExecutionMode::Live`, the evaluator runs per §4.

Under `ExecutionMode::DryRun` ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)), the evaluator runs but any retry is also dry-run — the retried output is a preview, not a write.

### 8. Interaction with Trust Compiler

The evaluator composite score is distinct from trust score. Trust ([ADR-0114](0114-scoring-unification.md), [DOS-5](https://linear.app/a8c/issue/DOS-5)) answers "how much do we believe this claim's inputs." Evaluator composite answers "how good is the synthesis the ability produced." Both feed `TrustAssessment`:

- Trust compiler produces per-claim `trust_score`.
- Evaluator composite is stored on the ability output (not per-claim) via a new `output_quality_score` field in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) amendment required).
- Surfaces render both: "high trust, high output quality" vs "high trust, low output quality" (the synthesis is wrong even though inputs are good).

The amendment to [ADR-0105](0105-provenance-as-first-class-output.md) adds:

```rust
pub struct Provenance {
    // ... existing fields
    pub output_quality_score: Option<f64>,
    pub output_quality_skipped: Option<SkipReason>,
}
```

### 9. Stability-as-confidence integration

[ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap A noted that runtime evaluator becomes more powerful with claim history. Now that [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) specifies append-only claim history via supersede pointers, the evaluator can include a **stability-as-rubric-dimension** option:

```toml
[rubric.stability]
enabled = true                         # Opt-in per ability
lookback_days = 30
prior_assertions_weight = 0.15         # Contributes 15% to composite
```

When enabled, the evaluator includes a sixth dimension: "is this assertion stable — has it held across prior runs, or is it flapping?" Stability is computed from the claim history table, not from the LLM. High stability increases composite; flapping decreases it. This dimension rewards abilities that converge on consistent outputs.

Opt-in because it only applies to abilities whose outputs map to stable claim fields (enrichment yes, meeting prep mostly no). Not all Transform abilities benefit.

### 10. Rollout plan

- **v1.4.0:** ADR proposed; no implementation. Enrichment research spike ([DOS-241](https://linear.app/a8c/issue/DOS-241)) includes evaluator integration in its design doc.
- **v1.4.1:** Evaluator hook infrastructure lands (trait extensions, `evaluation_traces` table, config loading). Gated off by default.
- **v1.5.0:** First Transform ability opts in — the enrichment dimension abilities coming out of the v1.4.1 refactor. Sampling starts at 0.2. Cost + quality tracked.
- **v1.5.1+:** Stability dimension activates for abilities with sufficient claim history (requires [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) R1.9 enrichment consolidation complete).
- **v1.6.0 Hardening:** Opt-in expanded based on measured gains; sampling rates tuned; judge model tier choices revisited.

## Consequences

### Positive

- **Closes [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap A.** Generator/evaluator separation operational at runtime, not just test time. Matches Anthropic's identified single-strongest harness lever.
- **Rubric reused from [ADR-0110](0110-evaluation-harness-for-abilities.md).** One rubric definition; test-time and runtime cannot drift.
- **Retry with critique is structural.** Not ad-hoc re-prompting — the retry is fingerprinted, provenanced, and traced like any other invocation.
- **Stability-as-confidence available** once claim history lands. Composes cleanly with [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
- **Cost is controlled.** Sampling + cheap judge model tier + failure-suspicion focus keeps budget tractable. Initial 20% sample on opt-in abilities only.
- **Mode-aware.** Evaluator does not run in `Evaluate` mode; test-time scoring ([ADR-0110](0110-evaluation-harness-for-abilities.md)) is unchanged.

### Negative / risks

- **Doubles LLM call count on evaluated invocations.** Mitigated by sampling, cheap tier, failure-suspicion. Cost budget must be measured during v1.5.0 rollout; revisit sample rates if cost materially impacts usage.
- **Evaluator can have its own bad days.** A misbehaving judge model produces false lows (unnecessary retries) or false highs (bad outputs ship). Mitigated by judge model also being eval-harness-tested per [ADR-0110](0110-evaluation-harness-for-abilities.md); regression tests for the evaluator itself.
- **Retry adds latency.** For user-facing abilities, a retry adds ~2x latency. For `prepare_meeting` (user-blocking), this matters; for background enrichment it doesn't. Per-ability opt-in handles this — enable on background abilities first.
- **Single retry without recursion may leave bad outputs shipped.** Accepted tradeoff — the alternative is unbounded re-prompt loops. Tracing captures when retry still failed; telemetry surfaces chronic failures for prompt revision.
- **`output_quality_score` amendment to [ADR-0105](0105-provenance-as-first-class-output.md).** Tracked as a required follow-up amendment; must land before v1.4.1 evaluator infrastructure.

### Neutral

- No user-facing change until an ability opts in. Rollout is incremental.
- Rubric tuning is a shared concern with [ADR-0110](0110-evaluation-harness-for-abilities.md); expect some cycles of "lowered test-time threshold, didn't mean to lower runtime threshold too" in early v1.5.0. Address with rubric-change reviews.
- The `evaluation_traces` table grows at the evaluated invocation rate × retention window. At 1M enrichment invocations/year × 20% sample = 200K rows/year; well under storage concern with 90-day retention.
