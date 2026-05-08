<!-- protocol-doc: dailyos-pr-template -->

## Linear ticket

<!-- Required. Paste the Linear ticket URL or DOS-### identifier. -->
<!-- The plan lives on the ticket per orchestration v1-lite §2 — reviewers will read it from there. -->



## Summary

<!-- One or two sentences. What does this PR change and why? -->



## What changed

<!-- Bullets. Concrete, file-level when relevant. -->

-

## Test plan

<!-- Mark each item once verified. Add or remove items as needed. -->

- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test --lib` passes
- [ ] `pnpm tsc --noEmit` clean
- [ ] `pnpm test` passes (if frontend changes)
- [ ] Manual verification: <describe what you walked through>

## Definition of Done

<!-- Per CLAUDE.md DoD. All four must be true at merge. -->

- [ ] Acceptance criteria from the Linear ticket are validated with real data (not stubs)
- [ ] End-to-end flow works through to rendered surfaces
- [ ] No stubs, TODOs, or "Phase 2" deferrals introduced (or each is tracked as its own ticket)
- [ ] Tests pass: `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`

## §4 Security

<!-- Required field. Set to true | false. CI validates. -->

`security_auditor_invoked: <true | false>`

<!-- If false, cite the exemption ID and rationale. -->

**Exemption ID** (if `security_auditor_invoked: false`): `EXEMPT-DOC-ONLY` | `EXEMPT-TEST-FIXTURE` | `EXEMPT-RENAME-NON-SENSITIVE` | `EXEMPT-STYLE` | _none_

**Exemption rationale**:

<!-- Yes/no questions. CI's path-prefix detection overrides false answers when a trigger path matches. -->

- [ ] Touches `src-tauri/src/services/`, `abilities/`, `bridges/`, `commands/`, or `intelligence/`?
- [ ] Modifies `.sql` migrations, `src-tauri/src/migrations.rs`, or `src-tauri/migrations/`?
- [ ] Touches a claim-substrate table (`intelligence_claims`, `claim_corroborations`, `claim_contradictions`, `agent_trust_ledger`, `claim_feedback`, `claim_repair_job`, `claim_projection_status`)?
- [ ] Changes a prompt template (`.claude/**/*.md`, `.claude/skills/**`, `.claude/hooks/**`, `.claude/routines/**`, `prompts/**`)?
- [ ] Modifies sensitivity, rendering policy, redaction, allowlist, or actor-filter logic?
- [ ] Modifies MCP configs (`.claude/settings*.json`), tool registry, or skill definitions?
- [ ] Adds a new trust boundary (auth, scope, sensitivity tier)?
- [ ] Defines a privileged action (push, merge, post, run code, write to claim tables, mutate sensitivity)?
- [ ] Adds or modifies `.github/workflows/*` with network egress, secret access, or merge/publish authority?
- [ ] Adds a new dependency (`Cargo.toml`, `package.json`, lockfiles)?

If any answer is yes, `security_auditor_invoked: true` is required.

## Follow-ups

<!-- Out-of-scope items discovered during this work. File as separate Linear tickets where appropriate. -->

-

## Notes for review

<!-- Anything reviewers should know that isn't obvious from the diff. -->



---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
