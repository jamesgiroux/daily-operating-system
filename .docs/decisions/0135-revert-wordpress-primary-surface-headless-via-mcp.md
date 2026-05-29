# ADR-0135 — Revert WordPress as Primary Surface; Headless via MCP Read/Write

**Status:** Accepted
**Date:** 2026-05-29
**Authors:** James Giroux, Claude
**Supersedes:** [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) (the "WordPress Studio promoted to primary surface" decision specifically)
**Reinforces:** [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) (the substrate is the product; heads are surfaces over it)
**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0130](0130-surface-independent-composition-contract.md)
**Relates to (planning):** `.docs/plans/dailyos-rearchitecture-examination-2026-05-29.md`, `.docs/plans/v1.4.9-reconciliation-ledger-2026-05-29.md`

## Context

[ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) (Proposed, 2026-05-10) promoted **WordPress Studio to the primary user-facing surface**, with the Tauri macOS app reorienting to runtime-host duties. Its load-bearing argument was economic: the v1.4.2+ "redesign tokens" are spent once, so spend them on WordPress blocks from the start rather than on a Tauri React surface that gets re-spent later.

Two things have happened since that wager:

1. **The substrate reset (2026-05-29).** A production DB-loss event triggered a clean-sheet architecture examination (`.docs/plans/dailyos-rearchitecture-examination-2026-05-29.md`), adversarially reviewed by four independent reviewers. Three of them independently flagged that **WordPress-as-primary worsens the deployment story for the actual user** (a non-technical knowledge worker): a constellation of *local WordPress Studio (a developer tool) + a headless daemon + an LLM client* is **more** technical to run and keep alive than a single signed macOS app — the opposite of the app's packaging virtue. They also flagged that the mission's load-bearing **proactive ("know before you ask") requirement** has no native push home on a WordPress surface, while it does in the app.

2. **WordPress-as-primary never reached production-readiness.** The v1.4.4 WordPress Surface Migration produced real, banked substrate (entity intelligence, claim receipts, briefing abilities) but the WordPress *surface* itself stayed a spike. Open WP-surface work (visual parity PR #367, per-block transport, theme chrome) was paving a layout the substrate reset abandons.

The economic argument in ADR-0129 assumed the redesign tokens were about to be spent on surfaces regardless. The reset re-orders priorities: **the differentiated work is the judgment moat (claims, trust, salience, belief revision), not the surface.** Spending redesign tokens on *any* new surface — WordPress or otherwise — before the moat is proven and before a non-technical deployment story exists is premature. ADR-0129's "pay once" framing was right about not paying twice; it was wrong about which thing to pay for, and when.

## Decision

1. **WordPress is no longer the primary user-facing surface.** ADR-0129's promotion of WordPress Studio to primary is reverted. WordPress-surface work (blocks, theme, parity) is halted. The substrate it produced is banked, not extended.

2. **Headless remains the right long-term frame — delivered via MCP read/write.** [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) stands and is reinforced: the substrate is the product, exposed headlessly through **MCP read and write tools** so any MCP host (Claude Desktop, Cursor, future agent shells) consumes DailyOS intelligence. Headless ≠ WordPress; headless = MCP.

3. **The macOS (Tauri) app remains the primary visual + control-plane surface.** It owns the daily ritual, lifecycle/supervision, and the only native **proactive push** channel the mission requires. It is not deprecated.

4. **New surface-design work is scoped to v1.5.0** (the "Surface Designs" version) and targets the **app + MCP**, not WordPress.

5. **ADR-0130's surface-independent composition contract survives as a principle** (abilities produce compositions; surfaces render them; don't reinvent composition per surface), but its concrete WordPress consumer is deferred. The contract is the durable part; the WordPress instance was the deprioritized part.

6. **The "spend redesign tokens once" rationale (ADR-0129) is explicitly rejected** as a reason to commit a surface pivot now. The cheapest moment to commit a surface is after the differentiator is proven, not before.

## Consequences

**Immediate:**
- WP visual-parity PR #367 is closed; WP-surface tickets are canceled (handled in the v1.4.4–v1.4.7 reconciliation, `.docs/plans/v1.4.9-reconciliation-ledger-2026-05-29.md`).
- v1.4.4 "WordPress Surface Migration" is closed; its "Tauri shell can be deprecated after this wave" premise is void — the shell stays.
- v1.4.9 substrate-reset surface direction reads: **MCP read/write + macOS app**, not WordPress Studio.

**What this makes easier:** one deployment artifact for a non-technical user; a real home for proactive push; effort concentrated on the judgment moat and the MCP tool surface (v1.4.9 W5) instead of a second render surface.

**What this gives up:** the editorial/block composability of WordPress and the WordPress.com publish-and-share path. Both are recoverable later — this is a revert "for now," not a permanent foreclosure.

## Revisit condition

WordPress (or any new visual surface) earns reconsideration when **all three** hold: (a) the judgment moat is demonstrably producing felt trust (the v1.4.9 eval-harness / "prove the moat" work, DOS-338); (b) a non-technical, local-first, single-artifact deployment story for a WordPress surface exists; (c) there is a concrete user pulling for a web/shareable surface that the app + MCP cannot serve. Absent all three, headless-via-MCP plus the app is the surface strategy.

## Anti-patterns explicitly rejected

- **Re-opening WordPress block work "to not waste" the v1.4.4 surface investment.** The substrate is banked; the WordPress *render* layer was a spike. Sunk cost is not a reason to keep paying.
- **Treating "headless" as a synonym for "WordPress."** Headless means MCP read/write into any host. WordPress was one candidate head, now deferred.
- **Deprecating the macOS app.** It is the packaging + proactive-push answer for the non-technical user; reverting WP-primary does not strand the user on a developer tool.
