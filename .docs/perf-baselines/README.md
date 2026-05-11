# Legacy Performance Baselines

This directory is the pre-contract Suite P baseline path. New published
performance evidence belongs under `.docs/perf/` and must follow the Evaluation
Evidence Contract in `.docs/evals/evaluation-evidence-contract.md`.

`scripts/suite-p.sh` is still the canonical Suite P runner, but DOS-348 owns
hardening it before its output can be trusted as published evidence.

Historical empty baselines are scaffolding only. They are not valid published W8
evidence, and future published Suite P runs must fail on zero real benches,
failed bench execution, malformed evidence, missing input hashes, customer data,
or absolute path leakage.
