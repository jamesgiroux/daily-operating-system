# L0 Packet C — C1 Starter Kit

**Current revision: V1.0 (initial draft, 2026-05-18). See §2 Changelog.**

## 1. Header

Date: 2026-05-18
Project: v1.4.3 — WordPress Foundation
Wave: W1 (Starter kit — gates W2 primitive translation + all subsequent block work)
Issues:
- DOS-678 — v1.4.3 W1 C1: Starter kit (block.json + render.php + producer + projection rule + shared integration fixture)
Surface: WordPress block authoring toolchain + integration test harness
Primary code (new):
- `wp/dailyos/scripts/new-block.mjs` (CLI scaffold)
- `wp/dailyos/scripts/templates/` (block.json + render.php + render-functions.php + style.css + edit.js scaffolds)
- `src-tauri/abilities-runtime/src/abilities/templates/producer_template.rs.tmpl` (Rust producer template)
- `src-tauri/abilities-runtime/src/abilities/templates/projection_rule_template.rs.tmpl` (projection rule template)
- `src-tauri/abilities-runtime/src/abilities/integration_test_harness.rs` (shared producer→projection→renderer test harness — load-bearing)
- `wp/dailyos/tests/blocks/StarterKitIntegrationTest.php` (PHP-side harness exercising a fixture block end-to-end)
- `wp/dailyos/scripts/generate-theme-json.mjs` (token-to-theme.json generator)
- `wp/dailyos/scripts/translate-tauri-to-block.mjs` (TSX/CSS Module → block.json/render.php translator)
Primary anchor: `.docs/plans/v1.4.3-waves.md` §"Wave 1 — C1 Starter Kit"
Reference implementation: v1.4.2 W4-F `dailyos/account-overview` (PR #298 — `wp/dailyos/blocks/account-overview/*` + `src-tauri/abilities-runtime/src/abilities/account_overview.rs`)
Diagnostic anchor: v1.4.3 stabilization Packet B §5.6 typed error mapping (the kit's render-functions.php template uses the same switch table)

This packet ships the v1.4.3 execution-constraint-C1 deliverable: the block authoring playbook as **working code**, not documentation. v1.4.2 W4-F proved the producer→projection→renderer pattern with a single composite block. v1.4.3 W1 generalizes it into a CLI + templates + integration test harness so any new block author plugs in without reading substrate source.

**Intelligence Loop integration check — exempt.** No claim/table/surface added; the kit is a code-generation toolchain over existing substrate primitives. No provenance/trust impact; no signal change; no runtime context surface consumes new state; no feedback loop change. CLAUDE.md §"Critical Rules — Intelligence Loop integration check" does not apply.

## 2. Changelog

- **V1.0 (2026-05-18):** Initial L0 draft. Authored against `.docs/plans/v1.4.3-waves.md` §W1 + Linear DOS-678 + v1.4.2 W4-F reference implementation. Reviewer panel set to codex challenge + codex consult + code-reviewer + DX review. CSO advisory only — no new trust boundaries.

## 3. Status Snapshot

- Linear ticket: DOS-678 (Backlog, v1.4.3 — WordPress Foundation, priority High).
- Reference: v1.4.2 W4-F `dailyos/account-overview` (PR #298, merged 2026-05-17).
- v1.4.3 W0 stabilization (Packets A + B) is implementing in parallel; the §5.6 typed-error switch from Packet B is consumed verbatim by this kit's render-functions.php template.
- 8 acceptance criteria specified per §7 below.
- Recommended landing shape: single PR with 4 commit groups; see §10.
- Reviewer panel: see §14.

## 4. Pre-work — substrate reuse audit

This packet REUSES the following existing primitives. The L0 reviewer panel must reject any net-new primitive in this packet that already exists:

| Capability | Existing primitive | File:line |
|---|---|---|
| Block.json scaffold pattern | `wp/dailyos/blocks/account-overview/block.json` | (full file, ~26 lines) |
| Render.php delegation pattern | `wp/dailyos/blocks/account-overview/render.php` | (full file, ~25 lines) |
| Render-functions.php pattern | `wp/dailyos/blocks/account-overview/render-functions.php` | (full file, ~232 lines — post-Packet-B V1.1.1) |
| Producer ability shape | `src-tauri/abilities-runtime/src/abilities/account_overview.rs` (`#[ability(...)]` macro + `normalize_input → prepare → commit_composition → finalize_provenance`) | `:83-120` |
| Projection rule shape | `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs` `project_known_block` + `validate_field_bindings` | `:393-612` |
| Composition + ProjectedBlock contract | `src-tauri/abilities-runtime/src/abilities/composition.rs` + `fallback_projection.rs` `ProjectedComposition` / `ProjectedBlock` | (existing) |
| WP block registration | `wp/dailyos/includes/class-dailyos-plugin.php` `register_blocks_from_metadata` enumeration | `:154-170` |
| Typed error switch (consumed) | `wp/dailyos/blocks/account-overview/render-functions.php` V1.1.1 switch table | `:61-130` (after Packet B lands) |
| Signed runtime client | `wp/dailyos/includes/transport/class-dailyos-runtime-client.php` | (existing) |
| Test fixture pattern | `wp/dailyos/tests/blocks/AccountOverviewBlockTest.php` (fake-runtime-client injection) | (existing) |
| Rust ability test pattern | `src-tauri/abilities-runtime/src/abilities/account_overview.rs` `#[cfg(test)] mod tests` | (existing) |
| Existing block render route | `surface_runtime::project_composition` | `surface_runtime/mod.rs:2280-2440` (post-Packet-B V1.1.1) |

**Reuse pattern (load-bearing):** the kit is **extraction** of patterns already proven by `dailyos/account-overview`, NOT invention of new primitives. The shared integration test harness is the one net-new primitive — and it directly mirrors the producer→projection→renderer fixture pattern that DOS-670 retrofitted into v1.4.2 W4-F after the L4 finding.

No DB schema changes. No new claim model. No new signal types. No new ability category. No new transport. No new runtime route.

## 5. What this packet authors net-new

### 5.1 CLI scaffold tool (`wp/dailyos/scripts/new-block.mjs`)

A Node.js (or Bash) CLI invoked as `pnpm dailyos:new-block <block-name> [--payload-shape simple|composite] [--ability <producer-name>]`. Default: simple Pill-shape (single payload field, e.g., `text`). `composite` shape gets the AccountOverview-style multi-block scaffold.

Responsibilities:
1. Validate `<block-name>` matches `[a-z][a-z0-9-]+` (Gutenberg block name slug rules).
2. Copy templates from `wp/dailyos/scripts/templates/<shape>/` into `wp/dailyos/blocks/<block-name>/` with name interpolation (`{{BLOCK_NAME}}`, `{{ABILITY_NAME}}`, `{{PHP_FUNCTION_PREFIX}}`).
3. Update `wp/dailyos/includes/class-dailyos-plugin.php` to register the new block (insert into the block-metadata enumeration at `:154-170`).
4. If `--ability` flag is provided AND the ability doesn't exist, copy the producer template into `src-tauri/abilities-runtime/src/abilities/<ability-name>.rs` and register in the ability registry.
5. Print the next steps: edit X, edit Y, run the kit integration fixture to verify.

CLI errors out on existing block name (no overwrite). Exit codes: 0 success, 1 validation error, 2 partial state (manual cleanup needed).

### 5.2 Block scaffold templates (`wp/dailyos/scripts/templates/{simple,composite}/`)

Two template sets (per-shape):

**Simple shape (Pill-class — single-payload primitive):**
- `block.json` — TypeScript schema-validated; declares `composition_id`, `composition_version`, `block_id`, `cache_hint_token`, `block_instance_id` (per v1.4.2 W4-F standard 5 attributes) plus per-block payload attrs interpolated from a `--payload-shape` schema.
- `render.php` — minimal entrypoint delegating to `render-functions.php` (same as v1.4.2 account-overview).
- `render-functions.php` — fetch-once pattern, calls `project_composition_for_surface` once, uses the §5.6 typed-error switch from Packet B verbatim, renders the ProjectedBlock payload.
- `style.css` — minimal CSS Module with token-only values (no hardcoded colors/spacing).
- `edit.js` — uses the reloadTrigger pattern from Packet B §5.2 (composition_id + account_id presence as derived trigger key; full reload callback deps; manual reload button).
- `editor.css` — minimal editor-specific styles.

**Composite shape (AccountOverview-class — multi-block composition):**
- Same as simple, plus per-block-type render branches in render-functions.php (mirroring the §5.6 switch + the existing payload.text/items/nodes/context dispatch from account-overview).

### 5.3 Producer ability template (`src-tauri/abilities-runtime/src/abilities/templates/producer_template.rs.tmpl`)

Rust file scaffold with placeholder regions:

```rust
// {{ABILITY_NAME}} producer ability (v1.4.3 W1 starter-kit scaffold).
//
// Generated by `pnpm dailyos:new-block --ability {{ABILITY_NAME}}`.
// Edit the marked TODO regions to define the producer's input/output shape;
// the surrounding scaffold (normalize_input → prepare → commit_composition →
// finalize_provenance) is the v1.4.2 W4-F load-bearing pattern.
//
// The integration test harness at
// `src-tauri/abilities-runtime/src/abilities/integration_test_harness.rs`
// exercises this producer end-to-end via the WP renderer; do not modify the
// commit/finalize call shapes or that harness will fail.

use chrono::{DateTime, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct {{InputStructName}} {
    pub schema_version: u32,
    pub composition_id: Option<String>,
    pub expected_composition_version: u64,
    // TODO: add per-block input fields here
}

#[ability(
    name = "dailyos/{{ABILITY_NAME_KEBAB}}",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.{{SCOPE_NAME}}"],
    mcp_exposure = Invocable,
    client_side_executable = false,
    composes = [],
    experimental = false
    // TODO: signal_policy if this producer reacts to upstream signals
)]
pub async fn {{ability_fn_name}}(
    ctx: &AbilityContext<'_>,
    input: {{InputStructName}},
) -> AbilityResult<Composition> {
    let input = normalize_input(input)?;
    let prepared = prepare_{{ability_fn_name}}(ctx, &input).await?;
    let committed = ctx
        .services()
        .commit_composition(prepared.proposal)
        .await
        .map_err(composition_commit_error)?;
    let output = prepared
        .provenance_builder
        .finalize(committed.composition)
        .map_err(provenance_error)?;
    Ok(output)
}

// TODO: implement normalize_input, prepare_{{ability_fn_name}}, helper types
```

### 5.4 Projection rule template (`src-tauri/abilities-runtime/src/abilities/templates/projection_rule_template.rs.tmpl`)

Rust snippet to be inserted into `fallback_projection.rs` registry. Template:

```rust
// Registered by `pnpm dailyos:new-block` for the {{ABILITY_NAME}} producer.
register_custom_block_schema(CustomBlockSchema::new("dailyos/{{ABILITY_NAME_KEBAB}}")
    .with_field_binding(BindingRole::Title, FieldPath::new("payload.title"))
    .with_field_binding(BindingRole::Text, FieldPath::new("payload.text"))
    // TODO: declare additional field bindings for this block's payload shape
);
```

### 5.5 Shared integration test harness (`src-tauri/abilities-runtime/src/abilities/integration_test_harness.rs`)

**Load-bearing primitive.** Exercises producer → projection → renderer end-to-end against a substrate test DB. Modeled on the v1.4.2 W4-F retrofit that DOS-670 forced.

Shape:

```rust
pub struct BlockIntegrationFixture {
    pub ability_name: String,           // e.g. "dailyos/account-overview"
    pub composition_id: String,         // synthetic, generated per test
    pub input_json: serde_json::Value,  // producer input
    pub expected_projection_keys: Vec<String>,  // ProjectedBlock keys the projection should emit
    pub expected_renderer_html_contains: Vec<String>,  // PHP renderer output assertions
}

/// Runs the full producer → projection → renderer pipeline for a block.
/// Caller passes a fixture; harness builds a substrate test DB, invokes the
/// producer, runs `project_from_ability_data` against the result, then
/// shells out to a PHP test harness (StarterKitIntegrationTest.php) to
/// render the projection and assert HTML shape.
///
/// Returns the rendered HTML on success; panics with a diagnostic on
/// contract mismatch (the kind that DOS-670 found).
pub async fn run_block_integration_fixture(fixture: BlockIntegrationFixture) -> String;

/// Convenience macro for new-block authors:
///
/// ```rust
/// integration_test_block!(my_block, BlockIntegrationFixture { ... });
/// ```
#[macro_export]
macro_rules! integration_test_block { ... }
```

The PHP-side harness at `wp/dailyos/tests/blocks/StarterKitIntegrationTest.php` accepts a projection JSON file path + ability name + expected HTML substring list, renders via the existing `render_block_with_filter` infrastructure (with a fake runtime client returning the projection), asserts substring presence, exit-codes 0 on success.

### 5.6 Token-to-theme.json generator (`wp/dailyos/scripts/generate-theme-json.mjs`)

Reads the three canonical token sources (`.docs/design/tokens/`, `src/styles/tokens.css`, `wp/dailyos/theme/theme.json`), merges per a documented precedence order (theme.json > tokens.css > design/tokens for runtime overrides; design/tokens > tokens.css > theme.json for canonical definitions — explicit pick in §6.5), produces a synced `wp/dailyos/theme/theme.json` with all token settings under `settings.color.palette`, `settings.typography.fontSizes`, `settings.spacing.spacingSizes`, etc.

Idempotent: re-running with no token changes produces zero diff. Validates that no two sources define the same token with different values; errors out with a conflict report if so.

### 5.7 Translation utility (`wp/dailyos/scripts/translate-tauri-to-block.mjs`)

Reads a Tauri React component path (e.g., `src/components/ui/Pill.tsx`) + its CSS Module (`Pill.module.css`), emits a `wp/dailyos/blocks/<name>/` directory with `block.json` + `render.php` + `render-functions.php` + `style.css` derived from:
- TSX → PHP: extract props, map to block.json attributes; extract JSX structure, emit equivalent PHP `<div>` / `<span>` tree; map React event handlers to data attributes (no JS interactivity for primitives).
- CSS Module → style.css: rewrite `.className` → `.wp-block-dailyos-<name>` selectors; preserve token references.

For complex shapes the translator emits a "needs human review" comment block in render.php; output is a 90% scaffold, not a 100% drop-in.

Tested on two reference primitives: Pill (simple) + AccountOverview (composite — verifies the existing v1.4.2 block can be re-generated from its TSX source).

## 6. Directional decisions resolved at L0

### 6.1 CLI scaffold IS the documentation

Per execution constraint C1, no separate "block authoring playbook" markdown doc ships. The kit's CLI tool's `--help` output, the templates' inline comments, and the integration test harness's macro/function docs are the documentation. This avoids the documentation-drifts-from-code problem the constraint exists to prevent.

### 6.2 Two template shapes only (simple + composite); no per-primitive presets

Wave 1 primitives (W2) all fit one of two shapes — single-payload (Pill, HealthBadge, etc.) or multi-block-composition (AccountOverview-style). Adding per-primitive presets is over-engineering for v1.4.3 scope. New shapes can land via packet amendment.

### 6.3 Integration test harness is Rust-side primary, PHP-side proxy

The harness lives in Rust (`integration_test_harness.rs`) and shells out to PHP for the renderer step. Rust owns the producer + projection invocations; PHP owns the render-block-with-filter call. This matches the v1.4.2 production architecture (Rust runtime, PHP renderer) and the test harness mirrors production.

### 6.4 No new substrate primitives

The kit consumes:
- Existing `#[ability(...)]` macro (substrate-side)
- Existing `CustomBlockSchema` registration (substrate-side)
- Existing `Composition`/`ProjectedComposition`/`ProjectedBlock` types
- Existing `project_composition_for_surface` route (post-Packet-B V1.1.1)
- Existing `render_block_with_filter` PHP infrastructure
- Existing `class-dailyos-runtime-client` PHP transport

Net-new: only `BlockIntegrationFixture` struct + `run_block_integration_fixture` function + `integration_test_block!` macro + CLI scripts + templates. All in `templates/` or `scripts/` directories — clear separation from production substrate.

### 6.5 Token-source precedence: theme.json is canonical runtime, design/tokens is canonical authoring

When the generator detects conflicts:
- For CSS custom-property values (colors, spacing, typography): the design/tokens directory wins (it's the design-system source of truth).
- For WP-specific runtime knobs (`spacingSizes`, `colorPalette` slugs): theme.json wins (it's the WP-runtime contract).

Generator errors loudly on conflicts that can't be resolved by precedence (e.g., same token name in both sources with different non-WP-runtime values).

### 6.6 CLI scaffold updates `class-dailyos-plugin.php` in place

The CLI inserts the new block's metadata-registration call into the existing enumeration. Alternative considered: scan the `blocks/` directory at PHP runtime instead of explicit registration. Rejected because the explicit registration is the v1.4.2 W4-F precedent and reviewers (CSO+code-reviewer) at W4-F preferred explicit-allowlist over directory-scan.

## 7. Acceptance criteria

1. **CLI scaffold produces a working block.** `pnpm dailyos:new-block test-block` creates `wp/dailyos/blocks/test-block/` with block.json, render.php, render-functions.php, style.css, edit.js, editor.css. Block registers in `class-dailyos-plugin.php`. WordPress block editor lists "Test Block" in the DailyOS category.
2. **Integration test harness catches contract mismatch.** A negative fixture where the producer emits `payload.text` but the projection rule binds `payload.body` → harness detects mismatch + emits a DOS-670-style diagnostic ("producer/projection contract mismatch: declared field `payload.body` not found in producer output").
3. **CLI scaffold registers block without manual editing.** `class-dailyos-plugin.php` enumeration is updated; no human edit required.
4. **Token-to-theme.json generator runs idempotently.** Two consecutive runs produce zero diff when token sources are unchanged.
5. **Translation utility produces working primitive.** Running the translator on `src/components/ui/Pill.tsx` produces a `wp/dailyos/blocks/pill/` that the integration test harness passes.
6. **Translation utility produces working composite.** Running the translator on the AccountOverview source generates a `wp/dailyos/blocks/account-overview-regenerated/` whose output HTML is byte-equal (modulo timestamps + IDs) to the live `dailyos/account-overview` block's HTML.
7. **Generated render-functions.php uses the Packet B §5.6 typed error switch verbatim.** No two render-functions.php files diverge in their error-handling structure; the kit template is the source.
8. **Integration test harness is invoked by CI on every block PR.** A new workflow file `block-kit-integration.yml` runs the harness against every changed `wp/dailyos/blocks/*` directory in a PR.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `c1_cli_rejects_invalid_block_name` | `pnpm dailyos:new-block FOO` (uppercase) exits 1 with validation error |
| 2 | `c1_cli_rejects_existing_block_name` | `pnpm dailyos:new-block account-overview` (existing) exits 1; doesn't overwrite |
| 3 | `c1_scaffold_block_registers` | After CLI run, `class-dailyos-plugin.php` enumeration contains the new block; the registration call is in the right block (per the file's section markers) |
| 4 | `c1_scaffold_block_renders_empty` | Generated block with no input renders the "no content to show" placeholder per the template's empty-state branch |
| 5 | `c1_integration_harness_detects_contract_mismatch` | Producer emits `payload.text`; projection rule binds `payload.body` → harness emits DOS-670-style diagnostic; exit code 1 |
| 6 | `c1_integration_harness_passes_account_overview` | Existing `dailyos/account-overview` block passes the harness (regression guard — proves the harness is calibrated correctly) |
| 7 | `c1_translator_simple_primitive` | Translator on `src/components/ui/Pill.tsx` produces a `wp/dailyos/blocks/pill-test/` that passes harness |
| 8 | `c1_translator_composite_byte_parity` | Translator on AccountOverview source produces output byte-equal (modulo timestamps + IDs) to existing live block |
| 9 | `c1_theme_json_idempotent` | Two consecutive `pnpm dailyos:generate-theme-json` runs produce zero diff (when token sources unchanged) |
| 10 | `c1_theme_json_conflict_errors` | When `tokens.css` and `theme.json` define same token with different non-WP-runtime values → generator errors with conflict report; exit code 1 |
| 11 | `c1_render_functions_typed_error_inherited` | Generated render-functions.php contains the Packet B §5.6 switch table verbatim (grep-gate on the 5 switch arms) |
| 12 | `c1_ci_workflow_present` | `.github/workflows/block-kit-integration.yml` exists; CI matrix includes all `wp/dailyos/blocks/*` directories |

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | Every new block in `wp/dailyos/blocks/` MUST have a corresponding integration fixture in `src-tauri/abilities-runtime/tests/block_integration_fixtures/<block-name>.rs` (or the kit's CI workflow rejects the PR). | New workflow `block-kit-integration.yml` enumerates `wp/dailyos/blocks/*` and asserts each has a fixture. |
| 2 | Render-functions.php template MUST contain the Packet B §5.6 switch table (verbatim — bytewise-comparable header + 5 switch arms). | grep gate on every `wp/dailyos/blocks/*/render-functions.php` for the switch header + each of the 5 arm patterns |
| 3 | Block.json scaffold MUST declare the 5 standard composition attributes (`composition_id`, `composition_version`, `block_id`, `cache_hint_token`, `block_instance_id`). | grep gate on every `wp/dailyos/blocks/*/block.json` for the 5 attribute keys |
| 4 | Token-to-theme.json generator MUST run cleanly on every PR touching `.docs/design/tokens/`, `src/styles/tokens.css`, or `wp/dailyos/theme/theme.json`. | CI step runs generator with `--check` flag (verifies idempotency); fails CI on diff |
| 5 | CLI scaffold MUST exit 0 on a valid block name + exit 1 on invalid names. | CI runs scaffold with fixture inputs and asserts exit codes |
| 6 | Translator MUST produce kit-passing output on the 2 reference primitives (Pill + AccountOverview). | CI runs translator + harness on each |

## 10. Interlocks

**W0 stabilization (Packets A + B) is a soft dependency.** The §5.6 typed-error switch from Packet B is consumed verbatim by the render-functions.php template. The kit can be authored against the V1.1.1 spec without waiting for the merge, but the kit's PR must rebase on top of the Packet B merge before landing. Cross-packet rebase cost is low — disjoint code regions (Packet B touches `wp/dailyos/blocks/account-overview/*`; kit touches `wp/dailyos/scripts/templates/*` + new infrastructure files).

**Landing shape (V1.0):** single v1.4.3 W1 PR with 4 commit groups:
1. **Templates + CLI scaffold** (`wp/dailyos/scripts/new-block.mjs` + `wp/dailyos/scripts/templates/{simple,composite}/`).
2. **Integration test harness** (Rust: `integration_test_harness.rs` + macro; PHP: `StarterKitIntegrationTest.php`).
3. **Token + translator utilities** (`generate-theme-json.mjs` + `translate-tauri-to-block.mjs`).
4. **CI workflow** (`block-kit-integration.yml`) + AC #11 fixtures + AC #12 manifest.

Splittable into 2 PRs only if review size demands: PR-C1 = groups 1+2 (the kit core); PR-C2 = groups 3+4 (translator + theme generator + CI). Do NOT split groups 1+2 — they reference each other.

**Cross-version interlock:** W2 (Wave 1 primitives) cannot start until W1 merges. v1.4.4 surface migration cannot start until W1 + W2 merge. The kit's stability is the gate for every subsequent block author.

## 11. What this packet explicitly does NOT own

- **W0 stabilization (DOS-671..675).** Separate L0 packets (A + B).
- **W2 Wave 1 primitive blocks.** Distinct work track — uses this kit.
- **W3 magazine theme.** Distinct work track — uses the token generator from this kit but ships the actual theme.json + templates.
- **W4 feedback write infrastructure.** Distinct work track.
- **W5 Studio sandbox compatibility (C3).** Distinct work track.
- **DocBlocks for block authors beyond the CLI `--help` output + template inline comments.** Per execution constraint C1: the kit IS the documentation.
- **Code-mod tooling for existing blocks.** Translator generates new blocks from Tauri sources; it does not retroactively rewrite existing blocks.
- **WP block-level JavaScript interactivity beyond the editor reload guard.** Primitives are render-only.
- **Variant generation (e.g., "make 4 Pill subtypes").** W2 owns; the kit produces one block per CLI invocation.
- **Block deletion / rename tooling.** Out of scope; manual workflow for v1.4.3.
- **Auto-promotion from proposed to integrated design-system status.** Pattern-doc + design-reviewer pass; not the kit's responsibility.

## 12. Open questions for L0 reviewers

1. **(For codex challenge):** The translator's "90% scaffold" output for complex shapes — what's the realistic boundary? Are there primitives in `.docs/design/primitives/` that the translator definitively cannot handle in v1.4.3 scope (e.g., primitives with JS-interactive editor states like InlineInput)? If yes, document as out-of-scope for translator (manual scaffold from simple template).
2. **(For codex consult):** Integration test harness shells out to PHP. Is the existing CI Rust+PHP toolchain configured to support this (PHP available in the Rust test container)? If not, the harness needs a Tauri-binary entrypoint instead.
3. **(For code-reviewer):** Does the existing `register_blocks_from_metadata` enumeration at `class-dailyos-plugin.php:154-170` have section markers / comments the CLI can use as insertion anchors? Or does the CLI need to parse the PHP file (more brittle)?
4. **(For DX review):** Should the CLI prompt interactively (`inquirer`-style) or take all params as flags? Interactive is friendlier; flags are scriptable. Recommendation?
5. **(For CSO advisory):** The CLI inserts new code into `class-dailyos-plugin.php`. Are there any trust-boundary concerns with an automated-code-insertion tool modifying the plugin's core registration file? (Expected answer: no, because the tool runs locally as the developer's own UID and the inserted code is template-bounded.)

## 13. Linear dependency edges

- v1.4.3 W1 PR closes DOS-678.
- No upstream Linear dependencies — substrate primitives all exist from v1.4.0–v1.4.2.
- Soft dependency on Packet B V1.1.1 merge (consumes §5.6 typed-error switch verbatim — see §10 Interlocks).
- Downstream: every v1.4.3+ block author plugs into this kit. W2 (Wave 1 primitives) is the first consumer.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | Why |
|---|---|---|
| `/codex challenge` | adversarial | Stress-test the integration test harness's failure-mode coverage. Can it really catch a DOS-670-style contract mismatch? What about subtler mismatches (field type drift, optional vs required fields, payload structure changes)? Stress-test the translator's reliability on edge cases. |
| `/codex consult` | implementation feasibility | Walk the CLI scaffold through every step against the existing repo state. Verify `register_blocks_from_metadata` insertion anchors. Verify the Rust↔PHP shell-out works in CI. Verify the translator can handle the existing primitives' actual file shapes. |
| `code-reviewer` (claude) | domain | Independent claude review of template + harness + CLI code quality. Service-layer discipline (no direct DB from CLI), error-handling completeness, doc coverage on public APIs. |
| `DX review` | developer experience | Are the templates ergonomic? Is the CLI's `--help` output the documentation it claims to be? Can a new contributor produce a working block in <5 minutes per AC #1? |
| `/cso` | advisory only | No new trust boundaries. Confirm no escalation vector via automated code insertion into plugin core. |

**Convergence rule:** unanimous APPROVE required before code lands. Any reviewer returning CONDITIONAL APPROVE → fold finding into V1.1 (or Linear maintenance backlog if non-AC) and re-run reviewers. Cycle cap: 10 cycles before L6 escalation (per James's overnight authorization).

## 15. Acceptance for L0 closure

- [ ] All 5 reviewers returned non-BLOCK on the latest cycle.
- [ ] All 8 acceptance criteria (§7) are testable; per-criterion fixture mapped to §8 (12 fixtures).
- [ ] All 6 CI invariants (§9) have concrete enforcement (grep/CI workflow/runtime check).
- [ ] All §12 open questions resolved.
- [ ] Landing shape (§10) confirmed: single PR with 4 commit groups, or 2-PR split per the spec.
- [ ] Linear DOS-678 references this packet in its description.
- [ ] No outstanding L0-cycle findings; packet is implementation-ready.

When all seven boxes check, L0 is closed and implementation begins. L1 (self) proof bundle includes: CLI scaffold invocation log producing a working block; integration harness output for the 12 fixtures; translator parity verification log; CI workflow dry-run; macOS hands-on confirming the scaffolded block renders in WP editor.
