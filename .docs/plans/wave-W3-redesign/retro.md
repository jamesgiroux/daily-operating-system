# W3 — Daily Briefing redesign patterns — retro

**Tickets:** DOS-425 (PredictionsSection), DOS-423 (MovingRow), DOS-424 (WatchRow), plus ProvenanceStat primitive (MovingRow's dep).

**Total:** 3 patterns + 1 primitive, 42 tests, 5 commits (4 component commits + this retro), wave-level L2 verdict PASS.

## What worked

- **The W1 pattern generalized cleanly to W3 patterns.** Same scout-then-fan-out shape: I wrote PredictionsSection + ProvenanceStat in-conversation as the W3 templates, then dispatched MovingRow + WatchRow as parallel codex agents. Fan-out output was indistinguishable from hand-written work per L2 verdict.
- **Cardinal-rules-as-prompt-input.** I added "no inline CSS" + ".root + camelCase children" + "STOP on contract-fit issue" to both codex prompts. Both agents honored every rule. L2 grep for `style={` hit zero. The discipline transfers if you tell the agent the discipline.
- **Discriminated-union exhaustiveness.** WatchRow's `default: { const exhaustive: never = row; ... }` block is the right TS pattern for kind-discriminated wire contracts. Future addition of a fifth `WatchRowViewModel` variant will fail to compile until the switch handles it. Codex picked this up from the contract type definition without prompting.
- **Custom-property-via-data-attribute** as the runtime-styling pattern. MovingRow's accent bar uses `[data-kind="customer"] { --moving-accent: var(--color-account-turmeric); }` rather than inline style. Memory's "narrow exception" for runtime-computed values is even narrower than I thought — most cases solve via attribute selectors, not inline.
- **Class-of-bug discipline carried over.** L2 review explicitly checked for ephemeral `DOS-XXX` refs in code comments (the W1 lesson). Zero hits across 12 files.

## What we'd change

- **Test class-name regex pattern.** `ProvenanceStat.test.tsx` initially used `/_up_|up$/` which would match `popup` or any class ending in "up." L2 caught the lenience even though no actual collision exists. Tightened to `/(^|_)up(_|$)/` post-review. Going forward: when asserting CSS-Module-hashed class presence by regex, anchor on `(^|_)` and `(_|$)` boundaries from the start.
- **Dead query in test.** Same file had `const valueEl = container.querySelector(...)` followed by `void valueEl` — leftover from earlier scaffolding. Trivial but L2 reviewer correctly flagged it as confusing for future readers. Deleted.

## Pattern handoff to W4

W4 (wire-ins) is different in shape — it integrates parent-track outputs (DOS-320 trust band, DOS-411 claim lifecycle) into existing W1/W3 components rather than building new ones. Discipline carries:
- Components consume contract types directly (no transformation).
- Mutations isolated via callback props.
- ds-inspector attributes universal.
- CSS Module convention: `.root + camelCase children`, no inline CSS.
- New L0 audit step: `git diff fork..dev` over surfaces touched, not just type-name grep (per `feedback_l0_reconcile_against_dev.md`).

## Wave gate

- [x] All 3 W3 patterns + ProvenanceStat primitive ship at `src/components/dashboard/`
- [x] All 4 spec md files at status `integrated`
- [x] `pnpm tsc --noEmit` clean
- [x] All 42 W3 tests pass
- [x] All 97 dashboard component tests pass when run together (W1 + W3)
- [x] L2 wave-level review: 0 critical, 0 major, 2 minor (both addressed inline)
