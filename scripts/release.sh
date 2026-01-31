#!/bin/bash
# Release script for Arq
# Usage: ./scripts/release.sh [patch|minor|major]
#
# Examples:
#   ./scripts/release.sh patch  # 0.2.0 -> 0.2.1
#   ./scripts/release.sh minor  # 0.2.0 -> 0.3.0
#   ./scripts/release.sh major  # 0.2.0 -> 1.0.0

set -e

BUMP_TYPE=${1:-patch}

if [[ ! "$BUMP_TYPE" =~ ^(patch|minor|major)$ ]]; then
    echo "Usage: $0 [patch|minor|major]"
    exit 1
fi

echo "==> Checking for uncommitted changes..."
if [[ -n $(git status --porcelain) ]]; then
    echo "Error: You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

echo "==> Running tests..."
cargo test --workspace

echo "==> Bumping version ($BUMP_TYPE)..."
cargo release $BUMP_TYPE --execute

echo ""
echo "Release complete! GitHub Actions will now build and publish the release."
echo "Check progress at: https://github.com/AssahBismarkabah/Arq/actions"
