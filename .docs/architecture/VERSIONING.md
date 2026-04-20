# Versioning

**Purpose:** Define when to bump which version number. DailyOS carries several independent version concepts; getting the wrong one is a common source of silent incompatibility.
**Date:** 2026-04-20 | **Reflects:** ADRs 0102, 0105, 0106, 0108, 0113
**Audience:** Anyone changing an ability, provenance shape, prompt, or data schema.

## The version numbers that exist

| Version | Lives on | What it describes |
|---|---|---|
| `ability_version` | Ability registry entry, every `AbilityOutput<T>`, every Provenance envelope | Semver of the ability's behavior |
| `ability_schema_version` | Ability registry entry, Provenance envelope | Semver of the ability's input/output type shape |
| `prompt_template_version` | Prompt template registry, `PromptFingerprint` | Semver of a specific prompt template |
| `provenance_schema_version` | Every Provenance envelope | Schema version of the envelope shape itself |
| `trust_version` | `intelligence_claims.trust_version` | Version of the Trust Compiler that scored this claim |
| `record_schema_version` | Every `InvocationRecord` | Schema version of the observability record shape |
| Migration number | `schema_version` table | Monotonic migration counter (108 and growing) |

## When to bump which

### `ability_version` ([ADR-0102](../decisions/0102-abilities-as-runtime-contract.md))

**Bump for:** any change that affects what the ability returns, how it composes, what sources it uses, what the user sees.

Semver rules:
- **PATCH** — bug fix; no observable output change for correct inputs.
- **MINOR** — new capabilities or signals; backward-compatible output shape.
- **MAJOR** — breaking change to output shape, category change (Read → Transform), or contract change.

Bumping `ability_version` invalidates cached outputs (Read ability cache keys include this). Bump even if the change feels minor — cache invalidation depends on it being accurate.

Version strings are `v<major>.<minor>.<patch>`, e.g., `v1.3.0`.

### `ability_schema_version` ([ADR-0102 §8](../decisions/0102-abilities-as-runtime-contract.md))

**Bump for:** changes to the ability's input or output type shape — adding a required field, removing a field, renaming a field, changing a type.

Semver rules same as above. **Does not** need to bump in lockstep with `ability_version` — behavioral changes (e.g., better prompt engineering for the same output shape) bump `ability_version` without touching `ability_schema_version`.

When fixtures reference expected output, they pin both. An `ability_schema_version` bump invalidates fixtures; author updates them as part of the change.

### `prompt_template_version` ([ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md))

**Bump for:** any edit to a prompt template's text content. Cosmetic edits (whitespace, formatting) are canonicalized out of the hash but the template version still bumps — the canonicalization layer is below the version, not around it.

Semver rules:
- **PATCH** — typo fix, whitespace adjustment, rephrase that's expected to produce identical output.
- **MINOR** — added a few-shot example, added a directive, expanded instructions.
- **MAJOR** — restructured the prompt, removed sections, changed role framing.

A `prompt_template_version` MINOR or MAJOR bump triggers eval regression classification ([ADR-0110 §3](../decisions/0110-evaluation-harness-for-abilities.md)) as `PromptChange` — reviewer rebaselines fixtures or approves the delta.

PATCH bumps are expected not to change output; if they do, the bump was wrong (should have been MINOR).

### `provenance_schema_version` ([ADR-0105](../decisions/0105-provenance-as-first-class-output.md))

**Bump for:** breaking changes to the `Provenance` envelope shape itself.

- Adding an optional field: **do not bump**. Backward-compatible.
- Removing or renaming a field: bump MAJOR.
- Changing semantic meaning of an existing field: bump MAJOR.

Currently at version `1`. Versions `2+` require an ADR amendment to justify and a consumer-side migration strategy.

Consumers parsing Provenance envelopes **must** parse forward-compatibly: unknown fields ignored; known fields retain their meaning. The version number exists so a consumer reading an older envelope knows which fields are present.

### `trust_version` ([DOS-7](https://linear.app/a8c/issue/DOS-7) + [ADR-0114](../decisions/0114-scoring-unification.md))

**Bump for:** every Trust Compiler recomputation on a claim row. Monotonic per claim.

This is not a semver version; it's a revision counter. The `trust_score` on a claim row is mutated in place per [ADR-0118 Gap C resolution](../decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md); each recomputation increments `trust_version`. The claim assertion itself is append-only (supersede via new row); only the trust annotation revises.

Consumers filtering on trust (e.g., "show me only claims rescored since this time") use `trust_computed_at`, not `trust_version`. The version number exists for optimistic-concurrency-style reasoning if two processes recompute the same claim's trust.

### `record_schema_version` ([ADR-0120](../decisions/0120-observability-contract.md))

**Bump for:** breaking changes to the `InvocationRecord` shape.

- Adding an optional field: do not bump.
- Adding a new `InvocationKind` variant: do not bump (enums are expected to extend).
- Removing a field, changing its type: bump MAJOR.

Version `1` is the initial shape defined in ADR-0120. Consumers parse forward-compatibly.

### Migration number

**Bump for:** every new SQL migration. Always `NNN_description.sql` in `src-tauri/src/migrations/`.

Migration numbers are never skipped, never reused, never renamed. If you add migration 109 and then realize you need a separate migration first, the pre-109 change becomes a new migration 110 (or you renumber 109 locally before committing). After commit, the number is immutable.

## Common mistakes

### Bumping the wrong version

**Scenario:** You change a prompt template. You bump `ability_version` because the ability produces slightly different output now. This is wrong.

Correct: bump `prompt_template_version`. `ability_version` bumps only if the ability's *contract* changed (inputs, outputs, composition). Changing the prompt changes how the ability produces its output — a quality question the evaluator answers — but not what the ability promises.

### Forgetting to bump at all

**Scenario:** You fix a bug in an ability. Behavior changes. You don't bump.

Consequence: cached outputs persist because the cache key includes `ability_version`. Users see stale state until cache entries naturally expire. Always bump PATCH on any bug fix that could produce different output.

### Bumping in lockstep when not needed

**Scenario:** You add a new field to an ability's output. You bump `ability_version` (correct) and `ability_schema_version` (correct) **and** `provenance_schema_version` (incorrect — provenance envelope didn't change).

Only bump the version that describes the thing that actually changed. Over-bumping creates false regression noise in eval harness classification.

### Bumping after the fact

Bumps are PR-time decisions. If you forgot, amend the PR; don't ship a hotfix that bumps versions for already-merged work — the downstream effects (cache invalidation, fixture regen) need to align with the code change.

## Quick reference

| You changed... | Bump |
|---|---|
| Ability's bug, same output shape and semantics | `ability_version` PATCH |
| Ability's behavior, same shape (e.g., better filtering, richer grounding) | `ability_version` MINOR |
| Ability's input or output type shape | `ability_version` MAJOR + `ability_schema_version` MAJOR |
| Prompt text within a template | `prompt_template_version` (PATCH/MINOR/MAJOR per rules above) |
| Provenance envelope shape | `provenance_schema_version` MAJOR (rare) |
| Trust Compiler formula or weights | `trust_version` bumps on every claim recomputation; compiler itself gets a new minor version of whatever code change (tracked via git) |
| `InvocationRecord` shape | `record_schema_version` MAJOR (rare) |
| SQL schema | New migration number |
| Ability went from experimental to real | `ability_version` starts at `v1.0.0`; earlier experimental versions are not graduation targets |

## The rule that ties it all together

A consumer reading any output should be able to tell, from its version numbers alone, whether it can be compared to another output. Two outputs with the same `ability_version` + `ability_schema_version` + `prompt_template_version` + `provenance_schema_version` are **directly comparable**. Any difference means the comparison may need annotation.

This is what lets the evaluation harness ([ADR-0110](../decisions/0110-evaluation-harness-for-abilities.md)) classify regressions correctly: it looks at which version changed and attributes the output delta accordingly.
