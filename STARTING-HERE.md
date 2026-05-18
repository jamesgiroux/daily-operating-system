# W1 C1 Starter Kit — Implementation Handoff

**Date:** 2026-05-18
**Branch:** `w1-c1-starter-kit` (worktree at `/private/tmp/dailyos-w1`)
**Base:** `docs/v143-l0-packets` at commit `fca8d8b1`
**Linear:** [DOS-678](https://linear.app/a8c/issue/DOS-678)
**L0 packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md` V1.3 (cycle-4 codex challenge APPROVE)

## Why this file exists

The overnight autonomous run got the W0 stabilization fully closed (PRs #301 + #302) and got W1 L0 through 4 cycles to APPROVE. Implementation deferred to the morning per the time budget (~1.5h remaining at handoff; W1 impl realistically takes 4-6h done right).

This file is the pick-up-where-we-left-off doc.

## L0 packet — final V1.3 status

5-reviewer panel:
- code-reviewer cycle 1: CONDITIONAL APPROVE (folded into V1.1)
- CSO cycle 1: CONDITIONAL APPROVE (folded into V1.1)
- DX cycle 1: CONDITIONAL APPROVE (folded into V1.1)
- codex challenge: cycle 1 BLOCK → cycle 2 BLOCK → cycle 3 BLOCK → **cycle 4 APPROVE**
- codex consult: R1 + R2 + R3 all died silently (codex companion stability issue — see `feedback_codex_rescue_stuck_after_research_phase` memory note)

Implementation cleared on 4/5 reviewers (toughest reviewer cleared on cycle 4). If codex consult R3 lands during morning work with substantive findings, fold as a hotfix during impl.

## V1.3 §10 — commit-group ordering (load-bearing)

**Group 1 MUST land first (CI gate enforcement):**
1. Rust integration test harness at `src-tauri/abilities-runtime/tests/block_kit_integration_harness.rs` (or similar) — `BlockIntegrationFixture` + `BindingExpectation` + `ProjectionDiagnostic` + `RendererBranchAssertion` + `BlockWrapperAssertion` value types + `run_block_integration_fixture` function + `integration_test_block!` macro
2. PHP harness at `wp/dailyos/tests/blocks/StarterKitIntegrationTest.php` — generic block-render entrypoint that registers target block metadata, injects fake runtime client, calls WP `render_block()`
3. CI workflow `.github/workflows/block-kit-integration.yml` — enumerates `wp/dailyos/blocks/*` and runs the harness against each on PR
4. First integration fixture against existing `dailyos/account-overview` block — proves the harness works on a real production block

**Group 2:** CLI + 3 template shapes (simple / typed-display / composite) at `wp/dailyos/scripts/`

**Group 3:** Token generator at `wp/dailyos/scripts/generate-theme-json.mjs`

**Group 4:** Translator with scope matrix at `wp/dailyos/scripts/translate-tauri-to-block.mjs`

## Critical implementation references

Real `BlockProjectionRule` shape (from `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255-262`):

```rust
struct BlockProjectionRule {
    block_type: BlockType,
    composition_kind: Option<&'static str>,
    type_namespace: Option<&'static str>,
    render_annotations: &'static [&'static str],
    fields: &'static [FieldPolicy],
    default_trust_band: TrustBand,
}
```

`FieldPolicy` (private at `:237`) is built via in-file helpers: `text_field(pointer, ClaimSensitivity)` / `number_field` / `bool_field` / `object_field` / `array_field` at `:1273+`.

Canonical reference rule: `account_overview_rule()` at `fallback_projection.rs:1415`.

5 paste-target locations in V1.3 §5.4 CLI output:
- `composition.rs:330` — BlockType enum variant
- `composition.rs:350` — BlockType::type_id() exhaustive match arm
- `fallback_projection.rs` near `:1409` — `const <NAME>_FIELDS: &[FieldPolicy]`
- `fallback_projection.rs:1236` — rule_for_block_type() match arm
- `fallback_projection.rs:1250` — known_projection_rules() Vec registration

## Key V1.3 deviations from typical patterns

1. **CLI does NOT modify `class-dailyos-plugin.php`** — existing `register_blocks()` at `:149-163` uses `glob('blocks/*/block.json')`; dropping a new block.json directory auto-registers.
2. **CLI does NOT modify `.rs` files** — emits paste snippets for the developer to apply manually (see V1.3 §5.4 output format).
3. **Schema-based harness fixture** (NOT substring-based) — `BindingExpectation { pointer, value_kind, required }` plus 4-field DOS-670 diagnostic format (location, declared, actual, did_you_mean via edit-distance).
4. **Translator scope matrix** with 4 categories — supported / supported-with-promotion / supported-with-inline-style-adaptation / NOT-supported (interactive). NOT-supported exits 1 with actionable diagnostic.

## Path-α maintenance already filed

- DOS-684 — repo-wide token source-name cleanup + primitive interaction classification column + extract account-overview private renderer

## Test plan for L1

When implementation lands:
```bash
cargo clippy --workspace -- -D warnings
cargo test --workspace
pnpm tsc --noEmit
# PHPCS + phpunit if configured
```

Plus the kit-specific fixture run:
```bash
pnpm dailyos:test-block account-overview  # proves harness on existing block
```

## L4 evidence batched

`/Users/jamesgiroux/.dailyos/l4-batch/W1/` — drop screenshots / video of the CLI walkthrough (CLI scaffold creates a working block; harness fixture passes; the block renders in WP editor) per AC #1 + #5 + #6.

## Status

- Worktree clean as of handoff (only this STARTING-HERE.md is uncommitted)
- L0 packet V1.3 closed at challenge APPROVE
- Linear DOS-678 status: ready for implementation
