#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const DEFAULT_REPORT = "src-tauri/target/release-gate/v147-e2e.json";
const TOOL_EVAL_REPORT = "src-tauri/target/v147_tool_eval/report.json";
const RELEASE_GATE_REPORT = "src-tauri/target/release-gate";

const AXES = [
  {
    id: "tool-shapes",
    description: "Catalog, registered handlers, local-stdio inventory, and submit handler smoke coverage.",
    commands: [
      cargo("request-envelope-shape", ["mcp_tool_request_envelope_wire_shape"]),
      cargo("response-envelope-shape", ["mcp_tool_response_envelope_ok_wire_shape"]),
      cargo("tool-description-shape", ["tool_description_wire_when_not_to_call_keeps_uppercase_alias"]),
      cargo("embedded-catalog", ["embedded_catalog_loads_clean"]),
      cargo("catalog-fixture-coverage", ["fixture_coverage_enforced"]),
      cargo("host-selection-unit", ["account_status_host_selection_positive_negative"]),
      cargo("registered-handlers-match-catalog", ["registers_workspace_placement_handler_from_catalog"]),
      cargo("local-stdio-tool-list", ["mcp_v2_tools_list_registered_handlers_only"]),
      cargo("local-stdio-grant-refresh", ["local_stdio_grants_prune_stale_tools_for_client"]),
      cargo("submit-note-handler", ["tool_note"]),
      cargo("submit-action-handler", ["tool_create_action"]),
      cargo("submit-action-status-handler", ["tool_update_action_status"]),
    ],
  },
  {
    id: "privacy",
    description: "Presenter and transport boundaries omit raw claim IDs, attendee emails, paths, and unsafe detail.",
    commands: [
      cargo("runtime-projection-redacts-non-renderable-fields", ["mcp_projection_redacts_non_renderable_fields"]),
      cargo("runtime-projection-blocks-prompt-injection", ["mcp_projection_blocks_prompt_injection_text"]),
      cargo("runtime-projection-strips-invisible-text", ["mcp_projection_strips_invisible_text_and_caps_item_size"]),
      cargo("workspace-search-name-redaction", ["redacts_entity_names_without_scope"]),
      cargo("meeting-briefing-privacy", ["meeting_presenter_omits_attendee_email_and_raw_claim_ids"]),
      cargo("workspace-memory-search-privacy", ["tool_workspace_search"]),
      cargo("workspace-source-provenance-privacy", ["tool_workspace_source_provenance"]),
      cargo("account-resource-privacy", ["account_resource_omits_raw_ids_names_and_claim_text_without_name_scope"]),
      cargo("source-resource-handle-privacy", ["source_resource_replaces_entity_ids_with_resource_handles"]),
      cargo("resource-privacy", ["tool_resources"]),
      cargo("bad-params-redaction", ["tool_error_bad_params_does_not_leak_detail"]),
      cargoTest("ability-data-redaction", "dos412_mcp_ability_data_redaction_test", []),
    ],
  },
  {
    id: "continuity",
    description: "Conversation handles are server-minted, reusable for the same client, and rejected across clients.",
    commands: [
      cargo("first-call-may-omit-handle", ["mcp_tool_request_envelope_first_call_omits_handle"]),
      cargo("response-returns-handle", ["mcp_tool_response_envelope_ok_wire_shape"]),
      cargo("revoked-handle-error-shape", ["tool_error_conversation_revoked_wire_shape"]),
      cargo("prior-handle-reuse", ["resolve_or_mint_handle_reuses_valid_prior_handle_for_same_client"]),
      cargo("prior-handle-cross-client-reject", ["resolve_or_mint_handle_rejects_prior_handle_for_other_client"]),
      cargo("expired-prior-handle-replacement", ["resolve_or_mint_handle_replaces_expired_prior_handle_for_same_client"]),
    ],
  },
  {
    id: "v145-fidelity",
    description: "Workspace-memory graph, placement receipt, and source provenance contracts still hold for MCP v2.",
    commands: [
      cargoTest("reject-legacy-comparison-authority", "mcp_v2_runtime_authority_lint_test", [
        "mcp_comparison_path_rejects_legacy_authority",
      ]),
      cargo("workspace-search-graph-input", ["builds_graph_input_from_plan_aligned_filters"]),
      cargoTest("headless-placement-registration", "v146_validation", ["mcp_placement_handler_registered_for_headless_path"]),
      cargo("placement-handler-graph", ["placement_handler_commits_claim_and_graph_without_path_leak"]),
      cargo("placement-service-graph", ["workspace_placement_success_commits_claim_and_graph_without_path_leak"]),
      cargo("workspace-graph-unit", ["workspace_ingestion::graph::tests"]),
      cargo("opaque-source-handle-aliases", ["accepts_known_opaque_handle_aliases"]),
      cargo("source-path-rejection", ["rejects_path_like_entry_ids"]),
      cargo("host-stable-trust-band-labels", ["trust_band_labels_are_host_stable"]),
      cargo("workspace-source-provenance", ["tool_workspace_source_provenance"]),
    ],
  },
  {
    id: "host-selection",
    description: "W5-A tool-selection eval passes against the production catalog and fixture corpus.",
    commands: [
      {
        label: "v147-tool-selection-eval",
        argv: ["bash", "tests/v147_tool_eval/run.sh"],
      },
    ],
  },
  {
    id: "release-gate",
    description: "Hermetic release gate passes after W5-B axes are green.",
    commands: [
      {
        label: "hermetic-release-gate",
        argv: ["pnpm", "release-gate", "--", "--mode", "hermetic"],
      },
    ],
  },
];

const DEFAULT_AXIS_IDS = ["tool-shapes", "privacy", "continuity", "v145-fidelity", "host-selection"];

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
  const status = overallStatus(axisResults);
  const toolNames = loadToolNames(repoRoot);
  const releaseGateRan = axisResults.some((axis) => axis.axis === "release-gate");
  const report = {
    schema_version: "v147_mcp_e2e_validation_v1",
    generated_at: finishedAt.toISOString(),
    git_sha: commandOrUnknown(repoRoot, "git", ["rev-parse", "HEAD"]),
    branch: commandOrUnknown(repoRoot, "git", ["branch", "--show-current"]),
    status,
    duration_ms: finishedAt.getTime() - startedAt.getTime(),
    axes: axisResults,
    tool_coverage: toolCoverage(toolNames, axisResults),
    continuity: continuityEvidence(axisResults),
    reports: {
      tool_eval: TOOL_EVAL_REPORT,
      release_gate: releaseGateRan ? RELEASE_GATE_REPORT : null,
    },
    evidence_policy:
      "Report stores command labels, counts, statuses, durations, and exit codes only. Payloads, prompts, entity names, paths, claim text, source handles, domains, and customer data are omitted.",
  };

  const outPath = path.resolve(repoRoot, args.out ?? DEFAULT_REPORT);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);

  const counts = axisCounts(axisResults);
  console.log(
    `v1.4.7 MCP e2e validation ${status}: axes=${axisResults.length} pass=${counts.pass} fail=${counts.fail} blocked=${counts.blocked}`,
  );
  console.log(`report: ${toRepoRelative(repoRoot, outPath)}`);
  for (const axis of axisResults) {
    console.log(`axis ${axis.axis}: ${axis.status}`);
  }

  if (status === "fail") {
    process.exit(1);
  }
  if (status === "blocked") {
    process.exit(2);
  }
}

function cargo(label, selectors) {
  return {
    label,
    argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", ...selectors, "--lib"],
  };
}

function cargoTest(label, testName, selectors) {
  return {
    label,
    argv: ["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "--test", testName, ...selectors],
  };
}

function runAxis(repoRoot, axis) {
  const startedAt = Date.now();
  const commands = axis.commands.map((command) => runCommand(repoRoot, command));
  const passed = commands.filter((command) => command.status === "pass").length;
  return {
    axis: axis.id,
    status: commands.every((command) => command.status === "pass") ? "pass" : "fail",
    evidence_ref: commands.map((command) => command.label).join(" + "),
    summary: `${passed}/${commands.length} command(s) passed`,
    duration_ms: Date.now() - startedAt,
    commands,
  };
}

function runCommand(repoRoot, command) {
  const startedAt = Date.now();
  const [bin, ...args] = command.argv;
  const result = spawnSync(bin, args, {
    cwd: repoRoot,
    stdio: ["ignore", "ignore", "ignore"],
  });
  return {
    label: command.label,
    status: result.status === 0 ? "pass" : "fail",
    exit_code: result.status ?? -1,
    duration_ms: Date.now() - startedAt,
  };
}

function selectAxes(axis) {
  if (axis === "all") {
    return DEFAULT_AXIS_IDS.map((id) => axisById(id));
  }
  if (axis === "all-with-release-gate") {
    return [...DEFAULT_AXIS_IDS, "release-gate"].map((id) => axisById(id));
  }
  return [axisById(axis)];
}

function axisById(id) {
  const found = AXES.find((candidate) => candidate.id === id);
  if (!found) {
    throw new Error(
      `unknown axis: ${id}. Known axes: all, all-with-release-gate, ${AXES.map((item) => item.id).join(", ")}`,
    );
  }
  return found;
}

function overallStatus(axisResults) {
  if (axisResults.some((axis) => axis.status === "fail")) {
    return "fail";
  }
  if (axisResults.some((axis) => axis.status === "blocked")) {
    return "blocked";
  }
  return "pass";
}

function axisCounts(axisResults) {
  return {
    pass: axisResults.filter((axis) => axis.status === "pass").length,
    fail: axisResults.filter((axis) => axis.status === "fail").length,
    blocked: axisResults.filter((axis) => axis.status === "blocked").length,
  };
}

function loadToolNames(repoRoot) {
  const catalogPath = path.join(repoRoot, "src-tauri/src/services/mcp_v2/resources/tool_descriptions.yaml");
  const catalog = JSON.parse(fs.readFileSync(catalogPath, "utf8"));
  const entries = Array.isArray(catalog) ? catalog : catalog.tools;
  if (!Array.isArray(entries)) {
    throw new Error("tool catalog must be an array or an object with a tools array");
  }
  return entries.map((tool) => tool.name).sort();
}

function toolCoverage(toolNames, axisResults) {
  const toolShapes = axisResults.find((axis) => axis.axis === "tool-shapes");
  if (!toolShapes) {
    return {
      expected: toolNames.length,
      validated: 0,
      missing: [],
      status: "not_run",
    };
  }
  const passed = toolShapes?.status === "pass";
  return {
    expected: toolNames.length,
    validated: passed ? toolNames.length : 0,
    missing: passed ? [] : toolNames,
    status: toolShapes.status,
  };
}

function continuityEvidence(axisResults) {
  const continuity = axisResults.find((axis) => axis.axis === "continuity");
  if (!continuity) {
    return {
      same_session: "not_run",
      cross_session_prior_handle: "not_run",
      cross_client_prior_handle_rejected: "not_run",
      expired_prior_handle_replaced: "not_run",
    };
  }
  return {
    same_session: statusForCommand(continuity, "response-returns-handle"),
    cross_session_prior_handle: statusForCommand(continuity, "prior-handle-reuse"),
    cross_client_prior_handle_rejected: statusForCommand(continuity, "prior-handle-cross-client-reject"),
    expired_prior_handle_replaced: statusForCommand(continuity, "expired-prior-handle-replacement"),
  };
}

function statusForCommand(axis, label) {
  return axis.commands.find((command) => command.label === label)?.status ?? "not_run";
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
  console.log(
    "Usage: node tests/v147_e2e/run.mjs [--axis all|all-with-release-gate|tool-shapes|privacy|continuity|v145-fidelity|host-selection|release-gate] [--out PATH]",
  );
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
