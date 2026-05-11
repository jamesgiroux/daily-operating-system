#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const ignoredDirectoryNames = new Set([".git", "node_modules"]);
const defaultRootNames = new Set([".docs/evals", ".docs/perf"]);
const rootPolicyDocAllowlist = new Set([
  ".docs/evals/evaluation-evidence-contract.md",
  ".docs/evals/evidence-record.schema.json",
  ".docs/evals/fixture-governance.md",
  ".docs/perf/README.md",
]);
const defaultTargets = [
  ".docs/evals",
  ".docs/perf",
  "src-tauri/tests/fixtures",
  "src-tauri/target/evidence",
].filter((target) =>
  fs.existsSync(path.resolve(repoRoot, target)),
);

const emailPattern =
  /\b[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@([A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+)\b/g;

const lineChecks = [
  {
    id: "file-url",
    pattern: /\bfile:\/\/[^\s"'<>)]*/i,
  },
  {
    id: "local-users-path",
    pattern: /(^|[\s"'(=:[,{])\/Users\/[^\s"'<>),}\]]+/,
  },
  {
    id: "local-home-path",
    pattern: /(^|[\s"'(=:[,{])\/home\/[A-Za-z0-9._-]+(?:\/|$)/,
  },
  {
    id: "windows-users-path",
    pattern: /(^|[\s"'(=:[,{])[A-Za-z]:\\Users\\[^\s"'<>),}\]]+/i,
  },
  {
    id: "windows-absolute-path",
    pattern: /(^|[\s"'(=:[,{])[A-Za-z]:\\[^\s"'<>),}\]]+/i,
  },
  {
    id: "mac-private-var-path",
    pattern: /(^|[\s"'(=:[,{])\/(?:private\/var|var\/folders)\/[^\s"'<>),}\]]*/i,
  },
  {
    id: "fixture-identity-map-reference",
    pattern: /\bfixture_identity_map\b/i,
  },
  {
    id: "phone-like-number",
    pattern: /(^|[^A-Za-z0-9])(?:\+?1[\s.-]?)?(?:\([0-9]{3}\)|[0-9]{3})[\s.-]+[0-9]{3}[\s.-]+[0-9]{4}($|[^A-Za-z0-9])/,
  },
  {
    id: "phone-like-number",
    pattern:
      /\b(?:phone|tel|telephone|mobile|cell|sms|call)\b[^\n]{0,32}\b[0-9]{3}[\s.-][0-9]{4}\b/i,
  },
  {
    id: "redacted-scrub-artifact",
    pattern: /\bREDACTED\b/,
  },
  {
    id: "private-home-path-token",
    pattern:
      /(^|[\s"'(=:[,{])~\/(?:\.dailyos|\.ssh|\.aws|\.config|\.gnupg|Desktop|Documents|Downloads|Library(?:\/(?:Application Support|Keychains|Mail|Calendars|Containers))?)(?:\/|$)/i,
  },
  {
    id: "private-path-token",
    pattern:
      /(^|[\s"'(=:[,{/])(?:id_rsa|id_ed25519|\.env(?:\.[A-Za-z0-9_-]+)?|\.netrc|\.npmrc|\.pypirc|\.git-credentials|credentials\.json|secrets\.json|\.cargo\/credentials(?:\.toml)?|\.aws\/credentials|\.ssh\/config)(?:$|[\s"'),}\]/])/i,
  },
  {
    id: "private-dailyos-cache-path",
    pattern: /(^|[\/~])\.dailyos(?:\/|$)/i,
  },
];

const requestedTargets = process.argv.slice(2).filter((arg) => arg !== "--");
const targets = requestedTargets.length > 0 ? requestedTargets : defaultTargets;

if (targets.length === 0) {
  console.log("PASS no default evidence roots exist");
  process.exit(0);
}

let failedRoots = 0;

for (const target of targets) {
  const result = lintRoot(target);

  if (result.violations.length > 0 || result.errors.length > 0) {
    failedRoots += 1;
    console.error(`FAIL ${target}`);
    for (const error of result.errors) {
      console.error(`  ${error}`);
    }
    for (const violation of result.violations) {
      console.error(
        `  ${violation.path}:${violation.line}:${violation.column} ${violation.rule}`,
      );
    }
    continue;
  }

  const skipped = result.skippedFiles === 0 ? "" : `, ${result.skippedFiles} skipped`;
  console.log(`PASS ${target} (${result.checkedFiles} files checked${skipped})`);
}

process.exit(failedRoots === 0 ? 0 : 1);

function lintRoot(target) {
  const resolved = path.resolve(repoRoot, target);
  const result = {
    checkedFiles: 0,
    skippedFiles: 0,
    errors: [],
    violations: [],
  };

  if (!fs.existsSync(resolved)) {
    result.errors.push(`target does not exist: ${target}`);
    return result;
  }

  const rootStat = fs.lstatSync(resolved);
  if (rootStat.isSymbolicLink()) {
    result.violations.push({
      path: toRepoRelative(resolved),
      line: 1,
      column: 1,
      rule: "root-symlink-not-allowed",
    });
    return result;
  }

  const stat = fs.statSync(resolved);
  const scanFiles = stat.isDirectory()
    ? collectFiles(resolved, { root: resolved, result })
    : [resolved];

  for (const file of scanFiles) {
    const relPath = toRepoRelative(file);

    if (hasIgnoredSegment(relPath)) {
      result.skippedFiles += 1;
      continue;
    }

    const fileResult = lintFile(file);
    if (fileResult.skipped) {
      result.skippedFiles += 1;
      continue;
    }

    result.checkedFiles += 1;
    result.violations.push(...fileResult.violations);
  }

  return result;
}

function collectFiles(root, options) {
  const files = [];
  const entries = fs
    .readdirSync(root, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name));

  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);

    if (entry.isSymbolicLink()) {
      options.result.violations.push({
        path: toRepoRelative(fullPath),
        line: 1,
        column: 1,
        rule: "symlink-not-allowed",
      });
      continue;
    }

    if (entry.isDirectory()) {
      if (ignoredDirectoryNames.has(entry.name) || shouldIgnoreTargetDirectory(fullPath)) {
        continue;
      }
      files.push(...collectFiles(fullPath, options));
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    if (shouldSkipDefaultRootDoc(fullPath, options)) {
      continue;
    }

    files.push(fullPath);
  }

  return files;
}

function shouldSkipDefaultRootDoc(file, options) {
  if (path.dirname(file) !== options.root) {
    return false;
  }

  return (
    defaultRootNames.has(toRepoRelative(options.root)) &&
    rootPolicyDocAllowlist.has(toRepoRelative(file))
  );
}

function shouldIgnoreTargetDirectory(dir) {
  const relPath = toRepoRelative(dir);
  if (!relPath.split("/").includes("target")) {
    return false;
  }
  return !relPath.startsWith("src-tauri/target/evidence");
}

function lintFile(file) {
  const relPath = toRepoRelative(file);
  const buffer = fs.readFileSync(file);

  if (buffer.includes(0)) {
    return {
      skipped: false,
      violations: [
        {
          path: relPath,
          line: 1,
          column: 1,
          rule: "binary-artifact-not-allowed",
        },
      ],
    };
  }

  const text = buffer.toString("utf8");
  const violations = [];
  const lines = text.split(/\r?\n/);

  lines.forEach((line, index) => {
    const lineNumber = index + 1;

    for (const violation of findEmailViolations(line, relPath, lineNumber)) {
      violations.push(violation);
    }

    const reportedRules = new Set();
    for (const check of lineChecks) {
      if (reportedRules.has(check.id)) {
        continue;
      }

      const match = check.pattern.exec(line);
      if (match) {
        reportedRules.add(check.id);
        violations.push({
          path: relPath,
          line: lineNumber,
          column: match.index + 1,
          rule: check.id,
        });
      }
    }
  });

  return { skipped: false, violations };
}

function findEmailViolations(line, relPath, lineNumber) {
  const violations = [];
  emailPattern.lastIndex = 0;

  for (const match of line.matchAll(emailPattern)) {
    const domain = match[1].toLowerCase();
    if (domain === "example.com" || domain.endsWith(".example.com")) {
      continue;
    }

    violations.push({
      path: relPath,
      line: lineNumber,
      column: match.index + 1,
      rule: "non-example-email",
    });
  }

  return violations;
}

function hasIgnoredSegment(relPath) {
  return relPath
    .split("/")
    .some((segment) => ignoredDirectoryNames.has(segment));
}

function toRepoRelative(file) {
  const relative = path.relative(repoRoot, file);
  if (relative === "") {
    return ".";
  }
  if (relative === ".." || relative.startsWith(`..${path.sep}`)) {
    return file.split(path.sep).join("/");
  }
  return relative.split(path.sep).join("/");
}
