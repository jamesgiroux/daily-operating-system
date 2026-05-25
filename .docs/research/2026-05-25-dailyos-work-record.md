# DailyOS Work Record Since April 22, 2026

Generated: May 25, 2026

Window: April 22, 2026 00:00 America/Toronto through May 25, 2026. The shipped-code anchor is `public/dev` as fetched on May 25. Open PRs and local working-tree changes are called out separately.

## Headline Stats

| Metric | Count | Source |
| --- | ---: | --- |
| Commits landed on `public/dev` | 932 | `git rev-list --since` |
| Non-merge commits on `public/dev` | 831 | `git rev-list --no-merges --since` |
| PRs created | 159 | GitHub API, PR #223 through #381 |
| PRs merged | 147 | GitHub API, PR #225 through #379 |
| PRs still open | 5 | GitHub API |
| Gross line churn on `public/dev` | 797,711 | Git commit `--numstat` |
| Net tracked-line delta on `public/dev` | +613,909 | `git diff` from April 21 baseline to `public/dev` |
| Files changed in net diff | 4,042 | `git diff --numstat` |
| Current tracked text lines on `public/dev` | 1,158,665 | `git grep -I -n '' public/dev` |
| Baseline tracked text lines | 544,650 | same count at April 21 baseline |
| Source-ish net line delta, including JSON fixtures/config | +441,840 | filtered `git diff --numstat`, excluding docs and lockfile |
| Source-ish net line delta, excluding JSON | +365,730 | same filter, no `.json` |

## Pull Requests

All 159 PRs created in the window target `dev`.

| State | Count |
| --- | ---: |
| Merged | 147 |
| Open | 5 |
| Closed without merge | 7 |

Open PRs as of May 25:

| PR | Title |
| ---: | --- |
| #296 | `[PRESERVATION - DO NOT MERGE] v1.4.2 DOS-653 W4-Sub L0 V2 (BLOCKED) + take-stock + L4 chase artifacts` |
| #355 | `feat(mcp_v2): McpToolHandler request-scoped context + re-entrancy guard (DOS-758)` |
| #367 | `fix(account-detail): close WordPress visual parity gaps` |
| #380 | `feat(onboarding): collapse first-run to single-screen, 3 actions` |
| #381 | `Resolve path-alpha maintenance batch` |

Note: `public/dev` contains 53 explicit `Merge pull request #...` commits in the window. GitHub shows 147 merged PRs because many PRs landed via squash/rebase-style commits rather than explicit merge commits.

## Line Change Breakdown

Net diff from baseline commit `b8201997` (April 21, 2026 17:42 -0400) to `public/dev`:

| Root | Insertions | Deletions | Total churn | Files |
| --- | ---: | ---: | ---: | ---: |
| `src-tauri` | 354,009 | 14,647 | 368,656 | 1,969 |
| `.docs` | 158,100 | 819 | 158,919 | 907 |
| `wp` | 78,230 | 0 | 78,230 | 618 |
| `src` | 23,299 | 5,157 | 28,456 | 389 |
| `scripts` | 7,498 | 0 | 7,498 | 41 |
| `pnpm-lock.yaml` | 3,818 | 299 | 4,117 | 1 |
| `.github` | 3,162 | 82 | 3,244 | 26 |
| `docs` | 2,192 | 0 | 2,192 | 31 |

Top file types by churn:

| Type | Insertions | Deletions | Total churn | Files |
| --- | ---: | ---: | ---: | ---: |
| Rust `.rs` | 264,190 | 13,207 | 277,397 | 782 |
| Markdown `.md` | 102,321 | 962 | 103,283 | 669 |
| JSON `.json` | 78,891 | 585 | 79,476 | 984 |
| CSS `.css` | 64,278 | 370 | 64,648 | 417 |
| PHP `.php` | 34,955 | 0 | 34,955 | 250 |
| HTML `.html` | 23,110 | 6 | 23,116 | 86 |
| React `.tsx` | 12,441 | 3,802 | 16,243 | 222 |
| SQL `.sql` | 10,715 | 108 | 10,823 | 199 |

Open work not yet in `public/dev`:

| PR | Commits ahead of `public/dev` | Diffstat |
| ---: | ---: | --- |
| #380 | 1 | 380 files, +6,079 / -38,783 |
| #381 | 2 | 17 files, +959 / -80 |

## Token Usage

There are two different token stories. Do not combine these numbers.

### Coding-agent work

Source: local Codex JSONL session logs under `~/.codex/sessions` and `~/.codex/archived_sessions`, filtered to sessions whose working directory was `dailyos-repo` or a DailyOS worktree. The aggregation takes the final cumulative `token_count.total_token_usage` event per session.

| Metric | Count |
| --- | ---: |
| Sessions | 2,032 |
| Total tokens, including cached input | 22,372,756,881 |
| Input tokens | 22,316,747,484 |
| Cached input tokens | 21,606,747,904 |
| Output tokens | 56,009,397 |
| Reasoning output tokens | 21,649,847 |
| Non-cached input plus output | 766,008,977 |

By originator:

| Originator | Sessions | Total tokens |
| --- | ---: | ---: |
| Claude Code | 867 | 7,668,239,584 |
| Codex Desktop | 526 | 7,637,098,479 |
| codex-tui | 135 | 5,972,473,257 |
| codex_exec | 504 | 1,094,945,561 |

Top token days:

| Date | Sessions | Total tokens | Non-cached input plus output |
| --- | ---: | ---: | ---: |
| 2026-05-22 | 72 | 4,945,921,331 | 141,859,507 |
| 2026-05-11 | 131 | 4,889,552,541 | 140,108,829 |
| 2026-05-09 | 120 | 2,945,923,617 | 101,611,425 |
| 2026-05-10 | 69 | 1,247,342,004 | 22,437,684 |
| 2026-05-23 | 116 | 1,220,118,530 | 43,771,394 |

### DailyOS runtime AI calls

Source: `~/.dailyos/audit.log`, `ai_call_completed` events since April 22. These are DailyOS app estimates, not provider tokenizer counts. The current implementation estimates tokens by whitespace splitting in `src-tauri/src/pty.rs`.

| Metric | Count |
| --- | ---: |
| AI calls | 10,349 |
| Estimated total tokens | 12,994,894 |
| Estimated prompt tokens | 11,901,760 |
| Estimated output tokens | 1,093,134 |

By status:

| Status | Calls | Estimated tokens |
| --- | ---: | ---: |
| success | 9,576 | 7,120,268 |
| timeout | 699 | 5,630,873 |
| auth_required | 36 | 20,116 |
| subscription_limit | 20 | 157,289 |
| rate_limited | 18 | 66,348 |

Top runtime call sites:

| Call site | Calls | Estimated tokens |
| --- | ---: | ---: |
| `intel_queue:background_entity_enrichment` | 920 | 6,759,186 |
| `email:action_extraction` | 6,899 | 3,290,978 |
| `meeting_prep:agenda_enrichment` | 889 | 572,370 |
| `workflow:today_briefing_generation` | 497 | 544,046 |
| `workflow:today_email_enrichment` | 393 | 474,570 |

## Demo-friendly Narrative

Since April 22, DailyOS went from a fast-moving native app prototype into a much heavier personal-intelligence substrate:

- The Rust/Tauri backend absorbed the bulk of the work: claim-backed intelligence, trust/lifecycle mechanics, signal invalidation, MCP v2, runtime bridges, release gates, and workspace memory.
- The WordPress surface became a serious external runtime target, with block scaffolds, token mappings, visual parity work, and surface-runtime safety checks.
- The app moved toward a claim-backed intelligence loop: briefing and entity surfaces now consume attributed, lifecycle-aware intelligence instead of loose display-only data.
- Engineering discipline became part of the product system: L0/L2/L3 review protocol, release-gate fixtures, hook caching, suite evidence, and path-alpha maintenance batches.
- The current open work simplifies first-run onboarding and tightens path-alpha maintenance issues before the demo.

## Caveats

- Git stats are anchored to `public/dev`; open PRs #380 and #381 are not included in the shipped-line totals.
- Git line churn double-counts rewritten lines across commits by design. The net diff gives the repository growth story; gross churn gives the work-throughput story.
- Coding-agent token totals include cached input, which is useful for workload shape but not the same as billable uncached tokens.
- DailyOS runtime token totals are app-side estimates from the audit log. They are useful directionally, not exact provider accounting.
