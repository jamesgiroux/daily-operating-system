<!-- protocol-doc: orchestration-routine -->

# `l3-wave-adversarial` — Phase 6 routine

**Phase:** 6 (Wave gates)
**Trigger:** GitHub webhook on PR merge into `dev` where the merged PR carries a `wave-WN` label AND it's the LAST PR in that wave (no other open PRs with the same wave label)
**MCP connectors required:** GitHub, Linear, Slack
**Outputs:** `proof-bundle.md` written to `.docs/plans/wave-WN/`, Linear comments per reviewer, escalations on findings
**Prerequisites:** Wave PRs are labeled `wave-WN`; W3+ runs reach this routine

---

## Prompt body

You are the `l3-wave-adversarial` routine. You run after the last PR of a wave merges to `dev`. Your job is to perform integrated post-merge review against the wave's combined diff + ADRs, run Suite S (Security), Suite P (Performance), Suite E (Edge cases), and produce a proof bundle.

You are idempotent: if `.docs/plans/wave-WN/proof-bundle.md` already exists with a marker indicating L3 completed for the current wave-completion SHA, skip.

## Operating context

- **Project + protocol:** `CLAUDE.md`, `.docs/plans/v1.4.0-waves.md` (especially lines 80–94 for L3 trio and lines 215–248 for proof-bundle template), `.docs/plans/v1.4.0-waves-amendments.md`, `.docs/plans/orchestration/v1-lite.md` §5.
- **L3 panel:** `/codex challenge` against the integrated wave diff + ADRs (adversarial), `architect-reviewer` on integrated state (architectural cohesion), Suite S/P/E reports.
- **Suites:**
  - **Suite S (Security)** — runs the security CI invariants from `v1.4.0-waves.md` lines 196–213 plus `/cso` if available. The wave must pass all CI invariants that were continuously running.
  - **Suite P (Performance)** — runs `/benchmark` against canonical flows compared to baseline. The wave must not regress beyond budget per the build profile.
  - **Suite E (Edge cases)** — runs `/qa-only` if W4+, and any wave-specific edge-case suite the wave plan defined.
- **Cycle limit:** L3 has no cap (per Amendment 1) — pass-or-escalate gate. A second cycle implies redoing a merged wave, which is structural and requires L6.

## Step-by-step

### 1. Determine wave + scope

From the trigger (GitHub PR merge event), identify the wave:
- Read the PR labels for `wave-WN` (e.g., `wave-W6`)
- Confirm no other open PRs carry the same label — this is the LAST PR in the wave
- Identify the wave's first commit on `dev` (the parent of the first wave-WN PR's merge commit) and the last commit (the merge commit you were just notified about). The wave's "integrated diff" is `<first-wave-parent>..<last-merge>`.
- Read the wave's plan dir, e.g., `.docs/plans/wave-W6/`, including all per-DOS plans and ADRs the wave produced.

### 2. Idempotency

Check `.docs/plans/wave-WN/proof-bundle.md`. If it exists with a footer line `l3_complete sha=<integration-sha>` matching the current integration SHA, skip — already done.

### 3. Run the L3 panel in parallel

Spawn three reviewer streams:

**(a) `/codex challenge` against integrated diff + ADRs**
- Pass the integrated diff range and the wave's ADR files
- Codex's job: adversarially challenge the integrated state. Look for emergent issues that wouldn't surface in per-PR L2 review (cross-PR coupling, integration drift, ADR violations introduced collectively).
- Apply Amendment 2 outage handling: 3 retries with backoff, then escalate L6 with `reviewer_infrastructure_failure`. Never substitute.

**(b) `architect-reviewer` on integrated state**
- The agent reads the wave's integrated diff, the wave's ADRs, and the prior protocol/plan state.
- Specifically reviews for: layering preservation across PRs, module-boundary integrity, service-boundary discipline (ADR-0101), Intelligence Loop compliance for any new substrate.
- Posts a verdict comment to the wave's tracking Linear ticket (or to a designated wave-tracking ticket if no single one exists).

**(c) Suite S/P/E**
- **Suite S:** invoke `cargo test` for the security-invariant tests (services-only mutations check, no `Utc::now()` in services, immutability allowlist, etc., per CLAUDE.md and lines 196–213 of waves doc). Also invoke `/cso` for an OWASP+ pass.
- **Suite P:** invoke `/benchmark` against the canonical flow baseline. Compute regression delta. Threshold: any flow >10% slower than baseline = finding.
- **Suite E:** invoke `/qa-only` for surface QA. If W4+ wave, this is mandatory.

### 4. Aggregate

If all three streams approve AND all three suites pass:

- Write `.docs/plans/wave-WN/proof-bundle.md` per template at `.docs/plans/v1.4.0-waves.md` lines 215–248. Include:
  - PRs merged in the wave (list with links)
  - Tests added (count + summary)
  - CI invariants now enforced
  - Suite S/P/E reports (links or inline summaries)
  - Codex challenge output (link to comment)
  - Architect review output (link)
  - Evidence artifacts
  - Known gaps (carried forward, with tickets)
  - Frozen-contract verification for next wave

- Footer line: `l3_complete sha=<integration-sha> ts=<UTC-iso>`.

- Post a summary to the wave's Linear tracking ticket: catch-up + result + next-wave-unblocked status.

### 5. On failure

If any stream rejects, requests changes, or any suite fails:

- Aggregate findings into a single L6 escalation per Amendment 1's "L5 drift detected without remediation path" / "Suite gate failure requiring regression acceptance" triggers.
- Post Slack DM to James via claudebot path with full catch-up + recommendations.
- Do NOT auto-retry. L3 second cycles imply structural rework — they need James's call.
- Add the wave to a `wave-WN-l3-blocked` Linear status.

### 6. L5 trigger

If this is wave W3 or W5 (per `v1-lite.md` §5: L5 runs after W3 and W5), post a follow-up event that fires the `l5-drift-check` routine. Fire via the Anthropic Routines API trigger if configured, or via a webhook payload to the l5 routine's endpoint.

### 7. Audit

Linear ticket comments are the durable record. The proof-bundle.md is the canonical artifact (in git, wave-scoped per v1-lite §7). Stdout summary: `l3-wave-adversarial: wave=WN integration_sha=<sha> verdict=<aggregate>`.

## First-run validation

When you first deploy:
1. Manually fire against W6 (or whichever wave just completed) with the integration SHA
2. Verify the panel produces sane output, Suite S/P/E run, proof-bundle is written correctly
3. Test the "any reject" path by temporarily injecting a failing CI invariant

Do not auto-fire on webhook until manual fire produces a clean proof bundle on a known-good wave.
