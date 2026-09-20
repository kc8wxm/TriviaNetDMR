---
name: software-release
description: >-
  Orchestrates end-to-end software release workflows for TriviaNetDMR, including
  Git-Flow release branches, pre-flight test suites, Cargo version bumping,
  local packaging, git tagging, GitHub synchronization, and release documentation.
  Use when the user requests to release a new version (e.g., "release v0.1.3",
  "publish release", or "prepare a new software release").
---

# Software Release Engineering Guide & Runbook

This skill provides comprehensive instructions for executing safe, reproducible,
and automated software releases for TriviaNetDMR using Git-Flow and GitHub Actions.

## Quick Reference Workflow

```bash
# Automated release script (handles steps 1 through 5):
./scripts/release.sh <version>
# Example:
./scripts/release.sh 0.1.3
```

---

## Detailed Step-by-Step Release Pipeline

### Step 1: Pre-Flight Safety Verification
Before initiating any release actions:
1. Ensure the working tree is clean:
   ```bash
   git status
   ```
2. Verify you are starting on the `develop` branch:
   ```bash
   git checkout develop
   git pull origin develop
   ```
3. Execute the full unit test suite:
   ```bash
   cargo test
   ```
   *All unit tests must pass before proceeding.*
4. Validate all trivia decks:
   ```bash
   cargo run -- --check Topic/
   ```

---

### Step 2: Version Bumping & Dependency Lock
1. Update package version in `Cargo.toml`:
   ```toml
   [package]
   name = "TriviaNetDMR"
   version = "X.Y.Z"
   ```
2. Sync `Cargo.lock` by running:
   ```bash
   cargo check
   ```

---

### Step 3: Git-Flow Release Lifecycle
1. Start the Git-Flow release branch:
   ```bash
   git flow release start vX.Y.Z
   # Or without prefix if configured: git flow release start X.Y.Z
   ```
2. Commit release preparations:
   ```bash
   git commit -am "Prepare release vX.Y.Z"
   ```
3. Finish the release branch:
   ```bash
   GIT_MERGE_AUTOEDIT=no git flow release finish -m "Release vX.Y.Z" vX.Y.Z
   ```
   *This merges changes into `master`, tags the commit as `vX.Y.Z`, and merges back into `develop`.*

---

### Step 4: Local Binary Packaging
Build optimized binaries and package local archives:
```bash
# 1. Build Linux release binary
cargo build --release

# 2. (Optional) Build Windows target if mingw-w64 is installed:
cargo build --release --target x86_64-pc-windows-gnu

# 3. Create release bundle
DIST_DIR="dist/TriviaNetDMR-vX.Y.Z-linux-x86_64"
mkdir -p "$DIST_DIR/Topic"
cp target/release/TriviaNetDMR "$DIST_DIR/"
cp -r Topic/*.md "$DIST_DIR/Topic/"
cp GEMINI.md "$DIST_DIR/README.md"
cp run.bat "$DIST_DIR/"
tar -czvf "dist/TriviaNetDMR-vX.Y.Z-linux-x86_64.tar.gz" -C dist TriviaNetDMR-vX.Y.Z-linux-x86_64
```

---

### Step 5: Remote Push & CI/CD Trigger
Push production code, development branch, and tags to GitHub:
```bash
git push origin master develop --tags
```
*Note: Pushing tag `vX.Y.Z` automatically triggers the `.github/workflows/release.yml` GitHub Actions pipeline, which builds multi-platform binaries (Linux x86_64, Windows x86_64, macOS Apple Silicon, macOS Intel) and attaches them to a new GitHub Release.*

---

### Step 6: Post-Release Documentation
1. Update `journal/index.org`: Update `Current Release: vX.Y.Z`.
2. Update `journal/dev_log.org`: Add a dated entry documenting:
   - Release focus and highlights.
   - List of features, enhancements, and bug fixes.
   - Passing test metrics.
3. Commit and push journal changes to `develop`.
