#!/usr/bin/env bash
# Install repo-local git hooks.
#
# Idempotent. Wired into `pnpm install` via the postinstall script so every
# clone gets the gates wired without manual setup. Manual: `bash scripts/install-hooks.sh`.
#
# Bypass per-commit: WIP=1 git commit ... | git commit --no-verify
# Disable entirely: git config --unset core.hooksPath

set -euo pipefail

# CI environments don't need local hooks (workflows enforce gates server-side).
# Worktrees / non-git invocations exit cleanly so postinstall doesn't fail.
if [ "${CI:-false}" = "true" ] || [ -n "${GITHUB_ACTIONS:-}" ]; then
  exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$REPO_ROOT" ] || [ ! -d "$REPO_ROOT/.githooks" ]; then
  exit 0
fi

cd "$REPO_ROOT"

# Skip if already pointed at .githooks (idempotent fast path).
CURRENT="$(git config --get core.hooksPath || echo)"
if [ "$CURRENT" = ".githooks" ]; then
  exit 0
fi

git config core.hooksPath .githooks
echo "git hooks installed: core.hooksPath=.githooks"
echo "active gates:"
ls -1 .githooks/ | sed 's/^/  /'
