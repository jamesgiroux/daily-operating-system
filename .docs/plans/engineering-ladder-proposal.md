# Engineering Ladder — Proposal

**Status:** Draft for James review — not yet adopted
**Date:** 2026-05-18
**Author:** Claude (under James direction)
**Supersedes (if adopted):** "Review Ladder L0–L6" naming in `CLAUDE.md`, `.docs/plans/v1.4.0-waves.md`, `.docs/plans/v1.4.0-waves-amendments.md`
**Scope note:** Local workflow only. Cloud routines, claudebot DMs, daily digest, and other unshipped orchestration-v1-lite pieces are explicitly out of scope.

---

## Why this proposal exists

The current `Review Ladder L0–L6` is a **gate** model: tiers of independent review that block merge until passed. It's good at catching drift at the gate, but every L0/L2 finding shaped like "you reinvented X" or "this substrate already exists" is the system saying the **authoring agent didn't know what was already there**. Memories like `check_substrate_before_authoring_primitives`, `two_similar_bugs_class_review`, `ground_first_drafts_in_real_codebase`, and `verify_before_claiming_fidelity` are the symptom catalog.

Gates can only catch drift; they can't prevent it. To prevent it you need a **knowledge layer** that flows forward into authoring and a **plan/implement/review/capture matrix** that makes every phase's skill explicit, not just review.

This proposal does four things:

1. Renames `Review Ladder` → **Engineering Ladder** (keeps L0–L6 numbering — too many downstream references to renumber).
2. Adds a **per-rung skill matrix** covering planning, implementation, review, and knowledge for each rung.
3. Adds a **knowledge channel (K)** — a continuous feedback loop, not a rung. Feeds *into* L0 (substrate-grep obligation) and *out of* L3/L5 (auto-capture class findings via `/ce-compound`).
4. Specifies which skills from **gstack**, **compound-engineering**, and the **existing ladder** are picked for each role.

---

## Why "Engineering Ladder" (and not a renumber)

**Rename, don't renumber.** L0–L6 is load-bearing in:

- `.githooks/commit-msg` (enforces `L2-status: passed|not-run-acknowledged|n-a-doc-only`)
- 15+ entries in `MEMORY.md` referencing L0/L1/L2/L3/L5/L6 by number
- Linear comments, ADRs, wave plans, agent prompts, retro docs
- `CLAUDE.md` § The Review Ladder

Renumbering would cost a multi-day sweep and risk silent breakage of the commit-msg gate. Renaming is one word in ~6 files.

**"Engineering Ladder"** because:

- It encompasses planning (L0 prep + L0 review), implementation (L1), review (L2/L3), verification (L4), drift (L5), and escalation (L6).
- It's neutral — doesn't lean on CE branding ("compound") or DailyOS-specific terms ("substrate").
- It pairs cleanly with the existing **Wave** primitive: waves are the project-management unit, the ladder is the per-unit lifecycle.

Alternatives considered: `Lifecycle Ladder` (acceptable, slightly more clinical), `Build Ladder` (too narrow), `Quality Ladder` (review-flavored), `Compound Ladder` (overloads CE's term).

---

## The Engineering Ladder L0–L6 — skill matrix

Each rung now declares four roles: **Plan**, **Implement**, **Review**, **Capture**. A rung may have N/A in a column when that role doesn't apply at that phase.

| Rung | Phase | Plan skill | Implement skill | Review skill | Capture hook |
|---|---|---|---|---|---|
| **L0 Prep** | Plan hardening | `/plan-eng-review` (always) + `/plan-ceo-review` (product/domain) + `/plan-design-review` (UI/workflow) + `/plan-devex-review` (MCP/API/DX) + `/ce-plan` (structure + open questions surfacing) + `/ce-strategy` when no STRATEGY exists | n/a | n/a | `/ce-sessions` — pull prior-session findings into the plan |
| **L0 Plan** | Plan review (gate) | n/a | n/a | `/codex challenge` + `architect-reviewer` (default) + `security-auditor` (when paths match Amendment 3) + `/codex consult` + `/ce-doc-review` (parallel persona reviewers on plan doc) | **K-in**: each L0 reviewer must grep `docs/solutions/` + `.docs/decisions/` for substrate the plan claims is net new; cite hits in the verdict. Unanimous required as today. |
| **L1 Self** | Implementation | n/a | `/ce-work` (executes against plan) + `/ce-debug` (root cause when stuck) + `/ce-simplify-code` (pre-PR polish) + manual editing | self-validation: tests pass, proof artifacts captured, demo evidence captured | `/ce-sessions` when picking up mid-task |
| **L2 Diff** | PR review (gate) | n/a | n/a | `/review` (gstack pre-landing) + `/codex review` + `code-reviewer` subagent + domain reviewer per matrix + `/ce-code-review` **only if** PR scope is small + scoped (use as a sanity-check second opinion, not a substitute) | **K-in**: domain reviewer cites any matching `docs/solutions/` entries when finding repeats a known class |
| **L3 Wave** | Integrated review (gate) | n/a | n/a | `/codex challenge` against integrated wave + ADRs + `architect-reviewer` on integrated state + Suite S/P/E | **K-out**: at retro close, run `/ce-compound` on every class-pattern finding and every "substrate-already-existed" finding. Headless mode. Output committed to `docs/solutions/<category>/`. |
| **L4 Surface** | User-facing QA | n/a | n/a | `/qa-only` first, `/qa` if remediation needed, `accessibility-tester` for user-facing, `/ce-test-browser` if no `/qa` config | **K-out**: capture surface bugs that recur across waves via `/ce-compound` |
| **L5 Drift** | Architecture drift | n/a | n/a | `/plan-eng-review` + `architect-reviewer` comparing integrated state to planned end-state | **K-out**: run `/ce-compound-refresh` on stale `.docs/decisions/` and `docs/solutions/` entries the drift sweep surfaces |
| **L6 Human** | Escalation | n/a | n/a | James — decision posted as a Linear comment on the affected ticket | **K-out**: L6 decisions captured as Linear comments (existing) + an ADR if the decision is architecturally durable |

### What's new vs current ladder

1. **Plan column** is now explicit — today the ladder text mentions `/plan-eng-review` once; the matrix names every planning skill per scope.
2. **Implement column** is new — the current ladder implies "agent codes" without naming the skills. CE's `/ce-work`, `/ce-debug`, `/ce-simplify-code` slot in here.
3. **Capture column** is new — this is the integration's center of gravity. Every gate that produces findings now has a defined knowledge sink.
4. **K-in obligation at L0** — reviewers must grep `docs/solutions/` and `.docs/decisions/` before approving. This is the mechanism that makes the knowledge layer *load-bearing* instead of decorative.

---

## The Knowledge Channel (K)

`K` is a continuous channel parallel to L0–L6, not a rung.

```
                     Knowledge channel (K)
        ┌──────────────────────────────────────────┐
        │  docs/solutions/    .docs/decisions/     │
        │  (CE writes)        (ADRs, you write)    │
        └──────────────────────────────────────────┘
              ▲ (K-out)              │ (K-in)
              │                       ▼
   L3 retro ──┘              ┌─── L0 reviewer must grep
   L5 drift ──┘              │    before approving
   L4 surface bugs ──┘       └─── L1 author may grep
                                  before drafting
```

### K-out (capture)

After each L3 wave retro, `/ce-compound` runs for each:

- **Class-pattern finding** (the "same shape twice = sweep" memory)
- **Substrate-already-existed finding** (the "check substrate first" memory)
- **Cross-wave drift finding** surfaced by L5

Each run writes one doc to `docs/solutions/<category>/<slug>-<date>.md` with YAML frontmatter (`module`, `tags`, `problem_type`, `track`).

**Three layers ensure K-out actually happens, defense-in-depth:**

1. **Autonomous (default).** Claude runs `/ce-compound mode:headless` for each qualifying finding as part of the L3 retro work — same set-and-forget authorization as `feedback_set_and_forget_wave_protocol` covers for impl→L1→commit→L2→fix→retro→tag. The retro is already the parked context at wave-end; K-out is just the closing step in the loop. James does not need to remember to ask.
2. **Reminder hook (safety net).** A Stop hook at `.claude/hooks/k-out-reminder.sh` scans the last assistant turn for trigger phrases (`class-pattern finding`, `same-shape twice`, `substrate already existed`, `reinvented`, `L3 retro complete`) and prints a one-line nudge when matched. Catches findings that surface mid-conversation outside a retro and the autonomous run missed them.
3. **Retro template gate (blocking).** The wave `retro.md` template gains a checklist item — `- [ ] K-out runs complete (paths of /ce-compound docs created)` — that must be filled in before the retro closes. Belt-and-braces: if the autonomous run forgot a finding, the gate catches it before retro-close.

In practice: tier 1 handles 90%+ silently, tier 2 is the prompt-level nudge when something slips through, tier 3 is the audit trail that proves K-out actually ran.

### K-in (consume)

L0 reviewer prompts (codex-challenge, architect-reviewer, security-auditor) get one line added:

> Before scoring this plan, grep `docs/solutions/` and `.docs/decisions/` for the substrate this plan claims to be net new. Cite any hits in your verdict. If the plan reinvents documented substrate, score it BLOCKED with the path of the existing doc.

That's the entire integration. The knowledge layer doesn't help unless reviewers are obligated to consult it.

### Discoverability

The `CLAUDE.md` § Critical Rules block gets one line added so all agents (not just L0 reviewers) discover the knowledge store:

> `docs/solutions/` — documented solutions to past problems (bugs, class-pattern findings, workflow learnings), organized by category with YAML frontmatter (`module`, `tags`, `problem_type`). Relevant when implementing or debugging in documented areas. Companion to `.docs/decisions/` (ADRs).

This is the CE Discoverability Check, calibrated to the existing CLAUDE.md tone.

---

## What we're picking from each system

### From gstack (keep)

- **L0 Plan**: `/plan-eng-review`, `/plan-ceo-review`, `/plan-design-review`, `/plan-devex-review`
- **L2 Diff**: `/review`, `/codex review`, `/codex challenge`, `/codex consult`
- **L4 Surface**: `/qa`, `/qa-only`, `/browse`, `/design-review`
- **Cross-cutting**: `/cso` (security audits), `/ship` (PR creation), `/retro` (weekly retros)
- **Why keep**: stricter than CE equivalents (unanimous, AC-bounded, wave-scoped), and memories are already tuned to them.

### From compound-engineering (adopt)

- **`/ce-compound`** — the centerpiece. Knowledge capture into `docs/solutions/`. Triggered automatically after L3 retros (headless) and manually by James (interactive).
- **`/ce-compound-refresh`** — drift sweep on stale `docs/solutions/` entries, fired by L5.
- **`/ce-sessions`** — cross-session search across Claude Code / Codex / Cursor. Used at L0 prep (pull prior context into plan) and L1 self (pick up mid-task).
- **`/ce-doc-review`** — additional persona reviewers on plan docs (parallel to existing L0 trio).
- **`/ce-resolve-pr-feedback`** — automate the "resolve and reply" cycle on PR review threads after L2.
- **`/ce-debug`** — root-cause-first debugging during L1.
- **`/ce-simplify-code`** — pre-PR polish during L1.
- **Why adopt**: these fill genuine gaps (knowledge layer, session memory, PR-feedback automation, debugging discipline).

### From compound-engineering (skip)

- **`/lfg`** — collapses too many gates. Replaces L0/L1/L2/L3 with one pipeline. Conflicts with wave semantics and unanimous-required L0/L2. Don't use.
- **`/ce-code-review`** — overlaps with the stronger gstack `/review` + `/codex review` + domain reviewer combo. Allowed as a sanity-check second opinion but not a substitute.
- **`/ce-plan`** — partial overlap with `/plan-eng-review`. Use only when no existing plan-review skill matches the scope.
- **`/ce-strategy`** — DailyOS already has `.docs/design/product/{MISSION,VISION,PRODUCT-THESIS}.md`. Skip unless starting a net-new product.
- **`/ce-commit`** — the project's `commit-msg` hook enforces `L2-status:`, `Co-authored-by:`, and the PII gate. `/ce-commit` doesn't know about those, so it generates non-conforming messages. Skip.
- **`/ce-dhh-rails-style`** — wrong stack (Rust + React + WP).
- **`/ce-frontend-design`** — overlaps with `/design-shotgun`, `/design-review`, `/design-html`. Skip.

### From existing ladder (keep entirely)

- L0–L6 numbering and pass rules (unanimous, AC-bounded, cycle cap = 2, codex outage retry policy, security-auditor on L0 when paths match).
- The Plan-review template at `.docs/plans/v1.4.0-waves.md` lines 102–144.
- The reviewer matrix and Suite S/P/E.
- The L6 escalation MUST / MUST NOT lists.
- Wave-scoped artifacts (proof bundle, retro, ADRs) in git.

---

## Concrete diffs to apply (if approved)

### 1. CLAUDE.md addition (~5 lines)

Add to the **Critical Rules** section, after the Intelligence Loop block:

```markdown
**Knowledge store discovery.** Before authoring substrate-touching work, grep `docs/solutions/` and `.docs/decisions/` for prior solutions and architectural decisions. `docs/solutions/` holds documented problem→fix entries with YAML frontmatter (`module`, `tags`, `problem_type`); `.docs/decisions/` holds ADRs. Reinventing documented substrate is a BLOCKED finding at L0.
```

And rename the section header `## The Review Ladder (L0–L6)` → `## The Engineering Ladder (L0–L6)`, with the opening sentence updated to:

> Used in chat: "L2 looks good." Used in docs/headings: "L2 (Diff) review verdict: approve." Each rung defines Plan / Implement / Review / Capture roles; see `.docs/plans/engineering-ladder.md` for the skill matrix.

(The full skill matrix lives in a dedicated doc, not in CLAUDE.md, to keep CLAUDE.md scannable.)

### 2. New file `.docs/plans/engineering-ladder.md`

Once this proposal is approved, the matrix section above becomes the canonical reference at `.docs/plans/engineering-ladder.md`. CLAUDE.md and the waves docs link to it.

### 3. Update `.docs/plans/v1.4.0-waves.md`

- Section header `# Review system — the integrity layer` → `# Engineering Ladder — the integrity layer`.
- Subsection `## Review ladder L0–L6` → `## Engineering Ladder L0–L6`.
- Add a paragraph after the existing table:
  > Each rung also names a Plan, Implement, and Capture skill — see `.docs/plans/engineering-ladder.md` for the full matrix. K-in (L0 substrate grep) and K-out (L3/L5 `/ce-compound` capture) are mandatory.

### 4. Update the canonical L0 reviewer prompt

The L0 panel runs locally (`/codex challenge`, `architect-reviewer`, `/codex consult`, `security-auditor` when paths match Amendment 3). Add one line to each reviewer's invocation prompt — codified in `.docs/plans/engineering-ladder.md` so future invocations pick it up:

> Before scoring, grep `docs/solutions/` and `.docs/decisions/` for the substrate this plan claims to be net new. Cite hits in your verdict. Reinvented documented substrate = BLOCKED, cite the path.

### 5. Add an L3 retro closeout step

The wave-retro template (currently in each `proof-bundle.md` / `retro.md` checklist) gains one final step:

> Run `/ce-compound` for each class-pattern finding and each substrate-already-existed finding from this wave's L2/L3 cycles. Commits one doc per finding to `docs/solutions/<category>/`.

### 6. New repo directory: `docs/solutions/`

Created empty with a README explaining the structure (categories, frontmatter schema, where to read from / write to). Subdirectories auto-created by `/ce-compound`.

### 7. `feedback_*` memory entries (optional)

Add one new memory entry:

```
- [Engineering Ladder L0–L6 — Plan/Implement/Review/Capture per rung](project_engineering_ladder.md) — ladder renamed from Review Ladder; K-in at L0 (substrate grep), K-out at L3/L5 (/ce-compound capture); skill matrix at .docs/plans/engineering-ladder.md
```

---

## Migration plan (if approved)

| Step | Action | Owner | Effort |
|---|---|---|---|
| 1 | Create `docs/solutions/README.md` + empty category dirs | James or Claude | 10 min |
| 2 | Write `.docs/plans/engineering-ladder.md` (the canonical matrix) | Claude | 30 min |
| 3 | Apply CLAUDE.md rename + Critical Rules addition | Claude | 5 min |
| 4 | Apply waves.md header rename + matrix pointer paragraph | Claude | 5 min |
| 5 | Codify L0 substrate-grep prompt line + L3 retro closeout step in `.docs/plans/engineering-ladder.md` | Claude | 10 min |
| 6 | Add `project_engineering_ladder` memory entry + update `feedback_set_and_forget_wave_protocol` to include K-out in the loop | Claude | 10 min |
| 7 | Write `.claude/hooks/k-out-reminder.sh` (Stop hook, trigger-phrase scan, one-line nudge) + register in `.claude/settings.json` | Claude | 20 min |
| 8 | Add K-out checklist item to wave `retro.md` template | Claude | 5 min |
| 9 | Backfill `docs/solutions/` from the last 3 retros (W3, W4, W5) — manual `/ce-compound` runs | James + Claude | 1–2 hours |
| 10 | Verify the substrate-grep obligation lands in the next L0 reviewer invocation | Claude on next L0 run | n/a |

Total wiring: ~1 hour. Backfill: 1–2 hours. After that, K-out runs automatically at every L3 retro and the layer compounds.

---

## The tradeoff (restated)

The integration only pays off if **K-out runs every retro** and **K-in is enforced at L0**. Without those two loops closed, `docs/solutions/` becomes a doc graveyard within two waves and you've added a fifth doc surface (ADRs, MEMORY.md, Linear, `.docs/plans/`, now `docs/solutions/`) for no benefit.

The commit is: **L3 retros MUST end with `/ce-compound` runs on findings, and L0 reviewer prompts MUST enforce the substrate-grep obligation.** Skip either and this is theater. Both happen locally — no cloud routines, no Slack DMs, no daily digest. Just James running the skills as part of the retro checklist and L0 invocation.

---

## Hooks + CI audit (approval precondition)

Per James's approval condition: audit current pre-commit / commit-msg / pre-push / GitHub workflows for redundancy and identify any changes the engineering-ladder integration requires. PII gate stays mandatory; nothing is removed without explicit sign-off.

### Current state — what's already well-engineered

**Local hooks** (`core.hooksPath=.githooks`):

- `.githooks/pre-commit` (270 lines): cheap checks always (PII blocklist, stub/TODO scan, schema/mock sync, ADR-0101 boundary, reference fidelity, prompt template gate); heavy checks (clippy + cargo test --lib + tsc) gated by `HAS_RUST` / `HAS_FRONTEND` path classification; `WIP=1` escape hatch defers heavy checks to pre-push; tree-SHA cache marker (`<git-dir>/hooks-state/last-green-tree`) lets pre-push skip the duplicate gauntlet when HEAD's tree is unchanged.
- `.githooks/commit-msg` (104 lines): enforces `L2-status: passed|not-run-acknowledged|n-a-doc-only` on code-pattern commits (`.rs`, `.ts`, `.tsx`, `.sql`, `.toml`, `.sh`, lockfiles, workflows). Doc-only commits exempt. Merge/revert/fixup/squash exempt.
- `.githooks/pre-push` (135 lines): mirrors pre-commit's path classification; uses the tree-SHA cache to skip the heavy gauntlet when pre-commit already passed against the same tree.

**CI workflows** (9 files, each explicitly cost-tuned):

- `rust.yml` — `push: main` only (macOS, ~10x Linux cost, deliberately narrow)
- `lint-frontend.yml` — PR + push to dev/trunk (Linux, fast)
- `security-audit.yml` — weekly + path-trigger on `Cargo.lock` / `pnpm-lock.yaml`
- `l2-review.yml` — PR-open against dev/trunk (existing L2 panel)
- `l3-review.yml` — PR-open against main (release-only)
- `release.yml`, `wp-plugin.yml`, `load-test.yml`, `block-kit-integration.yml` — release / surface-specific

### What the engineering-ladder integration changes

| Component | Impact on hooks/CI | Action needed |
|---|---|---|
| K-in (L0 substrate grep) | None — happens at plan-review time | None |
| K-out (`/ce-compound` after retro) | None at git level — happens during retro session | None |
| K-out reminder hook | Claude Code Stop hook (`.claude/hooks/k-out-reminder.sh`), not a git hook | New file; zero git-perf impact |
| `docs/solutions/` doc commits | Passes through existing PII scan (✓ STAGED_FILES iterates all paths) | Verify no false-positives in stub/TODO scan |
| L2-status on doc-only `docs/solutions/` commits | `.md` not in `is_code_file()` → correctly exempt | None |
| ADR-0101 boundary | `.rs` files only → not triggered | None |
| Schema/mock sync | Schema paths only → not triggered | None |
| Reference fidelity gate | `.docs/design/reference/...` paths only → not triggered | None |

**One real false-positive risk:** the stub/TODO scan (pre-commit step 4) greps every staged file for `TODO|FIXME|HACK|unimplemented!\(\)|todo!\(\)|Phase 2|// stub|// deferred|// placeholder`. `/ce-compound` captures will legitimately contain these terms in code snippets documenting the fix. Solution: exclude `docs/solutions/`, `.docs/decisions/`, and `docs/**` from the scan.

### Recommended changes — trim, fix, don't restructure

**A. Bug fixes (do regardless of ladder integration):**

1. **Stub/TODO scan exclude.** Add a `case "$file"` skip clause at the top of pre-commit step 4's `while` loop for `docs/solutions/*`, `.docs/decisions/*`, `.docs/plans/*`, `docs/**`. These are documentation surfaces and TODO/FIXME mentions are legitimate content.
2. **CLAUDE.md path reference fix.** CLAUDE.md references `.claude/hooks/pre-commit-gate.sh` (line: "The pre-commit gate at `.claude/hooks/pre-commit-gate.sh` enforces..."). That path doesn't exist — the active hook is `.githooks/pre-commit`. Update CLAUDE.md to point at the right path.
3. **Dead symlinks in `.git/hooks/`.** `.git/hooks/commit-msg` and `.git/hooks/pre-push` symlink to `.claude/git-hooks/{commit-msg,pre-push}` — but `.claude/git-hooks/` is empty. The symlinks are dead and override-shadowed by `core.hooksPath=.githooks`. Either delete the symlinks (clean) or point them at `.githooks/`.

**B. Efficiency wins (low risk, modest savings):**

4. **Batch the PII + stub/TODO scans.** Both currently loop `while IFS= read -r file` and invoke `git diff --cached -U0 -- "$file"` per file. On a diff with 50+ files this is 50+ git invocations. Replace with a single `git diff --cached -U0` invocation parsed per-file. Estimated save: 2–5s on large diffs. Risk: low — same content scanned, different invocation pattern.
5. **`lint-staged --concurrent false` → `--concurrent true`.** lint-staged is single-threaded today (line 100). Concurrent mode is safe for eslint/stylelint on non-overlapping files. Estimated save: 30–50% on multi-file frontend diffs.
6. **Skip prompt template / fingerprint boundary checks on doc-only commits.** Pre-commit lines 218–219 run unconditionally after the error block. Add a `HAS_PROMPT_RELEVANT_CHANGE` guard so doc-only commits skip the two `scripts/check_prompt_*.sh` invocations. Estimated save: 0.5–1s per doc-only commit.

**C. No changes recommended:**

- **PII blocklist.** James was explicit: keep mandatory. Memory `feedback_salesforce_blocklist_is_tool_not_customer` notes Salesforce hits are usually the SaaS tool name — handled per-commit by `--no-verify` with surfacing, not by removing the entry.
- **Tree-SHA cache mechanism.** Already sophisticated and working. Don't disturb.
- **WIP=1 escape hatch.** Keep.
- **All 9 GitHub workflows.** Already cost-tuned with explicit rationale comments. Touching them risks the careful PR-vs-push-vs-main tier separation.
- **Heavy check ordering (clippy → test → tsc).** Cargo test is the slow step (30–120s) but it's the main correctness net before push. Deferring to pre-push only would mean broken tests reach the local push gate. Status quo is right.

### Net effect on commit/push perf

| Diff size | Current cost | Post-optimization | Savings |
|---|---|---|---|
| Doc-only (`.md` only, no code) | ~1–2s (PII + stub scan + prompt checks unconditionally) | ~0.3s (only PII + stub scan, no prompt checks) | ~1s |
| Frontend-only (5 files) | ~10–15s (tsc + lint-staged sequential) | ~6–8s (lint-staged concurrent) | ~30% |
| Full-stack (20+ files) | ~60–90s (clippy + test + tsc + scans iterating per-file) | ~55–85s (scans batched) | ~5s |
| `WIP=1` commit | ~1s | ~1s | None |

Modest but real. Bigger wins (parallel hook runners, async test) would require restructuring and aren't worth it given current pain levels.

### Migration plan addendum

Adding three steps to the migration plan above:

| Step | Action | Owner | Effort |
|---|---|---|---|
| 11 | Apply pre-commit fixes: stub/TODO scan exclude (docs paths), batch PII+stub scans, lint-staged concurrency, prompt-check guard | Claude | 30 min |
| 12 | Fix CLAUDE.md path reference to `.githooks/pre-commit` | Claude | 2 min |
| 13 | Remove or repoint dead symlinks in `.git/hooks/` | James (or Claude with explicit approval — touches `.git/`) | 5 min |

---

## Open questions for James

1. **Rename target.** `Engineering Ladder` (recommended) vs `Lifecycle Ladder` vs leave as `Review Ladder` and just add the matrix?
2. **Skill matrix location.** Inline in CLAUDE.md (more visible, longer file) vs dedicated `.docs/plans/engineering-ladder.md` (recommended, keeps CLAUDE.md scannable)?
3. **K-out cadence.** Every L3 retro (recommended) vs only when retro flags class-pattern finding (lighter) vs continuous after every L2 fix (heavier)?
4. **`docs/solutions/` location.** Top-level `docs/solutions/` (CE default, recommended for grep simplicity) vs `.docs/solutions/` (matches existing `.docs/` convention)?
5. **Backfill scope.** Last 3 waves (W3/W4/W5) vs every retro from v1.4.0 (heavier) vs no backfill, start fresh at the next L3 (lightest)?
6. **`/ce-doc-review` slot at L0.** Add as a fourth required reviewer (quartet → quintet) or run as optional fifth-opinion?
7. **K-out automation tiers.** Ship all three (autonomous run + reminder hook + retro-template gate, recommended) vs autonomous + retro gate only (skip the hook, trust the loop + audit) vs autonomous only (no safety net)?
8. **Hook optimizations scope.** Ship A+B+C as proposed (bug fixes + efficiency wins, recommended) vs A only (just the false-positive + dead-path fixes, no perf changes) vs A+C (no batching/concurrency changes — leave hook logic untouched)?
9. **Dead `.git/hooks/` symlinks.** Delete cleanly (recommended) vs repoint at `.githooks/` (belt-and-braces in case `core.hooksPath` ever drops) vs leave alone?
