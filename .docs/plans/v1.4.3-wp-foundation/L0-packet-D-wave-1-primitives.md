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
- **V1.2 (2026-05-18):** Folded L0 cycle 2 reviewer findings. Cycle 2 verdicts: design-reviewer CONDITIONAL APPROVE (4 findings); code-reviewer CONDITIONAL APPROVE (3 findings + 1 path-α); codex consult CONDITIONAL (2 conditions); codex challenge BLOCK (after re-dispatch with tight prompt; 2 findings); CSO dropped per James 2026-05-18 (no new trust boundaries; render-only over already-CSO-approved W0 substrate). Class-pattern sweeps triggered on §5.8 (3 findings, 3 reviewers) and §6.4 ScoreBand authoring (3 findings, 2 reviewers).
  - **§5.8 chrome service class sweep.** V1.1's "single Rust producer rejected" reasoning was wrong — `Block.render_hints` (composition.rs:473) is the existing seam, ADR-0130 contract has headroom. V1.2 retracts that reasoning; the V1.1 *conclusion* (per-surface chrome service) still holds for W2 because display-only primitives can derive chrome state surface-side at render time from `claim_refs` presence + projection error states. Producer-side `render_hints.chrome_state` hint is noted as a future option (v1.4.4 W4+) but NOT required for W2. Tauri exports renamed to `PrimitiveEmpty / PrimitiveLoading / PrimitiveError` to avoid collision with existing page-scoped `src/components/editorial/EmptyState.tsx`. Explicit no-inline-CSS rule per memory `feedback_no_inline_css.md`. Paired markdown PR added to file `_chrome/` index under `.docs/design/primitives/` so design-reviewer has an L4 anchor for the visual treatment.
  - **§6.4 ScoreBand authoring sweep.** ScoreBand vocabulary collision with HealthBadge (cycle-2 design F2) resolved by declaring ScoreBand and HealthBadge as semantically distinct primitives with explicitly-distinct vocabulary: ScoreBand = `On Track | Watching | Action Needed | No Read` (DOS-325 score-band rendering); HealthBadge = `Healthy | Watching | Action Needed | No Read` (entity health rollup). Same words for two of the four bands is intentional — both consume the same band-rendering vocabulary but bind to different substrate fields (claim-trust-derived score vs entity-health-rollup). §6.4 step 2 hardened: `.docs/design/primitives/ScoreBand.md` MUST exist and pass design-reviewer L0 sign-off BEFORE PR-D4 block translation begins (new AC sub-task §7.1).
  - **§5.2 ProvenanceTag channel enumeration completed** to include `debug panels`, `logs`, `diagnostics` per ADR-0130 §3.1 line 133 + ADR-0108 actor/surface-filtered provenance rendering (codex challenge Shape B). §8 negative fixture #7 extended to assert all 10 channels.
  - **§10 PR shape revised** to resolve codex consult's merge-conflict finding. PR-D1 carries all 11 BlockType paste-snippets (composition.rs:330 + 350 + fallback_projection.rs:1236 + 1250 + 1408) plus the chrome service plus the §5.10 TypeBadge extraction. PR-D2/D3/D4 only touch `wp/dailyos/blocks/<slug>/` + per-primitive Rust fixture files. Zero merge conflict on shared substrate files.
  - **§5.10 TypeBadge split** into two components per code-reviewer F-1. `TypeBadge` (editable, useState + useRef + useEffect + dropdown) consumed by AccountHero; `TypeBadgeDisplay` (display-only, no state, no events) consumed by the W2 block translation. Shared CSS Module. Open question §12.9 answered NO to `readOnly` prop.
  - **§5.2 row fixes.** Avatar row corrected — Avatar.module.css does NOT exist; Avatar.tsx uses inline className-from-prop. Translator must scaffold CSS from inline rules at translation time (or Avatar primitive promotion authors the CSS Module as part of PR-D2). ScoreBand row marked "to be authored in PR-D4 per §6.4 step 2" — pre-existence not implied.
  - **§7.1 matrix expanded** to add focus / hover / compact / RTL / dark-mode state columns where applicable per design F1. Explicit `N/A` markers per primitive where a state doesn't apply (e.g., StatusDot has no hover; FreshnessIndicator has no compact variant).
  - **DOS-685 path-α append** filed: token-mapping manifest emission in `pnpm dailyos:translate-tauri` per §5.9. MUST land before first W2 PR opens. Codex consult confirmed `translate-tauri.mjs:414-419, 487-489` currently does NOT emit `.token-mapping.json`.
  - **Path-α filed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`:** paste-snippet step 6 (run kit integration fixture per primitive) not enumerated in AC #8 — code-reviewer noted as path-α not blocking V1.2.
- **V1.3 (2026-05-18):** Folded L0 cycle 3 findings. Cycle 3 verdicts: codex challenge #1 (§10 PR-D1 diff size) APPROVE — PR-D1 estimated ~1,143 changed LOC vs W1 PR #303 baseline 5,879 (~19%); codex challenge #2 (§6.4 ScoreBand vocab) BLOCK — primitives README forbids substrate-binding-as-disambiguator; codex challenge #3 (§5.8 chrome test coverage) BLOCK — grep gate specified in packet but absent from workflow file; codex consult (commit boundary + DOS-685 sequencing) APPROVE — one squash-merge at landing, DOS-685 lands first, D2-D4 parallel; code-reviewer CONDITIONAL (F-A chrome service test coverage, F-B commit ordering, path-α invariant #13 typo); design-reviewer CONDITIONAL (F1 HealthBadge.md vocabulary factual error, F2 chrome service contract enumeration deferred, F3 §7.1 matrix N/A mis-categorizations).
  - **§6.4 ScoreBand vocabulary rewrite.** V1.2's "intentional collision" defense was built on a phantom: HealthBadge.md:9 actually declares `band="green" | "yellow" | "red"` plus insufficient-data — NOT `Healthy | Watching | Action Needed | No Read`. The collision V1.2 described doesn't exist on disk today. V1.3 reframe: ScoreBand introduces the band-LABEL vocabulary per DOS-325 voice rule; HealthBadge today exposes color-BAND tokens; the same DOS-325-style label-discipline pass applied to HealthBadge files as a v1.4.4 follow-up ticket (`DOS-XXX-healthbadge-label-discipline`). No co-render collision exists today; future co-render collision is prevented by either distinct labels OR the DOS-325 voice-rule pass landing first. Per cycle 3 design F1 + challenge #2.
  - **§5.8 chrome service sweep — V1.3 closes the class-pattern.** V1.1 had reasoning wrong; V1.2 fixed naming + collision but left coverage gap; V1.3 lands the test fixture spec + workflow wiring + 6-bullet non-negotiable contract list AT L0. Per cycle 3 challenge #3 BLOCK + code-reviewer F-A + design F2. Concrete additions detailed in §5.8.
  - **§10 PR-D1 commit shape synthesis.** code-reviewer F-B wanted 6 ordered sub-commits for L2 reviewability; codex consult said one squash-merge at landing (per W1 PR #303 precedent). V1.3 synthesizes: PR-D1 opens with 6 ordered development commits (paste-snippets → chrome service + design spec → TypeBadge split + AccountHero migration → ScoreBand Tauri + spec → Avatar.module.css → TrustBandBadge README promotion); merges to `dev` as squash. Best of both — reviewer walks per-concern diffs, dev history sees one entry.
  - **§7.1 matrix N/A corrections (cycle 3 design F3).** Two mis-categorizations fixed: StatusDot compact = required per `StatusDot.md:9` (`size="sm" | "md"` IS the compact axis); FreshnessIndicator compact = required per `FreshnessIndicator.md:77` (chip-shaped is the default compact render; strip variant is the standard). Notation added to clarify variant naming.
  - **§9 invariant #13 typo → maintenance.** Path-α filed to project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per cycle 3 code-reviewer; not blocking V1.3.
  - **HealthBadge label-discipline-pass ticket** filed as new v1.4.4 lineage entry in §13 (`DOS-XXX-healthbadge-label-discipline`) — applies DOS-325 voice rule to HealthBadge labels alongside its existing color-band tokens.

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
| Avatar | `src/components/ui/Avatar.tsx` | **no CSS Module exists** — Avatar.tsx uses inline `className`-from-prop pattern (cycle 2 code-reviewer F-3) | Supported scaffold — **hand-translation required + CSS Module scaffolding** | (a) Author `src/components/ui/Avatar.module.css` as part of PR-D2 promotion (extract from current inline rules); (b) TSX has `useEffect` + Tauri `invoke('resolve_avatar_source')` + inline `style={{backgroundImage: ...}}`; WP render-php must resolve avatar source via the runtime ability path, not invoke from PHP; (c) push the runtime-resolved URL through a CSS custom property `--dailyos-avatar-bg-url`, not arbitrary style attributes (memory: no inline CSS); (d) fall back to initials when source is empty via §5.8 chrome service |
| TrustBandBadge | `src/components/ui/TrustBandBadge.tsx` | `TrustBandBadge.module.css` | SupportedWithSourcePromotion | Body: `proposed`→`integrated` README PR is mechanically unblocked by PR #304 (see §6.3); translation otherwise scaffold-only |
| IntelligenceQualityBadge | `src/components/entity/IntelligenceQualityBadge.tsx` | `IntelligenceQualityBadge.module.css` | Supported scaffold — **hand-translation required** | TSX has dynamic style logic (band threshold → variant class + label text + tooltip copy). WP render-php must derive variant + label from the projected claim's quality score; the variant→token mapping table lives in render-functions.php and consumes theme.json tokens via §5.9 manifest. Tooltip render path is on-demand only (display-safe — no raw factor names per DOS-477 + DOS-325 voice rules). |
| FreshnessIndicator | `src/components/ui/FreshnessIndicator.tsx` | `FreshnessIndicator.module.css` | Supported scaffold — **hand-translation required** | TSX runs dynamic date math (`formatRelativeDateShort` on `enrichedAt`). WP render-php must compute the relative-date string server-side at render time (PHP) so client never sees raw timestamps unless DOS-477-permitted; cache TTL aligned to the freshness band (likely_current vs use_with_caution vs needs_verification thresholds). |
| ProvenanceTag | `src/components/ui/ProvenanceTag.tsx` | `ProvenanceTag.module.css` | Supported scaffold — **hand-translation required (DOS-477 critical)** | TSX renders source attribution + age + freshness chip + tooltip. WP render-php MUST consume only display-safe provenance fields per DOS-477 (no raw source-internal identifiers, no email addresses, no internal note bodies, no debug carriers). Field allowlist for ProvenanceTag payload pulled from the `ProvenanceRef`-resolved envelope's actor-filtered render projection (ADR-0108 §2). DOS-9 cite-chip age/freshness tooltip pattern folds in as the `data-age` + tooltip render path. Negative fixture #7 asserts no disallowed field reaches **any of 10 channels per ADR-0130 §3.1 line 133**: rendered DOM, `data-*` attributes, HTML comments, serialized block attributes, REST preload state, hydration state, inspector UI, **debug panels, logs, diagnostics** (V1.2 additions per cycle-2 codex challenge Shape B). |
| EntityChip | `src/components/ui/EntityChip.tsx` | `EntityChip.module.css` | Translator-NotSupported (editable variants) | Display-only ships in W2 via `pnpm dailyos:new-block --template typed-display`; editable variant deferred to v1.4.4 surface migration where the W4 feedback router lands. |
| TypeBadge | extracted from `src/components/account/AccountHero.tsx:113-172` → `src/components/ui/TypeBadge.tsx` (see §5.10) | from `AccountHero.module.css` | Translator-NotSupported (editable variants) | Display-only ships in W2; editable variant deferred to v1.4.4. Extraction sub-task spec'd in §5.10 — must land same PR as TypeBadge block. |
| ScoreBand (new) | **to be authored in PR-D4 per §6.4 step 2** — `src/components/ui/ScoreBand.tsx` + `.module.css` + `.docs/design/primitives/ScoreBand.md` design spec MUST land in the same PR | new | New primitive (DOS-325 fold) | Sequence: (1) author `.docs/design/primitives/ScoreBand.md` design spec referencing DOS-325 source material + band vocabulary justification per V1.2 changelog; (2) author Tauri primitive (band-label only, no raw number in headline per DOS-325 voice rule); (3) translate to WP block via §5.1 default path. Consumes claim trust band + entity-intelligence envelope. TrendStrip + EvidenceDrawer NOT in W2 — file to v1.4.4 with explicit DOS-325 lineage (see §13). |

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

Per L0 cycle 1 L6 #3 + cycle 2 class sweep (design F4 + code F-2 + codex challenge Shape A): every primitive that renders against substrate data has empty / loading / error states. V1.0 implicitly let each primitive re-implement these; V1.1 centralized them as a service; V1.2 corrects the reasoning + naming + design-system anchoring.

**Producer-vs-surface boundary (V1.2 correction).** V1.1 claimed a single Rust producer was rejected because it would change the `Composition`/`Block` payload contract. That reasoning was wrong — `Block.render_hints` (surface-neutral hints) already exists at `src-tauri/abilities-runtime/src/abilities/composition.rs:473` per ADR-0130 §2, and could carry `chrome_state: Loading | Error | Empty | Ready` without contract change. The reasoning to retain per-surface implementation is different:

- **For W2 (display-only primitives):** chrome state is derived **surface-side at render time** from `claim_refs` presence + projection error states. The producer doesn't need to set a hint; the renderer sees the projected payload and classifies. This keeps W2 simple and consistent with the read-path render contract.
- **For v1.4.4 W4 (editable / write-bound primitives):** the producer may want to set `render_hints.chrome_state` explicitly to distinguish "not yet computed (loading)" from "computed and empty" from "errored." That's a future option, not W2 scope. ADR-0130 contract has the headroom; v1.4.4 W4 can adopt without contract change.

**Authoring seam (per-surface implementation, name-collision-free).** Token-driven, surface-agnostic semantics; matched implementations on both sides of the W1 translator:
- **Tauri side:** `src/components/ui/_chrome/PrimitiveChrome.tsx` exporting `<PrimitiveEmpty />`, `<PrimitiveLoading />`, `<PrimitiveError />` consumed by all 11 primitive components. **Renamed in V1.2** from `<EmptyState />` etc. to avoid collision with the existing page-scoped `src/components/editorial/EmptyState.tsx` (cycle 2 code-reviewer F-2). The editorial EmptyState is page-scoped (h2 + paragraph + buttons, inline-style-heavy at lines 33-110) and is NOT primitive chrome — primitives must NOT consume it. CSS Module driven by design tokens; **no inline styles** (cardinal rule per memory `feedback_no_inline_css.md`).
- **WP side:** `wp/dailyos/blocks/_shared/chrome/` with `render-empty.php`, `render-loading.php`, `render-error.php` partials that primitive `render-functions.php` files `require_once`. Token consumption via theme.json variables only (per §5.9 manifest gate).

**Design-system anchoring (V1.2 + V1.3 hardened).** Per cycle 2 design F4 + cycle 3 design F2, the chrome service is design-system net-new — no upstream EmptyState/LoadingState/ErrorState pattern exists in `.docs/design/patterns/`. PR-D1 ships a paired markdown PR adding `.docs/design/primitives/_chrome/README.md`.

**Non-negotiable spec contract (V1.3 — enumerated at L0, not deferred to PR-D1 prose):** the `_chrome/README.md` MUST cover all six items below. Design-reviewer L4 sign-off cannot pass with any item missing:

1. **Named theme.json palette entries consumed.** Specific palette slug list (not "tokens"), e.g., `wp--preset--color--neutral-100`, `wp--preset--color--neutral-300`, `wp--preset--color--danger-fill`. Each entry pinned by name.
2. **Skeleton density + spacing tokens.** Specific spacing/sizing tokens (not "visual treatment"), e.g., `wp--preset--spacing--40` for loading-skeleton bar spacing, `wp--custom--radius--sm` for skeleton corner radius.
3. **Explicit "MUST NOT consume `src/components/editorial/EmptyState.tsx`" boundary.** Page-scoped editorial empty state is a separate primitive; primitives MUST NOT import it. CI grep gate at fixture #12.
4. **Focus management for empty/error CTA targets.** When chrome renders an actionable element (e.g., "Retry" button in error state), focus management contract spec'd. For W2 display-only primitives without CTAs, this section reads "N/A for display-only primitives; defer to v1.4.4 W4 when interactive chrome lands."
5. **RTL + dark-mode coverage.** Same axes §7.1 demands of the primitives themselves. Chrome MUST render correctly under both.
6. **When to escalate to `editorial/EmptyState`.** Decision criteria: full-surface empty vs primitive-slot empty. E.g., entity-detail page with zero data → `editorial/EmptyState`; single ProvenanceTag with no source → primitive chrome.

**Test coverage (V1.3 — closes cycle 3 challenge #3 BLOCK + code-reviewer F-A):** the W1 harness today only asserts non-empty HTML on the projected composition. V1.3 adds **dedicated chrome service test fixtures** that drive a primitive through each chrome state branch and assert the shared partial output:

- **Rust harness extension** — `src-tauri/abilities-runtime/tests/chrome_service_integration_fixture.rs` (new). Drives the harness with four fixture compositions per primitive: (a) ready state with full payload; (b) loading state with empty `claim_refs` + `render_hints.chrome_state=Loading` if/when adopted, else surface-side derived; (c) empty state with empty `claim_refs` and resolved-to-no-data projection; (d) error state with projection error. For each state, the fixture asserts the rendered HTML contains the corresponding chrome partial marker (e.g., `data-chrome="empty"` set by `render-empty.php`) and does NOT contain the marker for any other state.
- **WP-side standalone test** — `wp/dailyos/tests/blocks/chrome/ChromeServiceTest.php` (new). PHPUnit test that invokes each `render-(empty|loading|error).php` partial directly with a minimal fixture context, asserts the rendered HTML matches a snapshot, and verifies no inline `style=` attribute appears in the output (per memory `feedback_no_inline_css.md`).
- **Workflow gate wiring (V1.3 fixes the absent gate cycle 3 challenge #3 caught)** — `.github/workflows/block-kit-integration.yml` adds a new step: "Chrome service coverage" that runs `cargo test -p abilities-runtime --test chrome_service_integration_fixture` AND `pnpm phpunit wp/dailyos/tests/blocks/chrome/` AND a grep gate over `wp/dailyos/blocks/<slug>/render-functions.php` asserting `require_once.*_shared/chrome/render-(empty|loading|error)\.php` is present whenever the corresponding state branch is handled.

**Acceptance:** no primitive's `render-functions.php` contains an inline empty/loading/error rendering path. The chrome state-branch tests pass for every primitive. The workflow gate runs on every PR touching `wp/dailyos/blocks/`. New CI invariants §9.14 + §9.15.

**Cycle 4 review focus** (if needed): chrome service contract sufficiency for v1.4.4 W4 editable-variant adoption, when producer-side `render_hints.chrome_state` is added.

### 5.9 Token-mapping manifest gate

Per L0 cycle 1 L6 #3 + design-reviewer cycle-1 finding. The W1 translator changed CSS variable names from `--color-spice-turmeric` to `var(--wp--preset--color--spice-turmeric)` via theme.json mapping (V1.0 open question #1). V1.1 closes this by requiring the translator to emit a per-primitive token-mapping manifest and gating CI on round-trip resolution.

**Manifest shape.** For each primitive, `pnpm dailyos:translate-tauri --primitive <Name>` emits `wp/dailyos/blocks/<slug>/.token-mapping.json` listing every `var(--wp--preset--color--*)` (and other WP-mapped tokens) used in `style.css`, paired with its source `--color-*` (or other token) name in the canonical Tauri CSS Module.

**CI gate** (new — `block-kit-integration.yml` step "Token-mapping manifest"):
1. For each `wp/dailyos/blocks/<slug>/.token-mapping.json`, assert every `wp--preset--color--*` entry resolves to a defined color in `wp/dailyos/theme/theme.json` settings.color.palette.
2. For each `var(--wp--preset--color--*)` in `wp/dailyos/blocks/<slug>/style.css`, assert it is listed in `.token-mapping.json`.
3. Grep gate: no raw color literal (`#`, `rgb(`, `hsl(`, named CSS color) in `wp/dailyos/blocks/<slug>/style.css` outside the manifest's allowed escape list.

**W1 amendment if generator does not emit manifest.** If `pnpm dailyos:translate-tauri` does not currently emit `.token-mapping.json`, file as DOS-685 path-α append (W1 starter-kit maintenance) and resolve before the first W2 PR lands. **This is the only allowed W1 starter-kit code edit during W2** — and only because the manifest is part of the translator's contract, not a per-primitive concern.

### 5.10 TypeBadge extraction + split (sub-task, lands in PR-D1)

Per L0 cycle 1 + cycle 2 findings (code-reviewer F-1 + open question §12.9 answered NO to `readOnly`): TypeBadge currently lives inline in `src/components/account/AccountHero.tsx:113-172` as `AccountTypeBadge`. The primitives README + wave plan §182 already specify TypeBadge as a primitive in its own right. V1.2 lands the extraction in PR-D1 (alongside all other substrate paste-snippets per §10), with the extraction split into two components rather than one `readOnly`-flagged component.

**Why split (cycle 2 code F-1):** the editable component is ~50 LOC of dropdown state machinery (`useState(open)`, click-outside `useEffect`, dropdown render). A `readOnly` prop forces the WP block translator to translate a component whose entire interactivity body is dead under W2 usage (display-only per §6.2). It also forces AccountHero's call site to pass `onChange` against a primitive whose contract says "ignored when readOnly." Two primitives, one job each — AccountHero imports the editable one, W2 block translates the display-only one (clean simple-template scaffold, no dropdown body).

**Sub-task acceptance:**
1. Create `src/components/ui/TypeBadgeDisplay.tsx` — display-only primitive. Props: `value: 'customer' | 'internal' | 'partner'`. No state, no events, no `useState`/`useRef`/`useEffect`/`ChevronDown`/dropdown. Renders the labeled badge for the given type.
2. Create `src/components/ui/TypeBadge.tsx` — editable primitive. Composes `TypeBadgeDisplay` for the visual + adds dropdown state machinery + `onChange` event. Props: `value`, `onChange: (v) => void`. The dropdown body is THIS component's responsibility, not TypeBadgeDisplay's.
3. Create `src/components/ui/TypeBadge.module.css` — shared CSS Module owning both components' styles (extracted from `AccountHero.module.css` lines 23-59 + 142-203 per cycle 2 codex consult).
4. Migrate `src/components/account/AccountHero.tsx` — `import TypeBadge from '@/components/ui/TypeBadge'`; remove the inline `AccountTypeBadge` definition + the `ACCOUNT_TYPES` table; remove `useState`, `useRef`, `useEffect`, `ChevronDown` imports if no longer used in AccountHero.
5. Move the listed CSS classes (`badge`, `customerBadge`, `internalBadge`, `partnerBadge`, `typeBadgeWrapper`, `typeBadgeButton`, `typeBadgeChevron`, `typeBadgeDropdown`, `typeBadgeOption`, `typeBadgeOptionActive`, `typeBadgeOptionCustomer/Internal/Partner`) from `AccountHero.module.css` into `TypeBadge.module.css`.
6. Confirm no other call sites reference the inline name: `grep -r "AccountTypeBadge" src/` returns zero matches.
7. `cargo clippy -- -D warnings && pnpm tsc --noEmit && pnpm test` green; AccountHero still renders the editable TypeBadge correctly at L4.
8. PR-D3 (W2 TypeBadge block) translates `TypeBadgeDisplay` — not `TypeBadge` — to a `wp/dailyos/blocks/type-badge/` block. PR-D3 is independent of the extraction (which lands in PR-D1) and only touches the WP block dir + per-primitive Rust fixture.

**Why this lands in PR-D1, not PR-D3 (V1.2 change from V1.1):** per cycle 2 codex consult merge-conflict finding, all shared-Rust paste-snippets + substrate-side Tauri primitives + chrome service land in PR-D1 to eliminate merge conflicts across parallel PRs. TypeBadge extraction is a Tauri-side primitive change; it belongs with the substrate-extension batch. PR-D3 only touches `wp/dailyos/blocks/type-badge/` + `src-tauri/abilities-runtime/tests/fixtures/type_badge_integration_fixture.rs`.

## 6. Directional decisions resolved at L0

### 6.1 Single batched packet vs per-primitive packets

Single batched L0 packet (this one). Per-primitive packets would 10x the L0 review burden with diminishing returns — the translation pattern is identical across primitives. Per-primitive divergence handled in this packet's §5.2.

### 6.2 EntityChip + TypeBadge: display-only in W2

Both have editable variants per .docs/design/primitives/. Editable variants need the W4 feedback-write infrastructure (click-bound router + nonce). W2 ships display-only; editable variants defer to W4 follow-up tickets.

### 6.3 TrustBandBadge proposed→integrated promotion

Per primitives README, TrustBandBadge is currently `proposed`. The W2 WP block is the first integrated consumer; promote to `integrated` in the same PR (markdown-only doc change to README + create `.docs/design/primitives/TrustBandBadge.md` proper if missing). Per primitives README "Adding a primitive that already exists in src/", promotion is markdown-PR only — no code consolidation required.

### 6.4 ScoreBand: 11th primitive, W2 takes ScoreBand only, authoring sequence locked

Resolved at L0 cycle 1 L6 (James, 2026-05-18): ScoreBand is a distinct visual primitive (band-label with no raw number in the headline, per DOS-325 voice rule). W2 takes ScoreBand only. The remaining two DOS-325 primitives (TrendStrip + EvidenceDrawer) file to v1.4.4 W2 Entity Surfaces with explicit DOS-325 lineage (see §13).

**Vocabulary (V1.3 rewrite — V1.2's "intentional collision" defense was built on a phantom):** HealthBadge.md:9 actually declares `band="green" | "yellow" | "red"` plus insufficient-data — NOT `Healthy | Watching | Action Needed | No Read`. The collision V1.2 described doesn't exist on disk today. The corrected framing:

- **ScoreBand introduces** the band-LABEL vocabulary `On Track | Watching | Action Needed | No Read` per DOS-325 issue body §"What good looks like": "renders a plain-language band label. No raw number in the headline." This is the FIRST primitive to apply DOS-325 voice rule to band rendering. ADR-0083 product vocabulary applies.
- **HealthBadge today exposes color-BAND tokens** (`green` / `yellow` / `red` / insufficient-data) per `HealthBadge.md:9`. No label vocabulary at all — the labels are caller-supplied or surface-rendered above the primitive.
- **The same DOS-325-style label-discipline pass** applied to HealthBadge files as a v1.4.4 follow-up ticket: `DOS-XXX-healthbadge-label-discipline` (see §13). That ticket folds DOS-325 voice rule into HealthBadge's spec; the vocabulary it lands on is a v1.4.4 decision, not v1.4.3 W2 scope.
- **Co-render collision risk** in a future v1.4.4 entity-detail mockup (account HealthBadge + per-claim ScoreBand on the same page): prevented by the HealthBadge label-discipline pass landing first OR by HealthBadge picking distinct labels at that pass. No collision exists today because HealthBadge has no label vocabulary today.

Primitives README discipline (`.docs/design/primitives/README.md:3-5`) is respected: ScoreBand stands on its labels + visuals (not "we have substrate-binding distinguishing us from HealthBadge" which V1.2 wrongly invoked). When `DOS-XXX-healthbadge-label-discipline` lands, that ticket owns the question of whether HealthBadge picks the same four labels or distinct ones — that's a design-system decision at the v1.4.4 boundary, not a v1.4.3 W2 question.

**Authoring sequence in PR-D4 (V1.2 — hardened):**
1. **`.docs/design/primitives/ScoreBand.md` design spec MUST exist + pass design-reviewer L0 sign-off BEFORE block translation** (new AC §7 step). Spec contents: band-vocabulary justification per V1.2 above; when-to-use vs HealthBadge boundary discipline (substrate-binding rule above); token consumption; visual treatment; evidence-drawer integration deferred-to-v1.4.4 note.
2. Add to `.docs/design/primitives/README.md` Wave 1 table marking ScoreBand `proposed` initially; promote `proposed`→`integrated` at WP block consumption per PR #304 design vocab.
3. Author Tauri React primitive at `src/components/ui/ScoreBand.tsx` + `ScoreBand.module.css`. Band labels only (per #1 spec). Consumes the claim's trust-band-derived score from the entity-intelligence envelope, not raw factor values.
4. Translate to WP block via `pnpm dailyos:new-block ScoreBand --template typed-display`.
5. Apply paste-pattern substrate touches per AC #8 — BlockType variant + projection rule. Per §10, these substrate paste-snippets land in PR-D1 (not PR-D4) to avoid merge conflicts; PR-D4 only touches `wp/dailyos/blocks/score-band/` + per-primitive Rust fixture + the new ScoreBand Tauri files + the design spec markdown.
6. Fixture + L4 parity proof per §7.1 matrix row.

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
12. **`.docs/design/primitives/ScoreBand.md` design spec authored + design-reviewer L0 sign-off** BEFORE PR-D4 block translation begins (V1.2 — per cycle 2 design F3 + §6.4 step 1).
13. **`.docs/design/primitives/_chrome/README.md` paired markdown spec authored + design-reviewer L4 sign-off** as part of PR-D1 chrome service shipping (V1.2 — per cycle 2 design F4 + §5.8 design-system anchoring).
14. **DOS-685 path-α append landed BEFORE first W2 PR opens** — token-mapping manifest emission in `pnpm dailyos:translate-tauri` per §5.9. Codex consult confirmed missing; this is the only allowed W1-side edit during W2.
15. **Chrome service state-branch test coverage** (V1.3 — closes cycle 3 challenge #3 BLOCK + code-reviewer F-A): `chrome_service_integration_fixture.rs` + `ChromeServiceTest.php` exist and pass; workflow gate "Chrome service coverage" wired into `.github/workflows/block-kit-integration.yml` and runs on every PR touching `wp/dailyos/blocks/`.
16. **`.docs/design/primitives/_chrome/README.md` carries all 6 non-negotiable contract items** (V1.3 — per cycle 3 design F2 enumeration locked at L0): named theme.json palette entries; skeleton density + spacing tokens; "MUST NOT consume editorial/EmptyState" boundary; focus management contract; RTL + dark-mode coverage; escalation criteria to editorial/EmptyState.

### 7.1 Visual parity matrix

Per L0 cycle 1 L6 #3 + cycle 2 design F1: every visual state captured side-by-side Tauri vs WP at L4 across two axes — **semantic** (claim-state-driven) and **interaction/layout** (UA-driven). Each cell corresponds to one screenshot pair filed in `/Users/jamesgiroux/.dailyos/l4-batch/W2/<primitive>/`.

**Axis 1: Semantic state** (claim/trust-band/type-derived)

| Primitive | Required semantic-state pairs (Tauri ↔ WP) |
|---|---|
| Pill | default, active, disabled, empty, loading, error |
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

**Axis 2: Interaction / layout state** (V1.2 added per cycle 2 design F1)

| Primitive | Focus | Hover | Compact / dense | RTL | Dark mode |
|---|---|---|---|---|---|
| Pill | required (interactive in some contexts) | required | N/A | required | required |
| HealthBadge | N/A (display-only) | N/A | N/A | required | required |
| StatusDot | N/A | N/A | **required (`size="sm"` variant per `StatusDot.md:9` + `StatusDot.tsx:4`; V1.3 correction)** | required (mirrors only if used with text) | required |
| Avatar | N/A (display-only in W2) | N/A | required (compact size token) | N/A (square) | required |
| TrustBandBadge | required (interactive when paired with tooltip) | required (tooltip trigger) | **required** (per `.docs/design/primitives/TrustBandBadge.md:35`) | required | required |
| IntelligenceQualityBadge | required (tooltip trigger) | required (tooltip) | N/A | required | required |
| FreshnessIndicator | N/A | N/A | **required — chip = default compact render per `FreshnessIndicator.md:77`; strip variant = standard render (V1.3 correction)** | required | required |
| ProvenanceTag | required (tooltip trigger) | required (tooltip) | N/A | required | required |
| EntityChip | required (link in some contexts) | required | N/A | required (mirrors entity-type icon) | required |
| TypeBadge (display-only) | N/A | N/A | N/A | required | required |
| ScoreBand | N/A (display-only in W2; editable in v1.4.4 with EvidenceDrawer) | N/A | required (compact variant — per ScoreBand.md spec from §6.4) | required | required |

L4 sign-off requires every non-N/A cell across both axes rendered, captured, and reviewed. CSS-value drift from theme.json token mapping is acceptable per §6.5; design-reviewer judges per-primitive. `N/A` markers are explicit, not implicit — a primitive marked N/A for a state MUST have a corresponding "N/A — primitive does not have this state" entry in the L4 batch directory so the matrix is verifiably complete.

## 8. Negative fixtures

| # | Fixture | Asserts |
|---|---|---|
| 1 | `d_translator_idempotent` | Re-running translator on a Supported primitive that already has a `wp/dailyos/blocks/<slug>/` exits 1 (W1 invariant — refuses overwrite) |
| 2 | `d_ci_rejects_missing_fixture` | A `wp/dailyos/blocks/<slug>/` without a matching Rust fixture → CI workflow fails (W1 invariant) |
| 3 | `d_harness_catches_attr_drift` | Each primitive fixture's BindingExpectation list, when broken (rename a binding pointer to a typo), → harness emits the 4-field diagnostic for that primitive |
| 4 | `d_entity_chip_editable_refused` | `pnpm dailyos:translate-tauri --primitive EntityChip` exits 1 with NotSupported diagnostic per §6.2 |
| 5 | `d_type_badge_editable_refused` | `pnpm dailyos:translate-tauri --primitive TypeBadge` exits 1 with NotSupported diagnostic per §6.2 |
| 6 | `d_trustband_promotion_check` | After W2 PR, `.docs/design/primitives/README.md` lists TrustBandBadge under integrated (CI: grep gate) |
| 7 | `d_provenance_tag_no_disallowed_leak` | ProvenanceTag block rendered against a fixture envelope containing internal note bodies + raw source IDs + email addresses + debug carriers: none of those values appear in **any of 10 channels per ADR-0130 §3.1 line 133** — (1) rendered HTML, (2) `data-*` attributes, (3) HTML comments, (4) serialized block attributes, (5) REST preload state, (6) hydration state, (7) inspector UI, (8) debug panels, (9) logs, (10) diagnostics. **V1.2 expanded from 7 to 10 channels** per cycle-2 codex challenge Shape B. Per DOS-477 + ADR-0108 actor/surface-filtered provenance rendering. |
| 8 | `d_token_mapping_manifest_drift` | A primitive's `style.css` containing a `var(--wp--preset--color--*)` that is NOT listed in its `.token-mapping.json` → CI workflow fails per §5.9. |
| 9 | `d_chrome_service_not_inlined` | A primitive `render-functions.php` that renders empty/loading/error inline (without `require_once _shared/chrome/render-*.php`) → CI grep gate fails per §5.8. |
| 10 | `d_scoreband_no_raw_number_in_headline` | ScoreBand block rendered against a fixture with a numeric score: no raw number string appears in the band-label headline rendered HTML (band label only — `On Track`, `Watching`, etc.). Per DOS-325 voice rule. |
| 11 | `d_typebadge_extraction_complete` | After PR-D1 lands: `grep -r "function AccountTypeBadge\|const AccountTypeBadge" src/` finds zero matches; `src/components/ui/TypeBadgeDisplay.tsx` + `src/components/ui/TypeBadge.tsx` + `src/components/ui/TypeBadge.module.css` exist; AccountHero imports the editable `TypeBadge`. Per §5.10 V1.2 split. |
| 12 | `d_chrome_no_editorial_state_collision` | `grep -rn "from.*editorial/EmptyState" src/components/ui/_chrome/ wp/dailyos/blocks/` finds zero matches (cycle 2 code F-2 — primitive chrome MUST NOT consume page-scoped `editorial/EmptyState`). |
| 13 | `d_scoreband_md_exists_before_block` | PR-D4 CI fails if `.docs/design/primitives/ScoreBand.md` is missing or `wp/dailyos/blocks/score-band/` lands without the design spec already on `dev` (per §6.4 step 1 + new AC #12). |
| 14 | `d_chrome_md_exists_with_pr_d1` | PR-D1 CI fails if `wp/dailyos/blocks/_shared/chrome/` or `src/components/ui/_chrome/` lands without `.docs/design/primitives/_chrome/README.md` (per §5.8 design-system anchoring + new AC #13). |

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
| 9 | No `AccountTypeBadge` symbol remains in `src/` after PR-D1 | New gate per §5.10 — grep gate runs on `src/**/*.tsx` |
| 10 | Every primitive's paste-snippet manifest applied (BlockType + projection rule entries exist) | New gate: per-block fixture asserts the primitive's `BlockType` variant is referenced in `composition.rs` AND its projection rule arm is present in `fallback_projection.rs:rule_for_block_type` AND its rule is registered in `known_projection_rules()` |
| 11 | `.docs/design/primitives/ScoreBand.md` exists before any `wp/dailyos/blocks/score-band/` files land | New gate per §6.4 step 1 + AC #12 — CI workflow asserts spec markdown precedes block dir |
| 12 | `.docs/design/primitives/_chrome/README.md` exists when `_chrome/` or `_shared/chrome/` directories land | New gate per §5.8 design-system anchoring + AC #13 |
| 13 | DOS-685 path-α landed on dev BEFORE PR-D1 opens (manifest emission in translate-tauri.mjs) | Pre-PR gate per §5.9 + AC #14 |
| 14 | Chrome service state-branch tests pass | `cargo test -p abilities-runtime --test chrome_service_integration_fixture` + `pnpm phpunit wp/dailyos/tests/blocks/chrome/` green; new workflow step "Chrome service coverage" per §5.8 V1.3 test coverage + AC #15 |
| 15 | `.docs/design/primitives/_chrome/README.md` non-negotiable 6-item contract complete | CI grep gate asserts spec file contains all 6 contract headings per §5.8 V1.3 + AC #16 |

## 10. Interlocks

**Upstream blocker:** W1 PR #303 MUST be mergeable. W2 can branch from `w1-c1-starter-kit` for parallel development, but the W2 PR(s) must rebase onto the W1-merged `dev` before landing.

**Landing shape (V1.2 — restructured to eliminate merge-conflict risk per cycle 2 codex consult finding):**

PR-D1 is the **shared-substrate + foundation** PR. It carries everything that touches files outside `wp/dailyos/blocks/<slug>/` or `src-tauri/abilities-runtime/tests/fixtures/<slug>_integration_fixture.rs`. PR-D2/D3/D4 are then **disjoint per-primitive PRs** that only edit their own block directory + their own integration fixture file. Result: zero merge conflicts on shared substrate files.

**PR-D1 — foundation + shared substrate (lands first; everything else gates on it):**
- All 11 BlockType paste-snippets applied to `src-tauri/abilities-runtime/src/abilities/composition.rs` (lines 330 + 350 per `new-block.mjs:121-125`) and `fallback_projection.rs` (lines 1236 + 1250 + 1408 per `new-block.mjs:222-251`). One commit, one diff, one merge.
- Chrome service (§5.8): `src/components/ui/_chrome/PrimitiveChrome.tsx` + `wp/dailyos/blocks/_shared/chrome/render-{empty,loading,error}.php` + `.docs/design/primitives/_chrome/README.md` design spec.
- TypeBadge extraction + split (§5.10): `src/components/ui/TypeBadgeDisplay.tsx` + `src/components/ui/TypeBadge.tsx` + `src/components/ui/TypeBadge.module.css` + AccountHero migration. **AccountHero stays Tauri-side; no WP block in this PR.**
- ScoreBand Tauri-side authoring (§6.4 + §5.2 row 11): `src/components/ui/ScoreBand.tsx` + `ScoreBand.module.css` + `.docs/design/primitives/ScoreBand.md` design spec + `.docs/design/primitives/README.md` Wave 1 table updated. **No WP block in this PR.**
- Avatar CSS Module scaffolding (§5.2 row 4): `src/components/ui/Avatar.module.css` extracted from current inline rules.
- TrustBandBadge promotion in `.docs/design/primitives/README.md` from `proposed` → `integrated` (PR #304 design vocab makes this legitimate).

**PR-D2 — simple-shape primitive blocks** (disjoint from D1; touches only block dirs + fixtures):
- `wp/dailyos/blocks/pill/` + `pill_integration_fixture.rs`
- `wp/dailyos/blocks/status-dot/` + `status_dot_integration_fixture.rs`
- `wp/dailyos/blocks/provenance-tag/` + `provenance_tag_integration_fixture.rs` (DOS-477 critical per §5.2)

**PR-D3 — typed-display primitive blocks** (disjoint from D1/D2):
- `wp/dailyos/blocks/health-badge/` + fixture
- `wp/dailyos/blocks/avatar/` + fixture (consumes Avatar.module.css from D1)
- `wp/dailyos/blocks/freshness-indicator/` + fixture
- `wp/dailyos/blocks/trust-band-badge/` + fixture
- `wp/dailyos/blocks/intelligence-quality-badge/` + fixture

**PR-D4 — composed + new primitive blocks** (disjoint from D1/D2/D3):
- `wp/dailyos/blocks/entity-chip/` + fixture
- `wp/dailyos/blocks/type-badge/` + fixture (translates `TypeBadgeDisplay` from D1)
- `wp/dailyos/blocks/score-band/` + fixture (translates ScoreBand from D1)

**Ordering:** PR-D1 lands first (hard dependency for chrome service + paste-snippets + per-block Tauri primitives). PR-D2, D3, D4 can land in any order or in parallel after PR-D1. Each PR brings its own per-block fixture set + green CI + per-block style.css + per-block `.token-mapping.json` (per §5.9).

**PR-D1 commit shape (V1.3 — synthesis of cycle 3 code-reviewer F-B + codex consult APPROVE).** PR-D1 opens with **6 ordered development commits** so reviewers walk per-concern diffs; the PR merges to `dev` as a **single squash-merge** so dev history sees one entry. Best of both — review granularity during cycle, atomic landing boundary.

Ordered development commits in PR-D1:
1. **Paste-snippet substrate touches.** 11 `BlockType` variants in `composition.rs` (lines 330 + 350) + 11 `<NAME>_FIELDS` + `<name>_rule` + `rule_for_block_type` arm + `known_projection_rules` Vec entry in `fallback_projection.rs` (lines 1236 + 1250 + 1408). Estimated +242 LOC per cycle 3 challenge #1.
2. **Chrome service + design spec.** `src/components/ui/_chrome/PrimitiveChrome.tsx` + `wp/dailyos/blocks/_shared/chrome/render-{empty,loading,error}.php` + `.docs/design/primitives/_chrome/README.md` (with 6-bullet contract per §5.8) + chrome service test fixtures (`chrome_service_integration_fixture.rs` + `ChromeServiceTest.php`) + workflow gate addition. Estimated +319 LOC.
3. **TypeBadge split + AccountHero migration.** `src/components/ui/TypeBadgeDisplay.tsx` + `src/components/ui/TypeBadge.tsx` + `src/components/ui/TypeBadge.module.css` + AccountHero migration. Estimated +198 / -159 LOC per cycle 3 challenge #1.
4. **ScoreBand Tauri authoring + design spec.** `src/components/ui/ScoreBand.tsx` + `ScoreBand.module.css` + `.docs/design/primitives/ScoreBand.md` (DOS-325 voice-rule vocabulary justification + when-to-use vs HealthBadge boundary per §6.4 V1.3 framing) + `.docs/design/primitives/README.md` Wave 1 table update. Estimated +165 / -1 LOC.
5. **Avatar CSS Module scaffolding.** `src/components/ui/Avatar.module.css` extracted from current inline rules per §5.2 row 4. Estimated +57 LOC.
6. **TrustBandBadge README promotion.** `.docs/design/primitives/README.md` updated to mark TrustBandBadge `integrated` per §6.3 (PR #304 design vocab makes this legitimate). Estimated +1 / -1 LOC.

PR-D1 total estimate: ~+982 / -161 = ~1,143 changed LOC per cycle 3 challenge #1 calculation, vs W1 PR #303 baseline of 5,879 (~19%). Merge-conflict elimination is worth the size — cycle 3 challenge #1 APPROVE.

**Pre-W2 gate (V1.2 hard requirement):** **DOS-685 path-α append MUST land before PR-D1 opens** — the token-mapping manifest emission in `pnpm dailyos:translate-tauri` is required by §5.9 CI gates, and §5.9 says it's the only allowed W1-side edit during W2. Filing DOS-685 maintenance ticket is a §15 closure precondition.

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

V1.0 + V1.1 cycle-2 questions (kept for traceability — all resolved in V1.2):

6. ~~**(codex challenge)** §5.8 chrome service seam.~~ **Resolved V1.2 §5.8** — V1.1's reasoning was wrong (`Block.render_hints` is the existing seam, no contract change needed). V1.1's conclusion (per-surface chrome service) still holds for W2 because display-only primitives derive state surface-side. Producer-side hint is a v1.4.4 W4+ option.
7. ~~**(codex challenge)** §5.9 token-mapping manifest emission scope drift.~~ **Resolved V1.2 §13** — DOS-685 path-α append filed; MUST land before PR-D1 opens (new pre-W2 gate, AC #14).
8. ~~**(codex consult)** ProvenanceTag DOS-477 channel enumeration.~~ **Resolved V1.2 §5.2 + §8 fixture #7** — expanded from 7 to 10 channels per ADR-0130 §3.1 line 133 (added debug panels + logs + diagnostics).
9. ~~**(code-reviewer)** §5.10 TypeBadge API — readOnly vs separate components.~~ **Resolved V1.2 §5.10** — answer is NO to readOnly; split into `TypeBadge` (editable) + `TypeBadgeDisplay` (display-only).
10. ~~**(design-reviewer)** §7.1 matrix state coverage.~~ **Resolved V1.2 §7.1** — added Axis 2 (focus / hover / compact / RTL / dark-mode) with explicit N/A markers per primitive.

Open for L0 cycle 3:

11. **(codex challenge cycle 3):** §10 PR-D1-carries-everything strategy — does PR-D1's scope (all 11 substrate paste-snippets + chrome service + TypeBadge extraction + ScoreBand Tauri authoring + Avatar CSS Module + TrustBandBadge promotion) create a single-PR-too-large risk? Or is the merge-conflict elimination worth the bigger diff?
12. **(codex consult cycle 3):** PR-D1 sequencing — is filing DOS-685 + landing it BEFORE PR-D1 opens the right ordering, or should PR-D1 absorb the manifest emission as part of its own commits (and accept the "only allowed W1-side edit" framing applies retroactively)?
13. **(design-reviewer cycle 3):** ScoreBand vocabulary justification (V1.2 §6.4) — the intentional overlap with HealthBadge on two of four bands. Is the substrate-binding distinction (entity-health-rollup vs single-claim-score) enough to keep the names co-located, or does the vocabulary collision still create confusion in the design system?
14. **(code-reviewer cycle 3):** §5.8 chrome service test coverage — V1.2 retracted V1.1's reasoning but kept the conclusion. Does the per-surface implementation get adequate test coverage from the W1 harness, or are new tests needed for the chrome service itself (separate from per-primitive integration fixtures)?

## 13. Linear dependency edges

- W2 PR(s) close **DOS-682**.
- Upstream: DOS-678 (W1 PR #303) merged 2026-05-18.
- Folded — closing in W2:
  - **DOS-9** (cite-chip age/freshness tooltip) — closes in PR-D2 (ProvenanceTag block).
  - **DOS-11** (trust-band UI) — closes in PR-D3 (TrustBandBadge block).
  - **DOS-325** (score bands + evidence drill-down) — **partial close in PR-D4 (ScoreBand)**. The remaining DOS-325 work files to v1.4.4 with explicit lineage (next bullet).
- Filed to v1.4.4 W2 Entity Surfaces with DOS-325 lineage (NEW tickets to file at V1.3 lock):
  - **DOS-325-TrendStrip** — translate TrendStrip primitive (Tauri authoring + WP block) for entity-detail surfaces.
  - **DOS-325-EvidenceDrawer** — translate EvidenceDrawer primitive + drawer integration into entity-detail pages (on-demand, display-safe per DOS-477).
  - **DOS-325-Surface-residue** — entity-envelope wiring, keyboard interaction, evidence-drawer integration on Account/Project/Person Detail surfaces.
  - **DOS-9-entity-envelope** — cite-chip tooltip wired into entity-detail envelope (surface-tier consumption beyond the primitive itself).
  - **DOS-11-keyboard** — trust-band UI keyboard navigation + a11y in the surface composition (the primitive itself is keyboard-accessible; the surface-level navigation is v1.4.4 scope).
  - **DOS-XXX-healthbadge-label-discipline** (V1.3 — per cycle 3 design F1 reframe) — apply DOS-325 voice rule to HealthBadge primitive: today HealthBadge.md:9 exposes color-band tokens only (`green` / `yellow` / `red` / insufficient-data); add band-label vocabulary alongside the color tokens. Owns the v1.4.4 decision on whether HealthBadge labels collide with or distinguish from ScoreBand's `On Track | Watching | Action Needed | No Read`. Files alongside the v1.4.4 W2 Entity Surfaces work where HealthBadge composes into account-detail.
- W1 amendment (V1.2 confirms required, files at V1.2 lock):
  - **DOS-685 path-α append** — token-mapping manifest emission in `pnpm dailyos:translate-tauri` per §5.9. Codex consult confirmed `translate-tauri.mjs:414-419, 487-489` does NOT currently emit `.token-mapping.json`. MUST land on `dev` BEFORE PR-D1 opens (new pre-W2 gate per §10 + AC #14).
- Downstream: every v1.4.4+ surface composes these primitives. v1.4.4 W0 surface audit (DOS-677) is the first downstream consumer.

## 14. L0 reviewer panel — required runners

| Reviewer | Mode | Cycle 4 focus (if dispatched) |
|---|---|---|
| `/codex challenge` | adversarial | §5.8 V1.3 test fixture spec sufficiency — do `chrome_service_integration_fixture.rs` + `ChromeServiceTest.php` adequately cover the four chrome state branches (ready / loading / empty / error) for all 11 primitives? Or does the spec rely on per-primitive integration fixtures to add their own state coverage? §6.4 V1.3 reframe — is filing `DOS-XXX-healthbadge-label-discipline` to v1.4.4 the right deferral, or should HealthBadge label-discipline land in v1.4.3 W2 alongside ScoreBand to avoid the v1.4.4 co-render collision risk? |
| `/codex consult` | implementation feasibility | PR-D1 commit shape (V1.3) — is 6 ordered commits → squash-merge feasible given the workflow gate dependencies (e.g., does the chrome service test fixture need ordering relative to its consuming primitives)? §13 v1.4.4 lineage ticket filing sequence — file at V1.3 lock or at PR-D1 open? |
| `code-reviewer` (claude) | domain | Verify V1.3 §5.8 test fixture spec actually compiles and runs against the W1 harness shape; verify §10 V1.3 6-commit ordering matches W1 PR #303 precedent; verify §7.1 V1.3 N/A corrections (StatusDot compact + FreshnessIndicator compact) match `StatusDot.md:9` + `FreshnessIndicator.md:77`. |
| `design-reviewer` (claude) | design system | Verify V1.3 §6.4 reframe is grounded in `HealthBadge.md:9` color-band declaration; verify §5.8 V1.3 6-bullet contract list is complete; verify §7.1 V1.3 corrections fix the N/A issues without introducing new mis-categorizations. |
| ~~`/cso`~~ | (dropped V1.2) | Dropped per James 2026-05-18 — packet declares no new trust boundaries. Reserve CSO for v1.4.3 W4 feedback-write-infrastructure packet. |

**Convergence rule:** unanimous APPROVE required before code lands. CONDITIONAL APPROVE folds into V1.4 (or maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` if non-AC). Cycle cap: 10 cycles before L6 escalation. Path-α partial convergence allowed per `.docs/plans/v1.4.0-waves.md` precedent if same-class findings recur across cycles.

**Cycle 4 dispatch shape (V1.3 — applies 5-way codex parallelism per [[codex-parallel-5way-allowed]]):** if cycle 4 is needed, run codex challenge + codex consult + code-reviewer + design-reviewer in parallel. Cycle 3 class-patterns (§5.8 chrome, §6.4 vocab) were closed in V1.3 with concrete spec changes — cycle 4 should validate the closures, not re-litigate the class. If cycle 4 returns substantive BLOCK on a NEW class, escalate to L6 per [[reviewer-dissent-is-signal]]; if cycle 4 returns unanimous APPROVE or CONDITIONAL with only minor fold work, lock the packet. 5-minute heartbeat per [[codex-heartbeat-5-min]] on every in-flight codex.

## 15. Acceptance for L0 closure

- Cycle 4 reviewer panel returns unanimous non-BLOCK (if cycle 4 dispatched) across codex challenge + codex consult + code-reviewer + design-reviewer. CSO dropped per V1.2.
- Cycle 3 BLOCKs resolved in V1.3:
  - challenge #2 §6.4 vocab BLOCK → V1.3 §6.4 reframe (HealthBadge uses color bands not labels; ScoreBand introduces labels; collision was phantom).
  - challenge #3 §5.8 tests BLOCK → V1.3 §5.8 adds concrete fixture spec + workflow gate wiring.
- §12 open questions 11-14 resolved (1-10 are V1.0/V1.1 carry-overs marked resolved in V1.2).
- §5.8 chrome service seam locked with 6-bullet non-negotiable contract + state-branch test fixtures + workflow gate.
- §6.4 ScoreBand authoring sequence locked (`.docs/design/primitives/ScoreBand.md` precedes block translation; vocabulary justified via DOS-325 voice rule without phantom HealthBadge collision).
- §5.10 TypeBadge split locked (`TypeBadge` + `TypeBadgeDisplay`).
- §7.1 visual parity matrix Axis 2 (interaction/layout) state coverage locked with V1.3 N/A corrections.
- §10 PR-D1-carries-everything strategy validated; PR-D1 commit shape = 6 ordered development commits → squash-merge at landing.
- **DOS-685 path-α append filed + landed on `dev`** BEFORE PR-D1 opens (per AC #14).
- **v1.4.4 lineage tickets filed in Linear** with explicit pointers to this packet: DOS-325-TrendStrip, DOS-325-EvidenceDrawer, DOS-325-Surface-residue, DOS-9-entity-envelope, DOS-11-keyboard, **DOS-XXX-healthbadge-label-discipline** (V1.3 addition).
- **Maintenance ticket filed** to project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` for path-α: paste-snippet step 6 not enumerated in AC #8; CI invariant #13 typo.

When unanimous APPROVE reached, this packet locks; W2 implementation can proceed on the basis spec'd here (with PR-D1 first, then D2/D3/D4 in parallel — up to 5-way codex per [[codex-parallel-5way-allowed]]).
