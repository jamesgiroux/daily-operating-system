# Repo Cleanup Progress — 2026-04-05

## Chunk: Linear migration batch 1 + migration notes
- **What changed**
  - Created Linear projects for `v1.1.2`, `v1.1.3`, `v1.3.0`, and `v1.4.0` based on the existing markdown plan briefs.
  - Created missing Linear issues for the requested active sets: `I660`, `I661`, `I662`, `I604`, `I605`, `I606`, `I607`, `I608`.
  - Added small README migration notes to `.docs/issues/` and `.docs/plans/` so the repo now states that active issue/planning execution lives in Linear.
  - Preserved the migration policy and inventory docs already prepared in `.docs/`.
- **What was migrated to Linear**
  - Projects:
    - `v1.1.2 — Transcript Routing Fix`
    - `v1.1.3 — Design Hardening`
    - `v1.3.0 — Report Engine Rebuild: Intelligence-First, Display-Only Reports`
    - `v1.4.0 — Publication + Portfolio + Intelligence Quality`
  - Issues:
    - `I660` → `DOS-36`
    - `I661` → `DOS-37`
    - `I662` → `DOS-38`
    - `I604` → `DOS-39`
    - `I605` → `DOS-40`
    - `I606` → `DOS-41`
    - `I607` → `DOS-42`
    - `I608` → `DOS-43`
- **Commit hash if committed**
  - `97e1e86` — `docs: record Linear migration policy and first migration batch`
- **Rollback notes**
  - Repo-side changes are limited to additive docs/notes and can be reverted with `git revert 97e1e86`.
  - Linear-side objects are now created; rollback there would require manual deletion/cancellation if desired.
- **Next recommended step**
  - Conservative non-destructive cleanup pass for clearly misplaced docs only.
  - Reassess whether any obviously safe destructive cleanup remains worth doing tonight.

## Chunk: Non-destructive misplaced-doc normalization
- **What changed**
  - Moved `design/CSS-DESIGN-SYSTEM-AUDIT-2026-03-21.md` into `.docs/audits/`.
  - Moved `docs/ai-poller-token-reduction-proposal-2026-03-29.md` into `.docs/research/`.
  - No files were deleted in this chunk.
- **What was migrated to Linear**
  - None.
- **Commit hash if committed**
  - `b3adc8b` — `docs: normalize misplaced audit and research notes`
- **Rollback notes**
  - Both moves can be reverted with `git revert b3adc8b`.
- **Next recommended step**
  - Only consider obviously safe destructive cleanup items next.
  - Safe candidates still visible tonight: `notification-demo.html` and `.stash/bob-phase4a-changes.patch`, but only remove them if James is comfortable losing local residue/history in git-tracked form.
