#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

const SCHEMA_VERSION = "evaluation_evidence_record_v1";
const RUNNER_VERSION = "dos261_abilities_smoke_v1";
const DEFAULT_SCOPE = "v1.4.1-W8";
const LANE = "DOS-261";
const SUITE = "abilities_eval";
const SCRIPT_PATH = "scripts/eval-abilities.mjs";
const SCHEMA_PATH = ".docs/evals/evidence-record.schema.json";
const MANIFEST_PATH = ".docs/evals/corpora/abilities-smoke/manifest.json";
const GOLD_PATH = ".docs/evals/corpora/abilities-smoke/gold.json";
const HARNESS_FIXTURE_PATH = "src-tauri/tests/fixtures/bundle-2";
const ALLOWED_MODES = new Set(["smoke", "published"]);

const METRIC_DEFINITIONS = [
  {
    namespace: "retrieval",
    name: "expected_source_recall",
    description: "Share of sealed-gold expected source ids present in deterministic top-k retrieval.",
    unit: "ratio",
    higher_is_better: true,
  },
  {
    namespace: "retrieval",
    name: "expected_source_precision",
    description: "Share of deterministic top-k retrieval results that match sealed-gold expected source ids.",
    unit: "ratio",
    higher_is_better: true,
  },
  {
    namespace: "answer_quality",
    name: "required_fact_coverage",
    description: "Share of sealed-gold required fact ids present in the deterministic answer evidence set.",
    unit: "ratio",
    higher_is_better: true,
  },
  {
    namespace: "surface_safety",
    name: "eval_bridge_smoke_passed",
    description: "Whether the existing DailyOS EvalAbilityBridge harness fixture executed successfully in Evaluate mode.",
    unit: "ratio",
    higher_is_better: true,
  },
];

const THRESHOLDS = {
  requires_baseline: false,
  expected_source_recall: {
    operator: "gte",
    value: 1,
  },
  expected_source_precision: {
    operator: "gte",
    value: 1,
  },
  required_fact_coverage: {
    operator: "gte",
    value: 1,
  },
  eval_bridge_smoke_passed: {
    operator: "gte",
    value: 1,
  },
};

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});

async function main() {
  const repoRoot = findRepoRoot();
  const startedAt = new Date();
  const options = parseArgs(process.argv.slice(2));

  if (options.help) {
    printUsage();
    return;
  }

  const mode = options.mode ?? "smoke";
  if (!ALLOWED_MODES.has(mode)) {
    throw new Error(`--mode must be one of: ${Array.from(ALLOWED_MODES).join(", ")}`);
  }

  const runId = options.runId ?? createRunId(mode, startedAt);
  const scope = options.scope ?? DEFAULT_SCOPE;
  const outPath = path.resolve(
    repoRoot,
    options.out ?? `src-tauri/target/evidence/abilities_eval/${runId}/record.json`,
  );
  const outRel = toRepoRelative(repoRoot, outPath);
  const manifest = readJson(path.join(repoRoot, MANIFEST_PATH));
  const gold = readJson(path.join(repoRoot, GOLD_PATH));
  const harnessSmoke = mode === "smoke" ? runEvalBridgeSmoke(repoRoot) : skippedHarnessSmoke();
  const evaluation = evaluateManifest(manifest, gold, harnessSmoke);
  const git = gitInfo(repoRoot);
  const inputHashes = collectInputHashes(repoRoot);
  const artifactPaths = buildArtifactPaths(outRel, inputHashes.corpus_manifest, mode);
  const finishedAt = new Date();
  const publishedBlocked = mode === "published";
  const result = publishedBlocked ? "blocked" : evaluation.passed ? "pass" : "fail";

  const record = {
    schema_version: SCHEMA_VERSION,
    suite: SUITE,
    lane: LANE,
    mode,
    scope,
    run_id: runId,
    commit: git.commit,
    branch: git.branch,
    dirty: git.dirty,
    command: renderCommand(outRel, options, mode),
    started_at: startedAt.toISOString(),
    finished_at: finishedAt.toISOString(),
    environment: {
      os: os.platform(),
      arch: os.arch(),
      node_version: process.version,
      pnpm_version: commandOrNull(repoRoot, "pnpm", ["--version"]),
      runner_version: RUNNER_VERSION,
      evaluator: "eval_bridge_smoke_plus_manifest_retrieval_scorer",
    },
    input_hashes: inputHashes,
    metric_definitions: METRIC_DEFINITIONS,
    metrics: buildMetrics(evaluation),
    thresholds: THRESHOLDS,
    result,
    artifact_paths: artifactPaths.map((artifact) => ({
      ...artifact,
      publishable: publishedBlocked ? false : artifact.publishable,
    })),
    privacy_class: "internal",
    publishable: false,
    dataset_source: manifest.dataset_source,
    dataset_license: manifest.dataset_license,
    dataset_hash: inputHashes.corpus_manifest,
    redaction_status: manifest.redaction_status,
    notes: publishedBlocked
      ? "Published mode is blocked: the abilities-smoke corpus is a lightweight synthetic smoke corpus, not a release or public benchmark corpus."
      : "Synthetic DOS-261 smoke record for abilities/retrieval evidence; no customer data, model transcripts, evaluator retry loop, evaluation_traces, or DB writes are used.",
    extensions: {
      evidence_counts: {
        fixture_count: evaluation.fixtureCount,
        sample_count: evaluation.fixtureCount,
      },
      corpus: {
        id: manifest.corpus_id,
        version: manifest.corpus_version,
        manifest_path: MANIFEST_PATH,
        gold_hash: inputHashes.sealed_gold,
      },
      methodology: {
        adapter:
          "existing DailyOS EvalAbilityBridge smoke fixture plus deterministic subject filter/query-term retrieval scorer",
        eval_bridge_fixture: HARNESS_FIXTURE_PATH,
        eval_bridge_command: harnessSmoke.command,
        sealed_gold_boundary: "Gold expectations are loaded from gold.json after runner output.",
        emits_fixture_level_details: false,
        uses_live_customer_data: false,
        writes_persistent_database: false,
        uses_runtime_evaluator_retry: false,
        writes_evaluation_traces: false,
        makes_trust_or_provenance_surface_claims: false,
      },
      score_summary: evaluation.scoreSummary,
      sample_ids: evaluation.sampleIds,
    },
  };

  if (publishedBlocked) {
    record.extensions.blocked_reason =
      "Minimal synthetic smoke corpus is sufficient for release-enough wiring evidence but insufficient for published benchmark evidence.";
  }

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(record, null, 2)}\n`);
  validateAndLintRecord(repoRoot, outRel, artifactPaths);
  console.log(outRel);

  if (publishedBlocked) {
    console.error(record.notes);
    process.exit(2);
  }

  if (!evaluation.passed) {
    console.error("abilities smoke evaluation failed threshold checks");
    process.exit(1);
  }
}

function parseArgs(args) {
  const options = {};

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];

    if (arg === "--") {
      continue;
    }
    if (arg === "--help" || arg === "-h") {
      options.help = true;
      continue;
    }
    if (arg === "--mode") {
      options.mode = readValue(args, (index += 1), "--mode");
      continue;
    }
    if (arg.startsWith("--mode=")) {
      options.mode = arg.slice("--mode=".length);
      continue;
    }
    if (arg === "--scope") {
      options.scope = readValue(args, (index += 1), "--scope");
      continue;
    }
    if (arg.startsWith("--scope=")) {
      options.scope = arg.slice("--scope=".length);
      continue;
    }
    if (arg === "--run-id") {
      options.runId = readValue(args, (index += 1), "--run-id");
      continue;
    }
    if (arg.startsWith("--run-id=")) {
      options.runId = arg.slice("--run-id=".length);
      continue;
    }
    if (arg === "--out") {
      options.out = readValue(args, (index += 1), "--out");
      continue;
    }
    if (arg.startsWith("--out=")) {
      options.out = arg.slice("--out=".length);
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }

  return options;
}

function readValue(args, index, name) {
  const value = args[index];
  if (!value || value === "--") {
    throw new Error(`${name} requires a value`);
  }
  return value;
}

function printUsage() {
  console.log(
    `Usage: node ${SCRIPT_PATH} [--mode smoke|published] [--scope <scope>] [--run-id <id>] [--out <path>]`,
  );
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function evaluateManifest(manifest, gold, harnessSmoke) {
  const documents = new Map(manifest.documents.map((document) => [document.id, document]));
  const evaluationAsOf = new Date(manifest.evaluation_asof);
  const currentWindowDays = manifest.temporal?.current_window_days ?? 45;
  const perFixture = manifest.fixtures.map((fixture) =>
    evaluateFixture(fixture, goldForFixture(gold, fixture), documents, evaluationAsOf, currentWindowDays),
  );

  const metricValues = {
    expected_source_recall: average(perFixture.map((fixture) => fixture.retrievalRecall)),
    expected_source_precision: average(perFixture.map((fixture) => fixture.retrievalPrecision)),
    required_fact_coverage: divide(
      sum(perFixture.map((fixture) => fixture.requiredFactsFound)),
      sum(perFixture.map((fixture) => fixture.requiredFactsTotal)),
    ),
    eval_bridge_smoke_passed: harnessSmoke.passed ? 1 : 0,
  };

  return {
    fixtureCount: perFixture.length,
    sampleIds: perFixture.map((fixture) => fixture.id),
    metricValues,
    passed: Object.entries(metricValues).every(([name, value]) =>
      evaluateThreshold(value, THRESHOLDS[name].operator, THRESHOLDS[name].value),
    ),
    scoreSummary: {
      fixtures_scored: perFixture.length,
      thresholds_met: Object.entries(metricValues).filter(([name, value]) =>
        evaluateThreshold(value, THRESHOLDS[name].operator, THRESHOLDS[name].value),
      ).length,
      thresholds_total: Object.keys(THRESHOLDS).filter((key) => key !== "requires_baseline").length,
      eval_bridge_fixture: harnessSmoke.fixture,
    },
  };
}

function evaluateFixture(fixture, gold, documents, evaluationAsOf, currentWindowDays) {
  const retrieved = retrieveFixtureDocuments(fixture, documents);
  const retrievedIds = retrieved.map((document) => document.id);
  const expectedIds = gold.expected_source_ids;
  const expectedIdSet = new Set(expectedIds);
  const facts = new Set(retrieved.flatMap((document) => document.facts));
  const expectedDocuments = expectedIds.map((id) => documents.get(id)).filter(Boolean);
  const newestExpectedAge = Math.min(
    ...expectedDocuments.map((document) => daysBetween(document.source_asof, evaluationAsOf)),
  );
  const isFreshEnoughForSmoke = newestExpectedAge <= currentWindowDays * 3;

  return {
    id: fixture.id,
    retrievalRecall: divide(countIntersection(retrievedIds, expectedIdSet), expectedIds.length),
    retrievalPrecision: divide(countIntersection(retrievedIds, expectedIdSet), retrievedIds.length),
    requiredFactsFound: isFreshEnoughForSmoke ? countIntersection(gold.required_fact_ids, facts) : 0,
    requiredFactsTotal: gold.required_fact_ids.length,
  };
}

function goldForFixture(gold, fixture) {
  const key = fixture.gold_ref ?? fixture.id;
  const expectation = gold.fixtures?.[key];
  if (!expectation) {
    throw new Error(`missing sealed gold for fixture ${fixture.id}`);
  }
  return expectation;
}

function retrieveFixtureDocuments(fixture, documents) {
  const topK = fixture.top_k ?? 2;
  const queryTerms = new Set(fixture.query_terms);

  return Array.from(documents.values())
    .filter((document) => document.subject_id === fixture.subject_id)
    .map((document) => ({
      document,
      score: document.tags.filter((tag) => queryTerms.has(tag)).length,
    }))
    .filter((candidate) => candidate.score > 0)
    .sort((left, right) => {
      if (right.score !== left.score) {
        return right.score - left.score;
      }
      const rightDate = Date.parse(right.document.source_asof);
      const leftDate = Date.parse(left.document.source_asof);
      if (rightDate !== leftDate) {
        return rightDate - leftDate;
      }
      return left.document.id.localeCompare(right.document.id);
    })
    .slice(0, topK)
    .map((candidate) => candidate.document);
}

function buildMetrics(evaluation) {
  return METRIC_DEFINITIONS.map((definition) => {
    const value = evaluation.metricValues[definition.name];
    const threshold = THRESHOLDS[definition.name];
    const passed = evaluateThreshold(value, threshold.operator, threshold.value);

    return {
      namespace: definition.namespace,
      name: definition.name,
      value,
      unit: definition.unit,
      status: passed ? "pass" : "fail",
    };
  });
}

function buildArtifactPaths(recordPath, corpusManifestHash, mode) {
  return [
    {
      path: recordPath,
      kind: "evidence-record",
      privacy_class: mode === "published" ? "public" : "internal",
      publishable: mode === "published",
      redaction_status: "synthetic",
    },
    {
      path: MANIFEST_PATH,
      kind: "corpus-manifest",
      sha256: corpusManifestHash,
      privacy_class: "public",
      publishable: true,
      redaction_status: "synthetic",
    },
  ];
}

function collectInputHashes(repoRoot) {
  const hashes = {};
  for (const [key, relPath] of [
    ["evaluation_evidence_schema", SCHEMA_PATH],
    ["corpus_manifest", MANIFEST_PATH],
    ["sealed_gold", GOLD_PATH],
    ["fixture_manifest", MANIFEST_PATH],
    ["adapter_script", SCRIPT_PATH],
    ["eval_bridge_fixture", HARNESS_FIXTURE_PATH],
  ]) {
    const resolved = path.join(repoRoot, relPath);
    if (fs.existsSync(resolved) && fs.statSync(resolved).isFile()) {
      hashes[key] = hashFile(resolved);
    } else if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
      hashes[key] = hashDirectory(resolved);
    }
  }
  return hashes;
}

function runEvalBridgeSmoke(repoRoot) {
  const args = [
    "test",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--features",
    "release-gate",
    "--test",
    "harness",
    "runner_invokes_bundle_fixture_through_eval_bridge_and_captures_output",
    "--",
    "--exact",
  ];
  const command = `cargo ${args.map(shellQuote).join(" ")}`;
  try {
    const output = execFileSync("cargo", args, {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    if (
      !/\brunning 1 test\b/.test(output) ||
      !/runner_invokes_bundle_fixture_through_eval_bridge_and_captures_output/.test(output)
    ) {
      throw new Error("EvalAbilityBridge smoke fixture did not execute exactly one target test");
    }
    return {
      command,
      fixture: HARNESS_FIXTURE_PATH,
      passed: true,
    };
  } catch (error) {
    const stderr = error.stderr ? String(error.stderr).trim() : "";
    throw new Error(`EvalAbilityBridge smoke fixture failed: ${stderr || error.message}`);
  }
}

function skippedHarnessSmoke() {
  return {
    command: "not run for blocked published mode",
    fixture: HARNESS_FIXTURE_PATH,
    passed: false,
  };
}

function validateAndLintRecord(repoRoot, recordPath, artifactPaths) {
  execFileSync("node", ["scripts/validate-evidence-record.mjs", recordPath], {
    cwd: repoRoot,
    stdio: "inherit",
  });
  const lintTargets = [recordPath, ...artifactPaths.map((artifact) => artifact.path)];
  execFileSync("node", ["scripts/lint-evidence-artifacts.mjs", ...lintTargets], {
    cwd: repoRoot,
    stdio: "inherit",
  });
}

function createRunId(mode, date) {
  const timestamp = date.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const suffix = crypto.randomBytes(4).toString("hex");
  return `abilities-${mode}-${timestamp}-${process.pid}-${suffix}`;
}

function gitInfo(repoRoot) {
  const commit = commandOrNull(repoRoot, "git", ["rev-parse", "HEAD"]) ?? "unknown";
  const currentBranch = commandOrNull(repoRoot, "git", ["branch", "--show-current"]);
  const abbrevBranch = commandOrNull(repoRoot, "git", ["rev-parse", "--abbrev-ref", "HEAD"]);
  const status = commandOrNull(repoRoot, "git", ["status", "--porcelain", "--untracked-files=all"]);

  return {
    commit,
    branch: currentBranch || abbrevBranch || "unknown",
    dirty: status === null ? true : status.length > 0,
  };
}

function commandOrNull(cwd, command, args) {
  try {
    return execFileSync(command, args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

function findRepoRoot() {
  const gitRoot = commandOrNull(process.cwd(), "git", ["rev-parse", "--show-toplevel"]);
  return gitRoot ? path.resolve(gitRoot) : process.cwd();
}

function renderCommand(outRel, options, mode) {
  const parts = ["node", SCRIPT_PATH];
  parts.push("--mode", options.mode ?? mode);
  if (options.scope) {
    parts.push("--scope", options.scope);
  }
  if (options.runId) {
    parts.push("--run-id", options.runId);
  }
  if (options.out) {
    parts.push("--out", outRel);
  }
  return parts.map(shellQuote).join(" ");
}

function shellQuote(value) {
  if (/^[A-Za-z0-9_./:=@-]+$/.test(value)) {
    return value;
  }
  return `'${value.replaceAll("'", "'\\''")}'`;
}

function toRepoRelative(repoRoot, targetPath) {
  const relative = path.relative(repoRoot, targetPath);
  if (relative === "" || relative.startsWith("..") || path.isAbsolute(relative)) {
    throw new Error("output path must be inside the repository");
  }
  return relative.split(path.sep).join("/");
}

function hashFile(filePath) {
  return hashString(fs.readFileSync(filePath));
}

function hashDirectory(dirPath) {
  const files = [];
  collectDirectoryFiles(dirPath, dirPath, files);
  const hash = crypto.createHash("sha256");
  for (const file of files.sort()) {
    hash.update(file.relative);
    hash.update("\0");
    hash.update(fs.readFileSync(file.absolute));
    hash.update("\0");
  }
  return `sha256:${hash.digest("hex")}`;
}

function collectDirectoryFiles(root, current, files) {
  for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
    const fullPath = path.join(current, entry.name);
    if (entry.isDirectory()) {
      collectDirectoryFiles(root, fullPath, files);
    } else if (entry.isFile()) {
      files.push({
        absolute: fullPath,
        relative: path.relative(root, fullPath).split(path.sep).join("/"),
      });
    }
  }
}

function hashString(value) {
  return `sha256:${crypto.createHash("sha256").update(value).digest("hex")}`;
}

function countIntersection(values, expectedSet) {
  return values.filter((value) => expectedSet.has(value)).length;
}

function sum(values) {
  return values.reduce((total, value) => total + value, 0);
}

function average(values) {
  return divide(sum(values), values.length);
}

function divide(numerator, denominator) {
  return denominator === 0 ? 0 : Number((numerator / denominator).toFixed(6));
}

function daysBetween(sourceDate, evaluationAsOf) {
  const source = new Date(`${sourceDate}T00:00:00Z`);
  return Math.floor((evaluationAsOf.getTime() - source.getTime()) / 86_400_000);
}

function evaluateThreshold(value, operator, thresholdValue) {
  switch (operator) {
    case "eq":
      return value === thresholdValue;
    case "neq":
      return value !== thresholdValue;
    case "lt":
      return value < thresholdValue;
    case "lte":
      return value <= thresholdValue;
    case "gt":
      return value > thresholdValue;
    case "gte":
      return value >= thresholdValue;
    default:
      return false;
  }
}
