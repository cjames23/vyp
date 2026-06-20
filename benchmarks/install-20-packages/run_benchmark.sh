#!/usr/bin/env bash
# Benchmark: lock + install for 20 packages — vyp vs uv
# Usage:
#   ./run_benchmark.sh           # warm caches (default)
#   ./run_benchmark.sh --cold    # true cold: both tools use empty cache dirs
#   COLD=1 ./run_benchmark.sh    # same as --cold
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
VYP="${VYP:-vyp}"
UV="${UV:-uv}"

if [[ "${1:-}" == "--cold" ]] || [[ "${COLD:-}" == "1" ]]; then
  COLD=1
else
  COLD=0
fi

if [[ "$COLD" == "1" ]]; then
  UV_CACHE_DIR=$(mktemp -d)
  VYP_CACHE_DIR=$(mktemp -d)
  export UV_CACHE_DIR VYP_CACHE_DIR
  echo "=== COLD START (empty caches: UV_CACHE_DIR=$UV_CACHE_DIR, VYP_CACHE_DIR=$VYP_CACHE_DIR) ==="
  trap 'rm -rf "$UV_CACHE_DIR" "$VYP_CACHE_DIR"' EXIT
else
  echo "=== WARM CACHES (lock + install, 20 direct packages) ==="
fi
echo "Project: $SCRIPT_DIR"
echo ""

# Clean state
rm -rf .venv pylock.toml uv.lock 2>/dev/null || true

# --- UV ---
echo "--- uv ---"
if ! command -v uv &>/dev/null; then
  echo "uv not found. Install with: curl -LsSf https://astral.sh/uv/install.sh | sh"
  UV_TIME="N/A"
else
  UV_START=$(python3 -c "import time; print(time.perf_counter())")
  $UV lock --quiet
  $UV sync --quiet
  UV_END=$(python3 -c "import time; print(time.perf_counter())")
  UV_TIME=$(python3 -c "print(round($UV_END - $UV_START, 2))")
  echo "uv lock + sync: ${UV_TIME}s"
fi
echo ""

# Clean for vyp (vyp install expects an existing venv with lib/)
rm -rf .venv 2>/dev/null || true
rm -f uv.lock pylock.toml 2>/dev/null || true
python3 -m venv "$SCRIPT_DIR/.venv"

# --- VYP ---
echo "--- vyp ---"
VYP_BIN="${VYP:-vyp}"
if [[ -f "$SCRIPT_DIR/../../target/release/vyp" ]] && [[ -z "${VYP:-}" ]]; then
  VYP_BIN="$SCRIPT_DIR/../../target/release/vyp"
elif [[ -f "$SCRIPT_DIR/../../target/debug/vyp" ]] && [[ -z "${VYP:-}" ]]; then
  VYP_BIN="$SCRIPT_DIR/../../target/debug/vyp"
fi
if ! [[ -x "$VYP_BIN" ]] && command -v vyp &>/dev/null; then
  VYP_BIN="vyp"
fi

VYP_START=$(python3 -c "import time; print(time.perf_counter())")
$VYP_BIN lock --project "$SCRIPT_DIR/pyproject.toml" --output "$SCRIPT_DIR/pylock.toml"
$VYP_BIN install --project "$SCRIPT_DIR/pyproject.toml" --lockfile "$SCRIPT_DIR/pylock.toml" --venv "$SCRIPT_DIR/.venv"
VYP_END=$(python3 -c "import time; print(time.perf_counter())")
VYP_TIME=$(python3 -c "print(round($VYP_END - $VYP_START, 2))")
echo "vyp lock + install: ${VYP_TIME}s"
echo ""

# Summary
echo "=== Summary ==="
if [[ "$UV_TIME" != "N/A" ]]; then
  echo "  uv:  ${UV_TIME}s"
fi
echo "  vyp: ${VYP_TIME}s"
if [[ "$UV_TIME" != "N/A" ]] && [[ "$UV_TIME" != "" ]]; then
  RATIO=$(python3 -c "print(round($VYP_TIME / $UV_TIME, 2))")
  echo "  (vyp/uv ratio: ${RATIO}x)"
fi
