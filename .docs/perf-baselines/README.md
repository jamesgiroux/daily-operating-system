# Performance baselines

Per-scope criterion bench output, written by `scripts/suite-p.sh` when L3
runs. Each scope's `scope-{id}.json` captures the integrated state's bench
numbers; subsequent scopes compare against the most recent prior baseline.

A regression beyond 10% on any flow fails L3 / suite-p. Tune the threshold
via `--threshold N` to `suite-p.sh`.

When no benches are registered in the workspace, the suite seeds an empty
baseline file rather than failing — first-pass deployment is intentionally
soft so the surface ships before benches are written.
