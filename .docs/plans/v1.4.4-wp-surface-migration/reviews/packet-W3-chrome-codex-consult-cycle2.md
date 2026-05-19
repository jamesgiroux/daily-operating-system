# L0 Codex Consult - Packet W3 Chrome Lane (Pulled Forward) - Cycle 2

**Reviewer:** `/codex consult`
**Date:** 2026-05-19
**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.1

## Verdict: CONDITIONAL APPROVE

V1.1 is close enough for L1 shape, but not as-is. Two cycle-1 folds are under-specified, and the deferral/ticket hygiene needs to be closed before L1 kickoff.

## K-in Re-grep

Mandatory re-grep completed across `docs/solutions/` and `.docs/decisions/`.

- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` still controls W4: existing `surface_nonce.rs` + WP transport already cover nonce substrate; real W4 gap is `verify_nonce` -> `record_claim_feedback` + action expansion.
- `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md:13-15,49-52` remains relevant to `functions.php` lint posture.
- ADR hits reaffirm the packet's boundary: ADR-0129 lines 58-63 custom block/theme/plugin split; ADR-0130 lines 148-188 renderers-not-authors; ADR-0077 lines 27-31 and 45-49 fixed shared chrome; ADR-0076 lines 81-92 entity/state color boundary.

## C1-C6

- **C1 - APPROVED in V1.1.** §10 adds W4-coupling matrix, cites the K-in solution, and asserts disjoint write sets. Note: current `.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md` still contains V1.0 `surface_feedback_nonces` / `services::surface_feedback` scope, so W4 remains independently blocked until revised. W3 can proceed if it stays inside `wp/dailyos/theme/**`.
- **C2 - APPROVED in V1.1.** §10 enumerates exactly `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, `MagazinePageLayout`; footer/body/claims/trust/provenance stay Gutenberg.
- **C3 - APPROVED in V1.1.** AC #23 adds stock-theme negative gate.
- **C4 - INSUFFICIENT.** AC #30 exists, but its source grep `var\(--[a-z-]+\)` misses real canonical tokens with digits and fallbacks, e.g. `--color-desk-charcoal-4`, `--color-spice-turmeric-10`, `--color-garden-sage-15`, `--color-alert-red, #dc2626`, `--local-pill-top, 0px`. Fix gate with a CSS parser or at least `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)`, then classify local computed props (`--pill-*`, `--local-pill-top`) separately from aliases.
- **C5 - INSUFFICIENT.** AC #28 is referenced in changelog/§8/§7, but no actual detailed acceptance criterion defines the static gate, path, or failure rule. Add a real AC #28 body and include the Pill reconciliation Linear ticket ID, not only the maintenance project ID.
- **C6 - APPROVED in V1.1.** §5.5 explicitly prohibits DB writes, runtime/transport calls, claim/trust/provenance branching, abilities runtime invocation, block/CPT/REST/admin registration, top-level execution, and closing `?>`.

## Higher-order Questions

- **30 ACs:** acceptable only if L1 is split into the three stated sub-tickets and each ticket maps its owned ACs. One mega-ticket cannot govern this cleanly.
- **Patch 9 maintenance:** bounded but real. 9a idempotency is WP-overlay appropriate. 9b refresh-button class/CSS is canonicalizable UI behavior; file an upstream canonical-chrome ticket or move it upstream before the next sync cycle.
- **§8 deferrals:** file Codebase Maintenance tickets before L1 for blockification cost, scroll-spy, and Pill reconciliation. Pill is claimed filed, but needs a ticket ID in the packet.
- **W4 dependency:** if W4 cycle-2 does not fold the K-in correction, W4 should not merge. W3 is not blocked if its diff stays in theme paths and L2 rejects any cross-edit into W4 runtime/plugin/block paths.
- **Pacing/L6:** this is not substantively blocked, but it is not converged. If the process treats these as cycle-2 clerical fixes, L6 is avoidable. If they require cycle-3 review, the packet's own "2 cycles without convergence => L6" rule applies.

## Required Fold Before L1

1. Fix AC #30 token extraction so numeric/fallback custom properties are covered.
2. Promote AC #28 into a real acceptance criterion with an enforceable static gate and Pill ticket ID.
3. Add ticket IDs for the §8 deferrals named above, or explicitly mark them as W3 non-blocking with owner/date.
