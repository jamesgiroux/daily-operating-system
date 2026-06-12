# AGENTS.md

DailyOS is a native macOS app (Tauri + React) that makes intelligence personal: memory plus judgment for the alone part of knowledge work. Open the app, your day is ready — AI produces, users consume, with trust as a product feature rather than a disclaimer. The system maintains a working understanding of the user's professional world over time, distinguishes fact from inference, remembers corrections, and surfaces context before the user asks. Customer Success is the first slice (James is customer-zero), but personal intelligence is general-purpose — not a CS-specific tool, not enterprise search, not a chatbot, not a note-taking app. Canonical framing lives in `.docs/design/product/` (MISSION, VISION, PRODUCT-THESIS, PHILOSOPHY, PRINCIPLES); read those, not just this header, before writing user-facing prose about what DailyOS is.

## Critical Rules

**Intelligence Loop integration check.** This is mandatory. Every new table, schema column, claim field, or user-visible intelligence surface must answer these 5 questions before shipping:
1. **Claim model:** Should this be represented as a claim with explicit subject attribution, temporal scope, sensitivity, and lifecycle state (instead of ad-hoc display-only data)?
2. **Provenance + trust:** How will provenance be captured (`source_asof`, source attribution, field attribution), and how does this data affect trust scoring or trust-band rendering (`likely_current`, `use_with_caution`, `needs_verification`)?
3. **Signals + invalidation:** What signals should this emit, and which propagation/invalidation paths should refresh derived state when it changes?
4. **Runtime + surfaces:** Which abilities and contexts should consume it (`build_intelligence_context()`, `gather_account_context()`, prep outputs, callouts), and what is the expected behavior across Tauri and MCP surfaces?
5. **Feedback loop:** How do user corrections, dismissals, corroborations, or contradictions feed back into claim state and source reliability/trust inputs?

Data that only renders in the frontend, without claim/provenance semantics, signal propagation, trust impact, runtime consumption, or feedback learning, is incomplete.

**All mutations go through `services/`.** No direct DB writes from command handlers. No exceptions.

**Reuse before build — DRY, SRP, separation of concerns. Non-negotiable.** Before authoring ANY new component, style, hook, or helper: search for an existing one and extend it. The canon is `src/components/` (especially `ui/`), design tokens in `src/styles/design-tokens.css`, and `.docs/design/reference/` (copy-paste-ready HTML for every token/primitive/pattern/surface, mirrored from production and gate-enforced). A second same-purpose component, a hardcoded value that shadows a token, or a re-implementation of an existing pattern is a BLOCKED finding at every review rung — extend or consume the existing unit, never fork it. **Surfaces never handle logic:** `src/pages/` and display components render props and dispatch user intent only — no business logic, no direct `invoke()` calls (those belong in hooks/services). Every plan and PR answers: *which existing components/styles does this consume, and what genuinely new unit does it add — and why couldn't an existing one be extended?*

**No customer-specific data in source code.** Never hardcode real customer domains, company names, email addresses, or account details in code, comments, or test fixtures. Use generic examples (`subsidiary.com`, `parent.com`, `user@example.com`). Customer data belongs in the encrypted DB, not in version-controlled files.

**No PII in commit messages, titles, or PR bodies — including references to PII cleanup work.** Commit subjects and bodies stay PII-free, AND must not describe what was scrubbed (e.g. avoid "scrub residual customer references", "rename PII-loaded identifiers"). Listing the very terms you're removing leaks them through the message itself. Frame the commit by what it accomplishes for the system (e.g. "tighten test fixture lint", "rename internal identifiers"), not by the inputs the discipline removes. The pre-commit gate at `.githooks/pre-commit` enforces the file-content blocklist from `.claude/pii-blocklist.txt`; commit messages are on you.

**Knowledge store discovery.** Before authoring substrate-touching work, grep `docs/solutions/` and `.docs/decisions/` for prior solutions and architectural decisions. `docs/solutions/` holds documented problem→fix entries with YAML frontmatter (`module`, `tags`, `problem_type`); `.docs/decisions/` holds ADRs. Reinventing documented substrate is a BLOCKED finding at L0. See `docs/solutions/README.md` and `.docs/plans/engineering-ladder.md`.

**Definition of Done.** "Compiles clean" is NOT "works." A task is done when:
1. Acceptance criteria from the issue spec or version brief are validated with real data
2. End-to-end flow works — real data through the Intelligence Loop to rendered surfaces
3. No stubs, no TODOs, no "Phase 2" deferrals
4. Tests pass: `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`

**Aim for completion, not the simplest thing that compiles.** Do not default to "what's the smallest change that lets clippy pass and tests turn green." Aim for completion of the work as written in the acceptance criteria and the brief's intent. Clippy clean, tsc clean, and tests passing are necessary signals; they are not the bar. If the brief names an existing v1.4.x capability (W3 abilities, claim substrate, lifecycle, trust bands, ability runtime) as the producer for new work, the wiring IS the work — shipping an empty-branch composer with a "producer is future" comment is missing the requirement, not deferring it. Before declaring any composer/surface/section "ships empty by design," grep the codebase for the named producer and read the relevant wave plan; if the producer exists, wire it.

## Commands

```bash
pnpm dev                            # Start Tauri dev (frontend + backend)
pnpm build                          # Production build
cargo test                          # Rust backend tests
pnpm test                           # Frontend tests
cargo clippy -- -D warnings         # Lint — must pass before any commit
pnpm tsc --noEmit                   # Frontend type check
```

## Workflow

**Branch from `dev`**, not `trunk`. Submit PRs to `dev`. `trunk` is stable releases only (tagged versions).

**Planning source of truth:** Linear is canonical for issues, backlog, and project execution. Git keeps durable decisions, architecture/design guidance, and occasional version or wave planning docs under `.docs/plans/`.

Legacy markdown issue specs and backlog material live under `.docs/_archive/` for historical reference only.

**Plan first.** For any non-trivial task (3+ steps or architectural decisions), use Codex's planning surface or write a short explicit plan before implementation. If something goes sideways, stop and re-plan — don't keep pushing.

## The Engineering Ladder (L0–L6)

Every plan or change passes through some subset of these review levels. The L-prefix is shorthand; the descriptor in parentheses names what each level actually does. Used in chat: "L2 looks good." Used in docs/headings: "L2 (Diff) review verdict: approve." Each rung names **Plan**, **Implement**, **Review**, and **Capture** skills; `.docs/plans/engineering-ladder.md` is the canonical skill matrix.

- **L0 Prep (Plan Hardening)** — optional for small issues, expected for foundational/blocking issues. Use `/plan-eng-review` to turn the Linear issue into a reviewable plan packet: architecture, data flow, edge cases, test plan, rollout risks, and Intelligence Loop fit. Add `/plan-ceo-review`, `/plan-devex-review`, or `/plan-design-review` when product/domain, MCP/API/developer experience, or UI/workflow scope is central. This hardens the Linear ticket but is not an approval gate.
- **L0 (Plan)** — pre-code review of the hardened plan doc or ticket plan on the Linear ticket. **Default 2 reviewers**: `/codex challenge` + ONE planning reviewer routed by what the plan touches (`ce-coherence-reviewer` for terminology drift, `ce-scope-guardian-reviewer` for scope creep, `ce-feasibility-reviewer` for architectural feasibility, `ce-product-lens-reviewer` for strategy, `ce-design-lens-reviewer` for UI/workflow, `ce-security-lens-reviewer` for plan-level security gaps). Add a second planning reviewer at Wave scope tier or when Amendment 3 paths are touched (trust-boundary, privacy, source, MCP, filesystem, claim/provenance, or write-path). **K-in mandatory:** `ce-learnings-researcher` runs in parallel with the planning reviewer and cites `docs/solutions/` + `.docs/decisions/` hits in verdict. Reinvented substrate = BLOCKED. Unanimous required.
- **L1 (Implementation)** — the implementing agent builds against the plan and self-validates: tests pass, proof artifacts produced, demo evidence captured, before opening the PR.
- **L2 (Diff)** — pre-merge review of the PR. **Default 1 reviewer**: `/codex review` orchestrated via the `l2-bounded-reviewer` agent (AC-scoped, path-α routing). Router adds 0–2 specialists based on what the diff touches: `pr-review-toolkit:type-design-analyzer` (new types/structs), `pr-review-toolkit:silent-failure-hunter` (error handling/fallbacks), `ce-performance-reviewer` (hot paths/queries), `ce-data-migrations-reviewer` (schema changes), `ce-api-contract-reviewer` (API routes), `ce-security-reviewer` (auth/permissions/user input), `ce-reliability-reviewer` (retries/timeouts/background jobs). **Advisory parallel (always-on, non-blocking)**: `ce-maintainability-reviewer`, `ce-pattern-recognition-specialist`, `ce-code-simplicity-reviewer`, `ce-project-standards-reviewer`, `ce-testing-reviewer` — findings file to the Codebase Maintenance project, never block merge. **L2 runs locally, before pushing the PR**, not in CI. The `.githooks/commit-msg` hook enforces L2 acknowledgment by requiring every code-touching commit to carry an `L2-status: passed | not-run-acknowledged | n-a-doc-only` line in its message. CI's L2 footprint is just the trust-boundary fence + PR-template validation.
- **L3 (Wave)** — after all wave PRs merge, integrated adversarial review: `/codex challenge` against integrated diff + ADRs, `ce-architecture-strategist` on integrated state, plus `ce-schema-drift-detector` (waves with migrations) and `ce-deployment-verification-agent` (production-touching). **Suite S/P/E fires by need**, not always: S when wave AC names security or Amendment 3 paths touched (owners: `ce-security-reviewer` + `ce-security-sentinel`), P when wave AC names perf or touches migrations/hot paths/projections (owner: `ce-performance-oracle`), E always (owner: `ce-testing-reviewer`). **For non-major versions (v1.x.y), L5 drift folds into L3** — `ce-architecture-strategist` does both passes. **K-out mandatory:** the retro closes with autonomous `/ce-compound mode:headless` runs on class-pattern and substrate-already-existed findings.
- **L4 (Surface)** — surface QA on user-facing changes when behavior or UI changes: `/qa-only` first, `/qa` if remediation needed, accessibility-tester for user-facing.
- **L5 (Drift)** — **major versions only** (v1.x.0). Compares integrated state to planned end-state via `/plan-eng-review` + `ce-architecture-strategist` at checkpoints named in the version wave/milestone plan, and before release. For v1.x.y (minor/patch), L5 folds into L3 — separate rung does not fire. **K-out:** run `/ce-compound-refresh` on stale `.docs/decisions/` and `docs/solutions/` entries the drift sweep surfaces.
- **L6 (Human)** — James's call when the system can't resolve. Decision is posted as a Linear comment on the affected ticket.

**Reviewer-composition principle (2026-05-23 revision):** default counts are deliberately small. The library of installed agents — `ce-*` and `pr-review-toolkit:*` — is large; the ladder **composes** specific agents per concern rather than running every reviewer on every change. Two design rules: (1) one default reviewer + router-selected specialists, and (2) advisory ≠ blocking — code-quality concerns (maintainability, simplicity, duplication) run in parallel as advisory and file findings to the maintenance project, not back to the cycle-N+1 reviewer loop.

**Preferred review runners:**
- L0 Prep: `/plan-eng-review` by default; add `/plan-ceo-review`, `/plan-devex-review`, or `/plan-design-review` when the issue needs product/domain, MCP/API/developer experience, or UI/workflow hardening.
- L0 Codex challenge: `/codex challenge`.
- L0 planning reviewer (router): pick one from `ce-coherence-reviewer` / `ce-scope-guardian-reviewer` / `ce-feasibility-reviewer` / `ce-product-lens-reviewer` / `ce-design-lens-reviewer` / `ce-security-lens-reviewer` based on what the plan touches. At Wave tier, add a second.
- L0 K-in (always parallel, not a panel slot): `ce-learnings-researcher`.
- L0 Amendment 3 add-on: when trust-boundary, privacy, source, MCP, filesystem, claim/provenance, or write-path behavior is touched, add `ce-security-lens-reviewer` (plan-level) and at L2 add `ce-security-reviewer` (diff-level); use `/cso` for the dedicated DailyOS security review skill where it adds project-specific context the ce-* agents lack.
- L2 default: `/codex review` orchestrated by the `l2-bounded-reviewer` agent. Plus router-selected specialists from the table above. Plus the always-on advisory parallel set.
- L3 architecture: `ce-architecture-strategist`. Plus `ce-schema-drift-detector` for waves with migrations, `ce-deployment-verification-agent` for production-touching waves.

**Knowledge Channel (K)** is a continuous feedback loop parallel to L0–L6: `docs/solutions/` + `.docs/decisions/` feed into L0 through `ce-learnings-researcher` (parallel on every L0), and L3/L5 feed findings back out through `/ce-compound` and `/ce-compound-refresh`. **K-out is now Tier 1 autonomous only** (2026-05-23 revision): Claude runs `/ce-compound mode:headless` for each class-pattern + substrate-already-existed finding at retro close. The previous Tier 2 Stop hook (`.claude/hooks/k-out-reminder.sh`) and Tier 3 retro-template gate were removed as duplicate layers on already-defended work. The retro template still names a K-out captures section — populated by the autonomous run — but no longer blocks retro-close. Outputs land under `docs/solutions/<category>/`.

**Wave is the project-management primitive** for big version programs (e.g., v1.4.0): parallel agents fan out within a wave, hard ordering across waves. Linear milestones may mirror waves, but the wave is the execution protocol and the milestone is the Linear container. Smaller versions don't need waves — they collapse to L0 → code → L2 → ship, with L4 added when the change is user-facing, L5 added when scope drift risk is material, and L6 if it escalates. L3 usually doesn't apply to non-wave work.

**Authority surfaces:**
- **Linear** is the canonical record. Plans live on the ticket. All reviewer verdicts, L6 decisions, mirror-gate holds, routine failures, and approvals are ticket comments.
- **Git** keeps wave-scoped artifacts (proof bundles, retros, ADRs) and the protocol docs themselves.

**Full protocol** at `.docs/plans/v1.4.0-waves.md` (with lean amendments at `.docs/plans/v1.4.0-waves-amendments.md`). `.docs/plans/orchestration/v1-lite.md` is design archive for cloud routines, claudebot, and daily digest; none of that has shipped. The current operating model is local invocation per `.docs/plans/engineering-ladder.md`.

**Parallel-wave migration slot reservations.** When launching parallel wave agents in the same version that all add migrations, claim non-overlapping slot blocks upfront in `.docs/plans/{version}-waves.md` (e.g., `W3 = v155–v159`, `W4-A = v160–v169`, `W4-B = v170–v179`). v1.4.1 W3-C/W4-A/W4-B independently claimed v155, which cost ~6 hours of CI rebase-renumber + fixture-carryover loops after W3-C merged first. Block reservations belong in the wave plan, not in each consumer plan.

## Gotchas

- Tauri Rust backend does NOT hot-reload — restart the app for backend changes
- Bare `use libc;` triggers clippy lint — the crate is accessible without the use statement
- `too_many_arguments` clippy limit is 7 — refactor before it blocks CI
- Partial commits cause CI-only failures — `git status` must show no uncommitted files referenced by committed code before tagging
- Three version files must stay in sync: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json`
- `Tauri externalBin`: `build-mcp.sh` creates empty stub BEFORE `cargo build`, overwrites after

## Code Style

- **Rust**: Standard rustfmt, descriptive error messages, doc comments on public APIs
- **TypeScript/React**: Functional components, custom hooks, TypeScript strict mode
- **CSS**: Design tokens via custom properties, editorial naming conventions
- **Commits**: Include `Co-authored-by: Codex <noreply@openai.com>` when Codex authored the change.

## Behavioral Rules

- Fix bugs autonomously — just fix them, don't ask for hand-holding
- Use Codex subagents for research and parallel analysis when the task explicitly calls for multi-agent work or the active Codex environment supports it safely.
- After any user correction, update `tasks/lessons.md` with the pattern
- Use product vocabulary (ADR-0083) for all user-facing strings — see `.docs/decisions/0083-product-vocabulary.md`
- **Don't swing past center when correcting course.** When the user pushes back on over-engineering, *trim* — don't strip. Preserve the differentiating substrate; remove only the scaffolding. "No consumers today" is a trap because standards precede consumers.
- **Same-shape findings twice = class-wide sweep, not a third patch.** When two L2/review cycles surface the same shape of issue, stop patching the named offenders. Audit the entire class across the codebase, fix in one pass, add a CI gate so re-drift is impossible. The sweep IS the work, not scope expansion.
- **Path-α L2 findings go to maintenance, not cycle-N+1.** L2 findings that aren't a literal acceptance-criterion violation, ADR-named contract violation, or PR-introduced regression → file in the **Codebase Maintenance & Production Quality** Linear project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) and unblock the substrate PR.

### Engineering discipline

This block is the **default posture** for one-shot edits, bug fixes, and small features. The Critical Rules and earlier Behavioral Rules bullets are situational overrides — when they apply, they take precedence:

- *"Fix bugs autonomously"* overrides Rule 1's *"Ask if unclear"* when the bug + fix are unambiguous.
- *"Aim for completion, not the simplest thing that compiles"* + memory `feedback_pick_more_complete_option_over_simpler` override Rule 2's *"Minimum code"* when the work is substrate / wave / acceptance-criteria territory.
- *"Same-shape findings twice = class-wide sweep"* overrides Rule 3's *"Touch only what you must"* when a class-pattern is in play.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

#### Rule 1 — Think before coding

State assumptions explicitly. If uncertain, ask rather than guess.
Present multiple interpretations when ambiguity exists.
Push back when a simpler approach exists.
Stop when confused. Name what's unclear.

#### Rule 2 — Simplicity first

Minimum code that solves the problem. Nothing speculative.
No features beyond what was asked. No abstractions for single-use code.
Test: would a senior engineer say this is overcomplicated? If yes, simplify.

#### Rule 3 — Surgical changes

Touch only what you must. Clean up only your own mess.
Don't "improve" adjacent code, comments, or formatting.
Don't refactor what isn't broken. Match existing style.

#### Rule 4 — Goal-driven execution

Define success criteria. Loop until verified.
Don't follow steps. Define success and iterate.
Strong success criteria let you loop independently.

#### Rule 5 — Use the model only for judgment calls

Use me for: classification, drafting, summarization, extraction.
Do NOT use me for: routing, retries, deterministic transforms.
If code can answer, code answers.

#### Rule 6 — Surface conflicts, don't average them

If two patterns contradict, pick one (more recent / more tested).
Explain why. Flag the other for cleanup.
Don't blend conflicting patterns.

#### Rule 7 — Read before you write

Before adding code, read exports, immediate callers, shared utilities.
"Looks orthogonal" is dangerous. If unsure why code is structured a way, ask.

#### Rule 8 — Tests verify intent, not just behavior

Tests must encode WHY behavior matters, not just WHAT it does.
A test that can't fail when business logic changes is wrong.

#### Rule 9 — Checkpoint after every significant step

Summarize what was done, what's verified, what's left.
Don't continue from a state you can't describe back.
If you lose track, stop and restate.

#### Rule 10 — Match the codebase's conventions, even if you disagree

Conformance > taste inside the codebase.
If you genuinely think a convention is harmful, surface it. Don't fork silently.

#### Rule 11 — Fail loud

"Completed" is wrong if anything was skipped silently.
"Tests pass" is wrong if any were skipped.
Default to surfacing uncertainty, not hiding it.

## Doc Authoring — Tier Ruleset

Planning docs sit in three tiers, picked by criteria rather than lists:

- **Tier 1 (HTML-first)** — author directly as `.html` against the shared design system at `.docs/design/reference/_shared/`. Use when layout carries information, the doc is for visual sharing, or it is a read-heavy reference. Examples: `.docs/plans/engineering-ladder.html`, wave dashboards, PRDs, strategy decks, version roadmaps.
- **Tier 2 (markdown + render)** — author `.md`, render to `.html` via a build script, commit both. Use when content is text-dominant prose with section structure. Examples: `.docs/design/product/{MISSION,VISION,PHILOSOPHY,PRINCIPLES}.md` rendered via `build-foundation-html.mjs`.
- **Tier 3 (markdown-only)** — author `.md`, no render. Use when code-adjacent, diff-reviewed, Linear-canonical, or grep-first. Examples: retros, proof bundles, `docs/solutions/`, ADRs, per-ticket Linear plans, MEMORY.md, commit messages.

Full ruleset, scaffolds, and dogfooding loop convention live at `.docs/plans/AUTHORING.md`. Tier 1 authoring uses `_templates/` and `_patterns/`; annotate design-system gaps with `<!-- TODO(ds): needs pattern -->`.

## Figma Design System Rules

Use these rules for every Figma-driven UI implementation, Figma-to-code pass, or design-system sync.

**Authority order:**
- Linear issue/spec acceptance criteria are the task contract.
- `.docs/design/` is the DailyOS design-system contract: tokens, primitives, patterns, surfaces, naming, inventory, and audits.
- Shipped `src/` is the current behavior source when implementing or preserving existing UI. If docs and source disagree, read `.docs/design/_audits/shipped-component-inventory.md` before deciding whether source is behind or the docs over-promoted a prototype.
- `.docs/design/reference/` is the visual parity/reference layer. Use it to compare layout, typography, spacing, chrome, and state, but do not treat reference HTML as a replacement for the TSX/CSS module source.
- Existing Figma files are helpful but not authoritative until they are complete and reconciled with `.docs/design/` and shipped source.

**Required Figma MCP flow:**
- Run `get_design_context` for the exact node(s) before implementing from Figma.
- If context is too large or truncated, run `get_metadata`, identify the needed child node(s), then re-run `get_design_context` narrowly.
- Run `get_screenshot` for the same node/variant before coding.
- Treat MCP React/Tailwind output as a design representation, not project-ready code.
- When writing back to Figma, use `.docs/design/` specs and `.docs/design/reference/` renders as the seed; search existing Figma design-system assets first and repair/reuse them instead of recreating duplicates.

**Implementation rules:**
- Reuse existing components before creating new ones. Start in `src/components/ui`, `src/components/shared`, `src/components/editorial`, `src/components/layout`, `src/components/entity`, domain component folders, and `src/features/settings-ui`.
- New routed surfaces live in `src/pages`; shared primitives/patterns live under the appropriate `src/components/*` owner, with co-located CSS modules unless the existing component family uses Tailwind utilities.
- Prefer CSS modules and design tokens for editorial/product UI. Existing shadcn-style primitives may keep Tailwind/CVA, but do not paste raw Tailwind from Figma when a DailyOS primitive/pattern exists.
- Use `@/` imports, strict TypeScript props, functional components, `clsx` or `cn` for class composition, and existing hooks/services.
- Do not use a `proposed` primitive or pattern unless the issue explicitly promotes it. If promotion is required, update the markdown spec, source component/CSS, reference render where applicable, inventory/index, and tests together.
- Promoted design-system elements must expose `data-ds-tier`, `data-ds-name`, and `data-ds-spec`; add `data-ds-variant` or `data-ds-state` when the variant/state is meaningful.

**DailyOS visual rules:**
- DailyOS is a magazine, not a dashboard. Typography, spacing, and reading order should do most of the structural work.
- Cards are for featured content only; most content should be editorial rows, rules, lists, and sections.
- Color communicates state, entity identity, trust, or action. Do not add decorative color.
- Editorial pages should have finite endings and use `FinisMarker` unless the surface spec says otherwise.
- Preserve the magazine shell: `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer`, and `MagazinePageLayout` conventions.
- Do not introduce raw pipeline vocabulary in user-facing copy (`enrichment`, `AI enrichment`, `intelligence pipeline`). Preserve canonical labels from specs/source when already established.

**Tokens and assets:**
- Runtime tokens live in `src/styles/design-tokens.css`; markdown specs live in `.docs/design/tokens/`; the reference mirror lives in `.docs/design/reference/_shared/styles/design-tokens.css`. Keep all three in sync when changing tokens.
- Never hardcode colors, fonts, spacing scales, shadows, radii, trust bands, or entity colors when a token exists.
- Design-only Figma exports, MCP notes, screenshots, and mapping artifacts belong under `.docs/design/figma/`.
- Runtime assets belong in `src/assets/` only when the app imports them; public-root assets belong in `public/` only when they must be served directly.
- Use Figma MCP localhost asset URLs directly during implementation. Do not commit placeholder assets, and do not add new icon packages; use existing `lucide-react` icons or the specific Figma-provided asset.

## gstack

Use `/browse` for all web browsing. Never use `mcp__claude-in-chrome__*` tools.

Available: `/plan-ceo-review`, `/plan-eng-review`, `/plan-design-review`, `/plan-devex-review`, `/cso`, `/codex`, `/review`, `/ship`, `/browse`, `/qa`, `/qa-only`, `/setup-browser-cookies`, `/retro`

If skills aren't working: `cd .claude/skills/gstack && ./setup`

## Skill routing

When the user's request matches an available Codex skill, use that skill as your first action. In Codex, this means opening the advertised `SKILL.md` or using the plugin/tool route associated with that skill, then following its workflow before answering directly or using ad-hoc tools. The skill descriptions in your tool list are the canonical trigger map; trust them rather than maintaining a duplicate routing table here.
