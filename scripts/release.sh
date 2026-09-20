#!/usr/bin/env bash
# ==============================================================================
# TriviaNetDMR Release Automation Script
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.1.3
# ==============================================================================

set -euo pipefail

# 1. Parameter Validation
if [ -z "${1:-}" ]; then
  echo "❌ Error: Release version argument required."
  echo "Usage: $0 <version> (e.g. $0 0.1.3)"
  exit 1
fi

RAW_VERSION="$1"
# Strip leading 'v' if supplied
VERSION="${RAW_VERSION#v}"
TAG="v${VERSION}"

echo "========================================================="
echo "🚀 Starting Software Release Pipeline for ${TAG}"
echo "========================================================="

# 2. Pre-flight Git Checks
echo "🔍 Checking Git status..."
if [ -n "$(git status --porcelain)" ]; then
  echo "❌ Error: Working directory has uncommitted changes. Stash or commit before releasing."
  git status --short
  exit 1
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$CURRENT_BRANCH" != "develop" ]; then
  echo "⚠️ Warning: Current branch is '$CURRENT_BRANCH', switching to 'develop'..."
  git checkout develop
fi

# 3. Test & Verification Suite
echo "🧪 Running full automated test suite..."
cargo test --quiet

echo "📚 Validating question decks..."
cargo run --quiet -- --check Topic/

# 4. Git-Flow Release Branch
echo "🌿 Starting Git-Flow release branch: ${TAG}..."
git flow release start "${TAG}"

# 5. Version Bumping
echo "📝 Bumping package version in Cargo.toml to ${VERSION}..."
sed -i -E "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml

echo "🔒 Updating Cargo.lock..."
cargo check --quiet

echo "💾 Committing release version bump..."
git commit -am "Prepare release ${TAG}"

# 6. Finish Git-Flow Release
echo "🏁 Finishing Git-Flow release..."
GIT_MERGE_AUTOEDIT=no git flow release finish -m "Release ${TAG}" "${TAG}"

# 7. Build Local Release Binary
echo "🔨 Compiling optimized release binary..."
cargo build --release --quiet

# 8. Package Distribution Archive
echo "📦 Packaging distribution archive..."
DIST_DIR="dist/TriviaNetDMR-${TAG}-linux-x86_64"
rm -rf "$DIST_DIR" "dist/TriviaNetDMR-${TAG}-linux-x86_64.tar.gz"
mkdir -p "$DIST_DIR/Topic"
cp target/release/TriviaNetDMR "$DIST_DIR/"
cp -r Topic/*.md "$DIST_DIR/Topic/"
cp GEMINI.md "$DIST_DIR/README.md"
cp run.bat "$DIST_DIR/"
tar -czf "dist/TriviaNetDMR-${TAG}-linux-x86_64.tar.gz" -C dist "TriviaNetDMR-${TAG}-linux-x86_64"

echo "========================================================="
echo "✅ Release ${TAG} completed successfully!"
echo "   - Git tag: ${TAG}"
echo "   - Merged to: master & develop"
echo "   - Linux Archive: dist/TriviaNetDMR-${TAG}-linux-x86_64.tar.gz"
echo ""
echo "To publish to GitHub and trigger multi-platform CI/CD builds, run:"
echo "   git push origin master develop --tags"
echo "========================================================="
