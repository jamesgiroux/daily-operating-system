# L0 Cycle-2 codex-consult Verdict — W2 Entity Surfaces Packet V1.1

**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md`
**Packet commit:** `ffccb485` (`docs(v1.4.4 W2): V1.1 fold — 18 findings across 5 reviewers folded`)
**Cycle-1 verdict:** PARTIAL APPROVE (2 advisory drift) — `packet-W2-codex-consult-cycle1.md`
**Run:** 2026-05-21, `codex exec` direct (4-check verification)

## VERDICT: APPROVE

All cycle-1 advisory drift resolved. The one apparent NEW-DRIFT codex flagged is a false positive on inspection (see Check 4).

## Per-check resolution

**Check 1 (cycle-1 D1 — K-in inventory refresh): RESOLVED.**
Packet §3 line 78 correctly cites "24 .md files at V1.1 scan." Disk `rg --files docs/solutions -g '*.md' | wc -l` returns 24. All 3 new W1 K-out entries cited by name in §3 (lines 85, 93, 103) with applied-to context in lines 112–115:
- `architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md`
- `tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md`
- `workflow-issues/parallel-agent-commit-hook-contention-2026-05-20.md`

**Check 2 (cycle-1 D2 — DOS-339 SHA footnote): RESOLVED.**
Packet line 143 carries the dedicated footnote: "`0243df65` is the **PR-merge SHA**…; the underlying feature commit is `ca8e7e21`." Git confirms both labels match (`ca8e7e21 feat(claim_receipt): fan-out signal wiring + useClaimReceiptSubscription hook (DOS-339)`, `0243df65 Merge feat/dos-339-claim-receipt-fanout into wave/v1.4.4-w1-stage1a`). Footnote correctly notes both reach the same tree; readers walking hunks point at `ca8e7e21`, PR-trail reconstruction uses `0243df65`.

**Check 3 (V1.1 §5.4 new citations): RESOLVED.**
Both `87df7cf6` (W1 Meeting EntityKind extension) and `c5c0578f` (merge into wave) present on `wave/v1.4.4-w1-stage1a`:
```
c5c0578f Merge worktree-agent-a5e003b64a9524cf8 (W1 Meeting EntityKind extension for W2 §5.4)
87df7cf6 feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)
```
Packet §5.4 (lines 449, 462, 487), §6 table (line 696), §13 (lines 870, 876) all cite consistently.

**Check 4 (V1.1 §1 base ref update from stale `deb682b0`): RESOLVED — not drift.**
Packet line 11 reads "Branch base: `wave/v1.4.4-w1-stage1a` at HEAD `c5c0578f`." Current `git rev-parse wave/v1.4.4-w1-stage1a` → `ffccb485`, which is the V1.1 packet commit itself (`docs(v1.4.4 W2): V1.1 fold…`). A packet cannot cite the commit that introduces it as its own base ref; `c5c0578f` is the correct semantic base — the last substrate commit before this packet revision. The drift codex flagged is a false positive arising from inspecting branch HEAD without distinguishing substrate base from packet self-commit.

## New drift identified

None. Recommend L0 cycle-2 closure on codex-consult panel.

---
**Reviewer:** codex-consult (cycle 2)
**Status:** APPROVE — proceed to wave-level convergence check across 5 reviewer panels
