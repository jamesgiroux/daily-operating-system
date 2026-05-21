Reading additional input from stdin...
OpenAI Codex v0.130.0
--------
workdir: /private/tmp/dailyos-v147-w1a
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019e4b9c-2bd5-7050-9047-cc833b9266fb
--------
user
Review the L0 plan packet at .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md as a developer-experience reviewer for the Claude Desktop MCP integration surface. Lens: end-to-end DX for the user pairing Claude Desktop, listing tools, calling tools, getting useful output. Onboarding friction. Tool name + summary + when_to_call clarity for Claudes selection.

K-in obligation: grep docs/solutions/ and .docs/decisions/ for ADR-0083 product vocabulary + ADR-0128 MCP product surface + DOS-478 catalog conventions. Also read src-tauri/resources/mcp_v2/tool_descriptions.yaml entries for account_status and daily_briefing.

Threat topology: local-to-local single-user. Don\t flag multi-actor scope gates or principal differentiation as DX issues — they would degrade single-user DX, not improve it.

Output structure:
- One-line VERDICT: APPROVE | NEEDS-CHANGES | BLOCKED
- Findings (severity, what the user/Claude would hit, mitigation)
- K-in citations

Be terse. ~250 words max.
2026-05-21T17:36:26.905003Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Auth(TokenRefreshFailed("Server returned error response: invalid_grant: Grant not found"))
2026-05-21T17:36:27.508176Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Deserialize(Error("data did not match any variant of untagged enum JsonRpcMessage", line: 0, column: 0))
codex
I’ll use the plan-devex-review lens and first gather the required local context: the plan packet, the cited ADR/catalog vocabulary, and the two MCP tool descriptions.
exec
/bin/zsh -lc "sed -n '1,220p' /Users/jamesgiroux/gstack/.agents/skills/gstack-plan-devex-review/SKILL.md" in /private/tmp/dailyos-v147-w1a
 exited 1 in 0ms:
sed: /Users/jamesgiroux/gstack/.agents/skills/gstack-plan-devex-review/SKILL.md: No such file or directory

codex
The duplicate skill path in the plugin list is stale, so I’m falling back to the canonical `r1` skill path from the session’s skill roots.
exec
/bin/zsh -lc "sed -n '1,220p' /Users/jamesgiroux/.agents/skills/gstack/plan-devex-review/SKILL.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
---
name: plan-devex-review
preamble-tier: 3
version: 2.0.0
description: |
  Interactive developer experience plan review. Explores developer personas,
  benchmarks against competitors, designs magical moments, and traces friction
  points before scoring. Three modes: DX EXPANSION (competitive advantage),
  DX POLISH (bulletproof every touchpoint), DX TRIAGE (critical gaps only).
  Use when asked to "DX review", "developer experience audit", "devex review",
  or "API design review".
  Proactively suggest when the user has a plan for developer-facing products
  (APIs, CLIs, SDKs, libraries, platforms, docs). (gstack)
  Voice triggers (speech-to-text aliases): "dx review", "developer experience review", "devex review", "devex audit", "API design review", "onboarding review".
benefits-from: [office-hours]
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
  - Bash
  - AskUserQuestion
  - WebSearch
triggers:
  - developer experience review
  - dx plan review
  - check developer onboarding
---
<!-- AUTO-GENERATED from SKILL.md.tmpl — do not edit directly -->
<!-- Regenerate: bun run gen:skill-docs -->

## Preamble (run first)

```bash
_UPD=$(~/.Codex/skills/gstack/bin/gstack-update-check 2>/dev/null || .Codex/skills/gstack/bin/gstack-update-check 2>/dev/null || true)
[ -n "$_UPD" ] && echo "$_UPD" || true
mkdir -p ~/.gstack/sessions
touch ~/.gstack/sessions/"$PPID"
_SESSIONS=$(find ~/.gstack/sessions -mmin -120 -type f 2>/dev/null | wc -l | tr -d ' ')
find ~/.gstack/sessions -mmin +120 -type f -exec rm {} + 2>/dev/null || true
_PROACTIVE=$(~/.Codex/skills/gstack/bin/gstack-config get proactive 2>/dev/null || echo "true")
_PROACTIVE_PROMPTED=$([ -f ~/.gstack/.proactive-prompted ] && echo "yes" || echo "no")
_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
echo "BRANCH: $_BRANCH"
_SKILL_PREFIX=$(~/.Codex/skills/gstack/bin/gstack-config get skill_prefix 2>/dev/null || echo "false")
echo "PROACTIVE: $_PROACTIVE"
echo "PROACTIVE_PROMPTED: $_PROACTIVE_PROMPTED"
echo "SKILL_PREFIX: $_SKILL_PREFIX"
source <(~/.Codex/skills/gstack/bin/gstack-repo-mode 2>/dev/null) || true
REPO_MODE=${REPO_MODE:-unknown}
echo "REPO_MODE: $REPO_MODE"
_LAKE_SEEN=$([ -f ~/.gstack/.completeness-intro-seen ] && echo "yes" || echo "no")
echo "LAKE_INTRO: $_LAKE_SEEN"
_TEL=$(~/.Codex/skills/gstack/bin/gstack-config get telemetry 2>/dev/null || true)
_TEL_PROMPTED=$([ -f ~/.gstack/.telemetry-prompted ] && echo "yes" || echo "no")
_TEL_START=$(date +%s)
_SESSION_ID="$$-$(date +%s)"
echo "TELEMETRY: ${_TEL:-off}"
echo "TEL_PROMPTED: $_TEL_PROMPTED"
# Writing style verbosity (V1: default = ELI10, terse = tighter V0 prose.
# Read on every skill run so terse mode takes effect without a restart.)
_EXPLAIN_LEVEL=$(~/.Codex/skills/gstack/bin/gstack-config get explain_level 2>/dev/null || echo "default")
if [ "$_EXPLAIN_LEVEL" != "default" ] && [ "$_EXPLAIN_LEVEL" != "terse" ]; then _EXPLAIN_LEVEL="default"; fi
echo "EXPLAIN_LEVEL: $_EXPLAIN_LEVEL"
# Question tuning (see /plan-tune). Observational only in V1.
_QUESTION_TUNING=$(~/.Codex/skills/gstack/bin/gstack-config get question_tuning 2>/dev/null || echo "false")
echo "QUESTION_TUNING: $_QUESTION_TUNING"
mkdir -p ~/.gstack/analytics
if [ "$_TEL" != "off" ]; then
echo '{"skill":"plan-devex-review","ts":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","repo":"'$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || echo "unknown")'"}'  >> ~/.gstack/analytics/skill-usage.jsonl 2>/dev/null || true
fi
# zsh-compatible: use find instead of glob to avoid NOMATCH error
for _PF in $(find ~/.gstack/analytics -maxdepth 1 -name '.pending-*' 2>/dev/null); do
  if [ -f "$_PF" ]; then
    if [ "$_TEL" != "off" ] && [ -x "~/.Codex/skills/gstack/bin/gstack-telemetry-log" ]; then
      ~/.Codex/skills/gstack/bin/gstack-telemetry-log --event-type skill_run --skill _pending_finalize --outcome unknown --session-id "$_SESSION_ID" 2>/dev/null || true
    fi
    rm -f "$_PF" 2>/dev/null || true
  fi
  break
done
# Learnings count
eval "$(~/.Codex/skills/gstack/bin/gstack-slug 2>/dev/null)" 2>/dev/null || true
_LEARN_FILE="${GSTACK_HOME:-$HOME/.gstack}/projects/${SLUG:-unknown}/learnings.jsonl"
if [ -f "$_LEARN_FILE" ]; then
  _LEARN_COUNT=$(wc -l < "$_LEARN_FILE" 2>/dev/null | tr -d ' ')
  echo "LEARNINGS: $_LEARN_COUNT entries loaded"
  if [ "$_LEARN_COUNT" -gt 5 ] 2>/dev/null; then
    ~/.Codex/skills/gstack/bin/gstack-learnings-search --limit 3 2>/dev/null || true
  fi
else
  echo "LEARNINGS: 0"
fi
# Session timeline: record skill start (local-only, never sent anywhere)
~/.Codex/skills/gstack/bin/gstack-timeline-log '{"skill":"plan-devex-review","event":"started","branch":"'"$_BRANCH"'","session":"'"$_SESSION_ID"'"}' 2>/dev/null &
# Check if AGENTS.md has routing rules
_HAS_ROUTING="no"
if [ -f AGENTS.md ] && grep -q "## Skill routing" AGENTS.md 2>/dev/null; then
  _HAS_ROUTING="yes"
fi
_ROUTING_DECLINED=$(~/.Codex/skills/gstack/bin/gstack-config get routing_declined 2>/dev/null || echo "false")
echo "HAS_ROUTING: $_HAS_ROUTING"
echo "ROUTING_DECLINED: $_ROUTING_DECLINED"
# Vendoring deprecation: detect if CWD has a vendored gstack copy
_VENDORED="no"
if [ -d ".Codex/skills/gstack" ] && [ ! -L ".Codex/skills/gstack" ]; then
  if [ -f ".Codex/skills/gstack/VERSION" ] || [ -d ".Codex/skills/gstack/.git" ]; then
    _VENDORED="yes"
  fi
fi
echo "VENDORED_GSTACK: $_VENDORED"
echo "MODEL_OVERLAY: Codex"
# Checkpoint mode (explicit = no auto-commit, continuous = WIP commits as you go)
_CHECKPOINT_MODE=$(~/.Codex/skills/gstack/bin/gstack-config get checkpoint_mode 2>/dev/null || echo "explicit")
_CHECKPOINT_PUSH=$(~/.Codex/skills/gstack/bin/gstack-config get checkpoint_push 2>/dev/null || echo "false")
echo "CHECKPOINT_MODE: $_CHECKPOINT_MODE"
echo "CHECKPOINT_PUSH: $_CHECKPOINT_PUSH"
# Detect spawned session (OpenClaw or other orchestrator)
[ -n "$OPENCLAW_SESSION" ] && echo "SPAWNED_SESSION: true" || true
```

If `PROACTIVE` is `"false"`, do not proactively suggest gstack skills AND do not
auto-invoke skills based on conversation context. Only run skills the user explicitly
types (e.g., /qa, /ship). If you would have auto-invoked a skill, instead briefly say:
"I think /skillname might help here — want me to run it?" and wait for confirmation.
The user opted out of proactive behavior.

If `SKILL_PREFIX` is `"true"`, the user has namespaced skill names. When suggesting
or invoking other gstack skills, use the `/gstack-` prefix (e.g., `/gstack-qa` instead
of `/qa`, `/gstack-ship` instead of `/ship`). Disk paths are unaffected — always use
`~/.Codex/skills/gstack/[skill-name]/SKILL.md` for reading skill files.

If output shows `UPGRADE_AVAILABLE <old> <new>`: read `~/.Codex/skills/gstack/gstack-upgrade/SKILL.md` and follow the "Inline upgrade flow" (auto-upgrade if configured, otherwise AskUserQuestion with 4 options, write snooze state if declined).

If output shows `JUST_UPGRADED <from> <to>` AND `SPAWNED_SESSION` is NOT set: tell
the user "Running gstack v{to} (just updated!)" and then check for new features to
surface. For each per-feature marker below, if the marker file is missing AND the
feature is plausibly useful for this user, use AskUserQuestion to let them try it.
Fire once per feature per user, NOT once per upgrade.

**In spawned sessions (`SPAWNED_SESSION` = "true"): SKIP feature discovery entirely.**
Just print "Running gstack v{to}" and continue. Orchestrators do not want interactive
prompts from sub-sessions.

**Feature discovery markers and prompts** (one at a time, max one per session):

1. `~/.Codex/skills/gstack/.feature-prompted-continuous-checkpoint` →
   Prompt: "Continuous checkpoint auto-commits your work as you go with `WIP:` prefix
   so you never lose progress to a crash. Local-only by default — doesn't push
   anywhere unless you turn that on. Want to try it?"
   Options: A) Enable continuous mode, B) Show me first (print the section from
   the preamble Continuous Checkpoint Mode), C) Skip.
   If A: run `~/.Codex/skills/gstack/bin/gstack-config set checkpoint_mode continuous`.
   Always: `touch ~/.Codex/skills/gstack/.feature-prompted-continuous-checkpoint`

2. `~/.Codex/skills/gstack/.feature-prompted-model-overlay` →
   Inform only (no prompt): "Model overlays are active. `MODEL_OVERLAY: {model}`
   shown in the preamble output tells you which behavioral patch is applied.
   Override with `--model` when regenerating skills (e.g., `bun run gen:skill-docs
   --model gpt-5.4`). Default is Codex."
   Always: `touch ~/.Codex/skills/gstack/.feature-prompted-model-overlay`

After handling JUST_UPGRADED (prompts done or skipped), continue with the skill
workflow.

If `WRITING_STYLE_PENDING` is `yes`: You're on the first skill run after upgrading
to gstack v1. Ask the user once about the new default writing style. Use AskUserQuestion:

> v1 prompts = simpler. Technical terms get a one-sentence gloss on first use,
> questions are framed in outcome terms, sentences are shorter.
>
> Keep the new default, or prefer the older tighter prose?

Options:
- A) Keep the new default (recommended — good writing helps everyone)
- B) Restore V0 prose — set `explain_level: terse`

If A: leave `explain_level` unset (defaults to `default`).
If B: run `~/.Codex/skills/gstack/bin/gstack-config set explain_level terse`.

Always run (regardless of choice):
```bash
rm -f ~/.gstack/.writing-style-prompt-pending
touch ~/.gstack/.writing-style-prompted
```

This only happens once. If `WRITING_STYLE_PENDING` is `no`, skip this entirely.

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

If A: run `~/.Codex/skills/gstack/bin/gstack-config set telemetry community`

If B: ask a follow-up AskUserQuestion:

> How about anonymous mode? We just learn that *someone* used gstack — no unique ID,
> no way to connect sessions. Just a counter that helps us know if anyone's out there.

Options:

exec
/bin/zsh -lc 'rg -n "DX TRIAGE|Findings|Output|Workflow|Severity|review" /Users/jamesgiroux/.agents/skills/gstack/plan-devex-review/SKILL.md' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
2:name: plan-devex-review
6:  Interactive developer experience plan review. Explores developer personas,
9:  DX POLISH (bulletproof every touchpoint), DX TRIAGE (critical gaps only).
10:  Use when asked to "DX review", "developer experience audit", "devex review",
11:  or "API design review".
14:  Voice triggers (speech-to-text aliases): "dx review", "developer experience review", "devex review", "devex audit", "API design review", "onboarding review".
25:  - developer experience review
26:  - dx plan review
70:echo '{"skill":"plan-devex-review","ts":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","repo":"'$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || echo "unknown")'"}'  >> ~/.gstack/analytics/skill-usage.jsonl 2>/dev/null || true
95:~/.Codex/skills/gstack/bin/gstack-timeline-log '{"skill":"plan-devex-review","event":"started","branch":"'"$_BRANCH"'","session":"'"$_SESSION_ID"'"}' 2>/dev/null &
283:- Code review, check my diff → invoke review
287:- Visual audit, design polish → invoke design-review
288:- Architecture review → invoke plan-eng-review
343:safety, and /ship review gates. If a nudge below conflicts with skill instructions,
373:**Tone:** direct, concrete, sharp, encouraging, serious about craft, occasionally funny, never corporate, never academic, never PR, never hype. Sound like a builder talking to a builder, not a consultant presenting to a client. Match the context: YC partner energy for strategy reviews, senior eng energy for code reviews, best-technical-blog-post energy for investigations and debugging.
379:**Connect to user outcomes.** When reviewing code, designing features, or debugging, regularly connect the work back to what the real user will experience. "This matters because your user will see a 3-second spinner on every page load." "The edge case you're skipping is the one that loses the customer's data." Make the user's user real.
416:  [ -f "$_PROJ/${_BRANCH}-reviews.jsonl" ] && echo "REVIEWS: $(wc -l < "$_PROJ/${_BRANCH}-reviews.jsonl" | tr -d ' ') entries"
440:(e.g., review,ship,review), suggest: "Based on your recent pattern, you probably
462:These rules apply to every AskUserQuestion, every response you write to the user, and every review finding. They compose with the AskUserQuestion Format section above: Format = *how* a question is structured; Writing Style = *the prose quality of the content inside it*.
466:   - **Pain reduction** (default for diagnostic / HOLD SCOPE / rigor review): "If someone double-clicks the button, is it OK for the action to run twice?" (instead of "Is this endpoint idempotent?")
660:~/.Codex/skills/gstack/bin/gstack-question-log '{"skill":"plan-devex-review","question_id":"<id>","question_summary":"<short>","category":"<approval|clarification|routing|cherry-pick|feedback-loop>","door_type":"<one-way|two-way>","options_count":N,"user_choice":"<key>","recommended":"<key>","session_id":"'"$_SESSION_ID"'"}' 2>/dev/null || true
784:`$B` (browse), `$D` (design), `codex exec`/`codex review`, writes to `~/.gstack/`,
799:section, run `~/.Codex/skills/gstack/bin/gstack-review-read` and append a report.
803:If a richer review report already exists, skip — review skills wrote it.
846:# /plan-devex-review: Developer Experience Plan Review
860:is to review and improve the plan's DX decisions with maximum rigor.
932:During each review pass, load the relevant section from:
933:\`~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md\`
994:> statement, premise challenge, and explored alternatives — it gives this review much
999:- A) Run /office-hours now (we'll pick up the review right after)
1000:- B) Skip — proceed with standard review
1002:If they skip: "No worries — standard review. If you ever want sharper input, try
1008:the review right where we left off."
1040:If a design doc is now found, read it and continue the review.
1041:If none was produced (user may have cancelled), proceed with standard review.
1055:"This plan doesn't appear to have developer-facing surfaces. /plan-devex-review
1056:reviews plans for APIs, CLIs, SDKs, libraries, platforms, and docs. Consider
1057:/plan-eng-review or /plan-design-review instead." Exit gracefully.
1116:**STOP.** Do NOT proceed until user responds. This persona shapes the entire review.
1124:Use the persona from 0A. Reference real files and content from the pre-review audit.
1190:Load the "## Pass 1" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`
1228:How deep should this DX review go?
1234:> "How deep should this DX review go?
1242:>    (recommended for most reviews)
1244:> C) **DX TRIAGE** -- Focus only on the critical DX gaps that would block adoption.
1252:* Bug fix or urgent ship → default DX TRIAGE
1290:**DX TRIAGE mode:** Only trace Install and Hello World stages. Skip the rest.
1328:Ground this in the ACTUAL docs and code from the pre-review audit. Not hypothetical.
1373:- **DX TRIAGE:** Only flag gaps that would block adoption (score below 5). Skip gaps
1378:**Anti-skip rule:** Never condense, abbreviate, or skip any review pass (1-8) regardless of plan type (strategy, spec, code, infra). Every pass in this skill exists for a reason. "This is a strategy doc so DX passes don't apply" is always wrong — DX gaps are where adoption breaks down. If a pass genuinely has zero findings, say "No issues found" and move on — but you must evaluate it.
1410:If learnings are found, incorporate them into your analysis. When a review finding
1420:Before starting review passes, check for prior DX reviews on this project:
1424:~/.Codex/skills/gstack/bin/gstack-review-read 2>/dev/null | grep plan-devex-review || echo "NO_PRIOR_DX_REVIEWS"
1427:If prior reviews exist, display the trend:
1429:DX TREND (prior reviews):
1443:Load reference: Read the "## Pass 1" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1472:Load reference: Read the "## Pass 2" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1496:Load reference: Read the "## Pass 3" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1521:Load reference: Read the "## Pass 4" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1537:Load reference: Read the "## Pass 5" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1555:Load reference: Read the "## Pass 6" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1573:Load reference: Read the "## Pass 7" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1589:Load reference: Read the "## Pass 8" section from `~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1595:- **Friction audits**: Periodic reviews planned?
1596:- **Boomerang readiness**: Will /devex-review be able to measure reality vs. plan?
1607:`~/.Codex/skills/gstack/plan-devex-review/dx-hall-of-fame.md`.
1615:After all review sections are complete, offer an independent second opinion from a
1617:thorough review.
1627:> "All review sections are complete. Want an outside voice? A different AI system can
1629:> risks, and blind spots that are hard to catch from inside the review. Takes about 2
1634:> thorough review. Completeness: A=9/10, B=7/10.
1642:**If A:** Construct the plan review prompt. Read the plan file being reviewed (the file
1643:the user pointed this review at, or the branch diff scope). If a CEO plan document
1650:"IMPORTANT: Do NOT read or execute any files under ~/.Codex/, ~/.agents/, .Codex/skills/, or agents/. These are Codex skill definitions meant for a different AI system. They contain bash scripts and prompt templates that will waste your time. Ignore them completely. Do NOT modify agents/openai.yaml. Stay focused on the repository code only.\n\nYou are a brutally honest technical reviewer examining a development plan that has
1651:already been through a multi-section review. Your job is NOT to repeat that review.
1653:survived the review scrutiny, overcomplexity (is there a fundamentally simpler
1654:approach the review was too deep in the weeds to see?), feasibility risks the review
1665:TMPERR_PV=$(mktemp /tmp/codex-planreview-XXXXXXXX)
1678:CODEX SAYS (plan review — outside voice):
1695:Subagent prompt: same plan review prompt as above.
1704:disagrees with the review findings from earlier sections. Flag these as:
1720:> "Cross-model disagreement on [topic]. The review found [X] but the outside voice
1735:If no tension points exist, note: "No cross-model tension — both reviewers agree."
1739:~/.Codex/skills/gstack/bin/gstack-review-log '{"skill":"codex-plan-review","timestamp":"'"$(date -u +%Y-%m-%dT%H:%M:%SZ)"'","status":"STATUS","source":"SOURCE","commit":"'"$(git rev-parse --short HEAD)"'"}'
1756:DX reviews:
1772:## Required Outputs
1781:The benchmark table from Step 0C, updated with the product's post-review scores.
1799:After all review passes are complete, present each potential TODO as its own individual
1878:After producing the DX Scorecard above, persist the review result.
1880:**PLAN MODE EXCEPTION — ALWAYS RUN:** This command writes review metadata to
1884:~/.Codex/skills/gstack/bin/gstack-review-log '{"skill":"plan-devex-review","timestamp":"TIMESTAMP","status":"STATUS","initial_score":N,"overall_score":N,"product_type":"TYPE","tthw_current":"TTHW_CURRENT","tthw_target":"TTHW_TARGET","mode":"MODE","persona":"PERSONA","competitive_tier":"TIER","pass_scores":{"getting_started":N,"api_design":N,"errors":N,"docs":N,"upgrade":N,"dev_env":N,"community":N,"measurement":N},"unresolved":N,"commit":"COMMIT"}'
1893:After completing the review, read the review log and config to display the dashboard.
1896:~/.Codex/skills/gstack/bin/gstack-review-read
1899:Parse the output. Find the most recent entry for each skill (plan-ceo-review, plan-eng-review, review, plan-design-review, design-review-lite, adversarial-review, codex-review, codex-plan-review). Ignore entries with timestamps older than 7 days. For the Eng Review row, show whichever is more recent between `review` (diff-scoped pre-landing review) and `plan-eng-review` (plan-stage architecture review). Append "(DIFF)" or "(PLAN)" to the status to distinguish. For the Adversarial row, show whichever is more recent between `adversarial-review` (new auto-scaled) and `codex-review` (legacy). For Design Review, show whichever is more recent between `plan-design-review` (full visual audit) and `design-review-lite` (code-level check). Append "(FULL)" or "(LITE)" to the status to distinguish. For the Outside Voice row, show the most recent `codex-plan-review` entry — this captures outside voices from both /plan-ceo-review and /plan-eng-review.
1901:**Source attribution:** If the most recent entry for a skill has a \`"via"\` field, append it to the status label in parentheses. Examples: `plan-eng-review` with `via:"autoplan"` shows as "CLEAR (PLAN via /autoplan)". `review` with `via:"ship"` shows as "CLEAR (DIFF via /ship)". Entries without a `via` field show as "CLEAR (PLAN)" or "CLEAR (DIFF)" as before.
1924:- **Eng Review (required by default):** The only review that gates shipping. Covers architecture, code quality, tests, performance. Can be disabled globally with \`gstack-config set skip_eng_review true\` (the "don't bother me" setting).
1927:- **Adversarial Review (automatic):** Always-on for every review. Every diff gets both Codex adversarial subagent and Codex adversarial challenge. Large diffs (200+ lines) additionally get Codex structured review with P1 gate. No configuration needed.
1928:- **Outside Voice (optional):** Independent plan review from a different AI model. Offered after all review sections complete in /plan-ceo-review and /plan-eng-review. Falls back to Codex subagent if Codex is unavailable. Never gates shipping.
1931:- **CLEARED**: Eng Review has >= 1 entry within 7 days from either \`review\` or \`plan-eng-review\` with status "clean" (or \`skip_eng_review\` is \`true\`)
1933:- CEO, Design, and Codex reviews are shown for context but never block shipping
1934:- If \`skip_eng_review\` config is \`true\`, Eng Review shows "SKIPPED (global)" and verdict is CLEARED
1936:**Staleness detection:** After displaying the dashboard, check if any existing reviews may be stale:
1938:- For each review entry that has a \`commit\` field: compare it against the current HEAD. If different, count elapsed commits: \`git rev-list --count STORED_COMMIT..HEAD\`. Display: "Note: {skill} review from {date} may be stale — {N} commits since review"
1939:- For entries without a \`commit\` field (legacy entries): display "Note: {skill} review from {date} has no commit tracking — consider re-running for accurate staleness detection"
1940:- If all reviews match the current HEAD, do not display any staleness notes
1945:**plan file** itself so review status is visible to anyone reading the plan.
1951:2. If not found, skip this section silently — not every review runs in plan mode.
1955:Read the review log output you already have from the Review Readiness Dashboard step above.
1958:- **plan-ceo-review**: \`status\`, \`unresolved\`, \`critical_gaps\`, \`mode\`, \`scope_proposed\`, \`scope_accepted\`, \`scope_deferred\`, \`commit\`
1959:  → Findings: "{scope_proposed} proposals, {scope_accepted} accepted, {scope_deferred} deferred"
1961:- **plan-eng-review**: \`status\`, \`unresolved\`, \`critical_gaps\`, \`issues_found\`, \`mode\`, \`commit\`
1962:  → Findings: "{issues_found} issues, {critical_gaps} critical gaps"
1963:- **plan-design-review**: \`status\`, \`initial_score\`, \`overall_score\`, \`unresolved\`, \`decisions_made\`, \`commit\`
1964:  → Findings: "score: {initial_score}/10 → {overall_score}/10, {decisions_made} decisions"
1965:- **plan-devex-review**: \`status\`, \`initial_score\`, \`overall_score\`, \`product_type\`, \`tthw_current\`, \`tthw_target\`, \`mode\`, \`persona\`, \`competitive_tier\`, \`unresolved\`, \`commit\`
1966:  → Findings: "score: {initial_score}/10 → {overall_score}/10, TTHW: {tthw_current} → {tthw_target}"
1967:- **devex-review**: \`status\`, \`overall_score\`, \`product_type\`, \`tthw_measured\`, \`dimensions_tested\`, \`dimensions_inferred\`, \`boomerang\`, \`commit\`
1968:  → Findings: "score: {overall_score}/10, TTHW: {tthw_measured}, {dimensions_tested} tested/{dimensions_inferred} inferred"
1969:- **codex-review**: \`status\`, \`gate\`, \`findings\`, \`findings_fixed\`
1970:  → Findings: "{findings} findings, {findings_fixed}/{findings} fixed"
1972:All fields needed for the Findings column are now present in the JSONL entries.
1973:For the review you just completed, you may use richer details from your own Completion
1974:Summary. For prior reviews, use the JSONL fields directly — they contain all required data.
1981:| Review | Trigger | Why | Runs | Status | Findings |
1983:| CEO Review | \`/plan-ceo-review\` | Scope & strategy | {runs} | {status} | {findings} |
1984:| Codex Review | \`/codex review\` | Independent 2nd opinion | {runs} | {status} | {findings} |
1985:| Eng Review | \`/plan-eng-review\` | Architecture & tests (required) | {runs} | {status} | {findings} |
1986:| Design Review | \`/plan-design-review\` | UI/UX gaps | {runs} | {status} | {findings} |
1987:| DX Review | \`/plan-devex-review\` | Developer experience gaps | {runs} | {status} | {findings} |
1992:- **CODEX:** (only if codex-review ran) — one-line summary of codex fixes
1993:- **CROSS-MODEL:** (only if both Codex and Codex reviews exist) — overlap analysis
1994:- **UNRESOLVED:** total unresolved decisions across all reviews
1995:- **VERDICT:** list reviews that are CLEAR (e.g., "CEO + ENG CLEARED — ready to implement").
1996:  If Eng Review is not CLEAR and not skipped globally, append "eng review required".
2001:file you are allowed to edit in plan mode. The plan file review report is part of the
2020:~/.Codex/skills/gstack/bin/gstack-learnings-log '{"skill":"plan-devex-review","type":"TYPE","key":"SHORT_KEY","insight":"DESCRIPTION","confidence":N,"source":"SOURCE","files":["path/to/relevant/file"]}'
2041:After displaying the Review Readiness Dashboard, recommend next reviews:
2043:**Recommend /plan-eng-review if eng review is not skipped globally** — DX issues often
2044:have architectural implications. If this DX review found API design problems, error
2045:handling gaps, or CLI ergonomics issues, eng review should validate the fixes.
2047:**Suggest /plan-design-review if user-facing UI exists** — DX review focuses on
2048:developer-facing surfaces; design review covers end-user-facing UI.
2050:**Recommend /devex-review after implementation** — the boomerang. Plan said TTHW would
2051:be [target from 0C]. Did reality match? Run /devex-review on the live product to find
2056:- **A)** Run /plan-eng-review next (required gate)
2057:- **B)** Run /plan-design-review (only if UI scope detected)
2058:- **C)** Ready to implement, run /devex-review after shipping
2063:             | DX EXPANSION     | DX POLISH          | DX TRIAGE

codex
I’ve got the review frame. Now I’m reading the actual packet and grepping the required K-in sources so the findings are tied to repository language instead of assumptions.
exec
/bin/zsh -lc "sed -n '1,260p' .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet

**Wave:** v1.4.7 W2-A
**Lane spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175)
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W2-A — DOS-175"
**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
**Authoring discipline:** narrow-scoped per K-in lessons; scope confined to wiring already-authored producers into the W1-A handler seam.

## Cycle-1 scope

W2-A ships two read-side MCP tool handlers wrapping abilities that exist today in `abilities-runtime`:

1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)

These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.

**Threat topology:** `local-to-local single-user`. Claude Desktop on the same machine, same user, loopback stdio MCP transport. Per `engineering-ladder.md` §"Threat-topology framing" + memory `feedback_local_to_local_security_overreach_primary_concern`: reviewers do NOT enforce multi-actor gates, principal differentiation, cross-actor poisoning, or scope-gated redaction of data the user already has filesystem access to. The only invariants that survive the topology test are: compile correctness, crate-boundary rules, sensitivity rendering already done in the producer (ADR-0108), and indirect-prompt-injection containment from third-party document content (ADR-0093). Reviewers who flag multi-actor gates on this surface waste a cycle; please scope your threat model to "Claude Desktop run by James on James's machine talking to DailyOS running as James."

## 1. What this unblocks

End-to-end demo path: user pairs Claude Desktop against `dailyos-mcp-v2`, asks "give me an account overview for Acme" or "what's on my plate today", Claude routes to the appropriate DailyOS tool, the W2-A handler invokes the underlying ability against the same `abilities-runtime` registry the Tauri UI uses, and the substrate response is returned to Claude Desktop. This is the v1.4.7 W2-A bar — first observable MCP product surface against real DailyOS substrate.

The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.

## 2. Substrate path (K-in confirmation)

`docs/solutions/` + `.docs/decisions/` grep results — substrate already present:

- **`AbilityRegistry`** — single global static. `abilities-runtime/src/abilities/registry.rs:838` exposes `pub fn global_checked() -> Result<&'static Self, ...>` backed by `OnceLock`. There is no parallel legacy registry.
- **`crate::abilities` shim** — `src-tauri/src/abilities/mod.rs:3` is `pub use abilities_runtime::abilities::*;`. Tauri commands and MCP bridges both consume the same crate-level re-export.
- **`invoke_registry_json`** — the shared async dispatch core. Called by Tauri (`bridges/tauri.rs:217`), legacy MCP (`bridges/mcp.rs:411-422`), and now W2-A.
- **`McpAbilityBridge::invoke_ability`** (`src-tauri/src/bridges/mcp.rs:385-430`) — the canonical pattern for "actor + ability_name + input_json → AbilityResponseJson" over the same registry. W2-A handlers reuse this pattern verbatim (no second invocation engine).
- **ADR-0102 §C.bis** — abilities-as-runtime contract; covers `mcp_exposure`, `allowed_actors`, attribute changes via re-approval.
- **ADR-0108** — claim-sensitivity rendering; producer abilities render sensitivity before returning. Handler is a thin JSON wrapper.
- **ADR-0125** — claim-sensitivity rendering depth; same as above.
- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
- **W1-A handler seam** — `gateway.rs:155` (`handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>`), `gateway.rs:177` (`register`), `gateway.rs:271` (`dispatch`). Scaffolding directory `services/mcp_v2/handlers/` already exists with 1-line placeholder files including `tool_briefing.rs`.
- **`McpToolHandler` trait** — `services/mcp_v2/contracts.rs:341`: sync `invoke(&self, &McpActor, params: Value) -> Result<Value, ToolError>`.

**No substrate reinvention.** W2-A is purely the wiring layer between W1-A's `McpToolHandler` registration seam and the existing `invoke_registry_json` dispatch.

## 3. Architecture

### 3.1 Handler shape

One handler struct per tool, in its own file under `services/mcp_v2/handlers/`. Each handler holds:

```rust
pub struct AccountStatusHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
}
```

The runtime handle is captured at construction time (during gateway registration, when we are inside the tokio runtime that rmcp drives). The sync `invoke()` method calls `self.runtime.block_on(async { ... })` to dispatch the underlying async ability. This resolves the sync/async bridge question from the handoff: **Option 2 from the handoff (carry-handle-in-struct)**, not Option 1 (`Handle::current()` lookup at call time, which risks panic if called outside a runtime context). The legacy MCP bridge implicitly assumes a runtime context because it's only called from rmcp; the W1-A `dispatch` trait is sync, so we must construct an explicit bridge.

Rejected alternatives:
- **Option 1** (`Handle::current()` lookup at invoke time): panics in non-runtime tests; capturing at construction is strictly safer.
- **Option 3** (change trait to async): forces W1-A test stubs + transport layer changes for a problem solvable in the handler.
- **Option 4** (return `Pin<Box<dyn Future>>` from trait): verbose, requires W1-A trait change.

### 3.2 Ability dispatch within the handler

The handler's `invoke` body mirrors `bridges/mcp.rs:391-422` (the legacy MCP bridge's `invoke_ability`):

```rust
fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
    self.runtime.block_on(async {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external)
            .with_actor(MCP_V2_ACTOR_LABEL);
        let invocation = InvocationContext {
            actor: BridgeActor::from_mcp_actor(actor),
            mode: ExecutionMode::Live,
            surface: BridgeSurface::McpTool,
            claim_dismissal_surface: ClaimDismissalSurface::McpTool,
            dry_run: false,
            confirmation: None,
            confirmation_store: None,
        };
        let response = invoke_registry_json(
            self.registry,
            &services,
            None,  // provider
            None,  // tracer (W2-A: no per-invocation tracing yet; W3 adds it)
            invocation,
            "account_overview",  // or "get_daily_briefing"
            params,
        )
        .await
        .map_err(map_invoke_error_to_tool_error)?;
        Ok(response.data)
    })
}
```

The actor label can reuse the existing `bridges/mcp.rs::MCP_ACTOR_LABEL = "mcp"` — local-to-local single-user, no audit-differentiation requirement that justifies a new constant. The `BridgeActor::from_mcp_actor` projection converts `services::mcp_v2::McpActor::Client { client_id, conversation_handle }` to `abilities_runtime::Actor::McpClient { client_id, conversation_handle }` — same shape, trivial conversion. Most likely lands as a small adapter in `services/mcp_v2/actor_policy.rs::project_actor` or sibling.

### 3.3 Registration helper

New module `services/mcp_v2/handlers/registration.rs`:

```rust
pub fn register_v147_handlers(
    gateway: &mut Gateway,
    catalog: &YamlTaxonomyCatalog,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
) -> Result<(), RegistrationError> {
    gateway.register(Arc::new(AccountStatusHandler::new(
        catalog.description_for(&account_status_name())?.clone(),
        registry,
        runtime.clone(),
    )));
    gateway.register(Arc::new(DailyBriefingHandler::new(
        catalog.description_for(&daily_briefing_name())?.clone(),
        registry,
        runtime.clone(),
    )));
    Ok(())
}
```

Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).

### 3.4 Ability attribute amendments

**`account_overview` (`abilities-runtime/src/abilities/account_overview.rs:34`):**

Current:
```rust
#[ability(
    name = "account_overview",
    category = Read,
    allowed_actors = [User, SurfaceClient],
    mcp_exposure = Invocable,
    ...
)]
```

Amendment:
```rust
    allowed_actors = [User, SurfaceClient, McpClient],
```

**Justification.** The ability already declares `mcp_exposure = Invocable`, so adding `McpClient` to the allowlist is a consistency fix, not a new exposure decision. Local-to-local single-user topology: no CSO escalation cycle warranted. Just amend.

**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**

Current:
```rust
#[ability(
    name = "get_daily_briefing",
    category = Read,
    allowed_actors = [User, SurfaceClient],
    mcp_exposure = None,
    ...
)]
```

Amendment:
```rust
    allowed_actors = [User, SurfaceClient, McpClient],
    mcp_exposure = Invocable,
```

**Justification.** Explicitly authorized by DOS-624 §1 and §3 (cycle-2 normative). The DOS-624 amendment is unanimous APPROVE on this branch.

### 3.5 Output translation

Per ADR-0125 + ADR-0108, the producer ability renders claim sensitivity before returning. The handler's translation is `serde_json::to_value(response.data)` — no re-rendering, no redaction layer in the handler. The `AbilityResponseJson` already wraps the rendered output. This matches the legacy `McpAbilityBridge::invoke_ability` pattern (line 411-430): the bridge returns `AbilityResponseJson` directly; the consumer (rmcp layer) serializes for wire. **Open question Q2** (§7): the V1 bridge wraps the response in a session-scoped provenance cache (`insert_provenance` at `bridges/mcp.rs:431-436`). W2-A does not include a per-session provenance cache yet. Reviewers: is this a W2-A AC or W2-B follow-up? Cycle-1 stance: defer to W2-B (the user-facing demo doesn't depend on detail-pane provenance lookup).

## 4. Out-of-scope (explicit deferrals)

These items are NOT in W2-A cycle 1 and have specific deferral reasoning:

1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
2. **`dailyos.read.portfolio_attention`** — producer ability does not exist; `get_entity_intelligence` is per-entity not per-portfolio. Defer to W3.
3. **`dailyos.search.workspace_memory`** — no producer ability; depends on v1.4.5 workspace substrate landing on this branch. Defer.
4. **`dailyos.read.workspace_source_provenance`** — same as above.
5. **`dailyos.write.place_document`** + **`dailyos.submit.{note,action,action_status}`** — Write-side abilities; require additional CSO amendment surface (W3-C parallel-amendment open question per DOS-624 §8 Q3). Defer.

## 5. Acceptance criteria

**AC-1.** `account_overview` ability attribute amended: `allowed_actors` includes `McpClient`. Compile + clippy green.

**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.

**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.

**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.

**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.

**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.

**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).

**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.

**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.

**AC-10.** No regression: `cargo check --features mcp --lib`, `cargo clippy --features mcp --lib --bin dailyos-mcp-v2 -- -D warnings`, `cargo test --features mcp` all green. No L1 self-validation gaps before opening the PR.

## 6. Test plan

- **Unit** (per handler): mock registry dispatch (provide test-only fake via cfg(test) hook or thread the registry as a generic param) and assert wire shape conversion. ~30 LOC each.
- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).

**Out of scope for cycle-1 tests:** concurrent burst latency (W2-B), provenance cache (W2-B), meeting_briefing existence oracle (deferred per §4).

## 7. Open questions for L0 reviewers

**Q1 (architecture).** Should the `BridgeActor::from_mcp_actor` projection live in `services/mcp_v2/actor_policy.rs` (next to `project_actor`) or in a new `bridges/mcp_v2.rs` mirroring the legacy bridge layout? The actor projection helper is the only cross-crate signature decision in W2-A; everything else follows W1-A precedent.

**Q2 (provenance).** Defer per-session provenance cache to W2-B? V1 bridge maintains a session-scoped `(InvocationId → RenderedProvenance)` map for detail-pane lookups. The end-to-end demo (Claude Desktop rendering substrate as text) doesn't depend on this. Cycle-1 stance: defer.

**Q3 (meeting_briefing deferral).** Is the §4.1 deferral of `meeting_briefing` to W2-A Phase-2 acceptable given DOS-624 §3.4's explicit AC inclusion? Cycle-1 stance: producer mapping work is non-trivial and decoupled from the demo outcome; phase-2 sub-ticket of DOS-175 captures the residual without losing the audit trail. The DOS-624 §3.4 timing-floor / p95-burst test is itself a multi-actor-style defense that the local-to-local topology arguably does not require; reviewers may consider whether the original DOS-624 §3.4 requirement should itself be revisited or whether it's load-bearing for a future remote-to-local surface (W3-C WP MCP adapter).

## 8. Reviewer dispatch (cycle 1)

- `/codex challenge` — adversarial: try to break the handler design, the sync/async bridge, the registration ordering.
- `architect-reviewer` — domain: substrate work, registry coupling, ADR-0102/0108/0125 conformance.
- `/plan-devex-review` — MCP/DX surface: tool exposure, Claude Desktop pairing flow, error shapes.
- `/codex consult` — fifth panelist: independent sanity check.

K-in mandatory for each: grep `docs/solutions/` and `.docs/decisions/` for substrate cited above; cite hits in verdict. Unanimous required.

## 9. Definition of done

The session reaches DoD when:
1. Unanimous L0 APPROVE from all four reviewers
2. AC-1 through AC-10 validated with real data
3. L2 (codex review + code-reviewer + architect-reviewer) unanimous APPROVE on the W2-A diff, bounded by AC
4. L4 manual smoke (AC-9) executed and captured: screenshot of Claude Desktop returning substrate response for "give me an account overview for Acme"

If reviewers convergently push back on the §4 deferrals (especially meeting_briefing), reset scope rather than stack patches per memory `feedback_review_loop_diminishing_returns_means_scope_is_wrong`.

exec
/bin/zsh -lc 'rg -n "ADR-0083|ADR 0083|0083|product vocabulary|Product Vocabulary|account status|daily briefing|Daily Briefing|Claude Desktop|MCP" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/README.md:37:- `integration-issues/` — cross-component, MCP, abilities, surface wiring
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:346:Codex flagged an inter-ADR conflict with [ADR-0111](0111-surface-independent-ability-invocation.md), which states the MCP bridge refuses confirmation fields. The `AgentWithAuthority` shape in this ADR creates a policy-bypass surface.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:46:- MCP tool is removed from hand-written registration; the registry-derived tool takes over.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:143:- **Stage 4 (cutover) rollback.** Revert the Tauri command wrapper and MCP tool registration to hand-written versions (retained as fallback). Old path runs again. User impact: if the new path was producing materially different output, users see the old output's characteristics return.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:33:- An MCP agent discovers `repair_graph_links` in the registry and invokes it with bad inputs, corrupting the graph.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:204:    Agent,      // MCP tool invocation
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:243:Agents (MCP) cannot invoke `global` maintenance under any circumstance, regardless of actor policy overrides. This is a hard floor enforced at the registry.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:379:A surface that has access only to `invoke_read` and `invoke_transform` cannot reach maintenance abilities even by name. Tauri commands for read/transform invocation do not expose a maintenance path. MCP tools for non-maintenance abilities route through the non-maintenance invokers. This is phased:
.docs/decisions/0027-mcp-dual-mode.md:1:# ADR-0027: MCP integration: dual-mode server + client
.docs/decisions/0027-mcp-dual-mode.md:12:The app is both an MCP server (exposes workspace tools/resources to Claude Desktop, agents, automation) and an MCP client (consumes external MCP services like Clay).
.docs/decisions/0027-mcp-dual-mode.md:18:**Interface parity (extends ADR-0025):** App, CLI, and MCP share the same registries (`~/.dailyos/`) and workspace.
.docs/decisions/0027-mcp-dual-mode.md:23:- Users in Claude Desktop can query DailyOS data without opening the app
.docs/decisions/0027-mcp-dual-mode.md:24:- MCP server binds to localhost only — no remote access
.docs/decisions/0027-mcp-dual-mode.md:26:- Phase 4 feature — but IPC commands (ADR-0025) should be designed to be MCP-exposable from the start
.docs/decisions/0053-dashboard-ux-redesign.md:8:A full UX review of the daily briefing/dashboard page identified several information architecture problems:
.docs/decisions/0043-meeting-intelligence-is-core.md:39:| CRM integration (Clay MCP) | Product-specific data source |
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:103:- `new_live` is the production constructor used by the app and the MCP server; callers supply real external clients.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:248:| MCP server | `Live` | Never (agents have no permission to escalate or de-escalate mode) |
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:348:5. **Provider and external-client wiring.** `ServiceContext` holds mode-aware external clients via `ExternalClients` (§2). `IntelligenceProvider` is held on `AbilityContext` rather than `ServiceContext` because multiple providers may co-exist (different abilities can use different providers), whereas external API clients are scoped to the service layer and share replay fixtures. `ServiceContext::new_simulate` and `new_evaluate` wire the mode-appropriate `ExternalClients` wrappers that route to replay fixtures. The surface (app, MCP, worker, harness) wires the mode-appropriate `IntelligenceProvider` into the `AbilityContext` it constructs. This split is intentional: externals are mode-routed by the service layer; provider selection is per-ability concern owned by the surface.
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:14:v1.4.3 W6 needed substrate work on 3 tickets (DOS-742 HMAC signing, DOS-743 MCP invoke audit migrate, DOS-576 11-site migration) plus DOS-741 done manually first. Per memory `feedback_parallel_agent_sizing`, mechanical work + tight rule + non-overlapping files → fan out parallel codex agents.
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:21:| DOS-743 (MCP migrate) | `/private/tmp/dailyos-743` | `surface_runtime/mod.rs` 3-branch migration | **0** |
.docs/decisions/0079-role-presets.md:22:4. **The daily briefing is universal.** Hero, Focus, The Meeting, Schedule, Priorities — none of these are CS-specific. What changes is how the AI frames urgency: renewal proximity (CS) vs close date (Sales) vs deadline (Agency/PM).
.docs/decisions/0079-role-presets.md:41:6. **Briefing emphasis** — What the daily briefing prioritizes for this role
.docs/decisions/0079-role-presets.md:45:- The daily briefing structure (Hero, Focus, The Meeting, Schedule, Priorities, Finis)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:54:  - DOS-743: MCP invoke audit threads `request_id` via `SurfacePairingAuditEvent.request_id`
.docs/decisions/0090-user-entity-page-architecture.md:94:- Downstream effect: quarterly priorities appear directly in the daily briefing's focus directive. They represent the briefing's understanding of what the user is actively moving forward.
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:145:`open_readonly()` (used by the MCP subprocess) also requires the key. The MCP subprocess runs as the same macOS user, so Keychain access is available. No architectural change needed.
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:315:- `open_readonly()` (MCP subprocess) requires the same key from Keychain. The subprocess runs as the same user -- Keychain access is available. No architectural change.
.docs/decisions/0083-product-vocabulary.md:1:# ADR-0083: Product Vocabulary — User-Facing Language Standard
.docs/decisions/0083-product-vocabulary.md:48:**The rule:** If a user can see it, it uses product vocabulary. No exceptions.
.docs/decisions/0083-product-vocabulary.md:76:| Daily Briefing | **Today** or **Daily Briefing** | Both work. "Today" is the nav label; "Daily Briefing" is the page identity. |
.docs/decisions/0083-product-vocabulary.md:132:The boundary is simple: **if a user can see it in the app, it uses product vocabulary.**
.docs/decisions/0089-user-entity-and-professional-context.md:25:**The existing `user_focus` field does not solve this.** It is a single freeform string ("What's important right now") — useful for the daily briefing's focus directive, but not structured enough to inform intelligence framing, signal prioritisation, or report generation.
.docs/decisions/0089-user-entity-and-professional-context.md:118:The user entity is personal professional context. It is never shared, never exposed via the MCP server to external tools, and never included in any outbound integration. The MCP sidecar's four read-only tools (`get_briefing`, `query_entity`, `list_entities`, `search_meetings`) do not expose user entity fields — they are for workspace intelligence, not the user's own professional context.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:294:`ClaimEnvelope::field_attribution()` exposes the full ordered list so field-attribution callouts (per ADR-0083) can render both the authoritative source AND alternatives ("primary source: X; also reported by Y").
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:472:- **MCP surface** — respects subject-ownership enforcement (DOS-288); signals do not leak across subject ownership; MCP-bridge re-tests in W4 merge gate
.docs/decisions/0081-event-driven-meeting-intelligence.md:189:### 6. Daily Briefing as Intelligence Assembly
.docs/decisions/0081-event-driven-meeting-intelligence.md:191:The daily briefing's relationship with meeting intelligence inverts. Today it *generates* intelligence. In the new model, it *assembles* and *narrates* pre-existing intelligence.
.docs/decisions/0081-event-driven-meeting-intelligence.md:295:- **Weekly forecast as primary surface vs. daily briefing.** The weekly forecast becomes the strategic view (intelligence across the week); the daily briefing becomes the tactical view (what's happening today with overnight updates). Neither is deprecated — they serve different time horizons.
.docs/decisions/0081-event-driven-meeting-intelligence.md:303:| **I220** (Meeting forecast 4-5 days ahead) | **Superseded by I327** | I220 proposed a forecast section bolted onto the daily briefing. I327 makes advance intelligence generation the default for all meetings — not an add-on, the core model. |
.docs/decisions/0048-three-tier-data-model.md:29:Meanwhile, the core insight behind ADR-0018 remains correct: **users own their data as portable files.** The workspace directory must be meaningful without the app. Markdown and JSON files are what make the archive valuable to Claude Desktop, Claude Code, and any future AI tool. This principle doesn't require the fiction that SQLite is disposable.
.docs/decisions/0048-three-tier-data-model.md:85:Claude Desktop, Claude Code, ChatGPT, and any other tool reads and writes the filesystem — markdown and JSON files. They never touch SQLite directly. The app bridges the filesystem and SQLite layers, syncing changes in both directions per ADR-0047's sync model.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:14:1. [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a typed, versioned ability. Many of those are Transform abilities that produce claims. The product will ship with multiple ability authors — `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative`, future MCP agents — each asserting claims into the same substrate.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:51:| `committed` | Claim is authoritative. Live in briefings, MCP, chat. | Included in default reads. |
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:90:Claims that do not pass the gate transition to `proposed` and become visible in the Analysis Inbox (v1.5.0 surface). They do not appear in briefings, MCP responses, or any default-read surface.
.docs/decisions/0057-entity-intelligence-architecture.md:21:- **Briefings consume entity intelligence.** The daily briefing, weekly briefing, and meeting prep are rendering surfaces. They should pull from per-entity intelligence, not generate their own. Cross-entity synthesis ("your busiest account this week is Nexus Analytics, but CloudCo's renewal is closer") is the briefing layer's job.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:65:    /// Fine on user's own surfaces, briefings, MCP. Never customer-facing or shared/published.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:70:    /// Fine on the user's own surfaces only. Not on MCP, not on Publish, not on briefings shared with others.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:202:**ADR-0102 (abilities runtime) assessment.** v1.4.6's `recommend_for_entity` ability targets the existing `mcp_exposure: McpExposure::MetadataOnly` tri-state value (added in the 2026-05-10 amendment). No further ADR-0102 amendment needed for v1.4.6 W0. v1.4.7 may amend ADR-0102 to flip `mcp_exposure` to `Invocable` once the MCP `dailyos.read.portfolio_attention` consumer ships.
.docs/decisions/0095-dual-mode-context-architecture.md:48:        endpoint: String,     // Glean MCP server URL
.docs/decisions/0095-dual-mode-context-architecture.md:62:The PTY enrichment path, signal bus, meeting prep queue, and `IntelligenceJson` schema are all unchanged. What changes is where `build_intelligence_context()` gets its data. The `GleanContextProvider` calls Glean's MCP server (search + read_document tools) to populate `IntelligenceContext.file_contents` and enriches `stakeholders` from Glean's org graph.
.docs/decisions/0095-dual-mode-context-architecture.md:107:| `src-tauri/src/context_provider/glean.rs` | `GleanContextProvider` — Glean MCP client, two-phase gather |
.docs/decisions/0095-dual-mode-context-architecture.md:127:- Glean MCP protocol may evolve — client must be maintained
.docs/decisions/0058-proactive-intelligence-maintenance.md:168:1. **Relationship drift detection.** Person temperature went from "hot" to "cold" (meeting frequency dropped). The daily briefing surfaces: "You haven't met with Sarah Chen in 6 weeks — she was previously a weekly contact."
.docs/decisions/0058-proactive-intelligence-maintenance.md:176:5. **Prep completeness audit.** Before the daily briefing generates, scan today's meetings: do all external meetings have entity intelligence? Do all attendees have resolved profiles? Surface gaps as "3 of 5 external meetings are fully prepped."
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:1:# ADR-0128 — Headless DailyOS: MCP as a co-equal product surface
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:5:**Amended:** 2026-05-04 — added §7 (CLI as a third head, mediated by MCP)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:8:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:9:**Linear:** [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:13:[ADR-0027](0027-mcp-dual-mode.md) established DailyOS as both an MCP server (exposing workspace tools and resources to external agents) and an MCP client (consuming external services). The v1.5.0 project description is sharp on guarantees: tools must route through abilities, not parallel paths; writes must capture structured intent and provenance; reads must return current substrate state; claim-worthy outputs must explain source, freshness, confidence, and salience.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:19:The frame is straightforward: **MCP is not an export pipe; it is a co-equal head over the same intelligence substrate as the Tauri app.** The Tauri app and the MCP head are two different consumption optimizations of one claim graph.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:22:- The **MCP head is responsive**: whatever question came in, answer it well inside the host model's conversation. Designed for the on-demand ask.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:26:This framing has product consequences. Treating MCP as plumbing produces a generic data API. Treating MCP as product produces a tool surface as carefully designed as any UI in the app — and a tool description corpus that is itself the highest-leverage user-facing copy in the product, because every host-model conversation routes through it.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:32:The MCP server is the **headless head** of DailyOS. It exposes the same intelligence substrate as the Tauri app, optimized for a different consumption surface (host-model conversation rather than direct visual reading).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:36:The heads are co-equal *as consumption optimizations*, not feature-parity-mandated. The MCP head does not need to expose every Tauri view, and the Tauri app does not need to mimic conversational interaction. Each surface picks the questions and ritual it serves best.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:40:MCP tool design is governed by the questions a user would ask through their host model, not by the rows in the database.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:56:- Match the voice and vocabulary of [ADR-0083](0083-product-vocabulary.md) — same product diction as the Tauri app.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:68:Consumption-first holds in the headless head. The MCP write surface is **feedback only**: corrections, dismissals, corroborations, contradictions, and tombstones, all flowing through the existing claim feedback path defined in [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:70:The MCP head does not expose:
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:76:A user in Claude Desktop can correct what DailyOS knows. They cannot use DailyOS as a write layer. Drafting and sending happen in the tool that's good at writing, with DailyOS context piped in.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:82:Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:86:### 7. CLI as a third head, mediated by MCP
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:90:**Transport.** CLI invocations route through the MCP server. There is no parallel CLI ingress to the abilities runtime. The default is MCP-mediated; a CLI-distinct transport is only justified if something genuinely cannot be expressed through the MCP surface, and that decision is deferred until the need is concrete. Keeping a single ingress to the abilities runtime is easier to reason about, easier to evolve, and prevents the v1.5.0 mission gate ("don't bypass guarantees") from being silently undermined by a second path.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:95:- MCP: substrate → claim-shaped JSON
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:102:- **CLI-for-agent.** A Claude Code skill, or any agent that shells out, invokes the CLI as part of a workflow. Skills are *macros over the MCP surface* — named, slash-invokable bundles of MCP calls with workflow shape. The agent is the consumer; the user is one step removed. Structurally similar to MCP-via-host-model.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:105:**Scope discipline is unchanged.** CLI commands mirror the **app loop** (briefing, prep, account status, commitments, action review), not the **org corpus** (Glean's territory). The displacement framing in §The displacement use case applies regardless of transport.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:112:These predate the abilities-first MCP surface. They should be refreshed once v1.5.0 lands so they consume the new tool surface and inherit its descriptions, output shape, and feedback semantics. This is a v1.5.x follow-on, not v1.5.0 substrate work — but it is the natural beneficiary of v1.5.0 once the MCP head is solid.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:118:A user is in Claude Desktop with both a Glean MCP and a DailyOS MCP connected. They ask: *"What's going on with Acme Corp?"*
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:136:The MCP surface succeeds at this question when the host model reaches for DailyOS *because the tool descriptions taught it to*, not because we blocked Glean. Each v1.5.0 tool description should be reviewable against this scenario.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:162:- "Headless" is metaphorical. Some contributors may read it as "MCP must implement every Tauri surface," which is not the intent. §1 must be explicit about co-equal-as-consumption-optimization, not feature-parity.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:164:- The displacement framing assumes a host model with multiple MCPs connected. For users with only DailyOS connected, the comparison is moot but the descriptions still teach the host model how to reason about coverage.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:165:- Feedback-only writes constrain integration depth. A user who wants Claude Desktop to "create a Linear ticket" via DailyOS will be redirected — that mutation goes through Linear's MCP or Claude Desktop's own tools, not ours. We accept this; the alternative is becoming a write layer, which the v1.5.0 project explicitly is not.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:177:- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:178:- [ADR-0083](0083-product-vocabulary.md) — Product vocabulary discipline
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:184:- [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:192:This amendment records the v1.4.7 operational consequences of the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment. It does not reopen the architecture. ADR-0128 remains the product-surface frame for headless DailyOS; ADR-0102 remains the runtime contract for MCP-originated ability invocation.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:196:The top-of-file `Linear:` citation still points to the historical v1.5.0 project. Per the 2026-05-17 renumber, the canonical project for this work is now v1.4.7. The historical header is left intact for traceability; the post-renumber canonical project is [v1.4.7 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:214:All three writes route through approved `services::*` boundaries per ADR-0101. Notes and observations flow through the claim/source services; actions and action-status updates flow through `services::actions`. The MCP head still does not expose claim creation or direct edit, file generation, calendar mutation, message composition, or external-system writes. Submit-class corrections and lightweight action tracking are the only mutations.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:224:The MCP client trust boundary is codified in the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment, especially §C: MCP client authentication, transport HMAC signing, server-side scope manifest authorization, per-actor × per-client × per-tool rate limits, revocation, and audit attribution with keyed HMAC parameter and response hashes. This ADR cites that contract because ADR-0128 is the product-surface frame, not the transport or authorization spec.
.docs/decisions/0102-abilities-as-runtime-contract.md:7:**Amended (later same day):** 2026-05-10 — promoted `mcp_exposure` from `bool` to tri-state `McpExposure { None | MetadataOnly | Invocable }` enum per Phase 0 artifact 05 lines 383-412; retained `client_side_executable: bool` as an orthogonal separate field for the WordPress 7.0 client-side `executeAbility()` JS path. The two fields govern different trust boundaries — `mcp_exposure` controls the network-facing MCP tool surface (discovery + invocation), `client_side_executable` controls whether a trusted in-product SurfaceClient may invoke the ability after WordPress capability + runtime scope + actor checks pass. The fields are independent; either may be true while the other is false. Default policy is `mcp_exposure = McpExposure::None` + `client_side_executable = false`, opt-in per ability. This cycle-3 reversal of the cycle-2 collapse was made after re-reading artifact 05's explicit warning against the symmetric `client_side_exposure` framing.  
.docs/decisions/0102-abilities-as-runtime-contract.md:8:**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
.docs/decisions/0102-abilities-as-runtime-contract.md:17:Depth of this kind cannot be assembled on demand, feature by feature. It has to be maintained continuously and composed consistently. Today, every feature in the product assembles context in its own command handler: meeting prep queries one set of tables, daily readiness queries another, the MCP sidecar queries a third. Context builders (`build_intelligence_context`, `gather_account_context`) are shared helpers but not the single invocation path. The result:
.docs/decisions/0102-abilities-as-runtime-contract.md:20:- Consumers receive subtly different context depending on how they entered the system (app vs. MCP vs. background worker).
.docs/decisions/0102-abilities-as-runtime-contract.md:30:Every product capability in DailyOS is implemented as an **ability**: a named, typed, versioned function that synthesizes state and signals into a structured output. All consumers — the desktop app, the MCP server, background workers, and any future surface — invoke capabilities exclusively through the abilities layer. The abilities layer is the runtime contract of the product.
.docs/decisions/0102-abilities-as-runtime-contract.md:182:- Inputs are versioned via `SchemaVersion` (newtype wrapping `u32`). External callers (MCP, future REST) MUST declare `schema_version`; no implicit latest. Internal callers (Tauri commands) may default to current since they rebuild with the ability. Versioning rules in §8.
.docs/decisions/0102-abilities-as-runtime-contract.md:183:- Outputs are JSON-serializable via `serde` and expose a JSON Schema via `schemars` for external introspection. No trait objects, no opaque handles. This makes Tauri bridge, MCP tool registration, and docs generation all derive from the same schema source.
.docs/decisions/0102-abilities-as-runtime-contract.md:225:    pub mcp_exposure: McpExposure,              // Tri-state exposure tier for MCP — both the Rust runtime's MCP server AND any
.docs/decisions/0102-abilities-as-runtime-contract.md:226:                                                  // SurfaceClient-mediated MCP server (e.g., WordPress MCP Adapter per ADR-0129).
.docs/decisions/0102-abilities-as-runtime-contract.md:237:    None,                                       // Ability not surfaced via MCP at all. Filtered from MCP discovery, introspection,
.docs/decisions/0102-abilities-as-runtime-contract.md:239:    MetadataOnly,                               // Ability appears in MCP discovery with name + description ONLY (no input/output
.docs/decisions/0102-abilities-as-runtime-contract.md:240:                                                  // schema, no blast-radius hints) and is not invocable via MCP. A client receives
.docs/decisions/0102-abilities-as-runtime-contract.md:244:    Invocable,                                  // Ability fully exposed via MCP, including name + description + input/output schema
.docs/decisions/0102-abilities-as-runtime-contract.md:270:- **Erased invocation** (used by external consumers — Tauri bridge, MCP, future REST): goes through the registry wrapper with JSON boundaries. Validated against schema on entry and exit.
.docs/decisions/0102-abilities-as-runtime-contract.md:278:- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
.docs/decisions/0102-abilities-as-runtime-contract.md:285:Registry enumeration is actor-scoped. An MCP client enumerating available tools sees only abilities whose `AbilityPolicy.allowed_actors` includes `Agent` AND whose `mcp_exposure` is `McpExposure::MetadataOnly` or `McpExposure::Invocable`; abilities with `mcp_exposure: None` are filtered. `MetadataOnly` abilities are enumerated with name + description only — the input/output schema and any blast-radius hints are withheld until promotion to `Invocable`; their invocation handler is not registered with the MCP server. `Invocable` abilities are enumerated with full schema and an invocation handler. Introspection does not leak ability names, input schemas, or blast radius to callers that are not authorized to invoke.
.docs/decisions/0102-abilities-as-runtime-contract.md:287:The Tauri bridge, invoked by the first-party app, sees the full registry subject to `User` and `System` filtering; the `mcp_exposure` and `client_side_executable` fields do not gate the first-party Tauri bridge because that bridge is the in-process path for the desktop app, not an MCP-mediated or SurfaceClient-mediated surface. Background workers see the `System` filter.
.docs/decisions/0102-abilities-as-runtime-contract.md:291:Surfaces (Tauri commands, MCP handlers, background workers) invoke abilities through the registry or through typed imports as described in §7.2. They do not construct `AbilityContext` directly and do not call ability functions outside these two paths. Phase 4 enforces this by module visibility.
.docs/decisions/0102-abilities-as-runtime-contract.md:301:3. If the invocation arrives via an MCP-mediated path (the Rust runtime's MCP server OR a SurfaceClient-mediated MCP server per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §4), the ability's `mcp_exposure` is `McpExposure::Invocable`. If `mcp_exposure` is `McpExposure::None`, MCP-mediated discovery AND invocation are rejected at the registry boundary. If `mcp_exposure` is `McpExposure::MetadataOnly`, MCP-mediated discovery enumerates name + description only (no input/output schema), and MCP-mediated invocation is rejected at the registry boundary. The tier check happens regardless of `allowed_actors`.
.docs/decisions/0102-abilities-as-runtime-contract.md:302:4. If the invocation arrives via a client-side SurfaceClient JS path (e.g., a WordPress 7.0 block invoking `executeAbility()`), the ability's `client_side_executable` is `true`. If `client_side_executable` is `false`, the SurfaceClient bridge rejects the invocation before registry lookup regardless of `allowed_actors`, `required_scopes`, or `mcp_exposure`. This check is orthogonal to the MCP-mediated tier check — the two fields govern different trust boundaries (per Phase 0 artifact 05 lines 383-412).
.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:308:- A SurfaceClient enumerating available abilities via an MCP-mediated path sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (c) has `mcp_exposure` of `MetadataOnly` or `Invocable`. Abilities with `mcp_exposure: None` are not enumerated. Abilities with `mcp_exposure: MetadataOnly` are enumerated with name + description only (no input/output schema); the invocation handler is not registered.
.docs/decisions/0102-abilities-as-runtime-contract.md:309:- A SurfaceClient enumerating available abilities via a client-side JS path (e.g., WordPress 7.0 `executeAbility()` enumeration) sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (d) has `client_side_executable: true`. Abilities with `client_side_executable: false` are not exposed to the client-side enumeration path even when fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:310:- Abilities failing the applicable filter ((a)/(b)/(c) for MCP-mediated paths; (a)/(b)/(d) for client-side JS paths) are not enumerated regardless of caller permission.
.docs/decisions/0102-abilities-as-runtime-contract.md:311:- Per-ability metadata fields (name, description, schema, annotations, category) are model-facing AND browser-facing API surface for SurfaceClient consumers. Description copy is reviewed with the same care as user-facing UI (per [ADR-0083](0083-product-vocabulary.md) discipline).
.docs/decisions/0102-abilities-as-runtime-contract.md:312:- A SurfaceClient does NOT see ability names, schemas, or blast radius for abilities outside its scope grants, for `mcp_exposure: None` abilities on MCP-mediated paths, or for `client_side_executable: false` abilities on client-side JS paths. Unauthorized introspection is closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:314:**Default policy.** Abilities default to `allowed_actors = [User]`, `required_scopes = vec![]`, `mcp_exposure = McpExposure::None`, `client_side_executable = false`. `SurfaceClient` exposure is opt-in per ability — the macro attribute must declare `allowed_actors` including `SurfaceClient`, plus `required_scopes` AND `mcp_exposure` AND `client_side_executable` explicitly (or `no_scope_required` in place of an empty `required_scopes` when the absence is intentional; see §7.1 macro). Most existing abilities do NOT need to be exposed to SurfaceClient instances; ability authors must consciously opt each one in. The two exposure fields govern different trust boundaries (MCP tool surface vs. client-side JS invocation) per Phase 0 artifact 05 lines 383-412 — either may be opted in while the other remains closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:342:External callers (MCP, REST) MUST include `schema_version` in every invocation. The registry rejects calls without it. This eliminates the "implicit latest" footgun where a breaking change silently returns a different shape to agents.
.docs/decisions/0102-abilities-as-runtime-contract.md:352:Tauri commands, MCP handlers, and future surfaces either invoke abilities through the registry (erased, validated, schema-checked) or through typed imports as described in §7.2. They do not construct `AbilityContext` themselves and do not invoke ability internals.
.docs/decisions/0102-abilities-as-runtime-contract.md:449:`prepare_meeting`, `get_daily_readiness`, `get_entity_context`, `list_open_loops`, `detect_risk_shift` migrate behind abilities. App Tauri commands become thin wrappers. MCP sidecar query handlers become thin wrappers over the same abilities. Parallel-run validation per [ADR-0109](0109-migration-strategy.md).
.docs/decisions/0102-abilities-as-runtime-contract.md:452:Remaining capabilities migrated. Direct Tauri commands for capability-level operations removed. MCP query handlers removed in favor of ability-wrapped tools. Pre-commit hook blocks new capability code outside `src-tauri/src/abilities/`.
.docs/decisions/0102-abilities-as-runtime-contract.md:459:The current architecture — Tauri commands that query services and build context inline, plus an MCP sidecar with parallel query paths — remains operational throughout the migration. Each capability is migrated individually:
.docs/decisions/0102-abilities-as-runtime-contract.md:463:3. The corresponding MCP tool is replaced with a thin wrapper over the same ability.
.docs/decisions/0102-abilities-as-runtime-contract.md:475:3. **Surface parity is structural.** App, MCP, and any future surface receive identical outputs because they invoke the same abilities. No "MCP version is thinner."
.docs/decisions/0102-abilities-as-runtime-contract.md:478:6. **Agents receive full product quality.** MCP tools are thin wrappers over abilities, not reduced subsets. When an agent invokes DailyOS, it receives the same briefing the app renders.
.docs/decisions/0102-abilities-as-runtime-contract.md:515:- **ADR-0108 (forthcoming): Surface-Independent Ability Invocation** — Specifies MCP and Tauri binding rules concretely (which follows from §7.3).
.docs/decisions/0102-abilities-as-runtime-contract.md:556:- Surface exposure: experimental abilities are registered and invokable only when a feature flag is active. Not exposed through MCP. Not exposed through Tauri commands unless the dev-mode flag is set.
.docs/decisions/0102-abilities-as-runtime-contract.md:571:  - **No MCP exposure:** experimental abilities are not enumerated by `registry.iter_for(Actor::Agent)` or MCP bridge output.
.docs/decisions/0102-abilities-as-runtime-contract.md:579:## Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace
.docs/decisions/0102-abilities-as-runtime-contract.md:581:**Origin:** v1.4.7 W0 (MCP Server v2, Abilities-First). Drafted as the substantive design output of v1.4.7's cycle-2 Class S finding ("MCP client trust boundary"). Cycles 2–5 polished the contract; this amendment lands the final shape.
.docs/decisions/0102-abilities-as-runtime-contract.md:583:**Scope:** the abilities runtime contract for MCP-originated invocations. Wire-level transport, pairing handshake server impl, per-tool grant manifest format, and gateway impl are W1-A scope and not codified by this amendment — only the actor-class contract and namespace freeze are.
.docs/decisions/0102-abilities-as-runtime-contract.md:603:`McpClientId` is a server-minted opaque handle issued during the MCP client pairing handshake (§C). Newtype lives at the abilities-runtime layer alongside `SurfaceClientId`; non-PII, non-enumerable, stable for the life of the pairing.
.docs/decisions/0102-abilities-as-runtime-contract.md:605:`OpaqueConversationHandle` is a server-minted, non-PII, non-enumerable token that tracks cross-conversation continuity (§D). `Option<…>` because the handle is absent on the very first MCP call in a fresh conversation — the gateway mints one transparently per §D.
.docs/decisions/0102-abilities-as-runtime-contract.md:614:- **McpClient** does not carry scopes because the MCP gateway (v1.4.7 `services/mcp_v2/gateway.rs`) enforces scope authorization against a **server-side per-client manifest** loaded by `services/mcp_v2/auth.rs` from the pairing record (§C). Caller-provided scope claims are rejected on principle; the gateway is the single source of truth. Once the gateway admits the call, the abilities runtime trusts the gateway's check and gates only on `AbilityPolicy.allowed_actors.contains(&ActorKind::McpClient)`.
.docs/decisions/0102-abilities-as-runtime-contract.md:616:This preserves the v1.4.2 SurfaceClient bridge contract verbatim while introducing a strictly distinct trust path for MCP-originated calls — different attribution semantics, different audit shape, different rate-limit budget (§C). `AbilityPolicy.required_scopes` remains a SurfaceClient-only enforcement field; W0 ships no new field on `AbilityPolicy`.
.docs/decisions/0102-abilities-as-runtime-contract.md:618:### C. MCP client authentication + session contract
.docs/decisions/0102-abilities-as-runtime-contract.md:620:Every MCP-originated invocation must pass through these four gates before reaching an ability:
.docs/decisions/0102-abilities-as-runtime-contract.md:622:1. **Pairing handshake.** First contact from a new MCP client issues an `McpClientId` after a server-side handshake (W1-A `auth::pair_client(handshake) -> McpClientId`). The handshake binds the client to a server-stored manifest of granted scopes + per-tool rate limits + per-tool exposure tier. Manifest content is set by the operator (DailyOS-as-product owner) at pairing time, not negotiated by the caller. Mirrors the v1.4.2 `SurfaceClient` pairing shape (`services::surface_pairing`, `services::surface_session_keychain`, `bridges::surface_client`) — the SurfaceClient pattern is the precedent and the same operator/audit ergonomics apply.
.docs/decisions/0102-abilities-as-runtime-contract.md:623:2. **Transport signing (HMAC-SHA256).** All stdio and HTTP transports MUST present a per-message HMAC-SHA256 signature derived from the pairing-time shared key. Unsigned or invalid-signature messages are rejected at the transport layer before any `Actor::McpClient` is constructed. (Local STDIO MCP processes share the key out-of-band via the pairing record; loopback HTTP carries it as a request header.)
.docs/decisions/0102-abilities-as-runtime-contract.md:635:- **Mint trigger.** Server mints on the first MCP call from a client that does not present an existing handle. The new handle is returned in the response envelope's top-level `conversation_handle` field (separate from the tool's typed payload).
.docs/decisions/0102-abilities-as-runtime-contract.md:644:Canonical convention for new MCP tool scopes:
.docs/decisions/0102-abilities-as-runtime-contract.md:652:- **v1.4.5-reused scopes stay unprefixed.** Scopes shipped by v1.4.5 substrate (e.g. `write.workspace_place_document`, `read.workspace_graph`, `read.entity_names`) are kept verbatim. v1.4.7 MCP tools that reuse them present the unprefixed scope to the manifest. The manifest validation logic accepts both forms but rejects any scope outside the canonical allowlist.
.docs/decisions/0102-abilities-as-runtime-contract.md:657:`AbilityPolicy.allowed_actors` may now include `ActorKind::McpClient` alongside the existing four kinds. Defaults are unchanged (`&[ActorKind::User]`, least-privilege floor); abilities opt in to MCP exposure by adding `ActorKind::McpClient` to their `allowed_actors` slice.
.docs/decisions/0102-abilities-as-runtime-contract.md:663:No new variants are added to `McpExposure { None | MetadataOnly | Invocable }`. The tri-state continues to govern MCP introspection behavior verbatim for v1.4.7 — `None` hides the ability from the v1.4.7 gateway's `list_tools` and `list_abilities`, `MetadataOnly` exposes name + description without invoke schema, `Invocable` exposes full schema.
.docs/decisions/0102-abilities-as-runtime-contract.md:669:- The MCP gateway implementation (`services/mcp_v2/gateway.rs` body) — that is W1-A (DOS-168).
.docs/decisions/0102-abilities-as-runtime-contract.md:675:- Any actual MCP tool handler logic — W2/W3/W4.
.docs/decisions/0102-abilities-as-runtime-contract.md:713:Origin: v1.4.7 W1-A L0 cycle 5 CSO HIGH finding + L6 verdict. The cycle-1 §C contract specifies per-message HMAC-SHA256 transport signing but does NOT require replay protection. A captured, signed MCP message could be replayed by an attacker within the validity window of the pairing key OR the `OpaqueConversationHandle` (up to 24 hours). For a v1.4.7 W4 write-tool (`dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`, `dailyos.write.place_document`), replay produces duplicate mutations.
.docs/decisions/0102-abilities-as-runtime-contract.md:717:Every MCP-originated message MUST include a `request_nonce` field whose value is signed by the transport HMAC alongside the payload. The nonce is server-issued during the pairing handshake (initial seed) and refreshed per response via a server-side ledger. Format: 128-bit cryptographically-random opaque token.
.docs/decisions/0102-abilities-as-runtime-contract.md:794:This amendment does NOT change: the HMAC-SHA256 signing algorithm; the pairing handshake (still issues the seed nonce); rate-limit ledger; manifest schema; audit shape (success-only per L6-2); `OpaqueConversationHandle` lifecycle. The 5-minute expiry window is fixed in W1-A; configurable TTL is filed as DOS-? "MCP nonce TTL configuration" in the Maintenance project for post-v1.4.7 tuning.
.docs/decisions/0025-app-native-governance.md:12:Governance logic lives in compiled Rust inside the Tauri app. The app reads `~/.dailyos/` registries and workspace structure directly. No Python subprocess calls for core operations. The app is the authority on first install; CLI and MCP are secondary interfaces that share the same registries.
.docs/decisions/0025-app-native-governance.md:18:- Interface parity: App, CLI, and Claude Code (via MCP) all read the same state
.docs/decisions/0076-brand-identity.md:96:Briefing surfaces (daily briefing, weekly briefing, meeting intelligence reports) are the primary reading experience. They use a **constrained subset** of the full palette to maintain editorial calm. The full 14-color system is the brand; the briefing palette is its most disciplined application.
.docs/decisions/0105-provenance-as-first-class-output.md:350:3. **Serialization overhead on every Tauri/MCP response.** Acceptable at scale but measurable.
.docs/decisions/0107-source-taxonomy-alignment.md:261:v1.4.5 Workspace Memory introduces ingestion of local workspace files into the claim store. Per ADR-0107 §1, adding a new `DataSource` variant requires an amendment, not the `Other` escape hatch. This amendment defines the typed variant plus its scoring/lifecycle posture so v1.4.5 W1+ lanes can freeze their lifecycle records and W4-C MCP placement can build against a stable contract. Triggered by v1.4.5 W0 close gate per `.docs/plans/v1.4.5-waves.md` §'Cycle 2 amendments' #4 and §'W0 close gate'.
.docs/decisions/0107-source-taxonomy-alignment.md:291:| `McpPlacement` | files placed by an AI agent via the MCP `workspace_place_document` ability |
.docs/decisions/0107-source-taxonomy-alignment.md:322:- v1.4.5 W4-C DOS-474: MCP `place_document` ability writes claims with `kind=McpPlacement`.
.docs/decisions/0108-provenance-rendering-and-privacy.md:13:1. **Rendering rules per surface** — what the Tauri app, MCP, P2 publications, and logs show.
.docs/decisions/0108-provenance-rendering-and-privacy.md:31:**MCP server (agent-facing).**
.docs/decisions/0108-provenance-rendering-and-privacy.md:34:- Actor-filtered contents: the MCP wrapper strips content from provenance that is not permitted for `Actor::Agent`:
.docs/decisions/0108-provenance-rendering-and-privacy.md:39:- An MCP tool `get_provenance(invocation_id)` provides a fuller view for authorized agents if needed; default response is summary-sized.
.docs/decisions/0108-provenance-rendering-and-privacy.md:70:The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.
.docs/decisions/0108-provenance-rendering-and-privacy.md:117:- **MCP default response:** provenance payload ≤ 10KB. Full detail available via `get_provenance(invocation_id)`.
.docs/decisions/0108-provenance-rendering-and-privacy.md:128:- Meeting prep, daily briefing, weekly narrative, risk assessment, entity context detail pages
.docs/decisions/0108-provenance-rendering-and-privacy.md:129:- MCP tool responses
.docs/decisions/0108-provenance-rendering-and-privacy.md:145:1. **Rendering rules are surface-specific.** The Tauri app gets rich detail; MCP gets actor-filtered; P2 gets heavy redaction; logs get operational metadata. No one-size-fits-all.
.docs/decisions/0108-provenance-rendering-and-privacy.md:156:3. **MCP `get_provenance(invocation_id)` tool adds surface.** Agents can request detail they did not receive by default.
.docs/decisions/0108-provenance-rendering-and-privacy.md:162:2. **Renderer drift across surfaces.** The Tauri app diverges from MCP in how it interprets provenance. Mitigation: `render_provenance_for` is the single entry point; surfaces cannot bypass it.
.docs/decisions/0108-provenance-rendering-and-privacy.md:164:4. **P2 publication leaks through "Details" toggle.** User opts in to full detail and leaks internal IDs. Mitigation: publication "Details" renders through the same actor filter as MCP when the P2 audience includes non-team members; team-internal publishes get full detail.
.docs/decisions/0111-surface-independent-ability-invocation.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:14:This ADR specifies the exact binding pattern for Tauri commands and MCP tools, how abilities are generated into both surfaces from the registry, and how surfaces construct `AbilityContext` with mode, actor, and provenance renderer wiring.
.docs/decisions/0111-surface-independent-ability-invocation.md:25:| MCP server | `Agent` | `Live` (never escalated) | `McpAbilityBridge` |
.docs/decisions/0111-surface-independent-ability-invocation.md:72:### 3. MCP Tool Registration
.docs/decisions/0111-surface-independent-ability-invocation.md:74:The MCP server iterates the registry at startup and registers one MCP tool per ability whose `AbilityPolicy.allowed_actors` includes `Agent`:
.docs/decisions/0111-surface-independent-ability-invocation.md:96:8. Returns the MCP response with domain output and filtered provenance.
.docs/decisions/0111-surface-independent-ability-invocation.md:98:**No hand-written MCP tool handlers.** Every MCP tool is an ability. Pre-existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) that do not correspond to an ability are either migrated to an ability or removed during Phase 3 cutover.
.docs/decisions/0111-surface-independent-ability-invocation.md:102:MCP tool discovery (the `list_tools` endpoint) is filtered by actor. An agent enumerating the server's tools sees only abilities whose policy permits `Agent`. Maintenance abilities, admin-only abilities, and abilities that specifically exclude agents never appear in the list.
.docs/decisions/0111-surface-independent-ability-invocation.md:157:- MCP server registers ability-derived tools alongside existing hand-written tools (both paths active).
.docs/decisions/0111-surface-independent-ability-invocation.md:158:- App and MCP progressively switch from hand-written to registry-backed.
.docs/decisions/0111-surface-independent-ability-invocation.md:163:- Hand-written MCP tools removed; all MCP tools come from the registry.
.docs/decisions/0111-surface-independent-ability-invocation.md:173:The original three actor classes (`User`, `Agent`, `System`) were sufficient when DailyOS's surfaces were exclusively first-party: Tauri app for `User`, MCP for `Agent`, background workers / eval / tests for `System`. The decision in [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) to support third-party local surfaces — initially WordPress, generalized to any local CMS, editor, extension, or paired app that mediates user interactions with the substrate — introduces an actor that does not fit those three.
.docs/decisions/0111-surface-independent-ability-invocation.md:222:Registry enumeration for `SurfaceClient` filters at two levels: the actor class (is `SurfaceClient` in the ability's `allowed_actors`?) AND the per-instance scope grants (does this specific instance have the scope required for this ability?). Both must pass. Abilities NOT permitted for `SurfaceClient` invocation never appear in any SurfaceClient's tool surface, even if they appear for `User`. Abilities with `mcp_exposure: McpExposure::None` (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1 + §7.6) are not enumerated for SurfaceClient-mediated MCP servers regardless of permission state; abilities with `mcp_exposure: McpExposure::MetadataOnly` are enumerated with name + description only (no input/output schema); no invocation handler is registered. Full schema is reserved for `McpExposure::Invocable`. The orthogonal `client_side_executable: bool` field on `AbilityPolicy` separately gates the WordPress 7.0 client-side `executeAbility()` JS path; an ability fully `Invocable` via MCP may still be `client_side_executable: false`, and vice versa, per Phase 0 artifact 05 lines 383-412.
.docs/decisions/0111-surface-independent-ability-invocation.md:239:1. **Surface parity is structural.** App, MCP, worker, eval harness all invoke abilities through the same registry path; output quality is identical across surfaces.
.docs/decisions/0111-surface-independent-ability-invocation.md:240:2. **MCP tool registration is automatic.** Adding a new Agent-accessible ability adds a new MCP tool with no hand-written code.
.docs/decisions/0111-surface-independent-ability-invocation.md:249:2. **MCP migration is non-trivial.** Existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) must be refactored into abilities or retired.
.docs/decisions/0111-surface-independent-ability-invocation.md:250:3. **Bridge code is mostly boilerplate.** Four bridges (Tauri, MCP, worker, eval) with similar shape. Potential to factor but not in v1.4.0.
.docs/decisions/0111-surface-independent-ability-invocation.md:254:1. **Hand-written MCP tool drift.** A contributor adds a hand-written MCP tool in Phase 2 that never gets migrated. Mitigation: Phase 3 gate requires zero hand-written MCP tools; CI enforces.
.docs/decisions/0111-surface-independent-ability-invocation.md:256:3. **Confirmation token misuse in MCP.** Agents attempting to supply confirmation tokens via MCP. Mitigation: MCP bridge refuses `confirmation` field; tokens are user-initiated via Tauri only.
.docs/decisions/0111-surface-independent-ability-invocation.md:258:5. **List-tools API bloat.** If many abilities are Agent-accessible, the MCP tool list becomes large. Mitigation: actor filtering already narrows scope; categorization lets agents discover by category.
.docs/decisions/0111-surface-independent-ability-invocation.md:263:- [ADR-0027: MCP Dual-Mode](0027-mcp-dual-mode.md) — Existing MCP sidecar; Phase 3 retires its hand-written tools in favor of registry-derived ones.
.docs/decisions/0042-per-operation-pipelines.md:8:The monolithic three-phase pattern (Prepare → Enrich → Deliver) was designed for the CLI, where one command produced the full daily briefing. The app has scheduling, background execution, and progressive UI rendering — it doesn't need everything in one shot.
.docs/decisions/0055-schedule-first-dashboard-layout.md:9:ADR-0053 shipped the readiness strip, sidebar flip, greeting removal, and maxVisible improvements. But the daily briefing page still buries the meeting schedule behind 440-560px of preamble (date heading, focus callout, summary card, readiness strip, intelligence card). On a 13" MacBook, the schedule can be completely below the fold on a loaded day.
.docs/decisions/0055-schedule-first-dashboard-layout.md:15:Restructure the daily briefing page from a sequential top-to-bottom flow to a two-column layout:
.docs/decisions/0046-entity-mode-architecture.md:57:Integrations are MCP data source connectors (ADR-0027). Any integration works with any entity mode.
.docs/decisions/0046-entity-mode-architecture.md:66:Each integration is an MCP server. The app is an MCP client. Community can build integrations without touching core code.
.docs/decisions/0046-entity-mode-architecture.md:177:- Community can build MCP integrations without touching core.
.docs/decisions/0046-entity-mode-architecture.md:185:- MCP client infrastructure must be production-ready before integrations work.
.docs/decisions/0046-entity-mode-architecture.md:193:- Chose MCP for integrations over custom protocol — leverages ecosystem, community-contributable, already decided in ADR-0027.
.docs/decisions/0059-entity-directory-template.md:14:2. **New accounts from the app had no scaffold** — external tools (Claude Desktop, ChatGPT, CLI tools) browsing an entity directory saw no organizational structure.
.docs/decisions/0096-glean-mode-local-footprint.md:35:| Clay enrichment metadata | Clay MCP | Clay disabled in both strategies |
.docs/decisions/0084-surface-jobs-to-be-done.md:6:**Related:** ADR-0073 (Editorial design language), ADR-0077 (Magazine layout), ADR-0081 (Event-driven meeting intelligence), ADR-0083 (Product vocabulary), I342 (Surface JTBD critique)
.docs/decisions/0084-surface-jobs-to-be-done.md:14:A six-agent audit (I342 Phases 1-2) inventoried every visible element across all five surface types: Daily Briefing, Weekly Forecast, Meeting Briefing, Actions, and Entity Detail. The audit found significant content duplication (four independent action row implementations, three meeting row implementations, the Lead Story duplicating 60-70% of Meeting Detail), dead code, system vocabulary throughout, and sections that couldn't articulate their job.
.docs/decisions/0084-surface-jobs-to-be-done.md:24:#### Daily Briefing (Today)
.docs/decisions/0084-surface-jobs-to-be-done.md:67:**Note:** "Your Meetings" and "Commitments" chapters are removed. They duplicated the daily briefing and actions page with less capability.
.docs/decisions/0084-surface-jobs-to-be-done.md:141:| A4 | "Later This Week" action group | Daily Briefing | Priorities becomes Overdue + Due Today only |
.docs/decisions/0084-surface-jobs-to-be-done.md:175:| B1 | EntityPicker on daily briefing | User prefers keeping editing affordances accessible on the reading surface |
.docs/decisions/0084-surface-jobs-to-be-done.md:187:- **Daily Briefing** SURFACES today's situational awareness from all sources. It is a reading surface that creates pull toward detail pages.
.docs/decisions/0084-surface-jobs-to-be-done.md:192:The expanded meeting card on the daily briefing schedule is a **pre-meeting orientation layer**. Its job is to answer "what is this meeting and what are we covering" — not to surface strategic context or account intelligence. That distinction keeps the card focused and the information hierarchy clear.
.docs/decisions/0084-surface-jobs-to-be-done.md:240:- **The daily briefing stays in its lane.** Today only. No weekly leakage.
.docs/decisions/0084-surface-jobs-to-be-done.md:251:- **Lead Story compaction** could reduce the daily briefing's perceived value if the intelligence narrative isn't compelling enough on its own. Mitigate by ensuring the narrative is high-quality, not just the first paragraph of the meeting prep.
.docs/decisions/0084-surface-jobs-to-be-done.md:260:- ADR-0083: Product vocabulary (verbal identity)
.docs/decisions/0127-presets-as-intelligence-contracts.md:29:5. **Feature flag + display labels** (DOS-177) — the `role_presets_enabled` flag gates the preset selection UI. Health dimension labels (Meeting Cadence, Champion Health, etc.) resolve from `preset.intelligence.dimensionLabels` at render time, falling back to hardcoded ADR-0083 vocabulary.
.docs/decisions/README.md:50:| [0027](0027-mcp-dual-mode.md) | MCP integration: dual-mode server + client | Accepted |
.docs/decisions/README.md:95:| [0075](0075-conversational-interface-architecture.md) | Conversational interface architecture — MCP-first chat, session memory, entity-focused queries | Accepted |
.docs/decisions/README.md:102:| [0083](0083-product-vocabulary.md) | Product vocabulary — user-facing language standard | Proposed |
.docs/decisions/README.md:130:| [0111](0111-surface-independent-ability-invocation.md) | Surface-independent ability invocation — Tauri/MCP/worker bridges (v1.4.0) | Proposed |
.docs/decisions/0130-surface-independent-composition-contract.md:156:| MCP head (headless) | `JsonBlockRenderer` | Claim-shaped JSON per ADR-0128 §4 |
.docs/decisions/0130-surface-independent-composition-contract.md:162:Surfaces MAY decline to render specific block types (e.g., MCP head returns claim-JSON for `AgenticPrompt` rather than rendering an invocable UI; CLI head emits a marker for `MagazineEnd` rather than visual spacing). The Composition stays canonical; surfaces decide what to show.
.docs/decisions/0130-surface-independent-composition-contract.md:188:The same composition rendered by the WordPress SurfaceClient, Tauri React surface, and MCP head produces equivalent intelligence — different paint, same content, same provenance.
.docs/decisions/0130-surface-independent-composition-contract.md:222:- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.
.docs/decisions/0130-surface-independent-composition-contract.md:243:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
.docs/decisions/0052-week-page-redesign.md:16:Weekly prep answers different questions than the daily briefing:
.docs/decisions/0073-editorial-design-language.md:21:### Design Principle: "A beautifully typeset daily briefing laid on a warm desk."
.docs/decisions/0073-editorial-design-language.md:134:- **Daily Briefing:** Narrative headline → focus pull-quote → meeting cards with accent bars → action rows → finite ending
.docs/decisions/0073-editorial-design-language.md:162:- Dashboard (daily briefing) — already partially aligned via ADR-0055, Sprint 27 refines
.docs/decisions/0077-magazine-layout-editorial-redesign.md:21:1. Daily Briefing (daily-magazine-layout.html)
.docs/decisions/0077-magazine-layout-editorial-redesign.md:52:  - Daily Briefing: turmeric + larkspur
.docs/decisions/0077-magazine-layout-editorial-redesign.md:91:The actions list groups by when (overdue → today → this week → upcoming), not by who. The daily briefing features the most important meeting. The weekly forecast organizes by day-of-week and theme, not just listing all events.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:107:**Daily Briefing:**
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:5:**Amended:** 2026-05-10 — §4 reframed (WP path uses WP Abilities API + MCP Adapter, not PHP MCP client) per Codex L0 Prep follow-up on DOS-546. Primary-source research confirmed Abilities API in WordPress core since 6.9 (December 2025); MCP Adapter ships at `github.com/WordPress/mcp-adapter`.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:7:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md), [ADR-0127](0127-presets-as-intelligence-contracts.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:8:**Linear:** [v1.4.1 — Abilities Runtime Completion](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7), [v1.4.2 — Entity Intelligence](https://linear.app/a8c/project/v142-entity-intelligence-accounts-projects-people-a448d2e072ad), [v1.4.7 — MCP Server v2](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9), [v1.4.9 — Self-Healing v2](https://linear.app/a8c/project/v149-self-healing-v2-da4bc5769164), [DOS-540](https://linear.app/a8c/issue/DOS-540)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:12:[ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) established the architectural frame: *the substrate is the product; the heads are surfaces over it.* It named two heads — the Tauri app (visual, opinionated, daily ritual) and the MCP server (responsive, headless, on-demand) — and explicitly anticipated more: *"Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern."*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:19:- v1.4.2 onward (Entity Intelligence, Briefing Experience, Claim/Trust UI, Salience, Workspace Memory, MCP Server v2, Reports, Self-Healing) is about to begin significant interface design and implementation work — the "redesign tokens" of the program.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:21:- The MCP head exists in v1.4.7's Linear scope as a late-program deliverable.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:25:- **MCP is now genuinely cross-vendor.** Spec 2025-11-25 with OAuth 2.1, Step-Up Auth, Async Tasks. OpenAI and Google ship MCP clients in production; the standard is no longer Anthropic-led.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:40:ADR-0128 framed the architecture as surface-pluggable. It treated the Tauri app as the primary visual surface and MCP as a co-equal headless head. This ADR commits to the next step in that architecture: **WordPress Studio is added as a new head and is promoted to primary**, with the Tauri app reorienting toward runtime-host duties and the MCP head elevating from a late-program feature to the foundation contract every v1.4.2+ wave consumes.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:60:- A magazine theme implementing the editorial reading surface, finite endings, hierarchy, and product vocabulary discipline (per [ADR-0083](0083-product-vocabulary.md)).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:63:- Composition templates for daily briefings, account pages, meeting prep, and report drafts — anchored shapes the user can deviate from. Layouts may also be **agent-composed on the fly** based on substrate salience, not bound to fixed templates.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:71:This ADR reorders the program: the substrate-to-surface contract (currently scoped inside v1.4.7) is **promoted to a foundation wave between v1.4.1 and v1.4.2**. v1.4.7's MCP server work continues as the first external implementation of the contract; the contract itself ships earlier so that v1.4.2 entity surfaces, v1.4.3 briefing surfaces, v1.4.4 claim/trust surfaces, and v1.4.5 salience surfaces all consume the same substrate-to-surface API.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:73:### 4. WordPress accesses the substrate via its own Abilities API + MCP Adapter; non-WP surfaces use the Rust runtime's MCP server directly
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:75:*Reframed 2026-05-10 per Codex L0 Prep follow-up on DOS-546. The original framing — "WordPress plugin opens an MCP session to the localhost-bound runtime" — was based on a PHP MCP client architecture that primary-source research showed isn't necessary and isn't the canonical WP integration target.*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:79:- **WordPress path.** The DailyOS WP plugin registers abilities into [WordPress's Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) (in WordPress core since 6.9, December 2025; client-side API merged into Gutenberg for 7.0, shipping 2026-05-20). Those abilities delegate to the Rust runtime over a local transport (loopback HTTP REST as primary candidate; Unix domain socket as fallback; spawned stdio as last-resort). The [WordPress MCP Adapter plugin](https://github.com/WordPress/mcp-adapter) exposes registered abilities as MCP tools automatically — WordPress itself becomes an MCP server consumable by Claude Desktop, Cursor, VS Code, and any other MCP client connected to the WP site. Gutenberg blocks consume the abilities through WP-native infrastructure (REST endpoints, capabilities, nonces, and the WP 7.0 client-side `executeAbility()`).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:80:- **Non-WP path.** The Rust runtime exposes its own MCP server for surfaces that don't sit inside WordPress (Claude Desktop direct without a WP install, Cursor, future PWA, future mobile). Same substrate, two MCP servers — one WP-mediated for WP-consuming clients, one direct for everything else.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:82:DailyOS does **not** consume MCP from PHP. The seam between WordPress and the runtime is a local PHP-to-Rust transport, not an MCP client; the WP Abilities API + MCP Adapter does the MCP server work on the WP side.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:84:This preserves [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §7's discipline (single ingress to the abilities runtime) — every external invocation, whether routed through the WP MCP server or the runtime's direct MCP server, ultimately resolves to one substrate access path inside the runtime. The runtime exposes one substrate access protocol; clients vary in how they render and compose against it.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:86:**MCP Adapter exposure policy.** Substrate-backed abilities must NOT be exposed by the *default* WP MCP server. DailyOS ships a custom MCP server configuration with an explicit ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. Permission callbacks check both WP capabilities and DailyOS `SurfaceClient(WordPress)` scopes (per [ADR-0111](0111-surface-independent-ability-invocation.md)); site-wide WP capabilities alone are not the isolation boundary.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:103:- **Templated layouts.** Curated page templates (daily briefing, account page, meeting prep, report) that the user can deviate from. Anchored shapes.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:113:- **Runtime host.** Rust services, cron, the MCP server, local privacy gate, abilities execution, signal propagation, claim writes. This is what Tauri does well and continues to do.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:116:The Rust substrate engine, abilities runtime, signal propagation, claim feedback path, and MCP server all continue exactly as v1.4.0/v1.4.1 designed them. None of that work is wasted. The investment has always been in the runtime; this ADR makes that explicit.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:122:ACP (Zed's Agent Client Protocol) is the leading candidate for the protocol implementation. The architectural commitment is to the **shape** (out-of-process agent, session-shaped invocation, MCP-shaped tool surface), not to the specific protocol. Implement ACP first as the leading candidate; leave room for a second protocol if a different one matures faster.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:124:This composes with §4: agents reach the substrate via MCP; surfaces reach the substrate via MCP; agent backend is pluggable; surface is pluggable; runtime is the constant.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:131:- **(New foundation wave between v1.4.1 and v1.4.2)** — substrate-to-surface contract. Formerly part of v1.4.7's scope. Typed read/write/subscribe APIs, composition primitives, identity bridge. MCP server is the first external implementation.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:137:- **v1.4.7** — MCP Server v2. Repurposed: the contract foundation has shipped earlier; v1.4.7 now focuses on the rich tool surface, displacement-test discipline (per ADR-0128), and the cross-surface story.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:156:- **Not abandoning the Tauri runtime.** The Rust substrate, abilities runtime, signal propagation, MCP server, cron, and local privacy gate continue exactly as v1.4.0/v1.4.1 design. The runtime is the product.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:157:- **Not requiring WordPress to consume DailyOS.** Headless surfaces (Claude Desktop via MCP, Cursor, future) work without WordPress. WordPress is the *primary* composable surface, not the *only* surface.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:171:- **Cross-surface story becomes coherent.** WordPress for primary composition; Claude Desktop and future agent shells for headless conversation; substrate sovereign. Each surface picks the questions and ritual it serves best, per ADR-0128 §1.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:184:- **MCP and ACP maturity dependencies.** MCP is mature and cross-vendor; ACP is younger. Section 8 mitigates by committing to the *shape* not the protocol.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:197:- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:198:- [ADR-0083](0083-product-vocabulary.md) — Product vocabulary discipline
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:204:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS: MCP as a co-equal product surface (this ADR extends)
.docs/decisions/0047-entity-dashboard-architecture.md:12:2. **The ecosystem** — Claude Desktop, Claude Code, ChatGPT, and any future AI tool pointed at the workspace directory needs rich, queryable, AI-consumable files
.docs/decisions/0047-entity-dashboard-architecture.md:22:- **Markdown must be rich.** An account dashboard read by Claude Desktop should contain company overview, stakeholder map, strategic programs, recent wins, open actions, engagement signals — everything needed to answer complex questions about the account.
.docs/decisions/0047-entity-dashboard-architecture.md:24:- **External tools must be able to write.** Claude Desktop asked to "update Acme's strategy section" should be able to do so without understanding the app's internals.
.docs/decisions/0047-entity-dashboard-architecture.md:149:1. External tool writes to `dashboard.json` (e.g., Claude Desktop updates strategy notes)
.docs/decisions/0116-tenant-control-plane-boundary.md:14:**Precise wording required — 2026-04-20 update (outside voice finding #2).** "Stays on your laptop" is not accurate without qualification when Glean is the intelligence provider (per [ADR-0100](0100-glean-first-intelligence-architecture.md)). Glean's MCP chat tool receives prompts, returns completions, and sees the user's Glean-connected source data (REDACTED, Zendesk, Gong, Slack) in the Glean infrastructure the customer has already contracted with. That is not DailyOS control-plane — it is the user's pre-existing relationship with Glean — but it's also not "on the laptop."
.docs/decisions/0075-conversational-interface-architecture.md:16:- ADR-0027: MCP dual-mode — already establishes DailyOS as both MCP server (expose tools) and client (consume external data). Chat via MCP tools is a natural extension of the server role.
.docs/decisions/0075-conversational-interface-architecture.md:22:### Phase 1: External via MCP (post-ship)
.docs/decisions/0075-conversational-interface-architecture.md:24:Claude Desktop (or any MCP-compatible client) calls DailyOS MCP tools to query entities. No in-app chat UI needed.
.docs/decisions/0075-conversational-interface-architecture.md:26:**MCP tools to expose:**
.docs/decisions/0075-conversational-interface-architecture.md:34:- Zero UI investment — Claude Desktop already has a polished chat interface
.docs/decisions/0075-conversational-interface-architecture.md:35:- Validates demand — if users actually use MCP tools, we have evidence for in-app investment
.docs/decisions/0075-conversational-interface-architecture.md:36:- Aligns with ADR-0027 — MCP server mode is already planned
.docs/decisions/0075-conversational-interface-architecture.md:87:Only build if Phase 1 MCP tools see significant usage. In-app chat would be:
.docs/decisions/0075-conversational-interface-architecture.md:97:- **Zero UI investment for Phase 1** — MCP tools are backend-only, exposed via existing Tauri command pattern
.docs/decisions/0075-conversational-interface-architecture.md:98:- **Validates demand** — usage data from MCP tools informs whether in-app chat is worth building
.docs/decisions/0075-conversational-interface-architecture.md:101:- **Platform-agnostic** — any MCP client can use the tools, not just Claude Desktop
.docs/decisions/0075-conversational-interface-architecture.md:104:- **MCP server implementation** — need to build and maintain MCP tool handlers
.docs/decisions/0100-glean-first-intelligence-architecture.md:6:**Context:** I559 validation spike confirmed Glean MCP `chat` tool produces structured JSON matching I508 schema with real cross-source data (Zendesk, REDACTED, Gong, Slack, org chart) in 10-30 seconds.
.docs/decisions/0100-glean-first-intelligence-architecture.md:14:The `chat` MCP tool replaces purpose-built Glean Agents (REST API not available on our instance) and can produce complete `IntelligenceJson`-compatible output in a single call, with data from sources DailyOS has zero local visibility into.
.docs/decisions/0100-glean-first-intelligence-architecture.md:24:1. The Glean MCP `chat` tool does multi-step reasoning, cross-source synthesis, and returns structured JSON on request
.docs/decisions/0100-glean-first-intelligence-architecture.md:51:  │         ├── Entity enrichment: chat MCP tool with I508 schema prompt
.docs/decisions/0100-glean-first-intelligence-architecture.md:52:  │         ├── Account discovery: chat MCP tool with user email
.docs/decisions/0100-glean-first-intelligence-architecture.md:53:  │         ├── People enrichment: chat MCP tool + search (people:)
.docs/decisions/0100-glean-first-intelligence-architecture.md:54:  │         ├── Gap filling: chat MCP tool with dimension-specific prompts
.docs/decisions/0100-glean-first-intelligence-architecture.md:151:  │     │     2. Call chat MCP tool (timeout: 60s)
.docs/decisions/0100-glean-first-intelligence-architecture.md:202:- Glean MCP API is not versioned — breaking changes possible

exec
/bin/zsh -lc 'rg -n "DOS-478|catalog conventions|catalog|when_to_call|tool description|Tool Catalog|account_status|daily_briefing" docs/solutions .docs/decisions src-tauri/resources/mcp_v2/tool_descriptions.yaml' in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc 'rg -n "ADR-0128|ADR 0128|0128|MCP product surface|product surface|Claude Desktop|MCP|account_status|daily_briefing" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/resources/mcp_v2/tool_descriptions.yaml:3:# Per ADR-0128 §3 + DOS-478 L0 packet §1: this catalog is the source of
src-tauri/resources/mcp_v2/tool_descriptions.yaml:6:# natural-language fields (summary, when_to_call, when_NOT_to_call) carry
src-tauri/resources/mcp_v2/tool_descriptions.yaml:16:- name: dailyos.read.account_status
src-tauri/resources/mcp_v2/tool_descriptions.yaml:19:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:31:    - dailyos.read.account_status
src-tauri/resources/mcp_v2/tool_descriptions.yaml:42:      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:46:      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:47:      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:54:- name: dailyos.read.daily_briefing
src-tauri/resources/mcp_v2/tool_descriptions.yaml:57:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:67:    - dailyos.read.daily_briefing
src-tauri/resources/mcp_v2/tool_descriptions.yaml:74:      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:78:      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:79:      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:89:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:121:      - { prompt: What's on my plate today overall?, expectedTool: dailyos.read.daily_briefing }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:126:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:154:      - { prompt: What's going on with Acme specifically?, expectedTool: dailyos.read.account_status }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:159:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:190:      - { prompt: What's the current state of the Acme account?, expectedTool: dailyos.read.account_status }
src-tauri/resources/mcp_v2/tool_descriptions.yaml:195:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:231:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:271:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:310:  when_to_call: |
src-tauri/resources/mcp_v2/tool_descriptions.yaml:354:  when_to_call: |
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:6:**Amended:** 2026-05-19 — same-date companion amendment to ADR-0102; codifies tool description as product copy (DOS-481 eval coverage), cross-conversation continuity wire shape (defers to ADR-0102 §D OpaqueConversationHandle lifecycle), restated v1.4.7 W4 write surface, displacement-use-case framing extended to v1.4.7 W5 host-selection eval, and Linear link refresh to the post-renumber v1.4.7 project. See section appended at end of doc.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:26:This framing has product consequences. Treating MCP as plumbing produces a generic data API. Treating MCP as product produces a tool surface as carefully designed as any UI in the app — and a tool description corpus that is itself the highest-leverage user-facing copy in the product, because every host-model conversation routes through it.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:51:Each tool description should:
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:136:The MCP surface succeeds at this question when the host model reaches for DailyOS *because the tool descriptions taught it to*, not because we blocked Glean. Each v1.5.0 tool description should be reviewable against this scenario.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:154:- v1.5.0 issue scoping has a positioning lens. *"Does this tool serve a question a user would ask?"* and *"Would this tool description steer a host model toward DailyOS over Glean for the right questions?"* become reviewable design questions.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:188:- [Anthropic — Model Context Protocol](https://modelcontextprotocol.io/) — tool description as model-facing affordance
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:200:Section 3 says "Tool descriptions are UI copy." v1.4.7 turns that posture into a versioned artifact and an eval gate. W1-A defines a typed `ToolDescription` struct in `services/mcp_v2/contracts.rs`; W1-B owns the tool-description YAML catalog at `src-tauri/resources/mcp_v2/tool_descriptions.yaml` as the version-controlled source.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:220:Every v1.4.7 tool description therefore ships with positive host-selection fixtures where the model should pick DailyOS for the right personal-working-context ask, and negative fixtures where the model should avoid DailyOS for either broad-corpus questions or adjacent-DailyOS-but-wrong-tool questions. The eval is a product-strategy artifact as much as a regression test.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:41:- An actor value is append-only to the catalog; once shipped, its shape does not change.
.docs/decisions/0120-observability-contract.md:202:- **Enumerated metric names.** A `const` list of permitted metric names in `observability::aggregate_metric_catalog`. New metrics require the same code review pressure as a new ADR amendment — can't slip in.
.docs/decisions/0115-signal-granularity-audit.md:39:Granularity is a property of the signal type, not the emitter. The signal catalog in `src-tauri/src/signals/types.rs` groups types by level for readability.
.docs/decisions/0115-signal-granularity-audit.md:232:- **Phase 0 (prerequisite work, lands before any policy registry):** Introduce `pub enum SignalType` in `src-tauri/src/signals/types.rs`. Initial variants mirror the existing string catalog found in signal-emitting code (`stakeholder_change`, `title_change`, `champion_risk`, `renewal_risk_escalation`, `engagement_warning`, `intelligence_confirmed`, `meeting_frequency_drop`, etc.). Plus the new claim-layer variants from [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (`ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimTrustChanged`, `ClaimContradiction`). The enum is `#[non_exhaustive]` for forward compatibility.
.docs/decisions/0102-abilities-as-runtime-contract.md:8:**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
.docs/decisions/0102-abilities-as-runtime-contract.md:278:- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
.docs/decisions/0102-abilities-as-runtime-contract.md:445:**Phase 1 (v1.4.0 planning window): Design and catalog.**  
.docs/decisions/0102-abilities-as-runtime-contract.md:446:[ADR-0103](0103-maintenance-ability-safety-constraints.md) through [ADR-0109](0109-migration-strategy.md) land as Proposed. Initial ability catalog drafted. Eval harness scaffolding in place per [ADR-0107](0107-evaluation-harness-for-abilities.md). No implementation churn yet.
.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/decisions/0102-abilities-as-runtime-contract.md:674:- Tool descriptions or the YAML catalog — W1-B (DOS-478).
.docs/decisions/0102-abilities-as-runtime-contract.md:681:ADR-0128 receives a same-date amendment recording the v1.4.7-specific consequences of this trust-boundary contract: tool description as product copy with eval coverage (DOS-481), cross-conversation continuity surfaced as a named affordance per ADR-0128 §6 with the wire-level shape defined here in §D, and extension of the displacement-use-case framing to v1.4.7 W5 host-selection evaluation. ADR-0128 §5 ("Feedback is the only write") narrows in v1.4.7's headless head: writes are restricted to the v1.4.7 W4 surface (note/observation, create_action, update_action_status), all routed through approved `services::*` per ADR-0101.

 succeeded in 0ms:
docs/solutions/README.md:37:- `integration-issues/` — cross-component, MCP, abilities, surface wiring
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:346:Codex flagged an inter-ADR conflict with [ADR-0111](0111-surface-independent-ability-invocation.md), which states the MCP bridge refuses confirmation fields. The `AgentWithAuthority` shape in this ADR creates a policy-bypass surface.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:46:- MCP tool is removed from hand-written registration; the registry-derived tool takes over.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:143:- **Stage 4 (cutover) rollback.** Revert the Tauri command wrapper and MCP tool registration to hand-written versions (retained as fallback). Old path runs again. User impact: if the new path was producing materially different output, users see the old output's characteristics return.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:33:- An MCP agent discovers `repair_graph_links` in the registry and invokes it with bad inputs, corrupting the graph.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:204:    Agent,      // MCP tool invocation
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:243:Agents (MCP) cannot invoke `global` maintenance under any circumstance, regardless of actor policy overrides. This is a hard floor enforced at the registry.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:379:A surface that has access only to `invoke_read` and `invoke_transform` cannot reach maintenance abilities even by name. Tauri commands for read/transform invocation do not expose a maintenance path. MCP tools for non-maintenance abilities route through the non-maintenance invokers. This is phased:
.docs/decisions/0027-mcp-dual-mode.md:1:# ADR-0027: MCP integration: dual-mode server + client
.docs/decisions/0027-mcp-dual-mode.md:12:The app is both an MCP server (exposes workspace tools/resources to Claude Desktop, agents, automation) and an MCP client (consumes external MCP services like Clay).
.docs/decisions/0027-mcp-dual-mode.md:18:**Interface parity (extends ADR-0025):** App, CLI, and MCP share the same registries (`~/.dailyos/`) and workspace.
.docs/decisions/0027-mcp-dual-mode.md:23:- Users in Claude Desktop can query DailyOS data without opening the app
.docs/decisions/0027-mcp-dual-mode.md:24:- MCP server binds to localhost only — no remote access
.docs/decisions/0027-mcp-dual-mode.md:26:- Phase 4 feature — but IPC commands (ADR-0025) should be designed to be MCP-exposable from the start
.docs/decisions/0043-meeting-intelligence-is-core.md:39:| CRM integration (Clay MCP) | Product-specific data source |
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:103:- `new_live` is the production constructor used by the app and the MCP server; callers supply real external clients.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:248:| MCP server | `Live` | Never (agents have no permission to escalate or de-escalate mode) |
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:348:5. **Provider and external-client wiring.** `ServiceContext` holds mode-aware external clients via `ExternalClients` (§2). `IntelligenceProvider` is held on `AbilityContext` rather than `ServiceContext` because multiple providers may co-exist (different abilities can use different providers), whereas external API clients are scoped to the service layer and share replay fixtures. `ServiceContext::new_simulate` and `new_evaluate` wire the mode-appropriate `ExternalClients` wrappers that route to replay fixtures. The surface (app, MCP, worker, harness) wires the mode-appropriate `IntelligenceProvider` into the `AbilityContext` it constructs. This split is intentional: externals are mode-routed by the service layer; provider selection is per-ability concern owned by the surface.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:1:# ADR-0128 — Headless DailyOS: MCP as a co-equal product surface
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:5:**Amended:** 2026-05-04 — added §7 (CLI as a third head, mediated by MCP)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:9:**Linear:** [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:13:[ADR-0027](0027-mcp-dual-mode.md) established DailyOS as both an MCP server (exposing workspace tools and resources to external agents) and an MCP client (consuming external services). The v1.5.0 project description is sharp on guarantees: tools must route through abilities, not parallel paths; writes must capture structured intent and provenance; reads must return current substrate state; claim-worthy outputs must explain source, freshness, confidence, and salience.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:19:The frame is straightforward: **MCP is not an export pipe; it is a co-equal head over the same intelligence substrate as the Tauri app.** The Tauri app and the MCP head are two different consumption optimizations of one claim graph.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:22:- The **MCP head is responsive**: whatever question came in, answer it well inside the host model's conversation. Designed for the on-demand ask.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:26:This framing has product consequences. Treating MCP as plumbing produces a generic data API. Treating MCP as product produces a tool surface as carefully designed as any UI in the app — and a tool description corpus that is itself the highest-leverage user-facing copy in the product, because every host-model conversation routes through it.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:32:The MCP server is the **headless head** of DailyOS. It exposes the same intelligence substrate as the Tauri app, optimized for a different consumption surface (host-model conversation rather than direct visual reading).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:36:The heads are co-equal *as consumption optimizations*, not feature-parity-mandated. The MCP head does not need to expose every Tauri view, and the Tauri app does not need to mimic conversational interaction. Each surface picks the questions and ritual it serves best.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:40:MCP tool design is governed by the questions a user would ask through their host model, not by the rows in the database.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:68:Consumption-first holds in the headless head. The MCP write surface is **feedback only**: corrections, dismissals, corroborations, contradictions, and tombstones, all flowing through the existing claim feedback path defined in [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:70:The MCP head does not expose:
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:76:A user in Claude Desktop can correct what DailyOS knows. They cannot use DailyOS as a write layer. Drafting and sending happen in the tool that's good at writing, with DailyOS context piped in.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:82:Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:86:### 7. CLI as a third head, mediated by MCP
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:90:**Transport.** CLI invocations route through the MCP server. There is no parallel CLI ingress to the abilities runtime. The default is MCP-mediated; a CLI-distinct transport is only justified if something genuinely cannot be expressed through the MCP surface, and that decision is deferred until the need is concrete. Keeping a single ingress to the abilities runtime is easier to reason about, easier to evolve, and prevents the v1.5.0 mission gate ("don't bypass guarantees") from being silently undermined by a second path.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:95:- MCP: substrate → claim-shaped JSON
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:102:- **CLI-for-agent.** A Claude Code skill, or any agent that shells out, invokes the CLI as part of a workflow. Skills are *macros over the MCP surface* — named, slash-invokable bundles of MCP calls with workflow shape. The agent is the consumer; the user is one step removed. Structurally similar to MCP-via-host-model.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:112:These predate the abilities-first MCP surface. They should be refreshed once v1.5.0 lands so they consume the new tool surface and inherit its descriptions, output shape, and feedback semantics. This is a v1.5.x follow-on, not v1.5.0 substrate work — but it is the natural beneficiary of v1.5.0 once the MCP head is solid.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:118:A user is in Claude Desktop with both a Glean MCP and a DailyOS MCP connected. They ask: *"What's going on with Acme Corp?"*
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:136:The MCP surface succeeds at this question when the host model reaches for DailyOS *because the tool descriptions taught it to*, not because we blocked Glean. Each v1.5.0 tool description should be reviewable against this scenario.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:162:- "Headless" is metaphorical. Some contributors may read it as "MCP must implement every Tauri surface," which is not the intent. §1 must be explicit about co-equal-as-consumption-optimization, not feature-parity.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:164:- The displacement framing assumes a host model with multiple MCPs connected. For users with only DailyOS connected, the comparison is moot but the descriptions still teach the host model how to reason about coverage.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:165:- Feedback-only writes constrain integration depth. A user who wants Claude Desktop to "create a Linear ticket" via DailyOS will be redirected — that mutation goes through Linear's MCP or Claude Desktop's own tools, not ours. We accept this; the alternative is becoming a write layer, which the v1.5.0 project explicitly is not.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:177:- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:184:- [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:192:This amendment records the v1.4.7 operational consequences of the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment. It does not reopen the architecture. ADR-0128 remains the product-surface frame for headless DailyOS; ADR-0102 remains the runtime contract for MCP-originated ability invocation.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:196:The top-of-file `Linear:` citation still points to the historical v1.5.0 project. Per the 2026-05-17 renumber, the canonical project for this work is now v1.4.7. The historical header is left intact for traceability; the post-renumber canonical project is [v1.4.7 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:202:The host-selection eval in DOS-481, scheduled for v1.4.7 W5-A, is the product-copy acceptance test. Each tool ships with at least two positive fixtures and at least two negative fixtures: one broad-corpus negative where DailyOS should yield to enterprise or web search, and one adjacent-DailyOS-but-wrong-tool negative where another DailyOS tool is the right choice. The `when_NOT_to_call` field is mandatory and explicit because avoiding DailyOS for the wrong ask is part of the product surface, not merely an eval trick.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:214:All three writes route through approved `services::*` boundaries per ADR-0101. Notes and observations flow through the claim/source services; actions and action-status updates flow through `services::actions`. The MCP head still does not expose claim creation or direct edit, file generation, calendar mutation, message composition, or external-system writes. Submit-class corrections and lightweight action tracking are the only mutations.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:224:The MCP client trust boundary is codified in the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment, especially §C: MCP client authentication, transport HMAC signing, server-side scope manifest authorization, per-actor × per-client × per-tool rate limits, revocation, and audit attribution with keyed HMAC parameter and response hashes. This ADR cites that contract because ADR-0128 is the product-surface frame, not the transport or authorization spec.
.docs/decisions/0079-role-presets.md:26:6. **The real differentiator between roles is vocabulary, not architecture.** "Churn risk" vs "deal stalled" vs "scope creep" vs "blocked by dependency" — same Watch List component, different words. The AI prompts are the product surface that matters most per role, not the UI layout or data model.
.docs/decisions/0046-entity-mode-architecture.md:57:Integrations are MCP data source connectors (ADR-0027). Any integration works with any entity mode.
.docs/decisions/0046-entity-mode-architecture.md:66:Each integration is an MCP server. The app is an MCP client. Community can build integrations without touching core code.
.docs/decisions/0046-entity-mode-architecture.md:177:- Community can build MCP integrations without touching core.
.docs/decisions/0046-entity-mode-architecture.md:185:- MCP client infrastructure must be production-ready before integrations work.
.docs/decisions/0046-entity-mode-architecture.md:193:- Chose MCP for integrations over custom protocol — leverages ecosystem, community-contributable, already decided in ADR-0027.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:472:- **MCP surface** — respects subject-ownership enforcement (DOS-288); signals do not leak across subject ownership; MCP-bridge re-tests in W4 merge gate
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:14:v1.4.3 W6 needed substrate work on 3 tickets (DOS-742 HMAC signing, DOS-743 MCP invoke audit migrate, DOS-576 11-site migration) plus DOS-741 done manually first. Per memory `feedback_parallel_agent_sizing`, mechanical work + tight rule + non-overlapping files → fan out parallel codex agents.
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:21:| DOS-743 (MCP migrate) | `/private/tmp/dailyos-743` | `surface_runtime/mod.rs` 3-branch migration | **0** |
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:145:`open_readonly()` (used by the MCP subprocess) also requires the key. The MCP subprocess runs as the same macOS user, so Keychain access is available. No architectural change needed.
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:315:- `open_readonly()` (MCP subprocess) requires the same key from Keychain. The subprocess runs as the same user -- Keychain access is available. No architectural change.
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:54:  - DOS-743: MCP invoke audit threads `request_id` via `SurfacePairingAuditEvent.request_id`
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:14:1. [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a typed, versioned ability. Many of those are Transform abilities that produce claims. The product will ship with multiple ability authors — `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative`, future MCP agents — each asserting claims into the same substrate.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:51:| `committed` | Claim is authoritative. Live in briefings, MCP, chat. | Included in default reads. |
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:90:Claims that do not pass the gate transition to `proposed` and become visible in the Analysis Inbox (v1.5.0 surface). They do not appear in briefings, MCP responses, or any default-read surface.
.docs/decisions/0100-glean-first-intelligence-architecture.md:6:**Context:** I559 validation spike confirmed Glean MCP `chat` tool produces structured JSON matching I508 schema with real cross-source data (Zendesk, REDACTED, Gong, Slack, org chart) in 10-30 seconds.
.docs/decisions/0100-glean-first-intelligence-architecture.md:14:The `chat` MCP tool replaces purpose-built Glean Agents (REST API not available on our instance) and can produce complete `IntelligenceJson`-compatible output in a single call, with data from sources DailyOS has zero local visibility into.
.docs/decisions/0100-glean-first-intelligence-architecture.md:24:1. The Glean MCP `chat` tool does multi-step reasoning, cross-source synthesis, and returns structured JSON on request
.docs/decisions/0100-glean-first-intelligence-architecture.md:51:  │         ├── Entity enrichment: chat MCP tool with I508 schema prompt
.docs/decisions/0100-glean-first-intelligence-architecture.md:52:  │         ├── Account discovery: chat MCP tool with user email
.docs/decisions/0100-glean-first-intelligence-architecture.md:53:  │         ├── People enrichment: chat MCP tool + search (people:)
.docs/decisions/0100-glean-first-intelligence-architecture.md:54:  │         ├── Gap filling: chat MCP tool with dimension-specific prompts
.docs/decisions/0100-glean-first-intelligence-architecture.md:151:  │     │     2. Call chat MCP tool (timeout: 60s)
.docs/decisions/0100-glean-first-intelligence-architecture.md:202:- Glean MCP API is not versioned — breaking changes possible
.docs/decisions/0096-glean-mode-local-footprint.md:35:| Clay enrichment metadata | Clay MCP | Clay disabled in both strategies |
.docs/decisions/0108-provenance-rendering-and-privacy.md:13:1. **Rendering rules per surface** — what the Tauri app, MCP, P2 publications, and logs show.
.docs/decisions/0108-provenance-rendering-and-privacy.md:31:**MCP server (agent-facing).**
.docs/decisions/0108-provenance-rendering-and-privacy.md:34:- Actor-filtered contents: the MCP wrapper strips content from provenance that is not permitted for `Actor::Agent`:
.docs/decisions/0108-provenance-rendering-and-privacy.md:39:- An MCP tool `get_provenance(invocation_id)` provides a fuller view for authorized agents if needed; default response is summary-sized.
.docs/decisions/0108-provenance-rendering-and-privacy.md:70:The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.
.docs/decisions/0108-provenance-rendering-and-privacy.md:117:- **MCP default response:** provenance payload ≤ 10KB. Full detail available via `get_provenance(invocation_id)`.
.docs/decisions/0108-provenance-rendering-and-privacy.md:129:- MCP tool responses
.docs/decisions/0108-provenance-rendering-and-privacy.md:145:1. **Rendering rules are surface-specific.** The Tauri app gets rich detail; MCP gets actor-filtered; P2 gets heavy redaction; logs get operational metadata. No one-size-fits-all.
.docs/decisions/0108-provenance-rendering-and-privacy.md:156:3. **MCP `get_provenance(invocation_id)` tool adds surface.** Agents can request detail they did not receive by default.
.docs/decisions/0108-provenance-rendering-and-privacy.md:162:2. **Renderer drift across surfaces.** The Tauri app diverges from MCP in how it interprets provenance. Mitigation: `render_provenance_for` is the single entry point; surfaces cannot bypass it.
.docs/decisions/0108-provenance-rendering-and-privacy.md:164:4. **P2 publication leaks through "Details" toggle.** User opts in to full detail and leaks internal IDs. Mitigation: publication "Details" renders through the same actor filter as MCP when the P2 audience includes non-team members; team-internal publishes get full detail.
.docs/decisions/0116-tenant-control-plane-boundary.md:14:**Precise wording required — 2026-04-20 update (outside voice finding #2).** "Stays on your laptop" is not accurate without qualification when Glean is the intelligence provider (per [ADR-0100](0100-glean-first-intelligence-architecture.md)). Glean's MCP chat tool receives prompts, returns completions, and sees the user's Glean-connected source data (REDACTED, Zendesk, Gong, Slack) in the Glean infrastructure the customer has already contracted with. That is not DailyOS control-plane — it is the user's pre-existing relationship with Glean — but it's also not "on the laptop."
.docs/decisions/0059-entity-directory-template.md:14:2. **New accounts from the app had no scaffold** — external tools (Claude Desktop, ChatGPT, CLI tools) browsing an entity directory saw no organizational structure.
.docs/decisions/0047-entity-dashboard-architecture.md:12:2. **The ecosystem** — Claude Desktop, Claude Code, ChatGPT, and any future AI tool pointed at the workspace directory needs rich, queryable, AI-consumable files
.docs/decisions/0047-entity-dashboard-architecture.md:22:- **Markdown must be rich.** An account dashboard read by Claude Desktop should contain company overview, stakeholder map, strategic programs, recent wins, open actions, engagement signals — everything needed to answer complex questions about the account.
.docs/decisions/0047-entity-dashboard-architecture.md:24:- **External tools must be able to write.** Claude Desktop asked to "update Acme's strategy section" should be able to do so without understanding the app's internals.
.docs/decisions/0047-entity-dashboard-architecture.md:149:1. External tool writes to `dashboard.json` (e.g., Claude Desktop updates strategy notes)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:5:**Amended:** 2026-05-10 — §4 reframed (WP path uses WP Abilities API + MCP Adapter, not PHP MCP client) per Codex L0 Prep follow-up on DOS-546. Primary-source research confirmed Abilities API in WordPress core since 6.9 (December 2025); MCP Adapter ships at `github.com/WordPress/mcp-adapter`.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:7:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md), [ADR-0127](0127-presets-as-intelligence-contracts.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:8:**Linear:** [v1.4.1 — Abilities Runtime Completion](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7), [v1.4.2 — Entity Intelligence](https://linear.app/a8c/project/v142-entity-intelligence-accounts-projects-people-a448d2e072ad), [v1.4.7 — MCP Server v2](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9), [v1.4.9 — Self-Healing v2](https://linear.app/a8c/project/v149-self-healing-v2-da4bc5769164), [DOS-540](https://linear.app/a8c/issue/DOS-540)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:12:[ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) established the architectural frame: *the substrate is the product; the heads are surfaces over it.* It named two heads — the Tauri app (visual, opinionated, daily ritual) and the MCP server (responsive, headless, on-demand) — and explicitly anticipated more: *"Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern."*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:19:- v1.4.2 onward (Entity Intelligence, Briefing Experience, Claim/Trust UI, Salience, Workspace Memory, MCP Server v2, Reports, Self-Healing) is about to begin significant interface design and implementation work — the "redesign tokens" of the program.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:21:- The MCP head exists in v1.4.7's Linear scope as a late-program deliverable.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:25:- **MCP is now genuinely cross-vendor.** Spec 2025-11-25 with OAuth 2.1, Step-Up Auth, Async Tasks. OpenAI and Google ship MCP clients in production; the standard is no longer Anthropic-led.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:38:### Why this ADR (vs. ADR-0128's existing framing)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:40:ADR-0128 framed the architecture as surface-pluggable. It treated the Tauri app as the primary visual surface and MCP as a co-equal headless head. This ADR commits to the next step in that architecture: **WordPress Studio is added as a new head and is promoted to primary**, with the Tauri app reorienting toward runtime-host duties and the MCP head elevating from a late-program feature to the foundation contract every v1.4.2+ wave consumes.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:42:This ADR does not contradict ADR-0128. It is the operationalization of ADR-0128's architectural thesis at the moment the substrate has matured enough to make pluggable surfaces real.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:50:This generalizes ADR-0128 §1 from *"the substrate is the product; the heads are surfaces over it"* to: *"the runtime is the product. The substrate is its memory; abilities are its capability surface; surfaces are how users compose against it."*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:71:This ADR reorders the program: the substrate-to-surface contract (currently scoped inside v1.4.7) is **promoted to a foundation wave between v1.4.1 and v1.4.2**. v1.4.7's MCP server work continues as the first external implementation of the contract; the contract itself ships earlier so that v1.4.2 entity surfaces, v1.4.3 briefing surfaces, v1.4.4 claim/trust surfaces, and v1.4.5 salience surfaces all consume the same substrate-to-surface API.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:73:### 4. WordPress accesses the substrate via its own Abilities API + MCP Adapter; non-WP surfaces use the Rust runtime's MCP server directly
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:75:*Reframed 2026-05-10 per Codex L0 Prep follow-up on DOS-546. The original framing — "WordPress plugin opens an MCP session to the localhost-bound runtime" — was based on a PHP MCP client architecture that primary-source research showed isn't necessary and isn't the canonical WP integration target.*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:79:- **WordPress path.** The DailyOS WP plugin registers abilities into [WordPress's Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) (in WordPress core since 6.9, December 2025; client-side API merged into Gutenberg for 7.0, shipping 2026-05-20). Those abilities delegate to the Rust runtime over a local transport (loopback HTTP REST as primary candidate; Unix domain socket as fallback; spawned stdio as last-resort). The [WordPress MCP Adapter plugin](https://github.com/WordPress/mcp-adapter) exposes registered abilities as MCP tools automatically — WordPress itself becomes an MCP server consumable by Claude Desktop, Cursor, VS Code, and any other MCP client connected to the WP site. Gutenberg blocks consume the abilities through WP-native infrastructure (REST endpoints, capabilities, nonces, and the WP 7.0 client-side `executeAbility()`).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:80:- **Non-WP path.** The Rust runtime exposes its own MCP server for surfaces that don't sit inside WordPress (Claude Desktop direct without a WP install, Cursor, future PWA, future mobile). Same substrate, two MCP servers — one WP-mediated for WP-consuming clients, one direct for everything else.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:82:DailyOS does **not** consume MCP from PHP. The seam between WordPress and the runtime is a local PHP-to-Rust transport, not an MCP client; the WP Abilities API + MCP Adapter does the MCP server work on the WP side.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:84:This preserves [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §7's discipline (single ingress to the abilities runtime) — every external invocation, whether routed through the WP MCP server or the runtime's direct MCP server, ultimately resolves to one substrate access path inside the runtime. The runtime exposes one substrate access protocol; clients vary in how they render and compose against it.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:86:**MCP Adapter exposure policy.** Substrate-backed abilities must NOT be exposed by the *default* WP MCP server. DailyOS ships a custom MCP server configuration with an explicit ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. Permission callbacks check both WP capabilities and DailyOS `SurfaceClient(WordPress)` scopes (per [ADR-0111](0111-surface-independent-ability-invocation.md)); site-wide WP capabilities alone are not the isolation boundary.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:113:- **Runtime host.** Rust services, cron, the MCP server, local privacy gate, abilities execution, signal propagation, claim writes. This is what Tauri does well and continues to do.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:116:The Rust substrate engine, abilities runtime, signal propagation, claim feedback path, and MCP server all continue exactly as v1.4.0/v1.4.1 designed them. None of that work is wasted. The investment has always been in the runtime; this ADR makes that explicit.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:122:ACP (Zed's Agent Client Protocol) is the leading candidate for the protocol implementation. The architectural commitment is to the **shape** (out-of-process agent, session-shaped invocation, MCP-shaped tool surface), not to the specific protocol. Implement ACP first as the leading candidate; leave room for a second protocol if a different one matures faster.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:124:This composes with §4: agents reach the substrate via MCP; surfaces reach the substrate via MCP; agent backend is pluggable; surface is pluggable; runtime is the constant.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:131:- **(New foundation wave between v1.4.1 and v1.4.2)** — substrate-to-surface contract. Formerly part of v1.4.7's scope. Typed read/write/subscribe APIs, composition primitives, identity bridge. MCP server is the first external implementation.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:137:- **v1.4.7** — MCP Server v2. Repurposed: the contract foundation has shipped earlier; v1.4.7 now focuses on the rich tool surface, displacement-test discipline (per ADR-0128), and the cross-surface story.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:156:- **Not abandoning the Tauri runtime.** The Rust substrate, abilities runtime, signal propagation, MCP server, cron, and local privacy gate continue exactly as v1.4.0/v1.4.1 design. The runtime is the product.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:157:- **Not requiring WordPress to consume DailyOS.** Headless surfaces (Claude Desktop via MCP, Cursor, future) work without WordPress. WordPress is the *primary* composable surface, not the *only* surface.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:161:- **Not deprecating ADR-0128.** This ADR extends ADR-0128's surface-pluggable thesis with a concrete primary-surface choice. The headless / co-equal-heads frame from ADR-0128 stays canonical.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:171:- **Cross-surface story becomes coherent.** WordPress for primary composition; Claude Desktop and future agent shells for headless conversation; substrate sovereign. Each surface picks the questions and ritual it serves best, per ADR-0128 §1.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:184:- **MCP and ACP maturity dependencies.** MCP is mature and cross-vendor; ACP is younger. Section 8 mitigates by committing to the *shape* not the protocol.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:189:- **ADR-0128 is preserved and extended.** The surface-pluggable thesis stays canonical; this ADR commits to the next step.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:197:- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:204:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS: MCP as a co-equal product surface (this ADR extends)
.docs/decisions/0107-source-taxonomy-alignment.md:261:v1.4.5 Workspace Memory introduces ingestion of local workspace files into the claim store. Per ADR-0107 §1, adding a new `DataSource` variant requires an amendment, not the `Other` escape hatch. This amendment defines the typed variant plus its scoring/lifecycle posture so v1.4.5 W1+ lanes can freeze their lifecycle records and W4-C MCP placement can build against a stable contract. Triggered by v1.4.5 W0 close gate per `.docs/plans/v1.4.5-waves.md` §'Cycle 2 amendments' #4 and §'W0 close gate'.
.docs/decisions/0107-source-taxonomy-alignment.md:291:| `McpPlacement` | files placed by an AI agent via the MCP `workspace_place_document` ability |
.docs/decisions/0107-source-taxonomy-alignment.md:322:- v1.4.5 W4-C DOS-474: MCP `place_document` ability writes claims with `kind=McpPlacement`.
.docs/decisions/0089-user-entity-and-professional-context.md:118:The user entity is personal professional context. It is never shared, never exposed via the MCP server to external tools, and never included in any outbound integration. The MCP sidecar's four read-only tools (`get_briefing`, `query_entity`, `list_entities`, `search_meetings`) do not expose user entity fields — they are for workspace intelligence, not the user's own professional context.
.docs/decisions/0075-conversational-interface-architecture.md:16:- ADR-0027: MCP dual-mode — already establishes DailyOS as both MCP server (expose tools) and client (consume external data). Chat via MCP tools is a natural extension of the server role.
.docs/decisions/0075-conversational-interface-architecture.md:22:### Phase 1: External via MCP (post-ship)
.docs/decisions/0075-conversational-interface-architecture.md:24:Claude Desktop (or any MCP-compatible client) calls DailyOS MCP tools to query entities. No in-app chat UI needed.
.docs/decisions/0075-conversational-interface-architecture.md:26:**MCP tools to expose:**
.docs/decisions/0075-conversational-interface-architecture.md:34:- Zero UI investment — Claude Desktop already has a polished chat interface
.docs/decisions/0075-conversational-interface-architecture.md:35:- Validates demand — if users actually use MCP tools, we have evidence for in-app investment
.docs/decisions/0075-conversational-interface-architecture.md:36:- Aligns with ADR-0027 — MCP server mode is already planned
.docs/decisions/0075-conversational-interface-architecture.md:87:Only build if Phase 1 MCP tools see significant usage. In-app chat would be:
.docs/decisions/0075-conversational-interface-architecture.md:97:- **Zero UI investment for Phase 1** — MCP tools are backend-only, exposed via existing Tauri command pattern
.docs/decisions/0075-conversational-interface-architecture.md:98:- **Validates demand** — usage data from MCP tools informs whether in-app chat is worth building
.docs/decisions/0075-conversational-interface-architecture.md:101:- **Platform-agnostic** — any MCP client can use the tools, not just Claude Desktop
.docs/decisions/0075-conversational-interface-architecture.md:104:- **MCP server implementation** — need to build and maintain MCP tool handlers
.docs/decisions/0111-surface-independent-ability-invocation.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:14:This ADR specifies the exact binding pattern for Tauri commands and MCP tools, how abilities are generated into both surfaces from the registry, and how surfaces construct `AbilityContext` with mode, actor, and provenance renderer wiring.
.docs/decisions/0111-surface-independent-ability-invocation.md:25:| MCP server | `Agent` | `Live` (never escalated) | `McpAbilityBridge` |
.docs/decisions/0111-surface-independent-ability-invocation.md:72:### 3. MCP Tool Registration
.docs/decisions/0111-surface-independent-ability-invocation.md:74:The MCP server iterates the registry at startup and registers one MCP tool per ability whose `AbilityPolicy.allowed_actors` includes `Agent`:
.docs/decisions/0111-surface-independent-ability-invocation.md:96:8. Returns the MCP response with domain output and filtered provenance.
.docs/decisions/0111-surface-independent-ability-invocation.md:98:**No hand-written MCP tool handlers.** Every MCP tool is an ability. Pre-existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) that do not correspond to an ability are either migrated to an ability or removed during Phase 3 cutover.
.docs/decisions/0111-surface-independent-ability-invocation.md:102:MCP tool discovery (the `list_tools` endpoint) is filtered by actor. An agent enumerating the server's tools sees only abilities whose policy permits `Agent`. Maintenance abilities, admin-only abilities, and abilities that specifically exclude agents never appear in the list.
.docs/decisions/0111-surface-independent-ability-invocation.md:157:- MCP server registers ability-derived tools alongside existing hand-written tools (both paths active).
.docs/decisions/0111-surface-independent-ability-invocation.md:158:- App and MCP progressively switch from hand-written to registry-backed.
.docs/decisions/0111-surface-independent-ability-invocation.md:163:- Hand-written MCP tools removed; all MCP tools come from the registry.
.docs/decisions/0111-surface-independent-ability-invocation.md:173:The original three actor classes (`User`, `Agent`, `System`) were sufficient when DailyOS's surfaces were exclusively first-party: Tauri app for `User`, MCP for `Agent`, background workers / eval / tests for `System`. The decision in [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) to support third-party local surfaces — initially WordPress, generalized to any local CMS, editor, extension, or paired app that mediates user interactions with the substrate — introduces an actor that does not fit those three.
.docs/decisions/0111-surface-independent-ability-invocation.md:194:- **Write authority.** Feedback-only per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5. No direct claim creation, no source-reliability mutation outside feedback events, no tombstoning of others' claims. Raw editor-side content diffs (e.g., a Gutenberg block save diff) never become substrate writes.
.docs/decisions/0111-surface-independent-ability-invocation.md:222:Registry enumeration for `SurfaceClient` filters at two levels: the actor class (is `SurfaceClient` in the ability's `allowed_actors`?) AND the per-instance scope grants (does this specific instance have the scope required for this ability?). Both must pass. Abilities NOT permitted for `SurfaceClient` invocation never appear in any SurfaceClient's tool surface, even if they appear for `User`. Abilities with `mcp_exposure: McpExposure::None` (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1 + §7.6) are not enumerated for SurfaceClient-mediated MCP servers regardless of permission state; abilities with `mcp_exposure: McpExposure::MetadataOnly` are enumerated with name + description only (no input/output schema); no invocation handler is registered. Full schema is reserved for `McpExposure::Invocable`. The orthogonal `client_side_executable: bool` field on `AbilityPolicy` separately gates the WordPress 7.0 client-side `executeAbility()` JS path; an ability fully `Invocable` via MCP may still be `client_side_executable: false`, and vice versa, per Phase 0 artifact 05 lines 383-412.
.docs/decisions/0111-surface-independent-ability-invocation.md:239:1. **Surface parity is structural.** App, MCP, worker, eval harness all invoke abilities through the same registry path; output quality is identical across surfaces.
.docs/decisions/0111-surface-independent-ability-invocation.md:240:2. **MCP tool registration is automatic.** Adding a new Agent-accessible ability adds a new MCP tool with no hand-written code.
.docs/decisions/0111-surface-independent-ability-invocation.md:249:2. **MCP migration is non-trivial.** Existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) must be refactored into abilities or retired.
.docs/decisions/0111-surface-independent-ability-invocation.md:250:3. **Bridge code is mostly boilerplate.** Four bridges (Tauri, MCP, worker, eval) with similar shape. Potential to factor but not in v1.4.0.
.docs/decisions/0111-surface-independent-ability-invocation.md:254:1. **Hand-written MCP tool drift.** A contributor adds a hand-written MCP tool in Phase 2 that never gets migrated. Mitigation: Phase 3 gate requires zero hand-written MCP tools; CI enforces.
.docs/decisions/0111-surface-independent-ability-invocation.md:256:3. **Confirmation token misuse in MCP.** Agents attempting to supply confirmation tokens via MCP. Mitigation: MCP bridge refuses `confirmation` field; tokens are user-initiated via Tauri only.
.docs/decisions/0111-surface-independent-ability-invocation.md:258:5. **List-tools API bloat.** If many abilities are Agent-accessible, the MCP tool list becomes large. Mitigation: actor filtering already narrows scope; categorization lets agents discover by category.
.docs/decisions/0111-surface-independent-ability-invocation.md:263:- [ADR-0027: MCP Dual-Mode](0027-mcp-dual-mode.md) — Existing MCP sidecar; Phase 3 retires its hand-written tools in favor of registry-derived ones.
.docs/decisions/0102-abilities-as-runtime-contract.md:7:**Amended (later same day):** 2026-05-10 — promoted `mcp_exposure` from `bool` to tri-state `McpExposure { None | MetadataOnly | Invocable }` enum per Phase 0 artifact 05 lines 383-412; retained `client_side_executable: bool` as an orthogonal separate field for the WordPress 7.0 client-side `executeAbility()` JS path. The two fields govern different trust boundaries — `mcp_exposure` controls the network-facing MCP tool surface (discovery + invocation), `client_side_executable` controls whether a trusted in-product SurfaceClient may invoke the ability after WordPress capability + runtime scope + actor checks pass. The fields are independent; either may be true while the other is false. Default policy is `mcp_exposure = McpExposure::None` + `client_side_executable = false`, opt-in per ability. This cycle-3 reversal of the cycle-2 collapse was made after re-reading artifact 05's explicit warning against the symmetric `client_side_exposure` framing.  
.docs/decisions/0102-abilities-as-runtime-contract.md:8:**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
.docs/decisions/0102-abilities-as-runtime-contract.md:17:Depth of this kind cannot be assembled on demand, feature by feature. It has to be maintained continuously and composed consistently. Today, every feature in the product assembles context in its own command handler: meeting prep queries one set of tables, daily readiness queries another, the MCP sidecar queries a third. Context builders (`build_intelligence_context`, `gather_account_context`) are shared helpers but not the single invocation path. The result:
.docs/decisions/0102-abilities-as-runtime-contract.md:20:- Consumers receive subtly different context depending on how they entered the system (app vs. MCP vs. background worker).
.docs/decisions/0102-abilities-as-runtime-contract.md:30:Every product capability in DailyOS is implemented as an **ability**: a named, typed, versioned function that synthesizes state and signals into a structured output. All consumers — the desktop app, the MCP server, background workers, and any future surface — invoke capabilities exclusively through the abilities layer. The abilities layer is the runtime contract of the product.
.docs/decisions/0102-abilities-as-runtime-contract.md:182:- Inputs are versioned via `SchemaVersion` (newtype wrapping `u32`). External callers (MCP, future REST) MUST declare `schema_version`; no implicit latest. Internal callers (Tauri commands) may default to current since they rebuild with the ability. Versioning rules in §8.
.docs/decisions/0102-abilities-as-runtime-contract.md:183:- Outputs are JSON-serializable via `serde` and expose a JSON Schema via `schemars` for external introspection. No trait objects, no opaque handles. This makes Tauri bridge, MCP tool registration, and docs generation all derive from the same schema source.
.docs/decisions/0102-abilities-as-runtime-contract.md:225:    pub mcp_exposure: McpExposure,              // Tri-state exposure tier for MCP — both the Rust runtime's MCP server AND any
.docs/decisions/0102-abilities-as-runtime-contract.md:226:                                                  // SurfaceClient-mediated MCP server (e.g., WordPress MCP Adapter per ADR-0129).
.docs/decisions/0102-abilities-as-runtime-contract.md:237:    None,                                       // Ability not surfaced via MCP at all. Filtered from MCP discovery, introspection,
.docs/decisions/0102-abilities-as-runtime-contract.md:239:    MetadataOnly,                               // Ability appears in MCP discovery with name + description ONLY (no input/output
.docs/decisions/0102-abilities-as-runtime-contract.md:240:                                                  // schema, no blast-radius hints) and is not invocable via MCP. A client receives
.docs/decisions/0102-abilities-as-runtime-contract.md:244:    Invocable,                                  // Ability fully exposed via MCP, including name + description + input/output schema
.docs/decisions/0102-abilities-as-runtime-contract.md:270:- **Erased invocation** (used by external consumers — Tauri bridge, MCP, future REST): goes through the registry wrapper with JSON boundaries. Validated against schema on entry and exit.
.docs/decisions/0102-abilities-as-runtime-contract.md:278:- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
.docs/decisions/0102-abilities-as-runtime-contract.md:285:Registry enumeration is actor-scoped. An MCP client enumerating available tools sees only abilities whose `AbilityPolicy.allowed_actors` includes `Agent` AND whose `mcp_exposure` is `McpExposure::MetadataOnly` or `McpExposure::Invocable`; abilities with `mcp_exposure: None` are filtered. `MetadataOnly` abilities are enumerated with name + description only — the input/output schema and any blast-radius hints are withheld until promotion to `Invocable`; their invocation handler is not registered with the MCP server. `Invocable` abilities are enumerated with full schema and an invocation handler. Introspection does not leak ability names, input schemas, or blast radius to callers that are not authorized to invoke.
.docs/decisions/0102-abilities-as-runtime-contract.md:287:The Tauri bridge, invoked by the first-party app, sees the full registry subject to `User` and `System` filtering; the `mcp_exposure` and `client_side_executable` fields do not gate the first-party Tauri bridge because that bridge is the in-process path for the desktop app, not an MCP-mediated or SurfaceClient-mediated surface. Background workers see the `System` filter.
.docs/decisions/0102-abilities-as-runtime-contract.md:291:Surfaces (Tauri commands, MCP handlers, background workers) invoke abilities through the registry or through typed imports as described in §7.2. They do not construct `AbilityContext` directly and do not call ability functions outside these two paths. Phase 4 enforces this by module visibility.
.docs/decisions/0102-abilities-as-runtime-contract.md:301:3. If the invocation arrives via an MCP-mediated path (the Rust runtime's MCP server OR a SurfaceClient-mediated MCP server per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §4), the ability's `mcp_exposure` is `McpExposure::Invocable`. If `mcp_exposure` is `McpExposure::None`, MCP-mediated discovery AND invocation are rejected at the registry boundary. If `mcp_exposure` is `McpExposure::MetadataOnly`, MCP-mediated discovery enumerates name + description only (no input/output schema), and MCP-mediated invocation is rejected at the registry boundary. The tier check happens regardless of `allowed_actors`.
.docs/decisions/0102-abilities-as-runtime-contract.md:302:4. If the invocation arrives via a client-side SurfaceClient JS path (e.g., a WordPress 7.0 block invoking `executeAbility()`), the ability's `client_side_executable` is `true`. If `client_side_executable` is `false`, the SurfaceClient bridge rejects the invocation before registry lookup regardless of `allowed_actors`, `required_scopes`, or `mcp_exposure`. This check is orthogonal to the MCP-mediated tier check — the two fields govern different trust boundaries (per Phase 0 artifact 05 lines 383-412).
.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:308:- A SurfaceClient enumerating available abilities via an MCP-mediated path sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (c) has `mcp_exposure` of `MetadataOnly` or `Invocable`. Abilities with `mcp_exposure: None` are not enumerated. Abilities with `mcp_exposure: MetadataOnly` are enumerated with name + description only (no input/output schema); the invocation handler is not registered.
.docs/decisions/0102-abilities-as-runtime-contract.md:309:- A SurfaceClient enumerating available abilities via a client-side JS path (e.g., WordPress 7.0 `executeAbility()` enumeration) sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (d) has `client_side_executable: true`. Abilities with `client_side_executable: false` are not exposed to the client-side enumeration path even when fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:310:- Abilities failing the applicable filter ((a)/(b)/(c) for MCP-mediated paths; (a)/(b)/(d) for client-side JS paths) are not enumerated regardless of caller permission.
.docs/decisions/0102-abilities-as-runtime-contract.md:312:- A SurfaceClient does NOT see ability names, schemas, or blast radius for abilities outside its scope grants, for `mcp_exposure: None` abilities on MCP-mediated paths, or for `client_side_executable: false` abilities on client-side JS paths. Unauthorized introspection is closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:314:**Default policy.** Abilities default to `allowed_actors = [User]`, `required_scopes = vec![]`, `mcp_exposure = McpExposure::None`, `client_side_executable = false`. `SurfaceClient` exposure is opt-in per ability — the macro attribute must declare `allowed_actors` including `SurfaceClient`, plus `required_scopes` AND `mcp_exposure` AND `client_side_executable` explicitly (or `no_scope_required` in place of an empty `required_scopes` when the absence is intentional; see §7.1 macro). Most existing abilities do NOT need to be exposed to SurfaceClient instances; ability authors must consciously opt each one in. The two exposure fields govern different trust boundaries (MCP tool surface vs. client-side JS invocation) per Phase 0 artifact 05 lines 383-412 — either may be opted in while the other remains closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:342:External callers (MCP, REST) MUST include `schema_version` in every invocation. The registry rejects calls without it. This eliminates the "implicit latest" footgun where a breaking change silently returns a different shape to agents.
.docs/decisions/0102-abilities-as-runtime-contract.md:352:Tauri commands, MCP handlers, and future surfaces either invoke abilities through the registry (erased, validated, schema-checked) or through typed imports as described in §7.2. They do not construct `AbilityContext` themselves and do not invoke ability internals.
.docs/decisions/0102-abilities-as-runtime-contract.md:449:`prepare_meeting`, `get_daily_readiness`, `get_entity_context`, `list_open_loops`, `detect_risk_shift` migrate behind abilities. App Tauri commands become thin wrappers. MCP sidecar query handlers become thin wrappers over the same abilities. Parallel-run validation per [ADR-0109](0109-migration-strategy.md).
.docs/decisions/0102-abilities-as-runtime-contract.md:452:Remaining capabilities migrated. Direct Tauri commands for capability-level operations removed. MCP query handlers removed in favor of ability-wrapped tools. Pre-commit hook blocks new capability code outside `src-tauri/src/abilities/`.
.docs/decisions/0102-abilities-as-runtime-contract.md:459:The current architecture — Tauri commands that query services and build context inline, plus an MCP sidecar with parallel query paths — remains operational throughout the migration. Each capability is migrated individually:
.docs/decisions/0102-abilities-as-runtime-contract.md:463:3. The corresponding MCP tool is replaced with a thin wrapper over the same ability.
.docs/decisions/0102-abilities-as-runtime-contract.md:475:3. **Surface parity is structural.** App, MCP, and any future surface receive identical outputs because they invoke the same abilities. No "MCP version is thinner."
.docs/decisions/0102-abilities-as-runtime-contract.md:478:6. **Agents receive full product quality.** MCP tools are thin wrappers over abilities, not reduced subsets. When an agent invokes DailyOS, it receives the same briefing the app renders.
.docs/decisions/0102-abilities-as-runtime-contract.md:515:- **ADR-0108 (forthcoming): Surface-Independent Ability Invocation** — Specifies MCP and Tauri binding rules concretely (which follows from §7.3).
.docs/decisions/0102-abilities-as-runtime-contract.md:556:- Surface exposure: experimental abilities are registered and invokable only when a feature flag is active. Not exposed through MCP. Not exposed through Tauri commands unless the dev-mode flag is set.
.docs/decisions/0102-abilities-as-runtime-contract.md:571:  - **No MCP exposure:** experimental abilities are not enumerated by `registry.iter_for(Actor::Agent)` or MCP bridge output.
.docs/decisions/0102-abilities-as-runtime-contract.md:579:## Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace
.docs/decisions/0102-abilities-as-runtime-contract.md:581:**Origin:** v1.4.7 W0 (MCP Server v2, Abilities-First). Drafted as the substantive design output of v1.4.7's cycle-2 Class S finding ("MCP client trust boundary"). Cycles 2–5 polished the contract; this amendment lands the final shape.
.docs/decisions/0102-abilities-as-runtime-contract.md:583:**Scope:** the abilities runtime contract for MCP-originated invocations. Wire-level transport, pairing handshake server impl, per-tool grant manifest format, and gateway impl are W1-A scope and not codified by this amendment — only the actor-class contract and namespace freeze are.
.docs/decisions/0102-abilities-as-runtime-contract.md:603:`McpClientId` is a server-minted opaque handle issued during the MCP client pairing handshake (§C). Newtype lives at the abilities-runtime layer alongside `SurfaceClientId`; non-PII, non-enumerable, stable for the life of the pairing.
.docs/decisions/0102-abilities-as-runtime-contract.md:605:`OpaqueConversationHandle` is a server-minted, non-PII, non-enumerable token that tracks cross-conversation continuity (§D). `Option<…>` because the handle is absent on the very first MCP call in a fresh conversation — the gateway mints one transparently per §D.
.docs/decisions/0102-abilities-as-runtime-contract.md:614:- **McpClient** does not carry scopes because the MCP gateway (v1.4.7 `services/mcp_v2/gateway.rs`) enforces scope authorization against a **server-side per-client manifest** loaded by `services/mcp_v2/auth.rs` from the pairing record (§C). Caller-provided scope claims are rejected on principle; the gateway is the single source of truth. Once the gateway admits the call, the abilities runtime trusts the gateway's check and gates only on `AbilityPolicy.allowed_actors.contains(&ActorKind::McpClient)`.
.docs/decisions/0102-abilities-as-runtime-contract.md:616:This preserves the v1.4.2 SurfaceClient bridge contract verbatim while introducing a strictly distinct trust path for MCP-originated calls — different attribution semantics, different audit shape, different rate-limit budget (§C). `AbilityPolicy.required_scopes` remains a SurfaceClient-only enforcement field; W0 ships no new field on `AbilityPolicy`.
.docs/decisions/0102-abilities-as-runtime-contract.md:618:### C. MCP client authentication + session contract
.docs/decisions/0102-abilities-as-runtime-contract.md:620:Every MCP-originated invocation must pass through these four gates before reaching an ability:
.docs/decisions/0102-abilities-as-runtime-contract.md:622:1. **Pairing handshake.** First contact from a new MCP client issues an `McpClientId` after a server-side handshake (W1-A `auth::pair_client(handshake) -> McpClientId`). The handshake binds the client to a server-stored manifest of granted scopes + per-tool rate limits + per-tool exposure tier. Manifest content is set by the operator (DailyOS-as-product owner) at pairing time, not negotiated by the caller. Mirrors the v1.4.2 `SurfaceClient` pairing shape (`services::surface_pairing`, `services::surface_session_keychain`, `bridges::surface_client`) — the SurfaceClient pattern is the precedent and the same operator/audit ergonomics apply.
.docs/decisions/0102-abilities-as-runtime-contract.md:623:2. **Transport signing (HMAC-SHA256).** All stdio and HTTP transports MUST present a per-message HMAC-SHA256 signature derived from the pairing-time shared key. Unsigned or invalid-signature messages are rejected at the transport layer before any `Actor::McpClient` is constructed. (Local STDIO MCP processes share the key out-of-band via the pairing record; loopback HTTP carries it as a request header.)
.docs/decisions/0102-abilities-as-runtime-contract.md:633:Host-model conversations are stateless; DailyOS is not. Cross-conversation continuity (per ADR-0128 §6) is implemented via a server-minted opaque token:
.docs/decisions/0102-abilities-as-runtime-contract.md:635:- **Mint trigger.** Server mints on the first MCP call from a client that does not present an existing handle. The new handle is returned in the response envelope's top-level `conversation_handle` field (separate from the tool's typed payload).
.docs/decisions/0102-abilities-as-runtime-contract.md:644:Canonical convention for new MCP tool scopes:
.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/decisions/0102-abilities-as-runtime-contract.md:652:- **v1.4.5-reused scopes stay unprefixed.** Scopes shipped by v1.4.5 substrate (e.g. `write.workspace_place_document`, `read.workspace_graph`, `read.entity_names`) are kept verbatim. v1.4.7 MCP tools that reuse them present the unprefixed scope to the manifest. The manifest validation logic accepts both forms but rejects any scope outside the canonical allowlist.
.docs/decisions/0102-abilities-as-runtime-contract.md:657:`AbilityPolicy.allowed_actors` may now include `ActorKind::McpClient` alongside the existing four kinds. Defaults are unchanged (`&[ActorKind::User]`, least-privilege floor); abilities opt in to MCP exposure by adding `ActorKind::McpClient` to their `allowed_actors` slice.
.docs/decisions/0102-abilities-as-runtime-contract.md:663:No new variants are added to `McpExposure { None | MetadataOnly | Invocable }`. The tri-state continues to govern MCP introspection behavior verbatim for v1.4.7 — `None` hides the ability from the v1.4.7 gateway's `list_tools` and `list_abilities`, `MetadataOnly` exposes name + description without invoke schema, `Invocable` exposes full schema.
.docs/decisions/0102-abilities-as-runtime-contract.md:669:- The MCP gateway implementation (`services/mcp_v2/gateway.rs` body) — that is W1-A (DOS-168).
.docs/decisions/0102-abilities-as-runtime-contract.md:675:- Any actual MCP tool handler logic — W2/W3/W4.
.docs/decisions/0102-abilities-as-runtime-contract.md:679:### I. Companion ADR-0128 amendment
.docs/decisions/0102-abilities-as-runtime-contract.md:681:ADR-0128 receives a same-date amendment recording the v1.4.7-specific consequences of this trust-boundary contract: tool description as product copy with eval coverage (DOS-481), cross-conversation continuity surfaced as a named affordance per ADR-0128 §6 with the wire-level shape defined here in §D, and extension of the displacement-use-case framing to v1.4.7 W5 host-selection evaluation. ADR-0128 §5 ("Feedback is the only write") narrows in v1.4.7's headless head: writes are restricted to the v1.4.7 W4 surface (note/observation, create_action, update_action_status), all routed through approved `services::*` per ADR-0101.
.docs/decisions/0102-abilities-as-runtime-contract.md:713:Origin: v1.4.7 W1-A L0 cycle 5 CSO HIGH finding + L6 verdict. The cycle-1 §C contract specifies per-message HMAC-SHA256 transport signing but does NOT require replay protection. A captured, signed MCP message could be replayed by an attacker within the validity window of the pairing key OR the `OpaqueConversationHandle` (up to 24 hours). For a v1.4.7 W4 write-tool (`dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`, `dailyos.write.place_document`), replay produces duplicate mutations.
.docs/decisions/0102-abilities-as-runtime-contract.md:717:Every MCP-originated message MUST include a `request_nonce` field whose value is signed by the transport HMAC alongside the payload. The nonce is server-issued during the pairing handshake (initial seed) and refreshed per response via a server-side ledger. Format: 128-bit cryptographically-random opaque token.
.docs/decisions/0102-abilities-as-runtime-contract.md:794:This amendment does NOT change: the HMAC-SHA256 signing algorithm; the pairing handshake (still issues the seed nonce); rate-limit ledger; manifest schema; audit shape (success-only per L6-2); `OpaqueConversationHandle` lifecycle. The 5-minute expiry window is fixed in W1-A; configurable TTL is filed as DOS-? "MCP nonce TTL configuration" in the Maintenance project for post-v1.4.7 tuning.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:125:**Scope.** v1.5.0 or later. Not substrate-blocking; pure product surface work.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:178:- Gap closure is distributed: A goes into the enrichment spike + v1.5.0; B is a [ADR-0110](0110-evaluation-harness-for-abilities.md) amendment; C is a [DOS-7](https://linear.app/a8c/issue/DOS-7) design decision; D is input into [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md); E is a v1.5.0+ product surface. None require a new foundational ADR.
.docs/decisions/0048-three-tier-data-model.md:29:Meanwhile, the core insight behind ADR-0018 remains correct: **users own their data as portable files.** The workspace directory must be meaningful without the app. Markdown and JSON files are what make the archive valuable to Claude Desktop, Claude Code, and any future AI tool. This principle doesn't require the fiction that SQLite is disposable.
.docs/decisions/0048-three-tier-data-model.md:85:Claude Desktop, Claude Code, ChatGPT, and any other tool reads and writes the filesystem — markdown and JSON files. They never touch SQLite directly. The app bridges the filesystem and SQLite layers, syncing changes in both directions per ADR-0047's sync model.
.docs/decisions/0105-provenance-as-first-class-output.md:350:3. **Serialization overhead on every Tauri/MCP response.** Acceptable at scale but measurable.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:65:    /// Fine on user's own surfaces, briefings, MCP. Never customer-facing or shared/published.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:70:    /// Fine on the user's own surfaces only. Not on MCP, not on Publish, not on briefings shared with others.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:202:**ADR-0102 (abilities runtime) assessment.** v1.4.6's `recommend_for_entity` ability targets the existing `mcp_exposure: McpExposure::MetadataOnly` tri-state value (added in the 2026-05-10 amendment). No further ADR-0102 amendment needed for v1.4.6 W0. v1.4.7 may amend ADR-0102 to flip `mcp_exposure` to `Invocable` once the MCP `dailyos.read.portfolio_attention` consumer ships.
.docs/decisions/0095-dual-mode-context-architecture.md:48:        endpoint: String,     // Glean MCP server URL
.docs/decisions/0095-dual-mode-context-architecture.md:62:The PTY enrichment path, signal bus, meeting prep queue, and `IntelligenceJson` schema are all unchanged. What changes is where `build_intelligence_context()` gets its data. The `GleanContextProvider` calls Glean's MCP server (search + read_document tools) to populate `IntelligenceContext.file_contents` and enriches `stakeholders` from Glean's org graph.
.docs/decisions/0095-dual-mode-context-architecture.md:107:| `src-tauri/src/context_provider/glean.rs` | `GleanContextProvider` — Glean MCP client, two-phase gather |
.docs/decisions/0095-dual-mode-context-architecture.md:127:- Glean MCP protocol may evolve — client must be maintained
.docs/decisions/README.md:50:| [0027](0027-mcp-dual-mode.md) | MCP integration: dual-mode server + client | Accepted |
.docs/decisions/README.md:95:| [0075](0075-conversational-interface-architecture.md) | Conversational interface architecture — MCP-first chat, session memory, entity-focused queries | Accepted |
.docs/decisions/README.md:130:| [0111](0111-surface-independent-ability-invocation.md) | Surface-independent ability invocation — Tauri/MCP/worker bridges (v1.4.0) | Proposed |
.docs/decisions/0130-surface-independent-composition-contract.md:9:**Relates to:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md)
.docs/decisions/0130-surface-independent-composition-contract.md:14:DailyOS has converged on a runtime + composable surfaces architecture (ADR-0128 surface-pluggable thesis; ADR-0129 WordPress as a leading concrete surface; ADR-0111 §8 generalized SurfaceClient as the fourth actor class for third-party local surfaces). The substrate-side contracts are in place:
.docs/decisions/0130-surface-independent-composition-contract.md:31:This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md) §1 from "surfaces invoke abilities" to "surfaces consume substrate output, including composed pages." It generalizes [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §1 from "the substrate is the product; the heads are surfaces over it" to apply at the *page* level, not just the *claim* level.
.docs/decisions/0130-surface-independent-composition-contract.md:156:| MCP head (headless) | `JsonBlockRenderer` | Claim-shaped JSON per ADR-0128 §4 |
.docs/decisions/0130-surface-independent-composition-contract.md:157:| CLI head | `MarkdownBlockRenderer` | Markdown per ADR-0128 §7 |
.docs/decisions/0130-surface-independent-composition-contract.md:162:Surfaces MAY decline to render specific block types (e.g., MCP head returns claim-JSON for `AgenticPrompt` rather than rendering an invocable UI; CLI head emits a marker for `MagazineEnd` rather than visual spacing). The Composition stays canonical; surfaces decide what to show.
.docs/decisions/0130-surface-independent-composition-contract.md:188:The same composition rendered by the WordPress SurfaceClient, Tauri React surface, and MCP head produces equivalent intelligence — different paint, same content, same provenance.
.docs/decisions/0130-surface-independent-composition-contract.md:200:- **Editing semantics.** A user editing a block's value (claim correction) routes through the existing feedback path per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5; this ADR does not redesign feedback.
.docs/decisions/0130-surface-independent-composition-contract.md:222:- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.
.docs/decisions/0130-surface-independent-composition-contract.md:228:- ADR-0128 §1 is preserved — substrate is the product, surfaces are heads over it.
.docs/decisions/0130-surface-independent-composition-contract.md:243:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
.docs/decisions/0025-app-native-governance.md:12:Governance logic lives in compiled Rust inside the Tauri app. The app reads `~/.dailyos/` registries and workspace structure directly. No Python subprocess calls for core operations. The app is the authority on first install; CLI and MCP are secondary interfaces that share the same registries.
.docs/decisions/0025-app-native-governance.md:18:- Interface parity: App, CLI, and Claude Code (via MCP) all read the same state

codex
The packet already covers the happy path and the local-to-local threat model cleanly. I’m zooming into the catalog entries now, since the likely DX risk is whether Claude gets enough shape from `tools/list` to choose and call correctly.
exec
/bin/zsh -lc "sed -n '1,120p' .docs/decisions/0083-product-vocabulary.md" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "sed -n '1,88p' src-tauri/resources/mcp_v2/tool_descriptions.yaml" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# ADR-0083: Product Vocabulary — User-Facing Language Standard

**Date:** 2026-02-18
**Status:** Proposed
**Participants:** James Giroux, Claude Code

**Related:** ADR-0073 (Editorial design language), ADR-0076 (Brand identity), ADR-0081 (Event-driven meeting intelligence), I329 (Intelligence quality indicators)

---

## Context

DailyOS has been built at speed. The codebase, the ADRs, the UI copy, and the backlog all use language that was convenient for the builders — "entity," "enrichment," "signal," "intelligence," "prep." These terms are precise for the system's internals but meaningless or confusing for the person using the app.

As the product matures, the vocabulary needs to split cleanly into two layers:

1. **System vocabulary** — what developers and ADRs use to describe architecture. Precise, abstract, composable.
2. **Product vocabulary** — what users see in the UI. Warm, specific, self-evident.

These have been conflated. "Entity" appears in UI picker labels. "Enrichment" leaks into loading states. "Signal" is used in user-facing badge descriptions. "Intelligence" means six different things depending on context. "Prep" describes the system's output as though the user hasn't done their job yet.

The editorial design language (ADR-0073) gave DailyOS a visual identity — serif headlines, material palette, magazine layout. This ADR gives it a verbal identity.

### The Problem in Practice

A user opening DailyOS today encounters:

- "Entity Picker" — Link what to what? What's an entity?
- "Needs Prep" badge — Whose job is that? The system's or mine?
- "Intelligence Report" — Am I a spy? What does this contain?
- "Enrichment in progress" — What is being enriched? With what?
- "Proposed actions" — Proposed by whom?
- "Signal bus" concepts leaking into UI through badge labels
- "Account" used for both external customers and internal teams (ADR-0070)

The chief-of-staff metaphor is the product's north star. A chief of staff doesn't say "I've enriched your entity intelligence with new signals." They say "I found something new about Acme before your meeting."

---

## Decision

### 1. Two Vocabularies, One Rule

**System vocabulary** lives in code, ADRs, and developer documentation. It is precise and abstract. `entity`, `signal`, `enrichment`, `intelligence`, `prep`, `resolution` — all fine here.

**Product vocabulary** lives in UI strings, user-visible copy, error messages, badges, labels, and any text a user reads. It is warm, specific, and self-evident. No system term appears in user-facing text without translation.

**The rule:** If a user can see it, it uses product vocabulary. No exceptions.

### 2. The Translation Table

#### Core Concepts

| System term | Product term | Rationale |
|---|---|---|
| Entity | *(use the specific type)* — Account, Project, Team, Person | Nobody has "entities." Always use the specific type. If you must refer to the category generically: "account or project" |
| Entity resolution | *(invisible)* | The system links meetings to the right account/project. Users don't need to know this happens. When it's wrong, they correct it by changing the link. |
| Intelligence (on a meeting) | **Briefing** | "Your meeting briefing." Familiar, implies preparation on your behalf. The meeting detail page is the briefing. |
| Intelligence (on an account/project) | **Insights** | "Account insights." Lighter than intelligence, implies distilled understanding. |
| Intelligence (the system's output generally) | **Context** | "Building context for Thursday's QBR." Generic, non-threatening, accurate. |
| Intelligence quality levels | See Section 3 | |
| Enrichment | *(invisible)* or "Updating" | Users don't enrich things. The system "found new information" or is "updating context." |
| Signal | **Update** or **Change** | "2 new updates since this morning." A signal is what the system sees; an update is what the user cares about. |
| Prep / prep file | **Briefing** | The output is a briefing, not a task the user needs to do. "Needs prep" becomes a quality indicator (see Section 3). |
| Proposed (action status) | **Suggested** | "AI suggested" is already in the I334 UI. Formalize it. |
| Archived (action status) | **Dismissed** | "Archived" sounds like filing. "Dismissed" is what the user did — they rejected the suggestion. |
| Signal bus | *(never user-facing)* | |
| Bayesian fusion | *(never user-facing)* | |
| Thompson Sampling | *(never user-facing)* | |
| Correction learning | *(invisible)* | The system just "gets smarter" or "learns your preferences." |

#### Surfaces

| System term | Product term | Rationale |
|---|---|---|
| Daily Briefing | **Today** or **Daily Briefing** | Both work. "Today" is the nav label; "Daily Briefing" is the page identity. |
| Weekly Forecast | **This Week** or **Week Ahead** | "Forecast" is fine but "This Week" is warmer as a nav label. |
| Meeting Detail Page | **Meeting Briefing** | It's the briefing for that meeting. Not a "detail page." |
| Account Detail Page | **Account** (by name) | "Acme Corp" not "Account Detail: Acme Corp." |
| Actions Page | **Actions** | This one's fine. Clear, universal. |

#### Actions & States

| System term | Product term | Rationale |
|---|---|---|
| Run Briefing | **Refresh** (when intelligence exists) / **Prepare my day** (cold start) | "Run" is a developer verb. You refresh a briefing. On first open, the system prepares your day. |
| Refresh Intelligence | **Check for updates** | "Is there anything new?" not "re-enrich the meeting intelligence record." |
| Complete action | **Done** | Checkbox affordance is enough. The verb is "mark as done." |
| Reopen action | **Not done** | Undo of "done." |
| Accept (proposed action) | **Accept** | This one works. Clear intent. |
| Reject (proposed action) | **Dismiss** | Softer than reject. You're not refusing the AI; you're setting this suggestion aside. |

### 3. Intelligence Quality Labels

ADR-0081 defines four quality levels: Sparse, Developing, Ready, Fresh. These are system-accurate but user-unfriendly. "Sparse" sounds like an error. "Developing" sounds like it's the system's problem.

**Product quality labels:**

| System level | Product label | Badge style | What it communicates |
|---|---|---|---|
| Sparse | **New** | Grey, understated | "This meeting just appeared. I'm still learning about it." |
| Developing | **Building** | Turmeric, warm | "I'm putting together context. More coming." |
| Ready | **Ready** | Sage, confident | "You're prepared. I've done the work." |
| Fresh | **Updated** | Sage + dot | "Ready, and I just found something new." |
| Stale (Ready but old) | **Ready** + subtle refresh icon | Sage, muted | "Good context, but it's been a while. Tap to check for updates." |

**The "new updates" indicator** (blue dot in ADR-0081) becomes simply a dot with no label. Its presence says "something changed." Tapping reveals what.

**What replaces "Needs Prep":** Nothing. The concept disappears. Every meeting has a quality level. "New" meetings will get context as the system learns. There is no state where the user is told they need to do something the system should have done.

### 4. The Chief of Staff Voice

Beyond individual terms, the product has a voice. The editorial design language (ADR-0073) established the visual voice: restrained, confident, magazine-editorial. The verbal voice should match:

**Confident, not apologetic.** "Ready" not "We think we have enough information." The system did its job.

**Specific, not abstract.** "2 new updates about Acme since yesterday" not "New signals detected on linked entity."

**Warm, not clinical.** "Building context" not "Enrichment in progress."


 succeeded in 0ms:
# DailyOS MCP v2 tool taxonomy — version-controlled product copy.
#
# Per ADR-0128 §3 + DOS-478 L0 packet §1: this catalog is the source of
# truth for what host models see when selecting tools. Authored per the
# product-vocabulary discipline in ADR-0083 + AC-4/AC-6 of the L0 packet:
# natural-language fields (summary, when_to_call, when_NOT_to_call) carry
# no system identifiers or `dailyos.` substrings; tool identifiers appear
# only in structured `selection_fixtures.*.expected_tool`.
#
# Per AC-2: `side` is one of `Read | SubmitCorrection | Write` verbatim.
# Per AC-3: `scopesRequired` matches the §5 table exactly.
# Per AC-5: `selectionFixtures` carries ≥ 2 positive + ≥ 2 broad-corpus
# negative + ≥ 1 adjacent-tool negative (≥ 2 where adjacent exists).
# Per AC-8: nested deny_unknown_fields applies — no typos.

- name: dailyos.read.account_status
  side: Read
  summary: Returns the current state of an account the user works with.
  when_to_call: |
    When the user asks about a specific account they have a working
    relationship with — status, recent signals, open commitments,
    champion engagement, contract state. The assistant persists context
    across conversations and updates as work evolves; rely on this for
    cross-conversation continuity.
  when_NOT_to_call: |
    Do NOT call for accounts the user has no working relationship with —
    this surface is for the user's personal working understanding of their
    professional world, not broad enterprise or web corpus search. Use
    enterprise search, web search, or Glean for broad corpus lookups.
  scopesRequired:
    - dailyos.read.account_status
  parameters:
    - name: subject
      schema: { type: string }
      required: true
      description: Account name or handle
  returns:
    schema: { type: object, additionalProperties: false }
    description: Account status payload with claim attribution and freshness per ADR-0105
  examples:
    - prompt: What's going on with Acme?
      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
      expectedResponseShape: { account_id: opaque, status: enum, as_of: iso8601 }
  selectionFixtures:
    positive:
      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
    negativeBroadCorpus:
      - { prompt: Search the web for SaaS pricing benchmarks, expectedToolClass: external }
      - { prompt: What's the company's vacation policy?, expectedToolClass: external }
    negativeAdjacentTool:
      - { prompt: What did I write about the Q1 readout?, expectedTool: dailyos.search.workspace_memory }

- name: dailyos.read.daily_briefing
  side: Read
  summary: Returns the user's current-day briefing.
  when_to_call: |
    When the user asks about today's priorities, meetings, accounts that
    need attention, or what to focus on. The assistant persists context
    across conversations so today's briefing reflects yesterday's open
    loops and updates as work evolves through the day.
  when_NOT_to_call: |
    Do NOT call for general news, weather, or org-wide announcements —
    use web search or enterprise search for broad corpus lookups. Do NOT
    call for a specific meeting's prep notes; that is a different surface.
  scopesRequired:
    - dailyos.read.daily_briefing
  parameters: []
  returns:
    schema: { type: object, additionalProperties: false }
    description: Briefing payload with prioritized accounts, meetings, and open commitments
  examples:
    - prompt: What's on my plate today?
      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
      expectedResponseShape: { date: iso8601, priorities: array, meetings: array }
  selectionFixtures:
    positive:
      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
    negativeBroadCorpus:
      - { prompt: What's the weather today?, expectedToolClass: external }
      - { prompt: Show me today's news headlines, expectedToolClass: external }
    negativeAdjacentTool:
      - { prompt: What should I prep for my 2pm with Acme?, expectedTool: dailyos.read.meeting_briefing }

- name: dailyos.read.meeting_briefing
  side: Read
  summary: Returns the prep briefing for a specific meeting.

exec
/bin/zsh -lc "sed -n '1,160p' .docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# ADR-0128 — Headless DailyOS: MCP as a co-equal product surface

**Status:** Accepted
**Date:** 2026-05-03
**Amended:** 2026-05-04 — added §7 (CLI as a third head, mediated by MCP)
**Amended:** 2026-05-19 — same-date companion amendment to ADR-0102; codifies tool description as product copy (DOS-481 eval coverage), cross-conversation continuity wire shape (defers to ADR-0102 §D OpaqueConversationHandle lifecycle), restated v1.4.7 W4 write surface, displacement-use-case framing extended to v1.4.7 W5 host-selection eval, and Linear link refresh to the post-renumber v1.4.7 project. See section appended at end of doc.
**Authors:** James Giroux, Claude
**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)
**Linear:** [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)

## Context

[ADR-0027](0027-mcp-dual-mode.md) established DailyOS as both an MCP server (exposing workspace tools and resources to external agents) and an MCP client (consuming external services). The v1.5.0 project description is sharp on guarantees: tools must route through abilities, not parallel paths; writes must capture structured intent and provenance; reads must return current substrate state; claim-worthy outputs must explain source, freshness, confidence, and salience.

What is missing is positioning. The existing scope tells contributors *what guarantees not to break*. It does not tell them *what the product is from a user's seat*.

This ADR fills that gap. It does not change the v1.5.0 substrate scope. It frames it.

The frame is straightforward: **MCP is not an export pipe; it is a co-equal head over the same intelligence substrate as the Tauri app.** The Tauri app and the MCP head are two different consumption optimizations of one claim graph.

- The **Tauri head is opinionated**: briefing → triage → prep, with hierarchy and callouts the user reads visually. Designed for the daily ritual.
- The **MCP head is responsive**: whatever question came in, answer it well inside the host model's conversation. Designed for the on-demand ask.

Same substrate, different consumption shapes for different moments.

This framing has product consequences. Treating MCP as plumbing produces a generic data API. Treating MCP as product produces a tool surface as carefully designed as any UI in the app — and a tool description corpus that is itself the highest-leverage user-facing copy in the product, because every host-model conversation routes through it.

## Decision

### 1. Headless DailyOS as the canonical frame

The MCP server is the **headless head** of DailyOS. It exposes the same intelligence substrate as the Tauri app, optimized for a different consumption surface (host-model conversation rather than direct visual reading).

Architectural implication: the substrate is the product; the heads are surfaces over it. Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern. This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md): a Surface invokes abilities; a Surface is not a feature implementation.

The heads are co-equal *as consumption optimizations*, not feature-parity-mandated. The MCP head does not need to expose every Tauri view, and the Tauri app does not need to mimic conversational interaction. Each surface picks the questions and ritual it serves best.

### 2. Tools are question-shaped, not data-shaped

MCP tool design is governed by the questions a user would ask through their host model, not by the rows in the database.

- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
- Rejected: `get_account(id)`, `list_claims(filter)`, `search(q)` — generic data accessors that bypass synthesis and force the host model to reassemble context.

A tool that does not exercise an ability is a candidate for removal. This extends the v1.5.0 project Scope Guidance: convenience tools that do not strengthen parity with the DailyOS app loop are out of scope.

### 3. Tool descriptions are UI copy

Tool names and descriptions are the highest-leverage user-facing copy in the product. They determine when a host model reaches for DailyOS over alternatives (Glean, Slack search, web search), how it cites, and how it hedges.

Each tool description should:

- State the question shape it serves.
- State when to prefer it over likely alternatives the host model has access to.
- State the freshness model — DailyOS persists context across conversations and updates as work evolves.
- Match the voice and vocabulary of [ADR-0083](0083-product-vocabulary.md) — same product diction as the Tauri app.

Tool description copy is reviewed with the same care as a button label or section header.

### 4. Claim-shaped output, with provenance leaking through

Tool outputs are claim-shaped, not document-shaped. Each user-meaningful field carries inline trust band and source attribution per [ADR-0105](0105-provenance-as-first-class-output.md). The output structure should make it natural for a host model to render hedging when trust is low — *"according to a transcript from last Thursday, though this hasn't been confirmed elsewhere…"* — without explicit prompting.

The substrate leaking through into the conversation *is* the product differentiator. It is what makes a DailyOS-mediated answer feel different from a flat retrieval over a corpus.

### 5. Feedback is the only write — same posture, different surface

Consumption-first holds in the headless head. The MCP write surface is **feedback only**: corrections, dismissals, corroborations, contradictions, and tombstones, all flowing through the existing claim feedback path defined in [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).

The MCP head does not expose:

- Claim creation or direct edit.
- File generation, drafting, or sending.
- Calendar mutation, message composition, or external-system writes.

A user in Claude Desktop can correct what DailyOS knows. They cannot use DailyOS as a write layer. Drafting and sending happen in the tool that's good at writing, with DailyOS context piped in.

This preserves the v1.5.0 user outcome ("writes capture structured intent and provenance") in spirit by routing all writes through the same propose/commit boundary the Tauri app uses, with the same actor distinctions per [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).

### 6. Cross-conversation continuity is a named affordance

Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.

This is a quiet superpower and should be made explicit in the surface itself, not left as an implementation detail. Tool descriptions name it. Returned objects, where appropriate, surface it ("last verified Tuesday; no contradicting signals since"). The host model is given the language to communicate substrate persistence to the user.

### 7. CLI as a third head, mediated by MCP

The same headless framing extends to a CLI surface. CLI is a third head over the substrate, not a separate substrate consumer.

**Transport.** CLI invocations route through the MCP server. There is no parallel CLI ingress to the abilities runtime. The default is MCP-mediated; a CLI-distinct transport is only justified if something genuinely cannot be expressed through the MCP surface, and that decision is deferred until the need is concrete. Keeping a single ingress to the abilities runtime is easier to reason about, easier to evolve, and prevents the v1.5.0 mission gate ("don't bypass guarantees") from being silently undermined by a second path.

**Render.** CLI's distinct contribution is the output shape. Three renders over one substrate:

- Tauri: substrate → React components
- MCP: substrate → claim-shaped JSON
- CLI: substrate → markdown

Markdown is the right CLI render because it is both terminal-readable and vault-compatible. A `dailyos brief` command emits markdown a user can read in the terminal, pipe to a file, or paste into Obsidian. This actively reinforces the "you always keep your files" defensibility position established by the v0.0.1 PARA + markdown foundation — the substrate's intelligence is reachable as markdown, not only as a GUI.

**Two consumption modes share this transport:**

- **CLI-for-agent.** A Claude Code skill, or any agent that shells out, invokes the CLI as part of a workflow. Skills are *macros over the MCP surface* — named, slash-invokable bundles of MCP calls with workflow shape. The agent is the consumer; the user is one step removed. Structurally similar to MCP-via-host-model.
- **CLI-for-human.** A developer or power user invokes the CLI directly in a terminal. Direct consumption like the Tauri app, text-shaped instead of visual-shaped.

**Scope discipline is unchanged.** CLI commands mirror the **app loop** (briefing, prep, account status, commitments, action review), not the **org corpus** (Glean's territory). The displacement framing in §The displacement use case applies regardless of transport.

**Existing skills bundles.** Distributable Claude Code skills already exist in the repository:

- `plugins/dailyos/skills/` — workspace-fluency, entity-intelligence, meeting-intelligence, political-intelligence, action-awareness, relationship-context, analytical-frameworks, role-vocabulary, loop-back.
- `plugins/dailyos-writer/skills/` — writer-core plus voice/structural/authenticity/mechanical review.

These predate the abilities-first MCP surface. They should be refreshed once v1.5.0 lands so they consume the new tool surface and inherit its descriptions, output shape, and feedback semantics. This is a v1.5.x follow-on, not v1.5.0 substrate work — but it is the natural beneficiary of v1.5.0 once the MCP head is solid.

## The displacement use case

This ADR adopts a worked example as a design check for v1.5.0 issues: the **Glean displacement** scenario.

A user is in Claude Desktop with both a Glean MCP and a DailyOS MCP connected. They ask: *"What's going on with Acme Corp?"*

The desired behavior: the host model reaches for DailyOS, not Glean.

Why DailyOS wins this question:

| Dimension | Glean | DailyOS |
|---|---|---|
| Corpus | Org-wide documents and messages | The user's working substrate |
| Output | Retrieval (matched documents) | Synthesis (current claim state) |
| Freshness | Updates when content is written | Updates when *anything* happens, with re-evaluation |
| Trust | Flat | Trust bands per claim |
| Permissions | Org policy | The user's local-first DB |

DailyOS does not replace Glean. Glean remains the right substrate for org-wide policy, how-to, and broad-corpus search. The two are complements with a clean seam:

> **DailyOS for the user's working understanding of their professional world; Glean for the corpus of the company they work at.**

The MCP surface succeeds at this question when the host model reaches for DailyOS *because the tool descriptions taught it to*, not because we blocked Glean. Each v1.5.0 tool description should be reviewable against this scenario.

The same complement-not-replacement framing extends to other corpus tools the host model may have connected: Notion search, Slack search, web search. The seam is always the same — DailyOS for *the user's working understanding*, the other tool for *its corpus*.

## Non-goals

Restated and extended from the v1.5.0 project description:

- **Not a wrapper for Claude.** DailyOS does not host conversation. The user converses in their host model of choice; DailyOS is the substrate the host model consults.
- **Not a write layer.** Drafting, generation, sending, and external-system mutation are out of scope. Feedback corrections are not "writes" in the user-facing sense; they are the same propose/commit primitive the Tauri app uses.
- **Not a Glean replacement.** Org-wide retrieval is not what DailyOS does. Personal-context synthesis is.
- **Not a parallel feature surface.** (Already established in v1.5.0 project description; restated here as it is a corollary of §1.)
- **Not a generic data API.** Tools that do not exercise abilities are not added for completeness.

## Consequences

### Positive

- v1.5.0 issue scoping has a positioning lens. *"Does this tool serve a question a user would ask?"* and *"Would this tool description steer a host model toward DailyOS over Glean for the right questions?"* become reviewable design questions.
- Tool descriptions become a recognized work product, not an afterthought. Likely produces a tool surface inventory deliverable under v1.5.0 (tool name, description, expected question shape, return shape, displacement test).
- The headless-head metaphor generalizes. Future surfaces (CLI, mobile, browser extension, agent shells) inherit the same posture without re-litigation.
- The displacement use case gives partnership and positioning conversations a concrete frame. DailyOS and Glean are complements, not competitors. Same posture extends to other corpus tools.
- Reinforces the consumption-first product principle established implicitly across v1.4.x by making it explicit and surface-portable.

### Negative / risks

exec
/bin/zsh -lc 'rg -n "DOS-478|W1-B|tool_descriptions|when_to_call|when_NOT_to_call|positive|negative|catalog" .docs/plans .docs/decisions docs/solutions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0082-entity-generic-prep-pipeline.md:293:- **1:1 detection accuracy.** A 2-person meeting isn't always a 1:1 (could be ad hoc, interview, etc.). Recurring + historical frequency signals mitigate false positives.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:200:Section 3 says "Tool descriptions are UI copy." v1.4.7 turns that posture into a versioned artifact and an eval gate. W1-A defines a typed `ToolDescription` struct in `services/mcp_v2/contracts.rs`; W1-B owns the tool-description YAML catalog at `src-tauri/resources/mcp_v2/tool_descriptions.yaml` as the version-controlled source.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:202:The host-selection eval in DOS-481, scheduled for v1.4.7 W5-A, is the product-copy acceptance test. Each tool ships with at least two positive fixtures and at least two negative fixtures: one broad-corpus negative where DailyOS should yield to enterprise or web search, and one adjacent-DailyOS-but-wrong-tool negative where another DailyOS tool is the right choice. The `when_NOT_to_call` field is mandatory and explicit because avoiding DailyOS for the wrong ask is part of the product surface, not merely an eval trick.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:220:Every v1.4.7 tool description therefore ships with positive host-selection fixtures where the model should pick DailyOS for the right personal-working-context ask, and negative fixtures where the model should avoid DailyOS for either broad-corpus questions or adjacent-DailyOS-but-wrong-tool questions. The eval is a product-strategy artifact as much as a regression test.
.docs/decisions/0085-email-as-intelligence-input.md:13:The chief of staff analogy: A great EA doesn't hand their executive a sorted stack of mail. They read the mail, connect it to what's happening in the business, and deliver a distillation: "Jack is confirming the Acme EBR agenda for Thursday. This aligns with the renewal discussion you had Tuesday. No new action items, but the tone suggests they're leaning positive on the expansion."
.docs/decisions/0115-signal-granularity-audit.md:39:Granularity is a property of the signal type, not the emitter. The signal catalog in `src-tauri/src/signals/types.rs` groups types by level for readability.
.docs/decisions/0115-signal-granularity-audit.md:232:- **Phase 0 (prerequisite work, lands before any policy registry):** Introduce `pub enum SignalType` in `src-tauri/src/signals/types.rs`. Initial variants mirror the existing string catalog found in signal-emitting code (`stakeholder_change`, `title_change`, `champion_risk`, `renewal_risk_escalation`, `engagement_warning`, `intelligence_confirmed`, `meeting_frequency_drop`, etc.). Plus the new claim-layer variants from [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (`ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimTrustChanged`, `ClaimContradiction`). The enum is `#[non_exhaustive]` for forward compatibility.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:93:Concrete user case documented during v1.0.0 GA: users remove a value from a multi-select (e.g., a role type); because enrichment is ephemeral and the AI does not see the prior state or the removal, the next run repopulates the field. The current workaround is to stub a sentinel value so the prompt has something to preserve. This is a symptom of two distinct missing primitives: claim history (Gap C) and negative-knowledge tombstones (Gap D).
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:138:| D — Tombstones / negative knowledge | Resolved | [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) §5 + R1.1 + R1.3 (consolidates existing `suppression_tombstones`, `DismissedItem`, `account_stakeholder_roles.dismissed_at`) |
.docs/decisions/0076-brand-identity.md:147:- **Full weight without blobbing** — ExtraBold is the sweet spot. Bold is too thin for presence; Black merges the petals into a shapeless mass. ExtraBold maintains clear negative space between petals while carrying substantial visual weight.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:175:| `freshness_decay_class` | `Medium` | Weeks-scale half-life. The recommendation's evidence ages out on its own freshness clock; the recommendation itself decays alongside. W1-B salience re-scoring rebuilds candidacy continuously on signal arrival, so the freshness class governs how long a *quiet* recommendation remains valid before W4-B deviation detection re-evaluates. |
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:176:| `commit_policy_class` | `Replace` | A newer same-subject recommendation supersedes the older one. Recommendations are uniquely identified by `(subject, recommended_action_kind)`; corroboration semantics (`Reinforce`) don't apply because two distinct agent runs producing the same recommendation should *replace* (newer evidence wins), not stack confidence. Contradiction (`Fork`) is handled at the salience layer (W1-B): mutually-contradictory recommendations on the same subject lower each other's salience score and surface as a triage decision, not as parallel claim rows. |
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:196:- Salience scoring engine → W1-B (`services/recommendations/salience.rs`)
.docs/decisions/0074-vector-search-entity-content.md:33:- **Retrieval-optimized.** Arctic-embed-s scores 51.98 NDCG@10 on MTEB retrieval benchmarks — 10 points higher than general-purpose STS models like all-MiniLM-L6-v2 (41.95) at the same parameter count. Snowflake trained the arctic-embed family specifically for passage retrieval with hard negative mining, which directly maps to our entity content search use case.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:41:- An actor value is append-only to the catalog; once shipped, its shape does not change.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:271:| `committed` | Authoritative positive assertion. | Non-null. | Included in default reads. |
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:272:| `tombstoned` | Authoritative negative assertion — "this field is absent by intent." | NULL; `retraction_reason` required. | Included in default reads as a negative; blocks writers. |
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:276:`tombstoned` is its own state, not a variant of `committed`. It is a positive assertion of absence. Supersede/withdraw are for already-committed claims that leave the default read. This removes the internal contradiction.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:311:Add a **shadow-sampling policy**. For agents with `posterior_score < floor`, a configurable fraction (default 10%) of their below-threshold claims are surfaced to the Analysis Inbox for human review anyway, bypassing the gate. If the human accepts, the agent's ledger recovers normally. This turns "quarantine" into "probation with structured opportunity to recover" and prevents false-positive quarantines from being permanent.
.docs/decisions/0130-surface-independent-composition-contract.md:140:If no candidate has a positive deterministic score, or if the selected candidate has no eligible schema intersection with the unknown schema, the renderer MUST render the block as generic `dailyos/text` with an empty payload, preserved `claim_refs`, preserved `provenance`, the banner `Rendered as dailyos/text — payload may be incomplete`, and trust band `needs_verification`.
.docs/decisions/0120-observability-contract.md:202:- **Enumerated metric names.** A `const` list of permitted metric names in `observability::aggregate_metric_catalog`. New metrics require the same code review pressure as a new ADR amendment — can't slip in.
.docs/decisions/0058-proactive-intelligence-maintenance.md:197:- False-positive detection (incorrect name resolution, wrong entity linking) requires graceful correction
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:215:**Structural tombstone-shadow contract:** Today's tombstone gate is text/hash-keyed (`item_hash`, exact canonical text, keyless text tiers). The shadow-mode rollout (§7) backfills structural-identity tombstone keys onto existing tombstones. Until the shadow-mode parity gate clears, **both** legacy text/hash tombstone gates AND new structural-identity tombstone gates run in parallel (belt + suspenders) for both query and candidate sides. Helper deletion (§10) is gated on shadow-mode parity proving the structural gate has zero false negatives vs the legacy gate.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:385:- Composition: positive paraphrases (40%), hard negatives (30%), contradiction pairs (15%), one-sided qualifier asymmetry (10%), low-trust duplicates (5%)
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:506:- Threshold calibration fixture with stated false-positive ceiling
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:515:| Threshold tuning regression | Calibration fixture (§9) with false-positive ceiling; threshold changes require corpus regeneration + gate re-clear |
.docs/decisions/0080-signal-intelligence-architecture.md:234:| Email sentiment turns negative | Email signal on person | Account risk if person is champion |
.docs/decisions/0097-account-health-scoring-architecture.md:80:| Champion Health | Champion meeting attendance + sentiment + email engagement | Is our champion still active and positive? |
.docs/decisions/0097-account-health-scoring-architecture.md:82:| Signal Momentum | Positive vs negative signal ratio, 30d window, time-decayed | Is the trend improving or declining? |
.docs/decisions/0097-account-health-scoring-architecture.md:88:- "Health is Yellow, but champion engagement is strong and email sentiment is positive" → relationship is a buffer
.docs/decisions/0097-account-health-scoring-architecture.md:96:| Org Yellow/Red + Relationship Strong | Product/support issues but relationship holding. TAM is effective. | Yellow/Red band + positive relationship note |
.docs/decisions/0097-account-health-scoring-architecture.md:152:Email enrichment already extracts structured sentiment (`positive | neutral | negative | mixed`). Meeting sentiment is arguably more valuable — a tense meeting is a stronger signal than a terse email.
.docs/decisions/0097-account-health-scoring-architecture.md:158:- overall: positive|neutral|negative|mixed
.docs/decisions/0097-account-health-scoring-architecture.md:159:- customer: positive|neutral|negative|mixed
.docs/decisions/0097-account-health-scoring-architecture.md:171:- `forward_looking` → positive momentum signal ("future speak" per ChurnZero)
.docs/decisions/0097-account-health-scoring-architecture.md:298:These would feed into the Meeting Cadence and Champion Health dimensions — measuring not just "did we meet" but "was the meeting productive and positive." This is a future enhancement beyond v1.1.0 scope but architecturally enabled by the transcript sentiment extraction added in this ADR.
.docs/decisions/0102-abilities-as-runtime-contract.md:445:**Phase 1 (v1.4.0 planning window): Design and catalog.**  
.docs/decisions/0102-abilities-as-runtime-contract.md:446:[ADR-0103](0103-maintenance-ability-safety-constraints.md) through [ADR-0109](0109-migration-strategy.md) land as Proposed. Initial ability catalog drafted. Eval harness scaffolding in place per [ADR-0107](0107-evaluation-harness-for-abilities.md). No implementation churn yet.
.docs/decisions/0102-abilities-as-runtime-contract.md:653:- **Encoding.** The convention is encoded both in this ADR amendment (canonical text) and in the v1.4.7 W0 `services/mcp_v2/contracts.rs` `Scope` newtype + `ScopedName` newtype, with a load-time validation pass in W1-B's `taxonomy.rs` that rejects out-of-allowlist tool names at registry boot.
.docs/decisions/0102-abilities-as-runtime-contract.md:674:- Tool descriptions or the YAML catalog — W1-B (DOS-478).
.docs/plans/v1.4.7-waves.md:40:6. **`when_NOT_to_call` renamed to `when_not_to_call`** (clippy snake_case); wire field stays uppercase via `#[serde(rename = "when_NOT_to_call")]`.
.docs/plans/v1.4.7-waves.md:50:12. **DOS-481 fixture density.** Minimum ≥2 positive + ≥2 negative per tool: one broad-corpus negative (web/enterprise search) + one adjacent-DailyOS-but-wrong-tool negative.
.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-waves.md:110:- W1-B unchanged: owns `taxonomy.rs` content + `src-tauri/resources/mcp_v2/tool_descriptions.yaml`.
.docs/plans/v1.4.7-waves.md:151:2. **v1.4.5-enabled expansion** (new issue set DOS-478–DOS-482, DOS-479, DOS-480, DOS-171): expose the v1.4.5 source registry / lifecycle / claim proposals / graph projection / placement contract through MCP tools that respect privacy rendering and sensitivity rules. AI agents can search workspace memory, place documents, inspect provenance, and correct source attribution — all through the same service boundary the Tauri app uses.
.docs/plans/v1.4.7-waves.md:171:- `WorkspaceCategoryRegistry` (W1-B) — per-entity category allowlist + path resolution.
.docs/plans/v1.4.7-waves.md:209:| **L0 Prep (Plan Hardening)** | Every issue + project-scope | `/plan-eng-review` default. `/plan-devex-review` mandatory for **every issue** (this is an entirely DX-facing project — every tool is consumer-facing). `/cso` mandatory for the gateway lane (DOS-168), all write-tool lanes (DOS-167, DOS-169, DOS-170), and the v1.4.5-substrate-consuming lanes that surface workspace data (DOS-171, DOS-479, DOS-480). `/plan-ceo-review` for DOS-478 (taxonomy + host-selection language is product strategy). |
.docs/plans/v1.4.7-waves.md:220:DX-mandatory lanes (every lane carries `/plan-devex-review` because the entire project is a DX surface, but these have heightened DX scrutiny): DOS-478 (taxonomy + host-selection contract), DOS-481 (tool-description eval), DOS-175 (briefing replacement — change in observable MCP behavior), DOS-172 (pagination/ranking — pagination contract is a permanent shape), DOS-173 (portfolio attention — net-new tool surface).
.docs/plans/v1.4.7-waves.md:222:CEO-review lane: DOS-478 — the tool-taxonomy + host-selection contract IS the product-strategy artifact. `/plan-ceo-review` validates the displacement use case framing.
.docs/plans/v1.4.7-waves.md:283:| **W1** — Contract & Policy | DOS-168, DOS-478 | 2 | 4–5 days |
.docs/plans/v1.4.7-waves.md:374:    - `contracts.rs` (W1-A only owner) — defines (cycle 2 amendment makes the snippet compile-ready: renamed `when_NOT_to_call` to clippy-conforming `when_not_to_call` with serde rename for wire; defined `ParamSpec`/`ReturnSpec`/`ToolExample`/`ParamSchema`/`Side`; added shared `Page<T>` envelope; added `OpaqueConversationHandle`):
.docs/plans/v1.4.7-waves.md:381:          pub when_to_call: String,            // "for the user's working understanding of..."
.docs/plans/v1.4.7-waves.md:382:          #[serde(rename = "when_NOT_to_call")]
.docs/plans/v1.4.7-waves.md:474:    - `taxonomy.rs` (DOS-478 / W1-B owner; W1-A creates empty placeholder).
.docs/plans/v1.4.7-waves.md:496:### Agent W1-B — DOS-478: Tool taxonomy + host-selection contract
.docs/plans/v1.4.7-waves.md:498:- **Spec:** [DOS-478](https://linear.app/a8c/issue/DOS-478).
.docs/plans/v1.4.7-waves.md:499:- **Goal:** Define the v1.4.7 tool taxonomy + host-selection language as a versioned, reviewable artifact. This is the "tool descriptions are product copy" project principle made concrete: a YAML/Rust catalog of every v1.4.7 MCP tool with name, scopes, summary, when-to-call, when-NOT-to-call, examples. W1-B owns the catalog file + the host-selection contract document.
.docs/plans/v1.4.7-waves.md:501:  - `src-tauri/src/services/mcp_v2/taxonomy.rs` (W1-B only owner of content; W1-A pre-created the empty placeholder).
.docs/plans/v1.4.7-waves.md:502:  - `src-tauri/resources/mcp_v2/tool_descriptions.yaml` (new YAML catalog of every v1.4.7 tool; loaded at runtime to populate the `ToolDescription` registry).
.docs/plans/v1.4.7-waves.md:503:  - `.docs/decisions/0128-*-amendment-v147.md` (W0 deliverable; W1-B may reference but does not modify after W0 close).
.docs/plans/v1.4.7-waves.md:515:  - `when_to_call` describes the user-question class: "When the user asks about an account, project, person, or meeting they have a working relationship with..."
.docs/plans/v1.4.7-waves.md:516:  - `when_NOT_to_call` is mandatory and explicit: "Do NOT call for broad enterprise search, web search, or generic Q&A — use other tools for those."
.docs/plans/v1.4.7-waves.md:523:  5. *Feedback loop:* DOS-481 evaluation surfaces description quality issues; revisions flow back through `tool_descriptions.yaml`.
.docs/plans/v1.4.7-waves.md:524:- **Tests required:** YAML loads cleanly into `Vec<ToolDescription>`; every tool has non-empty `when_to_call` + `when_NOT_to_call`; every scope referenced exists in the canonical scope registry; every tool name conforms to the naming convention; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.7-waves.md:530:- L0 plan approvals: architect-reviewer + `/cso` + `/plan-devex-review` for W1-A; architect-reviewer + `/plan-ceo-review` + `/plan-devex-review` for W1-B.
.docs/plans/v1.4.7-waves.md:596:- **Done when:** Both tools dispatch through the v1.4.5 `workspace_graph` ability; sensitivity redaction enforced (negative test: caller without `read.entity_names` gets null entity names); pagination round-trips per v1.4.5 cursor contract; `cargo test` + `cargo clippy -D warnings` clean; `/cso` L0 plan AND L2 diff approval recorded.
.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-waves.md:687:  2. **Broad-corpus negative:** prompts that should NOT select any DailyOS tool (e.g., "Search the web for SaaS pricing benchmarks", "What's the company's vacation policy?", "Find docs about React hooks") — assert no DailyOS tool selected.
.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-waves.md:692:- **Done when:** Eval harness runs on the v1.4.7 tool description corpus; **≥2 positive + ≥2 negative fixtures defined per tool**, with at least one broad-corpus negative AND one adjacent-tool negative per tool (cycle 2 amendment); pass rate threshold defined and met; CI integration; `/plan-devex-review` + `/plan-ceo-review` L0 plan approval recorded.
.docs/plans/v1.4.7-waves.md:741:- [ ] DOS-481 positive fixtures pass at threshold
.docs/plans/v1.4.7-waves.md:742:- [ ] DOS-481 negative fixtures pass at threshold (DailyOS NOT selected for broad corpus questions)
.docs/plans/v1.4.7-waves.md:746:- [ ] DOS-168 gateway scope enforcement: negative tests green
.docs/plans/v1.4.7-waves.md:782:| 10 | Tool description text licensing / review process | W1-B (DOS-478) | **Open** — defer to W1-B L0 plan with `/plan-ceo-review` |
.docs/plans/v1.4.7-waves.md:798:| DX-mandatory lanes | All 13 issues (entire project is a DX surface; heightened scrutiny on DOS-478, DOS-481, DOS-175, DOS-172, DOS-173) |
.docs/plans/v1.4.7-waves.md:799:| CEO-review lane | DOS-478 (tool taxonomy + host-selection language) |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:40:**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:50:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:58:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:144:    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:149:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:154:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:162:Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:215:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:233:**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:246:- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:625:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:628:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:24:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:644:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:645:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:652:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:181:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:668:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:52:3. Fixture density (challenge + ceo) — ADR-0128 §B requires ≥ 2 positive + ≥ 2 negative per tool; fixed by AC-5 update + per-entry YAML fixture-stub fields
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:677:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:134:- **ADR-0128 §3 + §6 + cycle-7 §B amendment** — tool descriptions are product copy; cross-conversation continuity named as affordance; host-selection eval (DOS-481) requires ≥ 2 positive + ≥ 2 negative fixtures per tool with broad-corpus AND adjacent-wrong-tool negatives.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:681:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:167:- **AC-4 Displacement framing in product vocabulary** (cycle-1 ceo + challenge convergent + cycle-2 ceo tightening). Every entry's `when_NOT_to_call` MUST contain BOTH (a) at least one broad-corpus keyword: `broad`, `enterprise search`, `web search`, `Glean`, `corpus`; AND (b) at least one adjacent-tool framing in PRODUCT LANGUAGE: `different surface`, `not for ... — use ... for`, `personal working`, or natural-language reference to the alternative. **AC-4 ALSO forbids**: substring `dailyos.` AND system identifiers (`Read|SubmitCorrection|Write|McpClient|ToolError|McpToolHandler|Gateway|Scope`) in `when_to_call` / `when_NOT_to_call` natural-language fields per cycle-2 CEO. Tool identifiers appear ONLY in structured `selection_fixtures.*.expected_tool` field.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:682:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:169:- **AC-6 Continuity affordance in product vocabulary** (cycle-1 ceo #5 + cycle-2 ceo tightening). Every Read entry's `when_to_call` MUST mention continuity using one of: `cross-conversation`, `persists context`, `updates as work evolves`, `working understanding ... over time`. Per ADR-0128 §6 named-affordance contract. Same vocabulary discipline as AC-4 — no `dailyos.` substrings, no system identifiers.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:703:.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:152:5. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — W1-A defines the CATALOG INTERFACE only: `pub trait TaxonomyCatalog { fn validate_against_handlers(&self, handlers: &[&dyn McpToolHandler]) -> Result<(), TaxonomyError> }` + `TaxonomyError::HandlerYamlMismatch { handler: ScopedName, yaml: Option<ScopedName> }`. W1-B fills the YAML loader and invokes `validate_against_handlers` at boot.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:719:.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:731:.docs/plans/v1.4.7-waves.md:220:DX-mandatory lanes (every lane carries `/plan-devex-review` because the entire project is a DX surface, but these have heightened DX scrutiny): DOS-478 (taxonomy + host-selection contract), DOS-481 (tool-description eval), DOS-175 (briefing replacement — change in observable MCP behavior), DOS-172 (pagination/ranking — pagination contract is a permanent shape), DOS-173 (portfolio attention — net-new tool surface).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:764:.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:765:.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:772:.docs/plans/v1.4.7-waves.md:798:| DX-mandatory lanes | All 13 issues (entire project is a DX surface; heightened scrutiny on DOS-478, DOS-481, DOS-175, DOS-172, DOS-173) |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:824:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1440:.docs/decisions/0130-surface-independent-composition-contract.md:140:If no candidate has a positive deterministic score, or if the selected candidate has no eligible schema intersection with the unknown schema, the renderer MUST render the block as generic `dailyos/text` with an empty payload, preserved `claim_refs`, preserved `provenance`, the banner `Rendered as dailyos/text — payload may be incomplete`, and trust band `needs_verification`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:14:Review the L0 plan packet at .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md as a developer-experience reviewer for the Claude Desktop MCP integration surface. Lens: end-to-end DX for the user pairing Claude Desktop, listing tools, calling tools, getting useful output. Onboarding friction. Tool name + summary + when_to_call clarity for Claudes selection.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:16:K-in obligation: grep docs/solutions/ and .docs/decisions/ for ADR-0083 product vocabulary + ADR-0128 MCP product surface + DOS-478 catalog conventions. Also read src-tauri/resources/mcp_v2/tool_descriptions.yaml entries for account_status and daily_briefing.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:29:I’ll use the plan-devex-review lens and first gather the required local context: the plan packet, the cited ADR/catalog vocabulary, and the two MCP tool descriptions.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:423:**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:433:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:441:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:527:    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:532:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:537:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:545:Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:598:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:616:**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:629:- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:670:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:952:/bin/zsh -lc 'rg -n "DOS-478|catalog conventions|catalog|when_to_call|tool description|Tool Catalog|account_status|daily_briefing" docs/solutions .docs/decisions src-tauri/resources/mcp_v2/tool_descriptions.yaml' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:956:src-tauri/resources/mcp_v2/tool_descriptions.yaml:3:# Per ADR-0128 §3 + DOS-478 L0 packet §1: this catalog is the source of
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:957:src-tauri/resources/mcp_v2/tool_descriptions.yaml:6:# natural-language fields (summary, when_to_call, when_NOT_to_call) carry
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:958:src-tauri/resources/mcp_v2/tool_descriptions.yaml:16:- name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:959:src-tauri/resources/mcp_v2/tool_descriptions.yaml:19:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:960:src-tauri/resources/mcp_v2/tool_descriptions.yaml:31:    - dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:961:src-tauri/resources/mcp_v2/tool_descriptions.yaml:42:      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:962:src-tauri/resources/mcp_v2/tool_descriptions.yaml:46:      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:963:src-tauri/resources/mcp_v2/tool_descriptions.yaml:47:      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:964:src-tauri/resources/mcp_v2/tool_descriptions.yaml:54:- name: dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:965:src-tauri/resources/mcp_v2/tool_descriptions.yaml:57:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:966:src-tauri/resources/mcp_v2/tool_descriptions.yaml:67:    - dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:967:src-tauri/resources/mcp_v2/tool_descriptions.yaml:74:      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:968:src-tauri/resources/mcp_v2/tool_descriptions.yaml:78:      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:969:src-tauri/resources/mcp_v2/tool_descriptions.yaml:79:      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:970:src-tauri/resources/mcp_v2/tool_descriptions.yaml:89:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:971:src-tauri/resources/mcp_v2/tool_descriptions.yaml:121:      - { prompt: What's on my plate today overall?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:972:src-tauri/resources/mcp_v2/tool_descriptions.yaml:126:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:973:src-tauri/resources/mcp_v2/tool_descriptions.yaml:154:      - { prompt: What's going on with Acme specifically?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:974:src-tauri/resources/mcp_v2/tool_descriptions.yaml:159:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:975:src-tauri/resources/mcp_v2/tool_descriptions.yaml:190:      - { prompt: What's the current state of the Acme account?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:976:src-tauri/resources/mcp_v2/tool_descriptions.yaml:195:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:977:src-tauri/resources/mcp_v2/tool_descriptions.yaml:231:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:978:src-tauri/resources/mcp_v2/tool_descriptions.yaml:271:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:979:src-tauri/resources/mcp_v2/tool_descriptions.yaml:310:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:980:src-tauri/resources/mcp_v2/tool_descriptions.yaml:354:  when_to_call: |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:981:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:989:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:200:Section 3 says "Tool descriptions are UI copy." v1.4.7 turns that posture into a versioned artifact and an eval gate. W1-A defines a typed `ToolDescription` struct in `services/mcp_v2/contracts.rs`; W1-B owns the tool-description YAML catalog at `src-tauri/resources/mcp_v2/tool_descriptions.yaml` as the version-controlled source.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:990:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:220:Every v1.4.7 tool description therefore ships with positive host-selection fixtures where the model should pick DailyOS for the right personal-working-context ask, and negative fixtures where the model should avoid DailyOS for either broad-corpus questions or adjacent-DailyOS-but-wrong-tool questions. The eval is a product-strategy artifact as much as a regression test.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:991:.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:41:- An actor value is append-only to the catalog; once shipped, its shape does not change.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:992:.docs/decisions/0120-observability-contract.md:202:- **Enumerated metric names.** A `const` list of permitted metric names in `observability::aggregate_metric_catalog`. New metrics require the same code review pressure as a new ADR amendment — can't slip in.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:993:.docs/decisions/0115-signal-granularity-audit.md:39:Granularity is a property of the signal type, not the emitter. The signal catalog in `src-tauri/src/signals/types.rs` groups types by level for readability.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:994:.docs/decisions/0115-signal-granularity-audit.md:232:- **Phase 0 (prerequisite work, lands before any policy registry):** Introduce `pub enum SignalType` in `src-tauri/src/signals/types.rs`. Initial variants mirror the existing string catalog found in signal-emitting code (`stakeholder_change`, `title_change`, `champion_risk`, `renewal_risk_escalation`, `engagement_warning`, `intelligence_confirmed`, `meeting_frequency_drop`, etc.). Plus the new claim-layer variants from [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (`ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimTrustChanged`, `ClaimContradiction`). The enum is `#[non_exhaustive]` for forward compatibility.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:997:.docs/decisions/0102-abilities-as-runtime-contract.md:445:**Phase 1 (v1.4.0 planning window): Design and catalog.**  
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:998:.docs/decisions/0102-abilities-as-runtime-contract.md:446:[ADR-0103](0103-maintenance-ability-safety-constraints.md) through [ADR-0109](0109-migration-strategy.md) land as Proposed. Initial ability catalog drafted. Eval harness scaffolding in place per [ADR-0107](0107-evaluation-harness-for-abilities.md). No implementation churn yet.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1000:.docs/decisions/0102-abilities-as-runtime-contract.md:674:- Tool descriptions or the YAML catalog — W1-B (DOS-478).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1010:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1052:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:202:The host-selection eval in DOS-481, scheduled for v1.4.7 W5-A, is the product-copy acceptance test. Each tool ships with at least two positive fixtures and at least two negative fixtures: one broad-corpus negative where DailyOS should yield to enterprise or web search, and one adjacent-DailyOS-but-wrong-tool negative where another DailyOS tool is the right choice. The `when_NOT_to_call` field is mandatory and explicit because avoiding DailyOS for the wrong ask is part of the product surface, not merely an eval trick.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1260:The packet already covers the happy path and the local-to-local threat model cleanly. I’m zooming into the catalog entries now, since the likely DX risk is whether Claude gets enough shape from `tools/list` to choose and call correctly.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:282:**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:292:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:300:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:386:    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:391:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:396:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:404:Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:457:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:475:**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:488:- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:623:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1063:    /// Register the taxonomy catalog. Production callers wire this with
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1065:    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1066:        self.taxonomy = Some(catalog);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1070:    /// every registered handler has a matching catalog entry with matching
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1071:    /// `Side` (per DOS-478 L0 AC-7). Returns operator-readable error if
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1072:    /// mismatch. Also returns catalog→handler pending-tool list as
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1078:        let Some(catalog) = self.taxonomy.as_ref() else {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1084:        catalog.validate_against_handlers(&handlers)?;
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1085:        Ok(catalog.validate_catalog_against_handlers(&handlers))
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1193:src-tauri/src/services/mcp_v2/taxonomy.rs:193:    /// catalog entry with matching `Side`. Production `Gateway::seal()`
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1194:src-tauri/src/services/mcp_v2/gateway.rs:188:    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1207:src-tauri/src/mcp_v2/main.rs:203:    gateway.set_taxonomy(catalog.clone());
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1211:src-tauri/src/services/mcp_v2/taxonomy.rs:193:    /// catalog entry with matching `Side`. Production `Gateway::seal()`
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1212:src-tauri/src/services/mcp_v2/gateway.rs:188:    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1420:    let catalog = YamlTaxonomyCatalog::load_embedded()
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1422:    let catalog: Arc<dyn dailyos_lib::services::mcp_v2::taxonomy::TaxonomyCatalog> =
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1423:        Arc::new(catalog);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1425:    gateway.set_taxonomy(catalog.clone());
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1435:        "mcp_v2 boot: pairing {} verified, 0 handlers registered, {} catalog entries pending. \
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1442:            "DAILYOS_MCP_V2_REQUIRE_HANDLERS=1 set but {} catalog entries lack handlers; refusing to start.",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1547:src-tauri/src/services/mcp_v2/contracts.rs:595:        // `when_NOT_to_call` deliberately preserves the uppercase NOT
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1665:        catalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1769:            // silently absent — pending in catalog, not invocable yet.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1899:    catalog: Arc<dyn TaxonomyCatalog>,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1917:        catalog: Arc<dyn TaxonomyCatalog>,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:1925:            catalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:670:**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:680:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:688:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:774:    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:779:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:784:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:792:Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:845:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:863:**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:876:- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:929:.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1230:    /// Register the taxonomy catalog. Production callers wire this with
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1232:    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1233:        self.taxonomy = Some(catalog);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1237:    /// every registered handler has a matching catalog entry with matching
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1238:    /// `Side` (per DOS-478 L0 AC-7). Returns operator-readable error if
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1239:    /// mismatch. Also returns catalog→handler pending-tool list as
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1245:        let Some(catalog) = self.taxonomy.as_ref() else {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1251:        catalog.validate_against_handlers(&handlers)?;
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1252:        Ok(catalog.validate_catalog_against_handlers(&handlers))
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1650:   186	    /// Register the taxonomy catalog. Production callers wire this with
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1652:   188	    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1653:   189	        self.taxonomy = Some(catalog);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1657:   193	    /// every registered handler has a matching catalog entry with matching
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1658:   194	    /// `Side` (per DOS-478 L0 AC-7). Returns operator-readable error if
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1659:   195	    /// mismatch. Also returns catalog→handler pending-tool list as
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1665:   201	        let Some(catalog) = self.taxonomy.as_ref() else {
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1671:   207	        catalog.validate_against_handlers(&handlers)?;
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1672:   208	        Ok(catalog.validate_catalog_against_handlers(&handlers))
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1815:     6	**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1825:    16	These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1833:    24	The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1919:   110	    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1924:   115	        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1929:   120	        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1937:   128	Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1990:   181	1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2008:   199	**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2021:   212	- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:6:**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:24:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:110:    catalog: &YamlTaxonomyCatalog,
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:128:Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:181:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:199:**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:212:- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:3:**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:86:    catalog: Arc<dyn TaxonomyCatalog>,
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:99:        catalog: Arc<dyn TaxonomyCatalog>,
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:169:- W1-B `YamlTaxonomyCatalog::description_for` (will move to trait per §4)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:170:- W1-B `Gateway::seal()`
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:198:- **AC-4 Boot logs.** Format: `mcp_v2 boot: pairing <client_id> verified, 0 handlers registered, 10 catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.` Strict mode env: `DAILYOS_MCP_V2_REQUIRE_HANDLERS=1` fails boot if no handlers registered.
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:205:  - `name` = catalog entry name (ScopedName)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:206:  - `description` = `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_not_to_call}")`
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:212:  - Subprocess negative tests: missing env → exit 1; unknown client_id → exit 1; revoked pairing → exit 1; keychain mismatch → exit 1; tools/call with replay of consumed nonce → BadParams (W1-A substrate via gateway)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:232:    - Integer edge cases (0, -0 rejection, max safe int, negative)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:263:- AC-12: subprocess negative tests per failure mode; env-wipe verified via in-process getenv (NOT /proc — acknowledged limitation)
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:284:- W1-B merged (PR #347): YamlTaxonomyCatalog + Gateway::seal
.docs/plans/wp-foundation-roadmap-reorientation.md:78:- **Audit + clean-machine validation**: DOS-575 negative fixture catalog, DOS-576 SurfaceClient audit attribution, DOS-577 clean-machine Studio validation (from v1.4.2 W5/W6).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:1:# DOS-478 — MCP v2 tool taxonomy + host-selection contract — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:3:**Wave:** v1.4.7 W1-B (parallel to W1-A)
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:4:**Lane spec:** [DOS-478](https://linear.app/a8c/issue/DOS-478)
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:5:**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W1-B — DOS-478"
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:17:   - `seal()` handler→catalog: stub set with EXTRA handler not in catalog → `Err(HandlerCatalogMismatch { handler, catalog_entry: None, nearest_candidate: Some(closest) })`
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:18:   - `validate_catalog_against_handlers(handlers)` catalog→handler: stub set missing one catalog tool → returns `Vec<ScopedName>{ missing_tool }` (NOT an error; production logs this)
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:34:   - `pub struct SelectionFixtures { positive: Vec<PromptFixture>, negative_broad_corpus: Vec<PromptFixture>, negative_adjacent_tool: Vec<AdjacentFixture> }` — separate type not in W0 contracts.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:39:2. **Bidirectional registry validation** (challenge HIGH AC-7). The W1-A `validate_against_handlers(handlers: &[...])` validates handler→catalog. Cycle-3 also adds `validate_catalog_against_handlers(handlers: &[...]) -> Vec<ScopedName>` returning catalog entries with no matching handler (NOT an error in production where W2-W4 handlers haven't shipped yet; emit operator-log warning so handler authors see what's pending). `Gateway::seal()` in production calls handler→catalog only (forward direction; an unknown handler is a typo). Test `dos478_taxonomy_catalog_test` uses BOTH directions on a stub set — handler→catalog detects extra/wrong handler; catalog→handler detects pending tools (logs, doesn't fail).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:41:3. **`HandlerCatalogMismatch.nearest_candidate` W1-A enum amendment** (architect). Cycle-3 additively extends the W1-A variant: `HandlerCatalogMismatch { handler, catalog_entry, nearest_candidate: Option<ScopedName> }`. Same shape-amendment precedent as W0 cycle-7 amendments (additive optional field; no consumers break). Update W1-A `taxonomy.rs` in this W1-B branch.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:43:4. **Product-vocabulary discipline in AC-4 + AC-6** (CEO NEEDS-CHANGES). Cycle-3 AC-4 + AC-6 forbid `dailyos.*` substrings AND system identifiers in `when_to_call` / `when_NOT_to_call` natural-language fields. Tool identifiers MAY only appear in structured `selection_fixtures.*.expected_tool` fields. Test asserts the natural-language fields contain no `dailyos.` substring + no `Read|SubmitCorrection|Write|McpClient|etc.` system terms.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:52:3. Fixture density (challenge + ceo) — ADR-0128 §B requires ≥ 2 positive + ≥ 2 negative per tool; fixed by AC-5 update + per-entry YAML fixture-stub fields
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:64:W1-B fills:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:66:1. **`src-tauri/resources/mcp_v2/tool_descriptions.yaml`** (NEW) — version-controlled catalog, parsed via `YamlToolEntry` DTO (NOT directly as `ToolDescription` — see cycle-3 fix #1). Entry shape:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:71:     when_to_call: |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:77:     when_NOT_to_call: |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:95:       positive:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:98:       negative_broad_corpus:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:101:       negative_adjacent_tool:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:104:   Note: natural-language fields (`summary`, `when_to_call`, `when_NOT_to_call`) MUST NOT contain `dailyos.` substrings or system identifiers (`Read | SubmitCorrection | Write | McpClient | ToolError | ...`). Tool identifiers appear only in structured `selection_fixtures.*.expected_tool` field (per AC-4/AC-6 cycle-3 #4). The sample above conforms.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:106:2. **`src-tauri/src/services/mcp_v2/taxonomy.rs` extensions** — W1-A shipped the trait; W1-B fills the loader implementation. Adds:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:116:   - `pub fn set_taxonomy(&mut self, catalog: Arc<dyn TaxonomyCatalog>)` — register the catalog reference
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:117:   - `pub fn seal(&self) -> Result<(), TaxonomyError>` — called by `main.rs` after `Gateway::register()` calls complete; invokes `catalog.validate_against_handlers(&self.handlers_as_slice())` and returns the error structured (NOT panic per cycle-1 devex resolution Q3)
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:120:4. **`src-tauri/tests/dos478_taxonomy_catalog_test.rs`** (NEW) — integration tests covering AC-1..AC-10.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:134:- **ADR-0128 §3 + §6 + cycle-7 §B amendment** — tool descriptions are product copy; cross-conversation continuity named as affordance; host-selection eval (DOS-481) requires ≥ 2 positive + ≥ 2 negative fixtures per tool with broad-corpus AND adjacent-wrong-tool negatives.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:145:The catalog ships entries for EXACTLY these 10 tool names. AC-1 enforces the count + identity:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:160:Note: pagination/list surface (DOS-172) is a discovery refinement applied across read tools, NOT a separate tool name. The frozen 10 is the surface for v1.4.7. Future tools amend the catalog at their own L0.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:164:- **AC-1 Inventory + load.** `YamlTaxonomyCatalog::load_embedded()` succeeds; resulting catalog has EXACTLY the 10 entries from §5 (assert by name + count). Extra or missing entries → test failure.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:166:- **AC-3 Scope mapping exact** (cycle-1 challenge #2). Each entry's `scopes_required` matches the §5 table verbatim. Test asserts per-tool: catalog scope set equals expected scope set (NOT just "in allowlist").
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:167:- **AC-4 Displacement framing in product vocabulary** (cycle-1 ceo + challenge convergent + cycle-2 ceo tightening). Every entry's `when_NOT_to_call` MUST contain BOTH (a) at least one broad-corpus keyword: `broad`, `enterprise search`, `web search`, `Glean`, `corpus`; AND (b) at least one adjacent-tool framing in PRODUCT LANGUAGE: `different surface`, `not for ... — use ... for`, `personal working`, or natural-language reference to the alternative. **AC-4 ALSO forbids**: substring `dailyos.` AND system identifiers (`Read|SubmitCorrection|Write|McpClient|ToolError|McpToolHandler|Gateway|Scope`) in `when_to_call` / `when_NOT_to_call` natural-language fields per cycle-2 CEO. Tool identifiers appear ONLY in structured `selection_fixtures.*.expected_tool` field.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:168:- **AC-5 Fixture coverage** (cycle-1 ceo + challenge convergent). Every entry has `selection_fixtures`: `positive` with ≥ 2 entries, `negative_broad_corpus` with ≥ 2 entries, `negative_adjacent_tool` with ≥ 1 entry (≥ 2 if the tool has an adjacent counterpart in §5; the `dailyos.write.place_document` lone-write tool may have only 1). DOS-481 W5-A will consume these fixtures to run the host-selection eval. `TaxonomyError::FixtureCoverage` on shortfall.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:169:- **AC-6 Continuity affordance in product vocabulary** (cycle-1 ceo #5 + cycle-2 ceo tightening). Every Read entry's `when_to_call` MUST mention continuity using one of: `cross-conversation`, `persists context`, `updates as work evolves`, `working understanding ... over time`. Per ADR-0128 §6 named-affordance contract. Same vocabulary discipline as AC-4 — no `dailyos.` substrings, no system identifiers.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:170:- **AC-7 Boot validation via `Gateway::seal()` returning `Result`** (cycle-1 architect #2 + devex #1; cycle-2 challenge + architect). `main.rs` calls `gateway.seal()?` after registration. Production `seal()` runs ONLY handler→catalog validation (an unknown handler is a typo blocking startup). Mismatch → `Err(TaxonomyError::HandlerCatalogMismatch { handler, catalog_entry, nearest_candidate: Option<ScopedName> })` per devex #4. No `panic!()`. Catalog→handler validation is a SEPARATE method `validate_catalog_against_handlers(handlers) -> Vec<ScopedName>` (returns pending tools; W2-W4 land them one-by-one); production logs the list as operator info, doesn't fail boot. **W1-A taxonomy.rs `HandlerCatalogMismatch` variant additively extended** with `nearest_candidate: Option<ScopedName>` field (this lane lands the additive amendment).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:171:- **AC-8 YAML hardening** (cycle-1 challenge #4). Loader rejects: duplicate `name` entries (`DuplicateName`), unknown YAML fields (serde deny_unknown_fields), empty `summary` / `when_to_call` / `when_NOT_to_call`. Per-failure test asserts the specific variant.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:174:- **AC-11 Required checks.** `cargo clippy --lib -- -D warnings && cargo test && pnpm tsc --noEmit` green. `cargo test dos478_taxonomy_catalog_test` passes (AC-1..AC-10 covered).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:176:## 7. Files owned (W1-B)
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:180:| `src-tauri/resources/mcp_v2/tool_descriptions.yaml` | NEW (10 entries) | exclusive |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:181:| `src-tauri/src/services/mcp_v2/taxonomy.rs` | W1-A shipped trait; W1-B adds variants + Display + YamlTaxonomyCatalog + load_embedded + load_from_path | shared (additive) |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:184:| `src-tauri/tests/dos478_taxonomy_catalog_test.rs` | NEW | exclusive |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:190:- **naming + side serialization** (AC-2): per-entry regex match; positive YAML `side: Submit` → ParseFailed.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:192:- **displacement framing** (AC-4): per-entry assert both keyword groups present in `when_NOT_to_call`.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:193:- **fixture coverage** (AC-5): per-entry assert positive ≥ 2, negative_broad_corpus ≥ 2, negative_adjacent_tool ≥ 1 (≥ 2 where adjacent exists).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:194:- **continuity affordance** (AC-6): per Read entry assert continuity keyword in `when_to_call`.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:195:- **boot validation Result** (AC-7): handler→catalog: stub set covering 10 → `seal()` returns `Ok(())`. EXTRA handler not in catalog → `seal()` returns `Err(HandlerCatalogMismatch { handler: <name>, catalog_entry: None, nearest_candidate: Some(<closest>) })`. **catalog→handler (separate method)**: stub set MISSING one catalog tool → `validate_catalog_against_handlers(handlers)` returns `Vec<ScopedName>{<missing>}` (NOT an error; logs pending; W2-W4 land incrementally).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:196:- **YAML hardening** (AC-8): duplicate-name YAML → `DuplicateName`; unknown top-level field → `ParseFailed`; empty-when_to_call YAML → `ParseFailed`; **nested unknown field on `SelectionFixtures` / `PromptFixture` / `AdjacentFixture` → `ParseFailed`** (cycle-4 #5 nested deny_unknown_fields).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:209:- Operator runbook for catalog edits — DX docs, ship with W1-B PR but as separate file.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:218:§6 AC-1..AC-11 met; L0 unanimous APPROVE; L2 unanimous APPROVE bounded by AC; commit-msg `L2-status: passed`; PR opens against `dev` after W1-A PR merges (sequential — W1-B depends on W1-A).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:16:3. **`services/mcp_v2/taxonomy.rs`** — actual `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` (architect cycle-6 LOW: catalog-neutral naming, NOT `HandlerYamlMismatch`) + ~130 lines of rustdoc with concrete Side::Read no-cursor example + concrete Side::Write required-cursor example + composite-mutation example + cursor-shape-per-service table (claim_id → CommittedClaim.new_claim_id at services/claims.rs:209; signal_id → emit* String return at services/signals.rs:46-106; action_id → services::actions::*). cargo check clean.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:62:Cycle-4 verdicts: codex challenge **BLOCK**; architect / CSO / devex **NEEDS-CHANGES**. **Unanimous (4/4)** on two issues: (i) §1 #1 still contradicted AC-2 on the absent-grant branch (3rd cycle hitting the same propagation gap); (ii) `ServiceMutationCursor` was claimed as landed but `rg` finds no such type in `contracts.rs` (and the substrate citations for `CommittedClaim.claim_id` and `services::signals::publish -> SignalId` were wrong — actual shapes are `CommittedClaim { claim, new_claim_id }` and `services::signals::emit*` returning `String`). 3/4 on namespace collision lint not credible (script absent, paths wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`, W1-B YAML doesn't exist yet). 2/3 on AC-12 ±5ms ceiling not portable on macOS/Linux CI.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:69:4. **Namespace collision CI REMOVED from W1-A scope** (3/4 convergent). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" (Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`). The CI lint requires W1-B YAML which doesn't exist yet AND clean reading of the legacy tool list at `src-tauri/src/mcp/main.rs:715-730` (challenge cycle-4 cited path is correct). Cycle-4 §7 + AC-10 reference + §11 entry all dropped.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:96:7. **Legacy coexistence interim guard** (CSO MED): **SUPERSEDED by cycle-5 #4** — the namespace collision CI is moved out of W1-A scope (script doesn't exist; cross-wave dep on W1-B YAML; path was wrong: `src/mcp/main.rs` should be `src-tauri/src/mcp/main.rs`). Filed as separate Linear ticket "v1.4.7 W1.5 — MCP v2 namespace collision CI" in Maintenance project. AC-10 records the spinoff.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:120:8. **W1-A/W1-B taxonomy seam restructured** (Class V — architect MED): W1-A defines the CATALOG INTERFACE in `taxonomy.rs` (validation function signature + binding contract). W1-B owns YAML LOADING + boot validation invocation. W1-A no longer dispatches off W1-B-owned YAML at runtime; the binding is purely a registry-time validation seam.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:152:5. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — W1-A defines the CATALOG INTERFACE only: `pub trait TaxonomyCatalog { fn validate_against_handlers(&self, handlers: &[&dyn McpToolHandler]) -> Result<(), TaxonomyError> }` + `TaxonomyError::HandlerYamlMismatch { handler: ScopedName, yaml: Option<ScopedName> }`. W1-B fills the YAML loader and invokes `validate_against_handlers` at boot.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:169:**W1-A does NOT ship:** per-tool handler bodies (handlers/*.rs placeholders untouched); tool taxonomy YAML (W1-B); MCP write allowlist extension (W4); new `Actor::McpClient` enum variant or `McpClientId` / `OpaqueConversationHandle` newtypes (W0 already shipped in `abilities-runtime/src/abilities/registry.rs:124,153,441`); new pairing transport design (mirrors surface_pairing).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:277:| `src-tauri/src/services/mcp_v2/taxonomy.rs` | W0 placeholder → W1-A trait + error type only | exclusive (interface only; W1-B fills loader) |
.docs/plans/v1.4.x-maintenance-waves.md:15:- **BLOCKED** — fix depends on v1.4.x substrate not yet landed (signal-policy registry W1-B, durable invalidation W1-C, ability-runtime W0.5, source taxonomy W1-A, etc.); cannot ship until that substrate closes.
.docs/plans/v1.4.x-maintenance-waves.md:35:| **SUBSUMED into v1.4.x** | ~8 | DOS-90, DOS-124 → W1-B; DOS-134 → W1-A (don't create 3rd DataSource); DOS-135 → W2-A amendment; DOS-377 → W8-C/DOS-348; DOS-516 → fold into W2-H/DOS-295; DOS-488 → fold into W2-B/DOS-214 |
.docs/plans/v1.4.x-maintenance-waves.md:36:| **BLOCKED on v1.4.x** | ~5 | DOS-126 (defer until W1-B lands), DOS-385 + DOS-517 (DOS-258 cutover not complete), DOS-87 (claim-native solution blocked), DOS-106 (`prep_frozen_at` reserved for AI snapshot freeze) |
.docs/plans/v1.4.x-maintenance-waves.md:182:| **A — DB-first JSON-on-disk** (parallel-safe) | DOS-80, DOS-102, DOS-111, DOS-84 | ORTHOGONAL with naming/conflict caveats. Coordinate migration numbers; add CI lint banning `read_dir`/`read_to_string` on `_today/data/**` outside allowlisted export modules. **DOS-111 SUBSUMED for skip-today via W1-B**; CONFLICTS for emails.json write-on-read (file path stale — actual offender is `services/entities.rs:399`, not `dashboard.rs`). |
.docs/plans/v1.4.x-maintenance-waves.md:191:2. DOS-111 skip-today: clean answer is W1-B signal policy registry; `app_state_kv` is acceptable interim with explicit follow-up.
.docs/plans/v1.4.x-maintenance-waves.md:202:| **DOS-90 + DOS-124** | Fold into v1.4.1 W1-B (DOS-235) as a paired `signal_key` schema. **W1-B has not yet shipped** — still BLOCKED. |
.docs/plans/v1.4.x-maintenance-waves.md:205:| **DOS-126** | Defer until W1-B lands; orthogonal but high merge-risk in `signals/` module. **Acceptance unreachable:** "under 1,000 lines" by extraction alone (~232 lines moved leaves rules.rs at ~1462) — needs clarification on test-move scope. |
.docs/plans/v1.4.x-maintenance-waves.md:212:- ⏳ W1-B (DOS-235) not yet shipped — DOS-90/124 + DOS-126 still BLOCKED until then
.docs/plans/v1.4.x-maintenance-waves.md:300:**Recommended sequence:** DOS-81 (independent perf, anytime) → DOS-108 (smallest, lowest risk; establishes `pollers/` module shape) → DOS-104 (reuses DOS-108's module shape via `poller_utils.rs`) → DOS-82 (most W1-B coupling; sequence after DOS-104 to avoid `executor.rs` merge churn).
.docs/plans/v1.4.x-maintenance-waves.md:306:| **DOS-104** | Path 2: shared `poller_utils.rs` for readiness/timing helpers reused by calendar + email. Reuses DOS-108's module shape. **Soft coupling to W1-B:** files `emit_or_log` migration as a separate W1-B follow-up; doesn't change signal semantics in this PR. |
.docs/plans/v1.4.x-maintenance-waves.md:307:| **DOS-82** | Path 3: W1-B-clean signature `sync_email_signals(ctx, db, engine, emails, domains)`; fall back to pure core + propagated wrapper if W1-B not yet merged. **Most W1-B coupling of the 4** — sequence after DOS-104. |
.docs/plans/v1.4.x-maintenance-waves.md:309:**W0.5 verification:** none of the 4 are pulled into `abilities-runtime` by DOS-349 — connectors/services/fetchers/background pollers stay in Tauri host crate. **W1-B coupling:** soft only on DOS-82 and DOS-104.
.docs/plans/v1.4.x-maintenance-waves.md:325:**Trigger:** **All of v1.4.1 W1 (W1-A, W1-B, W1-C, W1-D) and most of W2 (W2-A through W2-H) have shipped to dev.** Earlier classifications in this plan that called tickets BLOCKED on v1.4.x substrate are stale — the substrate is in. Re-evaluation needed across all SUBSUMED/BLOCKED entries.
.docs/plans/v1.4.x-maintenance-waves.md:332:| W1-B | DOS-235 signal policy registry | shipped (`418fd84c`, `signals/policy_registry.rs` exists) |
.docs/plans/v1.4.x-maintenance-waves.md:355:| **W1-B DOS-235 ship-on-top** | DOS-90, 124, 126 | 3 SHIP-ON-TOP. Refined acceptance criteria posted on each. **DOS-90 + DOS-124 share one `signal_key` migration — pair them.** DOS-126's merge-risk blocker dissolved; "<1,000 line" target flagged for owner sanity check (acceptance unreachable by extraction alone). | none |
.docs/plans/v1.4.x-maintenance-waves.md:376:| **DOS-90** | BLOCKED on W1-B | **ORTHOGONAL ship-on-top.** W1-B's policy registry shipped *without* the `signal_key` dedup column. The substrate is in; the dedup feature is the next step. Migration to add `signal_key` + unique constraint + upsert path can land on top of W1-B's policy routing. |
.docs/plans/v1.4.x-maintenance-waves.md:377:| **DOS-124** | BLOCKED on W1-B | **ORTHOGONAL ship-on-top.** Same as DOS-90 — pair these. The indexed dedup on `signal_events` extends W1-B's coalescing pattern (W1-C's `coalescing_key` on `invalidation_jobs` is a reference shape). |
.docs/plans/v1.4.x-maintenance-waves.md:378:| **DOS-126** | BLOCKED on W1-B | **ORTHOGONAL.** W1-B's restructuring of `signals/` is in; the merge-risk concern is resolved. Still need owner ruling on the "under 1,000 lines" acceptance (acceptance unreachable by extraction alone — clarify whether tests count). |
.docs/plans/v1.4.x-maintenance-waves.md:404:| Signal emission goes through `signals/policy_registry.rs`; no direct `db.execute` on signal events | v1.4.1 W1-B | Maint-W8 (DOS-90/124/134/135 fold-in), Maint-W7 (DOS-111 skip-today) |
.docs/plans/v1.4.x-maintenance-waves.md:427:  Maint-W8 (signals substrate) — fold into v1.4.1 W1-A/W1-B/W2-A
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:14:Closure location in v2: §3 "Mutation catalogue from `rg` audit" and §3 "`emails.rs` | `get_emails_enriched:SIG+FS`; ... `archive_email:D+SIG+EXT`; ... `retry_failed_emails:D+BG`"
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:15:Verification: v2 now has a real table and no "..." catch-all. It is not actually exhaustive. The current code has mutation-looking functions omitted from the table, including `emails::unarchive_email` at `src-tauri/src/services/emails.rs:1124` (`db.unarchive_email` + Gmail external side effect), `emails::unsuppress_email` at line 1181 (`db.unsuppress_email`), `emails::pin_email` at line 1186 (`db.toggle_pin_email` + signal), `accounts::snooze_triage_item` at `src-tauri/src/services/accounts.rs:1941`, and `entity_linking/rules/p2_thread_inheritance.rs:19` (`db.enqueue_thread_inheritance`). Mentioned with a table is not closed if the catalogue misses live mutators.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:38:Closure location in v2: §9 "Mandatory tests and commands:" and §9 "`services::context::tests::proptest_check_mutation_allowed_modes`: generator is `prop_oneof![Live, Simulate, Evaluate]` crossed with `MutationAttempt { kind: D|SQL|TX|SIG|FS|BG|EXT, public_catalog_index, uses_clock, uses_rng }`"
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:39:Verification: v2 makes the test names mandatory, specifies concrete generators, adds catalogue coverage, and lists exact clock/RNG lint regexes. It remains partial because the binding CI command in §9 narrows execution to `cargo test ... dos209`, while both DOS-209 acceptance and the W2 merge gate require the full `cargo test` suite to be green after this broad service migration.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:47:### NF1 — Exhaustive catalogue claim is false against current services tree (severity: Critical)
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:48:v2 over-corrects F2 by claiming a mechanical `rg` catalogue, but the catalogue misses live mutators. The omissions are not edge trivia: `unarchive_email`, `unsuppress_email`, `pin_email`, `snooze_triage_item`, and the P2 thread-inheritance enqueue path are exactly the DB, external, signal, and queue side effects DOS-209 is supposed to gate.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:49:Location: §3 "Mutation catalogue from `rg` audit" and §3 "`emails.rs` | `get_emails_enriched:SIG+FS`; ... `archive_email:D+SIG+EXT`; ... `retry_failed_emails:D+BG`"
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:50:What needs to change: Re-run the audit from the current tree, include every omitted mutator or explicitly justify exclusions under the taxonomy, and name the audit script/source that generates `dos209_mutation_catalog.rs` so the plan table and CI catalogue cannot drift.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:64:v2 closes most cycle-1 shape questions, but the remaining failures are substantive: the mutation catalogue is not exhaustive, sequencing contradicts the frozen DOS-209 contract, and the mandatory CI evidence is narrower than the wave gate. This is a cycle-2 REVISE outcome, so the correct verdict is ESCALATE-TO-L6, not a third plan revision.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:67:- ESCALATE-TO-L6: L6 must decide whether to override DOS-209's landing order and whether the plan author gets another catalogue repair pass despite the two-cycle cap.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-challenge-v2.md:71:Per the 2-revision-cycle hard cap: REVISE on cycle 2 escalates to L6, not a third revision attempt. These are not minor polish notes: the catalogue omission directly risks shipping ungated mutations, and the W2-B-first order is a frozen-contract conflict that the author cannot resolve unilaterally.
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md:121:  - `bypass_applies_only_to_local_to_local_single_user_read` (positive)
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md:123:  - `local_to_local_write_category_still_gates` (positive — Write/Maintenance/Publish/Transform unaffected)
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md:124:  - `default_granted_scopes_auto_derives_from_registry` (positive)
.docs/plans/trust-topology-realignment/L0-packet-B-substrate-trim.md:143:4. **Read-bypass positive test passes.** Named test `bypass_applies_only_to_local_to_local_single_user_read` exists and asserts the four cells of (topology × category) — only `LocalToLocalSingleUser × Read` bypasses.
.docs/plans/v1.4.6-w0-carveout-probe-2026-05-19.md:47:~3-5 days for one agent (DTO transcription + module skeleton + migration + tests). Empty placeholder files for `feedback.rs` / `boundary.rs` / `privacy.rs` / `contradiction.rs` / `render_rules.rs` (filled later by NEW v1.4.4 W1-B/C/D when surfaces are scoped).
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md:19:Verification: v2 replaces the prior n/a with all five answers: mode-aware signal routing to the ADR-0115 in-memory ring buffer, deterministic health scoring/debouncer via injected clock/RNG, future context-builder propagation, no briefing callouts, and feedback mutations included in the guard catalogue.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md:29:The revised plan closes all four cycle-1 findings with concrete contract text, construction surfaces, capability boundaries, wave gates, and named tests. Fresh review found enough implementation shape for coding: context carriers and visibility, transaction API/fallback, mutator catalogue, error variants, caller migration, capability compile-fail tests, runtime boundary tests, and CI lint seams are all specified.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v2.md:32:Confidence comes from the concrete W2-A implementation spine in `DOS-209-plan.md` §2-§4 and §9: `services/context.rs` ownership, exact carrier/visibility rules, `ServiceError` variants, `with_transaction_async` signature plus fallback, exhaustive mutator catalogue, surface-constructor test, mutation-catalog structural lint, mode-boundary runtime test, clock/RNG lint, and capability trybuild fixtures.
.docs/plans/v1.4.6-waves.md:11:**Substrate work** in this version is unchanged and proceeds per the L0-approved plan below: W1-A RecommendationClaim variant + `CLAIM_TYPE_REGISTRY` extension, W1-B salience scoring engine, W2-A surfacing policy, W2-B trigger policy, W4-A feedback loop, W4-C engagement telemetry, W5-A eval harness. All surface-agnostic.
.docs/plans/v1.4.6-waves.md:77:1. Cycle 2 #7 inline comment at `v1.4.6-waves.md:112` (`// Omitted-factor behavior: factor with value None is treated as factor_value = 0.0 (NOT skipped)`) is wrong; cycle 3 W1-B Class B already corrected to skip-from-average. Cycle 4 fixes the salience ability schema text to match: omitted factors skipped from BOTH numerator and denominator (true average over present factors); empty-factor edge case → `total = 0.0`. Comment at line 112 reworded to: `Omitted-factor behavior: factor with value: None is skipped from BOTH numerator and denominator (true weighted average over present factors); empty-factor edge case → total = 0.0`.
.docs/plans/v1.4.6-waves.md:116:7. W1-B aggregation formula spelled out in cycle 3, but no numeric weights and no tie-break rule. Cycle 4 adds to W1-B done-when:
.docs/plans/v1.4.6-waves.md:124:   W1-B owned-files migration v262–v264 includes the weight table as a seed row in `salience_factors_weights` (or hardcoded const if storage redundant; W1-B L0 plan picks). Per-user weight tuning (W4-A feedback loop) is layered on top, default weights ship at W0 close.
.docs/plans/v1.4.6-waves.md:139:- Cycle 2 #7 said `total = weighted sum (per W1-B aggregation formula)` but no formula in W1-B body. Add to W1-B done-when:
.docs/plans/v1.4.6-waves.md:145:- W1-B owned files list (`v1.4.6-waves.md:393-417`) explicitly adds `abilities-runtime/src/abilities/recommendations/salience.rs` (the ScoreSalience ability surface) alongside `services/recommendations/salience.rs` (the engine). Schema-version reject behavior: response includes `schema_version: 1`; future versions break by mismatch, callers MUST check.
.docs/plans/v1.4.6-waves.md:213:5. Per-file owner table now explicit: `recommendation.rs` → W1-A (RecommendationClaim impls); `salience.rs` → W1-B; `surfacing.rs` → W2-A; `triggers.rs` → W2-B; `feedback.rs` → W4-A; `deviation.rs` → W4-B; `engagement.rs` → W4-C; `render.rs` → W3-A (Suggested Next Steps backend); `eval.rs` → W5-A.
.docs/plans/v1.4.6-waves.md:219:7. v1.4.7 forward-coupling clarified: v1.4.7 W2-C `dailyos.read.portfolio_attention` says "consume v1.4.6 salience ability when v1.4.6 ships, OR implement recency-only ranking" — v1.4.6 commits to **producing the salience ability with stable schema by W2-A merge**, so v1.4.7 W2-C can consume without amendment. Salience ability contract added to W1-B done-when:
.docs/plans/v1.4.6-waves.md:234:       pub salience: SalienceScore,                   // total + Vec<SalienceFactor>; total = weighted sum (per W1-B aggregation formula)
.docs/plans/v1.4.6-waves.md:337:The other constraint is **stay quiet by default**. Surfacing budget caps daily/per-tier/per-entity volume. Cooldowns prevent same-claim resurfacing. Quiet/background tiers exist for items that scored salient but didn't clear the surfacing threshold. The eval harness explicitly tests "missed-important" cases (false negatives) alongside "noisy" (false positives) so the budget doesn't optimize for engagement.
.docs/plans/v1.4.6-waves.md:438:   - W1-B (DOS-330 salience scoring): consumes ADR-0074 vector search + ADR-0078 embed model for similarity / corroboration factors; ADR-0105 trust + freshness for trust factor; ADR-0080 signals for novelty + urgency factors. Does NOT introduce new primitives.
.docs/plans/v1.4.6-waves.md:490:| Salience score is computed from explicit inspectable factors, NOT ad-hoc LLM ranking judgment | `salience_factors` table writes per claim with one row per factor; UI render shows factor breakdown on demand; CI test asserts no LLM ranking call sites in `services/recommendations/salience.rs` | W1-B merge |
.docs/plans/v1.4.6-waves.md:531:        pub salience: SalienceScore,                  // see W1-B
.docs/plans/v1.4.6-waves.md:705:### Agent W1-B — DOS-330: Build salience scoring engine with explicit factors
.docs/plans/v1.4.6-waves.md:710:  - `src-tauri/src/services/recommendations/salience.rs` (placeholder from W1-A; W1-B fills) — `compute_salience(claim) -> SalienceScore`, `compute_factor(kind, claim) -> SalienceFactor`, factor weight table.
.docs/plans/v1.4.6-waves.md:711:  - `src-tauri/abilities-runtime/src/abilities/recommendations/salience.rs` (cycle 3 Class B: W1-B owns the ScoreSalience ability surface alongside the engine; matches cycle 2 #7 contract).
.docs/plans/v1.4.6-waves.md:737:  Stored as seed rows in `salience_factors_weights` (migration v262–v264) OR as `const` table in `services/recommendations/salience.rs` (W1-B L0 plan picks based on whether per-user tuning ships in v1.4.6; W4-A feedback-loop weight tuning currently expected post-v1.4.6).
.docs/plans/v1.4.6-waves.md:738:- **Tie-break rule (cycle 4 fix G):** when two `salience.total` values are equal, sort by: (1) higher `Importance` factor value, (2) higher `Urgency` factor value, (3) older (smaller) `ClaimId` lexical order. Deterministic across runs. W1-B test case: identical-total fixture pair sorts identically every run.
.docs/plans/v1.4.6-waves.md:765:- L0 plan approvals: architect-reviewer + `/plan-devex-review` for W1-A; architect-reviewer for W1-B.
.docs/plans/v1.4.6-waves.md:784:- **Don't touch:** Salience scoring (W1-B); trigger policy (W2-B); rendering (W3); `signals/policy_registry.rs` (**W0 pre-declares both v1.4.6 SignalType variants per cycle 4 fix C**; supersedes cycle 2 #10 W1-A ownership).
.docs/plans/v1.4.6-waves.md:805:- **Don't touch:** Surfacing decision (W2-A); salience computation (W1-B — consumed read-only); other recommendations submodules.
.docs/plans/v1.4.6-waves.md:810:  1. *Claim model:* Read-only; trigger fires salience re-compute via W1-B.
.docs/plans/v1.4.6-waves.md:812:  3. *Signals + invalidation:* `SalienceCandidateRefreshTriggered` emitted; consumed by W1-B salience re-compute path.
.docs/plans/v1.4.6-waves.md:905:- **Don't touch:** Salience scoring (W1-B — deviation is a Novelty factor INPUT, not a parallel scoring path); other submodules.
.docs/plans/v1.4.6-waves.md:936:  3. *Signals + invalidation:* No signals from engagement directly; feeds into salience re-compute via W1-B.
.docs/plans/v1.4.6-waves.md:964:  - **Missed-important:** false-negative test cases (engagement is NOT the optimization target — must catch important items even when prior user behavior would suggest filtering)
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-codex-challenge.md:23:- Recommendation: make the harness fixture schema-based, not substring-based: require JSON pointers with expected `ValueKind`, `required` vs `optional`, expected diagnostics/audits, expected renderer branch, and exact block wrapper/data attributes. Add negative fixtures for field rename, string↔number drift, required-to-optional drift, optional-to-required drift, and `payload.text`→`payload.body` nesting drift.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-codex-challenge.md:93:- Location: packet CLI responsibilities at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:72`; partial-state exit code at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:79`; negative fixtures at `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:281`.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-C-codex-challenge.md:94:- Observation: the CLI can exit 2 with "manual cleanup needed" (`.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:79`), but no negative fixture asserts what files may exist after a failed scaffold or how the developer detects stale generated state.
.docs/plans/wave-W6/DOS-320-plan.md:80:W1-B universal write fence is honored by construction: W6-D performs no DB writes, file writes, service calls beyond existing reads, signal emissions, claim updates, or projections.
.docs/plans/wave-W5/_prompts/_common.md:74:What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
.docs/plans/_patterns/two-column-policy.html:10:  <!-- Left column: positive list -->
.docs/plans/_patterns/two-column-policy.html:23:  <!-- Right column: negative list -->
.docs/plans/wave-W4/DOS-216-plan.md:14:All four 2026-04-24 amendments apply, with the last one setting W4's blocking slice. The production-readiness addendum requires Golden Daily Loop/user-loop fixture support and says "Golden Daily Loop fixture subset runs as a release-check command." The catalogue addendum says "report coverage by scenario bundle, not only by ability" with metadata such as `scenario_id`, `invariant`, `expected_render_policy`, and `surfaces_exercised`. The prompt/eval addendum says the harness should measure "query count, latency, repair fanout, cache hit rate" and scoped correction behavior. The plan-hardening amendment narrows W4: "The eval harness should land minimal replay-first infrastructure before broader capture/anonymization/live-judge work" and names one Read fixture, one Transform fixture, replay-only provider, exact output diff, provenance/source-warning diff, fixture loading from bundle directories, and hermetic CI as the first landing bar.
.docs/plans/wave-W4/DOS-216-plan.md:24:Add fixture directories under `src-tauri/tests/abilities/{ability}/fixtures/fixture_N/` with the Linear/ADR required files. Keep the ADR flat file names as required by ADR-0110 (`.docs/decisions/0110-evaluation-harness-for-abilities.md:22-39`), and add one per-fixture `metadata.json` only for the 2026-04-24 catalogue fields and anonymization certificate. Do not replace `inputs.json` with an `inputs/` directory or `expected_output.json` with an `expected_output/` directory unless L0 reviewers amend the ticket. Ability-level metadata stays in `fixtures/manifest.toml`, `evals/quality.toml`, and `evals/regression_baseline.json` per ADR-0110.
.docs/plans/wave-W4/DOS-216-plan.md:59:Fixture format: pick ADR/Linear flat required files plus additive `metadata.json` and `expected_state.json`. Reason: ADR-0110's fixture shape is concrete (`state.sql`, `inputs.json`, replay files, clock/seed, expected artifacts), while the 2026-04-24 catalogue amendment needs per-fixture scenario fields not covered by `expected_output.json`. `metadata.json` holds `bundle`, `labels`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, `source_lifecycle_refs`, `anonymization_cert`, `retention_policy`, `prompt_fingerprint_baseline`, optional `post_action_state`, and the surface selector for `expected_internal_provenance` versus `expected_rendered_provenance`. `expected_state.json` owns post-run DB assertions such as trust state; `expected_provenance.json` owns only W3-B provenance; `expected_output.json` owns rendered ability output. Cross-entity state stays in `state.sql`, using the real schema fields/tables such as `accounts.parent_id`, `account_domains`, aliases/DBA names, and portfolio-domain relationships.
.docs/plans/wave-W4/DOS-216-plan.md:111:W1-B universal write fence is honored: DOS-216 does not write `intelligence.json`, claim rows, projections, signals, or external systems. Test output goes to temp dirs/in-memory DBs and optional CI artifacts. Capture mode writes checked-in fixture candidates only by explicit developer command, not during app runtime or normal CI.
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult.md:26:Location in plan: §7 "If a metadata-row helper is unexpectedly required, pick up after W1-A migration N and W1-B migration N+1, so W2 starts at N+2."
.docs/plans/wave-W1-v147/proof-bundle.md:88:- UNIQUE(nonce, client_id) enforced (negative test)
.docs/plans/wave-W1-v147/proof-bundle.md:105:6. **MCP v2 namespace collision CI** — drops per cycle-5 L0 fix; filed as W1.5 ticket; requires W1-B YAML.
.docs/plans/wave-W1-v147/proof-bundle.md:124:2. **L3 wave gate** — runs after W1-B (DOS-478) also merges; codex-challenge + architect-reviewer + Suite S/P/E on integrated W1 state
.docs/plans/wave-W1-v147/proof-bundle.md:125:3. **W1-B (DOS-478)** — tool taxonomy YAML lane; L0 packet draft in progress in parallel
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v3.md:9:### NF1 (catalogue + audit script) — landed cleanly: yes
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v3.md:10:The L6 packet authorized only a script-based re-audit, committed catalogue output, and a no-drift CI test for DOS-209 (escalation lines 69-72, 104-106). Plan v3 adds that binding in §2/§3/§9: the PR must include `src-tauri/tests/dos209_mutation_catalog.rs` generated from the audit script (line 92), the catalogue is generated by `scripts/dos209-mutation-audit.sh` and checked against `src-tauri/tests/dos209_mutation_catalog.txt` (lines 108-109), and `mutation_catalog_no_drift` re-runs the script and asserts exact snapshot match (line 399). I also verified the script prints the deterministic catalogue (script lines 288-294), the script output matches the snapshot, and the five cycle-2 misses are now present in the plan/snapshot: `accounts::snooze_triage_item` (plan line 133), `emails::{unarchive_email,unsuppress_email,pin_email}` (lines 180-183), and `entity_linking::rules::p2_thread_inheritance::evaluate` (line 209).
.docs/plans/wave-W2/reviews/DOS-209-l0-codex-consult-v3.md:21:The L6-authorized DOS-209 scope was exactly catalogue/audit/no-drift, landing-order reconciliation, and full-suite CI restoration (escalation lines 67-72, 98-106; Linear 2026-04-29 decision comment decisions 1, 3, 4, 5). Plan v3's own revision history lists only those three closures (line 5), and the new material I found maps to those items: amendment text in §1/§7, catalogue/audit text and generated rows in §2/§3/§9, and the restored CI command in §9. The cycle-2 APPROVE conditions remain intact: DOS-304 capability-boundary handling is still present (plan line 78), caller construction surfaces remain explicit (lines 84-90), Intelligence Loop answers remain concrete (lines 364-369), the W1 hard pre-code gate remains binding (line 375), and the implementation spine I approved in cycle 2 remains present across `ServiceContext` carriers, visibility, errors, transactions, tests, and CI (cycle-2 review lines 28-32; plan lines 82-104, 391-404).
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-cso.md:19:| 2 | Cache lookup keyed by `current_db_version`, not `request.composition_version` | OK — `mod.rs:2336-2352` reads `current_composition_version_for_composition_id` from substrate; `:2361` uses `current_db_version`. Grep-gate test `dos671_project_composition_cache_lookup_current_db_version_grep_gate` enforces both positive and negative assertion. |
.docs/plans/wave-W6/DOS-283-plan.md:9:DOS-283 turns mock data into an executable fixture product for v1.4.0 validation, not a demo workspace. Load-bearing ticket lines: "Mock data is too often happy-path demo material." "Create a seeded v1.4.0 mock workspace that powers unit fixtures, ability evals, validation tests, and manual QA for the Golden Daily Loop." Acceptance requires "both positive and negative canonicalization cases", "stale vs current source timestamp cases", "at least one user correction/tombstone", "project detail parity cases", and documentation mapping "every seeded edge case to a test or validation assertion."
.docs/plans/wave-W6/DOS-283-plan.md:11:All 2026-04-24 amendments apply, narrowed to this W6-A slice. The hostile-workspace comment requires "two accounts sharing the same domain but different business units", "a prior user correction removing a wrong link/claim, which must survive refresh", and data that makes "cross-account bleed easy to reproduce and impossible to ignore." The catalogue comment says the workspace must seed eight bundles and that "Mock data that only looks realistic is insufficient." The feedback addendum requires claims across daily briefing, meeting briefing, account detail, and project detail, including a "recent but wrong-account claim" and "expected repaired display states." The plan-hardening amendment is the W6-A contract: "mock data is not just seeded demo richness; it is executable fixture input with expected assertions" and the minimum first bundles are "bundle 1: cross-entity / same-domain ambiguity" and "bundle 5: Golden Daily Loop correction resurrection."
.docs/plans/wave-W6/DOS-283-plan.md:13:The wave doc scopes W6-A to bundle files: `src-tauri/tests/fixtures/bundles/`, README row/source mappings, W3 schema, W4 harness, and done-when "bundles loadable via `cargo test`; manual QA load works in dev workspace" (`.docs/plans/v1.4.0-waves.md:606-612`). Suite E makes bundles 1+5 mandatory through W6 and blocking at ship (`.docs/plans/v1.4.0-waves.md:175-183`). This plan does not create other catalogue bundles.
.docs/plans/wave-W6/DOS-283-plan.md:113:Rollback is remove or skip the bundle fixture from the W4 harness manifest while preserving the failure report. No production migration or irreversible data rewrite exists. W1-B universal write fence is honored: W6-A does not write `intelligence.json`; any later legacy file projection remains DOS-301's responsibility through `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:223-238`). Schema epoch/drain primitives stay untouched (`src-tauri/src/intelligence/write_fence.rs:67-155`).
.docs/plans/_archive/v0.15.1-shipped.md:75:3. If workspace is not under iCloud scope (e.g., `~/DailyOS`): the warning never appears. No false positives.
.docs/plans/wave-W4/DOS-5-plan.md:37:Malformed config is rejected, not defaulted: factor values must be finite; weights must be finite and `>= 0`; at least one weight must be positive; and the geometric-mean denominator must be positive. Violations fail boot/tests before any score is produced.
.docs/plans/wave-W4/DOS-5-plan.md:75:All existing `FinalizeMode::ManualRefresh` and `FinalizeMode::QueueWorker` callers remain non-triggering. Current production calls at `src-tauri/src/services/intelligence.rs:792` and `src-tauri/src/intel_queue.rs:1002`, plus existing finalize tests around `services/intelligence.rs:3404-3492`, continue to pass existing variants and must get a negative test proving they do not recompute trust. Only a new caller that explicitly passes `FinalizeMode::TrustRecompute` triggers trust recompute.
.docs/plans/wave-W4/DOS-5-plan.md:79:End-state alignment: W4-A turns trust from an absent/zero placeholder into deterministic per-claim substrate for W6-A rendering and W5 ability outputs, while retiring the false-positive-prone binary contamination write gate. It forecloses row-creation-age freshness, six-canonical-factor drift, W3-C calling W4-A, bridge-coupled recompute, and env-var-driven hard rejection in enrichment.
.docs/plans/wave-W4/DOS-5-plan.md:106:Backfill/shadow run keeps DOS-5's stated <=30 min for 100K claims and distribution checks: no NaN, no negatives, <=5% at floor/ceiling. Floating point code uses `f64`, clamps before `ln`, and property-tests 10K random tuples.
.docs/plans/wave-W4/DOS-5-plan.md:135:If a migration/projection from W3 is running, W4-A honors W1-B/W3 write fence by running recompute after DOS-7 cutover and through service writes only. `FenceCycle` captures/rechecks `schema_epoch` at `src-tauri/src/intelligence/write_fence.rs:67-109`, drain/bump primitives at `:112-155`; W4-A does not bypass them.
.docs/plans/wave-W4/DOS-5-plan.md:138:Unit tests: `trust_geometric_mean_all_floor_05`, `trust_geometric_mean_all_one`, `trust_geometric_mean_mixed_08`, `trust_feedback_boost_clamped_to_ceiling`, `trust_contradiction_present_downranks`, `trust_nan_never_emitted_for_random_factor_tuples`, `trust_rejects_non_finite_factor_config`, `trust_rejects_non_finite_or_negative_weights`, `trust_rejects_zero_positive_weight_denominator`, `trust_factor_count_is_five_canonical_plus_local_helpers`, `freshness_context_timestamp_unknown_applies_penalty`, `freshness_context_timestamp_known_uses_source_asof_age`.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-code-reviewer.md:37:| 4 | cache lookup keyed by `current_db_version` | Rust grep gate `dos671_project_composition_cache_lookup_current_db_version_grep_gate` at `mod.rs:5465-5485` — positive + negative assertions both present. |
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-L2-code-reviewer.md:90:4. **Scope-creep observation (not a finding).** Commit 1 includes `check_claim_writer_allowlist.sh` `W6-A-meta-*` extension + `linear_issue_signals.rs` + `v178` migration guard. None of these are L0 §5 work for Packet B. They appear to be local-worktree validation-gate enablers per commit message. The migration guard and signals refactor are net-positive; the W6 fixture allowlist looks like cross-branch contamination. Author should confirm provenance before merge; no action required from L2 reviewer.
.docs/plans/wave-W5/DOS-218-plan.md:96:W1-B universal write fence is honored by construction: this Read ability performs no DB writes, file writes, signal propagation, queueing, projection, claim writes, or external calls. Evaluate fixtures run in-memory through W4-B and cannot fall through to Live providers.
.docs/plans/wave-W4/DOS-383-plan.md:114:W1-B universal write fence honored: replay layer never writes to DB, files, signals, or external systems.
.docs/plans/wave-W5/proof-bundle.md:108:- **Bundle-coverage expansion** — W5 ships with bundles 1, 5, 9-13. Other scenarios (multi-thread, stale Glean propagation across multiple meetings, etc.) belong in a follow-up bundle-catalog ticket.
.docs/plans/v1.4.0-waves.md:137:What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
.docs/plans/v1.4.0-waves.md:156:| Migration / projection / hot-path (W1-B, W3-C, W3-D, W4-A) | performance-engineer |
.docs/plans/v1.4.0-waves.md:376:| Tune reviewer prompts (e.g., `/codex challenge` phrasing) | Mid-wave allowed if false-positive rate is high | Document in retro |
.docs/plans/v1.4.0-waves.md:426:### Agent W1-B — DOS-311 migration fence + schema epoch + universal write fence
.docs/plans/v1.4.0-waves.md:436:- L0 plan approvals: both agents (architect-reviewer + performance-engineer for W1-B; architect-reviewer for W1-A)
.docs/plans/v1.4.0-waves.md:502:- **Depends on:** W0-A (DOS-309 dismiss fix), W1-A (per-entity invalidation primitive), W1-B (write fence), W3-DOS-308 (precondition: design contract + audit script + quarantine table + zero-quarantined-rows migration gate) — **all must be merged before this branch opens.**
.docs/plans/v1.4.0-waves.md:505:- **Done when:** 9-mechanism merge rules unit-tested per disposition table in DOS-7; hard-delete CHAIN refactored + lint rejects bare DELETE; backfill runs under W1-B fence with reconcile pass reporting zero ghost-resurrections on dev data; ghost-resurrection regression test green per mechanism; immutability allowlist CI lint rejects forbidden UPDATE columns; no-DELETE invariant test green.
.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:95:    + exact block wrapper/data attributes. Adds explicit negative-fixture
.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md:607:2. **Integration test harness catches contract mismatch.** A negative fixture where the producer emits `payload.text` but the projection rule binds `payload.body` → harness detects mismatch + emits a DOS-670-style diagnostic ("producer/projection contract mismatch: declared field `payload.body` not found in producer output").
.docs/plans/wave-W5/DOS-219-plan.md:102:W1-B write fence is honored because W5-B never writes `intelligence.json` directly. Legacy file projection, if any, goes through DOS-301 and `fenced_write_intelligence_json`.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-code-reviewer.md:137:write). The fixture list §8 #2/#3 covers the negative; add a positive
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-code-reviewer.md:258:| 2-4 (editor reload) | covered by F4 fold | add positive "first-mount fires reload" fixture |
.docs/plans/wave-W6/DOS-288-plan.md:71:Services-only mutations: W6-B writes no production state. `ownership.rs` is pure validation; `dos288_bleed_detection_test.rs` loads in-memory fixture SQL; no command handler, service mutation, signal emission, queue wake, `intelligence.json` write, or claim commit is introduced. The W1-B write fence is therefore honored by construction.
.docs/plans/dos-546/phase-0/04-runtime-host-inventory.md:5:open_questions: see ./INDEX.md (routed to W2-A and W1-B L0 Prep)
.docs/plans/wave-W6/DOS-281-plan.md:15:All 2026-04-24 amendments apply. The 12:44 tightening makes cross-entity/content bleed a hard invariant. The 12:52 catalogue amendment requires bundle coverage with seeded data, regression test, eval fixture, render behavior, and lint/activity-log expectations. The 13:08 addendum adds DOS-294/DOS-295 blockers and explicit feedback scenarios: outdated, wrong entity, wrong source, unverifiable, and partly right. The 19:30 plan-hardening amendment says "This issue should not remain only a checklist. It needs a runnable release command for the v1.4 subset, starting with bundle 1 and bundle 5, plus a separate manual evidence artifact for real-dev meetings / seven-day parallel runs" and separates hermetic CI from real-dev release evidence.
.docs/plans/wave-W6/DOS-281-plan.md:100:Rollback is migration-free: remove the `release-gate` bin entry, package script, and tests; no schema, projections, claims, signals, or source data are touched. W1-B universal write fence is honored because the binary never writes `intelligence.json`, claim rows, legacy projections, or external systems; all durable output is under `src-tauri/target/release-gate/`.
.docs/plans/wave-W4/DOS-217-plan.md:40:End-state alignment: this makes the W3 registry the only operation catalog across app, MCP, worker, and eval. It forecloses a second MCP/Tauri operations registry, actor-selected capability handles, and surface code that bypasses schema/provenance policy.
.docs/plans/wave-W4/DOS-217-plan.md:112:Rollback is mechanical and migration-free: remove `invoke_ability` from `generate_handler!`, stop constructing bridges, and let current hand-written commands/static MCP tools continue during Phase 2. W1-B universal write fence is honored because W4-C adds no direct write path; all writes remain inside abilities/services and the existing fence (`scripts/check_write_fence_usage.sh:1-22`).
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-challenge.md:20:  The repo already has a negative fixture showing `"User interaction is not allowed"` is not not-found (`src-tauri/src/db/key_provider.rs:1231-1240`).
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-challenge.md:97:  The negative fixture list includes mocked spawn failure and corrupt payload, but only "documented item-not-found" for real CLI classification (`.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:243-250`).
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:5:**Authorized cycle-3 scope:** NF1 (catalogue) + NF2 (landing order) + NF3 (CI command) only
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:25:Verification command: `bash scripts/dos209-mutation-audit.sh > /tmp/dos209-audit-fresh.txt && diff /tmp/dos209-audit-fresh.txt src-tauri/tests/dos209_mutation_catalog.txt`. Result: exit 0, zero-byte diff. Both files are 233 lines. The committed snapshot is a faithful, reproducible product of the committed script against the current `src-tauri/src/services/` tree.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:35:All five cycle-2 NF1 omissions are now catalogued with kind classifications consistent with the cycle-2 challenge's diagnosis (DB + external + signal + queue side effects).
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:38:Test name: `services::context::tests::mutation_catalog_no_drift`.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:39:Quoted spec from §9: "re-runs `scripts/dos209-mutation-audit.sh` and asserts stdout exactly matches `src-tauri/tests/dos209_mutation_catalog.txt`; any drift breaks CI. This closes cycle-2 challenge NF1." This is a concrete, named, mandatory test bound by the §9 commands list.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:65:- §2 Approach: unchanged from v2 (caller construction paths, single-PR migration, no feature flag, `dos209_mutation_catalog.rs` reference).
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:66:- §3 Key decisions: catalogue table is now generated from `scripts/dos209-mutation-audit.sh` (NF1) and committed as `dos209_mutation_catalog.txt`. The taxonomy paragraph, ServiceContext visibility paragraph, ServiceError paragraph, transaction-API paragraph, plan-set deferral paragraph, DOS-304 handling paragraph, and CURRENT_TIMESTAMP audit paragraph are unchanged from v2. The catalogue's textual content is the script's deterministic output, not author re-curation. Authorized (NF1).
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:72:- §9 Test evidence: NF3 CI command repair plus the new `mutation_catalog_no_drift` test (NF1). Other test names (`proptest_check_mutation_allowed_modes`, `dos209_surface_constructors`, `dos209_mode_boundary`, `dos209_mutation_catalog`, `dos209_lint_regex_test`, `dos209_capability_trybuild`, `dos209_transactions`) unchanged. Authorized (NF1, NF3).
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:80:Description: `git status` reports `scripts/dos209-mutation-audit.sh`, `src-tauri/tests/dos209_mutation_catalog.txt`, and `.docs/plans/wave-W2/DOS-209-plan.md` as untracked. The L6 ruling (Option A) explicitly required "Plan v3 commits the script, the catalogue output, and a test that re-runs the audit and asserts the catalogue file matches." The script content + snapshot match exactly when re-run, the no-drift test is named in §9, and the file paths exist on disk — the substantive cycle-3 work is done. The artifacts simply aren't staged/committed yet. This is a process-flag, not a contract-violation flag: the plan author has produced the artifacts the ruling required and they live at the paths §9 binds; merging the W2-A PR will commit them as part of the implementation work. Worth flagging for the operator to ensure the plan-v3 commit lands before W2-A coding starts so the no-drift test has a stable snapshot to assert against.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:82:What needs to change: commit `scripts/dos209-mutation-audit.sh`, `src-tauri/tests/dos209_mutation_catalog.txt`, and the plan-v3 file before W1 clears L3 / W2-A coding starts. No plan revision required.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:85:Description: The committed snapshot rows include both the function-definition line (`accounts::snooze_triage_item:1941`) and per-evidence line numbers (`D=src-tauri/src/services/accounts.rs:1944`). Any benign edit above a catalogued mutator (insert a comment, reflow imports, add an unrelated function) shifts every downstream line and breaks the no-drift test even when no mutator semantics changed. This is the canonical fragility of line-number-keyed snapshots.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:87:Mitigation already partially in place: the no-drift test fires only as part of `cargo test`, and the script is committed alongside, so re-running it produces a clean re-snapshot — fixing the test is a one-command operation (`bash scripts/dos209-mutation-audit.sh > src-tauri/tests/dos209_mutation_catalog.txt`). The fragility is therefore "extra one-line update on every services-tree edit," not "blocked merges." For DOS-209's substrate-freeze purpose this is acceptable — the test's job is to make sure the catalogue remains the source of truth, not to be edit-stable. Future tightening (sort by `module::fn` only without line numbers, or compute a content-hash per function body) is a v1.4.x follow-on, not a v3 blocker.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:89:Location: snapshot rows lines 1–233 of `src-tauri/tests/dos209_mutation_catalog.txt`; classify-and-emit logic at script lines 277–294.
.docs/plans/wave-W2/reviews/DOS-209-l0-code-reviewer-v3.md:103:NF1, NF2, and NF3 are all closed at the architectural and operational levels: the audit script exists, is programmatic, runs clean, matches the committed snapshot exactly, and includes all five cycle-2-omitted mutators; §1 and §7 acknowledge and reframe the 2026-04-29 amendment as the frozen contract; §9 restores the full `cargo clippy + cargo test + pnpm tsc --noEmit` command with targeted dos209 tests as supplemental evidence. Sections 2, 4, 5, 6, 8, and 10 are unchanged from the v2 state that architect-reviewer-v2 and codex-consult-v2 APPROVED, so cycle-3 scope discipline is maintained. The two fresh observations (uncommitted artifacts; line-number snapshot fragility) are Low severity and do not warrant pausing work for deeper architecture review — the catalogue gap that triggered the cycle-2 ESCALATE is closed by construction since the snapshot is now the deterministic output of a committed re-runnable script.
.docs/plans/v1.4.5-waves.md:55:Cycle 2 codex verdicts: CSO **APPROVE** (no findings); Challenge **BLOCK** (5 closed, 2 open, 1 high new); Eng **BLOCK** (2 closed, 3 open, 4 BLOCK/HIGH new); DX **APPROVE-WITH-AMENDMENTS** (3 closed, 2 open). Class pattern observed: stale references not propagated when cycle 2 amended individual lane sections (the §Wave shape named exceptions, W5-A slot reference, W1-B old signal name, DOS-474 idempotency key restated two ways). Plus one structural issue (W2-C requires claim rendering that W2 doesn't produce) and one cross-lane file-ownership pattern (`pipeline.rs` and `link.rs` documented as edited by lanes that don't own them).
.docs/plans/v1.4.5-waves.md:61:4. **W1-B signal name reconciled** — `WorkspaceIngestionRejected` removed; only `WorkspaceFileRejected` remains, emitted by W2-A pipeline at the boundary based on the typed error returned by `registry::open_validated`.
.docs/plans/v1.4.5-waves.md:73:Cycle 3 codex verdicts: CSO **APPROVE** (skipped, prior approve held); Challenge **BLOCK** (1 new finding — SignalEmitter trait wave-timing inversion); Eng **BLOCK** (4 new findings — same trait timing + mod.rs ownership + W2-D narrowing + W1-B emission ownership); DX **APPROVE** (no new findings). All cycle 2 carryover findings CLOSED in both Challenge and Eng. Class pattern observed (again): downstream propagation incomplete when amending — cycle 3 added `&dyn SignalEmitter` to `link::override_link` but kept the trait in W2-A; cycle 3 narrowed W2-C but missed the parallel narrowing in W2-D; cycle 3 amended #4 (boundary emission) but didn't update W1-B's gate text.
.docs/plans/v1.4.5-waves.md:79:4. **W1-B emission ownership clarified.** W1-B's `open_validated` returns a typed `RejectionReason` (the W1-A-owned enum from `contracts.rs` per cycle 6 promotion); W2-A's pipeline owns signal emission at the boundary (consistent with cycle 3 #4). W1-B no longer says it emits `WorkspaceFileRejected` (which would not compile in W1 anyway because the SignalType isn't registered until W3-B).
.docs/plans/v1.4.5-waves.md:90:2. **W1-C trait-method abstraction.** W1-C `link::override_link` now calls `emitter.emit_link_changed(...)` (the W1-A trait method), not `SignalType::WorkspaceFileEntityLinkChanged` (the Wave-3 enum variant). W1-C tests against `NullSignalEmitter` and a mock recorder of trait-method calls. The bus-level mapping (trait method → `SignalType::WorkspaceFile*` → `emit_signal`) is W3-B's responsibility. Closes the W1-C wave-timing inversion that mirrored W1-B's.
.docs/plans/v1.4.5-waves.md:102:1. **Shared types promoted to W1-A `contracts.rs`** (Eng cycle 5 #1). The `Extractor` and `SignalEmitter` trait signatures reference `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason`, and `SourceType`. If those types live in later-wave lanes (W3-A, W1-B respectively), the traits don't compile in W1-A. Cycle 6 moves the type definitions into `contracts.rs` (W1-A-owned). W1-B and W3-A then USE the types from `contracts` rather than defining them locally.
.docs/plans/v1.4.5-waves.md:103:2. **`RegistryRejection` renamed to unified `contracts::RejectionReason`** — W1-B's `open_validated` returns `Result<(File, FileIdentity), RejectionReason>` where the registry only emits the path-related variants and the pipeline emits the size/format variants. One enum, one ownership location.
.docs/plans/v1.4.5-waves.md:126:3. **`WorkspaceCategoryRegistry` added to W1-B** — defines per-entity-type allowed categories with extensibility hook (`Other(String)` registered names) and the path resolution rules: `Accounts/{entity}/{category}/{filename}` when category present, `Accounts/{entity}/{filename}` when absent.
.docs/plans/v1.4.5-waves.md:133:Estimated added work: ~1.5 days across W1-A (column + enum), W1-B (registry + path resolution), W2-A (auto-detection + path resolution call), W4-C (input/receipt/error extension), W3-C (projection facet + query filter). No new wave; no new migration slot; no architectural changes to the cycle 7-approved structure.
.docs/plans/v1.4.5-waves.md:245:Per-lane slot text updated below: W1-A v250+v251 (was v200/v201), W1-B v252 (was v202), W1-C v253–v254 (was v203/v204), W2-A inherits W2 block, W5-A v268 (was v218). The §Migration slot assignments table and Summary footer `v200–v219` → `v250–v269`.
.docs/plans/v1.4.5-waves.md:423:| All untrusted input (markdown content, MCP write payloads, attachment file paths) passes through validation before service entry — no raw content reaches `commit_claim` without sanitization | Suite S test at W2 gate; explicit lint script `check_workspace_path_validation.sh` (W1-B owns) forbids `std::fs::File::open` / `tokio::fs::File::open` on workspace-rooted paths outside `registry::open_validated` | W1-B merge → W2-A merge |
.docs/plans/v1.4.5-waves.md:496:  - **Full submodule skeleton pre-created** under `src-tauri/src/services/workspace_ingestion/`: `mod.rs` (contains ALL `pub mod` declarations for the submodules so subsequent lanes never edit `mod.rs`); `contracts.rs` (defines the `Extractor` and `SignalEmitter` traits AND ships `NullExtractor` + `NullSignalEmitter` default impls — these become the shared substrate that earlier-wave lanes like W1-C can compile against without depending on later waves); `lifecycle.rs` (W1-A's substantive content); empty placeholder files with module-level doc comments only for `registry.rs` (W1-B fills), `runs.rs` (W1-C fills), `link.rs` (W1-C fills), `pipeline.rs` (W2-A fills), `extract.rs` (W3-A fills), `signals.rs` (W3-B fills), `graph.rs` (W3-C fills), `wiring.rs` (W2-A fills).
.docs/plans/v1.4.5-waves.md:512:- **Done when:** `workspace_file_lifecycle` table exists with the seven-state lifecycle enum (including `pending_entity_assignment`), all required columns per the project DoD plus `category: Option<TEXT>` (cycle 8 Option B-prime — stores the auto-detected or caller-provided sub-category for AI navigation); Rust types compile inside `services::workspace_ingestion::lifecycle`; `services::workspace_ingestion::contracts` defines the **shared type surface** that every subsequent lane consumes (cycle 6 amendment promotes `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason` into `contracts.rs` because the `Extractor` trait signature would otherwise reference types owned by W1-B and W3-A; cycle 8 amendment adds `WorkspaceCategory`; **cycle 12 amendment** drops the `SourceType` mirror and consumes canonical `WorkspaceFileKind` directly + drops local `SourceAttribution` reinvention and consumes canonical 7-field `SourceAttribution` from `provenance/source.rs:303`):
.docs/plans/v1.4.5-waves.md:572:    With these types + traits + Null impls in place, W1-C can compile and test `link::override_link(emitter: &dyn SignalEmitter, ...)` against `NullSignalEmitter` in Wave 1, without referencing any `SignalType` variant (W3-B owns the bus-level emission mapping); W1-B's `open_validated` returns the W1-A-owned `FileIdentity` and `RejectionReason`; W3-A's `WorkspaceExtractor` impl produces W1-A-owned `WorkspaceClaimProposal` records. `mod.rs` declares all submodules with one declaration per line:
.docs/plans/v1.4.5-waves.md:588:- **Handoff notes:** All subsequent v1.4.5 lanes consume the W1-A skeleton: trait + Null impls from `contracts.rs`, lifecycle types from `lifecycle.rs`, full submodule layout pre-declared in `mod.rs`. No subsequent lane creates a submodule file or edits `mod.rs` — they fill the placeholder content of their owned submodule only. Submodule ownership: `lifecycle.rs` (W1-A), `contracts.rs` (W1-A), `mod.rs` (W1-A), `registry.rs` (W1-B), `runs.rs` (W1-C), `link.rs` (W1-C), `pipeline.rs` (W2-A), `wiring.rs` (W2-A), `extract.rs` (W3-A), `signals.rs` (W3-B), `graph.rs` (W3-C).
.docs/plans/v1.4.5-waves.md:592:### Agent W1-B — DOS-464: Registry migration for workspace source types
.docs/plans/v1.4.5-waves.md:596:- **Files owned (exclusive):** `src-tauri/src/services/workspace_ingestion/registry.rs` (W1-B is the only owner of this submodule); new migration(s) in `src-tauri/src/migrations.rs` (slot **v252** per cycle 11 renumber); no other files.
.docs/plans/v1.4.5-waves.md:597:- **Don't touch:** `signals/policy_registry.rs` (v1.4.1 owns; v1.4.5 signal additions belong to W3-B); `processor/classifier.rs` (W2 owns); `google_drive/sync.rs` (W2-B refactor lane owns the Drive mutation path); `services/workspace_ingestion/lifecycle.rs` (W1-A owns); `services/workspace_ingestion/mod.rs` (W1-A owns; W1-B contributes a re-export only via the agreed convention, no edits to this file).
.docs/plans/v1.4.5-waves.md:602:  - On rejection, return a typed `RejectionReason { PathTraversalAttempt | SymlinkRefused | SymlinkRaced | OutsideWorkspace }` error to the caller. **W1-B does not emit signals** (cycle 4 amendment to resolve the wave-timing inversion: `WorkspaceFileRejected` SignalType isn't registered until W3-B, so W1-B emitting it directly would not compile in W1). The W2-A pipeline owns signal emission at the boundary based on the typed error variant — `pipeline.rs` calls its `SignalEmitter` slot (constructed via `wiring.rs`) when handling a `RejectionReason` it propagates.
.docs/plans/v1.4.5-waves.md:608:  3. *Signals + invalidation:* No signals emitted in W1-B itself. Rejection signal type is `WorkspaceFileRejected` (registered in W3-B); W1-B's `open_validated` returns a typed error, and the consumer (W2-A pipeline) decides whether to emit the signal at the boundary based on the error reason. Rejection-reason variants are documented in the security gate.
.docs/plans/v1.4.5-waves.md:611:- **Tests required:** All negative fixtures from the security gate (TOCTOU symlink race, `%2e%2e`, NFC/NFD `..`, absolute-path, outside-workspace symlink); valid path acceptance test returning `(File, FileIdentity)`; `DataSource` variant mapping round-trip across all `WorkspaceFileKind` values; CI lint script green (no path-based file reads outside `registry::open_validated`); `cargo clippy -D warnings` clean.
.docs/plans/v1.4.5-waves.md:612:- **Done when:** `WorkspaceSourceRegistry::open_validated` exists and returns `(File, FileIdentity)` from a single atomic open; all negative fixtures rejected with the typed reasons listed in the security gate; CI lint script in place; ingestion call sites in subsequent lanes consume only the returned handle (verified via grep at W2-A merge); migration slot v252 used (cycle 11 renumber); all tests green; `/cso` L0 plan AND L2 diff approval recorded.
.docs/plans/v1.4.5-waves.md:621:- **Don't touch:** `services/workspace_ingestion/{lifecycle,contracts,mod}.rs` (W1-A); `services/workspace_ingestion/registry.rs` (W1-B); claim substrate; signal bus. (No file creation: W1-A pre-created the placeholders.)
.docs/plans/v1.4.5-waves.md:642:- L0 plan approvals: architect-reviewer for W1-A/C; architect-reviewer + `/cso` for W1-B.
.docs/plans/v1.4.5-waves.md:643:- L2 diff approvals on 3 PRs. `/cso` re-review on W1-B.
.docs/plans/v1.4.5-waves.md:683:  **Tie-break rules:** higher priority wins; within the same priority, first matched rule wins. **Validation:** auto-detected results MUST pass `WorkspaceCategoryRegistry::validate(category, entity_type)` — if the rule produces a category not allowed for the file's entity type, fall back to `None` (entity root) rather than silently using a disallowed category. **Content sniff bound:** `content_head` is the first 4KB of file content (sufficient for frontmatter parsing); never reads the full file. Pipeline records the resolved category in the lifecycle row and resolves the destination path via `WorkspaceCategoryRegistry` from W1-B.
.docs/plans/v1.4.5-waves.md:737:- **Tests required (cycle 3 amendment — fixture-injected claim rendering):** Block renders entity claim list from a fixture-seeded set of `intelligence_claims` rows for the fixture entity (the W2 demo uses hand-seeded claims because W2-A's pipeline produces zero claims; real ingestion-to-claim-render validation is W5-B's job); trust-band colors match `likely_current`/`use_with_caution`/`needs_verification`; intake ability invocation triggers `IngestPipeline::run` in unit test (verifies the wiring fires; the pipeline records the run but produces zero proposals at W2 time, which is the expected W2-A behavior); negative fixture: intake with path traversal path returns error, block shows error state (not a crash).
.docs/plans/v1.4.5-waves.md:738:- **Done when:** Block activates, renders fixture-seeded claim proposals for a fixture entity with trust bands inline; intake ability invocation triggers `IngestPipeline::run` and writes an ingestion run record (zero proposal expected at W2 time); intake ability declared with correct scopes and `mcp_exposure: None`; path traversal negative fixture green; `/qa-only` L4 surface QA passes in WP Studio against the fixture-seeded entity. Real ingestion-to-claim-render flow validation deferred to W5-B's E2E validation against real workspace data.
.docs/plans/v1.4.5-waves.md:818:- **Tests required:** Signal emission test for each of the five new variants (fixture-driven); typed payload struct has no `String` content/path fields (compile-time check); invalidation queue enqueued after `WorkspaceFileIngested` AND `WorkspaceFileQuarantined` AND `WorkspaceFileEntityLinkChanged`; invalidation queue NOT enqueued after `WorkspaceFileRejected` AND `WorkspaceFilePendingEntityAssignment` (negative tests); `from_name` round-trips for all five variants; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.5-waves.md:942:  - Pass the artifact-12-style negative fixture suite for each attack vector listed below.
.docs/plans/v1.4.5-waves.md:950:- **Tests required (negative fixture suite — must all reject/sanitize):** raw `<script>alert(1)</script>`; inline event handler `<img onerror=alert(1)>`; `javascript:alert(1)` href; `data:text/html,<script>` href; `data:image/svg+xml,<svg onload=alert(1)>` src; `<svg><script>alert(1)</script></svg>`; `<math><mtext><script>alert(1)</script></mtext></math>`; `<foreignObject>` HTML escape; `style="background:url(javascript:alert(1))"`; `style="background-image:url(http://evil/)"`; `<style>@import url(http://evil)</style>`; `<link rel=stylesheet href=...>`; `<object data=...>`; `<embed src=...>`; `<iframe src=...>`; `<!--[if IE]><script>alert(1)</script><![endif]-->`; remote `<img src="http://evil/track.png">`; valid markdown round-trip (h1, paragraphs, code block, link with http href, image with `dailyos-asset://` src, table) renders cleanly. Plus: `markdown_preview` Rust ability invokes correctly via SurfaceClient; PHP_CodeSniffer + WP Coding Standards clean.
.docs/plans/v1.4.5-waves.md:951:- **Done when:** Sanitizer is structurally allowlist-based (not blocklist patches); all negative fixtures sanitize correctly; valid markdown round-trips; Rust ability declared with correct scopes and `mcp_exposure: None`; `/cso` L0 plan AND L2 diff approval recorded; all tests green.
.docs/plans/v1.4.5-waves.md:970:      "category": Option<String>,               // OPTIONAL canonical slug from WorkspaceCategory::as_slug() — one of "presentations" / "transcripts" / "meetings" / "notes" / "contracts" / "attachments" or a registered Other(name) lowercase slug. Cycle 10 contract reconciliation — two distinct branches, no overlap: (a) ABSENT (null/missing): server runs pipeline::auto_detect_category() per the cycle 9 frozen detection rule table; (b) PRESENT: server validates against W1-B WorkspaceCategoryRegistry BEFORE constructing IngestRequest — well-formed AND registry-valid → category_hint = Some(...); malformed OR registry-disallowed → return PlacementError::CategoryNotAllowed and never invoke the pipeline. There is no "invalid hint falls through to auto-detection" path; that behavior would silently bury caller errors.
.docs/plans/v1.4.5-waves.md:1015:- **Tests required:** Ability invocation commits claim proposals via `IngestPipeline`; `client_source_asof` ignored (server time used) — test asserts the recorded `source_asof` differs from a synthetic future `client_source_asof`; `EntityNotFoundOrUnauthorized` returned when caller lacks subject; `RateLimited` after burst; `IdempotencyConflict` on duplicate `(entity_id, content_sha256, dedup_key)`; transactional rollback when signal emission fails (lifecycle row not committed); MCP allowlist enforcement (negative: ability not invocable without `write.workspace_place_document`); path/content sanitization equivalent to filesystem path; dry-run produces `DryRunSummary` without writes.
.docs/plans/v1.4.5-waves.md:1103:- [ ] CI invariant: DOS-474 MCP write allowlist enforcement (negative test green)
.docs/plans/v1.4.5-waves.md:1116:- [ ] dailyos/markdown-preview block: renders sanitized markdown; remote asset blocking confirmed; script injection negative fixtures all green
.docs/plans/v1.4.5-waves.md:1121:- [ ] DOS-464 path traversal: negative fixtures all green
.docs/plans/v1.4.5-waves.md:1124:- [ ] DOS-474 MCP write allowlist: ability not invocable without allowlist entry (negative test green); subject validation active
.docs/plans/_archive/v1.0.0.md:290:47a. Champion health `weak`/`lost` emits `negative_sentiment` on champion's person_id → triggers existing `rule_champion_sentiment` → `champion_risk` callout in morning briefing (I555).
.docs/plans/wave-W0/DOS-309-plan.md:45:- TOCTOU on in-memory `intel` value during reorder → DOS-311 (W1-B universal write fence) is the natural concurrency layer.
.docs/plans/wave-W0/DOS-309-plan.md:50:End-state alignment: closes a direct ghost-resurrection vector AND establishes the DB-before-file ordering invariant + post-commit-side-effects discipline that the universal write fence (DOS-311 / W1-B), the projection writer (DOS-301 / W3-D), and DOS-7's `commit_claim` consolidation all depend on.
.docs/plans/wave-W0/DOS-309-plan.md:326:- **TOCTOU on in-memory `intel`:** out of scope for W0. DOS-311 (W1-B) universal write fence is the natural concurrency-protection layer.
.docs/plans/wave-W0/DOS-309-plan.md:365:  1. **Signals — semantics tightened:** `intelligence_curated` and `field_conflict_*` signals only fire after the DB transaction commits. Downstream consumers see fewer false-positive curation signals.
.docs/plans/wave-W0/DOS-309-plan.md:382:  - **DOS-311 (W1-B universal write fence):** post-commit `write_intelligence_json(&dir, &intel)` call site IS the fence integration point. After W1-B, the fence wraps this call; the `log::warn!` becomes redundant once DOS-311 supplies structured failure handling. Marked `// TODO(DOS-311): replace with fence error handler`.
.docs/plans/wave-W0/DOS-309-plan.md:398:**TOCTOU on in-memory `intel`:** documented as out of scope for W0. DOS-311 (W1-B) is the natural concurrency layer.
.docs/plans/v1.4.1-waves.md:85:1. **W1 internal staging** — W0-A (DOS-241 ADR) + W1-E (DOS-306 contract) form a stage-1 gate that must merge before W1-B/C/D L0 plans are written. W1-A runs in parallel with stage 1 and stage 2 since it is independent of the W1-E contract. W1 merge gate (full wave closure) waits on stage 2 implementation + W1-E post-implementation verification.
.docs/plans/v1.4.1-waves.md:122:| Signal emission goes through `signals/policy_registry.rs` (no direct `db.execute` on signal events; channel inventory enforced — see W1-B done-when) | grep CI test + clippy + W1 Suite S signal-channel traversal property | W1-B merge |
.docs/plans/v1.4.1-waves.md:231:This wave converts the substrate's data model + reactive layer onto v1.4.1's invariants. **Two-stage internal ordering** — codex cycle-2 review found that running W1-E in parallel with W1-B/C while declaring W1-E "frozen contract before they start" was a logical contradiction. Resolution:
.docs/plans/v1.4.1-waves.md:237:Both W0-A and W1-E must merge before any of W1-B/C/D L0 plans are written. This is the hard precondition that makes the "DOS-241 is a hard input" claim true in execution, not just on paper.
.docs/plans/v1.4.1-waves.md:241:- **W1-B (Signal Policy Registry)** and **W1-C (Durable Invalidation Job Model)** run in parallel; each L0 plan cites the W1-E contract and the W0-A ADR's accepted/rejected DOS-235/236/237 amendments.
.docs/plans/v1.4.1-waves.md:242:- **W1-D (Signal Coalescing + Queue Bounds)** depends on W1-B + W1-C merging first; serial within stage 2.
.docs/plans/v1.4.1-waves.md:250:- **Don't touch:** signal/invalidation modules (W1-B/C/D own).
.docs/plans/v1.4.1-waves.md:253:### Agent W1-B — DOS-235 Signal Policy Registry + consolidated emit_signal
.docs/plans/v1.4.1-waves.md:256:- **Files owned (function-level on shared `signals/bus.rs`):** `src-tauri/src/signals/policy_registry.rs` (new, exclusive); inside `signals/bus.rs`, W1-B owns the `emit_signal` function family (`emit_signal`, plus any new `emit_signal_*` variants that route through the policy registry). Every direct `db.execute` on signal events outside `emit_signal` is refactored — the deny is a grep CI test, not a hand-traced sweep.
.docs/plans/v1.4.1-waves.md:257:- **Done when:** registry exhaustiveness check green; `emit_signal` is the only writer; **all signal-emission channels enumerated and routed through `policy_registry.rs`** (per `feedback_enumerate_channels_before_patching.md` — at minimum: services/, abilities/, Tauri/MCP/Worker/Eval bridges, trigger-derived state, replay binary from W2-I, telemetry sinks, **and any new binaries added under `src-tauri/src/bin/` post-W2-I** — pinned to a concrete shape rather than left as "future external clients") — channel inventory captured in `.docs/plans/wave-W1/W1-B-channel-inventory.md`; W1 Suite S asserts no signal can reach the invalidation queue without traversing `policy_registry.rs`; CI lint enforces.
.docs/plans/v1.4.1-waves.md:263:- **Don't touch:** signal registry (W1-B owns); coordinate via signal -> job emission contract.
.docs/plans/v1.4.1-waves.md:269:- **Files owned (function-level on shared `signals/bus.rs`, no overlap with W1-B's `emit_signal` family):** the `coalesce_*` function family in `signals/bus.rs` (added by this agent, new functions only — no edits inside `emit_signal*`); `invalidation_jobs` consumer module (exclusive); queue-bound config; load-test harness in `tests/`.
.docs/plans/v1.4.1-waves.md:270:- **Depends on:** W1-B + W1-C must be merged first (this is a single-agent gate but coordinates with both); W1-B's `emit_signal` consolidation defines the seam W1-D's coalescing wraps.
.docs/plans/v1.4.1-waves.md:277:- **Done when (stage-1 — gates W1-B/C/D L0 plan writing):** frozen contract artifact merged with `feedback_set_and_forget_wave_protocol.md`-style boundary spec covering signal bus durability, policy registry semantics, claim-event load envelope; **L0 reviewer (architect-reviewer + spec-writer) approval recorded on the contract doc PR**; no implementation evidence required at this stage. The W1-B/C/D verification confirmation moves to the W1 merge gate (post-implementation), not W1-E's own done-when.
.docs/plans/v1.4.1-waves.md:282:- L0 plan approvals (architect-reviewer + performance-engineer for W1-B/C/D; architect-reviewer for W1-A; spec writer for W1-E)
.docs/plans/v1.4.1-waves.md:284:- **W1-E contract verification (post-implementation, separate artifact):** verification doc at `.docs/plans/wave-W1/DOS-306-verification.md`, merged as its own PR after W1-B/C/D merge and before W1 closes. The verification doc cites file:line for each W1-B/C/D site that consumes the W1-E contract; reviewer (architect-reviewer + spec-writer) approves on the verification PR. **Not** appended to the already-merged W1-E PR — separate artifact, separate PR, separate review.
.docs/plans/v1.4.1-waves.md:518:- Build calibration fixture in `src-tauri/suites/E/canonicalization-thresholds/` per ADR-0131 §9: 500+ labeled pairs across claim types covering positive paraphrases (40%), hard negatives (30%), contradictions (15%), one-sided qualifier asymmetry (10%), low-trust duplicates (5%).
.docs/plans/v1.4.1-waves.md:786:- Stage 8b must not start until Stage 8a L2 passes validator/schema parity, command-surface `--` parity, and negative probes demonstrating malformed evidence fails closed.
.docs/plans/v1.4.1-waves.md:816:- **DOS-241 → W1 hard input.** W1-B/C/D L0 plans must cite the DOS-241 ADR's accepted/rejected amendments; not advisory (codex F5).
.docs/plans/wave-W2/retro.md:42:| health_debouncer missed entries (`3c9e6c1c`) | ~10 min | 2 catalog entries (schedule_recompute, drain_pending) returning `()` — pattern: `if ctx.check_mutation_allowed().is_err() { return; }` instead of `?`. |
.docs/plans/wave-W2/retro.md:62:3. **L3 catching what L1+L2 missed.** L1 (clippy + tests) and L2 (codex adversarial vs working tree) both passed. L3 (Wave adversarial against the W2-A commit range scoped via `--base 275cfc85`) caught the entity_linking::evaluate gap because it specifically asked "is the gate the FIRST call in EVERY catalogued mutator." A spot-check that L1/L2 didn't have a reason to perform.
.docs/plans/wave-W2/retro.md:74:2. **`entity_linking::evaluate` was missed in the original migration sweep.** The mutation catalog had `entity_linking::manual_set_primary`, `manual_dismiss`, `manual_undismiss`, etc. — but not the top-level `evaluate` function or the per-rule trait method. The catalog was authoritative for Group E, and the `evaluate`/`evaluate_meeting`/`evaluate_email` chain was outside it. This is a catalog-completeness gap, not a migration mechanics gap. The §9 mutation-catalog drift CI test would have caught this; not having it meant L3 was the first line of defense.
.docs/plans/wave-W2/retro.md:112:### Run mutation-catalog drift CI test before declaring substrate-wave complete
.docs/plans/wave-W2/retro.md:114:The §9 drift test was deferred. The result: a real catalog-completeness gap (`entity_linking::evaluate`) reached L3 review instead of being caught by CI. For W3+ substrate waves with similar surface enumeration: land the drift CI test BEFORE the migration starts. Costs ~30 min; pays back at the close-out gate.
.docs/plans/wave-W2/retro.md:153:- **§9 catalog-drift CI test** — Requires the mutation-catalog audit script output stable enough to be a CI-pinned golden file. Audit script exists; making it deterministic across local + CI environments needs ~2-4 hours.
.docs/plans/dos-546/phase-0/09-rate-limit-matrix.md:161:`retry_after_ms` is computed from the exhausted bucket that won precedence. The `Retry-After` header is rounded up to whole seconds, with a minimum of `1` when the retry interval is positive.
.docs/plans/dos-546/phase-0/09-rate-limit-matrix.md:279:- Expected: request 301 returns 429 with `axis: "surface_client"` and positive `retry_after_ms`.
.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:174:      fires the effect with current attributes once — benign). Add positive
.docs/plans/v1.4.3-wp-foundation/L0-packet-B-render-stabilization.md:175:      fixture "first-mount fires exactly one reload" alongside the negative
.docs/plans/wave-W4/_prompts/_common.md:74:What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
.docs/plans/v1.4.3-wp-foundation/W6-negative-fixture-catalog.md:3:DOS-575 closes the v1.4.3 foundation release gate by pinning every trust boundary crossed by the WordPress foundation to a named negative fixture. The canonical runner is:
.docs/plans/v1.4.3-wp-foundation/W6-negative-fixture-catalog.md:53:`src-tauri/src/release_gate.rs` treats `w6-fixtures.json` as mandatory hermetic evidence. With `run_tests=true`, the gate invokes `scripts/release-gate/run-w6-fixtures.sh`; with `--no-run-tests`, it reads `output-dir/w6-fixtures.json`. Every catalog ID above becomes a mandatory invariant. Missing IDs, stale binding, unknown statuses, failures, and skipped fixtures exit non-zero.
.docs/plans/wave-W0/proof-bundle.md:101:3. **TOCTOU on in-memory `intel` value during reorder** — deferred to [DOS-311](https://linear.app/a8c/issue/DOS-311) (W1-B universal write fence) which is the natural concurrency-protection layer.
.docs/plans/_archive/shipped/v1.0.3-shipped.md:134:7. Correct predictions emit positive feedback signal for the source that generated the prep risk. `✗ not raised` does not emit negative feedback; `⚡ surprise` logs a coverage gap.
.docs/plans/dos-546/phase-0/10-user-presence-nonce.md:244:target_false_positive_rate = 0.001
.docs/plans/dos-546/phase-0/10-user-presence-nonce.md:249:unknown, the DB result wins. Bloom false positives must never reject a valid
.docs/plans/wave-W4/_prompts/DOS-5.md:20:- Property test on geometric-mean math must be clippy clean. Floating-point gotchas: use `f64`, avoid `.sqrt()` on negative after subtraction.
.docs/plans/wave-W6/_prompts/_common.md:74:What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
.docs/plans/wave-W2/frozen-contract-drift-W2-to-W3.md:47:| `#[ability(category = …)]` AST inspection | 2026-04-20 decision: compile-time AST inspection of `services::*` mutation calls | mutation taxonomy & catalogue locked in W2-A `dos209_mutation_catalog.txt` (committed before W3) | ⚠️ — W3-A plan must reference the committed catalogue as the macro's allowlist seed; do not hand-roll a new list |
.docs/plans/wave-W2/frozen-contract-drift-W2-to-W3.md:72:| Hard-delete CHAIN refactor on `account_stakeholder_roles` | replace `set_team_member_role` and `remove_account_team_member` hard deletes with tombstone claim writes | W2-A mutation catalogue includes both sites with `D+TX+SIG` tags — they will receive `ctx.check_mutation_allowed()?` first; W3-C replaces the body content | ✅ — confirm in §7 that the W2-A catalogue addition + W3-C body replacement is sequenced correctly (W2-A merges first, W3-C rebases) |
.docs/plans/wave-W2/frozen-contract-drift-W2-to-W3.md:84:| `validators/json_columns.rs` shared between propose_claim and `db/accounts.rs:1205-1212` | ticket lock | W2-A catalogue lists `accounts::update_account_field_inner` and others writing those columns; the validators are pure value-checks, not stateful, no W2 dependency | ✅ |
.docs/plans/wave-W2/frozen-contract-drift-W2-to-W3.md:111:| Backfill ≥95% coverage | ticket DoD | W1-B fence + DOS-308 quarantine table both required upstream | ✅ |
.docs/plans/wave-W1/W1-B-channel-inventory.md:1:# W1-B Signal Emission Channel Inventory
.docs/plans/wave-W1/W1-B-channel-inventory.md:7:| Services facade | `src-tauri/src/services/signals.rs`; service callers under `src-tauri/src/services/**/*.rs` | `services::signals::*` gates mode/capability and delegates to `signals::bus::emit_signal*`, which resolves `policy_registry::policy_for` before any row write. | Read-only `SELECT ... FROM signal_events`; test modules seeding fixtures. | Existing `src-tauri/tests/dos209_regression.rs` blocks raw bus emission in `services/`; W1-B single-writer lint blocks direct production `INSERT` / `UPDATE` to `signal_events`. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:8:| Infrastructure services and processors | `src-tauri/src/intel_queue.rs`, `src-tauri/src/processor/**/*.rs`, `src-tauri/src/prepare/**/*.rs`, `src-tauri/src/context_provider/**/*.rs`, `src-tauri/src/intelligence/**/*.rs`, `src-tauri/src/proactive/**/*.rs`, `src-tauri/src/workflow/**/*.rs` | Direct infrastructure callers may call the bus, but every bus entrypoint routes through `policy_registry.rs`. Signal-derived background invalidation remains behind bus propagation hooks. | Telemetry `app.emit(...)`; test modules; read-only queries. | W1-B single-writer lint rejects direct production event-log writes outside the bus and policy registry. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:9:| Connector/background processors | `src-tauri/src/google.rs`, `src-tauri/src/executor.rs`, `src-tauri/src/clay/**/*.rs`, `src-tauri/src/gravatar/**/*.rs`, `src-tauri/src/linear/**/*.rs`, transcript/file processors under `src-tauri/src/granola`, `src-tauri/src/quill`, `src-tauri/src/processor` | Connector-originated correctness signals use `services::signals::*` or the bus; both traverse the registry. | UI refresh events such as `emails-updated`, auth status events, and operation progress are Tauri telemetry, not bus signals. | W1-B lint covers direct `signal_events` writes; inventory drift test requires these globs to stay listed. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:10:| Abilities runtime | `src-tauri/src/abilities/**/*.rs`; ability descriptors in `src-tauri/src/abilities/registry.rs` | Abilities currently declare `signal_policy` metadata only. Future output-change emission must go through `AbilityContext` / `ServiceContext` to `services::signals::*` and then `policy_registry.rs`. | Read abilities with `emits_on_output_change = []`; evaluation fixtures with no DB writer. | Ability surface tests continue to check descriptor metadata; W1-B inventory drift test pins the `abilities/` channel. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:11:| Tauri bridge | `src-tauri/src/bridges/tauri.rs`; command surfaces under `src-tauri/src/commands/**/*.rs` | Commands call services; services emit through the facade and registry. Bridge descriptors may declare ability signal policy but do not write `signal_events` directly. | `app.emit(...)` UI events are telemetry/progress and cannot enqueue invalidation jobs. | W1-B inventory drift test pins `bridges/tauri.rs` and `commands/`; single-writer lint catches event-log SQL. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:12:| MCP bridge | `src-tauri/src/bridges/mcp.rs`; `src-tauri/src/mcp/**/*.rs` | MCP invokes abilities/services. Signal declarations remain descriptor metadata until a service emits through the registry-backed bus. | Static MCP schema fixtures and descriptor tests; no raw event-log writer. | W1-B inventory drift test pins bridge and MCP globs; single-writer lint covers SQL writes. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:13:| Worker bridge | `src-tauri/src/bridges/worker.rs`; worker loops in `src-tauri/src/intel_queue.rs`, `src-tauri/src/meeting_prep_queue.rs`, schedulers under `src-tauri/src/scheduler.rs` and `src-tauri/src/hygiene/**/*.rs` | Worker-originated correctness signals use service facade or bus, then registry. Queue progress events are not bus signals. | Queue enqueue/dequeue state, leases, and UI progress events; read-only signal queries. | W1-B single-writer lint plus inventory drift test. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:14:| Eval bridge and replay harness | `src-tauri/src/bridges/eval.rs`, `src-tauri/src/harness/**/*.rs`, `src-tauri/src/services/external_replay/**/*.rs` | Evaluate/simulate paths may declare intended signals, but side-effecting DB emission must be blocked by `ServiceContext` or captured by explicit fixture helpers before any Live flush. | Hermetic replay data, fixture reads, and no-op descriptor defaults. | Existing external replay lint plus W1-B inventory drift test. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:15:| Trigger-derived state and subscribers | `src-tauri/src/signals/derived_state_subscribers.rs`, `src-tauri/src/signals/event_trigger.rs`, `src-tauri/src/services/derived_state.rs`, `src-tauri/src/services/entity_linking/**/*.rs` | Subscribers consume registry-backed emissions and write owned derived state through services. They may not insert `signal_events` directly. | Subscriber registry declarations and read-side queries. | W1-B single-writer lint catches direct event-log writes from subscribers or derived-state services. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:16:| Signal propagation rules | `src-tauri/src/signals/propagation.rs`, `src-tauri/src/signals/rules.rs`, `src-tauri/src/signals/invalidation.rs` | Source and derived signals are appended by the `emit_signal` family only; propagation rules return `DerivedSignal` data and the bus handles registry policy. | Unit-test fixture inserts in `#[cfg(test)]` modules. | W1-B single-writer lint allows production writes only in the bus emit family. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:17:| Replay / operational binaries | `src-tauri/src/bin/reconcile_post_migration.rs`, `src-tauri/src/bin/repair_entity_linking.rs`, `src-tauri/src/bin/release_gate.rs`, `src-tauri/src/bin/generate_canonicalization_corpus.rs` | Current binaries do not emit bus signals. Any future production-like binary must call the registry-backed bus; one-shot migration overrides must be explicit and inventoried here. | `release_gate` writes only gate artifacts; reconcile/repair binaries read or repair through services; `generate_canonicalization_corpus` writes JSON fixture files under `src-tauri/suites/E/canonicalization-thresholds/` only (ADR-0131 Phase B calibration corpus, no DB or bus writes). | W1-B inventory drift test enumerates every file under `src-tauri/src/bin`. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:18:| Dev mock fixture seeding | `src-tauri/src/devtools/mod.rs` | Mock `signal_events` rows use a registry-backed bus fixture helper so seeded rows still traverse policy lookup. | Debug/dev guard must remain in place; this is not a production mutation path. | W1-B single-writer lint permits only the bus helper to write event rows. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:19:| Migrations and schema checks | `src-tauri/src/migrations.rs`, `src-tauri/src/migrations/**/*.sql` | Schema creation and migration verification may seed tables directly as part of migration tests, not runtime emission. | Migration/test-only direct `INSERT` accepted. | W1-B single-writer lint allowlists migrations. |
.docs/plans/wave-W1/W1-B-channel-inventory.md:21:| Privacy/purge/lifecycle cleanup | `src-tauri/src/privacy.rs`, `src-tauri/src/db/data_lifecycle.rs`, `src-tauri/src/db/accounts.rs` | Cleanup and merge paths may delete or reassign historical rows; they do not emit new signals. New correctness notifications caused by these paths must use services/bus. | Direct `DELETE FROM signal_events` and merge reassignment are data lifecycle operations, not emissions. | W1-B lint rejects direct `INSERT` / `UPDATE signal_events SET ...` except documented lifecycle reassignment and bus-owned supersede. |
.docs/plans/dos-546/phase-0/15-runtime-php-to-rust-endpoint.md:582:- Artifact 12 (negative fixtures): covers partial projection degradation F-05, discovery leakage F-08, browser-origin rejection, Host rejection, auth failures, and replay failures.
.docs/plans/wave-W4/DOS-384-plan.md:10:DOS-384 defines the canonical adversarial bundle catalog for bundles 2, 3, 4, 6, 7, and 8. It is a research+design issue, not the harness implementation and not trust compiler code. The Linear contract is explicit: define adversarial scenario specifications, `state.sql` seeds, `inputs.json` ability invocations, `expected_output.json`, `expected_state.json`, `expected_provenance.json`, and anonymization requirements for each bundle so DOS-216 can claim coverage at v1.4.0.
.docs/plans/wave-W4/DOS-384-plan.md:12:This is a v1.4.0 scope expansion. The original roadmap deferred bundles 2-4 and 6-8 to v1.4.1; the L0 review cycle-1 user ruling brought them into v1.4.0 so the harness can report all eight adversarial bundles end to end. DOS-216 remains the harness owner, but its reporting claims for these six bundles are blocked until DOS-384 defines the canonical catalog.
.docs/plans/wave-W4/DOS-384-plan.md:22:Author a catalog specification that can later materialize into bundle fixture directories aligned with DOS-283's bundle-corpus shape: `src-tauri/tests/fixtures/bundles/bundle_N/`, unless L0 reviewers require Linear's shorter `tests/fixtures/bundle-N/` wording literally. Each bundle directory should contain ADR-0110 files plus DOS-216's `metadata.json` and `expected_state.json`. Ability-specific wrappers under `src-tauri/tests/abilities/{ability}/fixtures/` remain DOS-216/W5-owned consumers, not the canonical bundle source.
.docs/plans/wave-W4/DOS-384-plan.md:109:Fixture path: prefer `src-tauri/tests/fixtures/bundles/bundle_N/` for canonical bundle catalog consistency with DOS-283. Ability-specific ADR-0110 fixtures may wrap these bundles later, but the catalog source should not be duplicated by each ability.
.docs/plans/wave-W4/DOS-384-plan.md:137:DOS-283/W6-A owns bundle 5 and provides the reference bundle-catalog shape for SQL-first fixture data plus expectation sidecars. Bundle 1 remains anchored by `src-tauri/tests/dos287_substrate_bundle1_reproduction.rs` and DOS-5's later migration from contamination rejection to `cross_entity_coherence`.
.docs/plans/wave-W4/DOS-384-plan.md:175:1. Fixture path wording: Linear says `tests/fixtures/bundle-{2,3,4,6,7,8}/`, while DOS-283 plans `src-tauri/tests/fixtures/bundles/bundle_1/` and `bundle_5/`. Confirm whether DOS-384 should use the DOS-283 catalog path or the literal Linear shorthand.
.docs/plans/wave-W2/proof-bundle.md:156:- [x] Every catalogued service mutator has `ctx: &ServiceContext<'_>` as FIRST parameter — verified by `scripts/dos209-mutation-audit.sh` (0 missing at close).
.docs/plans/wave-W2/proof-bundle.md:159:- [x] C-kind rows (chrono::Utc::now()) replaced with `ctx.clock.now()` at all catalogued locations — verified in accounts.rs, commitment_bridge.rs, projects.rs, linear.rs, and others.
.docs/plans/wave-W2/proof-bundle.md:210:- Remaining §9 deferreds (proptest, trybuild, catalog drift CI, transactions): formally documented in `dos209_regression.rs` file-level doc comment; deferred to a standalone follow-up with explicit scope acknowledgment.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer.md:82:Location in plan: §9 "Add focused tests such as `services::context::tests::proptest_check_mutation_allowed_modes` ... `dos209_mode_boundary.rs::evaluate_mutation_returns_write_blocked_by_mode`" — the second test name is closer but the plan does not say it iterates over the mutator catalogue.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer.md:110:8. **F8 — Mutation-coverage contract test.** §9 adds a test that iterates the mutator catalogue (from F3's audit script) and asserts `WriteBlockedByMode` under Evaluate mode for every entry. The pure proptest on the 4-line match function is insufficient by itself.
.docs/plans/dos-546/phase-0/13-wp-plugin-skeleton.md:98:- `languages/` contains translation catalogs for the `dailyos` text domain.
.docs/plans/dos-546/phase-0/11-editable-composition-overlay.md:426:- 12 negative fixtures — stale, revoked, unknown, unauthorized, and failure-mode
.docs/plans/dos-546/phase-0/07-custom-block-fallback-projection.md:460:When no candidate has positive score, render as generic `dailyos/text`.
.docs/plans/dos-546/phase-0/07-custom-block-fallback-projection.md:833:If no nearest-known type has a positive deterministic match, or if the selected
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:18:Closure location in v2: §2 paragraph 3 (single PR, no feature flag, no backward-compat default; PR includes `dos209_mutation_catalog.rs` generated from the audit script; mixed final state forbidden, mid-PR working-tree may be temporarily compile-broken) and §3's full mutation catalogue table (24 service modules, ~210 mutator rows enumerated with kinds D/SQL/TX/SIG/FS/BG/EXT/C and coverage column).
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:19:Verification: All three sub-asks are answered. (a) Enumeration mechanism: audit script + checked-in `dos209_mutation_catalog.rs` structural lint, with the catalogue table in §3 as the human-readable artifact. (b) Mid-PR state: explicitly local-only compile-broken windows allowed, but the reviewable PR forbids mixed signatures. (c) `test_live()` scope: `#[cfg(test)]`-only, "not a production migration shim" — guard in CI. The catalogue-as-table is materially stronger than cycle 1's "grep audit" prose; it is now reviewable and mechanically checkable.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:22:Closure location in v2: §7 paragraph 2 ("W2-B opens/lands first per coordination guidance. W2-B owns provider extraction... W2-A then rebases and touches only service mutation boundaries in `services/intelligence.rs`...") with the explicit residual mutator list (`enrich_entity`, `persist_entity_keywords`, `upsert_assessment_from_enrichment`, `upsert_assessment_snapshot`, `upsert_health_outlook_signals`, `upsert_inferred_relationships_from_enrichment`, `update_intelligence_field`, `update_stakeholders`, `dismiss_intelligence_item`, `recompute_entity_health*`, `bulk_recompute_health`, `track_recommendation`, `dismiss_recommendation`, `mark_commitment_done`) and the protocol "If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2."
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:23:Verification: Explicit merge order chosen (W2-B first). The protocol covers the cross-file guard-preservation concern from cycle 1: any function W2-B extracts must retain the guard at L2 review time, with W2-A as the catalog owner. Closed.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:39:- `src-tauri/tests/dos209_mutation_catalog.rs::catalog_every_mutator_guarded_first_line` (structural lint over the catalogue)
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:40:- `src-tauri/tests/dos209_mode_boundary.rs::evaluate_catalog_public_mutators_return_write_blocked_by_mode` (runtime, iterates public mutators under Evaluate)
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:42:Verification: Cycle 1 asked for a contract test that enumerates every public mutator and asserts `WriteBlockedByMode` under Evaluate. v2 splits into two layers (structural lint over the full catalogue, runtime over the public subset), which is stronger than the single-test ask. The proptest is retained for the 4-line match but no longer load-bearing. Closed.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:45:Closure location in v2: §6 ("Services-only mutations: this PR activates the W2 CI enforcement mechanism from the wave invariants table. The guard catalogue prevents unguarded service mutators; lint rejects raw clock/RNG calls in `services/` and `abilities/` even though `src-tauri/src/abilities/` does not exist yet.") and Intelligence Loop 5-answers paragraph.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:54:**O1 — Catalog table coverage column reads "catalog+runtime" uniformly without distinguishing "public mutator" from "internal helper".** The runtime test (`evaluate_catalog_public_mutators_return_write_blocked_by_mode`) iterates *public* mutators only, but several catalogue rows (e.g. `update_account_field_inner`, `update_stakeholder_engagement_inner`, `add_stakeholder_role_inner`, `persist_and_invalidate_entity_links_sync`) are inner helpers reachable only via their public wrappers. These will not be hit by the runtime test directly, only via the structural lint. v2 already says "Evaluate runtime coverage where public" in §3, so this is documented; the catalogue column just doesn't visually distinguish. Not load-bearing — the structural lint covers them. No action needed.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:56:**O2 — `dashboard.rs` row says "no production mutation; `get_dashboard_data_inner` is clock-only and must use `ctx.clock`" with coverage "lint".** This is fine but worth flagging: if any future change adds a mutation to `dashboard.rs`, the catalogue must be updated. The structural lint only checks listed entries; un-listed mutators in un-listed files would slip. This is a known property of allow-list lints, not a v2 defect.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:69:No new architectural drift introduced by cycle-1 closures. The changes between v1 and v2 are additive specifications and a comprehensive mutation catalogue; no carrier was removed, no signature changed, no consumer-facing type was renamed. The freeze is reviewable, mechanically enforceable (catalog + runtime + lint + trybuild), and well-bounded by §10's narrow remaining-uncertainty list (DOS-304 interpretation + HRTB fallback).
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:73:APPROVE. All nine cycle-1 findings are closed at the architectural level — eight fully, one (F9) acceptably re-scoped to the W2 invariant DOS-209 actually owns. The mutation catalogue in §3 is materially stronger than v1's prose and is the right artifact for a 24-module sweep. The seam shape (eight carriers, IntelligenceProvider excluded, concrete `ExternalClients`, `TxCtx` with no external/provider) is consumable by W3-A, W3-B, W3-G, W4-A, W4-C, and W5 without breaking-change risk. Migration discipline (one PR, no feature flag, no backward-compat default, structural lint + runtime + trybuild + direct-clock/RNG regex lint) addresses the highest-blast-radius operation in the project with the right combination of mechanical and runtime enforcement. Failure modes (HRTB slippage, capability leakage, missed guard) all have named fallbacks. No High or Critical fresh finding. L6 escalation is not warranted.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v2.md:82:4. **Mutation catalogue is checked-in and mechanically verified.** `dos209_mutation_catalog.rs` structural lint plus `evaluate_catalog_public_mutators_return_write_blocked_by_mode` runtime coverage means a missed migration is a CI failure, not a runtime surprise. The ~210-row catalogue in §3 is the human-readable artifact reviewers can scan.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-A-codex-consult.md:162:- Packet section 8 lists 14 negative fixtures at `.docs/plans/v1.4.3-wp-foundation/L0-packet-A-lifecycle-hardening.md:243-260`.
.docs/plans/wave-W2/DOS-209-plan.md:92:Migration is one PR with no feature flag and no backward-compatible default constructor. Local working tree may be temporarily compile-broken during the sweep, but the reviewable PR must not contain a mixed final state where some production mutators still take raw `&ActionDb`. The PR must include `src-tauri/tests/dos209_mutation_catalog.rs`, generated from the audit script, so the catalogue below is checked mechanically.
.docs/plans/wave-W2/DOS-209-plan.md:100:Errors migrate one-shot for production mutators in this catalogue: mutators return `ServiceError`, with command/Tauri/MCP edges converting to `String` at the boundary. `ServiceError` includes `WriteBlockedByMode(ExecutionMode)` and `NestedTransactionsForbidden`. Read-only services may stay on `String` in this PR.
.docs/plans/wave-W2/DOS-209-plan.md:108:Mutation catalogue from `scripts/dos209-mutation-audit.sh`; output below is generated verbatim from the current development branch tree and committed as `src-tauri/tests/dos209_mutation_catalog.txt`. Abbreviations: D = DB method write; SQL = raw SQL state change; TX = transaction wrapper; SIG = signal emission; FS = filesystem write/delete/rename; BG = background queue/in-memory side effect; EXT = external side effect; C = direct clock/RNG replacement. Required first line for every listed mutator is `ctx.check_mutation_allowed()?`; coverage for every row is `dos209_mutation_catalog` structural lint plus Evaluate runtime coverage where public. Cycle-2 challenge NF1 is closed by the committed audit script plus the §9 no-drift CI test that re-runs this script against the committed snapshot.
.docs/plans/wave-W2/DOS-209-plan.md:362:Services-only mutations: this PR activates the W2 CI enforcement mechanism from the wave invariants table. The guard catalogue prevents unguarded service mutators; lint rejects raw clock/RNG calls in `services/` and `abilities/` even though `src-tauri/src/abilities/` does not exist yet.
.docs/plans/wave-W2/DOS-209-plan.md:369:- Feedback hook: no new hook, but feedback mutations are in the guard catalogue.
.docs/plans/wave-W2/DOS-209-plan.md:381:W2-B owns provider extraction from `services/intelligence.rs`: provider selection, Glean chat, PTY fallback, prompt fingerprint/replay hooks, and `generate_risk_briefing` provider calls move toward `intelligence/provider.rs`, `glean_provider.rs`, and `pty_provider.rs`. W2-A then rebases and touches only service mutation boundaries in `services/intelligence.rs`: `enrich_entity` residual DB reset/queue side effects if still present, `persist_entity_keywords`, `upsert_assessment_from_enrichment`, `upsert_assessment_snapshot`, `upsert_health_outlook_signals`, `upsert_inferred_relationships_from_enrichment`, `update_intelligence_field`, `update_stakeholders`, `dismiss_intelligence_item`, `recompute_entity_health*`, `bulk_recompute_health`, `track_recommendation`, `dismiss_recommendation`, and `mark_commitment_done`. If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2. This closes cycle-2 challenge NF2.
.docs/plans/wave-W2/DOS-209-plan.md:387:Missed guard: blocked by `dos209_mutation_catalog.rs` structural lint and Evaluate runtime tests. Capability leakage: blocked by trybuild compile-fail tests. Transaction bug: fallback is sync-within-async with no `.await` inside SQLite write lock. Panic/error rollback is covered by tests.
.docs/plans/wave-W2/DOS-209-plan.md:389:Rollback is a single PR revert if no timestamp migration is included. W1-B universal write fence is honored: W2-A does not bypass `write_intelligence_json` or add new `intelligence.json` writers.
.docs/plans/wave-W2/DOS-209-plan.md:394:- `services::context::tests::proptest_check_mutation_allowed_modes`: generator is `prop_oneof![Live, Simulate, Evaluate]` crossed with `MutationAttempt { kind: D|SQL|TX|SIG|FS|BG|EXT, public_catalog_index, uses_clock, uses_rng }`; Live accepts, Simulate/Evaluate return `WriteBlockedByMode`.
.docs/plans/wave-W2/DOS-209-plan.md:397:- `src-tauri/tests/dos209_mode_boundary.rs::evaluate_catalog_public_mutators_return_write_blocked_by_mode`.
.docs/plans/wave-W2/DOS-209-plan.md:398:- `src-tauri/tests/dos209_mutation_catalog.rs::catalog_every_mutator_guarded_first_line`.
.docs/plans/wave-W2/DOS-209-plan.md:399:- `services::context::tests::mutation_catalog_no_drift`: re-runs `scripts/dos209-mutation-audit.sh` and asserts stdout exactly matches `src-tauri/tests/dos209_mutation_catalog.txt`; any drift breaks CI. This closes cycle-2 challenge NF1.
.docs/plans/wave-W2/DOS-209-plan.md:404:CI command: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && pnpm tsc --noEmit`. This restores the full regression suite and closes cycle-2 challenge NF3. Additional DOS-209 evidence command: `cargo test --manifest-path src-tauri/Cargo.toml dos209`; this targeted invocation supplements the full regression suite and is not a replacement. Wave artifact: W2 proof bundle includes clippy/test output, mutation catalogue diff, direct clock/RNG lint output, and W1 Suite P comparison.
.docs/plans/wave-W2/DOS-209-plan.md:422:- `dos209_mutation_catalog.rs` (drift CI test) — deferred
.docs/plans/wave-W1/DOS-311-plan.md:28:**Agent slot:** W1-B
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:9:open_questions: none — fixture catalog is closed; downstream consumers (W5-C, W2-A) add fixtures as they implement
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:12:# 12 — Negative fixture catalog for Phase 1
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:16:Phase 1 needs explicit negative fixtures because the acceptance criteria name failure modes that can otherwise be missed by happy-path renderer tests. The WordPress Studio spike validates a trust boundary, not only a render path: the WP plugin is a `SurfaceClient`, the runtime owns `Composition`, provenance is canonical output, and WordPress is a projection target rather than the source of truth. These fixtures turn that contract into testable failure cases.
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:18:This catalog assumes the Wave 2 contracts are split as follows:
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:32:These fixtures should be implemented as negative tests against the boundary that fails, not only as end-to-end browser checks. Each fixture should have the narrowest test that proves the guard and at least one integration assertion that proves the user- or client-visible behavior.
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:38:- Seed one positive control when the fixture depends on a policy distinction.
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:409:- Host-model prompt injection through MCP tool descriptions. This catalog covers discovery and exposure denial; prompt-injection behavior belongs in ability eval and MCP tool-description review.
.docs/plans/dos-546/phase-0/12-negative-fixtures.md:411:- Data-loss recovery after disk failure. This catalog covers partial write safety and quarantine; backup/restore policy belongs in persistence operations work.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:15:2. The committed snapshot `src-tauri/tests/dos209_mutation_catalog.txt` (233 lines, 228 mutator rows + 5 comment headers) is byte-identical to the live script output. I ran `diff <(bash scripts/dos209-mutation-audit.sh) src-tauri/tests/dos209_mutation_catalog.txt` and confirmed exact match. The no-drift CI test (`mutation_catalog_no_drift` in §9) is therefore not stale-on-arrival; it will pass at PR open and any future drift will fail it.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:24:   The cycle-2 disagreement is structurally closed — not by re-asserting "exhaustive" prose, but by mechanically generating the catalogue from a parser whose taxonomy directly encodes §3's mutation definition.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:28:Failure modes for the script approach: an allow-list lint always carries the residual risk that a *new* file in `services/` containing only kinds the regex doesn't recognize would not appear. The current regex set is broad enough that new mutators using existing verbs are caught; entirely novel side-effect kinds (e.g., a new IPC channel) would not be. This is a known property of regex-based catalogue generation and is the same residual risk every grep-CI invariant in the wave plan carries. It is not a v3 defect — it is the structural shape of the closure the plan intends. The §9 `dos209_mode_boundary` runtime test is the second layer that catches mutators the structural lint missed (any `pub` function under `services/` invoked under `Evaluate` that does not return `WriteBlockedByMode` fails). Two layers, regex-based catalogue plus runtime evaluate boundary, are the right composition for this kind of sweep.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:30:Architectural verdict: the closure is sound. The script is the catalogue generator, the snapshot is the reviewable artifact, the no-drift test is the CI gate, and the runtime evaluate test is the second layer. This is the correct architectural shape for a 24-module 228-mutator sweep.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:47:The flip is architecturally net-positive (smaller mutation surface for the catalogue sweep after PTY orchestration extraction) and downstream-neutral (no consumer's compile dependency cares which W2 PR lands first — both shapes are frozen). The amendment preserves end-state alignment.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:49:§7 paragraph 4 also retains the cross-file guard-preservation protocol from cycle 2: "If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2." This is the right escape hatch — if W2-B's PTY extraction touches a function in the catalogue (e.g., the residual mutator list `enrich_entity`, `persist_entity_keywords`, etc.), W2-A re-runs the audit script, regenerates the snapshot, and the moved function's first line still reads `ctx.check_mutation_allowed()?`. Mechanically enforceable via the no-drift test.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:57:This matches CLAUDE.md's "Definition of Done" exactly (`cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`) and matches the W2 merge gate line "`cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green" in `v1.4.0-waves.md` §"Wave 2". The targeted `cargo test ... dos209` invocation is correctly framed as "supplements the full regression suite and is not a replacement," which is the right relationship — the targeted test set is evidence for the catalogue/mode-boundary/trybuild work, not a substitute for full regression.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:63:v3 freezes the same seam shape v2 froze (eight carriers, `IntelligenceProvider` excluded from `ServiceContext` and on `AbilityContext`, concrete `ExternalClients` with named fields, `TxCtx` with no external/provider) and adds three structural reinforcements: a programmatic catalogue generator + reviewable snapshot + no-drift CI test (NF1), a verbatim L6 amendment cite that aligns ticket and wave plan (NF2), and the full regression suite restored to the §9 CI line (NF3). None of the three changes alters a carrier, renames a type, or shifts an ownership boundary. They add enforcement and align contract truth — exactly the cycle-3 mandate.
.docs/plans/wave-W2/reviews/DOS-209-l0-architect-reviewer-v3.md:79:NF2's amendment cite is verbatim, anchored to the Linear issue id, preserves the original-vs-amended audit trail, and asserts "frozen contract per the L6 amendment" rather than v2's "per coordination guidance" (the language the cycle-2 challenge correctly flagged). The W2-B-first landing order is architecturally net-positive (smaller mutation surface after PTY extraction) and downstream-neutral for W3/W4/W5 consumers. The cross-file guard-preservation protocol is retained.
.docs/plans/dos-546/phase-0/05-ability-surface-inventory.md:20:surface-facing catalog entry that binds an existing ability contract to concrete
.docs/plans/dos-546/phase-0/05-ability-surface-inventory.md:550:at least one negative fixture that proves blocked vocabulary is rejected.
.docs/plans/v1.4.3-wp-foundation/reviews/packet-B-cycle2-code-reviewer.md:68:Fixture #5 (`dos671_first_mount_fires_one_reload`) covers the positive case
.docs/plans/_archive/shipped/v1.1.0-shipped.md:67:3. User clicks [Looks good] — the change is confirmed, positive feedback signal emitted. User clicks [Fix something] — inline correction UI opens (same pattern as intelligence field corrections). Correction emits `user_correction` signal (confidence 1.0), Bayesian weights penalize the source that got it wrong.
.docs/plans/_archive/shipped/v1.1.0-shipped.md:109:4. Accept updates field, changes source. Dismiss hides suggestion, emits negative feedback.
.docs/plans/dos-546/phase-0/14-gutenberg-block-account-overview.md:442:Additional negative paths: malformed Composition, unsupported child block
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:79:* **Canonical `AbilityPolicy` schema** — W0-D amends ADR-0102 §7.1 to (a) promote `mcp_exposure: bool` to the tri-state enum `McpExposure { None | MetadataOnly | Invocable }` and (b) keep `client_side_executable: bool` as a separate field governing SurfaceClient invocability per Phase 0 artifact 05 lines 389-412 (MCP exposure and SurfaceClient invocability govern different trust boundaries — an MCP tool may be invocable by an agent while a WP block hydrates a different ability that is never MCP-listed, and vice versa). W0-D also lands `required_scopes` as the two-level enforcement field. Two-level enforcement is enforceable substrate-side; the W1-B macro compile-error gate codifies the SurfaceClient + non-empty `required_scopes` discipline.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:87:* **Ability-surface inventory** — machine-readable catalog binding each DailyOS ability to its WP Abilities API registration, MCP exposure rule, scope requirements, and rendering copy. Per Phase 0 artifact 05.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:101:* **Negative fixture catalog** — Phase 1 ships the named failure cases from Phase 0 artifact 12 as actual tests at the boundary that fails, not only as end-to-end checks.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:123:* **Per-ability `required_scopes` population beyond v1.4.0/v1.4.1 defaults** — the W1-B schema lands the canonical fields and the macro compile-error for SurfaceClient-exposed abilities. Per-ability scope authorship (what scope each specific ability requires) lands per-ability in consuming releases (v1.4.3+), gated by the W1-B compile-error gate.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:154:3. Three-view consistency: substrate is the authority, WP DB is a projection, markdown is a durable read-side archive. Edit-in-WP and concurrent-tab scenarios from Phase 0 artifact 02 negative fixtures all pass; out-of-band edits in either WP DB or markdown from Phase 0 artifact 03 are detected, quarantined, and never silently promoted. Bidirectional markdown↔substrate edit propagation is named as v1.4.6 (Workspace Memory Refactor) scope and is NOT a v1.4.2 DoD criterion.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:155:4. Two-level enforcement: ability invocation outside `required_scopes` is rejected at `SurfaceClientBridge` before registry lookup; introspection outside `mcp_exposure` policy returns no ability metadata; both verified by negative tests from Phase 0 artifact 12.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:156:5. WP MCP Adapter only exposes the explicit DailyOS allowlist; the dedicated low-capability WP user for substrate access cannot reach abilities outside the allowlist; verified by negative tests from Phase 0 artifact 12.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:157:6. Pairing recovery: all four named threat paths (Reinstall, DB-Restore, Site-Switch, Exfiltration) ship with their named defense and a negative fixture per Phase 0 artifact 01.
.docs/plans/dos-546/v1.4.2-project/01-project-description.md:161:10. Negative fixture catalog from Phase 0 artifact 12 implemented; all named failure cases pass at the boundary that fails.
.docs/plans/wave-W1/DOS-310-plan.md:287:- **DOS-311 (W1-B)** owns: schema epoch table, universal `write_intelligence_json` fence, intel_queue drain protocol, reconcile SQL. **No file overlap** with this PR.
.docs/plans/wave-W1/DOS-310-plan.md:302:- **Universal write fence (DOS-311 / W1-B) interaction**: the DOS-310 bump runs inside `with_transaction`, which DOS-311's fence wraps. Compatible. The fence drains intel_queue but doesn't block claim writes; the bump runs as fast as a single SQL UPDATE.
.docs/plans/dos-546/phase-0/INDEX.md:22:| 04 | [`04-runtime-host-inventory.md`](./04-runtime-host-inventory.md) | Runtime host inventory: what Tauri owns, what WP owns, what MCP owns; runtime ownership + escape-hatch contract | W2-A, W1-B | spec:ready | Exact WP-↔-runtime transport; local vs remote WP trust assumptions; WP-invocable capability matrix without re-approval; Tauri MVP control surface; MCP health surfacing; `dailyos-mcp` stdio vs daemon; cross-component version/update coordination; cross-platform keychain provider plan; Tauri single-instance policy; legacy-command-to-ability migration list; WP-runtime-unavailable UX; Tauri-WP-down escape-hatch; support-bundle scope; per-op local confirmation list; "WP is primary" success criteria. |
.docs/plans/dos-546/phase-0/INDEX.md:30:| 12 | [`12-negative-fixtures.md`](./12-negative-fixtures.md) | Negative fixture catalog: every named failure case (auth, scope, nonce, version, signature, rate-limit, policy) + typed-error envelope contract | W5-C, W2-A | spec:ready | No artifact-internal open questions block promotion — fixture catalog is a closed enumeration; downstream consumers add fixtures as they implement, gated by W5-C. |
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:1:# L6 escalation: W2-A + W2-B plans hit cycle-2 cap; both surface a shared seam-ownership question + DOS-209 catalogue gap
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:18:### Disagreement 1 — DOS-209 mutation catalogue exhaustiveness
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:47:- **Mutation catalogue:** DOS-209's load-bearing invariant is "every mutator gets `check_mutation_allowed()?`." A non-exhaustive catalogue means W2-A ships with **ungated mutators** — the exact failure mode DOS-209 exists to prevent. This is the kind of substrate gap the W3 claims layer + W4 Trust Compiler + W5 pilots will compound on top of.
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:70:   - Re-audit the mutation catalogue using a programmatic script (commit it to the repo as `scripts/dos209-mutation-audit.sh` so future agents can re-run). Catalogue must include every match. Plan v3 commits the script, the catalogue output, and a test that re-runs the audit and asserts the catalogue file matches.
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:92:Ship plan v2 as-frozen. File three follow-up issues for v1.4.1: (1) Re-audit DOS-209 mutation catalogue post-W2 ship; (2) Re-route DOS-259 provider seam to `AbilityContext`; (3) Reconcile DOS-209 landing-order with wave plan. **Not recommended** — the catalogue gap is one-way-door; ungated mutators ship and W3+W4+W5 compose on broken substrate.
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:104:3. **DOS-209 catalogue:** confirm the script-based audit approach. Y/N.
.docs/plans/wave-W2/escalations/DOS-209-DOS-259-l6-decision.md:112:3. **Independent reviewer signal.** The cycle-2 reviewers did exactly the job they were designed for. The architect-reviewer caught architectural soundness; the consult caught the wrong-context provider routing; the challenge caught the catalogue gap and the landing-order conflict. The disagreement isn't reviewer noise — it's three independent lenses revealing real issues the v2 author missed. Strong signal.
.docs/plans/_archive/v1.1.0-dissolved.md:125:2. Frustrated customer parses as `customer: "negative"` and emits `transcript_sentiment` signal
.docs/plans/wave-W1/DOS-306-contract.md:3:**Status:** frozen W1 stage-1 contract. Implementation verification is deferred to `.docs/plans/wave-W1/DOS-306-verification.md` after W1-B/C/D merge.
.docs/plans/wave-W1/DOS-306-contract.md:26:W1-E imports the W0-A enrichment refactor design from historical commit `b2a24dc1` (`.docs/research/enrichment-refactor-design.md`). The imported amendments below are part of this frozen contract for W1-B, W1-C, and W1-D.
.docs/plans/wave-W1/DOS-306-contract.md:30:| DOS-235 signal policy registry distinguishes observation, invalidation, user feedback, and read-model-materialized signals | Accept | Every registry entry declares its signal role. Claim writes may enqueue downstream work only through the registry-declared invalidation path, and read-model-materialized signals must not recursively trigger uncontrolled propagation. | W1-B |
.docs/plans/wave-W1/DOS-306-contract.md:98:W1-B must write `.docs/plans/wave-W1/W1-B-channel-inventory.md` before refactoring emission call sites. The inventory is part of this contract, not an optional audit note.
.docs/plans/wave-W1/DOS-306-contract.md:212:### Contract artifact exists and gates W1-B/C/D
.docs/plans/wave-W1/DOS-306-contract.md:214:This document exists at `.docs/plans/wave-W1/DOS-306-contract.md`. W1-B, W1-C, and W1-D L0 plans cite it directly and either accept this contract or record an explicit amendment before implementation starts.
.docs/plans/wave-W1/DOS-306-contract.md:240:`.docs/plans/wave-W1/W1-B-channel-inventory.md` lists every signal-emission channel family in this contract, including all current `src-tauri/src/bin/` binaries. CI rejects direct production writes to `signal_events`, direct signal-originated job creation outside the allowlist, and new channel files that are absent from the inventory.
.docs/plans/wave-W1/DOS-306-contract.md:267:Implementation evidence is intentionally outstanding for W1-E. Per the v1.4.1 wave plan, the post-implementation verification document is separate and belongs at `.docs/plans/wave-W1/DOS-306-verification.md` after W1-B/C/D merge.
.docs/plans/wave-W1/DOS-306-contract.md:277:- `.docs/plans/v1.4.1-waves.md` Wave 1 stage-1/stage-2 ordering and W1-B/C/D done-when clauses.
.docs/plans/wave-W3/DOS-7-plan.md:99:W1-B universal write fence honored: intel workers capture/recheck `schema_epoch` via `FenceCycle` and `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:67-109`, `:223-238`). DOS-7 bumps epoch, drains, and rejects stale file writes rather than letting legacy `intelligence.json` overwrite claim-backed state.
.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md:232:- Missing, null, empty string, float, negative, object, array, or stringly typed `claim_version` rejects with `400 malformed_claim_version`.
.docs/plans/dos-546/v1.4.2-project/W4-E-L0-packet.md:860:47. `dos571_fixture_claim_version_wrong_type.rs` sends string/object/float/negative `claim_version` and receives `400 malformed_claim_version`.
.docs/plans/_archive/v0.13.1-shipped.md:120:1. Receive an email with negative sentiment from a contact at Acme. Check the Acme account detail page. The email's sentiment signal has been emitted to the entity signal graph — visible in the signal timeline or affecting the entity's health indicators.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:199:**Key acceptance shape:** History block renders user-relevant claim history (not raw audit log per receipt-vs-operational-audit boundary DOS-340); Email/Settings surfaces decided per W0; adversarial fixtures (DOS-446) exercise the negative-case matrix.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md:279:- Audit + clean-machine validation: DOS-575 negative fixtures, DOS-576 SurfaceClient audit attribution, DOS-577 clean-machine Studio validation.
.docs/plans/_archive/v1.0.1-dissolved.md:57:3. Renewal risk rating is one of: `low`, `medium`, `high`, `critical` — derived from `health_score`, `health_trend.direction`, and signal patterns. Not hallucinated. Verify: a healthy account with no risk signals gets `low` or `medium`; an account with multiple negative signals and a declining trend gets `high` or `critical`.
.docs/plans/_archive/v1.0.1-dissolved.md:89:4. The coaching language is factual and specific, never judgemental: "Accounts you met with 3+ times per month had a 60% higher positive signal rate than accounts you met with once or less." Not: "You should meet with accounts more often." Verify: the report reads as data-backed observation, not unsolicited advice.
.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md:33:- **Decision rule:** any negative fixture or W6-B clean-machine evidence must run against `~/Studio/dailyos-dev` or a fresh `studio site create`. Evidence against the marketing site is invalid because pre-existing content contaminates baselines.
.docs/plans/dos-546/v1.4.2-project/W3-L0-packet.md:100:  CI job runs against `wp/dailyos/**` on every PR; deliberate-violation negative fixture in phase-0 artifact 12 proves the gate triggers.
.docs/plans/dos-546/v1.4.2-project/W4-D-L0-packet.md:450:The implementation should include a negative fixture with sentinel values in
.docs/plans/wave-W3/DOS-299-plan.md:83:Rollback follows DOS-7's migration rollback posture: restore from pre-migration backup before completion; after completion, do not delete legacy JSON until projection parity passes. W1-B write fence is honored by running under the DOS-7 drain/schema-epoch cutover, so stale workers cannot overwrite backfilled claim state.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output:16:   711	- **Tests required:** Signal emission test for each of the five new variants (fixture-driven); typed payload struct has no `String` content/path fields (compile-time check); invalidation queue enqueued after `WorkspaceFileIngested` AND `WorkspaceFileQuarantined` AND `WorkspaceFileEntityLinkChanged`; invalidation queue NOT enqueued after `WorkspaceFileRejected` AND `WorkspaceFilePendingEntityAssignment` (negative tests); `from_name` round-trips for all five variants; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output:103:   322	| All untrusted input (markdown content, MCP write payloads, attachment file paths) passes through validation before service entry — no raw content reaches `commit_claim` without sanitization | Suite S test at W2 gate; explicit lint script `check_workspace_path_validation.sh` (W1-B owns) forbids `std::fs::File::open` / `tokio::fs::File::open` on workspace-rooted paths outside `registry::open_validated` | W1-B merge → W2-A merge |
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output:133:3. `WorkspaceCategory::from_slug()` has a W1-B dependency leak. Packet lines 38, 120, and 143 require registry-valid `Other` slugs, but the registry is owned by W1-B. W1-A can enforce canonical lexical slug shape; per-entity allowedness must be explicitly deferred to W1-B or this recreates wave-timing inversion.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle1.output:146:3. `WorkspaceCategory::from_slug()` has a W1-B dependency leak. Packet lines 38, 120, and 143 require registry-valid `Other` slugs, but the registry is owned by W1-B. W1-A can enforce canonical lexical slug shape; per-entity allowedness must be explicitly deferred to W1-B or this recreates wave-timing inversion.
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:6:Wave: 5 - Feedback + theme + negative fixtures
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:246:| `post_id` | integer | yes | positive integer; permission callback later requires `current_user_can('edit_post', post_id)` |
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:249:| `composition_version` | integer | yes | positive integer |
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:253:| `claim_version` | integer | yes | positive integer |
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:965:| W5-C | Negative fixture catalog | Hand off all `dos573_fixture_*` fixtures for bundle consolidation |
.docs/plans/dos-546/v1.4.2-project/W5-A-L0-packet.md:1039:8. W5-C fixture catalog receives the `dos573_fixture_*` list.
.docs/plans/wave-W3/DOS-294-plan.md:39:| `MarkFalse` | `claim_state=withdrawn`; broad tombstone/pre-gate if no source still qualifies | strong negative; agent `beta += 1.0` | modest source -0.05 one-shot | contradiction/tombstone reconcile | suppress default render; show only audit/history |
.docs/plans/wave-W3/DOS-294-plan.md:103:W1-B universal write fence honored: W3-E writes only inside service transactions, calls the DOS-7 invalidation primitive, and never writes legacy file projections directly.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle6.output:3:.docs/plans/v1.4.5-waves.md:102:1. **Shared types promoted to W1-A `contracts.rs`** (Eng cycle 5 #1). The `Extractor` and `SignalEmitter` trait signatures reference `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason`, and `SourceType`. If those types live in later-wave lanes (W3-A, W1-B respectively), the traits don't compile in W1-A. Cycle 6 moves the type definitions into `contracts.rs` (W1-A-owned). W1-B and W3-A then USE the types from `contracts` rather than defining them locally.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle6.output:9:.docs/plans/v1.4.5-waves.md:451:- **Done when:** `workspace_file_lifecycle` table exists with the seven-state lifecycle enum (including `pending_entity_assignment`), all required columns per the project DoD plus `category: Option<TEXT>` (cycle 8 Option B-prime — stores the auto-detected or caller-provided sub-category for AI navigation); Rust types compile inside `services::workspace_ingestion::lifecycle`; `services::workspace_ingestion::contracts` defines the **shared type surface** that every subsequent lane consumes (cycle 6 amendment promotes `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason` into `contracts.rs` because the `Extractor` trait signature would otherwise reference types owned by W1-B and W3-A; cycle 8 amendment adds `WorkspaceCategory`; **cycle 12 amendment** drops the `SourceType` mirror and consumes canonical `WorkspaceFileKind` directly + drops local `SourceAttribution` reinvention and consumes canonical 7-field `SourceAttribution` from `provenance/source.rs:303`):
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:149:10 sub-tickets organized into 4 sub-lanes (W1-A entity intelligence; W1-B meeting prep/readiness; W1-C briefing ability; W1-D claim receipt finishers). DOS-701 (claim_receipt substrate carve-out) is **Done** and is the precondition for W1-D.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:663:- `src-tauri/src/services/claim_receipt/auth.rs` (319 LOC) — `can_surface_for(state: &AppState, actor: &RenderActor, surface: RenderSurface, claim_id: &str) -> Result<(), AuthError>` (signature corrected per cycle-1 codex-consult F1) + positive/negative fixtures per sensitivity class.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md:804:- **AC-8.3** — Authorization matrix tested per row (positive + negative fixture per surface × actor × sensitivity).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:39:3. `FeedbackAction` enum reconciled to existing 9-variant substrate enum at `abilities-runtime/src/abilities/feedback.rs:31` (`ConfirmCurrent`, `MarkOutdated`, `MarkFalse`, `WrongSubject`, `WrongSource`, `CannotVerify`, `NeedsNuance`, `SurfaceInappropriate`, `NotRelevantHere`). The L0 Addendum's 8-vocab (`mark_wrong`, `should_not_show_here`) is a wire-label MISMATCH — cycle 2 adopts the substrate enum + its `to_storage_value` wire labels (`mark_false`, `surface_inappropriate`, `not_relevant_here`) as canonical. W1-B wire-format adapter, NOT substrate redesign.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:43:5. W1-A creates **EMPTY zero-byte placeholder** files for `feedback.rs` / `boundary.rs` / `privacy.rs` / `contradiction.rs` / `render_rules.rs` (cycle 2 adds the latter two to the W1-A skeleton so W3-B + W4-B don't force `mod.rs` edits later). W1-B/C/D and later W3-B/W4-B own the **contents** of their owned placeholder file; W1-A owns the file shells + `mod.rs` only.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:54:12. **Authorization matrix** (cycle 2 addition): `SurfaceContext` × actor kind × ability × `ClaimSensitivity` → allow/deny. One service-level helper (`services::claim_receipt::auth::can_surface_for(actor, surface, claim_id) -> Result<(), AuthError>`) reused by every receipt-rendering call site. Positive + negative fixtures per sensitivity class.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:58:14. **`metadata: Option<JSON>`** (W1-B `ClaimFeedbackRequest`) gets per-action JSON schemas (sourced from substrate `ClaimFeedbackMetadata` definition at `feedback.rs`), max 4KB serialized, max nesting 4, max keys 24, deny unknown fields. Audit-log copy excludes `metadata` raw — only schema-validated extracted fields propagate.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:64:18. **Privacy matrix split Activity vs Lint** with separate fixture suites for each surface; Operational audit becomes "storage/API non-disclosure" assertion class with its own positive/negative fixtures.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:130:- `services::claims::commit_claim` (sole writer for `intelligence_claims`); `services::claims::record_claim_feedback`; `services::claims::reconcile_contradiction` (W1-B confirms exact name at L0 plan via grep).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:280:Four agents. **W1-A (DOS-339 Shared Receipt DTO) merges first as stage 1a.** W1-B (DOS-8 semantic feedback), W1-C (DOS-340 receipt vs audit boundary), W1-D (DOS-341 privacy/redaction) all consume the DTO and run as **stage 1b in parallel after DOS-339 merges**.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:353:    pub struct ReceiptAction { /* see DOS-8 W1-B for full action enum */ }
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:356:  - **Empty zero-byte placeholder files** for `feedback.rs`, `boundary.rs`, `privacy.rs`, `contradiction.rs`, `render_rules.rs` (cycle 2 amendment: latter two added so W3-B + W4-B don't force `mod.rs` edits). W1-A creates the file shells + `mod.rs` declarations: `pub mod feedback; pub mod boundary; pub mod privacy; pub mod contradiction; pub mod render_rules;` (cycle 3 amendment per Eng nit: explicit list includes contradiction + render_rules). W1-B/C/D and later W3-B/W4-B own the **contents** of their owned placeholder file. This split-file/contents model resolves the cycle-1 finding that W1-A's "exclusive" file ownership conflicted with later lanes' "exclusive" content ownership.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:359:- **Don't touch:** `services/claims.rs` (consumer of receipt; not modified by W1-A); `services/feedback.rs`; W1-B/C/D submodule contents (they fill their own placeholders); `commands/*` (no command-layer changes in W1-A).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:367:  5. *Feedback loop:* Receipt carries `actions` Vec which consumers wire to W1-B's semantic feedback API. Roundtrip: action → service mutation → claim lifecycle transition → re-render via signal.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:371:### Agent W1-B — DOS-8: Semantic claim feedback actions using typed `claim_feedback`
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:374:- **Goal:** Ship the semantic-feedback action surface using the **existing typed `claim_feedback` substrate** — explicitly NOT a new `user_feedback_signals` table (the old DOS-8 spec proposed that; L0 doc explicitly rules it out). Wires the receipt's `actions` array to service-owned mutation APIs that route through `services::claims::record_claim_feedback` (W1-B confirms exact name at L0 plan via fresh grep).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:375:- **Files owned (exclusive):** `src-tauri/src/services/claim_receipt/feedback.rs` (placeholder created by W1-A; W1-B fills); function-level extension of `services::claims::record_claim_feedback` if new action variants are needed (W1-B's L0 plan documents which variants are reused vs added).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:376:- **Don't touch:** `services/claims.rs` core writer (W1-B is a caller-extender, not modifier); other W1 lanes' submodule files; existing `claim_feedback` schema (action variants extend the typed enum, not the table).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:379:- **Frozen action enum — cycle 2 reconciled to existing 9-variant substrate** at `src-tauri/abilities-runtime/src/abilities/feedback.rs:31`. v1.4.4 W1-B is a wire-format adapter, NOT substrate redesign:
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:381:  // Re-exported from abilities-runtime; v1.4.4 W1-B does NOT define a parallel enum
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:402:  **W1-B implementation:** consume `feedback_semantics(action)` for the proposed verification state + `apply_feedback_to_state(current, action)` for the actual transition (substrate handles the precedence rules). UI surfaces the user-label column; lifecycle/other-substrate columns explain expected backend behavior to design reviewers.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:441:  Each cell tested with positive + negative fixtures. Sensitivities use existing `ClaimSensitivity` enum (Public, Internal, Confidential, Restricted per ADR-0125).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:486:    | Operational audit (storage) | correlation ids, raw source ids, prompt/invocation metadata, tamper-evident record ids, debug payloads, raw model/tool i/o, raw document/message bodies (subject to audit policy + retention bound) | (no surface; storage only — non-disclosure assertion class with positive/negative fixtures) |
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:492:  5. *Feedback loop:* N/A directly; user feedback via W1-B may trigger render-policy re-evaluation (e.g., `WrongSource` action redacts source from future receipts).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:493:- **Tests required:** Suite S test for each row of the privacy matrix per surface (negative + positive fixtures); render-policy parity test for legacy v1.4.1 surfaces (no regressions on Entity Detail / Daily Briefing render output); `/cso` adversarial fixture suite.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:518:  3. User records one semantic feedback action from W1-B
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:524:- **Don't touch:** Shared receipt DTO (W1-A owns); feedback mutation API (W1-B owns); W3 queue/Activity/Lint UI (separate wave); v1.4.2 Entity Detail surfaces; v1.4.3 Briefing surfaces (Actions / Work is the v1.4.4 proof surface, not those).
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:532:  5. *Feedback loop:* User actions on rows route through W1-B's `ClaimFeedbackRequest`; UI re-renders on response receipt.
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:823:| 5 | Does `services::claims::record_claim_feedback` exist with that exact name, or is it `services::feedback::*`? W1-B routes there. | W1-B (DOS-8) | **Open** — defer to W1-B L0 plan with fresh grep |
.docs/plans/_archive/v1.4.4-waves-claim-experience-DISSOLVED-2026-05-17.md:827:| 9 | Does `check_claim_writer_allowlist.sh` need new allowlist entries for `services/claim_receipt/*` writes against `claim_feedback` table? | W1-B (DOS-8) | **Open** — defer to W1-B L0 plan |
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md:65:- **V1 (2026-05-13):** Initial L0 packet for DOS-589. Mirrors W4-B V9 section shape, inherits W4-B V9 sections 5/15/16/17/37, resolves dispatcher decisions, maps each Linear acceptance criterion to directional decisions and negative fixtures, and names the CI invariants needed to prevent scope-filter bypass.
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md:195:- `src-tauri/tests/dos589_fixture_*.rs` - negative fixtures and replay/backpressure tests.
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md:580:| AC | Directional decision(s) | Required negative fixture(s) |
.docs/plans/dos-546/v1.4.2-project/DOS-589-L0-packet.md:837:4. Every acceptance criterion cross-references a directional decision and at least one negative fixture.
.docs/plans/dos-546/v1.4.2-project/W3-handoff.md:107:- **DOS-586** — Forged-marker + clean-runtime → quarantine negative fixture (Medium)
.docs/plans/dos-546/v1.4.2-project/W3-handoff.md:108:- **DOS-587** — Unauthenticated REST schema introspection negative fixture (Medium)
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle6.output:3:362:| All untrusted input (markdown content, MCP write payloads, attachment file paths) passes through validation before service entry — no raw content reaches `commit_claim` without sanitization | Suite S test at W2 gate; explicit lint script `check_workspace_path_validation.sh` (W1-B owns) forbids `std::fs::File::open` / `tokio::fs::File::open` on workspace-rooted paths outside `registry::open_validated` | W1-B merge → W2-A merge |
.docs/plans/wave-W3/DOS-296-plan.md:97:Rollback for an applied additive migration is restore from the migration safety backup created before pending migrations (`src-tauri/src/migrations.rs:1019-1026`) or leave the unused nullable column in place. No data rewrite is required. W1-B universal write fence is honored because DOS-296 performs no `intelligence.json` writes; if bundled with DOS-7 cutover, it runs inside the same schema-epoch/drain envelope that `write_fence::bump_schema_epoch` and `drain_with_timeout` provide (`src-tauri/src/intelligence/write_fence.rs:120-155`, `:223-238`).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:10:All cycle-1 advisory drift resolved. The one apparent NEW-DRIFT codex flagged is a false positive on inspection (see Check 4).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-codex-consult-cycle2.md:32:Packet line 11 reads "Branch base: `wave/v1.4.4-w1-stage1a` at HEAD `c5c0578f`." Current `git rev-parse wave/v1.4.4-w1-stage1a` → `ffccb485`, which is the V1.1 packet commit itself (`docs(v1.4.4 W2): V1.1 fold…`). A packet cannot cite the commit that introduces it as its own base ref; `c5c0578f` is the correct semantic base — the last substrate commit before this packet revision. The drift codex flagged is a false positive arising from inspecting branch HEAD without distinguishing substrate base from packet self-commit.
.docs/plans/wave-W3/_prompts/_common.md:73:What breaks if migration/projection fails midway. Rollback path. Honors the W1-B
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-consult.output:1:.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:251:4. **Runtime + surfaces.** Registry consumed by W2 ingestion (`pipeline::run` calls `open_validated`) + W4-A source-management block (category picker).
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle2/codex-consult.output:2:.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:258:- **W4-A (DOS-472)** consumes `WorkspaceCategoryRegistry` for the category-picker UI.
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md:386:3. **Failure-path writes PRESERVED:** Site-Switch (site_nonce_mismatch, site_binding_mismatch) and Exfiltration (wp_user_hash_mismatch) failure paths still call `suspend_pairing`, transitioning `surface_client_pairings.lifecycle_state` to `suspended` per Phase 0 artifact 01. Verified by negative fixtures (`dos655_site_switch_suspends_pairing`, `dos655_exfiltration_suspends_pairing`).
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md:390:7. **Session validity rule:** Determined by `revoked_at IS NULL AND absolute_expires_at > ?now AND lifecycle_state = 'active'`. `inactive_expires_at` is no longer consulted. Verified by negative fixture: a session row with `inactive_expires_at` in the past but `absolute_expires_at` in the future still validates.
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md:410:    - Verified by unit tests + negative fixture `dos655_sentinel_substitution_detected_via_hmac_fail`.
.docs/plans/dos-546/v1.4.2-project/W4-F-L0-packet.md:478:### With W5 (Feedback + theme + negative fixtures)
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:740:The drawer surfaces 10 render channels per ADR-0130 §3.1. Each must pass the negative fixture asserting no disallowed field leaks:
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:747:- AC-689.2: Class-sweep negative fixture: all 10 channels covered.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:767:- AC-691.2: DOS-477 10-channel negative fixture green: tooltip body + data attributes + ARIA labels carry no disallowed source-internal data.
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:830:**AC #W2.5 — Receipt rendering audience-keyed.** Every claim-bearing inner block routes through `build_receipt_for_audience` (W1 DOS-341); AgentMcp audience field-allowlist row passes negative fixture (per W1 AC-341.12). **AgentMcp touchpoint render = Option B aggregate (V1.1 lock).** `dailyos/touchpoints-feed` under AgentMcp emits `{ count, recency: Recent|Aging|Stale, content: redacted }` only — NO per-item rows, NO touchpoint titles, NO `source_asof` timestamps, NO per-item trust bands. Recency tier resolves coarsely from the touchpoint set (no per-row timing oracle). Negative fixture asserts the aggregate-only shape (per W1 AC-341.12 + V1.1 §10 invariant "AgentMcp touchpoint aggregate render").
.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:977:Rationale (overrides V1.0 Option A): even a placeholder per-row entry with timestamp + trust band leaks an existence + timing oracle to the agent. The aggregate signal preserves coverage (agent knows there's activity) without per-item leakage. AC-W2.5 + §10 invariant "AgentMcp touchpoint aggregate render" enforce; negative fixture per W1 AC-341.12 asserts the shape.
.docs/plans/wave-W3/DOS-301-plan.md:120:W1-B universal write fence is honored: all `intelligence.json` projection writes go through `fenced_write_intelligence_json`. Direct `write_intelligence_json` remains implementation-only inside `intelligence/io.rs` and `intelligence/write_fence.rs` (`scripts/check_write_fence_usage.sh:12-22`).
.docs/plans/dos-546/v1.4.2-project/W4-A0-L0-packet.md:66:- Adds negative fixtures for missing account id, unauthorized scope, empty claim set, and absent trust band.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:8:**Success metric:** Clean-machine install on a previously-untouched macOS test box reaches a rendered daily briefing in WordPress Studio in ≤15 minutes of user time, with the four Phase 0 contract surfaces (concurrency, tamper, pairing, rate-limits) all proven by negative fixtures + manual exercise.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:12:DailyOS is a runtime. The runtime renders into surfaces. Per ADR-0129, the primary composable surface is WordPress Studio. v1.4.2 is the foundation release: it does NOT ship rich entity intelligence (that's v1.4.3 on this foundation); it ships the load-bearing contract — `SurfaceClient` actor class, canonical `AbilityPolicy`, `Composition` substrate types, loopback transport, pairing model, custom MCP server, first block, three-view consistency, negative fixtures, clean-machine validation. Every v1.4.3+ wave consumes what this release commits.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:87:* **Negative-fixture catalog from artifact 12 is mandatory release-gate scope, not nice-to-have.** **Six** new fixture bundles (19-24) join bundles 1-18 from v1.4.1. The sixth bundle (24, audit attribution) was promoted to mandatory after L0 cycle-2 (audit schema moved to W1-A0 stage-1 issue).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:111:| **W5** — Feedback + theme + negative fixtures | 3 (save-time feedback router, magazine theme, negative fixture catalog) | 3 | 1 week |
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:127:* **W1 internal staging.** ADR-0130 amendments (W0-A) must land before W1-E (Composition contract substrate types) starts. ADR-0102 §7.1 amendment (W0-D) must land before W1-B (AbilityPolicy canonical schema) starts. Inside W1, **stage-1a** = W1-A (SurfaceClient actor) + W1-B (AbilityPolicy) run in parallel (file-disjoint at `actor.rs` vs `policy.rs`). **Stage-1b** = W1-A0 (audit-log schema + `emit_surface_audit` helper) starts once W1-A merges — its helper signature pattern-matches `Actor::SurfaceClient { instance, .. }` and cannot compile without W1-A. W1-A0 can land in parallel with W1-B once W1-A has merged. All three (W1-A, W1-A0, W1-B) must merge before stage-2 starts. **Stage-2** = W1-C (inventory) + W1-D (description CI gate) + W1-E (Composition types) running in parallel after stage-1. Inventory + Composition reference SurfaceClient + AbilityPolicy + audit schema. (L0 cycle-3 finding #2 corrected stage-1 ordering: cycle-2 had described W1-A0 as W1-A-independent, but the helper's `Actor::SurfaceClient` pattern-match makes the type dependency hard.)
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:129:* **W3 internal staging.** Plugin skeleton (W3-A) before Runtime client + HMAC + pairing UI (W3-B); custom MCP server (W3-C) runs in parallel with W3-B once W3-A merges. **W3-A gates both W3-B and W3-C. W3-B additionally depends on W2 full merge. W3-C additionally depends on W2 full merge + W1-B + W1-C merges** (W1-B + W1-C precede W2 in the default merge-gate-on-prior-wave rule, but stated explicitly per architect cycle-1 finding B1 because the dependency is load-bearing).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:131:* **W5 free fan-out.** Feedback router (W5-A) depends on W4-A + W4-E. Theme (W5-B) is independent. Negative fixture catalog (W5-C) runs from W5 start but **closes last** — W5-C is the last issue to merge in W5; consolidation runs after W5-A and W5-B merge, and bundle 19-24 release-gate integration is the closing act (per architect cycle-1 finding B3).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:147:| No DailyOS substrate-backed ability exposed by the default WP MCP server | negative fixture run as part of release gate | W3-C merge |
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:153:| `AbilityPolicy.mcp_exposure` is the canonical tri-state `McpExposure { None | MetadataOnly | Invocable }` AND `AbilityPolicy.client_side_executable: bool` is a separate field governing SurfaceClient invocation (per artifact 05 lines 389-412); the two fields are independent | type test pins `McpExposure` enum shape; type test pins `AbilityPolicy` struct fields including `client_side_executable: bool` | W0-D merge (doc); W1-B merge (code) |
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:202:* **Don't touch:** any Rust types, ability code, inventory format Rust struct (W1-B + W1-C own those).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:223:* **W1-B (AbilityPolicy canonical schema)** lands `required_scopes` + `mcp_exposure: McpExposure` + `client_side_executable: bool` per W0-D's amended ADR-0102 §7.1 in `policy.rs`. Two independent fields per artifact 05 lines 389-412.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:226:* **W1-A0 (audit-log schema for SurfaceClient attribution)** lands the schema migration + `emit_surface_audit` helper + round-trip plumbing skeleton. **W1-A0 starts after W1-A merges** because its helper signature pattern-matches `Actor::SurfaceClient { instance, .. }` and will not compile without the variant. W1-A0 may run in parallel with W1-B (file-disjoint).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:228:All three (W1-A, W1-B, W1-A0) must merge before stage 2 starts. Stage 1 = 2-3 day window (1 day W1-A solo, then 1-2 days W1-A0 + W1-B parallel). Updated in L0 cycle-3 from cycle-2's "all three parallel from t=0" framing — codex-consult R3 + codex-challenge #2 both flagged the implicit W1-A0 → W1-A type edge.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:239:* **Don't touch:** `Actor` enum (W1-A owns); `AbilityPolicy` (W1-B owns); inventory format (W1-C); description CI gate (W1-D); Composition types (W1-E); the forensic-exercise + CI lint (W6-A owns).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:240:* **Depends on:** **W1-A merged** (the helper signature pattern-matches `Actor::SurfaceClient { instance, .. }`; updated in L0 cycle-3 from cycle-2's "no W1 dep" framing). File-disjoint with W1-B (parallel with W1-B once W1-A lands). Architect-reviewer cycle-1 finding + codex-consult + codex-challenge cycle-1 all converged on this moving earlier than W6; cycle-3 codex-consult R3 + codex-challenge #2 corrected the staging.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:246:* **Files owned:** `abilities-runtime/src/actor.rs` (`Actor::SurfaceClient` variant + `SurfaceClientId` + `ScopeSet` types). Audit log emission sites for actor-aware events. Eval fixture for SurfaceClient negative case.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:247:* **Don't touch:** `AbilityPolicy` struct (W1-B owns the schema extension); inventory format (W1-C); description CI gate (W1-D); Composition types (W1-E).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:250:### Agent W1-B — AbilityPolicy canonical schema: `required_scopes` + `mcp_exposure`
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:260:* **Depends on:** W1-A + W1-B merged.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:262:* **Don't touch:** `AbilityPolicy` (W1-B); actor types (W1-A); description scan rules (W1-D).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:276:* **Depends on:** ADR-0130 amendments merged (W0-A); SurfaceClient (W1-A) + AbilityPolicy (W1-B) merged.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:284:* L0 plan approvals (architect-reviewer for W1-A0/W1-A/B/E; spec-writer for W1-C; code-reviewer for W1-D). **/cso added for W1-A0, W1-A, W1-B, W1-E** (trust-boundary primitives — W1-A0 is the audit-attribution gate).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:311:* **Done when:** every request through `/v1/surface/*` requires a valid HMAC before reaching ability dispatch; negative tests from artifact 08 §"Negative cases" + artifact 12 all green.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:319:* **Done when:** all four named defenses (Reinstall, DB-Restore, Site-Switch, Exfiltration) implemented with negative tests from artifact 12; `POST /v1/pairing/handshake` accepts pairing code → returns session material; revocation flow live.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:367:* **Done when:** custom MCP server registers under the DailyOS namespace; default WP MCP server does NOT expose DailyOS abilities (negative fixture green); allowlist enforced; permission callbacks check both WP capability + DailyOS scope.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:402:* **Depends on:** W1-A (SurfaceClient actor), W1-B (AbilityPolicy schema), W1-E (Composition types), W4-B (composition_version source).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:418:* **Done when:** server-assigned `claim_version` works end-to-end; stale writes get `409`; concurrent writes resolve to one success + one `409`; negative fixtures from artifact 02 green.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:427:* **Done when:** Ed25519 signatures issued on every projection write per artifact 03 (no HMAC, no algorithm negotiation); offline verification on read detects out-of-band edits; key lifecycle (unknown-key refresh, revocation, replacement-key provisioning, queued re-sign, retired-key historical verification) all implemented per artifact 03 §"Fixture C" + §"Out-of-Band Detection"; quarantine preserves tampered state; banner on render when tamper detected; negative fixtures from artifact 03 green.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:458:# Wave 5 — Feedback + theme + negative fixtures
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:466:* **Don't touch:** block code itself (W4-A owns); nonce issue/verify (W4-E owns); theme (W5-B); fixture catalog (W5-C).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:467:* **Done when:** user-edited block triggers feedback event; runtime applies via existing claim/feedback service; block re-renders with corrected state; negative cases from artifact 12 §Feedback all green; CI lint asserts no direct claim-table write from plugin source.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:476:### Agent W5-C — Negative fixture catalog implementation
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:478:* **Spec:** Issue `Negative fixture catalog implementation: every named failure case from artifact 12`.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:480:* **Files owned:** `tests/v142_fixtures/` (or equivalent), `.docs/plans/dos-546/v1.4.2-project/fixture-catalog.md`, release-gate config extension for bundles 19-24.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:489:* L3 wave adversarial: codex-challenge + **Suite S full re-run** (feedback writes through SurfaceClient never bypass nonce; theme doesn't leak via CSS injection / SVG payload paths; fixture catalog covers all artifact-12 named cases) + **Suite E** (bundles 1-18 from v1.4.1 still green + bundles 19-24 from v1.4.2 mandatory).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:519:* All four threat paths from Phase 0 artifact 01 demonstrated by negative fixture + manual exercise.
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:525:* **WP MCP Adapter exposure policy verified by negative fixture** (no substrate ability in default server).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:526:* **Two-level enforcement verified by negative fixture** (insufficient scope rejected; mcp_exposure: None not enumerated).
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:577:- [ ] negative-fixture log per artifact 12 cases addressed in this wave
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:632:- [ ] Pairing service: all four named defenses (Reinstall, DB-Restore, Site-Switch, Exfiltration) with negative fixtures
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:633:- [ ] Rate-limit matrix enforced at SurfaceClientBridge across five axes with negative fixtures
.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md:642:- [ ] Custom MCP server registers; default WP MCP server does NOT expose substrate abilities (negative fixture green)
.docs/plans/wave-W3/_prompts/DOS-301.md:40:- `src-tauri/src/intelligence/write_fence.rs` (W1-B, already merged) — wrapped `write_intelligence_json`
.docs/plans/dos-546/v1.4.2-project/W3-handoff-v2.md:111:- DOS-586 — Forged-marker + clean-runtime → quarantine negative fixture (Medium)
.docs/plans/dos-546/v1.4.2-project/W3-handoff-v2.md:112:- DOS-587 — Unauthenticated REST schema introspection negative fixture (Medium)
.docs/plans/wave-W3/DOS-210-plan.md:19:Current repo reality: W2-A and W2-B are present. `ServiceContext` exists with `ExecutionMode`, clock/RNG, external clients, and private transaction handle (`src-tauri/src/services/context.rs:271-277`), and `check_mutation_allowed()` rejects non-Live writes (`src-tauri/src/services/context.rs:412-422`). W2 proof says every catalogued service mutator now takes `ServiceContext` first and gates first (`.docs/plans/wave-W2/proof-bundle.md:155-158`). The provider seam already reserves `select_provider(ctx: &AbilityContext, tier)` for W3-A (`src-tauri/src/intelligence/provider.rs:14-22`, `:309-326`).
.docs/plans/wave-W3/DOS-210-plan.md:31:AST enforcement is implemented in `abilities-macro/src/scoring.rs::mutation_allowlist()`. Do not put this in `src-tauri/src/signals/scoring.rs`: that file is runtime item relevance scoring (`src-tauri/src/signals/scoring.rs:1-7`) and would create a proc-macro dependency cycle. The macro crate build script reads `src-tauri/tests/dos209_mutation_catalog.txt`, whose header documents the Rust scanner and mutation regex method (`src-tauri/tests/dos209_mutation_catalog.txt:1-6`), and generates a static list of fully qualified service mutator paths. The visitor detects `services::*`, `crate::services::*`, and syntactic alias tracking for direct `use` imports to allowlisted service functions. Read/Transform with a detected mutation emits `compile_error!`; Publish/Maintenance record the inferred mutation set in `AbilityDescriptor::mutates`. Experimental abilities still record the set but do not block.
.docs/plans/wave-W3/DOS-210-plan.md:47:Mutation allowlist location: `abilities-macro/src/scoring.rs::mutation_allowlist()` generated from `src-tauri/tests/dos209_mutation_catalog.txt`, not runtime `signals::scoring`. Reason: proc macros cannot depend on the main crate; the W2 catalog is already the audited service mutation source (`src-tauri/src/services/context.rs:21-28`, `.docs/plans/wave-W2/proof-bundle.md:155-158`).
.docs/plans/wave-W3/DOS-210-plan.md:75:Macro AST scanning is compile-time only. The mutation allowlist generated from the 228-mutator catalog is a static slice; visitor cost is proportional to function body AST size. Link-symbol uniqueness adds no runtime cost. Documentation generation is offline and deterministic.
.docs/plans/wave-W3/DOS-210-plan.md:91:DOS-216 consumes registry enumeration for fixture harness discovery. DOS-217 consumes `invoke_by_name_json`, `iter_for(Actor::Agent)`, and descriptors for Tauri/MCP bridges. DOS-218+ consume the macro for real abilities. W3-C/D/E/F/G/H do not share files with W3-A, but W3-C's `services::claims::commit_claim` will be picked up by the generated mutation allowlist once the W2 mutation catalog is regenerated.
.docs/plans/wave-W3/DOS-210-plan.md:97:If docs generation fails, it does not block runtime invocation unless CI is running the docs snapshot test. If inventory registration breaks, registry count is zero and `ability_registry_self_check` fails. No SQL migration is involved. W1-B universal write fence is honored because W3-A introduces no new DB/file write path; future Maintenance abilities still mutate only through services and the existing fence.
.docs/plans/wave-W3/DOS-210-plan.md:125:3. Cycle rejection wording: inventory graph cycles can be rejected by startup registry validation and CI before merge, but not by a single proc-macro expansion. Is that acceptable as "build time" for the ticket, or does architect-reviewer require a centralized non-inventory catalog?
.docs/plans/dos-546/v1.4.2-project/W3-L3-rerun-architect.md:71:- **MCP exposure is fail-closed at three layers.** (a) `wp_register_ability()` default `mcp_exposure: None` keeps abilities invisible to any generic adapter enumerator (negative fixture at `tests/mcp/McpExposureNoneTest.php`). (b) `class-dailyos-mcp-server.php:217-248` filters `mcp_adapter_tools_list` to deny enumeration of DailyOS abilities by non-DailyOS servers AND per-tool runs `DailyOS_Mcp_Permission::check` against the resolved scope set even at list time. (c) Tool invocation runs as the `dailyos_substrate` low-cap WP user via `switch_to_substrate_user()` regardless of caller identity (line 229, 263, 372, 387).
.docs/plans/v1.4.1-waves/W7-A-L0-packet.md:79:- `state_changed_to_done` (resolution signal — feeds positive trend)
.docs/plans/v1.4.1-waves/W7-A-L0-packet.md:91:- Issue resolution rate (Done / Total per period) contributes to positive trend.
.docs/plans/v1.4.1-waves/W7-A-L0-packet.md:101:- Multiple issues moved to `Done` for the meeting's subject (positive change worth surfacing).
.docs/plans/wave-W3/DOS-300-plan.md:55:The two `ALTER TABLE ADD COLUMN ... DEFAULT ...` migrations are metadata-only in SQLite for constant defaults, but they still take the writer lane and must run in the DOS-7 cutover window under the W1-B fence. Current write-fence primitives capture and recheck `schema_epoch` before stale `intelligence.json` writes (`src-tauri/src/intelligence/write_fence.rs:1-23`, `:67-109`).
.docs/plans/wave-W3/DOS-300-plan.md:87:W1-B universal write fence is honored through DOS-7's cutover: schema epoch bump, drain, backfill/ALTER, reconcile, requeue. Stale legacy file writes are rejected by `FenceCycle` and `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:67-109`, `:112-120`).
.docs/plans/dos-546/v1.4.2-project/W4-A-L0-packet.md:48:  - **Class-wide sweep:** all attribute discipline negative rules now also forbid scope-derived material (hashes, fingerprints, scope set identifiers). The post_content scope-leak class is closed at the schema layer, not by case-by-case patching.
.docs/plans/v1.4.5-w0-reuse-audit-2026-05-19.md:44:| `WorkspaceCategoryRegistry` (W1-B) | ✅ does NOT exist (clean slate) |
.docs/plans/wave-W3/DOS-211-plan.md:119:Rollback is mechanical because W3-B has no migration and no runtime write path: revert the provenance module and its tests, then W3-A/W5 callers stop compiling until they drop imports. The W1-B universal write fence is honored by construction: this module does not write DB rows, files, signals, or external APIs. If a later ability writes after using provenance, that write remains behind ServiceContext/W1-B gates.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:55:1. `data_source_json` seed shape is not canonical serde for `DataSource::WorkspaceFile`. Packet seeds `{"kind":"workspace_file","name":{"kind":"inbox"}}` etc. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:80), [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:88). Actual `DataSource` is externally tagged with only `rename_all`, and `WorkspaceFile { kind }` is the variant. [source.rs](/private/tmp/dailyos-v145-w1a/src-tauri/abilities-runtime/src/abilities/provenance/source.rs:71), [source.rs](/private/tmp/dailyos-v145-w1a/src-tauri/abilities-runtime/src/abilities/provenance/source.rs:81). Use serde-produced JSON, e.g. `{"workspace_file":{"kind":"inbox"}}`.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:57:2. Category seed count conflicts with locked contract. Wave plan says defaults are all six categories for the per-entity registry. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:534). Packet seeds person with only four, omitting `presentations` and `contracts`. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:104). This makes done-when “all default pairs” ambiguous/incomplete. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:185).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:59:3. Trait/import compileability is underspecified: packet signature references bare `EntityType` and `CategoryNotAllowed`. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:34). Existing canonical `EntityType` is `crate::entity::EntityType`. [entity.rs](/private/tmp/dailyos-v145-w1a/src-tauri/src/entity.rs:13). `CategoryNotAllowed` only appears as a future `PlacementError` variant, not a registry error type. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:937).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:61:4. Security negative fixtures miss the packet’s own strict-child root-equality case. Contract says canonical path must be a strict child, no equality with root. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:128). Fixture list has traversal, absolute path, symlink, race, but no workspace-root path/directory rejection. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:142).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:63:5. Stale lint marker: packet tells allowlist exceptions to use `dos-146-allowed`, not DOS-464 or a neutral marker. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:151). This is inherited from wave text. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:542).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:70:1. `data_source_json` seed shape is not canonical serde for `DataSource::WorkspaceFile`. Packet seeds `{"kind":"workspace_file","name":{"kind":"inbox"}}` etc. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:80), [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:88). Actual `DataSource` is externally tagged with only `rename_all`, and `WorkspaceFile { kind }` is the variant. [source.rs](/private/tmp/dailyos-v145-w1a/src-tauri/abilities-runtime/src/abilities/provenance/source.rs:71), [source.rs](/private/tmp/dailyos-v145-w1a/src-tauri/abilities-runtime/src/abilities/provenance/source.rs:81). Use serde-produced JSON, e.g. `{"workspace_file":{"kind":"inbox"}}`.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:72:2. Category seed count conflicts with locked contract. Wave plan says defaults are all six categories for the per-entity registry. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:534). Packet seeds person with only four, omitting `presentations` and `contracts`. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:104). This makes done-when “all default pairs” ambiguous/incomplete. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:185).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:74:3. Trait/import compileability is underspecified: packet signature references bare `EntityType` and `CategoryNotAllowed`. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:34). Existing canonical `EntityType` is `crate::entity::EntityType`. [entity.rs](/private/tmp/dailyos-v145-w1a/src-tauri/src/entity.rs:13). `CategoryNotAllowed` only appears as a future `PlacementError` variant, not a registry error type. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:937).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:76:4. Security negative fixtures miss the packet’s own strict-child root-equality case. Contract says canonical path must be a strict child, no equality with root. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:128). Fixture list has traversal, absolute path, symlink, race, but no workspace-root path/directory rejection. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:142).
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-challenge.output:78:5. Stale lint marker: packet tells allowlist exceptions to use `dos-146-allowed`, not DOS-464 or a neutral marker. [L0-packet-W1-B-DOS-464.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:151). This is inherited from wave text. [v1.4.5-waves.md](/private/tmp/dailyos-v145-w1a/.docs/plans/v1.4.5-waves.md:542).
.docs/plans/v1.4.5-workspace-memory/proof-bundle-W1-A.md:59:- **W1-B (DOS-464)** — `registry.rs` + `WorkspaceCategoryRegistry`. Slot **v252**. Security-annotated (`/cso`). Compiles against W1-A's `contracts::{FileIdentity, RejectionReason, WorkspaceCategory}`.
.docs/plans/v1.4.1-waves/W6-D-L0-packet.md:16:- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; bundles 1-13 are present; the harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-consult.output:35:    63	///   entity type) is W1-B's `WorkspaceCategoryRegistry::validate` boundary, not
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-consult.output:57:2. Implementability gap: `validate(category, entity_type: EntityType)` invents/uses `EntityType`, while §5 says W1-B does not define `EntityType` and uses existing string entity types. Pin the signature to `&str`/`String` or cite a real type. See packet lines 34 and 61-62.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle1/codex-consult.output:72:2. Implementability gap: `validate(category, entity_type: EntityType)` invents/uses `EntityType`, while §5 says W1-B does not define `EntityType` and uses existing string entity types. Pin the signature to `&str`/`String` or cite a real type. See packet lines 34 and 61-62.
.docs/plans/v1.4.1-waves/W6-A-L0-packet.md:15:- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; current corpus is `bundle-1` through `bundle-13`; harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.1-waves/W6-A-L0-packet.md:57:- **Read fixture catalogue.** Current bundles 1-13 are documented at `src-tauri/tests/fixtures/bundle-README.md:31-45`. Bundles 1, 3, 5, 11, 12, and 13 already exercise ability-level paths (`get_entity_context`, `prepare_meeting`, stale-source, revoked-source); W6-A's 5-path-per-ability matrix must cite which existing bundle covers each cell and which cells are new. Source: `src-tauri/tests/fixtures/bundle-README.md:29-45`.
.docs/plans/v1.4.1-waves/W6-A-L0-packet.md:80:   - `dos282_claim_canonicalization.rs` (exact, near, negative-non-collapse)
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md:16:- **Fixture catalogue:** the committed fixture corpus is documented at `src-tauri/tests/fixtures/bundle-README.md`; it currently describes `bundle-1` through `bundle-13`, requires hyphenated `bundle-N` directories, and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md:60:- **Mapped bundle number.** The fixture catalogue documents committed `bundle-1` through `bundle-13`, the harness currently expects bundles 1 through 13, and the `bundle-15` grep returned no existing fixture or test. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/tests/harness.rs:42-65`.
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md:62:- **Found the fixture catalogue.** The catalogue defines `metadata.json` manifest fields including `bundle`, `scenario_id`, `invariant`, `expected_render_policy`, `surfaces_exercised`, source lifecycle refs, anonymization cert, trust factors, and pass/fail definition. Source: `src-tauri/tests/fixtures/bundle-README.md:8-27`.
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md:206:  - Fixture catalogue/harness shape is followed.
.docs/plans/v1.4.1-waves/W6-C-L0-packet.md:243:- **V1 - 2026-05-15:** Initial W6-C L0 packet. Assigned DOS-290 to bundle 15, cited the W6 plan and Amendment 1, grounded fixture architecture in the bundle catalogue/harness, mapped daily readiness, meeting briefing, entity detail, dashboard, and MCP implementation anchors, quoted DOS-290 problem/user-harm/required-behavior/acceptance, decomposed acceptance criteria, and listed `qa-expert` as the only required L0 reviewer.
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:31:> "`observability::aggregate_metric::AggregateMetric` struct matches ADR-0120 §10 shape exactly. Const `AGGREGATE_METRIC_CATALOG` lists every permitted metric name; adding new entries requires PR review. Local hourly aggregation. Anon install ID at first launch. Opt-in surface: one-time splash on first launch. Settings → Privacy panel: toggle ON/OFF; first time ON shows a sample of last 24h. Active indicator: small footer icon when telemetry is ON. Disabling stops emission immediately + flushes buffer. Local buffer cap of 24 hours. HTTPS POST to DailyOS-operated collector endpoint with TLS. CI test: every emission site references a catalog name; forbidden fields cause compile error."
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:51:  - `src-tauri/src/observability/aggregate_metric/lint.rs` (new): CI compile-time check that all emission sites reference catalog names AND forbidden fields cause compile errors.
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:87:A `pub const &[&str]` listing every permitted metric name. Adding an entry requires a PR review (CI lint enforces that emission sites reference only catalog names). Initial catalog (subject to revision): correction_rate, ability_invocation_count, glean_availability_pct, ghost_resurrection_incidents, eval_replay_match_pct, etc. — exact list lives in the catalog itself, not duplicated here.
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:110:- What's sent (list of catalog names + their AggregateValue shapes).
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:152:// Compile error — name not in catalog:
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:227:  - The CI lint actually catches catalog drift + forbidden-field additions.
.docs/plans/v1.4.1-waves/W7-E-L0-packet.md:255:- Adding metric names beyond the initial catalog (each addition is its own PR review).
.docs/plans/v1.4.1-waves/W6-F-L0-packet.md:16:- **Fixture catalogue:** committed corpus at `src-tauri/tests/fixtures/bundle-README.md`; bundles 1-13 are present; the harness discovers only hyphenated `bundle-N` directories with `metadata.json`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:16:- **Fixture catalogue:** the committed fixture corpus is documented at `src-tauri/tests/fixtures/bundle-README.md`; it currently describes `bundle-1` through `bundle-13`, requires hyphenated `bundle-N` directories, and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:58:- **Mapped bundle number.** `ls src-tauri/tests/bundle*.rs` currently finds bundle tests for 2, 3, 4, 6, 7, and 8 only; no `bundle14` sibling exists. The fixture catalogue documents committed `bundle-1` through `bundle-13`, and the harness currently expects bundles 1 through 13. Sources: `src-tauri/tests/fixtures/bundle-README.md:1-6`, `src-tauri/tests/harness.rs:42-65`.
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:60:- **Found the fixture catalogue.** Search terms used across `.docs/plans/`, `.docs/design/`, `src-tauri/tests`, and `src-tauri/tests/fixtures`: `bundle catalogue`, `bundle catalog`, `fixture catalogue`, `fixture catalog`, `seed-bundle`, `seed bundle`, `validation bundle`, and `bundle [0-9]+`. The catalogue is `src-tauri/tests/fixtures/bundle-README.md`. Source: `src-tauri/tests/fixtures/bundle-README.md:1-27`.
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:201:  - Fixture catalogue/harness shape is followed.
.docs/plans/v1.4.1-waves/W6-B-L0-packet.md:236:- **V1 - 2026-05-15:** Initial W6-B L0 packet. Assigned DOS-289 to bundle 14, cited the W6 plan and Amendment 1, grounded fixture architecture in the bundle catalogue/harness, mapped ability and trust runtime anchors, quoted DOS-289 problem/user-harm/required-behavior/acceptance, decomposed acceptance criteria, and listed `qa-expert` as the only required L0 reviewer.
.docs/plans/v1.4.1-waves/W5-A-L0-packet.md:20:- ADR source: `.docs/decisions/0102-abilities-as-runtime-contract.md:50-77` names `get_daily_readiness` in the Read ability module catalog.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:211:ADR-0102's 2026-05-10 amendment line declares `mcp_exposure: bool`. ADR-0111 §8 echoes the bool form. The v1.4.2 W1-B canonical schema work + Phase 0 artifact 05 ship a tri-state `McpExposure { None | MetadataOnly | Invocable }` to capture the WP MCP server's "describe-but-don't-invoke" tier. The tri-state is the correct design (architect A1 + challenge #1 + /cso refinement P2 + consult §"do not defer client_side_exposure" all converge), but the ADR text is stale. Artifact 05 lines 383-412 explicitly resolves the EoP P2 naming concern by **keeping `mcp_exposure` and `client_side_executable` separate**: they govern different trust boundaries. `mcp_exposure` controls the network-facing MCP tool surface (where a host agent discovers tools and sends JSON input); `client_side_executable` controls whether a trusted in-product SurfaceClient (e.g., a WP block hydrating a composition) may invoke the ability after WordPress capability + runtime scope + actor checks pass. The two fields can disagree: a WP block may need to hydrate an ability that should never be MCP-listed, and an MCP tool may be invocable by an agent while the WP client only renders returned data and does not call the ability directly.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:215:W1-B implements the canonical AbilityPolicy schema and will land code that contradicts the ADR text if the ADR is not amended first. Two-source schemas drift; pinning the canonical source before W1-B starts closes the loop. L0 cycle-3 reversed the cycle-2 collapse of `client_side_executable` into `mcp_exposure` after re-reading artifact 05's explicit warning — the cycle-2 collapse would have pushed WP renderability toward MCP exposure, which is the exact rigidity Phase 0 warned against.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:219:This issue is a documentation change only. It amends ADR-0102 §7.1 and aligns artifact 05's status note with the amended ADR. No Rust types change in this issue (those land in W1-B).
.docs/plans/dos-546/v1.4.2-project/02-issues.md:377:* **Signals:** Touched indirectly — signal subject-ownership checks need to accept `SurfaceClient` correctly. The signal-bus contract from v1.4.1 W1-B/E remains the authoritative routing layer.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:392:* [x] Signal granularity — Touched. Signal subject-ownership checks must accept `SurfaceClient` correctly; v1.4.1 W1-B policy registry contract preserved.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:394:* [x] Evaluation harness — Touched. Eval fixtures gain a `SurfaceClient` actor case for negative tests.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:401:* Stale `SurfaceClientId` (paired then revoked) — Handled: revocation check at bridge boundary; covered by negative fixture from artifact 12.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:495:* Issue: W0-D `ADR-0102 §7.1 amendment: harmonize mcp_exposure to tri-state` (must merge first — W1-B implements the amended schema).
.docs/plans/dos-546/v1.4.2-project/02-issues.md:623:* Description with a legitimate term that happens to match the blocklist — escape hatch is the same as for any false positive: explicit allowlist annotation per existing protocol. No silent bypass.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:745:* `Origin` header guard: **positive allowlist, PHP-curl-primary** — the canonical caller in v1.4.2 is the WP plugin's PHP runtime client (`class-dailyos-runtime-client.php` per W3-B), which makes server-side cURL requests with no `Origin` header. The primary allow path is therefore **empty/absent Origin**. As defense-in-depth backup (NOT as an invitation to browser-direct calls), `Origin` whose origin (scheme+host+port) exactly matches the paired site's `site_url` (captured at first-pair per W2-C) is also accepted; this backup exists to keep the door open for narrow PHP-side proxies that some hosting stacks inject, not to admit browser-originated requests. Reject all other `Origin` values. Reject `Origin: null`. Host + Origin checks run before auth, rate limits, ability lookup, feedback handling. Per L0 cycle-3 codex-consult R1: the canonical transport model in v1.4.2 is PHP-only (artifact 15) — W3-A, W3-C, W4-E all describe browser→runtime as out of scope, and the W2-A guard is now phrased to match. Browser-originated requests reach the runtime only via the PHP plugin's REST endpoint → `class-dailyos-runtime-client.php` → loopback cURL path (see W4-E "transport model"). The positive-allowlist phrasing also prevents implementation drift toward inverted-allow logic (per /cso path-α #2).
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1182:* The custom server enumerates abilities with `mcp_exposure: Invocable | MetadataOnly` only; `mcp_exposure: None` abilities are not enumerated. Verified by negative fixture from artifact 12.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1185:* The default WP MCP server (the one registered automatically by the MCP Adapter for built-in abilities) is verified to NOT expose any DailyOS-namespaced ability; a negative test asserts this.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1254:* `dailyos/account-overview` ability declared via `#[ability]` macro with `category: Read`, `allowed_actors: [User, SurfaceClient]`, `required_scopes: [read.account_overview]` (per the W1-B compile-error gate), `mcp_exposure: McpExposure::Invocable`, `client_side_executable: true` (required for WP block hydration per W1-B SurfaceClientBridge gate; the field is independent of `mcp_exposure` per artifact 05 lines 389-412), `composition.produces_blocks: true`.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1300:* Issue: `Promote AbilityPolicy to canonical schema` (W1-B) — provides `required_scopes` + `mcp_exposure`.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1437:* **Signals:** Touched directly. Watermark change → signal emission → downstream invalidation. Consumes v1.4.1 W1-B/E policy registry contract.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1686:* WP block JS requests a fresh nonce when the user initiates a feedback action (clicks a "correct" or "dismiss" affordance). The nonce is bound to the click, not to the page load. **Transport model:** the WP block JS calls a WP REST endpoint (e.g., `/wp-json/dailyos/v1/nonce`); the WP PHP layer issues the HMAC-signed runtime request via `class-dailyos-runtime-client.php` (W3-B) to `POST /v1/surface/nonce/issue`. The browser does NOT call the runtime directly — the W2-A positive Origin allowlist would reject browser-originated requests with a WP-served Origin. This keeps browser-to-runtime trust mediated through PHP per artifact 15 §"transport."
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1734:## Wave 5 — Feedback + theme + negative fixtures (3 issues)
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1884:### Negative fixture catalog implementation: every named failure case from artifact 12
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1892:Phase 0 artifact 12 names ~30 negative fixtures across pairing, projection freshness, rate-limits, ability discovery, allowlist, SurfaceClient identity, nonce, Gutenberg renderer, custom block fallback, markdown projection, audit, diagnostics. Without these as actual tests, the foundation's failure modes are unverified.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1900:This issue lands every fixture named in artifact 12. Individual fixtures are owned by their producing issue (e.g., HMAC negative fixtures live in the HMAC issue), but the consolidation, the per-fixture catalog file, and the release-gate integration live here. New fixtures discovered during implementation are scoped here.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1907:  * Seed one positive control per fixture.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1912:* Fixture catalog file `.docs/plans/dos-546/v1.4.2-project/fixture-catalog.md` enumerates every fixture, its source (artifact 12 + which §), its target test file, its current status.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1924:* **Feedback hook:** Indirect — feedback-path negative fixtures live here.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:1974:* End-to-end verification: a paired SurfaceClient (W3-B WP plugin) submits a correction; the audit row is written with `actor_instance` AND `wp_user_id` populated; a second negative test submits a malformed request without `wp_user_id` and confirms endpoint rejection per artifact 12.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:2013:* **How we prove it:** new CI run on a PR that adds a SurfaceClient emission site without the helper fails; forensic-query template reproduces the originator from a real audit row; the negative fixture from artifact 12 runs in the release gate.
.docs/plans/dos-546/v1.4.2-project/02-issues.md:2130:| W5 | Negative fixture catalog | 12 |
.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:18:- **Fixture catalogue:** committed fixture directories are hyphenated `bundle-N`; current catalogue documents bundles 1-13 and warns not to create a parallel `bundles/bundle_N` tree. Source: `src-tauri/tests/fixtures/bundle-README.md:1-6`.
.docs/plans/v1.4.1-waves/W6-E-L0-packet.md:294:- Fixture shape follows the catalogue and harness conventions.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-architecture-cycle1.md:22:**Architectural concern:** v1.4.6-waves.md §327 + §414 ("Reserved block: **v260–v279** (20 slots)") reserves v260–v279 for the v1.4.6 Salience & Recommendations wave (`recommendation_claims`, `salience_factors`, `surfacing_decisions`, `triggers_log`, `deviation_baselines`, `engagement_telemetry`). v1.4.6 wave is already through cycle-4 of its plan with these slots load-bearing for W1-A/W1-B substrate. v1.4.6 §327 also explicitly states "v1.4.4 holds v240–v249" — i.e., the cross-version coordination table says v1.4.4 owns v240–v249, NOT v250–v269. This packet claims v250–v269 unilaterally, which collides with v260–v269 on the v1.4.6 side AND breaks the cross-version coordination table v1.4.6 already committed to.
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:17:| **W1-B** | [DOS-464](https://linear.app/a8c/issue/DOS-464) | PARTIAL CONVERGENCE (cycle 5; architect + `/cso` APPROVE) | PASS | UNANIMOUS APPROVE after 3 cycles | 27 unit + 12 security fixtures |
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:28:| v252 | W1-B | `workspace_source_registry` (7 source seeds) + `workspace_category_registry` (18 category seeds) |
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:78:2d78cfde fix(v1.4.5 W1-B): L2 cycle 2 codex BLOCK fold — URL-encoded lex check is now load-bearing
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:79:694bab3f fix(v1.4.5 W1-B + W1-C): L2 cycle 1 codex BLOCK folds — AC violations resolved
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:80:638a0b9a feat(v1.4.5 W1-B + W1-C): COMPLETE — open_validated + start_run + add_link + all tests
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:81:7d4673bf feat(v1.4.5 W1-B + W1-C): substantive impls for 7 of 10 functions + 29 unit tests
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:82:ebd9361f feat(v1.4.5 W1-B + W1-C): L1 pre-stage — migrations + service skeletons
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:83:270790d2 docs(v1.4.5 W1-B + W1-C): L0 cycle 5 body-sweep + partial-convergence declaration
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:84:b873ae33 docs(v1.4.5 W1-B + W1-C): L0 packet V1.3 + cycle 3 transcripts
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:85:b11f42d4 docs(v1.4.5 W1-B + W1-C): L0 packet V1.2 + cycle 2 transcripts
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:86:a00c324d docs(v1.4.5 W1-B + W1-C): L0 packet V1.1 drafts + cycle 1 reviewer transcripts
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:104:- 8 placeholder files for downstream lanes (created with `//!` doc-comments only; W1-B/W1-C/W2-A/W3-A/W3-B/W3-C fill).
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:106:### W1-B (DOS-464) — registry + path validation
.docs/plans/v1.4.5-workspace-memory/HANDOFF-2026-05-21.md:138:- `.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md` (V1.3).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:126:**One enforceable gap:** §5.1 claims the degradation works but doesn't specify a verification gate. v1.4.3 W3 invariant line 76 includes "Suite S test: switch to TwentyTwentyFive; account-overview block still renders trust band + provenance label legibly." Packet §10 needs the equivalent **negative-confirmation** test: under TwentyTwentyFive, chrome.js does NOT load (no orphaned `<script>` tags from plugin paths), confirming chrome IS theme-bound. (Condition 2, below.)
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:157:### Condition 2 — Add a stock-theme negative-confirmation gate
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle1.md:181:The three conditions sharpen the invariant boundary so future contributors don't generalize the runtime-inject path beyond chrome, and add a negative-confirmation gate that mirrors v1.4.3 W3's stock-theme fallback discipline. They are doc/AC additions, not implementation rewrites.
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-consult.output:10:1. Migration-order blocker: W1-C owns v253/v254 but still runs parallel from W1-A; if v253/v254 apply before W1-B v252, the `version > current` runner permanently skips v252. Gate W1-C merge/apply after W1-B or integrate in order.
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle3/codex-consult.output:17:1. Migration-order blocker: W1-C owns v253/v254 but still runs parallel from W1-A; if v253/v254 apply before W1-B v252, the `version > current` runner permanently skips v252. Gate W1-C merge/apply after W1-B or integrate in order.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle3.md:65:2. **Stock-theme co-existence styles.** Stock-theme contexts deliberately don't load chrome assets per AC #23 (negative gate). **Not a counterexample.**
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:9:- **Wave:** W1 stage 1a (gates W1-B and W1-C; gates all of W2/W3/W4/W5)
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:112:    /// Per-entity registry-allowedness is W1-B's `WorkspaceCategoryRegistry::validate`.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:125:    /// per-entity registry-allowedness (that is W1-B's WorkspaceCategoryRegistry boundary).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:132:    PathTraversalAttempt, SymlinkRefused, SymlinkRaced, OutsideWorkspace,  // W1-B emits these
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:255:- `registry.rs` (W1-B), `runs.rs` (W1-C), `link.rs` (W1-C), `pipeline.rs` (W2-A), `wiring.rs` (W2-A), `extract.rs` (W3-A), `signals.rs` (W3-B), `graph.rs` (W3-C).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:375:- **W1-B (DOS-464)** fills `registry.rs` + `WorkspaceCategoryRegistry`. Slot **v252**. `/cso`.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-gstack-review-cycle2.md:39:| 1 | HIGH | AC-W1.9 — `check_w1_consumer_skeleton.sh` missing | `c2e857b8` | ✅ RESOLVED | Script exists at `src-tauri/scripts/check_w1_consumer_skeleton.sh` (80 LOC), wired at `.github/workflows/lint-frontend.yml:131` as named step "Enforce v1.4.4 W1 consumer-skeleton wiring (AC-W1.9)"; local execution exits 0 with PASS message; negative-fixture harness present at `*.test`. |
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W2-design-lens-cycle2.md:22:Q3 locks Option B (aggregate signal only) at V1.1. This resolves the core trust-contract question: no per-item existence oracle leaks to the agent. The negative fixture in AC-W2.5 asserts the aggregate-only shape.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle2.md:131:## Confirmations (not findings — positive verification)
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1W2-l2-cso-cycle1.md:135:This tool is not a substitute for a professional security audit. /cso is an AI-assisted scan that catches common vulnerability patterns — it is not comprehensive, not guaranteed, and not a replacement for hiring a qualified security firm. LLMs can miss subtle vulnerabilities, misunderstand complex auth flows, and produce false negatives. For production systems handling sensitive data, payments, or PII, engage a professional penetration testing firm. Use /cso as a first pass to catch low-hanging fruit and improve your security posture between professional audits — not as your only line of defense.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l2-cso-cycle1.md:26:- F11 → audit disclosure CI lint shipped (`check_audit_disclosure_allowlist.sh` + negative-test fixture `audit_disclosure_negative/` proves it catches violations).
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:4:.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md:321:- **W1-B (DOS-464)** fills `registry.rs` + ships `WorkspaceCategoryRegistry`. Slot **v252**. `/cso`.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:16:.docs/plans/v1.4.5-waves.md:79:4. **W1-B emission ownership clarified.** W1-B's `open_validated` returns a typed `RejectionReason` (the W1-A-owned enum from `contracts.rs` per cycle 6 promotion); W2-A's pipeline owns signal emission at the boundary (consistent with cycle 3 #4). W1-B no longer says it emits `WorkspaceFileRejected` (which would not compile in W1 anyway because the SignalType isn't registered until W3-B).
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:17:.docs/plans/v1.4.5-waves.md:102:1. **Shared types promoted to W1-A `contracts.rs`** (Eng cycle 5 #1). The `Extractor` and `SignalEmitter` trait signatures reference `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason`, and `SourceType`. If those types live in later-wave lanes (W3-A, W1-B respectively), the traits don't compile in W1-A. Cycle 6 moves the type definitions into `contracts.rs` (W1-A-owned). W1-B and W3-A then USE the types from `contracts` rather than defining them locally.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:18:.docs/plans/v1.4.5-waves.md:103:2. **`RegistryRejection` renamed to unified `contracts::RejectionReason`** — W1-B's `open_validated` returns `Result<(File, FileIdentity), RejectionReason>` where the registry only emits the path-related variants and the pipeline emits the size/format variants. One enum, one ownership location.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:22:.docs/plans/v1.4.5-waves.md:126:3. **`WorkspaceCategoryRegistry` added to W1-B** — defines per-entity-type allowed categories with extensibility hook (`Other(String)` registered names) and the path resolution rules: `Accounts/{entity}/{category}/{filename}` when category present, `Accounts/{entity}/{filename}` when absent.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:30:.docs/plans/v1.4.5-waves.md:431:- **Done when:** `workspace_file_lifecycle` table exists with the seven-state lifecycle enum (including `pending_entity_assignment`), all required columns per the project DoD plus `category: Option<TEXT>` (cycle 8 Option B-prime — stores the auto-detected or caller-provided sub-category for AI navigation); Rust types compile inside `services::workspace_ingestion::lifecycle`; `services::workspace_ingestion::contracts` defines the **shared type surface** that every subsequent lane consumes (cycle 6 amendment promotes `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason`, `SourceType` into `contracts.rs` because the `Extractor` trait signature would otherwise reference types owned by W1-B and W3-A; cycle 8 amendment adds `WorkspaceCategory`):
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:42:.docs/plans/v1.4.5-waves.md:485:    With these types + traits + Null impls in place, W1-C can compile and test `link::override_link(emitter: &dyn SignalEmitter, ...)` against `NullSignalEmitter` in Wave 1, without referencing any `SignalType` variant (W3-B owns the bus-level emission mapping); W1-B's `open_validated` returns the W1-A-owned `FileIdentity` and `RejectionReason`; W3-A's `WorkspaceExtractor` impl produces W1-A-owned `WorkspaceClaimProposal` records. `mod.rs` declares all submodules with one declaration per line:
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:45:.docs/plans/v1.4.5-waves.md:515:  - On rejection, return a typed `RejectionReason { PathTraversalAttempt | SymlinkRefused | SymlinkRaced | OutsideWorkspace }` error to the caller. **W1-B does not emit signals** (cycle 4 amendment to resolve the wave-timing inversion: `WorkspaceFileRejected` SignalType isn't registered until W3-B, so W1-B emitting it directly would not compile in W1). The W2-A pipeline owns signal emission at the boundary based on the typed error variant — `pipeline.rs` calls its `SignalEmitter` slot (constructed via `wiring.rs`) when handling a `RejectionReason` it propagates.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:47:.docs/plans/v1.4.5-waves.md:524:- **Tests required:** All negative fixtures from the security gate (TOCTOU symlink race, `%2e%2e`, NFC/NFD `..`, absolute-path, outside-workspace symlink); valid path acceptance test returning `(File, FileIdentity)`; `DataSource` variant mapping round-trip across all `WorkspaceFileKind` values; CI lint script green (no path-based file reads outside `registry::open_validated`); `cargo clippy -D warnings` clean.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:48:.docs/plans/v1.4.5-waves.md:525:- **Done when:** `WorkspaceSourceRegistry::open_validated` exists and returns `(File, FileIdentity)` from a single atomic open; all negative fixtures rejected with the typed reasons listed in the security gate; CI lint script in place; ingestion call sites in subsequent lanes consume only the returned handle (verified via grep at W2-A merge); migration slot v252 used (cycle 11 renumber); all tests green; `/cso` L0 plan AND L2 diff approval recorded.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:51:.docs/plans/v1.4.5-waves.md:596:  **Tie-break rules:** higher priority wins; within the same priority, first matched rule wins. **Validation:** auto-detected results MUST pass `WorkspaceCategoryRegistry::validate(category, entity_type)` — if the rule produces a category not allowed for the file's entity type, fall back to `None` (entity root) rather than silently using a disallowed category. **Content sniff bound:** `content_head` is the first 4KB of file content (sufficient for frontmatter parsing); never reads the full file. Pipeline records the resolved category in the lifecycle row and resolves the destination path via `WorkspaceCategoryRegistry` from W1-B.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-challenge-cycle3.output:58:.docs/plans/v1.4.5-waves.md:883:      "category": Option<String>,               // OPTIONAL canonical slug from WorkspaceCategory::as_slug() — one of "presentations" / "transcripts" / "meetings" / "notes" / "contracts" / "attachments" or a registered Other(name) lowercase slug. Cycle 10 contract reconciliation — two distinct branches, no overlap: (a) ABSENT (null/missing): server runs pipeline::auto_detect_category() per the cycle 9 frozen detection rule table; (b) PRESENT: server validates against W1-B WorkspaceCategoryRegistry BEFORE constructing IngestRequest — well-formed AND registry-valid → category_hint = Some(...); malformed OR registry-disallowed → return PlacementError::CategoryNotAllowed and never invoke the pipeline. There is no "invalid hint falls through to auto-detection" path; that behavior would silently bury caller errors.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:1:# L0 Packet — v1.4.5 W1-B — DOS-464 Workspace Source Registry + Path Validation
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:19:  1. **Fixture count consistency** (challenge #1 + consult #2): §7 header said "17 fixtures" but enumerated 19, and §8/§9 said "all 15". V1.3 §7 header says "19 fixtures (15 negative + 4 positive)"; §8/§9 say "all 19 fixtures".
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:29:  3. **CI lint gate scoped to ingestion-call-site files** (challenge #3 + consult #5): cycle-1 V1.1 made the gate repo-wide which would trip existing non-ingestion callsites. V1.2 scopes the gate to a positive allowlist:
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:44:  2. **`EntityType` uses canonical `crate::entity::EntityType` from `src-tauri/src/entity.rs:13`** (architect F1 + challenge #3 + consult #2): pre-grepped; the canonical type has variants `Account | Project | Person | Other` with `#[serde(rename_all = "snake_case")]`. W1-B does NOT define a parallel `EntityType`. `validate(category: &WorkspaceCategory, entity_type: crate::entity::EntityType) -> Result<(), CategoryNotAllowed>`.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:52:  10. **CI lint gate broadened** (security SEC-005): `check_workspace_path_validation.sh` updated to a positive-allowlist approach. Forbidden patterns: `std::fs::File::open`, `std::fs::OpenOptions::*::open`, `std::fs::read*`, `std::fs::metadata`, `std::fs::symlink_metadata`, `std::fs::read_link`, `std::fs::read_dir`, `tokio::fs::*`, `tokio::fs::OpenOptions`, `memmap2::Mmap::map`. Allowed entry points enumerated: `registry::open_validated` ONLY. Allowlist exception marker changed from stale `dos-146-allowed` to neutral `// workspace-path-allowed: <rationale>` (challenge #5).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:57:  15. **DataSource variant mapping round-trip test added** (consult #5): per wave-plan §W1-B done-when "DataSource variant mapping round-trip across all WorkspaceFileKind values", the test asserts that for each of the 7 `WorkspaceFileKind` variants, the `data_source_json` seed value parses back to the canonical `DataSource::WorkspaceFile { kind }` via serde.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:59:  17. **Concurrency positive test added** (security gap): N=8 parallel `open_validated` calls on the same valid path return identical `FileIdentity` triples.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:61:  19. **Windows platform decision** (architect F2 + security SEC-001 residual): V1.1 explicitly platform-gates the security boundary as **Unix-only at W1-A merge**, with Windows path validation deferred to a follow-up `feat/dos-464b-windows-path-validation` ticket. The Windows code-path returns `RejectionReason::OutsideWorkspace` with a documented "Windows path validation not implemented; W1-B Unix-only" message until that follow-up ships. Justification: DailyOS macOS-first per CLAUDE.md; Windows support is post-v1.4.x. Filed for Codebase Maintenance follow-up.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:66:## 3. Goal (verbatim from wave plan §Agent W1-B)
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:86:- `src-tauri/tests/workspace_registry_open_validated.rs` — security negative-fixture suite (see §8).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:100:| Contract | Location | W1-B relationship |
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:103:| `contracts::RejectionReason` | `…/contracts.rs:113` (W1-A; 6 variants — all consumed) | **consumes**; W1-B emits `PathTraversalAttempt`, `SymlinkRefused`, `SymlinkRaced`, `OutsideWorkspace` |
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:108:| Highest registered migration on dev | `migrations.rs:927` (v240) + W1-A v250 + v251 | W1-B claims **v252** per cycle 11 |
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:112:Re-verified zero collisions on all V1.1-introduced names. `WorkspaceSourceRegistry`, `WorkspaceCategoryRegistry`, `CategoryNotAllowed`, `RegisterError` are net-new W1-B primitives. Note: `PlacementError::CategoryNotAllowed` exists at wave-plan W4-C (DOS-474) — these are deliberately distinct types in different layers (registry-level vs placement-API-level), tracked as future maintenance if convergence is wanted post-W4-C.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:123:-- v1.4.5 W1-B — workspace source registry: source-type allowlist + per-entity-type category registry.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:215:### Negative fixture suite (V1.3 — **19 fixtures: 15 negative + 4 positive**, all return typed `RejectionReason` with zero bytes read)
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:234:18. **Concurrency positive**: N=8 parallel `open_validated` calls on same valid path return identical `FileIdentity`.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:239:`src-tauri/scripts/check_workspace_path_validation.sh` enforces a positive allowlist within an explicit file-path scope (V1.1's repo-wide scope would trip pre-existing non-ingestion callsites):
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:259:- `tests/workspace_registry_open_validated.rs` — all 19 fixtures in §7 (15 negative + 4 positive per V1.3 fold #1). Each negative asserts zero bytes read.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:287:- All 19 §7 security fixtures pass with typed `RejectionReason` variants (Unix; Windows skipped). 15 negative + 4 positive per V1.3 fold #1.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:299:1. **Claim model.** Registry rows are metadata; W1-B commits no claims.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:301:3. **Signals + invalidation.** W1-B emits no signals; `RejectionReason` typed errors route to W2-A pipeline boundary (per W1-A cycle 4 wave-timing fix).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-B-DOS-464.md:308:- **W4-C (DOS-474)** consumes `validate` for caller-provided category validation per cycle 10 contract reconciliation; its `PlacementError::CategoryNotAllowed` carries the same name as W1-B's `CategoryNotAllowed` but is a different envelope (deliberately distinct types in different layers).
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle2.md:23:- **C3 - APPROVED in V1.1.** AC #23 adds stock-theme negative gate.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-D-DOS-469.md:32:- **Required substrate before L1:** W1-A lifecycle/ownership model, W1-B `WorkspaceSourceRegistry::open_validated`, W1-C ingestion-run and link tracking, W2-A `IngestPipeline::run` + `LifecycleRepo`, the W1-extension PR adding `IngestionMode::Realtime` + canonical `UserRelink`, and W2-A V1.4 `LifecycleRepo::get` + `LifecycleRepo::set_entity`
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-A-DOS-466.md:74:**Tie-break rules:** higher priority wins; within the same priority, first matched rule wins. **Validation:** auto-detected results MUST pass `WorkspaceCategoryRegistry::validate(category, entity_type)` — if the rule produces a category not allowed for the file's entity type, fall back to `None` (entity root) rather than silently using a disallowed category. **Content sniff bound:** `content_head` is the first 4KB of file content (sufficient for frontmatter parsing); never reads the full file. Pipeline records the resolved category in the lifecycle row and resolves the destination path via `WorkspaceCategoryRegistry` from W1-B.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-A-DOS-466.md:195:- Caller-provided `IngestRequest.file_id` MUST match `file_id_from_identity(&request.identity, workspace_root)` per §0 V1.2 §2.1 and §2.4. Mismatch returns typed `IngestError::FileIdMismatch { expected, found }`; L1 must not downgrade this to `DbError`, `Io`, `RejectionReason`, or an untyped string. §10 requires the negative test.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-codex-challenge-cycle1.md:39:**Finding:** §5.2 (DOS-460) and §5.10 (DOS-507) IL checks answer each Q with ticket-specific substance (ClaimType::OpenLoop, WrongSubject routing, DOS-278 candidate-set invalidation, DOS-8 routing). §5.8 (DOS-340) IL check is structurally thin: Q1 "no claims written", Q3 "no new signals", Q5 "N/A". This is defensible for a pure render-side boundary filter, but functionally close to a fail-shape under AC-W1.3 ("substrate that fails any question is incomplete"). The packet should explicitly justify that "N/A for feedback loop" is acceptable for boundary-only substrate, OR re-frame Q5 to point at "violations of this boundary feed source reliability negatively via DOS-8 if a surface accidentally exposes audit-only fields" — which IS a real feedback path.
.docs/plans/v1.4.4-wp-surface-migration/reviews/wave-W1-l3-codex-challenge-cycle1.md:9:Codex was prompted to stress-test 5 specific wave-level integration points. All 5 returned with real defects (no false-positives, all cite file:line evidence).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md:56:> - **Tests required (cycle 3 amendment — fixture-injected claim rendering):** Block renders entity claim list from a fixture-seeded set of `intelligence_claims` rows for the fixture entity (the W2 demo uses hand-seeded claims because W2-A's pipeline produces zero claims; real ingestion-to-claim-render validation is W5-B's job); trust-band colors match `likely_current`/`use_with_caution`/`needs_verification`; intake ability invocation triggers `IngestPipeline::run` in unit test (verifies the wiring fires; the pipeline records the run but produces zero proposals at W2 time, which is the expected W2-A behavior); negative fixture: intake with path traversal path returns error, block shows error state (not a crash).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md:57:> - **Done when:** Block activates, renders fixture-seeded claim proposals for a fixture entity with trust bands inline; intake ability invocation triggers `IngestPipeline::run` and writes an ingestion run record (zero proposal expected at W2 time); intake ability declared with correct scopes and `mcp_exposure: None`; path traversal negative fixture green; `/qa-only` L4 surface QA passes in WP Studio against the fixture-seeded entity. Real ingestion-to-claim-render flow validation deferred to W5-B's E2E validation against real workspace data.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md:522:> **Tests required (cycle 3 amendment — fixture-injected claim rendering):** Block renders entity claim list from a fixture-seeded set of `intelligence_claims` rows for the fixture entity (the W2 demo uses hand-seeded claims because W2-A's pipeline produces zero claims; real ingestion-to-claim-render validation is W5-B's job); trust-band colors match `likely_current`/`use_with_caution`/`needs_verification`; intake ability invocation triggers `IngestPipeline::run` in unit test (verifies the wiring fires; the pipeline records the run but produces zero proposals at W2 time, which is the expected W2-A behavior); negative fixture: intake with path traversal path returns error, block shows error state (not a crash).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W2-C-DOS-468.md:550:> Block activates, renders fixture-seeded claim proposals for a fixture entity with trust bands inline; intake ability invocation triggers `IngestPipeline::run` and writes an ingestion run record (zero proposal expected at W2 time); intake ability declared with correct scopes and `mcp_exposure: None`; path traversal negative fixture green; `/qa-only` L4 surface QA passes in WP Studio against the fixture-seeded entity. Real ingestion-to-claim-render flow validation deferred to W5-B's E2E validation against real workspace data.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:38:**Concern:** `RECEIPT_ALLOWED_FIELDS` / `AUDIT_ONLY_DENYLIST` (declared in §5.8 DOS-340 but co-binding to §5.4 boundary work) is a **denylist + allowlist hybrid**. The packet's `filter_for_receipt` applies the receipt allowlist; the `AUDIT_ONLY_DENYLIST` is informational rather than primary. This is the structural rule from the L0 Addendum, but the packet's open architectural question §13.2 still flags it as undecided. ADR-0108 §2 (Agents never see…) is allowlist-shaped (positive list of what crosses). Denylist + allowlist hybrid leaks the moment a new field name lands.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:135:**Concern:** Privacy matrix `OperationalAuditStorage` row says "no surface — storage only; non-disclosure assertion class with positive/negative fixtures". Packet does not specify where the non-disclosure assertion is *enforced*. ADR-0108 §1 separates the security audit (`.jsonl`, ADR-0094) from operational audit (`maintenance_audit` SQLite). The risk is: a future surface (e.g., a debug viewer for power users, an admin tool) accidentally queries `maintenance_audit` and exposes raw model i/o through a code path that bypasses `apply_privacy_for_audience`. Memory `project_engineering_ladder` "trust-boundary fence" CI gate is the structural pattern.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-cso-cycle1.md:139:**Active-verification hook:** Extend `check_audit_disclosure_allowlist.sh` to also forbid any direct read of `maintenance_audit` table rows from `services::claim_receipt::*` (the privacy module is allowed to *write* assertions but never to *surface* raw rows). Pair with negative-fixture test: a deliberately-attempted disclosure path that the lint catches.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:63:### 5. AC #23 — stock-theme negative gate. **CONFIRMED; verification approach below.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-wp-skill-cycle2.md:67:This is correct WP behavior — `functions.php` of a non-active theme is never included. The negative gate is structural, not conditional.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle4/codex-challenge.output:5:215:### Negative fixture suite (V1.3 — **19 fixtures: 15 negative + 4 positive**, all return typed `RejectionReason` with zero bytes read)
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle4/codex-challenge.output:6:259:- `tests/workspace_registry_open_validated.rs` — all 15 fixtures in §7 (13 negative + 2 positive). Each negative asserts zero bytes read.
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-challenge.output:47:5. Non-blocking checks passed: no ClaimProposal/ClaimFeedbackInput/SourceAttribution substrate reinvention found; v253/v254 are correctly above W1-B v252; FK to `workspace_file_lifecycle(file_id)` is valid because W1-A makes `file_id` UNIQUE; `override_link` compiles against W1-A `SignalEmitter::emit_link_changed`.
.docs/plans/v1.4.5-workspace-memory/reviews/w1c-cycle1/codex-challenge.output:60:5. Non-blocking checks passed: no ClaimProposal/ClaimFeedbackInput/SourceAttribution substrate reinvention found; v253/v254 are correctly above W1-B v252; FK to `workspace_file_lifecycle(file_id)` is valid because W1-A makes `file_id` UNIQUE; `override_link` compiles against W1-A `SignalEmitter::emit_link_changed`.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-challenge.output:45:Substrate check: `CategoryNotAllowed`/`RegisterError` have no current code collision; `StartRunSeed` is not in this W1-B packet, only W1-C.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle2/codex-challenge.output:60:Substrate check: `CategoryNotAllowed`/`RegisterError` have no current code collision; `StartRunSeed` is not in this W1-B packet, only W1-C.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle2.output:107:5. Boundary check passes: `WorkspaceCategory::from_slug` explicitly limits W1-A to lexical slug shape and says per-entity allowedness is W1-B’s `WorkspaceCategoryRegistry::validate`. No W1-B dependency is reintroduced there.
.docs/plans/v1.4.5-workspace-memory/reviews/packet-W1-A-codex-consult-cycle2.output:120:5. Boundary check passes: `WorkspaceCategory::from_slug` explicitly limits W1-A to lexical slug shape and says per-entity allowedness is W1-B’s `WorkspaceCategoryRegistry::validate`. No W1-B dependency is reintroduced there.
.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md:58:    /// the pre-validated mtime cached by W1-B's registry.
.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md:392:W1-B ships `WorkspaceCategoryRegistry::validate(conn: &Connection, category: &WorkspaceCategory, entity_type: EntityType)`. The function requires a DB connection AND a typed `EntityType` — not stringly. Two consequences:
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-codex-consult-cycle1.md:20:- **C3 - Add stock-theme negative gate.** Under TwentyTwentyFive or any non-DailyOS theme, assert no `<script>` or stylesheet loads from `wp/dailyos/theme/assets/chrome/`. This pairs with `.docs/plans/v1.4.3-waves.md:76` stock-theme trust/provenance fallback.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:9:- **Wave:** W1 stage 1b (gates on W1-A merge; runs parallel with W1-B)
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:52:  5. **Migration ordering safeguard** (consult #1): if W1-C ships v253/v254 before W1-B v252 lands, the `version > current` runner permanently skips v252. V1.3 mitigation: this wave uses the **wave-PR merge model** (PR #345 = `wave/v1.4.5-w1-stage1a` → `dev`). W1-B + W1-C land on the wave branch via local merges in dependency order (W1-A → W1-B → W1-C → wave PR). The wave PR is a single atomic merge to dev. W1-C must NOT open a separate PR vs dev. Documented in §10 handoff + §1 branch.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:60:  1. **`override_link` + `reject_link` take `entity_type`** (consult #1 + architect #1): both APIs now signature `(emitter: &dyn SignalEmitter, file_id: &str, entity_type: crate::entity::EntityType, entity_id: &str, actor: &str, …) -> Result<(), LinkError>`. The unique key is the `(file_id, entity_type, entity_id)` triple, so every link-mutation API must carry all three. Reuses `crate::entity::EntityType` (canonical per W1-B V1.1 fold #2; no reinvention).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:77:- **V1.0 (2026-05-20):** Initial packet. Drafted in parallel with W1-B before cycle 1 review.
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:132:| Highest registered migration on dev | v240 + W1-A v250+v251 + W1-B v252 | W1-C claims **v253 + v254** per cycle 11 |
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:230:- **Migrations:** v253 + v254 apply cleanly after v252 (W1-B); FK to `workspace_file_lifecycle.file_id` enforced (CASCADE delete).
.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-C-DOS-465.md:284:**Wave-PR merge model (V1.3 fold #5):** W1-B + W1-C must NOT open separate PRs vs `dev`. They land on `wave/v1.4.5-w1-stage1a` via local merges in strict dependency order — W1-A → W1-B (v252) → W1-C (v253/v254). The single PR #345 (`wave/v1.4.5-w1-stage1a` → `dev`) is the atomic wave merge. This safeguard prevents the `version > current` migration runner permanently skipping v252 if v253/v254 land first via a parallel PR.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle5/codex-challenge.output:3:   287	- All 19 §7 security fixtures pass with typed `RejectionReason` variants (Unix; Windows skipped). 15 negative + 4 positive per V1.3 fold #1.
.docs/plans/v1.4.5-workspace-memory/reviews/w1b-cycle3/codex-consult.output:7:Substrate check: `CategoryNotAllowed`/`RegisterError` have no current code collision; `StartRunSeed` is not in this W1-B packet, only W1-C.
.docs/plans/v1.4.3-waves.md:107:| **W6** — Audit + clean-machine | DOS-576 SurfaceClient audit attribution forensic validation; DOS-577 clean-machine Studio validation + dev-mode runtime launcher; DOS-575 negative fixture catalog. | 1 packet, 3 commit groups | 1 | 2026-05-28 |
.docs/plans/v1.4.3-waves.md:232:- Restart-fixture harness — DOS-575 (W6 negative-fixture catalog) absorbs the restart-scenario rows directly, without a separate W5 harness driver.
.docs/plans/v1.4.3-waves.md:238:- **Tickets:** DOS-576 SurfaceClient audit attribution forensic validation; DOS-577 clean-machine Studio validation + dev-mode runtime launcher; DOS-575 negative fixture catalog.
.docs/plans/v1.4.3-waves.md:240:- **DOS-575 — Negative fixture catalog:** every error case in the L0 packets + every CI invariant + every L2-deferred path-α maintenance ticket gets a reproducible fixture in `src-tauri/src/services/tests/negative_fixtures/` so future regressions trip a named test, not a vague failure mode.
.docs/plans/v1.4.3-waves.md:243:- **DOS-575 release-gate catalog:** `.docs/plans/v1.4.3-wp-foundation/W6-negative-fixture-catalog.md` is the closed W6 negative-fixture catalog. `scripts/release-gate/run-w6-fixtures.sh` is mandatory release-gate evidence; the Rust release gate reads `w6-fixtures.json` and fails on any missing, failed, stale, unknown, or skipped fixture.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:24:### Condition 2 — Stock-theme negative-confirmation gate. **RESOLVED.**
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:73:- AC #29 Site Editor DB override safety + §13 reset path appendix close a real footgun cycle-1 didn't surface — that's net-positive for the packet.
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:96:- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:356` — AC #23 stock-theme negative gate
.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md:109:V1.1 folds the 3 cycle-1 conditions cleanly, adds 3 net-positive ACs (#26, #28, #29) that strengthen the substrate boundary beyond what cycle-1 asked for, codifies 3 new invariants in §10 (runtime-injection scope, class-contract-not-DOM-order, W4-coupling matrix), and surfaces no new architectural debt. Patch 9 design is sound; the overlay-vs-canonical decision is the established pattern for the lift and the failure mode (patch anchor lost) is loud. K-in citations verified accurate.

exec
/bin/zsh -lc "nl -ba src-tauri/resources/mcp_v2/tool_descriptions.yaml | sed -n '1,90p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md | sed -n '1,240p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
     1	# DailyOS MCP v2 tool taxonomy — version-controlled product copy.
     2	#
     3	# Per ADR-0128 §3 + DOS-478 L0 packet §1: this catalog is the source of
     4	# truth for what host models see when selecting tools. Authored per the
     5	# product-vocabulary discipline in ADR-0083 + AC-4/AC-6 of the L0 packet:
     6	# natural-language fields (summary, when_to_call, when_NOT_to_call) carry
     7	# no system identifiers or `dailyos.` substrings; tool identifiers appear
     8	# only in structured `selection_fixtures.*.expected_tool`.
     9	#
    10	# Per AC-2: `side` is one of `Read | SubmitCorrection | Write` verbatim.
    11	# Per AC-3: `scopesRequired` matches the §5 table exactly.
    12	# Per AC-5: `selectionFixtures` carries ≥ 2 positive + ≥ 2 broad-corpus
    13	# negative + ≥ 1 adjacent-tool negative (≥ 2 where adjacent exists).
    14	# Per AC-8: nested deny_unknown_fields applies — no typos.
    15	
    16	- name: dailyos.read.account_status
    17	  side: Read
    18	  summary: Returns the current state of an account the user works with.
    19	  when_to_call: |
    20	    When the user asks about a specific account they have a working
    21	    relationship with — status, recent signals, open commitments,
    22	    champion engagement, contract state. The assistant persists context
    23	    across conversations and updates as work evolves; rely on this for
    24	    cross-conversation continuity.
    25	  when_NOT_to_call: |
    26	    Do NOT call for accounts the user has no working relationship with —
    27	    this surface is for the user's personal working understanding of their
    28	    professional world, not broad enterprise or web corpus search. Use
    29	    enterprise search, web search, or Glean for broad corpus lookups.
    30	  scopesRequired:
    31	    - dailyos.read.account_status
    32	  parameters:
    33	    - name: subject
    34	      schema: { type: string }
    35	      required: true
    36	      description: Account name or handle
    37	  returns:
    38	    schema: { type: object, additionalProperties: false }
    39	    description: Account status payload with claim attribution and freshness per ADR-0105
    40	  examples:
    41	    - prompt: What's going on with Acme?
    42	      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
    43	      expectedResponseShape: { account_id: opaque, status: enum, as_of: iso8601 }
    44	  selectionFixtures:
    45	    positive:
    46	      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
    47	      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
    48	    negativeBroadCorpus:
    49	      - { prompt: Search the web for SaaS pricing benchmarks, expectedToolClass: external }
    50	      - { prompt: What's the company's vacation policy?, expectedToolClass: external }
    51	    negativeAdjacentTool:
    52	      - { prompt: What did I write about the Q1 readout?, expectedTool: dailyos.search.workspace_memory }
    53	
    54	- name: dailyos.read.daily_briefing
    55	  side: Read
    56	  summary: Returns the user's current-day briefing.
    57	  when_to_call: |
    58	    When the user asks about today's priorities, meetings, accounts that
    59	    need attention, or what to focus on. The assistant persists context
    60	    across conversations so today's briefing reflects yesterday's open
    61	    loops and updates as work evolves through the day.
    62	  when_NOT_to_call: |
    63	    Do NOT call for general news, weather, or org-wide announcements —
    64	    use web search or enterprise search for broad corpus lookups. Do NOT
    65	    call for a specific meeting's prep notes; that is a different surface.
    66	  scopesRequired:
    67	    - dailyos.read.daily_briefing
    68	  parameters: []
    69	  returns:
    70	    schema: { type: object, additionalProperties: false }
    71	    description: Briefing payload with prioritized accounts, meetings, and open commitments
    72	  examples:
    73	    - prompt: What's on my plate today?
    74	      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
    75	      expectedResponseShape: { date: iso8601, priorities: array, meetings: array }
    76	  selectionFixtures:
    77	    positive:
    78	      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
    79	      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
    80	    negativeBroadCorpus:
    81	      - { prompt: What's the weather today?, expectedToolClass: external }
    82	      - { prompt: Show me today's news headlines, expectedToolClass: external }
    83	    negativeAdjacentTool:
    84	      - { prompt: What should I prep for my 2pm with Acme?, expectedTool: dailyos.read.meeting_briefing }
    85	
    86	- name: dailyos.read.meeting_briefing
    87	  side: Read
    88	  summary: Returns the prep briefing for a specific meeting.
    89	  when_to_call: |
    90	    When the user asks for prep notes on a specific upcoming or recent

 succeeded in 0ms:
     1	# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
     2	
     3	**Wave:** v1.4.7 W2-A
     4	**Lane spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175)
     5	**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W2-A — DOS-175"
     6	**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
     7	**Authoring discipline:** narrow-scoped per K-in lessons; scope confined to wiring already-authored producers into the W1-A handler seam.
     8	
     9	## Cycle-1 scope
    10	
    11	W2-A ships two read-side MCP tool handlers wrapping abilities that exist today in `abilities-runtime`:
    12	
    13	1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
    14	2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
    15	
    16	These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
    17	
    18	**Threat topology:** `local-to-local single-user`. Claude Desktop on the same machine, same user, loopback stdio MCP transport. Per `engineering-ladder.md` §"Threat-topology framing" + memory `feedback_local_to_local_security_overreach_primary_concern`: reviewers do NOT enforce multi-actor gates, principal differentiation, cross-actor poisoning, or scope-gated redaction of data the user already has filesystem access to. The only invariants that survive the topology test are: compile correctness, crate-boundary rules, sensitivity rendering already done in the producer (ADR-0108), and indirect-prompt-injection containment from third-party document content (ADR-0093). Reviewers who flag multi-actor gates on this surface waste a cycle; please scope your threat model to "Claude Desktop run by James on James's machine talking to DailyOS running as James."
    19	
    20	## 1. What this unblocks
    21	
    22	End-to-end demo path: user pairs Claude Desktop against `dailyos-mcp-v2`, asks "give me an account overview for Acme" or "what's on my plate today", Claude routes to the appropriate DailyOS tool, the W2-A handler invokes the underlying ability against the same `abilities-runtime` registry the Tauri UI uses, and the substrate response is returned to Claude Desktop. This is the v1.4.7 W2-A bar — first observable MCP product surface against real DailyOS substrate.
    23	
    24	The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
    25	
    26	## 2. Substrate path (K-in confirmation)
    27	
    28	`docs/solutions/` + `.docs/decisions/` grep results — substrate already present:
    29	
    30	- **`AbilityRegistry`** — single global static. `abilities-runtime/src/abilities/registry.rs:838` exposes `pub fn global_checked() -> Result<&'static Self, ...>` backed by `OnceLock`. There is no parallel legacy registry.
    31	- **`crate::abilities` shim** — `src-tauri/src/abilities/mod.rs:3` is `pub use abilities_runtime::abilities::*;`. Tauri commands and MCP bridges both consume the same crate-level re-export.
    32	- **`invoke_registry_json`** — the shared async dispatch core. Called by Tauri (`bridges/tauri.rs:217`), legacy MCP (`bridges/mcp.rs:411-422`), and now W2-A.
    33	- **`McpAbilityBridge::invoke_ability`** (`src-tauri/src/bridges/mcp.rs:385-430`) — the canonical pattern for "actor + ability_name + input_json → AbilityResponseJson" over the same registry. W2-A handlers reuse this pattern verbatim (no second invocation engine).
    34	- **ADR-0102 §C.bis** — abilities-as-runtime contract; covers `mcp_exposure`, `allowed_actors`, attribute changes via re-approval.
    35	- **ADR-0108** — claim-sensitivity rendering; producer abilities render sensitivity before returning. Handler is a thin JSON wrapper.
    36	- **ADR-0125** — claim-sensitivity rendering depth; same as above.
    37	- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
    38	- **W1-A handler seam** — `gateway.rs:155` (`handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>`), `gateway.rs:177` (`register`), `gateway.rs:271` (`dispatch`). Scaffolding directory `services/mcp_v2/handlers/` already exists with 1-line placeholder files including `tool_briefing.rs`.
    39	- **`McpToolHandler` trait** — `services/mcp_v2/contracts.rs:341`: sync `invoke(&self, &McpActor, params: Value) -> Result<Value, ToolError>`.
    40	
    41	**No substrate reinvention.** W2-A is purely the wiring layer between W1-A's `McpToolHandler` registration seam and the existing `invoke_registry_json` dispatch.
    42	
    43	## 3. Architecture
    44	
    45	### 3.1 Handler shape
    46	
    47	One handler struct per tool, in its own file under `services/mcp_v2/handlers/`. Each handler holds:
    48	
    49	```rust
    50	pub struct AccountStatusHandler {
    51	    description: ToolDescription,
    52	    registry: &'static AbilityRegistry,
    53	    runtime: tokio::runtime::Handle,
    54	}
    55	```
    56	
    57	The runtime handle is captured at construction time (during gateway registration, when we are inside the tokio runtime that rmcp drives). The sync `invoke()` method calls `self.runtime.block_on(async { ... })` to dispatch the underlying async ability. This resolves the sync/async bridge question from the handoff: **Option 2 from the handoff (carry-handle-in-struct)**, not Option 1 (`Handle::current()` lookup at call time, which risks panic if called outside a runtime context). The legacy MCP bridge implicitly assumes a runtime context because it's only called from rmcp; the W1-A `dispatch` trait is sync, so we must construct an explicit bridge.
    58	
    59	Rejected alternatives:
    60	- **Option 1** (`Handle::current()` lookup at invoke time): panics in non-runtime tests; capturing at construction is strictly safer.
    61	- **Option 3** (change trait to async): forces W1-A test stubs + transport layer changes for a problem solvable in the handler.
    62	- **Option 4** (return `Pin<Box<dyn Future>>` from trait): verbose, requires W1-A trait change.
    63	
    64	### 3.2 Ability dispatch within the handler
    65	
    66	The handler's `invoke` body mirrors `bridges/mcp.rs:391-422` (the legacy MCP bridge's `invoke_ability`):
    67	
    68	```rust
    69	fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
    70	    self.runtime.block_on(async {
    71	        let clock = SystemClock;
    72	        let rng = SystemRng;
    73	        let external = ExternalClients::default();
    74	        let services = ServiceContext::new_live(&clock, &rng, &external)
    75	            .with_actor(MCP_V2_ACTOR_LABEL);
    76	        let invocation = InvocationContext {
    77	            actor: BridgeActor::from_mcp_actor(actor),
    78	            mode: ExecutionMode::Live,
    79	            surface: BridgeSurface::McpTool,
    80	            claim_dismissal_surface: ClaimDismissalSurface::McpTool,
    81	            dry_run: false,
    82	            confirmation: None,
    83	            confirmation_store: None,
    84	        };
    85	        let response = invoke_registry_json(
    86	            self.registry,
    87	            &services,
    88	            None,  // provider
    89	            None,  // tracer (W2-A: no per-invocation tracing yet; W3 adds it)
    90	            invocation,
    91	            "account_overview",  // or "get_daily_briefing"
    92	            params,
    93	        )
    94	        .await
    95	        .map_err(map_invoke_error_to_tool_error)?;
    96	        Ok(response.data)
    97	    })
    98	}
    99	```
   100	
   101	The actor label can reuse the existing `bridges/mcp.rs::MCP_ACTOR_LABEL = "mcp"` — local-to-local single-user, no audit-differentiation requirement that justifies a new constant. The `BridgeActor::from_mcp_actor` projection converts `services::mcp_v2::McpActor::Client { client_id, conversation_handle }` to `abilities_runtime::Actor::McpClient { client_id, conversation_handle }` — same shape, trivial conversion. Most likely lands as a small adapter in `services/mcp_v2/actor_policy.rs::project_actor` or sibling.
   102	
   103	### 3.3 Registration helper
   104	
   105	New module `services/mcp_v2/handlers/registration.rs`:
   106	
   107	```rust
   108	pub fn register_v147_handlers(
   109	    gateway: &mut Gateway,
   110	    catalog: &YamlTaxonomyCatalog,
   111	    registry: &'static AbilityRegistry,
   112	    runtime: tokio::runtime::Handle,
   113	) -> Result<(), RegistrationError> {
   114	    gateway.register(Arc::new(AccountStatusHandler::new(
   115	        catalog.description_for(&account_status_name())?.clone(),
   116	        registry,
   117	        runtime.clone(),
   118	    )));
   119	    gateway.register(Arc::new(DailyBriefingHandler::new(
   120	        catalog.description_for(&daily_briefing_name())?.clone(),
   121	        registry,
   122	        runtime.clone(),
   123	    )));
   124	    Ok(())
   125	}
   126	```
   127	
   128	Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).
   129	
   130	### 3.4 Ability attribute amendments
   131	
   132	**`account_overview` (`abilities-runtime/src/abilities/account_overview.rs:34`):**
   133	
   134	Current:
   135	```rust
   136	#[ability(
   137	    name = "account_overview",
   138	    category = Read,
   139	    allowed_actors = [User, SurfaceClient],
   140	    mcp_exposure = Invocable,
   141	    ...
   142	)]
   143	```
   144	
   145	Amendment:
   146	```rust
   147	    allowed_actors = [User, SurfaceClient, McpClient],
   148	```
   149	
   150	**Justification.** The ability already declares `mcp_exposure = Invocable`, so adding `McpClient` to the allowlist is a consistency fix, not a new exposure decision. Local-to-local single-user topology: no CSO escalation cycle warranted. Just amend.
   151	
   152	**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
   153	
   154	Current:
   155	```rust
   156	#[ability(
   157	    name = "get_daily_briefing",
   158	    category = Read,
   159	    allowed_actors = [User, SurfaceClient],
   160	    mcp_exposure = None,
   161	    ...
   162	)]
   163	```
   164	
   165	Amendment:
   166	```rust
   167	    allowed_actors = [User, SurfaceClient, McpClient],
   168	    mcp_exposure = Invocable,
   169	```
   170	
   171	**Justification.** Explicitly authorized by DOS-624 §1 and §3 (cycle-2 normative). The DOS-624 amendment is unanimous APPROVE on this branch.
   172	
   173	### 3.5 Output translation
   174	
   175	Per ADR-0125 + ADR-0108, the producer ability renders claim sensitivity before returning. The handler's translation is `serde_json::to_value(response.data)` — no re-rendering, no redaction layer in the handler. The `AbilityResponseJson` already wraps the rendered output. This matches the legacy `McpAbilityBridge::invoke_ability` pattern (line 411-430): the bridge returns `AbilityResponseJson` directly; the consumer (rmcp layer) serializes for wire. **Open question Q2** (§7): the V1 bridge wraps the response in a session-scoped provenance cache (`insert_provenance` at `bridges/mcp.rs:431-436`). W2-A does not include a per-session provenance cache yet. Reviewers: is this a W2-A AC or W2-B follow-up? Cycle-1 stance: defer to W2-B (the user-facing demo doesn't depend on detail-pane provenance lookup).
   176	
   177	## 4. Out-of-scope (explicit deferrals)
   178	
   179	These items are NOT in W2-A cycle 1 and have specific deferral reasoning:
   180	
   181	1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
   182	2. **`dailyos.read.portfolio_attention`** — producer ability does not exist; `get_entity_intelligence` is per-entity not per-portfolio. Defer to W3.
   183	3. **`dailyos.search.workspace_memory`** — no producer ability; depends on v1.4.5 workspace substrate landing on this branch. Defer.
   184	4. **`dailyos.read.workspace_source_provenance`** — same as above.
   185	5. **`dailyos.write.place_document`** + **`dailyos.submit.{note,action,action_status}`** — Write-side abilities; require additional CSO amendment surface (W3-C parallel-amendment open question per DOS-624 §8 Q3). Defer.
   186	
   187	## 5. Acceptance criteria
   188	
   189	**AC-1.** `account_overview` ability attribute amended: `allowed_actors` includes `McpClient`. Compile + clippy green.
   190	
   191	**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
   192	
   193	**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
   194	
   195	**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
   196	
   197	**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
   198	
   199	**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.
   200	
   201	**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
   202	
   203	**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
   204	
   205	**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
   206	
   207	**AC-10.** No regression: `cargo check --features mcp --lib`, `cargo clippy --features mcp --lib --bin dailyos-mcp-v2 -- -D warnings`, `cargo test --features mcp` all green. No L1 self-validation gaps before opening the PR.
   208	
   209	## 6. Test plan
   210	
   211	- **Unit** (per handler): mock registry dispatch (provide test-only fake via cfg(test) hook or thread the registry as a generic param) and assert wire shape conversion. ~30 LOC each.
   212	- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
   213	- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
   214	
   215	**Out of scope for cycle-1 tests:** concurrent burst latency (W2-B), provenance cache (W2-B), meeting_briefing existence oracle (deferred per §4).
   216	
   217	## 7. Open questions for L0 reviewers
   218	
   219	**Q1 (architecture).** Should the `BridgeActor::from_mcp_actor` projection live in `services/mcp_v2/actor_policy.rs` (next to `project_actor`) or in a new `bridges/mcp_v2.rs` mirroring the legacy bridge layout? The actor projection helper is the only cross-crate signature decision in W2-A; everything else follows W1-A precedent.
   220	
   221	**Q2 (provenance).** Defer per-session provenance cache to W2-B? V1 bridge maintains a session-scoped `(InvocationId → RenderedProvenance)` map for detail-pane lookups. The end-to-end demo (Claude Desktop rendering substrate as text) doesn't depend on this. Cycle-1 stance: defer.
   222	
   223	**Q3 (meeting_briefing deferral).** Is the §4.1 deferral of `meeting_briefing` to W2-A Phase-2 acceptable given DOS-624 §3.4's explicit AC inclusion? Cycle-1 stance: producer mapping work is non-trivial and decoupled from the demo outcome; phase-2 sub-ticket of DOS-175 captures the residual without losing the audit trail. The DOS-624 §3.4 timing-floor / p95-burst test is itself a multi-actor-style defense that the local-to-local topology arguably does not require; reviewers may consider whether the original DOS-624 §3.4 requirement should itself be revisited or whether it's load-bearing for a future remote-to-local surface (W3-C WP MCP adapter).
   224	
   225	## 8. Reviewer dispatch (cycle 1)
   226	
   227	- `/codex challenge` — adversarial: try to break the handler design, the sync/async bridge, the registration ordering.
   228	- `architect-reviewer` — domain: substrate work, registry coupling, ADR-0102/0108/0125 conformance.
   229	- `/plan-devex-review` — MCP/DX surface: tool exposure, Claude Desktop pairing flow, error shapes.
   230	- `/codex consult` — fifth panelist: independent sanity check.
   231	
   232	K-in mandatory for each: grep `docs/solutions/` and `.docs/decisions/` for substrate cited above; cite hits in verdict. Unanimous required.
   233	
   234	## 9. Definition of done
   235	
   236	The session reaches DoD when:
   237	1. Unanimous L0 APPROVE from all four reviewers
   238	2. AC-1 through AC-10 validated with real data
   239	3. L2 (codex review + code-reviewer + architect-reviewer) unanimous APPROVE on the W2-A diff, bounded by AC
   240	4. L4 manual smoke (AC-9) executed and captured: screenshot of Claude Desktop returning substrate response for "give me an account overview for Acme"

exec
/bin/zsh -lc "nl -ba .docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md | sed -n '40,90p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
    40	MCP tool design is governed by the questions a user would ask through their host model, not by the rows in the database.
    41	
    42	- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
    43	- Rejected: `get_account(id)`, `list_claims(filter)`, `search(q)` — generic data accessors that bypass synthesis and force the host model to reassemble context.
    44	
    45	A tool that does not exercise an ability is a candidate for removal. This extends the v1.5.0 project Scope Guidance: convenience tools that do not strengthen parity with the DailyOS app loop are out of scope.
    46	
    47	### 3. Tool descriptions are UI copy
    48	
    49	Tool names and descriptions are the highest-leverage user-facing copy in the product. They determine when a host model reaches for DailyOS over alternatives (Glean, Slack search, web search), how it cites, and how it hedges.
    50	
    51	Each tool description should:
    52	
    53	- State the question shape it serves.
    54	- State when to prefer it over likely alternatives the host model has access to.
    55	- State the freshness model — DailyOS persists context across conversations and updates as work evolves.
    56	- Match the voice and vocabulary of [ADR-0083](0083-product-vocabulary.md) — same product diction as the Tauri app.
    57	
    58	Tool description copy is reviewed with the same care as a button label or section header.
    59	
    60	### 4. Claim-shaped output, with provenance leaking through
    61	
    62	Tool outputs are claim-shaped, not document-shaped. Each user-meaningful field carries inline trust band and source attribution per [ADR-0105](0105-provenance-as-first-class-output.md). The output structure should make it natural for a host model to render hedging when trust is low — *"according to a transcript from last Thursday, though this hasn't been confirmed elsewhere…"* — without explicit prompting.
    63	
    64	The substrate leaking through into the conversation *is* the product differentiator. It is what makes a DailyOS-mediated answer feel different from a flat retrieval over a corpus.
    65	
    66	### 5. Feedback is the only write — same posture, different surface
    67	
    68	Consumption-first holds in the headless head. The MCP write surface is **feedback only**: corrections, dismissals, corroborations, contradictions, and tombstones, all flowing through the existing claim feedback path defined in [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
    69	
    70	The MCP head does not expose:
    71	
    72	- Claim creation or direct edit.
    73	- File generation, drafting, or sending.
    74	- Calendar mutation, message composition, or external-system writes.
    75	
    76	A user in Claude Desktop can correct what DailyOS knows. They cannot use DailyOS as a write layer. Drafting and sending happen in the tool that's good at writing, with DailyOS context piped in.
    77	
    78	This preserves the v1.5.0 user outcome ("writes capture structured intent and provenance") in spirit by routing all writes through the same propose/commit boundary the Tauri app uses, with the same actor distinctions per [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
    79	
    80	### 6. Cross-conversation continuity is a named affordance
    81	
    82	Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.
    83	
    84	This is a quiet superpower and should be made explicit in the surface itself, not left as an implementation detail. Tool descriptions name it. Returned objects, where appropriate, surface it ("last verified Tuesday; no contradicting signals since"). The host model is given the language to communicate substrate persistence to the user.
    85	
    86	### 7. CLI as a third head, mediated by MCP
    87	
    88	The same headless framing extends to a CLI surface. CLI is a third head over the substrate, not a separate substrate consumer.
    89	
    90	**Transport.** CLI invocations route through the MCP server. There is no parallel CLI ingress to the abilities runtime. The default is MCP-mediated; a CLI-distinct transport is only justified if something genuinely cannot be expressed through the MCP surface, and that decision is deferred until the need is concrete. Keeping a single ingress to the abilities runtime is easier to reason about, easier to evolve, and prevents the v1.5.0 mission gate ("don't bypass guarantees") from being silently undermined by a second path.

exec
/bin/zsh -lc "nl -ba .docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md | sed -n '196,224p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   196	The top-of-file `Linear:` citation still points to the historical v1.5.0 project. Per the 2026-05-17 renumber, the canonical project for this work is now v1.4.7. The historical header is left intact for traceability; the post-renumber canonical project is [v1.4.7 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9).
   197	
   198	### §B. Tool description as product copy — operationalized
   199	
   200	Section 3 says "Tool descriptions are UI copy." v1.4.7 turns that posture into a versioned artifact and an eval gate. W1-A defines a typed `ToolDescription` struct in `services/mcp_v2/contracts.rs`; W1-B owns the tool-description YAML catalog at `src-tauri/resources/mcp_v2/tool_descriptions.yaml` as the version-controlled source.
   201	
   202	The host-selection eval in DOS-481, scheduled for v1.4.7 W5-A, is the product-copy acceptance test. Each tool ships with at least two positive fixtures and at least two negative fixtures: one broad-corpus negative where DailyOS should yield to enterprise or web search, and one adjacent-DailyOS-but-wrong-tool negative where another DailyOS tool is the right choice. The `when_NOT_to_call` field is mandatory and explicit because avoiding DailyOS for the wrong ask is part of the product surface, not merely an eval trick.
   203	
   204	### §C. Cross-conversation continuity — wire shape
   205	
   206	Section 6 names continuity as a headless-head affordance: host-model conversations are stateless, DailyOS is not. v1.4.7 pins the wire shape for that affordance to a server-minted `OpaqueConversationHandle`. The handle is required on writes, with the gateway minting one transparently if absent on a first write, and is optional but recommended on reads.
   207	
   208	The lifecycle is intentionally not restated here. Expiry is 24 hours from last touch, revocation is server-side, and raw caller-provided conversation IDs are rejected; the authoritative mint, echo, expiry, first-write, revocation, and "no raw IDs" contract lives in [ADR-0102](0102-abilities-as-runtime-contract.md) §D.
   209	
   210	### §D. v1.4.7 W4 write surface — narrowed
   211	
   212	Section 5 originally said "feedback is the only write." v1.4.7 preserves the consumption-first spirit while naming the W4 submit surface precisely: `dailyos.submit.note` in W4-A, `dailyos.submit.action` in W4-B, and `dailyos.submit.action_status` in W4-C.
   213	
   214	All three writes route through approved `services::*` boundaries per ADR-0101. Notes and observations flow through the claim/source services; actions and action-status updates flow through `services::actions`. The MCP head still does not expose claim creation or direct edit, file generation, calendar mutation, message composition, or external-system writes. Submit-class corrections and lightweight action tracking are the only mutations.
   215	
   216	### §E. Displacement framing extended to W5 eval
   217	
   218	The displacement use case remains the governing design check: DailyOS should win for the user's working understanding, and broad-corpus systems should win for broad enterprise or web retrieval. v1.4.7 carries that distinction into DOS-481 rather than leaving it as prose.
   219	
   220	Every v1.4.7 tool description therefore ships with positive host-selection fixtures where the model should pick DailyOS for the right personal-working-context ask, and negative fixtures where the model should avoid DailyOS for either broad-corpus questions or adjacent-DailyOS-but-wrong-tool questions. The eval is a product-strategy artifact as much as a regression test.
   221	
   222	### §F. Trust boundary — defer to ADR-0102 amendment
   223	
   224	The MCP client trust boundary is codified in the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment, especially §C: MCP client authentication, transport HMAC signing, server-side scope manifest authorization, per-actor × per-client × per-tool rate limits, revocation, and audit attribution with keyed HMAC parameter and response hashes. This ADR cites that contract because ADR-0128 is the product-surface frame, not the transport or authorization spec.

codex
One concrete DX gap is emerging around validation breadth: the packet validates listing both tools, but the acceptance path only proves one actual Claude-style call. I’m checking the gateway list behavior before I call that a finding, because listing unregistered catalog slots would be a worse user-facing failure.
exec
/bin/zsh -lc 'rg -n "list_tools|tools/list|validate_catalog_against_handlers|seal|handlers|ToolDescription|description_for" src-tauri/src/services/mcp_v2 src-tauri/src/bin src-tauri/src/mcp_v2 2>/dev/null' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/mcp_v2/main.rs:205:        .seal()
src-tauri/src/mcp_v2/main.rs:206:        .map_err(|e| format!("gateway seal: {e}"))?;
src-tauri/src/mcp_v2/main.rs:209:    let require_handlers = std::env::var("DAILYOS_MCP_V2_REQUIRE_HANDLERS")
src-tauri/src/mcp_v2/main.rs:213:        "mcp_v2 boot: pairing {} verified, 0 handlers registered, {} catalog entries pending. \
src-tauri/src/mcp_v2/main.rs:214:         tools/list will return empty for this build. Expected for W1.5 transport-only.",
src-tauri/src/mcp_v2/main.rs:218:    if require_handlers && !pending.is_empty() {
src-tauri/src/mcp_v2/main.rs:220:            "DAILYOS_MCP_V2_REQUIRE_HANDLERS=1 set but {} catalog entries lack handlers; refusing to start.",
src-tauri/src/mcp_v2/main.rs:225:    // Seed nonce — for W1.5 transport-only build with no handlers, the
src-tauri/src/services/mcp_v2/taxonomy.rs:23:    McpToolHandler, ParamSpec, ReturnSpec, Scope, ScopedName, Side, ToolDescription, ToolExample,
src-tauri/src/services/mcp_v2/taxonomy.rs:28:use super::contracts::ToolDescription as _ToolDescriptionImported;
src-tauri/src/services/mcp_v2/taxonomy.rs:41:/// boot via `Gateway::seal()` returning `Err(...)`; no `panic!()`.
src-tauri/src/services/mcp_v2/taxonomy.rs:166:/// Every concrete [`McpToolHandler`] in `services::mcp_v2::handlers::*`
src-tauri/src/services/mcp_v2/taxonomy.rs:171:/// ## Side::Read handlers
src-tauri/src/services/mcp_v2/taxonomy.rs:174:/// `ToolDescription.returns.schema` advertised in the catalog. There
src-tauri/src/services/mcp_v2/taxonomy.rs:177:/// ## Side::Write and Side::SubmitCorrection handlers — `mutation_cursor` field
src-tauri/src/services/mcp_v2/taxonomy.rs:193:    /// catalog entry with matching `Side`. Production `Gateway::seal()`
src-tauri/src/services/mcp_v2/taxonomy.rs:195:    fn validate_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:197:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:202:    /// handlers incrementally; the gateway logs the list as operator
src-tauri/src/services/mcp_v2/taxonomy.rs:204:    fn validate_catalog_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:206:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:212:    /// Look up the full `ToolDescription` for a tool by name. Used by
src-tauri/src/services/mcp_v2/taxonomy.rs:213:    /// the v2 transport (W1.5) to compose `tools/list` descriptions
src-tauri/src/services/mcp_v2/taxonomy.rs:215:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription>;
src-tauri/src/services/mcp_v2/taxonomy.rs:218:    /// `tools/list` to enumerate handler candidates before filtering by
src-tauri/src/services/mcp_v2/taxonomy.rs:228:/// Splits into `(ToolDescription, SelectionFixtures)` at load time via
src-tauri/src/services/mcp_v2/taxonomy.rs:248:    pub fn into_description_and_fixtures(self) -> (ToolDescription, SelectionFixtures) {
src-tauri/src/services/mcp_v2/taxonomy.rs:249:        let description = ToolDescription {
src-tauri/src/services/mcp_v2/taxonomy.rs:315:    entries: HashMap<ScopedName, (ToolDescription, SelectionFixtures)>,
src-tauri/src/services/mcp_v2/taxonomy.rs:397:    pub fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:407:    ) -> impl Iterator<Item = (&ScopedName, &ToolDescription, &SelectionFixtures)> {
src-tauri/src/services/mcp_v2/taxonomy.rs:421:    fn validate_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:423:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:425:        for handler in handlers {
src-tauri/src/services/mcp_v2/taxonomy.rs:449:    fn validate_catalog_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:451:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:454:            handlers.iter().map(|h| &h.description().name).collect();
src-tauri/src/services/mcp_v2/taxonomy.rs:466:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:559:                catalog.description_for(&ScopedName::new(name)).is_some(),
src-tauri/src/services/mcp_v2/gateway.rs:155:    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
src-tauri/src/services/mcp_v2/gateway.rs:163:            handlers: HashMap::new(),
src-tauri/src/services/mcp_v2/gateway.rs:171:            handlers: HashMap::new(),
src-tauri/src/services/mcp_v2/gateway.rs:179:        self.handlers.insert(name, handler);
src-tauri/src/services/mcp_v2/gateway.rs:183:        self.handlers.keys()
src-tauri/src/services/mcp_v2/gateway.rs:192:    /// Seal the gateway after all handlers are registered. Validates
src-tauri/src/services/mcp_v2/gateway.rs:198:    /// Production `main.rs` calls `gateway.seal()?` after registration.
src-tauri/src/services/mcp_v2/gateway.rs:200:    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
src-tauri/src/services/mcp_v2/gateway.rs:205:        let handlers: Vec<&dyn McpToolHandler> =
src-tauri/src/services/mcp_v2/gateway.rs:206:            self.handlers.values().map(|h| h.as_ref()).collect();
src-tauri/src/services/mcp_v2/gateway.rs:207:        catalog.validate_against_handlers(&handlers)?;
src-tauri/src/services/mcp_v2/gateway.rs:208:        Ok(catalog.validate_catalog_against_handlers(&handlers))
src-tauri/src/services/mcp_v2/gateway.rs:357:        let Some(handler) = self.handlers.get(&envelope.tool_name) else {
src-tauri/src/services/mcp_v2/contracts.rs:5:pub struct ToolDescription {
src-tauri/src/services/mcp_v2/contracts.rs:342:    fn description(&self) -> &ToolDescription;
src-tauri/src/services/mcp_v2/contracts.rs:413:    //! layer rather than downstream. TS clients (handlers, frontends,
src-tauri/src/services/mcp_v2/contracts.rs:599:        let desc = ToolDescription {
src-tauri/src/services/mcp_v2/auth.rs:545:/// exposure = `Invocable`. Consumed by the W1.5 transport `tools/list`
src-tauri/src/services/mcp_v2/mod.rs:6://! for MCP-originated invocations. Tool handlers never read the DB directly;
src-tauri/src/services/mcp_v2/mod.rs:11://! - contracts — shared wire-shape types — ToolDescription, ScopedName,
src-tauri/src/services/mcp_v2/mod.rs:16://! - handlers — per-tool handler implementations — placeholder.
src-tauri/src/services/mcp_v2/mod.rs:28:pub mod handlers;
src-tauri/src/services/mcp_v2/transport.rs:19://!   in public `tools/list` `input_schema`.
src-tauri/src/services/mcp_v2/transport.rs:44:    ScopedName, ToolDescription, ToolError,
src-tauri/src/services/mcp_v2/transport.rs:98:/// Build a JSON Schema fragment from a `ToolDescription`'s parameter
src-tauri/src/services/mcp_v2/transport.rs:101:fn build_input_schema(desc: &ToolDescription) -> JsonObject {
src-tauri/src/services/mcp_v2/transport.rs:121:fn tool_from_description(desc: &ToolDescription) -> Tool {
src-tauri/src/services/mcp_v2/transport.rs:157:    async fn list_tools(
src-tauri/src/services/mcp_v2/transport.rs:178:            if let Some(desc) = self.catalog.description_for(grant_name) {
src-tauri/src/services/mcp_v2/transport.rs:343:    fn fake_desc(name: &str) -> ToolDescription {
src-tauri/src/services/mcp_v2/transport.rs:344:        ToolDescription {

exec
/bin/zsh -lc 'rg -n "struct Gateway|impl Gateway|fn seal|fn list|validate_catalog_against_handlers|description_for" src-tauri/src/services/mcp_v2' in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc 'rg -n "ToolDescription|when_NOT_to_call|when_not_to_call|selectionFixtures|selection_fixtures" src-tauri/src/services/mcp_v2' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/services/mcp_v2/taxonomy.rs:204:    fn validate_catalog_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:215:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription>;
src-tauri/src/services/mcp_v2/taxonomy.rs:397:    pub fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:449:    fn validate_catalog_against_handlers(
src-tauri/src/services/mcp_v2/taxonomy.rs:466:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:559:                catalog.description_for(&ScopedName::new(name)).is_some(),
src-tauri/src/services/mcp_v2/gateway.rs:154:pub struct Gateway {
src-tauri/src/services/mcp_v2/gateway.rs:160:impl Gateway {
src-tauri/src/services/mcp_v2/gateway.rs:200:    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
src-tauri/src/services/mcp_v2/gateway.rs:208:        Ok(catalog.validate_catalog_against_handlers(&handlers))
src-tauri/src/services/mcp_v2/gateway.rs:552:struct GatewayFailure {
src-tauri/src/services/mcp_v2/auth.rs:548:pub fn list_invocable_tool_grants(
src-tauri/src/services/mcp_v2/transport.rs:157:    async fn list_tools(
src-tauri/src/services/mcp_v2/transport.rs:178:            if let Some(desc) = self.catalog.description_for(grant_name) {

 succeeded in 0ms:
src-tauri/src/services/mcp_v2/taxonomy.rs:23:    McpToolHandler, ParamSpec, ReturnSpec, Scope, ScopedName, Side, ToolDescription, ToolExample,
src-tauri/src/services/mcp_v2/taxonomy.rs:28:use super::contracts::ToolDescription as _ToolDescriptionImported;
src-tauri/src/services/mcp_v2/taxonomy.rs:80:    /// An entry's `selectionFixtures` lacks the AC-5 coverage minimums
src-tauri/src/services/mcp_v2/taxonomy.rs:147:                "mcp_v2 taxonomy: tool `{tool}` selectionFixtures missing required \
src-tauri/src/services/mcp_v2/taxonomy.rs:174:/// `ToolDescription.returns.schema` advertised in the catalog. There
src-tauri/src/services/mcp_v2/taxonomy.rs:212:    /// Look up the full `ToolDescription` for a tool by name. Used by
src-tauri/src/services/mcp_v2/taxonomy.rs:214:    /// from `summary` + `when_to_call` + `when_NOT_to_call`.
src-tauri/src/services/mcp_v2/taxonomy.rs:215:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription>;
src-tauri/src/services/mcp_v2/taxonomy.rs:228:/// Splits into `(ToolDescription, SelectionFixtures)` at load time via
src-tauri/src/services/mcp_v2/taxonomy.rs:238:    #[serde(rename = "when_NOT_to_call")]
src-tauri/src/services/mcp_v2/taxonomy.rs:239:    pub when_not_to_call: String,
src-tauri/src/services/mcp_v2/taxonomy.rs:244:    pub selection_fixtures: SelectionFixtures,
src-tauri/src/services/mcp_v2/taxonomy.rs:248:    pub fn into_description_and_fixtures(self) -> (ToolDescription, SelectionFixtures) {
src-tauri/src/services/mcp_v2/taxonomy.rs:249:        let description = ToolDescription {
src-tauri/src/services/mcp_v2/taxonomy.rs:253:            when_not_to_call: self.when_not_to_call,
src-tauri/src/services/mcp_v2/taxonomy.rs:260:        (description, self.selection_fixtures)
src-tauri/src/services/mcp_v2/taxonomy.rs:315:    entries: HashMap<ScopedName, (ToolDescription, SelectionFixtures)>,
src-tauri/src/services/mcp_v2/taxonomy.rs:397:    pub fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:407:    ) -> impl Iterator<Item = (&ScopedName, &ToolDescription, &SelectionFixtures)> {
src-tauri/src/services/mcp_v2/taxonomy.rs:466:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:572:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:577:  selectionFixtures:
src-tauri/src/services/mcp_v2/taxonomy.rs:595:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:600:  selectionFixtures:
src-tauri/src/services/mcp_v2/taxonomy.rs:618:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:623:  selectionFixtures:
src-tauri/src/services/mcp_v2/taxonomy.rs:631:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:636:  selectionFixtures:
src-tauri/src/services/mcp_v2/taxonomy.rs:654:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:659:  selectionFixtures:
src-tauri/src/services/mcp_v2/taxonomy.rs:678:  when_NOT_to_call: z
src-tauri/src/services/mcp_v2/taxonomy.rs:683:  selectionFixtures:
src-tauri/src/services/mcp_v2/contracts.rs:5:pub struct ToolDescription {
src-tauri/src/services/mcp_v2/contracts.rs:9:    #[serde(rename = "when_NOT_to_call")]
src-tauri/src/services/mcp_v2/contracts.rs:10:    pub when_not_to_call: String,
src-tauri/src/services/mcp_v2/contracts.rs:342:    fn description(&self) -> &ToolDescription;
src-tauri/src/services/mcp_v2/contracts.rs:594:    fn tool_description_wire_when_not_to_call_keeps_uppercase_alias() {
src-tauri/src/services/mcp_v2/contracts.rs:595:        // `when_NOT_to_call` deliberately preserves the uppercase NOT
src-tauri/src/services/mcp_v2/contracts.rs:599:        let desc = ToolDescription {
src-tauri/src/services/mcp_v2/contracts.rs:603:            when_not_to_call: "Do NOT call for broad enterprise search.".to_string(),
src-tauri/src/services/mcp_v2/contracts.rs:614:        // The field is renamed to the literal `when_NOT_to_call` token.
src-tauri/src/services/mcp_v2/contracts.rs:615:        assert!(encoded.get("when_NOT_to_call").is_some());
src-tauri/src/services/mcp_v2/mod.rs:11://! - contracts — shared wire-shape types — ToolDescription, ScopedName,
src-tauri/src/services/mcp_v2/transport.rs:44:    ScopedName, ToolDescription, ToolError,
src-tauri/src/services/mcp_v2/transport.rs:98:/// Build a JSON Schema fragment from a `ToolDescription`'s parameter
src-tauri/src/services/mcp_v2/transport.rs:101:fn build_input_schema(desc: &ToolDescription) -> JsonObject {
src-tauri/src/services/mcp_v2/transport.rs:121:fn tool_from_description(desc: &ToolDescription) -> Tool {
src-tauri/src/services/mcp_v2/transport.rs:126:        desc.when_not_to_call.trim(),
src-tauri/src/services/mcp_v2/transport.rs:343:    fn fake_desc(name: &str) -> ToolDescription {
src-tauri/src/services/mcp_v2/transport.rs:344:        ToolDescription {
src-tauri/src/services/mcp_v2/transport.rs:348:            when_not_to_call: "test when not".into(),

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/auth.rs | sed -n '535,570p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/transport.rs | sed -n '110,205p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/src/mcp_v2/main.rs | sed -n '180,225p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   535	            nonce.as_str(),
   536	            client_id.as_str(),
   537	            now,
   538	            now + NONCE_EXPIRY_SECONDS * 1000,
   539	        ],
   540	    )?;
   541	    Ok(nonce)
   542	}
   543	
   544	/// Return every tool name from `mcp_tool_grant` for `client_id` where
   545	/// exposure = `Invocable`. Consumed by the W1.5 transport `tools/list`
   546	/// per the DOS-MCP-Transport L0 AC-6: only Invocable-tier grants are
   547	/// surfaced to the host model.
   548	pub fn list_invocable_tool_grants(
   549	    conn: &Connection,
   550	    client_id: &McpClientId,
   551	) -> Result<Vec<ScopedName>, AuthError> {
   552	    let mut stmt = conn.prepare_cached(
   553	        "SELECT tool_name FROM mcp_tool_grant \
   554	         WHERE client_id = ?1 AND exposure = 'Invocable' \
   555	         ORDER BY tool_name ASC",
   556	    )?;
   557	    let rows = stmt.query_map(params![client_id.as_str()], |row| {
   558	        let name: String = row.get(0)?;
   559	        Ok(ScopedName::new(name))
   560	    })?;
   561	    let mut out = Vec::new();
   562	    for row in rows {
   563	        out.push(row?);
   564	    }
   565	    Ok(out)
   566	}

 succeeded in 0ms:
   110	    let mut schema = serde_json::Map::new();
   111	    schema.insert("type".into(), serde_json::Value::String("object".into()));
   112	    schema.insert("properties".into(), serde_json::Value::Object(properties));
   113	    schema.insert("required".into(), serde_json::Value::Array(required));
   114	    schema.insert(
   115	        "additionalProperties".into(),
   116	        serde_json::Value::Bool(false),
   117	    );
   118	    schema
   119	}
   120	
   121	fn tool_from_description(desc: &ToolDescription) -> Tool {
   122	    let description = format!(
   123	        "{}\n\nWhen to call:\n{}\n\nWhen NOT to call:\n{}",
   124	        desc.summary.trim(),
   125	        desc.when_to_call.trim(),
   126	        desc.when_not_to_call.trim(),
   127	    );
   128	    Tool {
   129	        name: desc.name.as_str().to_string().into(),
   130	        description: description.into(),
   131	        input_schema: Arc::new(build_input_schema(desc)),
   132	    }
   133	}
   134	
   135	// ---------------------------------------------------------------------------
   136	// rmcp::ServerHandler impl
   137	// ---------------------------------------------------------------------------
   138	
   139	impl ServerHandler for V2ServerHandler {
   140	    fn get_info(&self) -> ServerInfo {
   141	        ServerInfo {
   142	            protocol_version: ProtocolVersion::V_2024_11_05,
   143	            capabilities: ServerCapabilities::builder().enable_tools().build(),
   144	            server_info: Implementation {
   145	                name: "dailyos-mcp-v2".into(),
   146	                version: env!("CARGO_PKG_VERSION").into(),
   147	            },
   148	            instructions: Some(
   149	                "DailyOS MCP v2. Personal working intelligence for the paired user — \
   150	                 accounts, briefings, attention, working memory, notes, actions. \
   151	                 Not for broad enterprise or web corpus search."
   152	                    .to_string(),
   153	            ),
   154	        }
   155	    }
   156	
   157	    async fn list_tools(
   158	        &self,
   159	        _request: PaginatedRequestParam,
   160	        _context: RequestContext<RoleServer>,
   161	    ) -> Result<ListToolsResult, ErrorData> {
   162	        // Per L0 AC-6: filter by `mcp_tool_grant` rows for
   163	        // `verified_client_id` where exposure = Invocable.
   164	        let grants = {
   165	            let conn_guard = self.conn.lock();
   166	            auth::list_invocable_tool_grants(&conn_guard, &self.verified_client_id).map_err(
   167	                |e| {
   168	                    ErrorData::internal_error(
   169	                        format!("mcp_v2 tool_grant lookup: {e}"),
   170	                        None,
   171	                    )
   172	                },
   173	            )?
   174	        };
   175	
   176	        let mut tools = Vec::with_capacity(grants.len());
   177	        for grant_name in &grants {
   178	            if let Some(desc) = self.catalog.description_for(grant_name) {
   179	                tools.push(tool_from_description(desc));
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

 succeeded in 0ms:
   180	            client_id.as_str()
   181	        ));
   182	    }
   183	
   184	    // Best-effort env-wipe per L0 AC-12 step 5. This is in-process
   185	    // hygiene only — does NOT scrub /proc/<pid>/environ which retains
   186	    // the initial exec environment for process lifetime per kernel
   187	    // behavior. Acceptable under §0 scope (same-user same-machine).
   188	    //
   189	    // SAFETY: runs at startup before any threads/runtime init; sets are
   190	    // not racing with reads.
   191	    unsafe {
   192	        std::env::remove_var(ENV_CLIENT_ID);
   193	        std::env::remove_var(ENV_TRANSPORT_KEY);
   194	    }
   195	
   196	    // Construct taxonomy + gateway. Seal at boot (empty handler set is
   197	    // expected for W1.5 transport-only build; pending list logged).
   198	    let catalog = YamlTaxonomyCatalog::load_embedded()
   199	        .map_err(|e| format!("load embedded taxonomy: {e}"))?;
   200	    let catalog: Arc<dyn dailyos_lib::services::mcp_v2::taxonomy::TaxonomyCatalog> =
   201	        Arc::new(catalog);
   202	    let mut gateway = Gateway::new();
   203	    gateway.set_taxonomy(catalog.clone());
   204	    let pending = gateway
   205	        .seal()
   206	        .map_err(|e| format!("gateway seal: {e}"))?;
   207	
   208	    // Per L0 AC-4 boot log to stderr (stdout reserved for MCP protocol).
   209	    let require_handlers = std::env::var("DAILYOS_MCP_V2_REQUIRE_HANDLERS")
   210	        .map(|v| v == "1")
   211	        .unwrap_or(false);
   212	    eprintln!(
   213	        "mcp_v2 boot: pairing {} verified, 0 handlers registered, {} catalog entries pending. \
   214	         tools/list will return empty for this build. Expected for W1.5 transport-only.",
   215	        client_id.as_str(),
   216	        pending.len(),
   217	    );
   218	    if require_handlers && !pending.is_empty() {
   219	        return Err(format!(
   220	            "DAILYOS_MCP_V2_REQUIRE_HANDLERS=1 set but {} catalog entries lack handlers; refusing to start.",
   221	            pending.len()
   222	        ));
   223	    }
   224	
   225	    // Seed nonce — for W1.5 transport-only build with no handlers, the

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/gateway.rs | sed -n '176,214p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   176	
   177	    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
   178	        let name = handler.description().name.clone();
   179	        self.handlers.insert(name, handler);
   180	    }
   181	
   182	    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
   183	        self.handlers.keys()
   184	    }
   185	
   186	    /// Register the taxonomy catalog. Production callers wire this with
   187	    /// `YamlTaxonomyCatalog::load_embedded()`. Tests may use a stub.
   188	    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
   189	        self.taxonomy = Some(catalog);
   190	    }
   191	
   192	    /// Seal the gateway after all handlers are registered. Validates
   193	    /// every registered handler has a matching catalog entry with matching
   194	    /// `Side` (per DOS-478 L0 AC-7). Returns operator-readable error if
   195	    /// mismatch. Also returns catalog→handler pending-tool list as
   196	    /// operator info; callers log it.
   197	    ///
   198	    /// Production `main.rs` calls `gateway.seal()?` after registration.
   199	    /// No `panic!()` — fail-fast via structured `Result`.
   200	    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
   201	        let Some(catalog) = self.taxonomy.as_ref() else {
   202	            // No taxonomy registered: tests / dev mode skip validation.
   203	            return Ok(Vec::new());
   204	        };
   205	        let handlers: Vec<&dyn McpToolHandler> =
   206	            self.handlers.values().map(|h| h.as_ref()).collect();
   207	        catalog.validate_against_handlers(&handlers)?;
   208	        Ok(catalog.validate_catalog_against_handlers(&handlers))
   209	    }
   210	
   211	    /// Handle a single MCP tool invocation. Full per-dispatch contract per
   212	    /// L0 packet §1 #1; gate ordering matches AC-2 verbatim. The Gate 0b
   213	    /// preissued nonce is captured and returned in the response envelope on
   214	    /// BOTH success and rejection paths (closes L2 code-reviewer HIGH +

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/transport.rs | sed -n '40,110p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
    40	
    41	use super::auth;
    42	use super::contracts::{
    43	    McpClientId, McpToolRequestEnvelope, McpToolResponseEnvelope, McpToolResult, OpaqueNonce,
    44	    ScopedName, ToolDescription, ToolError,
    45	};
    46	use super::gateway::Gateway;
    47	use super::taxonomy::TaxonomyCatalog;
    48	
    49	// ---------------------------------------------------------------------------
    50	// V2ServerHandler
    51	// ---------------------------------------------------------------------------
    52	
    53	/// Bridges `rmcp::ServerHandler` to the v2 `Gateway`. Constructed AFTER
    54	/// successful env-assertion + manifest lookup + keychain cross-check in
    55	/// `main.rs`. Construction failure (any of those checks fail) exits the
    56	/// process before the rmcp service runs.
    57	///
    58	/// rmcp::ServerHandler requires Clone; all fields are Arc-wrapped so
    59	/// cloning is shallow (no transport_key bytes copied per clone).
    60	#[derive(Clone)]
    61	pub struct V2ServerHandler {
    62	    gateway: Arc<Gateway>,
    63	    catalog: Arc<dyn TaxonomyCatalog>,
    64	    conn: Arc<Mutex<Connection>>,
    65	    verified_client_id: McpClientId,
    66	    /// Process-held transport key. Wrapped in `Zeroizing` so the bytes
    67	    /// are wiped on drop. Arc-wrapped so V2ServerHandler::Clone is
    68	    /// shallow (no per-clone copies of the secret).
    69	    transport_key: Arc<Zeroizing<[u8; 32]>>,
    70	    /// Most-recently preissued nonce — caller of the next `tools/call`
    71	    /// must present this as `request_nonce`.
    72	    next_nonce: Arc<Mutex<Option<OpaqueNonce>>>,
    73	}
    74	
    75	impl V2ServerHandler {
    76	    /// Construct after env-assertion + keychain cross-check succeed in
    77	    /// main.rs. `initial_nonce` is the seed nonce from `pair_client`
    78	    /// (issued at pairing time, valid until first successful call).
    79	    pub fn from_verified_pairing(
    80	        gateway: Arc<Gateway>,
    81	        catalog: Arc<dyn TaxonomyCatalog>,
    82	        conn: Arc<Mutex<Connection>>,
    83	        verified_client_id: McpClientId,
    84	        transport_key: Zeroizing<[u8; 32]>,
    85	        initial_nonce: OpaqueNonce,
    86	    ) -> Self {
    87	        Self {
    88	            gateway,
    89	            catalog,
    90	            conn,
    91	            verified_client_id,
    92	            transport_key: Arc::new(transport_key),
    93	            next_nonce: Arc::new(Mutex::new(Some(initial_nonce))),
    94	        }
    95	    }
    96	}
    97	
    98	/// Build a JSON Schema fragment from a `ToolDescription`'s parameter
    99	/// list. Per L0 AC-6, transport self-signs and `_dailyos_*` keys are
   100	/// NOT advertised — clients send normal handler params only.
   101	fn build_input_schema(desc: &ToolDescription) -> JsonObject {
   102	    let mut properties = serde_json::Map::new();
   103	    let mut required = Vec::new();
   104	    for param in &desc.parameters {
   105	        properties.insert(param.name.clone(), param.schema.0.clone());
   106	        if param.required {
   107	            required.push(serde_json::Value::String(param.name.clone()));
   108	        }
   109	    }
   110	    let mut schema = serde_json::Map::new();

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/transport.rs | sed -n '190,275p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
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
   233	        let response_envelope = {
   234	            let mut conn_guard = self.conn.lock();
   235	            self.gateway.handle_tool_call(
   236	                &mut conn_guard,
   237	                &self.verified_client_id,
   238	                envelope,
   239	                signature.as_ref(),
   240	            )
   241	        };
   242	
   243	        // Store the gateway's preissued next_nonce for the NEXT call.
   244	        // (Even on error responses the gateway returns a preissued nonce
   245	        // so the caller can recover.)
   246	        *self.next_nonce.lock() = Some(response_envelope.next_request_nonce.clone());
   247	
   248	        unwrap_response(response_envelope)
   249	    }
   250	}
   251	
   252	/// Translate the gateway's response envelope into an rmcp result.
   253	fn unwrap_response(env: McpToolResponseEnvelope) -> Result<CallToolResult, ErrorData> {
   254	    match env.result {
   255	        McpToolResult::Ok { value } => {
   256	            let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
   257	            Ok(CallToolResult::success(vec![Content::text(text)]))
   258	        }
   259	        McpToolResult::Error { error } => Err(tool_error_to_mcp_error(error)),
   260	    }
   261	}
   262	
   263	/// Per L0 AC-3 closed-matrix mapping from `ToolError` to `rmcp::ErrorData`.
   264	fn tool_error_to_mcp_error(err: ToolError) -> ErrorData {
   265	    use serde_json::json;
   266	    match err {
   267	        ToolError::Unauthorized { missing_scope } => ErrorData::invalid_request(
   268	            "tool requires scope your pairing lacks".to_string(),
   269	            Some(json!({
   270	                "kind": "unauthorized",
   271	                "missing_scope": missing_scope.as_str(),
   272	            })),
   273	        ),
   274	        ToolError::BadParams { detail: _ } => {
   275	            // detail intentionally NOT included in public payload per AC-3;

exec
/bin/zsh -lc "rg -n \"block_on\\(|Handle::current|spawn_blocking|McpToolHandler|invoke_registry_json\" src-tauri/src src-tauri/abilities-runtime/src | head -200" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/reports/book_of_business.rs:2033:            let result = rt.block_on(async {
src-tauri/src/services/claims_backfill.rs:1707:            let drain_remaining = tokio::task::spawn_blocking(move || drain_cutover_fence(context))
src-tauri/src/services/claims_backfill.rs:1744:            let drain_remaining = tokio::task::spawn_blocking(move || drain_cutover_fence(context))
src-tauri/src/services/context.rs:82:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:103:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:128:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:210:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:228:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:334:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:359:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:376:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/context.rs:418:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/reports.rs:1://! Report generation service — async dispatch with spawn_blocking.
src-tauri/src/services/reports.rs:33:    let task = tauri::async_runtime::spawn_blocking(move || -> Result<ReportRow, String> {
src-tauri/src/services/reports.rs:55:        // Single ActionDb open per spawn_blocking; threaded through gather, user-context, and write.
src-tauri/src/services/fail_improve.rs:87:        match tokio::task::spawn_blocking(move || {
src-tauri/src/services/fail_improve.rs:243:                    Ok(runtime) => runtime.block_on(Self::finish_signal_type_resolution(name)),
src-tauri/src/services/entity_intelligence/touchpoints.rs:41:            tokio::task::spawn_blocking(move || {
src-tauri/src/services/mcp_v2/taxonomy.rs:23:    McpToolHandler, ParamSpec, ReturnSpec, Scope, ScopedName, Side, ToolDescription, ToolExample,
src-tauri/src/services/mcp_v2/taxonomy.rs:35:/// registered [`McpToolHandler`] implementations at startup, or when
src-tauri/src/services/mcp_v2/taxonomy.rs:166:/// Every concrete [`McpToolHandler`] in `services::mcp_v2::handlers::*`
src-tauri/src/services/mcp_v2/taxonomy.rs:197:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:206:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:423:        handlers: &[&dyn McpToolHandler],
src-tauri/src/services/mcp_v2/taxonomy.rs:451:        handlers: &[&dyn McpToolHandler],
src-tauri/src/signals/bus.rs:1325:        runtime.block_on(async {
src-tauri/src/commands/planning_reports.rs:1011:/// All blocking work (DB lock + file I/O + PTY) runs in spawn_blocking
src-tauri/src/services/mcp_v2/handlers/mod.rs:1://! Per-tool MCP handler modules. Each handler implements McpToolHandler.
src-tauri/src/commands/workspace.rs:127:    tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/commands/workspace.rs:171:    tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/commands/workspace.rs:215:    tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/commands/workspace.rs:1534:    let result = tokio::task::spawn_blocking(move || {
src-tauri/src/services/meetings.rs:1417:            match tauri::async_runtime::spawn_blocking({
src-tauri/src/services/meetings.rs:4603:            match tauri::async_runtime::spawn_blocking({
src-tauri/src/services/meetings.rs:4944:    let result = match tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/lib.rs:315:                    match tokio::task::spawn_blocking(move || model.initialize(models_dir)).await {
src-tauri/src/lib.rs:341:            tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/intel_queue.rs:863:                    match tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/intel_queue.rs:1411:                match tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/intel_queue.rs:1454:            match tokio::task::spawn_blocking(move || {
src-tauri/src/intel_queue.rs:1596:            match tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/commands/integrations.rs:781:    let (result, attached_event) = tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/services/mcp_v2/contracts.rs:341:pub trait McpToolHandler: Send + Sync {
src-tauri/src/db_service.rs:338:        let (writer, hex_key) = tokio::task::spawn_blocking(move || {
src-tauri/src/db_service.rs:351:            let r = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
src-tauri/src/db_service.rs:394:        let writer = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
src-tauri/src/db_service.rs:409:            let r = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
src-tauri/src/db_service.rs:781:        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<_>, String> {
src-tauri/src/db_service.rs:945:        let writer_task = tokio::task::spawn_blocking(move || -> Result<(), String> {
src-tauri/src/harness/runner.rs:335:        .block_on(eval_bridge.invoke_json(
src-tauri/src/commands/app_support.rs:713:/// via `spawn_blocking` so it never ties up a Tauri IPC thread.
src-tauri/src/commands/app_support.rs:763:    let status = tokio::task::spawn_blocking(|| {
src-tauri/src/commands/app_support.rs:1051:    // NOTE: If `spawn_blocking` panics, the guard won't reset. Acceptable for
src-tauri/src/commands/app_support.rs:1053:    let result = tokio::task::spawn_blocking(move || {
src-tauri/src/bridges/mcp.rs:14:    confirmation_args_hash, invoke_registry_json, surface_error, BridgeNoopIntelligenceProvider,
src-tauri/src/bridges/mcp.rs:411:        let response = invoke_registry_json(
src-tauri/src/services/mcp_v2/gateway.rs:4://! loaded by `auth`; dispatches through registered `McpToolHandler`
src-tauri/src/services/mcp_v2/gateway.rs:24:    McpActor, McpClientId, McpToolHandler, McpToolRequestEnvelope, McpToolResponseEnvelope,
src-tauri/src/services/mcp_v2/gateway.rs:155:    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
src-tauri/src/services/mcp_v2/gateway.rs:177:    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
src-tauri/src/services/mcp_v2/gateway.rs:205:        let handlers: Vec<&dyn McpToolHandler> =
src-tauri/src/bridges/eval.rs:3:use crate::bridges::types::{invoke_registry_json, surface_error};
src-tauri/src/bridges/eval.rs:47:        invoke_registry_json(
src-tauri/src/bridges/worker.rs:3:use crate::bridges::types::{invoke_registry_json, provider_from_context_snapshot, surface_error};
src-tauri/src/bridges/worker.rs:57:        invoke_registry_json(
src-tauri/src/bridges/tauri.rs:13:    claim_dismissal_surface_for_non_tauri_bridge, invoke_registry_json,
src-tauri/src/bridges/tauri.rs:217:        invoke_registry_json(
src-tauri/src/bridges/tauri.rs:291:        invoke_registry_json(
src-tauri/src/bridges/types.rs:445:pub(crate) async fn invoke_registry_json<'a>(
src-tauri/src/bridges/types.rs:513:pub(crate) async fn invoke_registry_json_for_actor<'a>(
src-tauri/src/bridges/types.rs:1550:        let meeting_response = invoke_registry_json(
src-tauri/src/bridges/types.rs:1569:        let entity_detail_response = invoke_registry_json(
src-tauri/src/mcp_v2/main.rs:257:        .block_on(async move {
src-tauri/src/services/intelligence.rs:653:                let pty_result = tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/services/intelligence.rs:700:        let pty_result = tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/services/intelligence.rs:1866:    let task = tauri::async_runtime::spawn_blocking(move || {
src-tauri/src/surface_runtime/mod.rs:43:    invoke_registry_json_for_actor, provider_from_context_snapshot, surface_error,
src-tauri/src/surface_runtime/mod.rs:2146:                    match invoke_registry_json_for_actor(
src-tauri/src/surface_runtime/mod.rs:2609:    let invoke_outcome = invoke_registry_json_for_actor(
src-tauri/src/surface_runtime/mod.rs:4664:            .block_on(signed_route_response(
src-tauri/src/surface_runtime/mod.rs:4681:            .block_on(dispatch_surface_request(request, runtime, request_id))
src-tauri/src/surface_runtime/mod.rs:5035:            .block_on(crate::db_service::DbService::open_at_unencrypted(db_path))
src-tauri/src/surface_runtime/mod.rs:5038:        let outcome = tokio_runtime.block_on({
src-tauri/src/surface_runtime/mod.rs:5720:        let runtime = tokio_runtime.block_on(runtime_for_project_composition_tests(
src-tauri/src/surface_runtime/mod.rs:5725:        tokio_runtime.block_on(seed_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5741:        let response = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5766:        let runtime = tokio_runtime.block_on(runtime_for_project_composition_tests(
src-tauri/src/surface_runtime/mod.rs:5771:        tokio_runtime.block_on(seed_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5787:        let response = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5811:        let runtime = tokio_runtime.block_on(runtime_for_project_composition_tests(
src-tauri/src/surface_runtime/mod.rs:5816:        tokio_runtime.block_on(seed_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5824:        let response = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5862:        let runtime = tokio_runtime.block_on(runtime_for_project_composition_tests(
src-tauri/src/surface_runtime/mod.rs:5867:        tokio_runtime.block_on(seed_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5883:        tokio_runtime.block_on(commit_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5889:        let response = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5918:        tokio_runtime.block_on(commit_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5923:        let response = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5955:        let runtime = tokio_runtime.block_on(runtime_for_project_composition_tests(
src-tauri/src/surface_runtime/mod.rs:5960:        tokio_runtime.block_on(seed_composition_version_for_tests(
src-tauri/src/surface_runtime/mod.rs:5976:        let first = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:5982:        let second = tokio_runtime.block_on(surface_project_composition_response(
src-tauri/src/surface_runtime/mod.rs:6335:        tokio::task::spawn_blocking(move || {
src-tauri/src/surface_runtime/mod.rs:6402:            runtime.block_on(async { response.into_body().collect().await.unwrap().to_bytes() });
src-tauri/src/context_provider/glean.rs:163:                    rt.block_on(crate::glean::get_valid_access_token())
src-tauri/src/context_provider/glean.rs:1492:                        rt.block_on(async {
src-tauri/src/intelligence/pty_provider.rs:19://! `tokio::task::spawn_blocking`. This matches the existing
src-tauri/src/intelligence/pty_provider.rs:148:    /// from `std::thread::spawn` / `spawn_blocking`. The trait `complete()`
src-tauri/src/intelligence/pty_provider.rs:218:        let join = tokio::task::spawn_blocking(move || {
src-tauri/src/intelligence/pty_provider.rs:230:        .map_err(|e| ProviderError::Permanent(format!("spawn_blocking join error: {e}")))?;
src-tauri/src/google_api/mod.rs:446:    let token = tokio::task::spawn_blocking(load_token)
src-tauri/src/meeting_prep_queue.rs:370:        let result = tokio::task::spawn_blocking(move || {
src-tauri/src/meeting_prep_queue.rs:824:    let prep_json = match tokio::task::spawn_blocking(move || {
src-tauri/src/meeting_prep_queue.rs:863:    let enriched = match tokio::task::spawn_blocking(move || {

exec
/bin/zsh -lc "nl -ba src-tauri/src/bridges/mcp.rs | sed -n '370,435p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   370	    }
   371	
   372	    pub fn confirmation_enabled(&self) -> bool {
   373	        self.mcp_confirmation_enabled
   374	    }
   375	
   376	    /// Returns the cached actor-filtered descriptor list. The bridge does NOT
   377	    /// re-scan the registry on every call; it scans once at construction.
   378	    pub fn list_descriptors(&self) -> &[&'registry AbilityDescriptor] {
   379	        &self.actor_filtered_descriptors
   380	    }
   381	
   382	    /// Invoke an ability by name. Re-fetches policy by name at call time
   383	    /// (does not trust cached descriptors for authorization). Maps every
   384	    /// reject reason to BridgeSurfaceError::AbilityUnavailable byte-equal.
   385	    pub async fn invoke_ability(
   386	        &self,
   387	        session: McpSessionId,
   388	        ability_name: &str,
   389	        input_json: serde_json::Value,
   390	        dry_run: bool,
   391	        confirmation: Option<ConfirmationToken>,
   392	    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
   393	        let clock = SystemClock;
   394	        let rng = SystemRng;
   395	        let external = ExternalClients::default();
   396	        let mut services =
   397	            ServiceContext::new_live(&clock, &rng, &external).with_actor(MCP_ACTOR_LABEL);
   398	        if let Some(readers) = &self.workspace_readers {
   399	            services = readers.attach_to(services);
   400	        }
   401	        let invocation = InvocationContext {
   402	            actor: BridgeActor::Agent,
   403	            mode: ExecutionMode::Live,
   404	            surface: BridgeSurface::McpTool,
   405	            claim_dismissal_surface: crate::services::context::ClaimDismissalSurface::McpTool,
   406	            dry_run,
   407	            confirmation: confirmation.as_ref(),
   408	            confirmation_store: None,
   409	        };
   410	
   411	        let response = invoke_registry_json(
   412	            self.registry,
   413	            &services,
   414	            self.provider.as_ref(),
   415	            self.tracer.as_ref(),
   416	            invocation,
   417	            ability_name,
   418	            input_json,
   419	        )
   420	        .await
   421	        .map_err(surface_error)?;
   422	
   423	        self.insert_provenance(
   424	            session,
   425	            response.invocation_id,
   426	            response.render_cached_provenance(BridgeActor::Agent, BridgeSurface::McpToolDetail),
   427	        );
   428	
   429	        Ok(response)
   430	    }
   431	
   432	    /// Bounded session-scoped lookup. Returns RenderedProvenance only if the
   433	    /// (session, invocation_id) pair was previously returned by this same
   434	    /// session. No maintenance-audit fallback; no cross-session lookup.
   435	    pub fn get_provenance(

codex
VERDICT: NEEDS-CHANGES

Findings

- HIGH: `runtime.block_on(...)` inside sync `invoke()` will be reached from async MCP `call_tool`; Claude may see a panic/hung tool call instead of output. Mitigation: make the gateway/handler path async, or move dispatch onto a proven blocking boundary with a dedicated runtime. Cite: [plan §3.1](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:57), [transport call_tool](/private/tmp/dailyos-v147-w1a/src-tauri/src/services/mcp_v2/transport.rs:190).

- MED: E2E acceptance proves `tools/list` and only `account_status` call; `daily_briefing` is one of two shipped tools but not in AC-8/AC-9 manual smoke. User asking “what’s on my plate today?” remains unproven. Mitigation: update AC-8/AC-9 to call both tools and assert useful JSON output. Cite: [AC-8](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203), [AC-9](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205).

- LOW: `account_status.when_to_call` says “recent signals,” leaking system vocabulary into model-facing copy. Mitigation: use “recent updates/changes.” Cite: [tool YAML](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:20).

K-in citations

ADR-0083 product vocabulary rule: [0083](/private/tmp/dailyos-v147-w1a/.docs/decisions/0083-product-vocabulary.md:48). ADR-0128 tool descriptions as product copy: [0128](/private/tmp/dailyos-v147-w1a/.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:47), catalog/eval: [0128](/private/tmp/dailyos-v147-w1a/.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:200). DOS-478 conventions and entries read: [YAML](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:3), [account_status](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:16), [daily_briefing](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:54).
tokens used
129,015
VERDICT: NEEDS-CHANGES

Findings

- HIGH: `runtime.block_on(...)` inside sync `invoke()` will be reached from async MCP `call_tool`; Claude may see a panic/hung tool call instead of output. Mitigation: make the gateway/handler path async, or move dispatch onto a proven blocking boundary with a dedicated runtime. Cite: [plan §3.1](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:57), [transport call_tool](/private/tmp/dailyos-v147-w1a/src-tauri/src/services/mcp_v2/transport.rs:190).

- MED: E2E acceptance proves `tools/list` and only `account_status` call; `daily_briefing` is one of two shipped tools but not in AC-8/AC-9 manual smoke. User asking “what’s on my plate today?” remains unproven. Mitigation: update AC-8/AC-9 to call both tools and assert useful JSON output. Cite: [AC-8](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203), [AC-9](/private/tmp/dailyos-v147-w1a/.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205).

- LOW: `account_status.when_to_call` says “recent signals,” leaking system vocabulary into model-facing copy. Mitigation: use “recent updates/changes.” Cite: [tool YAML](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:20).

K-in citations

ADR-0083 product vocabulary rule: [0083](/private/tmp/dailyos-v147-w1a/.docs/decisions/0083-product-vocabulary.md:48). ADR-0128 tool descriptions as product copy: [0128](/private/tmp/dailyos-v147-w1a/.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:47), catalog/eval: [0128](/private/tmp/dailyos-v147-w1a/.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:200). DOS-478 conventions and entries read: [YAML](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:3), [account_status](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:16), [daily_briefing](/private/tmp/dailyos-v147-w1a/src-tauri/resources/mcp_v2/tool_descriptions.yaml:54).
