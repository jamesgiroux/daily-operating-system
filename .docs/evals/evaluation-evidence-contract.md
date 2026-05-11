# Evaluation Evidence Contract

This contract defines the reusable evidence shape for DailyOS benchmark, eval,
release-gate, and comparison runs.

## Vocabulary

- **Evaluation Evidence Contract**: this reusable contract.
- **Evaluation Evidence Record**: one machine-readable JSON record emitted by a
  suite, benchmark, eval, or comparison adapter.
- **Evaluation Evidence Packet**: a human-readable release or public comparison
  bundle built from one or more records.

Do not name the contract after a version or wave. Version/wave identifiers belong
in the `scope` field of each record.

## Required Record Fields

Every Evaluation Evidence Record uses schema version
`evaluation_evidence_record_v1` and includes:

- `schema_version`: contract version.
- `suite`: source suite or adapter, such as `suite_p`, `suite_s`, `suite_e`,
  `abilities_eval`, `gbrain_comparison`, or `release_gate`.
- `lane`: issue/lane identifier that produced the record.
- `mode`: `smoke`, `published`, `manual`, `ci`, or `dry_run`.
- `scope`: version, wave, PR, issue, or release scope.
- `run_id`: unique run identifier.
- `commit`, `branch`, `dirty`: git binding for the run.
- `command`: exact command used to produce the evidence.
- `started_at`, `finished_at`: RFC3339 timestamps.
- `environment`: OS, architecture, tool versions, runner version, and relevant
  model/config values.
- `input_hashes`: `sha256:<64 lowercase hex>` hashes of fixtures, datasets,
  schemas, configs, and adapters.
- `metric_definitions`: metric vocabulary used by this run.
- `metrics`: measured values.
- `thresholds`: pass/fail threshold definitions.
- `result`: `pass`, `fail`, `blocked`, or `not_run`.
- `artifact_paths`: repo-relative output paths with privacy metadata.
- `privacy_class`: highest sensitivity of the record.
- `publishable`: whether this record is eligible for public packets.
- `dataset_source`, `dataset_license`, `dataset_hash`: dataset binding.
  `dataset_hash` uses the same `sha256:<64 lowercase hex>` format.
- `redaction_status`: `synthetic`, `redacted`, `reviewed`, or `blocked`.
- `notes`: human-readable context.

Adapters that need extra non-contract data may use an `extensions` object. Do
not put required pass/fail, artifact, privacy, dataset, or metric data only in
`extensions`; that data belongs in the contract fields above.

## Modes

`smoke` proves wiring and contract validity with a small run. Smoke records are
useful for implementation confidence, but they are not published release
evidence unless an issue explicitly says so.

`published` is release or public evidence. Published records must fail closed on:

- zero benches or zero fixtures;
- failed benchmark/eval execution;
- missing baselines when a baseline comparison is required;
- customer data or private fixture payloads;
- absolute local paths;
- missing input hashes;
- malformed hash bindings;
- malformed JSON or schema violations.

Published records must include `extensions.evidence_counts`. Suite P records use
`bench_count`; abilities/retrieval and comparison records use `fixture_count` or
`sample_count`. If `thresholds.requires_baseline` is `true`, the record must bind
the baseline hash through `input_hashes.baseline`, `input_hashes.prior_baseline`,
or `extensions.baseline_binding.baseline_hash`.

## Metric Namespaces

Keep metric families separate:

- `performance_regression`
- `retrieval`
- `answer_quality`
- `provenance_quality`
- `temporal_correctness`
- `trust_band_correctness`
- `surface_safety`
- `public_comparison`

Do not create a blended competitive composite unless every component metric is
also independently named, comparable, and reported.

## Artifact Roots

- Raw local outputs: `src-tauri/target/evidence/<suite>/<run-id>/`
- Durable performance baselines: `.docs/perf/baselines/`
- Durable performance runs: `.docs/perf/runs/<run-id>/`
- Corpus manifests: `.docs/evals/corpora/<corpus-id>/manifest.json`
- Eval run summaries: `.docs/evals/runs/<run-id>/`
- Public comparison packets: `.docs/evals/comparisons/gbrain/<run-id>/`

Public records and packets use repo-relative paths only. Absolute paths, home
directories, Windows drive or UNC paths, identity-map paths, private cache paths,
and private fixture payload paths are not valid public evidence.

## Command Surface

Stage 8a exposes the reusable command surface that later adapters consume:

```bash
pnpm evidence:validate <record-or-directory>
pnpm evidence:lint <artifact-root>
pnpm evidence:contract-smoke -- --out <path>
pnpm eval:abilities -- --mode smoke
pnpm eval:gbrain -- --mode smoke
pnpm suite:p -- --scope <scope> --out <path>
pnpm suite:s -- --scope <scope> --out <path>
pnpm suite:e -- --scope <scope> --out <path>
pnpm wave8:smoke
```

`pnpm evidence:validate` validates Evaluation Evidence Records.
`pnpm evidence:lint` checks evidence artifact roots for customer data, local
paths, identity maps, scrub artifacts, and private path tokens.
`pnpm evidence:contract-smoke` emits a synthetic Stage 8a record under
`src-tauri/target/evidence/contract/<run-id>/record.json` by default.
`pnpm wave8:smoke` runs all three Stage 8a checks.
`pnpm eval:abilities` and `pnpm eval:gbrain` are present as fail-closed Stage
8b entry points until DOS-503 lands and DOS-505 opens those implementation lanes.

DOS-348, DOS-261, and DOS-504 own the later adapters that emit Suite P,
abilities-eval, and gbrain comparison records through this contract.

## Validator Requirements

The canonical validator must enforce this contract, not only check that familiar
field names exist. It must fail closed on:

- unknown top-level fields;
- unknown nested fields inside `metric_definitions`, `metrics`, and
  `artifact_paths`;
- metric values outside `number`, `boolean`, `string`, or `null`;
- missing, malformed, or wrong-typed nested fields, including metric definition
  booleans, metric and artifact enum values, artifact path strings, artifact
  `sha256` strings, publishability booleans, evidence count numbers, threshold
  operators, baseline-binding objects/strings, dataset binding strings, input
  hash strings, and `thresholds.requires_baseline` booleans;
- `pass` results with failed metrics, false thresholds, unknown threshold
  operators, or thresholds that have no matching metric;
- absolute paths, Windows local paths, home-directory paths, `file://` paths,
  identity-map paths, or private fixture payload references;
- publishable artifacts or records that are not public, or have blocked
  redaction state;
- published records without input hashes, required evidence counts, dataset
  binding, or baseline binding when `thresholds.requires_baseline` is `true`.
- publishable or published `extensions` with raw prompt, message, transcript,
  model-output, judge-output, or private fixture payload keys.

The command surface must also be consistent under `pnpm` invocation. Commands
with positional targets must prove `--` separator parity before Stage 8b starts.
Exceptions are only allowed for commands with no positional target surface, and
the owning issue must explain why the command is exempt.

`pnpm wave8:smoke` must include negative probes proving malformed records fail
closed and valid `--` separator usage succeeds for `evidence:validate` and
`evidence:lint`. Negative probes must cover extra fields and wrong value types
inside `metric_definitions`, `metrics`, and `artifact_paths`; invalid enum
strings; wrong-typed evidence counts; wrong-shaped baseline binding; missing
published evidence counts; missing required input hashes; missing dataset
binding; missing published Suite P bench manifest/config hash; missing required
baseline binding; malformed hashes; Windows/local/private paths; blocked
publishable artifacts; false result/threshold combinations; raw transcript or
prompt extension keys; binary artifact lint; nested and root symlink lint. Smoke
proof must include a `--` parity probe for each W8 command with a positional
target surface that uses a sentinel target whose result differs from defaults, or
an explicit no-positional-target exemption recorded in the owning issue.

## Evaluation Evidence Packet

An Evaluation Evidence Packet is a human-readable bundle generated from one or
more Evaluation Evidence Records. A packet manifest must include:

- packet id, title, scope, authoring issue, and generated timestamp;
- included record paths and hashes;
- artifact index with repo-relative paths and hashes;
- privacy class, publishable flag, dataset source/license/hash, and redaction
  status for the packet and each included record;
- metric caveats and public-claim rules;
- failure appendix, including excluded failures and the reason for exclusion;
- methodology notes, including whether the packet is release evidence, internal
  evidence, or public comparison evidence.

Public comparison packets must keep retrieval metrics, answer-quality metrics,
and DailyOS-native trust/provenance metrics in separate sections. Retrieval
recall is not answer quality.

## Privacy Rules

Public artifacts must not contain customer-specific names, domains, emails,
account details, identity maps, private fixture payloads, absolute local paths, or
unreviewed judge transcripts.

Evidence lint fails closed on binary artifacts unless a later lane introduces a
reviewed manifest with explicit hash, kind, privacy class, and redaction status.

Judge/model outputs are untrusted inputs. Schema-validate them, sanitize them
before publication, store them separately from claim/trust inputs, and never
write judge text as DailyOS intelligence.

## Intelligence Loop Check

Any evidence that scores claims, provenance, trust bands, temporal currentness,
runtime consumption, or feedback behavior must answer the Intelligence Loop
questions before it can be release or public evidence.

| Question | Evaluation evidence answer |
| --- | --- |
| Claim model | Metrics that score facts about accounts, people, projects, meetings, or commitments must state whether each scored fact is represented as a claim with subject attribution, temporal scope, sensitivity, and lifecycle state. Display-only facts cannot count as trustworthy-answer evidence. |
| Provenance + trust | Records must bind source/corpus/fixture hashes and score provenance, source freshness, and trust-band rendering separately from answer correctness. Public packets must not claim trustworthiness without provenance or citation metrics. |
| Signals + invalidation | Evidence that depends on derived state must name the invalidation or refresh path that would update the result in the app. Static corpus results must be labeled as corpus-bound, not live-current product state. |
| Runtime + surfaces | Surface-safety, MCP, Tauri, provenance, or trust-band evidence must use real bridge/rendering paths before making user-surface claims. Strip-based fixture comparison is not enough for public trust/surface claims. |
| Feedback loop | Evidence involving corrections, contradictions, dismissals, or corroborations must state whether feedback changes claim state, source reliability, trust inputs, or only the evaluated fixture expectation. |
