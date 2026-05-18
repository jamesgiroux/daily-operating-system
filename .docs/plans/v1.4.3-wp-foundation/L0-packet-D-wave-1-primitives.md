# L0 Packet D — v1.4.3 W2: Wave 1 Primitive Blocks (11-primitive translation batch)

## 1. Header

- **Author:** James Giroux (with Claude)
- **Date:** 2026-05-18
- **Linear:** [DOS-682](https://linear.app/a8c/issue/DOS-682) (v1.4.3 W2)
- **Branch:** TBD (`dos-682-w2-wave-1-primitives` once W1 PR #303 merges; can branch from `w1-c1-starter-kit` if parallel)
- **Wave plan:** `.docs/plans/v1.4.3-waves.md` §W2 (lines 167-189)
- **Upstream gate:** W1 starter kit (PR #303 against `dev`) MUST be mergeable; W2 consumes its CLI, templates, harness, translator, and CI gate

**Intelligence Loop integration check — exempt.** W2 ships render-only WP blocks consuming existing claim/projection substrate; no new claim/table/surface, no new provenance fields, no new signal types, no new feedback paths. Per CLAUDE.md the check does not apply.

## 2. Changelog

- **V1.0 (2026-05-18):** Initial L0 draft. Authored against `.docs/plans/v1.4.3-waves.md` §W2 + Linear DOS-682 + W1 starter kit (PR #303) execution artifacts. Reviewer panel: codex challenge + codex consult + code-reviewer + design-reviewer (parity validation against Tauri React + reference HTML). CSO advisory only (no new trust boundaries; same local-to-local render path as W1 starter kit).
- **V1.1 (2026-05-18):** Folded L0 cycle 1 reviewer findings + James's L6 decisions. Cycle-1 verdicts: codex challenge BLOCK (AC #8 wrong — paste-pattern substrate touches ARE the W1 design); code-reviewer CONDITIONAL APPROVE (TypeBadge needs extraction; IntelligenceQualityBadge source path needs pinning); design-reviewer CONDITIONAL APPROVE (take ScoreBand only from DOS-325's three primitives; token-mapping manifest gate); codex consult queued but never ran (skipped given the BLOCK). Folds in this revision:
  - **AC #8 rewritten** to allow W1-spec'd paste-pattern substrate touches (BlockType variant, type_id arm, fallback projection rule, rule_for_block_type arm, known_projection_rules Vec entry). What stays forbidden is edits to W1 *starter-kit code* (CLI, templates, translator, generator, harness body, CI workflow). Paste-snippets are emitted by `pnpm dailyos:new-block` per primitive — see `wp/dailyos/scripts/new-block.mjs:121-125` + `:210-251`.
  - **ScoreBand promoted to 11th primitive** (§5.2 + §10 PR-D4 + AC #1). DOS-325 has three primitives (ScoreBand, TrendStrip, EvidenceDrawer); W2 takes ScoreBand only; TrendStrip + EvidenceDrawer file to v1.4.4 W2 Entity Surfaces with explicit lineage from DOS-325 (see §6.4 + §13).
  - **§5.2 expanded** with per-primitive hand-translation tasks for the four tricky primitives (Avatar, FreshnessIndicator, IntelligenceQualityBadge, ProvenanceTag). ProvenanceTag carries explicit DOS-477 display-safe-leak guards.
  - **§5.8 added: Shared primitive chrome service** — empty/loading/error rendering centralized, token-driven, surface-agnostic, consumed by all 11 primitives.
  - **§5.9 added: Token-mapping manifest gate** — translator emits per-primitive token-mapping output; CI grep-gate that every `var(--wp--preset--color--*)` in a generated block's style.css resolves through the W1 theme.json output. Filed as DOS-685 path-α append if W1's generator doesn't already emit the manifest.
  - **§5.10 added: TypeBadge extraction sub-task** — extract `AccountTypeBadge` from `src/components/account/AccountHero.tsx:113-172` into `src/components/ui/TypeBadge.tsx`, migrate AccountHero to consume the extracted primitive in the same PR as the TypeBadge block translation.
  - **§5.2 IntelligenceQualityBadge source path pinned** to `src/components/entity/IntelligenceQualityBadge.tsx` (V1.0 had only the directory).
  - **§7 visual parity matrix** — per-primitive state enumeration table (every variant captured side-by-side Tauri vs WP at L4).
  - **§7 + §13 DOS-9/11/325 closure** — W2 closes DOS-9 + DOS-11 fully (cite-chip tooltip + trust-band UI ship in W2 primitives). DOS-325 closes for ScoreBand only; explicit follow-up lineage to v1.4.4 W2 for TrendStrip + EvidenceDrawer + surface-behavior residue (entity envelope, keyboard, drawer integration). Surface-behavior residue does not block DOS-9/11/325 closure — that is v1.4.4 surface-tier scope per the wave plan.
  - **TrustBandBadge promotion** is mechanically unblocked. PR #304 merged 2026-05-18 — the design-system primitives README now treats "WP block consumption counts as integrated" per ADR-0129 reorientation, so promoting `proposed`→`integrated` in the W2 block PR is the canonical promotion path.
  - **Doc-drift note:** wave plan §103 + §184 still call this packet `L0-packet-C-wave-1-primitives.md`; the W1 starter kit took packet-C. Actual filename is `L0-packet-D-wave-1-primitives.md`. Filed as wave-plan touch-up (not blocking this packet).

## 3. Status Snapshot

- Linear ticket: DOS-682 (Backlog, v1.4.3 — WordPress Foundation, priority High).
- Substrate reuse: 100% of the W1 *starter-kit code* (CLI + templates + translator + harness + CI gate). Per-primitive paste-pattern touches into `composition.rs` + `fallback_projection.rs` are the documented W1-spec'd extension path; zero net-new substrate *APIs*.
- Wave-2 primitive count: **11** = 10 from the `.docs/design/primitives/README.md` Wave 1 table (Pill, HealthBadge, StatusDot, Avatar, TrustBandBadge, IntelligenceQualityBadge, FreshnessIndicator, ProvenanceTag, EntityChip, TypeBadge) + **ScoreBand** from DOS-325 (TrendStrip + EvidenceDrawer file to v1.4.4 W2 Entity Surfaces).
- **W2 unlocks:** W3 magazine theme (consumes block primitives via patterns); v1.4.4 surface migration (composes primitives into surfaces, picks up DOS-325 TrendStrip + EvidenceDrawer + surface-behavior residue).

## 4. Pre-work — substrate reuse audit

| What we need | Where it lives (substrate, already shipped) |
|---|---|
| Block scaffold CLI | `wp/dailyos/scripts/new-block.mjs` (W1 Group 2) |
| 3 template shapes (simple / typed-display / composite) | `wp/dailyos/scripts/templates/{simple,typed-display,composite}/` (W1 Group 2) |
| Translator with scope matrix | `wp/dailyos/scripts/translate-tauri.mjs` (W1 Group 4) |
| Integration harness (Rust + PHP) | `src-tauri/abilities-runtime/tests/block_kit_integration_harness.rs` + `wp/dailyos/tests/blocks/StarterKitIntegrationTest.php` (W1 Group 1) |
| CI gate (per-block fixture invariant, translator parity, theme.json --check, CLI exit-codes, path filter) | `.github/workflows/block-kit-integration.yml` (W1 Group 1+L2) |
| Block.json composition attrs convention | `STANDARD_COMPOSITION_ATTRS` in `wp/dailyos/scripts/translate-tauri.mjs` |
| Token-driven theming | `wp/dailyos/theme/theme.json` generated by W1 Group 3 |
| Glob block registration | `wp/dailyos/includes/class-dailyos-plugin.php:149-163` (no edits needed) |
| Existing Tauri React primitive sources (per primitive) | `src/components/{ui,shared,entity}/<PrimitiveName>.tsx` + `.module.css` |
| Primitive design specs | `.docs/design/primitives/<PrimitiveName>.md` |
| Reference HTML for visual parity | `.docs/design/reference/_shared/primitives.css` + mockup directories |

**Net-new in W2:** only `wp/dailyos/blocks/<primitive>/` directories (10 of them) + matching Rust integration fixtures under `src-tauri/abilities-runtime/tests/fixtures/<primitive>_integration_fixture.rs`. Zero substrate edits.

## 5. What this packet authors

### 5.1 Per-primitive translation (default path)

For each primitive: run `pnpm dailyos:translate-tauri --primitive <PrimitiveName>`. The translator handles Supported-category primitives end-to-end (Pill + HealthBadge are AC §5.7 parity targets with hand-coded bodies; others get scaffold defaults that the per-primitive PR fills in).

### 5.2 Per-primitive hand-translation (where translator scaffold is insufficient)

Per W1 translator scope matrix (V1.3.1), augmented per L0 cycle 1 L6 #2 (all four tricky primitives hand-translated in W2, not deferred):

| Primitive | Source TSX (pinned) | Source CSS | Category | Hand-translation work in W2 |
|---|---|---|---|---|
| Pill | `src/components/ui/Pill.tsx` | `Pill.module.css` | Translator parity target (full output) | None — translator output ships verbatim; visual parity at L4 |
| HealthBadge | `src/components/shared/HealthBadge.tsx` | `HealthBadge.module.css` | Translator parity target (full output) | None — translator output ships verbatim; visual parity at L4 |
| StatusDot | `src/components/shared/StatusDot.tsx` | `StatusDot.module.css` | Supported scaffold (body TODO) | Body: status→token mapping (healthy/warning/critical/unknown); empty/loading/error chrome from §5.8 service |
| Avatar | `src/components/ui/Avatar.tsx` | `Avatar.module.css` | Supported scaffold — **hand-translation required** | TSX has `useEffect` + Tauri `invoke('resolve_avatar_source')` + inline `style={{backgroundImage: ...}}`; WP render-php must (a) resolve avatar source via the runtime ability path, not invoke from PHP; (b) push the runtime-resolved URL through a CSS custom property declared inline ONLY as `--dailyos-avatar-bg-url`, not arbitrary style attributes (memory: no inline CSS — use CSS custom-property pattern); (c) fall back to initials when source is empty via §5.8 chrome service |
| TrustBandBadge | `src/components/ui/TrustBandBadge.tsx` | `TrustBandBadge.module.css` | SupportedWithSourcePromotion | Body: `proposed`→`integrated` README PR is mechanically unblocked by PR #304 (see §6.3); translation otherwise scaffold-only |
| IntelligenceQualityBadge | `src/components/entity/IntelligenceQualityBadge.tsx` | `IntelligenceQualityBadge.module.css` | Supported scaffold — **hand-translation required** | TSX has dynamic style logic (band threshold → variant class + label text + tooltip copy). WP render-php must derive variant + label from the projected claim's quality score; the variant→token mapping table lives in render-functions.php and consumes theme.json tokens via §5.9 manifest. Tooltip render path is on-demand only (display-safe — no raw factor names per DOS-477 + DOS-325 voice rules). |
| FreshnessIndicator | `src/components/ui/FreshnessIndicator.tsx` | `FreshnessIndicator.module.css` | Supported scaffold — **hand-translation required** | TSX runs dynamic date math (`formatRelativeDateShort` on `enrichedAt`). WP render-php must compute the relative-date string server-side at render time (PHP) so client never sees raw timestamps unless DOS-477-permitted; cache TTL aligned to the freshness band (likely_current vs use_with_caution vs needs_verification thresholds). |
| ProvenanceTag | `src/components/ui/ProvenanceTag.tsx` | `ProvenanceTag.module.css` | Supported scaffold — **hand-translation required (DOS-477 critical)** | TSX renders source attribution + age + freshness chip + tooltip. WP render-php MUST consume only display-safe provenance fields per DOS-477 (no raw source-internal identifiers, no email addresses, no internal note bodies, no debug carriers). Field allowlist for ProvenanceTag payload pulled from the `ProvenanceRef`-resolved envelope's actor-filtered render projection (ADR-0108 §2). DOS-9 cite-chip age/freshness tooltip pattern folds in as the `data-age` + tooltip render path. Negative fixture #7 (see §8) asserts no disallowed field reaches the rendered DOM, data-* attributes, REST preload, hydration state, or inspector UI. |
| EntityChip | `src/components/ui/EntityChip.tsx` | `EntityChip.module.css` | Translator-NotSupported (editable variants) | Display-only ships in W2 via `pnpm dailyos:new-block --template typed-display`; editable variant deferred to v1.4.4 surface migration where the W4 feedback router lands. |
| TypeBadge | extracted from `src/components/account/AccountHero.tsx:113-172` → `src/components/ui/TypeBadge.tsx` (see §5.10) | from `AccountHero.module.css` | Translator-NotSupported (editable variants) | Display-only ships in W2; editable variant deferred to v1.4.4. Extraction sub-task spec'd in §5.10 — must land same PR as TypeBadge block. |
| ScoreBand (new) | none yet — design-spec lives in DOS-325; produce `src/components/ui/ScoreBand.tsx` + `.module.css` in the W2 PR-D4 commit | new | New primitive (DOS-325 fold) | Author Tauri primitive first (band-label only, no raw number in headline per DOS-325 voice rule), then translate to WP block. Consumes claim trust band + entity-intelligence envelope. TrendStrip + EvidenceDrawer NOT in W2 — file to v1.4.4 with explicit DOS-325 lineage. |

Per-primitive deliverable: `wp/dailyos/blocks/<slug>/{block.json,render.php,render-functions.php,style.css,edit.js,editor.css}` + matching `src-tauri/abilities-runtime/tests/fixtures/<slug>_integration_fixture.rs` + per-primitive paste-snippet manifest applied to `composition.rs` + `fallback_projection.rs` (per AC #8 / W1 `new-block.mjs` spec).

### 5.3 Fold v1.4.10-dissolved UX patterns

Per wave plan §183 + L0 cycle 1 L6 #5, three previously-parked v1.4.10 tickets fold into the v1.4.3/v1.4.4 program as follows:
- **DOS-9** (cite-chip age + freshness tooltip): folded into `ProvenanceTag` block as the `data-age` + tooltip attributes. Closes in W2 PR-D2 (ProvenanceTag PR).
- **DOS-11** (trust-band UI): folded into `TrustBandBadge` block as the primary visual semantic. Closes in W2 PR-D3 (TrustBandBadge PR).
- **DOS-325** (score bands + evidence drill-down): produces `ScoreBand` as the 11th primitive in W2 PR-D4. The remaining two DOS-325 primitives — **TrendStrip** + **EvidenceDrawer** — file to v1.4.4 W2 Entity Surfaces with explicit DOS-325 lineage (see §13). DOS-325 closes for the ScoreBand slice; lineage tickets keep the remaining work tracked without holding DOS-325 itself open.

Surface-behavior residue named in DOS-9 + DOS-11 + DOS-325 (entity-envelope wiring, keyboard interaction, evidence-drawer integration into entity-detail pages) is v1.4.4 W2 Entity Surfaces scope per the wave plan, not v1.4.3 W2. Closure of DOS-9/11/325 in v1.4.3 is bounded to "the primitive ships as a WP block with token consumption + visual parity" per L6 #5 stretch interpretation.

### 5.4 Implementation fan-out shape

Per wave plan §185: 2-up parallel codex per primitive (worktree-isolated). 5 sequential rounds × 2 primitives = 10 primitive PRs (or grouped into 2-3 commit-batches if PR overhead is excessive).

Per primitive, agent reads:
1. Existing TSX (`src/components/<sub>/<Name>.tsx`)
2. Existing CSS Module (`.module.css`)
3. Reference HTML (`.docs/design/reference/_shared/primitives.css` for the primitive's classes)
4. Design spec (`.docs/design/primitives/<Name>.md`)

Then: run translator (or new-block CLI if NotSupported variant), customize per spec, drop matching Rust fixture, run `cargo test -p abilities-runtime --test block_kit_integration_harness` to confirm parity. Commit.

### 5.8 Shared primitive chrome service (empty / loading / error)

Per L0 cycle 1 L6 #3: every primitive that renders against substrate data has empty / loading / error states. V1.0 implicitly let each primitive re-implement these; V1.1 centralizes them as a single chrome service all 11 primitives consume. Per CLAUDE.md service-layer discipline + memory `feedback_no_inline_css.md` + `feedback_design_system_taxonomy.md`.

**Authoring seam.** The service is token-driven, surface-agnostic, and has matched implementations on both sides of the W1 translator:
- **Tauri side:** `src/components/ui/_chrome/PrimitiveChrome.tsx` exposing `<EmptyState />`, `<LoadingState />`, `<ErrorState />` consumed by all 11 primitive components. CSS Module driven by design tokens; no inline styles.
- **WP side:** `wp/dailyos/blocks/_shared/chrome/` with `render-empty.php`, `render-loading.php`, `render-error.php` partials that primitive `render-functions.php` files require. Token consumption via theme.json variables only (per §5.9 manifest gate).

**Why not "single Rust producer."** A producer-side chrome emitter would change the `Composition` / `Block` payload shape (every block would carry chrome metadata), which is a substrate contract change outside W2 scope and would force ADR-0130 changes. Surface-side rendering with shared service modules per surface is the simpler seam.

**Acceptance:** no primitive's `render-functions.php` contains an inline empty/loading/error rendering path. Grep gate: `wp/dailyos/blocks/<slug>/render-functions.php` MUST `require_once` one of `_shared/chrome/render-*.php` when handling the corresponding state. New CI invariant §9.7.

**Design-reviewer + code-reviewer judge the seam in L0 cycle 2.** If the chrome service seam is wrong, this section is what changes — not the per-primitive sections.

### 5.9 Token-mapping manifest gate

Per L0 cycle 1 L6 #3 + design-reviewer cycle-1 finding. The W1 translator changed CSS variable names from `--color-spice-turmeric` to `var(--wp--preset--color--spice-turmeric)` via theme.json mapping (V1.0 open question #1). V1.1 closes this by requiring the translator to emit a per-primitive token-mapping manifest and gating CI on round-trip resolution.

**Manifest shape.** For each primitive, `pnpm dailyos:translate-tauri --primitive <Name>` emits `wp/dailyos/blocks/<slug>/.token-mapping.json` listing every `var(--wp--preset--color--*)` (and other WP-mapped tokens) used in `style.css`, paired with its source `--color-*` (or other token) name in the canonical Tauri CSS Module.

**CI gate** (new — `block-kit-integration.yml` step "Token-mapping manifest"):
1. For each `wp/dailyos/blocks/<slug>/.token-mapping.json`, assert every `wp--preset--color--*` entry resolves to a defined color in `wp/dailyos/theme/theme.json` settings.color.palette.
2. For each `var(--wp--preset--color--*)` in `wp/dailyos/blocks/<slug>/style.css`, assert it is listed in `.token-mapping.json`.
3. Grep gate: no raw color literal (`#`, `rgb(`, `hsl(`, named CSS color) in `wp/dailyos/blocks/<slug>/style.css` outside the manifest's allowed escape list.

**W1 amendment if generator does not emit manifest.** If `pnpm dailyos:translate-tauri` does not currently emit `.token-mapping.json`, file as DOS-685 path-α append (W1 starter-kit maintenance) and resolve before the first W2 PR lands. **This is the only allowed W1 starter-kit code edit during W2** — and only because the manifest is part of the translator's contract, not a per-primitive concern.

### 5.10 TypeBadge extraction (sub-task, same PR as TypeBadge block)

Per L0 cycle 1 L6 finding (code-reviewer): TypeBadge currently lives inline in `src/components/account/AccountHero.tsx:113-172` as `AccountTypeBadge`. The primitives README + wave plan §182 already specify TypeBadge as a primitive in its own right. W2 PR-D3 (TypeBadge block) MUST first extract the Tauri primitive into its canonical location.

**Sub-task acceptance:**
1. Create `src/components/ui/TypeBadge.tsx` with the extracted component (function + `ACCOUNT_TYPES` table + dropdown state). Props: `value: AccountType`, `onChange?: (v: AccountType) => void`, optional `readOnly` (forces display-only render).
2. Migrate `src/components/account/AccountHero.tsx` to `import TypeBadge from '@/components/ui/TypeBadge'` and remove the inline `AccountTypeBadge` definition + module CSS classes that are no longer used in AccountHero.
3. Move TypeBadge-owned CSS (`badge`, `customerBadge`, `internalBadge`, `partnerBadge`, `typeBadgeWrapper`, `typeBadgeButton`, `typeBadgeChevron`, `typeBadgeDropdown`, `typeBadgeOption`, `typeBadgeOptionActive`, `typeBadgeOptionCustomer/Internal/Partner`) from `AccountHero.module.css` into `TypeBadge.module.css`.
4. Confirm no other call sites reference the inline name (grep for `AccountTypeBadge`).
5. `cargo clippy -- -D warnings && pnpm tsc --noEmit && pnpm test` green; AccountHero still renders TypeBadge correctly at L4.
6. Then proceed with the W2 TypeBadge block translation against the newly-extracted primitive. Both extraction + block ship in the same PR.

This is **a 1:1 source extraction, not a refactor.** No new variants, no API changes beyond optional `readOnly`. The extraction is what makes the W2 block translation source-pinnable; without it the W2 block has no canonical primitive to translate from.

## 6. Directional decisions resolved at L0

### 6.1 Single batched packet vs per-primitive packets

Single batched L0 packet (this one). Per-primitive packets would 10x the L0 review burden with diminishing returns — the translation pattern is identical across primitives. Per-primitive divergence handled in this packet's §5.2.

### 6.2 EntityChip + TypeBadge: display-only in W2

Both have editable variants per .docs/design/primitives/. Editable variants need the W4 feedback-write infrastructure (click-bound router + nonce). W2 ships display-only; editable variants defer to W4 follow-up tickets.

### 6.3 TrustBandBadge proposed→integrated promotion

Per primitives README, TrustBandBadge is currently `proposed`. The W2 WP block is the first integrated consumer; promote to `integrated` in the same PR (markdown-only doc change to README + create `.docs/design/primitives/TrustBandBadge.md` proper if missing). Per primitives README "Adding a primitive that already exists in src/", promotion is markdown-PR only — no code consolidation required.

### 6.4 ScoreBand: 11th primitive, W2 takes ScoreBand only

Resolved at L0 cycle 1 L6 (James, 2026-05-18): ScoreBand is a distinct visual primitive (band-label with no raw number in the headline, per DOS-325 voice rule). W2 takes ScoreBand only. The remaining two DOS-325 primitives (TrendStrip + EvidenceDrawer) file to v1.4.4 W2 Entity Surfaces with explicit DOS-325 lineage (see §13). This resolves V1.0 open question §12.4.

ScoreBand authoring sequence in PR-D4:
1. Author Tauri React primitive at `src/components/ui/ScoreBand.tsx` + `ScoreBand.module.css`. Band labels only (`On Track`, `Watching`, `Action Needed`, `No Read`); no raw numbers in headline copy. Consumes claim trust band from the entity-intelligence envelope, not raw score values.
2. Add to `.docs/design/primitives/README.md` Wave 1 table + create `.docs/design/primitives/ScoreBand.md` design spec referencing DOS-325 source material.
3. Translate to WP block via `pnpm dailyos:new-block ScoreBand --template typed-display`.
4. Apply paste-pattern substrate touches per AC #8 (BlockType variant + projection rule).
5. Fixture + L4 parity proof.

### 6.5 Visual parity standard

Visual parity vs Tauri React: hero-state side-by-side screenshots at L4. CSS values may differ where W1 token generator + theme.json mapping forces them to (palette slug naming changed `--color-spice-turmeric` → `var(--wp--preset--color--spice-turmeric)`); design-reviewer judges acceptable drift per primitive.

## 7. Acceptance criteria

1. **All 11 primitives ship as `wp/dailyos/blocks/<primitive>/` directories** with block.json, render.php, render-functions.php, style.css, edit.js, editor.css. Roster: Pill, HealthBadge, StatusDot, Avatar, TrustBandBadge, IntelligenceQualityBadge, FreshnessIndicator, ProvenanceTag, EntityChip, TypeBadge, ScoreBand.
2. **Each block has a matching Rust integration fixture** in `src-tauri/abilities-runtime/tests/fixtures/<primitive>_integration_fixture.rs`. The W1 CI gate enforces this.
3. **All 11 fixtures pass** the `block_kit_integration_harness` test in a single `cargo test` run.
4. **Visual parity matrix to Tauri React** validated at L4 hands-on, per §7.1 below. Design-reviewer signs off the full matrix.
5. **TrustBandBadge promoted** in `.docs/design/primitives/README.md` from `proposed` → `integrated`. `.docs/design/primitives/TrustBandBadge.md` exists with full spec. Promotion legitimacy established by PR #304 (design vocab clarification, merged 2026-05-18).
6. **DOS-9 + DOS-11 close fully** as work shipped in W2 per §5.3. **DOS-325 closes for the ScoreBand slice**; TrendStrip + EvidenceDrawer + entity-detail integration file to v1.4.4 W2 Entity Surfaces with explicit lineage (see §13). DOS-325 does NOT stay open after W2 — the residual work has its own tickets.
7. **CI green** on the combined W2 PR(s): `cargo clippy -p abilities-runtime --all-targets -- -D warnings` + `cargo test -p abilities-runtime --test block_kit_integration_harness` + `block-kit-integration.yml` workflow (including new §5.9 token-mapping manifest gate + new §5.8 chrome service grep gate).
8. **No edits to W1 *starter-kit code***: no edits to `wp/dailyos/scripts/new-block.mjs`, `wp/dailyos/scripts/translate-tauri.mjs`, `wp/dailyos/scripts/generate-theme-json.mjs`, `wp/dailyos/scripts/templates/`, `src-tauri/abilities-runtime/tests/block_kit_integration_harness.rs` (harness body), `wp/dailyos/tests/blocks/StarterKitIntegrationTest.php`, or `.github/workflows/block-kit-integration.yml` (workflow file itself — new CI steps are additive, not edits to the W1 step bodies). Substrate extension via the W1-spec'd paste-snippet pattern (BlockType enum variant in `composition.rs:330`, `BlockType::type_id()` arm in `composition.rs:350`, `<NAME>_FIELDS` + `<name>_rule` in `fallback_projection.rs:~1409`, `rule_for_block_type()` arm at `fallback_projection.rs:1236`, `known_projection_rules()` Vec entry at `fallback_projection.rs:1250`) IS the expected per-primitive work — emitted by `pnpm dailyos:new-block` for each primitive. The only allowed W1-side change is the §5.9 token-mapping manifest emission, filed as DOS-685 path-α if not already in W1.
9. **TypeBadge extraction sub-task §5.10 acceptance hit** before TypeBadge block translation ships.
10. **§5.8 shared primitive chrome service shipped** with both Tauri and WP implementations; no primitive carries inline empty/loading/error rendering.
11. **§5.9 token-mapping manifest emitted** for every primitive; CI gate green.

### 7.1 Visual parity matrix

Per L0 cycle 1 L6 #3: every visual state (trust bands, account types, status states, empty/loading/error variants) captured side-by-side Tauri vs WP at L4. Each cell in the table below corresponds to one screenshot pair filed in `/Users/jamesgiroux/.dailyos/l4-batch/W2/<primitive>/`.

| Primitive | Required state pairs (Tauri ↔ WP) |
|---|---|
| Pill | default, hover, active, disabled, empty, loading, error |
| HealthBadge | healthy, watching, action-needed, no-read, empty, loading, error |
| StatusDot | healthy, warning, critical, unknown, empty, loading, error |
| Avatar | with-image, initials-fallback, loading, error |
| TrustBandBadge | likely_current, use_with_caution, needs_verification, empty, loading, error |
| IntelligenceQualityBadge | high-quality, medium-quality, low-quality, no-data, loading, error, tooltip-open |
| FreshnessIndicator | fresh (<24h), aging (24h-7d), stale (>7d), unknown, loading, error |
| ProvenanceTag | with-source, age-only, tooltip-open, DOS-477 redacted-source state, loading, error |
| EntityChip | account, project, person, meeting, unknown, loading, error |
| TypeBadge | customer, internal, partner, loading, error (display-only — editable variants deferred per §6.2) |
| ScoreBand | on-track, watching, action-needed, no-read, empty, loading, error |

L4 sign-off requires every cell rendered, captured, and reviewed. CSS-value drift from theme.json token mapping is acceptable per §6.5; design-reviewer judges per-primitive.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `d_translator_idempotent` | Re-running translator on a Supported primitive that already has a `wp/dailyos/blocks/<slug>/` exits 1 (W1 invariant — refuses overwrite) |
| 2 | `d_ci_rejects_missing_fixture` | A `wp/dailyos/blocks/<slug>/` without a matching Rust fixture → CI workflow fails (W1 invariant) |
| 3 | `d_harness_catches_attr_drift` | Each primitive fixture's BindingExpectation list, when broken (rename a binding pointer to a typo), → harness emits the 4-field diagnostic for that primitive |
| 4 | `d_entity_chip_editable_refused` | `pnpm dailyos:translate-tauri --primitive EntityChip` exits 1 with NotSupported diagnostic per §6.2 |
| 5 | `d_type_badge_editable_refused` | `pnpm dailyos:translate-tauri --primitive TypeBadge` exits 1 with NotSupported diagnostic per §6.2 |
| 6 | `d_trustband_promotion_check` | After W2 PR, `.docs/design/primitives/README.md` lists TrustBandBadge under integrated (CI: grep gate) |
| 7 | `d_provenance_tag_no_disallowed_leak` | ProvenanceTag block rendered against a fixture envelope containing internal note bodies + raw source IDs + email addresses + debug carriers: none of those values appear in the rendered HTML, `data-*` attributes, REST preload state, hydration state, inspector UI, or HTML comments. Per DOS-477 + ADR-0130 §3.1 unknown-payload-leak discipline. |
| 8 | `d_token_mapping_manifest_drift` | A primitive's `style.css` containing a `var(--wp--preset--color--*)` that is NOT listed in its `.token-mapping.json` → CI workflow fails per §5.9. |
| 9 | `d_chrome_service_not_inlined` | A primitive `render-functions.php` that renders empty/loading/error inline (without `require_once _shared/chrome/render-*.php`) → CI grep gate fails per §5.8. |
| 10 | `d_scoreband_no_raw_number_in_headline` | ScoreBand block rendered against a fixture with a numeric score: no raw number string appears in the band-label headline rendered HTML (band label only — `On Track`, `Watching`, etc.). Per DOS-325 voice rule. |
| 11 | `d_typebadge_extraction_complete` | `grep -r "function AccountTypeBadge\|const AccountTypeBadge" src/` finds zero matches after the W2 TypeBadge PR (sub-task §5.10 acceptance). |

## 9. CI invariants

| # | Invariant | Enforcement |
|---|---|---|
| 1 | Every `wp/dailyos/blocks/<slug>/` has matching Rust fixture | `block-kit-integration.yml` per-block fixture gate (inherited from W1) |
| 2 | Translator parity gate runs for Pill + HealthBadge on every PR | Inherited from W1 workflow step "Translator parity gate" |
| 3 | All primitive blocks include the 5 standard composition attrs in block.json | W1 grep gate (CI invariant #3) — auto-applies via W2 |
| 4 | All primitive render-functions.php use the W1 typed error switch verbatim | W1 grep gate (CI invariant #2) — auto-applies via W2 |
| 5 | Theme.json idempotency holds after W2 blocks land | `pnpm dailyos:generate-theme-json --check` step — inherited W1 |
| 6 | TrustBandBadge + ScoreBand proposed→integrated in primitives README | New gate: grep `.docs/design/primitives/README.md` for both `TrustBandBadge.*integrated` AND `ScoreBand.*integrated` |
| 7 | Every primitive's `style.css` token references resolve through `.token-mapping.json` → `theme.json` | New gate per §5.9 — `block-kit-integration.yml` step "Token-mapping manifest" |
| 8 | No primitive `render-functions.php` contains inline empty/loading/error rendering | New gate per §5.8 — grep gate asserts `require_once.*_shared/chrome/render-(empty|loading|error)\.php` is present whenever the corresponding state branch is rendered |
| 9 | No `AccountTypeBadge` symbol remains in `src/` after W2 TypeBadge PR | New gate per §5.10 — grep gate runs on `src/**/*.tsx` |
| 10 | Every primitive's paste-snippet manifest applied (BlockType + projection rule entries exist) | New gate: per-block fixture asserts the primitive's `BlockType` variant is referenced in `composition.rs` AND its projection rule arm is present in `fallback_projection.rs:rule_for_block_type` AND its rule is registered in `known_projection_rules()` |

## 10. Interlocks

**Upstream blocker:** W1 PR #303 MUST be mergeable. W2 can branch from `w1-c1-starter-kit` for parallel development, but the W2 PR(s) must rebase onto the W1-merged `dev` before landing.

**Landing shape:** 4 PRs per the L6 ScoreBand-as-11th decision, grouped by translator-scope category:
- **PR-D1:** Pill + StatusDot + ProvenanceTag (simple-shape primitives). PR-D1 also lands the §5.8 shared primitive chrome service (one PR carries the seam; others consume it).
- **PR-D2:** HealthBadge + Avatar + FreshnessIndicator (typed-display, includes 2 of 4 hand-translation primitives).
- **PR-D3:** TrustBandBadge + IntelligenceQualityBadge + EntityChip-display + TypeBadge-display (typed-display + promotion + §5.10 TypeBadge extraction sub-task). TypeBadge extraction MUST land in this PR before the TypeBadge block.
- **PR-D4:** ScoreBand (new primitive — authoring + Tauri React source + WP block + design-spec markdown).

PR-D1 lands first (chrome service is a §5.8 hard dependency for empty/loading/error rendering in PR-D2/D3/D4). PR-D2, D3, D4 are independently mergeable in any order after PR-D1, as long as W1 has landed. Each PR brings its own fixture set + green CI + paste-snippet manifest applied per primitive.

**Cross-version interlock:** v1.4.4 surface migration cannot start until W2 PRs all merge. v1.4.4 W0 surface audit (DOS-677) can be authored in parallel. v1.4.4 W2 Entity Surfaces will consume W2 primitives + pick up DOS-325 TrendStrip + EvidenceDrawer + DOS-9/11/325 surface-behavior residue (entity envelope wiring, keyboard interaction, drawer integration).

## 11. What this packet explicitly does NOT own

- **W1 substrate changes.** Filed as W1 amendments + resolved before W2 PRs land.
- **W3 magazine theme.** Distinct work track — consumes W2 primitives via WP patterns.
- **W4 feedback write infrastructure.** Distinct work track — enables EntityChip/TypeBadge editable variants in a future iteration.
- **W5 Studio sandbox compatibility.** Distinct work track.
- **EntityChip + TypeBadge editable variants.** Display-only ships in W2; editable in W4 follow-up.
- **Composite blocks composing primitives.** v1.4.4 surface migration owns; W2 ships primitives only.
- **Cross-primitive composition patterns** (e.g., `EntityChip` + `TrustBandBadge` in one card). v1.4.4 owns.
- **Block-editor JavaScript interactivity beyond the editor reload guard** (the W1 template's reloadTrigger pattern is the contract). Primitives are render-only.

## 12. Open questions for L0 reviewers

V1.0 questions that V1.1 resolves (kept for traceability):

1. ~~**(codex challenge)**: Visual parity / token-mapping completeness.~~ **Resolved V1.1 §5.9** — token-mapping manifest gate + CI grep covers this end-to-end. Cycle 2 codex challenge should pressure-test the manifest gate itself, not the V1.0 question.
2. ~~**(codex consult)**: Per-primitive PR vs grouped PRs.~~ **Resolved V1.1 §10** — 4 PRs (D1-D4) grouped by translator-scope category; PR-D1 lands chrome service first. Cycle 2 codex consult to validate this seam (consult was never run in cycle 1).
3. ~~**(code-reviewer)**: TrustBandBadge promotion mechanic.~~ **Resolved V1.1 changelog** — PR #304 (merged 2026-05-18) made WP block consumption count as integrated; markdown promotion is the canonical path.
4. ~~**(design-reviewer)**: ScoreBand decision.~~ **Resolved V1.1 §6.4** — ScoreBand is 11th primitive; TrendStrip + EvidenceDrawer to v1.4.4.
5. ~~**(design-reviewer)**: EntityChip + TypeBadge display-only restriction.~~ **Confirmed V1.1 §6.2** — display-only in W2; editable variants defer to v1.4.4 + W4 feedback router.

Open for L0 cycle 2:

6. **(codex challenge):** §5.8 shared primitive chrome service seam — is the per-surface implementation pattern (Tauri `_chrome/` + WP `_shared/chrome/`) the right shape, or does the chrome carry enough substrate-derived semantics that it belongs in a producer? Single Rust producer is explicitly rejected in §5.8 — does that reasoning hold under adversarial review?
7. **(codex challenge):** §5.9 token-mapping manifest emission — if W1's translator does NOT currently emit `.token-mapping.json`, the W1 amendment is the only allowed starter-kit edit during W2. Is that scope drift acceptable, or should we file as a strict v1.4.3 W1 path-α blocker before W2 starts?
8. **(codex consult):** ProvenanceTag DOS-477 display-safe leak guards (§5.2 + §8 negative fixture #7) — does the render path enumerate all the field channels (DOM + data-* + REST preload + hydration state + inspector UI + HTML comments + serialized block attributes)? Memory: `feedback_enumerate_channels_before_patching.md` — get this right at L0, not at L2.
9. **(code-reviewer):** §5.10 TypeBadge extraction — is the proposed `readOnly` prop the right API, or should display-only be a separate component (`TypeBadgeDisplay` + `TypeBadgeEditable`)?
10. **(design-reviewer):** §7.1 visual parity matrix — is the state coverage per primitive complete, or are there variants (e.g., RTL, dark-mode, dense, focus) missing? Check against `.docs/design/reference/_shared/primitives.css` for each primitive.

## 13. Linear dependency edges

- W2 PR(s) close **DOS-682**.
- Upstream: DOS-678 (W1 PR #303) merged 2026-05-18.
- Folded — closing in W2:
  - **DOS-9** (cite-chip age/freshness tooltip) — closes in PR-D2 (ProvenanceTag block).
  - **DOS-11** (trust-band UI) — closes in PR-D3 (TrustBandBadge block).
  - **DOS-325** (score bands + evidence drill-down) — **partial close in PR-D4 (ScoreBand)**. The remaining DOS-325 work files to v1.4.4 with explicit lineage (next bullet).
- Filed to v1.4.4 W2 Entity Surfaces with DOS-325 lineage (NEW tickets to file at V1.1 lock):
  - **DOS-325-TrendStrip** — translate TrendStrip primitive (Tauri authoring + WP block) for entity-detail surfaces.
  - **DOS-325-EvidenceDrawer** — translate EvidenceDrawer primitive + drawer integration into entity-detail pages (on-demand, display-safe per DOS-477).
  - **DOS-325-Surface-residue** — entity-envelope wiring, keyboard interaction, evidence-drawer integration on Account/Project/Person Detail surfaces.
  - **DOS-9-entity-envelope** — cite-chip tooltip wired into entity-detail envelope (surface-tier consumption beyond the primitive itself).
  - **DOS-11-keyboard** — trust-band UI keyboard navigation + a11y in the surface composition (the primitive itself is keyboard-accessible; the surface-level navigation is v1.4.4 scope).
- W1 amendment (filed at V1.1 lock if applicable):
  - **DOS-685 path-α append** — token-mapping manifest emission in `pnpm dailyos:translate-tauri` per §5.9, if not already present.
- Downstream: every v1.4.4+ surface composes these primitives. v1.4.4 W0 surface audit (DOS-677) is the first downstream consumer.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | Why |
|---|---|---|
| `/codex challenge` | adversarial | Cycle 2 focus: §5.8 chrome service seam (single Rust producer rejected — adversarially test that), §5.9 manifest gate sufficiency, ProvenanceTag DOS-477 channel enumeration completeness. Cycle 1 AC #8 BLOCK is folded into V1.1; cycle 2 should hunt new shape of failure. |
| `/codex consult` | implementation feasibility | NEVER RAN in cycle 1 (skipped given the BLOCK). Cycle 2 must run: walk PR-D1 → PR-D4 sequence, validate chrome service lands first, validate paste-snippet manifest application is workable per primitive, validate token-mapping manifest emission is W1-feasible. |
| `code-reviewer` (claude) | domain | Cycle 2: §5.10 TypeBadge extraction API (readOnly prop vs separate component), §5.8 chrome service implementation seam, all 11 primitives' translation pattern (now with ScoreBand authoring), paste-snippet manifest correctness. |
| `design-reviewer` (claude) | design system | Cycle 2: §7.1 visual parity matrix state completeness (RTL/dark/dense/focus coverage), ScoreBand voice-rule compliance (no raw number in headline), per-primitive variant coverage against `.docs/design/reference/_shared/primitives.css`. |
| `/cso` | advisory only | No new trust boundaries; W2 blocks render-only over W1 substrate (CSO-approved). Confirm ProvenanceTag DOS-477 enforcement strategy per §5.2 + §8 fixture #7 is sufficient; no new escalation vector via the primitive block scope or chrome service. |

**Convergence rule:** unanimous APPROVE required before code lands. CONDITIONAL APPROVE folds into V1.2 (or DOS-685 / maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` if non-AC). Cycle cap: 10 cycles before L6 escalation. Path-α partial convergence allowed per `.docs/plans/v1.4.0-waves.md` precedent if same-class findings recur across cycles.

## 15. Acceptance for L0 closure

- Cycle 2 reviewer panel returns unanimous non-BLOCK across codex challenge + codex consult + code-reviewer + design-reviewer + CSO advisory.
- §12 open questions 6-10 resolved (1-5 are V1.0 carry-overs marked resolved in V1.1).
- §5.8 chrome service seam locked.
- §5.9 token-mapping manifest emission confirmed in W1 (or DOS-685 path-α append filed + accepted before W2 implementation).
- §5.10 TypeBadge extraction API locked.
- §7.1 visual parity matrix state coverage locked.
- v1.4.4 lineage tickets (DOS-325-TrendStrip, DOS-325-EvidenceDrawer, DOS-325-Surface-residue, DOS-9-entity-envelope, DOS-11-keyboard) filed in Linear with explicit pointers to this packet.

When unanimous APPROVE reached, this packet locks; W2 implementation can proceed on the basis spec'd here.
