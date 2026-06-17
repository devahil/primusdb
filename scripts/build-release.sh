#!/usr/bin/env bash
#
# build-release.sh — Build PrimusDB in release mode
#
# Usage:
#   ./scripts/build-release.sh
#   ./scripts/build-release.sh --features "some-feature"
#
set -euo pipefail

FEATURES=""
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --features) FEATURES="$2"; shift 2 ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--features \"...\"]"
    echo "  --features    Space-separated feature flags"
    exit 0
fi

echo "Building PrimusDB release binary..."
echo "Features: ${FEATURES:-default}"
echo ""

BUILD_CMD=(cargo build --release --workspace)
if [[ -n "$FEATURES" ]]; then
    BUILD_CMD+=(--features "$FEATURES")
fi

time "${BUILD_CMD[@]}"

echo ""
echo "Release binaries:"
ls -lh target/release/primusdb{,-server,-cli} 2>/dev/null || true
echo ""
echo "Release build complete."
