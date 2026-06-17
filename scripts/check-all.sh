#!/usr/bin/env bash
#
# check-all.sh — Run all quality checks on the PrimusDB codebase
#
# Usage:
#   ./scripts/check-all.sh
#   ./scripts/check-all.sh --fix       # Auto-fix formatting issues
#   ./scripts/check-all.sh --skip-docs # Skip doc checks
#
set -euo pipefail

FIX=0
SKIP_DOCS=0
HELP=0
FAILED=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fix) FIX=1; shift ;;
        --skip-docs) SKIP_DOCS=1; shift ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--fix] [--skip-docs]"
    echo "  --fix         Auto-fix formatting issues"
    echo "  --skip-docs   Skip documentation checks"
    exit 0
fi

GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[0;33m"
NC="\033[0m"

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; FAILED=1; }
info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

echo "=== PrimusDB Quality Checks ==="
echo ""

# ── 1. Format ──────────────────────────────────────
echo "--- cargo fmt ---"
if [[ "$FIX" -eq 1 ]]; then
    cargo fmt --all
    pass "cargo fmt (fixed)"
else
    if cargo fmt --all --check 2>&1; then
        pass "cargo fmt"
    else
        fail "cargo fmt (run 'cargo fmt --all' to fix)"
    fi
fi

# ── 2. Clippy ──────────────────────────────────────
echo "--- cargo clippy ---"
if cargo clippy --workspace -- -D warnings 2>&1; then
    pass "cargo clippy"
else
    fail "cargo clippy"
fi

# ── 3. Build ──────────────────────────────────────
echo "--- cargo build ---"
if cargo build --workspace 2>&1; then
    pass "cargo build"
else
    fail "cargo build"
fi

# ── 4. Build release ─────────────────────────────────
echo "--- cargo build --release ---"
if cargo build --release --workspace 2>&1; then
    pass "cargo build --release"
else
    fail "cargo build --release"
fi

# ── 5. Tests ──────────────────────────────────────
echo "--- cargo test ---"
if cargo test --workspace 2>&1; then
    pass "cargo test"
else
    fail "cargo test"
fi

# ── 6. Doc tests ──────────────────────────────────
echo "--- cargo test --doc ---"
if cargo test --doc 2>&1; then
    pass "cargo test --doc"
else
    fail "cargo test --doc"
fi

# ── 7. Docs ───────────────────────────────────────
if [[ "$SKIP_DOCS" -eq 0 ]]; then
    echo "--- documentation checks ---"
    if [[ -f "scripts/check-docs.sh" ]]; then
        if bash scripts/check-docs.sh 2>&1; then
            pass "docs check"
        else
            fail "docs check"
        fi
    else
        info "scripts/check-docs.sh not found — skipping"
    fi
fi

# ── Summary ───────────────────────────────────────
echo ""
if [[ "$FAILED" -eq 1 ]]; then
    echo -e "${RED}Some checks failed.${NC}"
    exit 1
else
    echo -e "${GREEN}All checks passed.${NC}"
fi
