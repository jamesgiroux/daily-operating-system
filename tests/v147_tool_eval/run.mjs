#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const DEFAULT_MANIFEST = "tests/v147_tool_eval/fixtures/tool-selection-manifest.json";
const DEFAULT_REPORT = "src-tauri/target/v147_tool_eval/report.json";

main();

function main() {
  try {
    const repoRoot = findRepoRoot();
    const args = parseArgs(process.argv.slice(2));
    if (args.help) {
      printUsage();
      return;
    }

    const manifestPath = path.resolve(repoRoot, args.manifest ?? DEFAULT_MANIFEST);
    const manifest = readJson(manifestPath);
    const catalogPath = path.resolve(repoRoot, manifest.catalog_path);
    const catalog = readJson(catalogPath);
    const competitorToolsPath = manifest.competitor_tools_path
      ? path.resolve(repoRoot, manifest.competitor_tools_path)
      : null;
    const competitorTools = competitorToolsPath ? readJson(competitorToolsPath) : null;
    const report = evaluateCatalog({
      manifest,
      catalog,
      competitorTools,
      catalogPath: toRepoRelative(repoRoot, catalogPath),
      manifestPath: toRepoRelative(repoRoot, manifestPath),
      competitorToolsPath: competitorToolsPath ? toRepoRelative(repoRoot, competitorToolsPath) : null,
    });

    const outPath = path.resolve(repoRoot, args.out ?? DEFAULT_REPORT);
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);

    const summary = [
      `tools=${report.summary.tool_count}`,
      `fixtures=${report.summary.fixture_count}`,
      `positive=${formatRate(report.metrics.positive_selection_pass_rate)}`,
      `broad_negative=${formatRate(report.metrics.broad_negative_pass_rate)}`,
      `adjacent_negative=${formatRate(report.metrics.adjacent_negative_pass_rate)}`,
      `density=${formatRate(report.metrics.fixture_density_pass_rate)}`,
      `arguments=${formatRate(report.metrics.argument_key_pass_rate)}`,
      `unambiguous=${formatRate(report.metrics.prompt_unambiguity_pass_rate)}`,
    ].join(" ");
    console.log(`v1.4.7 MCP tool-selection eval ${report.status}: ${summary}`);
    console.log(`report: ${toRepoRelative(repoRoot, outPath)}`);

    if (report.status !== "pass") {
      for (const failure of report.failures) {
        console.error(`FAIL ${failure.tool ?? "catalog"}: ${failure.message}`);
      }
      process.exit(1);
    }
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

function evaluateCatalog({ manifest, catalog, competitorTools, catalogPath, manifestPath, competitorToolsPath }) {
  assertManifest(manifest);
  assertCompetitorTools(competitorTools, competitorToolsPath);
  if (!Array.isArray(catalog)) {
    throw new Error(`${catalogPath} must contain a JSON array of tool descriptions`);
  }

  const toolNames = new Set(catalog.map((tool) => tool.name));
  const failures = [];
  const promptExpectations = new Map();
  let fixtureCount = 0;
  let positivePass = 0;
  let positiveTotal = 0;
  let broadPass = 0;
  let broadTotal = 0;
  let adjacentPass = 0;
  let adjacentTotal = 0;
  let densityPass = 0;
  let argumentKeyPass = 0;
  let argumentKeyTotal = 0;

  for (const tool of catalog) {
    const fixtures = tool.selectionFixtures;
    if (!fixtures) {
      failures.push({
        tool: tool.name,
        message: "missing selectionFixtures",
      });
      continue;
    }

    const positive = fixtures.positive ?? [];
    const broad = fixtures.negativeBroadCorpus ?? [];
    const adjacent = fixtures.negativeAdjacentTool ?? [];
    fixtureCount += positive.length + broad.length + adjacent.length;

    const densityOk =
      positive.length >= manifest.minimums.positive_per_tool &&
      broad.length >= manifest.minimums.broad_negative_per_tool &&
      adjacent.length >= manifest.minimums.adjacent_negative_per_tool;
    if (densityOk) {
      densityPass += 1;
    } else {
      failures.push({
        tool: tool.name,
        message:
          `fixture density failed: positive=${positive.length}, ` +
          `broad=${broad.length}, adjacent=${adjacent.length}`,
      });
    }

    for (const fixture of positive) {
      positiveTotal += 1;
      const ok = fixture.expectedTool === tool.name && fixture.expectedToolClass === undefined;
      if (ok) {
        positivePass += 1;
      } else {
        failures.push({
          tool: tool.name,
          message: `positive fixture should expect ${tool.name}: ${fixture.prompt}`,
        });
      }
      recordPromptExpectation(promptExpectations, failures, fixture.prompt, {
        kind: "tool",
        expected: fixture.expectedTool,
        owner: tool.name,
      });
      argumentKeyTotal += 1;
      if (validateExpectedArgumentKeys(tool, fixture, failures)) {
        argumentKeyPass += 1;
      }
    }

    for (const fixture of broad) {
      broadTotal += 1;
      const ok = fixture.expectedTool === undefined && fixture.expectedToolClass === "external";
      if (ok) {
        broadPass += 1;
      } else {
        failures.push({
          tool: tool.name,
          message: `broad negative must expect external only: ${fixture.prompt}`,
        });
      }
      recordPromptExpectation(promptExpectations, failures, fixture.prompt, {
        kind: "external",
        expected: "external",
        owner: tool.name,
      });
    }

    for (const fixture of adjacent) {
      adjacentTotal += 1;
      const ok = toolNames.has(fixture.expectedTool) && fixture.expectedTool !== tool.name;
      if (ok) {
        adjacentPass += 1;
      } else {
        failures.push({
          tool: tool.name,
          message:
            `adjacent negative must point at another catalog tool: ` +
            `${fixture.prompt} -> ${fixture.expectedTool}`,
        });
      }
      recordPromptExpectation(promptExpectations, failures, fixture.prompt, {
        kind: "tool",
        expected: fixture.expectedTool,
        owner: tool.name,
      });
    }
  }

  const metrics = {
    positive_selection_pass_rate: rate(positivePass, positiveTotal),
    broad_negative_pass_rate: rate(broadPass, broadTotal),
    adjacent_negative_pass_rate: rate(adjacentPass, adjacentTotal),
    fixture_density_pass_rate: rate(densityPass, catalog.length),
    argument_key_pass_rate: rate(argumentKeyPass, argumentKeyTotal),
    prompt_unambiguity_pass_rate: failures.some((failure) => failure.code === "ambiguous_prompt")
      ? 0
      : 1,
  };

  for (const [metric, threshold] of Object.entries(manifest.thresholds)) {
    if ((metrics[metric] ?? 0) < threshold) {
      failures.push({
        code: "threshold_failed",
        message: `${metric}=${metrics[metric] ?? 0} below threshold ${threshold}`,
      });
    }
  }

  return {
    schema_version: "v147_tool_selection_eval_report_v1",
    status: failures.length === 0 ? "pass" : "fail",
    generated_at: new Date().toISOString(),
    manifest_path: manifestPath,
    catalog_path: catalogPath,
    competitor_tools_path: competitorToolsPath,
    fixture_source: manifest.fixture_source,
    thresholds: manifest.thresholds,
    metrics,
    summary: {
      tool_count: catalog.length,
      fixture_count: fixtureCount,
      positive_count: positiveTotal,
      broad_negative_count: broadTotal,
      adjacent_negative_count: adjacentTotal,
    },
    failures,
  };
}

function validateExpectedArgumentKeys(tool, fixture, failures) {
  const parameters = new Set((tool.parameters ?? []).map((parameter) => parameter.name));
  const requiredParameters = new Set(
    (tool.parameters ?? [])
      .filter((parameter) => parameter.required)
      .map((parameter) => parameter.name),
  );
  const keys = fixture.expectedArgumentKeys ?? [];
  if (requiredParameters.size === 0) {
    if (keys.length === 0) {
      return true;
    }
    failures.push({
      tool: tool.name,
      message: `positive fixture declares arguments for parameterless tool: ${fixture.prompt}`,
    });
    return false;
  }

  if (keys.length === 0) {
    failures.push({
      tool: tool.name,
      message: `positive fixture missing expectedArgumentKeys: ${fixture.prompt}`,
    });
    return false;
  }

  const unknown = keys.filter((key) => !parameters.has(key));
  if (unknown.length > 0) {
    failures.push({
      tool: tool.name,
      message: `positive fixture has unknown expectedArgumentKeys ${unknown.join(", ")}: ${fixture.prompt}`,
    });
    return false;
  }

  const missingRequired = Array.from(requiredParameters).filter((key) => !keys.includes(key));
  if (missingRequired.length > 0) {
    failures.push({
      tool: tool.name,
      message:
        `positive fixture missing required expectedArgumentKeys ` +
        `${missingRequired.join(", ")}: ${fixture.prompt}`,
    });
    return false;
  }

  return true;
}

function recordPromptExpectation(map, failures, prompt, expectation) {
  if (typeof prompt !== "string" || prompt.trim() === "") {
    failures.push({
      code: "empty_prompt",
      tool: expectation.owner,
      message: "fixture prompt must be non-empty",
    });
    return;
  }

  const key = prompt.trim().toLowerCase();
  const previous = map.get(key);
  if (!previous) {
    map.set(key, expectation);
    return;
  }

  if (previous.kind !== expectation.kind || previous.expected !== expectation.expected) {
    failures.push({
      code: "ambiguous_prompt",
      tool: expectation.owner,
      message:
        `prompt has conflicting expectations: "${prompt}" ` +
        `(${previous.owner} -> ${previous.expected}, ${expectation.owner} -> ${expectation.expected})`,
    });
  }
}

function assertManifest(manifest) {
  for (const field of ["schema_version", "catalog_path", "fixture_source", "minimums", "thresholds"]) {
    if (!(field in manifest)) {
      throw new Error(`tool-selection manifest missing ${field}`);
    }
  }
  if (manifest.schema_version !== "v147_tool_selection_eval_manifest_v1") {
    throw new Error(`unsupported manifest schema: ${manifest.schema_version}`);
  }
}

function assertCompetitorTools(competitorTools, competitorToolsPath) {
  if (!competitorToolsPath) {
    return;
  }
  if (!competitorTools || !Array.isArray(competitorTools.tools)) {
    throw new Error(`${competitorToolsPath} must contain a tools array`);
  }
  if (!competitorTools.tools.some((tool) => tool.class === "external")) {
    throw new Error(`${competitorToolsPath} must include at least one external tool`);
  }
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function parseArgs(args) {
  const parsed = {};
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
    } else if (arg === "--manifest") {
      parsed.manifest = requireValue(args, ++i, arg);
    } else if (arg === "--out") {
      parsed.out = requireValue(args, ++i, arg);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function printUsage() {
  console.log(`Usage: node tests/v147_tool_eval/run.mjs [--manifest PATH] [--out PATH]`);
}

function findRepoRoot() {
  let current = process.cwd();
  while (current !== path.dirname(current)) {
    if (fs.existsSync(path.join(current, "package.json")) && fs.existsSync(path.join(current, ".git"))) {
      return current;
    }
    current = path.dirname(current);
  }
  throw new Error("could not find repo root");
}

function toRepoRelative(repoRoot, filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join("/");
}

function rate(pass, total) {
  return total === 0 ? 0 : pass / total;
}

function formatRate(value) {
  return `${Math.round(value * 1000) / 10}%`;
}
