# L2 Codex Review — DOS-546 W1-E.1 (cycle 3)

- **Date:** 2026-05-11
- **Branch:** `dos-546-wp-studio-spike`
- **HEAD:** `7705e6fd`
- **Cumulative surface under review:** `9d09c1c3` (Composition types widen + `pub(crate)` constructor + `scripts/check_composition_authorship.sh`) + the workflow-step portion of `57a57e1f` (CI wiring of three Wave 1 gates into `.github/workflows/test.yml`).
- **Reviewer:** codex (gpt-5.5, xhigh reasoning, read-only sandbox)
- **Cycle:** 3
- **Acceptance criterion under verification:** `.docs/plans/dos-546/v1.4.2-project/02-issues.md` line 666 — *"`Composition` is produced ONLY by abilities (ADR-0130 §1, substrate-owned authorship); a CI lint asserts no non-ability code constructs `Composition` directly."*

## Verdict

**APPROVE.**

Cycle-2 P1-1 (workflow-step uncommitted) is closed.

## Evidence

- HEAD `7705e6fd` includes `57a57e1f` in history; the workflow step **"Enforce ADR-0130 substrate-owned composition authorship"** is committed in `.github/workflows/test.yml` (line 93) and invokes `./scripts/check_composition_authorship.sh` on line 94, immediately after the **"Enforce durable source comments"** step as planned.
- Rust enforcement layer (primary): `Composition::new` is `pub(crate)` in `src-tauri/abilities-runtime/src/abilities/composition.rs` (line 570). Cross-crate authorship is rejected by the compiler at the module boundary.
- CI defense-in-depth layer: `scripts/check_composition_authorship.sh` is present, executable, and uses ripgrep to scan for both construction shapes (`Composition::new(` and `Composition { ... }`) across the workspace, with `src-tauri/abilities-runtime/**`, target/, node_modules/, `_archive/`, and `*.md` correctly excluded. The script also excludes itself to avoid self-match.
- Running `./scripts/check_composition_authorship.sh` locally on HEAD exits 0 (no offending sites today).

## Cumulative gate state vs AC line 666

Both layers required by AC line 666 are present in HEAD:

1. **Rust visibility layer** — `pub(crate) fn new(...)` on `Composition` in the abilities-runtime crate; non-abilities call sites cannot link.
2. **CI grep lint** — workflow step wired in `.github/workflows/test.yml`; script committed at `scripts/check_composition_authorship.sh` and runs every PR.

Defense-in-depth posture is intact: if the constructor visibility is ever widened by accident, the grep gate catches struct-literal and `::new(...)` call sites in any non-substrate path.

## Notes

- Cycle-2 finding P1-1 (script existed but workflow step was uncommitted) is fully closed by `57a57e1f`.
- No new findings.
- No path-α residuals to file against the maintenance project.

**Verdict: APPROVE.**
