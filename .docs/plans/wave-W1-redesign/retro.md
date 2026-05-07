# W1 — Daily Briefing redesign components — retro

**Tickets:** DOS-422 (SignalDot), DOS-420 (DayStrip), DOS-421 (InferredActionSelector), DOS-426 (Lead).
**Total:** 4 components, 38 tests, 6 commits.
**Wall clock:** ~30 minutes from W0 close to W1 L2 fix landed.

## What worked

- **Scout-then-fan-out beat 4-way parallel from the start.** Driving SignalDot end-to-end first surfaced the kebab/camelCase contract-vs-spec drift (`SignalDotKind` is wire kebab-case, design system spec listed camelCase variant identifiers). That reconciliation became the template the 3 codex fan-out agents inherited — the lookup table at the prop boundary, no transformation in the contract.
- **Codex agents honored "STOP on contract-fit issue, don't invent."** DayStrip explicitly declined to add a missing-neighbor fallback because the contract requires `prev` and `next` — exactly the right move. InferredActionSelector caught its own a11y regression mid-cycle (option label missing space before confidence in trigger accessible-name) and re-ran gates.
- **Single-message wave-level L2 review beat per-component review.** One code-reviewer pass on all 4 components surfaced cross-component consistency issues (4-way CSS taxonomy split) that wouldn't have shown in per-component reviews. The systemic-look paid off here.
- **Class-level sweep of ephemeral refs caught 7 issues the L2 reviewer flagged 1 of.** Reviewer spotted only the inline DOS-413 in SignalDot; my grep across the 4 components + 4 CSS files found 8 total ephemeral refs in headers. All fixed in one commit.

## What we'd change

- **Spec drift.** Lead.md still had old API field names (`eyebrow` / `sentence` / `sharpClause`) from the design exploration phase, not the locked `LeadViewModel` shape. Codex caught and fixed in the impl turn, but it could have surfaced earlier in W0 if the contract → spec sync had been a W0 acceptance criterion. Worth adding to the W2 ticket plan acceptance: "spec API sketch matches the locked contract type."
- **CSS taxonomy convention should have been documented at W0, not discovered at W1 L2.** Four agents, four conventions. Now standardized on `.root + camelCase children` which matches pre-existing dashboard precedent. Document this in `.docs/design/README.md` or `NAMING.md` so W2 patterns inherit it without rediscovery.
- **Ephemeral-refs memory rule needs a pre-commit hook, not just a memory.** I caught my own violation across 8 files via post-hoc sweep. A simple grep gate at `.claude/hooks/pre-commit-gate.sh` would have failed the original commit. Add `DOS-[0-9]+|cycle-[0-9]+|fix #[0-9]+` to the term blocklist for code comments specifically (not commit messages, which already have it).

## Carry-forwards (not in W1)

Tracked for later land, not blocking W2 dispatch:

- `m2` Lead test: add negative assertion that `data-ds-name="Lead.punchLine"` is null when contract field absent.
- `m3` DayStrip test: no assertions on `title` attribute or `aria-label="Briefing days"` on the nav.
- `m6` InferredActionSelector divider position: contract should clarify "options[0].divider must be false" or component should suppress orphan separator. Either fix is small; needs decision.

## Pattern handoff to W2

- Components consume contract types directly from `@/types/briefing` — no transformation layer.
- `data-ds-name`, `data-ds-tier`, `data-ds-spec` attributes universal.
- Mutation isolation: components take callback props; parents own side effects.
- CSS Module convention: `.root` + camelCase children.
- Test shape: jsdom env, parameterized over union variants where applicable, ds-inspector attribute coverage.

W2 services (Rust) will follow a different shape — but the discipline of "consume the contract directly, don't reshape" carries.

## Wave gate

- [x] All 4 component .tsx + .module.css + .test.tsx files ship at `src/components/dashboard/`
- [x] All 4 spec md files at status `integrated`
- [x] `pnpm tsc --noEmit` clean
- [x] All 38 tests pass
- [x] L2 wave-level review verdict: ship-ready
- [x] L2 fix bundle landed (taxonomy, callback, ephemeral-refs sweep)
