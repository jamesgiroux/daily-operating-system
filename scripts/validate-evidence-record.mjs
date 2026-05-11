#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const defaultTargets = [".docs/evals/examples/evidence-record-smoke.example.json"];

const requiredFields = [
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

const allowedFields = new Set([...requiredFields, "extensions"]);

const suites = new Set([
  "suite_p",
  "suite_s",
  "suite_e",
  "abilities_eval",
  "gbrain_comparison",
  "release_gate",
  "custom",
]);
const modes = new Set(["smoke", "published", "manual", "ci", "dry_run"]);
const results = new Set(["pass", "fail", "blocked", "not_run"]);
const privacyClasses = new Set(["public", "internal", "private", "restricted"]);
const redactionStatuses = new Set(["synthetic", "redacted", "reviewed", "blocked"]);
const metricNamespaces = new Set([
  "performance_regression",
  "retrieval",
  "answer_quality",
  "provenance_quality",
  "temporal_correctness",
  "trust_band_correctness",
  "surface_safety",
  "public_comparison",
]);
const metricDefinitionFields = new Set(["namespace", "name", "description", "unit", "higher_is_better"]);
const metricFields = new Set(["namespace", "name", "value", "unit", "status"]);
const artifactFields = new Set(["path", "kind", "sha256", "privacy_class", "publishable", "redaction_status"]);
const evidenceCountFields = new Set(["record_count", "fixture_count", "bench_count", "sample_count"]);
const thresholdFields = new Set(["operator", "value"]);
const thresholdOperators = new Set(["eq", "neq", "lt", "lte", "gt", "gte"]);
const sha256Pattern = /^sha256:[0-9a-f]{64}$/;
const rfc3339Pattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;
const blockedExtensionKeys = new Set([
  "fixture_payload",
  "judge_transcript",
  "llm_output",
  "message",
  "messages",
  "model_output",
  "model_outputs",
  "payload",
  "private_fixture_payload",
  "prompt",
  "prompts",
  "raw_judge_transcript",
  "raw_model_output",
  "raw_output",
  "raw_payload",
  "raw_transcript",
  "transcript",
  "unreviewed_judge_transcript",
]);
const blockedExtensionKeySegments = new Set([
  "message",
  "messages",
  "payload",
  "payloads",
  "prompt",
  "prompts",
  "transcript",
  "transcripts",
]);

const targets = collectTargets(process.argv.slice(2));
let failures = 0;

if (targets.length === 0) {
  console.error("FAIL no JSON evidence records found");
  process.exit(1);
}

for (const target of targets) {
  const errors = validateFile(target);
  if (errors.length > 0) {
    failures += 1;
    console.error(`FAIL ${target}`);
    for (const error of errors) {
      console.error(`  - ${error}`);
    }
  } else {
    console.log(`PASS ${target}`);
  }
}

process.exit(failures === 0 ? 0 : 1);

function collectTargets(args) {
  const filteredArgs = args.filter((arg) => arg !== "--");
  const inputs = filteredArgs.length === 0 ? defaultTargets : filteredArgs;
  const files = [];

  for (const input of inputs) {
    const resolved = path.resolve(repoRoot, input);
    if (!fs.existsSync(resolved)) {
      files.push(input);
      continue;
    }

    const stat = fs.statSync(resolved);
    if (stat.isDirectory()) {
      for (const file of walkJson(resolved)) {
        files.push(path.relative(repoRoot, file));
      }
    } else {
      files.push(path.relative(repoRoot, resolved));
    }
  }

  return files;
}

function* walkJson(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walkJson(fullPath);
    } else if (entry.isFile() && entry.name.endsWith(".json")) {
      yield fullPath;
    }
  }
}

function validateFile(target) {
  const errors = [];
  const resolved = path.resolve(repoRoot, target);

  if (!fs.existsSync(resolved)) {
    return [`file does not exist: ${target}`];
  }

  let record;
  try {
    record = JSON.parse(fs.readFileSync(resolved, "utf8"));
  } catch (error) {
    return [`invalid JSON: ${error.message}`];
  }

  for (const field of requiredFields) {
    if (!(field in record)) {
      errors.push(`missing required field: ${field}`);
    }
  }
  for (const field of Object.keys(record)) {
    if (!allowedFields.has(field)) {
      errors.push(`unknown top-level field: ${field}`);
    }
  }

  if (record.schema_version !== "evaluation_evidence_record_v1") {
    errors.push("schema_version must be evaluation_evidence_record_v1");
  }
  requireEnum(errors, "suite", record.suite, suites);
  requireEnum(errors, "mode", record.mode, modes);
  requireEnum(errors, "result", record.result, results);
  requireEnum(errors, "privacy_class", record.privacy_class, privacyClasses);
  requireEnum(errors, "redaction_status", record.redaction_status, redactionStatuses);
  requireString(errors, "lane", record.lane);
  requireString(errors, "scope", record.scope);
  requireString(errors, "run_id", record.run_id);
  requireString(errors, "commit", record.commit);
  requireString(errors, "branch", record.branch);
  requireString(errors, "command", record.command);
  requireString(errors, "dataset_source", record.dataset_source);
  requireString(errors, "dataset_license", record.dataset_license);
  requireString(errors, "notes", record.notes);
  requireHash(errors, "dataset_hash", record.dataset_hash);
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

  validateEnvironment(errors, record.environment);
  validateInputHashes(errors, record.input_hashes);
  validateThresholds(errors, record.thresholds, record.metrics, record.result);
  validateMetricDefinitions(errors, record.metric_definitions);
  validateMetrics(errors, record.metrics);
  validateArtifacts(errors, record.artifact_paths);
  validateMetricBindings(errors, record.metric_definitions, record.metrics);
  validateExtensions(errors, record.extensions, record);
  validatePublicationRules(errors, record);
  validateSerializedPrivacy(errors, record);

  return errors;
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

function requireHash(errors, field, value) {
  if (typeof value !== "string" || !sha256Pattern.test(value)) {
    errors.push(`${field} must be sha256:<64 lowercase hex chars>`);
  }
}

function requireBoolean(errors, field, value) {
  if (typeof value !== "boolean") {
    errors.push(`${field} must be a boolean`);
  }
}

function requireDate(errors, field, value) {
  if (
    typeof value !== "string" ||
    !rfc3339Pattern.test(value) ||
    Number.isNaN(Date.parse(value))
  ) {
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
    if (typeof definition !== "object" || definition === null || Array.isArray(definition)) {
      errors.push(`metric_definitions[${index}] must be an object`);
      return;
    }
    rejectUnknownFields(errors, `metric_definitions[${index}]`, definition, metricDefinitionFields);
    requireEnum(errors, `metric_definitions[${index}].namespace`, definition.namespace, metricNamespaces);
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
    if (typeof metric !== "object" || metric === null || Array.isArray(metric)) {
      errors.push(`metrics[${index}] must be an object`);
      return;
    }
    rejectUnknownFields(errors, `metrics[${index}]`, metric, metricFields);
    requireEnum(errors, `metrics[${index}].namespace`, metric.namespace, metricNamespaces);
    requireString(errors, `metrics[${index}].name`, metric.name);
    requireString(errors, `metrics[${index}].unit`, metric.unit);
    requireEnum(errors, `metrics[${index}].status`, metric.status, new Set(["pass", "fail", "warn", "info", "not_applicable"]));
    if (!("value" in metric)) {
      errors.push(`metrics[${index}].value is required`);
    } else if (!isMetricValue(metric.value)) {
      errors.push(`metrics[${index}].value must be number, boolean, string, or null`);
    }
  });
}

function validateArtifacts(errors, artifacts) {
  if (!Array.isArray(artifacts)) {
    return;
  }
  artifacts.forEach((artifact, index) => {
    if (typeof artifact !== "object" || artifact === null || Array.isArray(artifact)) {
      errors.push(`artifact_paths[${index}] must be an object`);
      return;
    }
    rejectUnknownFields(errors, `artifact_paths[${index}]`, artifact, artifactFields);
    requireString(errors, `artifact_paths[${index}].path`, artifact.path);
    requireString(errors, `artifact_paths[${index}].kind`, artifact.kind);
    if ("sha256" in artifact) {
      requireHash(errors, `artifact_paths[${index}].sha256`, artifact.sha256);
    }
    requireEnum(errors, `artifact_paths[${index}].privacy_class`, artifact.privacy_class, privacyClasses);
    requireBoolean(errors, `artifact_paths[${index}].publishable`, artifact.publishable);
    requireEnum(errors, `artifact_paths[${index}].redaction_status`, artifact.redaction_status, redactionStatuses);

    if (typeof artifact.path === "string") {
      validateRepoRelativePath(errors, `artifact_paths[${index}].path`, artifact.path);
    }
    if (artifact.publishable && artifact.privacy_class !== "public") {
      errors.push(`artifact_paths[${index}] is publishable but not public`);
    }
    if (artifact.publishable && artifact.redaction_status === "blocked") {
      errors.push(`artifact_paths[${index}] is publishable but redaction_status is blocked`);
    }
  });
}

function validateEnvironment(errors, environment) {
  if (!isPlainObject(environment)) {
    return;
  }

  for (const [key, value] of Object.entries(environment)) {
    if (!isScalarOrNull(value)) {
      errors.push(`environment.${key} must be string, number, boolean, or null`);
    }
  }
}

function validateInputHashes(errors, inputHashes) {
  if (!isPlainObject(inputHashes)) {
    return;
  }

  for (const [key, value] of Object.entries(inputHashes)) {
    requireHash(errors, `input_hashes.${key}`, value);
  }
}

function validateThresholds(errors, thresholds, metrics, result) {
  if (!isPlainObject(thresholds)) {
    return;
  }

  if (
    "requires_baseline" in thresholds &&
    typeof thresholds.requires_baseline !== "boolean"
  ) {
    errors.push("thresholds.requires_baseline must be a boolean");
  }

  const metricsByName = new Map();
  if (Array.isArray(metrics)) {
    for (const metric of metrics) {
      if (isPlainObject(metric) && typeof metric.name === "string") {
        metricsByName.set(metric.name, metric);
      }
    }
  }

  if (result === "pass" && Array.isArray(metrics)) {
    metrics.forEach((metric, index) => {
      if (isPlainObject(metric) && metric.status === "fail") {
        errors.push(`result pass is inconsistent with metrics[${index}].status fail`);
      }
    });
  }

  for (const [name, threshold] of Object.entries(thresholds)) {
    if (name === "requires_baseline") {
      continue;
    }
    if (!isPlainObject(threshold)) {
      errors.push(`thresholds.${name} must be an object`);
      continue;
    }
    rejectUnknownFields(errors, `thresholds.${name}`, threshold, thresholdFields);
    requireEnum(errors, `thresholds.${name}.operator`, threshold.operator, thresholdOperators);
    if (!("value" in threshold)) {
      errors.push(`thresholds.${name}.value is required`);
    }
    const metric = metricsByName.get(name);
    if (!metric) {
      errors.push(`thresholds.${name} has no matching metric`);
      continue;
    }
    if (
      result === "pass" &&
      "value" in threshold &&
      thresholdOperators.has(threshold.operator) &&
      !evaluateThreshold(metric.value, threshold.operator, threshold.value)
    ) {
      errors.push(`result pass is inconsistent with thresholds.${name}`);
    }
  }
}

function validateExtensions(errors, extensions, record) {
  if (extensions === undefined) {
    return;
  }
  if (!isPlainObject(extensions)) {
    errors.push("extensions must be an object");
    return;
  }

  validateEvidenceCounts(errors, extensions.evidence_counts);
  validateBaselineBinding(errors, extensions.baseline_binding);
  if (record.publishable || record.mode === "published") {
    validatePublishedExtensions(errors, extensions);
  }
}

function validateEvidenceCounts(errors, counts) {
  if (counts === undefined) {
    return;
  }
  if (!isPlainObject(counts)) {
    errors.push("extensions.evidence_counts must be an object");
    return;
  }

  for (const [key, value] of Object.entries(counts)) {
    if (!evidenceCountFields.has(key)) {
      errors.push(`extensions.evidence_counts contains unknown field: ${key}`);
      continue;
    }
    if (!Number.isInteger(value) || value < 0) {
      errors.push(`extensions.evidence_counts.${key} must be a non-negative integer`);
    }
  }
}

function validateBaselineBinding(errors, baselineBinding) {
  if (baselineBinding === undefined) {
    return;
  }
  if (!isPlainObject(baselineBinding)) {
    errors.push("extensions.baseline_binding must be an object");
    return;
  }

  for (const [key, value] of Object.entries(baselineBinding)) {
    requireHash(errors, `extensions.baseline_binding.${key}`, value);
  }
}

function validateSerializedPrivacy(errors, record) {
  const serialized = JSON.stringify(record);
  const forbidden = {
    "file://": /\bfile:\/\//i,
    "fixture_identity_map": /\bfixture_identity_map\b/i,
    "identity_map": /\bidentity_map\b/i,
    "local users path": /(?:^|[^A-Za-z0-9])(?:\/Users\/|[A-Za-z]:\\Users\\|Users[\\/])/,
    "windows absolute path": /(?:^|[^A-Za-z0-9])[A-Za-z]:\\[^\s"'<>),}\]]+/i,
    "local home path": /(?:^|[^A-Za-z0-9])\/home\/[A-Za-z0-9._-]+(?:\/|$)/,
    "private var path": /(?:^|[^A-Za-z0-9])\/(?:private\/var|var\/folders)(?:\/|$)/,
    "private DailyOS cache path": /(?:^|[^A-Za-z0-9._-])\.dailyos(?:[\\/]|$)/i,
    "private path token": /(?:^|[^A-Za-z0-9._-])(?:id_rsa|id_ed25519|\.env(?:\.[A-Za-z0-9_-]+)?|\.netrc|\.npmrc|\.pypirc|\.git-credentials|credentials\.json|secrets\.json|\.cargo[\\/]credentials(?:\.toml)?|\.aws[\\/]credentials|\.ssh[\\/]config)(?=$|[^A-Za-z0-9._-])/i,
  };

  for (const [label, pattern] of Object.entries(forbidden)) {
    if (pattern.test(serialized)) {
      errors.push(`record contains forbidden private token: ${label}`);
    }
  }
}

function validatePublicationRules(errors, record) {
  if (record.publishable && record.privacy_class !== "public") {
    errors.push("publishable records must have privacy_class public");
  }
  if (record.publishable && record.redaction_status === "blocked") {
    errors.push("publishable records cannot have redaction_status blocked");
  }
  if (record.mode === "published") {
    if (!record.metrics || record.metrics.length === 0) {
      errors.push("published records must include at least one metric");
    }
    if (!record.artifact_paths || record.artifact_paths.length === 0) {
      errors.push("published records must include at least one artifact path");
    }
    if (!record.input_hashes || Object.keys(record.input_hashes).length === 0) {
      errors.push("published records must include input hashes");
    }
    if (typeof record.dataset_hash !== "string" || record.dataset_hash.trim() === "") {
      errors.push("published records must include dataset_hash");
    }
    requireHash(errors, "dataset_hash", record.dataset_hash);
    validatePublishedInputHashes(errors, record);
    validatePublishedEvidenceCounts(errors, record);
    validatePublishedBaselineBinding(errors, record);
    validatePublishedRedaction(errors, record);
  }
}

function validatePublishedInputHashes(errors, record) {
  if (!isPlainObject(record.input_hashes)) {
    return;
  }

  if (!hasAnyKey(record.input_hashes, ["schema", "evaluation_evidence_schema"])) {
    errors.push("published records must bind the evidence schema hash");
  }

  if (record.suite === "suite_p") {
    if (!hasAnyKey(record.input_hashes, ["bench_manifest", "bench_config"])) {
      errors.push("published Suite P records must bind a bench manifest/config hash");
    }
    return;
  }

  if (record.suite === "abilities_eval" || record.suite === "gbrain_comparison") {
    if (!hasAnyKey(record.input_hashes, ["fixture_manifest", "corpus_manifest", "dataset"])) {
      errors.push("published eval/comparison records must bind a fixture, corpus, or dataset hash");
    }
  }
}

function validatePublishedEvidenceCounts(errors, record) {
  const counts = record.extensions?.evidence_counts;
  if (!isPlainObject(counts)) {
    errors.push("published records must include extensions.evidence_counts");
    return;
  }

  if (record.suite === "suite_p") {
    if (!isPositiveInteger(counts.bench_count)) {
      errors.push("published Suite P records must include evidence_counts.bench_count > 0");
    }
    return;
  }

  if (record.suite === "abilities_eval" || record.suite === "gbrain_comparison") {
    const fixtureCount = isPositiveInteger(counts.fixture_count);
    const sampleCount = isPositiveInteger(counts.sample_count);
    if (!fixtureCount && !sampleCount) {
      errors.push("published eval/comparison records must include fixture_count or sample_count > 0");
    }
    return;
  }

  const totalCount = isPositiveInteger(counts.record_count);
  const fixtureCount = isPositiveInteger(counts.fixture_count);
  const benchCount = isPositiveInteger(counts.bench_count);
  const sampleCount = isPositiveInteger(counts.sample_count);
  if (!totalCount && !fixtureCount && !benchCount && !sampleCount) {
    errors.push("published records must include at least one positive evidence count");
  }
}

function validatePublishedBaselineBinding(errors, record) {
  const requiresBaseline = record.thresholds?.requires_baseline === true;
  if (!requiresBaseline) {
    return;
  }

  const baselineBinding = record.extensions?.baseline_binding;
  const inputHashBound = isPlainObject(record.input_hashes) && hasAnyKey(record.input_hashes, ["baseline", "prior_baseline"]);
  const extensionBound = isPlainObject(baselineBinding) && typeof baselineBinding.baseline_hash === "string" && baselineBinding.baseline_hash.trim() !== "";
  if (!inputHashBound && !extensionBound) {
    errors.push("published records that require a baseline must bind baseline or prior_baseline hash");
  }
}

function validatePublishedRedaction(errors, record) {
  if (record.redaction_status === "blocked" && record.result === "pass") {
    errors.push("published passing records cannot have redaction_status blocked");
  }
  if (Array.isArray(record.artifact_paths)) {
    record.artifact_paths.forEach((artifact, index) => {
      if (
        isPlainObject(artifact) &&
        artifact.redaction_status === "blocked" &&
        record.result === "pass"
      ) {
        errors.push(`published passing records cannot include blocked artifact_paths[${index}]`);
      }
    });
  }
}

function validateMetricBindings(errors, definitions, metrics) {
  if (!Array.isArray(definitions) || !Array.isArray(metrics)) {
    return;
  }
  const definitionKeys = new Set(
    definitions
      .filter((definition) => isPlainObject(definition))
      .map((definition) => `${definition.namespace}:${definition.name}`),
  );
  metrics.forEach((metric, index) => {
    if (!isPlainObject(metric)) {
      return;
    }
    const key = `${metric.namespace}:${metric.name}`;
    if (!definitionKeys.has(key)) {
      errors.push(`metrics[${index}] has no matching metric definition`);
    }
  });
}

function validatePublishedExtensions(errors, value, pathLabel = "extensions") {
  if (!isPlainObject(value) && !Array.isArray(value)) {
    return;
  }

  for (const [key, child] of Object.entries(value)) {
    const normalizedKey = normalizeKey(key);
    if (isBlockedExtensionKey(normalizedKey)) {
      errors.push(`${pathLabel}.${key} is not allowed in publishable evidence`);
    }
    if (isPlainObject(child) || Array.isArray(child)) {
      validatePublishedExtensions(errors, child, `${pathLabel}.${key}`);
    }
  }
}

function rejectUnknownFields(errors, pathLabel, object, allowed) {
  for (const field of Object.keys(object)) {
    if (!allowed.has(field)) {
      errors.push(`${pathLabel} contains unknown field: ${field}`);
    }
  }
}

function isMetricValue(value) {
  return (
    value === null ||
    typeof value === "number" ||
    typeof value === "boolean" ||
    typeof value === "string"
  );
}

function validateRepoRelativePath(errors, field, value) {
  const normalized = value.replaceAll("\\", "/");
  if (
    path.isAbsolute(value) ||
    path.win32.isAbsolute(value) ||
    /^[A-Za-z]:/.test(value) ||
    value.startsWith("\\") ||
    value.startsWith("//") ||
    value.startsWith("~")
  ) {
    errors.push(`${field} must be repo-relative`);
  }
  if (/^file:\/\//i.test(value)) {
    errors.push(`${field} must not use file://`);
  }
  if (normalized.split("/").includes("..")) {
    errors.push(`${field} must not escape with ..`);
  }
  if (hasPrivatePathSegment(normalized)) {
    errors.push(`${field} contains a forbidden local/private path segment`);
  }
}

function hasPrivatePathSegment(value) {
  const segments = value.split("/").filter(Boolean);
  if (segments.includes("Users") || segments.includes("fixture_identity_map") || segments.includes("identity_map")) {
    return true;
  }
  if (segments[0] === "home" && segments.length > 1) {
    return true;
  }
  if ((segments[0] === "private" && segments[1] === "var") || (segments[0] === "var" && segments[1] === "folders")) {
    return true;
  }
  if (segments.includes(".dailyos")) {
    return true;
  }
  return segments.some((segment, index) => {
    if (
      /^(id_rsa|id_ed25519|\.netrc|\.npmrc|\.pypirc|\.git-credentials|credentials\.json|secrets\.json)$/i.test(segment) ||
      /^\.env(?:\.[A-Za-z0-9_-]+)?$/i.test(segment)
    ) {
      return true;
    }
    const previous = segments[index - 1];
    return (
      (previous === ".ssh" && segment === "config") ||
      (previous === ".aws" && segment === "credentials") ||
      (previous === ".cargo" && /^credentials(?:\.toml)?$/i.test(segment))
    );
  });
}

function evaluateThreshold(metricValue, operator, thresholdValue) {
  switch (operator) {
    case "eq":
      return metricValue === thresholdValue;
    case "neq":
      return metricValue !== thresholdValue;
    case "lt":
    case "lte":
    case "gt":
    case "gte":
      if (typeof metricValue !== "number" || typeof thresholdValue !== "number") {
        return false;
      }
      if (operator === "lt") {
        return metricValue < thresholdValue;
      }
      if (operator === "lte") {
        return metricValue <= thresholdValue;
      }
      if (operator === "gt") {
        return metricValue > thresholdValue;
      }
      return metricValue >= thresholdValue;
    default:
      return false;
  }
}

function normalizeKey(key) {
  return key
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toLowerCase();
}

function isBlockedExtensionKey(normalizedKey) {
  if (blockedExtensionKeys.has(normalizedKey)) {
    return true;
  }
  const segments = normalizedKey.split("_").filter(Boolean);
  if (segments.some((segment) => blockedExtensionKeySegments.has(segment))) {
    return true;
  }
  if (/^(raw|unreviewed)(?:_|$)/.test(normalizedKey)) {
    return true;
  }
  return /(?:^|_)(?:raw|judge|llm|model)(?:_.*)?_outputs?$/.test(normalizedKey);
}

function isScalarOrNull(value) {
  return (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  );
}

function hasAnyKey(object, keys) {
  return keys.some((key) => typeof object[key] === "string" && object[key].trim() !== "");
}

function isPositiveInteger(value) {
  return Number.isInteger(value) && value > 0;
}

function isPlainObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
