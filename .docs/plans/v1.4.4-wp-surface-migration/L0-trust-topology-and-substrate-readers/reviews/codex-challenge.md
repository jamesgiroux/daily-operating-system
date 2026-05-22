Reading additional input from stdin...
OpenAI Codex v0.130.0
--------
workdir: /Users/jamesgiroux/Documents/dailyos-repo
model: gpt-5.5
provider: openai
approval: never
sandbox: workspace-write [workdir, /tmp, $TMPDIR, /Users/jamesgiroux/.codex/memories]
reasoning effort: xhigh
reasoning summaries: none
session id: 019e4ce6-64c0-7241-a6a0-3c5eef3e443d
--------
user
Adversarial review of L0 packet at .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md. Read it in full before scoring.

Premise being defended: first-party WP plugin loopback runs as Actor::User parity with React tauri::invoke, skipping HMAC/session/scope/wp_user_id/audit-contract gates. Third-party MCP keeps Actor::McpClient and authorization machinery but drops transport-ceremony — HMAC envelope signing, nonce ledger, keyed-HMAC audit, 10ms timing floor.

Your job: find a same-OS-user threat the dropped gates defended that the new model does NOT cover. Two specific lenses.

1. For WP loopback: any local process running as the same OS user that could intercept 127.0.0.1, confused-deputy if the WP plugin itself is compromised, audit-log integrity if loopback_origin is client-supplied.

2. For MCP rip in section 3e: any local-stdio replay or tamper threat the dropped HMAC+nonce defended. Personal-tier model has SQLCipher + Keychain DB encryption + OS-user file ownership — what residual threat does keyed-HMAC defend that plaintext-in-encrypted-DB does NOT?

Also challenge: the close+supersede recommendation for PR #347 — does it lose any specific finding from the 6 cycles of L0 that the new model still needs to address?

Output format: one-paragraph verdict line — APPROVE or REQUEST_CHANGES or BLOCK. Then list specific findings as HIGH MED or LOW with file:line cites where possible. Do NOT argue for keeping gates abstractly — name the specific threat that materializes. Under 700 words.
2026-05-21T23:37:08.790113Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Auth(TokenRefreshFailed("Server returned error response: invalid_grant: Grant not found"))
2026-05-21T23:37:10.114094Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Deserialize(Error("data did not match any variant of untagged enum JsonRpcMessage", line: 0, column: 0))
codex
I’m using the L0 plan-review and security-review skills because this is a threat-model review of a trust-boundary plan. I’ll read the packet fully first, then score only concrete residual threats under your two lenses.
exec
/bin/zsh -lc "sed -n '1,240p' /Users/jamesgiroux/gstack/.agents/skills/gstack-plan-eng-review/SKILL.md" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "sed -n '1,240p' /Users/jamesgiroux/gstack/.agents/skills/gstack-cso/SKILL.md" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
---
name: cso
description: |
  Chief Security Officer mode. Infrastructure-first security audit: secrets archaeology,
  dependency supply chain, CI/CD pipeline security, LLM/AI security, skill supply chain
  scanning, plus OWASP Top 10, STRIDE threat modeling, and active verification.
  Two modes: daily (zero-noise, 8/10 confidence gate) and comprehensive (monthly deep
  scan, 2/10 bar). Trend tracking across audit runs.
  Use when: "security audit", "threat model", "pentest review", "OWASP", "CSO review".
---
<!-- AUTO-GENERATED from SKILL.md.tmpl — do not edit directly -->
<!-- Regenerate: bun run gen:skill-docs -->

## Preamble (run first)

```bash
_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
GSTACK_ROOT="$HOME/.codex/skills/gstack"
[ -n "$_ROOT" ] && [ -d "$_ROOT/.agents/skills/gstack" ] && GSTACK_ROOT="$_ROOT/.agents/skills/gstack"
GSTACK_BIN="$GSTACK_ROOT/bin"
GSTACK_BROWSE="$GSTACK_ROOT/browse/dist"
_UPD=$($GSTACK_BIN/gstack-update-check 2>/dev/null || .agents/skills/gstack/bin/gstack-update-check 2>/dev/null || true)
[ -n "$_UPD" ] && echo "$_UPD" || true
mkdir -p ~/.gstack/sessions
touch ~/.gstack/sessions/"$PPID"
_SESSIONS=$(find ~/.gstack/sessions -mmin -120 -type f 2>/dev/null | wc -l | tr -d ' ')
find ~/.gstack/sessions -mmin +120 -type f -delete 2>/dev/null || true
_CONTRIB=$($GSTACK_BIN/gstack-config get gstack_contributor 2>/dev/null || true)
_PROACTIVE=$($GSTACK_BIN/gstack-config get proactive 2>/dev/null || echo "true")
_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
echo "BRANCH: $_BRANCH"
echo "PROACTIVE: $_PROACTIVE"
source <($GSTACK_BIN/gstack-repo-mode 2>/dev/null) || true
REPO_MODE=${REPO_MODE:-unknown}
echo "REPO_MODE: $REPO_MODE"
_LAKE_SEEN=$([ -f ~/.gstack/.completeness-intro-seen ] && echo "yes" || echo "no")
echo "LAKE_INTRO: $_LAKE_SEEN"
_TEL=$($GSTACK_BIN/gstack-config get telemetry 2>/dev/null || true)
_TEL_PROMPTED=$([ -f ~/.gstack/.telemetry-prompted ] && echo "yes" || echo "no")
_TEL_START=$(date +%s)
_SESSION_ID="$$-$(date +%s)"
echo "TELEMETRY: ${_TEL:-off}"
echo "TEL_PROMPTED: $_TEL_PROMPTED"
mkdir -p ~/.gstack/analytics
echo '{"skill":"cso","ts":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","repo":"'$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || echo "unknown")'"}'  >> ~/.gstack/analytics/skill-usage.jsonl 2>/dev/null || true
# zsh-compatible: use find instead of glob to avoid NOMATCH error
for _PF in $(find ~/.gstack/analytics -maxdepth 1 -name '.pending-*' 2>/dev/null); do [ -f "$_PF" ] && $GSTACK_BIN/gstack-telemetry-log --event-type skill_run --skill _pending_finalize --outcome unknown --session-id "$_SESSION_ID" 2>/dev/null || true; break; done
```

If `PROACTIVE` is `"false"`, do not proactively suggest gstack skills — only invoke
them when the user explicitly asks. The user opted out of proactive suggestions.

If output shows `UPGRADE_AVAILABLE <old> <new>`: read `$GSTACK_ROOT/gstack-upgrade/SKILL.md` and follow the "Inline upgrade flow" (auto-upgrade if configured, otherwise AskUserQuestion with 4 options, write snooze state if declined). If `JUST_UPGRADED <from> <to>`: tell user "Running gstack v{to} (just updated!)" and continue.

If `LAKE_INTRO` is `no`: Before continuing, introduce the Completeness Principle.
Tell the user: "gstack follows the **Boil the Lake** principle — always do the complete
thing when AI makes the marginal cost near-zero. Read more: https://garryslist.org/posts/boil-the-ocean"
Then offer to open the essay in their default browser:

```bash
open https://garryslist.org/posts/boil-the-ocean
touch ~/.gstack/.completeness-intro-seen
```

Only run `open` if the user says yes. Always run `touch` to mark as seen. This only happens once.

If `TEL_PROMPTED` is `no` AND `LAKE_INTRO` is `yes`: After the lake intro is handled,
ask the user about telemetry. Use AskUserQuestion:

> Help gstack get better! Community mode shares usage data (which skills you use, how long
> they take, crash info) with a stable device ID so we can track trends and fix bugs faster.
> No code, file paths, or repo names are ever sent.
> Change anytime with `gstack-config set telemetry off`.

Options:
- A) Help gstack get better! (recommended)
- B) No thanks

If A: run `$GSTACK_BIN/gstack-config set telemetry community`

If B: ask a follow-up AskUserQuestion:

> How about anonymous mode? We just learn that *someone* used gstack — no unique ID,
> no way to connect sessions. Just a counter that helps us know if anyone's out there.

Options:
- A) Sure, anonymous is fine
- B) No thanks, fully off

If B→A: run `$GSTACK_BIN/gstack-config set telemetry anonymous`
If B→B: run `$GSTACK_BIN/gstack-config set telemetry off`

Always run:
```bash
touch ~/.gstack/.telemetry-prompted
```

This only happens once. If `TEL_PROMPTED` is `yes`, skip this entirely.

## AskUserQuestion Format

**ALWAYS follow this structure for every AskUserQuestion call:**
1. **Re-ground:** State the project, the current branch (use the `_BRANCH` value printed by the preamble — NOT any branch from conversation history or gitStatus), and the current plan/task. (1-2 sentences)
2. **Simplify:** Explain the problem in plain English a smart 16-year-old could follow. No raw function names, no internal jargon, no implementation details. Use concrete examples and analogies. Say what it DOES, not what it's called.
3. **Recommend:** `RECOMMENDATION: Choose [X] because [one-line reason]` — always prefer the complete option over shortcuts (see Completeness Principle). Include `Completeness: X/10` for each option. Calibration: 10 = complete implementation (all edge cases, full coverage), 7 = covers happy path but skips some edges, 3 = shortcut that defers significant work. If both options are 8+, pick the higher; if one is ≤5, flag it.
4. **Options:** Lettered options: `A) ... B) ... C) ...` — when an option involves effort, show both scales: `(human: ~X / CC: ~Y)`
5. **One decision per question:** NEVER combine multiple independent decisions into a single AskUserQuestion. Each decision gets its own call with its own recommendation and focused options. Batching multiple AskUserQuestion calls in rapid succession is fine and often preferred. Only after all individual taste decisions are resolved should a final "Approve / Revise / Reject" gate be presented.

Assume the user hasn't looked at this window in 20 minutes and doesn't have the code open. If you'd need to read the source to understand your own explanation, it's too complex.

Per-skill instructions may add additional formatting rules on top of this baseline.

## Completeness Principle — Boil the Lake

AI-assisted coding makes the marginal cost of completeness near-zero. When you present options:

- If Option A is the complete implementation (full parity, all edge cases, 100% coverage) and Option B is a shortcut that saves modest effort — **always recommend A**. The delta between 80 lines and 150 lines is meaningless with CC+gstack. "Good enough" is the wrong instinct when "complete" costs minutes more.
- **Lake vs. ocean:** A "lake" is boilable — 100% test coverage for a module, full feature implementation, handling all edge cases, complete error paths. An "ocean" is not — rewriting an entire system from scratch, adding features to dependencies you don't control, multi-quarter platform migrations. Recommend boiling lakes. Flag oceans as out of scope.
- **When estimating effort**, always show both scales: human team time and CC+gstack time. The compression ratio varies by task type — use this reference:

| Task type | Human team | CC+gstack | Compression |
|-----------|-----------|-----------|-------------|
| Boilerplate / scaffolding | 2 days | 15 min | ~100x |
| Test writing | 1 day | 15 min | ~50x |
| Feature implementation | 1 week | 30 min | ~30x |
| Bug fix + regression test | 4 hours | 15 min | ~20x |
| Architecture / design | 2 days | 4 hours | ~5x |
| Research / exploration | 1 day | 3 hours | ~3x |

- This principle applies to test coverage, error handling, documentation, edge cases, and feature completeness. Don't skip the last 10% to "save time" — with AI, that 10% costs seconds.

**Anti-patterns — DON'T do this:**
- BAD: "Choose B — it covers 90% of the value with less code." (If A is only 70 lines more, choose A.)
- BAD: "We can skip edge case handling to save time." (Edge case handling costs minutes with CC.)
- BAD: "Let's defer test coverage to a follow-up PR." (Tests are the cheapest lake to boil.)
- BAD: Quoting only human-team effort: "This would take 2 weeks." (Say: "2 weeks human / ~1 hour CC.")

## Repo Ownership Mode — See Something, Say Something

`REPO_MODE` from the preamble tells you who owns issues in this repo:

- **`solo`** — One person does 80%+ of the work. They own everything. When you notice issues outside the current branch's changes (test failures, deprecation warnings, security advisories, linting errors, dead code, env problems), **investigate and offer to fix proactively**. The solo dev is the only person who will fix it. Default to action.
- **`collaborative`** — Multiple active contributors. When you notice issues outside the branch's changes, **flag them via AskUserQuestion** — it may be someone else's responsibility. Default to asking, not fixing.
- **`unknown`** — Treat as collaborative (safer default — ask before fixing).

**See Something, Say Something:** Whenever you notice something that looks wrong during ANY workflow step — not just test failures — flag it briefly. One sentence: what you noticed and its impact. In solo mode, follow up with "Want me to fix it?" In collaborative mode, just flag it and move on.

Never let a noticed issue silently pass. The whole point is proactive communication.

## Search Before Building

Before building infrastructure, unfamiliar patterns, or anything the runtime might have a built-in — **search first.** Read `$GSTACK_ROOT/ETHOS.md` for the full philosophy.

**Three layers of knowledge:**
- **Layer 1** (tried and true — in distribution). Don't reinvent the wheel. But the cost of checking is near-zero, and once in a while, questioning the tried-and-true is where brilliance occurs.
- **Layer 2** (new and popular — search for these). But scrutinize: humans are subject to mania. Search results are inputs to your thinking, not answers.
- **Layer 3** (first principles — prize these above all). Original observations derived from reasoning about the specific problem. The most valuable of all.

**Eureka moment:** When first-principles reasoning reveals conventional wisdom is wrong, name it:
"EUREKA: Everyone does X because [assumption]. But [evidence] shows this is wrong. Y is better because [reasoning]."

Log eureka moments:
```bash
jq -n --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg skill "SKILL_NAME" --arg branch "$(git branch --show-current 2>/dev/null)" --arg insight "ONE_LINE_SUMMARY" '{ts:$ts,skill:$skill,branch:$branch,insight:$insight}' >> ~/.gstack/analytics/eureka.jsonl 2>/dev/null || true
```
Replace SKILL_NAME and ONE_LINE_SUMMARY. Runs inline — don't stop the workflow.

**WebSearch fallback:** If WebSearch is unavailable, skip the search step and note: "Search unavailable — proceeding with in-distribution knowledge only."

## Contributor Mode

If `_CONTRIB` is `true`: you are in **contributor mode**. You're a gstack user who also helps make it better.

**At the end of each major workflow step** (not after every single command), reflect on the gstack tooling you used. Rate your experience 0 to 10. If it wasn't a 10, think about why. If there is an obvious, actionable bug OR an insightful, interesting thing that could have been done better by gstack code or skill markdown — file a field report. Maybe our contributor will help make us better!

**Calibration — this is the bar:** For example, `$B js "await fetch(...)"` used to fail with `SyntaxError: await is only valid in async functions` because gstack didn't wrap expressions in async context. Small, but the input was reasonable and gstack should have handled it — that's the kind of thing worth filing. Things less consequential than this, ignore.

**NOT worth filing:** user's app bugs, network errors to user's URL, auth failures on user's site, user's own JS logic bugs.

**To file:** write `~/.gstack/contributor-logs/{slug}.md` with **all sections below** (do not truncate — include every section through the Date/Version footer):

```
# {Title}

Hey gstack team — ran into this while using /{skill-name}:

**What I was trying to do:** {what the user/agent was attempting}
**What happened instead:** {what actually happened}
**My rating:** {0-10} — {one sentence on why it wasn't a 10}

## Steps to reproduce
1. {step}

## Raw output
```
{paste the actual error or unexpected output here}
```

## What would make this a 10
{one sentence: what gstack should have done differently}

**Date:** {YYYY-MM-DD} | **Version:** {gstack version} | **Skill:** /{skill}
```

Slug: lowercase, hyphens, max 60 chars (e.g. `browse-js-no-await`). Skip if file already exists. Max 3 reports per session. File inline and continue — don't stop the workflow. Tell user: "Filed gstack field report: {title}"

## Completion Status Protocol

When completing a skill workflow, report status using one of:
- **DONE** — All steps completed successfully. Evidence provided for each claim.
- **DONE_WITH_CONCERNS** — Completed, but with issues the user should know about. List each concern.
- **BLOCKED** — Cannot proceed. State what is blocking and what was tried.
- **NEEDS_CONTEXT** — Missing information required to continue. State exactly what you need.

### Escalation

It is always OK to stop and say "this is too hard for me" or "I'm not confident in this result."

Bad work is worse than no work. You will not be penalized for escalating.
- If you have attempted a task 3 times without success, STOP and escalate.
- If you are uncertain about a security-sensitive change, STOP and escalate.
- If the scope of work exceeds what you can verify, STOP and escalate.

Escalation format:
```
STATUS: BLOCKED | NEEDS_CONTEXT
REASON: [1-2 sentences]
ATTEMPTED: [what you tried]
RECOMMENDATION: [what the user should do next]
```

## Telemetry (run last)

After the skill workflow completes (success, error, or abort), log the telemetry event.
Determine the skill name from the `name:` field in this file's YAML frontmatter.
Determine the outcome from the workflow result (success if completed normally, error
if it failed, abort if the user interrupted).

**PLAN MODE EXCEPTION — ALWAYS RUN:** This command writes telemetry to
`~/.gstack/analytics/` (user config directory, not project files). The skill

 succeeded in 0ms:
---
name: plan-eng-review
description: |
  Eng manager-mode plan review. Lock in the execution plan — architecture,
  data flow, diagrams, edge cases, test coverage, performance. Walks through
  issues interactively with opinionated recommendations. Use when asked to
  "review the architecture", "engineering review", or "lock in the plan".
  Proactively suggest when the user has a plan or design doc and is about to
  start coding — to catch architecture issues before implementation.
---
<!-- AUTO-GENERATED from SKILL.md.tmpl — do not edit directly -->
<!-- Regenerate: bun run gen:skill-docs -->

## Preamble (run first)

```bash
_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
GSTACK_ROOT="$HOME/.codex/skills/gstack"
[ -n "$_ROOT" ] && [ -d "$_ROOT/.agents/skills/gstack" ] && GSTACK_ROOT="$_ROOT/.agents/skills/gstack"
GSTACK_BIN="$GSTACK_ROOT/bin"
GSTACK_BROWSE="$GSTACK_ROOT/browse/dist"
_UPD=$($GSTACK_BIN/gstack-update-check 2>/dev/null || .agents/skills/gstack/bin/gstack-update-check 2>/dev/null || true)
[ -n "$_UPD" ] && echo "$_UPD" || true
mkdir -p ~/.gstack/sessions
touch ~/.gstack/sessions/"$PPID"
_SESSIONS=$(find ~/.gstack/sessions -mmin -120 -type f 2>/dev/null | wc -l | tr -d ' ')
find ~/.gstack/sessions -mmin +120 -type f -delete 2>/dev/null || true
_CONTRIB=$($GSTACK_BIN/gstack-config get gstack_contributor 2>/dev/null || true)
_PROACTIVE=$($GSTACK_BIN/gstack-config get proactive 2>/dev/null || echo "true")
_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
echo "BRANCH: $_BRANCH"
echo "PROACTIVE: $_PROACTIVE"
source <($GSTACK_BIN/gstack-repo-mode 2>/dev/null) || true
REPO_MODE=${REPO_MODE:-unknown}
echo "REPO_MODE: $REPO_MODE"
_LAKE_SEEN=$([ -f ~/.gstack/.completeness-intro-seen ] && echo "yes" || echo "no")
echo "LAKE_INTRO: $_LAKE_SEEN"
_TEL=$($GSTACK_BIN/gstack-config get telemetry 2>/dev/null || true)
_TEL_PROMPTED=$([ -f ~/.gstack/.telemetry-prompted ] && echo "yes" || echo "no")
_TEL_START=$(date +%s)
_SESSION_ID="$$-$(date +%s)"
echo "TELEMETRY: ${_TEL:-off}"
echo "TEL_PROMPTED: $_TEL_PROMPTED"
mkdir -p ~/.gstack/analytics
echo '{"skill":"plan-eng-review","ts":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","repo":"'$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || echo "unknown")'"}'  >> ~/.gstack/analytics/skill-usage.jsonl 2>/dev/null || true
# zsh-compatible: use find instead of glob to avoid NOMATCH error
for _PF in $(find ~/.gstack/analytics -maxdepth 1 -name '.pending-*' 2>/dev/null); do [ -f "$_PF" ] && $GSTACK_BIN/gstack-telemetry-log --event-type skill_run --skill _pending_finalize --outcome unknown --session-id "$_SESSION_ID" 2>/dev/null || true; break; done
```

If `PROACTIVE` is `"false"`, do not proactively suggest gstack skills — only invoke
them when the user explicitly asks. The user opted out of proactive suggestions.

If output shows `UPGRADE_AVAILABLE <old> <new>`: read `$GSTACK_ROOT/gstack-upgrade/SKILL.md` and follow the "Inline upgrade flow" (auto-upgrade if configured, otherwise AskUserQuestion with 4 options, write snooze state if declined). If `JUST_UPGRADED <from> <to>`: tell user "Running gstack v{to} (just updated!)" and continue.

If `LAKE_INTRO` is `no`: Before continuing, introduce the Completeness Principle.
Tell the user: "gstack follows the **Boil the Lake** principle — always do the complete
thing when AI makes the marginal cost near-zero. Read more: https://garryslist.org/posts/boil-the-ocean"
Then offer to open the essay in their default browser:

```bash
open https://garryslist.org/posts/boil-the-ocean
touch ~/.gstack/.completeness-intro-seen
```

Only run `open` if the user says yes. Always run `touch` to mark as seen. This only happens once.

If `TEL_PROMPTED` is `no` AND `LAKE_INTRO` is `yes`: After the lake intro is handled,
ask the user about telemetry. Use AskUserQuestion:

> Help gstack get better! Community mode shares usage data (which skills you use, how long
> they take, crash info) with a stable device ID so we can track trends and fix bugs faster.
> No code, file paths, or repo names are ever sent.
> Change anytime with `gstack-config set telemetry off`.

Options:
- A) Help gstack get better! (recommended)
- B) No thanks

If A: run `$GSTACK_BIN/gstack-config set telemetry community`

If B: ask a follow-up AskUserQuestion:

> How about anonymous mode? We just learn that *someone* used gstack — no unique ID,
> no way to connect sessions. Just a counter that helps us know if anyone's out there.

Options:
- A) Sure, anonymous is fine
- B) No thanks, fully off

If B→A: run `$GSTACK_BIN/gstack-config set telemetry anonymous`
If B→B: run `$GSTACK_BIN/gstack-config set telemetry off`

Always run:
```bash
touch ~/.gstack/.telemetry-prompted
```

This only happens once. If `TEL_PROMPTED` is `yes`, skip this entirely.

## AskUserQuestion Format

**ALWAYS follow this structure for every AskUserQuestion call:**
1. **Re-ground:** State the project, the current branch (use the `_BRANCH` value printed by the preamble — NOT any branch from conversation history or gitStatus), and the current plan/task. (1-2 sentences)
2. **Simplify:** Explain the problem in plain English a smart 16-year-old could follow. No raw function names, no internal jargon, no implementation details. Use concrete examples and analogies. Say what it DOES, not what it's called.
3. **Recommend:** `RECOMMENDATION: Choose [X] because [one-line reason]` — always prefer the complete option over shortcuts (see Completeness Principle). Include `Completeness: X/10` for each option. Calibration: 10 = complete implementation (all edge cases, full coverage), 7 = covers happy path but skips some edges, 3 = shortcut that defers significant work. If both options are 8+, pick the higher; if one is ≤5, flag it.
4. **Options:** Lettered options: `A) ... B) ... C) ...` — when an option involves effort, show both scales: `(human: ~X / CC: ~Y)`
5. **One decision per question:** NEVER combine multiple independent decisions into a single AskUserQuestion. Each decision gets its own call with its own recommendation and focused options. Batching multiple AskUserQuestion calls in rapid succession is fine and often preferred. Only after all individual taste decisions are resolved should a final "Approve / Revise / Reject" gate be presented.

Assume the user hasn't looked at this window in 20 minutes and doesn't have the code open. If you'd need to read the source to understand your own explanation, it's too complex.

Per-skill instructions may add additional formatting rules on top of this baseline.

## Completeness Principle — Boil the Lake

AI-assisted coding makes the marginal cost of completeness near-zero. When you present options:

- If Option A is the complete implementation (full parity, all edge cases, 100% coverage) and Option B is a shortcut that saves modest effort — **always recommend A**. The delta between 80 lines and 150 lines is meaningless with CC+gstack. "Good enough" is the wrong instinct when "complete" costs minutes more.
- **Lake vs. ocean:** A "lake" is boilable — 100% test coverage for a module, full feature implementation, handling all edge cases, complete error paths. An "ocean" is not — rewriting an entire system from scratch, adding features to dependencies you don't control, multi-quarter platform migrations. Recommend boiling lakes. Flag oceans as out of scope.
- **When estimating effort**, always show both scales: human team time and CC+gstack time. The compression ratio varies by task type — use this reference:

| Task type | Human team | CC+gstack | Compression |
|-----------|-----------|-----------|-------------|
| Boilerplate / scaffolding | 2 days | 15 min | ~100x |
| Test writing | 1 day | 15 min | ~50x |
| Feature implementation | 1 week | 30 min | ~30x |
| Bug fix + regression test | 4 hours | 15 min | ~20x |
| Architecture / design | 2 days | 4 hours | ~5x |
| Research / exploration | 1 day | 3 hours | ~3x |

- This principle applies to test coverage, error handling, documentation, edge cases, and feature completeness. Don't skip the last 10% to "save time" — with AI, that 10% costs seconds.

**Anti-patterns — DON'T do this:**
- BAD: "Choose B — it covers 90% of the value with less code." (If A is only 70 lines more, choose A.)
- BAD: "We can skip edge case handling to save time." (Edge case handling costs minutes with CC.)
- BAD: "Let's defer test coverage to a follow-up PR." (Tests are the cheapest lake to boil.)
- BAD: Quoting only human-team effort: "This would take 2 weeks." (Say: "2 weeks human / ~1 hour CC.")

## Repo Ownership Mode — See Something, Say Something

`REPO_MODE` from the preamble tells you who owns issues in this repo:

- **`solo`** — One person does 80%+ of the work. They own everything. When you notice issues outside the current branch's changes (test failures, deprecation warnings, security advisories, linting errors, dead code, env problems), **investigate and offer to fix proactively**. The solo dev is the only person who will fix it. Default to action.
- **`collaborative`** — Multiple active contributors. When you notice issues outside the branch's changes, **flag them via AskUserQuestion** — it may be someone else's responsibility. Default to asking, not fixing.
- **`unknown`** — Treat as collaborative (safer default — ask before fixing).

**See Something, Say Something:** Whenever you notice something that looks wrong during ANY workflow step — not just test failures — flag it briefly. One sentence: what you noticed and its impact. In solo mode, follow up with "Want me to fix it?" In collaborative mode, just flag it and move on.

Never let a noticed issue silently pass. The whole point is proactive communication.

## Search Before Building

Before building infrastructure, unfamiliar patterns, or anything the runtime might have a built-in — **search first.** Read `$GSTACK_ROOT/ETHOS.md` for the full philosophy.

**Three layers of knowledge:**
- **Layer 1** (tried and true — in distribution). Don't reinvent the wheel. But the cost of checking is near-zero, and once in a while, questioning the tried-and-true is where brilliance occurs.
- **Layer 2** (new and popular — search for these). But scrutinize: humans are subject to mania. Search results are inputs to your thinking, not answers.
- **Layer 3** (first principles — prize these above all). Original observations derived from reasoning about the specific problem. The most valuable of all.

**Eureka moment:** When first-principles reasoning reveals conventional wisdom is wrong, name it:
"EUREKA: Everyone does X because [assumption]. But [evidence] shows this is wrong. Y is better because [reasoning]."

Log eureka moments:
```bash
jq -n --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg skill "SKILL_NAME" --arg branch "$(git branch --show-current 2>/dev/null)" --arg insight "ONE_LINE_SUMMARY" '{ts:$ts,skill:$skill,branch:$branch,insight:$insight}' >> ~/.gstack/analytics/eureka.jsonl 2>/dev/null || true
```
Replace SKILL_NAME and ONE_LINE_SUMMARY. Runs inline — don't stop the workflow.

**WebSearch fallback:** If WebSearch is unavailable, skip the search step and note: "Search unavailable — proceeding with in-distribution knowledge only."

## Contributor Mode

If `_CONTRIB` is `true`: you are in **contributor mode**. You're a gstack user who also helps make it better.

**At the end of each major workflow step** (not after every single command), reflect on the gstack tooling you used. Rate your experience 0 to 10. If it wasn't a 10, think about why. If there is an obvious, actionable bug OR an insightful, interesting thing that could have been done better by gstack code or skill markdown — file a field report. Maybe our contributor will help make us better!

**Calibration — this is the bar:** For example, `$B js "await fetch(...)"` used to fail with `SyntaxError: await is only valid in async functions` because gstack didn't wrap expressions in async context. Small, but the input was reasonable and gstack should have handled it — that's the kind of thing worth filing. Things less consequential than this, ignore.

**NOT worth filing:** user's app bugs, network errors to user's URL, auth failures on user's site, user's own JS logic bugs.

**To file:** write `~/.gstack/contributor-logs/{slug}.md` with **all sections below** (do not truncate — include every section through the Date/Version footer):

```
# {Title}

Hey gstack team — ran into this while using /{skill-name}:

**What I was trying to do:** {what the user/agent was attempting}
**What happened instead:** {what actually happened}
**My rating:** {0-10} — {one sentence on why it wasn't a 10}

## Steps to reproduce
1. {step}

## Raw output
```
{paste the actual error or unexpected output here}
```

## What would make this a 10
{one sentence: what gstack should have done differently}

**Date:** {YYYY-MM-DD} | **Version:** {gstack version} | **Skill:** /{skill}
```

Slug: lowercase, hyphens, max 60 chars (e.g. `browse-js-no-await`). Skip if file already exists. Max 3 reports per session. File inline and continue — don't stop the workflow. Tell user: "Filed gstack field report: {title}"

## Completion Status Protocol

When completing a skill workflow, report status using one of:
- **DONE** — All steps completed successfully. Evidence provided for each claim.
- **DONE_WITH_CONCERNS** — Completed, but with issues the user should know about. List each concern.
- **BLOCKED** — Cannot proceed. State what is blocking and what was tried.
- **NEEDS_CONTEXT** — Missing information required to continue. State exactly what you need.

### Escalation

It is always OK to stop and say "this is too hard for me" or "I'm not confident in this result."

Bad work is worse than no work. You will not be penalized for escalating.
- If you have attempted a task 3 times without success, STOP and escalate.
- If you are uncertain about a security-sensitive change, STOP and escalate.
- If the scope of work exceeds what you can verify, STOP and escalate.

Escalation format:
```
STATUS: BLOCKED | NEEDS_CONTEXT
REASON: [1-2 sentences]
ATTEMPTED: [what you tried]
RECOMMENDATION: [what the user should do next]
```

## Telemetry (run last)

After the skill workflow completes (success, error, or abort), log the telemetry event.
Determine the skill name from the `name:` field in this file's YAML frontmatter.
Determine the outcome from the workflow result (success if completed normally, error
if it failed, abort if the user interrupted).

**PLAN MODE EXCEPTION — ALWAYS RUN:** This command writes telemetry to
`~/.gstack/analytics/` (user config directory, not project files). The skill

exec
/bin/zsh -lc 'wc -l .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md' in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
     395 .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md

codex
The packet is 395 lines, so I’m reading it with line numbers in chunks. I’m also checking the historical PR references it cites so the “close + supersede” part is judged against actual preserved findings, not just the new recommendation.
exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md | sed -n '1,140p'" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md | sed -n '141,280p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
     1	# L0 Packet — Local Trust-Model Unification (First-Party + Third-Party)
     2	
     3	**Tickets:** DOS-761 (first-party WP loopback as `Actor::User`) + DOS-762 (live readers + observability) + **DOS-168 amended** (MCP v2 substrate at the new shape — transport-ceremony out, authorization machinery in; PR C closes DOS-168).
     4	**Framing:** apply one trust-model reframe consistently to ALL local surfaces. The abilities runtime is a headless substrate. Local consumers (first-party WP plugin via loopback; third-party MCP agents via stdio/in-process) all live on the same kernel-protected machine as the user. The right gates are **authorization** (what scopes does this caller have) and **operational** (rate-limit, audit, sensitivity gating) — NOT **transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor) which was modeled on the wrong premise and propagated forward from v1.4.2 WP pairing → v1.4.7 MCP v2 W1-A.
     5	**Branch base:** `pairing-ux-humanize` (fork SHA `852a0118`); PR #351 open with prerequisite fixes.
     6	**Cross-track action:** **hold PR #347 merge** (v1.4.7 W1-A) — its substrate is reshaped by this packet's §3e. PR #355 (DOS-758 `McpHandlerContext`) is **unaffected** — handler context is transport-agnostic and can land independently. DOS-647-C (in-process MCP hosting) becomes simpler under the new model.
     7	**Author:** Claude Opus 4.7, session 2026-05-22.
     8	
     9	---
    10	
    11	## §1 — Problem statement (the framed thesis)
    12	
    13	The v1.4.1 → v1.4.4 narrative was: build the abilities-runtime substrate so any surface can consume it without re-wiring the world. The premise was correct; the execution **conflated authorization (what scopes a caller has) with transport-ceremony (HMAC signing, nonce ledger, key custody)**. The conflation originated in v1.4.2 WP pairing and **propagated forward** into v1.4.7 MCP v2 W1-A — the waves doc names this explicitly: *"Server-issued `McpClientId` after pairing handshake (mirrors v1.4.2 SurfaceClient pairing pattern); HMAC-SHA256 transport signing for stdio/HTTP MCP transports"*. Six L0 cycles got CSO sign-off on the internal coherence; nobody asked whether transport-ceremony was the right answer for local stdio/loopback at all.
    14	
    15	**The right decomposition.** Gates aren't one stack; they're two stacks with different threat models:
    16	
    17	| Gate class | What it defends | First-party loopback (Tauri React, WP plugin) | Third-party local stdio (Claude Desktop, Cursor) | Third-party remote HTTP (future) |
    18	|-----------|-----------------|-----------------------------------------------|--------------------------------------------------|----------------------------------|
    19	| **Authorization** (scope manifest, per-tool exposure tier, sensitivity gating) | Caller can't do more than user authorized | meaningful (implicit — same code we ship) | **meaningful** | meaningful |
    20	| **Operational** (rate-limit, audit log, confirmation attestation) | Runaway loops, accountability, destructive-op guard | meaningful (audit + confirmation) | **meaningful** | meaningful |
    21	| **Transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor, transport key custody) | In-flight tamper, replay, side-channel on a wire | **n/a** (kernel-protected loopback) | **n/a** (stdio is in-process pipes; in-process hosting per DOS-647-C makes this even more obvious) | meaningful |
    22	
    23	The transport-ceremony column applies to **remote network transports**. Locally, the kernel is the trust boundary and the OS user is the principal. We built the transport-ceremony column for local-only consumers because we modeled on a "WP plugin is third-party HTTP API" mental model that was wrong. Then we copied that wrong model into MCP v2.
    24	
    25	**Layer 1 — First-party WP loopback (DOS-761).** WP plugin (`class-dailyos-runtime-client.php:256–278`) ships **16 HTTP headers per request** through 8 validation gates for an invoke from a process running as the same OS user as Tauri. WP plugin is code we ship. The Tauri React renderer skips all 8 gates by going through in-process `tauri::invoke` as `Actor::User`. WP plugin belongs in the same column.
    26	
    27	**Layer 2 — Substrate readers (DOS-762).** `attach_live_workspace_readers` at `src-tauri/src/services/context.rs:61` registers 9 reader handles. The `ServiceContext` API exposes 14. The 4 missing readers (`with_list_open_loops_reader`, `with_account_list_reader`, `with_person_list_reader`, `with_project_list_reader`) are exactly what W1 producers compose against. Producer error propagates through `bridges/types.rs:1003` and `surface_runtime/mod.rs:3400` to wire code `auth_missing`. **6 distinct rejection paths collapse into `auth_missing`** with no `log::warn!`. The wire code lies about the cause.
    28	
    29	**Layer 3 — Third-party MCP transport-ceremony rip (new ticket).** v1.4.7 W1-A's substrate (PR #347, not yet merged to dev) contains ~645 LOC of transport-ceremony layered on top of ~1100 LOC of authorization machinery. For local stdio (today's primary MCP transport) and in-process hosting (DOS-647-C's goal), the transport-ceremony defends threats that don't exist. Rip the transport-ceremony layer; keep the authorization machinery verbatim. Specifics in §2c and §3e.
    30	
    31	**Why all three at once.** They're the same correction applied consistently. Splitting them means the reframe lands in one place and is contradicted by adjacent code in another — exactly the conflation that produced this mess. The 6-cycle L0 on v1.4.7 W1-A was internally coherent but premised on a wrong taxonomy; landing this packet IS the correction.
    32	
    33	**Explicitly preserved (load-bearing — see §8):** authorization machinery (per-client scope manifest, per-tool exposure tier, rate-limit, plain audit, conversation handle lifecycle), `Actor::McpClient` as a distinct actor class (third-party agents need scope-distinct attribution even if transport-ceremony goes), v1.4.2 W3-C WP-mediated MCP path, the pairing handshake itself, PR #355's `McpHandlerContext` interface (transport-agnostic), connector OAuth (Slack/Gmail external calls).
    34	
    35	---
    36	
    37	## §2 — Current state, file:line grounded
    38	
    39	### 2a — Gate stack (DOS-761)
    40	
    41	| # | Gate | File:Line | What it does | Defends against (same-OS-user threat model) |
    42	|---|------|-----------|--------------|---------------------------------------------|
    43	| 1 | `validate_transport_headers` | `src-tauri/src/surface_runtime/mod.rs:3914` | Host = `127.0.0.1:<port>`, origin check | Browser CSRF (loopback only; n/a for PHP curl) |
    44	| 2 | Loopback rate-limit (token bucket) | `surface_runtime/mod.rs:1015` | Per-IP request budget | DoS from local app (negligible) |
    45	| 3 | HMAC signature verify | `surface_runtime/mod.rs:1109` | Verifies `X-DailyOS-Signature` over canonical request | Tamper of in-flight loopback traffic (kernel-protected) |
    46	| 4 | `validate_signed_session_readonly` | `surface_runtime/mod.rs:1154` (impl `services/surface_pairing.rs`) | Session id ↔ wp_user_id ↔ site-nonce binding | Cross-tenant session replay (single-tenant local) |
    47	| 5 | `validate_session_bound_wp_user_id_for_request` | `surface_runtime/mod.rs:1969` | Body/query/header wp_user_id matches session-bound | Inconsistent identity injection (no second identity exists) |
    48	| 6 | `SurfaceClientBridge::authorize` (scope + cap) | `surface_runtime/mod.rs:2123` | Granted scope contains requested ability; allowed_actors includes SurfaceClient | Privilege escalation (we're already Actor::User in React) |
    49	| 7 | Per-axis rate-limit | embedded in #6 | Per-ability budget for the surface client | Runaway local consumer (cheap to local-budget) |
    50	| 8 | Audit-emit contract: `Actor::SurfaceClient` ⇒ `wp_user_id = Some(_)` | `src-tauri/src/audit_log.rs:104,141` | Refuses to emit audit if SurfaceClient actor lacks wp_user_id | Audit accountability (compromised by collapsing to Actor::User — see §3) |
    51	
    52	**Materialization.** `registry_actor = validated.actor.clone()` (`mod.rs:2152`) wraps as `Actor::SurfaceClient { surface_client_id }`. The React path constructs `Actor::User` directly via `ServiceContext::new_live().with_actor(...)` in `services/context.rs:139`.
    53	
    54	### 2b — Missing readers (DOS-762)
    55	
    56	`ServiceContext` `with_*` methods (`abilities-runtime/src/services/context.rs`):
    57	
    58	| Reader | Line | Currently attached? | Producer dependency |
    59	|--------|------|---------------------|---------------------|
    60	| `with_entity_context_reader` | 1779 | ✓ | `get_entity_context`, `get_entity_intelligence` |
    61	| `with_entity_context_claim_reader` | 1784 | ✓ | same |
    62	| `with_list_open_loops_reader` | 1792 | **✗** | `list_open_loops`, composed by `get_entity_intelligence` |
    63	| `with_prepare_meeting_context_reader` | 1797 | ✓ | `prepare_meeting` |
    64	| `with_daily_readiness_context_reader` | 1805 | ✗ (orphan) | unreferenced — confirm before removing |
    65	| `with_trajectory_reader` | 1813 | ✓ | temporal abilities |
    66	| `with_temporal_maintenance` | 1818 | ✓ | maintenance loop |
    67	| `with_composition_commit_handle` | 1826 | ✓ | composition commits |
    68	| `with_entity_touchpoints_reader` | 1834 | ✓ | touchpoints feed |
    69	| `with_meeting_prep_status_reader` | 1842 | ✓ | meeting prep status |
    70	| `with_claim_receipt_reader` | 1850 | ✓ | claim receipts |
    71	| `with_account_list_reader` | 1858 | **✗** | `list_accounts` (producer at `abilities-runtime/src/abilities/list_accounts/mod.rs:219`) |
    72	| `with_person_list_reader` | 1863 | **✗** | `list_people` |
    73	| `with_project_list_reader` | 1868 | **✗** | `list_projects` |
    74	
    75	### 2c — Error collapse + wire code (DOS-762 observability)
    76	
    77	- `bridges/types.rs:541` — `validate_input_json_against_schema(...).map_err(|_| BridgeSurfaceError::AbilityUnavailable)?` — schema rejection → AbilityUnavailable.
    78	- `bridges/types.rs:1003` — `AbilityInvokeError::Ability(_) => ... _ => BridgeSurfaceError::AbilityUnavailable` — any non-stale producer error → AbilityUnavailable.
    79	- `bridges/types.rs:540` — `reject_reserved_input_fields(...)?` — reserved field → BridgeSurfaceError (variant).
    80	- `surface_runtime/mod.rs:3400` — `bridge_surface_error: AbilityUnavailable | Ownership(_) => SurfaceHttpError::auth_missing()`.
    81	- `surface_runtime/mod.rs:3649` — `auth_missing()` returns HTTP 401 with code `"auth_missing"` and message *"The requested DailyOS surface ability is not available."*
    82	
    83	No `log::warn!` instrumentation on any of these collapse points. The wire code is the only signal.
    84	
    85	### 2c-bis — v1.4.7 W1-A MCP v2 surface (DOS-168) — separability by trust layer
    86	
    87	Branch `v1.4.7-w1-foundation` (PR #347, not merged to dev). Total ~3576 LOC across 7 files + 4 migrations. Mapped by the structural survey:
    88	
    89	| File | Total LOC | KEEP (authorization + ops) | DROP (transport-ceremony) | Separability |
    90	|------|-----------|-----------------------------|----------------------------|--------------|
    91	| `services/mcp_v2/auth.rs` | 566 | ~252 (pair_client, load_client_record, resolve_tool_grant, revoke_*, exposure helpers) | ~310 (verify_transport_hmac:232–244, verify_and_consume_and_preissue:257–315, transport key persist/load:399–474) | Partial — keychain custody is isolated; nonce consume/preissue is coupled to fail-closed contract; HMAC verify is called from gateway.rs:281 |
    92	| `services/mcp_v2/transport.rs` | 464 | ~370 (list_tools:157–188, build_input_schema, tool_from_description, unwrap_response, tool_error_to_mcp_error, tests) | ~70 (transport_key field:69, next_nonce field:71–72, call_tool nonce state machine:200–209+260, HMAC signing block:218–229) | Partial — nonce state machine interleaved with rmcp dispatch; envelope-shape stays, nonce population goes |
    93	| `services/mcp_v2/gateway.rs` | 883 | ~820 (handler registration, dispatch flow:303–528, scope subset checks, rate-limit reserve:715–786, audit/signal emission) | ~65 (Gate 0a HMAC verify:278–284, Gate 0b nonce consume+preissue:286–297, TIMING_FLOOR_MILLIS:33 + sleep_until_floor:795–801, preissued nonce threading across error paths:291–302+320–353+437–438) | Partial — nonce preissue threading touches ~20 lines across 5 error paths; non-local edit |
    94	| `services/mcp_v2/audit.rs` | 344 | ~144 (write API:50–78, sanitize_detail:111–124, add_actor_attribution:126–144, insert_outbox:181–220) | ~200 (hmac_json:146–150, canonical_value:152–168, audit key keychain custody:239–343, AUDIT_KEY_* constants:22–26) | Partial — keychain bits clean; hashing (lines 91–95) woven through `detail_with_hashes()` and all call sites; rip requires switching to plaintext emission OR field-level masking |
    95	| `services/mcp_v2/actor_policy.rs` | 79 | 79 | 0 | Clean — entirely authorization |
    96	| `services/mcp_v2/taxonomy.rs` | 714 | 714 | 0 | Clean — entirely tool registry |
    97	| `services/mcp_v2/contracts.rs` | 851 | ~800 (ToolDescription, Scope, ScopedName, ParamSpec, ReturnSpec, OpaqueConversationHandle, McpClientId, McpActor, envelope shapes) | ~50 (OpaqueNonce type:223–241 — keep type for safety, drop usage; request_nonce/next_request_nonce fields on envelopes) | Clean (type-level) |
    98	| `migrations/255_mcp_client_manifest.sql` | 19 | 19 | 0 | KEEP entirely (manifest table + transport_key_ref column kept as reference; key material is gone) |
    99	| `migrations/256_mcp_conversation_handle.sql` | 11 | 11 | 0 | KEEP entirely |
   100	| `migrations/257_mcp_transport_nonce_ledger.sql` | 17 | 0 | 17 | **DROP entirely** |
   101	| `migrations/258_mcp_rate_limit_and_audit_outbox.sql` | 18 | 18 | 0 | KEEP (rate-limit ledger + audit outbox tables both load-bearing for ops layer) |
   102	
   103	**Net rip estimate:** ~645 LOC removed, ~30 LOC modified (envelope shapes drop nonce fields), ~2900 LOC untouched. Migration 257 dropped before merge (replace with a no-op or renumber the rip-out into a new migration if 257 ships first).
   104	
   105	**`services/mcp_v2/handlers/*` — transport-agnostic.** Handlers receive `(actor: &McpActor, params: Value) → Result<Value, ToolError>`. PR #355's `McpHandlerContext` adds `(ctx: &McpHandlerContext, actor: &McpActor, params: Value)`. Neither signature touches transport-ceremony types. Handlers don't change.
   106	
   107	**PR #355 / DOS-758 status under this packet.** Unaffected. Lands independently. The `McpHandlerContext` interface + re-entrancy guard are correct under both the old and new trust models. The CI gate (`scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh`) stays.
   108	
   109	### 2d — WP transport client
   110	
   111	`wp/dailyos/includes/transport/class-dailyos-runtime-client.php:214` — `signed_post()` builds the 16-header POST. `DailyOS_Hmac_Signer::sign_request` (lines 243–253) computes the signature. Marker storage in `wp_options`. Session credential cached server-side.
   112	
   113	### 2e — Reuse audit (per `feedback_l0_reconcile_against_dev`)
   114	
   115	Fork SHA: `852a0118`. `git diff 852a0118..dev -- src-tauri/src/services/context.rs src-tauri/src/surface_runtime/ src-tauri/src/audit_log.rs src-tauri/src/bridges/types.rs wp/dailyos/includes/transport/` returns **no changes** — dev has not advanced on any surface touched by this packet since the fork. Safe to land against `dev` after PR #351 merges. If `dev` advances before this packet's PR opens, re-run reconcile.
   116	
   117	---
   118	
   119	## §3 — Proposed direction
   120	
   121	### 3a — Trust topology (DOS-761): first-party WP plugin as peer surface
   122	
   123	**Premise.** The DailyOS WP plugin is first-party code (we ship it, we control it, we sign its releases). When it runs in the user's WP-Now/Studio environment as the same OS user as Tauri, and connects via `127.0.0.1`, it is functionally identical to the React Tauri renderer — same code provenance, same process owner, same kernel-protected loopback. It belongs in the same trust column as `tauri::invoke`. The third-party agent column (Claude Desktop, Cursor) is where the v1.4.7 W1-A ceremony belongs and stays.
   124	
   125	**Mechanism.** Introduce a new HTTP route `POST /v1/local/invoke` on the existing runtime listener, bound to `127.0.0.1` only (already the bind). The handler:
   126	
   127	1. Verifies `peer_addr().is_loopback()` (defense in depth — the bind already enforces this).
   128	2. Materializes `Actor::User` directly (same as React `tauri::invoke`).
   129	3. Deserializes `{ ability, input, projection_verification? }` (the canonical `SurfaceInvokeRequest` shape — same as the signed path).
   130	4. Calls `TauriAbilityBridge::invoke` (the React path's entrypoint) with a new `BridgeSurface::LocalLoopback` variant (decision in §5.Q1; recommended new variant for audit clarity).
   131	5. Returns the canonical envelope JSON.
   132	6. Audit emission: emits with `Actor::User`, required `loopback_origin: "wp_plugin"` field (today's only consumer; enum extensible). No `wp_user_id` contract — there is no wp_user.
   133	
   134	**Kept.** Connector OAuth (Slack/Gmail/etc external calls). Audit log (every invoke emits). Confirmation attestation for destructive ops (the existing `confirmation_args_hash` + `verify_confirmation_token` pathway — unchanged; runs inside `TauriAbilityBridge::invoke`).
   135	
   136	**Removed for `/v1/local/invoke` path only.** HMAC verify, session validate, wp_user_id-per-request, SurfaceClient bridge authorize, per-axis rate-limit, audit-emit `wp_user_id` contract (because actor isn't SurfaceClient).
   137	
   138	**Explicitly preserved — DO NOT touch in this packet:**
   139	- `/v1/surface/invoke` (the existing signed-transport route) stays fully wired. v1.4.7's MCP v2 substrate (PR #347) uses it; DOS-647-C in-process MCP hosting will use it; if the WP-mediated MCP path (v1.4.2 W3-C) reaches into it, that stays too.
   140	- `services/mcp_v2/*` — gateway, auth, audit, actor_policy, taxonomy, transport, contracts. None of the v1.4.7 substrate moves in this packet.

 succeeded in 0ms:
   141	- `Actor::McpClient` and the `McpClientId`-bound scope manifest, nonce ledger, keyed-HMAC audit. Third-party agent ceremony is correct and stays.
   142	- The pairing handshake itself (`/v1/pair/...` routes). One-shot, infrequent, signed. Unchanged.
   143	
   144	**WP client.** `class-dailyos-runtime-client.php` gains a `local_post()` method targeting `/v1/local/invoke` with two headers: `Content-Type: application/json` + `X-DailyOS-Request-Id` (for traceability only, not auth). Pairing/marker stays for the connect-Tauri-to-WP UX flow (the WP plugin still needs to know which port Tauri is on — that's the existing sentinel/marker dance, unchanged). All `signed_post()` invoke call sites route to `local_post()`. The pairing-handshake `signed_post()` calls stay (rare, one-shot at pair time, server-issues pairing credentials over a signed channel — separate from per-invoke ceremony).
   145	
   146	**Cross-track guarantee.** This packet's diff touches zero files under `src-tauri/src/services/mcp_v2/` or `src-tauri/src/mcp_v2/`. CI gate `scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh` (introduced by DOS-758 / PR #355) is unaffected. v1.4.7 W1-A's wire-shape parity tests are unaffected.
   147	
   148	### 3b — Substrate readers (DOS-762): wire the 4 missing
   149	
   150	In `src-tauri/src/services/context.rs::attach_live_workspace_readers`, add:
   151	
   152	- `with_list_open_loops_reader(Arc::new(LiveListOpenLoopsReader))`
   153	- `with_account_list_reader(Arc::new(LiveAccountListReader))`
   154	- `with_person_list_reader(Arc::new(LivePersonListReader))`
   155	- `with_project_list_reader(Arc::new(LiveProjectListReader))`
   156	
   157	Implement the 4 new `Live*Reader` structs as siblings to `LiveEntityContextReader` (same pattern: `Arc::new(LocalKeychain)` → `ActionDb::open` → call into existing `services::accounts::list`, `services::people::list`, etc — those service functions exist; the reader is just an adapter).
   158	
   159	Verify `with_daily_readiness_context_reader` truly has no live consumer. If unused, leave un-attached and add a `// orphan: no producer composes against this` comment at the trait definition. If grep finds a consumer, attach it too.
   160	
   161	### 3c — Observability: stop the `auth_missing` lie
   162	
   163	In `bridges/types.rs`:
   164	
   165	- `:541` — replace `.map_err(|_| BridgeSurfaceError::AbilityUnavailable)` with `.map_err(|err| { log::warn!(target: "abilities::dispatch", "schema validation failed for {ability_name}: {err}"); BridgeSurfaceError::InputSchemaInvalid })`. Add the new `BridgeSurfaceError::InputSchemaInvalid` variant.
   166	- `:1003` — split the `AbilityInvokeError::Ability` arm: on `AbilityErrorKind::*` that indicate missing context (TBD enum check — likely a new variant `ProducerContextUnavailable`), map to `BridgeSurfaceError::ProducerUnavailable` and `log::warn!`. Keep `AbilityUnavailable` for genuine "ability not registered."
   167	- `:540` — `reject_reserved_input_fields` adds `log::warn!` on rejection.
   168	
   169	In `surface_runtime/mod.rs`:
   170	
   171	- `:3400` — split `bridge_surface_error`: `AbilityUnavailable` → wire code `ability_not_registered` (HTTP 404), `InputSchemaInvalid` → wire code `input_schema_invalid` (HTTP 422), `ProducerUnavailable` → wire code `producer_unavailable` (HTTP 503), `Ownership(_)` → wire code `ownership_denied` (HTTP 403). Reserve `auth_missing` strictly for actual auth failures on the signed path.
   172	- New wire codes added to the WP-side error renderer in `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php` so chips can render the producer reason (`stale`, `not_processed_yet`, `producer_unavailable`) vs an auth wall.
   173	
   174	### 3d — Order of work (wave plan)
   175	
   176	Three coordinated PRs against `dev`, sequenced to keep CI green at every step:
   177	
   178	**PR A — Substrate readers + observability (DOS-762).** Lowest risk, immediate chip-rendering win.
   179	1. Add 4 live readers + wire in `attach_live_workspace_readers`.
   180	2. Split error wire codes + add `log::warn!` instrumentation.
   181	3. L4: chips render producer reasons honestly even on signed path.
   182	
   183	**PR B — First-party WP loopback (DOS-761).** Adds new route, cuts WP over.
   184	4. Add `/v1/local/invoke` route + `Actor::User` materialization for loopback + new `BridgeSurface::LocalLoopback` variant.
   185	5. WP `local_post()` method + cut over all invoke call sites in `class-dailyos-runtime-client.php`.
   186	6. L4: bring-a-trailer chips render real envelopes; Tauri log shows zero `auth_missing` from WP path.
   187	
   188	**PR C — MCP v2 substrate at the new shape (DOS-168 amended).** Coordinated with v1.4.7 W1-A.
   189	7. Hold PR #347 merge OR rebase PR #347 over PR C.
   190	8. Apply the §3e rip: delete migration 257, delete ~310 LOC from auth.rs, ~70 LOC from transport.rs, ~65 LOC from gateway.rs, ~200 LOC from audit.rs. Modify ~30 LOC across envelope shapes.
   191	9. PR #355 (DOS-758 handler context) lands independently and unaffected.
   192	
   193	**Total estimate:** PR A = 3–4h. PR B = 4–6h. PR C = 8–12h (the rip is mostly mechanical but L0 already passed on the wrong premise, so re-review is structural). Plus L2 + L4 on each.
   194	
   195	### 3e — Third-party MCP transport-ceremony rip (new ticket)
   196	
   197	**Premise.** v1.4.7 W1-A's transport-ceremony layer defends remote-network threats against a local-only transport. Local stdio (today's MCP transport) has no in-flight surface. In-process hosting (DOS-647-C) makes "transport" a Rust function call. The layer is dead weight.
   198	
   199	**What gets ripped (with file:line precision from §2c-bis):**
   200	
   201	**Migration drops:**
   202	- `src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql` — entire schema gone. If 257 ships in dev before PR C lands, replace with a `DROP TABLE IF EXISTS mcp_transport_nonce_ledger;` rip-out migration at the next slot.
   203	
   204	**Code drops (~645 LOC):**
   205	- `services/mcp_v2/auth.rs:232–244` — `verify_transport_hmac()`.
   206	- `services/mcp_v2/auth.rs:257–315` — `verify_and_consume_and_preissue()` fail-closed nonce ledger.
   207	- `services/mcp_v2/auth.rs:399–474` — transport key Keychain custody (`persist_transport_key`, `load_transport_key`, Zeroize wrappers).
   208	- `services/mcp_v2/auth.rs:32–51` — error variants `InvalidSignature`, `NonceReplayed`, `PreissueFailed`, `Keychain`.
   209	- `services/mcp_v2/transport.rs:69` — `transport_key: Arc<Zeroizing<[u8; 32]>>` field.
   210	- `services/mcp_v2/transport.rs:71–72` — `next_nonce: Arc<Mutex<Option<OpaqueNonce>>>` field.
   211	- `services/mcp_v2/transport.rs:200–209,260` — nonce pull-from-state and store-next-nonce.
   212	- `services/mcp_v2/transport.rs:218–229` — HMAC envelope signing block.
   213	- `services/mcp_v2/gateway.rs:33` — `TIMING_FLOOR_MILLIS` const.
   214	- `services/mcp_v2/gateway.rs:278–284` — Gate 0a HMAC verify call.
   215	- `services/mcp_v2/gateway.rs:286–297` — Gate 0b nonce consume+preissue call.
   216	- `services/mcp_v2/gateway.rs:795–801` — `sleep_until_floor()` timing floor.
   217	- `services/mcp_v2/audit.rs:22–26` — `AUDIT_KEY_*` constants.
   218	- `services/mcp_v2/audit.rs:146–150` — `hmac_json()`.
   219	- `services/mcp_v2/audit.rs:152–168` — `canonical_value()` (canonical JSON for HMAC).
   220	- `services/mcp_v2/audit.rs:239–343` — audit key Keychain custody (`load_or_create_audit_key`, `read_audit_key`, `persist_audit_key`, etc).
   221	
   222	**Code modifications (~30 LOC):**
   223	- `services/mcp_v2/contracts.rs:223–241` — keep `OpaqueNonce` type, remove all usage. (Or delete the type and accept the type-safety loss; recommend keep-the-type in case it earns its way back.)
   224	- `services/mcp_v2/contracts.rs` — `McpToolRequestEnvelope.request_nonce` field becomes `Option<OpaqueNonce>` or removed. `McpToolResponseEnvelope.next_request_nonce` similarly.
   225	- `services/mcp_v2/audit.rs:80–100` — `detail_with_hashes()` becomes `detail_with_attribution()`. Emit params/response **plaintext** in the detail (no HMAC hashing) OR switch to field-level masking via the existing `sanitize_detail()` (lines 111–124) — decision in §5.Q9.
   226	- `services/mcp_v2/gateway.rs:291–302,320–353,437–438` — unthread `preissued` from rejection-response building. Response envelope no longer carries `next_request_nonce`.
   227	- `services/mcp_v2/migrations/255_mcp_client_manifest.sql` — keep table; `transport_key_ref` column either nulls out (kept for schema stability) or drops in a sibling migration.
   228	
   229	**Kept (load-bearing authorization + ops):**
   230	- Entire authorization machinery: pair_client, manifest load, scope resolution, exposure-tier check, conversation handle lifecycle, scope-subset enforcement, rate-limit reserve, plain audit emission, audit outbox, `actor_policy.rs`, `taxonomy.rs`, `contracts.rs` type ecosystem (minus nonce/HMAC bits).
   231	- `Actor::McpClient` as a distinct actor class — third-party agents need scope-distinct attribution from `Actor::User`, even if they don't sign envelopes.
   232	- DOS-624 sensitivity gating amendment (briefing exposure) — independent of transport, composes downstream.
   233	- PR #355's `McpHandlerContext` interface + CI gate.
   234	
   235	**New invariant.** Audit log row format becomes: `{ actor: McpClient(client_id), conversation_handle, ability, params: <plaintext or masked>, response: <plaintext or masked>, request_id, timestamp }`. No HMAC hashes. Tamper-evidence is delegated to the OS — the SQLCipher DB the user controls is the audit substrate.
   236	
   237	**What CSO must re-pass on.** §6.5 reviewer prompt is rewritten to bound the CSO question precisely: "the personal-tier trust model puts the user in control of their own DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with SQLCipher + Keychain DB encryption, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't?"
   238	
   239	---
   240	
   241	## §4 — Acceptance criteria
   242	
   243	### DOS-762 (readers + observability)
   244	- [ ] `attach_live_workspace_readers` attaches `list_open_loops_reader`, `account_list_reader`, `person_list_reader`, `project_list_reader` with live SQLite adapters.
   245	- [ ] `cargo run --bin emit_ability_inventory` regenerates clean; CI ability-inventory gate green.
   246	- [ ] Direct curl to `list_accounts` returns populated envelope with non-empty `accounts` array.
   247	- [ ] Direct curl to `get_entity_intelligence` for a real entity returns populated envelope with non-empty `sections`.
   248	- [ ] `log::warn!` fires on schema rejection, reserved-field rejection, producer context unavailable — with ability name + reason. Verified by `RUST_LOG=warn` tail during a forced-fail invoke.
   249	- [ ] Wire codes split: HTTP responses for the 4 distinct rejection classes return the new codes (`ability_not_registered`, `input_schema_invalid`, `producer_unavailable`, `ownership_denied`). `auth_missing` returns ONLY on genuine signed-path auth failures.
   250	
   251	### DOS-761 (trust topology)
   252	- [ ] `POST /v1/local/invoke` route registered; binds to loopback only; rejects non-loopback peers with HTTP 404 (404, not 401 — pretends not to exist for non-loopback callers).
   253	- [ ] Handler materializes `Actor::User` and calls `TauriAbilityBridge::invoke` with the canonical path. No HMAC, session, scope, or wp_user_id validation in the loopback handler.
   254	- [ ] Audit log emits an entry per loopback invoke with `Actor::User` and optional `loopback_origin` for traceability. No SurfaceClient/wp_user_id contract.
   255	- [ ] WP `class-dailyos-runtime-client.php` adds `local_post()` and routes all ability-invoke call sites through it. `signed_post()` retained for pairing handshake only.
   256	- [ ] WP marker/sentinel flow unchanged (Tauri port discovery still works).
   257	- [ ] L4: bring-a-trailer hard-refresh → 24 chapters render real envelope data; `not_available` and `no_envelope` chips replaced by either real content or honest producer reasons (`stale`, `not_processed_yet`, `no_relevant_touchpoints`).
   258	- [ ] Tauri log tail shows zero `auth_missing` on the loopback path; if a producer fails, the wire code names the actual cause.
   259	
   260	### DOS-168 amended (MCP v2 substrate at the new shape — see §3e)
   261	- [ ] Migration 257 dropped (or rip-out migration filed at next slot if 257 ships first).
   262	- [ ] `services/mcp_v2/auth.rs` shrinks by ~310 LOC: HMAC verify, nonce ledger, transport key custody gone. Manifest load + scope resolution + handle lifecycle intact.
   263	- [ ] `services/mcp_v2/transport.rs` shrinks by ~70 LOC: nonce state machine + HMAC signing gone. `list_tools`, `tool_error_to_mcp_error`, dispatch flow intact.
   264	- [ ] `services/mcp_v2/gateway.rs` shrinks by ~65 LOC: Gate 0a, Gate 0b, timing floor, preissue threading gone. Dispatch + scope subset + rate-limit + audit emission intact.
   265	- [ ] `services/mcp_v2/audit.rs` shrinks by ~200 LOC: HMAC hashing + audit key custody gone. Emission + outbox + actor attribution intact. Detail emits plaintext-in-encrypted-DB OR field-masked (per §5.Q9 decision).
   266	- [ ] `Actor::McpClient` retained; scope manifest enforcement intact end-to-end.
   267	- [ ] Local MCP integration smoke test: a stdio MCP client (real or mock) connects, sees `tools/list` filtered by manifest grants, invokes a read tool, receives result. No signing, no nonces.
   268	- [ ] `services/mcp_v2/handlers/*` unchanged; PR #355 lands cleanly.
   269	- [ ] `cargo test --workspace --all-features --lib services::mcp_v2` green; `cargo test --test dos168_mcp_v2_migration_smoke_test` updated for the migration-257 drop.
   270	- [ ] DOS-624 sensitivity gating still composes correctly downstream of `Actor::McpClient` materialization.
   271	
   272	### Joint
   273	- [ ] `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green across all three PRs.
   274	- [ ] L2 unanimous APPROVE on each PR (codex challenge + codex consult + architect-reviewer + wp-skill-grounded reviewer + CSO + v1.4.7-cross-track reviewer).
   275	- [ ] PR descriptions name relevant tickets; commit messages carry `L2-status: passed`.
   276	- [ ] PR #347 merge held until PR C reshapes the substrate; or PR #347 author rebases over PR C; coordination decision in §5.Q8.
   277	
   278	---
   279	
   280	## §5 — Open decisions for L0 (the questions that shape the PR)

exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md | sed -n '281,430p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
   281	
   282	**Q1.** New `BridgeSurface::LocalLoopback` variant, or reuse `BridgeSurface::TauriApp`?
   283	- *Pro new:* keeps audit/traceability distinct so we can tell "React in-process invoke" from "WP-plugin first-party-loopback invoke" without parsing the actor.
   284	- *Pro reuse:* fewer surface enum cases; the trust model is genuinely identical between the two.
   285	- **Recommend:** new variant. Audit clarity is cheap; the WP plugin is a distinct first-party surface even if it shares the trust model with React.
   286	
   287	**Q2.** Touch the signed-transport invoke route at all?
   288	- **Hard no.** v1.4.7 W1-A (PR #347) uses it. DOS-758 / PR #355 builds on it. DOS-647-C will land more handlers into it. WP-mediated MCP (v1.4.2 W3-C) may reach into it. Stripping or deprecating it now would burn 6 cycles of L0 review and block all three open MCP tracks.
   289	- **Recommend:** `/v1/surface/invoke` is fully load-bearing for third-party agents and stays. This packet adds `/v1/local/invoke` as a new sibling route; it does not modify, deprecate, or annotate the existing route.
   290	
   291	**Q3.** Wire the 4 new readers all at once, or batched? (DOS-762 split into sub-tickets?)
   292	- All 4 live SQLite adapters are the same shape (10-15 lines each). Splitting buys nothing.
   293	- **Recommend:** all 4 in one commit, one set of tests.
   294	
   295	**Q4.** `daily_readiness_context_reader` — attach or leave orphan?
   296	- Need a grep audit to confirm no producer composes against it.
   297	- **Recommend:** the L1 author audits in commit 1 and either attaches (with live adapter) or files a Linear comment confirming orphan + adds explanatory comment at trait definition.
   298	
   299	**Q5.** Per-axis rate-limit replacement for loopback?
   300	- The signed-path's per-ability budget defends against runaway third-party agents.
   301	- First-party WP plugin as Actor::User has no such defense. Worth it?
   302	- *Position:* no rate-limit. If WP plugin runs away, that's a WP plugin bug to fix at the source, not paper over with a rate-limit on our own code. Parity with React `tauri::invoke` (also has no per-axis limit) is the point.
   303	- **Decided now:** no rate-limit on `/v1/local/invoke`. To keep runaway loops observable (not deferred), §3e's per-invoke audit emission + the §3c `log::warn!` instrumentation give a clear signal: if the same `(actor, ability)` pair fires N+ times in a window, the audit log shows it immediately. If a viewer surface is needed to surface that signal, it's a viewer ticket, not a rate-limit ticket.
   304	
   305	**Q6.** Should `Actor::User` materialization from `/v1/local/invoke` distinguish WP-plugin origin from a (future) in-process MCP host?
   306	- DOS-647-C is in-process MCP hosting — moving handlers from the standalone binary into the signed-route server. It still mints `Actor::McpClient` for the requesting agent, not `Actor::User` — the in-process move is a deployment change, not a trust-model change.
   307	- So `/v1/local/invoke` is exclusively for first-party surfaces (today: WP plugin; tomorrow: anything else we ship that's not a renderer of `tauri::invoke`).
   308	- **Recommend:** `loopback_origin` enum is restrictive — only first-party surface kinds. Add a CI lint that rejects new `loopback_origin` variants without a CSO sign-off comment in the PR body. Today's enum: `{ wp_plugin }`. MCP hosts do NOT use this route.
   309	
   310	**Q7.** Coordination with DOS-624 (v1.4.7 W2-A CSO amendment for `dailyos.read.daily_briefing` MCP exposure)?
   311	- DOS-624 reasons about which intelligence the third-party MCP path can expose. It does not constrain the first-party WP path because the WP path already renders the intelligence directly (it's the WP plugin's own UI).
   312	- *Risk:* if DOS-624 amends the briefing intelligence sensitivity gate AFTER this packet lands, the first-party WP path could continue rendering briefings that have been restricted from third-party MCP. That's the intended behavior — first-party gets the un-redacted view; third-party gets the gated view. Worth naming as an architectural invariant.
   313	- **Recommend:** add an invariant note in §3a: *"sensitivity gating composes downstream of actor materialization. First-party `Actor::User` and third-party `Actor::McpClient` may see different projections of the same intelligence by design."*
   314	
   315	**Q8.** PR #347 coordination — what happens to the existing open PR?
   316	- *Hold + rebase:* PR #347 author cherry-picks the KEEP set (~2900 LOC of authorization machinery), rebases over PR C's drops. Preserves PR review history; 6 cycles of L0 reviewer context stays attached. But: the 6 cycles validated the wrong premise; preserving them carries assumption-baggage forward.
   317	- *Close + supersede:* close PR #347 as superseded by PR C. New L2 cycle on the simplified substrate. Loses review history but the history was internally-coherent-on-wrong-premise.
   318	- **Decided now:** **close + supersede**. PR C lands the substrate at the new shape directly (authorization machinery + dispatch + handlers + migrations 255/256/258 + taxonomy + actor_policy + contracts) — it IS the substrate, not an amendment to it. PR #347 closes with a comment pointing at PR C. The v1.4.7 wave plan amendment lands as part of PR C's diff (`.docs/plans/v1.4.7-waves.md` updated in same PR). Nothing follows-up; nothing is parked. DOS-168 description + AC update at PR-open time.
   319	
   320	**Q9.** Audit detail emission — plaintext or field-masked?
   321	- `sanitize_detail()` already exists at audit.rs:111–124. The mechanism is built; the question is whether to use it.
   322	- *Read-class abilities:* params are entity IDs, depths, query strings. Plaintext-in-encrypted-DB. The user is the only auditor; SQLCipher + Keychain gates the file.
   323	- *Write-class / submit-class abilities:* params may carry user-typed content (note text, claim corrections, document body). Mask via `sanitize_detail()` to drop named payload fields. Preserves audit attribution without re-rendering user content into a separate log substrate.
   324	- **Decided now:** PR C ships both per ability `Side` (`Read` / `Write` / `SubmitCorrection` from contracts.rs:18–23). Read = plaintext detail. Write/SubmitCorrection = `sanitize_detail()`-masked. Wired in `services/mcp_v2/audit.rs::detail_with_attribution` based on the ability's declared Side. CSO confirms in §6.5 that the existing masking set in `PARAM_PAYLOAD_KEYS` / `RESPONSE_PAYLOAD_KEYS` is the right list for write-class.
   325	
   326	**Q10.** Where does PR C live as a ticket?
   327	- **Decided now:** amend DOS-168 directly. DOS-168 IS the v1.4.7 W1-A MCP v2 gateway substrate ticket; it has not merged. Updating its description + acceptance criteria to reflect the simplified substrate (drop transport-ceremony, keep authorization) is the honest move. PR C closes DOS-168 at the new shape. No new umbrella. DOS-168 description + AC update lands at PR C open time. PR #347 closes superseded-by-PR-C per §5.Q8.
   328	- Per `feedback_no_deferrals_period` and `feedback_path_alpha_as_sneaky_deferral`: this is not a follow-up, not an umbrella, not a maintenance bucket. PR C is the work and DOS-168 is the spec being amended to match it.
   329	
   330	---
   331	
   332	## §6 — Reviewer prompts
   333	
   334	Per the wave reviewer matrix + memory (`feedback_wp_skill_grounded_reviewer_for_wp_l0`) + CSO required (trust-boundary work, Amendment 3):
   335	
   336	1. **codex challenge** — adversarial: "find a same-OS-user threat that the gate stack defends and the loopback model doesn't. If you find one, propose the cheapest non-gate mitigation (process-level, audit-level, OS-level). Don't argue for keeping the gates abstractly."
   337	2. **codex consult** — design review: "is `/v1/local/invoke` the right shape, or should loopback dispatch happen via an in-process channel (mpsc/dashmap) with no HTTP at all? Trade-off the two for MCP-readiness."
   338	3. **architect-reviewer** — integrated state: "does this packet hold against W3-A.5/A.6 work scheduled later in v1.4.4? Any wiring this packet locks in that the workspace memory or salience waves would need to re-open?"
   339	4. **plan-design-review** — surface integrity: "chip rendering currently fails opaque; this packet ships honest producer-reason chips. Does the empty-chip CSS pattern + Magazine shell language degrade gracefully for `producer_unavailable` / `stale` / `not_processed_yet`?"
   340	5. **CSO** — trust-boundary scrutiny on the full packet: "(a) for first-party WP loopback: any third process running as the same OS user that could intercept the loopback port, confused-deputy risk if Tauri's own renderer is compromised, audit-log integrity if `loopback_origin` is client-supplied; (b) **for MCP transport-ceremony rip:** the personal-tier trust model puts the user in control of their own SQLCipher+Keychain DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with DB encryption + OS-user file ownership, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't? Same for nonce ledger — what local-stdio replay attack is the nonce defending? (c) confused-deputy / scope-elevation: if a third-party MCP client (Claude Desktop) is compromised by a hostile MCP server, does the scope manifest still hold without HMAC? (d) PR #347's 6-cycle L0 sign-off on the keyed-HMAC/nonce/timing-floor model — was it premise-validated or only internally-validated? If premise-validated, surface the threat model that changed."
   341	6. **wp-skill-grounded reviewer** (`/codex consult` with skill context) — "WP plugin transport: does the `signed_post` → `local_post` cutover correctly preserve the pairing handshake path? Any WP-CLI / activation hook / multisite consideration that breaks if the plugin's invoke stops carrying identity headers?"
   342	7. **v1.4.7-cross-track reviewer** (`/codex consult` pointed at `.docs/plans/v1.4.7-waves.md` + PR #347 + PR #355 + DOS-647-C ticket) — "this packet **reshapes** the v1.4.7 W1-A substrate, not just preserves it. Audit: (a) the §3e KEEP/DROP split in §2c-bis — anything mis-categorized? Specifically the conversation handle lifecycle (auth.rs:321–356), is it really transport-independent or does it depend on nonce-binding? (b) DOS-647-C's in-process MCP hosting plan — does dropping HMAC/nonce simplify or complicate the in-process move? (c) the §3d hybrid coordination (PR C ships substrate at new shape; PR #347 becomes docs-only amendment) — is this the right cycle hygiene or should PR #347 be closed-and-rebased? (d) PR #355 `McpHandlerContext` lands independently — confirm no transport-ceremony dependency. (e) DOS-624 sensitivity gating composes downstream of actor materialization — confirm intact under rip."
   343	
   344	**K-in obligation per CLAUDE.md.** Before scoring, every reviewer greps `docs/solutions/` and `.docs/decisions/` for prior trust-topology or reader-wiring decisions. ADR-0083 (product vocab) and any ADR in the 01xx range touching surface auth must be read.
   345	
   346	---
   347	
   348	## §7 — Memories load-first
   349	
   350	- `feedback_local_to_local_security_overreach_primary_concern` — primary concern; flagged here in §1.
   351	- `feedback_l4_debugging_can_hide_overengineering` — 4h of `auth_missing` debugging was the trigger.
   352	- `project_auth_overhaul_strip_hmac_for_local_same_user` — current state of this work; this packet IS the overhaul, narrowly scoped to first-party WP plugin loopback.
   353	- `feedback_wire_existing_substrate_not_future_producer` — wiring the readers IS the work; v1.4.3 had this working.
   354	- `feedback_dont_swing_past_center_when_correcting` — preserve v1.4.7 MCP v2 substrate verbatim; only reclassify first-party WP plugin.
   355	- `feedback_ci_gate_inputs_are_L1_deliverables_not_verification_steps` — observability instrumentation ships in this PR, not in a follow-up.
   356	- `feedback_l0_reconcile_against_dev` — reconciled at §2e.
   357	- `feedback_wp_skill_grounded_reviewer_for_wp_l0` — wp-skill panel added at §6.6.
   358	- `project_v142_wordpress_spike` — WordPress as primary surface; first-party trust column is the architectural commitment.
   359	
   360	---
   361	
   362	## §8 — What this packet does NOT do (hard scope guard)
   363	
   364	**Authorization machinery — diff against KEEP set must be zero except for envelope nonce-field removal:**
   365	- `services/mcp_v2/actor_policy.rs` — KEEP entirely.
   366	- `services/mcp_v2/taxonomy.rs` — KEEP entirely.
   367	- `services/mcp_v2/handlers/*` — KEEP entirely (transport-agnostic).
   368	- `services/mcp_v2/auth.rs` KEEP set (~252 LOC of pairing + manifest + grant resolution + revocation + exposure helpers).
   369	- `services/mcp_v2/gateway.rs` KEEP set (~820 LOC of dispatch + scope subset + rate-limit + audit emission + signal emission).
   370	- `services/mcp_v2/transport.rs` KEEP set (`list_tools`, dispatch, error mapping, tests).
   371	- `services/mcp_v2/audit.rs` KEEP set (write API, sanitize_detail, attribution, outbox).
   372	- `services/mcp_v2/contracts.rs` — KEEP all types except remove nonce fields from envelopes.
   373	- Migrations 255, 256, 258 — KEEP entirely.
   374	- `Actor::McpClient`, `McpClientId`, `OpaqueConversationHandle`, `ToolGrant`, `ClientRecord` — KEEP all types.
   375	- `/v1/surface/invoke` signed-transport route handler — KEEP (still serves the pairing handshake at minimum; long-term future-remote consumer; coordinate with §5.Q2).
   376	- `/v1/pair/*` pairing handshake routes — KEEP entirely.
   377	- `scripts/ci/check_mcp_v2_handlers_no_direct_db_open.sh` and hookups — KEEP entirely (PR #355's gate is correct under both models).
   378	- v1.4.2 W3-C WP-mediated MCP path — KEEP entirely.
   379	- Connector OAuth (Slack/Gmail external HTTP) — KEEP entirely.
   380	- DOS-624 CSO sensitivity gating — KEEP entirely (composes downstream of actor materialization).
   381	
   382	**Explicitly does NOT do:**
   383	- Does NOT implement DOS-647-C in-process MCP hosting (the rip simplifies the path; the move itself stays in v1.4.7).
   384	- Does NOT change handler signatures beyond what PR #355 already does.
   385	- Does NOT touch ability registry or producer code (orthogonal).
   386	- Substrate-stability / writer-mutex / Glean-validation issues from the post-merge handoff §5 are real and orthogonal — they're DB-layer issues, this packet is auth-layer. They need their own L0 packet (corruption recovery cadence, writer-mutex fairness queueing, Glean validation moving outside the writer transaction). If no Linear ticket exists by PR A merge, file one and link from PR A description — don't let "separate" become "deferred."
   387	- Inner-block envelope-shape mapping (Open Thread #2 in handoff) is one L4 verification cycle, not a separate workstream — runs as part of PR A's L4 since the readers + observability are what surface the shape-gap symptoms. If shape gaps exist, fix in PR A.
   388	- Remote-MCP transport: not built. If a remote consumer arrives later, gate-stack-revival is the right move at that point. "Don't pre-build for a threat that hasn't materialized" is different from "defer work that needs doing" — there is no caller, so there is no work.
   389	
   390	**Coordination commitment.**
   391	- PR A (DOS-762 readers + observability) can ship first independently.
   392	- PR B (DOS-761 WP loopback) ships independently of PR C.
   393	- PR C (DOS-NNN MCP rip) coordinates with v1.4.7 W1-A per §5.Q8 hybrid: PR C ships substrate at the new shape, PR #347 becomes a docs-only amendment to `.docs/plans/v1.4.7-waves.md`.
   394	- DOS-758 (PR #355) is **unaffected** and can land at any time; the `McpHandlerContext` interface is correct under both models.
   395	- DOS-624 CSO sensitivity gating amendment is **unaffected** and can land at any time; it composes downstream of actor materialization.

exec
/bin/zsh -lc 'rg -n "347|PR #347|cycle|HMAC|nonce|loopback_origin|Actor::|wp_user|audit|supersede|superseded|close" .docs/plans/v1.4.4-wp-surface-migration -S' in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:6:**Predecessor:** `.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md` (pre-merge handoff; superseded by this doc)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:15:DOS-335 meeting_prep_status read/write split · DOS-459 EntityIntelligenceEnvelope · DOS-339 claim_receipt fan-out · DOS-477 entity trust-boundary · DOS-340 audit boundary · DOS-341 privacy/redaction · DOS-460 touchpoints + open-loops · DOS-8 semantic feedback (10 variants) · DOS-461 fixture harness · DOS-507 get_daily_briefing composition.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:78:### 1. ROOT CAUSE FOUND: surface invoke fails on audit-emit contract (`wp_user_id` is None)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:80:**Status as of 2026-05-21T17:17 (after PR #346 merge + fresh repair):** all 24 inner blocks still render empty (19 `not_available` + 4 `no_envelope`). The outer block has `data-dailyos-envelope-handle="a0d3baf87fa224e78f109fb3f448146b71806a569d5bea8a98c07eddd381f71b"` — that's the **deterministic-hash of an empty/error response** per `wp/dailyos/blocks/_shared/envelope/envelope-resolver.php:122`. So the runtime IS being reached, signing IS verified, scope check IS passing (after the Phase A trim + DOS-746 scope-refresh), but the invoke fails at the **audit-emit gate**.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:84:WARN dailyos_lib::surface_runtime] surface pairing audit write failed:
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:85:emit_surface_audit contract violation:
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:86:Actor::SurfaceClient requires AuditFields.wp_user_id to be Some(_)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:89:**What this means:** when a `SurfaceClient` actor invokes an ability, the audit-event emitter validates that `AuditFields.wp_user_id` is `Some(_)` (the contract added during DOS-559 W2-C `preserve SurfaceClient attribution on post-HMAC validation rejections`). The WP plugin's signed POST IS sending `X-DailyOS-WP-User-Id` (see `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:244`), so either:
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:92:- something is stripping wp_user_id from the audit payload between request validation and the invoke audit emission
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:95:- `src-tauri/src/surface_runtime/mod.rs` — search for `wp_user_id` + `AuditFields` + `emit_surface_audit`
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:96:- `src-tauri/src/services/surface_pairing.rs` — `AuditFields` struct + the audit-emit pathway
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:97:- Look at recent commits touching audit-emit: `git log --oneline -- src-tauri/src/surface_runtime/mod.rs | head -10`
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:99:**Why empty chips:** the failed audit-emit causes the invoke to return an error response. The outer block catches `is_wp_error || !is_array` → returns empty chip. The inner blocks read the (cached, empty) envelope handle → render their own empty chips. Both behaviors are correct; the substrate fault is at the audit-emit contract.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:102:Once the audit-emit gate passes and real envelopes arrive, the 19 `not_available` chapters may STILL fail if their resolver expects different keys than the runtime ships. That's the secondary investigation. Don't start it until #1 is fixed — empty envelopes will mask shape mismatches.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:120:- **3 corruption-recovery cycles in 2 weeks** — `~/.dailyos/dailyos.db.corrupt-20260505`, `dailyos.db.corrupt-20260516`, `dailyos.db.corrupt-20260518`, `dailyos.db.corrupt-20260519`. SHM + WAL + DB snapshots preserved. Current DB passes `PRAGMA integrity_check` but the cadence (every ~7 days) suggests a write-path bug, not random hardware/power issues.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:148:3. **Tail the Tauri log:** when the page renders, watch for `emit_surface_audit contract violation: Actor::SurfaceClient requires AuditFields.wp_user_id`. That's the root-cause from Open Thread #1.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21-post-merge.md:149:4. **Fix the audit-emit contract** at `src-tauri/src/surface_runtime/mod.rs` or `services/surface_pairing.rs` — make `wp_user_id` extraction wrap properly into `Some(_)` from the signed POST headers.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:3:**Current revision: V1.1 (cycle-1 reviewer folds, 2026-05-20).**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:9:Scope: full v1.4.4 wave program — W0 (surface audit) → W6 (parity proof + Tauri shell deprecation). This is the **master wave-level L0**. Per-wave sub-L0 packets (W1 substrate gaps, W2 entity surfaces, W3 briefing surfaces, W4 action surfaces, W5 system/history) author after W0 audit lands and inherit invariants from this packet.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:16:- **v1.4.3 close** — strict prerequisite per reorientation doc §C2 ("v1.4.5+ versions are strict-gated on v1.4.4 closing"); v1.4.4 itself gates on v1.4.3 W6 clean-machine validation closing.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:18:Surfaces in scope: ~18 active Tauri user-facing surfaces, to be enumerated by DOS-677 (W0 surface audit) from `src/router.tsx` + `.docs/design/INVENTORY.md` + nav reachability. Each migrates to a WordPress Gutenberg composite block consuming real substrate (no façades over old data model).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:30:These shipped via the v1.4.4 W3 chrome lane pulled-forward parallel to v1.4.3 W4+. See `L0-packet-W3-chrome-lane-pulled-forward.md` for that lane's L0 close. The wave-level L0 here **acknowledges** chrome shipped and treats it as substrate that W3 briefing surfaces and W2 entity surfaces consume, rather than re-litigating.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:36:- **V1.1 (2026-05-20):** cycle-1 reviewer folds across 5 verdicts. Architecture (8 findings): F1 HIGH outer/inner ADR-0130 mapping (Reading A locked); F2 HIGH cursor envelope contract codified; F3 MEDIUM W0 5th list (dev/admin surfaces); F4 MEDIUM mid-wave gap escalation in AC #W5; F5 MEDIUM refresh-model invariant + uniform Intelligence Loop §3 answer; F6 MEDIUM forward-coupling hot spots (schema_version semver-additive + review-queue slot coordination); F7 LOW Gutenberg-default rule on runtime-injection; F8 LOW design-system canonicity rule extension. Design-lens (2 BLOCKING): state-coverage matrix per wave (W3/W4/W5); cross-wave dogfooding flow (briefing → meeting → entity → action). WP-skill (7 findings): F1 HIGH InnerBlocks + `templateLock: false` + global inner-block inventory + `providesContext`/`usesContext` + `do_blocks($content)`; F2 HIGH `executeAbility()` (WP 7.0) + `useAbilityCursor()` hook + list envelope shape `{ items, next_cursor, total_hint }`; F3 MEDIUM per-project tint via CSS custom property on outer wrapper; F4 MEDIUM flag-flip preserves externalBin + build-mcp.sh + runtime-still-running smoke; F5 LOW dailyos block category already registered (don't re-register); F6 LOW apiVersion 3 mandatory; F7 LOW synced patterns for composition templates, template parts for chrome. Codex challenge (4 findings): F1 HIGH W6 parity checklist tightened with state-matrix/trust-fidelity/feedback-round-trip; F2 HIGH outer-envelope-carries-first-page-slice clause on §13 #1; F3 HIGH C4 supersedes path-α when substrate gap blocks named AC; F4 MEDIUM W0 enumerates all `createFileRoute` + cmd+k + feature flags + pre-flip dry-run. Codex consult (3 findings): F1 sha range fix `52d25db5..26062496 inclusive`; F2 `26062496` is DOS-336, not chrome lane; F3 ADR-0132 added to K-in + References.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:39:## 3. K-in record (substrate-grep audit, 2026-05-20)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:50:- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — substrate already exists for surface nonces / feedback router; v1.4.4 sub-L0 packets MUST grep substrate-type names, not proposed-name aliases.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:52:- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — reviewer L0 cycle policy. Wave packet must be sized so 5+ net-new findings per cycle = scope reset, not fold-and-continue.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:58:ADR-0083, 0102, 0105, 0108, 0111, 0125, 0128, 0129, 0130, 0132. Consumed unchanged. **No ADR overrides.** ADR-0129 §9's wave reorientation has been superseded by the 2026-05-17 renumber (locked in reorientation doc; v1.4.4 today is the WP Surface Migration, not the original v1.4.4 Claim Experience). ADR-0129's substrate decisions remain authoritative. ADR-0132 (Pill primitive dual existence) is the durable contract for the chrome `.Pill_*` vs block `.dailyos-pill*` split referenced in §10 invariants and DOS-722 pulled-forward work [consult F3].
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:68:| **W0** | Surface audit (gate) | All ~18 candidates (active / carry-forward / inactive / open-questions) | `src/router.tsx`, `.docs/design/INVENTORY.md`, nav reachability | ~1 audit doc + Linear sign-off |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:73:| **W5** | System / history surfaces | History block; Email / Settings surfaces (subject to W0 audit); adversarial fixtures | Existing audit + claim history substrate; v1.4.3 W6 audit attribution | ~1,500–3,000 (scope depends on W0) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:80:### §5.0 W0 — Surface audit (gate)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:82:**Issue:** [DOS-677](https://linear.app/a8c/issue/DOS-677) — v1.4.4 W0: Surface audit — active vs carry-forward vs inactive. Priority: High. **Gates W1+ start.**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:84:**Outputs:** five lists committed to `.docs/plans/v1.4.4-surface-audit.md`:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:88:- **Dev/admin surfaces** [arch F3] — surfaces that stay visible post-W6 flag-flip because they are runtime-host concerns, not end-user surfaces: settings-runtime, keychain debug, ability runtime status, MCP server status, dev tooling. These are NOT migration targets and NOT flag-flip targets. The list is produced by the W0 audit; it is not pre-enumerated at the wave-L0 layer.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:110:| Entity-detail trust-boundary hardening (CSO work) | [DOS-477](https://linear.app/a8c/issue/DOS-477) | W2 (gates DOS-462/483/484 close) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:116:| Receipt vs operational audit boundary | DOS-340 | W5 audit blocks vs W4 receipt rendering |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:118:**W0 audit may surface additional substrate gaps** — they land in W1 alongside the named items, not deferred.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:125:- DOS-459 envelope `schema_version` lifecycle policy — semver-style? major-version-only ABI?
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:141:**Substrate dependencies:** W1 envelope (DOS-459); W1 open-loops/touchpoints (DOS-460); W1 trust-boundary hardening (DOS-477) — gates close; W1 entity fixture harness (DOS-461) — gates verification; v1.4.3 primitives (TrustBandBadge, EntityChip, FreshnessIndicator, ProvenanceTag, IntelligenceQualityBadge, HealthBadge); chrome lane FolioBar (already shipped) + chrome shell.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:143:**Key acceptance shape:** per DOS-462, every claim-backed intelligence section (Health view: SentimentHero / TriageSection / DivergenceSection / OutlookPanel / SupportingTension / AboutIntelligence; Context view: StakeholderGrid / StrategicLandscape / ValueCommitments / QuoteWall / CommercialShape / etc.; Work view; Record/Timeline) routes through the envelope, not legacy `detail.intelligence` / local freshness fragments / page-local raw-state composers. Required visible QA states: full / empty / stale / `needs_verification` / corrected/superseded / proposal cue+drawer / proposal accepted/dismissed/edited / touchpoints present / cite chip evidence drawer.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:179:**Substrate dependencies:** W1 Receipt DTO (DOS-339); W1 `services::claim_receipt` + claim review queue extensions; W1 receipt privacy/redaction rules (DOS-341); W1 receipt-vs-operational-audit boundary (DOS-340); DOS-8 semantic feedback action surface; v1.4.3 W4 feedback router (merged `d781f2b4`); chrome shell.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:181:**Key acceptance shape:** receipts surface across Actions/Work + at least one upstream proof path (per DOS-339 ACs); receipt rendering supports `likely_current` / `use_with_caution` / `needs_verification` bands; semantic actions feed DOS-8 / DOS-318 service-owned reconciliation; no component writes claim/feedback/lifecycle state directly (all mutations through services per CLAUDE.md critical rule); per-claim_type render rules covered for all `BlockType` taxonomy entries from ADR-0130 §3.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:189:**Mission:** History block + Email surfaces + Settings surfaces. Final scope decided post-W0 audit. Adversarial fixtures land here.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:191:**Surfaces in scope (subject to W0 audit):**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:193:- Email surfaces — W0 audit decides active vs deprecate.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:194:- Settings surfaces — W0 audit decides scope; pieces may defer.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:197:**Substrate dependencies:** existing audit substrate; v1.4.3 W6 audit attribution (DOS-576); claim history primitives; existing settings persistence.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:199:**Key acceptance shape:** History block renders user-relevant claim history (not raw audit log per receipt-vs-operational-audit boundary DOS-340); Email/Settings surfaces decided per W0; adversarial fixtures (DOS-446) exercise the negative-case matrix.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:203:- Self-Healing surfaces — explicitly deferred to v1.4.9 unless W0 audit finds existing Tauri equivalent already in production.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:211:| **W2 (entity)** | full / empty / stale / `needs_verification` / corrected/superseded / proposal cue + drawer / proposal accepted/dismissed/edited / touchpoints present / touchpoints empty / cite-chip evidence drawer / subject-scope filtered (per DOS-462) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:213:| **W4 (action)** | empty queue (no open actions) / open-actions populated / claim-pending-review (stuck in review queue) / contradiction-flagged (distinct from corrected/superseded) / contradiction-resolved/superseded / inline-edit in-flight (feedback claim emitted, projection not yet returned) / inline-edit success (projection re-rendered) / inline-edit error / per-claim_type render rule branches (ADR-0130 §3 BlockType taxonomy) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:214:| **W5 (system/history)** | empty (no inbox activity this week) / populated-but-unintelligent (inbox activity, no claim outcomes) / populated-with-outcomes / receipt-vs-operational-audit boundary respected (per DOS-340) / adversarial fixture coverage (per DOS-446) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:216:W6 parity proof checks the matrix is reachable for every active surface; missing states blocks W6 close.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:257:- Tauri shell deprecation — staged removal (UI surfaces removed per-wave during v1.4.4) or flag-flip (all Tauri React UI hidden behind a build flag at W6 close)?
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:265:- Claim model + lifecycle + sensitivity + trust bands per ADR-0125.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:271:- W4-F local-to-local read path: `surface_runtime/project_composition` route, signed-loopback transport, surface session lifecycle.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:278:- Feedback write infrastructure: DOS-571 user-presence nonce + DOS-573 click-bound feedback router (merged at `d781f2b4`); REST route `/dailyos/v1/nonce/verify`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:279:- Audit + clean-machine validation: DOS-575 negative fixtures, DOS-576 SurfaceClient audit attribution, DOS-577 clean-machine Studio validation.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:280:- v1.4.3 stabilization fixes: DOS-671..675 lifecycle hardening; Studio sandbox compatibility (C3).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:293:- `services::claims::record_claim_feedback` (and W4-E nonce sibling) for feedback writes.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:302:**AC #W1 — Tauri ↔ WP parity gate (W6).** Every surface on the W0 active list has a working WP block equivalent that (a) renders real substrate (no façade over old data model), (b) supports the visible-QA-state matrix called out per W2 in DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / etc.), (c) user can do the thing the Tauri surface enabled.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:310:**AC #W5 — Substrate-in-same-wave (C4).** Every substrate item named in W1 ships in W1, not W2+. W0 audit's substrate-gap column drives W1 scope; any gap surfaced mid-W2 escalates W1 reopening, not W1 deferral.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:314:**C4-vs-path-α tie-breaker [codex challenge F3]:** C4 (substrate-in-same-wave) supersedes path-α offload when the substrate gap blocks a sub-L0 packet's named acceptance criterion. Path-α applies ONLY when the finding is hardening unrelated to a named AC. Worked example: a W2 reviewer finds DOS-459 envelope does not expose `inclusion_reason` for the StakeholderGrid chapter — the W2 visible-QA-state matrix for "subject-scope filtered" cannot be exercised → this is a **C4 reopen** of W1, NOT a path-α maintenance ticket. Memories `feedback_no_deferrals_period` (no deferrals) and `feedback_l2_path_alpha_to_maintenance_project` (path-α to maintenance) are not in conflict once this tie-break rule applies: C4-blocking findings stay in-wave; non-blocking hardening goes to maintenance.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:328:| Self-Healing surfaces (review queue, audit, skillify proposals) | **v1.4.9** Self-Healing v2 — unless W0 audit reveals existing Tauri equivalent already in production |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:347:**Forward-coupling hot spots [arch F6]:** (a) DOS-459 envelope `schema_version` lifecycle is forward-coupling-critical for v1.4.5/v1.4.6 envelope extension; W1 sub-L0 picks **semver-style with additive-only minor bumps** (new fields land at minor; breaking changes at major); v1.4.5 Workspace Memory adds source/ingestion claims and v1.4.6 Salience adds `RecommendationProposal` claims as **minor bumps** to the envelope shipped at W1. (b) Review-queue migration slot in v1.4.4 W4 is coordinated with v1.4.6 review-queue hook migration slot (per §13 #6 — v1.4.6 lands the DOS-336 candidate extension hook) to avoid v155-pattern collision; W4 sub-L0 documents the reserved slot block, v1.4.6 pre-L0 claims a disjoint block.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:371:| **L2 bounded by acceptance criteria (memory `feedback_l2_must_review_against_acceptance_criteria`).** Path-α findings → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. **C4 supersedes path-α when the substrate gap blocks a named AC — see AC #W5 tie-break.** | CLAUDE.md + engineering ladder + codex challenge F3 | Every sub-L0 packet sizes ACs tightly; L2 cycle-2 path-α offload per memory; C4-blocking findings reopen W1, not maintenance |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:385:**Pass rule:** unanimous APPROVE. Per CLAUDE.md pacing rule, 2 revision cycles without convergence ⇒ L6 escalation. Memory `feedback_review_loop_l6_policy` allows continued looping on architectural/critical/high; class-pattern recurrence triggers system-wide sweep per memory `feedback_systemic_look_for_recurring_issue_classes`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:388:- **W1 (substrate gaps):** add `/cso` or `security-auditor` per Amendment 3 (substrate touches trust-boundary hardening DOS-477, receipt privacy DOS-341, surface authorization).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:393:- **W6 (parity proof):** add `architect-reviewer` (integrated state); Suites S / P / E run as L3 wave-close.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:408:- `.docs/plans/v1.4.4-waves.md` — placeholder (superseded by this packet).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:438:7. **Settings surface migration shape.** Per W0 audit — full migration of all Tauri Settings to WP blocks? Thin admin surface for runtime/keychain settings on Tauri side, with theme/feedback settings as WP? Migration path matters for clean-machine UX (DOS-577 carries to W5 / W6). Partly resolved by audit (user-facing tabs in W5, diagnostics deferred to maintenance). **Resolves at:** W5 sub-L0.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:9:Wave: W3 chrome (canonically W3 per `.docs/plans/v1.4.4-waves.md` placeholder + `wp-foundation-roadmap-reorientation.md`) — **pulled forward as a parallel lane** before W0 surface audit / W1 substrate gaps / W2 entity surfaces land.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:46:**Pill scope clarification (V1.2, cycle-2 challenge fold):** `Pill.module.css` is lifted but Pill is NOT standalone runtime-injected chrome. Pill is consumed as a **child primitive INSIDE** the 4 shell modules (e.g., NavIsland active-state pill chips, FolioBar status indicators, AtmosphereLayer watermark labels). Pill itself remains a Gutenberg block (`wp/dailyos/blocks/pill/`) for body-content authoring. The dual-namespace coexistence (`.Pill_*` chrome / `.dailyos-pill*` block) is governed by AC #28 allowlist gate.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:60:- **V1.3.1 (2026-05-19 post-L0-close, post-W4-merge confirmation):** v1.4.3 W4 merged at `d781f2b4` ("v1.4.3 W4-F: feedback wire-through (DOS-683) (#326)") shortly after L0 closure. Empirical `git show --stat d781f2b4 -- 'wp/dailyos/theme/'` returns ZERO files — §10 W4-coupling matrix disjoint-write-sets assertion held in practice. §6 expanded with post-W4 substrate inventory confirming chrome lane PRs land cleanly against post-W4 `public/dev`. New worktree at `worktrees/v1.4.4-w3-chrome-lane` (branched from post-W4 `public/dev`) carries this revision forward. No L0 re-review triggered — empirical confirmation, not architectural change.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:62:- **V1.3 (2026-05-19, cycle-3 fold + cycle-4 challenge-only dispatch):** Cycle-3 panel returned 4× APPROVE (architecture, design-lens, codex consult, wp-block-themes) + 1× CONDITIONAL APPROVE (codex challenge, 4 clerical must-fix). All cycle-2 NEW BLOCKERS confirmed RESOLVED across all 5 reviewers. 4 clerical items + 2 advisory notes folded:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:64:  - **AC #30 `--color-alert-red` orphan resolved (challenge cycle-3 #1).** Regex now catches `var(--color-alert-red, #dc2626)` references (Patch 9a area + canonical FolioBar.module.css), but no `--wp--preset--color--alert-red` source exists in theme.json. V1.3 fix: add explicit alias `--color-alert-red: var(--wp--preset--color--spice-chili)` to `token-aliases.css` (chili at #9b3a2a is the existing critical/red semantic token; canonical chrome.js fallback `#dc2626` becomes a non-load-bearing safety net once alias resolves). AC #30 §5.1 updated with alert-red mapping classification.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:65:  - **§5.4.1 reference substrate acknowledgment (challenge cycle-3 #2).** Added explicit framing: `.docs/design/reference/_shared/` is canonical Tauri-side mockup substrate that ordinarily mirrors shipped source; §5.4.1's pre-W3 edit is a **deliberate design-system improvement** (introduces class-based refresh-button styling that didn't previously exist anywhere), not a one-off exception to the mirror rule. Treated as a normal canonical PR with codex review + design-reviewer + design-system author awareness, not a sync-rule violation.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:66:  - **AC #31 split into #31a + #31b (challenge cycle-3 #3).** V1.2 AC #31 was hybrid (static check + L4 hover/focus-visible proof). V1.3 splits: **AC #31a** static-grep verifiable — class rule present in canonical FolioBar.module.css, no `style=` attribute on refresh-button in canonical chrome.js, class reference present in `class: F('folioRefreshButton')` call. **AC #31b** L4-hands-on verifiable — hover state actually swaps color/border on reference HTML pages, focus-visible outline renders on keyboard tab.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:67:  - **§10 canonical source-of-truth escape hatch (challenge cycle-3 #4).** V1.2 invariant was absolute. V1.3 adds escape valve: WP-only CSS deviations are permitted IF they (a) require L0 amendment + (b) land as separate non-synced overlay file at `wp/dailyos/theme/assets/chrome/styles/wp-overlay-*.css` enqueued after the synced modules + (c) document the WP-specific reason. Direct edits to synced module CSS files remain forbidden. This preserves canonical mirror discipline while leaving a defined path for legitimate WP-only needs.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:68:  - **V1.2 misstatement corrected (architecture cycle-3 F1 + challenge cycle-3 note 2).** V1.2 §1 + §5.4.1 incorrectly claimed React `src/components/ui/folio-refresh-button.tsx` "already uses class-shaped React state" — verified file actually uses inline `style={{}}` + `onMouseEnter`/`onMouseLeave` callbacks, same pattern as canonical chrome.js pre-§5.4.1-upgrade. V1.3 rephrases to: React production component currently uses inline-style + JS-driven hover callbacks (same shape as pre-upgrade canonical chrome.js); §5.4.1 upgrades canonical mockup substrate to class-based, surfacing the misalignment between mockup and production. React-component class-based refactor is a separate Tauri-side improvement deferred to Tauri-freeze-lift release (§8 new deferral row).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:69:  - **POSIX-portable regex spelling (challenge cycle-3 note 1).** AC #30 regex updated from `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)` to `var\(--[A-Za-z0-9_-]+(,[^)]+)?\)` — drops the `?:` non-capturing-group syntax for BSD-grep portability. Functionally identical; supports macOS grep without -E enhancement requirements (still uses -E for `+`).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:71:  No new BLOCKERs in cycle-3. Convergence trajectory clean across 3 cycles: 4 CONDITIONAL (cycle-1) → 3 CONDITIONAL + 2 APPROVE (cycle-2) → 1 CONDITIONAL + 4 APPROVE (cycle-3). Cycle-4 dispatches codex challenge only (other 4 reviewers' concerns unchanged in V1.3); if challenge APPROVE, L0 closes unanimous.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:73:- **V1.2 (2026-05-19, cycle-2 fold + cycle-3 dispatch):** Cycle-2 panel returned 2× APPROVE (architecture, design-lens), 3× CONDITIONAL APPROVE (codex consult, codex challenge, wp-block-themes-grounded). 3 NEW BLOCKERS surfaced by codex challenge + 14 clerical items folded:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:91:  - **§5.1 digit-segment audit (wp-skill A1):** Numeric segments in theme.json `settings.custom.*` keys hyphenate digit-by-digit (`abc123` → `abc-1-2-3`). Chrome consumes `--space-2xl`, `--space-3xl`, etc. via theme.json `settings.spacing.spacingSizes`; theme.json palette tokens like `spice-turmeric-10` are alphanumeric slugs (not multi-digit numeric segments) and emit cleanly. Verified by reading `wp/dailyos/theme/theme.json` palette entries.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:94:  - **`docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` solution still controls W4 (consult re-affirmation).** W4 cycle-2 has not yet folded the K-in correction; W4 is independently blocked. W3 chrome lane is unaffected as long as PRs stay in `wp/dailyos/theme/**` per the §10 W4-coupling matrix.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:98:- **V1.1 (2026-05-19, cycle-1 fold + cycle-2 dispatch):** [retained from V1.1 — see git history; 20 cycle-1 fold items consolidated into the structural shape V1.2 inherits]
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:100:- **V1.0 (2026-05-19, initial L0 draft):** First L0 cycle. 4× CONDITIONAL APPROVE returned; folded into V1.1.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:102:## 3. K-in record (substrate-grep audit, V1.2 rescan)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:130:- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — lines 14-20 record the W4 V1.0 reinvention finding (existing `surface_nonce.rs` substrate covers nonce; W4 should extend not parallel). **Strengthens** the chrome lane's disjoint-from-W4 claim. See §10 W4-coupling matrix.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:140:ADR-0073, ADR-0076, ADR-0077, ADR-0129, ADR-0130 consumed; reviewers cited specific line numbers in cycle-1 + cycle-2 verdicts.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:142:**Verdict: K-in complete across cycles. 5 ADRs consumed; no conflicts; no reinvention. Triangulated across 5 cycle-2 reviewers + 2 cycle-1 reviewers.**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:185:**Digit-segment audit (V1.2, wp-skill A1 fold):** Theme.json `settings.custom.*` keys with multi-digit numeric segments hyphenate digit-by-digit (`abc123` → `abc-1-2-3`). Audit confirmed `wp/dailyos/theme/theme.json` palette tokens like `spice-turmeric-10` use alphanumeric slugs (not multi-digit numeric segments), so `--wp--preset--color--spice-turmeric-10` emits cleanly. Spacing presets `2xl`/`3xl`/`4xl`/`5xl` also clean (single character + letters). No digit-segment hyphenation issues in current scope; AC #30 grep would surface them if introduced later.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:200:     - **Special: `--color-alert-red`** (V1.3 explicit handling per challenge cycle-3 #1). Appears in canonical chrome.js Patch 9a area + FolioBar.module.css with fallback `#dc2626`. **NOT defined in `design-tokens.css`.** Mapping: `token-aliases.css` declares `--color-alert-red: var(--wp--preset--color--spice-chili)` — aliases to existing chili token (`design-tokens.css:39` defines `--color-spice-chili: #9b3a2a`; theme.json palette has corresponding slug). Canonical fallback `#dc2626` remains as a non-load-bearing safety net for stock-theme contexts where chrome doesn't render anyway. Future option: define `--color-alert-red` as net-new token in `src/styles/design-tokens.css` if alert-red semantic distinction from chili emerges; for now, alias is sufficient.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:341:**Reference substrate acknowledgment (V1.3, challenge cycle-3 #2 fold):** `.docs/design/reference/_shared/` is canonical Tauri-side mockup substrate that ordinarily mirrors shipped source. §5.4.1's pre-W3 edit is a **deliberate design-system improvement** (introduces class-based refresh-button styling that didn't previously exist anywhere on either surface), NOT a one-off exception to the mirror rule. Treated as a normal canonical PR with codex review + design-reviewer + design-system author awareness — not a sync-rule violation.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:394:**V1.3.1 W4-merge confirmation (2026-05-19, post-L0-close):** v1.4.3 W4 merged at commit `d781f2b4` ("v1.4.3 W4-F: feedback wire-through (DOS-683) (#326)"). Empirical check of `git show --stat d781f2b4 -- 'wp/dailyos/theme/'` returns **zero files** — chrome lane's §10 W4-coupling matrix disjoint-write-sets assertion held in practice. W4 touched:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:397:- `wp/dailyos/includes/class-dailyos-plugin.php` — +290 LOC (new `/dailyos/v1/nonce/verify` REST route + supporting infra). Chrome lane does NOT consume this route (chrome reads CPT info via PHP page-context functions, not via REST). No impact on chrome.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:401:- `src-tauri/src/services/surface_nonce.rs` (+751 LOC), `surface_runtime/mod.rs` (+176 LOC), `version_dispatcher.rs` (+132 LOC), other substrate. Chrome lane is theme-side only; no Rust dependency.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:424:| Chrome-as-Gutenberg-blocks (~700-1100 LOC / 2-4 days per cycle-1 challenge cost analysis) | **DOS-723** (Codebase Maintenance, Medium) | Separate L0 packet when scoped |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:467:  **V1.3 escape hatch (per challenge cycle-3 #4):** Any chrome improvement that benefits both Tauri-side mockup substrate and WP-side rendering MUST land in canonical first (per §5.4.1 precedent), then re-sync. **WP-only CSS deviations** are permitted IF AND ONLY IF: (a) the deviation requires an L0 amendment documenting why a WP-only rule is correct (e.g., editor-iframe-specific styling that doesn't apply to Tauri), AND (b) it lands as a separate non-synced overlay file at `wp/dailyos/theme/assets/chrome/styles/wp-overlay-*.css` enqueued AFTER the synced modules, AND (c) the overlay file's purpose + scope is documented in its header comment. The synced module CSS files (`{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css`) themselves are not edited under any circumstance — overlays add WP-only rules without modifying canonical-mirrored content. This preserves the one-way sync discipline while leaving a defined path for legitimate WP-only needs.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:476:Other 4 cycle-3 reviewers (architecture, design-lens, codex consult, wp-block-themes) returned unconditional APPROVE in cycle-3 against V1.2. V1.3 deltas (AC #30 alert-red alias, §5.4.1 reference acknowledgment, AC #31a/#31b split, §10 escape hatch, V1.2 misstatement fix, POSIX regex) do not touch any of their cycle-3 concerns. Their APPROVE verdicts carry forward to V1.3.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:478:Cycle-4 dispatches `/codex challenge` only to verify the 4 clerical folds resolve. If APPROVE, L0 closes unanimous (4 carry-forward + 1 fresh). Verdict artifact: `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle4.md`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:480:Same 5-reviewer panel. Re-dispatched against V1.2 with explicit "verify V1.2 fold items resolve cycle-2 BLOCKERS + clerical items" framing.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:484:| `/codex challenge` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle3.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:485:| `ce-architecture-strategist` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:486:| `/codex consult` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:487:| `ce-design-lens-reviewer` | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:488:| WP-block-themes-grounded (general-purpose + skill refs) | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:490:**Pass rule:** Unanimous APPROVE. **CLAUDE.md pacing rule says "2 revision cycles without convergence ⇒ L6"** — V1.2 is the 2nd revision (cycle-3 dispatch). If V1.2 produces unanimous APPROVE, L0 closes. If any reviewer returns BLOCKED or substantive CONDITIONAL, L6 escalation per CLAUDE.md (single revision cycle remaining ≤ 0). Memory `feedback_review_loop_l6_policy` allows continued looping for high/critical/architectural issues vs L6 at strict cycle count; this packet treats cycle-3 as the convergence target with L6 as the escalation if blockers persist.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:3:**Current revision: V1.1 (cycle-1 fold, 2026-05-20). V1.0 was BLOCKED on architecture; V1.1 lifts the blockers. See §2 Changelog.**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:18:- [DOS-340](https://linear.app/a8c/issue/DOS-340) — Provenance receipt vs operational audit boundary
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:28:- **W4 — Action surfaces:** Actions/Work block (DOS-514), Activity Log block (DOS-444), Action Detail, Lint Mode (DOS-445), review queue (DOS-443). Consume DOS-339 receipts + DOS-8 semantic feedback + DOS-340 audit boundary + DOS-341 privacy.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:45:- **V1.0 (2026-05-20, initial L0 draft):** First L0 cycle. 10 sub-tickets covered. Pre-dispatch to 5-reviewer panel (codex challenge + codex consult + ce-architecture-strategist + `/cso` mandatory + ce-correctness-reviewer). Substrate ground truth verified: claim_receipt substrate from DOS-701 already shipped (contracts.rs / render.rs / auth.rs at 319+290+175 LOC); 5 placeholder files remain zero-byte; `get_entity_context` ability exists; `prepare_meeting` ability exists; `list_open_loops` ability + `services::context::ListOpenLoopsQuery` exist; `services::claims::record_claim_feedback` + `services::claims::reconcile_contradiction` confirmed at `services/claims.rs:6700` + `:8906`; no existing `get_entity_intelligence` envelope OR `get_daily_briefing` ability; no existing `meeting_prep_status` service. Slot block v250–v269 claimed (v240 taken by DOS-701 W1-RECEIPT migration).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:47:- **V1.1 (2026-05-20, cycle-1 fold — lifts V1.0 BLOCKED verdict):** Folds 23 findings from the 5-panel cycle-1 review. Verdict artifacts at `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-{architecture,correctness,cso,codex-challenge,codex-consult}-cycle1.md`. James's locked decisions 2026-05-20 encoded inline.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:53:    - F3 HIGH (DOS-335 signals gap) → new `MeetingPrepStatusChanged` signal pre-declared in `signals/policy_registry.rs` at W1 kickoff, mirroring v1.4.6 W0 cycle-4 fix C pattern; §5.5 Intelligence Loop check item 3 rewritten.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:65:    - F6–F10 (envelope error propagation, fixture matrix, idempotency window, audit denylist extensibility, prep state machine) → folded into AC additions (AC-459.10, AC-461.5b, AC-8.9, AC-340.7, AC-335.13).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:74:    - F8 (Agent actor denial) → AC-8.13: in v1.4.4 W1, `Actor::Agent` is denied at all surfaces; matrix row collapses to deny everywhere; broadening waits for v1.4.7 MCP scope manifest.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:77:    - F11 (OperationalAuditStorage enforcement) → AC-341.11: `check_audit_disclosure_allowlist.sh` extended to forbid any direct read of `maintenance_audit` from `services::claim_receipt::*`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:78:    - F12 (AgentMcp audience) → AC-341.12: explicit AgentMcp row with field allowlist (trust band, freshness coarsened, redaction level, lifecycle, sanitized evidence_summary, subject_type); forbid source labels, source_asof timestamps, claim_id.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:82:    - F3 LOW (§5.8 IL check Q5 thin) → §5.8 Q5 re-authored: "Boundary violations detected by §5.8 CI lint or by user reports of leaked audit-only fields feed back as DOS-8 `WrongSource`/`SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract."
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:89:  **Cycle-2 dispatch:** Not required. All 23 findings either (a) folded into V1.1 inline, (b) routed to maintenance project as path-α, or (c) info-only. Re-dispatch only if any cycle-1 reviewer disputes a fix surface. Per memory `feedback_l0_partial_convergence_when_class_recurs`: V1.1 reflects scope-correct, contract-complete.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:91:## 3. K-in record (substrate-grep audit)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:119:**Greps run:** `entity_intelligence`, `claim_receipt`, `meeting_prep`, `briefing`, `trust_boundary`, `feedback_action`, `provenance`, `redaction`, `open_loops`, `touchpoints`, `sensitivity`, `subject_isolation`, `audit_boundary`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:125:- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — when reviewers surface 5+ net-new findings per cycle, scope is wrong. Applied in §4: 10 sub-tickets stay scoped to substrate ONLY; surface-side concerns route to W2–W5 packets.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:157:| DOS-335 | `services::meeting_prep_status::{read,write}` (new — split per cycle-1 architecture F2) | W3 briefing chrome + meeting detail + FolioBar readiness signal | Y — v241 `meeting_prep_status_signals` index + v242 `meeting_prep_status_dismissals` | architect + codex challenge + `/cso` (touches sensitivity on user-authored notes) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:160:| DOS-340 | `services::claim_receipt::boundary` (fills DOS-701 placeholder) + `scripts/check_audit_disclosure_allowlist.sh` CI lint | All W2–W5 surfaces (boundary, not consume) | N — render-side filter + CI gate | **`/cso` MANDATORY** + correctness + codex challenge |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:214:  // All list-shape fields paginated per cycle-1 architecture F5 + correctness F4
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:279:2. *Provenance + trust:* Every `EntityFact` carries `source_asof`, `trust_band`, `freshness`, and `provenance: ProvenanceRef` (per cycle-1 architecture F9 / ADR-0130 §2 amendment — points into envelope-level `EnvelopeProvenance.sources`; display-safe with DOS-477 redaction applied at the service boundary, not the renderer). Missing `source_asof` → `Freshness::Unknown` + currentness caveat (never fabricated).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:280:3. *Signals + invalidation:* Envelope re-renders on existing claim-lifecycle signals + entity-link signals (DOS-258 path); no new signals from W1-A. `signal_policy = { emits_on_output_change = [], coalesce = false }` matches `get_daily_readiness` pattern.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:282:5. *Feedback loop:* Receipt-shaped actions on envelope items (e.g., `WrongSubject` on a metadata_proposal) route through DOS-8 typed feedback (§5.7); corrections feed claim lifecycle via existing `services::claims::record_claim_feedback`. Envelope re-fetches reflect the new state through standard lifecycle signals.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:291:- **AC-459.5** — Every rendered claim/proposal item carries DOS-477 target binding: `claim_id` OR `proposal_id`, `subject_ref`, `field_path`, `sensitivity`, `trust_band`, lifecycle state, display-safe provenance.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:292:- **AC-459.6** — Tests cover: account / project / person / empty / stale-fact / corrected-superseded / ambiguous-association / project-account-overlap / wrong-subject correction / subject isolation. Fixture suite cross-referenced from DOS-461 harness (§5.3).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:293:- **AC-459.7** — Reuse audit: at L0 plan, identify which existing producers are reused (`get_entity_context`, `list_open_loops`, `services::accounts`, etc.) and justify any new reader/composer.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:295:- **AC-459.9** — (cycle-1 architecture F5 + correctness F4) Every list-shape field in the envelope carries a typed `Paginated<T>` with `next_cursor` + `CursorState`. Consumer pagination tested via re-invocation; cursor is opaque server-signed (§13 Q11) and survives schema changes. Concurrent insert/retract during pagination is tested: harness inserts a claim between page fetches and asserts behavior matches the declared `CursorState`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:296:- **AC-459.10** — (cycle-1 correctness F6 + residual R3) Partial-failure + signal-driven refresh semantics specified. Per-section composition is independent (one section failing emits `Empty { reason: PartialFailure }`, others continue). Errors affecting subject identity OR auth fail the entire envelope. Signal-driven envelope refresh tested: every claim-lifecycle transition (`Active → Contested` etc.) triggers re-render within signal propagation budget. Fixture: `account_partial_section_failure.json`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:311:  pub upcoming: Paginated<Touchpoint>,       // per cycle-1 architecture F5
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:329:  pub provenance: ProvenanceRef,             // per cycle-1 architecture F9; points into envelope-level EnvelopeProvenance
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:338:  pub provenance: ProvenanceRef,                                          // per cycle-1 architecture F9
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:353:3. *Signals + invalidation:* Re-renders on entity-link signals (DOS-258), calendar-sync signals, claim-lifecycle signals. No new signals.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:383://       ├── account_corrected_superseded.json
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:404:3. *Signals + invalidation:* Fixture cases include stale-fact + corrected-superseded → harness verifies envelope reflects current state.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:415:- **AC-461.5** — Fixtures include stale fact, corrected-superseded claim, low-trust claim, metadata proposal (per subject-matrix below), open loop, upcoming touchpoint, recent touchpoint, thread summary, confidential/user-only claim, Glean citation, wrong-subject claim.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:416:- **AC-461.5b** — (cycle-1 correctness F7) Per-subject expected-fixture matrix is explicit. Absent expected fixture = harness fail:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:430:- **AC-461.6a** — (cycle-1 correctness F2 — binding check) Text-extraction harness pass: every visible claim-substantive string in the rendered DOM (text originating from `IntelligenceClaim.text` / `RenderableClaimText.body` per the envelope) MUST trace back to a `[data-claim-id]` ancestor. Text-without-binding FAILS. Red-first no-bypass test also FAILS if any proof path uses legacy AI JSON / `get_entity_context_entries` fallback / raw SQL / `AppState` / `ActionDb` / page-local intelligence composers / legacy direct detail commands as the source of rendered claim-backed intelligence.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:431:- **AC-461.6b** — (cycle-1 correctness F2 — stale vs bypass) Unresolved `data-claim-id` (claim no longer in envelope but DOM was cached from a prior render) renders as **stale**, NOT **bypass**. Distinguished failure mode with its own assertion + fixture (`account_claim_retracted_mid_render.json`). Harness produces separate counters for `stale_render_count` and `bypass_count`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:433:- **AC-461.8** — Reuse audit artifact in proof bundle lists existing abilities / services / DTOs / tests / UI primitives reused (per CLAUDE.md DoD #7 style).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:448:/// `composes` declaration) per cycle-1 CSO F3. Walker traverses the composition graph
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:467:/// Composes (does NOT re-implement) the shipped sensitivity gate per cycle-1 CSO F1.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:472:  actor: RenderActor,                // not the packet-introduced `Actor` — per cycle-1 CSO F1
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:473:  surface: RenderSurface,            // not `SurfaceContext` — per cycle-1 CSO F1
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:477:  // filesystem paths / private snippets / raw prompt/audit identifiers unless render policy allows reveal.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:485:- Delegates to shipped `services::claim_receipt::auth::can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>` (DOS-701; signature corrected per cycle-1 codex-consult F1).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:486:- Composes shipped `abilities_runtime::sensitivity::render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)` (per cycle-1 CSO F1). NO parallel sensitivity gate.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:500:- **AC-477.1** — Every rendered claim/proposal row in Account/Project/Person detail proof paths carries `claim_id` OR `proposal_id`, `subject_ref`, `field_path`, `sensitivity`, `trust_band`, lifecycle state, display-safe provenance.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:503:- **AC-477.4** — Cite chips + evidence drawers use sanitized display labels via `redact_provenance_for_surface`. No raw source_ref / Glean doc IDs / chunk IDs / meeting IDs / URLs / emails / filesystem paths / private snippets / raw prompt/audit identifiers unless render policy explicitly permits.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:505:- **AC-477.6** — Fixtures (cross-ref DOS-461 §5.3) include confidential/user-only claims, Glean citations, stale claims, corrected/superseded claims, wrong-subject claims, PII-safe fake data only.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:510:- **AC-477.11** — (cycle-1 CSO F1) `services::entity_intelligence::auth::redact_provenance_for_surface` MUST compose shipped `render_policy_for_surface` + `renderable_claim_text_with_value` — never re-implement. CI lint `src-tauri/scripts/check_sensitivity_gate_composition.sh` (modeled on `check_claim_writer_allowlist.sh`) forbids any `match … sensitivity` against `ClaimSensitivity` variants outside `abilities-runtime/src/sensitivity*`. Pairs with the existing `prompt-channel-sensitivity-class-sweep` precedent (`docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:511:- **AC-477.12** — (cycle-1 CSO F2 — allowlist-primary boundary) Receipt allowlist (`RECEIPT_ALLOWED_FIELDS` from §5.8) is the **primary** boundary contract. Denylist (`AUDIT_ONLY_DENYLIST`) is a redundant CI lint that fails the build if any field outside the allowlist appears in a `ClaimReceipt`-JSON-serialized snapshot. `filter_for_receipt` panics on encountering unknown field names (fail-loud, per Rule 11). Snapshot fixture set under `src-tauri/tests/claim_receipt_boundary/` — one serialized ClaimReceipt per (`SurfaceContext` × `ClaimSensitivity`) cell. New fields added to `ClaimReceipt` FORCE snapshot update + allowlist amendment.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:512:- **AC-477.13** — (cycle-1 CSO F3 — transitive composes) `validate_envelope_target` accepts an envelope-set (parent + transitively composed child envelopes via abilities-runtime `composes` declaration). Each ability's `composes = [...]` metadata defines the transitive set; the validator walks the composition graph at invocation-time. Property test: for every `(parent_ability, child_ability)` in `composes`, assert that a target valid for the child is valid for the parent. Failing pair = drift between renderer and validator. Concrete consumer: §5.10 DOS-507 daily briefing composes `get_entity_intelligence` per referenced subject; a `MarkFalse` on a claim shown in the briefing must validate against the transitively-composed envelope.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:583:// Per cycle-1 architecture F2 — read/write split enforces ADR-0102 §3 Read-ability call-graph.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:598:**State transition table** (per cycle-1 correctness F10; `next_allowed_transition` field references this):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:624:- W2 Meeting Detail block (when authored — currently in W0 audit list).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:629:3. *Signals + invalidation:* New invalidation path: entity-link signal (DOS-258) → `write::enqueue_refresh(RefreshReason::EntityRelinked)`. Recent-correction signal → `StaleReason::RecentCorrection`. **NEW signal emitted** (per cycle-1 architecture F3): `MeetingPrepStatusChanged { meeting_id, transition: PrepStatus → PrepStatus }`. Pre-declared in `signals/policy_registry.rs` at W1 kickoff, mirroring v1.4.6 W0 cycle-4 fix C pattern (`.docs/plans/v1.4.6-waves.md` §348). Consumers (FolioBar readiness chrome, Meeting Briefing block, Daily Briefing rollup) subscribe to this signal — no poll-on-render drift.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:630:4. *Runtime + surfaces:* FolioBar (W3 chrome — already substrate-rendered), Meeting Detail (W2/W3 — TBD per W0 audit), Daily Briefing (W3), Meeting Briefing (W3). Same DTO across all.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:631:5. *Feedback loop:* User suppression / dismissal writes update `status` to `UserSuppressed` / `UserDismissed`. These do not feed claim lifecycle (per ticket: "silent-by-default preparation; no proactive notifications, no insight queue, no interruption policy") — they only gate prep refresh re-enqueue.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:634:- v241 — `meeting_prep_status_signals` (indexed view materializing the join across `meetings` + `meeting_entity_links` + `meeting_prep_outputs` + `claim_invalidation_queue` rows for fast status compute). Use existing `services::invalidation_jobs` substrate; no new tables. (Renumbered from V1.0's v250 per cycle-1 architecture F1.)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:639:- **AC-335.2** — Implementation audit names every current prep/readiness writer + reader + fields used.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:646:- **AC-335.9** — Refresh respects correction / tombstone / lifecycle / trust / sensitivity / provenance invariants.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:648:- **AC-335.11** — Tests cover stale entity context / recent correction / ambiguous attendee match / no-op refresh / source-revoked evidence / manual link/relink/unlink / status convergence across FolioBar + event components / user-authored note preservation. Additional case: `decision_authored_during_refresh_eventually_visible_in_next_status_recompute_without_lost_update` (per cycle-1 correctness F1).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:649:- **AC-335.12** — (cycle-1 architecture F2) `services::meeting_prep_status::read::compute_status` call graph contains zero mutations; verified by a trybuild test OR call-graph lint at registration time per ADR-0102 §3. Static check lands as part of W1, not deferred. Pairs with §5.10 AC-507.7.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:650:- **AC-335.13** — (cycle-1 correctness F1 — write-commutativity contract, two-part):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:652:  - **Part B:** Decision/commitment claim emission via `write::record_user_authored` is a claim-store write, NOT a disjoint-column write. Status recompute observes the new claim through the standard claim-lifecycle-signal → invalidation path. Contract: "eventual consistency via existing signal substrate," not "commutes." The §13 Q7 V1.0 recommendation is corrected accordingly.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:653:- **AC-335.14** — (cycle-1 correctness F10) State transition table is exhaustive (every variant has an entry — see table above). Illegal transitions return `TransitionError`; legal transitions succeed.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:654:- **AC-335.15** — (cycle-1 architecture F3) `MeetingPrepStatusChanged` signal pre-declared in `signals/policy_registry.rs` at W1 kickoff. Consumer subscription test: dismissal write emits signal within 1 invalidation cycle; FolioBar readiness chrome re-renders without poll.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:661:- `src-tauri/src/services/claim_receipt/contracts.rs` (175 LOC) — full `ReceiptTarget`, `SurfaceContext`, `Freshness`, `RedactionLevel`, `ReceiptTrust`, `ReceiptLifecycle`, `ReceiptProvenance`, `ProvenanceSource`, `ClaimReceipt` per the L0 Addendum.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:662:- `src-tauri/src/services/claim_receipt/render.rs` (290 LOC) — `render_receipt_for(state: &AppState, target: ReceiptTarget, surface: SurfaceContext) -> Result<ClaimReceipt, RenderError>` (signature corrected per cycle-1 codex-consult F2).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:663:- `src-tauri/src/services/claim_receipt/auth.rs` (319 LOC) — `can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>` (signature corrected per cycle-1 codex-consult F1) + positive/negative fixtures per sensitivity class.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:667:**Proposal-receipt deferral note** (per cycle-1 codex-consult F3): `render_receipt_for` shipped at PR #323 only handles the `Claim` arm of `ReceiptTarget`. The `Proposal { .. }` and `WorkItem { .. }` arms return `RenderError::TargetNotFound`. DTO ships all three variants but render only implements Claim. **W4 (Actions/Work) does NOT budget against proposal-receipt rendering end-to-end** — `submit_claim_feedback` (§5.7) on a Proposal target returns a typed "no receipt yet, target-only" response. Either DOS-701-deferred follow-on or W4-specific extension; surface flag in W4 L0 to avoid downstream surprise.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:686:3. *Signals + invalidation:* Receipt state updates on claim lifecycle / feedback / contradiction / source state changes.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:693:- **AC-339.1** — TypeScript hook `useClaimReceiptSubscription(target, surface)` exists; subscribes to claim-lifecycle signals; re-renders on signal. The Tauri command behind it threads `&AppState` through to the shipped `render_receipt_for(&AppState, target, surface)`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:696:- **AC-339.4** — No component writes claim / feedback / lifecycle state directly; all mutations route through `services/`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:697:- **AC-339.5** — Reuse audit names owner for each behavior: service / ability / DTO / hook / primitive (per CLAUDE.md DoD #7).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:698:- **AC-339.6** — (cycle-1 architecture F4 — multi-surface fan-out) Receipt re-emission signal named explicitly: `ClaimVerificationStateChanged` (existing per ADR-0080 substrate; verify at L1). Coalesce policy on `useClaimReceiptSubscription`: **250ms trailing-edge debounce per `(target.claim_id, surface)` pair**. Test: feedback action on claim C from Actions/Work re-emits receipt to all subscribed Entity Detail / Briefing / Activity Log surfaces within 1 invalidation cycle; coalesce window prevents > N re-emits per (claim, surface) per second under burst feedback (e.g., user marks 5 stale claims in rapid succession).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:726:  pub lifecycle_changed: bool,
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:732:  state: &AppState,                            // per cycle-1 codex-consult F1 — shipped can_surface_for needs &AppState
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:734:  actor: &RenderActor,                         // per cycle-1 CSO F1 — not the proposed `Actor`
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:740:  //    (signature: &AppState, &RenderActor, RenderSurface, &str — per cycle-1 codex-consult F1)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:741:  //    Actor::Agent collapses to deny everywhere in v1.4.4 (per AC-8.13).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:756:**Symbol corrections from V1.0** (per cycle-1 CSO F1 + codex-consult F1/F2):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:773:| WrongSubject | "About wrong account/person" | → `Contested` | row + subject reattribution candidate | Adds | required `corrected_to: Option<SubjectRef>` (per ADR-0123 §1; per cycle-1 CSO F5) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:774:| WrongSource | "Bad source" | → `Contested` | row + source-reliability decay | (no change) | required **stable source content hash** per ADR-0131 (NOT `source_index` or `source_ref`; per cycle-1 CSO F6 + AC-8.11) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:780:**Authorization matrix (per CSO requirements in dissolved archive class C5; cycle-1 CSO F8 tightened Agent rows to deny everywhere — see AC-8.13):**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:794:2. *Provenance + trust:* Feedback updates lifecycle + verification state which feeds trust-band recompute via existing substrate.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:795:3. *Signals + invalidation:* Successful feedback emits existing claim-lifecycle signals → consumers re-render via DOS-339 receipt subscription.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:797:5. *Feedback loop:* THIS IS the feedback loop primitive. The Intelligence Loop's "feedback closes" guarantee is the contract this ticket lands.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:810:- **AC-8.9** — (cycle-1 CSO F4 + correctness F8) User-authored free-text fields in `ClaimFeedbackRequest.metadata` are persisted with `ClaimSensitivity::Confidential` by default (NOT `Internal`), unless the originating claim's sensitivity is higher (inherit). Render policy for surface unchanged — `Confidential` is hidden from MCP / log structured by default, click-to-reveal in Tauri. Document the default in ADR-0123 amendment (or v1.4.4 amendment-bundle). Retry-without-prior-key semantics tested: dropped-response retry produces at most one persisted feedback row per (claim, action, actor, timestamp window) — per `services::claims::record_claim_feedback` existing idempotency contract over `(claim_id, action, actor)` per `claims.rs:13283` test.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:811:- **AC-8.10** — (cycle-1 CSO F5) Per-action metadata JSON schema MUST match ADR-0123 §1 variant payload field names **verbatim**. Specifically: `WrongSubject` carries `corrected_to: Option<SubjectRef>` (NOT `intended_subject_ref`); `WrongSource` carries the source-content-hash field per AC-8.11 (NOT `source_index`); `NeedsNuance` carries `corrected_text: String`; `SurfaceInappropriate` carries `surface: SurfaceId`; `NotRelevantHere` carries `invocation_id: InvocationId`. Schema test asserts JSON-schema-emitted field set == ADR-0123 §1 variant field set (golden parity test, similar to TS mirror parity).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:812:- **AC-8.11** — (cycle-1 CSO F6) `WrongSource` metadata MUST carry a stable source content hash (per ADR-0131 structured embedding canonicalization), NOT an index. Server validates the hash against the **current** claim's source set at feedback-apply time. If the source no longer exists in the set, the feedback is rejected with `BadRequest::SourceNoLongerInClaim` (caller is asked to re-render + resubmit). Optionally retain `source_index` as a presentation hint but never as the authoritative identifier. Adversarial fixture: `feedback::tests::wrong_source_index_aliasing_race` — race a source revocation against a `WrongSource` submission; assert no source-reliability decay applied to innocent successor source.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:813:- **AC-8.12** — (cycle-1 CSO F7) User-authored `corrected_text` (NeedsNuance), `note` (CannotVerify), and any other free-text field in `ClaimFeedbackRequest.metadata` passes through the ADR-0108 §3 sanitizer pipeline BEFORE persistence. Sanitizer instance is shared with the existing `FieldAttribution.explanation` path. Sanitization warnings surface in the response (e.g., `ProvenanceWarning::ExplanationFiltered`) so the user knows their text was modified. CI lint forbids any persistence path for user-authored claim-adjacent text that bypasses `abilities_runtime::sanitizer::*`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:814:- **AC-8.13** — (cycle-1 CSO F8) In v1.4.4 W1, `actor: Actor::Agent` is **denied** at all surfaces (NOT allowed-up-to-Internal as V1.0 stated). The matrix row collapses to deny everywhere until an explicit ADR-0123 amendment + v1.4.7 MCP scope manifest expands it. Tighten in §5.7 substrate; broadening waits for v1.4.7 path. Authorization matrix table-test in `claim_receipt::feedback::tests::authorization_matrix` — for every `(surface × actor × sensitivity)` cell, assert the matrix outcome.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:815:- **AC-8.14** — (cycle-1 CSO F9) Server-issued idempotency key scoped per `(claim_id, action, actor, metadata_hash)`. TTL ≤ 60 seconds. Outside the window, retries are treated as new submissions (since two genuinely-distinct feedbacks at >60s separation reflect distinct user intent). Test: `feedback::tests::idempotency_window_boundary` — submit two distinct feedbacks on same claim with same metadata; second succeeds if outside 60s.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:819:### §5.8 — DOS-340: Provenance receipt vs operational audit boundary
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:834:  "lifecycle_state",
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:841:  "internal_audit_id",
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:856:  "correlation_id",       // when surfaced outside operational audit viewer
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:866:**CI lint script:** `src-tauri/scripts/check_audit_disclosure_allowlist.sh` modeled on existing `check_claim_writer_allowlist.sh` — forbids `audit_log` table reads outside an allowlisted set of audit-management commands. Scans `services/` AND `commands/`:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:867:- Allowed readers: `commands/audit_management*` ONLY.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:870:**Producer placement:** `services::claim_receipt::boundary` (Rust runtime filter); `scripts/check_audit_disclosure_allowlist.sh` (CI gate).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:872:**Consumer surfaces:** All W2–W5 surfaces consume the filter. The Activity Log + Lint surfaces (W4) consume the four product buckets (`fixed_automatically` / `watching` / `needs_evidence` / `needs_user_decision`) instead of raw audit rows.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:876:2. *Provenance + trust:* Boundary preserves provenance trust by preventing receipts leaking audit-only fields that would let users mistake audit identifiers for evidence.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:879:5. *Feedback loop:* (per cycle-1 codex-challenge F3) Boundary violations detected by the §5.8 CI lint OR by user reports of leaked audit-only fields feed back as DOS-8 `WrongSource` / `SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract. A surface that accidentally exposes audit-only fields will surface user feedback that loops back into source-reliability decay through the existing DOS-8 path.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:884:- **AC-340.1** — Receipt allowlist + audit-only denylist encoded in `boundary.rs` constants + enforced in `filter_for_receipt`/`filter_for_activity_log`/`filter_for_lint`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:885:- **AC-340.2** — CI lint script `check_audit_disclosure_allowlist.sh` committed + green; rejects `audit_log` table reads outside allowlist.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:886:- **AC-340.3** — Suite S test for every audit-only field per L0 Addendum — MUST NOT appear in any receipt rendering snapshot.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:887:- **AC-340.4** — Activity/Lint product buckets land as service-side typed projections (precedence rules from claim lifecycle/verification/contradiction/repair states); raw audit reads forbidden.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:890:- **AC-340.7** — (cycle-1 correctness F9) CI lint `check_audit_denylist_completeness.sh` enforces that every column added to the operational audit storage schema (any migration touching `audit_log` or `provenance_audit_storage`) explicitly adds an entry to `AUDIT_ONLY_DENYLIST` OR includes a comment justifying receipt-safety (`// receipt-safe: <reason>`). Pattern modeled on existing `check_claim_writer_allowlist.sh`. Pairs with AC-477.12 allowlist-primary boundary.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:896:**Contract shape (allowlist-primary build, NOT post-render transform — per cycle-1 CSO F10):**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:911:  "lifecycle_state",
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:930:  // Runs INSIDE render_receipt_for (per cycle-1 CSO F10).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:938:| Product receipt — UserTauri (Tauri / WP block render — both are local-to-local, same trust boundary per ADR-0129) | generic source label, source type, source-as-of, trust band, field/topic path, redacted evidence summary, lifecycle state | raw source id, prompt hash, internal audit id, local file path, private URL, full private email, full private snippet, inaccessible doc title, raw command/invocation metadata |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:939:| **AgentMcp (v1.4.7 consumer; explicit row per cycle-1 CSO F12)** | **trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary, subject_type** | **source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak), subject_id, raw source ids, prompt hashes, audit ids, file paths, private URLs/emails/snippets, command/invocation metadata** |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:940:| Activity (W4 Activity Log) | user-readable event, subject label, bucket, timestamp/source-as-of, surface, link to receipt/queue item | raw audit rows, command invocation metadata, prompt hashes, debug payloads |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:941:| Lint (W4 Lint Mode) | finding type, subject label, bucket, severity, link to receipt/queue item | raw audit rows, runtime evaluator output, prompt hashes, debug payloads |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:942:| Operational audit (storage) | correlation ids, raw source ids, prompt/invocation metadata, tamper-evident record ids, debug payloads, raw model/tool i/o, raw document/message bodies (per retention policy) | (no surface — storage only; non-disclosure tag enforced via AC-341.11 lint extension) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:952:4. *Runtime + surfaces:* Render policy varies by Tauri / WP block / MCP / Activity / Lint / audit storage.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:962:- **AC-341.5** — Activity / Lint surfaces (W4) do not expose operational audit-only fields (cross-referenced with DOS-340 boundary CI lint §5.8).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:965:- **AC-341.8** — No raw audit-only field appears in shared receipt DTO snapshots (snapshot test).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:967:- **AC-341.10** — (cycle-1 CSO F10 + correctness F5 — allowlist-primary + derived-claim handling) `build_receipt_for_audience` is a render-time primitive (NOT a post-render transform); audience is an input to construction. Same target + audience → byte-identical receipt. No post-render mutation; receipts are immutable. CI lint asserts JSON key set per audience is a subset of the audience-specific allowlist. Plus derived/composed-claim handling:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:972:- **AC-341.11** — (cycle-1 CSO F11 — OperationalAuditStorage enforcement) `services::claim_receipt::privacy::OperationalAuditStorage` audience is a **non-disclosure tag, not a render target**. Extend `check_audit_disclosure_allowlist.sh` (DOS-340 §5.8) to forbid any direct read of `maintenance_audit` table rows from `services::claim_receipt::*` (the privacy module is allowed to **write** assertions but never to **surface** raw rows). ANY call site that touches `maintenance_audit` rows must be either (a) audit-management commands allowlisted in DOS-340 §5.8 lint, OR (b) routed through `build_receipt_for_audience` with a non-OperationalAuditStorage audience. Negative-fixture test: a deliberately-attempted disclosure path that the lint catches.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:973:- **AC-341.12** — (cycle-1 CSO F12 — AgentMcp audience explicit row) Privacy matrix explicit `AgentMcp` row added. Field allowlist for AgentMcp: trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary (sanitizer per ADR-0108 §3), `subject_type` (NOT `subject_id`). Forbidden: source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak). AgentMcp snapshot fixtures in `claim_receipt/privacy/tests/agent_mcp_audience` — golden files per sensitivity. CI lint catches any field bleed.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1023:  pub upcoming_meetings: Paginated<MeetingBriefRef>,    // per cycle-1 architecture F5 + AC-507.10
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1032:/// Composed struct (NOT flat enum) per cycle-1 correctness F3 — real briefings have
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1064:  HasCorrections { superseded_claim_ids: Vec<String> },
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1081:3. *Signals + invalidation:* Re-renders on prep-status signals (DOS-335 §5.5) + claim-lifecycle signals + candidate-set invalidation (DOS-278 path).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1091:- **AC-507.4** — (cycle-1 correctness F3) Output fixture covers the **composed-state matrix**, NOT a single flat-enum value per fixture. Each fixture asserts the 4-tuple `(availability, freshness, integrity, advisories)` independently:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1097:  - `Available + Fresh + HasCorrections { superseded_claim_ids: [...] } + []`
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1105:- **AC-507.10** — (cycle-1 architecture F5) `upcoming_meetings: Paginated<MeetingBriefRef>` (NOT plain `Vec`). Consumer pagination tested via re-invocation; cursor is opaque server-signed; CursorState policy (`Stable | DataShifted | Invalidated`) per AC-459.9.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1130:**Reinvention rule:** if a primitive in this table covers a need, W1 consumes it. New substrate is introduced ONLY in §5.1 (envelope ability), §5.5 (prep status service), §5.10 (daily briefing ability) — and each of those is justified at the L0 reuse audit by the absence of an existing producer.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1136:- **AC-W1.1** — No W2 surface ships without its W1 producer landed on `dev`. The dependency map is (per cycle-1 codex-challenge F4 — DOS-461 harness gates Account / Project / Person equally):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1150:- **AC-W1.7** — Mandatory retro at W1 close; `/ce-compound` runs on K-out candidates surfaced during the wave.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1151:- **AC-W1.8** — Proof bundle at `.docs/plans/v1.4.4-wp-surface-migration/proof-bundles/W1-substrate-gaps.md` with reuse audit + Intelligence Loop check transcripts per sub-ticket.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1152:- **AC-W1.9** — (cycle-1 codex-challenge F1 — consumer-skeleton CI gate) `scripts/check_w1_consumer_skeleton.sh` (modeled on `check_claim_writer_allowlist.sh`): for each W1-shipped producer (`abilities-runtime/src/abilities/get_entity_intelligence/`, `get_daily_briefing/`, `services/meeting_prep_status/{read,write}.rs`, `services/entity_intelligence/touchpoints.rs`, filled placeholders in `services/claim_receipt/{boundary,contradiction,feedback,privacy,render_rules}.rs`), assert at least one block render PHP entry point under `wp/dailyos/blocks/**/render-functions.php` invokes it via the abilities runtime handle. Substrate-only PRs without at least one downstream consumer reference fail CI. The script IS the mechanical enforcement for AC-W1.2's "wiring IS the work" obligation.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1153:- **AC-W1.10** — (cycle-1 correctness G2) TS mirror parity test template: `src/services/claim-receipt/contracts.ts` is the canonical template; each sub-ticket's TS mirror cites this pattern rather than inventing its own.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1154:- **AC-W1.11** — (cycle-1 correctness G3) Trust-band recompute path under feedback (DOS-8) explicitly cited at L1: name the substrate path (likely `services::trust::*` or signals on `verification_state` change) before wiring `record_claim_feedback`, so receipts don't go stale-by-one-feedback.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1166:| Report substrate (typed report ability outputs, source contract, generation lifecycle) | Different version's mission | v1.4.8 — Reports as Shareable Intelligence |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1176:Per CLAUDE.md "Parallel-wave migration slot reservations" + memory `feedback_l0_reconcile_against_dev`: W1 declares its block upfront. **Cross-version coordination table on `.docs/plans/v1.4.6-waves.md` §327 + §414** explicitly states "v1.4.4 holds v240–v249" — V1.0's v250–v269 claim collided with v1.4.6's v260–v279 block (cycle-1 architecture F1). V1.1 reclaims v240–v249 per the coordination table; v1.4.6 §327 stays canonical.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1197:3. **Claim receipts emit signals that propagate via existing invalidation paths** (ADR-0125 + signals substrate). W1 consumes existing `ClaimVerificationStateChanged` (per AC-339.6), `entity-link-changed`, `source-asof-updated`, `contradiction-detected`, `correction-recorded` signals. **One new signal** introduced by W1 (per cycle-1 architecture F3 fold): `MeetingPrepStatusChanged` for DOS-335 dismissals + status transitions, pre-declared in `signals/policy_registry.rs` at W1 kickoff (AC-335.15). `signal_policy = { emits_on_output_change = [], coalesce = false }` on new Read abilities matches `get_daily_readiness` pattern.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1201:5. **Audit trail lives in Linear** (memory `feedback_linear_is_the_audit_trail`). Reviewer verdicts, L6 decisions, K-out runs, retro outcomes all land as Linear comments. Git keeps wave-scoped artifacts (proof bundles, retros, ADRs); operational logs live in the local audit storage and are NEVER surfaced as a product surface.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1221:| `/codex challenge` | Adversarial review — catches L0-class blockers before L1 burn | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1222:| `/codex consult` | Substrate-grep verification + ADR consumption sanity check | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1223:| `ce-architecture-strategist` | Producer/projection/renderer split + signals propagation correctness | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1224:| `ce-correctness-reviewer` | Test plans + fixture coverage + no-bypass harness shape (DOS-461) | `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md` |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1244:**Pass rule:** Unanimous APPROVE. Cycle-2 dispatch only if any reviewer returns BLOCKED or substantive CONDITIONAL. Per memory `feedback_review_loop_l6_policy`: continue looping on architectural/critical/high; expand scope on edge-case classes; L6 at 15 cycles or genuine decision. Per memory `feedback_l0_partial_convergence_when_class_recurs`: same-class recurrence across cycles + downstream-plan-impl-not-substrate-spec → accept path-α + transfer residuals to downstream plan AC.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1250:| `/codex challenge` | APPROVE WITH CHANGES (1 HIGH + 3 LOW) | All 4 findings folded into V1.1 (AC-W1.9 + LOC re-check + §5.8 Q5 rewrite + AC-W1.1 fix); awaiting cycle-1 reviewer confirmation that fold lifts approval to clean APPROVE. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1256:**Cycle-2 dispatch:** **Not required.** All 23 findings folded into V1.1. Re-dispatch only if any cycle-1 reviewer disputes a specific fix surface.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1290:- `src-tauri/scripts/check_claim_writer_allowlist.sh` — pattern source for §5.8 audit-disclosure CI lint
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1293:- `feedback_check_substrate_before_authoring_primitives` — applied throughout §6 substrate-consumed audit
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1312:**V1.1 resolutions (cycle-1 fold):**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1314:1. **Envelope composition with `build_intelligence_context()`.** **[RESOLVED V1.1 — coexist + path-α maintenance ticket.]** Per cycle-1 architecture F7 (path-α candidate). V1.0 recommendation accepted: coexist in v1.4.4; v1.5.x consolidation ticket FILED in DailyOS Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) at W1 retro, not deferred. No drift-detection AC needed for V1.0 lifetime; future merge of the two paths handled via the maintenance ticket.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1316:2. **What makes a claim_receipt "user-presentable" vs "operational" — DOS-340 §5.8 boundary.** **[RESOLVED V1.1 — allowlist-primary + CI lint extensibility gate.]** Per cycle-1 CSO F2: receipt allowlist (`RECEIPT_ALLOWED_FIELDS`) is **primary**; denylist (`AUDIT_ONLY_DENYLIST`) becomes redundant CI lint. AC-477.12 codifies; AC-340.7 adds `check_audit_denylist_completeness.sh` extensibility check (cycle-1 correctness F9). New columns on operational audit schema MUST update the denylist OR carry `// receipt-safe: <reason>` comment.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1318:3. **`get_daily_briefing` consumes existing prep DTO or extends it — DOS-335/DOS-507 coupling.** **[RESOLVED V1.1 — Read-only; §5.5 read/write split statically enforces.]** Per cycle-1 architecture F2: `services::meeting_prep_status::read` is pure read (call-graph-safe for `get_daily_briefing`); writers live in `::write`. AC-335.12 adds trybuild/call-graph lint asserting `compute_status` graph contains zero mutations. Viewed-tracking, if needed, lives in a sibling W2/W3 service mutation path — not in the briefing ability.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1322:5. **Trust-boundary hardening — DOS-477 §5.4: Tauri channel allowlist vs per-ability sensitivity gate.** **[RESOLVED V1.1 — BOTH; symbol names corrected.]** Sensitivity gate is the substrate (shipped at `render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)` — symbols corrected per cycle-1 CSO F1; V1.0's `render_policy_for` was wrong). IPC channel allowlist is W2+ surface concern (block render PHP enumerates allowed ability calls per surface). W1 lands the substrate composition (AC-477.11 + AC-477.12); W2 enumerates per surface.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1324:6. **`get_entity_intelligence` envelope vs `get_entity_context` entry-list compatibility — DOS-459 §5.1.** **[RESOLVED V1.1 — coexist + v1.5.x consolidation maintenance ticket filed at W1 retro.]** Same path-α resolution as Q1; see cycle-1 architecture F7. Leave clean (no deprecation noise in v1.4.4).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1326:7. **Meeting prep status DTO — DOS-335 §5.5 — concurrent writer ordering.** **[RESOLVED V1.1 — per cycle-1 correctness F1.]** V1.0's "writes commute on disjoint columns" recommendation was partially wrong: `record_user_authored` emits Decision/Commitment **claims**, which feed the v241 indexed view through the claim store and break the disjoint-column premise. **V1.1 two-part contract** (AC-335.13):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1328:   - **Part B:** Decision/commitment claim emission is a claim-store write, NOT disjoint-column. Status recompute observes the new claim via the standard claim-lifecycle-signal → invalidation path. Contract is **eventual consistency via existing signal substrate**, not commutativity.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1330:8. **DOS-461 no-bypass harness — what counts as "the source of rendered claim-backed intelligence."** **[RESOLVED V1.1 — three-mode assertion per cycle-1 correctness F2.]** AC-461.6a (text-extraction binding check): every claim-substantive string traces to `[data-claim-id]` ancestor. AC-461.6b (stale vs bypass distinction): unresolved `data-claim-id` renders as **stale**, NOT bypass; distinguished failure mode with `account_claim_retracted_mid_render.json` fixture. AC-461.5b adds the per-subject expected-fixture matrix (Account/Project/Person × fixture-class) so "comprehensive coverage" is mechanically provable.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1332:9. **DOS-507 `BriefingState::NeedsPreparation` — does it auto-enqueue prep, or strictly return the state?** **[RESOLVED V1.1 — Read-only.]** Per ADR-0102 §3 Read-ability call-graph (cycle-1 architecture F2). `get_daily_briefing` is pure read. The W3 block renderer (or a sibling write-ability `enqueue_briefing_prep`) handles the enqueue separately. AC-507.3 codifies. The §5.5 read/write split (AC-335.12) statically enforces no-mutation in the briefing call graph.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1334:11. **List-envelope cursor design.** **[RESOLVED V1.1 — opaque server-signed cursor + typed `CursorState`.]** Per cycle-1 correctness F4: cursor is opaque server-signed (survives schema changes, prevents tampering). Pagination behavior under concurrent writes is **named explicitly**: every list-shape envelope returns `Paginated<T> { items, next_cursor, cursor_state: CursorState }` where `CursorState = Stable | DataShifted { advisory } | Invalidated { reason, restart_required: true }`. Concurrent insert/retract tested per AC-459.9. **CSO question:** since cursor is pagination state (not sensitivity state), DOS-459 does NOT need `/cso`. Residual key-management concern (where the cursor-signing key lives, rotation policy) is correctness R1 — flag to v1.4.6/v1.4.7 if cursor signing key becomes a security-token concern, not a substrate change for W1.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1336:10. **W1 sub-lane sequencing within W1.** **[RESOLVED V1.1 — per cycle-1 architecture F6.]** §5.4 (DOS-477) moves from Stage 1b → Stage 1a because §5.7 (DOS-8) calls `validate_envelope_target` as the first step of `submit_claim_feedback`. Boundary primitive must exist before consumers wire it.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1343:    Migration vs contract-freeze boundary: §5.5 ships migrations v241+v242 in Stage 1a; DTO contract is frozen at Stage 1a close. §5.10 (Stage 1c) consumes the contract, not the migrations directly.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:1347:**End of L0 Packet — v1.4.4 W1 Substrate Gaps V1.1.** Cycle-1 fold complete. V1.0 BLOCKED → V1.1 lifts blockers. 23 cycle-1 findings folded inline; 2 path-α findings filed to maintenance project. No cycle-2 dispatch required.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:1:# WP-skill-grounded reviewer — W2 Entity Surfaces L0 cycle 2
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:7:## VERDICT: **APPROVE** (all 4 cycle-1 HIGH findings resolved; 1 net-new LOW informational; path-α items already routed to maintenance per V1.1 fold)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:9:V1.1 lands all 4 cycle-1 packet edits cleanly and tightens the contract at the right resolution. Sub-wave is convergent — no new HIGH, no class-pattern recurrence. Ready for L0 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:21:§5.1 lines 223-232 names the contract explicitly: (1) outer invokes `get_entity_intelligence` ONCE via 3-arg `invoke_ability`; (2) DOS-477 envelope cache (`services::entity_intelligence::auth::envelope_cache`) keys on `(envelope_render_id, actor_principal_id, surface)` where `envelope_render_id` is the deterministic hash of `(entity_type, entity_id, depth, sections, watermark)`; (3) outer writes `envelope_render_id` to context, inner blocks `usesContext: ["dailyos/envelopeHandle"]` and short-circuit to cached envelope; (4) per-request scope, evicts on response close. AC-462.4 + AC #W2.4 enforce 3-arg `invoke_ability` via `check_w1_consumer_skeleton.sh`. Inner block code-shape sketch at line 289-298 demonstrates the resolve pattern. Contract is now testable, not prose-only.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:38:**Finding:** §10 invariant still says "render.php calls `do_blocks($content)` for inner-block reordering." The Stage 1b skeleton at `00f38b3b` actually emits `'<div class="dailyos-inner-blocks-slot">' . $content . '</div>'` — the block parser pre-renders `$content` before render.php receives it; calling `do_blocks($content)` would double-render. Cycle-1 flagged this as L1 informational; V1.1 didn't sweep the wave-level invariant. Not a blocker — the §5.1–5.4 code sketches don't actually call `do_blocks()`, so the invariant is contradicted-but-harmless. Recommendation: in the next wave-plan revision, rephrase to "render.php emits the pre-rendered `$content` so user reorders persist." File against maintenance project at L0 close; doesn't block W2 L1.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:44:M1 / M2 / M3 from cycle-1 already routed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per V1.1 §13 Q7 + §10 invariants. Confirmed correctly filed; no further action.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md:54:W2 sub-wave packet is L0-close-ready from the WP-skill perspective. Re-review not needed unless other reviewers request V1.2 edits affecting block.json / render.php / pattern surfaces.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:132:## Conditions for cycle-3 fold
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:141:- **A1.** Note in §5.1 / AC #30 that numeric segments in `theme.json` `settings.custom` keys hyphenate digit-by-digit (`abc123` → `abc-1-2-3`); audit chrome modules' `var(--space-32)` / `var(--space-64)` style references against this rule when generating the alias source list.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:155:Cycle-2 must-fixes (M1, M2) fold into V1.2 in cycle-3. No critical rewrite. No L6 escalation trigger. No path-α threshold reached. No BLOCKED-with-cited-path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:157:This reviewer joins the cycle-1 panel's CONDITIONAL APPROVE pattern. Re-dispatch the panel against V1.2 once M1+M2 are folded; if remaining 3 cycle-1 reviewers (codex challenge, ce-architecture-strategist, codex consult, ce-design-lens-reviewer) likewise resolve, packet converges to unanimous APPROVE and exits L0.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:5:**Wave HEAD at this handoff:** `f9a0e02b` (cycle-3 claim_receipt ability registration merged)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:10:W1 + W2 substrate fully L1-implemented. L2 mid-cycle. L4 still needed before merge. Below is enough detail for a fresh session to pick up without re-discovery.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:16:DOS-335 meeting_prep_status read/write split + signal + state machine + v241/v242 + v244 transactional view, DOS-459 EntityIntelligenceEnvelope Read ability with Paginated<T>+CursorState, DOS-339 claim_receipt fan-out signal bridge + 250ms debounce, DOS-477 entity-intelligence auth boundary (validate_envelope_target with composes graph walk + allowlist-primary), DOS-340 audit boundary, DOS-341 privacy/redaction with build_receipt_for_audience per-audience construction, DOS-460 touchpoints + open-loops, DOS-8 semantic feedback (10 variants), DOS-461 no-bypass fixture harness, DOS-507 get_daily_briefing composed BriefingState.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:25:| **claim_receipt Read ability** (cycle-3 fix) | `8e49c34a` → `f9a0e02b` | WP block invocations of `invoke_ability('claim_receipt', ...)` failed because no ability registered; only Tauri command existed |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:27:### W2 surfaces (full 7 sections) — L1 done, L2 mid-cycle
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:39:### L2 cycle history (W1+W2 integrated diff)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:59:| 6 CI lint scripts (audit allowlist + denylist completeness + sensitivity gate + W1 consumer skeleton + inline style exception + migrations transactional) | ✅ all green |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:60:| cargo test --lib | ✅ 2648 passed / 0 failed (last verified at HEAD `bdade2cc`; needs re-run on `f9a0e02b` for cycle-3 confirmation) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:79:3. **L4 hasn't happened yet.** James was going to L4 after cycle-3 + L4 wiring lands. Once the in-flight agent returns, James does hands-on test.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:98:for s in check_audit_disclosure_allowlist.sh check_audit_denylist_completeness.sh check_sensitivity_gate_composition.sh check_w1_consumer_skeleton.sh check_no_inline_style_exception.sh check_migrations_transactional.sh; do
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:103:### Step 4 — L2 cycle-4 re-verify (only if substantial changes)
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:111:Otherwise treat L2 cycle-3 as the closing cycle (3 of 4 reviewers approved cycle-2; cycle-3 closed codex's one P2).
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:121:After steps 1-5 close:
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:125:4. Verify each section renders, receipts show trust bands (cycle-3 fix), empty-state chips appear where expected, AgentMcp aggregate render OK
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:130:After L4 unanimous (James's call) → squash-or-merge per repo convention → PR closes.
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:137:| DOS-749 | Codebase Maintenance | DOS-8 lifecycle_changed flag for lifecycle-only actions (path-α from L2 cycle-3) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:138:| DOS-750 | Codebase Maintenance | DOS-8 idempotency race atomic reserve-or-write (path-α from L2 cycle-3) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:140:| DOS-753 | Codebase Maintenance | N+1 claim_receipt invocations (W2 L2 cycle-2 path-α) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:141:| DOS-754 | Codebase Maintenance | touchpoints-feed AgentMcp gate body (W2 L2 cycle-2 path-α) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:142:| DOS-755 | Codebase Maintenance | List ability cursor signing (W2 L2 cycle-2 path-α) |
.docs/plans/v1.4.4-wp-surface-migration/HANDOFF-2026-05-21.md:150:- All L2 verdicts: `.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-*-cycle{1,2,3}.md`
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:16:Premise being defended: first-party WP plugin loopback runs as Actor::User parity with React tauri::invoke, skipping HMAC/session/scope/wp_user_id/audit-contract gates. Third-party MCP keeps Actor::McpClient and authorization machinery but drops transport-ceremony — HMAC envelope signing, nonce ledger, keyed-HMAC audit, 10ms timing floor.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:20:1. For WP loopback: any local process running as the same OS user that could intercept 127.0.0.1, confused-deputy if the WP plugin itself is compromised, audit-log integrity if loopback_origin is client-supplied.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:22:2. For MCP rip in section 3e: any local-stdio replay or tamper threat the dropped HMAC+nonce defended. Personal-tier model has SQLCipher + Keychain DB encryption + OS-user file ownership — what residual threat does keyed-HMAC defend that plaintext-in-encrypted-DB does NOT?
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:24:Also challenge: the close+supersede recommendation for PR #347 — does it lose any specific finding from the 6 cycles of L0 that the new model still needs to address?
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:27:2026-05-21T23:37:08.790113Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Auth(TokenRefreshFailed("Server returned error response: invalid_grant: Grant not found"))
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:28:2026-05-21T23:37:10.114094Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Deserialize(Error("data did not match any variant of untagged enum JsonRpcMessage", line: 0, column: 0))
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:39:  Chief Security Officer mode. Infrastructure-first security audit: secrets archaeology,
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:43:  scan, 2/10 bar). Trend tracking across audit runs.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:44:  Use when: "security audit", "threat model", "pentest review", "OWASP", "CSO review".
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:525:The packet is 395 lines, so I’m reading it with line numbers in chunks. I’m also checking the historical PR references it cites so the “close + supersede” part is judged against actual preserved findings, not just the new recommendation.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:533:     3	**Tickets:** DOS-761 (first-party WP loopback as `Actor::User`) + DOS-762 (live readers + observability) + **DOS-168 amended** (MCP v2 substrate at the new shape — transport-ceremony out, authorization machinery in; PR C closes DOS-168).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:534:     4	**Framing:** apply one trust-model reframe consistently to ALL local surfaces. The abilities runtime is a headless substrate. Local consumers (first-party WP plugin via loopback; third-party MCP agents via stdio/in-process) all live on the same kernel-protected machine as the user. The right gates are **authorization** (what scopes does this caller have) and **operational** (rate-limit, audit, sensitivity gating) — NOT **transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor) which was modeled on the wrong premise and propagated forward from v1.4.2 WP pairing → v1.4.7 MCP v2 W1-A.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:536:     6	**Cross-track action:** **hold PR #347 merge** (v1.4.7 W1-A) — its substrate is reshaped by this packet's §3e. PR #355 (DOS-758 `McpHandlerContext`) is **unaffected** — handler context is transport-agnostic and can land independently. DOS-647-C (in-process MCP hosting) becomes simpler under the new model.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:543:    13	The v1.4.1 → v1.4.4 narrative was: build the abilities-runtime substrate so any surface can consume it without re-wiring the world. The premise was correct; the execution **conflated authorization (what scopes a caller has) with transport-ceremony (HMAC signing, nonce ledger, key custody)**. The conflation originated in v1.4.2 WP pairing and **propagated forward** into v1.4.7 MCP v2 W1-A — the waves doc names this explicitly: *"Server-issued `McpClientId` after pairing handshake (mirrors v1.4.2 SurfaceClient pairing pattern); HMAC-SHA256 transport signing for stdio/HTTP MCP transports"*. Six L0 cycles got CSO sign-off on the internal coherence; nobody asked whether transport-ceremony was the right answer for local stdio/loopback at all.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:550:    20	| **Operational** (rate-limit, audit log, confirmation attestation) | Runaway loops, accountability, destructive-op guard | meaningful (audit + confirmation) | **meaningful** | meaningful |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:551:    21	| **Transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor, transport key custody) | In-flight tamper, replay, side-channel on a wire | **n/a** (kernel-protected loopback) | **n/a** (stdio is in-process pipes; in-process hosting per DOS-647-C makes this even more obvious) | meaningful |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:555:    25	**Layer 1 — First-party WP loopback (DOS-761).** WP plugin (`class-dailyos-runtime-client.php:256–278`) ships **16 HTTP headers per request** through 8 validation gates for an invoke from a process running as the same OS user as Tauri. WP plugin is code we ship. The Tauri React renderer skips all 8 gates by going through in-process `tauri::invoke` as `Actor::User`. WP plugin belongs in the same column.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:559:    29	**Layer 3 — Third-party MCP transport-ceremony rip (new ticket).** v1.4.7 W1-A's substrate (PR #347, not yet merged to dev) contains ~645 LOC of transport-ceremony layered on top of ~1100 LOC of authorization machinery. For local stdio (today's primary MCP transport) and in-process hosting (DOS-647-C's goal), the transport-ceremony defends threats that don't exist. Rip the transport-ceremony layer; keep the authorization machinery verbatim. Specifics in §2c and §3e.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:561:    31	**Why all three at once.** They're the same correction applied consistently. Splitting them means the reframe lands in one place and is contradicted by adjacent code in another — exactly the conflation that produced this mess. The 6-cycle L0 on v1.4.7 W1-A was internally coherent but premised on a wrong taxonomy; landing this packet IS the correction.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:563:    33	**Explicitly preserved (load-bearing — see §8):** authorization machinery (per-client scope manifest, per-tool exposure tier, rate-limit, plain audit, conversation handle lifecycle), `Actor::McpClient` as a distinct actor class (third-party agents need scope-distinct attribution even if transport-ceremony goes), v1.4.2 W3-C WP-mediated MCP path, the pairing handshake itself, PR #355's `McpHandlerContext` interface (transport-agnostic), connector OAuth (Slack/Gmail external calls).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:575:    45	| 3 | HMAC signature verify | `surface_runtime/mod.rs:1109` | Verifies `X-DailyOS-Signature` over canonical request | Tamper of in-flight loopback traffic (kernel-protected) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:576:    46	| 4 | `validate_signed_session_readonly` | `surface_runtime/mod.rs:1154` (impl `services/surface_pairing.rs`) | Session id ↔ wp_user_id ↔ site-nonce binding | Cross-tenant session replay (single-tenant local) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:577:    47	| 5 | `validate_session_bound_wp_user_id_for_request` | `surface_runtime/mod.rs:1969` | Body/query/header wp_user_id matches session-bound | Inconsistent identity injection (no second identity exists) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:578:    48	| 6 | `SurfaceClientBridge::authorize` (scope + cap) | `surface_runtime/mod.rs:2123` | Granted scope contains requested ability; allowed_actors includes SurfaceClient | Privilege escalation (we're already Actor::User in React) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:580:    50	| 8 | Audit-emit contract: `Actor::SurfaceClient` ⇒ `wp_user_id = Some(_)` | `src-tauri/src/audit_log.rs:104,141` | Refuses to emit audit if SurfaceClient actor lacks wp_user_id | Audit accountability (compromised by collapsing to Actor::User — see §3) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:582:    52	**Materialization.** `registry_actor = validated.actor.clone()` (`mod.rs:2152`) wraps as `Actor::SurfaceClient { surface_client_id }`. The React path constructs `Actor::User` directly via `ServiceContext::new_live().with_actor(...)` in `services/context.rs:139`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:617:    87	Branch `v1.4.7-w1-foundation` (PR #347, not merged to dev). Total ~3576 LOC across 7 files + 4 migrations. Mapped by the structural survey:
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:621:    91	| `services/mcp_v2/auth.rs` | 566 | ~252 (pair_client, load_client_record, resolve_tool_grant, revoke_*, exposure helpers) | ~310 (verify_transport_hmac:232–244, verify_and_consume_and_preissue:257–315, transport key persist/load:399–474) | Partial — keychain custody is isolated; nonce consume/preissue is coupled to fail-closed contract; HMAC verify is called from gateway.rs:281 |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:622:    92	| `services/mcp_v2/transport.rs` | 464 | ~370 (list_tools:157–188, build_input_schema, tool_from_description, unwrap_response, tool_error_to_mcp_error, tests) | ~70 (transport_key field:69, next_nonce field:71–72, call_tool nonce state machine:200–209+260, HMAC signing block:218–229) | Partial — nonce state machine interleaved with rmcp dispatch; envelope-shape stays, nonce population goes |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:623:    93	| `services/mcp_v2/gateway.rs` | 883 | ~820 (handler registration, dispatch flow:303–528, scope subset checks, rate-limit reserve:715–786, audit/signal emission) | ~65 (Gate 0a HMAC verify:278–284, Gate 0b nonce consume+preissue:286–297, TIMING_FLOOR_MILLIS:33 + sleep_until_floor:795–801, preissued nonce threading across error paths:291–302+320–353+437–438) | Partial — nonce preissue threading touches ~20 lines across 5 error paths; non-local edit |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:624:    94	| `services/mcp_v2/audit.rs` | 344 | ~144 (write API:50–78, sanitize_detail:111–124, add_actor_attribution:126–144, insert_outbox:181–220) | ~200 (hmac_json:146–150, canonical_value:152–168, audit key keychain custody:239–343, AUDIT_KEY_* constants:22–26) | Partial — keychain bits clean; hashing (lines 91–95) woven through `detail_with_hashes()` and all call sites; rip requires switching to plaintext emission OR field-level masking |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:627:    97	| `services/mcp_v2/contracts.rs` | 851 | ~800 (ToolDescription, Scope, ScopedName, ParamSpec, ReturnSpec, OpaqueConversationHandle, McpClientId, McpActor, envelope shapes) | ~50 (OpaqueNonce type:223–241 — keep type for safety, drop usage; request_nonce/next_request_nonce fields on envelopes) | Clean (type-level) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:630:   100	| `migrations/257_mcp_transport_nonce_ledger.sql` | 17 | 0 | 17 | **DROP entirely** |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:631:   101	| `migrations/258_mcp_rate_limit_and_audit_outbox.sql` | 18 | 18 | 0 | KEEP (rate-limit ledger + audit outbox tables both load-bearing for ops layer) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:633:   103	**Net rip estimate:** ~645 LOC removed, ~30 LOC modified (envelope shapes drop nonce fields), ~2900 LOC untouched. Migration 257 dropped before merge (replace with a no-op or renumber the rip-out into a new migration if 257 ships first).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:643:   113	### 2e — Reuse audit (per `feedback_l0_reconcile_against_dev`)
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:645:   115	Fork SHA: `852a0118`. `git diff 852a0118..dev -- src-tauri/src/services/context.rs src-tauri/src/surface_runtime/ src-tauri/src/audit_log.rs src-tauri/src/bridges/types.rs wp/dailyos/includes/transport/` returns **no changes** — dev has not advanced on any surface touched by this packet since the fork. Safe to land against `dev` after PR #351 merges. If `dev` advances before this packet's PR opens, re-run reconcile.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:658:   128	2. Materializes `Actor::User` directly (same as React `tauri::invoke`).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:660:   130	4. Calls `TauriAbilityBridge::invoke` (the React path's entrypoint) with a new `BridgeSurface::LocalLoopback` variant (decision in §5.Q1; recommended new variant for audit clarity).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:662:   132	6. Audit emission: emits with `Actor::User`, required `loopback_origin: "wp_plugin"` field (today's only consumer; enum extensible). No `wp_user_id` contract — there is no wp_user.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:666:   136	**Removed for `/v1/local/invoke` path only.** HMAC verify, session validate, wp_user_id-per-request, SurfaceClient bridge authorize, per-axis rate-limit, audit-emit `wp_user_id` contract (because actor isn't SurfaceClient).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:669:   139	- `/v1/surface/invoke` (the existing signed-transport route) stays fully wired. v1.4.7's MCP v2 substrate (PR #347) uses it; DOS-647-C in-process MCP hosting will use it; if the WP-mediated MCP path (v1.4.2 W3-C) reaches into it, that stays too.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:670:   140	- `services/mcp_v2/*` — gateway, auth, audit, actor_policy, taxonomy, transport, contracts. None of the v1.4.7 substrate moves in this packet.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:673:   141	- `Actor::McpClient` and the `McpClientId`-bound scope manifest, nonce ledger, keyed-HMAC audit. Third-party agent ceremony is correct and stays.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:716:   184	4. Add `/v1/local/invoke` route + `Actor::User` materialization for loopback + new `BridgeSurface::LocalLoopback` variant.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:721:   189	7. Hold PR #347 merge OR rebase PR #347 over PR C.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:722:   190	8. Apply the §3e rip: delete migration 257, delete ~310 LOC from auth.rs, ~70 LOC from transport.rs, ~65 LOC from gateway.rs, ~200 LOC from audit.rs. Modify ~30 LOC across envelope shapes.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:734:   202	- `src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql` — entire schema gone. If 257 ships in dev before PR C lands, replace with a `DROP TABLE IF EXISTS mcp_transport_nonce_ledger;` rip-out migration at the next slot.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:738:   206	- `services/mcp_v2/auth.rs:257–315` — `verify_and_consume_and_preissue()` fail-closed nonce ledger.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:742:   210	- `services/mcp_v2/transport.rs:71–72` — `next_nonce: Arc<Mutex<Option<OpaqueNonce>>>` field.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:743:   211	- `services/mcp_v2/transport.rs:200–209,260` — nonce pull-from-state and store-next-nonce.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:744:   212	- `services/mcp_v2/transport.rs:218–229` — HMAC envelope signing block.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:746:   214	- `services/mcp_v2/gateway.rs:278–284` — Gate 0a HMAC verify call.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:747:   215	- `services/mcp_v2/gateway.rs:286–297` — Gate 0b nonce consume+preissue call.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:749:   217	- `services/mcp_v2/audit.rs:22–26` — `AUDIT_KEY_*` constants.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:750:   218	- `services/mcp_v2/audit.rs:146–150` — `hmac_json()`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:751:   219	- `services/mcp_v2/audit.rs:152–168` — `canonical_value()` (canonical JSON for HMAC).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:752:   220	- `services/mcp_v2/audit.rs:239–343` — audit key Keychain custody (`load_or_create_audit_key`, `read_audit_key`, `persist_audit_key`, etc).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:756:   224	- `services/mcp_v2/contracts.rs` — `McpToolRequestEnvelope.request_nonce` field becomes `Option<OpaqueNonce>` or removed. `McpToolResponseEnvelope.next_request_nonce` similarly.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:757:   225	- `services/mcp_v2/audit.rs:80–100` — `detail_with_hashes()` becomes `detail_with_attribution()`. Emit params/response **plaintext** in the detail (no HMAC hashing) OR switch to field-level masking via the existing `sanitize_detail()` (lines 111–124) — decision in §5.Q9.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:758:   226	- `services/mcp_v2/gateway.rs:291–302,320–353,437–438` — unthread `preissued` from rejection-response building. Response envelope no longer carries `next_request_nonce`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:762:   230	- Entire authorization machinery: pair_client, manifest load, scope resolution, exposure-tier check, conversation handle lifecycle, scope-subset enforcement, rate-limit reserve, plain audit emission, audit outbox, `actor_policy.rs`, `taxonomy.rs`, `contracts.rs` type ecosystem (minus nonce/HMAC bits).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:763:   231	- `Actor::McpClient` as a distinct actor class — third-party agents need scope-distinct attribution from `Actor::User`, even if they don't sign envelopes.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:767:   235	**New invariant.** Audit log row format becomes: `{ actor: McpClient(client_id), conversation_handle, ability, params: <plaintext or masked>, response: <plaintext or masked>, request_id, timestamp }`. No HMAC hashes. Tamper-evidence is delegated to the OS — the SQLCipher DB the user controls is the audit substrate.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:769:   237	**What CSO must re-pass on.** §6.5 reviewer prompt is rewritten to bound the CSO question precisely: "the personal-tier trust model puts the user in control of their own DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with SQLCipher + Keychain DB encryption, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't?"
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:785:   253	- [ ] Handler materializes `Actor::User` and calls `TauriAbilityBridge::invoke` with the canonical path. No HMAC, session, scope, or wp_user_id validation in the loopback handler.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:786:   254	- [ ] Audit log emits an entry per loopback invoke with `Actor::User` and optional `loopback_origin` for traceability. No SurfaceClient/wp_user_id contract.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:794:   262	- [ ] `services/mcp_v2/auth.rs` shrinks by ~310 LOC: HMAC verify, nonce ledger, transport key custody gone. Manifest load + scope resolution + handle lifecycle intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:795:   263	- [ ] `services/mcp_v2/transport.rs` shrinks by ~70 LOC: nonce state machine + HMAC signing gone. `list_tools`, `tool_error_to_mcp_error`, dispatch flow intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:796:   264	- [ ] `services/mcp_v2/gateway.rs` shrinks by ~65 LOC: Gate 0a, Gate 0b, timing floor, preissue threading gone. Dispatch + scope subset + rate-limit + audit emission intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:797:   265	- [ ] `services/mcp_v2/audit.rs` shrinks by ~200 LOC: HMAC hashing + audit key custody gone. Emission + outbox + actor attribution intact. Detail emits plaintext-in-encrypted-DB OR field-masked (per §5.Q9 decision).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:798:   266	- [ ] `Actor::McpClient` retained; scope manifest enforcement intact end-to-end.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:799:   267	- [ ] Local MCP integration smoke test: a stdio MCP client (real or mock) connects, sees `tools/list` filtered by manifest grants, invokes a read tool, receives result. No signing, no nonces.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:802:   270	- [ ] DOS-624 sensitivity gating still composes correctly downstream of `Actor::McpClient` materialization.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md:808:   276	- [ ] PR #347 merge held until PR C reshapes the substrate; or PR #347 author rebases over PR C; coordination decision in §5.Q8.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md:42:   884	| **Substrate-in-same-wave (C4)** | If L0 review surfaces a substrate gap blocking a named W2 AC, W1 reopens — NOT deferred to v1.4.5+. C4 supersedes path-α. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md:43:   885	| **AgentMcp audience filter** | Every claim-bearing inner block routes receipt through `build_receipt_for_audience(target, audience, conn)` (W1 DOS-341); AgentMcp audience row's allowlist (trust band, coarsened freshness, redaction level, lifecycle, sanitized evidence_summary, subject_type) enforced. Negative fixture per W1 AC-341.12. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md:48:   890	| **Code-shape sketch obligation (NEW V1.1, per architecture A1 + wp-skill H1)** | Every block.json declaration named in §5 ships with a concrete code-shape sketch — block.json + render.php skeleton — alongside the prose. No spec-named-in-prose-only patterns. If a third spec-vs-sketch gap surfaces in cycle 2, switch to a class-wide sweep over §5. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md:60:   116	    /// → persists a typed proposal row only; no trust/lifecycle change on
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md:72:   128	### 2. `ClaimFeedback` row shape (closed)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:13:Path: **fold conditions into V1.1, no cycle-2 dispatch**. All conditions are corrections inside the packet's current scope.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:34:2. **Decision/commitment claim emission** is a claim-store write, NOT a disjoint-column write. Status recompute observes the new claim through the standard claim-lifecycle-signal → invalidation path. The contract is "eventual consistency via existing signal substrate," not "commutes."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:54:**Edge case B — claim_id present but stale.** If block render PHP caches a prior envelope's DOM and the cached `data-claim-id` no longer resolves to the live envelope (claim was retracted/superseded between renders), the harness flags "unresolved data-claim-id = bypass" — but the actual problem is stale render, not bypass. The harness should distinguish.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:60:**Severity rationale:** Without these the no-bypass harness has gaps that let two real regression classes through. Trace confidence ~75 (the rendering paths exist as patterns in `wp/dailyos/blocks/account-overview/view.js:503` which already uses `data-claim-id` extraction — the harness must close the gap).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:73:- Day has 3 meetings, prep is ready for all 3 but **one references a corrected-superseded claim** → `Corrected { superseded_claim_ids: [...] }` overrides `Full`?
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:157:"Fixtures include stale fact, corrected-superseded claim, low-trust claim, **metadata proposal where supported**, open loop, upcoming touchpoint, recent touchpoint, thread summary, confidential/user-only claim, Glean citation, wrong-subject claim."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:204:### F9 — LOW — DOS-340 audit-only denylist is a static const list, missing extensibility check
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:208:**Trace:** Hardcoded list. When a new audit field is added in v1.5.x, the denylist won't include it; receipts could expose the new field by default. The packet's recommended structural rule (§13 Q2) was correctly rejected ("enum-based + CI lint enforces; structural rule is a maintenance pattern, not a substrate pattern") — but the **CI lint** for adding-an-audit-field-without-denylist-update isn't specified.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:210:**Recommended fix:** Add AC-340.7: "CI lint `check_audit_denylist_completeness.sh` enforces that every column added to the operational audit storage schema (any migration touching `audit_log` or `provenance_audit_storage`) explicitly adds an entry to `AUDIT_ONLY_DENYLIST` OR includes a comment justifying receipt-safety (`// receipt-safe: <reason>`)." Pattern modeled on existing `check_claim_writer_allowlist.sh`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:237:- **R3 — `get_entity_intelligence` envelope re-renders on existing signals (§5.1 IL check item 3) — no new signals.** If the existing signal substrate doesn't fire on `claim_state` transitions that go `Active → Contested` (which it should, but isn't verified in the packet), envelopes will silently go stale. Recommend AC-459.10: "Signal-driven envelope refresh tested: trigger every claim-lifecycle transition + verify envelope re-renders within signal propagation budget."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md:243:- **G3 — Trust-band recompute path under feedback (DOS-8 §5.7 IL check item 2) — "feedback updates lifecycle + verification state which feeds trust-band recompute via existing substrate" — the existing trust-recompute trigger isn't named.** Cite the substrate path (likely `services::trust::*` or signals on `verification_state` change). Without the path named, an L1 implementer may wire `record_claim_feedback` but miss the trust-band-refresh hookup, producing receipts where trust band is stale-by-one-feedback.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:11:Branch base: `wave/v1.4.4-w1-stage1a` at HEAD `c5c0578f` (`Merge worktree-agent-a5e003b64a9524cf8 (W1 Meeting EntityKind extension for W2 §5.4) into wave/v1.4.4-w1-stage1a`; updated V1.1 from stale `deb682b0` per codex-challenge F6). W1 substrate fully landed including the Meeting EntityKind extension at `87df7cf6` that closes V1.0 F2 (Meeting). New W2 work branches from this base as `wave/v1.4.4-w2-entity-surfaces` after L0 close.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:22:- [DOS-328](https://linear.app/a8c/issue/DOS-328) — Metadata proposals interaction (Urgent; proposal lifecycle on the entity-detail composites).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:38:| Claim receipt fanout (DOS-339 wire-up + DOS-701 carve-out) | `src-tauri/src/services/claim_receipt/{mod,contracts,render,auth}.rs` | `0243df65` (PR #323 base) + `0a586218` (cycle-2 wiring) | Per-claim receipt rendering in every inner block (§5.1–5.4 + §5.7) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:39:| Receipt vs operational audit boundary (DOS-340) | `services::claim_receipt::boundary` + `scripts/check_audit_disclosure_allowlist.sh` | `e74d49dc` | Receipt rendering boundary (§5.1–5.4) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:42:| Entity fixture harness + no-bypass checks (DOS-461) | `tests/entity_intelligence_no_bypass/` | landed in W1 cycle-2 verdict cluster (verify at L1) | Gates every W2 AC (§7) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:44:| Meeting `EntityKind` extension for `get_entity_intelligence` (V1.1 fold) | `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/{contracts,producer}.rs` + `services::entity_intelligence::auth` | `87df7cf6` (merged at `c5c0578f`) | Meeting Detail block (§5.4) — closes V1.0 codex-challenge F2 |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:47:| L2 cycle-3 section-list + source_type redaction patches | n/a | `1a56d612` | Adopted as-is via the renderer projection. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:48:| L3 cycle-2 WP block consumer skeletons | `wp/dailyos/blocks/account-detail/render-functions.php` + sibling person/project skeletons | `00f38b3b` + `3e58bc13` | W2 specializes these skeletons into typed inner-block compositions. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:49:| CI gate `check_w1_consumer_skeleton.sh` | `src-tauri/scripts/check_w1_consumer_skeleton.sh` | `c2e857b8` (extended through W1 cycle-2) | Inherited as a passing gate for the W2 PR (§7 AC #W2.10). |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:73:- **V1.2 (2026-05-21):** Cycle-2 reviewer fold — codex-challenge cycle-2 surfaced 6 findings; all addressed without scope expansion. **F1 (CRITICAL carryover) — Account/Project inner-block count mismatch:** Account count 22→24 + `dailyos/file-list` inner block added to §5.1 template, projection table, and AC-462.2; Project count 14→15 + `dailyos/linear-issues-chapter` added to §5.2 template + projection table + AC-483.2 (1-to-1 with `src/pages/ProjectDetailEditorial.tsx:526` `LinearIssuesChapter`); scope-summary §4 inner-block lists realigned. **F3 (HIGH carryover) — DOS-725 inline-style CI regex too permissive:** tightened from `^--[a-z-]+:\s*var\(--[a-z-]+\);?$` to `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` — `--dailyos-*` namespace ONLY, arbitrary `--*` prefixes now rejected by `check_no_inline_style_exception.sh`. Updated in §5.2 prose + AC-W2.7 + §10 invariant footnote. **NEW — Template arrays only on Account:** added concrete `template` array sketches to §5.2 (Project, 15-entry), §5.3 (Person, 12-entry), §5.4 (Meeting, 10-entry) block.json sketches, satisfying §10 invariant "code-shape sketch obligation" across all 4 entity-detail composites. **NEW — Residual "Synced pattern" in AC-462.8:** replaced with "Filesystem pattern" + theme-registered semantics per wp-skill H4 V1.1 sweep (final residual H4 hit). **NEW — Empty-state code sketch contradicted §10 invariant:** §5.1 stakeholder-grid render.php sketch upgraded from empty `<div ... data-empty-reason="..."></div>` to a visible `dailyos-empty-chip` span with `data-empty-reason` + localized empty-state label, matching §10 invariant "quiet chip, never silent-hidden". **NEW (CRITICAL) — MergeIntent substrate gap:** V1.1 had filed `FeedbackAction::MergeIntent` as a v1.4.5 candidate variant; per James "every surface in 1.4.4 — no deferrals" mandate (2026-05-21), V1.2 escalates to a parallel W1 substrate amendment (sibling to Meeting `EntityKind` extension `87df7cf6`). Extension SHA carried as `01d0cff3 (merged at 0d82502f)` placeholder pending land; cycle-4 dispatch for §5.3 + §5.5 PRs waits for the extension. §5.3 Q5 disclaimer removed; AC-484.3 updated; §1 substrate-consumed table + §6 reuse map carry the extension as an additive enum variant + new `record_claim_feedback` arm (no other W1 contracts reopened).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:77:## 3. K-in record (substrate-grep audit, 2026-05-21)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:83:Greps run against the full inventory for: `entity-detail`, `account-detail`, `project-detail`, `person-detail`, `meeting-detail`, `gutenberg`, `inner-blocks`, `providescontext`, `usescontext`, `render-functions`, `theme-json`, `cursor`, `pagination`, `merge`, `tint`, `composition`, `envelope`, `agentmcp`, `audience`. Inventory grew from 16 → 24 between V1.0 and V1.1 (W1 close emitted 3 new entries this packet now cites; remaining 5 net-new are unrelated to W2 scope).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:118:- `tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md` — K-out from W1 close (NEW citation, V1.1). Applied to L1 codex-task dispatch: if worker emits `finalAnswerSeen` but no file writes land within 3 min, cancel + re-dispatch with smaller scope. Pairs with `codex-agent-dispatched-but-no-file-changes` as the dispatch-hygiene class for W2 parallel codex fan-out.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:119:- `workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — applied: 12 sub-tickets sized to 1 outer + N inner per entity. If reviewers surface 5+ net-new findings per cycle, scope reset, not fold-and-continue. Reviewer panel reads this as a precondition.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:170:**Wave-roll-up:** ~8,500 LOC across 4 outer entity-detail composites + 3 list shells + 2 metadata-proposal inner blocks + 5 primitive (new or patched) blocks. Bottom-up sized so a 5+ net-new-findings-per-cycle outcome triggers scope reset (per K-in discipline).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:234:4. Cache is per-request scope (lifetime = single PHP request); evicts on response close. DOS-477 cache implementation lives in W1 substrate; this is consumer-side discipline only.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:254:| `dailyos/account-hero` | Facts (identity facts: name, lifecycle, parent/child refs) | n/a — operational shell | No claim binding required at hero; entity-link via `SubjectRef`. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:276:| `dailyos/unified-timeline` | Record (record entries `Paginated<RecordEntry>`) + cross-cut from MetadataProposals lifecycle events | per-entry | Composes Record + lifecycle events. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:326:- AC-462.6: Visible-QA-state matrix from DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / proposal accepted / proposal dismissed / proposal edited / touchpoints present / cite-chip drawer) reachable in the WP block, screenshot-evidenced.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:327:- AC-462.7: DOS-477 trust-boundary hardening satisfied — envelope-set validation gates close.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:336:5. *Feedback loop:* Corrections from `dailyos/metadata-proposal-drawer` (§5.6) and DOS-8 typed feedback affordances on cite-chip drawer (§5.7) route through `services::claims::record_claim_feedback`; envelope re-fetch on next render reflects lifecycle state.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:420:| `dailyos/unified-timeline` | Record (`Paginated<RecordEntry>`) + MetadataProposals lifecycle | per-entry | |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:431:- AC-483.6: Visible-QA-state matrix from DOS-483 (full / empty / stale / `needs_verification` / corrected/superseded / touchpoints present / cite-chip drawer / parent-project portfolio state) reachable + screenshot-evidenced.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:481:**Merge-picker substrate (per W0 audit + §13 Q6):**
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:483:W0 surface-audit flagged "Person merge picker substrate" as **unfiled — needs Linear ticket if W2 includes the merge action in the WP block." This packet RECOMMENDS folding merge-picker scope into DOS-484. Merge today lives in Tauri React (`PersonDetailEditorial.tsx` Dialog + `handleMerge` on `useEntitySuppressions`); WP parity requires a merge ability. Two paths:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:486:- **Path β (W1 reopen):** add `merge_persons` ability to W1 substrate. Per wave AC #W5 C4 supersedes path-α when substrate gap blocks a named AC. Since DOS-484's named AC includes "merge flow… may remain on service-backed person commands," Tauri retention is acceptable; path α applies.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:488:**Locked at L0 close: path α.** If L4 reveals user friction with Tauri-only merge during the W2 build-out window, path β escalates as an in-wave amendment per wave AC #W5 — NOT deferred to v1.4.5+.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:500:| `dailyos/unified-timeline` | Record (`Paginated<RecordEntry>`) + MetadataProposals lifecycle | per-entry | |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:517:**Intelligence Loop check:** identical structure; Q3 inherits uniform answer. Q5 feedback loop adds `FeedbackAction::MergeIntent` as the 10th variant of the typed enum (unit-only enum per ADR-0123 V1.1 §1 implementation note; payload carried via `payload_json.merge_target: SubjectRef` + optional `supporting_evidence: String ≤500 chars`, validated by `validate_and_sanitize_metadata` arm). **V1.2 fold (2026-05-21):** per James "every surface in 1.4.4 — no deferrals" mandate, the v1.4.5 deferral previously noted here is removed. `MergeIntent` lands in a parallel W1 substrate amendment (sibling to the Meeting `EntityKind` extension at `87df7cf6`); the W1 enum extension SHA is `01d0cff3 (merged at 0d82502f)` (cycle-4 dispatch waits for the extension to land before opening W2 L1 PRs that emit `MergeIntent`). Path α holds as the user-facing affordance shape (WP block emits intent feedback; actual merge stays Tauri-side); the substrate extension is purely additive (new enum variant + `record_claim_feedback` arm) and does not reopen any other W1 contract.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:527:W1 substrate extended to support Meeting via commit `87df7cf6` (`feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)`), merged into wave at `c5c0578f`. `EntityKind` is now `{ Account, Project, Person, Meeting }` per `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs`. Meeting subject composes the standard 7 `EnvelopeSection` variants:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:536:AgentMcp audience scrub applies per W1 cycle-2 F2 pattern (no per-attendee identifying data, no raw meeting IDs, etc.).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:666:**List item shape:** `EntityListItem` typed projection from `get_entity_intelligence` with `depth: Shallow` and `sections: [Facts]` (the only `EnvelopeSection` variant that's load-bearing for a list row — name, lifecycle, trust-band-aggregate via `Facts.provenance`). NOT `sections: [Facts, Vitals]` (Vitals is not an `EnvelopeSection` variant — V1.0 invented that name; corrected V1.1 per codex-challenge F5).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:687:**Envelope consumption:** `EntityIntelligenceEnvelope.metadata_proposals: Vec<MetadataProposal>` (W1 DOS-459 envelope slice). Each `MetadataProposal` carries `proposal_id`, `subject_ref`, `field_path`, typed value, `sensitivity`, `lifecycle_state`, `trust_band`, `evidence_summary`, `provenance: ProvenanceRef` per DOS-328 AC.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:712:1. *Claim model:* `MetadataProposal` is a claim subtype per ADR-0125 typed registry; carries lifecycle state + sensitivity.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:714:3. *Signals + invalidation:* Uniform pull-on-render. Lifecycle state change re-renders on next envelope fetch.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:716:5. *Feedback loop:* All 3 actions feed claim lifecycle + source reliability via existing pipelines (`record_claim_feedback` → `update_source_reliability` per ADR-0105).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:741:1. `band_label`, 2. `score_value`, 3. `factor_breakdown`, 4. `evidence_summary`, 5. `citation_list`, 6. `freshness_caveat`, 7. `trust_band`, 8. `lifecycle_state`, 9. `provenance_ref` (actor-filtered), 10. `corrected_text` (sanitized).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:753:**Patch (no new block):** envelope-wiring + keyboard navigation across composites + drawer integration. Covered inline by §5.1–§5.3 specialization work. Tracking ticket gates wave-close per DOS-690 AC.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:803:| `services::claim_receipt::boundary` + audit-disclosure-allowlist CI lint | Receipt rendering boundary across W2 surfaces | `e74d49dc` | Reuse |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:807:| `FeedbackAction::MergeIntent` enum extension | Person Detail merge affordance (§5.3) + People-list merge intent (§5.5) | `01d0cff3 (merged at 0d82502f)` (parallel W1 amendment landing on `wave/v1.4.4-w1-stage1a`) | Reuse (extended W1 substrate per V1.2 codex-challenge cycle-2 NEW finding — removes V1.1 v1.4.5 deferral) |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:808:| DOS-461 fixture harness + no-bypass checks | All §5.1–5.4 ACs gate on green harness | W1 cycle-2 verdict cluster | Reuse |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:809:| L3 cycle-2 WP block consumer skeletons | All 4 outer blocks specialize these | `00f38b3b` + `3e58bc13` | Specialize |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:811:| L2 cycle-3 section-list normalization patches | Section-list rendering in `Composition.sections[]` | `1a56d612` | Reuse |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:834:**AC #W2.7 — Per-project tint via CSS custom property (V1.1 lock; V1.2 regex tightened).** `dailyos/project-detail` outer wrapper carries `--dailyos-project-tint: var(--color-garden-olive);` via `get_block_wrapper_attributes(['style' => ...])`. **Prerequisite for W2 L1 start:** ADR-0077 amendment ticket lands first, locking project tint = `olive`. Until the amendment lands, `chrome_config()` emits olive default (`$stub_tints['dailyos_project'] = 'olive'`). **New CI script `src-tauri/scripts/check_no_inline_style_exception.sh`** (file at L1 kickoff): asserts that any `style=` attribute body in `wp/dailyos/blocks/**/*.php` matches `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` (custom-property assignment only — `--dailyos-*` namespace ONLY; arbitrary `--*` prefixes are rejected, per V1.2 codex-challenge cycle-2 F3 fix). This is the narrow exception to `feedback_no_inline_css` per memory; CI gate enforces the boundary. AtmosphereLayer renders olive tint on `singular(dailyos_project)` and `is_post_type_archive(dailyos_project)` if CPT registered (default-no per §5.2).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:844:**AC #W2.12 — DOS-690 / DOS-691 / DOS-692 / DOS-693 surface-residue tickets close.** Each L4 hands-on validated; closure ledger updated on parent DOS-325 / DOS-9 / DOS-11.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:884:| **Substrate-in-same-wave (C4)** | If L0 review surfaces a substrate gap blocking a named W2 AC, W1 reopens — NOT deferred to v1.4.5+. C4 supersedes path-α. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:885:| **AgentMcp audience filter** | Every claim-bearing inner block routes receipt through `build_receipt_for_audience(target, audience, conn)` (W1 DOS-341); AgentMcp audience row's allowlist (trust band, coarsened freshness, redaction level, lifecycle, sanitized evidence_summary, subject_type) enforced. Negative fixture per W1 AC-341.12. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:890:| **Code-shape sketch obligation (NEW V1.1, per architecture A1 + wp-skill H1)** | Every block.json declaration named in §5 ships with a concrete code-shape sketch — block.json + render.php skeleton — alongside the prose. No spec-named-in-prose-only patterns. If a third spec-vs-sketch gap surfaces in cycle 2, switch to a class-wide sweep over §5. |
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:916:**Pass rule:** unanimous APPROVE. 2 revision cycles without convergence ⇒ L6 escalation. Memory `feedback_review_loop_l6_policy` allows continued looping on architectural / critical / high; class-pattern recurrence triggers system-wide sweep per memory `feedback_systemic_look_for_recurring_issue_classes`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:939:**W0 audit:** `.docs/plans/v1.4.4-surface-audit.md` (active list § with W2 surfaces; W0 open-questions resolved 2026-05-20).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:942:- `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-{architecture,correctness,cso,codex-challenge,codex-consult}-cycle1.md`.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:961:**Resolves at:** L0 close (assume canonical ordering default unless reviewer panel objects).
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:971:**Resolves at:** L0 close.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:981:**W1 substrate extended to support Meeting EntityKind via commit `87df7cf6`** (`feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)`), merged into wave at `c5c0578f`. `EntityKind` now `{ Account, Project, Person, Meeting }`. Meeting subject composes Facts / Health / Touchpoints / OpenLoops / Threads / Record / MetadataProposals as listed in §5.4 V1.1. AgentMcp audience scrub applies per W1 cycle-2 F2 pattern.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:987:No Linear ticket at packet-author time. **Locked recommendation (unchanged V1.1):** file new ticket "v1.4.4 W2 — Meeting Detail block" at L1 kickoff; priority High; project v1.4.4. AC list seeded from §5.4. The substrate gap that previously made this risky has been closed at `87df7cf6`; filing is now a routine L1-kickoff task.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:1005:**Resolves at:** L1 kickoff (ADR-0077 amendment lands first); maintenance tickets filed at L0 close, do not block.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md:1:# WP-skill-grounded reviewer — v1.4.4 wave-level L0 packet (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md:9:The wave-level shape (W0 audit → W1 substrate → W2–W5 surfaces → W6 parity proof + Tauri flag-flip) is sound and the substrate-in-same-wave + many-blocks-not-few invariants are correctly stated. The acknowledged anchored decisions on block granularity (outer + N inner) and cursor pagination are idiomatic Gutenberg. **Conditional** because the packet leaves four WP-specific decisions for sub-L0 packets that need wave-level guard rails BEFORE W2 enters sub-L0 (otherwise each sub-L0 reinvents them inconsistently across Account / Project / Person / Meeting / Briefing). The conditions are recorded as findings 1–4 below; once the packet adopts the recommended wave-level invariants, this reviewer approves.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md:140:AC #W4 says "default ... but editable" — synced patterns are the closer match because they're per-block-insertion defaults, not per-template-region. The packet doesn't name which.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md:161:Findings 1–4 should fold into §10 (invariants) and §13 (open questions / deferred) as wave-level guard rails in cycle 2. Findings 5–7 fold as one-line additions to §6 / §7 / §10. None of the findings expand scope; all narrow WP-specific decisions so the W1–W6 sub-L0 packets inherit consistent answers rather than reinventing them. After cycle 2 edits, expectation is APPROVE.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:13:- **C1 - Amend W4 coupling matrix.** Packet must cite `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` and stop treating W4 Packet F V1.0 as authoritative. Actual W4 gap is "extend existing nonce substrate + wire verify to `record_claim_feedback`", not new `surface_feedback` table/service. Assert disjoint write sets:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:14:  - W4 expected: `src-tauri/src/services/surface_nonce.rs`, `src-tauri/src/bridges/surface_client.rs`, `src-tauri/src/commands/surface_runtime.rs`, `wp/dailyos/includes/class-dailyos-plugin.php`, `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`, `wp/dailyos/blocks/account-overview/*`, tests.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:34:`v1.4.3-waves.md` W4 is a feedback write path: nonce lifecycle, REST endpoint, JS affordance, `record_claim_feedback`, tests (`.docs/plans/v1.4.3-waves.md:203-218`). Its architecture invariant is "feedback writes flow through substrate claim path" (`.docs/plans/v1.4.3-waves.md:77`).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:38:Important correction: W4 V1.0 was already called out as reinvention. `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:16-20` says existing `surface_nonce.rs` plus WP transport already cover the nonce substrate; W4 should extend, not create parallel. This strengthens disjointness because revised W4 shrinks away from new migration/service files.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:96:- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` - W4 V1.0 already blocked for missing existing nonce substrate. Relevant to coupling question.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:107:- `.docs/decisions/0130-surface-independent-composition-contract.md:200-202` - editing semantics and composition lifecycle stay in existing feedback/signal paths.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:3:**Tickets:** DOS-761 (first-party WP loopback as `Actor::User`) + DOS-762 (live readers + observability) + **DOS-168 amended** (MCP v2 substrate at the new shape — transport-ceremony out, authorization machinery in; PR C closes DOS-168).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:4:**Framing:** apply one trust-model reframe consistently to ALL local surfaces. The abilities runtime is a headless substrate. Local consumers (first-party WP plugin via loopback; third-party MCP agents via stdio/in-process) all live on the same kernel-protected machine as the user. The right gates are **authorization** (what scopes does this caller have) and **operational** (rate-limit, audit, sensitivity gating) — NOT **transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor) which was modeled on the wrong premise and propagated forward from v1.4.2 WP pairing → v1.4.7 MCP v2 W1-A.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:6:**Cross-track action:** **hold PR #347 merge** (v1.4.7 W1-A) — its substrate is reshaped by this packet's §3e. PR #355 (DOS-758 `McpHandlerContext`) is **unaffected** — handler context is transport-agnostic and can land independently. DOS-647-C (in-process MCP hosting) becomes simpler under the new model.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:13:The v1.4.1 → v1.4.4 narrative was: build the abilities-runtime substrate so any surface can consume it without re-wiring the world. The premise was correct; the execution **conflated authorization (what scopes a caller has) with transport-ceremony (HMAC signing, nonce ledger, key custody)**. The conflation originated in v1.4.2 WP pairing and **propagated forward** into v1.4.7 MCP v2 W1-A — the waves doc names this explicitly: *"Server-issued `McpClientId` after pairing handshake (mirrors v1.4.2 SurfaceClient pairing pattern); HMAC-SHA256 transport signing for stdio/HTTP MCP transports"*. Six L0 cycles got CSO sign-off on the internal coherence; nobody asked whether transport-ceremony was the right answer for local stdio/loopback at all.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:20:| **Operational** (rate-limit, audit log, confirmation attestation) | Runaway loops, accountability, destructive-op guard | meaningful (audit + confirmation) | **meaningful** | meaningful |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:21:| **Transport-ceremony** (HMAC envelope signing, nonce ledger, keyed-HMAC audit, timing floor, transport key custody) | In-flight tamper, replay, side-channel on a wire | **n/a** (kernel-protected loopback) | **n/a** (stdio is in-process pipes; in-process hosting per DOS-647-C makes this even more obvious) | meaningful |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:25:**Layer 1 — First-party WP loopback (DOS-761).** WP plugin (`class-dailyos-runtime-client.php:256–278`) ships **16 HTTP headers per request** through 8 validation gates for an invoke from a process running as the same OS user as Tauri. WP plugin is code we ship. The Tauri React renderer skips all 8 gates by going through in-process `tauri::invoke` as `Actor::User`. WP plugin belongs in the same column.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:29:**Layer 3 — Third-party MCP transport-ceremony rip (new ticket).** v1.4.7 W1-A's substrate (PR #347, not yet merged to dev) contains ~645 LOC of transport-ceremony layered on top of ~1100 LOC of authorization machinery. For local stdio (today's primary MCP transport) and in-process hosting (DOS-647-C's goal), the transport-ceremony defends threats that don't exist. Rip the transport-ceremony layer; keep the authorization machinery verbatim. Specifics in §2c and §3e.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:31:**Why all three at once.** They're the same correction applied consistently. Splitting them means the reframe lands in one place and is contradicted by adjacent code in another — exactly the conflation that produced this mess. The 6-cycle L0 on v1.4.7 W1-A was internally coherent but premised on a wrong taxonomy; landing this packet IS the correction.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:33:**Explicitly preserved (load-bearing — see §8):** authorization machinery (per-client scope manifest, per-tool exposure tier, rate-limit, plain audit, conversation handle lifecycle), `Actor::McpClient` as a distinct actor class (third-party agents need scope-distinct attribution even if transport-ceremony goes), v1.4.2 W3-C WP-mediated MCP path, the pairing handshake itself, PR #355's `McpHandlerContext` interface (transport-agnostic), connector OAuth (Slack/Gmail external calls).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:45:| 3 | HMAC signature verify | `surface_runtime/mod.rs:1109` | Verifies `X-DailyOS-Signature` over canonical request | Tamper of in-flight loopback traffic (kernel-protected) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:46:| 4 | `validate_signed_session_readonly` | `surface_runtime/mod.rs:1154` (impl `services/surface_pairing.rs`) | Session id ↔ wp_user_id ↔ site-nonce binding | Cross-tenant session replay (single-tenant local) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:47:| 5 | `validate_session_bound_wp_user_id_for_request` | `surface_runtime/mod.rs:1969` | Body/query/header wp_user_id matches session-bound | Inconsistent identity injection (no second identity exists) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:48:| 6 | `SurfaceClientBridge::authorize` (scope + cap) | `surface_runtime/mod.rs:2123` | Granted scope contains requested ability; allowed_actors includes SurfaceClient | Privilege escalation (we're already Actor::User in React) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:50:| 8 | Audit-emit contract: `Actor::SurfaceClient` ⇒ `wp_user_id = Some(_)` | `src-tauri/src/audit_log.rs:104,141` | Refuses to emit audit if SurfaceClient actor lacks wp_user_id | Audit accountability (compromised by collapsing to Actor::User — see §3) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:52:**Materialization.** `registry_actor = validated.actor.clone()` (`mod.rs:2152`) wraps as `Actor::SurfaceClient { surface_client_id }`. The React path constructs `Actor::User` directly via `ServiceContext::new_live().with_actor(...)` in `services/context.rs:139`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:87:Branch `v1.4.7-w1-foundation` (PR #347, not merged to dev). Total ~3576 LOC across 7 files + 4 migrations. Mapped by the structural survey:
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:91:| `services/mcp_v2/auth.rs` | 566 | ~252 (pair_client, load_client_record, resolve_tool_grant, revoke_*, exposure helpers) | ~310 (verify_transport_hmac:232–244, verify_and_consume_and_preissue:257–315, transport key persist/load:399–474) | Partial — keychain custody is isolated; nonce consume/preissue is coupled to fail-closed contract; HMAC verify is called from gateway.rs:281 |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:92:| `services/mcp_v2/transport.rs` | 464 | ~370 (list_tools:157–188, build_input_schema, tool_from_description, unwrap_response, tool_error_to_mcp_error, tests) | ~70 (transport_key field:69, next_nonce field:71–72, call_tool nonce state machine:200–209+260, HMAC signing block:218–229) | Partial — nonce state machine interleaved with rmcp dispatch; envelope-shape stays, nonce population goes |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:93:| `services/mcp_v2/gateway.rs` | 883 | ~820 (handler registration, dispatch flow:303–528, scope subset checks, rate-limit reserve:715–786, audit/signal emission) | ~65 (Gate 0a HMAC verify:278–284, Gate 0b nonce consume+preissue:286–297, TIMING_FLOOR_MILLIS:33 + sleep_until_floor:795–801, preissued nonce threading across error paths:291–302+320–353+437–438) | Partial — nonce preissue threading touches ~20 lines across 5 error paths; non-local edit |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:94:| `services/mcp_v2/audit.rs` | 344 | ~144 (write API:50–78, sanitize_detail:111–124, add_actor_attribution:126–144, insert_outbox:181–220) | ~200 (hmac_json:146–150, canonical_value:152–168, audit key keychain custody:239–343, AUDIT_KEY_* constants:22–26) | Partial — keychain bits clean; hashing (lines 91–95) woven through `detail_with_hashes()` and all call sites; rip requires switching to plaintext emission OR field-level masking |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:97:| `services/mcp_v2/contracts.rs` | 851 | ~800 (ToolDescription, Scope, ScopedName, ParamSpec, ReturnSpec, OpaqueConversationHandle, McpClientId, McpActor, envelope shapes) | ~50 (OpaqueNonce type:223–241 — keep type for safety, drop usage; request_nonce/next_request_nonce fields on envelopes) | Clean (type-level) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:100:| `migrations/257_mcp_transport_nonce_ledger.sql` | 17 | 0 | 17 | **DROP entirely** |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:101:| `migrations/258_mcp_rate_limit_and_audit_outbox.sql` | 18 | 18 | 0 | KEEP (rate-limit ledger + audit outbox tables both load-bearing for ops layer) |
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:103:**Net rip estimate:** ~645 LOC removed, ~30 LOC modified (envelope shapes drop nonce fields), ~2900 LOC untouched. Migration 257 dropped before merge (replace with a no-op or renumber the rip-out into a new migration if 257 ships first).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:113:### 2e — Reuse audit (per `feedback_l0_reconcile_against_dev`)
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:115:Fork SHA: `852a0118`. `git diff 852a0118..dev -- src-tauri/src/services/context.rs src-tauri/src/surface_runtime/ src-tauri/src/audit_log.rs src-tauri/src/bridges/types.rs wp/dailyos/includes/transport/` returns **no changes** — dev has not advanced on any surface touched by this packet since the fork. Safe to land against `dev` after PR #351 merges. If `dev` advances before this packet's PR opens, re-run reconcile.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:128:2. Materializes `Actor::User` directly (same as React `tauri::invoke`).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:130:4. Calls `TauriAbilityBridge::invoke` (the React path's entrypoint) with a new `BridgeSurface::LocalLoopback` variant (decision in §5.Q1; recommended new variant for audit clarity).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:132:6. Audit emission: emits with `Actor::User`, required `loopback_origin: "wp_plugin"` field (today's only consumer; enum extensible). No `wp_user_id` contract — there is no wp_user.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:136:**Removed for `/v1/local/invoke` path only.** HMAC verify, session validate, wp_user_id-per-request, SurfaceClient bridge authorize, per-axis rate-limit, audit-emit `wp_user_id` contract (because actor isn't SurfaceClient).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:139:- `/v1/surface/invoke` (the existing signed-transport route) stays fully wired. v1.4.7's MCP v2 substrate (PR #347) uses it; DOS-647-C in-process MCP hosting will use it; if the WP-mediated MCP path (v1.4.2 W3-C) reaches into it, that stays too.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:140:- `services/mcp_v2/*` — gateway, auth, audit, actor_policy, taxonomy, transport, contracts. None of the v1.4.7 substrate moves in this packet.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:141:- `Actor::McpClient` and the `McpClientId`-bound scope manifest, nonce ledger, keyed-HMAC audit. Third-party agent ceremony is correct and stays.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:184:4. Add `/v1/local/invoke` route + `Actor::User` materialization for loopback + new `BridgeSurface::LocalLoopback` variant.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:189:7. Hold PR #347 merge OR rebase PR #347 over PR C.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:190:8. Apply the §3e rip: delete migration 257, delete ~310 LOC from auth.rs, ~70 LOC from transport.rs, ~65 LOC from gateway.rs, ~200 LOC from audit.rs. Modify ~30 LOC across envelope shapes.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:202:- `src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql` — entire schema gone. If 257 ships in dev before PR C lands, replace with a `DROP TABLE IF EXISTS mcp_transport_nonce_ledger;` rip-out migration at the next slot.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:206:- `services/mcp_v2/auth.rs:257–315` — `verify_and_consume_and_preissue()` fail-closed nonce ledger.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:210:- `services/mcp_v2/transport.rs:71–72` — `next_nonce: Arc<Mutex<Option<OpaqueNonce>>>` field.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:211:- `services/mcp_v2/transport.rs:200–209,260` — nonce pull-from-state and store-next-nonce.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:212:- `services/mcp_v2/transport.rs:218–229` — HMAC envelope signing block.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:214:- `services/mcp_v2/gateway.rs:278–284` — Gate 0a HMAC verify call.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:215:- `services/mcp_v2/gateway.rs:286–297` — Gate 0b nonce consume+preissue call.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:217:- `services/mcp_v2/audit.rs:22–26` — `AUDIT_KEY_*` constants.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:218:- `services/mcp_v2/audit.rs:146–150` — `hmac_json()`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:219:- `services/mcp_v2/audit.rs:152–168` — `canonical_value()` (canonical JSON for HMAC).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:220:- `services/mcp_v2/audit.rs:239–343` — audit key Keychain custody (`load_or_create_audit_key`, `read_audit_key`, `persist_audit_key`, etc).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:224:- `services/mcp_v2/contracts.rs` — `McpToolRequestEnvelope.request_nonce` field becomes `Option<OpaqueNonce>` or removed. `McpToolResponseEnvelope.next_request_nonce` similarly.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:225:- `services/mcp_v2/audit.rs:80–100` — `detail_with_hashes()` becomes `detail_with_attribution()`. Emit params/response **plaintext** in the detail (no HMAC hashing) OR switch to field-level masking via the existing `sanitize_detail()` (lines 111–124) — decision in §5.Q9.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:226:- `services/mcp_v2/gateway.rs:291–302,320–353,437–438` — unthread `preissued` from rejection-response building. Response envelope no longer carries `next_request_nonce`.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:230:- Entire authorization machinery: pair_client, manifest load, scope resolution, exposure-tier check, conversation handle lifecycle, scope-subset enforcement, rate-limit reserve, plain audit emission, audit outbox, `actor_policy.rs`, `taxonomy.rs`, `contracts.rs` type ecosystem (minus nonce/HMAC bits).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:231:- `Actor::McpClient` as a distinct actor class — third-party agents need scope-distinct attribution from `Actor::User`, even if they don't sign envelopes.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:235:**New invariant.** Audit log row format becomes: `{ actor: McpClient(client_id), conversation_handle, ability, params: <plaintext or masked>, response: <plaintext or masked>, request_id, timestamp }`. No HMAC hashes. Tamper-evidence is delegated to the OS — the SQLCipher DB the user controls is the audit substrate.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:237:**What CSO must re-pass on.** §6.5 reviewer prompt is rewritten to bound the CSO question precisely: "the personal-tier trust model puts the user in control of their own DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with SQLCipher + Keychain DB encryption, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't?"
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:253:- [ ] Handler materializes `Actor::User` and calls `TauriAbilityBridge::invoke` with the canonical path. No HMAC, session, scope, or wp_user_id validation in the loopback handler.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:254:- [ ] Audit log emits an entry per loopback invoke with `Actor::User` and optional `loopback_origin` for traceability. No SurfaceClient/wp_user_id contract.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:262:- [ ] `services/mcp_v2/auth.rs` shrinks by ~310 LOC: HMAC verify, nonce ledger, transport key custody gone. Manifest load + scope resolution + handle lifecycle intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:263:- [ ] `services/mcp_v2/transport.rs` shrinks by ~70 LOC: nonce state machine + HMAC signing gone. `list_tools`, `tool_error_to_mcp_error`, dispatch flow intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:264:- [ ] `services/mcp_v2/gateway.rs` shrinks by ~65 LOC: Gate 0a, Gate 0b, timing floor, preissue threading gone. Dispatch + scope subset + rate-limit + audit emission intact.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:265:- [ ] `services/mcp_v2/audit.rs` shrinks by ~200 LOC: HMAC hashing + audit key custody gone. Emission + outbox + actor attribution intact. Detail emits plaintext-in-encrypted-DB OR field-masked (per §5.Q9 decision).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:266:- [ ] `Actor::McpClient` retained; scope manifest enforcement intact end-to-end.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:267:- [ ] Local MCP integration smoke test: a stdio MCP client (real or mock) connects, sees `tools/list` filtered by manifest grants, invokes a read tool, receives result. No signing, no nonces.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:270:- [ ] DOS-624 sensitivity gating still composes correctly downstream of `Actor::McpClient` materialization.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:276:- [ ] PR #347 merge held until PR C reshapes the substrate; or PR #347 author rebases over PR C; coordination decision in §5.Q8.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:283:- *Pro new:* keeps audit/traceability distinct so we can tell "React in-process invoke" from "WP-plugin first-party-loopback invoke" without parsing the actor.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:288:- **Hard no.** v1.4.7 W1-A (PR #347) uses it. DOS-758 / PR #355 builds on it. DOS-647-C will land more handlers into it. WP-mediated MCP (v1.4.2 W3-C) may reach into it. Stripping or deprecating it now would burn 6 cycles of L0 review and block all three open MCP tracks.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:296:- Need a grep audit to confirm no producer composes against it.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:297:- **Recommend:** the L1 author audits in commit 1 and either attaches (with live adapter) or files a Linear comment confirming orphan + adds explanatory comment at trait definition.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:301:- First-party WP plugin as Actor::User has no such defense. Worth it?
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:303:- **Decided now:** no rate-limit on `/v1/local/invoke`. To keep runaway loops observable (not deferred), §3e's per-invoke audit emission + the §3c `log::warn!` instrumentation give a clear signal: if the same `(actor, ability)` pair fires N+ times in a window, the audit log shows it immediately. If a viewer surface is needed to surface that signal, it's a viewer ticket, not a rate-limit ticket.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:305:**Q6.** Should `Actor::User` materialization from `/v1/local/invoke` distinguish WP-plugin origin from a (future) in-process MCP host?
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:306:- DOS-647-C is in-process MCP hosting — moving handlers from the standalone binary into the signed-route server. It still mints `Actor::McpClient` for the requesting agent, not `Actor::User` — the in-process move is a deployment change, not a trust-model change.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:308:- **Recommend:** `loopback_origin` enum is restrictive — only first-party surface kinds. Add a CI lint that rejects new `loopback_origin` variants without a CSO sign-off comment in the PR body. Today's enum: `{ wp_plugin }`. MCP hosts do NOT use this route.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:313:- **Recommend:** add an invariant note in §3a: *"sensitivity gating composes downstream of actor materialization. First-party `Actor::User` and third-party `Actor::McpClient` may see different projections of the same intelligence by design."*
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:315:**Q8.** PR #347 coordination — what happens to the existing open PR?
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:316:- *Hold + rebase:* PR #347 author cherry-picks the KEEP set (~2900 LOC of authorization machinery), rebases over PR C's drops. Preserves PR review history; 6 cycles of L0 reviewer context stays attached. But: the 6 cycles validated the wrong premise; preserving them carries assumption-baggage forward.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:317:- *Close + supersede:* close PR #347 as superseded by PR C. New L2 cycle on the simplified substrate. Loses review history but the history was internally-coherent-on-wrong-premise.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:318:- **Decided now:** **close + supersede**. PR C lands the substrate at the new shape directly (authorization machinery + dispatch + handlers + migrations 255/256/258 + taxonomy + actor_policy + contracts) — it IS the substrate, not an amendment to it. PR #347 closes with a comment pointing at PR C. The v1.4.7 wave plan amendment lands as part of PR C's diff (`.docs/plans/v1.4.7-waves.md` updated in same PR). Nothing follows-up; nothing is parked. DOS-168 description + AC update at PR-open time.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:321:- `sanitize_detail()` already exists at audit.rs:111–124. The mechanism is built; the question is whether to use it.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:322:- *Read-class abilities:* params are entity IDs, depths, query strings. Plaintext-in-encrypted-DB. The user is the only auditor; SQLCipher + Keychain gates the file.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:323:- *Write-class / submit-class abilities:* params may carry user-typed content (note text, claim corrections, document body). Mask via `sanitize_detail()` to drop named payload fields. Preserves audit attribution without re-rendering user content into a separate log substrate.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:324:- **Decided now:** PR C ships both per ability `Side` (`Read` / `Write` / `SubmitCorrection` from contracts.rs:18–23). Read = plaintext detail. Write/SubmitCorrection = `sanitize_detail()`-masked. Wired in `services/mcp_v2/audit.rs::detail_with_attribution` based on the ability's declared Side. CSO confirms in §6.5 that the existing masking set in `PARAM_PAYLOAD_KEYS` / `RESPONSE_PAYLOAD_KEYS` is the right list for write-class.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:327:- **Decided now:** amend DOS-168 directly. DOS-168 IS the v1.4.7 W1-A MCP v2 gateway substrate ticket; it has not merged. Updating its description + acceptance criteria to reflect the simplified substrate (drop transport-ceremony, keep authorization) is the honest move. PR C closes DOS-168 at the new shape. No new umbrella. DOS-168 description + AC update lands at PR C open time. PR #347 closes superseded-by-PR-C per §5.Q8.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:336:1. **codex challenge** — adversarial: "find a same-OS-user threat that the gate stack defends and the loopback model doesn't. If you find one, propose the cheapest non-gate mitigation (process-level, audit-level, OS-level). Don't argue for keeping the gates abstractly."
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:340:5. **CSO** — trust-boundary scrutiny on the full packet: "(a) for first-party WP loopback: any third process running as the same OS user that could intercept the loopback port, confused-deputy risk if Tauri's own renderer is compromised, audit-log integrity if `loopback_origin` is client-supplied; (b) **for MCP transport-ceremony rip:** the personal-tier trust model puts the user in control of their own SQLCipher+Keychain DB. Audit hashes were keyed-HMAC to defend against a tampered audit log. In personal-tier with DB encryption + OS-user file ownership, what's the residual threat that keyed-HMAC defended that plaintext-in-encrypted-DB doesn't? Same for nonce ledger — what local-stdio replay attack is the nonce defending? (c) confused-deputy / scope-elevation: if a third-party MCP client (Claude Desktop) is compromised by a hostile MCP server, does the scope manifest still hold without HMAC? (d) PR #347's 6-cycle L0 sign-off on the keyed-HMAC/nonce/timing-floor model — was it premise-validated or only internally-validated? If premise-validated, surface the threat model that changed."
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:342:7. **v1.4.7-cross-track reviewer** (`/codex consult` pointed at `.docs/plans/v1.4.7-waves.md` + PR #347 + PR #355 + DOS-647-C ticket) — "this packet **reshapes** the v1.4.7 W1-A substrate, not just preserves it. Audit: (a) the §3e KEEP/DROP split in §2c-bis — anything mis-categorized? Specifically the conversation handle lifecycle (auth.rs:321–356), is it really transport-independent or does it depend on nonce-binding? (b) DOS-647-C's in-process MCP hosting plan — does dropping HMAC/nonce simplify or complicate the in-process move? (c) the §3d hybrid coordination (PR C ships substrate at new shape; PR #347 becomes docs-only amendment) — is this the right cycle hygiene or should PR #347 be closed-and-rebased? (d) PR #355 `McpHandlerContext` lands independently — confirm no transport-ceremony dependency. (e) DOS-624 sensitivity gating composes downstream of actor materialization — confirm intact under rip."
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:364:**Authorization machinery — diff against KEEP set must be zero except for envelope nonce-field removal:**
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:369:- `services/mcp_v2/gateway.rs` KEEP set (~820 LOC of dispatch + scope subset + rate-limit + audit emission + signal emission).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:371:- `services/mcp_v2/audit.rs` KEEP set (write API, sanitize_detail, attribution, outbox).
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:372:- `services/mcp_v2/contracts.rs` — KEEP all types except remove nonce fields from envelopes.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:374:- `Actor::McpClient`, `McpClientId`, `OpaqueConversationHandle`, `ToolGrant`, `ClientRecord` — KEEP all types.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:387:- Inner-block envelope-shape mapping (Open Thread #2 in handoff) is one L4 verification cycle, not a separate workstream — runs as part of PR A's L4 since the readers + observability are what surface the shape-gap symptoms. If shape gaps exist, fix in PR A.
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:393:- PR C (DOS-NNN MCP rip) coordinates with v1.4.7 W1-A per §5.Q8 hybrid: PR C ships substrate at the new shape, PR #347 becomes a docs-only amendment to `.docs/plans/v1.4.7-waves.md`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle1.md:7:3. **HIGH — DOS-725 tint decision is not locked in code or ADR.** Evidence: W2 claims an ADR-0077 amendment and `$stub_tints['dailyos_project'] = 'olive'` at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:273`, `:298`, `:587`. `chrome_config()` has no `$stub_tints`; it returns direct branch arrays, and the project branch lacks `tint` at `wp/dailyos/theme/functions.php:161` and `:222`. ADR-0077 only lists olive as a color token at `.docs/decisions/0077-magazine-layout-editorial-redesign.md:75`; no amendment exists. Recommended fix: land/amend ADR-0077 before L0 close and update the packet to the real `chrome_config()` structure. Path-α candidate: **no**.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle1.md:9:4. **HIGH — AgentMcp placeholder leaks a timing/existence oracle.** Evidence: W2 Option A exposes “timestamp + trust band + lifecycle” at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:726` and explicitly wants the agent to know the touchpoint exists at `:729`. W1 AC-341.12 forbids AgentMcp source-asof timestamps as a timing oracle and forbids graph IDs at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:939` and `:973`; the builder nulls `source_asof`/`updated_at` for AgentMcp at `src-tauri/src/services/claim_receipt/privacy.rs:321` and `:328`. Recommended fix: choose Option B, or define a coarse aggregate coverage signal with no per-item timestamp. Path-α candidate: **no**.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:7:**Cycle-1 verdict reviewed:** `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md`
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:11:All 3 cycle-1 conditions resolved cleanly. V1.1 fold introduces no new architectural debt; the §10 W4-coupling matrix, the class-contract invariant, and Patch 9 design all hold under stress-test. K-in citations correct.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:22:- Matches the strengthening I proposed in cycle-1 §7 Condition 1 verbatim in scope (the language is tighter than my suggestion).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:27:- AC #23 covers both stylesheets and scripts (cycle-1 ask was script-only; V1.1 strengthens to both, correctly). Pairs with line-76 plugin-side test that confirms trust/provenance still renders.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:34:- Decision rule rationale (lines 43-47) addresses the 3 criteria I named in cycle-1 §7 Condition 3 (page-invariant, Tauri-WP parity load-bearing, end-user per-page authoring non-goal).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:43:- W4 paths per `L0-packet-W3-chrome-lane-pulled-forward.md:451`: `src-tauri/src/services/surface_nonce.rs`, `src-tauri/src/bridges/surface_client.rs`, `src-tauri/src/commands/surface_runtime.rs`, `wp/dailyos/includes/class-dailyos-plugin.php`, `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`, `wp/dailyos/blocks/account-overview/*`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:46:- **No hidden coupling surface** — `wp/dailyos/theme/theme.json` (auto-generated from `src/styles/design-tokens.css`) is listed in §6 as substrate-consumed-not-edited; if v1.4.3 W4 needs to mutate theme.json, the disjoint rule breaks. Greped `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` — revised W4 explicitly extends `surface_nonce.rs` (no theme.json touch). Disjointness holds.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:71:- AC #28 Pill dual-primitive allowlist gate is a structural CI gate, not a one-off check — addresses the maintenance flag from cycle-1 §4. Filed to Codebase Maintenance project per cycle-1 §7 non-blocking suggestion. Verified `wp/dailyos/blocks/pill/` exists and is plugin-owned; chrome `.Pill_*` selectors verified non-overlapping.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:73:- AC #29 Site Editor DB override safety + §13 reset path appendix close a real footgun cycle-1 didn't surface — that's net-positive for the packet.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:78:## 3. K-in audit (reviewer-independent re-grep)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:86:`.docs/decisions/` — re-confirmed cycle-1 finding: 5 ADRs consumed (0073, 0076, 0077, 0129, 0130); no conflicts; no reinvention.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:103:- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md` — cycle-1 verdict (3 conditions)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:109:V1.1 folds the 3 cycle-1 conditions cleanly, adds 3 net-positive ACs (#26, #28, #29) that strengthen the substrate boundary beyond what cycle-1 asked for, codifies 3 new invariants in §10 (runtime-injection scope, class-contract-not-DOM-order, W4-coupling matrix), and surfaces no new architectural debt. Patch 9 design is sound; the overlay-vs-canonical decision is the established pattern for the lift and the failure mode (patch anchor lost) is loud. K-in citations verified accurate.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:111:**Verdict: APPROVE.** Cycle-2 closes from this reviewer.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md:1:# L2 (Diff) — code-reviewer (correctness) — v1.4.4 W1+W2 wave, cycle-1
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md:19:**ADR cite:** ADR-0123 V1.1 + memory `feedback_wp_is_local_surface_not_remote` — "reads don't mutate sessions/audit/rate-limits; only feedback writes." `record_claim_feedback` resolves to `services::claims::record_claim_feedback` which opens a `db_write` connection and persists an append-only `claim_feedback` row.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md:23:**Fix:** Split into two helpers: `dailyos_*_claim_receipt_only( $claim_ref )` for render-time fanout (call `claim_receipt` only) and `dailyos_*_record_feedback( $claim_ref, $action, $metadata )` for nonce-gated POST handlers that emit typed feedback (MergeIntent etc.). Remove the unconditional `record_claim_feedback` from the render-path helper.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md:60:- **MergeIntent affordance UI** (`recommended-actions/render-functions.php:109-209`): Agent audience denied (AC-8.13); data-attrs only emitted; click handler is documented as POST→nonce→record_claim_feedback Tauri-side. No mutation on render.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md:73:3. No test exercises the merge-intent payload round-trip from a clicked button through the nonce endpoint into `record_claim_feedback`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:1:# L0 codex-consult — W2 packet substrate verification (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:21:- DOS-339/341/477 `0a586218` — fix(w1): L2 cycle-2 substrate patches
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:22:- DOS-340 `e74d49dc` — feat(claim_receipt): receipt vs operational audit boundary
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:26:- cycle-2/3 `00f38b3b`, `1a56d612`, `c2e857b8`, `aa48ffce`, `3e58bc13` — all reachable, subjects align
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:54:1. **K-in inventory count out of date.** Packet §3 line 75 states "16 .md files at scan time"; `git ls-files docs/solutions | grep '\.md$'` currently returns 24 paths (excluding architecture-patterns intra-dir). New entries since the packet scan: `hmac-canonical-change-requires-golden-vector-recompute-2026-05-20.md`, `codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md`, `wip-and-no-verify-escape-hatches-under-memory-pressure-2026-05-20.md`, plus other K-out from W1 close. Recommend re-running the K-in grep at L1 kickoff and updating §3.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md:62:**Approve to advance.** Drift flags should be fixed inline before L0 close but do not require re-review.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md:16:**Fix:** Update §5.7 sketch step 2 + §5.3 fixture note (line 357 comment) to show `&AppState` arg and `RenderActor`/`RenderSurface` types. AC-8.3 authorization matrix should explicitly call out the Actor→RenderActor adapter as a sub-task (mapping `Actor::{User,Agent,SurfaceClient,System}` ↔ `RenderActor`).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md:45:APPROVE pending F1+F2 sketch corrections (one-line fixes; not a re-review cycle). F3 deserves an explicit §5.6 sentence noting Proposal/WorkItem render arms are DOS-701-deferred so W4 doesn't budget against them. F4/F5 are info-only.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:6:**Scope (per James 2026-05-20 opt-in):** DOS-477 (§5.4), DOS-8 (§5.7), DOS-341 (§5.9). Read-path sub-tickets DOS-459 / DOS-460 / DOS-461 / DOS-335 / DOS-339-wiring / DOS-340 / DOS-507 are OUT of CSO scope this cycle.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:8:**Substrate ground truth verified:** `src-tauri/abilities-runtime/src/sensitivity.rs` exports `RenderSurface`, `RenderActor`, `ClaimSensitivity`, `RenderPolicy`, `RenderDecision`, `RenderableClaimText`, `ClaimVerificationState`, `render_policy_for_surface(claim, surface, actor)` + `renderable_claim_text_with_value(...)`. `src-tauri/src/services/claim_receipt/contracts.rs` exports `ReceiptTarget`, `SurfaceContext` (ActionsWork | EntityDetail | DailyBriefing | MeetingDetail | Mcp), `Freshness`, `RedactionLevel`, `ReceiptTrust`, `ReceiptLifecycle`, `ProvenanceSource`, `ClaimReceipt`. `claim_receipt/auth.rs` already at 319 LOC with `AuthError` + `can_surface_for` (referenced in §5.4 + §5.7).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:14:- **DOS-477 (§5.4) — entity-detail trust-boundary hardening:** **CONDITIONAL APPROVE.** Substrate composition is correct; sensitivity gate name in packet (`render_policy_for`) does not match the shipped symbol (`render_policy_for_surface`) and four high-severity gaps need to be closed before L1 dispatch.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:89:**Active-verification hook:** Adversarial fixture in `feedback::tests::wrong_source_index_aliasing_race` — race a source revocation against a `WrongSource` submission; assert no source-reliability decay applied to innocent successor source. Pairs with memory `feedback_enumerate_channels_before_patching` — every source-identity channel into the feedback writer audited in one sweep.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:105:**Concern:** §5.7 authorization matrix says `ActionsWork` + `EntityDetail` allow Agent (SurfaceClient) up to `Internal` sensitivity. But ADR-0125 §2 makes `Internal` the **default** for claims sourced from internal systems — meaning the bulk of claims an agent encounters are `Internal`-classified. The matrix as-stated lets the SurfaceClient agent submit feedback on ANY `Internal`-or-lower claim. Per ADR-0123 §8 ("Feedback from non-User actors. … v1.4.0 commits user-only"), agent-side feedback is **out of scope** at the ADR level today. Packet's matrix opens an authorization door the ADR closes.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:107:**Recommended fix:** AC-8.13 (new): in v1.4.4 W1, `actor: Actor::Agent` is **denied** at all surfaces (not allowed-up-to-Internal). The matrix row collapses to `deny` for Agent everywhere until an explicit ADR-0123 amendment + v1.4.7 MCP scope manifest expands it. Tighten in §5.7 substrate; defer broadening to v1.4.7 path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:125:**Concern:** `apply_privacy_for_audience(receipt, audience, sensitivity) -> ClaimReceipt` — signature implies a transform applied to an already-rendered receipt. The DOS-339 substrate at `render::render_receipt_for` already composes `render_policy_for_surface` for the claim text. Two layers of redaction (render-time text policy + post-render audience filter) is a recipe for drift — a confidential token could survive the post-render filter if it landed in a non-allowlisted field that gets added to `ClaimReceipt` in a future PR. Per memory `feedback_systemic_look_for_recurring_issue_classes`, this is the audit-disclosure-allowlist class.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:135:**Concern:** Privacy matrix `OperationalAuditStorage` row says "no surface — storage only; non-disclosure assertion class with positive/negative fixtures". Packet does not specify where the non-disclosure assertion is *enforced*. ADR-0108 §1 separates the security audit (`.jsonl`, ADR-0094) from operational audit (`maintenance_audit` SQLite). The risk is: a future surface (e.g., a debug viewer for power users, an admin tool) accidentally queries `maintenance_audit` and exposes raw model i/o through a code path that bypasses `apply_privacy_for_audience`. Memory `project_engineering_ladder` "trust-boundary fence" CI gate is the structural pattern.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:137:**Recommended fix:** AC-341.11 (new): `services::claim_receipt::privacy::OperationalAuditStorage` audience is a non-disclosure tag, not a render target. ANY call site that touches `maintenance_audit` rows must be either (a) the audit-management commands allowlisted in DOS-340 §5.8 lint, OR (b) routed through `apply_privacy_for_audience` with a non-OperationalAuditStorage audience. Cross-bind with `check_audit_disclosure_allowlist.sh` (DOS-340) — same lint, just expanded to cover the privacy-side as well.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:139:**Active-verification hook:** Extend `check_audit_disclosure_allowlist.sh` to also forbid any direct read of `maintenance_audit` table rows from `services::claim_receipt::*` (the privacy module is allowed to *write* assertions but never to *surface* raw rows). Pair with negative-fixture test: a deliberately-attempted disclosure path that the lint catches.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:147:**Recommended fix:** AC-341.12 (new): add explicit `AgentMcp` row to the privacy matrix. Field allowlist for AgentMcp: trust band, freshness (coarsened — Current/Aging/Stale only), redaction level, lifecycle state, sanitized evidence_summary (sanitizer per ADR-0108 §3), `subject_type` (not `subject_id`). Forbid: source labels (even generic), source_asof timestamps (timing oracle), claim_id (graph structure leak).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:159:**Active-verification hook:** Wave-integrated L2 `/cso` script invocation against the merged-on-dev W1 SHA range. Verdict logged as Linear comment per memory `feedback_linear_is_the_audit_trail`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:171:4. **One CI lint script covers the receipt-boundary class.** DOS-340 `check_audit_disclosure_allowlist.sh` + the new `check_sensitivity_gate_composition.sh` (Finding 1) + the privacy-side disclosure extension (Finding 11) all sit in `src-tauri/scripts/`. Memory `feedback_zoom_out_for_class_pattern_in_l2_loop` — this is the structural-gate class.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:179:Unanimous APPROVE required across the 4-panel + `/cso` on the three sub-tickets named. This verdict is `CONDITIONAL APPROVE` on each, with 12 specific findings to fold into V1.1. Recommend the packet author folds the findings + reposts V1.1; cycle-2 dispatch only if a reviewer disputes a fix surface. Per memory `feedback_l0_review_loop_diminishing_returns_means_scope_is_wrong`: if cycle-2 surfaces 5+ net-new findings, scope is wrong; this verdict reflects scope-correct, contract-incomplete.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle4.md:19:Conclusion: L0 unanimous APPROVE is now possible; the other 4 reviewers carry forward APPROVE from cycle-3.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:6:**Verdict:** APPROVE — both cycle-2 advisory observations resolved. No new design-blocking gaps. No cycle-2 regression.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:10:## K-in record (cycle-3 re-confirm)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:15:**`.docs/decisions/` grep:** ADR-0073 and ADR-0076 re-read for focus-visible + outline-color guidance below. ADR-0077 unchanged. No new ADRs since cycle-2. K-in clean.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:29:The row is in the formal deferrals table with a named recipient (W2 L0 author), a resolution mechanism (ADR-0077 amendment or new ADR), and a pre-condition gate (must resolve before CPT registration). The cycle-2 concern was that the obligation lived only in a code comment where a W2 L0 author might miss it. It now lives in the packet's structured §8 deferrals table — the canonical handoff surface. Ticket-ID placeholder follows the same "TBD-at-L1-kickoff" convention used consistently across §8. **Resolved.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:37:The AC is now specific enough to implement without guessing. The cycle-2 concern was that a competent implementer would have to infer the fixture requirement; V1.2 makes it explicit with an example. **Resolved.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:41:## Focus areas (cycle-3 dispatch)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:89:ADR-0073 does not prescribe a specific focus ring width or offset. The 2px/2px combination is the CSS specification's suggested baseline for `outline`-based focus indicators and is the pattern used in the existing `FolioBar.module.css` rules for other interactive elements (confirmed by cycle-2 review citing `.FolioBar_folioBreadcrumbButton:focus-visible` as a consistent reference). There is no ADR-0073 or ADR-0076 deviation.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:125:### 4. Regression check against cycle-2 APPROVE
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:127:Reviewing V1.2 changes against cycle-2 APPROVE baseline:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:133:- **AC #30 regex fix** — tightened from `var\(--[a-z-]+\)` to `var\(--[A-Za-z0-9_-]+(?:,[^)]+)?\)`. This closes a real gap the cycle-2 challenge surfaced. Makes the AC more correct, not less. Not a regression.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:139:**No cycle-2 regression found.** All V1.2 changes are either fold-of-challenge-blocker or fold-of-advisory-observation. The dimensional ratings from cycle-2 (Interaction state coverage 9/10, User flow completeness 8/10, Unresolved design decisions 9/10) hold or improve with the two advisories now resolved.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:143:## Dimensional rating (cycle-3 delta only)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md:155:Both cycle-2 advisory observations are resolved in V1.2. §5.4.1 CSS parity is complete against `folio-refresh-button.tsx:38-52` with two token-substitution improvements (border-radius, transition). The `:focus-visible` 2px turmeric outline is consistent with ADR-0073 accent semantics and the existing FolioBar interactive-element pattern. Disabled state behavior is coherent across React and chrome.js surfaces. No cycle-2 regression. The `[aria-busy="true"]` selector gap in the React component is pre-existing and belongs to maintenance, not this packet.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:22:**Architectural concern:** v1.4.6-waves.md §327 + §414 ("Reserved block: **v260–v279** (20 slots)") reserves v260–v279 for the v1.4.6 Salience & Recommendations wave (`recommendation_claims`, `salience_factors`, `surfacing_decisions`, `triggers_log`, `deviation_baselines`, `engagement_telemetry`). v1.4.6 wave is already through cycle-4 of its plan with these slots load-bearing for W1-A/W1-B substrate. v1.4.6 §327 also explicitly states "v1.4.4 holds v240–v249" — i.e., the cross-version coordination table says v1.4.4 owns v240–v249, NOT v250–v269. This packet claims v250–v269 unilaterally, which collides with v260–v269 on the v1.4.6 side AND breaks the cross-version coordination table v1.4.6 already committed to.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:50:**Architectural concern:** Per CLAUDE.md Critical Rule, every new table/schema-column/claim field/user-visible intelligence surface must answer "What signals does it emit, and which propagation/invalidation paths refresh derived state?" The packet says "no new signals" — but DOS-335 lands TWO new migrations (v250 indexed view, v251 dismissals table) AND introduces a new lifecycle state (`UserSuppressed` / `UserDismissed`). When a user dismisses prep status, downstream consumers (FolioBar readiness chrome — substrate-rendered per W3 chrome lane; Meeting Briefing block; Daily Briefing rollup) need to know the dismissal happened so they re-render. If "no new signals" is literally true, the only refresh path is poll-on-render, which means FolioBar will show stale prep status until next FolioBar tick.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:54:**Recommended fix:** Either (a) add a signal — `MeetingPrepStatusChanged { meeting_id, transition: PrepStatus → PrepStatus }` — pre-declared in `signals/policy_registry.rs` at W1 kickoff (mirror the v1.4.6 W0 cycle-4 fix C pattern at v1.4.6-waves.md §348); or (b) document the explicit poll-on-render contract with cycle bounds (FolioBar tick interval guarantees ≤ N seconds staleness) and surface this in §5.5 + §10 invariants. Option (a) is the substrate-coherent answer; option (b) ships drift. The 5-question gate per CLAUDE.md cannot pass with "no signal" if there's user-visible state to refresh.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:60:**Section cite:** §5.6 DOS-339 Intelligence Loop check item 3: "Receipt state updates on claim lifecycle / feedback / contradiction / source state changes."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:62:**Architectural concern:** The receipt is the shared DTO consumed by every W2–W5 surface. When a feedback action (DOS-8) fires → `record_claim_feedback` writes a row → claim verification state transitions → trust band recomputes → receipt SHOULD re-emit to all subscribers. The packet asserts the chain exists via "existing claim-lifecycle signals" but does NOT name (a) the specific signal type that triggers receipt re-emit, (b) the policy-registry entry that fans the signal out to receipt subscribers, or (c) what coalescing rule prevents N feedback actions on the same claim within 100ms from generating N receipt re-emits to the same WP block render PHP. Without (c), the W2/W4 surfaces will receive thundering-herd re-renders during burst feedback (e.g., user marks 5 stale claims in rapid succession in Actions/Work).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:66:**Recommended fix:** In §5.6, name the signal type explicitly (existing or new — likely existing `ClaimVerificationStateChanged` per ADR-0080 substrate; verify at L1). Specify the coalesce policy on the receipt-subscription hook (AC-339.1 already references `useClaimReceiptSubscription`; bind the coalesce window — recommendation: 250ms trailing-edge debounce per `(target.claim_id, surface)` pair). Add AC-339.6: "Multi-surface fan-out tested: feedback action on claim C from Actions/Work re-emits receipt to all subscribed Entity Detail / Briefing / Activity Log surfaces within 1 invalidation cycle; coalesce window prevents > N re-emits per (claim, surface) per second."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:88:Similar but smaller gap: §5.10 DOS-507 (Stage 1c) depends on §5.5 DOS-335 — packet calls this out (§13 Q10 last bullet) — but §5.5 ships migrations v250-v251 in Stage 1a; if §5.5's migrations land in Stage 1a but the DTO contract isn't frozen until Stage 1a closes, §5.10 can't start until 1a fully closes. The serial dependency is fine; the packet should make the migration-vs-contract-freeze boundary explicit.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md:1:# Design-Lens Review — v1.4.4 W2 Entity Surfaces (cycle 2)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md:26:Advisory (Confidence: 75): add one locked ARIA label to §10 invariant "AgentMcp touchpoint aggregate render" — e.g., `aria-label="N touchpoints; activity level: Recent|Aging|Stale"`. Without it the accessibility-tester pass at L4 will produce a finding that cycles back to this packet retroactively. The aggregate chip is user-visible (non-AgentMcp contexts render it too when content is redacted at the audience layer), so this is not speculative.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md:50:Advisory (Confidence: 50): if cycle-2 architecture review surfaces a similar concern independently, confirm the §5.1 sketch is explicitly marked as the cross-entity template in §10. No action required here.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md:66:None. Both cycle-1 blocking conditions are resolved. New considerations produce one advisory (ARIA label for aggregate touchpoint chip, pre-L4 obligation on DOS-692) and one cross-wave advisory (back-nav to W3). Neither blocks L0 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md:10:**APPROVE — no new findings. L0 architecture lane closes.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md:12:All three cycle-1 findings resolved; focus-area spot-checks land clean; convergence signal is real.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md:32:**3. DOS-725 tint as CSS custom property + CI gate.** §5.2 lines 353–359 + AC-W2.7 + §10 invariant (line 770). Pattern `style="--dailyos-project-tint: var(--color-garden-olive);"` on outer wrapper only, asserted via `^--[a-z-]+:\s*var\(--[a-z-]+\);?$`. Architecturally sound — narrowest possible inline-CSS exception (custom-property *assignment* only, no declaratives); ADR-0077 amendment gated as L1 prereq not L0-blocking; CI gate fails closed and opens narrowly; composes cleanly with ADR-0132.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md:45:**None.** Per memory `feedback_l0_review_loop_diminishing_returns_means_scope_is_wrong`, zero net-new in cycle 2 is the convergence signal — fold-and-close, not fold-and-cycle.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:7:**Scope (per Amendment 3):** DOS-477 (entity-detail trust-boundary), DOS-8 (semantic feedback write path), DOS-341 (privacy/redaction), DOS-340 (receipt-vs-audit boundary)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:9:**Active verification performed:** ran `src-tauri/scripts/check_sensitivity_gate_composition.sh` (clean), `src-tauri/scripts/check_audit_disclosure_allowlist.sh` (clean), `src-tauri/scripts/check_audit_disclosure_allowlist.sh.test` (PASS — lint catches disclosure attempt). Read paths exercised: `entity_intelligence/auth.rs`, `claim_receipt/{feedback,privacy,boundary,auth,render}.rs`, `commands/claim_feedback.rs`, plus the integration-test fixture matrices.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:26:- F11 → audit disclosure CI lint shipped (`check_audit_disclosure_allowlist.sh` + negative-test fixture `audit_disclosure_negative/` proves it catches violations).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:47:**Exploit scenario.** None directly — the receipt construction path enforces the same contract. The risk is **maintenance drift**: a future engineer adds a new field to `ClaimReceipt`, updates the per-audience builders, but forgets that `filter_for_receipt` exists as a separate allowlist (named identically to the one in `boundary.rs` but covering camelCase DTO keys, not snake_case audit columns). Two `RECEIPT_ALLOWED_FIELDS` constants with the same name in different modules — one in `entity_intelligence/auth.rs:455` and one in `claim_receipt/boundary.rs:52` — is a confusion hazard.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:63:**Path-α reason:** Substrate hardening; the only production caller (`commands/claim_feedback.rs:102`) hardcodes `RenderActor::user("user", None)`, so the bypass surface is server-trusted.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:65:**Concern.** `RenderActor::is_user()` returns true whenever `actor` starts with `"user:"` (case-insensitive). This is broad — any string `"user:agent-bypass-attempt"` would satisfy the check. In `submit_claim_feedback`, the Agent denial (line 266) relies on `is_user()` returning false for non-human actors. If a future caller exposes `RenderActor` construction to untrusted input (e.g., an MCP write-tool that synthesizes an actor from the tool-call payload), the prefix loophole becomes exploitable.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:69:**Recommended fix.** Tighten `is_user()` to require exact match `"user"` (or a typed enum, e.g., `RenderActor::Kind::User { id: String }` instead of a stringly-typed `actor` field). Today the kind is encoded in the constructor (`RenderActor::user(...)` vs `RenderActor::agent(...)`); make the kind a typed field, not a string prefix. Until then, document the constraint at the call site in `feedback.rs:266` so future MCP wiring can't accidentally hand the helper an attacker-controlled string.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:101:**Concern.** `boundary::filter_for_receipt(&OperationalAuditRow)` and `OperationalAuditRow` are unused outside tests. The DOS-340 boundary is enforced by the CI lint (`check_audit_disclosure_allowlist.sh`) which forbids `SELECT … FROM <audit_table>` in `services::claim_receipt::*`. The runtime filter is a backstop — useful if someone ever does need to project an audit row into a receipt-shaped view, but currently dead. Pairs with Finding 1: there are now TWO `filter_for_receipt` functions (this one + `entity_intelligence/auth.rs:469`) operating on different field-name vocabularies (snake_case audit columns vs camelCase DTO keys).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:105:**Recommended fix.** Either (a) rename one of the two functions (`filter_audit_row_for_receipt` vs `filter_receipt_snapshot_keys`) OR (b) consolidate into a single namespaced module. Lowest-cost: rename the boundary.rs one to `project_audit_row_to_receipt` since it's a different operation (row projection, not field filtering).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:141:**Trend vs L0 cycle-1:** L0 surfaced 12 findings (4 High, 5 Medium, 3 Informational). All 12 were folded into V1.1 ACs and shipped in the wave diff. L2 surfaces 0 net-new High findings — the contract closure is clean. The 5 Medium findings here are either deferred-wiring observations (Findings 1, 3, 4) or substrate hardening that's better suited to a structural pass than a fold-cycle (Findings 2, 5).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:149:This is an AI-assisted L2 (Diff) review against named acceptance criteria. It catches common substrate patterns and known ADR contracts; it is not a substitute for professional security audit on cutover (multi-user, MCP write surfaces, remote-agent flows in v1.4.7+). Re-run `/cso --comprehensive` after MCP write surfaces land — the threat model changes when actors stop being server-trusted.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:1:# L0 Architecture Review — v1.4.4 Wave Plan (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:55:The packet does not name this boundary. A naive flag-flip that hides ALL Tauri React UI breaks the runtime-host role; a flag-flip that hides "magazine surfaces only" needs an explicit list at W0 (it overlaps with the W0 surface audit's "active vs carry-forward vs inactive" classification but the audit doesn't currently distinguish dev/admin from end-user).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:57:**Recommended fix.** Amend §5.0 (W0 audit outputs) to add a fifth list: **Dev/admin surfaces** — surfaces that stay visible post-flag-flip because they're runtime-host concerns, not end-user surfaces. Settings-runtime, keychain debug, ability runtime status, MCP server status. These are NOT migration targets and they are NOT flag-flip targets. Reference this carve-out in §13 item #5: "Flag-flip hides W0-classified *end-user* Tauri React UI only. Dev/admin surfaces stay visible and are explicitly out of v1.4.4 scope."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:81:1. **Schema-stability of the W1 envelope (DOS-459) across v1.4.5 + v1.4.6.** v1.4.5 Workspace Memory adds source/ingestion claims; v1.4.6 Salience adds `RecommendationProposal` claims. Both will want to appear inside the `get_entity_intelligence` envelope shipped at v1.4.4 W1. The envelope's `schema_version` lifecycle (named in §5.1 as a W1 open question) is the forward-coupling pivot. If W1 picks "major-version-only ABI," v1.4.5 + v1.4.6 will need a major-version bump on the envelope they didn't ship. If W1 picks "semver," consumers need to negotiate. **This is an architectural decision the W1 sub-L0 will make; the wave packet should flag it as forward-coupling-critical.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:85:**Recommended fix.** Add §9 paragraph: "Forward-coupling hot spots: (a) DOS-459 envelope `schema_version` lifecycle is forward-coupling-critical for v1.4.5/v1.4.6 envelope extension; W1 sub-L0 picks semver-style with **additive-only minor bumps** (new fields land at minor; breaking changes at major); v1.4.5/v1.4.6 envelope extensions are minor bumps. (b) Review-queue migration slot in v1.4.4 W4 is coordinated with v1.4.6 review-queue hook migration slot to avoid v155-pattern collision."
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:91:The wave packet inherits this without naming the **default**. A naive reading of §13 item #3 (W3 FolioBar consumption) leaves room for a fifth runtime-injection module to creep in via a "briefing pattern needs chrome state bridge" argument. Memory `feedback_chrome_overlap_audit_before_new_pattern` calls this out — DS patterns can quietly duplicate chrome signaling.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md:140:3. F3 fix in §5.0 (5th audit list: dev/admin surfaces) + §13 item #5 (flag-flip scope).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:1:# L2 cycle-2 CSO verdict — wave/v1.4.4-w1-stage1a
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:4:**Diff range:** `9544d930..HEAD` (cycle-2 delta only)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:15:or regressions versus cycle-1 approval. The three modules audited implement the
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:16:expected cycle-1 fix shape:
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:18:1. **`envelope_cache.rs`** — server-side substrate primitive that solves the cycle-1 F3
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:44:   (cycle-1 F1 leak) would fail all three.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:46:The audited cycle-2 delta is sound. Defense-in-depth is preserved or improved on every
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:71:  cycle-2 regression. The cap exists; it just isn't an LRU. Memory cannot grow
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:100:  event scoping. No cycle-2 regression (cycle-1 had no bridge at all).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:173:pre-cycle-2 — the cache addition is strictly an *upgrade* path, never a
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:186:- **Bridge payload audit:** Read every field on `ClaimReceiptInvalidationPayload`,
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:210:This tool is not a substitute for a professional security audit. /cso is an
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:211:AI-assisted scan bounded to the cycle-2 delta on the three modules specified by
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:212:the L2 orchestrator. It does NOT re-audit cycle-1 substrate or PR scope outside
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:7:**Cycle-2 verdict reviewed:** `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md` (APPROVE)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:11:V1.2 strengthens the architectural posture (canonical pre-lift upgrade, source-of-truth invariant, sharpened Pill scope, AC #31 sequencing). Nothing introduced regresses the cycle-2 APPROVE; in fact, the source-of-truth invariant at §10 makes the substrate boundary tighter than V1.1 had it.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:60:- V1.2's removal of Patch 9b (refresh-button class replacement) from `patch-chrome-js.py` REDUCES the WP-overlay maintenance surface — cycle-2 noted Patch 9b's overlay-vs-canonical trade was "honest" and "bounded"; V1.2 promotes the cleaner half (canonical upgrade) and keeps only the WP-specific half (DOM idempotency, Patch 9a) as overlay. This is the architecturally cleaner choice and aligns with the cycle-2 §2 observation that 9b *could* go upstream but was held overlay to minimize Tauri regression surface — V1.2 reassesses regression risk and finds it low (the canonical mockup substrate has no production-runtime dependency).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:63:- §10's new "Canonical source-of-truth" invariant tightens the substrate boundary the cycle-2 verdict approved.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:67:## 2. K-in re-audit (cycle-3, V1.2)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:71:`.docs/decisions/` — no new ADRs introduced since cycle-2; the 5 cited (0073, 0076, 0077, 0129, 0130) remain consumed not overridden. Verified.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:90:- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md` — cycle-2 verdict (APPROVE)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:107:V1.2 cleanly resolves cycle-2's 3 NEW BLOCKERS via the architecturally-correct path (upstream to canonical, not WP-overlay), introduces a new §10 invariant that tightens the substrate boundary, sharpens Pill scope, and sequences the canonical pre-lift as a separate PR before chrome lane sync. The Tauri-UI freeze does not apply because §5.4.1 touches mockup substrate (`.docs/design/reference/_shared/`), not the React component. Cycle-2 APPROVE holds; V1.2 strictly strengthens.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md:109:**Verdict: APPROVE.** Cycle-3 closes from this reviewer.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:4:**Reviewer:** gstack `/review` skill (re-review of cycle-1 blockers + cycle-2 substrate scan)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:9:- `c2e857b8` — feat(w1): L2 cycle-2 consumer skeletons + CI lint wiring (AC-W1.2 + AC-W1.9)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:10:- `0a586218` — fix(w1): L2 cycle-2 substrate patches — DOS-339/341/477 wiring + bugfixes
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:12:- `f82d53ce` — docs(v1.4.4 W1): L2 cycle-1 verdicts (3 reviewers)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:14:**Bounding:** per `feedback_l2_must_review_against_acceptance_criteria` + path-α gate. Re-verified cycle-1 HIGH findings against the cycle-2 patch surface. New findings only surfaced if they meet L2 blocker criteria (literal AC violation, ADR-named contract violation, regression in PR-touched code).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:16:**Active verification this cycle:**
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:29:**APPROVE.** All three cycle-1 HIGH blockers are resolved by the cycle-2 patches. Cycle-2 introduced ~700 LOC of net-new substrate (`event_bridge.rs`, `envelope_cache.rs`, `render.rs` reshape, migration v243) — scanned for regressions in PR-touched code per `feedback_l2_must_review_against_acceptance_criteria`. No new L2-blocker findings surfaced. The wave clears L2 fully.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:31:Cycle-1 path-α residuals (Findings 4–10 in cycle-1 verdict) remain in scope for the maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — not re-litigated here.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:41:| 3 | HIGH | AC-340.2 / AC-340.7 / AC-477.11 — lint scripts not wired in CI | `c2e857b8` | ✅ RESOLVED | All 3 cycle-1-named scripts now wired as separate named workflow steps at `.github/workflows/lint-frontend.yml:121-128`; AC anchors named in step titles. Local execution: all 3 scripts exit 0. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:43:| 5–10 | MEDIUM / LOW / Informational | path-α only | — | DEFERRED to maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — not re-reviewed this cycle. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:45:Additional cycle-1 findings from sibling code-reviewer track that cycle-2 also addressed:
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:46:- **DOS-341 AC-341.10 wiring** — `render_receipt_for` now dispatches through `build_receipt_for_audience` rather than rendering directly; was a code-reviewer cycle-1 F1 finding. Verified at `claim_receipt/render.rs:55-115`. `PrivacyError`→`RenderError` transport via tag prefixes; exhaustive over all 6 variants.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:47:- **DOS-339 signal→event bridge** — new `claim_receipt/event_bridge.rs` (148 LOC) wires `ClaimVerificationStateChanged` signal to `claim_receipt:invalidated` Tauri event; 3 unit tests assert camelCase serialization, optional field omission, no-op on uninstalled handle. Was a codex review cycle-1 P1.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:48:- **DOS-335 v241 view non-determinism** — new migration `243_…sql` aggregates `meeting_entities` to one row per meeting via per-meeting MIN aggregation. Was codex review cycle-1 P2.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:52:## New Findings (cycle-2 substrate scan)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:58:- `src-tauri/src/services/entity_intelligence/envelope_cache.rs` (200 LOC, new module) — `parking_lot::RwLock` (already in deps), 5-min TTL, 2,048 cap, evict-on-read, `path_alpha_envelope_cache_v2` marker fn present per cycle-1 commitment.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:60:- `src-tauri/src/commands/claim_feedback.rs` (+114 LOC) — accepts `Option<String>` for `envelope_render_id`, falls back to logged-warning single-claim path on cache miss (cycle-1 behaviour preserved). New `ResolvedEnvelope` enum encapsulates cached vs fallback.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:68:Both are path-α per `feedback_l2_path_alpha_to_maintenance_project` and outside L2 cycle-2 blocker scope.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:72:## ACs verified met (delta over cycle-1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:74:In addition to all ACs verified in cycle-1 verdict, cycle-2 closes:
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:85:**APPROVE for merge to `dev`.** Wave clears L2 fully. Path-α residuals from cycle-1 (Findings 4–10) + new substrate caveats (envelope cache cap, AppHandle global) route to the Codebase Maintenance project per `feedback_l2_path_alpha_to_maintenance_project` — they do not block this PR.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:43:src-tauri/src/services/surface_nonce.rs:1883:                FeedbackAction::ConfirmCurrent,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:44:src-tauri/src/services/surface_nonce.rs:1888:                FeedbackAction::MarkOutdated,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:45:src-tauri/src/services/surface_nonce.rs:1893:                FeedbackAction::MarkFalse,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:46:src-tauri/src/services/surface_nonce.rs:1898:                FeedbackAction::WrongSubject,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:47:src-tauri/src/services/surface_nonce.rs:1903:                FeedbackAction::WrongSource,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:48:src-tauri/src/services/surface_nonce.rs:1908:                FeedbackAction::CannotVerify,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:49:src-tauri/src/services/surface_nonce.rs:1913:                FeedbackAction::NeedsNuance,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:50:src-tauri/src/services/surface_nonce.rs:1918:                FeedbackAction::SurfaceInappropriate,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:51:src-tauri/src/services/surface_nonce.rs:1923:                FeedbackAction::NotRelevantHere,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:52:src-tauri/src/services/surface_nonce.rs:1930:            assert_eq!(FeedbackAction::from(action), feedback_action);
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:53:src-tauri/src/services/surface_nonce.rs:1931:            assert_eq!(FeedbackAction::from(action).as_str(), expected);
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:54:src-tauri/src/services/surface_nonce.rs:2764:            FeedbackAction::MarkOutdated,
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:55:src-tauri/src/services/surface_nonce.rs:2772:        assert_eq!(outcome.action, FeedbackAction::MarkFalse);
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:56:src-tauri/src/services/surface_nonce.rs:2781:        assert_eq!(outcome.action, FeedbackAction::WrongSubject);
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md:57:src-tauri/src/services/surface_nonce.rs:2790:        assert_eq!(outcome.action, FeedbackAction::NeedsNuance);
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md:9:V1.2 resolves the cycle-2 consult blockers. No remaining consult finding should push this packet to L6.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md:22:- Broader grep over chrome/theme/token/wp/surface/nonce terms surfaces the same relevant entries: W4 K-in substrate correction, PHPCS warning-severity CI posture, and substrate-only/L0-amendment precedent.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md:30:- **2 cycles without convergence / L6:** V1.2 is expected to converge from consult. If another reviewer returns a substantive conditional or blocked verdict, follow §11 and escalate to L6.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:1:# W1 wave L2 — code-reviewer verdict (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:12:Two HIGH findings violate explicit wave acceptance criteria. One is an architectural gap (privacy module not in the production receipt-render call chain) and one is a CI gate that was promised in the L0 cycle-1 fold and never landed. A third HIGH is a placebo envelope-set construction at the Tauri command boundary that defeats AC-477.2 from the only consumer that actually runs in v1.4.4 W1.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:14:The V1.1 fold items from L0 cycle-1 (DOS-335 read/write split + AC-335.12 lint; BriefingState composed-struct; WrongSource content-hash; transitive composes walker; allowlist-primary boundary) all landed structurally — but the privacy gate is wired test-only, not in the production path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:27:- **AC-341.10** + **cycle-1 CSO F10 fold** — "`build_receipt_for_audience` is a render-time primitive (NOT a post-render transform); audience is an input to construction." Implementation is the latter — privacy lives in its own module called only by unit tests (`privacy.rs:770, 793, 829, 838, 858, 867, 876, 885, 908, 918`). `render_receipt_for` is not called with audience.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:44:**Trace:** `grep -l "get_entity_intelligence\|get_daily_briefing\|meeting_prep_status\|claim_receipt\|touchpoints" wp/dailyos/blocks/*/render-functions.php` returns zero hits. `ls src-tauri/scripts/` shows no `check_w1_consumer_skeleton.sh`. The wave is substrate-only; the CI gate that was the fold of cycle-1 codex-challenge F1 is absent.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:115:- ✓ **DOS-340 receipt vs audit boundary CI lint.** `src-tauri/scripts/check_audit_disclosure_allowlist.sh` + `.test` companion + `check_audit_denylist_completeness.sh` (AC-340.7) all present.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md:138:The substrate quality of this wave is high — the V1.1 fold items from L0 cycle-1 all landed structurally, the test harnesses are dense, the typed contracts (BriefingState composed struct, EnvelopeSet, FeedbackAction 9-variant taxonomy, PrepStatus state machine, audience-keyed receipt builders) read cleanly. The three HIGH findings cluster around a single theme: substrate is built, consumers are not wired. F1 fails to wire the privacy module into the renderer; F2 fails to wire any WP block to any producer; F3 fails to wire `get_entity_intelligence` into the Tauri command that depends on it. "Wiring IS the work" per AC-W1.1/W1.2 — and on these three axes the wiring is deferred.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle1.md:155:**Required fix:** Add Patch 9 (or extend Patch 4/8 scope) to replace the refresh button's inline style string with a CSS Module class (`F('folioRefreshButton')`) that is defined in `FolioBar.module.css`. Alternatively, add an AC that audits all `style=` attributes in the patched `chrome.js` and asserts each either has a corresponding module-class `:hover`/`:focus-visible` rule or is explicitly documented as mockup-substrate-only behavior that is acceptable in the WP production path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:5:**Cycle-1 verdict:** PARTIAL APPROVE (2 advisory drift) — `packet-W2-codex-consult-cycle1.md`
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:10:All cycle-1 advisory drift resolved. The one apparent NEW-DRIFT codex flagged is a false positive on inspection (see Check 4).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:14:**Check 1 (cycle-1 D1 — K-in inventory refresh): RESOLVED.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:20:**Check 2 (cycle-1 D2 — DOS-339 SHA footnote): RESOLVED.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:27:87df7cf6 feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:36:None. Recommend L0 cycle-2 closure on codex-consult panel.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:39:**Reviewer:** codex-consult (cycle 2)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:1:# L3 Codex Challenge — v1.4.4 W1 Wave (cycle 2)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:48:- Architect-reviewer cycle-1: APPROVE ✅
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:49:- Codex challenge cycle-2: 5/5 PATCHED CORRECTLY ✅
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:50:- Suite S (Security): GREEN — 4 CI lint scripts all exit 0; /cso cycle-2 APPROVE on cycle-2 substrate
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:52:- Suite E (Edge cases): GREEN — cargo test 2628/0 on cycle-2 wave (2637 on cycle-2 + F1-F5 patches still running for verification)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md:54:L3 closes. Proceed to PR open + K-out + next wave L0.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md:6:**Companion docs read:** `.docs/plans/wp-foundation-roadmap-reorientation.md` §"Anchored decisions" + §"v1.4.4"; `.docs/plans/v1.4.4-surface-audit.md`; `.docs/design/INVENTORY.md`; `.docs/design/product/{MISSION,PRODUCT-THESIS}.md`
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md:20:`docs/solutions/` grep for: `ux-state`, `interaction-state`, `empty-state`, `trust-band`, `user-flow`, `briefing`, `entity-detail`, `cross-wave`. No relevant prior solutions suppressed or reinvented. The prior design-lens reviews on the chrome lane packet (`packet-W3-chrome-design-lens-cycle3.md`) confirmed cycle-3 APPROVE on that lane; nothing in that record conflicts with findings here.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md:30:**Design concern:** AC #W1 says "supports the visible-QA-state matrix called out per W2 in DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / etc.)." The "etc." at the end plus the "per W2" scoping means sub-L0 authors for W3/W4/W5 have no authoritative state list. The W2 list in DOS-462 was designed for entity detail pages (Health/Context/Work views, claim-backed sections). Briefing surfaces and action surfaces have materially different state vocabularies:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md:33:- **W4 Action surfaces:** has a "no open actions" state (empty queue), a "claim pending review" state (stuck in review queue), a "contradiction flagged" state distinct from "corrected/superseded", and an inline-edit "in-flight" state where the WP block has emitted the feedback claim but the re-render projection has not returned. The distinction between "contradiction flagged awaiting user decision" and "contradiction resolved, superseded" is load-bearing for trust — DOS-318 is named but the visible states are not enumerated.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md:64:**Section cite:** §5.5 (W5), §8 (out-of-scope deferrals), surface-audit.md Active list MePage row.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:6:**Verdict:** APPROVE — all 6 cycle-1 gaps resolved. One observation (confidence 50, FYI-only, no AC required).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:10:## K-in record (cycle-2 re-confirm)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:12:**`docs/solutions/` grep:** Same files present as cycle-1. No new entries covering chrome lift, FolioBar, FloatingNavIsland, AtmosphereLayer, token-alias, or WP enqueue patterns. `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md` (lines 13-15, 32, 49-52) remains the only adjacent hit — WP PHPCS CI posture; pre-applied per §5.5. No reinvented documented substrate.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:14:**`.docs/decisions/` grep:** ADR-0077 tint table re-confirmed against V1.1 §5.5 forward-stub table (see Gap A below). ADR-0073/0076/0077/0129/0130 consumed. No new ADRs hit that cycle-1 missed. ADR-0077 line 64 ("Entity accent borders: larkspur (people/1:1)") confirms `dailyos_person → larkspur` assignment. ADR-0077 lines 52-56 confirm briefing/meeting/account → turmeric; actions → terracotta; weekly → larkspur. No canonical reference for a `project` tint exists in ADR-0077 — TBD-at-W2-L0 call is correct.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:134:## Additional cycle-2 scope
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:171:**Interaction state coverage: 9/10.** All cycle-1 gaps resolved. Every interactive element in the chrome (FolioBar action buttons, FloatingNavIsland pills, breadcrumb buttons) now has class-based hover/focus-visible rules specified or inherited from verbatim-lifted module CSS. Scroll-spy deferred explicitly. The 1-point gap: AC #27's idempotency guard uses a class-name sentinel that is implicitly locked by the §10 invariant but not verified by a CI gate — a static grep asserting the sentinel class exists in `FolioBar.module.css` would close it.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:175:**Unresolved design decisions: 9/10.** All cycle-1 "TBD" markers are resolved or explicitly deferred with a named future lane. The one remaining open item (`dailyos_project` tint) is correctly labeled TBD-at-W2 with an ADR-citation obligation. A 10 would have a Linear ticket filed to carry the W2 obligation.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md:183:All 6 cycle-1 gaps (A through F) are resolved in V1.1. Token values in Patch 9b are confirmed correct against the canonical `design-tokens.css`. The `post_type_exists` guard pattern is clean for the W2 handoff. No new design-blocking gaps introduced by V1.1. Two confidence-50 observations logged; neither requires an AC before L1 begins.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:1:# Codex Challenge — packet-W1-substrate-gaps cycle 1
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:3:**VERDICT: APPROVE WITH CHANGES** — DOS-701 reuse claim verifies on disk; IL checks have substance (not copy-paste); AC-W1.2 "wiring IS the work" gate is declared but mechanically unenforced — fixable with one CI script before W1 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:5:Scope: adversarial pass on 3 vectors (DOS-701 reuse claim, IL-check fluff hunt on §5.2/§5.8/§5.10, AC-W1.1/W1.2 enforcement). Architecture/correctness/CSO already covered in sibling cycle-1 packets.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:29:**Finding:** Disk audit confirms: `auth.rs` (12092B), `contracts.rs` (5517B), `render.rs` (11282B) all non-trivial. `boundary.rs`, `contradiction.rs`, `feedback.rs`, `privacy.rs`, `render_rules.rs` are 1-byte placeholders (correct per packet). However, line 892's LOC counts (319+290+175) don't match byte-size order (auth largest at 12KB, render next at 11KB, contracts smallest at 5.5KB). Either the LOC numbers are stale or the file/LOC mapping is mis-ordered. Doesn't change the reuse claim — DOS-701 substrate IS load-bearing — but the packet's specific LOC figures should match disk reality before L2.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:31:**Recommended fix:** Run `wc -l src-tauri/src/services/claim_receipt/{contracts,render,auth}.rs` and update line 892 with current LOC counts in matching order; re-verify before L0 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:39:**Finding:** §5.2 (DOS-460) and §5.10 (DOS-507) IL checks answer each Q with ticket-specific substance (ClaimType::OpenLoop, WrongSubject routing, DOS-278 candidate-set invalidation, DOS-8 routing). §5.8 (DOS-340) IL check is structurally thin: Q1 "no claims written", Q3 "no new signals", Q5 "N/A". This is defensible for a pure render-side boundary filter, but functionally close to a fail-shape under AC-W1.3 ("substrate that fails any question is incomplete"). The packet should explicitly justify that "N/A for feedback loop" is acceptable for boundary-only substrate, OR re-frame Q5 to point at "violations of this boundary feed source reliability negatively via DOS-8 if a surface accidentally exposes audit-only fields" — which IS a real feedback path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:41:**Recommended fix:** Re-author §5.8 Q5 to: "Boundary violations detected by the §5.8 CI lint or by user reports of leaked audit-only fields feed back as DOS-8 `WrongSubject`/`SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract." Keeps the IL-check honest and the CSO panel happy.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:57:DOS-701 reuse claim is real (vector 1). IL-checks on the 3 spot-checked sub-tickets are substantive, not boilerplate, with one thin section (§5.8) that should be re-authored (vector 2). The "wiring IS the work" acceptance gate (vector 3) is the only material gap — F1 is HIGH because the wave can mechanically pass all current AC criteria while shipping bare W1 producers, recreating exactly the failure mode the rule was written to prevent. Address F1 by adding AC-W1.9 + a CI script; F2–F4 are LOW polish items for L0 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:1:# codex challenge — v1.4.4 wave-plan L0 packet cycle 1
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:14:(a) **Substrate-coverage drift.** A WP block can render real substrate for the *happy path* and fall back to stubbed-empty for `needs_verification` / corrected/superseded / proposal-edited states. AC #W1 references DOS-462's visible-QA-state matrix for W2 but does not require it for W3/W4/W5 surfaces — a Daily Briefing block that has no error-state path passes the literal AC.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:48:**Fix:** add an invariant to §10: "C4 supersedes path-α offload when the substrate gap blocks a sub-L0 packet's *acceptance criteria*. Path-α applies only when the finding is hardening unrelated to a named AC. Cite both memories explicitly so reviewers don't pick." Add a worked example (e.g., "missing `inclusion_reason` for a chapter whose visible-QA-state matrix references subject-scope filtering = C4 reopen, not path-α").
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:52:## Finding 4 — W0 audit authority has no recovery procedure [MEDIUM]
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:56:**Concern:** Wave packet treats DOS-677's W0 audit as authoritative for "active" surface list. AC #W1 keys parity proof to "every surface on the W0 active list." If W0 misses a surface — reachable by deep-link, only via `cmd+k` palette, only when feature flag is on, only in dev builds — that surface gets no WP equivalent and no one notices until a user hits a dead route after W6 flag-flip (decision #5). The packet has no audit-miss recovery procedure.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:58:**Fix:** add to §5.0 an audit-completeness check: (a) W0 enumerates *all* `createFileRoute` entries in `src/router.tsx` not just nav-reachable, (b) W0 enumerates command-palette entries from `cmd+k` registry, (c) W0 names feature-flagged surfaces explicitly as a 5th list. Add to W6 (§5.6) a pre-flip dry-run procedure: scan Tauri shell for every routed component, cross-check against W0 active+inactive lists, fail W6 if any unaccounted-for route exists. Treats audit as falsifiable, not authoritative.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md:62:## Out of scope this cycle
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:1:# L2 (Diff) — /cso review — Wave v1.4.4 W1+W2 integrated, cycle 1
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:7:- **Scope:** W2 new write paths + audience-boundary surfaces only (W1 cycle-1 CSO findings already path-α'd to DOS-749 / DOS-750)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:16:One blocking finding: v245 migration repeats the same class of bug v244 just retroactively fixed (non-transactional multi-statement table rebuild). This is a literal regression against the L3 cycle-2 F3 K-out lesson captured at `migrations.rs:949-957`. Fix is two lines.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:60:* **Active-verification hook:** Add a regression test similar to v244's that opens two SQLite connections to a fixture DB, kicks off the v245 migration on connection A, and asserts connection B never observes "no such table: claim_feedback" during the migration window. If a class-level CI gate is desired (CLAUDE.md "same shape twice → audit the class, add a gate"), grep `src-tauri/src/migrations/*.sql` for the pattern `DROP TABLE\|DROP VIEW\|DROP INDEX` and require an enclosing `BEGIN IMMEDIATE; ... COMMIT;`. This would have caught v245 mechanically. Recommend tracking this gate as a Codebase Maintenance follow-up since it spans the migration substrate, not v1.4.4 W2 scope.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:66:## Items audited and cleared (no findings)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:72:* `FeedbackAction::MergeIntent` (10th variant) declared at `src-tauri/abilities-runtime/src/abilities/feedback.rs:80` with full doc-comment describing the typed-proposal-only semantics (no lifecycle ratchet, no trust impact, no repair queue).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:73:* Serde wire format `merge_intent` round-trips correctly; `feedback_action_serializes_only_ten_closed_values` and `merge_intent_round_trips_through_json` tests pass at `feedback.rs:418-454`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:75:* Render-policy uniqueness invariant preserved: `merge_intent_is_typed_proposal_no_lifecycle_or_trust_change` test at `feedback.rs:486-498` asserts the policy bucket; `every_action_has_distinct_render_policy` test at `feedback.rs:617-651` confirms `Default` policy is uniquely owned by MergeIntent (all 9 prior variants have distinct non-Default policies — verified by reading lines 254-372).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:93:* `list_accounts`: `#[ability(... allowed_actors = [User, Agent], mcp_exposure = None ...)]` at `list_accounts/mod.rs:31-45` — `Actor::McpClient` is correctly excluded from `allowed_actors`, and `mcp_exposure = None` keeps it out of MCP tool introspection. Test `agent_mcp_actor_is_denied` at `list_accounts/mod.rs:346` covers the deny path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:96:* Worth noting in the doc-comment at `list_accounts/mod.rs:11-14`: the rationale ("not yet exposed to agent-side MCP, and `mcp_exposure = None` keeps them out of MCP introspection") matches the cycle-2 CSO discipline applied to W1's list abilities. Consistent.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:129:After Finding 1 is fixed, this verdict converts to **APPROVE** for L2 cycle-1.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:135:This tool is not a substitute for a professional security audit. /cso is an AI-assisted scan that catches common vulnerability patterns — it is not comprehensive, not guaranteed, and not a replacement for hiring a qualified security firm. LLMs can miss subtle vulnerabilities, misunderstand complex auth flows, and produce false negatives. For production systems handling sensitive data, payments, or PII, engage a professional penetration testing firm. Use /cso as a first pass to catch low-hanging fruit and improve your security posture between professional audits — not as your only line of defense.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:1:# L3 Codex Challenge — v1.4.4 W1 Wave (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:4:**Diff:** `0f8533e1..ea05ddc6` (integrated wave through L2 cycle-3)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:7:## Verdict: 5 REAL DEFECTS — cycle-2 L3 patch required
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:33:`services/entity_intelligence/envelope_cache.rs:51` keyed by `envelope_render_id` only; no actor / surface / session / owner binding. Lookup at `envelope_cache.rs:144` by ID alone. Cache miss at `commands/claim_feedback.rs:114` falls back to single-claim envelope built from requested target — restoring the tautological check the cache was meant to remove. Command also hardcodes `RenderActor::user("user", None)` at `claim_feedback.rs:150`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:48:These are L3 wave-level integration defects. Cycle-2 L3 patch path per engineering ladder line 27 (pass rule "codex challenge + architect-reviewer approve"). Architect-reviewer cycle 1 was APPROVE; codex challenge needs cycle-2.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:50:Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop`: F1 + F4 share a class-pattern (cross-sub-ticket consistency assumptions). Folding into a single cycle-2 patch ticket would be appropriate, but distinct ACs make distinct patches cleaner.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:14:## 1. K-in audit (substrate-grep, before scoring)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:64:3. **The deferral is bounded and reversible.** Packet §8 explicitly names "Chrome-as-Gutenberg-blocks" as v1.4.5+ scope. The runtime-inject path doesn't foreclose the block path — once a working substrate-on-chrome reference exists, the translation has a target.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:141:| Font license compliance | Low | AC #10 LICENSES.md audit | Acceptable |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:149:**The three conditions are all docs/gates, not implementation rewrites. None block the lane's strategic shape; all can fold into cycle-1 or cycle-2 of L0 hardening without renumbering the wave.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:169:### Non-blocking suggestions (cycle-2 or maintenance, not L0 conditions)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:179:The packet is **strategically sound and tactically tight**. The lift is well-scoped (5 tiers in dep order, with tier 6/7 explicitly deferred), the trade-off is named not hidden, ADR consumption is correct and complete, file ownership respects the established theme/plugin split, and the K-in audit confirms no reinvention of documented substrate.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:183:**Verdict: CONDITIONAL APPROVE.** Cycle-2 folds the three conditions; cycle-2 then unanimous-APPROVE-able from this reviewer.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:1:# L2 (Diff) — gstack `/review` — v1.4.4 W1+W2 wave, cycle 1
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:7:**Scope:** W1 already L2/L3 unanimous at `e3907a63`; this cycle reviews W1 extensions (×3) + W2 surfaces (4 outer composites + 61 inner blocks + 3 list shells + 2 metadata-proposal blocks + 6 primitive folds)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:12:## VERDICT: **BLOCK** — 1 CRITICAL regression of L3 cycle-2 F3 fix (v245 lacks `BEGIN IMMEDIATE/COMMIT` wrapper that v244 was created to introduce)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:22:**Severity:** CRITICAL — same-shape defect class as L3 cycle-2 F3 (DOS-335 v244)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:29:**Why this isn't path-α:** This is a regression of an already-shipped fix (L3 cycle-2 F3 verdict was unanimous APPROVE on the v243→v244 patch at `e3907a63`). Per CLAUDE.md "Same-shape findings twice = class-wide sweep, not a third patch" — but this is the recurrence, the sweep IS to wrap v245 + audit v246+. Per memory `feedback_systemic_look_for_recurring_issue_classes` and `feedback_zoom_out_for_class_pattern_in_l2_loop`, this is the structural gate not a one-off patch.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:31:**Cross-model:** codex independent pass returns "**Recommendation: BLOCK because v245 reintroduces the exact untransactional destructive migration window v244 was created to close.**"
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:40:**Defect:** Cursor is base64-encoded JSON `{offset, watermark}` with no HMAC. Client can forge any `offset` value as long as `watermark` matches the current request fingerprint. The watermark check at `list_accounts/producer.rs:52` defends against filter-shape spoofing but not offset-skip. In local-only WP-surface deployment (v1.4.4 W2 is loopback HTTP per memory `feedback_wp_is_local_surface_not_remote`), the attacker model is "client app code injecting a forged cursor"; impact is pagination skip/seek, not row access (every row in scope of the filter is already authorized for the principal at the watermark).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:41:**Why path-α:** Code comment explicitly cites v1.4.6/v1.4.7 substrate work for HMAC signing. V1.1 W1 packet explicitly resolves this as not-a-W1-substrate-change. ADR-0123 / DOS-746 (per memory `feedback_canonical_signing_changes_invalidate_pairings`) is the substrate track for HMAC canonical signing changes.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:42:**Recommended fix:** File maintenance ticket against project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` titled "v1.4.6/v1.4.7 — HMAC-sign opaque pagination cursor (`list_pagination::encode_cursor`)". Reference DOS-746 substrate track. Cross-link from `list_pagination.rs:7-10` doc comment.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:57:## Wave-scope invariant audit (W2 §10 + V1.2 named gates)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:68:- Meeting `EntityKind` (`87df7cf6`) — extends `get_entity_intelligence` envelope; matched in `meeting-detail/render-functions.php:74-81`; no regression of the L3 cycle-2 F2 touchpoint audience filter at the consumer.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:78:1. **Fix F1 in one ~5-line edit** — wrap v245 body in `BEGIN IMMEDIATE; … COMMIT;` exactly as v244, update the misleading "Safe across multi-process readers" comment to point at the wrap. Commit message should explicitly cite "regression of L3 cycle-2 F3" to make the K-out searchable. Memory `feedback_zoom_out_for_class_pattern_in_l2_loop` says the third instance triggers a structural sweep — at the same edit, add a greppable CI gate (`grep -L 'BEGIN IMMEDIATE' migrations/*.sql | grep -E '(DROP TABLE|DROP VIEW|ALTER TABLE.*RENAME)'`) so v246+ cannot regress.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:79:2. **File F2 + F3 to maintenance project** `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per memory `feedback_l2_path_alpha_to_maintenance_project`. Reference: F2 = "HMAC-sign opaque list pagination cursor (v1.4.6/v1.4.7)", F3 = "Wrap-for-clarity audit of additive migrations v241/v242".
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:80:3. **Re-run this L2 cycle after F1 patch.** Expected outcome: cycle-2 APPROVE on the wave-integrated diff.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md:82:— gstack `/review` cycle-1, 2026-05-21
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md:1:# WP-skill-grounded reviewer — W2 Entity Surfaces L0 cycle 1
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md:7:## VERDICT: **REQUEST-CHANGES** (4 high-severity packet edits required before L0 close; 3 path-α items routed to maintenance)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md:65:- **L3.** No `viewScriptModule` declared for any block; consistent with read-only PHP-rendered approach. If DOS-689 EvidenceDrawer needs interactivity (drawer open/close), `viewScriptModule` lands at L1 — note in §5.7 sub-spec.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md:71:**Must edit before L0 close (H1–H4):** templateLock contract; envelopeHandle resolution; inline-style boundary gate; synced-vs-filesystem-patterns terminology.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:29:| **A1** | Digit-segment hyphenation audit | §5.1 audit block + §2 changelog | **YES** — audit confirms `spice-turmeric-10` etc. are alphanumeric slugs (single-digit segments after letters), not multi-digit numeric segments; emission is clean. AC #30 grep would surface future regressions. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:74:The "WP-only module-CSS overlay" path is genuinely closed. The §10 invariant is sound.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:76:**Verdict: invariant correctly forecloses drift. APPROVE.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:127:V1.2 resolves all 2 must-fix + 5 advisory items from cycle-2 cleanly with no regressions. All 4 cycle-3 stress-area probes return alignment with WP block-themes practice and skill-reference conventions:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:130:2. The "no WP-only module-CSS overlays" invariant correctly forecloses drift paths.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:136:From this reviewer's lens, the packet exits L0. Convergence to unanimous APPROVE depends on the remaining 4 cycle-3 reviewers (codex challenge, ce-architecture-strategist, codex consult, ce-design-lens-reviewer).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:10:**Active verification:** `cargo clippy -- -D warnings` (green); read paths exercised: `services/{meeting_prep_status,claim_receipt,entity_intelligence}/*`, `abilities-runtime/src/abilities/get_{entity_intelligence,daily_briefing}/*`, `commands/claim_feedback.rs`, `signals/policy_registry.rs`, migrations v241+v242, all four new lint scripts; grep audit of `wp/dailyos/blocks/**/render-functions.php` consumer references.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:35:**Recommended fix:** Add `src-tauri/scripts/check_w1_consumer_skeleton.sh` per the packet pattern (modeled on `check_claim_writer_allowlist.sh`); wire into `.github/workflows/lint-frontend.yml`. Alternatively: file a V1.2 packet amendment narrowing AC-W1.9 to a follow-up (cycle-1 path-α convergence under `feedback_l0_partial_convergence_when_class_recurs`).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:48:**AC cite:** AC-340.2 — "CI lint script `check_audit_disclosure_allowlist.sh` committed + green"; AC-340.7 — "CI lint `check_audit_denylist_completeness.sh` enforces"; AC-477.11 — references `check_sensitivity_gate_composition.sh` "by path."
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:49:**File:line:** Scripts exist at `src-tauri/scripts/check_audit_disclosure_allowlist.sh`, `:check_audit_denylist_completeness.sh`, `:check_sensitivity_gate_composition.sh`. Verified zero references from `.github/workflows/` or `.githooks/`. Precedent `check_ability_surface_drift.sh` IS wired at `lint-frontend.yml:114`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:53:bash src-tauri/scripts/check_audit_disclosure_allowlist.sh
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:54:bash src-tauri/scripts/check_audit_denylist_completeness.sh
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:71:**File:line:** `src-tauri/scripts/check_audit_disclosure_allowlist.sh:28` AND `:113`. Two scripts concatenated; second `set` line is harmless but suggests merge artifact. Also `:115` uses `REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"` which is brittle vs `git rev-parse --show-toplevel`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:75:### Finding 6 — LOW — DOS-340 — `audit_tables` regex drift between two scripts
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:79:**File:line:** `check_audit_disclosure_allowlist.sh:40` lists 5 tables including `legacy_user_note_migration_audit`; `check_audit_denylist_completeness.sh:37` lists 4. Two scripts that should share a source-of-truth diverge silently.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:81:**Recommended fix:** Path-α maintenance. Extract `audit_tables` to a shared sourced file (e.g., `src-tauri/scripts/_audit_tables.sh`) and `source` it from both lints.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:86:**AC cite:** AC-335.5 — "Manual entity linking… causes prep status to recompute or enqueue refresh through services/abilities (not UI-local state)"; AC-335.15 — signal emit within one invalidation cycle. Both met partially.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:87:**File:line:** `src-tauri/src/services/meeting_prep_status/write.rs:57-64`. `enqueue_refresh` only validates the `from → Queued` transition and emits the signal; no actual queue push (no `INSERT INTO meeting_prep_queue` or similar). Code comment at `:46-50` explicitly defers the queue plumbing to "Stage 1c when the meeting prep queue read API stabilizes." The signal emit DOES satisfy AC-335.15 (literal text: "dismissal write emits signal within 1 invalidation cycle").
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:95:**File:line:** `src-tauri/src/services/entity_intelligence/auth.rs:455` AND `src-tauri/src/services/claim_receipt/boundary.rs:52`. Both consts named `RECEIPT_ALLOWED_FIELDS` but operate on DIFFERENT field-name spaces (top-level `ClaimReceipt` JSON keys: `target/surfaceContext/renderedText/...` vs audit-row column names: `source_label/source_type/source_asof/...`). auth.rs comment at `:451-454` acknowledges the duplication but the consequence is two unrelated lists with the same name — invites future confusion.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md:124:- AC-341.11 — `OperationalAuditStorage` returns `PrivacyError::NonDisclosureAudience`; lint script enforces no `maintenance_audit` reads from `services::claim_receipt::*`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md:1:# Design-Lens Review — v1.4.4 W2 Entity Surfaces (cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md:17:2. **Corrected/superseded state on the originating chapter:** §5.6 specifies the proposal lifecycle in the drawer correctly. The post-accept visual state on the chapter that contained the original claim is unspecified — does it re-render silently on next fetch, or show a "correction applied" affordance?
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md:26:**Unresolved design decisions: 6/10** — it's a 6 because Q3 is marked "resolves at L0 close" but the specific display string and ARIA label are not locked in the packet, and the empty-state content pattern (described above) will surface as an unresolved question for each of the 48+ inner blocks at L1. Both are implementer-blocking.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md:44:§13 Q3 resolves to Option A (placeholder) with "Redacted touchpoint" as the proposed text, but the packet says "resolves at L0 close with /cso panel review." The display string and ARIA label are still open at packet-author time. Announcing "Redacted touchpoint" to a screen reader reveals that a touchpoint exists — which may be intentional under Option A's trust-contract rationale, but must be explicitly affirmed. The `dailyos/touchpoints-feed` inner block cannot be implemented without a locked string.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md:46:**Condition for APPROVE:** lock display string (e.g., "Redacted touchpoint" confirmed) and ARIA label (e.g., `aria-label="Touchpoint available; details redacted for this view"`) in the packet before L0 closes. `/cso` sign-off on the trust-contract rationale included in the same amendment.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:9:V1.1 is close enough for L1 shape, but not as-is. Two cycle-1 folds are under-specified, and the deferral/ticket hygiene needs to be closed before L1 kickoff.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:15:- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14-20` still controls W4: existing `surface_nonce.rs` + WP transport already cover nonce substrate; real W4 gap is `verify_nonce` -> `record_claim_feedback` + action expansion.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:21:- **C1 - APPROVED in V1.1.** §10 adds W4-coupling matrix, cites the K-in solution, and asserts disjoint write sets. Note: current `.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md` still contains V1.0 `surface_feedback_nonces` / `services::surface_feedback` scope, so W4 remains independently blocked until revised. W3 can proceed if it stays inside `wp/dailyos/theme/**`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:31:- **Patch 9 maintenance:** bounded but real. 9a idempotency is WP-overlay appropriate. 9b refresh-button class/CSS is canonicalizable UI behavior; file an upstream canonical-chrome ticket or move it upstream before the next sync cycle.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:33:- **W4 dependency:** if W4 cycle-2 does not fold the K-in correction, W4 should not merge. W3 is not blocked if its diff stays in theme paths and L2 rejects any cross-edit into W4 runtime/plugin/block paths.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:34:- **Pacing/L6:** this is not substantively blocked, but it is not converged. If the process treats these as cycle-2 clerical fixes, L6 is avoidable. If they require cycle-3 review, the packet's own "2 cycles without convergence => L6" rule applies.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:1:# L3 Wave Review — v1.4.4 W1 Substrate (architect-reviewer, cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:5:**Diff:** `0f8533e1..HEAD` (30 commits — 10 sub-tickets + 3 L2 cycle patches)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:10:L3 evaluates the integrated wave state as a whole. All six focus areas hold up architecturally. Two L2 cycle-3 P2 findings (DOS-8 idempotency race + lifecycle_changed accuracy) are correctly routed to path-α per James 2026-05-20 — they are not wave-architectural and do not block the L3 gate. No new wave-level findings.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:21:- `get_daily_briefing` (DOS-507) — Read/User-only ability with `BriefingState { availability, freshness, integrity, advisories }` composed shape per cycle-1 correctness F3 fold. NotExposed to MCP per L0 decision.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:31:- `MeetingPrepStatusChanged` at `:74` / `:201` / `:319` — pre-declared at W1 kickoff per cycle-1 arch F3 fold; policy fn at `:663` (`meeting_prep_status_changed_policy`).
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:49:| v243 | `243_meeting_prep_status_indexed_view_deterministic.sql` | DOS-335 (cycle-2 view determinism patch) |
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:90:- **Suite S (Security) — GREEN.** Three CI lint scripts pass: `check_w1_consumer_skeleton.sh`, `check_sensitivity_gate_composition.sh` ("All claim-sensitivity branching routes through the canonical gate"), `check_audit_disclosure_allowlist.sh` ("DOS-340: receipt-vs-audit disclosure allowlist clean"). `/cso` cycle-2 + cycle-3 APPROVE recorded at `wave-W1-l2-cso-cycle2.md`. DOS-749 / DOS-750 path-α tickets filed for cycle-3 P2 findings.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:100:The two L2 cycle-3 P2 findings (idempotency race + lifecycle_changed accuracy in DOS-8 feedback path) are correctly path-α; both are local correctness gaps in `services/claim_receipt/feedback.rs`, not contract violations, signal-propagation breakages, or boundary failures. The class-pattern sweep recommendation in the cycle-3 verdict (feedback-path edge cases) is the right structural response and belongs in maintenance, not in W1 cycle-N+1.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:104:## L3 close
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md:108:**Proceed to PR open + L4 (Surface) when W2 begins consuming.** L5 (Drift) checkpoint is not required at W1 close — drift risk is W2+ when surfaces start consuming the envelope.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md:1:# W2 Entity Surfaces — Architecture Review (L0 cycle 1)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md:63:**APPROVE.** All 3 findings are path-α inline edits to the V1.0 packet — none requires another full reviewer cycle. Author should:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md:64:1. Fold A1 + A2 + A3 as a V1.1 amendment (single commit) before unanimous L0 close.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md:65:2. Reviewer panel ratifies V1.1 by inspection; no second-cycle review needed for this reviewer.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:1:# L2 Codex Review — v1.4.4 W1 Wave (cycle 3)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:4:**Diff:** `0f8533e1..ea05ddc6` (full wave including cycle-1 + cycle-2 patches + cycle-3 P2 patches)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:15:### [P2] lifecycle_changed computed from verification state — `services/claim_receipt/feedback.rs:362`
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:17:`lifecycle_changed` compares the new verification state to the old verification state, so lifecycle-only actions like `mark_outdated`, `mark_false`, `wrong_subject`, or `needs_nuance` return `lifecycleChanged: false` even though claim_state/surfacing_state changed and the receipt should update. Track the lifecycle outcome from the writer or derive it from the action/result instead of reusing verification state.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:21:Per memory `feedback_l2_path_alpha_to_maintenance_project` + James's decision 2026-05-20: neither finding is a literal AC-8 violation. The 3 named L2 reviewers (`/review` + `code-reviewer` + `/cso`) all approved cycle-2; the engineering ladder pass rule is met. Both findings filed as Codebase Maintenance project tickets:
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:24:- Maintenance ticket #2: lifecycle_changed accuracy for lifecycle-only action variants
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md:28:L2 cycle 3 closes; proceed to L3.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:1:# W1 wave L2 — code-reviewer verdict (cycle 2)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:4:**Diff range:** `0f8533e1..HEAD` (cycle-2 patches: `0a586218` substrate + `c2e857b8` consumer skeletons + CI)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:12:All three cycle-1 HIGH findings (F1 / F2 / F3) are remediated. No new HIGH or MEDIUM findings surfaced against the cycle-2 patch surface. Path-α residuals from cycle-1 (F4–F6) are unchanged — already routed to maintenance per the cycle-1 verdict.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:42:The Tauri command now takes `envelope_render_id: Option<String>` and consults `envelope_cache::lookup_envelope_for_render` before constructing the `EnvelopeSet`. Cache hit ⇒ `CachedEnvelopeAdapter` carries the producer-recorded claim_ids + proposal_ids; cache miss ⇒ logged fallback to the cycle-1 single-claim envelope with explicit `path_alpha_envelope_cache_v2` marker symbol. The cycle-1 verdict explicitly named this as the acceptable remediation: "explicitly file AC-477.2 enforcement as a W2 dependency... the binding check is satisfied at the substrate layer but not at the Tauri command surface in v1.4.4 W1."
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:48:## New findings (cycle-2 patch surface)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:61:## Path-α residuals (carried from cycle-1, unchanged)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:67:All three remain path-α per cycle-1 routing. Not L2 blockers.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md:73:Per memory `feedback_l2_must_review_against_acceptance_criteria` + `feedback_l2_path_alpha_to_maintenance_project`: cycle-2 patches resolve the three literal AC violations from cycle-1 (AC-341.1/4/10/12, AC-W1.9 / AC-W1.2, AC-477.2 via option-b deferral). No PR-introduced regressions on cycle-2 patch surface. APPROVE.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-consult-cycle1.md:55:Three fixes (F1 + F2 + F3) all in §1 line 20 / §3 line 57 / §12 line 363 / §12 References. Mechanical edits, ~15 min. Re-issue cycle 1 verdict as APPROVE after the sha-range correction lands; the ADR cite set is otherwise clean.

codex
I’ve finished the packet pass. The remaining question is whether PR #347’s old review history contained a concrete finding that the new packet forgot, so I’m searching the repo for the DOS-168/W1-A review artifacts and the exact transport/security assumptions they recorded.
exec
/bin/zsh -lc "rg --files .docs docs | rg -i \"(v1\\.4\\.7|dos-168|mcp|347|w1-a|review|l0)\"" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
.docs/strategy/2026-04-20-persona-reviews.md
.docs/reviews/dos-546-w0-l2-cso-2026-05-11.md
.docs/reviews/dos-546-w1-e-1-l2-codex-cycle-3-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-codex-2026-05-11.md
.docs/reviews/dos-546-l0-cso-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cycle-2-revision-summary-2026-05-10.md
.docs/reviews/dos-546-w1-a0-l2-codex-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-2026-05-10.md
.docs/reviews/dos-546-w1-b-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-2026-05-10.md
.docs/reviews/dos-546-w1-e-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-e-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-2026-05-10.md
.docs/reviews/dos-546-w1-c-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-b-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-e-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-e-1-l2-codex-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cycle-3-revision-summary-2026-05-10.md
.docs/reviews/dos-546-w1-c-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-d-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-2026-05-10.md
.docs/reviews/dos-546-w1-d-l2-codex-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-2-2026-05-10.md
.docs/reviews/dos-546-w1-a-2-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-1-2026-05-10.md
.docs/reviews/dos-546-w1-c-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-cycle-2-2026-05-10.md
.docs/reviews/dos-546-w1-d-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-cycle-2-2026-05-10.md
.docs/reviews/dos-546-w1-b-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-l0-prep-codex-2026-05-10.md
.docs/reviews/dos-546-w1-c-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-cycle-3-2026-05-10.md
.docs/reviews/dos-546-w1-a-2-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-cycle-3-2026-05-10.md
.docs/reviews/dos-546-w1-a-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-architect-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-2026-05-10.md
.docs/reviews/dos-546-w1-d-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w1-a0-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-a0-l2-cso-2026-05-11.md
.docs/reviews/dos-546-w0-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w0-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-architect-cycle-2-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-3-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-3-1-2026-05-10.md
.docs/reviews/dos-546-w0-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w1-b-1-l2-codex-2026-05-11.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-2-2026-05-10.md
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
.docs/decisions/0027-mcp-dual-mode.md
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md
.docs/decisions/0063-meeting-preview-type-architecture.md
.docs/_archive/issues/_done/i347-done.md
.docs/proofs/v1.4.1/W5-mcp-bridge-retest.md
.docs/_archive/0063-meeting-preview-context.md
.docs/plans/v1.4.7-waves.md
.docs/research/2026-03-03-architecture-first-principles-review.md
.docs/plans/_archive/v1.0.0-review-fixes.md
.docs/plans/_patterns/reviewer-matrix.html
.docs/plans/wave-W6/DOS-412-mcp-static-surface-enumeration.md
.docs/plans/wave-W6/DOS-412-mcp-ability-data-enumeration.md
.docs/plans/wave-W2/reviews/DOS-259-l0-code-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer-v2.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer.md
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v3.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md
.docs/plans/storage-shape-review-2026-04-24.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-dx-review.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-code-reviewer.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-code-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-architect-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-codex-review-cycle1.output
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-D-DOS-469.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-A-DOS-466.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-B-DOS-467.md
.docs/plans/v1.4.5-workspace-memory/proof-bundle-W1-A.md
.docs/plans/v1.4.5-workspace-memory/L1-residuals-from-L0-cycle-5.md
.docs/design/reference/journeys/review-a-stakeholder.html
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle1.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle4.md
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-consult.output
.docs/plans/orchestration/routines/l0-reviewer.md
.docs/plans/claim-anatomy-review-2026-04-24.md
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle5/codex-challenge.output
.docs/_archive/mockups/_archive/account-intelligence-review-v2.html
.docs/plans/trust-topology-realignment/L0-packet-C1-adr-wave-retrofits.md
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md
.docs/plans/trust-topology-realignment/L0-packet.md
.docs/plans/trust-topology-realignment/L0-packet-C0-reviewer-scoping.md
.docs/plans/v1.4.1-waves/W7-A-L0-packet.md
.docs/plans/v1.4.1-waves/W6-A-L0-packet.md
.docs/plans/v1.4.1-waves/W6-F-L0-packet.md
.docs/plans/v1.4.1-waves/W5-A-L0-packet.md
.docs/plans/v1.4.1-waves/W5-C-L0-packet.md
.docs/plans/v1.4.1-waves/W5-B-L0-packet.md
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md
.docs/plans/v1.4.1-waves/W7-B-L0-packet.md
.docs/plans/v1.4.1-waves/W6-D-L0-packet.md
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md
.docs/plans/v1.4.1-waves/W7-C-L0-packet.md
.docs/plans/v1.4.1-waves/W7-D-L0-packet.md
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md
.docs/plans/v1.4.1-waves/W6-E-L0-packet.md
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-consult.output
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md
.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-A-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/reviews/codex-challenge.md
.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle4.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle5.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle5.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle4/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle5.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle3.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-consult.output
.docs/plans/dos-546/v1.4.2-project/W4-C-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-D-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle6.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle2.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle4.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle3.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle4.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle2.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle6.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle5/codex-challenge.output
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-codex-review-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle3/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle3/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle2.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle4/codex-challenge.output
.docs/_archive/mockups/claude-design-project/preview/folio-bar.html
.docs/_archive/mockups/claude-design-project/preview/colors-paper.html
.docs/_archive/mockups/claude-design-project/preview/colors-spice.html
.docs/_archive/mockups/claude-design-project/preview/type-display.html
.docs/_archive/mockups/claude-design-project/preview/margin-grid.html
.docs/_archive/mockups/claude-design-project/preview/pills.html
.docs/_archive/mockups/claude-design-project/preview/spacing-scale.html
.docs/_archive/mockups/claude-design-project/preview/avatars.html
.docs/_archive/mockups/claude-design-project/preview/schedule-row.html
.docs/_archive/mockups/claude-design-project/preview/colors-garden.html
.docs/_archive/mockups/claude-design-project/preview/brand-mark.html
.docs/_archive/mockups/claude-design-project/preview/nav-island.html
.docs/_archive/mockups/claude-design-project/preview/type-mono.html
.docs/_archive/mockups/claude-design-project/preview/colors-entity.html
.docs/_archive/mockups/claude-design-project/preview/colors-desk.html
.docs/_archive/mockups/claude-design-project/preview/type-scale.html
.docs/_archive/mockups/claude-design-project/preview/pull-quote.html
.docs/_archive/mockups/claude-design-project/preview/finis-marker.html
.docs/_archive/mockups/claude-design-project/preview/buttons.html
.docs/_archive/mockups/claude-design-project/preview/radius-shadow.html
.docs/design/reference/_shared/styles/TemplatePreview.module.css
.docs/_archive/dailyos-poc/dailyos/templates/agents/content/writer-authenticity-review.md

exec
/bin/zsh -lc "rg -n \"PR #347|#347|DOS-168|v1\\.4\\.7|W1-A|MCP v2|mcp_v2|nonce ledger|keyed-HMAC|transport HMAC|timing floor|replay|HMAC\" .docs docs src-tauri -S" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "git branch --all --list '*v1.4.7*' '*347*' '*mcp*'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
  dos-743-mcp-migrate
+ dos-758-mcp-handler-interface
+ v1.4.7-w1-foundation
  remotes/public/dos-758-mcp-handler-interface
  remotes/public/v1.4.7-w0-foundation
  remotes/public/v1.4.7-w1-foundation

 succeeded in 114ms:
src-tauri/tests/fixtures/bundle-18/expected_state.json:238:    "eval_replay_runs": [
src-tauri/tests/fixtures/bundle-18/expected_state.json:243:        "provider_replay_key": "bundle-18-v1-synthetic-fingerprint"
src-tauri/tests/fixtures/bundle-18/expected_state.json:249:        "provider_replay_key": "bundle-18-v1-synthetic-fingerprint"
src-tauri/tests/fixtures/bundle-18/provider_replay.json:3:  "provider": "replay",
src-tauri/tests/fixtures/bundle-18/provider_replay.json:4:  "model": "replay",
src-tauri/tests/fixtures/bundle-18/provider_replay.json:26:  "eval_replay": {
src-tauri/tests/fixtures/bundle-18/expected_output.json:78:      "provider_replay_key": "bundle-18-v1-synthetic-fingerprint",
src-tauri/tests/fixtures/bundle-11/metadata.json:27:    "provider_replay": "Pinned to the live prepare_meeting Transform canonical replay key for this claim-backed fixture."
src-tauri/tests/fixtures/bundle-18/metadata.json:4:  "invariant": "User-authored corrections, agenda items, notes, and dismissals survive refresh pressure; generated claim writes are monotonic through claim_version; optional child failures and offline stale state are explicit in provenance; coalesced invalidations and duplicate refreshes stay idempotent; eval replay is deterministic under injected clock, seed, and provider replay.",
src-tauri/tests/fixtures/bundle-18/metadata.json:34:  "pass_fail_definition": "PASS = all eight DOS-293 scenario ids are present; user correction survives the sequential enrichment attempt; older retry output is rejected with the persisted claim_version/generation unchanged and a generated_output_rejections row; agenda, notes, and dismissals are all preserved; daily readiness provenance uses OptionalComposedReadFailed for the optional child read failure; offline provenance warning carries class OfflineStale and stale_age_hours; coalescing preserves invalidations; duplicate refresh creates one claim and one commitment; eval outputs are canonical-JSON equal through the harness scoring path; bundle-18 is mandatory. FAIL = any user-authored row is missing, stale retry advances generation, partial failure is silent or uses new warning vocabulary, stale offline age is unnamed, coalescing drops invalidations, duplicate refresh produces duplicate claims or commitments, eval replay differs canonically, or bundle-18 is not mandatory.",
src-tauri/tests/fixtures/bundle-18/metadata.json:37:    "provider_replay": "Pins a deterministic provider completion for the fixture input. The validation assertions use provider replay only as part of the eval determinism contract and do not call live providers.",
src-tauri/tests/fixtures/bundle-18/metadata.json:38:    "external_replay": "Pins offline source age, coalesced invalidation, duplicate refresh, and partial child-read inputs with synthetic source ids.",
src-tauri/tests/fixtures/bundle-18/state.sql:238:CREATE TABLE eval_replay_runs (
src-tauri/tests/fixtures/bundle-18/state.sql:244:    provider_replay_key TEXT NOT NULL,
src-tauri/tests/fixtures/bundle-18/state.sql:257:    ('eval-determinism-canonical-equality', 'eval_replay_runs', 'canonical_json_equal', 'Two replay runs share clock, seed, provider replay, and canonical output.');
src-tauri/tests/fixtures/bundle-18/state.sql:696:INSERT INTO eval_replay_runs (
src-tauri/tests/fixtures/bundle-18/state.sql:697:    id, scenario_id, run_label, clock, seed, provider_replay_key, canonical_output_json
src-tauri/tests/fixtures/bundle-10/metadata.json:28:    "provider_replay": "Pinned to the live prepare_meeting Transform canonical replay key for this claim-backed fixture."

exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.7-waves.md | sed -n '1,260p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
     1	# v1.4.7 — MCP Server v2 (Abilities-First) — Wave Plan
     2	
     3	## Renumber Note (2026-05-19)
     4	
     5	File number unchanged. Internal cross-refs were updated on 2026-05-19 to match the 2026-05-17 Linear renumber: references to **v1.4.5** in this doc are **Workspace Memory** (was v1.4.6); references to **v1.4.6** are **Salience & Recommendations** (was v1.4.5). `v1.4.7 implementation gates on v1.4.5 merge` (workspace memory). Canonical map at `.docs/plans/wp-foundation-roadmap-reorientation.md` §"v1.4.x active sequence (post-renumber, 2026-05-17)".
     6	
     7	## WordPress Foundation Reorientation (2026-05-15)
     8	
     9	This wave plan was authored as if v1.4.7 is the greenfield MCP layer. Per [ADR-0129](../decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md) and the v1.4.2 WordPress foundation work, **v1.4.2 W3-C already ships a WordPress-mediated MCP server** via the WP MCP Adapter with a DailyOS ability allowlist + dedicated low-cap WP user + read-mostly defaults.
    10	
    11	**Reframed:** v1.4.7 is the **second MCP path** — direct headless MCP from runtime to Claude Desktop / Cursor / other agents WITHOUT WordPress in the loop. The two paths coexist: WP-mediated for blocks consuming substrate; headless direct for agents consuming substrate without a surface.
    12	
    13	**Coordination required (already filed):**
    14	- [DOS-624](https://linear.app/a8c/issue/DOS-624) — v1.4.7 W2-A `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` MCP tools wrap intelligence the v1.4.3 carve-out reserved as User-only. CSO-approved L0 amendment required before W2-A implementation; amendment must also coordinate with v1.4.2 W3-C MCP path to avoid two parallel MCP exposures of the same intelligence.
    15	
    16	**Surface-agnostic substrate** (per anchored decision #6): parity across agent / Claude Desktop / Cursor / WordPress / future surfaces. Block producers in v1.4.2+ should be invocable from MCP for headless agents; the same producer renders the WP block AND answers the MCP tool call. Producer/renderer split per ADR-0130 already supports this; v1.4.7 W1-A's ability-surface inventory should treat producer abilities as first-class MCP-exposed citizens (same allowlist discipline as v1.4.2 W3-C).
    17	
    18	**Tauri UI freeze** does not affect this version (v1.4.7 surfaces are headless agents, not Tauri UI).
    19	
    20	**No new full L0 panel** is required for the reorientation. DOS-624 captures the one substantive new L0 concern (CSO-approved actor exposure amendment). See [`.docs/plans/wp-foundation-roadmap-reorientation.md`](./wp-foundation-roadmap-reorientation.md).
    21	
    22	---
    23	
    24	## Cycle 2 amendments (2026-05-14)
    25	
    26	Cycle 1 codex verdicts: Challenge **BLOCK** (5 HIGH); Eng **BLOCK** (3 HIGH + 2 MED, 4 overlap); DX **APPROVE-WITH-CONDITIONS** (1 HIGH + 4 MED + 1 LOW); CSO **BLOCK** (1 HIGH + 3 MED + 1 LOW). Deduplicated to ~14 unique findings across 6 classes. The biggest finding is Class S (MCP client trust boundary) — the only finding requiring substantive new design; rest is propagation/cleanup.
    27	
    28	Cycle 2 changes (folded in below):
    29	
    30	**Class S — MCP client trust boundary defined.**
    31	1. **`Actor::McpClient` ownership assigned to W0** — `actor.rs` change shipped as part of the ADR-0102 amendment commit, not a separate lane.
    32	2. **MCP client authentication + session contract added to W0/W1-A.** Server-issued `McpClientId` after pairing handshake (mirrors v1.4.2 SurfaceClient pairing pattern); HMAC-SHA256 transport signing for stdio/HTTP MCP transports; scoped grants loaded server-side from a per-client manifest (NOT caller-provided in `granted_scopes`); revocation path; per-actor + per-client + per-tool rate-limit registry. Closes CSO HIGH #1.
    33	
    34	**Class A — Staging contradictions resolved (class-wide propagation sweep).**
    35	3. **W3 serialized.** Stage 3a = W3-A only; 3b = W3-B; 3c = W3-C (DOS-171 depends on W3-B per its own contract).
    36	4. **W4 serialized.** Stage 4a = W4-B only (DOS-169 has simplest deps); 4b = W4-A + W4-C in parallel after W4-B (W4-A depends on W2+W3+W4-B; W4-C depends on W4-B).
    37	
    38	**Class B — W1-A skeleton compile-readiness.**
    39	5. **`handlers/mod.rs` added.** Top-level `mod.rs` declares `pub mod handlers;`; `handlers/mod.rs` declares `pub mod tool_*;` for each handler. W1-A creates both files.
    40	6. **`when_NOT_to_call` renamed to `when_not_to_call`** (clippy snake_case); wire field stays uppercase via `#[serde(rename = "when_NOT_to_call")]`.
    41	7. **`ParamSpec`, `ReturnSpec`, `ToolExample`, `ParamSchema`, `Side` types defined** in the same `contracts.rs` snippet.
    42	
    43	**Class C — Scope naming convention frozen + reconciled with v1.4.5.**
    44	8. **Canonical convention: `<dot-namespace>.<verb>.<noun>`** — e.g., `dailyos.read.account_status`, `dailyos.write.place_document`, `dailyos.submit.note`. v1.4.5 already shipped `write.workspace_place_document` for the AI placement ability; v1.4.7 W3-B's MCP tool reuses that exact scope verbatim (no rename). Scope-mapping table added to W0. Closes Challenge HIGH #5 + L0 question #3.
    45	
    46	**Class D — DX contract sharpness.**
    47	9. **Cross-conversation continuity contract pinned.** Server-minted `OpaqueConversationHandle` REQUIRED on every MCP write tool; optional but recommended on reads. Closes DX HIGH #1 + CSO MED #3 (raw caller-provided IDs forbidden in audit/signal payloads).
    48	10. **Read tool naming convention sharpened.** Allow small verb-read class explicitly: `search`, `list`, `get`, `prepare` are verb-prefix-allowed for reads. Naming becomes `dailyos.<verb>.<noun>` (verb may be `read` for property-style reads or one of the four read-verbs). `dailyos.search_workspace_memory` → `dailyos.search.workspace_memory` for consistency with the dot-separator convention.
    49	11. **Shared `Page<T>` envelope** added to W1-A `contracts.rs`. v1.4.5 `WorkspaceGraphResponse v1` is the documented pass-through exception (since v1.4.7 W3-A surfaces it unchanged for AI consumers).
    50	12. **DOS-481 fixture density.** Minimum ≥2 positive + ≥2 negative per tool: one broad-corpus negative (web/enterprise search) + one adjacent-DailyOS-but-wrong-tool negative.
    51	
    52	**Class E — CSO privacy hardening.**
    53	13. **Keyed HMAC for audit hashes** — `params_hash` and `response_hash` are HMAC-SHA256 over canonical JSON with a per-install audit key. Closes CSO MED #2.
    54	14. **Opaque resource IDs** — `dailyos://account/{id}` `id` is a server-minted opaque handle, not the underlying entity_id. Closes CSO MED #4.
    55	15. **DOS-171 redaction helper** — DOS-171 reuses the same redaction helper DOS-479 ships, with the same scope matrix (`read.entity_names`, `read.file_paths`, `read.claim_text` — none of which v1.4.7 grants by default). Closes CSO LOW #5.
    56	
    57	Cycle 2 is heavy — 15 changes across 6 classes — but most are propagation/cleanup. The single substantive new design (Class S MCP client trust boundary) ships as a W0 deliverable. No wave shape changes; no migration slot changes (still v220–v239); no new ADRs beyond the already-planned ADR-0102 + ADR-0128 amendments (Class S folds into ADR-0102 amendment scope).
    58	
    59	## Cycle 3 amendments (2026-05-14)
    60	
    61	Cycle 2 codex verdicts: CSO **APPROVE-WITH-CONDITIONS** (5 CLOSED, 0 new); Challenge **BLOCK** (1 CLOSED, 4 OPEN — all class-A propagation gaps); Eng **BLOCK** (2 CLOSED, 4 OPEN/PARTIAL + 3 new MED); DX **REQUEST-CHANGES** (4 CLOSED, 2 PARTIAL + 3 new MED).
    62	
    63	Class A propagation hit AGAIN (third v1.4.7 cycle, sixth time across v1.4.5+v1.4.7 combined). Discipline failure on my part — when amending convention, I update the changelog/named-exceptions table but miss the lane-body text downstream.
    64	
    65	Cycle 3 changes:
    66	1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
    67	2. **W3 wave header serial language** (was "in parallel after"). Now matches §"Named exceptions" cycle 2 staging.
    68	3. **W4 wave header serial language** (was "Three agents in parallel"). Now matches §"Named exceptions" cycle 2 staging.
    69	4. **`contracts.rs` nested types now derive `Serialize, Deserialize`** — `Side`, `ParamSpec`, `ParamSchema`, `ReturnSpec`, `ToolExample`, `Page<T>`, `OpaqueConversationHandle`, `McpActor`, `McpClientId`, `ScopedName`, `Scope`. Snippet now compile-ready end-to-end.
    70	5. **Scope-check field reference fixed** (Eng new MED): security gate text now reads `tool.description().scopes_required.is_subset(actor.granted_scopes)` instead of the erroneous `actor.scopes_required`.
    71	6. **`OpaqueConversationHandle` lifecycle contract pinned** (DX new MED): explicit mint trigger, response envelope field, echo location in request envelope, 24h expiry, first-write behavior, revocation, "no raw conversation IDs accepted" rule. Public wire shape now reviewer-quality.
    72	7. **Scope namespace rule disambiguated** (DX new MED): v1.4.5-reused scopes stay unprefixed verbatim; new v1.4.7 scopes use `dailyos.<verb>.<noun>` prefix; manifest validation rejects unlisted aliases.
    73	8. **Cross-session continuity check added to release gate** (Eng new MED): release gate now requires both same-conversation AND new-session-with-prior-handle fixtures.
    74	
    75	Cycle 3 is pure propagation discipline + contract polish; no architectural change. Class-A discipline note added: **future amendment commits MUST grep every active reference to changed contract names/values BEFORE committing.** This is the third v1.4.7 cycle hit by the same class — done with patches; the discipline IS the work.
    76	
    77	## Cycle 4 amendments (2026-05-14)
    78	
    79	Cycle 3 codex verdicts: Challenge **BLOCK** (4 CLOSED + 3 NEW from class-A miss); Eng **REQUEST CHANGES** (5 CLOSED + 3 NEW); DX **BLOCK** (5 OPEN converging on same set as Challenge/Eng); CSO held APPROVE-WITH-CONDITIONS from cycle 2. All three cycle-3 lanes converged on the same 5-finding cluster: I claimed "all lane bodies use canonical names" in cycle 3 changelog item #1 then immediately violated it for `dailyos.place_document` in three locations. Plus 3 contradictions/typos that snuck in.
    80	
    81	Cycle 4 fixes (all tight, all surgical — grep-validated this time):
    82	
    83	1. **`dailyos.place_document` → `dailyos.write.place_document` everywhere** (W0 scope mapping table line 199; W3-B done-when line 548; release gate line 674). Closes Challenge/Eng/DX shared finding.
    84	2. **`dailyos.read.briefing` → `dailyos.read.daily_briefing` + `dailyos.read.meeting_briefing`** in W0 scope mapping table (the actual W2-A tool surface ships TWO briefing tools, not one). Closes Eng MED.
    85	3. **OpaqueConversationHandle first-write contradiction resolved** (Eng/DX HIGH cycle 3). Gateway now mints a handle transparently if absent on writes — no `ConversationRequired` error path. The error variant is removed from `ToolError`. Revoked handles still return `Unauthorized { missing_scope: "valid_conversation" }`. Reconciles gateway/lifecycle/W3-B security gate to the same single behavior: writes don't fail on missing handle; first-write mints.
    86	4. **W3-A new scope `read.workspace_source_provenance` → `dailyos.read.workspace_source_provenance`** per cycle 2 #8 namespace rule (new v1.4.7 scopes get the `dailyos.<verb>.<noun>` prefix; only v1.4.5-reused scopes stay unprefixed). Closes Eng MED.
    87	5. **`conversation_id` → `conversation_handle`** in architecture invariants table line 251 + `McpToolInvoked` signal payload line 421 + Eng-flagged audit references. Aligns audit/signal language with the cycle 3 lifecycle contract that forbids raw caller-provided conversation IDs. Closes Eng MED.
    88	
    89	Class-A discipline this cycle: I grep-audited BEFORE editing this time (commands: `grep -nE 'dailyos\.[a-z_]+(?<!\.read|\.write|\.submit)\.|...'`, `grep -n 'conversation_id'`, `grep -n 'ConversationRequired'`). All five fixes anchored at known line numbers. Commit will pass cycle 4 verification or class-A is no longer survivable as a discipline failure — at that point the doc needs a structural rewrite, not patches.
    90	
    91	Cycle 4 is contract polish only; no architectural change. CSO not re-dispatched (cycle 2 APPROVE-WITH-CONDITIONS still holds — cycle 3+4 changes are tool-name + lifecycle-contradiction reconciliation, no new trust-boundary surface).
    92	
    93	## Cycle 5 amendments (2026-05-14)
    94	
    95	Cycle 4 codex verdicts: Eng **APPROVE** (5 CLOSED, no new); Challenge **APPROVE** (5 CLOSED, no new); DX **BLOCK** (3 CLOSED, 1 OPEN + 1 new MED — both the same finding: `McpActor::Client.conversation` field name doesn't match lifecycle envelope name `conversation_handle`).
    96	
    97	Cycle 5 fix (one):
    98	1. **`McpActor::Client.conversation` → `McpActor::Client.conversation_handle`** — field name now matches the cycle 3 lifecycle contract that pins `conversation_handle` as the canonical envelope name on both request and response sides.
    99	
   100	Cycle 5 is a one-line rename; no semantic change. Final L0 cycle expected.
   101	
   102	## Cycle 6 amendment (2026-05-19) — W0 absorbs skeleton-creation scope
   103	
   104	W0 (this work, branch `v1.4.7-w0-foundation`) absorbed the `services/mcp_v2/` module-skeleton + `contracts.rs` content originally assigned to W1-A (see W1-A "Files owned" lines 333–460). The rationale is the v1.4.5-L0 lesson folded in at §"Lessons folded in from v1.4.5 L0" #2: pre-creating every cross-lane contract type and `pub mod` declaration in a single foundation PR prevents the trait-timing inversions that cost v1.4.5 cycles 3–6.
   105	
   106	**Ownership shift recorded here so W1-A's L0 reviewer reads the right boundary:**
   107	
   108	- W0 ships: top-level `mod.rs` with all submodule declarations; `contracts.rs` with full shared type set (`ToolDescription`, `ScopedName`, `Scope`, `Verb` enum + `CANONICAL_NAMESPACE` const, `ParamSpec` / `ReturnSpec` / `ToolExample` / `ParamSchema` / `Side` / `Page<T>` / `OpaqueConversationHandle` / `McpClientId` / `McpActor` / `McpToolHandler` trait / `ToolError` / `McpToolRequestEnvelope` / `McpToolResponseEnvelope` / `McpToolResult`); `gateway.rs` / `auth.rs` / `audit.rs` / `actor_policy.rs` / `taxonomy.rs` as empty doc-only placeholders; `handlers/mod.rs` + 10 per-tool handler files as empty placeholders.
   109	- W1-A ships: bodies of `gateway.rs` (dispatch, scope validation, audit emission, signal emission), `auth.rs` (pairing handshake, manifest loading, HMAC transport verification, conversation handle minting), `audit.rs` (HMAC-SHA256 keyed param/response hashing), `actor_policy.rs` (scope/permission matrix consumed by gateway), plus new migrations (v220–v222) and `SignalType::McpToolInvoked` registration.
   110	- W1-B unchanged: owns `taxonomy.rs` content + `src-tauri/resources/mcp_v2/tool_descriptions.yaml`.
   111	
   112	W0 also lands the wire-shape parity test suites for both layers (10 runtime parity tests in `abilities-runtime/src/abilities/registry.rs` + 25 wire-shape parity tests in `services/mcp_v2/contracts.rs`). W1-A's L2 will inherit these as the contract regression net.
   113	
   114	**Cycle 6 codex review surfaced four AC-bound mediums** (proc-macro `ActorArg::McpClient` parser arm; `Verb` enum + namespace const not encoded; `ToolError::Unauthorized` abusing `Scope` for auth-state sentinels; missing top-level request/response envelope types) — all four addressed in this same W0 branch before merge. Companion ADR-0102 §C / §D wording reconciled with the new `PairingRevoked` / `ConversationRevoked` `ToolError` variants.
   115	
   116	---
   117	
   118	## L0 prep posture (2026-05-14)
   119	
   120	Unlike v1.4.5 (which carried an approved project-level L0 verdict in the Linear description), v1.4.7's project description has no prior L0 approve. Cycle 1 of the L0 loop folds **project-scope review and wave-plan review into one pass** — codex Challenge / Eng / DX / CSO agents review both dimensions simultaneously against this packet. The packet is structured so each lane block names its project-scope rationale (mission-gate fit, scope-boundary justification) alongside its wave-scope contract (files owned, depends-on, done-when), so the same review cycle can clear both.
   121	
   122	## Lessons folded in from v1.4.5 L0 (10 cycles to converge)
   123	
   124	1. **Class-A propagation discipline.** When amending an API or relocating a contract, every active reference is updated in the same edit. v1.4.5 cycles 2/3/4/5 all hit "downstream propagation incomplete" findings; v1.4.7's draft pre-empts by grepping every named contract and its references in this initial draft.
   125	2. **Shared types in W1-A `contracts.rs` from day one.** Trait wave-timing inversions cost v1.4.5 cycles 3-6. v1.4.7 W1-A pre-defines every cross-lane contract in `services/mcp_v2/contracts.rs` and pre-creates the full submodule skeleton with `pub mod` declarations so subsequent lanes never edit `mod.rs`.
   126	3. **Frozen contract surfaces from cycle 1.** v1.4.5 spent cycles 8-10 on contract-sharpness amendments (DOS-474 idempotency key, DOS-489 query input, etc.). v1.4.7 pins typed input/output/error structs at cycle 1 to avoid the same back-and-forth.
   127	4. **Migration slot block reserved upfront.** v220-v239 (after v1.4.5's v200-v219 + buffer). v1.4.7 likely needs few migrations because it's a contract layer over existing substrate.
   128	5. **Cross-version dependency citations are explicit.** Every lane that consumes v1.4.5 substrate cites the exact frozen schema (`WorkspaceGraphProjection v1`, `PlaceDocumentInput v1`, etc.) rather than vague "uses v1.4.5 workspace memory."
   129	6. **Issue-body cleanup at W0.** Several v1.4.7 Linear issues still reference the pre-renumbering `v1.4.9 / v1.4.10` numbering (these are now v1.4.5 / v1.4.7 respectively). W0 close gate includes a cleanup pass.
   130	
   131	---
   132	
   133	**Date:** 2026-05-14
   134	**Target outcome:** Ship MCP v2 as a **co-equal head** over the same personal intelligence substrate as the Tauri app — abilities, claims, provenance, trust, lifecycle, AND the just-merged v1.4.5 workspace memory contracts. Host models (Claude Desktop, Cursor, ChatGPT) reach for DailyOS for the user's working understanding of their professional world (accounts, projects, people, meetings, sources, open loops) and explicitly NOT for broad corpus / web search jobs. Tool descriptions function as product copy.
   135	**Source of truth:** Linear project [v1.4.7 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9) + this wave plan at `.docs/plans/v1.4.7-waves.md`.
   136	**Predecessor projects/docs:**
   137	- v1.4.5 waves: `.docs/plans/v1.4.5-waves.md` — workspace memory substrate. **v1.4.7 implementation gates on v1.4.5 merge.** Specifically v1.4.7 consumes: `WorkspaceGraphProjection v1` schema (DOS-489 / W3-C), `PlaceDocumentInput / Receipt / Error` contract (DOS-474 / W4-C), `WorkspaceCategoryRegistry` + `WorkspaceCategory` enum (W1-A `contracts.rs`), `services::workspace_ingestion` mutation surface, `link::override_link` API (DOS-465 / W1-C), `pipeline::quarantine_source` API (W2-A), `WorkspaceFile { kind: WorkspaceFileKind }` `DataSource` variant (ADR-0107 amendment).
   138	- v1.4.0/v1.4.1 substrate: claim service (`commit_claim`), Signal Policy Registry, Durable Invalidation Job Model, trust scoring, freshness decay, provenance rendering, `services::feedback`, `services::actions`.
   139	- v1.4.2 SurfaceClient foundation: `SurfaceClient` actor variant + loopback transport (which DailyOS already uses for the Tauri+WP bridge; v1.4.7 MCP head sits beside the SurfaceClient as another consumer of the same ability runtime, not on top of it).
   140	- ADRs: 0080 (signals), 0098 (source-aware lifecycle), 0102 (abilities runtime — likely amended by v1.4.7), 0105 (trust scoring), 0107 (source taxonomy), 0108 (provenance rendering + privacy), 0125 (claim anatomy), 0128 (headless DailyOS — likely amended by v1.4.7), 0129 (composable surfaces).
   141	**Success metric:** Tool-description host-selection eval (DOS-481) shows the displacement use case working — when a host model has DailyOS + broad corpus tools available, it picks DailyOS for personal-working-context questions (accounts, projects, people, meetings, sources, open loops) AND explicitly avoids picking DailyOS for broad enterprise/web corpus questions. Plus E2E validation (DOS-482) on real workspace data confirms tool outputs respect v1.4.5 source lifecycle, sensitivity rendering, provenance, and signal/invalidation.
   142	**Migration slots reserved:** v220–v239 (20 slots). v1.4.5 holds v200–v219; v180–v199 is v1.4.2 W5/W6 expansion buffer; v176–v179 is DOS-589; v175 is W4-E; v171–v174 is v1.4.2 W4-C.
   143	
   144	---
   145	
   146	## Release thesis
   147	
   148	DailyOS already exposes an MCP server, but most of it is legacy: stale briefing snapshots, ad-hoc reads, no ability-backed write surface, no source-aware tools, no host-selection language. v1.4.7 closes that gap as a **co-equal head**, not an export pipe. Two layers of expansion:
   149	
   150	1. **Original MCP v2 expansion** (legacy issue set DOS-167–DOS-175): replace stale reads with ability-backed current context, add portfolio attention + ranked discovery + pagination, add controlled feedback/note/action writes through `services::*` with receipts and signal emission.
   151	2. **v1.4.5-enabled expansion** (new issue set DOS-478–DOS-482, DOS-479, DOS-480, DOS-171): expose the v1.4.5 source registry / lifecycle / claim proposals / graph projection / placement contract through MCP tools that respect privacy rendering and sensitivity rules. AI agents can search workspace memory, place documents, inspect provenance, and correct source attribution — all through the same service boundary the Tauri app uses.
   152	
   153	The non-trivial product-design constraint: **tool descriptions are the product surface**. A vague description ("query DailyOS data") makes Claude pick generic search tools. A sharp description ("for the user's working understanding of their professional world: accounts, projects, people, meetings, open loops; NOT for broad enterprise / web corpus search") creates the right boundary. v1.4.7 treats tool-description text as version-controlled product copy with eval coverage (DOS-481).
   154	
   155	---
   156	
   157	## What v1.4.0–v1.4.5 leave behind (consumed contracts)
   158	
   159	This section lists the EXACT frozen contracts v1.4.7 consumes. Citing them precisely is how we avoid v1.4.5 cycles 2-5's "downstream propagation incomplete" failure mode.
   160	
   161	**From v1.4.0/v1.4.1:**
   162	- `services::claims::commit_claim` (sole writer for `intelligence_claims`); `services::feedback`; `services::actions`; Signal Policy Registry; durable invalidation queue; trust scoring; freshness decay; provenance rendering substrate (ADR-0108).
   163	- `CLAIM_TYPE_REGISTRY` (`abilities-runtime/src/abilities/claims.rs::ClaimType`); `ClaimSensitivity` (ADR-0125).
   164	
   165	**From v1.4.2 (relevant subset; v1.4.7 does not consume the WordPress UI bits):**
   166	- `Actor::SurfaceClient` actor variant (`actor.rs`) — v1.4.7 introduces a sibling `Actor::McpClient` variant for MCP-originated calls; both consume the same ability runtime.
   167	- ADR-0130 composition contract is NOT consumed by v1.4.7 (no Gutenberg blocks shipping in v1.4.7).
   168	
   169	**From v1.4.5 (cited explicitly per consumer):**
   170	- `services::workspace_ingestion::contracts::WorkspaceCategory` enum + slug serialization (cycle 9 frozen).
   171	- `WorkspaceCategoryRegistry` (W1-B) — per-entity category allowlist + path resolution.
   172	- `WorkspaceGraphProjection v1` schema + `WorkspaceGraphQueryInput v1` request shape + `WorkspaceGraphResponse v1 { Projection | NotModified }` (W3-C frozen API; v1.4.7 W3 lanes consume).
   173	- `WorkspaceGraphAudit v1` JSON schema (W3-C; v1.4.7 W5 evaluation lane consumes).
   174	- `place_document` ability + `PlaceDocumentInput v1` (`category: Option<String>`) + `PlaceDocumentReceipt v1` (`category`, `resolved_path: Option<String>` gated by `read.entity_names`) + `PlacementError` typed enum (W4-C frozen API; v1.4.7 W3 DOS-480 lane consumes).
   175	- `IngestPipeline::run(IngestRequest)` (W2-A; v1.4.7 doesn't call directly — goes through `place_document` ability).
   176	- `services::workspace_ingestion::link::override_link(emitter: &dyn SignalEmitter, ...)` (W1-C; v1.4.7 may surface relink as an MCP write).
   177	- `services::workspace_ingestion::pipeline::quarantine_source(file_id, reason, actor)` (W2-A; v1.4.7 may surface quarantine as an MCP write).
   178	- New `SignalType` variants: `WorkspaceFileIngested`, `WorkspaceFileRejected`, `WorkspaceFilePendingEntityAssignment`, `WorkspaceFileQuarantined`, `WorkspaceFileEntityLinkChanged` (W3-B; v1.4.7 W3 invalidation behavior consumes).
   179	- `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` (ADR-0107 amendment merged in v1.4.5 W0; v1.4.7 surfaces this kind in graph projection responses).
   180	
   181	**v1.4.7 does NOT ship:**
   182	- New v1.4.5-style schema (workspace_ingestion is owned by v1.4.5; v1.4.7 only adds MCP-facing query / write tool layers).
   183	- New trust UI primitives, claim-receipt UI, lint mode (those are v1.4.4 scope).
   184	- Salience scoring / recommendations (v1.4.6 scope).
   185	- New `CLAIM_TYPE_REGISTRY` variants (consumes existing).
   186	- Bulk high-trust ingestion of AI-generated work product (explicit non-goal per project description).
   187	- External-system mutation beyond approved DailyOS abilities (explicit non-goal).
   188	
   189	---
   190	
   191	## Reading order
   192	
   193	1. This doc — wave assignments, merge gates, frozen contracts.
   194	2. v1.4.7 Linear project description — full scope + Definition of Done.
   195	3. Per-issue Linear ticket — frozen implementation contract.
   196	4. v1.4.5 wave plan — for the substrate v1.4.7 consumes; specifically the W3-C and W4-C lane sections that define the frozen schemas above.
   197	5. ADR-0128 (headless DailyOS — likely amended in W0) and ADR-0102 (abilities runtime — likely amended in W0).
   198	6. Shared Primitive and Service Ownership doc (Linear doc `c729429f-79c9-4131-a243-5500d6e2a605`) — applies to every issue.
   199	7. Prior wave proof bundles when working after Wave 0.
   200	
   201	---
   202	
   203	## Review ladder for v1.4.7
   204	
   205	The L0–L6 ladder from v1.4.0-waves.md applies. v1.4.7-specific amendments:
   206	
   207	| Level | Applies | Notes |
   208	|---|---|---|
   209	| **L0 Prep (Plan Hardening)** | Every issue + project-scope | `/plan-eng-review` default. `/plan-devex-review` mandatory for **every issue** (this is an entirely DX-facing project — every tool is consumer-facing). `/cso` mandatory for the gateway lane (DOS-168), all write-tool lanes (DOS-167, DOS-169, DOS-170), and the v1.4.5-substrate-consuming lanes that surface workspace data (DOS-171, DOS-479, DOS-480). `/plan-ceo-review` for DOS-478 (taxonomy + host-selection language is product strategy). |
   210	| **L0 (Plan)** | Every issue + project-scope | Codex challenge + domain reviewer + Codex consult. `/cso` required for security-annotated lanes. |
   211	| **L1 (Self)** | Every issue | Standard. |
   212	| **L2 (Diff)** | Every issue | **L2 runs locally before push.** Codex review + code-reviewer + `/plan-devex-review` (every diff is a DX surface). `/cso` re-review for security lanes. |
   213	| **L3 (Wave)** | W1, W3, W4 (heaviest contract or write surfaces) | Codex challenge + architect-reviewer + Suite S/P/E. |
   214	| **L4 (Surface)** | W5 — host-model selection eval (DOS-481) | Tool-description text reviewed as product copy by `/plan-devex-review`; eval against fixture host-model prompts. |
   215	| **L5 (Drift)** | W5 release close | `/plan-eng-review` + architect-reviewer against v1.4.7 DoD AND v1.4.5 substrate-consumer parity. |
   216	| **L6 (Human)** | If escalated | Same triggers as v1.4.x. |
   217	
   218	Security-annotated lanes (carry `/cso` at L0 plan AND L2 diff): DOS-168 (gateway + actor policy), DOS-167 (note/observation through claim/source services), DOS-169 (action creation), DOS-170 (action status update), DOS-171 (privacy-rendered resources), DOS-479 (workspace memory search — sensitivity boundary), DOS-480 (work-product placement — write boundary).
   219	
   220	DX-mandatory lanes (every lane carries `/plan-devex-review` because the entire project is a DX surface, but these have heightened DX scrutiny): DOS-478 (taxonomy + host-selection contract), DOS-481 (tool-description eval), DOS-175 (briefing replacement — change in observable MCP behavior), DOS-172 (pagination/ranking — pagination contract is a permanent shape), DOS-173 (portfolio attention — net-new tool surface).
   221	
   222	CEO-review lane: DOS-478 — the tool-taxonomy + host-selection contract IS the product-strategy artifact. `/plan-ceo-review` validates the displacement use case framing.
   223	
   224	---
   225	
   226	## Migration slot assignments
   227	
   228	Reserved block: **v220–v239** (20 slots). Allocation guidance — most v1.4.7 lanes need NO migration (they're contract / tool-handler / eval layers over existing substrate); the few that do are listed below.
   229	
   230	| Wave | Likely slots | Issues that may need migrations |
   231	|---|---|---|
   232	| W1 — Contract & Policy | v220–v224 | DOS-168 gateway tables (actor attribution audit log; tool-call ledger if needed) |
   233	| W2 — Ability-Backed Reads | (none expected) | DOS-172/173/175 are read-side; no schema change |
   234	| W3 — Workspace Memory Tools | (none expected) | All consume v1.4.5 contracts; no new schema |
   235	| W4 — Controlled Feedback & Writes | v225–v229 | DOS-167 note capture if a new claim-source kind is needed; DOS-169/170 if action-attribution audit needs new columns |
   236	| W5 — Evaluation & Release Gate | (none expected) | Eval fixtures live in tests, not migrations |
   237	
   238	If a wave needs more slots than allocated, the wave coordinator pauses and claims from the unassigned reserve (v230–v239) — never silently borrows from adjacent blocks.
   239	
   240	---
   241	
   242	## Scope cleanup before kickoff (W0)
   243	
   244	Linear hygiene items to resolve before W0 closes:
   245	
   246	1. **Issue-body renumbering cleanup.** Several v1.4.7 issues still reference the pre-renumbering `v1.4.9 / v1.4.10` taxonomy: DOS-167, DOS-168, DOS-170, DOS-171, DOS-172, DOS-173, DOS-175, DOS-479, DOS-480, DOS-481, DOS-482 all contain `v1.4.9` (now v1.4.5) and/or `v1.4.10` (now v1.4.7) references in their bodies. W0 includes a one-pass edit on each issue body to substitute current numbering. Doesn't change scope; eliminates a class of reviewer confusion.
   247	
   248	2. **Cross-version dependency confirmation — v1.4.5.** Confirm v1.4.5 has merged to `dev` with these specific contracts live and frozen: `WorkspaceGraphProjection v1` + `WorkspaceGraphQueryInput v1` + `WorkspaceGraphResponse v1` (W3-C); `PlaceDocumentInput v1` + `PlaceDocumentReceipt v1` + `PlacementError` typed enum (W4-C); `WorkspaceCategoryRegistry` + `WorkspaceCategory` enum slug serialization (W1-A `contracts.rs`); `pipeline::quarantine_source` (W2-A); `link::override_link(&dyn SignalEmitter, ...)` (W1-C); five new `WorkspaceFile*` SignalType variants + invalidation allowlist additions (W3-B); ADR-0107 `WorkspaceFile { kind: WorkspaceFileKind }` `DataSource` variant. **If any item is not yet merged, v1.4.7 implementation is blocked; document the gap on the affected v1.4.7 issue.**
   249	
   250	3. **ADR amendments needed.** Two:
   251	   - **ADR-0102 (abilities runtime) — expanded scope per cycle 2 Class S.** Covers: (a) `Actor::McpClient` enum variant added to `actor.rs` (the actual code change ships in this same PR — not deferred to W1-A); (b) MCP client authentication + session contract: `McpClientId` issued server-side after pairing handshake, HMAC-SHA256 transport signing, server-side scope manifest (no caller-provided granted scopes), revocation path, per-actor + per-client + per-tool rate-limit registry; (c) scope naming convention pinned (see #4); (d) `mcp_exposure` tri-state `None | MetadataOnly | Invocable` confirmed sufficient (no new variants needed).
   252	   - **ADR-0128 (headless DailyOS)** — for v2 expansion principles: tool-description as product copy, host-selection language, cross-conversation continuity contract (server-minted `OpaqueConversationHandle`, REQUIRED on writes, recommended on reads).
   253	   Both amendments drafted, reviewed, and merged before W1 L0 plans open. **The ADR-0102 amendment PR also lands the `Actor::McpClient` code change in `actor.rs`** — closes Eng cycle 1 HIGH #1 ownership gap. Resolves L0 questions #1, #2, #3, #6.
   254	
   255	4. **Scope naming convention frozen (cycle 2 amendment closes Challenge HIGH #5 + L0 question #3).** v1.4.7 adopts `<dot-namespace>.<verb>.<noun>` convention. v1.4.5's already-shipped `write.workspace_place_document` scope is kept verbatim — v1.4.7 W3-B's `place_document` MCP tool reuses it without rename. Scope-mapping table:
   256	
   257	   | v1.4.5 scope (already shipped) | v1.4.7 reuse / extension |
   258	   |---|---|
   259	   | `write.workspace_place_document` | reused verbatim by `dailyos.write.place_document` MCP tool (W3-B) |
   260	   | `read.workspace_graph` | reused verbatim by `dailyos.search.workspace_memory` (W3-A) |

exec
/bin/zsh -lc "git ls-tree -r --name-only v1.4.7-w1-foundation | rg -i \"(v1\\.4\\.7|DOS-168|mcp|review|l0|347|w1)\"" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
.docs/_archive/0063-meeting-preview-context.md
.docs/_archive/dailyos-poc/dailyos/templates/agents/content/writer-authenticity-review.md
.docs/_archive/issues/_done/i347-done.md
.docs/_archive/mockups/_archive/account-intelligence-review-v2.html
.docs/decisions/0027-mcp-dual-mode.md
.docs/decisions/0063-meeting-preview-type-architecture.md
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md
.docs/design/reference/_shared/styles/TemplatePreview.module.css
.docs/design/reference/journeys/review-a-stakeholder.html
.docs/plans/_archive/v1.0.0-review-fixes.md
.docs/plans/_patterns/reviewer-matrix.html
.docs/plans/claim-anatomy-review-2026-04-24.md
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-A-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-B-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-C-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-D-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md
.docs/plans/dos-647/dos-647-l0-plan.md
.docs/plans/orchestration/routines/l0-reviewer.md
.docs/plans/storage-shape-review-2026-04-24.md
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md
.docs/plans/trust-topology-realignment/L0-packet-C0-reviewer-scoping.md
.docs/plans/trust-topology-realignment/L0-packet-C1-adr-wave-retrofits.md
.docs/plans/trust-topology-realignment/L0-packet.md
.docs/plans/v1.4.1-waves/W5-A-L0-packet.md
.docs/plans/v1.4.1-waves/W5-B-L0-packet.md
.docs/plans/v1.4.1-waves/W5-C-L0-packet.md
.docs/plans/v1.4.1-waves/W6-A-L0-packet.md
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md
.docs/plans/v1.4.1-waves/W6-D-L0-packet.md
.docs/plans/v1.4.1-waves/W6-E-L0-packet.md
.docs/plans/v1.4.1-waves/W6-F-L0-packet.md
.docs/plans/v1.4.1-waves/W7-A-L0-packet.md
.docs/plans/v1.4.1-waves/W7-B-L0-packet.md
.docs/plans/v1.4.1-waves/W7-C-L0-packet.md
.docs/plans/v1.4.1-waves/W7-D-L0-packet.md
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md
.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-L2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-cycle2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-codex-consult.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-code-reviewer.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-codex-challenge.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-cso.md
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-dx-review.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-correctness-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-architecture-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-challenge-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-wp-skill-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-challenge-cycle4.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-design-lens-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-architecture-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-codex-consult-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-design-lens-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-wave-wp-skill-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-code-reviewer-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-codex-review-cycle3.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-architect-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle2.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-code-reviewer-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-codex-review-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-gstack-review-cycle1.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-A-DOS-466.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-B-DOS-467.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-D-DOS-469.md
.docs/plans/v1.4.5-workspace-memory/L1-residuals-from-L0-cycle-5.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-architect-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-code-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/l2-reviews/l2-codex-review-cycle1.output
.docs/plans/v1.4.5-workspace-memory/proof-bundle-W1-A.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle1.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle2.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle3.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle4.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-architect-reviewer-cycle5.md
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle2.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle4.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle5.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle6.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle1.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle2.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle3.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle4.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle5.output
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle6.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle3/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle3/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle4/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle5/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-consult.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle4/codex-challenge.output
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle5/codex-challenge.output
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md
.docs/plans/v1.4.7-waves.md
.docs/plans/wave-W1-v147/proof-bundle.md
.docs/plans/wave-W1/DOS-306-contract.md
.docs/plans/wave-W1/DOS-310-plan.md
.docs/plans/wave-W1/DOS-311-plan.md
.docs/plans/wave-W1/W1-B-channel-inventory.md
.docs/plans/wave-W1/proof-bundle.md
.docs/plans/wave-W1/provider-trait-frozen.md
.docs/plans/wave-W1/retro.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer.md
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v3.md
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer-v2.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-architect-reviewer.md
.docs/plans/wave-W2/reviews/DOS-259-l0-code-reviewer-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v2.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult-v3.md
.docs/plans/wave-W2/reviews/DOS-259-l0-codex-consult.md
.docs/plans/wave-W6/DOS-412-mcp-ability-data-enumeration.md
.docs/plans/wave-W6/DOS-412-mcp-static-surface-enumeration.md
.docs/proofs/v1.4.1/W5-mcp-bridge-retest.md
.docs/proofs/v1.4.2/W1-proof-bundle.md
.docs/research/2026-03-03-architecture-first-principles-review.md
.docs/reviews/dos-546-l0-cso-2026-05-10.md
.docs/reviews/dos-546-l0-prep-codex-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-cycle-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-architect-cycle-3-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-1-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-challenge-cycle-3-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-3-1-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-codex-consult-cycle-3-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-cycle-2-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cso-cycle-3-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cycle-2-revision-summary-2026-05-10.md
.docs/reviews/dos-546-v1.4.2-plan-l0-cycle-3-revision-summary-2026-05-10.md
.docs/reviews/dos-546-w0-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w0-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w0-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w0-l2-cso-2026-05-11.md
.docs/reviews/dos-546-w1-a-2-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-a-2-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-architect-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-a-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-a0-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-a0-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-a0-l2-cso-2026-05-11.md
.docs/reviews/dos-546-w1-b-1-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-b-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-b-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-b-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-c-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-c-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-c-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-c-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w1-d-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-d-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-d-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-d-l2-codex-cycle-2-2026-05-11.md
.docs/reviews/dos-546-w1-e-1-l2-codex-2026-05-11.md
.docs/reviews/dos-546-w1-e-1-l2-codex-cycle-3-2026-05-11.md
.docs/reviews/dos-546-w1-e-l2-architect-2026-05-11.md
.docs/reviews/dos-546-w1-e-l2-code-reviewer-2026-05-11.md
.docs/reviews/dos-546-w1-e-l2-codex-2026-05-11.md
.docs/strategy/2026-04-20-persona-reviews.md
.github/actions/l3-reviewer-job/action.yml
.github/reviewer-prompts/accessibility-tester.md
.github/reviewer-prompts/architect-reviewer.md
.github/reviewer-prompts/code-reviewer.md
.github/reviewer-prompts/l3-architect-reviewer.md
.github/reviewer-prompts/l3-codex-challenge.md
.github/reviewer-prompts/matrix.yml
.github/reviewer-prompts/performance-engineer.md
.github/reviewer-prompts/security-auditor.md
.github/workflows/l2-review.yml
.github/workflows/l3-review.yml
.playwright-mcp/console-2026-04-23T13-28-54-532Z.log
.playwright-mcp/console-2026-05-04T11-56-15-778Z.log
.playwright-mcp/console-2026-05-04T11-56-54-754Z.log
.playwright-mcp/page-2026-04-23T13-28-57-053Z.yml
.playwright-mcp/page-2026-05-04T11-56-16-081Z.yml
.playwright-mcp/page-2026-05-04T11-56-55-030Z.yml
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
plugins/dailyos-writer/commands/review.md
plugins/dailyos-writer/skills/authenticity-review/SKILL.md
plugins/dailyos-writer/skills/mechanical-review/SKILL.md
plugins/dailyos-writer/skills/structural-review/SKILL.md
plugins/dailyos-writer/skills/templates/status-reports/quarterly-review.md
plugins/dailyos-writer/skills/voice-review/SKILL.md
plugins/dailyos/.mcp.json
src-tauri/abilities-runtime/tests/fixtures/meeting_claims_for_review_integration_fixture.rs
src-tauri/resources/mcp_v2/tool_descriptions.yaml
src-tauri/scripts/build-mcp.sh
src-tauri/scripts/check_mcp_tool_handler_allowlist.sh
src-tauri/scripts/check_w1_consumer_skeleton.sh
src-tauri/scripts/check_w1_consumer_skeleton.sh.test
src-tauri/src/bridges/mcp.rs
src-tauri/src/mcp/main.rs
src-tauri/src/mcp_v2/main.rs
src-tauri/src/migrations/114_account_stakeholders_review_queue_idx.sql
src-tauri/src/migrations/240_claim_review_deferrals.sql
src-tauri/src/migrations/255_mcp_client_manifest.sql
src-tauri/src/migrations/256_mcp_conversation_handle.sql
src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql
src-tauri/src/migrations/258_mcp_rate_limit_and_audit_outbox.sql
src-tauri/src/services/claim_review_queue/candidates.rs
src-tauri/src/services/claim_review_queue/mod.rs
src-tauri/src/services/claim_review_queue/queue.rs
src-tauri/src/services/mcp_v2/actor_policy.rs
src-tauri/src/services/mcp_v2/audit.rs
src-tauri/src/services/mcp_v2/auth.rs
src-tauri/src/services/mcp_v2/contracts.rs
src-tauri/src/services/mcp_v2/gateway.rs
src-tauri/src/services/mcp_v2/handlers/mod.rs
src-tauri/src/services/mcp_v2/handlers/registration.rs
src-tauri/src/services/mcp_v2/handlers/tool_account_status.rs
src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs
src-tauri/src/services/mcp_v2/handlers/tool_create_action.rs
src-tauri/src/services/mcp_v2/handlers/tool_note.rs
src-tauri/src/services/mcp_v2/handlers/tool_pagination.rs
src-tauri/src/services/mcp_v2/handlers/tool_placement.rs
src-tauri/src/services/mcp_v2/handlers/tool_portfolio.rs
src-tauri/src/services/mcp_v2/handlers/tool_resources.rs
src-tauri/src/services/mcp_v2/handlers/tool_update_action_status.rs
src-tauri/src/services/mcp_v2/handlers/tool_workspace_search.rs
src-tauri/src/services/mcp_v2/handlers/tool_workspace_source_provenance.rs
src-tauri/src/services/mcp_v2/mod.rs
src-tauri/src/services/mcp_v2/taxonomy.rs
src-tauri/src/services/mcp_v2/transport.rs
src-tauri/tests/claim_receipt_boundary/mcp__confidential.json
src-tauri/tests/claim_receipt_boundary/mcp__internal.json
src-tauri/tests/claim_receipt_boundary/mcp__public.json
src-tauri/tests/claim_receipt_boundary/mcp__user_only.json
src-tauri/tests/claim_receipt_privacy/agent_mcp__internal__risk.json
src-tauri/tests/dos168_mcp_v2_migration_smoke_test.rs
src-tauri/tests/dos412_mcp_ability_data_redaction_test.rs
src-tauri/tests/dos412_mcp_static_surface_test.rs
src-tauri/tests/mcp_hybrid_handler_test.rs
src-tauri/tests/w05_a_mcp_dynamic_tool_readers_test.rs
src-tauri/tests/workspace_ingestion_w1_migrations.rs
src/components/entity/TemplatePreview.module.css
src/components/entity/TemplatePreview.tsx
wp/dailyos/blocks/meeting-claims-for-review/block.json
wp/dailyos/blocks/meeting-claims-for-review/render-functions.php
wp/dailyos/blocks/meeting-claims-for-review/render.php
wp/dailyos/includes/mcp/class-dailyos-mcp-audit.php
wp/dailyos/includes/mcp/class-dailyos-mcp-permission.php
wp/dailyos/includes/mcp/class-dailyos-mcp-roles.php
wp/dailyos/includes/mcp/class-dailyos-mcp-server.php
wp/dailyos/tests/mcp/McpExposureNoneTest.php
wp/dailyos/tests/mcp/McpPermissionTest.php
wp/dailyos/tests/mcp/RoleCapabilitiesDriftTest.php
wp/dailyos/tests/mcp/ScopeSetDriftTest.php
wp/dailyos/theme/assets/styles/reference/TemplatePreview.module.css

exec
/bin/zsh -lc 'git log --oneline --decorate --max-count=30 v1.4.7-w1-foundation' in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
c288d1f6 (public/v1.4.7-w1-foundation, v1.4.7-w1-foundation) docs(dos-647): V1 L0 packet (BLOCKED) + split summary
830518be docs(dos-647): session handoff capturing substrate audit + Track A/B scope
10aecb28 fix(mcp_v2): request-scoped dispatch + live readers + version pre-read
9e5009d7 feat(mcp_v2): W2-A Phase-A — wire account_status MCP tool to abilities-runtime
9a4c84c7 merge: public/dev into v1.4.7-w1-foundation — v1.4.4 W1 + v1.4.5 W1 landed
f9b286c4 Merge pull request #346 from jamesgiroux/wave/v1.4.4-w1-stage1a
6349f2de (public/wave/v1.4.4-w1-stage1a) fix(tests/lint): satisfy PHPCS — docblocks + capitalized short descriptions
08c241ae chore: strip ephemeral issue refs from comments — batch 5 (7 files)
28996ae6 chore: strip ephemeral issue refs from comments — batch 4 (13 files)
1351091f chore: strip ephemeral issue refs from comments — batch 3 (11 files)
d9d91ab4 chore: strip ephemeral issue refs from comments — batch 2 (10 files)
53da091d chore: strip ephemeral issue refs from comments — batch 1 (13 files)
414ef085 chore(lint): regenerate ability inventory + de-ephemeral allowlist comment
faa3908a wire block kit fixture registry
36005c22 add block kit fixture batch five
0b747659 add block kit fixture batch four
200bd8b7 add block kit fixture batch three
fd76ab90 add block kit fixture batch two
e6865d17 add block kit fixture batch one
ff786f77 test(wp/wave-l1-fix): align PHPUnit + ability-allowlist with substrate trim
731620dc fix(theme/empty-chip): force display:block on --empty wrappers + quiet pill styling
25bbec93 chore(wave/v1.4.4 stage1a): trust-topology trim — auto-derived default scopes + reviewer-prompt scoping
14509784 Map account timeline finishers to reference classes
9539b932 Map account record blocks to reference classes
1e96090a Map evidence account blocks to reference classes
f7c27c58 Merge remote-tracking branch 'public/dev' into wave/v1.4.4-w1-stage1a
5b6abffa Map health blocks to reference classes
2e4235ba Merge pull request #349 from jamesgiroux/wave/v1.4.5-w2-l0
6b951c3c feat(blocks/account-hero): canonical class rewire — test case for visual parity sweep
23918ae1 feat(v1.4.5 W2 L0): V1.4 packets + cycle-5 partial convergence with L1 residuals

exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md | nl -ba | sed -n '1,260p'" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md | nl -ba | sed -n '1,260p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
     1	# v1.4.7 — MCP v2 Transport Ingress (rmcp ServerHandler → Gateway) — L0 plan packet
     2	
     3	**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
     4	**Linear ticket:** TBD (mint after L0 approve)
     5	**Scope:** local-to-local same-machine only. No remote MCP transport in scope. Trust boundary is "same user on this machine."
     6	**Authoring discipline:** narrow-scoped per K-in lessons.
     7	
     8	## Cycle-5 changelog (2026-05-21)
     9	
    10	Cycle-4 verdicts: architect APPROVE, challenge NEEDS-CHANGES, CSO BLOCK, devex BLOCK. CSO + devex convergent on per-envelope HMAC client-incompatibility for standard MCP clients (Claude Desktop, Cursor don't compute HMAC); devex provided the concrete fix.
    11	
    12	**Cycle-5 architectural commit (user-directed):** Option A — startup identity is the auth boundary for stdio; transport self-signs internally. Scope explicitly local-to-local same-machine. No remote MCP transport in v1.4.7.
    13	
    14	Cycle-5 fixes:
    15	
    16	1. **Trust model differentiated by transport class.** Stdio transport (Claude Desktop, Cursor): auth = env-asserted startup identity. Per-envelope HMAC + nonce ledger = transport-internal hygiene (consistent with W1-A substrate; zero attacker value for this transport class given the local-to-local scope). WP adapter (v1.4.2 W3-C) + custom SDK clients: per-envelope HMAC authenticates the middleware/SDK to the gateway (real client auth at the envelope boundary). Gateway substrate unchanged — it always verifies HMAC + consumes nonce; the transport decides where the trust boundary lives.
    17	
    18	2. **Transport self-signs internally** (devex BLOCK fix). `tools/call` accepts normal MCP tool params from Claude Desktop / Cursor (no `_dailyos_*` keys in `arguments`). Transport reads `verified_client_id` + transport_key (process-held) + assigns request_nonce (from W1-A preissue ledger), constructs `McpToolRequestEnvelope`, signs with JCS canonicalization, calls `Gateway::handle_tool_call`. `_dailyos_envelope` and `_dailyos_signature` are NOT exposed in public `tools/list` input_schema.
    19	
    20	3. **Scope statement: local-to-local same-machine** (user direction). Documented in §0 scope. The /proc/<pid>/environ exposure (challenge + CSO cycle-4) is acceptable because any attacker who can read it is already same-user same-machine and has access to ~/.dailyos/keychain etc. env-wipe via `std::env::remove_var` reworded as best-effort in-process hygiene (limits subsequent `getenv` reads in same process), not a /proc mitigation.
    21	
    22	4. **AC-3 add public `message` field** (devex cycle-4). rmcp::Error carries both `code` + `data.kind` + a short human-readable `message` so MCP clients can render a meaningful error.
    23	
    24	5. **AC-6 input_schema clean** — only handler params + `_dailyos_*` excluded (devex cycle-4 + transport self-signs).
    25	
    26	6. **Cursor compatibility — `"type": "stdio"`** (devex cycle-4). `pair --format claude-desktop` snippet includes `"type": "stdio"` for Cursor compatibility (harmless for Claude Desktop).
    27	
    28	7. **JCS hardening** (challenge + CSO cycle-4). Duplicate-key rejection at all object levels; I-JSON-compatible values; golden fixtures expanded to cover ASCII + Unicode (no normalization) + escape sequences + nested arrays/objects + floats + nulls + integer edge cases. Fixtures shipped for SDK authors of WP adapter / custom SDK clients (real auth path).
    29	
    30	8. **claude_desktop_config.json file perms note** (challenge cycle-4). `pair --format claude-desktop` stderr warning extended: "Recommend `chmod 600` on claude_desktop_config.json; the transport_key is in the env block." Soft guidance — local-to-local scope means file perms are advisory, not load-bearing.
    31	
    32	9. **`McpToolRequestEnvelope` has no client_id** (devex cycle-4 confirmed). Transport passes `verified_client_id` as the `asserted_client_id` argument to `Gateway::handle_tool_call` (existing W1-A API). Envelope shape is unchanged.
    33	
    34	10. **Documented trust-model softening for stdio** in §11 DoD. W1-A's "every call HMAC-verified" claim still technically true, but for stdio transport the HMAC is self-signed by the same process that holds the key. Honest framing in proof bundle.
    35	
    36	## 0. Scope statement
    37	
    38	**In scope:** local stdio MCP transport (Claude Desktop, Cursor, custom local SDK clients) on the same machine as the dailyos-mcp-v2 binary. Trust boundary: same user on this machine.
    39	
    40	**Out of scope:** remote MCP transport (HTTP/TLS to external clients). Out-of-machine attackers. Cross-user attackers on the same machine (multi-tenant systems). Network-attached MCP servers. If you have a remote-MCP threat model, this lane is the wrong substrate.
    41	
    42	## 1. What this lane ships
    43	
    44	The MCP v2 Gateway (W1-A) has no transport ingress — it's a Rust function nobody can call over the wire. Legacy `dailyos-mcp` uses `rmcp::ServerHandler` on stdio. This lane bridges `rmcp` to the v2 `Gateway` so standard MCP clients (Claude Desktop, Cursor, custom) invoke v2 tools through the W1-A trust contract under the §0 scope.
    45	
    46	### Architecture: env-asserted startup identity, transport-internal signing
    47	
    48	Each `dailyos-mcp-v2` stdio process serves one verified pairing for its lifetime. Identity is asserted via environment variables at process startup; per-call envelopes are constructed + signed BY THE TRANSPORT itself, then dispatched via `Gateway::handle_tool_call` which verifies HMAC + consumes nonce as substrate hygiene.
    49	
    50	```
    51	1. pair CLI emits config snippet → operator pastes into claude_desktop_config.json
    52	2. Claude Desktop spawns `dailyos-mcp-v2 serve` with env vars set
    53	3. Subprocess startup:
    54	     a. Read DAILYOS_MCP_CLIENT_ID + DAILYOS_MCP_TRANSPORT_KEY from env
    55	     b. Look up mcp_client_manifest row for client_id; verify exists + not revoked
    56	     c. Read transport_key from keychain via row.keychain_ref; cross-check
    57	        against env-asserted key (Zeroizing comparison, constant-time)
    58	     d. Store verified_client_id + transport_key in V2ServerHandler
    59	     e. Best-effort: std::env::remove_var on env vars (limits subsequent
    60	        in-process getenv reads; does NOT scrub /proc/<pid>/environ — see §0 scope)
    61	     f. Begin serving rmcp::ServerHandler
    62	4. Standard MCP `initialize`/`get_info` handshake
    63	5. `tools/list` returns registered handlers FILTERED by mcp_tool_grant rows
    64	   for verified_client_id where exposure = Invocable. input_schema exposes
    65	   only handler-defined params (no _dailyos_* keys).
    66	6. `tools/call` flow:
    67	     a. Receive { name, arguments } — normal MCP shape, no _dailyos_* keys
    68	     b. Transport assigns request_nonce (W1-A preissue path)
    69	     c. Transport constructs McpToolRequestEnvelope { tool_name=name,
    70	        request_nonce, params=arguments, conversation_handle, tool_grant_id }
    71	     d. Transport canonicalizes envelope via RFC 8785 JCS; signs with
    72	        process-held transport_key using HMAC-SHA256
    73	     e. Transport calls Gateway::handle_tool_call(conn, &verified_client_id,
    74	        envelope, signature)
    75	     f. Gateway substrate (W1-A, unchanged): verify HMAC, consume nonce,
    76	        check manifest scope grant, dispatch handler, audit, signal-emit
    77	     g. Transport unwraps response envelope into rmcp::CallToolResult
    78	7. Process exit = session end; no persistent state
    79	```
    80	
    81	### Deliverable 1: `src-tauri/src/services/mcp_v2/transport.rs` (NEW)
    82	
    83	```rust
    84	pub struct V2ServerHandler {
    85	    gateway: Arc<Gateway>,
    86	    catalog: Arc<dyn TaxonomyCatalog>,
    87	    db: Arc<Mutex<ActionDb>>,
    88	    verified_client_id: McpClientId,                  // immutable, set at construction
    89	    transport_key: Zeroizing<[u8; 32]>,               // process-held; signs envelopes internally
    90	    server_info: ServerInfo,
    91	}
    92	
    93	impl V2ServerHandler {
    94	    /// Constructed AFTER successful env-assertion + manifest lookup +
    95	    /// keychain cross-check in main.rs. Construction failure exits the
    96	    /// process before the rmcp service runs.
    97	    pub fn from_verified_pairing(
    98	        gateway: Arc<Gateway>,
    99	        catalog: Arc<dyn TaxonomyCatalog>,
   100	        db: Arc<Mutex<ActionDb>>,
   101	        verified_client_id: McpClientId,
   102	        transport_key: Zeroizing<[u8; 32]>,
   103	    ) -> Self { ... }
   104	}
   105	
   106	impl rmcp::ServerHandler for V2ServerHandler {
   107	    fn get_info(&self) -> ServerInfo { ... }                   // mirrors legacy
   108	    async fn list_tools(...) -> Result<ListToolsResult, McpError> { ... }
   109	    async fn call_tool(...) -> Result<CallToolResult, McpError> { ... }
   110	    // Internal: construct + sign envelope from request before gateway dispatch
   111	}
   112	```
   113	
   114	### Deliverable 2: `src-tauri/src/mcp_v2/main.rs` (NEW binary `dailyos-mcp-v2`)
   115	
   116	```bash
   117	dailyos-mcp-v2 serve [--legacy-config-path <p>]   # stdio MCP server
   118	dailyos-mcp-v2 pair --client-name <s>             # pair a new client
   119	                    --grant <tool>:<scope1,scope2>:<exposure>  # repeatable
   120	                    [--format json|claude-desktop]
   121	dailyos-mcp-v2 unpair --client-id <id>             # revoke a pairing
   122	```
   123	
   124	### Deliverable 3: `pair --format claude-desktop` output
   125	
   126	```json
   127	{
   128	  "mcpServers": {
   129	    "<client-name>": {
   130	      "type": "stdio",
   131	      "command": "<resolved-absolute-path>",
   132	      "args": ["serve"],
   133	      "env": {
   134	        "DAILYOS_MCP_CLIENT_ID": "mcp_client_<hex>",
   135	        "DAILYOS_MCP_TRANSPORT_KEY": "<hex>"
   136	      }
   137	    }
   138	  }
   139	}
   140	```
   141	
   142	Stderr (never stdout):
   143	```
   144	WARNING: DAILYOS_MCP_TRANSPORT_KEY printed ONCE. Record it now — there is
   145	no recovery. Re-pair via 'unpair --client-id <id>' then 'pair' regenerates
   146	a new key.
   147	
   148	Recommended: chmod 600 on claude_desktop_config.json — the env block
   149	contains the transport_key. (Same-user same-machine threat model; advisory
   150	only.)
   151	```
   152	
   153	## 2. What this lane does NOT ship
   154	
   155	- Per-tool handlers (W2 / W3 / W4)
   156	- Remote MCP transport (HTTP/TLS) — §0 out-of-scope for v1.4.7
   157	- MCP v1 deprecation — legacy binary stays per W1-A AC-10
   158	- Production `BusSignalEmitter` (separate W1.5 lane)
   159	- Operator pairing UI beyond CLI (post-v1.4.7)
   160	- `client_label` schema column for pair-by-name revoke (separate ticket)
   161	
   162	## 3. Frozen substrate citations
   163	
   164	- `rmcp = "0.1"` already in Cargo.toml; legacy precedent at `src/mcp/main.rs:973-1031` + `:1312-1379`
   165	- W1-A `Gateway::handle_tool_call(conn, &asserted_client_id, envelope, signature)` — dispatch entry, unchanged
   166	- W1-A `auth::pair_client`, `auth::load_client_record`, `auth::verify_transport_hmac`, `auth::verify_and_consume_and_preissue` — all reused; no parallel paths
   167	- W1-A keychain via `keychain_ref` in `mcp_client_manifest` — env-asserted key cross-checked here
   168	- W1-A `mcp_transport_nonce_ledger` — request_nonce ledger consumed per envelope
   169	- W1-B `YamlTaxonomyCatalog::description_for` (will move to trait per §4)
   170	- W1-B `Gateway::seal()`
   171	- ADR-0128 §B — MCP as product surface
   172	- ADR-0102 §C.bis.replay/refresh — per-envelope HMAC + nonce
   173	
   174	## 4. K-in + substrate extensions
   175	
   176	- **K-in**: grep `docs/solutions/` + `.docs/decisions/`. Legacy `dailyos-mcp` is the rmcp precedent.
   177	- **`TaxonomyCatalog` trait extension**: add `fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription>`. One-line additive.
   178	
   179	## 5. Acceptance criteria (cycle-5 normative)
   180	
   181	- **AC-1 rmcp::ServerHandler implemented.** `V2ServerHandler` implements `get_info`, `list_tools`, `call_tool`. No custom JSON-RPC methods. Standard MCP clients work without modification.
   182	- **AC-2 Transport self-signs.** `tools/call` receives normal MCP `{ name, arguments }`. Transport (NOT client) constructs `McpToolRequestEnvelope` with `tool_name = name`, `params = arguments`, assigns `request_nonce` from W1-A ledger, signs with process-held `transport_key` over JCS canonicalization. No `_dailyos_*` keys exposed in public input_schema.
   183	- **AC-3 ToolError → rmcp::Error closed-matrix mapping** including public human-readable `message`.
   184	
   185	   | ToolError | rmcp::Error code | data.kind | message (public) | log fields (server-side) |
   186	   |---|---|---|---|---|
   187	   | `Unauthorized { missing_scope }` | -32600 invalid_request | `unauthorized` | "tool requires scope your pairing lacks" | client_id, tool_name, missing_scope |
   188	   | `BadParams { detail }` | -32602 invalid_params | `bad_params` | "tool params are malformed" | client_id, tool_name, detail (server-side only) |
   189	   | `RateLimited { retry_after_seconds }` | -32099 custom | `rate_limited` | "too many calls; retry later" | client_id, tool_name, retry_after_seconds |
   190	   | `ExposureForbidden { tool_name }` | -32601 method_not_found | `exposure_forbidden` | "tool is not exposed to your pairing" | client_id, tool_name |
   191	   | `PairingRevoked` | -32600 invalid_request | `pairing_revoked` | "pairing has been revoked" | client_id |
   192	   | `ConversationRevoked` | -32600 invalid_request | `conversation_revoked` | "conversation has been revoked" | client_id, conversation_handle |
   193	   | `NotFound { resource }` | -32601 method_not_found | `not_found` | "requested resource not found" | client_id, tool_name, resource (opaque ID) |
   194	   | `UpstreamFailure { detail }` | -32603 internal_error | `upstream_failure` | "upstream system failure; try again later" | trace_id (opaque); detail server-side only |
   195	   | `Internal { trace_id }` | -32603 internal_error | `internal` | "internal error" | trace_id |
   196	
   197	   Test asserts every variant; CI lint fails on orphan variant.
   198	- **AC-4 Boot logs.** Format: `mcp_v2 boot: pairing <client_id> verified, 0 handlers registered, 10 catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.` Strict mode env: `DAILYOS_MCP_V2_REQUIRE_HANDLERS=1` fails boot if no handlers registered.
   199	- **AC-5 `pair` CLI:**
   200	  - `dailyos-mcp-v2 pair --client-name <s> --grant <tool>:<scope1>[,<scope2>...]:<invocable|metadata-only> [--grant ...] [--format json|claude-desktop]`
   201	  - `--format claude-desktop` → snippet includes `"type": "stdio"` (Cursor compat) + env block (client_id + transport_key). **`command` field MUST be an absolute path** resolved via `std::env::current_exe()` (cycle-5 devex finding: GUI-launched Claude Desktop / Cursor don't reliably inherit shell PATH; bare binary names fail to spawn)
   202	  - Stderr warnings: key-printed-once + chmod 600 advisory (§0 scope acknowledgment)
   203	  - No name-based re-pair (substrate gap filed separately)
   204	- **AC-6 `tools/list` filtered + composed description.** Returns only registered handlers WHOSE `mcp_tool_grant` row for `verified_client_id` has `exposure = Invocable`. Each `rmcp::Tool` carries:
   205	  - `name` = catalog entry name (ScopedName)
   206	  - `description` = `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_not_to_call}")`
   207	  - `input_schema` = JSON Schema for handler params from `ToolDescription.parameters`. **Excludes `_dailyos_*` keys** (transport self-signs; client doesn't construct them).
   208	- **AC-7 Legacy `dailyos-mcp` unchanged.** Build matrix verifies both binaries compile + bin paths don't collide.
   209	- **AC-8 Integration tests** (cover AC-1..AC-14):
   210	  - Unit: `V2ServerHandler::call_tool` direct invocation with stub handler
   211	  - **Subprocess test**: spawn `dailyos-mcp-v2 serve` with env; perform full MCP `initialize` → `tools/list` → `tools/call` over stdin/stdout; assert stdout protocol-only (per legacy `src/mcp/main.rs:1312` precedent)
   212	  - Subprocess negative tests: missing env → exit 1; unknown client_id → exit 1; revoked pairing → exit 1; keychain mismatch → exit 1; tools/call with replay of consumed nonce → BadParams (W1-A substrate via gateway)
   213	- **AC-10 Required checks.** `cargo build --features mcp --bin dailyos-mcp-v2` clean. `cargo clippy --features mcp -- -D warnings` clean. `cargo test --features mcp` passes including subprocess + JCS golden tests.
   214	- **AC-11 Legacy executable identity guardrail.** `serve --legacy-config-path <p>` refuses to start if config claims v2-owned tools AND points (via filesystem canonicalization following symlinks) at the legacy `dailyos-mcp` binary. Error message includes resolved-path + suggested remediation. Hardlinks/copies acknowledged limitation; binary self-ID `--version-id` fallback filed as separate ticket.
   215	- **AC-12 Startup env-assertion** (replaces cycle-3 identify ACs).
   216	  1. Read `DAILYOS_MCP_CLIENT_ID` + `DAILYOS_MCP_TRANSPORT_KEY` from env (into Zeroizing buffers). Missing → exit 1 with operator-readable error.
   217	  2. Look up `mcp_client_manifest` row. Not found → exit 1.
   218	  3. Revoked → exit 1.
   219	  4. Read transport_key from keychain via `row.keychain_ref`; constant-time compare against env-asserted key (Zeroizing). Mismatch → exit 1.
   220	  5. **Best-effort env-wipe**: `unsafe { std::env::remove_var }` on both env vars BEFORE spawning the rmcp service / any threads. Documented as best-effort in-process hygiene — does NOT scrub `/proc/<pid>/environ` (which retains the initial exec environment for process lifetime per Linux kernel behavior). Acceptable under §0 scope (same-user same-machine threat model; attacker reading /proc already has keychain access).
   221	  6. Construct `V2ServerHandler::from_verified_pairing` and begin serving.
   222	- **AC-13 Per-envelope replay protection via W1-A substrate.** `tools/call` calls `Gateway::handle_tool_call` which invokes `auth::verify_transport_hmac` + `auth::verify_and_consume_and_preissue`. Replay (consumed nonce) → BadParams. **Note**: for stdio transport, HMAC verification + nonce consumption are transport-internal hygiene (the same process signs and verifies via the same key). Real client auth for stdio is the startup env-assertion (AC-12). For WP adapter / SDK transports, HMAC + nonce remain real client-to-gateway authentication.
   223	- **AC-14 RFC 8785 JCS canonicalization with hardened spec + golden fixtures.**
   224	  - Duplicate-key rejection at all object levels (UnknownField / DuplicateKey error)
   225	  - I-JSON-compatible values only (no NaN, no Infinity, no -0, no excessive precision)
   226	  - Golden fixtures shipped in `src-tauri/tests/fixtures/jcs_envelope_*.json` covering:
   227	    - ASCII text
   228	    - Unicode (BMP + surrogate pairs)
   229	    - Escape sequences (`\n`, `\t`, `\\`, `ÿ`)
   230	    - Nested objects (depth 5)
   231	    - Arrays (mixed types)
   232	    - Integer edge cases (0, -0 rejection, max safe int, negative)
   233	    - Floats (0.1, 1e10, 1e-10)
   234	    - Null values
   235	    - Empty object / empty array
   236	    - Duplicate-key rejection (input + expected error)
   237	
   238	## 6. Files owned
   239	
   240	| File | State | Owner |
   241	|---|---|---|
   242	| `src-tauri/src/services/mcp_v2/transport.rs` | NEW | exclusive |
   243	| `src-tauri/src/services/mcp_v2/taxonomy.rs` | additive — `description_for` to trait | shared (one-line) |
   244	| `src-tauri/src/services/mcp_v2/mod.rs` | additive — `pub mod transport;` | shared (one-line) |
   245	| `src-tauri/src/mcp_v2/main.rs` | NEW (binary with serve / pair / unpair) | exclusive |
   246	| `src-tauri/Cargo.toml` | additive — `[[bin]]` block for `dailyos-mcp-v2` | shared (additive) |
   247	| `src-tauri/tests/dos_mcp_transport_test.rs` | NEW | exclusive |
   248	| `src-tauri/tests/dos_mcp_transport_subprocess_test.rs` | NEW | exclusive |
   249	| `src-tauri/tests/fixtures/jcs_envelope_*.json` | NEW (10 golden fixtures per AC-14) | exclusive |
   250	
   251	## 7. Test plan (per-AC)
   252	
   253	- AC-1: get_info shape assertion
   254	- AC-2: wrapper construction unit test (transport builds envelope from MCP request params, signs, dispatches)
   255	- AC-3: for-each ToolError variant assert code + message + data.kind + log fields; CI lint enforces no orphan
   256	- AC-4: subprocess boot stderr parse against exact format
   257	- AC-5: subprocess pair invocation asserts stdout JSON parseable + "type": "stdio" present + `command` field is an absolute path that resolves to an executable file + stderr warnings + DB rows present
   258	- AC-6: subprocess pair with grant for A not B; tools/list returns [A] with composed description, no _dailyos_* in input_schema
   259	- AC-7: build matrix asserts both binaries compile
   260	- AC-8: full subprocess MCP handshake test; stdout protocol-only assertion

 succeeded in 0ms:
     1	# DOS-168 — MCP v2 ability/service gateway + actor policy + auth + audit — L0 plan packet
     2	
     3	**Wave:** v1.4.7 W1-A
     4	**Lane spec:** [DOS-168](https://linear.app/a8c/issue/DOS-168)
     5	**Wave plan:** `.docs/plans/v1.4.7-waves.md` §"Agent W1-A — DOS-168"
     6	**Authoring discipline:** scoped tight; cycle-2 same-shape findings driven by my substrate-grep errors → cycle-3 grounds every claim with the verified path. Cycle-2 architectural questions resolved via small W0 amendment (~30 LOC + ADR-0102 cycle-7 text) folded into this packet as a precondition deliverable.
     7	
     8	## Cycle-7 changelog (2026-05-20) — close cycle-6 NEW HIGHs via substrate landing
     9	
    10	Cycle-6 verdicts: challenge **BLOCK**; architect / CSO / devex **NEEDS-CHANGES**. 3 convergent NEW HIGH: (a) per-response nonce refresh + transport envelope shape (CSO + challenge + architect MED); (b) nonce ledger lifecycle gaps (issued_at/expires_at, fail-closed) (architect MED + challenge HIGH + CSO MED); (c) L6-3 handler docs NOT actually landed (devex HIGH + challenge) — taxonomy.rs was still a stub despite my packet claim.
    11	
    12	**Substrate landed BEFORE cycle-7 dispatch (per "stop the shape-only is a trap" pattern):**
    13	
    14	1. **`services/mcp_v2/contracts.rs`** — `OpaqueNonce` newtype + `request_nonce: OpaqueNonce` on `McpToolRequestEnvelope` + `next_request_nonce: OpaqueNonce` on `McpToolResponseEnvelope` + parity tests (`opaque_nonce_wire_is_transparent_string` + updated 4 envelope parity tests). 27/27 contracts tests pass.
    15	2. **ADR-0102 cycle-9 amendment** — §C.bis.refresh (per-response nonce refresh; seed at pairing, fresh nonce per response, asymmetric flow), §C.bis.schema (full ledger DDL with `issued_at`/`expires_at`/`consumed_at` + 3 indexes + atomic consume SQL), §C.bis.fail-closed (consumed nonce stays consumed even on downstream failure — fail-closed against TCP-reset replay attack), §C.bis.sweep (background sweep cadence + grace window).
    16	3. **`services/mcp_v2/taxonomy.rs`** — actual `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` (architect cycle-6 LOW: catalog-neutral naming, NOT `HandlerYamlMismatch`) + ~130 lines of rustdoc with concrete Side::Read no-cursor example + concrete Side::Write required-cursor example + composite-mutation example + cursor-shape-per-service table (claim_id → CommittedClaim.new_claim_id at services/claims.rs:209; signal_id → emit* String return at services/signals.rs:46-106; action_id → services::actions::*). cargo check clean.
    17	
    18	Cycle-7 packet changes:
    19	
    20	1. **§1 #1 gateway flow narrative** updated to use the new request envelope shape: Gate 0 verifies HMAC over (payload + `envelope.request_nonce`); calls `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh. On success, gateway invokes handler; on response, calls `auth::issue_next_nonce(client_id)` → returns `next_request_nonce` in response envelope.
    21	2. **AC-2** unchanged structurally (decision order preserved); Gate 0 references new `verify_and_consume_nonce`.
    22	3. **AC-6** mutation_cursor convention: SHOULD-with-warning (NOT MUST-fail per devex cycle-6 MED). Side::Write without cursor → Suite-S `mcp_write_handler_missing_cursor` signal + audit row written without cursor field. Forced MUST-fail (dev-mode validator that rejects test fixtures) filed as separate W1.5 ticket.
    23	4. **AC-7 reject signal durability** (challenge cycle-6 MED): `emit_signal` for `McpInvocationRejected` uses the emit-or-log pattern; failure → fallback warning log + Suite-S alert `mcp_reject_signal_emit_failed`. No forensic blind spot.
    24	5. **AC-11 + AC-12 + AC-5** unchanged (already correct per cycle 6).
    25	6. **Migration v243 schema reflects ADR-0102 §C.bis.schema**: full DDL with `nonce, client_id, issued_at, expires_at, consumed_at` + 3 indexes + atomic consume predicate + background sweep + 1h grace window.
    26	7. **L6-3 substrate-truth verified**: taxonomy.rs now contains `TaxonomyCatalog` trait + rustdoc; not a stub.
    27	
    28	Cycle-7 net delta: ~25 lines packet + 60 LOC code (contracts.rs OpaqueNonce + envelope amendments + tests) + 130 LOC code (taxonomy.rs trait + rustdoc) + 100 lines ADR cycle-9 amendment. Cumulative cycles 1→7: 151 → 296 LOC packet (+145 over 7 cycles, well within the K-in `<80/cycle` discipline target).
    29	
    30	---
    31	
    32	## Cycle-6 changelog (2026-05-20) — L6 decisions folded in
    33	
    34	User L6 verdict (3 structural decisions) folded in this cycle:
    35	
    36	**L6-1 (replay guard) = nonce ledger.** Per ADR-0102 §C cycle-8 amendment (this cycle): every MCP message MUST carry a server-issued `request_nonce` signed in the HMAC; server stores recently-seen nonces in `mcp_transport_nonce_ledger` (v243-shifted-up to v244 — see slot reallocation below); replay attempts within window → `ToolError::BadParams { detail: "nonce_replayed" }`. Mirrors `services::surface_nonce::verify_and_consume` pattern verbatim. Closes cycle-5 CSO HIGH.
    37	
    38	**L6-2 (reject audit) = drop `internal_reject_reason`.** AC-6 narrows: audit append happens ONLY on handler success. Auth-state rejections (`PairingRevoked` / `ExposureForbidden` / `Unauthorized` / `ConversationRevoked`) emit Suite-S signal `mcp_invocation_rejected` (per ADR-0115 NonPiiMetadata) carrying `client_id` (if manifest resolved) or `<unresolved>` plus the reject reason. No audit row for rejects. Closes cycle-5 challenge HIGH + architect MED. Removes the AC-6 vs handler-success-precondition contradiction.
    39	
    40	**L6-3 (handler docs) = rustdoc on TaxonomyCatalog trait.** Cursor convention documented as rustdoc on `TaxonomyCatalog` trait in `services/mcp_v2/taxonomy.rs`. ~30 lines: trait-level overview + per-method examples (`Ok(json!({..., "mutation_cursor": {"claim_id": 42}}))`); naming conventions; allowed cursor ID shapes (UUID, integer, opaque hash; NEVER PII per AC-6). `cargo doc` surfaces it for handler authors. Closes cycle-5 devex HIGH.
    41	
    42	Cycle-6 packet edits:
    43	
    44	1. **AC-2 + §1 #1 + AC-12** add nonce check as Gate 0 (BEFORE PairingRevoked): `auth::verify_transport_hmac(payload, sig, presented_nonce)` verifies HMAC AND checks/consumes nonce; failure → `ToolError::BadParams { detail: "invalid_signature" }` OR `{ detail: "nonce_replayed" }`. Identical wall-clock floor via `sleep_until(start + 10ms)` to prevent timing oracle.
    45	2. **AC-6 reworded** (L6-2): "Handler success precedes audit append. On rejection, NO audit row is written; instead a `SignalType::McpInvocationRejected` is emitted (NonPiiMetadata; payload: `client_id_or_unresolved`, `reject_reason`). Audit detail JSON: `{client_id, conversation_handle, tool_name, params_hash, response_hash, mutation_cursor?}` — `internal_reject_reason` field REMOVED (per L6-2; rejections handled via SignalType not audit). Outbox path unchanged for JSONL append failures on success-rows."
    46	3. **AC-7 extended**: registers TWO new SignalTypes — `McpToolInvoked` (existing) + `McpInvocationRejected` (new). Both NonPiiMetadata per ADR-0115. Same 5-touchpoint wiring.
    47	4. **AC-11 + AC-12** unchanged structurally (Zeroizing wrapper, 10ms floor preserved); applies to nonce-check path too.
    48	5. **§14 + taxonomy.rs**: trait gets rustdoc per L6-3. Concrete examples for handler authors.
    49	6. **Migration slot reallocation**: insert v243 for nonce ledger; bump original v243 (rate-limit ledger + audit outbox) → v244.
    50	   - **v241** `mcp_client_manifest` (unchanged)
    51	   - **v242** `mcp_conversation_handle` (unchanged: `(handle, client_id, mint_at, last_touched_at, revoked_at)` per cycle-3 binding)
    52	   - **v243** `mcp_transport_nonce_ledger` (`nonce, client_id, consumed_at`; UNIQUE composite on `(nonce, client_id)`; index for cleanup; window = 5 minutes)
    53	   - **v244** `mcp_tool_call_ledger` + `mcp_audit_outbox` (was v243)
    54	7. **ADR-0102 §C cycle-8 amendment** (this cycle) documents the nonce ledger requirement: 5-minute window, `services::surface_nonce` precedent, BadParams variants. Lands in Deliverable 0 alongside cycle-7.
    55	
    56	Cycle-6 net delta: +30 lines packet + 1 substrate landing (nonce ledger spec) + 1 ADR amendment.
    57	
    58	---
    59	
    60	## Cycle-5 changelog (2026-05-20)
    61	
    62	Cycle-4 verdicts: codex challenge **BLOCK**; architect / CSO / devex **NEEDS-CHANGES**. **Unanimous (4/4)** on two issues: (i) §1 #1 still contradicted AC-2 on the absent-grant branch (3rd cycle hitting the same propagation gap); (ii) `ServiceMutationCursor` was claimed as landed but `rg` finds no such type in `contracts.rs` (and the substrate citations for `CommittedClaim.claim_id` and `services::signals::publish -> SignalId` were wrong — actual shapes are `CommittedClaim { claim, new_claim_id }` and `services::signals::emit*` returning `String`). 3/4 on namespace collision lint not credible (script absent, paths wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`, W1-B YAML doesn't exist yet). 2/3 on AC-12 ±5ms ceiling not portable on macOS/Linux CI.
    63	
    64	Cycle-5 changes (all surgical, all substrate-verified via `rg` before writing):
    65	
    66	1. **§1 #1 absent-grant branch fixed** (4/4 convergent). Narrative now reads: "absent grant OR non-Invocable exposure → `ExposureForbidden { tool_name }`; grant present AND invocable, but `scopes_required.is_subset(scopes_granted)` false → `Unauthorized { missing_scope }`." Matches AC-2 verbatim. Grep-validated: `rg 'absent grant' dos-168-l0-plan.md` returns only the matching new wording.
    67	2. **ServiceMutationCursor REMOVED from Deliverable 0 / W0 amendment scope** (4/4 convergent). The `McpToolHandler::invoke` W0-frozen trait signature does NOT change. Cursor enforcement happens at the gateway level via JSON inspection: gateway extracts `result_value.get("mutation_cursor")` after handler success for `Side::Write` handlers. If absent for `Side::Write`, gateway logs a Suite-S warning `mcp_write_handler_missing_cursor` (forensic alert; handler effects preserved) and proceeds to audit append with `mutation_cursor` field absent in detail JSON. The cursor convention is documented in W1-A `taxonomy.rs` `TaxonomyCatalog` trait docs (Side::Write handlers SHOULD populate cursor). No new W0 type added.
    68	3. **Substrate citations corrected** (architect HIGH + devex HIGH + CSO MED). `CommittedClaim` is at `services/claims.rs:209` with `new_claim_id` field (not `claim_id`); `services::signals::emit*` returns `String` (at `services/signals.rs:46-106`), not `publish -> SignalId`. AC-6 cursor format: `mutation_cursor: serde_json::Value` (typed-less, IDs only — no payloads per CSO MED). Convention documented in taxonomy.rs trait docs.
    69	4. **Namespace collision CI REMOVED from W1-A scope** (3/4 convergent). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" (Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`). The CI lint requires W1-B YAML which doesn't exist yet AND clean reading of the legacy tool list at `src-tauri/src/mcp/main.rs:715-730` (challenge cycle-4 cited path is correct). Cycle-4 §7 + AC-10 reference + §11 entry all dropped.
    70	5. **AC-12 timing floor reworked** (2/3 — challenge + CSO). Drop ±5ms ceiling (not portable). Gateway uses `tokio::time::sleep_until(start + Duration::from_millis(10))` before returning ANY `Unauthorized` / `ExposureForbidden` / `PairingRevoked` / `ConversationRevoked` error. Test asserts both missing-client and invalid-HMAC branches return after `>= 10ms` wall-clock; no upper bound. Identical error shape constraint preserved (uniform JSON keys).
    71	6. **AC-11 key custody spelled out** (CSO not-closed). `auth::verify_transport_hmac` wraps loaded key in `Zeroizing<[u8; 32]>` from the `zeroize` crate; per `Zeroize` Drop impl the buffer is wiped on function exit. (NOT the line 623 derivation; correct precedent is `SecretBytes32` at `src/surface_runtime/hmac.rs:643`.) Test asserts `verify_transport_hmac` does not retain key reference past function return.
    72	7. **AC-6 internal reject reason** (architect MED). Audit detail JSON includes `internal_reject_reason: Option<String>` carrying machine-readable cause (`absent_grant`, `non_invocable`, `missing_scope:<scope>`, `invalid_hmac`, `pairing_revoked`, `conversation_revoked`) for forensic ordering. Wire shape stays merged — caller sees only the typed `ToolError`. No leak.
    73	8. **AC-5 ledger pruning order** (challenge tightening). Explicit transaction order: `BEGIN IMMEDIATE` → `DELETE FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2 AND called_at < (?3 - window_seconds_ms)` → `SELECT COUNT(*) FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2` → if `count >= max_calls` ROLLBACK + return `RateLimited`; else `INSERT INTO mcp_tool_call_ledger VALUES (?1, ?2, ?3)` → `COMMIT`.
    74	9. **`mcp_audit_outbox` access control** (CSO MED). Same access pattern as JSONL audit log per ADR-0094: operator-only via existing audit inspection paths (`get_audit_log_records` / `export_audit_log` Tauri commands). No separate CLI / no MCP exposure of outbox contents. Outbox is forensic only.
    75	10. **AC-6 mutation_cursor cap** (devex MED). `mutation_cursor` JSON field max depth 4, max byte size 2 KiB (uniform with existing `MCP_INVOCATION_CACHE_ENTRY_BYTE_CAP / 5` budget at `bridges/mcp.rs`). Test asserts oversize → gateway logs warning + truncates to placeholder `"truncated_oversize"` in audit detail; handler effects preserved.
    76	11. **DX note added** (devex MED). 10ms response floor on auth-state errors adds latency vs sub-ms paths. Acceptable security tradeoff (prevents client_id enumeration). Hosts amortize via successful-result caching.
    77	
    78	Cycle-5 packet net delta: +35 lines. Cumulative cycle 1→5: 151 → ~253. Cycle-5 is the convergence cycle; no new scope, only AC + narrative tightening + 1 ticket spinoff.
    79	
    80	---
    81	
    82	## Cycle-4 changelog (2026-05-20)
    83	
    84	Cycle-3 verdicts: codex challenge **BLOCK** (1 NEW HIGH AC-2 + 2 MED; cycle-2 "STILL-OPEN" findings were all about Deliverable 0 not having landed yet); architect **NEEDS-CHANGES** (7/7 closed; 1 NEW HIGH AC-2; 1 MED wave-plan-stale); CSO **NEEDS-CHANGES** (2 STILL-OPEN due to Deliverable 0 unlanded + 1 unverified-floor; 3 NEW MED + 1 LOW); devex **NEEDS-CHANGES** (4/4 packet-level closed; 1 NEW HIGH "Deliverable 0 must actually land" + 1 NEW HIGH mutation_cursor + 1 MED fixture realism).
    85	
    86	Convergent NEW HIGH (3/4): AC-2 internal inconsistency between §1 #1 and the AC body on `Unauthorized` vs `ExposureForbidden` branching. **All cycle-3 "STILL-OPEN" findings about Deliverable 0 are now closed by substrate landing** — `contracts.rs` carries `ExposureForbidden` variant + golden parity test; ADR-0102 carries cycle-7 amendment (§C.bis + §D.bis); wave plan stale text fixed at lines 85, 453, 467.
    87	
    88	Cycle-4 changes:
    89	
    90	1. **Deliverable 0 has landed in this same branch** (closes the 4/4 "shape-only is a trap" finding). Substrate-truth, not packet promise. Cycle-4 reviewers can grep `ExposureForbidden` in `contracts.rs` + read ADR-0102 cycle-7 amendment + verify wave plan lines 85/453/467 carry the new text.
    91	2. **AC-2 reworded to architect's recommended decision order** (closes convergent NEW HIGH). Decision order: (i) `revoked_at.is_some()` → `PairingRevoked`; (ii) absent grant OR `exposure != Invocable` → `ExposureForbidden { tool_name }`; (iii) grant present + invocable but `scopes_required.is_subset(scopes_granted)` false → `Unauthorized { missing_scope }`; (iv) caller-asserted scope/conversation_id in params → `BadParams`. §1 #1 narrative aligned.
    92	3. **AC-12 timing oracle gains numeric floor** (CSO MED): floor = 10ms minimum response time on `Unauthorized` failure paths (missing-client + invalid-HMAC); tolerance = ± 5ms jitter ceiling. Test asserts measured wall-clock floor is `>= 10ms - 5ms = 5ms` for both branches. Implementation: gateway uses `tokio::time::sleep_until(start + 10ms)` before returning either error.
    93	4. **AC-11 transport key uses `Zeroizing<[u8; 32]>`** (CSO MED): per existing `surface_runtime/hmac.rs:623` precedent — Rust drop is insufficient for transient key material. `auth::verify_transport_hmac` borrows the key inside `Zeroizing` wrapper; on function exit the buffer is zeroized.
    94	5. **mcp_tool_call_ledger pruning + composite index** (challenge MED): v243 migration adds `idx_mcp_tool_call_ledger_client_tool_called ON (client_id, tool_name, called_at)` + same-transaction prune step in the rate-limit reservation: `DELETE FROM mcp_tool_call_ledger WHERE client_id = ?1 AND tool_name = ?2 AND called_at < ?3 - window_seconds_ms` before the `BEGIN IMMEDIATE` insert.
    95	6. **mutation_cursor specified per service** (devex HIGH + challenge MED): for `Side::Write` handlers, the gateway requires the handler's `Ok(value)` payload to carry a `mutation_cursor: ServiceMutationCursor` field. `ServiceMutationCursor` is a tagged enum: `Claim { claim_id: ClaimId }`, `Signal { signal_id: SignalId }`, `Action { action_id: ActionId }`, `Multi(Vec<ServiceMutationCursor>)` — handlers route through `services::claims::commit_claim` (returns `CommittedClaim { claim_id: ClaimId, ... }` per `services/claims.rs:204`), `services::signals::publish` (returns `SignalId` per `services/signals.rs:46-106`), or `services::actions::*` (returns `ActionId`). For `Side::Read` handlers, `mutation_cursor` is omitted from the audit detail (no mutation happened). New W0 type added in cycle-4 substrate landing: `ServiceMutationCursor` in `services/mcp_v2/contracts.rs` (small additive type, sibling to `ToolError`).
    96	7. **Legacy coexistence interim guard** (CSO MED): **SUPERSEDED by cycle-5 #4** — the namespace collision CI is moved out of W1-A scope (script doesn't exist; cross-wave dep on W1-B YAML; path was wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" in Maintenance project. AC-10 records the spinoff.
    97	8. **AC-3b fixture realism** (devex MED): cross-client test fixture uses two real paired `McpClientId`s minted via the pairing handshake (not synthetic strings), and a real `OpaqueConversationHandle` minted under client A presented by client B.
    98	9. **Wave-plan stale text fixed and verified in same commit** (architect MED): lines 85, 453, 467 now carry the dedicated-variant language per ADR-0102 cycle-7 amendment.
    99	
   100	Cycle-4 packet net delta: +20 lines (198 → ~218). Tight; cycle-4 is convergence cycle, not new scope.
   101	
   102	---
   103	
   104	## Cycle-3 changelog (2026-05-20)
   105	
   106	Cycle-2 verdicts: codex challenge **BLOCK** (1 STILL-OPEN HIGH + 2 NEW HIGH + 2 MED); architect **NEEDS-CHANGES** (2 STILL-OPEN + 3 NEW HIGH + 2 MED); CSO **NEEDS-CHANGES** (0 STILL-OPEN + 3 MED + 1 LOW); devex **BLOCK** (2 STILL-OPEN HIGH + 3 NEW HIGH + 2 MED). Convergent: sentinel-scope-on-Unauthorized (3/4), `bridges/mcp.rs:737` misidentified (2/4 — confirmed factually as test fixture), audit wire envelope mutation (2/4 — confirmed factually as W0 violation), handle client_id binding gap (2/4).
   107	
   108	Cycle-3 changes:
   109	
   110	1. **W0 amendment commit added as Deliverable 0** (Class N — 3/4 convergent + Class O — 2/4 convergent). Lands BEFORE W1-A body. Two changes to `src-tauri/src/services/mcp_v2/contracts.rs`:
   111	   - New `ToolError::ExposureForbidden { tool_name: ScopedName }` variant. Same shape as `PairingRevoked`/`ConversationRevoked` per ADR-0102 §C — auth-state classes that do not abuse `Scope` newtype. New parity test added.
   112	   - `OpaqueConversationHandle` wire shape unchanged (still opaque string); server-side binding spec amended in ADR-0102 §D — handles bind to `client_id` at mint time, resolve by `(client_id, handle)`, cross-client presentation returns `ConversationRevoked`. Wire layer stays compatible.
   113	   ADR-0102 §C/§D cycle-7 amendment text drafted as part of this commit (~6 lines added to ADR-0102).
   114	2. **§1 #8 bridges/mcp.rs change REMOVED** (Class Q — 2/4, factually confirmed). The `todo!()` at `src/bridges/mcp.rs:737` is inside `provenance_actor_for_test` (`#[cfg(test)] mod tests`), not production routing. There is no production `Actor::McpClient` arm in `bridges/mcp.rs` to wire in W1-A. Test fixture is filled by W2+ when the first `Actor::McpClient`-using handler ships parity tests.
   115	3. **Audit wire envelope mutation REMOVED** (Class R — 2/4, factually confirmed). `audit_outbox_pending` warning no longer surfaces on `McpToolResponseEnvelope` (W0 frozen with only `conversation_handle` + `result`). Outbox state stays behind `audit.rs` — operator-visible only (Suite-S regression alert when outbox row count > 0).
   116	4. **Audit signature corrected** (Class S — 1/4, factually confirmed). `AuditLogger::append_with_actor(event: &str, actor: &Actor, fields: AuditFields)`. `event = "mcp.tool_invoked"` is the SEPARATE arg; `AuditFields { category: "security", detail: {...}, wp_user_id: None, wp_user_hash: None, request_id: Some(...) }` carries the structured payload. `audit_log.rs:325` already documents that McpClient routes through this path with `wp_user_id/hash = None`.
   117	5. **AC-2 reworded** (Class N closure): non-Invocable exposure returns `ToolError::ExposureForbidden { tool_name }` (W0 amendment, item 1 above). No sentinel scope. Missing scope still returns `ToolError::Unauthorized { missing_scope }`.
   118	6. **Handle migration shape updated** (Class O closure): v242 schema is `mcp_conversation_handle { handle, client_id, mint_at, last_touched_at, revoked_at }` with composite unique on `(handle, client_id)`. Resolve takes `(client_id, handle)`; mismatch → `ConversationRevoked`. AC-3b adds explicit cross-client rejection test.
   119	7. **Migration filename clarified** (Class T — architect HIGH #3): registered `version` field is authoritative. Filenames mirror version with no offset: `241_mcp_client_manifest.sql`, `242_mcp_conversation_handle.sql`, `243_mcp_rate_limit_and_audit_outbox.sql`. The `migration-filename-version-offset` solution doc applies to a different convention; cited here only as a discipline reminder, not as offset guidance.
   120	8. **W1-A/W1-B taxonomy seam restructured** (Class V — architect MED): W1-A defines the CATALOG INTERFACE in `taxonomy.rs` (validation function signature + binding contract). W1-B owns YAML LOADING + boot validation invocation. W1-A no longer dispatches off W1-B-owned YAML at runtime; the binding is purely a registry-time validation seam.
   121	9. **Manifest lookup is indexed** (Class U — architect MED): per-dispatch DB query is `SELECT * FROM mcp_tool_grant WHERE client_id = ? AND tool_name = ?` (indexed composite key), NOT `tool_grants: Vec<ToolGrant>` full-load. Manifest struct holds one resolved `ToolGrant` per call, not the full vector.
   122	10. **AC-6 audit-required emit-or-log caveat addressed** (Class W — challenge MED): if JSONL append fails AND outbox insert also fails, gateway returns `ToolError::Internal { trace_id }` to caller (handler effects preserved but caller is warned via Internal error), emits Suite-S alert `mcp_audit_double_failure` for operator action. Documented as a documented degradation path per the cited emit-or-log caveat.
   123	11. **Mid-call revocation audit state** (Class X — challenge MED): if revocation lands between manifest load and handler return, audit row's `detail` includes `revocation_state: "revoked_during_invocation"` per ADR-0102 §C.
   124	12. **Key custody AC added** (Class Y — CSO MED): transport key loaded from keychain for ONE HMAC verification per dispatch and dropped after; gateway state never caches the raw key. Test asserts `Manifest` struct contains `KeychainRef`, not raw bytes.
   125	13. **Timing oracle AC added** (Class Z — CSO LOW): missing-client and invalid-HMAC return the same `ToolError::Unauthorized` shape with the same response timing (bounded by a uniform sleep floor). Test asserts both paths return the same wall-clock-floor.
   126	14. **AC-6 temporal ordering note added** (Class AA — devex MED): audit is post-commit attribution, not source-of-order truth. AC-6 detail carries mutation cursor IDs (e.g., `claim_id`, `signal_seq`) so audit ordering can be reconstructed from substrate even when audit timestamp != mutation timestamp.
   127	15. **Taxonomy mismatch returns structured error** (Class BB — devex MED): boot-time mismatch returns `TaxonomyError::HandlerYamlMismatch { handler, yaml }` to the registry init path; operator-friendly error message + operator-init suggested fix. No `panic!()`.
   128	16. **Wave plan stale text fixed** (Class G — devex HIGH): doc-only edit to `.docs/plans/v1.4.7-waves.md:451+467` removing `Unauthorized { missing_scope: "valid_conversation" }`, lands in same commit as W0 amendment.
   129	
   130	Cycle-3 packet net delta from cycle-2: +60 lines (151 → 184 → ~245). Most of the delta is the changelog above + AC tightening + the W0 amendment specification. Not in cycle-2's "5+ net-new findings per cycle / 80+ line revision" diminishing-returns range.
   131	
   132	---
   133	
   134	## Deliverable 0 — W0 amendment commit (precondition)
   135	
   136	Lands BEFORE W1-A body (own commit, own L2 cycle). Files (all already landed in this branch as of cycle-3/cycle-6 substrate work):
   137	
   138	- `src-tauri/src/services/mcp_v2/contracts.rs` — `ToolError::ExposureForbidden { tool_name: ScopedName }` variant + golden parity test (`tool_error_exposure_forbidden_wire_shape`). LANDED.
   139	- `.docs/decisions/0102-abilities-as-runtime-contract.md` — cycle-7 amendment (§C.bis sentinel-scope ExposureForbidden + §D.bis handle client_id binding) + cycle-8 amendment (§C.bis.replay nonce ledger per L6-1 user verdict). LANDED.
   140	- `.docs/plans/v1.4.7-waves.md` — stale `Unauthorized { missing_scope: "valid_conversation" }` text fixed at lines 85 + 451 + 467. LANDED.
   141	
   142	Total: ~45 LOC code + ~50 lines doc. All grep-verified against substrate before cycle-6 dispatch.
   143	
   144	---
   145	
   146	## 1. What this lane ships (W1-A body, lands AFTER Deliverable 0)
   147	
   148	1. **`src-tauri/src/services/mcp_v2/gateway.rs`** — `Gateway::handle_tool_call(envelope: McpToolRequestEnvelope) -> McpToolResponseEnvelope`. Per-dispatch flow (matches AC-2 verbatim): **Gate 0a** `auth::verify_transport_hmac(payload, sig)` — HMAC failure → `ToolError::BadParams { detail: "invalid_signature" }`. **Gate 0b** `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh (cycle-9 amendment) — atomic SQL `UPDATE mcp_transport_nonce_ledger SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`; on `rowsAffected = 0` reject with `{ detail: "nonce_replayed" }` (row consumed) or `{ detail: "invalid_signature" }` (row absent/expired — uniform shape). Fail-closed: consumed nonce stays consumed even on downstream failure. Then: `auth::load_client_record(client_id)` → reject if `revoked_at.is_some()` (→ `PairingRevoked`); `auth::resolve_tool_grant(client_id, tool_name)` (indexed lookup, single row) → **absent grant OR `grant.exposure != Invocable` → `ExposureForbidden { tool_name }`**; **grant present AND invocable but `tool.description().scopes_required.is_subset(grant.scopes_granted)` false → `Unauthorized { missing_scope }`**; `auth::resolve_or_mint_handle(client_id, envelope.conversation_handle)` (mismatch → `ConversationRevoked`); enforce rate limit BEFORE dispatch via atomic ledger insert (AC-5 explicit tx order); construct `McpActor::Client`; dispatch via registered `McpToolHandler`; on success → extract `mutation_cursor` from `result_value.get("mutation_cursor")` for `Side::Write` (Suite-S warning if absent for Write — SHOULD not MUST per AC-6); → `audit::write(event, fields, actor)` via emit-or-log path (handler effects preserved on audit failure); emit `SignalType::McpToolInvoked`; **call `auth::issue_next_nonce(client_id)` to mint fresh nonce** → return `next_request_nonce` in response envelope. On any auth-state rejection (BadParams from Gate 0a/0b, PairingRevoked, ExposureForbidden, Unauthorized, ConversationRevoked): NO audit row written; emit `SignalType::McpInvocationRejected` via emit-or-log helper (failure → fallback log + Suite-S alert `mcp_reject_signal_emit_failed` per AC-7); gateway calls `tokio::time::sleep_until(start + Duration::from_millis(10))` for AC-12 timing parity; mint fresh nonce on response anyway (so client can recover).
   149	2. **`src-tauri/src/services/mcp_v2/auth.rs`** — `pair_client(handshake) -> McpClientId` (mirrors `services::surface_pairing` + `services::surface_session_keychain` per ADR-0102 §C); `load_client_record(client_id) -> ClientRecord`; `resolve_tool_grant(client_id, tool_name) -> Option<ToolGrant>` (indexed query); `verify_transport_hmac(payload, sig)` (loads key for one verify, drops); `resolve_or_mint_handle(client_id, presented) -> OpaqueConversationHandle` (mint on None; resolve by `(client_id, handle)`; mismatch → `ConversationRevoked`); `revoke_handle / revoke_client` admin paths.
   150	3. **`src-tauri/src/services/mcp_v2/audit.rs`** — `write(event, actor, params, response_or_error, request_id)`. Computes keyed HMAC-SHA256 hashes per ADR-0102 §C. Calls `AuditLogger::append_with_actor(event = "mcp.tool_invoked", actor = &Actor::McpClient { .. }, fields = AuditFields { category: "security", detail: serde_json!({client_id, conversation_handle, tool_name, params_hash, response_hash, revocation_state?, mutation_cursor?}), wp_user_id: None, wp_user_hash: None, request_id: Some(...) })`. On JSONL append failure → insert into `mcp_audit_outbox` table. On outbox failure → return `ToolError::Internal { trace_id }` to caller + emit Suite-S `mcp_audit_double_failure` alert.
   151	4. **`src-tauri/src/services/mcp_v2/actor_policy.rs`** — `ClientRecord -> AbilityPolicy` projection consumed by gateway. Per ADR-0102 §B asymmetry, `Actor::McpClient` carries no scopes; this module owns the projection.
   152	5. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — W1-A defines the CATALOG INTERFACE only: `pub trait TaxonomyCatalog { fn validate_against_handlers(&self, handlers: &[&dyn McpToolHandler]) -> Result<(), TaxonomyError> }` + `TaxonomyError::HandlerYamlMismatch { handler: ScopedName, yaml: Option<ScopedName> }`. W1-B fills the YAML loader and invokes `validate_against_handlers` at boot.
   153	6. **`src-tauri/src/signals/policy_registry.rs`** — adds `SignalType::McpToolInvoked` at 5 sites per ADR-0115: enum variant; `from_name`; `canonical_name`; known-names const; `policy_for` returning `SignalPolicy::local_observation + NonPiiMetadata`. Payload: `tool_name`, `client_id`, `conversation_handle`. NO raw params/response. Suite S invariant.
   154	7. **`src-tauri/src/migrations.rs`** — FOUR migrations in v241–v244 (cycle-6 added v243 nonce ledger per L6-1; original v243 rate-limit + audit-outbox shifted to v244). Latest registered is v240 at `migrations.rs:927`; filename mirrors version with no offset:
   155	   - **v241** `241_mcp_client_manifest.sql` — `mcp_client_manifest (client_id, paired_at, revoked_at, transport_key_ref)` + `mcp_tool_grant (client_id, tool_name, scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs)` with index `idx_mcp_tool_grant_client_tool ON (client_id, tool_name)`.
   156	   - **v242** `242_mcp_conversation_handle.sql` — `mcp_conversation_handle (handle, client_id, mint_at, last_touched_at, revoked_at)` with `UNIQUE(handle, client_id)` + `idx_mcp_conversation_handle_lookup ON (client_id, handle)`.
   157	   - **v243** (cycle-6 NEW per L6-1; cycle-7 schema completed per ADR-0102 §C.bis.schema) `243_mcp_transport_nonce_ledger.sql` — `mcp_transport_nonce_ledger (nonce TEXT, client_id TEXT, issued_at INTEGER, expires_at INTEGER, consumed_at INTEGER NULL)` with `UNIQUE(nonce, client_id)` + `idx_mcp_nonce_lookup ON (client_id, nonce)` + `idx_mcp_nonce_consumed ON (consumed_at)` + `idx_mcp_nonce_expires ON (expires_at)`. Window: 5 minutes (`expires_at = issued_at + 5min`) per ADR-0102 §C.bis.refresh. Atomic consume per §C.bis.schema: `UPDATE ... SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`. Background sweep every 5 min deletes rows where `expires_at < now() - 1 hour` (1h grace per §C.bis.sweep). Fail-closed per §C.bis.fail-closed.
   158	   - **v244** (was v243) `244_mcp_rate_limit_and_audit_outbox.sql` — `mcp_tool_call_ledger (client_id, tool_name, called_at)` for sliding-window rate accounting (`BEGIN IMMEDIATE` for atomic reservation) + `mcp_audit_outbox (id, event, detail_json, actor_kind, request_id, created_at, drained_at)` forensic store.
   159	8. **`src-tauri/scripts/check_mcp_tool_handler_allowlist.sh`** — CI lint forbidding direct DB writes from `services/mcp_v2/handlers/*.rs`. Acknowledged porous per `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md`; structural enforcement filed as a separate maintenance ticket if a real bypass surfaces in W2+.
   160	
   161	## 2. Coexistence with legacy MCP
   162	
   163	The wave plan §W1-A explicitly admits coexistence: v2 introduces a v2-paired ingress for clients that opt into the ADR-0102 §C contract; legacy `src/mcp/main.rs` + `src/bridges/mcp.rs:402` continue serving `Actor::Agent` (pre-pairing substrate). The two paths route by `Actor` variant at the abilities-runtime layer (per ADR-0102 §F `AbilityPolicy.allowed_actors`).
   164	
   165	The challenge HIGH "every MCP-originated invocation must pass §C four gates" applies forward — to v1.4.7-introduced ingress. Legacy is grandfathered; retirement is a separate Linear ticket (filed at L0 close).
   166	
   167	**W1-A does NOT touch legacy code paths.** The `Actor::McpClient` test fixture in `src/bridges/mcp.rs:737` is filled by W2+ when first handler ships its parity test; production legacy code (`mcp/main.rs:1379` rmcp stdio serving) stays unchanged.
   168	
   169	**W1-A does NOT ship:** per-tool handler bodies (handlers/*.rs placeholders untouched); tool taxonomy YAML (W1-B); MCP write allowlist extension (W4); new `Actor::McpClient` enum variant or `McpClientId` / `OpaqueConversationHandle` newtypes (W0 already shipped in `abilities-runtime/src/abilities/registry.rs:124,153,441`); new pairing transport design (mirrors surface_pairing).
   170	
   171	## 3. Frozen substrate citations (W0 + W0-amendment + ADRs)
   172	
   173	- **W0 contracts (`src-tauri/src/services/mcp_v2/contracts.rs`)** — `ToolDescription`, `Side`, `ScopedName`, `Scope`, `ParamSpec`, `ReturnSpec`, `ToolExample`, `Page<T>`, `OpaqueConversationHandle` (wire envelope, opaque string), `McpClientId`, `McpActor::Client`, `McpToolRequestEnvelope` (no granted_scopes — gateway hydrates), `McpToolResponseEnvelope` (frozen: `conversation_handle` + `result`; no audit warnings), `McpToolResult`, `McpToolHandler` trait, `ToolError` (Deliverable 0 adds `ExposureForbidden`).
   174	- **`abilities-runtime/src/abilities/registry.rs:106-153,441-480`** — `Actor::McpClient`, `McpClientId`, `OpaqueConversationHandle` runtime types.
   175	- **`abilities-runtime/src/inventory.rs:63,412-433,611,679`** — `ActorKind::McpClient` discriminator + `AbilityActor::project()` Agent+Invocable → McpClient.
   176	- **`src/audit_log.rs:47,149,325`** — `AuditRecord`, `AuditFields`, `AuditLogger::append_with_actor(event, actor, fields)` (event SEPARATE arg). McpClient explicitly noted at line 350-354.
   177	- **ADR-0102 §C/§D/§E + cycle-7 amendment (Deliverable 0)** — four-gate trust contract, handle lifecycle with client_id binding, scope namespace.
   178	- **ADR-0094** — JSONL append-only audit substrate.
   179	- **ADR-0111 §8** — SurfaceClient pairing precedent.
   180	- **ADR-0115** — signal registration discipline.
   181	- **ADR-0128 §6 + §C** — continuity affordance; wire shape defers to ADR-0102 §D.
   182	
   183	## 4. K-in citations (present-on-dev)
   184	
   185	- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — applied; cycle-3 grounded every claim with a verified path.
   186	- `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md` — audit-required caveat at line 50 addressed via §1 #3 double-failure escalation.
   187	- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — acknowledged porous; structural enforcement filed if W2+ surfaces a bypass.
   188	- `docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md` — relevant only as filename discipline reminder; v241–v243 do NOT use an offset; `registered version` is authoritative.
   189	- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — Deliverable 0 W0 amendment satisfies the precedent.
   190	
   191	## 5. Per-client substrate format (frozen at L0 cycle 3)
   192	
   193	```rust
   194	// src-tauri/src/services/mcp_v2/auth.rs
   195	pub struct ClientRecord {
   196	    pub client_id: McpClientId,
   197	    pub paired_at: SystemTime,
   198	    pub revoked_at: Option<SystemTime>,
   199	    pub transport_key_ref: KeychainRef,        // raw key never in ClientRecord/state
   200	}
   201	
   202	pub struct ToolGrant {                          // resolved per-call via indexed query
   203	    pub tool_name: ScopedName,
   204	    pub scopes_granted: Vec<Scope>,
   205	    pub exposure: McpExposure,
   206	    pub rate_limit: ToolRateLimit,
   207	}
   208	
   209	pub struct ToolRateLimit { pub max_calls: u32, pub window_seconds: u32 }
   210	```
   211	
   212	Manifest is composed at call time from `ClientRecord` + `Option<ToolGrant>` — never bulk-loaded.
   213	
   214	## 6. Acceptance criteria
   215	
   216	- **AC-1 Gateway dispatch.** Unknown tool → `ToolError::BadParams { detail: "unknown tool name" }`. Handler dispatch ONLY after AC-2..AC-5 pass.
   217	- **AC-2 Scope + exposure authorization (cycle-4 decision-order locked).** Per-dispatch, in this exact order: (i) `ClientRecord.revoked_at.is_some()` → `ToolError::PairingRevoked`. (ii) `auth::resolve_tool_grant(client_id, tool_name)` indexed query: absent grant OR `grant.exposure != Invocable` → `ToolError::ExposureForbidden { tool_name }`. (iii) grant present AND invocable, but `tool.description().scopes_required.is_subset(grant.scopes_granted)` is false → `ToolError::Unauthorized { missing_scope }`. (iv) caller-asserted `granted_scopes` or `conversation_id` in `params` → `ToolError::BadParams`. `McpActor::Client` constructed ONLY after manifest resolution (post step iii pass).
   218	- **AC-3a Handle lifecycle.** First call no handle → mint(client_id), dispatch, return new handle. Subsequent call with handle → resolve `(client_id, handle)`, refresh last_touched_at, dispatch. Expired handle (24h sliding) → silent mint of replacement (transparent). Revoked handle → `ConversationRevoked`. Caller-asserted `conversation_id` in params → `BadParams`.
   219	- **AC-3b Cross-session + cross-client tests.** (a) New MCP session with same client_id + non-revoked handle → resolves successfully, handler sees same handle. (b) Different client_id presenting another client's handle → `ConversationRevoked` (binding enforcement). Both integration tests required.
   220	- **AC-4 Pairing revocation.** Per-dispatch `ClientRecord` load → revoked → `PairingRevoked`. Propagates within ≤ 1 call per ADR-0102 §C.
   221	- **AC-5 Rate limit (cycle-5 tx order).** Atomic reservation via explicit transaction sequence: `BEGIN IMMEDIATE` → `DELETE FROM mcp_tool_call_ledger WHERE client_id=?1 AND tool_name=?2 AND called_at < (?3 - window_seconds_ms)` (prune) → `SELECT COUNT(*) FROM mcp_tool_call_ledger WHERE client_id=?1 AND tool_name=?2` → if `count >= max_calls` ROLLBACK + return `RateLimited { retry_after_seconds }`; else `INSERT INTO mcp_tool_call_ledger VALUES (?1, ?2, ?3)` → `COMMIT`. Key tuple: `(ActorKind::McpClient, McpClientId, ScopedName)` (ledger is MCP-only so ActorKind is implicit; audit + signal carry the full tuple). Concurrent burst test required.
   222	- **AC-6 Audit append (cycle-6 L6-2: success-only).** Handler success precedes audit append. Gateway calls `AuditLogger::append_with_actor(event="mcp.tool_invoked", actor=&actor, fields=AuditFields::new("security", detail).with_request_id(req_id))`. Detail JSON: `{client_id, conversation_handle, tool_name, params_hash, response_hash, mutation_cursor?, revocation_state?}` — `mutation_cursor` is IDs-only `serde_json::Value` (max depth 4, max 2 KiB; truncate → `"truncated_oversize"`), required for `Side::Write` (handler-filled per `taxonomy.rs` rustdoc), omitted for `Side::Read`; `revocation_state` only present if revocation lands mid-call per ADR-0102 §C. Auth-state rejections do NOT write an audit row — handled via `McpInvocationRejected` signal (AC-7) per L6-2. JSONL append fails → insert into `mcp_audit_outbox` (operator-only access via existing `get_audit_log_records` / `export_audit_log` paths; no MCP exposure, no separate CLI). Outbox insert also fails → `ToolError::Internal { trace_id }` + Suite-S alert `mcp_audit_double_failure`. Raw params/responses NEVER stored. Audit is post-commit attribution, not source-of-order truth.
   223	- **AC-7 Signal emission (cycle-7 with reject-signal durability).** TWO new `SignalType` variants, both registered at 5 sites per ADR-0115 (variant + `from_name` + `canonical_name` + known-names const + `policy_for`); both `SignalPolicy::local_observation + NonPiiMetadata`: (a) `SignalType::McpToolInvoked` on dispatch success — payload `{tool_name, client_id, conversation_handle}`. (b) `SignalType::McpInvocationRejected` on auth-state rejection — payload `{client_id_or_unresolved, reject_reason, tool_name_or_unresolved}`. Reject-signal emission uses the emit-or-log pattern: on emission failure, fallback to operator warning log + Suite-S alert `mcp_reject_signal_emit_failed` (closes cycle-6 challenge MED — replaces audit forensics for rejects so emission failure cannot become a forensic blind spot). `client_id_or_unresolved` is either the resolved `McpClientId` or the literal `"unresolved"` (Gate 0a/0b failures, missing-client). `tool_name_or_unresolved` always present (even Gate 0 failures echo the envelope's `tool_name`). NO raw params/response in either event.
   224	- **AC-8 CI gate.** `scripts/check_mcp_tool_handler_allowlist.sh` rejects direct DB writes from `services/mcp_v2/handlers/*.rs`. Wired in CI.
   225	- **AC-9 Required checks.** `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green.
   226	- **AC-10 Legacy coexistence.** Legacy `src-tauri/src/bridges/mcp.rs:402` + `src-tauri/src/mcp/main.rs:1379` continue with `Actor::Agent`; W1-A does NOT touch them. Test fixture at `src-tauri/src/bridges/mcp.rs:737` left `todo!()` (W2+ fills when first McpClient handler ships). Namespace collision CI (was cycle-4 #7) is OUT of W1-A scope per cycle-5 spinoff Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" (Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).
   227	- **AC-11 Key custody (cycle-5 spelled out).** `auth::verify_transport_hmac` wraps loaded key bytes in `Zeroizing<[u8; 32]>` from the `zeroize` crate; per `Zeroize` Drop impl the buffer is wiped on function exit. Pattern follows `SecretBytes32` precedent at `src-tauri/src/surface_runtime/hmac.rs:643` (NOT the line 623 key derivation, which is a separate concern). `ClientRecord` never holds raw key bytes — only `KeychainRef`. Tests: (a) `ClientRecord` struct field shape is `KeychainRef`, not `[u8; 32]`; (b) post-call snapshot of stack/heap shows no zeroed-pattern lingering bytes for the loaded key.
   228	- **AC-12 Timing oracle (cycle-6 expanded for Gate 0).** Gateway calls `tokio::time::sleep_until(start + Duration::from_millis(10))` before returning ANY of: `BadParams { detail: "invalid_signature" }`, `BadParams { detail: "nonce_replayed" }` (Gate 0 failures per L6-1), `PairingRevoked`, `ExposureForbidden`, `Unauthorized`, `ConversationRevoked`. Test asserts measured wall-clock from request entry to error response is `>= 10ms` for ALL branches (missing-client, invalid-HMAC, replayed-nonce, missing-grant, non-invocable, missing-scope, expired-handle, cross-client-handle). NO upper bound assertion (per cycle-4 challenge/CSO portability finding). Identical error-shape constraint preserved: same JSON keys, same JSON-canonical byte length within 16 bytes (BadParams variants distinguishable only by `detail` field content — same shape).
   229	
   230	## 7. Test plan
   231	
   232	- **Unit**: dispatch, unknown-tool, missing-scope, ExposureForbidden, revoked-pairing, revoked-handle, first-write-mint, expired-handle-replacement, raw-conversation-id-rejection, raw-granted-scopes-rejection, cross-client-handle-rejection.
   233	- **Integration**: stub `McpToolHandler` proving (a) audit row iff handler success, (b) JSONL fail → outbox path, (c) JSONL+outbox fail → Internal + Suite-S alert, (d) signal payload carries only `tool_name + client_id + conversation_handle`, (e) rate-limit ledger consumes per full tuple atomically, (f) cross-session same-client (AC-3b.a), (g) cross-client rejection (AC-3b.b), (h) revocation within 1 call.
   234	- **Property**: HMAC canonical-JSON parity for `params_hash`.
   235	- **Concurrency**: `BEGIN IMMEDIATE` rate-limit burst test.
   236	- **Timing**: AC-12 oracle test — missing-client vs invalid-HMAC indistinguishable in error shape + bounded floor.
   237	- **CI lint**: planted direct-conn fixture rejected.
   238	
   239	## 8. Security gates (CSO + plan-devex-review at L0 AND L2)
   240	
   241	Per ADR-0102 §C four gates + W0 cycle-7 amendment + cycle-3 ACs:
   242	1. Caller scopes ignored at dispatch (AC-2).
   243	2. Caller conversation IDs rejected (AC-3a).
   244	3. Audit hashes keyed HMAC-SHA256, raw data forbidden (AC-6).
   245	4. Signal no-leak (AC-7).
   246	5. Atomic pre-dispatch rate limit (AC-5).
   247	6. Per-dispatch revocation, ≤ 1-call propagation (AC-2 + AC-3a + AC-4).
   248	7. Per-tool exposure tier enforced via `ExposureForbidden` variant (AC-2 + Deliverable 0).
   249	8. Handle client_id binding (AC-3b + Deliverable 0).
   250	9. Key custody — single-verify, no caching (AC-11).
   251	10. Timing oracle — uniform failure shape (AC-12).
   252	11. Audit double-failure → Internal + Suite-S alert (AC-6).
   253	12. Mid-call revocation captured in audit `revocation_state` (AC-6 + ADR-0102 §C).
   254	
   255	## 9. Out-of-scope findings → DailyOS Maintenance
   256	
   257	L0/L2 findings not bound by AC-1..AC-12 or by named ADR sections file as separate Linear tickets to `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. Already-identified followups to file at L0 close:
   258	- "v1.4.7 W1.5 — legacy MCP retirement plan" (ADR-0102 §C forward-application clarification + plan to flip clients to v2 or deprecate the legacy path).
   259	- "MCP gateway handler crate split" (structural replacement for `check_mcp_tool_handler_allowlist.sh` grep lint when W2+ surfaces a real bypass).
   260	

exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:.docs/_archive/issues/_done/i347-done.md | nl -ba | sed -n '1,220p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
     1	# I347 — SWOT Report Type — Account Analysis from Existing Intelligence
     2	
     3	**Status:** Absorbed into I397
     4	**Priority:** —
     5	**Version:** 0.14.0
     6	
     7	## Summary
     8	
     9	I347 described a SWOT report generated from existing entity intelligence as a format within Report Mode.
    10	
    11	**This issue is absorbed into I396** (Report Infrastructure). SWOT ships as one of the three report types bundled with the infrastructure (alongside Account Health Review and EBR/QBR). See I396 acceptance criterion 9 for the SWOT-specific requirements.
    12	
    13	The SWOT synthesis approach: active positive signals → Strengths, active risk/warning signals → Weaknesses and Threats (distinction: weaknesses are internal/controllable, threats are external), identified opportunities → Opportunities. One AI synthesis call, stored in `reports` table, consumed mechanically on subsequent views.

exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md | nl -ba | sed -n '260,380p'" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md | nl -ba | sed -n '260,380p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
   260	
   261	## 10. Open questions
   262	
   263	| # | Question | Default | Resolution path |
   264	|---|---|---|---|
   265	| Q1 | Rate-limit default budgets | 60/min per (client_id, tool_name); per-client global 600/min | CSO L0 sign-off on this default; tunable in manifest at pairing |
   266	| Q2 | Forensic outbox drain cadence | On next gateway invocation; if N>100 backlog, drain immediately | architect L0 sign-off |
   267	
   268	## 11. Files (W1-A exclusive ownership)
   269	
   270	| File | State | Owner |
   271	|---|---|---|
   272	| `src-tauri/src/services/mcp_v2/contracts.rs` | Deliverable 0 (W0 amendment: +`ExposureForbidden` variant + parity test) | shared (additive only) |
   273	| `src-tauri/src/services/mcp_v2/gateway.rs` | W0 placeholder → W1-A body | exclusive |
   274	| `src-tauri/src/services/mcp_v2/auth.rs` | W0 placeholder → W1-A body | exclusive |
   275	| `src-tauri/src/services/mcp_v2/audit.rs` | W0 placeholder → W1-A body | exclusive |
   276	| `src-tauri/src/services/mcp_v2/actor_policy.rs` | W0 placeholder → W1-A body | exclusive |
   277	| `src-tauri/src/services/mcp_v2/taxonomy.rs` | W0 placeholder → W1-A trait + error type only | exclusive (interface only; W1-B fills loader) |
   278	| `src-tauri/src/services/mcp_v2/handlers/*.rs` | W0 placeholders | **untouched** |
   279	| `src-tauri/src/signals/policy_registry.rs` | function-level addition (5 sites) | shared (additive) |
   280	| `src-tauri/src/migrations.rs` | new v241–v243 entries | shared (additive) |
   281	| `src-tauri/src/audit_log.rs` + `audit.rs` | **read-only** (reused via `append_with_actor`) | shared |
   282	| `src-tauri/src/bridges/mcp.rs` | **untouched** (production); test fixture at line 737 untouched | n/a |
   283	| `src-tauri/scripts/check_mcp_tool_handler_allowlist.sh` | NEW | exclusive |
   284	| `.docs/decisions/0102-abilities-as-runtime-contract.md` | Deliverable 0 (cycle-7 amendment +6 lines) | shared (additive) |
   285	| `.docs/plans/v1.4.7-waves.md` | Deliverable 0 (lines 451 + 467 stale-text fix) | shared |
   286	
   287	## 12. Depends-on
   288	
   289	- W0 (PR #322) — merged ✅.
   290	- Deliverable 0 — lands as part of this lane's first commit.
   291	- v1.4.5 frozen contracts — NOT a hard dep for W1-A.
   292	
   293	## 13. Definition of Done
   294	
   295	§6 AC-1..AC-12 met; L0 unanimous APPROVE on cycle 3; Deliverable 0 commit + W1-A body commit each pass L2; commit-msg `L2-status: passed`; PR opens against `dev` after L2 clears.

 succeeded in 0ms:
   260	- AC-8: full subprocess MCP handshake test; stdout protocol-only assertion
   261	- AC-10: CI
   262	- AC-11: synthetic config triggers refusal with expected error text
   263	- AC-12: subprocess negative tests per failure mode; env-wipe verified via in-process getenv (NOT /proc — acknowledged limitation)
   264	- AC-13: replay test asserts second tools/call with same envelope nonce → BadParams from gateway
   265	- AC-14: golden fixture round-trip tests across all 10 fixtures; duplicate-key fixture asserts error
   266	
   267	## 8. Security gates
   268	
   269	`/cso` mandatory. Cycle-5 must verify the local-to-local same-machine scope statement (§0) makes the env-asserted identity model acceptable, AND that the trust-model differentiation (stdio = internal hygiene; WP/SDK = real auth) is documented honestly without misleading W1-A's "every call HMAC-verified" framing.
   270	
   271	## 9. Path-α (separate Linear tickets)
   272	
   273	- `client_label` schema for pair-by-name revoke (cycle-2)
   274	- `dailyos-mcp-v2 migrate-config` subcommand (AC-11 error reference)
   275	- Binary self-identification via `--version-id` (AC-11 challenge cycle-3)
   276	- Per-process keychain ACL gating transport_key reads (post-v1.4.7 hardening)
   277	- Loopback HTTP transport (Phase 2 — out of §0 scope for v1.4.7)
   278	- Tauri operator pairing UI (post-v1.4.7)
   279	- MCP client SDK code samples + JCS reference implementations
   280	
   281	## 10. Depends-on
   282	
   283	- W1-A merged (PR #347): Gateway + auth + nonce ledger + keychain substrate
   284	- W1-B merged (PR #347): YamlTaxonomyCatalog + Gateway::seal
   285	- rmcp crate already in deps
   286	
   287	## 11. Definition of Done
   288	
   289	§5 AC-1..AC-14 met. L0 unanimous APPROVE. L2 unanimous APPROVE bounded by AC. Commit-msg `L2-status: passed`. Pushed to PR #347. CI green including subprocess + JCS golden tests.
   290	
   291	**Trust-model honest framing in proof bundle:** W1-A claims "every Gateway::handle_tool_call verifies HMAC + consumes nonce" — that remains true. For stdio transport, the signer IS the verifier (same process holds the key). The real client auth is the startup env-assertion + keychain cross-check (AC-12). For non-stdio transports (WP adapter, SDK clients), the HMAC verification authenticates the external signer to the gateway. Both transport classes share the same gateway substrate; their trust boundaries are at different points.
   292	
   293	## 12. Open questions resolved
   294	
   295	| # | Question | Resolution |
   296	|---|---|---|
   297	| Q1 | rmcp wrapper format | DELETED (cycle-5: transport self-signs internally; no `_dailyos_*` in client params) |
   298	| Q2 | pair CLI argparse | clap |
   299	| Q3 | empty handler set at boot | warning + strict-mode env |
   300	| Q4 | HTTP transport | path-α (§0 scope out for v1.4.7) |
   301	| Q5 (NEW) | trust-model class | Differentiated by transport: stdio = startup identity + internal hygiene; WP/SDK = per-envelope HMAC real auth |
   302	| Q6 (NEW) | local-to-local same-machine scope | locked in §0 (user direction) |
   303	
   304	## 13. Reviewer dispatch
   305	
   306	- **CSO** (mandatory; verify local-to-local scope statement + honest trust framing)
   307	- **/codex challenge** (adversarial within scope; out-of-scope vectors documented)
   308	- **architect-reviewer** (V2ServerHandler shape; transport-internal signing seam)
   309	- **/plan-devex-review** (Claude Desktop + Cursor compatibility; client integration UX)

exec
/bin/zsh -lc 'git diff --stat dev..v1.4.7-w1-foundation -- src-tauri/src/services/mcp_v2 .docs/plans/v1.4.7-w1-foundation .docs/plans/v1.4.7-waves.md src-tauri/src/migrations/255_mcp_client_manifest.sql src-tauri/src/migrations/256_mcp_conversation_handle.sql src-tauri/src/migrations/257_mcp_transport_nonce_ledger.sql src-tauri/src/migrations/258_mcp_rate_limit_and_audit_outbox.sql' in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
 .../plans/v1.4.7-w1-foundation/dos-168-l0-plan.md  |  295 +
 .../plans/v1.4.7-w1-foundation/dos-175-l0-plan.md  |  264 +
 .../plans/v1.4.7-w1-foundation/dos-478-l0-plan.md  |  227 +
 .../v1.4.7-w1-foundation/dos-624-cso-amendment.md  |  132 +
 .../dos-mcp-transport-l0-plan.md                   |  309 +
 .../reviews/dos-175-cycle-1/architect.md           | 5011 +++++++++++++
 .../reviews/dos-175-cycle-1/codex-challenge.md     | 7437 ++++++++++++++++++++
 .../reviews/dos-175-cycle-1/consult.md             | 2133 ++++++
 .../reviews/dos-175-cycle-1/devex.md               | 3792 ++++++++++
 .docs/plans/v1.4.7-waves.md                        |   11 +-
 .../src/migrations/255_mcp_client_manifest.sql     |   19 +
 .../src/migrations/256_mcp_conversation_handle.sql |   11 +
 .../migrations/257_mcp_transport_nonce_ledger.sql  |   17 +
 .../258_mcp_rate_limit_and_audit_outbox.sql        |   18 +
 src-tauri/src/services/mcp_v2/actor_policy.rs      |   83 +-
 src-tauri/src/services/mcp_v2/audit.rs             |  347 +-
 src-tauri/src/services/mcp_v2/auth.rs              |  570 +-
 src-tauri/src/services/mcp_v2/contracts.rs         |  105 +-
 src-tauri/src/services/mcp_v2/gateway.rs           |  887 ++-
 src-tauri/src/services/mcp_v2/handlers/mod.rs      |    2 +
 .../src/services/mcp_v2/handlers/registration.rs   |   59 +
 .../mcp_v2/handlers/tool_account_status.rs         |  290 +
 src-tauri/src/services/mcp_v2/mod.rs               |    1 +
 src-tauri/src/services/mcp_v2/taxonomy.rs          |  715 +-
 src-tauri/src/services/mcp_v2/transport.rs         |  464 ++
 25 files changed, 23164 insertions(+), 35 deletions(-)

exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/gateway.rs | nl -ba | sed -n '260,360p'" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/audit.rs | nl -ba | sed -n '1,220p'" in /Users/jamesgiroux/Documents/dailyos-repo
exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/auth.rs | nl -ba | sed -n '220,330p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
   260	                    result: McpToolResult::Error { error },
   261	                }
   262	            }
   263	        }
   264	    }
   265	
   266	    /// Inner per-dispatch flow. On success: handler result + preissued next
   267	    /// nonce + active conversation handle. On failure: `GatewayFailure` with
   268	    /// safe attribution + the preissued next nonce (if Gate 0b admitted) so
   269	    /// rejection responses carry the SAME nonce the caller's next request
   270	    /// must present, per ADR-0102 §C.bis.refresh.
   271	    fn dispatch(
   272	        &self,
   273	        conn: &mut Connection,
   274	        asserted_client_id: &McpClientId,
   275	        envelope: &McpToolRequestEnvelope,
   276	        signature: &[u8],
   277	    ) -> Result<Dispatched, Box<GatewayFailure>> {
   278	        // Gate 0a: HMAC over WHOLE envelope including request_nonce. On
   279	        // failure, attribution is "unresolved" — the asserted client_id is
   280	        // not yet verified, so AC-7 says we MUST NOT signal it.
   281	        if let Err(err) = auth::verify_transport_hmac(conn, asserted_client_id, envelope, signature)
   282	        {
   283	            return Err(auth_to_failure(err, envelope, None, None));
   284	        }
   285	
   286	        // Gate 0b: fail-closed consume + preissue per ADR-0102 §C.bis.refresh
   287	        // + §C.bis.fail-closed. After this point, the nonce is permanently
   288	        // consumed (commit happens inside auth::verify_and_consume_and_preissue
   289	        // phase a); the returned `next_nonce` is what the response envelope
   290	        // MUST carry whether the rest of dispatch succeeds or fails.
   291	        let next_nonce =
   292	            match auth::verify_and_consume_and_preissue(conn, asserted_client_id, &envelope.request_nonce) {
   293	                Ok(nonce) => nonce,
   294	                Err(err) => {
   295	                    return Err(auth_to_failure(err, envelope, None, None))
   296	                }
   297	            };
   298	
   299	        // Manifest gates per AC-2 decision order. From here on attribution
   300	        // is verified (HMAC + nonce passed) so client_id is admissible in
   301	        // reject signals + we carry the preissued next_nonce on rejection.
   302	        let preissued = Some(next_nonce.clone());
   303	        let record = match auth::load_client_record(conn, asserted_client_id) {
   304	            Ok(r) => r,
   305	            Err(err) => {
   306	                return Err(auth_to_failure(
   307	                    err,
   308	                    envelope,
   309	                    Some(asserted_client_id),
   310	                    preissued.clone(),
   311	                ))
   312	            }
   313	        };
   314	        if record.revoked_at.is_some() {
   315	            return Err(Box::new(GatewayFailure {
   316	                error: ToolError::PairingRevoked,
   317	                reject_reason: "pairing_revoked".to_string(),
   318	                tool_name_for_signal: Some(envelope.tool_name.clone()),
   319	                client_id_for_signal: Some(asserted_client_id.clone()),
   320	                preissued_next_nonce: preissued.clone(),
   321	            }));
   322	        }
   323	
   324	        let grant = match auth::resolve_tool_grant(conn, asserted_client_id, &envelope.tool_name) {
   325	            Ok(g) => g,
   326	            Err(err) => {
   327	                return Err(auth_to_failure(err, envelope, Some(asserted_client_id), preissued.clone()))
   328	            }
   329	        };
   330	        let Some(grant) = grant else {
   331	            return Err(Box::new(GatewayFailure {
   332	                error: ToolError::ExposureForbidden {
   333	                    tool_name: envelope.tool_name.clone(),
   334	                },
   335	                reject_reason: "absent_grant".to_string(),
   336	                tool_name_for_signal: Some(envelope.tool_name.clone()),
   337	                client_id_for_signal: Some(asserted_client_id.clone()),
   338	                preissued_next_nonce: preissued.clone(),
   339	            }));
   340	        };
   341	        if !matches!(
   342	            grant.exposure,
   343	            abilities_runtime::abilities::registry::McpExposure::Invocable
   344	        ) {
   345	            return Err(Box::new(GatewayFailure {
   346	                error: ToolError::ExposureForbidden {
   347	                    tool_name: envelope.tool_name.clone(),
   348	                },
   349	                reject_reason: "non_invocable".to_string(),
   350	                tool_name_for_signal: Some(envelope.tool_name.clone()),
   351	                client_id_for_signal: Some(asserted_client_id.clone()),
   352	                preissued_next_nonce: preissued.clone(),
   353	            }));
   354	        }
   355	
   356	        // Handler lookup. Unknown tool → BadParams per AC-1.
   357	        let Some(handler) = self.handlers.get(&envelope.tool_name) else {
   358	            return Err(Box::new(GatewayFailure {
   359	                error: ToolError::BadParams {
   360	                    detail: "unknown tool name".to_string(),

 succeeded in 0ms:
     1	//! MCP v2 audit log writer.
     2	//!
     3	//! This module is intentionally a thin MCP-specific layer over
     4	//! [`crate::audit_log::AuditLogger::append_with_actor`]. It owns the MCP
     5	//! success-row privacy contract: raw request/response payloads are converted
     6	//! to keyed HMAC-SHA256 hashes before anything reaches JSONL audit storage.
     7	
     8	use std::process::Command;
     9	use std::sync::Arc;
    10	
    11	use chrono::{SecondsFormat, Utc};
    12	use rand::Rng;
    13	use ring::hmac;
    14	use rusqlite::params;
    15	use serde_json::{Map, Value};
    16	use zeroize::Zeroizing;
    17	
    18	use abilities_runtime::abilities::registry::Actor;
    19	
    20	use crate::audit_log::{AuditError as AuditLogError, AuditFields, AuditLogger};
    21	
    22	const AUDIT_KEY_SERVICE: &str = "com.dailyos.desktop.mcp-v2-audit";
    23	const AUDIT_KEY_ACCOUNT: &str = "mcp-v2-audit-hmac-key";
    24	const AUDIT_KEY_BYTES: usize = 32;
    25	const PARAM_PAYLOAD_KEYS: &[&str] = &["params", "parameters"];
    26	const RESPONSE_PAYLOAD_KEYS: &[&str] = &["response", "response_or_error", "result"];
    27	
    28	/// MCP-specific audit write failures.
    29	#[derive(Debug, thiserror::Error)]
    30	pub enum AuditError {
    31	    #[error("audit key unavailable: {0}")]
    32	    Keychain(String),
    33	    #[error("audit detail serialization failed: {0}")]
    34	    Serialization(#[from] serde_json::Error),
    35	    #[error("audit log append failed: {0}")]
    36	    Append(String),
    37	    #[error("audit log append failed and audit outbox insert also failed: append={append_error}; outbox={outbox_error}")]
    38	    DoubleFailure {
    39	        append_error: String,
    40	        outbox_error: String,
    41	    },
    42	}
    43	
    44	/// Write an MCP success-row audit event.
    45	///
    46	/// `detail` may include raw top-level `params` and `response` /
    47	/// `response_or_error` / `result` fields. Those fields are never persisted:
    48	/// this helper replaces them with `params_hash` and `response_hash`, computed
    49	/// with a per-install key stored in Keychain.
    50	pub fn write(
    51	    actor: &Actor,
    52	    event: &str,
    53	    detail: Value,
    54	    request_id: Option<String>,
    55	) -> Result<(), AuditError> {
    56	    let audit_key = load_or_create_audit_key()?;
    57	    let detail_with_hashes = detail_with_hashes(actor, detail, &audit_key)?;
    58	
    59	    let mut fields = AuditFields::new("security", detail_with_hashes.clone());
    60	    if let Some(id) = request_id.clone() {
    61	        fields = fields.with_request_id(id);
    62	    }
    63	
    64	    let mut logger = AuditLogger::new(crate::audit_log::default_audit_log_path());
    65	    match logger.append_with_actor(event, actor, fields) {
    66	        Ok(()) => Ok(()),
    67	        Err(AuditLogError::Write(append_error)) => {
    68	            let detail_json = serde_json::to_string(&detail_with_hashes)?;
    69	            insert_outbox(event, &detail_json, actor_kind(actor), request_id.as_deref()).map_err(
    70	                |outbox_error| AuditError::DoubleFailure {
    71	                    append_error,
    72	                    outbox_error,
    73	                },
    74	            )
    75	        }
    76	        Err(error) => Err(AuditError::Append(error.to_string())),
    77	    }
    78	}
    79	
    80	fn detail_with_hashes(
    81	    actor: &Actor,
    82	    detail: Value,
    83	    audit_key: &[u8; AUDIT_KEY_BYTES],
    84	) -> Result<Value, serde_json::Error> {
    85	    let params = payload_for_keys(&detail, PARAM_PAYLOAD_KEYS);
    86	    let response = payload_for_keys(&detail, RESPONSE_PAYLOAD_KEYS);
    87	
    88	    let mut sanitized = sanitize_detail(detail);
    89	    sanitized.insert(
    90	        "params_hash".to_string(),
    91	        Value::String(hmac_json(audit_key, &params)?),
    92	    );
    93	    sanitized.insert(
    94	        "response_hash".to_string(),
    95	        Value::String(hmac_json(audit_key, &response)?),
    96	    );
    97	    add_actor_attribution(actor, &mut sanitized);
    98	
    99	    Ok(Value::Object(sanitized))
   100	}
   101	
   102	fn payload_for_keys(detail: &Value, keys: &[&str]) -> Value {
   103	    let Some(map) = detail.as_object() else {
   104	        return Value::Null;
   105	    };
   106	    keys.iter()
   107	        .find_map(|key| map.get(*key).cloned())
   108	        .unwrap_or(Value::Null)
   109	}
   110	
   111	fn sanitize_detail(detail: Value) -> Map<String, Value> {
   112	    let mut map = match detail {
   113	        Value::Object(map) => map,
   114	        _ => Map::new(),
   115	    };
   116	    for key in PARAM_PAYLOAD_KEYS
   117	        .iter()
   118	        .chain(RESPONSE_PAYLOAD_KEYS.iter())
   119	        .copied()
   120	    {
   121	        map.remove(key);
   122	    }
   123	    map
   124	}
   125	
   126	fn add_actor_attribution(actor: &Actor, detail: &mut Map<String, Value>) {
   127	    if let Actor::McpClient {
   128	        client_id,
   129	        conversation_handle,
   130	    } = actor
   131	    {
   132	        detail
   133	            .entry("client_id".to_string())
   134	            .or_insert_with(|| Value::String(client_id.as_str().to_string()));
   135	        detail
   136	            .entry("conversation_handle".to_string())
   137	            .or_insert_with(|| {
   138	                conversation_handle
   139	                    .as_ref()
   140	                    .map(|handle| Value::String(handle.as_str().to_string()))
   141	                    .unwrap_or(Value::Null)
   142	            });
   143	    }
   144	}
   145	
   146	fn hmac_json(key_bytes: &[u8; AUDIT_KEY_BYTES], value: &Value) -> Result<String, serde_json::Error> {
   147	    let canonical = serde_json::to_vec(&canonical_value(value))?;
   148	    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
   149	    Ok(hex::encode(hmac::sign(&key, &canonical).as_ref()))
   150	}
   151	
   152	fn canonical_value(value: &Value) -> Value {
   153	    match value {
   154	        Value::Array(items) => Value::Array(items.iter().map(canonical_value).collect()),
   155	        Value::Object(map) => {
   156	            let mut keys: Vec<&String> = map.keys().collect();
   157	            keys.sort();
   158	            let mut ordered = Map::new();
   159	            for key in keys {
   160	                if let Some(value) = map.get(key) {
   161	                    ordered.insert(key.clone(), canonical_value(value));
   162	                }
   163	            }
   164	            Value::Object(ordered)
   165	        }
   166	        _ => value.clone(),
   167	    }
   168	}
   169	
   170	fn actor_kind(actor: &Actor) -> &'static str {
   171	    match actor {
   172	        Actor::Agent => "agent",
   173	        Actor::User => "user",
   174	        Actor::Admin => "admin",
   175	        Actor::System => "system",
   176	        Actor::SurfaceClient { .. } => "surface_client",
   177	        Actor::McpClient { .. } => "mcp_client",
   178	    }
   179	}
   180	
   181	fn insert_outbox(
   182	    event: &str,
   183	    detail_json: &str,
   184	    actor_kind: &str,
   185	    request_id: Option<&str>,
   186	) -> Result<(), String> {
   187	    let event = event.to_string();
   188	    let detail_json = detail_json.to_string();
   189	    let actor_kind = actor_kind.to_string();
   190	    let request_id = request_id.map(str::to_string);
   191	    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
   192	
   193	    if let Some(service) = crate::db_service::try_global() {
   194	        let writer = service.writer();
   195	        return writer
   196	            .call_sync(move |conn| {
   197	                insert_outbox_row(
   198	                    conn,
   199	                    &event,
   200	                    &detail_json,
   201	                    &actor_kind,
   202	                    request_id.as_deref(),
   203	                    &created_at,
   204	                )
   205	            })
   206	            .map_err(|error| error.to_string());
   207	    }
   208	
   209	    let db = crate::db::ActionDb::open(Arc::new(crate::db::LocalKeychain::new()))
   210	        .map_err(|error| error.to_string())?;
   211	    insert_outbox_row(
   212	        db.conn_ref(),
   213	        &event,
   214	        &detail_json,
   215	        &actor_kind,
   216	        request_id.as_deref(),
   217	        &created_at,
   218	    )
   219	    .map_err(|error| error.to_string())
   220	}

 succeeded in 0ms:
   220	        },
   221	    }))
   222	}
   223	
   224	/// Verify HMAC-SHA256 signature over canonical-JSON bytes of the WHOLE
   225	/// request envelope (including `request_nonce`) per cycle-7 challenge HIGH.
   226	/// Loads key into a `Zeroizing` wrapper for one verify then drops per AC-11.
   227	///
   228	/// Returns the asserted `McpClientId` on verify success. Caller passes the
   229	/// candidate `client_id` (out-of-band: transport-layer header). The HMAC
   230	/// inclusion of the WHOLE envelope means an attacker cannot swap nonces on
   231	/// captured signed payloads (closes cycle-7 challenge HIGH).
   232	pub fn verify_transport_hmac(
   233	    conn: &Connection,
   234	    asserted_client_id: &McpClientId,
   235	    envelope: &McpToolRequestEnvelope,
   236	    signature: &[u8],
   237	) -> Result<McpClientId, AuthError> {
   238	    let record = load_client_record(conn, asserted_client_id)?;
   239	    let key_bytes = Zeroizing::new(load_transport_key(&record.transport_key_ref)?);
   240	    let mac_key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes.as_ref());
   241	    let canonical = serde_json::to_vec(envelope)?;
   242	    hmac::verify(&mac_key, &canonical, signature).map_err(|_| AuthError::InvalidSignature)?;
   243	    Ok(asserted_client_id.clone())
   244	}
   245	
   246	/// Fail-closed consume + preissue per ADR-0102 §C.bis.fail-closed.
   247	///
   248	/// Implemented as TWO separate phases per L2 cycle-1 code-reviewer HIGH:
   249	/// (a) commit the consume in its own transaction — once committed the nonce
   250	///     is permanently consumed regardless of downstream failure;
   251	/// (b) insert the next nonce in a second statement (autocommit). If (b)
   252	///     fails the consume from (a) is NOT rolled back — caller receives
   253	///     `AuthError::PreissueFailed`, must re-pair.
   254	///
   255	/// Cycle-7 CSO HIGH + architect MED on issue-next failure path is closed
   256	/// this way: consume is fail-closed, preissue failure is surfaced explicitly.
   257	pub fn verify_and_consume_and_preissue(
   258	    conn: &mut Connection,
   259	    client_id: &McpClientId,
   260	    presented: &OpaqueNonce,
   261	) -> Result<OpaqueNonce, AuthError> {
   262	    let now = now_millis();
   263	
   264	    // Phase (a): consume-only transaction. Either consumes successfully (and
   265	    // commits — fail-closed point) or returns the typed AuthError.
   266	    {
   267	        let tx = conn.transaction()?;
   268	        let consumed = tx.execute(
   269	            "UPDATE mcp_transport_nonce_ledger SET consumed_at = ?3 \
   270	             WHERE client_id = ?1 AND nonce = ?2 \
   271	               AND consumed_at IS NULL AND expires_at >= ?3",
   272	            params![client_id.as_str(), presented.as_str(), now],
   273	        )?;
   274	        if consumed != 1 {
   275	            // Disambiguate replayed vs absent/expired for caller-error shape
   276	            // per §C.bis.schema. Same wire shape; only `detail` differs.
   277	            let already_consumed: Option<i64> = tx
   278	                .query_row(
   279	                    "SELECT consumed_at FROM mcp_transport_nonce_ledger \
   280	                     WHERE client_id = ?1 AND nonce = ?2",
   281	                    params![client_id.as_str(), presented.as_str()],
   282	                    |row| row.get(0),
   283	                )
   284	                .optional()?
   285	                .flatten();
   286	            // tx auto-rolls back on drop — fine here, nothing has changed.
   287	            return Err(if already_consumed.is_some() {
   288	                AuthError::NonceReplayed
   289	            } else {
   290	                AuthError::InvalidSignature
   291	            });
   292	        }
   293	        tx.commit()?;
   294	        // Fail-closed point: consume is now permanent in the ledger.
   295	    }
   296	
   297	    // Phase (b): preissue next nonce. If this fails, consume from (a) stays
   298	    // committed; caller learns the failure via `AuthError::PreissueFailed`
   299	    // and must re-pair (or use the recovery path the gateway provides if
   300	    // any). Per ADR-0102 §C.bis.fail-closed.
   301	    let next_nonce = OpaqueNonce::new(random_hex(NONCE_BYTES));
   302	    conn.execute(
   303	        "INSERT INTO mcp_transport_nonce_ledger \
   304	         (nonce, client_id, issued_at, expires_at, consumed_at) \
   305	         VALUES (?1, ?2, ?3, ?4, NULL)",
   306	        params![
   307	            next_nonce.as_str(),
   308	            client_id.as_str(),
   309	            now,
   310	            now + NONCE_EXPIRY_SECONDS * 1000,
   311	        ],
   312	    )
   313	    .map_err(|err| AuthError::PreissueFailed(err.to_string()))?;
   314	    Ok(next_nonce)
   315	}
   316	
   317	/// Resolve presented handle to a server-side record (or mint a new one if
   318	/// absent). Composite `(handle, client_id)` per ADR-0102 §D.bis cross-client
   319	/// binding: a handle issued to client A presented by client B →
   320	/// `ConversationRevoked`.
   321	pub fn resolve_or_mint_handle(
   322	    conn: &mut Connection,
   323	    client_id: &McpClientId,
   324	    presented: Option<&OpaqueConversationHandle>,
   325	) -> Result<OpaqueConversationHandle, AuthError> {
   326	    let now = now_millis();
   327	    let Some(handle) = presented else {
   328	        return mint_handle(conn, client_id, now);
   329	    };
   330	    let mut stmt = conn.prepare_cached(

exec
/bin/zsh -lc "git show v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/transport.rs | nl -ba | sed -n '180,280p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
   180	            }
   181	            // Catalog entries without a matching registered handler are
   182	            // silently absent — pending in catalog, not invocable yet.
   183	        }
   184	        Ok(ListToolsResult {
   185	            next_cursor: None,
   186	            tools,
   187	        })
   188	    }
   189	
   190	    async fn call_tool(
   191	        &self,
   192	        request: CallToolRequestParam,
   193	        _context: RequestContext<RoleServer>,
   194	    ) -> Result<CallToolResult, ErrorData> {
   195	        // Per L0 AC-2: receive normal MCP `{ name, arguments }` shape;
   196	        // transport builds + signs envelope.
   197	        let tool_name = ScopedName::new(request.name.to_string());
   198	        let params = serde_json::Value::Object(request.arguments.unwrap_or_default());
   199	
   200	        // Pull the preissued nonce (initial or from prior response).
   201	        let request_nonce = {
   202	            let mut guard = self.next_nonce.lock();
   203	            guard.take().ok_or_else(|| {
   204	                ErrorData::internal_error(
   205	                    "mcp_v2 transport: no preissued nonce; re-pair to recover".to_string(),
   206	                    None,
   207	                )
   208	            })?
   209	        };
   210	
   211	        let envelope = McpToolRequestEnvelope {
   212	            request_nonce,
   213	            conversation_handle: None,
   214	            tool_name,
   215	            params,
   216	        };
   217	
   218	        // Sign envelope with process-held transport_key. Matches W1-A
   219	        // `auth::verify_transport_hmac` canonicalization (serde_json::to_vec);
   220	        // see auth.rs:verify_transport_hmac.
   221	        //
   222	        // KNOWN (path-α): L0 AC-14 specified RFC 8785 JCS; W1-A shipped
   223	        // serde_json::to_vec. Flipping either side requires flipping both
   224	        // atomically. Filed as separate ticket.
   225	        let canonical = serde_json::to_vec(&envelope).map_err(|e| {
   226	            ErrorData::internal_error(format!("mcp_v2 envelope canonicalize: {e}"), None)
   227	        })?;
   228	        let mac_key = hmac::Key::new(hmac::HMAC_SHA256, self.transport_key.as_ref().as_ref());
   229	        let signature = hmac::sign(&mac_key, &canonical);
   230	
   231	        // Call gateway. W1-A substrate verifies HMAC, consumes nonce,
   232	        // dispatches handler, audits, signal-emits.
   233	        //
   234	        // Per DOS-175 cycle-2 §3.2: `handle_tool_call` is sync and the
   235	        // handler chain underneath it calls `runtime.block_on(...)` on a
   236	        // captured tokio handle to dispatch async abilities. Calling that
   237	        // directly from this async context would nested-runtime-panic, so
   238	        // we move dispatch onto a blocking-pool thread via
   239	        // `tokio::task::spawn_blocking`. Inside the blocking thread the
   240	        // handler's `block_on` is safe because we are not on a worker.
   241	        let gateway = self.gateway.clone();
   242	        let conn = self.conn.clone();
   243	        let client_id = self.verified_client_id.clone();
   244	        let signature_bytes = signature.as_ref().to_vec();
   245	        let response_envelope = tokio::task::spawn_blocking(move || {
   246	            let mut conn_guard = conn.lock();
   247	            gateway.handle_tool_call(&mut conn_guard, &client_id, envelope, &signature_bytes)
   248	        })
   249	        .await
   250	        .map_err(|join_err| {
   251	            ErrorData::internal_error(
   252	                format!("mcp_v2 dispatch task join failed: {join_err}"),
   253	                None,
   254	            )
   255	        })?;
   256	
   257	        // Store the gateway's preissued next_nonce for the NEXT call.
   258	        // (Even on error responses the gateway returns a preissued nonce
   259	        // so the caller can recover.)
   260	        *self.next_nonce.lock() = Some(response_envelope.next_request_nonce.clone());
   261	
   262	        unwrap_response(response_envelope)
   263	    }
   264	}
   265	
   266	/// Translate the gateway's response envelope into an rmcp result.
   267	fn unwrap_response(env: McpToolResponseEnvelope) -> Result<CallToolResult, ErrorData> {
   268	    match env.result {
   269	        McpToolResult::Ok { value } => {
   270	            let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
   271	            Ok(CallToolResult::success(vec![Content::text(text)]))
   272	        }
   273	        McpToolResult::Error { error } => Err(tool_error_to_mcp_error(error)),
   274	    }
   275	}
   276	
   277	/// Per L0 AC-3 closed-matrix mapping from `ToolError` to `rmcp::ErrorData`.
   278	fn tool_error_to_mcp_error(err: ToolError) -> ErrorData {
   279	    use serde_json::json;
   280	    match err {

codex
REQUEST_CHANGES — I found concrete same-OS-user threats where the dropped gates were doing more than ceremony: WP loopback becomes an unauthenticated local Actor::User oracle, and MCP scope manifests lose a credible client-identity binding if transport keys/HMAC are removed without replacement.

HIGH — `/v1/local/invoke` lets any same-user local process invoke as `Actor::User`. The packet’s handler only checks loopback, then materializes `Actor::User`, and removes HMAC/session/scope gates for that path (`.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:125-136`, `:252-254`). That means a rogue local app can hit localhost and ask DailyOS to decrypt/render private intelligence through the running app, even if SQLCipher/Keychain would have blocked raw DB reads. The old HMAC/session gate defended this exact “local process can connect but lacks pairing credential” case.

HIGH — MCP per-client authorization is not meaningful if `McpClientId` is no longer cryptographically or process-bound. §3e keeps `Actor::McpClient`, scope manifests, and grant enforcement while dropping HMAC verification and transport key custody (`L0-packet.md:204-220`, `:229-231`). In PR #347, the HMAC check verified the asserted client id using the client’s transport key before manifest gates ran (`v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/auth.rs:224-243`); the transport plan also used startup keychain cross-check as stdio identity (`v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:46-58`). Without an equivalent, a same-user process that learns or reuses another client id can claim that client’s grants. This is scope elevation, not abstract transport ceremony.

MED — Compromised WP plugin loses its blast-radius cap. The old `SurfaceClientBridge::authorize` constrained ability access and per-axis budgets (`L0-packet.md:48-49`); the new WP path deliberately removes scope validation and rate limits (`:136`, `:299-303`). A compromised first-party plugin can invoke any user-reachable ability, not just WP-rendering abilities. Confirmation protects some destructive paths, but read exfiltration and non-confirmed mutations remain.

MED — Keyed audit HMAC still defends a residual audit-content threat that SQLCipher does not: a same-user process or app bug with a DB write primitive while DailyOS is unlocked can rewrite plaintext audit details; DB encryption only protects the file at rest. PR #347’s audit layer removed raw params/responses and stored keyed hashes (`v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/audit.rs:44-99`). Row deletion was never solved by this, so this is not a BLOCK, but plaintext/masked audit (`L0-packet.md:225`, `:235`, `:320-324`) should be an explicit product/security choice.

LOW — The close+supersede path risks dropping a non-premise-dependent CSO finding: opaque resource IDs and redaction helper reuse from the old v1.4.7 cycle (`.docs/plans/v1.4.7-waves.md:52-55`). The new KEEP list preserves authorization, actor policy, rate limit, audit, and DOS-624 sensitivity, but does not explicitly carry that resource-ID finding forward (`L0-packet.md:229-233`, `:364-380`). Add it to PR C scope or name its downstream ticket before closing #347.
tokens used
156,954
REQUEST_CHANGES — I found concrete same-OS-user threats where the dropped gates were doing more than ceremony: WP loopback becomes an unauthenticated local Actor::User oracle, and MCP scope manifests lose a credible client-identity binding if transport keys/HMAC are removed without replacement.

HIGH — `/v1/local/invoke` lets any same-user local process invoke as `Actor::User`. The packet’s handler only checks loopback, then materializes `Actor::User`, and removes HMAC/session/scope gates for that path (`.docs/plans/v1.4.4-wp-surface-migration/L0-trust-topology-and-substrate-readers/L0-packet.md:125-136`, `:252-254`). That means a rogue local app can hit localhost and ask DailyOS to decrypt/render private intelligence through the running app, even if SQLCipher/Keychain would have blocked raw DB reads. The old HMAC/session gate defended this exact “local process can connect but lacks pairing credential” case.

HIGH — MCP per-client authorization is not meaningful if `McpClientId` is no longer cryptographically or process-bound. §3e keeps `Actor::McpClient`, scope manifests, and grant enforcement while dropping HMAC verification and transport key custody (`L0-packet.md:204-220`, `:229-231`). In PR #347, the HMAC check verified the asserted client id using the client’s transport key before manifest gates ran (`v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/auth.rs:224-243`); the transport plan also used startup keychain cross-check as stdio identity (`v1.4.7-w1-foundation:.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:46-58`). Without an equivalent, a same-user process that learns or reuses another client id can claim that client’s grants. This is scope elevation, not abstract transport ceremony.

MED — Compromised WP plugin loses its blast-radius cap. The old `SurfaceClientBridge::authorize` constrained ability access and per-axis budgets (`L0-packet.md:48-49`); the new WP path deliberately removes scope validation and rate limits (`:136`, `:299-303`). A compromised first-party plugin can invoke any user-reachable ability, not just WP-rendering abilities. Confirmation protects some destructive paths, but read exfiltration and non-confirmed mutations remain.

MED — Keyed audit HMAC still defends a residual audit-content threat that SQLCipher does not: a same-user process or app bug with a DB write primitive while DailyOS is unlocked can rewrite plaintext audit details; DB encryption only protects the file at rest. PR #347’s audit layer removed raw params/responses and stored keyed hashes (`v1.4.7-w1-foundation:src-tauri/src/services/mcp_v2/audit.rs:44-99`). Row deletion was never solved by this, so this is not a BLOCK, but plaintext/masked audit (`L0-packet.md:225`, `:235`, `:320-324`) should be an explicit product/security choice.

LOW — The close+supersede path risks dropping a non-premise-dependent CSO finding: opaque resource IDs and redaction helper reuse from the old v1.4.7 cycle (`.docs/plans/v1.4.7-waves.md:52-55`). The new KEEP list preserves authorization, actor policy, rate limit, audit, and DOS-624 sensitivity, but does not explicitly carry that resource-ID finding forward (`L0-packet.md:229-233`, `:364-380`). Add it to PR C scope or name its downstream ticket before closing #347.
