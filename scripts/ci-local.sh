#!/usr/bin/env bash
# CAFramework — Local CI Verification Gate (VS-R, VS-F, VS-G, VS-A)
# Exits non-zero on the first failure.
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CAF_DATA_DIR="$(mktemp -d)"
trap 'rm -rf "$CAF_DATA_DIR"' EXIT

echo "=== CAFramework Local CI Verification ==="
echo "Workspace root: $ROOT_DIR"
echo "Isolated data dir: $CAF_DATA_DIR"
echo "Rustc: $(rustc -V)"
echo "Cargo: $(cargo -V)"

# Tracking results
declare -a STEPS=()
declare -a RESULTS=()

run_step() {
  local name="$1"
  shift
  echo ""
  echo ">>> [RUNNING] $name"
  local start_time=$(date +%s)
  if "$@"; then
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    echo ">>> [PASSED] $name (${duration}s)"
    STEPS+=("$name")
    RESULTS+=("PASS (${duration}s)")
  else
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    echo ">>> [FAILED] $name (${duration}s)"
    STEPS+=("$name")
    RESULTS+=("FAIL (${duration}s)")
    print_summary 1
    exit 1
  fi
}

print_summary() {
  local exit_code="$1"
  echo ""
  echo "=========================================="
  echo "           CI VERIFICATION SUMMARY        "
  echo "=========================================="
  for i in "${!STEPS[@]}"; do
    printf "%-40s | %s\n" "${STEPS[$i]}" "${RESULTS[$i]}"
  done
  echo "=========================================="
  if [ "$exit_code" -eq 0 ]; then
    echo "ALL GATES PASSED (GREEN)"
  else
    echo "GATE FAILURE (RED)"
  fi
}

# --- VS-R: Rust Core & Tools ---
run_step "VS-R: cargo build (non-app)" cargo build --workspace --exclude caf-app --all-targets
run_step "VS-R: cargo fmt --check" cargo fmt --all -- --check
run_step "VS-R: cargo clippy (strict)" cargo clippy --workspace --all-targets --exclude caf-app -- -D warnings
run_step "VS-R: cargo test (non-app)" cargo test --workspace --exclude caf-app

if command -v cargo-deny >/dev/null 2>&1; then
  run_step "VS-R: cargo deny check" cargo deny check
else
  echo ">>> [SKIPPED] cargo deny (cargo-deny binary not installed)"
fi

# --- VS-F: Frontend ---
# Per task.md §1.1: npm run check must report 0 errors and 0 a11y warnings from P16 on.
if [ "${STRICT_SVELTE_CHECK:-0}" = "1" ] || [ -f "$ROOT_DIR/.p16_active" ]; then
  run_step "VS-F: npm run check (strict)" bash -c "cd frontend && npm run check"
else
  run_step "VS-F: npm run check (pre-P16)" bash -c "cd frontend && (npm run check || true)"
fi
run_step "VS-F: npm run build" bash -c "cd frontend && npm run build"
run_step "VS-F: npm run test" bash -c "cd frontend && npm run test"

# --- VS-G: Guards & Codegen ---
run_step "VS-G: xtask codegen --check" cargo run -p caf-xtask -- codegen --check
run_step "VS-G: xtask guard" cargo run -p caf-xtask -- guard

if [ -f scripts/guard-negatives.sh ]; then
  run_step "VS-G: guard negatives" bash scripts/guard-negatives.sh
fi

# --- VS-A: Tauri App (if system dependencies present) ---
if pkg-config --exists webkit2gtk-4.1 gtk+-3.0 2>/dev/null; then
  run_step "VS-A: cargo check -p caf-app" cargo check -p caf-app
  run_step "VS-A: cargo clippy -p caf-app" cargo clippy -p caf-app -- -D warnings
else
  echo ">>> [SKIPPED] VS-A (webkit2gtk-4.1 / gtk+-3.0 missing, OWNER-VERIFY)"
fi

print_summary 0
exit 0
