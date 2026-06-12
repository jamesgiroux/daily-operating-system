# v1.5.0 WR — Detail-surface reconciliation — Proof Bundle

**Ticket:** DOS-852 · **Branch:** `dos-852-v150-wr-detail-surface-reconciliation` · **Date:** 2026-06-10

Account-first reconciliation of the v1.5.0 Composable Surfaces program: producer↔projection parity, field provenance, value formatting, Account variant-D rendering with trust-as-opacity, and editable snapshot fields.

## Sub-stream status

| Sub-stream | Status | Commit(s) |
|---|---|---|
| R0 — Producer↔projection parity gate | ✅ | `ae1ea93b`, `aee5c79d` |
| R1 — Field provenance at file-sync | ✅ | `695968f4` |
| R2 — Producer-side value formatting | ✅ | `02667aac` |
| R3 — Account variant-D rendering + trust-as-opacity | ✅ (pending L4 visual review) | `a27fff60`, `d668299c`, `1537b745`, `337d5c40` |
| R4 — Editable type/lifecycle chip + vitals | ✅ | `d668299c` |
| R5 — Proof / verification / L4 | ◑ this doc; L4 visual review with James pending | — |

## What shipped this wave (R3–R5)

### Producer_unavailable regression (closed) — `a27fff60`
- **Root cause:** R2 added `display_only` bindings for `/vitals/*/display_value` + `/vitals/*/kind` but never added the matching patterns to `ACCOUNT_OVERVIEW_FIELDS`. Any account with vitals failed projection — `InvalidProducerOutput { BindingTargetsUnknownField }` — surfacing as `producer_unavailable`. It only appeared once R1 let vitals through on enriched accounts.
- **Gate gap:** the R0 parity fixture carried **no vitals**, so the vitals binding set was empty and the desync was never exercised — a false green.
- **Fix:** added the two rule patterns; hardened the R0 fixture to inject headline vitals through projection and assert `display_value` survives.

### Chrome-free editorial hero + editable snapshot fields — `d668299c` (R3 hero, R4)
- Hero rendered to variant-D parity against the **shipped spine-D components** (`AccountHero`/`VitalsStrip`/`IntelligenceQualityBadge`), not the kitchen-sink reference: chrome-free masthead, dot-separated vitals, ambient freshness dot (not a trust band), no per-line provenance, identity-only (no lede).
- **Editability (R4):** type chip (`TypeBadge`) + each vital route snapshot-field corrections through `update_account_field` (service layer) then `composition.refetch()` re-projects so the edit shows. Never the bespoke account hook; `useChapterLayout` untouched (ADR-0136). `CompositionVitalsStrip` does display-formatted / edit-raw with columns mapped from vital labels.

### Per-claim provenance_kind signal — `1537b745` (R3)
- Each claim block emits `provenance_kind: "sourced" | "inferred"` (keyed off `source_ref` presence — hard fact vs enrichment inference), injected at the `build_claim_block` chokepoint (top-level / `/items/0` / `/nodes/0`), with display-only bindings and matching patterns in all five claim-block rules.
- R0 fixture extended to exercise the Health + Relationship placements so every claim-block rule's `provenance_kind` binding is gate-validated.

### Trust-as-opacity rendering — `337d5c40` (R3)
- `BlockShell` keys presence off `provenance_kind`: `inferred` → faded (~62%, restored on hover/focus) + tooltip + the existing confirm/contest (`IntelligenceCorrection`); `sourced` → full presence.
- Pulled back the routine trust chrome (always-on `TrustBandBadge`, `ProvenanceTag`, per-block freshness, routine "from N sources"). Kept the **safety** provenance states (unavailable / masked / pending) and claim feedback.
- Renderer test proves the §3.1 unknown-block fallback privacy boundary holds **in edit mode** (ADR-0136).

## Verification

| Gate | Result |
|---|---|
| `pnpm tsc --noEmit` | ✅ clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ clean (exit 0) |
| `cargo test -p abilities-runtime --lib account_overview` (incl. R0 parity gate) | ✅ 16 passed |
| Frontend changed-area tests (`ReactBlockRenderer.test.tsx` 12/12; adjacent account/entity suites) | ✅ pass (main repo) |
| `cargo test` (full workspace) | ⏳ runs at pre-push (touched crate green; main `dailyos` crate unchanged by R3/R4) |

**Environment note (not a blocker):** full `pnpm test` reports failures, but **all** are from a stale sibling worktree (`.worktrees/codex/v1.5.0-list-surfaces-pr-b/`) that vitest globs into; the main-repo copies of those same suites pass. Pre-existing config gap (vitest should exclude `.worktrees/`) — filed to path-α below, not a WR blocker.

## Trust-rendering model decision (supersedes literal R3 "renders trust bands" wording)

Decided with James (2026-06-09/10): **trust surfaces as opacity, not chips.** Hard-sourced claims (provenance class, `source_ref` present) render at full presence; enrichment-inferred claims fade with a tooltip + confirm/contest. The fade is keyed off the producer's `provenance_kind`, **not** the trust score. Rationale: the cold-start trust score compresses ~90% of claims into 0.70–0.76 around the 0.75 threshold, so it can't separate good sources from bad yet (see DOS-853). Provenance class is correct today; the confirm/contest feedback trains the score over time.

## R4 routing note (for L2)

The chip/vital edit routes through `update_account_field` (service layer): writes the column, sets `user_edit` provenance, emits a `field_updated` signal, and records a self-healing enrichment correction — then propagates via `composition.refetch()`. This satisfies the AC substance (structured action, propagates, no fabricated provenance, parity gate green). It is **not** the pure claim-feedback `IntelligenceCorrection` path the AC names as the reference pattern; that path records a correction claim but does not write the snapshot column, so it would not propagate to the rendered value within WR (the AC's explicit "if it cannot propagate, …" condition). Flagged for L2 to confirm the routing choice.

## L4 — pending (James's single review)

Per the agreed cadence ("all chapters, then one review"), the Account Detail scroll renders live on the replica (`DAILYOS_DB_MODE=replica`, enriched accounts: Digital-Marketing-Technology / Heroku / Credit-Karma). Visual review to capture: hero parity, one-composed-scroll readability, the opacity fade on enrichment-inferred chapter claims (+ tooltip + confirm/contest), and per-chapter editorial polish. Screenshots to be attached on review.

## Path-α / follow items (separated from blockers)

- **DOS-853** — trust-score cold-start recalibration (system-wide; own L0).
- **vitest `.worktrees/` exclusion** — config gap polluting full `pnpm test`; Codebase Maintenance.
- **Lifecycle vital** — ideally source-populated (glean/salesforce); local-only fallback could be typeahead-from-history (validation/trigger tradeoffs). Plain text input accepted for now.
- **Forward surfaces** (Project/Person/Action variant-D + their `provenance_kind`) — follow wave; the `build_claim_block` pattern + per-rule field additions are account-only here by scope.
