# Session prompt — N1 briefing L4 (claims power the daily surface)

Fresh-session kickoff for the L4 of the revived daily-briefing composition.
Written 2026-06-12 ~13:00 by the prior session at James's request. Verify any
state marked LIVE before relying on it — parallel workers were still running
at handoff.

## Context in one paragraph

v2.0 was trimmed (James, 2026-06-12) to a three-item Now list: **N1** wire the
daily briefing from claims, **N2** email→signal-only + retire EmailsPage,
**N3** retire WeekPage — everything else (Prompt/Library/Queue/chat) is
parked in `.docs/plans/v2.0-waves.html`. N1's implementation is DONE and
awaiting James's L4: the never-merged W4 vertical (daily-briefing +
meeting-detail composition producers, frontend consumption) was recovered
from archive branch commit `bf6dcdfd`, reconciled onto post-WR dev, and
pushed as **PR #452** (+18,528/−3,178, gauntlet green, reconciliation
re-review APPROVED). The W4 origin evidence — proof bundle, screenshots,
bounded review — is at `.docs/proofs/v1.5.0/W4-proof-bundle.md`.

## Read first
1. `.docs/plans/v2.0-waves.html` — the Now list (trimmed program; parked items).
2. `.docs/proofs/v1.5.0/W4-proof-bundle.md` — what the briefing composition
   is supposed to do (sections, trust bands, provenance, freshness, claim refs;
   composition id grammar `dailyos/daily-briefing:briefing:local~{yyyy-mm-dd}`;
   meeting tokens for `dailyos/meeting-detail`).
3. Linear DOS-883 (N1 ticket) + the L2 trail on DOS-852/DOS-866 for recent history.
4. Memory: `project-v20-shell-rebuild-decision` carries the full program state.

## The L4 itself
```
cd /private/tmp/dailyos-n1 && DAILYOS_DB_MODE=replica pnpm tauri dev
```
(worktree = PR #452's branch `n1-briefing-revival`, node_modules hydrated,
externalBin stub present.)

What to check, per the W4 acceptance evidence:
- **Daily briefing page**: the projected briefing composition renders ABOVE
  the legacy briefing content (deliberate side-by-side for comparison);
  producer-owned sections carry trust bands, rendered provenance, source
  freshness, claim refs; schedule/actions behavior below is unchanged.
- **Meeting detail** (any meeting): projected composition before the legacy
  intelligence chapters; prep AND recap states render
  (`CompositionKind::Custom`); the finite editorial ending survives.
- Known shapes to L4 against: empty day, meeting with no prep, degraded
  composition (`needs_verification` fallback), legacy-vs-projected drift.
- Memory rule: **L4 before L2-merge for user-facing** — James's eye gates the
  merge, per-chapter walkthrough style (as done for WR on 2026-06-10).

**On PASS:** merge PR #452 (squash, `gh pr merge 452 --squash`; repo
disallows --auto, CI ~15 min — watch checks, merge on green, VERIFY with
`gh pr view 452 -q .state` — never trust chain echoes). Then: ticket DOS-883
comment + schedule legacy-briefing-path deletion (only after the new one
survives daily use; DOS-278's one-canonical-candidate-set is the AC), prune
the n1 worktree+branch, and kick the **D-spine skin design session**
(James+Claude; DayStrip blocks at `.docs/design/reference/`, spec 2026-06-10)
plus DOS-869's rescoped instrumentation (reads/corrections/value-per-token on
the wired briefing).
**On FAIL:** findings → bounded codex fix cycle in the n1 worktree
(codex edits files; the session does git — codex sandboxes cannot write
worktree gitdirs under the main repo's `.git/`).

## Parallel/LIVE state at handoff (verify!)
- **N2/N3** (DOS-881/882): codex mid-implementation in
  `/private/tmp/dailyos-n2n3` (branch `n2n3-email-signal-week-retire`) —
  email LLM-enrichment removal + EmailsPage retirement (commit 1), WeekPage
  retirement (commit 2). On completion: verify gates + commits EXIST
  (`git log public/dev..HEAD`), push, PR, bounded L2. Output log:
  `/tmp/n2n3-codex.out`.
- **Deflation run** (post-rebuild, James's word): DOS-866's
  `intelligence_maintenance` cleanup command, `dry_run: true` first — review
  per-subject withdrawal counts with James, then apply. Targets the
  pathological generated-claim subjects (max was 2,143 claims on one subject).
- **Dev**: at `15796011`+ (866 merge + gate/lint/mirror fix). The drift gates
  now compare against **HEAD** (James-authorized 2026-06-12) — reference
  mirrors the branch beside it, not released main.
- **James's running app**: possibly still quiet mode
  (`DAILYOS_DISABLE_BACKGROUND_WORKERS=1`); post-866 rebuild makes normal
  mode safe.

## Open James-decisions carried over
1. Three gate-bypass authorizations (DOS-873 false-positive class): the
   devtools mock-fixture stash (`git stash list` → "devtools mock-fixture
   expansion"), the docs-consolidation archive commit
   (`~/Documents/dailyos-docs-consolidation`, 79 staged files), the two
   archive-branch pushes (`codex/v1.5.0-w4` — now partially superseded by the
   revival — and `codex/enrichment-pty-150s-hardening`).
2. DOS-288 bleed-gate mechanism-change one-line ack (from PR #450's L2).
3. Actions→Tasks rename (noted, deferred).
4. DOS-868 checkpoint fixes — next substrate item after the Now list; then
   the Gate-1 latency re-measurement.

## Operating rules learned the hard way today (binding)
- **No pipes in &&-chains** (`cmd | tail && next` masks failures — 4 false
  successes on 2026-06-12). `set -o pipefail`, capture to files, and VERIFY
  state after every push/commit/merge.
- **Codex edits files; the session does git** in worktrees (sandbox can't
  write gitdirs). Dispatch prompts must forbid `--no-verify` and credential
  lookup explicitly.
- Orchestrate via codex subagents (preferred) / lower-model agents; main loop
  = decomposition, gates, folds, James-facing synthesis.
- Don't over-engineer; don't anchor on James's latest remark (classify
  uncorroborated statements as context, not premises).
