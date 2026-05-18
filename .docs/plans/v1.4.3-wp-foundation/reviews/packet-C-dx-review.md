# L0 Packet C — DX review verdict

**Reviewer:** DX review (developer experience)
**Packet:** `.docs/plans/v1.4.3-wp-foundation/L0-packet-C-starter-kit.md` V1.0
**Cycle:** 1
**Date:** 2026-05-18
**Lens:** "Can a new contributor ship a working WordPress block in <5 minutes using this kit?" (AC #1)
**Branch:** docs/v143-l0-packets · **Linear:** DOS-678

## Verdict: **CONDITIONAL APPROVE**

The kit shape is right — extracting v1.4.2 W4-F (`dailyos/account-overview`) into
templates + CLI + shared harness is the correct abstraction, and the load-bearing
integration harness directly addresses the DOS-670 contract-mismatch failure mode.
Five DX gaps need folding into V1.1 before implementation:

1. **CLI ergonomics — flags-only without sane defaults invites failure modes.** (§5.1, F1)
2. **`--help`-IS-docs claim doesn't survive a 5-minute first-time author scenario.** (§6.1, F2)
3. **Template ergonomics — the "edit the TODO regions" boundary is under-specified.** (§5.3 / §5.4, F3)
4. **Integration harness macro syntax + failure-mode diagnostic shape missing.** (§5.5, F4)
5. **Translation utility's "needs human review" boundary not enumerated.** (§5.7, F5)

A sixth concern (first-time author runbook) is downgraded to a recommendation
rather than a blocker — see F6. None of these contest the architectural shape;
all are V1.1-foldable in one pass.

---

## Per-focus findings

### F1 — CLI ergonomics (§5.1) — BLOCKING

**Finding.** The spec lands on `pnpm dailyos:new-block <name> [--payload-shape simple|composite] [--ability <producer-name>]` (flags only). Two issues:

- **No `--shape` default is stated for `--payload-shape`** beyond §5.1's prose ("Default: simple Pill-shape"). AC #1's exemplar (`pnpm dailyos:new-block test-block`) doesn't pass the flag, so the default IS the test — codify it in the AC, not just in narrative.
- **`--ability` flag is overloaded.** §5.1 step 4 says "if the ability doesn't exist, copy the producer template" — but provides no way to opt OUT of producer generation (e.g., when the block consumes an existing ability). Flag should split into `--ability <name>` (existing, link only) vs `--new-ability <name>` (scaffold + register), with `--ability` defaulting to "infer from block name kebab-case" when omitted.

**Interactive vs flag-only.** Recommend **hybrid**: flag-only when ALL required flags are present (scriptable for codex tasks); fall back to interactive prompts (`prompts`/`enquirer`, NOT `inquirer` — `inquirer` v9 is ESM-only and breaks in Tauri's build script context) when invoked bare. This matches how `pnpm create vite` and `cargo generate` work and serves both first-time authors (interactive guidance) and codex-rescue tasks (scriptable flags). Avoid `--yes` flag escape hatches; the validator at step 1 already guards on name shape.

**Fold for V1.1.** §5.1: enumerate the default behavior table (no flags → simple, no ability), split `--ability` semantics, name the prompt library, and add AC #1a "bare `pnpm dailyos:new-block test-block` produces a working simple block with no prompts when stdin is non-TTY (CI mode)."

### F2 — `--help`-IS-docs claim (§6.1) — BLOCKING

**Finding.** §6.1 codifies "the CLI's `--help` output + template inline comments + harness macro/function docs ARE the documentation." That's defensible for the **second** block author (who's seen one example) — it fails for the **first** author for three reasons:

1. **`--help` describes flags, not flows.** A first-time author reading `--help` learns what `--payload-shape` does, not when to choose simple vs composite, where to put their payload schema, how the producer wires to the projection, or what the harness fixture file looks like. Template inline comments handle some of this but only after the scaffold lands.
2. **The integration test harness has no example fixture in the spec.** §5.5 shows the struct shape but no end-to-end "here's what a real fixture looks like for the simple shape" example. A first-time author has nothing to copy.
3. **The "your second block" path is fine, the "your first block" path isn't.** §6.1's anti-drift argument is sound for steady-state — drift between docs and code is the failure mode the constraint exists to prevent — but a 30-line `wp/dailyos/scripts/templates/README.md` that points at the AC #1 invocation, the reference block (`dailyos/account-overview`), and the harness fixture example is NOT documentation that drifts. It's a launch checklist.

**Recommendation.** Refine §6.1 to: "Authoring playbook prose ships as ONE short `wp/dailyos/scripts/templates/README.md` (≤50 lines, structural pointers only — no API surface description). Substantive guidance lives in `--help` + template inline comments + harness docs (as authored). This file is on the CI-grep allowlist for the no-drift sweep: the README MUST reference only files that exist; CI rejects PR if any referenced path is missing."

**Fold for V1.1.** §6.1: relax the absolutism (`--help` PLUS one structural pointer file). Add AC #9: "`wp/dailyos/scripts/templates/README.md` exists; every file path it mentions resolves; CI gate (`docs-path-resolve.sh`) enforces."

### F3 — Template ergonomics (§5.2, §5.3, §5.4) — BLOCKING

**Finding.** Reading the spec's producer template (§5.3) end-to-end with eyes of a developer who has seen `account_overview.rs`:

- The template body shows `normalize_input → prepare → commit_composition → finalize_provenance` and says "TODO: implement normalize_input, prepare_{{ability_fn_name}}, helper types." That's the **hard part** of authoring a producer — what `prepare_*` actually does (build a `CompositionProposal`, attach blocks, declare provenance) is exactly the surface the v1.4.2 W4-F author had to discover by reading 500+ lines of substrate.
- The simple-shape projection rule template (§5.4) declares two field bindings (Title + Text) but the second-block author doesn't know what `BindingRole` variants exist or how `FieldPath::new("payload.x")` resolves against producer output. A grep-driven discovery loop here is exactly what the kit is supposed to eliminate.
- The simple ↔ composite template boundary is underspecified. Spec says composite is "same as simple, plus per-block-type render branches" — but the W4-F composite ships 4 payload dispatch branches (`payload.text`, `payload.items`, `payload.nodes`, `payload.context`). Does the composite scaffold ship a stub for each? Does it ship one branch + a "add more here" comment? Different answers, different DX.

**Recommendation for V1.1:**

- §5.3: producer template body should include a **WORKED prepare_ body for the simple shape** (10-15 lines, building a single-block composition with one payload field). Mark with `// REPLACE WITH YOUR LOGIC — this is the simple-shape reference.` The harness fixture exercises this exact body, so the developer's first run is green out of the box; they then edit toward their actual payload.
- §5.4: projection rule template comment block should enumerate the `BindingRole` variants inline (Title, Text, Subtitle, Status, Timestamp, etc. — whatever the enum actually defines; grep `BindingRole::` in `fallback_projection.rs`) so the second author doesn't have to grep substrate.
- §5.2: explicitly state composite-shape ships ONE dispatch branch (`payload.text`) plus a `// Add additional payload variants here — see dailyos/account-overview for the canonical multi-variant pattern.` comment. Don't ship 4 stubs (3 of which the author will delete).

**Acceptance criterion gap.** AC #1 says the CLI "produces a working block" but doesn't assert the **default scaffold passes the integration harness without ANY developer edits**. Add AC #1b: "Bare scaffold (no edits beyond CLI invocation) passes the kit's reference integration fixture." This is the load-bearing DX assertion — if a developer can't run the harness green on the un-edited scaffold, the kit is broken.

### F4 — Integration test harness UX (§5.5) — BLOCKING

**Finding.** §5.5 shows the `BlockIntegrationFixture` struct and names `integration_test_block!` macro + `run_block_integration_fixture` function — but punts on macro body (`macro_rules! integration_test_block { ... }`) and doesn't specify the failure-mode diagnostic shape. Two concrete gaps:

- **Macro shape isn't named.** Is it `integration_test_block!(my_block, { ability_name: "...", ... })` (struct-literal-shape) or `integration_test_block!(my_block, fixture_fn_returning_fixture)` (builder shape)? The struct-literal form is friendlier for first-time authors (everything inline); the builder form is friendlier for sharing fixture chunks across multiple test invocations. Pick struct-literal for v1.4.3 simplicity (W2 has ~6 primitives; sharing is premature).
- **DOS-670 diagnostic shape isn't spec'd.** §5.5 says "panics with a diagnostic on contract mismatch (the kind that DOS-670 found)" but doesn't show what the diagnostic looks like. A first-time author hitting a producer/projection mismatch needs to see something like:

  ```
  CONTRACT MISMATCH in dailyos/my-block:
    Projection declared field binding: payload.body (BindingRole::Text)
    Producer output keys (from Composition):
      - payload.text  ← did you mean this?
      - payload.title
    Fix: update the projection rule's FieldPath, or update the producer
    to emit payload.body. See projection-rule template @ <abs path>.
  ```

  Without spec'ing the diagnostic body, "DOS-670-style" is hand-wavy. The harness can be technically correct (returns error) but operationally useless (cryptic message).

**Recommendation for V1.1:**

- §5.5: spec the macro as struct-literal-form with one inline example showing the full simple-shape fixture for the kit's reference block.
- §5.5: spec the diagnostic format (4 fields: location, declared, actual, suggested-fix) with a worked example. Add AC #2a: "Diagnostic output for fixture #5 includes (a) the offending block name, (b) the declared field path, (c) the producer's actual output keys, (d) a 'did you mean' suggestion via edit-distance match."
- §5.5: name the PHP harness invocation contract explicitly. "Shells out to PHP" is fine but specify: cwd, env vars, exit-code semantics for distinguishing "renderer-found-mismatch" from "PHP-process-crashed" — the second case must not be silently reported as the first.

### F5 — Translation utility expectations (§5.7) — CONDITIONAL

**Finding.** §5.7 names the boundary as "90% scaffold" with "needs human review" comments for complex shapes, and §12 open question 1 asks codex challenge to identify primitives the translator definitively can't handle. That's the right escalation, but the spec needs a **concrete enumeration** before the developer hits it, not after.

Examples of likely-unsupported shapes (from a glance at `src/components/ui/`):
- React components with hooks (`useState`, `useEffect`) — no PHP equivalent; primitive must be re-authored as render-only.
- Components consuming context via `useContext` — block.json has no context model.
- Components with imperative refs or animations — out of scope for render-only blocks.
- Components conditionally rendering different child trees via runtime state — translator can map static conditionals but not state-driven.

**Recommendation for V1.1.** §5.7: add an **unsupported-shapes table** that the translator detects up-front and exits cleanly with "this primitive needs manual scaffolding from the simple template; reason: <hook|context|ref|state-driven-children>." Don't emit a 90% scaffold for these — emit nothing and the actionable error. AC #5a: "Translator on a hook-using primitive (e.g., InlineInput) exits 1 with the unsupported-shape diagnostic; does NOT emit a partial scaffold." This prevents the failure mode where a developer trusts the 90% scaffold and ships a broken block because the 10% gap was load-bearing.

### F6 — First-time author runbook — RECOMMENDATION (non-blocking)

**Finding.** Question 6 in the prompt asks whether the plan should include a "your first block in 5 minutes" walkthrough. Per F2, the right shape is a ≤50-line `templates/README.md` with structural pointers (invocation + reference block + fixture example), NOT a separate walkthrough doc that will drift. The recommendation in F2 covers this; no additional artifact needed.

If implementation reveals first-time-author friction during L1/L4, file follow-up to extend the README to a recipe-shape ("here's a worked Pill block from `pnpm new-block` to merged PR in 5 commits") — but don't speculate-author it in V1.1.

---

## Recommendations summary

| # | Section | Recommendation | Blocking? |
|---|---|---|---|
| F1 | §5.1 | Hybrid CLI (flags + interactive fallback via `prompts`/`enquirer`); split `--ability` semantics; codify defaults in AC #1a | YES |
| F2 | §6.1 | Allow ONE ≤50-line `templates/README.md` structural pointer file; CI gate path-resolution; AC #9 | YES |
| F3 | §5.2/§5.3/§5.4 | Worked `prepare_*` body in producer template; enumerate `BindingRole` variants in projection template; composite ships 1 branch + extension comment; AC #1b "bare scaffold passes harness" | YES |
| F4 | §5.5 | Spec macro as struct-literal-form with example; spec diagnostic format (location/declared/actual/did-you-mean); PHP shell-out contract; AC #2a | YES |
| F5 | §5.7 | Enumerate unsupported-shape table; translator exits 1 with diagnostic on unsupported input rather than emitting partial scaffold; AC #5a | YES (CONDITIONAL — small fold) |
| F6 | §6.1 follow-on | No new artifact; README from F2 covers; revisit if L1/L4 surfaces friction | NO |

## Convergence path

All five blocking findings are **mechanical V1.1 folds** — none require re-litigating §6.1's core anti-drift principle, the §5.5 harness architecture, or the translator's 90%-scaffold premise. V1.1 author lands all five in one pass; DX review re-runs cycle-2 to confirm folds.

Expected cycle-2 verdict on clean folds: APPROVE.
