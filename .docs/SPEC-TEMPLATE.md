# Spec Template

Copy the relevant sections into a new Linear issue description (or the body of a new ADR). Every issue starts with the **Core** block; then add whichever **Shape-specific** block matches the work — a new capability, a schema change, a migration, a bug fix, a refactor, a research spike, or a prompt edit. Delete sections that do not apply rather than leaving them blank. Mark any item N-A with a one-line reason.

The Build-ready and Done checklists are copied into the PR description at the appropriate milestone. Keep them in the issue too so the spec is self-contained.

---

## Core — every issue

### Problem

<One sentence a non-expert can understand.>

### Why now

<Specific. User impact, what this unblocks, the incident that caused it. Not "improves quality.">

### Scope limits

<What this deliberately does not do. What the spec explicitly leaves to a follow-on.>

### Acceptance criteria

<Measurable. Name the real data, entities, or invocations that will be used for validation. Bullets should be falsifiable — a reviewer can tell whether each one is met.>

### Intelligence Loop fit — CLAUDE.md critical rule

Every issue touching data or behaviour answers all five. "Does not apply" is a valid answer; silence is not.

- **Signals:** <does this emit signals, and which propagation rules consume them?>
- **Health scoring:** <does this feed any of the six health dimensions, or the confidence on one?>
- **Intel context:** <does this edit `build_intelligence_context()` or `gather_account_context()`, or the prompts that consume them?>
- **Briefing callouts:** <does this trigger callouts, and which dedup rule applies?>
- **Feedback hook:** <how does user interaction flow back into source reliability or trust inputs?>

### Architectural surfaces touched

Walk through this list. Mark each Touched / Not touched / N-A. A surface that is touched but not addressed in the spec is the usual source of missed detail.

- [ ] Services layer — mutations, signal emission, propagation
- [ ] Abilities contract — capability category, typed input/output, composition
- [ ] Provenance — envelope, trust score, source attribution
- [ ] Execution mode — mode-aware services, clock injection, deterministic replay
- [ ] Source taxonomy — `DataSource` classification, lifecycle (fresh / stale / revoked)
- [ ] Temporal primitives — trajectory snapshots, health curves, relationship strength
- [ ] Claims layer — assertion append-only vs mutate, trust versioning, tombstones
- [ ] Signal granularity — policy registry, coalescing, durable invalidation
- [ ] Migration — parallel run, divergence, cutover
- [ ] Evaluation harness — fixtures, quality thresholds, regression gate
- [ ] Surface parity — Tauri, MCP, chat behave identically
- [ ] Privacy rendering — per-surface masking, dev-only vs user-facing

### Edge cases

Enumerate. Mark each Handled with a test, Out-of-scope, or Deferred to #NNN. Add domain-specific cases. Minimum set:

- Empty or missing input state.
- Stale data (source freshness below threshold).
- Null, empty, or malformed inputs.
- Race — parallel invocation on the same target; idempotency story.
- User intent persistence — a user-removed or user-corrected value is not silently overwritten.
- Value instability — same input producing different outputs run-to-run; detection or suppression.
- Revoked or lifecycle-constrained source.

### Dependencies

<Every blocker with link and current status. Specify landing order if more than two. Flag any dependency that is "accepted in concept but not yet written" — unwritten dependencies are not dependencies you can rely on.>

---

## Shape-specific blocks — add the one that matches

### Shape A — New capability or ability

- **Category:** Read / Transform / Publish / Maintenance.
- **Call-graph effect:** <must match category; Read and Transform have no transitive mutation>.
- **Input type:** <name>.
- **Output type:** <name, wrapped in the standard output + provenance envelope>.
- **Composition:** <which capabilities feed context; which this invokes>.
- **Consumers:** <which surfaces call this — Tauri command, MCP tool, another capability>.

### Shape B — Schema or data-model change

- **New tables or columns:** <schema + indexes>.
- **Append-only vs mutate-in-place:** <decided, with reason. Err toward append-only for claim-like or AI-derived data so run-over-run diffs and stability-as-confidence remain possible>.
- **Negative knowledge / tombstones:** <how "absent by user intent" is distinguished from "absent because unknown" — required when users can remove values that AI might otherwise repopulate>.
- **Pruning policy:** <retention window and mechanism if data volume is unbounded>.
- **Read path:** <how existing queries change; any indexes dropped>.
- **Write path:** <single site per mutation type; service function that owns it>.

### Shape C — Migration replacing existing capability or table

- **Parallel-run plan:** <writes to where, reads from where, duration>.
- **Divergence monitor:** <cadence, threshold, alert target>.
- **Cutover criteria:** <numerical. Default: ≥100 invocations, ≤1% unexplained divergence over 7 days>.
- **Rollback path:** <feature flag; tested in dev before Phase 1 ships anywhere>.
- **Backfill strategy (if any):** <offline batch, online lazy, or N-A. Runtime estimate measured before enabling>.

### Shape D — Bug fix

- **Symptom:** <what the user or system sees; exact reproduction conditions>.
- **Minimum reproduction:** <smallest input that triggers it. Committed as a failing test before the fix>.
- **Root cause:** <required. No fix ships without a named root cause. If the root cause is "not yet known," this is a research spike, not a bug fix>.
- **Fix strategy:** <what changes and why it resolves the root cause, not the symptom>.
- **Regression test:** <the test that proves the bug stays fixed. Lives in the repo alongside the fix>.
- **Adjacent surfaces:** <other places the same root cause could manifest. Each either checked or filed as a follow-on>.
- **Escape analysis:** <why existing tests / types / reviews did not catch this. Feeds `tasks/lessons.md` when the answer is reusable>.

### Shape E — Refactor (same observable behavior)

- **Observable behavior preserved:** <what a user, consumer, or API client cannot tell changed>.
- **What changes internally:** <files, abstractions, boundaries moved>.
- **How we prove behavior is preserved:** <existing test coverage, parallel run, A/B, type-system-only change, etc.>.
- **Why now:** <the concrete pressure forcing the refactor; "cleaner" is not a reason>.

### Shape F — Research spike

- **Open questions:** <what is unknown that blocks committing to a design>.
- **Options under consideration:** <at least two; a spike that already knows the answer is an implementation, not a spike>.
- **Decision criteria:** <how we pick the winner. Specific, measurable where possible>.
- **Deliverable:** <design doc, ADR draft, POC code behind a feature flag, or benchmark. Committed to the repo, not a Slack thread>.
- **Exit criteria:** <what lets us call the spike finished and start implementation>.

### Shape G — Prompt edit (model-facing template change)

- **Template ID:** <stable identifier>.
- **Version bump:** <prior → new; bumped per semver even if cosmetic>.
- **What changed semantically:** <what the model now produces differently>.
- **Fixture impact:** <which fixtures regenerate; new baseline committed>.
- **Expected eval delta:** <score change direction on rubric dimensions; rebaseline if score moves outside tolerance>.
- **Rollback:** <previous template version remains checked out; feature flag or config switch toggles in an emergency>.

---

## Build-ready checklist — paste into PR description

Keep the lines that apply to the shape. Drop the rest.

- [ ] Output envelope / provenance populated end-to-end; trust score computed non-zero where applicable
- [ ] Source attribution populated for every AI-synthesized field
- [ ] Prompt template has stable ID + semver; hash whitespace-stable (prompt-edit or capability shapes)
- [ ] No `Utc::now()` or `rand::thread_rng()` in code paths intended to be replay-deterministic; clock + RNG injected from context
- [ ] Mutation guards in place on every write; transactions wrap multi-statement mutations
- [ ] Sources classified per the project's source taxonomy; stale and revoked handled explicitly
- [ ] ≥1 eval fixture authored; quality thresholds set for AI-producing code paths
- [ ] All mutations flow through the services layer; signal emission inside the service call
- [ ] Regression test committed (bug-fix shape)
- [ ] `cargo clippy -- -D warnings` + `cargo test` + `pnpm tsc --noEmit` green; `pnpm test` when frontend touched

## Done checklist — paste into PR description

- [ ] Validated with the real data named in acceptance criteria, not a synthetic equivalent
- [ ] End-to-end flow demonstrated in the PR (screenshot, recording, or reproducible command)
- [ ] Cutover evidence attached if this was a migration (divergence log, final sample size)
- [ ] Output renders correctly across every surface it can reach (Tauri, MCP, chat where relevant)
- [ ] Follow-on issues filed for every deferred concern; linked in PR description
- [ ] `tasks/lessons.md` updated if a near-miss, correction, or escape surfaced a reusable pattern

---

## Failure modes this template catches

If a completed spec reads as if any of these could happen, the spec is not ready.

- Acceptance criterion is "works" rather than naming the test data.
- New data-producing table without a decision on append-only vs mutate, producing silent history loss later.
- Capability that transitively mutates state without declaring it.
- AI-produced output merged with a zero or defaulted trust score.
- Time-dependent logic using wall-clock directly, breaking deterministic replay.
- A user-removed value silently resurrected by the next automated run.
- Bug fix merged without a named root cause — symptom patched, not cause.
- Research spike merged without a committed deliverable.
- Prompt edit merged without fixture regeneration or rebaseline.
- "Phase 2" TODO committed as shipped code.
- Dependency on an unwritten design doc, flagged only verbally.
- Parallel-run cutover declared on a sample of 20 invocations.

---

## ADR adaptation

For ADRs, wrap the Core block's Problem / Why now / Scope limits as Context. Use the relevant shape-specific block and the architectural surfaces list as Decision detail. Drop Build-ready and Done checklists — those live on issues, not ADRs. Add a Consequences section (Positive / Negative / Neutral) as ADRs require.
