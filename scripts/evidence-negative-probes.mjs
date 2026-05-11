#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = findRepoRoot();
const baseRecordPath = path.join(
  repoRoot,
  ".docs/evals/examples/evidence-record-smoke.example.json",
);
const stage8aRecordPath = path.join(
  repoRoot,
  ".docs/evals/examples/evidence-record-stage8a-smoke.example.json",
);
const schemaPath = path.join(repoRoot, ".docs/evals/evidence-record.schema.json");
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "dailyos-evidence-probes-"));
const baseRecord = JSON.parse(fs.readFileSync(baseRecordPath, "utf8"));
const stage8aRecord = JSON.parse(fs.readFileSync(stage8aRecordPath, "utf8"));
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));

const probes = [
  {
    name: "unknown top-level field",
    mutate(record) {
      record.unexpected = true;
    },
  },
  {
    name: "extra metric definition field",
    mutate(record) {
      record.metric_definitions[0].unexpected = true;
    },
  },
  {
    name: "wrong metric definition boolean type",
    mutate(record) {
      record.metric_definitions[0].higher_is_better = "true";
    },
  },
  {
    name: "object metric value",
    mutate(record) {
      record.metrics[0].value = { bad: true };
    },
  },
  {
    name: "array metric value",
    mutate(record) {
      record.metrics[0].value = ["bad"];
    },
  },
  {
    name: "extra metric field",
    mutate(record) {
      record.metrics[0].unexpected = true;
    },
  },
  {
    name: "invalid metric namespace enum",
    mutate(record) {
      record.metrics[0].namespace = "combined_score";
    },
  },
  {
    name: "invalid top-level enum",
    mutate(record) {
      record.mode = "release";
    },
  },
  {
    name: "invalid metric status enum",
    mutate(record) {
      record.metrics[0].status = "ok";
    },
  },
  {
    name: "invalid artifact privacy enum",
    mutate(record) {
      record.artifact_paths[0].privacy_class = "external";
    },
  },
  {
    name: "extra artifact field",
    mutate(record) {
      record.artifact_paths[0].unexpected = true;
    },
  },
  {
    name: "wrong artifact hash type",
    mutate(record) {
      record.artifact_paths[0].sha256 = 42;
    },
  },
  {
    name: "absolute artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "/Users/example/private-record.json";
    },
  },
  {
    name: "windows absolute artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "C:\\Users\\Example\\private-record.json";
    },
  },
  {
    name: "home directory artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "home/example/private-record.json";
    },
  },
  {
    name: "private var artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "private/var/folders/private-record.json";
    },
  },
  {
    name: "home artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "~/.dailyos/private-record.json";
    },
  },
  {
    name: "file URL artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "file:///tmp/private-record.json";
    },
  },
  {
    name: "escaping artifact path",
    mutate(record) {
      record.artifact_paths[0].path = "../private-record.json";
    },
  },
  {
    name: "identity-map artifact path",
    mutate(record) {
      record.artifact_paths[0].path = ".docs/evals/fixture_identity_map/private.json";
    },
  },
  {
    name: "ssh config artifact path",
    mutate(record) {
      record.artifact_paths[0].path = ".ssh/config";
    },
  },
  {
    name: "publishable non-public artifact",
    mutate(record) {
      record.artifact_paths[0].privacy_class = "internal";
    },
  },
  {
    name: "publishable blocked artifact",
    mutate(record) {
      record.artifact_paths[0].redaction_status = "blocked";
    },
  },
  {
    name: "publishable non-public record",
    mutate(record) {
      record.privacy_class = "internal";
    },
  },
  {
    name: "wrong input hash type",
    mutate(record) {
      record.input_hashes.schema = { bad: true };
    },
  },
  {
    name: "malformed input hash",
    mutate(record) {
      record.input_hashes.schema = "not-a-hash";
    },
  },
  {
    name: "malformed dataset hash",
    mutate(record) {
      record.dataset_hash = "not-a-hash";
    },
  },
  {
    name: "malformed artifact hash",
    mutate(record) {
      record.artifact_paths[0].sha256 = "not-a-hash";
    },
  },
  {
    name: "wrong environment value type",
    mutate(record) {
      record.environment.node = { version: "bad" };
    },
  },
  {
    name: "wrong requires_baseline type",
    mutate(record) {
      record.thresholds.requires_baseline = "true";
    },
  },
  {
    name: "wrong evidence count type",
    base: publishedSuitePRecord,
    mutate(record) {
      record.extensions.evidence_counts.bench_count = "1";
    },
  },
  {
    name: "wrong baseline binding shape",
    base: publishedSuitePRecord,
    mutate(record) {
      record.extensions.baseline_binding = "sha256:baseline";
    },
  },
  {
    name: "malformed baseline binding hash",
    base: publishedSuitePRecord,
    mutate(record) {
      record.extensions.baseline_binding.baseline_hash = "not-a-hash";
    },
  },
  {
    name: "missing published evidence counts",
    base: publishedSuitePRecord,
    mutate(record) {
      delete record.extensions.evidence_counts;
    },
  },
  {
    name: "generic published zero evidence count",
    mutate(record) {
      record.mode = "published";
      record.suite = "custom";
      record.input_hashes = {
        schema: exampleHash("1"),
      };
      record.extensions = {
        evidence_counts: {
          record_count: 0,
        },
      };
    },
  },
  {
    name: "missing published input hashes",
    base: publishedSuitePRecord,
    mutate(record) {
      record.input_hashes = {};
    },
  },
  {
    name: "missing dataset binding",
    base: publishedSuitePRecord,
    mutate(record) {
      record.dataset_hash = "";
    },
  },
  {
    name: "wrong dataset binding type",
    mutate(record) {
      record.dataset_source = 42;
    },
  },
  {
    name: "wrong notes type",
    mutate(record) {
      record.notes = 42;
    },
  },
  {
    name: "missing Suite P bench manifest/config hash",
    base: publishedSuitePRecord,
    mutate(record) {
      delete record.input_hashes.bench_manifest;
      delete record.input_hashes.bench_config;
    },
  },
  {
    name: "missing required baseline binding",
    base: publishedSuitePRecord,
    mutate(record) {
      delete record.input_hashes.baseline;
      delete record.extensions.baseline_binding;
    },
  },
  {
    name: "non-positive published evidence count",
    base: publishedSuitePRecord,
    mutate(record) {
      record.extensions.evidence_counts.bench_count = 0;
    },
  },
  {
    name: "missing abilities fixture hash",
    base: publishedAbilitiesRecord,
    mutate(record) {
      record.input_hashes = {
        schema: exampleHash("1"),
      };
    },
  },
  {
    name: "raw judge transcript extension",
    mutate(record) {
      record.extensions = {
        raw_judge_transcript: "private prompt and model output",
      };
    },
  },
  {
    name: "plural raw model output extension",
    mutate(record) {
      record.extensions = {
        raw_model_outputs: "private prompt and model output",
      };
    },
  },
  {
    name: "camel raw model output extension",
    mutate(record) {
      record.extensions = {
        rawModelOutputs: "private prompt and model output",
      };
    },
  },
  {
    name: "nested raw judge transcript extension",
    mutate(record) {
      record.extensions = {
        adapter: {
          raw_judge_transcript: "private prompt and model output",
        },
      };
    },
  },
  {
    name: "failed metric with passing result",
    schemaReject: false,
    mutate(record) {
      record.metrics[0].status = "fail";
      record.result = "pass";
    },
  },
  {
    name: "false threshold with passing result",
    schemaReject: false,
    mutate(record) {
      record.metrics[0].value = false;
      record.thresholds.schema_validation.value = true;
      record.result = "pass";
    },
  },
  {
    name: "threshold without matching metric",
    schemaReject: false,
    mutate(record) {
      record.thresholds.missing_metric = {
        operator: "eq",
        value: true,
      };
    },
  },
  {
    name: "unknown threshold operator",
    mutate(record) {
      record.thresholds.schema_validation.operator = "approximately";
    },
  },
  {
    name: "date-only timestamp",
    mutate(record) {
      record.started_at = "2026-05-09";
    },
  },
  {
    name: "private token in record metadata",
    schemaReject: false,
    mutate(record) {
      record.notes = "private token .env.local leaked";
    },
  },
  {
    name: "windows absolute path in record metadata",
    schemaReject: false,
    mutate(record) {
      record.notes = "leaked local path D:\\build\\out.json";
    },
  },
];

const lintProbes = [
  {
    name: "non-example email privacy lint",
    content: "contact: user@customer.invalid\n",
  },
  {
    name: "phone-like number privacy lint",
    content: "phone: 555-123-4567\n",
  },
  {
    name: "redacted artifact privacy lint",
    content: "value: REDACTED\n",
  },
  {
    name: "local users path privacy lint",
    content: "path: /Users/example/private.json\n",
  },
  {
    name: "windows users path privacy lint",
    content: "path: C:\\Users\\Example\\private.json\n",
  },
  {
    name: "windows absolute path privacy lint",
    content: "path: D:\\build\\out.json\n",
  },
  {
    name: "home private path privacy lint",
    content: "path: ~/.ssh/config\n",
  },
  {
    name: "private path token privacy lint",
    content: "path: .env.local\n",
  },
  {
    name: "dailyos cache privacy lint",
    content: "path: ~/.dailyos/cache.json\n",
  },
  {
    name: "fixture identity map privacy lint",
    content: "path: fixture_identity_map/private.json\n",
  },
  {
    name: "file URL privacy lint",
    content: "path: file:///tmp/private.json\n",
  },
];

let failures = 0;

for (const [name, record] of [
  ["base example schema parity", baseRecord],
  ["stage8a example schema parity", stage8aRecord],
]) {
  const schemaErrors = validateAgainstSchema(record, schema);
  const file = path.join(tempDir, `${slug(name)}.json`);
  fs.writeFileSync(file, `${JSON.stringify(record, null, 2)}\n`);
  const result = run("node", ["scripts/validate-evidence-record.mjs", file]);
  if (schemaErrors.length > 0 || result.status !== 0) {
    failures += 1;
    console.error(`FAIL ${name}`);
    if (schemaErrors.length > 0) {
      console.error(`  schema unexpectedly rejected: ${schemaErrors.slice(0, 5).join("; ")}`);
    }
    printCommandOutput(result);
  } else {
    console.log(`PASS ${name}`);
  }
}

for (const probe of probes) {
  const record = cloneRecord(probe.base ? probe.base() : baseRecord);
  probe.mutate(record);
  const file = path.join(tempDir, `${slug(probe.name)}.json`);
  fs.writeFileSync(file, `${JSON.stringify(record, null, 2)}\n`);

  const result = run("node", ["scripts/validate-evidence-record.mjs", file]);
  const schemaErrors = validateAgainstSchema(record, schema);
  const expectSchemaReject = probe.schemaReject !== false;
  if (result.status === 0 || (expectSchemaReject && schemaErrors.length === 0)) {
    failures += 1;
    console.error(`FAIL ${probe.name}`);
    if (result.status === 0) {
      console.error("  malformed record unexpectedly passed validation");
    }
    if (expectSchemaReject && schemaErrors.length === 0) {
      console.error("  malformed record unexpectedly passed schema validation");
    }
  } else {
    console.log(`PASS ${probe.name}`);
  }
}

for (const probe of lintProbes) {
  const root = path.join(tempDir, slug(probe.name));
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(path.join(root, "artifact.txt"), probe.content);

  const result = run("node", ["scripts/lint-evidence-artifacts.mjs", root]);
  if (result.status === 0) {
    failures += 1;
    console.error(`FAIL ${probe.name}`);
    console.error("  privacy lint unexpectedly passed");
  } else {
    console.log(`PASS ${probe.name}`);
  }
}

{
  const root = path.join(tempDir, "binary-privacy-lint");
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(path.join(root, "artifact.bin"), Buffer.from([0x73, 0x00, 0x79]));
  const result = run("node", ["scripts/lint-evidence-artifacts.mjs", root]);
  if (result.status === 0) {
    failures += 1;
    console.error("FAIL binary privacy lint");
    console.error("  privacy lint unexpectedly passed");
  } else {
    console.log("PASS binary privacy lint");
  }
}

{
  const root = path.join(tempDir, "symlink-privacy-lint");
  fs.mkdirSync(root, { recursive: true });
  const target = path.join(root, "target.txt");
  const link = path.join(root, "linked.txt");
  fs.writeFileSync(target, "synthetic\n");
  try {
    fs.symlinkSync(target, link);
    const result = run("node", ["scripts/lint-evidence-artifacts.mjs", root]);
    if (result.status === 0) {
      failures += 1;
      console.error("FAIL symlink privacy lint");
      console.error("  privacy lint unexpectedly passed");
    } else {
      console.log("PASS symlink privacy lint");
    }
  } catch (error) {
    failures += 1;
    console.error("FAIL symlink privacy lint");
    console.error(`  could not create symlink probe: ${error.message}`);
  }
}

{
  const target = path.join(tempDir, "root-symlink-target");
  const link = path.join(tempDir, "root-symlink-link");
  fs.mkdirSync(target, { recursive: true });
  fs.writeFileSync(path.join(target, "artifact.txt"), "synthetic\n");
  try {
    fs.symlinkSync(target, link, "dir");
    const result = run("node", ["scripts/lint-evidence-artifacts.mjs", link]);
    if (result.status === 0) {
      failures += 1;
      console.error("FAIL root symlink privacy lint");
      console.error("  privacy lint unexpectedly passed");
    } else {
      console.log("PASS root symlink privacy lint");
    }
  } catch (error) {
    failures += 1;
    console.error("FAIL root symlink privacy lint");
    console.error(`  could not create root symlink probe: ${error.message}`);
  }
}

for (const parity of [
  {
    name: "evidence:validate pnpm separator parity honors target",
    command: ["evidence:validate", "--", path.join(tempDir, "validate-parity")],
    prepare() {
      const root = path.join(tempDir, "validate-parity");
      fs.mkdirSync(root, { recursive: true });
      const record = cloneRecord(baseRecord);
      record.metrics[0].value = { bad: true };
      fs.writeFileSync(path.join(root, "malformed.json"), `${JSON.stringify(record, null, 2)}\n`);
    },
    expectStatus: "fail",
  },
  {
    name: "evidence:lint pnpm separator parity honors target",
    command: ["evidence:lint", "--", path.join(tempDir, "lint-parity")],
    prepare() {
      const root = path.join(tempDir, "lint-parity");
      fs.mkdirSync(root, { recursive: true });
      fs.writeFileSync(path.join(root, "artifact.txt"), "path: /Users/example/private.json\n");
    },
    expectStatus: "fail",
  },
  {
    name: "evidence:contract-smoke pnpm separator parity honors out",
    command: [
      "evidence:contract-smoke",
      "--",
      "--out",
      "src-tauri/target/evidence/contract/negative-probes/contract-smoke-parity.json",
      "--run-id",
      "negative-probes-contract-smoke-parity",
      "--scope",
      "v1.4.1-W8",
    ],
    expectStatus: "pass",
    verify() {
      return fs.existsSync(
        path.join(
          repoRoot,
          "src-tauri/target/evidence/contract/negative-probes/contract-smoke-parity.json",
        ),
      );
    },
  },
]) {
  parity.prepare?.();
  const result = run("pnpm", parity.command);
  const passed =
    parity.expectStatus === "fail"
      ? result.status !== 0
      : result.status === 0 && (parity.verify ? parity.verify() : true);
  if (!passed) {
    failures += 1;
    console.error(`FAIL ${parity.name}`);
    printCommandOutput(result);
  } else {
    console.log(`PASS ${parity.name}`);
  }
}

fs.rmSync(tempDir, { recursive: true, force: true });
process.exit(failures === 0 ? 0 : 1);

function publishedSuitePRecord() {
  const record = cloneRecord(baseRecord);
  record.suite = "suite_p";
  record.lane = "DOS-348";
  record.mode = "published";
  record.command = "pnpm suite:p -- --mode published --scope v1.4.1-W8";
  record.metric_definitions = [
    {
      namespace: "performance_regression",
      name: "p95_ms",
      description: "Example Suite P p95 latency",
      unit: "ms",
      higher_is_better: false,
    },
  ];
  record.metrics = [
    {
      namespace: "performance_regression",
      name: "p95_ms",
      value: 900,
      unit: "ms",
      status: "pass",
    },
  ];
  record.input_hashes = {
    schema: exampleHash("1"),
    bench_manifest: exampleHash("2"),
    bench_config: exampleHash("3"),
    baseline: exampleHash("4"),
  };
  record.thresholds = {
    requires_baseline: true,
    p95_ms: {
      operator: "lte",
      value: 1000,
    },
  };
  record.artifact_paths = [
    {
      path: ".docs/perf/runs/example-suite-p-published/record.json",
      kind: "evidence-record",
      sha256: exampleHash("5"),
      privacy_class: "public",
      publishable: true,
      redaction_status: "synthetic",
    },
  ];
  record.extensions = {
    evidence_counts: {
      bench_count: 1,
    },
    baseline_binding: {
      baseline_hash: exampleHash("4"),
    },
  };
  return record;
}

function publishedAbilitiesRecord() {
  const record = cloneRecord(baseRecord);
  record.suite = "abilities_eval";
  record.lane = "DOS-261";
  record.mode = "published";
  record.command = "pnpm eval:abilities -- --mode published --scope v1.4.1-W8";
  record.input_hashes = {
    schema: exampleHash("1"),
    fixture_manifest: exampleHash("6"),
  };
  record.artifact_paths = [
    {
      path: ".docs/evals/runs/example-abilities-published/record.json",
      kind: "evidence-record",
      sha256: exampleHash("7"),
      privacy_class: "public",
      publishable: true,
      redaction_status: "synthetic",
    },
  ];
  record.extensions = {
    evidence_counts: {
      fixture_count: 1,
    },
  };
  return record;
}

function run(command, args) {
  return spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function printCommandOutput(result) {
  if (result.stdout.trim()) {
    console.error(result.stdout.trim());
  }
  if (result.stderr.trim()) {
    console.error(result.stderr.trim());
  }
}

function cloneRecord(record) {
  return JSON.parse(JSON.stringify(record));
}

function slug(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

function exampleHash(nibble) {
  return `sha256:${nibble.repeat(64)}`;
}

function validateAgainstSchema(value, schemaDocument) {
  return validateSchemaNode(schemaDocument, value, schemaDocument, "$");
}

function validateSchemaNode(schemaNode, value, rootSchema, pathLabel) {
  if (schemaNode === true) {
    return [];
  }
  if (schemaNode === false) {
    return [`${pathLabel} is not allowed`];
  }
  if (!schemaNode || typeof schemaNode !== "object") {
    return [];
  }

  if (schemaNode.$ref) {
    return validateSchemaNode(resolveRef(schemaNode.$ref, rootSchema), value, rootSchema, pathLabel);
  }

  const errors = [];

  if (Array.isArray(schemaNode.allOf)) {
    for (const child of schemaNode.allOf) {
      errors.push(...validateSchemaNode(child, value, rootSchema, pathLabel));
    }
  }
  if (Array.isArray(schemaNode.anyOf)) {
    const matched = schemaNode.anyOf.some(
      (child) => validateSchemaNode(child, value, rootSchema, pathLabel).length === 0,
    );
    if (!matched) {
      errors.push(`${pathLabel} must match at least one allowed schema`);
    }
  }
  if (schemaNode.if) {
    const conditionMatched = validateSchemaNode(schemaNode.if, value, rootSchema, pathLabel).length === 0;
    if (conditionMatched && schemaNode.then) {
      errors.push(...validateSchemaNode(schemaNode.then, value, rootSchema, pathLabel));
    }
  }
  if (schemaNode.not) {
    const matched = validateSchemaNode(schemaNode.not, value, rootSchema, pathLabel).length === 0;
    if (matched) {
      errors.push(`${pathLabel} must not match forbidden schema`);
    }
  }

  if (schemaNode.const !== undefined && value !== schemaNode.const) {
    errors.push(`${pathLabel} must equal ${JSON.stringify(schemaNode.const)}`);
  }
  if (schemaNode.enum && !schemaNode.enum.includes(value)) {
    errors.push(`${pathLabel} must be one of ${schemaNode.enum.join(", ")}`);
  }
  if (schemaNode.type && !matchesSchemaType(value, schemaNode.type)) {
    errors.push(`${pathLabel} must be ${Array.isArray(schemaNode.type) ? schemaNode.type.join("|") : schemaNode.type}`);
    return errors;
  }

  if (typeof value === "string") {
    if (schemaNode.minLength !== undefined && value.length < schemaNode.minLength) {
      errors.push(`${pathLabel} must have length >= ${schemaNode.minLength}`);
    }
    if (schemaNode.pattern && !new RegExp(schemaNode.pattern).test(value)) {
      errors.push(`${pathLabel} must match ${schemaNode.pattern}`);
    }
    if (schemaNode.format === "date-time" && Number.isNaN(Date.parse(value))) {
      errors.push(`${pathLabel} must be date-time`);
    }
  }

  if (typeof value === "number" && schemaNode.minimum !== undefined && value < schemaNode.minimum) {
    errors.push(`${pathLabel} must be >= ${schemaNode.minimum}`);
  }

  if (Array.isArray(value)) {
    if (schemaNode.minItems !== undefined && value.length < schemaNode.minItems) {
      errors.push(`${pathLabel} must have at least ${schemaNode.minItems} items`);
    }
    if (schemaNode.items) {
      value.forEach((item, index) => {
        errors.push(...validateSchemaNode(schemaNode.items, item, rootSchema, `${pathLabel}[${index}]`));
      });
    }
  }

  if (isObject(value)) {
    const properties = schemaNode.properties ?? {};
    if (schemaNode.required) {
      for (const field of schemaNode.required) {
        if (!(field in value)) {
          errors.push(`${pathLabel}.${field} is required`);
        }
      }
    }
    if (schemaNode.minProperties !== undefined && Object.keys(value).length < schemaNode.minProperties) {
      errors.push(`${pathLabel} must have at least ${schemaNode.minProperties} properties`);
    }
    if (schemaNode.propertyNames) {
      for (const key of Object.keys(value)) {
        errors.push(...validateSchemaNode(schemaNode.propertyNames, key, rootSchema, `${pathLabel}.{${key}}`));
      }
    }
    for (const [field, childSchema] of Object.entries(properties)) {
      if (field in value) {
        errors.push(...validateSchemaNode(childSchema, value[field], rootSchema, `${pathLabel}.${field}`));
      }
    }
    if (schemaNode.additionalProperties === false) {
      for (const field of Object.keys(value)) {
        if (!(field in properties)) {
          errors.push(`${pathLabel}.${field} is not allowed`);
        }
      }
    } else if (isObject(schemaNode.additionalProperties)) {
      for (const [field, child] of Object.entries(value)) {
        if (!(field in properties)) {
          errors.push(
            ...validateSchemaNode(
              schemaNode.additionalProperties,
              child,
              rootSchema,
              `${pathLabel}.${field}`,
            ),
          );
        }
      }
    }
  }

  return errors;
}

function resolveRef(ref, rootSchema) {
  if (!ref.startsWith("#/")) {
    throw new Error(`unsupported schema ref: ${ref}`);
  }
  return ref
    .slice(2)
    .split("/")
    .reduce((node, segment) => node[segment], rootSchema);
}

function matchesSchemaType(value, type) {
  if (Array.isArray(type)) {
    return type.some((item) => matchesSchemaType(value, item));
  }
  if (type === "array") {
    return Array.isArray(value);
  }
  if (type === "object") {
    return isObject(value);
  }
  if (type === "integer") {
    return Number.isInteger(value);
  }
  if (type === "number") {
    return typeof value === "number" && Number.isFinite(value);
  }
  if (type === "null") {
    return value === null;
  }
  return typeof value === type;
}

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function findRepoRoot() {
  const result = spawnSync("git", ["rev-parse", "--show-toplevel"], {
    cwd: process.cwd(),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  return result.status === 0 ? path.resolve(result.stdout.trim()) : process.cwd();
}
