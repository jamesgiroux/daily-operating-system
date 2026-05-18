# Verdict

APPROVE after cycle 4 targeted re-verify. Packet C V1.3 resolves the cycle-3 blockers: §5.4 now matches the real `BlockProjectionRule` shape and paste-point coverage, and §8 fixtures #8/#10 now target HealthBadge and token graph alias normalization.

# Critical

## C1. Projection rule template is not a valid extraction of the existing substrate API

- Location: packet §5.4, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:163`; packet snippet at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:167`; existing API at `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:210`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:219`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:286`, `src-tauri/abilities-runtime/src/abilities/composition.rs:173`, and `src-tauri/abilities-runtime/src/abilities/provenance/field.rs:20`.
- Observation: the packet's template calls `CustomBlockSchema::new(...).with_field_binding(BindingRole::Title, FieldPath::new("payload.title"))` and `BindingRole::Text` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:169`). The current `CustomBlockSchema` only exposes `type_id`, `composition_kind`, `required_pointers`, `optional_pointers`, and `render_annotations` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:211`); there is no `with_field_binding`. The current `BindingRole` enum is `Source | ComputedFrom | DisplayOnly | FeedbackTarget` (`src-tauri/abilities-runtime/src/abilities/composition.rs:175`). `FieldPath::new` rejects non-empty paths that do not start with `/` (`src-tauri/abilities-runtime/src/abilities/provenance/field.rs:20`), so `payload.text` is invalid path syntax.
- Recommendation: rewrite §5.4 around the real registry shape: set `required_pointers` / `optional_pointers` with JSON pointers such as `/title` and `/text`, add compile-fail/compile-pass tests for generated projection snippets, and remove nonexistent `BindingRole::Title/Text` language.

## C2. CLI scaffold targets a registration anchor that current PHP no longer has

- Location: packet §5.1, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:72`; packet §6.6, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:262`; existing PHP at `wp/dailyos/includes/class-dailyos-plugin.php:146`.
- Observation: the packet says the CLI updates `class-dailyos-plugin.php` by inserting into an explicit block-metadata enumeration (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:75`) and justifies in-place insertion as preserving explicit registration (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:264`). The actual `register_blocks()` implementation uses `glob( DAILYOS_PLUGIN_DIR . 'blocks/*/block.json' )` (`wp/dailyos/includes/class-dailyos-plugin.php:154`) and loops over the resulting files (`wp/dailyos/includes/class-dailyos-plugin.php:160`). There is no clean insertion anchor to find, and adding one-off generated lines would fight the current runtime.
- Recommendation: change AC #1/#3 and §5.1 to say the CLI does not edit `class-dailyos-plugin.php` while glob registration remains the production contract. If an explicit allowlist is required, make that a separate PHP refactor with deterministic markers before the scaffold tool depends on it.

## C3. The harness fixture shape cannot prove the subtle producer/projection/renderer contract class

- Location: packet §5.5, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:176`; fixture fields at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:183`; PHP proxy at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:210`; projection model at `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:40`; renderer behavior at `wp/dailyos/blocks/account-overview/render-functions.php:123`.
- Observation: the fixture only names `expected_projection_keys` and `expected_renderer_html_contains` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:187`). That can catch a gross missing key if implemented carefully, but it does not encode value kind, required vs optional semantics, nesting, diagnostics, provenance/edit-route expectations, or renderer branch coverage. The runtime `ProjectedBlock.payload` is an untyped `serde_json::Value` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:46`), and the PHP renderer silently ignores non-string body values or wrong nested shapes (`wp/dailyos/blocks/account-overview/render-functions.php:153`, `wp/dailyos/blocks/account-overview/render-functions.php:155`, `wp/dailyos/blocks/account-overview/render-functions.php:163`, `wp/dailyos/blocks/account-overview/render-functions.php:171`). Existing PHP block tests mostly assert substrings (`wp/dailyos/tests/blocks/AccountOverviewBlockTest.php:103`).
- Recommendation: make the harness fixture schema-based, not substring-based: require JSON pointers with expected `ValueKind`, `required` vs `optional`, expected diagnostics/audits, expected renderer branch, and exact block wrapper/data attributes. Add negative fixtures for field rename, string↔number drift, required-to-optional drift, optional-to-required drift, and `payload.text`→`payload.body` nesting drift.

# High

## H1. Translator coverage overclaims against the primitive inventory

- Location: packet §5.7, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:218`; primitive inventory at `.docs/design/primitives/README.md:15`; Wave 2 translation target at `.docs/plans/v1.4.3-waves.md:171`.
- Observation: the translator claims TSX + CSS Module in, PHP block out, with event handlers downgraded to data attributes and "no JS interactivity" (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:220`). That is not a 90% scaffold for several documented primitives. Definitive hard-no list for automatic working translation: `InlineInput` is not shipped source (`.docs/design/primitives/InlineInput.md:60`) and requires `onChange` editing (`.docs/design/primitives/InlineInput.md:50`); `EditableText` swaps to input/textarea and commits on blur/Tab/Enter (`.docs/design/primitives/EditableText.md:14`); `Switch` is an aria-checked toggle with change handler (`.docs/design/primitives/Switch.md:32`, `.docs/design/primitives/Switch.md:50`); `Segmented` owns selected/focus state and `onChange` (`.docs/design/primitives/Segmented.md:32`, `.docs/design/primitives/Segmented.md:51`); `RemovableChip` owns remove affordance and removal callback (`.docs/design/primitives/RemovableChip.md:33`, `.docs/design/primitives/RemovableChip.md:50`); `FolioRefreshButton` is an action button and currently uses inline styles, not a CSS module (`.docs/design/primitives/FolioRefreshButton.md:14`, `.docs/design/primitives/FolioRefreshButton.md:30`); `EntityChip` has removable/editable variants (`.docs/design/primitives/EntityChip.md:33`); `TypeBadge` editable mode is a dropdown and its source is still local `AccountHero` code, not a standalone primitive file (`.docs/design/primitives/TypeBadge.md:33`, `.docs/design/primitives/TypeBadge.md:57`).
- Recommendation: replace "90% scaffold" with an explicit translator support matrix: static render-only with TSX+CSS module, static render-only with inline styles, source-promotion required, and interactive/manual-only. Make unsupported primitives fail loud with a reason and a pointer to the simple template path.

## H2. Two scaffold shapes are not enough for Wave 1 primitives

- Location: packet §5.2, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:81`; packet §6.2, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:234`; Wave 1 primitive list at `.docs/design/primitives/README.md:17`.
- Observation: "simple = single payload field" and "composite = AccountOverview-style multi-block composition" leaves no first-class shape for typed display primitives with multiple scalar attributes and derived rendering. Wave 1 includes `HealthBadge` with score/band/size/insufficient-data (`.docs/design/primitives/HealthBadge.md:9`), `Avatar` with photo URL/cache/initials/size (`.docs/design/primitives/Avatar.md:9`), `FreshnessIndicator` with timestamp format and staleness threshold (`.docs/design/primitives/FreshnessIndicator.md:14`, `.docs/design/primitives/FreshnessIndicator.md:48`), `EntityChip` with entity type/name/icon/removable/editable variants (`.docs/design/primitives/EntityChip.md:30`), and `TypeBadge` with account-type mapping plus editable dropdown affordance (`.docs/design/primitives/TypeBadge.md:28`). These are not single-field pills, and making each one a composite block is excess ceremony.
- Recommendation: add a third `typed-display` template shape: multiple typed attrs, variant enum validation, optional asset URL, deterministic PHP formatter, no producer commit by default, and harness assertions for each attr's type/requiredness.

## H3. Token generator source paths and precedence rules can produce stale or wrong output

- Location: packet §5.6, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:212`; packet §6.5, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:254`; wave invariant at `.docs/plans/v1.4.3-waves.md:75`; token contract at `.docs/design/tokens/color.md:135`; runtime CSS at `src/styles/design-tokens.css:1`.
- Observation: the packet names `src/styles/tokens.css` as canonical input (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:214`), but the token docs name `src/styles/design-tokens.css` as code source (`.docs/design/tokens/color.md:137`), and the runtime file itself declares the single source of truth for design values (`src/styles/design-tokens.css:1`). The precedence rules also conflict: §5.6 says `theme.json > tokens.css > design/tokens` for runtime overrides and the reverse for canonical definitions (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:214`), while §6.5 says design tokens win for CSS custom-property values and theme wins for WP knobs (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:256`). Real-world conflict: docs define semantic aliases such as `--color-account` mapping to `--color-spice-turmeric` (`.docs/design/tokens/color.md:70`), while CSS stores `--color-account: var(--color-spice-turmeric)` (`src/styles/design-tokens.css:86`). A naive string merge either false-conflicts or lets stale theme output win silently.
- Recommendation: correct the source path, treat `theme.json` as generated output until W3 creates it (`.docs/plans/v1.4.3-waves.md:194`), normalize aliases through a token graph before conflict detection, and fail on any same-token value mismatch outside an explicit allowlist of WP-only generated slugs.

## H4. The proposed split delays the CI gate that C1 says is load-bearing

- Location: packet §7 AC #8, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:275`; packet §10 landing shape, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:309`; wave C1 invariant at `.docs/plans/v1.4.3-waves.md:54`.
- Observation: C1 says no block ships without a passing integration fixture (`.docs/plans/v1.4.3-waves.md:54`), and packet AC #8 requires CI on every block PR (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:275`). But the optional split puts groups 1+2 in PR-C1 and moves the CI workflow to group 4 / PR-C2 (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:309`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:315`). That means the "kit core" can be reviewed or even merged without the required gate unless the split is treated as unmergeable staging, which defeats the review-friendly split.
- Recommendation: if split, land harness + CI together first against `account-overview`, then land CLI/templates, then translator/theme generator. Otherwise keep one PR and drop the claim that PR-C1 is independently landable.

## H5. AccountOverview is not a valid translator parity target as described

- Location: packet reference implementation at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:23`; translator input contract at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:220`; AC #6 at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:273`.
- Observation: the packet identifies the reference implementation as the WP block plus Rust ability (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:23`), but the translator accepts a Tauri React component path plus CSS Module (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:220`). AC #6 then requires translating "the AccountOverview source" and byte-equal output to the live block (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:273`). Hypothesis: there is no Tauri `AccountOverview.tsx` source to feed this translator; the packet is mixing the WP reference block with a TSX translator contract.
- Recommendation: replace AC #6 with a real composite TSX fixture, or state that AccountOverview parity is tested by template regeneration from the existing WP/Rust reference, not by the Tauri-to-block translator.

# Medium

## M1. PHP shell-out path is account-overview-specific, not a reusable block renderer

- Location: packet PHP harness claim at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:210`; current helper at `wp/dailyos/includes/class-dailyos-plugin.php:675`; account renderer at `wp/dailyos/blocks/account-overview/render-functions.php:32`.
- Observation: the packet says `StarterKitIntegrationTest.php` renders through existing `render_block_with_filter` infrastructure (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:210`). The current helper is private and explicitly named for account overview (`wp/dailyos/includes/class-dailyos-plugin.php:682`), requires `blocks/account-overview/render-functions.php` (`wp/dailyos/includes/class-dailyos-plugin.php:683`), and calls `dailyos_account_overview_render` directly (`wp/dailyos/includes/class-dailyos-plugin.php:690`). That will not render arbitrary generated blocks without either reflection or copy-paste.
- Recommendation: define a generic PHP test entrypoint that registers the target block metadata, injects a fake runtime client, and calls WordPress `render_block`/metadata render callback for the requested block name.

## M2. Optional vs required drift is named as a concern but not represented in generated contracts

- Location: packet fixture definition at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:183`; current schema model at `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:211`; current test schema setup at `src-tauri/abilities-runtime/tests/dos570_fallback_projection.rs:92`.
- Observation: the existing `CustomBlockSchema` can distinguish `required_pointers` and `optional_pointers` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:214`), and tests set them directly (`src-tauri/abilities-runtime/tests/dos570_fallback_projection.rs:95`). Packet C's projection template and fixture do not expose that distinction; they only list expected keys and HTML substrings.
- Recommendation: add required/optional pointer declarations to the projection template and require the integration fixture to fail when a required pointer is absent, null, wrong-kind, or only present under an optional fallback branch.

## M3. Theme generator is bundled with translator utilities even though its dependency graph is different

- Location: packet commit groups at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:309`; W3 theme deliverable at `.docs/plans/v1.4.3-waves.md:192`.
- Observation: group 3 combines `generate-theme-json.mjs` with `translate-tauri-to-block.mjs` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:312`). The generator feeds W3 theme output (`.docs/plans/v1.4.3-waves.md:194`); the translator feeds W2 block translation (`.docs/plans/v1.4.3-waves.md:171`). They share Node scripting mechanics, but not acceptance risk.
- Recommendation: split group 3 internally for review: translator with block kit/harness, token generator with token/theme tests and W3 consumers.

# Low

## L1. `--payload-shape` is overloaded between scaffold family and schema details

- Location: packet CLI signature at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:70`; simple template attrs at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:85`.
- Observation: `--payload-shape simple|composite` chooses a template family, but §5.2 also says per-block payload attrs are interpolated from a `--payload-shape` schema (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:86`). Those are different concepts.
- Recommendation: rename the family flag to `--template simple|typed-display|composite` and add a separate `--schema path/to/schema.json` or generated TODO region.

## L2. Verbatim typed-error switch enforcement is brittle as a long-term template invariant

- Location: packet AC #7 at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:274`; CI invariant at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:299`.
- Observation: a grep gate for 5 switch arms proves textual copy, not behavioral equivalence. It will either block harmless formatting changes or miss semantically wrong surrounding control flow.
- Recommendation: keep the grep gate for W1 if speed matters, but add a PHP fixture that injects each typed error and asserts the rendered verification banner class/copy.

## L3. Partial-state handling is named but not made testable

- Location: packet CLI responsibilities at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:72`; partial-state exit code at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:79`; negative fixtures at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:281`.
- Observation: the CLI can exit 2 with "manual cleanup needed" (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:79`), but no negative fixture asserts what files may exist after a failed scaffold or how the developer detects stale generated state.
- Recommendation: add a failure fixture that simulates template copy success plus registry/ability registration failure, then asserts the CLI prints an exact cleanup manifest and leaves no modified tracked file unless `--keep-partial` is passed.

# Path-alpha maintenance items

- → Linear maintenance: Add a repo-wide token source-name cleanup. Location: wave invariant `.docs/plans/v1.4.3-waves.md:75`; token docs `.docs/design/tokens/color.md:137`. Observation: planning docs still say `src/styles/tokens.css`; recommendation: standardize all docs on `src/styles/design-tokens.css`.
- → Linear maintenance: Add a design inventory flag for primitives that are render-only vs interactive. Location: `.docs/design/primitives/README.md:58`; examples `Switch` at `.docs/design/primitives/Switch.md:14` and `Pill` at `.docs/design/primitives/Pill.md:14`. Observation: translator scope decisions require this classification; recommendation: add an `interaction` column to the primitive index.
- → Linear maintenance: Consider extracting account-overview's private block render helper into a public test helper after W1. Location: `wp/dailyos/includes/class-dailyos-plugin.php:675`. Observation: useful beyond Packet C but not necessary if StarterKitIntegrationTest owns a fresh generic path.

## Cycle 2 re-verify

Scope: targeted V1.1 re-verify only for DOS-678, per requested questions 1-6.

Direct check answers: §5.1/§6.6 drop PHP mutation yes (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:221`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:517`).
Direct check answers: §5.4 real API no (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:332`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`).
Direct check answers: §5.5 DOS-670 fixture coverage yes as specified (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:387`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:429`).
Direct check answers: §5.6 token path yes in body, with stale CI residue below (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:454`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:554`).

Cycle-1 CRITICAL verdicts:

- C1 — STILL-BLOCKING: §5.4 still emits non-repo Rust API despite naming the right production flow.
  V1.1 drops the old fake `register_custom_block_schema(...).with_field_binding(...)` path and names `BlockType` + `known_projection_rules()` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:311`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:317`).
  But the emitted snippet imports `KnownProjectionRule` / `ProjectionRuleResult` and calls `KnownProjectionRule::for_block(...)` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:332`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:336`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:341`).
  The actual repo API is private `struct BlockProjectionRule` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`) and private `fn known_projection_rules() -> Vec<BlockProjectionRule>` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1250`).
  Existing constructors are in-file functions like `fn account_overview_rule() -> BlockProjectionRule` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1415`), so the proposed new `{{ability_name}}_rule.rs` file (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:329`) cannot return that private type.
- C2 — RESOLVED: §5.1 now explicitly drops plugin-core mutation.
  It says "**NO modification of `wp/dailyos/includes/class-dailyos-plugin.php`**" and relies on glob registration (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:221`).
  §6.6 says V1.1 keeps glob and no longer modifies plugin core (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:517`).
  The repo confirms `blocks/*/block.json` registration via `glob(...)` (`wp/dailyos/includes/class-dailyos-plugin.php:154`).
- C3 — RESOLVED: §5.5 now specifies a schema-based harness.
  It uses `BindingExpectation` pointer + `ValueKind` + `required` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:367`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:387`) plus diagnostics, renderer branches, and wrapper assertions (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:377`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:380`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:383`).
  The required negative classes cover field rename, type drift, required/optional drift, and payload nesting drift (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:429`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:431`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:432`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:434`).
  This matches the risk because runtime projection payloads are untyped JSON (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:46`) and projection type filtering is kind-based (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:897`).

Cycle-1 HIGH verdicts claimed by the V1.1 changelog:

- H1 — STILL-HIGH: changelog claims a support matrix (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:85`), but §5.7 still says event handlers become data attributes and complex output is a "90% scaffold" (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:474`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:477`).
- H2 — STILL-HIGH: changelog says V1.1 adds `typed-display` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:93`) and §5.1 includes it in the CLI signature (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:210`).
  But §5.2 still defines only simple/composite templates (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:227`) and §6.2 still says two shapes only (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:487`).
- H3 — RESOLVED: §5.6 names `src/styles/design-tokens.css` as canonical runtime CSS input (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:449`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:454`), treats `wp/dailyos/theme/theme.json` as generated output (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:456`), and the referenced CSS file exists (`src/styles/design-tokens.css:1`).
- H4 — STILL-HIGH: changelog says harness + CI land together first (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:135`), but §10 still says group 4 is CI and PR-C2 is translator + theme generator + CI (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:566`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:568`).
- H5 — STILL-HIGH: changelog says AC #6 was replaced with Pill + HealthBadge (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:143`), but §5.7 still tests AccountOverview from TSX source (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:479`) and AC #6 still requires byte-equal AccountOverview regeneration (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:526`).

New V1.1 findings:

- HIGH: The body/acceptance/fixture/CI sections still encode old V1.0 work.
  §5.1 drops plugin-core mutation (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:221`), but AC #3 and fixture #3 still require a `class-dailyos-plugin.php` enumeration update (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:523`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:536`).
- HIGH: The stale-contract gap also affects token and translator gates.
  §5.6 corrects the input path to `src/styles/design-tokens.css` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:454`), but CI invariant #4 still watches `src/styles/tokens.css` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:554`).
  Changelog H5 says Pill + HealthBadge (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:146`), but CI invariant #6 still uses Pill + AccountOverview (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:556`).
- MEDIUM: §6.4 still describes old substrate reuse.
  It names `CustomBlockSchema` registration and `render_block_with_filter` (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:497`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:502`), while §5.4 rejects the dynamic custom-schema path (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:354`) and §5.5 rejects the account-overview-specific render helper (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:437`).

Acceptability call on changelog deferrals:

- Acceptable as non-blocking: path-alpha residuals already filed to DOS-684 remain out of this cycle-2 scope; I did not re-litigate the repo-wide token-name cleanup, primitive inventory interaction column, or public helper extraction.
- Not acceptable as closure: typed-display, worked prepare body, translator scope matrix, landing split, and AC #6 can be body-edits-to-follow only if this packet remains BLOCK.
  They cannot count as resolved while body and AC/CI still instruct old work (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:477`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:487`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:526`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:568`).

Final cycle-2 verdict: BLOCK. C2 and C3 are resolved, but C1 remains a compile-shape blocker against the real projection API, and several changelog-claimed HIGH fixes are not yet propagated into the implementation contract.

## Cycle 3 re-verify

Verdict: BLOCK. Targeted V1.2 re-check for DOS-678 finds the V1.1 `KnownProjectionRule::for_block` issue is gone, but the replacement paste-not-template snippet still does not match the private Rust projection API.

1. §5.4 paste-not-template API check: still blocking.

- The V1.2 approach correctly stops generating a separate Rust module and instead asks the developer to paste an in-file rule function into `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs` (§5.4, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:319`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:324`).
- That resolves the cycle-2 privacy/scope mismatch: `fallback_projection::BlockProjectionRule` is private (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`) and `fallback_projection::known_projection_rules()` is private (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1250`), so same-file paste is the only zero-substrate-change route.
- The emitted function body is still not compile-shaped. Packet §5.4 emits `BlockProjectionRule { block_type, required_pointers, optional_pointers, render_annotations }` with `FieldPath::new(...)` values (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:342`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:345`).
- The real `fallback_projection::BlockProjectionRule` fields are `block_type`, `composition_kind`, `type_namespace`, `render_annotations`, `fields`, and `default_trust_band` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`). There are no `required_pointers` or `optional_pointers` fields on the production rule.
- The real rule payload path is `fields: &'static [FieldPolicy]`; `FieldPolicy` is private and created through in-file helpers like `text_field`, `number_field`, `object_field`, `bool_field`, and `array_field` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:237`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1273`).
- Existing valid rule shape is `account_overview_rule() -> BlockProjectionRule`, which supplies `composition_kind`, `type_namespace`, `render_annotations`, `fields`, and `default_trust_band` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1415`).
- `FieldPath::new("/payload/text")` is valid JSON-pointer syntax, but it returns `Result<FieldPath, FieldAttributionError>` (`src-tauri/abilities-runtime/src/abilities/provenance/field.rs:20`), so it would not type-check inside a direct `vec![FieldPath::new(...)]` field even if the omitted fields existed.
- Packet step [1] also points at the wrong source anchor: it says "BlockType enum at composition.rs:175" (§5.4, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:334`), but line 175 is `BindingRole`; the `composition::BlockType` enum starts at `composition.rs:330`.
- Adding a new `BlockType` variant also requires updating `BlockType::type_id()`'s exhaustive match (`src-tauri/abilities-runtime/src/abilities/composition.rs:350`) and `rule_for_block_type()` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1236`); §5.4 only mentions the enum variant and `known_projection_rules()` (§5.4, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:334`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:353`).
- Compile-shaped §5.4 output needs a same-file `const {{BLOCK_TYPE}}_FIELDS: &[FieldPolicy] = &[...]`, using the existing helper constructors rather than `FieldPath` vectors (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1330`).
- It needs to choose `text_field`, `number_field`, `object_field`, `bool_field`, or `array_field` per binding value kind (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1273`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1294`).
- It needs `ClaimSensitivity` per field; that type is already in-scope inside `fallback_projection.rs` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:19`).
- It needs `default_trust_band: TrustBand::...`, because that field is mandatory on `BlockProjectionRule` (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:261`).
- It needs `composition_kind` and `type_namespace` policy decisions, even when they are `None`, because both fields are mandatory (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:257`).

2. V1.1 changelog-body propagation: partially fixed, not complete.

- Fixed: §5.2 now has the typed-display template body and examples (§5.2, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:227`).
- Fixed: §5.7 now has a four-category translator scope matrix and Pill + HealthBadge parity targets (§5.7, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:483`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:503`).
- Fixed: §6.2 and §6.4 now name three template shapes and the real reuse list (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:511`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:519`).
- Fixed: §7 AC #3, #5, and #6 now use glob registration, Pill, and HealthBadge instead of plugin-core enum edits or AccountOverview parity (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:548`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:550`).
- Fixed: §8 fixture #3 now asserts glob auto-registration and no `class-dailyos-plugin.php` mutation (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:561`).
- Fixed: §9 invariants #4 and #6 now watch `src/styles/design-tokens.css` and Pill + HealthBadge (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:579`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:581`).
- Still stale: §8 fixture #8 still requires "Translator on AccountOverview source" byte parity (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:566`), contradicting §5.7's statement that AccountOverview is not a translator target (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:503`).
- Still stale: §8 fixture #10 still names `tokens.css` and `theme.json` as competing sources (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:568`), contradicting §5.6's canonical input/output split (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:464`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:468`).
- No remaining stale-body issue found in §7 acceptance criteria after the V1.2 edits (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:546`).
- No remaining stale-body issue found in §9 CI invariants after the V1.2 edits (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:572`).
- The remaining propagation gap is confined to §8 fixture rows, not the main body/AC/CI sections.

3. New paste-not-template architectural concerns.

- The approach is directionally defensible because it avoids making private projection internals public just to satisfy a scaffold tool (§5.4, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:366`; `fallback_projection::BlockProjectionRule`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`).
- The concern is enforcement: §5.4 says the CLI prints snippets and does not edit Rust files (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:324`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:364`), but no compile fixture/snapshot gate proves those emitted snippets still match the private in-file shape.
- That risk has already materialized in V1.2: the pasted snippet copies `CustomBlockSchema`-style required/optional pointers into `BlockProjectionRule`, whose production contract is `fields: &'static [FieldPolicy]` plus trust/scope metadata (`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:210`, `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255`).

4. Commit-group structure and split policy.

- §10 is structurally clean: CI workflow ships in group 1 with the harness and first fixture, so the "no block without fixture" gate exists before templates/CLI land (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:587`).
- §10 also cleanly separates templates/CLI, token generator, and translator into groups 2-4 (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:589`).
- The split policy is clean: PR-C1 keeps groups 1+2 together and is independently mergeable, while token generator and translator split into PR-C2/PR-C3; it explicitly says not to split groups 1+2 (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:593`).
- This structure does not rescue C1: group 2 would still ship invalid projection snippets unless §5.4 is corrected to the actual `FieldPolicy`/`BlockProjectionRule` shape.
- The split policy is otherwise no longer carrying the cycle-2 H4 problem: CI is in group 1, not delayed to the translator/theme group (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:587`, `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:599`).

Bottom line: keep BLOCK. To approve, §5.4 must emit a same-file snippet shaped like the existing `*_FIELDS` const + `*_rule() -> BlockProjectionRule` pattern, including `composition_kind`, `type_namespace`, `fields`, `default_trust_band`, `BlockType::type_id()`, and `rule_for_block_type()` updates; §8 must replace fixture #8 and fixture #10 with the V1.2 Pill/HealthBadge and `design-tokens.css` contracts.

## Cycle 4 re-verify

Scope: targeted V1.3 re-verify only for DOS-678 cycle-3 blockers. Not a full cycle-3 rerun.

1. PASS — §5.4 paste snippet matches the real `BlockProjectionRule` struct shape.

- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:255-262` defines `BlockProjectionRule { block_type, composition_kind, type_namespace, render_annotations, fields, default_trust_band }`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:358-366` emits those same six fields in `fn {{ability_name}}_rule() -> BlockProjectionRule`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:350-356` emits `const {{ABILITY_NAME_UPPER}}_FIELDS: &[FieldPolicy] = &[ text_field("/payload/text", ClaimSensitivity::Internal), ... ];`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1273-1279` confirms `text_field(pointer, sensitivity)` returns a `FieldPolicy`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1415-1423` confirms the `account_overview_rule()` pattern: same struct fields, `fields: ACCOUNT_OVERVIEW_FIELDS`, and `default_trust_band`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:388` explicitly says `required_pointers` / `optional_pointers` were the V1.1/V1.2 wrong shape, not the V1.3 emitted snippet.

2. PASS — §5.4 paste-point coverage includes the full required set.

- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:334-338` covers the `BlockType` variant addition.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:340-343` covers the `BlockType::type_id()` exhaustive match arm.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:345-367` covers the `FIELDS` const plus the rule function.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:369-372` covers the `rule_for_block_type()` match arm.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:374-378` covers the `known_projection_rules()` Vec registration.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1236-1247` and `:1250-1260` confirm those two paste targets are distinct required functions in the real source.

3. PASS — source anchors for paste targets point at the right code.

- Evidence: `src-tauri/abilities-runtime/src/abilities/composition.rs:330` is `pub enum BlockType`, matching packet step [1] at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:334`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/composition.rs:350` is `pub fn type_id(&self) -> &str`, matching packet step [2] at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:340-343`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1236` is `fn rule_for_block_type(block_type: &BlockType) -> Option<BlockProjectionRule>`, matching packet step [4] at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:369-372`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1250` is `fn known_projection_rules() -> Vec<BlockProjectionRule>`, matching packet step [5] at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:374-378`.
- Evidence: `src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1415` is `fn account_overview_rule() -> BlockProjectionRule`, matching the packet's canonical pattern reference at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:322`.
- Note: all requested anchors are exact in the current source; no minor line-drift callout needed.

4. PASS — §8 fixture #8 now references HealthBadge translator parity, not AccountOverview as the target.

- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:602` defines fixture #8 as `c1_translator_typed_display_healthbadge`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:602` targets `src/components/shared/HealthBadge.tsx` and generated `wp/dailyos/blocks/health-badge/`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:539` names Pill + HealthBadge as translator parity targets and says AccountOverview is not a translator target.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:587` AC #6 also targets `src/components/shared/HealthBadge.tsx`.
- Note: the fixture row mentions AccountOverview only as the replaced V1.0 claim, not as the V1.3 fixture target.

5. PASS — §8 fixture #10 now references token graph alias normalization with paths consistent with §5.6.

- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:604` defines fixture #10 as `c1_theme_json_alias_normalization`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:604` references `.docs/design/tokens/color.md:70` and `src/styles/design-tokens.css:86`.
- Evidence: `.docs/design/tokens/color.md:70` defines `--color-account` as an alias to `--color-spice-turmeric`.
- Evidence: `src/styles/design-tokens.css:86` defines `--color-account: var(--color-spice-turmeric);`.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:500-508` names `.docs/design/tokens/` + `src/styles/design-tokens.css` as canonical inputs and describes token graph normalization.
- Evidence: `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:504-505` keeps `wp/dailyos/theme/theme.json` as generated output, consistent with fixture #10's V1.3 correction note at line 604.

Final targeted verdict: APPROVE.
All five requested items pass against the current packet and source anchors.
No cycle-3 passed items were re-verified beyond source reads needed for the five targeted questions.
No BLOCK or MINOR findings remain in this targeted scope.
