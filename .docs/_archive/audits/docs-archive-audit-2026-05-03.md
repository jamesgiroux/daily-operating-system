# `.docs` Archive Audit — 2026-05-03

## Scope

Audit of `.docs` and immediate subdirectories for files or folders that should move to an `_archive` directory, be migrated to Linear, or be excluded from git as generated operational output.

## Current State

- `.docs` has local changes outside this audit:
  - modified: `.docs/architecture/DATA-MODEL.md`, `.docs/architecture/MODULE-MAP.md`
  - untracked: current design/mockup drops, prompt logs, and the learning draft dated 2026-04-26
- Existing policy still holds: `.docs/linear-migration-policy.md` says Linear is canonical for issues, project execution, version briefs, backlog management, and status tracking.
- Git tags now include releases through `v1.2.2` and wave completion tags through `v1.4.0-w3-substrate-complete`.
- Human direction on 2026-05-03: leave wave directories in place, leave `v1.4.0+` plans in place, and keep active/current work.
- Follow-up direction on 2026-05-03: archive contracts, audits, explorations, issues, mockups, and the legacy markdown backlog because Linear is now the exclusive issue/backlog/project tracking surface.

## Cleanup Applied

### Archived issue files moved

Moved to `.docs/issues/_archive/`:

- `.docs/issues/i199-archived.md`
- `.docs/issues/i482-archived.md`
- `.docs/issues/i543-archived.md`
- `.docs/issues/i546-archived.md`
- `.docs/issues/i547-archived.md`
- `.docs/issues/i550-archived.md`

### Shipped version briefs moved

Moved to `.docs/plans/_archive/shipped/`:

- `.docs/plans/v1.0.1.md` -> `.docs/plans/_archive/shipped/v1.0.1-shipped.md`
- `.docs/plans/v1.0.2.md` -> `.docs/plans/_archive/shipped/v1.0.2-shipped.md`
- `.docs/plans/v1.0.3.md` -> `.docs/plans/_archive/shipped/v1.0.3-shipped.md`
- `.docs/plans/v1.0.4.md` -> `.docs/plans/_archive/shipped/v1.0.4-shipped.md`
- `.docs/plans/v1.1.0.md` -> `.docs/plans/_archive/shipped/v1.1.0-shipped.md`
- `.docs/plans/v1.1.1.md` -> `.docs/plans/_archive/shipped/v1.1.1-shipped.md`
- `.docs/plans/v1.1.2.md` -> `.docs/plans/_archive/shipped/v1.1.2-shipped.md`
- `.docs/plans/v1.1.3.md` -> `.docs/plans/_archive/shipped/v1.1.3-shipped.md`
- `.docs/plans/v1.2.0.md` -> `.docs/plans/_archive/shipped/v1.2.0-shipped.md`

### Root-level issue artifacts moved

Moved to `.docs/_archive/issue-artifacts/i652/`:

- `.docs/i652_phase7_integration_tests_summary.md`
- `.docs/i652_phase7_verification_report.md`

### Migration inventory files moved

Moved to `.docs/audits/`, then archived with the rest of the audit history under `.docs/_archive/audits/`:

- `.docs/issues-migration-inventory-2026-04-05.md`
- `.docs/plans-migration-inventory-2026-04-05.md`
- `.docs/linear-migration-prep-2026-04-05.md`

### Whole directories archived

Moved into `.docs/_archive/`:

- `.docs/audits/` -> `.docs/_archive/audits/`
- `.docs/contracts/` -> `.docs/_archive/contracts/`
- `.docs/explorations/` -> `.docs/_archive/explorations/`
- `.docs/issues/` -> `.docs/_archive/issues/`
- `.docs/mockups/` -> `.docs/_archive/mockups/`

### Legacy backlog archived

Moved to `.docs/_archive/BACKLOG.md`.

## Strong Candidates, Needs Quick Human Signoff

### Plan files with naming/content drift

- `.docs/plans/v2.1.0.md` has historical inventory evidence of a heading mismatch.
- `.docs/plans/v2.2.0.md` has historical inventory evidence of a heading mismatch.

Recommendation: leave in place for now because they are `v1.4.0+` by filename. Revisit only in a separate planning cleanup.

## Generated Output, Not Archive Material

These should not become durable archive content unless there is a specific reason to preserve raw agent/tool transcripts.

### Prompt logs

Untracked prompt logs account for roughly 17.5 MB:

- `.docs/plans/wave-W3/_prompts/_logs/` — about 6.0 MB
- `.docs/plans/wave-W4/_prompts/_logs/` — about 4.1 MB
- `.docs/plans/wave-W5/_prompts/_logs/` — about 2.3 MB
- `.docs/plans/wave-W6/_prompts/_logs/` — about 5.1 MB

Recommendation: add an ignore rule for `**/_prompts/_logs/` and delete or move these outside the repo. If evidence is needed, preserve a small proof-bundle summary instead of raw logs.

### Design inventory scratch files

- `.docs/design/_pending-inventory-updates.log` is untracked and contains generated inventory state.
- `.docs/design/_pending-inventory-updates.log.tmp` is already ignored by the global `*.tmp` rule.

Recommendation: either convert the log into a real audit note or ignore/delete it. Do not archive as-is.

## Larger Cleanup Candidates

### Archived directories

The archived directories should remain historical reference only. New issue/backlog/project execution material belongs in Linear. New canonical design guidance belongs in `.docs/design/`; exploratory mockups should only be restored from archive if they are actively being promoted.

## Keep Active

Do not archive these without a separate content review:

- `.docs/ARCHITECTURE.md`
- `.docs/architecture/`
- `.docs/decisions/`
- `.docs/design/` canonical docs, primitives, patterns, surfaces, tokens, and reference files
- `.docs/plans/wave-W*/`
- `.docs/plans/v1.3.0.md`
- `.docs/plans/v1.4.0.md`
- `.docs/plans/v1.4.0-implementation.md`
- `.docs/plans/v1.4.0-waves.md`
- `.docs/plans/v2.1.0.md`
- `.docs/plans/v2.2.0.md`
- `.docs/research/`
- `.docs/runbooks/`
- `.docs/fixtures/`
- `.docs/strategy/`
- `.docs/learnings/`

## Recommended Cleanup Order

1. Add an ignore rule for prompt logs, then remove the untracked `_prompts/_logs/` directories from the working tree.
2. Keep extracting durable knowledge from archived issues/backlog into ADRs, architecture, design, research, or runbooks only when it is still useful.
