# Wake-up status — v1.4.4 — 2026-05-21 (early AM PT)

**TL;DR:** W1 shipped clean (PR #346). W2 L0 cycle-1 surfaced 2 scope decisions only you can make. Stopped per pacing-rule discipline before fold.

## What landed while you slept

### ✅ W1 wave: shipped via PR #346

**PR:** https://github.com/jamesgiroux/daily-operating-system/pull/346

- 10 sub-tickets implemented across 3 stages (1a/1b/1c)
- L2: 3 cycles to convergence (cycle-3 codex P2s → DOS-749 + DOS-750 maintenance tickets)
- L3: 2 cycles. Codex challenge cycle-1 surfaced 5 wave-level integration defects L2 missed (signal coalesce, touchpoint audience bypass, migration race, envelope cache binding, decorative WP skeletons). Cycle-2 patches fixed all 5; codex cycle-2 confirmed PATCHED CORRECTLY.
- Cargo test on integrated post-L3 wave: **2643 passed / 0 failed** (35 new tests from cycle-2/3 patches)
- All 4 CI lint scripts green (`check_audit_disclosure_allowlist.sh`, `check_audit_denylist_completeness.sh`, `check_sensitivity_gate_composition.sh`, `check_w1_consumer_skeleton.sh`)
- 3 K-out `docs/solutions/` entries captured per engineering-ladder.md K-out obligation

**Verdict trail under** `.docs/plans/v1.4.4-wp-surface-migration/reviews/`:
- L2: 4 cycles × 4 reviewers (gstack /review + code-reviewer + /cso + codex review)
- L3: architect cycle-1 + codex challenge cycle-1 + cycle-2

### ⚠️ W2 L0 cycle-1: mixed — needs your scope calls before V1.1 fold

Drafted W2 L0 packet at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md` (749 lines, 15 sub-tickets across 7 sections). Dispatched 5-reviewer panel. Outcome:

| Reviewer | Verdict |
|---|---|
| codex challenge | BLOCKED — 2 CRITICAL + 3 HIGH |
| architecture | APPROVE w/ 2 MEDIUM + 1 LOW |
| design-lens | CONDITIONAL APPROVE — 2 BLOCKING |
| wp-skill | REQUEST-CHANGES — 4 HIGH + 3 path-α |
| codex consult | APPROVE — 2 advisory drifts |

## 🛑 2 scope decisions only you can make

These are NOT mechanical fold items. They require your judgment on scope. **Without your call, I should not do the V1.1 fold** because both involve substantive trade-offs:

### Scope decision 1: Outer/inner contract drift (codex challenge F1 CRITICAL)

**Issue:** The W2 packet was drafted to ADR-0130's `Composition.sections[].blocks[]` mental model — but the W1 envelope actually ships as `BTreeMap<EnvelopeSection, SectionState>` over 7 variants (Facts/Health/MetadataProposals/OpenLoops/Touchpoints/Threads/Record). The 22 inner blocks named in W2 §5.1 (outlook, on_track, stakeholders, etc.) have no home in the 7 envelope variants.

**Two paths:**

- **A — Amend W1**: emit ADR-0130 `Composition` from `get_entity_intelligence`; map BlockType taxonomy. Requires W1 PR #346 reopen + substrate redo. Larger blast radius but correct per the locked decision.
- **B — Amend W2**: rewrite the 22-inner-block taxonomy onto the 7 actual EnvelopeSection variants. Inner blocks become sub-sections of Facts/Health/etc., NOT free-form chapters. Smaller scope shift but the user-facing IA changes (you lose the "outlook / on track / stakeholders" chapter naming).

**Recommendation: B** — substrate is shipped, surface taxonomy is more forgiving. But it changes the W2 surface IA you may have had a strong opinion on.

### Scope decision 2: Meeting Detail block — W1 reopen or W2 removal (codex challenge F2 CRITICAL)

**Issue:** `EntityKind` enum in `get_entity_intelligence/contracts.rs:28` only has Account/Project/Person — no Meeting. W2 §5.4 includes Meeting Detail block but the substrate doesn't support it.

**Two paths:**

- **A — Reopen W1**: add Meeting variant to EntityKind + meeting-shaped section composers. W1 PR #346 reopen + new sub-ticket + L2 + L3 cycle on extension. ~1-2 days work.
- **B — Drop Meeting Detail from W2**: defer to a sub-wave (e.g., W2.5) or push to a later version (v1.4.5). Cleaner W2 close; user loses Meeting Detail surface in v1.4.4.

**Recommendation: B (defer)** unless Meeting Detail is load-bearing for your daily flow. Keeps W2 scope tight and lets W1 stay shipped.

## What I'd fold if you wake up and say "go on both with my recommendations"

If you accept B + B above, V1.1 fold is mechanical and ~30 min of agent work:

1. W2 §5.1-5.4 rewrite inner-block taxonomy onto 7 EnvelopeSection variants
2. W2 §5.4 Meeting Detail → file as DOS-XXX deferred to v1.4.5; remove from W2 scope
3. DOS-725 tint: file ADR-0077 amendment ticket as prerequisite (or accept current chrome_config olive-by-default)
4. AgentMcp Option A → Option B (no per-item timestamp/lifecycle for AgentMcp redacted touchpoints)
5. block.json snippets: add `templateLock: false` + `template` array
6. "Synced patterns" → "filesystem patterns" terminology fix (4 sections)
7. Empty-state inner block pattern: lock as §10 invariant
8. envelopeHandle context key resolution contract: define + cite shared hook path
9. Inline CSS DOS-725 boundary: scope to custom-properties-only + add CI gate

Then re-dispatch L0 panel for cycle-2.

## Other state worth knowing

- **`wave/v1.4.4-w1-stage1a` branch pushed** — PR #346 is up to date with W2 L0 packet + verdicts + K-out solutions
- **DOS-749 + DOS-750** filed in Codebase Maintenance project (L2 cycle-3 codex P2 path-α)
- **Codex reliability** notably improved after SQLite log compaction (2.7 GB → 1.7 GB, killed 4 stale app-server daemons). All subsequent codex dispatches via `codex exec` direct (per memory) or `codex-companion review` worked reliably.
- **No destructive operations performed.** No tags. No merges to dev. Only the wave branch push to remote.

## Codex memory updates worth noting

You manually edited the codex memory at `~/.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/feedback_codex_exec_direct_for_oneshot_reviews.md` mid-session with the correct CLI flag note (`--skip-git-repo-check -C <dir>` + `< /dev/null`, no `--output-format=json`). Updates applied through the rest of the session.

## Recommended morning sequence

1. Review PR #346 (W1 wave) — likely needs L4 hands-on smoke before merge per `feedback_l4_before_l2_for_user_facing` (though W1 is substrate, not user-facing; could merge after spot-check)
2. Decide on the 2 W2 scope questions above
3. Reply or run a one-line directive and I'll do the W2 V1.1 fold + re-dispatch L0
4. Once W2 L0 unanimous → W2 L1 implementation dispatch (5+ parallel agents per stage)
