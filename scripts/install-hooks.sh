#!/usr/bin/env bash
# Install repo-local git hooks.
#
# Idempotent. Run once per clone (or after `core.hooksPath` is changed by
# something else).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
echo "installed: core.hooksPath=.githooks"
echo "hooks:"
ls -1 .githooks/ | sed 's/^/  /'
