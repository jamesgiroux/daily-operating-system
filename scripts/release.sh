#!/usr/bin/env bash
# Tag a release on Automattic (trunk) and mirror the tag + code to the public
# jamesgiroux/daily-operating-system repo, which triggers the public release
# workflow to build and publish the DMG.
#
# Usage: ./scripts/release.sh v1.3.0

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version-tag>"
    echo "Example: $0 v1.3.0"
    exit 1
fi

TAG="$1"

if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "ERROR: Tag must be in format vX.Y.Z (e.g., v1.3.0). Got: $TAG"
    exit 1
fi

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "trunk" ]; then
    echo "ERROR: Must be on trunk branch to release. Currently on: $CURRENT_BRANCH"
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "ERROR: Working directory is not clean. Commit or stash changes first."
    exit 1
fi

if ! git remote | grep -q '^public$'; then
    echo "ERROR: Remote 'public' not configured. Run:"
    echo "  git remote add public git@github.com:jamesgiroux/daily-operating-system.git"
    exit 1
fi

TAURI_VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: "\(.*\)".*/\1/')
EXPECTED_TAG="v${TAURI_VERSION}"
if [ "$TAG" != "$EXPECTED_TAG" ]; then
    echo "ERROR: Tag ($TAG) does not match tauri.conf.json version ($EXPECTED_TAG)."
    echo "Update src-tauri/tauri.conf.json, src-tauri/Cargo.toml, and package.json first."
    exit 1
fi

echo "Fetching latest from origin (Automattic)..."
git fetch origin trunk

LOCAL=$(git rev-parse trunk)
REMOTE=$(git rev-parse origin/trunk)
if [ "$LOCAL" != "$REMOTE" ]; then
    echo "ERROR: Local trunk is not in sync with origin/trunk."
    echo "  Local:  $LOCAL"
    echo "  Remote: $REMOTE"
    exit 1
fi

echo ""
echo "Ready to release $TAG from $(git rev-parse --short HEAD):"
git log -1 --format="  %h %s"
echo ""
read -r -p "Continue? [y/N] " response
if [[ ! "$response" =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo "Creating tag $TAG..."
git tag -a "$TAG" -m "Release $TAG"

echo "Pushing tag to Automattic..."
git push origin "$TAG"

echo "Mirroring to public repo (jamesgiroux/daily-operating-system)..."
git push --force public "trunk:main"
git push public "$TAG"

echo ""
echo "Done. Public release workflow should be building the DMG now:"
echo "  https://github.com/jamesgiroux/daily-operating-system/actions"
