# Lessons

- 2026-05-22: When a surface starts rendering claim-backed rows, treat opaque claim ids as machine metadata only. Visible block copy must come from rendered claim text or a surface-specific assessment composer, and class-wide row renderer sweeps need a regression gate so one fixed block does not leave sibling blocks leaking ids.
- 2026-05-22: Do not assume another agent session still owns dirty branch state after a handoff. Re-check the worktree and commit the active branch state directly when the user confirms no peer session is running.
