# CLAUDE.md

DailyOS makes intelligence personal: memory plus judgment for the alone part of knowledge work. Native macOS app (Tauri + React) where AI produces and users consume, with trust as a product feature, not a disclaimer. Customer Success is the first slice (James is customer-zero); the substrate is general-purpose. Canonical framing lives in `.docs/design/product/` (MISSION, VISION, PRODUCT-THESIS, PHILOSOPHY, PRINCIPLES) — read those before writing user-facing prose about what DailyOS is.

## Critical Rules

**Intelligence Loop integration check.** Every new table, schema column, claim field, or user-visible intelligence surface must answer these 5 questions before shipping:
1. **Claim model:** Represented as a claim with subject attribution, temporal scope, sensitivity, and lifecycle state — not ad-hoc display data?
2. **Provenance + trust:** How is provenance captured (`source_asof`, source/field attribution), and how does it affect trust scoring / trust-band rendering (`likely_current`, `use_with_caution`, `needs_verification`)?
3. **Signals + invalidation:** What signals does it emit, and which propagation/invalidation paths refresh derived state?
4. **Runtime + surfaces:** Which abilities and contexts consume it (`build_intelligence_context()`, `gather_account_context()`, prep outputs, callouts), and what's the expected behavior across Tauri and MCP surfaces?
5. **Feedback loop:** How do corrections, dismissals, corroborations, contradictions feed back into claim state and source reliability?

Data that only renders in the frontend, without claim/provenance semantics, signal propagation, trust impact, runtime consumption, or feedback learning, is incomplete.

**All mutations go through `services/`.** No direct DB writes from command handlers. No exceptions.

**Reuse before build — DRY, SRP, separation of concerns. Non-negotiable.** Before authoring ANY new component, style, hook, or helper: search for an existing one and extend it. The canon is `src/components/` (especially `ui/`), design tokens in `src/styles/design-tokens.css`, and `.docs/design/reference/` (copy-paste-ready HTML for every token/primitive/pattern/surface, mirrored from production and gate-enforced). A second same-purpose component, a hardcoded value that shadows a token, or a re-implementation of an existing pattern is a BLOCKED finding at every review rung — extend or consume the existing unit, never fork it. **Surfaces never handle logic:** `src/pages/` and display components render props and dispatch user intent only — no business logic, no direct `invoke()` calls (those belong in hooks/services). Every plan and PR answers: *which existing components/styles does this consume, and what genuinely new unit does it add — and why couldn't an existing one be extended?*

**No customer-specific data in source code.** Never hardcode real customer domains, company names, email addresses, or account details in code, comments, or test fixtures. Use generic examples (`subsidiary.com`, `parent.com`, `user@example.com`). Customer data belongs in the encrypted DB.

**No PII in commit messages, titles, or PR bodies — including references to PII cleanup work.** Frame commits by what they accomplish for the system ("tighten test fixture lint"), not by the inputs being scrubbed ("scrub residual customer references" leaks the term it's removing). The pre-commit gate at `.githooks/pre-commit` enforces the blocklist on file content (PII blocklist source: `.claude/pii-blocklist.txt`); commit messages are on you.

**Knowledge store discovery.** Before authoring substrate-touching work, grep `docs/solutions/` and `.docs/decisions/` for prior solutions and architectural decisions. `docs/solutions/` holds documented problem→fix entries with YAML frontmatter (`module`, `tags`, `problem_type`); `.docs/decisions/` holds ADRs. Reinventing documented substrate is a BLOCKED finding at L0. See `docs/solutions/README.md` and `.docs/plans/engineering-ladder.md`.

**Definition of Done — completion, not "compiles clean."** A task is done when:
1. Acceptance criteria from the issue spec or version brief are validated with real data
2. End-to-end flow works — real data through the Intelligence Loop to rendered surfaces
3. No stubs, TODOs, or "Phase 2" deferrals
4. Tests pass: `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`

If the brief names an existing v1.4.x capability (W3 abilities, claim substrate, lifecycle, trust bands, ability runtime) as the producer for new work, **the wiring IS the work** — shipping an empty composer with a "producer is future" comment is missing the requirement. Grep for the named producer and read the wave plan before declaring anything "ships empty by design."

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

**Planning source of truth:** Linear is canonical for issues, backlog, and project execution. Git keeps durable decisions, architecture/design guidance, and occasional version or wave planning docs under `.docs/plans/`. Legacy markdown issue specs live under `.docs/_archive/` for historical reference only.

**Plan first.** Enter plan mode for any non-trivial task (3+ steps or architectural decisions). If something goes sideways, stop and re-plan — don't keep pushing.

## The Engineering Ladder (L0–L6)

Used in chat: "L2 looks good." Used in docs/headings: "L2 (Diff) review verdict: approve." Each rung names **Plan**, **Implement**, **Review**, and **Capture** skills — see **`.docs/plans/engineering-ladder.md`** for the full skill matrix.

- **L0 Prep (Plan Hardening)** — `/plan-eng-review` turns the Linear issue into a reviewable plan packet; add `/plan-ceo-review`, `/plan-devex-review`, or `/plan-design-review` when product/domain, MCP/API/DX, or UI/workflow scope is central. Hardens the ticket; not an approval gate.
- **L0 (Plan)** — pre-code review of the hardened plan. **Default 2 reviewers**: `/codex challenge` + ONE planning reviewer routed by what the plan touches (`ce-coherence-reviewer` / `ce-scope-guardian-reviewer` / `ce-feasibility-reviewer` / `ce-product-lens-reviewer` / `ce-design-lens-reviewer` / `ce-security-lens-reviewer`). Add a second planning reviewer at Wave scope or when Amendment 3 paths are touched. **K-in mandatory**: `ce-learnings-researcher` runs in parallel and cites `docs/solutions/` + `.docs/decisions/` hits. Reinvented substrate = BLOCKED. Unanimous required.
- **L1 (Implementation)** — implementing agent builds against the plan and self-validates: tests pass, proof artifacts produced, demo evidence captured — before opening the PR.
- **L2 (Diff)** — pre-merge review of the PR. **Default 1 reviewer**: `/codex review` orchestrated via the `l2-bounded-reviewer` agent (AC-scoped, path-α routing). Router adds 0–2 specialists based on what the diff touches (`pr-review-toolkit:type-design-analyzer`, `pr-review-toolkit:silent-failure-hunter`, `ce-performance-reviewer`, `ce-data-migrations-reviewer`, `ce-api-contract-reviewer`, `ce-security-reviewer`, `ce-reliability-reviewer`). **Advisory parallel (always-on, non-blocking)**: `ce-maintainability-reviewer`, `ce-pattern-recognition-specialist`, `ce-code-simplicity-reviewer`, `ce-project-standards-reviewer`, `ce-testing-reviewer` file findings to the Codebase Maintenance project, never block merge. **Runs locally, before pushing the PR**, not in CI. The `.githooks/commit-msg` hook requires every code-touching commit to carry `L2-status: passed | not-run-acknowledged | n-a-doc-only`.
- **L3 (Wave)** — after all wave PRs merge: `/codex challenge` against integrated diff + ADRs, `ce-architecture-strategist` on integrated state, plus `ce-schema-drift-detector` (waves with migrations) and `ce-deployment-verification-agent` (production-touching). **Suite S/P/E fires by need**, not always: S when wave AC names security or Amendment 3 paths touched, P when wave AC names perf or touches migrations/hot paths, E always (cheap). For non-major versions (v1.x.y), L5 drift folds into L3. **K-out mandatory**: autonomous `/ce-compound mode:headless` for each class-pattern + substrate-already-existed finding.
- **L4 (Surface)** — user-facing change QA: `/qa-only` first, `/qa` if remediation needed, `accessibility-tester` for user-facing.
- **L5 (Drift)** — **major versions only** (v1.x.0). `/plan-eng-review` + `ce-architecture-strategist` comparing integrated state to planned end-state. **K-out**: `/ce-compound-refresh` on stale `docs/solutions/` entries.
- **L6 (Human)** — James's call when the system can't resolve. Decision posted as Linear comment on the affected ticket.

**Knowledge Channel (K)** is a continuous feedback loop parallel to L0–L6: `docs/solutions/` + `.docs/decisions/` feed *into* L0 (`ce-learnings-researcher` parallel-grep on every L0) and *out of* L3 / L5 (autonomous `/ce-compound mode:headless` at retro close — single-tier; the Stop-hook reminder and retro-template gate were removed 2026-05-23 as duplicate layers).

**Wave is the project-management primitive** for big version programs (e.g., v1.4.0): parallel agents fan out within a wave, hard ordering across waves. Linear milestones may mirror waves; the wave is the execution protocol, the milestone is the Linear container. Smaller versions collapse to L0 → code → L2 → ship (add L4 if user-facing, L5 if drift risk is material, L6 if it escalates; L3 usually doesn't apply).

**Authority surface — Linear.** Linear is the canonical record. Plans live on the ticket. All reviewer verdicts, L6 decisions, and approvals are ticket comments. Git keeps wave-scoped artifacts (proof bundles, retros, ADRs) and the protocol docs.

**Full protocol** at `.docs/plans/v1.4.0-waves.md` (amendments at `.docs/plans/v1.4.0-waves-amendments.md`). The async orchestration design at `.docs/plans/orchestration/v1-lite.md` (cloud routines, claudebot, daily digest) is design-archive — none of it has shipped; current operating model is local invocation per `.docs/plans/engineering-ladder.md`.

**Parallel-wave migration slot reservations.** When parallel wave agents in the same version all add migrations, claim non-overlapping slot blocks upfront in `.docs/plans/{version}-waves.md` (e.g., `W4-A = v160–v169`, `W4-B = v170–v179`). v1.4.1 W3-C/W4-A/W4-B independently claimed v155, costing ~6h of CI rebase-renumber loops.

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
- **Commits**: Include `Co-authored-by: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`

## Behavioral Rules

- Fix bugs autonomously — just fix them, don't ask for hand-holding
- Use subagents liberally for research and parallel analysis
- Use product vocabulary (ADR-0083) for all user-facing strings
- **Don't swing past center when correcting course.** When the user pushes back on over-engineering, *trim* — don't strip. Preserve the differentiating substrate; remove only the scaffolding.
- **Same-shape findings twice = class-wide sweep, not a third patch.** When two L2/review cycles surface the same shape of issue, audit the entire class across the codebase, fix in one pass, add a CI gate. The sweep IS the work.
- **Path-α L2 findings go to maintenance, not cycle-N+1.** L2 findings that aren't a literal acceptance-criterion violation, ADR-named contract violation, or PR-introduced regression → file in **Codebase Maintenance & Production Quality** (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) and unblock the substrate PR.

**Default engineering discipline (Rules 1–11)** — think before coding, simplicity, surgical changes, goal-driven loops, judgment-only model use, surface conflicts, read before write, intent-encoding tests, checkpoints, conformance, fail loud. Full text + override interactions at **`.docs/engineering-discipline.md`**. Read it for one-shot edits, bug fixes, and small features.

## Doc Authoring — Tier Ruleset

Planning docs sit in three tiers, picked by criteria not lists:

- **Tier 1 (HTML-first)** — author directly as `.html` against the shared design system at `.docs/design/reference/_shared/`. Apply when layout carries information (status grids, matrices, dashboards, diagrams), the doc is for visual sharing, or it's a read-heavy reference. Examples: `.docs/plans/engineering-ladder.html`, wave dashboards, PRDs, strategy decks, version roadmaps.
- **Tier 2 (markdown + render)** — author `.md`, render to `.html` via a build script, commit both. Apply when content is text-dominant prose with section structure. Examples: `.docs/design/product/{MISSION,VISION,PHILOSOPHY,PRINCIPLES}.md` rendered via `build-foundation-html.mjs`.
- **Tier 3 (markdown-only)** — author `.md`, no render. Apply when code-adjacent, diff-reviewed, Linear-canonical, or grep-first. Examples: retros, proof bundles, `docs/solutions/`, ADRs, per-ticket Linear plans, MEMORY.md, commit messages.

Full ruleset, scaffolds, and dogfooding loop convention at **`.docs/plans/AUTHORING.md`**. Tier 1 authoring uses `_templates/` and `_patterns/` libraries; gaps annotated with `<!-- TODO(ds): needs pattern -->` roll into DS work.

## Figma Design System Rules

For every Figma-driven UI implementation, Figma-to-code pass, or design-system sync: authority order, required MCP flow (`get_design_context` → `get_screenshot` before coding), reuse-before-create rules, DailyOS magazine visual rules, token sync, and asset placement live at **`.docs/design/FIGMA-RULES.md`**.

## gstack

Use `/browse` for all web browsing. Never use `mcp__claude-in-chrome__*` tools.

Available: `/plan-ceo-review`, `/plan-eng-review`, `/plan-design-review`, `/plan-devex-review`, `/cso`, `/codex`, `/review`, `/ship`, `/browse`, `/qa`, `/qa-only`, `/setup-browser-cookies`, `/retro`. If skills aren't working: `cd .claude/skills/gstack && ./setup`.

## Skill routing

When the user's request matches an available skill, invoke it via the Skill tool as your FIRST action. The skill descriptions in your tool list are the canonical trigger map.
