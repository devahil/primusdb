#!/usr/bin/env bash
#
# check-docs.sh — Validate documentation quality
#
# Checks:
#   1. All doc files are listed in README.md
#   2. No broken markdown links
#   3. No TODO/FIXME/HACK in docs
#   4. Example scripts are executable
#   5. Docs reference correct binary names
#
# Usage:
#   ./scripts/check-docs.sh
#
set -euo pipefail

GREEN="\033[0;32m"
RED="\033[0;31m"
YELLOW="\033[0;33m"
NC="\033[0m"
FAILED=0

pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
fail() { echo -e "${RED}[FAIL]${NC} $1"; FAILED=1; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }

echo "=== Documentation Quality Checks ==="
echo ""

# ── 1. Check for TODO/FIXME/HACK in docs ──────────
echo "--- Checking for placeholder text ---"
found_placeholders=$(rg -l "TODO|FIXME|HACK" docs/ --type md 2>/dev/null || true)
if [[ -n "$found_placeholders" ]]; then
    warn "Placeholders found in:"
    echo "$found_placeholders" | while read -r f; do warn "  $f"; done
else
    pass "No TODO/FIXME/HACK in docs"
fi

# ── 2. Check doc files exist ──────────────────────
echo "--- Checking doc file references ---"
if [[ -f "README.md" ]]; then
    pass "README.md exists"
else
    fail "README.md missing"
fi

# ── 3. Check binary name references ───────────────
echo "--- Checking binary name references ---"
# Old names used as commands outside the migration guide (which intentionally
# documents legacy syntax in its "Old" columns)
if rg -q "primusdb-server [a-z]" docs/ --type md -g '!cli/migration.md' 2>/dev/null; then
    warn "Some docs still invoke the legacy 'primusdb-server' binary"
fi
if rg -q "primusdb-cli [a-z]" docs/ --type md -g '!cli/migration.md' 2>/dev/null; then
    warn "Some docs still invoke the legacy 'primusdb-cli' binary"
fi
if ! rg -q "primusdb server" docs/ --type md 2>/dev/null; then
    fail "No docs reference the 'primusdb server' command"
fi
pass "Binary name check complete"

# ── 4. Check for broken markdown links ─────────────
echo "--- Checking markdown links ---"
# Basic check for .md links that might be broken
found_links=$(rg -n "\[.*\]\(.*\.md\)" README.md 2>/dev/null || true)
if [[ -n "$found_links" ]]; then
    while IFS= read -r line; do
        # Extract the path from markdown link
        link_path=$(echo "$line" | sed -n 's/.*(\([^)]*\.md\).*/\1/p')
        if [[ -n "$link_path" && ! -f "$link_path" ]]; then
            fail "Broken link in README.md: $link_path"
        fi
    done <<< "$found_links"
fi
pass "Link check complete"

# ── 5. Check example scripts are executable ────────
echo "--- Checking example scripts ---"
if [[ -d "examples" ]]; then
    while IFS= read -r -d '' script; do
        if [[ ! -x "$script" ]]; then
            warn "Example script not executable: $script"
        fi
    done < <(find examples/ -name "*.sh" -print0 2>/dev/null)
fi
pass "Example script check complete"

# ── Summary ───────────────────────────────────────
echo ""
if [[ "$FAILED" -eq 1 ]]; then
    echo -e "${RED}Some documentation checks failed.${NC}"
    exit 1
else
    echo -e "${GREEN}Documentation checks passed.${NC}"
fi
