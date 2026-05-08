# Performance baselines

Per-wave criterion bench output, written by `scripts/suite-p.sh` when the L3
wave-review workflow runs. Each wave's `wave-WN.json` captures the integrated
state's bench numbers; subsequent waves compare against the most recent prior
baseline.

A regression beyond 10% on any flow fails L3 / suite-p. Tune the threshold
via `--threshold N` to `suite-p.sh`.

When no benches are registered in the workspace, the suite seeds an empty
baseline file rather than failing — first-pass deployment is intentionally
soft so the surface ships before benches are written.
