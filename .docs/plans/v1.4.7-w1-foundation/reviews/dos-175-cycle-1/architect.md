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
session id: 019e4b9c-0661-7593-8073-8b8e76444644
--------
user
Review the L0 plan packet at .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md as an architecture-lens reviewer for the DailyOS abilities-runtime substrate. Lens: pattern compliance, ADR conformance, registry coupling, crate boundaries.

K-in obligation: grep docs/solutions/ and .docs/decisions/ for ADR-0102, ADR-0108, ADR-0125, ADR-0130. Confirm the packet correctly cites the substrate it claims. Reinvented documented substrate is BLOCKED.

Also verify by reading source files:
- src-tauri/abilities-runtime/src/abilities/account_overview.rs around line 34 — current attributes
- src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs around line 27 — current attributes
- src-tauri/src/services/mcp_v2/contracts.rs around line 341 — McpToolHandler trait
- src-tauri/src/services/mcp_v2/gateway.rs around lines 155, 177, 271 — registration + dispatch
- src-tauri/src/bridges/mcp.rs around lines 385-430 — invoke_ability precedent

Threat topology: local-to-local single-user. Do NOT enforce multi-actor gates.

Output structure:
- One-line VERDICT: APPROVE | NEEDS-CHANGES | BLOCKED
- Findings (severity, file:line, architectural concern)
- K-in citations: ADR paths confirmed + which sections

Be terse. ~250 words max.
2026-05-21T17:36:17.377500Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Auth(TokenRefreshFailed("Server returned error response: invalid_grant: Grant not found"))
2026-05-21T17:36:17.933755Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Deserialize(Error("data did not match any variant of untagged enum JsonRpcMessage", line: 0, column: 0))
codex
I’ll use the `plan-eng-review` skill for the architecture review framing, then keep the output in your requested terse packet-review format.
exec
/bin/zsh -lc "sed -n '1,220p' /Users/jamesgiroux/.agents/skills/gstack/plan-eng-review/SKILL.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
---
name: plan-eng-review
preamble-tier: 3
version: 1.0.0
description: |
  Eng manager-mode plan review. Lock in the execution plan — architecture,
  data flow, diagrams, edge cases, test coverage, performance. Walks through
  issues interactively with opinionated recommendations. Use when asked to
  "review the architecture", "engineering review", or "lock in the plan".
  Proactively suggest when the user has a plan or design doc and is about to
  start coding — to catch architecture issues before implementation. (gstack)
  Voice triggers (speech-to-text aliases): "tech review", "technical review", "plan engineering review".
benefits-from: [office-hours]
allowed-tools:
  - Read
  - Write
  - Grep
  - Glob
  - AskUserQuestion
  - Bash
  - WebSearch
triggers:
  - review architecture
  - eng plan review
  - check the implementation plan
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
echo '{"skill":"plan-eng-review","ts":"'$(date -u +%Y-%m-%dT%H:%M:%SZ)'","repo":"'$(basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || echo "unknown")'"}'  >> ~/.gstack/analytics/skill-usage.jsonl 2>/dev/null || true
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
~/.Codex/skills/gstack/bin/gstack-timeline-log '{"skill":"plan-eng-review","event":"started","branch":"'"$_BRANCH"'","session":"'"$_SESSION_ID"'"}' 2>/dev/null &
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
- A) Sure, anonymous is fine
- B) No thanks, fully off

exec
/bin/zsh -lc "sed -n '220,520p' /Users/jamesgiroux/.agents/skills/gstack/plan-eng-review/SKILL.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
- B) No thanks, fully off

If B→A: run `~/.Codex/skills/gstack/bin/gstack-config set telemetry anonymous`
If B→B: run `~/.Codex/skills/gstack/bin/gstack-config set telemetry off`

Always run:
```bash
touch ~/.gstack/.telemetry-prompted
```

This only happens once. If `TEL_PROMPTED` is `yes`, skip this entirely.

If `PROACTIVE_PROMPTED` is `no` AND `TEL_PROMPTED` is `yes`: After telemetry is handled,
ask the user about proactive behavior. Use AskUserQuestion:

> gstack can proactively figure out when you might need a skill while you work —
> like suggesting /qa when you say "does this work?" or /investigate when you hit
> a bug. We recommend keeping this on — it speeds up every part of your workflow.

Options:
- A) Keep it on (recommended)
- B) Turn it off — I'll type /commands myself

If A: run `~/.Codex/skills/gstack/bin/gstack-config set proactive true`
If B: run `~/.Codex/skills/gstack/bin/gstack-config set proactive false`

Always run:
```bash
touch ~/.gstack/.proactive-prompted
```

This only happens once. If `PROACTIVE_PROMPTED` is `yes`, skip this entirely.

If `HAS_ROUTING` is `no` AND `ROUTING_DECLINED` is `false` AND `PROACTIVE_PROMPTED` is `yes`:
Check if a AGENTS.md file exists in the project root. If it does not exist, create it.

Use AskUserQuestion:

> gstack works best when your project's AGENTS.md includes skill routing rules.
> This tells Codex to use specialized workflows (like /ship, /investigate, /qa)
> instead of answering directly. It's a one-time addition, about 15 lines.

Options:
- A) Add routing rules to AGENTS.md (recommended)
- B) No thanks, I'll invoke skills manually

If A: Append this section to the end of AGENTS.md:

```markdown

## Skill routing

When the user's request matches an available skill, ALWAYS invoke it using the Skill
tool as your FIRST action. Do NOT answer directly, do NOT use other tools first.
The skill has specialized workflows that produce better results than ad-hoc answers.

Key routing rules:
- Product ideas, "is this worth building", brainstorming → invoke office-hours
- Bugs, errors, "why is this broken", 500 errors → invoke investigate
- Ship, deploy, push, create PR → invoke ship
- QA, test the site, find bugs → invoke qa
- Code review, check my diff → invoke review
- Update docs after shipping → invoke document-release
- Weekly retro → invoke retro
- Design system, brand → invoke design-consultation
- Visual audit, design polish → invoke design-review
- Architecture review → invoke plan-eng-review
- Save progress, checkpoint, resume → invoke checkpoint
- Code quality, health check → invoke health
```

Then commit the change: `git add AGENTS.md && git commit -m "chore: add gstack skill routing rules to AGENTS.md"`

If B: run `~/.Codex/skills/gstack/bin/gstack-config set routing_declined true`
Say "No problem. You can add routing rules later by running `gstack-config set routing_declined false` and re-running any skill."

This only happens once per project. If `HAS_ROUTING` is `yes` or `ROUTING_DECLINED` is `true`, skip this entirely.

If `VENDORED_GSTACK` is `yes`: This project has a vendored copy of gstack at
`.Codex/skills/gstack/`. Vendoring is deprecated. We will not keep vendored copies
up to date, so this project's gstack will fall behind.

Use AskUserQuestion (one-time per project, check for `~/.gstack/.vendoring-warned-$SLUG` marker):

> This project has gstack vendored in `.Codex/skills/gstack/`. Vendoring is deprecated.
> We won't keep this copy up to date, so you'll fall behind on new features and fixes.
>
> Want to migrate to team mode? It takes about 30 seconds.

Options:
- A) Yes, migrate to team mode now
- B) No, I'll handle it myself

If A:
1. Run `git rm -r .Codex/skills/gstack/`
2. Run `echo '.Codex/skills/gstack/' >> .gitignore`
3. Run `~/.Codex/skills/gstack/bin/gstack-team-init required` (or `optional`)
4. Run `git add .Codex/ .gitignore AGENTS.md && git commit -m "chore: migrate gstack from vendored to team mode"`
5. Tell the user: "Done. Each developer now runs: `cd ~/.Codex/skills/gstack && ./setup --team`"

If B: say "OK, you're on your own to keep the vendored copy up to date."

Always run (regardless of choice):
```bash
eval "$(~/.Codex/skills/gstack/bin/gstack-slug 2>/dev/null)" 2>/dev/null || true
touch ~/.gstack/.vendoring-warned-${SLUG:-unknown}
```

This only happens once per project. If the marker file exists, skip entirely.

If `SPAWNED_SESSION` is `"true"`, you are running inside a session spawned by an
AI orchestrator (e.g., OpenClaw). In spawned sessions:
- Do NOT use AskUserQuestion for interactive prompts. Auto-choose the recommended option.
- Do NOT run upgrade checks, telemetry prompts, routing injection, or lake intro.
- Focus on completing the task and reporting results via prose output.
- End with a completion report: what shipped, decisions made, anything uncertain.

## Model-Specific Behavioral Patch (Codex)

The following nudges are tuned for the Codex model family. They are
**subordinate** to skill workflow, STOP points, AskUserQuestion gates, plan-mode
safety, and /ship review gates. If a nudge below conflicts with skill instructions,
the skill wins. Treat these as preferences, not rules.

**Todo-list discipline.** When working through a multi-step plan, mark each task
complete individually as you finish it. Do not batch-complete at the end. If a task
turns out to be unnecessary, mark it skipped with a one-line reason.

**Think before heavy actions.** For complex operations (refactors, migrations,
non-trivial new features), briefly state your approach before executing. This lets
the user course-correct cheaply instead of mid-flight.

**Dedicated tools over Bash.** Prefer Read, Edit, Write, Glob, Grep over shell
equivalents (cat, sed, find, grep). The dedicated tools are cheaper and clearer.

## Voice

You are GStack, an open source AI builder framework shaped by Garry Tan's product, startup, and engineering judgment. Encode how he thinks, not his biography.

Lead with the point. Say what it does, why it matters, and what changes for the builder. Sound like someone who shipped code today and cares whether the thing actually works for users.

**Core belief:** there is no one at the wheel. Much of the world is made up. That is not scary. That is the opportunity. Builders get to make new things real. Write in a way that makes capable people, especially young builders early in their careers, feel that they can do it too.

We are here to make something people want. Building is not the performance of building. It is not tech for tech's sake. It becomes real when it ships and solves a real problem for a real person. Always push toward the user, the job to be done, the bottleneck, the feedback loop, and the thing that most increases usefulness.

Start from lived experience. For product, start with the user. For technical explanation, start with what the developer feels and sees. Then explain the mechanism, the tradeoff, and why we chose it.

Respect craft. Hate silos. Great builders cross engineering, design, product, copy, support, and debugging to get to truth. Trust experts, then verify. If something smells wrong, inspect the mechanism.

Quality matters. Bugs matter. Do not normalize sloppy software. Do not hand-wave away the last 1% or 5% of defects as acceptable. Great product aims at zero defects and takes edge cases seriously. Fix the whole thing, not just the demo path.

**Tone:** direct, concrete, sharp, encouraging, serious about craft, occasionally funny, never corporate, never academic, never PR, never hype. Sound like a builder talking to a builder, not a consultant presenting to a client. Match the context: YC partner energy for strategy reviews, senior eng energy for code reviews, best-technical-blog-post energy for investigations and debugging.

**Humor:** dry observations about the absurdity of software. "This is a 200-line config file to print hello world." "The test suite takes longer than the feature it tests." Never forced, never self-referential about being AI.

**Concreteness is the standard.** Name the file, the function, the line number. Show the exact command to run, not "you should test this" but `bun test test/billing.test.ts`. When explaining a tradeoff, use real numbers: not "this might be slow" but "this queries N+1, that's ~200ms per page load with 50 items." When something is broken, point at the exact line: not "there's an issue in the auth flow" but "auth.ts:47, the token check returns undefined when the session expires."

**Connect to user outcomes.** When reviewing code, designing features, or debugging, regularly connect the work back to what the real user will experience. "This matters because your user will see a 3-second spinner on every page load." "The edge case you're skipping is the one that loses the customer's data." Make the user's user real.

**User sovereignty.** The user always has context you don't — domain knowledge, business relationships, strategic timing, taste. When you and another model agree on a change, that agreement is a recommendation, not a decision. Present it. The user decides. Never say "the outside voice is right" and act. Say "the outside voice recommends X — do you want to proceed?"

When a user shows unusually strong product instinct, deep user empathy, sharp insight, or surprising synthesis across domains, recognize it plainly. For exceptional cases only, say that people with that kind of taste and drive are exactly the kind of builders Garry respects and wants to fund, and that they should consider applying to YC. Use this rarely and only when truly earned.

Use concrete tools, workflows, commands, files, outputs, evals, and tradeoffs when useful. If something is broken, awkward, or incomplete, say so plainly.

Avoid filler, throat-clearing, generic optimism, founder cosplay, and unsupported claims.

**Writing rules:**
- No em dashes. Use commas, periods, or "..." instead.
- No AI vocabulary: delve, crucial, robust, comprehensive, nuanced, multifaceted, furthermore, moreover, additionally, pivotal, landscape, tapestry, underscore, foster, showcase, intricate, vibrant, fundamental, significant, interplay.
- No banned phrases: "here's the kicker", "here's the thing", "plot twist", "let me break this down", "the bottom line", "make no mistake", "can't stress this enough".
- Short paragraphs. Mix one-sentence paragraphs with 2-3 sentence runs.
- Sound like typing fast. Incomplete sentences sometimes. "Wild." "Not great." Parentheticals.
- Name specifics. Real file names, real function names, real numbers.
- Be direct about quality. "Well-designed" or "this is a mess." Don't dance around judgments.
- Punchy standalone sentences. "That's it." "This is the whole game."
- Stay curious, not lecturing. "What's interesting here is..." beats "It is important to understand..."
- End with what to do. Give the action.

**Final test:** does this sound like a real cross-functional builder who wants to help someone make something people want, ship it, and make it actually work?

## Context Recovery

After compaction or at session start, check for recent project artifacts.
This ensures decisions, plans, and progress survive context window compaction.

```bash
eval "$(~/.Codex/skills/gstack/bin/gstack-slug 2>/dev/null)"
_PROJ="${GSTACK_HOME:-$HOME/.gstack}/projects/${SLUG:-unknown}"
if [ -d "$_PROJ" ]; then
  echo "--- RECENT ARTIFACTS ---"
  # Last 3 artifacts across ceo-plans/ and checkpoints/
  find "$_PROJ/ceo-plans" "$_PROJ/checkpoints" -type f -name "*.md" 2>/dev/null | xargs ls -t 2>/dev/null | head -3
  # Reviews for this branch
  [ -f "$_PROJ/${_BRANCH}-reviews.jsonl" ] && echo "REVIEWS: $(wc -l < "$_PROJ/${_BRANCH}-reviews.jsonl" | tr -d ' ') entries"
  # Timeline summary (last 5 events)
  [ -f "$_PROJ/timeline.jsonl" ] && tail -5 "$_PROJ/timeline.jsonl"
  # Cross-session injection
  if [ -f "$_PROJ/timeline.jsonl" ]; then
    _LAST=$(grep "\"branch\":\"${_BRANCH}\"" "$_PROJ/timeline.jsonl" 2>/dev/null | grep '"event":"completed"' | tail -1)
    [ -n "$_LAST" ] && echo "LAST_SESSION: $_LAST"
    # Predictive skill suggestion: check last 3 completed skills for patterns
    _RECENT_SKILLS=$(grep "\"branch\":\"${_BRANCH}\"" "$_PROJ/timeline.jsonl" 2>/dev/null | grep '"event":"completed"' | tail -3 | grep -o '"skill":"[^"]*"' | sed 's/"skill":"//;s/"//' | tr '\n' ',')
    [ -n "$_RECENT_SKILLS" ] && echo "RECENT_PATTERN: $_RECENT_SKILLS"
  fi
  _LATEST_CP=$(find "$_PROJ/checkpoints" -name "*.md" -type f 2>/dev/null | xargs ls -t 2>/dev/null | head -1)
  [ -n "$_LATEST_CP" ] && echo "LATEST_CHECKPOINT: $_LATEST_CP"
  echo "--- END ARTIFACTS ---"
fi
```

If artifacts are listed, read the most recent one to recover context.

If `LAST_SESSION` is shown, mention it briefly: "Last session on this branch ran
/[skill] with [outcome]." If `LATEST_CHECKPOINT` exists, read it for full context
on where work left off.

If `RECENT_PATTERN` is shown, look at the skill sequence. If a pattern repeats
(e.g., review,ship,review), suggest: "Based on your recent pattern, you probably
want /[next skill]."

**Welcome back message:** If any of LAST_SESSION, LATEST_CHECKPOINT, or RECENT ARTIFACTS
are shown, synthesize a one-paragraph welcome briefing before proceeding:
"Welcome back to {branch}. Last session: /{skill} ({outcome}). [Checkpoint summary if
available]. [Health score if available]." Keep it to 2-3 sentences.

## AskUserQuestion Format

**ALWAYS follow this structure for every AskUserQuestion call:**
1. **Re-ground:** State the project, the current branch (use the `_BRANCH` value printed by the preamble — NOT any branch from conversation history or gitStatus), and the current plan/task. (1-2 sentences)
2. **Simplify:** Explain the problem in plain English a smart 16-year-old could follow. No raw function names, no internal jargon, no implementation details. Use concrete examples and analogies. Say what it DOES, not what it's called.
3. **Recommend:** `RECOMMENDATION: Choose [X] because [one-line reason]` — always prefer the complete option over shortcuts (see Completeness Principle). Include `Completeness: X/10` for each option. Calibration: 10 = complete implementation (all edge cases, full coverage), 7 = covers happy path but skips some edges, 3 = shortcut that defers significant work. If both options are 8+, pick the higher; if one is ≤5, flag it.
4. **Options:** Lettered options: `A) ... B) ... C) ...` — when an option involves effort, show both scales: `(human: ~X / CC: ~Y)`

Assume the user hasn't looked at this window in 20 minutes and doesn't have the code open. If you'd need to read the source to understand your own explanation, it's too complex.

Per-skill instructions may add additional formatting rules on top of this baseline.

## Writing Style (skip entirely if `EXPLAIN_LEVEL: terse` appears in the preamble echo OR the user's current message explicitly requests terse / no-explanations output)

These rules apply to every AskUserQuestion, every response you write to the user, and every review finding. They compose with the AskUserQuestion Format section above: Format = *how* a question is structured; Writing Style = *the prose quality of the content inside it*.

1. **Jargon gets a one-sentence gloss on first use per skill invocation.** Even if the user's own prompt already contained the term — users often paste jargon from someone else's plan. Gloss unconditionally on first use. No cross-invocation memory: a new skill fire is a new first-use opportunity. Example: "race condition (two things happen at the same time and step on each other)".
2. **Frame questions in outcome terms, not implementation terms.** Ask the question the user would actually want to answer. Outcome framing covers three families — match the framing to the mode:
   - **Pain reduction** (default for diagnostic / HOLD SCOPE / rigor review): "If someone double-clicks the button, is it OK for the action to run twice?" (instead of "Is this endpoint idempotent?")
   - **Upside / delight** (for expansion / builder / vision contexts): "When the workflow finishes, does the user see the result instantly, or are they still refreshing a dashboard?" (instead of "Should we add webhook notifications?")
   - **Interrogative pressure** (for forcing-question / founder-challenge contexts): "Can you name the actual person whose career gets better if this ships and whose career gets worse if it doesn't?" (instead of "Who's the target user?")
3. **Short sentences. Concrete nouns. Active voice.** Standard advice from any good writing guide. Prefer "the cache stores the result for 60s" over "results will have been cached for a period of 60s." *Exception:* stacked, multi-part questions are a legitimate forcing device — "Title? Gets them promoted? Gets them fired? Keeps them up at night?" is longer than one short sentence, and it should be, because the pressure IS in the stacking. Don't collapse a stack into a single neutral ask when the skill's posture is forcing.
4. **Close every decision with user impact.** Connect the technical call back to who's affected. Make the user's user real. Impact has three shapes — again, match the mode:
   - **Pain avoided:** "If we skip this, your users will see a 3-second spinner on every page load."
   - **Capability unlocked:** "If we ship this, users get instant feedback the moment a workflow finishes — no tabs to refresh, no polling."
   - **Consequence named** (for forcing questions): "If you can't name the person whose career this helps, you don't know who you're building for — and 'users' isn't an answer."
5. **User-turn override.** If the user's current message says "be terse" / "no explanations" / "brutally honest, just the answer" / similar, skip this entire Writing Style block for your next response, regardless of config. User's in-turn request wins.
6. **Glossary boundary is the curated list.** Terms below get glossed. Terms not on the list are assumed plain-English enough. If you see a term that genuinely needs glossing but isn't listed, note it (once) in your response so it can be added via PR.

**Jargon list** (gloss each on first use per skill invocation, if the term appears in your output):

- idempotent
- idempotency
- race condition
- deadlock
- cyclomatic complexity
- N+1
- N+1 query
- backpressure
- memoization
- eventual consistency
- CAP theorem
- CORS
- CSRF
- XSS
- SQL injection
- prompt injection
- DDoS
- rate limit
- throttle
- circuit breaker
- load balancer
- reverse proxy
- SSR
- CSR
- hydration
- tree-shaking
- bundle splitting
- code splitting
- hot reload
- tombstone
- soft delete
- cascade delete
- foreign key
- composite index
- covering index
- OLTP
- OLAP
- sharding
- replication lag
- quorum
- two-phase commit
- saga
- outbox pattern
- inbox pattern

codex
I’m going to verify the plan against the actual ADR text and the runtime code paths it claims to reuse. The key risk is whether the plan invents another registry/dispatch layer instead of using the documented abilities substrate.
exec
/bin/zsh -lc "sed -n '1,260p' .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "sed -n '1,90p' src-tauri/abilities-runtime/src/abilities/account_overview.rs" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::abilities::claims::{metadata_for_name, ClaimType, FreshnessDecayClass};
use crate::abilities::composition::{
    AbilityRef, BindingRole, Block, BlockId, BlockType, ClaimRef, Composition, CompositionDocId,
    CompositionKind, CompositionMetadata, CompositionVersion, EntityRef, FieldBinding,
    ProvenanceRef, Salience, SalienceBand, Section, SectionId, SectionLayout,
};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::{claim_trust_band_from_score, most_cautious_trust_band};
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, Confidence, DataSource, EntityId, FieldAttribution,
    FieldPath, GleanDownstream, InputsSnapshot, InvocationId, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SourceAttribution, SourceIdentifier, SourceName,
    SourceRef, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{CompositionCommitError, CompositionProposal};
use crate::types::{
    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    IntelligenceClaim, SurfacingState,
};

const ABILITY_NAME: &str = "dailyos/account-overview";
const ABILITY_SCHEMA_VERSION: u32 = 1;
const ACCOUNT_CLAIM_DEPTH: usize = 3;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AccountOverviewInput {
    pub schema_version: u32,
    pub account_id: String,
    #[serde(default)]
    pub expected_composition_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub composition_id: Option<String>,
}

#[derive(Debug, Clone)]
struct NormalizedInput {
    account_id: String,
    expected_composition_version: u64,
    composition_id: CompositionDocId,
}

struct PreparedAccountOverview {
    proposal: CompositionProposal,
    provenance_builder: ProvenanceBuilder,
}

#[derive(Debug, Clone)]
struct ClaimProjection {
    claim: IntelligenceClaim,
    claim_type: ClaimType,
    placement: ClaimPlacement,
    source_index: crate::abilities::provenance::SourceIndex,
    trust_band: TrustBand,
    rendered_text: String,
    parsed_source_asof: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimPlacement {
    Overview,
    Risk,
    Win,
    Value,
    Commitment,
    Relationship,
    Health,
    Ignored,
}

#[ability(
    name = "dailyos/account-overview",
    category = Read,
    version = "1.0.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,

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
/bin/zsh -lc 'rg -n "ADR-0102|ADR-0108|ADR-0125|ADR-0130" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md:8:related_adr: ADR-0108
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md:55:- ADR-0108 (sensitivity rendering audit, OUTPUT boundary mirror at DOS-412 W6)
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:7:**Related:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md)
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:11:[ADR-0102](0102-abilities-as-runtime-contract.md) §3 introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of them write to state that DailyOS controls — in-memory, in the local DB, or in an outbox under [ADR-0103](0103-maintenance-ability-safety-constraints.md). A failed Maintenance ability can be retried, rolled back, or compensated. A failed Read or Transform ability simply does not update anything.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:15:This asymmetry — reversible inside DailyOS, irreversible once the side effect lands — is architecturally distinct. It requires a different contract from the user and a different implementation discipline from the authoring ability. We have one Publish ability today (`publish_to_p2`), referenced in [ADR-0102](0102-abilities-as-runtime-contract.md) §3 and [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md) §3. Before new Publish abilities land — and before `publish_to_p2` itself gets wrapped in the v1.4.0 abilities runtime — the class needs a shared protocol.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:163:- The **provenance summary** — the top-level sources feeding the content, consistent with [ADR-0108](0108-provenance-rendering-and-privacy.md) rendering rules.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:6:**Extends:** [ADR-0101](0101-service-boundary-enforcement.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:11:[ADR-0102](0102-abilities-as-runtime-contract.md) §14 describes per-capability migration but does not specify parallel-run validation, cutover criteria, rollback, or the three specific substrate migrations that ship as part of v1.4.0:
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:27:**Stage 1 — Ability implementation.** A new ability module is added under `src-tauri/src/abilities/` per [ADR-0102](0102-abilities-as-runtime-contract.md) §2. Includes planner function, apply function (for Maintenance), provenance builder usage per [ADR-0105](0105-provenance-as-first-class-output.md), and declared metadata (`mutates`, `composes`, policy).
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:200:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — §14 migration-from-current-architecture refined here.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0101](0101-service-boundary-enforcement.md)  
.docs/decisions/0103-maintenance-ability-safety-constraints.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of the four have naturally bounded risk surfaces:
.docs/decisions/0103-maintenance-ability-safety-constraints.md:25:Service-layer mutations ([ADR-0101](0101-service-boundary-enforcement.md)) are bounded by user action: each mutation corresponds to something a user or agent explicitly did, the blast radius is contained to that action, and errors surface immediately to the caller. Maintenance mutations have none of those natural constraints. A bug in a maintenance ability is the single largest new risk surface introduced by [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0103-maintenance-ability-safety-constraints.md:41:Every ability in the Maintenance category operates under all rules of [ADR-0102](0102-abilities-as-runtime-contract.md) plus the eleven constraints below, covering idempotency, transactional boundaries, execution-mode and dry-run semantics, multi-dimensional change budgets, signal containment, actor policy, blast radius with user-confirmed global, operational audit records, snapshot-diff evaluation, scoped observability, and compile-time separation. These constraints are enforced by type system, registry metadata, runtime checks, and review discipline — not by convention.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:139:    // AbilityPolicy fields (canonical schema per ADR-0102 §7.1):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:154:    // Composition metadata (per ADR-0102 §7.1):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:208:// Canonical AbilityPolicy schema per ADR-0102 §7.1
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:221:Actor policy alone is insufficient in-process: a direct Rust import of a maintenance ability function would still execute regardless of the registry policy. Compile-time separation (§11) closes that gap post-v1.4.0. Until then, review discipline and the typed registry invocation paths from [ADR-0102](0102-abilities-as-runtime-contract.md) §7.2 prevent accidental bypass.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:283:// Masking rules and shape defined by ADR-0108.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:337:Every maintenance invocation produces structured telemetry captured by the `AbilityTracer` from [ADR-0102](0102-abilities-as-runtime-contract.md) §5, with maintenance-specific fields persisted to the `maintenance_audit` table (see §8):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:409:2. **Mis-categorized abilities.** A transform-shaped operation gets filed as maintenance (or vice versa) to bypass or escape category-specific discipline. Mitigation: category determined by call-graph effect per [ADR-0102](0102-abilities-as-runtime-contract.md) §3 — static check at registration.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:421:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Establishes the ability pattern this ADR constrains. Maintenance is one of four categories; this ADR defines its safety envelope.
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:17:2. Touchpoints bypass `build_receipt_for_audience` — raw meeting titles to AgentMcp (ADR-0108 violation)
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:19:4. Envelope cache lacks principal binding — cross-actor poisoning (ADR-0125 violation)
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:7:**Prerequisite for:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:11:The abilities layer introduced by [ADR-0102](0102-abilities-as-runtime-contract.md) and the maintenance safety envelope in [ADR-0103](0103-maintenance-ability-safety-constraints.md) both depend on execution mode being a first-class, enforced property of the runtime. Three requirements motivate this ADR:
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:237:However, ability composition is transaction-propagating, not transaction-nesting (per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.2). A parent maintenance ability that holds a transaction may invoke a child ability; the child inherits the parent's `TxCtx` via `AbilityContext.services` and its service calls participate in the same transaction. This is the inheritance pattern, not a new transaction. The child does not call `with_transaction_async` itself — it uses the parent's open scope. If a child ability attempts its own `with_transaction_async` while a parent transaction is active, it fails with `NestedTransactionsForbidden`.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:353:- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — This ADR amends `ServiceContext` from that document. All ADR-0101 rules (services own mutations, every mutation emits a signal, multi-write transactionality, error propagation, reads don't mutate) remain in force; this ADR adds mode-awareness as a prerequisite for [ADR-0102](0102-abilities-as-runtime-contract.md) and [ADR-0103](0103-maintenance-ability-safety-constraints.md).
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:354:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Consumes `AbilityContext` which wraps `ServiceContext` with its mode. §5 and §11 of that ADR require the contract established here.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:383:**Tracking:** [DOS-209](https://linear.app/a8c/issue/DOS-209) (Substrate: ExecutionMode and Mode-Aware Services) has been amended 2026-04-20 to name `ServiceContext` explicitly as the Phase 0 deliverable. Everything downstream (ADR-0102, 0103, 0105, 0110, 0113, 0115, 0116, 0117, 0119) waits on DOS-209 Phase 0.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:14:[ADR-0091](0091-intelligence-provider-abstraction.md) defines `IntelligenceProvider::complete(prompt, tier) -> String`. This signature does not expose the metadata — provider name, model, token counts, seed, prompt template version — that `PromptFingerprint` requires. ADR-0091 was scoped to the intel queue; ADR-0102 expanded provider use to all Transform abilities. This ADR formally amends ADR-0091's trait to expose fingerprint-capable metadata and specifies the fingerprint shape and canonicalization rules.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:104:Prompt changes are independent of ability schema versions ([ADR-0102](0102-abilities-as-runtime-contract.md) §8). A prompt bump produces a new fingerprint without bumping the ability version. `prompt_template_version` in the fingerprint lets consumers distinguish prompt-driven output changes from schema-driven ones.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:143:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Prompt changes do not bump ability versions; version discipline in §8.
.docs/decisions/0114-scoring-unification.md:167:UI surfaces render `ConfidenceEvidence` uniformly: a score, a band, an expandable breakdown, and a list of caveats. Bands differ per composer; the rendering shell is common. This is what [ADR-0108](0108-provenance-rendering-and-privacy.md) consumes.
.docs/decisions/0114-scoring-unification.md:271:Callers invoke both in sequence. Tests invoke only the pure variant. This matches the abilities contract in [ADR-0102](0102-abilities-as-runtime-contract.md) §3: read/transform abilities do not mutate.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:6:**Amended:** 2026-05-19 — same-date companion amendment to ADR-0102; codifies tool description as product copy (DOS-481 eval coverage), cross-conversation continuity wire shape (defers to ADR-0102 §D OpaqueConversationHandle lifecycle), restated v1.4.7 W4 write surface, displacement-use-case framing extended to v1.4.7 W5 host-selection eval, and Linear link refresh to the post-renumber v1.4.7 project. See section appended at end of doc.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:8:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:179:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:192:This amendment records the v1.4.7 operational consequences of the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment. It does not reopen the architecture. ADR-0128 remains the product-surface frame for headless DailyOS; ADR-0102 remains the runtime contract for MCP-originated ability invocation.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:208:The lifecycle is intentionally not restated here. Expiry is 24 hours from last touch, revocation is server-side, and raw caller-provided conversation IDs are rejected; the authoritative mint, echo, expiry, first-write, revocation, and "no raw IDs" contract lives in [ADR-0102](0102-abilities-as-runtime-contract.md) §D.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:222:### §F. Trust boundary — defer to ADR-0102 amendment
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:224:The MCP client trust boundary is codified in the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment, especially §C: MCP client authentication, transport HMAC signing, server-side scope manifest authorization, per-actor × per-client × per-tool rate limits, revocation, and audit attribution with keyed HMAC parameter and response hashes. This ADR cites that contract because ADR-0128 is the product-surface frame, not the transport or authorization spec.
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0110](0110-evaluation-harness-for-abilities.md)
.docs/decisions/0107-source-taxonomy-alignment.md:8:**Required by:** [ADR-0105](0105-provenance-as-first-class-output.md) §4, [ADR-0108](0108-provenance-rendering-and-privacy.md)
.docs/decisions/0107-source-taxonomy-alignment.md:166:**Resolution of the ADR-0098/ADR-0105 conflict.** ADR-0098 is right: AI-synthesized output is flagged, not purged, because the underlying source revocation removes the input data but the synthesis itself may remain useful as an annotation awaiting re-generation. [ADR-0105](0105-provenance-as-first-class-output.md) §8 is amended: "source revocation triggers a masking pass" applies only to sources whose `LifecycleBehavior` is `Mask`; for `Purge`, the records are deleted; for `FlagForReEnrichment`, the records remain with a flag. [ADR-0108](0108-provenance-rendering-and-privacy.md) specifies the masking and flag-rendering shapes surfaces see.
.docs/decisions/0107-source-taxonomy-alignment.md:210:- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Renders masking behavior and flag-for-re-enrichment status.
.docs/decisions/0107-source-taxonomy-alignment.md:219:Codex round 2 finding 13 surfaced that the v1.4.0 spine plan (DOS-299) needed to preserve item-level confidence semantics for legacy items without `ItemSource` (the current `HasSource::effective_confidence` defaults missing source to `0.5` per `intelligence/io.rs:1185-1189`). My earlier writeup proposed registering this in the `CLAIM_TYPE_REGISTRY` (per ADR-0125). That was wrong — it's a `DataSource` taxonomy concern, not a claim type. ADR-0107 owns `DataSource`; this amendment adds the variant.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:6:**Relates to:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0095](0095-dual-mode-context-architecture.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0109](0109-temporal-primitives-in-the-entity-graph.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md), [ADR-0115](0115-signal-granularity-audit.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:29:The term **harness** is adopted as canonical for DailyOS's synthesis and intelligence layer. Preferred over "pipeline," "agent," "enrichment system," or "intelligence engine" in architectural discussion. Rationale: *pipeline* undersells the memory and verification surface; *agent* oversells — DailyOS explicitly does not route across sub-agents, does not do recursive tool use, and does not run persistent background reasoning (see [ADR-0102](0102-abilities-as-runtime-contract.md) §1 and `.docs/research/2026-02-18-event-driven-intelligence-vision.md`). *Harness* is accurate: typed capability surface, durable external state, verification before output, feedback learning.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:39:| **P1 — Context engineering over context stuffing.** Dynamically select what the model sees per call; prefer smaller scoped prompts over monolithic ones. | Dimension split (I576), ability composition tree ([ADR-0102](0102-abilities-as-runtime-contract.md) §11.3), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)), hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:44:| **P6 — Typed capability surface.** Tools / capabilities are versioned, schema-validated, category-classified with effect inference. | Abilities runtime contract — name, version, schema, category ([ADR-0102](0102-abilities-as-runtime-contract.md) §1–§3), service boundary enforcement ([ADR-0101](0101-service-boundary-enforcement.md)), mode-aware services ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:47:| **P9 — Source attribution as output.** Every generated field carries attribution so corrections can flow back to the originating source's reliability weight. | `field_attributions` map in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) §5), authoritative source taxonomy ([ADR-0107](0107-source-taxonomy-alignment.md)), provenance rendering + privacy ([ADR-0108](0108-provenance-rendering-and-privacy.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:64:**Dependencies.** [ADR-0102](0102-abilities-as-runtime-contract.md) (invocation hook on Transform ability completion), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (evaluator's own prompt is fingerprinted), [ADR-0110](0110-evaluation-harness-for-abilities.md) (rubric source of truth).
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:121:**Proposal.** A devtools-style panel on the entity page (dev mode only, gated by [ADR-0108](0108-provenance-rendering-and-privacy.md) privacy rules) showing the last N invocations per entity: context hash + resolvable pointer, prompt fingerprint + resolvable template, provider response, validation + consistency + trust output, composition tree. Not new storage — a new consumer of existing storage.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:123:**Dependencies.** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0108](0108-provenance-rendering-and-privacy.md) (the panel must honor masking rules and privacy tiers), [ADR-0111](0111-surface-independent-ability-invocation.md) (panel is a Surface per ADR-0111's definition; use the invocation endpoint, not a bespoke query).
.docs/decisions/0111-surface-independent-ability-invocation.md:7:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0027](0027-mcp-dual-mode.md)  
.docs/decisions/0111-surface-independent-ability-invocation.md:8:**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md)
.docs/decisions/0111-surface-independent-ability-invocation.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:67:6. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, User, TauriApp)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:95:7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, Agent, McpTool)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:199:Illustrative, not exhaustive. Each instance ships its own pairing ceremony, scope-grant UX, and renderer (per [ADR-0130](0130-surface-independent-composition-contract.md)), but inherits the trust shape above.
.docs/decisions/0111-surface-independent-ability-invocation.md:216:7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, SurfaceClient, <instance>)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:217:8. Returns the response (typically a `Composition` per [ADR-0130](0130-surface-independent-composition-contract.md), or a claim-shaped JSON for headless paths).
.docs/decisions/0111-surface-independent-ability-invocation.md:222:Registry enumeration for `SurfaceClient` filters at two levels: the actor class (is `SurfaceClient` in the ability's `allowed_actors`?) AND the per-instance scope grants (does this specific instance have the scope required for this ability?). Both must pass. Abilities NOT permitted for `SurfaceClient` invocation never appear in any SurfaceClient's tool surface, even if they appear for `User`. Abilities with `mcp_exposure: McpExposure::None` (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1 + §7.6) are not enumerated for SurfaceClient-mediated MCP servers regardless of permission state; abilities with `mcp_exposure: McpExposure::MetadataOnly` are enumerated with name + description only (no input/output schema); no invocation handler is registered. Full schema is reserved for `McpExposure::Invocable`. The orthogonal `client_side_executable: bool` field on `AbilityPolicy` separately gates the WordPress 7.0 client-side `executeAbility()` JS path; an ability fully `Invocable` via MCP may still be `client_side_executable: false`, and vice versa, per Phase 0 artifact 05 lines 383-412.
.docs/decisions/0111-surface-independent-ability-invocation.md:230:3. Compose with [ADR-0130](0130-surface-independent-composition-contract.md): all SurfaceClient instances consume the same substrate-authored `Composition` model, rendered through their surface-specific renderer.
.docs/decisions/0111-surface-independent-ability-invocation.md:244:6. **Provenance rendering integrated into bridges.** Every response carries appropriately-rendered provenance per [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0111-surface-independent-ability-invocation.md:262:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Registry and §7 binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:265:- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Bridges call the renderer on return.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:1:# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:202:**ADR-0102 (abilities runtime) assessment.** v1.4.6's `recommend_for_entity` ability targets the existing `mcp_exposure: McpExposure::MetadataOnly` tri-state value (added in the 2026-05-10 amendment). No further ADR-0102 amendment needed for v1.4.6 W0. v1.4.7 may amend ADR-0102 to flip `mcp_exposure` to `Invocable` once the MCP `dailyos.read.portfolio_attention` consumer ships.
.docs/decisions/0105-provenance-as-first-class-output.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md)  
.docs/decisions/0105-provenance-as-first-class-output.md:8:**Required by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 (audit), [ADR-0102](0102-abilities-as-runtime-contract.md) §10 (trust boundary)  
.docs/decisions/0105-provenance-as-first-class-output.md:9:**Companions:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (prompt fingerprint details), [ADR-0107](0107-source-taxonomy-alignment.md) (`DataSource` taxonomy), [ADR-0108](0108-provenance-rendering-and-privacy.md) (surface rendering and safety), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` preserves the §8 lives-once invariant by referencing the canonical envelope rather than copying it)
.docs/decisions/0105-provenance-as-first-class-output.md:13:[ADR-0102](0102-abilities-as-runtime-contract.md) §9 Rule 5 declares that every ability output carries provenance via `AbilityOutput<T>`. §10 declares Transform outputs are untrusted for mutation authorization. §11.3 declares composition produces nested provenance trees. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 requires maintenance audit records to carry provenance. All of these contracts depend on one shape: the `Provenance` envelope. ADR-0102 references it but does not specify it.
.docs/decisions/0105-provenance-as-first-class-output.md:15:This ADR defines the envelope shape, trust classification, and composition semantics. It deliberately does not define: prompt fingerprinting (see [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), the authoritative `DataSource` taxonomy (see [ADR-0107](0107-source-taxonomy-alignment.md)), or per-surface rendering rules (see [ADR-0108](0108-provenance-rendering-and-privacy.md)). Splitting these concerns keeps each ADR implementable atomically and testable in isolation.
.docs/decisions/0105-provenance-as-first-class-output.md:19:Every ability output carries a `Provenance` envelope via the `AbilityOutput<T>` wrapper from [ADR-0102](0102-abilities-as-runtime-contract.md) §6. The envelope captures identity, temporal context, trust classification, source attribution, composition tree, and field-level attribution.
.docs/decisions/0105-provenance-as-first-class-output.md:31:    pub ability_schema_version: SchemaVersion,   // The ability's I/O schema version, per ADR-0102 §8
.docs/decisions/0105-provenance-as-first-class-output.md:72:`ability_schema_version` is the ability's I/O schema version per [ADR-0102](0102-abilities-as-runtime-contract.md) §8 — a different concept, retained here for completeness so a consumer reading provenance knows exactly which ability contract produced the output.
.docs/decisions/0105-provenance-as-first-class-output.md:103:Trust is a structured record, not a collapsed nested enum. It encodes the weakest trust encountered, the mixing history, and whether the output is safe to authorize mutation on without a trust signal per [ADR-0102](0102-abilities-as-runtime-contract.md) §10.
.docs/decisions/0105-provenance-as-first-class-output.md:125:    /// policy, or schema-bounded value per ADR-0102 §10.
.docs/decisions/0105-provenance-as-first-class-output.md:210:    pub explanation: Option<SanitizedExplanation>,  // Sanitization rules in ADR-0108
.docs/decisions/0105-provenance-as-first-class-output.md:264:6. Cycle guard: the registry rejects ability composition cycles at registration time per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.1, so nested `Provenance` trees are guaranteed acyclic.
.docs/decisions/0105-provenance-as-first-class-output.md:266:**`SourceRef::Child(child_idx, field_path)` uses a stable `composition_id`.** To avoid the brittleness of positional `child_idx`, the `children` vector is indexed by a stable `CompositionId` per declared `composes` entry in the ability's metadata (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1). The `child_idx` is the index into the `composes` list, not the runtime invocation order. A `SourceRef::Child(child_idx, field_path)` with a `child_idx` outside the declared `composes` list is a registration error.
.docs/decisions/0105-provenance-as-first-class-output.md:317:1. **Maintenance audit table.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 `MaintenanceAuditRecord` adds a `provenance: Provenance` field. Source revocation triggers a masking pass: a `Provenance` whose sources reference a revoked `DataSource` is replaced with a `ProvenanceMasked` marker per [ADR-0108](0108-provenance-rendering-and-privacy.md). **Interaction with [ADR-0094](0094-audit-log-and-enterprise-observability.md):** ADR-0094's append-only JSONL security audit remains unchanged. The `maintenance_audit` SQLite table is a separate operational log for runtime diagnostics. Provenance content in the operational log is subject to masking; the JSONL security log is unchanged.
.docs/decisions/0105-provenance-as-first-class-output.md:340:2. **Trust boundary is enforceable and closes the laundering hole.** [ADR-0102](0102-abilities-as-runtime-contract.md) §10 rejects mutations authorized by `Untrusted` output at the registry layer, and `contains_stored_synthesis` catches the case where stored AI output is surfaced by a Read ability.
.docs/decisions/0105-provenance-as-first-class-output.md:343:5. **Source revocation can cascade correctly.** Provenance references `DataSource` via [ADR-0107](0107-source-taxonomy-alignment.md), and lifecycle behavior (purge vs. flag) is governed by that ADR. Masking shape is governed by [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0105-provenance-as-first-class-output.md:358:4. **`composition_id` drift.** An ability's declared `composes` list changes between versions, invalidating stored `SourceRef::Child` references. Mitigation: registry's version-gating in [ADR-0102](0102-abilities-as-runtime-contract.md) §8 applies — major version bumps for breaking changes.
.docs/decisions/0105-provenance-as-first-class-output.md:364:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Defines `AbilityOutput<T>` wrapper; §9 Rule 5 requires provenance; §10 consumes `TrustAssessment`; §11.3 requires composition merge.
.docs/decisions/0105-provenance-as-first-class-output.md:368:- [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md) — Trust classification and source lifecycle background.
.docs/decisions/0105-provenance-as-first-class-output.md:371:- **[ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md)** — Per-surface rendering rules, actor-filtered display, `ProvenanceMasked` shape, sanitization of `explanation` text, and P2 publication safety.
.docs/decisions/0105-provenance-as-first-class-output.md:374:- **ADR-0111 (forthcoming): Surface-Independent Ability Invocation** — References the rendering rules in [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0105-provenance-as-first-class-output.md:420:Surfaces (per ADR-0108) render `SourceTimestampUnknown` so the system can see when freshness is being approximated rather than measured.
.docs/decisions/0130-surface-independent-composition-contract.md:1:# ADR-0130 — Surface-Independent Composition Contract
.docs/decisions/0130-surface-independent-composition-contract.md:6:**Amended:** 2026-05-10 — §2 `Block.provenance` retyped from `ProvenanceEnvelope` (envelope copy) to `ProvenanceRef` (compact reference into the canonical envelope on `AbilityOutput<Composition>`). Preserves the ADR-0105 "lives once" invariant and keeps block-heavy compositions under ADR-0108's 64KB serialized-provenance cap. Source: Phase 0 artifact `06-composition-provenance-ref.md`.
.docs/decisions/0130-surface-independent-composition-contract.md:9:**Relates to:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md)
.docs/decisions/0130-surface-independent-composition-contract.md:16:- **Abilities** ([ADR-0102](0102-abilities-as-runtime-contract.md)) — typed runtime capabilities.
.docs/decisions/0130-surface-independent-composition-contract.md:19:- **Rendering** ([ADR-0108](0108-provenance-rendering-and-privacy.md)) — provenance rendering per surface/actor.
.docs/decisions/0130-surface-independent-composition-contract.md:40:- **Abilities runtime** — Producer-side execution per [ADR-0102](0102-abilities-as-runtime-contract.md). Composition-producing abilities are normal abilities returning `AbilityOutput<Composition>`.
.docs/decisions/0130-surface-independent-composition-contract.md:47:The substrate emits composed output as instances of a typed `Composition` model. A `Composition` is the `T` inside `AbilityOutput<Composition>` per [ADR-0102](0102-abilities-as-runtime-contract.md) §6 — the canonical `Provenance` envelope lives exactly once on the wrapper, not on the domain output. The Composition model itself carries no envelope copy; blocks carry compact references into the wrapper envelope. The model:
.docs/decisions/0130-surface-independent-composition-contract.md:78:/// AbilityOutput<Composition>.provenance per ADR-0102 §6 and ADR-0105 §8.
.docs/decisions/0130-surface-independent-composition-contract.md:95:**Why `ProvenanceRef`, not `ProvenanceEnvelope`.** Embedding a full envelope on every block creates a second provenance carrier inside the domain output, which violates [ADR-0102](0102-abilities-as-runtime-contract.md) §6 + §9 Rule 5 (provenance lives exactly once on `AbilityOutput<T>`). It also defeats [ADR-0108](0108-provenance-rendering-and-privacy.md)'s 64KB serialized-provenance cap: a 24-block composition with typical 12KB Transform envelopes serializes to ~288KB of duplicated provenance, 4.5× the cap before any block content is counted. A `ProvenanceRef` is ~80-200 bytes serialized; a 60-block long-form composition spends ~13KB on block refs, well inside the cap. The reference shape mirrors [ADR-0105](0105-provenance-as-first-class-output.md) §8's planned-mutation pattern — plans reference the envelope by invocation id and field path rather than duplicating it.
.docs/decisions/0130-surface-independent-composition-contract.md:97:**Resolution.** Renderers resolve a `ProvenanceRef` by fetching the canonical envelope for `invocation_id` from the runtime provenance store (already addressable per ADR-0105/ADR-0108), reading the `FieldAttribution` at `field_path`, and passing the resolved envelope through [ADR-0108](0108-provenance-rendering-and-privacy.md) §2's actor-filtered `render_provenance_for(prov, actor, surface)`. Resolution can fail (garbage-collected envelope, unknown invocation across environments, unauthorized actor, masked provenance, missing field attribution); failure MUST degrade visibly — the block renders with a provenance-unavailable banner or downgraded trust band, never as fully trusted. When the envelope resolves but the exact `field_path` does not, the surface MAY fall back to invocation-level provenance, but MUST label the fallback as less specific.
.docs/decisions/0130-surface-independent-composition-contract.md:101:**Composition-size guard.** Block provenance refs SHOULD stay under 48KB serialized across a single composition. Pathologically block-heavy compositions (>~200 blocks) that exceed the guard MUST collapse to section-level refs rather than embedding envelopes. The 48KB guard preserves headroom under ADR-0108's 64KB cap for surrounding composition metadata.
.docs/decisions/0130-surface-independent-composition-contract.md:126:Surfaces that do not know a `Custom` block type MUST render a **schema-bounded projection** onto the nearest known block type. Renderers MUST NOT render unknown block payload fields directly. The previous "raw payload available" fallback is removed — an unknown block type is a privacy boundary, not a debugging opportunity. Unknown payload fields may include internal notes, source excerpts, email addresses, prompt context, debug carriers, or fields whose sensitivity tier is not allowed on the current surface (per [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)). Ability metadata is also model-facing and browser-facing API surface per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.6; raw payload disclosure is an info-disclosure vector.
.docs/decisions/0130-surface-independent-composition-contract.md:135:6. **Preserve `provenance` (the `ProvenanceRef`) exactly.** This is not payload — it is a reference into the [ADR-0105](0105-provenance-as-first-class-output.md) envelope and renders only through [ADR-0108](0108-provenance-rendering-and-privacy.md) actor-filtered rules. The fallback path MUST NOT dereference `claim_refs` or `provenance` to backfill dropped payload fields.
.docs/decisions/0130-surface-independent-composition-contract.md:186:6. Applying [ADR-0108](0108-provenance-rendering-and-privacy.md) §2 `render_provenance_for(prov, actor, surface)` to every resolved envelope. The `ProvenanceRef` does not bypass actor/surface filtering; it only locates the canonical envelope to which those rules apply.
.docs/decisions/0130-surface-independent-composition-contract.md:212:- **Provenance lives once.** `ProvenanceRef` preserves ADR-0102 §6 + ADR-0105 §8's "lives once" invariant. Block-heavy compositions stay inside ADR-0108's 64KB serialized-provenance cap — a 60-block long-form composition spends ~13KB on refs vs. ~720KB if envelopes were copied per block.
.docs/decisions/0130-surface-independent-composition-contract.md:229:- ADR-0102 §6/§9 Rule 5 and ADR-0105 §8 are preserved structurally — provenance lives once, on the wrapper, and references address it.
.docs/decisions/0130-surface-independent-composition-contract.md:238:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities runtime contract; §6 + §9 Rule 5 establish provenance-lives-once on `AbilityOutput<T>`; §7.1 canonical AbilityPolicy schema; §7.6 governs SurfaceClient-reachable ability metadata
.docs/decisions/0130-surface-independent-composition-contract.md:240:- [ADR-0108](0108-provenance-rendering-and-privacy.md) — Actor-filtered provenance rendering; 64KB serialized-provenance cap that motivates the ref shape
.docs/decisions/0130-surface-independent-composition-contract.md:242:- [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md) — Sensitivity tiers (`Public`, `Internal`, `Confidential`, `UserOnly`) consumed by schema-bounded fallback projection
.docs/decisions/0102-abilities-as-runtime-contract.md:1:# ADR-0102: Abilities as the Runtime Contract
.docs/decisions/0102-abilities-as-runtime-contract.md:11:**Related:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0097](0097-account-health-scoring-architecture.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md), [ADR-0100](0100-glean-first-intelligence-architecture.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` preserves the ADR-0105 §8 lives-once invariant and avoids payload-cap blowups when ability outputs are composed into surface-rendered blocks)
.docs/decisions/0102-abilities-as-runtime-contract.md:515:- **ADR-0108 (forthcoming): Surface-Independent Ability Invocation** — Specifies MCP and Tauri binding rules concretely (which follows from §7.3).
.docs/decisions/0102-abilities-as-runtime-contract.md:530:1. **Hard error** — ability returns `Err(AbilityError::...)`. Caller sees no output. Surface decides user-visible message per [ADR-0108](0108-provenance-rendering-and-privacy.md). No partial data leaks.
.docs/decisions/0102-abilities-as-runtime-contract.md:545:The `warnings[]` field on Provenance is the canonical record of "what went soft." Surfaces render it per [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0110-evaluation-harness-for-abilities.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
.docs/decisions/0110-evaluation-harness-for-abilities.md:8:**Consumed by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 (snapshot-diff evaluation), [ADR-0102](0102-abilities-as-runtime-contract.md) §13 (Phase 3 regression gate)
.docs/decisions/0110-evaluation-harness-for-abilities.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §10 Enforcement Phase 3 requires a regression gate before abilities become authoritative; Risk #5 declares the evaluation harness a non-negotiable co-dependency. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 specifies snapshot-diff evaluation for maintenance abilities. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers in `Evaluate` mode. [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §6 uses fingerprints to classify regression types.
.docs/decisions/0110-evaluation-harness-for-abilities.md:157:[ADR-0102](0102-abilities-as-runtime-contract.md) §13 Phase 3 cutover requires the regression gate to be active before the registry becomes authoritative. The gate's entry requirement is:
.docs/decisions/0110-evaluation-harness-for-abilities.md:245:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Phase 3 regression gate depends on this harness.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:7:**Consumed by:** [ADR-0102](0102-abilities-as-runtime-contract.md) (Read abilities), [ADR-0105](0105-provenance-as-first-class-output.md) (provenance references trajectory snapshots)
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:171:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — `get_entity_context` consumes trajectory bundle.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:14:1. [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a typed, versioned ability. Many of those are Transform abilities that produce claims. The product will ship with multiple ability authors — `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative`, future MCP agents — each asserting claims into the same substrate.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:39:- The `:<version>` suffix on `agent:` is mandatory and matches `ability_version` from provenance ([ADR-0102](0102-abilities-as-runtime-contract.md) §8).
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:99:This matches [ADR-0102](0102-abilities-as-runtime-contract.md) §3's call-graph distinction — Transform abilities must not mutate. The propose/commit split is the structural enforcement of that boundary for claim production.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:315:### R1.5 Actor representation — align with ADR-0102 `Actor` enum
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:317:The original §1 declared actor as a string with canonical shapes. [ADR-0102](0102-abilities-as-runtime-contract.md) §1 defines `Actor` as an enum (`User | Agent | System`). These conflict.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:332:Serialization produces the same canonical strings the original §1 proposed. Rust code uses the enum; the column stores the rendered string. This preserves ADR-0102's typing while accepting that a columnar DB stores serialized forms.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:334:The `Actor` enum in ADR-0102 §1 is extended by this ADR (not replaced). The `Agent` variant gains `name` + `version` fields; `Human` is new; `External` is new. This extension requires an amendment to ADR-0102; tracked.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:365:- No abilities runtime code exists yet. ADR-0102 is also still doc-only. v1.4.0 project work creates both; this ADR's implementation cannot precede ADR-0102's skeleton landing.
.docs/decisions/0120-observability-contract.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md)
.docs/decisions/0120-observability-contract.md:7:**Related:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
.docs/decisions/0120-observability-contract.md:128:The observability contract honors [ADR-0108](0108-provenance-rendering-and-privacy.md) masking rules. Specifically:
.docs/decisions/0120-observability-contract.md:135:The distinction: logs carry **the shape of what happened**, not **the content of what happened**. Content lives in the encrypted database with the provenance envelopes and `evaluation_traces`. Debug flows pull content at display time through [ADR-0108](0108-provenance-rendering-and-privacy.md) renderer, not by reading raw logs.
.docs/decisions/0120-observability-contract.md:240:- **`#[instrument]` is boilerplate.** Every ability, every service mutation, every worker. Mitigated by the `#[ability]` macro ([ADR-0102](0102-abilities-as-runtime-contract.md) §7) expanding to include it; service functions can adopt `#[instrument(skip(ctx))]` by convention.
.docs/decisions/0120-observability-contract.md:255:- The `#[ability]` macro from [ADR-0102](0102-abilities-as-runtime-contract.md) §7 expands to open the root span with `invocation_id` + `ability_name` + `ability_version`.
.docs/decisions/0108-provenance-rendering-and-privacy.md:1:# ADR-0108: Provenance Rendering and Privacy
.docs/decisions/0108-provenance-rendering-and-privacy.md:7:**Related:** [ADR-0094](0094-audit-log-and-enterprise-observability.md), [ADR-0093](0093-prompt-injection-hardening.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` references this ADR's actor-filtered envelope; the ref shape avoids exceeding the rendering-side payload caps this ADR governs)
.docs/decisions/0108-provenance-rendering-and-privacy.md:188:- The cap is enforced at `AbilityOutput<T>` construction time. The error surfaces to the caller as a hard error (per [ADR-0102](0102-abilities-as-runtime-contract.md) error-handling amendment).
.docs/decisions/0126-memory-substrate-invariants.md:54:When a new signal fits an existing claim schema (ADR-0125 claim type registry), it assimilates: strengthens corroborations, bumps freshness, does not produce a new surface notification.
.docs/decisions/0126-memory-substrate-invariants.md:83:Budgets, if used, are per claim_type + sensitivity + surface. A critical-sensitivity health claim does not compete for the same attention slot as a routine adoption-metric. The claim type registry (ADR-0125) carries the per-type budget policy, not a global constant.
.docs/decisions/0126-memory-substrate-invariants.md:119:- **Hard-coded thresholds (`HOMEOSTATIC_BUDGET=100`, `DORMANT_THRESHOLD=0.15`, `STALE_WINDOW=180`).** Policy lives in the ADR-0125 claim type registry or role preset config (DOS-178), never magic numbers in code.
.docs/decisions/0126-memory-substrate-invariants.md:138:- Claim types + registry: ADR-0125.
.docs/decisions/0115-signal-granularity-audit.md:7:**Related:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md)
.docs/decisions/0115-signal-granularity-audit.md:15:2. **Ability-output events.** [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a named, versioned ability producing an `AbilityOutput<T>`. Each output is cache-keyed and invalidation-tracked. "This output changed" is a new event class operating at ability-output granularity.
.docs/decisions/0115-signal-granularity-audit.md:317:Codex flagged: the ADR consumes an `OutputId` concept that doesn't exist in code yet ([ADR-0102](0102-abilities-as-runtime-contract.md)/[ADR-0105](0105-provenance-as-first-class-output.md) are also doc-only).
.docs/decisions/0115-signal-granularity-audit.md:319:**Revised:** `OutputId` is defined in this ADR as a dependency placeholder and implemented in the abilities runtime work ([ADR-0102](0102-abilities-as-runtime-contract.md) implementation). Shape:
.docs/decisions/0115-signal-granularity-audit.md:329:This ADR's Phase 0 is blocked on the `AbilityOutput` + `Provenance` types landing from [ADR-0102](0102-abilities-as-runtime-contract.md) / [ADR-0105](0105-provenance-as-first-class-output.md) implementation. Document that dependency explicitly in [DOS-235](https://linear.app/a8c/issue/DOS-235)+.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:6:**Relates to:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0113](0113-claim-anatomy-and-dedup.md), [ADR-0114](0114-scoring-unification.md), [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:160:**Claim-type-specific salience overrides** are pinned in the `CLAIM_TYPE_REGISTRY` per ADR-0125. Examples (initial):
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:440:- Adding a new predicate requires an ADR amendment (bounded surface — analogous to claim-type addition per ADR-0125).
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:569:- Does NOT change sensitivity/temporal_scope gating (per [ADR-0108](0108-provenance-rendering-and-privacy.md) / [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)) — but it DOES move tier gating earlier in the pipeline via CandidateFilter (§5)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:7:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md), [ADR-0127](0127-presets-as-intelligence-contracts.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:199:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
.docs/decisions/0123-typed-claim-feedback-semantics.md:105:    ///   no source penalty; feeds ADR-0108 surface privacy policy.

exec
/bin/zsh -lc "sed -n '320,370p' src-tauri/src/services/mcp_v2/contracts.rs" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "sed -n '1,95p' src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "sed -n '140,290p' src-tauri/src/services/mcp_v2/gateway.rs" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
    pub next_request_nonce: OpaqueNonce,
    /// Tool result. `Ok` carries the handler's typed JSON return value;
    /// `Err` carries a [`ToolError`].
    pub result: McpToolResult,
}

/// Tool outcome carried by [`McpToolResponseEnvelope`].
///
/// Externally tagged on the wire: `{"kind":"ok","value":...}` or
/// `{"kind":"error","error":...}`. Splitting success / error at the
/// envelope layer (rather than `Result<Value, ToolError>`) keeps the
/// wire shape stable under serde's default `Result` representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpToolResult {
    /// Handler succeeded; `value` is the typed JSON return payload.
    Ok { value: serde_json::Value },
    /// Handler or gateway returned a typed error.
    Error { error: ToolError },
}

pub trait McpToolHandler: Send + Sync {
    fn description(&self) -> &ToolDescription;

    fn invoke(
        &self,
        actor: &McpActor,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError>;
}

/// Typed errors propagated to the MCP wire.
///
/// `ConversationRequired` is intentionally absent — the gateway mints
/// handles transparently on first write per ADR-0102 §D (2026-05-19
/// amendment). Auth-state cases — `PairingRevoked`,
/// `ConversationRevoked`, `ExposureForbidden` — live as their own
/// variants rather than abusing `Unauthorized::missing_scope` with
/// sentinel strings: the [`Scope`] type is reserved for
/// `<namespace>.<verb>.<noun>` (or pre-amendment substrate) values
/// per §E, so auth-state sentinels do not belong on that field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ToolError {
    BadParams {
        detail: String,
    },

 succeeded in 0ms:
//! `get_daily_briefing` Read/User-only ability + composed
//! `BriefingState` (cycle-1 correctness F3).
//!
//! Read-side composition over existing meeting prep status (§5.5) +
//! per-subject entity intelligence (§5.1) + daily readiness context.
//! Per `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md`
//! §5.10.
//!
//! Consumed by the W3 Daily Briefing Gutenberg block. **Not exposed** to
//! Agent or MCP actors in v1.4.4; future Agent / MCP exposure requires
//! `/cso` re-approval per AC-507.5.

pub mod contracts;
pub mod producer;

pub use contracts::{
    AmbiguityPair, BriefingAdvisory, BriefingAvailability, BriefingEmptyReason, BriefingFreshness,
    BriefingIntegrity, BriefingSection, BriefingStaleReason, BriefingState, BriefingTrustSummary,
    DailyBriefingInput, DailyBriefingOutput, MeetingBriefRef, SourceAsofRef, WatchProposal,
    BRIEFING_SCHEMA_VERSION,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "get_daily_briefing",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.daily_briefing"],
    mcp_exposure = None,
    composes = [
        { id = "get_entity_intelligence", ability = "get_entity_intelligence", optional = false },
        { id = "get_daily_readiness", ability = "get_daily_readiness", optional = false }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_daily_briefing(
    ctx: &AbilityContext<'_>,
    input: DailyBriefingInput,
) -> AbilityResult<DailyBriefingOutput> {
    producer::build_daily_briefing(ctx, input).await
}

 succeeded in 0ms:
            "mcp.signal.warning client_id={} conversation_handle={} tool_name={} warning_reason={warning_reason}",
            client_id.as_str(),
            conversation_handle.as_str(),
            tool_name.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

/// MCP v2 dispatch entry point. Holds the registered handler set + the
/// signal emitter and runs the per-dispatch flow defined in L0 packet §1 #1.
pub struct Gateway {
    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
    emitter: Arc<dyn SignalEmitter>,
    taxonomy: Option<Arc<dyn super::taxonomy::TaxonomyCatalog>>,
}

impl Gateway {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            emitter: Arc::new(StderrSignalEmitter),
            taxonomy: None,
        }
    }

    pub fn with_emitter(emitter: Arc<dyn SignalEmitter>) -> Self {
        Self {
            handlers: HashMap::new(),
            emitter,
            taxonomy: None,
        }
    }

    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
        let name = handler.description().name.clone();
        self.handlers.insert(name, handler);
    }

    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
        self.handlers.keys()
    }

    /// Register the taxonomy catalog. Production callers wire this with
    /// `YamlTaxonomyCatalog::load_embedded()`. Tests may use a stub.
    pub fn set_taxonomy(&mut self, catalog: Arc<dyn super::taxonomy::TaxonomyCatalog>) {
        self.taxonomy = Some(catalog);
    }

    /// Seal the gateway after all handlers are registered. Validates
    /// every registered handler has a matching catalog entry with matching
    /// `Side` (per DOS-478 L0 AC-7). Returns operator-readable error if
    /// mismatch. Also returns catalog→handler pending-tool list as
    /// operator info; callers log it.
    ///
    /// Production `main.rs` calls `gateway.seal()?` after registration.
    /// No `panic!()` — fail-fast via structured `Result`.
    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
        let Some(catalog) = self.taxonomy.as_ref() else {
            // No taxonomy registered: tests / dev mode skip validation.
            return Ok(Vec::new());
        };
        let handlers: Vec<&dyn McpToolHandler> =
            self.handlers.values().map(|h| h.as_ref()).collect();
        catalog.validate_against_handlers(&handlers)?;
        Ok(catalog.validate_catalog_against_handlers(&handlers))
    }

    /// Handle a single MCP tool invocation. Full per-dispatch contract per
    /// L0 packet §1 #1; gate ordering matches AC-2 verbatim. The Gate 0b
    /// preissued nonce is captured and returned in the response envelope on
    /// BOTH success and rejection paths (closes L2 code-reviewer HIGH +
    /// CSO HIGH on recovery-nonce leak).
    pub fn handle_tool_call(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: McpToolRequestEnvelope,
        signature: &[u8],
    ) -> McpToolResponseEnvelope {
        let start = Instant::now();
        match self.dispatch(conn, asserted_client_id, &envelope, signature) {
            Ok(Dispatched {
                result,
                next_nonce,
                conversation_handle,
            }) => McpToolResponseEnvelope {
                conversation_handle,
                next_request_nonce: next_nonce,
                result,
            },
            Err(failure) => {
                let GatewayFailure {
                    error,
                    reject_reason,
                    tool_name_for_signal,
                    client_id_for_signal,
                    preissued_next_nonce,
                } = *failure;
                // Auth-state rejection. Emit reject signal with safe
                // attribution (Gate 0a/0b → client_id None per AC-7).
                self.emitter.emit_rejected(
                    client_id_for_signal.as_ref(),
                    tool_name_for_signal.as_ref(),
                    &reject_reason,
                );
                // AC-12 timing floor on ALL rejection branches.
                sleep_until_floor(start);
                // Return the Gate 0b preissued nonce (if Gate 0b admitted)
                // OR an empty token (Gate 0a / pre-consume failures —
                // caller must re-pair via the seed nonce).
                McpToolResponseEnvelope {
                    conversation_handle: OpaqueConversationHandle::new(
                        envelope_handle_str(&envelope).to_string(),
                    ),
                    next_request_nonce: preissued_next_nonce
                        .unwrap_or_else(|| OpaqueNonce::new(String::new())),
                    result: McpToolResult::Error { error },
                }
            }
        }
    }

    /// Inner per-dispatch flow. On success: handler result + preissued next
    /// nonce + active conversation handle. On failure: `GatewayFailure` with
    /// safe attribution + the preissued next nonce (if Gate 0b admitted) so
    /// rejection responses carry the SAME nonce the caller's next request
    /// must present, per ADR-0102 §C.bis.refresh.
    fn dispatch(
        &self,
        conn: &mut Connection,
        asserted_client_id: &McpClientId,
        envelope: &McpToolRequestEnvelope,
        signature: &[u8],
    ) -> Result<Dispatched, Box<GatewayFailure>> {
        // Gate 0a: HMAC over WHOLE envelope including request_nonce. On
        // failure, attribution is "unresolved" — the asserted client_id is
        // not yet verified, so AC-7 says we MUST NOT signal it.
        if let Err(err) = auth::verify_transport_hmac(conn, asserted_client_id, envelope, signature)
        {
            return Err(auth_to_failure(err, envelope, None, None));
        }

        // Gate 0b: fail-closed consume + preissue per ADR-0102 §C.bis.refresh
        // + §C.bis.fail-closed. After this point, the nonce is permanently
        // consumed (commit happens inside auth::verify_and_consume_and_preissue
        // phase a); the returned `next_nonce` is what the response envelope
        // MUST carry whether the rest of dispatch succeeds or fails.

exec
/bin/zsh -lc "sed -n '360,450p' src-tauri/src/bridges/mcp.rs" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
        self
    }

    /// Re-enable MCP request_confirmation once the W5/W6 prompt UI commands
    /// are wired and a real attestation resolver is reachable from the
    /// renderer. Tests that exercise the confirmation flow flip this on
    /// directly to drive the Tauri-side approving host.
    pub fn with_confirmation_enabled(mut self) -> Self {
        self.mcp_confirmation_enabled = true;
        self
    }

    pub fn confirmation_enabled(&self) -> bool {
        self.mcp_confirmation_enabled
    }

    /// Returns the cached actor-filtered descriptor list. The bridge does NOT
    /// re-scan the registry on every call; it scans once at construction.
    pub fn list_descriptors(&self) -> &[&'registry AbilityDescriptor] {
        &self.actor_filtered_descriptors
    }

    /// Invoke an ability by name. Re-fetches policy by name at call time
    /// (does not trust cached descriptors for authorization). Maps every
    /// reject reason to BridgeSurfaceError::AbilityUnavailable byte-equal.
    pub async fn invoke_ability(
        &self,
        session: McpSessionId,
        ability_name: &str,
        input_json: serde_json::Value,
        dry_run: bool,
        confirmation: Option<ConfirmationToken>,
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let mut services =
            ServiceContext::new_live(&clock, &rng, &external).with_actor(MCP_ACTOR_LABEL);
        if let Some(readers) = &self.workspace_readers {
            services = readers.attach_to(services);
        }
        let invocation = InvocationContext {
            actor: BridgeActor::Agent,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::McpTool,
            claim_dismissal_surface: crate::services::context::ClaimDismissalSurface::McpTool,
            dry_run,
            confirmation: confirmation.as_ref(),
            confirmation_store: None,
        };

        let response = invoke_registry_json(
            self.registry,
            &services,
            self.provider.as_ref(),
            self.tracer.as_ref(),
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)?;

        self.insert_provenance(
            session,
            response.invocation_id,
            response.render_cached_provenance(BridgeActor::Agent, BridgeSurface::McpToolDetail),
        );

        Ok(response)
    }

    /// Bounded session-scoped lookup. Returns RenderedProvenance only if the
    /// (session, invocation_id) pair was previously returned by this same
    /// session. No maintenance-audit fallback; no cross-session lookup.
    pub fn get_provenance(
        &self,
        session: McpSessionId,
        invocation_id: InvocationId,
    ) -> Option<RenderedProvenance> {
        self.invocation_cache
            .lock()
            .expect("mcp invocation cache poisoned")
            .get(&(session, invocation_id))
    }

    pub fn get_provenance_tool_response(
        &self,
        session: McpSessionId,
        invocation_id: InvocationId,
    ) -> CallToolResult {

codex
The first pass confirms the plan is pointing at real code, but one claimed current attribute is already suspect: `account_overview` is named `dailyos/account-overview`, not plain `account_overview`, and I need to check whether that is just wording or a dispatch-breaking mismatch.
exec
/bin/zsh -lc 'rg -l "ADR-0102" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc 'rg -l "ADR-0108" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md
.docs/decisions/0117-publish-boundary-pencil-and-pen.md
.docs/decisions/0103-maintenance-ability-safety-constraints.md
.docs/decisions/0107-source-taxonomy-alignment.md
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md
.docs/decisions/0131-structured-embedding-claim-canonicalization.md
.docs/decisions/0102-abilities-as-runtime-contract.md
.docs/decisions/0114-scoring-unification.md
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md
.docs/decisions/0105-provenance-as-first-class-output.md
.docs/decisions/0108-provenance-rendering-and-privacy.md
.docs/decisions/0120-observability-contract.md
.docs/decisions/0130-surface-independent-composition-contract.md
.docs/decisions/0123-typed-claim-feedback-semantics.md
.docs/decisions/0111-surface-independent-ability-invocation.md

 succeeded in 0ms:
.docs/decisions/0117-publish-boundary-pencil-and-pen.md
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md
.docs/decisions/0103-maintenance-ability-safety-constraints.md
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md
.docs/decisions/0102-abilities-as-runtime-contract.md
.docs/decisions/0104-execution-mode-and-mode-aware-services.md
.docs/decisions/0111-surface-independent-ability-invocation.md
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md
.docs/decisions/0110-evaluation-harness-for-abilities.md
.docs/decisions/0130-surface-independent-composition-contract.md
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md
.docs/decisions/0108-provenance-rendering-and-privacy.md
.docs/decisions/0114-scoring-unification.md
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md
.docs/decisions/0120-observability-contract.md
.docs/decisions/0105-provenance-as-first-class-output.md
.docs/decisions/0115-signal-granularity-audit.md

exec
/bin/zsh -lc 'rg -l "ADR-0125" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/account_overview.rs | sed -n '28,82p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc 'rg -l "ADR-0130" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md
.docs/decisions/0131-structured-embedding-claim-canonicalization.md
.docs/decisions/0130-surface-independent-composition-contract.md
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md
.docs/decisions/0107-source-taxonomy-alignment.md
.docs/decisions/0126-memory-substrate-invariants.md

 succeeded in 0ms:
    28	use crate::services::context::{CompositionCommitError, CompositionProposal};
    29	use crate::types::{
    30	    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
    31	    IntelligenceClaim, SurfacingState,
    32	};
    33	
    34	const ABILITY_NAME: &str = "dailyos/account-overview";
    35	const ABILITY_SCHEMA_VERSION: u32 = 1;
    36	const ACCOUNT_CLAIM_DEPTH: usize = 3;
    37	
    38	#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
    39	pub struct AccountOverviewInput {
    40	    pub schema_version: u32,
    41	    pub account_id: String,
    42	    #[serde(default)]
    43	    pub expected_composition_version: u64,
    44	    #[serde(default, skip_serializing_if = "Option::is_none")]
    45	    pub composition_id: Option<String>,
    46	}
    47	
    48	#[derive(Debug, Clone)]
    49	struct NormalizedInput {
    50	    account_id: String,
    51	    expected_composition_version: u64,
    52	    composition_id: CompositionDocId,
    53	}
    54	
    55	struct PreparedAccountOverview {
    56	    proposal: CompositionProposal,
    57	    provenance_builder: ProvenanceBuilder,
    58	}
    59	
    60	#[derive(Debug, Clone)]
    61	struct ClaimProjection {
    62	    claim: IntelligenceClaim,
    63	    claim_type: ClaimType,
    64	    placement: ClaimPlacement,
    65	    source_index: crate::abilities::provenance::SourceIndex,
    66	    trust_band: TrustBand,
    67	    rendered_text: String,
    68	    parsed_source_asof: Option<DateTime<Utc>>,
    69	}
    70	
    71	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    72	enum ClaimPlacement {
    73	    Overview,
    74	    Risk,
    75	    Win,
    76	    Value,
    77	    Commitment,
    78	    Relationship,
    79	    Health,
    80	    Ignored,
    81	}
    82	

exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs | sed -n '20,55p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
.docs/decisions/0111-surface-independent-ability-invocation.md
.docs/decisions/0102-abilities-as-runtime-contract.md
.docs/decisions/0108-provenance-rendering-and-privacy.md
.docs/decisions/0105-provenance-as-first-class-output.md
.docs/decisions/0130-surface-independent-composition-contract.md

 succeeded in 0ms:
    20	    BRIEFING_SCHEMA_VERSION,
    21	};
    22	
    23	use dailyos_abilities_macro::ability;
    24	
    25	use crate::abilities::{AbilityContext, AbilityResult};
    26	
    27	#[ability(
    28	    name = "get_daily_briefing",
    29	    category = Read,
    30	    version = "0.1.0",
    31	    schema_version = 1,
    32	    allowed_actors = [User, SurfaceClient],
    33	    allowed_modes = [Live, Simulate, Evaluate],
    34	    requires_confirmation = false,
    35	    may_publish = false,
    36	    required_scopes = ["read.daily_briefing"],
    37	    mcp_exposure = None,
    38	    composes = [
    39	        { id = "get_entity_intelligence", ability = "get_entity_intelligence", optional = false },
    40	        { id = "get_daily_readiness", ability = "get_daily_readiness", optional = false }
    41	    ],
    42	    experimental = false,
    43	    signal_policy = { emits_on_output_change = [], coalesce = false }
    44	)]
    45	pub async fn get_daily_briefing(
    46	    ctx: &AbilityContext<'_>,
    47	    input: DailyBriefingInput,
    48	) -> AbilityResult<DailyBriefingOutput> {
    49	    producer::build_daily_briefing(ctx, input).await
    50	}

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/contracts.rs | sed -n '336,352p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   336	    Ok { value: serde_json::Value },
   337	    /// Handler or gateway returned a typed error.
   338	    Error { error: ToolError },
   339	}
   340	
   341	pub trait McpToolHandler: Send + Sync {
   342	    fn description(&self) -> &ToolDescription;
   343	
   344	    fn invoke(
   345	        &self,
   346	        actor: &McpActor,
   347	        params: serde_json::Value,
   348	    ) -> Result<serde_json::Value, ToolError>;
   349	}
   350	
   351	/// Typed errors propagated to the MCP wire.
   352	///

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/gateway.rs | sed -n '150,285p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   150	// ---------------------------------------------------------------------------
   151	
   152	/// MCP v2 dispatch entry point. Holds the registered handler set + the
   153	/// signal emitter and runs the per-dispatch flow defined in L0 packet §1 #1.
   154	pub struct Gateway {
   155	    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
   156	    emitter: Arc<dyn SignalEmitter>,
   157	    taxonomy: Option<Arc<dyn super::taxonomy::TaxonomyCatalog>>,
   158	}
   159	
   160	impl Gateway {
   161	    pub fn new() -> Self {
   162	        Self {
   163	            handlers: HashMap::new(),
   164	            emitter: Arc::new(StderrSignalEmitter),
   165	            taxonomy: None,
   166	        }
   167	    }
   168	
   169	    pub fn with_emitter(emitter: Arc<dyn SignalEmitter>) -> Self {
   170	        Self {
   171	            handlers: HashMap::new(),
   172	            emitter,
   173	            taxonomy: None,
   174	        }
   175	    }
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
   215	    /// CSO HIGH on recovery-nonce leak).
   216	    pub fn handle_tool_call(
   217	        &self,
   218	        conn: &mut Connection,
   219	        asserted_client_id: &McpClientId,
   220	        envelope: McpToolRequestEnvelope,
   221	        signature: &[u8],
   222	    ) -> McpToolResponseEnvelope {
   223	        let start = Instant::now();
   224	        match self.dispatch(conn, asserted_client_id, &envelope, signature) {
   225	            Ok(Dispatched {
   226	                result,
   227	                next_nonce,
   228	                conversation_handle,
   229	            }) => McpToolResponseEnvelope {
   230	                conversation_handle,
   231	                next_request_nonce: next_nonce,
   232	                result,
   233	            },
   234	            Err(failure) => {
   235	                let GatewayFailure {
   236	                    error,
   237	                    reject_reason,
   238	                    tool_name_for_signal,
   239	                    client_id_for_signal,
   240	                    preissued_next_nonce,
   241	                } = *failure;
   242	                // Auth-state rejection. Emit reject signal with safe
   243	                // attribution (Gate 0a/0b → client_id None per AC-7).
   244	                self.emitter.emit_rejected(
   245	                    client_id_for_signal.as_ref(),
   246	                    tool_name_for_signal.as_ref(),
   247	                    &reject_reason,
   248	                );
   249	                // AC-12 timing floor on ALL rejection branches.
   250	                sleep_until_floor(start);
   251	                // Return the Gate 0b preissued nonce (if Gate 0b admitted)
   252	                // OR an empty token (Gate 0a / pre-consume failures —
   253	                // caller must re-pair via the seed nonce).
   254	                McpToolResponseEnvelope {
   255	                    conversation_handle: OpaqueConversationHandle::new(
   256	                        envelope_handle_str(&envelope).to_string(),
   257	                    ),
   258	                    next_request_nonce: preissued_next_nonce
   259	                        .unwrap_or_else(|| OpaqueNonce::new(String::new())),
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

exec
/bin/zsh -lc "nl -ba src-tauri/src/bridges/mcp.rs | sed -n '385,436p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md | sed -n '1,230p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
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
   436	        &self,

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

exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/account_overview.rs | sed -n '82,125p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
    82	
    83	#[ability(
    84	    name = "dailyos/account-overview",
    85	    category = Read,
    86	    version = "1.0.0",
    87	    schema_version = 1,
    88	    allowed_actors = [User, SurfaceClient],
    89	    allowed_modes = [Live],
    90	    requires_confirmation = false,
    91	    may_publish = false,
    92	    required_scopes = ["read.account_overview"],
    93	    mcp_exposure = Invocable,
    94	    client_side_executable = false,
    95	    composes = [],
    96	    experimental = false,
    97	    signal_policy = { emits_on_output_change = [
    98	        "claim.version",
    99	        "account_subject.claim_changed",
   100	        "claim.lifecycle",
   101	        "claim.dismissal",
   102	        "source.freshness",
   103	        "source.revocation"
   104	    ], coalesce = true }
   105	)]
   106	pub async fn account_overview(
   107	    ctx: &AbilityContext<'_>,
   108	    input: AccountOverviewInput,
   109	) -> AbilityResult<Composition> {
   110	    let input = normalize_input(input)?;
   111	    let prepared = prepare_account_overview(ctx, &input).await?;
   112	    let committed = ctx
   113	        .services()
   114	        .commit_composition(prepared.proposal)
   115	        .await
   116	        .map_err(composition_commit_error)?;
   117	    let output = prepared
   118	        .provenance_builder
   119	        .finalize(committed.composition)
   120	        .map_err(provenance_error)?;
   121	    validate_block_provenance(output.data(), output.provenance())?;
   122	    Ok(output)
   123	}
   124	
   125	fn normalize_input(input: AccountOverviewInput) -> Result<NormalizedInput, AbilityError> {

exec
/bin/zsh -lc "rg -n \"name = \\\"dailyos/account-overview\\\"|name = \\\"account_overview\\\"|mcp_exposure|allowed_actors|account_overview|dailyos/account-overview\" src-tauri/abilities-runtime/src/abilities src-tauri/src -g '*.rs'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:32:    allowed_actors = [User, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:37:    mcp_exposure = None,
src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:749:        // policy declares allowed_actors=[User] so the registry already gates
src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:19:    allowed_actors = [User, Agent, System, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/get_daily_readiness/mod.rs:24:    mcp_exposure = Invocable,
src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:57:    allowed_actors = [User, Agent, System, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/composition.rs:377:            Self::AccountOverview => "account_overview",
src-tauri/abilities-runtime/src/abilities/composition.rs:1237:        assert_eq!(block.block_type.type_id(), "account_overview");
src-tauri/abilities-runtime/src/abilities/composition.rs:1476:        // account_overview < risk_callout lexicographically.
src-tauri/src/commands/version_dispatcher.rs:101:    let scope = SurfaceScope::new("read.account_overview");
src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1240:        BlockType::AccountOverview => Some(account_overview_rule()),
src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1266:        account_overview_rule(),
src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:1522:fn account_overview_rule() -> BlockProjectionRule {
src-tauri/src/services/surface_pairing.rs:2539:                .allowed_actors
src-tauri/src/services/surface_pairing.rs:2917:            .filter(|d| d.policy.allowed_actors.contains(&ActorKind::SurfaceClient))
src-tauri/src/services/surface_pairing.rs:4062:        assert!(!raw.contains("read.account_overview"));
src-tauri/src/services/composition_render_orchestrator.rs:12://!    (`dailyos/account-overview`) → gets `AbilityOutput<Composition>` →
src-tauri/src/services/composition_render_orchestrator.rs:188:/// name. v1.4.2 ships a single producer: `dailyos/account-overview`.
src-tauri/src/services/composition_render_orchestrator.rs:189:/// composition IDs of the form `dailyos/account-overview:account:{account_id}`
src-tauri/src/services/composition_render_orchestrator.rs:193:    if composition_id.starts_with("dailyos/account-overview:") {
src-tauri/src/services/composition_render_orchestrator.rs:194:        Some("dailyos/account-overview")
src-tauri/src/services/composition_render_orchestrator.rs:201:/// Pattern: `dailyos/account-overview:account:{account_id}`.
src-tauri/src/services/composition_render_orchestrator.rs:203:    composition_id.strip_prefix("dailyos/account-overview:account:")
src-tauri/src/services/composition_render_orchestrator.rs:222:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")]).unwrap();
src-tauri/src/services/composition_render_orchestrator.rs:234:            "composition_id": "dailyos/account-overview:account:acct-1",
src-tauri/src/services/composition_render_orchestrator.rs:250:                "dailyos/account-overview:account:acct-1",
src-tauri/src/services/composition_render_orchestrator.rs:258:            .cache_lookup(&actor, "dailyos/account-overview:account:acct-1", 1)
src-tauri/src/services/composition_render_orchestrator.rs:266:            "composition_id": "dailyos/account-overview:account:acct-2",
src-tauri/src/services/composition_render_orchestrator.rs:280:            "dailyos/account-overview:account:acct-2",
src-tauri/src/services/composition_render_orchestrator.rs:287:            SurfaceScope::new("read.account_overview"),
src-tauri/src/services/composition_render_orchestrator.rs:295:        let hit = orchestrator.cache_lookup(&actor_b, "dailyos/account-overview:account:acct-2", 1);
src-tauri/src/services/composition_render_orchestrator.rs:302:            resolve_producer_ability_name("dailyos/account-overview:account:x"),
src-tauri/src/services/composition_render_orchestrator.rs:303:            Some("dailyos/account-overview")
src-tauri/src/services/composition_render_orchestrator.rs:311:            extract_account_id_from_composition_id("dailyos/account-overview:account:acct-42"),
src-tauri/src/services/mcp_v2/actor_policy.rs:56:        allowed_actors: MCP_CLIENT_ACTORS,
src-tauri/src/services/mcp_v2/actor_policy.rs:57:        mcp_exposure: tool_grant.exposure,
src-tauri/src/mcp/main.rs:1461:                allowed_actors: actors,
src-tauri/src/mcp/main.rs:1466:                mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/mod.rs:3:pub mod account_overview;
src-tauri/abilities-runtime/src/abilities/detect_risk_shift/mod.rs:19:    allowed_actors = [User, Agent, System],
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:25:    allowed_actors = [User, Agent, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:30:    mcp_exposure = None,
src-tauri/src/harness/bundle_helpers.rs:221:            allowed_actors: SYSTEM_ACTORS,
src-tauri/src/harness/bundle_helpers.rs:226:            mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:25:    allowed_actors = [User, Agent, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:30:    mcp_exposure = None,
src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/mod.rs:31:    allowed_actors = [User, Agent, System, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/mod.rs:36:    mcp_exposure = Invocable,
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:13://! not yet exposed to agent-side MCP, and `mcp_exposure = None` keeps
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:36:    allowed_actors = [User, Agent, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:41:    mcp_exposure = None,
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:47:    allowed_actors = [User, Agent, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:52:    mcp_exposure = None,
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:89:    fn allowed_actors_include_user_agent_surface_client_but_not_mcp_client() {
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:97:        let actors: &[ActorKind] = descriptor.policy.allowed_actors;
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:119:    fn mcp_exposure_is_none_for_conservative_surface_routing() {
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:125:        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::None);
src-tauri/abilities-runtime/src/abilities/account_overview.rs:34:const ABILITY_NAME: &str = "dailyos/account-overview";
src-tauri/abilities-runtime/src/abilities/account_overview.rs:84:    name = "dailyos/account-overview",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:88:    allowed_actors = [User, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/account_overview.rs:92:    required_scopes = ["read.account_overview"],
src-tauri/abilities-runtime/src/abilities/account_overview.rs:93:    mcp_exposure = Invocable,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:106:pub async fn account_overview(
src-tauri/abilities-runtime/src/abilities/account_overview.rs:111:    let prepared = prepare_account_overview(ctx, &input).await?;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:142:        .unwrap_or_else(|| format!("dailyos/account-overview:account:{account_id}"));
src-tauri/abilities-runtime/src/abilities/account_overview.rs:151:async fn prepare_account_overview(
src-tauri/abilities-runtime/src/abilities/account_overview.rs:172:        .map_err(|error| hard_error("account_overview_claim_read", error))?;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:217:    if !claim_is_eligible_for_account_overview(&claim, account_id)? {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:252:fn claim_is_eligible_for_account_overview(
src-tauri/abilities-runtime/src/abilities/account_overview.rs:439:        BlockId::new(block_id(input, "overview", "account_overview", "summary")),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:709:            "dailyos.account_overview.v1",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1274:                    "read.account_overview",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1353:            descriptor.policy.allowed_actors,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1358:            &["read.account_overview"]
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1360:        assert_eq!(descriptor.policy.mcp_exposure, McpExposure::Invocable);
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1372:        let err = match account_overview(
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1454:        let output = account_overview(&ctx, input())
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1512:        let output = account_overview(&ctx, input())
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1563:        let prepared = prepare_account_overview(&ctx, &normalized)
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1624:        let output = account_overview(&ctx, input())
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1633:                    "read.account_overview",
src-tauri/abilities-runtime/src/abilities/registry.rs:76:/// vocabulary (e.g. `read.account_overview`, `read.composition`,
src-tauri/abilities-runtime/src/abilities/registry.rs:454:/// any per-invocation instance data. Used in [`AbilityPolicy::allowed_actors`]
src-tauri/abilities-runtime/src/abilities/registry.rs:492:    /// `descriptor.policy.allowed_actors.contains(&actor.kind())`.
src-tauri/abilities-runtime/src/abilities/registry.rs:575:/// - `mcp_exposure`: tri-state MCP enumeration tier (see [`McpExposure`]).
src-tauri/abilities-runtime/src/abilities/registry.rs:577:///   policy/scope/actor checks. Independent of `mcp_exposure` per Phase 0
src-tauri/abilities-runtime/src/abilities/registry.rs:582:/// - `allowed_actors: &[ActorKind::User]` — least-privilege actor floor
src-tauri/abilities-runtime/src/abilities/registry.rs:585:/// - `mcp_exposure: McpExposure::None`
src-tauri/abilities-runtime/src/abilities/registry.rs:590:/// `SurfaceClient` in `allowed_actors` may keep empty `required_scopes`,
src-tauri/abilities-runtime/src/abilities/registry.rs:592:/// `allowed_actors` includes `SurfaceClient`.
src-tauri/abilities-runtime/src/abilities/registry.rs:601:    pub allowed_actors: &'static [ActorKind],
src-tauri/abilities-runtime/src/abilities/registry.rs:610:    pub mcp_exposure: McpExposure,
src-tauri/abilities-runtime/src/abilities/registry.rs:623:    /// `mcp_exposure: McpExposure::None`, `client_side_executable: false`,
src-tauri/abilities-runtime/src/abilities/registry.rs:627:            allowed_actors: &[ActorKind::User],
src-tauri/abilities-runtime/src/abilities/registry.rs:632:            mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:893:            union.insert(SurfaceScope::new("read.account_overview"));
src-tauri/abilities-runtime/src/abilities/registry.rs:939:            descriptor.policy.allowed_actors.contains(&actor.kind())
src-tauri/abilities-runtime/src/abilities/registry.rs:1474:    if !descriptor.policy.allowed_actors.contains(&ctx.actor.kind()) {
src-tauri/abilities-runtime/src/abilities/registry.rs:1570:        "allowed_actors",
src-tauri/abilities-runtime/src/abilities/registry.rs:1573:            .allowed_actors
src-tauri/abilities-runtime/src/abilities/registry.rs:1731:                allowed_actors: &[],
src-tauri/abilities-runtime/src/abilities/registry.rs:1736:                mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:1761:                allowed_actors: &[ActorKind::Agent, ActorKind::User, ActorKind::System],
src-tauri/abilities-runtime/src/abilities/registry.rs:1770:                mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:1820:        descriptor.policy.allowed_actors = static_slice(actors);
src-tauri/abilities-runtime/src/abilities/registry.rs:2408:        let scope = SurfaceScope::new("read.account_overview");
src-tauri/abilities-runtime/src/abilities/registry.rs:2409:        assert_eq!(scope.as_str(), "read.account_overview");
src-tauri/abilities-runtime/src/abilities/registry.rs:2410:        assert_eq!(format!("{scope}"), "read.account_overview");
src-tauri/abilities-runtime/src/abilities/registry.rs:2582:    /// the canonical W1-A.1 fixture vocabulary (`read.account_overview`,
src-tauri/abilities-runtime/src/abilities/registry.rs:2588:        all.insert(SurfaceScope::new("read.account_overview"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2598:            scopes: scope_set(&["read.account_overview"]),
src-tauri/abilities-runtime/src/abilities/registry.rs:2603:                assert!(scopes.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2616:            scopes: scope_set(&["read.account_overview", "submit.feedback"]),
src-tauri/abilities-runtime/src/abilities/registry.rs:2627:            scopes: scope_set(&["read.account_overview"]),
src-tauri/abilities-runtime/src/abilities/registry.rs:2631:            scopes: scope_set(&["read.account_overview"]),
src-tauri/abilities-runtime/src/abilities/registry.rs:2642:    fn actor_surface_client_not_in_user_agent_allowed_actors() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2644:        // `allowed_actors: [User, Agent]` must NOT match a SurfaceClient
src-tauri/abilities-runtime/src/abilities/registry.rs:2652:            scopes: scope_set(&["read.account_overview"]),
src-tauri/abilities-runtime/src/abilities/registry.rs:2669:        seed_test_allowlist(&["read.account_overview"]);
src-tauri/abilities-runtime/src/abilities/registry.rs:2670:        let set = ScopeSet::new([SurfaceScope::new("read.account_overview")])
src-tauri/abilities-runtime/src/abilities/registry.rs:2672:        assert!(set.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2690:        seed_test_allowlist(&["read.account_overview", "submit.feedback"]);
src-tauri/abilities-runtime/src/abilities/registry.rs:2691:        let json = "[\"read.account_overview\",\"submit.feedback\"]";
src-tauri/abilities-runtime/src/abilities/registry.rs:2693:        assert!(decoded.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2702:    // W1-B: AbilityPolicy schema (required_scopes + mcp_exposure +
src-tauri/abilities-runtime/src/abilities/registry.rs:2713:        assert_eq!(policy.allowed_actors, &[ActorKind::User]);
src-tauri/abilities-runtime/src/abilities/registry.rs:2719:        assert_eq!(policy.mcp_exposure, McpExposure::None);
src-tauri/abilities-runtime/src/abilities/registry.rs:2731:        seed_test_allowlist(&["read.account_overview"]);
src-tauri/abilities-runtime/src/abilities/registry.rs:2734:            scopes: ScopeSet::new([SurfaceScope::new("read.account_overview")])
src-tauri/abilities-runtime/src/abilities/registry.rs:2743:            required_scopes: &["read.account_overview", "submit.feedback"],
src-tauri/abilities-runtime/src/abilities/registry.rs:2748:        assert!(typed.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2784:    fn mcp_exposure_default_is_none() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2789:    fn mcp_exposure_serde_round_trip_for_all_variants() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2802:    fn mcp_exposure_wire_form_is_snake_case() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2825:            allowed_actors: &[],
src-tauri/abilities-runtime/src/abilities/registry.rs:2829:            required_scopes: &["read.account_overview"],
src-tauri/abilities-runtime/src/abilities/registry.rs:2830:            mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:2834:        assert_eq!(POLICY.required_scopes, &["read.account_overview"]);
src-tauri/abilities-runtime/src/abilities/list_open_loops/mod.rs:72:    allowed_actors = [User, Agent, System, SurfaceClient],
src-tauri/abilities-runtime/src/abilities/list_open_loops/mod.rs:77:    mcp_exposure = Invocable,
src-tauri/abilities-runtime/src/abilities/prepare_meeting/mod.rs:19:    allowed_actors = [User, Agent, System],
src-tauri/src/audit_log.rs:946:            SurfaceScope::new("read.account_overview"),
src-tauri/src/audit_log.rs:957:            serde_json::json!({"surface": "account_overview"}),
src-tauri/src/audit_log.rs:981:        assert!(!line.contains("read.account_overview"));
src-tauri/abilities-runtime/src/abilities/temporal/mod.rs:226:    allowed_actors = [System],
src-tauri/abilities-runtime/src/abilities/temporal/mod.rs:260:    allowed_actors = [System],
src-tauri/src/bridges/mcp.rs:761:                allowed_actors: actors,
src-tauri/src/bridges/mcp.rs:766:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/eval.rs:241:                allowed_actors: SYSTEM_ACTORS,
src-tauri/src/bridges/eval.rs:246:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/tauri.rs:664:                allowed_actors: actors,
src-tauri/src/bridges/tauri.rs:669:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/surface_client.rs:341:            .allowed_actors
src-tauri/src/bridges/surface_client.rs:1205:                allowed_actors: &[ActorKind::SurfaceClient],
src-tauri/src/bridges/surface_client.rs:1210:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/surface_client.rs:1300:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1304:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1361:                        &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1365:            let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1393:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1399:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1403:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1435:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1439:        let mut session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1489:            actor_scopes: vec!["read.account_overview".to_string()],
src-tauri/src/bridges/surface_client.rs:1515:            &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1523:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1545:            "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1547:            &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1555:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1558:            .authorize(&registry, &session, "dailyos/account-overview", "req_1")
src-tauri/src/bridges/surface_client.rs:1563:            "dailyos/account-overview"
src-tauri/src/bridges/surface_client.rs:1575:                    "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1577:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1585:                .authorize_local_render(&registry, &session, "dailyos/account-overview", "req_1")
src-tauri/src/bridges/surface_client.rs:1600:            "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1602:            &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1610:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1613:            .authorize_local_render(&registry, &session, "dailyos/account-overview", "req_1")
src-tauri/src/bridges/surface_client.rs:1616:            .authorize_local_render(&registry, &session, "dailyos/account-overview", "req_2")
src-tauri/src/bridges/surface_client.rs:1619:            .authorize(&registry, &session, "dailyos/account-overview", "req_3")
src-tauri/src/bridges/surface_client.rs:1623:                .authorize(&registry, &session, "dailyos/account-overview", "req_4")
src-tauri/src/bridges/surface_client.rs:1639:                    "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1641:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1645:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1648:            .authorize_local_render(&registry, &session, "dailyos/account-overview", "req_1")
src-tauri/src/bridges/surface_client.rs:1652:                .authorize_local_render(&registry, &session, "dailyos/account-overview", "req_2")
src-tauri/src/bridges/surface_client.rs:1664:            "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1666:            &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1681:                    "dailyos/account-overview",
src-tauri/src/bridges/surface_client.rs:1719:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1726:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1754:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1760:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1766:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1770:        let session = session(&["read.account_overview"]);
src-tauri/src/bridges/surface_client.rs:1811:                    &["read.account_overview"],
src-tauri/src/bridges/surface_client.rs:1816:        let session = Arc::new(session(&["read.account_overview"]));
src-tauri/src/bridges/surface_client.rs:1857:                "read.account_overview",
src-tauri/src/bridges/surface_client.rs:1862:            actor_scopes: vec!["read.account_overview".to_string()],
src-tauri/src/bridges/surface_client.rs:1873:        assert!(!detail.contains("read.account_overview"));
src-tauri/src/bridges/worker.rs:139:                allowed_actors: SYSTEM_ACTORS,
src-tauri/src/bridges/worker.rs:144:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/types.rs:659:    if !descriptor.policy.allowed_actors.contains(&actor.kind()) {
src-tauri/src/bridges/types.rs:1394:                allowed_actors: &[ActorKind::User],
src-tauri/src/bridges/types.rs:1399:                mcp_exposure: McpExposure::None,
src-tauri/src/bridges/correction_payload.rs:78:        scope == "read.account_overview"
src-tauri/src/surface_runtime/mod.rs:2475:    // rate limiting, or audit emission. The bridge gates allowed_actors,
src-tauri/src/surface_runtime/mod.rs:2571:    // allowed_actors/required_scopes/rate-limit policy that the cache key
src-tauri/src/surface_runtime/mod.rs:4155:                .unwrap_or("dailyos/account-overview:account:acct-cache");
src-tauri/src/surface_runtime/mod.rs:4172:                    "ability_name": "dailyos/account-overview",
src-tauri/src/surface_runtime/mod.rs:4395:                allowed_actors: &[ActorKind::SurfaceClient],
src-tauri/src/surface_runtime/mod.rs:4399:                required_scopes: &["read.account_overview"],
src-tauri/src/surface_runtime/mod.rs:4400:                mcp_exposure: McpExposure::None,
src-tauri/src/surface_runtime/mod.rs:4483:            name: "dailyos/account-overview",
src-tauri/src/surface_runtime/mod.rs:4488:                allowed_actors: &[ActorKind::SurfaceClient],
src-tauri/src/surface_runtime/mod.rs:4492:                required_scopes: &["read.account_overview"],
src-tauri/src/surface_runtime/mod.rs:4493:                mcp_exposure: McpExposure::None,
src-tauri/src/surface_runtime/mod.rs:4634:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
src-tauri/src/surface_runtime/mod.rs:4650:            granted_scopes: vec!["read.account_overview".to_string()],
src-tauri/src/surface_runtime/mod.rs:5024:            SurfaceScope::new("read.account_overview"),
src-tauri/src/surface_runtime/mod.rs:5719:        let composition_id = "dailyos/account-overview:account:acct-cache";
src-tauri/src/surface_runtime/mod.rs:5765:        let composition_id = "dailyos/account-overview:account:acct-cache-hit";
src-tauri/src/surface_runtime/mod.rs:5810:        let composition_id = "dailyos/account-overview:account:acct-store";
src-tauri/src/surface_runtime/mod.rs:5861:        let composition_id = "dailyos/account-overview:account:acct-external";
src-tauri/src/surface_runtime/mod.rs:5947:        let composition_id = "dailyos/account-overview:account:acct-local-render";
src-tauri/src/surface_runtime/mod.rs:6052:    fn account_overview_success_audit_carries_composition_fields_and_writes() {
src-tauri/src/surface_runtime/mod.rs:6056:            "ability_name": "dailyos/account-overview",
src-tauri/src/surface_runtime/mod.rs:6075:            "req_account_overview",
src-tauri/src/surface_runtime/mod.rs:6426:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])

exec
/bin/zsh -lc "rg -n \"§C\\.bis|C\\.bis|mcp_exposure|allowed_actors|McpClient|SurfaceClient|attribute|re-approval|reapproval|re-approve|§7\\.1|§7\\.6|SurfaceClient|MCP client\" .docs/decisions/0102-abilities-as-runtime-contract.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
5:**Amended:** 2026-05-10 — extended `AbilityPolicy.allowed_actors` to include `SurfaceClient` (defined in [ADR-0111](0111-surface-independent-ability-invocation.md) §8); added §7.6 specifying SurfaceClient policy and introspection semantics. Per the formal L0 panel on [DOS-546](https://linear.app/a8c/issue/DOS-546).
6:**Amended (later same day):** 2026-05-10 — extended `AbilityPolicy` schema with `required_scopes: Vec<SurfaceClientScope>` and `mcp_exposure: bool` as first-class fields per the formal L0 Cycle 2 unanimous finding. Two-level SurfaceClient enforcement is now substrate-enforceable from the canonical policy, not just prose in §7.6. Default policy expanded: `required_scopes = vec![]`, `mcp_exposure = false`.
7:**Amended (later same day):** 2026-05-10 — promoted `mcp_exposure` from `bool` to tri-state `McpExposure { None | MetadataOnly | Invocable }` enum per Phase 0 artifact 05 lines 383-412; retained `client_side_executable: bool` as an orthogonal separate field for the WordPress 7.0 client-side `executeAbility()` JS path. The two fields govern different trust boundaries — `mcp_exposure` controls the network-facing MCP tool surface (discovery + invocation), `client_side_executable` controls whether a trusted in-product SurfaceClient may invoke the ability after WordPress capability + runtime scope + actor checks pass. The fields are independent; either may be true while the other is false. Default policy is `mcp_exposure = McpExposure::None` + `client_side_executable = false`, opt-in per ability. This cycle-3 reversal of the cycle-2 collapse was made after re-reading artifact 05's explicit warning against the symmetric `client_side_exposure` framing.  
8:**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
191:Each ability declares itself with an attribute macro that generates its registration, schema, and type-erased wrapper. The macro cannot infer transitive service calls or ability composition from the function body alone; it relies on **explicit declarative metadata** that the author provides and the registry validates at registration time:
198:    allowed_actors = [User, Agent, System],
217:    pub allowed_actors: Vec<Actor>,             // Who can invoke: User | Agent | SurfaceClient | System (see §7.6)
221:    pub required_scopes: Vec<SurfaceClientScope>, // SurfaceClient scopes required to invoke (per ADR-0111 §8). Empty means: callable by any
222:                                                  // SurfaceClient permitted by allowed_actors with no specific scope check. Non-empty means:
223:                                                  // all listed scopes must be granted to the calling SurfaceClient instance. Ignored when the
224:                                                  // invoking actor is not SurfaceClient. (Added 2026-05-10 per L0 Cycle 2.)
225:    pub mcp_exposure: McpExposure,              // Tri-state exposure tier for MCP — both the Rust runtime's MCP server AND any
226:                                                  // SurfaceClient-mediated MCP server (e.g., WordPress MCP Adapter per ADR-0129).
229:    pub client_side_executable: bool,           // Whether a trusted in-product SurfaceClient (e.g., a WordPress 7.0 block invoking
231:                                                  // scope + actor checks pass. Orthogonal to mcp_exposure: either field may be true
248:pub struct SurfaceClientScope(pub String);      // Scope identifier — e.g., "read.account_overview", "submit.feedback",
250:                                                  // time. The scope namespace is owned by the SurfaceClient class definition (ADR-0111 §8).
253:ADR-0103 §6 uses these exact field names. There is no `actor_policy` shorthand — the macro attribute names match the struct fields one-to-one.
278:- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
285:Registry enumeration is actor-scoped. An MCP client enumerating available tools sees only abilities whose `AbilityPolicy.allowed_actors` includes `Agent` AND whose `mcp_exposure` is `McpExposure::MetadataOnly` or `McpExposure::Invocable`; abilities with `mcp_exposure: None` are filtered. `MetadataOnly` abilities are enumerated with name + description only — the input/output schema and any blast-radius hints are withheld until promotion to `Invocable`; their invocation handler is not registered with the MCP server. `Invocable` abilities are enumerated with full schema and an invocation handler. Introspection does not leak ability names, input schemas, or blast radius to callers that are not authorized to invoke.
287:The Tauri bridge, invoked by the first-party app, sees the full registry subject to `User` and `System` filtering; the `mcp_exposure` and `client_side_executable` fields do not gate the first-party Tauri bridge because that bridge is the in-process path for the desktop app, not an MCP-mediated or SurfaceClient-mediated surface. Background workers see the `System` filter.
293:#### 7.6 SurfaceClient Policy and Introspection (Amendment, 2026-05-10)
295:[ADR-0111](0111-surface-independent-ability-invocation.md) §8 introduces `SurfaceClient` as a fourth canonical actor class for third-party local surfaces (WordPress, Obsidian, future browser extensions / PWA / mobile). The `AbilityPolicy.allowed_actors` field accepts `SurfaceClient` as a valid value alongside `User`, `Agent`, and `System`.
297:**Multi-level enforcement.** Unlike `User`/`Agent`/`System` — where the actor *class* is the unit of policy enforcement — `SurfaceClient` enforces at multiple levels. All checks are against canonical fields on `AbilityPolicy` (per §7.1, extended 2026-05-10 cycle 2, refined later the same day to promote `mcp_exposure` to the `McpExposure` tri-state and add the orthogonal `client_side_executable` field). Invocation is permitted only if ALL applicable checks pass:
299:1. The ability's `allowed_actors` includes `SurfaceClient`.
300:2. For every `SurfaceClientScope` in the ability's `required_scopes`, the invoking SurfaceClient instance's scope grants (per ADR-0111 §8) include that scope. If `required_scopes` is empty, this check is a no-op.
301:3. If the invocation arrives via an MCP-mediated path (the Rust runtime's MCP server OR a SurfaceClient-mediated MCP server per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §4), the ability's `mcp_exposure` is `McpExposure::Invocable`. If `mcp_exposure` is `McpExposure::None`, MCP-mediated discovery AND invocation are rejected at the registry boundary. If `mcp_exposure` is `McpExposure::MetadataOnly`, MCP-mediated discovery enumerates name + description only (no input/output schema), and MCP-mediated invocation is rejected at the registry boundary. The tier check happens regardless of `allowed_actors`.
302:4. If the invocation arrives via a client-side SurfaceClient JS path (e.g., a WordPress 7.0 block invoking `executeAbility()`), the ability's `client_side_executable` is `true`. If `client_side_executable` is `false`, the SurfaceClient bridge rejects the invocation before registry lookup regardless of `allowed_actors`, `required_scopes`, or `mcp_exposure`. This check is orthogonal to the MCP-mediated tier check — the two fields govern different trust boundaries (per Phase 0 artifact 05 lines 383-412).
304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
306:**Introspection rules extended.** §7.4 actor-filtered introspection extends as follows for `SurfaceClient` callers:
308:- A SurfaceClient enumerating available abilities via an MCP-mediated path sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (c) has `mcp_exposure` of `MetadataOnly` or `Invocable`. Abilities with `mcp_exposure: None` are not enumerated. Abilities with `mcp_exposure: MetadataOnly` are enumerated with name + description only (no input/output schema); the invocation handler is not registered.
309:- A SurfaceClient enumerating available abilities via a client-side JS path (e.g., WordPress 7.0 `executeAbility()` enumeration) sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (d) has `client_side_executable: true`. Abilities with `client_side_executable: false` are not exposed to the client-side enumeration path even when fully `Invocable` via MCP.
311:- Per-ability metadata fields (name, description, schema, annotations, category) are model-facing AND browser-facing API surface for SurfaceClient consumers. Description copy is reviewed with the same care as user-facing UI (per [ADR-0083](0083-product-vocabulary.md) discipline).
312:- A SurfaceClient does NOT see ability names, schemas, or blast radius for abilities outside its scope grants, for `mcp_exposure: None` abilities on MCP-mediated paths, or for `client_side_executable: false` abilities on client-side JS paths. Unauthorized introspection is closed.
314:**Default policy.** Abilities default to `allowed_actors = [User]`, `required_scopes = vec![]`, `mcp_exposure = McpExposure::None`, `client_side_executable = false`. `SurfaceClient` exposure is opt-in per ability — the macro attribute must declare `allowed_actors` including `SurfaceClient`, plus `required_scopes` AND `mcp_exposure` AND `client_side_executable` explicitly (or `no_scope_required` in place of an empty `required_scopes` when the absence is intentional; see §7.1 macro). Most existing abilities do NOT need to be exposed to SurfaceClient instances; ability authors must consciously opt each one in. The two exposure fields govern different trust boundaries (MCP tool surface vs. client-side JS invocation) per Phase 0 artifact 05 lines 383-412 — either may be opted in while the other remains closed.
316:**Compile-error gate (substrate, not code-review discipline).** The `#[ability]` proc-macro emits a compile-time error if `allowed_actors` includes `SurfaceClient` AND `required_scopes` is empty AND the `no_scope_required` opt-out is absent. The gate fires on `allowed_actors` membership regardless of `client_side_executable` value, because actor membership is what makes the ability reachable from a SurfaceClient at all. This makes empty-scope SurfaceClient abilities a build-time failure, not a runtime gap or a reviewer-trust convention — drift is structurally impossible.
318:The spike at [DOS-546](https://linear.app/a8c/issue/DOS-546) validates this gating empirically for the WordPress SurfaceClient instance.
320:**Audit attribution.** Every substrate operation invoked by a SurfaceClient is logged with the SurfaceClient instance identity (per ADR-0111 §8). Reads and writes both. This makes "which surface invoked this" forensically answerable.
564:- Every experimental ability's `#[ability(experimental = true, registered_at = "2026-04-20")]` attribute requires a `registered_at` ISO-8601 date string.
565:- A CI check (`cargo test --test experimental_expiry`) reads all `experimental = true` attributes, parses `registered_at`, and fails the build if the date is older than 90 days (one release cycle, configurable).
579:## Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace
581:**Origin:** v1.4.7 W0 (MCP Server v2, Abilities-First). Drafted as the substantive design output of v1.4.7's cycle-2 Class S finding ("MCP client trust boundary"). Cycles 2–5 polished the contract; this amendment lands the final shape.
585:### A. `Actor::McpClient` variant
587:A fifth `Actor` variant joins `User`, `Agent`, `Admin`, `System`, `SurfaceClient`:
595:    SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet },
596:    McpClient {
597:        client_id: McpClientId,
603:`McpClientId` is a server-minted opaque handle issued during the MCP client pairing handshake (§C). Newtype lives at the abilities-runtime layer alongside `SurfaceClientId`; non-PII, non-enumerable, stable for the life of the pairing.
607:The corresponding `ActorKind::McpClient` discriminator joins the existing four kinds and is the const-friendly token used in `AbilityPolicy.allowed_actors`. `Actor::kind()` gains a `McpClient { .. } => ActorKind::McpClient` arm.
609:### B. Gateway-mediated scope enforcement (asymmetry with `SurfaceClient`)
611:`Actor::McpClient` carries **no** scopes on the variant. This is an intentional asymmetry with `Actor::SurfaceClient { .. scopes: ScopeSet }`:
613:- **SurfaceClient** carries `ScopeSet` on the variant because scopes are bridge-time material — the bridge (per ADR-0111 §8) reads `actor.scopes` directly to enforce `AbilityPolicy.required_scopes` before registry lookup. The scope grant rides with the invocation.
614:- **McpClient** does not carry scopes because the MCP gateway (v1.4.7 `services/mcp_v2/gateway.rs`) enforces scope authorization against a **server-side per-client manifest** loaded by `services/mcp_v2/auth.rs` from the pairing record (§C). Caller-provided scope claims are rejected on principle; the gateway is the single source of truth. Once the gateway admits the call, the abilities runtime trusts the gateway's check and gates only on `AbilityPolicy.allowed_actors.contains(&ActorKind::McpClient)`.
616:This preserves the v1.4.2 SurfaceClient bridge contract verbatim while introducing a strictly distinct trust path for MCP-originated calls — different attribution semantics, different audit shape, different rate-limit budget (§C). `AbilityPolicy.required_scopes` remains a SurfaceClient-only enforcement field; W0 ships no new field on `AbilityPolicy`.
618:### C. MCP client authentication + session contract
622:1. **Pairing handshake.** First contact from a new MCP client issues an `McpClientId` after a server-side handshake (W1-A `auth::pair_client(handshake) -> McpClientId`). The handshake binds the client to a server-stored manifest of granted scopes + per-tool rate limits + per-tool exposure tier. Manifest content is set by the operator (DailyOS-as-product owner) at pairing time, not negotiated by the caller. Mirrors the v1.4.2 `SurfaceClient` pairing shape (`services::surface_pairing`, `services::surface_session_keychain`, `bridges::surface_client`) — the SurfaceClient pattern is the precedent and the same operator/audit ergonomics apply.
623:2. **Transport signing (HMAC-SHA256).** All stdio and HTTP transports MUST present a per-message HMAC-SHA256 signature derived from the pairing-time shared key. Unsigned or invalid-signature messages are rejected at the transport layer before any `Actor::McpClient` is constructed. (Local STDIO MCP processes share the key out-of-band via the pairing record; loopback HTTP carries it as a request header.)
624:3. **Scope manifest authorization.** Gateway loads the server-side scope manifest for the `McpClientId` and verifies `tool.description().scopes_required.is_subset(manifest.granted_scopes)` BEFORE handler invocation. Caller-provided scope assertions in tool params are rejected with `ToolError::BadParams`.
625:4. **Per-actor × per-client × per-tool rate limits.** Gateway maintains a rate-limit registry keyed on `(ActorKind::McpClient, McpClientId, ScopedName)`. Limits are loaded from the pairing manifest; bursts exceeding them return `ToolError::RateLimited { retry_after_seconds }`. Independent of `AbilityRateLimit` (which remains a per-ability ceiling).
627:**Revocation.** Server-side admin API (W1-A scope) can revoke an `McpClientId`. Subsequent invocations from a revoked client fail the manifest-load step and return `ToolError::PairingRevoked`. Revocation propagates within one in-flight call; in-progress invocations are not interrupted but their audit row records the revoked state if revocation happens mid-call. (`PairingRevoked` is a dedicated `ToolError` variant rather than `Unauthorized` with a sentinel `missing_scope` because the `Scope` newtype is reserved for `<namespace>.<verb>.<noun>` / substrate-shipped values per §E — auth-state revocation is not a scope-deficit case.)
629:**Audit attribution.** Every ability invocation with `Actor::McpClient` records `(client_id, conversation_handle, tool_name, params_hash, response_hash, timestamp)` in the audit log. `params_hash` and `response_hash` are keyed HMAC-SHA256 over canonical JSON with a per-install audit key (per cycle 2 CSO MED #2) — raw param / response data never lands in audit storage.
655:### F. `AbilityPolicy.allowed_actors`
657:`AbilityPolicy.allowed_actors` may now include `ActorKind::McpClient` alongside the existing four kinds. Defaults are unchanged (`&[ActorKind::User]`, least-privilege floor); abilities opt in to MCP exposure by adding `ActorKind::McpClient` to their `allowed_actors` slice.
659:The proc macro emitting `AbilityDescriptor` (per `abilities-macro/src/lib.rs`) gains a `McpClient` arm in the `ActorArg` parser. `inventory.rs` `AbilityActor::project(ActorKind, McpExposure)` gains the same arm.
687:### §C.bis — `ExposureForbidden` joins `PairingRevoked` / `ConversationRevoked` as a dedicated `ToolError` variant
701:- **Mint records `(client_id, handle)`.** Server-side storage indexes the handle by the issuing `McpClientId`.
702:- **Resolution is keyed on `(client_id, handle)`.** A presented handle that exists but was issued to a different `McpClientId` resolves to "no matching binding" and returns `ToolError::ConversationRevoked` (the substrate's existing handle-revocation path; from the caller's perspective the handle is no longer valid and a fresh conversation must be started).
705:The runtime `Actor::McpClient { client_id, conversation_handle: Option<OpaqueConversationHandle> }` shape is unchanged — `client_id` is already present on the variant and naturally serves as the binding key when the gateway constructs `McpActor::Client` after manifest resolution.
709:This amendment does **not** change: the `Actor::McpClient` enum variant, `McpClientId` / `OpaqueConversationHandle` newtypes, the `McpExposure` tri-state, any wire-shape envelope, the pairing handshake, transport HMAC signing, the manifest schema's other fields, rate-limit registry, audit attribution, or signal payload shape.
715:### §C.bis.replay — Mandatory per-message nonce
719:Server-side enforcement, executed at the transport layer before `Actor::McpClient` construction (per the cycle-1 §C "rejected at the transport layer before any `Actor::McpClient` is constructed" rule):
732:Origin: v1.4.7 W1-A L0 cycle 6 — three convergent findings on the cycle-8 nonce-ledger amendment. Architect MED + challenge HIGH + CSO HIGH flagged that §C.bis.replay defined consume-once semantics but did not pin: (a) where the next nonce comes from after the seed is consumed, (b) ledger schema needs `issued_at`/`expires_at` for expiry of unconsumed nonces, (c) fail-closed semantics if downstream checks fail after consume.
734:### §C.bis.refresh — Per-response nonce refresh
751:### §C.bis.schema — Ledger table shape
784:### §C.bis.fail-closed — Fail-closed semantics after consume
788:### §C.bis.sweep — Background nonce sweep

exec
/bin/zsh -lc 'rg -n "render_provenance_for|64KB|surface|actor|sensitivity|claim|cap|mask|redact|OUTPUT|render" .docs/decisions/0108-provenance-rendering-and-privacy.md' in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc 'rg -n "invoke_registry_json|global_checked|struct AbilityRegistry|OnceLock" src-tauri/abilities-runtime/src/abilities/registry.rs src-tauri/src/bridges/tauri.rs src-tauri/src/bridges/mcp.rs src-tauri/src/abilities/mod.rs' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
7:**Related:** [ADR-0094](0094-audit-log-and-enterprise-observability.md), [ADR-0093](0093-prompt-injection-hardening.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` references this ADR's actor-filtered envelope; the ref shape avoids exceeding the rendering-side payload caps this ADR governs)
11:[ADR-0105](0105-provenance-as-first-class-output.md) defines the `Provenance` envelope and requires surfaces to render it. It defers three concerns to this ADR:
13:1. **Rendering rules per surface** — what the Tauri app, MCP, P2 publications, and logs show.
14:2. **Privacy and safety** — what gets masked, redacted, or filtered when rendered to different actors or published externally.
17:[ADR-0094](0094-audit-log-and-enterprise-observability.md) governs the append-only JSONL security audit with strict PII hygiene. [ADR-0093](0093-prompt-injection-hardening.md) established that LLM outputs can contain adversarial content. This ADR integrates both: provenance must be renderable to users and agents without leaking internal graph structure or PII, and sanitization of LLM-generated `explanation` text must prevent re-injection.
44:- Expandable "Details" block is available in the publication renderer but only if the publishing user confirms at publish time that the detail level is appropriate for the P2 audience. This is an explicit confirmation in the publish dialog, not a default.
45:- Synthesis markers render as "AI-generated summary" prefix on the publication body.
46:- Trust classification does not render to external readers; it is internal operational metadata.
52:- [ADR-0094](0094-audit-log-and-enterprise-observability.md)'s append-only JSONL security audit remains unchanged — it captures security events (auth, permissions, source revocations, etc.) with its existing hygiene rules. Provenance lives in the operational `maintenance_audit` SQLite table, separate from the security audit.
56:A universal renderer transforms `Provenance` into an actor-appropriate view:
59:pub fn render_provenance_for(prov: &Provenance, actor: Actor, surface: Surface) -> RenderedProvenance { ... }
65:    P2Publication,    // External audience, heavy redaction
70:The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.
76:LLM-generated `explanation` text is untrusted per [ADR-0093](0093-prompt-injection-hardening.md). It can contain prompt injection attempts, PII from other sources, or links to attacker-controlled content. Every explanation is passed through a sanitizer before rendering:
78:1. **HTML entities encoded.** Any `<`, `>`, `&`, `"`, `'` is entity-escaped. No raw HTML or Markdown links rendered from explanations.
82:5. **No executable content.** Regardless of surface, explanations are never passed to a markdown renderer, a JS `eval` context, or any other dynamic evaluator.
84:The sanitizer is shared across all surfaces; the rendering layer calls it once per `FieldAttribution` before composing the rendered output.
88:When a source is revoked and its associated provenance records are masked per [ADR-0107](0107-source-taxonomy-alignment.md) §5:
95:    pub masked_at: DateTime<Utc>,
96:    pub mask_reason: MaskReason,
97:    pub sources_masked: Vec<DataSource>,     // Which sources triggered the masking
108:The `ProvenanceOrMasked` enum in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8's `MaintenanceAuditRecord` is either the full envelope or this masked placeholder. Consumers handle both.
110:**Masking is irreversible.** Once a provenance is masked, the original content is discarded. Re-running the ability after the source is re-connected produces a new invocation with its own provenance. This is the cost of the mask — diagnosis of past runs becomes unavailable for revoked sources, but user privacy is protected.
116:- **Tauri app initial render:** top-level summary ≤ 2KB. Expansion fetches additional detail on demand.
120:When rendering exceeds a budget, the renderer truncates deepest-first (children elided first, sources last) and surfaces a `ProvenanceWarning::TruncatedForRender { surface, budget_bytes, original_bytes }`.
124:[ADR-0105](0105-provenance-as-first-class-output.md) §7 declared "every surface that displays ability output MUST be capable of rendering provenance." This ADR narrows the "MUST" to outputs where provenance rendering is meaningful:
139:The test: if the output is a *composed* or *synthesized* artifact the user will reason about, render provenance. If it is a *presentation affordance* showing a value, a count, or a name, the rendering is optional (but the provenance still exists in storage and can be queried).
145:1. **Rendering rules are surface-specific.** The Tauri app gets rich detail; MCP gets actor-filtered; P2 gets heavy redaction; logs get operational metadata. No one-size-fits-all.
147:3. **LLM-generated explanations are sanitized.** Prompt injection through `explanation` fields is defanged before rendering.
149:5. **Size budgets bounded per surface.** Rendering cost is predictable; runaway provenance payloads are truncated with an explicit warning.
150:6. **Rendering-required boundary is clear.** Not every UI element bears the "MUST render" burden; composed and synthesized artifacts do.
154:1. **Renderer complexity grows.** A shared `render_provenance_for(prov, actor, surface)` function with branching logic per surface and actor is non-trivial. Worth it.
156:3. **MCP `get_provenance(invocation_id)` tool adds surface.** Agents can request detail they did not receive by default.
162:2. **Renderer drift across surfaces.** The Tauri app diverges from MCP in how it interprets provenance. Mitigation: `render_provenance_for` is the single entry point; surfaces cannot bypass it.
163:3. **Masked provenance hides legitimate debugging.** User reports data issue; the relevant maintenance audit is masked. Mitigation: masking is irreversible as a matter of privacy. Live re-run reproduces the data issue with fresh provenance.
164:4. **P2 publication leaks through "Details" toggle.** User opts in to full detail and leaks internal IDs. Mitigation: publication "Details" renders through the same actor filter as MCP when the P2 audience includes non-team members; team-internal publishes get full detail.
169:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — Defines the envelope; this ADR specifies rendering.
170:- [ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md) — Specifies `DataSource::display_name()` and `ScoringClass`; rendering uses both.
171:- [ADR-0094: Audit Log and Enterprise Observability](0094-audit-log-and-enterprise-observability.md) — Security audit remains separate from provenance rendering.
174:- **ADR-0111 (forthcoming): Surface-Independent Ability Invocation** — Consumes the per-surface rendering rules specified here.
178:## Amendment — 2026-04-20 — Enforce 64 KB provenance size cap
182:§6 originally introduced size budgets as a soft guideline. In practice a deeply-composed ability (meeting prep composing context which composes claims which compose trajectories) can construct a provenance envelope of several hundred KB. Tauri IPC has serialization limits; this will hit them. Persona review flagged: named limits without enforcement become silent bloat.
184:**Hard cap: 64 KB serialized provenance per ability output.**
186:- The cap applies to the full `Provenance` envelope after JSON serialization, before any surface rendering.
187:- Abilities that would exceed the cap return `Err(AbilityError::ProvenanceTooLarge { size_bytes, cap_bytes })`. No silent truncation.
188:- The cap is enforced at `AbilityOutput<T>` construction time. The error surfaces to the caller as a hard error (per [ADR-0102](0102-abilities-as-runtime-contract.md) error-handling amendment).
193:2. **Shallower composition** — refactor the ability to compose fewer children or to use an alternative data source.
194:3. **Conscious cap increase** — via ADR amendment, after measuring real production composition shapes. Not an automatic bump.
196:**Starting value of 64 KB is conservative.** Measure during the first end-to-end slice (strategy doc § Path forward action 1). If real meeting-prep provenance is larger than 64 KB on representative entities, raise the cap via amendment with the measurement as justification. Do not raise silently.
198:**Rendering budget (original §6) vs serialization budget (this amendment):** different concerns. The rendering budget limits what the renderer emits for a given surface; this cap limits what the envelope itself can carry regardless of surface. Both exist independently; rendering truncation does not fix a too-large envelope because the envelope is already over-cap before the renderer sees it.

 succeeded in 0ms:
src-tauri/src/bridges/mcp.rs:14:    confirmation_args_hash, invoke_registry_json, surface_error, BridgeNoopIntelligenceProvider,
src-tauri/src/bridges/mcp.rs:411:        let response = invoke_registry_json(
src-tauri/src/bridges/tauri.rs:13:    claim_dismissal_surface_for_non_tauri_bridge, invoke_registry_json,
src-tauri/src/bridges/tauri.rs:217:        invoke_registry_json(
src-tauri/src/bridges/tauri.rs:291:        invoke_registry_json(
src-tauri/abilities-runtime/src/abilities/registry.rs:10:use std::sync::{OnceLock, RwLock};
src-tauri/abilities-runtime/src/abilities/registry.rs:232:/// The container is [`RwLock`] (not [`OnceLock`]) so tests can replace the
src-tauri/abilities-runtime/src/abilities/registry.rs:824:pub struct AbilityRegistry {
src-tauri/abilities-runtime/src/abilities/registry.rs:838:    pub fn global_checked() -> Result<&'static Self, &'static [RegistryViolation]> {
src-tauri/abilities-runtime/src/abilities/registry.rs:839:        static REGISTRY: OnceLock<Result<AbilityRegistry, Vec<RegistryViolation>>> =
src-tauri/abilities-runtime/src/abilities/registry.rs:840:            OnceLock::new();
src-tauri/abilities-runtime/src/abilities/registry.rs:896:            // OnceLock::set returns Err if already initialized — that's

exec
/bin/zsh -lc 'rg -n "AbilityOutput<Composition>|Composition|ProvenanceRef|Provenance lives once|surface|actor-filtered|schema-bounded|ADR-0102|ADR-0108|ADR-0125" .docs/decisions/0130-surface-independent-composition-contract.md' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
1:# ADR-0130 — Surface-Independent Composition Contract
6:**Amended:** 2026-05-10 — §2 `Block.provenance` retyped from `ProvenanceEnvelope` (envelope copy) to `ProvenanceRef` (compact reference into the canonical envelope on `AbilityOutput<Composition>`). Preserves the ADR-0105 "lives once" invariant and keeps block-heavy compositions under ADR-0108's 64KB serialized-provenance cap. Source: Phase 0 artifact `06-composition-provenance-ref.md`.
7:**Amended:** 2026-05-10 — §3 fallback rewritten from embed-model nearest-known + "rendered as nearest known type" indicator to deterministic schema-bounded projection at JSON-Pointer granularity. Unknown payload fields are dropped, not displayed; `claim_refs` and `provenance_ref` are preserved; rendered block degrades to `needs_verification` and carries a non-dismissible banner. Source: Phase 0 artifact `07-custom-block-fallback-projection.md`.
9:**Relates to:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md)
10:**Linear:** [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike (validates this contract for the WP instance)
14:DailyOS has converged on a runtime + composable surfaces architecture (ADR-0128 surface-pluggable thesis; ADR-0129 WordPress as a leading concrete surface; ADR-0111 §8 generalized SurfaceClient as the fourth actor class for third-party local surfaces). The substrate-side contracts are in place:
16:- **Abilities** ([ADR-0102](0102-abilities-as-runtime-contract.md)) — typed runtime capabilities.
18:- **Invocation** ([ADR-0111](0111-surface-independent-ability-invocation.md)) — bridge-per-surface ability invocation with actor-filtered discovery.
19:- **Rendering** ([ADR-0108](0108-provenance-rendering-and-privacy.md)) — provenance rendering per surface/actor.
21:What's missing is the contract for *composition*: how a substrate-produced briefing, account page, prep pack, or report is described independently of any rendering technology. ADR-0129 §6 commits to Gutenberg blocks as "the right shape" for composed surfaces, but does not separate the *generic* shape (typed blocks composed into structured pages) from the *implementation* (Gutenberg). Without that separation, WordPress quietly becomes more than a surface — it becomes the document model. That cuts against ADR-0129 §1's own claim that no surface is sacred.
23:The L0 Prep follow-up on DOS-546 (Codex Pass 2 formal L0, 2026-05-10) named this gap explicitly: "The biggest decision L0 should force is canonical composition authority. ADR-0129 commits to Gutenberg blocks as 'the right shape', but does not decide whether DailyOS has its own surface-independent composition IR. It should." This ADR fills that gap.
29:Composed surfaces (briefings, entity pages, prep packs, reports, callouts, agentic-block compositions) are described by substrate primitives independent of any rendering technology. Surfaces render the composition; surfaces do not author it.
31:This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md) §1 from "surfaces invoke abilities" to "surfaces consume substrate output, including composed pages." It generalizes [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §1 from "the substrate is the product; the heads are surfaces over it" to apply at the *page* level, not just the *claim* level.
35:The Composition contract is a producer/consumer seam on top of existing v1.4.0/v1.4.1 substrate. Composition-producing abilities and surface renderers CONSUME the following primitives; they do not reinvent them:
37:- **Retrieval and salience** — Hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)). The substrate owns the relevance question. The Composition contract names `Salience.weight` as a producer field; the score itself flows from the retrieval primitives.
38:- **Trust scoring** — Per-claim trust assessment per [ADR-0105](0105-provenance-as-first-class-output.md). Renderers surface trust bands from the resolved provenance envelope; they do not compute trust.
39:- **Signals + invalidation** — Signal propagation per [ADR-0080](0080-signal-intelligence-architecture.md) drives composition refresh. Compositions are derived state over claims; signal-driven invalidation is the canonical refresh path.
40:- **Abilities runtime** — Producer-side execution per [ADR-0102](0102-abilities-as-runtime-contract.md). Composition-producing abilities are normal abilities returning `AbilityOutput<Composition>`.
41:- **Ability invocation** — Surface-independent invocation per [ADR-0111](0111-surface-independent-ability-invocation.md) §8. The `SurfaceClient` actor class is the invocation channel for third-party local surfaces consuming compositions.
45:### 2. Composition primitives
47:The substrate emits composed output as instances of a typed `Composition` model. A `Composition` is the `T` inside `AbilityOutput<Composition>` per [ADR-0102](0102-abilities-as-runtime-contract.md) §6 — the canonical `Provenance` envelope lives exactly once on the wrapper, not on the domain output. The Composition model itself carries no envelope copy; blocks carry compact references into the wrapper envelope. The model:
50:pub struct Composition {
51:    pub id: CompositionId,
52:    pub kind: CompositionKind,  // Briefing | EntityPage | PrepPack | Report | Callout | Custom
72:    pub provenance: ProvenanceRef, // Compact reference into AbilityOutput<Composition>.provenance
78:/// AbilityOutput<Composition>.provenance per ADR-0102 §6 and ADR-0105 §8.
81:pub struct ProvenanceRef {
95:**Why `ProvenanceRef`, not `ProvenanceEnvelope`.** Embedding a full envelope on every block creates a second provenance carrier inside the domain output, which violates [ADR-0102](0102-abilities-as-runtime-contract.md) §6 + §9 Rule 5 (provenance lives exactly once on `AbilityOutput<T>`). It also defeats [ADR-0108](0108-provenance-rendering-and-privacy.md)'s 64KB serialized-provenance cap: a 24-block composition with typical 12KB Transform envelopes serializes to ~288KB of duplicated provenance, 4.5× the cap before any block content is counted. A `ProvenanceRef` is ~80-200 bytes serialized; a 60-block long-form composition spends ~13KB on block refs, well inside the cap. The reference shape mirrors [ADR-0105](0105-provenance-as-first-class-output.md) §8's planned-mutation pattern — plans reference the envelope by invocation id and field path rather than duplicating it.
97:**Resolution.** Renderers resolve a `ProvenanceRef` by fetching the canonical envelope for `invocation_id` from the runtime provenance store (already addressable per ADR-0105/ADR-0108), reading the `FieldAttribution` at `field_path`, and passing the resolved envelope through [ADR-0108](0108-provenance-rendering-and-privacy.md) §2's actor-filtered `render_provenance_for(prov, actor, surface)`. Resolution can fail (garbage-collected envelope, unknown invocation across environments, unauthorized actor, masked provenance, missing field attribution); failure MUST degrade visibly — the block renders with a provenance-unavailable banner or downgraded trust band, never as fully trusted. When the envelope resolves but the exact `field_path` does not, the surface MAY fall back to invocation-level provenance, but MUST label the fallback as less specific.
101:**Composition-size guard.** Block provenance refs SHOULD stay under 48KB serialized across a single composition. Pathologically block-heavy compositions (>~200 blocks) that exceed the guard MUST collapse to section-level refs rather than embedding envelopes. The 48KB guard preserves headroom under ADR-0108's 64KB cap for surrounding composition metadata.
107:**Salience computation uses existing substrate primitives.** The `Salience.weight` field is typically computed via the substrate's hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)), combined with signal correlation, recency, and claim-trust factors. The Composition contract does not mandate the scoring algorithm — it names the substrate's existing retrieval primitives as the canonical inputs. **Composition-producing abilities MUST NOT reinvent retrieval.** They consume the substrate's embed-model-backed search; the substrate owns the relevance question. This rule prevents the failure mode where an ability author rebuilds salience scoring from scratch without realizing the substrate already has it.
126:Surfaces that do not know a `Custom` block type MUST render a **schema-bounded projection** onto the nearest known block type. Renderers MUST NOT render unknown block payload fields directly. The previous "raw payload available" fallback is removed — an unknown block type is a privacy boundary, not a debugging opportunity. Unknown payload fields may include internal notes, source excerpts, email addresses, prompt context, debug carriers, or fields whose sensitivity tier is not allowed on the current surface (per [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)). Ability metadata is also model-facing and browser-facing API surface per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.6; raw payload disclosure is an info-disclosure vector.
130:1. **Select the nearest known type deterministically.** Rank candidate known block types by (a) matching `composition_kind` (weight 100), (b) compatible required-pointer overlap (weight 10 per pointer), (c) compatible optional-pointer overlap (weight 2 per pointer), (d) render-annotation similarity (0-20), (e) namespace-prefix similarity (0-5). Break ties by `kind_match`, then `required_overlap`, then `optional_overlap`, then `annotation_similarity`, then **lexicographic block type id**. The lexicographic final tie-break guarantees all renderers across all surfaces select the same fallback for the same input. Candidate eligibility is gated by `allowed_surfaces` and actor reachability.
135:6. **Preserve `provenance` (the `ProvenanceRef`) exactly.** This is not payload — it is a reference into the [ADR-0105](0105-provenance-as-first-class-output.md) envelope and renders only through [ADR-0108](0108-provenance-rendering-and-privacy.md) actor-filtered rules. The fallback path MUST NOT dereference `claim_refs` or `provenance` to backfill dropped payload fields.
146:Source: Phase 0 artifact [`07-custom-block-fallback-projection.md`](../plans/dos-546/phase-0/07-custom-block-fallback-projection.md). (Fallback policy added 2026-05-10 per Custom block fallback finding in DOS-546 L0 Cycle 2; rewritten 2026-05-10 per /cso refinement 10 to schema-bounded projection.)
150:Each surface ships a renderer that maps `BlockType` to its native rendering technology:
158:| Future surfaces | Their own renderers | Per BlockType |
160:Renderers are surface-side code. They do not modify the composition; they project it. The same `Composition` can render in any surface and the user gets equivalent intelligence with surface-appropriate visual treatment.
162:Surfaces MAY decline to render specific block types (e.g., MCP head returns claim-JSON for `AgenticPrompt` rather than rendering an invocable UI; CLI head emits a marker for `MagazineEnd` rather than visual spacing). The Composition stays canonical; surfaces decide what to show.
166:Abilities that produce composed pages (briefings, entity pages, prep packs, reports) return `Composition` as their output. The composition is authored by the ability with full provenance attribution per [ADR-0105](0105-provenance-as-first-class-output.md). Surfaces never construct compositions; they receive them through the normal ability-invocation path and render them.
170:- A `briefing.daily` ability produces a `Composition { kind: Briefing, sections: [...], ... }`.
171:- An `entity.account_overview` ability produces a `Composition { kind: EntityPage, subject: Some(account_ref), ... }`.
172:- A `prep.meeting` ability produces a `Composition { kind: PrepPack, subject: Some(meeting_ref), ... }`.
175:This is the cleanest expression of the AI-native principle from [PHILOSOPHY.md](../../design/product/PHILOSOPHY.md): AI produces, users consume. Authorship is substrate-side; consumption is surface-side; the contract between them is the `Composition` model.
179:A `SurfaceClient` instance (per [ADR-0111](0111-surface-independent-ability-invocation.md) §8) consumes compositions by:
182:2. Receiving an `AbilityOutput<Composition>` — the `Composition` is the domain output; the canonical `Provenance` envelope lives once on the wrapper.
183:3. Rendering blocks via its surface-specific renderer. For each block, resolve `provenance: ProvenanceRef` against the wrapper envelope. Batch lookups by unique `invocation_id` on first paint to avoid per-block round trips; the first visible viewport SHOULD complete within ~50ms p95 for local-runtime lookups and ~150ms p95 for remote-runtime lookups. Content MAY render immediately with a pending trust band when the lookup is slower.
184:4. Honoring `Salience` for visibility/ordering decisions where the surface allows differential treatment (e.g., hiding `Background` blocks in compact mode).
186:6. Applying [ADR-0108](0108-provenance-rendering-and-privacy.md) §2 `render_provenance_for(prov, actor, surface)` to every resolved envelope. The `ProvenanceRef` does not bypass actor/surface filtering; it only locates the canonical envelope to which those rules apply.
188:The same composition rendered by the WordPress SurfaceClient, Tauri React surface, and MCP head produces equivalent intelligence — different paint, same content, same provenance.
192:Per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §6, layouts can be agent-composed at runtime based on substrate salience rather than fixed templates. Under this contract, on-the-fly composition is an ability behavior: the producing ability emits a `Composition` whose `sections` and `blocks` are chosen at invocation time based on what's salient. The contract is unchanged; the ability's logic does the choosing.
194:Templated layouts are a special case: the ability emits compositions whose section/block shape matches a predeclared template. Both modes use the same `Composition` model.
199:- **Renderer implementations for each surface.** Out of scope for this ADR; lives in surface-specific implementation work (e.g., DOS-546 validates the WP renderer).
200:- **Editing semantics.** A user editing a block's value (claim correction) routes through the existing feedback path per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5; this ADR does not redesign feedback.
208:- **Surfaces are genuinely interchangeable.** A new SurfaceClient instance (Obsidian, browser extension, mobile, future-thing) ships a renderer and consumes the same compositions. No substrate change required to add a surface.
209:- **The Composition Contract is the seam DOS-546 actually validates.** The spike validates that the WordPress SurfaceClient renderer can render substrate-authored compositions; it doesn't have to invent the composition model.
211:- **Salience-driven rendering is structurally enabled.** Per-block salience flows through the composition; surfaces honor it.
212:- **Provenance lives once.** `ProvenanceRef` preserves ADR-0102 §6 + ADR-0105 §8's "lives once" invariant. Block-heavy compositions stay inside ADR-0108's 64KB serialized-provenance cap — a 60-block long-form composition spends ~13KB on refs vs. ~720KB if envelopes were copied per block.
213:- **Unknown blocks are a privacy boundary, not a debug surface.** Schema-bounded fallback projection prevents internal notes, source excerpts, email addresses, prompt context, and debug carriers from reaching the DOM, REST preload state, hydration state, or inspector UI via unknown payload fields. Sparse-but-safe is the correct behavior for unknown content.
214:- **Cross-surface intelligence parity is structural.** Same composition → equivalent intelligence in any surface, with the same deterministic fallback selection (lexicographic tie-break) when a renderer encounters an unknown type.
218:- **Composition model is a substantial new contract.** Several types, several taxonomies, plus the `ProvenanceRef` resolution and schema-bounded projection paths. Implementation cost is real.
219:- **Existing compositions are implicit.** The Tauri React magazine builds composed pages today via React components, not via a `Composition` model. Migration to producing `Composition` from the substrate side is real work (likely in v1.4.2-v1.4.4 wave program).
220:- **BlockType registry needs governance.** Every new block type ships with a JSON Schema (required + optional pointer sets, render annotations, allowed surfaces, default trust band) so fallback projection has a deterministic target. A type that ships without a schema is unrenderable by foreign surfaces.
221:- **Provenance-resolver retention.** Compositions that outlive runtime provenance retention need either longer retention for published-composition invocations or a durable exported provenance bundle keyed by `invocation_id`. The block ref shape is the right address either way; what changes is where the envelope is fetched from.
222:- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.
228:- ADR-0128 §1 is preserved — substrate is the product, surfaces are heads over it.
229:- ADR-0102 §6/§9 Rule 5 and ADR-0105 §8 are preserved structurally — provenance lives once, on the wrapper, and references address it.
238:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities runtime contract; §6 + §9 Rule 5 establish provenance-lives-once on `AbilityOutput<T>`; §7.1 canonical AbilityPolicy schema; §7.6 governs SurfaceClient-reachable ability metadata
239:- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance envelope shape; §8 establishes the reference pattern that `ProvenanceRef` mirrors
240:- [ADR-0108](0108-provenance-rendering-and-privacy.md) — Actor-filtered provenance rendering; 64KB serialized-provenance cap that motivates the ref shape
241:- [ADR-0111](0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation; §8 `SurfaceClient` is the consumer for the Composition Contract
242:- [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md) — Sensitivity tiers (`Public`, `Internal`, `Confidential`, `UserOnly`) consumed by schema-bounded fallback projection
243:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
244:- [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) — Composable surfaces; this ADR generalizes §6's block-shaped composition claim; WP Studio is the first concrete renderer
245:- [Phase 0 artifact 06](../plans/dos-546/phase-0/06-composition-provenance-ref.md) — Source for the `ProvenanceRef` shape (§2 amendment)
246:- [Phase 0 artifact 07](../plans/dos-546/phase-0/07-custom-block-fallback-projection.md) — Source for schema-bounded fallback projection (§3.1 amendment)
247:- [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike; validates the WP renderer against this contract
249:- [Product Thesis](../../design/product/PRODUCT-THESIS.md) — Personal intelligence that compounds across surfaces
253:- [WordPress Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) — One concrete renderer target for the Composition Contract

exec
/bin/zsh -lc 'rg -n "CLAIM_TYPE_REGISTRY|sensitivity|claim type|registry|Public|Internal|Confidential|UserOnly|Temporal|claim" .docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
1:# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry
5:**Amended:** 2026-05-19 — added §5 `RecommendationClaim` variant for v1.4.6 Salience & Recommendations. Registers `ClaimType::Recommendation` in `CLAIM_TYPE_REGISTRY` (canonical persisted name `"recommendation"`) with `State` temporal scope, `Internal` sensitivity, `Medium` freshness decay, `Replace` commit policy, agent-only authorship, attaches to `Account` / `Project` / `Person` subjects. Salience factors, user-feedback state (`Accepted` / `Dismissed` / `NotUseful` / `TooNoisy` / `Converted`), conversion state, and supersession + dismissal lifecycle are declared at the substrate level; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`. Per the v1.4.6 W0 hardening cycle in [`.docs/plans/v1.4.6-waves.md`](../plans/v1.4.6-waves.md).
6:**Target:** v1.4.0 (schema + enums + registry); v1.4.1 (DOS-10 freshness + DOS-214 render policy); v1.4.2/3 (per-surface enforcement); v1.4.6 (Recommendation variant)
7:**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0114](0114-scoring-unification.md)
12:The 2026-04-24 Claim Anatomy Review (`.docs/plans/claim-anatomy-review-2026-04-24.md`) walked every dimension a claim should express in the v1.4.x substrate. Three dimensions were under-scrutinized at the substrate level and warrant a substrate primitive in v1.4.0 spine:
14:1. **Temporal scope** (Review §11) — distinguishes PointInTime / State / Trend claims; affects freshness, supersession, and contradiction semantics.
15:2. **Sensitivity** (Review §12) — claim-level sensitivity tier; structural safeguard against accidental surface leakage.
16:3. **Claim type taxonomy** (Review §15) — currently free-form strings; should be a closed-but-extensible registry following the ADR-0115 Signal Policy Registry pattern.
18:This ADR is the substrate allowance: schema fields + types + registry mechanism. Downstream enforcement (freshness scoring, render gates) is named per-dimension and lands in v1.4.1+.
22:### 1. Temporal scope
25:pub enum TemporalScope {
52:ALTER TABLE intelligence_claims ADD COLUMN temporal_scope TEXT NOT NULL DEFAULT 'state';
63:    Public,
67:    /// **Default for claims sourced from internal systems.**
68:    Internal,
72:    Confidential,
76:    UserOnly,
83:ALTER TABLE intelligence_claims ADD COLUMN sensitivity TEXT NOT NULL DEFAULT 'internal';
86:Default `Internal` is the conservative choice — most claims sourced from internal systems should default Internal. `Public` requires explicit author choice or source-class inheritance. `Confidential` / `UserOnly` require explicit author choice.
90:### 3. Claim type registry
92:Today `claim_type` is a free-form string. Every Transform ability defines its own implicitly. Dedup (per claim_type), freshness decay (per claim_type per DOS-10), commit policy gates, rendering policy — all key on this string. Drift is invisible.
94:This ADR introduces `ClaimTypeRegistry` — a compile-time exhaustive const slice mapping every legal `claim_type` to its metadata:
98:    /// Canonical string written into the `intelligence_claims.claim_type` column.
101:    /// Default temporal scope for this claim type. Authors may override per-claim
103:    pub default_temporal_scope: TemporalScope,
105:    /// Default sensitivity. Same author-override semantics.
106:    pub default_sensitivity: ClaimSensitivity,
115:    /// Which actor classes may write this claim type.
118:    /// Which `SubjectRef` variants this claim type can attach to.
119:    /// Example: `stakeholder_role` claims attach to `Person` only;
124:pub const CLAIM_TYPE_REGISTRY: &[ClaimTypeMetadata] = &[
127:        default_temporal_scope: TemporalScope::State,
128:        default_sensitivity: ClaimSensitivity::Internal,
136:        default_temporal_scope: TemporalScope::State,
137:        default_sensitivity: ClaimSensitivity::Internal,
149:- A claim row written with a `claim_type` not in the registry **fails CI lint**.
150:- New claim types require an ADR amendment (or registry-extension PR with documented rationale linking back to this ADR).
153:v1.4.0 spine ships the registry mechanism + initial set covering the two pilot abilities (DOS-218 + DOS-219). New claim types are added incrementally as new abilities ship in v1.4.1+.
159:| Temporal scope | column + enum + serde | DOS-10 freshness consults it; supersession semantics | — | — | `Recommendation` uses `State` |
160:| Sensitivity | column + enum + serde | DOS-214 render layer enforces | entity surfaces enforce ceiling | briefing surfaces enforce ceiling | `Recommendation` defaults `Internal` |
161:| Claim type registry | mechanism + initial set + CI lint | extensions as new abilities ship | extensions | extensions | `Recommendation` variant added (W0) |
165:**Decision (2026-05-19, W0 of v1.4.6 Salience & Recommendations).** Recommendations are first-class claims, not a parallel substrate. They register in `CLAIM_TYPE_REGISTRY` and ride the same provenance, trust, supersession, and sensitivity primitives every other claim type already inherits from this ADR. The W0 amendment fixes the substrate-level shape; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`.
167:**Why a claim-type, not a separate table.** A recommendation says *"this action is recommended for this subject as of this moment, with this evidence and this expected impact"* — that is a claim. Trust band, freshness decay, contradiction handling, and the existing claim lifecycle (assert → corroborate → contradict → supersede → retract) already apply. Making it a claim means every existing intelligence-loop surface (entity context, briefing prep, surface render) consumes recommendations through the same render path as every other claim, with the same provenance receipt and the same trust signaling. A parallel `recommendation_claims` table would have re-invented all of that.
174:| `default_sensitivity` | `Internal` | Recommendations are derived intelligence. They reference upstream evidence whose own sensitivity may be higher; render-time masking still consults the upstream evidence's sensitivity (per §2). |
176:| `commit_policy_class` | `Replace` | A newer same-subject recommendation supersedes the older one. Recommendations are uniquely identified by `(subject, recommended_action_kind)`; corroboration semantics (`Reinforce`) don't apply because two distinct agent runs producing the same recommendation should *replace* (newer evidence wins), not stack confidence. Contradiction (`Fork`) is handled at the salience layer (W1-B): mutually-contradictory recommendations on the same subject lower each other's salience score and surface as a triage decision, not as parallel claim rows. |
177:| `allowed_actor_classes` | `[Agent]` | Recommendations are agent-generated. User feedback (Accepted / Dismissed / NotUseful / TooNoisy / Converted) flows through `services::claims::record_claim_feedback` and emits `ClaimFeedbackRecorded` signals — it does *not* write a separate user-authored Recommendation claim. |
180:**Lifecycle additions beyond the base claim envelope.** Recommendations carry three substrate fields above what other claim types use, declared at the ADR level here and frozen as wire format in W1-A `contracts.rs`:
183:- **User-feedback state.** Closed enum `{ Accepted | Dismissed | NotUseful | TooNoisy | Converted }`. Default `None` (no feedback yet). State transitions are append-only at the `claim_feedback` level (per existing v1.4.0 substrate); the denormalized current-state field on the recommendation claim row is rebuilt from the feedback log by the W4-A feedback loop on each `ClaimFeedbackRecorded` signal.
186:**Supersession + dismissal.** Standard claim lifecycle applies:
187:- *Supersession*: a newer recommendation on the same `(subject, recommended_action_kind)` writes a `supersedes` reference to the older claim's id and the older claim's `verification_state` transitions to `Superseded`.
188:- *Dismissal*: `record_claim_feedback` with action `Dismissed` flips the user-feedback state to `Dismissed` and emits `ClaimFeedbackRecorded`. The W2-A surfacing policy treats `Dismissed` as a hard suppression for the recommendation's lifetime (with a configurable cooldown); W4-A feedback weights downgrade the source factors that contributed to the dismissed recommendation, so future similar candidates score lower.
191:**Surface render contract (v1.4.6 W3 consumers).** The `Recommendation` claim's render output goes through the same Shared Receipt DTO (`services::claim_receipt::contracts::ClaimReceipt`) every other claim uses. Per the v1.4.6 W0 close gate cross-version row, this ADR amendment is unblocked only after v1.4.4 W1-A ships the `ClaimReceipt` substrate.
195:- `recommendation_claims` table vs `intelligence_claims` extension → W1-A migration choice (slot range v260–v279)
200:- Cross-surface rendering → W3 (consumes `claim_receipt::contracts::ClaimReceipt`)
207:- Per-surface sensitivity ceilings (v1.4.1+).
209:- Counter-claim denormalized boolean (v1.4.1; deferred per anatomy review §13).
210:- Causal lineage between claims (v1.5.x; deferred per anatomy review §14).
220:- **Temporal scope** makes "this event happened" structurally distinct from "this state holds" — eliminates the wrong-decay-on-PointInTime bug before it ships.
221:- **Sensitivity** makes surface-leak prevention substrate-level instead of surface-policy convention. Surfaces that respect the sensitivity ceiling cannot leak Internal content into Public surfaces by construction.
222:- **Claim type registry** forces deliberation on new claim types and prevents silent taxonomy drift. Adding a new claim type is now an explicit, reviewed event — not a string typed into a Transform ability.
226:- Three new schema fields. Acceptable; all default to safe values (`State`, `Internal`).
227:- Registry maintenance burden — every new claim type requires a registration PR. Mitigated by mirroring ADR-0115 Signal Policy Registry pattern (proven on signals; same author cadence).
228:- Authors may forget to set non-default `temporal_scope` and `sensitivity`. Mitigated by `ClaimTypeMetadata.default_*` values being looked up at write time when claim row leaves either field as default — the metadata supplies the right value for the type.
233:- The registry pattern matches an existing precedent (ADR-0115); no new architectural concept.
237:- [ADR-0113: Human and Agent Analysis as First-Class Claim Sources](0113-human-and-agent-analysis-as-first-class-claim-sources.md) — claim row schema; this ADR adds three fields.
238:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `Provenance.warnings` interacts with sensitivity at render-time masking.
240:- [ADR-0115: Signal Granularity, Policy Registry, and Durable Invalidation](0115-signal-granularity-audit.md) — pattern parallel for the claim type registry's compile-time exhaustiveness.
242:- `.docs/plans/claim-anatomy-review-2026-04-24.md` — sourcing review (§11, §12, §15 for the three dimensions covered here).

exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/registry.rs | sed -n '920,965p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/registry.rs | sed -n '1458,1490p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   920	
   921	    /// Iterate every descriptor in the registry, no actor filter.
   922	    ///
   923	    /// Used by the W1-C `emit_ability_inventory` binary to project
   924	    /// the full ability set into the surface-facing inventory artifact.
   925	    /// Tooling-facing only — runtime callers should prefer
   926	    /// [`AbilityRegistry::iter_for`] so the actor gate stays in force.
   927	    pub fn iter_all(&self) -> impl Iterator<Item = &AbilityDescriptor> {
   928	        self.by_name.values()
   929	    }
   930	
   931	    pub fn iter_for(&self, actor: Actor) -> impl Iterator<Item = &AbilityDescriptor> {
   932	        self.by_name.values().filter(move |descriptor| {
   933	            if descriptor.experimental && actor != Actor::System {
   934	                return false;
   935	            }
   936	            if actor == Actor::Agent && descriptor.category == AbilityCategory::Maintenance {
   937	                return false;
   938	            }
   939	            descriptor.policy.allowed_actors.contains(&actor.kind())
   940	        })
   941	    }
   942	
   943	    pub async fn invoke_read(
   944	        &self,
   945	        ctx: &AbilityContext<'_>,
   946	        name: &str,
   947	        input: serde_json::Value,
   948	    ) -> Result<serde_json::Value, AbilityError> {
   949	        self.invoke_with_category(ctx, name, input, AbilityCategory::Read)
   950	            .await
   951	    }
   952	
   953	    pub async fn invoke_transform(
   954	        &self,
   955	        ctx: &AbilityContext<'_>,
   956	        name: &str,
   957	        input: serde_json::Value,
   958	    ) -> Result<serde_json::Value, AbilityError> {
   959	        self.invoke_with_category(ctx, name, input, AbilityCategory::Transform)
   960	            .await
   961	    }
   962	
   963	    pub async fn invoke_publish(
   964	        &self,
   965	        ctx: &AbilityContext<'_>,

 succeeded in 0ms:
  1458	        let age_days = Utc::now()
  1459	            .signed_duration_since(parsed.with_timezone(&Utc))
  1460	            .num_days();
  1461	        if age_days > 90 {
  1462	            violations.push(RegistryViolation::ExperimentalExpired {
  1463	                ability: descriptor.name.to_string(),
  1464	                age_days,
  1465	            });
  1466	        }
  1467	    }
  1468	}
  1469	
  1470	fn validate_invocation_policy(
  1471	    ctx: &AbilityContext<'_>,
  1472	    descriptor: &AbilityDescriptor,
  1473	) -> Result<(), AbilityError> {
  1474	    if !descriptor.policy.allowed_actors.contains(&ctx.actor.kind()) {
  1475	        return Err(AbilityError {
  1476	            kind: AbilityErrorKind::Capability,
  1477	            message: format!(
  1478	                "actor {:?} is not allowed to invoke `{}`",
  1479	                ctx.actor, descriptor.name
  1480	            ),
  1481	        });
  1482	    }
  1483	
  1484	    if !descriptor.policy.allowed_modes.contains(&ctx.mode()) {
  1485	        return Err(AbilityError {
  1486	            kind: AbilityErrorKind::Capability,
  1487	            message: format!(
  1488	                "mode {:?} is not allowed to invoke `{}`",
  1489	                ctx.mode(),
  1490	                descriptor.name

exec
/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/registry.rs | sed -n '820,845p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   820	    },
   821	}
   822	
   823	#[derive(Debug)]
   824	pub struct AbilityRegistry {
   825	    by_name: HashMap<&'static str, AbilityDescriptor>,
   826	}
   827	
   828	impl AbilityRegistry {
   829	    /// Collect from inventory and validate. Fails closed on any violation.
   830	    pub fn from_inventory_checked() -> Result<Self, Vec<RegistryViolation>> {
   831	        let descriptors = inventory::iter::<AbilityDescriptor>
   832	            .into_iter()
   833	            .cloned()
   834	            .collect();
   835	        Self::from_descriptors_checked(descriptors)
   836	    }
   837	
   838	    pub fn global_checked() -> Result<&'static Self, &'static [RegistryViolation]> {
   839	        static REGISTRY: OnceLock<Result<AbilityRegistry, Vec<RegistryViolation>>> =
   840	            OnceLock::new();
   841	        match REGISTRY.get_or_init(Self::from_inventory_checked) {
   842	            Ok(registry) => Ok(registry),
   843	            Err(violations) => Err(violations.as_slice()),
   844	        }
   845	    }

exec
/bin/zsh -lc "nl -ba src-tauri/src/bridges/tauri.rs | sed -n '205,225p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   205	            None
   206	        };
   207	        let invocation = InvocationContext {
   208	            actor: bridge_actor,
   209	            mode: ExecutionMode::Live,
   210	            surface: context.surface,
   211	            claim_dismissal_surface: context.claim_dismissal_surface,
   212	            dry_run: context.dry_run,
   213	            confirmation: context.confirmation,
   214	            confirmation_store,
   215	        };
   216	
   217	        invoke_registry_json(
   218	            self.registry,
   219	            &services,
   220	            provider,
   221	            &NOOP_ABILITY_TRACER,
   222	            invocation,
   223	            ability_name,
   224	            input_json,
   225	        )

exec
/bin/zsh -lc "rg -n \"account_status|daily_briefing|dailyos.read.account_status|dailyos.read.daily_briefing|read.account_overview|read.daily_briefing\" src-tauri/src/services/mcp_v2 .docs/plans/v1.4.7-w1-foundation docs/solutions .docs/decisions -g '*.*'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/services/mcp_v2/taxonomy.rs:547:            "dailyos.read.account_status",
src-tauri/src/services/mcp_v2/taxonomy.rs:548:            "dailyos.read.daily_briefing",
src-tauri/src/services/mcp_v2/taxonomy.rs:702:            Some(ScopedName::new("dailyos.read.account_status")),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:20:User\s stated outcome this session: get end-to-end Claude Desktop -> DailyOS substrate working so "give me an account overview for acme" returns real data, comparable to Glean. Is the packet scoped sharply enough to deliver that, or could it be even tighter (e.g., cut daily_briefing and ship just account_status)?
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:35:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:47:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:48:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:50:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:71:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:125:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:149:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:154:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:186:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:191:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:225:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:227:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:229:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:231:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:235:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:237:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:239:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:247:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:286:**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:326:v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:327:- `dailyos.read.daily_briefing` — current-day briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:353:2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:371:- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:589:The first pass found an important tension: DOS-624 explicitly says W2-A includes both `daily_briefing` and `meeting_briefing`, while the L0 packet defers `meeting_briefing`. I’m checking the wave plan and the cited substrate docs now to see whether that is a real blocker or a documented scope reset.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:591:/bin/zsh -lc 'rg -n "DOS-175|W2-A|account_status|daily_briefing|meeting_briefing|get_daily_briefing|account_overview|McpToolHandler|AbilityRegistry|invoke_registry_json|McpAbilityBridge|ADR-0102|ADR-0108|ADR-0125|ADR-0093|ADR-0128|W1-A handler seam" .docs/plans/v1.4.7-waves.md .docs/plans/v1.4.7-w1-foundation/*.md' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:594:.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:6:**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:602:.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:46:v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:603:.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:47:- `dailyos.read.daily_briefing` — current-day briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:609:.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:73:2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:612:.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:91:- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:617:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:1:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:623:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:13:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:624:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:14:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:625:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:635:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:37:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:642:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:91:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:644:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:645:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:648:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:152:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:649:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:157:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:654:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:191:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:655:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:193:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:656:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:195:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:657:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:197:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:658:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:201:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:659:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:660:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:661:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:213:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:670:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:68:   - name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:671:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:84:     scopes_required: [dailyos.read.account_status]
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:672:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:92:         invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:673:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:96:         - { prompt: "What's going on with Acme?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:674:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:97:         - { prompt: "How are things looking with the Hooli deal?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:678:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:149:| 1 | `dailyos.read.account_status` | Read | `dailyos.read.account_status` | W2 (out-of-scope handler — discovery surface from DOS-172 ranked entity lookup) |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:679:.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:150:| 2 | `dailyos.read.daily_briefing` | Read | `dailyos.read.daily_briefing` | W2-A (DOS-175) |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:715:.docs/plans/v1.4.7-waves.md:14:- [DOS-624](https://linear.app/a8c/issue/DOS-624) — v1.4.7 W2-A `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` MCP tools wrap intelligence the v1.4.3 carve-out reserved as User-only. CSO-approved L0 amendment required before W2-A implementation; amendment must also coordinate with v1.4.2 W3-C MCP path to avoid two parallel MCP exposures of the same intelligence.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:717:.docs/plans/v1.4.7-waves.md:44:8. **Canonical convention: `<dot-namespace>.<verb>.<noun>`** — e.g., `dailyos.read.account_status`, `dailyos.write.place_document`, `dailyos.submit.note`. v1.4.5 already shipped `write.workspace_place_document` for the AI placement ability; v1.4.7 W3-B's MCP tool reuses that exact scope verbatim (no rename). Scope-mapping table added to W0. Closes Challenge HIGH #5 + L0 question #3.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:719:.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:720:.docs/plans/v1.4.7-waves.md:84:2. **`dailyos.read.briefing` → `dailyos.read.daily_briefing` + `dailyos.read.meeting_briefing`** in W0 scope mapping table (the actual W2-A tool surface ships TWO briefing tools, not one). Closes Eng MED.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:737:.docs/plans/v1.4.7-waves.md:263:   | (new) | `dailyos.read.account_status`, `dailyos.read.portfolio_attention`, `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing` (W2 lanes) |
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:745:.docs/plans/v1.4.7-waves.md:379:          pub name: ScopedName,                // e.g., "dailyos.read.account_status"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:755:.docs/plans/v1.4.7-waves.md:510:  - Read tools: verb is `read` (default), or one of `search` / `list` / `get` / `prepare` for action-shaped reads. E.g., `dailyos.read.account_status`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:760:.docs/plans/v1.4.7-waves.md:550:- **Done when:** `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` tools (canonical naming per cycle 2 #8) dispatch to ability-backed current-context queries; legacy snapshot read code paths removed (or wrapped with deprecation warnings); response shape includes provenance + freshness per ADR-0108; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:764:.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:765:.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:775:/bin/zsh -lc 'rg -n "AbilityRegistry|invoke_registry_json|McpAbilityBridge|McpToolHandler|mcp_exposure|allowed_actors|ADR-0102|ADR-0108|ADR-0125|ADR-0093|ADR-0128|claim sensitivity|provenance|briefing|meeting_briefing|get_daily_briefing|account_overview|substrate|handler seam|crate::abilities|abilities-runtime" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:895:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1074:.docs/decisions/0102-abilities-as-runtime-contract.md:248:pub struct SurfaceClientScope(pub String);      // Scope identifier — e.g., "read.account_overview", "submit.feedback",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1083:.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1112:.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1606:.docs/decisions/0111-surface-independent-ability-invocation.md:191:- **Scope grants.** An explicit enumerable set of scopes granted at pairing time (e.g., `read.account_overview`, `read.briefing`, `submit.feedback`, `manage.pairing`). Scope grants are *not* equivalent to host-OS user privileges or host-CMS site-wide capabilities. The host's own permission model (WP capabilities, OS user role, etc.) is one layer; DailyOS scopes are a separate, independent layer; both must pass for invocation.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:1639:- **Done when:** `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` tools (canonical naming per cycle 2 #8) dispatch to ability-backed current-context queries; legacy snapshot read code paths removed (or wrapped with deprecation warnings); response shape includes provenance + freshness per ADR-0108; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:16:K-in obligation: grep docs/solutions/ and .docs/decisions/ for ADR-0083 product vocabulary + ADR-0128 MCP product surface + DOS-478 catalog conventions. Also read src-tauri/resources/mcp_v2/tool_descriptions.yaml entries for account_status and daily_briefing.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:418:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:430:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:431:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:433:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:454:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:508:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:532:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:537:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:569:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:574:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:608:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:610:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:612:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:614:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:618:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:620:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:622:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:630:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:774:.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:952:/bin/zsh -lc 'rg -n "DOS-478|catalog conventions|catalog|when_to_call|tool description|Tool Catalog|account_status|daily_briefing" docs/solutions .docs/decisions src-tauri/resources/mcp_v2/tool_descriptions.yaml' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:954:/bin/zsh -lc 'rg -n "ADR-0128|ADR 0128|0128|MCP product surface|product surface|Claude Desktop|MCP|account_status|daily_briefing" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:958:src-tauri/resources/mcp_v2/tool_descriptions.yaml:16:- name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:960:src-tauri/resources/mcp_v2/tool_descriptions.yaml:31:    - dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:961:src-tauri/resources/mcp_v2/tool_descriptions.yaml:42:      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:962:src-tauri/resources/mcp_v2/tool_descriptions.yaml:46:      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:963:src-tauri/resources/mcp_v2/tool_descriptions.yaml:47:      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:964:src-tauri/resources/mcp_v2/tool_descriptions.yaml:54:- name: dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:966:src-tauri/resources/mcp_v2/tool_descriptions.yaml:67:    - dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:967:src-tauri/resources/mcp_v2/tool_descriptions.yaml:74:      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:968:src-tauri/resources/mcp_v2/tool_descriptions.yaml:78:      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:969:src-tauri/resources/mcp_v2/tool_descriptions.yaml:79:      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:971:src-tauri/resources/mcp_v2/tool_descriptions.yaml:121:      - { prompt: What's on my plate today overall?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:973:src-tauri/resources/mcp_v2/tool_descriptions.yaml:154:      - { prompt: What's going on with Acme specifically?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:975:src-tauri/resources/mcp_v2/tool_descriptions.yaml:190:      - { prompt: What's the current state of the Acme account?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:984:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:999:.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1033:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1188:.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1219:.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1403:- name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1418:    - dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1429:      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1433:      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1434:      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1441:- name: dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1454:    - dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1461:      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1465:      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1466:      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1521:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1687:.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1711:.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1713:.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1722:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:50:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1725:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:149:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1726:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:154:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1731:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:625:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1733:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:644:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1734:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:645:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1741:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:719:.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1743:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:764:.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1744:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md:765:.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1749:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:16:K-in obligation: grep docs/solutions/ and .docs/decisions/ for ADR-0083 product vocabulary + ADR-0128 MCP product surface + DOS-478 catalog conventions. Also read src-tauri/resources/mcp_v2/tool_descriptions.yaml entries for account_status and daily_briefing.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1752:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:433:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1755:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:532:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1756:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:537:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1762:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:952:/bin/zsh -lc 'rg -n "DOS-478|catalog conventions|catalog|when_to_call|tool description|Tool Catalog|account_status|daily_briefing" docs/solutions .docs/decisions src-tauri/resources/mcp_v2/tool_descriptions.yaml' in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1765:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:958:src-tauri/resources/mcp_v2/tool_descriptions.yaml:16:- name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1767:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:960:src-tauri/resources/mcp_v2/tool_descriptions.yaml:31:    - dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1768:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:961:src-tauri/resources/mcp_v2/tool_descriptions.yaml:42:      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1769:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:962:src-tauri/resources/mcp_v2/tool_descriptions.yaml:46:      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1770:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:963:src-tauri/resources/mcp_v2/tool_descriptions.yaml:47:      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1771:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:964:src-tauri/resources/mcp_v2/tool_descriptions.yaml:54:- name: dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1773:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:966:src-tauri/resources/mcp_v2/tool_descriptions.yaml:67:    - dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1774:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:967:src-tauri/resources/mcp_v2/tool_descriptions.yaml:74:      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1775:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:968:src-tauri/resources/mcp_v2/tool_descriptions.yaml:78:      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1776:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:969:src-tauri/resources/mcp_v2/tool_descriptions.yaml:79:      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1778:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:971:src-tauri/resources/mcp_v2/tool_descriptions.yaml:121:      - { prompt: What's on my plate today overall?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1780:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:973:src-tauri/resources/mcp_v2/tool_descriptions.yaml:154:      - { prompt: What's going on with Acme specifically?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1782:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:975:src-tauri/resources/mcp_v2/tool_descriptions.yaml:190:      - { prompt: What's the current state of the Acme account?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1802:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:292:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1805:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:391:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1806:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:396:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1839:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:680:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1842:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:779:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1843:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:784:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1868:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1825:    16	These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1871:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1924:   115	        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1872:.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1929:   120	        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1878:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1881:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:1882:.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2479:.docs/plans/dos-546/v1.4.2-project/02-issues.md:1254:* `dailyos/account-overview` ability declared via `#[ability]` macro with `category: Read`, `allowed_actors: [User, SurfaceClient]`, `required_scopes: [read.account_overview]` (per the W1-B compile-error gate), `mcp_exposure: McpExposure::Invocable`, `client_side_executable: true` (required for WP block hydration per W1-B SurfaceClientBridge gate; the field is independent of `mcp_exposure` per artifact 05 lines 389-412), `composition.produces_blocks: true`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2627:    16	- name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2642:    31	    - dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2653:    42	      invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2657:    46	      - { prompt: What's going on with Acme?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2658:    47	      - { prompt: How are things looking with the Hooli deal?, expectedTool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2665:    54	- name: dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2678:    67	    - dailyos.read.daily_briefing
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2685:    74	      invocation: { name: dailyos.read.daily_briefing, arguments: {} }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2689:    78	      - { prompt: What's on my plate today?, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2690:    79	      - { prompt: Give me my morning rundown, expectedTool: dailyos.read.daily_briefing }
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2704:     1	# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2716:    13	1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2717:    14	2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2719:    16	These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2740:    37	- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2794:    91	            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2818:   115	        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2823:   120	        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2855:   152	**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2860:   157	    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2894:   191	**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2896:   193	**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2898:   195	**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2900:   197	**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2904:   201	**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2906:   203	**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2908:   205	**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2916:   213	- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md:2950:    42	- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
src-tauri/src/services/mcp_v2/contracts.rs:27:/// `<NAMESPACE>.<verb>.<noun>` — for example `dailyos.read.account_status`.
src-tauri/src/services/mcp_v2/contracts.rs:440:        let scope = Scope::new("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/contracts.rs:442:        assert_eq!(encoded, "\"dailyos.read.account_status\"");
src-tauri/src/services/mcp_v2/contracts.rs:461:        let name = ScopedName::new("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/contracts.rs:463:        assert_eq!(encoded, "\"dailyos.read.account_status\"");
src-tauri/src/services/mcp_v2/contracts.rs:477:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:479:                Scope::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:490:                    "toolName": "dailyos.read.account_status",
src-tauri/src/services/mcp_v2/contracts.rs:491:                    "grantedScopes": ["dailyos.read.account_status", "read.entity_names"],
src-tauri/src/services/mcp_v2/contracts.rs:600:            name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:611:            scopes_required: vec![Scope::new("dailyos.read.account_status")],
src-tauri/src/services/mcp_v2/contracts.rs:670:            invocation: json!({ "name": "dailyos.read.account_status" }),
src-tauri/src/services/mcp_v2/contracts.rs:702:        let name = compose_scoped_name(Verb::Read, "account_status");
src-tauri/src/services/mcp_v2/contracts.rs:703:        assert_eq!(name.as_str(), "dailyos.read.account_status");
src-tauri/src/services/mcp_v2/contracts.rs:734:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:741:                "toolName": "dailyos.read.account_status",
src-tauri/src/services/mcp_v2/contracts.rs:753:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:762:                "toolName": "dailyos.read.account_status",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:277:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:289:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:290:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:292:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:313:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:367:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:391:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:396:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:428:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:433:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:467:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:469:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:471:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:473:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:477:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:479:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:481:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:489:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:630:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:667:.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:2119:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:32:    allowed_actors = [User, SurfaceClient],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:2127:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:748:        // Agent / Admin / System / SurfaceClient / McpClient — `get_daily_briefing`
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:2128:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:749:        // policy declares allowed_actors=[User] so the registry already gates
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md:2129:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:767:        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
src-tauri/src/services/mcp_v2/gateway.rs:828:        assert!(scope_is_subset(&[], &[Scope::new("dailyos.read.account_status")]));
src-tauri/src/services/mcp_v2/gateway.rs:834:        let granted = vec![Scope::new("dailyos.read.account_status")];
src-tauri/src/services/mcp_v2/gateway.rs:840:        let required = vec![Scope::new("dailyos.read.account_status")];
src-tauri/src/services/mcp_v2/gateway.rs:842:            Scope::new("dailyos.read.account_status"),
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
src-tauri/src/services/mcp_v2/transport.rs:367:        let desc = fake_desc("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/transport.rs:387:        let desc = fake_desc("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/transport.rs:398:            missing_scope: Scope::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/transport.rs:403:        assert_eq!(data["missing_scope"], "dailyos.read.account_status");
src-tauri/src/services/mcp_v2/transport.rs:420:            tool_name: ScopedName::new("dailyos.read.account_status"),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:20:- src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs around line 27 — current attributes
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:665:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:677:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:678:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:680:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:701:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:755:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:779:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:784:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:816:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:821:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:855:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:857:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:859:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:861:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:865:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:867:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:869:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:877:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:950:.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1075:/bin/zsh -lc "sed -n '1,95p' src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs" in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1132://! `get_daily_briefing` Read/User-only ability + composed
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1159:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1167:    required_scopes = ["read.daily_briefing"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1176:pub async fn get_daily_briefing(
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1180:    producer::build_daily_briefing(ctx, input).await
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1549:/bin/zsh -lc "nl -ba src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs | sed -n '20,55p'" in /private/tmp/dailyos-v147-w1a
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1566:    28	    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1574:    36	    required_scopes = ["read.daily_briefing"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1583:    45	pub async fn get_daily_briefing(
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1587:    49	    producer::build_daily_briefing(ctx, input).await
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1810:     1	# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1822:    13	1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1823:    14	2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1825:    16	These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1846:    37	- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1900:    91	            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1924:   115	        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1929:   120	        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1961:   152	**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:1966:   157	    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2000:   191	**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2002:   193	**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2004:   195	**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2006:   197	**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2010:   201	**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2012:   203	**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2014:   205	**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2022:   213	- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2054:    92	    required_scopes = ["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2092:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:32:    allowed_actors = [User, SurfaceClient],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2093:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:37:    mcp_exposure = None,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2094:src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:749:        // policy declares allowed_actors=[User] so the registry already gates
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2101:src-tauri/src/commands/version_dispatcher.rs:101:    let scope = SurfaceScope::new("read.account_overview");
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2107:src-tauri/src/services/surface_pairing.rs:4062:        assert!(!raw.contains("read.account_overview"));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2115:src-tauri/src/services/composition_render_orchestrator.rs:222:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")]).unwrap();
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2121:src-tauri/src/services/composition_render_orchestrator.rs:287:            SurfaceScope::new("read.account_overview"),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2152:src-tauri/abilities-runtime/src/abilities/account_overview.rs:92:    required_scopes = ["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2163:src-tauri/abilities-runtime/src/abilities/account_overview.rs:1274:                    "read.account_overview",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2165:src-tauri/abilities-runtime/src/abilities/account_overview.rs:1358:            &["read.account_overview"]
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2172:src-tauri/abilities-runtime/src/abilities/account_overview.rs:1633:                    "read.account_overview",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2173:src-tauri/abilities-runtime/src/abilities/registry.rs:76:/// vocabulary (e.g. `read.account_overview`, `read.composition`,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2187:src-tauri/abilities-runtime/src/abilities/registry.rs:893:            union.insert(SurfaceScope::new("read.account_overview"));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2197:src-tauri/abilities-runtime/src/abilities/registry.rs:2408:        let scope = SurfaceScope::new("read.account_overview");
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2198:src-tauri/abilities-runtime/src/abilities/registry.rs:2409:        assert_eq!(scope.as_str(), "read.account_overview");
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2199:src-tauri/abilities-runtime/src/abilities/registry.rs:2410:        assert_eq!(format!("{scope}"), "read.account_overview");
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2200:src-tauri/abilities-runtime/src/abilities/registry.rs:2582:    /// the canonical W1-A.1 fixture vocabulary (`read.account_overview`,
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2201:src-tauri/abilities-runtime/src/abilities/registry.rs:2588:        all.insert(SurfaceScope::new("read.account_overview"));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2202:src-tauri/abilities-runtime/src/abilities/registry.rs:2598:            scopes: scope_set(&["read.account_overview"]),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2203:src-tauri/abilities-runtime/src/abilities/registry.rs:2603:                assert!(scopes.contains(&SurfaceScope::new("read.account_overview")));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2204:src-tauri/abilities-runtime/src/abilities/registry.rs:2616:            scopes: scope_set(&["read.account_overview", "submit.feedback"]),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2205:src-tauri/abilities-runtime/src/abilities/registry.rs:2627:            scopes: scope_set(&["read.account_overview"]),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2206:src-tauri/abilities-runtime/src/abilities/registry.rs:2631:            scopes: scope_set(&["read.account_overview"]),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2209:src-tauri/abilities-runtime/src/abilities/registry.rs:2652:            scopes: scope_set(&["read.account_overview"]),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2210:src-tauri/abilities-runtime/src/abilities/registry.rs:2669:        seed_test_allowlist(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2211:src-tauri/abilities-runtime/src/abilities/registry.rs:2670:        let set = ScopeSet::new([SurfaceScope::new("read.account_overview")])
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2212:src-tauri/abilities-runtime/src/abilities/registry.rs:2672:        assert!(set.contains(&SurfaceScope::new("read.account_overview")));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2213:src-tauri/abilities-runtime/src/abilities/registry.rs:2690:        seed_test_allowlist(&["read.account_overview", "submit.feedback"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2214:src-tauri/abilities-runtime/src/abilities/registry.rs:2691:        let json = "[\"read.account_overview\",\"submit.feedback\"]";
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2215:src-tauri/abilities-runtime/src/abilities/registry.rs:2693:        assert!(decoded.contains(&SurfaceScope::new("read.account_overview")));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2219:src-tauri/abilities-runtime/src/abilities/registry.rs:2731:        seed_test_allowlist(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2220:src-tauri/abilities-runtime/src/abilities/registry.rs:2734:            scopes: ScopeSet::new([SurfaceScope::new("read.account_overview")])
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2221:src-tauri/abilities-runtime/src/abilities/registry.rs:2743:            required_scopes: &["read.account_overview", "submit.feedback"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2222:src-tauri/abilities-runtime/src/abilities/registry.rs:2748:        assert!(typed.contains(&SurfaceScope::new("read.account_overview")));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2227:src-tauri/abilities-runtime/src/abilities/registry.rs:2829:            required_scopes: &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2229:src-tauri/abilities-runtime/src/abilities/registry.rs:2834:        assert_eq!(POLICY.required_scopes, &["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2233:src-tauri/src/audit_log.rs:946:            SurfaceScope::new("read.account_overview"),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2235:src-tauri/src/audit_log.rs:981:        assert!(!line.contains("read.account_overview"));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2247:src-tauri/src/bridges/surface_client.rs:1300:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2248:src-tauri/src/bridges/surface_client.rs:1304:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2249:src-tauri/src/bridges/surface_client.rs:1361:                        &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2250:src-tauri/src/bridges/surface_client.rs:1365:            let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2251:src-tauri/src/bridges/surface_client.rs:1393:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2252:src-tauri/src/bridges/surface_client.rs:1399:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2253:src-tauri/src/bridges/surface_client.rs:1403:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2254:src-tauri/src/bridges/surface_client.rs:1435:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2255:src-tauri/src/bridges/surface_client.rs:1439:        let mut session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2256:src-tauri/src/bridges/surface_client.rs:1489:            actor_scopes: vec!["read.account_overview".to_string()],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2257:src-tauri/src/bridges/surface_client.rs:1515:            &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2258:src-tauri/src/bridges/surface_client.rs:1523:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2260:src-tauri/src/bridges/surface_client.rs:1547:            &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2261:src-tauri/src/bridges/surface_client.rs:1555:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2265:src-tauri/src/bridges/surface_client.rs:1577:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2268:src-tauri/src/bridges/surface_client.rs:1602:            &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2269:src-tauri/src/bridges/surface_client.rs:1610:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2275:src-tauri/src/bridges/surface_client.rs:1641:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2276:src-tauri/src/bridges/surface_client.rs:1645:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2280:src-tauri/src/bridges/surface_client.rs:1666:            &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2282:src-tauri/src/bridges/surface_client.rs:1719:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2283:src-tauri/src/bridges/surface_client.rs:1726:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2284:src-tauri/src/bridges/surface_client.rs:1754:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2285:src-tauri/src/bridges/surface_client.rs:1760:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2286:src-tauri/src/bridges/surface_client.rs:1766:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2287:src-tauri/src/bridges/surface_client.rs:1770:        let session = session(&["read.account_overview"]);
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2288:src-tauri/src/bridges/surface_client.rs:1811:                    &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2289:src-tauri/src/bridges/surface_client.rs:1816:        let session = Arc::new(session(&["read.account_overview"]));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2290:src-tauri/src/bridges/surface_client.rs:1857:                "read.account_overview",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2291:src-tauri/src/bridges/surface_client.rs:1862:            actor_scopes: vec!["read.account_overview".to_string()],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2292:src-tauri/src/bridges/surface_client.rs:1873:        assert!(!detail.contains("read.account_overview"));
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2298:src-tauri/src/bridges/correction_payload.rs:78:        scope == "read.account_overview"
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2304:src-tauri/src/surface_runtime/mod.rs:4399:                required_scopes: &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2308:src-tauri/src/surface_runtime/mod.rs:4492:                required_scopes: &["read.account_overview"],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2310:src-tauri/src/surface_runtime/mod.rs:4634:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2311:src-tauri/src/surface_runtime/mod.rs:4650:            granted_scopes: vec!["read.account_overview".to_string()],
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2312:src-tauri/src/surface_runtime/mod.rs:5024:            SurfaceScope::new("read.account_overview"),
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2321:src-tauri/src/surface_runtime/mod.rs:6426:        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")])
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2341:248:pub struct SurfaceClientScope(pub String);      // Scope identifier — e.g., "read.account_overview", "submit.feedback",
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md:2354:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:1:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:13:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:14:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:37:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:91:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:152:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:157:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:191:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:193:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:195:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:197:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:201:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:213:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/decisions/0111-surface-independent-ability-invocation.md:191:- **Scope grants.** An explicit enumerable set of scopes granted at pairing time (e.g., `read.account_overview`, `read.briefing`, `submit.feedback`, `manage.pairing`). Scope grants are *not* equivalent to host-OS user privileges or host-CMS site-wide capabilities. The host's own permission model (WP capabilities, OS user role, etc.) is one layer; DailyOS scopes are a separate, independent layer; both must pass for invocation.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:68:   - name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:84:     scopes_required: [dailyos.read.account_status]
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:92:         invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:96:         - { prompt: "What's going on with Acme?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:97:         - { prompt: "How are things looking with the Hooli deal?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:149:| 1 | `dailyos.read.account_status` | Read | `dailyos.read.account_status` | W2 (out-of-scope handler — discovery surface from DOS-172 ranked entity lookup) |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:150:| 2 | `dailyos.read.daily_briefing` | Read | `dailyos.read.daily_briefing` | W2-A (DOS-175) |
.docs/decisions/0102-abilities-as-runtime-contract.md:248:pub struct SurfaceClientScope(pub String);      // Scope identifier — e.g., "read.account_overview", "submit.feedback",
.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:6:**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:46:v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:47:- `dailyos.read.daily_briefing` — current-day briefing
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:73:2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:91:- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.

codex
The plan is mostly aligned with the intended seam, but I found a concrete registry-coupling problem: the proposed account handler uses the Rust function name, while the registry key is the attribute name. I’m checking the MCP v2 serve path once more for the sync/async bridge claim before finalizing.
exec
/bin/zsh -lc "rg -n \"struct McpActor|enum McpActor|McpActor::Client|project_actor|Actor::McpClient|MCP_CLIENT_ACTORS|Scope|grant|manifest|allowed_actors|mcp_exposure\" src-tauri/src/services/mcp_v2 src-tauri/abilities-runtime/src/abilities/registry.rs -g '*.rs'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "rg -n \"run_serve|Gateway::new|handle_tool_call|dispatch\\(|register\\(|seal\\(|tokio::runtime|Handle::current|block_on|serve\" src-tauri/src/mcp src-tauri/src/services/mcp_v2 src-tauri/src/bin src-tauri/src/main.rs src-tauri/src/lib.rs -g '*.rs'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/lib.rs:3:// functions are reserved for future use.
src-tauri/src/lib.rs:574:                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:576:                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:582:                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:603:                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:605:                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:619:                        #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/lib.rs:667:                        #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
src-tauri/src/mcp/main.rs:14:use rmcp::handler::server::tool::ToolCallContext;
src-tauri/src/mcp/main.rs:41:/// Read-only MCP server for DailyOS workspace intelligence.
src-tauri/src/mcp/main.rs:724:    // Keep this descriptor-based listing until the deferred MCP server migration
src-tauri/src/mcp/main.rs:978:            server_info: Implementation {
src-tauri/src/mcp/main.rs:983:                "DailyOS MCP server. Provides read-only access to your daily briefing, \
src-tauri/src/mcp/main.rs:1312:/// The MCP server communicates over stdio. Any writes to stdout before rmcp
src-tauri/src/mcp/main.rs:1335:    // Initialize embedding model synchronously — MCP server starts before any
src-tauri/src/mcp/main.rs:1370:    let server = DailyOsMcp::new(
src-tauri/src/mcp/main.rs:1379:    let service = server.serve(rmcp::transport::io::stdio()).await?;
src-tauri/src/mcp/main.rs:1533:    fn mcp_serverhandler_tool_box_macro_removed_and_manual_routes_static_or_ability() {
src-tauri/src/mcp/main.rs:1538:        let serverhandler_tool_box =
src-tauri/src/mcp/main.rs:1542:        assert!(!source.contains(serverhandler_tool_box));
src-tauri/src/services/mcp_v2/taxonomy.rs:41:/// boot via `Gateway::seal()` returning `Err(...)`; no `panic!()`.
src-tauri/src/services/mcp_v2/taxonomy.rs:193:    /// catalog entry with matching `Side`. Production `Gateway::seal()`
src-tauri/src/services/mcp_v2/gateway.rs:3://! Validates Actor::McpClient policy against the server-side scope manifest
src-tauri/src/services/mcp_v2/gateway.rs:177:    pub fn register(&mut self, handler: Arc<dyn McpToolHandler>) {
src-tauri/src/services/mcp_v2/gateway.rs:198:    /// Production `main.rs` calls `gateway.seal()?` after registration.
src-tauri/src/services/mcp_v2/gateway.rs:200:    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
src-tauri/src/services/mcp_v2/gateway.rs:216:    pub fn handle_tool_call(
src-tauri/src/services/mcp_v2/gateway.rs:224:        match self.dispatch(conn, asserted_client_id, &envelope, signature) {
src-tauri/src/services/mcp_v2/gateway.rs:271:    fn dispatch(
src-tauri/src/services/mcp_v2/gateway.rs:426:        // Rate limit per AC-5: BEGIN IMMEDIATE → prune → count → reserve OR ROLLBACK.
src-tauri/src/services/mcp_v2/gateway.rs:427:        match reserve_rate_limit(conn, asserted_client_id, &envelope.tool_name, &grant) {
src-tauri/src/services/mcp_v2/gateway.rs:638:/// stderr server-side so operators can grep by trace_id.
src-tauri/src/services/mcp_v2/gateway.rs:715:fn reserve_rate_limit(
src-tauri/src/services/mcp_v2/gateway.rs:812:    eprintln!("mcp.alert.audit_double_failure: handler effects preserved, audit lost");
src-tauri/src/services/mcp_v2/transport.rs:18://!   [`Gateway::handle_tool_call`]. `_dailyos_*` keys are NOT exposed
src-tauri/src/services/mcp_v2/transport.rs:144:            server_info: Implementation {
src-tauri/src/services/mcp_v2/transport.rs:235:            self.gateway.handle_tool_call(
src-tauri/src/services/mcp_v2/transport.rs:276:            // logged server-side only by the gateway audit path.
src-tauri/src/services/mcp_v2/transport.rs:281:                    "detail_logged_server_side": true,
src-tauri/src/services/mcp_v2/transport.rs:326:            // detail NOT included on wire per AC-3; logged server-side.
src-tauri/src/services/mcp_v2/transport.rs:331:                    "detail_logged_server_side": true,
src-tauri/src/services/mcp_v2/transport.rs:430:            detail: "secret-server-side-detail".into(),
src-tauri/src/services/mcp_v2/transport.rs:434:            !data_str.contains("secret-server-side-detail"),
src-tauri/src/services/mcp_v2/transport.rs:437:        assert!(data_str.contains("detail_logged_server_side"));
src-tauri/src/services/mcp_v2/auth.rs:65:/// Operator-supplied pairing input that the MCP server uses to mint a new
src-tauri/src/services/mcp_v2/auth.rs:70:/// This W1-A scope produces the server-side state needed to admit a future
src-tauri/src/services/mcp_v2/auth.rs:76:    /// stored for operator visibility; never serves auth.
src-tauri/src/services/mcp_v2/auth.rs:84:/// the server retains only the `KeychainRef` and never the raw bytes.
src-tauri/src/services/mcp_v2/auth.rs:90:    /// leaves the server.
src-tauri/src/services/mcp_v2/auth.rs:317:/// Resolve presented handle to a server-side record (or mint a new one if
src-tauri/src/services/mcp_v2/auth.rs:517:/// Used by the W1.5 transport binary at `serve` startup to bootstrap
src-tauri/src/services/mcp_v2/actor_policy.rs:67:/// ADR-0102 asymmetry this module exists to preserve.
src-tauri/src/services/mcp_v2/mod.rs:19://! - auth — pairing handshake; server-side scope manifest loading;
src-tauri/src/services/mcp_v2/contracts.rs:217:/// the server-side ledger keys nonces by `(client_id, nonce)` and tracks
src-tauri/src/services/mcp_v2/contracts.rs:221:/// request envelope, server returns `next_request_nonce` on the response
src-tauri/src/services/mcp_v2/contracts.rs:264:/// resolved/hydrated gateway-side envelope; carries granted_scopes because the gateway loads them from the server-side manifest before dispatching. Distinct from the abilities-runtime Actor::McpClient variant which carries no scopes per ADR-0102 amendment §B asymmetry.
src-tauri/src/services/mcp_v2/contracts.rs:358:/// sentinel strings: the [`Scope`] type is reserved for
src-tauri/src/services/mcp_v2/contracts.rs:595:        // `when_NOT_to_call` deliberately preserves the uppercase NOT
src-tauri/src/services/mcp_v2/contracts.rs:843:        // strings; the server-side ledger keys them by (client_id,

 succeeded in 0ms:
src-tauri/abilities-runtime/src/abilities/registry.rs:40:/// `SurfaceClientBridge` consumes it for per-instance scope grants.
src-tauri/abilities-runtime/src/abilities/registry.rs:66:/// A named scope grant a [`Actor::SurfaceClient`] instance carries into
src-tauri/abilities-runtime/src/abilities/registry.rs:71:/// `required_scopes: Vec<SurfaceScope>`; the `SurfaceClientBridge` will
src-tauri/abilities-runtime/src/abilities/registry.rs:72:/// enforce that every required scope is present in the instance's grant
src-tauri/abilities-runtime/src/abilities/registry.rs:78:/// substrate recompile, while [`ScopeSet`] enforces a runtime-registered
src-tauri/abilities-runtime/src/abilities/registry.rs:86:pub struct SurfaceScope(String);
src-tauri/abilities-runtime/src/abilities/registry.rs:88:impl SurfaceScope {
src-tauri/abilities-runtime/src/abilities/registry.rs:100:impl std::fmt::Display for SurfaceScope {
src-tauri/abilities-runtime/src/abilities/registry.rs:106:/// Stable, non-PII identifier for a paired [`Actor::McpClient`] instance.
src-tauri/abilities-runtime/src/abilities/registry.rs:114:/// Unlike [`Actor::SurfaceClient`], the [`Actor::McpClient`] variant
src-tauri/abilities-runtime/src/abilities/registry.rs:115:/// carries no [`ScopeSet`] — the gateway enforces scopes against a
src-tauri/abilities-runtime/src/abilities/registry.rs:116:/// server-side per-client manifest before invocation per the ADR-0102
src-tauri/abilities-runtime/src/abilities/registry.rs:145:/// conversation continuity for an [`Actor::McpClient`] invocation path.
src-tauri/abilities-runtime/src/abilities/registry.rs:153:/// The corresponding field on [`Actor::McpClient`] is `Option<_>`
src-tauri/abilities-runtime/src/abilities/registry.rs:179:/// Construction / deserialization errors for [`ScopeSet`].
src-tauri/abilities-runtime/src/abilities/registry.rs:181:pub enum ScopeSetError {
src-tauri/abilities-runtime/src/abilities/registry.rs:188:    UnknownScopes(Vec<SurfaceScope>),
src-tauri/abilities-runtime/src/abilities/registry.rs:191:impl std::fmt::Display for ScopeSetError {
src-tauri/abilities-runtime/src/abilities/registry.rs:194:            ScopeSetError::Empty => formatter.write_str(
src-tauri/abilities-runtime/src/abilities/registry.rs:195:                "ScopeSet construction rejected: SurfaceClient requires at least one scope",
src-tauri/abilities-runtime/src/abilities/registry.rs:197:            ScopeSetError::UnknownScopes(scopes) => {
src-tauri/abilities-runtime/src/abilities/registry.rs:200:                    "ScopeSet construction rejected: unknown scope(s) outside the registered allowlist: {}",
src-tauri/abilities-runtime/src/abilities/registry.rs:203:                        .map(SurfaceScope::as_str)
src-tauri/abilities-runtime/src/abilities/registry.rs:212:impl std::error::Error for ScopeSetError {}
src-tauri/abilities-runtime/src/abilities/registry.rs:219:///   [`ScopeSet`] construction and deserialization. An explicitly seeded
src-tauri/abilities-runtime/src/abilities/registry.rs:224:///   [`ScopeSet::initialize_allowlist`] wins and flips
src-tauri/abilities-runtime/src/abilities/registry.rs:228:/// - **Tests:** [`ScopeSet::set_allowlist_for_tests`] overwrites the lock
src-tauri/abilities-runtime/src/abilities/registry.rs:235:static SCOPE_ALLOWLIST: RwLock<Option<BTreeSet<SurfaceScope>>> = RwLock::new(None);
src-tauri/abilities-runtime/src/abilities/registry.rs:238:/// [`ScopeSet::initialize_allowlist`] to reject double-initialization without
src-tauri/abilities-runtime/src/abilities/registry.rs:240:/// [`ScopeSet::set_allowlist_for_tests`], which both writes the lock and
src-tauri/abilities-runtime/src/abilities/registry.rs:244:/// A typed, non-empty set of [`SurfaceScope`] values carried by every
src-tauri/abilities-runtime/src/abilities/registry.rs:250:/// 1. **Non-empty.** An empty grant is a misconfiguration; see
src-tauri/abilities-runtime/src/abilities/registry.rs:251:///    [`ScopeSetError::Empty`].
src-tauri/abilities-runtime/src/abilities/registry.rs:258:/// Deserialization (via `serde`) routes through [`ScopeSet::new`] so both
src-tauri/abilities-runtime/src/abilities/registry.rs:264:pub struct ScopeSet(BTreeSet<SurfaceScope>);
src-tauri/abilities-runtime/src/abilities/registry.rs:266:impl ScopeSet {
src-tauri/abilities-runtime/src/abilities/registry.rs:267:    /// Construct a [`ScopeSet`] from any iterator of [`SurfaceScope`].
src-tauri/abilities-runtime/src/abilities/registry.rs:269:    /// Returns [`ScopeSetError::Empty`] if the resulting set is empty
src-tauri/abilities-runtime/src/abilities/registry.rs:274:    /// Returns [`ScopeSetError::UnknownScopes`] when the global allowlist is
src-tauri/abilities-runtime/src/abilities/registry.rs:276:    pub fn new(scopes: impl IntoIterator<Item = SurfaceScope>) -> Result<Self, ScopeSetError> {
src-tauri/abilities-runtime/src/abilities/registry.rs:277:        let set: BTreeSet<SurfaceScope> = scopes.into_iter().collect();
src-tauri/abilities-runtime/src/abilities/registry.rs:279:            return Err(ScopeSetError::Empty);
src-tauri/abilities-runtime/src/abilities/registry.rs:285:            let unknown: Vec<SurfaceScope> = set
src-tauri/abilities-runtime/src/abilities/registry.rs:291:                return Err(ScopeSetError::UnknownScopes(unknown));
src-tauri/abilities-runtime/src/abilities/registry.rs:299:    pub fn contains(&self, scope: &SurfaceScope) -> bool {
src-tauri/abilities-runtime/src/abilities/registry.rs:304:    pub fn iter(&self) -> impl Iterator<Item = &SurfaceScope> {
src-tauri/abilities-runtime/src/abilities/registry.rs:308:    /// Always returns `false` — a [`ScopeSet`] cannot be empty by
src-tauri/abilities-runtime/src/abilities/registry.rs:331:        scopes: impl IntoIterator<Item = SurfaceScope>,
src-tauri/abilities-runtime/src/abilities/registry.rs:332:    ) -> Result<(), BTreeSet<SurfaceScope>> {
src-tauri/abilities-runtime/src/abilities/registry.rs:333:        let set: BTreeSet<SurfaceScope> = scopes.into_iter().collect();
src-tauri/abilities-runtime/src/abilities/registry.rs:362:    pub fn set_allowlist_for_tests(scopes: impl IntoIterator<Item = SurfaceScope>) {
src-tauri/abilities-runtime/src/abilities/registry.rs:381:impl Serialize for ScopeSet {
src-tauri/abilities-runtime/src/abilities/registry.rs:390:impl<'de> Deserialize<'de> for ScopeSet {
src-tauri/abilities-runtime/src/abilities/registry.rs:395:        let raw = BTreeSet::<SurfaceScope>::deserialize(deserializer)?;
src-tauri/abilities-runtime/src/abilities/registry.rs:396:        ScopeSet::new(raw).map_err(serde::de::Error::custom)
src-tauri/abilities-runtime/src/abilities/registry.rs:404:/// instance identity AND its scope grant as a struct variant per ADR-0111
src-tauri/abilities-runtime/src/abilities/registry.rs:410:/// an owned [`SurfaceClientId`] (`String`) and [`ScopeSet`] (`BTreeSet`).
src-tauri/abilities-runtime/src/abilities/registry.rs:420:    /// See ADR-0111 §8. Both identity and scope grant ride on the variant;
src-tauri/abilities-runtime/src/abilities/registry.rs:427:        /// The scope grant this instance carries for this request. Always
src-tauri/abilities-runtime/src/abilities/registry.rs:428:        /// non-empty by [`ScopeSet`] construction. Surfaces audit emission's
src-tauri/abilities-runtime/src/abilities/registry.rs:430:        scopes: ScopeSet,
src-tauri/abilities-runtime/src/abilities/registry.rs:437:    /// scope manifest (see amendment §B for the asymmetry with
src-tauri/abilities-runtime/src/abilities/registry.rs:439:    /// [`ScopeSet`] — the gateway is the source of truth for what scopes a
src-tauri/abilities-runtime/src/abilities/registry.rs:454:/// any per-invocation instance data. Used in [`AbilityPolicy::allowed_actors`]
src-tauri/abilities-runtime/src/abilities/registry.rs:460:/// owned [`SurfaceClientId`] and [`ScopeSet`] data, so it cannot itself be
src-tauri/abilities-runtime/src/abilities/registry.rs:480:    /// Mirrors [`Actor::McpClient`]. Per-invocation client identity and
src-tauri/abilities-runtime/src/abilities/registry.rs:484:    /// per-client manifest and do not ride on the variant.
src-tauri/abilities-runtime/src/abilities/registry.rs:492:    /// `descriptor.policy.allowed_actors.contains(&actor.kind())`.
src-tauri/abilities-runtime/src/abilities/registry.rs:500:            Actor::McpClient { .. } => ActorKind::McpClient,
src-tauri/abilities-runtime/src/abilities/registry.rs:571:///   for `inventory::submit!`. Callers that need typed [`SurfaceScope`]
src-tauri/abilities-runtime/src/abilities/registry.rs:573:///   reconciles AC line 446's informal `Vec<SurfaceClientScope>` with the
src-tauri/abilities-runtime/src/abilities/registry.rs:575:/// - `mcp_exposure`: tri-state MCP enumeration tier (see [`McpExposure`]).
src-tauri/abilities-runtime/src/abilities/registry.rs:577:///   policy/scope/actor checks. Independent of `mcp_exposure` per Phase 0
src-tauri/abilities-runtime/src/abilities/registry.rs:582:/// - `allowed_actors: &[ActorKind::User]` — least-privilege actor floor
src-tauri/abilities-runtime/src/abilities/registry.rs:585:/// - `mcp_exposure: McpExposure::None`
src-tauri/abilities-runtime/src/abilities/registry.rs:590:/// `SurfaceClient` in `allowed_actors` may keep empty `required_scopes`,
src-tauri/abilities-runtime/src/abilities/registry.rs:592:/// `allowed_actors` includes `SurfaceClient`.
src-tauri/abilities-runtime/src/abilities/registry.rs:601:    pub allowed_actors: &'static [ActorKind],
src-tauri/abilities-runtime/src/abilities/registry.rs:605:    /// Scope vocabulary a [`Actor::SurfaceClient`] must carry before this
src-tauri/abilities-runtime/src/abilities/registry.rs:610:    pub mcp_exposure: McpExposure,
src-tauri/abilities-runtime/src/abilities/registry.rs:623:    /// `mcp_exposure: McpExposure::None`, `client_side_executable: false`,
src-tauri/abilities-runtime/src/abilities/registry.rs:627:            allowed_actors: &[ActorKind::User],
src-tauri/abilities-runtime/src/abilities/registry.rs:632:            mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:640:    /// Materialize `required_scopes` as typed [`SurfaceScope`] values.
src-tauri/abilities-runtime/src/abilities/registry.rs:645:    /// for [`ScopeSet`] enforcement (W2-B) calls this helper.
src-tauri/abilities-runtime/src/abilities/registry.rs:646:    pub fn required_scopes_typed(&self) -> Vec<SurfaceScope> {
src-tauri/abilities-runtime/src/abilities/registry.rs:649:            .map(|s| SurfaceScope::new(*s))
src-tauri/abilities-runtime/src/abilities/registry.rs:880:            // W1-B: seed the global SurfaceScope allowlist with the union
src-tauri/abilities-runtime/src/abilities/registry.rs:885:            // scope at ScopeSet construction; once seeded here, unknown
src-tauri/abilities-runtime/src/abilities/registry.rs:888:            let mut union: BTreeSet<SurfaceScope> = by_name
src-tauri/abilities-runtime/src/abilities/registry.rs:891:                .map(|s| SurfaceScope::new(*s))
src-tauri/abilities-runtime/src/abilities/registry.rs:893:            union.insert(SurfaceScope::new("read.account_overview"));
src-tauri/abilities-runtime/src/abilities/registry.rs:894:            union.insert(SurfaceScope::new("read.composition"));
src-tauri/abilities-runtime/src/abilities/registry.rs:895:            union.insert(SurfaceScope::new("submit.feedback"));
src-tauri/abilities-runtime/src/abilities/registry.rs:899:            if ScopeSet::initialize_allowlist(union).is_err() {
src-tauri/abilities-runtime/src/abilities/registry.rs:939:            descriptor.policy.allowed_actors.contains(&actor.kind())
src-tauri/abilities-runtime/src/abilities/registry.rs:1474:    if !descriptor.policy.allowed_actors.contains(&ctx.actor.kind()) {
src-tauri/abilities-runtime/src/abilities/registry.rs:1570:        "allowed_actors",
src-tauri/abilities-runtime/src/abilities/registry.rs:1573:            .allowed_actors
src-tauri/abilities-runtime/src/abilities/registry.rs:1731:                allowed_actors: &[],
src-tauri/abilities-runtime/src/abilities/registry.rs:1736:                mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:1761:                allowed_actors: &[ActorKind::Agent, ActorKind::User, ActorKind::System],
src-tauri/abilities-runtime/src/abilities/registry.rs:1770:                mcp_exposure: McpExposure::None,
src-tauri/abilities-runtime/src/abilities/registry.rs:1820:        descriptor.policy.allowed_actors = static_slice(actors);
src-tauri/abilities-runtime/src/abilities/registry.rs:2408:        let scope = SurfaceScope::new("read.account_overview");
src-tauri/abilities-runtime/src/abilities/registry.rs:2415:        let scope = SurfaceScope::new("write.feedback");
src-tauri/abilities-runtime/src/abilities/registry.rs:2418:        let decoded: SurfaceScope = serde_json::from_str(&encoded).expect("deserializes");
src-tauri/abilities-runtime/src/abilities/registry.rs:2426:        set.insert(SurfaceScope::new("read.x"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2427:        set.insert(SurfaceScope::new("write.y"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2428:        set.insert(SurfaceScope::new("read.x")); // dup
src-tauri/abilities-runtime/src/abilities/registry.rs:2433:    // McpClient actor class — identity / handle newtype + Actor::McpClient
src-tauri/abilities-runtime/src/abilities/registry.rs:2504:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2513:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2522:        // Locks the JSON wire shape of `Actor::McpClient` so any future
src-tauri/abilities-runtime/src/abilities/registry.rs:2526:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2546:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2564:    /// Test helper: build a non-empty [`ScopeSet`] from string slices.
src-tauri/abilities-runtime/src/abilities/registry.rs:2575:    fn scope_set(scopes: &[&str]) -> ScopeSet {
src-tauri/abilities-runtime/src/abilities/registry.rs:2577:        ScopeSet::new(scopes.iter().map(|s| SurfaceScope::new(*s)))
src-tauri/abilities-runtime/src/abilities/registry.rs:2586:        let mut all: BTreeSet<SurfaceScope> =
src-tauri/abilities-runtime/src/abilities/registry.rs:2587:            scopes.iter().map(|s| SurfaceScope::new(*s)).collect();
src-tauri/abilities-runtime/src/abilities/registry.rs:2588:        all.insert(SurfaceScope::new("read.account_overview"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2589:        all.insert(SurfaceScope::new("read.composition"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2590:        all.insert(SurfaceScope::new("submit.feedback"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2591:        ScopeSet::set_allowlist_for_tests(all);
src-tauri/abilities-runtime/src/abilities/registry.rs:2603:                assert!(scopes.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2642:    fn actor_surface_client_not_in_user_agent_allowed_actors() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2644:        // `allowed_actors: [User, Agent]` must NOT match a SurfaceClient
src-tauri/abilities-runtime/src/abilities/registry.rs:2658:    // W1-A.1: ScopeSet construction + deserialization invariants
src-tauri/abilities-runtime/src/abilities/registry.rs:2663:        let result = ScopeSet::new(std::iter::empty::<SurfaceScope>());
src-tauri/abilities-runtime/src/abilities/registry.rs:2664:        assert!(matches!(result, Err(ScopeSetError::Empty)));
src-tauri/abilities-runtime/src/abilities/registry.rs:2670:        let set = ScopeSet::new([SurfaceScope::new("read.account_overview")])
src-tauri/abilities-runtime/src/abilities/registry.rs:2672:        assert!(set.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2673:        assert!(!set.contains(&SurfaceScope::new("submit.feedback")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2680:        let err = serde_json::from_str::<ScopeSet>("[]").expect_err("empty must fail");
src-tauri/abilities-runtime/src/abilities/registry.rs:2692:        let decoded: ScopeSet = serde_json::from_str(json).expect("decodes");
src-tauri/abilities-runtime/src/abilities/registry.rs:2693:        assert!(decoded.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2694:        assert!(decoded.contains(&SurfaceScope::new("submit.feedback")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2697:        let redecoded: ScopeSet = serde_json::from_str(&encoded).expect("re-decodes");
src-tauri/abilities-runtime/src/abilities/registry.rs:2702:    // W1-B: AbilityPolicy schema (required_scopes + mcp_exposure +
src-tauri/abilities-runtime/src/abilities/registry.rs:2713:        assert_eq!(policy.allowed_actors, &[ActorKind::User]);
src-tauri/abilities-runtime/src/abilities/registry.rs:2719:        assert_eq!(policy.mcp_exposure, McpExposure::None);
src-tauri/abilities-runtime/src/abilities/registry.rs:2734:            scopes: ScopeSet::new([SurfaceScope::new("read.account_overview")])
src-tauri/abilities-runtime/src/abilities/registry.rs:2748:        assert!(typed.contains(&SurfaceScope::new("read.account_overview")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2749:        assert!(typed.contains(&SurfaceScope::new("submit.feedback")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2784:    fn mcp_exposure_default_is_none() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2789:    fn mcp_exposure_serde_round_trip_for_all_variants() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2802:    fn mcp_exposure_wire_form_is_snake_case() {
src-tauri/abilities-runtime/src/abilities/registry.rs:2825:            allowed_actors: &[],
src-tauri/abilities-runtime/src/abilities/registry.rs:2830:            mcp_exposure: McpExposure::None,
src-tauri/src/services/mcp_v2/taxonomy.rs:23:    McpToolHandler, ParamSpec, ReturnSpec, Scope, ScopedName, Side, ToolDescription, ToolExample,
src-tauri/src/services/mcp_v2/taxonomy.rs:44:    /// A registered handler's `ScopedName` has no matching entry in
src-tauri/src/services/mcp_v2/taxonomy.rs:51:        handler: ScopedName,
src-tauri/src/services/mcp_v2/taxonomy.rs:52:        catalog_entry: Option<ScopedName>,
src-tauri/src/services/mcp_v2/taxonomy.rs:53:        nearest_candidate: Option<ScopedName>,
src-tauri/src/services/mcp_v2/taxonomy.rs:67:    InvalidScope { tool: ScopedName, scope: Scope },
src-tauri/src/services/mcp_v2/taxonomy.rs:70:    DuplicateName { name: ScopedName },
src-tauri/src/services/mcp_v2/taxonomy.rs:75:        handler: ScopedName,
src-tauri/src/services/mcp_v2/taxonomy.rs:83:        tool: ScopedName,
src-tauri/src/services/mcp_v2/taxonomy.rs:124:            Self::InvalidScope { tool, scope } => write!(
src-tauri/src/services/mcp_v2/taxonomy.rs:207:    ) -> Vec<ScopedName>;
src-tauri/src/services/mcp_v2/taxonomy.rs:210:    fn side_for(&self, tool_name: &ScopedName) -> Option<Side>;
src-tauri/src/services/mcp_v2/taxonomy.rs:215:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription>;
src-tauri/src/services/mcp_v2/taxonomy.rs:217:    /// Iterate every catalog entry's `ScopedName`. Used by transport
src-tauri/src/services/mcp_v2/taxonomy.rs:219:    /// manifest grants.
src-tauri/src/services/mcp_v2/taxonomy.rs:220:    fn iter_names(&self) -> Box<dyn Iterator<Item = &ScopedName> + '_>;
src-tauri/src/services/mcp_v2/taxonomy.rs:233:    pub name: ScopedName,
src-tauri/src/services/mcp_v2/taxonomy.rs:240:    pub scopes_required: Vec<Scope>,
src-tauri/src/services/mcp_v2/taxonomy.rs:278:    pub expected_tool: Option<ScopedName>,
src-tauri/src/services/mcp_v2/taxonomy.rs:287:    pub expected_tool: ScopedName,
src-tauri/src/services/mcp_v2/taxonomy.rs:315:    entries: HashMap<ScopedName, (ToolDescription, SelectionFixtures)>,
src-tauri/src/services/mcp_v2/taxonomy.rs:354:            // Scope allowlist.
src-tauri/src/services/mcp_v2/taxonomy.rs:357:                    return Err(TaxonomyError::InvalidScope {
src-tauri/src/services/mcp_v2/taxonomy.rs:397:    pub fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:401:    pub fn fixtures_for(&self, name: &ScopedName) -> Option<&SelectionFixtures> {
src-tauri/src/services/mcp_v2/taxonomy.rs:407:    ) -> impl Iterator<Item = (&ScopedName, &ToolDescription, &SelectionFixtures)> {
src-tauri/src/services/mcp_v2/taxonomy.rs:452:    ) -> Vec<ScopedName> {
src-tauri/src/services/mcp_v2/taxonomy.rs:453:        let handler_names: std::collections::HashSet<&ScopedName> =
src-tauri/src/services/mcp_v2/taxonomy.rs:462:    fn side_for(&self, tool_name: &ScopedName) -> Option<Side> {
src-tauri/src/services/mcp_v2/taxonomy.rs:466:    fn description_for(&self, tool_name: &ScopedName) -> Option<&ToolDescription> {
src-tauri/src/services/mcp_v2/taxonomy.rs:470:    fn iter_names(&self) -> Box<dyn Iterator<Item = &ScopedName> + '_> {
src-tauri/src/services/mcp_v2/taxonomy.rs:478:    fn nearest_name(&self, needle: &str) -> Option<ScopedName> {
src-tauri/src/services/mcp_v2/taxonomy.rs:486:fn scope_in_allowlist(scope: &Scope) -> bool {
src-tauri/src/services/mcp_v2/taxonomy.rs:559:                catalog.description_for(&ScopedName::new(name)).is_some(),
src-tauri/src/services/mcp_v2/taxonomy.rs:606:            Err(TaxonomyError::InvalidScope { .. }) => {}
src-tauri/src/services/mcp_v2/taxonomy.rs:607:            other => panic!("expected InvalidScope, got {other:?}"),
src-tauri/src/services/mcp_v2/taxonomy.rs:702:            Some(ScopedName::new("dailyos.read.account_status")),
src-tauri/src/services/mcp_v2/actor_policy.rs:3://! ADR-0102 §B makes MCP asymmetric with `SurfaceClient`: MCP grants are
src-tauri/src/services/mcp_v2/actor_policy.rs:6://! the small adapter between the MCP manifest records and the abilities
src-tauri/src/services/mcp_v2/actor_policy.rs:16:use super::contracts::{McpClientId, OpaqueConversationHandle, Scope, ScopedName};
src-tauri/src/services/mcp_v2/actor_policy.rs:18:const MCP_CLIENT_ACTORS: &[ActorKind] = &[ActorKind::McpClient];
src-tauri/src/services/mcp_v2/actor_policy.rs:33:/// Per-tool invocation rate limit from the MCP client manifest.
src-tauri/src/services/mcp_v2/actor_policy.rs:40:/// Per-call manifest grant resolved by indexed `(client_id, tool_name)` lookup.
src-tauri/src/services/mcp_v2/actor_policy.rs:43:    pub tool_name: ScopedName,
src-tauri/src/services/mcp_v2/actor_policy.rs:44:    pub scopes_granted: Vec<Scope>,
src-tauri/src/services/mcp_v2/actor_policy.rs:49:/// Project a manifest grant into the runtime policy shell used by the
src-tauri/src/services/mcp_v2/actor_policy.rs:52:/// MCP scope checks are gateway-mediated against [`ToolGrant::scopes_granted`].
src-tauri/src/services/mcp_v2/actor_policy.rs:54:pub fn project_policy(tool_grant: &ToolGrant) -> AbilityPolicy {
src-tauri/src/services/mcp_v2/actor_policy.rs:56:        allowed_actors: MCP_CLIENT_ACTORS,
src-tauri/src/services/mcp_v2/actor_policy.rs:57:        mcp_exposure: tool_grant.exposure,
src-tauri/src/services/mcp_v2/actor_policy.rs:64:/// `tool_grant` is intentionally accepted here even though scopes are not put
src-tauri/src/services/mcp_v2/actor_policy.rs:66:/// as "manifest resolved before runtime actor construction", which is the
src-tauri/src/services/mcp_v2/actor_policy.rs:68:pub fn project_actor(
src-tauri/src/services/mcp_v2/actor_policy.rs:70:    tool_grant: &ToolGrant,
src-tauri/src/services/mcp_v2/actor_policy.rs:73:    let _ = project_policy(tool_grant);
src-tauri/src/services/mcp_v2/actor_policy.rs:74:    Actor::McpClient {
src-tauri/src/services/mcp_v2/contracts.rs:6:    pub name: ScopedName,
src-tauri/src/services/mcp_v2/contracts.rs:15:    pub scopes_required: Vec<Scope>,
src-tauri/src/services/mcp_v2/contracts.rs:38:/// validation against the per-client manifest is the gateway's
src-tauri/src/services/mcp_v2/contracts.rs:42:/// §E. [`Scope`] itself stays a permissive newtype to keep that
src-tauri/src/services/mcp_v2/contracts.rs:59:    /// across releases; used by the gateway's manifest validator and by
src-tauri/src/services/mcp_v2/contracts.rs:74:    /// manifest allowlist + W1-B taxonomy validation pass.
src-tauri/src/services/mcp_v2/contracts.rs:92:/// Compose a canonical [`ScopedName`] from the namespace prefix, a verb,
src-tauri/src/services/mcp_v2/contracts.rs:94:pub fn compose_scoped_name(verb: Verb, noun: &str) -> ScopedName {
src-tauri/src/services/mcp_v2/contracts.rs:95:    ScopedName::new(format!(
src-tauri/src/services/mcp_v2/contracts.rs:103:/// Compose a canonical [`Scope`] from the namespace prefix, a verb,
src-tauri/src/services/mcp_v2/contracts.rs:105:pub fn compose_scope(verb: Verb, noun: &str) -> Scope {
src-tauri/src/services/mcp_v2/contracts.rs:106:    Scope::new(format!(
src-tauri/src/services/mcp_v2/contracts.rs:117:pub struct ScopedName(pub String);
src-tauri/src/services/mcp_v2/contracts.rs:119:impl ScopedName {
src-tauri/src/services/mcp_v2/contracts.rs:129:impl std::fmt::Display for ScopedName {
src-tauri/src/services/mcp_v2/contracts.rs:135:/// canonical form dailyos.<verb>.<noun> for new tools; pre-2026-05-19 substrate-shipped scopes — read.workspace_graph, write.workspace_place_document, read.entity_names — stay unprefixed verbatim per ADR-0102 amendment §E; manifest validation rejects out-of-allowlist.
src-tauri/src/services/mcp_v2/contracts.rs:138:pub struct Scope(pub String);
src-tauri/src/services/mcp_v2/contracts.rs:140:impl Scope {
src-tauri/src/services/mcp_v2/contracts.rs:150:impl std::fmt::Display for Scope {
src-tauri/src/services/mcp_v2/contracts.rs:264:/// resolved/hydrated gateway-side envelope; carries granted_scopes because the gateway loads them from the server-side manifest before dispatching. Distinct from the abilities-runtime Actor::McpClient variant which carries no scopes per ADR-0102 amendment §B asymmetry.
src-tauri/src/services/mcp_v2/contracts.rs:267:pub enum McpActor {
src-tauri/src/services/mcp_v2/contracts.rs:271:        tool_name: ScopedName,
src-tauri/src/services/mcp_v2/contracts.rs:272:        granted_scopes: Vec<Scope>,
src-tauri/src/services/mcp_v2/contracts.rs:296:    pub tool_name: ScopedName,
src-tauri/src/services/mcp_v2/contracts.rs:358:/// sentinel strings: the [`Scope`] type is reserved for
src-tauri/src/services/mcp_v2/contracts.rs:371:    /// Scope-deficit authorization failure — the caller's manifest grant
src-tauri/src/services/mcp_v2/contracts.rs:373:    /// canonical or substrate-shipped [`Scope`] per ADR-0102 §E.
src-tauri/src/services/mcp_v2/contracts.rs:375:        missing_scope: Scope,
src-tauri/src/services/mcp_v2/contracts.rs:384:    /// The caller's manifest does not include an invocable grant for
src-tauri/src/services/mcp_v2/contracts.rs:385:    /// this tool — either no grant exists for `tool_name` OR the grant's
src-tauri/src/services/mcp_v2/contracts.rs:388:    /// sentinel `Scope` value: exposure is an auth-state distinct from
src-tauri/src/services/mcp_v2/contracts.rs:391:        tool_name: ScopedName,
src-tauri/src/services/mcp_v2/contracts.rs:440:        let scope = Scope::new("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/contracts.rs:443:        let decoded: Scope = serde_json::from_str(&encoded).expect("deserializes");
src-tauri/src/services/mcp_v2/contracts.rs:451:        // `read.workspace_graph`) stay unprefixed verbatim. The `Scope`
src-tauri/src/services/mcp_v2/contracts.rs:453:        // manifest validation in `auth` rejects out-of-allowlist values.
src-tauri/src/services/mcp_v2/contracts.rs:454:        let legacy = Scope::new("write.workspace_place_document");
src-tauri/src/services/mcp_v2/contracts.rs:461:        let name = ScopedName::new("dailyos.read.account_status");
src-tauri/src/services/mcp_v2/contracts.rs:464:        let decoded: ScopedName = serde_json::from_str(&encoded).expect("deserializes");
src-tauri/src/services/mcp_v2/contracts.rs:473:        // / `granted_scopes` all rename.
src-tauri/src/services/mcp_v2/contracts.rs:474:        let actor = McpActor::Client {
src-tauri/src/services/mcp_v2/contracts.rs:477:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:478:            granted_scopes: vec![
src-tauri/src/services/mcp_v2/contracts.rs:479:                Scope::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:480:                Scope::new("read.entity_names"),
src-tauri/src/services/mcp_v2/contracts.rs:491:                    "grantedScopes": ["dailyos.read.account_status", "read.entity_names"],
src-tauri/src/services/mcp_v2/contracts.rs:501:        let actor = McpActor::Client {
src-tauri/src/services/mcp_v2/contracts.rs:504:            tool_name: ScopedName::new("dailyos.submit.note"),
src-tauri/src/services/mcp_v2/contracts.rs:505:            granted_scopes: vec![Scope::new("dailyos.submit.note")],
src-tauri/src/services/mcp_v2/contracts.rs:515:                    "grantedScopes": ["dailyos.submit.note"],
src-tauri/src/services/mcp_v2/contracts.rs:539:            missing_scope: Scope::new("dailyos.write.place_document"),
src-tauri/src/services/mcp_v2/contracts.rs:546:                "missingScope": "dailyos.write.place_document",
src-tauri/src/services/mcp_v2/contracts.rs:600:            name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:611:            scopes_required: vec![Scope::new("dailyos.read.account_status")],
src-tauri/src/services/mcp_v2/contracts.rs:728:        // Per ADR-0102 §C/§G cycle-7 amendment: absent grant or
src-tauri/src/services/mcp_v2/contracts.rs:731:        // with a sentinel `Scope` value. Wire shape mirrors the other
src-tauri/src/services/mcp_v2/contracts.rs:734:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:753:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/contracts.rs:776:            tool_name: ScopedName::new("dailyos.submit.note"),
src-tauri/src/services/mcp_v2/mod.rs:4://! amendment to ADR-0102 — Actor::McpClient actor variant + MCP client
src-tauri/src/services/mcp_v2/mod.rs:11://! - contracts — shared wire-shape types — ToolDescription, ScopedName,
src-tauri/src/services/mcp_v2/mod.rs:12://!   Scope, McpActor, ToolError, Page, OpaqueConversationHandle envelope,
src-tauri/src/services/mcp_v2/mod.rs:14://! - gateway — single entry-point dispatcher; loads scope manifest;
src-tauri/src/services/mcp_v2/mod.rs:15://!   validates Actor::McpClient policy; emits audit + signals.
src-tauri/src/services/mcp_v2/mod.rs:18://! - actor_policy — scope/permission matrix for Actor::McpClient.
src-tauri/src/services/mcp_v2/mod.rs:19://! - auth — pairing handshake; server-side scope manifest loading;
src-tauri/src/services/mcp_v2/gateway.rs:3://! Validates Actor::McpClient policy against the server-side scope manifest
src-tauri/src/services/mcp_v2/gateway.rs:25:    McpToolResult, OpaqueConversationHandle, OpaqueNonce, Scope, ScopedName, Side, ToolError,
src-tauri/src/services/mcp_v2/gateway.rs:65:        tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:74:        tool_name: Option<&ScopedName>,
src-tauri/src/services/mcp_v2/gateway.rs:93:        tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:109:        tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:122:        tool_name: Option<&ScopedName>,
src-tauri/src/services/mcp_v2/gateway.rs:136:        tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:155:    handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>,
src-tauri/src/services/mcp_v2/gateway.rs:182:    pub fn registered_tools(&self) -> impl Iterator<Item = &ScopedName> {
src-tauri/src/services/mcp_v2/gateway.rs:200:    pub fn seal(&self) -> Result<Vec<ScopedName>, super::taxonomy::TaxonomyError> {
src-tauri/src/services/mcp_v2/gateway.rs:324:        let grant = match auth::resolve_tool_grant(conn, asserted_client_id, &envelope.tool_name) {
src-tauri/src/services/mcp_v2/gateway.rs:330:        let Some(grant) = grant else {
src-tauri/src/services/mcp_v2/gateway.rs:335:                reject_reason: "absent_grant".to_string(),
src-tauri/src/services/mcp_v2/gateway.rs:342:            grant.exposure,
src-tauri/src/services/mcp_v2/gateway.rs:369:        // Scope subset per AC-2 step (iii).
src-tauri/src/services/mcp_v2/gateway.rs:370:        let required: &[Scope] = &handler.description().scopes_required;
src-tauri/src/services/mcp_v2/gateway.rs:371:        if !scope_is_subset(required, &grant.scopes_granted) {
src-tauri/src/services/mcp_v2/gateway.rs:374:                .find(|r| !grant.scopes_granted.contains(r))
src-tauri/src/services/mcp_v2/gateway.rs:376:                .unwrap_or_else(|| Scope::new(""));
src-tauri/src/services/mcp_v2/gateway.rs:390:            if params_obj.contains_key("granted_scopes") {
src-tauri/src/services/mcp_v2/gateway.rs:393:                        detail: "granted_scopes not accepted in params".to_string(),
src-tauri/src/services/mcp_v2/gateway.rs:427:        match reserve_rate_limit(conn, asserted_client_id, &envelope.tool_name, &grant) {
src-tauri/src/services/mcp_v2/gateway.rs:444:            actor_policy::project_actor(asserted_client_id, &grant, Some(&conversation_handle));
src-tauri/src/services/mcp_v2/gateway.rs:445:        let wire_actor = McpActor::Client {
src-tauri/src/services/mcp_v2/gateway.rs:449:            granted_scopes: grant.scopes_granted.clone(),
src-tauri/src/services/mcp_v2/gateway.rs:555:    tool_name_for_signal: Option<ScopedName>,
src-tauri/src/services/mcp_v2/gateway.rs:563:/// verified (post-Gate 0a/0b admit — manifest gates onward) and `None` for
src-tauri/src/services/mcp_v2/gateway.rs:653:fn scope_is_subset(required: &[Scope], granted: &[Scope]) -> bool {
src-tauri/src/services/mcp_v2/gateway.rs:654:    required.iter().all(|r| granted.contains(r))
src-tauri/src/services/mcp_v2/gateway.rs:660:    tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:718:    tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/gateway.rs:719:    grant: &ToolGrant,
src-tauri/src/services/mcp_v2/gateway.rs:722:    let window_ms = grant.rate_limit.window_seconds as i64 * 1000;
src-tauri/src/services/mcp_v2/gateway.rs:757:    if count as u32 >= grant.rate_limit.max_calls {
src-tauri/src/services/mcp_v2/gateway.rs:761:        return Err(grant.rate_limit.window_seconds);
src-tauri/src/services/mcp_v2/gateway.rs:828:        assert!(scope_is_subset(&[], &[Scope::new("dailyos.read.account_status")]));
src-tauri/src/services/mcp_v2/gateway.rs:833:        let required = vec![Scope::new("dailyos.write.place_document")];
src-tauri/src/services/mcp_v2/gateway.rs:834:        let granted = vec![Scope::new("dailyos.read.account_status")];
src-tauri/src/services/mcp_v2/gateway.rs:835:        assert!(!scope_is_subset(&required, &granted));
src-tauri/src/services/mcp_v2/gateway.rs:840:        let required = vec![Scope::new("dailyos.read.account_status")];
src-tauri/src/services/mcp_v2/gateway.rs:841:        let granted = vec![
src-tauri/src/services/mcp_v2/gateway.rs:842:            Scope::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/gateway.rs:843:            Scope::new("dailyos.read.portfolio_attention"),
src-tauri/src/services/mcp_v2/gateway.rs:845:        assert!(scope_is_subset(&required, &granted));
src-tauri/src/services/mcp_v2/auth.rs:1://! MCP v2 authentication + session: pairing, manifest load, transport HMAC,
src-tauri/src/services/mcp_v2/auth.rs:22:    McpClientId, McpToolRequestEnvelope, OpaqueConversationHandle, OpaqueNonce, Scope, ScopedName,
src-tauri/src/services/mcp_v2/auth.rs:78:    /// Initial per-tool grant set chosen by the operator at pairing time.
src-tauri/src/services/mcp_v2/auth.rs:79:    pub tool_grants: Vec<ToolGrant>,
src-tauri/src/services/mcp_v2/auth.rs:99:/// Pair a new MCP client: mint client_id, transport key, manifest rows, and
src-tauri/src/services/mcp_v2/auth.rs:118:        "INSERT INTO mcp_client_manifest \
src-tauri/src/services/mcp_v2/auth.rs:123:    for grant in &handshake.tool_grants {
src-tauri/src/services/mcp_v2/auth.rs:125:            "INSERT INTO mcp_tool_grant \
src-tauri/src/services/mcp_v2/auth.rs:126:             (client_id, tool_name, scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs) \
src-tauri/src/services/mcp_v2/auth.rs:130:                grant.tool_name.as_str(),
src-tauri/src/services/mcp_v2/auth.rs:131:                serde_json::to_string(&grant.scopes_granted)?,
src-tauri/src/services/mcp_v2/auth.rs:132:                exposure_tag(grant.exposure),
src-tauri/src/services/mcp_v2/auth.rs:133:                grant.rate_limit.max_calls,
src-tauri/src/services/mcp_v2/auth.rs:134:                grant.rate_limit.window_seconds,
src-tauri/src/services/mcp_v2/auth.rs:151:    // pairing audit event sink; not stored on the manifest schema for v1.4.7.
src-tauri/src/services/mcp_v2/auth.rs:163:/// Load the manifest record for a client — indexed PK lookup; no cache (per
src-tauri/src/services/mcp_v2/auth.rs:171:         FROM mcp_client_manifest WHERE client_id = ?1",
src-tauri/src/services/mcp_v2/auth.rs:191:pub fn resolve_tool_grant(
src-tauri/src/services/mcp_v2/auth.rs:194:    tool_name: &ScopedName,
src-tauri/src/services/mcp_v2/auth.rs:197:        "SELECT scopes_granted_json, exposure, rate_limit_max, rate_limit_window_secs \
src-tauri/src/services/mcp_v2/auth.rs:198:         FROM mcp_tool_grant WHERE client_id = ?1 AND tool_name = ?2",
src-tauri/src/services/mcp_v2/auth.rs:212:    let scopes_granted: Vec<Scope> = serde_json::from_str(&scopes_json)?;
src-tauri/src/services/mcp_v2/auth.rs:215:        scopes_granted,
src-tauri/src/services/mcp_v2/auth.rs:389:        "UPDATE mcp_client_manifest SET revoked_at = ?2 WHERE client_id = ?1",
src-tauri/src/services/mcp_v2/auth.rs:544:/// Return every tool name from `mcp_tool_grant` for `client_id` where
src-tauri/src/services/mcp_v2/auth.rs:546:/// per the DOS-MCP-Transport L0 AC-6: only Invocable-tier grants are
src-tauri/src/services/mcp_v2/auth.rs:548:pub fn list_invocable_tool_grants(
src-tauri/src/services/mcp_v2/auth.rs:551:) -> Result<Vec<ScopedName>, AuthError> {
src-tauri/src/services/mcp_v2/auth.rs:553:        "SELECT tool_name FROM mcp_tool_grant \
src-tauri/src/services/mcp_v2/auth.rs:559:        Ok(ScopedName::new(name))
src-tauri/src/services/mcp_v2/transport.rs:9://! - **Scope**: local-to-local same-machine only. Trust boundary is
src-tauri/src/services/mcp_v2/transport.rs:44:    ScopedName, ToolDescription, ToolError,
src-tauri/src/services/mcp_v2/transport.rs:54:/// successful env-assertion + manifest lookup + keychain cross-check in
src-tauri/src/services/mcp_v2/transport.rs:162:        // Per L0 AC-6: filter by `mcp_tool_grant` rows for
src-tauri/src/services/mcp_v2/transport.rs:164:        let grants = {
src-tauri/src/services/mcp_v2/transport.rs:166:            auth::list_invocable_tool_grants(&conn_guard, &self.verified_client_id).map_err(
src-tauri/src/services/mcp_v2/transport.rs:169:                        format!("mcp_v2 tool_grant lookup: {e}"),
src-tauri/src/services/mcp_v2/transport.rs:176:        let mut tools = Vec::with_capacity(grants.len());
src-tauri/src/services/mcp_v2/transport.rs:177:        for grant_name in &grants {
src-tauri/src/services/mcp_v2/transport.rs:178:            if let Some(desc) = self.catalog.description_for(grant_name) {
src-tauri/src/services/mcp_v2/transport.rs:197:        let tool_name = ScopedName::new(request.name.to_string());
src-tauri/src/services/mcp_v2/transport.rs:341:    use crate::services::mcp_v2::contracts::{ParamSchema, ParamSpec, ReturnSpec, Scope, Side};
src-tauri/src/services/mcp_v2/transport.rs:345:            name: ScopedName::new(name),
src-tauri/src/services/mcp_v2/transport.rs:398:            missing_scope: Scope::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/transport.rs:420:            tool_name: ScopedName::new("dailyos.read.account_status"),
src-tauri/src/services/mcp_v2/audit.rs:127:    if let Actor::McpClient {
src-tauri/src/services/mcp_v2/audit.rs:177:        Actor::McpClient { .. } => "mcp_client",

exec
/bin/zsh -lc "rg -n \"pub enum Actor|McpClient|pub enum ActorKind|fn kind\\(\" src-tauri/abilities-runtime/src/abilities src-tauri/src/bridges -g '*.rs'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/bridges/mcp.rs:737:            Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/src/bridges/tauri.rs:421:        BridgeActor::McpClient => "mcp_client",
src-tauri/src/bridges/tauri.rs:639:            Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:748:        // Agent / Admin / System / SurfaceClient / McpClient — `get_daily_briefing`
src-tauri/abilities-runtime/src/abilities/get_daily_briefing/producer.rs:767:        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
src-tauri/src/bridges/types.rs:44:    McpClient,
src-tauri/src/bridges/types.rs:57:            Self::McpClient => {
src-tauri/src/bridges/types.rs:58:                panic!("BridgeActor::McpClient requires request-scoped Actor::McpClient")
src-tauri/src/bridges/types.rs:70:            Actor::McpClient { .. } => Self::McpClient,
src-tauri/src/bridges/types.rs:840:        BridgeActor::McpClient => "mcp_client",
src-tauri/src/bridges/types.rs:953:        BridgeActor::McpClient => ProvenanceActor::External {
src-tauri/abilities-runtime/src/abilities/fallback_projection.rs:951:        Actor::McpClient { client_id, .. } => {
src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:2063:        RegistryActor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:2159:        RegistryActor::McpClient { .. } => {
src-tauri/abilities-runtime/src/abilities/get_daily_readiness/synthesis.rs:2160:            todo!("McpClient invocation routing pending")
src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:437:        Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/get_entity_context.rs:476:        Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/detect_risk_shift/synthesis.rs:1277:        RegistryActor::McpClient { .. } => {
src-tauri/abilities-runtime/src/abilities/detect_risk_shift/synthesis.rs:1278:            todo!("McpClient invocation routing pending")
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:11://! Allowed actors: `User` + `Agent`. `AgentMcp` (`Actor::McpClient`) is
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:66:    use crate::abilities::registry::{AbilityRegistry, McpClientId};
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:350:            Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:351:                client_id: McpClientId::new("mcp-test"),
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:357:        .expect_err("McpClient is denied");
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:18://! runtime client). `McpClient` is denied — receipt rendering is
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:92:        // WP runtime client invocation works; McpClient denied
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:105:            !actors.contains(&ActorKind::McpClient),
src-tauri/abilities-runtime/src/abilities/claim_receipt/mod.rs:106:            "McpClient must be denied (receipt rendering is sensitivity-bearing)"
src-tauri/abilities-runtime/src/abilities/claim_receipt/producer.rs:149:        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1011:        Actor::McpClient { client_id, .. } => crate::abilities::provenance::Actor::External {
src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1852:        RegistryActor::McpClient { .. } => {
src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1853:            todo!("McpClient invocation routing pending")
src-tauri/abilities-runtime/src/abilities/list_open_loops/mod.rs:321:        Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/list_open_loops/mod.rs:586:        Actor::McpClient { .. } => todo!("McpClient invocation routing pending"),
src-tauri/abilities-runtime/src/abilities/list_pagination.rs:142:        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
src-tauri/abilities-runtime/src/abilities/registry.rs:106:/// Stable, non-PII identifier for a paired [`Actor::McpClient`] instance.
src-tauri/abilities-runtime/src/abilities/registry.rs:114:/// Unlike [`Actor::SurfaceClient`], the [`Actor::McpClient`] variant
src-tauri/abilities-runtime/src/abilities/registry.rs:124:pub struct McpClientId(String);
src-tauri/abilities-runtime/src/abilities/registry.rs:126:impl McpClientId {
src-tauri/abilities-runtime/src/abilities/registry.rs:138:impl std::fmt::Display for McpClientId {
src-tauri/abilities-runtime/src/abilities/registry.rs:145:/// conversation continuity for an [`Actor::McpClient`] invocation path.
src-tauri/abilities-runtime/src/abilities/registry.rs:153:/// The corresponding field on [`Actor::McpClient`] is `Option<_>`
src-tauri/abilities-runtime/src/abilities/registry.rs:414:pub enum Actor {
src-tauri/abilities-runtime/src/abilities/registry.rs:434:    /// Server-issued [`McpClientId`] after a pairing handshake; per-call
src-tauri/abilities-runtime/src/abilities/registry.rs:441:    McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:444:        client_id: McpClientId,
src-tauri/abilities-runtime/src/abilities/registry.rs:468:pub enum ActorKind {
src-tauri/abilities-runtime/src/abilities/registry.rs:480:    /// Mirrors [`Actor::McpClient`]. Per-invocation client identity and
src-tauri/abilities-runtime/src/abilities/registry.rs:483:    /// scopes for `McpClient` are gateway-enforced from a server-side
src-tauri/abilities-runtime/src/abilities/registry.rs:485:    McpClient,
src-tauri/abilities-runtime/src/abilities/registry.rs:493:    pub const fn kind(&self) -> ActorKind {
src-tauri/abilities-runtime/src/abilities/registry.rs:500:            Actor::McpClient { .. } => ActorKind::McpClient,
src-tauri/abilities-runtime/src/abilities/registry.rs:2433:    // McpClient actor class — identity / handle newtype + Actor::McpClient
src-tauri/abilities-runtime/src/abilities/registry.rs:2435:    // `McpClientId` after pairing; opaque conversation handle; gateway-
src-tauri/abilities-runtime/src/abilities/registry.rs:2442:        let id = McpClientId::new("mcp-client-alpha");
src-tauri/abilities-runtime/src/abilities/registry.rs:2445:        assert_eq!(format!("{id:?}"), "McpClientId(\"mcp-client-alpha\")");
src-tauri/abilities-runtime/src/abilities/registry.rs:2450:        let id = McpClientId::new("mcp-client-alpha");
src-tauri/abilities-runtime/src/abilities/registry.rs:2454:        let decoded: McpClientId = serde_json::from_str(&encoded).expect("deserializes");
src-tauri/abilities-runtime/src/abilities/registry.rs:2462:        set.insert(McpClientId::new("alpha"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2463:        set.insert(McpClientId::new("beta"));
src-tauri/abilities-runtime/src/abilities/registry.rs:2464:        set.insert(McpClientId::new("alpha")); // dup
src-tauri/abilities-runtime/src/abilities/registry.rs:2466:        assert!(set.contains(&McpClientId::new("alpha")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2467:        assert!(set.contains(&McpClientId::new("beta")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2468:        assert!(!set.contains(&McpClientId::new("gamma")));
src-tauri/abilities-runtime/src/abilities/registry.rs:2504:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2505:            client_id: McpClientId::new("mcp-client-alpha"),
src-tauri/abilities-runtime/src/abilities/registry.rs:2508:        assert_eq!(actor.kind(), ActorKind::McpClient);
src-tauri/abilities-runtime/src/abilities/registry.rs:2513:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2514:            client_id: McpClientId::new("mcp-client-alpha"),
src-tauri/abilities-runtime/src/abilities/registry.rs:2517:        assert_eq!(actor.kind(), ActorKind::McpClient);
src-tauri/abilities-runtime/src/abilities/registry.rs:2522:        // Locks the JSON wire shape of `Actor::McpClient` so any future
src-tauri/abilities-runtime/src/abilities/registry.rs:2526:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2527:            client_id: McpClientId::new("mcp-client-alpha"),
src-tauri/abilities-runtime/src/abilities/registry.rs:2534:                "McpClient": {
src-tauri/abilities-runtime/src/abilities/registry.rs:2546:        let actor = Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/registry.rs:2547:            client_id: McpClientId::new("mcp-client-alpha"),
src-tauri/abilities-runtime/src/abilities/registry.rs:2554:                "McpClient": {
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:55:    use crate::abilities::registry::{AbilityRegistry, McpClientId};
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:273:            Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:274:                client_id: McpClientId::new("mcp-test"),
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:280:        .expect_err("McpClient denied");
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:219:pub enum Actor {
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:55:    use crate::abilities::registry::{AbilityRegistry, McpClientId};
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:269:            Actor::McpClient {
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:270:                client_id: McpClientId::new("mcp-test"),
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:276:        .expect_err("McpClient denied");
src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/producer.rs:281:        Actor::McpClient { .. } => RenderActor::agent("mcp_client"),
src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/producer.rs:1062:        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {

exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.7-waves.md | sed -n '540,570p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   540	
   541	Three agents. DOS-175 (briefing replacement — Bug priority Urgent) merges first; DOS-172 + DOS-173 run in parallel after.
   542	
   543	### Agent W2-A — DOS-175: Replace stale briefing MCP reads with ability-backed current context
   544	
   545	- **Spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175).
   546	- **Goal:** The current MCP briefing path reads stale generated snapshots. Replace with ability-backed current intelligence so MCP is a co-equal head, not an export layer.
   547	- **Files owned (exclusive):** `src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs` (W1-A pre-created placeholder; W2-A fills); legacy briefing snapshot read paths to deprecate (enumerated at L0 plan via grep).
   548	- **Don't touch:** Gateway / contracts / actor policy (W1 owners); other handlers (other lane owners).
   549	- **Depends on:** W1 merged.
   550	- **Done when:** `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` tools (canonical naming per cycle 2 #8) dispatch to ability-backed current-context queries; legacy snapshot read code paths removed (or wrapped with deprecation warnings); response shape includes provenance + freshness per ADR-0108; `cargo test` + `cargo clippy -D warnings` clean.
   551	
   552	### Agent W2-B — DOS-172: Pagination + ranked discovery
   553	
   554	- **Spec:** [DOS-172](https://linear.app/a8c/issue/DOS-172).
   555	- **Goal:** Entity discovery tools (account list, person list, project list) get cursor-based pagination + ranked discovery so host workflows stay fast on large portfolios. **Cycle 1 amendment (preempt v1.4.5 cycle 8 lesson):** pagination cursor format frozen at L0 plan as opaque base64 (same convention as v1.4.5 W3-C `WorkspaceGraphResponse`). Page size capped at 200 default, 500 max.
   556	- **Files owned (exclusive):** `src-tauri/src/services/mcp_v2/handlers/tool_pagination.rs` (W1-A pre-created placeholder).
   557	- **Don't touch:** Other handlers; v1.4.5 graph projection (DOS-479 lane consumes that).
   558	- **Depends on:** W1 merged; W2-A merged (depends on the briefing replacement establishing the response-shape pattern with provenance/freshness for read tools).
   559	- **Done when:** Entity discovery tools support cursor pagination; ranked discovery returns top-N candidates ordered by recency × interaction × user-fit (existing v1.4.6-substrate scoring if present, else recency-only fallback); pagination cursor opaque + server-rotation-safe; `cargo test` + `cargo clippy -D warnings` clean.
   560	
   561	### Agent W2-C — DOS-173: Portfolio attention tool
   562	
   563	- **Spec:** [DOS-173](https://linear.app/a8c/issue/DOS-173).
   564	- **Goal:** One efficient MCP tool — `dailyos.read.portfolio_attention` (canonical naming per cycle 2 #8) — that asks "which accounts/projects/people/open loops need attention" without chaining many low-level calls. Ability-backed, claim/source-aware, trust-rendered.
   565	- **Files owned (exclusive):** `src-tauri/src/services/mcp_v2/handlers/tool_portfolio.rs` (W1-A pre-created placeholder); new ability `abilities-runtime/src/abilities/portfolio_attention.rs` if no existing ability covers it (confirm at L0 plan from grep).
   566	- **Don't touch:** Other handlers; v1.4.6 substrate (consume read-only).
   567	- **Depends on:** W1 merged; W2-A merged.
   568	- **Done when:** `dailyos.read.portfolio_attention` returns ranked attention items with claim/trust band + provenance refs + freshness; ability-backed (no direct DB queries); pagination via DOS-172's pattern; `cargo test` + `cargo clippy -D warnings` clean.
   569	
   570	### W2 merge gate

exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md | sed -n '1,115p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
     1	# DOS-624 — CSO L0 amendment for MCP exposure of daily/meeting briefing intelligence
     2	
     3	**Type:** L0 amendment (security-annotated)
     4	**Wave:** v1.4.7 W2-A predecessor (gates DOS-175 implementation)
     5	**Lane spec:** [DOS-624](https://linear.app/a8c/issue/DOS-624)
     6	**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.
     7	
     8	## Cycle-2 changelog (2026-05-20)
     9	
    10	Cycle-1 verdicts: all 4 NEEDS-CHANGES. **Convergent (2+ reviewers):**
    11	- **meeting_briefing existence oracle / enumeration** (CSO + challenge)
    12	- **W3-C two-paths actor_kind tagging is factually wrong** (architect + challenge + devex)
    13	
    14	Cycle-2 fixes:
    15	
    16	1. **Architect §4 rewrite — actor_kind correctly distinguishes paths** (architect HIGH + challenge + devex convergent). The cycle-1 amendment claimed "both paths write via Actor::McpClient variants distinguishable by actor_kind." Architect correctly flagged: there's only ONE `Actor::McpClient { client_id, conversation_handle }` variant; `audit_log.rs:215` collapses all McpClient invocations to `actor_kind = "mcp_client"`. The W3-C WordPress MCP Adapter path uses `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 — audit tags `actor_kind = "surface_client"`. So the two paths ARE distinguishable, just by `surface_client` vs `mcp_client`, not by two McpClient variants. §4 corrected.
    17	
    18	2. **§4 W3-C policy store correction** (architect HIGH). ADR-0129 §4: WP uses WP capability + WP MCP Adapter allowlist + `SurfaceClient` scopes per ADR-0111 §8. Not `mcp_client_manifest` / `mcp_tool_grant` (which is W1-A direct-path-only). §4 specifies independent policy stores per path; operator grants briefing access in BOTH places independently. No shared manifest schema.
    19	
    20	3. **§3 sensitivity gate composition** (architect MED). The cycle-1 §3.1 protection said "Handler MUST filter to caller's read.entity_names scope." Architect correctly flagged this should COMPOSE the existing centralized claim sensitivity gate (ADR-0125 + per-call ClaimSensitivity render in services::claims::render), NOT copy redaction logic. §3.1 rewritten to require DOS-175 handler routes briefing output through the centralized sensitivity render with caller's manifest scopes; no parallel redaction.
    21	
    22	4. **meeting_briefing uniform unavailable response in DOS-175 AC** (CSO HIGH + challenge HIGH convergent). Existence-oracle defense: DOS-175 W2-A must include `dailyos.read.meeting_briefing(meeting_id)` returning a uniform `{status: "unavailable"}` typed result for ALL of: nonexistent meeting_id, manifest scope insufficient for entity_names, filtered by sensitivity, stale beyond freshness threshold. This pushes the "object resolution hygiene" into W2-A AC explicitly. CSO Q2 conditional sign-off requires this.
    23	
    24	5. **CSO #2 audit coverage for unavailable responses** — DOS-175 makes unavailable responses successful typed results (per #4), so the W1-A success-path audit (`audit::write` with `event="mcp.tool_invoked"`) covers them naturally. `params_hash` over `(meeting_id)` fingerprints probing patterns; operator detects via duplicate `params_hash` across many `conversation_handle`s.
    25	
    26	6. **Q1 sign-off folded — per-pairing only** (CSO + devex confirm). Default-denied per-pairing grants + revocation are sufficient for v1.4.7. Global policy registry remains path-α (separate ticket: "DOS-? v1.4.7+ MCP global briefing policy registry").
    27	
    28	7. **Q2 sign-off folded — manifest scope authorization + uniform unavailable response** (CSO conditional sign-off). Combined gate.
    29	
    30	8. **Singleton substrate hardening filed as separate tickets, NOT folded into this amendment** (per `feedback_review_loop_diminishing_returns_means_scope_is_wrong` + `feedback_l2_path_alpha_to_maintenance_project`):
    31	   - **DOS-? "MCP pairing credential-theft defense"** (challenge #1) — host/client-key binding, one-time pairing codes, short pairing TTL, operator-visible device identity. Real substrate work for v1.4.7+; not blocking this amendment because the post-pairing trust contract already covers replay (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed which CSO independently verified holds).
    32	   - **DOS-? "MCP per-user global briefing rate-limit budget"** (challenge #4) — path-coalesced anomaly detection across W1-A direct + W3-C WP paths. Substrate work for v1.4.7+.
    33	   - **DOS-? "MCP scope grant UI: no glob, explicit briefing scope, confirmation copy"** (challenge #5) — operator UI work, post-v1.4.7.
    34	   - **DOS-? "MCP response_hash side-channel mitigation"** (challenge #2) — per-row salt or security-admin-only fingerprint querying. Substrate work for v1.4.7+.
    35	   - **DOS-? "MCP operator pairing UX: briefing intelligence section"** (devex #1) — operator-facing docs + manifest UI changes. Post-v1.4.7.
    36	   - **DOS-? "MCP rate-limit config keys + retry-after surfacing"** (devex #2) — DOS-175 should make `rate_limit_max=60`, `rate_limit_window_secs=3600` discoverable; gateway's `ToolError::RateLimited { retry_after_seconds }` already exists in W1-A so this is documentation + config wiring, not substrate.
    37	   - **DOS-? "MCP audit dashboard query for briefing replay anomaly detection"** (devex #3) — explicit SQL query schema for operator dashboards.
    38	   - **DOS-? "MCP two-paths-one-ability operator runbook"** (devex #4) — pre-W2-A docs work; how to grant/revoke each path independently; how to audit both sources via `actor_kind` filter.
    39	
    40	   These are real, named, scoped — filed in this amendment rather than folded keeps the amendment as a policy decision (its proper shape) rather than turning it into a substrate-hardening initiative.
    41	
    42	---
    43	
    44	## 1. What this amendment unblocks (unchanged from cycle 1)
    45	
    46	v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
    47	- `dailyos.read.daily_briefing` — current-day briefing
    48	- `dailyos.read.meeting_briefing(meeting_id)` — prep briefing for a specific meeting
    49	
    50	The v1.4.3 carve-out blocks them pending this amendment.
    51	
    52	## 2. Why MCP exposure is appropriate now (unchanged + CSO verified)
    53	
    54	The v1.4.3 carve-out predates v1.4.7 W1-A's MCP trust contract. Pre-W1-A, MCP exposure meant unauthenticated unbounded ability invocation. v1.4.7 W1-A landed (commits `45b3f53d`..`3e5d7155` on `v1.4.7-w1-foundation`; L2 unanimous APPROVE; CSO confirmed in this cycle's review: "No new transport replay vector found. ADR-0102 nonce consume + per-response refresh + fail-closed semantics are implemented in `auth.rs`/`gateway.rs` as described.") Every MCP-originated invocation now passes through:
    55	
    56	1. Pairing handshake → server-issued `McpClientId` + operator-set manifest
    57	2. Transport HMAC-SHA256 signing over canonical-JSON envelope including `request_nonce`; key wrapped in `Zeroizing`
    58	3. Server-side nonce ledger consume-once + fail-closed (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed)
    59	4. Server-side manifest scope authorization (caller-asserted scopes rejected)
    60	5. Per-tool exposure tier (None / MetadataOnly / Invocable; default None)
    61	6. Per `(McpClientId, ScopedName)` rate limit with atomic BEGIN IMMEDIATE
    62	7. Audit attribution with keyed HMAC-SHA256 hashes (no raw payloads)
    63	8. Signal emission via `McpToolInvoked` / `McpInvocationRejected` (NonPiiMetadata)
    64	
    65	Pre-W1-A threat model (unauthenticated MCP exposure of personal briefing intelligence) no longer applies.
    66	
    67	## 3. Briefing-specific protections (cycle-2 sensitivity gate composed)
    68	
    69	Beyond W1-A's universal gates, DOS-175 (W2-A) MUST honor these briefing-specific ACs:
    70	
    71	1. **Sensitivity rendering composes the centralized claim sensitivity gate** (cycle-2 fix #3 per architect MED). DOS-175 handler routes briefing output through `services::claims::render` (or the equivalent centralized sensitivity rendering path per ADR-0125 + ADR-0108) using the caller's manifest-resolved scopes. PII-tier claims (file paths, raw entity names, raw claim text in aggregates) redact when caller's manifest does not grant the corresponding read scope (`read.entity_names`). NO parallel redaction logic in the handler.
    72	
    73	2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
    74	
    75	3. **Per-invocation rate limit default = 60 calls/hour** (unchanged from cycle 1). Operator-tunable.
    76	
    77	4. **Uniform unavailable response for meeting_briefing(meeting_id)** (cycle-2 fix #4 per CSO HIGH + challenge HIGH). DOS-175 handler returns `{status: "unavailable", reason_class: "unavailable"}` (NOT distinguishing nonexistent / scope-insufficient / sensitivity-filtered / stale-beyond-freshness — uniform shape) for ALL of:
    78	   - meeting_id not in caller's accessible meeting set
    79	   - meeting exists but scope insufficient for entity_names disclosure
    80	   - meeting exists, accessible, but ClaimSensitivity gate filters all surfaceable claims
    81	   - meeting exists but briefing freshness exceeds operator-configured threshold (stale)
    82	   
    83	   **Uniform timing floor for unavailable success responses** (cycle-3 CSO normative tightening). The W1-A AC-12 10ms floor applies only to auth-state rejections, NOT to successful typed responses. DOS-175 handler **MUST** call `tokio::time::sleep_until(start + Duration::from_millis(20))` (20ms or greater; not "recommended" — required) before returning ANY `{status: "unavailable"}` response, regardless of which internal branch (nonexistent / scope-insufficient / sensitivity-filtered / stale) produced it. **Concurrent burst test required: p95 latency across all 4 unavailable branches MUST be within ± 5ms of each other** (burst of ≥ 200 calls under concurrent load, asserted in `tests/dos175_meeting_briefing_existence_oracle_test.rs`). Closes CSO cycle-2 + cycle-3 enumeration-via-latency findings.
    84	
    85	5. **Audit covers unavailable responses** (cycle-2 fix #5 per CSO #2). Because unavailable is a successful typed result, W1-A's success-path audit naturally fingerprints it via `params_hash` over `(meeting_id)`. Operators detect probing via duplicate `params_hash` across many `conversation_handle`s. No new audit substrate needed.
    86	
    87	## 4. Coordination with v1.4.2 W3-C WordPress MCP Adapter (cycle-2 corrected)
    88	
    89	Per cycle-2 fix #1 + #2 (architect HIGH convergent):
    90	
    91	- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
    92	- **Different actor classes, distinguishable in audit**:
    93	  - W1-A direct path (Claude Desktop / Cursor / other agents over loopback HTTP or stdio MCP) → `Actor::McpClient { client_id, conversation_handle }` → audit `actor_kind = "mcp_client"`
    94	  - W3-C WP MCP Adapter path (WordPress block + adapter) → `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 §4 → audit `actor_kind = "surface_client"`
    95	- **Different policy stores per path** (cycle-2 fix #2):
    96	  - W1-A direct: `mcp_client_manifest` + `mcp_tool_grant` rows (v241 migration; loaded by `services::mcp_v2::auth`)
    97	  - W3-C WP: WP capability + WP MCP Adapter allowlist + `SurfaceClient` scope grants per ADR-0111 §8
    98	  - Operator MUST grant briefing access in BOTH places independently — no shared manifest schema, no implicit propagation.
    99	- **No duplicate audit rows** — each invocation through either path writes ONE audit row attributed to its respective Actor variant; the audit log's `actor_kind` field distinguishes the source.
   100	- **Operator runbook for two-paths coordination** filed as separate ticket (cycle-2 fix #8 — "DOS-? MCP two-paths-one-ability operator runbook").
   101	
   102	## 5. Audit + signal payload changes (NONE required) (unchanged)
   103	
   104	This amendment adds no new audit event types, no new signal types, no new claim types. The W1-A substrate handles briefing invocations identically to any other tool. The amendment is a scope decision, not a substrate change.
   105	
   106	## 6. Acceptance
   107	
   108	- **CSO L0 sign-off** on this amendment (Q1 + Q2 cycle-1 sign-offs folded per cycle-2 fix #6 + #7).
   109	- **No code changes in this amendment** — scope/policy only. Implementation lands in DOS-175 (W2-A).
   110	- **W2-A DOS-175 L0 packet** (separate) MUST cite this amendment as predecessor + enumerate §3 ACs in its own AC (especially #1 sensitivity-gate composition + #4 uniform unavailable response + #5 audit fingerprinting).
   111	
   112	## 7. Spinoff Linear tickets (cycle-2 fix #8 filed list)
   113	
   114	Filed to DailyOS Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per path-α discipline:
   115	

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/gateway.rs | sed -n '300,455p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/transport.rs | sed -n '140,255p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
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

 succeeded in 0ms:
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
   361	                },
   362	                reject_reason: "unknown_tool".to_string(),
   363	                tool_name_for_signal: Some(envelope.tool_name.clone()),
   364	                client_id_for_signal: Some(asserted_client_id.clone()),
   365	                preissued_next_nonce: preissued.clone(),
   366	            }));
   367	        };
   368	
   369	        // Scope subset per AC-2 step (iii).
   370	        let required: &[Scope] = &handler.description().scopes_required;
   371	        if !scope_is_subset(required, &grant.scopes_granted) {
   372	            let missing = required
   373	                .iter()
   374	                .find(|r| !grant.scopes_granted.contains(r))
   375	                .cloned()
   376	                .unwrap_or_else(|| Scope::new(""));
   377	            return Err(Box::new(GatewayFailure {
   378	                error: ToolError::Unauthorized {
   379	                    missing_scope: missing.clone(),
   380	                },
   381	                reject_reason: format!("missing_scope:{}", missing.as_str()),
   382	                tool_name_for_signal: Some(envelope.tool_name.clone()),
   383	                client_id_for_signal: Some(asserted_client_id.clone()),
   384	                preissued_next_nonce: preissued.clone(),
   385	            }));
   386	        }
   387	
   388	        // Reject caller-asserted scope/conversation fields in params per AC-2 (iv).
   389	        if let Some(params_obj) = envelope.params.as_object() {
   390	            if params_obj.contains_key("granted_scopes") {
   391	                return Err(Box::new(GatewayFailure {
   392	                    error: ToolError::BadParams {
   393	                        detail: "granted_scopes not accepted in params".to_string(),
   394	                    },
   395	                    reject_reason: "caller_asserted_scopes".to_string(),
   396	                    tool_name_for_signal: Some(envelope.tool_name.clone()),
   397	                    client_id_for_signal: Some(asserted_client_id.clone()),
   398	                    preissued_next_nonce: preissued.clone(),
   399	                }));
   400	            }
   401	            if params_obj.contains_key("conversation_id") {
   402	                return Err(Box::new(GatewayFailure {
   403	                    error: ToolError::BadParams {
   404	                        detail: "use envelope conversation_handle".to_string(),
   405	                    },
   406	                    reject_reason: "caller_asserted_conversation_id".to_string(),
   407	                    tool_name_for_signal: Some(envelope.tool_name.clone()),
   408	                    client_id_for_signal: Some(asserted_client_id.clone()),
   409	                    preissued_next_nonce: preissued.clone(),
   410	                }));
   411	            }
   412	        }
   413	
   414	        // Resolve/mint conversation handle per AC-3a/AC-3b §D.bis.
   415	        let conversation_handle = match auth::resolve_or_mint_handle(
   416	            conn,
   417	            asserted_client_id,
   418	            envelope.conversation_handle.as_ref(),
   419	        ) {
   420	            Ok(h) => h,
   421	            Err(err) => {
   422	                return Err(auth_to_failure(err, envelope, Some(asserted_client_id), preissued.clone()))
   423	            }
   424	        };
   425	
   426	        // Rate limit per AC-5: BEGIN IMMEDIATE → prune → count → reserve OR ROLLBACK.
   427	        match reserve_rate_limit(conn, asserted_client_id, &envelope.tool_name, &grant) {
   428	            Ok(()) => {}
   429	            Err(retry_after) => {
   430	                return Err(Box::new(GatewayFailure {
   431	                    error: ToolError::RateLimited {
   432	                        retry_after_seconds: retry_after,
   433	                    },
   434	                    reject_reason: "rate_limited".to_string(),
   435	                    tool_name_for_signal: Some(envelope.tool_name.clone()),
   436	                    client_id_for_signal: Some(asserted_client_id.clone()),
   437	                    preissued_next_nonce: preissued.clone(),
   438	                }));
   439	            }
   440	        }
   441	
   442	        // Construct the runtime actor and dispatch.
   443	        let runtime_actor =
   444	            actor_policy::project_actor(asserted_client_id, &grant, Some(&conversation_handle));
   445	        let wire_actor = McpActor::Client {
   446	            client_id: asserted_client_id.clone(),
   447	            conversation_handle: Some(conversation_handle.clone()),
   448	            tool_name: envelope.tool_name.clone(),
   449	            granted_scopes: grant.scopes_granted.clone(),
   450	        };
   451	        let invocation = handler.invoke(&wire_actor, envelope.params.clone());
   452	
   453	        let result = match invocation {
   454	            Ok(value) => {
   455	                // Extract + cap mutation_cursor for Side::Write per AC-6.

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/actor_policy.rs | sed -n '1,90p'" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/contracts.rs | sed -n '255,282p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
     1	//! MCP v2 actor and ability-policy projection.
     2	//!
     3	//! ADR-0102 §B makes MCP asymmetric with `SurfaceClient`: MCP grants are
     4	//! resolved and enforced at the gateway, while the runtime actor variant
     5	//! carries only identity and optional conversation continuity. This module is
     6	//! the small adapter between the MCP manifest records and the abilities
     7	//! runtime shapes.
     8	
     9	use std::time::SystemTime;
    10	
    11	use abilities_runtime::abilities::registry::{
    12	    AbilityPolicy, Actor, ActorKind, McpClientId as RuntimeMcpClientId, McpExposure,
    13	    OpaqueConversationHandle as RuntimeOpaqueConversationHandle,
    14	};
    15	
    16	use super::contracts::{McpClientId, OpaqueConversationHandle, Scope, ScopedName};
    17	
    18	const MCP_CLIENT_ACTORS: &[ActorKind] = &[ActorKind::McpClient];
    19	
    20	/// Opaque Keychain account reference for an MCP client's transport key.
    21	#[derive(Debug, Clone, PartialEq, Eq)]
    22	pub struct KeychainRef(pub String);
    23	
    24	/// Gateway-loaded pairing record. Raw transport key bytes never live here.
    25	#[derive(Debug, Clone, PartialEq, Eq)]
    26	pub struct ClientRecord {
    27	    pub client_id: McpClientId,
    28	    pub paired_at: SystemTime,
    29	    pub revoked_at: Option<SystemTime>,
    30	    pub transport_key_ref: KeychainRef,
    31	}
    32	
    33	/// Per-tool invocation rate limit from the MCP client manifest.
    34	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    35	pub struct ToolRateLimit {
    36	    pub max_calls: u32,
    37	    pub window_seconds: u32,
    38	}
    39	
    40	/// Per-call manifest grant resolved by indexed `(client_id, tool_name)` lookup.
    41	#[derive(Debug, Clone, PartialEq, Eq)]
    42	pub struct ToolGrant {
    43	    pub tool_name: ScopedName,
    44	    pub scopes_granted: Vec<Scope>,
    45	    pub exposure: McpExposure,
    46	    pub rate_limit: ToolRateLimit,
    47	}
    48	
    49	/// Project a manifest grant into the runtime policy shell used by the
    50	/// abilities gate for MCP-originated calls.
    51	///
    52	/// MCP scope checks are gateway-mediated against [`ToolGrant::scopes_granted`].
    53	/// The runtime actor carries no scopes, so `required_scopes` stays empty here.
    54	pub fn project_policy(tool_grant: &ToolGrant) -> AbilityPolicy {
    55	    AbilityPolicy {
    56	        allowed_actors: MCP_CLIENT_ACTORS,
    57	        mcp_exposure: tool_grant.exposure,
    58	        ..AbilityPolicy::default()
    59	    }
    60	}
    61	
    62	/// Construct the runtime actor variant for an MCP-originated invocation.
    63	///
    64	/// `tool_grant` is intentionally accepted here even though scopes are not put
    65	/// on the actor. Passing it through this API makes the gateway call site read
    66	/// as "manifest resolved before runtime actor construction", which is the
    67	/// ADR-0102 asymmetry this module exists to preserve.
    68	pub fn project_actor(
    69	    client_id: &McpClientId,
    70	    tool_grant: &ToolGrant,
    71	    conversation_handle: Option<&OpaqueConversationHandle>,
    72	) -> Actor {
    73	    let _ = project_policy(tool_grant);
    74	    Actor::McpClient {
    75	        client_id: RuntimeMcpClientId::new(client_id.as_str()),
    76	        conversation_handle: conversation_handle
    77	            .map(|handle| RuntimeOpaqueConversationHandle::new(handle.as_str())),
    78	    }
    79	}

 succeeded in 0ms:
   255	    }
   256	}
   257	
   258	impl std::fmt::Display for McpClientId {
   259	    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
   260	        formatter.write_str(&self.0)
   261	    }
   262	}
   263	
   264	/// resolved/hydrated gateway-side envelope; carries granted_scopes because the gateway loads them from the server-side manifest before dispatching. Distinct from the abilities-runtime Actor::McpClient variant which carries no scopes per ADR-0102 amendment §B asymmetry.
   265	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   266	#[serde(rename_all_fields = "camelCase")]
   267	pub enum McpActor {
   268	    Client {
   269	        client_id: McpClientId,
   270	        conversation_handle: Option<OpaqueConversationHandle>,
   271	        tool_name: ScopedName,
   272	        granted_scopes: Vec<Scope>,
   273	    },
   274	}
   275	
   276	/// Top-level MCP request envelope per ADR-0102 §D (2026-05-19 amendment).
   277	///
   278	/// The `conversation_handle` rides at the envelope level, separate from
   279	/// the tool's typed `params` payload. `None` on the first call in a
   280	/// fresh conversation; the gateway mints a new handle and returns it on
   281	/// the response envelope.
   282	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/gateway.rs | sed -n '442,520p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   442	        // Construct the runtime actor and dispatch.
   443	        let runtime_actor =
   444	            actor_policy::project_actor(asserted_client_id, &grant, Some(&conversation_handle));
   445	        let wire_actor = McpActor::Client {
   446	            client_id: asserted_client_id.clone(),
   447	            conversation_handle: Some(conversation_handle.clone()),
   448	            tool_name: envelope.tool_name.clone(),
   449	            granted_scopes: grant.scopes_granted.clone(),
   450	        };
   451	        let invocation = handler.invoke(&wire_actor, envelope.params.clone());
   452	
   453	        let result = match invocation {
   454	            Ok(value) => {
   455	                // Extract + cap mutation_cursor for Side::Write per AC-6.
   456	                let mutation_cursor = if matches!(handler.description().side, Side::Write) {
   457	                    let extracted = value.get("mutation_cursor").cloned();
   458	                    if extracted.is_none() {
   459	                        // SHOULD-with-warning per L0 cycle-6 devex MED.
   460	                        // Use emit_warning (distinct from emit_rejected) so
   461	                        // operator dashboards don't conflate auth rejections
   462	                        // with successful-write soft warnings — closes L2
   463	                        // cycle-2 convergent 3/4 MED on warning-as-rejection.
   464	                        self.emitter.emit_warning(
   465	                            asserted_client_id,
   466	                            &conversation_handle,
   467	                            &envelope.tool_name,
   468	                            "mcp_write_handler_missing_cursor",
   469	                        );
   470	                    }
   471	                    extracted.map(|cursor| {
   472	                        let (capped, was_truncated) = cap_mutation_cursor(cursor);
   473	                        if was_truncated {
   474	                            // Per L2 cycle-2 devex MED: oversize truncation
   475	                            // promised in taxonomy.rs rustdoc must emit a
   476	                            // Suite-S warning, not just silently truncate.
   477	                            self.emitter.emit_warning(
   478	                                asserted_client_id,
   479	                                &conversation_handle,
   480	                                &envelope.tool_name,
   481	                                "mcp_write_handler_cursor_truncated",
   482	                            );
   483	                        }
   484	                        capped
   485	                    })
   486	                } else {
   487	                    None
   488	                };
   489	
   490	                let audit_detail = build_audit_detail(
   491	                    asserted_client_id,
   492	                    &conversation_handle,
   493	                    &envelope.tool_name,
   494	                    &envelope.params,
   495	                    Some(&value),
   496	                    mutation_cursor.as_ref(),
   497	                );
   498	                if let Err(err) =
   499	                    audit::write(&runtime_actor, "mcp.tool_invoked", audit_detail, None)
   500	                {
   501	                    if matches!(err, audit::AuditError::DoubleFailure { .. }) {
   502	                        emit_double_failure_alert();
   503	                        return Ok(Dispatched {
   504	                            result: McpToolResult::Error {
   505	                                error: ToolError::Internal {
   506	                                    trace_id: "mcp_audit_double_failure".to_string(),
   507	                                },
   508	                            },
   509	                            next_nonce,
   510	                            conversation_handle,
   511	                        });
   512	                    }
   513	                    eprintln!("mcp_v2 audit single-failure: {err}");
   514	                }
   515	
   516	                self.emitter
   517	                    .emit_invoked(asserted_client_id, &conversation_handle, &envelope.tool_name);
   518	                McpToolResult::Ok { value }
   519	            }
   520	            Err(error) => McpToolResult::Error { error },

exec
/bin/zsh -lc 'rg -n "struct AccountOverviewInput|schema_version|account_id|subject|account_status" src-tauri/src/services/mcp_v2/taxonomy.rs src-tauri/abilities-runtime/src/abilities/account_overview.rs .docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:68:   - name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:84:     scopes_required: [dailyos.read.account_status]
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:86:       - { name: subject, schema: { type: string }, required: true, description: "Account name or handle" }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:92:         invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:93:         expectedResponseShape: { account_id: <opaque>, status: <enum>, as_of: <iso8601> }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:96:         - { prompt: "What's going on with Acme?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:97:         - { prompt: "How are things looking with the Hooli deal?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:149:| 1 | `dailyos.read.account_status` | Read | `dailyos.read.account_status` | W2 (out-of-scope handler — discovery surface from DOS-172 ranked entity lookup) |
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:1:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:13:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:193:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:197:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:201:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:213:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
src-tauri/src/services/mcp_v2/taxonomy.rs:547:            "dailyos.read.account_status",
src-tauri/src/services/mcp_v2/taxonomy.rs:702:            Some(ScopedName::new("dailyos.read.account_status")),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:30:    prompt_input_sensitivity_allowed, subject_ref_from_json, ClaimState, ClaimSubjectRef,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:39:pub struct AccountOverviewInput {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:40:    pub schema_version: u32,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:41:    pub account_id: String,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:50:    account_id: String,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:87:    schema_version = 1,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:99:        "account_subject.claim_changed",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:126:    if input.schema_version != ABILITY_SCHEMA_VERSION {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:128:            "unsupported schema_version `{}` for `{ABILITY_NAME}`",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:129:            input.schema_version
src-tauri/abilities-runtime/src/abilities/account_overview.rs:132:    let account_id = input.account_id.trim();
src-tauri/abilities-runtime/src/abilities/account_overview.rs:133:    if account_id.is_empty() {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:134:        return Err(validation_error("account_id must be non-empty"));
src-tauri/abilities-runtime/src/abilities/account_overview.rs:142:        .unwrap_or_else(|| format!("dailyos/account-overview:account:{account_id}"));
src-tauri/abilities-runtime/src/abilities/account_overview.rs:145:        account_id: account_id.to_string(),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:167:            input.account_id.clone(),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:174:    let subject_ref = SubjectRef::Account(input.account_id.clone());
src-tauri/abilities-runtime/src/abilities/account_overview.rs:175:    let subject = SubjectAttribution::direct_confident(subject_ref);
src-tauri/abilities-runtime/src/abilities/account_overview.rs:179:    provenance_builder.set_subject(subject.clone());
src-tauri/abilities-runtime/src/abilities/account_overview.rs:184:            project_claim(ctx, &input.account_id, claim, &mut provenance_builder)?
src-tauri/abilities-runtime/src/abilities/account_overview.rs:196:        &subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:213:    account_id: &str,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:217:    if !claim_is_eligible_for_account_overview(&claim, account_id)? {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:235:    let source = source_for_claim(ctx, account_id, &claim)?;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:254:    account_id: &str,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:272:    let value: Value = serde_json::from_str(&claim.subject_ref)
src-tauri/abilities-runtime/src/abilities/account_overview.rs:273:        .map_err(|error| validation_error(format!("invalid claim subject_ref JSON: {error}")))?;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:274:    match subject_ref_from_json(&value)
src-tauri/abilities-runtime/src/abilities/account_overview.rs:275:        .map_err(|error| validation_error(format!("invalid claim subject_ref: {error}")))?
src-tauri/abilities-runtime/src/abilities/account_overview.rs:277:        ClaimSubjectRef::Account { id } => Ok(id == account_id),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:325:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:333:        subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:346:            subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:365:                subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:384:        Some(EntityRef::new(format!("account:{}", input.account_id))),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:390:            schema_version: SchemaVersion(ABILITY_SCHEMA_VERSION),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:397:    attribute_static_composition_fields(provenance_builder, subject)?;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:404:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:431:        "account_id": input.account_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:457:            subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:470:            subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:482:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:512:        subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:521:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:649:        subject,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:657:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:663:        "/subject",
src-tauri/abilities-runtime/src/abilities/account_overview.rs:676:                FieldAttribution::constant(subject.clone()),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:683:            FieldAttribution::constant(subject.clone()),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:689:            FieldAttribution::constant(subject.clone()),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:698:    subject: &SubjectAttribution,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:703:        FieldAttribution::constant(subject.clone())
src-tauri/abilities-runtime/src/abilities/account_overview.rs:705:        FieldAttribution::direct(subject.clone(), source_indexes[0])
src-tauri/abilities-runtime/src/abilities/account_overview.rs:708:            subject.clone(),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:901:    account_id: &str,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:910:            entity_id: EntityId::new(account_id.to_string()),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:986:    config.ability_schema_version = SchemaVersion(ABILITY_SCHEMA_VERSION);
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1285:            schema_version: ABILITY_SCHEMA_VERSION,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1286:            account_id: "acct-fixture-1".to_string(),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1304:            subject_ref: json!({"kind": "account", "id": "acct-fixture-1"}).to_string(),
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1367:    async fn missing_account_id_rejects_before_claim_read_or_commit() {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1375:                account_id: " ".to_string(),

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/taxonomy.rs | sed -n '520,570p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   520	    let mut prev: Vec<usize> = (0..=n).collect();
   521	    let mut curr = vec![0usize; n + 1];
   522	    for i in 1..=m {
   523	        curr[0] = i;
   524	        for j in 1..=n {
   525	            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
   526	            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
   527	        }
   528	        std::mem::swap(&mut prev, &mut curr);
   529	    }
   530	    prev[n]
   531	}
   532	
   533	#[cfg(test)]
   534	mod tests {
   535	    use super::*;
   536	
   537	    #[test]
   538	    fn embedded_catalog_loads_clean_with_ten_entries() {
   539	        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");
   540	        assert_eq!(catalog.len(), 10, "expected 10 tool entries per DOS-478 §5");
   541	    }
   542	
   543	    #[test]
   544	    fn embedded_catalog_contains_required_inventory() {
   545	        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");
   546	        for name in [
   547	            "dailyos.read.account_status",
   548	            "dailyos.read.daily_briefing",
   549	            "dailyos.read.meeting_briefing",
   550	            "dailyos.read.portfolio_attention",
   551	            "dailyos.search.workspace_memory",
   552	            "dailyos.read.workspace_source_provenance",
   553	            "dailyos.write.place_document",
   554	            "dailyos.submit.note",
   555	            "dailyos.submit.action",
   556	            "dailyos.submit.action_status",
   557	        ] {
   558	            assert!(
   559	                catalog.description_for(&ScopedName::new(name)).is_some(),
   560	                "missing required tool: {name}",
   561	            );
   562	        }
   563	    }
   564	
   565	    #[test]
   566	    fn invalid_name_rejected() {
   567	        let yaml = r#"
   568	- name: NotDailyos.read.foo
   569	  side: Read
   570	  summary: x

exec
/bin/zsh -lc "nl -ba src-tauri/src/services/mcp_v2/taxonomy.rs | sed -n '680,715p'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
   680	  parameters: []
   681	  returns: { schema: {}, description: x }
   682	  examples: []
   683	  selectionFixtures:
   684	    positive: [{ prompt: a, expectedTool: dailyos.read.foo }]
   685	    negativeBroadCorpus: [{ prompt: c, expectedToolClass: external }, { prompt: d, expectedToolClass: external }]
   686	    negativeAdjacentTool: [{ prompt: e, expectedTool: dailyos.read.bar }]
   687	"#;
   688	        match YamlTaxonomyCatalog::load_from_str(yaml) {
   689	            Err(TaxonomyError::FixtureCoverage { missing, .. }) => {
   690	                assert!(missing.contains("positive"));
   691	            }
   692	            other => panic!("expected FixtureCoverage, got {other:?}"),
   693	        }
   694	    }
   695	
   696	    #[test]
   697	    fn nearest_candidate_returned_for_typo() {
   698	        let catalog = YamlTaxonomyCatalog::load_embedded().expect("embedded YAML loads");
   699	        let near = catalog.nearest_name("dailyos.read.accont_status"); // typo
   700	        assert_eq!(
   701	            near,
   702	            Some(ScopedName::new("dailyos.read.account_status")),
   703	            "nearest should suggest the real name",
   704	        );
   705	    }
   706	
   707	    #[test]
   708	    fn levenshtein_basic() {
   709	        assert_eq!(levenshtein("kitten", "sitting"), 3);
   710	        assert_eq!(levenshtein("", "abc"), 3);
   711	        assert_eq!(levenshtein("abc", ""), 3);
   712	        assert_eq!(levenshtein("same", "same"), 0);
   713	    }
   714	}

exec
/bin/zsh -lc "rg -n \"pub struct AbilityResponseJson|struct AbilityResponseJson|data:|invocation_id|render_cached_provenance\" src-tauri/src src-tauri/abilities-runtime/src -g '*.rs'" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
src-tauri/src/reports/generator.rs:24:    pub extra_data: Option<String>,
src-tauri/src/reports/swot.rs:225:        extra_data: None,
src-tauri/src/reports/ebr_qbr.rs:275:        extra_data: None,
src-tauri/src/reports/weekly_impact.rs:394:        extra_data: None,
src-tauri/src/reports/account_health.rs:84:    // Gather supplemental data: meeting count (90d), email signal count, renewal date
src-tauri/src/reports/account_health.rs:298:        extra_data: None,
src-tauri/src/reports/monthly_wrapped.rs:615:        extra_data: None,
src-tauri/src/reports/book_of_business.rs:1064:        extra_data: Some(extra_data),
src-tauri/src/google_drive/client.rs:153:        .map_err(|e| format!("Failed to fetch file metadata: {}", e))?;
src-tauri/src/google_drive/client.rs:155:    let metadata: serde_json::Value = metadata_response
src-tauri/src/google_drive/client.rs:158:        .map_err(|e| format!("Failed to parse metadata: {}", e))?;
src-tauri/src/services/source_asof_backfill.rs:547:    mut metadata: Value,
src-tauri/src/services/source_asof_backfill.rs:582:    metadata: &Value,
src-tauri/src/services/source_asof_backfill.rs:639:fn backfill_mechanism(metadata: &Value, provenance: &Value) -> Option<String> {
src-tauri/src/services/source_asof_backfill.rs:644:fn metadata_indicates_known_timestamp_source(mechanism: Option<&str>, metadata: &Value) -> bool {
src-tauri/src/services/source_asof_backfill.rs:666:    metadata: &Value,
src-tauri/src/services/source_asof_backfill.rs:688:    metadata: &Value,
src-tauri/src/services/source_asof_backfill.rs:698:fn subject_kind_and_id(row: &LegacyClaimRow, metadata: &Value) -> Option<(String, String)> {
src-tauri/src/services/source_asof_backfill.rs:841:fn legacy_entity_id(row: &LegacyClaimRow, metadata: &Value) -> String {
src-tauri/src/services/source_asof_backfill.rs:851:fn legacy_field_path(row: &LegacyClaimRow, metadata: &Value) -> String {
src-tauri/src/services/source_asof_backfill.rs:863:fn legacy_item_hash(row: &LegacyClaimRow, metadata: &Value) -> Option<String> {
src-tauri/src/services/source_asof_backfill.rs:923:        metadata: Value,
src-tauri/src/services/source_asof_backfill.rs:941:        metadata: Value,
src-tauri/src/services/context.rs:481:                "invalid metadata: {message}"
src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1989:                fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/abilities/prepare_meeting/synthesis.rs:1993:                    ..FingerprintMetadata::default()
src-tauri/abilities-runtime/src/abilities/list_projects/mod.rs:158:                fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/abilities/composition.rs:289:/// for `invocation_id` from the runtime provenance store, reading the
src-tauri/abilities-runtime/src/abilities/composition.rs:296:    pub invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/composition.rs:301:    pub fn new(invocation_id: InvocationId, field_path: FieldPath) -> Self {
src-tauri/abilities-runtime/src/abilities/composition.rs:303:            invocation_id,
src-tauri/abilities-runtime/src/abilities/composition.rs:312:        invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/composition.rs:316:            invocation_id,
src-tauri/abilities-runtime/src/abilities/composition.rs:540:        if provenance.invocation_id == InvocationId(uuid::Uuid::nil()) {
src-tauri/abilities-runtime/src/abilities/composition.rs:570:        if provenance.invocation_id != envelope.invocation_id {
src-tauri/abilities-runtime/src/abilities/composition.rs:630:    pub metadata: CompositionMetadata,
src-tauri/abilities-runtime/src/abilities/composition.rs:656:        metadata: CompositionMetadata,
src-tauri/abilities-runtime/src/abilities/composition.rs:695:            metadata: CompositionMetadata {
src-tauri/abilities-runtime/src/abilities/composition.rs:719:    #[error("ProvenanceRef.invocation_id is nil; blocks must reference a real ability invocation")]
src-tauri/abilities-runtime/src/abilities/composition.rs:721:    #[error("ProvenanceRef.invocation_id does not match the canonical envelope")]
src-tauri/abilities-runtime/src/abilities/composition.rs:1215:        // Pin the invocation_id to match sample_provenance_ref so resolution
src-tauri/abilities-runtime/src/abilities/composition.rs:1217:        envelope.invocation_id = InvocationId(uuid::Uuid::from_u128(
src-tauri/abilities-runtime/src/abilities/composition.rs:1244:            envelope.invocation_id,
src-tauri/abilities-runtime/src/abilities/list_people/mod.rs:153:                fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/abilities/provenance/builder.rs:27:    pub invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/builder.rs:42:            invocation_id: InvocationId::new(
src-tauri/abilities-runtime/src/abilities/provenance/builder.rs:184:    pub fn finalize<T: Serialize>(mut self, data: T) -> Result<AbilityOutput<T>, ProvenanceError> {
src-tauri/abilities-runtime/src/abilities/provenance/builder.rs:209:            invocation_id: self.config.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/builder.rs:735:            fingerprint_metadata: FingerprintMetadata {
src-tauri/src/services/dashboard.rs:139:        data: DashboardData,
src-tauri/src/services/dashboard.rs:163:    Success { data: WeekOverview },
src-tauri/src/services/dashboard.rs:1366:        data: DashboardData {
src-tauri/src/services/dashboard.rs:1565:    WeekResult::Success { data: week }
src-tauri/abilities-runtime/src/abilities/list_accounts/mod.rs:166:                fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/abilities/provenance/trust.rs:250:            producer_invocation_id: InvocationId::new(
src-tauri/abilities-runtime/src/abilities/provenance/trust.rs:289:            invocation_id: InvocationId::new(
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:361:    invocation_id: crate::abilities::provenance::InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:375:        invocation_id: crate::abilities::provenance::InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:379:        Self::issue_at(audience, invocation_id, user_id, expiry_secs, Utc::now())
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:388:        invocation_id: crate::abilities::provenance::InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:400:            invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:411:        invocation_id: crate::abilities::provenance::InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:421:            && self.claims.invocation_id == invocation_id
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:638:        "invocation_id",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:657:        "children.[].provenance.invocation_id",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:693:        "invocation_id",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:710:        "invocation_id",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:772:        "invocation_id",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1159:        object.insert("invocation_id".to_string(), json!(prov.invocation_id));
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1331:            "original_invocation_id".to_string(),
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1332:            json!(masked.original_invocation_id),
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1358:            "original_invocation_id".to_string(),
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1359:            json!(masked.original_invocation_id),
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1411:        "original_invocation_id": masked.original_invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1451:        .map(|(token, user_id)| token.is_valid_for_p2_detail(prov.invocation_id, user_id))
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1485:        "invocation_id": prov.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:1612:        "invocation_id": child.provenance.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:2558:            provenance.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:2831:        assert!(user_detail.value.get("invocation_id").is_none());
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:2835:        assert!(admin_detail.value.get("invocation_id").is_some());
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:3027:            original_invocation_id: provenance().invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:3083:            "invocation_id": "secret-invocation",
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:3293:            provenance.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/render.rs:3314:            provenance.invocation_id,
src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs:228:    data: Value,
src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs:293:    data: &Value,
src-tauri/abilities-runtime/src/abilities/provenance/ownership.rs:712:fn rendered_text_at_path(data: &Value, field_path: &FieldPath) -> String {
src-tauri/src/services/composition_projection.rs:63:                        provenance_invocation_id: block
src-tauri/src/services/composition_projection.rs:66:                            .map(|provenance| provenance.invocation_id.0.to_string()),
src-tauri/abilities-runtime/src/abilities/list_open_loops/mod.rs:387:fn metadata_string(metadata: Option<&Value>, keys: &[&str]) -> Option<String> {
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:304:    pub(in crate::abilities::provenance) data: T,
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:311:    pub(in crate::abilities::provenance) fn new(data: T, provenance: Provenance) -> Self {
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:349:    pub invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:501:    pub original_invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:541:        invocation_id: InvocationId::new(
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:621:    fn schemars_schema_for_invocation_id_is_uuid_string() {
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:776:    fn invocation_id_rejects_non_uuid_string() {
src-tauri/abilities-runtime/src/abilities/provenance/envelope.rs:789:    fn invocation_id_accepts_well_formed_uuid_string() {
src-tauri/abilities-runtime/src/abilities/provenance/source.rs:297:    pub producer_invocation_id: crate::abilities::provenance::InvocationId,
src-tauri/src/util.rs:274:- **dashboard.json** — Mechanical data: lifecycle, health, team, domains, metadata
src-tauri/src/hygiene/mod.rs:388:            metadata: None,
src-tauri/src/hygiene/mod.rs:418:            metadata: None,
src-tauri/src/services/fail_improve.rs:140:    #[error("failed to serialize fail-improve data: {0}")]
src-tauri/src/services/fail_improve.rs:345:            invocation_id: format!("fi-{:016x}", ctx.rng.random_u64()),
src-tauri/src/services/fail_improve.rs:372:                    invocation_id: entry.invocation_id,
src-tauri/src/services/fail_improve.rs:395:            invocation_id: format!("fi-{}", Uuid::new_v4().simple()),
src-tauri/src/services/fail_improve.rs:667:    pub invocation_id: String,
src-tauri/src/services/fail_improve.rs:677:    pub invocation_id: String,
src-tauri/src/services/fail_improve.rs:692:    pub invocation_id: String,
src-tauri/src/services/fail_improve.rs:931:            fingerprint_metadata: Default::default(),
src-tauri/src/services/fail_improve.rs:1234:                fingerprint_metadata: Default::default(),
src-tauri/src/services/fail_improve.rs:1314:        assert!(entry.invocation_id.starts_with("fi-"));
src-tauri/src/migrations/v166_semantic_merge_safety.rs:75:        .map_err(|e| format!("update legacy claim metadata: {e}"))?;
src-tauri/src/migrations/v166_semantic_merge_safety.rs:458:        metadata: Option<Value>,
src-tauri/src/migrations/v166_semantic_merge_safety.rs:472:        let metadata: String = conn
src-tauri/abilities-runtime/src/abilities/account_overview.rs:177:    let invocation_id = provenance_config.invocation_id;
src-tauri/abilities-runtime/src/abilities/account_overview.rs:197:        invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:326:    invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:334:        invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:347:            invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:366:                invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:405:    invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:444:            invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:483:    invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:498:            invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:522:    invocation_id: InvocationId,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:638:            invocation_id,
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1181:                    metadata: CompositionMetadata {
src-tauri/abilities-runtime/src/abilities/account_overview.rs:1207:                fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/abilities/feedback.rs:234:    pub requires_action_metadata: bool,
src-tauri/abilities-runtime/src/abilities/feedback.rs:255:            requires_action_metadata: false,
src-tauri/abilities-runtime/src/abilities/feedback.rs:269:            requires_action_metadata: false,
src-tauri/abilities-runtime/src/abilities/feedback.rs:283:            requires_action_metadata: false,
src-tauri/abilities-runtime/src/abilities/feedback.rs:298:            requires_action_metadata: false,
src-tauri/abilities-runtime/src/abilities/feedback.rs:312:            requires_action_metadata: true,
src-tauri/abilities-runtime/src/abilities/feedback.rs:321:            requires_action_metadata: false,
src-tauri/abilities-runtime/src/abilities/feedback.rs:338:            requires_action_metadata: true,
src-tauri/abilities-runtime/src/abilities/feedback.rs:347:            requires_action_metadata: true,
src-tauri/abilities-runtime/src/abilities/feedback.rs:356:            requires_action_metadata: true,
src-tauri/abilities-runtime/src/abilities/feedback.rs:374:            requires_action_metadata: true,
src-tauri/src/operations/mod.rs:785:            "invocation_id": output["provenance"]["invocation_id"].clone(),
src-tauri/src/operations/mod.rs:841:            "invocation_id": output["provenance"]["invocation_id"].clone(),
src-tauri/src/entity_io.rs:40:    data: &T,
src-tauri/src/commands/workspace.rs:421:    Success { data: Vec<crate::types::Email> },
src-tauri/src/commands/workspace.rs:445:                EmailsResult::Success { data: emails }
src-tauri/src/migrations/v172_dos_567_w4b_versions_and_outbox.rs:64:            generated_by_invocation_id TEXT NOT NULL,
src-tauri/src/google.rs:1089:    let refresh_data: serde_json::Value = serde_json::from_str(&raw)
src-tauri/src/google.rs:1205:    let data: serde_json::Value = match serde_json::from_str(&raw) {
src-tauri/src/google.rs:1704:            metadata: None,
src-tauri/src/self_healing/remediation.rs:181:        // With no data: imminence=0, staleness=1, quality_deficit=0.5, importance=0, signals=0
src-tauri/src/harness/runner.rs:437:                fingerprint_metadata: FingerprintMetadata::default(),
src-tauri/src/harness/runner.rs:489:    let default = FingerprintMetadata::default();
src-tauri/src/commands/integrations.rs:1157:    Ok(Some(format!("data:image/png;base64,{}", b64)))
src-tauri/src/commands/integrations.rs:2239:    metadata: String,
src-tauri/src/commands/integrations.rs:2243:        .map_err(|e| format!("Invalid JSON metadata: {}", e))?;
src-tauri/src/harness/bundle_helpers.rs:258:                "invocation_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
src-tauri/src/mcp/main.rs:736:    let mut invocation_id_schema = JsonObject::new();
src-tauri/src/mcp/main.rs:737:    invocation_id_schema.insert(
src-tauri/src/mcp/main.rs:744:        "invocation_id".to_string(),
src-tauri/src/mcp/main.rs:745:        serde_json::Value::Object(invocation_id_schema),
src-tauri/src/mcp/main.rs:763:        serde_json::Value::Array(vec![serde_json::Value::String("invocation_id".to_string())]),
src-tauri/src/mcp/main.rs:891:    let invocation_id = get_provenance_invocation_id(&request)?;
src-tauri/src/mcp/main.rs:892:    Ok(ability_bridge.get_provenance_tool_response(session_id, invocation_id))
src-tauri/src/mcp/main.rs:941:fn get_provenance_invocation_id(request: &CallToolRequestParam) -> Result<InvocationId, McpError> {
src-tauri/src/mcp/main.rs:954:    let Some(invocation_id) = arguments
src-tauri/src/mcp/main.rs:955:        .get("invocation_id")
src-tauri/src/mcp/main.rs:963:    InvocationId::parse(invocation_id)
src-tauri/src/mcp/main.rs:1853:        let invocation_id = ability_value["invocation_id"].as_str().unwrap();
src-tauri/src/mcp/main.rs:1858:            request("get_provenance", json!({ "invocation_id": invocation_id })),
src-tauri/src/mcp/main.rs:1865:        assert_eq!(provenance_value["value"]["invocation_id"], invocation_id);
src-tauri/src/mcp/main.rs:1870:            request("get_provenance", json!({ "invocation_id": invocation_id })),
src-tauri/abilities-runtime/src/observability/mod.rs:14:    pub invocation_id: Uuid,
src-tauri/abilities-runtime/src/observability/mod.rs:90:            invocation_id: Uuid::new_v4(),
src-tauri/abilities-runtime/src/observability/mod.rs:106:        assert_ne!(rec.invocation_id, Uuid::nil());
src-tauri/src/processor/router.rs:608:            metadata: None,
src-tauri/src/bridges/mcp.rs:425:            response.invocation_id,
src-tauri/src/bridges/mcp.rs:426:            response.render_cached_provenance(BridgeActor::Agent, BridgeSurface::McpToolDetail),
src-tauri/src/bridges/mcp.rs:433:    /// (session, invocation_id) pair was previously returned by this same
src-tauri/src/bridges/mcp.rs:438:        invocation_id: InvocationId,
src-tauri/src/bridges/mcp.rs:443:            .get(&(session, invocation_id))
src-tauri/src/bridges/mcp.rs:449:        invocation_id: InvocationId,
src-tauri/src/bridges/mcp.rs:451:        match self.get_provenance(session, invocation_id) {
src-tauri/src/bridges/mcp.rs:525:        invocation_id: InvocationId,
src-tauri/src/bridges/mcp.rs:528:        let key = (session, invocation_id);
src-tauri/src/bridges/mcp.rs:688:    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
src-tauri/src/bridges/mcp.rs:689:        let invocation_id = data
src-tauri/src/bridges/mcp.rs:690:            .get("invocation_id")
src-tauri/src/bridges/mcp.rs:698:            "provenance": typed_provenance_json(ctx, invocation_id)
src-tauri/src/bridges/mcp.rs:702:    fn typed_provenance_json(ctx: &AbilityContext<'_>, invocation_id: &str) -> serde_json::Value {
src-tauri/src/bridges/mcp.rs:713:        provenance.invocation_id =
src-tauri/src/bridges/mcp.rs:714:            InvocationId::parse(invocation_id).expect("test invocation id is valid");
src-tauri/src/bridges/mcp.rs:834:                "invocation_id": uuid::Uuid::from_u128(index).to_string(),
src-tauri/src/bridges/mcp.rs:844:                "invocation_id": uuid::Uuid::from_u128(index).to_string(),
src-tauri/src/bridges/mcp.rs:1392:            .get_provenance(session, response.invocation_id)
src-tauri/src/bridges/mcp.rs:1500:            .get_provenance(session, response.invocation_id)
src-tauri/src/bridges/mcp.rs:1510:                rendered["invocation_id"],
src-tauri/src/bridges/mcp.rs:1519:    fn mcp_bridge_get_provenance_returns_none_for_unknown_session_or_invocation_id() {
src-tauri/src/bridges/mcp.rs:1530:    fn mcp_bridge_get_provenance_rejects_cross_session_invocation_id() {
src-tauri/src/bridges/mcp.rs:1569:    fn mcp_bridge_get_provenance_tool_response_rejects_cross_session_invocation_id() {
src-tauri/src/workflow/deliver.rs:659:pub fn write_json(path: &Path, data: &Value) -> Result<(), String> {
src-tauri/src/workflow/deliver.rs:4310:    schedule_data: &Value,
src-tauri/src/workflow/deliver.rs:4311:    actions_data: &Value,
src-tauri/src/workflow/deliver.rs:4312:    emails_data: &Value,
src-tauri/src/harness/types.rs:63:    pub metadata: FixtureMetadata,
src-tauri/src/commands/app_support.rs:255:            metadata: None,
src-tauri/src/commands/app_support.rs:296:                    metadata: existing.as_ref().and_then(|e| e.metadata.clone()),
src-tauri/src/linear/client.rs:126:            .map_err(|e| format!("Failed to deserialize Linear data: {}", e))
src-tauri/src/bridges/eval.rs:217:    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
src-tauri/src/bridges/eval.rs:223:                "invocation_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
src-tauri/src/gravatar/cache.rs:50:pub fn upsert_cache(conn: &Connection, data: &CachedGravatar) -> Result<(), String> {
src-tauri/src/processor/mod.rs:473:        let meta = metadata::parse_action_metadata(raw_title);
src-tauri/abilities-runtime/src/intelligence/provider.rs:155:    pub fingerprint_metadata: FingerprintMetadata,
src-tauri/abilities-runtime/src/intelligence/provider.rs:348:            let key = replay_fixture_key(&p, &FingerprintMetadata::default());
src-tauri/abilities-runtime/src/intelligence/provider.rs:353:                    fingerprint_metadata: FingerprintMetadata::default(),
src-tauri/abilities-runtime/src/intelligence/provider.rs:483:            fingerprint_metadata: FingerprintMetadata {
src-tauri/abilities-runtime/src/intelligence/provider.rs:487:                ..FingerprintMetadata::default()
src-tauri/abilities-runtime/src/intelligence/provider.rs:546:                replay_fixture_key(&p, &FingerprintMetadata::default()),
src-tauri/abilities-runtime/src/intelligence/provider.rs:569:        let meta = FingerprintMetadata::default();
src-tauri/src/bridges/surface_client.rs:1182:            Ok(json!({"data": {}, "provenance": {"invocation_id": "inv_test"}, "diagnostics": {}}))
src-tauri/src/commands.rs:59:pub use projects_data::*;
src-tauri/src/signals/scoring.rs:376:            metadata: crate::presets::schema::PresetMetadataConfig {
src-tauri/src/bridges/worker.rs:120:                    "invocation_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:22:    pub fingerprint_metadata: &'a FingerprintMetadata,
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:40:pub fn replay_fixture_key(prompt: &PromptInput, metadata: &FingerprintMetadata) -> String {
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:43:        fingerprint_metadata: metadata,
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:220:        let meta = FingerprintMetadata::default();
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:241:        let meta = FingerprintMetadata::default();
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:278:                fingerprint_metadata: &meta,
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:291:                fingerprint_metadata: &meta,
src-tauri/abilities-runtime/src/intelligence/prompt_fingerprint.rs:334:            fingerprint_metadata: metadata(),
src-tauri/src/bridges/tauri.rs:595:    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
src-tauri/src/bridges/tauri.rs:615:        provenance.invocation_id = InvocationId::parse("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
src-tauri/src/intel_queue.rs:4992:    fn trust_recompute_does_not_require_bridge_invocation_id() {
src-tauri/src/intel_queue.rs:5021:            assert!(!value.contains("invocation_id"));
src-tauri/src/intel_queue.rs:5069:                    let metadata: serde_json::Value = serde_json::from_str(&metadata_json).ok()?;
src-tauri/src/processor/hooks.rs:321:            metadata: None,
src-tauri/src/processor/hooks.rs:376:            metadata: None,
src-tauri/src/bridges/types.rs:248:pub struct AbilityResponseJson {
src-tauri/src/bridges/types.rs:249:    pub invocation_id: InvocationId,
src-tauri/src/bridges/types.rs:253:    pub data: serde_json::Value,
src-tauri/src/bridges/types.rs:268:    pub(crate) fn render_cached_provenance(
src-tauri/src/bridges/types.rs:291:        state.serialize_field("invocation_id", &self.invocation_id)?;
src-tauri/src/bridges/types.rs:888:    let invocation_id = parse_invocation_id(&provenance)?;
src-tauri/src/bridges/types.rs:891:        invocation_id,
src-tauri/src/bridges/types.rs:895:        data: render_ability_data(surface, data, &provenance),
src-tauri/src/bridges/types.rs:902:fn parse_invocation_id(provenance: &serde_json::Value) -> Result<InvocationId, AbilityInvokeError> {
src-tauri/src/bridges/types.rs:903:    let invocation_id = provenance
src-tauri/src/bridges/types.rs:904:        .get("invocation_id")
src-tauri/src/bridges/types.rs:907:    InvocationId::parse(invocation_id).map_err(|_| AbilityInvokeError::InvalidEnvelope)
src-tauri/src/bridges/types.rs:961:    data: serde_json::Value,
src-tauri/src/bridges/types.rs:976:    data: serde_json::Value,
src-tauri/src/bridges/types.rs:1291:        invocation_id: InvocationId,
src-tauri/src/bridges/types.rs:1299:        let key = (session_id, invocation_id);
src-tauri/src/bridges/types.rs:1318:        invocation_id: InvocationId,
src-tauri/src/bridges/types.rs:1321:            .get(&(session_id, invocation_id))
src-tauri/src/commands/core.rs:116:    Success { data: FullMeetingPrep },
src-tauri/src/clay/client.rs:542:/// Smithery may return SSE-formatted responses with `data:` prefixed lines.
src-tauri/src/clay/client.rs:548:    // Look for `data:` lines in SSE format
src-tauri/src/clay/client.rs:551:        if let Some(data) = line.strip_prefix("data:") {
src-tauri/src/observability/aggregate_metric/mod.rs:440:        assert!(serialized.get("invocation_id").is_none());
src-tauri/src/processor/transcript.rs:381:                let meta = super::metadata::parse_action_metadata(raw);
src-tauri/src/processor/transcript.rs:1227:fn generate_meeting_record_markdown(data: &MeetingRecordData<'_>) -> String {
src-tauri/src/processor/transcript.rs:1440:fn generate_and_persist_meeting_record(workspace: &Path, data: &MeetingRecordData<'_>) {
src-tauri/src/processor/transcript.rs:1881:        let meta = super::metadata::parse_action_metadata(raw_title);
src-tauri/src/processor/transcript.rs:1995:fn persist_enriched_transcript_data(db: &crate::db::ActionDb, data: &EnrichedTranscriptData<'_>) {
src-tauri/src/processor/transcript.rs:4007:            metadata: None,
src-tauri/src/observability/aggregate_metric/lint.rs:11:    "invocation_id",
src-tauri/src/observability/aggregate_metric/install_id.rs:178:    let Ok(name) = CString::new("com.apple.metadata:com_apple_backup_excludeItem") else {
src-tauri/src/processor/enrich.rs:861:        let meta = metadata::parse_action_metadata(raw_title);
src-tauri/src/executor.rs:34:    schedule_data: serde_json::Value,
src-tauri/src/executor.rs:35:    actions_data: serde_json::Value,
src-tauri/src/executor.rs:36:    emails_data: serde_json::Value,
src-tauri/src/accounts.rs:1086:            metadata: None,
src-tauri/src/accounts.rs:1394:            metadata: None,
src-tauri/src/accounts.rs:1477:            metadata: None,
src-tauri/src/accounts.rs:1499:            metadata: None,
src-tauri/src/accounts.rs:1520:            metadata: None,
src-tauri/src/accounts.rs:1560:            metadata: None,
src-tauri/src/accounts.rs:1582:            metadata: None,
src-tauri/src/accounts.rs:1603:            metadata: None,
src-tauri/src/devtools/mod.rs:417:    pub has_today_data: bool,
src-tauri/src/devtools/mod.rs:1092:/// Seed Linear mock data: projects, issues, entity links, action links.
src-tauri/src/devtools/mod.rs:1131:    log::info!("seed_linear_mock_data: 3 projects, 10 issues, 2 entity links, 2 action links");
src-tauri/src/devtools/mod.rs:1135:/// Seed Glean-enriched intelligence data: Gong summaries, REDACTED context, support health.
src-tauri/src/devtools/mod.rs:1191:    log::info!("seed_glean_enriched_data: Gong + REDACTED + Zendesk data patched");
src-tauri/src/devtools/mod.rs:5028:            sufficient_data: true,
src-tauri/src/devtools/mod.rs:5248:            sufficient_data: true,
src-tauri/src/devtools/mod.rs:5458:            sufficient_data: true,
src-tauri/src/devtools/mod.rs:5695:            sufficient_data: true,
src-tauri/src/processor/metadata.rs:64:    let mut meta = ActionMetadata::default();
src-tauri/src/json_loader.rs:205:    pub account_data: Option<serde_json::Value>,
src-tauri/src/json_loader.rs:217:    pub project_data: Option<serde_json::Value>,
src-tauri/src/json_loader.rs:220:    pub person_data: Option<serde_json::Value>,
src-tauri/src/glean/oauth.rs:327:            "Failed to parse resource metadata: {} (body starts with: {})",
src-tauri/src/glean/oauth.rs:430:            "Failed to parse AS metadata: {} (body starts with: {})",
src-tauri/src/services/surface_pairing.rs:74:    pub client_metadata: Option<serde_json::Value>,
src-tauri/src/services/surface_pairing.rs:2810:            client_metadata: Some(json!({"plugin_version": "0.0.0"})),
src-tauri/src/db/projects.rs:28:            metadata: row.get(12).unwrap_or(None),
src-tauri/src/db/types.rs:179:    pub metadata: Option<String>,
src-tauri/src/db/types.rs:791:    pub metadata: Option<String>,
src-tauri/src/db/mod_tests.rs:671:        metadata: None,
src-tauri/src/db/mod_tests.rs:713:        metadata: None,
src-tauri/src/db/mod_tests.rs:733:        metadata: None,
src-tauri/src/db/mod_tests.rs:1236:        metadata: None,
src-tauri/src/db/mod_tests.rs:1354:        metadata: None,
src-tauri/src/db/mod_tests.rs:1556:        metadata: None,
src-tauri/src/db/mod_tests.rs:1578:        metadata: None,
src-tauri/src/db/mod_tests.rs:1600:        metadata: None,
src-tauri/src/db/mod_tests.rs:1631:        metadata: None,
src-tauri/src/db/mod_tests.rs:1653:        metadata: None,
src-tauri/src/db/mod_tests.rs:1685:        metadata: None,
src-tauri/src/db/mod_tests.rs:1722:        metadata: None,
src-tauri/src/db/mod_tests.rs:1756:        metadata: None,
src-tauri/src/db/mod_tests.rs:1883:        metadata: None,
src-tauri/src/db/mod_tests.rs:1917:    // No data: should be stable
src-tauri/src/db/mod_tests.rs:2238:        metadata: None,
src-tauri/src/db/mod_tests.rs:2647:        metadata: None,
src-tauri/src/db/mod_tests.rs:2788:        metadata: None,
src-tauri/src/db/mod_tests.rs:3942:        metadata: None,
src-tauri/src/db/mod_tests.rs:4234:        metadata: None,
src-tauri/src/db/mod_tests.rs:5656:        metadata: None,
src-tauri/src/db/mod_tests.rs:5845:        metadata: None,
src-tauri/src/services/composition_render_orchestrator.rs:175:    data: &serde_json::Value,
src-tauri/src/google_api/gmail.rs:328:    data: Option<String>,
src-tauri/src/google_api/gmail.rs:405:fn decode_url_safe_base64(data: &str) -> Option<String> {
src-tauri/src/services/projection_signing.rs:220:    pub provenance_invocation_id: Option<String>,
src-tauri/src/services/projection_signing.rs:276:    pub provenance_invocation_id: Option<String>,
src-tauri/src/services/projection_signing.rs:1296:                     field_path, provenance_invocation_id, provenance_field_path, scope_grant_hash)
src-tauri/src/services/projection_signing.rs:1305:                    claim_ref.provenance_invocation_id,
src-tauri/src/services/projection_signing.rs:1751:                        provenance_invocation_id: claim_ref.provenance_invocation_id.clone(),
src-tauri/src/services/projection_signing.rs:1854:                    "provenance_invocation_id": claim_ref.provenance_invocation_id,
src-tauri/src/services/projection_signing.rs:2409:                        provenance_invocation_id: Some("invocation-1".to_string()),
src-tauri/src/services/projection_signing.rs:2423:                        provenance_invocation_id: Some("invocation-2".to_string()),
src-tauri/src/intelligence/glean_provider.rs:941:            fingerprint_metadata: FingerprintMetadata {
src-tauri/src/db/accounts.rs:1998:            _ => return Err(format!("Invalid entity type for metadata: {}", entity_type)),
src-tauri/src/db/accounts.rs:2003:            .map_err(|e| format!("Failed to update metadata: {}", e))?;
src-tauri/src/db/accounts.rs:2016:            _ => return Err(format!("Invalid entity type for metadata: {}", entity_type)),
src-tauri/src/db/accounts.rs:2024:            .map_err(|e| format!("Failed to get metadata: {}", e))
src-tauri/src/db/accounts.rs:2550:            metadata: row.get(15).unwrap_or(None),
src-tauri/src/db/accounts.rs:3301:        ref_data: &AccountSourceRef<'_>,
src-tauri/src/services/compositions.rs:281:                     generated_by_invocation_id = ?4,
src-tauri/src/services/compositions.rs:288:                    generated_by_invocation_id(ctx, &proposal.composition),
src-tauri/src/services/compositions.rs:316:                  generated_by_invocation_id, generated_by_actor_kind)
src-tauri/src/services/compositions.rs:321:                    generated_by_invocation_id(ctx, &proposal.composition),
src-tauri/src/services/compositions.rs:375:fn generated_by_invocation_id(ctx: &ServiceContext<'_>, composition: &Composition) -> String {
src-tauri/src/intelligence/compute.rs:462:            metadata: None,
src-tauri/src/services/claim_receipt/privacy.rs:162:    #[error("invalid claim metadata: {0}")]
src-tauri/src/migrations.rs:2129:        out.insert(col.map_err(|e| format!("Failed reading column metadata: {e}"))?);
src-tauri/src/presets/schema.rs:64:    pub metadata: PresetMetadataConfig,
src-tauri/src/services/surface_nonce.rs:1805:                    generated_by_invocation_id, generated_by_actor_kind
src-tauri/src/intelligence/pty_provider.rs:186:            fingerprint_metadata: FingerprintMetadata {
src-tauri/src/intelligence/pty_provider.rs:242:            fingerprint_metadata: FingerprintMetadata {
src-tauri/src/intelligence/pty_provider.rs:329:            ..FingerprintMetadata::default()
src-tauri/src/services/entity_linking/cascade.rs:502:            metadata: None,
src-tauri/src/surface_runtime/mod.rs:4171:                    "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
src-tauri/src/surface_runtime/mod.rs:4197:                    "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
src-tauri/src/surface_runtime/mod.rs:5015:            client_metadata: Some(json!({"plugin_version": "0.0.0"})),
src-tauri/src/surface_runtime/mod.rs:6055:            "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
src-tauri/src/intelligence/prompts.rs:1421:    // Collect child data: (name, id, health, intel, signal_count, latest_signal_time)
src-tauri/src/intelligence/prompts.rs:1422:    let mut child_data: Vec<(
src-tauri/src/intelligence/prompts.rs:1548:    // Collect child data: (name, id, status, intel, signals)
src-tauri/src/intelligence/prompts.rs:1549:    let mut child_data: Vec<(
src-tauri/src/intelligence/prompts.rs:2652:        sufficient_data: false, // DOS-84: LLM-derived scores lack dimension data
src-tauri/src/intelligence/prompts.rs:4102:            metadata: None,
src-tauri/src/services/entity_linking/stakeholder_domains.rs:226:            metadata: None,
src-tauri/src/services/actions.rs:366:    Success { data: Vec<Action> },
src-tauri/src/services/actions.rs:404:        ActionsResult::Success { data: actions }
src-tauri/src/services/intelligence.rs:2387:            metadata: None,
src-tauri/src/services/intelligence.rs:4489:                sufficient_data: false, // Only 1 dimension populated in test
src-tauri/src/intelligence/io.rs:230:    pub sufficient_data: bool,
src-tauri/src/intelligence/io.rs:2025:        sufficient_data: false, // DOS-84: DB-restored scores lack dimension context
src-tauri/src/intelligence/io.rs:3029:            metadata: None,
src-tauri/src/intelligence/io.rs:3083:            metadata: None,
src-tauri/src/intelligence/health_scoring.rs:1026:                "Gong call data: {} signals, avg confidence {:.0}%",
src-tauri/src/intelligence/health_scoring.rs:1607:            metadata: None,
src-tauri/src/intelligence/health_scoring.rs:1933:            sufficient_data: true,
src-tauri/src/intelligence/health_scoring.rs:2136:        // Simulate sparse data: only one dimension available
src-tauri/src/intelligence/health_scoring.rs:2252:            sufficient_data: true,
src-tauri/src/db_backup.rs:115:fn backup_created_at(path: &Path, metadata: &fs::Metadata) -> String {
src-tauri/src/db_backup.rs:257:            .map_err(|e| format!("Failed to inspect backup metadata: {e}"))?;
src-tauri/src/db_backup.rs:452:        .map_err(|e| format!("Failed to read database metadata: {e}"))?
src-tauri/src/parser.rs:236:        // Meeting metadata: type: customer, account: Acme Corp
src-tauri/src/services/claim_receipt/render.rs:131:        PrivacyError::InvalidMetadata(m) => format!("{PRIVACY_TAG_STORAGE}invalid metadata: {m}"),
src-tauri/src/intelligence/predictions.rs:24:    pub has_data: bool,
src-tauri/src/intelligence/predictions.rs:632:            has_data: true,
src-tauri/src/services/claims.rs:1643:            generated_by_invocation_id TEXT NOT NULL,
src-tauri/src/services/claims.rs:5151:    metadata: &ClaimFeedbackMetadata,
src-tauri/src/services/claims.rs:5188:        FeedbackAction::NotRelevantHere => require_payload_string(action, payload, "invocation_id"),
src-tauri/src/services/claims.rs:5280:    mut metadata: ClaimFeedbackMetadata,
src-tauri/src/services/claims.rs:6981:    invocation_id: Option<String>,
src-tauri/src/services/claims.rs:7048:    let Some(invocation_id) = route.invocation_id.as_deref() else {
src-tauri/src/services/claims.rs:7063:            count_claims_for_ability_invocation(tx, &ability_id, invocation_id)?;
src-tauri/src/services/claims.rs:7068:                invocation_id,
src-tauri/src/services/claims.rs:7119:    let mut route = ClaimCommitRouteMetadata::default();
src-tauri/src/services/claims.rs:7149:        route.invocation_id = route.invocation_id.or_else(|| {
src-tauri/src/services/claims.rs:7153:                    &["invocation_id"][..],
src-tauri/src/services/claims.rs:7154:                    &["producer_invocation_id"][..],
src-tauri/src/services/claims.rs:7155:                    &["claim_generation", "invocation_id"][..],
src-tauri/src/services/claims.rs:7187:    route.invocation_id = route
src-tauri/src/services/claims.rs:7188:        .invocation_id
src-tauri/src/services/claims.rs:7240:    invocation_id: &str,
src-tauri/src/services/claims.rs:7256:                     json_extract(metadata_json, '$.invocation_id') = ?2
src-tauri/src/services/claims.rs:7257:                     OR json_extract(metadata_json, '$.producer_invocation_id') = ?2
src-tauri/src/services/claims.rs:7258:                     OR json_extract(metadata_json, '$.claim_generation.invocation_id') = ?2
src-tauri/src/services/claims.rs:7271:                     json_extract(provenance_json, '$.invocation_id') = ?2
src-tauri/src/services/claims.rs:7272:                     OR json_extract(provenance_json, '$.producer_invocation_id') = ?2
src-tauri/src/services/claims.rs:7273:                     OR json_extract(provenance_json, '$.claim_generation.invocation_id') = ?2
src-tauri/src/services/claims.rs:7276:            params![ability_id, invocation_id],
src-tauri/src/services/claims.rs:7525:            fingerprint_metadata: &fingerprint_metadata,
src-tauri/src/services/claims.rs:11769:                Some(serde_json::json!({ "invocation_id": "invocation-fixture" }).to_string())
src-tauri/src/services/claims.rs:13472:                    "invocation_id": format!("invocation-{ability_id}"),
src-tauri/src/services/claims.rs:13516:        let invocation_id = "targeted-repair-budget-invocation";
src-tauri/src/services/claims.rs:13524:                    "invocation_id": invocation_id
src-tauri/src/services/claims.rs:13536:                "invocation_id": invocation_id
src-tauri/src/services/claims.rs:13559:                "invocation_id": "caller-reported-budget",
src-tauri/src/services/claims.rs:14723:            (FeedbackAction::NotRelevantHere, "invocation_id"),
src-tauri/src/services/claims.rs:17112:                panic!("metadata_json must be valid JSON, got {metadata:?}: {e}")
src-tauri/src/prepare/meeting_context.rs:2263:                metadata: None,
src-tauri/src/prepare/meeting_context.rs:2482:            metadata: None,
src-tauri/src/services/mutations.rs:398:    metadata: &str,
src-tauri/src/services/mutations.rs:922:            metadata: None,
src-tauri/src/services/trust_extraction.rs:406:        metadata: Option<&str>,
src-tauri/src/services/meetings.rs:5008:                        log::warn!("Failed to persist transcript metadata: {}", e);
src-tauri/src/services/meetings.rs:5082:                let role_changes_data: Vec<(String, Option<String>, Option<String>)> = result
src-tauri/src/intelligence/provider.rs:342:    mut metadata: FingerprintMetadata,
src-tauri/src/intelligence/provider.rs:362:        fingerprint_metadata: metadata,
src-tauri/src/services/claim_receipt/feedback.rs:14://!    `CannotVerify.note`, `SurfaceInappropriate.surface`, `NotRelevantHere.invocation_id`
src-tauri/src/services/claim_receipt/feedback.rs:75:    pub metadata: Option<serde_json::Value>,
src-tauri/src/services/claim_receipt/feedback.rs:421:    metadata: Option<serde_json::Value>,
src-tauri/src/services/claim_receipt/feedback.rs:433:    metadata: Option<&serde_json::Value>,
src-tauri/src/services/claim_receipt/feedback.rs:614:            // Required `invocation_id: InvocationId` (string token).
src-tauri/src/services/claim_receipt/feedback.rs:617:                .and_then(|m| m.get("invocation_id"))
src-tauri/src/services/claim_receipt/feedback.rs:623:                        "not_relevant_here.invocation_id is required".to_string(),
src-tauri/src/services/claim_receipt/feedback.rs:628:                    "not_relevant_here.invocation_id exceeds 128 char budget".to_string(),
src-tauri/src/services/claim_receipt/feedback.rs:685:        metadata: sanitized_metadata,
src-tauri/src/services/claim_receipt/feedback.rs:700:        FeedbackAction::NotRelevantHere => &["invocation_id"],
src-tauri/src/services/claim_receipt/feedback.rs:1023:        // NotRelevantHere: required invocation_id.
src-tauri/src/services/claim_receipt/feedback.rs:1029:            Some(&serde_json::json!({"invocation_id": "invocation-1"})),
src-tauri/src/services/claim_receipt/feedback.rs:1031:        .expect("not_relevant_here with invocation_id");
src-tauri/src/services/claim_receipt/feedback.rs:1192:                metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1218:            metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1248:            metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1283:                metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1320:                    metadata: Some(serde_json::json!({
src-tauri/src/services/claim_receipt/feedback.rs:1358:                metadata: Some(serde_json::json!({
src-tauri/src/services/claim_receipt/feedback.rs:1424:                    metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1458:                metadata: None,
src-tauri/src/services/claim_receipt/feedback.rs:1487:                metadata: Some(serde_json::json!({
src-tauri/src/services/claim_receipt/feedback.rs:1529:                metadata: Some(serde_json::json!({"source_content_hash": expected})),
src-tauri/src/services/accounts.rs:4214:            metadata: None,
src-tauri/src/services/emails.rs:1641:    let mut data: serde_json::Value = serde_json::from_str(&content)
src-tauri/src/prepare/orchestrate.rs:3007:fn write_directive(path: &Path, data: &Value) -> Result<(), ExecutionError> {

codex
VERDICT: NEEDS-CHANGES

Findings:
- HIGH, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:91`, registry coupling: handler dispatches `"account_overview"`, but actual registered ability name is `"dailyos/account-overview"` at `src-tauri/abilities-runtime/src/abilities/account_overview.rs:84`. `invoke_registry_json` is name-keyed, so this will miss the descriptor.
- HIGH, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:69`, input boundary: plan forwards raw MCP params, but catalog uses `subject` while `AccountOverviewInput` requires `schema_version` + `account_id` at `account_overview.rs:39`. Needs subject→account_id resolution or schema alignment.
- MED, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:57`, async bridge: `transport.rs:190-248` calls sync gateway from an async rmcp handler; `Handle::block_on` inside handler risks nested-runtime panic. Carrying a handle does not solve that.
- MED, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:101`, crate boundary: do not invent `BridgeActor::from_mcp_actor`; existing boundary adapter is `services/mcp_v2/actor_policy.rs:68`.

K-in citations:
- `.docs/decisions/0102-abilities-as-runtime-contract.md`: §7.1 policy schema; §7.6 introspection; amendment §B/§C/§F for `Actor::McpClient`, manifest scopes, `allowed_actors`.
- `.docs/decisions/0108-provenance-rendering-and-privacy.md`: §2 `render_provenance_for`; §6 + 2026-04-20 amendment 64KB cap.
- `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md`: §§2-3 sensitivity tiers + `CLAIM_TYPE_REGISTRY`.
- `.docs/decisions/0130-surface-independent-composition-contract.md`: §§1-2 `AbilityOutput<Composition>` and `ProvenanceRef`; §3.1 schema-bounded fallback.

No substrate reinvention found, but packet needs the fixes above.
tokens used
166,664
VERDICT: NEEDS-CHANGES

Findings:
- HIGH, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:91`, registry coupling: handler dispatches `"account_overview"`, but actual registered ability name is `"dailyos/account-overview"` at `src-tauri/abilities-runtime/src/abilities/account_overview.rs:84`. `invoke_registry_json` is name-keyed, so this will miss the descriptor.
- HIGH, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:69`, input boundary: plan forwards raw MCP params, but catalog uses `subject` while `AccountOverviewInput` requires `schema_version` + `account_id` at `account_overview.rs:39`. Needs subject→account_id resolution or schema alignment.
- MED, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:57`, async bridge: `transport.rs:190-248` calls sync gateway from an async rmcp handler; `Handle::block_on` inside handler risks nested-runtime panic. Carrying a handle does not solve that.
- MED, `.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:101`, crate boundary: do not invent `BridgeActor::from_mcp_actor`; existing boundary adapter is `services/mcp_v2/actor_policy.rs:68`.

K-in citations:
- `.docs/decisions/0102-abilities-as-runtime-contract.md`: §7.1 policy schema; §7.6 introspection; amendment §B/§C/§F for `Actor::McpClient`, manifest scopes, `allowed_actors`.
- `.docs/decisions/0108-provenance-rendering-and-privacy.md`: §2 `render_provenance_for`; §6 + 2026-04-20 amendment 64KB cap.
- `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md`: §§2-3 sensitivity tiers + `CLAIM_TYPE_REGISTRY`.
- `.docs/decisions/0130-surface-independent-composition-contract.md`: §§1-2 `AbilityOutput<Composition>` and `ProvenanceRef`; §3.1 schema-bounded fallback.

No substrate reinvention found, but packet needs the fixes above.
