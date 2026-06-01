# Session handoff — v1.5.0 Composable Surfaces (2026-06-01)

> For a **new session starting cold.** Read this top-to-bottom; it assumes no prior context. The work below is planning-complete and W0 is cleared to build.

---

## TL;DR

A list of 5 UX fixes James spotted while dogfooding grew (deliberately) into the **v1.5.0 "Composable Surfaces" program**. All planning + L0 hardening is **done and merged to `dev`** (PR #423, squash `67643bfa`). **W0 is L0-PASSED and cleared to implement (L1).** The immediate next action is building W0 — see §6.

---

## 1. What v1.5.0 is

The Tauri (macOS) app learns to **render the block model the substrate already produces** (ADR-0130 `Composition`), and lets the user **compose** it — toggle blocks/chapters on/off, reorder, pick a variant, edit inline. Built **Gutenberg-informed but native** so it's ~80% portable back to WordPress later, without forking WordPress.

The headline: *"I shape the app to me — chapter by chapter."* AI produces the Composition; the user arranges it; the substrate keeps it trustworthy.

---

## 2. The three docs (all on `dev`, under `.docs/plans/`)

- **`v1.5.0-waves.html`** — the wave plan (Tier-1 HTML, `plans.css`). Source of truth for scope/sequencing.
- **`abilities-intelligence-loop.html`** — plain-language explainer: how abilities → the intelligence loop → on-screen content (with a worked example + an honest "what's fuzzy").
- **`claim-lifecycle-freshness-editability.html`** — living "position" doc on claim lifecycle (4 states), per-type freshness profiles, and the inline-edit-eligibility rule. **Has 4 open policy dials James must set — see §7.**
- **`v1.5.0-w0-l0-packet.md`** — the W0 L0 plan packet (verified-codebase appendix + §6 L0 review record).

Render Tier-1 HTML locally with: `python3 -m http.server 8919 --directory .docs` then open `http://localhost:8919/plans/<file>.html`. (`file://` is blocked; relative CSS needs the server rooted at `.docs/`.)

---

## 3. Decisions locked (do NOT relitigate)

| Decision | Call |
|---|---|
| ADR-0130 scope | **Full Composition end-to-end** — v1.5.0 implements the Tauri renderer + a thin producer slice. |
| Substrate | **Consume, don't re-ADR.** The block model + BlockType registry + §3.1 fallback are already built (~55% of ADR-0130). No new chapter-registry ADR. |
| Build posture | **Gutenberg-informed native.** Mirror block.json / edit-save / RichText patterns on our design system + dnd-kit. **Do NOT fork `@wordpress/*`** (drags WP's paradigm in, fights the canonical design system). |
| WordPress | **Bank, don't delete.** Preserve the WP surface work for the future headless-runtime-powers-WP state (ADR-0135). Park it out of the active build/CI path. |
| Surfaces | **New designs, not old templates.** Account Detail first as the end-to-end reference; then work forward surface-by-surface. Don't build into the throwaway templates. |
| Tabs | **Dropped** in favor of Composition `Section`s (one composed scroll). Design-check on Account in W1. |
| Entity types | **3 (Account/Project/Person) now.** User-defined types + presets-dissolve are north-star v1.6+. |
| Edit mode | **In-context primary** ("Customize this page" on the surface); Settings → Surfaces panel secondary. |
| Abilities runtime | Keeps its **own** waves doc (`abilities-runtime-producer-remediation-waves.html`); hard-linked. v1.5.0 owns only the consume side + the thin producer slice. |

Background: `~/.claude/.../memory/project_v150_gutenberg_shaped_tauri_blocks.md` (saved this session).

---

## 4. Wave structure

`W0 → W1 (Account reference) → (W2 ∥ W3 ∥ W4) → W5`, plus a parallel **List-Surfaces** lane.

- **W0** — Bank WP + substrate-genericity audit + accept ADRs. *(L0 PASS — see §6.)*
- **W1** — Account Detail end-to-end reference: extend `account-overview` producer to emit the new design's blocks + build the React `BlockRenderer` + the Composition→React transport + render the new design. The proving ground.
- **W2** — In-context edit/customize mode (toggle/reorder/variant/inline-edit; per-type layout overlay; dnd-kit).
- **W3** — Forward surfaces: Project / Person / Actions (each = producer + new design).
- **W4** — Forward surfaces: briefing + meeting recap (the biggest; the produce→consume seam).
- **W5** — Settings redesign + per-type config surface (lifecycle stages + default layout).
- **List-Surfaces (parallel):** the 5 original dogfooding fixes.

---

## 5. Substrate reality (what's built vs net-new) — the most important framing

**Already built (CONSUME it):**
- `src-tauri/abilities-runtime/src/abilities/composition.rs` (~1,481 lines) — `Composition`/`Section`/`Block`, 18 `BlockType`s + Wave-1 primitives, `Salience`, `ProvenanceRef`.
- `.../abilities/fallback_projection.rs` (~1,716 lines) — §3.1 unknown-block fallback (a **privacy boundary**: drop unknown fields, cap trust at `needs_verification`, non-dismissible banner).
- `.../abilities/account_overview.rs` — the **only** ability producing a `Composition` today.
- `src-tauri/src/services/composition_render_orchestrator.rs` — `resolve_producer_ability_name` (`:198`, **account-hardcoded** prefix `dailyos/account-overview:account:` `:209`).
- `/v1/local/project-composition` loopback — route `src-tauri/src/surface_runtime/mod.rs:1118` → handler `:2347` (`Actor::User`). **The Tauri renderer's transport target.** WP-facing sibling is `/v1/surface/project-composition`.
- `src/components/ui/FreshnessIndicator.tsx` — already the wired freshness primitive (AccountDetailPage renders it ~15 sites). **No `ChapterFreshness` symbol exists** (case-sensitive).

**Net-new (BUILD it):**
- A React `BlockRenderer` (block-type → component map + component library) — **0% today**; the heart of W1.
- The Composition→React transport (Tauri command vs the local-loopback fetch — decide at W1 L0; React uses `invoke()` today, the orchestrator is WP-facing).
- More Composition producers (Project/Person in W1; briefing/meeting in W4).
- The per-type **layout overlay** (which blocks · order · variant) — the genuinely new contract; one new `NNN_` SQL migration (next free slot is **273**; not the `vNNN` Rust series).

---

## 6. W0 — cleared to implement (L1). Start here.

Linear project **v1.5.0 — Surface Designs** (`4a2a7eeb`), DailyOS team (`409a42a0`). **L0 PASS — unanimous APPROVE** (`/codex challenge` + `ce-feasibility-reviewer`); verdicts on each ticket.

- **DOS-835 (High)** — Bank WP + carve-out + **CI-gate sweep**:
  - Park the v1.4.4 WP-migration plan + the **89** `wp/dailyos/blocks/` blocks (preserved, not deleted — name parking location); mark DOS-677 obsolete.
  - **Sweep ALL WP-block-presence CI gates in one pass** (same-shape class): `check_w1_consumer_skeleton.sh` + `lint-frontend.yml:134`; `block-kit-integration.yml`; verify `wp-plugin.yml`. Parking the blocks **red-fails these** if not retired.
  - **Do NOT remove** `composition_render_orchestrator.rs` or the `/v1/local/project-composition` loopback — shared substrate W1 reuses.
  - **Add a smoke test** for `/v1/local/project-composition` (`Actor::User`, currently untested) proving it resolves a Composition post-bank.
  - AC: W0 PR CI-green across all three workflows; smoke test passes; WP preserved + unbuilt; parking location bound to CI path-filters.
- **DOS-836 (High)** — Audit `get_entity_intelligence` envelope + producers for WordPress-shape assumptions; name the account-hardcoded resolver generalization as W1 work (don't fix it in W0).
- **DOS-837 (Med)** — Accept **ADR-0130** (Proposed→Accepted) + **ADR-0122 as Option A** (all-or-nothing freshness + instrumentation, NOT per-chapter). **Freshness = no code action** (FreshnessIndicator already wired).

**Next action:** branch off `dev` (e.g. `feat/dos-835-w0-bank-wp`), implement against the three ACs, smoke test as proof.

---

## 7. Open — James's calls (not yet decided)

Four policy dials in `claim-lifecycle-freshness-editability.html` §4 (proposed defaults in the doc, awaiting confirm):
1. **Staleness → screen:** fade at ~1 half-life, fold-under-"stale" at ~2, never silently hide.
2. **Edit low-confidence claims?** Proposed yes (block only the §3.1 fallback state).
3. **Is "false" forever?** Proposed yes + a short undo window.
4. **Who can edit?** Proposed: user yes; agents propose, never silently edit.

Plus one design check (W1, not a blocker): does the Account page read well as **one composed scroll** (Sections, no tabs)?

---

## 8. List-Surfaces lane (the original 5 fixes)

- **DOS-826** multi-select on entity-list pages · **DOS-830** bulk archive (blocked by 826) · **DOS-828** orphaned-folder reconciliation on archive (High) · **DOS-829** reparent drag-and-drop (shares dnd-kit) · **DOS-827** editable lifecycle stages, per-type (lands via W5's per-type config surface).
- **Premise-check (prod vs dev) is a gate before writing AC** for 826/828/830.

---

## 9. Gotchas surfaced this session (save future pain)

- **codex CLI is flaky** (`codex-cli 0.133.0`): `codex exec --output-format=json` errors; plain `codex exec "<prompt>"` sometimes emits nothing to stdout foreground. Backgrounding it (`run_in_background`) worked and returned a verdict. Don't vortex on it — the `ce-*` reviewers cover the substantive ground.
- **Case-sensitive grep for symbols.** A `grep -ril ChapterFreshness` (case-insensitive) falsely matched the camelCase `chapterFreshness` *prop*; case-sensitive showed zero. Verify "X exists" by opening the file.
- **L2 `validate-pr-template`** needs a top-level `security_auditor_invoked: true|false` (§4 Security). For doc-only PRs: `false` + `EXEMPT-DOC-ONLY` (passes because no security-trigger path matches). The workflow reads the body from the **event payload** — editing the body + re-running won't update it; you must **push a commit (synchronize)** to re-trigger with the new body.
- **PII blocklist** (`~/Documents/.claude/pii-blocklist.txt`) contains short customer abbreviations that substring-match ordinary English words — a 3-letter entry matched a common word in this session's prose (a false positive). Sweep before push; reword the false positive rather than `--no-verify`. (This handoff was itself reworded for that reason — don't reintroduce the literal term.)
- **Repo topology:** remote is **`public`** (no `origin`); PRs target **`dev`**. Pre-push gauntlet runs clippy/test/tsc but skips for doc-only trees.

---

## 10. Git + Linear state at handoff

- **PR #423 merged to `dev`** (squash `67643bfa`, 2026-06-01 14:04Z); branch `docs/v1.5.0-planning` deleted. Local `dev` synced.
- Working tree currently on `dev`.
- Linear: W0 = DOS-835/836/837 (L0 PASS, Backlog); list lane = DOS-826/827/828/829/830 (Backlog); all in project `4a2a7eeb`.
- This handoff doc is **untracked** in `.docs/plans/` — commit it if you want it durable.
