#!/usr/bin/env bash
# CAFramework — Feature Matrix Build & Test Gate (P13.1 / A-01)
# Verifies caf-core with no default features, each single feature, and all features.
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
export OPENSSL_NO_VENDOR=1

echo "=== CAFramework Feature Matrix Verification ==="
echo "Workspace root: $ROOT_DIR"

FEATURES=(
  "mod-profiles"
  "mod-ai"
  "mod-ai-advanced"
  "mod-pro"
  "mod-feedback"
  "mod-updater"
  "mod-crash"
  "mod-integrations"
  "mod-scheduler"
  "mod-support-page"
  "mod-background"
  "builder"
)

echo ""
echo ">>> [1/3] Checking core with --no-default-features"
cargo check -p caf-core --no-default-features

echo ""
echo ">>> [2/3] Checking core with each single feature"
for feat in "${FEATURES[@]}"; do
  echo "--> Feature: $feat"
  cargo check -p caf-core --no-default-features --features "$feat"
done

echo ""
echo ">>> [3/3] Checking core with --all-features"
cargo check -p caf-core --all-features

echo ""
echo "=== Feature matrix check passed (GREEN) ==="
exit 0
