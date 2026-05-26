#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const DEFAULT_REPORT = "src-tauri/target/v147_e2e/validation-report.json";

const AXES = [
  {
    id: "host-selection",
    description: "W5-A tool-description eval passes against the catalog fixture corpus.",
    commands: [
      {
        label: "v147-tool-selection-eval",
        argv: ["node", "tests/v147_tool_eval/run.mjs", "--out", "src-tauri/target/v147_e2e/tool-selection-report.json"],
      },
    ],
  },
  {
    id: "tool-inventory",
    description: "All registered MCP v2 handlers are catalog-backed and visible through local stdio grants.",
    commands: [
      {
        label: "registered-handlers-match-catalog",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "registers_workspace_placement_handler_from_catalog",
          "--lib",
        ],
      },
      {
        label: "local-stdio-tool-list",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "mcp_v2_tools_list_registered_handlers_only",
          "--lib",
        ],
      },
    ],
  },
  {
    id: "read-boundaries",
    description: "Read tools preserve runtime projection shape, presenter privacy, and workspace-memory redaction.",
    commands: [
      {
        label: "account-status-runtime-projection",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "account_status_exec_briefing_fixture_uses_runtime_projection",
          "--lib",
        ],
      },
      {
        label: "daily-briefing-presenter-shape",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "presenter_builds_readable_daily_briefing_answer",
          "--lib",
        ],
      },
      {
        label: "meeting-briefing-privacy",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "meeting_presenter_omits_attendee_email_and_raw_claim_ids",
          "--lib",
        ],
      },
      {
        label: "workspace-memory-search",
        argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "tool_workspace_search", "--lib"],
      },
      {
        label: "workspace-source-provenance",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "tool_workspace_source_provenance",
          "--lib",
        ],
      },
    ],
  },
  {
    id: "resource-privacy",
    description: "MCP resources use opaque handles and avoid raw entity ids, names, and claim text unless scoped.",
    commands: [
      {
        label: "mcp-resource-privacy",
        argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "tool_resources", "--lib"],
      },
    ],
  },
  {
    id: "write-boundaries",
    description: "Write/submit tools route through services and return cursor-only receipts.",
    commands: [
      {
        label: "submit-note-handler",
        argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "tool_note", "--lib"],
      },
      {
        label: "submit-note-service",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "create_user_note_claim_in_db",
          "--lib",
        ],
      },
      {
        label: "submit-action-handler",
        argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "tool_create_action", "--lib"],
      },
      {
        label: "submit-action-service",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "create_action_in_db_returns_receipt_and_preserves_mcp_attribution",
          "--lib",
        ],
      },
      {
        label: "submit-action-status-handler",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "tool_update_action_status",
          "--lib",
        ],
      },
      {
        label: "submit-action-status-service",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "submit_action_status_in_db",
          "--lib",
        ],
      },
      {
        label: "place-document-handler",
        argv: [
          "cargo",
          "test",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "placement_handler_commits_claim_and_graph_without_path_leak",
          "--lib",
        ],
      },
    ],
  },
  {
    id: "binary-smoke",
    description: "The headless MCP binary compiles with the mcp feature enabled.",
    commands: [
      {
        label: "dailyos-mcp-binary",
        argv: [
          "cargo",
          "check",
          "--manifest-path",
          "src-tauri/Cargo.toml",
          "--bin",
          "dailyos-mcp",
          "--features",
          "mcp",
        ],
      },
    ],
  },
];

main();

function main() {
  const repoRoot = findRepoRoot();
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    printUsage();
    return;
  }

  const selected = selectAxes(args.axis ?? "all");
  const startedAt = new Date();
  const axisResults = selected.map((axis) => runAxis(repoRoot, axis));
  const finishedAt = new Date();
  const status = axisResults.every((axis) => axis.status === "pass") ? "pass" : "fail";
  const report = {
    schema_version: "v147_mcp_e2e_validation_v1",
    status,
    generated_at: finishedAt.toISOString(),
    duration_ms: finishedAt.getTime() - startedAt.getTime(),
    git_sha: commandOrUnknown(repoRoot, "git", ["rev-parse", "HEAD"]),
    branch: commandOrUnknown(repoRoot, "git", ["branch", "--show-current"]),
    axes: axisResults,
    summary: {
      total_axes: axisResults.length,
      passed_axes: axisResults.filter((axis) => axis.status === "pass").length,
      failed_axes: axisResults.filter((axis) => axis.status !== "pass").length,
      total_commands: axisResults.reduce((sum, axis) => sum + axis.commands.length, 0),
    },
    privacy_note:
      "Report intentionally stores command labels and statuses only; payloads, prompts, entity names, paths, claim text, and source handles are omitted.",
  };

  const outPath = path.resolve(repoRoot, args.out ?? DEFAULT_REPORT);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);

  console.log(
    `v1.4.7 MCP e2e validation ${status}: axes=${report.summary.total_axes} commands=${report.summary.total_commands}`,
  );
  console.log(`report: ${toRepoRelative(repoRoot, outPath)}`);
  for (const axis of axisResults) {
    console.log(`axis ${axis.id}: ${axis.status}`);
  }

  if (status !== "pass") {
    process.exit(1);
  }
}

function runAxis(repoRoot, axis) {
  const startedAt = Date.now();
  const commands = axis.commands.map((command) => runCommand(repoRoot, command));
  return {
    id: axis.id,
    description: axis.description,
    status: commands.every((command) => command.status === "pass") ? "pass" : "fail",
    duration_ms: Date.now() - startedAt,
    commands,
  };
}

function runCommand(repoRoot, command) {
  const startedAt = Date.now();
  const [bin, ...args] = command.argv;
  const result = spawnSync(bin, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  return {
    label: command.label,
    status: result.status === 0 ? "pass" : "fail",
    exit_code: result.status ?? -1,
    duration_ms: Date.now() - startedAt,
    output_hash: stableHash(`${result.stdout ?? ""}\n${result.stderr ?? ""}`),
  };
}

function selectAxes(axis) {
  if (axis === "all") {
    return AXES;
  }
  const found = AXES.find((candidate) => candidate.id === axis);
  if (!found) {
    throw new Error(`unknown axis: ${axis}. Known axes: all, ${AXES.map((item) => item.id).join(", ")}`);
  }
  return [found];
}

function parseArgs(args) {
  const parsed = {};
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--help" || arg === "-h") {
      parsed.help = true;
    } else if (arg === "--axis") {
      parsed.axis = requireValue(args, ++i, arg);
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
  console.log(`Usage: node tests/v147_e2e/run.mjs [--axis all|host-selection|tool-inventory|read-boundaries|resource-privacy|write-boundaries|binary-smoke] [--out PATH]`);
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

function commandOrUnknown(cwd, bin, args) {
  const result = spawnSync(bin, args, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  return result.status === 0 ? result.stdout.trim() : "unknown";
}

function stableHash(value) {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i += 1) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}
