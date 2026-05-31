# Verdict

BLOCK. Packet C V1.0 is not implementation-ready. The core kit claims are stronger than the current contracts support: the projection template does not match the Rust API, the CLI registration step targets a PHP anchor that does not exist, the harness can still miss the subtle DOS-670-shaped drift class, and the translator/template scope overclaims against the primitive inventory.

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
