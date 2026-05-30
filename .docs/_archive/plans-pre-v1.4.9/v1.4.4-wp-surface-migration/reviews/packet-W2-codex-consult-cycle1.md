# L0 codex-consult — W2 packet substrate verification (cycle 1)

**Packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md`
**Reviewer:** codex exec (gpt-5.5, xhigh reasoning)
**Date:** 2026-05-21
**Scope:** substrate-grep verification only (W1 commit SHAs, ADR citations, K-in record completeness, Linear ticket existence)

# VERDICT: PARTIAL (advisory)

All 4 checks pass on the substantive substrate question (every cited commit / ADR / Linear ID exists). Two minor drift flags surfaced that do not block the packet but should be reconciled before L1 kickoff.

## Check 1 — W1 commit SHAs (PASS)

All 15 SHAs exist and are reachable from `wave/v1.4.4-w1-stage1a`:

- DOS-459 `2b3915ef` — feat(abilities): get_entity_intelligence Read ability + EntityIntelligenceEnvelope
- DOS-460 `6444568c` — feat(abilities): canonical entity touchpoints + open-loops contract
- DOS-477 `e9b0ed41` — feat(entity_intelligence/auth): trust-boundary hardening + envelope-set validation
- DOS-477/8 `7bbbc6f7` — feat(claim_receipt): semantic feedback actions + envelope validation
- DOS-339 `ca8e7e21` — feat(claim_receipt): fan-out signal wiring + useClaimReceiptSubscription
- DOS-339/341/477 `0a586218` — fix(w1): L2 cycle-2 substrate patches
- DOS-340 `e74d49dc` — feat(claim_receipt): receipt vs operational audit boundary
- DOS-341 `e4d72de6` — feat(claim_receipt): privacy/redaction matrix + build_receipt_for_audience
- DOS-341 `6b437c5f` — Merge DOS-341 privacy into wave/v1.4.4-w1-stage1a
- DOS-507 `ed51d1b7` — feat(abilities): get_daily_briefing Read-only ability + composed BriefingState
- cycle-2/3 `00f38b3b`, `1a56d612`, `c2e857b8`, `aa48ffce`, `3e58bc13` — all reachable, subjects align

Reachability via `git branch --contains` confirms `* wave/v1.4.4-w1-stage1a` for every SHA.

## Check 2 — ADR citations (PASS)

All 5 ADRs exist at `.docs/decisions/`:

- ADR-0077: `0077-magazine-layout-editorial-redesign.md`
- ADR-0083: `0083-product-vocabulary.md`
- ADR-0129: TWO ADRs share this number — `0129-commitment-claim-identity.md` and `0129-composable-surfaces-wordpress-studio-as-primary-surface.md` (numbering collision, pre-existing, not introduced by W2)
- ADR-0130: `0130-surface-independent-composition-contract.md`
- ADR-0132: `0132-pill-primitive-dual-existence.md`

## Check 3 — K-in record completeness (PASS, with inventory drift)

`docs/solutions/` greps for `entity-detail`, `account-detail`, `project-detail`, `person-detail`, `gutenberg`, `inner-blocks`, `render-functions` returned **zero matches** — no surface-rendering K-out entries exist yet. Packet correctly does not reference what isn't there. K-out from W1 retro is the source of the next entries (per §3 cross-reference list).

Packet §3 K-in cross-references all 9 relevant existing entries (architecture-patterns L3-bypass, parallel-agent commit-hook, codex-dispatched no-changes, L0 review-loop diminishing returns, premise-check, k-in-grep, substrate-only landing, prompt-channel sensitivity, append-only JSONL).

## Check 4 — Linear ticket existence (PASS)

All 11 requested ticket IDs cited in packet body: DOS-462, DOS-483, DOS-484, DOS-328, DOS-688, DOS-689, DOS-690, DOS-691, DOS-692, DOS-693, DOS-725.

§5.4 Meeting Detail correctly acknowledges no Linear ticket exists: "**Linear ticket status:** no Linear ticket at packet-author time. **File at L1 kickoff** ..." — no surreptitious claim.

## Drift flags (advisory, non-blocking)

1. **K-in inventory count out of date.** Packet §3 line 75 states "16 .md files at scan time"; `git ls-files docs/solutions | grep '\.md$'` currently returns 24 paths (excluding architecture-patterns intra-dir). New entries since the packet scan: `hmac-canonical-change-requires-golden-vector-recompute-2026-05-20.md`, `codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md`, `wip-and-no-verify-escape-hatches-under-memory-pressure-2026-05-20.md`, plus other K-out from W1 close. Recommend re-running the K-in grep at L1 kickoff and updating §3.

2. **DOS-339 SHA labeling.** Packet table (lines 38, 556) cites `0243df65` as the DOS-339 base. `0243df65` is the MERGE commit `Merge feat/dos-339-claim-receipt-fanout into wave/v1.4.4-w1-stage1a`; the underlying substrate commit is `ca8e7e21` (`feat(claim_receipt): fan-out signal wiring + useClaimReceiptSubscription hook (DOS-339)`). Both reach the same tree. Packet phrasing "PR #323 base" is defensible (merge commit IS the PR-merge SHA), but a reader doing a git-log walk to inspect the diff hunks should be pointed at `ca8e7e21`. Recommend either footnoting the merge/feature distinction or swapping to `ca8e7e21` for grep consistency.

## L0 panel role

Codex-consult role: substrate verification. Verdict feeds the consensus alongside codex-challenge / architecture / design-lens / wp-skill panels. No BLOCKED findings; two advisory items above.

**Approve to advance.** Drift flags should be fixed inline before L0 close but do not require re-review.
