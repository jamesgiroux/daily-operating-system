# Abilities Smoke Corpus

This corpus is a small synthetic/procedural fixture set for DOS-261 smoke
evidence. It exists to prove that the abilities/retrieval adapter can emit a
valid Evaluation Evidence Record through the DOS-503 contract.

The manifest is public and synthetic. It contains no live account data, no
private fixture material, no model or judge transcripts, and no database write
requirements. Gold expectations live in `gold.json` so fixture inputs and
scoring expectations stay separated; the runner only emits aggregate smoke
scores.

The smoke command also runs the existing EvalAbilityBridge harness fixture to
prove the evaluation path still exercises DailyOS's Evaluate-mode bridge rather
than a parallel runtime.

Published mode intentionally blocks for now. This corpus is enough for
release-enough wiring evidence, but it is not a release or public benchmark
corpus.
