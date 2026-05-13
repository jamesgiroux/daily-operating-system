#!/usr/bin/env bash
# Suite P - Criterion-backed performance evidence for v1.4.x hot paths.

set -euo pipefail

MODE="smoke"
OUT=""
RUN_ID=""
SCOPE=""
THRESHOLD="10"
BASELINE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --) shift ;;
    --mode) MODE="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --run-id) RUN_ID="$2"; shift 2 ;;
    --scope) SCOPE="$2"; shift 2 ;;
    --threshold) THRESHOLD="$2"; shift 2 ;;
    --baseline) BASELINE="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

case "$MODE" in
  smoke|published) ;;
  *) echo "--mode must be smoke or published" >&2; exit 2 ;;
esac

[[ -n "$SCOPE" ]] || { echo "--scope required" >&2; exit 2; }

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

if [[ -z "$RUN_ID" ]]; then
  RUN_ID="suite-p-${MODE}-$(date -u +%Y%m%dT%H%M%SZ)"
fi

SAFE_SCOPE="$(printf '%s' "$SCOPE" | tr -c 'A-Za-z0-9._-' '-')"
RAW_RUN_DIR="src-tauri/target/evidence/suite_p/${RUN_ID}"
DURABLE_RUN_DIR=".docs/perf/runs/${RUN_ID}"
BASELINE_DIR=".docs/perf/baselines"
MANIFEST_PATH=".docs/perf/suite-p-bench-manifest.json"
SCHEMA_PATH=".docs/evals/evidence-record.schema.json"
SUMMARY_PATH="${RAW_RUN_DIR}/bench-summary.json"
CURRENT_BASELINE="${BASELINE_DIR}/scope-${SAFE_SCOPE}.json"
BASELINE_CANDIDATE="${RAW_RUN_DIR}/baseline-candidate.json"
PRIOR_BASELINE=""

if [[ -z "$OUT" ]]; then
  if [[ "$MODE" == "published" ]]; then
    OUT="${DURABLE_RUN_DIR}/record.json"
  else
    OUT="${RAW_RUN_DIR}/record.json"
  fi
fi

if [[ "$MODE" == "published" ]]; then
  SUMMARY_PATH="${DURABLE_RUN_DIR}/bench-summary.json"
  if [[ -n "$BASELINE" ]]; then
    if [[ ! -f "$BASELINE" ]]; then
      echo "published Suite P baseline not found: $BASELINE" >&2
      exit 1
    fi
    PRIOR_BASELINE="$BASELINE"
  elif [[ -f "$CURRENT_BASELINE" ]]; then
    PRIOR_BASELINE="$CURRENT_BASELINE"
  else
    echo "published Suite P requires an existing comparator baseline; pass --baseline or commit ${CURRENT_BASELINE}" >&2
    exit 1
  fi
fi

mkdir -p "$RAW_RUN_DIR" "$(dirname "$OUT")"
[[ "$MODE" == "published" ]] && mkdir -p "$DURABLE_RUN_DIR" "$BASELINE_DIR"
[[ "$MODE" == "published" ]] && rm -f "$BASELINE_CANDIDATE"

STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
COMMAND="pnpm suite:p -- --mode ${MODE} --scope ${SCOPE} --run-id ${RUN_ID} --out ${OUT}"
[[ "$THRESHOLD" != "10" ]] && COMMAND="${COMMAND} --threshold ${THRESHOLD}"
[[ -n "$BASELINE" ]] && COMMAND="${COMMAND} --baseline ${BASELINE}"

CARGO_ARGS=(
  bench
  --manifest-path src-tauri/Cargo.toml
  --features bench-harness
  --bench suite_p_baseline
)

bash src-tauri/scripts/build-mcp.sh --stub >/dev/null

if [[ "$MODE" == "smoke" ]]; then
  DAILYOS_SUITE_P_BENCH_BUILD=1 cargo "${CARGO_ARGS[@]}" --no-run
else
  if [[ -d src-tauri/target/criterion ]]; then
    find src-tauri/target/criterion -type d -name "$RUN_ID" -prune -exec rm -rf {} + 2>/dev/null || true
  fi
  DAILYOS_SUITE_P_BENCH_BUILD=1 cargo "${CARGO_ARGS[@]}" -- --save-baseline "$RUN_ID"
fi

node --input-type=module - \
  "$MODE" "$SCOPE" "$RUN_ID" "$OUT" "$STARTED_AT" "$COMMAND" "$THRESHOLD" \
  "$CURRENT_BASELINE" "$PRIOR_BASELINE" "$SUMMARY_PATH" "$MANIFEST_PATH" "$SCHEMA_PATH" \
  "$BASELINE_CANDIDATE" \
  <<'JS'
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import { spawnSync } from "node:child_process";

const [
  mode,
  scope,
  runId,
  outPath,
  startedAt,
  command,
  thresholdArg,
  currentBaselinePath,
  priorBaselineInputPath,
  summaryPath,
  manifestPath,
  schemaPath,
  baselineCandidatePath,
] = process.argv.slice(2);

const repoRoot = process.cwd();
const thresholdPct = Number(thresholdArg);
if (!Number.isFinite(thresholdPct) || thresholdPct < 0) {
  throw new Error("--threshold must be a non-negative number");
}

const commit = git(["rev-parse", "HEAD"]).trim();
const branch = git(["branch", "--show-current"]).trim() || "detached";
const dirty = git(["status", "--short"]).trim().length > 0;
const finishedAt = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
const schemaHash = shaFile(schemaPath);
const manifestHash = shaFile(manifestPath);
const manifest = readJson(manifestPath);
const manifestBenchIds = manifestBenchIdsFrom(manifest);
const benchConfigHash = shaText(
  [
    readText("src-tauri/Cargo.toml"),
    readText("src-tauri/benches/suite_p_baseline.rs"),
    readText("scripts/suite-p.sh"),
  ].join("\n--- suite-p config boundary ---\n"),
);

fs.mkdirSync(path.dirname(path.resolve(repoRoot, summaryPath)), { recursive: true });

const estimates = mode === "published" ? collectCriterionEstimates(runId) : [];
const priorBaselinePath = priorBaselineInputPath || "";
const priorBaseline = priorBaselinePath ? readBaseline(priorBaselinePath) : null;
const priorBaselineHash = priorBaselinePath ? shaFile(priorBaselinePath) : null;
const manifestComparison = mode === "published"
  ? compareManifestBenches(estimates, manifestBenchIds)
  : { missing: [], extra: [] };
const priorBaselineComparison = priorBaseline
  ? compareManifestBenches(priorBaseline.benchmarks ?? [], manifestBenchIds)
  : { missing: [], extra: [] };
const currentInvalidNumericBenches = mode === "published"
  ? invalidNumericBenches(estimates, manifestBenchIds, { requirePositive: true })
  : [];
const priorInvalidNumericBenches = priorBaseline
  ? invalidNumericBenches(priorBaseline.benchmarks ?? [], manifestBenchIds, { requirePositive: true })
  : [];
const regressions = priorBaseline
  ? compareRegressions(estimates, priorBaseline.benchmarks ?? [], thresholdPct)
  : [];
const result = mode === "published" && (
  estimates.length === 0 ||
  manifestComparison.missing.length > 0 ||
  manifestComparison.extra.length > 0 ||
  priorBaselineComparison.missing.length > 0 ||
  priorBaselineComparison.extra.length > 0 ||
  currentInvalidNumericBenches.length > 0 ||
  priorInvalidNumericBenches.length > 0 ||
  regressions.length > 0
)
  ? "fail"
  : "pass";

const summary = {
  suite: "suite_p",
  lane: "DOS-348",
  mode,
  scope,
  run_id: runId,
  generated_at: finishedAt,
  criterion_baseline: mode === "published" ? runId : null,
  prior_baseline: priorBaselinePath ? toRepoRelative(priorBaselinePath) : null,
  threshold_pct: thresholdPct,
  manifest_benches: manifestBenchIds,
  bench_count: estimates.length,
  missing_bench_count: manifestComparison.missing.length,
  extra_bench_count: manifestComparison.extra.length,
  regression_count: regressions.length,
  benchmarks: estimates,
  missing_benches: manifestComparison.missing,
  extra_benches: manifestComparison.extra,
  current_invalid_numeric_bench_count: currentInvalidNumericBenches.length,
  current_invalid_numeric_benches: currentInvalidNumericBenches,
  prior_missing_bench_count: priorBaselineComparison.missing.length,
  prior_extra_bench_count: priorBaselineComparison.extra.length,
  prior_missing_benches: priorBaselineComparison.missing,
  prior_extra_benches: priorBaselineComparison.extra,
  prior_invalid_numeric_bench_count: priorInvalidNumericBenches.length,
  prior_invalid_numeric_benches: priorInvalidNumericBenches,
  regressions,
};
writeJson(summaryPath, summary);

let baselineHash = null;
if (mode === "published" && result === "pass") {
  writeJson(baselineCandidatePath, {
    baseline_version: "suite_p_baseline_v1",
    suite: "suite_p",
    lane: "DOS-348",
    scope,
    run_id: runId,
    generated_at: finishedAt,
    threshold_pct: thresholdPct,
    benchmarks: estimates,
  });
  baselineHash = shaFile(baselineCandidatePath);
}

const summaryHash = shaFile(summaryPath);
const artifactPaths = [
  artifact(summaryPath, "suite-p-summary", summaryHash, mode),
];
if (mode === "published") {
  artifactPaths.push(artifact(manifestPath, "suite-p-bench-manifest", manifestHash, mode));
  if (baselineHash) {
    artifactPaths.push(artifact(currentBaselinePath, "suite-p-baseline", baselineHash, mode));
  }
}

const inputHashes = {
  schema: schemaHash,
  bench_manifest: manifestHash,
  bench_config: benchConfigHash,
};
if (mode === "published" && baselineHash) {
  inputHashes.baseline = baselineHash;
}
if (mode === "published" && priorBaselineHash) {
  inputHashes.prior_baseline = priorBaselineHash;
}

const metricDefinitions = [
  {
    namespace: "performance_regression",
    name: "bench_count",
    description: "Number of Criterion benchmarks with collected estimate data.",
    unit: "count",
    higher_is_better: true,
  },
  {
    namespace: "performance_regression",
    name: "missing_bench_count",
    description: "Number of manifest-declared Criterion benchmarks missing from the current published run.",
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "extra_bench_count",
    description: "Number of Criterion benchmark estimates not declared in the Suite P manifest.",
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "regression_count",
    description: `Number of benchmarks regressing more than ${thresholdPct}% against the bound baseline when available.`,
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "current_invalid_numeric_bench_count",
    description: "Number of manifest-declared current benchmarks missing a positive numeric mean estimate.",
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "prior_missing_bench_count",
    description: "Number of manifest-declared Criterion benchmarks missing from the comparator baseline.",
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "prior_extra_bench_count",
    description: "Number of comparator baseline benchmark estimates not declared in the Suite P manifest.",
    unit: "count",
    higher_is_better: false,
  },
  {
    namespace: "performance_regression",
    name: "prior_invalid_numeric_bench_count",
    description: "Number of manifest-declared comparator benchmarks missing a positive numeric mean estimate.",
    unit: "count",
    higher_is_better: false,
  },
];
const metrics = [
  {
    namespace: "performance_regression",
    name: "bench_count",
    value: estimates.length,
    unit: "count",
    status: mode === "published" && estimates.length !== manifestBenchIds.length ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "missing_bench_count",
    value: manifestComparison.missing.length,
    unit: "count",
    status: manifestComparison.missing.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "extra_bench_count",
    value: manifestComparison.extra.length,
    unit: "count",
    status: manifestComparison.extra.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "regression_count",
    value: regressions.length,
    unit: "count",
    status: regressions.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "current_invalid_numeric_bench_count",
    value: currentInvalidNumericBenches.length,
    unit: "count",
    status: currentInvalidNumericBenches.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "prior_missing_bench_count",
    value: priorBaselineComparison.missing.length,
    unit: "count",
    status: priorBaselineComparison.missing.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "prior_extra_bench_count",
    value: priorBaselineComparison.extra.length,
    unit: "count",
    status: priorBaselineComparison.extra.length > 0 ? "fail" : "pass",
  },
  {
    namespace: "performance_regression",
    name: "prior_invalid_numeric_bench_count",
    value: priorInvalidNumericBenches.length,
    unit: "count",
    status: priorInvalidNumericBenches.length > 0 ? "fail" : "pass",
  },
];

for (const estimate of estimates) {
  const metricName = `criterion_mean_ns_${sanitizeMetricName(estimate.bench)}`;
  metricDefinitions.push({
    namespace: "performance_regression",
    name: metricName,
    description: `Criterion mean point estimate for ${estimate.bench}.`,
    unit: "ns",
    higher_is_better: false,
  });
  metrics.push({
    namespace: "performance_regression",
    name: metricName,
    value: estimate.mean_ns,
    unit: "ns",
    status: "info",
  });
}

const record = {
  schema_version: "evaluation_evidence_record_v1",
  suite: "suite_p",
  lane: "DOS-348",
  mode,
  scope,
  run_id: runId,
  commit,
  branch,
  dirty,
  command,
  started_at: startedAt,
  finished_at: finishedAt,
  environment: {
    os: `${os.type()} ${os.release()}`,
    arch: os.arch(),
    node: process.version,
    rustc: commandOutput("rustc", ["--version"]),
    cargo: commandOutput("cargo", ["--version"]),
    runner: "scripts/suite-p.sh",
  },
  input_hashes: inputHashes,
  metric_definitions: metricDefinitions,
  metrics,
  thresholds: {
    requires_baseline: mode === "published",
    bench_count: {
      operator: mode === "published" ? "eq" : "gte",
      value: mode === "published" ? manifestBenchIds.length : 0,
    },
    missing_bench_count: {
      operator: "eq",
      value: 0,
    },
    extra_bench_count: {
      operator: "eq",
      value: 0,
    },
    regression_count: {
      operator: "eq",
      value: 0,
    },
    current_invalid_numeric_bench_count: {
      operator: "eq",
      value: 0,
    },
    prior_missing_bench_count: {
      operator: "eq",
      value: 0,
    },
    prior_extra_bench_count: {
      operator: "eq",
      value: 0,
    },
    prior_invalid_numeric_bench_count: {
      operator: "eq",
      value: 0,
    },
  },
  result,
  artifact_paths: artifactPaths,
  privacy_class: mode === "published" ? "public" : "internal",
  publishable: mode === "published",
  dataset_source: "DailyOS synthetic Criterion hot-path benchmarks",
  dataset_license: "DailyOS synthetic/public",
  dataset_hash: manifestHash,
  redaction_status: "synthetic",
  notes:
    mode === "published"
      ? "Published Suite P Criterion evidence for synthetic v1.4.x hot-path benchmarks."
      : "Smoke Suite P evidence validates command wiring and bench compilation only.",
  extensions: {
    evidence_counts: {
      bench_count: estimates.length,
    },
  },
};
if (mode === "published" && baselineHash) {
  record.extensions.baseline_binding = {
    baseline_hash: baselineHash,
    prior_baseline_hash: priorBaselineHash,
  };
}

writeJson(outPath, record);

function collectCriterionEstimates(baselineName) {
  const criterionRoot = path.resolve(repoRoot, "src-tauri/target/criterion");
  const files = findFiles(criterionRoot, "estimates.json")
    .filter((file) => file.split(path.sep).includes(baselineName))
    .sort();
  return files.map((file) => {
    const data = JSON.parse(fs.readFileSync(file, "utf8"));
    const rel = path.relative(criterionRoot, file).split(path.sep).join("/");
    const marker = `/${baselineName}/estimates.json`;
    const bench = rel.endsWith(marker) ? rel.slice(0, -marker.length) : rel;
    return {
      bench,
      mean_ns: finiteNumber(data.mean?.point_estimate),
      median_ns: finiteNumber(data.median?.point_estimate),
      std_dev_ns: finiteNumber(data.std_dev?.point_estimate),
    };
  });
}

function manifestBenchIdsFrom(manifest) {
  if (!Array.isArray(manifest.benches)) {
    throw new Error(`${manifestPath} must contain a benches array`);
  }
  const ids = manifest.benches.map((bench) => bench?.id).sort();
  if (ids.some((id) => typeof id !== "string" || id.trim() === "")) {
    throw new Error(`${manifestPath} benches must have non-empty string ids`);
  }
  return ids;
}

function compareManifestBenches(estimates, expectedIds) {
  const expected = new Set(expectedIds);
  const actual = new Set(estimates.map((estimate) => estimate.bench));
  return {
    missing: expectedIds.filter((id) => !actual.has(id)),
    extra: Array.from(actual).filter((id) => !expected.has(id)).sort(),
  };
}

function invalidNumericBenches(estimates, expectedIds, options = {}) {
  const requirePositive = options.requirePositive === true;
  const byBench = new Map(estimates.map((estimate) => [estimate.bench, estimate]));
  return expectedIds.filter((benchId) => {
    const estimate = byBench.get(benchId);
    if (!estimate || !Number.isFinite(estimate.mean_ns)) {
      return true;
    }
    return requirePositive && estimate.mean_ns <= 0;
  });
}

function compareRegressions(current, prior, threshold) {
  const priorByBench = new Map(
    prior
      .filter((bench) => typeof bench.bench === "string" && Number.isFinite(bench.mean_ns))
      .map((bench) => [bench.bench, bench.mean_ns]),
  );
  const regressions = [];
  for (const bench of current) {
    const previous = priorByBench.get(bench.bench);
    if (!Number.isFinite(previous) || previous <= 0 || !Number.isFinite(bench.mean_ns)) {
      continue;
    }
    const deltaPct = ((bench.mean_ns - previous) / previous) * 100;
    if (deltaPct > threshold) {
      regressions.push({
        bench: bench.bench,
        prior_mean_ns: previous,
        current_mean_ns: bench.mean_ns,
        delta_pct: Number(deltaPct.toFixed(2)),
      });
    }
  }
  return regressions;
}

function newestPriorBaseline(current) {
  const dir = path.resolve(repoRoot, ".docs/perf/baselines");
  if (!fs.existsSync(dir)) {
    return "";
  }
  const currentAbs = path.resolve(repoRoot, current);
  const candidates = fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => path.join(dir, name))
    .filter((file) => path.resolve(file) !== currentAbs)
    .map((file) => ({ file, mtimeMs: fs.statSync(file).mtimeMs }))
    .sort((left, right) => right.mtimeMs - left.mtimeMs);
  return candidates[0]?.file ?? "";
}

function readBaseline(file) {
  return JSON.parse(fs.readFileSync(path.resolve(repoRoot, file), "utf8"));
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(path.resolve(repoRoot, file), "utf8"));
}

function artifact(file, kind, hash, modeValue) {
  return {
    path: toRepoRelative(file),
    kind,
    sha256: hash,
    privacy_class: modeValue === "published" ? "public" : "internal",
    publishable: modeValue === "published",
    redaction_status: "synthetic",
  };
}

function finiteNumber(value) {
  return Number.isFinite(value) ? value : null;
}

function sanitizeMetricName(value) {
  return value.replace(/[^A-Za-z0-9_]+/g, "_").replace(/^_+|_+$/g, "").toLowerCase();
}

function findFiles(root, fileName) {
  if (!fs.existsSync(root)) {
    return [];
  }
  const out = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const full = path.join(root, entry.name);
    if (entry.isDirectory()) {
      out.push(...findFiles(full, fileName));
    } else if (entry.isFile() && entry.name === fileName) {
      out.push(full);
    }
  }
  return out;
}

function shaFile(file) {
  return shaText(fs.readFileSync(path.resolve(repoRoot, file)));
}

function shaText(value) {
  return `sha256:${crypto.createHash("sha256").update(value).digest("hex")}`;
}

function readText(file) {
  return fs.readFileSync(path.resolve(repoRoot, file), "utf8");
}

function writeJson(file, value) {
  const resolved = path.resolve(repoRoot, file);
  fs.mkdirSync(path.dirname(resolved), { recursive: true });
  fs.writeFileSync(resolved, `${JSON.stringify(value, null, 2)}\n`);
}

function toRepoRelative(file) {
  return path.relative(repoRoot, path.resolve(repoRoot, file)).split(path.sep).join("/");
}

function git(args) {
  return commandOutput("git", args);
}

function commandOutput(commandName, args) {
  const result = spawnSync(commandName, args, {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    return "";
  }
  return result.stdout.trim();
}
JS

node scripts/validate-evidence-record.mjs "$OUT"
node scripts/lint-evidence-artifacts.mjs "$OUT" "$SUMMARY_PATH"
if [[ "$MODE" == "published" ]]; then
  PUBLISHED_LINT_TARGETS=("$DURABLE_RUN_DIR" "$MANIFEST_PATH")
  [[ -f "$BASELINE_CANDIDATE" ]] && PUBLISHED_LINT_TARGETS+=("$BASELINE_CANDIDATE")
  node scripts/lint-evidence-artifacts.mjs "${PUBLISHED_LINT_TARGETS[@]}"
fi

RESULT="$(node -e 'const fs=require("fs"); console.log(JSON.parse(fs.readFileSync(process.argv[1],"utf8")).result)' "$OUT")"
if [[ "$RESULT" != "pass" ]]; then
  echo "Suite P ${MODE} result: ${RESULT}" >&2
  exit 1
fi

if [[ "$MODE" == "published" ]]; then
  [[ -f "$BASELINE_CANDIDATE" ]] || {
    echo "Suite P published baseline candidate missing after pass" >&2
    exit 1
  }
  mkdir -p "$(dirname "$CURRENT_BASELINE")"
  BASELINE_TMP="${CURRENT_BASELINE}.tmp.$$"
  trap 'rm -f "$BASELINE_TMP"' EXIT
  cp "$BASELINE_CANDIDATE" "$BASELINE_TMP"
  mv "$BASELINE_TMP" "$CURRENT_BASELINE"
  trap - EXIT
fi

printf '%s\n' "$OUT"
