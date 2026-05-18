# L0 Packet C — C1 Starter Kit

**Current revision: V1.1 (cycle-1 fold + critical-rewrite, 2026-05-18). See §2 Changelog.**

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

- **V1.1 (2026-05-18, cycle-1 critical-rewrite):** 4 of 5 cycle-1 reviewers
  returned non-BLOCK (CSO CA, DX CA, code-reviewer CA, codex challenge BLOCK).
  Codex consult R1 died silently; re-dispatched for R2 against V1.1.

  Codex challenge BLOCK + code-reviewer's 3 mis-citations + CSO's factual
  amendment triangulated on **three load-bearing substrate-reuse errors** in
  V1.0 that would have derailed implementation. Folded inline:

  - **§5.1 + §5.4 drop the "CLI modifies class-dailyos-plugin.php" scope.**
    CSO + code-reviewer + codex challenge C2 all confirm: the existing
    `register_blocks()` at `wp/dailyos/includes/class-dailyos-plugin.php:149-163`
    is a `glob('blocks/*/block.json')` directory scan. There is no enumeration
    to insert into, and the CLI shouldn't fight glob registration. **V1.1
    simplifies dramatically: CLI just drops the block.json + supporting files
    into `wp/dailyos/blocks/<name>/`; glob auto-registers on next request.**
    Removes the largest automated-mutation surface from the kit.
  - **§5.4 projection rule template completely rewritten** (codex challenge C1).
    V1.0 templated `register_custom_block_schema(CustomBlockSchema::new(...))`
    with `BindingRole::Title/Text` — but `register_custom_block_schema` is
    only called in tests (`tests/dos570_fallback_projection.rs:101`), and the
    `BindingRole` enum doesn't have `Title`/`Text` variants (real:
    `Source | ComputedFrom | DisplayOnly | FeedbackTarget` at
    `composition.rs:175`). `FieldPath::new` also requires `/`-prefixed JSON
    pointers (`provenance/field.rs:20`), so `payload.text` is invalid syntax.
    Production projection dispatches through the `BlockType` enum
    (`composition.rs:330+`) and `known_projection_rules()` Vec at
    `fallback_projection.rs:1236-1252,1415-1423`. V1.1 either (a) templates
    against the real flow (BlockType + known_projection_rules + concrete
    rule constructor like `account_overview_rule()`) OR (b) promotes
    `register_custom_block_schema` from test-only to production as net-new
    substrate work in scope. V1.1 picks (a) — extends `BlockType` enum +
    `known_projection_rules()` per new block, no net-new substrate.
  - **§5.5 harness fixture schema-rewritten** (codex challenge C3). V1.0
    fixture was substring-based (`expected_projection_keys` +
    `expected_renderer_html_contains`) which can catch gross missing keys
    but NOT the subtle DOS-670 drift class (value-kind mismatch, optional vs
    required drift, nesting drift, payload.text→payload.body, etc).
    V1.1 fixture is schema-based: JSON pointer + `ValueKind` + `required` vs
    `optional` per binding + expected diagnostics + expected renderer branch
    + exact block wrapper/data attributes. Adds explicit negative-fixture
    classes for field rename, type drift, required-to-optional drift,
    optional-to-required drift, payload nesting drift. DOS-670-style
    diagnostic gets a 4-field shape: location, declared, actual, did-you-mean
    (via edit-distance) — per DX F4.
  - **§5.6 token source paths corrected** (code-reviewer + codex challenge
    H3). V1.0 cited `src/styles/tokens.css` — actual file is
    `src/styles/design-tokens.css` (`.docs/design/tokens/color.md:137`).
    V1.0 cited `wp/dailyos/theme/theme.json` as canonical source — that
    file doesn't exist; theme.json is W3-owned generated OUTPUT. V1.1
    treats theme.json as generated output (kit's generator produces it,
    W3 ships the finalized version), not input. Token graph normalization
    added so semantic aliases like `--color-account → --color-spice-turmeric`
    don't false-conflict.
  - **§5.7 translator scope matrix replaces "90% scaffold" prose** (codex
    challenge H1 + DX F5). V1.0's "90% scaffold" was hand-wavy and overclaimed.
    V1.1 enumerates a 4-row support matrix:
    - **Supported (static render-only, TSX + CSS Module):** Pill, HealthBadge, StatusDot, Avatar, IntelligenceQualityBadge, FreshnessIndicator, ProvenanceTag, TrustBandBadge (post-promotion)
    - **Supported with promotion (proposed → integrated source-only):** TrustBandBadge (currently proposed)
    - **Supported with inline-style adaptation:** FolioRefreshButton (no CSS module today — translator extracts inline styles)
    - **NOT supported (interactive — manual template):** InlineInput, EditableText, Switch, Segmented, RemovableChip, EntityChip editable variants, TypeBadge editable mode
    Translator exits 1 with actionable diagnostic on unsupported input (NOT a partial scaffold a dev might trust).
  - **§5.2 add 3rd template shape "typed-display"** (codex challenge H2 +
    DX F3). V1.0 simple+composite isn't enough for Wave 1 primitives with
    multiple typed attrs (HealthBadge score/band/size/insufficient-data,
    Avatar photo-url/cache/initials/size, FreshnessIndicator timestamp/staleness,
    EntityChip type/name/icon, TypeBadge account-type). V1.1 adds typed-display:
    multiple typed attrs, variant enum validation, optional asset URL,
    deterministic PHP formatter, harness assertions per-attr type/requiredness.
  - **§5.1 CLI hybrid pattern** (DX F1). V1.1 spec: flag-only when all flags
    provided (scriptable for codex agents); interactive fallback via
    `prompts` or `enquirer` (NOT `inquirer` v9 which is ESM-only and breaks
    Tauri build scripts) when invoked bare. Splits `--ability <name>` (link
    existing) from `--new-ability <name>` (scaffold producer).
  - **§5.3 producer template adds a worked `prepare_` body** (DX F3). V1.0
    template scaffolded around `prepare_*` but punted on what it actually
    does — the hard part of producer authoring. V1.1 ships a complete
    simple-shape prepare body so bare scaffold passes the harness without
    further edits.
  - **§6.1 allow ONE templates/README.md ≤50 lines** (DX F2). V1.0's
    "CLI --help-as-documentation" absolutism fails first-time author path.
    V1.1 allows a single ≤50-line `wp/dailyos/scripts/templates/README.md`
    with structural pointers (invocation + reference block + harness fixture
    example), CI-gated for path resolution. Anti-drift principle intact;
    the launch checklist isn't drift-prone.
  - **§6.6 REMOVED.** V1.0 §6.6 justified "explicit allowlist over directory
    scan" citing a fictional W4-F precedent. The actual existing pattern IS
    glob; V1.1 keeps glob. Section removed entirely; CLI no longer modifies
    plugin core.
  - **§5.5 harness PHP shell-out generic-not-account-specific** (codex
    challenge M1). V1.0 said harness reuses existing `render_block_with_filter`
    — that helper is account-overview-specific (`class-dailyos-plugin.php:675-690`).
    V1.1 spec: a generic PHP test entrypoint that registers target block
    metadata, injects fake runtime client, calls WordPress
    `render_block`/metadata render callback. (Path-α maintenance: extract
    account-overview's helper into public test helper.)
  - **§5.5 + §9 invariant #2 harden against template drift** (CSO L-4):
    require `esc_html` / `esc_attr` / `wp_kses_post` wrapping in translator
    output; add `php -l` validation + grep gate to invariant set; PHP harness
    input contract restricted to (projection-path under fixtures/, allowlisted
    ability name, sibling expected.json); no eval/include/exec of argv-derived
    paths.
  - **§5.6 token generator atomicity** (CSO L-2): tmpfile+rename atomic
    write; validate JSON before swap; rollback on validation failure.
  - **§10 landing shape restructured** (codex challenge H4). V1.0 split
    delayed the CI gate that C1 says is load-bearing. V1.1 split (if needed):
    harness + CI workflow + first integration fixture vs `account-overview`
    LAND TOGETHER FIRST → then CLI + templates → then translator + theme
    generator (separately, different dep graphs per codex challenge M3).
  - **§7 AC #1 tagged L4-hands-on** (code-reviewer #6). The "produces a
    working block in <5 minutes" criterion is hands-on, not unit-testable.
    V1.1 marks it explicitly for L4 batch validation.
  - **§7 AC #6 redefined** (code-reviewer #4). V1.0 "byte-equal (modulo
    timestamps + IDs)" was squishy. V1.1: replace AccountOverview parity
    target — there's no Tauri `AccountOverview.tsx` source to feed the
    translator (codex challenge H5). New AC #6: translator + harness pass
    on a 2-primitive fixture set: Pill (simple-shape) + HealthBadge
    (typed-display shape).
  - **§11 block-name conflict policy added** (code-reviewer #5): CLI
    enumerates existing `wp/dailyos/blocks/*/block.json` slugs + errors out
    if `<name>` collides. Documents the conflict-resolution decision tree.
  - **§11 BlockType enum extension story added** (code-reviewer #5):
    every new block requires a corresponding `BlockType` variant addition
    + `<name>_rule()` constructor + `known_projection_rules()` registration.
    CLI generates the Rust scaffold stub; developer adds the actual
    rule logic. Tagged in AC #1b.
  - **§1.4.3-waves.md:75 token source naming fix** (code-reviewer + codex
    challenge H3): change `src/styles/tokens.css` → `src/styles/design-tokens.css`
    in the wave plan invariants table.
  - **DEFERRED to v1.x maintenance backlog** (DOS-684 already filed):
    - Repo-wide token source-name cleanup pass
    - Design inventory render-only-vs-interactive flag (interaction column)
    - Extract account-overview's private renderer into public test helper

  V1.1 LINE COUNT EXPECTED: ~700 lines (was 370). Substantial growth reflects
  the rewritten §5.4 + §5.5 + §5.7 with the actual substrate APIs + concrete
  schema-based contracts. The architectural shape (extract W4-F into kit
  templates + integration harness + token/translator utilities) is UNCHANGED;
  V1.1 corrects the substrate-reuse citations + concrete API signatures + scope
  boundaries to match repo ground truth.

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

A Node.js CLI invoked as `pnpm dailyos:new-block <block-name> [--template simple|typed-display|composite] [--ability <existing-ability-name>] [--new-ability <new-ability-name>]`. Defaults: `--template simple`, no ability flag (block consumes existing producer or is a pure-render placeholder).

**Hybrid invocation pattern (V1.1 per DX F1):** flag-only when all required flags
provided (scriptable for codex agents); interactive fallback via `prompts` or
`enquirer` (NOT `inquirer` v9 which is ESM-only and breaks Tauri build scripts)
when invoked bare.

Responsibilities:
1. Validate `<block-name>` matches `[a-z][a-z0-9-]+` (Gutenberg block name slug rules).
2. Enumerate existing `wp/dailyos/blocks/*/block.json` slugs; error out if `<block-name>` collides (V1.1 block-name conflict policy per code-reviewer #5).
3. Copy templates from `wp/dailyos/scripts/templates/<template>/` into `wp/dailyos/blocks/<block-name>/` with name interpolation (`{{BLOCK_NAME}}`, `{{ABILITY_NAME}}`, `{{PHP_FUNCTION_PREFIX}}`, `{{BlockType}}`).
4. **NO modification of `wp/dailyos/includes/class-dailyos-plugin.php`** (V1.1 critical correction per CSO + code-reviewer + codex challenge triangulation). The existing `register_blocks()` at `class-dailyos-plugin.php:149-163` uses `glob('blocks/*/block.json')` — dropping a new `block.json` into the right directory is sufficient; glob picks it up automatically on next request.
5. If `--new-ability` is provided: copy producer template into `src-tauri/abilities-runtime/src/abilities/<ability-name>.rs`, generate the BlockType enum variant + `<name>_rule()` constructor + `known_projection_rules()` registration stub (Rust scaffold the developer fills in per AC #1b). If `--ability` (link existing) is provided: validate the named ability exists in the registry; do NOT touch ability/registration files.
6. Print the next steps: edit X (TODO regions in producer + projection rule), run `pnpm dailyos:test-block <name>` to invoke the kit's integration fixture, run `pnpm dev` to view in WordPress editor.

CLI errors out on existing block-name collision (no overwrite). Exit codes: 0 success, 1 validation error (bad name, collision, missing dep), 2 partial state (template copy + ability scaffold succeeded; registration stub failed → emits an exact cleanup manifest per L3 fix). `--keep-partial` overrides the cleanup-on-failure behavior for debugging.

### 5.2 Block scaffold templates (V1.2 — three shapes)

`wp/dailyos/scripts/templates/{simple,typed-display,composite}/` — three template sets:

**typed-display shape (added V1.1 per codex challenge H2 + DX F3):** for Wave 1 primitives with multiple typed attrs (HealthBadge score/band/size/insufficient-data, Avatar photo-url/cache/initials/size, FreshnessIndicator timestamp/staleness, EntityChip type/name/icon, TypeBadge account-type). Per-attr type validation via block.json attribute schemas; deterministic PHP formatter; harness assertions for each attr's `ValueKind` + required-vs-optional.

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

### 5.4 Projection rule template (rewritten V1.1 — real API)

V1.0 templated `register_custom_block_schema(...).with_field_binding(BindingRole::Title, ...)` — **but that API doesn't exist in production** (codex challenge C1):
- `register_custom_block_schema` is test-only (`tests/dos570_fallback_projection.rs:101`).
- `CustomBlockSchema` exposes `type_id`, `composition_kind`, `required_pointers`, `optional_pointers`, `render_annotations` — no `with_field_binding` (`fallback_projection.rs:211`).
- The real `BindingRole` enum is `Source | ComputedFrom | DisplayOnly | FeedbackTarget` (`composition.rs:175`) — no `Title`/`Text` variants.
- `FieldPath::new` requires `/`-prefixed JSON pointers (`provenance/field.rs:20`) — `payload.text` is invalid syntax.

**V1.2 templates against the real flow (V1.1 attempt had wrong type-signatures):**
- `BlockProjectionRule` is private at `fallback_projection.rs:255` (cannot be returned from a separate module)
- `known_projection_rules()` is private at `fallback_projection.rs:1250`
- In-file pattern: each block rule lives as a private in-file `fn <name>_rule() -> BlockProjectionRule` (`account_overview_rule` at `:1415`)

**V1.2 approach: CLI generates code snippets the developer pastes IN-FILE.** No new template module file (would need pub-exposing private types). The kit's CLI prints exact patch instructions; the developer applies them with one paste each. No automated file edits to Rust source — that's the boundary the kit respects (CLI does NOT modify any `.rs` file beyond creating the new ability scaffold).

**CLI output for `pnpm dailyos:new-block <name> --new-ability <ability-name>`:**

```
Block scaffold created at wp/dailyos/blocks/<name>/
Producer ability scaffold created at src-tauri/abilities-runtime/src/abilities/<ability-name>.rs

Manual steps required (paste these into the named files):

[1] Add to BlockType enum at src-tauri/abilities-runtime/src/abilities/composition.rs:175:

    #[serde(rename = "dailyos/<ability-name-kebab>")]
    {{BlockType}},

[2] Add the rule fn to src-tauri/abilities-runtime/src/abilities/fallback_projection.rs
    (paste below account_overview_rule at line ~1415):

    fn {{ability_name}}_rule() -> BlockProjectionRule {
        BlockProjectionRule {
            block_type: BlockType::{{BlockType}},
            required_pointers: vec![FieldPath::new("/payload/text")],
            optional_pointers: vec![FieldPath::new("/payload/subtitle")],
            render_annotations: Vec::new(),
            // TODO: extract per-binding projector fns here (mirror
            // account_overview_rule structure)
        }
    }

[3] Add to known_projection_rules() at fallback_projection.rs:1250 (inside the Vec literal):

    {{ability_name}}_rule(),

[4] Run the kit integration fixture:
    pnpm dailyos:test-block <name>

The harness fails fast if the producer's payload shape doesn't match the
required/optional pointers in your rule — that's the DOS-670 catch.
```

**Rationale for the paste-not-edit approach (V1.2 vs V1.1):** Automated Rust file editing is brittle (formatting, import collisions, comment markers). The CLI's job is generating correct snippets; the developer's job is applying them. This is consistent with §5.1 V1.1's "CLI does NOT modify `class-dailyos-plugin.php`" decision — the kit respects file boundaries and emits snippets-for-paste.

**Note on the alternative substrate-extension path:** Making `BlockProjectionRule` + `known_projection_rules()` `pub` + adding a registration helper would let the kit templatize as a separate file. V1.2 rejects this as scope expansion — the in-file paste pattern works against the existing API surface with zero substrate change. The pub-exposure work belongs in v1.x maintenance if/when a real consumer emerges (file as DOS-685 follow-up if codex consult requests it in cycle 3).

### 5.5 Shared integration test harness (`src-tauri/abilities-runtime/src/abilities/integration_test_harness.rs`)

**Load-bearing primitive.** Exercises producer → projection → renderer end-to-end against a substrate test DB. Modeled on the v1.4.2 W4-F retrofit that DOS-670 forced.

V1.0 fixture was substring-based (`expected_projection_keys` +
`expected_renderer_html_contains`) — codex challenge C3 found this WON'T catch
the subtle DOS-670 drift class (value-kind mismatch, optional vs required
drift, payload nesting drift like `payload.text` → `payload.body`).

**V1.1 schema-based fixture shape:**

```rust
pub struct BlockIntegrationFixture {
    pub ability_name: String,           // e.g. "dailyos/account-overview"
    pub composition_id: String,         // synthetic, generated per test
    pub input_json: serde_json::Value,  // producer input

    /// Schema-based assertions per binding (NOT substring-based).
    /// Each entry: JSON pointer (/-prefixed) + ValueKind + required-vs-optional.
    pub expected_bindings: Vec<BindingExpectation>,

    /// Expected diagnostics (e.g., warnings the projection rule should emit).
    pub expected_diagnostics: Vec<ProjectionDiagnostic>,

    /// Renderer branch coverage assertions (which switch arms must fire).
    pub expected_renderer_branches: Vec<RendererBranchAssertion>,

    /// Exact block wrapper element + data attributes the PHP renderer outputs.
    pub expected_wrapper: BlockWrapperAssertion,
}

pub struct BindingExpectation {
    pub pointer: String,           // e.g. "/payload/text" — /-prefixed JSON pointer
    pub value_kind: ValueKind,     // String | Number | Bool | Array | Object | Null
    pub required: bool,            // missing required → harness fails with DOS-670 diagnostic
}

pub struct RendererBranchAssertion {
    pub branch_label: String,      // e.g. "single-claim-text"
    pub expected_html_pattern: String, // exact pattern, anchored
}

/// Runs the full producer → projection → renderer pipeline.
/// On contract mismatch, panics with a 4-field DOS-670-style diagnostic
/// (per DX F4): { location, declared, actual, did_you_mean (edit-distance) }.
pub async fn run_block_integration_fixture(fixture: BlockIntegrationFixture) -> RenderedHtml;

/// Convenience macro for new-block authors:
///
/// ```rust
/// integration_test_block!(
///     my_block,
///     BlockIntegrationFixture {
///         ability_name: "dailyos/my-block".into(),
///         composition_id: "test:cmp:1".into(),
///         input_json: json!({ "schema_version": 1, "key": "value" }),
///         expected_bindings: vec![
///             BindingExpectation { pointer: "/payload/text".into(), value_kind: ValueKind::String, required: true },
///             BindingExpectation { pointer: "/payload/subtitle".into(), value_kind: ValueKind::String, required: false },
///         ],
///         expected_diagnostics: vec![],
///         expected_renderer_branches: vec![RendererBranchAssertion {
///             branch_label: "single-claim-text".into(),
///             expected_html_pattern: r#"<article class="dailyos-block dailyos-block-my-block">"#.into(),
///         }],
///         expected_wrapper: BlockWrapperAssertion { tag: "section", class: "wp-block-dailyos-my-block", data_attrs: vec![...] },
///     }
/// );
/// ```
#[macro_export]
macro_rules! integration_test_block { /* generates a #[tokio::test] fn */ }
```

**Negative fixture classes required (V1.1 per codex challenge C3):**
- Field rename: producer emits `payload.text`; rule binds `payload.body` → harness fails with DOS-670 diagnostic (`location: "/payload/body"`, `declared: required-string`, `actual: missing`, `did_you_mean: "/payload/text" (edit_distance=2)`)
- Type drift: producer emits `payload.count: "5"` (string); rule binds `Number` → harness fails with `location: "/payload/count"`, `declared: Number`, `actual: String`, `did_you_mean: null`
- Required-to-optional drift: existing fixture says `required: true`; producer made it optional → harness fails on shape change
- Optional-to-required drift: existing fixture says `required: false`; producer made it required without notice → harness fails on contract narrowing
- Payload nesting drift: producer emits `payload.summary.text`; rule binds `payload.text` → harness diagnostic flags the nesting collapse

**Generic PHP shell-out path (V1.1 per codex challenge M1):**
`wp/dailyos/tests/blocks/StarterKitIntegrationTest.php` is a generic block-render entrypoint (NOT the account-overview-specific `render_block_with_filter` at `class-dailyos-plugin.php:675-690`). It:
1. Registers the target block's `block.json` metadata at test-setup time.
2. Injects a fake runtime client that returns the provided projection JSON.
3. Calls WordPress `render_block()` / metadata-driven render callback for the requested block name.
4. Asserts wrapper element shape + data attributes + renderer-branch presence + expected-binding values.
5. **Input contract** (V1.1 per CSO L-3): only accepts (projection-JSON-path under `tests/fixtures/blocks/`, allowlisted ability name from `BlockType` enum, sibling `expected.json` schema file). No `eval()` / `include()` / `exec()` of argv-derived paths.

Rust harness shells out via `std::process::Command` with PHP CLI; PHP exit-codes 0 on success, 1 on contract violation (with the 4-field diagnostic on stderr).

### 5.6 Token-to-theme.json generator (V1.1 — corrected paths)

V1.0 cited `src/styles/tokens.css` and treated `wp/dailyos/theme/theme.json` as input — **both wrong** (code-reviewer + codex challenge H3):
- Actual canonical CSS source: `src/styles/design-tokens.css` (`.docs/design/tokens/color.md:137`)
- `wp/dailyos/theme/theme.json` doesn't exist yet — it's W3-owned generated **output**, not input.

**V1.1 canonical sources (input):**
- `.docs/design/tokens/` (design-system contract — canonical authoring source for semantic aliases)
- `src/styles/design-tokens.css` (runtime CSS source — declares CSS custom properties)

**V1.1 generated output:**
- `wp/dailyos/theme/theme.json` — kit's generator produces this; W3 magazine theme ships the finalized version. The kit's output IS the seed for W3.

**Token graph normalization (V1.1 per codex challenge H3):**
Token docs define semantic aliases like `--color-account` → `--color-spice-turmeric` (`.docs/design/tokens/color.md:70`); CSS stores `--color-account: var(--color-spice-turmeric)` (`design-tokens.css:86`). Generator builds a token graph, resolves aliases to terminal values, then compares for conflicts — prevents false-conflict on alias re-declarations.

**Precedence rules (V1.1 §6.5 reconciled):**
- For CSS custom-property terminal values: `.docs/design/tokens/` wins (design system is the contract).
- For WP-runtime knobs (slug names in `settings.color.palette`, `settings.spacing.spacingSizes`): `theme.json` (when it exists post-generation) is the runtime contract — but during initial generation, the kit derives these directly from `design-tokens.css` + the design-tokens docs.
- Generator errors loudly on same-token-name-different-value conflicts that can't be resolved by alias normalization.

**Atomicity (V1.1 per CSO L-2):** generator writes to `theme.json.tmp`, validates the result is valid JSON + parses to a `theme.json` schema, then atomic-renames to `theme.json`. On validation failure: delete tmpfile, error out, leave existing `theme.json` (if any) untouched.

**Idempotency:** re-running with no token-source changes produces zero diff. CI gate runs the generator with `--check` flag (verifies idempotency); fails CI on diff.

### 5.7 Translation utility (V1.2 — scope matrix)

V1.0/V1.1 "90% scaffold" was overclaimed (codex challenge H1). V1.2 enumerates a hard support matrix:

| Category | Inputs | Output | Primitives covered |
|---|---|---|---|
| **Supported (static render-only, TSX + CSS Module)** | `*.tsx` + `*.module.css` | Full scaffold: `block.json`, `render.php`, `render-functions.php`, `style.css`, `edit.js` | Pill, HealthBadge, StatusDot, Avatar, IntelligenceQualityBadge, FreshnessIndicator, ProvenanceTag |
| **Supported with source promotion (proposed → integrated)** | `*.tsx` (TBD source) + design-system spec | Full scaffold + reminder to promote source-only entry to integrated | TrustBandBadge |
| **Supported with inline-style adaptation** | `*.tsx` + inline-style extraction | Full scaffold, inline styles extracted to `style.css` | FolioRefreshButton |
| **NOT supported (interactive — manual template)** | — | Translator exits 1 with diagnostic | InlineInput, EditableText, Switch, Segmented, RemovableChip, EntityChip editable variants, TypeBadge editable mode |

For NOT-supported primitives, translator emits actionable diagnostic:
```
error: <PrimitiveName> requires interactive event handlers (onChange/onClick).
       Use `pnpm dailyos:new-block --template <simple|typed-display> <name>` to scaffold manually.
       See .docs/design/primitives/<PrimitiveName>.md for the interactive contract this primitive ships.
```

CLI mode: `pnpm dailyos:translate-tauri --primitive <PrimitiveName>` reads `.docs/design/primitives/README.md` to determine category, then attempts translation (or refuses with the above diagnostic).

**Translator parity targets (replaces V1.0 AC #6 AccountOverview claim per codex challenge H5):** Pill (simple shape) + HealthBadge (typed-display shape). Both have shipped Tauri React sources (`src/components/ui/Pill.tsx`, `src/components/shared/HealthBadge.tsx`). AccountOverview is NOT a translator target — there is no Tauri `AccountOverview.tsx` source; the existing WP `dailyos/account-overview` block is hand-authored.

## 6. Directional decisions resolved at L0

### 6.1 CLI scaffold IS the documentation

Per execution constraint C1, no separate "block authoring playbook" markdown doc ships. The kit's CLI tool's `--help` output, the templates' inline comments, and the integration test harness's macro/function docs are the documentation. This avoids the documentation-drifts-from-code problem the constraint exists to prevent.

### 6.2 Three template shapes (V1.2: added typed-display per codex challenge H2)

Wave 1 primitives fit one of three shapes: single-payload (`simple` — Pill, StatusDot, ProvenanceTag), multi-typed-attrs (`typed-display` — HealthBadge, Avatar, FreshnessIndicator, EntityChip, TypeBadge, IntelligenceQualityBadge), or multi-block composition (`composite` — AccountOverview-style). Per-primitive presets remain out of scope. New shapes can land via packet amendment.

### 6.3 Integration test harness is Rust-side primary, PHP-side proxy

The harness lives in Rust (`integration_test_harness.rs`) and shells out to PHP for the renderer step. Rust owns the producer + projection invocations; PHP owns the render-block-with-filter call. This matches the v1.4.2 production architecture (Rust runtime, PHP renderer) and the test harness mirrors production.

### 6.4 No new substrate primitives (V1.2 — corrected reuse list)

The kit consumes:
- Existing `#[ability(...)]` macro (substrate-side)
- Existing `BlockType` enum + `known_projection_rules()` + private `BlockProjectionRule` (production projection flow — V1.2 paste pattern; NOT the test-only `register_custom_block_schema`)
- Existing `Composition`/`ProjectedComposition`/`ProjectedBlock` types
- Existing `project_composition_for_surface` route (post-Packet-B V1.1.1)
- Existing `register_blocks()` glob at `class-dailyos-plugin.php:149-163` (CLI drops `block.json`; glob picks it up — V1.2 simplification)
- Existing `class-dailyos-runtime-client` PHP transport
- Generic `render_block()` test entrypoint (V1.2 — NOT the account-overview-specific `render_block_with_filter` at `class-dailyos-plugin.php:675-690`; that's filed as DOS-684 path-α for extraction)

Net-new: only `BlockIntegrationFixture` struct + `BindingExpectation`/`ProjectionDiagnostic`/`RendererBranchAssertion`/`BlockWrapperAssertion` value types + `run_block_integration_fixture` function + `integration_test_block!` macro + CLI scripts + 3 template shapes + token graph normalizer + scope-matrix translator. All in `templates/` or `scripts/` directories or as test-only Rust modules — clear separation from production substrate.

### 6.5 Token-source precedence: theme.json is canonical runtime, design/tokens is canonical authoring

When the generator detects conflicts:
- For CSS custom-property values (colors, spacing, typography): the design/tokens directory wins (it's the design-system source of truth).
- For WP-specific runtime knobs (`spacingSizes`, `colorPalette` slugs): theme.json wins (it's the WP-runtime contract).

Generator errors loudly on conflicts that can't be resolved by precedence (e.g., same token name in both sources with different non-WP-runtime values).

### 6.6 (REMOVED in V1.1)

V1.0 §6.6 justified "explicit allowlist over directory scan" citing a fictional W4-F precedent. The actual existing pattern at `class-dailyos-plugin.php:149-163` is `glob('blocks/*/block.json')` (directory scan). V1.1 keeps glob — CLI no longer modifies plugin core. See V1.1 §5.1 for the simplified flow.

## 7. Acceptance criteria

1. **CLI scaffold produces a working block.** `pnpm dailyos:new-block test-block` creates `wp/dailyos/blocks/test-block/` with block.json, render.php, render-functions.php, style.css, edit.js, editor.css. Block registers in `class-dailyos-plugin.php`. WordPress block editor lists "Test Block" in the DailyOS category.
2. **Integration test harness catches contract mismatch.** A negative fixture where the producer emits `payload.text` but the projection rule binds `payload.body` → harness detects mismatch + emits a DOS-670-style diagnostic ("producer/projection contract mismatch: declared field `payload.body` not found in producer output").
3. **CLI scaffold integrates with glob registration without modifying plugin core.** Block auto-registers via existing `glob('blocks/*/block.json')` at `class-dailyos-plugin.php:149-163`; no edits to `class-dailyos-plugin.php` required.
4. **Token-to-theme.json generator runs idempotently.** Two consecutive runs produce zero diff when token sources are unchanged.
5. **Translation utility produces working simple-shape primitive.** Running the translator on `src/components/ui/Pill.tsx` produces a `wp/dailyos/blocks/pill/` that the integration test harness passes.
6. **Translation utility produces working typed-display primitive.** Running the translator on `src/components/shared/HealthBadge.tsx` produces a `wp/dailyos/blocks/health-badge/` that the integration test harness passes. (V1.2: replaces V1.0's AccountOverview parity target — there is no Tauri `AccountOverview.tsx` source to feed the translator.)
7. **Generated render-functions.php uses the Packet B §5.6 typed error switch verbatim.** No two render-functions.php files diverge in their error-handling structure; the kit template is the source.
8. **Integration test harness is invoked by CI on every block PR.** A new workflow file `block-kit-integration.yml` runs the harness against every changed `wp/dailyos/blocks/*` directory in a PR.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `c1_cli_rejects_invalid_block_name` | `pnpm dailyos:new-block FOO` (uppercase) exits 1 with validation error |
| 2 | `c1_cli_rejects_existing_block_name` | `pnpm dailyos:new-block account-overview` (existing) exits 1; doesn't overwrite |
| 3 | `c1_scaffold_block_auto_registers_via_glob` | After CLI run, the new block's `block.json` exists at `wp/dailyos/blocks/<name>/block.json`; a fresh `register_blocks()` invocation picks it up via the existing glob; `class-dailyos-plugin.php` is NOT modified (V1.2 correction) |
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
| 4 | Token-to-theme.json generator MUST run cleanly on every PR touching `.docs/design/tokens/`, `src/styles/design-tokens.css`, or `wp/dailyos/theme/theme.json`. | CI step runs generator with `--check` flag (verifies idempotency); fails CI on diff |
| 5 | CLI scaffold MUST exit 0 on a valid block name + exit 1 on invalid names. | CI runs scaffold with fixture inputs and asserts exit codes |
| 6 | Translator MUST produce kit-passing output on the 2 reference primitives (Pill simple-shape + HealthBadge typed-display shape). | CI runs translator + harness on each (V1.2: replaces V1.0's AccountOverview target which lacks a Tauri TSX source) |

## 10. Interlocks

**W0 stabilization (Packets A + B) is a soft dependency.** The §5.6 typed-error switch from Packet B is consumed verbatim by the render-functions.php template. The kit can be authored against the V1.1.1 spec without waiting for the merge, but the kit's PR must rebase on top of the Packet B merge before landing. Cross-packet rebase cost is low — disjoint code regions (Packet B touches `wp/dailyos/blocks/account-overview/*`; kit touches `wp/dailyos/scripts/templates/*` + new infrastructure files).

**Landing shape (V1.2 — restructured per codex challenge H4):** single v1.4.3 W1 PR with 4 commit groups in dependency order — **CI workflow ships in group 1 to enforce the C1 invariant from PR open**:
1. **Harness + CI workflow + first integration fixture** (Rust: `integration_test_harness.rs` + macro + `BindingExpectation`/etc value types; PHP: `StarterKitIntegrationTest.php`; CI: `.github/workflows/block-kit-integration.yml`). First fixture validates harness against existing `dailyos/account-overview` block — proves the harness works on a real production block before any new blocks land.
2. **Templates + CLI scaffold** (`wp/dailyos/scripts/new-block.mjs` + 3 template shapes at `wp/dailyos/scripts/templates/{simple,typed-display,composite}/`).
3. **Token generator** (`generate-theme-json.mjs` + token graph normalizer + atomic write).
4. **Translator with scope matrix** (`translate-tauri-to-block.mjs` + 2-primitive reference fixtures).

**Split policy if review size demands** (V1.2 per codex challenge M3 — translator and theme generator have different dep graphs):
- PR-C1 = groups 1+2 (the kit core: harness + CI + CLI + templates). MERGEABLE on its own — every claim about "no block ships without integration fixture" is enforced.
- PR-C2 = group 3 (token generator with token/theme tests + W3 consumer prep).
- PR-C3 = group 4 (translator + parity fixtures).
- Do NOT split groups 1+2.

CI workflow in group 1 is the load-bearing enforcement: from the first commit forward, any PR touching `wp/dailyos/blocks/*` must have a passing integration fixture or CI fails.

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
