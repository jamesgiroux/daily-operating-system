#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";

const SCHEMA_VERSION = "evaluation_evidence_record_v1";
const RUNNER_VERSION = "stage8a_contract_smoke_v1";
const DEFAULT_SCOPE = "v1.4.1-W8";
const LANE = "DOS-503";
const SUITE = "custom";
const MODE = "smoke";
const RESULT = "pass";
const METRIC_NAMESPACE = "surface_safety";
const METRIC_NAME = "contract_schema_validation";
const SCRIPT_PATH = "scripts/evidence-contract-smoke.mjs";
const SCHEMA_PATH = ".docs/evals/evidence-record.schema.json";
const BASE_EXAMPLE_PATH = ".docs/evals/examples/evidence-record-smoke.example.json";
const STAGE8A_EXAMPLE_PATH = ".docs/evals/examples/evidence-record-stage8a-smoke.example.json";

const REQUIRED_FIELDS = [
  "schema_version",
  "suite",
  "lane",
  "mode",
  "scope",
  "run_id",
  "commit",
  "branch",
  "dirty",
  "command",
  "started_at",
  "finished_at",
  "environment",
  "input_hashes",
  "metric_definitions",
  "metrics",
  "thresholds",
  "result",
  "artifact_paths",
  "privacy_class",
  "publishable",
  "dataset_source",
  "dataset_license",
  "dataset_hash",
  "redaction_status",
  "notes",
];

const ENUMS = {
  suite: new Set([
    "suite_p",
    "suite_s",
    "suite_e",
    "abilities_eval",
    "gbrain_comparison",
    "release_gate",
    "custom",
  ]),
  mode: new Set(["smoke", "published", "manual", "ci", "dry_run"]),
  result: new Set(["pass", "fail", "blocked", "not_run"]),
  privacy_class: new Set(["public", "internal", "private", "restricted"]),
  redaction_status: new Set(["synthetic", "redacted", "reviewed", "blocked"]),
  metric_namespace: new Set([
    "performance_regression",
    "retrieval",
    "answer_quality",
    "provenance_quality",
    "temporal_correctness",
    "trust_band_correctness",
    "surface_safety",
    "public_comparison",
  ]),
  metric_status: new Set(["pass", "fail", "warn", "info", "not_applicable"]),
};

main();

function main() {
  const repoRoot = findRepoRoot();
  const startedAt = new Date();
  const options = parseArgs(process.argv.slice(2));

  if (options.help) {
    printUsage();
    return;
  }

  const runId = options.runId ?? createRunId(startedAt);
  const scope = options.scope ?? DEFAULT_SCOPE;
  const outPath = path.resolve(
    repoRoot,
    options.out ?? `src-tauri/target/evidence/contract/${runId}/record.json`,
  );
  const outRel = toRepoRelative(repoRoot, outPath);

  const git = gitInfo(repoRoot);
  const inputHashes = collectInputHashes(repoRoot);
  const finishedAt = new Date();

  const record = {
    schema_version: SCHEMA_VERSION,
    suite: SUITE,
    lane: LANE,
    mode: MODE,
    scope,
    run_id: runId,
    commit: git.commit,
    branch: git.branch,
    dirty: git.dirty,
    command: renderCommand(repoRoot, outRel, options),
    started_at: startedAt.toISOString(),
    finished_at: finishedAt.toISOString(),
    environment: {
      os: os.platform(),
      arch: os.arch(),
      node_version: process.version,
      pnpm_version: commandOrNull(repoRoot, "pnpm", ["--version"]),
      runner_version: RUNNER_VERSION,
    },
    input_hashes: inputHashes,
    metric_definitions: [
      {
        namespace: METRIC_NAMESPACE,
        name: METRIC_NAME,
        description:
          "Evidence record contains the required Evaluation Evidence Contract fields, enums, and privacy-safe artifact paths.",
        unit: "boolean",
        higher_is_better: true,
      },
    ],
    metrics: [
      {
        namespace: METRIC_NAMESPACE,
        name: METRIC_NAME,
        value: true,
        unit: "boolean",
        status: "pass",
      },
    ],
    thresholds: {
      [METRIC_NAME]: {
        operator: "eq",
        value: true,
      },
    },
    result: RESULT,
    artifact_paths: [
      {
        path: outRel,
        kind: "evidence-record",
        privacy_class: "public",
        publishable: true,
        redaction_status: "synthetic",
      },
    ],
    privacy_class: "public",
    publishable: true,
    dataset_source: "synthetic-contract-smoke",
    dataset_license: "DailyOS synthetic example",
    dataset_hash: hashString(stableJson(inputHashes)),
    redaction_status: "synthetic",
    notes:
      "Synthetic Stage 8a smoke record for the Evaluation Evidence Contract; no customer data or private fixture payloads are used.",
    extensions: {
      contract_validation: {
        validator: SCRIPT_PATH,
        required_fields_checked: REQUIRED_FIELDS.length,
        enum_sets_checked: Object.keys(ENUMS).length,
        input_paths: inputPaths(repoRoot),
      },
    },
  };

  const validationErrors = validateRecord(record);
  record.extensions.contract_validation.errors = validationErrors;

  if (validationErrors.length > 0) {
    record.metrics[0].value = false;
    record.metrics[0].status = "fail";
    record.result = "fail";
  }

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(record, null, 2)}\n`);
  console.log(outRel);

  if (validationErrors.length > 0) {
    for (const error of validationErrors) {
      console.error(`contract validation failed: ${error}`);
    }
    process.exit(1);
  }
}

function parseArgs(args) {
  const options = {};

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];

    if (arg === "--help" || arg === "-h") {
      options.help = true;
    } else if (arg === "--") {
      continue;
    } else if (arg === "--out") {
      index += 1;
      if (!args[index]) {
        throw new Error("--out requires a path");
      }
      options.out = args[index];
    } else if (arg.startsWith("--out=")) {
      options.out = arg.slice("--out=".length);
    } else if (arg === "--run-id") {
      index += 1;
      if (!args[index]) {
        throw new Error("--run-id requires a value");
      }
      options.runId = args[index];
    } else if (arg.startsWith("--run-id=")) {
      options.runId = arg.slice("--run-id=".length);
    } else if (arg === "--scope") {
      index += 1;
      if (!args[index]) {
        throw new Error("--scope requires a value");
      }
      options.scope = args[index];
    } else if (arg.startsWith("--scope=")) {
      options.scope = arg.slice("--scope=".length);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }

  return options;
}

function printUsage() {
  console.log(`Usage: node ${SCRIPT_PATH} [--out <path>] [--run-id <id>] [--scope <scope>]`);
}

function createRunId(date) {
  const timestamp = date.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
  const suffix = crypto.randomBytes(4).toString("hex");
  return `contract-smoke-${timestamp}-${process.pid}-${suffix}`;
}

function findRepoRoot() {
  const gitRoot = commandOrNull(process.cwd(), "git", ["rev-parse", "--show-toplevel"]);
  return gitRoot ? path.resolve(gitRoot) : process.cwd();
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

function collectInputHashes(repoRoot) {
  const hashes = {};
  for (const [key, relPath] of [
    ["evaluation_evidence_schema", SCHEMA_PATH],
    ["base_smoke_example_record", BASE_EXAMPLE_PATH],
    ["stage8a_smoke_example_record", STAGE8A_EXAMPLE_PATH],
    ["contract_smoke_emitter", SCRIPT_PATH],
  ]) {
    const resolved = path.join(repoRoot, relPath);
    if (fs.existsSync(resolved)) {
      hashes[key] = hashFile(resolved);
    }
  }

  return hashes;
}

function inputPaths(repoRoot) {
  return [SCHEMA_PATH, BASE_EXAMPLE_PATH, STAGE8A_EXAMPLE_PATH, SCRIPT_PATH].filter((relPath) =>
    fs.existsSync(path.join(repoRoot, relPath)),
  );
}

function hashFile(filePath) {
  return hashString(fs.readFileSync(filePath));
}

function hashString(value) {
  return `sha256:${crypto.createHash("sha256").update(value).digest("hex")}`;
}

function stableJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function renderCommand(repoRoot, outRel, options) {
  const parts = ["node", SCRIPT_PATH];
  if (options.out) {
    parts.push("--out", outRel);
  }
  if (options.runId) {
    parts.push("--run-id", options.runId);
  }
  if (options.scope) {
    parts.push("--scope", options.scope);
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
    throw new Error(`output path must be inside the repository: ${targetPath}`);
  }
  return relative.split(path.sep).join("/");
}

function validateRecord(record) {
  const errors = [];

  for (const field of REQUIRED_FIELDS) {
    if (!(field in record)) {
      errors.push(`missing required field: ${field}`);
    }
  }

  requireEqual(errors, "schema_version", record.schema_version, SCHEMA_VERSION);
  requireEnum(errors, "suite", record.suite, ENUMS.suite);
  requireEnum(errors, "mode", record.mode, ENUMS.mode);
  requireEnum(errors, "result", record.result, ENUMS.result);
  requireEnum(errors, "privacy_class", record.privacy_class, ENUMS.privacy_class);
  requireEnum(errors, "redaction_status", record.redaction_status, ENUMS.redaction_status);
  requireString(errors, "lane", record.lane);
  requireString(errors, "scope", record.scope);
  requireString(errors, "run_id", record.run_id);
  requireString(errors, "commit", record.commit);
  requireString(errors, "branch", record.branch);
  requireString(errors, "command", record.command);
  requireString(errors, "dataset_source", record.dataset_source);
  requireString(errors, "dataset_license", record.dataset_license);
  requireString(errors, "dataset_hash", record.dataset_hash);
  requireBoolean(errors, "dirty", record.dirty);
  requireBoolean(errors, "publishable", record.publishable);
  requireDate(errors, "started_at", record.started_at);
  requireDate(errors, "finished_at", record.finished_at);
  requireObject(errors, "environment", record.environment);
  requireObject(errors, "input_hashes", record.input_hashes);
  requireObject(errors, "thresholds", record.thresholds);
  requireArray(errors, "metric_definitions", record.metric_definitions);
  requireArray(errors, "metrics", record.metrics);
  requireArray(errors, "artifact_paths", record.artifact_paths);

  validateMetricDefinitions(errors, record.metric_definitions);
  validateMetrics(errors, record.metrics);
  validateArtifacts(errors, record.artifact_paths);
  validatePrivacy(errors, record);

  return errors;
}

function requireEqual(errors, field, value, expected) {
  if (value !== expected) {
    errors.push(`${field} must be ${expected}`);
  }
}

function requireEnum(errors, field, value, allowed) {
  if (!allowed.has(value)) {
    errors.push(`${field} must be one of: ${Array.from(allowed).join(", ")}`);
  }
}

function requireString(errors, field, value) {
  if (typeof value !== "string" || value.trim() === "") {
    errors.push(`${field} must be a non-empty string`);
  }
}

function requireBoolean(errors, field, value) {
  if (typeof value !== "boolean") {
    errors.push(`${field} must be a boolean`);
  }
}

function requireDate(errors, field, value) {
  if (typeof value !== "string" || Number.isNaN(Date.parse(value))) {
    errors.push(`${field} must be a valid RFC3339 timestamp`);
  }
}

function requireObject(errors, field, value) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    errors.push(`${field} must be an object`);
  }
}

function requireArray(errors, field, value) {
  if (!Array.isArray(value)) {
    errors.push(`${field} must be an array`);
  }
}

function validateMetricDefinitions(errors, definitions) {
  if (!Array.isArray(definitions)) {
    return;
  }

  definitions.forEach((definition, index) => {
    if (!isPlainObject(definition)) {
      errors.push(`metric_definitions[${index}] must be an object`);
      return;
    }
    requireEnum(errors, `metric_definitions[${index}].namespace`, definition.namespace, ENUMS.metric_namespace);
    requireString(errors, `metric_definitions[${index}].name`, definition.name);
    requireString(errors, `metric_definitions[${index}].description`, definition.description);
    requireString(errors, `metric_definitions[${index}].unit`, definition.unit);
    requireBoolean(errors, `metric_definitions[${index}].higher_is_better`, definition.higher_is_better);
  });
}

function validateMetrics(errors, metrics) {
  if (!Array.isArray(metrics)) {
    return;
  }

  metrics.forEach((metric, index) => {
    if (!isPlainObject(metric)) {
      errors.push(`metrics[${index}] must be an object`);
      return;
    }
    requireEnum(errors, `metrics[${index}].namespace`, metric.namespace, ENUMS.metric_namespace);
    requireString(errors, `metrics[${index}].name`, metric.name);
    requireString(errors, `metrics[${index}].unit`, metric.unit);
    requireEnum(errors, `metrics[${index}].status`, metric.status, ENUMS.metric_status);
    if (!("value" in metric)) {
      errors.push(`metrics[${index}].value is required`);
    }
  });
}

function validateArtifacts(errors, artifacts) {
  if (!Array.isArray(artifacts)) {
    return;
  }

  artifacts.forEach((artifact, index) => {
    if (!isPlainObject(artifact)) {
      errors.push(`artifact_paths[${index}] must be an object`);
      return;
    }
    requireString(errors, `artifact_paths[${index}].path`, artifact.path);
    requireString(errors, `artifact_paths[${index}].kind`, artifact.kind);
    requireEnum(errors, `artifact_paths[${index}].privacy_class`, artifact.privacy_class, ENUMS.privacy_class);
    requireBoolean(errors, `artifact_paths[${index}].publishable`, artifact.publishable);
    requireEnum(errors, `artifact_paths[${index}].redaction_status`, artifact.redaction_status, ENUMS.redaction_status);

    if (typeof artifact.path === "string") {
      if (path.isAbsolute(artifact.path) || artifact.path.startsWith("~")) {
        errors.push(`artifact_paths[${index}].path must be repo-relative`);
      }
      if (artifact.path.startsWith("file://")) {
        errors.push(`artifact_paths[${index}].path must not use file://`);
      }
      if (artifact.path.split(/[\\/]+/).includes("..")) {
        errors.push(`artifact_paths[${index}].path must not escape with ..`);
      }
    }
  });
}

function isPlainObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validatePrivacy(errors, record) {
  const serialized = JSON.stringify(record);
  for (const token of ["/Users/", "file://", "fixture_identity_map", "identity_map"]) {
    if (serialized.includes(token)) {
      errors.push(`record contains forbidden private token: ${token}`);
    }
  }
  if (record.publishable && record.privacy_class !== "public") {
    errors.push("publishable records must have privacy_class public");
  }
}
