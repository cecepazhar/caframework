#!/usr/bin/env bash
# Runs all negative guard tests proving guards fail when invariants are violated.
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== Running Guard Negative Fixture Tests ==="
cargo test -p caf-xtask --test guard_tests fails_on_ -- --nocapture
echo "All negative guard fixtures verified (exits != 0 on invalid input)."
