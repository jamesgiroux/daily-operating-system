# Performance Evidence

Performance evidence uses the Evaluation Evidence Contract defined in
`.docs/evals/evaluation-evidence-contract.md`.

## Roots

- Published baselines: `.docs/perf/baselines/`
- Published run records: `.docs/perf/runs/<run-id>/`
- Raw local outputs: `src-tauri/target/evidence/suite_p/<run-id>/`

## Suite P Modes

Smoke mode proves command wiring and record validity. It does not prove release
performance unless the owning issue explicitly promotes a smoke run.

Published mode is release evidence. It must fail when there are zero real
benches, bench execution fails, a required baseline is missing, the evidence
record is malformed, input hashes are missing, or privacy/publication rules fail.

`scripts/suite-p.sh` is the canonical runner. Published runs bind the manifest,
bench config, current baseline, and comparator baseline hashes, and compare every
manifest bench before updating the current baseline artifact.

## Stage 8a Checks

```bash
pnpm evidence:validate
pnpm evidence:lint .docs/perf
pnpm wave8:smoke
```

## Legacy Path

`.docs/perf-baselines/` contains the pre-contract Suite P baseline path. New
published evidence should use `.docs/perf/` and emit Evaluation Evidence Records.
Empty baselines are historical scaffolding only and are not valid published W8
evidence.
